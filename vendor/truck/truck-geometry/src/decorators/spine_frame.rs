#![deny(clippy::unwrap_used)]

//! BG-CG-009-BREP — the parametric spine/profile realization.
//!
//! [`SpineFrameSurface`] realizes one side face of a spine sweep,
//! `X(s, v) = C(s) + frame(s) · P(s, v)`, over the profile-edge window
//! `[s0, s1] × [v0, v1]`; [`SpineFrameCurve`] realizes the trajectory of one
//! fixed profile point, `E(s) = X(s, v_p)`, shared by the two adjacent side
//! faces by identity (never re-derived). Both hold a `SpineFrameRecipe`
//! fragment — the landed recipe evaluators — and a stored `Matrix4`
//! placement. Constructors validate to `DirectTolerance::position` and refuse
//! via `ConstructError`; the `Outcome` mapping happens at modeling's entry
//! (the CG-007 pattern).
//!
//! The canonical `Curve`/`Surface` enums store these decorators at `S = Curve`
//! (`Curve: SpineCurve` below), so a recipe over any landed `SpineCurve` can be stored
//! by converting the spine to its canonical `Curve` carrier.

use super::*;
use crate::constructive::{
    ConstructError, DirectTolerance, Frame3, FrameData, FrameLaw, LineSpine, PolylineSpine,
    Profile2D, ProfileLaw, ScalarLaw, SpineCurve, SpineFrameRecipe,
};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use truck_base::evidence::{
    Budget, Certificate, Certified, Margin, Method, Modulus, Outcome, PropMap, Refusal,
};

/// The profile ring parameter of profile vertex `j` out of `k`: `v = j / k`.
/// The per-edge-uniform convention the landed `profile.rs` evaluator is booked
/// on (vertex `j` sits at `v = j / k`).
#[inline(always)]
pub(crate) fn ring_parameter(j: usize, k: usize) -> f64 {
    j as f64 / k as f64
}

/// The canonical `Curve` is itself a landed spine: its `subs`/`der` are the
/// position and tangent, its parameter range is the domain. This is what lets
/// the closed `Curve`/`Surface` enums carry the decorators at `S = Curve`
/// (the ExtrudedCurve precedent: the enum instantiates the decorator over the
/// canonical closed type).
impl SpineCurve for Curve {
    fn domain(&self) -> (f64, f64) {
        self.range_tuple()
    }
    fn position_at(&self, s: f64) -> std::result::Result<Point3, ConstructError> {
        if !s.is_finite() {
            return Err(ConstructError::NonFinite { at: s });
        }
        let (s0, s1) = self.range_tuple();
        let tolerance = DirectTolerance::default().parameter;
        if s < s0 - tolerance || s > s1 + tolerance {
            return Err(ConstructError::InvalidInput);
        }
        Ok(self.subs(s))
    }
    fn derivative_at(&self, s: f64) -> std::result::Result<Vector3, ConstructError> {
        if !s.is_finite() {
            return Err(ConstructError::NonFinite { at: s });
        }
        let (s0, s1) = self.range_tuple();
        let tolerance = DirectTolerance::default().parameter;
        if s < s0 - tolerance || s > s1 + tolerance {
            return Err(ConstructError::InvalidInput);
        }
        Ok(self.der(s))
    }
}

/// A boxed spine is a spine: the canonical enums store the decorators at
/// `S = Box<Curve>` (the indirection that breaks the closed-enum recursion,
/// the `IntersectionCurve` precedent).
impl<S: SpineCurve + Clone> SpineCurve for Box<S> {
    fn domain(&self) -> (f64, f64) {
        (**self).domain()
    }
    fn position_at(&self, s: f64) -> std::result::Result<Point3, ConstructError> {
        (**self).position_at(s)
    }
    fn derivative_at(&self, s: f64) -> std::result::Result<Vector3, ConstructError> {
        (**self).derivative_at(s)
    }
}

/// A straight-segment spine is the canonical line carrier.
impl From<LineSpine> for Curve {
    #[inline(always)]
    fn from(spine: LineSpine) -> Curve {
        Curve::Line(Line(spine.start, spine.end))
    }
}

/// A polyline spine becomes the exact degree-1 B-spline through its vertices
/// (the polyline IS that spline). The C1 gate is not re-derived here: the
/// landed `PolylineSpine::derivative_at` refusal fires at evaluation time on
/// the original spine (the frame laws consume it), before any storage spine is
/// converted.
impl From<PolylineSpine> for Curve {
    fn from(spine: PolylineSpine) -> Curve {
        let n = spine.vertices.len();
        let knot_vec = KnotVec::uniform_knot(1, n - 1);
        Curve::BSplineCurve(BSplineCurve::new(knot_vec, spine.vertices))
    }
}

