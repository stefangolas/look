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

//! CC-030-BLEND-SPINE (spine seam S12): the two-support rolling-ball blend
//! trace — a walk in the admissible stratum graph (theory §5.1–5.2) over the
//! landed certified substrate.
//!
//! # The branch (Section 1, realized here)
//!
//! For two supports `(S_1, S_2)` with signed offset sides `ε_i` and a radius
//! law, the constrained branch of a rolling ball of radius `r` is the 1-D
//! solution family of `c = S_1(u_1) + ε_1 r n̂_1 = S_2(u_2) + ε_2 r n̂_2`
//! closed by `Φ(r) = 0` through the landed radius evaluators. This packet
//! realizes the walk on the **affine offset-corner class** (CC-020's scope
//! guard): every support is a certified surface map that is affine over its
//! certified region (identically zero second-partial coefficient grids over
//! every touched patch), so the offset centre chart is exact and every
//! defining function below is a certified linear form. A support that is not
//! affine over its region is refused [`ConstructRefusal::InvalidInput`] at
//! chart build time — the curved-support system is a later packet's booking,
//! exactly as in CC-020.
//!
//! With constant radius `ρ` (the packet's fixtures; a non-degenerate radius
//! closure is refused here and booked to CC-031), the branch is the straight
//! centre line `L = { c : (c − a_1)·n̂_1 = ε_1 ρ, (c − a_2)·n̂_2 = ε_2 ρ }`,
//! walked in a certified predictor/corrector loop. The **arity-3 corrector**
//! is the engine's `krawczyk_c1_n3` over the square system
//!
//! ```text
//! G_1(c) = (c − a_1)·n̂_1 − ε_1 ρ         (offset to the first support)
//! G_2(c) = (c − a_2)·n̂_2 − ε_2 ρ         (offset to the second support)
//! G_3(c) = (c − p_k)·d̂ − dir·δ           (the tangent/pin row)
//! ```
//!
//! with `d̂` the certified tangent of the branch chart and `δ` the predictor
//! arc step. The predictor is the tangent step `p_k + dir·δ·d̂` from the last
//! certified step; a step is certified only when the operator contracts with
//! strict interior, otherwise the step is halved. Every certified continuation
//! box is the accepted box (the certificate carries the box it ran over), and
//! the certified contact data of each accepted step is read from that box.
//!
//! # The event vocabulary (Section 2, realized here)
//!
//! [`EventKind`] is the CC-000 stub vocabulary verbatim. Events are detected
//! at the certified-step boundaries as **isolated-root problems of the same
//! arity-3 chart**: an event is a certified centre where the rolling ball
//! reaches a boundary of the admissible region, i.e. where a certified linear
//! form vanishes:
//!
//! - [`EventKind::Trim`] — a contact parameter reaches a support's trim
//!   boundary (a foot parameter equals a region bound of its support);
//! - [`EventKind::ThirdFace`] — the ball reaches tangency with a declared
//!   third support: `solve_triple_node` (CC-020, seam S11) certifies the node
//!   on the branch. Nodes are SOLVED ONCE per chain and referenced (P6);
//! - [`EventKind::Collision`] — `ball_clearance` (CC-004, seam S7) flips to
//!   Rejected for the rolling ball against a declared excluded boundary;
//! - [`EventKind::Rank`] / [`EventKind::Focal`] — the branch's certified rank
//!   (step-Jacobian submersion) / radius-regularity margins collapse.
//!
//! The discrete state `Σ` of theory §5.2 is [`WalkState`]: every accepted
//! certified step must keep every component's defining function certified
//! strictly separated (feet interior to their regions, the ball clear of every
//! excluded boundary, the submersion and radius margins above the floor). A
//! step that cannot is never accepted — the walk stops at the certified event
//! boundary instead, so `Σ` is identical across every accepted step between
//! two events.
//!
//! # The walk (Section 3, realized here)
//!
//! [`trace_blend_chain`] walks each [`BranchSeed`] in the slice, terminates a
//! branch only at certified events, and records those events in walk order in
//! [`BlendTrace`]. Between events the topology is FIXED — no speculation and
//! no extrapolation past an undecided step: an undecided step (a step the
//! corrector cannot certify, or a budget exhaustion) surfaces as an `Err` of
//! the refusal family ([`ConstructRefusal::ConditioningBelowThreshold`], the
//! frozen-contract counterpart of the conditioning failure), never as a
//! guessed continuation. [`trace_branch_steps`] exposes the per-branch
//! certified walk (accepted steps plus events) so the stop rule is observable
//! at the certified-step level.
//!
//! # Scope guards (stop conditions)
//!
//! (1) The deliverable ends at the certified event record: face consumption is
//! CC-032 and setback corners are CC-033. (2) Only the constant-radius
//! fixtures run here: a radius law whose closure over the arc is not a single
//! positive point is refused (the foot-point variable-radius formulation rides
//! the SAME arity-3 system through CC-031's amendment). (3) The event
//! vocabulary is closed: new event kinds are a CC-000 amendment, never a local
//! enum.
//!
//! **H-1.** This module carries no `unwrap`, no `expect`, and no `panic!`, and
//! adds no module-level `allow`. Every float reduction runs in a fixed order
//! with directed rounding (C9), and every `Interval` below is the C3 universe
//! (`construct::Interval`), never a second interval type.

