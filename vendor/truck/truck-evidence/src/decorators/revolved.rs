//! BG-ENC-004-REVOLVED: `EnclosureSurface` for the `RevolutedCurve` decorator.
//!
//! `RevolutedCurve<C>` sweeps a profile curve around an axis:
//!
//! ```text
//! S(u, v) = o + R(v)·(C(u) − o)
//! ```
//!
//! where `R(v)` is the rotation matrix about the axis and `o` the origin. It
//! is a *decorator*: its enclosure is a composition, computed by calling the
//! inner carrier's `enclose` / `enclose_der` for the `u` factor and combining
//! the resulting boxes with an interval rotation matrix for the `v` factor.
//! Nothing here evaluates a parameterisation directly; the only arithmetic of
//! its own is the interval trigonometry that builds the rotation matrix.
//!
//! The rotation-derivative matrix is this packet's whole design. The carrier
//! writes every partial as
//!
//! ```text
//! der_mn(m, n, u, v) = R^(n)(v)·u_part(m, u) + (o if (m, n) == (0, 0)),
//! ```
//!
//! with `u_part(0, u) = C(u) − o` and `u_part(m, u) = C^(m)(u)` otherwise, and
//! [`rot_der`] mirrors `from_axis_angle_derivation` line for line, so the
//! interval product is the same row sums the carrier computes in `f64`.
//!
//! **Sound but loose (BG-ENC-001).** The matrix-box product encloses
//! `{ R·p : R ∈ M, p ∈ u_part }`, a superset of the true set, because it lets
//! the rotation and the profile point vary independently when in truth `R`
//! depends only on `v` and `p` only on `u`. That decorrelation is precisely
//! what makes the product an over-estimate — acceptable by BG-ENC-001, and
//! why this enclosure is noticeably wider than the analytic `Torus` carrier's
//! for the same patch. Tightening it is not this impl's job.
//!
//! **The singular locus is where the profile curve meets the axis.** There
//! `C(u) − o` is parallel to the axis, rotating it does nothing, `S_v = 0`,
//! and the surface has no normal — the apex of a revolved cone, the poles of a
//! revolved circle. `RevolutedCurve` even carries `is_front_fixed()` /
//! `is_back_fixed()` as hints that a profile end sits on the axis; the generic
//! construction here detects the singularity numerically (the normal cone
//! returns `None`, the immersion bound falls to `0.0`) without calling them.
//!
//! The interval trigonometric functions are this crate's own certified pair
//! ([`crate::elementary`]), not `inari`'s feature-gated ones — `inari` is
//! taken with `default-features = false`. `cos(vv)`/`sin(vv)` are outward
//! rounded and account for the interior extrema at `kπ/2`; they must never be
//! replaced by endpoint evaluation.

use crate::elementary::{cos, sin};
use crate::enclosure::{
    cross_box, immersion_lower_bound_box, interval_at, midpoint_ball_cone, Box3, DirCone,
    EnclosureCurve, EnclosureSurface,
};
use inari::Interval;
use truck_base::cgmath64::{Point3, Vector3};
use truck_geometry::decorators::RevolutedCurve;

/// The interval enclosure of the `n`-th `v`-derivative of the rotation matrix
/// about `axis`, for `v ∈ vv`.
///
/// Mirrors `from_axis_angle_derivation` (in `revolved_curve.rs`) exactly. The
/// returned array is `M[col][row]` — column-major, the same layout
/// `cgmath::Matrix3::new` writes its nine arguments in — so each column reads
/// off the carrier's source line for line. A transposed layout is the most
/// likely defect in this family and the transpose test exists to catch it.
fn rot_der(n: usize, axis: Vector3, vv: Interval) -> [[Interval; 3]; 3] {
    let s = sin(vv);
    let c = cos(vv);
    // The derivative cycles mod 4 exactly as the carrier's Rad does.
    let (s, c) = match n % 4 {
        0 => (s, c),
        1 => (c, -s),
        2 => (-s, -c),
        _ => (-c, s),
    };
    // The 1 − cos coefficient is keyed on n itself, not on n % 4.
    let k = match n {
        0 => interval_at(1.0) - c,
        _ => -c,
    };
    // The axis is already unit (Revolution::new normalises it at
    // construction), so its components are treated as exact degenerate
    // intervals — the same f64 values the carrier multiplies.
    let (ax, ay, az) = (
        interval_at(axis.x),
        interval_at(axis.y),
        interval_at(axis.z),
    );
    // Column 0 (rows 0, 1, 2): the carrier's first three Matrix3::new args.
    let col0 = [k * ax * ax + c, k * ax * ay + s * az, k * ax * az - s * ay];
    let col1 = [k * ax * ay - s * az, k * ay * ay + c, k * ay * az + s * ax];
    let col2 = [k * ax * az + s * ay, k * ay * az - s * ax, k * az * az + c];
    [col0, col1, col2]
}

