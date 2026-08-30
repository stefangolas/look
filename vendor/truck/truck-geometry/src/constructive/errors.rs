#![deny(clippy::unwrap_used)]

//! BG-CG-000-CONTRACT — typed refusals of constructive evaluation.

use thiserror::Error;

/// Typed refusal of a constructive evaluation or construction.
#[derive(Debug, Clone, Copy, PartialEq, Error)]
pub enum ConstructError {
    /// The spine derivative vanished at `at`; the frame is undefined there.
    /// Refused, never clamped (normative: plan §3.2, "Spine smoothness
    /// contract").
    #[error("zero tangent at s = {at}")]
    ZeroTangent {
        /// The spine parameter where the tangent vanished.
        at: f64,
    },
    /// The named frame law is singular at `at` (e.g. `ArchitecturalUp` with
    /// `up ∥ t`). `law` names the law (see `FrameLaw::law_name`); the recipe
    /// refuses, it never rotates the frame silently.
    #[error("frame law `{law}` is singular at s = {at}")]
    FrameSingular {
        /// The spine parameter where the frame law is singular.
        at: f64,
        /// Which law refused; matches `FrameLaw::law_name`.
        law: &'static str,
    },
    /// The spine is not C¹ on the evaluated interval (tangent discontinuity
    /// beyond `DirectTolerance::parameter`, or a declaration-based detection).
    /// Non-C¹ spines are typed-refused, never clamped or silently smoothed.
    #[error("spine is not C1 at s = {at}")]
    SpineNotC1 {
        /// The spine parameter where C¹ fails.
        at: f64,
    },
    /// `ProfileLaw::LinearCorrespondence` was asked to pair profiles whose
    /// vertex counts differ. Correspondence is explicit, never inferred.
    #[error("profile correspondence mismatch")]
    ProfileCorrespondenceMismatch,
    /// The profile degenerated at `at` (e.g. a `Scale` law through zero).
    #[error("profile collapses at s = {at}")]
    ProfileCollapse {
        /// The spine parameter where the profile collapsed.
        at: f64,
    },
    /// A computed value was non-finite at `at`.
    #[error("non-finite value at s = {at}")]
    NonFinite {
        /// The spine parameter where a non-finite value appeared.
        at: f64,
    },
    /// Structurally invalid input to a constructor (wrong arity, non-finite
    /// fixture data, a non-orthonormal frame). Constructor validation only;
    /// evaluation-time failures use the parameter-bearing variants above.
    #[error("invalid input")]
    InvalidInput,
}
