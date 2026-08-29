//! BG-CAD-P4-UNTIL — the packet's nine required acceptance tests.
//!
//! The certified `until` sweep reduction and plane projection, exercised on
//! the extrude.rs test pattern: the [1,3]² rectangle flagship, the oblique
//! target plane through (0,0,2), and the dyadic witness conventions (compare
//! planes by data, never by unit length).

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]
// Test-only allow: H-1 bans unwrap/expect/panic on paths reachable from
// untrusted geometry; test assertions on hand-built witnesses are not such a
// path (the recognize.rs test-module precedent). The deny list above stays;
// `expect_ok` unwraps via `match` + `panic` so the deny lints stay satisfied.
#![allow(clippy::panic)]

use std::f64::consts::TAU;
use truck_base::evidence::{Budget, EnvelopeCase, Outcome, Refusal};
use truck_geometry::arrange::{arrange, Arrangement};
use truck_modeling::cad::solid_bounding_box;
use truck_modeling::extrude::extrude_profile_vector;
use truck_modeling::until::{extrude_until, project_profile, Until};
use truck_modeling::{
    Curve, Face, InnerSpace, Line, Matrix4, Plane, Point3, Processor, Solid, Surface, TrimmedCurve,
    UnitCircle, Vector3, Vector4,
};

/// The sweep direction of the flagship fixtures.
const SWEEP: Vector3 = Vector3::new(0.0, 0.0, 2.0);

/// Unwraps an `Outcome` via `match` + `panic` so the deny lints stay
/// satisfied (the recognize.rs test-module precedent).
fn expect_ok<T>(r: Outcome<T>) -> T {
    match r {
        Ok(ok) => ok.value,
        Err(refusal) => panic!("expected a certified value, got {refusal:?}"),
    }
}

/// The [1,3]² CCW square on z = 0 (the until fixture profile).
fn square_profile() -> Vec<Curve> {
    vec![
        Curve::Line(Line(Point3::new(1.0, 1.0, 0.0), Point3::new(3.0, 1.0, 0.0))),
        Curve::Line(Line(Point3::new(3.0, 1.0, 0.0), Point3::new(3.0, 3.0, 0.0))),
        Curve::Line(Line(Point3::new(3.0, 3.0, 0.0), Point3::new(1.0, 3.0, 0.0))),
        Curve::Line(Line(Point3::new(1.0, 3.0, 0.0), Point3::new(1.0, 1.0, 0.0))),
    ]
}

/// The L-shaped (non-convex) profile: a 6-edge CCW polygon with a reflex
/// vertex at (2,2).
fn l_shape_profile() -> Vec<Curve> {
    vec![
        Curve::Line(Line(Point3::new(1.0, 1.0, 0.0), Point3::new(3.0, 1.0, 0.0))),
        Curve::Line(Line(Point3::new(3.0, 1.0, 0.0), Point3::new(3.0, 2.0, 0.0))),
        Curve::Line(Line(Point3::new(3.0, 2.0, 0.0), Point3::new(2.0, 2.0, 0.0))),
        Curve::Line(Line(Point3::new(2.0, 2.0, 0.0), Point3::new(2.0, 3.0, 0.0))),
        Curve::Line(Line(Point3::new(2.0, 3.0, 0.0), Point3::new(1.0, 3.0, 0.0))),
        Curve::Line(Line(Point3::new(1.0, 3.0, 0.0), Point3::new(1.0, 1.0, 0.0))),
    ]
}

/// A placed full-range unit circle with the given center and radius.
fn circle_at(center: Point3, radius: f64) -> Curve {
    Curve::Circle(Processor::with_transform(
        TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, TAU)),
        Matrix4 {
            x: Vector4::new(radius, 0.0, 0.0, 0.0),
            y: Vector4::new(0.0, radius, 0.0, 0.0),
            z: Vector4::new(0.0, 0.0, 1.0, 0.0),
            w: Vector4::new(center.x, center.y, center.z, 1.0),
        },
    ))
}

/// The parallel target z = 2 (the §9 metamorphic fixture).
fn plane_z2() -> Plane {
    Plane::new(
        Point3::new(0.0, 0.0, 2.0),
        Point3::new(1.0, 0.0, 2.0),
        Point3::new(0.0, 1.0, 2.0),
    )
}

/// The oblique target: the plane through (0,0,2) with normal (−1,0,1)/√2
/// (z = x + 2), built from three exact dyadic points.
fn oblique_plane() -> Plane {
    Plane::new(
        Point3::new(0.0, 0.0, 2.0),
        Point3::new(1.0, 0.0, 3.0),
        Point3::new(0.0, 1.0, 2.0),
    )
}

