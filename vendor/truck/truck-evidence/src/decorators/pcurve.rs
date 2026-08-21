//! BG-ENC-004-PCURVE: `EnclosureCurve` for `PCurve<BSplineCurve<Point2>, S>`.
//!
//! `PCurve<C, S>` is a curve that lives in a surface's parameter space:
//! `subs(t) = S(C(t))`, with the parameter curve `C` a 2D B-spline and `S` an
//! `EnclosureSurface`. A decorator's enclosure is a **composition**, never a
//! re-derivation: it calls the inner carriers' `enclose` / `enclose_der` and
//! combines the boxes (BG-ENC-004). Here the composition has two levels:
//!
//! 1. **Hull the parameter curve in 2D.** The parameter curve is a
//!    `BSplineCurve<Point2>`, hulled by the same convex-hull machinery
//!    `bspline.rs` lands for `Point3` — sub-curve extraction by Boehm knot
//!    insertion, control-point min/max, `HULL_PAD (1 + |·|)` outward pad,
//!    boundary values, out-of-range → unbounded — just over two coordinates
//!    and producing a parameter box `(uu, vv)` instead of a `Box3`. The helper
//!    set is deliberately duplicated from `bspline.rs` (the sibling packets
//!    have disjoint write sets and run in parallel).
//! 2. **Take the surface's enclosure over that parameter box.** The hulled
//!    parameter image contains every `c(t)`, so by BG-ENC-001 of the inner
//!    carrier the surface's box over the hull contains every `S(c(t))`:
//!
//!    ```text
//!    { S(c(t)) : t ∈ tt } ⊆ { S(u, v) : (u, v) ∈ uu × vv } ⊆ surface.enclose(uu, vv)
//!    ```
//!
//! The derivative boxes compose by the **chain rule** in inari over the
//! surface's `enclose_der` boxes and the parameter hodographs' hulls — exactly
//! the forms the carrier's `der` / `der2` / `der3` spell in truck-geometry.
//! Every box over-estimates its true set and interval arithmetic is monotone,
//! so each composed `Dn` over-estimates `{ der_n(t) : t ∈ tt }`; the
//! decorrelation over-estimation grows with `n` (acceptable, BG-ENC-001
//! permits over-estimation). Fourth and higher orders are the unbounded box:
//! the fourth-order chain rule is Faà di Bruno over surface partials, no
//! kernel consumer asks past third order, and a sound widest box is the honest
//! answer rather than an unverified formula.
//!
//! `tangent_cone` is the ball-around-midpoint cone off the `n = 1` box, the
//! identical construction `bspline.rs` lands for the hodograph hull. `None` is
//! the derivative-hull-contains-zero case — here the parameter curve's
//! velocity vanishing (a cusp in parameter space) or both surface partials
//! degenerating at a pole.

use crate::enclosure::{Box3, DirCone, EnclosureCurve, EnclosureSurface};
use inari::Interval;
use truck_base::cgmath64::control_point::ControlPoint;
use truck_base::cgmath64::{InnerSpace, Point2, Point3, Vector2, Vector3};
use truck_base::tolerance::Tolerance;
use truck_geometry::decorators::PCurve;
use truck_geometry::nurbs::BSplineCurve;
use truck_geotrait::{Cut, ParametricCurve};

/// The largest half-angle a [`DirCone`] can report. `asin` saturates at π/2,
/// but the ulp nudge that guards the cone against inward rounding can push a
/// near-π/2 half-angle slightly past it, so the clamp keeps the half-angle
/// within its meaning as an angle. Copied from `bspline.rs`.
const MAX_HALF_ANGLE: f64 = core::f64::consts::PI;

/// The relative outward pad per hull endpoint, as a multiple of `EPSILON`.
/// Copied from `bspline.rs`: Boehm insertion and `cut` recompute control
/// points in `f64`, so the extracted sub-curve's control points are perturbed
/// relative to the source curve's by several ulps; `64 EPSILON (1 + magnitude)`
/// covers the measured escapes with margin.
const HULL_PAD: f64 = 64.0 * f64::EPSILON;

/// A degenerate interval from a runtime `f64`. Finite coordinates always
/// construct; a NaN widens to the empty interval rather than panicking (H-1).
fn interval_at(x: f64) -> Interval {
    Interval::try_from((x, x)).unwrap_or(Interval::EMPTY)
}

/// Coordinate access without `Index`, which H-1's `clippy::indexing_slicing`
/// denial bans. Both `Point2` and `Vector2` are `ControlPoint<f64>` and carry
/// their coordinates as fields — `0..=1` by `.x`, `.y` — and the same fields
/// are read for the hodographs' vector control points.
trait Coord: ControlPoint<f64> {
    /// The `i`-th coordinate, `0..=1`.
    fn coord(self, i: usize) -> f64;
}

impl Coord for Point2 {
    fn coord(self, i: usize) -> f64 {
        match i {
            0 => self.x,
            _ => self.y,
        }
    }
}

impl Coord for Vector2 {
    fn coord(self, i: usize) -> f64 {
        match i {
            0 => self.x,
            _ => self.y,
        }
    }
}

