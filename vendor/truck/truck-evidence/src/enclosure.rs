//! BG-ENC-001 — the enclosure interface.
//!
//! A parallel interface, not a rewrite: the existing `f64` traits survive
//! untouched as the fast path. Every certified quantity in the formal system is
//! an enclosure over a box, so every carrier needs these.
//!
//! **BG-ENC-001 (Soundness):** for every carrier and every box,
//! `enclose(box) ⊇ { f(p) : p ∈ box }`. Over-estimation is always acceptable;
//! **under-estimation is a silent-wrong-answer bug** and invalidates every
//! certificate built on top of it.
//!
//! **BG-ENC-002 (Convergence):** `width(enclose(box)) → 0` as `width(box) → 0`.
//!
//! **BG-ENC-003 (Outward rounding):** all interval arithmetic rounds outward.
//! Never compile enclosure code with fast-math or FMA contraction that could
//! round inward. (inari is compiled with `-Ctarget-feature=+avx,+fma` on x86_64
//! for its directed-rounding primitives; rustc does not contract `a*b+c` into
//! FMA without fast-math, so float results remain bit-identical.)

pub use inari::Interval;
use truck_base::cgmath64::{InnerSpace, Point3, Vector3};
use truck_geometry::nurbs::BSplineCurve;
use truck_geometry::specifieds::Plane;
use truck_geotrait::{ParametricCurve, ParametricSurface};

/// An axis-aligned box in 3-space, each coordinate an outward-rounded interval.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Box3 {
    /// x-coordinate enclosure.
    pub x: Interval,
    /// y-coordinate enclosure.
    pub y: Interval,
    /// z-coordinate enclosure.
    pub z: Interval,
}

impl Box3 {
    /// The empty box (NaN on every axis).
    pub fn empty() -> Self {
        Self {
            x: Interval::EMPTY,
            y: Interval::EMPTY,
            z: Interval::EMPTY,
        }
    }

    /// The degenerate box at a point. Finite coordinates always construct
    /// successfully; a NaN coordinate widens to the empty interval rather than
    /// panicking (H-1).
    pub fn point(p: Point3) -> Self {
        let from = |x: f64| Interval::try_from((x, x)).unwrap_or(Interval::EMPTY);
        Self {
            x: from(p.x),
            y: from(p.y),
            z: from(p.z),
        }
    }

    /// Tests whether a point lies inside every coordinate interval.
    pub fn contains(&self, p: Point3) -> bool {
        self.x.contains(p.x) && self.y.contains(p.y) && self.z.contains(p.z)
    }

    /// The width of the widest coordinate interval (0 for a point).
    pub fn width(&self) -> f64 {
        let wx = self.x.sup() - self.x.inf();
        let wy = self.y.sup() - self.y.inf();
        let wz = self.z.sup() - self.z.inf();
        wx.max(wy).max(wz)
    }
}

/// An enclosure of a set of unit directions: an axis plus a half-angle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DirCone {
    /// The cone axis (a unit vector).
    pub axis: Vector3,
    /// The half-angle of the cone.
    pub half_angle: f64,
}

/// A degenerate interval from a runtime `f64`. Finite coordinates always
/// construct; a NaN widens to the empty interval rather than panicking (H-1).
pub(crate) fn interval_at(x: f64) -> Interval {
    Interval::try_from((x, x)).unwrap_or(Interval::EMPTY)
}

/// The interval cross product of two boxes, written out componentwise.
///
/// Sound but loose: it encloses `{ p x q : p in a, q in b }`, a superset of
/// `{ S_u(x) x S_v(x) : x in box }` because it lets `p` and `q` vary
/// independently where in truth they are evaluated at the same parameter
/// point. Over-estimation is always acceptable (BG-ENC-001).
pub(crate) fn cross_box(a: &Box3, b: &Box3) -> Box3 {
    Box3 {
        x: a.y * b.z - a.z * b.y,
        y: a.z * b.x - a.x * b.z,
        z: a.x * b.y - a.y * b.x,
    }
}

/// The midpoint-ball direction cone of a derivative box: `Some(cone)` iff
/// every element of the box lies within a half-angle `asin(rho/cn)` of the
/// box's midpoint direction, with `rho = ‖h‖` rounded up and `cn = ‖c‖`
/// rounded down so the f64 arithmetic cannot make the cone too narrow.
/// `None` when the box may contain the zero vector or straddle enough
/// directions that no cone bounds it — including any singular locus. That
/// arm is the contract, not a convenience.
pub(crate) fn midpoint_ball_cone(n: &Box3) -> Option<DirCone> {
    let c = Vector3::new(n.x.mid(), n.y.mid(), n.z.mid());
    let h = Vector3::new(n.x.wid() / 2.0, n.y.wid() / 2.0, n.z.wid() / 2.0);
    let norm = |x: Interval, y: Interval, z: Interval| (x.sqr() + y.sqr() + z.sqr()).sqrt();
    let rho = norm(interval_at(h.x), interval_at(h.y), interval_at(h.z)).sup();
    let cn = norm(interval_at(c.x), interval_at(c.y), interval_at(c.z)).inf();
    // `cn <= rho` is the packet's `!(cn > rho)` in clippy-clean form; the
    // NaN cases that would otherwise make the negated comparison fire are
    // caught by the finiteness tests, so the two are equivalent.
    if !cn.is_finite() || !rho.is_finite() || cn <= rho {
        return None;
    }
    let axis = c.normalize();
    let half_angle =
        ((rho / cn).asin() * (1.0 + 8.0 * f64::EPSILON) + 8.0 * f64::EPSILON).min(MAX_HALF_ANGLE);
    Some(DirCone { axis, half_angle })
}

