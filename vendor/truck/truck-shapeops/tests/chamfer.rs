//! BG-CAD-P6-REWRITE — the LocalBoundaryRewrite engine, proven on plane-plane
//! chamfer: dyadic witnesses only (tests 1-9 of PACKET.md). The box
//! [0,4]²×[0,2] is built via `truck_modeling::primitive::cuboid` (A4); the
//! probe's vertical edge at (4,4) is the primary witness (P1).
//!
//! Every assertion is a statement about the `chamfer()` rewrite in
//! `truck_shapeops::rewrite`, whose `Solid::try_new` acceptance gate (D6) is
//! exercised directly by the `expect_ok` helper.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]
// Test-only allow: H-1 bans unwrap/expect/panic on paths reachable from
// untrusted geometry. This file is integration-test assertions on hand-built
// dyadic witnesses - not such a path.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::HashSet;
use std::f64::consts::TAU;
use truck_base::bounding_box::BoundingBox;
use truck_base::cgmath64::{Matrix4, Point2, Point3, Vector3, Vector4, Zero};
use truck_base::evidence::{Budget, EnvelopeCase, Outcome, Refusal};
use truck_geometry::arrange::{arrange, Arrangement};
use truck_geometry::canonical::{Curve, Surface};
use truck_geometry::prelude::*;
use truck_modeling::extrude::extrude_profile;
use truck_modeling::primitive::cuboid;
use truck_shapeops::boolean::assemble::boolean;
use truck_shapeops::boolean::BoolOp;
use truck_shapeops::rewrite::{chamfer, ChamferSpec};
use truck_topology::Solid;

// ---------------------------------------------------------------------------
// construction helpers
// ---------------------------------------------------------------------------

/// The box solid [0,4]²×[0,2] via the landed cuboid primitive.
fn box_solid() -> Solid<Point3, Curve, Surface> {
    cuboid(BoundingBox::from_iter([
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(4.0, 4.0, 2.0),
    ]))
}

/// The probe's primary witness: the vertical edge at (4,4), symmetric d=1.
fn probe_edge() -> ChamferSpec {
    ChamferSpec {
        a: Point3::new(4.0, 4.0, 0.0),
        b: Point3::new(4.0, 4.0, 2.0),
        d_first: 1.0,
        d_second: 1.0,
    }
}

/// A placed full-period circle at `center` with radius `r`.
fn placed_circle(center: Point3, r: f64) -> Processor<TrimmedCurve<UnitCircle<Point3>>, Matrix4> {
    Processor::with_transform(
        TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, TAU)),
        Matrix4 {
            x: Vector4::new(r, 0.0, 0.0, 0.0),
            y: Vector4::new(0.0, r, 0.0, 0.0),
            z: Vector4::new(0.0, 0.0, 1.0, 0.0),
            w: Vector4::new(center.x, center.y, center.z, 1.0),
        },
    )
}

/// A pure-disk profile: one full circle of radius `r` at `center`.
fn disk_profile(center: Point2, r: f64) -> (Vec<Curve>, Arrangement) {
    let circle = Curve::Circle(placed_circle(Point3::new(center.x, center.y, 0.0), r));
    let profile = vec![circle];
    let ok = arrange(&profile, None).unwrap();
    (profile, ok.value)
}

/// The solid `height`-extrude of a profile (the cylinder-carrying fixture).
fn extrude_solid(
    profile: &[Curve],
    arr: &Arrangement,
    height: f64,
) -> Solid<Point3, Curve, Surface> {
    extrude_profile(profile, arr, height)
        .expect("the dyadic profile extrudes")
        .value
}

/// The match-based `Ok` unwrapper of the packet (D1).
fn expect_ok<T>(r: Outcome<T>) -> T {
    match r {
        Ok(c) => c.value,
        Err(e) => panic!("unexpected refusal: {e:?}"),
    }
}

