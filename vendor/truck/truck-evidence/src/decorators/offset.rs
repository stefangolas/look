//! BG-ENC-004-OFFSET: `EnclosureSurface` for the `Offset` decorator, by
//! composition over two new vector/scalar field traits.
//!
//! `Offset<S, N>` is the pointwise sum `S(u, v) + N(u, v)` of two parametric
//! surfaces, and truck-geometry's only `ParametricSurface` impl for it requires
//! `N: ParametricSurface<Point = C::Vector>` — the offset field is
//! *vector*-valued. `EnclosureSurface` is bounded `ParametricSurface<Point =
//! Point3>`, so the canonical field `NormalField<S, F>` (`Point = Vector3`) can
//! never be an `EnclosureSurface`: the naive `impl EnclosureSurface for
//! Offset<S, N>` does not typecheck for any choice of the two. That is a type
//! error, not a curvature bound, and it is why this packet is a design item.
//!
//! The decided resolution is composition over new interface surface
//! (BG-ENC-004-OFFSET): two new traits in `enclosure.rs` — `EnclosureVectorField`
//! for vector-valued fields (it is `EnclosureSurface` minus the `Point3`
//! bound) and `EnclosureScalarField2` for their scalar factor — and this module
//! supplies the impls: the constant `f64` scalar, the `NormalField` unit-normal
//! field, and the `Offset` sum. `N` is never an `EnclosureSurface`.
//!
//! **The `NormalField` field.** `NormalField<S, F>::subs = S.normal(u, v) ·
//! F.subs(u, v)` with `normal = (S_u × S_v)/‖S_u × S_v‖`. Its position box is
//! the base's cross-product box scaled by `[0, 1/L]`, where `L` is the base's
//! *own* certified immersion margin, because `1/‖S_u × S_v‖ ∈ (0, 1/L]`; when
//! `L = 0` (a singular locus in the box) the unit-normal position still exists
//! — a unit vector never leaves the unit ball — so the `[-1, 1]³` fallback is
//! always sound. Its first partials use the projection form of the quotient
//! rule, `n_u = (I − nnᵀ)·(c_u/‖c‖)`, which the scratch proved is the only form
//! tight enough to be usable; when `L = 0` the derivative is genuinely
//! unbounded at the singular locus, and the honest answer is `ENTIRE` per axis,
//! never a sample (spec decision 4). Higher partials (m + n ≥ 2) need the
//! base's third partials and the full shape-operator chain, so they are also
//! `ENTIRE` per axis (the ISC/PCURVE fourth-order precedent).
//!
//! **The `Offset` composition.** `enclose` and `enclose_der` add the two
//! carriers' boxes componentwise. Two composition details, both found by the
//! scratch:
//!
//! - `enclose_der(0, 0)` must return `self.enclose(uu, vv)`, NOT the
//!   composition of the carriers' `enclose_der(0, 0)`: `plane.rs` returns the
//!   ZERO box at `(0, 0)` (an outlier; `line.rs`/`cone.rs`/`revolved.rs` return
//!   the point box). Composing a plane base would under-estimate to zero, a
//!   BG-ENC-001 violation. The `(0, 0)` partial is the position, and the
//!   position is `enclose`.
//! - `normal_cone` and `immersion_lower_bound` follow the family construction
//!   off the summed derivative boxes' cross product
//!   (`cross_box(enclose_der(1,0), enclose_der(0,1))`), certifying per cell.
//!   For the `NormalField` constant case the offset normal EQUALS the base
//!   normal (`n·(S_u + d·n_u) = 0` since `n ⊥ S_u` and `n·n_u = 0`, scratch
//!   dot = 1.0); that tightness fact is recorded in a comment, not assumed —
//!   the family construction is the generic one for a GENERIC `N`.

use crate::enclosure::{
    cross_box, immersion_lower_bound_box, interval_at, midpoint_ball_cone, Box3, DirCone,
    EnclosureScalarField2, EnclosureSurface, EnclosureVectorField,
};
use inari::Interval;
use truck_geometry::decorators::{NormalField, Offset, ScalarFunctionD2};
use truck_geotrait::ParametricSurface3D;