/// The number of profile ring vertices the law produces.
#[inline(always)]
fn profile_vertex_count(profile_law: &ProfileLaw) -> usize {
    match profile_law {
        ProfileLaw::Constant(profile) => profile.vertices.len(),
        ProfileLaw::Scale { profile, .. } => profile.vertices.len(),
        ProfileLaw::LinearCorrespondence { start, .. } => start.vertices.len(),
    }
}

/// The two endpoints of profile ring edge `e` under the law at station `s`.
/// Edge `e` runs vertex `e` → vertex `(e + 1) % k`; under `Scale` the pair is
/// scaled (a through-zero scale refuses `ProfileCollapse`, matching the landed
/// evaluator), under `LinearCorrespondence` the pair is the vertex-wise lerp.
fn profile_edge_vertices(
    profile_law: &ProfileLaw,
    s: f64,
    e: usize,
) -> std::result::Result<(Point2, Point2), ConstructError> {
    let k = profile_vertex_count(profile_law);
    if k == 0 {
        return Err(ConstructError::InvalidInput);
    }
    let e1 = (e + 1) % k;
    match profile_law {
        ProfileLaw::Constant(profile) => Ok((profile.vertices[e], profile.vertices[e1])),
        ProfileLaw::Scale { profile, scale } => {
            let c = scale.at(s);
            if c.abs() <= DirectTolerance::default().parameter {
                return Err(ConstructError::ProfileCollapse { at: s });
            }
            Ok((profile.vertices[e] * c, profile.vertices[e1] * c))
        }
        ProfileLaw::LinearCorrespondence { start, end } => {
            let a = start.vertices[e] + (end.vertices[e] - start.vertices[e]) * s;
            let b = start.vertices[e1] + (end.vertices[e1] - start.vertices[e1]) * s;
            Ok((a, b))
        }
    }
}

/// `∂P/∂v` at `(s, v)`: the profile law is LINEAR in `v` along an edge (the
/// landed `profile.rs` law), so the derivative is the edge direction times
/// `k` (the `v`-parameter spans `1/k` per edge).
fn profile_derivative_v(
    profile_law: &ProfileLaw,
    s: f64,
    v: f64,
) -> std::result::Result<Vector2, ConstructError> {
    if !(0.0..=1.0).contains(&v) {
        return Err(ConstructError::InvalidInput);
    }
    let k = profile_vertex_count(profile_law);
    if k == 0 {
        return Err(ConstructError::InvalidInput);
    }
    let e = ((v * k as f64).floor() as usize).min(k - 1);
    let (a, b) = profile_edge_vertices(profile_law, s, e)?;
    Ok((b - a) * k as f64)
}

/// `∂P/∂s` at `(s, v)`: the profile law's explicit `s`-derivative (the
/// `Constant` law has none; `Scale` differentiates the scalar law;
/// `LinearCorrespondence` differentiates the vertex-wise lerp).
fn profile_derivative_s(
    profile_law: &ProfileLaw,
    _s: f64,
    v: f64,
) -> std::result::Result<Vector2, ConstructError> {
    if !(0.0..=1.0).contains(&v) {
        return Err(ConstructError::InvalidInput);
    }
    let k = profile_vertex_count(profile_law);
    if k == 0 {
        return Err(ConstructError::InvalidInput);
    }
    let e = ((v * k as f64).floor() as usize).min(k - 1);
    let e1 = (e + 1) % k;
    let f = v * k as f64 - e as f64;
    match profile_law {
        ProfileLaw::Constant(_) => Ok(Vector2::zero()),
        ProfileLaw::Scale { profile, scale } => {
            let dc = match *scale {
                ScalarLaw::Constant(_) => 0.0,
                ScalarLaw::Linear { start, end } => end - start,
            };
            let a = profile.vertices[e];
            let b = profile.vertices[e1];
            Ok((a + (b - a) * f).to_vec() * dc)
        }
        ProfileLaw::LinearCorrespondence { start, end } => {
            let a = end.vertices[e] - start.vertices[e];
            let b = end.vertices[e1] - start.vertices[e1];
            Ok(a + (b - a) * f)
        }
    }
}

