//! BG-CAD-P7-FILLET â€” the plane-plane fillet on the rewrite engine (D1/D2/D3):
//! dyadic witnesses only (tests 1-8 of PACKET.md). The box [0,4]Â²Ã—[0,2] is
//! built via `truck_modeling::primitive::cuboid`; the probe's vertical edge at
//! (4,4) with radius 1 is the primary witness.
//!
//! Every assertion is a statement about the `fillet()` rewrite in
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
use truck_base::bounding_box::BoundingBox;
use truck_base::cgmath64::{Point2, Point3, Vector3};
use truck_base::evidence::{Budget, EnvelopeCase, Outcome, Refusal};
use truck_geometry::arrange::{arrange, Arrangement};
use truck_geometry::canonical::{Curve, Surface};
use truck_geometry::prelude::*;
use truck_geometry::recognize::{
    recognize_surface, CanonicalCarrier, CanonicalCarrierWitness, CanonicalSurface,
};
use truck_modeling::extrude::extrude_profile;
use truck_modeling::primitive::cuboid;
use truck_shapeops::boolean::assemble::boolean;
use truck_shapeops::boolean::BoolOp;
use truck_shapeops::rewrite::{fillet, FilletSpec};
use truck_topology::Solid;

// ---------------------------------------------------------------------------
// construction helpers
// ---------------------------------------------------------------------------

/// The box solid [0,4]Â²Ã—[0,2] via the landed cuboid primitive.
fn box_solid() -> Solid<Point3, Curve, Surface> {
    cuboid(BoundingBox::from_iter([
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(4.0, 4.0, 2.0),
    ]))
}

/// The probe's primary witness: the vertical edge at (4,4), radius 1.
fn probe_edge() -> FilletSpec {
    FilletSpec {
        a: Point3::new(4.0, 4.0, 0.0),
        b: Point3::new(4.0, 4.0, 2.0),
        radius: 1.0,
    }
}

/// A pure-disk profile: one full circle of radius `r` at `center`.
fn disk_profile(center: Point2, r: f64) -> (Vec<Curve>, Arrangement) {
    let circle = Curve::Circle(Processor::with_transform(
        TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, std::f64::consts::TAU)),
        Matrix4 {
            x: Vector4::new(r, 0.0, 0.0, 0.0),
            y: Vector4::new(0.0, r, 0.0, 0.0),
            z: Vector4::new(0.0, 0.0, 1.0, 0.0),
            w: Vector4::new(center.x, center.y, 0.0, 1.0),
        },
    ));
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

