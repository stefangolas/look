//! BG-CAD-P5-REVOLVE — the packet's nine required acceptance tests.
//!
//! The revolve of line-edge profiles via the carrier table, exercised on the
//! extrude.rs test pattern: the rectangle flagship, the trapezoid carrier
//! table, the D5 metamorphic gate (revolve ≅ analytic primitive), the partial
//! angle, the refusal families, and the downstream-consumability census.

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

use std::f64::consts::{FRAC_PI_2, FRAC_PI_8, TAU};
use truck_base::evidence::{Budget, EnvelopeCase, Outcome, Refusal};
use truck_geometry::arrange::{arrange, Arrangement};
use truck_modeling::cad::solid_bounding_box;
use truck_modeling::extrude::extrude_profile;
use truck_modeling::revolve::revolve_profile;
use truck_modeling::{
    Curve, Line, Matrix4, Point3, Processor, Solid, Surface, TrimmedCurve, UnitCircle, Vector4,
    TOLERANCE,
};

/// Unwraps an `Outcome` via `match` + `panic` so the deny lints stay satisfied
/// (the recognize.rs test-module precedent).
fn expect_ok<T>(r: Outcome<T>) -> T {
    match r {
        Ok(ok) => ok.value,
        Err(refusal) => panic!("expected a certified value, got {refusal:?}"),
    }
}

/// The working copy of a profile: each point (x, 0, z) mapped to (x, z, 0) —
/// the frame the arrangement and `revolve_profile` consume.
fn work_copy(profile: &[Curve]) -> Vec<Curve> {
    profile
        .iter()
        .map(|c| match c {
            Curve::Line(Line(a, b)) => {
                Curve::Line(Line(Point3::new(a.x, a.z, 0.0), Point3::new(b.x, b.z, 0.0)))
            }
            Curve::Circle(p) => {
                let m = *p.transform();
                let swap = |v: Vector4| Vector4::new(v.x, v.z, v.y, v.w);
                Curve::Circle(Processor::with_transform(
                    *p.entity(),
                    Matrix4 {
                        x: swap(m.x),
                        y: swap(m.y),
                        z: swap(m.z),
                        w: swap(m.w),
                    },
                ))
            }
            _ => panic!("test profiles are Line/Circle only"),
        })
        .collect()
}

/// Arranges an xz-plane profile's working copy and returns both.
fn arrange_profile(profile: &[Curve]) -> (Vec<Curve>, Arrangement) {
    let working = work_copy(profile);
    let arrangement = expect_ok(arrange(&working, None));
    (working, arrangement)
}

/// The flagship rectangle: x ∈ [1, 3], z ∈ [0, 2] (r1 = 1, r2 = 3, h = 2),
/// CCW in the (x, z) plane as seen from +y.
fn rectangle_profile() -> Vec<Curve> {
    vec![
        Curve::Line(Line(Point3::new(1.0, 0.0, 0.0), Point3::new(3.0, 0.0, 0.0))),
        Curve::Line(Line(Point3::new(3.0, 0.0, 0.0), Point3::new(3.0, 0.0, 2.0))),
        Curve::Line(Line(Point3::new(3.0, 0.0, 2.0), Point3::new(1.0, 0.0, 2.0))),
        Curve::Line(Line(Point3::new(1.0, 0.0, 2.0), Point3::new(1.0, 0.0, 0.0))),
    ]
}

/// The trapezoid: bottom z = 0 spanning x ∈ [1, 3], right edge slanted down
/// from (3, 0, 0) to (2, 0, 2), top z = 2 spanning x ∈ [1, 2], left edge at
/// x = 1.
fn trapezoid_profile() -> Vec<Curve> {
    vec![
        Curve::Line(Line(Point3::new(1.0, 0.0, 0.0), Point3::new(3.0, 0.0, 0.0))),
        Curve::Line(Line(Point3::new(3.0, 0.0, 0.0), Point3::new(2.0, 0.0, 2.0))),
        Curve::Line(Line(Point3::new(2.0, 0.0, 2.0), Point3::new(1.0, 0.0, 2.0))),
        Curve::Line(Line(Point3::new(1.0, 0.0, 2.0), Point3::new(1.0, 0.0, 0.0))),
    ]
}

