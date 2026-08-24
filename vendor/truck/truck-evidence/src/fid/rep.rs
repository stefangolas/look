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
//! Scope, decided for you: CURVE components first (REP-CRV-001); the SURFACE
//! case (REP-SRF-001) has since LANDED in this module: `rep_surface`
//! approximates one exact SURFACE patch to `tau_rep` over a certified
//! uniform-per-axis tensor-product partition and returns the achieved error,
//! the achieved normal-angle margin AND the per-cell (iv-b) discharge TOGETHER
//! (BG-FID-005-SRF). The surface discharge is per-cell over the shared
//! parameter grid: (b) the grid-vertex projection correspondence at every
//! interior vertex (bivariate Krawczyk) and (c) non-adjacent separation over a
//! 2D BVH. The surface double-sheet negative test — two sheets inside one
//! normal tube with correct tangent planes on BOTH — lives in the test module.
//!
//! `sigma_cl` is NOT gated here: standalone rep has no arrangement context;
//! BG-FID-006's consumer adds its condition where it exists.

#![deny(clippy::unwrap_used)]

use super::isotopy::{
    angle_pass_form, box_distance, build_tree, curvature_radius_lower_span, interval,
    norm_sup, self_separation_lower_span, sup_distance_box, uniform_cells, CurveBoundary,
    CurveScaleComponents, KdCell, KdNode,
};
use super::lfs::curvature_radius_lower;
use crate::enclosure::{
    cross_box, immersion_lower_bound_box, interval_at, Box3, DirCone, EnclosureCurve,
    EnclosureSurface, Interval,
};
use crate::num::krawczyk::{krawczyk, KrawczykProof, KrawczykSystem};
use std::ops::Bound;
use truck_base::cgmath64::{EuclideanSpace, Point3, Vector3, Zero};
use truck_base::evidence::{Budget, EnvelopeCase, Refusal, UnresolvedWitness};
use truck_geotrait::{ParameterRange, ParametricCurve, ParametricSurface};

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

// =========================================================================
// SURFACE CASE (BG-FID-005-SRF): `rep_surface`, the tensor-product Hermite
// emitter, the two span helpers, and the per-cell surface (iv-b) discharge.
// =========================================================================

/// The boundary kind of ONE surface patch, per direction, vouched for by the
/// CALLER (the BG-FID-003-r2 CurveBoundary decision lifted to 2D). Drives
/// wrap adjacency in the (iv-b)(c) separation and wrapped gaps in
/// self-separation ONLY; rep_surface runs NO boundary-correspondence gate
/// (the curve rep ran none; that condition belongs to the isotopy checker,
/// which has no surface form yet).
///
/// @establishes the caller's boundary-kind input for ONE surface patch
/// @does-not-establish closedness | openness | any topology claim
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceBoundary {
    /// Both parameter directions are genuine boundary.
    Open,
    /// The u endpoints are identified (periodic in u).
    ClosedU,
    /// The v endpoints are identified (periodic in v).
    ClosedV,
    /// Both directions identified (a torus-like patch).
    ClosedUV,
}

/// Typed refusal. Mirrors RepError's arms and refusal mapping exactly
/// (the fid/ house pattern: one typed enum per operator).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepSurfaceError {
    /// tau_rep <= 0 / non-finite, gap <= 0 / non-finite, or a
    /// non-finitely-bounded exact span on either axis.
    InvalidMargin,
    /// The scale components could not be certified at all (collapsing
    /// geometry, or the scale-stage budget exhausted — BOTH are the
    /// certification-failure route). Routes to §5 collapse via
    /// [`RepSurfaceError::into_refusal`]. NEVER fired merely because
    /// tube_scale is small: small-but-positive refines (Decision 4).
    ReachTooSmall,
    /// Refinement did not reach target within budget, or eps stalled above
    /// target at the enclosure width floor. Carries the spend; never a
    /// best-effort surface.
    Unresolved { subdivisions: u32 },
}

impl RepSurfaceError {
    /// The §4-level view of this refusal.
    ///
    /// `ReachTooSmall` converts to `UnsupportedEnvelope(ReachTooSmall)`, the
    /// §5 collapse route. `Unresolved` converts to `NumericallyUnresolved`
    /// carrying the subdivision spend. `InvalidMargin` has NO §4 arm —
    /// garbage input is `InvalidMargin` here precisely because `Refusal` is
    /// not stretched — so its conversion is `debug_assert!`d never to fire
    /// and returns the nearest arm documenting why.
    pub fn into_refusal(self) -> Refusal {
        match self {
            RepSurfaceError::ReachTooSmall => {
                Refusal::UnsupportedEnvelope(EnvelopeCase::ReachTooSmall)
            }
            RepSurfaceError::InvalidMargin => {
                debug_assert!(
                    false,
                    "InvalidMargin has no §4 arm; rep_surface validates its inputs before any work"
                );
                Refusal::NumericallyUnresolved {
                    spent: Budget::new(0, 0, 0),
                    witness: UnresolvedWitness::UncertifiedContainment,
                }
            }
            RepSurfaceError::Unresolved { subdivisions } => Refusal::NumericallyUnresolved {
                spent: Budget::new(subdivisions, 0, 0),
                witness: UnresolvedWitness::DeviationUncertified,
            },
        }
    }
}

/// Certified scale components for ONE surface patch, named under the
/// BG-FID-001 amendment's rules (the CurveScaleComponents mirror): no field
/// claims tube/reach/lfs semantics; promotion is L-FEDERER-PATCH (open).
/// `+inf` values are intentional (flat; empty separation slice).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceScaleComponents {
    /// From [`surface_curvature_radius_lower_span`].
    pub curvature_radius_lower: f64,
    /// From [`surface_self_separation_lower_span`]; `+inf` when no pair
    /// qualifies (the empty-set identity).
    pub self_separation_lower: f64,
}

impl SurfaceScaleComponents {
    /// `min(curvature_radius_lower, self_separation_lower / 2)` — the
    /// Federer-motivation composition, a gate bound ONLY, never reach.
    pub fn tube_scale_lower(&self) -> f64 {
        self.curvature_radius_lower
            .min(self.self_separation_lower / 2.0)
    }
}

/// The emitted approximant: tensor-product bicubic Hermite in Bezier form
/// over a certified uniform-per-axis grid (Decision 2). Implements
/// [`ParametricSurface`] + [`EnclosureSurface`] so every downstream consumer
/// consumes it through the same traits as any other surface.
#[derive(Debug, Clone)]
pub struct HermiteSurface {
    /// Ascending u knots, `len = n_u + 1`.
    u_knots: Vec<f64>,
    /// Ascending v knots, `len = n_v + 1`.
    v_knots: Vec<f64>,
    /// One tensor-product Hermite cell per grid cell, row-major
    /// `(iu * n_v + iv)`.
    cells: Vec<HermiteCellSurface>,
    /// The u parameter span.
    u_lo: f64,
    /// The u parameter span.
    u_hi: f64,
    /// The v parameter span.
    v_lo: f64,
    /// The v parameter span.
    v_hi: f64,
}

/// One bicubic Hermite cell in Bezier form over `[a, b] x [c, d]`.
#[derive(Debug, Clone, Copy)]
struct HermiteCellSurface {
    /// Cell start parameter in u.
    a: f64,
    /// Cell end parameter in u.
    b: f64,
    /// Cell start parameter in v.
    c: f64,
    /// Cell end parameter in v.
    d: f64,
    /// `b - a`.
    hu: f64,
    /// `d - c`.
    hv: f64,
    /// The 4x4 control net, `net[i][j]` with i = u-index, j = v-index.
    net: [[Point3; 4]; 4],
}

/// The Bernstein factor `3!/(3-m)!` of a degree-3 Bezier's m-th derivative.
/// H-3: dimensionless combinatorial factor of the derivative net.
const FAC: [f64; 4] = [1.0, 3.0, 6.0, 6.0]; // H-3: 3!/(3-m)! for m = 0..=3, dimensionless

/// The relative certificate-change threshold of the two surface span
/// helpers: a level whose certificate moves by less than 5% of itself is
/// converged (Decision 3).
const SURF_CERT_CONV: f64 = 0.05; // H-3: dimensionless relative certificate-change threshold
/// The level cap of the surface span helpers: uniform quad refinement to
/// 2^7 x 2^7 = 16384 cells (Decision 3).
const SURF_LEVEL_CAP: u32 = 7; // H-3: maximum uniform-quad refinement level, dimensionless
/// The per-axis sub-cell subdivision count of the surface measure: each grid
/// cell is split into this many equal sub-cells per axis.
const SURF_MEASURE_SUB: u32 = 4; // H-3: dimensionless per-axis sub-cell subdivision count

/// The width floor of the sliver rule: at or below `8 ulps` at the box's own
/// magnitude the intersection routes through DIRECT derivative evaluation
/// (Decision 2). H-3: a dimensionless width in parameter units, not a length.
fn surface_width_floor(lo: f64, hi: f64) -> f64 {
    8.0 * f64::EPSILON * lo.abs().max(hi.abs()).max(1.0)
}

/// Whether a parameter box lies strictly above the width floor.
fn surface_can_split(iv: &Interval) -> bool {
    iv.sup() - iv.inf() > surface_width_floor(iv.inf(), iv.sup())
}

/// Bisect a parameter box at its midpoint.
fn surface_split(iv: &Interval) -> (Interval, Interval) {
    let mid = 0.5 * iv.inf() + 0.5 * iv.sup();
    (interval(iv.inf(), mid), interval(mid, iv.sup()))
}

/// The midpoint of a box's three coordinate intervals.
fn midpoint_vec(b: &Box3) -> Vector3 {
    Vector3::new(b.x.mid(), b.y.mid(), b.z.mid())
}

/// The midpoint of a degenerate derivative enclosure, the deterministic
/// tangent/twist choice of Decision 2.
fn mid_box_der(b: &Box3) -> Vector3 {
    midpoint_vec(b)
}

/// Linear interpolation of two vectors.
fn lerp_vec(a: Vector3, b: Vector3, t: f64) -> Vector3 {
    a + (b - a) * t
}

/// De Casteljau evaluation of a Bezier curve (in vector space) at `s`.
fn bezier_eval_vec(pts: &[Vector3], s: f64) -> Vector3 {
    let mut cur: Vec<Vector3> = pts.to_vec();
    while cur.len() > 1 {
        let next: Vec<Vector3> = cur.windows(2).map(|w| lerp_vec(w[0], w[1], s)).collect();
        cur = next;
    }
    cur.first().copied().unwrap_or(Vector3::zero())
}

/// Split a cubic Bezier (in vector space) at `t`.
fn bezier_split_vec(p: [Vector3; 4], t: f64) -> ([Vector3; 4], [Vector3; 4]) {
    let [p0, p1, p2, p3] = p;
    let q0 = lerp_vec(p0, p1, t);
    let q1 = lerp_vec(p1, p2, t);
    let q2 = lerp_vec(p2, p3, t);
    let r0 = lerp_vec(q0, q1, t);
    let r1 = lerp_vec(q1, q2, t);
    let s0 = lerp_vec(r0, r1, t);
    ([p0, q0, r0, s0], [s0, r1, q2, p3])
}

/// Restrict a cubic Bezier over `[a, b]` to `[lo, hi]` inside it. A
/// degenerate sub-interval (a point at either end or inside) collapses to
/// the point column.
fn restrict_curve(p: [Point3; 4], a: f64, b: f64, lo: f64, hi: f64) -> [Point3; 4] {
    let h = b - a;
    let s1 = (lo - a) / h;
    let s2 = (hi - a) / h;
    if s1 >= 1.0 {
        return [p[3], p[3], p[3], p[3]];
    }
    if s2 <= 0.0 {
        return [p[0], p[0], p[0], p[0]];
    }
    let (_, right) = bezier_split(p, s1);
    let t2 = (s2 - s1) / (1.0 - s1);
    let (sub, _) = bezier_split(right, t2);
    sub
}

/// Restrict a cubic Bezier (in vector space) over `[a, b]` to `[lo, hi]`.
fn restrict_curve_vec(p: &[Vector3; 4], a: f64, b: f64, lo: f64, hi: f64) -> [Vector3; 4] {
    let h = b - a;
    let s1 = (lo - a) / h;
    let s2 = (hi - a) / h;
    if s1 >= 1.0 {
        return [p[3], p[3], p[3], p[3]];
    }
    if s2 <= 0.0 {
        return [p[0], p[0], p[0], p[0]];
    }
    let (_, right) = bezier_split_vec(*p, s1);
    let t2 = (s2 - s1) / (1.0 - s1);
    let (sub, _) = bezier_split_vec(right, t2);
    sub
}

/// One forward difference along the u axis (rows) of a vector net: each
/// v-column's successive rows are differenced, `n` rows in, `n-1` out.
fn u_diff_once(rows: &[[Vector3; 4]]) -> Vec<[Vector3; 4]> {
    rows.windows(2)
        .map(|w| std::array::from_fn(|j| w[1][j] - w[0][j]))
        .collect()
}

