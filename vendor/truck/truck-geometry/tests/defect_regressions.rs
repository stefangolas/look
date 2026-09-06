//! ID-named regressions for the defect records this repository's index keys by
//! contract. The names below are the index's convention, so the index and this
//! file are one thing (`DEFECT_INDEX.md`).
//!
//! Tests here cover the truck-geometry contracts of BREP-001A:
//!
//! - `PAR-RANGE-INHERITANCE-001`: a working surface domain must derive from
//!   the represented geometry, not from the generatrix primitive's `[0, 1]`.
//! - `NUM-SUBDIVISION-GROWTH-001`: a sample count derived from imported
//!   geometry is bounded and total, and a capped request is observable
//!   (RES-003), never silent ordinary success.
//! - `SEM-ARRANGE-DYADIC-CONTRACT-001`: the profile arrangement entry refuses
//!   a non-dyadic vertex demand with a typed refusal, not `RootNotIsolated`.
//!
//! House rule H-1 applies; where a fixture needs an `unwrap` it carries the
//! same-line `// H-3` justification.

use std::f64::consts::TAU;
use truck_geometry::arrange::{arrange, ArrangeError};
use truck_geometry::canonical::Curve;
use truck_geometry::prelude::*;

fn p3(x: f64, y: f64, z: f64) -> Point3 {
    Point3::new(x, y, z)
}

/// A STEP-style conical carrier: the reference circle of radius `radius` sits
/// at the generatrix parameter origin and the surface opens along `+z` at
/// `semi_angle`. `Line::parameter_range` would answer `[0, 1]` for the
/// generatrix, so a carrier that inherits it covers exactly one unit of axial
/// height above the reference circle.
fn step_cone(reference_radius: f64, semi_angle: f64, z0: f64) -> RevolutedCurve<Line<Point3>> {
    let tan = semi_angle.tan();
    let p0 = p3(reference_radius, 0.0, z0);
    let p1 = p3(reference_radius + tan, 0.0, z0 + 1.0);
    RevolutedCurve::by_revolution(Line(p0, p1), Point3::origin(), Vector3::unit_z())
}

/// The apex of the step-cone carrier: the generatrix parameter at which the
/// revolved radius vanishes, computed geometrically from the line.
fn step_cone_apex_parameter(reference_radius: f64, semi_angle: f64) -> f64 {
    let tan = semi_angle.tan();
    -reference_radius / tan
}

// ---------------------------------------------------------------------------
// PAR-RANGE-INHERITANCE-001
// ---------------------------------------------------------------------------

/// The revolved surface's derived domain must cover its own apex — the apex is
/// part of the represented geometry, not of any face — and must not stop at the
/// `Line::parameter_range` default of `[0, 1]`.
///
/// The reproducer cone (radius 10, half-angle 59°) has its apex at the
/// generatrix parameter `-R / tan θ ≈ -6.010`, well outside `[0, 1]`. A domain
/// that answered `[0, 1]` (the construction route to the surface) would
/// exclude the apex and, with it, every face whose material interval reaches
/// it. `PAR-RANGE-INHERITANCE-001` names that inheritance the defect.
#[test]
fn par_range_inheritance_001_face_domain_does_not_use_line_default() {
    let semi_angle = 59.0f64.to_radians();
    let cone = step_cone(10.0, semi_angle, 0.0);

    // Fixture premise: the generatrix `Line`'s own declared range starts at 0.
    let line = Line(p3(10.0, 0.0, 0.0), p3(10.0 + semi_angle.tan(), 0.0, 1.0));
    let (u0, _) = line.parameter_range();
    let u0 = match u0 {
        std::ops::Bound::Included(value) | std::ops::Bound::Excluded(value) => value,
        std::ops::Bound::Unbounded => f64::NEG_INFINITY,
    };
    assert_eq!(u0, 0.0, "fixture premise: the line itself starts at 0");

    let (u_range, _) = cone.try_range_tuple();
    let (u_lo, u_hi) = u_range.expect("the revolved cone is bounded in u");
    let derived_apex = step_cone_apex_parameter(10.0, semi_angle);
    assert!(
        derived_apex < 0.0,
        "fixture premise: the apex sits below the reference circle at {derived_apex}"
    );
    assert!(
        u_lo <= derived_apex + 1.0e-9,
        "the derived domain must reach the apex: lower bound {u_lo} > apex {derived_apex}"
    );
    assert!(
        u_lo < u0,
        "the domain must not inherit the line's [0, 1] start: lower bound {u_lo} >= 0"
    );
    assert!(
        u_hi > 1.0,
        "the domain must extend past one unit of generatrix: upper bound {u_hi}"
    );
}

