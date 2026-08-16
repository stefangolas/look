//! Setting Tolerance
//!
//! The legacy absolute tolerances — `TOLERANCE`, the `Tolerance` and `Origin`
//! traits, and the assertion macros — sit beside the scale-relative context
//! `ToleranceCtx` that later packets migrate call sites onto, one crate at a
//! time. This module only adds the type; it migrates nothing.
//!
//! > Every migrated site carries `// BG-TOL-001: model` or `// BG-TOL-001: param`,
//! > naming which kind of quantity it compares. A site whose kind is genuinely
//! > unclear gets `FIXME(BG-TOL-001)` and is reported, never guessed — guessing
//! > converts an obvious absolute-tolerance bug into a subtle scale bug.

use crate::cgmath64::*;
use crate::evidence::{
    Budget, Certificate, Certified, EnvelopeCase, Margin, Method, Modulus, Outcome, PropMap,
    Refusal,
};
use cgmath::AbsDiffEq;
use std::fmt::Debug;

/// general tolerance
pub const TOLERANCE: f64 = 1.0e-6;

/// general tolerance of square order
pub const TOLERANCE2: f64 = TOLERANCE * TOLERANCE;

/// Defines a tolerance in the whole package
pub trait Tolerance: AbsDiffEq<Epsilon = f64> + Debug {
    /// The "distance" is less than `TOLERANCE`.
    fn near(&self, other: &Self) -> bool {
        self.abs_diff_eq(other, TOLERANCE)
    }

    /// The "distance" is less than `TOLERANCR2`.
    fn near2(&self, other: &Self) -> bool {
        self.abs_diff_eq(other, TOLERANCE2)
    }
}

impl<T: AbsDiffEq<Epsilon = f64> + Debug> Tolerance for T {}

/// Asserts that `left.near(&right)` (using `Tolerance`).
#[macro_export]
macro_rules! assert_near {
    ($left: expr, $right: expr $(,)?) => {{
        let (left, right) = ($left, $right);
        assert!(
            $crate::tolerance::Tolerance::near(&left, &right),
            "assertion failed: `left` is near `right`\nleft: {left:?},\nright: {right:?}",
        )
    }};
    ($left: expr, $right: expr, $($arg: tt)+) => {{
        let (left, right) = ($left, $right);
        assert!(
            $crate::tolerance::Tolerance::near(&left, &right),
            "assertion failed: `left` is near `right`\nleft: {left:?},\nright: {right:?}: {}",
            format_args!($($arg)+),
        )
    }};
}

/// Similar to `assert_near!`, but returns a test failure instead of panicking if the condition fails.
#[macro_export]
macro_rules! prop_assert_near {
    ($left: expr, $right: expr $(,)?) => {{
        let (left, right) = ($left, $right);
        prop_assert!(
            $crate::tolerance::Tolerance::near(&left, &right),
            "assertion failed: `left` is near `right`\nleft: {left:?},\nright: {right:?}",
        )
    }};
    ($left: expr, $right: expr, $($arg: tt)+) => {{
        let (left, right) = ($left, $right);
        prop_assert!(
            $crate::tolerance::Tolerance::near(&left, &right),
            "assertion failed: `left` is near `right`\nleft: {left:?}, right: {right:?}: {}",
            format_args!($($arg)+),
        )
    }};
}

#[test]
#[should_panic]
fn assert_near_without_msg() {
    assert_near!(1.0, 2.0)
}

#[test]
#[should_panic]
fn assert_near_with_msg() {
    assert_near!(1.0, 2.0, "{}", "test OK")
}

/// Asserts that `left.near2(&right)` (using `Tolerance`).
#[macro_export]
macro_rules! assert_near2 {
    ($left: expr, $right: expr $(,)?) => {{
        let (left, right) = ($left, $right);
        assert!(
            $crate::tolerance::Tolerance::near2(&left, &right),
            "assertion failed: `left` is near `right`\nleft: {left:?},\nright: {right:?}",
        )
    }};
    ($left: expr, $right: expr, $($arg: tt)+) => {{
        let (left, right) = ($left, $right);
        assert!(
            $crate::tolerance::Tolerance::near2(&left, &right),
            "assertion failed: `left` is near `right`\nleft: {left:?},\nright: {right:?}: {}",
            format_args!($($arg)+),
        )
    }};
}

/// Similar to `assert_near2!`, but returns a test failure instead of panicking if the condition fails.
#[macro_export]
macro_rules! prop_assert_near2 {
    ($left: expr, $right: expr $(,)?) => {{
        let (left, right) = ($left, $right);
        prop_assert!(
            $crate::tolerance::Tolerance::near2(&left, &right),
            "assertion failed: `left` is near `right`\nleft: {left:?},\nright: {right:?}",
        )
    }};
    ($left: expr, $right: expr, $($arg: tt)+) => {
        let (left, right) = ($left, $right);
        prop_assert!(
            $crate::tolerance::Tolerance::near2(&left, &right),
            "assertion failed: `left` is near `right`\nleft: {left:?},\nright: {right:?}: {}",
            format_args!($($arg)+),
        )
    };
}

#[test]
#[should_panic]
fn assert_near2_without_msg() {
    assert_near2!(1.0, 2.0)
}

#[test]
#[should_panic]
fn assert_near2_with_msg() {
    assert_near2!(1.0, 2.0, "{}", "test OK")
}

/// The structs defined the origin. `f64`, `Vector`, and so on.
pub trait Origin: Tolerance + Zero {
    /// near origin
    #[inline(always)]
    fn so_small(&self) -> bool {
        self.near(&Self::zero())
    }