/// The m-th forward difference along the u axis of a 4x4 control net
/// (`net[iu][iv]`), returning the `(4-m) x 4` vector net with the rows
/// u-difference-indexed and each row v-indexed.
fn u_diff_net(net: &[[Point3; 4]; 4], m: usize) -> Vec<[Vector3; 4]> {
    let mut cur: Vec<[Vector3; 4]> = (0..4)
        .map(|i| {
            [
                net[i][0].to_vec(),
                net[i][1].to_vec(),
                net[i][2].to_vec(),
                net[i][3].to_vec(),
            ]
        })
        .collect();
    for _ in 0..m {
        cur = u_diff_once(&cur);
    }
    cur
}

/// The n-th forward difference along the v axis of one row.
fn v_diff(row: &[Vector3; 4], n: usize) -> Vec<Vector3> {
    let mut cur: Vec<Vector3> = row.to_vec();
    for _ in 0..n {
        cur = cur.windows(2).map(|w| w[1] - w[0]).collect();
    }
    cur
}

/// The m-th u- and n-th v-difference net of a 4x4 control net, scaled by the
/// Bernstein factors `3!/(3-m)!/hu^m` and `3!/(3-n)!/hv^n`. Result: a
/// `(4-m) x (4-n)` vector net.
fn derivative_net(
    net: &[[Point3; 4]; 4],
    hu: f64,
    hv: f64,
    m: usize,
    n: usize,
) -> Vec<Vec<Vector3>> {
    let du = u_diff_net(net, m);
    let scale = (FAC[m] / hu.powi(m as i32)) * (FAC[n] / hv.powi(n as i32));
    (0..(4 - m))
        .map(|i| v_diff(&du[i], n).into_iter().map(|x| x * scale).collect())
        .collect()
}

/// Tensor-product Bernstein evaluation of a `(4-m) x (4-n)` vector net at the
/// local parameters `(s, t)`.
fn tensor_eval(net: &[Vec<Vector3>], s: f64, t: f64) -> Vector3 {
    let n_rows = net.len();
    let n_cols = net.first().map(|r| r.len()).unwrap_or(0);
    let mut vpts: Vec<Vector3> = Vec::with_capacity(n_cols);
    for j in 0..n_cols {
        let col: Vec<Vector3> = (0..n_rows).map(|i| net[i][j]).collect();
        vpts.push(bezier_eval_vec(&col, s));
    }
    bezier_eval_vec(&vpts, t)
}

/// The 1D derivative machinery of the sliver route: given a cubic Bezier
/// (in vector space) over `[a, b]`, return the control points of its k-th
/// derivative restricted to `[lo, hi]` (scaled by `3!/(3-k)!/w^k` with
/// `w = hi - lo`), or the direct derivative evaluation at the point when the
/// intersection is degenerate.
fn curve_machinery(p: &[Vector3; 4], a: f64, b: f64, lo: f64, hi: f64, k: usize) -> Vec<Vector3> {
    if lo == hi {
        let t = (lo - a) / (b - a);
        let der: Vec<Vector3> = v_diff(p, k)
            .into_iter()
            .map(|x| x * (FAC[k] / (b - a).powi(k as i32)))
            .collect();
        vec![bezier_eval_vec(&der, t)]
    } else {
        let sub = restrict_curve_vec(p, a, b, lo, hi);
        let w = hi - lo;
        v_diff(&sub, k)
            .into_iter()
            .map(|x| x * (FAC[k] / w.powi(k as i32)))
            .collect()
    }
}

impl HermiteCellSurface {
    /// Build the cell from the exact surface's corner data (Decision 2's
    /// 4x4 net). Positions are `exact.subs` values; tangents and twists are
    /// the midpoints of the degenerate derivative enclosures.
    fn from_exact(exact: &impl EnclosureSurface, a: f64, b: f64, c: f64, d: f64) -> Self {
        let hu = b - a;
        let hv = d - c;
        let hu3 = hu / 3.0;
        let hv3 = hv / 3.0;
        let wh = (hu * hv) / 9.0;
        let p00 = exact.subs(a, c);
        let p30 = exact.subs(b, c);
        let p03 = exact.subs(a, d);
        let p33 = exact.subs(b, d);
        let u00 = mid_box_der(&exact.enclose_der(1, 0, interval_at(a), interval_at(c)));
        let u30 = mid_box_der(&exact.enclose_der(1, 0, interval_at(b), interval_at(c)));
        let u03 = mid_box_der(&exact.enclose_der(1, 0, interval_at(a), interval_at(d)));
        let u33 = mid_box_der(&exact.enclose_der(1, 0, interval_at(b), interval_at(d)));
        let v00 = mid_box_der(&exact.enclose_der(0, 1, interval_at(a), interval_at(c)));
        let v30 = mid_box_der(&exact.enclose_der(0, 1, interval_at(b), interval_at(c)));
        let v03 = mid_box_der(&exact.enclose_der(0, 1, interval_at(a), interval_at(d)));
        let v33 = mid_box_der(&exact.enclose_der(0, 1, interval_at(b), interval_at(d)));
        let w00 = mid_box_der(&exact.enclose_der(1, 1, interval_at(a), interval_at(c)));
        let w30 = mid_box_der(&exact.enclose_der(1, 1, interval_at(b), interval_at(c)));
        let w03 = mid_box_der(&exact.enclose_der(1, 1, interval_at(a), interval_at(d)));
        let w33 = mid_box_der(&exact.enclose_der(1, 1, interval_at(b), interval_at(d)));
        let mut net = [[Point3::new(0.0, 0.0, 0.0); 4]; 4];
        net[0][0] = p00;
        net[3][0] = p30;
        net[0][3] = p03;
        net[3][3] = p33;
        net[1][0] = p00 + u00 * hu3;
        net[2][0] = p30 - u30 * hu3;
        net[1][3] = p03 + u03 * hu3;
        net[2][3] = p33 - u33 * hu3;
        net[0][1] = p00 + v00 * hv3;
        net[0][2] = p03 - v03 * hv3;
        net[3][1] = p30 + v30 * hv3;
        net[3][2] = p33 - v33 * hv3;
        net[1][1] = p00 + u00 * hu3 + v00 * hv3 + w00 * wh;
        net[2][1] = p30 - u30 * hu3 + v30 * hv3 - w30 * wh;
        net[1][2] = p03 + u03 * hu3 - v03 * hv3 - w03 * wh;
        net[2][2] = p33 - u33 * hu3 - v33 * hv3 + w33 * wh;
        HermiteCellSurface {
            a,
            b,
            c,
            d,
            hu,
            hv,
            net,
        }
    }

    /// Restrict the cell's net to the sub-rectangle `[lo_u, hi_u] x
    /// [lo_v, hi_v]` (de Casteljau splits per axis).
    fn restrict_2d(&self, lo_u: f64, hi_u: f64, lo_v: f64, hi_v: f64) -> [[Point3; 4]; 4] {
        let mut mid = [[Point3::new(0.0, 0.0, 0.0); 4]; 4];
        for j in 0..4 {
            let col = [self.net[0][j], self.net[1][j], self.net[2][j], self.net[3][j]];
            let r = restrict_curve(col, self.a, self.b, lo_u, hi_u);
            for i in 0..4 {
                mid[i][j] = r[i];
            }
        }
        let mut out = [[Point3::new(0.0, 0.0, 0.0); 4]; 4];
        for i in 0..4 {
            let row = [mid[i][0], mid[i][1], mid[i][2], mid[i][3]];
            out[i] = restrict_curve(row, self.c, self.d, lo_v, hi_v);
        }
        out
    }

    /// An enclosure of the `(m, n)` derivative over the intersection
    /// `[lo_u, hi_u] x [lo_v, hi_v]` (Decision 2). An intersection whose
    /// width in an axis is at or below the width floor routes through DIRECT
    /// evaluation in that axis (the sliver rule: the restricted-net
    /// derivative scaling divides by the intersection width and explodes on
    /// ulp-wide slivers).
    fn derivative_enclosure(
        &self,
        m: usize,
        n: usize,
        lo_u: f64,
        hi_u: f64,
        lo_v: f64,
        hi_v: f64,
    ) -> Box3 {
        let hu = hi_u - lo_u;
        let hv = hi_v - lo_v;
        let u_sliver = hu <= surface_width_floor(lo_u, hi_u);
        let v_sliver = hv <= surface_width_floor(lo_v, hi_v);
        let pts: Vec<Vector3> = if !u_sliver && !v_sliver {
            let sub = self.restrict_2d(lo_u, hi_u, lo_v, hi_v);
            let net = derivative_net(&sub, hu, hv, m, n);
            net.iter().flatten().copied().collect()
        } else if u_sliver && !v_sliver {
            // u-direct: the u-derivative column at the intersection midpoint
            // (degree-(3-m) Bernstein evaluation of the u-difference net over
            // the CELL u-span), then the 1D v-curve machinery over the v
            // intersection.
            let du = u_diff_net(&self.net, m);
            let su = FAC[m] / self.hu.powi(m as i32);
            let s_mid = (0.5 * (lo_u + hi_u) - self.a) / self.hu;
            let vcur: [Vector3; 4] = std::array::from_fn(|j| {
                let col: Vec<Vector3> = (0..(4 - m)).map(|i| du[i][j] * su).collect();
                bezier_eval_vec(&col, s_mid)
            });
            curve_machinery(&vcur, self.c, self.d, lo_v, hi_v, n)
        } else if !u_sliver && v_sliver {
            // v-direct: the v-derivative column at the intersection midpoint
            // (degree-(3-n) Bernstein evaluation of the v-difference net over
            // the CELL v-span), then the 1D u-curve machinery over the u
            // intersection.
            let sv = FAC[n] / self.hv.powi(n as i32);
            let t_mid = (0.5 * (lo_v + hi_v) - self.c) / self.hv;
            let ucur: [Vector3; 4] = std::array::from_fn(|i| {
                let row = [
                    self.net[i][0].to_vec(),
                    self.net[i][1].to_vec(),
                    self.net[i][2].to_vec(),
                    self.net[i][3].to_vec(),
                ];
                let scaled: Vec<Vector3> = v_diff(&row, n).into_iter().map(|x| x * sv).collect();
                bezier_eval_vec(&scaled, t_mid)
            });
            curve_machinery(&ucur, self.a, self.b, lo_u, hi_u, m)
        } else {
            // Both axes sliver: direct tensor evaluation at the midpoint of
            // the cell's full derivative net (well-conditioned: the scaling
            // uses the CELL widths, never the intersection widths).
            let net = derivative_net(&self.net, self.hu, self.hv, m, n);
            let s = (0.5 * (lo_u + hi_u) - self.a) / self.hu;
            let t = (0.5 * (lo_v + hi_v) - self.c) / self.hv;
            vec![tensor_eval(&net, s, t)]
        };
        hull_box_vec(&pts)
    }
}

/// The index of the cell containing parameter `t` on an ascending knot axis.
fn axis_cell_index(knots: &[f64], t: f64) -> usize {
    let j = knots.partition_point(|k| *k <= t);
    let n = knots.len().saturating_sub(1);
    let idx = j.saturating_sub(1);
    if idx < n {
        idx
    } else {
        n.saturating_sub(1)
    }
}

impl HermiteSurface {
    /// Build the tensor-product Hermite surface over the given ascending knot
    /// vectors from the exact surface, with corner tangents and twists taken
    /// as the exact surface's degenerate enclosure midpoints.
    fn build(
        exact: &impl EnclosureSurface,
        u_knots: Vec<f64>,
        v_knots: Vec<f64>,
    ) -> HermiteSurface {
        let u_lo = u_knots.first().copied().unwrap_or(0.0);
        let u_hi = u_knots.last().copied().unwrap_or(0.0);
        let v_lo = v_knots.first().copied().unwrap_or(0.0);
        let v_hi = v_knots.last().copied().unwrap_or(0.0);
        let n_u = u_knots.len().saturating_sub(1);
        let n_v = v_knots.len().saturating_sub(1);
        let mut cells = Vec::with_capacity(n_u * n_v);
        for iu in 0..n_u {
            let a = u_knots[iu];
            let b = u_knots[iu + 1];
            for iv in 0..n_v {
                let c = v_knots[iv];
                let d = v_knots[iv + 1];
                cells.push(HermiteCellSurface::from_exact(exact, a, b, c, d));
            }
        }
        HermiteSurface {
            u_knots,
            v_knots,
            cells,
            u_lo,
            u_hi,
            v_lo,
            v_hi,
        }
    }

    /// The tensor evaluation of the containing cell's `(m, n)` derivative net
    /// at `(u, v)` (the direct, well-conditioned route: no restriction).
    fn der_at(&self, m: usize, n: usize, u: f64, v: f64) -> Vector3 {
        let n_v = self.v_knots.len().saturating_sub(1);
        let iu = axis_cell_index(&self.u_knots, u);
        let iv = axis_cell_index(&self.v_knots, v);
        let cell = &self.cells[iu * n_v + iv];
        let s = (u - cell.a) / cell.hu;
        let t = (v - cell.c) / cell.hv;
        let net = derivative_net(&cell.net, cell.hu, cell.hv, m, n);
        tensor_eval(&net, s, t)
    }
}