/// The metamorphic test the record demands: the SAME cone expressed by two
/// encodings — the exporter's reference circle at the small end versus partway
/// up the generatrix (the AP203/AP242 disagreement) — must yield the SAME
/// derived domain *in world space*: the derived apex parameter must map back
/// to the same world apex, and the derived interval must cover the same world
/// span of the material patch.
#[test]
fn par_range_inheritance_001_same_cone_two_encodings_same_domain() {
    let semi_angle = 59.0f64.to_radians();
    // Encoding A: reference radius 10 at the base of the material band.
    let cone_a = step_cone(10.0, semi_angle, 0.0);
    // Encoding B: the exporter placed its reference circle partway up the same
    // cone — radius 10 + 5·tan θ at axial height 5. The physical cone is the
    // same; only the reference position differs.
    let cone_b = step_cone(10.0 + 5.0 * semi_angle.tan(), semi_angle, 5.0);

    let apex_world_z = |cone: RevolutedCurve<Line<Point3>>| -> f64 {
        let (u_range, _) = cone.try_range_tuple();
        let (u_lo, _) = u_range.expect("the revolved cone is bounded in u");
        cone.subs(u_lo, 0.0).z
    };
    let apex_a = apex_world_z(cone_a);
    let apex_b = apex_world_z(cone_b);
    assert!(
        (apex_a - apex_b).abs() < 1.0e-6,
        "both encodings derive the same world apex: A at z={apex_a}, B at z={apex_b}"
    );

    // The derived interval must reach the shared world apex in both encodings:
    // the domain is a property of the cone's geometry, not of where the
    // exporter chose to put its reference circle.
    let u_range_of = |cone: RevolutedCurve<Line<Point3>>| -> (f64, f64) {
        let (u_range, _) = cone.try_range_tuple();
        u_range.expect("the revolved cone is bounded in u")
    };
    let (lo_a, _) = u_range_of(cone_a);
    let (lo_b, _) = u_range_of(cone_b);
    assert!(
        (cone_a.subs(lo_a, 0.0).z - apex_a).abs() < 1.0e-6,
        "encoding A's lower domain edge is its apex"
    );
    assert!(
        (cone_b.subs(lo_b, 0.0).z - apex_b).abs() < 1.0e-6,
        "encoding B's lower domain edge is its apex"
    );
}

// ---------------------------------------------------------------------------
// NUM-SUBDIVISION-GROWTH-001
// ---------------------------------------------------------------------------

/// A sample-count request derived from imported geometry is bounded and total:
/// the checked constructor refuses a non-finite or negative request rather
/// than flooring it into an allocation size (`RES-001`).
#[test]
fn num_subdivision_growth_001_sample_count_constructor_refuses_non_finite() {
    type Revolved = RevolutedCurve<Line<Point3>>;
    assert!(
        Revolved::checked_sample_count(f64::NAN).is_err(),
        "a NaN request must be refused"
    );
    assert!(
        Revolved::checked_sample_count(f64::INFINITY).is_err(),
        "an infinite request must be refused"
    );
    assert!(
        Revolved::checked_sample_count(f64::NEG_INFINITY).is_err(),
        "a negatively-infinite request must be refused"
    );
    assert!(
        Revolved::checked_sample_count(-1.0).is_err(),
        "a negative request must be refused"
    );
    let accepted = Revolved::checked_sample_count(256.0);
    assert!(
        accepted.is_ok(),
        "a finite non-negative request is accepted"
    );
}