/// The float-method certificate for the structural `include` predicates
/// (H-6: concrete float arithmetic certifies `Float`, never `Exact`). Shared
/// by the realization decorator and the closed whole-sweep value
/// (`constructive::SpineFrameSweep`), whose include predicate is the same
/// boundary-line comparison.
#[inline(always)]
pub(crate) fn float_certificate() -> Certificate {
    Certificate {
        props: PropMap::new(),
        method: Method::Float,
        budget_left: Budget::new(0, 0, 0),
        margin: Margin::UNBOUNDED,
        modulus: Modulus::Unbounded,
    }
}

/// The parametric realization of ONE profile edge of a spine sweep (build-spec
/// §8B): `X(s, v) = C(s) + frame(s) · P(s, v)` over the edge window
/// `[s0, s1] × [v0, v1]`. One side face of a spine sweep rides on this
/// surface; the `v`-axis runs along a single straight profile edge.
///
/// A stored `Matrix4` placement composes every evaluation (the
/// `Transformed`-by-storage pattern); a singular placement makes the certified
/// inverse-requiring operations refuse typed rather than approximate.
///
/// `PartialEq` is deliberately NOT derived: the payload spine is the closed
/// `Curve`, which carries no equality.
#[derive(Clone, Debug)]
pub struct SpineFrameSurface<S> {
    recipe: SpineFrameRecipe<S, ProfileLaw, FrameLaw>,
    s0: f64,
    s1: f64,
    v0: f64,
    v1: f64,
    transform: Matrix4,
}

/// The trajectory of one fixed profile point under the recipe (build-spec
/// §8B): `E(s) = X(s, v_p)`, `v_p` the ring parameter of the profile vertex.
/// Shared by the two adjacent side faces BY IDENTITY — one `Edge` handle
/// cloned, never re-derived.
///
/// `PartialEq` is deliberately NOT derived (see [`SpineFrameSurface`]).
#[derive(Clone, Debug)]
pub struct SpineFrameCurve<S> {
    recipe: SpineFrameRecipe<S, ProfileLaw, FrameLaw>,
    s0: f64,
    s1: f64,
    v_p: f64,
    transform: Matrix4,
}

/// A spine parameter within the recipe's domain, within the parameter
/// tolerance.
fn validate_spine_parameter(
    spine: &dyn SpineCurve,
    s: f64,
) -> std::result::Result<(), ConstructError> {
    if !s.is_finite() {
        return Err(ConstructError::NonFinite { at: s });
    }
    let (s_min, s_max) = spine.domain();
    let tolerance = DirectTolerance::default().parameter;
    if s < s_min - tolerance || s > s_max + tolerance {
        return Err(ConstructError::InvalidInput);
    }
    Ok(())
}

/// The SHARED surface-window validation (BG-KV2-501-C6): the window contract a
/// realized surface over one profile edge must satisfy. Both the windowed
/// realization decorator [`SpineFrameSurface::try_new`] and the closed
/// whole-sweep value (`constructive::SpineFrameSweep::try_new`) run this same
/// check, so a sweep stored on the closed `Surface::SpineFrameSurface` variant
/// and the per-face decorator derived from it can never disagree on a valid
/// window. Validation: both spine parameters inside the recipe's spine domain,
/// the window ascending, both `v` parameters inside `[0, 1]` and within
/// `DirectTolerance::parameter` of a profile-edge boundary (`j/k`), and every
/// corner evaluation succeeding (the frame/profile gates fire here).
pub(crate) fn validate_surface_window<S: SpineCurve>(
    recipe: &SpineFrameRecipe<S, ProfileLaw, FrameLaw>,
    s0: f64,
    s1: f64,
    v0: f64,
    v1: f64,
) -> std::result::Result<(), ConstructError> {
    validate_spine_parameter(&recipe.spine, s0)?;
    validate_spine_parameter(&recipe.spine, s1)?;
    if s1 <= s0 {
        return Err(ConstructError::InvalidInput);
    }
    if !(0.0..=1.0).contains(&v0) || !(0.0..=1.0).contains(&v1) || v1 <= v0 {
        return Err(ConstructError::InvalidInput);
    }
    let k = profile_vertex_count(&recipe.profile_law);
    if k < 3 {
        return Err(ConstructError::InvalidInput);
    }
    let tolerance = DirectTolerance::default().parameter;
    let edge = (v0 * k as f64).floor() as usize;
    if edge >= k
        || ((v0 - ring_parameter(edge, k)).abs() > tolerance)
        || ((v1 - ring_parameter(edge + 1, k)).abs() > tolerance)
    {
        return Err(ConstructError::InvalidInput);
    }
    for &s in &[s0, s1] {
        for &v in &[v0, v1] {
            recipe.position(s, v)?;
        }
    }
    Ok(())
}

