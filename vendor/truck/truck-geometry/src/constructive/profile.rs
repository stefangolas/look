#![deny(clippy::unwrap_used)]

//! BG-CG-001-RECIPE — per-station profile evaluation.

use super::errors::ConstructError;
use super::{DirectTolerance, Profile2D, ProfileLaw};
use truck_base::cgmath64::*;

/// DEF-SPINEFRAME-GRAZE-R2 — the ONE named H-3 profile-v domain pad: the
/// `v`-domain refusal is padded by this on BOTH ends at every v-check site
/// (profile evaluation and the two profile derivatives), sized at 1.75×
/// `DirectTolerance::parameter` against the r1 witness class (the measured
/// v-axis overshoot ladder: 1.21× then 1.53× of the parameter tolerance).
/// This is the SECOND net, defense in depth; the primary fix is the SEARCH
/// layer clamp ([`SweepWindowClamp`](crate::constructive::SweepWindowClamp))
/// that keeps search iterates inside the sweep's certified window in the
/// first place. The pad still refuses the landed `-2×` probe, so no landed
/// refusal assertion weakens.
// H-3 profile-v domain pad, 1.75x DirectTolerance::parameter
pub(crate) const PROFILE_V_DOMAIN_PAD: f64 = 1.75e-6;

impl ProfileLaw {
    /// The profile point P(s, v): the profile law applied at spine station
    /// `s`, ring parameter `v ∈ [0, 1]` (admitted within the named H-3
    /// [`PROFILE_V_DOMAIN_PAD`] beyond each end — the search path's
    /// defense-in-depth second net, DEF-SPINEFRAME-GRAZE-R2).
    ///
    /// Refusals (CG-001): either parameter non-finite → `NonFinite { at: s }`
    /// (both kinds report the spine parameter `s`); `v` beyond the padded
    /// domain → `InvalidInput`; a `Scale` law whose scalar magnitude is within
    /// `DirectTolerance::default().parameter` of zero → `ProfileCollapse`.
    pub fn evaluate(&self, s: f64, v: f64) -> Result<Point2, ConstructError> {
        if !s.is_finite() || !v.is_finite() {
            return Err(ConstructError::NonFinite { at: s });
        }
        if !(-PROFILE_V_DOMAIN_PAD..=1.0 + PROFILE_V_DOMAIN_PAD).contains(&v) {
            return Err(ConstructError::InvalidInput);
        }
        match self {
            ProfileLaw::Constant(p) => Ok(ring_point(p, v)),
            ProfileLaw::Scale { profile, scale } => {
                let c = scale.at(s);
                if c.abs() <= DirectTolerance::default().parameter {
                    return Err(ConstructError::ProfileCollapse { at: s });
                }
                Ok(ring_point(profile, v) * c)
            }
            ProfileLaw::LinearCorrespondence { start, end } => {
                // SEM-FACET-CORRESPONDENCE-TRUNCATION-001: a correspondence
                // whose vertex counts differ is refused at EVALUATION time too,
                // not only by `try_linear_correspondence` — a struct-literal
                // law bypasses the constructor, and `zip` would silently
                // truncate the longer profile. Correspondence is never
                // inferred.
                super::validation::validate_correspondence(
                    start.vertices.len(),
                    end.vertices.len(),
                )?;
                let interpolated = Profile2D {
                    vertices: start
                        .vertices
                        .iter()
                        .zip(end.vertices.iter())
                        .map(|(a, b)| a + (b - a) * s)
                        .collect(),
                };
                Ok(ring_point(&interpolated, v))
            }
        }
    }
}

/// The profile ring point at `v ∈ [0, 1]`, uniform per edge (NOT arc-length):
/// with `k` vertices, vertex `j` sits at `v = j / k`, and `v = 1.0` lands on
/// vertex 0 (the closing edge's end == start — the implicit closure).
fn ring_point(profile: &Profile2D, v: f64) -> Point2 {
    let k = profile.vertices.len();
    let x = v * k as f64;
    let e = (x.floor() as usize).min(k - 1);
    let f = x - e as f64;
    profile.vertices[e] + (profile.vertices[(e + 1) % k] - profile.vertices[e]) * f
}
