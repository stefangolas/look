//! BG-ENC-004-EXTRUDED: `EnclosureSurface` for the `ExtrudedCurve` decorator.
//!
//! `ExtrudedCurve<C, Vector3>` sweeps an inner curve `C` along a constant
//! vector,
//!
//! ```text
//! S(u, v) = C(u) + v·V,
//! ```
//!
//! so this is the first *compositional* carrier: the enclosure calls the inner
//! curve's `enclose` / `enclose_der` and combines the resulting boxes
//! (BG-ENC-004) instead of evaluating a parameterisation directly. `v` is not
//! clamped — `subs` accepts any `v` — so `enclose` must be correct for any
//! `vv`, including negative and mixed-sign intervals. inari's interval
//! arithmetic rounds outward and handles mixed-sign multiplication, which is
//! all the arithmetic this impl needs.
//!
//! The singular locus is the point of this packet. `S_u × S_v = C'(u) × V`
//! vanishes exactly where the curve's tangent is parallel (or antiparallel) to
//! the extrusion vector: a line extruded along its own direction is a
//! degenerate strip, not a plane. That is the `None` case of `normal_cone` and
//! the `0.0` case of `immersion_lower_bound`. `plane.rs` and `line.rs` are the
//! reference pattern for structure and doc tone.

use crate::enclosure::{
    cross_box, immersion_lower_bound_box, interval_at, midpoint_ball_cone, Box3, DirCone,
    EnclosureCurve, EnclosureSurface,
};
use inari::Interval;
use truck_base::cgmath64::Vector3;
use truck_geometry::decorators::ExtrudedCurve;

/// An enclosure of `{ S_u(u, v) × S_v(u, v) : u ∈ uu, v ∈ vv }` for the
/// extruded surface: the interval cross product of the two derivative boxes.
/// `normal_cone` and `immersion_lower_bound` both go through this one private
/// helper, which is the construction the whole BG-ENC-004 family shares.
fn normal_box<C: EnclosureCurve<Vector = Vector3>>(
    surface: &ExtrudedCurve<C, Vector3>,
    uu: Interval,
    vv: Interval,
) -> Box3 {
    let a = surface.enclose_der(1, 0, uu, vv); // encloses S_u
    let b = surface.enclose_der(0, 1, uu, vv); // encloses S_v
    cross_box(&a, &b)
}

impl<C: EnclosureCurve<Vector = Vector3>> EnclosureSurface for ExtrudedCurve<C, Vector3> {
    fn enclose(&self, uu: Interval, vv: Interval) -> Box3 {
        // S = C(u) + v·V: the inner curve's box shifted by vv·V, componentwise
        // and entirely in inari arithmetic, which rounds outward for us. `vv`
        // is signed; inari handles mixed-sign multiplication correctly, so do
        // not hand-roll a sign case analysis.
        let c = self.entity_curve().enclose(uu);
        let v = self.extruding_vector();
        Box3 {
            x: c.x + vv * interval_at(v.x),
            y: c.y + vv * interval_at(v.y),
            z: c.z + vv * interval_at(v.z),
        }
    }

    fn enclose_der(&self, m: usize, n: usize, uu: Interval, vv: Interval) -> Box3 {
        // Mirror `ExtrudedCurve::der_mn` exactly (truck-geometry
        // `decorators/extruded_curve.rs`): (0, 0) is subs(u, v).to_vec() — a
        // vector whose components equal the point's coordinates, so the zeroth
        // enclosure is the point box, the crate convention (match the carrier,
        // do not "fix" it); (0, 1) is the constant extrusion vector; (m, 0)
        // delegates to the inner curve; everything else vanishes because S is
        // affine in v.
        match (m, n) {
            (0, 0) => self.enclose(uu, vv),
            (0, 1) => {
                let v = self.extruding_vector();
                Box3 {
                    x: interval_at(v.x),
                    y: interval_at(v.y),
                    z: interval_at(v.z),
                }
            }
            (_, 0) => self.entity_curve().enclose_der(m, uu),
            _ => Box3 {
                x: interval_at(0.0),
                y: interval_at(0.0),
                z: interval_at(0.0),
            },
        }
    }

    fn normal_cone(&self, uu: Interval, vv: Interval) -> Option<DirCone> {
        // The shared midpoint-ball cone off the cross-product box; the
        // construction (rounding directions, refusal condition, ulp nudge and
        // clamp) lives in `crate::enclosure::midpoint_ball_cone`.
        midpoint_ball_cone(&normal_box(self, uu, vv))
    }

    fn immersion_lower_bound(&self, uu: Interval, vv: Interval) -> f64 {
        // The shared mignitude-immersion lower bound off the cross-product box
        // (`crate::enclosure::immersion_lower_bound_box`).
        immersion_lower_bound_box(&normal_box(self, uu, vv))
    }
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect on paths reachable from untrusted
// geometry. Unit-test assertions on hand-built witnesses are not such a path.
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::harness::{assert_converges, assert_encloses_surface};
    use truck_base::cgmath64::{InnerSpace, Point3};
    use truck_geometry::specifieds::{Line, UnitCircle};
    use truck_geotrait::ParametricSurface;