impl<S: SpineCurve + Clone> SpineFrameSurface<S> {
    /// Assembles the surface over `[s0, s1] × [v0, v1]` after validating:
    /// both spine parameters inside the recipe's spine domain, the window
    /// ascending, both `v` parameters inside `[0, 1]` and within
    /// `DirectTolerance::parameter` of a profile-edge boundary (`j/k`), and
    /// every corner evaluation succeeding (the frame/profile gates fire here).
    /// The validation is [`validate_surface_window`], the SAME window check
    /// the closed whole-sweep value (`constructive::SpineFrameSweep`) runs —
    /// this decorator is a windowed realization view derived from that sweep's
    /// closed value, never an independent authority on what a window means.
    pub fn try_new(
        recipe: SpineFrameRecipe<S, ProfileLaw, FrameLaw>,
        s0: f64,
        s1: f64,
        v0: f64,
        v1: f64,
    ) -> std::result::Result<Self, ConstructError> {
        validate_surface_window(&recipe, s0, s1, v0, v1)?;
        Ok(SpineFrameSurface {
            recipe,
            s0,
            s1,
            v0,
            v1,
            transform: Matrix4::identity(),
        })
    }

    /// The recipe fragment the surface realizes.
    #[inline(always)]
    pub fn recipe(&self) -> &SpineFrameRecipe<S, ProfileLaw, FrameLaw> {
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
    /// The ring parameter of the edge's first vertex.
    #[inline(always)]
    pub fn v0(&self) -> f64 {
        self.v0
    }
    /// The ring parameter of the edge's second vertex.
    #[inline(always)]
    pub fn v1(&self) -> f64 {
        self.v1
    }
    /// The stored placement.
    #[inline(always)]
    pub fn transform(&self) -> &Matrix4 {
        &self.transform
    }
}

impl<S: SpineCurve + Clone> SpineFrameCurve<S> {
    /// Assembles the trajectory of the profile point at ring parameter `v_p`
    /// after validating: both spine parameters inside the recipe's spine
    /// domain, `v_p` inside `[0, 1]`, and both endpoint evaluations succeeding.
    pub fn try_new(
        recipe: SpineFrameRecipe<S, ProfileLaw, FrameLaw>,
        s0: f64,
        s1: f64,
        v_p: f64,
    ) -> std::result::Result<Self, ConstructError> {
        validate_spine_parameter(&recipe.spine, s0)?;
        validate_spine_parameter(&recipe.spine, s1)?;
        if s1 <= s0 {
            return Err(ConstructError::InvalidInput);
        }
        if !(0.0..=1.0).contains(&v_p) {
            return Err(ConstructError::InvalidInput);
        }
        recipe.position(s0, v_p)?;
        recipe.position(s1, v_p)?;
        Ok(SpineFrameCurve {
            recipe,
            s0,
            s1,
            v_p,
            transform: Matrix4::identity(),
        })
    }

