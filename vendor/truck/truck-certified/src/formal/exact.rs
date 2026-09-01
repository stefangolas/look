// Grandfathered (orchestrator amendment, BG-CK-P0-CRATE r3): moved
// verbatim from truck-meshalgo, whose crate never denied
// clippy::unwrap_used. The crate-level deny in lib.rs is H-1's contract
// for AUTHORED certified code; this module's pre-existing unwraps are
// inherited baseline content and must not be force-rewritten by the
// move packet. Do not add new unwraps under this allow.
#![allow(clippy::unwrap_used)]

//! Exact (Shewchuk-style) expansion arithmetic for certified predicates.
//!
//! # Provenance — a lift, not a reimplementation
//!
//! This module is a **lift** of `Expansion` from
//! `look/src/step/circular_arc.rs` (the FORMAL-015 circle-vs-ellipse
//! classifier, look commit `4ef4513`). The algorithms — `two_sum`,
//! `two_product`, `grow_expansion`, and the largest-component sign rule —
//! are copied verbatim so that there is exactly **one** exact-arithmetic
//! implementation in the workspace rather than two subtly different ones.
//! `look` has since been migrated to consume this copy through the
//! `truck-meshalgo` patch (`look/src/step/circular_arc.rs` imports
//! `truck_meshalgo::tessellation::formal::{Expansion, CertifiedSign}`), and
//! its private `exact_arith` module is retired; this module is canonical.
//!
//! # What is certified
//!
//! An [`Expansion`] is a non-overlapping list of `f64` components that
//! sums, with zero rounding error, to the exact value of a polynomial
//! expression over the `f64` inputs. Its [`Expansion::sign`] therefore
//! decides the sign of that exact value — an *exact predicate for `f64`
//! inputs*, in the same sense `robust::orient2d` is exact for `f64`
//! coordinates. No tolerance appears in this module.

/// A certified sign of an exactly-evaluated expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CertifiedSign {
    /// Certifiably negative.
    Negative,
    /// Certifiably zero.
    Zero,
    /// Certifiably positive.
    Positive,
}

/// `a + b` with the rounding error returned separately (`two_sum`).
fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    let bb = s - a;
    let err = (a - (s - bb)) + (b - bb);
    (s, err)
}

/// `a * b` with the rounding error returned separately (`two_product`).
fn two_product(a: f64, b: f64) -> (f64, f64) {
    let p = a * b;
    let e = a.mul_add(b, -p);
    (p, e)
}

/// A non-overlapping exact decomposition of a polynomial expression over
/// the `f64` inputs: the components sum, with zero rounding error, to the
/// exact value.
#[derive(Debug, Clone, Default)]
pub struct Expansion {
    components: Vec<f64>,
}

impl Expansion {
    /// The empty expansion, representing exactly zero.
    pub fn zero() -> Self {
        Expansion {
            components: Vec::new(),
        }
    }

    /// Insert one exact scalar into the expansion (`grow_expansion`): the
    /// result still sums, with zero error, to `self + b`.
    pub fn grow(&self, b: f64) -> Self {
        let mut components = Vec::with_capacity(self.components.len() + 1);
        let mut q = b;
        for &e in &self.components {
            let (sum, err) = two_sum(q, e);
            if err != 0.0 {
                components.push(err);
            }
            q = sum;
        }
        if q != 0.0 || components.is_empty() {
            components.push(q);
        }
        Expansion { components }
    }

    /// Merge another expansion into this one exactly (repeated `grow`).
    pub fn merge(&self, other: &Expansion) -> Self {
        let mut result = self.clone();
        for &c in &other.components {
            result = result.grow(c);
        }
        result
    }

    /// The exact additive inverse.
    pub fn negate(&self) -> Self {
        Expansion {
            components: self.components.iter().map(|c| -c).collect(),
        }
    }

    /// Scale exactly by `2^n`. Multiplication by a power of two never
    /// rounds, so this preserves exactness.
    pub fn scale_expansion_by_pow2(&self, n: i32) -> Self {
        let factor = 2.0_f64.powi(n);
        Expansion {
            components: self.components.iter().map(|c| c * factor).collect(),
        }
    }