/// Runs one chamfer with a fresh budget.
fn run_chamfer(
    solid: &Solid<Point3, Curve, Surface>,
    specs: &[ChamferSpec],
) -> Outcome<Solid<Point3, Curve, Surface>> {
    let mut budget = Budget::new(1000, 1000, 1000);
    chamfer(solid, specs, &mut budget)
}

// ---------------------------------------------------------------------------
// measurement helpers
// ---------------------------------------------------------------------------

/// The axis-aligned bounding box of a solid's vertices.
fn solid_box(solid: &Solid<Point3, Curve, Surface>) -> ((f64, f64), (f64, f64), (f64, f64)) {
    let mut lo = Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
    let mut hi = Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for v in solid.vertex_iter() {
        let p = v.point();
        lo.x = lo.x.min(p.x);
        lo.y = lo.y.min(p.y);
        lo.z = lo.z.min(p.z);
        hi.x = hi.x.max(p.x);
        hi.y = hi.y.max(p.y);
        hi.z = hi.z.max(p.z);
    }
    ((lo.x, hi.x), (lo.y, hi.y), (lo.z, hi.z))
}

/// The unique-edge count (per-use iteration deduped by edge id).
fn unique_edges(solid: &Solid<Point3, Curve, Surface>) -> usize {
    solid
        .edge_iter()
        .map(|e| e.id())
        .collect::<HashSet<_>>()
        .len()
}

/// The unique-vertex count (per-use iteration deduped by vertex id).
fn unique_vertices(solid: &Solid<Point3, Curve, Surface>) -> usize {
    solid
        .vertex_iter()
        .map(|v| v.id())
        .collect::<HashSet<_>>()
        .len()
}