    /// The ring parameter of the fixed profile point.
    #[inline(always)]
    pub fn v_p(&self) -> f64 {
        self.v_p
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
    /// The stored placement.
    #[inline(always)]
    pub fn transform(&self) -> &Matrix4 {
        &self.transform
    }
}

/// Evaluates `X(s, v)` under the stored placement. The constructor validated
/// the window, so an evaluation refusal inside it is unreachable; this is the
/// match-based unwrap the house rules sanction (no `.unwrap()` in source).
/// Shared with the closed whole-sweep value's evaluation path.
#[inline(always)]
pub(crate) fn evaluate_position<S: SpineCurve>(
    recipe: &SpineFrameRecipe<S, ProfileLaw, FrameLaw>,
    transform: &Matrix4,
    s: f64,
    v: f64,
) -> Point3 {
    match recipe.position(s, v) {
        Ok(point) => transform.transform_point(point),
        Err(err) => panic!("spine-frame evaluation refused at ({s}, {v}): {err}"),
    }
}

/// The frame at `s`, matched-unwrapped: the constructor validated every corner
/// and the frame laws' singularities refuse deterministically, so a refusal
/// inside the validated window is unreachable.
#[inline(always)]
fn evaluate_frame<S: SpineCurve>(
    recipe: &SpineFrameRecipe<S, ProfileLaw, FrameLaw>,
    s: f64,
) -> Frame3 {
    match recipe.frame(s) {
        Ok(frame) => frame,
        Err(err) => panic!("spine-frame refused frame at {s}: {err}"),
    }
}

/// `S_v = frame(s) · ∂P/∂v`: analytic (the profile law is linear in `v` along
/// an edge — landed `profile.rs`), then placed.
#[inline(always)]
pub(crate) fn surface_vder<S: SpineCurve>(
    recipe: &SpineFrameRecipe<S, ProfileLaw, FrameLaw>,
    transform: &Matrix4,
    s: f64,
    v: f64,
) -> Vector3 {
    let frame = evaluate_frame(recipe, s);
    match profile_derivative_v(&recipe.profile_law, s, v) {
        Ok(pv) => transform.transform_vector(frame.normal * pv.x + frame.binormal * pv.y),
        Err(err) => panic!("spine-frame refused v-derivative at ({s}, {v}): {err}"),
    }
}

/// `S_s = C'(s) + frame(s) · ∂P/∂s`: the spine derivative plus the frame
/// evaluator (the frame-twist term is outside the landed evaluators), then
/// placed. Central differences at `DirectTolerance::parameter` scale are
/// sanctioned only on the search path, never here.
#[inline(always)]
pub(crate) fn surface_uder<S: SpineCurve>(
    recipe: &SpineFrameRecipe<S, ProfileLaw, FrameLaw>,
    transform: &Matrix4,
    s: f64,
    v: f64,
) -> Vector3 {
    let c = match recipe.spine.derivative_at(s) {
        Ok(c) => c,
        Err(err) => panic!("spine-frame refused spine derivative at {s}: {err}"),
    };
    let frame = evaluate_frame(recipe, s);
    match profile_derivative_s(&recipe.profile_law, s, v) {
        Ok(ps) => transform.transform_vector(c + frame.normal * ps.x + frame.binormal * ps.y),
        Err(err) => panic!("spine-frame refused s-derivative at ({s}, {v}): {err}"),
    }
}

/// The central-difference second derivative of a first derivative, at
/// `DirectTolerance::parameter` scale, sampled along `s`. Sanctioned for the
/// SEARCH path only: `SearchParameter`/`SearchNearestParameter` are numerical
/// searches and their certificates never quote these values. Shared by the
/// realization decorator and the closed whole-sweep value (the same landed
/// derivative machinery, closure-form so both carriers can call it).
pub(crate) fn central_difference_s(s: f64, v: f64, first: impl Fn(f64, f64) -> Vector3) -> Vector3 {
    let h = DirectTolerance::default().parameter;
    (first(s + h, v) - first(s - h, v)) / (2.0 * h)
}

/// The central-difference second derivative with respect to `v` (see
/// [`central_difference_s`]).
pub(crate) fn central_difference_v(s: f64, v: f64, first: impl Fn(f64, f64) -> Vector3) -> Vector3 {
    let h = DirectTolerance::default().parameter;
    (first(s, v + h) - first(s, v - h)) / (2.0 * h)
}

impl<S: SpineCurve + Clone> ParametricSurface for SpineFrameSurface<S> {
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

impl<S: SpineCurve + Clone> ParametricSurface3D for SpineFrameSurface<S> {}

impl<S: SpineCurve + Clone> BoundedSurface for SpineFrameSurface<S> {}

impl<S: SpineCurve + Clone> ParameterDivision2D for SpineFrameSurface<S> {
    fn parameter_division(
        &self,
        range: ((f64, f64), (f64, f64)),
        tol: f64,
    ) -> (Vec<f64>, Vec<f64>) {
        algo::surface::parameter_division(self, range, tol)
    }
}

impl<S: SpineCurve + Clone> SearchParameter<D2> for SpineFrameSurface<S> {
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
                algo::surface::presearch(self, point, (u, v), PRESEARCH_DIVISION)
            }
            SPHint2D::None => {
                algo::surface::presearch(self, point, (urange, vrange), PRESEARCH_DIVISION)
            }
        };
        algo::surface::search_parameter(self, point, hint, trials)
    }
}

impl<S: SpineCurve + Clone> SearchNearestParameter<D2> for SpineFrameSurface<S> {
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
                algo::surface::presearch(self, point, (u, v), PRESEARCH_DIVISION)
            }
            SPHint2D::None => {
                algo::surface::presearch(self, point, (urange, vrange), PRESEARCH_DIVISION)
            }
        };
        algo::surface::search_nearest_parameter(self, point, hint, trials)
    }
}

impl<S: SpineCurve + Clone> Invertible for SpineFrameSurface<S> {
    #[inline(always)]
    fn invert(&mut self) {
        std::mem::swap(&mut self.v0, &mut self.v1);
    }
}

