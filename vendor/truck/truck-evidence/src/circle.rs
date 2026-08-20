//! BG-ENC-002-CIRCLE: `EnclosureCurve` for the `UnitCircle<Point3>` carrier.
//!
//! The purest instance of the interval-trigonometry obligation: the whole
//! carrier is two trig functions,
//!
//! ```text
//! C(t) = (cos t, sin t, 0),   t ∈ [0, 2π) periodic,
//! tangent(t) = (−sin t, cos t, 0),
//! ```
//!
//! with derivatives cycling mod 4. Every method here is exact-form interval
//! evaluation on the carrier — never endpoint-only, never numerical
//! differentiation — using the crate's own certified pair
//! `crate::elementary::{cos, sin}` (BG-ENC-005), which account for the
//! interior extrema at `kπ/2`. `plane.rs` is the reference pattern for tone
//! and for the `interval_at` helper.

use crate::elementary::{cos, sin};
use crate::enclosure::{Box3, DirCone, EnclosureCurve};
use inari::Interval;
use truck_base::cgmath64::{Point3, Vector3};
use truck_geometry::specifieds::UnitCircle;

/// Arc width beyond which the midpoint-tangent cone stops being tight.
///
/// A cone's half-angle is meaningful only up to `π`; once an arc spans more
/// than `π` the tangents cover more than half the horizontal disk and the
/// midpoint construction is no longer valid, so the sound-but-loose horizontal
/// disk cone (`axis = z`, `half_angle = π/2`) takes over. Named, not literal
/// (H-3).
const FULL_ARC_THRESHOLD: f64 = core::f64::consts::PI;

/// A degenerate interval from a runtime `f64`. Finite coordinates always
/// construct; a NaN widens to the empty interval rather than panicking (H-1).
fn interval_at(x: f64) -> Interval {
    Interval::try_from((x, x)).unwrap_or(Interval::EMPTY)
}

impl EnclosureCurve for UnitCircle<Point3> {
    fn enclose(&self, tt: Interval) -> Box3 {
        // C(t) = (cos t, sin t, 0); z is identically zero.
        Box3 {
            x: cos(tt),
            y: sin(tt),
            z: interval_at(0.0),
        }
    }

    fn enclose_der(&self, n: usize, tt: Interval) -> Box3 {
        // der_n cycles mod 4 exactly as the carrier does: (cos, sin),
        // (−sin, cos), (−cos, −sin), (sin, −cos). Each component is the
        // corresponding interval trig call on `tt`, not a derivative of it.
        match n % 4 {
            0 => Box3 {
                x: cos(tt),
                y: sin(tt),
                z: interval_at(0.0),
            },
            1 => Box3 {
                x: -sin(tt),
                y: cos(tt),
                z: interval_at(0.0),
            },
            2 => Box3 {
                x: -cos(tt),
                y: -sin(tt),
                z: interval_at(0.0),
            },
            _ => Box3 {
                x: sin(tt),
                y: -cos(tt),
                z: interval_at(0.0),
            },
        }
    }

    fn tangent_cone(&self, tt: Interval) -> Option<DirCone> {
        // Same arc rule as a cylinder's normal cone, rotated a quarter turn:
        // tangent(θ) = (−sin θ, cos θ, 0) is a unit horizontal vector whose
        // direction angle is θ + π/2. Over an arc of width w ≤ π the tangents
        // are within w/2 of the tangent at the midpoint angle; over a wider
        // arc they cover more than half the horizontal disk, so every tangent
        // is horizontal and the axis-z disk cone contains them all (sound, not
        // tight). The derivative never vanishes on a unit circle.
        let w = tt.sup() - tt.inf();
        if w <= FULL_ARC_THRESHOLD {
            let mid = (tt.inf() + tt.sup()) / 2.0;
            Some(DirCone {
                axis: Vector3::new(-mid.sin(), mid.cos(), 0.0),
                half_angle: w / 2.0,
            })
        } else {
            Some(DirCone {
                axis: Vector3::unit_z(),
                half_angle: core::f64::consts::FRAC_PI_2,
            })
        }
    }
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect on paths reachable from untrusted
// geometry. Unit-test assertions on hand-built witnesses are not such a path.
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::harness::assert_encloses_curve;
    use truck_base::cgmath64::InnerSpace;

    const PI: f64 = core::f64::consts::PI;
    const TAU: f64 = core::f64::consts::TAU;

    /// Build a test interval, degrading to EMPTY (and failing its assertion)
    /// rather than panicking on a malformed bound.
    fn iv(lo: f64, hi: f64) -> Interval {
        Interval::try_from((lo, hi)).unwrap_or(Interval::EMPTY)
    }

    #[test]
    fn circle_encloses_sampled_points() {
        let c = UnitCircle::<Point3>::new();
        // A short arc, an arc straddling π/2, one straddling π, an arc wider
        // than π, and a full 2π sweep — the family of boxes where the interior
        // extrema of sin/cos are the point (BG-ENC-001 soundness, sampled).
        let cases = [
            iv(0.1, 0.5),
            iv(0.4 * PI, 0.6 * PI),
            iv(0.8 * PI, 1.2 * PI),
            iv(0.0, 4.0),
            iv(0.0, TAU),
        ];
        for tt in cases {
            assert_encloses_curve(&c, tt, 50);
        }
    }

