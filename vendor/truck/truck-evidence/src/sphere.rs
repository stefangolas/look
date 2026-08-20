//! BG-ENC-002-SPHERE: `EnclosureSurface` for the `Sphere` carrier.
//!
//! The parameterisation is
//!
//! ```text
//! S(u, v) = center + r·(sin u·cos v,  sin u·sin v,  cos u),   normal = the same unit vector
//! ```
//!
//! with `u` the polar angle from `+z` and `v` the azimuth. Unlike
//! `Cylinder::new`, `Sphere::new` does not validate the radius, so every
//! method here stays total (H-1): a radius that is `<= 0` or non-finite flows
//! through the interval arithmetic without panicking. `inari` provides no
//! `sin`/`cos` without its `gmp` feature, so the crate's own certified pair
//! (`elementary`) supplies them; they already account for the interior
//! extrema at `kπ/2`, so endpoint-only trig evaluation is never used.

use crate::elementary::{cos, sin};
use crate::enclosure::{Box3, DirCone, EnclosureSurface};
use inari::Interval;
use truck_base::cgmath64::{InnerSpace, Vector3};
use truck_geometry::specifieds::Sphere;

/// A degenerate interval from a runtime `f64`. Finite values always
/// construct; a NaN widens to the empty interval rather than panicking (H-1).
fn interval_at(x: f64) -> Interval {
    Interval::try_from((x, x)).unwrap_or(Interval::EMPTY)
}

/// The unit sphere normal at parameter `(u, v)`, which is also the unit offset
/// of the surface point from the center.
fn unit_normal(u: f64, v: f64) -> Vector3 {
    Vector3::new(u.sin() * v.cos(), u.sin() * v.sin(), u.cos())
}

/// Corner-set convexity threshold. A cone of half-angle below `π/2` is
/// geodesically convex on the sphere, so the patch is the geodesic hull of its
/// corners and the corner-average cone contains the whole patch. At or beyond
/// it the corner-set argument no longer applies and the everything-cone is
/// emitted instead.
const CONVEX_HALF_ANGLE: f64 = core::f64::consts::FRAC_PI_2;

/// A corner-sum shorter than this is treated as (near-)zero: the four corner
/// normals roughly cancel (a `u`-range spanning both poles, say), leaving no
/// meaningful average axis to centre the cone on.
const CORNER_SUM_MIN: f64 = 1.0e-14; // H-3: magnitude of a sum of four unit direction vectors, dimensionless, not a length

/// The cone that contains every direction: axis `+z`, half-angle `π`.
fn everything_cone() -> DirCone {
    DirCone {
        axis: Vector3::unit_z(),
        half_angle: core::f64::consts::PI,
    }
}

impl EnclosureSurface for Sphere {
    fn enclose(&self, uu: Interval, vv: Interval) -> Box3 {
        // S = center + r·(sin u·cos v, sin u·sin v, cos u) evaluated in
        // interval arithmetic. The certified sin/cos account for the interior
        // extrema at kπ/2 and inari's products round outward, so soundness
        // composes; a degenerate radius just flows through the arithmetic.
        let r = interval_at(self.radius());
        let (su, cu) = (sin(uu), cos(uu));
        let (sv, cv) = (sin(vv), cos(vv));
        let c = self.center();
        Box3 {
            x: interval_at(c.x) + r * su * cv,
            y: interval_at(c.y) + r * su * sv,
            z: interval_at(c.z) + r * cu,
        }
    }

