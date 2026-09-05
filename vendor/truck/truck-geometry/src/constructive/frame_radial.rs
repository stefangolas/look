#![deny(clippy::unwrap_used)]

//! BG-CG-002-FRAMES-ANALYTIC — the `RadialAboutAxis` frame law.
//!
//! Analytic from a fixed axis: the normal is the unit component of
//! `spine_point − origin` perpendicular to the axis (profile-y points radially
//! outward), and `b = t × n` with `t` the unit spine tangent handed in by the
//! dispatcher. Rotated copies are equivariant under a rotation about the axis
//! modulo floating-point. Refuses `FrameSingular` when the axis is non-finite
//! or zero, when `spine_point − origin` is zero or non-finite, or when the
//! perpendicular component vanishes (the spine point lies on the axis).
//! Reachable only through the recipe dispatcher (`FrameLaw::RadialAboutAxis`).

use super::{ConstructError, DirectTolerance, Frame3};
use truck_base::cgmath64::*;

/// The `RadialAboutAxis` law: the normal is the unit perpendicular component
/// of `spine_point − origin` away from the axis; the binormal is `t × n`.
/// Refuses `FrameSingular` for the axis degeneracies listed in the module
/// docs, or when the completed triple fails the orthonormal right-handed
/// `Frame3` gate (a spine tangent with a radial component refuses).
pub(super) fn radial_about_axis(
    origin: Point3,
    axis: Vector3,
    spine_point: Point3,
    tangent: Vector3,
    at: f64,
) -> Result<Frame3, ConstructError> {
    if !axis.x.is_finite() || !axis.y.is_finite() || !axis.z.is_finite() {
        return Err(ConstructError::FrameSingular {
            at,
            law: "RadialAboutAxis",
        });
    }
    let axis_mag = axis.magnitude();
    if axis_mag <= DirectTolerance::default().position {
        return Err(ConstructError::FrameSingular {
            at,
            law: "RadialAboutAxis",
        });
    }
    let axis_hat = axis / axis_mag;
    let d = spine_point - origin;
    if !d.x.is_finite() || !d.y.is_finite() || !d.z.is_finite() {
        return Err(ConstructError::FrameSingular {
            at,
            law: "RadialAboutAxis",
        });
    }
    if d.magnitude() <= DirectTolerance::default().position {
        return Err(ConstructError::FrameSingular {
            at,
            law: "RadialAboutAxis",
        });
    }
    let radial = d - axis_hat * d.dot(axis_hat);
    let radial_mag = radial.magnitude();
    if radial_mag <= DirectTolerance::default().position {
        return Err(ConstructError::FrameSingular {
            at,
            law: "RadialAboutAxis",
        });
    }
    let normal3 = radial / radial_mag;
    let binormal = tangent.cross(normal3);
    // ORI-FRAME-ORTHONORMALITY-GATE-001: every frame law routes its result
    // through the landed validated constructor. The completed frame is
    // orthonormal only while the spine tangent is perpendicular to the radial
    // normal; a tangent with a radial component makes `b = t × n` sub-unit and
    // the `Frame3` gate refuses — the typed `FrameSingular`, never a silently
    // sheared cross-section.
    Frame3::try_new(tangent, normal3, binormal).map_err(|_| ConstructError::FrameSingular {
        at,
        law: "RadialAboutAxis",
    })
}
