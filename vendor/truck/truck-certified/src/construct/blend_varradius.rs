#![cfg_attr(not(debug_assertions), deny(warnings))]
#![deny(clippy::all, rust_2018_idioms)]
#![deny(clippy::unwrap_used)]
#![warn(
    missing_docs,
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]

//! CC-031-BLEND-VARRADIUS (spine S10 consumer; theory §5.3): the variable
//! rolling-ball blend trace closed by the FOOT-POINT pair instead of the
//! constant-radius closure Φ.
//!
//! # The foot-point closure (Section 1, realized here)
//!
//! A variable radius adds a certified guide curve `G` and an admissible radius
//! law `R`. For a ball centre `c` the guide foot parameter `λ` is fixed by the
//! foot-point equation `(c − G(λ))·G′(λ) = 0` and the radius is then
//! `r = R(λ)`. The λ-derivative of the foot residual is
//! `∂_λ[(c − G)·G′] = −‖G′‖² + (c − G)·G″`; over a compact guide sub-region
//! where the certified upper bound of that derivative is strictly negative the
//! foot point is UNIQUE on the region — one scalar unknown and two equations
//! (the foot-point pair) replace the constant-radius closure Φ, so the
//! corrector dimensionality is IDENTICAL to the CC-030 constant-radius case
//! (arity-3 over the certified centre box). No nearest-point projection and no
//! tubular-radius bottleneck computation appear anywhere.
//!
//! [`foot_point_gate`] certifies that bound from the landed map margins:
//! `‖G′‖` through `CertifiedCurveMap::rank_margin` is not needed verbatim —
//! the derivative hull path (the CC-002 discipline) bounds `G′` and `G″`
//! per coordinate — and the ball-centre enclosure enters through the interval
//! centre argument. A gate that cannot certify the strictly-negative
//! derivative refuses [`ConstructRefusal::ConditioningBelowThreshold`]: the
//! foot point is not locally unique on that region. The GLOBAL branch — a
//! distant part of `G` passing near the centre — is not the gate's job: the
//! certified walk always runs the P5 clearance predicate (`ball_clearance`
//! through the C2 manifest edge) and stays on the certified local branch.
//!
//! # The amended walk (Section 2, realized here)
//!
//! [`trace_blend_chain_variable`] is the CC-030 walk with the system closed by
//! the foot-point pair. Each certified continuation step targets a certified
//! foot parameter `λ` on the guide (isolated by a certified monotone scalar
//! root), takes the certified radius point `R(λ)` through the landed
//! `radius_eval` evaluator (CC-025, seam S10), and certifies the branch centre
//! as the arity-3 root of the two support-offset rows at that radius plus the
//! foot row at `λ`. A CONSTANT radius law makes the foot-point pair degenerate
//! and the amended walk reduces EXACTLY to the CC-030 system:
//! [`trace_blend_chain_variable`] then delegates to [`trace_blend_chain`] so
//! both walks produce identical event records (the dimensionality claim is
//! made observable by the conformance test).
//!
//! Every accepted step is certified against the isolation battery: both
//! contact feet strictly interior to their regions, the ball P5-clear of every
//! declared excluded boundary (run regardless of the guide), the step
//! Jacobian submersion above the normative floor, and the radius law
//! admissible (strictly positive) over the step's certified foot region. An
//! undecided step (an uncertifiable corrector box, a foot isolation that
//! cannot certify, or budget exhaustion) refuses the
//! `ConditioningBelowThreshold` family exactly as CC-030 does.
//!
//! # Scope guards (stop conditions)
//!
//! (1) The deliverable ends at the certified event record: the network
//! optimizer that chooses all radii simultaneously is OUT OF SCOPE by theory
//! §5.3 — the kernel answers whether the REQUESTED law certifies, never
//! invents one. (2) [`BlendTrace`] and [`EventKind`] are frozen: this module
//! consumes them and extends nothing. (3) Every admissible v1 radius law is
//! evaluated only through the landed polynomial `radius_eval` evaluator, so
//! the foot-point system stays polynomial and the admissible law list stays
//! closed (no `QUESTION.md` is needed).
//!
//! **H-1.** This module carries no `unwrap`, no `expect`, and no `panic!`, and
//! adds no module-level `allow`. Every float reduction runs in a fixed order
//! with directed rounding (C9), and every [`Interval`] is the C3 universe
//! (`construct::Interval`), never a second interval type.

use crate::certified_map::{CertifiedCurveMap, CertifiedSurfaceMap, CurveRegion, SurfaceRegion};
use crate::construct::blend::{
    trace_blend_chain, BlendEvent, BlendTrace, BranchSeed, ClearanceBoundary, SupportChart,
    WalkState,
};
use crate::construct::canal::radius_eval;
use crate::construct::config::{CC_DEPTH_MAX, CC_ETA_J, CC_MU_CLEAR};
use crate::construct::refusal::ConstructRefusal;
use crate::construct::stubs::{EventKind, RadiusLaw};
use crate::construct::Interval;
use crate::hull::{bernstein_derivative_1d, bernstein_derivative_2d, hull_bernstein_1d};
use crate::kernel::engine::{krawczyk_c1_n3, SquareResidualEval};
use crate::kernel::evidence::ClaimVerdict;
use crate::kernel::patch::{CertifiedPositive, IBox3};
use truck_base::cgmath64::Point3;
use truck_base::evidence::Budget;
use truck_evidence::clear::{ball_clearance, BallAdmissibility};
use truck_evidence::enclosure::{Box3, Interval as InariInterval};
use truck_geometry::specifieds::Plane;

/// The nominal half-span of the certified foot-search window on the guide,
/// measured in guide source parameter units.
///
/// Each continuation step targets the guide foot of the predicted centre; the
/// search window is this radius around the previous certified foot, wide
/// enough for a full predictor arc step plus the certified centre-box slack.
const FOOT_WINDOW_RADIUS: f64 = 0.08;

/// The certified half-width of the event-root refinement enclosure.
const EVENT_HALF: f64 = 0.004;

/// The certified half-width of a continuation centre box along every axis.
const CERTIFIED_HALF: f64 = 0.02;

/// The search slack added beyond a rejected predictor step when locating the
/// certified event root at a certified-step boundary.
const EVENT_SLACK: f64 = 0.08;

/// The nominal arc step of the predictor between certified steps.
const ARC_STEP: f64 = 0.04;

/// The tolerance of the certified monotone foot bisection, in guide units.
const FOOT_BISECT_TOL: f64 = 1e-6;

/// The foot-point uniqueness gate (Section 1).
///
/// Certifies the strictly-negative λ-derivative of the foot-point residual
/// `(c − G(λ))·G′(λ)` over the compact guide sub-region `sub`, using the
/// ball-centre enclosure `c` and the CC-002 Bernstein 1-D hull path for `G′`
/// and `G″`. Returns the certified enclosure of the derivative over `sub` when
/// its upper endpoint is strictly negative (the foot point is locally unique
/// on the region); a gate that cannot certify strict negativity — the
/// curvature product `‖c − G‖·κ_G` reaches one — refuses
/// [`ConstructRefusal::ConditioningBelowThreshold`].
pub fn foot_point_gate(
    map: &CertifiedCurveMap,
    c: &[Interval; 3],
    sub: CurveRegion,
) -> Result<Interval, ConstructRefusal> {
    if !sub.0.is_finite() || !sub.1.is_finite() || sub.0 >= sub.1 {
        return Err(ConstructRefusal::InvalidInput);
    }
    let derivative = foot_derivative(map, c, sub)?;
    if derivative.is_finite() && derivative.hi < 0.0 {
        Ok(derivative)
    } else {
        Err(ConstructRefusal::ConditioningBelowThreshold)
    }
}

/// One accepted certified step of the variable-radius walk.
#[derive(Debug, Clone, PartialEq)]
pub struct VariableStep {
    /// The certified centre box of the step (the box the corrector certified).
    pub centre: IBox3,
    /// The certified enclosure of the step's position along the branch arc
    /// (measured from the certified seed centre).
    pub arc: Interval,
    /// The discrete state `Σ` the step carries.
    pub state: WalkState,
    /// The certified foot region of the step's centre on the guide (guide
    /// source parameter units).
    pub foot: Interval,
    /// The certified radius of the step over its certified foot region, from
    /// the landed radius evaluators.
    pub radius: Interval,
}

/// The per-branch certified variable-radius walk: the accepted certified steps
/// and the events the walk terminated at.
#[derive(Debug, Clone, PartialEq)]
pub struct VariableBranch {
    /// The certified events of this branch, in walk order.
    pub events: Vec<BlendEvent>,
    /// The accepted certified steps between the events, in walk order.
    pub steps: Vec<VariableStep>,
}

/// A named refusal of a per-branch variable-radius walk.
///
/// The refusal carries the [`VariableBranch`] recorded up to the refusing
/// step, so an undecided step is observable as a stop EXACTLY at the last
/// certified step — never as a guessed continuation.
#[derive(Debug, Clone, PartialEq)]
pub struct VariableBranchRefusal {
    /// The construct-layer refusal of the walk.
    pub refusal: ConstructRefusal,
    /// The certified steps and events recorded before the refusal.
    pub partial: VariableBranch,
}

