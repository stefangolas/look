//! BG-ENC-002-CONE: `EnclosureSurface` for the `Cone` carrier.
//!
//! This is the first carrier in this family that is not an immersion
//! everywhere. The parameterisation (read off `specifieds/cone.rs`) is
//!
//! ```text
//! S(u, v) = apex + v·tan(α)·(cos u, sin u, 0) + (0, 0, v)
//! ```
//!
//! with `u ∈ [0, 2π)` periodic, `v` unbounded and **signed**, and α the half
//! angle. `Cone::new` refuses anything outside `0 < α < π/2`, so
//! `tan(α) > 0` and finite is an invariant every method below may rely on. The
//! carrier is a *double* cone joined at the apex: `v < 0` is the opposite
//! nappe.
//!
//! **The apex is the point of this packet.** At `v = 0` the whole `u` circle
//! collapses to the single point `apex`; `S_u` vanishes identically, the cross
//! product `S_u × S_v` is zero, and there is no normal direction — that is why
//! the `Option<DirCone>` in the trait earns its existence. Cells whose `vv`
//! contains `0.0` get `None` from `normal_cone`, with no exceptions.
//!
//! The interval trigonometric functions are this crate's own certified pair
//! ([`crate::elementary`]), not `inari`'s feature-gated ones (`inari` is taken
//! with `default-features = false`). `cos(uu)` and `sin(uu)` are outward-rounded
//! enclosures that account for the interior extrema at `kπ/2`, so they must
//! never be replaced by endpoint evaluation.

use crate::elementary::{cos, sin};
use crate::enclosure::{Box3, DirCone, EnclosureSurface};
use inari::Interval;
use std::f64::consts::PI;
use truck_base::cgmath64::Vector3;
use truck_geometry::specifieds::Cone;

/// The `u`-arc width at which the normals no longer fit around their midpoint
/// bisector: at an arc wider than π, every normal on one nappe lies in the
/// hemisphere around the `−sign(v)·z` axis, which is the sound and tighter
/// answer (H-3).
const FULL_ARC_HALF_PI: f64 = PI;

/// A degenerate interval from a runtime `f64`. Finite values always construct;
/// a NaN widens to the empty interval rather than panicking (H-1).
fn interval_at(x: f64) -> Interval {
    Interval::try_from((x, x)).unwrap_or(Interval::EMPTY)
}

/// The zero box: the enclosure of an identically-zero derivative.
fn zero_box() -> Box3 {
    Box3 {
        x: interval_at(0.0),
        y: interval_at(0.0),
        z: interval_at(0.0),
    }
}

impl EnclosureSurface for Cone {
    fn enclose(&self, uu: Interval, vv: Interval) -> Box3 {
        // S(u, v) = apex + s·v·(cos u, sin u, 0) + (0, 0, v) with s = tan(α).
        // s is a degenerate interval computed once because the f64 tangent is
        // not exact; everything else is plain outward-rounded interval
        // arithmetic, and vv being signed is handled by inari, not by a sign
        // case analysis.
        let s = interval_at(self.half_angle().tan());
        let apex = self.apex();
        Box3 {
            x: interval_at(apex.x) + s * vv * cos(uu),
            y: interval_at(apex.y) + s * vv * sin(uu),
            z: interval_at(apex.z) + vv,
        }
    }

    fn enclose_der(&self, m: usize, n: usize, uu: Interval, vv: Interval) -> Box3 {
        let s = interval_at(self.half_angle().tan());
        // S is affine in v for fixed u, so every second-and-higher v-derivative
        // vanishes identically (both the radial amplitude and the z term are
        // zero there).
        if n >= 2 {
            return zero_box();
        }
        if m == 0 {
            // (0,0) is the surface itself; (0,1) is vder, whose z-component is
            // the exact degenerate interval at 1.
            if n == 0 {
                return self.enclose(uu, vv);
            }
            return Box3 {
                x: s * cos(uu),
                y: s * sin(uu),
                z: interval_at(1.0),
            };
        }
        // The u-derivatives cycle with period 4 in m; the v amplitude is s·v
        // for n == 0 and s for n == 1, and the z component is zero for m >= 1.
        let (xu, yu) = match m % 4 {
            0 => (cos(uu), sin(uu)),
            1 => (-sin(uu), cos(uu)),
            2 => (-cos(uu), -sin(uu)),
            _ => (sin(uu), -cos(uu)),
        };
        let amp = if n == 0 { s * vv } else { s };
        Box3 {
            x: amp * xu,
            y: amp * yu,
            z: interval_at(0.0),
        }
    }

