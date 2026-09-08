//! ADM-L5-RECIPROCAL conformance: the certified reciprocal-power primitive
//! (`truck-evidence::num::reciprocal::certified_reciprocal_power`), exercised
//! through the public API.
//!
//! The four required tests:
//!
//! - `constant_weight_exact_no_remainder` — a constant weight `W ≡ w` is
//!   returned EXACTLY: `Q_0 = w^{-p}` with `m = 0`, degenerate coefficients,
//!   and `eps_m = 0` (the hull collapses, `ρ = 0`, the series is the single
//!   `k = 0` term);
//! - `linear_weight_against_closed_form` — for a linear weight the returned
//!   `Q_m` agrees with the truncated binomial series evaluated directly (the
//!   closed form), and for `p = 1` with the geometric partial-sum identity
//!   `Q_m = (1 − (−δ)^{m+1}) / W`;
//! - `tail_bound_brackets_true_error_on_adversarial_fixtures` — a
//!   near-vanishing weight (`W ≈ 1e-3`) where the certified geometric tail
//!   bound `eps_m` must bracket the true uniform error, verified by
//!   high-precision interval evaluation on a dense dyadic grid (including both
//!   hull endpoints, where `|δ|` is maximal);
//! - `non_positive_weight_bracket_refuses` — a bracket that is not strictly
//!   positive refuses typed (`UnsupportedEnvelope(NonPositiveNurbsWeight)`),
//!   never a division by an uncertified sign.

use inari::Interval;
use truck_base::evidence::{EnvelopeCase, Refusal};
use truck_evidence::num::reciprocal::certified_reciprocal_power;

/// A degenerate interval at `x`.
fn iv(x: f64) -> Interval {
    Interval::try_from((x, x)).unwrap()
}

/// Whether an interval is the degenerate `[x, x]`.
fn degenerate(i: Interval, x: f64) -> bool {
    i.inf() == x && i.sup() == x
}

/// de Casteljau evaluation of a Bernstein coefficient list at `t ∈ [0, 1]`,
/// in interval arithmetic (sound for interval coefficients).
fn bern_eval(coeffs: &[Interval], t: f64) -> Interval {
    let a = iv(1.0 - t);
    let b = iv(t);
    let mut level: Vec<Interval> = coeffs.to_vec();
    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len() - 1);
        for w in level.windows(2) {
            next.push(a * w[0] + b * w[1]);
        }
        level = next;
    }
    level[0]
}

/// `x^p` in interval arithmetic (small integer `p`, positive base).
fn interval_pow(x: Interval, p: u32) -> Interval {
    let mut r = x;
    for _ in 1..p {
        r *= x;
    }
    r
}

/// The supremum distance between two intervals: `max |a − b|`.
fn sup_distance(a: Interval, b: Interval) -> f64 {
    (a.sup() - b.inf()).max(b.sup() - a.inf()).max(0.0)
}

/// The certified closed-form geometric tail
/// `w₋^{-p} − w₀^{-p}·Σ_{k≤m} C(k+p−1, p−1)·ρ^k` in plain `f64`, for
/// independent confirmation that the returned `eps` is the real geometric-sum
/// bound (not a fabricated number).
fn closed_form_tail(w_lo: f64, w_hi: f64, p: u32, m: usize) -> f64 {
    let w0 = 0.5 * (w_lo + w_hi);
    let rho = (w_hi - w_lo) / (w_hi + w_lo);
    let mut s = 1.0;
    let mut term = 1.0;
    for k in 1..=m {
        term *= rho * (k as f64 + p as f64 - 1.0) / k as f64;
        s += term;
    }
    w_lo.powf(-(p as f64)) - w0.powf(-(p as f64)) * s
}

