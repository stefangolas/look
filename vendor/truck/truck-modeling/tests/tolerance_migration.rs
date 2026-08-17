//! BG-TOL-001-TOPO-MOD tolerance migration tests: the one-sided range guard and
//! the scale-free invariant every `param` classification in the site table
//! depends on.

#![deny(clippy::unwrap_used)]

use truck_base::tolerance::{ToleranceCtx, TOLERANCE};

fn ctx_at(scale: f64) -> ToleranceCtx {
    match ToleranceCtx::new(scale, TOLERANCE, TOLERANCE, TOLERANCE) {
        Ok(certified) => certified.value,
        Err(_) => ToleranceCtx::unscaled_legacy(),
    }
}

/// The one-sided rewrite `t < t0 + ctx.ratio_margin()` must agree with the
/// legacy `t < t0 + TOLERANCE` for every `t`, including values well below
/// `t0` — where `is_small_ratio(t - t0)`, which takes an absolute value,
/// answers differently. A test that only samples `t` near `t0` passes against
/// the bug and is worthless.
#[test]
fn one_sided_range_guard_keeps_its_legacy_answers() {
    let ctx = ToleranceCtx::unscaled_legacy();
    let t0 = 0.5; // H-3: a dimensionless curve parameter, not a tolerance itself
                  // Sweep from far below t0 to far above it so the one-sided guard is
                  // exercised where the symmetric predicate diverges, not just at the edge.
    let ts = [
        0.0,                  // H-3: a parameter far below t0
        0.1,                  // H-3: a parameter far below t0
        0.25,                 // H-3: a parameter far below t0
        t0 - 2.0 * TOLERANCE, // H-3: a guard gap below the threshold, not a tolerance itself
        t0,
        t0 + 2.0 * TOLERANCE, // H-3: a guard gap above the threshold, not a tolerance itself
        0.75,                 // H-3: a parameter far above t0
        1.0,                  // H-3: a parameter far above t0
    ];
    for t in ts {
        assert_eq!(
            t < t0 + ctx.ratio_margin(),
            t < t0 + TOLERANCE,
            "one-sided guard diverged from the legacy threshold at t = {t}"
        );
    }
    let far_below = 0.0; // H-3: a parameter far below t0, where the symmetric predicate answers differently
    assert!(
        far_below < t0 + ctx.ratio_margin(),
        "the one-sided guard admits a parameter far below t0"
    );
    assert!(
        !ctx.is_small_ratio(far_below - t0),
        "is_small_ratio takes an absolute value and rejects a parameter far below t0"
    );
}

/// Dimensionless predicates (`ratio_margin`, `is_small_ratio`) must be
/// identical at every model scale, while model-space predicates
/// (`length_margin`, `is_small_len`) must scale with it.
#[test]
fn param_sites_are_unaffected_by_model_scale() {
    let scales = [0.001, 0.1, 1.0, 10.0, 1000.0]; // H-3: dimensionless model-scale factors spread across orders of magnitude
    let ratios = [0.0, 0.5 * TOLERANCE, TOLERANCE, 2.0 * TOLERANCE]; // H-3: dimensionless ratio quantities bracketing tau_rep
    let lengths = [0.5 * TOLERANCE, TOLERANCE, 2.0 * TOLERANCE]; // H-3: model-space length quantities bracketing tau_rep * model_scale
    let reference = ctx_at(1.0);

    for &ratio in &ratios {
        let baseline = reference.is_small_ratio(ratio);
        for &scale in scales.iter().skip(1) {
            assert_eq!(
                ctx_at(scale).is_small_ratio(ratio),
                baseline,
                "is_small_ratio must be scale-free at model_scale {scale}"
            );
            assert_eq!(
                ctx_at(scale).ratio_margin(),
                reference.ratio_margin(),
                "ratio_margin must be scale-free at model_scale {scale}"
            );
        }
    }

    // A length that is within tolerance at scale 1.0 stays within tolerance
    // at every other scale only after being multiplied by the model factor.
    for &scale in scales.iter().skip(1) {
        for &length in &lengths {
            assert_eq!(
                ctx_at(scale).is_small_len(scale * length),
                reference.is_small_len(length),
                "is_small_len must scale with the model at scale {scale}, length {length}"
            );
            assert_eq!(
                ctx_at(scale).length_margin(),
                scale * reference.length_margin(),
                "length_margin must scale with the model at scale {scale}"
            );
        }
    }

    // Unlike is_small_ratio, is_small_len at a fixed length must change with
    // the model scale.
    let differs = lengths.iter().any(|&length| {
        let baseline = reference.is_small_len(length);
        scales
            .iter()
            .skip(1)
            .any(|&scale| ctx_at(scale).is_small_len(length) != baseline)
    });
    assert!(differs, "is_small_len must scale with the model");
}
