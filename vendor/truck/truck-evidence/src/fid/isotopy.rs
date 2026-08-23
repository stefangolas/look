//! BG-FID-003: the whole-span isotopy CONDITIONS (i)-(iv-a) for CURVE
//! components.
//!
//! This module certifies the CONDITIONS of the isotopy lemma on ONE curve
//! component pair — it never certifies isotopy itself. Conditions (i)-(iii)
//! make the normal projection restricted to an approximant a proper local
//! homeomorphism — a covering of SOME constant finite degree. Condition (iv-a)
//! certifies, at ONE witnessed normal disc, that the fibre multiplicity is
//! one. Together they are DESIGNED to discharge the hypotheses of [CCS05]
//! Thms 2.1/2.2 — but that discharge is CONDITIONAL on the open bridge lemmas
//! below; this module certifies the CONDITIONS, never isotopy itself, and
//! never claims more than one witnessed disc for (iv-a): the promotion of one
//! witnessed fibre to whole-span one-sheetness is the open L-COVERING lemma's
//! consequence of (i)-(iii), not something this module proves.
//!
//! Scope, decided for you: CURVE components only, one (exact, approx) pair
//! per call. The surface case and the discharge (iv-b) land with BG-FID-005,
//! where the emitter's cell partition makes them free; both deferrals are
//! documented here, neither is stubbed.
//!
//! # The bridge lemmas (certificate site)
//!
//! L-TUBE       eps < reach(X) => the closed eps-tube of a compact C??
//!              surface-with-boundary is a topological thickening whose sides
//!              are the offset sheets. STATUS: OPEN (closed case = classical
//!              tubular neighborhood theorem; the with-boundary restriction is
//!              ours).
//! L-FEDERER-PATCH  a cell at certified distance h from its trimmed boundary,
//!              curvature bounded above by K, and certified exclusion of
//!              non-incident sheets within radius r has a single-valued normal
//!              tube of radius min(1/K, r, h). STATUS: OPEN — until it lands,
//!              CurveScaleComponents and tube_scale_lower() are certified
//!              COMPONENTS and a gate bound, never reach.
//! L-COVERING   transversality/local-inverse (ii) + properness + certified
//!              fibre multiplicity one (iv) => the fibre projection is a
//!              ONE-SHEETED COVERING => homeomorphism. STATUS: OPEN. The
//!              promotion of ONE witnessed fibre to whole-span one-sheetness
//!              is exactly this lemma's consequence of (i)-(iii): NOT proved
//!              here, NOT claimed here.
//! L-SEPARATES  a continuous one-sheet SECTION of the product thickening
//!              separates the thickening's sides; the section property comes
//!              from L-COVERING's homeomorphism inverse. STATUS: OPEN.
//! Chain: (i)-(iii) + (iv) => local homeomorphism => covering => homeomorphism
//!         => continuous section => side separation => CCS05 Thm 2.1 isotopy.
//! THIS MODULE ESTABLISHES THE CONDITIONS. THE CHAIN IS NOT A PROOF UNTIL THE
//! LEMMAS LAND.

#![deny(clippy::unwrap_used)]

use super::one_sheet::{fibre_degree_one_auto, FibreMultiplicity, OneSheetError};
use crate::enclosure::{
    cross_box, immersion_lower_bound_box, interval_at, Box3, EnclosureCurve, Interval,
};
use truck_base::evidence::Budget;

/// The boundary kind of ONE curve component, vouched for by the CALLER.
/// `EnclosureCurve` carries no topology: whether a component's parameter
/// endpoints are identified (Closed) or are genuine boundary (Open) is a
/// claim about the carrier's topology, supplied here as input. This type
/// makes no claim of its own; a wrong claim from the caller is outside
/// this module's certificate (the both-Closed seam gate below detects
/// gross inconsistency, it does not establish closedness).
///
/// @establishes the caller's boundary-kind input for ONE curve component
/// @does-not-establish closedness | openness | any topology claim
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveBoundary {
    /// Parameter endpoints are the same geometric point (periodic).
    Closed,
    /// Parameter endpoints are genuine boundary points.
    Open,
}

/// Certified scale components for ONE curve component's whole span, named
/// under the BG-FID-001 amendment's rules (see lfs.rs's FaceScaleComponents,
/// the pattern to mirror): each field certifies exactly ONE direction and
/// composes into nothing. `+inf` values are intentional (straight line;
/// empty separation slice).
///
/// @via-open-lemma FID-L-FEDERER-PATCH
/// @establishes component-wise certified directions (this struct)
/// @does-not-establish reach | tube width | lfs
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurveScaleComponents {
    /// From [`curvature_radius_lower_span`]; `+inf` for a straight line.
    pub curvature_radius_lower: f64,
    /// From [`self_separation_lower_span`]; `+inf` when no cell pair
    /// qualifies at the requested parameter gap.
    pub self_separation_lower: f64,
}

impl CurveScaleComponents {
    /// Plain component-wise minimum (the FaceScaleComponents mirror).
    /// Extended-real: `+inf` components are ignored by `f64::min`.
    pub fn conservative_min(&self) -> f64 {
        self.curvature_radius_lower.min(self.self_separation_lower)
    }

    /// `min(curvature_radius_lower, self_separation_lower / 2)` — the
    /// Federer-motivation composition `reach = min(1/kappa_max, half the
    /// bottleneck)` for a CLOSED curve, used ONLY as the gate bound in the
    /// inequality form (BG-FID-007: substituting a lower bound can only
    /// refuse more). This method claims NO reach semantics: the promotion
    /// of this composition to a tube/reach statement is L-FEDERER-PATCH,
    /// open. The `1/2` is the motivation shape, not a proved equality.
    pub fn tube_scale_lower(&self) -> f64 {
        self.curvature_radius_lower
            .min(self.self_separation_lower / 2.0)
    }
}

/// The certified inputs and achieved margins of one whole-span conditions
/// check on one curve component pair.
///
/// @feeds [CCS05, Thm 2.1:H1-H3]          # would discharge, conditional on lemmas
/// @via-open-lemma FID-L-TUBE | FID-L-FEDERER-PATCH | FID-L-COVERING | FID-L-SEPARATES
/// @establishes
///     conditions (i)-(iii) of ??6.2 on ONE curve component pair
///     + (iv-a): certified fibre cardinality on ONE witnessed normal disc
/// @does-not-establish
///     isotopy | homeomorphism | side separation | whole-span one-sheet (iv) |
///     surface case | (iv-b) | reach semantics for the scale components
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IsotopyConditionsReport {
    /// The eps every condition was certified against (the input, echoed).
    pub eps: f64,
    /// The scale components every gate was evaluated against (the input,
    /// echoed). There is deliberately NO bare `rho_lower` field: the
    /// achieved gate bound is `scale.tube_scale_lower()`, and echoing it
    /// as a value would re-claim what the components only motivate.
    pub scale: CurveScaleComponents,
}

/// Typed failures. Every `*Unresolved` arm is EPISTEMIC: a claim about the
/// run, never about the geometry. The `*Violation`/`MultiSheet` arms are
/// POSITIVE certified claims that the condition fails.
///
/// @feeds [CCS05, Thm 2.1:H1-H3]          # would discharge, conditional on lemmas
/// @via-open-lemma FID-L-TUBE | FID-L-FEDERER-PATCH | FID-L-COVERING | FID-L-SEPARATES
/// @establishes the certified failures and epistemic refusals of the conditions check
/// @does-not-establish
///   isotopy | homeomorphism | side separation | whole-span one-sheet (iv)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IsotopyConditionsError {
    /// eps <= 0, non-finite eps, a parameter span not finitely bounded on
    /// either curve, or (on the separation helper) arc_gap <= 0 / non-finite.
    InvalidMargin,
    /// `2*eps >= scale.tube_scale_lower()`: the tube budget exceeds the
    /// composed certified bound. EPISTEMIC per spec: the BOUND could not be
    /// certified large enough — it says nothing about the geometry.
    ReachLowerBoundTooSmall,
    /// (i) certified failed: a floor-width cell box has certified distance
    /// > eps to EVERY cell of the other curve.
    ClosenessViolation { witness_cell: Interval },
    /// (ii) certified failed: a paired cell box exhibits a tangent pair
    /// whose SPACE angle reaches the bound (Decision 2's two-sided test).
    AngleViolation {
        approx_cell: Interval,
        exact_cell: Interval,
    },
    /// (iii) certified failed: boundary kinds disagree (one Closed, one
    /// Open — circle-vs-interval is not isotopy and no geometric endpoint
    /// check can catch it), an endpoint of one curve is > eps from every
    /// endpoint of the other, or a both-Closed input fails the seam
    /// consistency gate.
    BoundaryMismatch,
    /// (iv-a): the witnessed disc met the approximant a certified count
    /// != 1 times (`count == 0` is the coverage-violation arm).
    MultiSheet { count: usize },
    /// (i) could not decide within budget / width floor.
    ClosenessUnresolved,
    /// (ii) could not decide within budget / width floor.
    AngleUnresolved,
    /// (iv-a) propagated from BG-FID-008: root isolation unresolved.
    DegreeOneUnresolved,
    /// (iv-a) propagated from BG-FID-008: bad witness (all ladder points
    /// refused).
    InvalidWitness,
    /// `curvature_radius_lower_span` could not certify a positive immersion
    /// bound at any refinement (epistemic; returning `+inf` here would be
    /// the over-estimate this crate must never produce).
    CurvatureUnresolved,
    /// `self_separation_lower_span` could not complete within budget
    /// (epistemic).
    SeparationUnresolved,
}