    fn enclose_der(&self, m: usize, n: usize, uu: Interval, vv: Interval) -> Box3 {
        // ∂^(m+n) S/∂u^m∂v^n is `r` times the same cyclic derivative table
        // `der_mn` in truck-geometry evaluates: differentiating sin ↦ cos ↦
        // −sin ↦ −cos (and likewise in `v`) cycles every four orders, so the
        // table below is a closed form for every (m, n) and no order needs a
        // whole-space fallback. Each component is a product of one u-trig and
        // one v-trig factor, enclosed over (uu, vv).
        let (su, cu) = (sin(uu), cos(uu));
        let (sv, cv) = (sin(vv), cos(vv));
        let r = interval_at(self.radius());
        // u-derivative factor, one entry per m mod 4: (x = y, z).
        let (ux, uz) = match m % 4 {
            0 => (su, cu),
            1 => (cu, -su),
            2 => (-su, -cu),
            _ => (-cu, su),
        };
        // v-derivative factor, one entry per n mod 4. The z-component of the
        // v-part is 1 only for n = 0 and 0 otherwise (cos u has no
        // v-dependence).
        let vz = if n == 0 {
            interval_at(1.0)
        } else {
            interval_at(0.0)
        };
        let (vx, vy) = match n % 4 {
            0 => (cv, sv),
            1 => (-sv, cv),
            2 => (-cv, -sv),
            _ => (sv, -cv),
        };
        let (cx, cy, cz) = if m == 0 && n == 0 {
            let c = self.center();
            (interval_at(c.x), interval_at(c.y), interval_at(c.z))
        } else {
            (interval_at(0.0), interval_at(0.0), interval_at(0.0))
        };
        Box3 {
            x: cx + r * ux * vx,
            y: cy + r * ux * vy,
            z: cz + r * uz * vz,
        }
    }

    fn normal_cone(&self, uu: Interval, vv: Interval) -> Option<DirCone> {
        // The normal is the unit vector (sin u cos v, sin u sin v, cos u) and
        // does not depend on the radius, so a degenerate radius cannot break
        // this method. Corner rule: the axis is the normalized sum of the four
        // corner directions and the half-angle is the largest corner angle
        // from it. While the half-angle is below π/2 the cone is geodesically
        // convex and the patch is the geodesic hull of its corners, so the
        // cone contains the whole patch. A wider patch, or a cancelling
        // corner set, gets the everything-cone instead: sound, not tight.
        let (u0, u1) = (uu.inf(), uu.sup());
        let (v0, v1) = (vv.inf(), vv.sup());
        let corners = [(u0, v0), (u0, v1), (u1, v0), (u1, v1)];
        let mut sum = Vector3::new(0.0, 0.0, 0.0);
        for &(u, v) in &corners {
            sum += unit_normal(u, v);
        }
        let mag = sum.magnitude();
        if !mag.is_finite() || mag < CORNER_SUM_MIN {
            // Degenerate box (NaN corners) or cancelling corners (e.g. a
            // u-range spanning both poles): no meaningful average axis.
            return Some(everything_cone());
        }
        let axis = sum / mag;
        let mut half_angle: f64 = 0.0;
        for &(u, v) in &corners {
            let dot = unit_normal(u, v).dot(axis).clamp(-1.0, 1.0);
            half_angle = half_angle.max(dot.acos());
        }
        if half_angle < CONVEX_HALF_ANGLE {
            Some(DirCone { axis, half_angle })
        } else {
            // half_angle >= π/2: the corner-set convexity argument does not
            // extend to a patch this wide, so emit the cone that contains
            // every direction. Sound, not tight.
            Some(everything_cone())
        }
    }

    fn immersion_lower_bound(&self, uu: Interval, _vv: Interval) -> f64 {
        // ‖S_u × S_v‖ = r²·sin u, so a lower bound is r²·sin(uu).inf clamped
        // at 0: sin can interval-contain 0 when uu reaches a pole (u = 0 or
        // π), and the honest answer there is exactly 0 — the parameterization
        // is singular at the poles. The outer clamp also maps a degenerate
        // radius (NaN) to the trivial answer 0.0.
        let su = sin(uu).inf().max(0.0);
        (self.radius() * self.radius() * su).max(0.0)
    }
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect on paths reachable from untrusted
// geometry. Unit-test assertions on hand-built witnesses are not such a path.
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::harness::{assert_converges, assert_encloses_surface};
    use inari::const_interval;
    use truck_base::cgmath64::Point3;
    use truck_geotrait::ParametricSurface;

