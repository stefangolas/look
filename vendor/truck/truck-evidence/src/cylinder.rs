//! BG-ENC-002-CYLINDER: `EnclosureSurface` for the `Cylinder` carrier.
//!
//! The cylinder is the carrier where the classic interval-trigonometry bug
//! lives. The parameterisation is
//!
//! ```text
//! S(u, v) = center + r·(cos u, sin u, 0) + (0, 0, v),   u ∈ [0, 2π) periodic,
//! S_u = r·(−sin u, cos u, 0),   S_v = (0, 0, 1),   ‖S_u × S_v‖ = r,
//! ```
//!
//! with `Cylinder::new` refusing `r ≤ 0`, so `r > 0` is an invariant every
//! method here may rely on. The `cos`/`sin` terms have interior extrema at
//! `kπ/2` that endpoint evaluation cannot see; every enclosure therefore
//! evaluates the crate's own certified interval pair
//! `crate::elementary::{cos, sin}` (BG-ENC-005) on the whole cell — never the
//! endpoints only. `v` is affine, so the `z` bound is exact interval
//! arithmetic. `plane.rs` is the reference pattern for structure and tone.

use crate::elementary::{cos, sin};
use crate::enclosure::{Box3, DirCone, EnclosureSurface};
use inari::Interval;
use truck_base::cgmath64::Vector3;
use truck_geometry::specifieds::Cylinder;

/// The semicircle, π radians: the arc width at which the midpoint-direction
/// cone stops being sound.
///
/// An arc of width `w ≤ π` is contained in a cone of half-angle `w/2` around
/// the direction at its midpoint angle. An arc longer than a semicircle —
/// including the full `2π` sweep — covers more than half the horizontal disk,
/// so every cylinder normal is horizontal and the axis-z cone of half-angle
/// `π/2` contains them all. Named, not literal (H-3).
const SEMICIRCLE: f64 = core::f64::consts::PI;

/// A degenerate interval from a runtime `f64`. Finite coordinates always
/// construct; a NaN widens to the empty interval rather than panicking (H-1).
/// The same helper as `plane.rs`, duplicated because `plane.rs` is reference
/// code this packet may not edit.
fn interval_at(x: f64) -> Interval {
    Interval::try_from((x, x)).unwrap_or(Interval::EMPTY)
}

impl EnclosureSurface for Cylinder {
    fn enclose(&self, uu: Interval, vv: Interval) -> Box3 {
        // S = center + r·(cos u, sin u, 0) + (0, 0, v), evaluated componentwise
        // in interval arithmetic, which rounds outward for us. cos/sin are the
        // certified crate pair, so a cell straddling an interior extremum
        // contains it; `r` and the center enter through `interval_at` and the
        // v-coordinate is affine, hence the `z` bound is exact.
        let c = self.center();
        let r = interval_at(self.radius());
        Box3 {
            x: interval_at(c.x) + r * cos(uu),
            y: interval_at(c.y) + r * sin(uu),
            z: interval_at(c.z) + vv,
        }
    }

    fn enclose_der(&self, m: usize, n: usize, uu: Interval, _vv: Interval) -> Box3 {
        // S_u = r·(−sin u, cos u, 0), S_v = (0, 0, 1); every second or higher
        // partial of the parameterisation vanishes identically — the same
        // reasoning as `plane.rs`.
        let r = interval_at(self.radius());
        match (m, n) {
            (1, 0) => Box3 {
                // x = −r·sin u, written as the negated product so the sign is
                // exact (interval negation is exact, as is multiplication by
                // the positive scalar r).
                x: -(r * sin(uu)),
                y: r * cos(uu),
                z: interval_at(0.0),
            },
            (0, 1) => Box3 {
                x: interval_at(0.0),
                y: interval_at(0.0),
                z: interval_at(1.0),
            },
            _ => Box3 {
                x: interval_at(0.0),
                y: interval_at(0.0),
                z: interval_at(0.0),
            },
        }
    }