impl<S: SpineCurve + Clone> Transformed<Matrix4> for SpineFrameSurface<S> {
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

/// Whether the surface includes one of its own boundary curves. A ring `Line`
/// is compared structurally (a placed ring is still a `Line`); a trajectory
/// `SpineFrameCurve`'s containment cannot be certified structurally — the
/// canonical `Curve` carries no equality and a placed trajectory is not even
/// representable as a canonical `SpineFrameCurve` — so the question refuses
/// typed (`UncertifiedContainment`), the BG-S0-001 doctrine, rather than
/// approximate.
impl IncludeCurve<Curve> for SpineFrameSurface<Box<Curve>> {
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

impl<S: SpineCurve + Clone> ParametricCurve for SpineFrameCurve<S> {
    type Point = Point3;
    type Vector = Vector3;
    #[inline(always)]
    fn subs(&self, t: f64) -> Point3 {
        evaluate_position(&self.recipe, &self.transform, t, self.v_p)
    }
    #[inline(always)]
    fn der(&self, t: f64) -> Vector3 {
        surface_uder(&self.recipe, &self.transform, t, self.v_p)
    }
    #[inline(always)]
    fn der2(&self, t: f64) -> Vector3 {
        let h = DirectTolerance::default().parameter;
        (self.der(t + h) - self.der(t - h)) / (2.0 * h)
    }
    #[inline(always)]
    fn der_n(&self, n: usize, t: f64) -> Vector3 {
        match n {
            0 => self.subs(t).to_vec(),
            1 => self.der(t),
            _ => self.der2(t),
        }
    }
    #[inline(always)]
    fn parameter_range(&self) -> ParameterRange {
        (Bound::Included(self.s0), Bound::Included(self.s1))
    }
}

impl<S: SpineCurve + Clone> BoundedCurve for SpineFrameCurve<S> {}

impl<S: SpineCurve + Clone> ParameterDivision1D for SpineFrameCurve<S> {
    type Point = Point3;
    #[inline(always)]
    fn parameter_division(&self, range: (f64, f64), tol: f64) -> (Vec<f64>, Vec<Self::Point>) {
        algo::curve::parameter_division(self, range, tol)
    }
}

impl<S: SpineCurve + Clone> Cut for SpineFrameCurve<S> {
    #[inline(always)]
    fn cut(&mut self, t: f64) -> Self {
        let tail = Self {
            s0: t,
            ..self.clone()
        };
        self.s1 = t;
        tail
    }
}

impl<S: SpineCurve + Clone> Invertible for SpineFrameCurve<S> {
    #[inline(always)]
    fn invert(&mut self) {
        std::mem::swap(&mut self.s0, &mut self.s1);
    }
}

impl<S: SpineCurve + Clone> Transformed<Matrix4> for SpineFrameCurve<S> {
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

impl<S: SpineCurve + Clone> SearchNearestParameter<D1> for SpineFrameCurve<S> {
    type Point = Point3;
    fn search_nearest_parameter<H: Into<SPHint1D>>(
        &self,
        point: Point3,
        hint: H,
        trials: usize,
    ) -> Option<f64> {
        let hint = match hint.into() {
            SPHint1D::Parameter(t) => t,
            SPHint1D::Range(a, b) => {
                algo::curve::presearch(self, point, (a, b), PRESEARCH_DIVISION)
            }
            SPHint1D::None => {
                algo::curve::presearch(self, point, self.range_tuple(), PRESEARCH_DIVISION)
            }
        };
        algo::curve::search_nearest_parameter(self, point, hint, trials)
    }
}

impl<S: SpineCurve + Clone> SearchParameter<D1> for SpineFrameCurve<S> {
    type Point = Point3;
    fn search_parameter<H: Into<SPHint1D>>(
        &self,
        point: Point3,
        hint: H,
        trials: usize,
    ) -> Option<f64> {
        // The landed IntersectionCurve Newton pattern: delegate to the host
        // surface's `SearchParameter`, restricted to the vertex line v = v_p.
        let hint = hint.into();
        let host_hint = match hint {
            SPHint1D::Parameter(t) => SPHint2D::Parameter(t, self.v_p),
            SPHint1D::Range(a, b) => SPHint2D::Range((a, b), (self.v_p, self.v_p)),
            SPHint1D::None => SPHint2D::None,
        };
        // The host surface is the vertex line v = v_p of the same recipe: the
        // degenerate window `[v_p, v_p]`. Built directly (not through
        // `try_new`, whose one-edge window contract rejects a zero window) —
        // the recipe was already validated when the curve itself was built.
        let host = SpineFrameSurface {
            recipe: self.recipe.clone(),
            s0: self.s0,
            s1: self.s1,
            v0: self.v_p,
            v1: self.v_p,
            transform: self.transform,
        };
        let (s, v) = host.search_parameter(point, host_hint, trials)?;
        if (v - self.v_p).abs() > DirectTolerance::default().parameter {
            return None;
        }
        if (self.subs(s) - point).magnitude() <= DirectTolerance::default().position {
            Some(s)
        } else {
            None
        }
    }
}

/// Whether this trajectory curve includes exactly itself. The canonical
/// `Curve` carries no equality, so the structural check cannot be certified;
/// the question refuses typed (`UncertifiedContainment`) rather than
/// approximate (the BG-S0-001 doctrine).
impl IncludeCurve<Curve> for SpineFrameCurve<Box<Curve>> {
    fn include(&self, curve: &Curve) -> Outcome<bool> {
        match curve {
            Curve::SpineFrameCurve(trajectory) => {
                let same = trajectory.s0() == self.s0
                    && trajectory.s1() == self.s1
                    && trajectory.v_p() == self.v_p
                    && *trajectory.transform() == self.transform;
                if same {
                    Ok(Certified::new(true, float_certificate()))
                } else {
                    Err(Refusal::NumericallyUnresolved {
                        spent: Budget::new(0, 0, 0),
                        witness: truck_base::evidence::UnresolvedWitness::UncertifiedContainment,
                    })
                }
            }
            _ => Ok(Certified::new(false, float_certificate())),
        }
    }
}

impl Serialize for ScalarLaw {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            ScalarLaw::Constant(c) => {
                serializer.serialize_newtype_variant("ScalarLaw", 0, "Constant", &c)
            }
            ScalarLaw::Linear { start, end } => {
                serializer.serialize_newtype_variant("ScalarLaw", 1, "Linear", &(start, end))
            }
        }
    }
}

impl<'de> Deserialize<'de> for ScalarLaw {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        enum Repr {
            Constant(f64),
            Linear(f64, f64),
        }
        Ok(match Repr::deserialize(deserializer)? {
            Repr::Constant(c) => ScalarLaw::Constant(c),
            Repr::Linear(start, end) => ScalarLaw::Linear { start, end },
        })
    }
}

