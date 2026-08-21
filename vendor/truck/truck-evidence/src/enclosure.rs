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
use truck_base::cgmath64::{Point3, Vector3};
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
}
