//! BG-ENC-003-BSPLINE: `EnclosureCurve for BSplineCurve<Point3>`.
//!
//! The first **spline** carrier: a `BSplineCurve<Point3>` is not closed-form but
//! a basis sum, and its enclosure is *not* computed by evaluating that sum in
//! interval arithmetic. The technique is the **convex-hull property**: over a
//! knot span a B-spline lies in the convex hull of its control points, so to
//! enclose the curve over a box `tt` one extracts the sub-curve over `tt` by
//! Boehm knot insertion and bounds the sub-curve's control points. This is
//! tighter than naive interval arithmetic on the basis sum (which suffers
//! dependency loss) and cheaper (no interval basis evaluation).
//!
//! The tangent cone comes off the **hodograph**: `BSplineCurve::derivation()`
//! returns the derivative curve, whose control points are the scaled forward
//! differences. The derivative hull contains 0 exactly where the tangent
//! direction is undefined — that is the `None` case.
//!
//! Four decisions deviate from the packet (recorded in `RESULT.json`):
//!
//! - **Out-of-range `tt`.** Decision 5 unions `Box3::point(origin)` on the claim
//!   that the Cox–de Boor basis is non-negative everywhere and sums to at most 1
//!   outside the active domain. That claim is false for this crate's evaluation
//!   path: `der_n` runs the basis recursion with the parameter left outside the
//!   knot range, and the extrapolation is the boundary polynomial itself
//!   (verified: the quadratic `t²−t` witness returns `subs(−10) = (110, 0, 0)`
//!   and `subs(10) = (90, 0, 0)`), which is unbounded as `|t| → ∞`. The origin
//!   union under-estimates — a BG-ENC-001 silent wrong answer. The sound
//!   replacement is the entire line per axis.
//! - **Degenerate point boxes.** Decision 5's `lo >= hi` branch returns the
//!   empty box, but a box `[0.25, 0.25]` has image the single point
//!   `subs(0.25)`; the packet's own witness requires that point box, so `lo ==
//!   hi` is hulled as the point (decision 4's widening included).
//! - **Hull widening.** Decision 4 widens each hull endpoint a single ulp, but
//!   the f64 Boehm insertion and `cut` recompute control points with rounding
//!   that can push the source curve's evaluation several ulps outside the
//!   extracted sub-curve's control-point hull (measured up to ~10 ulps). The
//!   endpoints are therefore padded by `HULL_PAD (1 + |·|)` instead.
//! - **Degree-0 boundary values.** `subs(hi)` of the source curve uses the
//!   right-open value from the piece just past `hi`, which a degree-0
//!   (piecewise-constant) hodograph does not represent in its sub-curve; the
//!   hull therefore also includes `subs(lo)` and `subs(hi)` themselves.

use crate::enclosure::{Box3, DirCone, EnclosureCurve};
use inari::Interval;
use truck_base::cgmath64::control_point::ControlPoint;
use truck_base::cgmath64::{InnerSpace, Point3, Vector3};
use truck_base::tolerance::Tolerance;
use truck_geometry::nurbs::BSplineCurve;
use truck_geotrait::{Cut, ParametricCurve};

/// The largest half-angle a [`DirCone`] can report. `asin` saturates at π/2,
/// but the ulp nudge that guards the cone against inward rounding can push a
/// near-π/2 half-angle slightly past it, so the clamp keeps the half-angle
/// within its meaning as an angle.
const MAX_HALF_ANGLE: f64 = core::f64::consts::PI;

/// The relative outward pad per hull endpoint, as a multiple of `EPSILON`
/// (deviation from decision 4's single `next_down`/`next_up` step).
///
/// Boehm insertion and `cut` recompute control points in `f64`, so the
/// extracted sub-curve's control points are perturbed relative to the source
/// curve's, and the two evaluation paths can disagree by several ulps of a
/// coordinate — measured up to ~10 ulps on the packet's own witnesses (e.g.
/// the uniform quadratic's `enclose_der(1, [0.4, 0.6])` missed `der(0.4)` by
/// ~4 ulps before this pad, a BG-ENC-001 under-estimation). A single outward
/// ulp is therefore not enough. `64 EPSILON (1 + magnitude)` covers the
/// measured escapes with margin; the pad is proportional to the coordinate
/// magnitude because the rounding it absorbs is proportional to it.
const HULL_PAD: f64 = 64.0 * f64::EPSILON;

