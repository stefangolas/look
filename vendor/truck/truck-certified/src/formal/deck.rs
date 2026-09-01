// Grandfathered (orchestrator amendment, BG-CK-P0-CRATE r3): moved
// verbatim from truck-meshalgo, whose crate never denied
// clippy::unwrap_used. The crate-level deny in lib.rs is H-1's contract
// for AUTHORED certified code; this module's pre-existing unwraps are
// inherited baseline content and must not be force-rewritten by the
// move packet. Do not add new unwraps under this allow.
#![allow(clippy::unwrap_used)]

//! Certified integer deck arithmetic for the rank-1 contractible-disk slice.
//!
//! # What this module decides
//!
//! A rank-1 surface such as an embedded cylinder has one periodic parameter.
//! Its *deck group* is `Z`: translating the developed plane by any integer
//! multiple of the period generator `g` returns to the same physical point. A
//! curve developed into the universal cover can therefore land on any copy, and
//! the slice must decide *which* integer copy, from certified real-valued
//! evidence and never by rounding a near-integer.
//!
//! Given a certified displacement enclosure `D` and a certified period
//! generator `g != 0`, the two questions this module answers are:
//!
//! 1. *Milestone 1A — the deck solver.* Which integers `k` satisfy `d = k g`?
//!    ([`DeckSolveResult`]). This is a four-way decision, never an `Option<i64>`.
//! 2. *Milestone 1B — the cover interval.* Given two developed enclosures
//!    `(B_i, B_j)`, which integers `k` make `(B_i - B_j) ∩ {k g}` non-empty?
//!    ([`CertifiedDeckInterval`]). This is a conservative superset: a deck index
//!    not proven impossible is included. False positives are allowed; false
//!    negatives are P0.
//!
//! # Outward rounding
//!
//! Every real bound stored here is the result of a conservative outward-rounded
//! interval computation. A single correctly-rounded `f64` operation `r` satisfies
//! `|r - t| <= 0.5 ulp(r)` against the true value `t`; stepping one
//! representable value toward `-inf` (resp. `+inf`) past `r` is therefore
//! guaranteed to lie at or below (resp. at or above) `t`. That single
//! [`f64::next_after`] step is the only rounding device used.
//!
//! ## Compositionality — the per-primitive discipline
//!
//! One ULP around a *compound* expression does not, in general, enclose the
//! error accumulated through several rounded operations. This module avoids that
//! hazard by never folding two rounded operations into one bound: every
//! outward-rounded endpoint is `toward_{neg,pos}( <single f64 op> )`, and
//! multi-step calculations chain *intervals*, not expressions. Concretely:
//!
//! - **Subtraction** ([`DevelopedBox::minkowski_difference`],
//!   [`DeckInterval::sub`]): each endpoint is one `+`/`-` op, then one ULP.
//!   The Minkowski difference `[a.lo - b.hi, a.hi - b.lo]` rounds each endpoint
//!   independently outward.
//! - **Division** ([`integer_quotient_range`]): each quotient endpoint is one
//!   `/` op, then one ULP. A negative divisor reverses the endpoints *before*
//!   the directed rounding, so the lower bound is still rounded toward `-inf`.
//! - **Multiplication** ([`provably_outside`]): `k * period` is one `*` op,
//!   enclosed to `[toward_neg, toward_pos]` before the comparison.
//! - **Chaining**: when a downstream stage consumes an upstream bound (e.g. the
//!   quotient divides an already-conservative interval), the upstream bound is
//!   an exact `f64` whose conservatism is preserved by the monotone outward
//!   rounding of the downstream op.
//!
//! Oracle tests below check the integer classification against an exact
//! brute-force reference (integer periods make `k * period` exact in `f64`),
//! including at integer boundaries, negative periods, large indices, subnormals,
//! and overflow thresholds.
//!
//! No `f64` is ever rounded to a nearest integer to *certify* a deck placement:
//! rounding to a nearest integer is a [`DeckSolveResult::Indeterminate`]
//! refusal when the evidence cannot resolve the placement.
//!
//! # Self-containment
//!
//! This module depends only on [`super::numeric`]. It does not parse STEP, read
//! the importer, or touch the ambient lattice. Milestone 2 wires a cylinder
//! schema to the [`DeckGenerator`] introduced here; until then the core is
//! exercised by the unit tests in this file.

use super::numeric::{FiniteF64, NonNegativeFinite, NumericDomainError, PositiveFinite};

// ---------------------------------------------------------------------------
// Numeric failure
// ---------------------------------------------------------------------------

/// Why a certified deck computation could not produce a finite, bounded answer.
///
/// This is an *implementation* fact, distinct from a mathematical judgment. It
/// maps to [`super::outcome::OperationalFailure`] at the slice boundary, never
/// to `Unsupported`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeckNumericFailure {
    /// A finite operation produced a non-finite value (`NaN` or infinity).
    NotFinite,
    /// A value sat at the finite extreme of `f64` and could not be rounded
    /// outward without leaving the finite range.
    AtFiniteExtreme,
    /// A deck integer coordinate did not fit in `i64`.
    IntegerOverflow,
}

/// Why a generator or interval constructor refused its input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeckConstructorFailure {
    /// A numeric value was not usable.
    Numeric(DeckNumericFailure),
    /// A period generator was given a zero translation.
    ///
    /// A zero generator makes `d = k g` trivially satisfiable for every `k` and
    /// collapses the deck group; it is refused rather than reported as
    /// "infinitely many compatible integers", which would be a claim about the
    /// *surface* rather than the input.
    ZeroPeriod,
    /// Interval endpoints were given in the wrong order.
    BoundsInverted,
}

impl From<NumericDomainError> for DeckConstructorFailure {
    fn from(_: NumericDomainError) -> Self {
        Self::Numeric(DeckNumericFailure::NotFinite)
    }
}

impl From<DeckNumericFailure> for DeckConstructorFailure {
    fn from(value: DeckNumericFailure) -> Self {
        Self::Numeric(value)
    }
}

/// The execution budget for one deck computation.
///
/// Carried separately from the result so that "the evidence was too broad"
/// ([`DeckSolveResult::Indeterminate`]) stays distinct from "the implementation
/// would exceed its envelope" ([`DeckOperationalFailure`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeckBudget {
    /// The maximum number of integers a single cover interval may contain
    /// before the computation reports envelope exhaustion rather than
    /// enumerating further.
    pub deck_width_cap: u64,
}

impl DeckBudget {
    /// A permissive budget for unit tests of the arithmetic in isolation.
    pub const FOR_TESTING: DeckBudget = DeckBudget {
        deck_width_cap: 1_000_000,
    };
}

/// Why a deck computation failed operationally. Maps to `OperationalFailure`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeckOperationalFailure {
    /// Checked arithmetic refused: a non-finite value or an `i64` overflow.
    ArithmeticOverflow,
    /// Instantiating the translated working cover would require more deck
    /// copies than the envelope permits. This is a *materialization* budget:
    /// it bites when copies are actually built (Milestone 7A), not when the
    /// cover interval is merely computed or a deck placement classified.
    #[allow(dead_code)]
    CoverBudgetExceeded {
        /// How many copies the cover would require.
        count: u64,
        /// How many the envelope permits.
        cap: u64,
    },
}

