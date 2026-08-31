#![deny(clippy::unwrap_used)]

//! BG-CG-001-RECIPE — per-station profile evaluation.

use super::errors::ConstructError;
use super::{DirectTolerance, Profile2D, ProfileLaw};
use truck_base::cgmath64::*;

impl ProfileLaw {
    /// The profile point P(s, v): the profile law applied at spine station
    /// `s`, ring parameter `v ∈ [0, 1]`.
    ///
    /// Refusals (CG-001): either parameter non-finite → `NonFinite { at: s }`
    /// (both kinds report the spine parameter `s`); `v` beyond `[0, 1]` →
    /// `InvalidInput`; a `Scale` law whose scalar magnitude is within
    /// `DirectTolerance::default().parameter` of zero → `ProfileCollapse`.
    pub fn evaluate(&self, s: f64, v: f64) -> Result<Point2, ConstructError> {
        if !s.is_finite() || !v.is_finite() {
            return Err(ConstructError::NonFinite { at: s });
        }
        if !(0.0..=1.0).contains(&v) {
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