/// A degenerate interval from a runtime `f64`. Finite coordinates always
/// construct; a NaN widens to the empty interval rather than panicking (H-1).
fn interval_at(x: f64) -> Interval {
    Interval::try_from((x, x)).unwrap_or(Interval::EMPTY)
}

/// Coordinate access without `Index`, which H-1's `clippy::indexing_slicing`
/// denial bans. Both `Point3` and `Vector3` are `ControlPoint<f64>` and carry
/// their coordinates as fields; the same values are read for the hodograph's
/// vector control points.
trait Coord: ControlPoint<f64> {
    /// The `i`-th coordinate, `0..=2`.
    fn coord(self, i: usize) -> f64;
}

impl Coord for Point3 {
    fn coord(self, i: usize) -> f64 {
        match i {
            0 => self.x,
            1 => self.y,
            _ => self.z,
        }
    }
}

impl Coord for Vector3 {
    fn coord(self, i: usize) -> f64 {
        match i {
            0 => self.x,
            1 => self.y,
            _ => self.z,
        }
    }
}

/// The multiplicity of the knot value `x` in `bsp`'s knot vector, counted over
/// **exact** knot equality. `KnotVec::multiplicity` matches by tolerance and
/// would count a *different* knot value within the legacy tolerance of `x`,
/// which under-inserts in the raising loop and extracts an over-wide sub-curve
/// whenever `x` sits within tolerance of another knot (the terminal strip of
/// every knot range).
fn knot_multiplicity<P: ControlPoint<f64>>(bsp: &BSplineCurve<P>, x: f64) -> usize {
    bsp.knot_vec().iter().filter(|&&k| k == x).count()
}

/// Raises the knot value `x` to full multiplicity `degree + 1` by repeated
/// Boehm insertion (decision 3). `add_knot` inserts a single exact copy and
/// never validates; inserting past `degree + 1` would make an invalid knot
/// vector, so the loop stops exactly at the maximum multiplicity.
fn raise_to_full_multiplicity<P: ControlPoint<f64> + Tolerance>(
    bsp: &mut BSplineCurve<P>,
    x: f64,
    degree: usize,
) {
    while knot_multiplicity(bsp, x) < degree + 1 {
        bsp.add_knot(x);
    }
}

/// The sub-curve over `[lo, hi]` (decision 3), where `lo < hi` are already
/// clamped into the knot range. Both endpoints are first raised to full knot
/// multiplicity so that `cut`'s tolerance snapping is exact — `t − t == 0.0`,
/// so `cut` inserts zero further copies — and then the curve is cut at `hi`
/// (keeping the front) and at `lo` (returning the middle). Over `[lo, hi]` the
/// basis functions of the extracted curve are non-negative and sum to 1, so
/// every curve point over `[lo, hi]` is a convex combination of the sub-curve's
/// control points: its axis-aligned box is an enclosure (the convex-hull
/// property).
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
/// outward per endpoint (decision 4's outward-rounding discipline, sized to the
/// measured Boehm-insertion rounding; see `HULL_PAD`).
fn hull_interval(lo: f64, hi: f64) -> Interval {
    if !lo.is_finite() || !hi.is_finite() {
        return Interval::EMPTY;
    }
    let pad = HULL_PAD * (1.0 + lo.abs().max(hi.abs()));
    Interval::try_from((lo - pad, hi + pad)).unwrap_or(Interval::EMPTY)
}

/// One hull-coordinate interval: `[mn, mx]` extended by the two boundary
/// values `a`, `b` and padded `HULL_PAD (1 + |·|)` outward per endpoint.
fn hull_min_max((mn, mx): (f64, f64), a: f64, b: f64) -> Interval {
    hull_interval(mn.min(a).min(b), mx.max(a).max(b))
}

/// The axis-aligned box of the sub-curve's control points over `[lo, hi]`
/// (decisions 3 + 4), together with the source curve's values at the two
/// boundary parameters.
///
/// The boundary points are load-bearing for a degree-0 (piecewise-constant)
/// hodograph: the sub-curve's own evaluation at its right boundary `hi` uses
/// the left-limit piece, but the source curve's `subs(hi)` uses the right-open
/// value from the piece just past `hi` — a *different* point that is still in
/// the image over `[lo, hi]`. Including `subs(lo)` and `subs(hi)` explicitly
/// keeps the hull sound there; for continuous curves they lie inside the
/// sub-curve hull up to rounding and change nothing.
fn hull_sub_curve<P: ControlPoint<f64> + Tolerance + Coord>(
    bsp: &BSplineCurve<P>,
    lo: f64,
    hi: f64,
) -> Box3 {
    let sub = sub_curve(bsp, lo, hi);
    let lo_pt = ParametricCurve::subs(bsp, lo);
    let hi_pt = ParametricCurve::subs(bsp, hi);
    Box3 {
        x: hull_min_max(min_max(&sub, 0), lo_pt.coord(0), hi_pt.coord(0)),
        y: hull_min_max(min_max(&sub, 1), lo_pt.coord(1), hi_pt.coord(1)),
        z: hull_min_max(min_max(&sub, 2), lo_pt.coord(2), hi_pt.coord(2)),
    }
}

