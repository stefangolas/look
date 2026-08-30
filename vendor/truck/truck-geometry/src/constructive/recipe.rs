#![deny(clippy::unwrap_used)]

//! BG-CG-000-CONTRACT — the core evaluator: X(s, v) = C(s) + T(s)·P(s, v).

use super::errors::ConstructError;
use super::Frame3;
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

    /// The realized point `X(s, v) = C(s) + T(s)·P(s, v)`.
    ///
    /// DEVIATION NOTE (frozen here, do not relitigate): the program plan
    /// spelled this `fn position(&self, s, v) -> Point3`. CG-000 freezes it
    /// fallible — a stub body must be total without lying (H-1 forbids
    /// panics; a fabricated zero point is a lie), and `profile` is fallible in
    /// the plan's own signature, so the composition cannot be less fallible
    /// than its parts. Semantics on the success path are unchanged.
    ///
    /// STUB (BG-CG-000-CONTRACT): CG-001 fills the evaluation.
    pub fn position(&self, _s: f64, _v: f64) -> Result<Point3, ConstructError> {
        Err(ConstructError::InvalidInput)
    }

    /// The frame at `s` (see `Frame3` for the axis convention).
    ///
    /// STUB (BG-CG-000-CONTRACT): the frame laws land with CG-002 (analytic)
    /// and CG-003 (transport).
    pub fn frame(&self, _s: f64) -> Result<Frame3, ConstructError> {
        Err(ConstructError::InvalidInput)
    }

    /// The transported profile point `P(s, v)` in the frame plane.
    ///
    /// STUB (BG-CG-000-CONTRACT): CG-001 fills the evaluation.
    pub fn profile(&self, _s: f64, _v: f64) -> Result<Point2, ConstructError> {
        Err(ConstructError::InvalidInput)
    }
}