use crate::certified_map::{CertifiedSurfaceMap, SurfaceRegion};
use crate::construct::canal::radius_eval;
use crate::construct::config::{CC_DEPTH_MAX, CC_ETA_J, CC_MU_CLEAR};
use crate::construct::contact3::{solve_triple_node, ReducedSystem, TripleNodeOutcome};
use crate::construct::refusal::ConstructRefusal;
use crate::construct::stubs::{EventKind, RadiusLaw, TripleContactNode};
use crate::construct::Interval;
use crate::hull::bernstein_derivative_2d;
use crate::kernel::engine::{krawczyk_c1_n3, SquareResidualEval};
use crate::kernel::evidence::ClaimVerdict;
use crate::kernel::patch::{CertifiedPositive, IBox3};
use truck_base::cgmath64::Point3;
use truck_base::evidence::Budget;
use truck_evidence::clear::{ball_clearance, BallAdmissibility};
use truck_evidence::enclosure::{Box3, Interval as InariInterval};
use truck_geometry::specifieds::Plane;

/// The half-width of a certified continuation box along every axis.
///
/// The corrector certifies boxes of this half-width around the predicted
/// centre; the certificate carries the box it ran over.
pub const BLEND_CERTIFIED_HALF: f64 = 0.02;

/// The nominal arc step of the predictor between certified steps.
pub const BLEND_ARC_STEP: f64 = 0.04;

/// The half-width of a refined certified event enclosure.
pub const BLEND_EVENT_HALF: f64 = 0.004;

/// The search slack added beyond a rejected predictor step when locating the
/// certified event root at a certified-step boundary.
pub const BLEND_EVENT_SLACK: f64 = 0.08;

/// The S12 blend-trace output record (seam S12): the complete walk of the
/// chain as its certified events in walk order.
///
/// Between events there is no topology: [`BlendTrace`] records only certified
/// events with their certified enclosures.
#[derive(Debug, Clone, PartialEq)]
pub struct BlendTrace {
    /// The certified events of the whole chain, in walk order.
    pub events: Vec<BlendEvent>,
}

/// One certified blend event (seam S12).
#[derive(Debug, Clone, PartialEq)]
pub struct BlendEvent {
    /// The event kind (the closed CC-000 vocabulary).
    pub kind: EventKind,
    /// The certified enclosure of the event's position along the branch arc.
    pub at: Interval,
    /// The certified triple-contact node, present exactly on a
    /// [`EventKind::ThirdFace`] event.
    pub node: Option<TripleContactNode>,
}

/// The discrete state `Σ` of one accepted certified step (theory §5.2).
///
/// Every accepted step between two events carries an IDENTICAL [`WalkState`]:
/// the isolation invariant is that no defining function of the fixed topology
/// changed sign or lost separation on any accepted step. A step that would
/// change the state is not accepted — the walk stops at the certified event
/// boundary instead.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WalkState {
    /// The first support is in certified contact with the ball.
    pub first_in_contact: bool,
    /// The second support is in certified contact with the ball.
    pub second_in_contact: bool,
    /// The signed offset side of the first support (`+1` / `−1`).
    pub first_side: f64,
    /// The signed offset side of the second support (`+1` / `−1`).
    pub second_side: f64,
    /// The certified contact feet of both supports are strictly interior to
    /// their certified regions.
    pub feet_interior: bool,
    /// The rolling ball is certified clear of every excluded boundary
    /// (`true` when no excluded boundary is declared).
    pub clearance_clear: bool,
    /// The certified step-Jacobian submersion margin is above the normative
    /// floor.
    pub rank_regular: bool,
    /// The certified radius law stays admissible (positive, slope-gated).
    pub radius_admissible: bool,
}

/// One accepted certified continuation step of a branch walk.
#[derive(Debug, Clone, PartialEq)]
pub struct CertifiedStep {
    /// The certified centre box of the step (the box the corrector certified).
    pub centre: IBox3,
    /// The certified enclosure of the step's position along the branch arc
    /// (measured from the certified seed centre).
    pub arc: Interval,
    /// The discrete state `Σ` the step carries.
    pub state: WalkState,
}

/// The per-branch certified walk: the accepted certified steps and the events
/// the walk terminated at.
#[derive(Debug, Clone, PartialEq)]
pub struct CertifiedBranch {
    /// The certified events of this branch, in walk order.
    pub events: Vec<BlendEvent>,
    /// The accepted certified steps between the events, in walk order.
    pub steps: Vec<CertifiedStep>,
}

/// A named refusal of a per-branch certified walk.
///
/// The refusal carries the [`CertifiedBranch`] recorded up to the refusing
/// step, so an undecided step is observable as a stop EXACTLY at the last
/// certified step — never as a guessed continuation.
#[derive(Debug, Clone, PartialEq)]
pub struct BranchRefusal {
    /// The construct-layer refusal of the walk.
    pub refusal: ConstructRefusal,
    /// The certified steps and events recorded before the refusal.
    pub partial: CertifiedBranch,
}