/// The unbounded box, the sound enclosure of the image of a box that reaches
/// outside the knot range (deviation from decision 5's origin union; see the
/// module comment).
fn unbounded_box() -> Box3 {
    Box3 {
        x: Interval::ENTIRE,
        y: Interval::ENTIRE,
        z: Interval::ENTIRE,
    }
}

/// The enclosure of `{ bsp.subs(t) : t ∈ tt }` by the convex-hull property
/// (decision 5), shared verbatim by `enclose` and, over the hodograph, by
/// `enclose_der`.
fn hull_of<P: ControlPoint<f64> + Tolerance + Coord>(bsp: &BSplineCurve<P>, tt: Interval) -> Box3 {
    // tt empty or non-finite (NaN bounds, inf > sup) → the empty box.
    if tt.is_empty() || !tt.inf().is_finite() || !tt.sup().is_finite() {
        return Box3::empty();
    }
    let kmin = match bsp.knot_vec().first() {
        Some(k) => *k,
        None => return Box3::empty(),
    };
    let kmax = match bsp.knot_vec().last() {
        Some(k) => *k,
        None => return Box3::empty(),
    };
    let lo = tt.inf().max(kmin);
    let hi = tt.sup().min(kmax);
    let mut box3 = if lo < hi {
        hull_sub_curve(bsp, lo, hi)
    } else if lo == hi {
        // Decision 5's `lo >= hi` branch would return the empty box here, but
        // the image over the single parameter value is the one point subs(lo)
        // (deviation, recorded): hull it as the degenerate point box, decision
        // 4's widening included.
        let pt = ParametricCurve::subs(bsp, lo);
        Box3 {
            x: hull_interval(pt.coord(0), pt.coord(0)),
            y: hull_interval(pt.coord(1), pt.coord(1)),
            z: hull_interval(pt.coord(2), pt.coord(2)),
        }
    } else {
        Box3::empty()
    };
    // A box reaching outside the knot range images onto extrapolated curve
    // values that no control-point hull can bound (deviation from the origin
    // union, which under-estimates — see the module comment).
    if tt.inf() < kmin || tt.sup() > kmax {
        box3 = unbounded_box();
    }
    box3
}

impl EnclosureCurve for BSplineCurve<Point3> {
    fn enclose(&self, tt: Interval) -> Box3 {
        hull_of(self, tt)
    }

    fn exact_spline(&self) -> Option<BSplineCurve<Point3>> {
        Some(self.clone())
    }

    fn enclose_der(&self, n: usize, tt: Interval) -> Box3 {
        if n == 0 {
            return self.enclose(tt);
        }
        // The n-fold hodograph: derivation() on a BSplineCurve<Point3> yields a
        // BSplineCurve<Vector3>, and on BSplineCurve<Vector3> another
        // BSplineCurve<Vector3> (Vector3 is a ControlPoint). The hodograph's
        // subs(t) reproduces der_n(t, self) on the same basis-evaluation path,
        // so the identical hull construction (sub-curve, hull, out-of-range
        // box) encloses { der_n(t) : t ∈ tt }.
        let mut hodograph = self.derivation();
        for _ in 1..n {
            hodograph = hodograph.derivation();
        }
        hull_of(&hodograph, tt)
    }