impl Serialize for Profile2D {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Profile2D", 1)?;
        state.serialize_field("vertices", &self.vertices)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Profile2D {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Repr {
            vertices: Vec<Point2>,
        }
        let Repr { vertices } = Repr::deserialize(deserializer)?;
        Ok(Profile2D { vertices })
    }
}

impl Serialize for ProfileLaw {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            ProfileLaw::Constant(profile) => {
                serializer.serialize_newtype_variant("ProfileLaw", 0, "Constant", profile)
            }
            ProfileLaw::Scale { profile, scale } => {
                serializer.serialize_newtype_variant("ProfileLaw", 1, "Scale", &(profile, scale))
            }
            ProfileLaw::LinearCorrespondence { start, end } => serializer
                .serialize_newtype_variant("ProfileLaw", 2, "LinearCorrespondence", &(start, end)),
        }
    }
}

impl<'de> Deserialize<'de> for ProfileLaw {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        enum Repr {
            Constant(Profile2D),
            Scale(Profile2D, ScalarLaw),
            LinearCorrespondence(Profile2D, Profile2D),
        }
        Ok(match Repr::deserialize(deserializer)? {
            Repr::Constant(profile) => ProfileLaw::Constant(profile),
            Repr::Scale(profile, scale) => ProfileLaw::Scale { profile, scale },
            Repr::LinearCorrespondence(start, end) => {
                ProfileLaw::LinearCorrespondence { start, end }
            }
        })
    }
}

impl Serialize for FrameLaw {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            FrameLaw::FixedPlane { normal } => {
                serializer.serialize_newtype_variant("FrameLaw", 0, "FixedPlane", &normal)
            }
            FrameLaw::ArchitecturalUp { up } => {
                serializer.serialize_newtype_variant("FrameLaw", 1, "ArchitecturalUp", &up)
            }
            FrameLaw::ParallelTransport { initial_normal } => serializer.serialize_newtype_variant(
                "FrameLaw",
                2,
                "ParallelTransport",
                &initial_normal,
            ),
            FrameLaw::RadialAboutAxis { origin, axis } => serializer.serialize_newtype_variant(
                "FrameLaw",
                3,
                "RadialAboutAxis",
                &(origin, axis),
            ),
        }
    }
}

