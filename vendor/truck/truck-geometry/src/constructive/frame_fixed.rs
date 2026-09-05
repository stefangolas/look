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
/// non-finite or of vanishing magnitude (the zero plane normal), or when the
/// completed triple fails the orthonormal right-handed `Frame3` gate (a
/// non-planar spine, whose tangent leaves the pinned plane, refuses in v1).
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
    let frame_normal = binormal.cross(tangent);
    // ORI-FRAME-ORTHONORMALITY-GATE-001: every frame law routes its result
    // through the landed validated constructor. The completed frame is
    // orthonormal only while the spine tangent stays perpendicular to the
    // pinned plane normal — on a non-planar spine `|b × t|` drops below unit,
    // the `Frame3` gate refuses, and the law reports the typed
    // `FrameSingular`. v1 REFUSES on non-planar spines; projection onto the
    // spine's osculating plane is a later amendment.
    Frame3::try_new(tangent, frame_normal, binormal).map_err(|_| ConstructError::FrameSingular {
        at,
        law: "FixedPlane",
    })
}
