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

//! The ℚ-polynomial substrate of the exact-vanishing verifier (CTE-001-QPOLY).
//!
//! A hand-rolled multivariate polynomial ring over the rationals — **no CAS
//! dependency** (product boundary, `AGENTS.md`). [`QPoly`] is a sorted
//! [`BTreeMap`] from a monomial (an exponent vector) to an exact rational
//! coefficient [`QCoeff`], so iteration is deterministic (monomial
//! lexicographic order) and coefficient comparison is order-independent by
//! construction.
//!
//! **Exactness.** Every coefficient is an `i128`-numerator/`i128`-denominator
//! pair in lowest terms with a positive denominator. Ring arithmetic is exact
//! and **refusing on `i128` overflow** — overflow is a named
//! [`WitnessRefusal`], never a wrap and never a panic (scope decision 1; H-1,
//! H-2). Bernstein degrees in the CTE fixtures keep coefficients small; a real
//! overflow on a fixture is a degree/coefficient-scale decision the loop must
//! make (a `SPEC_GAP`), not something this module papers over.
//!
//! The ring has a fixed arity (number of variables) per value; mixing arities
//! in a binary operation refuses ([`WitnessRefusal::InvalidInput`]). A
//! monomial is an exponent vector of length `arity`; the constant polynomial
//! is the all-zero exponent vector.
//!
//! **H-1.** This module carries no `unwrap`, no `expect`, and no `panic!`,
//! and adds no module-level `allow`.

use crate::tangency::witness::WitnessRefusal;
use std::collections::BTreeMap;

/// The absolute value gcd used for rational reduction. Inputs are non-negative
/// by construction (`unsigned_abs`), so this never overflows.
fn gcd_u128(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

/// An exact rational coefficient: `num / den` in lowest terms with
/// `den > 0` (the `0` value is normalized to `0 / 1`).
///
/// Constructed only through [`QCoeff::new`] (which refuses a zero denominator
/// and normalizes) or [`QCoeff::from_int`]. Arithmetic refuses on `i128`
/// overflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QCoeff {
    num: i128,
    den: i128,
}

impl QCoeff {
    /// The exact zero.
    pub const fn zero() -> Self {
        QCoeff { num: 0, den: 1 }
    }

    /// The exact one.
    pub const fn one() -> Self {
        QCoeff { num: 1, den: 1 }
    }

    /// The exact integer value `n`.
    pub const fn from_int(n: i128) -> Self {
        QCoeff { num: n, den: 1 }
    }

    /// Build a normalized rational `num / den`.
    ///
    /// Refuses ([`WitnessRefusal::InvalidInput`]) a zero denominator, and
    /// [`WitnessRefusal::Overflow`] when the sign normalization overflows
    /// (only possible for `den == i128::MIN`, which cannot be negated).
    pub fn new(num: i128, den: i128) -> Result<Self, WitnessRefusal> {
        if den == 0 {
            return Err(WitnessRefusal::InvalidInput);
        }
        let (num, den) = if den < 0 {
            let num = num.checked_neg().ok_or(WitnessRefusal::Overflow)?;
            let den = den.checked_neg().ok_or(WitnessRefusal::Overflow)?;
            (num, den)
        } else {
            (num, den)
        };
        if num == 0 {
            return Ok(QCoeff::zero());
        }
        let g = gcd_u128(num.unsigned_abs(), den.unsigned_abs());
        // `g` divides `den <= i128::MAX`, so the cast is lossless.
        Ok(QCoeff {
            num: num / g as i128,
            den: den / g as i128,
        })
    }

    /// The signed numerator, verbatim.
    pub const fn num(&self) -> i128 {
        self.num
    }

    /// The positive denominator, verbatim.
    pub const fn den(&self) -> i128 {
        self.den
    }

    /// Exact zero test.
    pub const fn is_zero(self) -> bool {
        self.num == 0
    }

