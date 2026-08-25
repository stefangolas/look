//! BG-CE-002 — the whole-span leader-vs-carrier deviation certificate.
//!
//! For an edge use with parametric trace `pc_u` on face `f` and leader curve
//! `c_e` with parameter correspondence `phi_u`, certifies
//!
//! ```text
//! || Γ_f(pc_u(t)) − c_e(phi_u(t)) || ≤ τ_e   for ALL t in the span
//! ```
//!
//! by **interval evaluation over the whole span** (BG-ENC-001), never by
//! sampling — sampling is the classic false pass for a claim over a continuum.
//!
//! Two routes.
//!
//! **Route 1** (the main path) applies when both sides are exactly B-splines: a
//! `BSplineCurve<Point3>` directly, or a `PCurve<BSplineCurve<Point2>, Plane>`
//! composed exactly into one (decision 2's `exact_spline`). It subtracts the
//! two curves *as splines* — coefficientwise, after knot merge — and hulls the
//! difference. That kills the interval-dependency problem: an exact-agreement
//! pair has a difference spline whose control points are all ~0, so the
//! whole-span hull certifies at any `tau` with zero subdivisions. Route 1 is
//! one-shot for exact-spline pairs.
//!
//! **Route 2** (the generic bisection fallback) encloses both curves
//! independently per cell and subtracts the boxes. It does not scale to small
//! `tau`: the residual box of two independently-enclosed curves over-estimates
//! by ~`(‖c'‖+‖l'‖)·width` per cell (the interval dependency problem), so
//! certifying `tau` on a span costs `O((‖c'‖+‖l'‖)·span/tau)` subdivisions —
//! callers budget accordingly. Route 1 is one-shot for exact-spline pairs.
//!
//! The certificate's `method` is `Interval` and its sole prop is
//! `SoundEnclosure`: the whole claim is the sound enclosure of the deviation.
//! `margin`/`modulus` follow the house pattern of the analytic modules — no
//! stability claim is made.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use crate::enclosure::{Box3, EnclosureCurve};
use inari::Interval;
use truck_base::cgmath64::{EuclideanSpace, InnerSpace, Point3};
use truck_base::evidence::{
    Budget, Certificate, Certified, Margin, Method, Modulus, Outcome, Prop, PropMap, Refusal,
    Truth, UnresolvedWitness,
};
use truck_geometry::nurbs::{BSplineCurve, KnotVec};
use truck_geotrait::{Cut, ParametricCurve};

/// The relative outward pad per hull endpoint, as a multiple of `EPSILON`.
///
/// Copied from the landed carriers (`bspline.rs` and siblings): the f64
/// recomputations along route 1 — Boehm insertion, degree elevation, knot
/// reversal, sub-curve extraction — perturb the difference spline's control
/// points by ulp-class amounts, and `64 EPSILON (1 + |·|)` covers them with
/// margin at the tolerance scales the certificate targets.
const HULL_PAD: f64 = 64.0 * f64::EPSILON;

/// A degenerate interval from a runtime `f64`. Finite coordinates always
/// construct; a NaN widens to the empty interval rather than panicking (H-1).
/// Duplicated from the sibling carriers, which carry their own copies.
fn interval_at(x: f64) -> Interval {
    Interval::try_from((x, x)).unwrap_or(Interval::EMPTY)
}

/// An interval from two runtime `f64` bounds; a malformed pair (NaN bounds or
/// inf > sup) widens to the empty interval rather than panicking (H-1).
fn interval(lo: f64, hi: f64) -> Interval {
    Interval::try_from((lo, hi)).unwrap_or(Interval::EMPTY)
}

/// What a call consumed: the budget at entry minus what remains. `Budget` is
/// `Copy`; a refusal carries the difference, never the remainder (BG-NUM-001).
/// Duplicated from `truck-geometry`'s `af_surface.rs`.
fn budget_spent(initial: Budget, remaining: Budget) -> Budget {
    Budget {
        subdiv: initial.subdiv - remaining.subdiv,
        newton: initial.newton - remaining.newton,
        depth: initial.depth - remaining.depth,
    }
}

/// The count of the knot value `x` in `bsp`'s knot vector, over **exact**
/// equality. `KnotVec::multiplicity` matches by tolerance and would inflate the
/// count next to a *different* knot within tolerance (decision 0's defect
/// class), which under-inserts in the raising loop; every raising, merging and
/// flipping step in this module counts exactly.
fn exact_count(bsp: &BSplineCurve<Point3>, x: f64) -> usize {
    bsp.knot_vec().iter().filter(|&&k| k == x).count()
}

