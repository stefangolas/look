//! BG-FID-005: the `rep` operator — the ONLY sanctioned path from an exact
//! result into the emitted geometry class. `rep_curve` approximates ONE exact
//! CURVE component to `tau_rep` over a certified partition and returns the
//! achieved error, the achieved tangent margin AND the degree-one certificate
//! TOGETHER — never a bare curve, and never (eps, theta) alone, since
//! (eps, theta) without the (iv) discharge is precisely the unsound pairing
//! (conditions (i)-(iii) pass on a double cover; nothing above the certificate
//! is sound if (iv) is missing).
//!
//! The design point this packet exists to honour: `rep` already subdivides to
//! hit (eps, theta), so its cell decomposition IS the partition that the
//! (iv-b) form of the one-sheet condition requires — per-cell fibre-block
//! containment, per-cell injectivity and non-adjacent separation cost no new
//! subdivision structure, only new assertions on boxes the loop already
//! computes. Implementing (iv) as a separate post-pass over an opaque emitted
//! curve is the expensive way to get the same certificate and is a review
//! reject.
//!
//! The emitter shares the exact curve's parameter space, so cell `D_j` of the
//! emitted curve and cell `I_j` of the exact curve are the SAME interval: the
//! (iv-b) pairing is the identity and no search is needed. The per-cell
//! discharge is:
//!
//! 1. **fibre-block containment (a)** — `sup_distance(H_j, E_j) <= eps_now`,
//!    already guaranteed by the eps measurement, together with item 3 below;
//! 2. **per-cell injectivity (b)** — the knot-projection correspondence: every
//!    INTERIOR knot `t*` of the partition has its projected parameter within
//!    the shared closure of its two cells, certified by isolating the
//!    implicit-function zero `G(s) = <phi(t*) - X(s), X'(s)>` over a small
//!    `s`-interval around `t*` (Krawczyk, BG-NUM-003) and requiring the unique
//!    zero box to touch `t*`;
//! 3. **non-adjacent separation (c)** — for every pair `(j, k)` with `k`
//!    non-adjacent to `j` (`|j-k| = 1` is adjacent, PLUS wrap adjacency for
//!    `Closed`): `box_distance(H_j, E_k) > eps_now`, over the balanced BVH
//!    exposed from [`super::isotopy`] — no O(N^2) scan.
//!
//! What a positive answer establishes is (i)-(iii) of the isotopy conditions
//! between the exact and the emitted curve ON THIS PARTITION plus (iv-b)
//! per-cell fibre-block degree one on the same partition. It establishes NOT
//! isotopy, homeomorphism, side separation, whole-span one-sheet as a
//! topological claim, reach semantics, or the surface case.
//!
//! Scope, decided for you: CURVE components only (REP-CRV-001). The surface
//! rep (REP-SRF-001), the surface (iv-b) discharge and the surface
//! double-sheet negative test are BG-FID-005-SRF, a separate packet; the
//! deferral is documented in the module docs, never stubbed.
//!
//! `sigma_cl` is NOT gated here: standalone rep has no arrangement context;
//! BG-FID-006's consumer adds its condition where it exists.
//!
//! The surface case and the discharge (iv-b) wait on BG-FID-005-SRF.

#![deny(clippy::unwrap_used)]

use super::isotopy::{
    angle_pass_form, box_distance, build_tree, curvature_radius_lower_span, interval,
    self_separation_lower_span, sup_distance_box, uniform_cells, CurveBoundary,
    CurveScaleComponents, KdCell, KdNode,
};
use crate::enclosure::{interval_at, Box3, EnclosureCurve, Interval};
use crate::num::krawczyk::{krawczyk, KrawczykProof, KrawczykSystem};
use std::ops::Bound;
use truck_base::cgmath64::{EuclideanSpace, Point3, Vector3, Zero};
use truck_base::evidence::{Budget, EnvelopeCase, Refusal, UnresolvedWitness};
use truck_geotrait::{ParameterRange, ParametricCurve};

/// Typed refusal. Mirrors the spec's refusal names; converts into the
/// landed §4 `Refusal` (whose `EnvelopeCase::ReachTooSmall` arm is
/// documented for exactly this packet). `Refusal` has no invalid-input
/// arm and is not stretched: garbage input is `InvalidMargin` here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepError {
    /// `tau_rep <= 0` / non-finite, `arc_gap <= 0` / non-finite, or a
    /// non-finitely-bounded exact span.
    InvalidMargin,
    /// The scale components could not be certified at all (collapsing
    /// geometry: a corner's tangent enclosure contains both branch
    /// directions at every refinement). Routes to §5 collapse via
    /// [`RepError::into_refusal`]. NEVER fired merely because `tube_scale` is
    /// small: small-but-positive refines (Decision 3).
    ReachTooSmall,
    /// Refinement did not reach target within budget, or eps stalled above
    /// target at the enclosure width floor. Carries the spend; never a
    /// best-effort curve.
    Unresolved { subdivisions: u32 },
}

impl RepError {
    /// The §4-level view of this refusal.
    ///
    /// `ReachTooSmall` converts to `UnsupportedEnvelope(ReachTooSmall)`, the
    /// §5 collapse route this packet owns. `Unresolved` converts to
    /// `NumericallyUnresolved` carrying the subdivision spend. `InvalidMargin`
    /// has NO §4 arm — garbage input is `InvalidMargin` here precisely because
    /// `Refusal` is not stretched — so its conversion is `debug_assert!`d
    /// never to fire and returns the nearest arm (a zero-spend unresolved)
    /// documenting why.
    pub fn into_refusal(self) -> Refusal {
        match self {
            RepError::ReachTooSmall => Refusal::UnsupportedEnvelope(EnvelopeCase::ReachTooSmall),
            RepError::InvalidMargin => {
                debug_assert!(
                    false,
                    "InvalidMargin has no §4 arm; rep_curve validates its inputs before any work"
                );
                Refusal::NumericallyUnresolved {
                    spent: Budget::new(0, 0, 0),
                    witness: UnresolvedWitness::UncertifiedContainment,
                }
            }
            RepError::Unresolved { subdivisions } => Refusal::NumericallyUnresolved {
                spent: Budget::new(subdivisions, 0, 0),
                witness: UnresolvedWitness::DeviationUncertified,
            },
        }
    }
}

/// The emitted approximant: piecewise cubic Hermite in Bezier form over a
/// certified partition (Decision 2). Implements [`ParametricCurve`] +
/// [`EnclosureCurve`] via the Bernstein hull property, so every downstream
/// consumer (including `curve_isotopy_conditions` itself) consumes it through
/// the same trait as any other curve.
///
/// Per cell `[a, b]` of the partition, positions and tangents are the
/// MIDPOINTS of the exact curve's degenerate endpoint enclosures (deterministic;
/// a wrong-but-deterministic choice is correctable, an unstable one is not):
///
/// ```text
/// p0 = X(a),  p3 = X(b)
/// p1 = p0 + (h/3) * T(a),  p2 = p3 - (h/3) * T(b)      # T = tangent midpoint
/// ```
#[derive(Debug, Clone)]
pub struct HermiteCurve {
    /// Ascending partition knots, `len = cells + 1`.
    knots: Vec<f64>,
    /// One cubic Hermite cell per partition interval.
    cells: Vec<HermiteCell>,
    /// The parameter span (the exact curve's span; same parameter space).
    lo: f64,
    /// The parameter span (the exact curve's span; same parameter space).
    hi: f64,
}

/// One cubic Hermite cell in Bezier form over `[a, b]`.
#[derive(Debug, Clone, Copy)]
struct HermiteCell {
    /// Cell start parameter.
    a: f64,
    /// Cell end parameter.
    b: f64,
    /// `b - a`.
    h: f64,
    /// Bezier control point at `a`.
    p0: Point3,
    /// Bezier control point at `a + h/3`.
    p1: Point3,
    /// Bezier control point at `b - h/3`.
    p2: Point3,
    /// Bezier control point at `b`.
    p3: Point3,
}

impl HermiteCell {
    /// The constant third derivative: `6(p3 - 3p2 + 3p1 - p0) / h^3`.
    fn der3_vec(&self) -> Vector3 {
        let d = (self.p3 - self.p0) - (self.p2 - self.p1) * 3.0;
        d * (6.0 / (self.h * self.h * self.h))
    }

