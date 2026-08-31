#![deny(clippy::unwrap_used)]

//! BG-CG-006-DIAG — integration tests for the manifold-diagnostics
//! aggregate. Every fixture is pure combinatorial topology with `()` payloads
//! (line edges, no curves, no tessellation, no geometry). Each fixture's
//! premise is machine-checked before its diagnostics are asserted.

use std::collections::HashMap;
use truck_topology::manifold::{diagnose, orientation_parity, EdgeIrregularity, VertexLinkClass};
use truck_topology::shell::ShellCondition;
use truck_topology::*;

/// The `(vertices, edges, wires)` of the cube fixture.
type CubeFixture = (Vec<Vertex<()>>, Vec<Edge<(), ()>>, Vec<Wire<(), ()>>);

/// The 6-wire, 8-vertex, 12-edge cube used by most fixtures: five faces are
/// already mutually oriented and the sixth (top) face is closed by one
/// `invert()` — the same construction as the `ShellCondition::Closed`
/// doctest.
fn cube_fixture() -> CubeFixture {
    let v = Vertex::news([(); 8]);
    let edge = vec![
        Edge::new(&v[0], &v[1], ()),
        Edge::new(&v[1], &v[2], ()),
        Edge::new(&v[2], &v[3], ()),
        Edge::new(&v[3], &v[0], ()),
        Edge::new(&v[0], &v[4], ()),
        Edge::new(&v[1], &v[5], ()),
        Edge::new(&v[2], &v[6], ()),
        Edge::new(&v[3], &v[7], ()),
        Edge::new(&v[4], &v[5], ()),
        Edge::new(&v[5], &v[6], ()),
        Edge::new(&v[6], &v[7], ()),
        Edge::new(&v[7], &v[4], ()),
    ];
    let wires = vec![
        wire![&edge[0], &edge[1], &edge[2], &edge[3]],
        wire![&edge[0].inverse(), &edge[4], &edge[8], &edge[5].inverse()],
        wire![&edge[1].inverse(), &edge[5], &edge[9], &edge[6].inverse()],
        wire![&edge[2].inverse(), &edge[6], &edge[10], &edge[7].inverse()],
        wire![&edge[3].inverse(), &edge[7], &edge[11], &edge[4].inverse()],
        wire![&edge[8], &edge[9], &edge[10], &edge[11]],
    ];
    (v, edge, wires)
}

/// A closed, outward-oriented cube.
fn closed_cube() -> Shell<(), (), ()> {
    let (_, _, wires) = cube_fixture();
    let mut faces: Vec<Face<(), (), ()>> = wires
        .into_iter()
        .map(|wire| Face::new(vec![wire], ()))
        .collect();
    faces[5].invert();
    faces.into()
}

#[test]
fn closed_cube_is_closed_manifold() {
    let shell = closed_cube();
    assert_eq!(shell.shell_condition(), ShellCondition::Closed);
    let diagnostics = diagnose(&shell);
    assert_eq!(diagnostics.shell_condition, ShellCondition::Closed);
    assert_eq!(diagnostics.connected_components, 1);
    assert!(diagnostics.boundary_edges.is_empty());
    assert!(diagnostics.orientation_conflicts.is_empty());
    assert!(diagnostics.irregular_edges.is_empty());
    assert_eq!(diagnostics.singular_vertices.len(), 8);
    assert!(diagnostics
        .singular_vertices
        .iter()
        .all(|d| d.classification == VertexLinkClass::ClosedCycle));
}

#[test]
fn open_box_has_boundary_path_links() {
    let (v, _, wires) = cube_fixture();
    let shell: Shell<(), (), ()> = wires
        .into_iter()
        .take(5)
        .map(|wire| Face::new(vec![wire], ()))
        .collect();
    assert_eq!(shell.shell_condition(), ShellCondition::Oriented);
    let diagnostics = diagnose(&shell);
    assert_eq!(diagnostics.boundary_edges.len(), 4);
    let top_rim = [v[4].id(), v[5].id(), v[6].id(), v[7].id()];
    let mut boundary_path = 0;
    let mut closed_cycle = 0;
    for d in &diagnostics.singular_vertices {
        if top_rim.contains(&d.vertex) {
            assert_eq!(d.classification, VertexLinkClass::BoundaryPath);
            boundary_path += 1;
        } else {
            assert_eq!(d.classification, VertexLinkClass::ClosedCycle);
            closed_cycle += 1;
        }
    }
    assert_eq!(boundary_path, 4);
    assert_eq!(closed_cycle, 4);
}

#[test]
fn two_sheets_pinch_is_nonmanifold_at_vertex() {
    let v = Vertex::news([(); 5]);
    let e0 = Edge::new(&v[0], &v[1], ());
    let e1 = Edge::new(&v[1], &v[2], ());
    let e2 = Edge::new(&v[2], &v[0], ());
    let e3 = Edge::new(&v[0], &v[3], ());
    let e4 = Edge::new(&v[3], &v[4], ());
    let e5 = Edge::new(&v[4], &v[0], ());
    let shell: Shell<(), (), ()> = vec![
        Face::new(vec![wire![&e0, &e1, &e2]], ()),
        Face::new(vec![wire![&e3, &e4, &e5]], ()),
    ]
    .into();
    assert_eq!(shell.singular_vertices(), vec![v[0].clone()]);
    let diagnostics = diagnose(&shell);
    assert_eq!(diagnostics.boundary_edges.len(), 6);
    assert_eq!(diagnostics.singular_vertices.len(), 5);
    let shared = v[0].id();
    let mut nonmanifold = 0;
    let mut boundary_path = 0;
    for d in &diagnostics.singular_vertices {
        if d.vertex == shared {
            assert_eq!(d.classification, VertexLinkClass::NonManifold);
            nonmanifold += 1;
        } else {
            assert_eq!(d.classification, VertexLinkClass::BoundaryPath);
            boundary_path += 1;
        }
    }
    assert_eq!(nonmanifold, 1);
    assert_eq!(boundary_path, 4);
}