/// Trace a variable-radius blend chain (Section 2, the amended walk).
///
/// Each [`BranchSeed`] is walked under the guide curve `guide` and radius law
/// `law`. When `law` is constant over its arc the foot-point pair is
/// degenerate and the system reduces EXACTLY to the CC-030 system: the walk
/// delegates to [`trace_blend_chain`] and returns identical event records. For
/// a genuinely variable law each branch is walked with the foot-point closure
/// and the certified events of every branch are returned in walk order. Any
/// branch that cannot complete its walk to a certified event refuses with the
/// underlying refusal family.
pub fn trace_blend_chain_variable(
    branches: &[BranchSeed],
    guide: &CertifiedCurveMap,
    law: &RadiusLaw,
    budget: &mut Budget,
) -> Result<BlendTrace, ConstructRefusal> {
    if is_constant_law(law)? {
        return trace_blend_chain(branches, law, budget);
    }
    let mut events: Vec<BlendEvent> = Vec::new();
    for seed in branches {
        let branch =
            trace_branch_steps_variable(seed, guide, law, budget, None).map_err(|r| r.refusal)?;
        events.extend(branch.events);
    }
    Ok(BlendTrace { events })
}

/// Trace one branch of the variable-radius walk (the per-branch amended walk).
///
/// `boundary` is the optional excluded boundary the rolling ball must stay
/// clear of; the P5 clearance predicate is run on every accepted step
/// regardless of the guide curve, so a distant part of `G` that would place
/// the ball across the boundary is certified excluded rather than walked.
pub fn trace_branch_steps_variable(
    seed: &BranchSeed,
    guide: &CertifiedCurveMap,
    law: &RadiusLaw,
    budget: &mut Budget,
    boundary: Option<ClearanceBoundary>,
) -> Result<VariableBranch, VariableBranchRefusal> {
    let empty = VariableBranch {
        events: Vec::new(),
        steps: Vec::new(),
    };
    let chart = match VarChart::try_new(seed, boundary) {
        Ok(chart) => chart,
        Err(refusal) => {
            return Err(VariableBranchRefusal {
                refusal,
                partial: empty.clone(),
            })
        }
    };
    let seed_step = match certify_seed_step(&chart, guide, law, budget) {
        Ok(step) => step,
        Err(refusal) => {
            return Err(VariableBranchRefusal {
                refusal,
                partial: empty.clone(),
            })
        }
    };
    let mut steps: Vec<VariableStep> = vec![seed_step.public_step(&chart.seed_origin, &chart.dir)];
    let mut events: Vec<BlendEvent> = Vec::new();
    if let Err(refusal) = walk_direction(
        &chart,
        guide,
        law,
        &seed_step,
        1.0,
        budget,
        &mut steps,
        &mut events,
    ) {
        return Err(VariableBranchRefusal {
            refusal,
            partial: VariableBranch {
                events: events.clone(),
                steps: steps.clone(),
            },
        });
    }
    if let Err(refusal) = walk_direction(
        &chart,
        guide,
        law,
        &seed_step,
        -1.0,
        budget,
        &mut steps,
        &mut events,
    ) {
        return Err(VariableBranchRefusal {
            refusal,
            partial: VariableBranch {
                events: events.clone(),
                steps: steps.clone(),
            },
        });
    }
    Ok(VariableBranch { events, steps })
}

/// Whether the radius law is constant over its whole normalized arc.
///
/// A law is constant when its certified radius value over `[0, 1]` is
/// degenerate — the exact gate CC-030's own chart build applies — so the
/// delegation in [`trace_blend_chain_variable`] is exactly the CC-030 system.
fn is_constant_law(law: &RadiusLaw) -> Result<bool, ConstructRefusal> {
    let whole = Interval { lo: 0.0, hi: 1.0 };
    let radius = radius_eval(law, whole)?;
    Ok(radius.is_degenerate())
}

/// One affine support chart of the amended walk: the recovered chart data of a
/// [`SupportChart`] plus the branch data the walk needs.
#[derive(Debug, Clone)]
struct VarChart {
    /// The first support chart.
    first: SupportChart,
    /// The second support chart.
    second: SupportChart,
    /// The recovered affine data of the first support.
    first_affine: AffineData,
    /// The recovered affine data of the second support.
    second_affine: AffineData,
    /// The certified unit branch direction `unit(n̂_1 × n̂_2)`.
    dir: [f64; 3],
    /// The certified seed box.
    seed: IBox3,
    /// The geometric midpoint of the seed box (the arc origin).
    seed_origin: [f64; 3],
    /// The excluded boundaries (a junction is not reachable through the frozen
    /// seed accessors; a declared clearance plane is).
    exclusions: Vec<Exclusion>,
}

impl VarChart {
    /// Build the branch chart of a seed.
    fn try_new(
        seed: &BranchSeed,
        boundary: Option<ClearanceBoundary>,
    ) -> Result<VarChart, ConstructRefusal> {
        let first = seed.first().clone();
        let second = seed.second().clone();
        let first_affine = recover_affine(&first)?;
        let second_affine = recover_affine(&second)?;
        let dir = branch_direction(&first, &second)?;
        let mut exclusions: Vec<Exclusion> = Vec::new();
        if let Some(boundary) = boundary {
            exclusions.push(Exclusion::try_new(
                boundary.origin,
                boundary.normal,
                boundary.mode,
                ExclusionRole::Collision,
            )?);
        }
        let seed_box = seed.seed();
        let seed_origin = box_midpoint(&seed_box);
        Ok(VarChart {
            first,
            second,
            first_affine,
            second_affine,
            dir,
            seed: seed_box,
            seed_origin,
            exclusions,
        })
    }

    /// The signed offset row `(c − a)·n̂ − ερ` of the first support.
    fn offset_first(&self, radius: f64) -> LinEq {
        offset_row(&self.first, radius)
    }

    /// The signed offset row of the second support.
    fn offset_second(&self, radius: f64) -> LinEq {
        offset_row(&self.second, radius)
    }
}

/// The recovered affine chart data of one planar support patch.
#[derive(Debug, Clone, Copy, PartialEq)]
struct AffineData {
    /// A base point on the support plane.
    base: [f64; 3],
    /// The source-unit tangent `S_u`.
    su: [f64; 3],
    /// The source-unit tangent `S_v`.
    sv: [f64; 3],
    /// The source parameter origin `(u_0, v_0)` of the recovered chart.
    origin: (f64, f64),
}

/// The role of an excluded face in the event vocabulary.
#[derive(Debug, Clone, Copy, PartialEq)]
enum ExclusionRole {
    /// The face is only an excluded boundary: reaching it is a certified
    /// [`EventKind::Collision`].
    Collision,
}

/// One certified clearance face the ball must stay clear of during the walk.
#[derive(Debug, Clone)]
struct Exclusion {
    /// The origin point of the face plane.
    origin: [f64; 3],
    /// The unit normal of the face plane.
    normal: [f64; 3],
    /// The containment side of the ball.
    mode: BallAdmissibility,
    /// The excluded half-space box (exact for an axis-aligned plane).
    box_: Box3,
}

impl Exclusion {
    /// Build an exclusion record, deriving the exact half-space box for an
    /// axis-aligned plane. Refuses [`ConstructRefusal::InvalidInput`] on a
    /// non-axis-aligned plane, a zero normal, or a non-finite origin.
    fn try_new(
        origin: [f64; 3],
        normal: [f64; 3],
        mode: BallAdmissibility,
        _role: ExclusionRole,
    ) -> Result<Exclusion, ConstructRefusal> {
        if !origin.iter().all(|v| v.is_finite()) {
            return Err(ConstructRefusal::InvalidInput);
        }
        let norm = norm3(&normal);
        if !norm.is_finite() || norm == 0.0 {
            return Err(ConstructRefusal::InvalidInput);
        }
        let inv = 1.0 / norm;
        let unit = [normal[0] * inv, normal[1] * inv, normal[2] * inv];
        let axis = dominant_axis(&unit)?;
        for (i, component) in unit.iter().enumerate() {
            if i != axis && *component != 0.0 {
                return Err(ConstructRefusal::InvalidInput);
            }
        }
        let signed = unit[axis];
        let plane_coord = origin[axis];
        let (lo, hi) = match (mode, signed > 0.0) {
            (BallAdmissibility::Round, true) => (plane_coord, f64::INFINITY),
            (BallAdmissibility::Round, false) => (f64::NEG_INFINITY, plane_coord),
            (BallAdmissibility::Fillet, true) => (f64::NEG_INFINITY, plane_coord),
            (BallAdmissibility::Fillet, false) => (plane_coord, f64::INFINITY),
        };
        let box_axis = inari_interval(lo, hi)?;
        let other = inari_interval(f64::NEG_INFINITY, f64::INFINITY)?;
        let mut x = other;
        let mut y = other;
        let mut z = other;
        match axis {
            0 => x = box_axis,
            1 => y = box_axis,
            _ => z = box_axis,
        }
        Ok(Exclusion {
            origin,
            normal: unit,
            mode,
            box_: Box3 { x, y, z },
        })
    }

    /// The certified tangency row: the centre where a point ball first meets
    /// this face (the signed offset form with the face's own contact side).
    fn tangency(&self, radius: f64) -> LinEq {
        let eps = if self.mode == BallAdmissibility::Round {
            -1.0
        } else {
            1.0
        };
        let k = -dot3(&self.normal, &self.origin) - eps * radius;
        LinEq { w: self.normal, k }
    }

