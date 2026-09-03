#![deny(clippy::unwrap_used)]

//! BG-CG-000-CONTRACT — the core evaluator: X(s, v) = C(s) + T(s)·P(s, v).
//!
//! BG-KV2-203-C1DELTA (r3): the spine trait renames to [`SpineCurve`] and the
//! spec-§5.2 [`Spine`] enum lands beside it — `Ph(PhSpine)` (the exact PH
//! fast path; never an admission criterion) and `General(Box<dyn SpineCurve>)`
//! (procedural, non-rational, first-class). The recipe gains [`FrameData`]
//! (spec §5.3): the declared double-reflection refinement level, defaulted to
//! the landed 64-station grid so default-level behavior is bit-identical.

use super::errors::ConstructError;
use super::spine_ph::PhSpine;
use super::{DirectTolerance, Frame3, FrameLaw, ProfileLaw};
use serde::{Deserialize, Serialize};
use truck_base::cgmath64::*;

/// The declared transport refinement level, stored as data (spec §5.3).
///
/// For `FrameLaw::ParallelTransport` over a *general* spine the double-
/// reflection transport grid runs at [`refinement_level`](Self) stations
/// instead of the landed hardcoded 64; the default stays 64 so landed
/// behavior is bit-identical at the default (the stop-condition-3 premise).
/// Changing the recorded level changes the transported frame — and therefore
/// the surface — by design, and the surface is resolution-independent once
/// frozen at a level. A `Ph` spine does not consume this field: its frame is
/// the exact rational rotation-minimizing frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameData {
    /// The transport station count. Must be >= 2 (the station-grid arithmetic
    /// divides by `n - 1`); a recipe evaluated with a smaller level refuses.
    pub refinement_level: u32,
}

impl FrameData {
    /// The landed transport station count (the pre-C1DELTA hardcoded
    /// `TRANSPORT_STATIONS` in `constructive/frame_transport.rs`). The
    /// [`Default`] refinement level: keeps landed behavior bit-identical.
    pub const DEFAULT_REFINEMENT_LEVEL: u32 = 64;
}

impl Default for FrameData {
    fn default() -> Self {
        Self {
            refinement_level: Self::DEFAULT_REFINEMENT_LEVEL,
        }
    }
}

/// The core recipe: a spine curve, a profile law transported along it, the
/// frame law that orients the profile, and the declared transport refinement
/// level. `S` is the spine; CG-000 freezes the struct and the evaluator
/// signatures — the spine trait surface and the evaluation bodies land with
/// CG-001, so `S` carries no bound here yet.
#[derive(Debug, Clone, PartialEq)]
pub struct SpineFrameRecipe<S, P, F> {
    /// The spine curve C(s). Unbounded until CG-001 books the spine trait.
    pub spine: S,
    /// The profile law P(s, v).
    pub profile_law: P,
    /// The frame law producing T(s).
    pub frame_law: F,
    /// The declared transport refinement level (spec §5.3), stored as data.
    pub frame_data: FrameData,
}

impl<S, P, F> SpineFrameRecipe<S, P, F> {
    /// Assembles a recipe. No validation yet: construction is structural;
    /// refusal happens at evaluation, with a spine parameter attached. The
    /// frame data defaults to the landed 64-station refinement level.
    pub const fn new(spine: S, profile_law: P, frame_law: F) -> Self {
        Self {
            spine,
            profile_law,
            frame_law,
            frame_data: FrameData {
                refinement_level: FrameData::DEFAULT_REFINEMENT_LEVEL,
            },
        }
    }

    /// Returns the recipe with the given declared transport refinement level
    /// (a `try_new`-style constructor: it defaults `frame_data` unless told
    /// otherwise, and records what it set).
    pub fn with_frame_data(mut self, frame_data: FrameData) -> Self {
        self.frame_data = frame_data;
        self
    }

    /// The declared transport refinement level stored on the recipe.
    pub fn frame_data(&self) -> FrameData {
        self.frame_data
    }
}