    /// The Bezier control points of the curve restricted to `[lo, hi]`, where
    /// `[lo, hi]` is inside this cell's span. Restriction is two de Casteljau
    /// splits; the hull of the restricted control points is a TIGHT Bernstein
    /// enclosure of the curve over the sub-interval (the whole-cell control
    /// hull would over-state by a whole cell's width).
    fn restrict(&self, lo: f64, hi: f64) -> [Point3; 4] {
        let s1 = (lo - self.a) / self.h;
        let s2 = (hi - self.a) / self.h;
        let full = [self.p0, self.p1, self.p2, self.p3];
        if s1 >= 1.0 {
            // The sub-interval is degenerate at the cell's end (a knot):
            // `(s2 - s1)/(1 - s1)` would divide by zero; the segment is the
            // single point p3.
            return [self.p3, self.p3, self.p3, self.p3];
        }
        if s2 <= 0.0 {
            // Degenerate at the cell's start.
            return [self.p0, self.p0, self.p0, self.p0];
        }
        let (_, right) = bezier_split(full, s1);
        let t2 = (s2 - s1) / (1.0 - s1);
        let (sub, _) = bezier_split(right, t2);
        sub
    }
}

/// Split a cubic Bezier at parameter `t` into its left and right sub-curves.
fn bezier_split(p: [Point3; 4], t: f64) -> ([Point3; 4], [Point3; 4]) {
    let [p0, p1, p2, p3] = p;
    let q0 = lerp(p0, p1, t);
    let q1 = lerp(p1, p2, t);
    let q2 = lerp(p2, p3, t);
    let r0 = lerp(q0, q1, t);
    let r1 = lerp(q1, q2, t);
    let s0 = lerp(r0, r1, t);
    ([p0, q0, r0, s0], [s0, r1, q2, p3])
}

/// The first-derivative control points of a cubic, divided by `h`.
fn der_controls(sub: [Point3; 4], h: f64) -> [Vector3; 3] {
    let [s0, s1, s2, s3] = sub;
    let k = 3.0 / h;
    [(s1 - s0) * k, (s2 - s1) * k, (s3 - s2) * k]
}

/// The second-derivative control points of a cubic, divided by `h^2`.
fn der2_controls(sub: [Point3; 4], h: f64) -> [Vector3; 2] {
    let [s0, s1, s2, s3] = sub;
    let k = 6.0 / (h * h);
    [((s2 - s1) - (s1 - s0)) * k, ((s3 - s2) - (s2 - s1)) * k]
}

impl HermiteCurve {
    /// Build the Hermite curve over the given ascending knots from the exact
    /// curve, with endpoint tangents taken as the exact curve's degenerate
    /// tangent-enclosure midpoints.
    fn build(exact: &impl EnclosureCurve, knots: Vec<f64>) -> HermiteCurve {
        let lo = knots.first().copied().unwrap_or(0.0);
        let hi = knots.last().copied().unwrap_or(0.0);
        let mut cells = Vec::with_capacity(knots.len().saturating_sub(1));
        for pair in knots.windows(2) {
            if let [a, b] = pair {
                let h = b - a;
                let p0 = exact.subs(*a);
                let p3 = exact.subs(*b);
                let ta = tangent_midpoint(exact, *a);
                let tb = tangent_midpoint(exact, *b);
                cells.push(HermiteCell {
                    a: *a,
                    b: *b,
                    h,
                    p0,
                    p1: p0 + ta * (h / 3.0),
                    p2: p3 - tb * (h / 3.0),
                    p3,
                });
            }
        }
        HermiteCurve {
            knots,
            cells,
            lo,
            hi,
        }
    }

    /// The index of the cell containing parameter `t` (the first one, at a
    /// shared knot). `t` is inside the span by construction at every call
    /// site; the fallback clamps to the last cell.
    fn cell_index(&self, t: f64) -> usize {
        let j = self.knots.partition_point(|k| *k <= t);
        let n = self.cells.len();
        let idx = j.saturating_sub(1);
        if idx < n {
            idx
        } else {
            n.saturating_sub(1)
        }
    }

    /// Evaluate the Bezier at `t`.
    fn eval(&self, t: f64) -> Point3 {
        let idx = self.cell_index(t);
        match self.cells.get(idx) {
            Some(c) => {
                let s = (t - c.a) / c.h;
                bezier(c.p0, c.p1, c.p2, c.p3, s)
            }
            None => Point3::new(0.0, 0.0, 0.0),
        }
    }

    /// Evaluate the first derivative at `t`.
    fn eval_der(&self, t: f64) -> Vector3 {
        let idx = self.cell_index(t);
        match self.cells.get(idx) {
            Some(c) => {
                let s = (t - c.a) / c.h;
                bezier_der(c.p0, c.p1, c.p2, c.p3, s, c.h)
            }
            None => Vector3::zero(),
        }
    }
}

impl ParametricCurve for HermiteCurve {
    type Point = Point3;
    type Vector = Vector3;

    fn subs(&self, t: f64) -> Point3 {
        self.eval(t)
    }

    fn der(&self, t: f64) -> Vector3 {
        self.eval_der(t)
    }

    fn der2(&self, t: f64) -> Vector3 {
        let idx = self.cell_index(t);
        match self.cells.get(idx) {
            Some(c) => {
                let s = (t - c.a) / c.h;
                bezier_der2(c.p0, c.p1, c.p2, c.p3, s, c.h)
            }
            None => Vector3::zero(),
        }
    }

    fn der_n(&self, n: usize, t: f64) -> Vector3 {
        match n {
            0 => self.subs(t).to_vec(),
            1 => self.der(t),
            2 => self.der2(t),
            3 => {
                let idx = self.cell_index(t);
                match self.cells.get(idx) {
                    Some(c) => c.der3_vec(),
                    None => Vector3::zero(),
                }
            }
            _ => Vector3::zero(),
        }
    }

    fn parameter_range(&self) -> ParameterRange {
        (Bound::Included(self.lo), Bound::Included(self.hi))
    }
}

/// Whether a curve cell's interval overlaps a query interval. Closed intervals
/// touch at shared boundary points, so a naive intersection test would pull
/// the neighbouring cells' control hulls into every cell's enclosure and
/// inflate it by ~3 cells' width (measured: the depth-13 circle hull came out
/// 3x too wide). A cell contributes to `enclose(tt)` only when its interior
/// overlaps `tt`'s interior, or when `tt` is a degenerate point inside the
/// cell.
fn cell_overlaps(cell: Interval, tt: Interval) -> bool {
    let inter = cell.intersection(tt);
    if inter.is_empty() {
        return false;
    }
    if inter.wid() > 0.0 {
        return true;
    }
    // A degenerate intersection: `tt` is a point on the cell boundary (or the
    // cell touches `tt` at one end). Include it only when `tt` itself is that
    // point and lies inside the cell.
    !tt.is_empty() && tt.inf() == tt.sup() && tt.inf() >= cell.inf() && tt.sup() <= cell.sup()
}

impl EnclosureCurve for HermiteCurve {
    fn enclose(&self, tt: Interval) -> Box3 {
        let mut acc = Box3::empty();
        for c in &self.cells {
            let cell = interval(c.a, c.b);
            if cell_overlaps(cell, tt) {
                let lo = tt.inf().max(c.a);
                let hi = tt.sup().min(c.b);
                let sub = c.restrict(lo, hi);
                acc = hull_join(&acc, &hull_box(&sub));
            }
        }
        acc
    }

    fn enclose_der(&self, n: usize, tt: Interval) -> Box3 {
        // A degenerate interval is a single parameter point: the derivative is
        // the CURVE's tangent there, not the derivative of a degenerate
        // sub-curve (which would be the zero vector). The curve is C1 at the
        // knots, so either adjacent cell evaluates the same tangent.
        if !tt.is_empty() && tt.inf() == tt.sup() {
            let t0 = tt.inf();
            if n == 0 {
                return self.enclose(tt);
            }
            let idx = self.cell_index(t0);
            if let Some(c) = self.cells.get(idx) {
                let s = (t0 - c.a) / c.h;
                let v = match n {
                    1 => bezier_der(c.p0, c.p1, c.p2, c.p3, s, c.h),
                    2 => bezier_der2(c.p0, c.p1, c.p2, c.p3, s, c.h),
                    3 => c.der3_vec(),
                    _ => Vector3::zero(),
                };
                return hull_box_vec(&[v]);
            }
        }
        let mut acc = Box3::empty();
        for c in &self.cells {
            let cell = interval(c.a, c.b);
            if cell_overlaps(cell, tt) {
                let lo = tt.inf().max(c.a);
                let hi = tt.sup().min(c.b);
                let sub = c.restrict(lo, hi);
                let h = hi - lo;
                let b = match n {
                    0 => hull_box(&sub),
                    1 => hull_box_vec(&der_controls(sub, h)),
                    2 => hull_box_vec(&der2_controls(sub, h)),
                    3 => hull_box_vec(&[c.der3_vec()]),
                    _ => Box3 {
                        x: interval(0.0, 0.0),
                        y: interval(0.0, 0.0),
                        z: interval(0.0, 0.0),
                    },
                };
                acc = hull_join(&acc, &b);
            }
        }
        acc
    }

    fn tangent_cone(&self, _tt: Interval) -> Option<crate::enclosure::DirCone> {
        None
    }
}