    /// The certified implicit-field plane of the face.
    fn plane(&self) -> Plane {
        plane_from_normal(self.origin, self.normal)
    }
}

/// A signed linear equation `w·c + k = 0` in the centre chart.
#[derive(Debug, Clone, Copy)]
struct LinEq {
    w: [f64; 3],
    k: f64,
}

impl LinEq {
    /// The outward-rounded enclosure of `w·c + k` over the box.
    fn eval_iv(&self, c: &[Interval; 3]) -> Interval {
        let mut acc = Interval::point(self.k);
        #[allow(clippy::needless_range_loop)]
        // the fixed 0..3 axis accumulation order is the determinism contract
        for axis in 0..3 {
            let term = Interval::point(self.w[axis]).mul(&c[axis]);
            acc = acc.add(&term);
        }
        acc
    }
}

/// The constant 3×3 Jacobian of a linear chart system, row-major.
#[derive(Debug, Clone, Copy)]
struct ChartJacobian {
    rows: [[f64; 3]; 3],
}

impl ChartJacobian {
    /// The outward-rounded interval determinant `det J`.
    fn det_iv(&self) -> Interval {
        let r = self.rows;
        let a = Interval::point(r[0][0]);
        let b = Interval::point(r[0][1]);
        let c = Interval::point(r[0][2]);
        let d = Interval::point(r[1][0]);
        let e = Interval::point(r[1][1]);
        let f = Interval::point(r[1][2]);
        let g = Interval::point(r[2][0]);
        let h = Interval::point(r[2][1]);
        let i = Interval::point(r[2][2]);
        a.mul(&e.mul(&i).sub(&f.mul(&h)))
            .sub(&b.mul(&d.mul(&i).sub(&f.mul(&g))))
            .add(&c.mul(&d.mul(&h).sub(&e.mul(&g))))
    }

    /// The scalar determinant in `f64` (used for the certified-root Newton
    /// refinement). `None` when the determinant is not finite.
    fn det_f64(&self) -> Option<f64> {
        let r = self.rows;
        let det = r[0][0] * (r[1][1] * r[2][2] - r[1][2] * r[2][1])
            - r[0][1] * (r[1][0] * r[2][2] - r[1][2] * r[2][0])
            + r[0][2] * (r[1][0] * r[2][1] - r[1][1] * r[2][0]);
        if det.is_finite() {
            Some(det)
        } else {
            None
        }
    }
}

/// One certified arity-3 linear chart system: the two offset rows at a radius
/// plus a third row chosen by the caller (the foot row of a continuation step
/// or a trim/tangency row of an event).
#[derive(Debug, Clone, Copy)]
struct ChartSystem {
    rows: [LinEq; 3],
    jac: ChartJacobian,
}

impl SquareResidualEval for ChartSystem {
    fn arity(&self) -> usize {
        3
    }

    fn eval(&self, b: &[Interval]) -> Vec<Interval> {
        if b.len() != 3 {
            return vec![unbounded(); 3];
        }
        let c = [b[0], b[1], b[2]];
        let mut out = Vec::with_capacity(3);
        for row in &self.rows {
            out.push(row.eval_iv(&c));
        }
        out
    }

    fn jac_encl(&self, _b: &[Interval]) -> Vec<Vec<Interval>> {
        let mut out = Vec::with_capacity(3);
        for row in &self.jac.rows {
            out.push(row.iter().map(|v| Interval::point(*v)).collect());
        }
        out
    }
}

/// The certified outcome of one arity-3 Krawczyk certification.
#[derive(Debug, Clone, Copy)]
enum CertifyOutcome {
    /// The box contains a certified root.
    Proven(IBox3),
    /// The box certifiably contains no root.
    Disproven,
    /// The operator could not decide on the box.
    Inconclusive,
}

/// The certified root or absence inside a seed box.
#[derive(Debug, Clone, Copy)]
enum RootOutcome {
    /// A certified root, refined to a tight certified enclosure.
    Root(IBox3),
    /// No certified root inside the seed box.
    Absent,
}

/// One certified boundary event located by the walker.
#[derive(Debug, Clone)]
struct LocatedEvent {
    /// The kind of the located event.
    kind: EventKind,
    /// The certified centre enclosure of the event.
    centre: IBox3,
}

/// One certified continuation step of the amended walk, carrying the foot and
/// radius data the next step needs.
#[derive(Debug, Clone)]
struct CertifiedVarStep {
    /// The certified centre box.
    centre: IBox3,
    /// The certified foot parameter point the step's system targeted.
    foot_point: f64,
    /// The certified foot region of the step's centre box on the guide.
    foot_region: Interval,
    /// The certified radius interval over the foot region.
    radius_region: Interval,
    /// The step's state.
    state: WalkState,
}

impl CertifiedVarStep {
    /// The public step record of this certified step.
    fn public_step(&self, origin: &[f64; 3], dir: &[f64; 3]) -> VariableStep {
        VariableStep {
            centre: self.centre,
            arc: arc_projection(&self.centre, origin, dir),
            state: self.state,
            foot: self.foot_region,
            radius: self.radius_region,
        }
    }
}

/// Certify the seed step: the branch point whose guide foot is the foot of the
/// seed box, certified inside the seed box.
fn certify_seed_step(
    chart: &VarChart,
    guide: &CertifiedCurveMap,
    law: &RadiusLaw,
    budget: &mut Budget,
) -> Result<CertifiedVarStep, ConstructRefusal> {
    let seed_box = chart.seed;
    let centre_iv = box_iv3(&seed_box);
    let guess = foot_guess(guide, &centre_iv);
    let window = foot_window(guide, guess)?;
    let foot_region = foot_certified_region(guide, &centre_iv, window)?;
    let foot_point = midpoint(&foot_region);
    let radius_point = law_radius_point(law, guide, foot_point)?;
    let system = continuation_system(chart, guide, foot_point, radius_point)?;
    let outcome = certify_box(&system, seed_box, budget, &weight()?)?;
    let centre = match outcome {
        CertifyOutcome::Proven(boxed) => boxed,
        _ => return Err(ConstructRefusal::ConditioningBelowThreshold),
    };
    let centre_window = foot_window(guide, foot_point)?;
    let foot_region_box = foot_certified_region(guide, &box_iv3(&centre), centre_window)?;
    let radius_region = law_radius_region(law, guide, &foot_region_box)?;
    let state = step_state(
        chart,
        guide,
        &centre,
        foot_point,
        radius_point,
        radius_region,
    )?;
    if !state.feet_interior
        || !state.clearance_clear
        || !state.rank_regular
        || !state.radius_admissible
    {
        return Err(ConstructRefusal::InvalidInput);
    }
    Ok(CertifiedVarStep {
        centre,
        foot_point,
        foot_region: foot_region_box,
        radius_region,
        state,
    })
}

/// Walk one arc direction of a branch from the seed step until the first
/// certified event.
#[allow(clippy::too_many_arguments)]
fn walk_direction(
    chart: &VarChart,
    guide: &CertifiedCurveMap,
    law: &RadiusLaw,
    seed_step: &CertifiedVarStep,
    direction: f64,
    budget: &mut Budget,
    steps: &mut Vec<VariableStep>,
    events: &mut Vec<BlendEvent>,
) -> Result<(), ConstructRefusal> {
    let mut current = seed_step.clone();
    loop {
        let next = certify_next_step(chart, guide, law, &current, direction, budget)?;
        let window = foot_window(guide, next.foot_point)?;
        let candidate_region = foot_certified_region(guide, &box_iv3(&next.centre), window)?;
        let radius_region = law_radius_region(law, guide, &candidate_region)?;
        let state = step_state(
            chart,
            guide,
            &next.centre,
            next.foot_point,
            next.radius_point,
            radius_region,
        )?;
        let radius_consistent =
            radius_region.lo <= next.radius_point && next.radius_point <= radius_region.hi;
        let accepted = state.feet_interior
            && state.clearance_clear
            && state.rank_regular
            && state.radius_admissible
            && radius_consistent;
        if accepted {
            let step = CertifiedVarStep {
                centre: next.centre,
                foot_point: next.foot_point,
                foot_region: candidate_region,
                radius_region,
                state,
            };
            steps.push(step.public_step(&chart.seed_origin, &chart.dir));
            current = step;
            continue;
        }
        let located = locate_event(
            chart,
            guide,
            &current,
            &next.centre,
            direction,
            next.radius_point,
            budget,
        )?;
        match located {
            Some(event) => {
                events.push(BlendEvent {
                    kind: event.kind,
                    at: arc_projection(&event.centre, &chart.seed_origin, &chart.dir),
                    node: None,
                });
                return Ok(());
            }
            None => return Err(ConstructRefusal::ConditioningBelowThreshold),
        }
    }
}

/// The continuation step data of a certified next step.
#[derive(Debug, Clone, Copy)]
struct NextStep {
    /// The certified centre box.
    centre: IBox3,
    /// The certified foot parameter point the step targeted.
    foot_point: f64,
    /// The certified radius point the offset rows used.
    radius_point: f64,
}

