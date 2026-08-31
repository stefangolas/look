#![deny(clippy::unwrap_used)]

//! BG-CG-000-CONTRACT — the core evaluator: X(s, v) = C(s) + T(s)·P(s, v).

use super::errors::ConstructError;
use super::{DirectTolerance, Frame3, FrameLaw, ProfileLaw};
use truck_base::cgmath64::*;

/// The core recipe: a spine curve, a profile law transported along it, and the
/// frame law that orients the profile. `S` is the spine; CG-000 freezes the
/// struct and the evaluator signatures — the spine trait surface and the
/// evaluation bodies land with CG-001, so `S` carries no bound here yet.
#[derive(Debug, Clone, PartialEq)]
pub struct SpineFrameRecipe<S, P, F> {
    /// The spine curve C(s). Unbounded until CG-001 books the spine trait.
    pub spine: S,
    /// The profile law P(s, v).
    pub profile_law: P,
    /// The frame law producing T(s).
    pub frame_law: F,
}

impl<S, P, F> SpineFrameRecipe<S, P, F> {
    /// Assembles a recipe. No validation yet: construction is structural;
    /// refusal happens at evaluation, with a spine parameter attached.
    pub const fn new(spine: S, profile_law: P, frame_law: F) -> Self {
        Self {
            spine,
            profile_law,
            frame_law,
        }
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
pub trait Spine {
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

impl Spine for LineSpine {
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

impl Spine for PolylineSpine {
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

impl<S: Spine> SpineFrameRecipe<S, ProfileLaw, FrameLaw> {
    /// The realized point `X(s, v) = C(s) + T(s)·P(s, v)`.
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
        Ok(c + f.tangent * p.x + f.normal * p.y)
    }

    /// The frame at `s` (see `Frame3` for the axis convention).
    ///
    /// STUB (BG-CG-000-CONTRACT): the frame laws land with CG-002 (analytic)
    /// and CG-003 (transport); CG-001 filled everything frame-adjacent (the
    /// spine trait, C¹ refusals) so the frame laws only swap this body.
    pub fn frame(&self, _s: f64) -> Result<Frame3, ConstructError> {
        Err(ConstructError::InvalidInput)
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
