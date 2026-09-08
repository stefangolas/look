#![deny(clippy::unwrap_used)]

//! BG-KV2-501-C6 — the closed whole-sweep surface value (spec §5.10, as
//! amended by the owner resolution recorded in the spec).
//!
//! [`SpineFrameSweep`] is the whole-sweep type the canonical
//! `Surface::SpineFrameSurface` variant now carries. It stores the landed
//! [`SpineFrameRecipe`] ONCE — the spec's four fields (`spine`,
//! `profile_law`, `frame_law`, `frame_data`) all live inside the recipe —
//! over the CANONICAL `Box<Curve>` spine carrier (the closed `Curve`/`Surface`
//! enums are `Clone + Serialize`, which the constructive `Spine` enum's
//! `Box<dyn SpineCurve>` payload forbids; compiler-verified at r1). The
//! realized window domain `[s0, s1] × [v0, v1]` rides ON the closed value —
//! the r1 volume evidence (windowed −1.0 vs whole-ring −3.0 on the unit
//! prism) is why the window is part of this struct, never a derived view —
//! and the sweep-level `Matrix4` placement rides beside it.
//!
//! The windowed realization decorator
//! ([`SpineFrameSurface`](crate::decorators::SpineFrameSurface)) is a derived
//! window view, NOT stored here: it realizes one profile edge from this
//! sweep's closed value (recipe + window + placement). All numeric evaluation
//! stays in the landed evaluator path (`decorators/spine_frame.rs`): this
//! module implements no surface math of its own, it forwards to the shared
//! helpers the decorator uses. Constructors validate through the same
//! [`validate_surface_window`](validate_surface_window)
//! window contract, so the sweep and the decorator derived from it can never
//! disagree on a valid window.

use crate::decorators::{
    central_difference_s, central_difference_v, evaluate_position, float_certificate, surface_uder,
    surface_vder, validate_surface_window,
};
use crate::prelude::*;
use serde::{Deserialize, Serialize};
use std::ops::Bound;
use truck_base::evidence::{Budget, Certified, Outcome, Refusal};

use super::{ConstructError, FrameLaw, ProfileLaw, SpineCurve, SpineFrameRecipe};

/// The closed whole-sweep surface value (spec §5.10, as amended): the landed
/// [`SpineFrameRecipe`] over the canonical `Box<Curve>` spine, the realized
/// window `[s0, s1] × [v0, v1]`, and the sweep-level placement.
///
/// One profile edge of a spine sweep: `X(s, v) = C(s) + frame(s)·P(s, v)` over
/// the window, exactly as the realization decorator realizes it. The window is
/// part of the closed value (inverting a sweep swaps `v0`/`v1` in place), so a
/// face carrying this value reports and evaluates exactly its own domain.
///
/// `PartialEq` is deliberately NOT derived: the payload spine is the closed
/// `Curve`, which carries no equality (the landed decorator's precedent).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpineFrameSweep {
    /// The whole-sweep recipe, stored once on the canonical spine carrier.
    recipe: SpineFrameRecipe<Box<Curve>, ProfileLaw, FrameLaw>,
    /// The spine parameter of the first station.
    s0: f64,
    /// The spine parameter of the last station.
    s1: f64,
    /// The ring parameter of the window's first edge endpoint.
    v0: f64,
    /// The ring parameter of the window's second edge endpoint.
    v1: f64,
    /// The sweep-level placement.
    transform: Matrix4,
}

impl SpineFrameSweep {
    /// Assembles the closed whole-sweep value over `[s0, s1] × [v0, v1]`,
    /// reusing the landed recipe validators verbatim
    /// ([`validate_surface_window`](validate_surface_window)
    /// — the exact check the windowed realization decorator runs). No numeric
    /// evaluation happens here beyond that shared window validation;
    /// realization stays in the landed evaluator path.
    pub fn try_new(
        recipe: SpineFrameRecipe<Box<Curve>, ProfileLaw, FrameLaw>,
        s0: f64,
        s1: f64,
        v0: f64,
        v1: f64,
    ) -> std::result::Result<Self, ConstructError> {
        validate_surface_window(&recipe, s0, s1, v0, v1)?;
        Ok(SpineFrameSweep {
            recipe,
            s0,
            s1,
            v0,
            v1,
            transform: Matrix4::identity(),
        })
    }