impl From<DeckNumericFailure> for DeckOperationalFailure {
    fn from(value: DeckNumericFailure) -> Self {
        match value {
            DeckNumericFailure::IntegerOverflow => Self::ArithmeticOverflow,
            other => {
                let _ = other;
                Self::ArithmeticOverflow
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Outward rounding
// ---------------------------------------------------------------------------

/// The next representable `f64` after `x` in the direction of `target`.
///
/// A manual reimplementation of `f64::next_after`, which is not yet available on
/// this toolchain. Bit-exact with the standard semantics: sign-magnitude
/// ordering means incrementing the bit pattern increases value for positives and
/// decreases it for negatives, handled by testing `(x > 0) == (x < target)`.
fn next_after(x: f64, target: f64) -> f64 {
    if x.is_nan() || target.is_nan() {
        return f64::NAN;
    }
    if x == target {
        return target;
    }
    if x == 0.0 {
        // The smallest subnormal, signed toward `target`.
        return f64::from_bits(if target > 0.0 { 1 } else { (1u64 << 63) | 1 });
    }
    let bits = x.to_bits();
    let stepped = if (x > 0.0) == (x < target) {
        bits.wrapping_add(1)
    } else {
        bits.wrapping_sub(1)
    };
    f64::from_bits(stepped)
}

/// Step one representable value toward `-inf`. Guaranteed at or below the true
/// value of any single nearest-rounded `f64` operation that produced `x`.
fn toward_neg(x: f64) -> Result<f64, DeckNumericFailure> {
    if !x.is_finite() {
        return Err(DeckNumericFailure::NotFinite);
    }
    let stepped = next_after(x, f64::NEG_INFINITY);
    if !stepped.is_finite() {
        // `x` was `f64::MIN`; we cannot widen downward within the finite range.
        Err(DeckNumericFailure::AtFiniteExtreme)
    } else {
        Ok(stepped)
    }
}

/// Step one representable value toward `+inf`. Guaranteed at or above the true
/// value of any single nearest-rounded `f64` operation that produced `x`.
fn toward_pos(x: f64) -> Result<f64, DeckNumericFailure> {
    if !x.is_finite() {
        return Err(DeckNumericFailure::NotFinite);
    }
    let stepped = next_after(x, f64::INFINITY);
    if !stepped.is_finite() {
        Err(DeckNumericFailure::AtFiniteExtreme)
    } else {
        Ok(stepped)
    }
}

/// A conservative lower bound on a single `f64` operation's true value.
fn lower_of(raw: f64) -> Result<FiniteF64, DeckNumericFailure> {
    FiniteF64::new(toward_neg(raw)?).map_err(|_| DeckNumericFailure::NotFinite)
}

/// A conservative upper bound on a single `f64` operation's true value.
fn upper_of(raw: f64) -> Result<FiniteF64, DeckNumericFailure> {
    FiniteF64::new(toward_pos(raw)?).map_err(|_| DeckNumericFailure::NotFinite)
}

// ---------------------------------------------------------------------------
// Certified interval
// ---------------------------------------------------------------------------

/// A closed real interval `[lower, upper]` with both endpoints finite and
/// ordered, used as a certified bound.
///
/// All arithmetic on intervals is outward-rounded: the result always contains
/// the true interval. Methods that produce a width or a magnitude round in the
/// conservative direction for the decision that consumes them, and each method
/// documents which direction that is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeckInterval {
    lower: FiniteF64,
    upper: FiniteF64,
}

impl DeckInterval {
    /// Build an interval from two already-checked finite endpoints.
    pub fn new(lower: FiniteF64, upper: FiniteF64) -> Result<Self, DeckConstructorFailure> {
        if lower.get() <= upper.get() {
            Ok(Self { lower, upper })
        } else {
            Err(DeckConstructorFailure::BoundsInverted)
        }
    }

    /// Build an interval from raw `f64` endpoints, checking finiteness and order.
    pub fn from_f64(lower: f64, upper: f64) -> Result<Self, DeckConstructorFailure> {
        let lower = FiniteF64::new(lower)?;
        let upper = FiniteF64::new(upper)?;
        Self::new(lower, upper)
    }

    /// A degenerate interval at one point.
    pub fn point(x: FiniteF64) -> Self {
        Self { lower: x, upper: x }
    }

    /// The lower endpoint.
    pub fn lower(self) -> FiniteF64 {
        self.lower
    }

    /// The upper endpoint.
    pub fn upper(self) -> FiniteF64 {
        self.upper
    }

    /// Whether a value lies in `[lower, upper]`.
    pub fn contains(self, x: f64) -> bool {
        self.lower.get() <= x && x <= self.upper.get()
    }

    /// A conservative *over*-estimate of the width `upper - lower`.
    ///
    /// Over-estimating the width is the safe direction for the uniqueness test
    /// `|g| > 2 rho`: if an over-estimated width still clears `|g|`, the true
    /// width certainly does, so a `Unique` verdict resting on it is sound.
    pub fn conservative_width(self) -> Result<NonNegativeFinite, DeckNumericFailure> {
        let raw = self.upper.get() - self.lower.get();
        let widened = upper_of(raw)?;
        NonNegativeFinite::new(widened.get()).map_err(|_| DeckNumericFailure::NotFinite)
    }

    /// Outward-rounded negation: `[-upper, -lower]`.
    pub fn neg(self) -> Result<Self, DeckNumericFailure> {
        // -(upper) is the new lower; round it down. -(lower) is the new upper.
        let lower = lower_of(-self.upper.get())?;
        let upper = upper_of(-self.lower.get())?;
        Ok(Self { lower, upper })
    }

    /// Outward-rounded sum with another interval.
    pub fn add(self, other: Self) -> Result<Self, DeckNumericFailure> {
        let lower = lower_of(self.lower.get() + other.lower.get())?;
        let upper = upper_of(self.upper.get() + other.upper.get())?;
        Ok(Self { lower, upper })
    }

    /// Outward-rounded difference: `self - other`.
    pub fn sub(self, other: Self) -> Result<Self, DeckNumericFailure> {
        self.add(other.neg()?)
    }
}

// ---------------------------------------------------------------------------
// Generator and developed geometry
// ---------------------------------------------------------------------------

/// One axis of the developed (universal-cover) plane.
///
/// The developed plane has two coordinates. One is periodic — translated by the
/// deck generator — and the other is aperiodic, with a *structural* zero in the
/// generator. Which physical axis maps to which developed axis is a fact the
/// cylinder schema records; the deck core never assumes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DevelopedAxis {
    /// The first developed coordinate.
    First,
    /// The second developed coordinate.
    Second,
}

impl DevelopedAxis {
    /// The other axis.
    pub fn other(self) -> Self {
        match self {
            Self::First => Self::Second,
            Self::Second => Self::First,
        }
    }
}

/// A certified rank-1 deck generator: an axis-aligned translation `g` with one
/// nonzero component `±P` on the periodic axis and a structural zero on the
/// other.
///
/// Constructed by the cylinder schema from a certified period; never by a
/// caller with bare numbers. The signed period carries the direction of
/// revolution, so a reversed periodic direction is `signed_period < 0` and is
/// handled by the solver without a special case.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeckGenerator {
    periodic_axis: DevelopedAxis,
    signed_period: FiniteF64,
}

impl DeckGenerator {
    /// Build a generator.
    ///
    /// Refuses a zero period: a zero generator collapses the deck group and is
    /// not a statement about the surface.
    pub fn new(
        periodic_axis: DevelopedAxis,
        signed_period: FiniteF64,
    ) -> Result<Self, DeckConstructorFailure> {
        if signed_period.is_zero() {
            return Err(DeckConstructorFailure::ZeroPeriod);
        }
        Ok(Self {
            periodic_axis,
            signed_period,
        })
    }