/// The planes of the solid's faces whose normal is parallel to `n`
/// (sign-agnostic).
fn faces_parallel_to(
    solid: &Solid<Point3, Curve, Surface>,
    n: Vector3,
) -> Vec<truck_geometry::specifieds::Plane> {
    solid
        .face_iter()
        .filter_map(|face| match face.surface() {
            Surface::Plane(p) if p.normal().cross(n) == Vector3::zero() => Some(p),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Test 1: the P1 witness through the engine.
// ---------------------------------------------------------------------------

#[test]
fn chamfer_symmetric_box() {
    let boxed = box_solid();
    let result = expect_ok(run_chamfer(&boxed, &[probe_edge()]));
    assert_eq!(result.boundaries().len(), 1);
    let shell = result.boundaries().first().unwrap();
    assert_eq!(shell.face_iter().count(), 7);
    assert_eq!(unique_edges(&result), 15);
    assert_eq!(unique_vertices(&result), 10);
    // The chamfer plane x+y=7, normal sign-agnostic, offset exact.
    let planes = faces_parallel_to(&result, Vector3::new(1.0, 1.0, 0.0));
    assert_eq!(planes.len(), 1);
    let plane = planes[0];
    for p in [Point3::new(4.0, 3.0, 0.0), Point3::new(3.0, 4.0, 0.0)] {
        assert_eq!((p - plane.origin()).dot(plane.normal()), 0.0);
        assert_eq!(p.x + p.y, 7.0);
    }
    // The bounding box is exactly [0,4]²×[0,2].
    assert_eq!(solid_box(&result), ((0.0, 4.0), (0.0, 4.0), (0.0, 2.0)));
}

// ---------------------------------------------------------------------------
// Test 2: the P2 asymmetric witness (the D2 ordering contract).
// ---------------------------------------------------------------------------

#[test]
fn chamfer_asymmetric_box() {
    let boxed = box_solid();
    // The y=4 face's outward normal (0,1,0) is lexicographically smaller than
    // the x=4 face's (1,0,0), so d_first = 0.5 trims the y=4 face and
    // d_second = 1.0 the x=4 face.
    let spec = ChamferSpec {
        a: Point3::new(4.0, 4.0, 0.0),
        b: Point3::new(4.0, 4.0, 2.0),
        d_first: 0.5,
        d_second: 1.0,
    };
    let result = expect_ok(run_chamfer(&boxed, &[spec]));
    assert_eq!(result.face_iter().count(), 7);
    // The chamfer plane through (4,3) and (3.5,4): normal ∝ (2,1), 2x+y=11.
    let planes = faces_parallel_to(&result, Vector3::new(2.0, 1.0, 0.0));
    assert_eq!(planes.len(), 1);
    let plane = planes[0];
    for p in [Point3::new(4.0, 3.0, 0.0), Point3::new(3.5, 4.0, 0.0)] {
        assert_eq!((p - plane.origin()).dot(plane.normal()), 0.0);
        assert_eq!(2.0 * p.x + p.y, 11.0);
    }
}

// ---------------------------------------------------------------------------
// Test 3: the P3 witness — two independent chamfered edges.
// ---------------------------------------------------------------------------

#[test]
fn chamfer_two_independent_edges() {
    let boxed = box_solid();
    let specs = [
        probe_edge(),
        ChamferSpec {
            a: Point3::new(0.0, 0.0, 0.0),
            b: Point3::new(0.0, 0.0, 2.0),
            d_first: 1.0,
            d_second: 1.0,
        },
    ];
    let result = expect_ok(run_chamfer(&boxed, &specs));
    assert_eq!(result.face_iter().count(), 8);
    assert_eq!(unique_edges(&result), 18);
    assert_eq!(unique_vertices(&result), 12);
    // The two chamfer planes: x+y=7 at (4,4) and x+y=1 at (0,0).
    let planes = faces_parallel_to(&result, Vector3::new(1.0, 1.0, 0.0));
    assert_eq!(planes.len(), 2);
    let mut offsets: Vec<f64> = planes.iter().map(|p| p.origin().x + p.origin().y).collect();
    offsets.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(offsets, vec![1.0, 7.0]);
}

// ---------------------------------------------------------------------------
// Test 4: the P4 witness — two chamfered edges on one shared face.
// ---------------------------------------------------------------------------

#[test]
fn chamfer_same_face_pair() {
    let boxed = box_solid();
    let specs = [
        ChamferSpec {
            a: Point3::new(4.0, 0.0, 0.0),
            b: Point3::new(4.0, 0.0, 2.0),
            d_first: 1.0,
            d_second: 1.0,
        },
        probe_edge(),
    ];
    let result = expect_ok(run_chamfer(&boxed, &specs));
    assert_eq!(result.face_iter().count(), 8);
    assert_eq!(unique_edges(&result), 18);
    assert_eq!(unique_vertices(&result), 12);
    // The shared x=4 face trims to y ∈ [1, 3].
    let x4 = result
        .face_iter()
        .find(|f| matches!(f.surface(), Surface::Plane(p) if p.normal() == Vector3::unit_x()))
        .expect("the x=4 face survives");
    let mut ys: Vec<f64> = x4.absolute_boundaries()[0]
        .vertex_iter()
        .map(|v| v.point().y)
        .collect();
    ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(ys, vec![1.0, 1.0, 3.0, 3.0]);
    // The two chamfer planes: x−y=3 at (4,0) and x+y=7 at (4,4).
    let minus_planes = faces_parallel_to(&result, Vector3::new(1.0, -1.0, 0.0));
    assert_eq!(minus_planes.len(), 1);
    let minus_origin = minus_planes[0].origin();
    assert_eq!(minus_origin.x - minus_origin.y, 3.0);
    let plus_planes = faces_parallel_to(&result, Vector3::new(1.0, 1.0, 0.0));
    assert_eq!(plus_planes.len(), 1);
    let plus_origin = plus_planes[0].origin();
    assert_eq!(plus_origin.x + plus_origin.y, 7.0);
}

// ---------------------------------------------------------------------------
// Test 5: the D6 trim-distance certificate on the P1 result.
// ---------------------------------------------------------------------------

#[test]
fn trim_distance_certificate() {
    let boxed = box_solid();
    let result = expect_ok(run_chamfer(&boxed, &[probe_edge()]));
    let a = Point3::new(4.0, 4.0, 0.0);
    let b = Point3::new(4.0, 4.0, 2.0);
    let dir = b - a;
    let len = dir.magnitude();
    for p in [
        Point3::new(4.0, 3.0, 0.0),
        Point3::new(3.0, 4.0, 0.0),
        Point3::new(4.0, 3.0, 2.0),
        Point3::new(3.0, 4.0, 2.0),
    ] {
        assert!(result.vertex_iter().any(|v| v.point() == p));
        let distance = (p - a).cross(dir).magnitude() / len;
        assert_eq!(distance, 1.0);
    }
}

// ---------------------------------------------------------------------------
// Test 6: a cylinder-carrying solid refuses at the lift, budget untouched.
// ---------------------------------------------------------------------------

#[test]
fn chamfer_nonplane_refuses() {
    let (profile, arr) = disk_profile(Point2::new(2.0, 2.0), 1.0);
    let cylinder = extrude_solid(&profile, &arr, 2.0);
    let mut budget = Budget::new(1000, 1000, 1000);
    let before = budget;
    let result = chamfer(&cylinder, &[probe_edge()], &mut budget);
    assert!(matches!(
        result,
        Err(Refusal::UnsupportedEnvelope(
            EnvelopeCase::NonCanonicalCarrier
        ))
    ));
    assert_eq!(budget, before);
}

// ---------------------------------------------------------------------------
// Test 7: the trim reaches the opposite boundary edge → `Refusal::Empty`.
// ---------------------------------------------------------------------------

#[test]
fn chamfer_trim_overflow_refuses() {
    let boxed = box_solid();
    let spec = ChamferSpec {
        a: Point3::new(4.0, 4.0, 0.0),
        b: Point3::new(4.0, 4.0, 2.0),
        d_first: 4.0,
        d_second: 4.0,
    };
    let mut budget = Budget::new(1000, 1000, 1000);
    let result = chamfer(&boxed, &[spec], &mut budget);
    assert!(matches!(result, Err(Refusal::Empty)));
}

// ---------------------------------------------------------------------------
// Test 8: the D4 distance-angle form: d=1, α=45° = the P1 mesh.
// ---------------------------------------------------------------------------

#[test]
fn chamfer_distance_angle() {
    let boxed = box_solid();
    let spec = ChamferSpec::by_angle(
        Point3::new(4.0, 4.0, 0.0),
        Point3::new(4.0, 4.0, 2.0),
        1.0,
        std::f64::consts::FRAC_PI_4,
    );
    let result = expect_ok(run_chamfer(&boxed, &[spec]));
    assert_eq!(result.face_iter().count(), 7);
    // The d·tan(45°) second trim lands at exactly 1.0 on this box, so the
    // P1 plane data x+y=7 reproduces exactly.
    let planes = faces_parallel_to(&result, Vector3::new(1.0, 1.0, 0.0));
    assert_eq!(planes.len(), 1);
    let plane = planes[0];
    for p in [Point3::new(4.0, 3.0, 0.0), Point3::new(3.0, 4.0, 0.0)] {
        assert_eq!((p - plane.origin()).dot(plane.normal()), 0.0);
        assert_eq!(p.x + p.y, 7.0);
    }
}

// ---------------------------------------------------------------------------
// Test 9: the P1 result downstream-consumes in the landed Boolean.
// ---------------------------------------------------------------------------

#[test]
fn chamfer_result_survives_boolean() {
    let boxed = box_solid();
    let chamfered = expect_ok(run_chamfer(&boxed, &[probe_edge()]));
    // The small box [1.5,2.5]²×[0.5,2.5] crosses the top boundary (the
    // resew convention).
    let small = cuboid(BoundingBox::from_iter([
        Point3::new(1.5, 1.5, 0.5),
        Point3::new(2.5, 2.5, 2.5),
    ]));
    let mut budget = Budget::new(1000, 1000, 1000);
    let diff = boolean(&chamfered, BoolOp::Difference, &small, &mut budget);
    assert!(diff.is_ok());
}