/// The smallest `‖·‖` over a derivative box:
/// `sqrt(mig_x² + mig_y² + mig_z²)` — each coordinate attains its mignitude
/// independently, so this is exactly the box's minimum norm, and since the
/// box contains the true set it is a valid lower bound on the true minimum.
/// Computed in inari and read from the LOWER endpoint (a bound one rounding
/// unit too large is a soundness bug, not a tightness one). An empty or
/// overflowing box contributes nothing: `0.0`.
pub(crate) fn immersion_lower_bound_box(n: &Box3) -> f64 {
    let norm = (interval_at(n.x.mig()).sqr()
        + interval_at(n.y.mig()).sqr()
        + interval_at(n.z.mig()).sqr())
    .sqrt();
    let bound = norm.inf();
    if bound.is_finite() {
        bound
    } else {
        0.0
    }
}

/// The whole-sphere clamp for computed half-angles; keeps an ulp-nudged
/// value from exceeding PI.
const MAX_HALF_ANGLE: f64 = core::f64::consts::PI;

/// Certified enclosure interface for parametric curves.
pub trait EnclosureCurve: ParametricCurve<Point = Point3> {
    /// An enclosure of `{ self.subs(t) : t ∈ tt }` (BG-ENC-001).
    fn enclose(&self, tt: Interval) -> Box3;

    /// An enclosure of `{ self.der_n(n, t) : t ∈ tt }`.
    fn enclose_der(&self, n: usize, tt: Interval) -> Box3;

    /// A cone of tangent directions, `None` when the derivative enclosure
    /// contains 0 (direction undefined).
    fn tangent_cone(&self, tt: Interval) -> Option<DirCone>;

    /// This curve exactly represented as a `BSplineCurve<Point3>`, when it is one
    /// — including by exact affine composition of a planar pcurve. `None` for any
    /// curve whose exact representation is not a plain B-spline (circles, NURBS,
    /// lines, general pcurves). Route 1 of BG-CE-002's deviation certificate
    /// builds on this; the default keeps every other carrier on the generic
    /// bisection route.
    fn exact_spline(&self) -> Option<BSplineCurve<Point3>> {
        None
    }
}

/// Certified enclosure interface for parametric surfaces.
pub trait EnclosureSurface: ParametricSurface<Point = Point3> {
    /// An enclosure of `{ self.subs(u, v) : u ∈ uu, v ∈ vv }` (BG-ENC-001).
    fn enclose(&self, uu: Interval, vv: Interval) -> Box3;

    /// An enclosure of `{ self.der_mn(m, n, u, v) : u ∈ uu, v ∈ vv }`.
    fn enclose_der(&self, m: usize, n: usize, uu: Interval, vv: Interval) -> Box3;

    /// A cone of normal directions over the box, `None` when the immersion is
    /// singular somewhere inside it. Drives §9.1's transversality predicate.
    fn normal_cone(&self, uu: Interval, vv: Interval) -> Option<DirCone>;

    /// A lower bound on ‖S_u × S_v‖ over the box (§10 immersion margin ι).
    fn immersion_lower_bound(&self, uu: Interval, vv: Interval) -> f64;

    /// This surface exactly, when it is a `Plane` (the exact affine carrier).
    /// `None` otherwise. Used by `PCurve`'s `exact_spline` to compose a planar
    /// pcurve into a spline exactly.
    fn as_plane(&self) -> Option<&Plane> {
        None
    }
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect on paths reachable from untrusted
// geometry. Unit-test assertions on hand-built witnesses are not such a path.
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use inari::const_interval;

    #[test]
    fn box3_contains_and_width() {
        let b = Box3 {
            x: const_interval!(-1.0, 1.0),
            y: const_interval!(0.0, 2.0),
            z: const_interval!(-0.5, 0.5),
        };
        assert!(b.contains(Point3::new(0.0, 1.0, 0.0)));
        assert!(!b.contains(Point3::new(2.0, 0.0, 0.0)));
        assert_eq!(b.width(), 2.0);
    }