/// The parameter curve's knot range, the domain over which the basis is a
/// partition of unity. A valid `BSplineCurve` always has non-empty knots; the
/// `None` arm is total-behaviour defence (H-1), matching `bspline.rs`'s
/// `hull_of`.
fn knot_range<P: ControlPoint<f64>>(bsp: &BSplineCurve<P>) -> Option<(f64, f64)> {
    match (bsp.knot_vec().first(), bsp.knot_vec().last()) {
        (Some(&lo), Some(&hi)) => Some((lo, hi)),
        _ => None,
    }
}

/// The multiplicity of the knot value `x` in `bsp`'s knot vector, counted over
/// **exact** knot equality. `KnotVec::multiplicity` matches by tolerance and
/// would count a *different* knot value within the legacy tolerance of `x`,
/// which under-inserts in the raising loop and extracts an over-wide sub-curve
/// whenever `x` sits within tolerance of another knot (the terminal strip of
/// every knot range). Copied from `bspline.rs`.
fn knot_multiplicity<P: ControlPoint<f64>>(bsp: &BSplineCurve<P>, x: f64) -> usize {
    bsp.knot_vec().iter().filter(|&&k| k == x).count()
}

/// Raises the knot value `x` to full multiplicity `degree + 1` by repeated
/// Boehm insertion. `add_knot` inserts a single exact copy and never
/// validates; inserting past `degree + 1` would make an invalid knot vector,
/// so the loop stops exactly at the maximum multiplicity. Copied from
/// `bspline.rs`.
fn raise_to_full_multiplicity<P: ControlPoint<f64> + Tolerance>(
    bsp: &mut BSplineCurve<P>,
    x: f64,
    degree: usize,
) {
    while knot_multiplicity(bsp, x) < degree + 1 {
        bsp.add_knot(x);
    }
}

/// The sub-curve over `[lo, hi]`, where `lo < hi` are already clamped into the
/// knot range. Both endpoints are first raised to full knot multiplicity so
/// that `cut`'s tolerance snapping is exact, and then the curve is cut at
/// `hi` (keeping the front) and at `lo` (returning the middle). Over `[lo, hi]`
/// the basis functions of the extracted curve are non-negative and sum to 1,
/// so every curve point over `[lo, hi]` is a convex combination of the
/// sub-curve's control points. Copied from `bspline.rs`.
fn sub_curve<P: ControlPoint<f64> + Tolerance>(
    bsp: &BSplineCurve<P>,
    lo: f64,
    hi: f64,
) -> BSplineCurve<P> {
    let degree = bsp.degree();
    let mut raised = bsp.clone();
    for x in [lo, hi] {
        raise_to_full_multiplicity(&mut raised, x, degree);
    }
    let mut c = raised;
    let _tail = c.cut(hi);
    c.cut(lo)
}

/// The `min`/`max` of the `coord`-th coordinate of `bsp`'s control points.
/// Copied from `bspline.rs`.
fn min_max<P: Coord>(bsp: &BSplineCurve<P>, coord: usize) -> (f64, f64) {
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for p in bsp.control_points().iter() {
        let c = p.coord(coord);
        min = min.min(c);
        max = max.max(c);
    }
    (min, max)
}

/// One hull-coordinate interval: `[lo, hi]` padded `HULL_PAD (1 + |·|)`
/// outward per endpoint. Copied from `bspline.rs`.
fn hull_interval(lo: f64, hi: f64) -> Interval {
    if !lo.is_finite() || !hi.is_finite() {
        return Interval::EMPTY;
    }
    let pad = HULL_PAD * (1.0 + lo.abs().max(hi.abs()));
    Interval::try_from((lo - pad, hi + pad)).unwrap_or(Interval::EMPTY)
}

/// One hull-coordinate interval: `[mn, mx]` extended by the two boundary
/// values `a`, `b` and padded `HULL_PAD (1 + |·|)` outward per endpoint.
/// Copied from `bspline.rs`.
fn hull_min_max((mn, mx): (f64, f64), a: f64, b: f64) -> Interval {
    hull_interval(mn.min(a).min(b), mx.max(a).max(b))
}

/// The pair of hull intervals `(u, v)` of the sub-curve's control points over
/// `[lo, hi]`, together with the source curve's values at the two boundary
/// parameters.
///
/// The boundary points are load-bearing for a degree-0 (piecewise-constant)
/// hodograph: the sub-curve's own evaluation at its right boundary `hi` uses
/// the left-limit piece, but the source curve's `subs(hi)` uses the right-open
/// value from the piece just past `hi` — a *different* point that is still in
/// the image over `[lo, hi]`. Including `subs(lo)` and `subs(hi)` explicitly
/// keeps the hull sound there; for continuous curves they lie inside the
/// sub-curve hull up to rounding and change nothing.
fn hull_sub_curve2<P: ControlPoint<f64> + Tolerance + Coord>(
    bsp: &BSplineCurve<P>,
    lo: f64,
    hi: f64,
) -> (Interval, Interval) {
    let sub = sub_curve(bsp, lo, hi);
    let lo_pt = ParametricCurve::subs(bsp, lo);
    let hi_pt = ParametricCurve::subs(bsp, hi);
    let u = hull_min_max(min_max(&sub, 0), lo_pt.coord(0), hi_pt.coord(0));
    let v = hull_min_max(min_max(&sub, 1), lo_pt.coord(1), hi_pt.coord(1));
    (u, v)
}

