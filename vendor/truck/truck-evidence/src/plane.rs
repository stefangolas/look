//! BG-ENC-002 reference: `EnclosureSurface` for `Plane`.
//!
//! This is the item the build spec (P-6) uses as the reference pattern for
//! every later carrier impl. A plane is affine:
//!
//! ```text
//! S(u, v) = o + u·(p − o) + v·(q − o)
//! ```
//!
//! so the enclosure over a box is exact interval arithmetic on the
//! parameterisation (no subdivision needed), the normal is constant, and the
//! immersion is constant. Every method here is closed-form; the certificate is
//! `μ = Exact` because no rounding enters the affine image bounds.

use crate::enclosure::{Box3, DirCone, EnclosureSurface};
use inari::Interval;
use truck_base::cgmath64::InnerSpace;
use truck_geometry::specifieds::Plane;

/// A degenerate interval from a runtime `f64`. Finite coordinates always
/// construct; a NaN widens to the empty interval rather than panicking (H-1).
fn interval_at(x: f64) -> Interval {
    Interval::try_from((x, x)).unwrap_or(Interval::EMPTY)
}

impl EnclosureSurface for Plane {
    fn as_plane(&self) -> Option<&Plane> {
        Some(self)
    }

    fn enclose(&self, uu: Interval, vv: Interval) -> Box3 {
        // S = o + u·a + v·b with a = p−o, b = q−o. Each coordinate is
        // o_c + uu·a_c + vv·b_c in interval arithmetic: affine, hence exact.
        let o = self.origin();
        let a = self.u_axis();
        let b = self.v_axis();
        let x = interval_at(o.x) + uu * interval_at(a.x) + vv * interval_at(b.x);
        let y = interval_at(o.y) + uu * interval_at(a.y) + vv * interval_at(b.y);
        let z = interval_at(o.z) + uu * interval_at(a.z) + vv * interval_at(b.z);
        Box3 { x, y, z }
    }

    fn enclose_der(&self, m: usize, n: usize, _uu: Interval, _vv: Interval) -> Box3 {
        // ∂S/∂u = a, ∂S/∂v = b, all higher derivatives are zero.
        if m == 1 && n == 0 {
            let a = self.u_axis();
            Box3 {
                x: interval_at(a.x),
                y: interval_at(a.y),
                z: interval_at(a.z),
            }
        } else if m == 0 && n == 1 {
            let b = self.v_axis();
            Box3 {
                x: interval_at(b.x),
                y: interval_at(b.y),
                z: interval_at(b.z),
            }
        } else {
            // Higher derivatives vanish identically on an affine surface.
            Box3 {
                x: interval_at(0.0),
                y: interval_at(0.0),
                z: interval_at(0.0),
            }
        }
    }

    fn normal_cone(&self, _uu: Interval, _vv: Interval) -> Option<DirCone> {
        // The normal is constant and unit: n = (a × b)/‖a × b‖. A degenerate
        // plane (a × b = 0) has no well-defined normal direction. Test the raw
        // cross product: normalizing a zero vector yields NaN, so the check
        // must run before `.normalize()`.
        let a = self.u_axis();
        let b = self.v_axis();
        let cross = a.cross(b);
        if cross.magnitude() == 0.0 {
            None
        } else {
            let n = cross.normalize();
            Some(DirCone {
                axis: n,
                half_angle: 0.0,
            })
        }
    }

    fn immersion_lower_bound(&self, _uu: Interval, _vv: Interval) -> f64 {
        // ‖S_u × S_v‖ is constant = ‖a × b‖.
        self.u_axis().cross(self.v_axis()).magnitude()
    }
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect on paths reachable from untrusted
// geometry. Unit-test assertions on hand-built witnesses are not such a path.
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::enclosure::EnclosureCurve;
    use inari::const_interval;
    use truck_base::cgmath64::{Point3, Vector3};
    use truck_geotrait::ParametricSurface;

    fn xy_plane() -> Plane {
        Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        )
    }

    #[test]
    fn xy_encloses_sampled_points() {
        let p = xy_plane();
        let uu = const_interval!(-1.0, 2.0);
        let vv = const_interval!(0.5, 3.0);
        let box3 = p.enclose(uu, vv);
        // Sample a dense grid and require every point to be contained
        // (direct BG-ENC-001 check).
        let n = 50;
        for i in 0..n {
            for j in 0..n {
                let u = -1.0 + 3.0 * (i as f64) / (n as f64 - 1.0);
                let v = 0.5 + 2.5 * (j as f64) / (n as f64 - 1.0);
                let pt = p.subs(u, v);
                assert!(
                    box3.contains(pt),
                    "point ({u},{v}) -> {pt:?} escaped {box3:?}"
                );
            }
        }
    }

    #[test]
    fn xy_enclose_is_tight() {
        let p = xy_plane();
        let uu = const_interval!(-1.0, 2.0);
        let vv = const_interval!(0.5, 3.0);
        let box3 = p.enclose(uu, vv);
        // Affine → the enclosure is exactly [min, max] per coordinate.
        assert_eq!(box3.x.inf(), -1.0);
        assert_eq!(box3.x.sup(), 2.0);
        assert_eq!(box3.y.inf(), 0.5);
        assert_eq!(box3.y.sup(), 3.0);
        assert_eq!(box3.z.inf(), 0.0);
        assert_eq!(box3.z.sup(), 0.0);
    }

    #[test]
    fn der_enclosures_match_partials() {
        let p = xy_plane();
        let uu = const_interval!(0.0, 1.0);
        let vv = const_interval!(0.0, 1.0);
        let du = p.enclose_der(1, 0, uu, vv);
        assert_eq!(du.x.inf(), 1.0);
        assert_eq!(du.y.inf(), 0.0);
        let dv = p.enclose_der(0, 1, uu, vv);
        assert_eq!(dv.x.inf(), 0.0);
        assert_eq!(dv.y.inf(), 1.0);
        let zero = p.enclose_der(2, 0, uu, vv);
        assert_eq!(zero.width(), 0.0);
    }

    #[test]
    fn normal_cone_is_constant_axis() {
        let p = xy_plane();
        let cone = p
            .normal_cone(const_interval!(0.0, 1.0), const_interval!(0.0, 1.0))
            .expect("non-degenerate plane");
        assert_eq!(cone.half_angle, 0.0);
        assert!((cone.axis - Vector3::unit_z()).magnitude() < 1.0e-12);
    }

    #[test]
    fn immersion_lower_bound_is_cross_norm() {
        let p = xy_plane();
        let lb = p.immersion_lower_bound(const_interval!(0.0, 1.0), const_interval!(0.0, 1.0));
        // ‖a × b‖ = 1 for the xy plane.
        assert!((lb - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn degenerate_plane_has_no_normal_cone() {
        // Three collinear points make a zero-area "plane".
        let p = Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
        );
        assert!(p
            .normal_cone(const_interval!(0.0, 1.0), const_interval!(0.0, 1.0))
            .is_none());
        assert_eq!(
            p.immersion_lower_bound(const_interval!(0.0, 1.0), const_interval!(0.0, 1.0)),
            0.0
        );
    }

    // Keep the EnclosureCurve import alive (the harness is generic over it).
    #[allow(dead_code)]
    fn _unused(_: &impl EnclosureCurve) {}
}
