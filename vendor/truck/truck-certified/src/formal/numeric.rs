//! Checked finite numerics for formal certificates.
//!
//! `FORMAL_SYSTEM.md` §XVI lists "numerical embedding" and "correct curve
//! enclosures" among the *assumed* propositions. An assumption is only as good
//! as the number carrying it, and a bound of `NaN` compares false against every
//! threshold while a bound of `+inf` compares true against every threshold.
//! Either one silently converts a certificate into a tautology.
//!
//! These wrappers make such a value unrepresentable. Every bound stored in a
//! formal certificate in this subtree is one of these three types; no formal
//! certificate holds a raw `f64`.
//!
//! # Equality and hashing
//!
//! `PartialEq` is implemented, `Eq` and `Hash` are not. Excluding `NaN` makes
//! `f64` equality reflexive, so `Eq` would be *sound* — but `+0.0 == -0.0`
//! while their bit patterns differ, so a derived `Hash` would break the
//! `Hash`/`Eq` agreement, and a hand-written one would have to commit to a
//! normalization no Step 1 consumer needs. Step 1 compares periods and
//! translations; it does not key maps on them. Nothing more is implemented.

use std::fmt;

/// Why a checked numeric constructor refused its input.
///
/// There is deliberately no `Other` variant: a refusal that cannot be named is
/// a refusal that cannot be acted on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericDomainError {
    /// The value was `NaN`.
    NotANumber,
    /// The value was `+inf` or `-inf`.
    Infinite,
    /// The value was finite but negative, where nonnegativity is required.
    Negative,
    /// The value was finite and nonnegative but zero, where positivity is
    /// required.
    Zero,
}

impl fmt::Display for NumericDomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::NotANumber => "value is NaN",
            Self::Infinite => "value is infinite",
            Self::Negative => "value is negative",
            Self::Zero => "value is zero",
        };
        f.write_str(text)
    }
}

impl std::error::Error for NumericDomainError {}

/// A `f64` proved finite: neither `NaN` nor either infinity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FiniteF64(f64);

impl FiniteF64 {
    /// The only way to obtain one.
    pub fn new(value: f64) -> Result<Self, NumericDomainError> {
        match value {
            v if v.is_nan() => Err(NumericDomainError::NotANumber),
            v if v.is_infinite() => Err(NumericDomainError::Infinite),
            v => Ok(Self(v)),
        }
    }

    /// The underlying value. Named `get`, not `value`, and there is no `Deref`:
    /// a finite number should not silently become a `f64` at a call site that
    /// never proved anything about it.
    pub fn get(self) -> f64 {
        self.0
    }

    /// Whether this value is exactly zero. `+0.0` and `-0.0` both answer true;
    /// the predicate asked is "is this the additive identity", not "which zero".
    pub fn is_zero(self) -> bool {
        self.0 == 0.0
    }
}

/// A finite `f64` proved nonnegative. Tolerances and residual bounds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NonNegativeFinite(FiniteF64);

impl NonNegativeFinite {
    /// The only way to obtain one.
    pub fn new(value: f64) -> Result<Self, NumericDomainError> {
        let finite = FiniteF64::new(value)?;
        match finite.get() < 0.0 {
            true => Err(NumericDomainError::Negative),
            false => Ok(Self(finite)),
        }
    }

    /// The underlying finite value.
    pub fn finite(self) -> FiniteF64 {
        self.0
    }

    /// The underlying value.
    pub fn get(self) -> f64 {
        self.0.get()
    }
}

/// A finite `f64` proved strictly positive. Period values and any bound whose
/// zero would make a quotient degenerate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PositiveFinite(NonNegativeFinite);

impl PositiveFinite {
    /// The only way to obtain one.
    pub fn new(value: f64) -> Result<Self, NumericDomainError> {
        let nonnegative = NonNegativeFinite::new(value)?;
        match nonnegative.get() == 0.0 {
            true => Err(NumericDomainError::Zero),
            false => Ok(Self(nonnegative)),
        }
    }

    /// The underlying nonnegative value.
    pub fn nonnegative(self) -> NonNegativeFinite {
        self.0
    }

    /// The underlying value.
    pub fn get(self) -> f64 {
        self.0.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nan_is_not_a_finite_value() {
        assert_eq!(
            FiniteF64::new(f64::NAN),
            Err(NumericDomainError::NotANumber)
        );
        assert_eq!(
            NonNegativeFinite::new(f64::NAN),
            Err(NumericDomainError::NotANumber)
        );
        assert_eq!(
            PositiveFinite::new(f64::NAN),
            Err(NumericDomainError::NotANumber)
        );
    }

    #[test]
    fn infinity_is_not_a_finite_value() {
        assert_eq!(
            FiniteF64::new(f64::INFINITY),
            Err(NumericDomainError::Infinite)
        );
        assert_eq!(
            FiniteF64::new(f64::NEG_INFINITY),
            Err(NumericDomainError::Infinite)
        );
        // A positive infinity is not a "very large positive period": the
        // positivity check must never be reached.
        assert_eq!(
            PositiveFinite::new(f64::INFINITY),
            Err(NumericDomainError::Infinite)
        );
    }

    #[test]
    fn zero_is_not_positive() {
        assert_eq!(PositiveFinite::new(0.0), Err(NumericDomainError::Zero));
        assert_eq!(PositiveFinite::new(-0.0), Err(NumericDomainError::Zero));
        assert!(NonNegativeFinite::new(0.0).is_ok(), "zero is nonnegative");
    }

    #[test]
    fn negative_is_not_nonnegative() {
        assert_eq!(
            NonNegativeFinite::new(-1.0),
            Err(NumericDomainError::Negative)
        );
        assert_eq!(
            PositiveFinite::new(-1.0),
            Err(NumericDomainError::Negative),
            "a negative value is refused as negative, not as zero"
        );
    }

    #[test]
    fn a_finite_value_round_trips() {
        let period = PositiveFinite::new(std::f64::consts::PI * 2.0).unwrap();
        assert_eq!(period.get(), std::f64::consts::PI * 2.0);
        assert!(!period.nonnegative().finite().is_zero());
    }
}