/// The pair of parameter-hull intervals `(uu, vv)` of `{ bsp.subs(t) : t ∈ tt }`
/// by the convex-hull property — the 2D analogue of `bspline.rs`'s `hull_of`,
/// with `bspline.rs`'s case analysis verbatim (empty/non-finite `tt` →
/// `(EMPTY, EMPTY)`; clamp into the knot range; `lo > hi` → empty; `lo == hi`
/// → the point hull; else the sub-curve hull including the boundary values),
/// everything padded by `HULL_PAD (1 + |·|)`.
///
/// Out-of-range `tt` is *not* handled here: [`EnclosureCurve::enclose`] and
/// [`EnclosureCurve::enclose_der`] test it themselves and return the unbounded
/// box directly (decision 4), so the parameter box reaching `hull2_of` is
/// always inside the knot range.
fn hull2_of<P: ControlPoint<f64> + Tolerance + Coord>(
    bsp: &BSplineCurve<P>,
    tt: Interval,
) -> (Interval, Interval) {
    // tt empty or non-finite (NaN bounds, inf > sup) → the empty pair.
    if tt.is_empty() || !tt.inf().is_finite() || !tt.sup().is_finite() {
        return (Interval::EMPTY, Interval::EMPTY);
    }
    let kmin = match bsp.knot_vec().first() {
        Some(k) => *k,
        None => return (Interval::EMPTY, Interval::EMPTY),
    };
    let kmax = match bsp.knot_vec().last() {
        Some(k) => *k,
        None => return (Interval::EMPTY, Interval::EMPTY),
    };
    let lo = tt.inf().max(kmin);
    let hi = tt.sup().min(kmax);
    if lo < hi {
        hull_sub_curve2(bsp, lo, hi)
    } else if lo == hi {
        // Degenerate point box: the image over the single parameter value is
        // the one point subs(lo), so hull it as the point (the `lo == hi`
        // correction `bspline.rs` records, inherited here).
        let pt = ParametricCurve::subs(bsp, lo);
        (
            hull_interval(pt.coord(0), pt.coord(0)),
            hull_interval(pt.coord(1), pt.coord(1)),
        )
    } else {
        (Interval::EMPTY, Interval::EMPTY)
    }
}

/// The unbounded box, the sound enclosure of the image of a box that reaches
/// outside the knot range (decision 4). Returned directly, never forwarded to
/// the inner surface: the surface carriers' behavior on unbounded parameter
/// boxes is not uniform (`bspline.rs`'s `hull_of` returns the EMPTY box for a
/// non-finite `tt`, and an empty composed box would under-estimate).
fn unbounded_box() -> Box3 {
    Box3 {
        x: Interval::ENTIRE,
        y: Interval::ENTIRE,
        z: Interval::ENTIRE,
    }
}

/// The `n = 1` derivative box, the carrier's `der` composed by the chain rule:
/// `D1 = S_10·cu + S_01·cv` with `(cu, cv)` the first hodograph's hull over
/// `tt` and `S_mn = surface.enclose_der(m, n, uu, vv)` (decision 5). All
/// products/sums are inari interval operations.
fn der1_box<S: EnclosureSurface>(curve: &BSplineCurve<Point2>, surface: &S, tt: Interval) -> Box3 {
    let (uu, vv) = hull2_of(curve, tt);
    let (cu, cv) = hull2_of(&curve.derivation(), tt);
    let s10 = surface.enclose_der(1, 0, uu, vv);
    let s01 = surface.enclose_der(0, 1, uu, vv);
    Box3 {
        x: s10.x * cu + s01.x * cv,
        y: s10.y * cu + s01.y * cv,
        z: s10.z * cu + s01.z * cv,
    }
}

/// The `n = 2` derivative box, the carrier's `der2` composed by the chain rule
/// over the surface's second-order partial boxes and the first two hodographs'
/// hulls (decision 5). The scalar coefficient `2.0` is an exact small integer
/// folded into the interval product; all arithmetic is inari.
fn der2_box<S: EnclosureSurface>(curve: &BSplineCurve<Point2>, surface: &S, tt: Interval) -> Box3 {
    let (uu, vv) = hull2_of(curve, tt);
    let (cu, cv) = hull2_of(&curve.derivation(), tt);
    let (cuu, cvv) = hull2_of(&curve.derivation().derivation(), tt);
    let two = interval_at(2.0);
    let s20 = surface.enclose_der(2, 0, uu, vv);
    let s11 = surface.enclose_der(1, 1, uu, vv);
    let s02 = surface.enclose_der(0, 2, uu, vv);
    let s10 = surface.enclose_der(1, 0, uu, vv);
    let s01 = surface.enclose_der(0, 1, uu, vv);
    Box3 {
        x: s20.x * (cu * cu)
            + s11.x * (cu * cv * two)
            + s02.x * (cv * cv)
            + s10.x * cuu
            + s01.x * cvv,
        y: s20.y * (cu * cu)
            + s11.y * (cu * cv * two)
            + s02.y * (cv * cv)
            + s10.y * cuu
            + s01.y * cvv,
        z: s20.z * (cu * cu)
            + s11.z * (cu * cv * two)
            + s02.z * (cv * cv)
            + s10.z * cuu
            + s01.z * cvv,
    }
}