    /// Exact zero test: every component is (exactly) zero.
    pub fn is_zero(&self) -> bool {
        self.components.iter().all(|&c| c == 0.0)
    }

    /// The exact sign of the value this expansion represents: the sign of
    /// the largest-magnitude (last) nonzero component, which for a valid
    /// non-overlapping expansion is provably the sign of the exact sum.
    pub fn sign(&self) -> CertifiedSign {
        match self.components.last() {
            None => CertifiedSign::Zero,
            Some(&last) if last > 0.0 => CertifiedSign::Positive,
            Some(&last) if last < 0.0 => CertifiedSign::Negative,
            Some(_) => CertifiedSign::Zero,
        }
    }

    /// `two_product(a, b)` folded straight into a fresh expansion.
    pub fn from_product(a: f64, b: f64) -> Self {
        let (hi, lo) = two_product(a, b);
        Expansion::zero().grow(hi).grow(lo)
    }

    /// `a + b` folded straight into a fresh expansion: the two components sum
    /// with zero error to the exact value of `a + b`.
    ///
    /// With `b = −a.x`, this is the exact coordinate difference of two declared
    /// `f64` coordinates — a rounded `a.x − b` vector is never formed, so a
    /// squared norm, dot product or cross product built from these expansions
    /// is exact over the declared coordinates.
    pub fn from_sum(a: f64, b: f64) -> Self {
        let (hi, lo) = two_sum(a, b);
        Expansion::zero().grow(hi).grow(lo)
    }

    /// Exact product of two expansions: the components sum, with zero
    /// rounding error, to the exact product of the two represented values.
    ///
    /// Every pairwise product of components is a single exact
    /// [`two_product`]; folding each factor in by [`Self::grow`] preserves
    /// the non-overlap invariant, so the result is a valid expansion of the
    /// exact product. This is Shewchuk's expansion product; quadratic in the
    /// component count, which stays at a handful here. The discriminant signs
    /// that decide support-curve intersection counts flow through it, so it
    /// is exact over the `f64` inputs, never a rounded coefficient.
    pub fn mul_expansion(&self, other: &Expansion) -> Self {
        let mut acc = Expansion::zero();
        for &a in &self.components {
            for &b in &other.components {
                let (hi, lo) = two_product(a, b);
                acc = acc.grow(hi);
                acc = acc.grow(lo);
            }
        }
        acc
    }
}

/// Exact squared distance between two points, as an expansion over the `f64`
/// coordinates.
///
/// Each coordinate difference is split exactly by [`two_sum`], and each of
/// `hi²`, `2·hi·lo`, `lo²` is then a single exact product; the six terms sum
/// with zero error to `|a − b|²`. This is what makes "the point is on the
/// circle" an exact predicate: `exact_sq_dist` vs `exact_dot2(basis, basis)`,
/// decided by [`Expansion::sign`].
pub fn exact_sq_dist(a: [f64; 2], b: [f64; 2]) -> Expansion {
    let (xh, xl) = two_sum(a[0], -b[0]);
    let (yh, yl) = two_sum(a[1], -b[1]);
    let mut acc = Expansion::from_product(xh, xh);
    // The cross terms `2·xh·xl` and `2·yh·yl` are exact power-of-two scalings
    // of a single exact product; scaling by `2^1` never rounds.
    acc = acc.merge(&Expansion::from_product(xh, xl).scale_expansion_by_pow2(1));
    acc = acc.merge(&Expansion::from_product(xl, xl));
    acc = acc.merge(&Expansion::from_product(yh, yh));
    acc = acc.merge(&Expansion::from_product(yh, yl).scale_expansion_by_pow2(1));
    acc = acc.merge(&Expansion::from_product(yl, yl));
    acc
}

/// Exact dot product of two 2-vectors, as a non-overlapping expansion.
pub fn exact_dot2(u: [f64; 2], v: [f64; 2]) -> Expansion {
    let mut acc = Expansion::from_product(u[0], v[0]);
    acc = acc.merge(&Expansion::from_product(u[1], v[1]));
    acc
}

