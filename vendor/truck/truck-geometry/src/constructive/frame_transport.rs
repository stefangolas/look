#![deny(clippy::unwrap_used)]

//! BG-CG-003-TRANSPORT — the `ParallelTransport` frame law.
//!
//! Hanson–Ma double reflection over a station polyline: the normal is
//! transported station-to-station by reflecting it across the plane
//! perpendicular to the incoming chord and then across the plane bisecting
//! the incoming and outgoing chord directions. The composition of the two
//! reflections is the rotation carrying the incoming chord direction onto the
//! outgoing one, which yields the rotation-minimizing (Bishop) frame — stable
//! at zero curvature and through inflections, with twist only O(h²) per
//! transition. The transport grid is `refinement_level` uniform stations
//! (spec §5.3's `FrameData`: the declared refinement level, defaulted to the
//! landed 64-station count) over the spine's full domain plus the queried
//! parameter `s` as the final station when `s` is not exactly on the grid, so
//! `frame(s)` costs a bounded number of closed-form steps (the §3.3
//! fast-path contract). The frame is a pure function of
//! `(spine, initial_normal, refinement_level, s)`: no mutable state, no
//! caching. Reachable only through the recipe dispatcher
//! (`FrameLaw::ParallelTransport`). A `Ph` spine does not route here: its
//! exact rational rotation-minimizing frame is the PhSpine fast path.

use super::{ConstructError, DirectTolerance, Frame3, SpineCurve};
use truck_base::cgmath64::*;

/// The `ParallelTransport` law: the rotation-minimizing frame at `s` via the
/// Hanson–Ma double-reflection transport of `initial_normal` along the spine
/// over `refinement_level` uniform stations (spec §5.3's `FrameData`; the
/// default level 64 reproduces the landed behavior bit-identically).
///
/// The start tangent is `C'(s_min)/‖C'(s_min)‖`; `initial_normal` is
/// orthonormalized against it, and a non-finite, zero, or tangent-parallel
/// `initial_normal` refuses `FrameSingular` reported at the QUERIED `s`
/// (the caller's parameter). Refusals from the spine (`position_at` /
/// `derivative_at`, including `SpineNotC1` and out-of-domain `InvalidInput`)
/// propagate unchanged, and a vanishing tangent refuses `ZeroTangent` at the
/// parameter where it vanished. The frame is re-orthonormalized after every
/// transition and satisfies the `Frame3` convention (`t × n == b`, unit
/// lengths) at the emitted frame. A `refinement_level < 2` is structurally
/// invalid (the grid arithmetic divides by `n - 1`) and refuses
/// `ConstructError::InvalidInput`.
pub(super) fn parallel_transport(
    initial_normal: Vector3,
    spine: &dyn SpineCurve,
    refinement_level: usize,
    s: f64,
) -> Result<Frame3, ConstructError> {
    if refinement_level < 2 {
        return Err(ConstructError::InvalidInput);
    }
    let tolerance = DirectTolerance::default().position;
    let (s_min, s_max) = spine.domain();

    let mut stations: Vec<f64> = Vec::with_capacity(refinement_level + 1);
    stations.push(s_min);
    for i in 1..refinement_level {
        let station = s_min + (s_max - s_min) * (i as f64) / ((refinement_level - 1) as f64);
        if station <= s {
            stations.push(station);
        } else {
            break;
        }
    }
    if stations[stations.len() - 1] != s {
        stations.push(s);
    }

    let mut positions: Vec<Point3> = Vec::with_capacity(stations.len());
    for &station in &stations {
        positions.push(spine.position_at(station)?);
    }

    let start_tangent = unit_tangent(spine, s_min, tolerance)?;
    if !initial_normal.x.is_finite()
        || !initial_normal.y.is_finite()
        || !initial_normal.z.is_finite()
    {
        return Err(ConstructError::FrameSingular {
            at: s,
            law: "ParallelTransport",
        });
    }
    let mut normal = initial_normal - initial_normal.dot(start_tangent) * start_tangent;
    let residual = normal.magnitude();
    if residual <= tolerance {
        return Err(ConstructError::FrameSingular {
            at: s,
            law: "ParallelTransport",
        });
    }
    normal /= residual;
    let mut binormal = start_tangent.cross(normal);
    let mut tangent = start_tangent;

    for k in 0..(stations.len() - 1) {
        let incoming = positions[k + 1] - positions[k];
        let incoming_len2 = incoming.dot(incoming);
        if incoming_len2 > tolerance {
            let first = 2.0 * normal.dot(incoming) / incoming_len2;
            normal -= first * incoming;

            if k + 1 < stations.len() - 1 {
                let outgoing = positions[k + 2] - positions[k + 1];
                let outgoing_len2 = outgoing.dot(outgoing);
                if outgoing_len2 > tolerance {
                    let bisector =
                        incoming / incoming_len2.sqrt() + outgoing / outgoing_len2.sqrt();
                    let bisector_len2 = bisector.dot(bisector);
                    if bisector_len2 > tolerance {
                        let second = 2.0 * normal.dot(bisector) / bisector_len2;
                        normal -= second * bisector;
                    }
                }
            }
        }

        tangent = unit_tangent(spine, stations[k + 1], tolerance)?;
        normal -= normal.dot(tangent) * tangent;
        let magnitude = normal.magnitude();
        if magnitude <= tolerance {
            return Err(ConstructError::FrameSingular {
                at: s,
                law: "ParallelTransport",
            });
        }
        normal /= magnitude;
        binormal = tangent.cross(normal);
    }

    Ok(Frame3 {
        tangent,
        normal,
        binormal,
    })
}

/// The unit spine tangent `C'(at)/‖C'(at)‖`, refusing `ZeroTangent` when the
/// derivative vanishes within the given position bound.
fn unit_tangent(
    spine: &dyn SpineCurve,
    at: f64,
    tolerance: f64,
) -> Result<Vector3, ConstructError> {
    let derivative = spine.derivative_at(at)?;
    let magnitude = derivative.magnitude();
    if magnitude <= tolerance {
        return Err(ConstructError::ZeroTangent { at });
    }
    Ok(derivative / magnitude)
}
