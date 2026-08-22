//! BG-ENC-004-PROCESSOR: `EnclosureSurface` for the `Processor` decorator.
//!
//! `Processor<S, Matrix4>` is the placement decorator — it is how every
//! transformed surface in this kernel arrives. Its enclosure is a
//! *composition*: the inner carrier's `enclose` / `enclose_der` is evaluated
//! on the orientation-resolved parameter box, and the resulting box is mapped
//! through the transform matrix in interval arithmetic. Nothing here evaluates
//! a parameterisation directly.
//!
//! `enclose` maps the inner box through the *homogeneous* interval transform:
//! each output row is `Σ_j m_{i j} · b_j` with the fourth input coordinate the
//! degenerate interval at `1.0`, divided by the transformed `w` row. For an
//! affine map this is exactly as tight as hulling the eight mapped corners —
//! each output coordinate is a linear function in which every input interval
//! appears exactly once, so interval arithmetic has no dependency loss and
//! returns precisely the bounding box of the mapped box. The interval form
//! additionally gets outward rounding for free (BG-ENC-003) and extends to the
//! projective case, which a corner hull does not.
//!
//! `enclose_der` mirrors `Processor`'s own `der_mn`: the orientation swap
//! resolves to `(bm, bn)` and `(au, av)` once, `(0, 0)` returns the point box
//! (the crate's convention — `der_mn(0, 0)` returns `subs(u, v).to_vec()`, so
//! the zeroth enclosure is `enclose`), and every other order applies the
//! *linear* part of the matrix, mirroring `transform_vector` exactly — no `w`
//! column and no `w` divide. For a non-affine bottom row truck's `der_mn` is
//! not the derivative of truck's `subs`; that is a property of the carrier,
//! and this trait's contract is to enclose `der_mn`, so the carrier is
//! mirrored rather than "fixed".
//!
//! `normal_cone` and `immersion_lower_bound` both go through one private
//! helper on the interval cross product: `a = enclose_der(1, 0, ..)` encloses
//! `S_u`, `b = enclose_der(0, 1, ..)` encloses `S_v`, and `n = a × b` is the
//! componentwise interval cross product. This is sound but loose: it encloses
//! `{ p × q : p ∈ a, q ∈ b }`, a superset of
//! `{ S_u(x) × S_v(x) : x ∈ box }`, because it lets `p` and `q` vary
//! independently when in truth they are evaluated at the same point.
//! Over-estimation is always acceptable (BG-ENC-001); tightening is not this
//! packet's job. The cross product and the normal-cone / immersion-bound
//! constructions live once in `crate::enclosure` (`cross_box`,
//! `midpoint_ball_cone`, `immersion_lower_bound_box`) and every BG-ENC-004
//! carrier delegates to them (BG-ENC-004-SHARED-CONE).
//!
//! With the parameters swapped, `enclose_der(1, 0, ..)` already returns an
//! enclosure of the *outer* `S_u`, which is the transformed inner `S_v`, and
//! the cross product gets the reversed normal for free — no manual sign flip.

use crate::enclosure::{
    cross_box, immersion_lower_bound_box, interval_at, midpoint_ball_cone, Box3, DirCone,
    EnclosureSurface,
};
use inari::Interval;
use truck_base::cgmath64::{Matrix4, Vector3};
use truck_geometry::prelude::Processor;

/// The interval cross product of the two first-order derivative enclosures of
/// `s` over the box: a box that encloses every `S_u(x) × S_v(x)` there. Sound
/// but loose — the reasoning is in the module doc.
fn normal_box<S: EnclosureSurface<Vector = Vector3>>(
    s: &Processor<S, Matrix4>,
    uu: Interval,
    vv: Interval,
) -> Box3 {
    let a = s.enclose_der(1, 0, uu, vv);
    let b = s.enclose_der(0, 1, uu, vv);
    cross_box(&a, &b)
}