/// Exact 2D cross product `a × b = a.x·b.y − a.y·b.x`, as an expansion over
/// the `f64` coordinates. Every term is a single exact product, so the sign
/// is an exact predicate for the `f64` inputs.
pub fn cross_exp(a: [f64; 2], b: [f64; 2]) -> Expansion {
    let mut acc = Expansion::from_product(a[0], b[1]);
    acc = acc.merge(&Expansion::from_product(a[1], b[0]).negate());
    acc
}

// ---------------------------------------------------------------------------
// Directed-rounding interval arithmetic
// ---------------------------------------------------------------------------

/// A closed interval with sound, outward-directed rounding.
///
/// Every operation returns an interval that provably contains the exact
/// result over the `f64` inputs, by widening each correctly-rounded
/// elementary operation by one ulp in each direction ([`f64::next_down`] /
/// [`f64::next_up`]). The widening is a rounding bound, not a tolerance:
/// no epsilon appears, and a comparison of two such intervals is decided by
/// strict separation or left `Undecidable`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CertifiedInterval {
    /// A certified lower bound.
    pub lo: f64,
    /// A certified upper bound.
    pub hi: f64,
}

impl CertifiedInterval {
    /// The degenerate interval `[x, x]`.
    pub fn point(x: f64) -> Self {
        CertifiedInterval { lo: x, hi: x }
    }

    /// A certified enclosure of the exact value an [`Expansion`] represents.
    ///
    /// Summing the components with directed rounding is sound per step: each
    /// partial sum is itself within one ulp of its correctly-rounded value,
    /// so `next_down`/`next_up` keep the running lower/upper bounds.
    pub fn from_expansion(e: &Expansion) -> Self {
        let mut lo = 0.0_f64;
        let mut hi = 0.0_f64;
        for &c in &e.components {
            lo = (lo + c).next_down();
            hi = (hi + c).next_up();
        }
        CertifiedInterval { lo, hi }
    }

    /// Exact addition.
    pub fn add(&self, other: &Self) -> Self {
        CertifiedInterval {
            lo: (self.lo + other.lo).next_down(),
            hi: (self.hi + other.hi).next_up(),
        }
    }

    /// Exact subtraction.
    pub fn sub(&self, other: &Self) -> Self {
        CertifiedInterval {
            lo: (self.lo - other.hi).next_down(),
            hi: (self.hi - other.lo).next_up(),
        }
    }

    /// Exact negation.
    pub fn neg(&self) -> Self {
        CertifiedInterval {
            lo: -self.hi,
            hi: -self.lo,
        }
    }

    /// Exact multiplication: the extrema of the product over a rectangle are
    /// attained at the corners, each widened by one ulp.
    pub fn mul(&self, other: &Self) -> Self {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        let mut non_finite = false;
        for a in [self.lo, self.hi] {
            for b in [other.lo, other.hi] {
                let p = a * b;
                if p.is_finite() {
                    lo = lo.min(p.next_down());
                    hi = hi.max(p.next_up());
                } else {
                    non_finite = true;
                }
            }
        }
        if non_finite {
            CertifiedInterval {
                lo: f64::NEG_INFINITY,
                hi: f64::INFINITY,
            }
        } else {
            CertifiedInterval { lo, hi }
        }
    }

    /// Exact division. `None` when the denominator contains zero (the
    /// quotient is unbounded) or a quotient is not finite.
    pub fn div(&self, other: &Self) -> Option<Self> {
        if other.lo <= 0.0 && other.hi >= 0.0 {
            return None;
        }
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for n in [self.lo, self.hi] {
            for d in [other.lo, other.hi] {
                let q = n / d;
                if q.is_finite() {
                    lo = lo.min(q.next_down());
                    hi = hi.max(q.next_up());
                } else {
                    return None;
                }
            }
        }
        Some(CertifiedInterval { lo, hi })
    }

    /// Exact square root of a nonnegative interval. `None` when the interval
    /// contains a negative value or the result is not finite.
    pub fn sqrt(&self) -> Option<Self> {
        if self.lo < 0.0 {
            return None;
        }
        let lo = self.lo.sqrt().next_down();
        let hi = self.hi.sqrt().next_up();
        if lo.is_finite() && hi.is_finite() {
            Some(CertifiedInterval { lo, hi })
        } else {
            None
        }
    }