    #[test]
    fn point_box_is_degenerate() {
        let b = Box3::point(Point3::new(1.0, 2.0, 3.0));
        assert_eq!(b.width(), 0.0);
        assert!(b.contains(Point3::new(1.0, 2.0, 3.0)));
    }

    /// Build a test interval, degrading to EMPTY (and failing its assertion)
    /// rather than panicking on a malformed bound.
    fn iv(lo: f64, hi: f64) -> Interval {
        Interval::try_from((lo, hi)).unwrap_or(Interval::EMPTY)
    }

    #[test]
    fn midpoint_ball_cone_contains_off_axis_directions() {
        // A small box around (2, 1, 0.5), well clear of the origin: the cone
        // exists, and every box-corner direction must lie within the reported
        // half-angle of the axis. Compared via dot products against
        // cos(half_angle), with a slack const for float rounding.
        let n = Box3 {
            x: iv(1.9, 2.1),
            y: iv(0.9, 1.1),
            z: iv(0.4, 0.6),
        };
        let cone = midpoint_ball_cone(&n).expect("an off-axis box has a cone");
        let corners = [
            Vector3::new(1.9, 0.9, 0.4),
            Vector3::new(1.9, 0.9, 0.6),
            Vector3::new(1.9, 1.1, 0.4),
            Vector3::new(1.9, 1.1, 0.6),
            Vector3::new(2.1, 0.9, 0.4),
            Vector3::new(2.1, 0.9, 0.6),
            Vector3::new(2.1, 1.1, 0.4),
            Vector3::new(2.1, 1.1, 0.6),
        ];
        const CORNER_SLACK: f64 = 1.0e-12; // H-3: float slack between two direction cosines, not a length
        for corner in corners {
            let d = corner.normalize();
            let cos_angle = cone.axis.dot(d);
            assert!(
                cos_angle >= cone.half_angle.cos() - CORNER_SLACK,
                "corner direction {d:?} escaped the cone (cos {cos_angle})"
            );
        }
    }

    #[test]
    fn midpoint_ball_cone_refuses_when_the_box_straddles_the_origin() {
        // A box symmetric about the origin: the midpoint direction is zero, so
        // cn = 0 <= rho and no cone bounds the directions.
        let straddling = Box3 {
            x: iv(-1.0, 1.0),
            y: iv(-0.5, 0.5),
            z: iv(-2.0, 2.0),
        };
        assert!(midpoint_ball_cone(&straddling).is_none());
        // A box containing the origin off-centre still contains the zero
        // vector, so every direction is in its span and no cone bounds it.
        let containing = Box3 {
            x: iv(-0.1, 0.2),
            y: iv(0.0, 0.5),
            z: iv(-0.3, 0.1),
        };
        assert!(midpoint_ball_cone(&containing).is_none());
        // The empty box has no directions at all.
        assert!(midpoint_ball_cone(&Box3::empty()).is_none());
    }

    #[test]
    fn cross_box_encloses_the_componentwise_formula() {
        // a = (x:[1,2], y:[0,1], z:[-1,1]) and b = (x:[0,1], y:[2,2], z:[1,1]):
        // the exact cross product at every corner combination is enumerable by
        // hand, and the interval cross product must contain all of them.
        let a = Box3 {
            x: iv(1.0, 2.0),
            y: iv(0.0, 1.0),
            z: iv(-1.0, 1.0),
        };
        let b = Box3 {
            x: iv(0.0, 1.0),
            y: iv(2.0, 2.0),
            z: iv(1.0, 1.0),
        };
        let p = cross_box(&a, &b);
        for ax in [1.0, 2.0] {
            for ay in [0.0, 1.0] {
                for az in [-1.0, 1.0] {
                    for bx in [0.0, 1.0] {
                        let (by, bz) = (2.0, 1.0);
                        let cx = ay * bz - az * by;
                        let cy = az * bx - ax * bz;
                        let cz = ax * by - ay * bx;
                        assert!(
                            p.x.contains(cx) && p.y.contains(cy) && p.z.contains(cz),
                            "corner cross product ({cx}, {cy}, {cz}) escaped the box"
                        );
                    }
                }
            }
        }
        // A degenerate input reproduces the schoolbook result exactly:
        // (2,3,1) x (1,0,4) = (3·4 − 1·0, 1·1 − 2·4, 2·0 − 3·1) = (12, −7, −3).
        let d = Box3 {
            x: iv(2.0, 2.0),
            y: iv(3.0, 3.0),
            z: iv(1.0, 1.0),
        };
        let e = Box3 {
            x: iv(1.0, 1.0),
            y: iv(0.0, 0.0),
            z: iv(4.0, 4.0),
        };
        let q = cross_box(&d, &e);
        assert_eq!(q.x, iv(12.0, 12.0));
        assert_eq!(q.y, iv(-7.0, -7.0));
        assert_eq!(q.z, iv(-3.0, -3.0));
    }
}