/// Certifies conditions (i)-(iii) and (iv-a) of ??6.2 on ONE curve component
/// pair, entirely by interval evaluation. This is a conditions checker, not
/// an isotopy certificate: every step below is a certified bound on the
/// CONDITIONS, and the conditions-to-isotopy step is the open lemma chain of
/// the module docs. Order of evaluation: `InvalidMargin` checks — the tube
/// gate — (i) two-sided eps-closeness — (iii) endpoint correspondence — (ii)
/// the tangent-SPACE angle bound — (iv-a) the witnessed one-sheet disc.
/// (iv-a) is LAST because its decisiveness hangs on (i)-(iii) holding (the
/// L-COVERING dependency); the landed `fibre_degree_one` documents that
/// precondition and this order honours it.
///
/// The both-Closed seam gate is a consistency gate on the caller's Closed
/// claim (a truly closed curve's two endpoint enclosures contain the same
/// geometric point, so they are ~0 apart), NOT a closedness certificate:
/// closure of either curve is never claimed as a topological fact; the
/// carrier owns its own topology.
///
/// @feeds [CCS05, Thm 2.1:H1-H3]          # would discharge, conditional on lemmas
/// @via-open-lemma FID-L-TUBE | FID-L-FEDERER-PATCH | FID-L-COVERING | FID-L-SEPARATES
/// @establishes
///     conditions (i)-(iii) of ??6.2 on ONE curve component pair
///     + (iv-a): certified fibre cardinality on ONE witnessed normal disc
/// @does-not-establish
///     isotopy | homeomorphism | side separation | whole-span one-sheet (iv) |
///     surface case | (iv-b) | reach semantics for the scale components
pub fn curve_isotopy_conditions(
    exact: &impl EnclosureCurve,
    exact_boundary: CurveBoundary,
    approx: &impl EnclosureCurve,
    approx_boundary: CurveBoundary,
    eps: f64,
    scale: &CurveScaleComponents,
    budget: &mut Budget,
) -> Result<IsotopyConditionsReport, IsotopyConditionsError> {
    if eps <= 0.0 || !eps.is_finite() {
        return Err(IsotopyConditionsError::InvalidMargin);
    }
    let Some((e_lo, e_hi)) = exact.try_range_tuple() else {
        return Err(IsotopyConditionsError::InvalidMargin);
    };
    if !(e_lo.is_finite() && e_hi.is_finite()) || e_lo >= e_hi {
        return Err(IsotopyConditionsError::InvalidMargin);
    }
    let Some((a_lo, a_hi)) = approx.try_range_tuple() else {
        return Err(IsotopyConditionsError::InvalidMargin);
    };
    if !(a_lo.is_finite() && a_hi.is_finite()) || a_lo >= a_hi {
        return Err(IsotopyConditionsError::InvalidMargin);
    }
    if 2.0 * eps >= scale.tube_scale_lower() {
        return Err(IsotopyConditionsError::ReachLowerBoundTooSmall);
    }
    let pairs = closeness_check(approx, (a_lo, a_hi), exact, (e_lo, e_hi), eps, budget)?;
    endpoint_check(
        exact,
        (e_lo, e_hi),
        exact_boundary,
        approx,
        (a_lo, a_hi),
        approx_boundary,
        eps,
    )?;
    angle_check(pairs, approx, exact, eps, scale.tube_scale_lower(), budget)?;
    match fibre_degree_one_auto(exact, approx, eps, budget) {
        Ok(FibreMultiplicity::ExactlyOne) => {}
        Ok(FibreMultiplicity::NotOne { count }) => {
            return Err(IsotopyConditionsError::MultiSheet { count });
        }
        Err(OneSheetError::SheetCountUnresolved) => {
            return Err(IsotopyConditionsError::DegreeOneUnresolved);
        }
        Err(OneSheetError::InvalidWitness) => {
            return Err(IsotopyConditionsError::InvalidWitness);
        }
    }
    Ok(IsotopyConditionsReport { eps, scale: *scale })
}

/// Certified lower bound on the exact curve's minimum curvature radius over
/// its whole span: `1 / kappa_upper` with
/// `kappa_upper = sup_t |X' x X''| / (inf_t |X'|)^3`. `+inf` when the
/// numerator bracket is 0 (a straight line). Uses
/// `crate::enclosure::cross_box` and `immersion_lower_bound_box` (both
/// already `pub(crate)`) — do NOT duplicate them locally. A span whose
/// tangent enclosure contains zero at every refinement refuses
/// `CurvatureUnresolved` (never `+inf`: that would claim straightness).
///
/// The span is bisected uniformly; the certificate is the min over the
/// cells' certified radii and the loop converges when the certificate stops
/// moving by more than [`CERT_CONV`] per level (the same refinement
/// discipline as [`self_separation_lower_span`]).
///
/// @via-open-lemma FID-L-FEDERER-PATCH
/// @establishes component-wise certified directions (a lower bound on the
///   minimum curvature radius over the whole span)
/// @does-not-establish reach | tube width | lfs
pub fn curvature_radius_lower_span(
    exact: &impl EnclosureCurve,
    budget: &mut Budget,
) -> Result<f64, IsotopyConditionsError> {
    let Some((lo, hi)) = exact.try_range_tuple() else {
        return Err(IsotopyConditionsError::InvalidMargin);
    };
    if !(lo.is_finite() && hi.is_finite()) || lo >= hi {
        return Err(IsotopyConditionsError::InvalidMargin);
    }
    let mut cells: Vec<Interval> = vec![interval(lo, hi)];
    let mut prev = f64::INFINITY;
    loop {
        let mut best = f64::INFINITY;
        let mut had_imm_zero = false;
        let mut zero_at_floor = false;
        for tt in cells.iter() {
            let d1 = exact.enclose_der(1, *tt);
            let imm = immersion_lower_bound_box(&d1);
            if imm == 0.0 {
                had_imm_zero = true;
                if !can_subdivide(*tt) {
                    zero_at_floor = true;
                }
                continue;
            }
            let d2 = exact.enclose_der(2, *tt);
            let numerator = norm_sup(&cross_box(&d1, &d2));
            if numerator == 0.0 {
                continue;
            }
            let radius = imm * imm * imm / numerator;
            if radius < best {
                best = radius;
            }
        }
        if zero_at_floor {
            return Err(IsotopyConditionsError::CurvatureUnresolved);
        }
        if best.is_infinite() && !had_imm_zero {
            // Every certifiable cell is straight: the whole span is straight,
            // and `+inf` is the intentional straight-line value. Never reach
            // this arm with an imm==0 cell present (that would claim
            // straightness where the tangent is undefined).
            return Ok(f64::INFINITY);
        }
        let cur = best;
        let change = if prev.is_infinite() || cur.is_infinite() {
            f64::INFINITY
        } else {
            (cur - prev).abs()
        };
        if change < CERT_CONV && cur != 0.0 {
            return Ok(cur);
        }
        prev = cur;
        let mut next = Vec::with_capacity(cells.len() * 2);
        let mut refined = false;
        for tt in cells {
            if can_subdivide(tt) {
                budget
                    .spend_subdiv(1)
                    .map_err(|_| IsotopyConditionsError::CurvatureUnresolved)?;
                let (a, b) = split(tt);
                next.push(a);
                next.push(b);
                refined = true;
            } else {
                next.push(tt);
            }
        }
        cells = next;
        if !refined {
            return Ok(cur);
        }
    }
}

