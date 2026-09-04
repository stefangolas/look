#![cfg_attr(not(debug_assertions), deny(warnings))]
#![deny(clippy::all, rust_2018_idioms)]
#![deny(clippy::unwrap_used)]
#![warn(
    missing_docs,
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]

//! The Rump / Ogita / Oishi residual fallback (CC-001-BANDED, theory §1 P1
//! fallback): the certified enclosure for dense systems OUTSIDE the
//! banded-TP class that the no-pivot banded fast path in [`crate::construct::banded`]
//! does not serve (Hermite ribbons, radius-law splines).
//!
//! # Proof identity and bound derivation
//!
//! The caller supplies a float preconditioner `R` with `R·A ≈ I` (a plain
//! 2×2 / 3×3 adjugate inverse suffices; this module never builds
//! preconditioners). Writing `E = I − R·A`, the identity `R·A = I − E` is
//! exact. For any matrix `A₀ ∈ A` and right-hand side `b₀ ∈ b` admitted by the
//! interval data, and any exact solution `x*` of `A₀ x* = b₀`, the error
//! `e = x* − x̂` satisfies
//!
//! ```text
//! e = R·(b₀ − A₀·x̂) + (I − R·A₀)·e = R·r₀ + E·e
//! ```
//!
//! so `‖e‖ ≤ ‖R·r₀‖ + ‖E‖·‖e‖`, and whenever `η := ‖E‖_∞ < 1`,
//!
//! ```text
//! ‖x* − x̂‖_∞ ≤ ‖R·r₀‖_∞ / (1 − η).
//! ```
//!
//! The module computes `η` as an upper bound on `‖I − R·A‖_∞` in interval
//! arithmetic and computes the residual enclosure `r = b − A·x̂`, which
//! encloses every `r₀`; `‖R·r‖_∞ / (1 − η)` is then a certified bound on every
//! admissible `x*`. `η ≥ 1` means the residual bound does not contract and the
//! method refuses ([`ConstructRefusal::ConditioningBelowThreshold`]) instead
//! of returning a widening enclosure.
//!
//! All interval work is outward-rounded `CertifiedInterval` arithmetic; every
//! `‖·‖_∞` sum is accumulated in fixed ascending order and rounded outward
//! (`next_up` per addition), so the method is deterministic and the returned
//! bound is a certified upper bound.
//!
//! **H-1.** This module carries no `unwrap`, no `expect`, and no `panic!`, and
//! adds no module-level `allow`.

use crate::construct::refusal::ConstructRefusal;
use crate::construct::Interval;

/// The Rump / Ogita / Oishi residual fallback for dense systems outside the
/// banded-TP class (theory §1 P1 fallback; consumers: Hermite ribbons,
/// radius-law splines).
///
/// `a` is the interval system matrix, `b` the interval right-hand side,
/// `r_inv` the CALLER-supplied float preconditioner `R ≈ A⁻¹`, and `x_hat` an
/// approximate float solution. When the computed `η = ‖I − R·A‖_∞` is `≥ 1`,
/// the method refuses [`ConstructRefusal::ConditioningBelowThreshold`]. When
/// `η < 1` it forms the residual enclosure `r = b − A·x̂`, computes the
/// certified bound `‖x − x̂‖_∞ ≤ ‖R·r‖_∞ / (1 − η)`, and returns the enclosure
/// `x̂ ± bound` (outward-rounded endpoints).
pub fn residual_solve_dense<const N: usize>(
    a: &[[Interval; N]; N],
    r_inv: &[[f64; N]; N],
    x_hat: &[f64; N],
    b: &[Interval; N],
) -> Result<[Interval; N], ConstructRefusal> {
    // η := ‖I − R·A‖_∞, an upper bound in interval arithmetic, fixed order.
    let mut eta = 0.0_f64;
    for i in 0..N {
        let mut row_sum = 0.0_f64;
        for j in 0..N {
            let mut e = if i == j {
                Interval::point(1.0)
            } else {
                Interval { lo: 0.0, hi: 0.0 }
            };
            for k in 0..N {
                let ra = Interval::point(r_inv[i][k]).mul(&a[k][j]);
                e = e.sub(&ra);
            }
            row_sum = (row_sum + abs_sup(&e)).next_up();
        }
        if !row_sum.is_finite() {
            eta = f64::INFINITY;
            break;
        }
        if row_sum > eta {
            eta = row_sum;
        }
    }
    if eta >= 1.0 {
        return Err(ConstructRefusal::ConditioningBelowThreshold);
    }

    // The residual enclosure r = b − A·x̂, fixed ascending order per row.
    let mut r = [Interval { lo: 0.0, hi: 0.0 }; N];
    for i in 0..N {
        let mut acc = b[i];
        for j in 0..N {
            acc = acc.sub(&a[i][j].mul(&Interval::point(x_hat[j])));
        }
        r[i] = acc;
    }

    // The certified numerator ‖R·r‖_∞, fixed ascending order per row.
    let mut num = 0.0_f64;
    for i in 0..N {
        let mut row_sum = 0.0_f64;
        for j in 0..N {
            let rr = Interval::point(r_inv[i][j]).mul(&r[j]);
            row_sum = (row_sum + abs_sup(&rr)).next_up();
        }
        if !row_sum.is_finite() {
            return Err(ConstructRefusal::ConditioningBelowThreshold);
        }
        if row_sum > num {
            num = row_sum;
        }
    }

    // ‖x − x̂‖_∞ ≤ num / (1 − η); widen the denominator downward and the
    // quotient upward so the bound stays certified.
    let denom = (1.0 - eta).next_down();
    if denom <= 0.0 {
        return Err(ConstructRefusal::ConditioningBelowThreshold);
    }
    let bound = (num / denom).next_up();

    let mut out = [Interval { lo: 0.0, hi: 0.0 }; N];
    for j in 0..N {
        out[j] = Interval {
            lo: (x_hat[j] - bound).next_down(),
            hi: (x_hat[j] + bound).next_up(),
        };
    }
    Ok(out)
}

/// An outward-rounded upper bound on `sup |x|` over the interval.
#[inline]
fn abs_sup(value: &Interval) -> f64 {
    value.lo.abs().max(value.hi.abs()).next_up()
}