/// The spine curve C(s) of a recipe: position and first derivative over a
/// bounded parameter domain. This is the CG-001 spine surface; realizations
/// that need higher derivatives book them additively later.
///
/// C¹ contract (normative, plan §3.2): a spine consumed on an interval must be
/// C¹ there. There is no global screening pass in CG-001 — the refusal fires
/// where the tangent is actually consumed (frame laws, CG-002/003) or where
/// the spine type itself declares non-C¹ (`PolylineSpine::derivative_at`
/// refuses at corners). This boundary is deliberate; do not add a scan.
pub trait SpineCurve {
    /// The closed parameter domain `[s_min, s_max]`.
    fn domain(&self) -> (f64, f64);

    /// The spine point C(s). Total on the domain; outside it (beyond
    /// `DirectTolerance::parameter`), refuse `ConstructError::InvalidInput`.
    fn position_at(&self, s: f64) -> Result<Point3, ConstructError>;

    /// The (unnormalized) tangent C'(s). Frame laws normalize; a vanishing
    /// derivative is refused downstream as `ZeroTangent` (CG-002's business,
    /// not the spine's).
    fn derivative_at(&self, s: f64) -> Result<Vector3, ConstructError>;
}

/// The spec-§5.2 spine: the enum that names the two first-class spine kinds.
///
/// - [`Spine::Ph`] is the exact fast path: a Pythagorean-hodograph spine
///   whose rotation-minimizing frame is rational (spec §5.3: no ODE, no
///   approximation). `Ph` is a fast path, never an admission criterion.
/// - [`Spine::General`] wraps any landed [`SpineCurve`] behind a trait
///   object: procedural, non-rational, first-class. A general B-spline spine
///   sweeps to a working surface; it is never refused for promotion.
///
/// The enum itself implements [`SpineCurve`] by delegation, so a recipe whose
/// spine is `Spine` evaluates exactly as it evaluated the trait.
pub enum Spine {
    /// A Pythagorean-hodograph spine (exact rational frame fast path).
    Ph(PhSpine),
    /// A procedural, non-rational spine behind the trait object.
    General(Box<dyn SpineCurve>),
}

impl std::fmt::Debug for Spine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The `General` payload is a `dyn SpineCurve`, which carries no
        // `Debug`; the variant is printed opaquely (the enum is not `Clone`
        // either — the trait object forbids it).
        match self {
            Spine::Ph(ph) => f.debug_tuple("Spine::Ph").field(ph).finish(),
            Spine::General(_) => f.write_str("Spine::General(<dyn SpineCurve>)"),
        }
    }
}

impl Spine {
    /// Wraps a Pythagorean-hodograph spine.
    pub fn ph(ph: PhSpine) -> Self {
        Spine::Ph(ph)
    }

    /// Wraps any landed spine curve as a first-class general spine.
    pub fn general(curve: impl SpineCurve + 'static) -> Self {
        Spine::General(Box::new(curve))
    }

    /// The closed parameter domain.
    pub fn domain(&self) -> (f64, f64) {
        match self {
            Spine::Ph(ph) => ph.domain(),
            Spine::General(general) => general.domain(),
        }
    }

    /// The spine point C(s).
    pub fn position_at(&self, s: f64) -> Result<Point3, ConstructError> {
        match self {
            Spine::Ph(ph) => ph.position_at(s),
            Spine::General(general) => general.position_at(s),
        }
    }

    /// The (unnormalized) tangent C'(s).
    pub fn derivative_at(&self, s: f64) -> Result<Vector3, ConstructError> {
        match self {
            Spine::Ph(ph) => ph.derivative_at(s),
            Spine::General(general) => general.derivative_at(s),
        }
    }
}

impl SpineCurve for Spine {
    fn domain(&self) -> (f64, f64) {
        Spine::domain(self)
    }

    fn position_at(&self, s: f64) -> Result<Point3, ConstructError> {
        Spine::position_at(self, s)
    }

    fn derivative_at(&self, s: f64) -> Result<Vector3, ConstructError> {
        Spine::derivative_at(self, s)
    }
}

impl From<PhSpine> for Spine {
    fn from(ph: PhSpine) -> Self {
        Spine::Ph(ph)
    }
}

impl From<crate::constructive::spine_ph::RmErfSeptic> for Spine {
    fn from(spine: crate::constructive::spine_ph::RmErfSeptic) -> Self {
        Spine::Ph(PhSpine::RmErfSeptic(Box::new(spine)))
    }
}