/// Whether two planes agree by DATA (the defining point triple), never by
/// unit length.
fn same_plane(a: &Plane, b: &Plane) -> bool {
    a.origin() == b.origin() && a.u_axis() == b.u_axis() && a.v_axis() == b.v_axis()
}

/// The sorted 3-D point multiset (order-independent comparison).
fn sorted_pts(pts: &[Point3]) -> Vec<Point3> {
    let mut out: Vec<Point3> = pts.to_vec();
    out.sort_by(|a, b| {
        let by_x = a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal);
        by_x.then(a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal))
            .then(a.z.partial_cmp(&b.z).unwrap_or(std::cmp::Ordering::Equal))
    });
    out
}

/// The axis-aligned box of a face's boundary vertices.
fn face_box(face: &Face) -> (Point3, Point3) {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut min_z = f64::INFINITY;
    let mut max_z = f64::NEG_INFINITY;
    for wire in face.boundaries() {
        for edge in wire.edge_iter() {
            let p = edge.front().point();
            min_x = min_x.min(p.x);
            max_x = max_x.max(p.x);
            min_y = min_y.min(p.y);
            max_y = max_y.max(p.y);
            min_z = min_z.min(p.z);
            max_z = max_z.max(p.z);
        }
    }
    (
        Point3::new(min_x, min_y, min_z),
        Point3::new(max_x, max_y, max_z),
    )
}

/// Whether the material region's outer cycle carries a reflex vertex — the
/// machine-check that the convexity predicate actually fires on the region
/// extraction (mirrors `until.rs`'s selection and turn predicate).
fn outer_cycle_has_reflex(arrangement: &Arrangement) -> bool {
    let mut region = None;
    for candidate in &arrangement.regions {
        if candidate.bounded && candidate.winding == 1 {
            region = Some(candidate);
        }
    }
    let Some(region) = region else { return false };
    let Some(cycle) = region.boundaries.first() else {
        return false;
    };
    let mut pts: Vec<Point3> = Vec::new();
    for &h in cycle {
        let Some(he) = arrangement.half_edges.get(h) else {
            continue;
        };
        let Some(v) = arrangement.vertices.get(he.origin) else {
            continue;
        };
        pts.push(v.point);
    }
    let n = pts.len();
    if n < 3 {
        return false;
    }
    for i in 0..n {
        let a = match pts.get(i % n) {
            Some(p) => *p,
            None => return false,
        };
        let b = match pts.get((i + 1) % n) {
            Some(p) => *p,
            None => return false,
        };
        let c = match pts.get((i + 2) % n) {
            Some(p) => *p,
            None => return false,
        };
        let cross = (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x);
        if cross < 0.0 {
            return true;
        }
    }
    false
}

/// 1. The parallel target (z = 2, dir (0,0,2)) is the §9 metamorphic case: the
///    solid is face-count- and box-equal to a direct
///    `extrude_profile_vector` call with the same arguments, and the box is
///    the exact dyadic [1,3]×[1,3]×[0,2].
#[test]
fn until_parallel_target_metamorphic() {
    let profile = square_profile();
    let arrangement = expect_ok(arrange(&profile, None));
    let until_solid = expect_ok(extrude_until(
        &profile,
        &arrangement,
        SWEEP,
        &Until::Plane(plane_z2()),
    ));
    let direct = expect_ok(extrude_profile_vector(&profile, &arrangement, SWEEP, false));
    assert_eq!(until_solid.face_iter().count(), direct.face_iter().count());
    let mut budget = Budget::new(0, 0, 0);
    let b_until = expect_ok(solid_bounding_box(&until_solid, &mut budget));
    let b_direct = expect_ok(solid_bounding_box(&direct, &mut budget));
    assert_eq!(b_until.min(), b_direct.min());
    assert_eq!(b_until.max(), b_direct.max());
    assert_eq!(b_until.min(), Point3::new(1.0, 1.0, 0.0));
    assert_eq!(b_until.max(), Point3::new(3.0, 3.0, 2.0));
}