/// The `[-1, 1]³` box that contains every unit vector's coordinates. Sound as a
/// position enclosure of a unit normal even at a singular locus (spec decision
/// 4): a unit vector never leaves the unit ball. Never replaced by a sample.
fn unit_ball_box() -> Box3 {
    let ball = Interval::try_from((-1.0, 1.0)).unwrap_or(Interval::EMPTY);
    Box3 {
        x: ball,
        y: ball,
        z: ball,
    }
}

/// The unbounded box (`ENTIRE` per axis): the honest answer when curvature is
/// genuinely unbounded (a singular locus, or a partial needing the base's
/// third derivatives).
fn entire_box() -> Box3 {
    Box3 {
        x: Interval::ENTIRE,
        y: Interval::ENTIRE,
        z: Interval::ENTIRE,
    }
}

/// The scale interval `[0, 1/l]`, outward rounded, for a certified positive
/// immersion margin `l`. Since `l` is a true lower bound on `‖S_u × S_v‖`,
/// `1/‖S_u × S_v‖ ∈ (0, 1/l]`; the division runs in inari so the upper endpoint
/// rounds up (BG-ENC-003), and `convex_hull` keeps the lower endpoint at 0.
fn scale_1_over(l: f64) -> Interval {
    interval_at(0.0).convex_hull(interval_at(1.0) / interval_at(l))
}

/// The unit normal's position box over the box: the interval cross product of
/// the base's two first-partial enclosures scaled by `[0, 1/L]` when the base
/// is an immersion (`L > 0`), the `[-1, 1]³` fallback otherwise (spec decision
/// 4). `L = S.immersion_lower_bound(uu, vv)`.
fn unit_normal_box<S: ParametricSurface3D + EnclosureSurface>(
    s: &S,
    uu: Interval,
    vv: Interval,
) -> Box3 {
    let l = s.immersion_lower_bound(uu, vv);
    if l > 0.0 {
        let c = cross_box(&s.enclose_der(1, 0, uu, vv), &s.enclose_der(0, 1, uu, vv));
        let scale = scale_1_over(l);
        Box3 {
            x: c.x * scale,
            y: c.y * scale,
            z: c.z * scale,
        }
    } else {
        unit_ball_box()
    }
}

/// Intersects a box with the unit ball per axis. A unit vector has coordinates
/// in `[-1, 1]`, so this is always sound; it keeps the projector in
/// [`normal_partial_box`] bounded.
fn intersect_unit_ball(b: &Box3) -> Box3 {
    let ball = unit_ball_box();
    Box3 {
        x: b.x.intersection(ball.x),
        y: b.y.intersection(ball.y),
        z: b.z.intersection(ball.z),
    }
}

/// The enclosure of a first partial (`m + n == 1`) of the unit normal field of
/// the base, by the projection form of the quotient rule:
///
/// ```text
/// n_u = (I − nnᵀ)·(c_u/‖c‖),   c = S_u × S_v,   c_u = S_uu × S_v + S_u × S_uv
/// ```
///
/// (and symmetrically `c_v` from `S_uv, S_v, S_u, S_vv`). `c` and `c_mn` are
/// enclosed via the shared `cross_box`, the denominator by the base's certified
/// immersion margin `L` (so `w = c_mn · [0, 1/L]`), and the normal's position
/// box is intersected with the unit ball before the projector. When `L = 0` the
/// derivative is genuinely unbounded at a singular locus: `ENTIRE` per axis
/// (spec decision 4), never a sample.
fn normal_partial_box<S: ParametricSurface3D + EnclosureSurface>(
    s: &S,
    m: usize,
    n: usize,
    uu: Interval,
    vv: Interval,
) -> Box3 {
    let l = s.immersion_lower_bound(uu, vv);
    if l == 0.0 {
        return entire_box();
    }
    let du = s.enclose_der(1, 0, uu, vv);
    let dv = s.enclose_der(0, 1, uu, vv);
    let (a, b) = if (m, n) == (1, 0) {
        // c_u = S_uu × S_v + S_u × S_uv.
        (
            cross_box(&s.enclose_der(2, 0, uu, vv), &dv),
            cross_box(&du, &s.enclose_der(1, 1, uu, vv)),
        )
    } else {
        // c_v = S_uv × S_v + S_u × S_vv.
        (
            cross_box(&s.enclose_der(1, 1, uu, vv), &dv),
            cross_box(&du, &s.enclose_der(0, 2, uu, vv)),
        )
    };
    let c_mn = Box3 {
        x: a.x + b.x,
        y: a.y + b.y,
        z: a.z + b.z,
    };
    let scale = scale_1_over(l);
    let w = Box3 {
        x: c_mn.x * scale,
        y: c_mn.y * scale,
        z: c_mn.z * scale,
    };
    let n_pos = intersect_unit_ball(&unit_normal_box(s, uu, vv));
    // (I − nnᵀ)w = w − n(n·w), all in inari over the boxed n and w. Sound but
    // loose: the three occurrences of n are decorrelated, which is acceptable
    // (BG-ENC-001) and keeps the projector bounded via the unit-ball cap.
    let dot = n_pos.x * w.x + n_pos.y * w.y + n_pos.z * w.z;
    Box3 {
        x: w.x - n_pos.x * dot,
        y: w.y - n_pos.y * dot,
        z: w.z - n_pos.z * dot,
    }
}

