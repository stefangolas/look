#![deny(clippy::unwrap_used)]

//! BG-CG-002-FRAMES-ANALYTIC — the `FixedPlane` frame law.
//!
//! Pins the binormal to the (normalized) plane normal: `b̂ = normalize(normal)`,
//! `n = b̂ × t`, with `t` the unit spine tangent from the dispatcher. Preferred
//! for planar spines, whose frames are constant. Reachable only through the
//! recipe dispatcher (`FrameLaw::FixedPlane`).

use super::{ConstructError, DirectTolerance, Frame3};
use truck_base::cgmath64::*;

/// The `FixedPlane` law: the binormal is the normalized plane normal, the
/// normal is `b̂ × t`. Refuses `FrameSingular` when the plane normal is
/// non-finite or of vanishing magnitude (the zero plane normal).
pub(super) fn fixed_plane(
    normal: Vector3,
    tangent: Vector3,
    at: f64,
) -> Result<Frame3, ConstructError> {
    if !normal.x.is_finite() || !normal.y.is_finite() || !normal.z.is_finite() {
        return Err(ConstructError::FrameSingular {
            at,
            law: "FixedPlane",
        });
    }
    let mag = normal.magnitude();
    if mag <= DirectTolerance::default().position {
        return Err(ConstructError::FrameSingular {
            at,
            law: "FixedPlane",
        });
    }
    let binormal = normal / mag;
    Ok(Frame3 {
        tangent,
        normal: binormal.cross(tangent),
        binormal,
    })
}