/// 2. The oblique target over the square: a valid solid, every face's carrier
///    is `Plane`, face count 6 (bottom, 4 walls, oblique cap) — machine-check
///    the count by role: exactly one cap on the target plane, exactly one
///    bottom on z = 0, and 4 curtain walls.
#[test]
fn until_oblique_rectangle_prism() {
    let profile = square_profile();
    let arrangement = expect_ok(arrange(&profile, None));
    let target = oblique_plane();
    let solid = expect_ok(extrude_until(
        &profile,
        &arrangement,
        SWEEP,
        &Until::Plane(target),
    ));
    assert!(Solid::try_new(solid.boundaries().clone()).is_ok());
    let faces: Vec<Face> = solid.face_iter().cloned().collect();
    assert_eq!(faces.len(), 6, "bottom + 4 walls + the oblique cap");
    let mut cap_count = 0usize;
    let mut bottom_count = 0usize;
    let mut wall_count = 0usize;
    for face in &faces {
        let Surface::Plane(plane) = face.surface() else {
            panic!("every face of the prism must carry a Plane carrier");
        };
        if same_plane(&plane, &target) {
            cap_count += 1;
        } else if plane.origin().z == 0.0 && plane.normal() == Vector3::new(0.0, 0.0, 1.0) {
            bottom_count += 1;
        } else {
            wall_count += 1;
        }
    }
    assert_eq!(cap_count, 1, "the oblique cap in the target plane");
    assert_eq!(bottom_count, 1, "the bottom cap on z = 0");
    assert_eq!(wall_count, 4, "the four curtain walls");
}

/// 3. The oblique fixture has exactly one face whose Plane data equals the
///    target's exactly; its box is the cap polygon's, and the cap's vertices
///    are the machine-checked D4 formula images `v' = v + t(v)·dir` with
///    `t(x,y) = (2 + x) / 2`.
#[test]
fn until_oblique_cap_in_target_plane() {
    let profile = square_profile();
    let arrangement = expect_ok(arrange(&profile, None));
    let target = oblique_plane();
    let solid = expect_ok(extrude_until(
        &profile,
        &arrangement,
        SWEEP,
        &Until::Plane(target),
    ));
    let mut caps: Vec<Face> = Vec::new();
    for face in solid.face_iter() {
        let Surface::Plane(plane) = face.surface() else {
            continue;
        };
        if same_plane(&plane, &target) {
            caps.push(face.clone());
        }
    }
    assert_eq!(caps.len(), 1, "exactly one cap face on the target plane");
    let cap = match caps.first() {
        Some(cap) => cap,
        None => panic!("expected the cap face"),
    };
    let (cap_min, cap_max) = face_box(cap);
    assert_eq!(cap_min, Point3::new(1.0, 1.0, 3.0));
    assert_eq!(cap_max, Point3::new(3.0, 3.0, 5.0));
    let expected = sorted_pts(&[
        Point3::new(1.0, 1.0, 3.0),
        Point3::new(3.0, 1.0, 5.0),
        Point3::new(3.0, 3.0, 5.0),
        Point3::new(1.0, 3.0, 3.0),
    ]);
    let mut actual: Vec<Point3> = Vec::new();
    for wire in cap.boundaries() {
        for edge in wire.edge_iter() {
            let p = edge.front().point();
            if !actual.contains(&p) {
                actual.push(p);
            }
        }
    }
    assert_eq!(sorted_pts(&actual), expected, "the D4 cap polygon vertices");
}

/// 4. A target behind the profile along +z (z = −1) has every boundary
///    `t(p) < 0` — there is no termination, so `Refusal::Empty`.
#[test]
fn until_misses_refuses() {
    let profile = square_profile();
    let arrangement = expect_ok(arrange(&profile, None));
    let target = Until::Plane(Plane::new(
        Point3::new(0.0, 0.0, -1.0),
        Point3::new(1.0, 0.0, -1.0),
        Point3::new(0.0, 1.0, -1.0),
    ));
    match extrude_until(&profile, &arrangement, SWEEP, &target) {
        Err(Refusal::Empty) => {}
        other => panic!("expected Refusal::Empty, got {other:?}"),
    }
}

/// 5. A target plane parallel to the sweep direction (x = 5, `n ⊥ dir`) never
///    terminates: `Refusal::Empty`.
#[test]
fn until_parallel_sweep_refuses() {
    let profile = square_profile();
    let arrangement = expect_ok(arrange(&profile, None));
    let target = Until::Plane(Plane::new(
        Point3::new(5.0, 0.0, 0.0),
        Point3::new(5.0, 1.0, 0.0),
        Point3::new(5.0, 0.0, 1.0),
    ));
    match extrude_until(&profile, &arrangement, SWEEP, &target) {
        Err(Refusal::Empty) => {}
        other => panic!("expected Refusal::Empty, got {other:?}"),
    }
}