    /// Exact addition. Refuses ([`WitnessRefusal::Overflow`]) when the
    /// cross-multiplied numerator or the product denominator overflows `i128`.
    pub fn add(&self, rhs: &QCoeff) -> Result<Self, WitnessRefusal> {
        let a = self
            .num
            .checked_mul(rhs.den)
            .ok_or(WitnessRefusal::Overflow)?;
        let b = rhs
            .num
            .checked_mul(self.den)
            .ok_or(WitnessRefusal::Overflow)?;
        let num = a.checked_add(b).ok_or(WitnessRefusal::Overflow)?;
        let den = self
            .den
            .checked_mul(rhs.den)
            .ok_or(WitnessRefusal::Overflow)?;
        QCoeff::new(num, den)
    }

    /// Exact subtraction. Refuses on overflow.
    pub fn sub(&self, rhs: &QCoeff) -> Result<Self, WitnessRefusal> {
        self.add(&rhs.neg()?)
    }

    /// Exact multiplication. Refuses on overflow.
    pub fn mul(&self, rhs: &QCoeff) -> Result<Self, WitnessRefusal> {
        let num = self
            .num
            .checked_mul(rhs.num)
            .ok_or(WitnessRefusal::Overflow)?;
        let den = self
            .den
            .checked_mul(rhs.den)
            .ok_or(WitnessRefusal::Overflow)?;
        QCoeff::new(num, den)
    }

    /// The exact additive inverse. Refuses only when `num == i128::MIN`.
    pub fn neg(&self) -> Result<Self, WitnessRefusal> {
        Ok(QCoeff {
            num: self.num.checked_neg().ok_or(WitnessRefusal::Overflow)?,
            den: self.den,
        })
    }
}

/// A monomial: the exponent vector of one term of a fixed-arity ring.
///
/// Ordering is the derived lexicographic order on the exponent vector, which
/// makes the [`BTreeMap`] iteration in [`QPoly`] deterministic.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Monomial(Vec<u32>);

impl Monomial {
    /// Exponent-wise sum of two monomials of the same arity. Refuses on an
    /// arity mismatch or an exponent overflow.
    fn multiply(&self, rhs: &Monomial) -> Result<Monomial, WitnessRefusal> {
        if self.0.len() != rhs.0.len() {
            return Err(WitnessRefusal::InvalidInput);
        }
        let mut exps = Vec::with_capacity(self.0.len());
        for (&a, &b) in self.0.iter().zip(rhs.0.iter()) {
            exps.push(a.checked_add(b).ok_or(WitnessRefusal::Overflow)?);
        }
        Ok(Monomial(exps))
    }
}

/// An exact multivariate polynomial over ℚ in a fixed-arity ring: a sorted
/// `BTreeMap<Monomial, QCoeff>` with the zero coefficient omitted.
///
/// Every stored monomial has exponent length `arity`; the constant polynomial
/// is the single all-zero exponent vector. Iteration is in monomial
/// lexicographic order (deterministic; scope decision 6). Ring operations are
/// exact and refuse on `i128` coefficient overflow (H-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QPoly {
    /// The number of ring variables.
    arity: usize,
    /// The ordered term map.
    terms: BTreeMap<Monomial, QCoeff>,
}

impl QPoly {
    /// The zero polynomial of an `arity`-variable ring.
    pub fn zero(arity: usize) -> Self {
        QPoly {
            arity,
            terms: BTreeMap::new(),
        }
    }

    /// The constant one of an `arity`-variable ring.
    pub fn one(arity: usize) -> Self {
        let mut terms = BTreeMap::new();
        terms.insert(Monomial(vec![0u32; arity]), QCoeff::one());
        QPoly { arity, terms }
    }

    /// The ring arity (number of variables).
    pub const fn arity(&self) -> usize {
        self.arity
    }

    /// Exact zero test: the term map is empty.
    pub fn is_zero(&self) -> bool {
        self.terms.is_empty()
    }