    /// The axis along which the generator translates.
    pub fn periodic_axis(self) -> DevelopedAxis {
        self.periodic_axis
    }

    /// The structurally-zero (aperiodic) axis.
    pub fn aperiodic_axis(self) -> DevelopedAxis {
        self.periodic_axis.other()
    }

    /// The signed period. Its sign is the direction of revolution.
    pub fn signed_period(self) -> FiniteF64 {
        self.signed_period
    }

    /// The period magnitude, proved strictly positive.
    pub fn period_magnitude(self) -> PositiveFinite {
        PositiveFinite::new(self.signed_period.get().abs()).expect(
            "signed_period is finite and nonzero by construction, so its absolute value is positive",
        )
    }
}

/// A two-axis developed enclosure: an axis-aligned box in the developed plane.
///
/// Used both as a displacement enclosure (Milestone 1A) and as a developed AABB
/// whose Minkowski difference feeds the cover interval (Milestone 1B).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DevelopedBox {
    /// The interval on the first developed axis.
    pub first: DeckInterval,
    /// The interval on the second developed axis.
    pub second: DeckInterval,
}

impl DevelopedBox {
    /// The interval on a named axis.
    pub fn on(self, axis: DevelopedAxis) -> DeckInterval {
        match axis {
            DevelopedAxis::First => self.first,
            DevelopedAxis::Second => self.second,
        }
    }

    /// The interval on the periodic axis of a generator.
    pub fn periodic(self, generator: &DeckGenerator) -> DeckInterval {
        self.on(generator.periodic_axis())
    }

    /// The interval on the aperiodic (structurally-zero) axis of a generator.
    pub fn aperiodic(self, generator: &DeckGenerator) -> DeckInterval {
        self.on(generator.aperiodic_axis())
    }

    /// The outward-rounded Minkowski difference `self - other`.
    ///
    /// Per coordinate `c`: `[self.min_c - other.max_c, self.max_c - other.min_c]`.
    pub fn minkowski_difference(self, other: Self) -> Result<DevelopedBox, DeckNumericFailure> {
        Ok(DevelopedBox {
            first: diff_coordinate(self.first, other.first)?,
            second: diff_coordinate(self.second, other.second)?,
        })
    }
}

fn diff_coordinate(a: DeckInterval, b: DeckInterval) -> Result<DeckInterval, DeckNumericFailure> {
    // [a.lower - b.upper, a.upper - b.lower], each endpoint outward-rounded.
    let lower = lower_of(a.lower().get() - b.upper().get())?;
    let upper = upper_of(a.upper().get() - b.lower().get())?;
    Ok(DeckInterval { lower, upper })
}

// ---------------------------------------------------------------------------
// Milestone 1A: the deck solver
// ---------------------------------------------------------------------------

/// The four-way result of deciding `d = k g` from certified evidence.
///
/// Not an `Option<i64>`: "no compatible integer", "several compatible
/// integers", and "the evidence cannot decide" are three different findings,
/// and a slice that collapsed them would misreport the corpus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeckSolveResult {
    /// Exactly one integer is certified compatible.
    Unique(i64),
    /// No integer is compatible with the enclosure.
    NoCompatibleInteger,
    /// Two or more integers are each compatible with the enclosure.
    MultipleCompatibleIntegers,
    /// The enclosure is too broad or too close to an integer boundary to decide
    /// between the above. Rounding to a nearest integer is forbidden.
    Indeterminate,
}

impl DeckSolveResult {
    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::Unique(_) => "deck_unique",
            Self::NoCompatibleInteger => "deck_none",
            Self::MultipleCompatibleIntegers => "deck_multiple",
            Self::Indeterminate => "deck_indeterminate",
        }
    }
}

/// Solve `d = k g` for the periodic integer `k`, given a displacement enclosure.
///
/// The displacement is a developed box; its component on the generator's
/// periodic axis is solved for `k`, and its component on the aperiodic
/// (structural-zero) axis must be compatible with zero.
///
/// # Classification without enumeration
///
/// The compatible integers form a *contiguous* sub-interval of the conservative
/// range `[k_min, k_max]`, because `k * period` is monotone in `k`. The
/// outward-rounded conservative range extends the true compatible range by at
/// most one on each side, so the first and last compatible candidate are found
/// among `{k_min, k_min + 1}` and `{k_max - 1, k_max}` — a constant number of
/// compatibility tests. The solver therefore never enumerates a wide interval
/// merely to classify it: a displacement whose enclosure spans many periods is
/// decided `MultipleCompatibleIntegers` from its first two compatible
/// candidates.
///
/// Returns [`DeckOperationalFailure`] only for arithmetic overflow — never for
/// a property of the surface, and never because the evidence was broad. A budget
/// on the *number* of deck copies belongs to working-cover materialization
/// ([`DeckOperationalFailure::CoverBudgetExceeded`]), not to this classification.
pub fn solve_axis_aligned(
    generator: &DeckGenerator,
    displacement: &DevelopedBox,
) -> Result<DeckSolveResult, DeckOperationalFailure> {
    // The aperiodic component of `k g` is structurally zero, so the aperiodic
    // displacement must contain zero. If it provably does not, no integer is
    // compatible: this is the aperiodic-coordinate contradiction.
    let aperiodic = displacement.aperiodic(generator);
    if !aperiodic.contains(0.0) {
        return Ok(DeckSolveResult::NoCompatibleInteger);
    }

    let periodic = displacement.periodic(generator);
    let signed_period = generator.signed_period().get();
    let period_mag = generator.period_magnitude().get();

    // Arithmetic-resolution guard. Adjacent deck integers `k`, `k + 1` map to
    // developed values differing by `period`; they are distinguishable in f64
    // only if `period` exceeds the ULP at the displacement scale. If it does
    // not, the enclosure cannot be trusted to *count* compatible integers, and
    // the honest result is `Indeterminate`. This is an epistemic limit of the
    // arithmetic enclosure, not a resource exhaustion.
    if !period_resolvable(period_mag, periodic) {
        return Ok(DeckSolveResult::Indeterminate);
    }

    // Conservative integer superset from the outward-rounded quotient.
    let (k_min, k_max) = integer_quotient_range(signed_period, periodic)?;
    if k_min > k_max {
        return Ok(DeckSolveResult::NoCompatibleInteger);
    }

    // Constant-time classification via the contiguous-compatible property.
    let first = first_compatible(signed_period, periodic, k_min, k_max);
    let last = last_compatible(signed_period, periodic, k_min, k_max);
    match (first, last) {
        (Some(a), Some(b)) if a == b => Ok(DeckSolveResult::Unique(a)),
        (Some(_), Some(_)) => Ok(DeckSolveResult::MultipleCompatibleIntegers),
        // No integer in the conservative range is compatible: the true set is
        // empty (the range was extended by rounding past an interval that holds
        // no integer multiple).
        _ => Ok(DeckSolveResult::NoCompatibleInteger),
    }
}