/// The `n = 3` derivative box, the carrier's `der3` composed by the chain rule
/// over the surface's third-order partial boxes and the first three
/// hodographs' hulls (decision 5). The scalar coefficients `3.0` are exact
/// small integers folded into the interval products; all arithmetic is inari.
fn der3_box<S: EnclosureSurface>(curve: &BSplineCurve<Point2>, surface: &S, tt: Interval) -> Box3 {
    let (uu, vv) = hull2_of(curve, tt);
    let (cu, cv) = hull2_of(&curve.derivation(), tt);
    let (cuu, cvv) = hull2_of(&curve.derivation().derivation(), tt);
    let (cuuu, cvvv) = hull2_of(&curve.derivation().derivation().derivation(), tt);
    let three = interval_at(3.0);
    let s30 = surface.enclose_der(3, 0, uu, vv);
    let s21 = surface.enclose_der(2, 1, uu, vv);
    let s12 = surface.enclose_der(1, 2, uu, vv);
    let s03 = surface.enclose_der(0, 3, uu, vv);
    let s20 = surface.enclose_der(2, 0, uu, vv);
    let s11 = surface.enclose_der(1, 1, uu, vv);
    let s02 = surface.enclose_der(0, 2, uu, vv);
    let s10 = surface.enclose_der(1, 0, uu, vv);
    let s01 = surface.enclose_der(0, 1, uu, vv);
    // The nine chain-rule terms of the carrier's der3, one per surface partial:
    // D3 = S_30·cu³ + S_21·(cu²·cv·3) + S_12·(cu·cv²·3) + S_03·cv³
    //    + S_20·(cuu·cu·3) + S_11·((cuu·cv + cvv·cu)·3) + S_02·(cvv·cv·3)
    //    + S_10·cuuu + S_01·cvvv
    Box3 {
        x: s30.x * (cu * cu * cu)
            + s21.x * (cu * cu * cv * three)
            + s12.x * (cu * cv * cv * three)
            + s03.x * (cv * cv * cv)
            + s20.x * (cuu * cu * three)
            + s11.x * ((cuu * cv + cvv * cu) * three)
            + s02.x * (cvv * cv * three)
            + s10.x * cuuu
            + s01.x * cvvv,
        y: s30.y * (cu * cu * cu)
            + s21.y * (cu * cu * cv * three)
            + s12.y * (cu * cv * cv * three)
            + s03.y * (cv * cv * cv)
            + s20.y * (cuu * cu * three)
            + s11.y * ((cuu * cv + cvv * cu) * three)
            + s02.y * (cvv * cv * three)
            + s10.y * cuuu
            + s01.y * cvvv,
        z: s30.z * (cu * cu * cu)
            + s21.z * (cu * cu * cv * three)
            + s12.z * (cu * cv * cv * three)
            + s03.z * (cv * cv * cv)
            + s20.z * (cuu * cu * three)
            + s11.z * ((cuu * cv + cvv * cu) * three)
            + s02.z * (cvv * cv * three)
            + s10.z * cuuu
            + s01.z * cvvv,
    }
}

impl<S: EnclosureSurface<Vector = Vector3>> EnclosureCurve for PCurve<BSplineCurve<Point2>, S> {
    fn exact_spline(&self) -> Option<BSplineCurve<Point3>> {
        let plane = self.surface().as_plane()?;
        // S(p) = o + p.x * a + p.y * b is affine, and B-spline evaluation is
        // linear in the control points, so the composed curve is the B-spline
        // with the same knots and control points o + cx_i * a + cy_i * b —
        // exact, not an approximation.
        let o = plane.origin();
        let a = plane.u_axis();
        let b = plane.v_axis();
        let cps: Vec<Point3> = self
            .curve()
            .control_points()
            .iter()
            .map(|p| o + a * p.x + b * p.y)
            .collect();
        Some(BSplineCurve::new(self.curve().knot_vec().clone(), cps))
    }

    fn enclose(&self, tt: Interval) -> Box3 {
        // tt empty or non-finite (NaN bounds, inf > sup) → the empty box.
        if tt.is_empty() || !tt.inf().is_finite() || !tt.sup().is_finite() {
            return Box3::empty();
        }
        let curve = self.curve();
        let (kmin, kmax) = match knot_range(curve) {
            Some(range) => range,
            None => return Box3::empty(),
        };
        // tt reaching outside the knot range → the unbounded box, returned
        // directly (decision 4). The basis extrapolates outside the range, so
        // no control-point hull can bound the image, and forwarding an
        // unbounded parameter box into surface.enclose would not be sound
        // uniformly across the landed surface carriers.
        if tt.inf() < kmin || tt.sup() > kmax {
            return unbounded_box();
        }
        // The composition: hull the parameter curve over tt in 2D, then take
        // the inner surface's enclosure over the hulled parameter image. The
        // hull contains every c(t), so by BG-ENC-001 of the surface the box
        // contains every S(c(t)).
        let (uu, vv) = hull2_of(curve, tt);
        self.surface().enclose(uu, vv)
    }