/// What `rep` proved, and what it achieved. This IS the certificate — `rep`
/// never returns the curve without it.
#[derive(Debug, Clone, PartialEq)]
pub struct RepCertificate {
    /// Certified achieved two-sided sup-distance exact-vs-emitted.
    pub eps_achieved: f64,
    /// Certified min |cos| over all paired tangent boxes (the (ii) margin).
    pub angle_cos_lower: f64,
    /// Final uniform partition depth (2^depth cells).
    pub depth: u32,
    /// The knots, ascending, echo of the certified partition.
    pub partition: Vec<f64>,
    /// Refinement levels spent from the first attempt to the certificate.
    pub subdivisions_spent: u32,
    /// The scale components every gate was evaluated against (echo).
    pub scale: CurveScaleComponents,
}

/// `rep_curve`'s success: the curve AND the certificate, together.
#[derive(Debug, Clone)]
pub struct RepCurveOutput {
    /// The emitted piecewise cubic Hermite approximant.
    pub curve: HermiteCurve,
    /// The certificate of what was achieved and what was discharged.
    pub certificate: RepCertificate,
}

/// Approximate one exact curve component to `tau_rep`, certifying (i)-(iii)
/// and discharging (iv-b) on the same partition.
///
/// @feeds [CCS05, Thm 2.1:H1-H3]          # would discharge, conditional on lemmas
/// @via-open-lemma FID-L-TUBE | FID-L-FEDERER-PATCH | FID-L-COVERING | FID-L-SEPARATES
/// @establishes
///     (i)-(iii) of §6.2 between exact and emitted curve
///     + (iv-b) per-cell fibre-block degree-one on the emitted partition
/// @does-not-establish
///     isotopy | homeomorphism | side separation | whole-span one-sheet as a
///     topological claim | surface case (BG-FID-005-SRF) | reach semantics
pub fn rep_curve(
    exact: &impl EnclosureCurve,
    boundary: CurveBoundary,
    tau_rep: f64,
    arc_gap: f64,
    initial_depth: u32,
    budget: &mut Budget,
) -> Result<RepCurveOutput, RepError> {
    if tau_rep <= 0.0 || !tau_rep.is_finite() {
        return Err(RepError::InvalidMargin);
    }
    if arc_gap <= 0.0 || !arc_gap.is_finite() {
        return Err(RepError::InvalidMargin);
    }
    let Some((lo, hi)) = exact.try_range_tuple() else {
        return Err(RepError::InvalidMargin);
    };
    if !(lo.is_finite() && hi.is_finite()) || lo >= hi {
        return Err(RepError::InvalidMargin);
    }

    // Decision 1: scale components, computed once. Their epistemic refusals
    // (CurvatureUnresolved / SeparationUnresolved) propagate as ReachTooSmall
    // — the collapsing-geometry route (a corner refuses here; a
    // small-but-positive bound does NOT, see Decision 3).
    let curvature =
        curvature_radius_lower_span(exact, budget).map_err(|_| RepError::ReachTooSmall)?;
    let separation = self_separation_lower_span(exact, boundary, arc_gap, budget)
        .map_err(|_| RepError::ReachTooSmall)?;
    let scale = CurveScaleComponents {
        curvature_radius_lower: curvature,
        self_separation_lower: separation,
    };
    let tube = scale.tube_scale_lower();
    let target_eps = tau_rep.min(tube / 2.0);

    let mut depth = initial_depth;
    let mut subdivisions_spent = 0u32;
    let mut prev_eps = f64::INFINITY;
    let mut stalls = 0u32;

    loop {
        // Decision 3: Budget's own exhaustion at the top of each attempt.
        budget.spend_subdiv(1).map_err(|_| RepError::Unresolved {
            subdivisions: subdivisions_spent,
        })?;
        subdivisions_spent += 1;

        let cells = uniform_cells(lo, hi, depth);
        if cells.is_empty() {
            return Err(RepError::Unresolved {
                subdivisions: subdivisions_spent,
            });
        }
        let knots = knots_from_cells(&cells);
        let curve = HermiteCurve::build(exact, knots.clone());
        let (eps_now, theta_now, cell_eps) = measure(&curve, exact, &knots);

        if eps_now > target_eps {
            // eps stalled above target at the enclosure width floor: two
            // consecutive depths that barely improve it are Unresolved, never
            // a best-effort curve.
            if prev_eps.is_finite() && eps_now >= prev_eps - STALL_TOL * prev_eps {
                stalls += 1;
                if stalls >= 2 {
                    return Err(RepError::Unresolved {
                        subdivisions: subdivisions_spent,
                    });
                }
            } else {
                stalls = 0;
            }
            prev_eps = eps_now;
            depth += 1;
            continue;
        }
        if theta_now <= target_eps / tube {
            // (ii) gate at the achieved eps; a failing tangent margin refines.
            depth += 1;
            continue;
        }
        match ivb_check(&curve, exact, boundary, &knots, &cell_eps, budget) {
            IvbOutcome::Pass => {
                let certificate = RepCertificate {
                    eps_achieved: eps_now,
                    angle_cos_lower: theta_now,
                    depth,
                    partition: knots,
                    subdivisions_spent,
                    scale,
                };
                return Ok(RepCurveOutput { curve, certificate });
            }
            IvbOutcome::CellFailure => {
                depth += 1;
                continue;
            }
        }
    }
}

/// Build the ascending knot list from the uniform cell list.
fn knots_from_cells(cells: &[Interval]) -> Vec<f64> {
    let mut knots = Vec::with_capacity(cells.len() + 1);
    if let Some(first) = cells.first() {
        knots.push(first.inf());
    }
    for c in cells {
        knots.push(c.sup());
    }
    knots
}

/// The midpoint of the exact curve's degenerate tangent enclosure at `t`.
fn tangent_midpoint(exact: &impl EnclosureCurve, t: f64) -> Vector3 {
    let tb = exact.enclose_der(1, interval_at(t));
    Vector3::new(tb.x.mid(), tb.y.mid(), tb.z.mid())
}

/// Measure `eps_now` (max over identity-paired cells of the two-sided
/// box-to-box sup distance between the emitted hull and the exact cell box)
/// and `theta_now` (min over the same pairs of the (ii) pass form), entirely
/// by interval evaluation on the cell boxes — never by sampling. The
/// per-cell max sup is also returned, because the (iv-b)(c) separation gate
/// is a PER-CELL statement: a fast-sweeping part of the curve sets the global
/// max while a slow part has a far smaller certified deviation, and using the
/// global max there would over-refuse (measured on the ellipse).
///
/// Each partition cell is split into [`MEASURE_SUB`] sub-cells and the
/// quantities are measured per sub-cell. The single-cell box-to-box sup
/// distance is the box DIAGONAL (≈ √2·cell width on a circle), which exceeds
/// the certified gap to the nearest non-adjacent cell (≈ cell width) at every
/// uniform depth — the packet's own d=0..3 witnesses are the TRUE radial
/// error, not the diagonal. Evaluating on sub-cells gives a strictly tighter
/// (still sound) upper bound on the true sup distance, which is what keeps
/// the (iv-b)(c) separation gate satisfiable on the same partition.
fn measure(
    curve: &HermiteCurve,
    exact: &impl EnclosureCurve,
    knots: &[f64],
) -> (f64, f64, Vec<f64>) {
    let m = MEASURE_SUB;
    let mut eps_now = 0.0;
    let mut theta_now = f64::INFINITY;
    let mut cell_eps = Vec::with_capacity(knots.len().saturating_sub(1));
    for pair in knots.windows(2) {
        if let [a, b] = pair {
            let h = b - a;
            let mut cell_max = 0.0;
            for s in 0..m {
                let lo = a + h * (s as f64) / (m as f64);
                let hi = a + h * ((s + 1) as f64) / (m as f64);
                let sub = interval(lo, hi);
                let sup = sup_distance_box(&curve.enclose(sub), &exact.enclose(sub));
                if sup > cell_max {
                    cell_max = sup;
                }
                if sup > eps_now {
                    eps_now = sup;
                }
                let dh = curve.enclose_der(1, sub);
                let de = exact.enclose_der(1, sub);
                let ratio = angle_pass_form(&dh, &de);
                if ratio < theta_now {
                    theta_now = ratio;
                }
            }
            cell_eps.push(cell_max);
        }
    }
    (eps_now, theta_now, cell_eps)
}

/// The outcome of one per-cell (iv-b) discharge pass.
enum IvbOutcome {
    /// Every cell passed; the certificate is complete.
    Pass,
    /// A cell failed a per-cell (iv-b) assertion: refuse-and-refine. The
    /// loop's mapping spends one subdivision on the next attempt, and a
    /// genuinely exhausted budget surfaces there as `Unresolved`.
    CellFailure,
}