/// Raises the knot value `x` to full multiplicity `degree + 1` by repeated
/// Boehm insertion — the identical chain the landed carriers use, with decision
/// 0's exact counting. `add_knot` inserts a single exact copy; inserting past
/// `degree + 1` would make an invalid knot vector, so the loop stops exactly at
/// the maximum multiplicity.
fn raise_to_full_multiplicity(bsp: &mut BSplineCurve<Point3>, x: f64, degree: usize) {
    while exact_count(bsp, x) < degree + 1 {
        bsp.add_knot(x);
    }
}

/// The parameter correspondence phi: `phi(t) = scale * t + offset`. Moved to
/// `truck_base::param_map` (BG-SOL-P0-REC) so the structural recognizer in
/// truck-geometry can name it in its witness; re-exported here so
/// `use truck_evidence::ParamMap` keeps resolving.
pub use truck_base::param_map::ParamMap;

/// phi(tt) applied in outward-rounded interval arithmetic — the certified
/// application that the base `ParamMap` (plain f64) intentionally does not
/// carry, because `truck-base` has no `inari` (BG-SOL-P0-REC).
fn apply_param_map(phi: &ParamMap, tt: Interval) -> Interval {
    interval_at(phi.scale) * tt + interval_at(phi.offset)
}

/// The leader under the flip correspondence: knots k -> offset - k (the
/// mapped list reversed back to ascending), control points reversed. Valid
/// only when both endpoint knots are at full multiplicity `degree + 1`;
/// `None` otherwise (the caller falls back to route 2).
fn flipped_spline(leader: &BSplineCurve<Point3>, offset: f64) -> Option<BSplineCurve<Point3>> {
    let degree = leader.degree();
    let first = *leader.knot_vec().first()?;
    let last = *leader.knot_vec().last()?;
    if exact_count(leader, first) < degree + 1 || exact_count(leader, last) < degree + 1 {
        return None;
    }
    // The mapped list is descending; reversed back to ascending.
    let knots: Vec<f64> = leader.knot_vec().iter().map(|k| offset - k).rev().collect();
    let mut cps = leader.control_points().clone();
    cps.reverse();
    let kv = KnotVec::try_from(knots).ok()?;
    Some(BSplineCurve::new_unchecked(kv, cps))
}

/// Raises each curve at the other's distinct knot values until both share
/// one knot vector. Returns false when the two knot vectors cannot be made
/// element-by-element equal (the caller falls back to route 2). A degree
/// mismatch is first elevated away on the lower-degree curve — elevation
/// recomputes control points in f64, ulp-class, covered by the hull pad.
fn merge_knots(a: &mut BSplineCurve<Point3>, b: &mut BSplineCurve<Point3>) -> bool {
    while a.degree() < b.degree() {
        a.elevate_degree();
    }
    while b.degree() < a.degree() {
        b.elevate_degree();
    }
    // The distinct knot values of both, by exact equality (decision 0).
    let mut values: Vec<f64> = Vec::new();
    for &k in a.knot_vec().iter().chain(b.knot_vec().iter()) {
        if !values.contains(&k) {
            values.push(k);
        }
    }
    for &x in &values {
        let target = exact_count(a, x).max(exact_count(b, x));
        while exact_count(a, x) < target {
            a.add_knot(x);
        }
        while exact_count(b, x) < target {
            b.add_knot(x);
        }
    }
    a.knot_vec().len() == b.knot_vec().len()
        && a.knot_vec()
            .iter()
            .zip(b.knot_vec().iter())
            .all(|(x, y)| x == y)
}

/// The per-axis control-point hull box of `bsp`, padded `HULL_PAD (1 + |.|)`
/// outward per endpoint, and additionally unioning the two point values `a`,
/// `b` into each axis. The pad covers the ulp-class recomputations along the
/// way (merge insertion, degree elevation, reversal, extraction); the two
/// point values cover the right-open endpoint semantics (BG-AUD-002): a
/// degree-0 sub-piece carries only the value on `[lo, hi)`, so the value the
/// source difference spline attains at the piece's right endpoint is cut away
/// and must be supplied explicitly (the `hull_sub_curve` boundary pattern).
fn control_point_box(bsp: &BSplineCurve<Point3>, a: Point3, b: Point3) -> Box3 {
    let hull = |i: usize| -> Interval {
        let coord = |p: &Point3| match i {
            0 => p.x,
            1 => p.y,
            _ => p.z,
        };
        let mut mn = f64::INFINITY;
        let mut mx = f64::NEG_INFINITY;
        for p in bsp.control_points().iter() {
            let c = coord(p);
            mn = mn.min(c);
            mx = mx.max(c);
        }
        for p in [&a, &b] {
            let c = coord(p);
            mn = mn.min(c);
            mx = mx.max(c);
        }
        let pad = HULL_PAD * (1.0 + mn.abs().max(mx.abs()));
        Interval::try_from((mn - pad, mx + pad)).unwrap_or(Interval::EMPTY)
    };
    Box3 {
        x: hull(0),
        y: hull(1),
        z: hull(2),
    }
}