/// Certified lower bound on `min |X(s) - X(t)|` over parameter pairs at
/// certified PARAMETER gap >= arc_gap: partition the span by bisection;
/// for every pair of cells (I, J) whose parameter gap qualifies, the
/// box-to-box distance of the position enclosures is a certified lower
/// bound on the arc-to-arc distance; the minimum over qualifying pairs is
/// the certificate. PARAMETER-gap semantics, stated in the doc: with a
/// derivative lower bound m, parameter gap G implies arc gap >= m*G, and a
/// consumer wanting arc gap A passes G = A/m. For `CurveBoundary::Closed`
/// the parameter gap is the WRAPPED distance `min(|s-t|, span-|s-t|)`
/// (a closed loop's two sides both count); for Open it is `|s-t|`.
/// `+inf` when no pair qualifies (the empty-set identity, e.g. any curve
/// with arc_gap >= span). arc_gap <= 0 / non-finite -> InvalidMargin.
/// Budget exhaustion -> SeparationUnresolved.
///
/// The span is bisected uniformly; the certificate converges when it stops
/// moving by more than [`CERT_CONV`] per level. The qualifying-pair search
/// runs over the same balanced spatial tree as condition (i), with
/// best-so-far pruning (`box_distance >= current best` cannot lower the
/// minimum) and the parameter-gap precondition pruning.
///
/// @via-open-lemma FID-L-FEDERER-PATCH
/// @establishes component-wise certified directions (a lower bound on the
///   minimum self-separation at certified parameter gap)
/// @does-not-establish reach | tube width | lfs
pub fn self_separation_lower_span(
    exact: &impl EnclosureCurve,
    boundary: CurveBoundary,
    arc_gap: f64,
    budget: &mut Budget,
) -> Result<f64, IsotopyConditionsError> {
    if arc_gap <= 0.0 || !arc_gap.is_finite() {
        return Err(IsotopyConditionsError::InvalidMargin);
    }
    let Some((lo, hi)) = exact.try_range_tuple() else {
        return Err(IsotopyConditionsError::InvalidMargin);
    };
    if !(lo.is_finite() && hi.is_finite()) || lo >= hi {
        return Err(IsotopyConditionsError::InvalidMargin);
    }
    let span = hi - lo;
    let closed = boundary == CurveBoundary::Closed;
    if closed {
        if arc_gap > 0.5 * span {
            return Ok(f64::INFINITY);
        }
    } else if arc_gap >= span {
        return Ok(f64::INFINITY);
    }
    let mut cells: Vec<Interval> = vec![interval(lo, hi)];
    let mut prev = f64::INFINITY;
    loop {
        let tree = build_curve_tree(&cells, exact);
        let cur = min_separation(&tree, &cells, exact, arc_gap, closed, span);
        let change = if prev.is_infinite() || cur.is_infinite() {
            f64::INFINITY
        } else {
            (cur - prev).abs()
        };
        if change < CERT_CONV && cur != 0.0 {
            return Ok(cur);
        }
        prev = cur;
        let mut next = Vec::with_capacity(cells.len() * 2);
        let mut refined = false;
        for tt in cells {
            if can_subdivide(tt) {
                budget
                    .spend_subdiv(1)
                    .map_err(|_| IsotopyConditionsError::SeparationUnresolved)?;
                let (a, b) = split(tt);
                next.push(a);
                next.push(b);
                refined = true;
            } else {
                next.push(tt);
            }
        }
        cells = next;
        if !refined {
            return Ok(cur);
        }
    }
}

/// Convergence threshold for the two span-certificate refinement loops: a
/// refinement level that moves the certificate by less than this is deemed
/// converged. The interval enclosures make the certified bounds' deficit
/// linear in the cell width, so an aggressively small threshold would push
/// the bisection to impractical depths; this threshold stops once the
/// certificate is within the tests' witness tolerance. H-3: a dimensionless
/// change threshold on the certified values, not a model-space length.
const CERT_CONV: f64 = 0.01; // H-3: dimensionless certificate-change threshold

/// A valid parameter interval from a runtime pair; a malformed pair degrades
/// to the empty interval rather than panicking (H-1).
pub(crate) fn interval(lo: f64, hi: f64) -> Interval {
    Interval::try_from((lo, hi)).unwrap_or(Interval::EMPTY)
}

/// The uniform cell list over a finite span: the `2^depth` cells
/// `[lo + k·h, lo + (k+1)·h]` with `h = (hi - lo)/2^depth`. BG-FID-005's
/// emitter partition is exactly this list, so rep's (iv-b) discharge shares
/// the exact curve's parameter space cell-for-cell (Decision 4 of the packet:
/// the pairing is the identity and no search is needed).
pub(crate) fn uniform_cells(lo: f64, hi: f64, depth: u32) -> Vec<Interval> {
    let n = match 1usize.checked_shl(depth) {
        Some(n) => n,
        None => return Vec::new(),
    };
    let h = (hi - lo) / n as f64;
    let mut cells = Vec::with_capacity(n);
    for k in 0..n {
        cells.push(interval(lo + (k as f64) * h, lo + ((k + 1) as f64) * h));
    }
    cells
}

/// At or below this width a parameter box cannot subdivide further. The
/// floor is RELATIVE to the parameter magnitude: 8 ulps at the box's own
/// scale, never below 8 ulps of a unit-width interval (the same floor
/// `one_sheet.rs` uses).
/// H-3: a dimensionless width in parameter units, not a model-space length.
fn width_floor(tt: &Interval) -> f64 {
    8.0 * f64::EPSILON * tt.inf().abs().max(tt.sup().abs()).max(1.0) // H-3: 8 ulps at the box magnitude
}

/// Whether a parameter box lies strictly above the width floor and can
/// therefore be bisected.
fn can_subdivide(tt: Interval) -> bool {
    tt.sup() - tt.inf() > width_floor(&tt)
}

/// Bisect a parameter box at its midpoint and return the two halves.
fn split(tt: Interval) -> (Interval, Interval) {
    let mid = 0.5 * tt.inf() + 0.5 * tt.sup();
    (interval(tt.inf(), mid), interval(mid, tt.sup()))
}

/// The interval dot product of two boxes, an enclosure of
/// `{ a · b : a in A, b in B }`. Duplicated locally exactly as `one_sheet.rs`
/// duplicates it; `enclosure.rs` visibility stays untouched.
pub(crate) fn dot_box(a: &Box3, b: &Box3) -> Interval {
    a.x * b.x + a.y * b.y + a.z * b.z
}

