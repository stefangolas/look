//! BG-ENC-003-NURBS: `EnclosureCurve for NurbsCurve<Vector4>`.
//!
//! The **rational** spline carrier. A `NurbsCurve<Vector4>` wraps a
//! `BSplineCurve<Vector4>` whose control points are *homogeneous* —
//! `(w·x, w·y, w·z, w)` — and whose `subs(t)` is the perspective divide of
//! `non_rationalized().subs(t)`. The enclosure technique is [`crate::bspline`]'s
//! convex-hull property, but it lives in **homogeneous coordinates**: over a
//! knot span the homogeneous curve `A(t) = Σ Nᵢ(t)·(wᵢxᵢ, wᵢyᵢ, wᵢzᵢ, wᵢ)` is an
//! ordinary B-spline, so sub-curve extraction by Boehm knot insertion and
//! control-point hulling bound the 4D image, and the 3D image is bounded by
//! **projecting the 4D hull** — each coordinate interval divided by the weight
//! interval in outward-rounded inari division. Project after bounding, never
//! before: the projection of a hull is not the hull of the projection unless
//! every weight is positive.
//!
//! A non-positive weight is a **refusal**: the denominator `Σ Nᵢ(t) wᵢ` can
//! vanish on the domain, so the "curve" is not a well-defined rational curve at
//! all (`EnvelopeCase::NonPositiveNurbsWeight`), never a silently mis-enclosed
//! box. The certified entry is [`try_enclose`]. The trait impl is deliberately
//! **total**: on a non-positive-weight curve it degrades to the *widest sound*
//! answers (`enclose`/`enclose_der` → the unbounded box, `tangent_cone` →
//! `None`), documented as such — it can never return a narrow box for a curve
//! it cannot bound.
//!
//! `enclose_der` is **not** the projection of the homogeneous hodograph. The
//! rational derivative satisfies the classical Leibniz identity
//!
//! ```text
//! C⁽ⁿ⁾ = ( A⁽ⁿ⁾_xyz − Σ_{k=1..n} binom(n, k) · w⁽ᵏ⁾ · C⁽ⁿ⁻ᵏ⁾ ) / w
//! ```
//!
//! and that recursion, in box form, is what `enclose_der` evaluates.
//!
//! The four deviations of `BG-ENC-003-BSPLINE` are inherited as landed
//! behavior, not re-derived: the basis window *extrapolates* outside the knot
//! range (so out-of-range boxes are the whole line per axis, with no origin
//! union and no clamped-hull fallback); the degenerate `lo == hi` box is the
//! point box (not empty); each hull endpoint is padded `HULL_PAD (1 + |·|)`;
//! and the degree-0 boundary values `subs(lo)` and `subs(hi)` join the hull.
//! Each holds per homogeneous coordinate exactly as `bspline.rs` records for a
//! `Point3` curve.

use crate::enclosure::{Box3, DirCone, EnclosureCurve};
use inari::Interval;
use truck_base::cgmath64::control_point::ControlPoint;
use truck_base::cgmath64::{Homogeneous, InnerSpace, Vector3, Vector4};
use truck_base::evidence::{
    Budget, Certificate, Certified, EnvelopeCase, Margin, Method, Modulus, Outcome, Prop, PropMap,
    Refusal, Truth,
};
use truck_base::tolerance::Tolerance;
use truck_geometry::nurbs::{BSplineCurve, NurbsCurve};
use truck_geotrait::{Cut, ParametricCurve};

/// The largest half-angle a [`DirCone`] can report. `asin` saturates at π/2,
/// but the ulp nudge that guards the cone against inward rounding can push a
/// near-π/2 half-angle slightly past it, so the clamp keeps the half-angle
/// within its meaning as an angle. Copied from `bspline.rs`.
const MAX_HALF_ANGLE: f64 = core::f64::consts::PI;

/// The relative outward pad per hull endpoint, as a multiple of `EPSILON`.
///
/// Copied from `bspline.rs` (decision 5's third local change): Boehm insertion
/// and `cut` recompute homogeneous control points in `f64`, so the extracted
/// sub-curve's control points are perturbed relative to the source curve's by
/// several ulps; `64 EPSILON (1 + magnitude)` covers the measured escapes with
/// margin, per coordinate including the weight.
const HULL_PAD: f64 = 64.0 * f64::EPSILON;