impl ParametricSurface for HermiteSurface {
    type Point = Point3;
    type Vector = Vector3;

    fn subs(&self, u: f64, v: f64) -> Point3 {
        Point3::from_vec(self.der_at(0, 0, u, v))
    }

    fn uder(&self, u: f64, v: f64) -> Vector3 {
        self.der_at(1, 0, u, v)
    }

    fn vder(&self, u: f64, v: f64) -> Vector3 {
        self.der_at(0, 1, u, v)
    }

    fn uuder(&self, u: f64, v: f64) -> Vector3 {
        self.der_at(2, 0, u, v)
    }

    fn uvder(&self, u: f64, v: f64) -> Vector3 {
        self.der_at(1, 1, u, v)
    }

    fn vvder(&self, u: f64, v: f64) -> Vector3 {
        self.der_at(0, 2, u, v)
    }

    fn der_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Vector3 {
        if m > 3 || n > 3 {
            return Vector3::zero();
        }
        self.der_at(m, n, u, v)
    }

    fn parameter_range(&self) -> (ParameterRange, ParameterRange) {
        (
            (Bound::Included(self.u_lo), Bound::Included(self.u_hi)),
            (Bound::Included(self.v_lo), Bound::Included(self.v_hi)),
        )
    }
}

/// The half-open cell-index range on an ascending knot axis overlapping the
/// query, under the curve module's `cell_overlaps` rule (interior overlap, or
/// a degenerate point lying inside the cell). The overlapping cells are
/// contiguous, so a binary search beats an O(cells) scan.
fn overlapping_axis_range(knots: &[f64], q: Interval) -> (usize, usize) {
    let n = knots.len().saturating_sub(1);
    if n == 0 || q.is_empty() {
        return (0, 0);
    }
    if q.inf() == q.sup() {
        // Degenerate point: the cells containing it (at most two, straddling
        // a knot).
        let p = q.inf();
        let j = knots.partition_point(|k| *k <= p);
        let lo = if j >= 1 && j.saturating_sub(1) < n && knots[j.saturating_sub(1)] == p {
            j.saturating_sub(2).min(n)
        } else {
            j.saturating_sub(1).min(n)
        };
        let hi = j.min(n);
        (lo, hi.max(lo))
    } else {
        let lo = knots.partition_point(|k| *k <= q.inf()).saturating_sub(1).min(n);
        let hi = knots.partition_point(|k| *k < q.sup()).min(n);
        (lo, hi.max(lo))
    }
}

impl EnclosureSurface for HermiteSurface {
    fn enclose(&self, uu: Interval, vv: Interval) -> Box3 {
        let n_v = self.v_knots.len().saturating_sub(1);
        let (u_lo, u_hi) = overlapping_axis_range(&self.u_knots, uu);
        let (v_lo, v_hi) = overlapping_axis_range(&self.v_knots, vv);
        let mut acc = Box3::empty();
        for iu in u_lo..u_hi {
            let lo_u = uu.inf().max(self.u_knots[iu]);
            let hi_u = uu.sup().min(self.u_knots[iu + 1]);
            for iv in v_lo..v_hi {
                let lo_v = vv.inf().max(self.v_knots[iv]);
                let hi_v = vv.sup().min(self.v_knots[iv + 1]);
                let cell = &self.cells[iu * n_v + iv];
                let sub = cell.restrict_2d(lo_u, hi_u, lo_v, hi_v);
                let pts: Vec<Point3> = sub.iter().flatten().copied().collect();
                acc = hull_join(&acc, &hull_box(&pts));
            }
        }
        acc
    }

    fn enclose_der(&self, m: usize, n: usize, uu: Interval, vv: Interval) -> Box3 {
        if m > 3 || n > 3 {
            return Box3 {
                x: interval(0.0, 0.0),
                y: interval(0.0, 0.0),
                z: interval(0.0, 0.0),
            };
        }
        let n_v = self.v_knots.len().saturating_sub(1);
        let (u_lo, u_hi) = overlapping_axis_range(&self.u_knots, uu);
        let (v_lo, v_hi) = overlapping_axis_range(&self.v_knots, vv);
        let mut acc = Box3::empty();
        for iu in u_lo..u_hi {
            let lo_u = uu.inf().max(self.u_knots[iu]);
            let hi_u = uu.sup().min(self.u_knots[iu + 1]);
            for iv in v_lo..v_hi {
                let lo_v = vv.inf().max(self.v_knots[iv]);
                let hi_v = vv.sup().min(self.v_knots[iv + 1]);
                let cell = &self.cells[iu * n_v + iv];
                let b = cell.derivative_enclosure(m, n, lo_u, hi_u, lo_v, hi_v);
                acc = hull_join(&acc, &b);
            }
        }
        acc
    }

    fn normal_cone(&self, _uu: Interval, _vv: Interval) -> Option<DirCone> {
        // The emitter provides no cones; consumers use `enclose_der` — the
        // HermiteCurve `tangent_cone` precedent.
        None
    }

    fn immersion_lower_bound(&self, uu: Interval, vv: Interval) -> f64 {
        let du = self.enclose_der(1, 0, uu, vv);
        let dv = self.enclose_der(0, 1, uu, vv);
        immersion_lower_bound_box(&cross_box(&du, &dv))
    }
}

/// What rep_surface proved, and what it achieved. This IS the certificate.
#[derive(Debug, Clone, PartialEq)]
pub struct RepSurfaceCertificate {
    /// Certified achieved two-sided sup-distance exact-vs-emitted.
    pub eps_achieved: f64,
    /// Certified min |cos| over all paired normal boxes (the (ii) margin).
    pub angle_cos_lower: f64,
    /// Final per-axis partition depths (2^depth cells per axis).
    pub depth_u: u32,
    pub depth_v: u32,
    /// The u knots, ascending, echo of the certified partition.
    pub partition_u: Vec<f64>,
    /// The v knots, ascending, echo of the certified partition.
    pub partition_v: Vec<f64>,
    /// Refinement levels spent from the first attempt to the certificate.
    pub subdivisions_spent: u32,
    /// The scale components every gate was evaluated against (echo).
    pub scale: SurfaceScaleComponents,
}

/// rep_surface's success: the surface AND the certificate, together.
#[derive(Debug, Clone)]
pub struct RepSurfaceOutput {
    /// The emitted tensor-product Hermite approximant.
    pub surface: HermiteSurface,
    /// The certificate of what was achieved and what was discharged.
    pub certificate: RepSurfaceCertificate,
}

/// The outcome of the per-cell surface (iv-b) discharge on one partition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceIvbOutcome {
    /// Every interior grid vertex certified and no non-adjacent overlap.
    Pass,
    /// A grid-vertex projection could not be certified: refine.
    ProjectionFailure,
    /// Certified non-adjacent overlap (row-major cell indices): two sheets in
    /// one normal-tube region. Either a partition too coarse (refinement
    /// fixes it) or a genuine self-overlap (route to the self-intersection
    /// engine) — the caller decides. Positive certified claim, not epistemic.
    MultiSheet { cells: (usize, usize) },
}

/// Typed scale-stage refusal (the isotopy helpers' house pattern).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceScaleError {
    /// Bad span or gap input.
    InvalidMargin,
    /// The curvature span helper could not certify (immersion collapse at the
    /// floor, or scale-stage budget exhaustion).
    CurvatureUnresolved,
    /// The separation span helper could not complete.
    SeparationUnresolved,
}

/// Certified lower bound on the exact surface's minimum curvature radius over
/// its whole span: uniform quad refinement, per cell
/// `lfs::curvature_radius_lower` (landed, pub), min over certifiable cells;
/// relative convergence (level change < 5% of the certificate) or the level
/// cap 7 (Decision 3). `+inf` when every cell is flat.
pub fn surface_curvature_radius_lower_span(
    exact: &impl EnclosureSurface,
    budget: &mut Budget,
) -> Result<f64, SurfaceScaleError> {
    let (Some((u0, u1)), Some((v0, v1))) = exact.try_range_tuple() else {
        return Err(SurfaceScaleError::InvalidMargin);
    };
    if !(u0.is_finite() && u1.is_finite() && v0.is_finite() && v1.is_finite()) || u0 >= u1 || v0 >= v1
    {
        return Err(SurfaceScaleError::InvalidMargin);
    }
    let mut cells: Vec<(Interval, Interval)> = vec![(interval(u0, u1), interval(v0, v1))];
    let mut level = 0u32;
    let mut prev = f64::INFINITY;
    loop {
        let mut best = f64::INFINITY;
        let mut had_err = false;
        let mut err_at_floor = false;
        for (uu, vv) in cells.iter() {
            match curvature_radius_lower(exact, (*uu, *vv)) {
                Ok(r) => {
                    if r < best {
                        best = r;
                    }
                }
                Err(_) => {
                    had_err = true;
                    if !surface_can_split(uu) && !surface_can_split(vv) {
                        err_at_floor = true;
                    }
                }
            }
        }
        if err_at_floor {
            return Err(SurfaceScaleError::CurvatureUnresolved);
        }
        if best.is_infinite() && !had_err {
            // Every cell is flat and none refuse: `+inf` is intentional.
            return Ok(f64::INFINITY);
        }
        let cur = best;
        let change = if prev.is_infinite() || cur.is_infinite() {
            f64::INFINITY
        } else {
            (cur - prev).abs()
        };
        if change < SURF_CERT_CONV * cur && cur != 0.0 {
            return Ok(cur);
        }
        if level >= SURF_LEVEL_CAP {
            return Ok(cur);
        }
        prev = cur;
        let mut next = Vec::with_capacity(cells.len() * 4);
        for (uu, vv) in cells {
            if surface_can_split(&uu) && surface_can_split(&vv) {
                budget
                    .spend_subdiv(1)
                    .map_err(|_| SurfaceScaleError::CurvatureUnresolved)?;
                let (u1c, u2c) = surface_split(&uu);
                let (v1c, v2c) = surface_split(&vv);
                next.push((u1c, v1c));
                next.push((u2c, v1c));
                next.push((u1c, v2c));
                next.push((u2c, v2c));
            } else {
                next.push((uu, vv));
            }
        }
        cells = next;
        level += 1;
    }
}

/// A surface cell of the 2D BVH: its parameter box and position enclosure.
#[derive(Clone, Copy)]
struct SurfaceKdCell {
    /// The u parameter box.
    uu: Interval,
    /// The v parameter box.
    vv: Interval,
    /// The position enclosure.
    bb: Box3,
    /// The row-major cell index.
    index: usize,
}

/// One node of the 2D balanced tree over a surface's cells. A node carries
/// the union position box and the union u/v parameter ranges of its subtree,
/// both used for pruning. Median split on the widest position-box axis.
struct SurfaceKdNode {
    /// The union position box of the subtree.
    bb: Box3,
    /// The lower union u bound of the subtree.
    u_lo: f64,
    /// The upper union u bound of the subtree.
    u_hi: f64,
    /// The lower union v bound of the subtree.
    v_lo: f64,
    /// The upper union v bound of the subtree.
    v_hi: f64,
    /// The left child.
    left: Option<Box<SurfaceKdNode>>,
    /// The right child.
    right: Option<Box<SurfaceKdNode>>,
    /// The leaf cell, when this node is a leaf.
    cell: Option<SurfaceKdCell>,
}

/// Build the balanced 2D spatial tree over a surface's cells. `cells` is
/// non-empty at every production call site; an empty input degrades to an
/// empty leaf (defensive, H-1).
fn surface_build_tree(cells: &[SurfaceKdCell]) -> Box<SurfaceKdNode> {
    let Some(first) = cells.first().copied() else {
        return Box::new(SurfaceKdNode {
            bb: Box3::empty(),
            u_lo: f64::INFINITY,
            u_hi: f64::NEG_INFINITY,
            v_lo: f64::INFINITY,
            v_hi: f64::NEG_INFINITY,
            left: None,
            right: None,
            cell: None,
        });
    };
    let mut bb = first.bb;
    let mut u_lo = first.uu.inf();
    let mut u_hi = first.uu.sup();
    let mut v_lo = first.vv.inf();
    let mut v_hi = first.vv.sup();
    for c in cells.iter().skip(1) {
        bb.x = bb.x.convex_hull(c.bb.x);
        bb.y = bb.y.convex_hull(c.bb.y);
        bb.z = bb.z.convex_hull(c.bb.z);
        u_lo = u_lo.min(c.uu.inf());
        u_hi = u_hi.max(c.uu.sup());
        v_lo = v_lo.min(c.vv.inf());
        v_hi = v_hi.max(c.vv.sup());
    }
    if cells.len() == 1 {
        return Box::new(SurfaceKdNode {
            bb,
            u_lo,
            u_hi,
            v_lo,
            v_hi,
            left: None,
            right: None,
            cell: Some(first),
        });
    }
    let wx = bb.x.sup() - bb.x.inf();
    let wy = bb.y.sup() - bb.y.inf();
    let wz = bb.z.sup() - bb.z.inf();
    let axis = if wx >= wy && wx >= wz {
        0usize
    } else if wy >= wz {
        1usize
    } else {
        2usize
    };
    let mid_of = |c: &SurfaceKdCell| match axis {
        0 => c.bb.x.mid(),
        1 => c.bb.y.mid(),
        _ => c.bb.z.mid(),
    };
    let mut keyed: Vec<(f64, SurfaceKdCell)> = cells.iter().map(|c| (mid_of(c), *c)).collect();
    keyed.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mid = keyed.len() / 2;
    let right_keyed = keyed.split_off(mid);
    let left_cells: Vec<SurfaceKdCell> = keyed.into_iter().map(|(_, c)| c).collect();
    let right_cells: Vec<SurfaceKdCell> = right_keyed.into_iter().map(|(_, c)| c).collect();
    Box::new(SurfaceKdNode {
        bb,
        u_lo,
        u_hi,
        v_lo,
        v_hi,
        left: Some(surface_build_tree(&left_cells)),
        right: Some(surface_build_tree(&right_cells)),
        cell: None,
    })
}