impl<'de> Deserialize<'de> for FrameLaw {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        enum Repr {
            FixedPlane(Vector3),
            ArchitecturalUp(Vector3),
            ParallelTransport(Vector3),
            RadialAboutAxis(Point3, Vector3),
        }
        Ok(match Repr::deserialize(deserializer)? {
            Repr::FixedPlane(normal) => FrameLaw::FixedPlane { normal },
            Repr::ArchitecturalUp(up) => FrameLaw::ArchitecturalUp { up },
            Repr::ParallelTransport(initial_normal) => {
                FrameLaw::ParallelTransport { initial_normal }
            }
            Repr::RadialAboutAxis(origin, axis) => FrameLaw::RadialAboutAxis { origin, axis },
        })
    }
}

impl<S: Serialize, P: Serialize, F: Serialize> Serialize for SpineFrameRecipe<S, P, F> {
    fn serialize<Ser>(&self, serializer: Ser) -> std::result::Result<Ser::Ok, Ser::Error>
    where
        Ser: Serializer,
    {
        let mut state = serializer.serialize_struct("SpineFrameRecipe", 4)?;
        state.serialize_field("spine", &self.spine)?;
        state.serialize_field("profile_law", &self.profile_law)?;
        state.serialize_field("frame_law", &self.frame_law)?;
        state.serialize_field("frame_data", &self.frame_data)?;
        state.end()
    }
}

impl<'de, S: Deserialize<'de>, P: Deserialize<'de>, F: Deserialize<'de>> Deserialize<'de>
    for SpineFrameRecipe<S, P, F>
{
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Repr<S, P, F> {
            spine: S,
            profile_law: P,
            frame_law: F,
            frame_data: FrameData,
        }
        let Repr {
            spine,
            profile_law,
            frame_law,
            frame_data,
        } = Repr::deserialize(deserializer)?;
        Ok(SpineFrameRecipe {
            spine,
            profile_law,
            frame_law,
            frame_data,
        })
    }
}

impl<S: Serialize> Serialize for SpineFrameSurface<S> {
    fn serialize<Ser>(&self, serializer: Ser) -> std::result::Result<Ser::Ok, Ser::Error>
    where
        Ser: Serializer,
    {
        let mut state = serializer.serialize_struct("SpineFrameSurface", 6)?;
        state.serialize_field("recipe", &self.recipe)?;
        state.serialize_field("s0", &self.s0)?;
        state.serialize_field("s1", &self.s1)?;
        state.serialize_field("v0", &self.v0)?;
        state.serialize_field("v1", &self.v1)?;
        state.serialize_field("transform", &self.transform)?;
        state.end()
    }
}

impl<'de, S: Deserialize<'de>> Deserialize<'de> for SpineFrameSurface<S> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Repr<S> {
            recipe: SpineFrameRecipe<S, ProfileLaw, FrameLaw>,
            s0: f64,
            s1: f64,
            v0: f64,
            v1: f64,
            transform: Matrix4,
        }
        let Repr {
            recipe,
            s0,
            s1,
            v0,
            v1,
            transform,
        } = Repr::deserialize(deserializer)?;
        Ok(SpineFrameSurface {
            recipe,
            s0,
            s1,
            v0,
            v1,
            transform,
        })
    }
}

impl<S: Serialize> Serialize for SpineFrameCurve<S> {
    fn serialize<Ser>(&self, serializer: Ser) -> std::result::Result<Ser::Ok, Ser::Error>
    where
        Ser: Serializer,
    {
        let mut state = serializer.serialize_struct("SpineFrameCurve", 5)?;
        state.serialize_field("recipe", &self.recipe)?;
        state.serialize_field("s0", &self.s0)?;
        state.serialize_field("s1", &self.s1)?;
        state.serialize_field("v_p", &self.v_p)?;
        state.serialize_field("transform", &self.transform)?;
        state.end()
    }
}

impl<'de, S: Deserialize<'de>> Deserialize<'de> for SpineFrameCurve<S> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Repr<S> {
            recipe: SpineFrameRecipe<S, ProfileLaw, FrameLaw>,
            s0: f64,
            s1: f64,
            v_p: f64,
            transform: Matrix4,
        }
        let Repr {
            recipe,
            s0,
            s1,
            v_p,
            transform,
        } = Repr::deserialize(deserializer)?;
        Ok(SpineFrameCurve {
            recipe,
            s0,
            s1,
            v_p,
            transform,
        })
    }
}
