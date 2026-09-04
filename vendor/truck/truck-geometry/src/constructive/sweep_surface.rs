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

use super::{ConstructError, FrameLaw, ProfileLaw, SpineFrameRecipe};

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

impl SearchParameter<D2> for SpineFrameSweep {
    type Point = Point3;
    fn search_parameter<H: Into<SPHint2D>>(
        &self,
        point: Point3,
        hint: H,
        trials: usize,
    ) -> Option<(f64, f64)> {
        let hint = hint.into();
        let (urange, vrange) = self.range_tuple();
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
        let hint = hint.into();
        let (urange, vrange) = self.range_tuple();
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