/// The `u`-factor of the partial: `{ C(u) − o }` for `m = 0`, `{ C^(m)(u) }`
/// otherwise, as boxes from the inner carrier's own enclosures.
fn u_part<C: EnclosureCurve<Vector = Vector3>>(
    curve: &C,
    origin: Point3,
    m: usize,
    uu: Interval,
) -> Box3 {
    let box3 = if m == 0 {
        curve.enclose(uu)
    } else {
        curve.enclose_der(m, uu)
    };
    if m == 0 {
        // der_mn's (0, n) case feeds C(u) − o into the rotation; subtract the
        // origin componentwise in interval arithmetic so the subtraction's
        // rounding is carried rather than assumed away.
        Box3 {
            x: box3.x - interval_at(origin.x),
            y: box3.y - interval_at(origin.y),
            z: box3.z - interval_at(origin.z),
        }
    } else {
        box3
    }
}

/// The interval cross product of the two partial enclosures of `surface` over
/// the box: a box that encloses every `S_u(x) × S_v(x)` there, via the shared
/// `crate::enclosure::cross_box`.
fn normal_box<C: EnclosureCurve<Vector = Vector3>>(
    surface: &RevolutedCurve<C>,
    uu: Interval,
    vv: Interval,
) -> Box3 {
    cross_box(
        &surface.enclose_der(1, 0, uu, vv),
        &surface.enclose_der(0, 1, uu, vv),
    )
}

impl<C: EnclosureCurve<Vector = Vector3>> EnclosureSurface for RevolutedCurve<C> {
    fn enclose(&self, uu: Interval, vv: Interval) -> Box3 {
        // enclose is the (0, 0) partial — the point box — by the crate's
        // convention, so write it in terms of the one shared expression.
        self.enclose_der(0, 0, uu, vv)
    }

    fn enclose_der(&self, m: usize, n: usize, uu: Interval, vv: Interval) -> Box3 {
        let up = u_part(self.entity_curve(), self.origin(), m, uu);
        let [[m00, m01, m02], [m10, m11, m12], [m20, m21, m22]] = rot_der(n, self.axis(), vv);
        // The ordinary three row sums in intervals, exactly the product the
        // carrier writes in f64: out_row = Σ_col M[col][row] · p[col].
        let x = m00 * up.x + m10 * up.y + m20 * up.z;
        let y = m01 * up.x + m11 * up.y + m21 * up.z;
        let z = m02 * up.x + m12 * up.y + m22 * up.z;
        if m == 0 && n == 0 {
            // der_mn(0, 0) returns subs(u, v).to_vec(), whose components equal
            // the point's coordinates, so the (0, 0) enclosure is the point
            // box: the rotation image of C(u) − o shifted back by the origin.
            // This is the crate's convention (line.rs, cone.rs); plane.rs and
            // cylinder.rs return the zero box at (0, 0) instead and are the
            // outliers — do not copy them on this point.
            let o = self.origin();
            Box3 {
                x: x + interval_at(o.x),
                y: y + interval_at(o.y),
                z: z + interval_at(o.z),
            }
        } else {
            Box3 { x, y, z }
        }
    }