/// The certified maximum parameter gap between a cell and a parameter range,
/// the 2D lift of isotopy's `param_gap_max`: on a closed axis of period P the
/// wrapped farthest gap saturates at P/2; on an open axis it is the farthest
/// endpoint distance.
fn surface_param_gap_max(a: &Interval, b_lo: f64, b_hi: f64, closed: bool, period: f64) -> f64 {
    let (a_lo, a_hi) = (a.inf(), a.sup());
    let lo = (b_lo - a_hi).max(a_lo - b_hi).max(0.0);
    let hi = (b_hi - a_lo).max(a_hi - b_lo);
    if !closed {
        return hi;
    }
    let half = 0.5 * period;
    if lo <= half && half <= hi {
        half
    } else if hi < half {
        hi
    } else {
        period - lo
    }
}

/// One query cell's descent of the separation tree: prune nodes whose
/// box-to-box distance to the query cannot lower `best`, prune nodes from
/// which no leaf can satisfy the parameter-gap qualification, and update
/// `best` at qualifying leaves.
fn surface_min_separation(
    node: &SurfaceKdNode,
    query: &SurfaceKdCell,
    gap: f64,
    closed_u: bool,
    closed_v: bool,
    u_period: f64,
    v_period: f64,
    best: &mut f64,
) {
    if box_distance(&query.bb, &node.bb) >= *best {
        return;
    }
    let gu = surface_param_gap_max(&query.uu, node.u_lo, node.u_hi, closed_u, u_period);
    let gv = surface_param_gap_max(&query.vv, node.v_lo, node.v_hi, closed_v, v_period);
    if gu < gap && gv < gap {
        return;
    }
    if let Some(cell) = node.cell {
        let gu = surface_param_gap_max(&query.uu, cell.uu.inf(), cell.uu.sup(), closed_u, u_period);
        let gv = surface_param_gap_max(&query.vv, cell.vv.inf(), cell.vv.sup(), closed_v, v_period);
        if gu.max(gv) >= gap {
            let d = box_distance(&query.bb, &cell.bb);
            if d < *best {
                *best = d;
            }
        }
        return;
    }
    if let Some(l) = &node.left {
        surface_min_separation(l, query, gap, closed_u, closed_v, u_period, v_period, best);
    }
    if let Some(r) = &node.right {
        surface_min_separation(r, query, gap, closed_u, closed_v, u_period, v_period, best);
    }
}

/// Certified lower bound on `min |S(p) - S(q)|` over parameter pairs at
/// Chebyshev parameter gap >= `gap` (Decision 3's qualifying rule). `+inf`
/// when no pair qualifies.
pub fn surface_self_separation_lower_span(
    exact: &impl EnclosureSurface,
    boundary: SurfaceBoundary,
    gap: f64,
    budget: &mut Budget,
) -> Result<f64, SurfaceScaleError> {
    if gap <= 0.0 || !gap.is_finite() {
        return Err(SurfaceScaleError::InvalidMargin);
    }
    let (Some((u0, u1)), Some((v0, v1))) = exact.try_range_tuple() else {
        return Err(SurfaceScaleError::InvalidMargin);
    };
    if !(u0.is_finite() && u1.is_finite() && v0.is_finite() && v1.is_finite()) || u0 >= u1 || v0 >= v1
    {
        return Err(SurfaceScaleError::InvalidMargin);
    }
    let closed_u = matches!(
        boundary,
        SurfaceBoundary::ClosedU | SurfaceBoundary::ClosedUV
    );
    let closed_v = matches!(
        boundary,
        SurfaceBoundary::ClosedV | SurfaceBoundary::ClosedUV
    );
    let u_period = u1 - u0;
    let v_period = v1 - v0;
    let max_gap_u = if closed_u { 0.5 * u_period } else { u_period };
    let max_gap_v = if closed_v { 0.5 * v_period } else { v_period };
    if max_gap_u.max(max_gap_v) < gap {
        // The empty-set identity: no parameter pair can qualify.
        return Ok(f64::INFINITY);
    }
    let mut cells: Vec<(Interval, Interval)> = vec![(interval(u0, u1), interval(v0, v1))];
    let mut level = 0u32;
    let mut prev = f64::INFINITY;
    loop {
        let kd_cells: Vec<SurfaceKdCell> = cells
            .iter()
            .enumerate()
            .map(|(idx, (uu, vv))| SurfaceKdCell {
                uu: *uu,
                vv: *vv,
                bb: exact.enclose(*uu, *vv),
                index: idx,
            })
            .collect();
        let tree = surface_build_tree(&kd_cells);
        let mut best = f64::INFINITY;
        for q in kd_cells.iter() {
            surface_min_separation(
                &tree,
                q,
                gap,
                closed_u,
                closed_v,
                u_period,
                v_period,
                &mut best,
            );
        }
        let cur = best;
        let change = if prev.is_infinite() || cur.is_infinite() {
            f64::INFINITY
        } else {
            (cur - prev).abs()
        };
        if change < SURF_CERT_CONV * cur && cur != 0.0 {
            return Ok(cur);
        }
        if level >= SURF_LEVEL_CAP {
            return Ok(cur);
        }
        prev = cur;
        let mut next = Vec::with_capacity(cells.len() * 4);
        for (uu, vv) in cells {
            if surface_can_split(&uu) && surface_can_split(&vv) {
                budget
                    .spend_subdiv(1)
                    .map_err(|_| SurfaceScaleError::SeparationUnresolved)?;
                let (u1c, u2c) = surface_split(&uu);
                let (v1c, v2c) = surface_split(&vv);
                next.push((u1c, v1c));
                next.push((u2c, v1c));
                next.push((u1c, v2c));
                next.push((u2c, v2c));
            } else {
                next.push((uu, vv));
            }
        }
        cells = next;
        level += 1;
    }
}

/// Measure `eps_now` (max over sub-boxes of the two-sided box-to-box sup
/// distance between the emitted hull and the exact box), `theta_now` (min
/// over the same sub-boxes of the normal-angle (ii) pass form), and the
/// first-order extent per axis, entirely by interval evaluation on the
/// sub-boxes — never by sampling. The per-cell max sup is also returned,
/// because the (iv-b)(c) separation gate is a PER-CELL statement.
fn surface_measure(
    approx: &impl EnclosureSurface,
    exact: &impl EnclosureSurface,
    u_knots: &[f64],
    v_knots: &[f64],
) -> (f64, f64, f64, f64, Vec<f64>) {
    let m = SURF_MEASURE_SUB;
    let n_u = u_knots.len().saturating_sub(1);
    let n_v = v_knots.len().saturating_sub(1);
    let mut eps_now = 0.0;
    let mut theta_now = f64::INFINITY;
    let mut ext_u = 0.0;
    let mut ext_v = 0.0;
    let mut cell_eps = Vec::with_capacity(n_u * n_v);
    for iu in 0..n_u {
        let au = u_knots[iu];
        let bu = u_knots[iu + 1];
        let hu = bu - au;
        for iv in 0..n_v {
            let av = v_knots[iv];
            let bv = v_knots[iv + 1];
            let hv = bv - av;
            let mut cell_max = 0.0;
            for su in 0..m {
                let lo_u = au + hu * (su as f64) / (m as f64);
                let hi_u = au + hu * ((su + 1) as f64) / (m as f64);
                let subw_u = hi_u - lo_u;
                for sv in 0..m {
                    let lo_v = av + hv * (sv as f64) / (m as f64);
                    let hi_v = av + hv * ((sv + 1) as f64) / (m as f64);
                    let subw_v = hi_v - lo_v;
                    let uu = interval(lo_u, hi_u);
                    let vv = interval(lo_v, hi_v);
                    let sup = sup_distance_box(&approx.enclose(uu, vv), &exact.enclose(uu, vv));
                    if sup > cell_max {
                        cell_max = sup;
                    }
                    if sup > eps_now {
                        eps_now = sup;
                    }
                    let an = cross_box(
                        &approx.enclose_der(1, 0, uu, vv),
                        &approx.enclose_der(0, 1, uu, vv),
                    );
                    let en = cross_box(
                        &exact.enclose_der(1, 0, uu, vv),
                        &exact.enclose_der(0, 1, uu, vv),
                    );
                    let ratio = angle_pass_form(&an, &en);
                    if ratio < theta_now {
                        theta_now = ratio;
                    }
                    let eu_box = exact.enclose_der(1, 0, uu, vv);
                    let ev_box = exact.enclose_der(0, 1, uu, vv);
                    let ex_u = subw_u * norm_sup(&eu_box);
                    if ex_u > ext_u {
                        ext_u = ex_u;
                    }
                    let ex_v = subw_v * norm_sup(&ev_box);
                    if ex_v > ext_v {
                        ext_v = ex_v;
                    }
                }
            }
            cell_eps.push(cell_max);
        }
    }
    (eps_now, theta_now, ext_u, ext_v, cell_eps)
}

/// The Krawczyk system for the grid-vertex projection zero: the bivariate
/// normal-projection correspondence `F(s, t) = [<phi - S, S_u>,
/// <phi - S, S_v>]` with `phi = phi(u*, v*)` held fixed (Decision 6(b)). The
/// Jacobian is the second-fundamental-form bracket, never evaluated as a
/// formula, only interval-checked.
struct SurfaceKnotProjection<'a, C: EnclosureSurface> {
    /// The exact surface.
    exact: &'a C,
    /// The emitted grid-vertex point `phi(u*, v*)`.
    phi: Point3,
}

impl<'a, C: EnclosureSurface> KrawczykSystem<2> for SurfaceKnotProjection<'a, C> {
    fn f_point(&self, x: &[f64; 2]) -> [Interval; 2] {
        let [u0, v0] = *x;
        let s = self.exact.enclose(interval_at(u0), interval_at(v0));
        let su = self.exact.enclose_der(1, 0, interval_at(u0), interval_at(v0));
        let sv = self.exact.enclose_der(0, 1, interval_at(u0), interval_at(v0));
        let d = box_minus_point(&s, self.phi);
        [dot_box(&d, &su), dot_box(&d, &sv)]
    }

    fn jacobian(&self, b: &[Interval; 2]) -> [[Interval; 2]; 2] {
        let [uu, vv] = *b;
        let s = self.exact.enclose(uu, vv);
        let su = self.exact.enclose_der(1, 0, uu, vv);
        let sv = self.exact.enclose_der(0, 1, uu, vv);
        let suu = self.exact.enclose_der(2, 0, uu, vv);
        let suv = self.exact.enclose_der(1, 1, uu, vv);
        let svv = self.exact.enclose_der(0, 2, uu, vv);
        let d = box_minus_point(&s, self.phi);
        let j00 = dot_box(&d, &suu) - dot_box(&su, &su);
        let j01 = dot_box(&d, &suv) - dot_box(&su, &sv);
        let j10 = dot_box(&d, &suv) - dot_box(&sv, &su);
        let j11 = dot_box(&d, &svv) - dot_box(&sv, &sv);
        [[j00, j01], [j10, j11]]
    }

    fn preconditioner(&self, x: &[f64; 2]) -> Option<[[f64; 2]; 2]> {
        let [u0, v0] = *x;
        let s = self.exact.enclose(interval_at(u0), interval_at(v0));
        let su = self.exact.enclose_der(1, 0, interval_at(u0), interval_at(v0));
        let sv = self.exact.enclose_der(0, 1, interval_at(u0), interval_at(v0));
        let suu = self.exact.enclose_der(2, 0, interval_at(u0), interval_at(v0));
        let suv = self.exact.enclose_der(1, 1, interval_at(u0), interval_at(v0));
        let svv = self.exact.enclose_der(0, 2, interval_at(u0), interval_at(v0));
        let d = box_minus_point(&s, self.phi);
        let m00 = (dot_box(&d, &suu) - dot_box(&su, &su)).mid();
        let m01 = (dot_box(&d, &suv) - dot_box(&su, &sv)).mid();
        let m10 = (dot_box(&d, &suv) - dot_box(&sv, &su)).mid();
        let m11 = (dot_box(&d, &svv) - dot_box(&sv, &sv)).mid();
        let det = m00 * m11 - m01 * m10;
        if det.is_finite() && det != 0.0 {
            Some([[m11 / det, -m01 / det], [-m10 / det, m00 / det]])
        } else {
            None
        }
    }
}

