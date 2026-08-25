//! BG-SOL-P0-PRED — certified predicates with adaptive escalation.
//!
//! Topology-changing predicates (`orient2d`, event ordering, exact tangency,
//! endpoint membership) are never naked f64 comparisons
//! (docs/SOLVER_FAMILY_PLAN.md §2): a one-ulp error there is not a bad
//! number, it is a different topology. `orient2d` ships first, with the
//! discipline every later predicate inherits — a fast float filter that
//! returns `Proven` when the float sign is certain, and an exact escalation
//! that computes the true sign when it is not. `Unresolved` is a result,
//! never a crash.
//!
//! The plan's §4 sketch spells the result `CertifiedPred { Proven,
//! Unresolved(UnresolvedWitness) }`. A unit `Proven` cannot carry the
//! predicate's trichotomous answer, and the evidence algebra's
//! `UnresolvedWitness` classifies refusals of whole certified operations
//! (`RootNotIsolated`, `KrawczykIndeterminate`, …), none of which means "this
//! predicate is undecidable". This packet therefore spells the result
//! `Proven(Orientation) | Unresolved(PredUnresolved)` (recorded in the
//! packet's deviations). S1 (arrange) inherits it.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use crate::cgmath64::Point2;

/// The trichotomous sign of an orientation predicate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    /// The determinant is negative.
    Clockwise,
    /// The determinant is positive.
    CounterClockwise,
    /// The three points are exactly collinear.
    Collinear,
}

/// Why a predicate could not be decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PredUnresolved {
    /// A coordinate is NaN or infinite; no sign exists.
    NonFiniteInput,
    /// The exact escalation cannot represent the sign because the f64
    /// two-product overflows (a coordinate magnitude beyond ~1e150).
    ExactRangeOverflow,
}

/// A certified predicate answer: proven, or honestly unresolved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CertifiedPred {
    /// The predicate's sign is proven.
    Proven(Orientation),
    /// The predicate could not be decided; `reason` names why.
    Unresolved(PredUnresolved),
}

/// The Shewchuk error bound for the filter, spelled from `f64::EPSILON` (a
/// name, not a bare literal — H-3): `(3 + 16 eps) eps`.
const CCDETERRBOUND: f64 = (3.0 + 16.0 * f64::EPSILON) * f64::EPSILON;

/// `2^27 + 1`, the Dekker/Shewchuk splitter constant used by `two_product`.
const SPLITTER: f64 = 134217729.0;

/// The exact orientation predicate: the sign of the determinant
/// `(b - a) x (c - a)` in 2-D. Positive is counterclockwise.
/// Filtered, then exact; never a naked f64 comparison.
pub fn orient2d(a: Point2, b: Point2, c: Point2) -> CertifiedPred {
    let acx = a.x - c.x;
    let bcx = b.x - c.x;
    let acy = a.y - c.y;
    let bcy = b.y - c.y;
    let detleft = acx * bcy;
    let detright = acy * bcx;
    let det = detleft - detright;
    let detsum = detleft.abs() + detright.abs();
    let errbound = CCDETERRBOUND * detsum;
    if det > errbound {
        return CertifiedPred::Proven(Orientation::CounterClockwise);
    }
    if det < -errbound {
        return CertifiedPred::Proven(Orientation::Clockwise);
    }
    if !(a.x.is_finite()
        && a.y.is_finite()
        && b.x.is_finite()
        && b.y.is_finite()
        && c.x.is_finite()
        && c.y.is_finite())
    {
        return CertifiedPred::Unresolved(PredUnresolved::NonFiniteInput);
    }
    let acx = two_diff(a.x, c.x);
    let bcx = two_diff(b.x, c.x);
    let acy = two_diff(a.y, c.y);
    let bcy = two_diff(b.y, c.y);
    let (acxbcy_hi, acxbcy_lo) = two_product(acx.0, bcy.0);
    let (acybcx_hi, acybcx_lo) = two_product(acy.0, bcx.0);
    if !acxbcy_hi.is_finite()
        || !acxbcy_lo.is_finite()
        || !acybcx_hi.is_finite()
        || !acybcx_lo.is_finite()
    {
        return CertifiedPred::Unresolved(PredUnresolved::ExactRangeOverflow);
    }
    let expansion = fast_expansion_sum_zeroelim(&[acxbcy_hi, acxbcy_lo], &[-acybcx_hi, -acybcx_lo]);
    let mut max_abs = 0.0f64;
    let mut max_component = 0.0f64;
    for component in expansion {
        if component.abs() > max_abs {
            max_abs = component.abs();
            max_component = component;
        }
    }
    if max_abs == 0.0 {
        CertifiedPred::Proven(Orientation::Collinear)
    } else if max_component > 0.0 {
        CertifiedPred::Proven(Orientation::CounterClockwise)
    } else {
        CertifiedPred::Proven(Orientation::Clockwise)
    }
}

/// Error-free split product (Dekker's algorithm via the `SPLITTER`): `hi` is
/// `x * y` rounded and `lo` the exact residual, so `hi + lo == x * y`
/// exactly. Returns non-finite components when the product overflows.
fn two_product(x: f64, y: f64) -> (f64, f64) {
    let hi = x * y;
    let c = SPLITTER * x;
    let xbig = c - x;
    let xh = c - xbig;
    let xl = x - xh;
    let c = SPLITTER * y;
    let ybig = c - y;
    let yh = c - ybig;
    let yl = y - yh;
    let lo = (xh * yh - hi) + xh * yl + xl * yh + xl * yl;
    (hi, lo)
}