    fn normal_cone(&self, uu: Interval, vv: Interval) -> Option<DirCone> {
        // The shared midpoint-ball cone off the interval cross product of the
        // two partial enclosures; the construction (rounding directions,
        // refusal condition, ulp nudge and clamp) lives in
        // `crate::enclosure::midpoint_ball_cone`.
        midpoint_ball_cone(&normal_box(self, uu, vv))
    }

    fn immersion_lower_bound(&self, uu: Interval, vv: Interval) -> f64 {
        // The shared mignitude-immersion lower bound off the interval cross
        // product of the two partial enclosures
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
    use truck_base::cgmath64::{EuclideanSpace, InnerSpace};
    use truck_geometry::specifieds::{Line, UnitCircle};
    use truck_geotrait::ParametricSurface;

    const TAU: f64 = core::f64::consts::TAU;

    /// Build a test interval, degrading to EMPTY (and failing its assertion)
    /// rather than panicking on a malformed bound.
    fn iv(lo: f64, hi: f64) -> Interval {
        Interval::try_from((lo, hi)).unwrap_or(Interval::EMPTY)
    }

    /// A line swept about the z axis at radius one: a cylinder, an immersion
    /// everywhere — the easy case, and the one whose normals are unit.
    fn revolved_cylinder() -> RevolutedCurve<Line<Point3>> {
        RevolutedCurve::by_revolution(
            Line(Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 0.0, 1.0)),
            Point3::origin(),
            Vector3::unit_z(),
        )
    }