impl<S: EnclosureSurface<Vector = Vector3>> EnclosureSurface for Processor<S, Matrix4> {
    fn enclose(&self, uu: Interval, vv: Interval) -> Box3 {
        // Trap 1: an inverted processor evaluates `entity.subs(v, u)`, so the
        // parameter box is swapped before the inner carrier is asked. The
        // swap is resolved once here; nothing downstream needs the orientation
        // again.
        let (au, av) = if self.orientation() {
            (uu, vv)
        } else {
            (vv, uu)
        };
        let b = self.entity().enclose(au, av);
        let t = *self.transform();
        // The homogeneous image of the box: output rows 1..4 of T · (x y z 1).
        // `t.x..t.w` are the four columns of the column-major matrix.
        let nx = interval_at(t.x.x) * b.x
            + interval_at(t.y.x) * b.y
            + interval_at(t.z.x) * b.z
            + interval_at(t.w.x);
        let ny = interval_at(t.x.y) * b.x
            + interval_at(t.y.y) * b.y
            + interval_at(t.z.y) * b.z
            + interval_at(t.w.y);
        let nz = interval_at(t.x.z) * b.x
            + interval_at(t.y.z) * b.y
            + interval_at(t.z.z) * b.z
            + interval_at(t.w.z);
        let w = interval_at(t.x.w) * b.x
            + interval_at(t.y.w) * b.y
            + interval_at(t.z.w) * b.z
            + interval_at(t.w.w);
        if w.contains(0.0) || w.is_empty() {
            // A matrix that projects part of the box to infinity. An affine
            // matrix cannot reach this arm: its bottom row is (0, 0, 0, 1), so
            // `w` is the degenerate interval at 1.0 and the division below is
            // exact — the arm is a sound fallback for the projective case, not
            // dead code.
            Box3 {
                x: Interval::ENTIRE,
                y: Interval::ENTIRE,
                z: Interval::ENTIRE,
            }
        } else {
            Box3 {
                x: nx / w,
                y: ny / w,
                z: nz / w,
            }
        }
    }