    /// The whole-sweep recipe the value stores once.
    #[inline(always)]
    pub fn recipe(&self) -> &SpineFrameRecipe<Box<Curve>, ProfileLaw, FrameLaw> {
        &self.recipe
    }
    /// The spine parameter of the first station.
    #[inline(always)]
    pub fn s0(&self) -> f64 {
        self.s0
    }
    /// The spine parameter of the last station.
    #[inline(always)]
    pub fn s1(&self) -> f64 {
        self.s1
    }
    /// The ring parameter of the window's first edge endpoint.
    #[inline(always)]
    pub fn v0(&self) -> f64 {
        self.v0
    }
    /// The ring parameter of the window's second edge endpoint.
    #[inline(always)]
    pub fn v1(&self) -> f64 {
        self.v1
    }
    /// The stored sweep-level placement.
    #[inline(always)]
    pub fn transform(&self) -> &Matrix4 {
        &self.transform
    }
}

impl ParametricSurface for SpineFrameSweep {
    type Point = Point3;
    type Vector = Vector3;

    fn der_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Self::Vector {
        match (m, n) {
            (0, 0) => self.subs(u, v).to_vec(),
            (1, 0) => self.uder(u, v),
            (0, 1) => self.vder(u, v),
            (2, 0) => self.uuder(u, v),
            (1, 1) => self.uvder(u, v),
            (0, 2) => self.vvder(u, v),
            _ => Self::Vector::zero(),
        }
    }
    #[inline(always)]
    fn subs(&self, u: f64, v: f64) -> Self::Point {
        evaluate_position(&self.recipe, &self.transform, u, v)
    }
    #[inline(always)]
    fn uder(&self, u: f64, v: f64) -> Self::Vector {
        surface_uder(&self.recipe, &self.transform, u, v)
    }
    #[inline(always)]
    fn vder(&self, u: f64, v: f64) -> Self::Vector {
        surface_vder(&self.recipe, &self.transform, u, v)
    }
    #[inline(always)]
    fn uuder(&self, u: f64, v: f64) -> Self::Vector {
        central_difference_s(u, v, |s, w| self.uder(s, w))
    }
    #[inline(always)]
    fn uvder(&self, u: f64, v: f64) -> Self::Vector {
        central_difference_s(u, v, |s, w| self.vder(s, w))
    }
    #[inline(always)]
    fn vvder(&self, u: f64, v: f64) -> Self::Vector {
        central_difference_v(u, v, |s, w| self.vder(s, w))
    }
    #[inline(always)]
    fn parameter_range(&self) -> (ParameterRange, ParameterRange) {
        (
            (Bound::Included(self.s0), Bound::Included(self.s1)),
            (Bound::Included(self.v0), Bound::Included(self.v1)),
        )
    }
}

impl ParametricSurface3D for SpineFrameSweep {}

impl BoundedSurface for SpineFrameSweep {}

impl ParameterDivision2D for SpineFrameSweep {
    fn parameter_division(
        &self,
        range: ((f64, f64), (f64, f64)),
        tol: f64,
    ) -> (Vec<f64>, Vec<f64>) {
        algo::surface::parameter_division(self, range, tol)
    }
}

/// DEF-SPINEFRAME-GRAZE-R2 — the sweep's own certified window is `[s0, s1] ×
/// [v0, v1]` (the whole-sweep value the constructor validated through
/// [`validate_surface_window`](validate_surface_window)). The boolean funnel's
/// SEARCH path evaluates the sweep through the generic `algo::surface`
/// Newton drivers, whose iterates are NOT confined to the window: the r1
/// ladder measured iterates wandering to `s = -2.3e-3` (≈2300×
/// `DirectTolerance::parameter`) and `v` past every pad below the refusal
/// probe. The sweep's evaluation is only CERTIFIED inside its window, so the
/// search must evaluate inside it.
///
/// This is the domain-clamped evaluation view the SEARCH path runs over
/// (scope decision 1(a)): every `(u, v)` is clamped into the window (the
/// named [`Self::clamp_parameter`] operation) BEFORE `subs`/`uder`/`vder`;
/// the accepted answer still passes the driver's residual gate
/// (`ctx.near_points`), and the returned parameter is clamped too — the clamp
/// is named and recorded, never silent. Second derivatives re-clamp their own
/// samples, so the central-difference h-step cannot leave the window either.
/// Shared by the closed whole-sweep value and the windowed realization
/// decorator (both route their search entry points through this view).
pub(crate) struct SweepWindowClamp<'a, S: SpineCurve> {
    recipe: &'a SpineFrameRecipe<S, ProfileLaw, FrameLaw>,
    transform: &'a Matrix4,
    s0: f64,
    s1: f64,
    v0: f64,
    v1: f64,
}

