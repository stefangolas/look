//! Scale-relative tolerance context (BG-TOL-001-TYPE) integration tests.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use truck_base::cgmath64::*;
use truck_base::evidence::{EnvelopeCase, Refusal};
use truck_base::tolerance::{Tolerance, ToleranceCtx, TOLERANCE};

fn ctx(model_scale: f64) -> ToleranceCtx {
    match ToleranceCtx::new(model_scale, 0.000001, 0.000001, 0.000001) {
        Ok(certified) => certified.value,
        Err(_) => {
            unreachable!("a finite positive scale with finite non-negative taus is always accepted")
        }
    }
}

#[test]
fn near_pt_scales_with_the_model() {
    let a = Point3::new(0.0, 0.0, 0.0);
    let b = Point3::new(0.001, 0.0, 0.0);
    assert!(ctx(2000.0).near_pt(a, b));
    assert!(!ctx(100.0).near_pt(a, b));
}

#[test]
fn dimensionless_predicates_do_not_scale() {
    let margin = ctx(1.0).sin_margin();
    for s in [0.001_f64, 1.0, 1000.0] {
        let c = ctx(s);
        assert_eq!(c.sin_margin(), margin);
        assert!(c.is_small_ratio(0.0000005));
        assert!(!c.is_small_ratio(0.002));
    }
}

#[test]
fn scaled_context_preserves_every_predicate() {
    let base = ctx(1.0);
    let mut state: u64 = 0x0000_B6F3_2A4D_0816;
    for _ in 0..500 {
        let d = banded_len(&mut state);
        let u = Vector3::new(
            rand(&mut state) * 2.0 - 1.0,
            rand(&mut state) * 2.0 - 1.0,
            rand(&mut state) * 2.0 - 1.0,
        );
        let v = u * (d / u.magnitude());
        let len = v.magnitude();
        let s = 0.0001 + rand(&mut state) * 999.9;
        let scaled = match base.scaled(s) {
            Ok(certified) => certified.value,
            Err(_) => unreachable!("scaled() refuses only a non-finite or non-positive scale"),
        };
        let q = Point3::new(v.x, v.y, v.z);
        let sq = Point3::new(v.x * s, v.y * s, v.z * s);
        assert_eq!(
            scaled.near_pt(Point3::new(0.0, 0.0, 0.0), sq),
            base.near_pt(Point3::new(0.0, 0.0, 0.0), q)
        );
        assert_eq!(scaled.is_small_len(s * len), base.is_small_len(len));
    }
}

#[test]
fn entity_tolerance_never_below_boundary_tolerance() {
    let c = ctx(1.0);
    for boundary in [0.0, 0.000001, 0.0001, 0.01, 1.0] {
        let entity = c.entity_tau(boundary);
        assert!(entity >= boundary);
        assert!(entity >= c.tau_rep);
    }
    assert_eq!(c.entity_tau(c.tau_rep), c.tau_rep);
}

#[test]
fn non_finite_or_non_positive_scale_is_refused() {
    for scale in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(matches!(
            ToleranceCtx::new(scale, 0.000001, 0.000001, 0.000001),
            Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ChartDegenerate))
        ));
    }
    for bad_tau in [f64::NAN, f64::INFINITY, -0.000001] {
        assert!(matches!(
            ToleranceCtx::new(1.0, bad_tau, 0.000001, 0.000001),
            Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ChartDegenerate))
        ));
    }
    for bad_scale in [0.0, -5.0, f64::NAN, f64::INFINITY] {
        assert!(matches!(
            ctx(1.0).scaled(bad_scale),
            Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ChartDegenerate))
        ));
    }
}

/// A length clearly either below or above the 0.000001 threshold, so the
/// scaled-context test never sits on the boundary where float rounding could
/// flip a comparison.
fn banded_len(state: &mut u64) -> f64 {
    if lcg(state).is_multiple_of(2) {
        0.0000001 + rand(state) * 0.0000003
    } else {
        0.000002 + rand(state) * 0.000006
    }
}

/// Deterministic LCG so a failure is reproducible. Seeds the pseudo-random
/// points, directions and scale factors of the scaled-context test.
fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state
}

/// Uniform in `[0, 1)` from the deterministic LCG.
fn rand(state: &mut u64) -> f64 {
    (lcg(state) % 1_000_000_000) as f64 / 1_000_000_000.0
}

#[test]
fn unscaled_legacy_carries_the_legacy_epsilon() {
    let c = ToleranceCtx::unscaled_legacy();
    assert_eq!(c.model_scale(), 1.0); // H-3: a dimensionless scale of 1.0, so tau * scale is the legacy absolute epsilon
    let just_under = TOLERANCE - 1.0e-12; // H-3: a guard gap below the legacy absolute epsilon, not a tolerance itself
    let just_over = TOLERANCE + 1.0e-12; // H-3: a guard gap above the legacy absolute epsilon, not a tolerance itself
    assert!(c.is_small_ratio(just_under));
    assert!(c.is_small_len(just_under));
    assert!(!c.is_small_ratio(just_over));
    assert!(!c.is_small_len(just_over));
}