    fn enclose_der(&self, m: usize, n: usize, uu: Interval, vv: Interval) -> Box3 {
        if (m, n) == (0, 0) {
            // `der_mn(0, 0)` returns `subs(u, v).to_vec()`, a vector whose
            // components equal the point's coordinates, so the zeroth
            // enclosure is the point box. This is the crate's convention
            // (`line.rs` and `cone.rs` document the same choice); `plane.rs`
            // and `cylinder.rs` return the zero box here instead — they are
            // the outliers, and this file does not copy them on this point.
            return self.enclose(uu, vv);
        }
        // Trap 1, both halves: the parameter box *and* the derivative orders
        // swap together.
        let (au, av) = if self.orientation() {
            (uu, vv)
        } else {
            (vv, uu)
        };
        let (bm, bn) = if self.orientation() { (m, n) } else { (n, m) };
        let d = self.entity().enclose_der(bm, bn, au, av);
        let t = *self.transform();
        // The linear part of the matrix applied to the inner derivative box:
        // rows 1..3 of T · (dx dy dz 0), mirroring `transform_vector` — no
        // `t.w` column and no `w` divide.
        Box3 {
            x: interval_at(t.x.x) * d.x + interval_at(t.y.x) * d.y + interval_at(t.z.x) * d.z,
            y: interval_at(t.x.y) * d.x + interval_at(t.y.y) * d.y + interval_at(t.z.y) * d.z,
            z: interval_at(t.x.z) * d.x + interval_at(t.y.z) * d.y + interval_at(t.z.z) * d.z,
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
    use inari::const_interval;
    use truck_base::cgmath64::{InnerSpace, Matrix4, Point3, Rad, Vector3};
    use truck_geometry::specifieds::{Cylinder, Plane, Sphere};
    use truck_geotrait::{Invertible, ParametricSurface};

    const PI: f64 = core::f64::consts::PI;
    const TAU: f64 = core::f64::consts::TAU;

    fn unit_cylinder() -> Cylinder {
        Cylinder::new(Point3::new(0.0, 0.0, 0.0), 1.0)
            .expect("a finite positive radius is always accepted")
            .value
    }

    fn unit_sphere() -> Sphere {
        Sphere::new(Point3::new(0.0, 0.0, 0.0), 1.0)
    }

    /// A plane with different `u` and `v` axes: `subs(u, v) = (2u, v, 0)`, so
    /// it is not symmetric in its parameters and cannot hide an orientation
    /// swap.
    fn asymmetric_plane() -> Plane {
        Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        )
    }

    /// A placement matrix with translation, rotation and non-uniform scale. An
    /// identity or pure-translation matrix would pass a transposed
    /// row/column mistake, so every test uses this combination.
    fn test_matrix() -> Matrix4 {
        let translation = Matrix4::from_translation(Vector3::new(1.0, -2.0, 0.5));
        let rotation = Matrix4::from_axis_angle(Vector3::new(1.0, 1.0, 0.0).normalize(), Rad(0.7));
        let scale = Matrix4::from_nonuniform_scale(2.0, 0.5, 1.5);
        translation * rotation * scale
    }

    /// DirCone containment of a unit direction `d` by angle:
    /// `cos(angle between axis and d) >= cos(half_angle)`. A `half_angle` at
    /// or near `π/2` needs the `>=` with a float tolerance to survive
    /// rounding.
    fn cone_contains(cone: DirCone, d: Vector3) -> bool {
        let cos_angle = cone.axis.dot(d) / (cone.axis.magnitude() * d.magnitude());
        cos_angle >= cone.half_angle.cos() - 1.0e-12 // H-3: float slack between two direction cosines, not a length
    }

    #[test]
    fn processor_encloses_sampled_points() {
        let m = test_matrix();
        let plane = Processor::<_, Matrix4>::with_transform(asymmetric_plane(), m);
        let cylinder = Processor::<_, Matrix4>::with_transform(unit_cylinder(), m);
        let sphere = Processor::<_, Matrix4>::with_transform(unit_sphere(), m);
        // Several boxes over at least two inner carriers, each carrying
        // translation, rotation and non-uniform scale. Negative parameters and
        // arcs crossing π/2 and π are included for the curved carriers.
        // affine carrier, mixed-sign parameters
        assert_encloses_surface(
            &plane,
            const_interval!(-1.0, 2.0),
            const_interval!(-0.5, 1.0),
            21,
        );
        // affine carrier, all-negative parameters
        assert_encloses_surface(
            &plane,
            const_interval!(-2.0, -0.5),
            const_interval!(-1.5, -0.2),
            21,
        );
        // small cylinder arc
        assert_encloses_surface(
            &cylinder,
            const_interval!(0.1, 0.5),
            const_interval!(-1.0, 1.0),
            21,
        );
        // cylinder arc crossing π/2
        assert_encloses_surface(
            &cylinder,
            const_interval!(0.4 * PI, 0.6 * PI),
            const_interval!(0.0, 1.0),
            21,
        );
        // cylinder arc spanning more than π
        assert_encloses_surface(
            &cylinder,
            const_interval!(0.0, 4.0),
            const_interval!(0.0, 1.0),
            21,
        );
        // sphere patch
        assert_encloses_surface(
            &sphere,
            const_interval!(0.3, 0.5),
            const_interval!(0.4, 0.9),
            21,
        );
        // sphere patch straddling u = π/2
        assert_encloses_surface(
            &sphere,
            const_interval!(1.4, 1.75),
            const_interval!(0.1, 0.5),
            21,
        );
    }

    #[test]
    fn processor_inverted_orientation_swaps_the_parameters() {
        let upright = Processor::<_, Matrix4>::with_transform(asymmetric_plane(), test_matrix());
        let inverted = upright.inverse();
        assert!(
            !inverted.orientation(),
            "inverse() must set orientation = false"
        );
        // Asymmetric box: uu != vv, so a swap-blind enclosure misses.
        let uu = const_interval!(0.3, 1.7);
        let vv = const_interval!(-0.6, 0.9);
        // The inverted processor evaluates subs(v, u); its enclosure must
        // contain those sampled points. A swap-blind implementation fails
        // here.
        assert_encloses_surface(&inverted, uu, vv, 21);
        // And positively: the inverted enclosure over (uu, vv) is exactly the
        // upright enclosure over (vv, uu).
        assert_eq!(inverted.enclose(uu, vv), upright.enclose(vv, uu));
    }

    #[test]
    fn processor_der_enclosures_match_partials() {
        let m = test_matrix();
        let uu = const_interval!(0.4, 1.2);
        let vv = const_interval!(0.5, 1.6);
        for inverted in [false, true] {
            let mut processor = Processor::<_, Matrix4>::with_transform(unit_sphere(), m);
            if inverted {
                processor.invert();
            }
            for (mm, nn) in [(0, 0), (1, 0), (0, 1), (2, 0), (1, 1), (0, 2)] {
                let box3 = processor.enclose_der(mm, nn, uu, vv);
                const GRID: usize = 21;
                for i in 0..GRID {
                    for j in 0..GRID {
                        let u = 0.4 + 0.8 * (i as f64) / (GRID as f64 - 1.0);
                        let v = 0.5 + 1.1 * (j as f64) / (GRID as f64 - 1.0);
                        let d: Vector3 = processor.der_mn(mm, nn, u, v);
                        assert!(
                            box3.contains(Point3::new(d.x, d.y, d.z)),
                            "der({mm},{nn}) at ({u},{v}) = {d:?} escaped {box3:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn processor_normal_cone_contains_sampled_normals() {
        let m = test_matrix();
        let upright = Processor::<_, Matrix4>::with_transform(unit_sphere(), m);
        let inverted = upright.inverse();
        let uu = const_interval!(0.55, 0.65);
        let vv = const_interval!(0.6, 0.7);
        // Every sampled unit normal (S_u × S_v).normalize() lies inside the
        // returned cone, by angle, for both orientations. The patch is kept
        // small: the sound-but-loose interval cross product is wide enough
        // under the non-uniform scale that a larger patch's box straddles too
        // many directions for any cone, and `None` is the honest answer.
        for p in [&upright, &inverted] {
            let cone = p
                .normal_cone(uu, vv)
                .expect("a small sphere patch has a cone");
            const GRID: usize = 21;
            for i in 0..GRID {
                for j in 0..GRID {
                    let u = 0.55 + 0.1 * (i as f64) / (GRID as f64 - 1.0);
                    let v = 0.6 + 0.1 * (j as f64) / (GRID as f64 - 1.0);
                    let n = p.uder(u, v).cross(p.vder(u, v)).normalize();
                    assert!(
                        cone_contains(cone, n),
                        "normal at ({u},{v}) escaped the cone"
                    );
                }
            }
        }
        // The two orientations' axes point into opposite half-spaces: the
        // normal reversal falls out of the parameter swap, not a manual sign
        // flip.
        let axis_up = upright
            .normal_cone(uu, vv)
            .expect("upright small patch has a cone")
            .axis;
        let axis_down = inverted
            .normal_cone(uu, vv)
            .expect("inverted small patch has a cone")
            .axis;
        assert!(axis_up.dot(axis_down) < 0.0, "axes must oppose");
        // A full 2π sweep of a transformed cylinder: the normals cover every
        // direction around the axis, so no cone bounds them — the None arm is
        // the contract.
        let cylinder = Processor::<_, Matrix4>::with_transform(unit_cylinder(), m);
        assert!(
            cylinder
                .normal_cone(const_interval!(0.0, TAU), const_interval!(0.0, 1.0))
                .is_none(),
            "a full sweep of normals cannot fit a cone"
        );
    }

    #[test]
    fn processor_immersion_lower_bound_is_a_true_lower_bound() {
        let base = test_matrix();
        // A second matrix with a different non-uniform scale: the "scale the
        // inner bound by one factor" shortcut fails one of these.
        let scaled = Matrix4::from_nonuniform_scale(3.0, 0.25, 2.0) * base;
        let uu = const_interval!(0.5, 1.2);
        let vv = const_interval!(0.3, 1.4);
        for m in [base, scaled] {
            let p = Processor::<_, Matrix4>::with_transform(unit_sphere(), m);
            let lb = p.immersion_lower_bound(uu, vv);
            const GRID: usize = 21;
            for i in 0..GRID {
                for j in 0..GRID {
                    let u = 0.5 + 0.7 * (i as f64) / (GRID as f64 - 1.0);
                    let v = 0.3 + 1.1 * (j as f64) / (GRID as f64 - 1.0);
                    let norm = p.uder(u, v).cross(p.vder(u, v)).magnitude();
                    assert!(
                        lb <= norm,
                        "lower bound {lb} > sampled norm {norm} at ({u},{v})"
                    );
                }
            }
        }
    }

    #[test]
    fn processor_enclosure_converges_under_bisection() {
        let m = test_matrix();
        let p = Processor::<_, Matrix4>::with_transform(unit_cylinder(), m);
        let uu = const_interval!(0.0, 1.0);
        let vv = const_interval!(-1.0, 1.0);
        let initial = p.enclose(uu, vv).width();
        assert_converges(&p, uu, vv, initial, 20);
    }
}