/// Whether the period is large enough, relative to the displacement scale, for
/// adjacent deck integers to be distinguishable in `f64`.
fn period_resolvable(period_mag: f64, interval: DeckInterval) -> bool {
    let scale = interval
        .lower()
        .get()
        .abs()
        .max(interval.upper().get().abs())
        .max(period_mag);
    period_mag > scale * f64::EPSILON
}

/// The conservative integer range `k` with `k * period` possibly in `interval`.
///
/// Always a superset of the true set: every integer not proven impossible is
/// included. `ceil` of the conservative-lower quotient and `floor` of the
/// conservative-upper quotient, both rounded outward before the integer step.
fn integer_quotient_range(
    period: f64,
    interval: DeckInterval,
) -> Result<(i64, i64), DeckOperationalFailure> {
    let plo = interval.lower().get();
    let phi = interval.upper().get();
    // Dividing by a negative period reverses the quotient interval.
    let (raw_lower_quotient, raw_upper_quotient) = if period > 0.0 {
        (plo / period, phi / period)
    } else {
        (phi / period, plo / period)
    };
    if !raw_lower_quotient.is_finite() || !raw_upper_quotient.is_finite() {
        return Err(DeckOperationalFailure::ArithmeticOverflow);
    }
    let lower = toward_neg(raw_lower_quotient)?;
    let upper = toward_pos(raw_upper_quotient)?;
    let k_min = ceil_to_i64(lower)?;
    let k_max = floor_to_i64(upper)?;
    Ok((k_min, k_max))
}

/// Whether `k * period` is provably outside `interval`.
///
/// Returns `false` whenever `k * period` could still lie inside, so the caller
/// never excludes a truly-compatible integer (no false negatives). `k * period`
/// is bounded by a one-ulp enclosure on each side before the comparison.
fn provably_outside(k: i64, period: f64, interval: DeckInterval) -> bool {
    let kp = (k as f64) * period;
    // If the multiplication itself is non-finite the test cannot certify
    // "outside"; treat it as not-provably-outside (safe, no false negative).
    let Ok(kp_lo) = toward_neg(kp) else {
        return false;
    };
    let Ok(kp_hi) = toward_pos(kp) else {
        return false;
    };
    // k*period < lower (true)  <=>  kp_hi < lower
    // k*period > upper (true)  <=>  kp_lo > upper
    kp_hi < interval.lower().get() || kp_lo > interval.upper().get()
}

/// The first compatible integer in `[k_min, k_max]`, scanning upward.
///
/// The conservative range extends the true compatible range by at most one
/// below, so the first compatible candidate is `k_min` or `k_min + 1`; checking
/// those two (clamped to the range) is sufficient and never enumerates a wide
/// interval. `None` if no integer in the range is compatible.
fn first_compatible(period: f64, interval: DeckInterval, k_min: i64, k_max: i64) -> Option<i64> {
    for k in [k_min, k_min + 1] {
        if k > k_max {
            break;
        }
        if !provably_outside(k, period, interval) {
            return Some(k);
        }
    }
    None
}

/// The last compatible integer in `[k_min, k_max]`, scanning downward.
///
/// Symmetric to [`first_compatible`]: the last compatible candidate is `k_max`
/// or `k_max - 1`.
fn last_compatible(period: f64, interval: DeckInterval, k_min: i64, k_max: i64) -> Option<i64> {
    for k in [k_max, k_max - 1] {
        if k < k_min {
            break;
        }
        if !provably_outside(k, period, interval) {
            return Some(k);
        }
    }
    None
}

/// Number of integers in `[k_min, k_max]`, with overflow guard.
fn integer_range_count(k_min: i64, k_max: i64) -> Result<u64, DeckOperationalFailure> {
    debug_assert!(k_min <= k_max, "caller checks ordering");
    let span = (k_max as i128) - (k_min as i128) + 1;
    if span < 0 || span > u64::MAX as i128 {
        return Err(DeckOperationalFailure::ArithmeticOverflow);
    }
    Ok(span as u64)
}

/// `ceil` of an `f64` to `i64`, refusing values outside the representable range.
fn ceil_to_i64(x: f64) -> Result<i64, DeckNumericFailure> {
    int_from_float(x.ceil(), x)
}

/// `floor` of an `f64` to `i64`, refusing values outside the representable range.
fn floor_to_i64(x: f64) -> Result<i64, DeckNumericFailure> {
    int_from_float(x.floor(), x)
}

/// Cast a rounded float to `i64`, guarding the `i64` range. The original `x` is
/// checked for finiteness; the rounded value is range-checked well inside
/// `i64`'s span so the cast is exact for every value the deck ever sees.
fn int_from_float(rounded: f64, original: f64) -> Result<i64, DeckNumericFailure> {
    if !original.is_finite() || !rounded.is_finite() {
        return Err(DeckNumericFailure::NotFinite);
    }
    // `f64` cannot distinguish integers above 2^53; keep the guard well inside.
    const I64_GUARD_FLOOR: f64 = -9.0e18;
    const I64_GUARD_CEIL: f64 = 9.0e18;
    if rounded < I64_GUARD_FLOOR || rounded > I64_GUARD_CEIL {
        return Err(DeckNumericFailure::IntegerOverflow);
    }
    Ok(rounded as i64)
}

// ---------------------------------------------------------------------------
// Milestone 1B: the cover interval
// ---------------------------------------------------------------------------

/// The certified set of deck indices whose translate can meet a Minkowski
/// difference of two developed enclosures.
///
/// A conservative superset by construction: every integer not proven impossible
/// is included. A broad difference yields a broad `Finite` range — two `i64`
/// endpoints cost nothing to compute — and a budget on *materializing* those
/// copies is enforced separately at the working-cover step
/// ([`DeckOperationalFailure::CoverBudgetExceeded`]). `Indeterminate` is
/// reserved for the case where the arithmetic enclosure itself cannot resolve
/// adjacent deck integers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CertifiedDeckInterval {
    /// A finite contiguous integer range `[min, max]`.
    Finite {
        /// The least included deck index.
        min: i64,
        /// The greatest included deck index.
        max: i64,
    },
    /// No deck index can meet the difference.
    Empty,
    /// The period is too small, at the displacement scale, for the arithmetic
    /// enclosure to resolve adjacent deck integers.
    Indeterminate,
}