/// A lower bound on the point-set distance between two boxes: per-axis
/// `max(lo_b - hi_a, lo_a - hi_b)` clamped at 0, Euclidean-combined.
/// Duplicated locally exactly as `one_sheet.rs` does; the point-box
/// degenerate case (`b_lo == b_hi`) is what makes this the sibling of the
/// sup form below.
pub(crate) fn box_distance(a: &Box3, b: &Box3) -> f64 {
    let gap = |lo_a: f64, hi_a: f64, lo_b: f64, hi_b: f64| (lo_b - hi_a).max(lo_a - hi_b).max(0.0);
    let dx = gap(a.x.inf(), a.x.sup(), b.x.inf(), b.x.sup());
    let dy = gap(a.y.inf(), a.y.sup(), b.y.inf(), b.y.sup());
    let dz = gap(a.z.inf(), a.z.sup(), b.z.inf(), b.z.sup());
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// An upper bound on the point-set distance between two boxes: the farthest
/// corner pair, per coordinate `max(|a_lo - b_hi|, |a_hi - b_lo|)`, then
/// `sqrt(sum_i of squares)`. The box-to-box form Decision 2(i) mandates; the
/// point box is the degenerate case `b_lo == b_hi`. Do NOT reuse
/// `one_sheet::sup_distance`: that helper's second operand is a `Point3`
/// (box-to-point), and the box operand needs this form.
pub(crate) fn sup_distance_box(a: &Box3, b: &Box3) -> f64 {
    let farthest =
        |lo_a: f64, hi_a: f64, lo_b: f64, hi_b: f64| (lo_a - hi_b).abs().max((hi_a - lo_b).abs());
    let dx = farthest(a.x.inf(), a.x.sup(), b.x.inf(), b.x.sup());
    let dy = farthest(a.y.inf(), a.y.sup(), b.y.inf(), b.y.sup());
    let dz = farthest(a.z.inf(), a.z.sup(), b.z.inf(), b.z.sup());
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// The smallest magnitude over an interval, `0.0` when it contains 0.
pub(crate) fn abs_lower(i: Interval) -> f64 {
    if i.contains(0.0) {
        0.0
    } else {
        i.inf().abs().min(i.sup().abs())
    }
}

/// The largest magnitude over an interval.
fn abs_upper(i: Interval) -> f64 {
    i.inf().abs().max(i.sup().abs())
}

/// An upper bound on `‖v‖` over a box: the interval norm's upper endpoint.
pub(crate) fn norm_sup(b: &Box3) -> f64 {
    (b.x.sqr() + b.y.sqr() + b.z.sqr()).sqrt().sup()
}

/// A lower bound on `‖v‖` over a box: the interval norm's lower endpoint.
pub(crate) fn norm_inf(b: &Box3) -> f64 {
    (b.x.sqr() + b.y.sqr() + b.z.sqr()).sqrt().inf()
}

/// The per-pair tangent-box evaluation of condition (ii) in its PASS form:
/// `abs_lower(dot)/(norm_sup · norm_sup)` for two first-derivative boxes
/// (denominators only shrink, so a ratio above `s` is a certified pass).
/// This is EXACTLY the `pass_ratio` of [`angle_check`]; rep measures its
/// whole-partition `theta_now` as the minimum of this form over the identity
/// pairings (BG-FID-005 Decision 3).
pub(crate) fn angle_pass_form(da: &Box3, de: &Box3) -> f64 {
    let na = norm_sup(da);
    let ne = norm_sup(de);
    if na == 0.0 || ne == 0.0 {
        return 0.0;
    }
    abs_lower(dot_box(da, de)) / (na * ne)
}

/// A curve cell: its parameter box and its position enclosure.
#[derive(Clone, Copy)]
pub(crate) struct KdCell {
    /// The parameter box of the cell.
    pub(crate) tt: Interval,
    /// The position enclosure of the cell.
    pub(crate) bb: Box3,
}

/// One node of the balanced binary tree over a curve's cells. A node carries
/// the union position box and the union parameter range of its subtree, both
/// used for pruning. Median split on the widest-interval axis.
pub(crate) struct KdNode {
    /// The union position box of the subtree.
    pub(crate) bb: Box3,
    /// The lower union parameter bound of the subtree.
    pub(crate) param_lo: f64,
    /// The upper union parameter bound of the subtree.
    pub(crate) param_hi: f64,
    /// The left child.
    pub(crate) left: Option<Box<KdNode>>,
    /// The right child.
    pub(crate) right: Option<Box<KdNode>>,
    /// The leaf cell, when this node is a leaf.
    pub(crate) cell: Option<KdCell>,
}

/// Build the balanced spatial tree over a curve's cells. `cells` is
/// non-empty at every call site (every span is a finite box); an empty input
/// degrades to a leaf that never matches (defensive, H-1).
pub(crate) fn build_tree(cells: &[KdCell]) -> Box<KdNode> {
    let first = cells.first().copied().unwrap_or(KdCell {
        tt: Interval::EMPTY,
        bb: Box3::empty(),
    });
    let mut bb = first.bb;
    let mut param_lo = first.tt.inf();
    let mut param_hi = first.tt.sup();
    for c in cells.iter().skip(1) {
        bb.x = bb.x.convex_hull(c.bb.x);
        bb.y = bb.y.convex_hull(c.bb.y);
        bb.z = bb.z.convex_hull(c.bb.z);
        param_lo = param_lo.min(c.tt.inf());
        param_hi = param_hi.max(c.tt.sup());
    }
    if cells.len() == 1 {
        return Box::new(KdNode {
            bb,
            param_lo,
            param_hi,
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
    let mid_of = |c: &KdCell| match axis {
        0 => c.bb.x.mid(),
        1 => c.bb.y.mid(),
        _ => c.bb.z.mid(),
    };
    let mut keyed: Vec<(f64, KdCell)> = cells.iter().map(|c| (mid_of(c), *c)).collect();
    keyed.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mid = keyed.len() / 2;
    let right_keyed = keyed.split_off(mid);
    let left_cells: Vec<KdCell> = keyed.into_iter().map(|(_, c)| c).collect();
    let right_cells: Vec<KdCell> = right_keyed.into_iter().map(|(_, c)| c).collect();
    Box::new(KdNode {
        bb,
        param_lo,
        param_hi,
        left: Some(build_tree(&left_cells)),
        right: Some(build_tree(&right_cells)),
        cell: None,
    })
}

/// Enclose every cell of a parameter partition and build the tree over them.
pub(crate) fn build_curve_tree(cells: &[Interval], curve: &impl EnclosureCurve) -> Box<KdNode> {
    let kd_cells: Vec<KdCell> = cells
        .iter()
        .map(|tt| KdCell {
            tt: *tt,
            bb: curve.enclose(*tt),
        })
        .collect();
    build_tree(&kd_cells)
}

/// Whether the tree contains a leaf whose box is within `eps` (sup-distance)
/// of the query box. Pruning: `box_distance > eps` implies `sup_distance >
/// eps`, so no partner hides inside a pruned node.
pub(crate) fn node_partner(node: &KdNode, query: &Box3, eps: f64) -> bool {
    if box_distance(query, &node.bb) > eps {
        return false;
    }
    if let Some(cell) = node.cell {
        return sup_distance_box(query, &cell.bb) <= eps;
    }
    if let Some(l) = &node.left {
        if node_partner(l, query, eps) {
            return true;
        }
    }
    if let Some(r) = &node.right {
        if node_partner(r, query, eps) {
            return true;
        }
    }
    false
}

/// Collect into `out` the parameter box of every leaf whose box is within
/// `eps` (sup-distance) of the query box.
pub(crate) fn collect_partners(node: &KdNode, query: &Box3, eps: f64, out: &mut Vec<Interval>) {
    if box_distance(query, &node.bb) > eps {
        return;
    }
    if let Some(cell) = node.cell {
        if sup_distance_box(query, &cell.bb) <= eps {
            out.push(cell.tt);
        }
        return;
    }
    if let Some(l) = &node.left {
        collect_partners(l, query, eps, out);
    }
    if let Some(r) = &node.right {
        collect_partners(r, query, eps, out);
    }
}

/// Whether the query box is at certified distance > eps from EVERY leaf box
/// of the tree (used at the width floor to certify a closeness violation).
pub(crate) fn all_farther(node: &KdNode, query: &Box3, eps: f64) -> bool {
    box_distance(query, &node.bb) > eps
}

/// Condition (i): two-sided eps-closeness by cell pairing with box-to-box
/// distances. Returns the certified pairings (every within-eps cross pair at
/// the final refinement) for condition (ii) to consume. The search is pruned
/// by the mandated balanced tree; any O(N*M) whole-array double loop here is
/// a review reject.
fn closeness_check(
    approx: &impl EnclosureCurve,
    approx_span: (f64, f64),
    exact: &impl EnclosureCurve,
    exact_span: (f64, f64),
    eps: f64,
    budget: &mut Budget,
) -> Result<Vec<(Interval, Interval)>, IsotopyConditionsError> {
    let span1 = interval(approx_span.0, approx_span.1);
    let span2 = interval(exact_span.0, exact_span.1);
    let mut cells1: Vec<Interval> = vec![span1];
    let mut cells2: Vec<Interval> = vec![span2];
    loop {
        let mut subdivided = false;
        let mut pending = false;
        let tree2 = build_curve_tree(&cells2, exact);
        let mut new1 = Vec::with_capacity(cells1.len() * 2);
        for tt in cells1 {
            let bb = approx.enclose(tt);
            if node_partner(&tree2, &bb, eps) {
                new1.push(tt);
            } else if all_farther(&tree2, &bb, eps) {
                // Certified at ANY width: box_distance(cell, whole other
                // curve) > eps is stable under every further subdivision, so
                // the violation fires as soon as it is certifiable.
                return Err(IsotopyConditionsError::ClosenessViolation { witness_cell: tt });
            } else if can_subdivide(tt) {
                budget
                    .spend_subdiv(1)
                    .map_err(|_| IsotopyConditionsError::ClosenessUnresolved)?;
                let (a, b) = split(tt);
                new1.push(a);
                new1.push(b);
                subdivided = true;
            } else {
                pending = true;
                new1.push(tt);
            }
        }
        cells1 = new1;
        let tree1 = build_curve_tree(&cells1, approx);
        let mut new2 = Vec::with_capacity(cells2.len() * 2);
        for tt in cells2 {
            let bb = exact.enclose(tt);
            if node_partner(&tree1, &bb, eps) {
                new2.push(tt);
            } else if all_farther(&tree1, &bb, eps) {
                return Err(IsotopyConditionsError::ClosenessViolation { witness_cell: tt });
            } else if can_subdivide(tt) {
                budget
                    .spend_subdiv(1)
                    .map_err(|_| IsotopyConditionsError::ClosenessUnresolved)?;
                let (a, b) = split(tt);
                new2.push(a);
                new2.push(b);
                subdivided = true;
            } else {
                pending = true;
                new2.push(tt);
            }
        }
        cells2 = new2;
        if !subdivided {
            if pending {
                return Err(IsotopyConditionsError::ClosenessUnresolved);
            }
            break;
        }
    }
    let tree2 = build_curve_tree(&cells2, exact);
    let mut pairs: Vec<(Interval, Interval)> = Vec::new();
    let mut partners: Vec<Interval> = Vec::new();
    for tt in cells1.iter() {
        partners.clear();
        let bb = approx.enclose(*tt);
        collect_partners(&tree2, &bb, eps, &mut partners);
        for p in partners.iter() {
            pairs.push((*tt, *p));
        }
    }
    Ok(pairs)
}

/// Condition (iii): endpoint correspondence at an explicit boundary kind.
/// Kinds must agree; every endpoint point-box of either curve must be within
/// `eps` (sup-distance, degenerate point box) of SOME endpoint point-box of
/// the other; a both-Closed input additionally passes the per-curve seam
/// consistency gate `box_distance(E_lo, E_hi) <= 2*eps`. The seam gate is a
/// consistency gate on the caller's Closed claim — NOT a closedness
/// certificate; closure is never claimed as a topological fact here.
fn endpoint_check(
    exact: &impl EnclosureCurve,
    exact_span: (f64, f64),
    exact_boundary: CurveBoundary,
    approx: &impl EnclosureCurve,
    approx_span: (f64, f64),
    approx_boundary: CurveBoundary,
    eps: f64,
) -> Result<(), IsotopyConditionsError> {
    if exact_boundary != approx_boundary {
        return Err(IsotopyConditionsError::BoundaryMismatch);
    }
    let e_lo = exact.enclose(interval_at(exact_span.0));
    let e_hi = exact.enclose(interval_at(exact_span.1));
    let a_lo = approx.enclose(interval_at(approx_span.0));
    let a_hi = approx.enclose(interval_at(approx_span.1));
    let e_endpoints = [e_lo, e_hi];
    let a_endpoints = [a_lo, a_hi];
    for ep in e_endpoints {
        if !a_endpoints.iter().any(|q| sup_distance_box(&ep, q) <= eps) {
            return Err(IsotopyConditionsError::BoundaryMismatch);
        }
    }
    for ap in a_endpoints {
        if !e_endpoints.iter().any(|q| sup_distance_box(&ap, q) <= eps) {
            return Err(IsotopyConditionsError::BoundaryMismatch);
        }
    }
    if exact_boundary == CurveBoundary::Closed {
        if box_distance(&e_lo, &e_hi) > 2.0 * eps {
            return Err(IsotopyConditionsError::BoundaryMismatch);
        }
        if box_distance(&a_lo, &a_hi) > 2.0 * eps {
            return Err(IsotopyConditionsError::BoundaryMismatch);
        }
    }
    Ok(())
}

/// Condition (ii): the tangent-SPACE angle bound on the certified pairings.
/// For every pairing (A, B) certified in (i), with first-derivative boxes
/// `D'` (approx) and `D` (exact) and `s = eps / tube`, the unoriented
/// tangent-space condition `|cos| > s` is decided in cosine form, both sides
/// sound: `abs_lower(dot)/(norm.sup * norm.sup) > s` passes (denominators
/// only shrink), `abs_upper(dot)/(norm.inf * norm.inf) <= s` is a certified
/// violation (every pair in the boxes fails), and anything strictly between
/// subdivides the pair. A derivative box whose norm infimum is 0 cannot be
/// tested and subdivides; at the floor — `AngleUnresolved`. Condition (ii)
/// consumes EXACTLY the pairs (i) certified; it never scans cells on its own.
fn angle_check(
    pairs: Vec<(Interval, Interval)>,
    approx: &impl EnclosureCurve,
    exact: &impl EnclosureCurve,
    eps: f64,
    tube: f64,
    budget: &mut Budget,
) -> Result<(), IsotopyConditionsError> {
    let s = eps / tube;
    let mut worklist = pairs;
    while let Some((acell, ecell)) = worklist.pop() {
        let d_approx = approx.enclose_der(1, acell);
        let d_exact = exact.enclose_der(1, ecell);
        let na_sup = norm_sup(&d_approx);
        let ne_sup = norm_sup(&d_exact);
        let na_inf = norm_inf(&d_approx);
        let ne_inf = norm_inf(&d_exact);
        if na_inf == 0.0 || ne_inf == 0.0 {
            let Some(children) = subdivide_pair(acell, ecell, budget)? else {
                return Err(IsotopyConditionsError::AngleUnresolved);
            };
            worklist.extend(children);
            continue;
        }
        let dot = dot_box(&d_approx, &d_exact);
        let pass_ratio = abs_lower(dot) / (na_sup * ne_sup);
        if pass_ratio > s {
            continue;
        }
        let fail_ratio = abs_upper(dot) / (na_inf * ne_inf);
        if fail_ratio <= s {
            return Err(IsotopyConditionsError::AngleViolation {
                approx_cell: acell,
                exact_cell: ecell,
            });
        }
        let Some(children) = subdivide_pair(acell, ecell, budget)? else {
            return Err(IsotopyConditionsError::AngleUnresolved);
        };
        worklist.extend(children);
    }
    Ok(())
}

/// Subdivide the wider of a certified pair at its midpoint, spending one
/// subdivision. `Ok(None)` when both cells are at the width floor (nothing
/// left to bisect); `Err` when the budget cannot pay (the AngleUnresolved
/// arm).
fn subdivide_pair(
    acell: Interval,
    ecell: Interval,
    budget: &mut Budget,
) -> Result<Option<Vec<(Interval, Interval)>>, IsotopyConditionsError> {
    let a_floor = !can_subdivide(acell);
    let e_floor = !can_subdivide(ecell);
    if a_floor && e_floor {
        return Ok(None);
    }
    let aw = acell.sup() - acell.inf();
    let ew = ecell.sup() - ecell.inf();
    if !a_floor && (aw >= ew || e_floor) {
        budget
            .spend_subdiv(1)
            .map_err(|_| IsotopyConditionsError::AngleUnresolved)?;
        let (a1, a2) = split(acell);
        Ok(Some(vec![(a1, ecell), (a2, ecell)]))
    } else if !e_floor {
        budget
            .spend_subdiv(1)
            .map_err(|_| IsotopyConditionsError::AngleUnresolved)?;
        let (e1, e2) = split(ecell);
        Ok(Some(vec![(acell, e1), (acell, e2)]))
    } else {
        Ok(None)
    }
}

/// The certified minimum over qualifying cell pairs of the box-to-box
/// distance, or `+inf` when no pair qualifies. Runs over the mandated
/// balanced tree with best-so-far pruning and the parameter-gap precondition
/// pruning; the query cell is itself a leaf of the tree and its self-pair
/// (gap 0) never qualifies.
fn min_separation(
    tree: &KdNode,
    cells: &[Interval],
    curve: &impl EnclosureCurve,
    arc_gap: f64,
    closed: bool,
    span: f64,
) -> f64 {
    let mut best = f64::INFINITY;
    for tt in cells {
        let bb = curve.enclose(*tt);
        descend_separation(tree, tt, &bb, arc_gap, closed, span, &mut best);
    }
    best
}

/// One query cell's descent of the separation tree: prune nodes whose
/// box-to-box distance to the query cannot lower `best`, prune nodes from
/// which no leaf can satisfy the parameter-gap precondition, and update
/// `best` at qualifying leaves.
fn descend_separation(
    node: &KdNode,
    query_tt: &Interval,
    query_bb: &Box3,
    arc_gap: f64,
    closed: bool,
    span: f64,
    best: &mut f64,
) {
    if box_distance(query_bb, &node.bb) >= *best {
        return;
    }
    if param_gap_max(query_tt, node.param_lo, node.param_hi, closed, span) < arc_gap {
        return;
    }
    if let Some(cell) = node.cell {
        if param_gap_max(query_tt, cell.tt.inf(), cell.tt.sup(), closed, span) >= arc_gap {
            let d = box_distance(query_bb, &cell.bb);
            if d < *best {
                *best = d;
            }
        }
        return;
    }
    if let Some(l) = &node.left {
        descend_separation(l, query_tt, query_bb, arc_gap, closed, span, best);
    }
    if let Some(r) = &node.right {
        descend_separation(r, query_tt, query_bb, arc_gap, closed, span, best);
    }
}

/// The certified maximum parameter gap between a cell and a parameter range,
/// used both as the leaf qualifying test and the pruning precondition
/// (`max_gap < arc_gap` rules the whole node out). A cell pair QUALIFIES when
/// it contains at least one parameter pair at gap >= arc_gap; the pair
/// containing the true minimizer then always qualifies and its box-to-box
/// distance is a certified lower bound on the true value, which is what makes
/// the minimum over qualifying pairs sound. For Closed, `d = |s-t|` ranges
/// over the ordinary gap interval and the wrapped value `min(d, span-d)` peaks
/// at `span/2`.
fn param_gap_max(a: &Interval, b_lo: f64, b_hi: f64, closed: bool, span: f64) -> f64 {
    let (a_lo, a_hi) = (a.inf(), a.sup());
    let lo = (b_lo - a_hi).max(a_lo - b_hi).max(0.0);
    let hi = (b_hi - a_lo).max(a_hi - b_lo);
    if !closed {
        return hi;
    }
    if lo <= 0.5 * span && 0.5 * span <= hi {
        0.5 * span
    } else if hi < 0.5 * span {
        hi
    } else {
        span - lo
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
    use std::ops::Bound;
    use truck_base::cgmath64::{EuclideanSpace, Point3, Vector3};
    use truck_geotrait::{ParameterRange, ParametricCurve};

    /// Exact circle radius, model units.
    const RADIUS: f64 = 2.0; // H-3: exact circle radius in model units, the witness length scale
    /// The eps every condition is certified against.
    const EPS: f64 = 0.05; // H-3: the margin, a model-space length relative to RADIUS
    /// The full-circle parameter span `[0, 2π]`.
    const FULL_SPAN: f64 = core::f64::consts::TAU; // H-3: the full circle span in radians, dimensionless
    /// The double-cover parameter span `[0, 4π]`.
    const DOUBLE_SPAN: f64 = 2.0 * core::f64::consts::TAU; // H-3: the double-cover span in radians, dimensionless
    /// The half-offset approximant's radius `R + eps/2`.
    const SINGLE_SHEET_RADIUS: f64 = RADIUS + 0.5 * EPS; // H-3: single-sheet radius, a model-space length
    /// The radial sinusoid's amplitude `a <= eps`.
    const SINUSOID_AMPLITUDE: f64 = 0.04; // H-3: sinusoid amplitude, a model-space length strictly below EPS
    /// The radial sinusoid's frequency in radians.
    const SINUSOID_OMEGA: f64 = 4000.0; // H-3: sinusoid angular frequency, a dimensionless oscillation rate
    /// The double cover's amplitude `a < eps` so condition (i) can certify.
    const DOUBLE_COVER_AMPLITUDE: f64 = 0.5 * EPS; // H-3: double-cover amplitude, a model-space length strictly below EPS
    /// The coarse-radius circle's radius, below 2*eps.
    const COARSE_RADIUS: f64 = 0.08; // H-3: coarse radius in model units, below the 2*eps tube budget
    /// The half-offset between the two parallel lines of test 8.
    const LINE_OFFSET: f64 = 0.5 * EPS; // H-3: line offset, a model-space length strictly below EPS
    /// The open segment span (parameter length) of the line fixtures.
    const LINE_SPAN: f64 = 1.0; // H-3: line parameter span, a dimensionless parameter length
    /// The trimmed segment's start parameter.
    const TRIMMED_LO: f64 = 0.1; // H-3: trimmed start parameter, dimensionless
    /// The near-full-circle gap: `[0, 2π - CLOSED_GAP]` still passes every
    /// geometric endpoint check while being Open.
    const CLOSED_GAP: f64 = 0.001; // H-3: angular gap in radians, dimensionless
    /// The hairpin's hand-built curvature component (gentle turnaround).
    const HAIRPIN_CURVATURE: f64 = 10.0; // H-3: hairpin curvature radius in model units
    /// The hairpin's hand-built separation component (tight strand gap).
    const HAIRPIN_SEPARATION: f64 = 0.12; // H-3: hairpin strand gap in model units
    /// The ellipse's semi-major axis.
    const ELLIPSE_A: f64 = 2.0; // H-3: ellipse semi-major axis in model units
    /// The ellipse's semi-minor axis.
    const ELLIPSE_B: f64 = 0.5; // H-3: ellipse semi-minor axis in model units
    /// The wrapped parameter gap at which the ellipse's minimum is attained.
    const ELLIPSE_ARC_GAP: f64 = 2.0; // H-3: ellipse wrapped parameter gap in radians, dimensionless
    /// The brute-force reference (4000x4000 wrapped grid) the certified
    /// value must not exceed.
    const ELLIPSE_REFERENCE: f64 = 0.84179354; // H-3: the ellipse reference distance in model units
    /// The usefulness floor: a certificate below this is vacuous.
    const ELLIPSE_USEFUL_FLOOR: f64 = 0.75; // H-3: the ellipse usefulness floor in model units
    /// Tolerance for the circle's certified curvature radius near R.
    const CURV_SLACK: f64 = 0.05; // H-3: slack on a curvature radius in model units
    /// Tolerance for the circle's certified self-separation near 2R.
    const SEP_SLACK: f64 = 0.05; // H-3: slack on a self-separation in model units
    /// The exact circle's self-separation at parameter gap pi.
    const TRUE_SELF_SEPARATION: f64 = 2.0 * RADIUS; // H-3: the true antipodal distance in model units
    /// Subdivision budget for the span-helper certificates.
    const HELPER_BUDGET_SUBDIV: u32 = 1 << 18; // H-3: subdivision budget count, dimensionless
    /// Subdivision budget for a full conditions check.
    const MAIN_BUDGET_SUBDIV: u32 = 1 << 20; // H-3: subdivision budget count, dimensionless

    /// Test-only unwrap that stays under the crate's deny list: unit tests
    /// assert on hand-built witnesses, so a refusal here is a test bug.
    fn must<T>(r: Result<T, IsotopyConditionsError>) -> T {
        match r {
            Ok(value) => value,
            Err(_) => unreachable!("unit-test witness must certify"),
        }
    }

    /// Compare a certified float against a target within `slack`; equal
    /// infinities pass.
    fn assert_close(a: f64, b: f64, slack: f64, what: &str) {
        if a.is_infinite() {
            assert!(b.is_infinite(), "{what}: {a} vs {b}");
        } else {
            assert!(
                (a - b).abs() <= slack,
                "{what}: bound {a} not within {slack} of {b}"
            );
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

    /// The radial sinusoid approximant `(R + a*sin(omega*t))*e(t)`: it stays
    /// within `a <= eps` of the exact circle but swings its tangent by up to
    /// `atan(a*omega/R)` — the motivating (ii) failure.
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
            let mut acc = Vector3::new(0.0, 0.0, 0.0);
            let mut binom = 1.0_f64;
            for k in 0..=n {
                let rad_k = match k {
                    0 => self.r + self.a * (self.omega * t).sin(),
                    1 => self.a * self.omega * (self.omega * t).cos(),
                    2 => -self.a * self.omega * self.omega * (self.omega * t).sin(),
                    3 => -self.a * self.omega * self.omega * self.omega * (self.omega * t).cos(),
                    _ => {
                        self.a
                            * self.omega.powi(k as i32)
                            * (self.omega * t + (k as f64) * core::f64::consts::FRAC_PI_2).sin()
                    }
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
                    // r^(k)(t) = a * omega^k * sin(omega*t + k*pi/2).
                    let shift = (k as f64) * core::f64::consts::FRAC_PI_2;
                    interval_at(self.a)
                        * interval_at(self.omega.powi(k as i32))
                        * sin(wtt + interval_at(shift))
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

    /// The double-cover approximant `(R + a*cos(t/2))*e(t)` over `[0, 4π]`:
    /// the canonical 2-to-1 fibre witness, with `a < eps` so condition (i)
    /// can certify.
    #[derive(Clone)]
    struct DoubleCover {
        r: f64,
        a: f64,
        lo: f64,
        hi: f64,
    }

    impl ParametricCurve for DoubleCover {
        type Point = Point3;
        type Vector = Vector3;

        fn subs(&self, t: f64) -> Point3 {
            let rad = self.r + self.a * (t / 2.0).cos();
            Point3::new(rad * t.cos(), rad * t.sin(), 0.0)
        }

        fn der(&self, t: f64) -> Vector3 {
            let rad = self.r + self.a * (t / 2.0).cos();
            let drad = -0.5 * self.a * (t / 2.0).sin();
            Vector3::new(
                drad * t.cos() - rad * t.sin(),
                drad * t.sin() + rad * t.cos(),
                0.0,
            )
        }

        fn der2(&self, t: f64) -> Vector3 {
            let rad = self.r + self.a * (t / 2.0).cos();
            let drad = -0.5 * self.a * (t / 2.0).sin();
            let d2rad = -0.25 * self.a * (t / 2.0).cos();
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
                    self.r + self.a * (t / 2.0).cos()
                } else {
                    self.a
                        * 0.5_f64.powi(k as i32)
                        * (t / 2.0 + (k as f64) * core::f64::consts::FRAC_PI_2).cos()
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

    impl EnclosureCurve for DoubleCover {
        fn enclose(&self, tt: Interval) -> Box3 {
            let rad = interval_at(self.r) + interval_at(self.a) * cos(tt / interval_at(2.0));
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
            let half = interval_at(2.0);
            let mut x = interval_at(0.0);
            let mut y = interval_at(0.0);
            let mut binom = 1.0_f64;
            for k in 0..=n {
                let rad_k = if k == 0 {
                    interval_at(self.r) + interval_at(self.a) * cos(tt / half)
                } else {
                    interval_at(self.a)
                        * interval_at(0.5_f64.powi(k as i32))
                        * cos(tt / half + interval_at((k as f64) * core::f64::consts::FRAC_PI_2))
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

    /// A straight segment `(t, y, 0)` over `[lo, hi]`.
    #[derive(Clone)]
    struct Line {
        y: f64,
        lo: f64,
        hi: f64,
    }

    impl ParametricCurve for Line {
        type Point = Point3;
        type Vector = Vector3;

        fn subs(&self, t: f64) -> Point3 {
            Point3::new(t, self.y, 0.0)
        }

        fn der(&self, _t: f64) -> Vector3 {
            Vector3::new(1.0, 0.0, 0.0)
        }

        fn der2(&self, _t: f64) -> Vector3 {
            Vector3::new(0.0, 0.0, 0.0)
        }

        fn der_n(&self, n: usize, _t: f64) -> Vector3 {
            match n {
                0 => self.subs(_t).to_vec(),
                1 => Vector3::new(1.0, 0.0, 0.0),
                _ => Vector3::new(0.0, 0.0, 0.0),
            }
        }

        fn parameter_range(&self) -> ParameterRange {
            (Bound::Included(self.lo), Bound::Included(self.hi))
        }
    }

    impl EnclosureCurve for Line {
        fn enclose(&self, tt: Interval) -> Box3 {
            Box3 {
                x: tt,
                y: interval_at(self.y),
                z: interval_at(0.0),
            }
        }

        fn enclose_der(&self, n: usize, tt: Interval) -> Box3 {
            if n == 0 {
                return self.enclose(tt);
            }
            Box3 {
                x: interval_at(if n == 1 { 1.0 } else { 0.0 }),
                y: interval_at(0.0),
                z: interval_at(0.0),
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

    /// The exact circle for every conditions test.
    fn exact_circle() -> Circle {
        Circle {
            r: RADIUS,
            lo: 0.0,
            hi: FULL_SPAN,
        }
    }

    /// The single-sheet approximant circle of test 1.
    fn single_sheet_circle() -> Circle {
        Circle {
            r: SINGLE_SHEET_RADIUS,
            lo: 0.0,
            hi: FULL_SPAN,
        }
    }

    /// The whole-span scale components of the exact circle, built from the
    /// two certified span helpers at parameter gap `pi`.
    fn circle_scale() -> CurveScaleComponents {
        let exact = exact_circle();
        let mut cb = Budget::new(HELPER_BUDGET_SUBDIV, 0, 0);
        let curvature = must(curvature_radius_lower_span(&exact, &mut cb));
        let mut sb = Budget::new(HELPER_BUDGET_SUBDIV, 0, 0);
        let separation = must(self_separation_lower_span(
            &exact,
            CurveBoundary::Closed,
            core::f64::consts::PI,
            &mut sb,
        ));
        CurveScaleComponents {
            curvature_radius_lower: curvature,
            self_separation_lower: separation,
        }
    }

    #[test]
    fn single_sheet_circle_conditions_hold() {
        let exact = exact_circle();
        let approx = single_sheet_circle();
        let scale = circle_scale();
        assert_close(
            scale.curvature_radius_lower,
            RADIUS,
            CURV_SLACK,
            "curvature radius near R",
        );
        assert_close(
            scale.self_separation_lower,
            TRUE_SELF_SEPARATION,
            SEP_SLACK,
            "self-separation near 2R",
        );
        let mut budget = Budget::new(MAIN_BUDGET_SUBDIV, 0, 0);
        let report = must(curve_isotopy_conditions(
            &exact,
            CurveBoundary::Closed,
            &approx,
            CurveBoundary::Closed,
            EPS,
            &scale,
            &mut budget,
        ));
        assert_eq!(report.eps, EPS);
        assert_eq!(report.scale, scale);
    }

    #[test]
    fn radial_sinusoid_fails_angle_condition() {
        let exact = exact_circle();
        let approx = RadialSinusoid {
            r: RADIUS,
            a: SINUSOID_AMPLITUDE,
            omega: SINUSOID_OMEGA,
            lo: 0.0,
            hi: FULL_SPAN,
        };
        let scale = circle_scale();
        let mut budget = Budget::new(MAIN_BUDGET_SUBDIV, 0, 0);
        let out = curve_isotopy_conditions(
            &exact,
            CurveBoundary::Closed,
            &approx,
            CurveBoundary::Closed,
            EPS,
            &scale,
            &mut budget,
        );
        assert!(
            matches!(out, Err(IsotopyConditionsError::AngleViolation { .. })),
            "the radial sinusoid must fail condition (ii), got {out:?}"
        );
    }

    #[test]
    fn double_cover_is_multisheet() {
        let exact = exact_circle();
        let approx = DoubleCover {
            r: RADIUS,
            a: DOUBLE_COVER_AMPLITUDE,
            lo: 0.0,
            hi: DOUBLE_SPAN,
        };
        let scale = circle_scale();
        let mut budget = Budget::new(MAIN_BUDGET_SUBDIV, 0, 0);
        let out = curve_isotopy_conditions(
            &exact,
            CurveBoundary::Closed,
            &approx,
            CurveBoundary::Closed,
            EPS,
            &scale,
            &mut budget,
        );
        assert!(
            matches!(out, Err(IsotopyConditionsError::MultiSheet { count: 2 })),
            "the double cover must certify a two-sheet fibre, got {out:?}"
        );
    }

    #[test]
    fn trimmed_approx_boundary_mismatch() {
        let exact = Line {
            y: 0.0,
            lo: 0.0,
            hi: LINE_SPAN,
        };
        let approx = Line {
            y: 0.0,
            lo: TRIMMED_LO,
            hi: LINE_SPAN,
        };
        let mut cb = Budget::new(HELPER_BUDGET_SUBDIV, 0, 0);
        let curvature = must(curvature_radius_lower_span(&exact, &mut cb));
        let mut sb = Budget::new(HELPER_BUDGET_SUBDIV, 0, 0);
        let separation = must(self_separation_lower_span(
            &exact,
            CurveBoundary::Open,
            LINE_SPAN,
            &mut sb,
        ));
        let scale = CurveScaleComponents {
            curvature_radius_lower: curvature,
            self_separation_lower: separation,
        };
        let mut budget = Budget::new(MAIN_BUDGET_SUBDIV, 0, 0);
        let out = curve_isotopy_conditions(
            &exact,
            CurveBoundary::Open,
            &approx,
            CurveBoundary::Open,
            EPS,
            &scale,
            &mut budget,
        );
        assert!(
            matches!(out, Err(IsotopyConditionsError::ClosenessViolation { .. })),
            "the trimmed approx's displaced tail is a two-sided closeness \
             violation (condition (i)) before (iii) is reached, got {out:?}"
        );
    }

    #[test]
    fn coarse_radius_refuses() {
        let exact = Circle {
            r: COARSE_RADIUS,
            lo: 0.0,
            hi: FULL_SPAN,
        };
        let approx = Circle {
            r: COARSE_RADIUS + 0.5 * EPS,
            lo: 0.0,
            hi: FULL_SPAN,
        };
        let scale = circle_scale_for(&exact);
        let mut budget = Budget::new(MAIN_BUDGET_SUBDIV, 0, 0);
        let out = curve_isotopy_conditions(
            &exact,
            CurveBoundary::Closed,
            &approx,
            CurveBoundary::Closed,
            EPS,
            &scale,
            &mut budget,
        );
        assert!(
            matches!(out, Err(IsotopyConditionsError::ReachLowerBoundTooSmall)),
            "2*eps >= tube_scale_lower must refuse, got {out:?}"
        );
    }

    /// The whole-span scale components of an arbitrary circle, built from the
    /// two certified span helpers at parameter gap `pi`.
    fn circle_scale_for(exact: &Circle) -> CurveScaleComponents {
        let mut cb = Budget::new(HELPER_BUDGET_SUBDIV, 0, 0);
        let curvature = must(curvature_radius_lower_span(exact, &mut cb));
        let mut sb = Budget::new(HELPER_BUDGET_SUBDIV, 0, 0);
        let separation = must(self_separation_lower_span(
            exact,
            CurveBoundary::Closed,
            core::f64::consts::PI,
            &mut sb,
        ));
        CurveScaleComponents {
            curvature_radius_lower: curvature,
            self_separation_lower: separation,
        }
    }

    #[test]
    fn invalid_margin_refuses() {
        let exact = exact_circle();
        let approx = single_sheet_circle();
        let scale = circle_scale();
        for bad_eps in [0.0, -EPS, f64::NAN, f64::INFINITY] {
            let mut budget = Budget::new(MAIN_BUDGET_SUBDIV, 0, 0);
            let out = curve_isotopy_conditions(
                &exact,
                CurveBoundary::Closed,
                &approx,
                CurveBoundary::Closed,
                bad_eps,
                &scale,
                &mut budget,
            );
            assert!(
                matches!(out, Err(IsotopyConditionsError::InvalidMargin)),
                "eps = {bad_eps} must refuse as InvalidMargin"
            );
        }
        for bad_gap in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            let mut budget = Budget::new(HELPER_BUDGET_SUBDIV, 0, 0);
            let out =
                self_separation_lower_span(&exact, CurveBoundary::Closed, bad_gap, &mut budget);
            assert!(
                matches!(out, Err(IsotopyConditionsError::InvalidMargin)),
                "arc_gap = {bad_gap} must refuse as InvalidMargin"
            );
        }
    }

    #[test]
    fn zero_budget_refuses_unresolved() {
        let exact = exact_circle();
        let approx = single_sheet_circle();
        let scale = circle_scale();
        let mut budget = Budget::new(0, 0, 0);
        let out = curve_isotopy_conditions(
            &exact,
            CurveBoundary::Closed,
            &approx,
            CurveBoundary::Closed,
            EPS,
            &scale,
            &mut budget,
        );
        assert!(
            matches!(
                out,
                Err(IsotopyConditionsError::ClosenessUnresolved)
                    | Err(IsotopyConditionsError::AngleUnresolved)
                    | Err(IsotopyConditionsError::DegreeOneUnresolved)
            ),
            "a zero budget must refuse as an *Unresolved arm, got {out:?}"
        );
    }

    #[test]
    fn line_pair_conditions_hold() {
        let exact = Line {
            y: 0.0,
            lo: 0.0,
            hi: LINE_SPAN,
        };
        let approx = Line {
            y: LINE_OFFSET,
            lo: 0.0,
            hi: LINE_SPAN,
        };
        let mut cb = Budget::new(HELPER_BUDGET_SUBDIV, 0, 0);
        let curvature = must(curvature_radius_lower_span(&exact, &mut cb));
        let mut sb = Budget::new(HELPER_BUDGET_SUBDIV, 0, 0);
        let separation = must(self_separation_lower_span(
            &exact,
            CurveBoundary::Open,
            LINE_SPAN,
            &mut sb,
        ));
        assert_eq!(curvature, f64::INFINITY);
        assert_eq!(separation, f64::INFINITY);
        let scale = CurveScaleComponents {
            curvature_radius_lower: curvature,
            self_separation_lower: separation,
        };
        let mut budget = Budget::new(MAIN_BUDGET_SUBDIV, 0, 0);
        let report = must(curve_isotopy_conditions(
            &exact,
            CurveBoundary::Open,
            &approx,
            CurveBoundary::Open,
            EPS,
            &scale,
            &mut budget,
        ));
        assert_eq!(report.eps, EPS);
        assert_eq!(report.scale, scale);
    }

    #[test]
    fn reversed_parameterization_matches_forward() {
        let exact = exact_circle();
        let approx = RevCircle {
            r: SINGLE_SHEET_RADIUS,
            lo: 0.0,
            hi: FULL_SPAN,
        };
        let scale = circle_scale();
        let mut budget = Budget::new(MAIN_BUDGET_SUBDIV, 0, 0);
        let report = must(curve_isotopy_conditions(
            &exact,
            CurveBoundary::Closed,
            &approx,
            CurveBoundary::Closed,
            EPS,
            &scale,
            &mut budget,
        ));
        assert_eq!(report.eps, EPS);
        assert_eq!(report.scale, scale);
    }

    #[test]
    fn closed_exact_open_approx_mismatches() {
        let exact = exact_circle();
        let approx = Circle {
            r: RADIUS,
            lo: 0.0,
            hi: FULL_SPAN - CLOSED_GAP,
        };
        let scale = circle_scale();
        let mut budget = Budget::new(MAIN_BUDGET_SUBDIV, 0, 0);
        let out = curve_isotopy_conditions(
            &exact,
            CurveBoundary::Closed,
            &approx,
            CurveBoundary::Open,
            EPS,
            &scale,
            &mut budget,
        );
        assert!(
            matches!(out, Err(IsotopyConditionsError::BoundaryMismatch)),
            "a closed exact with an open approx is circle-vs-interval, got {out:?}"
        );
    }

    #[test]
    fn hairpin_scale_refuses_on_separation() {
        let exact = exact_circle();
        let approx = single_sheet_circle();
        let scale = CurveScaleComponents {
            curvature_radius_lower: HAIRPIN_CURVATURE,
            self_separation_lower: HAIRPIN_SEPARATION,
        };
        assert_eq!(scale.tube_scale_lower(), 0.06);
        assert!(2.0 * EPS >= scale.tube_scale_lower());
        let mut budget = Budget::new(MAIN_BUDGET_SUBDIV, 0, 0);
        let out = curve_isotopy_conditions(
            &exact,
            CurveBoundary::Closed,
            &approx,
            CurveBoundary::Closed,
            EPS,
            &scale,
            &mut budget,
        );
        assert!(
            matches!(out, Err(IsotopyConditionsError::ReachLowerBoundTooSmall)),
            "the composed tube bound (min 10, 0.06) must refuse, got {out:?}"
        );
    }

    #[test]
    fn ellipse_separation_soundness() {
        let ellipse = Ellipse {
            a: ELLIPSE_A,
            b: ELLIPSE_B,
            lo: 0.0,
            hi: FULL_SPAN,
        };
        let mut budget = Budget::new(HELPER_BUDGET_SUBDIV, 0, 0);
        let sep = must(self_separation_lower_span(
            &ellipse,
            CurveBoundary::Closed,
            ELLIPSE_ARC_GAP,
            &mut budget,
        ));
        assert!(
            sep <= ELLIPSE_REFERENCE,
            "certified separation {sep} exceeds the brute-force reference {ELLIPSE_REFERENCE}"
        );
        assert!(
            sep >= ELLIPSE_USEFUL_FLOOR,
            "certified separation {sep} below the usefulness floor {ELLIPSE_USEFUL_FLOOR}"
        );
    }
}