/// (sup, inf) of ||v|| for v in the box, by interval arithmetic.
fn norm_bounds(b: &Box3) -> (f64, f64) {
    let norm = (b.x * b.x + b.y * b.y + b.z * b.z).sqrt();
    (norm.sup(), norm.inf())
}

/// Certifies the whole-span deviation bound
/// `|| carrier(t) − leader(phi(t)) || ≤ tau` for every `t ∈ tt`, by interval
/// evaluation (BG-ENC-001) — never by sampling, which is the classic false
/// pass for a claim over a continuum.
///
/// Two routes. **Route 1** (the main path) applies when both curves are exactly
/// B-splines (`EnclosureCurve::exact_spline`) and `phi` is the identity or a
/// flip: it subtracts the two curves *as splines* (coefficientwise, after knot
/// merge) and hulls the difference, which kills the interval-dependency
/// problem and certifies an exact-agreement pair at any `tau` with zero
/// subdivisions. **Route 2** is the generic bisection fallback: per cell it
/// encloses both curves independently, subtracts the boxes per axis and norms
/// the residual, accumulating certified cells and subdividing the rest.
///
/// `tau` is taken as a bare `f64` with NO validity guard: a nonpositive or NaN
/// `tau` is handled by the loop's own logic — `upper <= tau` is false for every
/// cell, no lower bound can prove violation, and the budget or the width floor
/// eventually refuses. Honest refusals, no panics, no special case. Callers
/// derive `tau` from `ToleranceCtx::entity_tau` in real use.
///
/// ```
/// use truck_base::cgmath64::{Point2, Point3};
/// use truck_base::evidence::Budget;
/// use truck_base::tolerance::{TOLERANCE, ToleranceCtx};
/// use truck_evidence::{certify_deviation, ParamMap};
/// use truck_evidence::enclosure::Interval;
/// use truck_geometry::decorators::PCurve;
/// use truck_geometry::nurbs::{BSplineCurve, KnotVec};
/// use truck_geometry::specifieds::Plane;
///
/// // The plane witness S(u, v) = (u, v, u + v) over the parabola (t, t²):
/// // the composed carrier is (t, t², t + t²), and the leader is the same
/// // curve as a plain B-spline, so the exact pair certifies one-shot.
/// let plane = Plane::new(
///     Point3::new(0.0, 0.0, 0.0),
///     Point3::new(1.0, 0.0, 1.0),
///     Point3::new(0.0, 1.0, 1.0),
/// );
/// let parabola = BSplineCurve::new(
///     KnotVec::bezier_knot(2),
///     vec![Point2::new(0.0, 0.0), Point2::new(0.5, 0.0), Point2::new(1.0, 1.0)],
/// );
/// let carrier = PCurve::new(parabola, plane);
/// let leader = BSplineCurve::new(
///     KnotVec::bezier_knot(2),
///     vec![
///         Point3::new(0.0, 0.0, 0.0),
///         Point3::new(0.5, 0.0, 0.5),
///         Point3::new(1.0, 1.0, 2.0),
///     ],
/// );
/// let ctx = ToleranceCtx::new(1.0, TOLERANCE, TOLERANCE, TOLERANCE)
///     .expect("the legacy context values are valid")
///     .value;
/// let tau = ctx.entity_tau(TOLERANCE);
/// let mut budget = Budget::new(1 << 16, 0, 0);
/// let span = Interval::try_from((0.0, 1.0)).expect("valid span");
/// let out = certify_deviation(&leader, &carrier, ParamMap::IDENTITY, span, tau, &mut budget)
///     .expect("the exact pair certifies");
/// assert!(out.value <= tau);
/// ```
pub fn certify_deviation<L, C>(
    leader: &L,
    carrier: &C,
    phi: ParamMap,
    tt: Interval,
    tau: f64,
    budget: &mut Budget,
) -> Outcome<f64>
where
    L: EnclosureCurve,
    C: EnclosureCurve,
{
    // The shared preface: an empty span (or one with NaN/inverted bounds) has
    // nothing to certify on either route.
    if tt.is_empty() || !tt.inf().is_finite() || !tt.sup().is_finite() {
        return Err(Refusal::Empty);
    }
    // Budget is Copy; the entry snapshot makes a refusal's `spent` report what
    // this whole call consumed, including any route-1 subdivisions.
    let initial = *budget;
    match route1(leader, carrier, phi, tt, tau, budget, initial) {
        Some(out) => out,
        None => route2(leader, carrier, phi, tt, tau, budget, initial),
    }
}