    fn unit_sphere() -> Sphere {
        Sphere::new(Point3::new(0.0, 0.0, 0.0), 1.0)
    }

    #[test]
    fn sphere_encloses_sampled_points() {
        // A non-unit radius and a non-zero center so the enclosure exercises
        // the translation and scaling as well as the unit case.
        let s = Sphere::new(Point3::new(1.0, -2.0, 0.5), 2.5);
        let boxes: &[(Interval, Interval)] = &[
            // small patch off the poles
            (const_interval!(0.3, 0.5), const_interval!(0.4, 0.9)),
            // patch straddling u = π/2 (interior trig extrema)
            (const_interval!(1.4, 1.75), const_interval!(0.1, 0.5)),
            // full-azimuth thin band
            (
                const_interval!(0.6, 0.7),
                const_interval!(0.0, core::f64::consts::TAU),
            ),
            // box containing a pole in its u-range
            (const_interval!(0.0, 0.2), const_interval!(0.3, 1.2)),
            // near-hemisphere
            (const_interval!(0.15, 2.99), const_interval!(0.0, 3.1)),
        ];
        for &(uu, vv) in boxes {
            assert_encloses_surface(&s, uu, vv, 21);
        }
    }

    #[test]
    fn sphere_trig_extrema_inside_interval() {
        // Unit sphere at the origin, uu straddling π/2. The z-enclosure is
        // r·cos(uu); cos(u) = 0 at u = π/2 is attained in the interior of the
        // interval, so the enclosure must contain 0. Assert the relation
        // (contains 0) rather than bit-equality.
        let s = unit_sphere();
        let uu = const_interval!(1.4, 1.75); // π/2 ≈ 1.5708 lies inside
        let vv = const_interval!(0.1, 0.2);
        let box3 = s.enclose(uu, vv);
        assert!(
            box3.z.inf() <= 0.0 && box3.z.sup() >= 0.0,
            "z enclosure {box3:?} must contain cos(pi/2) = 0"
        );
        // The same interior-extremum property for the sin factor in x/y: sin
        // reaches its maximum 1 at the interior u = π/2 while both endpoint
        // sin values are below 1, so the x-enclosure must reach cos(v).
        let c_max = cos(vv).sup();
        assert!(
            box3.x.sup() >= c_max - 1.0e-12, // H-3: float slack between a unit-scaled x bound and a direction cosine, not a length
            "x enclosure must reach the interior sin maximum, got {box3:?}"
        );
    }

    #[test]
    fn sphere_enclosure_converges_under_bisection() {
        let s = Sphere::new(Point3::new(1.0, -1.0, 2.0), 2.0);
        // Pole-free box: the whole enclosure shrinks with the box. (A box
        // whose u-range contains a pole does not converge to zero in u-width
        // for x/y — the parameterization is singular at the pole — so
        // convergence is only asserted pole-free.)
        let uu = const_interval!(0.3, 0.7);
        let vv = const_interval!(0.4, 1.2);
        assert_converges(&s, uu, vv, 4.0, 20);
    }