/// One affine support chart: a certified surface map over a certified region
/// with its signed offset side.
///
/// The support must be affine over `region` (the CC-020 offset-corner class):
/// every touched Bézier patch is exactly flat and shares one tangent frame, so
/// the recovered `base` / `su` / `sv` / unit `normal` data is exact. The side
/// `ε ∈ {+1, −1}` fixes which side of the support the rolling-ball centre lies
/// on (`+1` = the `+n̂` side).
#[derive(Debug, Clone)]
pub struct SupportChart {
    /// The admitted certified surface map of the support.
    map: CertifiedSurfaceMap,
    /// The certified region the branch may contact on the support.
    region: SurfaceRegion,
    /// The signed offset side `ε ∈ {+1, −1}`.
    side: f64,
    /// A base point on the support plane (a patch-lower-left surface value).
    base: [f64; 3],
    /// The source-unit tangent `S_u` (the affine chart's first direction).
    su: [f64; 3],
    /// The source-unit tangent `S_v` (the affine chart's second direction).
    sv: [f64; 3],
    /// The oriented unit normal `n̂ = (S_u × S_v)/|S_u × S_v|`.
    normal: [f64; 3],
    /// The source parameter origin `(u_0, v_0)` of the recovered affine chart.
    origin: (f64, f64),
}

/// An excluded boundary the rolling ball must stay clear of.
///
/// The plane is given by `origin` and the unit `normal`; `mode` fixes the
/// containment side ([`BallAdmissibility::Round`]: the ball stays where the
/// implicit field is non-positive; [`BallAdmissibility::Fillet`]: where it is
/// non-negative). The exclusion box used by the P5 predicate is the exact
/// half-space of an axis-aligned plane; a non-axis-aligned clearance plane is
/// refused at chart build time (the v1 constant-radius fixtures are
/// axis-aligned; general clearance planes are a later booking).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClearanceBoundary {
    /// A point on the excluded plane.
    pub origin: [f64; 3],
    /// The unit normal of the excluded plane.
    pub normal: [f64; 3],
    /// Which side of the plane the ball must stay on.
    pub mode: BallAdmissibility,
}

/// The S12 blend branch seed (production posture): the two support
/// descriptions, their signed sides, the certified seed box, and the optional
/// third-support / excluded-boundary data that closes the branch.
///
/// The certified seed box is a centre region that must contain the branch near
/// its interior: the walker certifies the seed step there and walks both arc
/// directions until the first certified event in each.
#[derive(Debug, Clone)]
pub struct BranchSeed {
    /// The first support of the pair.
    first: SupportChart,
    /// The second support of the pair.
    second: SupportChart,
    /// The certified seed centre box on the branch.
    seed: IBox3,
    /// The optional third support the branch joins at a triple-contact node.
    junction: Option<SupportChart>,
    /// The optional excluded boundary the rolling ball must stay clear of.
    clearance: Option<ClearanceBoundary>,
}

impl SupportChart {
    /// Build a support chart over an admitted affine surface map.
    ///
    /// `map` is the certified surface map of the support, `region` its
    /// certified contact region, and `side` the signed offset side. Refuses
    /// [`ConstructRefusal::InvalidInput`] when the side is not `±1`, the region
    /// is not a finite non-degenerate rectangle, the certified rank margin over
    /// the region is not strictly positive, or a touched patch is not affine
    /// with one shared tangent frame.
    pub fn try_new(
        map: CertifiedSurfaceMap,
        region: SurfaceRegion,
        side: f64,
    ) -> Result<SupportChart, ConstructRefusal> {
        if side != 1.0 && side != -1.0 {
            return Err(ConstructRefusal::InvalidInput);
        }
        if !region_finite(region) {
            return Err(ConstructRefusal::InvalidInput);
        }
        let margin = map
            .rank_margin(region)
            .map_err(|_| ConstructRefusal::InvalidInput)?;
        if margin.lo <= 0.0 {
            return Err(ConstructRefusal::InvalidInput);
        }
        let boxes = map.patch_boxes();
        let grids = map.patch_grids();
        let mut first: Option<ChartAffine> = None;
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
                first = Some(ChartAffine {
                    base: patch_base(patch_grids)?,
                    su,
                    sv,
                    origin: (patch_box.0 .0, patch_box.1 .0),
                });
            }
        }
        let affine = match first {
            Some(affine) => affine,
            None => return Err(ConstructRefusal::InvalidInput),
        };
        let normal = unit_normal(&affine.su, &affine.sv)?;
        Ok(SupportChart {
            map,
            region,
            side,
            base: affine.base,
            su: affine.su,
            sv: affine.sv,
            normal,
            origin: affine.origin,
        })
    }

    /// The certified surface map of the support.
    pub fn map(&self) -> &CertifiedSurfaceMap {
        &self.map
    }

    /// The certified contact region of the support.
    pub fn region(&self) -> SurfaceRegion {
        self.region
    }

    /// The signed offset side `ε`.
    pub fn side(&self) -> f64 {
        self.side
    }

    /// A base point on the support plane.
    pub fn base(&self) -> [f64; 3] {
        self.base
    }

    /// The oriented unit normal `n̂` of the support plane.
    pub fn normal(&self) -> [f64; 3] {
        self.normal
    }
}

impl BranchSeed {
    /// Build a branch seed over the two support charts.
    ///
    /// `seed` is the certified seed centre box that must contain the branch
    /// near its interior; `junction` is the optional third support the branch
    /// joins at a triple-contact node, and `clearance` the optional excluded
    /// boundary the rolling ball must stay clear of. Refuses
    /// [`ConstructRefusal::InvalidInput`] on a non-finite or inverted seed
    /// box.
    pub fn try_new(
        first: SupportChart,
        second: SupportChart,
        seed: IBox3,
        junction: Option<SupportChart>,
        clearance: Option<ClearanceBoundary>,
    ) -> Result<BranchSeed, ConstructRefusal> {
        for axis in 0..3 {
            if !seed.lo[axis].is_finite()
                || !seed.hi[axis].is_finite()
                || seed.lo[axis] > seed.hi[axis]
            {
                return Err(ConstructRefusal::InvalidInput);
            }
        }
        Ok(BranchSeed {
            first,
            second,
            seed,
            junction,
            clearance,
        })
    }

