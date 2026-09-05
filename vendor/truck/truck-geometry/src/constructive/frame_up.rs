#![deny(clippy::unwrap_used)]

//! BG-CG-002-FRAMES-ANALYTIC — the `ArchitecturalUp` frame law.
//!
//! The architectural up vector orients the binormal: `b = normalize(up × t)`,
//! `n = b × t`, with `t` the unit spine tangent from the dispatcher. The
//! completion is right-handed (`t × n = b`), matching the `Frame3`
//! convention. Refuses `FrameSingular` when `up` is non-finite, zero, or
//! parallel to the tangent — never silently rotates the frame, and there is
//! no fallback policy in this packet. Reachable only through the recipe
//! dispatcher (`FrameLaw::ArchitecturalUp`).

use super::{ConstructError, DirectTolerance, Frame3};
use truck_base::cgmath64::*;

/// The `ArchitecturalUp` law: the binormal is `normalize(up × t)`, the normal
/// is `b × t`. Refuses `FrameSingular` when `up` is non-finite, zero, or
/// parallel to `t` (the `up × t` magnitude is within the position bound), or
/// when the completed triple fails the orthonormal right-handed `Frame3`
/// gate.
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
    // ORI-FRAME-HANDEDNESS-001: the right-handed completion of the b-up law is
    // `n = b × t` (so `t × n = b`), never `t × b` (which makes `t × n = -b`).
    let normal = binormal.cross(tangent);
    // ORI-FRAME-ORTHONORMALITY-GATE-001: every frame law routes its result
    // through the landed validated constructor; a gate failure is a typed
    // `FrameSingular`, never a silently-degraded frame.
    Frame3::try_new(tangent, normal, binormal).map_err(|_| ConstructError::FrameSingular {
        at,
        law: "ArchitecturalUp",
    })
}