/// `f64` is a constant scalar field: the field is the degenerate interval
/// `[x, x]` and every partial vanishes except the zeroth.
impl EnclosureScalarField2 for f64 {
    fn enclose(&self, _uu: Interval, _vv: Interval) -> Interval {
        interval_at(*self)
    }

    fn enclose_der(&self, m: usize, n: usize, _uu: Interval, _vv: Interval) -> Interval {
        if m == 0 && n == 0 {
            interval_at(*self)
        } else {
            interval_at(0.0)
        }
    }
}

impl<S, F> EnclosureVectorField for NormalField<S, F>
where
    S: ParametricSurface3D + EnclosureSurface,
    F: ScalarFunctionD2 + EnclosureScalarField2,
{
    fn enclose(&self, uu: Interval, vv: Interval) -> Box3 {
        // N = n·f: the unit normal's box times the scalar's interval,
        // componentwise and outward-rounded.
        let n = unit_normal_box(self.entity(), uu, vv);
        let f = self.scalar().enclose(uu, vv);
        Box3 {
            x: n.x * f,
            y: n.y * f,
            z: n.z * f,
        }
    }

    fn enclose_der(&self, m: usize, n: usize, uu: Interval, vv: Interval) -> Box3 {
        if m == 0 && n == 0 {
            // The zeroth partial is the position — the field's own enclose.
            return self.enclose(uu, vv);
        }
        if m + n >= 2 {
            // Higher partials need the base's third partials and the full
            // shape-operator chain: the honest answer is unbounded (the
            // ISC/PCURVE fourth-order precedent).
            return entire_box();
        }
        // N_mn = n_mn·f + n·f_mn.
        let n_der = normal_partial_box(self.entity(), m, n, uu, vv);
        let n_pos = intersect_unit_ball(&unit_normal_box(self.entity(), uu, vv));
        let f = self.scalar().enclose(uu, vv);
        let f_der = self.scalar().enclose_der(m, n, uu, vv);
        Box3 {
            x: n_der.x * f + n_pos.x * f_der,
            y: n_der.y * f + n_pos.y * f_der,
            z: n_der.z * f + n_pos.z * f_der,
        }
    }
}

/// The interval cross product of the two summed first-partial boxes of an
/// `Offset`: a box that encloses every `S_u(x) × S_v(x)` there. `normal_cone`
/// and `immersion_lower_bound` both go through this one private helper, the
/// shared BG-ENC-004 construction.
fn offset_normal_box<S, N>(s: &Offset<S, N>, uu: Interval, vv: Interval) -> Box3
where
    S: ParametricSurface3D + EnclosureSurface,
    N: EnclosureVectorField,
{
    cross_box(&s.enclose_der(1, 0, uu, vv), &s.enclose_der(0, 1, uu, vv))
}