    /// The first support chart of the seed.
    pub fn first(&self) -> &SupportChart {
        &self.first
    }

    /// The second support chart of the seed.
    pub fn second(&self) -> &SupportChart {
        &self.second
    }

    /// The certified seed centre box of the seed.
    pub fn seed(&self) -> IBox3 {
        self.seed
    }
}

/// The affine chart data recovered from one flat support patch.
#[derive(Debug, Clone, Copy, PartialEq)]
struct ChartAffine {
    base: [f64; 3],
    su: [f64; 3],
    sv: [f64; 3],
    origin: (f64, f64),
}

/// The role of an excluded face in the event vocabulary.
#[derive(Debug, Clone, Copy, PartialEq)]
enum ExclusionRole {
    /// The face is a third support: reaching it is a certified
    /// [`EventKind::ThirdFace`] triple-contact node.
    Junction,
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
    /// What reaching this face means.
    role: ExclusionRole,
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
        // The cofactor expansion along the first row, fixed term order.
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

/// One certified arity-3 linear chart system (Section 1's reduced system).
///
/// The three rows are the offset rows of the two supports plus a third row
/// chosen by the caller (the tangent/pin row of a continuation step, a trim
/// row, or a tangency row to a third face).
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
    /// The certified triple node for a `ThirdFace` event.
    node: Option<TripleContactNode>,
}

/// The per-chain store of solved triple nodes (P6): every node is solved ONCE
/// and referenced by every incident branch.
#[derive(Debug, Clone, Default)]
struct NodeBank {
    solved: Vec<(Vec<[f64; 7]>, TripleContactNode)>,
}

/// The branch chart: everything the walker needs about one two-support branch.
#[derive(Debug, Clone)]
struct BranchChart {
    first: SupportChart,
    second: SupportChart,
    junction: Option<SupportChart>,
    radius: Interval,
    dir: [f64; 3],
    seed: IBox3,
    seed_origin: [f64; 3],
    exclusions: Vec<Exclusion>,
    offset_first: LinEq,
    offset_second: LinEq,
}

/// Certify one two-support branch walk over a seed (Section 3, per branch).
///
/// The branch is built, the seed step certified, and both arc directions are
/// walked from the seed until each direction's first certified event. Returns
/// the accepted certified steps and the events in walk order.
///
/// An undecided step (an uncertifiable corrector box, or budget exhaustion)
/// refuses with [`ConstructRefusal::ConditioningBelowThreshold`]; the returned
/// [`BranchRefusal`] carries the certified steps and events recorded up to
/// that refusal so the stop is observable at the last certified step.
pub fn trace_branch_steps(
    seed: &BranchSeed,
    radius: &RadiusLaw,
    budget: &mut Budget,
) -> Result<CertifiedBranch, BranchRefusal> {
    let mut bank = NodeBank::default();
    trace_branch_with_bank(seed, radius, budget, &mut bank)
}

/// Trace the whole blend chain (seam S12, Section 3).
///
/// Each [`BranchSeed`] is walked to its certified events; branches that meet
/// at a triple-contact node share the node certificate (P6: solved once,
/// referenced). The events of every branch are returned in walk order. Any
/// branch that cannot complete its walk to a certified event refuses with the
/// underlying refusal family.
pub fn trace_blend_chain(
    branches: &[BranchSeed],
    radius: &RadiusLaw,
    budget: &mut Budget,
) -> Result<BlendTrace, ConstructRefusal> {
    let mut bank = NodeBank::default();
    let mut events: Vec<BlendEvent> = Vec::new();
    for seed in branches {
        let branch = trace_branch_with_bank(seed, radius, budget, &mut bank)
            .map_err(|refusal| refusal.refusal)?;
        events.extend(branch.events);
    }
    Ok(BlendTrace { events })
}

/// The shared per-branch trace body over one node bank.
fn trace_branch_with_bank(
    seed: &BranchSeed,
    radius: &RadiusLaw,
    budget: &mut Budget,
    bank: &mut NodeBank,
) -> Result<CertifiedBranch, BranchRefusal> {
    let empty = CertifiedBranch {
        events: Vec::new(),
        steps: Vec::new(),
    };
    let chart = build_chart(seed, radius).map_err(|refusal| BranchRefusal {
        refusal,
        partial: empty.clone(),
    })?;
    let weight = CertifiedPositive::try_new(1.0)
        .map_err(|_| ConstructRefusal::InvalidInput)
        .map_err(|refusal| BranchRefusal {
            refusal,
            partial: empty.clone(),
        })?;
    let seed_step =
        certify_seed_step(&chart, budget, &weight).map_err(|refusal| BranchRefusal {
            refusal,
            partial: empty.clone(),
        })?;
    let mut steps: Vec<CertifiedStep> = vec![seed_step.clone()];
    let mut events: Vec<BlendEvent> = Vec::new();
    walk_direction(
        &chart,
        &seed_step,
        1.0,
        radius,
        budget,
        &weight,
        bank,
        &mut steps,
        &mut events,
    )?;
    walk_direction(
        &chart,
        &seed_step,
        -1.0,
        radius,
        budget,
        &weight,
        bank,
        &mut steps,
        &mut events,
    )?;
    Ok(CertifiedBranch { events, steps })
}