/// Whether two row-major surface cells are Chebyshev-1 adjacent in the grid
/// indices, PLUS wrap adjacency per closed direction (corner-sharing cells
/// share a fibre and MUST be exempt).
fn surface_adjacent(
    j: usize,
    k: usize,
    n_u: usize,
    n_v: usize,
    closed_u: bool,
    closed_v: bool,
) -> bool {
    let (ju, jv) = (j / n_v, j % n_v);
    let (ku, kv) = (k / n_v, k % n_v);
    let du = if closed_u {
        index_wrap_dist(ju, ku, n_u)
    } else {
        (ju as i64 - ku as i64).unsigned_abs() as usize
    };
    let dv = if closed_v {
        index_wrap_dist(jv, kv, n_v)
    } else {
        (jv as i64 - kv as i64).unsigned_abs() as usize
    };
    du.max(dv) <= 1
}

/// The wrapped index distance on a closed axis of `n` cells.
fn index_wrap_dist(a: usize, b: usize, n: usize) -> usize {
    let d = (a as i64 - b as i64).unsigned_abs();
    d.min(n as u64 - d) as usize
}

/// (iv-b)(c): whether ANY non-adjacent cell pair has
/// `box_distance(H_j, E_k) <= cell_eps[j]`, and the first row-major pair.
/// Runs over the 2D BVH with union-box pruning — never an O(N^2) whole-array
/// double loop. Exposed `pub(crate)` so the seam test can call it directly
/// (the curve module's `separation_violation` pattern).
pub(crate) fn surface_separation_violation(
    u_knots: &[f64],
    v_knots: &[f64],
    emitted_boxes: &[Box3],
    exact_boxes: &[Box3],
    cell_eps: &[f64],
    boundary: SurfaceBoundary,
) -> Option<(usize, usize)> {
    let n_u = u_knots.len().saturating_sub(1);
    let n_v = v_knots.len().saturating_sub(1);
    let closed_u = matches!(boundary, SurfaceBoundary::ClosedU | SurfaceBoundary::ClosedUV);
    let closed_v = matches!(boundary, SurfaceBoundary::ClosedV | SurfaceBoundary::ClosedUV);
    let mut kd: Vec<SurfaceKdCell> = Vec::with_capacity(n_u * n_v);
    for iu in 0..n_u {
        let uu = interval(u_knots[iu], u_knots[iu + 1]);
        for iv in 0..n_v {
            let idx = iu * n_v + iv;
            kd.push(SurfaceKdCell {
                uu,
                vv: interval(v_knots[iv], v_knots[iv + 1]),
                bb: exact_boxes[idx],
                index: idx,
            });
        }
    }
    let tree = surface_build_tree(&kd);
    for j in 0..emitted_boxes.len() {
        let eps_j = cell_eps.get(j).copied().unwrap_or(0.0);
        if let Some(k) = surface_close_non_adjacent(
            &tree,
            &emitted_boxes[j],
            eps_j,
            j,
            n_u,
            n_v,
            closed_u,
            closed_v,
        ) {
            return Some((j, k));
        }
    }
    None
}

/// Whether any leaf of the tree with a box within `eps` of the query box is
/// non-adjacent to cell `j`; the first such leaf's index.
fn surface_close_non_adjacent(
    node: &SurfaceKdNode,
    query: &Box3,
    eps: f64,
    j: usize,
    n_u: usize,
    n_v: usize,
    closed_u: bool,
    closed_v: bool,
) -> Option<usize> {
    if box_distance(query, &node.bb) > eps {
        return None;
    }
    if let Some(cell) = node.cell {
        let k = cell.index;
        if !surface_adjacent(j, k, n_u, n_v, closed_u, closed_v)
            && box_distance(query, &cell.bb) <= eps
        {
            return Some(k);
        }
        return None;
    }
    if let Some(l) = &node.left {
        if let Some(k) = surface_close_non_adjacent(l, query, eps, j, n_u, n_v, closed_u, closed_v)
        {
            return Some(k);
        }
    }
    if let Some(r) = &node.right {
        if let Some(k) = surface_close_non_adjacent(r, query, eps, j, n_u, n_v, closed_u, closed_v)
        {
            return Some(k);
        }
    }
    None
}

/// Discharge (iv-b) per cell on a SHARED parameter grid: (b) at every
/// INTERIOR grid vertex and (c) over whole-cell boxes. `cell_eps[j]` is the
/// per-cell certified deviation (row-major), from the same measurement the
/// loop uses (Decision 4). The failing pair is reported row-major.
///
/// (a) own-cell containment is the per-cell measurement itself — NOT
/// re-implemented here as a radial tube test.
pub fn surface_ivb_discharge(
    exact: &impl EnclosureSurface,
    approx: &impl EnclosureSurface,
    grid: (&[f64], &[f64]),
    boundary: SurfaceBoundary,
    cell_eps: &[f64],
    budget: &mut Budget,
) -> SurfaceIvbOutcome {
    let (u_knots, v_knots) = grid;
    let n_u = u_knots.len().saturating_sub(1);
    let n_v = v_knots.len().saturating_sub(1);
    if n_u == 0 || n_v == 0 {
        return SurfaceIvbOutcome::ProjectionFailure;
    }
    // (b) the grid-vertex projection correspondence at every interior grid
    // vertex (seam lines are NOT checked on closed directions).
    for iu in 1..n_u {
        for iv in 1..n_v {
            let u_star = u_knots[iu];
            let v_star = v_knots[iv];
            let wu = (u_star - u_knots[iu - 1]).max(u_knots[iu + 1] - u_star);
            let wv = (v_star - v_knots[iv - 1]).max(v_knots[iv + 1] - v_star);
            let start = [
                interval(u_star - wu, u_star + wu),
                interval(v_star - wv, v_star + wv),
            ];
            let phi = exact.subs(u_star, v_star);
            let system = SurfaceKnotProjection { exact, phi };
            match krawczyk(&system, &start, budget) {
                Ok(cert) if cert.value == KrawczykProof::Unique => {}
                _ => return SurfaceIvbOutcome::ProjectionFailure,
            }
        }
    }
    // (c) non-adjacent separation over the 2D BVH.
    let mut emitted_boxes = Vec::with_capacity(n_u * n_v);
    let mut exact_boxes = Vec::with_capacity(n_u * n_v);
    for iu in 0..n_u {
        let uu = interval(u_knots[iu], u_knots[iu + 1]);
        for iv in 0..n_v {
            let vv = interval(v_knots[iv], v_knots[iv + 1]);
            emitted_boxes.push(approx.enclose(uu, vv));
            exact_boxes.push(exact.enclose(uu, vv));
        }
    }
    if let Some((j, k)) = surface_separation_violation(
        u_knots,
        v_knots,
        &emitted_boxes,
        &exact_boxes,
        cell_eps,
        boundary,
    ) {
        return SurfaceIvbOutcome::MultiSheet { cells: (j, k) };
    }
    SurfaceIvbOutcome::Pass
}