/// Discharge (iv-b) per cell on the SAME partition as the eps/theta
/// measurement. Item (a) is the eps measurement itself (own-cell containment)
/// plus item (c) (non-adjacent separation); item (b) is the knot-projection
/// correspondence at every interior knot. The separation gate is per-cell:
/// `cell_eps[j]` is the certified deviation of cell j, and non-adjacent cells
/// must be beyond THAT bound of cell j's block.
fn ivb_check(
    curve: &HermiteCurve,
    exact: &impl EnclosureCurve,
    boundary: CurveBoundary,
    knots: &[f64],
    cell_eps: &[f64],
    budget: &mut Budget,
) -> IvbOutcome {
    // (b) per-cell injectivity: the knot-projection correspondence. Every
    // interior knot t* has its projected parameter s(t*), the unique zero of
    // G(s) = <phi(t*) - X(s), X'(s)>, within the shared closure of its two
    // cells: the unique zero box must touch t*. Because phi(t*) = X(t*)
    // (Hermite interpolation), t* IS a root of G, so a Unique Krawczyk proof
    // over [t* - w, t* + w] certifies that the root stays in the knot's
    // neighbourhood; NoRoot certifies a fold (the projection jumped away) and
    // an indeterminate box is an epistemic refusal, both refuse-and-refine.
    for pair in knots.windows(3) {
        if let [prev, cur, next] = pair {
            let t_star = *cur;
            let w = (t_star - prev).max(next - t_star);
            let s_box = interval(t_star - w, t_star + w);
            match knot_projection_ok(exact, t_star, s_box, budget) {
                Ok(true) => {}
                Ok(false) | Err(()) => return IvbOutcome::CellFailure,
            }
        }
    }

    // (c) non-adjacent separation over the balanced BVH of exact cell boxes.
    let n = knots.len().saturating_sub(1);
    let cells: Vec<Interval> = knots
        .windows(2)
        .filter_map(|w| match w {
            [a, b] => Some(interval(*a, *b)),
            _ => None,
        })
        .collect();
    let curve_boxes: Vec<Box3> = cells.iter().map(|c| curve.enclose(*c)).collect();
    let exact_boxes: Vec<Box3> = cells.iter().map(|c| exact.enclose(*c)).collect();
    if separation_violation(
        knots,
        &cells,
        &curve_boxes,
        &exact_boxes,
        cell_eps,
        boundary,
        n,
    ) {
        return IvbOutcome::CellFailure;
    }
    IvbOutcome::Pass
}

/// The (b) knot-projection check: certify the unique zero of
/// `G(s) = <phi(t*) - X(s), X'(s)>` over `s_box` via the Krawczyk operator.
/// `Ok(true)` = unique zero in the box (touches t*, since t* is a root);
/// `Ok(false)` = no zero in the box (a certified fold); `Err(())` = the
/// operator could not decide (epistemic).
fn knot_projection_ok(
    exact: &impl EnclosureCurve,
    t_star: f64,
    s_box: Interval,
    budget: &mut Budget,
) -> Result<bool, ()> {
    let phi = exact.subs(t_star);
    let system = KnotProjection { exact, phi };
    match krawczyk(&system, &[s_box], budget) {
        Ok(cert) => Ok(cert.value == KrawczykProof::Unique),
        Err(_) => Err(()),
    }
}

/// The Krawczyk system for the knot-projection zero: `G(s) = <phi - X(s),
/// X'(s)>` with `phi = phi(t*)` held fixed. The Jacobian is
/// `G'(s) = -<X'(s),X'(s)> + <phi - X(s), X''(s)>`, the denominator of the
/// projected-parameter formula of Decision 4(b) — positive by the tube gate,
/// never evaluated as a formula, only interval-checked. The point centers use
/// the degenerate-point enclosures (outward-rounded point values), so the
/// system needs only [`EnclosureCurve`] methods and no associated `Vector`.
struct KnotProjection<'a, C: EnclosureCurve> {
    /// The exact curve.
    exact: &'a C,
    /// The emitted knot point `phi(t*)`.
    phi: Point3,
}

impl<'a, C: EnclosureCurve> KrawczykSystem<1> for KnotProjection<'a, C> {
    fn f_point(&self, s: &[f64; 1]) -> [Interval; 1] {
        let [s0] = *s;
        let e = self.exact.enclose(interval_at(s0));
        let e1 = self.exact.enclose_der(1, interval_at(s0));
        [dot_box(&box_minus_point(&e, self.phi), &e1)]
    }

    fn jacobian(&self, b: &[Interval; 1]) -> [[Interval; 1]; 1] {
        let [b0] = *b;
        let e = self.exact.enclose(b0);
        let e1 = self.exact.enclose_der(1, b0);
        let e2 = self.exact.enclose_der(2, b0);
        let gprime = -dot_box(&e1, &e1) + dot_box(&box_minus_point(&e, self.phi), &e2);
        [[gprime]]
    }

    fn preconditioner(&self, s: &[f64; 1]) -> Option<[[f64; 1]; 1]> {
        let [s0] = *s;
        let e = self.exact.enclose(interval_at(s0));
        let e1 = self.exact.enclose_der(1, interval_at(s0));
        let e2 = self.exact.enclose_der(2, interval_at(s0));
        let gprime = (-dot_box(&e1, &e1) + dot_box(&box_minus_point(&e, self.phi), &e2)).mid();
        if gprime.is_finite() && gprime != 0.0 {
            Some([[1.0 / gprime]])
        } else {
            None
        }
    }
}

/// (iv-b)(c): whether ANY non-adjacent pair has `box_distance(H_j, E_k)
/// <= cell_eps[j]` (the certified deviation of cell j). The BVH prunes nodes
/// whose union box is already beyond the query cell's bound (box-distance to
/// a union is a lower bound for every leaf inside it).
fn separation_violation(
    knots: &[f64],
    cells: &[Interval],
    curve_boxes: &[Box3],
    exact_boxes: &[Box3],
    cell_eps: &[f64],
    boundary: CurveBoundary,
    n: usize,
) -> bool {
    let kd: Vec<KdCell> = cells
        .iter()
        .zip(exact_boxes.iter())
        .map(|(tt, bb)| KdCell { tt: *tt, bb: *bb })
        .collect();
    let tree = build_tree(&kd);
    for (j, hbox) in curve_boxes.iter().enumerate() {
        let eps_j = cell_eps.get(j).copied().unwrap_or(0.0);
        if any_close_non_adjacent(&tree, hbox, eps_j, j, n, boundary, knots) {
            return true;
        }
    }
    false
}

/// The adjacency predicate of Decision 4(c): the identity pairing (j == k),
/// `|j-k| == 1`, plus wrap adjacency `(0, n-1)` when `Closed`.
fn adjacent(j: usize, k: usize, n: usize, boundary: CurveBoundary) -> bool {
    if j == k {
        return true;
    }
    let d = (j as i64 - k as i64).abs();
    if d == 1 {
        return true;
    }
    boundary == CurveBoundary::Closed
        && ((j == 0 && k == n.saturating_sub(1)) || (j == n.saturating_sub(1) && k == 0))
}

/// The index of a leaf cell by its parameter box, found against the ascending
/// knots (binary search; the leaf is exactly one cell of the partition).
fn cell_index(knots: &[f64], tt: &Interval) -> usize {
    let j = knots.partition_point(|k| *k <= tt.inf());
    let idx = j.saturating_sub(1);
    let n = knots.len();
    if idx + 1 < n {
        idx
    } else {
        n.saturating_sub(2)
    }
}

/// Whether any leaf of the tree with a box within `eps` of the query box is
/// non-adjacent to cell `j`.
fn any_close_non_adjacent(
    node: &KdNode,
    query: &Box3,
    eps: f64,
    j: usize,
    n: usize,
    boundary: CurveBoundary,
    knots: &[f64],
) -> bool {
    if box_distance(query, &node.bb) > eps {
        return false;
    }
    if let Some(cell) = node.cell {
        let k = cell_index(knots, &cell.tt);
        return !adjacent(j, k, n, boundary) && box_distance(query, &cell.bb) <= eps;
    }
    if let Some(l) = &node.left {
        if any_close_non_adjacent(l, query, eps, j, n, boundary, knots) {
            return true;
        }
    }
    if let Some(r) = &node.right {
        if any_close_non_adjacent(r, query, eps, j, n, boundary, knots) {
            return true;
        }
    }
    false
}

/// The interval dot product of two boxes (duplicated locally exactly as the
/// sibling fid modules do; `enclosure.rs` visibility stays untouched).
fn dot_box(a: &Box3, b: &Box3) -> Interval {
    a.x * b.x + a.y * b.y + a.z * b.z
}

/// Shift a box by minus a point: `{ p - q : p in box }` for fixed `q`.
fn box_minus_point(a: &Box3, p: Point3) -> Box3 {
    Box3 {
        x: a.x - interval_at(p.x),
        y: a.y - interval_at(p.y),
        z: a.z - interval_at(p.z),
    }
}