/// Build the branch chart of a seed (Section 1 realization on the affine
/// class).
fn build_chart(seed: &BranchSeed, radius: &RadiusLaw) -> Result<BranchChart, ConstructRefusal> {
    let radius_arc = radius_eval(radius, Interval { lo: 0.0, hi: 1.0 })?;
    if !radius_arc.is_degenerate() || radius_arc.lo <= 0.0 || !radius_arc.is_finite() {
        return Err(ConstructRefusal::InvalidInput);
    }
    let first = &seed.first;
    let second = &seed.second;
    let mut exclusions: Vec<Exclusion> = Vec::new();
    if let Some(junction) = &seed.junction {
        exclusions.push(exclusion_of_chart(junction, ExclusionRole::Junction)?);
    }
    if let Some(clearance) = &seed.clearance {
        exclusions.push(exclusion_of_clearance(*clearance)?);
    }
    let dir = branch_direction(first, second)?;
    let offset_first = offset_row(first, radius_arc.lo);
    let offset_second = offset_row(second, radius_arc.lo);
    let seed_origin = box_midpoint(&seed.seed);
    Ok(BranchChart {
        first: first.clone(),
        second: second.clone(),
        junction: seed.junction.clone(),
        radius: radius_arc,
        dir,
        seed: seed.seed,
        seed_origin,
        exclusions,
        offset_first,
        offset_second,
    })
}