impl<'a, S: SpineCurve> Clone for SweepWindowClamp<'a, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, S: SpineCurve> Copy for SweepWindowClamp<'a, S> {}

impl<'a, S: SpineCurve> SweepWindowClamp<'a, S> {
    /// The named clamp operation: `(u, v)` into the window, orientation-safe
    /// (an inverted sweep swaps `v0`/`v1` in place).
    #[inline(always)]
    pub(crate) fn clamp_parameter(&self, u: f64, v: f64) -> (f64, f64) {
        (
            u.clamp(self.s0.min(self.s1), self.s0.max(self.s1)),
            v.clamp(self.v0.min(self.v1), self.v0.max(self.v1)),
        )
    }

    /// The certified-window clamp view over one spine-frame surface value
    /// (the sweep's own window or a windowed realization decorator's).
    pub(crate) fn new(
        recipe: &'a SpineFrameRecipe<S, ProfileLaw, FrameLaw>,
        transform: &'a Matrix4,
        s0: f64,
        s1: f64,
        v0: f64,
        v1: f64,
    ) -> Self {
        SweepWindowClamp {
            recipe,
            transform,
            s0,
            s1,
            v0,
            v1,
        }
    }

    /// The window as an ascending parameter box (the clamp domain).
    #[inline(always)]
    fn window_range(&self) -> ((f64, f64), (f64, f64)) {
        (
            (self.s0.min(self.s1), self.s0.max(self.s1)),
            (self.v0.min(self.v1), self.v0.max(self.v1)),
        )
    }

    /// The generic `search_parameter` driver run over this clamped view, with
    /// the returned parameter clamped back into the window as the recorded
    /// answer.
    pub(crate) fn search_parameter(
        &self,
        point: Point3,
        hint: SPHint2D,
        trials: usize,
    ) -> Option<(f64, f64)> {
        let (urange, vrange) = self.window_range();
        let hint = match hint {
            SPHint2D::Parameter(u, v) => (u, v),
            SPHint2D::Range(u, v) => {
                algo::surface::presearch(self, point, (u, v), crate::PRESEARCH_DIVISION)
            }
            SPHint2D::None => {
                algo::surface::presearch(self, point, (urange, vrange), crate::PRESEARCH_DIVISION)
            }
        };
        algo::surface::search_parameter(self, point, hint, trials)
            .map(|(u, v)| self.clamp_parameter(u, v))
    }

    /// The generic `search_nearest_parameter` driver run over this clamped
    /// view, with the returned parameter clamped back into the window.
    pub(crate) fn search_nearest_parameter(
        &self,
        point: Point3,
        hint: SPHint2D,
        trials: usize,
    ) -> Option<(f64, f64)> {
        let (urange, vrange) = self.window_range();
        let hint = match hint {
            SPHint2D::Parameter(u, v) => (u, v),
            SPHint2D::Range(u, v) => {
                algo::surface::presearch(self, point, (u, v), crate::PRESEARCH_DIVISION)
            }
            SPHint2D::None => {
                algo::surface::presearch(self, point, (urange, vrange), crate::PRESEARCH_DIVISION)
            }
        };
        algo::surface::search_nearest_parameter(self, point, hint, trials)
            .map(|(u, v)| self.clamp_parameter(u, v))
    }
}