    fn normal_cone(&self, uu: Interval, _vv: Interval) -> Option<DirCone> {
        // The normal (cos u, sin u, 0) is a unit horizontal vector at direction
        // angle u, so the cone construction is entirely about the arc width.
        // An arc of width w ≤ π fits in a cone of half-angle w/2 around the
        // normal at the midpoint angle; a wider arc leaves the horizontal
        // disk, and the axis-z cone of half-angle π/2 contains every horizontal
        // normal regardless of arc length (sound, not tight — tightness is
        // BG-ENC-004's problem). The immersion never vanishes on a cylinder
        // (r > 0), so there is no singular cell and the cone is always Some.
        let w = uu.sup() - uu.inf();
        if w <= SEMICIRCLE {
            let mid = (uu.inf() + uu.sup()) / 2.0;
            Some(DirCone {
                axis: Vector3::new(mid.cos(), mid.sin(), 0.0),
                half_angle: w / 2.0,
            })
        } else {
            Some(DirCone {
                axis: Vector3::unit_z(),
                half_angle: core::f64::consts::FRAC_PI_2,
            })
        }
    }

    fn immersion_lower_bound(&self, _uu: Interval, _vv: Interval) -> f64 {
        // ‖S_u × S_v‖ = ‖r·(cos u, sin u, 0) × (0, 0, 1)‖ = r exactly, constant
        // over the whole cell.
        self.radius()
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
    use truck_base::cgmath64::{InnerSpace, Point3, Vector3};

    const PI: f64 = core::f64::consts::PI;
    const TAU: f64 = core::f64::consts::TAU;

    fn unit_cylinder() -> Cylinder {
        Cylinder::new(Point3::new(0.0, 0.0, 0.0), 1.0)
            .expect("a finite positive radius is always accepted")
            .value
    }

    fn offset_cylinder() -> Cylinder {
        Cylinder::new(Point3::new(1.0, -2.0, 3.0), 2.5)
            .expect("a finite positive radius is always accepted")
            .value
    }

    /// Build a test interval, degrading to EMPTY (and failing its assertion)
    /// rather than panicking on a malformed bound.
    fn iv(lo: f64, hi: f64) -> Interval {
        Interval::try_from((lo, hi)).unwrap_or(Interval::EMPTY)
    }

    /// DirCone containment of a unit direction `d` by angle:
    /// `cos(angle between axis and d) >= cos(half_angle)`. `half_angle = π/2`
    /// needs the `>=` with a float tolerance to survive rounding.
    fn cone_contains(cone: DirCone, d: Vector3) -> bool {
        let cos_angle = cone.axis.dot(d) / (cone.axis.magnitude() * d.magnitude());
        cos_angle >= cone.half_angle.cos() - 1.0e-12 // H-3: float slack between two direction cosines, not a length
    }

    #[test]
    fn cylinder_encloses_sampled_points() {
        let unit = unit_cylinder();
        let offset = offset_cylinder();
        let cases = [
            // A small arc, well inside one quadrant.
            (unit, iv(0.1, 0.5), iv(-0.5, 0.5)),
            // An arc crossing π/2, the trig-extremum direction.
            (unit, iv(0.4 * PI, 0.6 * PI), iv(0.0, 1.0)),
            // An arc spanning more than π.
            (unit, iv(0.0, 4.0), iv(0.0, 1.0)),
            // A full 2π sweep.
            (unit, iv(0.0, TAU), iv(0.0, 1.0)),
            // A v-range of mixed sign, sweeping through z = 0.
            (offset, iv(0.2, 1.1), iv(-2.0, 1.5)),
        ];
        for (c, uu, vv) in cases {
            assert_encloses_surface(&c, uu, vv, 20);
        }
    }

    #[test]
    fn cylinder_trig_extrema_inside_interval() {
        // The spec's mandated unit test for the interval-trigonometry bug. On a
        // unit cylinder at the origin, uu = [0.4π, 0.6π] gives x = cos uu, whose
        // value at the interior point π/2 is cos(π/2) = 0 > cos(0.6π) ≈ −0.309:
        // an enclosure that evaluated trig only at the endpoints could sit at
        // or below the lower endpoint value, and the interior value would be
        // missed. Stated as relations, not bit-equality.
        let c = unit_cylinder();
        let uu = iv(0.4 * PI, 0.6 * PI);
        let vv = iv(0.0, 1.0);
        let box3 = c.enclose(uu, vv);
        assert!(
            box3.x.sup() >= 0.0,
            "x enclosure {box3:?} must contain cos(π/2) = 0"
        );
        // The endpoint-evaluation's max over the cell is cos(0.6π); the
        // interior bump cos(π/2) = 0 sits strictly above it, so the enclosure's
        // sup must too.
        let endpoint_max = (0.6 * PI).cos();
        assert!(
            box3.x.sup() > endpoint_max,
            "endpoint-only max {endpoint_max} must be strictly below enclosure {}",
            box3.x.sup()
        );
        // The same cell peaks in y = sin uu at the interior point sin(π/2) = 1;
        // the enclosure must reach it as well.
        assert!(
            box3.y.sup() >= 1.0 - 1e-15, // H-3: float slack on a sine bound already in [-1, 1], not a length
            "y enclosure {box3:?} must contain sin(π/2) = 1"
        );
    }

    #[test]
    fn cylinder_enclosure_converges_under_bisection() {
        // BG-ENC-002 convergence from a moderate box: 20 bisections of the
        // wider axis must never widen the enclosure and must shrink it below
        // the initial width.
        let c = unit_cylinder();
        let uu = const_interval!(0.0, 1.0);
        let vv = const_interval!(-1.0, 1.0);
        let initial = c.enclose(uu, vv).width();
        assert_converges(&c, uu, vv, initial, 20);
    }

    #[test]
    fn cylinder_normal_cone_over_arc_and_full_circle() {
        let c = unit_cylinder();
        // Short arc: axis ≈ the normal at the midpoint angle, half-angle ≈ w/2.
        let short = const_interval!(0.2, 0.7);
        let cone = c
            .normal_cone(short, const_interval!(0.0, 1.0))
            .expect("a cylinder always has normals");
        let mid: f64 = (0.2 + 0.7) / 2.0;
        let expected = Vector3::new(mid.cos(), mid.sin(), 0.0);
        assert!(
            (cone.axis - expected).magnitude() < 1.0e-12, // H-3: float slack between two unit direction vectors, not a length
            "axis {:?} != normal at midpoint {:?}",
            cone.axis,
            expected
        );
        assert!(
            (cone.half_angle - 0.25).abs() < 1.0e-12, // H-3: float slack between two half-angles in radians, not a length
            "half_angle {} != w/2",
            cone.half_angle
        );
        // Full sweep: axis z, half-angle π/2, and every sampled normal inside
        // by angle.
        let full = const_interval!(0.0, TAU);
        let cone = c
            .normal_cone(full, const_interval!(0.0, 1.0))
            .expect("a cylinder always has normals");
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
            let u = TAU * (i as f64) / (N as f64 - 1.0);
            let normal = Vector3::new(u.cos(), u.sin(), 0.0);
            assert!(
                cone_contains(cone, normal),
                "normal at u={u} outside the full-circle cone"
            );
        }
    }

