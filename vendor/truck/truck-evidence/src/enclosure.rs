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
use truck_base::cgmath64::control_point::ControlPoint;
use truck_base::cgmath64::{InnerSpace, Point3, Vector3};
use truck_base::tolerance::Tolerance;
use truck_geometry::nurbs::{BSplineCurve, BSplineSurface, KnotVec};
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

// ---------------------------------------------------------------------------
// CL-000-SPLINE-ADMIT: BSplineSurface spline-carrier enclosures (additive).
// ---------------------------------------------------------------------------

/// The relative outward pad per hull endpoint (mirrors `bspline.rs`'s HULL_PAD).
const SPLINE_SURFACE_HULL_PAD: f64 = 64.0 * f64::EPSILON;

/// Coordinate access for a control net without `Index` (H-1's
/// `clippy::indexing_slicing` denial bans indexing). `Point3` and `Vector3`
/// both carry `x`, `y`, `z` fields.
trait SurfaceCoord {
    /// The `x` coordinate.
    fn x(self) -> f64;
    /// The `y` coordinate.
    fn y(self) -> f64;
    /// The `z` coordinate.
    fn z(self) -> f64;
}

impl SurfaceCoord for Point3 {
    fn x(self) -> f64 {
        self.x
    }
    fn y(self) -> f64 {
        self.y
    }
    fn z(self) -> f64 {
        self.z
    }
}

impl SurfaceCoord for Vector3 {
    fn x(self) -> f64 {
        self.x
    }
    fn y(self) -> f64 {
        self.y
    }
    fn z(self) -> f64 {
        self.z
    }
}

/// The outward-rounded control-net box of a spline carrier.
///
/// The pad is applied per coordinate relative to the coordinate magnitude,
/// mirroring `bspline.rs`: the control net lives in `f64`, and evaluation of
/// the spline (or its derivative) can land a few ulps outside the raw hull.
/// The returned box therefore contains the whole parameterized image on the
/// spline's clamped domain by the convex-hull property of B-splines, padded
/// outward.
fn control_net_box<P: Copy + SurfaceCoord>(surface: &BSplineSurface<P>) -> Box3 {
    let mut lo_x = f64::INFINITY;
    let mut hi_x = f64::NEG_INFINITY;
    let mut lo_y = f64::INFINITY;
    let mut hi_y = f64::NEG_INFINITY;
    let mut lo_z = f64::INFINITY;
    let mut hi_z = f64::NEG_INFINITY;
    for row in surface.control_points().iter() {
        for &pt in row.iter() {
            let (x, y, z) = (pt.x(), pt.y(), pt.z());
            lo_x = lo_x.min(x);
            hi_x = hi_x.max(x);
            lo_y = lo_y.min(y);
            hi_y = hi_y.max(y);
            lo_z = lo_z.min(z);
            hi_z = hi_z.max(z);
        }
    }
    let expand = |lo: f64, hi: f64| -> Interval {
        let pad = SPLINE_SURFACE_HULL_PAD * (1.0 + lo.abs().max(hi.abs()));
        Interval::try_from((lo - pad, hi + pad)).unwrap_or(Interval::EMPTY)
    };
    Box3 {
        x: expand(lo_x, hi_x),
        y: expand(lo_y, hi_y),
        z: expand(lo_z, hi_z),
    }
}

/// The spline's clamped-interior parameter rectangle (where the B-spline
/// basis is a partition of unity), or `None` when the box reaches outside it
/// (the carrier cannot certify there; the outward answer is the whole line).
fn spline_interior(surface: &BSplineSurface<Point3>) -> Option<((f64, f64), (f64, f64))> {
    let (du, dv) = surface.degrees();
    let (u0, u1) = interior_axis(surface.uknot_vec(), du)?;
    let (v0, v1) = interior_axis(surface.vknot_vec(), dv)?;
    Some(((u0, u1), (v0, v1)))
}