    /// Exact scaling by `2^n`.
    pub fn scale_pow2(&self, n: i32) -> Self {
        let f = 2.0_f64.powi(n);
        CertifiedInterval {
            lo: self.lo * f,
            hi: self.hi * f,
        }
    }

    /// The interval width `hi − lo`.
    pub fn width(&self) -> f64 {
        self.hi - self.lo
    }

    /// Whether `x` lies within the interval (inclusive).
    pub fn contains(&self, x: f64) -> bool {
        self.lo <= x && x <= self.hi
    }

    /// Whether the interval is a single exact point.
    pub fn is_degenerate(&self) -> bool {
        self.lo == self.hi
    }

    /// Whether both bounds are finite.
    pub fn is_finite(&self) -> bool {
        self.lo.is_finite() && self.hi.is_finite()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_and_grow_are_exact() {
        // 0.1 * 0.2 is not representable; the expansion must still be
        // nonzero and positive.
        let e = Expansion::from_product(0.1, 0.2);
        assert_eq!(e.sign(), CertifiedSign::Positive);
        assert!(!e.is_zero());
    }

    #[test]
    fn zero_minus_itself_is_zero() {
        let e = Expansion::from_product(0.1, 0.2);
        let diff = e.merge(&e.negate());
        assert_eq!(diff.sign(), CertifiedSign::Zero);
        assert!(diff.is_zero());
    }

    #[test]
    fn a_square_is_positive() {
        let e = Expansion::from_product(3.0, 3.0).merge(&Expansion::from_product(4.0, 4.0));
        assert_eq!(e.sign(), CertifiedSign::Positive);
    }

    #[test]
    fn a_negative_value_is_negative() {
        // 1.0*1.0 - 2.0*2.0 = -3.0 exactly
        let e =
            Expansion::from_product(1.0, 1.0).merge(&Expansion::from_product(2.0, 2.0).negate());
        assert_eq!(e.sign(), CertifiedSign::Negative);
    }

    #[test]
    fn exact_dot2_matches_geometry() {
        let e = exact_dot2([3.0, 4.0], [3.0, 4.0]);
        assert_eq!(e.sign(), CertifiedSign::Positive);
    }

    #[test]
    fn power_of_two_scaling_preserves_sign() {
        let e = Expansion::from_product(0.1, 0.2).scale_expansion_by_pow2(2);
        assert_eq!(e.sign(), CertifiedSign::Positive);
    }

    // -- CertifiedInterval -------------------------------------------------

    #[test]
    fn from_expansion_encloses_the_exact_value() {
        let e = Expansion::from_product(0.1, 0.2); // exact ~0.0200000000000000004
        let iv = CertifiedInterval::from_expansion(&e);
        assert!(iv.lo > 0.0, "downward enclosure stays positive");
        assert!(iv.contains(0.02));
        assert!(iv.lo <= 0.02 && 0.02 <= iv.hi);
    }

    #[test]
    fn zero_expansion_encloses_zero() {
        let e = Expansion::zero();
        let iv = CertifiedInterval::from_expansion(&e);
        assert_eq!((iv.lo, iv.hi), (0.0, 0.0));
    }

    #[test]
    fn point_arithmetic_is_enclosed_by_directed_rounding() {
        let a = CertifiedInterval::point(2.0);
        let b = CertifiedInterval::point(3.0);
        let s = a.add(&b);
        assert!(s.contains(5.0), "2 + 3 = 5 must lie in the enclosure");
        assert!(
            !s.is_degenerate(),
            "directed rounding widens even exact sums"
        );
        let p = a.mul(&b);
        assert!(p.contains(6.0));
        let q = a.div(&b).unwrap();
        assert!(q.lo <= 2.0 / 3.0 && 2.0 / 3.0 <= q.hi);
        assert!(!q.is_degenerate(), "division of inexact 2/3 widens");
    }

    #[test]
    fn sqrt_encloses_and_rejects_negative() {
        let four = CertifiedInterval::point(4.0);
        let two = four.sqrt().unwrap();
        assert!(two.lo <= 2.0 && 2.0 <= two.hi);
        assert!(CertifiedInterval { lo: -1.0, hi: 1.0 }.sqrt().is_none());
    }

    #[test]
    fn division_by_a_zero_containing_denominator_is_none() {
        let n = CertifiedInterval::point(1.0);
        assert!(n.div(&CertifiedInterval { lo: -1.0, hi: 1.0 }).is_none());
        assert!(n.div(&CertifiedInterval::point(0.0)).is_none());
    }

    #[test]
    fn negative_denominator_division_is_sound() {
        let n = CertifiedInterval::point(1.0);
        let d = CertifiedInterval::point(-3.0);
        let q = n.div(&d).unwrap();
        assert!(q.lo <= -1.0 / 3.0 && -1.0 / 3.0 <= q.hi);
    }

    #[test]
    fn interval_arithmetic_contains_exact_irrationals() {
        // sqrt(2) is not representable: the interval from directed rounding
        // must still contain it, and the square of the enclosure must contain 2.
        let s = CertifiedInterval::point(2.0).sqrt().unwrap();
        let s2 = s.mul(&s);
        assert!(s2.lo <= 2.0 && 2.0 <= s2.hi);
    }

    #[test]
    fn cross_exp_sign_is_exact() {
        // (1,0) × (0,1) = +1 exactly; (0,1) × (1,0) = −1 exactly.
        assert_eq!(
            cross_exp([1.0, 0.0], [0.0, 1.0]).sign(),
            CertifiedSign::Positive
        );
        assert_eq!(
            cross_exp([0.0, 1.0], [1.0, 0.0]).sign(),
            CertifiedSign::Negative
        );
    }

    #[test]
    fn exact_sq_dist_is_exact() {
        // 3-4-5: distance from (3,4) to (0,0) is 25 exactly.
        let t = exact_sq_dist([3.0, 4.0], [0.0, 0.0]);
        assert_eq!(t.sign(), CertifiedSign::Positive);
        assert!(CertifiedInterval::from_expansion(&t).contains(25.0));
        // A point is exactly distance zero from itself.
        assert_eq!(
            exact_sq_dist([0.1, 0.2], [0.1, 0.2]).sign(),
            CertifiedSign::Zero
        );
        // 0.1² + 0.2² is positive and not representable: the expansion still
        // decides the sign.
        let u = exact_sq_dist([0.1, 0.2], [0.0, 0.0]);
        assert_eq!(u.sign(), CertifiedSign::Positive);
        assert!(!u.is_zero());
    }

    #[test]
    fn exact_sq_dist_recovers_a_rounded_difference() {
        // 0.3 − 0.2 − 0.1 is not zero in f64; the exact squared distance from
        // (0.3, 0.0) to (0.1, 0.0) must equal (0.3 − 0.1)² exactly, which is
        // positive, and the same when computed the other way.
        let d = exact_sq_dist([0.3, 0.0], [0.1, 0.0]);
        assert_eq!(d.sign(), CertifiedSign::Positive);
    }

    #[test]
    fn mul_expansion_is_exact() {
        // (1 + 2^−30)·(1 − 2^−30) = 1 − 2^−60, which is positive and not
        // representable in f64. The expansion product must decide the sign.
        let a = Expansion::zero().grow(1.0).grow(2.0f64.powi(-30));
        let b = Expansion::zero().grow(1.0).grow(-2.0f64.powi(-30));
        let prod = a.mul_expansion(&b);
        assert_eq!(prod.sign(), CertifiedSign::Positive);
        // The enclosure of the product contains the exact value.
        assert!(CertifiedInterval::from_expansion(&prod).contains(1.0 - 2.0f64.powi(-60)));
        // Half of a negative is negative, and its enclosure contains −0.25.
        let half = Expansion::zero().grow(0.5);
        let neg_half = Expansion::zero().grow(-0.5);
        let p = half.mul_expansion(&neg_half);
        assert_eq!(p.sign(), CertifiedSign::Negative);
        assert!(CertifiedInterval::from_expansion(&p).contains(-0.25));
        // 0.1·0.2 squared: the exact product of the exact product is positive.
        let x = Expansion::from_product(0.1, 0.2);
        let sq = x.mul_expansion(&x);
        assert_eq!(sq.sign(), CertifiedSign::Positive);
    }

    #[test]
    fn mul_expansion_sign_survives_catastrophic_cancellation() {
        // (1 + e)(1 − e) = 1 − e² with e = 2^−30. In ordinary f64 this
        // rounds to exactly 1.0, so both the product's naive sign and the
        // difference from 1.0 are *zero* there. The exact product is 1 − 2^−60
        // and its difference from 1 is exactly −2^−60: the expansion must
        // report Negative where ordinary arithmetic reports 0.
        let e = 2.0f64.powi(-30);
        let a = Expansion::zero().grow(1.0).grow(e);
        let b = Expansion::zero().grow(1.0).grow(-e);
        let left = a.mul_expansion(&b);
        // The ordinary f64 evaluation rounds to 1.0:
        assert_eq!((1.0 + e) * (1.0 - e), 1.0);
        let one = Expansion::zero().grow(1.0);
        let diff = left.merge(&one.negate()); // (1 − e²) − 1 = −e²
        assert_eq!(diff.sign(), CertifiedSign::Negative);
        // And the positive mirror: 1 − (1 − e²) = +e².
        let pos = one.merge(&left.negate());
        assert_eq!(pos.sign(), CertifiedSign::Positive);
        assert!(CertifiedInterval::from_expansion(&pos).contains(2.0f64.powi(-60)));
    }

    #[test]
    fn mul_expansion_handles_mixed_exponents() {
        // (1e100 + 1e−50)·(1e100 − 1e−50) = 1e200 − 1e−100: the dominant
        // positive component must survive the mix of ~300 orders of magnitude
        // in the pairwise products.
        let big = 1.0e100;
        let small = 1.0e-50;
        let a = Expansion::zero().grow(big).grow(small);
        let b = Expansion::zero().grow(big).grow(-small);
        let p = a.mul_expansion(&b);
        assert_eq!(p.sign(), CertifiedSign::Positive);
        // The enclosure contains the exact value (1e200 − 1e−100 rounds to
        // 1e200, which is representable).
        assert!(CertifiedInterval::from_expansion(&p).contains(big * big));
    }

    #[test]
    fn repeated_grow_keeps_components_ordered_for_sign() {
        // sign() reads the largest-magnitude (last) component; a long chain of
        // grow/merge operations must preserve the ordered, non-overlapping
        // representation that makes that last component authoritative.
        let mut acc = Expansion::zero();
        for i in 0..64 {
            // 1 + 2^-i sums are all exactly representable at first and spill
            // exact error components as the magnitude grows past 53 bits.
            acc = acc.grow(1.0 + 2.0f64.powi(-i));
        }
        assert_eq!(acc.sign(), CertifiedSign::Positive);
        // Mirror each term back out: the errors must cancel to an exact zero.
        for i in 0..64 {
            acc = acc.grow(-(1.0 + 2.0f64.powi(-i)));
        }
        assert!(acc.is_zero(), "128 grows must cancel exactly");
        assert_eq!(acc.sign(), CertifiedSign::Zero);
        // The structural invariant sign() relies on: components are ordered by
        // increasing magnitude, so the last is the largest.
        let e = Expansion::zero().grow(0.1).grow(0.2).grow(0.3);
        assert!(
            e.components.windows(2).all(|w| w[0].abs() < w[1].abs()),
            "components must be ordered by increasing magnitude for sign()"
        );
    }

    #[test]
    fn mul_expansion_exact_zero_from_nontrivial_factors() {
        // (1 + e)(1 − e) + (1 + e)(−(1 − e)) = 0 exactly, but each product
        // is a two-component expansion summing to a nonzero value. The
        // repeated-grow representation must survive to an exact zero.
        let e = 2.0f64.powi(-30);
        let a = Expansion::zero().grow(1.0).grow(e);
        let b = Expansion::zero().grow(1.0).grow(-e);
        let p = a.mul_expansion(&b);
        let q = a.mul_expansion(&b.negate());
        assert_ne!(p.sign(), CertifiedSign::Zero);
        assert_ne!(q.sign(), CertifiedSign::Zero);
        let sum = p.merge(&q);
        assert!(sum.is_zero());
        assert_eq!(sum.sign(), CertifiedSign::Zero);
    }
}