    /// Build the single-term polynomial `coeff · x^exponents`.
    ///
    /// Refuses ([`WitnessRefusal::InvalidInput`]) an exponent vector whose
    /// length does not equal `arity`. A zero coefficient yields the zero
    /// polynomial.
    pub fn term(arity: usize, exponents: &[u32], coeff: QCoeff) -> Result<Self, WitnessRefusal> {
        if exponents.len() != arity {
            return Err(WitnessRefusal::InvalidInput);
        }
        let mut terms = BTreeMap::new();
        if !coeff.is_zero() {
            terms.insert(Monomial(exponents.to_vec()), coeff);
        }
        Ok(QPoly { arity, terms })
    }

    /// Build a polynomial from integer coefficients, one `(exponents, coeff)`
    /// pair per term. Duplicate exponent vectors merge by exact addition.
    pub fn from_int_terms(arity: usize, terms: &[(&[u32], i128)]) -> Result<Self, WitnessRefusal> {
        let mut acc = QPoly::zero(arity);
        for (exponents, coeff) in terms {
            let t = QPoly::term(arity, exponents, QCoeff::from_int(*coeff))?;
            acc = acc.add(&t)?;
        }
        Ok(acc)
    }

    /// Exact addition. Refuses on an arity mismatch or coefficient overflow.
    pub fn add(&self, rhs: &QPoly) -> Result<QPoly, WitnessRefusal> {
        if self.arity != rhs.arity {
            return Err(WitnessRefusal::InvalidInput);
        }
        let mut out = self.clone();
        for (monomial, coeff) in &rhs.terms {
            match out.terms.remove(monomial) {
                None => {
                    out.terms.insert(monomial.clone(), *coeff);
                }
                Some(prev) => {
                    let sum = prev.add(coeff)?;
                    if !sum.is_zero() {
                        out.terms.insert(monomial.clone(), sum);
                    }
                }
            }
        }
        Ok(out)
    }

    /// Exact subtraction. Refuses on an arity mismatch or coefficient
    /// overflow.
    pub fn sub(&self, rhs: &QPoly) -> Result<QPoly, WitnessRefusal> {
        self.add(&rhs.neg()?)
    }

    /// Exact multiplication. Refuses on an arity mismatch, coefficient
    /// overflow, or exponent overflow.
    pub fn mul(&self, rhs: &QPoly) -> Result<QPoly, WitnessRefusal> {
        if self.arity != rhs.arity {
            return Err(WitnessRefusal::InvalidInput);
        }
        let mut out = QPoly::zero(self.arity);
        for (ma, ca) in &self.terms {
            for (mb, cb) in &rhs.terms {
                let monomial = ma.multiply(mb)?;
                let coeff = ca.mul(cb)?;
                match out.terms.remove(&monomial) {
                    None => {
                        out.terms.insert(monomial, coeff);
                    }
                    Some(prev) => {
                        let sum = prev.add(&coeff)?;
                        if !sum.is_zero() {
                            out.terms.insert(monomial, sum);
                        }
                    }
                }
            }
        }
        Ok(out)
    }

    /// The exact `n`-th power by exponentiation by squaring. `pow(0)` is the
    /// constant one. Refuses on coefficient overflow.
    pub fn pow(&self, n: u32) -> Result<QPoly, WitnessRefusal> {
        let mut result = QPoly::one(self.arity);
        let mut base = self.clone();
        let mut exp = n;
        while exp > 0 {
            if exp & 1 == 1 {
                result = result.mul(&base)?;
            }
            exp >>= 1;
            if exp > 0 {
                base = base.mul(&base)?;
            }
        }
        Ok(result)
    }

    /// The exact additive inverse. Refuses on coefficient negation overflow.
    pub fn neg(&self) -> Result<QPoly, WitnessRefusal> {
        let mut terms = BTreeMap::new();
        for (monomial, coeff) in &self.terms {
            terms.insert(monomial.clone(), coeff.neg()?);
        }
        Ok(QPoly {
            arity: self.arity,
            terms,
        })
    }