/// Certify one continuation step at `direction * ARC_STEP` from the last
/// certified step, halving the predictor step until the corrector certifies.
///
/// When the predicted centre's guide foot lies beyond the guide domain in the
/// walk direction (the requested law ends at the guide edge), the step
/// certifies the branch centre AT the guide-domain edge instead: the branch of
/// the requested law is exhausted there and the walk terminates at the event
/// that centre produces.
fn certify_next_step(
    chart: &VarChart,
    guide: &CertifiedCurveMap,
    law: &RadiusLaw,
    current: &CertifiedVarStep,
    direction: f64,
    budget: &mut Budget,
) -> Result<NextStep, ConstructRefusal> {
    let p = box_midpoint(&current.centre);
    let mut delta = ARC_STEP;
    let mut halvings = 0u32;
    loop {
        let predicted = add3_scaled(&p, &chart.dir, direction * delta);
        let window = foot_window(guide, current.foot_point)?;
        let point_iv: [Interval; 3] = predicted.map(Interval::point);
        foot_point_gate(guide, &point_iv, window)?;
        match refine_foot_point(guide, &predicted, window)? {
            FootRoot::Point(foot_point) => {
                let radius_point = law_radius_point(law, guide, foot_point)?;
                let system = continuation_system(chart, guide, foot_point, radius_point)?;
                // The certified root of the continuation rows sits at the
                // exact linear-solution centre of the system, which on a
                // variable-radius branch is offset from the raw predictor
                // point (the branch curves with the radius law). Recentre the
                // certification box on that centre so the corrector has a fair
                // box.
                let guess = match centre_solve(&system) {
                    Some(guess) => guess,
                    None => return Err(ConstructRefusal::ConditioningBelowThreshold),
                };
                let candidate = certified_box(guess, CERTIFIED_HALF)?;
                match certify_box(&system, candidate, budget, &weight()?)? {
                    CertifyOutcome::Proven(boxed) => {
                        return Ok(NextStep {
                            centre: boxed,
                            foot_point,
                            radius_point,
                        })
                    }
                    CertifyOutcome::Disproven | CertifyOutcome::Inconclusive => {
                        halvings += 1;
                        if halvings > CC_DEPTH_MAX {
                            return Err(ConstructRefusal::ConditioningBelowThreshold);
                        }
                        delta *= 0.5;
                    }
                }
            }
            FootRoot::Right => {
                let edge = guide_domain(guide).1;
                return certify_domain_edge(chart, guide, law, edge, budget);
            }
            FootRoot::Left => {
                let edge = guide_domain(guide).0;
                return certify_domain_edge(chart, guide, law, edge, budget);
            }
        }
    }
}

/// Certify the last branch centre on the guide-domain edge: the certified
/// continuation step whose foot parameter is the guide-domain bound `edge`.
fn certify_domain_edge(
    chart: &VarChart,
    guide: &CertifiedCurveMap,
    law: &RadiusLaw,
    edge: f64,
    budget: &mut Budget,
) -> Result<NextStep, ConstructRefusal> {
    let radius_point = law_radius_point(law, guide, edge)?;
    let system = continuation_system(chart, guide, edge, radius_point)?;
    let guess = match centre_solve(&system) {
        Some(guess) => guess,
        None => return Err(ConstructRefusal::ConditioningBelowThreshold),
    };
    let candidate = certified_box(guess, CERTIFIED_HALF)?;
    match certify_box(&system, candidate, budget, &weight()?)? {
        CertifyOutcome::Proven(boxed) => Ok(NextStep {
            centre: boxed,
            foot_point: edge,
            radius_point,
        }),
        CertifyOutcome::Disproven | CertifyOutcome::Inconclusive => {
            Err(ConstructRefusal::ConditioningBelowThreshold)
        }
    }
}

/// The exact `f64` centre of a linear chart system (the rows solved against
/// zero by Cramer's rule); `None` on a singular or non-finite system.
fn centre_solve(system: &ChartSystem) -> Option<[f64; 3]> {
    let z = [0.0_f64; 3];
    let f = [
        eval_point(&system.rows[0], &z),
        eval_point(&system.rows[1], &z),
        eval_point(&system.rows[2], &z),
    ];
    solve_linear(&system.jac, &f)
}

/// The certified foot search window around a reference foot parameter.
fn foot_window(guide: &CertifiedCurveMap, reference: f64) -> Result<CurveRegion, ConstructRefusal> {
    let domain = guide_domain(guide);
    let lo = (reference - FOOT_WINDOW_RADIUS).max(domain.0);
    let hi = (reference + FOOT_WINDOW_RADIUS).min(domain.1);
    if lo.is_finite() && hi.is_finite() && lo < hi {
        Ok((lo, hi))
    } else {
        Err(ConstructRefusal::InvalidInput)
    }
}

/// The continuation system of a step: the two offset rows at `radius_point`
/// plus the foot row at `foot_point`.
fn continuation_system(
    chart: &VarChart,
    guide: &CertifiedCurveMap,
    foot_point: f64,
    radius_point: f64,
) -> Result<ChartSystem, ConstructRefusal> {
    let foot_row = guide_foot_row(guide, foot_point)?;
    let jac = ChartJacobian {
        rows: [chart.first.normal(), chart.second.normal(), foot_row.w],
    };
    Ok(ChartSystem {
        rows: [
            chart.offset_first(radius_point),
            chart.offset_second(radius_point),
            foot_row,
        ],
        jac,
    })
}

/// The certified foot row `(c − G(λ))·G′(λ) = 0` at a guide parameter point.
fn guide_foot_row(guide: &CertifiedCurveMap, t: f64) -> Result<LinEq, ConstructRefusal> {
    let g = curve_point(guide, t)?;
    let gp = curve_deriv_point(guide, t)?;
    Ok(LinEq {
        w: gp,
        k: -dot3(&gp, &g),
    })
}

/// The certified step state `Σ` of a certified centre box (the isolation
/// battery).
#[allow(clippy::too_many_arguments)]
fn step_state(
    chart: &VarChart,
    guide: &CertifiedCurveMap,
    centre: &IBox3,
    foot_point: f64,
    radius_point: f64,
    radius: Interval,
) -> Result<WalkState, ConstructRefusal> {
    let iv = box_iv3(centre);
    let feet_interior = feet_interior(chart, &iv)?;
    let clearance_clear = clearances_clear(chart, centre, &radius)?;
    let tangent = curve_deriv_point(guide, foot_point)?;
    if norm3(&tangent) == 0.0 {
        return Err(ConstructRefusal::ConditioningBelowThreshold);
    }
    let jac = ChartJacobian {
        rows: [chart.first.normal(), chart.second.normal(), tangent],
    };
    let det = jac.det_iv();
    let rank_regular = det.is_finite() && abs_lower(det) > CC_ETA_J;
    let radius_admissible = radius.is_finite() && radius.lo > 0.0 && radius_point > 0.0;
    Ok(WalkState {
        first_in_contact: true,
        second_in_contact: true,
        first_side: chart.first.side(),
        second_side: chart.second.side(),
        feet_interior,
        clearance_clear,
        rank_regular,
        radius_admissible,
    })
}