/// Approximate one exact surface patch to `tau_rep`, certifying the eps/theta
/// gates and discharging (iv-b) on the same partition.
///
/// @feeds [CCS05, Thm 2.1:H1-H3]          # would discharge, conditional on lemmas
/// @via-open-lemma FID-L-TUBE | FID-L-FEDERER-PATCH | FID-L-COVERING | FID-L-SEPARATES
/// @establishes
///     (i)-(ii) between exact and emitted surface at the achieved (eps, theta)
///     + (iv-b) per-cell fibre-block degree-one on the emitted partition
/// @does-not-establish
///     isotopy | homeomorphism | side separation | whole-span one-sheet as a
///     topological claim | surface isotopy conditions (iii) | reach semantics
pub fn rep_surface(
    exact: &impl EnclosureSurface,
    boundary: SurfaceBoundary,
    tau_rep: f64,
    gap: f64,
    initial_depth: u32,
    budget: &mut Budget,
) -> Result<RepSurfaceOutput, RepSurfaceError> {
    if tau_rep <= 0.0 || !tau_rep.is_finite() {
        return Err(RepSurfaceError::InvalidMargin);
    }
    if gap <= 0.0 || !gap.is_finite() {
        return Err(RepSurfaceError::InvalidMargin);
    }
    let (Some((u0, u1)), Some((v0, v1))) = exact.try_range_tuple() else {
        return Err(RepSurfaceError::InvalidMargin);
    };
    if !(u0.is_finite() && u1.is_finite() && v0.is_finite() && v1.is_finite()) || u0 >= u1 || v0 >= v1
    {
        return Err(RepSurfaceError::InvalidMargin);
    }

    // Decision 1: scale components, computed once. Their epistemic refusals
    // propagate as ReachTooSmall — the certification-failure route.
    let scale_error = |e: SurfaceScaleError| match e {
        SurfaceScaleError::InvalidMargin => RepSurfaceError::InvalidMargin,
        SurfaceScaleError::CurvatureUnresolved | SurfaceScaleError::SeparationUnresolved => {
            RepSurfaceError::ReachTooSmall
        }
    };
    let curvature = surface_curvature_radius_lower_span(exact, budget).map_err(scale_error)?;
    let separation =
        surface_self_separation_lower_span(exact, boundary, gap, budget).map_err(scale_error)?;
    let scale = SurfaceScaleComponents {
        curvature_radius_lower: curvature,
        self_separation_lower: separation,
    };
    let tube = scale.tube_scale_lower();
    let target_eps = tau_rep.min(tube / 2.0);

    let mut du = initial_depth;
    let mut dv = initial_depth;
    let mut subdivisions_spent = 0u32;
    let mut prev_eps = f64::INFINITY;
    let mut stalls = 0u32;

    loop {
        // Decision 4: Budget's own exhaustion at the top of each attempt.
        budget.spend_subdiv(1).map_err(|_| RepSurfaceError::Unresolved {
            subdivisions: subdivisions_spent,
        })?;
        subdivisions_spent += 1;

        let u_cells = uniform_cells(u0, u1, du);
        let v_cells = uniform_cells(v0, v1, dv);
        if u_cells.is_empty() || v_cells.is_empty() {
            return Err(RepSurfaceError::Unresolved {
                subdivisions: subdivisions_spent,
            });
        }
        let u_knots = knots_from_cells(&u_cells);
        let v_knots = knots_from_cells(&v_cells);
        let surface = HermiteSurface::build(exact, u_knots.clone(), v_knots.clone());
        let (eps_now, theta_now, ext_u, ext_v, cell_eps) =
            surface_measure(&surface, exact, &u_knots, &v_knots);
        let n_v = v_knots.len().saturating_sub(1);

        if eps_now > target_eps {
            // eps stalled above target at the enclosure width floor: two
            // consecutive depths that barely improve it are Unresolved, never
            // a best-effort surface.
            if prev_eps.is_finite() && eps_now >= prev_eps - STALL_TOL * prev_eps {
                stalls += 1;
                if stalls >= 2 {
                    return Err(RepSurfaceError::Unresolved {
                        subdivisions: subdivisions_spent,
                    });
                }
            } else {
                stalls = 0;
            }
            prev_eps = eps_now;
            if ext_u >= ext_v {
                du += 1;
            } else {
                dv += 1;
            }
            continue;
        }
        if theta_now <= target_eps / tube {
            // (ii) gate at the achieved eps; a failing tangent margin refines.
            if ext_u >= ext_v {
                du += 1;
            } else {
                dv += 1;
            }
            continue;
        }
        match surface_ivb_discharge(
            exact,
            &surface,
            (&u_knots, &v_knots),
            boundary,
            &cell_eps,
            budget,
        ) {
            SurfaceIvbOutcome::Pass => {
                let certificate = RepSurfaceCertificate {
                    eps_achieved: eps_now,
                    angle_cos_lower: theta_now,
                    depth_u: du,
                    depth_v: dv,
                    partition_u: u_knots,
                    partition_v: v_knots,
                    subdivisions_spent,
                    scale,
                };
                return Ok(RepSurfaceOutput {
                    surface,
                    certificate,
                });
            }
            SurfaceIvbOutcome::ProjectionFailure => {
                if ext_u >= ext_v {
                    du += 1;
                } else {
                    dv += 1;
                }
                continue;
            }
            SurfaceIvbOutcome::MultiSheet { cells: (j, k) } => {
                // The refine arm of Decision 4: refine the axis in which the
                // failing pair's index distance is ZERO (the non-separating
                // axis); both nonzero -> larger extent, tie -> u.
                let (ju, jv) = (j / n_v, j % n_v);
                let (ku, kv) = (k / n_v, k % n_v);
                let du_idx = (ju as i64 - ku as i64).unsigned_abs();
                let dv_idx = (jv as i64 - kv as i64).unsigned_abs();
                if du_idx == 0 {
                    du += 1;
                } else if dv_idx == 0 {
                    dv += 1;
                } else if ext_u >= ext_v {
                    du += 1;
                } else {
                    dv += 1;
                }
                continue;
            }
        }
    }
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
    use truck_base::cgmath64::{EuclideanSpace, InnerSpace, Point3, Vector3, Zero};
    use truck_base::evidence::{Budget, EnvelopeCase, Refusal};
    use truck_geotrait::{ParameterRange, ParametricCurve, ParametricSurface};

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

    // ---------------------------------------------------------------------
    // BG-FID-005-SRF: the SURFACE rep witnesses (the packet's 12 tests).
    // ---------------------------------------------------------------------

    /// The house surface radius, in model units.
    const SURF_R: f64 = 2.0; // H-3: house surface radius, a model-space length
    /// The surface rep tolerance (model-space length).
    const SURF_TAU: f64 = 0.3; // H-3: surface rep tolerance, a model-space length
    /// The self-separation parameter gap of the house witnesses.
    const SURF_GAP: f64 = core::f64::consts::PI; // H-3: parameter gap, dimensionless
    /// The belt/patch u start parameter.
    const SURF_U_LO: f64 = core::f64::consts::FRAC_PI_4; // H-3: u start parameter, dimensionless
    /// The belt/patch u end parameter.
    const SURF_U_HI: f64 = 3.0 * core::f64::consts::FRAC_PI_4; // H-3: u end parameter, dimensionless
    /// The open-patch v start parameter.
    const SURF_V_LO: f64 = core::f64::consts::FRAC_PI_4; // H-3: v start parameter, dimensionless
    /// The open-patch v end parameter.
    const SURF_V_HI: f64 = 3.0 * core::f64::consts::FRAC_PI_4; // H-3: v end parameter, dimensionless
    /// The full azimuth span of the belt witnesses.
    const SURF_V_FULL: f64 = core::f64::consts::TAU; // H-3: full azimuth span, a parameter
    /// The small belt's radius: small-but-positive tube_scale EMITS.
    const SMALL_R: f64 = 0.3; // H-3: small belt radius, a model-space length
    /// The pole patch's u end: `u = 0` is the pole cell at EVERY level.
    const POLE_U_HI: f64 = core::f64::consts::FRAC_PI_3; // H-3: pole patch u end, a parameter
    /// Subdivision budget for a full surface rep run.
    const SURF_REP_BUDGET: u32 = 1 << 20; // H-3: subdivision budget count, dimensionless
    /// Subdivision budget for the pole collapse route.
    const SURF_POLE_BUDGET: u32 = 1 << 12; // H-3: subdivision budget count, dimensionless
    /// Subdivision budget for measuring the surface scale spend (test 5).
    const SURF_SCALE_BUDGET: u32 = 1 << 18; // H-3: subdivision budget count, dimensionless
    /// The double sheet's amplitude: strictly INSIDE the tolerance (eps/2).
    const DOUBLE_A: f64 = 0.025; // H-3: double-sheet amplitude, a model-space length
    /// The double-sheet tolerance trap the amplitude must stay strictly below.
    const DOUBLE_EPS: f64 = 0.05; // H-3: the amplitude-halving tolerance, a model-space length
    /// The double cover's u span: the azimuth is covered TWICE.
    const DOUBLE_U_SPAN: f64 = 4.0 * core::f64::consts::PI; // H-3: double-cover u span, a parameter
    /// The hand-widened seam-test box half-extent (test 8).
    const SEAM_HALF_SURF: f64 = 3.0; // H-3: seam box half-extent in model units

    /// Test-only unwrap that stays under the crate's deny list.
    fn must_surf<T>(r: Result<T, RepSurfaceError>) -> T {
        match r {
            Ok(value) => value,
            Err(_) => unreachable!("unit-test surface witness must certify"),
        }
    }

    /// The m-th derivative of sin, point version.
    fn sphere_sin_der_m(x: f64, m: usize) -> f64 {
        match m % 4 {
            0 => x.sin(),
            1 => x.cos(),
            2 => -x.sin(),
            _ => -x.cos(),
        }
    }

    /// The m-th derivative of cos, point version.
    fn sphere_cos_der_m(x: f64, m: usize) -> f64 {
        match m % 4 {
            0 => x.cos(),
            1 => -x.sin(),
            2 => -x.cos(),
            _ => x.sin(),
        }
    }

    /// The m-th derivative of sin, interval version.
    fn sin_der_m(xx: Interval, m: usize) -> Interval {
        match m % 4 {
            0 => sin(xx),
            1 => cos(xx),
            2 => -sin(xx),
            _ => -cos(xx),
        }
    }

    /// The m-th derivative of cos, interval version.
    fn cos_der_m(xx: Interval, m: usize) -> Interval {
        match m % 4 {
            0 => cos(xx),
            1 => -sin(xx),
            2 => -cos(xx),
            _ => sin(xx),
        }
    }

    /// The unit sphere `r * (sin u cos v, sin u sin v, cos u)` over arbitrary
    /// spans: the belt, the open patch, the small belt and the pole patch.
    #[derive(Clone)]
    struct SpherePatch {
        r: f64,
        u_lo: f64,
        u_hi: f64,
        v_lo: f64,
        v_hi: f64,
    }

    impl ParametricSurface for SpherePatch {
        type Point = Point3;
        type Vector = Vector3;

        fn subs(&self, u: f64, v: f64) -> Point3 {
            Point3::new(
                self.r * u.sin() * v.cos(),
                self.r * u.sin() * v.sin(),
                self.r * u.cos(),
            )
        }

        fn uder(&self, u: f64, v: f64) -> Vector3 {
            Vector3::new(
                self.r * u.cos() * v.cos(),
                self.r * u.cos() * v.sin(),
                -self.r * u.sin(),
            )
        }

        fn vder(&self, u: f64, v: f64) -> Vector3 {
            Vector3::new(
                -self.r * u.sin() * v.sin(),
                self.r * u.sin() * v.cos(),
                0.0,
            )
        }

        fn uuder(&self, u: f64, v: f64) -> Vector3 {
            Vector3::new(
                -self.r * u.sin() * v.cos(),
                -self.r * u.sin() * v.sin(),
                -self.r * u.cos(),
            )
        }

        fn uvder(&self, u: f64, v: f64) -> Vector3 {
            Vector3::new(-self.r * u.cos() * v.sin(), self.r * u.cos() * v.cos(), 0.0)
        }

        fn vvder(&self, u: f64, v: f64) -> Vector3 {
            Vector3::new(
                -self.r * u.sin() * v.cos(),
                -self.r * u.sin() * v.sin(),
                0.0,
            )
        }

        fn der_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Vector3 {
            let su = sphere_sin_der_m(u, m);
            let cu = sphere_cos_der_m(u, m);
            let sv = sphere_sin_der_m(v, n);
            let cv = sphere_cos_der_m(v, n);
            let z = if n == 0 { cu } else { 0.0 };
            Vector3::new(self.r * su * cv, self.r * su * sv, self.r * z)
        }

        fn parameter_range(&self) -> (ParameterRange, ParameterRange) {
            (
                (Bound::Included(self.u_lo), Bound::Included(self.u_hi)),
                (Bound::Included(self.v_lo), Bound::Included(self.v_hi)),
            )
        }
    }

    impl EnclosureSurface for SpherePatch {
        fn enclose(&self, uu: Interval, vv: Interval) -> Box3 {
            if uu.is_empty() || vv.is_empty() {
                return Box3::empty();
            }
            let r = interval_at(self.r);
            let su = sin(uu);
            let cu = cos(uu);
            let sv = sin(vv);
            let cv = cos(vv);
            Box3 {
                x: r * su * cv,
                y: r * su * sv,
                z: r * cu,
            }
        }

        fn enclose_der(&self, m: usize, n: usize, uu: Interval, vv: Interval) -> Box3 {
            if uu.is_empty() || vv.is_empty() {
                return Box3::empty();
            }
            let r = interval_at(self.r);
            let su = sin_der_m(uu, m);
            let cu = cos_der_m(uu, m);
            let sv = sin_der_m(vv, n);
            let cv = cos_der_m(vv, n);
            Box3 {
                x: r * su * cv,
                y: r * su * sv,
                z: if n == 0 {
                    r * cu
                } else {
                    interval_at(0.0)
                },
            }
        }

        fn normal_cone(&self, _uu: Interval, _vv: Interval) -> Option<DirCone> {
            None
        }

        fn immersion_lower_bound(&self, uu: Interval, vv: Interval) -> f64 {
            let du = self.enclose_der(1, 0, uu, vv);
            let dv = self.enclose_der(0, 1, uu, vv);
            immersion_lower_bound_box(&cross_box(&du, &dv))
        }
    }

    /// The graph `(u, v, 0.5 + 0.5 sin u sin v)` over `[pi/4, 3pi/4]^2`.
    #[derive(Clone)]
    struct Graph {
        u_lo: f64,
        u_hi: f64,
        v_lo: f64,
        v_hi: f64,
    }

    impl ParametricSurface for Graph {
        type Point = Point3;
        type Vector = Vector3;

        fn subs(&self, u: f64, v: f64) -> Point3 {
            Point3::new(u, v, 0.5 + 0.5 * u.sin() * v.sin())
        }

        fn uder(&self, u: f64, v: f64) -> Vector3 {
            Vector3::new(1.0, 0.0, 0.5 * u.cos() * v.sin())
        }

        fn vder(&self, u: f64, v: f64) -> Vector3 {
            Vector3::new(0.0, 1.0, 0.5 * u.sin() * v.cos())
        }

        fn uuder(&self, u: f64, v: f64) -> Vector3 {
            Vector3::new(0.0, 0.0, -0.5 * u.sin() * v.sin())
        }

        fn uvder(&self, u: f64, v: f64) -> Vector3 {
            Vector3::new(0.0, 0.0, 0.5 * u.cos() * v.cos())
        }

        fn vvder(&self, u: f64, v: f64) -> Vector3 {
            Vector3::new(0.0, 0.0, -0.5 * u.sin() * v.sin())
        }

        fn der_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Vector3 {
            let x = if n == 0 {
                match m {
                    0 => u,
                    1 => 1.0,
                    _ => 0.0,
                }
            } else {
                0.0
            };
            let y = if m == 0 {
                match n {
                    0 => v,
                    1 => 1.0,
                    _ => 0.0,
                }
            } else {
                0.0
            };
            let z = if m == 0 && n == 0 {
                0.5 + 0.5 * u.sin() * v.sin()
            } else {
                0.5 * sphere_sin_der_m(u, m) * sphere_sin_der_m(v, n)
            };
            Vector3::new(x, y, z)
        }

        fn parameter_range(&self) -> (ParameterRange, ParameterRange) {
            (
                (Bound::Included(self.u_lo), Bound::Included(self.u_hi)),
                (Bound::Included(self.v_lo), Bound::Included(self.v_hi)),
            )
        }
    }

    impl EnclosureSurface for Graph {
        fn enclose(&self, uu: Interval, vv: Interval) -> Box3 {
            if uu.is_empty() || vv.is_empty() {
                return Box3::empty();
            }
            Box3 {
                x: uu,
                y: vv,
                z: interval_at(0.5) + interval_at(0.5) * sin(uu) * sin(vv),
            }
        }

        fn enclose_der(&self, m: usize, n: usize, uu: Interval, vv: Interval) -> Box3 {
            if uu.is_empty() || vv.is_empty() {
                return Box3::empty();
            }
            let x = if n == 0 {
                match m {
                    0 => uu,
                    1 => interval_at(1.0),
                    _ => interval_at(0.0),
                }
            } else {
                interval_at(0.0)
            };
            let y = if m == 0 {
                match n {
                    0 => vv,
                    1 => interval_at(1.0),
                    _ => interval_at(0.0),
                }
            } else {
                interval_at(0.0)
            };
            let z = if m == 0 && n == 0 {
                interval_at(0.5) + interval_at(0.5) * sin(uu) * sin(vv)
            } else {
                interval_at(0.5) * sin_der_m(uu, m) * sin_der_m(vv, n)
            };
            Box3 { x, y, z }
        }

        fn normal_cone(&self, _uu: Interval, _vv: Interval) -> Option<DirCone> {
            None
        }

        fn immersion_lower_bound(&self, uu: Interval, vv: Interval) -> f64 {
            let du = self.enclose_der(1, 0, uu, vv);
            let dv = self.enclose_der(0, 1, uu, vv);
            immersion_lower_bound_box(&cross_box(&du, &dv))
        }
    }

    /// The Decision-7 double sheet
    /// `D(u,v) = (R + a cos(u/2)) * (sin v cos u, sin v sin u, cos v)` over
    /// `u in [0, 4pi]` (the azimuth covered twice), `v in [pi/4, 3pi/4]`.
    /// The interval Leibniz table (derive_mn by Leibniz over the u-factors).
    #[derive(Clone)]
    struct DoubleCover {
        r: f64,
        a: f64,
        u_lo: f64,
        u_hi: f64,
        v_lo: f64,
        v_hi: f64,
    }

    impl DoubleCover {
        fn rad_der(&self, u: f64, k: usize) -> f64 {
            if k == 0 {
                self.r + self.a * (u / 2.0).cos()
            } else {
                self.a
                    * 0.5_f64.powi(k as i32)
                    * (u / 2.0 + (k as f64) * core::f64::consts::FRAC_PI_2).cos()
            }
        }

        fn rad_der_iv(&self, uu: Interval, k: usize) -> Interval {
            let half = interval_at(2.0);
            if k == 0 {
                interval_at(self.r) + interval_at(self.a) * cos(uu / half)
            } else {
                interval_at(self.a)
                    * interval_at(0.5_f64.powi(k as i32))
                    * cos(uu / half + interval_at((k as f64) * core::f64::consts::FRAC_PI_2))
            }
        }

        /// `∂u^p ∂v^q` of the underlying sphere factor
        /// `(sin v cos u, sin v sin u, cos v)`.
        fn sphere_der(&self, p: usize, q: usize, u: f64, v: f64) -> Vector3 {
            let sx = sphere_sin_der_m(v, q) * sphere_cos_der_m(u, p);
            let sy = sphere_sin_der_m(v, q) * sphere_sin_der_m(u, p);
            let sz = if p == 0 {
                sphere_cos_der_m(v, q)
            } else {
                0.0
            };
            Vector3::new(sx, sy, sz)
        }

        fn sphere_der_iv(&self, p: usize, q: usize, uu: Interval, vv: Interval) -> Box3 {
            let sx = sin_der_m(vv, q) * cos_der_m(uu, p);
            let sy = sin_der_m(vv, q) * sin_der_m(uu, p);
            let sz = if p == 0 {
                cos_der_m(vv, q)
            } else {
                interval_at(0.0)
            };
            Box3 {
                x: sx,
                y: sy,
                z: sz,
            }
        }
    }

    impl ParametricSurface for DoubleCover {
        type Point = Point3;
        type Vector = Vector3;

        fn subs(&self, u: f64, v: f64) -> Point3 {
            Point3::from_vec(self.der_mn(0, 0, u, v))
        }

        fn uder(&self, u: f64, v: f64) -> Vector3 {
            self.der_mn(1, 0, u, v)
        }

        fn vder(&self, u: f64, v: f64) -> Vector3 {
            self.der_mn(0, 1, u, v)
        }

        fn uuder(&self, u: f64, v: f64) -> Vector3 {
            self.der_mn(2, 0, u, v)
        }

        fn uvder(&self, u: f64, v: f64) -> Vector3 {
            self.der_mn(1, 1, u, v)
        }

        fn vvder(&self, u: f64, v: f64) -> Vector3 {
            self.der_mn(0, 2, u, v)
        }

        fn der_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Vector3 {
            let mut acc = Vector3::new(0.0, 0.0, 0.0);
            let mut binom = 1.0_f64;
            for k in 0..=m {
                let rad_k = self.rad_der(u, k);
                let sphere = self.sphere_der(m - k, n, u, v);
                acc += sphere * (binom * rad_k);
                binom *= (m - k) as f64 / (k + 1) as f64;
            }
            acc
        }

        fn parameter_range(&self) -> (ParameterRange, ParameterRange) {
            (
                (Bound::Included(self.u_lo), Bound::Included(self.u_hi)),
                (Bound::Included(self.v_lo), Bound::Included(self.v_hi)),
            )
        }
    }

    impl EnclosureSurface for DoubleCover {
        fn enclose(&self, uu: Interval, vv: Interval) -> Box3 {
            if uu.is_empty() || vv.is_empty() {
                return Box3::empty();
            }
            let rad = self.rad_der_iv(uu, 0);
            let s = self.sphere_der_iv(0, 0, uu, vv);
            Box3 {
                x: rad * s.x,
                y: rad * s.y,
                z: rad * s.z,
            }
        }

        fn enclose_der(&self, m: usize, n: usize, uu: Interval, vv: Interval) -> Box3 {
            if uu.is_empty() || vv.is_empty() {
                return Box3::empty();
            }
            let mut x = interval_at(0.0);
            let mut y = interval_at(0.0);
            let mut z = interval_at(0.0);
            let mut binom = 1.0_f64;
            for k in 0..=m {
                let rad_k = self.rad_der_iv(uu, k);
                let s = self.sphere_der_iv(m - k, n, uu, vv);
                let c = interval_at(binom);
                x += s.x * rad_k * c;
                y += s.y * rad_k * c;
                z += s.z * rad_k * c;
                binom *= (m - k) as f64 / (k + 1) as f64;
            }
            Box3 { x, y, z }
        }

        fn normal_cone(&self, _uu: Interval, _vv: Interval) -> Option<DirCone> {
            None
        }

        fn immersion_lower_bound(&self, uu: Interval, vv: Interval) -> f64 {
            let du = self.enclose_der(1, 0, uu, vv);
            let dv = self.enclose_der(0, 1, uu, vv);
            immersion_lower_bound_box(&cross_box(&du, &dv))
        }
    }

    /// A parameterization transposition wrapper: `(u, v)` swapped.
    #[derive(Clone)]
    struct Transposed<S>(S);

    impl<S: EnclosureSurface<Vector = Vector3>> ParametricSurface for Transposed<S> {
        type Point = Point3;
        type Vector = Vector3;

        fn subs(&self, u: f64, v: f64) -> Point3 {
            self.0.subs(v, u)
        }

        fn uder(&self, u: f64, v: f64) -> Vector3 {
            self.0.vder(v, u)
        }

        fn vder(&self, u: f64, v: f64) -> Vector3 {
            self.0.uder(v, u)
        }

        fn uuder(&self, u: f64, v: f64) -> Vector3 {
            self.0.vvder(v, u)
        }

        fn uvder(&self, u: f64, v: f64) -> Vector3 {
            self.0.uvder(v, u)
        }

        fn vvder(&self, u: f64, v: f64) -> Vector3 {
            self.0.uuder(v, u)
        }

        fn der_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Vector3 {
            self.0.der_mn(n, m, v, u)
        }

        fn parameter_range(&self) -> (ParameterRange, ParameterRange) {
            let (ur, vr) = self.0.parameter_range();
            (vr, ur)
        }
    }

    impl<S: EnclosureSurface<Vector = Vector3>> EnclosureSurface for Transposed<S> {
        fn enclose(&self, uu: Interval, vv: Interval) -> Box3 {
            self.0.enclose(vv, uu)
        }

        fn enclose_der(&self, m: usize, n: usize, uu: Interval, vv: Interval) -> Box3 {
            self.0.enclose_der(n, m, vv, uu)
        }

        fn normal_cone(&self, uu: Interval, vv: Interval) -> Option<DirCone> {
            self.0.normal_cone(vv, uu)
        }

        fn immersion_lower_bound(&self, uu: Interval, vv: Interval) -> f64 {
            self.0.immersion_lower_bound(vv, uu)
        }
    }

    /// The belt witness: radius SURF_R, `u in [pi/4, 3pi/4]`,
    /// `v in [0, 2pi]`, ClosedV.
    fn belt() -> SpherePatch {
        SpherePatch {
            r: SURF_R,
            u_lo: SURF_U_LO,
            u_hi: SURF_U_HI,
            v_lo: 0.0,
            v_hi: SURF_V_FULL,
        }
    }

    /// The open patch witness: `u, v in [pi/4, 3pi/4]`, Open.
    fn open_patch() -> SpherePatch {
        SpherePatch {
            r: SURF_R,
            u_lo: SURF_U_LO,
            u_hi: SURF_U_HI,
            v_lo: SURF_V_LO,
            v_hi: SURF_V_HI,
        }
    }

    /// The small belt: radius SMALL_R, same spans.
    fn small_belt() -> SpherePatch {
        SpherePatch {
            r: SMALL_R,
            u_lo: SURF_U_LO,
            u_hi: SURF_U_HI,
            v_lo: 0.0,
            v_hi: SURF_V_FULL,
        }
    }

    /// The pole patch: `u in [0, pi/3]` (touches the pole), `v in [pi/4,
    /// 3pi/4]`.
    fn pole_patch() -> SpherePatch {
        SpherePatch {
            r: SURF_R,
            u_lo: 0.0,
            u_hi: POLE_U_HI,
            v_lo: SURF_V_LO,
            v_hi: SURF_V_HI,
        }
    }

    /// The graph witness over `[pi/4, 3pi/4]^2`.
    fn graph() -> Graph {
        Graph {
            u_lo: SURF_U_LO,
            u_hi: SURF_U_HI,
            v_lo: SURF_V_LO,
            v_hi: SURF_V_HI,
        }
    }

    /// The double-cover witness (Decision 7).
    fn double_cover() -> DoubleCover {
        DoubleCover {
            r: SURF_R,
            a: DOUBLE_A,
            u_lo: 0.0,
            u_hi: DOUBLE_U_SPAN,
            v_lo: SURF_V_LO,
            v_hi: SURF_V_HI,
        }
    }

    /// The min |cos| between the double cover's surface normal and the
    /// underlying sphere's normal over a dense sample (correct tangent planes
    /// on BOTH sheets).
    fn double_sheet_normal_cos_lower(exact: &DoubleCover) -> f64 {
        const N_SAMPLES: usize = 40; // H-3: dimensionless dense-sample count per axis
        let mut min_cos = f64::INFINITY;
        for i in 0..N_SAMPLES {
            let u = (0.0 + (i as f64 + 0.5) / (N_SAMPLES as f64)) * DOUBLE_U_SPAN;
            for j in 0..N_SAMPLES {
                let v = SURF_V_LO
                    + (j as f64 + 0.5) / (N_SAMPLES as f64) * (SURF_V_HI - SURF_V_LO);
                let n = exact.uder(u, v).cross(exact.vder(u, v)).normalize();
                let sphere_n = Vector3::new(v.sin() * u.cos(), v.sin() * u.sin(), v.cos());
                let c = n.dot(sphere_n).abs();
                if c < min_cos {
                    min_cos = c;
                }
            }
        }
        min_cos
    }

    #[test]
    fn rep_surface_belt_from_coarse_certifies() {
        let exact = belt();
        let mut budget = Budget::new(SURF_REP_BUDGET, 0, 0);
        let out = rep_surface(
            &exact,
            SurfaceBoundary::ClosedV,
            SURF_TAU,
            SURF_GAP,
            0,
            &mut budget,
        );
        let output = must_surf(out);
        // Machine: eps 0.118463 at (4,5), angle 0.784307 against s = 0.5.
        assert!(
            output.certificate.eps_achieved <= 0.26,
            "eps {} exceeds the certified bound",
            output.certificate.eps_achieved
        );
        assert!(
            output.certificate.angle_cos_lower >= 0.6,
            "angle {} below the certified margin",
            output.certificate.angle_cos_lower
        );
        assert!(
            output.certificate.depth_u >= 3 && output.certificate.depth_v >= 4,
            "depths ({}, {}) below the machine-emitting (4, 5)",
            output.certificate.depth_u,
            output.certificate.depth_v
        );
        assert!(output.certificate.partition_u.len() >= 9);
        assert!(output.certificate.partition_v.len() >= 17);
        assert!(
            output.certificate.subdivisions_spent >= 2,
            "refined past the coarse depths, spent {}",
            output.certificate.subdivisions_spent
        );
    }

    #[test]
    fn rep_surface_open_patch_certifies() {
        let exact = open_patch();
        let mut budget = Budget::new(SURF_REP_BUDGET, 0, 0);
        let out = rep_surface(
            &exact,
            SurfaceBoundary::Open,
            SURF_TAU,
            SURF_GAP,
            0,
            &mut budget,
        );
        let output = must_surf(out);
        // Machine: eps 0.118463 at (4,3).
        assert!(
            output.certificate.eps_achieved <= 0.297,
            "eps {} exceeds the certified bound",
            output.certificate.eps_achieved
        );
        assert!(
            output.certificate.scale.self_separation_lower.is_infinite(),
            "the open patch's separation must be the +inf empty-set identity"
        );
    }

    #[test]
    fn coarse_belt_refines_and_emits() {
        let exact = small_belt();
        let mut budget = Budget::new(SURF_REP_BUDGET, 0, 0);
        let out = rep_surface(
            &exact,
            SurfaceBoundary::ClosedV,
            SURF_TAU,
            SURF_GAP,
            0,
            &mut budget,
        );
        let output = must_surf(out);
        // target = min(0.3, 0.076008/2) = 0.038004; machine eps 0.017769.
        assert!(
            output.certificate.eps_achieved <= 0.0381,
            "small belt must emit at its own target, achieved {}",
            output.certificate.eps_achieved
        );
    }

    #[test]
    fn pole_patch_routes_to_collapse() {
        let exact = pole_patch();
        let mut budget = Budget::new(SURF_POLE_BUDGET, 0, 0);
        let out = rep_surface(
            &exact,
            SurfaceBoundary::Open,
            SURF_TAU,
            SURF_GAP,
            0,
            &mut budget,
        );
        let e = match out {
            Err(e) => e,
            Ok(_) => unreachable!("a pole patch must route to collapse"),
        };
        assert!(matches!(e, RepSurfaceError::ReachTooSmall));
        assert!(matches!(
            e.into_refusal(),
            Refusal::UnsupportedEnvelope(EnvelopeCase::ReachTooSmall)
        ));
    }

    #[test]
    fn surface_budget_exhaustion_refuses() {
        let exact = belt();
        let mut cb = Budget::new(SURF_SCALE_BUDGET, 0, 0);
        let _ = surface_curvature_radius_lower_span(&exact, &mut cb);
        let curv_spent = SURF_SCALE_BUDGET - cb.subdiv;
        let mut sb = Budget::new(SURF_SCALE_BUDGET, 0, 0);
        let _ = surface_self_separation_lower_span(&exact, SurfaceBoundary::ClosedV, SURF_GAP, &mut sb);
        let sep_spent = SURF_SCALE_BUDGET - sb.subdiv;
        let mut budget = Budget::new(curv_spent + sep_spent + 2, 0, 0);
        let out = rep_surface(
            &exact,
            SurfaceBoundary::ClosedV,
            SURF_TAU,
            SURF_GAP,
            0,
            &mut budget,
        );
        match out {
            Err(RepSurfaceError::Unresolved { subdivisions }) => {
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
    fn rep_surface_is_idempotent() {
        let exact = belt();
        let mut b1 = Budget::new(SURF_REP_BUDGET, 0, 0);
        let e1 = must_surf(rep_surface(
            &exact,
            SurfaceBoundary::ClosedV,
            SURF_TAU,
            SURF_GAP,
            0,
            &mut b1,
        ));
        let mut b2 = Budget::new(SURF_REP_BUDGET, 0, 0);
        let e2 = must_surf(rep_surface(
            &exact,
            SurfaceBoundary::ClosedV,
            SURF_TAU,
            SURF_GAP,
            0,
            &mut b2,
        ));
        // (a) determinism: the two certificates are EQUAL.
        assert_eq!(e1.certificate, e2.certificate);
        // (b) the metamorphic re-rep: the emission implements EnclosureSurface;
        // this exercises Decision 2's sliver routing (the new grid's queries
        // straddle the emission's knots).
        let mut b3 = Budget::new(SURF_REP_BUDGET, 0, 0);
        let rerun = must_surf(rep_surface(
            &e1.surface,
            SurfaceBoundary::ClosedV,
            SURF_TAU,
            SURF_GAP,
            0,
            &mut b3,
        ));
        assert!(
            rerun.certificate.eps_achieved <= 0.26,
            "re-rep must emit, achieved {}",
            rerun.certificate.eps_achieved
        );
    }

    #[test]
    fn transposed_parameterization_also_emits() {
        let exact = open_patch();
        let mut b1 = Budget::new(SURF_REP_BUDGET, 0, 0);
        let e1 = must_surf(rep_surface(
            &exact,
            SurfaceBoundary::Open,
            SURF_TAU,
            SURF_GAP,
            0,
            &mut b1,
        ));
        let mut b2 = Budget::new(SURF_REP_BUDGET, 0, 0);
        let e2 = must_surf(rep_surface(
            &Transposed(exact),
            SurfaceBoundary::Open,
            SURF_TAU,
            SURF_GAP,
            0,
            &mut b2,
        ));
        // Machine: (4,3) and (3,4) respectively.
        assert!(e1.certificate.eps_achieved <= 0.297);
        assert!(e2.certificate.eps_achieved <= 0.297);
    }

    #[test]
    fn ivb_separation_failure_reports_multisheet() {
        // The seam test: build the belt emission at the machine-emitting grid
        // (4, 5), compute the cell boxes and cell_eps, and call the pub(crate)
        // separation scan directly: no violation on the clean grid, then a
        // violation whose reported pair includes the hand-widened cell.
        let exact = belt();
        let u_cells = uniform_cells(SURF_U_LO, SURF_U_HI, 4);
        let v_cells = uniform_cells(0.0, SURF_V_FULL, 5);
        let u_knots = knots_from_cells(&u_cells);
        let v_knots = knots_from_cells(&v_cells);
        let surface = HermiteSurface::build(&exact, u_knots.clone(), v_knots.clone());
        let (_, _, _, _, cell_eps) = surface_measure(&surface, &exact, &u_knots, &v_knots);
        let n_u = u_knots.len().saturating_sub(1);
        let n_v = v_knots.len().saturating_sub(1);
        let mut emitted_boxes = Vec::with_capacity(n_u * n_v);
        let mut exact_boxes = Vec::with_capacity(n_u * n_v);
        for iu in 0..n_u {
            let uu = interval(u_knots[iu], u_knots[iu + 1]);
            for iv in 0..n_v {
                let vv = interval(v_knots[iv], v_knots[iv + 1]);
                emitted_boxes.push(surface.enclose(uu, vv));
                exact_boxes.push(exact.enclose(uu, vv));
            }
        }
        assert!(
            surface_separation_violation(
                &u_knots,
                &v_knots,
                &emitted_boxes,
                &exact_boxes,
                &cell_eps,
                SurfaceBoundary::ClosedV
            )
            .is_none(),
            "the emitting grid must have no non-adjacent separation violation"
        );
        if let Some(b) = exact_boxes.get_mut(0) {
            *b = Box3 {
                x: interval(-SEAM_HALF_SURF, SEAM_HALF_SURF),
                y: interval(-SEAM_HALF_SURF, SEAM_HALF_SURF),
                z: interval(-SEAM_HALF_SURF, SEAM_HALF_SURF),
            };
        }
        match surface_separation_violation(
            &u_knots,
            &v_knots,
            &emitted_boxes,
            &exact_boxes,
            &cell_eps,
            SurfaceBoundary::ClosedV,
        ) {
            Some((j, k)) => {
                assert!(
                    j == 0 || k == 0,
                    "the widened cell must be in the reported pair, got ({j}, {k})"
                );
            }
            None => unreachable!("the widened cell must drive a non-adjacent pair within eps"),
        }
    }

    #[test]
    fn double_sheet_is_multisheet() {
        // THE negative test: two sheets inside one normal tube with correct
        // tangent planes on BOTH. Build the emission at the FIXED grid
        // (7, 5), measure cell_eps, and call the discharge directly.
        let exact = double_cover();
        let u_cells = uniform_cells(0.0, DOUBLE_U_SPAN, 7);
        let v_cells = uniform_cells(SURF_V_LO, SURF_V_HI, 5);
        let u_knots = knots_from_cells(&u_cells);
        let v_knots = knots_from_cells(&v_cells);
        let surface = HermiteSurface::build(&exact, u_knots.clone(), v_knots.clone());
        let (_, _, _, _, cell_eps) = surface_measure(&surface, &exact, &u_knots, &v_knots);
        let mut budget = Budget::new(SURF_REP_BUDGET, 0, 0);
        let outcome = surface_ivb_discharge(
            &exact,
            &surface,
            (&u_knots, &v_knots),
            SurfaceBoundary::ClosedU,
            &cell_eps,
            &mut budget,
        );
        match outcome {
            SurfaceIvbOutcome::MultiSheet { cells: (j, k) } => {
                let n_u = u_knots.len().saturating_sub(1);
                let n_v = v_knots.len().saturating_sub(1);
                let (ju, jv) = (j / n_v, j % n_v);
                let (ku, kv) = (k / n_v, k % n_v);
                let d = (ju as i64 - ku as i64).unsigned_abs();
                let half = (n_u / 2) as i64;
                assert!(
                    ((d as i64) - half).abs() <= 2,
                    "u-index distance {d} not within 2 of n_u/2 = {half} (cells ({ju},{jv}) x ({ku},{kv}))"
                );
            }
            SurfaceIvbOutcome::Pass => unreachable!("a double sheet must be MultiSheet, not Pass"),
            SurfaceIvbOutcome::ProjectionFailure => {
                unreachable!("a double sheet must be MultiSheet, not ProjectionFailure")
            }
        }
        // The witness's own geometry: amplitude strictly inside eps, and
        // correct tangent planes on BOTH sheets.
        assert!(
            DOUBLE_A < DOUBLE_EPS,
            "the amplitude must be strictly inside the tolerance"
        );
        assert!(
            double_sheet_normal_cos_lower(&exact) >= 0.999,
            "the sheets' tangent planes must be within 0.999 |cos| of the sphere's"
        );
    }

    #[test]
    fn double_cover_rep_never_emits() {
        // The double cover's separation certificate is 0 (the sheets coincide),
        // so tube = 0 and target = 0: the eps gate can never pass. Measure the
        // deterministic scale spend, then hand the loop exactly that plus ~2:
        // the loop exhausts and refuses Unresolved carrying the spend — rep
        // never certifies a double sheet.
        let exact = double_cover();
        let mut cb = Budget::new(SURF_SCALE_BUDGET, 0, 0);
        let _ = surface_curvature_radius_lower_span(&exact, &mut cb);
        let curv_spent = SURF_SCALE_BUDGET - cb.subdiv;
        let mut sb = Budget::new(SURF_SCALE_BUDGET, 0, 0);
        let _ = surface_self_separation_lower_span(&exact, SurfaceBoundary::ClosedU, SURF_GAP, &mut sb);
        let sep_spent = SURF_SCALE_BUDGET - sb.subdiv;
        let mut budget = Budget::new(curv_spent + sep_spent + 2, 0, 0);
        let out = rep_surface(
            &exact,
            SurfaceBoundary::ClosedU,
            SURF_TAU,
            SURF_GAP,
            0,
            &mut budget,
        );
        match out {
            Err(RepSurfaceError::Unresolved { subdivisions }) => {
                assert!(
                    subdivisions >= 2,
                    "the spend must be carried, got {subdivisions}"
                )
            }
            Ok(_) => unreachable!("rep must never certify a double sheet"),
            Err(_) => unreachable!("the double cover routes to Unresolved"),
        }
    }

    #[test]
    fn surface_invalid_inputs_refuse() {
        let exact = belt();
        for bad_tau in [0.0, -SURF_TAU, f64::NAN, f64::INFINITY] {
            let mut budget = Budget::new(SURF_REP_BUDGET, 0, 0);
            let out = rep_surface(
                &exact,
                SurfaceBoundary::ClosedV,
                bad_tau,
                SURF_GAP,
                0,
                &mut budget,
            );
            assert!(
                matches!(out, Err(RepSurfaceError::InvalidMargin)),
                "tau = {bad_tau} must refuse as InvalidMargin"
            );
            assert_eq!(
                budget.subdiv, SURF_REP_BUDGET,
                "no budget spend on invalid input"
            );
        }
        for bad_gap in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            let mut budget = Budget::new(SURF_REP_BUDGET, 0, 0);
            let out = rep_surface(
                &exact,
                SurfaceBoundary::ClosedV,
                SURF_TAU,
                bad_gap,
                0,
                &mut budget,
            );
            assert!(
                matches!(out, Err(RepSurfaceError::InvalidMargin)),
                "gap = {bad_gap} must refuse as InvalidMargin"
            );
            assert_eq!(
                budget.subdiv, SURF_REP_BUDGET,
                "no budget spend on invalid input"
            );
        }
    }

    #[test]
    fn rep_surface_family_conditions_hold() {
        surface_family_check(&belt(), SurfaceBoundary::ClosedV);
        surface_family_check(&open_patch(), SurfaceBoundary::Open);
        surface_family_check(&graph(), SurfaceBoundary::Open);
    }

    /// One surface family member: rep at tau must emit within the tolerance.
    fn surface_family_check<C: EnclosureSurface>(exact: &C, boundary: SurfaceBoundary) {
        let mut budget = Budget::new(SURF_REP_BUDGET, 0, 0);
        let out = rep_surface(exact, boundary, SURF_TAU, SURF_GAP, 0, &mut budget);
        let output = must_surf(out);
        assert!(
            output.certificate.eps_achieved <= 0.3,
            "eps {} exceeds the family tolerance",
            output.certificate.eps_achieved
        );
    }

    #[test]
    #[ignore]
    fn scratch_rerep() {
        let exact = belt();
        let mut b1 = Budget::new(SURF_REP_BUDGET, 0, 0);
        let e1 = must_surf(rep_surface(
            &exact,
            SurfaceBoundary::ClosedV,
            SURF_TAU,
            SURF_GAP,
            0,
            &mut b1,
        ));
        eprintln!(
            "first rep ok at ({}, {})",
            e1.certificate.depth_u, e1.certificate.depth_v
        );
        // scale spends of the emission
        let mut cb = Budget::new(SURF_SCALE_BUDGET, 0, 0);
        let t0 = std::time::Instant::now();
        let curv = surface_curvature_radius_lower_span(&e1.surface, &mut cb);
        eprintln!("re-rerep curvature = {curv:?} in {:.2}s", t0.elapsed().as_secs_f64());
        let mut sb = Budget::new(SURF_SCALE_BUDGET, 0, 0);
        let t1 = std::time::Instant::now();
        let sep = surface_self_separation_lower_span(&e1.surface, SurfaceBoundary::ClosedV, SURF_GAP, &mut sb);
        eprintln!("re-rerep separation = {sep:?} in {:.2}s", t1.elapsed().as_secs_f64());
        let mut b3 = Budget::new(SURF_REP_BUDGET, 0, 0);
        let t2 = std::time::Instant::now();
        let rerun = rep_surface(&e1.surface, SurfaceBoundary::ClosedV, SURF_TAU, SURF_GAP, 0, &mut b3);
        eprintln!("re-rerep total {:.2}s -> {rerun:?}", t2.elapsed().as_secs_f64());
    }
}