    fn enclose_der(&self, n: usize, tt: Interval) -> Box3 {
        if n == 0 {
            return self.enclose(tt);
        }
        // Empty tt → the empty box.
        if tt.is_empty() || !tt.inf().is_finite() || !tt.sup().is_finite() {
            return Box3::empty();
        }
        let curve = self.curve();
        let (kmin, kmax) = match knot_range(curve) {
            Some(range) => range,
            None => return Box3::empty(),
        };
        // Out-of-range tt → the unbounded box, decision 4's rule for the same
        // reason: the extrapolated hodographs are unbounded and no hull can
        // bound them.
        if tt.inf() < kmin || tt.sup() > kmax {
            return unbounded_box();
        }
        // n >= 4 → the unbounded box. The fourth-order chain rule is Faà di
        // Bruno over surface partials; no kernel consumer asks past third
        // order (the carrier itself special-cases der/der2/der3), and a sound
        // widest box is the honest answer rather than an unverified formula.
        if n >= 4 {
            return unbounded_box();
        }
        let surface = self.surface();
        match n {
            1 => der1_box(curve, surface, tt),
            2 => der2_box(curve, surface, tt),
            _ => der3_box(curve, surface, tt),
        }
    }

    fn tangent_cone(&self, tt: Interval) -> Option<DirCone> {
        // The ball-around-midpoint cone off the n = 1 box, the same
        // construction bspline.rs uses for the hodograph hull and extruded.rs
        // for normals, with "tangent" substituted for "normal". Let c be the
        // box's midpoint vector and h its half-width vector; every element of
        // the box lies within rho = ‖h‖ of c, so if rho < cn = ‖c‖ every
        // element makes an angle of at most asin(rho / cn) with c. rho >= cn
        // is exactly the case where the box may contain the zero vector or
        // straddle enough directions that no cone bounds it — the derivative
        // crossing zero (a cusp in parameter space, or both surface partials
        // degenerating at a pole) — so None is the contract. rho rounds UP
        // (.sup()) and cn DOWN (.inf()); the ratio, the asin and the
        // normalize() are then nudged up by a few ulps so none of them can
        // round the cone too narrow.
        let b = self.enclose_der(1, tt);
        let c = Vector3::new(b.x.mid(), b.y.mid(), b.z.mid());
        let h = Vector3::new(b.x.wid() / 2.0, b.y.wid() / 2.0, b.z.wid() / 2.0);
        let norm = |x: Interval, y: Interval, z: Interval| (x.sqr() + y.sqr() + z.sqr()).sqrt();
        let rho = norm(interval_at(h.x), interval_at(h.y), interval_at(h.z)).sup();
        let cn = norm(interval_at(c.x), interval_at(c.y), interval_at(c.z)).inf();
        // The guard in the packet's order and form; `!(cn > rho)` is rejected
        // by clippy's neg_cmp_op_on_partial_ord, and the finiteness tests make
        // the two equivalent (they differ only on NaN). The empty box or a
        // hull containing the origin yields NaN or zero cn.
        if !cn.is_finite() || !rho.is_finite() || cn <= rho {
            return None;
        }
        let axis = c.normalize();
        let half_angle = (rho / cn).asin();
        let half_angle = half_angle * (1.0 + 8.0 * f64::EPSILON) + 8.0 * f64::EPSILON;
        Some(DirCone {
            axis,
            half_angle: half_angle.min(MAX_HALF_ANGLE),
        })
    }
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect on paths reachable from untrusted
// geometry. Unit-test assertions on hand-built witnesses are not such a path.
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::harness::assert_encloses_curve;
    use truck_base::cgmath64::Point3;
    use truck_geometry::decorators::ExtrudedCurve;
    use truck_geometry::nurbs::KnotVec;
    use truck_geometry::specifieds::{Plane, Sphere};

    /// Build a test interval, degrading to EMPTY (and failing its assertion)
    /// rather than panicking on a malformed bound.
    fn iv(lo: f64, hi: f64) -> Interval {
        Interval::try_from((lo, hi)).unwrap_or(Interval::EMPTY)
    }

    /// The terminal-strip widths, in descending powers of ten. Each probes the
    /// last `w` of the knot range, where the tolerance-based knot count left
    /// the hull plateaued at the whole-tail width (BG-ENC-002's convergence
    /// violated in the strip). Copied from `bspline.rs`'s test module. Each
    /// width is a dimensionless knot-range fraction, not a length.
    const STRIP_W_2: f64 = 1.0e-2; // H-3: a dimensionless knot-range width probing the terminal strip, not a length
    const STRIP_W_3: f64 = 1.0e-3; // H-3: a dimensionless knot-range width probing the terminal strip, not a length
    const STRIP_W_4: f64 = 1.0e-4; // H-3: a dimensionless knot-range width probing the terminal strip, not a length
    const STRIP_W_5: f64 = 1.0e-5; // H-3: a dimensionless knot-range width probing the terminal strip, not a length
    const STRIP_W_6: f64 = 1.0e-6; // H-3: a dimensionless knot-range width probing the terminal strip, not a length
    const STRIP_W_7: f64 = 1.0e-7; // H-3: a dimensionless knot-range width probing the terminal strip, not a length
    const STRIP_W_8: f64 = 1.0e-8; // H-3: a dimensionless knot-range width probing the terminal strip, not a length
    const STRIP_W_9: f64 = 1.0e-9; // H-3: a dimensionless knot-range width probing the terminal strip, not a length
    const STRIP_W_10: f64 = 1.0e-10; // H-3: a dimensionless knot-range width probing the terminal strip, not a length
    const STRIP_W_11: f64 = 1.0e-11; // H-3: a dimensionless knot-range width probing the terminal strip, not a length
    const STRIP_W_12: f64 = 1.0e-12; // H-3: a dimensionless knot-range width probing the terminal strip, not a length
    const STRIP_W_13: f64 = 1.0e-13; // H-3: a dimensionless knot-range width probing the terminal strip, not a length
    const STRIP_WIDTHS: [f64; 12] = [
        STRIP_W_2, STRIP_W_3, STRIP_W_4, STRIP_W_5, STRIP_W_6, STRIP_W_7, STRIP_W_8, STRIP_W_9,
        STRIP_W_10, STRIP_W_11, STRIP_W_12, STRIP_W_13,
    ];

