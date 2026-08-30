#![deny(clippy::unwrap_used)]

//! BG-CG-000-CONTRACT — the spine sampling policy.

use super::errors::ConstructError;

/// How the spine parameter axis is sampled for realization.
///
/// Determinism is normative (plan §7): identical ordered input + tolerance
/// produces byte-identical sample lists, repeated runs. Resolved sample lists
/// are sorted ascending and contain no duplicates; nothing about the output
/// may derive from hash-map iteration order.
#[derive(Debug, Clone, PartialEq)]
pub enum SamplingPolicy {
    /// `spine` uniformly spaced stations over the recipe's spine parameter
    /// domain (inclusive of both endpoints).
    UniformCount {
        /// The number of spine stations, >= 2.
        spine: usize,
    },
    /// The exact station list, caller-owned and used verbatim (sorted).
    CustomParameters(Vec<f64>),
    /// Refine until the chordal deviation of the spine polyline is within the
    /// given bound (a length; compared through `DirectTolerance::position`
    /// semantics, never a bare literal).
    ChordTolerance(f64),
    /// Refine until the tangent-direction change between adjacent stations is
    /// within the given bound (radians).
    AngularTolerance(f64),
}

impl SamplingPolicy {
    /// Resolves the policy over the spine parameter window `[s0, s1]` into a
    /// sorted, duplicate-free station list.
    ///
    /// STUB (BG-CG-000-CONTRACT): the resolution fills in with the recipe
    /// work (CG-001+). Until then every call refuses; it never panics (H-1)
    /// and never returns a fabricated sample list.
    pub fn resolve(&self, _s0: f64, _s1: f64) -> Result<Vec<f64>, ConstructError> {
        Err(ConstructError::InvalidInput)
    }
}