    #[test]
    fn sphere_normal_cone_over_patch() {
        let s = unit_sphere();
        // Small patch: corner-average axis, small half-angle, and every
        // sampled normal inside the cone.
        let uu = const_interval!(0.3, 0.4);
        let vv = const_interval!(0.6, 0.8);
        let cone = s.normal_cone(uu, vv).expect("small patch has a cone");
        let mut sum = Vector3::new(0.0, 0.0, 0.0);
        for &(u, v) in &[(0.3, 0.6), (0.3, 0.8), (0.4, 0.6), (0.4, 0.8)] {
            sum += unit_normal(u, v);
        }
        let corner_axis = sum / sum.magnitude();
        assert!(
            (cone.axis - corner_axis).magnitude() < 1.0e-12, // H-3: float slack between two unit direction vectors, not a length
            "axis {cone:?} not the corner-average direction"
        );
        assert!(
            cone.half_angle < core::f64::consts::FRAC_PI_2,
            "small patch must be a tight cone, got half_angle {}",
            cone.half_angle
        );
        const GRID: usize = 31;
        for i in 0..GRID {
            for j in 0..GRID {
                let u = 0.3 + 0.1 * (i as f64) / (GRID as f64 - 1.0);
                let v = 0.6 + 0.2 * (j as f64) / (GRID as f64 - 1.0);
                let n = unit_normal(u, v);
                let angle = n.dot(cone.axis).clamp(-1.0, 1.0).acos();
                assert!(
                    angle <= cone.half_angle + 1.0e-12, // H-3: float slack between two angles in radians, not a length
                    "normal at ({u},{v}) escapes the cone by angle {angle}"
                );
            }
        }

        // Wide patch: the everything-cone comes back and still contains every
        // sampled normal (trivially, since its half-angle is π).
        let uu = const_interval!(0.15, 2.99);
        let vv = const_interval!(0.0, core::f64::consts::PI);
        let cone = s.normal_cone(uu, vv).expect("wide patch has a cone");
        assert_eq!(cone.axis, Vector3::unit_z());
        assert_eq!(cone.half_angle, core::f64::consts::PI);
        for i in 0..GRID {
            for j in 0..GRID {
                let u = 0.15 + 2.84 * (i as f64) / (GRID as f64 - 1.0);
                let v = core::f64::consts::PI * (j as f64) / (GRID as f64 - 1.0);
                let n = unit_normal(u, v);
                let angle = n.dot(cone.axis).clamp(-1.0, 1.0).acos();
                assert!(
                    angle <= core::f64::consts::PI,
                    "everything-cone must contain the normal at ({u},{v})"
                );
            }
        }
    }

    #[test]
    fn sphere_immersion_lower_bound_and_poles() {
        let s = Sphere::new(Point3::new(0.0, 0.0, 0.0), 2.0);
        // Pole-free box: the bound equals r²·sin(u_min) up to float slack.
        let uu = const_interval!(0.3, 0.6);
        let vv = const_interval!(0.0, 1.0);
        let lb = s.immersion_lower_bound(uu, vv);
        let expected = 4.0 * 0.3_f64.sin();
        assert!(
            (lb - expected).abs() < 1.0e-12, // H-3: float slack between two immersion-norm lower bounds, dimensionless, not a length
            "bound {lb} != r^2*sin(u_min) = {expected}"
        );
        // Box whose uu touches u = 0: the bound is exactly 0.0 — the
        // parameterization is singular at the pole.
        let uu = const_interval!(0.0, 0.2);
        assert_eq!(s.immersion_lower_bound(uu, vv), 0.0);
    }

    #[test]
    fn sphere_der_enclosures_match_partials() {
        let s = Sphere::new(Point3::new(1.0, 2.0, -1.0), 1.5);
        let uu = const_interval!(0.4, 1.2);
        let vv = const_interval!(0.5, 1.6);
        const GRID: usize = 41;
        for &(m, n) in &[(1, 0), (0, 1), (2, 0)] {
            let box3 = s.enclose_der(m, n, uu, vv);
            for i in 0..GRID {
                for j in 0..GRID {
                    let u = 0.4 + 0.8 * (i as f64) / (GRID as f64 - 1.0);
                    let v = 0.5 + 1.1 * (j as f64) / (GRID as f64 - 1.0);
                    let d: Vector3 = s.der_mn(m, n, u, v);
                    assert!(
                        box3.x.contains(d.x) && box3.y.contains(d.y) && box3.z.contains(d.z),
                        "der({m},{n}) at ({u},{v}) = {d:?} escaped {box3:?}"
                    );
                }
            }
        }
    }
}