    fn tangent_cone(&self, tt: Interval) -> Option<DirCone> {
        // The ball-around-midpoint cone off the first hodograph's hull, the
        // same construction decorators/extruded.rs uses for normals with
        // "tangent" substituted for "normal". Let c be the hull's midpoint
        // vector and h its half-width vector; every element of the hull lies
        // within rho = ‖h‖ of c, so if rho < cn = ‖c‖ every element makes an
        // angle of at most asin(rho / cn) with c. rho >= cn is exactly the
        // case where the hull may contain the zero vector or straddle enough
        // directions that no cone bounds it — the derivative crossing zero (a
        // cusp or an inflection with horizontal tangent) — so None is the
        // contract. rho rounds UP (.sup()) and cn DOWN (.inf()); the ratio,
        // the asin and the normalize() are then nudged up by a few ulps so
        // none of them can round the cone too narrow. This is sound but loose
        // the same way extruded.rs's is: it bounds the hull, not the true
        // derivative set.
        let b = hull_of(&self.derivation(), tt);
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

    /// A dimensionless knot offset inside the legacy tolerance of the terminal
    /// knot: the tolerance-based count treats `1.0 - OFFSET` as `1.0`, the
    /// exact count does not.
    const TINY_KNOT_OFFSET: f64 = 1.0e-6; // H-3: a dimensionless knot offset probing exact-count knot multiplicity, not a length

    /// The terminal-strip widths, in descending powers of ten. Each probes the
    /// last `w` of the knot range, where the tolerance-based knot count left
    /// the hull plateaued at the whole-tail width (BG-ENC-002's convergence
    /// violated in the strip). Each width is a dimensionless knot-range
    /// fraction, not a length.
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
    fn knot_multiplicity_counts_exactly() {
        // A clamped quadratic: knots [0, 0, 0, 1, 1, 1].
        let bsp = BSplineCurve::new(KnotVec::bezier_knot(2), vec![Point3::new(0.0, 0.0, 0.0); 3]);
        assert_eq!(knot_multiplicity(&bsp, 1.0), 3);
        // A parameter within tolerance of the terminal knot is a *different*
        // value: the exact count is 0. `KnotVec::multiplicity` would count the
        // terminal copies and skip the insertions the raising loop needs.
        assert_eq!(knot_multiplicity(&bsp, 1.0 - TINY_KNOT_OFFSET), 0);
        let mut raised = bsp.clone();
        let degree = raised.degree();
        raise_to_full_multiplicity(&mut raised, 1.0 - TINY_KNOT_OFFSET, degree);
        // Three insertions reach full multiplicity degree + 1 = 3.
        assert_eq!(knot_multiplicity(&raised, 1.0 - TINY_KNOT_OFFSET), 3);
    }

    #[test]
    fn bspline_hull_converges_into_the_terminal_strip() {
        // On the quad witness x = t² − t, the true x-span over [1 − w, 1] is
        // ~w. The enclosure must keep shrinking with w all the way into the
        // terminal strip: with the tolerance-based knot count the extraction
        // under-inserted next to the terminal knot and the hull plateaued at
        // the whole-tail width for every w inside the tolerance. Only the
        // exact count converges here.
        let mut prev = f64::INFINITY;
        for w in STRIP_WIDTHS {
            let box3 = quad().enclose(iv(1.0 - w, 1.0));
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

    /// The quadratic Bézier `x(t) = y(t) = z(t) = t²−t` on `[0, 1]`, control
    /// ordinates `[0, −1/2, 0]` per coordinate (dyadic, so the hull endpoints
    /// are exact): true range `[−1/4, 0]`, hull `[−1/2, 0]`. The derivative
    /// `2t − 1` (and its y, z siblings) vanishes at `t = 1/2`.
    fn quad() -> BSplineCurve<Point3> {
        BSplineCurve::new(
            KnotVec::bezier_knot(2),
            vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(-0.5, 0.5, -0.5),
                Point3::new(0.0, 0.0, 0.0),
            ],
        )
    }

    /// The cubic Bézier `x(t) = y(t) = z(t) = t³−t` on `[0, 1]`, control
    /// ordinates `[0, −1/3, −2/3, 0]` per coordinate: true range
    /// `[−2/(3√3), 0]`, hull `[−2/3, 0]`.
    fn cubic() -> BSplineCurve<Point3> {
        BSplineCurve::new(
            KnotVec::bezier_knot(3),
            vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(-1.0 / 3.0, -1.0 / 3.0, -1.0 / 3.0),
                Point3::new(-2.0 / 3.0, -2.0 / 3.0, -2.0 / 3.0),
                Point3::new(0.0, 0.0, 0.0),
            ],
        )
    }

    /// A helix-like 3D cubic Bézier with mixed-sign control points. The z
    /// coordinate is affine (`z(t) = 3t`), so the derivative never vanishes and
    /// the tangent sweeps a range — a witness for the tangent cone.
    fn helix() -> BSplineCurve<Point3> {
        BSplineCurve::new(
            KnotVec::bezier_knot(3),
            vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 2.0, 1.0),
                Point3::new(2.0, -1.0, 2.0),
                Point3::new(3.0, 1.0, 3.0),
            ],
        )
    }

    /// A non-Bézier witness: the clamped uniform quadratic on `[0, 1]` with
    /// interior knots at `0.25, 0.5, 0.75` and mixed-sign control points.
    fn uniform() -> BSplineCurve<Point3> {
        BSplineCurve::new(
            KnotVec::uniform_knot(2, 4),
            vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
                Point3::new(1.0, 1.0, 0.0),
                Point3::new(1.0, 0.5, 0.0),
                Point3::new(2.0, 0.0, 0.0),
                Point3::new(3.0, 0.5, 0.5),
            ],
        )
    }

    /// A full-period witness: a cubic Bézier whose control polygon winds once
    /// around the origin, so its derivative hull over the whole domain covers
    /// every direction.
    fn loop_witness() -> BSplineCurve<Point3> {
        BSplineCurve::new(
            KnotVec::bezier_knot(3),
            vec![
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
                Point3::new(-1.0, 0.0, 0.0),
                Point3::new(0.0, -1.0, 0.0),
            ],
        )
    }

    /// A witness whose control-point hull strictly excludes the origin (every
    /// coordinate positive), for the origin-union test.
    fn away_from_origin() -> BSplineCurve<Point3> {
        BSplineCurve::new(
            KnotVec::bezier_knot(2),
            vec![
                Point3::new(2.0, 1.0, 1.0),
                Point3::new(3.0, 2.0, 1.0),
                Point3::new(2.0, 1.0, 2.0),
            ],
        )
    }

    #[test]
    fn bspline_encloses_sampled_points() {
        let q = quad();
        let u = uniform();
        let c = cubic();
        // On the Bézier witness: an interior sub-box, the full [0, 1], the
        // degenerate point box [0.25, 0.25] (hull is the point, up to the
        // widening), a box with negative lo, one with hi > 1 (both reach
        // outside the knot range), and a large box.
        for tt in [
            iv(0.2, 0.7),
            iv(0.0, 1.0),
            iv(0.25, 0.25),
            iv(-0.5, 0.5),
            iv(0.5, 1.5),
            iv(-10.0, 10.0),
        ] {
            assert_encloses_curve(&q, tt, 40);
        }
        // On the uniform witness: a box straddling the interior knot 0.5, the
        // full [0, 1], an out-of-range box, and a large box.
        for tt in [iv(0.4, 0.6), iv(0.0, 1.0), iv(-0.5, 1.5), iv(-10.0, 10.0)] {
            assert_encloses_curve(&u, tt, 40);
        }
        // A cubic interior box.
        assert_encloses_curve(&c, iv(0.25, 0.75), 40);
    }

    #[test]
    fn bspline_out_of_range_box_unions_the_origin() {
        let a = away_from_origin();
        // A box entirely beyond the knot range: the extrapolated image is
        // unbounded, so the enclosure contains the origin (and everything
        // else).
        assert!(a.enclose(iv(5.0, 7.0)).contains(Point3::origin()));
        assert!(a.enclose(iv(-7.0, -5.0)).contains(Point3::origin()));
        // A box entirely inside the knot range: no origin union, and the
        // sub-curve hull excludes the origin.
        assert!(!a.enclose(iv(0.25, 0.75)).contains(Point3::origin()));
        assert!(!a.enclose(iv(0.4, 0.6)).contains(Point3::origin()));
    }

    #[test]
    fn bspline_der_enclosures_match_partials() {
        let c = cubic();
        let u = uniform();
        // Cubic witness, tt interior and tt straddling an interior knot of the
        // uniform witness, for the first three orders: every sampled der_n
        // lies in the hodograph hull.
        let cells = [(c, iv(0.25, 0.75)), (u, iv(0.4, 0.6))];
        for (bsp, tt) in cells {
            for n in 1..=3 {
                let enc = bsp.enclose_der(n, tt);
                const N: usize = 30;
                for i in 0..N {
                    let t = tt.inf() + (tt.sup() - tt.inf()) * (i as f64) / (N as f64 - 1.0);
                    let d = bsp.der_n(n, t);
                    assert!(
                        enc.contains(Point3::new(d.x, d.y, d.z)),
                        "der_{n} at t={t} escaped its enclosure {enc:?}"
                    );
                }
            }
        }
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
    fn bspline_tangent_cone_contains_sampled_tangents() {
        let h = helix();
        let tt = iv(0.25, 0.75);
        let cone = h
            .tangent_cone(tt)
            .expect("the helix derivative never vanishes over [0.25, 0.75]");
        const N: usize = 60;
        for i in 0..N {
            let t = tt.inf() + (tt.sup() - tt.inf()) * (i as f64) / (N as f64 - 1.0);
            let d = h.der(t);
            assert!(
                cone_contains(&cone, d),
                "unit tangent at t={t} outside the cone: {:?}",
                d.normalize()
            );
        }
    }

    #[test]
    fn bspline_tangent_cone_refuses_when_the_hodograph_hull_contains_zero() {
        let q = quad();
        // Any box containing t = 1/2, where the derivative 2t − 1 vanishes
        // (and the y, z siblings with it): the hodograph hull contains zero.
        for tt in [iv(0.4, 0.6), iv(0.0, 1.0), iv(0.49, 0.51)] {
            assert!(
                q.tangent_cone(tt).is_none(),
                "quadratic cone over {tt:?} must be None"
            );
        }
        // A box bounded away from t = 1/2: the sub-curve hodograph hull stays
        // clear of zero, so a cone exists.
        assert!(
            q.tangent_cone(iv(0.0, 0.4)).is_some(),
            "quadratic cone over [0, 0.4] must exist"
        );
        assert!(
            helix().tangent_cone(iv(0.25, 0.75)).is_some(),
            "helix cone must exist"
        );
        // A full-period witness whose derivative hull covers every direction:
        // no cone bounds the tangents.
        assert!(
            loop_witness().tangent_cone(iv(0.0, 1.0)).is_none(),
            "loop cone must be None"
        );
    }

    /// Asserts that the hull enclosure is contained in the naive one, up to the
    /// `HULL_PAD`-size outward pad that decision 4 (as corrected for the
    /// measured Boehm rounding) adds to each hull endpoint — far smaller than
    /// the naive-vs-hull gaps this test measures — and is strictly narrower.
    fn assert_hull_strictly_narrower(hull: &Box3, naive: &Box3) {
        let slack = |x: f64| 256.0 * f64::EPSILON * (1.0 + x.abs());
        for (h, n) in [(hull.x, naive.x), (hull.y, naive.y), (hull.z, naive.z)] {
            assert!(
                h.inf() >= n.inf() - slack(n.inf()),
                "hull inf {} escaped below naive inf {}",
                h.inf(),
                n.inf()
            );
            assert!(
                h.sup() <= n.sup() + slack(n.sup()),
                "hull sup {} escaped above naive sup {}",
                h.sup(),
                n.sup()
            );
        }
        assert!(
            hull.width() < naive.width(),
            "hull width {} not strictly below naive width {}",
            hull.width(),
            naive.width()
        );
    }

    #[test]
    fn bspline_enclosure_is_tighter_than_naive_interval_arithmetic() {
        let tt = iv(0.0, 1.0);
        // Quadratic witness x = y = z = t²−t: hull [−1/2, 0], naive Horner
        // (tt·tt − tt) = [−1, 1].
        let q = quad();
        let hull_q = q.enclose(tt);
        let naive_q = Box3 {
            x: tt * tt - tt,
            y: tt * tt - tt,
            z: tt * tt - tt,
        };
        assert_hull_strictly_narrower(&hull_q, &naive_q);
        // Cubic witness x = y = z = t³−t: hull [−2/3, 0], naive Horner
        // (tt·tt − 1)·tt = [−1, 0].
        let c = cubic();
        let hull_c = c.enclose(tt);
        let naive_c = Box3 {
            x: (tt * tt - interval_at(1.0)) * tt,
            y: (tt * tt - interval_at(1.0)) * tt,
            z: (tt * tt - interval_at(1.0)) * tt,
        };
        assert_hull_strictly_narrower(&hull_c, &naive_c);
        // The containment direction that matters: the naive box is sound too —
        // it contains every sampled curve point; only the hull is tight.
        for (bsp, naive) in [(&q, &naive_q), (&c, &naive_c)] {
            const N: usize = 50;
            for i in 0..N {
                let t = (i as f64) / (N as f64 - 1.0);
                assert!(
                    naive.contains(bsp.subs(t)),
                    "naive box did not contain a sampled curve point at t={t}"
                );
            }
        }
    }

    #[test]
    fn bspline_enclosure_converges_under_bisection() {
        let h = helix();
        let mut tt = iv(0.05, 0.95);
        let initial = h.enclose(tt).width();
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
            let cur = h.enclose(tt).width();
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