/// The certificate of (f): the deviation's sound enclosure is the whole claim.
fn certified_deviation(sup_bound: f64, budget: &Budget) -> Outcome<f64> {
    let mut props = PropMap::new();
    props.set(Prop::SoundEnclosure, Truth::True);
    Ok(Certified::new(
        sup_bound,
        Certificate {
            props,
            method: Method::Interval,
            budget_left: *budget,
            margin: Margin::UNBOUNDED,
            modulus: Modulus::Unbounded,
        },
    ))
}

/// The `NumericallyUnresolved` refusal both routes share: the cell could not be
/// bisected at representable width, or the subdivision budget was exhausted.
fn unresolved(initial: Budget, budget: &Budget) -> Refusal {
    Refusal::NumericallyUnresolved {
        spent: budget_spent(initial, *budget),
        witness: UnresolvedWitness::DeviationUncertified,
    }
}

/// Route 1 — the difference spline. Returns `None` when any precondition fails
/// (a side is not an exact B-spline, `phi` is not identity/flip, the span is
/// outside the difference spline's knot range, or the knot merge cannot
/// equalize the vectors), in which case the caller falls back to route 2.
fn route1<L, C>(
    leader: &L,
    carrier: &C,
    phi: ParamMap,
    tt: Interval,
    tau: f64,
    budget: &mut Budget,
    initial: Budget,
) -> Option<Outcome<f64>>
where
    L: EnclosureCurve,
    C: EnclosureCurve,
{
    let leader_spline = leader.exact_spline()?;
    let carrier_spline = carrier.exact_spline()?;
    // Apply phi to the leader: identity or flip only. Any other map (in
    // particular a `from_ranges` rescaling) falls back to route 2.
    let leader_spline = if phi == ParamMap::IDENTITY {
        leader_spline
    } else if phi.scale == -1.0 {
        flipped_spline(&leader_spline, phi.offset)?
    } else {
        return None;
    };
    // Equalize degrees, merge the knot vectors, subtract coefficientwise. The
    // difference spline's control points are tight around the true residual,
    // which is exactly what kills the interval-dependency problem.
    let mut carrier_spline = carrier_spline;
    let mut leader_spline = leader_spline;
    if !merge_knots(&mut carrier_spline, &mut leader_spline) {
        return None;
    }
    let diff_cps: Vec<Point3> = carrier_spline
        .control_points()
        .iter()
        .zip(leader_spline.control_points().iter())
        .map(|(c, l)| Point3::new(c.x - l.x, c.y - l.y, c.z - l.z))
        .collect();
    let diff = BSplineCurve::new_unchecked(carrier_spline.knot_vec().clone(), diff_cps);
    // The certification span must lie inside the difference spline's knot
    // range, and must be a proper (non-degenerate) span. `lo` and `hi` are
    // finite here (the entry preface rejected NaN bounds), so `lo >= hi` is
    // the correct degenerate test.
    let lo = tt.inf();
    let hi = tt.sup();
    let first = *diff.knot_vec().first()?;
    let last = *diff.knot_vec().last()?;
    if lo < first || hi > last || lo >= hi {
        return None;
    }
    // Raise both endpoints to exact full multiplicity, then extract the
    // sub-piece the same way the landed carriers' `sub_curve` does: `cut`
    // mutates self to the FRONT piece and returns the TAIL, so cut(hi)
    // discards the tail and cut(lo) yields the middle piece [lo, hi].
    let degree = diff.degree();
    let mut raised = diff.clone();
    for x in [lo, hi] {
        raise_to_full_multiplicity(&mut raised, x, degree);
    }
    let mut c = raised;
    let _tail = c.cut(hi);
    let piece = c.cut(lo);
    // Worklist over pieces: accumulate certified cells, prove violations, and
    // bisect the rest at the parameter midpoint. Because the pre-raised cuts
    // leave a piece's knot range equal to its span, the endpoints come
    // straight off the knot vector.
    let mut worklist: Vec<BSplineCurve<Point3>> = vec![piece];
    let mut sup_bound: f64 = 0.0;
    while let Some(piece) = worklist.pop() {
        // The piece's knot-range span `[a, b]` in the ORIGINAL difference
        // spline's parameter space (the pre-raised cuts leave a piece's knot
        // range equal to its span). `subs` is right-open at interior knots
        // (knot_vec.rs), so for a degree-0 piece the value `diff` attains at
        // `b` lives in the NEXT span and is omitted by the piece's own control
        // points; unioning `diff.subs(a)` and `diff.subs(b)` into the hull
        // keeps the certificate sound at the right-open endpoints (BG-AUD-002,
        // the `hull_sub_curve` boundary pattern). For degree >= 1 the endpoint
        // values lie inside the piece hull up to rounding and change nothing.
        let a = *piece.knot_vec().first()?;
        let b = *piece.knot_vec().last()?;
        let va = diff.subs(a);
        let vb = diff.subs(b);
        let (upper, lower) = norm_bounds(&control_point_box(&piece, va, vb));
        if upper <= tau {
            sup_bound = sup_bound.max(upper);
        } else if lower > tau {
            return Some(Err(Refusal::ForwardToleranceExceeded {
                bound: lower,
                allowed: tau,
            }));
        } else {
            // The hull is ambiguous, but the two unioned endpoint values are
            // genuine parameters IN the certified span: `diff.subs(a)` and
            // `diff.subs(b)` are the deviation at those parameters (right-open
            // at `b`, which is the span's value there). A single parameter
            // value with deviation exceeding `tau` refutes the whole-span
            // claim, and for a degree-0 piece it is the only way the
            // cut-away right-open endpoint can be proved out (the piece hull
            // also contains the interior value, pinning its infimum at 0).
            let da = va.to_vec().magnitude();
            let db = vb.to_vec().magnitude();
            if da > tau || db > tau {
                return Some(Err(Refusal::ForwardToleranceExceeded {
                    bound: da.max(db),
                    allowed: tau,
                }));
            }
            let mid = (a + b) / 2.0;
            if !(a < mid && mid < b) {
                return Some(Err(unresolved(initial, budget)));
            }
            if budget.spend_subdiv(1).is_err() {
                return Some(Err(unresolved(initial, budget)));
            }
            let mut c = piece;
            raise_to_full_multiplicity(&mut c, mid, degree);
            let tail = c.cut(mid);
            worklist.push(tail);
            worklist.push(c);
        }
    }
    Some(certified_deviation(sup_bound, budget))
}