/// A degenerate interval from a runtime `f64`. Finite coordinates always
/// construct; a NaN widens to the empty interval rather than panicking (H-1).
fn interval_at(x: f64) -> Interval {
    Interval::try_from((x, x)).unwrap_or(Interval::EMPTY)
}

/// Coordinate access without `Index`, which H-1's `clippy::indexing_slicing`
/// denial bans. `Vector4` is a `ControlPoint<f64>` and carries its coordinates
/// as fields; the same fields are read for the hodograph's vector control
/// points. `0..=3` by `x, y, z, w` — fields, not `Index`.
trait Coord: ControlPoint<f64> {
    /// The `i`-th coordinate, `0..=3`.
    fn coord(self, i: usize) -> f64;
}

impl Coord for Vector4 {
    fn coord(self, i: usize) -> f64 {
        match i {
            0 => self.x,
            1 => self.y,
            2 => self.z,
            _ => self.w,
        }
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
/// Boehm insertion. `add_knot` inserts a single exact copy and never validates;
/// inserting past `degree + 1` would make an invalid knot vector, so the loop
/// stops exactly at the maximum multiplicity. Copied from `bspline.rs`.
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
/// that `cut`'s tolerance snapping is exact — `t − t == 0.0`, so `cut` inserts
/// zero further copies — and then the curve is cut at `hi` (keeping the front)
/// and at `lo` (returning the middle). Over `[lo, hi]` the basis functions of
/// the extracted curve are non-negative and sum to 1, so every homogeneous
/// curve point over `[lo, hi]` is a convex combination of the sub-curve's
/// control points. Copied from `bspline.rs`; the convex-hull property holds per
/// homogeneous coordinate.
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

/// One hull-coordinate interval: `[min, max]` padded `HULL_PAD (1 + |·|)`
/// outward per endpoint. Copied from `bspline.rs`; the weight coordinate is
/// padded the same way.
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

/// The four hull-coordinate intervals of a `BSplineCurve<Vector4>` over a box:
/// the homogeneous analog of `bspline.rs`'s three-coordinate `Box3`.
#[derive(Clone, Copy, Debug)]
struct Hull4 {
    /// x-coordinate hull.
    x: Interval,
    /// y-coordinate hull.
    y: Interval,
    /// z-coordinate hull.
    z: Interval,
    /// weight-coordinate hull.
    w: Interval,
}

/// The empty `Hull4` (NaN on every axis), for the never-firing `unwrap_or`
/// fallbacks of the derivative recursion, whose invariant guarantees every
/// index is present.
fn empty_hull4() -> Hull4 {
    Hull4 {
        x: Interval::EMPTY,
        y: Interval::EMPTY,
        z: Interval::EMPTY,
        w: Interval::EMPTY,
    }
}

/// The axis-aligned hull of the homogeneous sub-curve's control points over
/// `[lo, hi]` (with `lo < hi`), together with the source curve's values at the
/// two boundary parameters.
///
/// The boundary points are load-bearing for a degree-0 (piecewise-constant)
/// hodograph exactly as in `bspline.rs`: the sub-curve's own evaluation at its
/// right boundary `hi` uses the left-limit piece, but the source curve's
/// `subs(hi)` uses the right-open value from the piece just past `hi` — a
/// *different* homogeneous point that is still in the image over `[lo, hi]`.
/// Including `subs(lo)` and `subs(hi)` explicitly keeps the hull sound there;
/// for continuous curves they lie inside the sub-curve hull up to rounding.
fn hull_sub_curve(bsp: &BSplineCurve<Vector4>, lo: f64, hi: f64) -> Hull4 {
    let sub = sub_curve(bsp, lo, hi);
    let lo_pt = ParametricCurve::subs(bsp, lo);
    let hi_pt = ParametricCurve::subs(bsp, hi);
    Hull4 {
        x: hull_min_max(min_max(&sub, 0), lo_pt.coord(0), hi_pt.coord(0)),
        y: hull_min_max(min_max(&sub, 1), lo_pt.coord(1), hi_pt.coord(1)),
        z: hull_min_max(min_max(&sub, 2), lo_pt.coord(2), hi_pt.coord(2)),
        w: hull_min_max(min_max(&sub, 3), lo_pt.coord(3), hi_pt.coord(3)),
    }
}

/// A `Hull4` over the single parameter value `lo`: the padded degenerate
/// intervals of the one homogeneous point `subs(lo)` (the degenerate-box
/// deviation, inherited from `bspline.rs`).
fn point_hull(pt: Vector4) -> Hull4 {
    Hull4 {
        x: hull_interval(pt.coord(0), pt.coord(0)),
        y: hull_interval(pt.coord(1), pt.coord(1)),
        z: hull_interval(pt.coord(2), pt.coord(2)),
        w: hull_interval(pt.coord(3), pt.coord(3)),
    }
}

/// The `Hull4` of the homogeneous curve over the clamped `[lo, hi]`, handling
/// the degenerate `lo == hi` box as the point hull.
fn hull4_of(bsp: &BSplineCurve<Vector4>, lo: f64, hi: f64) -> Hull4 {
    if lo < hi {
        hull_sub_curve(bsp, lo, hi)
    } else {
        point_hull(ParametricCurve::subs(bsp, lo))
    }
}

/// Projects a homogeneous hull to 3-space: each coordinate interval divided by
/// the weight interval in inari's outward-rounded, sign-case-aware interval
/// division.
///
/// All sub-curve weights are positive (decision 3's gate, preserved by Boehm
/// insertion's convex combinations), so `h4.w` is a positive interval up to its
/// pad; if a legitimately tiny weight (≲ `HULL_PAD`) makes the padded
/// `h4.w.inf()` reach zero, inari's division over a denominator straddling zero
/// returns the whole line — **sound automatically, no special case**. That is
/// why no zero-weight guard is needed beyond decision 3.
fn project(h4: Hull4) -> Box3 {
    Box3 {
        x: h4.x / h4.w,
        y: h4.y / h4.w,
        z: h4.z / h4.w,
    }
}

/// The unbounded box, the sound enclosure of the image of a box that reaches
/// outside the knot range (the inherited `bspline.rs` first deviation) and of a
/// non-positive-weight curve (decision 2's total degradation).
fn unbounded_box() -> Box3 {
    Box3 {
        x: Interval::ENTIRE,
        y: Interval::ENTIRE,
        z: Interval::ENTIRE,
    }
}

/// True when every source control-point weight is positive (decision 3's gate).
///
/// Weights are carrier data, not computed values, so a plain f64 comparison is
/// decisive — no intervals. NaN fails `> 0.0` and is refused with the same arm:
/// a curve whose denominator can vanish or misbehave is never given a narrow
/// box. Note `!(w <= 0.0)` is deliberately **not** written (it differs on NaN,
/// and clippy's `neg_cmp_op_on_partial_ord` bites related forms). The gate
/// checks the **source** curve's control points: the sub-curve's weights are
/// convex combinations of these under Boehm insertion, so positivity is
/// preserved along the way — the soundness link that makes the projected hull
/// bound the image.
fn positive_weights(curve: &NurbsCurve<Vector4>) -> bool {
    curve.control_points().iter().all(|v| v.weight() > 0.0)
}

/// The box form of the weighted-derivative recursion of decision 8, evaluated
/// over the `n + 1` homogeneous hulls `h4s` (`h4s[k]` the `k`-fold hodograph's
/// hull over the box).
///
/// ```text
/// Box(C⁽⁰⁾)   = project(H4_0)
/// Box(C⁽ⁿ⁾)_c = ( H4_n.c − Σ_{k=1..n} binom(n,k) · H4_k.w · Box(C⁽ⁿ⁻ᵏ⁾)_c ) / H4_0.w
/// ```
///
/// Soundness: each `H4_k` over-estimates the true hodograph image by the hull
/// property + pad, each `Box(C⁽ⁿ⁻ᵏ⁾)` over-estimates by induction, and interval
/// arithmetic is monotone — so the right-hand side over-estimates
/// `{ C⁽ⁿ⁾(t) : t ∈ tt }`. It over-estimates *more* as `n` grows (decorrelated
/// repeated factors) — acceptable, BG-ENC-001 permits over-estimation. Every
/// operation is an inari interval op; no step rounds inward.
///
/// `h4s` and the box list always have length `n + 1` before any read, so the
/// `get`/`unwrap_or` fallbacks below are unreachable invariants, written this
/// way for H-1 (no `Index`).
fn der_recursion(h4s: &[Hull4], n: usize) -> Box3 {
    let h4_0 = h4s.first().copied().unwrap_or_else(empty_hull4);
    let mut boxes: Vec<Box3> = Vec::with_capacity(n + 1);
    boxes.push(project(h4_0));
    for i in 1..=n {
        let h4_i = h4s.get(i).copied().unwrap_or_else(empty_hull4);
        let mut x = h4_i.x;
        let mut y = h4_i.y;
        let mut z = h4_i.z;
        for k in 1..=i {
            let wk = h4s.get(k).map(|h| h.w).unwrap_or(Interval::EMPTY);
            let prev = boxes.get(i - k).copied().unwrap_or_else(Box3::empty);
            let coef = interval_at(binomial(i, k)) * wk;
            x -= coef * prev.x;
            y -= coef * prev.y;
            z -= coef * prev.z;
        }
        boxes.push(Box3 {
            x: x / h4_0.w,
            y: y / h4_0.w,
            z: z / h4_0.w,
        });
    }
    boxes.last().copied().unwrap_or_else(Box3::empty)
}

/// `binom(n, k)` as an exact `f64` integer, by a small product loop (decision
/// 8's binomials: exact integers in f64 for any `n` a curve degree produces;
/// never a float approximation).
fn binomial(n: usize, k: usize) -> f64 {
    let mut num = 1.0;
    let mut den = 1.0;
    for j in 1..=k {
        num *= (n - k + j) as f64;
        den *= j as f64;
    }
    num / den
}

/// The certified enclosure of `{ curve.subs(t) : t ∈ tt }`, refusing a
/// non-positive-weight curve outright (decision 3).
///
/// The refusal is `Err(Refusal::UnsupportedEnvelope(EnvelopeCase::NonPositiveNurbsWeight))`.
/// On a valid curve the certificate says what [`Method::Interval`] means here:
/// the hull endpoints are f64 `min`/`max` padded outward by a relative
/// `HULL_PAD` and the projection is an outward-rounded inari division — no step
/// in the construction rounds inward. `SoundEnclosure` is the BG-ENC-001 prop:
/// the box provably contains the image. No `τ_rep` anywhere.
pub fn try_enclose(curve: &NurbsCurve<Vector4>, tt: Interval) -> Outcome<Box3> {
    if !positive_weights(curve) {
        return Err(Refusal::UnsupportedEnvelope(
            EnvelopeCase::NonPositiveNurbsWeight,
        ));
    }
    let box3 = curve.enclose(tt);
    let mut props = PropMap::new();
    props.set(Prop::SoundEnclosure, Truth::True);
    Ok(Certified::new(
        box3,
        Certificate {
            props,
            method: Method::Interval,
            budget_left: Budget::new(0, 0, 0),
            margin: Margin::UNBOUNDED,
            modulus: Modulus::Unbounded,
        },
    ))
}

impl EnclosureCurve for NurbsCurve<Vector4> {
    fn enclose(&self, tt: Interval) -> Box3 {
        // Total behavior, all cases spelled (mirroring `bspline.rs`'s hull_of,
        // whose case analysis this copies):
        // - tt empty or non-finite (NaN bounds, inf > sup) → the empty box.
        // - non-positive weight → the unbounded box (never a narrow box for a
        //   curve the denominator can break).
        // - out-of-range tt → the unbounded box. This inherits bspline.rs's
        //   first recorded deviation: the basis window *extrapolates* outside
        //   the knot range (verified there: subs(±10) lands far outside any
        //   origin union), so there is no origin union and no clamped-hull
        //   fallback — the whole line per axis.
        // - clamped (lo, hi): lo > hi → empty; lo == hi → the projected point
        //   box (the degenerate-box deviation, inherited); otherwise Hull4 of
        //   the homogeneous sub-curve over [lo, hi], then project.
        if tt.is_empty() || !tt.inf().is_finite() || !tt.sup().is_finite() {
            return Box3::empty();
        }
        if !positive_weights(self) {
            return unbounded_box();
        }
        let bsp = self.non_rationalized();
        let kmin = match bsp.knot_vec().first() {
            Some(k) => *k,
            None => return Box3::empty(),
        };
        let kmax = match bsp.knot_vec().last() {
            Some(k) => *k,
            None => return Box3::empty(),
        };
        if tt.inf() < kmin || tt.sup() > kmax {
            return unbounded_box();
        }
        let lo = tt.inf().max(kmin);
        let hi = tt.sup().min(kmax);
        if lo > hi {
            return Box3::empty();
        }
        project(hull4_of(bsp, lo, hi))
    }

    fn enclose_der(&self, n: usize, tt: Interval) -> Box3 {
        // n == 0 → the point enclosure, the same construction as `enclose`.
        if n == 0 {
            return self.enclose(tt);
        }
        // Non-positive weight or out-of-range/empty tt → the same total
        // behavior as `enclose`.
        if tt.is_empty() || !tt.inf().is_finite() || !tt.sup().is_finite() {
            return Box3::empty();
        }
        if !positive_weights(self) {
            return unbounded_box();
        }
        let bsp = self.non_rationalized();
        let kmin = match bsp.knot_vec().first() {
            Some(k) => *k,
            None => return Box3::empty(),
        };
        let kmax = match bsp.knot_vec().last() {
            Some(k) => *k,
            None => return Box3::empty(),
        };
        if tt.inf() < kmin || tt.sup() > kmax {
            return unbounded_box();
        }
        let lo = tt.inf().max(kmin);
        let hi = tt.sup().min(kmax);
        if lo > hi {
            return Box3::empty();
        }
        // The k-fold homogeneous hodographs, hulled over [lo, hi]. derivation()
        // on a BSplineCurve<Vector4> yields another BSplineCurve<Vector4>
        // (Vector4 is its own ControlPoint::Diff), so the chain never changes
        // type; derivation() of a degree-0 curve returns the zero curve, so n
        // past the degree hulls to zero without a special case.
        let mut h4s: Vec<Hull4> = Vec::with_capacity(n + 1);
        let mut hodograph = bsp.clone();
        h4s.push(hull4_of(&hodograph, lo, hi));
        for _ in 1..=n {
            hodograph = hodograph.derivation();
            h4s.push(hull4_of(&hodograph, lo, hi));
        }
        der_recursion(&h4s, n)
    }

    fn tangent_cone(&self, tt: Interval) -> Option<DirCone> {
        // The identical ball-around-midpoint construction as `bspline.rs`
        // decision 7, off Box(C⁽¹⁾) from the weighted-derivative recursion. Let
        // c be the hull's midpoint vector and h its half-width vector; every
        // element of the hull lies within rho = ‖h‖ of c, so if rho < cn = ‖c‖
        // every element makes an angle of at most asin(rho / cn) with c.
        // rho >= cn is exactly the case where the hull may contain the zero
        // vector or straddle enough directions that no cone bounds it — the
        // derivative crossing zero — so None is the contract. rho rounds UP
        // (.sup()) and cn DOWN (.inf()); the ratio, the asin and the
        // normalize() are then nudged up by a few ulps so none of them can
        // round the cone too narrow. A non-positive weight certifies no
        // direction at all → None.
        if !positive_weights(self) {
            return None;
        }
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
    use truck_geometry::nurbs::KnotVec;

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
    /// hull endpoints plus the boundary-value widening `hull_sub_curve` adds.
    const STRIP_SLACK: f64 = 256.0 * f64::EPSILON;

    #[test]
    fn nurbs_hull_converges_into_the_terminal_strip() {
        // A unit-weight rationalization of the parabola (t, t², 0): the
        // homogeneous hull projects back to x = t exactly, so the true x-span
        // over [1 − w, 1] is w. The sub-curve extraction must keep shrinking
        // into the terminal strip; the tolerance-based knot count
        // under-inserted next to the terminal knot and plateaued there.
        let curve = NurbsCurve::new(BSplineCurve::new(
            KnotVec::bezier_knot(2),
            vec![
                Vector4::new(0.0, 0.0, 0.0, 1.0),
                Vector4::new(0.5, 0.0, 0.0, 1.0),
                Vector4::new(1.0, 1.0, 0.0, 1.0),
            ],
        ));
        let mut prev = f64::INFINITY;
        for w in STRIP_WIDTHS {
            let box3 = curve.enclose(iv(1.0 - w, 1.0));
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

    /// The NURBS unit circle on `[0, 1]`: the 9-control-point quadratic from
    /// `nurbs/mod.rs`'s own doc example, weights 1 and 2, interior knots at
    /// 1/4, 1/2, 3/4 (each of multiplicity 2). Every sampled point satisfies
    /// `x² + y² == 1` to machine precision: an exact oracle with mixed weights.
    fn circle() -> NurbsCurve<Vector4> {
        NurbsCurve::new(BSplineCurve::new(
            KnotVec::from(vec![
                0.0, 0.0, 0.0, 0.25, 0.25, 0.5, 0.5, 0.75, 0.75, 1.0, 1.0, 1.0,
            ]),
            vec![
                Vector4::new(0.0, -2.0, 0.0, 2.0),
                Vector4::new(1.0, -1.0, 0.0, 1.0),
                Vector4::new(1.0, 0.0, 0.0, 1.0),
                Vector4::new(1.0, 1.0, 0.0, 1.0),
                Vector4::new(0.0, 2.0, 0.0, 2.0),
                Vector4::new(-1.0, 1.0, 0.0, 1.0),
                Vector4::new(-1.0, 0.0, 0.0, 1.0),
                Vector4::new(-1.0, -1.0, 0.0, 1.0),
                Vector4::new(0.0, -2.0, 0.0, 2.0),
            ],
        ))
    }

    /// The rationalized polynomial: `try_from_bspline_and_weights` with all
    /// weights `1.0` on the quadratic `t²−t` Bernstein ordinates
    /// `[0, −1/2, 0]` per coordinate (with the y sign flipped, as in
    /// `bspline.rs`'s quad): the curve is the polynomial and the derivative
    /// `2t − 1` (up to sign per coordinate) vanishes at `t = 1/2` — the
    /// cone-refusal witness.
    fn polynomial() -> NurbsCurve<Vector4> {
        let bsp = BSplineCurve::new(
            KnotVec::bezier_knot(2),
            vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(-0.5, 0.5, -0.5),
                Point3::new(0.0, 0.0, 0.0),
            ],
        );
        NurbsCurve::try_from_bspline_and_weights(bsp, vec![1.0, 1.0, 1.0])
            .expect("the equal-weight rationalization of a quadratic is valid")
    }

    /// The constant curve: all control points `(1, 2, 3)` with mixed positive
    /// weights. The curve is the single point, the derivative is identically
    /// zero, and the cone is `None` everywhere.
    fn constant() -> NurbsCurve<Vector4> {
        let bsp = BSplineCurve::new(
            KnotVec::bezier_knot(2),
            vec![
                Point3::new(1.0, 2.0, 3.0),
                Point3::new(1.0, 2.0, 3.0),
                Point3::new(1.0, 2.0, 3.0),
            ],
        );
        NurbsCurve::try_from_bspline_and_weights(bsp, vec![1.0, 1.5, 2.0])
            .expect("the constant curve with positive weights is valid")
    }

    /// The quadratic `t²−t` witness rationalized with the given first weight
    /// (negative or zero for the refusal witness; the length always matches).
    fn bad_weight(weight: f64) -> NurbsCurve<Vector4> {
        let bsp = BSplineCurve::new(
            KnotVec::bezier_knot(2),
            vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(-0.5, 0.5, -0.5),
                Point3::new(0.0, 0.0, 0.0),
            ],
        );
        NurbsCurve::try_from_bspline_and_weights(bsp, vec![weight, 1.0, 1.0])
            .expect("the weight list matches the control-point count")
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
    fn nurbs_encloses_sampled_points() {
        let c = circle();
        let p = polynomial();
        let k = constant();
        // On the circle: the full range, an interior sub-box, a box straddling
        // the interior knot 1/2, and the degenerate point box [0.25, 0.25]
        // (hull is the point, up to the widening).
        for tt in [iv(0.0, 1.0), iv(0.2, 0.6), iv(0.4, 0.6), iv(0.25, 0.25)] {
            assert_encloses_curve(&c, tt, 30);
        }
        // The equal-weights polynomial witness.
        assert_encloses_curve(&p, iv(0.0, 1.0), 30);
        assert_encloses_curve(&p, iv(0.2, 0.8), 30);
        // The constant curve.
        assert_encloses_curve(&k, iv(0.0, 1.0), 30);
    }

    #[test]
    fn nurbs_negative_weight_is_refused() {
        let neg = bad_weight(-1.0);
        let zero = bad_weight(0.0);
        let valid = polynomial();
        for bad in [&neg, &zero] {
            // Decision 3's gate: the certified entry refuses both a negative
            // and a zero weight.
            assert!(matches!(
                try_enclose(bad, iv(0.0, 1.0)),
                Err(Refusal::UnsupportedEnvelope(
                    EnvelopeCase::NonPositiveNurbsWeight
                ))
            ));
            // The total trait impl degrades to the widest sound answer: the
            // unbounded box per axis, never a narrow mis-enclosure.
            let b = bad.enclose(iv(0.0, 1.0));
            assert_eq!(b.x, Interval::ENTIRE);
            assert_eq!(b.y, Interval::ENTIRE);
            assert_eq!(b.z, Interval::ENTIRE);
            assert!(bad.tangent_cone(iv(0.0, 1.0)).is_none());
        }
        // On a valid curve the certificate asserts its method and prop
        // field-by-field (decision 4).
        let out = try_enclose(&valid, iv(0.2, 0.8)).expect("valid curve is certifiable");
        assert_eq!(out.cert.method, Method::Interval);
        assert_eq!(out.cert.props.get(Prop::SoundEnclosure), Truth::True);
    }

    #[test]
    fn nurbs_out_of_range_box_is_unbounded() {
        let c = circle();
        // A box with lo < 0, one with hi > 1, and a large box: all reach
        // outside the knot range, so the inherited bspline.rs first deviation
        // gives the whole line per axis.
        for tt in [iv(-0.5, 0.5), iv(0.5, 1.5), iv(-10.0, 10.0)] {
            let b = c.enclose(tt);
            assert_eq!(b.x, Interval::ENTIRE);
            assert_eq!(b.y, Interval::ENTIRE);
            assert_eq!(b.z, Interval::ENTIRE);
        }
        // A fully interior box is finite on every axis.
        let b = c.enclose(iv(0.2, 0.8));
        assert!(b.x.inf().is_finite() && b.x.sup().is_finite());
        assert!(b.y.inf().is_finite() && b.y.sup().is_finite());
        assert!(b.z.inf().is_finite() && b.z.sup().is_finite());
    }

    #[test]
    fn nurbs_der_enclosures_match_partials() {
        let c = circle();
        let p = polynomial();
        // The recursion's soundness test: every sampled der_n must lie inside
        // enclose_der(n, tt), on interior and knot-straddling boxes. The
        // certificate-witness circle with its mixed weights is the one that
        // catches a wrong rational-derivative formula.
        let cells = [
            (&c, iv(0.6, 0.7)), // an interior span of the circle
            (&c, iv(0.4, 0.6)), // straddles the interior knot 1/2
            (&p, iv(0.2, 0.8)),
            (&p, iv(0.4, 0.6)), // around the derivative zero
        ];
        for (curve, tt) in cells {
            for n in 1..=3 {
                let enc = curve.enclose_der(n, tt);
                const N: usize = 25;
                for i in 0..N {
                    let t = tt.inf() + (tt.sup() - tt.inf()) * (i as f64) / (N as f64 - 1.0);
                    let d = curve.der_n(n, t);
                    assert!(
                        enc.contains(Point3::new(d.x, d.y, d.z)),
                        "der_{n} at t={t} escaped its enclosure {enc:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn nurbs_tangent_cone_contains_sampled_tangents() {
        let c = circle();
        // The first quadrant of the circle (t = 0.25 → angle 0, t = 0.5 →
        // angle 90°), away from the parameter ends: both coordinates stay
        // strictly positive, so the derivative hull stays clear of the axis
        // and the midpoint cone exists.
        let tt = iv(0.3, 0.45);
        let cone = c
            .tangent_cone(tt)
            .expect("the circle derivative never vanishes over [0.3, 0.45]");
        const N: usize = 60;
        for i in 0..N {
            let t = tt.inf() + (tt.sup() - tt.inf()) * (i as f64) / (N as f64 - 1.0);
            let d = c.der(t);
            assert!(
                cone_contains(&cone, d),
                "unit tangent at t={t} outside the cone: {:?}",
                d.normalize()
            );
        }
    }

    #[test]
    fn nurbs_tangent_cone_refuses_when_the_derivative_hull_contains_zero() {
        let p = polynomial();
        let k = constant();
        // Any box containing t = 1/2, where the derivative 2t − 1 vanishes
        // (and the y, z siblings with it): the derivative hull contains zero.
        for tt in [iv(0.4, 0.6), iv(0.0, 1.0), iv(0.49, 0.51)] {
            assert!(
                p.tangent_cone(tt).is_none(),
                "polynomial cone over {tt:?} must be None"
            );
        }
        // A box bounded away from t = 1/2: the derivative hull stays clear of
        // zero, so a cone exists.
        assert!(
            p.tangent_cone(iv(0.0, 0.4)).is_some(),
            "polynomial cone over [0, 0.4] must exist"
        );
        // The constant curve has an identically zero derivative: None
        // everywhere, including out-of-range boxes.
        for tt in [iv(0.0, 1.0), iv(0.2, 0.8), iv(-0.5, 1.5)] {
            assert!(
                k.tangent_cone(tt).is_none(),
                "constant curve cone over {tt:?} must be None"
            );
        }
    }

    #[test]
    fn nurbs_subbox_enclosure_is_tighter_than_full_range() {
        let c = circle();
        let full = c.enclose(iv(0.0, 1.0));
        // [1/16, 1/8] is a single-span arc inside (0, 1/4): its enclosure is
        // strictly narrower than the full-range one in at least one coordinate.
        let sub = c.enclose(iv(1.0 / 16.0, 1.0 / 8.0));
        let (sx, sy) = (sub.x.sup() - sub.x.inf(), sub.y.sup() - sub.y.inf());
        let (fx, fy) = (full.x.sup() - full.x.inf(), full.y.sup() - full.y.inf());
        assert!(
            sx < fx || sy < fy,
            "arc box {sx}x{sy} not strictly narrower than the full range {fx}x{fy}"
        );
        // Both boxes are sound; only the arc box is tight, so the full-range
        // box must still contain every sampled point of the arc.
        const N: usize = 40;
        for i in 0..N {
            let t = 1.0 / 16.0 + (1.0 / 8.0 - 1.0 / 16.0) * (i as f64) / (N as f64 - 1.0);
            assert!(
                full.contains(c.subs(t)),
                "full-range box must contain the arc point at t={t}"
            );
        }
    }

    #[test]
    fn nurbs_enclosure_converges_under_bisection() {
        let c = circle();
        let mut tt = iv(0.05, 0.95);
        let initial = c.enclose(tt).width();
        let mut prev = initial;
        // Bisect 16 times toward the left endpoint of the box. The hull width
        // is non-increasing up to the HULL_PAD-size outward pad, which is
        // roughly constant across iterations (the pad scales with the control
        // coordinates, which converge as the box does), so it cancels out of
        // the comparison.
        let slack = |w: f64| 256.0 * f64::EPSILON * (1.0 + w);
        for _ in 0..16 {
            let mid = (tt.inf() + tt.sup()) / 2.0;
            tt = iv(tt.inf(), mid);
            let cur = c.enclose(tt).width();
            assert!(
                cur <= prev + slack(prev),
                "enclosure widened under bisection: {prev} -> {cur}"
            );
            prev = cur;
        }
        // Only bisection-convergence explains a 16-bisection shrink below
        // initial/16: each bisection roughly halves the sub-curve's control
        // polygon width, so the final width is far below a sixteenth of the
        // initial one.
        assert!(
            prev < initial / 16.0,
            "final width {prev} not below initial/16 = {}",
            initial / 16.0
        );
    }
}