impl<S, N> EnclosureSurface for Offset<S, N>
where
    S: ParametricSurface3D + EnclosureSurface,
    N: EnclosureVectorField,
{
    fn enclose(&self, uu: Interval, vv: Interval) -> Box3 {
        // The composition is the geometry's own arithmetic: add the two
        // carriers' boxes componentwise.
        let s = self.entity().enclose(uu, vv);
        let n = self.offset().enclose(uu, vv);
        Box3 {
            x: s.x + n.x,
            y: s.y + n.y,
            z: s.z + n.z,
        }
    }

    fn enclose_der(&self, m: usize, n: usize, uu: Interval, vv: Interval) -> Box3 {
        if m == 0 && n == 0 {
            // der_mn(0, 0) returns subs(u, v).to_vec(), so the zeroth enclosure
            // is the position — `enclose`. NOT the composition of the carriers'
            // enclose_der(0, 0): plane.rs returns the ZERO box at (0, 0) (an
            // outlier; line.rs/cone.rs/revolved.rs return the point box), and
            // composing a plane base under-estimates to zero, a BG-ENC-001
            // violation. Do not copy plane.rs/cylinder.rs on this point.
            return self.enclose(uu, vv);
        }
        let s = self.entity().enclose_der(m, n, uu, vv);
        let n = self.offset().enclose_der(m, n, uu, vv);
        Box3 {
            x: s.x + n.x,
            y: s.y + n.y,
            z: s.z + n.z,
        }
    }

    fn normal_cone(&self, uu: Interval, vv: Interval) -> Option<DirCone> {
        // The shared midpoint-ball cone off the interval cross product of the
        // two summed derivative boxes. For the NormalField constant case the
        // offset normal EQUALS the base normal (n·(S_u + d·n_u) = 0 since n ⊥
        // S_u and n·n_u = 0), which is why the family construction is tight
        // there; the tightness fact is recorded here, not assumed — the
        // construction is the generic one for a GENERIC N.
        midpoint_ball_cone(&offset_normal_box(self, uu, vv))
    }

    fn immersion_lower_bound(&self, uu: Interval, vv: Interval) -> f64 {
        // The shared mignitude-immersion lower bound off the same cross product
        // (`crate::enclosure::immersion_lower_bound_box`), certifying per cell.
        immersion_lower_bound_box(&offset_normal_box(self, uu, vv))
    }
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect on paths reachable from untrusted
// geometry. Unit-test assertions on hand-built witnesses are not such a path.
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use truck_base::cgmath64::{InnerSpace, Point3, Vector3};
    use truck_geometry::specifieds::{Plane, Sphere};
    use truck_geotrait::ParametricSurface;

    /// Build a test interval, degrading to EMPTY (and failing its assertion)
    /// rather than panicking on a malformed bound.
    fn iv(lo: f64, hi: f64) -> Interval {
        Interval::try_from((lo, hi)).unwrap_or(Interval::EMPTY)
    }

    /// Whether the unit direction `d` lies inside `cone`, by angle:
    /// `cos(∠(axis, d)) ≥ cos(half_angle)`. The slack absorbs float rounding in
    /// the cosines; a half_angle at or near π/2 needs the `≥` with the slack to
    /// survive the rounding.
    fn cone_contains(cone: &DirCone, d: Vector3) -> bool {
        let cos_ang = cone.axis.dot(d);
        cos_ang >= cone.half_angle.cos() - 1.0e-12 // H-3: float slack between two direction cosines, not a length
    }

    #[test]
    fn offset_sphere_constant_distance_encloses() {
        let base = Sphere::new(Point3::new(0.0, 0.0, 0.0), 1.0);
        let d = 0.3_f64;
        let offset = Offset::new(base, NormalField::new(base, d));
        let uu = iv(0.3, 0.9);
        let vv = iv(0.4, 1.3);

        // BG-ENC-001 soundness on the pole-free box: every sampled subs lies in
        // enclose.
        let box3 = offset.enclose(uu, vv);
        const N_ENCLOSE: usize = 25;
        for i in 0..N_ENCLOSE {
            for j in 0..N_ENCLOSE {
                let u = uu.inf() + (uu.sup() - uu.inf()) * (i as f64) / (N_ENCLOSE as f64 - 1.0);
                let v = vv.inf() + (vv.sup() - vv.inf()) * (j as f64) / (N_ENCLOSE as f64 - 1.0);
                assert!(
                    box3.contains(offset.subs(u, v)),
                    "offset point at ({u},{v}) escaped {box3:?}"
                );
            }
        }

        // Every sampled first partial lies in its enclosure.
        for (m, n) in [(1usize, 0usize), (0, 1)] {
            let der_box = offset.enclose_der(m, n, uu, vv);
            const N_DER1: usize = 15;
            for i in 0..N_DER1 {
                for j in 0..N_DER1 {
                    let u = uu.inf() + (uu.sup() - uu.inf()) * (i as f64) / (N_DER1 as f64 - 1.0);
                    let v = vv.inf() + (vv.sup() - vv.inf()) * (j as f64) / (N_DER1 as f64 - 1.0);
                    let d = offset.der_mn(m, n, u, v);
                    assert!(
                        der_box.contains(Point3::new(d.x, d.y, d.z)),
                        "der({m},{n}) at ({u},{v}) = {d:?} escaped {der_box:?}"
                    );
                }
            }
        }

        // Second-order partials are honestly unbounded (ENTIRE), so every
        // sample trivially lies inside.
        for (m, n) in [(2usize, 0usize), (1, 1), (0, 2)] {
            let der_box = offset.enclose_der(m, n, uu, vv);
            const N_DER2: usize = 9;
            for i in 0..N_DER2 {
                for j in 0..N_DER2 {
                    let u = uu.inf() + (uu.sup() - uu.inf()) * (i as f64) / (N_DER2 as f64 - 1.0);
                    let v = vv.inf() + (vv.sup() - vv.inf()) * (j as f64) / (N_DER2 as f64 - 1.0);
                    let d = offset.der_mn(m, n, u, v);
                    assert!(
                        der_box.contains(Point3::new(d.x, d.y, d.z)),
                        "der({m},{n}) at ({u},{v}) = {d:?} escaped {der_box:?}"
                    );
                }
            }
        }

        // A modest cell has a bounded normal cone that contains every sampled
        // unit normal by angle (scratch: half-angle ≈ 0.86 rad), and a
        // strictly positive immersion bound (scratch: ≈ 0.50).
        let cell_u = iv(1.15, 1.25);
        let cell_v = iv(0.7, 0.9);
        let cone = offset
            .normal_cone(cell_u, cell_v)
            .expect("a modest pole-free sphere cell has a cone");
        const N_CONE: usize = 20;
        for i in 0..N_CONE {
            for j in 0..N_CONE {
                let u = cell_u.inf()
                    + (cell_u.sup() - cell_u.inf()) * (i as f64) / (N_CONE as f64 - 1.0);
                let v = cell_v.inf()
                    + (cell_v.sup() - cell_v.inf()) * (j as f64) / (N_CONE as f64 - 1.0);
                let normal = offset.uder(u, v).cross(offset.vder(u, v)).normalize();
                assert!(
                    cone_contains(&cone, normal),
                    "offset normal at ({u},{v}) escaped cone {cone:?}"
                );
            }
        }
        assert!(
            offset.immersion_lower_bound(cell_u, cell_v) > 0.0,
            "immersion bound must be strictly positive on the pole-free cell"
        );

        // For a constant-distance NormalField the offset's unit normal equals
        // the base's unit normal at a sample point (scratch dot = 1.0):
        // n·(S_u + d·n_u) = 0 since n ⊥ S_u and n·n_u = 0.
        let (u, v) = (0.5, 0.8);
        let n_offset = offset.uder(u, v).cross(offset.vder(u, v)).normalize();
        const DOT_SLACK: f64 = 1.0e-9; // H-3: slack on a dot product of two unit direction vectors, dimensionless
        assert!(
            n_offset.dot(base.normal(u, v)).abs() > 1.0 - DOT_SLACK,
            "offset normal {n_offset:?} does not match the base normal at ({u},{v})"
        );
    }

    #[test]
    fn offset_plane_constant_distance_is_exact() {
        let base = Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        );
        let d = 0.5_f64;
        let offset = Offset::new(base, NormalField::new(base, d));
        let uu = iv(-0.5, 1.5);
        let vv = iv(-0.5, 1.5);

        // Every sampled offset point lies in the enclosure.
        let box3 = offset.enclose(uu, vv);
        const N: usize = 21;
        for i in 0..N {
            for j in 0..N {
                let u = uu.inf() + (uu.sup() - uu.inf()) * (i as f64) / (N as f64 - 1.0);
                let v = vv.inf() + (vv.sup() - vv.inf()) * (j as f64) / (N as f64 - 1.0);
                assert!(
                    box3.contains(offset.subs(u, v)),
                    "offset point at ({u},{v}) escaped {box3:?}"
                );
            }
        }

        // The affine-exact case: the box width is the plane's own (scratch:
        // 2.0), not inflated by the offset.
        assert_eq!(offset.enclose(uu, vv).width(), base.enclose(uu, vv).width());

        // The plane normal is constant: a cone with essentially zero
        // half-angle.
        const CONE_HALF_ANGLE_SLACK: f64 = 1.0e-9; // H-3: slack on a cone half-angle in radians, dimensionless
        let cone = offset
            .normal_cone(uu, vv)
            .expect("a non-degenerate offset plane has a cone");
        assert!(
            cone.half_angle < CONE_HALF_ANGLE_SLACK,
            "plane cone half-angle {}, want < {CONE_HALF_ANGLE_SLACK}",
            cone.half_angle
        );

        // The immersion is the plane's constant cross norm (≈ 1.0).
        const IMMERSION_SLACK: f64 = 1.0e-12; // H-3: slack on an immersion-norm lower bound, dimensionless
        assert!(
            (offset.immersion_lower_bound(uu, vv) - 1.0).abs() < IMMERSION_SLACK,
            "plane immersion lower bound must be ≈ 1.0"
        );
    }

    #[test]
    fn offset_pole_box_degrades_honestly() {
        // A sphere box touching a pole: uu = [0.0, 0.15] reaches u = 0.
        let base = Sphere::new(Point3::new(0.0, 0.0, 0.0), 1.0);
        let d = 0.3_f64;
        let field = NormalField::new(base, d);
        let offset = Offset::new(base, field);
        let uu = iv(0.0, 0.15);
        let vv = iv(0.0, 1.0);

        // The base immersion vanishes on the box.
        assert_eq!(base.immersion_lower_bound(uu, vv), 0.0);

        // The unit-ball fallback keeps the field's position enclosure sound:
        // every sampled normal·d lies inside.
        let box3 = offset.offset().enclose(uu, vv);
        const N_FIELD: usize = 21;
        for i in 0..N_FIELD {
            for j in 0..N_FIELD {
                let u = uu.inf() + (uu.sup() - uu.inf()) * (i as f64) / (N_FIELD as f64 - 1.0);
                let v = vv.inf() + (vv.sup() - vv.inf()) * (j as f64) / (N_FIELD as f64 - 1.0);
                let p = base.normal(u, v) * d;
                assert!(
                    box3.contains(Point3::new(p.x, p.y, p.z)),
                    "field point at ({u},{v}) escaped {box3:?}"
                );
            }
        }

        // The first partial of the field is honestly unbounded at the singular
        // locus: ENTIRE per axis, never a finite sample.
        let der = offset.offset().enclose_der(1, 0, uu, vv);
        assert_eq!(der.x, Interval::ENTIRE);
        assert_eq!(der.y, Interval::ENTIRE);
        assert_eq!(der.z, Interval::ENTIRE);

        // The offset's position enclosure stays sound even there.
        let box3 = offset.enclose(uu, vv);
        const N_OFFSET: usize = 21;
        for i in 0..N_OFFSET {
            for j in 0..N_OFFSET {
                let u = uu.inf() + (uu.sup() - uu.inf()) * (i as f64) / (N_OFFSET as f64 - 1.0);
                let v = vv.inf() + (vv.sup() - vv.inf()) * (j as f64) / (N_OFFSET as f64 - 1.0);
                assert!(
                    box3.contains(offset.subs(u, v)),
                    "offset point at ({u},{v}) escaped {box3:?}"
                );
            }
        }

        // The honest singular-locus arm: no cone bounds the undefined normals.
        assert!(offset.normal_cone(uu, vv).is_none());
    }
}