/// The house outward pad per hull endpoint: `64 EPSILON (1 + |coord|)`, the
/// same relative pad the BG-ENC-003 bspline carrier uses.
const HULL_PAD: f64 = 64.0 * f64::EPSILON; // H-3: relative outward hull pad, dimensionless ulp multiple

/// The relative eps-stall threshold of the refine loop: a depth whose eps
/// improves by less than this over the previous is a stall, and two
/// consecutive stalls above target are Unresolved.
const STALL_TOL: f64 = 0.01; // H-3: dimensionless relative certificate-change threshold

/// The per-cell subdivision count of the eps/theta measurement: each
/// partition cell is split into this many equal sub-cells, and the box
/// quantities are evaluated per sub-cell. A power of two keeps every
/// sub-cell boundary off the partition knots' dyadic structure concerns.
const MEASURE_SUB: u32 = 4; // H-3: dimensionless sub-cell subdivision count

/// One hull-coordinate interval `[lo, hi]` padded `HULL_PAD (1 + |·|)`
/// outward per endpoint.
fn pad_iv(lo: f64, hi: f64) -> Interval {
    let pad = HULL_PAD * (1.0 + lo.abs().max(hi.abs()));
    interval(lo - pad, hi + pad)
}

/// The padded axis-aligned hull of a set of points.
fn hull_box(pts: &[Point3]) -> Box3 {
    let mut lo = Vector3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
    let mut hi = Vector3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for p in pts {
        lo.x = lo.x.min(p.x);
        lo.y = lo.y.min(p.y);
        lo.z = lo.z.min(p.z);
        hi.x = hi.x.max(p.x);
        hi.y = hi.y.max(p.y);
        hi.z = hi.z.max(p.z);
    }
    Box3 {
        x: pad_iv(lo.x, hi.x),
        y: pad_iv(lo.y, hi.y),
        z: pad_iv(lo.z, hi.z),
    }
}

/// The padded axis-aligned hull of a set of vectors.
fn hull_box_vec(vs: &[Vector3]) -> Box3 {
    let mut lo = Vector3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
    let mut hi = Vector3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for v in vs {
        lo.x = lo.x.min(v.x);
        lo.y = lo.y.min(v.y);
        lo.z = lo.z.min(v.z);
        hi.x = hi.x.max(v.x);
        hi.y = hi.y.max(v.y);
        hi.z = hi.z.max(v.z);
    }
    Box3 {
        x: pad_iv(lo.x, hi.x),
        y: pad_iv(lo.y, hi.y),
        z: pad_iv(lo.z, hi.z),
    }
}

/// Join two boxes by per-axis convex hull.
fn hull_join(a: &Box3, b: &Box3) -> Box3 {
    Box3 {
        x: a.x.convex_hull(b.x),
        y: a.y.convex_hull(b.y),
        z: a.z.convex_hull(b.z),
    }
}

/// Linear interpolation between two points.
fn lerp(a: Point3, b: Point3, t: f64) -> Point3 {
    a + (b - a) * t
}

/// De Casteljau evaluation of the cubic Bezier at `s in [0, 1]`.
fn bezier(p0: Point3, p1: Point3, p2: Point3, p3: Point3, s: f64) -> Point3 {
    let p01 = lerp(p0, p1, s);
    let p12 = lerp(p1, p2, s);
    let p23 = lerp(p2, p3, s);
    let p012 = lerp(p01, p12, s);
    let p123 = lerp(p12, p23, s);
    lerp(p012, p123, s)
}

/// The Bezier's first derivative (divided by `h` for the parameter `t`).
fn bezier_der(p0: Point3, p1: Point3, p2: Point3, p3: Point3, s: f64, h: f64) -> Vector3 {
    let u = 1.0 - s;
    let d0 = (p1 - p0) * (3.0 * u * u);
    let d1 = (p2 - p1) * (6.0 * u * s);
    let d2 = (p3 - p2) * (3.0 * s * s);
    (d0 + d1 + d2) * (1.0 / h)
}

/// The Bezier's second derivative (divided by `h^2` for the parameter `t`).
fn bezier_der2(p0: Point3, p1: Point3, p2: Point3, p3: Point3, s: f64, h: f64) -> Vector3 {
    let u = 1.0 - s;
    let c0 = (p2 - p1) - (p1 - p0);
    let c1 = (p3 - p2) - (p2 - p1);
    (c0 * (6.0 * u) + c1 * (6.0 * s)) * (1.0 / (h * h))
}

#[cfg(test)]
mod tests {
    // GATE-1: the fid module (including its test module) stays under the
    // crate's unwrap denial; unit tests assert on hand-built witnesses, and
    // `must` below is the deny-clean spelling of an unwrap.
    #![deny(clippy::unwrap_used)]

    use super::*;
    use crate::elementary::{cos, sin};
    use crate::enclosure::DirCone;
    use crate::fid::isotopy::{curve_isotopy_conditions, IsotopyConditionsError};
    use std::ops::Bound;
    use truck_base::cgmath64::{EuclideanSpace, Point3, Vector3, Zero};
    use truck_base::evidence::{Budget, EnvelopeCase, Refusal};
    use truck_geotrait::{ParameterRange, ParametricCurve};

    /// Exact circle radius, model units.
    const RADIUS: f64 = 2.0; // H-3: exact circle radius in model units, the witness length scale
    /// The rep tolerance (model-space length).
    const TAU_REP: f64 = 0.05; // H-3: rep tolerance, a model-space length relative to RADIUS
    /// The self-separation parameter gap of the house witnesses.
    const ARC_GAP: f64 = core::f64::consts::PI; // H-3: parameter gap in radians, dimensionless
    /// The full-circle parameter span `[0, 2π]`.
    const FULL_SPAN: f64 = core::f64::consts::TAU; // H-3: the full circle span in radians, dimensionless
    /// The coarse-radius circle's radius, below 2*tau: the over-refusal guard.
    const COARSE_RADIUS: f64 = 0.08; // H-3: coarse radius in model units, below the 2*tau tube budget
    /// The coarse circle's target: `min(tau, R/2)`.
    const COARSE_TARGET: f64 = 0.04; // H-3: coarse rep target eps, a model-space length
    /// The ellipse's semi-major axis.
    const ELLIPSE_A: f64 = 2.0; // H-3: ellipse semi-major axis in model units
    /// The ellipse's semi-minor axis.
    const ELLIPSE_B: f64 = 0.5; // H-3: ellipse semi-minor axis in model units
    /// The radial sinusoid's amplitude `a <= eps`.
    const SINUSOID_A: f64 = 0.04; // H-3: sinusoid amplitude, a model-space length strictly below tau
    /// The radial sinusoid's frequency in radians.
    const SINUSOID_OMEGA: f64 = 8.0; // H-3: sinusoid angular frequency, a dimensionless oscillation rate
    /// The slack added to the achieved eps for the independent (iv-a) cross-check.
    const CROSS_SLACK: f64 = 0.001; // H-3: cross-check slack, a model-space length
    /// The slack added to the achieved eps for the family cross-check.
    const FAMILY_SLACK: f64 = 0.001; // H-3: family cross-check slack, a model-space length
    /// Subdivision budget for a full rep run.
    const REP_BUDGET: u32 = 1 << 20; // H-3: subdivision budget count, dimensionless
    /// Subdivision budget for measuring the scale components' spend (test 5).
    const SCALE_MEASURE_BUDGET: u32 = 1 << 18; // H-3: subdivision budget count, dimensionless
    /// Subdivision budget for the V-corner collapse route.
    const V_BUDGET: u32 = 1 << 16; // H-3: subdivision budget count, dimensionless
    /// The V-corner fixture's parameter span.
    const V_LO: f64 = 0.0; // H-3: V-corner start parameter, dimensionless
    const V_HI: f64 = 2.0; // H-3: V-corner end parameter, dimensionless
    /// The V-corner's corner parameter. Deliberately NOT a dyadic fraction of
    /// the span: a corner on a uniform bisection boundary (1.0 was) makes the
    /// scale helpers see straight cells at every depth and return `+inf`
    /// instead of refusing. At 1.3 a straddling cell exists at every depth and
    /// the collapsing-geometry refusal actually fires.
    const V_CORNER_T: f64 = 1.3; // H-3: V-corner corner parameter, dimensionless
    /// The second V-leg's travel direction, chosen so the corner cell's
    /// tangent box hull contains the origin (the two branch directions at the
    /// corner straddle it) — the collapsing-geometry witness.
    const V_DIR2_X: f64 = -0.5; // H-3: second-leg direction x, dimensionless
    const V_DIR2_Y: f64 = -0.8660254037844386; // H-3: second-leg direction y (=-sqrt(3)/2), dimensionless
    /// The hand-widened seam-test box half-extent (a box so wide that every
    /// non-adjacent pair comes within eps).
    const SEAM_HALF: f64 = 3.0; // H-3: seam box half-extent in model units