    fn normal_cone(&self, uu: Interval, vv: Interval) -> Option<DirCone> {
        // At v = 0 the whole u-circle collapses to the apex: S_u = 0 and there
        // is no normal direction. A cell whose vv contains 0 also straddles
        // both nappes, whose normals point into opposite half-spaces, so no
        // single cone of directions can contain them. Either way: None, with no
        // exceptions, including the degenerate vv = [0, 0].
        if vv.contains(0.0) {
            return None;
        }
        let slope = self.half_angle().tan();
        let norm = (1.0 + slope * slope).sqrt();
        // sign(v) is constant over a cell that does not touch zero.
        let sgn = if vv.inf() > 0.0 { 1.0 } else { -1.0 };
        let w = uu.sup() - uu.inf();
        if w > FULL_ARC_HALF_PI {
            // Every normal on one nappe makes the constant angle α < π/2 with
            // the −sign(v)·z axis, so a hemisphere around it contains all of
            // them. Sound, loose, correct.
            Some(DirCone {
                axis: Vector3::new(0.0, 0.0, -sgn),
                half_angle: PI / 2.0,
            })
        } else {
            // The normals over an arc of angular width w are *not* spread by
            // w/2 about their bisector once they are tilted out of the plane —
            // the tilt shrinks the spread. w/2 is therefore an over-estimate,
            // which is sound: a cone that is too wide still contains every
            // normal. Tightness is BG-ENC-004's problem, not this impl's.
            let m = (uu.inf() + uu.sup()) / 2.0;
            let (su, cu) = m.sin_cos();
            let axis = sgn * Vector3::new(cu, su, -slope) / norm;
            Some(DirCone {
                axis,
                half_angle: w / 2.0,
            })
        }
    }