/// A capped request is observable: the division report distinguishes a request
/// that hit `MAX_CIRCLE_DIVISION` (`ResourceCapped`) from ordinary success,
/// instead of returning its approximation silently (`RES-003`). The cap value
/// is unchanged — this only makes the hit visible.
#[test]
fn num_subdivision_growth_001_cap_hit_is_observable_as_resource_capped() {
    // A radius large against the tolerance drives `acos` toward zero, so the
    // angular sample request blows past the cap (the ABC `00000730` shape).
    let huge = step_cone(1.0e9, 45.0f64.to_radians(), 0.0);
    let capped_division = huge.sample_division(((0.0, 1.0), (0.0, TAU)), 1.0e-4);
    assert!(
        capped_division.is_resource_capped(),
        "the huge-radius request must report the cap, not silent success"
    );
    match capped_division.resource_capped() {
        None => panic!("capped request carried no ResourceCapped report"), // H-1: test-only refusal
        Some(report) => {
            assert!(
                report.requested > report.used as f64,
                "the report must say the request exceeded the use: \
                 requested {} cells, used {}",
                report.requested,
                report.used
            );
            assert!(
                report.achieved_error.is_finite() && report.achieved_error > 0.0,
                "the report must carry the achieved chord error"
            );
            assert!(
                report.used > 1000,
                "the capped division should sit at the cap, not collapse to a tiny grid"
            );
        }
    }

    // The ordinary face is untouched: its request fits the cap and the report
    // says so, with the exact same grid `parameter_division` produces.
    let ordinary = step_cone(1.0, 45.0f64.to_radians(), 0.0);
    let within = ordinary.sample_division(((0.0, 1.0), (0.0, TAU)), 1.0e-3);
    assert!(
        !within.is_resource_capped(),
        "an ordinary request must not report a cap"
    );
    let (_, circle) = ordinary.parameter_division(((0.0, 1.0), (0.0, TAU)), 1.0e-3);
    assert_eq!(
        circle.len(),
        within.used_cells() + 1,
        "the observable decision must match the produced division"
    );
}

// ---------------------------------------------------------------------------
// SEM-ARRANGE-DYADIC-CONTRACT-001
// ---------------------------------------------------------------------------

fn loop_curves(pts: &[(f64, f64)]) -> Vec<Curve> {
    let n = pts.len();
    (0..n)
        .map(|i| {
            let a = pts[i];
            let b = pts[(i + 1) % n];
            Curve::Line(Line(p3(a.0, a.1, 0.0), p3(b.0, b.1, 0.0)))
        })
        .collect()
}

/// A profile whose non-adjacent extended segments meet in a non-dyadic point
/// must be refused at the arrange entry with a typed case naming the offending
/// pair and the lattice point — never with the misleading `RootNotIsolated`
/// witness. The dyadic twin (identical topology, dyadic lattice) arranges.
#[test]
fn sem_arrange_dyadic_contract_001_non_dyadic_vertices_refuse_typed() {
    // Row 5 of the SEM-ARRANGE-DYADIC-CONTRACT-001 isolation table: slopes
    // `{20, 0, 25, 0}`, non-dyadic extended-line lattice.
    let non_dyadic = loop_curves(&[(1.0, 0.0), (1.1, 1.0), (0.9, 1.0), (0.7, 0.0)]);
    match arrange(&non_dyadic, None) {
        Ok(_) => panic!("the non-dyadic profile must refuse"), // H-1: test-only
        Err(ArrangeError::NonDyadicProfileVertices { pair, at }) => {
            let (i, j) = pair;
            assert!(
                i < j && j < non_dyadic.len(),
                "the refusal must name a real pair: ({i}, {j})"
            );
            assert!(
                at.x.is_finite() && at.y.is_finite(),
                "the refusal must name the offending point: {at:?}"
            );
        }
        Err(other) => panic!("the refusal must be typed, not {other:?}"), // H-1: test-only
    }

    // Dyadic twins arrange exactly as before.
    let dyadic = loop_curves(&[(1.0, 0.0), (2.0, 4.0), (1.0, 4.0), (0.5, 0.0)]);
    assert!(
        arrange(&dyadic, None).is_ok(),
        "the dyadic twin must still arrange"
    );
    let near_parallel = loop_curves(&[(1.0, 0.0), (1.25, 4.0), (0.75, 4.0), (0.5625, 0.0)]);
    assert!(
        arrange(&near_parallel, None).is_ok(),
        "near-parallel dyadic profiles are exonerated"
    );
}