    /// Test-only unwrap that stays under the crate's deny list: unit tests
    /// assert on hand-built witnesses, so a refusal here is a test bug.
    fn must_rep<T>(r: Result<T, RepError>) -> T {
        match r {
            Ok(value) => value,
            Err(_) => unreachable!("unit-test witness must certify"),
        }
    }

    /// Test-only unwrap for the landed isotopy checker.
    fn must_iso<T>(r: Result<T, IsotopyConditionsError>) -> T {
        match r {
            Ok(value) => value,
            Err(_) => unreachable!("unit-test witness must certify"),
        }
    }

    /// A circle `r * e(t)` over `[lo, hi]`.
    #[derive(Clone)]
    struct Circle {
        r: f64,
        lo: f64,
        hi: f64,
    }

    impl ParametricCurve for Circle {
        type Point = Point3;
        type Vector = Vector3;

        fn subs(&self, t: f64) -> Point3 {
            Point3::new(self.r * t.cos(), self.r * t.sin(), 0.0)
        }

        fn der(&self, t: f64) -> Vector3 {
            Vector3::new(-self.r * t.sin(), self.r * t.cos(), 0.0)
        }

        fn der2(&self, t: f64) -> Vector3 {
            Vector3::new(-self.r * t.cos(), -self.r * t.sin(), 0.0)
        }

        fn der_n(&self, n: usize, t: f64) -> Vector3 {
            match n % 4 {
                0 => self.subs(t).to_vec(),
                1 => self.der(t),
                2 => self.der2(t),
                _ => Vector3::new(self.r * t.sin(), -self.r * t.cos(), 0.0),
            }
        }

        fn parameter_range(&self) -> ParameterRange {
            (Bound::Included(self.lo), Bound::Included(self.hi))
        }
    }

    impl EnclosureCurve for Circle {
        fn enclose(&self, tt: Interval) -> Box3 {
            Box3 {
                x: cos(tt) * interval_at(self.r),
                y: sin(tt) * interval_at(self.r),
                z: interval_at(0.0),
            }
        }

        fn enclose_der(&self, n: usize, tt: Interval) -> Box3 {
            match n % 4 {
                0 => self.enclose(tt),
                1 => Box3 {
                    x: -sin(tt) * interval_at(self.r),
                    y: cos(tt) * interval_at(self.r),
                    z: interval_at(0.0),
                },
                2 => Box3 {
                    x: -cos(tt) * interval_at(self.r),
                    y: -sin(tt) * interval_at(self.r),
                    z: interval_at(0.0),
                },
                _ => Box3 {
                    x: sin(tt) * interval_at(self.r),
                    y: -cos(tt) * interval_at(self.r),
                    z: interval_at(0.0),
                },
            }
        }

        fn tangent_cone(&self, _tt: Interval) -> Option<DirCone> {
            None
        }
    }

    /// The ellipse `(a cos t, b sin t, 0)` over `[lo, hi]`.
    #[derive(Clone)]
    struct Ellipse {
        a: f64,
        b: f64,
        lo: f64,
        hi: f64,
    }

    impl ParametricCurve for Ellipse {
        type Point = Point3;
        type Vector = Vector3;

        fn subs(&self, t: f64) -> Point3 {
            Point3::new(self.a * t.cos(), self.b * t.sin(), 0.0)
        }

        fn der(&self, t: f64) -> Vector3 {
            Vector3::new(-self.a * t.sin(), self.b * t.cos(), 0.0)
        }

        fn der2(&self, t: f64) -> Vector3 {
            Vector3::new(-self.a * t.cos(), -self.b * t.sin(), 0.0)
        }

        fn der_n(&self, n: usize, t: f64) -> Vector3 {
            let angle = t + (n as f64) * core::f64::consts::FRAC_PI_2;
            Vector3::new(self.a * angle.cos(), self.b * angle.sin(), 0.0)
        }

        fn parameter_range(&self) -> ParameterRange {
            (Bound::Included(self.lo), Bound::Included(self.hi))
        }
    }

    impl EnclosureCurve for Ellipse {
        fn enclose(&self, tt: Interval) -> Box3 {
            Box3 {
                x: interval_at(self.a) * cos(tt),
                y: interval_at(self.b) * sin(tt),
                z: interval_at(0.0),
            }
        }

        fn enclose_der(&self, n: usize, tt: Interval) -> Box3 {
            if n == 0 {
                return self.enclose(tt);
            }
            let shift = (n as f64) * core::f64::consts::FRAC_PI_2;
            Box3 {
                x: interval_at(self.a) * cos(tt + interval_at(shift)),
                y: interval_at(self.b) * sin(tt + interval_at(shift)),
                z: interval_at(0.0),
            }
        }

        fn tangent_cone(&self, _tt: Interval) -> Option<DirCone> {
            None
        }
    }

    /// The radial sinusoid `(R + a sin(omega t)) * e(t)` over `[lo, hi]`.
    #[derive(Clone)]
    struct RadialSinusoid {
        r: f64,
        a: f64,
        omega: f64,
        lo: f64,
        hi: f64,
    }

    impl ParametricCurve for RadialSinusoid {
        type Point = Point3;
        type Vector = Vector3;

        fn subs(&self, t: f64) -> Point3 {
            let rad = self.r + self.a * (self.omega * t).sin();
            Point3::new(rad * t.cos(), rad * t.sin(), 0.0)
        }

        fn der(&self, t: f64) -> Vector3 {
            let rad = self.r + self.a * (self.omega * t).sin();
            let drad = self.a * self.omega * (self.omega * t).cos();
            Vector3::new(
                drad * t.cos() - rad * t.sin(),
                drad * t.sin() + rad * t.cos(),
                0.0,
            )
        }

        fn der2(&self, t: f64) -> Vector3 {
            let rad = self.r + self.a * (self.omega * t).sin();
            let drad = self.a * self.omega * (self.omega * t).cos();
            let d2rad = -self.a * self.omega * self.omega * (self.omega * t).sin();
            Vector3::new(
                (d2rad - rad) * t.cos() - 2.0 * drad * t.sin(),
                (d2rad - rad) * t.sin() + 2.0 * drad * t.cos(),
                0.0,
            )
        }

        fn der_n(&self, n: usize, t: f64) -> Vector3 {
            if n == 0 {
                return self.subs(t).to_vec();
            }
            let mut acc = Vector3::new(0.0, 0.0, 0.0);
            let mut binom = 1.0_f64;
            for k in 0..=n {
                let rad_k = if k == 0 {
                    self.r + self.a * (self.omega * t).sin()
                } else {
                    self.a
                        * self.omega.powi(k as i32)
                        * (self.omega * t + (k as f64) * core::f64::consts::FRAC_PI_2).sin()
                };
                let angle = t + (n - k) as f64 * core::f64::consts::FRAC_PI_2;
                acc += Vector3::new(angle.cos(), angle.sin(), 0.0) * (binom * rad_k);
                binom *= (n - k) as f64 / (k + 1) as f64;
            }
            acc
        }

        fn parameter_range(&self) -> ParameterRange {
            (Bound::Included(self.lo), Bound::Included(self.hi))
        }
    }

    impl EnclosureCurve for RadialSinusoid {
        fn enclose(&self, tt: Interval) -> Box3 {
            let w = interval_at(self.omega);
            let rad = interval_at(self.r) + interval_at(self.a) * sin(w * tt);
            Box3 {
                x: rad * cos(tt),
                y: rad * sin(tt),
                z: interval_at(0.0),
            }
        }

        fn enclose_der(&self, n: usize, tt: Interval) -> Box3 {
            if n == 0 {
                return self.enclose(tt);
            }
            let w = interval_at(self.omega);
            let wtt = w * tt;
            let mut x = interval_at(0.0);
            let mut y = interval_at(0.0);
            let mut binom = 1.0_f64;
            for k in 0..=n {
                let rad_k = if k == 0 {
                    interval_at(self.r) + interval_at(self.a) * sin(wtt)
                } else {
                    interval_at(self.a)
                        * interval_at(self.omega.powi(k as i32))
                        * sin(wtt + interval_at((k as f64) * core::f64::consts::FRAC_PI_2))
                };
                let shift = (n - k) as f64 * core::f64::consts::FRAC_PI_2;
                let ex = cos(tt + interval_at(shift));
                let ey = sin(tt + interval_at(shift));
                let c = interval_at(binom);
                x += ex * rad_k * c;
                y += ey * rad_k * c;
                binom *= (n - k) as f64 / (k + 1) as f64;
            }
            Box3 {
                x,
                y,
                z: interval_at(0.0),
            }
        }

        fn tangent_cone(&self, _tt: Interval) -> Option<DirCone> {
            None
        }
    }

    /// A circle traversed backwards: `rev(t) = base(lo + hi - t)`.
    #[derive(Clone)]
    struct RevCircle {
        r: f64,
        lo: f64,
        hi: f64,
    }