/// A full circle of radius 1 in the xz-plane (y = 0) at center (2, 0, 2).
fn xz_circle() -> Curve {
    Curve::Circle(Processor::with_transform(
        TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, TAU)),
        Matrix4 {
            x: Vector4::new(1.0, 0.0, 0.0, 0.0),
            y: Vector4::new(0.0, 0.0, 1.0, 0.0),
            z: Vector4::new(0.0, 1.0, 0.0, 0.0),
            w: Vector4::new(2.0, 0.0, 2.0, 1.0),
        },
    ))
}

/// A full circle of radius `r` about the z-axis at height 0 (the annulus
/// profile for the metamorphic extrude).
fn axis_circle(r: f64) -> Curve {
    Curve::Circle(Processor::with_transform(
        TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, TAU)),
        Matrix4 {
            x: Vector4::new(r, 0.0, 0.0, 0.0),
            y: Vector4::new(0.0, r, 0.0, 0.0),
            z: Vector4::new(0.0, 0.0, 1.0, 0.0),
            w: Vector4::new(0.0, 0.0, 0.0, 1.0),
        },
    ))
}

/// The carrier multiset of a solid: `(kind, data)` per face, sorted.
fn carrier_tags(solid: &Solid) -> Vec<(u8, [f64; 4])> {
    let mut tags: Vec<(u8, [f64; 4])> = Vec::new();
    for face in solid.face_iter() {
        let tag = match face.surface() {
            Surface::Plane(plane) => (0, [plane.origin().z, 0.0, 0.0, 0.0]),
            Surface::Cylinder(cyl) => (
                1,
                [cyl.center().x, cyl.center().y, cyl.center().z, cyl.radius()],
            ),
            Surface::Cone(cone) => (
                2,
                [
                    cone.apex().x,
                    cone.apex().y,
                    cone.apex().z,
                    cone.half_angle(),
                ],
            ),
            _ => panic!("unexpected carrier in {solid:?}"),
        };
        tags.push(tag);
    }
    let bits = |v: &[f64; 4]| {
        [
            v[0].to_bits(),
            v[1].to_bits(),
            v[2].to_bits(),
            v[3].to_bits(),
        ]
    };
    tags.sort_by(|a, b| a.0.cmp(&b.0).then(bits(&a.1).cmp(&bits(&b.1))));
    tags
}

/// 1. The full-turn revolve of the flagship rectangle is a tube: 4 faces (the
///    inner/outer cylinders and the bottom/top annuli — the two end caps
///    coincide at the interior profile region), box exactly [−3, 3]² × [0, 2].
#[test]
fn revolve_rectangle_full_turn_is_tube() {
    let profile = rectangle_profile();
    let (_, arrangement) = arrange_profile(&profile);
    let solid = expect_ok(revolve_profile(&profile, &arrangement, TAU));
    assert_eq!(solid.face_iter().count(), 4);
    let mut budget = Budget::new(0, 0, 0);
    let hull = expect_ok(solid_bounding_box(&solid, &mut budget));
    assert_eq!(hull.min(), Point3::new(-3.0, -3.0, 0.0));
    assert_eq!(hull.max(), Point3::new(3.0, 3.0, 2.0));
    assert!(Solid::try_new(solid.boundaries().clone()).is_ok());
}

