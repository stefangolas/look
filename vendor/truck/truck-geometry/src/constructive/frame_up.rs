#![deny(clippy::unwrap_used)]

//! BG-CG-002-FRAMES-ANALYTIC — the `ArchitecturalUp` frame law.
//!
//! The architectural up vector orients the binormal: `b = normalize(up × t)`,
//! `n = t × b`, with `t` the unit spine tangent from the dispatcher. Refuses
//! `FrameSingular` when `up` is non-finite, zero, or parallel to the tangent —
//! never silently rotates the frame, and there is no fallback policy in this
//! packet. Reachable only through the recipe dispatcher
//! (`FrameLaw::ArchitecturalUp`).

use super::{ConstructError, DirectTolerance, Frame3};
use truck_base::cgmath64::*;

/// The `ArchitecturalUp` law: the binormal is `normalize(up × t)`, the normal
/// is `t × b`. Refuses `FrameSingular` when `up` is non-finite, zero, or
/// parallel to `t` (the `up × t` magnitude is within the position bound).
pub(super) fn architectural_up(
    up: Vector3,
    tangent: Vector3,
    at: f64,
) -> Result<Frame3, ConstructError> {
    if !up.x.is_finite() || !up.y.is_finite() || !up.z.is_finite() {
        return Err(ConstructError::FrameSingular {
            at,
            law: "ArchitecturalUp",
        });
    }
    let cross = up.cross(tangent);
    let mag = cross.magnitude();
    if mag <= DirectTolerance::default().position {
        return Err(ConstructError::FrameSingular {
            at,
            law: "ArchitecturalUp",
        });
    }
    let binormal = cross / mag;
    Ok(Frame3 {
        tangent,
        normal: tangent.cross(binormal),
        binormal,
    })
}