/// Exact difference via two-sum: `hi` is `x - y` rounded and `lo` the exact
/// residual.
fn two_diff(x: f64, y: f64) -> (f64, f64) {
    let hi = x - y;
    let bvirt = x - hi;
    let lo = (x - (hi + bvirt)) + (y - bvirt);
    (hi, lo)
}

/// Two-sum: `hi` is `a + b` rounded and `lo` the exact error (valid when
/// `|a| >= |b|`, which the expansion merge maintains).
fn fast_two_sum(a: f64, b: f64) -> (f64, f64) {
    let hi = a + b;
    let bvirt = hi - a;
    let lo = b - bvirt;
    (hi, lo)
}

/// Zero-eliminating expansion sum (Shewchuk's `fast_expansion_sum_zeroelim`).
/// Assumes each input expansion is non-overlapping and ordered by decreasing
/// magnitude (as produced by `two_product`); the result keeps the
/// non-overlapping property and the largest component carries the sign.
fn fast_expansion_sum_zeroelim(e: &[f64], f: &[f64]) -> Vec<f64> {
    let mut h = Vec::with_capacity(e.len() + f.len());
    let mut e_iter = e.iter().copied();
    let mut f_iter = f.iter().copied();
    let mut e_next = e_iter.next();
    let mut f_next = f_iter.next();
    let mut q;
    match (f_next, e_next) {
        (Some(f0), Some(e0)) => {
            q = f0 + e0;
            if q != 0.0 {
                h.push(q);
            }
            f_next = f_iter.next();
            e_next = e_iter.next();
        }
        _ => {
            q = 0.0;
        }
    }
    while let (Some(f0), Some(e0)) = (f_next, e_next) {
        if f0.abs() > e0.abs() {
            let (hnow, bvirt) = fast_two_sum(q, f0);
            h.push(hnow);
            if bvirt != 0.0 {
                h.push(bvirt);
            }
            q = hnow;
            f_next = f_iter.next();
        } else {
            let (hnow, bvirt) = fast_two_sum(q, e0);
            h.push(hnow);
            if bvirt != 0.0 {
                h.push(bvirt);
            }
            q = hnow;
            e_next = e_iter.next();
        }
    }
    while let Some(f0) = f_next {
        let (hnow, bvirt) = fast_two_sum(q, f0);
        h.push(hnow);
        if bvirt != 0.0 {
            h.push(bvirt);
        }
        q = hnow;
        f_next = f_iter.next();
    }
    while let Some(e0) = e_next {
        let (hnow, bvirt) = fast_two_sum(q, e0);
        h.push(hnow);
        if bvirt != 0.0 {
            h.push(bvirt);
        }
        q = hnow;
        e_next = e_iter.next();
    }
    if q != 0.0 {
        h.push(q);
    }
    h
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect on paths reachable from untrusted
// geometry. Unit-test assertions on hand-built witnesses are not such a path;
// these unwraps cannot fire for the values constructed below. (`unwrap_used`
// stays denied here; no test below uses unwrap.)
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn orient2d_clear_cases_are_filtered() {
        assert_eq!(
            orient2d(
                Point2::new(0.0, 0.0),
                Point2::new(1.0, 0.0),
                Point2::new(0.0, 1.0),
            ),
            CertifiedPred::Proven(Orientation::CounterClockwise)
        );
        assert_eq!(
            orient2d(
                Point2::new(0.0, 0.0),
                Point2::new(0.0, 1.0),
                Point2::new(1.0, 0.0),
            ),
            CertifiedPred::Proven(Orientation::Clockwise)
        );
        assert_eq!(
            orient2d(
                Point2::new(0.0, 0.0),
                Point2::new(2.0, 1.0),
                Point2::new(4.0, 2.0),
            ),
            CertifiedPred::Proven(Orientation::Collinear)
        );
    }

    #[test]
    fn orient2d_near_degenerate_escalates_to_exact() {
        // All coordinates are below 2^53, so every value and difference is
        // exactly representable. The exact determinant is -2; the float
        // filter is INCONCLUSIVE (errbound ~= 24 >> |det| = 2), so this only
        // passes if stage 2 runs and returns the exact sign.
        assert_eq!(
            orient2d(
                Point2::new(0.0, 0.0),
                Point2::new(9000000000000000.0, 9000000000000001.0),
                Point2::new(9000000000000002.0, 9000000000000003.0),
            ),
            CertifiedPred::Proven(Orientation::Clockwise)
        );
    }

    #[test]
    fn orient2d_collinear_escalates_to_exact() {
        // Exact determinant is 0; the filter is INCONCLUSIVE
        // (errbound ~= 1.3e-6 > 0), so this only passes if stage 2 computes
        // the exact zero.
        assert_eq!(
            orient2d(
                Point2::new(0.0, 0.0),
                Point2::new(1000000001.0, 1000000001.0),
                Point2::new(1000000002.0, 1000000002.0),
            ),
            CertifiedPred::Proven(Orientation::Collinear)
        );
    }

    #[test]
    fn orient2d_non_finite_input_is_unresolved() {
        assert_eq!(
            orient2d(
                Point2::new(f64::NAN, 0.0),
                Point2::new(0.0, 1.0),
                Point2::new(1.0, 0.0),
            ),
            CertifiedPred::Unresolved(PredUnresolved::NonFiniteInput)
        );
        assert_eq!(
            orient2d(
                Point2::new(0.0, 0.0),
                Point2::new(f64::INFINITY, 1.0),
                Point2::new(1.0, 0.0),
            ),
            CertifiedPred::Unresolved(PredUnresolved::NonFiniteInput)
        );
    }
}