/// Route 2 — the generic bisection fallback, exactly the box-minus-box loop
/// the cost model measured. Per cell the residual box is the carrier's
/// enclosure minus the leader's enclosure at the mapped span, per axis; the
/// norm's sup accumulates into the bound, its inf proves a violation, and
/// anything in between is bisected at the midpoint.
fn route2<L, C>(
    leader: &L,
    carrier: &C,
    phi: ParamMap,
    tt: Interval,
    tau: f64,
    budget: &mut Budget,
    initial: Budget,
) -> Outcome<f64>
where
    L: EnclosureCurve,
    C: EnclosureCurve,
{
    let mut sup_bound: f64 = 0.0;
    let mut worklist: Vec<Interval> = vec![tt];
    while let Some(cell) = worklist.pop() {
        let carrier_box = carrier.enclose(cell);
        let leader_box = leader.enclose(apply_param_map(&phi, cell));
        let residual = Box3 {
            x: carrier_box.x - leader_box.x,
            y: carrier_box.y - leader_box.y,
            z: carrier_box.z - leader_box.z,
        };
        let (upper, lower) = norm_bounds(&residual);
        if upper <= tau {
            sup_bound = sup_bound.max(upper);
        } else if lower > tau {
            return Err(Refusal::ForwardToleranceExceeded {
                bound: lower,
                allowed: tau,
            });
        } else {
            let lo = cell.inf();
            let hi = cell.sup();
            let mid = (lo + hi) / 2.0;
            if !(lo < mid && mid < hi) {
                return Err(unresolved(initial, budget));
            }
            if budget.spend_subdiv(1).is_err() {
                return Err(unresolved(initial, budget));
            }
            worklist.push(interval(mid, hi));
            worklist.push(interval(lo, mid));
        }
    }
    certified_deviation(sup_bound, budget)
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect on paths reachable from untrusted
// geometry. Unit-test assertions on hand-built witnesses are not such a path.
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::enclosure::EnclosureSurface;
    use truck_base::cgmath64::{InnerSpace, Point2, Vector3};
    use truck_base::tolerance::{ToleranceCtx, TOLERANCE};
    use truck_geometry::decorators::PCurve;
    use truck_geometry::specifieds::{Line, Plane};
    use truck_geotrait::ParametricCurve;

    /// The route-2 tolerance for the line-pair tests, a dimensionless
    /// deviation tolerance at unit scale.
    const ROUTE2_TAU: f64 = 1.0e-4; // H-3: a dimensionless deviation tolerance for the line-pair witness, not a length

    /// The sampling slack of the falsification guard: sampling may only
    /// falsify, never establish, so a sampled deviation may exceed the
    /// certified bound by at most this rounding allowance.
    const SAMPLING_SLACK: f64 = 1.0e-12; // H-3: sampling slack, not a length tolerance

    /// Build a test interval, degrading to EMPTY (and failing its assertion)
    /// rather than panicking on a malformed bound.
    fn iv(lo: f64, hi: f64) -> Interval {
        Interval::try_from((lo, hi)).unwrap_or(Interval::EMPTY)
    }

    /// The legacy unscaled tolerance context, built with the real
    /// constructor: `ToleranceCtx::new(1.0, TOLERANCE, ...)` is numerically the
    /// Stage-A scaffold, but avoids the GATE-4 ratchet, which counts uses of
    /// that scaffold's constructor against a ceiling that only moves down.
    fn legacy_ctx() -> ToleranceCtx {
        ToleranceCtx::new(1.0, TOLERANCE, TOLERANCE, TOLERANCE)
            .expect("the legacy context values are valid")
            .value
    }

    /// The plane witness's surface: `S(u, v) = (u, v, u + v)`, an oblique slab
    /// whose two partials are distinct. Copied from `pcurve.rs`'s test module.
    fn plane() -> Plane {
        Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 1.0),
            Point3::new(0.0, 1.0, 1.0),
        )
    }

    /// The quadratic Bézier `c(t) = (t, t²)` on `[0, 1]`, control points
    /// `(0, 0), (1/2, 0), (1, 1)`. Copied from `pcurve.rs`'s test module.
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

    /// The carrier witness: `PCurve(parabola2, plane)`, composed to
    /// `(t, t², t + t²)`.
    fn carrier_witness() -> PCurve<BSplineCurve<Point2>, Plane> {
        PCurve::new(parabola2(), plane())
    }

    /// The leader witness: the SAME curve as a `BSplineCurve<Point3>` with
    /// control points `(0,0,0), (1/2,0,1/2), (1,1,2)` on
    /// `KnotVec::bezier_knot(2)` — Bernstein: x = t, y = t², z = t + t².
    /// Bit-exact agreement with the flattened carrier.
    fn leader_witness() -> BSplineCurve<Point3> {
        BSplineCurve::new(
            KnotVec::bezier_knot(2),
            vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(0.5, 0.0, 0.5),
                Point3::new(1.0, 1.0, 2.0),
            ],
        )
    }

    #[test]
    fn exact_spline_exposes_the_plane_composition() {
        let flat = carrier_witness()
            .exact_spline()
            .expect("the planar pcurve composes exactly");
        assert_eq!(flat, leader_witness());
        assert!(plane().as_plane().is_some());
        let line = Line(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0));
        assert!(line.exact_spline().is_none());
    }

    #[test]
    fn route1_exact_pair_certifies_one_shot() {
        let tau = legacy_ctx().entity_tau(TOLERANCE);
        let mut budget = Budget::new(1 << 16, 0, 0);
        let initial = budget;
        let out = certify_deviation(
            &leader_witness(),
            &carrier_witness(),
            ParamMap::IDENTITY,
            iv(0.0, 1.0),
            tau,
            &mut budget,
        )
        .expect("the exact pair certifies");
        assert!(out.value <= tau);
        // The route-1 claim: zero subdivisions spent.
        assert_eq!(budget.subdiv, initial.subdiv);
        assert_eq!(out.cert.method, Method::Interval);
        assert_eq!(out.cert.props.get(Prop::SoundEnclosure), Truth::True);
    }

    #[test]
    fn route1_offset_pcurve_fails_one_shot() {
        let tau = legacy_ctx().entity_tau(TOLERANCE);
        // The leader translated by 2·tau in z: the difference spline's z
        // control points sit at −2·tau, so the hull's lower norm bound proves
        // the violation outright. A checker that passes everything is the
        // failure mode this test exists for.
        let mut leader = leader_witness();
        leader.transform_control_points(|p| *p += Vector3::unit_z() * (2.0 * tau));
        let mut budget = Budget::new(1 << 16, 0, 0);
        let initial = budget;
        let err = certify_deviation(
            &leader,
            &carrier_witness(),
            ParamMap::IDENTITY,
            iv(0.0, 1.0),
            tau,
            &mut budget,
        )
        .unwrap_err();
        match err {
            Refusal::ForwardToleranceExceeded { bound, allowed } => {
                assert!(bound > tau, "bound {bound} must exceed tau {tau}");
                assert_eq!(allowed, tau);
            }
            other => unreachable!("expected ForwardToleranceExceeded, got {other:?}"),
        }
        // Decisive violation with zero subdivisions.
        assert_eq!(budget.subdiv, initial.subdiv);
    }

    #[test]
    fn route1_flip_correspondence_certifies() {
        let tau = legacy_ctx().entity_tau(TOLERANCE);
        // The reversed leader: control points in reverse order, so the curve
        // `L(s) = carrier(1 − s)`. With phi = flip(0, 1), `L(phi(t)) =
        // carrier(t)` and the pair is exact.
        let reversed = BSplineCurve::new(
            KnotVec::bezier_knot(2),
            vec![
                Point3::new(1.0, 1.0, 2.0),
                Point3::new(0.5, 0.0, 0.5),
                Point3::new(0.0, 0.0, 0.0),
            ],
        );
        let mut budget = Budget::new(1 << 16, 0, 0);
        let initial = budget;
        let out = certify_deviation(
            &reversed,
            &carrier_witness(),
            ParamMap::flip(0.0, 1.0),
            iv(0.0, 1.0),
            tau,
            &mut budget,
        )
        .expect("the flipped pair certifies");
        assert!(out.value <= tau);
        assert_eq!(budget.subdiv, initial.subdiv);
    }

    #[test]
    fn route1_degree_mismatch_elevates_and_certifies() {
        let tau = legacy_ctx().entity_tau(TOLERANCE);
        // Elevating the flattened carrier once keeps the same curve but raises
        // the degree 2 → 3; certifying against the degree-2 leader exercises
        // merge_knots's elevation step and must stay one-shot.
        let mut flat = carrier_witness()
            .exact_spline()
            .expect("the planar pcurve composes exactly");
        flat.elevate_degree();
        assert_eq!(flat.degree(), 3);
        let mut budget = Budget::new(1 << 16, 0, 0);
        let initial = budget;
        let out = certify_deviation(
            &leader_witness(),
            &flat,
            ParamMap::IDENTITY,
            iv(0.0, 1.0),
            tau,
            &mut budget,
        )
        .expect("elevating the same curve keeps the pair exact");
        assert!(out.value <= tau);
        assert_eq!(budget.subdiv, initial.subdiv);
    }

    #[test]
    fn route2_line_pair_with_rescaled_range_certifies() {
        // The fallback: lines never expose `exact_spline`, so this exercises
        // route 2 end to end with an exact-agreement pair under a rescaling
        // correspondence.
        //
        // carrier: t ∈ [0, 1] → (2t, 0, 0), the segment (0,0,0)→(2,0,0) at
        //          speed 2.
        // leader:  s ∈ [0, 2] → (s, 0, 0), the same segment at speed 1.
        // phi:     from_ranges(0, 1, 0, 2) = phi(t) = 2t, so
        //          leader(phi(t)) = (2t, 0, 0) = carrier(t).
        let carrier = Line(Point3::new(0.0, 0.0, 0.0), Point3::new(2.0, 0.0, 0.0));
        let leader = Line(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0));
        let phi = ParamMap::from_ranges(0.0, 1.0, 0.0, 2.0).expect("non-degenerate range");
        let mut budget = Budget::new(1 << 16, 0, 0);
        let initial = budget;
        let out = certify_deviation(
            &leader,
            &carrier,
            phi,
            iv(0.0, 1.0),
            ROUTE2_TAU,
            &mut budget,
        )
        .expect("the rescaled line pair certifies under route 2");
        assert!(out.value <= ROUTE2_TAU);
        // Route 2's loop is load-bearing here: it actually subdivided.
        assert!(budget.subdiv < initial.subdiv);
    }

    #[test]
    fn route2_budget_exhaustion_refuses() {
        let carrier = Line(Point3::new(0.0, 0.0, 0.0), Point3::new(2.0, 0.0, 0.0));
        let leader = Line(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0));
        let phi = ParamMap::from_ranges(0.0, 1.0, 0.0, 2.0).expect("non-degenerate range");
        let mut budget = Budget::new(0, 0, 0);
        let err = certify_deviation(
            &leader,
            &carrier,
            phi,
            iv(0.0, 1.0),
            ROUTE2_TAU,
            &mut budget,
        )
        .unwrap_err();
        // Nothing was spendable, and the witness names the deviation
        // certificate as the unresolved operation.
        match err {
            Refusal::NumericallyUnresolved { spent, witness } => {
                assert_eq!(spent.subdiv, 0);
                assert_eq!(witness, UnresolvedWitness::DeviationUncertified);
            }
            other => unreachable!("expected NumericallyUnresolved, got {other:?}"),
        }
    }

    #[test]
    fn deviation_empty_span_refuses_empty() {
        let mut budget = Budget::new(1 << 16, 0, 0);
        let empty = certify_deviation(
            &leader_witness(),
            &carrier_witness(),
            ParamMap::IDENTITY,
            Interval::EMPTY,
            ROUTE2_TAU,
            &mut budget,
        );
        assert!(matches!(empty, Err(Refusal::Empty)));
        // A NaN-bound box is empty too, and hits the same shared preface.
        let nan_box = iv(f64::NAN, 1.0);
        let err = certify_deviation(
            &leader_witness(),
            &carrier_witness(),
            ParamMap::IDENTITY,
            nan_box,
            ROUTE2_TAU,
            &mut budget,
        );
        assert!(matches!(err, Err(Refusal::Empty)));
    }

    #[test]
    fn deviation_bound_dominates_sampled_deviations() {
        // The falsification guard: sampling may only falsify, never establish.
        // Certify the exact pair on the interior span [0.2, 0.8] by route 1,
        // then verify that every sampled deviation lies at or below the
        // certified bound (plus rounding slack).
        let tau = legacy_ctx().entity_tau(TOLERANCE);
        let mut budget = Budget::new(1 << 16, 0, 0);
        let out = certify_deviation(
            &leader_witness(),
            &carrier_witness(),
            ParamMap::IDENTITY,
            iv(0.2, 0.8),
            tau,
            &mut budget,
        )
        .expect("the exact pair certifies on the interior span");
        const N: usize = 200;
        let leader = leader_witness();
        let carrier = carrier_witness();
        for i in 0..N {
            let t = 0.2 + 0.6 * (i as f64) / (N as f64 - 1.0);
            let deviation =
                (carrier.subs(t) - leader.subs(ParamMap::IDENTITY.apply_f64(t))).magnitude();
            assert!(
                deviation <= out.value + SAMPLING_SLACK,
                "sampled deviation {deviation} exceeds the certified bound {} + slack at t = {t}",
                out.value
            );
        }
    }

    #[test]
    fn route1_degree0_half_span_endpoint_deviation_refuses() {
        // BG-AUD-002 witness: carrier is the degree-0 spline with value 0 on
        // [0, 0.5) and 1 on [0.5, 1], the leader is the identically-zero
        // degree-0 spline on the same knots. The half span [0, 0.5] must
        // refuse: the right-open endpoint convention (knot_vec.rs) evaluates
        // subs(0.5) = 1, which the sub-piece [0, 0.5) omits, so the true
        // deviation at t = 0.5 is 1 > tau = 0.5. Before the AUD-002 fix this
        // half span certified a bound near zero — the cut-away hull.
        let knots = KnotVec::try_from(vec![0.0, 0.5, 1.0]).expect("sorted");
        let carrier = BSplineCurve::new(
            knots.clone(),
            vec![Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 0.0, 1.0)],
        );
        let leader = BSplineCurve::new(
            knots,
            vec![Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 0.0, 0.0)],
        );
        let mut budget = Budget::new(1 << 16, 0, 0);
        let err = certify_deviation(
            &leader,
            &carrier,
            ParamMap::IDENTITY,
            iv(0.0, 0.5),
            0.5,
            &mut budget,
        )
        .expect_err("the degree-0 half span must refuse: the true endpoint deviation is 1");
        assert!(
            matches!(err, Refusal::ForwardToleranceExceeded { .. }),
            "expected ForwardToleranceExceeded, got {err:?}"
        );
    }

    #[test]
    fn route1_degree0_exact_pair_still_certifies() {
        // The same knots and span as the refusal witness, but with the leader
        // equal to the carrier (both control points (0,0,0) and (0,0,1)): the
        // exact degree-0 pair must still certify at tau = 0.5. The union fix
        // adds the endpoint value (0,0,1) to the hull, which the zero
        // difference spline's hull already contains, so the fix must not turn
        // exact degree-0 pairs into refusals.
        let knots = KnotVec::try_from(vec![0.0, 0.5, 1.0]).expect("sorted");
        let carrier = BSplineCurve::new(
            knots.clone(),
            vec![Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 0.0, 1.0)],
        );
        let leader = BSplineCurve::new(
            knots,
            vec![Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 0.0, 1.0)],
        );
        let mut budget = Budget::new(1 << 16, 0, 0);
        let out = certify_deviation(
            &leader,
            &carrier,
            ParamMap::IDENTITY,
            iv(0.0, 0.5),
            0.5,
            &mut budget,
        )
        .expect("the exact degree-0 pair certifies on the half span");
        assert!(out.value <= 0.5);
    }
}