    const PI: f64 = core::f64::consts::PI;
    const TAU: f64 = core::f64::consts::TAU;

    /// Build a test interval, degrading to EMPTY (and failing its assertion)
    /// rather than panicking on a malformed bound.
    fn iv(lo: f64, hi: f64) -> Interval {
        Interval::try_from((lo, hi)).unwrap_or(Interval::EMPTY)
    }

    /// The cylinder: the unit circle `(cos t, sin t, 0)` extruded along z.
    fn extruded_circle() -> ExtrudedCurve<UnitCircle<Point3>, Vector3> {
        ExtrudedCurve::by_extrusion(UnitCircle::<Point3>::new(), Vector3::unit_z())
    }

    /// A plane patch: an oblique line extruded along a non-parallel vector.
    fn extruded_line() -> ExtrudedCurve<Line<Point3>, Vector3> {
        let p = Point3::new(-1.0, 0.5, 2.0);
        let q = Point3::new(2.0, -3.0, 1.0);
        ExtrudedCurve::by_extrusion(Line(p, q), Vector3::new(0.0, 1.0, 0.5))
    }

    /// Cone containment by angle: cos(angle between axis and d) >=
    /// cos(half_angle). A half_angle at or near π/2 needs the `>=` with float
    /// tolerance to survive rounding, so the slack lives here in the test,
    /// never in the cone.
    fn cone_contains(cone: &DirCone, d: Vector3) -> bool {
        let cos_angle = cone.axis.dot(d.normalize());
        cos_angle >= cone.half_angle.cos() - 1.0e-12 // H-3: float slack between two direction cosines, not a length
    }

    /// The sampled ‖S_u × S_v‖ at one grid point of `s`.
    fn sampled_immersion_norm<C: EnclosureCurve<Vector = Vector3>>(
        s: &ExtrudedCurve<C, Vector3>,
        u: f64,
        w: f64,
    ) -> f64 {
        s.der_mn(1, 0, u, w).cross(s.der_mn(0, 1, u, w)).magnitude()
    }

    #[test]
    fn extruded_encloses_sampled_points() {
        let circle = extruded_circle();
        let line = extruded_line();
        // The uu/vv family from the decision: a small arc, an arc crossing
        // π/2, one spanning more than π, a full 2π sweep, a vv entirely
        // negative, and a vv straddling zero.
        let cases = [
            (iv(0.1, 0.5), iv(0.0, 1.0)),
            (iv(0.4 * PI, 0.6 * PI), iv(0.0, 1.0)),
            (iv(0.0, 4.0), iv(0.0, 1.0)),
            (iv(0.0, TAU), iv(0.0, 1.0)),
            (iv(0.2, 1.0), iv(-3.0, -1.0)),
            (iv(0.2, 1.0), iv(-2.0, 0.5)),
        ];
        for (uu, vv) in cases {
            assert_encloses_surface(&circle, uu, vv, 20);
            assert_encloses_surface(&line, uu, vv, 20);
        }
    }

    /// For a generic extruded surface: every sampled `der_mn` lies in the
    /// corresponding enclosure; (0, 1) is exactly the degenerate box at the
    /// extrusion vector; and (1, 1) / (0, 2) are the zero box because S is
    /// affine in v.
    fn assert_der_enclosures<C: EnclosureCurve<Vector = Vector3>>(
        s: &ExtrudedCurve<C, Vector3>,
        uu: Interval,
        vv: Interval,
    ) {
        const N: usize = 20;
        let zero = Box3 {
            x: interval_at(0.0),
            y: interval_at(0.0),
            z: interval_at(0.0),
        };
        let e00 = s.enclose_der(0, 0, uu, vv);
        let e10 = s.enclose_der(1, 0, uu, vv);
        let e01 = s.enclose_der(0, 1, uu, vv);
        let e20 = s.enclose_der(2, 0, uu, vv);
        for i in 0..N {
            let u = uu.inf() + (uu.sup() - uu.inf()) * (i as f64) / (N as f64 - 1.0);
            for j in 0..N {
                let w = vv.inf() + (vv.sup() - vv.inf()) * (j as f64) / (N as f64 - 1.0);
                for (m, n, enclosure) in [
                    (0usize, 0usize, &e00),
                    (1, 0, &e10),
                    (0, 1, &e01),
                    (2, 0, &e20),
                ] {
                    let d = s.der_mn(m, n, u, w);
                    assert!(
                        enclosure.contains(Point3::new(d.x, d.y, d.z)),
                        "der_{m}{n} at ({u},{w}) escaped its enclosure"
                    );
                }
                // (1, 1) and (0, 2) vanish identically on the affine-in-v
                // surface.
                let duv = s.der_mn(1, 1, u, w);
                assert!(zero.contains(Point3::new(duv.x, duv.y, duv.z)));
                let dvv = s.der_mn(0, 2, u, w);
                assert!(zero.contains(Point3::new(dvv.x, dvv.y, dvv.z)));
            }
        }
        // (0, 1) is exactly the degenerate box at the extrusion vector.
        let v = s.extruding_vector();
        assert_eq!(e01.width(), 0.0);
        assert_eq!(e01.x.inf(), v.x);
        assert_eq!(e01.y.inf(), v.y);
        assert_eq!(e01.z.inf(), v.z);
        // (1, 1) and (0, 2) are the zero box.
        assert_eq!(s.enclose_der(1, 1, uu, vv), zero);
        assert_eq!(s.enclose_der(0, 2, uu, vv), zero);
    }