/// The clamped-interior interval of one knot axis.
fn interior_axis(knots: &KnotVec, degree: usize) -> Option<(f64, f64)> {
    let lo = knots.get(degree)?;
    let n = knots.len();
    if n <= degree {
        return None;
    }
    let hi = knots.get(n - 1 - degree)?;
    if lo.is_finite() && hi.is_finite() && lo < hi {
        Some((*lo, *hi))
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// CFP-001-LIFT-ENCLOSURE: the sub-box spline hull primitive
// (NUM-SPLINE-ENCLOSURE-CONVERGENCE-001 correction; BG-ENC-002).
//
// `enclose`/`enclose_der`/`normal_cone` used to return whole-net bounds
// regardless of the query box: sound but non-convergent, so any loop that
// separates on enclosures could not terminate on spline pairs and cones never
// shrank under subdivision. The correction is the shared sub-box spline hull
// the lift screen consumes too: locate the knot spans overlapping the query
// box, de Casteljau-restrict each span's net to the box∩span intersection
// (exact knot manipulation via the landed `KnotVec` insertion ops — no new
// spline math), and union the per-span hulls. The F-C7 cross-check test guards
// the duplicated leaf math (F1).
// ---------------------------------------------------------------------------

/// The multiplicity of the knot value `x` in a knot vector, counted over exact
/// knot equality. `KnotVec::multiplicity` matches by tolerance and would count
/// a *different* knot value within the legacy tolerance of `x`, which
/// under-inserts in the raising loop and extracts an over-wide sub-surface
/// whenever `x` sits within tolerance of another knot (the `bspline.rs` exact
/// count, applied to surfaces).
fn exact_knot_multiplicity(knots: &KnotVec, x: f64) -> usize {
    knots.iter().filter(|&&k| k == x).count()
}

/// Raises the u-knot value `x` to full multiplicity `degree + 1` by repeated
/// exact Boehm insertion. `add_uknot` inserts a single exact copy and never
/// validates; inserting past `degree + 1` would make an invalid knot vector,
/// so the loop stops exactly at the maximum multiplicity.
fn raise_uknot_full<P: ControlPoint<f64> + Tolerance + Clone>(
    surface: &mut BSplineSurface<P>,
    x: f64,
    degree: usize,
) {
    while exact_knot_multiplicity(surface.uknot_vec(), x) < degree + 1 {
        surface.add_uknot(x);
    }
}

/// Raises the v-knot value `x` to full multiplicity `degree + 1` by repeated
/// exact Boehm insertion (the v-axis sibling of [`raise_uknot_full`]).
fn raise_vknot_full<P: ControlPoint<f64> + Tolerance + Clone>(
    surface: &mut BSplineSurface<P>,
    x: f64,
    degree: usize,
) {
    while exact_knot_multiplicity(surface.vknot_vec(), x) < degree + 1 {
        surface.add_vknot(x);
    }
}

/// The sub-surface over the rectangle `[u_lo, u_hi] × [v_lo, v_hi]`, where
/// `u_lo < u_hi` and `v_lo < v_hi` are already clamped into the carrier's knot
/// range (the per-span restriction primitive; `bspline.rs`'s `sub_curve`,
/// lifted to two axes). Each endpoint is first raised to full knot multiplicity
/// so that `ucut`/`vcut`'s tolerance snapping is exact — `x − x == 0.0`, so the
/// cut inserts no further copies — and then the surface is cut at `u_hi`
/// (keeping the front) and at `u_lo` (returning the middle), then likewise
/// along `v`. Over that rectangle the extracted surface's basis functions are
/// non-negative and sum to 1, so every image point is a convex combination of
/// the sub-surface's control points: its outward-rounded axis-aligned box is an
/// enclosure (the convex-hull property).
fn restrict_surface<P: ControlPoint<f64> + Tolerance + Clone>(
    surface: &BSplineSurface<P>,
    u_lo: f64,
    u_hi: f64,
    v_lo: f64,
    v_hi: f64,
) -> BSplineSurface<P> {
    let u_degree = surface.udegree();
    let v_degree = surface.vdegree();
    let mut s = surface.clone();
    for x in [u_lo, u_hi] {
        raise_uknot_full(&mut s, x, u_degree);
    }
    let _tail_u = s.ucut(u_hi);
    let mut s = s.ucut(u_lo);
    for y in [v_lo, v_hi] {
        raise_vknot_full(&mut s, y, v_degree);
    }
    let _tail_v = s.vcut(v_hi);
    s.vcut(v_lo)
}

/// The positive-width intersections of `[lo, hi]` with the non-degenerate knot
/// spans of one axis, in increasing order (determinism: low-before-high, exact
/// knot comparison). A span whose clipped width is zero contributes nothing —
/// the shared edge belongs to the neighbouring span.
fn span_clips(knots: &KnotVec, lo: f64, hi: f64) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    let mut prev: Option<f64> = None;
    for &k in knots.iter() {
        if let Some(p) = prev {
            if p < k {
                let a = p.max(lo);
                let b = k.min(hi);
                if a < b {
                    out.push((a, b));
                }
            }
        }
        prev = Some(k);
    }
    out
}

/// The outward-rounded union of two boxes: per coordinate the lower endpoint
/// is the min of the two lowers and the upper endpoint the max of the two
/// uppers, so the union encloses both. `min`/`max` on already outward-rounded
/// endpoints stays outward (BG-ENC-003).
fn union_box(a: &Box3, b: &Box3) -> Box3 {
    let union = |x: Interval, y: Interval| -> Interval {
        Interval::try_from((x.inf().min(y.inf()), x.sup().max(y.sup()))).unwrap_or(Interval::EMPTY)
    };
    Box3 {
        x: union(a.x, b.x),
        y: union(a.y, b.y),
        z: union(a.z, b.z),
    }
}

/// The sub-box spline hull of a spline carrier over an in-domain query box:
/// the union over every knot span overlapping the box of the per-span
/// restriction's control-net hull (BG-ENC-002). Empty when the box clips no
/// positive span (a fully degenerate box).
fn sub_box_hull<P: ControlPoint<f64> + Tolerance + Copy + Clone + SurfaceCoord>(
    surface: &BSplineSurface<P>,
    uu: Interval,
    vv: Interval,
) -> Box3 {
    let u_clips = span_clips(surface.uknot_vec(), uu.inf(), uu.sup());
    let v_clips = span_clips(surface.vknot_vec(), vv.inf(), vv.sup());
    let mut acc = Box3::empty();
    let mut any = false;
    for (u_lo, u_hi) in u_clips {
        for &(v_lo, v_hi) in &v_clips {
            let sub = restrict_surface(surface, u_lo, u_hi, v_lo, v_hi);
            let b = control_net_box(&sub);
            acc = if any { union_box(&acc, &b) } else { b };
            any = true;
        }
    }
    if any {
        acc
    } else {
        Box3::empty()
    }
}

impl EnclosureSurface for BSplineSurface<Point3> {
    fn enclose(&self, uu: Interval, vv: Interval) -> Box3 {
        if uu.is_empty() || vv.is_empty() || !uu.inf().is_finite() || !uu.sup().is_finite() {
            return Box3::empty();
        }
        let Some(((u0, u1), (v0, v1))) = spline_interior(self) else {
            return Box3 {
                x: Interval::ENTIRE,
                y: Interval::ENTIRE,
                z: Interval::ENTIRE,
            };
        };
        if uu.inf() < u0 || uu.sup() > u1 || vv.inf() < v0 || vv.sup() > v1 {
            return Box3 {
                x: Interval::ENTIRE,
                y: Interval::ENTIRE,
                z: Interval::ENTIRE,
            };
        }
        // CFP-001: a query box spanning the whole clamped domain returns the
        // whole control-net hull (the landed whole-domain answer, a superset of
        // the refined per-span union); every strictly-smaller box returns the
        // sub-box spline hull, whose width → 0 as the box → 0 (BG-ENC-002).
        if uu.inf() == u0 && uu.sup() == u1 && vv.inf() == v0 && vv.sup() == v1 {
            return control_net_box(self);
        }
        sub_box_hull(self, uu, vv)
    }

    fn enclose_der(&self, m: usize, n: usize, uu: Interval, vv: Interval) -> Box3 {
        if uu.is_empty() || vv.is_empty() || !uu.inf().is_finite() || !uu.sup().is_finite() {
            return Box3::empty();
        }
        let Some(((u0, u1), (v0, v1))) = spline_interior(self) else {
            return Box3 {
                x: Interval::ENTIRE,
                y: Interval::ENTIRE,
                z: Interval::ENTIRE,
            };
        };
        if uu.inf() < u0 || uu.sup() > u1 || vv.inf() < v0 || vv.sup() > v1 {
            return Box3 {
                x: Interval::ENTIRE,
                y: Interval::ENTIRE,
                z: Interval::ENTIRE,
            };
        }
        // CL-000: the derivative of a B-spline surface is a B-spline surface
        // of degree k - 1 whose control points are derived exactly from the
        // carrier's net (truck-geometry's `uderivation` / `vderivation`). The
        // derivative image is bounded by the derived net's sub-box spline hull
        // over the same query box (BG-ENC-002, the convergence the defect
        // record demands).
        let derived: BSplineSurface<Vector3> = match uderive_chain(self, m, n) {
            Some(d) => d,
            None => {
                // (0, 0): the surface itself.
                return self.enclose(uu, vv);
            }
        };
        if uu.inf() == u0 && uu.sup() == u1 && vv.inf() == v0 && vv.sup() == v1 {
            return control_net_box(&derived);
        }
        sub_box_hull(&derived, uu, vv)
    }

    fn normal_cone(&self, uu: Interval, vv: Interval) -> Option<DirCone> {
        let su = self.enclose_der(1, 0, uu, vv);
        let sv = self.enclose_der(0, 1, uu, vv);
        let normal_box = cross_box(&su, &sv);
        midpoint_ball_cone(&normal_box)
    }

    fn immersion_lower_bound(&self, uu: Interval, vv: Interval) -> f64 {
        let su = self.enclose_der(1, 0, uu, vv);
        let sv = self.enclose_der(0, 1, uu, vv);
        let normal_box = cross_box(&su, &sv);
        immersion_lower_bound_box(&normal_box)
    }
}

/// Apply `m` u-derivations then `n` v-derivations to a spline surface,
/// yielding the derivative surface. Returns `None` for order (0, 0).
fn uderive_chain(
    surface: &BSplineSurface<Point3>,
    m: usize,
    n: usize,
) -> Option<BSplineSurface<Vector3>> {
    if m == 0 && n == 0 {
        return None;
    }
    // truck-geometry BSplineSurface::uderivation/vderivation reduce the
    // degree on the derived axis and return a surface whose control points
    // are the exact derivative coefficients.
    let mut derived: BSplineSurface<Vector3> = if m > 0 {
        surface.uderivation()
    } else {
        surface.vderivation()
    };
    for _ in 1..m {
        derived = derived.uderivation();
    }
    for _ in 1..n {
        derived = derived.vderivation();
    }
    Some(derived)
}

/// Certified enclosure interface for vector-valued parametric fields
/// (`Point = Vector3`), the companion of [`EnclosureSurface`] for the
/// `Offset<S, N>` decorator (BG-ENC-004-OFFSET). `N` in `Offset<T, N>` is
/// *vector*-valued, so it can never satisfy `EnclosureSurface`'s
/// `Point = Point3` bound; this trait is `EnclosureSurface` minus that bound.
///
/// A cone of directions is deliberately *not* part of the interface: whenever
/// a tight path needs one it is derivable from the field's own enclosure box
/// via [`midpoint_ball_cone`]. The composition needs only the two enclosure
/// methods.
pub trait EnclosureVectorField: ParametricSurface<Point = Vector3, Vector = Vector3> {
    /// MUST contain `{ self.subs(u, v) : (u,v) ∈ uu×vv }` (BG-ENC-001).
    fn enclose(&self, uu: Interval, vv: Interval) -> Box3;

    /// MUST contain `{ self.der_mn(m, n, u, v) : (u,v) ∈ uu×vv }`.
    fn enclose_der(&self, m: usize, n: usize, uu: Interval, vv: Interval) -> Box3;
}

/// Certified enclosure interface for two-variable scalar fields — the `F` in
/// `NormalField<S, F>` (BG-ENC-004-OFFSET). No supertrait: the constant case
/// (`f64`) is v1's only impl, and a variable-distance scalar field gets an
/// impl only when a carrier needs one.
pub trait EnclosureScalarField2 {
    /// MUST contain `{ self.subs(u, v) : (u,v) ∈ uu×vv }`.
    fn enclose(&self, uu: Interval, vv: Interval) -> Interval;

    /// MUST contain `{ self.der_mn(m, n, u, v) : (u,v) ∈ uu×vv }`.
    fn enclose_der(&self, m: usize, n: usize, uu: Interval, vv: Interval) -> Interval;
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

    /// A clamped uniform spline surface of bidegree `(du, dv)` with a few
    /// interior knot spans, used as a spline-carrier witness.
    fn spline_witness(du: usize, dv: usize) -> BSplineSurface<Point3> {
        let uknot = KnotVec::uniform_knot(du, 4);
        let vknot = KnotVec::uniform_knot(dv, 4);
        let nu = uknot.len() - du - 1;
        let nv = vknot.len() - dv - 1;
        let ctrl = (0..nu)
            .map(|i| {
                (0..nv)
                    .map(|j| {
                        let u = i as f64 / nu as f64;
                        let v = j as f64 / nv as f64;
                        Point3::new(
                            u,
                            v,
                            0.1 * u * u + 0.2 * v * v + 0.05 * u * v + (i as f64).cos() * 0.01,
                        )
                    })
                    .collect()
            })
            .collect();
        BSplineSurface::new((uknot, vknot), ctrl)
    }

    /// CL-000 required test: for at least three surfaces the first-partial
    /// derivative enclosures bracket a thousand brute samples per axis on an
    /// interior box.
    #[test]
    fn bspline_enclosure_brackets_brute_sample() {
        let surfaces = [
            spline_witness(1, 1),
            spline_witness(2, 2),
            spline_witness(3, 3),
        ];
        const SAMPLES: usize = 1000;
        for surface in surfaces.iter() {
            let uu = iv(0.2, 0.8);
            let vv = iv(0.2, 0.8);
            let e_du = surface.enclose_der(1, 0, uu, vv);
            let e_dv = surface.enclose_der(0, 1, uu, vv);
            for i in 0..SAMPLES {
                let u = uu.inf() + (uu.sup() - uu.inf()) * (i as f64) / (SAMPLES as f64 - 1.0);
                let v = vv.inf() + (vv.sup() - vv.inf()) * (i as f64) / (SAMPLES as f64 - 1.0);
                let du = surface.uder(u, v);
                let dv = surface.vder(u, v);
                assert!(
                    e_du.contains(Point3::new(du.x, du.y, du.z)),
                    "u-derivative at ({u}, {v}) escaped the enclosure"
                );
                assert!(
                    e_dv.contains(Point3::new(dv.x, dv.y, dv.z)),
                    "v-derivative at ({u}, {v}) escaped the enclosure"
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // CFP-001 fixture data, copied from `truck-certified/src/cfp/fixtures.rs`
    // as read-only constants (F1 — never import the module). The evaluation
    // assertions the fixtures record as expectations land HERE (the parallel
    // contract): F-C2's containment/convergence/monotonicity against the
    // landed `BSplineSurface` enclosures, and F-C7's evidence-side hull against
    // a locally-reproduced certified-side per-span hull.
    // -----------------------------------------------------------------------

    /// The F-C2 fixed bicubic net `z = i·j` over the `4 × 4` grid, verbatim.
    const FC2_NET: [[[f64; 3]; 4]; 4] = [
        [
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 2.0, 0.0],
            [0.0, 3.0, 0.0],
        ],
        [
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 1.0],
            [1.0, 2.0, 2.0],
            [1.0, 3.0, 3.0],
        ],
        [
            [2.0, 0.0, 0.0],
            [2.0, 1.0, 2.0],
            [2.0, 2.0, 4.0],
            [2.0, 3.0, 6.0],
        ],
        [
            [3.0, 0.0, 0.0],
            [3.0, 1.0, 3.0],
            [3.0, 2.0, 6.0],
            [3.0, 3.0, 9.0],
        ],
    ];
    /// The F-C2 outer box `B₁`.
    const FC2_B1: ((f64, f64), (f64, f64)) = ((0.0, 1.0), (0.0, 1.0));
    /// The F-C2 middle box `B₂ ⊂ B₁`.
    const FC2_B2: ((f64, f64), (f64, f64)) = ((0.25, 0.75), (0.25, 0.75));
    /// The F-C2 inner box `B₃ ⊂ B₂`.
    const FC2_B3: ((f64, f64), (f64, f64)) = ((0.4375, 0.5625), (0.4375, 0.5625));

    /// The F-C2 battery surface: the fixed bicubic net over the clamped unit
    /// square (one Bézier span per axis; the F-C2 boxes are nested sub-patches
    /// of that single span).
    fn battery_surface() -> BSplineSurface<Point3> {
        let ctrl: Vec<Vec<Point3>> = FC2_NET
            .iter()
            .map(|row| row.iter().map(|&p| Point3::new(p[0], p[1], p[2])).collect())
            .collect();
        BSplineSurface::new((KnotVec::bezier_knot(3), KnotVec::bezier_knot(3)), ctrl)
    }

    /// A recorded `((f64, f64), (f64, f64))` box as the enclosure intervals.
    fn recorded_box_iv(b: ((f64, f64), (f64, f64))) -> (Interval, Interval) {
        (iv(b.0 .0, b.0 .1), iv(b.1 .0, b.1 .1))
    }

    /// Asserts that `inner` lies inside `outer` coordinate-wise up to a
    /// `HULL_PAD`-scale outward slack (the box comparison used by the
    /// monotonicity and cross-check assertions; the slack absorbs the last-ulp
    /// float drift between independently-restricted control nets, never a real
    /// gap).
    fn assert_box_within(inner: &Box3, outer: &Box3, what: &str) {
        let slack = |x: f64| 256.0 * f64::EPSILON * (1.0 + x.abs());
        for (axis, i, o) in [
            ("x", inner.x, outer.x),
            ("y", inner.y, outer.y),
            ("z", inner.z, outer.z),
        ] {
            assert!(
                i.inf() >= o.inf() - slack(o.inf()) && i.sup() <= o.sup() + slack(o.sup()),
                "{what}: {axis} [{}, {}] escapes [{}, {}]",
                i.inf(),
                i.sup(),
                o.inf(),
                o.sup()
            );
        }
    }

    /// F-C2 (BG-ENC-001): on the fixed battery surface, a dense point sampling
    /// of each recorded box's image — the surface itself and both first
    /// partials — is contained in the corresponding enclosure.
    #[test]
    fn fc2_enclosure_containment_randomized() {
        let surface = battery_surface();
        const SAMPLES: usize = 12;
        for (b, name) in [(FC2_B1, "B1"), (FC2_B2, "B2"), (FC2_B3, "B3")] {
            let (uu, vv) = recorded_box_iv(b);
            let enc = surface.enclose(uu, vv);
            let e_du = surface.enclose_der(1, 0, uu, vv);
            let e_dv = surface.enclose_der(0, 1, uu, vv);
            for i in 0..SAMPLES {
                for j in 0..SAMPLES {
                    let u = b.0 .0 + (b.0 .1 - b.0 .0) * (i as f64) / (SAMPLES as f64 - 1.0);
                    let v = b.1 .0 + (b.1 .1 - b.1 .0) * (j as f64) / (SAMPLES as f64 - 1.0);
                    let p = surface.subs(u, v);
                    assert!(
                        enc.contains(p),
                        "{name}: surface point at ({u}, {v}) escaped"
                    );
                    let du = surface.uder(u, v);
                    assert!(
                        e_du.contains(Point3::new(du.x, du.y, du.z)),
                        "{name}: u-derivative at ({u}, {v}) escaped"
                    );
                    let dv = surface.vder(u, v);
                    assert!(
                        e_dv.contains(Point3::new(dv.x, dv.y, dv.z)),
                        "{name}: v-derivative at ({u}, {v}) escaped"
                    );
                }
            }
        }
    }

    /// F-C2 (BG-ENC-002, the permanent guard on
    /// `NUM-SPLINE-ENCLOSURE-CONVERGENCE-001`): the enclosure width strictly
    /// decreases down the recorded nest `B₁ ⊃ B₂ ⊃ B₃` — the width is no
    /// longer constant in the query box.
    #[test]
    fn fc2_enclosure_convergence_under_bisection() {
        let surface = battery_surface();
        let (u1, v1) = recorded_box_iv(FC2_B1);
        let (u2, v2) = recorded_box_iv(FC2_B2);
        let (u3, v3) = recorded_box_iv(FC2_B3);
        let w1 = surface.enclose(u1, v1).width();
        let w2 = surface.enclose(u2, v2).width();
        let w3 = surface.enclose(u3, v3).width();
        let slack = |w: f64| 256.0 * f64::EPSILON * (1.0 + w);
        assert!(
            w2 <= w1 + slack(w1),
            "width not non-increasing down the nest: {w1} -> {w2}"
        );
        assert!(
            w3 <= w2 + slack(w2),
            "width not non-increasing down the nest: {w2} -> {w3}"
        );
        assert!(
            w3 < w1,
            "width did not strictly decrease down the nest: {w1} -> {w3}"
        );
    }

    /// F-C2 (monotonicity): `B ⊆ B′ ⇒ enclose(B) ⊆ enclose(B′)` on the
    /// recorded nest.
    #[test]
    fn fc2_enclosure_monotonicity() {
        let surface = battery_surface();
        let (u1, v1) = recorded_box_iv(FC2_B1);
        let (u2, v2) = recorded_box_iv(FC2_B2);
        let (u3, v3) = recorded_box_iv(FC2_B3);
        let e1 = surface.enclose(u1, v1);
        let e2 = surface.enclose(u2, v2);
        let e3 = surface.enclose(u3, v3);
        assert_box_within(&e2, &e1, "enclose(B2) ⊆ enclose(B1)");
        assert_box_within(&e3, &e2, "enclose(B3) ⊆ enclose(B2)");
    }

    /// One F-C7 cross-check input: a bicubic net and a sub-box of its domain.
    struct Fc7Input {
        /// The `4 × 4` spline control net.
        net: [[[f64; 3]; 4]; 4],
        /// The parameter sub-box `(u, v)`.
        box_: ((f64, f64), (f64, f64)),
    }

    /// The F-C7 fixed seed, verbatim.
    const FC7_SEED: u64 = 0x5EED_C700;
    /// The F-C7 input count, verbatim.
    const FC7_COUNT: usize = 8;

    /// The F-C7 LCG step (the fixtures' generator, verbatim).
    fn fc7_next(state: &mut u64) -> f64 {
        *state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((*state >> 11) & 0xFFFF) as f64 / 65_535.0
    }

    /// The fixtures' `ordered_pair`: an ordered pair, degenerate draws falling
    /// back to the unit interval (deterministic).
    fn fc7_ordered_pair(a: f64, b: f64) -> (f64, f64) {
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        if lo < hi {
            (lo, hi)
        } else {
            (0.0, 1.0)
        }
    }

    /// Regenerate the F-C7 seeded inputs (the fixtures' generator, verbatim).
    fn fc7_inputs() -> Vec<Fc7Input> {
        let mut state = FC7_SEED;
        let mut inputs = Vec::with_capacity(FC7_COUNT);
        for _ in 0..FC7_COUNT {
            let mut net = [[[0.0f64; 3]; 4]; 4];
            for point in net.iter_mut().flatten() {
                for component in point.iter_mut() {
                    *component = fc7_next(&mut state) * 3.0;
                }
            }
            let box_ = (
                fc7_ordered_pair(fc7_next(&mut state), fc7_next(&mut state)),
                fc7_ordered_pair(fc7_next(&mut state), fc7_next(&mut state)),
            );
            inputs.push(Fc7Input { net, box_ });
        }
        inputs
    }

    /// The F-C7 surface over one input's net, clamped over the unit square.
    fn fc7_surface(input: &Fc7Input) -> BSplineSurface<Point3> {
        let ctrl: Vec<Vec<Point3>> = input
            .net
            .iter()
            .map(|row| row.iter().map(|&p| Point3::new(p[0], p[1], p[2])).collect())
            .collect();
        BSplineSurface::new((KnotVec::bezier_knot(3), KnotVec::bezier_knot(3)), ctrl)
    }

    /// The distinct knot values strictly between `lo` and `hi`, in order.
    fn interior_knot_values(knots: &KnotVec, lo: f64, hi: f64) -> Vec<f64> {
        let mut out = Vec::new();
        let mut prev: Option<f64> = None;
        for &k in knots.iter() {
            if let Some(p) = prev {
                if p != k && lo < k && k < hi {
                    out.push(k);
                }
            }
            prev = Some(k);
        }
        out
    }

    /// The certified-side per-span hull (the `SplinePatchStack` view of the F1
    /// duplication): admit the surface into its per-span Bézier patches once
    /// (D1 — every interior knot raised to full multiplicity), then restrict
    /// each span overlapping the box to the box∩span intersection and union the
    /// per-span control-net hulls. Structurally the certified admission +
    /// restriction CFP-003 performs on `AdmittedPatch`s; reproduced here so the
    /// cross-check lives on the evidence side of the F1 edge.
    fn certified_per_span_hull(
        surface: &BSplineSurface<Point3>,
        uu: Interval,
        vv: Interval,
    ) -> Box3 {
        let Some(((u0, u1), (v0, v1))) = spline_interior(surface) else {
            return Box3 {
                x: Interval::ENTIRE,
                y: Interval::ENTIRE,
                z: Interval::ENTIRE,
            };
        };
        let mut admitted = surface.clone();
        for x in interior_knot_values(surface.uknot_vec(), u0, u1) {
            let degree = admitted.udegree();
            raise_uknot_full(&mut admitted, x, degree);
        }
        for y in interior_knot_values(surface.vknot_vec(), v0, v1) {
            let degree = admitted.vdegree();
            raise_vknot_full(&mut admitted, y, degree);
        }
        sub_box_hull(&admitted, uu, vv)
    }

    /// F-C7 (F1): the evidence-side sub-box hull (`BSplineSurface::enclose`)
    /// agrees with the certified-side per-span hull on the seeded inputs.
    #[test]
    fn fc7_cross_check_evidence_hull_matches_certified_side() {
        for (idx, input) in fc7_inputs().iter().enumerate() {
            let surface = fc7_surface(input);
            let (uu, vv) = recorded_box_iv(input.box_);
            let evidence = surface.enclose(uu, vv);
            let certified = certified_per_span_hull(&surface, uu, vv);
            assert_box_within(&evidence, &certified, "evidence hull ⊆ certified-side hull");
            assert_box_within(&certified, &evidence, "certified-side hull ⊆ evidence hull");
            // Both hulls enclose the sampled image (BG-ENC-001 sanity on the
            // cross-check inputs).
            const SAMPLES: usize = 8;
            let b = input.box_;
            for i in 0..SAMPLES {
                for j in 0..SAMPLES {
                    let u = b.0 .0 + (b.0 .1 - b.0 .0) * (i as f64) / (SAMPLES as f64 - 1.0);
                    let v = b.1 .0 + (b.1 .1 - b.1 .0) * (j as f64) / (SAMPLES as f64 - 1.0);
                    let p = surface.subs(u, v);
                    assert!(
                        evidence.contains(p),
                        "input {idx}: point escaped the evidence hull"
                    );
                    assert!(
                        certified.contains(p),
                        "input {idx}: point escaped the certified-side hull"
                    );
                }
            }
        }
    }
}