/// 6. An L-shaped region (6-edge polygon, one reflex vertex) refuses
///    `UnsupportedEnvelope(NonCanonicalCarrier)`; the convexity predicate is
///    machine-checked to actually fire on the region extraction.
#[test]
fn until_nonconvex_refuses() {
    let profile = l_shape_profile();
    let arrangement = expect_ok(arrange(&profile, None));
    assert!(
        outer_cycle_has_reflex(&arrangement),
        "the L-shape's outer cycle must carry a reflex vertex"
    );
    let target = Until::Plane(oblique_plane());
    match extrude_until(&profile, &arrangement, SWEEP, &target) {
        Err(Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)) => {}
        other => panic!("expected UnsupportedEnvelope(NonCanonicalCarrier), got {other:?}"),
    }
}

/// 7. A circle profile with an OBLIQUE target refuses at the lift (the
///    termination is an Ellipse); the same circle with a PARALLEL target
///    assembles (the parallel case rides the landed extrude, which already
///    handles circle walls).
#[test]
fn until_circle_profile_oblique_refuses() {
    let circle = circle_at(Point3::new(2.0, 2.0, 0.0), 2.0);
    let profile = vec![circle];
    let arrangement = expect_ok(arrange(&profile, None));
    let oblique = Until::Plane(oblique_plane());
    match extrude_until(&profile, &arrangement, SWEEP, &oblique) {
        Err(Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)) => {}
        other => panic!("expected UnsupportedEnvelope(NonCanonicalCarrier), got {other:?}"),
    }
    let parallel = Until::Plane(plane_z2());
    let solid = expect_ok(extrude_until(&profile, &arrangement, SWEEP, &parallel));
    assert!(Solid::try_new(solid.boundaries().clone()).is_ok());
}

/// 8. A parallel projection is a translation: the returned curves are 4
///    `Line`s whose endpoints are the profile's endpoints translated by
///    (0,0,2) exactly.
#[test]
fn project_parallel_is_translation() {
    let profile = square_profile();
    let arrangement = expect_ok(arrange(&profile, None));
    let curves = expect_ok(project_profile(
        &profile,
        &arrangement,
        SWEEP,
        &Until::Plane(plane_z2()),
    ));
    assert_eq!(curves.len(), 4);
    let mut expected: Vec<Point3> = Vec::new();
    for curve in &profile {
        let Curve::Line(Line(a, b)) = curve else {
            panic!("the fixture profile carries only Lines");
        };
        expected.push(*a + SWEEP);
        expected.push(*b + SWEEP);
    }
    let mut actual: Vec<Point3> = Vec::new();
    for curve in &curves {
        let Curve::Line(Line(a, b)) = curve else {
            panic!("a translated line stays a Line");
        };
        assert_eq!(a.z, 2.0, "endpoint {a:?} off the z = 2 plane");
        assert_eq!(b.z, 2.0, "endpoint {b:?} off the z = 2 plane");
        actual.push(*a);
        actual.push(*b);
    }
    assert_eq!(sorted_pts(&actual), sorted_pts(&expected));
}

/// 9. An oblique projection maps each Line edge to the Line between its
///    endpoints' images: 4 `Line`s whose endpoints are certified on Π (the
///    plane equation holds exactly at the dyadic points).
#[test]
fn project_oblique_lines() {
    let profile = square_profile();
    let arrangement = expect_ok(arrange(&profile, None));
    let target = oblique_plane();
    let curves = expect_ok(project_profile(
        &profile,
        &arrangement,
        SWEEP,
        &Until::Plane(target),
    ));
    assert_eq!(curves.len(), 4);
    let n = target.normal();
    let o = target.origin();
    let mut actual: Vec<Point3> = Vec::new();
    for curve in &curves {
        let Curve::Line(Line(a, b)) = curve else {
            panic!("an oblique projection of a line edge stays a Line");
        };
        assert_eq!((*a - o).dot(n), 0.0, "endpoint {a:?} off the target plane");
        assert_eq!((*b - o).dot(n), 0.0, "endpoint {b:?} off the target plane");
        actual.push(*a);
        actual.push(*b);
    }
    let expected = sorted_pts(&[
        Point3::new(1.0, 1.0, 3.0),
        Point3::new(3.0, 1.0, 5.0),
        Point3::new(3.0, 3.0, 5.0),
        Point3::new(1.0, 3.0, 3.0),
    ]);
    let mut actual_dedup: Vec<Point3> = Vec::new();
    for p in &actual {
        if !actual_dedup.contains(p) {
            actual_dedup.push(*p);
        }
    }
    assert_eq!(
        sorted_pts(&actual_dedup),
        expected,
        "the D4 endpoint images"
    );
}
