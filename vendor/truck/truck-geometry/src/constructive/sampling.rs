#![deny(clippy::unwrap_used)]

//! BG-CG-000-CONTRACT — the spine sampling policy.

use super::errors::ConstructError;
use std::cmp::Ordering;

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
    /// - A descending window (`s0 > s1`) refuses `InvalidInput` — the window
    ///   must be ascending.
    /// - `UniformCount { spine }` needs at least 2 stations and returns the
    ///   n stations `s0 + (s1 - s0) * (i / (n - 1))` for `i in 0..n`,
    ///   inclusive of both endpoints.
    /// - `CustomParameters(list)` uses the caller-owned list verbatim after
    ///   validation and normalization: non-empty, every member finite, sorted
    ///   ascending, adjacent duplicates removed by exact `f64` equality. The
    ///   `[s0, s1]` window is deliberately IGNORED for this variant —
    ///   caller-owned parameters take precedence.
    /// - `ChordTolerance(_) | AngularTolerance(_)` still refuse
    ///   `Err(InvalidInput)` in CG-001: they require spine-aware refinement
    ///   (they must consume `Spine::derivative_at`); booked as a follow-up
    ///   packet, deliberately NOT filled here.
    pub fn resolve(&self, s0: f64, s1: f64) -> Result<Vec<f64>, ConstructError> {
        if s0 > s1 {
            return Err(ConstructError::InvalidInput);
        }
        match self {
            SamplingPolicy::UniformCount { spine } => {
                if *spine < 2 {
                    return Err(ConstructError::InvalidInput);
                }
                let n = *spine;
                let denom = (n - 1) as f64;
                Ok((0..n)
                    .map(|i| s0 + (s1 - s0) * (i as f64) / denom)
                    .collect())
            }
            SamplingPolicy::CustomParameters(list) => {
                if list.is_empty() || list.iter().any(|p| !p.is_finite()) {
                    return Err(ConstructError::InvalidInput);
                }
                let mut sorted = list.clone();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
                sorted.dedup();
                Ok(sorted)
            }
            SamplingPolicy::ChordTolerance(_) | SamplingPolicy::AngularTolerance(_) => {
                Err(ConstructError::InvalidInput)
            }
        }
    }
}
