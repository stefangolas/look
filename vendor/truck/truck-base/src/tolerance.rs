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

    /// MODEL-SPACE, generic over the point type. True when `a` and `b` are within
    /// representation tolerance, scaled by the model.
    ///
    /// [`Self::near_pt`] is this specialised to `Point3` and is kept because it is
    /// the common case and reads better at a call site. Generic code — the
    /// topology crate is generic over its point type, and cannot name `Point3` —
    /// uses this.
    pub fn near_points<P>(&self, a: P, b: P) -> bool
    where
        P: MetricSpace<Metric = f64>,
    {
        a.distance(b) <= self.tau_rep * self.model_scale
    }

    /// MODEL-SPACE. True when a length is negligible at this model's scale.
    pub fn is_small_len(&self, l: f64) -> bool {
        l.abs() <= self.tau_rep * self.model_scale
    }

    /// MODEL-SPACE. The absolute margin a length comparison uses at this model's
    /// scale: `tau_rep * model_scale`.
    ///
    /// This exists for **one-sided** comparisons, which the symmetric predicates
    /// cannot express. `a < b + ctx.length_margin()` asks whether `a` is at or
    /// below `b` within tolerance; `is_small_len(a - b)` asks whether they are
    /// close, and answers differently for every `a` far below `b`. Turning a
    /// one-sided comparison into a symmetric one is a behaviour change disguised
    /// as a migration.
    pub fn length_margin(&self) -> f64 {
        self.tau_rep * self.model_scale
    }

    /// DIMENSIONLESS — deliberately NOT scaled. A sine is a ratio; multiplying
    /// a ratio by a length is a category error. Callers comparing angles, knot
    /// values, normalized parameters or weights use this and nothing else.
    pub fn sin_margin(&self) -> f64 {
        self.tau_rep
    }

    /// DIMENSIONLESS — deliberately NOT scaled. The one-sided counterpart of
    /// [`Self::sin_margin`], named for what it bounds rather than for sines, since
    /// most call sites comparing it are comparing curve parameters and knot values
    /// rather than angles. Identical in value to `sin_margin`; both return
    /// `tau_rep`. They are kept separate because they are named for different
    /// quantities, and a later packet that gives angles their own tolerance will
    /// change one and not the other.
    pub fn ratio_margin(&self) -> f64 {
        self.tau_rep
    }

    /// DIMENSIONLESS. True when a ratio-valued quantity is within `sin_margin`.
    pub fn is_small_ratio(&self, x: f64) -> bool {
        x.abs() <= self.tau_rep
    }

    /// MODEL-SPACE, DEGREE 2 IN LENGTH. The absolute margin a squared-length
    /// comparison uses at this model's scale: `(tau_rep * model_scale)^2`.
    ///
    /// The one-sided squared counterpart of [`Self::length_margin`], for
    /// quantities that are degree two in length: squared distances, squared
    /// magnitudes, twice a triangle's area. Under a model rescale by `k` such a
    /// quantity scales as `k^2`, and so does this margin.
    pub fn length2_margin(&self) -> f64 {
        self.length_margin() * self.length_margin()
    }

    /// MODEL-SPACE, DEGREE 2 IN LENGTH. True when a quantity of degree two in
    /// length is negligible at this model's scale: `q <= (tau_rep *
    /// model_scale)^2`.
    ///
    /// This is the sqrt-free form of [`Self::is_small_len`] for squared
    /// distances: `d.distance2(c) <= TOLERANCE2` migrates to
    /// `ctx.is_small_len2(d.distance2(c))` with identical behaviour at Stage A
    /// (`model_scale == 1.0` makes the margin exactly `TOLERANCE2`). At the
    /// boundary it can differ from `is_small_len(q.sqrt())` by one ulp — the
    /// squared form is the predicate, not an approximation of the sqrt form.
    /// The argument must be non-negative by construction (a squared distance,
    /// an area); `.abs()` is applied anyway so a stray negative is small
    /// rather than silently never-small.
    pub fn is_small_len2(&self, q: f64) -> bool {
        q.abs() <= self.length2_margin()
    }

    /// DIMENSIONLESS, DEGREE ZERO — deliberately NOT scaled, and deliberately
    /// the SQUARE of `ratio_margin`. The legacy family used `TOLERANCE2` as a
    /// "much tighter than tau" floor for iteration convergence and
    /// normalization checks on dimensionless quantities (knot values, Newton
    /// parameters). Degree zero means scale-invariant: the tight floor is
    /// correct at every model scale, and this predicate names it instead of
    /// leaving the bare constant at the call site. It is a floor, not a derived
    /// quantity — do not use it for anything that is genuinely a squared
    /// length; that is [`Self::is_small_len2`].
    pub fn is_small_ratio2(&self, x: f64) -> bool {
        x.abs() <= self.ratio_margin() * self.ratio_margin()
    }

    /// BG-TOL-003: an entity's tolerance may never be tighter than its
    /// boundary's. Returns the entity tolerance to use given a boundary
    /// tolerance, which is the larger of the two.
    pub fn entity_tau(&self, boundary_tau: f64) -> f64 {
        self.tau_rep.max(boundary_tau)
    }
}

#[test]
fn is_small_len2_reproduces_tolerance2_at_stage_a() {
    let ctx = ToleranceCtx::unscaled_legacy();
    assert!(ctx.is_small_len2(TOLERANCE2));
    assert!(!ctx.is_small_len2(TOLERANCE2 * 2.0));
}

#[test]
fn is_small_len2_scales_quadratically() {
    let Ok(c) = ToleranceCtx::new(10.0, TOLERANCE, TOLERANCE, TOLERANCE) else {
        unreachable!()
    };
    let ctx = c.value;
    assert!(ctx.is_small_len2(50.0 * TOLERANCE2));
    assert!(!ctx.is_small_len2(200.0 * TOLERANCE2));
}

#[test]
fn is_small_ratio2_is_scale_invariant() {
    let legacy = ToleranceCtx::unscaled_legacy();
    let Ok(c) = ToleranceCtx::new(10.0, TOLERANCE, TOLERANCE, TOLERANCE) else {
        unreachable!()
    };
    let scaled = c.value;
    for ctx in [legacy, scaled] {
        assert!(ctx.is_small_ratio2(TOLERANCE2));
        assert!(!ctx.is_small_ratio2(TOLERANCE2 * 2.0));
    }
}

#[test]
fn length2_margin_is_the_square_of_length_margin() {
    let legacy = ToleranceCtx::unscaled_legacy();
    let Ok(c) = ToleranceCtx::new(10.0, TOLERANCE, TOLERANCE, TOLERANCE) else {
        unreachable!()
    };
    let scaled = c.value;
    for ctx in [legacy, scaled] {
        assert_eq!(
            ctx.length2_margin(),
            ctx.length_margin() * ctx.length_margin()
        );
    }
}