/// 2. The trapezoid's full-turn carriers are exactly {Plane ×2, Cylinder ×1,
///    Cone ×1}, and each Cone/Cylinder's derived data matches the edge it came
///    from: the cylinder is r = 1 on the axis, the cone has apex (0, 0, 6) and
///    half angle atan(1/2).
#[test]
fn revolve_carriers_are_analytic() {
    let profile = trapezoid_profile();
    let (_, arrangement) = arrange_profile(&profile);
    let solid = expect_ok(revolve_profile(&profile, &arrangement, TAU));
    let mut planes = 0usize;
    let mut cylinders = 0usize;
    let mut cones = 0usize;
    let mut cylinder_data: Option<(Point3, f64)> = None;
    let mut cone_data: Option<(Point3, f64)> = None;
    for face in solid.face_iter() {
        match face.surface() {
            Surface::Plane(_) => planes += 1,
            Surface::Cylinder(cyl) => {
                cylinders += 1;
                cylinder_data = Some((cyl.center(), cyl.radius()));
            }
            Surface::Cone(cone) => {
                cones += 1;
                cone_data = Some((cone.apex(), cone.half_angle()));
            }
            _ => panic!("unexpected carrier"),
        }
    }
    assert_eq!(planes, 2);
    assert_eq!(cylinders, 1);
    assert_eq!(cones, 1);
    let (center, radius) = match cylinder_data {
        Some(data) => data,
        None => panic!("expected a cylinder"),
    };
    assert_eq!(center, Point3::new(0.0, 0.0, 0.0));
    assert_eq!(radius, 1.0);
    let (apex, half_angle) = match cone_data {
        Some(data) => data,
        None => panic!("expected a cone"),
    };
    assert_eq!(apex, Point3::new(0.0, 0.0, 6.0));
    assert!((half_angle - f64::atan(0.5)).abs() <= TOLERANCE);
}

/// 3. The D5 metamorphic gate: the full-turn revolve of the rectangle is
///    carrier- and box-equal to the landed extrude of the annulus r1 = 1,
///    r2 = 3, height 2.
#[test]
fn revolve_matches_extruded_annulus() {
    let profile = rectangle_profile();
    let (_, arrangement) = arrange_profile(&profile);
    let tube = expect_ok(revolve_profile(&profile, &arrangement, TAU));

    let annulus = vec![axis_circle(1.0), axis_circle(3.0)];
    let annulus_arrangement = expect_ok(arrange(&annulus, None));
    let extruded = expect_ok(extrude_profile(&annulus, &annulus_arrangement, 2.0));

    assert_eq!(tube.face_iter().count(), extruded.face_iter().count());
    assert_eq!(carrier_tags(&tube), carrier_tags(&extruded));

    let mut budget = Budget::new(0, 0, 0);
    let tube_hull = expect_ok(solid_bounding_box(&tube, &mut budget));
    let mut budget = Budget::new(0, 0, 0);
    let extruded_hull = expect_ok(solid_bounding_box(&extruded, &mut budget));
    assert_eq!(tube_hull.min(), extruded_hull.min());
    assert_eq!(tube_hull.max(), extruded_hull.max());
    assert_eq!(tube_hull.min(), Point3::new(-3.0, -3.0, 0.0));
    assert_eq!(tube_hull.max(), Point3::new(3.0, 3.0, 2.0));
}

/// 4. The partial revolve of the flagship rectangle by π/2 is a valid solid
///    with 6 faces (2 end caps + 4 walls), box exactly [0, 3] × [0, 3] × [0, 2].
#[test]
fn revolve_partial_angle_valid() {
    let profile = rectangle_profile();
    let (_, arrangement) = arrange_profile(&profile);
    let solid = expect_ok(revolve_profile(&profile, &arrangement, FRAC_PI_2));
    assert_eq!(solid.face_iter().count(), 6);
    let mut budget = Budget::new(0, 0, 0);
    let hull = expect_ok(solid_bounding_box(&solid, &mut budget));
    // The derived box is [0, 3] × [0, 3] × [0, 2] to the representation
    // tolerance: the landed interval enclosure rounds outward (BG-ENC-001),
    // which puts the quarter-arc's exact minimum at cos(π/2) below zero by a
    // float epsilon.
    let lo = hull.min();
    let hi = hull.max();
    for (got, want) in [
        (lo.x, 0.0),
        (lo.y, 0.0),
        (lo.z, 0.0),
        (hi.x, 3.0),
        (hi.y, 3.0),
        (hi.z, 2.0),
    ] {
        assert!(
            (got - want).abs() <= TOLERANCE,
            "box coordinate {got} is not the nominal {want}"
        );
    }
    assert!(Solid::try_new(solid.boundaries().clone()).is_ok());
}