/// The certified unit tangent of the branch: the canonical sign of
/// `unit(n̂_1 × n̂_2)`.
fn branch_direction(
    first: &SupportChart,
    second: &SupportChart,
) -> Result<[f64; 3], ConstructRefusal> {
    let cross = cross3(&first.normal, &second.normal);
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

/// The offset row `(c − a)·n̂ − ερ` of a support.
fn offset_row(chart: &SupportChart, radius: f64) -> LinEq {
    let n = chart.normal;
    let a = chart.base;
    let k = -dot3(&n, &a) - chart.side * radius;
    LinEq { w: n, k }
}

/// The tangent/pin row `(c − p)·d̂ − dir·δ`.
fn pin_row(p: &[f64; 3], dir: &[f64; 3], direction: f64, delta: f64) -> LinEq {
    let k = -dot3(p, dir) - direction * delta;
    LinEq { w: *dir, k }
}

/// The excluded-boundary record of a junction support chart.
fn exclusion_of_chart(
    chart: &SupportChart,
    role: ExclusionRole,
) -> Result<Exclusion, ConstructRefusal> {
    let mode = if chart.side > 0.0 {
        BallAdmissibility::Fillet
    } else {
        BallAdmissibility::Round
    };
    Exclusion::try_new(chart.base, chart.normal, mode, role)
}

/// The excluded-boundary record of a declared clearance plane.
fn exclusion_of_clearance(clearance: ClearanceBoundary) -> Result<Exclusion, ConstructRefusal> {
    Exclusion::try_new(
        clearance.origin,
        clearance.normal,
        clearance.mode,
        ExclusionRole::Collision,
    )
}

impl Exclusion {
    /// Build an exclusion record, deriving the exact half-space box for an
    /// axis-aligned plane. Refuses [`ConstructRefusal::InvalidInput`] on a
    /// non-axis-aligned plane, a zero normal, or a non-finite origin.
    fn try_new(
        origin: [f64; 3],
        normal: [f64; 3],
        mode: BallAdmissibility,
        role: ExclusionRole,
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
        // Admissible ball side: Round keeps `f <= 0`, Fillet keeps `f >= 0`.
        // The excluded half-space is the opposite strict side.
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
            role,
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
///
/// `u = c`, `v = n̂ × c` for a unit vector `c ⊥ n̂`, so `u × v = n̂` exactly
/// (up to normalization) and the implicit field reads `n̂·(p − origin)`.
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

/// Build the certified seed step: certify the branch point at the seed box's
/// centre (the tangent pin `(c − seed_mid)·d̂ = 0`).
fn certify_seed_step(
    chart: &BranchChart,
    budget: &mut Budget,
    weight: &CertifiedPositive,
) -> Result<CertifiedStep, ConstructRefusal> {
    let mid = box_midpoint(&chart.seed);
    let system = continuation_system(chart, &mid, 1.0, 0.0);
    let outcome = certify_box(&system, chart.seed, budget, weight)?;
    let centre = match outcome {
        CertifyOutcome::Proven(boxed) => boxed,
        _ => return Err(ConstructRefusal::ConditioningBelowThreshold),
    };
    let state = step_state(chart, &centre)?;
    if !state.feet_interior
        || !state.clearance_clear
        || !state.rank_regular
        || !state.radius_admissible
    {
        return Err(ConstructRefusal::InvalidInput);
    }
    Ok(CertifiedStep {
        centre,
        arc: arc_projection(&centre, &chart.seed_origin, &chart.dir),
        state,
    })
}

/// Walk one arc direction of a branch from the seed step until the first
/// certified event.
#[allow(clippy::too_many_arguments)]
fn walk_direction(
    chart: &BranchChart,
    seed_step: &CertifiedStep,
    direction: f64,
    radius: &RadiusLaw,
    budget: &mut Budget,
    weight: &CertifiedPositive,
    bank: &mut NodeBank,
    steps: &mut Vec<CertifiedStep>,
    events: &mut Vec<BlendEvent>,
) -> Result<(), BranchRefusal> {
    let mut current = seed_step.clone();
    loop {
        let (candidate, _delta) =
            match certify_next_step(chart, &current, direction, budget, weight) {
                Ok(step) => step,
                Err(refusal) => {
                    return Err(BranchRefusal {
                        refusal,
                        partial: CertifiedBranch {
                            events: events.clone(),
                            steps: steps.clone(),
                        },
                    })
                }
            };
        let state = match step_state(chart, &candidate) {
            Ok(state) => state,
            Err(refusal) => {
                return Err(BranchRefusal {
                    refusal,
                    partial: CertifiedBranch {
                        events: events.clone(),
                        steps: steps.clone(),
                    },
                })
            }
        };
        let accepted = state.feet_interior
            && state.clearance_clear
            && state.rank_regular
            && state.radius_admissible;
        if accepted {
            let step = CertifiedStep {
                centre: candidate,
                arc: arc_projection(&candidate, &chart.seed_origin, &chart.dir),
                state,
            };
            steps.push(step.clone());
            current = step;
            continue;
        }
        let located = match locate_event(
            chart, &current, &candidate, direction, radius, budget, weight, bank,
        ) {
            Ok(located) => located,
            Err(refusal) => {
                return Err(BranchRefusal {
                    refusal,
                    partial: CertifiedBranch {
                        events: events.clone(),
                        steps: steps.clone(),
                    },
                })
            }
        };
        match located {
            Some(event) => {
                events.push(BlendEvent {
                    kind: event.kind,
                    at: arc_projection(&event.centre, &chart.seed_origin, &chart.dir),
                    node: event.node,
                });
                return Ok(());
            }
            None => {
                return Err(BranchRefusal {
                    refusal: ConstructRefusal::ConditioningBelowThreshold,
                    partial: CertifiedBranch {
                        events: events.clone(),
                        steps: steps.clone(),
                    },
                })
            }
        }
    }
}

/// Certify one continuation step at `direction * BLEND_ARC_STEP` from the last
/// certified step, halving the predictor step until the corrector certifies
/// with strict interior.
fn certify_next_step(
    chart: &BranchChart,
    current: &CertifiedStep,
    direction: f64,
    budget: &mut Budget,
    weight: &CertifiedPositive,
) -> Result<(IBox3, f64), ConstructRefusal> {
    let p = box_midpoint(&current.centre);
    let mut delta = BLEND_ARC_STEP;
    let mut halvings = 0u32;
    loop {
        let predicted = add3_scaled(&p, &chart.dir, direction * delta);
        let candidate = certified_box(predicted, BLEND_CERTIFIED_HALF)?;
        let system = continuation_system(chart, &p, direction, delta);
        match certify_box(&system, candidate, budget, weight)? {
            CertifyOutcome::Proven(boxed) => return Ok((boxed, delta)),
            CertifyOutcome::Disproven | CertifyOutcome::Inconclusive => {
                halvings += 1;
                if halvings > CC_DEPTH_MAX {
                    return Err(ConstructRefusal::ConditioningBelowThreshold);
                }
                delta *= 0.5;
            }
        }
    }
}

/// The continuation system of a step at `dir·δ` from the reference point `p`.
fn continuation_system(
    chart: &BranchChart,
    p: &[f64; 3],
    direction: f64,
    delta: f64,
) -> ChartSystem {
    ChartSystem {
        rows: [
            chart.offset_first,
            chart.offset_second,
            pin_row(p, &chart.dir, direction, delta),
        ],
        jac: ChartJacobian {
            rows: [chart.offset_first.w, chart.offset_second.w, chart.dir],
        },
    }
}

/// The certified step state `Σ` of a certified box (the isolation battery).
fn step_state(chart: &BranchChart, centre: &IBox3) -> Result<WalkState, ConstructRefusal> {
    let iv = box_iv3(centre);
    let feet_interior = feet_interior(chart, &iv)?;
    let clearance_clear = clearances_clear(chart, centre)?;
    let det = continuation_jacobian(chart).det_iv();
    let rank_regular = det.is_finite() && abs_lower(det) > CC_ETA_J;
    let radius_admissible = chart.radius.is_finite() && chart.radius.lo > 0.0;
    Ok(WalkState {
        first_in_contact: true,
        second_in_contact: true,
        first_side: chart.first.side,
        second_side: chart.second.side,
        feet_interior,
        clearance_clear,
        rank_regular,
        radius_admissible,
    })
}

/// The constant step Jacobian of the branch chart.
fn continuation_jacobian(chart: &BranchChart) -> ChartJacobian {
    ChartJacobian {
        rows: [chart.offset_first.w, chart.offset_second.w, chart.dir],
    }
}

/// Whether both certified contact feet are strictly interior to their regions.
fn feet_interior(chart: &BranchChart, centre: &[Interval; 3]) -> Result<bool, ConstructRefusal> {
    for chart_data in [&chart.first, &chart.second] {
        let rows = param_rows(chart_data)?;
        let region = chart_data.region;
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
fn clearances_clear(chart: &BranchChart, centre: &IBox3) -> Result<bool, ConstructRefusal> {
    if chart.exclusions.is_empty() {
        return Ok(true);
    }
    let centre_box = inari_box(centre)?;
    let r_iv = inari_interval(chart.radius.lo, chart.radius.hi)?;
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
    chart: &BranchChart,
    last: &CertifiedStep,
    rejected: &IBox3,
    direction: f64,
    radius: &RadiusLaw,
    budget: &mut Budget,
    weight: &CertifiedPositive,
    bank: &mut NodeBank,
) -> Result<Option<LocatedEvent>, ConstructRefusal> {
    let search = event_search_box(chart, last, rejected, direction)?;
    let last_centre = box_midpoint(&last.centre);
    let mut best: Option<LocatedEvent> = None;
    let mut best_distance: f64 = f64::INFINITY;
    let accept = |kind: EventKind,
                  centre: IBox3,
                  node: Option<TripleContactNode>,
                  best: &mut Option<LocatedEvent>,
                  best_distance: &mut f64| {
        let m = box_midpoint(&centre);
        let s = (m[0] - last_centre[0]) * chart.dir[0]
            + (m[1] - last_centre[1]) * chart.dir[1]
            + (m[2] - last_centre[2]) * chart.dir[2];
        let distance = direction * s;
        if distance > 0.0 && distance < *best_distance {
            *best_distance = distance;
            *best = Some(LocatedEvent { kind, centre, node });
        }
    };
    for support in [&chart.first, &chart.second] {
        let rows = param_rows(support)?;
        let region = support.region;
        for (row, bounds) in [(rows.0, region.0), (rows.1, region.1)] {
            for bound in [bounds.0, bounds.1] {
                let system = trim_system(chart, &row, bound)?;
                let root = certify_root(&system, search, budget, weight)?;
                if let RootOutcome::Root(boxed) = root {
                    accept(EventKind::Trim, boxed, None, &mut best, &mut best_distance);
                }
            }
        }
    }
    for exclusion in &chart.exclusions {
        let tangent = exclusion.tangency(chart.radius.lo);
        let system = tangency_system(chart, &tangent)?;
        let root = certify_root(&system, search, budget, weight)?;
        if let RootOutcome::Root(boxed) = root {
            match exclusion.role {
                ExclusionRole::Collision => {
                    accept(
                        EventKind::Collision,
                        boxed,
                        None,
                        &mut best,
                        &mut best_distance,
                    );
                }
                ExclusionRole::Junction => {
                    let node = certified_node(chart, &boxed, radius, budget, bank)?;
                    accept(
                        EventKind::ThirdFace,
                        boxed,
                        Some(node),
                        &mut best,
                        &mut best_distance,
                    );
                }
            }
        }
    }
    Ok(best)
}

/// The certified triple-contact node at a certified tangency on the branch
/// (P6: solved once per chain and referenced).
fn certified_node(
    chart: &BranchChart,
    tangency: &IBox3,
    radius: &RadiusLaw,
    budget: &mut Budget,
    bank: &mut NodeBank,
) -> Result<TripleContactNode, ConstructRefusal> {
    let junction = match &chart.junction {
        Some(junction) => junction.clone(),
        None => return Err(ConstructRefusal::InvalidInput),
    };
    let unsorted = [
        chart_entry(&chart.first),
        chart_entry(&chart.second),
        chart_entry(&junction),
    ];
    let mut order = [0usize, 1, 2];
    order.sort_by(|a, b| compare_entries(&unsorted[*a], &unsorted[*b]));
    let entries = vec![unsorted[order[0]], unsorted[order[1]], unsorted[order[2]]];
    if let Some(node) = bank.lookup(&entries) {
        return Ok(node);
    }
    let seed_box = node_seed_box(chart, tangency)?;
    let maps = [chart.first.map(), chart.second.map(), junction.map()];
    let regions = [chart.first.region, chart.second.region, junction.region];
    let eps = [chart.first.side, chart.second.side, junction.side];
    let system = ReducedSystem::try_new(
        [maps[order[0]], maps[order[1]], maps[order[2]]],
        [regions[order[0]], regions[order[1]], regions[order[2]]],
        [eps[order[0]], eps[order[1]], eps[order[2]]],
        radius,
        seed_box,
    )
    .map_err(|_| ConstructRefusal::InvalidInput)?;
    match solve_triple_node(&system, budget).map_err(|_| ConstructRefusal::InvalidInput)? {
        TripleNodeOutcome::Node(node) => {
            bank.insert(entries, node.clone());
            Ok(node)
        }
        TripleNodeOutcome::Empty => Err(ConstructRefusal::InvalidInput),
    }
}

/// The canonical geometric entry of a support chart: base, normal, side.
fn chart_entry(chart: &SupportChart) -> [f64; 7] {
    [
        chart.base[0],
        chart.base[1],
        chart.base[2],
        chart.normal[0],
        chart.normal[1],
        chart.normal[2],
        chart.side,
    ]
}

/// Compare two canonical entries lexicographically by total order.
fn compare_entries(a: &[f64; 7], b: &[f64; 7]) -> std::cmp::Ordering {
    for (x, y) in a.iter().zip(b.iter()) {
        let ord = x.total_cmp(y);
        if ord != std::cmp::Ordering::Equal {
            return ord;
        }
    }
    std::cmp::Ordering::Equal
}

impl NodeBank {
    /// Look up a solved node by its canonical entries.
    fn lookup(&self, entries: &[[f64; 7]]) -> Option<TripleContactNode> {
        for (key, node) in &self.solved {
            if key == entries {
                return Some(node.clone());
            }
        }
        None
    }

    /// Insert a solved node under its canonical entries.
    fn insert(&mut self, entries: Vec<[f64; 7]>, node: TripleContactNode) {
        self.solved.push((entries, node));
    }
}

/// The certified node-solve seed box around a certified tangency enclosure.
fn node_seed_box(chart: &BranchChart, tangency: &IBox3) -> Result<IBox4, ConstructRefusal> {
    let margin = BLEND_EVENT_HALF;
    let lo = [
        tangency.lo[0] - margin,
        tangency.lo[1] - margin,
        tangency.lo[2] - margin,
        chart.radius.lo - margin,
    ];
    let hi = [
        tangency.hi[0] + margin,
        tangency.hi[1] + margin,
        tangency.hi[2] + margin,
        chart.radius.hi + margin,
    ];
    ibox4(lo, hi)
}

/// The certified trim system: the offset rows plus one foot-parameter row.
fn trim_system(
    chart: &BranchChart,
    row: &LinEq,
    bound: f64,
) -> Result<ChartSystem, ConstructRefusal> {
    let trim = LinEq {
        w: row.w,
        k: row.k - bound,
    };
    Ok(ChartSystem {
        rows: [chart.offset_first, chart.offset_second, trim],
        jac: ChartJacobian {
            rows: [chart.offset_first.w, chart.offset_second.w, trim.w],
        },
    })
}

/// The certified tangency system: the offset rows plus one tangency row.
fn tangency_system(chart: &BranchChart, tangent: &LinEq) -> Result<ChartSystem, ConstructRefusal> {
    Ok(ChartSystem {
        rows: [chart.offset_first, chart.offset_second, *tangent],
        jac: ChartJacobian {
            rows: [chart.offset_first.w, chart.offset_second.w, tangent.w],
        },
    })
}

/// The certified foot-parameter rows (value form `w·c + k`) of a support.
///
/// The foot of a centre on the support plane has parameter
/// `u = u_0 + ((foot − base)·su)/|su|²` through the Gram inversion of the
/// affine chart; because the foot projection drops the normal component the
/// row is a pure linear function of the centre.
fn param_rows(chart: &SupportChart) -> Result<(LinEq, LinEq), ConstructRefusal> {
    let su = chart.su;
    let sv = chart.sv;
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
    let base = chart.base;
    let ku = chart.origin.0 - (g22 * dot3(&base, &su) - g12 * dot3(&base, &sv)) * inv;
    let kv = chart.origin.1 - (g11 * dot3(&base, &sv) - g12 * dot3(&base, &su)) * inv;
    if !wu.iter().all(|v| v.is_finite())
        || !wv.iter().all(|v| v.is_finite())
        || !ku.is_finite()
        || !kv.is_finite()
    {
        return Err(ConstructRefusal::InvalidInput);
    }
    Ok((LinEq { w: wu, k: ku }, LinEq { w: wv, k: kv }))
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
    weight: &CertifiedPositive,
) -> Result<RootOutcome, ConstructRefusal> {
    match certify_box(system, search, budget, weight)? {
        CertifyOutcome::Disproven => Ok(RootOutcome::Absent),
        CertifyOutcome::Inconclusive => Ok(RootOutcome::Absent),
        CertifyOutcome::Proven(boxed) => {
            let refined = refine_root(system, &boxed, budget, weight)?;
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
    weight: &CertifiedPositive,
) -> Result<IBox3, ConstructRefusal> {
    let guess = newton_guess(system, proven);
    let candidate = certified_box(guess, BLEND_EVENT_HALF)?;
    match certify_box(system, candidate, budget, weight)? {
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
    chart: &BranchChart,
    last: &CertifiedStep,
    rejected: &IBox3,
    direction: f64,
) -> Result<IBox3, ConstructRefusal> {
    let last_centre = box_midpoint(&last.centre);
    let ahead = add3_scaled(&last_centre, &chart.dir, direction * BLEND_EVENT_SLACK);
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for boxed in [&last.centre, rejected] {
        for axis in 0..3 {
            lo[axis] = lo[axis].min(boxed.lo[axis]).min(ahead[axis]);
            hi[axis] = hi[axis].max(boxed.hi[axis]).max(ahead[axis]);
        }
    }
    ibox3(lo, hi)
}

/// A certified box of the given half-width around a centre.
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

/// The certified length-4 reduced box of the S11 node solve.
type IBox4 = crate::kernel::patch::IBox<4>;

/// A certified `IBox4`.
fn ibox4(lo: [f64; 4], hi: [f64; 4]) -> Result<IBox4, ConstructRefusal> {
    IBox4::try_new(lo, hi).map_err(|_| ConstructRefusal::InvalidInput)
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

/// The oriented unit normal of a tangent pair.
fn unit_normal(su: &[f64; 3], sv: &[f64; 3]) -> Result<[f64; 3], ConstructRefusal> {
    let cross = cross3(su, sv);
    let norm = norm3(&cross);
    if !norm.is_finite() || norm == 0.0 {
        return Err(ConstructRefusal::InvalidInput);
    }
    let inv = 1.0 / norm;
    Ok([cross[0] * inv, cross[1] * inv, cross[2] * inv])
}

/// Whether a region is finite and non-degenerate on both axes.
fn region_finite(region: SurfaceRegion) -> bool {
    let ((u0, u1), (v0, v1)) = region;
    u0.is_finite() && u1.is_finite() && v0.is_finite() && v1.is_finite() && u0 <= u1 && v0 <= v1
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