    /// A line from the origin to (1, 0, 1) swept about the z axis: a cone
    /// whose profile starts on the axis, so u near 0 is singular.
    fn revolved_cone() -> RevolutedCurve<Line<Point3>> {
        RevolutedCurve::by_revolution(
            Line(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 1.0)),
            Point3::origin(),
            Vector3::unit_z(),
        )
    }

    /// The unit circle (cos t, sin t, 0) swept about the y axis: the circle
    /// crosses that axis at t = ±π/2, giving two singular parameters and a
    /// curved profile.
    fn revolved_circle() -> RevolutedCurve<UnitCircle<Point3>> {
        RevolutedCurve::by_revolution(UnitCircle::new(), Point3::origin(), Vector3::unit_y())
    }

    /// A cylinder-like line about a non-axis-aligned axis at a non-zero
    /// origin. An axis-aligned test passes a transposed rot_der, which is the
    /// defect most likely to be present, so one configuration must be oblique.
    fn revolved_oblique() -> RevolutedCurve<Line<Point3>> {
        RevolutedCurve::by_revolution(
            Line(Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 0.0, 1.0)),
            Point3::new(0.5, -0.25, 0.125),
            Vector3::new(1.0, 1.0, 1.0).normalize(),
        )
    }

    /// Whether the unit direction `d` lies inside `cone`, by angle:
    /// `cos(∠(axis, d)) ≥ cos(half_angle)`. The slack absorbs float rounding
    /// in the cosines; a half_angle at or near π/2 needs the `≥` with the
    /// slack to survive the rounding.
    fn cone_contains(cone: &DirCone, d: Vector3) -> bool {
        let cos_ang = cone.axis.dot(d);
        cos_ang >= cone.half_angle.cos() - 1.0e-12 // H-3: float slack between two direction cosines, not a length
    }

    #[test]
    fn revolved_encloses_sampled_points() {
        // vv cells covering the family of interest: a crossing of π/2 (the
        // interior extrema of the interval trig), an arc wider than π, a full
        // 2π sweep, and an entirely negative cell. All four configurations
        // must enclose their sampled points over every cell (BG-ENC-001).
        let vvs = [
            iv(0.3, 2.0),   // crosses π/2
            iv(0.0, 4.0),   // spans more than π
            iv(0.0, TAU),   // a full 2π sweep
            iv(-3.0, -0.5), // entirely negative
        ];
        let uu = iv(0.0, 1.0);
        for &vv in &vvs {
            assert_encloses_surface(&revolved_cylinder(), uu, vv, 25);
            assert_encloses_surface(&revolved_cone(), uu, vv, 25);
            assert_encloses_surface(&revolved_oblique(), uu, vv, 25);
            assert_encloses_surface(&revolved_circle(), iv(0.0, TAU), vv, 25);
        }
    }

    /// The three profiles whose `u_part` at `u = 1` is exactly the
    /// corresponding basis direction: Line(origin, e_x) recovers matrix
    /// column 0, e_y column 1, e_z column 2.
    fn basis_profiles() -> [Line<Point3>; 3] {
        [
            Line(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)),
            Line(Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0)),
            Line(Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 0.0, 1.0)),
        ]
    }

    #[test]
    fn revolved_rotation_matrix_derivatives_match() {
        // The transpose test. A profile line through the origin along e_col
        // makes u_part = u·e_col, so der_mn(0, n, u, v) = R^(n)(v)·u·e_col = u
        // times column `col` of the rotation-derivative matrix. Rotating the
        // three basis lines therefore recovers the three columns of rot_der
        // one at a time. A transposed matrix would hand every probe the wrong
        // column and fail here; the non-axis-aligned axis would too, which is
        // why it is included.
        let axes = [
            Vector3::unit_x(),
            Vector3::unit_y(),
            Vector3::unit_z(),
            Vector3::new(1.0, 1.0, 1.0).normalize(),
        ];
        let vs = [0.0, 0.7, 2.1, -1.3, 5.8];
        for &axis in &axes {
            for n in 0..=4 {
                for &v in &vs {
                    for (col, profile) in basis_profiles().iter().enumerate() {
                        let surface =
                            RevolutedCurve::by_revolution(*profile, Point3::origin(), axis);
                        // The surface's own stored axis (post-normalisation) is
                        // the one the carrier rotates with.
                        let [[c0r0, c0r1, c0r2], [c1r0, c1r1, c1r2], [c2r0, c2r1, c2r2]] =
                            rot_der(n, surface.axis(), interval_at(v));
                        // u = 1 so u_part is exactly the basis direction and
                        // der_mn divides cleanly by it.
                        let deriv = surface.der_mn(0, n, 1.0, v);
                        let (e0, e1, e2) = match col {
                            0 => (c0r0, c0r1, c0r2),
                            1 => (c1r0, c1r1, c1r2),
                            _ => (c2r0, c2r1, c2r2),
                        };
                        assert!(
                            e0.contains(deriv.x),
                            "axis={axis:?} n={n} v={v} col={col}: M[{col}][0] {:?} misses {}",
                            e0,
                            deriv.x
                        );
                        assert!(
                            e1.contains(deriv.y),
                            "axis={axis:?} n={n} v={v} col={col}: M[{col}][1] {:?} misses {}",
                            e1,
                            deriv.y
                        );
                        assert!(
                            e2.contains(deriv.z),
                            "axis={axis:?} n={n} v={v} col={col}: M[{col}][2] {:?} misses {}",
                            e2,
                            deriv.z
                        );
                    }
                }
            }
        }
    }

    /// For one configuration, every listed partial enclosure must contain the
    /// surface's own `der_mn` sampled over a grid.
    fn assert_der_enclosures<S>(surface: &S, uu: Interval, vv: Interval)
    where
        S: EnclosureSurface + ParametricSurface<Vector = Vector3>,
    {
        for (m, n) in [(0, 0), (1, 0), (0, 1), (2, 0), (1, 1), (0, 2)] {
            let enclosure = surface.enclose_der(m, n, uu, vv);
            const SAMPLES: usize = 20;
            for i in 0..SAMPLES {
                for j in 0..SAMPLES {
                    let u = uu.inf() + (uu.sup() - uu.inf()) * (i as f64) / (SAMPLES as f64 - 1.0);
                    let v = vv.inf() + (vv.sup() - vv.inf()) * (j as f64) / (SAMPLES as f64 - 1.0);
                    let deriv = surface.der_mn(m, n, u, v);
                    assert!(
                        enclosure.contains(Point3::new(deriv.x, deriv.y, deriv.z)),
                        "partial ({m},{n}) at ({u},{v}) = {deriv:?} escaped {enclosure:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn revolved_der_enclosures_match_partials() {
        assert_der_enclosures(&revolved_cylinder(), iv(0.1, 0.9), iv(0.3, 1.7));
        assert_der_enclosures(&revolved_cone(), iv(0.1, 1.0), iv(0.3, 1.7));
        assert_der_enclosures(&revolved_circle(), iv(0.2, 4.0), iv(0.3, 2.5));
        assert_der_enclosures(&revolved_oblique(), iv(0.1, 0.9), iv(0.3, 1.7));
    }

    #[test]
    fn revolved_normal_cone_contains_sampled_normals() {
        // The cylinder at a modest vv cell bounded away from the axis has a
        // bounded normal cone, and the cone must contain every sampled unit
        // normal (S_u × S_v).normalize() by angle. A full 2π sweep spreads the
        // normals across the whole horizontal plane and cannot be bounded by
        // one cone: None is the contract, not a failure.
        let surface = revolved_cylinder();
        let uu = iv(0.0, 1.0);
        let vv = iv(0.2, 0.7);
        let cone = surface
            .normal_cone(uu, vv)
            .expect("a modest vv cell of the cylinder has a bounded cone");
        const SAMPLES: usize = 20;
        for i in 0..SAMPLES {
            for j in 0..SAMPLES {
                let u = uu.inf() + (uu.sup() - uu.inf()) * (i as f64) / (SAMPLES as f64 - 1.0);
                let v = vv.inf() + (vv.sup() - vv.inf()) * (j as f64) / (SAMPLES as f64 - 1.0);
                let normal = surface.uder(u, v).cross(surface.vder(u, v)).normalize();
                assert!(
                    cone_contains(&cone, normal),
                    "normal at ({u},{v}) = {normal:?} escaped cone {cone:?}"
                );
            }
        }
        assert!(
            surface.normal_cone(uu, iv(0.0, TAU)).is_none(),
            "a full 2π sweep cannot be bounded by one cone"
        );
    }

    #[test]
    fn revolved_immersion_lower_bound_vanishes_on_the_axis() {
        // A cone cell whose profile u reaches 0 sits on the axis: at u = 0 the
        // profile equals the origin, so S_v = R'(v)·(C(u) − o) = 0 and the
        // immersion vanishes exactly.
        assert_eq!(
            revolved_cone().immersion_lower_bound(iv(0.0, 0.5), iv(0.2, 0.9)),
            0.0
        );
        // The revolved circle crosses the y axis at t = π/2 (and 3π/2): there
        // C(t) is on the axis and S_v = 0 again.
        assert_eq!(
            revolved_circle().immersion_lower_bound(iv(1.4, 1.7), iv(0.3, 0.9)),
            0.0
        );
        // Away from the axis the bound is strictly positive and a genuine
        // lower bound on the sampled ‖S_u × S_v‖ at every grid point.
        let surface = revolved_cylinder();
        let uu = iv(0.0, 1.0);
        let vv = iv(0.2, 0.7);
        let lb = surface.immersion_lower_bound(uu, vv);
        assert!(lb > 0.0, "cylinder bound must be strictly positive: {lb}");
        const SAMPLES: usize = 20;
        for i in 0..SAMPLES {
            for j in 0..SAMPLES {
                let u = uu.inf() + (uu.sup() - uu.inf()) * (i as f64) / (SAMPLES as f64 - 1.0);
                let v = vv.inf() + (vv.sup() - vv.inf()) * (j as f64) / (SAMPLES as f64 - 1.0);
                let sampled = surface.uder(u, v).cross(surface.vder(u, v)).magnitude();
                assert!(
                    lb <= sampled,
                    "bound {lb} exceeds sampled ‖S_u × S_v‖ = {sampled} at ({u},{v})"
                );
            }
        }
    }

    #[test]
    fn revolved_enclosure_converges_under_bisection() {
        // The cylinder is an immersion everywhere, so the harness's point
        // convergence applies (BG-ENC-002) from a moderate box to depth ~20.
        let surface = revolved_cylinder();
        let uu = iv(0.0, 1.0);
        let vv = iv(0.3, 1.7);
        let initial = surface.enclose(uu, vv).width();
        assert_converges(&surface, uu, vv, initial, 20);
    }
}