    /// Exact scalar multiplication by `factor`. Refuses on overflow.
    pub fn scale(&self, factor: QCoeff) -> Result<QPoly, WitnessRefusal> {
        if factor.is_zero() {
            return Ok(QPoly::zero(self.arity));
        }
        let mut terms = BTreeMap::new();
        for (monomial, coeff) in &self.terms {
            terms.insert(monomial.clone(), coeff.mul(&factor)?);
        }
        Ok(QPoly {
            arity: self.arity,
            terms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tiny deterministic LCG for the seeded ring tests. Fixed order.
    struct Lcg(u64);

    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            self.0 >> 33
        }
    }

    /// A random-but-seeded sparse polynomial in `arity` variables: up to four
    /// terms, exponents in `0..=3`, integer coefficients in `-4..=4`.
    fn random_poly(rng: &mut Lcg, arity: usize) -> QPoly {
        let mut acc = QPoly::zero(arity);
        let count = 1 + (rng.next() % 4) as usize;
        for _ in 0..count {
            let exponents: Vec<u32> = (0..arity).map(|_| (rng.next() % 4) as u32).collect();
            let coeff = (rng.next() % 9) as i128 - 4;
            if coeff == 0 {
                continue;
            }
            let term = QPoly::term(arity, &exponents, QCoeff::from_int(coeff))
                .expect("an arity-length exponent vector is valid");
            acc = acc.add(&term).expect("same-arity addition does not refuse");
        }
        acc
    }

    #[test]
    fn qpoly_ring_arithmetic_is_exact() {
        let mut rng = Lcg(0xC7E_001);
        for _ in 0..16 {
            let a = random_poly(&mut rng, 3);
            let b = random_poly(&mut rng, 3);
            let c = random_poly(&mut rng, 3);

            let a_sq = a.mul(&a).expect("a^2");
            let b_sq = b.mul(&b).expect("b^2");
            let ab = a.mul(&b).expect("a*b");
            let two_ab = ab.scale(QCoeff::from_int(2)).expect("2*a*b");

            // (a + b)^2 == a^2 + 2ab + b^2, exactly.
            let lhs = a
                .add(&b)
                .expect("a + b")
                .mul(&a.add(&b).expect("a + b"))
                .expect("(a + b)^2");
            let rhs = a_sq
                .add(&two_ab)
                .expect("a^2 + 2ab")
                .add(&b_sq)
                .expect("a^2 + 2ab + b^2");
            assert_eq!(lhs, rhs, "(a + b)^2 must equal a^2 + 2ab + b^2 exactly");

            // Distributivity: (a + b)c == ac + bc, exactly.
            let lhs = a.add(&b).expect("a + b").mul(&c).expect("(a + b)c");
            let rhs = a
                .mul(&c)
                .expect("ac")
                .add(&b.mul(&c).expect("bc"))
                .expect("ac + bc");
            assert_eq!(lhs, rhs, "distributivity must hold exactly");

            // Associativity of addition, exactly.
            let lhs = a.add(&b).expect("a + b").add(&c).expect("(a + b) + c");
            let rhs = a.add(&b.add(&c).expect("b + c")).expect("a + (b + c)");
            assert_eq!(lhs, rhs, "addition must be associative exactly");

            // pow: a^3 == a * a * a; a^0 == 1.
            let lhs = a.pow(3).expect("a^3");
            let rhs = a.mul(&a).expect("a^2").mul(&a).expect("a^2 * a");
            assert_eq!(lhs, rhs, "pow(3) must equal the repeated product exactly");
            let one = QPoly::one(3);
            assert_eq!(
                a.pow(0).expect("a^0"),
                one,
                "pow(0) must be the constant one"
            );

            // Exact zero: a - a == 0.
            assert!(a.sub(&a).expect("a - a").is_zero());
        }
    }
}