    #[test]
    fn extruded_der_enclosures_match_partials() {
        let circle = extruded_circle();
        let line = extruded_line();
        let uu = iv(0.2, 0.9);
        let vv = iv(-0.5, 1.5);
        assert_der_enclosures(&circle, uu, vv);
        assert_der_enclosures(&line, uu, vv);
    }

    #[test]
    fn extruded_normal_cone_contains_sampled_normals() {
        let s = extruded_circle();
        let uu = iv(0.2, 0.7);
        let vv = iv(-1.0, 2.0);
        let cone = s
            .normal_cone(uu, vv)
            .expect("a moderate arc of the cylinder is an immersion");
        const N: usize = 40;
        for i in 0..N {
            let u = uu.inf() + (uu.sup() - uu.inf()) * (i as f64) / (N as f64 - 1.0);
            for j in 0..N {
                let w = vv.inf() + (vv.sup() - vv.inf()) * (j as f64) / (N as f64 - 1.0);
                let n = s.der_mn(1, 0, u, w).cross(s.der_mn(0, 1, u, w)).normalize();
                assert!(
                    cone_contains(&cone, n),
                    "unit normal at ({u},{w}) outside the cone"
                );
            }
        }
    }

    #[test]
    fn extruded_normal_cone_refuses_when_the_tangent_meets_the_extrusion() {
        // A line extruded along its own direction: S_u × S_v ≡ 0 over the whole
        // surface, the singular strip of the packet. No cone bounds the
        // undefined normals.
        let p = Point3::new(1.0, 2.0, 3.0);
        let q = Point3::new(2.0, 3.0, 4.0);
        let degenerate = ExtrudedCurve::by_extrusion(Line(p, q), q - p);
        assert!(degenerate
            .normal_cone(iv(-1.0, 2.0), iv(0.0, 1.0))
            .is_none());
        // A full 2π sweep of the extruded circle: the normals cover every
        // horizontal direction, so no cone bounds them either.
        let circle = extruded_circle();
        assert!(circle.normal_cone(iv(0.0, TAU), iv(0.0, 1.0)).is_none());
        // A moderate arc bounded away from the singular locus has a cone.
        assert!(circle.normal_cone(iv(0.2, 0.7), iv(0.0, 1.0)).is_some());
    }

    #[test]
    fn extruded_immersion_lower_bound_is_a_true_lower_bound() {
        let circle = extruded_circle();
        let line = extruded_line();
        let cells = [
            (iv(0.1, 0.5), iv(0.0, 1.0)),
            (iv(0.2, 0.7), iv(-1.0, 2.0)),
            (iv(0.4 * PI, 0.6 * PI), iv(0.0, 1.0)),
        ];
        for (uu, vv) in cells {
            // Both witnesses are immersions over these cells: the bound must be
            // positive and must never exceed a sampled ‖S_u × S_v‖.
            let lb_circle = circle.immersion_lower_bound(uu, vv);
            let lb_line = line.immersion_lower_bound(uu, vv);
            assert!(lb_circle > 0.0, "cylinder cell is an immersion");
            assert!(lb_line > 0.0, "plane-patch cell is an immersion");
            const N: usize = 20;
            for i in 0..N {
                let u = uu.inf() + (uu.sup() - uu.inf()) * (i as f64) / (N as f64 - 1.0);
                for j in 0..N {
                    let w = vv.inf() + (vv.sup() - vv.inf()) * (j as f64) / (N as f64 - 1.0);
                    assert!(
                        lb_circle <= sampled_immersion_norm(&circle, u, w),
                        "lower bound {lb_circle} above sampled norm at ({u},{w})"
                    );
                    assert!(
                        lb_line <= sampled_immersion_norm(&line, u, w),
                        "lower bound {lb_line} above sampled norm at ({u},{w})"
                    );
                }
            }
        }
        // The singular strip: the lower bound is exactly 0.0.
        let p = Point3::new(1.0, 2.0, 3.0);
        let q = Point3::new(2.0, 3.0, 4.0);
        let degenerate = ExtrudedCurve::by_extrusion(Line(p, q), q - p);
        assert_eq!(
            degenerate.immersion_lower_bound(iv(-1.0, 2.0), iv(0.0, 1.0)),
            0.0
        );
    }

    #[test]
    fn extruded_enclosure_converges_under_bisection() {
        let s = extruded_circle();
        let uu = iv(0.0, 0.8);
        let vv = iv(0.0, 1.0);
        let initial = s.enclose(uu, vv).width();
        assert_converges(&s, uu, vv, initial, 20);
    }
}