    /// near origin in square order
    #[inline(always)]
    fn so_small2(&self) -> bool {
        self.near2(&Self::zero())
    }
}

impl<T: Tolerance + Zero> Origin for T {}

/// The three tolerance budgets of the formal system, carried together with the
/// scale they are relative to.
///
/// `model_scale` is the declared characteristic length of the model. Every
/// **model-space** comparison in the kernel is `tau * model_scale`; every
/// **dimensionless** comparison is `tau` alone. Which one a call site needs is
/// a judgement the call site must state, never a default this type picks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToleranceCtx {
    model_scale: f64,
    /// Backward: the perturbation admitted by validation and repair.
    pub tau_in: f64,
    /// Representation error.
    pub tau_rep: f64,
    /// The collapse quotient.
    pub tau_col: f64,
}

impl ToleranceCtx {
    /// Refuses a `model_scale` that is not finite and strictly positive: a
    /// zero, negative, or NaN scale makes every length predicate below
    /// meaningless, and silently substituting 1.0 would make a wrong answer
    /// look like a right one.
    pub fn new(model_scale: f64, tau_in: f64, tau_rep: f64, tau_col: f64) -> Outcome<Self> {
        if !model_scale.is_finite() || model_scale <= 0.0 {
            return Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ChartDegenerate));
        }
        if !tau_in.is_finite()
            || tau_in < 0.0
            || !tau_rep.is_finite()
            || tau_rep < 0.0
            || !tau_col.is_finite()
            || tau_col < 0.0
        {
            return Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ChartDegenerate));
        }
        Ok(Certified::new(
            Self {
                model_scale,
                tau_in,
                tau_rep,
                tau_col,
            },
            Certificate {
                props: PropMap::new(),
                // The context is validated float arithmetic, never exact (H-6).
                method: Method::Float,
                budget_left: Budget::new(0, 0, 0),
                margin: Margin::UNBOUNDED,
                modulus: Modulus::Unbounded,
            },
        ))
    }

    /// The same context at a different model scale (BG-TOL-002). The taus are
    /// dimensionless ratios and therefore unchanged; only the scale moves.
    pub fn scaled(&self, s: f64) -> Outcome<Self> {
        Self::new(s, self.tau_in, self.tau_rep, self.tau_col)
    }

    /// The migration scaffold for BG-TOL-001 Stage A: a context whose predicates
    /// are numerically the legacy absolute ones.
    ///
    /// `model_scale` is 1.0 and `tau_rep` is [`TOLERANCE`], so `is_small_len` and
    /// `is_small_ratio` use exactly the epsilon the legacy `Tolerance` trait used.
    /// A site migrated onto this context therefore keeps its present behaviour;
    /// what the migration buys is that the site now *states* whether it compares a
    /// model-space length or a dimensionless quantity, which is the judgement that
    /// cannot be made mechanically later.
    ///
    /// **This is scaffolding and is expected to reach zero uses.** A real
    /// `model_scale` comes from the model, and every call here is a site whose
    /// entry point has not yet been threaded (Stage B). `scripts/kernel-gates.sh`
    /// counts these against a ceiling that only moves down; BG-TOL-001 is not
    /// discharged until the count is zero. Do not call it from new code that has a
    /// real scale available.
    ///
    /// Infallible by construction — every argument is a compile-time constant that
    /// `new` accepts — so it returns `Self`, not `Outcome<Self>`. That is
    /// deliberate: an `Outcome` here would force ~184 migration sites to handle an
    /// error that cannot occur, and H-1 forbids the `unwrap` they would reach for.
    ///
    /// `near_pt` is deliberately the Euclidean predicate, not the legacy
    /// componentwise one: a `(TOLERANCE, TOLERANCE, TOLERANCE)` difference has
    /// magnitude `TOLERANCE * sqrt(3)` and is rejected here even though every
    /// coordinate is within `TOLERANCE`, because a tolerance that depends on the
    /// coordinate frame is not a tolerance.
    pub fn unscaled_legacy() -> Self {
        Self {
            model_scale: 1.0, // H-3: a dimensionless scale of 1.0, so tau * scale is the legacy absolute epsilon
            tau_in: TOLERANCE,
            tau_rep: TOLERANCE,
            tau_col: TOLERANCE,
        }
    }

    /// The declared characteristic length.
    pub fn model_scale(&self) -> f64 {
        self.model_scale
    }

    /// MODEL-SPACE. True when `a` and `b` are within representation tolerance,
    /// scaled by the model: `|a - b| <= tau_rep * model_scale`.
    pub fn near_pt(&self, a: Point3, b: Point3) -> bool {
        (a - b).magnitude() <= self.tau_rep * self.model_scale
    }

    /// MODEL-SPACE. True when a length is negligible at this model's scale.
    pub fn is_small_len(&self, l: f64) -> bool {
        l.abs() <= self.tau_rep * self.model_scale
    }

    /// DIMENSIONLESS — deliberately NOT scaled. A sine is a ratio; multiplying
    /// a ratio by a length is a category error. Callers comparing angles, knot
    /// values, normalized parameters or weights use this and nothing else.
    pub fn sin_margin(&self) -> f64 {
        self.tau_rep
    }

    /// DIMENSIONLESS. True when a ratio-valued quantity is within `sin_margin`.
    pub fn is_small_ratio(&self, x: f64) -> bool {
        x.abs() <= self.tau_rep
    }

    /// BG-TOL-003: an entity's tolerance may never be tighter than its
    /// boundary's. Returns the entity tolerance to use given a boundary
    /// tolerance, which is the larger of the two.
    pub fn entity_tau(&self, boundary_tau: f64) -> f64 {
        self.tau_rep.max(boundary_tau)
    }
}