    /// The pad allowance in the convergence assertion: the two `HULL_PAD`
    /// hull endpoints plus the boundary-value widening `hull_sub_curve2` adds.
    const STRIP_SLACK: f64 = 256.0 * f64::EPSILON;

    #[test]
    fn pcurve_hull_converges_into_the_terminal_strip() {
        // On the plane witness x = u(t) = t, so the true x-span over
        // [1 − w, 1] is exactly w (the composed z-coordinate is the widest,
        // ~3w, but x is the coordinate the assertion's "true span is w" is
        // written against). The parameter curve's sub-curve extraction must
        // keep shrinking into the terminal strip; the tolerance-based knot
        // count under-inserted next to the terminal knot and plateaued there.
        let mut prev = f64::INFINITY;
        for w in STRIP_WIDTHS {
            let box3 = plane_witness().enclose(iv(1.0 - w, 1.0));
            let xw = box3.x.sup() - box3.x.inf();
            assert!(
                xw <= 4.0 * w + STRIP_SLACK,
                "x-width {xw} exceeds 4w + slack at strip width {w}"
            );
            assert!(
                xw < prev,
                "x-width {xw} not strictly below the previous {prev} at strip width {w}"
            );
            prev = xw;
        }
    }

    /// The plane witness's surface: `S(u, v) = (u, v, u + v)`, an oblique slab
    /// whose two partials are distinct (`S_u = (1, 0, 1)`, `S_v = (0, 1, 1)`),
    /// so a parameter-swap or partial-mix bug in the composition is visible.
    fn plane() -> Plane {
        Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 1.0),
            Point3::new(0.0, 1.0, 1.0),
        )
    }

    /// The quadratic Bézier `c(t) = (t, t²)` on `[0, 1]`, control points
    /// `(0, 0), (1/2, 0), (1, 1)` (dyadic, so the hull endpoints are exact).
    /// The composed image `S(c(t)) = (t, t², t + t²)` is a polynomial on the
    /// plane with the closed forms the tests assert against.
    fn parabola2() -> BSplineCurve<Point2> {
        BSplineCurve::new(
            KnotVec::bezier_knot(2),
            vec![
                Point2::new(0.0, 0.0),
                Point2::new(0.5, 0.0),
                Point2::new(1.0, 1.0),
            ],
        )
    }

    /// The plane witness: `PCurve(parabola2, plane)`.
    fn plane_witness() -> PCurve<BSplineCurve<Point2>, Plane> {
        PCurve::new(parabola2(), plane())
    }

    /// The sphere witness: a meridional arc of the sphere. The parameter curve
    /// `c(t) = (u(t), 0)` with `u(t) = 1/4 + 3t/4` on `[0, 1]` (a collinear
    /// quadratic Bézier, dyadic control points `(1/4, 0), (5/8, 0), (1, 0)`),
    /// so the composed curve is a circle of radius `r` in the xz-plane. Every
    /// sampled point satisfies `|p − centre| == r` to machine precision.
    fn sphere_witness() -> PCurve<BSplineCurve<Point2>, Sphere> {
        let curve = BSplineCurve::new(
            KnotVec::bezier_knot(2),
            vec![
                Point2::new(0.25, 0.0),
                Point2::new(0.625, 0.0),
                Point2::new(1.0, 0.0),
            ],
        );
        PCurve::new(curve, Sphere::new(Point3::new(1.0, -1.0, 0.5), 2.0))
    }

    /// The extruded-composition witness: `PCurve` over an `ExtrudedCurve` of a
    /// `BSplineCurve<Point3>` — decorator on decorator. Soundness is asserted
    /// by sampling only.
    fn extruded_witness(
    ) -> PCurve<BSplineCurve<Point2>, ExtrudedCurve<BSplineCurve<Point3>, Vector3>> {
        let base = BSplineCurve::new(
            KnotVec::bezier_knot(2),
            vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(0.5, 1.0, 0.25),
                Point3::new(1.0, 0.5, 1.0),
            ],
        );
        let surface = ExtrudedCurve::by_extrusion(base, Vector3::unit_z());
        PCurve::new(parabola2(), surface)
    }

    /// The constant parameter curve: all control points equal, so the composed
    /// image is one surface point and the velocity vanishes everywhere.
    fn constant_witness() -> PCurve<BSplineCurve<Point2>, Plane> {
        let curve = BSplineCurve::new(KnotVec::bezier_knot(2), vec![Point2::new(0.3, 0.7); 3]);
        PCurve::new(curve, plane())
    }

    /// The cone-refusal witness: `c(t) = (t² − t, 0)`, the quadratic Bernstein
    /// ordinates `[0, −1/2, 0]` on the u axis. `c'(1/2) = (0, 0)`, so the
    /// composed derivative vanishes at `t = 1/2` and no cone bounds the
    /// tangents on any box containing it.
    fn refusal_witness() -> PCurve<BSplineCurve<Point2>, Plane> {
        let curve = BSplineCurve::new(
            KnotVec::bezier_knot(2),
            vec![
                Point2::new(0.0, 0.0),
                Point2::new(-0.5, 0.0),
                Point2::new(0.0, 0.0),
            ],
        );
        PCurve::new(curve, plane())
    }

    /// Cone containment by angle: cos(angle between axis and d) >=
    /// cos(half_angle). A half_angle at or near π/2 needs the `>=` with float
    /// tolerance to survive rounding, so the slack lives here in the test,
    /// never in the cone.
    fn cone_contains(cone: &DirCone, d: Vector3) -> bool {
        let cos_angle = cone.axis.dot(d.normalize());
        cos_angle >= cone.half_angle.cos() - 1.0e-12 // H-3: float slack between two direction cosines, not a length
    }

    #[test]
    fn pcurve_encloses_sampled_points() {
        let w = plane_witness();
        // The plane witness: the full range and an interior sub-box.
        assert_encloses_curve(&w, iv(0.0, 1.0), 40);
        assert_encloses_curve(&w, iv(0.2, 0.7), 40);
        // The sphere witness: every sampled point lies on the sphere (|p −
        // centre| == r to machine precision) AND in the composed enclosure.
        let s = sphere_witness();
        assert_encloses_curve(&s, iv(0.0, 1.0), 40);
        const N: usize = 40;
        let center = Point3::new(1.0, -1.0, 0.5);
        let radius_slack = 1.0e-12; // H-3: radius residual of the unit-scale sphere witness, not a length
        for i in 0..N {
            let t = (i as f64) / (N as f64 - 1.0);
            let err = (s.subs(t) - center).magnitude() - 2.0;
            assert!(
                err.abs() < radius_slack,
                "sampled point off the sphere radius by {err}"
            );
        }
        // The extruded-composition witness.
        assert_encloses_curve(&extruded_witness(), iv(0.2, 0.8), 40);
        // The constant parameter curve: the image is one surface point.
        assert_encloses_curve(&constant_witness(), iv(0.2, 0.8), 40);
        // The degenerate point box on the plane witness.
        assert_encloses_curve(&w, iv(0.25, 0.25), 30);
    }

    #[test]
    fn pcurve_out_of_range_box_is_unbounded() {
        let w = plane_witness();
        // The plane witness's parameter curve has knot range [0, 1]; a box
        // with lo < kmin, one with hi > kmax, and a large straddling box all
        // extrapolate the basis outside the range, where no hull can bound the
        // image — the whole line per axis is the sound answer.
        for tt in [iv(-1.0, 0.5), iv(0.5, 2.0), iv(-10.0, 10.0)] {
            let b = w.enclose(tt);
            assert_eq!(b.x, Interval::ENTIRE, "x not unbounded for {tt:?}");
            assert_eq!(b.y, Interval::ENTIRE, "y not unbounded for {tt:?}");
            assert_eq!(b.z, Interval::ENTIRE, "z not unbounded for {tt:?}");
        }
        // An interior box is finite on every axis.
        let b = w.enclose(iv(0.2, 0.7));
        for axis in [b.x, b.y, b.z] {
            assert!(
                axis.inf().is_finite() && axis.sup().is_finite(),
                "interior box unbounded"
            );
        }
    }

    #[test]
    fn pcurve_der_enclosures_match_partials() {
        const N: usize = 20;
        // Plane witness, interior box: every sampled der_n lies in the
        // composed chain-rule box.
        let w = plane_witness();
        let tt = iv(0.2, 0.7);
        for n in 1..=3 {
            let enc = w.enclose_der(n, tt);
            for i in 0..N {
                let t = tt.inf() + (tt.sup() - tt.inf()) * (i as f64) / (N as f64 - 1.0);
                let d: Vector3 = w.der_n(n, t);
                assert!(
                    enc.contains(Point3::new(d.x, d.y, d.z)),
                    "plane der_{n} at t={t} escaped {enc:?}"
                );
            }
        }
        // The plane witness's der_n have closed forms (c = (t, t²),
        // S = (u, v, u + v)): der = (1, 2t, 1 + 2t), der2 = (0, 2, 2),
        // der3 = (0, 0, 0).
        let (enc1, enc2, enc3) = (
            w.enclose_der(1, tt),
            w.enclose_der(2, tt),
            w.enclose_der(3, tt),
        );
        for i in 0..N {
            let t = tt.inf() + (tt.sup() - tt.inf()) * (i as f64) / (N as f64 - 1.0);
            assert!(enc1.contains(Point3::new(1.0, 2.0 * t, 1.0 + 2.0 * t)));
            assert!(enc2.contains(Point3::new(0.0, 2.0, 2.0)));
            assert!(enc3.contains(Point3::new(0.0, 0.0, 0.0)));
        }
        // Sphere witness, interior box: every sampled der_n lies in the
        // composed chain-rule box.
        let s = sphere_witness();
        let tt = iv(0.1, 0.9);
        for n in 1..=3 {
            let enc = s.enclose_der(n, tt);
            for i in 0..N {
                let t = tt.inf() + (tt.sup() - tt.inf()) * (i as f64) / (N as f64 - 1.0);
                let d: Vector3 = s.der_n(n, t);
                assert!(
                    enc.contains(Point3::new(d.x, d.y, d.z)),
                    "sphere der_{n} at t={t} escaped {enc:?}"
                );
            }
        }
    }

    #[test]
    fn pcurve_tangent_cone_contains_sampled_tangents() {
        let w = plane_witness();
        // A plane-witness box bounded away from t = 1/2: the composed
        // derivative (1, 2t, 1 + 2t) never vanishes there, so a cone exists
        // and must contain every sampled unit tangent.
        let tt = iv(0.2, 0.4);
        let cone = w
            .tangent_cone(tt)
            .expect("the plane witness has a cone away from t = 1/2");
        const N: usize = 40;
        for i in 0..N {
            let t = tt.inf() + (tt.sup() - tt.inf()) * (i as f64) / (N as f64 - 1.0);
            let d = w.der(t);
            assert!(
                cone_contains(&cone, d),
                "unit tangent at t={t} outside the cone: {:?}",
                d.normalize()
            );
        }
    }

    #[test]
    fn pcurve_tangent_cone_refuses_when_the_derivative_hull_contains_zero() {
        let r = refusal_witness();
        // c'(1/2) = (0, 0): any box containing t = 1/2 has a derivative hull
        // that contains the zero vector, so no cone bounds the tangents.
        for tt in [iv(0.4, 0.6), iv(0.0, 1.0), iv(0.49, 0.51)] {
            assert!(
                r.tangent_cone(tt).is_none(),
                "refusal-witness cone over {tt:?} must be None"
            );
        }
        // A box bounded away from t = 1/2: the velocity stays away from zero,
        // so a cone exists.
        assert!(
            r.tangent_cone(iv(0.0, 0.4)).is_some(),
            "refusal-witness cone over [0, 0.4] must exist"
        );
        // The constant parameter curve has a vanishing derivative everywhere,
        // so its cone is None on every box.
        let c = constant_witness();
        for tt in [iv(0.2, 0.8), iv(0.0, 1.0)] {
            assert!(
                c.tangent_cone(tt).is_none(),
                "constant-curve cone over {tt:?} must be None"
            );
        }
    }

    #[test]
    fn pcurve_subbox_enclosure_is_tighter_than_full_range() {
        let w = plane_witness();
        let full = w.enclose(iv(0.0, 1.0));
        let sub = w.enclose(iv(0.2, 0.7));
        // On the plane witness x = u, so the sub-box hull is strictly narrower
        // than the full-range hull in the x-coordinate: [0, 1] vs [0.2, 0.7].
        assert!(
            sub.x.wid() < full.x.wid(),
            "sub-box x-width {} not below full-range x-width {}",
            sub.x.wid(),
            full.x.wid()
        );
        // Both are sound; only the sub-box is tight — the full-range box
        // still contains every sampled point of the sub-box.
        const N: usize = 40;
        for i in 0..N {
            let t = 0.2 + 0.5 * (i as f64) / (N as f64 - 1.0);
            assert!(
                full.contains(w.subs(t)),
                "full-range box missed a sub-box point at t={t}"
            );
        }
    }

    #[test]
    fn pcurve_enclosure_converges_under_bisection() {
        let w = plane_witness();
        let mut tt = iv(0.05, 0.95);
        let initial = w.enclose(tt).width();
        let mut prev = initial;
        // The hull width is non-increasing up to the HULL_PAD-size outward
        // pad, which is roughly constant across iterations (the pad scales
        // with the control coordinates, which converge as the box does), so it
        // cancels out of the comparison.
        let slack = |x: f64| 256.0 * f64::EPSILON * (1.0 + x);
        for _ in 0..16 {
            let mid = (tt.inf() + tt.sup()) / 2.0;
            tt = iv(tt.inf(), mid);
            let cur = w.enclose(tt).width();
            assert!(
                cur <= prev + slack(prev),
                "enclosure widened under bisection: {prev} -> {cur}"
            );
            prev = cur;
        }
        // Only bisection-convergence explains a 16-bisection shrink below
        // initial/16: each bisection roughly halves the hull's u-width, so the
        // final width is far below a sixteenth of the initial one.
        assert!(
            prev < initial / 16.0,
            "final width {prev} not below initial/16 = {}",
            initial / 16.0
        );
    }

    #[test]
    fn pcurve_der_above_three_is_unbounded() {
        let w = plane_witness();
        // Fourth and seventh order: the deliberate ceiling of decision 5 — the
        // fourth-order chain rule is Faà di Bruno over surface partials, so
        // the honest answer is the whole line per axis.
        for n in [4usize, 7] {
            let b = w.enclose_der(n, iv(0.2, 0.7));
            assert_eq!(b.x, Interval::ENTIRE, "x not unbounded for n = {n}");
            assert_eq!(b.y, Interval::ENTIRE, "y not unbounded for n = {n}");
            assert_eq!(b.z, Interval::ENTIRE, "z not unbounded for n = {n}");
        }
    }
}