    fn immersion_lower_bound(&self, _uu: Interval, vv: Interval) -> f64 {
        // ‖S_u × S_v‖ = s·|v|·sqrt(1 + s²), minimized at the v of smallest
        // absolute value. Computed in interval arithmetic and read from the
        // lower endpoint so the directed rounding rounds down: a lower bound a
        // rounding-unit too large is a soundness bug, not a tightness one.
        let slope = self.half_angle().tan();
        let scale = interval_at(slope) * (interval_at(1.0) + interval_at(slope).sqr()).sqrt();
        let v_min = if vv.contains(0.0) {
            // The cell touches the apex, where the immersion vanishes exactly.
            interval_at(0.0)
        } else {
            let lo = vv.inf().abs();
            let hi = vv.sup().abs();
            interval_at(lo.min(hi))
        };
        (v_min * scale).inf()
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
    use truck_geotrait::ParametricSurface3D;

    /// Unwraps a cone from its construction outcome. The half angle is always
    /// within the supported envelope `(0, PI/2)`, so the expectation cannot
    /// fail.
    fn cone_at(half_angle: f64) -> Cone {
        Cone::new(Point3::new(0.0, 0.0, 0.0), half_angle)
            .expect("half angle in (0, PI/2)")
            .value
    }

    /// Whether the unit direction `d` lies inside `cone`, by angle:
    /// `cos(∠(axis, d)) ≥ cos(half_angle)`. The slack absorbs float rounding in
    /// the cosines; `half_angle = π/2` needs the `≥` with the slack to survive
    /// the rounding.
    fn cone_contains(cone: &DirCone, d: Vector3) -> bool {
        let cos_ang = cone.axis.dot(d);
        cos_ang >= cone.half_angle.cos() - 1.0e-12 // H-3: float slack between two direction cosines, not a length
    }

    #[test]
    fn cone_encloses_sampled_points() {
        let cone = cone_at(PI / 6.0);
        // A small arc on the positive nappe; an arc crossing π/2; an arc wider
        // than π; a full 2π sweep; a box entirely on the far (negative) nappe;
        // and a box whose vv straddles the apex (normal cone None, enclosure
        // must still hold).
        let cases = [
            (const_interval!(0.1, 1.0), const_interval!(1.0, 2.0)),
            (const_interval!(1.0, 2.0), const_interval!(0.5, 1.5)),
            (const_interval!(0.2, 3.5), const_interval!(0.5, 2.0)),
            (const_interval!(0.0, 2.0 * PI), const_interval!(0.3, 1.2)),
            (const_interval!(0.0, 1.5), const_interval!(-2.0, -0.5)),
            (const_interval!(0.2, 1.0), const_interval!(-1.0, 1.5)),
        ];
        for (uu, vv) in cases {
            assert_encloses_surface(&cone, uu, vv, 25);
        }
    }

    #[test]
    fn cone_trig_extrema_inside_interval() {
        let cone = cone_at(PI / 6.0);
        let s = cone.half_angle().tan();
        // uu spans π/2 strictly inside, where cos(0.5π) = 0 is attained (and
        // where sin peaks at 1). vv is bounded away from the apex on the
        // positive nappe.
        let uu = const_interval!(0.4 * PI, 0.6 * PI);
        let vv = const_interval!(1.0, 2.0);
        let box3 = cone.enclose(uu, vv);
        // The x-coordinate is s·v·cos(u): cos(0.5π) = 0 is attained inside the
        // cell, so the x-interval must contain it. Endpoint-only evaluation
        // would only see cos at 0.4π and 0.6π and cannot reach an interior
        // value whose parameter sits between them.
        assert!(
            box3.x.contains(0.0),
            "x-interval {:?} must contain the interior value cos(pi/2) = 0",
            box3.x
        );
        // sin(0.5π) = 1 is the interior extremum of the same cell: the
        // y-interval must reach the peak scaled by the cell, s·vv.sup(), and
        // must be strictly wider than the endpoint-only hull, which only sees
        // sin ≈ 0.951 at both u-endpoints.
        let endpoint_only_y = s * vv.sup() * (0.4 * PI).sin();
        assert!(
            endpoint_only_y < s * vv.sup(),
            "test cell must not peak at a u-endpoint"
        );
        assert!(
            box3.y.sup() >= s * vv.sup(),
            "y-interval {:?} must reach the interior peak sin(pi/2) = 1 scaled by the cell",
            box3.y
        );
        assert!(
            box3.y.sup() > endpoint_only_y,
            "y-interval {:?} must be strictly wider than endpoint-only evaluation {}",
            box3.y,
            endpoint_only_y
        );
    }

    #[test]
    fn cone_enclosure_converges_under_bisection() {
        let cone = cone_at(PI / 6.0);
        // A moderate box on the positive nappe, bounded away from the apex:
        // the harness demands point convergence (BG-ENC-002).
        let uu = const_interval!(0.0, 2.0);
        let vv = const_interval!(1.0, 3.0);
        let initial = cone.enclose(uu, vv).width();
        assert_converges(&cone, uu, vv, initial, 20);

        // A box containing the apex cannot be driven to a point — at v = 0 the
        // whole u-circle collapses to the apex, so the carrier is not an
        // immersion there. Assert what is actually true: bisection still
        // shrinks the enclosure in u and v and never widens it.
        let mut uu = const_interval!(0.0, 2.0);
        let mut vv = const_interval!(-1.0, 1.0);
        let initial = cone.enclose(uu, vv).width();
        let mut prev = initial;
        for _ in 0..30 {
            if uu.sup() - uu.inf() >= vv.sup() - vv.inf() {
                let mid = (uu.inf() + uu.sup()) / 2.0;
                uu = Interval::try_from((uu.inf(), mid)).unwrap_or(uu);
            } else {
                let mid = (vv.inf() + vv.sup()) / 2.0;
                vv = Interval::try_from((vv.inf(), mid)).unwrap_or(vv);
            }
            let cur = cone.enclose(uu, vv).width();
            assert!(
                cur <= prev,
                "apex box widened under bisection: {prev} -> {cur}"
            );
            prev = cur;
        }
        assert!(
            prev < initial,
            "apex box still shrinks in u and v: {prev} >= {initial}"
        );
    }

    #[test]
    fn cone_normal_cone_refuses_across_the_apex() {
        let cone = cone_at(PI / 6.0);
        // Cells whose vv touches or straddles the apex have no normal
        // direction: v = 0 collapses the u-circle to the apex, and straddling
        // cells also contain both nappes, whose normals point into opposite
        // half-spaces.
        for vv in [
            const_interval!(-1.0, 1.0),
            const_interval!(0.0, 1.0),
            const_interval!(-1.0, 0.0),
            const_interval!(0.0, 0.0),
        ] {
            assert!(
                cone.normal_cone(const_interval!(0.0, 2.0), vv).is_none(),
                "vv = {vv:?} must not admit a normal cone"
            );
        }
        // Away from the apex the cone is Some on either nappe and contains
        // every sampled unit normal, by angle.
        let some_cases = [
            (const_interval!(0.0, 1.0), const_interval!(1.0, 2.0)),
            (const_interval!(1.0, 3.0), const_interval!(0.5, 1.5)),
            (const_interval!(0.2, 3.5), const_interval!(1.0, 2.0)),
            (const_interval!(0.0, 1.5), const_interval!(-2.0, -0.5)),
        ];
        for (uu, vv) in some_cases {
            let dc = cone
                .normal_cone(uu, vv)
                .expect("cell away from the apex has a normal cone");
            // The sign of v is constant over the cell; sample at its midpoint
            // and sweep u.
            let v = (vv.inf() + vv.sup()) / 2.0;
            const SAMPLES: usize = 30;
            for i in 0..SAMPLES {
                let u = uu.inf() + (uu.sup() - uu.inf()) * (i as f64) / (SAMPLES as f64 - 1.0);
                let normal = cone.normal(u, v);
                assert!(
                    cone_contains(&dc, normal),
                    "normal at ({u},{v}) = {normal:?} escaped cone {dc:?}"
                );
            }
        }
    }

    #[test]
    fn cone_immersion_lower_bound_vanishes_at_the_apex() {
        let cone = cone_at(PI / 6.0);
        let slope = cone.half_angle().tan();
        // Every cell containing v = 0 touches the apex, where S_u × S_v = 0:
        // the lower bound is exactly zero.
        for vv in [
            const_interval!(-1.0, 1.0),
            const_interval!(0.0, 1.0),
            const_interval!(-1.0, 0.0),
            const_interval!(0.0, 0.0),
            const_interval!(-0.1, 0.2),
        ] {
            assert_eq!(
                cone.immersion_lower_bound(const_interval!(0.0, 1.0), vv),
                0.0
            );
        }
        // Away from the apex the bound is strictly positive and a genuine
        // lower bound on ‖S_u × S_v‖ = s·|v|·sqrt(1 + s²) at every sample.
        let uu = const_interval!(0.1, 2.0);
        for vv in [const_interval!(0.5, 1.5), const_interval!(-2.0, -1.0)] {
            let lb = cone.immersion_lower_bound(uu, vv);
            assert!(lb > 0.0, "bound on {vv:?} must be strictly positive: {lb}");
            const SAMPLES: usize = 20;
            for i in 0..SAMPLES {
                for j in 0..SAMPLES {
                    let u = uu.inf() + (uu.sup() - uu.inf()) * (i as f64) / (SAMPLES as f64 - 1.0);
                    let v = vv.inf() + (vv.sup() - vv.inf()) * (j as f64) / (SAMPLES as f64 - 1.0);
                    let sampled = slope * v.abs() * (1.0 + slope * slope).sqrt();
                    assert!(
                        lb <= sampled,
                        "bound {lb} exceeds sampled ‖S_u × S_v‖ = {sampled} at ({u},{v})"
                    );
                }
            }
        }
    }

    #[test]
    fn cone_der_enclosures_match_partials() {
        let cone = cone_at(PI / 6.0);
        let slope = cone.half_angle().tan();
        let uu = const_interval!(0.1, 2.0);
        let vv = const_interval!(0.5, 1.5);
        let e10 = cone.enclose_der(1, 0, uu, vv);
        let e01 = cone.enclose_der(0, 1, uu, vv);
        let e20 = cone.enclose_der(2, 0, uu, vv);
        let e11 = cone.enclose_der(1, 1, uu, vv);
        const SAMPLES: usize = 20;
        for i in 0..SAMPLES {
            for j in 0..SAMPLES {
                let u = uu.inf() + (uu.sup() - uu.inf()) * (i as f64) / (SAMPLES as f64 - 1.0);
                let v = vv.inf() + (vv.sup() - vv.inf()) * (j as f64) / (SAMPLES as f64 - 1.0);
                let (su, cu) = u.sin_cos();
                let partials = [
                    (slope * v * Vector3::new(-su, cu, 0.0), e10),
                    (
                        slope * Vector3::new(cu, su, 0.0) + Vector3::new(0.0, 0.0, 1.0),
                        e01,
                    ),
                    (slope * v * Vector3::new(-cu, -su, 0.0), e20),
                    (slope * Vector3::new(-su, cu, 0.0), e11),
                ];
                for (partial, enclosure) in partials {
                    assert!(
                        enclosure.contains(Point3::new(partial.x, partial.y, partial.z)),
                        "partial at ({u},{v}) = {partial:?} escaped {enclosure:?}"
                    );
                }
            }
        }
        // (0,2) is the zero box: S is affine in v for fixed u.
        let dvv = cone.enclose_der(0, 2, uu, vv);
        assert_eq!(dvv.x.inf(), 0.0);
        assert_eq!(dvv.x.sup(), 0.0);
        assert_eq!(dvv.y.inf(), 0.0);
        assert_eq!(dvv.y.sup(), 0.0);
        assert_eq!(dvv.z.inf(), 0.0);
        assert_eq!(dvv.z.sup(), 0.0);
    }
}