impl CertifiedDeckInterval {
    /// A short stable tag, for probe records.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::Finite { .. } => "cover_finite",
            Self::Empty => "cover_empty",
            Self::Indeterminate => "cover_indeterminate",
        }
    }

    /// Whether a given integer is included.
    pub fn contains(&self, k: i64) -> bool {
        match self {
            Self::Finite { min, max } => k >= *min && k <= *max,
            Self::Empty => false,
            Self::Indeterminate => true,
        }
    }

    /// The number of integers, if finite. `None` for empty or indeterminate.
    pub fn finite_count(&self) -> Option<u64> {
        match self {
            Self::Finite { min, max } => Some(integer_range_count(*min, *max).ok()?),
            _ => None,
        }
    }
}

/// Compute `K_ij = { k in Z : (B_i - B_j) ∩ {k g} != empty }`.
///
/// The Minkowski difference is taken per coordinate with outward rounding. On
/// the generator's structural-zero (aperiodic) axis the difference must contain
/// zero, or no translate can meet it. On the periodic axis the quotient of the
/// difference interval by the period gives the conservative integer range,
/// returned as a `Finite` interval of two `i64` endpoints regardless of width.
///
/// Every integer not proven impossible is included. This never refuses a
/// relevant deck index: a false positive is acceptable, a false negative is P0.
/// A budget on the *number of materialized copies* is enforced at the
/// working-cover step, not here.
pub fn deck_cover_interval(
    generator: &DeckGenerator,
    a: &DevelopedBox,
    b: &DevelopedBox,
) -> Result<CertifiedDeckInterval, DeckOperationalFailure> {
    let difference = a
        .minkowski_difference(*b)
        .map_err(DeckOperationalFailure::from)?;

    // Structural-zero axis: {k g} has zero component here, so the difference
    // must contain zero. A definite test on a certified interval — never an
    // `== 0.0` float equality.
    let aperiodic_difference = difference.aperiodic(generator);
    if !aperiodic_difference.contains(0.0) {
        return Ok(CertifiedDeckInterval::Empty);
    }

    let periodic_difference = difference.periodic(generator);
    let signed_period = generator.signed_period().get();
    let period_mag = generator.period_magnitude().get();

    if !period_resolvable(period_mag, periodic_difference) {
        return Ok(CertifiedDeckInterval::Indeterminate);
    }

    let (k_min, k_max) = integer_quotient_range(signed_period, periodic_difference)?;
    if k_min > k_max {
        return Ok(CertifiedDeckInterval::Empty);
    }
    Ok(CertifiedDeckInterval::Finite {
        min: k_min,
        max: k_max,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gen(period: f64, axis: DevelopedAxis) -> DeckGenerator {
        DeckGenerator::new(axis, FiniteF64::new(period).unwrap()).unwrap()
    }

    fn interval(lo: f64, hi: f64) -> DeckInterval {
        DeckInterval::from_f64(lo, hi).unwrap()
    }

    fn box_periodic_aperiodic(
        periodic: (f64, f64),
        aperiodic: (f64, f64),
        generator: &DeckGenerator,
    ) -> DevelopedBox {
        let pi = interval(periodic.0, periodic.1);
        let ai = interval(aperiodic.0, aperiodic.1);
        match generator.periodic_axis() {
            DevelopedAxis::First => DevelopedBox {
                first: pi,
                second: ai,
            },
            DevelopedAxis::Second => DevelopedBox {
                first: ai,
                second: pi,
            },
        }
    }

    fn displacement(
        periodic: (f64, f64),
        aperiodic: (f64, f64),
        generator: &DeckGenerator,
    ) -> DevelopedBox {
        box_periodic_aperiodic(periodic, aperiodic, generator)
    }

    // ----- Interval arithmetic -------------------------------------------

    #[test]
    fn next_after_matches_ieee_semantics() {
        use super::next_after;
        // Stepping a positive value up increases it.
        assert!(next_after(1.0, f64::INFINITY) > 1.0);
        // Stepping a positive value down decreases it.
        assert!(next_after(1.0, f64::NEG_INFINITY) < 1.0);
        // Stepping a negative value toward +inf increases it (magnitude shrinks).
        assert!(next_after(-1.0, f64::INFINITY) > -1.0);
        assert!(next_after(-1.0, f64::NEG_INFINITY) < -1.0);
        // Stepping through zero lands on a signed subnormal.
        let up = next_after(0.0_f64, 1.0);
        assert!(up > 0.0 && up.is_subnormal());
        let down = next_after(0.0_f64, -1.0);
        assert!(down < 0.0);
        // Equal target returns it.
        assert_eq!(next_after(2.5, 2.5), 2.5);
        // One step is exactly one ulp at the value's magnitude.
        let step = next_after(1.0, f64::INFINITY) - 1.0;
        assert!((step - f64::EPSILON).abs() < 1e-30);
    }

    #[test]
    fn interval_rejects_inverted_bounds() {
        assert!(matches!(
            DeckInterval::from_f64(1.0, 0.0),
            Err(DeckConstructorFailure::BoundsInverted)
        ));
    }

    #[test]
    fn interval_add_is_outward_rounded_and_contains_truth() {
        // [1, 2] + [10, 20] = [11, 22]. The true sum endpoints are exactly
        // representable, but the outward rounding still produces an interval
        // that contains them.
        let a = interval(1.0, 2.0);
        let b = interval(10.0, 20.0);
        let s = a.add(b).unwrap();
        assert!(s.lower().get() <= 11.0);
        assert!(s.upper().get() >= 22.0);
    }

    #[test]
    fn interval_sub_is_outward_rounded() {
        // [10, 20] - [1, 2] = [8, 19].
        let a = interval(10.0, 20.0);
        let b = interval(1.0, 2.0);
        let d = a.sub(b).unwrap();
        assert!(d.lower().get() <= 8.0);
        assert!(d.upper().get() >= 19.0);
    }

    #[test]
    fn conservative_width_overestimates() {
        let i = interval(0.0, 1.0);
        let w = i.conservative_width().unwrap();
        assert!(w.get() >= 1.0);
    }

    // ----- DeckSolver: uniqueness and refusal ----------------------------

    #[test]
    fn unique_zero() {
        // Displacement exactly zero on the periodic axis, period 2*pi.
        let g = gen(std::f64::consts::TAU, DevelopedAxis::First);
        let d = displacement((0.0, 0.0), (0.0, 0.0), &g);
        assert_eq!(
            solve_axis_aligned(&g, &d).unwrap(),
            DeckSolveResult::Unique(0)
        );
    }

    #[test]
    fn unique_positive_integer() {
        // One full revolution forward: displacement == period.
        let p = std::f64::consts::TAU;
        let g = gen(p, DevelopedAxis::First);
        let d = displacement((p, p), (0.0, 0.0), &g);
        assert_eq!(
            solve_axis_aligned(&g, &d).unwrap(),
            DeckSolveResult::Unique(1)
        );
    }

    #[test]
    fn unique_negative_integer() {
        let p = std::f64::consts::TAU;
        let g = gen(p, DevelopedAxis::First);
        let d = displacement((-3.0 * p, -3.0 * p), (0.0, 0.0), &g);
        assert_eq!(
            solve_axis_aligned(&g, &d).unwrap(),
            DeckSolveResult::Unique(-3)
        );
    }

    #[test]
    fn large_valid_integer() {
        let p = std::f64::consts::TAU;
        let g = gen(p, DevelopedAxis::First);
        let k = 12345_i64;
        let target = k as f64 * p;
        let d = displacement((target, target), (0.0, 0.0), &g);
        assert_eq!(
            solve_axis_aligned(&g, &d).unwrap(),
            DeckSolveResult::Unique(k)
        );
    }

    #[test]
    fn no_compatible_integer() {
        // Half a period, well away from any integer multiple.
        let p = std::f64::consts::TAU;
        let g = gen(p, DevelopedAxis::First);
        let d = displacement((0.5 * p, 0.5 * p), (0.0, 0.0), &g);
        assert_eq!(
            solve_axis_aligned(&g, &d).unwrap(),
            DeckSolveResult::NoCompatibleInteger
        );
    }

    #[test]
    fn multiple_compatible_integers() {
        // A broad enclosure spanning several periods (width > period).
        let p = std::f64::consts::TAU;
        let g = gen(p, DevelopedAxis::First);
        // [0, 3p] spans three full periods: at least k=0,1,2,3 land inside.
        let d = displacement((0.0, 3.0 * p), (0.0, 0.0), &g);
        assert_eq!(
            solve_axis_aligned(&g, &d).unwrap(),
            DeckSolveResult::MultipleCompatibleIntegers
        );
    }

    #[test]
    fn broad_interval_is_multiple_not_indeterminate() {
        // A very wide enclosure spans many periods. Broad evidence is not
        // turned into Indeterminate: the solver proves MultipleCompatibleIntegers
        // from its first two compatible candidates, without enumerating the
        // whole interval. (A budget on *materialized copies* is a separate,
        // operational concern enforced at the working-cover step.)
        let p = std::f64::consts::TAU;
        let g = gen(p, DevelopedAxis::First);
        let d = displacement((-1.0e6 * p, 1.0e6 * p), (0.0, 0.0), &g);
        assert_eq!(
            solve_axis_aligned(&g, &d).unwrap(),
            DeckSolveResult::MultipleCompatibleIntegers
        );
    }

    #[test]
    fn indeterminate_when_period_unresolvable() {
        // The period is smaller than the ULP at the displacement scale, so
        // adjacent deck integers are indistinguishable in f64. The arithmetic
        // enclosure cannot be trusted to count compatible integers: the honest
        // result is Indeterminate. This is an epistemic limit, not a budget.
        let g = gen(1.0e-20, DevelopedAxis::First);
        let d = displacement((1.0, 1.0), (0.0, 0.0), &g);
        assert_eq!(
            solve_axis_aligned(&g, &d).unwrap(),
            DeckSolveResult::Indeterminate
        );
    }

    #[test]
    fn aperiodic_coordinate_contradiction() {
        // Periodic component is a clean integer multiple, but the aperiodic
        // component provably excludes zero: no integer is compatible.
        let p = std::f64::consts::TAU;
        let g = gen(p, DevelopedAxis::First);
        let d = displacement((0.0, 0.0), (1.0, 2.0), &g);
        assert_eq!(
            solve_axis_aligned(&g, &d).unwrap(),
            DeckSolveResult::NoCompatibleInteger
        );
    }

    #[test]
    fn period_sign_reversal() {
        // A negative period reverses the quotient but yields the same placement.
        let p = std::f64::consts::TAU;
        let g_pos = gen(p, DevelopedAxis::First);
        let g_neg = gen(-p, DevelopedAxis::First);
        // Displacement of one positive copy under g_pos is k=1; under g_neg the
        // same physical displacement is k=-1 (the generator points the other
        // way). Both must be certified Unique.
        let d = displacement((p, p), (0.0, 0.0), &g_pos);
        assert_eq!(
            solve_axis_aligned(&g_pos, &d).unwrap(),
            DeckSolveResult::Unique(1)
        );
        let d = displacement((-p, -p), (0.0, 0.0), &g_neg);
        assert_eq!(
            solve_axis_aligned(&g_neg, &d).unwrap(),
            DeckSolveResult::Unique(1)
        );
    }

    #[test]
    fn periodic_axis_swap() {
        // Same numbers, but the period lives on the second developed axis.
        let p = std::f64::consts::TAU;
        let g = gen(p, DevelopedAxis::Second);
        let d = displacement((p, p), (0.0, 0.0), &g);
        assert_eq!(
            solve_axis_aligned(&g, &d).unwrap(),
            DeckSolveResult::Unique(1)
        );
    }

    #[test]
    fn near_integer_refusal() {
        // A displacement a sliver short of one period. A naive `round(d / p)`
        // would round `0.9999...` to `1`; the solver must NOT, because the
        // displacement is provably not an integer multiple. It reports
        // NoCompatible rather than rounding to the nearest integer.
        let p = std::f64::consts::TAU;
        let g = gen(p, DevelopedAxis::First);
        let d = displacement((p - 1.0e-9, p - 1.0e-9), (0.0, 0.0), &g);
        assert_eq!(
            solve_axis_aligned(&g, &d).unwrap(),
            DeckSolveResult::NoCompatibleInteger,
            "a displacement a sliver short of a period must not be rounded to k=1"
        );
    }

    #[test]
    fn combined_error_radius_threshold() {
        // Each endpoint radius eps, difference radius 2*eps; the uniqueness
        // condition is |g| > 4*eps. Just above the threshold stays Unique; at
        // the threshold the test should not certify uniqueness.
        let p = std::f64::consts::TAU;
        let g = gen(p, DevelopedAxis::First);
        // eps small: 4*eps << p, so well unique.
        let eps = 1e-6;
        let d = displacement((-2.0 * eps, 2.0 * eps), (-2.0 * eps, 2.0 * eps), &g);
        assert_eq!(
            solve_axis_aligned(&g, &d).unwrap(),
            DeckSolveResult::Unique(0)
        );
        // eps large: 4*eps > p, separation fails.
        let eps_big = p; // 4*eps_big >> p
        let d = displacement((-2.0 * eps_big, 2.0 * eps_big), (0.0, 0.0), &g);
        let r = solve_axis_aligned(&g, &d).unwrap();
        assert!(
            !matches!(r, DeckSolveResult::Unique(_)),
            "above the 4*eps threshold uniqueness must not be certified, got {r:?}"
        );
    }

    #[test]
    fn overflow_or_envelope_exhaustion() {
        // Operational overflow reaches the pipeline through interval arithmetic
        // on extreme inputs: two boxes whose Minkowski difference leaves the
        // finite range. (The solver's own quotient cannot overflow once the
        // period is resolvable: resolvability bounds the quotient below
        // 1/EPSILON, far inside f64 and i64. A sub-resolvable period is
        // Indeterminate, not overflow.)
        let p = std::f64::consts::TAU;
        let g = gen(p, DevelopedAxis::First);
        let big = 1.0e308;
        let bi = aabb((0., 0.), (0., 0.), &g, (big, big), (0.0, 0.0));
        let bj = aabb((0., 0.), (0., 0.), &g, (-big, -big), (0.0, 0.0));
        assert!(matches!(
            deck_cover_interval(&g, &bi, &bj),
            Err(DeckOperationalFailure::ArithmeticOverflow)
        ));
    }

    #[test]
    fn zero_period_is_refused_at_construction() {
        assert!(matches!(
            DeckGenerator::new(DevelopedAxis::First, FiniteF64::new(0.0).unwrap()),
            Err(DeckConstructorFailure::ZeroPeriod)
        ));
    }

    // ----- Cover interval -------------------------------------------------

    fn aabb(
        first: (f64, f64),
        second: (f64, f64),
        generator: &DeckGenerator,
        periodic: (f64, f64),
        aperiodic: (f64, f64),
    ) -> DevelopedBox {
        // Helper kept explicit: tests name the periodic/aperiodic content.
        let _ = (first, second);
        box_periodic_aperiodic(periodic, aperiodic, generator)
    }

    #[test]
    fn cover_positive_period_single_candidate() {
        let p = std::f64::consts::TAU;
        let g = gen(p, DevelopedAxis::First);
        // B_i spans one period around k=0; B_j is the origin. Difference can
        // meet k=0 (and possibly a neighbour at the boundary).
        let bi = aabb((0., 0.), (0., 0.), &g, (-0.1, 0.1), (0.0, 0.0));
        let bj = aabb((0., 0.), (0., 0.), &g, (0.0, 0.0), (0.0, 0.0));
        let cover = deck_cover_interval(&g, &bi, &bj).unwrap();
        assert!(cover.contains(0), "k=0 must be in the cover, got {cover:?}");
    }

    #[test]
    fn cover_negative_period() {
        let p = std::f64::consts::TAU;
        let g = gen(-p, DevelopedAxis::First);
        let bi = aabb((0., 0.), (0., 0.), &g, (-0.1, 0.1), (0.0, 0.0));
        let bj = aabb((0., 0.), (0., 0.), &g, (0.0, 0.0), (0.0, 0.0));
        let cover = deck_cover_interval(&g, &bi, &bj).unwrap();
        assert!(cover.contains(0));
    }

    #[test]
    fn cover_periodic_axis_swap() {
        let p = std::f64::consts::TAU;
        let g = gen(p, DevelopedAxis::Second);
        let bi = aabb((0., 0.), (0., 0.), &g, (-0.1, 0.1), (0.0, 0.0));
        let bj = aabb((0., 0.), (0., 0.), &g, (0.0, 0.0), (0.0, 0.0));
        let cover = deck_cover_interval(&g, &bi, &bj).unwrap();
        assert!(cover.contains(0));
    }

    #[test]
    fn cover_structural_zero_component_excludes() {
        // The aperiodic difference excludes zero -> empty cover.
        let p = std::f64::consts::TAU;
        let g = gen(p, DevelopedAxis::First);
        let bi = aabb((0., 0.), (0., 0.), &g, (0.0, 0.0), (5.0, 6.0));
        let bj = aabb((0., 0.), (0., 0.), &g, (0.0, 0.0), (0.0, 0.0));
        let cover = deck_cover_interval(&g, &bi, &bj).unwrap();
        assert_eq!(cover, CertifiedDeckInterval::Empty);
    }

    #[test]
    fn cover_empty_overlap() {
        // Boxes separated by more than a period on the periodic axis.
        let p = std::f64::consts::TAU;
        let g = gen(p, DevelopedAxis::First);
        // B_i near k=0, B_j near k=5p but only the difference by integer
        // multiples is relevant; difference near 0 only if they overlap modulo p.
        let bi = aabb((0., 0.), (0., 0.), &g, (0.0, 0.1), (0.0, 0.0));
        let bj = aabb((0., 0.), (0., 0.), &g, (0.3 * p, 0.3 * p + 0.1), (0.0, 0.0));
        let cover = deck_cover_interval(&g, &bi, &bj).unwrap();
        // Difference periodic component is around -0.3p, which is not within an
        // integer multiple neighborhood of zero that includes any k... it is a
        // single point at -0.3p, no integer k has k*p == -0.3p.
        assert_eq!(cover, CertifiedDeckInterval::Empty);
    }

    #[test]
    fn cover_single_candidate() {
        let p = std::f64::consts::TAU;
        let g = gen(p, DevelopedAxis::First);
        // Difference exactly zero -> k=0 only.
        let bi = aabb((0., 0.), (0., 0.), &g, (0.0, 0.0), (0.0, 0.0));
        let bj = aabb((0., 0.), (0., 0.), &g, (0.0, 0.0), (0.0, 0.0));
        let cover = deck_cover_interval(&g, &bi, &bj).unwrap();
        match cover {
            CertifiedDeckInterval::Finite { min, max } => {
                assert!(min <= 0 && max >= 0, "k=0 included");
            }
            other => panic!("expected finite, got {other:?}"),
        }
    }

    #[test]
    fn cover_multiple_candidates() {
        let p = std::f64::consts::TAU;
        let g = gen(p, DevelopedAxis::First);
        // A wide box minus a point spans many periods.
        let bi = aabb((0., 0.), (0., 0.), &g, (-2.0 * p, 2.0 * p), (0.0, 0.0));
        let bj = aabb((0., 0.), (0., 0.), &g, (0.0, 0.0), (0.0, 0.0));
        let cover = deck_cover_interval(&g, &bi, &bj).unwrap();
        match cover {
            CertifiedDeckInterval::Finite { min, max } => {
                assert!(min <= -2 && max >= 2);
            }
            other => panic!("expected finite, got {other:?}"),
        }
    }

    #[test]
    fn cover_distant_valid_candidate() {
        let p = std::f64::consts::TAU;
        let g = gen(p, DevelopedAxis::First);
        // B_i near k=7 (i.e. near 7p), B_j at origin -> difference near 7p ->
        // k=7 must be included.
        let target = 7.0 * p;
        let bi = aabb(
            (0., 0.),
            (0., 0.),
            &g,
            (target - 0.05, target + 0.05),
            (0.0, 0.0),
        );
        let bj = aabb((0., 0.), (0., 0.), &g, (0.0, 0.0), (0.0, 0.0));
        let cover = deck_cover_interval(&g, &bi, &bj).unwrap();
        assert!(cover.contains(7), "k=7 must be included, got {cover:?}");
    }

    #[test]
    fn cover_outward_rounding_boundary_case() {
        // A difference landing exactly on an integer multiple must include that
        // integer: the outward-rounded quotient pulls `ceil`/`floor` past the
        // boundary so the index is never lost (false positives acceptable, false
        // negatives P0).
        let p = std::f64::consts::TAU;
        let g = gen(p, DevelopedAxis::First);
        let bi = aabb((0., 0.), (0., 0.), &g, (p, p), (0.0, 0.0));
        let bj = aabb((0., 0.), (0., 0.), &g, (0.0, 0.0), (0.0, 0.0));
        let cover = deck_cover_interval(&g, &bi, &bj).unwrap();
        assert!(
            cover.contains(1),
            "k=1 must be included at the boundary, got {cover:?}"
        );
    }

    #[test]
    fn cover_near_boundary_inclusion() {
        // The difference reaches k=1's multiple from below: must include 1.
        let p = std::f64::consts::TAU;
        let g = gen(p, DevelopedAxis::First);
        let bi = aabb((0., 0.), (0., 0.), &g, (p - 1.0e-6, p), (0.0, 0.0));
        let bj = aabb((0., 0.), (0., 0.), &g, (0.0, 0.0), (0.0, 0.0));
        let cover = deck_cover_interval(&g, &bi, &bj).unwrap();
        assert!(cover.contains(1), "k=1 must be included, got {cover:?}");
    }

    #[test]
    fn cover_broad_difference_is_finite_not_indeterminate() {
        // A huge difference spans many periods but is still just two i64
        // endpoints; computing it costs nothing and is not a budget event.
        // Broad evidence is Finite, not Indeterminate. A budget on
        // *materializing* the copies is enforced later, at the working-cover
        // step.
        let p = std::f64::consts::TAU;
        let g = gen(p, DevelopedAxis::First);
        let bi = aabb(
            (0., 0.),
            (0., 0.),
            &g,
            (-1000.0 * p, 1000.0 * p),
            (0.0, 0.0),
        );
        let bj = aabb((0., 0.), (0., 0.), &g, (0.0, 0.0), (0.0, 0.0));
        let cover = deck_cover_interval(&g, &bi, &bj).unwrap();
        match cover {
            CertifiedDeckInterval::Finite { min, max } => {
                assert!(
                    min <= -1000 && max >= 1000,
                    "broad cover kept finite: {cover:?}"
                );
            }
            other => panic!("broad evidence must be Finite, got {other:?}"),
        }
    }

    #[test]
    fn cover_unresolvable_period_is_indeterminate() {
        // A period below the displacement-scale ULP cannot be resolved: the
        // cover cannot certify which integers are relevant.
        let g = gen(1.0e-20, DevelopedAxis::First);
        let bi = aabb((0., 0.), (0., 0.), &g, (0.9, 1.1), (0.0, 0.0));
        let bj = aabb((0., 0.), (0., 0.), &g, (0.0, 0.0), (0.0, 0.0));
        let cover = deck_cover_interval(&g, &bi, &bj).unwrap();
        assert_eq!(cover, CertifiedDeckInterval::Indeterminate);
    }

    #[test]
    fn aperiodic_zero_containment_is_structural_not_float_equality() {
        // Aperiodic interval that contains zero only via its interior, not as an
        // exact endpoint, still admits the solve.
        let p = std::f64::consts::TAU;
        let g = gen(p, DevelopedAxis::First);
        let d = displacement((0.0, 0.0), (-0.3, 0.7), &g);
        assert_eq!(
            solve_axis_aligned(&g, &d).unwrap(),
            DeckSolveResult::Unique(0)
        );
    }

    // ----- Exact-arithmetic oracle ---------------------------------------
    //
    // For an integer-valued period (1.0, 2.0, ...) `k * period` is exact in f64
    // for |k| < 2^53, so the compatible set can be computed by brute force with
    // no rounding ambiguity. That brute force is an exact oracle for the solver,
    // exercised here at integer boundaries, negative periods, large indices,
    // subnormal-adjacent scales, and broad intervals.

    /// The exact compatible-integer classification, by brute force. `period`
    /// must be an integer-valued f64 so `k * period` is exact.
    fn oracle_classify(period: f64, plo: f64, phi: f64) -> DeckSolveResult {
        let pmag = period.abs();
        let bound = ((plo.abs().max(phi.abs()) / pmag).ceil() as i64) + 4;
        let mut count = 0_i64;
        let mut first = 0_i64;
        for k in -bound..=bound {
            let kp = k as f64 * period;
            if plo <= kp && kp <= phi {
                count += 1;
                if count == 1 {
                    first = k;
                }
            }
        }
        match count {
            0 => DeckSolveResult::NoCompatibleInteger,
            1 => DeckSolveResult::Unique(first),
            _ => DeckSolveResult::MultipleCompatibleIntegers,
        }
    }

    fn assert_matches_oracle(period: f64, plo: f64, phi: f64) {
        let g = gen(period, DevelopedAxis::First);
        // Aperiodic comfortably contains zero so it never decides the outcome.
        let d = displacement((plo, phi), (-0.5, 0.5), &g);
        let got = solve_axis_aligned(&g, &d).unwrap();
        let want = oracle_classify(period, plo, phi);
        assert_eq!(
            got, want,
            "period={period}, enclosure=[{plo}, {phi}]: solver {got:?} != oracle {want:?}"
        );
    }

    #[test]
    fn oracle_exact_integer_period_points_and_intervals() {
        for period in [1.0_f64, 2.0, 3.0, 5.0, 7.0, 10.0] {
            // Points exactly on multiples -> Unique.
            for k in -3_i64..=3 {
                let m = k as f64 * period;
                assert_matches_oracle(period, m, m);
            }
            // Points exactly between two multiples -> NoCompatible.
            let mid = 0.5 * period;
            assert_matches_oracle(period, mid, mid);
            // An interval spanning several periods -> Multiple.
            assert_matches_oracle(period, -2.0 * period, 2.0 * period);
            // A sliver short of one multiple -> NoCompatible (no rounding).
            assert_matches_oracle(period, period - 0.25, period - 0.25);
            // A sliver past one multiple -> NoCompatible.
            assert_matches_oracle(period, period + 0.25, period + 0.25);
            // A tight enclosure around one multiple -> Unique.
            assert_matches_oracle(period, period - 0.25, period + 0.25);
        }
    }

    #[test]
    fn oracle_negative_period_matches_positive() {
        // A negative period reverses deck sign but the *count* classification
        // must match the positive-period oracle on mirrored enclosures.
        for period in [1.0_f64, 2.0, 5.0] {
            for (plo, phi) in [
                (period, period),
                (2.0 * period, 2.0 * period),
                (-2.0 * period, 2.0 * period),
                (0.5 * period, 0.5 * period),
            ] {
                assert_matches_oracle(-period, plo, phi);
            }
        }
    }

    #[test]
    fn oracle_large_indices() {
        // Large |k|: k*period stays exact for integer period well within 2^53.
        let period = 2.0_f64;
        for k in [1_000_i64, 100_000, 1_000_000, -999_999] {
            let m = k as f64 * period;
            assert_matches_oracle(period, m, m);
            assert_matches_oracle(period, m - 0.5, m + 0.5);
        }
    }

    #[test]
    fn oracle_axis_swap_matches() {
        // The same numbers with the period on the second developed axis must
        // classify identically.
        let period = 3.0_f64;
        let g0 = gen(period, DevelopedAxis::First);
        let g1 = gen(period, DevelopedAxis::Second);
        for (plo, phi) in [
            (period, period),
            (0.5 * period, 0.5 * period),
            (-period, 2.0 * period),
        ] {
            let d0 = displacement((plo, phi), (-0.5, 0.5), &g0);
            let d1 = box_periodic_aperiodic((plo, phi), (-0.5, 0.5), &g1);
            let want = oracle_classify(period, plo, phi);
            assert_eq!(solve_axis_aligned(&g0, &d0).unwrap(), want);
            assert_eq!(solve_axis_aligned(&g1, &d1).unwrap(), want);
        }
    }
}