/// 5. A profile with one vertex at x = −1 refuses
///    `UnsupportedEnvelope(NonCanonicalCarrier)` — the revolve map
///    double-covers there (REV-AXIS-CROSS).
#[test]
fn revolve_axis_crossing_refuses() {
    let profile = vec![
        Curve::Line(Line(
            Point3::new(-1.0, 0.0, 0.0),
            Point3::new(3.0, 0.0, 0.0),
        )),
        Curve::Line(Line(Point3::new(3.0, 0.0, 0.0), Point3::new(3.0, 0.0, 2.0))),
        Curve::Line(Line(
            Point3::new(3.0, 0.0, 2.0),
            Point3::new(-1.0, 0.0, 2.0),
        )),
        Curve::Line(Line(
            Point3::new(-1.0, 0.0, 2.0),
            Point3::new(-1.0, 0.0, 0.0),
        )),
    ];
    let (_, arrangement) = arrange_profile(&profile);
    match revolve_profile(&profile, &arrangement, TAU) {
        Err(Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)) => {}
        other => panic!("expected UnsupportedEnvelope(NonCanonicalCarrier), got {other:?}"),
    }
}

/// 6. An edge endpoint exactly at x = 0 refuses `Refusal::Collapsed`.
#[test]
fn revolve_axis_touch_refuses_collapsed() {
    let profile = vec![
        Curve::Line(Line(Point3::new(0.0, 0.0, 0.0), Point3::new(3.0, 0.0, 0.0))),
        Curve::Line(Line(Point3::new(3.0, 0.0, 0.0), Point3::new(3.0, 0.0, 2.0))),
        Curve::Line(Line(Point3::new(3.0, 0.0, 2.0), Point3::new(0.0, 0.0, 2.0))),
        Curve::Line(Line(Point3::new(0.0, 0.0, 2.0), Point3::new(0.0, 0.0, 0.0))),
    ];
    let (_, arrangement) = arrange_profile(&profile);
    match revolve_profile(&profile, &arrangement, TAU) {
        Err(Refusal::Collapsed(..)) => {}
        other => panic!("expected the Collapsed refusal, got {other:?}"),
    }
}

/// 7. angle 0, angle −1, and angle TAU + π/8 each refuse `Refusal::Empty`.
#[test]
fn revolve_angle_bounds_refuse() {
    let profile = rectangle_profile();
    let (_, arrangement) = arrange_profile(&profile);
    for bad in [0.0, -1.0, TAU + FRAC_PI_8] {
        match revolve_profile(&profile, &arrangement, bad) {
            Err(Refusal::Empty) => {}
            other => panic!("expected the Empty refusal for angle {bad}, got {other:?}"),
        }
    }
}

/// 8. A profile region whose boundary carries a Circle edge refuses
///    `UnsupportedEnvelope(NonCanonicalCarrier)` at the lift (table 6.3 is
///    Tier 2).
#[test]
fn revolve_circle_profile_refuses() {
    let profile = vec![xz_circle()];
    let (_, arrangement) = arrange_profile(&profile);
    match revolve_profile(&profile, &arrangement, TAU) {
        Err(Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)) => {}
        other => panic!("expected UnsupportedEnvelope(NonCanonicalCarrier), got {other:?}"),
    }
}

/// 9. The tube downstream-consumes. truck-shapeops is NOT a truck-modeling
///    dependency (a dev-dependency cycle — Cargo rejects it), so the boolean
///    call cannot run from this test crate; per the packet's fallback, this
///    asserts `Solid::try_new` re-validation plus a tessellation-free
///    face/wire census. The downstream-consumability invariant is re-asserted
///    at the P8 battery.
#[test]
fn revolve_result_survives_boolean() {
    let profile = rectangle_profile();
    let (_, arrangement) = arrange_profile(&profile);
    let solid = expect_ok(revolve_profile(&profile, &arrangement, TAU));
    assert!(Solid::try_new(solid.boundaries().clone()).is_ok());
    let mut planes = 0usize;
    let mut cylinders = 0usize;
    for face in solid.face_iter() {
        match face.surface() {
            Surface::Plane(_) => planes += 1,
            Surface::Cylinder(_) => cylinders += 1,
            _ => panic!("unexpected carrier"),
        }
        let wires = face.absolute_boundaries();
        assert_eq!(wires.len(), 2, "each wall carries two boundary wires");
        for wire in wires {
            assert_eq!(wire.len(), 1, "each wire is a single circle self-loop");
        }
    }
    assert_eq!(planes, 2);
    assert_eq!(cylinders, 2);
}