/// Runs one fillet with a fresh budget.
fn run_fillet(
    solid: &Solid<Point3, Curve, Surface>,
    specs: &[FilletSpec],
) -> Outcome<Solid<Point3, Curve, Surface>> {
    let mut budget = Budget::new(1000, 1000, 1000);
    fillet(solid, specs, &mut budget)
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

/// The cylinder carrier of a recognized surface, unwrapped from an exact,
/// derived, or placed carrier.
fn carrier_cylinder(carrier: &CanonicalCarrier) -> Option<truck_geometry::specifieds::Cylinder> {
    match carrier {
        CanonicalCarrier::Surface(CanonicalSurface::Cylinder(c)) => Some(*c),
        CanonicalCarrier::Surface(CanonicalSurface::Placed(p)) => match &**p.entity() {
            CanonicalSurface::Cylinder(c) => Some(*c),
            _ => None,
        },
        _ => None,
    }
}

/// The cylinder carriers of the solid's faces, exact or placed.
fn cylinder_faces(
    solid: &Solid<Point3, Curve, Surface>,
) -> Vec<truck_geometry::specifieds::Cylinder> {
    solid
        .face_iter()
        .filter_map(|face| match recognize_surface(&face.surface()) {
            CanonicalCarrierWitness::ExactCanonical { carrier, .. }
            | CanonicalCarrierWitness::Derived { carrier, .. } => carrier_cylinder(&carrier),
            _ => None,
        })
        .collect()
}

/// The sphere carrier of a recognized surface, unwrapped from an exact,
/// derived, or placed carrier.
fn carrier_sphere(carrier: &CanonicalCarrier) -> Option<truck_geometry::specifieds::Sphere> {
    match carrier {
        CanonicalCarrier::Surface(CanonicalSurface::Sphere(s)) => Some(*s),
        CanonicalCarrier::Surface(CanonicalSurface::Placed(p)) => match &**p.entity() {
            CanonicalSurface::Sphere(s) => Some(*s),
            _ => None,
        },
        _ => None,
    }
}

/// The sphere carriers of the solid's faces, exact or placed.
fn sphere_faces(solid: &Solid<Point3, Curve, Surface>) -> Vec<truck_geometry::specifieds::Sphere> {
    solid
        .face_iter()
        .filter_map(|face| match recognize_surface(&face.surface()) {
            CanonicalCarrierWitness::ExactCanonical { carrier, .. }
            | CanonicalCarrierWitness::Derived { carrier, .. } => carrier_sphere(&carrier),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Test 1: the F1 probe witness through the engine.
// ---------------------------------------------------------------------------

#[test]
fn fillet_symmetric_box() {
    let boxed = box_solid();
    let result = expect_ok(run_fillet(&boxed, &[probe_edge()]));
    assert_eq!(result.boundaries().len(), 1);
    let shell = result.boundaries().first().unwrap();
    assert_eq!(shell.face_iter().count(), 7);
    assert_eq!(unique_edges(&result), 15);
    assert_eq!(unique_vertices(&result), 10);
    // The realized face is the canonical z-axis quarter cylinder about (3,3).
    let cylinders = cylinder_faces(&result);
    assert_eq!(cylinders.len(), 1);
    let cylinder = cylinders[0];
    assert_eq!(cylinder.radius(), 1.0);
    assert_eq!(cylinder.center(), Point3::new(3.0, 3.0, 0.0));
    // The cap faces (z=0, z=2) carry the quarter arc edges.
    let arcs: Vec<(Point3, Point3)> = result
        .face_iter()
        .filter_map(|face| match face.surface() {
            Surface::Plane(_) => {
                let mut out = Vec::new();
                for wire in face.absolute_boundaries() {
                    for edge in wire.edge_iter() {
                        if matches!(edge.curve(), Curve::Circle(_)) {
                            let (a, b) = edge.absolute_ends();
                            out.push((a.point(), b.point()));
                        }
                    }
                }
                Some(out)
            }
            _ => None,
        })
        .flatten()
        .collect();
    assert_eq!(arcs.len(), 2);
    let ordered = |a: Point3, b: Point3| {
        if a.x < b.x || (a.x == b.x && a.y < b.y) {
            (a, b)
        } else {
            (b, a)
        }
    };
    assert!(arcs.contains(&ordered(
        Point3::new(4.0, 3.0, 0.0),
        Point3::new(3.0, 4.0, 0.0)
    )));
    assert!(arcs.contains(&ordered(
        Point3::new(4.0, 3.0, 2.0),
        Point3::new(3.0, 4.0, 2.0)
    )));
    // The bounding box is exactly [0,4]Â²Ã—[0,2].
    assert_eq!(solid_box(&result), ((0.0, 4.0), (0.0, 4.0), (0.0, 2.0)));
}

// ---------------------------------------------------------------------------
// Test 2: two independent filleted edges.
// ---------------------------------------------------------------------------

#[test]
fn fillet_two_independent_edges() {
    let boxed = box_solid();
    let specs = [
        probe_edge(),
        FilletSpec {
            a: Point3::new(0.0, 0.0, 0.0),
            b: Point3::new(0.0, 0.0, 2.0),
            radius: 1.0,
        },
    ];
    let result = expect_ok(run_fillet(&boxed, &specs));
    assert_eq!(result.face_iter().count(), 8);
    assert_eq!(unique_edges(&result), 18);
    assert_eq!(unique_vertices(&result), 12);
    // The two quarter cylinders at (3,3) and (1,1).
    let cylinders = cylinder_faces(&result);
    assert_eq!(cylinders.len(), 2);
    let mut centers: Vec<(f64, f64)> = cylinders
        .iter()
        .map(|c| (c.center().x, c.center().y))
        .collect();
    centers.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(centers, vec![(1.0, 1.0), (3.0, 3.0)]);
}

// ---------------------------------------------------------------------------
// Test 3: two filleted edges on one shared face.
// ---------------------------------------------------------------------------

#[test]
fn fillet_same_face_pair() {
    let boxed = box_solid();
    let specs = [
        FilletSpec {
            a: Point3::new(4.0, 0.0, 0.0),
            b: Point3::new(4.0, 0.0, 2.0),
            radius: 1.0,
        },
        probe_edge(),
    ];
    let result = expect_ok(run_fillet(&boxed, &specs));
    assert_eq!(result.face_iter().count(), 8);
    assert_eq!(unique_edges(&result), 18);
    assert_eq!(unique_vertices(&result), 12);
    // The shared x=4 face trims to y âˆˆ [1, 3].
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
    // Two cylinder faces.
    let cylinders = cylinder_faces(&result);
    assert_eq!(cylinders.len(), 2);
}

// ---------------------------------------------------------------------------
// Test 4: the F4 three-plane corner sphere.
// ---------------------------------------------------------------------------

#[test]
fn fillet_three_edge_corner_sphere() {
    let boxed = box_solid();
    let specs = [
        probe_edge(),
        FilletSpec {
            a: Point3::new(4.0, 0.0, 2.0),
            b: Point3::new(4.0, 4.0, 2.0),
            radius: 1.0,
        },
        FilletSpec {
            a: Point3::new(4.0, 4.0, 2.0),
            b: Point3::new(0.0, 4.0, 2.0),
            radius: 1.0,
        },
    ];
    let result = expect_ok(run_fillet(&boxed, &specs));
    // 10 faces: 6 planes + 3 cylinders + 1 sphere.
    assert_eq!(result.face_iter().count(), 10);
    let spheres = sphere_faces(&result);
    assert_eq!(spheres.len(), 1);
    assert_eq!(spheres[0].center(), Point3::new(3.0, 3.0, 1.0));
    assert_eq!(spheres[0].radius(), 1.0);
    let cylinders = cylinder_faces(&result);
    assert_eq!(cylinders.len(), 3);
    // The three junction quarter-arcs lie on the sphere: every sphere wire
    // vertex is at exactly distance 1 from the center (the D3 machine-check).
    let sphere_face = result
        .face_iter()
        .find(|f| {
            matches!(
                recognize_surface(&f.surface()),
                CanonicalCarrierWitness::ExactCanonical {
                    carrier: CanonicalCarrier::Surface(CanonicalSurface::Sphere(_)),
                    ..
                }
            )
        })
        .expect("the sphere face exists");
    let wire = sphere_face.absolute_boundaries().first().expect("one wire");
    assert_eq!(wire.edge_iter().count(), 3);
    let mut pole_seen = false;
    for v in wire.vertex_iter() {
        let p = v.point();
        assert_eq!((p - Point3::new(3.0, 3.0, 1.0)).magnitude(), 1.0);
        if p == Point3::new(3.0, 3.0, 2.0) {
            pole_seen = true;
        }
    }
    // The pole (u=0 in the sphere frame) is a regular wire vertex of the patch.
    assert!(pole_seen);
    // The top face z=2 carries the pole as its corner vertex: the sphere
    // touches the top plane at exactly the parameter-frame pole (3,3,2).
    let top = result
        .face_iter()
        .find(|f| matches!(f.surface(), Surface::Plane(p) if p.normal() == Vector3::unit_z()))
        .expect("the top face survives");
    let top_pts: Vec<Point3> = top.absolute_boundaries()[0]
        .vertex_iter()
        .map(|v| v.point())
        .collect();
    assert!(top_pts.contains(&Point3::new(3.0, 3.0, 2.0)));
    // Machine-checked deviation from the packet's "hexagon" prose: the two
    // tangent lines meet the sphere junction at (3,3,2), so the kept region is
    // the quad [0,3]Ã—[0,3] at z=2 (see RESULT.json notes).
    assert_eq!(top_pts.len(), 4);
}

// ---------------------------------------------------------------------------
// Test 5: the D6 tangent-distance certificate on the F1 result.
// ---------------------------------------------------------------------------

#[test]
fn tangent_distance_certificate() {
    let boxed = box_solid();
    let result = expect_ok(run_fillet(&boxed, &[probe_edge()]));
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
fn fillet_nonplane_refuses() {
    let (profile, arr) = disk_profile(Point2::new(2.0, 2.0), 1.0);
    let cylinder = extrude_solid(&profile, &arr, 2.0);
    let mut budget = Budget::new(1000, 1000, 1000);
    let before = budget;
    let result = fillet(&cylinder, &[probe_edge()], &mut budget);
    assert!(matches!(
        result,
        Err(Refusal::UnsupportedEnvelope(
            EnvelopeCase::NonCanonicalCarrier
        ))
    ));
    assert_eq!(budget, before);
}

// ---------------------------------------------------------------------------
// Test 7: the radius reaches the adjacent extent â†’ `Refusal::Empty`.
// ---------------------------------------------------------------------------

#[test]
fn fillet_radius_overflow_refuses() {
    let boxed = box_solid();
    let spec = FilletSpec {
        a: Point3::new(4.0, 4.0, 0.0),
        b: Point3::new(4.0, 4.0, 2.0),
        radius: 4.0,
    };
    let mut budget = Budget::new(1000, 1000, 1000);
    let result = fillet(&boxed, &[spec], &mut budget);
    assert!(matches!(result, Err(Refusal::Empty)));
}

// ---------------------------------------------------------------------------
// Test 8: the F1 result downstream-consumes in the landed Boolean.
// ---------------------------------------------------------------------------

#[test]
fn fillet_result_survives_boolean() {
    let boxed = box_solid();
    let filleted = expect_ok(run_fillet(&boxed, &[probe_edge()]));
    // The small box [1.5,2.5]Â²Ã—[0.5,2.5] crosses the top boundary (the
    // resew convention).
    let small = cuboid(BoundingBox::from_iter([
        Point3::new(1.5, 1.5, 0.5),
        Point3::new(2.5, 2.5, 2.5),
    ]));
    let mut budget = Budget::new(1000, 1000, 1000);
    let diff = boolean(&filleted, BoolOp::Difference, &small, &mut budget);
    assert!(diff.is_ok());
}