#[test]
fn irregular_shell_lists_over_shared_edge() {
    let v = Vertex::news([(); 5]);
    let edge = [
        Edge::new(&v[0], &v[1], ()),
        Edge::new(&v[0], &v[2], ()),
        Edge::new(&v[0], &v[3], ()),
        Edge::new(&v[0], &v[4], ()),
        Edge::new(&v[1], &v[2], ()),
        Edge::new(&v[1], &v[3], ()),
        Edge::new(&v[1], &v[4], ()),
    ];
    let shell: Shell<(), (), ()> = vec![
        Face::new(vec![wire![&edge[0], &edge[4], &edge[1].inverse()]], ()),
        Face::new(vec![wire![&edge[0], &edge[5], &edge[2].inverse()]], ()),
        Face::new(vec![wire![&edge[0], &edge[6], &edge[3].inverse()]], ()),
    ]
    .into();
    assert_eq!(shell.shell_condition(), ShellCondition::Irregular);
    let diagnostics = diagnose(&shell);
    assert_eq!(diagnostics.irregular_edges.len(), 1);
    if let Some(diagnosis) = diagnostics.irregular_edges.first() {
        assert_eq!(diagnosis.edge, edge[0].id());
        assert_eq!(
            diagnosis.classification,
            EdgeIrregularity::OverShared { use_count: 3 }
        );
    }
}

#[test]
fn inverted_face_produces_orientation_conflicts() {
    let (_, _, wires) = cube_fixture();
    let mut faces: Vec<Face<(), (), ()>> = wires
        .into_iter()
        .map(|wire| Face::new(vec![wire], ()))
        .collect();
    faces[5].invert();
    let inverted_id = faces[1].id();
    faces[1].invert();
    let shell: Shell<(), (), ()> = faces.into();
    assert_eq!(shell.shell_condition(), ShellCondition::Regular);
    let diagnostics = diagnose(&shell);
    assert_eq!(diagnostics.orientation_conflicts.len(), 4);
    assert!(diagnostics
        .orientation_conflicts
        .iter()
        .all(|c| c.face_a == inverted_id || c.face_b == inverted_id));
    assert!(orientation_parity(&shell).is_none());
}

#[test]
fn parity_assignment_is_some_for_oriented_shell() {
    let shell = closed_cube();
    assert_eq!(shell.shell_condition(), ShellCondition::Closed);
    let diagnostics = diagnose(&shell);
    assert!(diagnostics.orientation_conflicts.is_empty());
    assert!(orientation_parity(&shell).is_some());
    let seed_id = shell.first().map(|face| face.id());
    if let Some(parity) = orientation_parity(&shell) {
        assert_eq!(parity.len(), 6);
        if let Some(seed_id) = seed_id {
            assert_eq!(parity.get(&seed_id), Some(&true));
        }
    }
}

#[test]
fn diagnostics_output_order_is_deterministic() {
    let (_, _, wires) = cube_fixture();
    let mut faces: Vec<Face<(), (), ()>> = wires
        .into_iter()
        .map(|wire| Face::new(vec![wire], ()))
        .collect();
    faces[5].invert();
    faces[1].invert();
    // Faces assembled in a scrambled order: the shell's iteration order does
    // not match the vertex/edge creation order.
    let shell: Shell<(), (), ()> = vec![
        faces[5].clone(),
        faces[2].clone(),
        faces[0].clone(),
        faces[4].clone(),
        faces[1].clone(),
        faces[3].clone(),
    ]
    .into();
    assert_eq!(shell.shell_condition(), ShellCondition::Regular);
    let first = diagnose(&shell);
    let second = diagnose(&shell);
    assert_eq!(first, second);
    // Recompute the iteration ordinals the same way the implementation does.
    let vertex_ordinal: HashMap<VertexID<()>, usize> = shell
        .vertex_iter()
        .enumerate()
        .map(|(i, vertex)| (vertex.id(), i))
        .collect();
    let edge_ordinal: HashMap<EdgeID<()>, usize> = shell
        .edge_iter()
        .enumerate()
        .map(|(i, edge)| (edge.id(), i))
        .collect();
    let face_ordinal: HashMap<FaceID<()>, usize> = shell
        .face_iter()
        .enumerate()
        .map(|(i, face)| (face.id(), i))
        .collect();
    assert_ordered_by(&first.boundary_edges, |edge| edge_ordinal[edge]);
    assert_ordered_by(&first.irregular_edges, |diagnosis| {
        edge_ordinal[&diagnosis.edge]
    });
    assert_ordered_by(&first.singular_vertices, |diagnosis| {
        vertex_ordinal[&diagnosis.vertex]
    });
    assert_ordered_by(&first.orientation_conflicts, |conflict| {
        (
            edge_ordinal[&conflict.edge],
            face_ordinal[&conflict.face_a],
            face_ordinal[&conflict.face_b],
        )
    });
}

/// Asserts `items` is non-decreasing under `key`.
fn assert_ordered_by<T, K>(items: &[T], key: impl Fn(&T) -> K)
where
    K: PartialOrd,
{
    assert!(items.windows(2).all(|pair| key(&pair[0]) <= key(&pair[1])));
}