/// Whether both certified contact feet are strictly interior to their regions.
fn feet_interior(chart: &VarChart, centre: &[Interval; 3]) -> Result<bool, ConstructRefusal> {
    for (support, affine) in [
        (&chart.first, &chart.first_affine),
        (&chart.second, &chart.second_affine),
    ] {
        let rows = param_rows(affine)?;
        let region = support.region();
        for (row, bounds) in [(rows.0, region.0), (rows.1, region.1)] {
            let value = row.eval_iv(centre);
            if !(value.lo > bounds.0 && value.hi < bounds.1) {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

/// Whether the rolling ball is certified clear of every excluded boundary.
fn clearances_clear(
    chart: &VarChart,
    centre: &IBox3,
    radius: &Interval,
) -> Result<bool, ConstructRefusal> {
    if chart.exclusions.is_empty() {
        return Ok(true);
    }
    let centre_box = inari_box(centre)?;
    let r_iv = inari_interval(radius.lo, radius.hi)?;
    for exclusion in &chart.exclusions {
        let plane = exclusion.plane();
        match ball_clearance(
            &plane,
            &centre_box,
            &exclusion.box_,
            r_iv,
            CC_MU_CLEAR,
            exclusion.mode,
        ) {
            Ok(true) => {}
            Ok(false) => return Ok(false),
            Err(_) => return Ok(false),
        }
    }
    Ok(true)
}

/// Locate and certify the first event at the boundary between the last
/// accepted step and the rejected candidate step.
#[allow(clippy::too_many_arguments)]
fn locate_event(
    chart: &VarChart,
    guide: &CertifiedCurveMap,
    last: &CertifiedVarStep,
    rejected: &IBox3,
    direction: f64,
    radius: f64,
    budget: &mut Budget,
) -> Result<Option<LocatedEvent>, ConstructRefusal> {
    let search = event_search_box(chart, &last.centre, rejected, direction)?;
    let last_centre = box_midpoint(&last.centre);
    let mut best: Option<LocatedEvent> = None;
    let mut best_distance: f64 = f64::INFINITY;
    let accept = |kind: EventKind,
                  centre: IBox3,
                  best: &mut Option<LocatedEvent>,
                  best_distance: &mut f64| {
        let m = box_midpoint(&centre);
        let s = (m[0] - last_centre[0]) * chart.dir[0]
            + (m[1] - last_centre[1]) * chart.dir[1]
            + (m[2] - last_centre[2]) * chart.dir[2];
        let distance = direction * s;
        if distance > 0.0 && distance < *best_distance {
            *best_distance = distance;
            *best = Some(LocatedEvent { kind, centre });
        }
    };
    for (support, affine) in [
        (&chart.first, &chart.first_affine),
        (&chart.second, &chart.second_affine),
    ] {
        let rows = param_rows(affine)?;
        let region = support.region();
        for (row, bounds) in [(rows.0, region.0), (rows.1, region.1)] {
            for bound in [bounds.0, bounds.1] {
                let system = trim_system(chart, &row, bound, radius)?;
                let root = certify_root(&system, search, budget)?;
                if let RootOutcome::Root(boxed) = root {
                    accept(EventKind::Trim, boxed, &mut best, &mut best_distance);
                }
            }
        }
    }
    for exclusion in &chart.exclusions {
        let tangent = exclusion.tangency(radius);
        let system = tangency_system(chart, &tangent, radius)?;
        let root = certify_root(&system, search, budget)?;
        if let RootOutcome::Root(boxed) = root {
            accept(EventKind::Collision, boxed, &mut best, &mut best_distance);
        }
    }
    let _ = guide;
    Ok(best)
}

/// The certified radius point of the law at a guide parameter point.
fn law_radius_point(
    law: &RadiusLaw,
    guide: &CertifiedCurveMap,
    t: f64,
) -> Result<f64, ConstructRefusal> {
    let unit = point_unit_image(guide, t)?;
    let r = radius_eval(law, unit)?;
    if !r.is_finite() || r.lo <= 0.0 {
        return Err(ConstructRefusal::ConditioningBelowThreshold);
    }
    Ok(midpoint(&r))
}

/// The certified radius enclosure of the law over a certified foot region.
fn law_radius_region(
    law: &RadiusLaw,
    guide: &CertifiedCurveMap,
    region: &Interval,
) -> Result<Interval, ConstructRefusal> {
    let unit = region_unit_image(guide, region)?;
    radius_eval(law, unit).map_err(|_| ConstructRefusal::ConditioningBelowThreshold)
}

/// The certified trim system: the offset rows at a radius plus one
/// foot-parameter row.
fn trim_system(
    chart: &VarChart,
    row: &LinEq,
    bound: f64,
    radius: f64,
) -> Result<ChartSystem, ConstructRefusal> {
    let trim = LinEq {
        w: row.w,
        k: row.k - bound,
    };
    Ok(ChartSystem {
        rows: [
            chart.offset_first(radius),
            chart.offset_second(radius),
            trim,
        ],
        jac: ChartJacobian {
            rows: [chart.first.normal(), chart.second.normal(), trim.w],
        },
    })
}

/// The certified tangency system: the offset rows at a radius plus one
/// tangency row.
fn tangency_system(
    chart: &VarChart,
    tangent: &LinEq,
    radius: f64,
) -> Result<ChartSystem, ConstructRefusal> {
    Ok(ChartSystem {
        rows: [
            chart.offset_first(radius),
            chart.offset_second(radius),
            *tangent,
        ],
        jac: ChartJacobian {
            rows: [chart.first.normal(), chart.second.normal(), tangent.w],
        },
    })
}

/// One certified arity-3 Krawczyk certification (one budgeted Newton spend).
fn certify_box(
    system: &ChartSystem,
    boxed: IBox3,
    budget: &mut Budget,
    weight: &CertifiedPositive,
) -> Result<CertifyOutcome, ConstructRefusal> {
    budget
        .spend_newton(1)
        .map_err(|_| ConstructRefusal::ConditioningBelowThreshold)?;
    match krawczyk_c1_n3(system, boxed, &[*weight]) {
        ClaimVerdict::Proven(cert) => Ok(CertifyOutcome::Proven(cert.box_)),
        ClaimVerdict::Disproven(_) => Ok(CertifyOutcome::Disproven),
        ClaimVerdict::Inconclusive(_) => Ok(CertifyOutcome::Inconclusive),
    }
}

/// Certify a root of the linear chart system inside the search box, refining
/// the certified enclosure around the root.
fn certify_root(
    system: &ChartSystem,
    search: IBox3,
    budget: &mut Budget,
) -> Result<RootOutcome, ConstructRefusal> {
    match certify_box(system, search, budget, &weight()?)? {
        CertifyOutcome::Disproven => Ok(RootOutcome::Absent),
        CertifyOutcome::Inconclusive => Ok(RootOutcome::Absent),
        CertifyOutcome::Proven(boxed) => {
            let refined = refine_root(system, &boxed, budget)?;
            Ok(RootOutcome::Root(refined))
        }
    }
}

/// Refine a certified root to a tight certified enclosure around the linear
/// root (one Newton recentre, then one more Krawczyk certification).
fn refine_root(
    system: &ChartSystem,
    proven: &IBox3,
    budget: &mut Budget,
) -> Result<IBox3, ConstructRefusal> {
    let guess = newton_guess(system, proven);
    let candidate = certified_box(guess, EVENT_HALF)?;
    match certify_box(system, candidate, budget, &weight()?)? {
        CertifyOutcome::Proven(boxed) => Ok(boxed),
        CertifyOutcome::Disproven | CertifyOutcome::Inconclusive => Ok(*proven),
    }
}

/// The certified-root Newton guess: `z − J⁻¹·F(z)` in plain `f64` over the
/// exact constant Jacobian (linear chart; the guess lands on the root up to
/// rounding, and the follow-up Krawczyk certification is the certificate).
fn newton_guess(system: &ChartSystem, boxed: &IBox3) -> [f64; 3] {
    let z = box_midpoint(boxed);
    let f = [
        eval_point(&system.rows[0], &z),
        eval_point(&system.rows[1], &z),
        eval_point(&system.rows[2], &z),
    ];
    solve_linear(&system.jac, &f).map_or(z, |step| [z[0] + step[0], z[1] + step[1], z[2] + step[2]])
}

/// Evaluate a linear row at a point in `f64`.
fn eval_point(row: &LinEq, p: &[f64; 3]) -> f64 {
    dot3(&row.w, p) + row.k
}

/// Solve `J·x = −f` for the Newton step by Cramer's rule in `f64`; `None` on
/// a zero or non-finite determinant.
fn solve_linear(jac: &ChartJacobian, f: &[f64; 3]) -> Option<[f64; 3]> {
    let det = jac.det_f64()?;
    if det == 0.0 {
        return None;
    }
    let det0 = determinant_with_column(&jac.rows, 0, f);
    let det1 = determinant_with_column(&jac.rows, 1, f);
    let det2 = determinant_with_column(&jac.rows, 2, f);
    if !(det0.is_finite() && det1.is_finite() && det2.is_finite()) {
        return None;
    }
    Some([det0 / det, det1 / det, det2 / det])
}

/// The determinant of the row matrix with column `col` replaced by `−f`.
fn determinant_with_column(rows: &[[f64; 3]; 3], col: usize, f: &[f64; 3]) -> f64 {
    let mut m = *rows;
    for r in 0..3 {
        m[r][col] = -f[r];
    }
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

/// The certified event search box spanning the last accepted step and the
/// rejected candidate step, with slack ahead in the walk direction.
fn event_search_box(
    chart: &VarChart,
    last: &IBox3,
    rejected: &IBox3,
    direction: f64,
) -> Result<IBox3, ConstructRefusal> {
    let last_centre = box_midpoint(last);
    let ahead = add3_scaled(&last_centre, &chart.dir, direction * EVENT_SLACK);
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for boxed in [last, rejected] {
        for axis in 0..3 {
            lo[axis] = lo[axis].min(boxed.lo[axis]).min(ahead[axis]);
            hi[axis] = hi[axis].max(boxed.hi[axis]).max(ahead[axis]);
        }
    }
    ibox3(lo, hi)
}

/// The offset row `(c − a)·n̂ − ερ` of a support.
fn offset_row(chart: &SupportChart, radius: f64) -> LinEq {
    let n = chart.normal();
    let a = chart.base();
    let k = -dot3(&n, &a) - chart.side() * radius;
    LinEq { w: n, k }
}

/// The certified unit tangent of the branch: the canonical sign of
/// `unit(n̂_1 × n̂_2)`.
fn branch_direction(
    first: &SupportChart,
    second: &SupportChart,
) -> Result<[f64; 3], ConstructRefusal> {
    let cross = cross3(&first.normal(), &second.normal());
    let norm = norm3(&cross);
    if !norm.is_finite() || norm <= 0.0 {
        return Err(ConstructRefusal::InvalidInput);
    }
    let inv = 1.0 / norm;
    let mut dir = [cross[0] * inv, cross[1] * inv, cross[2] * inv];
    for component in dir.iter_mut() {
        if *component != 0.0 {
            if *component < 0.0 {
                for value in dir.iter_mut() {
                    *value = -*value;
                }
            }
            break;
        }
    }
    Ok(dir)
}

/// Recover the affine chart data of a support from its certified map.
///
/// The support must be affine over its certified region (the CC-020
/// offset-corner class, guaranteed by the chart's own construction): every
/// touched Bézier patch is exactly flat and shares one tangent frame. This is
/// the mirror of the chart constructor's own recovery, re-derived here because
/// the source-unit tangents and the parameter origin are not exposed by the
/// frozen chart accessors.
fn recover_affine(chart: &SupportChart) -> Result<AffineData, ConstructRefusal> {
    let region = chart.region();
    let map: &CertifiedSurfaceMap = chart.map();
    let boxes = map.patch_boxes();
    let grids = map.patch_grids();
    let mut first: Option<AffineData> = None;
    for (patch_box, patch_grids) in boxes.iter().zip(grids.iter()) {
        if !touches(*patch_box, region) {
            continue;
        }
        if !patch_flat(patch_grids) {
            return Err(ConstructRefusal::InvalidInput);
        }
        let (su, sv) = patch_tangents(patch_grids, *patch_box)?;
        if let Some(proto) = first.as_ref() {
            if proto.su != su || proto.sv != sv {
                return Err(ConstructRefusal::InvalidInput);
            }
        } else {
            first = Some(AffineData {
                base: patch_base(patch_grids)?,
                su,
                sv,
                origin: (patch_box.0 .0, patch_box.1 .0),
            });
        }
    }
    match first {
        Some(affine) => Ok(affine),
        None => Err(ConstructRefusal::InvalidInput),
    }
}

/// The certified foot-parameter rows (value form `w·c + k`) of an affine
/// support.
///
/// The foot of a centre on the support plane has parameter
/// `u = u_0 + ((foot − base)·su)/|su|²` through the Gram inversion of the
/// affine chart; because the foot projection drops the normal component the
/// row is a pure linear function of the centre.
fn param_rows(affine: &AffineData) -> Result<(LinEq, LinEq), ConstructRefusal> {
    let su = affine.su;
    let sv = affine.sv;
    let g11 = dot3(&su, &su);
    let g12 = dot3(&su, &sv);
    let g22 = dot3(&sv, &sv);
    let det = g11 * g22 - g12 * g12;
    if !det.is_finite() || det <= 0.0 {
        return Err(ConstructRefusal::InvalidInput);
    }
    let inv = 1.0 / det;
    let wu = [
        (g22 * su[0] - g12 * sv[0]) * inv,
        (g22 * su[1] - g12 * sv[1]) * inv,
        (g22 * su[2] - g12 * sv[2]) * inv,
    ];
    let wv = [
        (g11 * sv[0] - g12 * su[0]) * inv,
        (g11 * sv[1] - g12 * su[1]) * inv,
        (g11 * sv[2] - g12 * su[2]) * inv,
    ];
    let base = affine.base;
    let ku = affine.origin.0 - (g22 * dot3(&base, &su) - g12 * dot3(&base, &sv)) * inv;
    let kv = affine.origin.1 - (g11 * dot3(&base, &sv) - g12 * dot3(&base, &su)) * inv;
    if !wu.iter().all(|v| v.is_finite())
        || !wv.iter().all(|v| v.is_finite())
        || !ku.is_finite()
        || !kv.is_finite()
    {
        return Err(ConstructRefusal::InvalidInput);
    }
    Ok((LinEq { w: wu, k: ku }, LinEq { w: wv, k: kv }))
}

/// The λ-derivative of the foot-point residual over a guide region.
///
/// `∂_λ[(c − G(λ))·G′(λ)] = −‖G′(λ)‖² + (c − G(λ))·G″(λ)` enclosed through
/// the certified value and derivative hulls of the map, the interval centre
/// argument, and a fixed coordinate reduction order.
fn foot_derivative(
    map: &CertifiedCurveMap,
    c: &[Interval; 3],
    sub: CurveRegion,
) -> Result<Interval, ConstructRefusal> {
    let g = map
        .enclosure(sub)
        .map_err(|_| ConstructRefusal::InvalidInput)?;
    let gp = curve_deriv_hull(map, 1, sub)?;
    let gpp = curve_deriv_hull(map, 2, sub)?;
    let mut acc = Interval::point(0.0);
    for axis in 0..3 {
        let sq = gp[axis].mul(&gp[axis]);
        acc = acc.sub(&sq);
        let diff = c[axis].sub(&g[axis]);
        acc = acc.add(&diff.mul(&gpp[axis]));
    }
    Ok(acc)
}

/// The certified enclosure of the foot-point residual `(c − G(λ))·G′(λ)` over
/// a guide region.
fn foot_residual(
    map: &CertifiedCurveMap,
    c: &[Interval; 3],
    sub: CurveRegion,
) -> Result<Interval, ConstructRefusal> {
    let g = map
        .enclosure(sub)
        .map_err(|_| ConstructRefusal::InvalidInput)?;
    let gp = curve_deriv_hull(map, 1, sub)?;
    let mut acc = Interval::point(0.0);
    for axis in 0..3 {
        let diff = c[axis].sub(&g[axis]);
        acc = acc.add(&diff.mul(&gp[axis]));
    }
    Ok(acc)
}

/// Certify the foot region of a centre box on the guide inside a window.
///
/// The foot map is strictly decreasing over the window (the local uniqueness
/// gate over the window certifies it for the whole centre enclosure), the
/// foot of the box midpoint is isolated by a certified monotone bisection, and
/// the box's whole foot spread is bounded by the certified implicit-derivative
/// margin `‖G′‖_sup/|∂_λ f|`.
fn foot_certified_region(
    map: &CertifiedCurveMap,
    c: &[Interval; 3],
    window: CurveRegion,
) -> Result<Interval, ConstructRefusal> {
    foot_point_gate(map, c, window)?;
    let mid_rep = midpoint_vec(c)?;
    let root = match refine_foot_point(map, &mid_rep, window)? {
        FootRoot::Point(root) => root,
        FootRoot::Right | FootRoot::Left => {
            return Err(ConstructRefusal::ConditioningBelowThreshold)
        }
    };
    let derivative = foot_derivative(map, c, window)?;
    let eta = -derivative.hi;
    if !eta.is_finite() || eta <= 0.0 {
        return Err(ConstructRefusal::ConditioningBelowThreshold);
    }
    let gsup = deriv_norm_sup(map, window)?;
    let half_diag = box_half_diagonal(c)?;
    let margin = (half_diag * gsup / eta).next_up();
    if !margin.is_finite() {
        return Err(ConstructRefusal::ConditioningBelowThreshold);
    }
    let domain = guide_domain(map);
    let lo = (root - margin).max(domain.0);
    let hi = (root + margin).min(domain.1);
    if !lo.is_finite() || !hi.is_finite() || lo > hi {
        return Err(ConstructRefusal::ConditioningBelowThreshold);
    }
    let region = Interval { lo, hi };
    let gate = foot_point_gate(map, c, (lo, hi))?;
    if gate.hi < 0.0 {
        Ok(region)
    } else {
        Err(ConstructRefusal::ConditioningBelowThreshold)
    }
}

/// The certified monotone-foot isolation outcome of a centre point inside a
/// window.
#[derive(Debug, Clone, Copy, PartialEq)]
enum FootRoot {
    /// A certified root parameter inside the window.
    Point(f64),
    /// The residual is strictly positive over the whole window: the root lies
    /// to the right of the window (the foot map is certified decreasing).
    Right,
    /// The residual is strictly negative over the whole window: the root lies
    /// to the left of the window.
    Left,
}

/// Isolate the certified foot parameter point of a centre point inside a
/// window by monotone bisection of the certified residual.
///
/// The residual is certified monotone over the window by the caller's
/// foot-point gate; a window whose residual does not straddle zero reports
/// [`FootRoot::Right`] or [`FootRoot::Left`] instead of a spurious root.
fn refine_foot_point(
    map: &CertifiedCurveMap,
    centre: &[f64; 3],
    window: CurveRegion,
) -> Result<FootRoot, ConstructRefusal> {
    let c: [Interval; 3] = centre.map(Interval::point);
    let whole = foot_residual(map, &c, window)?;
    if !whole.is_finite() {
        return Err(ConstructRefusal::ConditioningBelowThreshold);
    }
    if whole.lo > 0.0 {
        return Ok(FootRoot::Right);
    }
    if whole.hi < 0.0 {
        return Ok(FootRoot::Left);
    }
    let mut a = window.0;
    let mut b = window.1;
    for _ in 0..128 {
        if b - a <= FOOT_BISECT_TOL {
            return Ok(FootRoot::Point((a + b) * 0.5));
        }
        let mid = (a + b) * 0.5;
        let left = foot_residual(map, &c, (a, mid))?;
        if left.lo <= 0.0 && left.hi >= 0.0 {
            b = mid;
        } else {
            a = mid;
        }
    }
    Ok(FootRoot::Point((a + b) * 0.5))
}

/// A plain `f64` guess of the foot parameter of a centre box: the guide sample
/// where the point-evaluated residual is smallest.
fn foot_guess(map: &CertifiedCurveMap, centre: &[Interval; 3]) -> f64 {
    let domain = guide_domain(map);
    let samples = 32usize;
    let rep = midpoint_vec(centre).unwrap_or([0.0; 3]);
    let mut best = domain.0;
    let mut best_abs = f64::INFINITY;
    for i in 0..samples {
        let t = domain.0 + (domain.1 - domain.0) * (i as f64) / ((samples - 1) as f64);
        if let Ok(value) = foot_point_at(map, &rep, t) {
            if value.abs() < best_abs {
                best_abs = value.abs();
                best = t;
            }
        }
    }
    best
}

/// The point-evaluated foot residual at a guide parameter (plain `f64`).
fn foot_point_at(
    map: &CertifiedCurveMap,
    centre: &[f64; 3],
    t: f64,
) -> Result<f64, ConstructRefusal> {
    let g = curve_point(map, t)?;
    let gp = curve_deriv_point(map, t)?;
    let mut acc = 0.0_f64;
    for axis in 0..3 {
        acc += (centre[axis] - g[axis]) * gp[axis];
    }
    Ok(acc)
}

/// The certified sup of `‖G′‖` over a guide region.
fn deriv_norm_sup(map: &CertifiedCurveMap, sub: CurveRegion) -> Result<f64, ConstructRefusal> {
    let components = curve_deriv_hull(map, 1, sub)?;
    let mut sum = 0.0_f64;
    for component in &components {
        if !component.is_finite() {
            return Err(ConstructRefusal::InvalidInput);
        }
        let max_abs = component.lo.abs().max(component.hi.abs());
        let square = (max_abs * max_abs).next_up();
        if !square.is_finite() {
            return Err(ConstructRefusal::InvalidInput);
        }
        sum = (sum + square).next_up();
        if !sum.is_finite() {
            return Err(ConstructRefusal::InvalidInput);
        }
    }
    let root = sum.sqrt();
    if !root.is_finite() {
        return Err(ConstructRefusal::InvalidInput);
    }
    Ok(root.next_up())
}

/// The half diagonal of an interval box.
fn box_half_diagonal(c: &[Interval; 3]) -> Result<f64, ConstructRefusal> {
    let mut sum = 0.0_f64;
    for axis in c {
        if !axis.is_finite() {
            return Err(ConstructRefusal::InvalidInput);
        }
        let half = (axis.hi - axis.lo) * 0.5;
        sum += half * half;
    }
    Ok(sum.sqrt())
}

/// The certified per-coordinate `order`-derivative hull of a curve map over a
/// compact sub-region, in source units (the CC-002 hull path).
fn curve_deriv_hull(
    map: &CertifiedCurveMap,
    order: u32,
    sub: CurveRegion,
) -> Result<[Interval; 3], ConstructRefusal> {
    if order != 1 && order != 2 {
        return Err(ConstructRefusal::InvalidInput);
    }
    let intervals = map.piece_intervals();
    let grids = map.piece_grids();
    if intervals.is_empty() {
        return Err(ConstructRefusal::InvalidInput);
    }
    let domain = (intervals[0].0, intervals[intervals.len() - 1].1);
    if !sub.0.is_finite()
        || !sub.1.is_finite()
        || sub.0 < domain.0
        || sub.1 > domain.1
        || sub.0 >= sub.1
    {
        return Err(ConstructRefusal::InvalidInput);
    }
    let mut acc = [Interval {
        lo: f64::INFINITY,
        hi: f64::NEG_INFINITY,
    }; 3];
    for (interval, coeffs) in intervals.iter().zip(grids.iter()) {
        let (t0, t1) = *interval;
        if sub.0 > t1 || sub.1 < t0 {
            continue;
        }
        let overlap = (sub.0.max(t0), sub.1.min(t1));
        if overlap.0 >= overlap.1 {
            continue;
        }
        let (u_lo, u_hi) = unit_image(*interval, overlap)?;
        let width = t1 - t0;
        if !width.is_finite() || width <= 0.0 {
            return Err(ConstructRefusal::InvalidInput);
        }
        let inv_width = 1.0 / width;
        for (k, vector) in coeffs.iter().enumerate() {
            let first: Vec<f64> = bernstein_derivative_1d(vector)
                .iter()
                .map(|c| c * inv_width)
                .collect();
            let source = if order == 1 {
                first
            } else {
                bernstein_derivative_1d(&first)
                    .iter()
                    .map(|c| c * inv_width)
                    .collect()
            };
            let hull = hull_bernstein_1d(&source, (u_lo, u_hi))
                .map_err(|_| ConstructRefusal::InvalidInput)?;
            acc[k].lo = acc[k].lo.min(hull.lo);
            acc[k].hi = acc[k].hi.max(hull.hi);
        }
    }
    if acc.iter().all(|a| a.is_finite()) {
        Ok(acc)
    } else {
        Err(ConstructRefusal::InvalidInput)
    }
}

/// The declared source domain of a certified curve map.
fn guide_domain(map: &CertifiedCurveMap) -> CurveRegion {
    let intervals = map.piece_intervals();
    (intervals[0].0, intervals[intervals.len() - 1].1)
}

/// A certified point on the guide: the midpoint of the value enclosure at a
/// small parameter sample around `t`.
fn curve_point(map: &CertifiedCurveMap, t: f64) -> Result<[f64; 3], ConstructRefusal> {
    let domain = guide_domain(map);
    let sample = sample_interval(domain, t);
    let enclosure = map
        .enclosure(sample)
        .map_err(|_| ConstructRefusal::InvalidInput)?;
    Ok(enclosure.map(|iv| midpoint(&iv)))
}

/// A certified point tangent of the guide: the midpoint of the derivative
/// enclosure at a small parameter sample around `t`.
fn curve_deriv_point(map: &CertifiedCurveMap, t: f64) -> Result<[f64; 3], ConstructRefusal> {
    let domain = guide_domain(map);
    let sample = sample_interval(domain, t);
    let hull = curve_deriv_hull(map, 1, sample)?;
    Ok(hull.map(|iv| midpoint(&iv)))
}

/// A small positive-width parameter sample around `t` inside the domain.
fn sample_interval(domain: CurveRegion, t: f64) -> CurveRegion {
    let eps = (domain.1 - domain.0) * 1e-7;
    let lo = (t - eps).max(domain.0);
    let hi = (t + eps).min(domain.1);
    if lo < hi {
        (lo, hi)
    } else {
        (domain.0, domain.0 + (domain.1 - domain.0) * 1e-7)
    }
}

/// The unit image of a guide parameter point (for the radius law, which is
/// declared over the normalized `[0, 1]` arc).
fn point_unit_image(map: &CertifiedCurveMap, t: f64) -> Result<Interval, ConstructRefusal> {
    let domain = guide_domain(map);
    let width = domain.1 - domain.0;
    if !width.is_finite() || width <= 0.0 {
        return Err(ConstructRefusal::InvalidInput);
    }
    let inv = 1.0 / width;
    let u = Interval::point((t - domain.0) * inv);
    if u.lo < 0.0 || u.hi > 1.0 {
        return Err(ConstructRefusal::InvalidInput);
    }
    Ok(u)
}

/// The unit image of a guide region (for the radius law evaluator), enclosed
/// in `Interval` arithmetic and clamped to `[0, 1]`.
fn region_unit_image(
    map: &CertifiedCurveMap,
    region: &Interval,
) -> Result<Interval, ConstructRefusal> {
    let domain = guide_domain(map);
    let span_iv = Interval::point(domain.0);
    let width_iv = Interval::point(domain.1 - domain.0);
    let lo_u = Interval::point(region.lo)
        .sub(&span_iv)
        .div(&width_iv)
        .ok_or(ConstructRefusal::InvalidInput)?;
    let hi_u = Interval::point(region.hi)
        .sub(&span_iv)
        .div(&width_iv)
        .ok_or(ConstructRefusal::InvalidInput)?;
    let u_lo = lo_u.lo.min(hi_u.lo).clamp(0.0, 1.0);
    let u_hi = lo_u.hi.max(hi_u.hi).clamp(0.0, 1.0);
    if u_lo.is_finite() && u_hi.is_finite() && u_lo <= u_hi {
        Ok(Interval { lo: u_lo, hi: u_hi })
    } else {
        Err(ConstructRefusal::InvalidInput)
    }
}

/// The exact unit-parameter image of an overlap under the span's own
/// source-to-unit affine map, enclosed in `Interval` arithmetic and clamped to
/// `[0, 1]`.
fn unit_image(span: CurveRegion, overlap: CurveRegion) -> Result<(f64, f64), ConstructRefusal> {
    let (a, b) = span;
    let (lo, hi) = overlap;
    let a_iv = Interval::point(a);
    let span_iv = Interval::point(b).sub(&a_iv);
    let lo_u = Interval::point(lo)
        .sub(&a_iv)
        .div(&span_iv)
        .ok_or(ConstructRefusal::InvalidInput)?;
    let hi_u = Interval::point(hi)
        .sub(&a_iv)
        .div(&span_iv)
        .ok_or(ConstructRefusal::InvalidInput)?;
    let u_lo = lo_u.lo.min(hi_u.lo).clamp(0.0, 1.0);
    let u_hi = lo_u.hi.max(hi_u.hi).clamp(0.0, 1.0);
    if u_lo.is_finite() && u_hi.is_finite() && u_lo <= u_hi {
        Ok((u_lo, u_hi))
    } else {
        Err(ConstructRefusal::InvalidInput)
    }
}

/// A certified positive weight value.
fn weight() -> Result<CertifiedPositive, ConstructRefusal> {
    CertifiedPositive::try_new(1.0).map_err(|_| ConstructRefusal::InvalidInput)
}

/// The certified box of a centre with the given half-width.
fn certified_box(centre: [f64; 3], half: f64) -> Result<IBox3, ConstructRefusal> {
    let lo = [centre[0] - half, centre[1] - half, centre[2] - half];
    let hi = [centre[0] + half, centre[1] + half, centre[2] + half];
    ibox3(lo, hi)
}

/// The geometric midpoint of a box.
fn box_midpoint(boxed: &IBox3) -> [f64; 3] {
    [
        (boxed.lo[0] + boxed.hi[0]) / 2.0,
        (boxed.lo[1] + boxed.hi[1]) / 2.0,
        (boxed.lo[2] + boxed.hi[2]) / 2.0,
    ]
}

/// The midpoint of an interval enclosure.
fn midpoint(v: &Interval) -> f64 {
    (v.lo + v.hi) * 0.5
}

/// The componentwise midpoints of an interval vector.
fn midpoint_vec(v: &[Interval; 3]) -> Result<[f64; 3], ConstructRefusal> {
    if v.iter().all(|x| x.is_finite()) {
        Ok(v.map(|x| (x.lo + x.hi) * 0.5))
    } else {
        Err(ConstructRefusal::InvalidInput)
    }
}

/// The certified arc interval of a box along the unit direction `dir`, measured
/// from the seed centre `origin`.
fn arc_projection(boxed: &IBox3, origin: &[f64; 3], dir: &[f64; 3]) -> Interval {
    let mut acc = Interval::point(0.0);
    for axis in 0..3 {
        let span = Interval {
            lo: boxed.lo[axis] - origin[axis],
            hi: boxed.hi[axis] - origin[axis],
        };
        acc = acc.add(&Interval::point(dir[axis]).mul(&span));
    }
    acc
}

/// The per-axis intervals of a box.
fn box_iv3(b: &IBox3) -> [Interval; 3] {
    [
        Interval {
            lo: b.lo[0],
            hi: b.hi[0],
        },
        Interval {
            lo: b.lo[1],
            hi: b.hi[1],
        },
        Interval {
            lo: b.lo[2],
            hi: b.hi[2],
        },
    ]
}

/// A certified `IBox3`.
fn ibox3(lo: [f64; 3], hi: [f64; 3]) -> Result<IBox3, ConstructRefusal> {
    IBox3::try_new(lo, hi).map_err(|_| ConstructRefusal::InvalidInput)
}

/// The certified half-space interval of an axis-aligned exclusion.
fn inari_interval(lo: f64, hi: f64) -> Result<InariInterval, ConstructRefusal> {
    InariInterval::try_from((lo, hi)).map_err(|_| ConstructRefusal::InvalidInput)
}

/// The certified `truck-evidence` box of a certified centre box.
fn inari_box(boxed: &IBox3) -> Result<Box3, ConstructRefusal> {
    Ok(Box3 {
        x: inari_interval(boxed.lo[0], boxed.hi[0])?,
        y: inari_interval(boxed.lo[1], boxed.hi[1])?,
        z: inari_interval(boxed.lo[2], boxed.hi[2])?,
    })
}

/// The vacuous enclosure of the full real line (the engine convention for an
/// invalid evaluation request).
fn unbounded() -> Interval {
    Interval {
        lo: f64::NEG_INFINITY,
        hi: f64::INFINITY,
    }
}

/// The `truck-base` point from coordinates.
fn point3(p: [f64; 3]) -> Point3 {
    Point3::new(p[0], p[1], p[2])
}

/// Vector addition.
fn add3(a: &[f64; 3], b: &[f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

/// Scaled vector addition `a + s·d`.
fn add3_scaled(a: &[f64; 3], d: &[f64; 3], s: f64) -> [f64; 3] {
    [a[0] + s * d[0], a[1] + s * d[1], a[2] + s * d[2]]
}

/// The dot product.
fn dot3(a: &[f64; 3], b: &[f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// The cross product.
fn cross3(a: &[f64; 3], b: &[f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// The Euclidean norm.
fn norm3(a: &[f64; 3]) -> f64 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}

/// A certified lower bound of `min |x|` over an interval enclosure: `0` when
/// the enclosure contains zero, else the nearer endpoint's magnitude.
fn abs_lower(v: Interval) -> f64 {
    if v.lo > 0.0 {
        v.lo
    } else if v.hi < 0.0 {
        -v.hi
    } else {
        0.0
    }
}

/// The index of the dominant axis of a unit vector.
fn dominant_axis(v: &[f64; 3]) -> Result<usize, ConstructRefusal> {
    let mut axis = 0usize;
    let mut best = v[0].abs();
    for (i, component) in v.iter().enumerate().skip(1) {
        let mag = component.abs();
        if mag > best {
            best = mag;
            axis = i;
        }
    }
    if best == 0.0 {
        return Err(ConstructRefusal::InvalidInput);
    }
    Ok(axis)
}

/// A `truck-geometry` plane through `origin` with the given unit normal.
fn plane_from_normal(origin: [f64; 3], n: [f64; 3]) -> Plane {
    let seed = if n[2].abs() < 0.9 {
        [0.0, 0.0, 1.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let w = cross3(&n, &seed);
    let w = unit_or_zero(&w);
    let b = cross3(&n, &w);
    Plane::new(
        point3(origin),
        point3(add3(&origin, &w)),
        point3(add3(&origin, &b)),
    )
}

/// The unit vector of `v`, or a deterministic fallback unit vector when `v`
/// vanishes.
fn unit_or_zero(v: &[f64; 3]) -> [f64; 3] {
    let norm = norm3(v);
    if norm == 0.0 {
        [1.0, 0.0, 0.0]
    } else {
        let inv = 1.0 / norm;
        [v[0] * inv, v[1] * inv, v[2] * inv]
    }
}

/// Whether a patch box and the region share a point.
fn touches(patch: SurfaceRegion, region: SurfaceRegion) -> bool {
    axis_touches(patch.0, region.0) && axis_touches(patch.1, region.1)
}

/// Whether two closed intervals overlap (inclusive).
fn axis_touches(a: (f64, f64), b: (f64, f64)) -> bool {
    !(a.1 < b.0 || b.1 < a.0)
}

/// The CC-002 flatness gate on one Bézier patch: the three second-partial
/// coefficient grids are EXACTLY zero.
fn patch_flat(grids: &[Vec<Vec<f64>>; 3]) -> bool {
    for grid in grids {
        if !flat_grid(grid) {
            return false;
        }
    }
    true
}

/// Whether one coefficient grid is exactly affine over its patch.
fn flat_grid(grid: &[Vec<f64>]) -> bool {
    let duu = bernstein_derivative_2d(&bernstein_derivative_2d(grid, 0), 0);
    let dvv = bernstein_derivative_2d(&bernstein_derivative_2d(grid, 1), 1);
    let duv = bernstein_derivative_2d(&bernstein_derivative_2d(grid, 0), 1);
    all_zero(&duu) && all_zero(&dvv) && all_zero(&duv)
}

/// Whether every entry of a grid is exactly zero.
fn all_zero(grid: &[Vec<f64>]) -> bool {
    grid.iter().all(|row| row.iter().all(|c| *c == 0.0))
}

/// The source-unit tangent pair of a flat patch.
fn patch_tangents(
    grids: &[Vec<Vec<f64>>; 3],
    patch_box: SurfaceRegion,
) -> Result<([f64; 3], [f64; 3]), ConstructRefusal> {
    let width_u = patch_box.0 .1 - patch_box.0 .0;
    let width_v = patch_box.1 .1 - patch_box.1 .0;
    if !width_u.is_finite() || !width_v.is_finite() || width_u <= 0.0 || width_v <= 0.0 {
        return Err(ConstructRefusal::InvalidInput);
    }
    let inv_u = 1.0 / width_u;
    let inv_v = 1.0 / width_v;
    let mut su = [0.0_f64; 3];
    let mut sv = [0.0_f64; 3];
    for (k, grid) in grids.iter().enumerate() {
        let du = bernstein_derivative_2d(grid, 0);
        let dv = bernstein_derivative_2d(grid, 1);
        su[k] = constant_of(&du, inv_u)?;
        sv[k] = constant_of(&dv, inv_v)?;
    }
    Ok((su, sv))
}

/// Read the (exact) common value of a constant coefficient grid, scaled by
/// `scale`. A non-constant grid is refused.
fn constant_of(grid: &[Vec<f64>], scale: f64) -> Result<f64, ConstructRefusal> {
    let first = match grid.first().and_then(|row| row.first()) {
        Some(value) => *value,
        None => return Err(ConstructRefusal::InvalidInput),
    };
    for row in grid {
        for value in row {
            if *value != first {
                return Err(ConstructRefusal::InvalidInput);
            }
        }
    }
    Ok(first * scale)
}

/// The value of the flat patch at its lower-left source corner.
fn patch_base(grids: &[Vec<Vec<f64>>; 3]) -> Result<[f64; 3], ConstructRefusal> {
    let mut base = [0.0_f64; 3];
    for (k, grid) in grids.iter().enumerate() {
        let value = match grid.first().and_then(|row| row.first()) {
            Some(value) => *value,
            None => return Err(ConstructRefusal::InvalidInput),
        };
        base[k] = value;
    }
    Ok(base)
}
