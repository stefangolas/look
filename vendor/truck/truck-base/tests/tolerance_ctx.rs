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
use truck_base::tolerance::ToleranceCtx;

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