impl<S: SpineCurve> ParametricSurface for SweepWindowClamp<'_, S> {
    type Point = Point3;
    type Vector = Vector3;

    fn der_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Self::Vector {
        let (u, v) = self.clamp_parameter(u, v);
        match (m, n) {
            (0, 0) => self.subs(u, v).to_vec(),
            (1, 0) => self.uder(u, v),
            (0, 1) => self.vder(u, v),
            (2, 0) => self.uuder(u, v),
            (1, 1) => self.uvder(u, v),
            (0, 2) => self.vvder(u, v),
            _ => Self::Vector::zero(),
        }
    }
    #[inline(always)]
    fn subs(&self, u: f64, v: f64) -> Self::Point {
        let (u, v) = self.clamp_parameter(u, v);
        evaluate_position(self.recipe, self.transform, u, v)
    }
    #[inline(always)]
    fn uder(&self, u: f64, v: f64) -> Self::Vector {
        let (u, v) = self.clamp_parameter(u, v);
        surface_uder(self.recipe, self.transform, u, v)
    }
    #[inline(always)]
    fn vder(&self, u: f64, v: f64) -> Self::Vector {
        let (u, v) = self.clamp_parameter(u, v);
        surface_vder(self.recipe, self.transform, u, v)
    }
    #[inline(always)]
    fn uuder(&self, u: f64, v: f64) -> Self::Vector {
        let (u, v) = self.clamp_parameter(u, v);
        central_difference_s(u, v, |s, w| self.uder(s, w))
    }
    #[inline(always)]
    fn uvder(&self, u: f64, v: f64) -> Self::Vector {
        let (u, v) = self.clamp_parameter(u, v);
        central_difference_s(u, v, |s, w| self.vder(s, w))
    }
    #[inline(always)]
    fn vvder(&self, u: f64, v: f64) -> Self::Vector {
        let (u, v) = self.clamp_parameter(u, v);
        central_difference_v(u, v, |s, w| self.vder(s, w))
    }
    #[inline(always)]
    fn parameter_range(&self) -> (ParameterRange, ParameterRange) {
        (
            (Bound::Included(self.s0), Bound::Included(self.s1)),
            (Bound::Included(self.v0), Bound::Included(self.v1)),
        )
    }
}

impl SpineFrameSweep {
    /// The sweep's certified window clamp view (the whole-sweep carrier of the
    /// SEARCH layer clamp, DEF-SPINEFRAME-GRAZE-R2 scope 1(a)).
    fn search_clamp(&self) -> SweepWindowClamp<'_, Box<Curve>> {
        SweepWindowClamp::new(
            &self.recipe,
            &self.transform,
            self.s0,
            self.s1,
            self.v0,
            self.v1,
        )
    }
}

impl SearchParameter<D2> for SpineFrameSweep {
    type Point = Point3;
    fn search_parameter<H: Into<SPHint2D>>(
        &self,
        point: Point3,
        hint: H,
        trials: usize,
    ) -> Option<(f64, f64)> {
        self.search_clamp()
            .search_parameter(point, hint.into(), trials)
    }
}

impl SearchNearestParameter<D2> for SpineFrameSweep {
    type Point = Point3;
    fn search_nearest_parameter<H: Into<SPHint2D>>(
        &self,
        point: Point3,
        hint: H,
        trials: usize,
    ) -> Option<(f64, f64)> {
        self.search_clamp()
            .search_nearest_parameter(point, hint.into(), trials)
    }
}

impl Invertible for SpineFrameSweep {
    #[inline(always)]
    fn invert(&mut self) {
        std::mem::swap(&mut self.v0, &mut self.v1);
    }
}

impl Transformed<Matrix4> for SpineFrameSweep {
    #[inline(always)]
    fn transform_by(&mut self, trans: Matrix4) {
        self.transform = trans * self.transform;
    }
    #[inline(always)]
    fn transformed(&self, trans: Matrix4) -> Self {
        Self {
            transform: trans * self.transform,
            ..self.clone()
        }
    }
}