/// The truncated binomial series evaluated directly at `t` for the linear
/// weight `W(t) = a + (b − a)t`: `Q_m(t) = w₀^{-p} Σ_{k≤m} (−1)^k
/// C(k+p−1, p−1) δ(t)^k`, `δ = (W − w₀)/w₀`. This is the closed form the
/// module's Bernstein construction must reproduce.
fn closed_form_linear(t: f64, a: f64, b: f64, p: u32, m: usize) -> f64 {
    let w0 = 0.5 * (a + b);
    let w = a + (b - a) * t;
    let delta = (w - w0) / w0;
    let mut sum = 0.0;
    let mut c = 1.0; // C(k+p-1, p-1)
    let mut x = 1.0; // δ^k
    for k in 0..=m {
        let sign = if k % 2 == 0 { 1.0 } else { -1.0 };
        sum += sign * c * x;
        c *= (k as f64 + p as f64) / (k as f64 + 1.0);
        x *= delta;
    }
    w0.powf(-(p as f64)) * sum
}

#[test]
fn constant_weight_exact_no_remainder() {
    // W ≡ 8, p = 3: the hull collapses to [8, 8], ρ = 0, so the primitive
    // returns Q_0 = 8^{-3} = 1/512 EXACTLY with m = 0 and eps_m = 0.
    let out = certified_reciprocal_power(&[8.0], 3, 1.0e-12).unwrap();
    assert_eq!(out.value.degree, 0);
    assert_eq!(out.value.m, 0);
    assert_eq!(out.value.q.len(), 1);
    // 8^{-3} = 1/512 = 0.001953125 is exactly representable; the coefficient
    // must be degenerate at it.
    let c = out.value.q[0];
    assert!(
        degenerate(c, 8.0_f64.powi(-3)),
        "coefficient {c:?} not exact"
    );
    assert!(
        degenerate(out.value.eps, 0.0),
        "eps {out:?} not exactly zero"
    );
    assert_eq!(out.value.bracket, (8.0, 8.0));
}

#[test]
fn linear_weight_against_closed_form() {
    // W(t) = (1−t) + 3t = 1 + 2t, so w₀ = 2 (a power of two), δ = t − 1/2 and
    // every coefficient of the truncated series is exactly representable.
    let p = 3u32;
    let target = 1.0e-12;
    let out = certified_reciprocal_power(&[1.0, 3.0], p, target).unwrap();
    assert_eq!(out.value.degree, 1);
    assert!(out.value.m >= 1);
    assert_eq!(out.value.q.len(), out.value.m + 1);
    assert!(out.value.eps.sup() <= target);
    // The certified bound is the real geometric-sum closed form.
    let tail = closed_form_tail(1.0, 3.0, p, out.value.m);
    assert!(
        out.value.eps.inf() <= tail + 1.0e-9 && out.value.eps.sup() >= tail - 1.0e-9,
        "eps {:?} does not enclose the closed-form tail {tail}",
        out.value.eps
    );

    // Q_m must reproduce the truncated binomial series (the closed form) on a
    // dense dyadic grid, to high precision.
    let n = 512usize;
    let mut worst = 0.0_f64;
    for i in 0..=n {
        let t = i as f64 / n as f64;
        let eval = bern_eval(&out.value.q, t).mid();
        let ref_ = closed_form_linear(t, 1.0, 3.0, p, out.value.m);
        worst = worst.max((eval - ref_).abs());
    }
    assert!(
        worst < 1.0e-7,
        "Q_m deviates from the closed form by {worst}"
    );

    // p = 1: the geometric partial sum has the closed form
    // Q_m = w₀^{-1}·Σ_{k≤m}(−δ)^k = (1 − (−δ)^{m+1})/W, an independent
    // identity the returned polynomial must satisfy.
    let out1 = certified_reciprocal_power(&[1.0, 3.0], 1, target).unwrap();
    let mut worst1 = 0.0_f64;
    for i in 0..=n {
        let t = i as f64 / n as f64;
        let w = 1.0 + 2.0 * t;
        let delta = (w - 2.0) / 2.0;
        let closed = (1.0 - (-delta).powi(out1.value.m as i32 + 1)) / w;
        let eval = bern_eval(&out1.value.q, t).mid();
        worst1 = worst1.max((eval - closed).abs());
    }
    assert!(
        worst1 < 1.0e-7,
        "p=1 polynomial deviates from the geometric closed form by {worst1}"
    );
}