#[test]
fn unscaled_legacy_is_never_looser_than_the_legacy_predicate() {
    let c = ToleranceCtx::unscaled_legacy();
    let origin = Point3::new(0.0, 0.0, 0.0); // H-3: the zero reference of the fixed pair set
    let pairs = [
        (origin, Point3::new(0.0, 0.0, 0.0)), // H-3: zero difference
        (origin, Point3::new(TOLERANCE, 0.0, 0.0)), // H-3: zero off-axis components
        (origin, Point3::new(0.5 * TOLERANCE, 0.5 * TOLERANCE, 0.0)), // H-3: half the legacy epsilon per axis
        (origin, Point3::new(TOLERANCE, TOLERANCE, 0.0)), // H-3: zero off-axis component
        (origin, Point3::new(TOLERANCE, TOLERANCE, TOLERANCE)),
        (
            origin,
            Point3::new(1000.0 * TOLERANCE, 0.0, 0.0), // H-3: far beyond the legacy epsilon, so both reject
        ),
    ];
    for (a, b) in pairs {
        if c.near_pt(a, b) {
            assert!(
                a.near(&b),
                "near_pt must never be true where the legacy componentwise predicate is false"
            );
        }
    }
    let corner = Point3::new(TOLERANCE, TOLERANCE, TOLERANCE);
    assert!(
        !c.near_pt(origin, corner),
        "Euclidean magnitude is TOLERANCE * sqrt(3)"
    );
    assert!(
        origin.near(&corner),
        "every coordinate is exactly TOLERANCE, so the componentwise predicate accepts it"
    );
}

#[test]
fn unscaled_legacy_agrees_with_new_at_scale_one() {
    let legacy = ToleranceCtx::unscaled_legacy();
    let scale_one = 1.0; // H-3: a dimensionless scale of 1.0, so tau * scale is the legacy absolute epsilon
    let fresh = match ToleranceCtx::new(scale_one, TOLERANCE, TOLERANCE, TOLERANCE) {
        Ok(certified) => certified.value,
        Err(_) => {
            unreachable!("a finite positive scale with finite non-negative taus is always accepted")
        }
    };
    assert_eq!(legacy.model_scale(), fresh.model_scale());
    assert_eq!(legacy.tau_in, fresh.tau_in);
    assert_eq!(legacy.tau_rep, fresh.tau_rep);
    assert_eq!(legacy.tau_col, fresh.tau_col);
    assert_eq!(legacy.sin_margin(), fresh.sin_margin());
    assert_eq!(legacy.entity_tau(TOLERANCE), fresh.entity_tau(TOLERANCE));
    let samples = [0.0, 0.5 * TOLERANCE, TOLERANCE, 2.0 * TOLERANCE]; // H-3: lengths/ratios spanning the legacy epsilon boundary
    for x in samples {
        assert_eq!(legacy.is_small_len(x), fresh.is_small_len(x));
        assert_eq!(legacy.is_small_ratio(x), fresh.is_small_ratio(x));
    }
    let a = Point3::new(0.0, 0.0, 0.0); // H-3: the zero reference of the sample set
    for b in [
        Point3::new(TOLERANCE, 0.0, 0.0), // H-3: zero off-axis components
        Point3::new(TOLERANCE, TOLERANCE, 0.0), // H-3: zero off-axis component
        Point3::new(TOLERANCE, TOLERANCE, TOLERANCE),
    ] {
        assert_eq!(legacy.near_pt(a, b), fresh.near_pt(a, b));
    }
}

#[test]
fn one_sided_margins_match_the_legacy_threshold() {
    let ctx = ToleranceCtx::unscaled_legacy();
    assert_eq!(ctx.length_margin(), TOLERANCE);
    assert_eq!(ctx.ratio_margin(), TOLERANCE);
    let t0 = 0.5; // H-3: a dimensionless curve parameter, not a tolerance itself
    for t in [
        0.0,
        0.1,
        0.25,
        t0 - 2.0 * TOLERANCE, // H-3: a guard gap below the threshold, not a tolerance itself
        t0 - TOLERANCE,
        t0,
        t0 + TOLERANCE,
        t0 + 2.0 * TOLERANCE, // H-3: a guard gap above the threshold, not a tolerance itself
        1.0,
    ] {
        assert_eq!(t < t0 + ctx.ratio_margin(), t < t0 + TOLERANCE);
        assert_eq!(t < t0 + ctx.length_margin(), t < t0 + TOLERANCE);
    }
    let far_below = 0.0; // H-3: a parameter far below t0, where the symmetric predicate answers differently
    assert!(
        far_below < t0 + ctx.ratio_margin(),
        "the one-sided comparison is true for a parameter far below t0"
    );
    assert!(
        !ctx.is_small_ratio(far_below - t0),
        "the symmetric predicate takes an absolute value and is false there"
    );
}

#[test]
fn near_points_agrees_with_near_pt_on_point3() {
    let ctx = ToleranceCtx::unscaled_legacy();
    let origin = Point3::new(0.0, 0.0, 0.0); // H-3: the zero reference of the fixed pair set
    let pairs = [
        (origin, Point3::new(0.0, 0.0, 0.0)), // H-3: zero difference
        (origin, Point3::new(0.5 * TOLERANCE, 0.0, 0.0)), // H-3: half the legacy epsilon per axis
        (origin, Point3::new(TOLERANCE, 0.0, 0.0)), // H-3: exactly the legacy epsilon
        (origin, Point3::new(2.0 * TOLERANCE, 0.0, 0.0)), // H-3: twice the legacy epsilon
        (origin, Point3::new(TOLERANCE, TOLERANCE, 0.0)), // H-3: diagonal within the Euclidean epsilon
        (origin, Point3::new(TOLERANCE, TOLERANCE, TOLERANCE)),
    ];
    for (a, b) in pairs {
        assert_eq!(ctx.near_points(a, b), ctx.near_pt(a, b));
    }
}

#[test]
fn near_points_works_in_two_dimensions() {
    let a = Point2::new(0.0, 0.0); // H-3: the zero reference of the fixed pair
    let b = Point2::new(0.001, 0.0); // H-3: a model-space length of 0.001, not a tolerance itself
    assert!(ctx(2000.0).near_points(a, b));
    assert!(!ctx(100.0).near_points(a, b));
}