    impl ParametricCurve for RevCircle {
        type Point = Point3;
        type Vector = Vector3;

        fn subs(&self, t: f64) -> Point3 {
            let base = self.lo + self.hi - t;
            Point3::new(self.r * base.cos(), self.r * base.sin(), 0.0)
        }

        fn der(&self, t: f64) -> Vector3 {
            let base = self.lo + self.hi - t;
            Vector3::new(-self.r * (-base.sin()), -self.r * base.cos(), 0.0)
        }

        fn der2(&self, t: f64) -> Vector3 {
            let base = self.lo + self.hi - t;
            Vector3::new(-self.r * base.cos(), self.r * base.sin(), 0.0)
        }

        fn der_n(&self, n: usize, t: f64) -> Vector3 {
            let base = self.lo + self.hi - t;
            let base_n = (n as f64) * core::f64::consts::FRAC_PI_2;
            let sign = if n % 2 == 1 { -1.0 } else { 1.0 };
            Vector3::new(
                sign * self.r * (base + base_n).cos(),
                sign * self.r * (base + base_n).sin(),
                0.0,
            )
        }

        fn parameter_range(&self) -> ParameterRange {
            (Bound::Included(self.lo), Bound::Included(self.hi))
        }
    }

    impl EnclosureCurve for RevCircle {
        fn enclose(&self, tt: Interval) -> Box3 {
            let base = interval_at(self.lo + self.hi) - tt;
            Box3 {
                x: cos(base) * interval_at(self.r),
                y: sin(base) * interval_at(self.r),
                z: interval_at(0.0),
            }
        }

        fn enclose_der(&self, n: usize, tt: Interval) -> Box3 {
            if n == 0 {
                return self.enclose(tt);
            }
            let base = interval_at(self.lo + self.hi) - tt;
            let shift = (n as f64) * core::f64::consts::FRAC_PI_2;
            let sign = if n % 2 == 1 { -1.0 } else { 1.0 };
            Box3 {
                x: interval_at(sign * self.r) * cos(base + interval_at(shift)),
                y: interval_at(sign * self.r) * sin(base + interval_at(shift)),
                z: interval_at(0.0),
            }
        }

        fn tangent_cone(&self, _tt: Interval) -> Option<DirCone> {
            None
        }
    }

    /// The V-corner: two line segments meeting at 60 degrees, traversed so the
    /// corner cell's tangent enclosure contains BOTH branch directions at
    /// every refinement (and its box hull straddles the origin), so the scale
    /// components cannot be certified at all and rep routes to
    /// `RepError::ReachTooSmall`.
    #[derive(Clone)]
    struct VCorn {
        lo: f64,
        hi: f64,
        corner: f64,
    }

    impl ParametricCurve for VCorn {
        type Point = Point3;
        type Vector = Vector3;

        fn subs(&self, t: f64) -> Point3 {
            if t <= self.corner {
                Point3::new(t - self.corner, 0.0, 0.0)
            } else {
                let d = t - self.corner;
                Point3::new(d * V_DIR2_X, d * V_DIR2_Y, 0.0)
            }
        }

        fn der(&self, t: f64) -> Vector3 {
            if t <= self.corner {
                Vector3::new(1.0, 0.0, 0.0)
            } else {
                Vector3::new(V_DIR2_X, V_DIR2_Y, 0.0)
            }
        }

        fn der2(&self, _t: f64) -> Vector3 {
            Vector3::zero()
        }

        fn der_n(&self, n: usize, t: f64) -> Vector3 {
            match n {
                0 => self.subs(t).to_vec(),
                1 => self.der(t),
                _ => Vector3::zero(),
            }
        }

        fn parameter_range(&self) -> ParameterRange {
            (Bound::Included(self.lo), Bound::Included(self.hi))
        }
    }

    impl EnclosureCurve for VCorn {
        fn enclose(&self, tt: Interval) -> Box3 {
            let a = tt.inf();
            let b = tt.sup();
            let mut acc = Box3::empty();
            if a < self.corner {
                let lo_t = a;
                let hi_t = b.min(self.corner);
                acc = hull_join(
                    &acc,
                    &Box3 {
                        x: interval(lo_t - self.corner, hi_t - self.corner),
                        y: interval(0.0, 0.0),
                        z: interval(0.0, 0.0),
                    },
                );
            }
            if b > self.corner {
                let lo_t = a.max(self.corner);
                let x1 = (b - self.corner) * V_DIR2_X;
                let x2 = (lo_t - self.corner) * V_DIR2_X;
                let y1 = (b - self.corner) * V_DIR2_Y;
                let y2 = (lo_t - self.corner) * V_DIR2_Y;
                let bx = if x2 < x1 {
                    interval(x2, x1)
                } else {
                    interval(x1, x2)
                };
                let by = if y2 < y1 {
                    interval(y2, y1)
                } else {
                    interval(y1, y2)
                };
                acc = hull_join(
                    &acc,
                    &Box3 {
                        x: bx,
                        y: by,
                        z: interval(0.0, 0.0),
                    },
                );
            }
            acc
        }

        fn enclose_der(&self, n: usize, tt: Interval) -> Box3 {
            if n == 0 {
                return self.enclose(tt);
            }
            let a = tt.inf();
            let b = tt.sup();
            let mut acc = Box3::empty();
            if a < self.corner {
                acc = hull_join(
                    &acc,
                    &Box3 {
                        x: interval(1.0, 1.0),
                        y: interval(0.0, 0.0),
                        z: interval(0.0, 0.0),
                    },
                );
            }
            if b > self.corner {
                acc = hull_join(
                    &acc,
                    &Box3 {
                        x: interval(V_DIR2_X, V_DIR2_X),
                        y: interval(V_DIR2_Y, V_DIR2_Y),
                        z: interval(0.0, 0.0),
                    },
                );
            }
            // A degenerate interval exactly at the corner has BOTH branch
            // directions in its tangent enclosure (the tangent is undefined
            // there); the sound enclosure is the hull of both, never empty.
            if acc.x.is_empty() && a == self.corner && b == self.corner {
                acc = hull_join(
                    &acc,
                    &Box3 {
                        x: interval(V_DIR2_X, 1.0),
                        y: interval(V_DIR2_Y, 0.0),
                        z: interval(0.0, 0.0),
                    },
                );
            }
            acc
        }

        fn tangent_cone(&self, _tt: Interval) -> Option<DirCone> {
            None
        }
    }

    /// The exact circle for every witness: radius RADIUS over `[0, 2π]`.
    fn exact_circle() -> Circle {
        Circle {
            r: RADIUS,
            lo: 0.0,
            hi: FULL_SPAN,
        }
    }

    #[test]
    fn rep_circle_from_coarse_certifies() {
        let exact = exact_circle();
        let mut budget = Budget::new(REP_BUDGET, 0, 0);
        let out = rep_curve(
            &exact,
            CurveBoundary::Closed,
            TAU_REP,
            ARC_GAP,
            0,
            &mut budget,
        );
        let output = must_rep(out);
        // d=0 error 0.336512 and d=1 error 0.429204 (dense-sampling witness)
        // both exceed target 0.05; the emission is only certified deeper.
        assert!(
            output.certificate.subdivisions_spent >= 2,
            "refined past the coarse depths, spent {}",
            output.certificate.subdivisions_spent
        );
        assert!(output.certificate.eps_achieved <= TAU_REP);
        assert!(output.certificate.partition.len() >= 4);
        // Independent cross-check: (iv-a) through the landed checker AGREES
        // with (iv-b) on the emitted partition.
        let eps_check = output.certificate.eps_achieved + CROSS_SLACK;
        let mut cb = Budget::new(REP_BUDGET, 0, 0);
        let report = must_iso(curve_isotopy_conditions(
            &exact,
            CurveBoundary::Closed,
            &output.curve,
            CurveBoundary::Closed,
            eps_check,
            &output.certificate.scale,
            &mut cb,
        ));
        assert_eq!(report.eps, eps_check);
    }

    #[test]
    fn rep_does_not_emit_at_coarse_depth() {
        let exact = exact_circle();
        let mut budget = Budget::new(REP_BUDGET, 0, 0);
        let out = rep_curve(
            &exact,
            CurveBoundary::Closed,
            TAU_REP,
            ARC_GAP,
            1,
            &mut budget,
        );
        let output = must_rep(out);
        assert!(
            output.certificate.partition.len() > 2,
            "a depth-1 start must still refine past its 3-knot partition"
        );
    }

    #[test]
    fn coarse_circle_refines_and_emits() {
        let exact = Circle {
            r: COARSE_RADIUS,
            lo: 0.0,
            hi: FULL_SPAN,
        };
        let mut budget = Budget::new(REP_BUDGET, 0, 0);
        let out = rep_curve(
            &exact,
            CurveBoundary::Closed,
            TAU_REP,
            ARC_GAP,
            0,
            &mut budget,
        );
        let output = must_rep(out);
        // target = min(tau, tube_scale_lower/2) = min(0.05, 0.08/2): the
        // over-refusal guard — small-but-positive tube_scale EMITS.
        assert!(
            output.certificate.eps_achieved <= COARSE_TARGET,
            "coarse circle must emit at target 0.04, achieved {}",
            output.certificate.eps_achieved
        );
    }