impl From<LineSpine> for Spine {
    fn from(spine: LineSpine) -> Self {
        Spine::General(Box::new(spine))
    }
}

impl From<PolylineSpine> for Spine {
    fn from(spine: PolylineSpine) -> Self {
        Spine::General(Box::new(spine))
    }
}

/// A straight segment spine: C(s) = start + (end - start) * s on [0, 1].
/// C¹ trivially; `derivative_at` is the constant `end - start` (not
/// normalized). A degenerate start == end is NOT refused here — the zero
/// tangent refuses downstream (`ZeroTangent`, frame side), because the spine
/// itself is still a total, honest map.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineSpine {
    /// The segment start, C(0).
    pub start: Point3,
    /// The segment end, C(1).
    pub end: Point3,
}

impl SpineCurve for LineSpine {
    fn domain(&self) -> (f64, f64) {
        (0.0, 1.0)
    }

    fn position_at(&self, s: f64) -> Result<Point3, ConstructError> {
        if !s.is_finite() {
            return Err(ConstructError::NonFinite { at: s });
        }
        let tol = DirectTolerance::default().parameter;
        if s < -tol || s > 1.0 + tol {
            return Err(ConstructError::InvalidInput);
        }
        Ok(self.start + (self.end - self.start) * s)
    }

    fn derivative_at(&self, s: f64) -> Result<Vector3, ConstructError> {
        if !s.is_finite() {
            return Err(ConstructError::NonFinite { at: s });
        }
        let tol = DirectTolerance::default().parameter;
        if s < -tol || s > 1.0 + tol {
            return Err(ConstructError::InvalidInput);
        }
        Ok(self.end - self.start)
    }
}

/// A piecewise-linear spine through `vertices`: segment i covers
/// [i, i + 1], so the domain is [0, vertices.len() - 1] and the interior
/// integers 1 ..= n - 2 are CORNERS. Declared non-C¹:
/// `derivative_at` refuses `ConstructError::SpineNotC1 { at: s }` for any s
/// within `DirectTolerance::default().parameter` of a corner, and succeeds
/// mid-segment with that segment's (constant) direction. `position_at` is
/// total on the domain (piecewise-linear interpolation). This typed refusal
/// is the plan §7 C¹ gate: the fixture refuses, it never clamps or smooths.
#[derive(Debug, Clone, PartialEq)]
pub struct PolylineSpine {
    /// The polyline vertices in order; segment i joins vertex i to i + 1.
    pub vertices: Vec<Point3>,
}

impl PolylineSpine {
    /// Validates and builds the spine: at least two vertices, every
    /// coordinate finite; otherwise `ConstructError::InvalidInput`.
    pub fn try_new(vertices: Vec<Point3>) -> Result<PolylineSpine, ConstructError> {
        if vertices.len() < 2 {
            return Err(ConstructError::InvalidInput);
        }
        let finite = vertices
            .iter()
            .all(|v| v.x.is_finite() && v.y.is_finite() && v.z.is_finite());
        if !finite {
            return Err(ConstructError::InvalidInput);
        }
        Ok(PolylineSpine { vertices })
    }
}

impl SpineCurve for PolylineSpine {
    fn domain(&self) -> (f64, f64) {
        (0.0, (self.vertices.len() - 1) as f64)
    }

    fn position_at(&self, s: f64) -> Result<Point3, ConstructError> {
        let n = self.vertices.len();
        if !s.is_finite() {
            return Err(ConstructError::NonFinite { at: s });
        }
        let tol = DirectTolerance::default().parameter;
        let hi = (n - 1) as f64;
        if s < -tol || s > hi + tol {
            return Err(ConstructError::InvalidInput);
        }
        let i = (s.floor() as usize).min(n - 2);
        let f = s - i as f64;
        let a = self.vertices[i];
        let b = self.vertices[i + 1];
        Ok(a + (b - a) * f)
    }