    #[test]
    fn cylinder_immersion_lower_bound_is_radius() {
        let c = offset_cylinder();
        // ‖S_u × S_v‖ = r exactly and constant, whatever the cell.
        for (uu, vv) in [
            (const_interval!(0.0, 1.0), const_interval!(0.0, 1.0)),
            (iv(0.4 * PI, 0.6 * PI), iv(-2.0, 2.0)),
            (const_interval!(0.0, TAU), iv(0.0, 5.0)),
        ] {
            assert_eq!(c.immersion_lower_bound(uu, vv), 2.5);
        }
    }

    #[test]
    fn cylinder_der_enclosures_match_partials() {
        let c = unit_cylinder();
        let uu = const_interval!(0.2, 0.9);
        let vv = const_interval!(-1.0, 2.0);
        let du = c.enclose_der(1, 0, uu, vv);
        let dv = c.enclose_der(0, 1, uu, vv);
        const N: usize = 50;
        for i in 0..N {
            for j in 0..N {
                let u = 0.2 + 0.7 * (i as f64) / (N as f64 - 1.0);
                let v = -1.0 + 3.0 * (j as f64) / (N as f64 - 1.0);
                let s_u = Vector3::new(-u.sin(), u.cos(), 0.0);
                assert!(
                    du.contains(Point3::new(s_u.x, s_u.y, s_u.z)),
                    "S_u at ({u},{v}) escaped {du:?}"
                );
                let s_v = Vector3::new(0.0, 0.0, 1.0);
                assert!(
                    dv.contains(Point3::new(s_v.x, s_v.y, s_v.z)),
                    "S_v at ({u},{v}) escaped {dv:?}"
                );
            }
        }
        // Second and higher partials vanish identically, whatever the order.
        for (m, n) in [(2, 0), (0, 2), (1, 1), (3, 0)] {
            let zero = c.enclose_der(m, n, uu, vv);
            assert_eq!(zero.width(), 0.0, "der({m},{n}) must be the zero box");
        }
    }
}