#[test]
fn tail_bound_brackets_true_error_on_adversarial_fixtures() {
    // Near-vanishing linear weight: W ranges over [1/256, 3/256] ~ 1e-3, so
    // W^{-3} reaches ~1.7e7 and the hull spread ρ = 1/2 is exactly dyadic.
    // The certified geometric tail bound must bracket the true uniform error
    // of the returned polynomial, verified by interval evaluation on a dense
    // dyadic grid that includes both hull endpoints (where |δ| is maximal).
    let a = 1.0 / 256.0;
    let b = 3.0 / 256.0;
    let p = 3u32;
    let target = 1.0;
    let out = certified_reciprocal_power(&[a, b], p, target).unwrap();
    assert_eq!(out.value.degree, 1);
    assert!(out.value.m >= 1);
    assert!(out.value.eps.sup() <= target);
    assert!(out.value.eps.inf() >= 0.0);

    // Sanity: the certified bound is the closed-form geometric tail.
    let tail = closed_form_tail(a, b, p, out.value.m);
    assert!(
        out.value.eps.inf() <= tail * (1.0 + 1.0e-6) + 1.0e-3
            && out.value.eps.sup() >= tail * (1.0 - 1.0e-6) - 1.0e-3,
        "eps {:?} does not enclose the closed-form tail {tail}",
        out.value.eps
    );

    // The true pointwise error |W^{-p} − Q_m^⋆|, bounded by interval
    // evaluation of both factors on each grid point, must never exceed the
    // certified bound.
    let n = 8192usize;
    let mut worst = 0.0_f64;
    for i in 0..=n {
        let t = i as f64 / n as f64;
        let w = iv(a + (b - a) * t);
        let a_iv = interval_pow(w.recip(), p);
        let b_iv = bern_eval(&out.value.q, t);
        worst = worst.max(sup_distance(a_iv, b_iv));
    }
    // The certified bound eps.sup() covers the exact Q_m^⋆; the measured
    // point bound additionally carries the (tiny) interval width of the
    // coefficient enclosures and the grid evaluation, so a small slack is the
    // honest comparison.
    assert!(
        worst <= out.value.eps.sup() + 1.0e-3,
        "true error {worst} exceeds the certified bound {:?}",
        out.value.eps
    );
    // The measured error must be a substantial fraction of the bound — i.e.
    // the bound brackets the truth tightly and is not a vacuous overestimate.
    assert!(
        worst > out.value.eps.sup() * 0.01,
        "bound {:?} is vacuous against measured error {worst}",
        out.value.eps
    );
}

#[test]
fn non_positive_weight_bracket_refuses() {
    // A bracket that reaches zero or goes negative refuses typed, never a
    // division by an uncertified sign.
    let err_zero = certified_reciprocal_power(&[0.0, 2.0], 3, 1.0e-6).unwrap_err();
    assert!(matches!(
        err_zero,
        Refusal::UnsupportedEnvelope(EnvelopeCase::NonPositiveNurbsWeight)
    ));
    let err_neg = certified_reciprocal_power(&[-1.0, 2.0], 3, 1.0e-6).unwrap_err();
    assert!(matches!(
        err_neg,
        Refusal::UnsupportedEnvelope(EnvelopeCase::NonPositiveNurbsWeight)
    ));
    // Degenerate inputs refuse Empty (typed, not a panic).
    assert!(matches!(
        certified_reciprocal_power(&[1.0, 2.0], 0, 1.0e-6).unwrap_err(),
        Refusal::Empty
    ));
    assert!(matches!(
        certified_reciprocal_power(&[1.0, 2.0], 3, f64::NAN).unwrap_err(),
        Refusal::Empty
    ));
}