    #[test]
    fn circle_trig_extrema_inside_interval() {
        let c = UnitCircle::<Point3>::new();
        // [0.4π, 0.6π] straddles π/2 where sin peaks at 1 in the interior.
        // Endpoint-only evaluation returns at most sin(0.6π) ≈ 0.951 and is the
        // historic under-estimation bug; the relation below is the point.
        let tt = iv(0.4 * PI, 0.6 * PI);
        let box3 = c.enclose(tt);
        assert!(
            box3.y.sup() >= 1.0 - 1e-15, // H-3: float slack on a sine bound already in [-1, 1], not a length
            "y enclosure {box3:?} must contain sin(π/2) = 1"
        );
        let endpoint_max = (0.6 * PI).sin();
        assert!(
            box3.y.sup() > endpoint_max,
            "endpoint-only max {endpoint_max} must be strictly below enclosure {}",
            box3.y.sup()
        );
    }

    #[test]
    fn circle_enclosure_converges_under_bisection() {
        let c = UnitCircle::<Point3>::new();
        // Curve version of `assert_converges` (that one is surface-only):
        // halving the box must never widen the enclosure and the width must
        // drop below the initial width (BG-ENC-002 convergence).
        let mut tt = iv(0.0, 0.3 * PI);
        let initial = c.enclose(tt).width();
        let mut prev = initial;
        for _ in 0..20 {
            let mid = (tt.inf() + tt.sup()) / 2.0;
            tt = iv(tt.inf(), mid);
            let cur = c.enclose(tt).width();
            assert!(
                cur <= prev,
                "enclosure widened under bisection: {prev} -> {cur}"
            );
            prev = cur;
        }
        assert!(
            prev < initial,
            "enclosure did not converge below initial width {initial}: {prev}"
        );
    }

    #[test]
    fn circle_tangent_cone_over_arc_and_full_circle() {
        let c = UnitCircle::<Point3>::new();
        // Short arc: axis is the tangent at the midpoint angle, half-angle w/2.
        let short = iv(0.2, 0.7);
        let cone = c
            .tangent_cone(short)
            .expect("unit circle has no vanishing tangent");
        let mid: f64 = (0.2 + 0.7) / 2.0;
        let expected = Vector3::new(-mid.sin(), mid.cos(), 0.0);
        assert!(
            (cone.axis - expected).magnitude() < 1.0e-12, // H-3: float slack between two unit direction vectors, not a length
            "axis {:?} != tangent at midpoint {:?}",
            cone.axis,
            expected
        );
        assert!(
            (cone.half_angle - 0.25).abs() < 1.0e-12, // H-3: float slack between two half-angles in radians, not a length
            "half_angle {} != w/2",
            cone.half_angle
        );
        // Full sweep: axis z, half-angle π/2, and every sampled tangent inside
        // by angle — cos(angle) ≥ cos(half_angle) with a float tolerance.
        let full = iv(0.0, TAU);
        let cone = c
            .tangent_cone(full)
            .expect("unit circle has no vanishing tangent");
        assert!(
            (cone.axis - Vector3::unit_z()).magnitude() < 1.0e-12, // H-3: float slack between two unit direction vectors, not a length
            "axis {:?} != z",
            cone.axis
        );
        assert!(
            (cone.half_angle - core::f64::consts::FRAC_PI_2).abs() < 1.0e-12, // H-3: float slack between two half-angles in radians, not a length
            "half_angle {} != π/2",
            cone.half_angle
        );
        const N: usize = 64;
        for i in 0..N {
            let t = TAU * (i as f64) / (N as f64 - 1.0);
            let tan = Vector3::new(-t.sin(), t.cos(), 0.0);
            let cos_angle = cone.axis.dot(tan);
            assert!(
                cos_angle >= cone.half_angle.cos() - 1.0e-12, // H-3: float slack between two direction cosines, not a length
                "tangent at t={t} outside the cone: cos(angle)={cos_angle}"
            );
        }
    }

    #[test]
    fn circle_der_enclosures_cycle_mod_four() {
        let c = UnitCircle::<Point3>::new();
        let tt = iv(0.2, 0.9);
        // Order 0 is the same construction as `enclose`, so it matches exactly.
        assert_eq!(c.enclose_der(0, tt), c.enclose(tt));
        // Order 1 must contain sampled tangents (−sin t, cos t, 0).
        let d1 = c.enclose_der(1, tt);
        const N: usize = 50;
        for i in 0..N {
            let t = 0.2 + 0.7 * (i as f64) / (N as f64 - 1.0);
            let tan = Vector3::new(-t.sin(), t.cos(), 0.0);
            assert!(
                d1.contains(Point3::new(tan.x, tan.y, tan.z)),
                "der tangent at t={t} escaped {d1:?}"
            );
        }
        // Period 4 on a comparable-width box: der_n wraps mod 4.
        assert_eq!(c.enclose_der(4, tt), c.enclose_der(0, tt));
        assert_eq!(c.enclose_der(5, tt), c.enclose_der(1, tt));
    }
}
