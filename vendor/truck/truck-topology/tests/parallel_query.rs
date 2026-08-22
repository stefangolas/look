#![deny(clippy::unwrap_used)]
//! BG-CE-003-MIGRATE regression: with `Arc<G>` geometry storage there is no
//! lock anywhere on the topology, so concurrent construction and query cannot
//! deadlock. Before the migration, mapping a wire while reading geometry in
//! the closure was the documented deadlock hazard.

use rayon::prelude::*;
use std::ops::Bound;
use truck_geotrait::{BoundedCurve, Cut, ParametricCurve, SPHint1D, SearchParameter, D1};
truck_topology::prelude!((), (), ());

/// A minimal piecewise-linear curve that can be cut and searched, so an edge
/// can be cut concurrently without real geometry. Mirrors the kernel's own
/// `TestCutCurve`.
#[derive(Clone, Debug)]
struct TestCutCurve(usize, usize);

impl ParametricCurve for TestCutCurve {
    type Point = usize;
    type Vector = usize;
    fn subs(&self, t: f64) -> usize {
        if t < 0.5 {
            self.0
        } else {
            self.1
        }
    }
    fn der(&self, _: f64) -> usize {
        self.1 - self.0
    }
    fn der2(&self, _: f64) -> usize {
        self.1 - self.0
    }
    fn der_n(&self, _: usize, _: f64) -> usize {
        self.1 - self.0
    }
    fn parameter_range(&self) -> truck_geotrait::ParameterRange {
        (Bound::Included(0.0), Bound::Included(1.0))
    }
}

impl BoundedCurve for TestCutCurve {}

impl Cut for TestCutCurve {
    fn cut(&mut self, _t: f64) -> Self {
        self.clone()
    }
}

impl SearchParameter<D1> for TestCutCurve {
    type Point = usize;
    fn search_parameter<H: Into<SPHint1D>>(
        &self,
        point: usize,
        _hint: H,
        _trials: usize,
    ) -> Option<f64> {
        if point == self.0 {
            Some(0.25)
        } else if point == self.1 {
            Some(0.75)
        } else {
            None
        }
    }
}

#[test]
fn parallel_query_never_deadlocks() {
    // A closed tetrahedron shell with empty geometry, from the lib.rs doc.
    let v = Vertex::news([(); 4]);
    let edge = [
        Edge::new(&v[0], &v[1], ()),
        Edge::new(&v[0], &v[2], ()),
        Edge::new(&v[0], &v[3], ()),
        Edge::new(&v[1], &v[2], ()),
        Edge::new(&v[1], &v[3], ()),
        Edge::new(&v[2], &v[3], ()),
    ];
    let wire = vec![
        wire![&edge[0], &edge[3], &edge[1].inverse()],
        wire![&edge[1], &edge[5], &edge[2].inverse()],
        wire![&edge[2], &edge[4].inverse(), &edge[0].inverse()],
        wire![&edge[3], &edge[5], &edge[4].inverse()],
    ];
    let mut face: Vec<Face> = wire
        .into_iter()
        .map(|wire| Face::new(vec![wire], ()))
        .collect();
    face[3].invert();
    let shell: Shell = face.into();

    // The vertex-level regression from the old doc remarks: mapping a vertex
    // while reading its own point in the closure.
    let v0 = truck_topology::Vertex::<usize>::new(0usize);
    let v1 = v0.mapped(|p| {
        let _ = v0.point();
        *p + 1
    });
    assert_eq!(v1.point(), 1usize);

    (0..8).into_par_iter().for_each(|i| {
        // Clone vertices and query them concurrently.
        let tetra: Vec<Vertex> = shell.vertex_iter().collect();
        let distinct: std::collections::HashSet<_> = tetra.iter().map(|v| v.id()).collect();
        assert_eq!(distinct.len(), 4);
        for vertex in &tetra {
            vertex.point();
            let _ = vertex.id();
            let _ = vertex.count();
        }

        // Map a wire.
        let wire0 = shell[0].boundaries()[0].mapped(|p: &()| *p, |c: &()| *c);
        assert_eq!(wire0.len(), 3);

        // Cut an edge with a numeric curve.
        let cv = truck_topology::Vertex::<usize>::news([0usize, 1usize]);
        let cut = truck_topology::Vertex::<usize>::new(0usize);
        let e =
            truck_topology::Edge::<usize, TestCutCurve>::new(&cv[0], &cv[1], TestCutCurve(0, 1));
        match e.cut(&cut) {
            Some((a, b)) => {
                assert_eq!(a.front().point(), 0usize);
                assert_eq!(b.back().point(), 1usize);
            }
            None => unreachable!("thread {i}: the midpoint cut must succeed"),
        }

        // Format an edge with Debug.
        let debug = format!("{:?}", e);
        assert!(!debug.is_empty());

        // The edge-level regression: mapping an edge while reading its own
        // curve in the closure.
        let mapped_edge = e.mapped(
            |p: &usize| {
                let _ = e.curve();
                *p
            },
            |c: &TestCutCurve| c.clone(),
        );
        assert_eq!(mapped_edge.curve().0, 0);
    });
}