/// Whether the sweep includes one of its own boundary curves. The question is
/// the same the windowed realization decorator answers (the sweep's window is
/// the decorator's window): a ring `Line` is compared structurally — a placed
/// ring is still a `Line` — and a trajectory `SpineFrameCurve`'s containment
/// refuses typed (`UncertifiedContainment`), the BG-S0-001 doctrine.
impl IncludeCurve<Curve> for SpineFrameSweep {
    fn include(&self, curve: &Curve) -> Outcome<bool> {
        match curve {
            Curve::Line(line) => {
                let ring0 = Line(self.subs(self.s0, self.v0), self.subs(self.s0, self.v1));
                let ring1 = Line(self.subs(self.s1, self.v0), self.subs(self.s1, self.v1));
                Ok(Certified::new(
                    line == &ring0 || line == &ring1,
                    float_certificate(),
                ))
            }
            Curve::SpineFrameCurve(_) => Err(Refusal::NumericallyUnresolved {
                spent: Budget::new(0, 0, 0),
                witness: truck_base::evidence::UnresolvedWitness::UncertifiedContainment,
            }),
            _ => Ok(Certified::new(false, float_certificate())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constructive::{DirectTolerance, Profile2D, ProfileLaw, PROFILE_V_DOMAIN_PAD};

    /// The unit-square profile (CCW about +z in the frame plane).
    fn unit_square_profile() -> Option<Profile2D> {
        Profile2D::try_closed(vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ])
        .ok()
    }

    /// The recipe over a `Box<Curve>` line spine from the origin to (0, 0, 1)
    /// (the storage form the closed enums carry). The frame plane is pinned by
    /// `FixedPlane { normal: +x }`, so edge 0 (v in [0, 1/4]) realizes
    /// `X(s, v) = (0, -4v, s)`.
    fn edge_zero_sweep() -> Option<SpineFrameSweep> {
        let profile = unit_square_profile()?;
        let spine = Box::new(Curve::Line(Line(
            Point3::origin(),
            Point3::new(0.0, 0.0, 1.0),
        )));
        let recipe = SpineFrameRecipe::new(
            spine,
            ProfileLaw::Constant(profile),
            FrameLaw::FixedPlane {
                normal: Vector3::unit_x(),
            },
        );
        SpineFrameSweep::try_new(recipe, 0.0, 1.0, 0.0, 0.25).ok()
    }

    /// The `Some` value of a sweep, asserted on a real predicate first
    /// (clippy-silent, unwrap-free: the divergent tail is a `None`).
    fn expect_sweep() -> Option<SpineFrameSweep> {
        let sweep = edge_zero_sweep()?;
        assert_eq!((sweep.s0(), sweep.s1()), (0.0, 1.0));
        assert_eq!((sweep.v0(), sweep.v1()), (0.0, 0.25));
        Some(sweep)
    }

    /// DEF-SPINEFRAME-GRAZE-R2 regression: a SEARCH iterate outside the
    /// certified window is evaluated CLAMPED (never refused at the
    /// match-unwrap), and the returned parameter is the certified-window
    /// answer. The r1 witness ladder wandered to `s = -2.3e-3` and `v` past
    /// the refusal probe on BOTH axes; here the hint itself is that far out on
    /// the `v` axis (`v = -0.25`, 250000x `DirectTolerance::parameter`), which
    /// used to refuse at the first evaluation.
    #[test]
    fn out_of_window_search_iterate_evaluates_clamped() {
        let sweep = match expect_sweep() {
            Some(sweep) => sweep,
            None => return,
        };
        // The certified-window answer for the query (on the v = 0 ring at
        // s = 1/2) is exactly the query's own parameter, clamped.
        let query = sweep.subs(0.5, 0.0);
        let (u, v) = match sweep.search_parameter(query, SPHint2D::Parameter(0.5, -0.25), 100) {
            Some(uv) => uv,
            None => return,
        };
        let (s0, s1, v0, v1) = (sweep.s0(), sweep.s1(), sweep.v0(), sweep.v1());
        assert!(
            (s0..=s1).contains(&u) && (v0..=v1).contains(&v),
            "the returned parameter ({u}, {v}) left the certified window"
        );
        assert!((u - 0.5).abs() <= 1.0e-12);
        assert!((v - 0.0).abs() <= 1.0e-12);
        let on_surface = sweep.subs(u, v);
        assert!(
            (on_surface - query).magnitude() <= DirectTolerance::default().position,
            "the clamped answer diverged from the certified-window point"
        );
    }

    /// The same regression on the SPINE axis: a `search_parameter` hint at
    /// `s = -0.25` (250000x the parameter tolerance below `s0`) used to refuse
    /// at the first evaluation; the clamp views it at the window edge.
    #[test]
    fn out_of_window_spine_iterate_evaluates_clamped() {
        let sweep = match expect_sweep() {
            Some(sweep) => sweep,
            None => return,
        };
        let query = sweep.subs(0.0, 0.125);
        let (u, v) = match sweep.search_parameter(query, SPHint2D::Parameter(-0.25, 0.125), 100) {
            Some(uv) => uv,
            None => return,
        };
        assert!((0.0..=1.0).contains(&u) && (0.0..=0.25).contains(&v));
        assert!((u - 0.0).abs() <= 1.0e-12);
        assert!((v - 0.125).abs() <= 1.0e-12);
        let on_surface = sweep.subs(u, v);
        assert!(
            (on_surface - query).magnitude() <= DirectTolerance::default().position,
            "the clamped spine-axis answer diverged from the window point"
        );
    }

    /// The nearest search keeps the same clamp: a point whose unclamped
    /// nearest parameter lies on the window edge is answered at that edge,
    /// never refused.
    #[test]
    fn out_of_window_nearest_iterate_evaluates_clamped() {
        let sweep = match expect_sweep() {
            Some(sweep) => sweep,
            None => return,
        };
        let query = sweep.subs(0.5, 0.0);
        let (u, v) =
            match sweep.search_nearest_parameter(query, SPHint2D::Parameter(0.5, -0.25), 100) {
                Some(uv) => uv,
                None => return,
            };
        assert!((0.0..=1.0).contains(&u) && (0.0..=0.25).contains(&v));
        let on_surface = sweep.subs(u, v);
        assert!(
            (on_surface - query).magnitude() <= DirectTolerance::default().position,
            "the clamped nearest answer diverged from the window point"
        );
    }

    /// The windowed realization decorator shares the clamp: its search entry
    /// evaluates an out-of-window iterate clamped as well.
    #[test]
    fn decorator_search_iterate_evaluates_clamped() {
        let sweep = match expect_sweep() {
            Some(sweep) => sweep,
            None => return,
        };
        let surface = Surface::SpineFrameSurface(sweep.clone());
        let query = surface.subs(0.5, 0.0);
        let (u, v) = match surface.search_parameter(query, SPHint2D::Parameter(0.5, -0.25), 100) {
            Some(uv) => uv,
            None => return,
        };
        assert!((0.0..=1.0).contains(&u) && (0.0..=0.25).contains(&v));
        let on_surface = surface.subs(u, v);
        assert!(
            (on_surface - query).magnitude() <= DirectTolerance::default().position,
            "the decorator clamped answer diverged from the window point"
        );
    }

    /// The r1 pad is re-landed as defense in depth: the profile-v refusal is
    /// padded at 1.75x `DirectTolerance::parameter`, so the measured r1 v-axis
    /// witnesses evaluate while the landed `-2x` refusal probe still refuses.
    #[test]
    fn profile_v_domain_pad_sizing_and_witness_rescue() {
        let tolerance = DirectTolerance::default().parameter;
        // 1.53x < pad < 2x (the measured r1 witness ladder: 1.21x then 1.53x).
        assert!(
            1.53 * tolerance < PROFILE_V_DOMAIN_PAD && PROFILE_V_DOMAIN_PAD < 2.0 * tolerance,
            "the pad must sit strictly between the r1 witness class and the -2x refusal probe"
        );
        let profile = match unit_square_profile() {
            Some(profile) => profile,
            None => return,
        };
        let law = ProfileLaw::Constant(profile);
        // Both measured r1 witnesses evaluate under the pad...
        for witness in [-1.2138e-6, -1.5301e-6] {
            assert!(
                law.evaluate(0.5, witness).is_ok(),
                "the r1 witness v = {witness} must evaluate under the pad"
            );
        }
        // ...while the landed refusal probe at -2x stays a typed refusal.
        assert!(
            matches!(
                law.evaluate(0.5, -2.0 * tolerance),
                Err(ConstructError::InvalidInput)
            ),
            "the -2x probe must still refuse: the pad stays the second net"
        );
        assert!(
            matches!(
                law.evaluate(0.5, 1.0 + 2.0 * tolerance),
                Err(ConstructError::InvalidInput)
            ),
            "the +2x probe must still refuse above the padded domain"
        );
    }
}