    fn derivative_at(&self, s: f64) -> Result<Vector3, ConstructError> {
        let n = self.vertices.len();
        if !s.is_finite() {
            return Err(ConstructError::NonFinite { at: s });
        }
        let tol = DirectTolerance::default().parameter;
        let hi = (n - 1) as f64;
        if s < -tol || s > hi + tol {
            return Err(ConstructError::InvalidInput);
        }
        for corner in 1..=(n - 2) {
            let c = corner as f64;
            if (s - c).abs() <= tol {
                return Err(ConstructError::SpineNotC1 { at: s });
            }
        }
        let i = (s.floor() as usize).min(n - 2);
        let a = self.vertices[i];
        let b = self.vertices[i + 1];
        Ok(b - a)
    }
}

impl<S: SpineCurve> SpineFrameRecipe<S, ProfileLaw, FrameLaw> {
    /// The realized point `X(s, v) = C(s) + T(s)·P(s, v)`.
    ///
    /// The profile plane maps profile-x to the frame NORMAL and profile-y to
    /// the frame BINORMAL — the cross-section rides the plane perpendicular
    /// to the tangent (r2: the tangent-embedded reading makes every straight
    /// sweep coplanar; proven empirically by the r1 worker).
    ///
    /// DEVIATION NOTE (frozen here, do not relitigate): the program plan
    /// spelled this `fn position(&self, s, v) -> Point3`. CG-000 freezes it
    /// fallible — a stub body must be total without lying (H-1 forbids
    /// panics; a fabricated zero point is a lie), and `profile` is fallible in
    /// the plan's own signature, so the composition cannot be less fallible
    /// than its parts. Semantics on the success path are unchanged.
    ///
    /// Filled (BG-CG-001-RECIPE): the composition is ordered — profile first
    /// (collapse/correspondence refusals fire before any frame work), spine
    /// second, frame last. The frame step is currently the stub refusal, so
    /// `position` refuses on every valid input until CG-002/003 land.
    pub fn position(&self, s: f64, v: f64) -> Result<Point3, ConstructError> {
        if !s.is_finite() || !v.is_finite() {
            return Err(ConstructError::NonFinite { at: s });
        }
        let p = self.profile(s, v)?;
        let c = self.spine.position_at(s)?;
        let f = self.frame(s)?;
        Ok(c + f.normal * p.x + f.binormal * p.y)
    }

    /// The frame at `s` (see `Frame3` for the axis convention).
    ///
    /// Filled (BG-CG-003-TRANSPORT).
    pub fn frame(&self, s: f64) -> Result<Frame3, ConstructError> {
        let d = self.spine.derivative_at(s)?;
        if !s.is_finite() {
            return Err(ConstructError::NonFinite { at: s });
        }
        let mag = d.magnitude();
        if mag <= DirectTolerance::default().position {
            return Err(ConstructError::ZeroTangent { at: s });
        }
        let t = d / mag;
        match self.frame_law {
            FrameLaw::FixedPlane { normal } => super::frame_fixed::fixed_plane(normal, t, s),
            FrameLaw::ArchitecturalUp { up } => super::frame_up::architectural_up(up, t, s),
            FrameLaw::RadialAboutAxis { origin, axis } => {
                let c = self.spine.position_at(s)?;
                super::frame_radial::radial_about_axis(origin, axis, c, t, s)
            }
            FrameLaw::ParallelTransport { initial_normal } => {
                // The double-reflection transport runs at the recipe's
                // DECLARED refinement level (spec §5.3). A `Ph` spine's exact
                // rational rotation-minimizing frame is the PhSpine fast path
                // and is exposed at the `PhSpine` level; routing it through
                // this generic dispatcher is the §5.10 closed-surface seam
                // (the landed decorators are generic over `S: SpineCurve`, so
                // specialization-free Rust cannot branch on `S == Spine::Ph`
                // here).
                super::frame_transport::parallel_transport(
                    initial_normal,
                    &self.spine,
                    self.frame_data.refinement_level as usize,
                    s,
                )
            }
        }
    }

    /// The transported profile point `P(s, v)` in the frame plane.
    ///
    /// Filled (BG-CG-001-RECIPE): delegates to `ProfileLaw::evaluate` — no
    /// duplicated semantics.
    pub fn profile(&self, s: f64, v: f64) -> Result<Point2, ConstructError> {
        if !s.is_finite() || !v.is_finite() {
            return Err(ConstructError::NonFinite { at: s });
        }
        self.profile_law.evaluate(s, v)
    }
}