    #[test]
    fn v_corner_routes_to_collapse() {
        let corner = VCorn {
            lo: V_LO,
            hi: V_HI,
            corner: V_CORNER_T,
        };
        let mut budget = Budget::new(V_BUDGET, 0, 0);
        let out = rep_curve(
            &corner,
            CurveBoundary::Open,
            TAU_REP,
            ARC_GAP,
            0,
            &mut budget,
        );
        let e = match out {
            Err(e) => e,
            Ok(_) => unreachable!("a V-corner must route to collapse"),
        };
        assert!(matches!(e, RepError::ReachTooSmall));
        assert!(matches!(
            e.into_refusal(),
            Refusal::UnsupportedEnvelope(EnvelopeCase::ReachTooSmall)
        ));
    }

    #[test]
    fn budget_exhaustion_refuses() {
        let exact = exact_circle();
        // Measure the scale components' deterministic spend, then hand the rep
        // exactly that plus ~2 subdivisions: the refine loop exhausts and
        // refuses Unresolved carrying the spend — never a best-effort curve.
        let mut cb = Budget::new(SCALE_MEASURE_BUDGET, 0, 0);
        let _ = curvature_radius_lower_span(&exact, &mut cb);
        let curv_spent = SCALE_MEASURE_BUDGET - cb.subdiv;
        let mut sb = Budget::new(SCALE_MEASURE_BUDGET, 0, 0);
        let _ = self_separation_lower_span(&exact, CurveBoundary::Closed, ARC_GAP, &mut sb);
        let sep_spent = SCALE_MEASURE_BUDGET - sb.subdiv;
        let mut budget = Budget::new(curv_spent + sep_spent + 2, 0, 0);
        let out = rep_curve(
            &exact,
            CurveBoundary::Closed,
            TAU_REP,
            ARC_GAP,
            0,
            &mut budget,
        );
        match out {
            Err(RepError::Unresolved { subdivisions }) => {
                assert!(
                    subdivisions >= 2,
                    "the spend must be carried, got {subdivisions}"
                )
            }
            Ok(_) => unreachable!("an exhausted budget must refuse, never emit"),
            Err(_) => unreachable!("budget exhaustion must be Unresolved"),
        }
    }

    #[test]
    fn rep_idempotent_at_same_tolerance() {
        let exact = exact_circle();
        let mut b1 = Budget::new(REP_BUDGET, 0, 0);
        let e1 = must_rep(rep_curve(
            &exact,
            CurveBoundary::Closed,
            TAU_REP,
            ARC_GAP,
            0,
            &mut b1,
        ));
        let mut b2 = Budget::new(REP_BUDGET, 0, 0);
        let e2 = must_rep(rep_curve(
            &exact,
            CurveBoundary::Closed,
            TAU_REP,
            ARC_GAP,
            0,
            &mut b2,
        ));
        let mut cb = Budget::new(REP_BUDGET, 0, 0);
        let _ = must_iso(curve_isotopy_conditions(
            &e1.curve,
            CurveBoundary::Closed,
            &e2.curve,
            CurveBoundary::Closed,
            TAU_REP,
            &e1.certificate.scale,
            &mut cb,
        ));
    }

    #[test]
    fn reversed_exact_emits_reversed() {
        let fwd = exact_circle();
        let rev = RevCircle {
            r: RADIUS,
            lo: 0.0,
            hi: FULL_SPAN,
        };
        let mut b1 = Budget::new(REP_BUDGET, 0, 0);
        let ef = must_rep(rep_curve(
            &fwd,
            CurveBoundary::Closed,
            TAU_REP,
            ARC_GAP,
            0,
            &mut b1,
        ));
        let mut b2 = Budget::new(REP_BUDGET, 0, 0);
        let er = must_rep(rep_curve(
            &rev,
            CurveBoundary::Closed,
            TAU_REP,
            ARC_GAP,
            0,
            &mut b2,
        ));
        let mut cb = Budget::new(REP_BUDGET, 0, 0);
        let _ = must_iso(curve_isotopy_conditions(
            &ef.curve,
            CurveBoundary::Closed,
            &er.curve,
            CurveBoundary::Closed,
            TAU_REP,
            &ef.certificate.scale,
            &mut cb,
        ));
    }

    #[test]
    fn ivb_separation_failure_refines() {
        // The seam test: build a depth-2 circle's cells, hand-widen ONE exact
        // cell box so a non-adjacent pair comes within eps, and call the
        // per-cell (iv-b) separation check directly: it reports the failure.
        // The loop's mapping of that failure is depth += 1 (refine), whose
        // next attempt spends one subdivision from the budget.
        let exact = exact_circle();
        let cells = uniform_cells(0.0, FULL_SPAN, 2);
        let knots = knots_from_cells(&cells);
        let curve = HermiteCurve::build(&exact, knots.clone());
        let (_, _, cell_eps) = measure(&curve, &exact, &knots);
        let n = knots.len().saturating_sub(1);
        let cs: Vec<Interval> = knots
            .windows(2)
            .filter_map(|w| match w {
                [a, b] => Some(interval(*a, *b)),
                _ => None,
            })
            .collect();
        let curve_boxes: Vec<Box3> = cs.iter().map(|c| curve.enclose(*c)).collect();
        let mut exact_boxes: Vec<Box3> = cs.iter().map(|c| exact.enclose(*c)).collect();
        if let Some(b) = exact_boxes.get_mut(0) {
            *b = Box3 {
                x: interval(-SEAM_HALF, SEAM_HALF),
                y: interval(-SEAM_HALF, SEAM_HALF),
                z: interval(-SEAM_HALF, SEAM_HALF),
            };
        }
        assert!(
            separation_violation(
                &knots,
                &cs,
                &curve_boxes,
                &exact_boxes,
                &cell_eps,
                CurveBoundary::Closed,
                n
            ),
            "the widened cell must drive a non-adjacent pair within eps"
        );
    }

    #[test]
    fn invalid_inputs_refuse() {
        let exact = exact_circle();
        for bad_tau in [0.0, -TAU_REP, f64::NAN, f64::INFINITY] {
            let mut budget = Budget::new(REP_BUDGET, 0, 0);
            let out = rep_curve(
                &exact,
                CurveBoundary::Closed,
                bad_tau,
                ARC_GAP,
                0,
                &mut budget,
            );
            assert!(
                matches!(out, Err(RepError::InvalidMargin)),
                "tau = {bad_tau} must refuse as InvalidMargin"
            );
            assert_eq!(
                budget.subdiv, REP_BUDGET,
                "no budget spend on invalid input"
            );
        }
        for bad_gap in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            let mut budget = Budget::new(REP_BUDGET, 0, 0);
            let out = rep_curve(
                &exact,
                CurveBoundary::Closed,
                TAU_REP,
                bad_gap,
                0,
                &mut budget,
            );
            assert!(
                matches!(out, Err(RepError::InvalidMargin)),
                "arc_gap = {bad_gap} must refuse as InvalidMargin"
            );
            assert_eq!(
                budget.subdiv, REP_BUDGET,
                "no budget spend on invalid input"
            );
        }
    }

    #[test]
    fn rep_family_conditions_hold() {
        let circle = exact_circle();
        let ellipse = Ellipse {
            a: ELLIPSE_A,
            b: ELLIPSE_B,
            lo: 0.0,
            hi: FULL_SPAN,
        };
        let sinusoid = RadialSinusoid {
            r: RADIUS,
            a: SINUSOID_A,
            omega: SINUSOID_OMEGA,
            lo: 0.0,
            hi: FULL_SPAN,
        };
        family_check(&circle);
        family_check(&ellipse);
        family_check(&sinusoid);
    }

    /// One family member: rep at tau, then the independent (iv-a) cross-check
    /// at the achieved eps (+ slack) must agree with the emitted partition.
    fn family_check<C: EnclosureCurve>(exact: &C) {
        let mut budget = Budget::new(REP_BUDGET, 0, 0);
        let out = rep_curve(
            exact,
            CurveBoundary::Closed,
            TAU_REP,
            ARC_GAP,
            0,
            &mut budget,
        );
        let output = must_rep(out);
        let eps_check = output.certificate.eps_achieved + FAMILY_SLACK;
        let mut cb = Budget::new(REP_BUDGET, 0, 0);
        let _ = must_iso(curve_isotopy_conditions(
            exact,
            CurveBoundary::Closed,
            &output.curve,
            CurveBoundary::Closed,
            eps_check,
            &output.certificate.scale,
            &mut cb,
        ));
    }
}
