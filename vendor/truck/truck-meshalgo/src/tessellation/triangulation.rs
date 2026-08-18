#![allow(clippy::many_single_char_names)]
// PR 4A.1 adds sidecar diagnostic types and outcome entry points that are not
// yet consumed by callers. Silence their dead-code until the census examples
// wire them up; remove this `allow` when the diagnostic API is finalized.
#![allow(dead_code, unused)]

use super::diagnosis;
use super::diagnosis::ObservedClosure;
use super::domain::lattice::Axis;
use super::domain::lattice::AxisPeriodStatus;
use super::domain::lattice::CertifiedLattice;
use super::domain::lattice::CollapseWitness;
use super::formal;
use super::source_evidence::{
    BoundId, EdgeUseId, ErasedOrientationMechanism, OrientationEvidence, OrientationOrigin,
    SourceBoundInput, SourceEdgeOrientationEvidence, SourceEdgeUse, SourceEdgeUseInput,
    SourceEvidenceError, SourceFaceInput, SourceFaceOrientationEvidence, SourceVertexKey,
};
use super::validity;
use super::*;
use crate::filters::NormalFilters;
use crate::Point2;
use array_macro::array;
use handles::{FixedDirectedEdgeHandle, FixedUndirectedEdgeHandle, FixedVertexHandle};
use itertools::Itertools;
use rustc_hash::FxHashMap as HashMap;
use rustc_hash::FxHashSet as HashSet;
use serde::Serialize;
use truck_geotrait::algo;

#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;

type SPoint2 = spade::Point2<f64>;
type Cdt = ConstrainedDelaunayTriangulation<SPoint2, (), ConstraintEdgeData>;
std::thread_local! {
    /// Optional document-local source face id, declared face index, and
    /// parameter-space periodic rank for probes.
    /// Which reading of material parity the flood in
    /// `triangulation_into_polymesh_outcome` is to use for the next call.
    ///
    /// A thread-local rather than a parameter because the reading has to reach
    /// a function seven call levels down whose signature is shared with the
    /// legacy path, and because the winding retry is a *second* whole
    /// tessellation of one face rather than a branch inside the first â€” see
    /// the retry stage in the per-face chain for why it has to run there and
    /// not where the contradiction is detected.
    static PARITY_READING: std::cell::Cell<ParityReading> =
        const { std::cell::Cell::new(ParityReading::TraversalParity) };

    static PROBE_FACE_CONTEXT: std::cell::Cell<(Option<u64>, usize, u8)> =
        const { std::cell::Cell::new((None, usize::MAX, 0)) };

    /// Refinement support points: argmax-deviation sample UVs of unsafe
    /// material triangles, collected by the outcome pass and consumed by the
    /// refinement loop's next-pass CDT rebuild. Cleared per face per pass.
    static REFINE_SUPPORT_CELL: std::cell::RefCell<Vec<Point2>> =
        const { std::cell::RefCell::new(Vec::new()) };

    /// Refinement trajectory: the per-pass row the outcome pass records
    /// (triangle count, unsafe count, max exact deviation, deviation excess
    /// sum, worst-triangle provenance). The max-deviation field is the
    /// acceptance functional the refinement loop decides on. Cleared per face
    /// per pass.
    static REFINE_TRAJECTORY: std::cell::RefCell<Vec<RefineTrajectoryRow>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

type MeshedShell = Shell<Point3, PolylineCurve, Option<PolygonMesh>>;
type MeshedCShell = CompressedShell<Point3, PolylineCurve, Option<PolygonMesh>>;

pub(super) trait SP<S>:
    Fn(&S, Point3, Option<(f64, f64)>) -> Option<(f64, f64)> + Parallelizable
{
}
impl<S, F> SP<S> for F where
    F: Fn(&S, Point3, Option<(f64, f64)>) -> Option<(f64, f64)> + Parallelizable
{
}

/// Print taxonomy summary.
pub fn print_taxonomy_summary() {}

pub(super) fn by_search_parameter<S>(
    surface: &S,
    point: Point3,
    hint: Option<(f64, f64)>,
) -> Option<(f64, f64)>
where
    S: MeshableSurface,
{
    surface
        .search_parameter(point, hint, 100)
        .or_else(|| surface.search_parameter(point, None, 100))
}

/// What the projection chain did on one point, for `TRUCK_PROBE_PROJ`.
///
/// Recorded because a bare count of failing points cannot tell the three
/// readings apart that decide what to do about them: a chain that stopped
/// early, a seed route that ran but had only one seed to offer (and so did
/// nothing the plain call had not already done), and a seed route that
/// converged to just outside tolerance â€” which is a tolerance question, not an
/// initialisation one.
#[derive(Clone, Copy, Debug)]
pub(super) struct ProjectionAttempt {
    /// Furthest link of the five-step chain reached, 1-based. 5 is
    /// `by_structural_seeds`.
    pub link: u8,
    /// Seeds `search_parameter_seeds` actually offered. One seed means the
    /// route fired and did nothing different from the plain call.
    pub seeds: usize,
    /// The best seed's residual, or `f64::NAN` if no seed converged at all.
    /// Residuals clustered just above `tol` mean the class is a tolerance
    /// question rather than an initialisation one.
    pub best_residual: f64,
    /// PROJ-002 fields below. Filled only under `TRUCK_PROBE_PROJ_DEEP`.
    pub deep: bool,
    /// Whether each production link returned a parameter. Links are already
    /// tracked; their individual *results* were not.
    pub link_results: [bool; 4],
    /// Whether the caller supplied a previous-UV hint.
    pub had_hint: bool,
    /// Structural seeds actually probed, after the cap.
    pub seeds_tested: usize,
    /// Whether the seed cap truncated the probe.
    pub seed_cap_hit: bool,
    /// The best instrumented nearest search launched from the routes
    /// production already uses â€” the caller's hint and the presearch start.
    pub prod_best: NearestOutcome,
    /// The best instrumented nearest search launched from a structural seed.
    pub seed_best: NearestOutcome,
    /// Which seed produced `seed_best`.
    pub seed_best_index: usize,
    /// Searches abandoned on a singular Jacobian.
    pub degenerate_hits: usize,
    /// Searches that exhausted their trial budget without meeting `near2`.
    pub nonconvergent: usize,
    /// Instrumented searches run for this point.
    pub searches_run: usize,
}

/// What one instrumented nearest-parameter search reached.
///
/// The production call cannot answer this. `search_nearest_parameter` is
/// `newton::solve(..).ok()`, so its `None` means **the iteration did not
/// converge**, not that the nearest point is far away â€” and in a release build
/// `NewtonLog` stores nothing, so the iterate it gave up on is unrecoverable
/// through the existing API. This records the iterate itself.
#[derive(Clone, Copy, Debug)]
pub(super) struct NearestOutcome {
    /// The best parameter the iteration reached.
    pub uv: (f64, f64),
    /// World-space `|surface(u, v) - point|` there. `NAN` if nothing ran.
    pub residual: f64,
    /// Whether Newton met its own `near2` convergence test.
    pub converged: bool,
    /// Whether the iteration stopped on a singular Jacobian.
    pub degenerate: bool,
    /// Whether `uv` lies inside the surface's declared parameter range.
    pub in_domain: bool,
    /// Iterations consumed.
    pub iterations: usize,
}

impl NearestOutcome {
    const NONE: Self = Self {
        uv: (f64::NAN, f64::NAN),
        residual: f64::NAN,
        converged: false,
        degenerate: false,
        in_domain: false,
        iterations: 0,
    };

    fn ran(&self) -> bool {
        self.residual.is_finite()
    }
}

impl ProjectionAttempt {
    const EMPTY: Self = Self {
        link: 0,
        seeds: 0,
        best_residual: f64::NAN,
        deep: false,
        link_results: [false; 4],
        had_hint: false,
        seeds_tested: 0,
        seed_cap_hit: false,
        prod_best: NearestOutcome::NONE,
        seed_best: NearestOutcome::NONE,
        seed_best_index: usize::MAX,
        degenerate_hits: 0,
        nonconvergent: 0,
        searches_run: 0,
    };
}

impl Default for ProjectionAttempt {
    fn default() -> Self {
        Self::EMPTY
    }
}

thread_local! {
    /// The last projection attempt, filled by [`by_search_nearest_parameter`]
    /// and read at the failure site in `PolyBoundaryPiece::try_new`. A
    /// thread-local for the same reason the other probes here are: the two
    /// sites are far apart and the signature between them is shared with paths
    /// that must stay untouched.
    static PROJECTION_ATTEMPT: std::cell::Cell<ProjectionAttempt> =
        const { std::cell::Cell::new(ProjectionAttempt::EMPTY) };
}

pub(super) fn last_projection_attempt() -> ProjectionAttempt {
    PROJECTION_ATTEMPT.with(std::cell::Cell::get)
}

/// Read once: this gate sits on the per-boundary-point path, where an
/// `env::var_os` per call would be a syscall per point across the whole model.
fn projection_probe_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var_os("TRUCK_PROBE_PROJ").is_some() || projection_deep_probe_enabled()
    })
}

/// PROJ-003 Stage A gate, read once for the same reason: the recovery check
/// runs on the per-boundary-point failure path.
fn proj_residual_recovery_enabled_cached() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(diagnosis::proj_residual_recovery_enabled)
}

/// PROJ-003 Stage B gate, read once for the same reason as Stage A's.
fn proj_seed_recovery_enabled_cached() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(diagnosis::proj_seed_recovery_enabled)
}

/// PROJ-003 Stage C gate, read once for the same reason as Stage A's.
fn proj_domain_recovery_enabled_cached() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(diagnosis::proj_domain_recovery_enabled)
}

/// PROJ-003 Stage D gate, read once for the same reason as Stage A's.
fn proj_domain_constrained_enabled_cached() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(diagnosis::proj_domain_constrained_enabled)
}

/// PROJ-002's deep inverse probe. Same reason for the `OnceLock`, and more of
/// it: this one runs a full Newton solve per seed per failing point.
fn projection_deep_probe_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("TRUCK_PROBE_PROJ_DEEP").is_some())
}

/// Structural seeds probed per failing point.
const DEEP_SEED_CAP: usize = 24;
/// Newton iterations per instrumented search, matching the production budget.
const DEEP_TRIALS: usize = 100;
/// Presearch grid, matching `truck-geometry`'s private `PRESEARCH_DIVISION`.
const DEEP_PRESEARCH_DIVISION: usize = 50;
/// Failing points deep-probed per face. The probe is for population-level
/// mechanism classification, not for optimizing every point, and `00000414`
/// alone carries 1,175 NURBS projection faces.
const DEEP_POINT_CAP: usize = 8;

/// The uniform-grid presearch the hintless nearest search starts from.
///
/// Delegates to the shared `algo::surface::presearch` at `truck-geometry`'s
/// division, which is the same start the hintless production link uses, so the
/// iterate recorded here is the iterate that link would have seen.
fn deep_presearch<S>(
    surface: &S,
    point: Point3,
    (u0, u1): (f64, f64),
    (v0, v1): (f64, f64),
) -> (f64, f64)
where
    S: ParametricSurface3D,
{
    algo::surface::presearch(
        surface,
        point,
        ((u0, u1), (v0, v1)),
        DEEP_PRESEARCH_DIVISION,
    )
}

/// The one nearest-parameter solve the tessellation asks, shared with the
/// production chain.
///
/// `probe_nearest` used to be a hand-maintained mirror of `newton::solve` that
/// kept the best iterate; `algo::surface::search_nearest_parameter_outcome` is
/// now that solve, and this maps its outcome back onto the diagnostic type and
/// records the parameter-domain status, which is tessellation-specific.
fn probe_nearest<S>(surface: &S, point: Point3, hint: (f64, f64)) -> NearestOutcome
where
    S: ParametricSurface3D,
{
    let (urange, vrange) = surface.try_range_tuple();
    let in_domain = |(u, v): (f64, f64)| {
        urange.is_none_or(|(a, b)| u >= a && u <= b) && vrange.is_none_or(|(a, b)| v >= a && v <= b)
    };
    let outcome =
        algo::surface::search_nearest_parameter_outcome(surface, point, hint, DEEP_TRIALS);
    NearestOutcome {
        uv: outcome.best,
        residual: outcome.best_residual,
        converged: outcome.converged.is_some(),
        degenerate: outcome.degenerate,
        in_domain: in_domain(outcome.best),
        iterations: 0,
    }
}

/// Run the bounded structural-seed nearest search and retain the best iterate.
///
/// One `probe_nearest` per structural (knot-span) seed, capped at
/// [`DEEP_SEED_CAP`]. The best iterate over the seeds is folded into
/// `attempt.seed_best` and its index into `attempt.seed_best_index`. Shared by
/// the PROJ-002 deep probe and the PROJ-003 Stage B recovery, so the two read
/// the identical candidate.
fn probe_structural_seeds<S>(
    surface: &S,
    point: Point3,
    mut attempt: ProjectionAttempt,
) -> ProjectionAttempt
where
    S: ParametricSurface3D + MeshableSurface,
{
    let seeds = surface.search_parameter_seeds();
    attempt.seeds = attempt.seeds.max(seeds.len());
    attempt.seed_cap_hit = seeds.len() > DEEP_SEED_CAP;
    for (index, seed) in seeds.into_iter().take(DEEP_SEED_CAP).enumerate() {
        let outcome = probe_nearest(surface, point, seed);
        attempt.searches_run += 1;
        attempt.seeds_tested += 1;
        let merged = better_outcome(attempt.seed_best, outcome);
        if merged.residual == outcome.residual && merged.uv == outcome.uv {
            attempt.seed_best_index = index;
        }
        attempt.seed_best = merged;
    }
    attempt
}

/// Classify one failing boundary point from what the deep probe found.
///
/// `prod` is the best over the starts production's own chain already uses; the
/// point of separating it from `seed` is that the two imply different fixes.
/// If a production start already reaches within tolerance, nothing needs new
/// seeds â€” the convergence test threw a good answer away. Only if it does not,
/// and a structural seed does, is this an initialisation problem.
fn classify_projection_point(
    prod: NearestOutcome,
    seed: NearestOutcome,
    tol: f64,
) -> (diagnosis::PointVerdict, diagnosis::NearestRoute) {
    use diagnosis::{NearestRoute, PointVerdict};
    if !tol.is_finite() || tol <= 0.0 {
        return (PointVerdict::Inconclusive, NearestRoute::None);
    }
    let within = |o: &NearestOutcome| o.ran() && o.residual <= tol;
    // A solution inside tolerance *and* inside the declared domain is the only
    // one that would be admissible; one inside tolerance but outside the range
    // is a domain question, not a search question.
    if within(&prod) && prod.in_domain {
        return (PointVerdict::ProductionMiss, NearestRoute::ProductionStart);
    }
    if within(&seed) && seed.in_domain {
        return (PointVerdict::SeedBasinGap, NearestRoute::StructuralSeed);
    }
    if within(&prod) {
        return (
            PointVerdict::DomainOrContractIssue,
            NearestRoute::ProductionStart,
        );
    }
    if within(&seed) {
        return (
            PointVerdict::DomainOrContractIssue,
            NearestRoute::StructuralSeed,
        );
    }
    let best = better_outcome(prod, seed);
    if !best.ran() {
        return (PointVerdict::NoInverseFound, NearestRoute::None);
    }
    let route = if best.residual == seed.residual && seed.ran() {
        NearestRoute::StructuralSeed
    } else {
        NearestRoute::ProductionStart
    };
    // A converged stationary point whose residual exceeds tolerance is a
    // geometric statement about the face: the boundary does not lie on this
    // surface. An unconverged one is not â€” it is only where the iteration got
    // to, so it cannot certify a distance.
    match best.converged {
        true => (PointVerdict::NearestTooFar, route),
        false => (PointVerdict::Inconclusive, route),
    }
}

/// Keep whichever outcome is the better answer: converged beats unconverged,
/// then in-domain beats out, then smaller residual.
fn better_outcome(a: NearestOutcome, b: NearestOutcome) -> NearestOutcome {
    if !b.ran() {
        return a;
    }
    if !a.ran() {
        return b;
    }
    let rank = |o: &NearestOutcome| (o.converged, o.in_domain);
    match rank(&b).cmp(&rank(&a)) {
        std::cmp::Ordering::Greater => b,
        std::cmp::Ordering::Less => a,
        std::cmp::Ordering::Equal if b.residual < a.residual => b,
        std::cmp::Ordering::Equal => a,
    }
}

/// Record the DIAG-002 boundary-projection refusal witness from the last
/// projection attempt, at the refusal site.
///
/// Everything here is already in hand at the refusal: the attempt the
/// production chain just made, the failing world point, the tolerance, and the
/// walk's own counts. No additional geometric work is done.
fn record_projection_refusal_witness(
    attempt: &ProjectionAttempt,
    world_point: Point3,
    tol: f64,
    attempted_samples: usize,
    successful_samples: usize,
    first_failed_index: Option<usize>,
) {
    let best = better_outcome(attempt.prod_best, attempt.seed_best);
    let kind = if !best.ran() {
        diagnosis::ProjectionFailureKind::NoInverseCandidate
    } else if best.degenerate {
        diagnosis::ProjectionFailureKind::SingularEvaluation
    } else if !best.in_domain {
        diagnosis::ProjectionFailureKind::EvaluatorOutOfDomain
    } else if best.converged && best.residual > tol {
        diagnosis::ProjectionFailureKind::ResidualAboveTolerance
    } else {
        diagnosis::ProjectionFailureKind::Other
    };
    let mut residuals = [attempt.prod_best, attempt.seed_best]
        .into_iter()
        .filter(|o| o.ran())
        .map(|o| o.residual);
    let min_residual = residuals.clone().reduce(f64::min);
    let max_residual = residuals.reduce(f64::max);
    diagnosis::record_projection_refusal(diagnosis::ProjectionRefusalWitness {
        kind,
        attempted_samples,
        successful_samples,
        failed_samples: attempted_samples.saturating_sub(successful_samples),
        first_failed_sample: first_failed_index,
        min_residual,
        max_residual,
        acceptance_tolerance: tol.is_finite().then_some(tol),
        source_parameter: None,
        candidate_uv: best.ran().then_some([best.uv.0, best.uv.1]),
        world_point: world_point.is_finite().then_some([
            world_point.x,
            world_point.y,
            world_point.z,
        ]),
        periodic_candidate_count: None,
    });
}

/// The residual-certified admission contract, on one best candidate.
///
/// Split from the per-stage wrappers so the mechanism tests can exercise the
/// contract directly: a candidate is admitted only if its UV is finite, lies
/// inside the surface's declared parameter range, evaluates to a finite world
/// point, and re-evaluates to a world residual within the caller tolerance.
/// No tolerance widening and no domain normalization happen here.
fn admit_outcome<S: PreMeshableSurface>(
    surface: &S,
    point: Point3,
    tol: f64,
    best: NearestOutcome,
) -> Option<((f64, f64), f64)> {
    if !best.ran() {
        return None;
    }
    let (u, v) = best.uv;
    if !u.is_finite() || !v.is_finite() {
        return None;
    }
    // Re-verify the domain here, where the surface is in hand. A `None` axis
    // range means that axis is unbounded and therefore cannot be out of domain.
    let (urange, vrange) = surface.try_range_tuple();
    let inside = urange.is_none_or(|(a, b)| u >= a && u <= b)
        && vrange.is_none_or(|(a, b)| v >= a && v <= b);
    if !inside {
        return None;
    }
    let evaluated = surface.subs(u, v);
    if !evaluated.x.is_finite() || !evaluated.y.is_finite() || !evaluated.z.is_finite() {
        return None;
    }
    let residual = evaluated.distance(point);
    if !residual.is_finite() || residual > tol {
        return None;
    }
    Some(((u, v), residual))
}

/// The Stage A admission contract, without the environment gate.
///
/// Split from [`residual_certified_recovery`] so the mechanism tests can
/// exercise the contract directly instead of depending on a process-global
/// cached environment flag.
fn residual_certified_admission<S: PreMeshableSurface>(
    surface: &S,
    point: Point3,
    tol: f64,
    attempt: ProjectionAttempt,
) -> Option<((f64, f64), f64)> {
    admit_outcome(surface, point, tol, attempt.prod_best)
}

/// PROJ-003 Stage A: residual-certified admission of a production-start
/// iterate the legacy projection chain rejected.
///
/// The legacy chain treats `search_nearest_parameter(..) == newton::solve(..).ok()`
/// as the projection answer, so its `None` means only that Newton's `near2`
/// convergence test was not met â€” not that the surface is far from the boundary
/// point. PROJ-002 showed ~two thirds of the `BoundaryProjectionFailed`
/// population is exactly that: a start production already uses reaches a
/// finite, in-domain parameter whose world residual is within the caller's
/// tolerance, and Newton's convergence gate threw it away. This admits such a
/// candidate under the contract in [`residual_certified_admission`]:
///
/// - runs only where the legacy chain returned `None` for this point
/// - the candidate comes only from the caller's hint or the hintless presearch
///   start â€” the starts the production chain itself already uses
/// - Newton's `near2` condition is explicitly NOT required
///
/// It is refinement-only by construction: a face that projected through the
/// legacy chain is byte-identical, because this runs only after that chain
/// returned `None`. Returns the admitted parameter and its certified residual.
fn residual_certified_recovery<S: PreMeshableSurface>(
    surface: &S,
    point: Point3,
    tol: f64,
    attempt: ProjectionAttempt,
) -> Option<((f64, f64), f64)> {
    if !proj_residual_recovery_enabled_cached() {
        return None;
    }
    residual_certified_admission(surface, point, tol, attempt)
}

/// PROJ-003 Stage B admission contract, without the environment gate.
///
/// The same contract Stage A enforces, but the candidate is the best iterate of
/// the bounded nearest searches launched from the surface's structural
/// (knot-span) seeds, which Stage A does not consider. Runs only after Stage A
/// refused the point.
fn residual_certified_seed_admission<S: PreMeshableSurface>(
    surface: &S,
    point: Point3,
    tol: f64,
    attempt: ProjectionAttempt,
) -> Option<((f64, f64), f64)> {
    admit_outcome(surface, point, tol, attempt.seed_best)
}

/// PROJ-003 Stage B: residual-certified admission of a structural-seed
/// nearest iterate the legacy projection chain rejected.
///
/// PROJ-002's deep probe found `SeedBasinGap` faces: the production starts
/// (the caller's hint and the hintless presearch) do not reach a within-tol
/// solution, but a nearest search started from a structural knot-span seed
/// does. The legacy chain's seed link runs `search_parameter` per seed, which
/// throws away any iterate Newton's `near2` test did not certify; Stage B
/// promotes the diagnostic nearest-search into production, admitting the
/// bounded seed search's best iterate under the same contract as Stage A.
///
/// Refinement-only by construction: it runs only where the whole legacy chain
/// (including the seed link) returned `None` *and* Stage A refused the point,
/// so it can change nothing that already projected or was recovered by A.
/// Returns the admitted parameter and its certified residual.
fn residual_certified_seed_recovery<S: PreMeshableSurface>(
    surface: &S,
    point: Point3,
    tol: f64,
    attempt: ProjectionAttempt,
) -> Option<((f64, f64), f64)> {
    if !proj_seed_recovery_enabled_cached() {
        return None;
    }
    residual_certified_seed_admission(surface, point, tol, attempt)
}

/// The relative scale at which an out-of-range coordinate counts as a
/// boundary-epsilon candidate for Stage C classification.
///
/// A coordinate within this fraction of the declared span outside the range is
/// treated as numerical error at a closed boundary; beyond it, the candidate is
/// a genuine domain disagreement. Deliberately conservative and scale-aware
/// (relative to the parameter extent, not an absolute UV epsilon).
const DOMAIN_BOUNDARY_EPSILON_REL: f64 = 1.0e-6;

/// Classify why a within-tolerance candidate lies outside the declared range.
///
/// `best` must be a candidate with a finite residual `<= tol` whose UV is
/// outside the declared parameter range (a `DomainOrContractIssue` point).
/// Returns [`diagnosis::DomainRecoveryClass::Unknown`] for any input that is
/// not such a point.
fn classify_domain_point(
    best: NearestOutcome,
    tol: f64,
    urange: Option<(f64, f64)>,
    vrange: Option<(f64, f64)>,
    lattice: &CertifiedLattice,
) -> diagnosis::DomainRecoveryClass {
    use diagnosis::DomainRecoveryClass as C;
    if !best.ran() || !best.residual.is_finite() || best.residual > tol || best.in_domain {
        return C::Unknown;
    }
    let (u, v) = best.uv;
    // Classify one axis. Returns `None` when the coordinate is inside range (so
    // that axis is not the question), and the axis's class otherwise.
    let class_axis = |coord: f64, range: Option<(f64, f64)>, gen: Option<f64>| -> Option<C> {
        let (lo, hi) = range?;
        if coord >= lo && coord <= hi {
            return None;
        }
        if let Some(period) = gen {
            if period.is_finite() && period > 0.0 {
                let mid = 0.5 * (lo + hi);
                let shifted = coord - ((coord - mid) / period).round() * period;
                if shifted >= lo && shifted <= hi {
                    return Some(C::PeriodicEquivalent);
                }
            }
        }
        let span = hi - lo;
        if span <= 0.0 || !span.is_finite() {
            return Some(C::TrueOutOfDomain);
        }
        let outside = if coord > hi { coord - hi } else { lo - coord };
        let frac = outside / span;
        if frac <= DOMAIN_BOUNDARY_EPSILON_REL {
            Some(C::BoundaryEpsilon)
        } else if frac >= 1.0 {
            Some(C::RepresentationRangeMismatch)
        } else {
            Some(C::TrueOutOfDomain)
        }
    };
    let ua = class_axis(u, urange, lattice.u_generator());
    let va = class_axis(v, vrange, lattice.v_generator());
    match (ua, va) {
        (None, None) => C::Unknown,
        (Some(a), None) => a,
        (None, Some(b)) => b,
        (Some(a), Some(b)) => {
            // With two out-of-range axes the class is the more privileged one:
            // a periodically reducible axis names a periodic equivalent, a
            // boundary epsilon names numerical error, and the rest are genuine
            // domain disagreements.
            let rank = |c: C| match c {
                C::PeriodicEquivalent => 3,
                C::BoundaryEpsilon => 2,
                C::RepresentationRangeMismatch => 1,
                _ => 0,
            };
            if rank(a) >= rank(b) {
                a
            } else {
                b
            }
        }
    }
}

/// PROJ-003 Stage C: recover a within-tolerance candidate outside the declared
/// parameter range, through principled domain/periodicity semantics only.
///
/// The `DomainOrContractIssue` population is not one mechanism. This admits a
/// candidate only when a certified periodic axis maps it back into the declared
/// range by an integer number of periods (`PeriodicEquivalent`), or when it
/// sits microscopically outside a closed boundary and clamping to the boundary
/// still re-evaluates within the caller tolerance (`BoundaryEpsilon`). Every
/// other class is diagnostic-only. Every admission is re-certified under the
/// existing contract: finite UV, in-domain after the transformation, finite
/// evaluation, `|S(u, v) - P| <= tol`. Returns the admitted parameter, its
/// certified residual, and the class that justified the admission.
fn residual_certified_domain_recovery<S: PreMeshableSurface>(
    surface: &S,
    point: Point3,
    tol: f64,
    attempt: ProjectionAttempt,
    lattice: &CertifiedLattice,
) -> Option<((f64, f64), f64, diagnosis::DomainRecoveryClass)> {
    use diagnosis::DomainRecoveryClass as C;
    if !proj_domain_recovery_enabled_cached() {
        return None;
    }
    // The within-tolerance candidate that made this a domain question.
    let candidate = if attempt.prod_best.ran() && attempt.prod_best.residual <= tol {
        attempt.prod_best
    } else if attempt.seed_best.ran() && attempt.seed_best.residual <= tol {
        attempt.seed_best
    } else {
        return None;
    };
    if !candidate.in_domain {
        let (urange, vrange) = surface.try_range_tuple();
        let class = classify_domain_point(candidate, tol, urange, vrange, lattice);
        let (u, v) = candidate.uv;
        let transform = |coord: f64, range: Option<(f64, f64)>, gen: Option<f64>| -> Option<f64> {
            let (lo, hi) = range?;
            if coord >= lo && coord <= hi {
                return Some(coord);
            }
            if let Some(period) = gen {
                if period.is_finite() && period > 0.0 {
                    let mid = 0.5 * (lo + hi);
                    let shifted = coord - ((coord - mid) / period).round() * period;
                    if shifted >= lo && shifted <= hi {
                        return Some(shifted);
                    }
                }
            }
            None
        };
        let (nu, nv) = match class {
            // Normalize every out-of-range axis by certified periods. An axis
            // that is out of range and *not* periodically reducible refuses the
            // whole admission â€” Stage C never mixes a periodic shift with a
            // clamp or an epsilon to force a candidate in.
            C::PeriodicEquivalent => {
                let nu = transform(u, urange, lattice.u_generator())?;
                let nv = transform(v, vrange, lattice.v_generator())?;
                (nu, nv)
            }
            // Clamp each out-of-range coordinate to the closed boundary, then
            // re-certify below.
            C::BoundaryEpsilon => {
                let (urange, vrange) = (urange?, vrange?);
                let nu = if u < urange.0 {
                    urange.0
                } else if u > urange.1 {
                    urange.1
                } else {
                    u
                };
                let nv = if v < vrange.0 {
                    vrange.0
                } else if v > vrange.1 {
                    vrange.1
                } else {
                    v
                };
                (nu, nv)
            }
            _ => return None,
        };
        if !nu.is_finite() || !nv.is_finite() {
            return None;
        }
        let evaluated = surface.subs(nu, nv);
        if !evaluated.x.is_finite() || !evaluated.y.is_finite() || !evaluated.z.is_finite() {
            return None;
        }
        let residual = evaluated.distance(point);
        if !residual.is_finite() || residual > tol {
            return None;
        }
        Some(((nu, nv), residual, class))
    } else {
        None
    }
}

/// Grid divisions per axis of the Stage D constrained inverse.
///
/// The grid is evaluated only on the projection-failure path, so its cost is
/// paid by faces that would otherwise fail. `64` resolves 1/64 of a declared
/// span, which is fine for both boundary-adjacent roots and the wrapped
/// representatives of a surface whose evaluator repeats with the span.
const DOMAIN_CONSTRAINED_GRID: usize = 64;
/// Clamped-Newton iterations after the grid.
const DOMAIN_CONSTRAINED_ITERS: usize = 64;

/// PROJ-003 Stage D: the constrained inverse over the declared parameter range.
///
/// The unconstrained projection chain (and Stages A-C) operate on the
/// *unconstrained* Newton iterate. A boundary point whose nearest root lies on
/// an extrapolated branch — a spline that genuinely wraps despite an open
/// declaration, or a degenerate edge — can come back out of the declared range
/// even though an in-range root exists on the same surface. This runs a fresh
/// inverse **restricted to `D_accept`**: a grid over the declared range, then
/// clamped Newton refinement pinned to the box, re-certified under the
/// residual contract (finite UV, finite evaluation, `|S(u, v) - P| <= tol`).
///
/// Refinement-only by construction: it runs only where the whole legacy chain
/// and Stages A-C refused the point, so it changes nothing that already
/// projected or was recovered. It never clamps a solver answer into range and
/// never modulos a non-periodic axis — it searches inside the range in the
/// first place, so an admitted coordinate is genuinely in `D_accept`.
fn constrained_domain_inverse<S: PreMeshableSurface>(
    surface: &S,
    point: Point3,
    tol: f64,
) -> Option<((f64, f64), f64)> {
    let (urange, vrange) = surface.try_range_tuple();
    let (u0, u1) = urange?;
    let (v0, v1) = vrange?;
    if !(u1 > u0)
        || !(v1 > v0)
        || !u0.is_finite()
        || !u1.is_finite()
        || !v0.is_finite()
        || !v1.is_finite()
    {
        return None;
    }
    // Dense grid over the declared range.
    let mut best = (f64::INFINITY, 0.0f64, 0.0f64);
    for i in 0..=DOMAIN_CONSTRAINED_GRID {
        let u = u0 + (u1 - u0) * (i as f64 / DOMAIN_CONSTRAINED_GRID as f64);
        for j in 0..=DOMAIN_CONSTRAINED_GRID {
            let v = v0 + (v1 - v0) * (j as f64 / DOMAIN_CONSTRAINED_GRID as f64);
            let evaluated = surface.subs(u, v);
            if !evaluated.x.is_finite() || !evaluated.y.is_finite() || !evaluated.z.is_finite() {
                continue;
            }
            let residual = evaluated.distance(point);
            if residual < best.0 {
                best = (residual, u, v);
            }
        }
    }
    if !best.0.is_finite() {
        return None;
    }
    // Clamped Newton refinement from the best cell, pinned to the box.
    let mut u = best.1;
    let mut v = best.2;
    let mut best_ref = (best.0, u, v);
    for _ in 0..DOMAIN_CONSTRAINED_ITERS {
        let diff = surface.subs(u, v) - point;
        let uder = surface.uder(u, v);
        let vder = surface.vder(u, v);
        let (uu, uv, vv) = (uder.dot(uder), uder.dot(vder), vder.dot(vder));
        let jac = Matrix2::new(uu, uv, uv, vv);
        let val = Vector2::new(uder.dot(diff), vder.dot(diff));
        let Some(step) = jac.invert() else {
            break;
        };
        let next = Vector2::new(u, v) - step * val;
        u = next.x.clamp(u0, u1);
        v = next.y.clamp(v0, v1);
        let evaluated = surface.subs(u, v);
        if !evaluated.x.is_finite() || !evaluated.y.is_finite() || !evaluated.z.is_finite() {
            break;
        }
        let residual = evaluated.distance(point);
        if residual < best_ref.0 {
            best_ref = (residual, u, v);
        }
    }
    let (residual, u, v) = best_ref;
    if !residual.is_finite() || residual > tol {
        return None;
    }
    Some(((u, v), residual))
}

/// The Stage D recovery wrapper, gated like the other recovery stages.
fn constrained_domain_recovery<S: PreMeshableSurface>(
    surface: &S,
    point: Point3,
    tol: f64,
) -> Option<((f64, f64), f64)> {
    if !proj_domain_constrained_enabled_cached() {
        return None;
    }
    constrained_domain_inverse(surface, point, tol)
}

#[cfg(test)]
mod constrained_domain_inverse_tests {
    use super::constrained_domain_inverse;
    use truck_geometry::prelude::{Plane, Point3};

    fn plane() -> Plane {
        Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        )
    }

    /// Positive: an on-surface point inside the declared range is found by the
    /// constrained inverse and returned with a certified residual, pinned to
    /// the box.
    #[test]
    fn in_range_point_is_found() {
        let s = plane();
        let p = Point3::new(0.25, 0.75, 0.0);
        let got = constrained_domain_inverse(&s, p, 1.0e-6);
        let ((u, v), residual) = got.expect("an on-plane point is found in range");
        assert!((u - 0.25).abs() < 1.0e-6);
        assert!((v - 0.75).abs() < 1.0e-6);
        assert!(residual <= 1.0e-6);
    }

    /// Positive: a point on the domain boundary (the `u = 1` edge) is found in
    /// range; the returned coordinate is never pushed outside `D_accept`.
    #[test]
    fn boundary_edge_point_is_found_in_range() {
        let s = plane();
        let p = Point3::new(1.0, 0.5, 0.0);
        let got = constrained_domain_inverse(&s, p, 1.0e-6);
        let ((u, v), residual) = got.expect("a boundary-edge point is found");
        assert!(u >= 0.0 && u <= 1.0, "u must stay in D_accept, got {u}");
        assert!(v >= 0.0 && v <= 1.0, "v must stay in D_accept, got {v}");
        assert!(residual <= 1.0e-6);
    }

    /// Negative: no in-range solution within tolerance, so the constrained
    /// inverse refuses and the face stays out of domain.
    #[test]
    fn no_in_range_root_refuses() {
        let s = plane();
        // 0.5 off the plane in z: no point on the plane is within tol.
        let p = Point3::new(5.0, 5.0, 0.5);
        assert!(constrained_domain_inverse(&s, p, 1.0e-6).is_none());
    }

    /// Negative: an in-range point whose certified residual exceeds the caller
    /// tolerance is refused (ResidualAboveTolerance semantics preserved).
    #[test]
    fn above_tolerance_refuses() {
        let s = plane();
        let p = Point3::new(0.5, 0.5, 1.0e-3);
        assert!(constrained_domain_inverse(&s, p, 1.0e-6).is_none());
    }
}

pub(super) fn by_search_nearest_parameter<S>(
    surface: &S,
    point: Point3,
    hint: Option<(f64, f64)>,
) -> Option<(f64, f64)>
where
    S: RobustMeshableSurface,
{
    let mut attempt = ProjectionAttempt::default();
    attempt.had_hint = hint.is_some();
    let mut result = None;
    if projection_probe_enabled() {
        // The instrumented chain: identical calls in an identical order, with the
        // link recorded as it is reached. Kept as a separate arm so the production
        // path stays one expression and cannot drift from it silently.
        for link in 1..=4u8 {
            attempt.link = link;
            result = match link {
                1 => surface.search_parameter(point, hint, 100),
                2 => surface.search_parameter(point, None, 100),
                3 => surface.search_nearest_parameter(point, hint, 100),
                _ => surface.search_nearest_parameter(point, None, 100),
            };
            attempt.link_results[usize::from(link) - 1] = result.is_some();
            if result.is_some() {
                break;
            }
        }
        if result.is_none() {
            attempt.link = 5;
            let seeds = surface.search_parameter_seeds();
            attempt.seeds = seeds.len();
            result = by_structural_seeds(surface, point, hint);
            // The best residual over the seeds, recomputed rather than threaded out
            // of `by_structural_seeds` so that route stays exactly as it ships.
            attempt.best_residual = seeds
                .into_iter()
                .filter_map(|seed| surface.search_parameter(point, seed, 100))
                .map(|uv| surface.subs(uv.0, uv.1).distance(point))
                .fold(f64::INFINITY, f64::min);
            if attempt.best_residual.is_infinite() {
                attempt.best_residual = f64::NAN;
            }
        }
    } else {
        // Production path: keep the exact legacy expression. Identical calls in
        // an identical order, so a face that projects today gets the identical
        // parameter. The thread-local is still filled below, so the
        // residual-certified recovery can run where the chain returns `None`.
        result = surface
            .search_parameter(point, hint, 100)
            .or_else(|| surface.search_parameter(point, None, 100))
            .or_else(|| surface.search_nearest_parameter(point, hint, 100))
            .or_else(|| surface.search_nearest_parameter(point, None, 100))
            // Last, so it is reached only where every existing attempt returned
            // `None` â€” which is exactly the population that becomes
            // `BoundaryProjectionFailed`. A face that projects today projects
            // through the identical chain and gets the identical parameter.
            .or_else(|| by_structural_seeds(surface, point, hint));
    }
    // PROJ-003 Stage A. On a point the whole production chain has already
    // rejected, record the best iterate reached from the starts production
    // itself uses â€” the caller's hint and the hintless presearch start. This
    // runs whenever the residual-certified recovery is enabled, because the
    // recovery consumes it, and additionally under the deep probe, whose
    // witness needs it. It never changes what the chain above returned.
    if result.is_none()
        && (proj_residual_recovery_enabled_cached() || projection_deep_probe_enabled())
    {
        if let Some(hint) = hint {
            attempt.prod_best =
                better_outcome(attempt.prod_best, probe_nearest(surface, point, hint));
            attempt.searches_run += 1;
        }
        // The hintless route's start. `search_nearest_parameter(point, None, _)`
        // presearches a uniform grid over the declared range and iterates from
        // the best cell; reproduced here so its iterate is visible too.
        if let (Some(urange), Some(vrange)) = surface.try_range_tuple() {
            let presearch = deep_presearch(surface, point, urange, vrange);
            attempt.prod_best =
                better_outcome(attempt.prod_best, probe_nearest(surface, point, presearch));
            attempt.searches_run += 1;
        }
    }
    // PROJ-003 Stage B. On a point the whole production chain (including the
    // spline-seed link) has rejected, run the bounded structural-seed *nearest*
    // search and retain the best iterate, so the Stage B recovery has a
    // candidate. The deep probe needs the identical iterate to classify, so it
    // runs here too whenever either gate is open.
    if result.is_none() && (proj_seed_recovery_enabled_cached() || projection_deep_probe_enabled())
    {
        attempt = probe_structural_seeds(surface, point, attempt);
    }
    // PROJ-002. Only on a point production has already rejected, and only under
    // its own gate: this labels the attempt for classification. The structural
    // seed probing above already ran wherever the deep probe runs.
    if result.is_none() && projection_deep_probe_enabled() {
        attempt.deep = true;
        for outcome in [attempt.prod_best, attempt.seed_best] {
            if outcome.degenerate {
                attempt.degenerate_hits += 1;
            } else if outcome.ran() && !outcome.converged {
                attempt.nonconvergent += 1;
            }
        }
    }
    PROJECTION_ATTEMPT.with(|cell| cell.set(attempt));
    result
}

/// Retry the parameter inverse from the starts the surface's own structure
/// suggests.
///
/// The chain above fails as a *numerical* matter, not a geometric one: it runs
/// a Newton iteration from a single start â€” a caller's hint, or the best cell
/// of a uniform presearch grid â€” and a single start is not enough on a
/// piecewise surface whose pieces the grid does not see. `search_parameter_seeds`
/// supplies one start per knot span, so every polynomial piece gets its own
/// attempt. Only the initialisation changes; the iteration is the same one.
///
/// This returns a parameter, not a verdict. A returned parameter is still
/// subject to the caller's incidence check â€” a nearest point is not an
/// incidence â€” so nothing is admitted here that the pipeline would not have
/// admitted from any other start.
fn by_structural_seeds<S>(
    surface: &S,
    point: Point3,
    hint: Option<(f64, f64)>,
) -> Option<(f64, f64)>
where
    S: MeshableSurface,
{
    if !diagnosis::spline_seed_recovery_enabled() {
        return None;
    }
    let seeds = surface.search_parameter_seeds();
    if seeds.is_empty() {
        return None;
    }
    let mut best: Option<((f64, f64), f64, f64)> = None;
    for seed in seeds {
        let Some(uv) = surface.search_parameter(point, seed, 100) else {
            continue;
        };
        let residual = surface.subs(uv.0, uv.1).distance(point);
        // Distance from the hint, in parameter space. Among starts that
        // converge equally well this is what keeps the boundary walk monotone:
        // a spline can carry the same 3D point in more than one span, and
        // taking whichever one happened to converge first would step the
        // traversal across the domain.
        let drift = match hint {
            Some((u0, v0)) => (uv.0 - u0).hypot(uv.1 - v0),
            None => 0.0,
        };
        let better = match best {
            None => true,
            Some((_, best_residual, best_drift)) => {
                if residual < best_residual * SEED_RESIDUAL_TIE
                    && best_residual < residual * SEED_RESIDUAL_TIE
                {
                    drift < best_drift
                } else {
                    residual < best_residual
                }
            }
        };
        if better {
            best = Some((uv, residual, drift));
        }
    }
    best.map(|(uv, _, _)| uv)
}

/// Within this factor two seeds' residuals are the same answer, and the choice
/// between them is made on traversal continuity instead.
///
/// Both are converged solutions of the same equation; their residuals differ
/// only by where the iteration stopped. Comparing them exactly would let
/// floating-point noise decide which span the boundary walk continues in.
const SEED_RESIDUAL_TIE: f64 = 1.0 + 1.0e-6;

/// Tessellates faces
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn shell_tessellation<'a, C, S>(
    shell: &Shell<Point3, C, S>,
    tol: f64,
    sp: impl SP<S>,
    lattice_of: impl Fn(&S) -> CertifiedLattice + Parallelizable,
) -> MeshedShell
where
    C: PolylineableCurve + 'a,
    S: PreMeshableSurface + 'a,
{
    let vmap: HashMap<_, _> = shell
        .vertex_par_iter()
        .map(|v| (v.id(), v.mapped(Point3::clone)))
        .collect();
    let eset: HashMap<_, _> = shell.edge_par_iter().map(move |e| (e.id(), e)).collect();
    let edge_map: HashMap<_, _> = eset
        .into_par_iter()
        .map(move |(id, edge)| {
            let v0 = vmap.get(&edge.absolute_front().id()).unwrap();
            let v1 = vmap.get(&edge.absolute_back().id()).unwrap();
            let curve = edge.curve();
            let poly = PolylineCurve::from_curve(&curve, curve.evaluation_range(), tol);
            (id, Edge::debug_new(v0, v1, poly))
        })
        .collect();
    let create_edge = |edge: &Edge<Point3, C>| -> Edge<_, _> {
        let new_edge = edge_map.get(&edge.id()).unwrap();
        match edge.orientation() {
            true => new_edge.clone(),
            false => new_edge.inverse(),
        }
    };
    let create_boundary =
        |wire: &Wire<Point3, C>| -> Wire<_, _> { wire.edge_iter().map(create_edge).collect() };
    let create_face = move |face: &Face<Point3, C, S>| -> Face<_, _, _> {
        let wires: Vec<_> = face
            .absolute_boundaries()
            .iter()
            .map(create_boundary)
            .collect();
        let lattice = lattice_of(&face.surface());
        shell_create_polygon(
            &face.surface(),
            wires,
            face.orientation(),
            tol,
            &sp,
            &lattice,
        )
    };
    shell.face_par_iter().map(create_face).collect()
}

/// Tessellates faces
#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn shell_tessellation_single_thread<'a, C, S>(
    shell: &'a Shell<Point3, C, S>,
    tol: f64,
    sp: impl SP<S>,
    lattice_of: impl Fn(&S) -> CertifiedLattice + Parallelizable,
) -> MeshedShell
where
    C: PolylineableCurve + 'a,
    S: PreMeshableSurface + 'a,
{
    use truck_base::entry_map::FxEntryMap as EntryMap;
    use truck_topology::Vertex as TVertex;
    let mut vmap = EntryMap::new(
        move |v: &TVertex<Point3>| v.id(),
        move |v| v.mapped(Point3::clone),
    );
    let mut edge_map = EntryMap::new(
        move |edge: &'a Edge<Point3, C>| edge.id(),
        move |edge| {
            let vf = edge.absolute_front();
            let v0 = vmap.entry_or_insert(vf).clone();
            let vb = edge.absolute_back();
            let v1 = vmap.entry_or_insert(vb).clone();
            let curve = edge.curve();
            let poly = PolylineCurve::from_curve(&curve, curve.evaluation_range(), tol);
            Edge::debug_new(&v0, &v1, poly)
        },
    );
    let mut create_edge = move |edge: &'a Edge<Point3, C>| -> Edge<_, _> {
        let new_edge = edge_map.entry_or_insert(edge);
        match edge.orientation() {
            true => new_edge.clone(),
            false => new_edge.inverse(),
        }
    };
    let mut create_boundary = move |wire: &'a Wire<Point3, C>| -> Wire<_, _> {
        wire.edge_iter().map(&mut create_edge).collect()
    };
    let create_face = move |face: &'a Face<Point3, C, S>| -> Face<_, _, _> {
        let wires: Vec<_> = face
            .absolute_boundaries()
            .iter()
            .map(&mut create_boundary)
            .collect();
        let lattice = lattice_of(&face.surface());
        shell_create_polygon(
            &face.surface(),
            wires,
            face.orientation(),
            tol,
            &sp,
            &lattice,
        )
    };
    shell.face_iter().map(create_face).collect()
}

/// A meshed shell together with why each face that failed did so.
///
/// **G8.** The failure vector is positionally aligned with `shell.faces`:
/// `face_failures[i]` explains `shell.faces[i]`, and is `None` exactly when
/// that face tessellated. It is returned *with* the shell rather than emitted
/// or logged, so a caller cannot consume the mesh while ignoring the reason â€”
/// which is what made the previous empty-mesh convention lossy.
///
/// Each face's own identity remains on `CompressedFace::provenance`, so a
/// failure can be reported against a source entity rather than an index.
#[derive(Clone, Debug)]
pub struct MeshedShellOutcome {
    /// The meshed shell, shaped exactly as the legacy path produced it.
    pub shell: MeshedCShell,
    /// Why each face failed, positionally aligned with `shell.faces`.
    pub face_failures: Vec<Option<TessellationFailure>>,
    /// DIAG-002: the finalized structured diagnostic record for each failed
    /// face, positionally aligned with `shell.faces`. `None` for a face that
    /// tessellated.
    pub face_diagnoses: Vec<Option<diagnosis::FaceDiagnosticRecord>>,
    /// DIAG-001: what the formal cylinder-band fallback did on each face,
    /// positionally aligned with `shell.faces`. `None` for every face the
    /// fallback was not eligible for, which includes every face when the gate
    /// is closed.
    pub band_attempts: Vec<Option<CylinderBandAttempt>>,
    /// What the formal conical essential-band route did on each face,
    /// positionally aligned with `shell.faces`.
    ///
    /// A second vector rather than a widened first one, because the two routes
    /// have genuinely different vocabularies: the cylinder's exit set names a
    /// nonconformant-source repair this cell has none of, and this cell's names
    /// nappe and apex obligations the cylinder has none of. Flattening them
    /// would force a reconciliation to guess which cell a shared tag came from.
    /// Both are `None` on any face neither route was eligible for.
    pub cone_band_attempts: Vec<Option<ConeBandAttempt>>,
    /// What the torus annulus route did on each face, positionally aligned
    /// with `shell.faces`.
    ///
    /// A third vector rather than a widened first two, for the same reason:
    /// the torus route has its own vocabulary of typed exits
    /// ([`formal::TorusAnnulusExit`]) and its own conformance tag
    /// ([`formal::torus_cell::ConformanceTag`]), neither of which is
    /// reconcilable with the cylinder or cone cell's exit types. `None` on
    /// any face the torus route was not eligible for.
    pub torus_band_attempts: Vec<Option<TorusAnnulusAttempt>>,
}

/// What the cylinder-band fallback did on one eligible face.
///
/// Deliberately not a new taxonomy: the unrecovered arm carries
/// [`formal::cylinder_band::BandExit`] unchanged, which already names the stage
/// the attempt left from. "Attempted" is `Some(_)`; "recovered" is the first
/// variant.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CylinderBandAttempt {
    /// `run_cylinder_band` returned a validated annular mesh, and that mesh
    /// replaced the preserved legacy failure.
    Recovered {
        /// Triangles in the validated annulus.
        triangles: usize,
        /// Whether the source that produced it was conformant, or was
        /// repaired by a named nonconformant normalization. A recovery from a
        /// malformed file is still a recovery, and is still not a clean read;
        /// carrying the distinction here is what keeps a census able to say
        /// which it was.
        conformance: formal::cylinder_band::SourceConformance,
    },
    /// `run_cylinder_band` returned a typed exit, and the original legacy
    /// failure was preserved unchanged.
    Refused(formal::cylinder_band::BandExit),
}

/// What the conical essential-band route did on one eligible face.
///
/// The same shape as [`CylinderBandAttempt`], and deliberately not the same
/// type: the unrecovered arm carries [`formal::cone_band::ConicalBandExit`]
/// unchanged, which names this cell's own obligations â€” same nappe, apex
/// exclusion, carrier order â€” and the recovered arm carries what the *source*
/// declared about outer-bound standing rather than a conformance repair,
/// because this cell has no repair to report.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConeBandAttempt {
    /// `run_conical_essential_band` returned a validated annular mesh, and
    /// that mesh replaced the preserved legacy failure.
    Recovered {
        /// Triangles in the validated annulus.
        triangles: usize,
        /// Which nappe the band was certified on.
        nappe: formal::cone::Nappe,
        /// What the source declared about outer-bound standing. Retained as
        /// provenance; the material region did not come from it.
        standing: formal::cone_band::ConicalSourceStanding,
    },
    /// `run_conical_essential_band` returned a typed exit, and the original
    /// legacy failure was preserved unchanged.
    Refused(formal::cone_band::ConicalBandExit),
}

/// What the torus annulus route did on one eligible face.
///
/// The same shape as [`CylinderBandAttempt`] and [`ConeBandAttempt`], and
/// deliberately a separate type: the unrecovered arm carries
/// [`formal::TorusAnnulusExit`], which names this route's own typed
/// distinctions (on-torus, winding, homology, material authority, realization),
/// and the recovered arm carries the [`formal::ConformanceTag`] that records
/// whether the certified cell relied on a malformed-source normalization.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TorusAnnulusAttempt {
    /// The torus annulus was certified and realized, and the validated mesh
    /// replaced the preserved legacy failure.
    Recovered {
        /// Triangles in the validated annulus.
        triangles: usize,
        /// Whether the source that produced it was well-formed, or carried
        /// the double-outer-bound malformation. A recovery from a malformed
        /// source is still a recovery, and is still not a clean read.
        conformance: formal::torus_cell::ConformanceTag,
    },
    /// The torus annulus route returned a typed exit, and the original legacy
    /// failure was preserved unchanged.
    Refused(formal::TorusAnnulusExit),
}

/// Tessellates faces, discarding why any of them failed.
///
/// Legacy shape. Prefer [`cshell_tessellation_with_outcomes`].
pub(super) fn cshell_tessellation<'a, C, S>(
    shell: &CompressedShell<Point3, C, S>,
    tol: f64,
    sp: impl SP<S>,
    lattice_of: impl Fn(&S) -> CertifiedLattice + Parallelizable,
) -> MeshedCShell
where
    C: PolylineableCurve + 'a,
    S: PreMeshableSurface + 'a,
{
    cshell_tessellation_with_outcomes(
        shell,
        tol,
        sp,
        lattice_of,
        |_| {
            formal::SupportSurfaceSchema::not_structurally_identified(
                formal::SchemaIdentificationFailure::NoStructuralReader {
                    representation: "legacy_entry_point_reads_no_schema",
                },
            )
        },
        |_| {
            formal::CurveSchema::not_structurally_identified(
                formal::CurveSchemaFailure::NoStructuralReader {
                    representation: "legacy_entry_point_reads_no_schema",
                },
            )
        },
    )
    .shell
}

/// Tessellates faces, preserving why each face that failed did so.
pub(super) fn cshell_tessellation_with_outcomes<'a, C, S>(
    shell: &CompressedShell<Point3, C, S>,
    tol: f64,
    sp: impl SP<S>,
    lattice_of: impl Fn(&S) -> CertifiedLattice + Parallelizable,
    schema_of: impl Fn(&S) -> formal::SupportSurfaceSchema + Parallelizable,
    curve_schema_of: impl Fn(&C) -> formal::CurveSchema + Parallelizable,
) -> MeshedShellOutcome
where
    C: PolylineableCurve + 'a,
    S: PreMeshableSurface + 'a,
{
    cshell_tessellation_with_outcomes_and_cylinder(
        shell,
        tol,
        sp,
        lattice_of,
        schema_of,
        curve_schema_of,
        |_: &S| -> std::result::Result<formal::CertifiedEmbeddedCylinder, &'static str> {
            Err("cylinder_evidence_not_provided")
        },
        |_: &C| {
            formal::CurveSchema::not_structurally_identified(
                formal::CurveSchemaFailure::NoStructuralReader {
                    representation: "cylinder_evidence_not_provided",
                },
            )
        },
        |_: &C| None,
    )
}

/// [`cshell_tessellation_with_outcomes_and_cylinder`], additionally threading
/// the conical-surface adapter the conical essential-band route needs.
///
/// A fourth entry point rather than a widened third one, so every caller with
/// no conical evidence to offer keeps compiling and keeps behaving identically:
/// the delegation below supplies a `cone_of` that refuses every surface, which
/// makes the cone route unreachable and the cylinder entry point's output what
/// it was.
///
/// Only one new closure is needed. The two curve readers are surface-agnostic â€”
/// they classify a `Curve3D` into a [`formal::SourceCurveFamily`] and know
/// nothing about what the face is trimmed from â€” so the cone route reads its
/// complete source circles through the same two the cylinder route does.
#[allow(clippy::too_many_arguments)]
pub(super) fn cshell_tessellation_with_outcomes_and_cone<'a, C, S>(
    shell: &CompressedShell<Point3, C, S>,
    tol: f64,
    sp: impl SP<S>,
    lattice_of: impl Fn(&S) -> CertifiedLattice + Parallelizable,
    schema_of: impl Fn(&S) -> formal::SupportSurfaceSchema + Parallelizable,
    curve_schema_of: impl Fn(&C) -> formal::CurveSchema + Parallelizable,
    cylinder_of: impl Fn(&S) -> std::result::Result<formal::CertifiedEmbeddedCylinder, &'static str>
        + Parallelizable,
    cylinder_curve_schema_of: impl Fn(&C) -> formal::CurveSchema + Parallelizable,
    cylinder_curve_family_of: impl Fn(&C) -> Option<formal::SourceCurveFamily> + Parallelizable,
    cone_of: impl Fn(&S) -> std::result::Result<formal::CertifiedEmbeddedCone, &'static str>
        + Parallelizable,
) -> MeshedShellOutcome
where
    C: PolylineableCurve + 'a,
    S: PreMeshableSurface + 'a,
{
    cshell_tessellation_inner(
        shell,
        tol,
        sp,
        lattice_of,
        schema_of,
        curve_schema_of,
        cylinder_of,
        cylinder_curve_schema_of,
        cylinder_curve_family_of,
        cone_of,
        |_: &S| -> std::result::Result<formal::CertifiedEmbeddedTorus, &'static str> {
            Err("torus_evidence_not_provided")
        },
    )
}

/// [`cshell_tessellation_with_outcomes_and_cone`], additionally threading
/// the torus-surface adapter the torus annulus route needs.
///
/// A sixth entry point rather than a widened fifth one, for the same reason
/// every previous one was added: a caller with no torus evidence to offer
/// keeps compiling against the cone form, and that form's output is unchanged
/// by this route's existence because it supplies a `torus_of` that refuses
/// every surface.
///
/// Only one new closure. The torus route reads its complete source circles
/// through the same two curve readers the cylinder and cone routes do; what
/// differs is what the cell then requires of the circle it was handed â€”
/// on-torus membership and `ZÂ²` winding, not constant-coordinate or nappe
/// obligations.
#[allow(clippy::too_many_arguments)]
pub(super) fn cshell_tessellation_with_outcomes_and_torus<'a, C, S>(
    shell: &CompressedShell<Point3, C, S>,
    tol: f64,
    sp: impl SP<S>,
    lattice_of: impl Fn(&S) -> CertifiedLattice + Parallelizable,
    schema_of: impl Fn(&S) -> formal::SupportSurfaceSchema + Parallelizable,
    curve_schema_of: impl Fn(&C) -> formal::CurveSchema + Parallelizable,
    cylinder_of: impl Fn(&S) -> std::result::Result<formal::CertifiedEmbeddedCylinder, &'static str>
        + Parallelizable,
    cylinder_curve_schema_of: impl Fn(&C) -> formal::CurveSchema + Parallelizable,
    cylinder_curve_family_of: impl Fn(&C) -> Option<formal::SourceCurveFamily> + Parallelizable,
    cone_of: impl Fn(&S) -> std::result::Result<formal::CertifiedEmbeddedCone, &'static str>
        + Parallelizable,
    torus_of: impl Fn(&S) -> std::result::Result<formal::CertifiedEmbeddedTorus, &'static str>
        + Parallelizable,
) -> MeshedShellOutcome
where
    C: PolylineableCurve + 'a,
    S: PreMeshableSurface + 'a,
{
    cshell_tessellation_inner(
        shell,
        tol,
        sp,
        lattice_of,
        schema_of,
        curve_schema_of,
        cylinder_of,
        cylinder_curve_schema_of,
        cylinder_curve_family_of,
        cone_of,
        torus_of,
    )
}

/// [`cshell_tessellation_with_outcomes`], additionally threading the rank-1
/// cylinder evidence readers (Milestone A / FORMAL-013-015).
///
/// The three cylinder closures are `look`'s composition-layer readers â€”
/// `step::cylinder::identify_source_cylinder`,
/// `step::lattice::cylinder_curve_schema_of` and
/// `step::lattice::cylinder_curve_family_of` â€” reduced to `Option`/tag-only
/// return types here so this crate does not depend on `look`'s error types.
/// Kept as a second entry point, rather than changing
/// [`cshell_tessellation_with_outcomes`]'s signature, so every existing
/// caller (and the STL/non-STEP paths, which have no cylinder evidence to
/// offer) compiles unchanged.
#[allow(clippy::too_many_arguments)]
pub(super) fn cshell_tessellation_with_outcomes_and_cylinder<'a, C, S>(
    shell: &CompressedShell<Point3, C, S>,
    tol: f64,
    sp: impl SP<S>,
    lattice_of: impl Fn(&S) -> CertifiedLattice + Parallelizable,
    schema_of: impl Fn(&S) -> formal::SupportSurfaceSchema + Parallelizable,
    curve_schema_of: impl Fn(&C) -> formal::CurveSchema + Parallelizable,
    cylinder_of: impl Fn(&S) -> std::result::Result<formal::CertifiedEmbeddedCylinder, &'static str>
        + Parallelizable,
    cylinder_curve_schema_of: impl Fn(&C) -> formal::CurveSchema + Parallelizable,
    cylinder_curve_family_of: impl Fn(&C) -> Option<formal::SourceCurveFamily> + Parallelizable,
) -> MeshedShellOutcome
where
    C: PolylineableCurve + 'a,
    S: PreMeshableSurface + 'a,
{
    cshell_tessellation_inner(
        shell,
        tol,
        sp,
        lattice_of,
        schema_of,
        curve_schema_of,
        cylinder_of,
        cylinder_curve_schema_of,
        cylinder_curve_family_of,
        // No conical evidence was offered, so none is claimed and the conical
        // route is unreachable. This entry point's output is unchanged by the
        // route's existence, by construction.
        |_: &S| -> std::result::Result<formal::CertifiedEmbeddedCone, &'static str> {
            Err("cone_evidence_not_provided")
        },
        |_: &S| -> std::result::Result<formal::CertifiedEmbeddedTorus, &'static str> {
            Err("torus_evidence_not_provided")
        },
    )
}

/// The outcome of tessellating one source edge.
///
/// Every edge is either sampled into a polyline, or had no certified source
/// traversal. The latter is *not* an invalid edge and is *not* sampled over
/// its evaluator domain: a closed source crescent sampled as a full loop
/// re-emits the malformed boundary this abstraction exists to remove. A face
/// whose boundary references an unresolved edge fails its boundary
/// construction, which reaches the caller through the tessellation outcome
/// mechanism as a named reason.
enum EstablishedEdge {
    /// The edge was sampled into a polyline.
    Mesh(CompressedEdge<PolylineCurve>),
    /// The edge's source traversal could not be established.
    Unresolved {
        /// The edge's source vertices, preserved for the meshed shell record.
        vertices: (usize, usize),
        /// A short stable reason tag.
        reason: &'static str,
    },
}

/// The one tessellation body every entry point above funnels into.
#[allow(clippy::too_many_arguments)]
fn cshell_tessellation_inner<'a, C, S>(
    shell: &CompressedShell<Point3, C, S>,
    tol: f64,
    sp: impl SP<S>,
    lattice_of: impl Fn(&S) -> CertifiedLattice + Parallelizable,
    schema_of: impl Fn(&S) -> formal::SupportSurfaceSchema + Parallelizable,
    curve_schema_of: impl Fn(&C) -> formal::CurveSchema + Parallelizable,
    cylinder_of: impl Fn(&S) -> std::result::Result<formal::CertifiedEmbeddedCylinder, &'static str>
        + Parallelizable,
    cylinder_curve_schema_of: impl Fn(&C) -> formal::CurveSchema + Parallelizable,
    cylinder_curve_family_of: impl Fn(&C) -> Option<formal::SourceCurveFamily> + Parallelizable,
    cone_of: impl Fn(&S) -> std::result::Result<formal::CertifiedEmbeddedCone, &'static str>
        + Parallelizable,
    torus_of: impl Fn(&S) -> std::result::Result<formal::CertifiedEmbeddedTorus, &'static str>
        + Parallelizable,
) -> MeshedShellOutcome
where
    C: PolylineableCurve + 'a,
    S: PreMeshableSurface + 'a,
{
    let vertices = shell.vertices.clone();
    let edge_probe = std::env::var_os("TRUCK_PROBE_EDGE").is_some();
    let evidence_probe = std::env::var_os("TRUCK_PROBE_EVIDENCE").is_some();
    let ambient_probe = std::env::var_os("TRUCK_PROBE_AMBIENT").is_some();
    // The planar vertical slice. `TRUCK_PROBE_SLICE` runs it in shadow and
    // reports; the recovery gate additionally lets a validated formal mesh
    // replace a face the *legacy path lost*, and nothing else.
    //
    // Every recovery gate below is **default-on with explicit opt-out** since
    // `WAVE-2C` â€” see `diagnosis::recovery_route_enabled`. Each route is
    // refinement-only (it is entered only where `failure.is_some()`), so
    // enabling one cannot change a face that already meshed; the worst it can
    // do is decline to recover. `TRUCK_FORMAL_RECOVERY=0` restores the pure
    // legacy tessellation, and any single `_ROUTE=0` restores that route's.
    let slice_probe = std::env::var_os("TRUCK_PROBE_SLICE").is_some();
    let recovery_gate = diagnosis::formal_recovery_enabled();
    // The planar-holes expansion carries its own gate under the master, so
    // "rank-0 one-bound recovery" and "rank-0 one-bound + holes recovery" are
    // separately measurable runs rather than one conflated population. Closing
    // the master gate closes this one with it.
    let holes_recovery_gate =
        recovery_gate && diagnosis::recovery_route_enabled("TRUCK_FORMAL_RECOVERY_HOLES");
    // The rank-1 cylinder route, on the identical two-tier pattern as the
    // holes route above: a stable route tag (`_CYLINDER`) under the same
    // master gate, plus its own shadow probe.
    let cylinder_probe = std::env::var_os("TRUCK_PROBE_CYLINDER").is_some();
    let cylinder_recovery_gate =
        recovery_gate && diagnosis::recovery_route_enabled("TRUCK_FORMAL_RECOVERY_CYLINDER");
    let run_cylinder_slice = cylinder_probe || cylinder_recovery_gate;
    let run_slice = slice_probe || recovery_gate;
    // The rank-1 cylinder *band* route: the two-bound annulus. Same two-tier
    // pattern again, and it carries no shadow probe of its own â€” the attempt
    // is reported through `MeshedShellOutcome::band_attempts`, which is typed
    // and needs no parsing, rather than through another stderr channel.
    let band_recovery_gate = diagnosis::cylinder_band_recovery_enabled();
    // The rank-2 torus annulus route. Two modes under one gate:
    // `TRUCK_PROBE_TORUS` runs the certification in shadow and records the
    // typed outcome without replacing the legacy mesh â€” the observer that
    // reproduces the census. `TRUCK_FORMAL_RECOVERY_TORUS` additionally lets
    // a validated torus annulus mesh replace a face the legacy path lost.
    // Nested under the master gate since `WAVE-2C`: it used to stand outside
    // it so a torus run could be measured without the planar route's
    // recoveries mixed in, and under default-on that is instead done by
    // setting `TRUCK_FORMAL_RECOVERY_TORUS=0` and diffing.
    let torus_probe = std::env::var_os("TRUCK_PROBE_TORUS").is_some();
    let torus_recovery_gate =
        recovery_gate && diagnosis::recovery_route_enabled("TRUCK_FORMAL_RECOVERY_TORUS");
    // Whether a recovery announces itself on stderr.
    //
    // While the routes were opt-in, every recovery printed a `RECOVERED` line
    // (and the planar route a `RECOVERED_VERTEX` line per vertex) because the
    // only reason to have opened the gate was to read them. Default-on makes
    // that an unconditional side effect of rendering: 525 lines on
    // `00009190`, on a tool whose stderr an agent is expected to parse. The
    // recovery is still fully reported â€” `MeshedShellOutcome`'s typed
    // `band_attempts`, `cone_band_attempts` and `torus_band_attempts` carry it
    // structurally, which is what the census reads â€” so the log is now opt-in
    // behind its own probe rather than being the only channel.
    let recovery_log = std::env::var_os("TRUCK_PROBE_RECOVERY").is_some();
    // The deck-consistent two-loop join. Unlike the routes above it does not
    // build a mesh of its own: it rebuilds the *same* boundary with the second
    // loop traversed in the direction that satisfies `Î£Î´ = 0`, and re-runs the
    // ordinary tessellator on it. So it inherits every check the legacy path
    // makes, and adds no geometry the legacy path would not have accepted.
    let deck_join_gate = diagnosis::deck_join_recovery_enabled();
    let run_torus = torus_probe || torus_recovery_gate;
    // A per-run shell ordinal, so a `FaceKey` is unique across shells:
    // `declared_face_index` is an index *within* a shell and collides between
    // them. Assigned once per shell here, before the parallel face loop, so
    // every face of one shell shares it.
    let shell_ordinal = SHELL_ORDINAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    // The source-incidence tolerance for this shell's edges. The STEP source
    // uncertainty declared by the shape's geometric representation context is
    // the authority for whether a source vertex realizes a point on its
    // edge-curve carrier; Truck's fixed numerical tolerance is only the
    // fallback for a source that declares none. The two remain distinct
    // concepts: this value admits incidence residuals, it never merges root
    // candidates or licenses a seam wrap (see `source_edge`).
    let source_tolerance = shell
        .source_geometric_uncertainty
        .filter(|uncertainty| uncertainty.is_finite() && *uncertainty > 0.0)
        .unwrap_or(source_edge::SOURCE_INCIDENCE_TOLERANCE);
    let tessellate_edge = |edge: &CompressedEdge<C>| {
        let curve = &edge.curve;
        let range = curve.range_tuple();
        if edge_probe {
            // How much of its own period a curve is asked to cover. An edge
            // whose start and end vertices coincide gives the importer no
            // independent parameter for each end -- they are the same point
            // modulo the period -- so a generic endpoint solver can resolve
            // them into copies two periods apart. The ratio, not the absolute
            // range, is what says so: a shifted circle may legitimately run
            // over [-pi, 3pi], which is still two periods.
            let span = range.1 - range.0;
            let ratio = curve.period().map(|period| span / period);
            // The radius the converted curve actually has. The source file's
            // circles are a short, known inventory, so a converted radius that
            // is not in it means the conversion built the wrong geometry for
            // the right entity; one that is in it exonerates the curve and
            // points at the face-to-surface pairing instead.
            let fitted = {
                let (t0, t1) = range;
                let (a, b, c) = (
                    curve.subs(t0),
                    curve.subs(t0 + (t1 - t0) / 3.0),
                    curve.subs(t0 + 2.0 * (t1 - t0) / 3.0),
                );
                let (ab, ac) = (b - a, c - a);
                let normal = ab.cross(ac);
                let n2 = normal.magnitude2();
                match n2 > f64::EPSILON {
                    true => {
                        let centre = a
                            + (ac.magnitude2() * ab.cross(normal)
                                - ab.magnitude2() * ac.cross(normal))
                                / (2.0 * n2);
                        Some(centre.distance(a))
                    }
                    false => None,
                }
            };
            eprintln!(
                "EDGE range=({:.6},{:.6}) span={span:.6} period={:?} span/period={:?} \
                 same_vertex={} fitted_radius={:?}",
                range.0,
                range.1,
                curve.period(),
                ratio,
                edge.vertices.0 == edge.vertices.1,
                fitted.map(|r| (r * 1.0e5).round() / 1.0e5),
            );
        }
        let mut range = curve.evaluation_range();
        // Establish which portion of the curve this topological edge actually
        // denotes. The evaluator domain is the traversal only when the evidence
        // says so; a closed source crescent with interior source vertices is
        // sampled over its source interval instead, and a traversal that cannot
        // be established is propagated as `Unresolved` rather than sampled as a
        // whole loop.
        let (start_pos, end_pos) = match (
            vertices.as_slice().get(edge.vertices.0),
            vertices.as_slice().get(edge.vertices.1),
        ) {
            (Some(start), Some(end)) => (*start, *end),
            _ => {
                return EstablishedEdge::Unresolved {
                    vertices: edge.vertices,
                    reason: "source_vertex_position_missing",
                }
            }
        };
        let traversal_verdict = source_edge::establish_source_edge_traversal(
            curve,
            start_pos,
            end_pos,
            edge.vertices.0 == edge.vertices.1,
            source_tolerance,
            tol,
        );
        range = match traversal_verdict {
            source_edge::SourceEdgeTraversal::CanonicalByEvalRange { range } => range,
            source_edge::SourceEdgeTraversal::CanonicalBySourceInterval { traversal, .. } => {
                let poly = source_edge::sample_traversal(curve, &traversal, tol);
                return EstablishedEdge::Mesh(CompressedEdge {
                    vertices: edge.vertices,
                    curve: poly,
                });
            }
            source_edge::SourceEdgeTraversal::Unresolved { reason } => {
                return EstablishedEdge::Unresolved {
                    vertices: edge.vertices,
                    reason,
                };
            }
        };
        if edge.vertices.0 == edge.vertices.1 && (range.1 - range.0).abs() < 1e-4 {
            if let Some(period) = curve.period() {
                if period > 1e-4 {
                    range = (range.0, range.0 + period);
                }
            }
        }
        // A closed edge's source extent may carry the closure arc beyond the
        // interior knot span. Those samples are genuine boundary points only
        // where the curve's basis is still a partition of unity; in the
        // exporter's unclamped closure sliver it is not, and evaluating there
        // invents the off-curve/origin endpoints P1 exists to remove. Sample
        // the declared extent exactly when the basis certificate holds at its
        // ends, and not otherwise -- the domain is `D_source_edge_use ∩
        // D_basis_partition_of_unity`, never the bare declared range.
        if edge.vertices.0 == edge.vertices.1 {
            if let Some((rt0, rt1)) = curve.try_range_tuple() {
                if rt0 < range.0 - 1.0e-12 && curve.basis_is_partition_of_unity(rt0) {
                    range.0 = rt0;
                }
                if rt1 > range.1 + 1.0e-12 && curve.basis_is_partition_of_unity(rt1) {
                    range.1 = rt1;
                }
            }
        }
        let mut poly = PolylineCurve::from_curve(curve, range, tol);
        if poly.len() <= 2 && range.1 - range.0 > 1e-4 {
            let mut pts = Vec::new();
            const STEPS: usize = 16;
            for i in 0..=STEPS {
                let t = range.0 + (i as f64 / STEPS as f64) * (range.1 - range.0);
                pts.push(curve.subs(t));
            }
            poly = PolylineCurve::from(pts);
        }
        EstablishedEdge::Mesh(CompressedEdge {
            vertices: edge.vertices,
            curve: poly,
        })
    };
    #[cfg(not(target_arch = "wasm32"))]
    let edges: Vec<EstablishedEdge> = shell.edges.par_iter().map(tessellate_edge).collect();
    #[cfg(target_arch = "wasm32")]
    let edges: Vec<EstablishedEdge> = shell.edges.iter().map(tessellate_edge).collect();
    // Which surface in this shell does a face's own boundary actually lie on?
    //
    // A residual says the boundary and the surface it was handed are
    // incompatible; it does not say whether the pairing is wrong or one of the
    // entities is built wrong. Testing the boundary against *every* surface in
    // the shell separates them. If another surface fits to near zero, this is
    // an association defect and the correct partner is named. If none fits, one
    // of the two entities is constructed incorrectly.
    if std::env::var_os("TRUCK_PROBE_ASSOC").is_some() {
        const GRID: usize = 60;
        let nearest = |surface: &S, point: Point3| {
            let (urange, vrange) = surface.try_range_tuple();
            let axis = |range: Option<(f64, f64)>, period: Option<f64>| match (range, period) {
                (Some(r), _) => r,
                (None, Some(p)) => (-p, p),
                (None, None) => (-1.0, 1.0),
            };
            let (ulo, uhi) = axis(urange, surface.u_period());
            let (vlo, vhi) = axis(vrange, surface.v_period());
            let mut best = f64::INFINITY;
            for i in 0..=GRID {
                let u = ulo + (uhi - ulo) * i as f64 / GRID as f64;
                for j in 0..=GRID {
                    let v = vlo + (vhi - vlo) * j as f64 / GRID as f64;
                    best = best.min(surface.subs(u, v).distance(point));
                }
            }
            best
        };
        for (index, face) in shell.faces.iter().enumerate() {
            // A handful of boundary samples is enough to decide a pairing.
            let samples: Vec<Point3> = face
                .boundaries
                .iter()
                .flatten()
                .filter_map(|e| match edges.get(e.index) {
                    Some(EstablishedEdge::Mesh(edge)) => Some(edge),
                    _ => None,
                })
                .flat_map(|e| e.curve.iter().copied())
                .step_by(3)
                .take(8)
                .collect();
            if samples.is_empty() {
                continue;
            }
            let worst = |surface: &S| {
                samples
                    .iter()
                    .fold(0.0_f64, |acc, p| acc.max(nearest(surface, *p)))
            };
            let own = worst(&face.surface);
            if own <= tol * 3.0 {
                continue;
            }
            let mut best = (usize::MAX, f64::INFINITY);
            for (other, candidate) in shell.faces.iter().enumerate() {
                if other == index {
                    continue;
                }
                let d = worst(&candidate.surface);
                if d < best.1 {
                    best = (other, d);
                }
            }
            eprintln!(
                "ASSOC face={index} own={own:.4e} best_other=face{} at {:.4e} tol={tol:.4e}",
                best.0, best.1
            );
        }
    }
    let tessellate_face = |(declared_face_index, face): (usize, &CompressedFace<S>)| {
        let source_face_id = face.provenance.best_id().map(SourceEntityId::get);
        let source_use_id = face.provenance.use_id.map(SourceEntityId::get);
        let periodic_rank = u8::from(face.surface.u_period().is_some())
            + u8::from(face.surface.v_period().is_some());
        PROBE_FACE_CONTEXT.with(|context| {
            context.set((source_face_id, declared_face_index, periodic_rank));
        });
        let periodic_axes = diagnosis::PeriodicAxes {
            u: face.surface.u_period().is_some(),
            v: face.surface.v_period().is_some(),
        };
        let bound_count = face.boundaries.len();
        let diag = diagnosis::diag_enabled();
        // DIAG-002 structural facts, counted from the face's own wires: cheap,
        // and exactly the source/topology structure the record's contract
        // demands. `edge_use_count` is the number of source edge uses the
        // face references; `distinct_vertex_count` the distinct source
        // vertices behind them.
        let edge_use_count: usize = face.boundaries.iter().map(|wire| wire.len()).sum();
        let mut distinct_vertices: Vec<usize> = Vec::new();
        for wire in &face.boundaries {
            for edge_idx in wire {
                let vertices = match edges.get(edge_idx.index) {
                    Some(EstablishedEdge::Mesh(edge)) => edge.vertices,
                    Some(EstablishedEdge::Unresolved { vertices, .. }) => *vertices,
                    None => continue,
                };
                if !distinct_vertices.contains(&vertices.0) {
                    distinct_vertices.push(vertices.0);
                }
                if !distinct_vertices.contains(&vertices.1) {
                    distinct_vertices.push(vertices.1);
                }
            }
        }
        let distinct_vertex_count = distinct_vertices.len();

        let boundaries = face.boundaries.clone();
        let surface = &face.surface;
        let lattice = lattice_of(surface);
        // Whether the lattice certified every periodic axis from
        // representation-derived evidence, so the record can claim the
        // source-declared closure honestly.
        let all_periods_certified = (!periodic_axes.u
            || matches!(lattice.u, AxisPeriodStatus::Exact { .. }))
            && (!periodic_axes.v || matches!(lattice.v, AxisPeriodStatus::Exact { .. }));
        diagnosis::begin_face(
            diagnosis::document_context(),
            source_face_id,
            source_use_id,
            periodic_axes,
            bound_count,
            edge_use_count,
            distinct_vertex_count,
            periodic_rank,
            tol,
            all_periods_certified,
            shell.source_geometric_uncertainty,
        );
        // The structural schema, read before the lattice erases which producer
        // said `NonPeriodic`. Nothing in the legacy chain below reads it.
        let schema = schema_of(surface);
        // Step 0: build the rewrite's input seam beside the legacy path and
        // report it. Nothing below reads it, so geometry is unchanged by
        // construction â€” the point is to count what the seam carries before
        // the pipeline that depends on it exists.
        if evidence_probe {
            let input = source_face_input_from_compressed(
                declared_face_index,
                source_face_id,
                face,
                // The source edge identities are preserved verbatim by
                // `tessellate_edge`; the tessellated polyline carrier is not
                // what this seam reads.
                &shell.edges,
            );
            emit_evidence_probe(&input, source_face_id, declared_face_index, &lattice);
        }
        // Step 1: resolve the same lattice through the formal ambient-period
        // model and report what it concludes. Nothing below reads the result --
        // it is dropped at the end of this block -- so geometry is unchanged by
        // construction. The point is to measure where the legacy
        // representation collapses uncertainty, before anything depends on it.
        if ambient_probe {
            emit_ambient_probe(
                source_face_id,
                declared_face_index,
                shell_ordinal,
                &lattice,
                &schema,
            );
        }
        let create_boundary = |(bound_index, wire): (usize, &Vec<CompressedEdgeIndex>)| {
            let bound = BoundId(bound_index);
            // Each wire item becomes a tagged polyline: the curve exactly as
            // `create_edge` produced it before, plus the synthetic source
            // identity `(BoundId(bound_index), use_index, orientation)` that is
            // the last cheap provenance this seam still has.
            //
            // A wire that references an edge with no established source
            // traversal fails the boundary outright: dropping that edge would
            // let the remaining samples close over the missing arc, which is
            // exactly the invented geometry `Unresolved` exists to refuse.
            let mut wire_iter = Vec::with_capacity(wire.len());
            for (use_index, edge_idx) in wire.iter().enumerate() {
                let edge = match edges.get(edge_idx.index) {
                    Some(EstablishedEdge::Mesh(edge)) => edge,
                    Some(EstablishedEdge::Unresolved { .. }) => {
                        // DIAG-002: the source-edge traversal refusal witness.
                        // The failing bound and edge use are in hand here; the
                        // caller tolerance and the source's declared geometric
                        // uncertainty are the operative numbers.
                        diagnosis::record_source_edge_refusal(diagnosis::SourceEdgeWitness {
                            source_bound: Some(bound_index),
                            source_edge_use: Some(use_index),
                            endpoint_residuals: None,
                            declared_source_uncertainty: shell.source_geometric_uncertainty,
                            effective_incidence_tolerance: None,
                            caller_tolerance: Some(tol),
                            carrier_closure: None,
                        });
                        return Err(TessellationFailureReason::EdgeTraversalUnresolved);
                    }
                    None => continue,
                };
                let curve = match edge_idx.orientation {
                    true => edge.curve.clone(),
                    false => edge.curve.inverse(),
                };
                wire_iter.push(SourcePolyline {
                    curve,
                    source: SourceEdgeUse {
                        bound,
                        index: use_index,
                        orientation: edge_idx.orientation,
                    },
                });
            }
            PolyBoundaryPiece::try_new(surface, wire_iter.into_iter(), &sp, tol, &lattice)
        };
        let preboundary: std::result::Result<Vec<_>, _> =
            boundaries.iter().enumerate().map(create_boundary).collect();
        // G8: the same computation as before, with the failure kept rather than
        // flattened into an empty mesh.
        //
        // `surface` is left exactly as the legacy path produced it â€” `None`
        // when no boundary could be built, `Some(empty)` when tessellation
        // itself failed â€” so the meshed shell is unchanged and this commit adds
        // information without moving any face between populations. The reason
        // travels beside it instead of being destroyed.
        // The pieces are retained only for a face that can reach the two-loop
        // join at all â€” a periodic chart presenting exactly two bounds â€” so the
        // clone is paid on the band population rather than on every face.
        let deck_join_candidate = deck_join_gate
            && (lattice.declared_u_period().is_some() || lattice.declared_v_period().is_some())
            && preboundary.as_ref().is_ok_and(|pieces| pieces.len() == 2);
        // FACE-VALIDITY Detector B: measure the constructed boundary pieces and
        // reject a certified degenerate face before it enters the CDT or any
        // formal recovery route. Production-active for the certified world-rank
        // < 2 class: the measurement is the world-space numerical rank of the
        // lifted boundary, taken against a floating-point conditioning bound
        // (never a meshing tolerance), so every boundary with two real world
        // directions — however small or thin — survives and no currently
        // rendering face is touched. A rejection is the one terminal state no
        // recovery route may touch: a zero-area trim must not be "recovered"
        // into triangles. The exception is the certified closed-loop re-lift
        // below, which runs only on a degenerate initial lift and only when a
        // rank-2 on-surface source interval can be independently certified.
        let mut preboundary = preboundary;
        let mut validity_certificate: Option<FaceValidityCertificate> = match preboundary.as_ref() {
            Ok(pieces) => detect_degenerate_trim(pieces, surface),
            Err(_) => None,
        };
        // Certified closed-loop re-lift (recovery workstream). A face whose
        // initial boundary lift degenerated — Detector B above fired — is not
        // automatically rejected: when the source is a topologically closed
        // edge whose evaluator overshoot leaves the owning surface, and a
        // closed on-surface sub-domain of the source traversal can be
        // independently certified, the boundary is re-lifted over that interval
        // and the ordinary pipeline re-measures it. The re-lift is the only
        // path that may turn a Detector-B firing face into geometry, and it
        // does so only under the activation theorem (see `closed_loop_relift`).
        if validity_certificate.is_some() && closed_loop_relift::recovery_gate() {
            if let Ok(pieces) = preboundary.as_ref() {
                let relifted = closed_loop_relift::try_relift_face(
                    shell,
                    surface,
                    &face.boundaries,
                    &edges,
                    &sp,
                    tol,
                    &lattice,
                    pieces,
                );
                if let Some(relifted) = relifted {
                    // Re-measure the re-lifted boundary under the same
                    // detector: the recovery stands only if the re-lift is
                    // itself non-degenerate.
                    let recertificate = detect_degenerate_trim(&relifted, surface);
                    if recertificate.is_none() {
                        preboundary = Ok(relifted);
                        validity_certificate = None;
                    }
                }
            }
        }
        let rejected = validity_certificate.is_some();
        // The legacy boundary, kept only for a face whose parity flood
        // contradicted itself, so the winding retry at the end of this chain
        // has something to re-tessellate. Nothing else reads it, and on every
        // other face it stays `None`.
        let mut parity_retry_boundary: Option<PolyBoundary> = None;
        let (polygon, failure) = match preboundary {
            Err(reason) => (
                None,
                Some(diagnosis::fail(
                    reason,
                    diagnosis::failure_stage_for_reason(reason),
                )),
            ),
            Ok(_preboundary) if rejected => {
                // A certified rejection is terminal: no recovery route may touch
                // it. The certificate travels inside the record; the census reads
                // it to classify the face as `rejected_intrinsic`.
                let failure = diagnosis::reject(
                    TessellationFailureReason::RejectedDegenerate,
                    diagnosis::FailureStage::ValidityClassification,
                    validity_certificate.unwrap_or_else(|| {
                        FaceValidityCertificate::all_bounds_collapsed(bound_count)
                    }),
                );
                (None, Some(failure))
            }
            Ok(preboundary) => {
                let retained = deck_join_candidate.then(|| preboundary.clone());
                let boundary = PolyBoundary::new(preboundary, &surface, tol, &lattice);
                match trimming_tessellation_result(&surface, &boundary, tol, &lattice) {
                    Ok(mesh) => (Some(mesh), None),
                    // Refinement-only, structurally: the corrected join is
                    // reached only from the arm where the legacy path produced
                    // no mesh, so it can replace a failure and nothing else.
                    Err(failure) => {
                        if failure.reason == TessellationFailureReason::ContradictoryDualParity
                            && diagnosis::winding_parity_enabled()
                        {
                            parity_retry_boundary = Some(boundary.clone());
                        }
                        // The DIAG-001 record, and with it the loss bucket the
                        // band routes admit on, must keep describing the legacy
                        // boundary â€” not a mixture of it and this second
                        // attempt.
                        let _suspension = diagnosis::SinkSuspension::new();
                        let recovered = retained.and_then(|pieces| {
                            let (boundary, outcome) = PolyBoundary::new_with_join(
                                pieces,
                                &surface,
                                tol,
                                &lattice,
                                TwoLoopJoinPolicy::DeckConsistent,
                            );
                            // Rebuilding is only worth a tessellation pass when
                            // the equation actually selected the other
                            // traversal. Every other outcome reproduces the
                            // boundary that was just tried.
                            let applied = TwoLoopJoinOutcome::ForwardResolves { applied: true };
                            (outcome == applied)
                                .then(|| {
                                    trimming_tessellation_result(&surface, &boundary, tol, &lattice)
                                })
                                .and_then(std::result::Result::ok)
                        });
                        match recovered {
                            Some(mesh) => {
                                if recovery_log {
                                    eprintln!(
                                        "RECOVERED\tsource_face_id={}\t\
                                         declared_face_index={declared_face_index}\t\
                                         triangles={}\tpath=deck_join",
                                        source_face_id
                                            .map(|id| id.to_string())
                                            .unwrap_or_else(|| "none".into()),
                                        mesh.tri_faces().len(),
                                    );
                                }
                                (Some(mesh), None)
                            }
                            // The corrected join was not attempted, or was
                            // itself refused. The legacy failure is preserved
                            // exactly.
                            None => (Some(PolygonMesh::default()), Some(failure)),
                        }
                    }
                }
            }
        };
        // A certified rejection -- intrinsic degenerate (FACE-VALIDITY) or a
        // genuine source-level singular ambiguity -- is the one terminal state
        // no recovery route may touch. `rejected` covers the pre-tessellation
        // validity certificate. `RejectedAmbiguous` exists as a reason for a
        // *certificate-backed* source ambiguity (two distinct source-consistent
        // continuations); no production mechanism currently constructs one, so
        // the P2 singular-transition analysis leaves the lift unresolved
        // (`AmbiguousLift`) instead of emitting it. The union below is
        // deliberately kept so a future genuine certificate is still terminal.
        let rejected_terminal = rejected
            || matches!(
                failure,
                Some(TessellationFailure {
                    reason: TessellationFailureReason::RejectedAmbiguous,
                    ..
                })
            );
        // The legacy verdict, classified here and not later: the cylinder-band
        // fallback admits one loss bucket, the bucket is derived from conflict
        // witnesses the sink holds, and the terminal finalizer below consumes
        // them. Reading it at the seam where the legacy path finished also
        // makes it unambiguously a statement about *that* result, and not
        // about whatever a formal route replaced it with.
        let legacy_bucket = match (band_recovery_gate, &failure) {
            (true, Some(failure)) => Some(diagnosis::derived_bucket(failure.reason)),
            _ => None,
        };
        // The planar vertical slice, run beside the legacy result. Its input
        // is Step 0's source evidence, Step 1's certified lattice and the
        // structural schemas; it reads nothing the legacy path produced, so a
        // face's formal verdict is independent of whether the legacy path
        // succeeded.
        let (polygon, failure) = if !run_slice || rejected_terminal {
            (polygon, failure)
        } else {
            let outcome = run_slice_for_face(
                declared_face_index,
                source_face_id,
                shell_ordinal,
                face,
                &shell.edges,
                &shell.vertices,
                &lattice,
                &schema,
                &curve_schema_of,
                tol,
            );
            let slice_record = outcome.as_ref().map(|outcome| outcome.planar.clone());
            let holes_record = outcome.as_ref().map(|outcome| outcome.holes.clone());
            if slice_probe {
                emit_slice_probe(
                    source_face_id,
                    declared_face_index,
                    shell_ordinal,
                    &slice_record,
                );
                emit_holes_probe(
                    source_face_id,
                    declared_face_index,
                    shell_ordinal,
                    &holes_record,
                );
            }
            if let Some(developed) = outcome.as_ref().and_then(|o| o.developed.as_ref()) {
                emit_developed_probe(source_face_id, declared_face_index, developed);
            }
            // The recovery gate. Every conjunct is explicit: a validated formal
            // mesh replaces a face the legacy path *lost*, and never a face it
            // meshed.
            //
            // The hole-free slice is consulted first, so opening the holes gate
            // cannot move a face that the original rank-0 path already
            // recovered â€” those recoveries stay bit-identical. The two
            // populations are disjoint anyway (one slice delegates wherever the
            // other applies), and the ordering makes that independent of the
            // delegation logic rather than reliant on it.
            let legacy_failed = failure.is_some();
            let resolved_mesh = match legacy_failed {
                false => None,
                true => {
                    let planar =
                        slice_record
                            .as_ref()
                            .filter(|_| recovery_gate)
                            .and_then(|record| {
                                match record.stage == formal::SliceStage::FinalValidity
                                    && record.category == formal::SliceCategory::Resolved
                                {
                                    true => {
                                        record.mesh.as_ref().map(|mesh| ("rank0_one_bound", mesh))
                                    }
                                    false => None,
                                }
                            });
                    let holes = || {
                        holes_record
                            .as_ref()
                            .filter(|_| holes_recovery_gate)
                            .and_then(|record| {
                                match !record.delegated
                                    && record.stage == formal::SliceStage::FinalValidity
                                    && record.category == formal::SliceCategory::Resolved
                                {
                                    true => record.mesh.as_ref().map(|mesh| ("rank0_holes", mesh)),
                                    false => None,
                                }
                            })
                    };
                    planar.or_else(holes)
                }
            };
            match resolved_mesh {
                Some((path, formal_mesh)) => {
                    if recovery_log {
                        eprintln!(
                            "RECOVERED\tsource_face_id={}\t\
                             declared_face_index={declared_face_index}\ttriangles={}\tpath={path}",
                            source_face_id
                                .map(|id| id.to_string())
                                .unwrap_or_else(|| "none".into()),
                            formal_mesh.triangles.len(),
                        );
                        // The recovered geometry, so a corpus face can become a
                        // regression fixture without shipping the 400 MB model
                        // it came from.
                        for position in &formal_mesh.positions {
                            eprintln!(
                                "RECOVERED_VERTEX\tsource_face_id={}\tx={:?}\ty={:?}\tz={:?}",
                                source_face_id
                                    .map(|id| id.to_string())
                                    .unwrap_or_else(|| "none".into()),
                                position.x,
                                position.y,
                                position.z,
                            );
                        }
                    }
                    (Some(planar_mesh_to_polygon(formal_mesh)), None)
                }
                None => (polygon, failure),
            }
        };
        // The rank-1 cylinder slice, on the identical additive discipline as
        // the planar slice above: it only ever runs after the planar block
        // has already had its chance, and it only ever replaces a face the
        // legacy path *still* has no mesh for â€” never a face the planar
        // rank-0 path just recovered, and never a successful legacy mesh.
        let (polygon, failure) = if !run_cylinder_slice || rejected_terminal {
            (polygon, failure)
        } else {
            let record = run_cylinder_slice_for_face(
                declared_face_index,
                source_face_id,
                face,
                &shell.edges,
                &shell.vertices,
                &cylinder_of,
                &cylinder_curve_schema_of,
                &cylinder_curve_family_of,
                tol,
            );
            if cylinder_probe {
                emit_cylinder_probe(source_face_id, declared_face_index, shell_ordinal, &record);
            }
            let legacy_failed = failure.is_some();
            match (
                legacy_failed,
                cylinder_recovery_gate,
                record.category == formal::SliceCategory::Resolved,
                &record.mesh,
            ) {
                (true, true, true, Some(mesh)) => {
                    if recovery_log {
                        eprintln!(
                            "RECOVERED\tsource_face_id={}\t\
                             declared_face_index={declared_face_index}\ttriangles={}\tpath=cylinder",
                            source_face_id
                                .map(|id| id.to_string())
                                .unwrap_or_else(|| "none".into()),
                            record.triangles,
                        );
                    }
                    (Some(mesh.clone()), None)
                }
                _ => (polygon, failure),
            }
        };
        // The formal cylinder-band fallback. The exact production rule:
        //
        //   legacy success                     -> the legacy mesh, unchanged
        //   legacy SyntheticSyntheticCrossing
        //     + certified cylinder support
        //     + exactly two authoritative bounds
        //                                      -> attempt `run_cylinder_band`
        //       validated mesh                 -> the formal mesh
        //       typed exit                     -> the legacy failure, preserved
        //   any other legacy failure           -> the legacy failure, preserved
        //
        // `failure.is_some()` is the "legacy success" arm: a face that has a
        // mesh at this point â€” because the legacy path meshed it, or because a
        // formal route above already recovered it â€” is never attempted. The
        // bucket, the cylinder certificate and the bound count are the other
        // three conjuncts, each checked explicitly and none of them repaired.
        let (polygon, failure, band_attempt) = match (
            band_recovery_gate,
            failure.is_some(),
            legacy_bucket == Some(diagnosis::LossBucket::SyntheticSyntheticCrossing),
        ) {
            (true, true, true) if !rejected_terminal => {
                match run_cylinder_band_for_face(
                    declared_face_index,
                    source_face_id,
                    face,
                    &shell.edges,
                    &shell.vertices,
                    &cylinder_of,
                    &cylinder_curve_schema_of,
                    &cylinder_curve_family_of,
                    tol,
                ) {
                    None => (polygon, failure, None),
                    Some(Ok((mesh, conformance))) => {
                        let triangles = mesh.tri_faces().len();
                        diagnosis::record_route_decision(
                            diagnosis::RecoveryRoute::CylinderBand,
                            true,
                            diagnosis::RouteOutcome::Recovered,
                            None,
                        );
                        (
                            Some(mesh),
                            None,
                            Some(CylinderBandAttempt::Recovered {
                                triangles,
                                conformance,
                            }),
                        )
                    }
                    Some(Err(exit)) => {
                        diagnosis::record_route_decision(
                            diagnosis::RecoveryRoute::CylinderBand,
                            true,
                            diagnosis::RouteOutcome::Refused,
                            Some(exit.tag()),
                        );
                        (polygon, failure, Some(CylinderBandAttempt::Refused(exit)))
                    }
                }
            }
            _ => {
                diagnosis::record_route_decision(
                    diagnosis::RecoveryRoute::CylinderBand,
                    band_recovery_gate,
                    match band_recovery_gate {
                        true => diagnosis::RouteOutcome::PreconditionUnmet,
                        false => diagnosis::RouteOutcome::GateClosed,
                    },
                    None,
                );
                (polygon, failure, None)
            }
        };
        // The conical essential-band route, on the identical production rule
        // and under the identical gate. It runs only after the cylinder band
        // has had its chance and only on a face that *still* has no mesh, so
        // the two cells cannot both claim one face: `cylinder_of` and `cone_of`
        // are mutually exclusive on any one surface anyway â€” a revolved line is
        // either parallel to its axis or tilted from it, and each identifier
        // refuses the other's case by name â€” but the ordering makes that a
        // property of the pipeline rather than only of the adapters.
        let (polygon, failure, cone_band_attempt) = match (
            band_recovery_gate,
            failure.is_some(),
            legacy_bucket == Some(diagnosis::LossBucket::SyntheticSyntheticCrossing),
        ) {
            (true, true, true) if !rejected_terminal => {
                match run_conical_band_for_face(
                    declared_face_index,
                    source_face_id,
                    face,
                    &shell.edges,
                    &shell.vertices,
                    &cone_of,
                    &cylinder_curve_schema_of,
                    &cylinder_curve_family_of,
                    tol,
                ) {
                    None => (polygon, failure, None),
                    Some(Ok((mesh, nappe, standing))) => {
                        let triangles = mesh.tri_faces().len();
                        diagnosis::record_route_decision(
                            diagnosis::RecoveryRoute::ConeBand,
                            true,
                            diagnosis::RouteOutcome::Recovered,
                            None,
                        );
                        (
                            Some(mesh),
                            None,
                            Some(ConeBandAttempt::Recovered {
                                triangles,
                                nappe,
                                standing,
                            }),
                        )
                    }
                    Some(Err(exit)) => {
                        diagnosis::record_route_decision(
                            diagnosis::RecoveryRoute::ConeBand,
                            true,
                            diagnosis::RouteOutcome::Refused,
                            Some(exit.tag()),
                        );
                        (polygon, failure, Some(ConeBandAttempt::Refused(exit)))
                    }
                }
            }
            _ => {
                diagnosis::record_route_decision(
                    diagnosis::RecoveryRoute::ConeBand,
                    band_recovery_gate,
                    match band_recovery_gate {
                        true => diagnosis::RouteOutcome::PreconditionUnmet,
                        false => diagnosis::RouteOutcome::GateClosed,
                    },
                    None,
                );
                (polygon, failure, None)
            }
        };
        // The torus annulus route. Two modes:
        //
        //   shadow (TRUCK_PROBE_TORUS):  always runs on a torus face, records
        //       the typed outcome in `torus_band_attempts`, and does NOT
        //       replace the legacy mesh. This is the observer that reproduces
        //       the corrected census.
        //
        //   recovery (TRUCK_FORMAL_RECOVERY_TORUS): runs only on a face that
        //       still has no mesh (legacy failed, and no earlier formal route
        //       recovered it), and replaces the failure with the validated
        //       torus annulus mesh. Production recovery is gated separately
        //       from the observer so the census can be confirmed before any
        //       mesh is changed.
        //
        // The torus route runs after the cone route and only on a face that
        // is a toroidal surface â€” `torus_of` refuses every non-torus surface
        // by name, so `cylinder_of`, `cone_of`, and `torus_of` are mutually
        // exclusive on any one surface. A certified-degenerate face skips the
        // route entirely: it is the one state no recovery route may touch.
        let (polygon, failure, torus_band_attempt) = if run_torus && !rejected_terminal {
            match run_torus_annulus_for_face(
                declared_face_index,
                source_face_id,
                face,
                &shell.edges,
                &shell.vertices,
                &torus_of,
                &cylinder_curve_family_of,
                tol,
            ) {
                None => (polygon, failure, None),
                Some(Ok((mesh, conformance))) => {
                    let triangles = mesh.tri_faces().len();
                    if torus_recovery_gate && failure.is_some() {
                        if recovery_log {
                            eprintln!(
                                "RECOVERED\tsource_face_id={}\t\
                                 declared_face_index={declared_face_index}\t\
                                 triangles={}\tpath=torus",
                                source_face_id
                                    .map(|id| id.to_string())
                                    .unwrap_or_else(|| "none".into()),
                                triangles,
                            );
                        }
                        diagnosis::record_route_decision(
                            diagnosis::RecoveryRoute::TorusAnnulus,
                            true,
                            diagnosis::RouteOutcome::Recovered,
                            None,
                        );
                        (
                            Some(mesh),
                            None,
                            Some(TorusAnnulusAttempt::Recovered {
                                triangles,
                                conformance,
                            }),
                        )
                    } else {
                        // Shadow mode: record the outcome but preserve the
                        // legacy mesh and failure unchanged.
                        (
                            polygon,
                            failure,
                            Some(TorusAnnulusAttempt::Recovered {
                                triangles,
                                conformance,
                            }),
                        )
                    }
                }
                Some(Err(exit)) => {
                    diagnosis::record_route_decision(
                        diagnosis::RecoveryRoute::TorusAnnulus,
                        true,
                        diagnosis::RouteOutcome::Refused,
                        Some(exit.tag()),
                    );
                    (polygon, failure, Some(TorusAnnulusAttempt::Refused(exit)))
                }
            }
        } else {
            diagnosis::record_route_decision(
                diagnosis::RecoveryRoute::TorusAnnulus,
                torus_recovery_gate,
                match torus_recovery_gate {
                    true => diagnosis::RouteOutcome::PreconditionUnmet,
                    false => diagnosis::RouteOutcome::GateClosed,
                },
                None,
            );
            (polygon, failure, None)
        };
        // The winding retry, and it runs **last** for a reason that cost a
        // measurement to learn.
        //
        // Material parity is the boundary's winding number mod 2, so an edge
        // the boundary traversed twice separates nothing; the flood reads the
        // *set* of realized constraint edges and so toggles across it once.
        // That is the whole of `ContradictoryDualParity`: on `00009190` all
        // 126 contradicting faces have a repeated traversal and none of the
        // 23,258 that flood cleanly does.
        //
        // Run inside the first tessellation instead, this recovers the same
        // 126 faces but **pre-empts the torus annulus route on 8 of them**,
        // replacing a validated 64-triangle annulus with a 1â€“2 triangle
        // remnant. Those 8 are `two_outer_bounds_on_certified_torus_annulus`:
        // the source declares the whole bound twice, every edge is traversed
        // twice, and the winding reading correctly cancels the entire
        // boundary â€” the face is not a slit, it is malformed, and the repair
        // belongs to the route that knows that. Placing the retry after every
        // other route makes "cancels to nothing" a failure this face already
        // recovered from, rather than a mesh that replaces the recovery.
        let (polygon, failure) = match (&failure, parity_retry_boundary) {
            (Some(f), Some(boundary))
                if f.reason == TessellationFailureReason::ContradictoryDualParity =>
            {
                // The DIAG-001 record must keep describing the legacy attempt.
                let _suspension = diagnosis::SinkSuspension::new();
                // The parity reading is already `TraversalParity` on the primary
                // path (ARR-SEAM W3), so no set/reset is needed here any more.
                let retried = trimming_tessellation_result(&surface, &boundary, tol, &lattice);
                diagnosis::record_route_decision(
                    diagnosis::RecoveryRoute::WindingParity,
                    true,
                    match retried {
                        Ok(_) => diagnosis::RouteOutcome::Recovered,
                        Err(_) => diagnosis::RouteOutcome::Refused,
                    },
                    None,
                );
                match retried {
                    Ok(mesh) => {
                        if recovery_log {
                            eprintln!(
                                "RECOVERED\tsource_face_id={}\t\
                                 declared_face_index={declared_face_index}\t\
                                 triangles={}\tpath=winding_parity",
                                source_face_id
                                    .map(|id| id.to_string())
                                    .unwrap_or_else(|| "none".into()),
                                mesh.tri_faces().len(),
                            );
                        }
                        (Some(mesh), None)
                    }
                    // The retry refused in its own right â€” `NoOddParityRegion`
                    // for a boundary that cancels completely. The legacy
                    // failure is preserved exactly.
                    Err(_) => (polygon, failure),
                }
            }
            _ => (polygon, failure),
        };
        PROBE_FACE_CONTEXT.with(|context| context.set((None, usize::MAX, 0)));
        // The single terminal finalizer: exactly one record per face that ends
        // this closure without a mesh. A face recovered by a formal route has
        // `failure == None` here and emits nothing; a face whose legacy attempt
        // failed and whose every retry also failed emits its legacy record
        // once. The finalized record rides both inside the returned failure
        // and in `face_diagnoses` for the census.
        let (failure, face_diagnosis) = if diag {
            match failure {
                Some(failure) => {
                    let failure = diagnosis::finalize_and_emit(failure);
                    let record = failure.diagnostic.clone();
                    (Some(failure), Some(record))
                }
                None => (None, None),
            }
        } else {
            (failure, None)
        };
        (
            CompressedFace {
                boundaries,
                orientation: face.orientation,
                surface: polygon,
                // Tessellation is the stage most likely to produce nothing, so
                // it is the stage that most needs to say which face produced
                // nothing. `polygon` is `None` on failure, and the identity is
                // then the only thing left that can name what was lost.
                provenance: face.provenance,
            },
            failure,
            face_diagnosis,
            band_attempt,
            cone_band_attempt,
            torus_band_attempt,
        )
    };
    #[cfg(not(target_arch = "wasm32"))]
    let results: Vec<_> = shell
        .faces
        .par_iter()
        .enumerate()
        .map(tessellate_face)
        .collect();
    #[cfg(target_arch = "wasm32")]
    let results: Vec<_> = shell
        .faces
        .iter()
        .enumerate()
        .map(tessellate_face)
        .collect();
    let mut faces = Vec::with_capacity(results.len());
    let mut face_failures = Vec::with_capacity(results.len());
    let mut face_diagnoses = Vec::with_capacity(results.len());
    let mut band_attempts = Vec::with_capacity(results.len());
    let mut cone_band_attempts = Vec::with_capacity(results.len());
    let mut torus_band_attempts = Vec::with_capacity(results.len());
    for (f, ff, fd, ba, cba, tba) in results {
        faces.push(f);
        face_failures.push(ff);
        face_diagnoses.push(fd);
        band_attempts.push(ba);
        cone_band_attempts.push(cba);
        torus_band_attempts.push(tba);
    }
    MeshedShellOutcome {
        shell: MeshedCShell {
            vertices,
            // The meshed shell's edge record: polyline edges as sampled, and a
            // deliberately empty polyline for an edge whose source traversal
            // was not established. No face bound may reference the latter --
            // boundary construction fails on it -- so the empty record never
            // reaches a renderer.
            edges: edges
                .into_iter()
                .map(|established| match established {
                    EstablishedEdge::Mesh(edge) => edge,
                    EstablishedEdge::Unresolved { vertices, .. } => CompressedEdge {
                        vertices,
                        curve: PolylineCurve::from(Vec::new()),
                    },
                })
                .collect(),
            faces,
            // The meshed shell is the same source representation as the input,
            // so the declared geometric uncertainty is carried through.
            source_geometric_uncertainty: shell.source_geometric_uncertainty,
        },
        face_failures,
        face_diagnoses,
        band_attempts,
        cone_band_attempts,
        torus_band_attempts,
    }
}

/// Builds the source-evidence view of one compressed face.
///
/// Step 0 of the formal rewrite: this runs beside the legacy boundary
/// construction and feeds nothing but the probe below. Its purpose is to
/// establish, by measurement on the corpus rather than by reading the
/// converter, what the rewrite's input seam actually carries.
///
/// Three facts survive here that the legacy path discards, and they are the
/// three the deck solve will need:
///
/// - **edge-use structure.** `create_boundary` flattens a bound's edge uses
///   into one point vector, after which no arc has endpoints.
/// - **source vertex identity.** `CompressedEdge::vertices` is read for the
///   first time on this path; `create_edge` takes only `.curve`.
/// - **composed `s_b Â· s_o`.** `create_edge` applies it as `curve.inverse()`
///   and keeps nothing.
///
/// **Contracts:** retains what `TOP-005` requires; `TOP-001` identity is
/// carried but not re-checked here, since the converter's arena already
/// discharges it.
fn source_face_input_from_compressed<S, C>(
    declared_face_index: usize,
    source_face_id: Option<u64>,
    face: &CompressedFace<S>,
    edges: &[CompressedEdge<C>],
) -> std::result::Result<SourceFaceInput, SourceEvidenceError> {
    let mut bounds = Vec::with_capacity(face.boundaries.len());
    for (bound_index, wire) in face.boundaries.iter().enumerate() {
        let bound = BoundId(bound_index);
        if wire.is_empty() {
            // Not a malformed bound, and not grounds for discarding the face: a
            // collapsed `VERTEX_LOOP` contributes no trim segment by design,
            // the compressed form cannot tell that from a bound that lost its
            // edges, and the face's *other* bounds are unaffected either way.
            bounds.push(SourceBoundInput::DegenerateEvidenceUnavailable { id: bound });
            continue;
        }
        let mut edge_uses = Vec::with_capacity(wire.len());
        for (use_index, edge_idx) in wire.iter().enumerate() {
            let id = EdgeUseId::new(bound, use_index);
            let edge =
                edges
                    .get(edge_idx.index)
                    .ok_or(SourceEvidenceError::EdgeIndexOutOfRange {
                        edge_use: id,
                        index: edge_idx.index,
                    })?;
            // The edge's vertices are stated in the edge's own direction, so
            // the composed sense selects which is the *use's* start. This is
            // the one place the retained orientation is read as a fact rather
            // than performed as an inversion â€” and both orders are kept, so a
            // consumer cannot apply the same fact twice undetected.
            let source_vertices = (
                SourceVertexKey::ShellVertex(edge.vertices.0),
                SourceVertexKey::ShellVertex(edge.vertices.1),
            );
            let use_vertices = match edge_idx.orientation {
                true => source_vertices,
                false => (source_vertices.1, source_vertices.0),
            };
            edge_uses.push(SourceEdgeUseInput {
                id,
                source_edge_index: edge_idx.index,
                source_vertices,
                use_vertices,
                orientation: SourceEdgeOrientationEvidence {
                    bound_times_oriented_edge: OrientationEvidence::Retained {
                        forward: edge_idx.orientation,
                        origin: OrientationOrigin::BoundTimesOrientedEdge,
                    },
                    // Folded into the converted curve at import. Sound
                    // mechanism, no surviving Boolean.
                    edge_curve_same_sense: OrientationEvidence::HistoryErased {
                        mechanism:
                            ErasedOrientationMechanism::EdgeCurveSenseFoldedIntoConvertedCurve,
                    },
                    selected_curve_direction: OrientationEvidence::HistoryErased {
                        mechanism:
                            ErasedOrientationMechanism::SelectedCurveDirectionFoldedIntoConvertedCurve,
                    },
                },
            });
        }
        bounds.push(SourceBoundInput::EdgeUses {
            id: bound,
            edge_uses,
        });
    }
    Ok(SourceFaceInput {
        source_face_id,
        declared_face_index,
        bounds,
        orientation: SourceFaceOrientationEvidence {
            face_use_orientation: OrientationEvidence::Retained {
                forward: face.orientation,
                origin: OrientationOrigin::CompressedFaceOrientation,
            },
            // Folded in by `surface.invert()`, which is recorded as breaking
            // curve-on-surface incidence rather than only reversing the
            // parameterization. This asserts the erasure, not that any
            // particular face had `same_sense == false`.
            face_surface_same_sense: OrientationEvidence::HistoryErased {
                mechanism: ErasedOrientationMechanism::FaceSurfaceSenseFoldedViaSurfaceInvert,
            },
        },
    })
}

/// Per-run shell ordinal for the Step-1 `FaceKey`. Diagnostic only.
static SHELL_ORDINAL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// The formal envelope the ambient probe evaluates against.
///
/// **This is not a production policy.** No project document specifies values
/// for `s_max`, `n_max`, `e_max`, `w_max`, `x_max`, `v_max` or `g_max` --
/// `FORMAL_SYSTEM.md` Definition 6 says only that they are policy and that the
/// closure proof needs them finite -- so these are diagnostic constants chosen
/// to be non-binding. The one value the documents *do* fix, `r_max <= 2`, is
/// set to that maximum.
///
/// The rank clause is the only one Step 1 evaluates. Every other bound is
/// unreachable at this stage and is stated because the envelope has no
/// `Default` and must be given in full.
fn diagnostic_envelope() -> formal::FormalEnvelope {
    formal::FormalEnvelope::new(
        formal::PolicyInstanceId::new(0),
        2,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        u64::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    )
    .expect("r_max = 2 is Definition 6's own maximum")
}

/// Step-1 ambient-period probe.
///
/// One `eprintln!` per successfully converted compressed face, for the same
/// reason as the `EVIDENCE` probe: face tessellation is parallel and a record
/// split across calls interleaves. Consumers must parse order-independently and
/// key on `source_face_id` / `declared_face_index`.
///
/// The record reports the legacy `declared_rank` and `certified_rank` beside
/// the formal resolution so the two can be compared face by face. They are
/// **not expected to agree**: `certified_rank == 0` covers both "proved
/// aperiodic" and "periodicity declared and never certified", and separating
/// those is what this step exists for.
fn emit_ambient_probe(
    source_face_id: Option<u64>,
    declared_face_index: usize,
    shell_ordinal: u64,
    lattice: &CertifiedLattice,
    schema: &formal::SupportSurfaceSchema,
) {
    let declared_rank = usize::from(lattice.declared_u_period().is_some())
        + usize::from(lattice.declared_v_period().is_some());
    let legacy_certified_rank = lattice.certified_rank();
    let id = match source_face_id {
        Some(id) => id.to_string(),
        None => "none".to_string(),
    };

    let record = match formal::ambient_evidence_from_schema(
        schema,
        lattice,
        formal::LatticeOrigin::UnattributedLegacyLattice,
    ) {
        Err(error) => format!(
            "u_state=none\tv_state=none\tformal_resolution=adapter_error\tformal_rank=none\t\
             unresolved_reason=none\tinconsistency_reason=none\tunsupported_clause=none\t\
             diagnostic_hint_count=0\tauthoritative_generator_count=0\tadapter_error={}",
            error.tag(),
        ),
        Ok(evidence) => {
            let u_state = evidence.u.tag();
            let v_state = evidence.v.tag();
            let hints = evidence.diagnostic_hints().len();
            let generators = evidence.authoritative_generator_count();
            let face = formal::FaceKey {
                document: formal::DocumentScope::SingleDocumentRun,
                shell: formal::ShellKey::new(shell_ordinal),
                source_face_id: source_face_id.map(formal::SourceEntityKey::new),
                declared_face_index,
            };
            // An operational failure is not a semantic judgment, so it is
            // reported as itself rather than folded into a resolution.
            let (resolution, rank, unresolved, inconsistency, unsupported) =
                match formal::resolve_ambient_periods(evidence, &diagnostic_envelope(), face) {
                    Err(failure) => (failure.tag(), "none".to_string(), "none", "none", "none"),
                    Ok(outcome) => {
                        let tag = outcome.tag();
                        match &outcome {
                            formal::StageOutcome::Resolved(resolved) => {
                                (tag, resolved.rank().to_string(), "none", "none", "none")
                            }
                            formal::StageOutcome::Unresolved(report) => (
                                tag,
                                "none".to_string(),
                                report.reason().tag(),
                                "none",
                                "none",
                            ),
                            formal::StageOutcome::Inconsistent(report) => (
                                tag,
                                "none".to_string(),
                                "none",
                                report.reason().tag(),
                                "none",
                            ),
                            formal::StageOutcome::Unsupported(report) => (
                                tag,
                                "none".to_string(),
                                "none",
                                "none",
                                report.cause().tag(),
                            ),
                            formal::StageOutcome::Ambiguous(report) => (
                                tag,
                                "none".to_string(),
                                "none",
                                report.reason().tag(),
                                "none",
                            ),
                        }
                    }
                };
            format!(
                "u_state={u_state}\tv_state={v_state}\tformal_resolution={resolution}\t\
                 formal_rank={rank}\tunresolved_reason={unresolved}\t\
                 inconsistency_reason={inconsistency}\tunsupported_clause={unsupported}\t\
                 diagnostic_hint_count={hints}\tauthoritative_generator_count={generators}\t\
                 adapter_error=none"
            )
        }
    };
    let support_schema = schema.tag();
    eprintln!(
        "AMBIENT\tsource_face_id={id}\tdeclared_face_index={declared_face_index}\t\
         shell_ordinal={shell_ordinal}\tdeclared_rank={declared_rank}\t\
         legacy_certified_rank={legacy_certified_rank}\tsupport_schema={support_schema}\t\
         {record}"
    );
}

// ---------------------------------------------------------------------------
// The planar vertical slice, run beside the legacy tessellator
// ---------------------------------------------------------------------------

/// Run the formal planar slice for one face, when it is a candidate.
///
/// `None` means the face never entered: it is not a structurally identified
/// plane, or Step 1 did not resolve it to a certified rank-0 lattice. Those two
/// populations are already reported by the ambient probe, so they are counted
/// there rather than duplicated here as slice exits.
///
/// Nothing in here reads the legacy result, so a face's formal verdict is
/// independent of whether the legacy path succeeded on it.
#[allow(clippy::too_many_arguments)]
fn run_slice_for_face<S, C>(
    declared_face_index: usize,
    source_face_id: Option<u64>,
    shell_ordinal: u64,
    face: &CompressedFace<S>,
    edges: &[CompressedEdge<C>],
    vertices: &[Point3],
    lattice: &CertifiedLattice,
    schema: &formal::SupportSurfaceSchema,
    curve_schema_of: &impl Fn(&C) -> formal::CurveSchema,
    tol: f64,
) -> Option<FormalSliceOutcome> {
    // Step 1. The same call the ambient probe makes, on the same evidence.
    let plane = schema.plane()?;
    let evidence = formal::ambient_evidence_from_schema(
        schema,
        lattice,
        formal::LatticeOrigin::UnattributedLegacyLattice,
    )
    .ok()?;
    let key = formal::FaceKey {
        document: formal::DocumentScope::SingleDocumentRun,
        shell: formal::ShellKey::new(shell_ordinal),
        source_face_id: source_face_id.map(formal::SourceEntityKey::new),
        declared_face_index,
    };
    let formal::StageOutcome::Resolved(certified) =
        formal::resolve_ambient_periods(evidence, &diagnostic_envelope(), key).ok()?
    else {
        return None;
    };

    // Step 0. The same seam the evidence probe reports.
    let input =
        source_face_input_from_compressed(declared_face_index, source_face_id, face, edges).ok()?;

    let mut curve_of = |edge_index: usize| match edges.get(edge_index) {
        Some(edge) => curve_schema_of(&edge.curve),
        None => formal::CurveSchema::not_structurally_identified(
            formal::CurveSchemaFailure::NoStructuralReader {
                representation: "edge_index_out_of_range",
            },
        ),
    };
    let vertex_position = |vertex| match vertex {
        SourceVertexKey::ShellVertex(index) => vertices.get(index).copied(),
        _ => None,
    };

    // Both paths run on the same evidence. The hole-free slice is unchanged and
    // still reports `multiple_bounds_or_holes` for a multi-bound face, so its
    // funnel stays comparable with the frozen baseline; the holes slice
    // delegates on a single-bound face rather than answering for a population
    // the other module owns.
    let planar = formal::run_planar_slice(
        &input,
        plane,
        &certified,
        face.provenance.outer_bound,
        &mut curve_of,
        &vertex_position,
        tol,
    );
    let holes = formal::run_planar_holes_slice(
        &input,
        plane,
        &certified,
        face.provenance.outer_bound,
        &mut curve_of,
        &vertex_position,
        tol,
    );
    // The developed-curve track, run only when its probe asks for it. It
    // produces no mesh and nothing below reads it, so it cannot alter a face's
    // geometry â€” but it does cost an O(pieces^2) certified pairwise pass, and
    // that is not worth spending on every planar face of every model by
    // default.
    let developed = std::env::var_os("TRUCK_PROBE_DEVELOPED")
        .is_some()
        .then(|| formal::run_developed_face(&input, plane, &mut curve_of, &vertex_position, tol));
    Some(FormalSliceOutcome {
        planar,
        holes,
        developed,
    })
}

/// Both rank-0 formal paths' verdicts on one face.
///
/// They are kept apart rather than merged into one record: each names the
/// population it is answerable for, and a face that delegates carries no
/// verdict from the module that declined it.
struct FormalSliceOutcome {
    /// The hole-free slice's record. Always present.
    planar: formal::SliceRecord,
    /// The planar-holes slice's record. `delegated` when the face has no inner
    /// bounds.
    holes: formal::HoleSliceRecord,
    /// The developed-curve track's survey, when `TRUCK_PROBE_DEVELOPED` asked
    /// for it. An observer: it builds no mesh and nothing consumes it.
    developed: Option<formal::DevelopedFaceRecord>,
}

/// Turn a validated planar mesh into the polygon mesh the shell holds.
///
/// The normal is the support plane's chart normal, constant over the face
/// because the face is planar. It carries no material-side meaning: Step 0
/// measured the normalized physical sign as unavailable on all 110,770 edge
/// uses, and nothing here invents one.
fn planar_mesh_to_polygon(mesh: &formal::PlanarMesh) -> PolygonMesh {
    let positions = mesh.positions.clone();
    let normals = vec![mesh.chart_normal; positions.len()];
    let tri_faces: Vec<[StandardVertex; 3]> = mesh
        .triangles
        .iter()
        .map(|indices| {
            array![i => StandardVertex {
                pos: indices[i],
                uv: None,
                nor: Some(indices[i]),
            }; 3]
        })
        .collect();
    PolygonMesh::debug_new(
        StandardAttributes {
            positions,
            uv_coords: Vec::new(),
            normals,
        },
        Faces::from_tri_and_quad_faces(tri_faces, Vec::new()),
    )
}

// ---------------------------------------------------------------------------
// The rank-1 cylinder vertical slice, run beside the legacy tessellator
// (Milestone A / FORMAL-013-015)
// ---------------------------------------------------------------------------

/// Diagnostic materialization budget for [`formal::build_working_cover`].
///
/// Not a proved bound: a real rank-1 face's angular sweep is bounded by a
/// handful of source arcs, so a working cover needing thousands of deck
/// copies is already a sign of a pathological input rather than a face this
/// budget should quietly widen for. Chosen generously enough that no
/// legitimate single-face cylinder disk should ever hit it; a face that does
/// exits `OperationalFailure` rather than materializing unboundedly.
const CYLINDER_DECK_BUDGET: formal::DeckBudget = formal::DeckBudget {
    deck_width_cap: 4096,
};

/// One face's rank-1 cylinder-slice verdict, for the funnel and the recovery
/// gate.
#[derive(Debug, Clone)]
struct CylinderSliceRecord {
    /// The furthest stage reached: `"surface"`, `"evidence"`, `"traversal"`,
    /// `"witness"`, `"holonomy"`, `"cover"`, `"arrangement"`, `"mesh"`, or
    /// `"final_validity"` on success.
    stage: &'static str,
    /// The taxonomy category of the outcome. `Resolved` only when a valid
    /// physical mesh was certified.
    category: formal::SliceCategory,
    /// A stable tag naming the specific exit, or `"resolved"`.
    tag: &'static str,
    /// How many of the face's edge uses classified as an axial line.
    line_edge_uses: usize,
    /// How many classified as a circumferential arc.
    arc_edge_uses: usize,
    /// How many classified as neither (unsupported representation, or a
    /// non-circular affine image).
    unsupported_edge_uses: usize,
    /// The certified physical mesh, when fully resolved.
    mesh: Option<PolygonMesh>,
    /// Triangle count, when resolved (`0` otherwise), for the probe line.
    triangles: usize,
}

impl CylinderSliceRecord {
    fn exit(
        stage: &'static str,
        category: formal::SliceCategory,
        tag: &'static str,
        line_edge_uses: usize,
        arc_edge_uses: usize,
        unsupported_edge_uses: usize,
    ) -> Self {
        Self {
            stage,
            category,
            tag,
            line_edge_uses,
            arc_edge_uses,
            unsupported_edge_uses,
            mesh: None,
            triangles: 0,
        }
    }
}

/// Map a certified cylinder mesh's developed triangulation and physical lift
/// onto the [`PolygonMesh`] the shell holds.
///
/// Unlike [`planar_mesh_to_polygon`]'s single chart normal, a cylinder is
/// curved: each vertex gets its own outward radial normal, `normalize(x -
/// origin - axial(x) * axis)`, computed directly from the certified schema
/// rather than approximated from the mesh.
fn cylinder_mesh_to_polygon(
    mesh: &formal::CertifiedCylinderMesh,
    schema: &formal::CylinderSchema,
) -> PolygonMesh {
    cylinder_polygon_from_lifted(&mesh.physical_vertices, &mesh.developed.triangles, schema)
}

/// A `PolygonMesh` from vertices already lifted onto a certified cylinder.
///
/// Shared by the disk and band routes because the step is the same one: the
/// lift is done, the triangles index it, and the outward normal at a point of
/// a cylinder is its own radial direction. Nothing here re-derives geometry.
fn cylinder_polygon_from_lifted(
    positions: &[Point3],
    triangles: &[[usize; 3]],
    schema: &formal::CylinderSchema,
) -> PolygonMesh {
    let positions = positions.to_vec();
    let normals: Vec<Vector3> = positions
        .iter()
        .map(|p| {
            let r = *p - schema.origin();
            let radial = r - r.dot(schema.axis()) * schema.axis();
            let magnitude = radial.magnitude();
            match magnitude > 0.0 {
                true => radial / magnitude,
                false => schema.axis(),
            }
        })
        .collect();
    let tri_faces: Vec<[StandardVertex; 3]> = triangles
        .iter()
        .map(|indices| {
            array![i => StandardVertex {
                pos: indices[i],
                uv: None,
                nor: Some(indices[i]),
            }; 3]
        })
        .collect();
    PolygonMesh::debug_new(
        StandardAttributes {
            positions,
            uv_coords: Vec::new(),
            normals,
        },
        Faces::from_tri_and_quad_faces(tri_faces, Vec::new()),
    )
}

/// Run the formal rank-1 cylinder slice for one face, when it is a
/// candidate: the authoritative surface adapter, real source traversal, real
/// source curve adapters, then FORMAL-007 through FORMAL-012 unchanged.
///
/// Nothing in here reads the legacy result, so a face's formal verdict is
/// independent of whether the legacy path succeeded on it â€” the same
/// discipline [`run_slice_for_face`] follows for the planar rank-0 route.
#[allow(clippy::too_many_arguments)]
fn run_cylinder_slice_for_face<S, C>(
    declared_face_index: usize,
    source_face_id: Option<u64>,
    face: &CompressedFace<S>,
    edges: &[CompressedEdge<C>],
    vertices: &[Point3],
    cylinder_of: &impl Fn(&S) -> std::result::Result<formal::CertifiedEmbeddedCylinder, &'static str>,
    cylinder_curve_schema_of: &impl Fn(&C) -> formal::CurveSchema,
    cylinder_curve_family_of: &impl Fn(&C) -> Option<formal::SourceCurveFamily>,
    tol: f64,
) -> CylinderSliceRecord {
    let cylinder = match cylinder_of(&face.surface) {
        Ok(cylinder) => cylinder,
        // `tag` distinguishes "not a `CylindricalSurface` representation at
        // all" from "a `CylindricalSurface` representation that
        // `identify_cylinder` itself refused" (a cone smuggled in by a
        // degenerate transform, a zero radius, an unverified angular
        // period) â€” see `look::step::cylinder::CylinderSurfaceAdapterFailure`,
        // whose `.tag()` this closure forwards without `truck-meshalgo`
        // depending on that type.
        Err(tag) => {
            return CylinderSliceRecord::exit(
                "surface",
                formal::SliceCategory::Unsupported,
                tag,
                0,
                0,
                0,
            )
        }
    };

    // The curve-family funnel is counted independently of the traversal
    // gate below, so a face that fails traversal for an unrelated reason
    // (a second declared outer bound, a broken cyclic join) still reports
    // what its edges structurally *were*.
    let (mut line_edge_uses, mut arc_edge_uses, mut unsupported_edge_uses) =
        (0usize, 0usize, 0usize);
    for wire in &face.boundaries {
        for edge_idx in wire {
            match edges
                .get(edge_idx.index)
                .and_then(|edge| cylinder_curve_family_of(&edge.curve))
            {
                Some(formal::SourceCurveFamily::Line) => line_edge_uses += 1,
                // A complete circle counts in the same arc column: the funnel
                // reports which structural family an edge use presented, and
                // a closed circle is a circular arc that covers its whole
                // period rather than a separate family.
                Some(formal::SourceCurveFamily::CircularArc { .. })
                | Some(formal::SourceCurveFamily::CompleteCircle { .. }) => arc_edge_uses += 1,
                None => unsupported_edge_uses += 1,
            }
        }
    }

    let Ok(input) =
        source_face_input_from_compressed(declared_face_index, source_face_id, face, edges)
    else {
        return CylinderSliceRecord::exit(
            "evidence",
            formal::SliceCategory::Unresolved,
            "source_evidence_error",
            line_edge_uses,
            arc_edge_uses,
            unsupported_edge_uses,
        );
    };

    let mut curve_of = |edge_index: usize| match edges.get(edge_index) {
        Some(edge) => cylinder_curve_schema_of(&edge.curve),
        None => formal::CurveSchema::not_structurally_identified(
            formal::CurveSchemaFailure::NoStructuralReader {
                representation: "edge_index_out_of_range",
            },
        ),
    };

    let traversal_record = match formal::build_cylinder_face(
        source_face_id,
        cylinder,
        &input,
        face.provenance.outer_bound,
        &mut curve_of,
    ) {
        Ok(record) => record,
        Err(exit) => {
            return CylinderSliceRecord::exit(
                "traversal",
                exit.category(),
                exit.tag(),
                line_edge_uses,
                arc_edge_uses,
                unsupported_edge_uses,
            )
        }
    };

    let schema = traversal_record.cylinder.schema().clone();
    let vertex_position = |vertex| match vertex {
        SourceVertexKey::ShellVertex(index) => vertices.get(index).copied(),
        _ => None,
    };
    // The single production classification route: derives the witness class
    // and (for an arc) declared sweep from the edge's own source
    // representation, never from a caller assertion. By construction this
    // agrees with the gate `curve_of` above already applied â€” both route
    // through `cylinder_curve_family_of`/`cylinder_curve_schema_of`'s
    // identical `decode_transformed_circle` check â€” so the `Line` fallback
    // below is defensively unreachable, not a silent misclassification: a
    // genuinely wrong family here still fails the witness's own on-cylinder
    // and constant-coordinate checks rather than certifying incorrectly.
    let family_of = |edge_use: EdgeUseId| {
        traversal_record
            .traversal
            .occurrences
            .iter()
            .find(|occurrence| occurrence.edge_use == edge_use)
            .and_then(|occurrence| edges.get(occurrence.source_edge_index))
            .and_then(|edge| cylinder_curve_family_of(&edge.curve))
            .unwrap_or(formal::SourceCurveFamily::Line)
    };

    let developed = match formal::develop_traversal_from_source(
        &traversal_record.traversal,
        &schema,
        &vertex_position,
        &family_of,
    ) {
        Ok(developed) => developed,
        Err(exit) => {
            return CylinderSliceRecord::exit(
                "witness",
                exit.category(),
                exit.tag(),
                line_edge_uses,
                arc_edge_uses,
                unsupported_edge_uses,
            )
        }
    };

    let lift = match formal::propagate_and_classify_holonomy(&developed, schema.deck_generator()) {
        Ok(lift) => lift,
        Err(exit) => {
            return CylinderSliceRecord::exit(
                "holonomy",
                exit.category(),
                exit.tag(),
                line_edge_uses,
                arc_edge_uses,
                unsupported_edge_uses,
            )
        }
    };

    let cover = match formal::build_working_cover(
        &developed.witnesses,
        &lift.placements,
        schema.deck_generator(),
        CYLINDER_DECK_BUDGET,
    ) {
        Ok(cover) => cover,
        Err(exit) => {
            return CylinderSliceRecord::exit(
                "cover",
                exit.category(),
                exit.tag(),
                line_edge_uses,
                arc_edge_uses,
                unsupported_edge_uses,
            )
        }
    };

    let disk = match formal::certify_cylinder_disk(
        &developed.edge_uses,
        &developed.witnesses,
        &lift.placements,
        schema.deck_generator(),
        face.provenance.outer_bound,
        &cover.materialized_copies,
    ) {
        Ok(disk) => disk,
        Err(exit) => {
            return CylinderSliceRecord::exit(
                "arrangement",
                exit.category(),
                exit.tag(),
                line_edge_uses,
                arc_edge_uses,
                unsupported_edge_uses,
            )
        }
    };

    let occurrences = formal::placed_occurrences(
        &developed.edge_uses,
        &developed.witnesses,
        &lift.placements,
        &schema.deck_generator(),
    );
    let mesh = match formal::certify_cylinder_mesh(&disk, &occurrences, &schema, tol) {
        Ok(mesh) => mesh,
        Err(exit) => {
            return CylinderSliceRecord::exit(
                "mesh",
                exit.category(),
                exit.tag(),
                line_edge_uses,
                arc_edge_uses,
                unsupported_edge_uses,
            )
        }
    };

    let triangles = mesh.developed.triangles.len();
    let polygon = cylinder_mesh_to_polygon(&mesh, &schema);
    CylinderSliceRecord {
        stage: "final_validity",
        category: formal::SliceCategory::Resolved,
        tag: "resolved",
        line_edge_uses,
        arc_edge_uses,
        unsupported_edge_uses,
        mesh: Some(polygon),
        triangles,
    }
}

/// Run the formal cylinder-band path for one face, when it is eligible.
///
/// `None` is "not eligible, nothing was attempted": no certified cylinder
/// support, no source evidence, or a bound count other than two authoritative
/// bounds. `Some` means `run_cylinder_band` was actually called and the value
/// is its verdict.
///
/// This is an adapter and nothing more. Every input it hands the band path is
/// one production already produces:
///
/// - the **cylinder** comes from `cylinder_of`, the same authoritative surface
///   adapter the rank-1 disk route uses, so "certified cylinder support" is
///   that certificate and not a surface-kind string match;
/// - the **face** comes from [`source_face_input_from_compressed`], the same
///   Step-0 evidence seam the disk route reads;
/// - the **curve schema** and **curve family** come from the same two
///   structural readers the disk route passes;
/// - the **vertex positions** are the shell's own vertex table, keyed by
///   [`SourceVertexKey::ShellVertex`];
/// - the **outer bound** is the face's declared standing, unchanged.
///
/// The one thing built here is `family_of`, and it is built from *identity*:
/// [`EdgeUseId`] selects the source edge use, the use names its
/// `source_edge_index`, and that indexes the shell's edge table. No tessellated
/// coordinate is consulted to decide what a curve is. (The disk route resolves
/// the same map through its traversal record; there is no traversal record here
/// because the band path runs one traversal per bound *inside*
/// `certify_cylinder_band`, so the map is resolved through the evidence the
/// traversal is itself built from.)
#[allow(clippy::too_many_arguments)]
fn run_cylinder_band_for_face<S, C>(
    declared_face_index: usize,
    source_face_id: Option<u64>,
    face: &CompressedFace<S>,
    edges: &[CompressedEdge<C>],
    vertices: &[Point3],
    cylinder_of: &impl Fn(&S) -> std::result::Result<formal::CertifiedEmbeddedCylinder, &'static str>,
    cylinder_curve_schema_of: &impl Fn(&C) -> formal::CurveSchema,
    cylinder_curve_family_of: &impl Fn(&C) -> Option<formal::SourceCurveFamily>,
    tol: f64,
) -> Option<
    std::result::Result<
        (PolygonMesh, formal::cylinder_band::SourceConformance),
        formal::cylinder_band::BandExit,
    >,
> {
    use diagnosis::{RecoveryRoute::CylinderBand, RouteIneligible};
    let Ok(cylinder) = cylinder_of(&face.surface) else {
        diagnosis::record_route_ineligible(CylinderBand, RouteIneligible::SurfaceNotCertified);
        return None;
    };
    let Ok(input) =
        source_face_input_from_compressed(declared_face_index, source_face_id, face, edges)
    else {
        diagnosis::record_route_ineligible(CylinderBand, RouteIneligible::SourceInputUnavailable);
        return None;
    };
    // Exactly two bounds, and both of them authoritative. A face with a
    // degenerate-evidence bound is not a two-bound face with one bound missing;
    // it is a face this route has no evidence for, and it is left alone rather
    // than attempted and refused.
    if input.bounds.len() != 2 || input.regular_bound_count() != 2 {
        diagnosis::record_route_ineligible(
            CylinderBand,
            RouteIneligible::BoundsNotTwoAuthoritative,
        );
        return None;
    }

    let schema = cylinder.schema().clone();
    let mut curve_of = |edge_index: usize| match edges.get(edge_index) {
        Some(edge) => cylinder_curve_schema_of(&edge.curve),
        None => formal::CurveSchema::not_structurally_identified(
            formal::CurveSchemaFailure::NoStructuralReader {
                representation: "edge_index_out_of_range",
            },
        ),
    };
    let vertex_position = |vertex| match vertex {
        SourceVertexKey::ShellVertex(index) => vertices.get(index).copied(),
        _ => None,
    };
    // The same defensive `Line` fallback the disk route carries, for the same
    // reason: an edge use whose family cannot be read is not silently
    // misclassified, because a wrong family still fails the witness's own
    // on-cylinder and constant-coordinate checks rather than certifying.
    let family_of = |edge_use: EdgeUseId| {
        input
            .edge_uses()
            .find(|use_| use_.id == edge_use)
            .and_then(|use_| edges.get(use_.source_edge_index))
            .and_then(|edge| cylinder_curve_family_of(&edge.curve))
            .unwrap_or(formal::SourceCurveFamily::Line)
    };

    Some(
        formal::cylinder_band::run_cylinder_band(
            source_face_id,
            cylinder,
            &input,
            face.provenance.outer_bound,
            &mut curve_of,
            &vertex_position,
            &family_of,
            tol,
        )
        .map(|(_, mesh)| {
            (
                cylinder_polygon_from_lifted(
                    &mesh.physical_vertices,
                    &mesh.developed.triangles,
                    &schema,
                ),
                mesh.conformance,
            )
        }),
    )
}

/// Build the recovered conical band's mesh, with a per-vertex normal read from
/// the certified cone rather than from the triangles.
///
/// The cone's outward unit normal at a point is perpendicular to the generator
/// through it and to the parallel through it, which in the certified frame is
/// the radial direction tilted back by the half-angle: `(axis_component,
/// radial_component)` proportional to `(-slope Â· sign(s), 1)`, normalized. It
/// is derived from the certificate â€” the apex, the axis and the half-angle â€”
/// and not averaged from adjacent facets, so a coarse band and a fine one carry
/// the same normal field.
///
/// At the apex the normal is undefined, and every vertex of a certified band is
/// strictly off it; the fallback exists only so the function is total.
fn cone_polygon_from_lifted(
    positions: &[Point3],
    triangles: &[[usize; 3]],
    schema: &formal::ConeSchema,
) -> PolygonMesh {
    let positions = positions.to_vec();
    let slope = schema.slope().get();
    let scale = 1.0 / (1.0 + slope * slope).sqrt();
    let normals: Vec<Vector3> = positions
        .iter()
        .map(|p| {
            let r = *p - schema.apex();
            let s = r.dot(schema.axis());
            let radial = r - s * schema.axis();
            let magnitude = radial.magnitude();
            match magnitude > 0.0 {
                true => scale * (radial / magnitude - slope * s.signum() * schema.axis()),
                false => schema.axis(),
            }
        })
        .collect();
    let tri_faces: Vec<[StandardVertex; 3]> = triangles
        .iter()
        .map(|indices| {
            array![i => StandardVertex {
                pos: indices[i],
                uv: None,
                nor: Some(indices[i]),
            }; 3]
        })
        .collect();
    PolygonMesh::debug_new(
        StandardAttributes {
            positions,
            uv_coords: Vec::new(),
            normals,
        },
        Faces::from_tri_and_quad_faces(tri_faces, Vec::new()),
    )
}

/// Run the formal conical essential-band path for one face, when it is
/// eligible.
///
/// `None` is "not eligible, nothing was attempted": no certified cone support,
/// no source evidence, or a bound count other than two authoritative bounds.
/// `Some` means `run_conical_essential_band` was actually called and the value
/// is its verdict.
///
/// This is an adapter and nothing more, and every input it hands over is one
/// production already produces â€” the same list [`run_cylinder_band_for_face`]
/// documents, with `cone_of` in place of `cylinder_of`. The two curve readers
/// are literally the same closures: they classify a source curve into a
/// [`formal::SourceCurveFamily`] and know nothing about the ambient surface, so
/// a complete source `CIRCLE` is read identically whichever cell will consume
/// it. What differs is entirely in what the cell then requires of that circle,
/// and that lives in [`formal::cone_band`].
#[allow(clippy::too_many_arguments)]
fn run_conical_band_for_face<S, C>(
    declared_face_index: usize,
    source_face_id: Option<u64>,
    face: &CompressedFace<S>,
    edges: &[CompressedEdge<C>],
    vertices: &[Point3],
    cone_of: &impl Fn(&S) -> std::result::Result<formal::CertifiedEmbeddedCone, &'static str>,
    cylinder_curve_schema_of: &impl Fn(&C) -> formal::CurveSchema,
    cylinder_curve_family_of: &impl Fn(&C) -> Option<formal::SourceCurveFamily>,
    tol: f64,
) -> Option<
    std::result::Result<
        (
            PolygonMesh,
            formal::cone::Nappe,
            formal::cone_band::ConicalSourceStanding,
        ),
        formal::cone_band::ConicalBandExit,
    >,
> {
    use diagnosis::{RecoveryRoute::ConeBand, RouteIneligible};
    let Ok(cone) = cone_of(&face.surface) else {
        diagnosis::record_route_ineligible(ConeBand, RouteIneligible::SurfaceNotCertified);
        return None;
    };
    let Ok(input) =
        source_face_input_from_compressed(declared_face_index, source_face_id, face, edges)
    else {
        diagnosis::record_route_ineligible(ConeBand, RouteIneligible::SourceInputUnavailable);
        return None;
    };
    // Exactly two bounds, and both of them authoritative. A face with a
    // degenerate-evidence bound is not a two-bound face with one bound missing;
    // it is a face this route has no evidence for, and it is left alone rather
    // than attempted and refused.
    //
    // This exit is the cone signature: a one-bound apex cone never enters this
    // route and therefore falls through to the generic lift, which was
    // invisible while every early return was the same bare `None`.
    if input.bounds.len() != 2 || input.regular_bound_count() != 2 {
        diagnosis::record_route_ineligible(ConeBand, RouteIneligible::BoundsNotTwoAuthoritative);
        return None;
    }

    let schema = cone.schema().clone();
    let mut curve_of = |edge_index: usize| match edges.get(edge_index) {
        Some(edge) => cylinder_curve_schema_of(&edge.curve),
        None => formal::CurveSchema::not_structurally_identified(
            formal::CurveSchemaFailure::NoStructuralReader {
                representation: "edge_index_out_of_range",
            },
        ),
    };
    let vertex_position = |vertex| match vertex {
        SourceVertexKey::ShellVertex(index) => vertices.get(index).copied(),
        _ => None,
    };
    // Resolved from *identity*: `EdgeUseId` selects the source edge use, the
    // use names its `source_edge_index`, and that indexes the shell's edge
    // table. No tessellated coordinate is consulted to decide what a curve is.
    //
    // Unlike the cylinder route this has no `Line` fallback, and that is a
    // deliberate difference rather than an oversight. This cell admits exactly
    // one curve family, so an edge use whose family cannot be read has nothing
    // it could safely be defaulted to; `None` reaches
    // `BoundNotACompleteSourceCircle` and says the boundary was not readable as
    // a complete circle, which is the true statement.
    let family_of = |edge_use: EdgeUseId| {
        input
            .edge_uses()
            .find(|use_| use_.id == edge_use)
            .and_then(|use_| edges.get(use_.source_edge_index))
            .and_then(|edge| cylinder_curve_family_of(&edge.curve))
    };

    Some(
        formal::cone_band::run_conical_essential_band(
            source_face_id,
            cone,
            &input,
            face.provenance.outer_bound,
            &mut curve_of,
            &vertex_position,
            &family_of,
            tol,
        )
        .map(|(_, mesh)| {
            (
                cone_polygon_from_lifted(
                    &mesh.physical_vertices,
                    &mesh.developed.triangles,
                    &schema,
                ),
                mesh.nappe,
                mesh.standing,
            )
        }),
    )
}

/// Build a `PolygonMesh` from a realized torus annulus, with per-vertex normals
/// recomputed from the certified torus surface (not averaged from adjacent
/// facets).
fn torus_polygon_from_realized(
    realized: &formal::torus_realize::RealizedTorusAnnulus,
    deck: &formal::torus::CertifiedRankTwoDeck,
) -> PolygonMesh {
    let schema = deck.schema();
    let center = schema.center();
    let axis = schema.axis();
    let large = schema.large_radius().get();
    let small = schema.small_radius().get();
    let positions = realized.vertices.clone();
    let normals: Vec<Vector3> = positions
        .iter()
        .map(|p| {
            let rel = *p - center;
            let h = rel.dot(axis);
            let radial = rel - h * axis;
            let magnitude = radial.magnitude();
            match magnitude > 0.0 {
                true => {
                    let radial_dir = radial / magnitude;
                    let cos_v = (magnitude - large) / small;
                    let sin_v = h / small;
                    let n = cos_v * radial_dir + sin_v * axis;
                    let n_mag = n.magnitude();
                    if n_mag > 0.0 {
                        n / n_mag
                    } else {
                        axis
                    }
                }
                false => axis,
            }
        })
        .collect();
    let tri_faces: Vec<[StandardVertex; 3]> = realized
        .triangles
        .iter()
        .map(|indices| {
            array![i => StandardVertex {
                pos: indices[i],
                uv: None,
                nor: Some(indices[i]),
            }; 3]
        })
        .collect();
    PolygonMesh::debug_new(
        StandardAttributes {
            positions,
            uv_coords: Vec::new(),
            normals,
        },
        Faces::from_tri_and_quad_faces(tri_faces, Vec::new()),
    )
}

/// Run the torus annulus route for one face, when it is eligible.
///
/// `None` is "not eligible, nothing was attempted": no certified torus support.
/// `Some` means the certification pipeline was actually run and the value is
/// its verdict â€” `Ok` for a certified and realized annulus, `Err` for a typed
/// refusal.
///
/// This adapter mirrors [`run_cylinder_band_for_face`] and
/// [`run_conical_band_for_face`]: it identifies the surface, extracts the
/// boundary loop placements from the source edges, certifies each circle on the
/// torus via the whole-interval Fourier test, checks homology and material
/// authority, and realizes the annulus mesh. The typed exit maps each formal
/// stage's refusal to one category of the corrected torus census.
#[allow(clippy::too_many_arguments)]
fn run_torus_annulus_for_face<S, C>(
    _declared_face_index: usize,
    _source_face_id: Option<u64>,
    face: &CompressedFace<S>,
    edges: &[CompressedEdge<C>],
    _vertices: &[Point3],
    torus_of: &impl Fn(&S) -> std::result::Result<formal::CertifiedEmbeddedTorus, &'static str>,
    cylinder_curve_family_of: &impl Fn(&C) -> Option<formal::SourceCurveFamily>,
    tol: f64,
) -> Option<
    std::result::Result<
        (PolygonMesh, formal::torus_cell::ConformanceTag),
        formal::TorusAnnulusExit,
    >,
>
where
    C: PolylineableCurve,
    S: PreMeshableSurface,
{
    use formal::torus_cell::{
        BoundaryLoopPlacement, ConformanceTag, SourceBoundaryComposition, TorusCellFailure,
        TwoOuterBoundMalformation,
    };
    use formal::{CircleFamily, CircleOnTorusStatus};
    use truck_topology::compress::OuterBoundStanding;

    // 1. Identify the torus. `None` (not eligible) when the surface is not a
    //    certified toroidal surface.
    let embedded = match torus_of(&face.surface) {
        Ok(embedded) => embedded,
        Err(_) => {
            diagnosis::record_route_ineligible(
                diagnosis::RecoveryRoute::TorusAnnulus,
                diagnosis::RouteIneligible::SurfaceNotCertified,
            );
            return None;
        }
    };
    let deck = embedded.deck();
    let schema = deck.schema();
    let torus_center = schema.center();
    let torus_axis = schema.axis();
    let large = schema.large_radius().get();
    let small = schema.small_radius().get();

    // 2. Extract boundary loop placements. Each wire must reduce to exactly one
    //    complete source circle; any other edge or a multi-circle wire marks
    //    `extra` and the face is not a two-complete-circle annulus.
    let mut circle_placements: Vec<(formal::CompleteCirclePlacement, bool)> = Vec::new();
    let mut extra = false;
    for wire in &face.boundaries {
        let mut wire_circles: Vec<(formal::CompleteCirclePlacement, bool)> = Vec::new();
        for edge_ref in wire {
            let Some(edge) = edges.get(edge_ref.index) else {
                extra = true;
                continue;
            };
            match cylinder_curve_family_of(&edge.curve) {
                Some(formal::SourceCurveFamily::CompleteCircle { placement }) => {
                    wire_circles.push((placement, edge_ref.orientation));
                }
                _ => {
                    extra = true;
                }
            }
        }
        if wire_circles.len() == 1 {
            circle_placements.push(wire_circles[0]);
        } else {
            extra = true;
        }
    }
    if circle_placements.len() != 2 {
        return Some(Err(formal::TorusAnnulusExit::NotEligible));
    }

    // 3. Source boundary composition: the double-outer-bound malformation is
    //    detected from the face's provenance, not inferred from bound count.
    let outer_bound_malformation = match face.provenance.outer_bound {
        OuterBoundStanding::Declared { declared_count, .. } if declared_count >= 2 => {
            Some(TwoOuterBoundMalformation)
        }
        _ => None,
    };
    let composition = SourceBoundaryComposition {
        component_count: face.boundaries.len(),
        extra_source_edge: extra,
        outer_bound_malformation,
    };

    // 4. Build BoundaryLoopPlacement with a placeholder sign for the on-torus
    //    certification. The sign is not read by `certify_circle_on_torus`; it
    //    is only needed by `certify_torus_annular_cell` for the material
    //    authority check, and is filled in after the winding is certified.
    let (pa, orient_a) = circle_placements[0];
    let (pb, orient_b) = circle_placements[1];
    let placement_a = BoundaryLoopPlacement {
        center: pa.center,
        normal: pa.sweep_axis,
        radius: pa.radius,
        effective_orientation_sign: 0,
    };
    let placement_b = BoundaryLoopPlacement {
        center: pb.center,
        normal: pb.sweep_axis,
        radius: pb.radius,
        effective_orientation_sign: 0,
    };

    // 5. Certify each circle on the torus via the whole-interval Fourier test.
    //    This is scale-invariant, unlike the cell's own `certify_loop` check.
    let status_a = formal::certify_circle_on_torus(deck, &placement_a);
    let status_b = formal::certify_circle_on_torus(deck, &placement_b);

    let witness_a = match &status_a {
        CircleOnTorusStatus::CertifiedOnTorus { witness } => *witness,
        CircleOnTorusStatus::ProvedNotOnTorus { .. } => {
            return Some(Err(formal::TorusAnnulusExit::CircleNotOnTorus));
        }
        CircleOnTorusStatus::OnTorusUnresolved { .. } => {
            return Some(Err(formal::TorusAnnulusExit::CertificationFailure));
        }
        CircleOnTorusStatus::OperationalFailure => {
            return Some(Err(formal::TorusAnnulusExit::OperationalFailure));
        }
    };
    let witness_b = match &status_b {
        CircleOnTorusStatus::CertifiedOnTorus { witness } => *witness,
        CircleOnTorusStatus::ProvedNotOnTorus { .. } => {
            return Some(Err(formal::TorusAnnulusExit::CircleNotOnTorus));
        }
        CircleOnTorusStatus::OnTorusUnresolved { .. } => {
            return Some(Err(formal::TorusAnnulusExit::CertificationFailure));
        }
        CircleOnTorusStatus::OperationalFailure => {
            return Some(Err(formal::TorusAnnulusExit::OperationalFailure));
        }
    };

    // 6. Primitivity: only parallel `(Â±1, 0)` and meridian `(0, Â±1)` windings
    //    are admitted â€” the two-complete-circle parallel/meridian annulus
    //    theorem. Diagonal `(Â±1, Â±1)` and other primitive windings are refused.
    let family_a = witness_a.family;
    let family_b = witness_b.family;
    if !matches!(family_a, CircleFamily::Parallel | CircleFamily::Meridian)
        || !matches!(family_b, CircleFamily::Parallel | CircleFamily::Meridian)
    {
        return Some(Err(formal::TorusAnnulusExit::IncompatibleLoopWindings));
    }

    // 7. Homology: the two loops must have the same unsigned winding.
    let wa = witness_a.winding;
    let wb = witness_b.winding;
    if wa[0].unsigned_abs() != wb[0].unsigned_abs() || wa[1].unsigned_abs() != wb[1].unsigned_abs()
    {
        return Some(Err(formal::TorusAnnulusExit::IncompatibleLoopWindings));
    }

    // 8. Effective orientation sign. The winding from `lift_circle_winding`
    //    includes the `Processor`'s curve orientation (folded into
    //    `sweep_axis`), but the material-authority check must use the sign
    //    WITHOUT the curve orientation â€” the curve orientation is a property
    //    of the curve's parameterization, not of the loop's traversal in the
    //    face's boundary. The edge use orientation (already folded in
    //    `CompressedEdgeIndex::orientation`) is the correct place for the
    //    traversal direction.
    //
    //    `sign_c` (without curve orientation) = `winding_sign / curve_orientation`
    //    = `winding_sign * curve_orientation` (since orientation is Â±1).
    let curve_orient_a = if pa.curve_orientation { 1 } else { -1 };
    let curve_orient_b = if pb.curve_orientation { 1 } else { -1 };
    let sign_a = (if family_a == CircleFamily::Parallel {
        wa[0]
    } else {
        wa[1]
    }) * curve_orient_a;
    let sign_b = (if family_b == CircleFamily::Parallel {
        wb[0]
    } else {
        wb[1]
    }) * curve_orient_b;
    let eff_a = (sign_a * if orient_a { 1 } else { -1 }) as i8;
    let eff_b = (sign_b * if orient_b { 1 } else { -1 }) as i8;

    let placement_a = BoundaryLoopPlacement {
        effective_orientation_sign: eff_a,
        ..placement_a
    };
    let placement_b = BoundaryLoopPlacement {
        effective_orientation_sign: eff_b,
        ..placement_b
    };

    // 9. Certify the annular cell using the pre-certified witnesses, which
    //    skips the cell's scale-relative on-torus check (already discharged
    //    by the Fourier test). The disjointness and material authority checks
    //    are identical to `certify_torus_annular_cell`.
    let cell = match formal::certify_torus_annular_cell_with_witnesses(
        deck,
        placement_a,
        placement_b,
        witness_a,
        witness_b,
        &composition,
    ) {
        Ok(cell) => cell,
        Err(TorusCellFailure::InconsistentBoundaryHomology) => {
            return Some(Err(formal::TorusAnnulusExit::InconsistentBoundaryHomology));
        }
        Err(TorusCellFailure::IntersectingBoundaries) => {
            return Some(Err(formal::TorusAnnulusExit::IntersectingBoundaries));
        }
        Err(TorusCellFailure::NonprimitiveWinding) | Err(TorusCellFailure::InhomologousLoops) => {
            return Some(Err(formal::TorusAnnulusExit::IncompatibleLoopWindings));
        }
        Err(TorusCellFailure::SourceContradiction) => {
            return Some(Err(formal::TorusAnnulusExit::CircleNotOnTorus));
        }
        Err(TorusCellFailure::WrongSourceBoundaryComponentCount)
        | Err(TorusCellFailure::ExtraSourceEdgePresent) => {
            return Some(Err(formal::TorusAnnulusExit::NotEligible));
        }
        Err(TorusCellFailure::UnresolvedMaterialAuthority) => {
            return Some(Err(formal::TorusAnnulusExit::CertificationFailure));
        }
    };

    // 10. Realize the annulus mesh. Grid resolution is derived from the
    //     tolerance and the torus radii, so chord deviation stays below `tol`.
    let nu = ((std::f64::consts::TAU * (large + small) / tol).sqrt() as usize)
        .max(8)
        .min(256);
    let nv = ((std::f64::consts::TAU * small / tol).sqrt() as usize)
        .max(4)
        .min(128);
    let realized =
        match formal::realize_torus_annulus(&cell, embedded.entity(), embedded.transform(), nu, nv)
        {
            Ok(r) => r,
            Err(_) => return Some(Err(formal::TorusAnnulusExit::RealizationFailure)),
        };

    // 11. Convert to PolygonMesh with per-vertex normals from the torus surface.
    let mesh = torus_polygon_from_realized(&realized, deck);
    let conformance = cell.material_authority().conformance();
    Some(Ok((mesh, conformance)))
}

/// `CYLINDER\t...` diagnostic probe, one line per candidate face. Emitted
/// from inside a parallel tessellation, so consumers must parse
/// order-independently and key on `source_face_id` / `declared_face_index`,
/// exactly as the `SLICE`/`HOLES` probes already require.
fn emit_cylinder_probe(
    source_face_id: Option<u64>,
    declared_face_index: usize,
    shell_ordinal: u64,
    record: &CylinderSliceRecord,
) {
    let id = source_face_id
        .map(|id| id.to_string())
        .unwrap_or_else(|| "none".to_string());
    eprintln!(
        "CYLINDER\tsource_face_id={id}\tdeclared_face_index={declared_face_index}\t\
         shell_ordinal={shell_ordinal}\tstage={}\tcategory={:?}\texit={}\t\
         line_edge_uses={}\tarc_edge_uses={}\tunsupported_edge_uses={}\t\
         triangles={}",
        record.stage,
        record.category,
        record.tag,
        record.line_edge_uses,
        record.arc_edge_uses,
        record.unsupported_edge_uses,
        record.triangles,
    );
}

/// One tab-separated record per candidate face, for the funnel and the
/// obstruction histogram.
///
/// Emitted from inside a parallel tessellation, so consumers must parse
/// order-independently and key on `source_face_id` / `declared_face_index`.
/// One tab-separated record per multi-bound candidate face, for the
/// planar-holes funnel.
///
/// A face that delegated â€” no inner bounds â€” is not reported: it belongs to the
/// `SLICE` funnel, and emitting it here would double-count it.
fn emit_holes_probe(
    source_face_id: Option<u64>,
    declared_face_index: usize,
    shell_ordinal: u64,
    record: &Option<formal::HoleSliceRecord>,
) {
    let Some(record) = record else { return };
    if record.delegated {
        return;
    }
    let id = source_face_id
        .map(|id| id.to_string())
        .unwrap_or_else(|| "none".to_string());
    let join = |counts: &[usize]| match counts.is_empty() {
        true => "none".to_string(),
        false => counts
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(","),
    };
    let curves = match record.curve_representations.is_empty() {
        true => "none".to_string(),
        false => record.curve_representations.join(","),
    };
    let exit = record.exit.map_or("none", formal::SliceExit::tag);
    let (triangles, cycles, euler) = match record.validity {
        Some(validity) => (
            validity.triangles.to_string(),
            validity.boundary_cycles.to_string(),
            validity.euler_characteristic.to_string(),
        ),
        None => ("none".into(), "none".into(), "none".into()),
    };
    eprintln!(
        "HOLES\tsource_face_id={id}\tdeclared_face_index={declared_face_index}\t\
         shell_ordinal={shell_ordinal}\tstage={}\tcategory={}\texit={exit}\t\
         bounds={}\tinner_bounds={}\tedge_uses_per_bound={}\t\
         polygon_vertices_per_bound={}\tcurves={curves}\ttriangles={triangles}\t\
         boundary_cycles={cycles}\teuler={euler}\tobstruction_bound={}",
        record.stage.tag(),
        record.category.tag(),
        record.bound_count,
        record.inner_bound_count,
        join(&record.edge_uses_per_bound),
        join(&record.polygon_vertices_per_bound),
        record
            .obstruction_bound
            .map_or("none", formal::BoundRole::tag),
    );
}

/// One line per bound of one face: how the developed-curve track read it.
///
/// `crossings` is the number package 6 turns on. Read over the corpus's lost
/// planar faces it says whether the legacy tessellator's
/// `ConstraintInsertionIncomplete` is a real arrangement â€” boundary curves that
/// genuinely cross, needing face extraction and parity selection â€” or an
/// artefact of approximating those curves by chords before asking. The
/// polyline the legacy path asks on is a different object from the analytic
/// curve this track asks on, and only the second answer is about the face.
fn emit_developed_probe(
    source_face_id: Option<u64>,
    declared_face_index: usize,
    record: &formal::DevelopedFaceRecord,
) {
    let id = source_face_id
        .map(|id| id.to_string())
        .unwrap_or_else(|| "none".to_string());
    for (bound_index, bound) in record.bounds.iter().enumerate() {
        let (occurrences, arcs, pieces, crossings) = match bound.survey {
            Some(survey) => (
                survey.occurrences.to_string(),
                survey.arcs.to_string(),
                survey.pieces.to_string(),
                survey.certified_crossings.to_string(),
            ),
            None => (
                "none".to_string(),
                "none".to_string(),
                "none".to_string(),
                "none".to_string(),
            ),
        };
        eprintln!(
            "DEV\tsource_face_id={id}\tdeclared_face_index={declared_face_index}\t\
             bound={bound_index}\tbounds={}\toutcome={}\tcategory={}\t\
             occurrences={occurrences}\tarcs={arcs}\tpieces={pieces}\t\
             crossings={crossings}",
            record.bound_count,
            bound.tag(),
            bound.exit.map_or("resolved", |exit| exit.category().tag()),
        );
    }
}

fn emit_slice_probe(
    source_face_id: Option<u64>,
    declared_face_index: usize,
    shell_ordinal: u64,
    record: &Option<formal::SliceRecord>,
) {
    let id = source_face_id
        .map(|id| id.to_string())
        .unwrap_or_else(|| "none".to_string());
    let Some(record) = record else {
        eprintln!(
            "SLICE\tsource_face_id={id}\tdeclared_face_index={declared_face_index}\t\
             shell_ordinal={shell_ordinal}\tcandidate=0\tstage=not_attempted\t\
             category=unresolved\texit=not_a_planar_rank0_candidate\tbounds=0\t\
             edge_uses=0\touter_bound=none\tcurves=none\tcertificate_route=none\t\
             polygon_vertices=none\ttriangles=none"
        );
        return;
    };
    let curves = match record.curve_representations.is_empty() {
        true => "none".to_string(),
        false => record.curve_representations.join(","),
    };
    let exit = record.exit.map_or("none", formal::SliceExit::tag);
    let route = record
        .certificate_route
        .map_or("none", formal::CertificateRoute::tag);
    let polygon_vertices = record
        .polygon_vertices
        .map_or("none".to_string(), |count| count.to_string());
    let triangles = record.validity.map_or("none".to_string(), |validity| {
        validity.triangles.to_string()
    });
    eprintln!(
        "SLICE\tsource_face_id={id}\tdeclared_face_index={declared_face_index}\t\
         shell_ordinal={shell_ordinal}\tcandidate=1\tstage={}\tcategory={}\texit={exit}\t\
         bounds={}\tedge_uses={}\touter_bound={}\tcurves={curves}\t\
         certificate_route={route}\tpolygon_vertices={polygon_vertices}\t\
         triangles={triangles}",
        record.stage.tag(),
        record.category.tag(),
        record.bound_count,
        record.edge_use_count,
        record.outer_bound.tag(),
    );
}

// Step-0 evidence-audit probe. Remove after `SourceFaceInput` becomes the
// production input, or promote these fields into the permanent census.
//
// One `eprintln!` per face, because face tessellation is parallel and a record
// split across calls interleaves. Consumers must parse order-independently and
// key on `source_face_id` / `declared_face_index`; two runs' probe output are
// not comparable byte-for-byte.
fn emit_evidence_probe(
    input: &std::result::Result<SourceFaceInput, SourceEvidenceError>,
    source_face_id: Option<u64>,
    declared_face_index: usize,
    lattice: &CertifiedLattice,
) {
    let declared_rank = usize::from(lattice.declared_u_period().is_some())
        + usize::from(lattice.declared_v_period().is_some());
    let certified_rank = lattice.certified_rank();
    let id = match source_face_id {
        Some(id) => id.to_string(),
        None => "none".to_string(),
    };
    let record = match input {
        Ok(input) => {
            // The two ratios are the quantitative result. Every other field is
            // a categorical fact about the seam, and there is deliberately no
            // aggregate orientation status: the factors are reported one apiece
            // because collapsing them is what this seam exists to avoid.
            // Per-factor *counts*, never a representative value from the first
            // edge use: that is correct only while every use is constructed
            // identically, and it would silently pick a winner the moment one
            // adapter retains a factor another erases.
            let edge_curve = input.edge_curve_sense_counts();
            let selected_curve = input.selected_curve_direction_counts();
            format!(
                "bounds_total={}\tbounds_regular={}\tbounds_degenerate={}\tedge_uses={}\t\
                 endpoint_ids_complete={}\tendpoints_consistent={}\t\
                 continuous_regular_bounds={}/{}\tcomputable_normalized_signs={}/{}\t\
                 face_use_orientation={}\tface_surface_history={}\t\
                 edge_curve_retained={}\tedge_curve_history_erased={}\tedge_curve_missing={}\t\
                 selected_curve_retained={}\tselected_curve_history_erased={}\t\
                 selected_curve_missing={}\tadapter_error=none",
                input.bounds.len(),
                input.regular_bound_count(),
                input.degenerate_bound_count(),
                input.edge_use_count(),
                u8::from(input.endpoint_ids_complete()),
                u8::from(input.endpoints_consistent()),
                input.continuous_regular_bound_count(),
                input.regular_bound_count(),
                input.computable_normalized_sign_count(),
                input.edge_use_count(),
                input.orientation.face_use_orientation.tag(),
                input.orientation.face_surface_same_sense.tag(),
                edge_curve.retained,
                edge_curve.history_erased,
                edge_curve.missing,
                selected_curve.retained,
                selected_curve.history_erased,
                selected_curve.missing,
            )
        }
        Err(error) => format!(
            "bounds_total=0\tbounds_regular=0\tbounds_degenerate=0\tedge_uses=0\t\
             endpoint_ids_complete=0\tendpoints_consistent=0\tcontinuous_regular_bounds=0/0\t\
             computable_normalized_signs=0/0\tface_use_orientation=missing\t\
             face_surface_history=missing\tedge_curve_retained=0\t\
             edge_curve_history_erased=0\tedge_curve_missing=0\t\
             selected_curve_retained=0\tselected_curve_history_erased=0\t\
             selected_curve_missing=0\tadapter_error={}",
            error.tag(),
        ),
    };
    eprintln!(
        "EVIDENCE\tsource_face_id={id}\tdeclared_face_index={declared_face_index}\t{record}\t\
         declared_rank={declared_rank}\tcertified_rank={certified_rank}"
    );
}

fn shell_create_polygon<S: PreMeshableSurface>(
    surface: &S,
    wires: Vec<Wire<Point3, PolylineCurve>>,
    orientation: bool,
    tol: f64,
    sp: impl SP<S>,
    lattice: &CertifiedLattice,
) -> Face<Point3, PolylineCurve, Option<PolygonMesh>> {
    let preboundary = wires
        .iter()
        .enumerate()
        .map(|(bound_index, wire): (usize, &Wire<_, _>)| {
            // The shell path has no `CompressedEdgeIndex`, so the synthetic
            // identity is minted from the wire's own structure: the bound's
            // position and each edge use's position, with the orientation the
            // edge itself carries (`Edge::oriented_curve` has already applied
            // it to the curve).
            let bound = BoundId(bound_index);
            let wire_iter = wire.iter().enumerate().map(|(use_index, edge)| {
                let curve = edge.oriented_curve();
                SourcePolyline {
                    curve,
                    source: SourceEdgeUse {
                        bound,
                        index: use_index,
                        orientation: edge.orientation(),
                    },
                }
            });
            PolyBoundaryPiece::try_new(surface, wire_iter, &sp, tol, lattice)
        })
        .collect::<std::result::Result<Vec<_>, _>>()
        .ok();
    let polygon: Option<PolygonMesh> = preboundary.map(|preboundary| {
        let boundary = PolyBoundary::new(preboundary, &surface, tol, lattice);
        trimming_tessellation(surface, &boundary, tol, lattice)
    });
    let mut new_face = Face::debug_new(wires, polygon);
    if !orientation {
        new_face.invert();
    }
    new_face
}

/// The parameter-space endpoints of a presented boundary segment.
fn segment_endpoints(a: Point2, b: Point2) -> diagnosis::SegmentEndpoints2 {
    diagnosis::SegmentEndpoints2 {
        a: (a.x, a.y),
        b: (b.x, b.y),
    }
}

/// The parameter-space endpoints of an edge already in the triangulation.
fn spade_endpoints(positions: [SPoint2; 2]) -> diagnosis::SegmentEndpoints2 {
    diagnosis::SegmentEndpoints2 {
        a: (positions[0].x, positions[0].y),
        b: (positions[1].x, positions[1].y),
    }
}

#[derive(Clone, Copy, Debug, derive_more::Deref, derive_more::DerefMut)]
pub(in crate::tessellation) struct SurfacePoint {
    pub(in crate::tessellation) point: Point3,
    #[deref]
    #[deref_mut]
    pub(in crate::tessellation) uv: Point2,
}

impl From<(Point2, Point3)> for SurfacePoint {
    fn from((uv, point): (Point2, Point3)) -> Self {
        Self { point, uv }
    }
}

/// Reconciles a UV step entering or leaving a detected collapsed direction.
///
/// A small derivative only proposes a chart substitution. The candidate UV
/// must also evaluate back to the associated 3D point within model tolerance.
/// An outgoing bridge retains the previous singular 3D point; only its UV
/// representative changes to connect the two chart branches.
fn reconcile_singular_transition<S: ParametricSurface3D>(
    surface: &S,
    previous_uv: Point2,
    previous_point: Point3,
    current_uv: &mut Point2,
    current_point: Point3,
    tolerance: f64,
    output: &mut Vec<SurfacePoint>,
) {
    let ctx = ToleranceCtx::unscaled_legacy();
    let represents =
        |uv: Point2, point: Point3| surface.subs(uv.x, uv.y).distance(point) <= tolerance;
    let cur_uder = surface.uder(current_uv.x, current_uv.y).magnitude();
    let cur_vder = surface.vder(current_uv.x, current_uv.y).magnitude();
    let prev_uder = surface.uder(previous_uv.x, previous_uv.y).magnitude();
    let prev_vder = surface.vder(previous_uv.x, previous_uv.y).magnitude();
    if !ctx.is_small_ratio(previous_uv.x - current_uv.x) && ctx.is_small_len(cur_uder) {
        // BG-TOL-001: param+model
        let candidate = Point2::new(previous_uv.x, current_uv.y);
        if represents(candidate, current_point) {
            current_uv.x = previous_uv.x;
        }
    }
    if !ctx.is_small_ratio(previous_uv.y - current_uv.y) && ctx.is_small_len(cur_vder) {
        // BG-TOL-001: param+model
        let candidate = Point2::new(current_uv.x, previous_uv.y);
        if represents(candidate, current_point) {
            current_uv.y = previous_uv.y;
        }
    }
    if !ctx.is_small_ratio(previous_uv.x - current_uv.x) && ctx.is_small_len(prev_uder) {
        // BG-TOL-001: param+model
        let candidate = Point2::new(current_uv.x, previous_uv.y);
        if represents(candidate, previous_point) {
            output.push((candidate, previous_point).into());
        }
    }
    if !ctx.is_small_ratio(previous_uv.y - current_uv.y) && ctx.is_small_len(prev_vder) {
        // BG-TOL-001: param+model
        let candidate = Point2::new(previous_uv.x, current_uv.y);
        if represents(candidate, previous_point) {
            output.push((candidate, previous_point).into());
        }
    }
}

/// One tessellated source edge use, before the wire is flattened.
///
/// The wrapper exists so `PolyBoundaryPiece::try_new` can record *which* source
/// edge use each presented polyline came from. `create_edge` still emits the
/// clone/inverse of the curve exactly as before â€” the wrapper only adds
/// identity, it never changes curve geometry or ordering.
#[derive(Clone, Debug)]
struct SourcePolyline {
    curve: PolylineCurve,
    source: SourceEdgeUse,
}

/// The source-use provenance of one boundary segment: which source edge uses
/// contributed the presented segment between two consecutive points.
///
/// An empty entry means the segment is synthetic â€” a seam, a closure bridge, a
/// collapsed-periodic bridge, or a reconstructed dense run â€” and carries no
/// source edge use. Source segments carry exactly their own [`SourceEdgeUse`].
/// Nothing in INFRA consumes this beyond recording it; PLANAR-C reads it when a
/// realized constraint edge's provenance is needed.
type SegmentSources = Vec<SourceEdgeUse>;

/// The per-segment provenance of a synthetic part: one empty contributor set
/// per internal segment, because no source edge use describes it.
///
/// `point_count` is the part's point count; a part's segments number one fewer.
fn untagged_sources(point_count: usize) -> Vec<SegmentSources> {
    vec![Vec::new(); point_count.saturating_sub(1)]
}

#[derive(Debug, Default, Clone)]
struct PolyBoundaryPiece(Vec<SurfacePoint>, Vec<SegmentSources>);

impl PolyBoundaryPiece {
    /// A piece whose segments carry no source attribution.
    ///
    /// Test construction only: a piece built from synthetic points has no
    /// source edge uses, so its provenance is empty rather than fabricated.
    fn untagged(points: Vec<SurfacePoint>) -> Self {
        let n = points.len();
        Self(points, vec![Vec::new(); n])
    }

    fn try_new<S: PreMeshableSurface>(
        surface: &S,
        wire: impl Iterator<Item = SourcePolyline>,
        sp: impl SP<S>,
        tol: f64,
        lattice: &CertifiedLattice,
    ) -> std::result::Result<Self, TessellationFailureReason> {
        let ctx = ToleranceCtx::unscaled_legacy();
        // Audit A-ambient: periodicity now arrives as a descriptor whose type
        // distinguishes exact from accessor-only evidence. `declared_period`
        // is what this path read before, so the boundary is introduced with no
        // semantic change. The P2 singular continuation and the P3b cap are
        // theorem paths whose hypothesis is a *genuine* period: they consume
        // the representation-derived generators, so an uncertified accessor
        // value never silently certifies them.
        let (up, vp) = (lattice.declared_u_period(), lattice.declared_v_period());
        let (up_gen, vp_gen) = (lattice.u_generator(), lattice.v_generator());
        let (urange, vrange) = surface.try_range_tuple();
        // How many polylines this bound is assembled from, and how long each
        // is. A bound winding twice is either fed two once-winding pieces --
        // assembly -- or one piece that the lift doubles. This separates them.
        let mut piece_lengths: Vec<usize> = Vec::new();
        // The flattened 3D samples, and the source edge use each one came from.
        // A point inside a straight edge's N=8 expansion belongs to that edge;
        // a lift-refinement midpoint inherits its parent sample's source; a
        // degenerate-periodic reconstruction belongs to nothing.
        let mut bdry3d: Vec<Point3> = Vec::new();
        let mut source_tags: Vec<Option<SourceEdgeUse>> = Vec::new();
        for poly_edge in wire {
            piece_lengths.push(poly_edge.curve.len());
            let source = poly_edge.source;
            if poly_edge.curve.len() == 2 {
                let p0 = poly_edge.curve[0];
                let p1 = poly_edge.curve[1];
                let mut pts = Vec::new();
                const N: usize = 8;
                for i in 0..N {
                    let frac = i as f64 / N as f64;
                    pts.push(p0 + (p1 - p0) * frac);
                }
                bdry3d.extend(pts);
                source_tags.extend(std::iter::repeat_n(Some(source), N));
            } else {
                let n = poly_edge.curve.len().saturating_sub(1);
                bdry3d.extend(poly_edge.curve.into_iter().take(n));
                source_tags.extend(std::iter::repeat_n(Some(source), n));
            }
        }
        // A wire that contributed no points cannot bound a face. This
        // constructor is already fallible, so say so rather than closing the
        // boundary by indexing a vector that is empty. Real exports do produce
        // such wires, and panicking here aborts the whole model.
        if bdry3d.is_empty() {
            // DIAG-002: the boundary-construction refusal witness. The wire
            // carried polylines but every one of them contributed no points.
            diagnosis::record_boundary_refusal(diagnosis::BoundaryWitness {
                bound: None,
                edge_or_piece: None,
                pieces_attempted: piece_lengths.len(),
                pieces_accepted: 0,
                point_counts: piece_lengths.clone(),
                constraints_that_would_be_presented: 0,
                refusal: Some("wire_empty"),
            });
            return Err(TessellationFailureReason::BoundaryWireEmpty);
        }
        bdry3d.push(bdry3d[0]);
        source_tags.push(source_tags[0]);
        let lift_probe = std::env::var_os("TRUCK_PROBE_LIFT").is_some();
        // Apex/pole singular-transition recovery. On by default: it is
        // refinement-only (it fires solely where the legacy path would return
        // `AmbiguousLift`) and validated zero-regression on the NIST and ABC
        // corpora. A face that lifts through the legacy path is byte-identical
        // with this on or off -- it only changes what happens *after* bisection
        // would otherwise give up. Set TRUCK_LIFT_SINGULAR_RECOVERY=0 to
        // disable (emergency withdrawal).
        let lift_singular_recovery =
            !matches!(std::env::var("TRUCK_LIFT_SINGULAR_RECOVERY"), Ok(v) if v == "0");
        // PROJ-001. Under this probe the walk does *not* stop at the first
        // failing point: the ratio of failing points to boundary points is the
        // measurement, and three failures out of 400 is a different diagnosis
        // from three out of five. The face still fails, at the bottom of the
        // loop, with the same reason.
        // The same gate the projection closure uses, so `TRUCK_PROBE_PROJ_DEEP`
        // alone is sufficient. Read through the shared helper rather than the
        // raw variable: this site and `by_search_nearest_parameter` have to
        // agree, and when they did not, the deep probe filled its thread-local
        // for every failing point and the walk returned before reading any of
        // it â€” every witness silently empty.
        let proj_probe = projection_probe_enabled();
        let mut failed_points = 0usize;
        // PROJ-003 Stage A per-face accumulators: boundary points the residual
        // recovery admitted, and the certified residual spread across them.
        let mut recovered_points = 0usize;
        let mut recovered_residual_min = f64::INFINITY;
        let mut recovered_residual_max = 0.0f64;
        // PROJ-003 Stage B per-face accumulators: structural-seed admissions.
        let mut recovered_b_points = 0usize;
        let mut recovered_b_residual_min = f64::INFINITY;
        let mut recovered_b_residual_max = 0.0f64;
        // PROJ-003 Stage C per-face accumulators: domain/periodicity admissions
        // and the class that justified each.
        let mut recovered_c_points = 0usize;
        let mut recovered_c_residual_min = f64::INFINITY;
        let mut recovered_c_residual_max = 0.0f64;
        let mut domain_class_counts: rustc_hash::FxHashMap<diagnosis::DomainRecoveryClass, usize> =
            rustc_hash::FxHashMap::default();
        // PROJ-003 Stage D per-face accumulators: constrained-inverse
        // admissions and the certified residual spread across them.
        let mut recovered_d_points = 0usize;
        let mut recovered_d_residual_min = f64::INFINITY;
        let mut recovered_d_residual_max = 0.0f64;
        // PROJ-002 per-face accumulators.
        let mut deep_probed = 0usize;
        let mut deep_point_cap_hit = false;
        let mut deep_seed_cap_hit = false;
        let mut deep_seeds_offered = 0usize;
        let mut deep_degenerate = 0usize;
        let mut deep_nonconvergent = 0usize;
        let mut deep_best_ratio = f64::INFINITY;
        let mut deep_worst_ratio = 0.0f64;
        let mut deep_point_verdicts: Vec<(diagnosis::PointVerdict, diagnosis::NearestRoute)> =
            Vec::new();
        // PROJ-002/PROJ-003 structured point evidence, aligned with
        // `deep_point_verdicts`, carrying the best candidate and its domain
        // class for the face's witness.
        let mut deep_point_evidence: Vec<diagnosis::ProjectionPointEvidence> = Vec::new();
        let mut failed_links = [0usize; 6];
        let mut seeds_offered = 0usize;
        let mut best_residual = f64::INFINITY;
        let mut first_failed_point: Option<Point3> = None;
        let mut previous: Option<(f64, f64)> = None;
        let mut previous_pt: Option<Point3> = None;
        let mut vec: Vec<SurfacePoint> = Vec::with_capacity(bdry3d.len());
        // The source use each lifted point belongs to, parallel to `vec`. A
        // lift-refinement midpoint inherits the parent sample's source; the
        // degenerate-periodic reconstruction clears every entry. Only the
        // final segment provenance is derived from this; the tags themselves
        // are not retained.
        let mut lifted_tags: Vec<Option<SourceEdgeUse>> = Vec::with_capacity(bdry3d.len());
        // Samples still to lift, most recent last. A step whose periodic
        // representative is ambiguous pushes its own chord midpoint and then
        // revisits itself, so density is spent only where the lift is unsafe
        // rather than across every edge in the model.
        // The flag marks a point this refinement invented rather than one the
        // edge supplied; the tag is the point's source use, inherited by any
        // midpoint it spawns.
        let mut pending: Vec<(Point3, bool, Option<SourceEdgeUse>)> = Vec::new();
        for (point, tag) in bdry3d.iter().zip(&source_tags) {
            pending.clear();
            pending.push((*point, false, *tag));
            let mut refinements = 0usize;
            // The originating real boundary sample for the current bisection
            // chain. Set when the first real sample is found ambiguous; read at
            // exhaustion to admit a singular half-period transition on the
            // original sample (not the synthetic midpoint that exhausted).
            let mut origin: Option<(f64, f64, Point3, Option<SourceEdgeUse>)> = None;
            while let Some((pt, synthetic, tag)) = pending.pop() {
                let projected = sp(surface, pt, previous);
                // A midpoint is only a device for disambiguating the step, and
                // a chord midpoint of a coarse arc does not lie on the surface,
                // so its projection can legitimately fail. Dropping it costs
                // only the refinement; failing the face over it costs the face,
                // which is how this turned 276 surfaceless faces into 391.
                let (mut u, mut v) = match (projected, synthetic) {
                    (Some(uv), _) => uv,
                    (None, true) => continue,
                    (None, false) => {
                        let attempt = last_projection_attempt();
                        // PROJ-003 Stage A: before giving the face up, admit a
                        // residual-certified iterate from a start production
                        // already used. Runs only here â€” after the whole legacy
                        // chain returned `None` â€” so it is refinement-only.
                        if let Some((uv, residual)) =
                            residual_certified_recovery(surface, pt, tol, attempt)
                        {
                            recovered_points += 1;
                            recovered_residual_min = recovered_residual_min.min(residual);
                            recovered_residual_max = recovered_residual_max.max(residual);
                            uv
                        } else if let Some((uv, residual)) =
                            // PROJ-003 Stage B: residual-certified admission of
                            // the structural-seed nearest iterate. Runs only
                            // after Stage A refused the point, so neither a
                            // legacy success nor a Stage-A success can be
                            // altered by it.
                            residual_certified_seed_recovery(
                                surface, pt, tol, attempt,
                            )
                        {
                            recovered_b_points += 1;
                            recovered_b_residual_min = recovered_b_residual_min.min(residual);
                            recovered_b_residual_max = recovered_b_residual_max.max(residual);
                            uv
                        } else if let Some((uv, residual, class)) =
                            // PROJ-003 Stage C: domain/contract recovery. Runs
                            // only after the whole legacy chain, Stage A, and
                            // Stage B all refused the point.
                            residual_certified_domain_recovery(
                                    surface, pt, tol, attempt, lattice,
                                )
                        {
                            recovered_c_points += 1;
                            recovered_c_residual_min = recovered_c_residual_min.min(residual);
                            recovered_c_residual_max = recovered_c_residual_max.max(residual);
                            *domain_class_counts.entry(class).or_insert(0) += 1;
                            uv
                        } else if let Some((uv, residual)) =
                            // PROJ-003 Stage D: the constrained inverse over the
                            // declared range. Runs only after the whole legacy
                            // chain and Stages A-C refused the point, so an
                            // in-range root that the unconstrained Newton
                            // iterate missed is preferred over the extrapolated
                            // branch that left the declared range.
                            constrained_domain_recovery(surface, pt, tol)
                        {
                            recovered_d_points += 1;
                            recovered_d_residual_min = recovered_d_residual_min.min(residual);
                            recovered_d_residual_max = recovered_d_residual_max.max(residual);
                            uv
                        } else if proj_probe {
                            failed_points += 1;
                            failed_links[usize::from(attempt.link).min(5)] += 1;
                            seeds_offered = seeds_offered.max(attempt.seeds);
                            if attempt.best_residual.is_finite() {
                                best_residual = best_residual.min(attempt.best_residual);
                            }
                            first_failed_point.get_or_insert(pt);
                            // PROJ-002. `tol` is known here and not in the closure,
                            // so the classification has to happen at this end of
                            // the thread-local.
                            if attempt.deep {
                                if deep_probed < DEEP_POINT_CAP {
                                    deep_probed += 1;
                                    deep_seeds_offered = deep_seeds_offered.max(attempt.seeds);
                                    deep_seed_cap_hit |= attempt.seed_cap_hit;
                                    deep_degenerate += attempt.degenerate_hits;
                                    deep_nonconvergent += attempt.nonconvergent;
                                    let prod = attempt.prod_best;
                                    let seed = attempt.seed_best;
                                    let best = better_outcome(prod, seed);
                                    let (verdict, route) =
                                        classify_projection_point(prod, seed, tol);
                                    if best.ran() {
                                        let ratio = best.residual / tol;
                                        deep_best_ratio = deep_best_ratio.min(ratio);
                                        deep_worst_ratio = deep_worst_ratio.max(ratio);
                                    }
                                    deep_point_verdicts.push((verdict, route));
                                    // PROJ-003. The structured point evidence:
                                    // the best candidate's UV/residual and, for a
                                    // domain/contract point, its mechanism class.
                                    let domain_class = match verdict {
                                        diagnosis::PointVerdict::DomainOrContractIssue => {
                                            let within =
                                                |o: &NearestOutcome| o.ran() && o.residual <= tol;
                                            let candidate = if within(&prod) { prod } else { seed };
                                            Some(classify_domain_point(
                                                candidate, tol, urange, vrange, lattice,
                                            ))
                                        }
                                        _ => None,
                                    };
                                    deep_point_evidence.push(diagnosis::ProjectionPointEvidence {
                                        verdict,
                                        route,
                                        best_uv: best.uv,
                                        best_residual: best.residual,
                                        domain_class,
                                    });
                                } else {
                                    deep_point_cap_hit = true;
                                }
                            }
                            continue;
                        } else {
                            record_projection_refusal_witness(
                                &attempt,
                                pt,
                                tol,
                                bdry3d.len(),
                                vec.len(),
                                None,
                            );
                            return Err(TessellationFailureReason::BoundaryProjectionFailed);
                        }
                    }
                };
                // A nearest point is not an incidence.
                //
                // `search_nearest_parameter` answers whether or not the query
                // lies on the surface, so a boundary belonging to a different
                // face still yields a plausible parameter, and the uv path
                // built from it is smooth enough to triangulate into a large
                // wrong region. Every symptom chased downstream of this â€” a
                // doubled periodic winding, bounds landing in different period
                // copies, a domain spanning the whole chart â€” was a reading of
                // that path as though it meant something.
                //
                // The contract is that a face's boundary lies on its own
                // surface. Check it where the answer is produced, and refuse
                // the boundary rather than pass a fiction downstream. Measured
                // on shell 160039: 0.027 against a 0.003 tolerance, nine times
                // over, with no nearer solution anywhere in the domain.
                if !synthetic {
                    let residual = surface.subs(u, v).distance(pt);
                    if residual > tol * compatibility_factor() {
                        if std::env::var_os("TRUCK_PROBE_COMPAT").is_some() {
                            // The residual as a multiple of tolerance is the
                            // number the sweep needs: it says where this
                            // rejection would land under any other factor, so
                            // one run yields the whole distribution rather
                            // than one point on it.
                            eprintln!(
                                "COMPAT boundary point off surface: residual={residual:.4e} \
                                 permitted={:.4e} ratio={:.4}",
                                tol * compatibility_factor(),
                                residual / tol,
                            );
                        }
                        // DIAG-002: the projection refusal witness. The
                        // candidate was admitted but lies further from the
                        // surface than the compatibility policy permits.
                        diagnosis::record_compatibility_factor(compatibility_factor());
                        diagnosis::record_projection_refusal(diagnosis::ProjectionRefusalWitness {
                            kind: diagnosis::ProjectionFailureKind::ResidualAboveTolerance,
                            attempted_samples: bdry3d.len(),
                            successful_samples: vec.len(),
                            failed_samples: 1,
                            first_failed_sample: None,
                            min_residual: Some(residual),
                            max_residual: Some(residual),
                            acceptance_tolerance: Some(tol * compatibility_factor()),
                            source_parameter: None,
                            candidate_uv: Some([u, v]),
                            world_point: pt.is_finite().then_some([pt.x, pt.y, pt.z]),
                            periodic_candidate_count: None,
                        });
                        return Err(TessellationFailureReason::BoundaryPointOffSurface);
                    }
                }
                let raw = (u, v);
                if let (Some(up), Some((u0, _))) = (up, previous) {
                    u = get_mindiff(u, u0, up);
                }
                if let (Some(vp), Some((_, v0))) = (vp, previous) {
                    v = get_mindiff(v, v0, vp);
                }
                if lift_probe {
                    // Each sample's raw projection, the periodic representative
                    // chosen for it, and the step that choice implies. Aliasing
                    // shows up as a step near or beyond half a period, or as a
                    // step that closes a loop which should have stayed open.
                    let (du, dv) = match previous {
                        Some((u0, v0)) => (u - u0, v - v0),
                        None => (0.0, 0.0),
                    };
                    let frac = |d: f64, p: Option<f64>| p.map_or(0.0, |p| d / p);
                    eprintln!(
                        "LIFT raw=({:.6},{:.6}) chosen=({u:.6},{v:.6}) \
                         step=({du:+.6},{dv:+.6}) step/period=({:+.4},{:+.4})",
                        raw.0,
                        raw.1,
                        frac(du, up),
                        frac(dv, vp),
                    );
                }
                // Halve the step rather than guess which copy was meant. The
                // projection of the chord midpoint recovers a point the curve
                // actually passes through, so each half advances by less and
                // the nearest copy becomes unambiguous.
                if let (Some((u0, v0)), Some(previous_point)) = (previous, previous_pt) {
                    let ambiguous = |now: f64, before: f64, period: Option<f64>| {
                        period.is_some_and(|period| {
                            f64::abs(now - before) >= AMBIGUOUS_STEP_FRACTION * period
                        })
                    };
                    if ambiguous(u, u0, up) || ambiguous(v, v0, vp) {
                        // Remember the originating real sample for this bisection
                        // chain. Only the first real sample is the boundary point;
                        // every later sample in the chain is a synthetic midpoint.
                        if !synthetic && origin.is_none() {
                            origin = Some((u, v, pt, tag));
                        }
                        if refinements < MAX_LIFT_REFINEMENTS {
                            refinements += 1;
                            pending.push((pt, synthetic, tag));
                            pending.push((previous_point.midpoint(pt), true, tag));
                            continue;
                        }
                        // G2. Bisection is exhausted and the step is still
                        // ambiguous, so no evidence distinguishes the two
                        // candidate period copies. Previously control fell
                        // through here and the ambiguous value was pushed with
                        // nothing recording that it was a guess â€” the face then
                        // proceeded as though the lift were certified. FS
                        // Def. 14 requires a continuous lift; an unresolved
                        // branch is not one.
                        //
                        // Singular-transition recovery (TRUCK_LIFT_SINGULAR_RECOVERY,
                        // default off): exhaustion is the singularity certificate.
                        // A regular surface -- e.g. a cylinder -- resolves a
                        // half-period step via bisection (the chord midpoint
                        // projects to the mid-angle and the step shrinks), so it
                        // never reaches this branch and is never admitted. Only a
                        // rank-deficient transition (cone apex, sphere pole) leaves
                        // the step unshrunk at exhaustion: the chord midpoint is
                        // off-surface and projects to one of the two branches. At
                        // the exact half-period tie the two candidate deck copies
                        // are equidistant and differ by one full period, so the
                        // nearest-copy representative is a continuous half-period
                        // step rather than a full-period fold. Admit the ORIGINAL
                        // real sample (not the synthetic midpoint that exhausted).
                        let half_period_tie = |now: f64, before: f64, period: Option<f64>| {
                            period.is_some_and(|period| {
                                ((f64::abs(now - before) / period) - 0.5).abs()
                                    <= SINGULAR_HALF_PERIOD_TOL
                            })
                        };
                        if lift_singular_recovery {
                            if let Some((ou, ov, opt_pt, origin_tag)) = origin {
                                let tie = (!ambiguous(ou, u0, up) || half_period_tie(ou, u0, up))
                                    && (!ambiguous(ov, v0, vp) || half_period_tie(ov, v0, vp));
                                if tie {
                                    vec.push((Point2::new(ou, ov), opt_pt).into());
                                    lifted_tags.push(origin_tag);
                                    previous = Some((ou, ov));
                                    previous_pt = Some(opt_pt);
                                    // Discard remaining synthetic midpoints; the
                                    // walk continues from the admitted sample.
                                    break;
                                }
                                // P2. The half-period tie is one special case of
                                // a rank-deficient periodic transition. Where the
                                // exhausted step departs from (or enters) a chart
                                // singularity in the *longitude* direction -- the
                                // sphere pole -- bisection can never shrink it,
                                // and the branch is fixed by the leaving edge's
                                // plane rather than by the pole's own
                                // (undefined) longitude. Recover the branch from
                                // the oriented incident geometry; a transition
                                // whose plane does not determine it is a
                                // certified singular ambiguity, and a transition
                                // that is not this mechanism stays unresolved.
                                if let (Some((u0, v0)), Some(previous_point)) =
                                    (previous, previous_pt)
                                {
                                    match singular_transition_branch(
                                        surface,
                                        &sp,
                                        up_gen,
                                        vp_gen,
                                        (u0, v0),
                                        previous_point,
                                        (ou, ov, opt_pt),
                                        &vec,
                                        &bdry3d,
                                    ) {
                                        SingularTransitionOutcome::Continue {
                                            pole_uv,
                                            pole_point,
                                            resume_point,
                                        } => {
                                            // The pole is the sample the step
                                            // left from; it was already
                                            // admitted, so correct its
                                            // bookkeeping longitude rather than
                                            // admitting a duplicate.
                                            let pole_matches = vec.last().is_some_and(|p| {
                                                p.point.distance(pole_point) < 1.0e-3
                                            });
                                            if pole_matches {
                                                if let Some(last) = vec.last_mut() {
                                                    last.uv = pole_uv;
                                                }
                                            } else {
                                                vec.push((pole_uv, pole_point).into());
                                                lifted_tags.push(origin_tag);
                                            }
                                            previous = Some((pole_uv.x, pole_uv.y));
                                            previous_pt = Some(pole_point);
                                            // Resume the ordinary lift from the
                                            // leaving edge's first real sample,
                                            // discarding the synthetic midpoints
                                            // that bisection invented.
                                            let resume_tag = if resume_point.distance(opt_pt)
                                                < 1.0e-3
                                            {
                                                origin_tag
                                            } else {
                                                bdry3d
                                                    .iter()
                                                    .position(|p| p.distance(resume_point) < 1.0e-3)
                                                    .map(|i| source_tags[i + 1])
                                                    .flatten()
                                            };
                                            pending.clear();
                                            pending.push((resume_point, false, resume_tag));
                                            continue;
                                        }
                                        SingularTransitionOutcome::InsufficientEvidence => {
                                            // This mechanism could not select a
                                            // continuation. Negative evidence is
                                            // not a source-level ambiguity
                                            // certificate, so the lift falls
                                            // through to the ordinary
                                            // `AmbiguousLift` (unresolved), never
                                            // to a `RejectedAmbiguous` rejection.
                                            if lift_probe {
                                                eprintln!(
                                                    "P2_INSUFFICIENT_EVIDENCE step left \
                                                     singular without a determined branch"
                                                );
                                            }
                                        }
                                        SingularTransitionOutcome::NotApplicable => {}
                                    }
                                }
                            }
                        }
                        // The two candidate deck copies are [1, 0] (a one-period
                        // advance along `u`) and [0, 1] (one along `v`). An
                        // advance along an axis is structurally legal only when
                        // that axis is periodic, so the candidate set is
                        // filtered by the same `periodic_axes` evidence the
                        // diagnostic reports before any numerical dominance is
                        // attempted. A step that appears to advance one period
                        // along a certified-non-periodic axis is an artefact of
                        // an uncertified declared period, not a real branch.
                        let periodic_u = surface.u_period().is_some();
                        let periodic_v = surface.v_period().is_some();
                        let legal_candidates =
                            legal_deck_shifts(&[[1, 0], [0, 1]], periodic_u, periodic_v);
                        match legal_candidates.len() {
                            // Both axes certified periodic: both deck copies are
                            // structurally legal and the numerical ambiguity
                            // stands. DIAG-002: the lift refusal witness.
                            // Bisection exhausted the step without
                            // disambiguating the two period copies.
                            2 => {
                                diagnosis::record_lift_refusal(diagnosis::LiftWitness {
                                    candidate_lift_count: legal_candidates.len(),
                                    periodic_axes: diagnosis::PeriodicAxes {
                                        u: periodic_u,
                                        v: periodic_v,
                                    },
                                    candidate_deck_shifts: legal_candidates,
                                    closure_seam_evidence: None,
                                    dominance_explanation: Some("bisection_exhausted"),
                                });
                                return Err(TessellationFailureReason::AmbiguousLift);
                            }
                            // Exactly one axis certified periodic: the other
                            // deck copy is structurally impossible, so the
                            // ambiguity is resolved by the certified axes, not
                            // by `get_mindiff` between the candidates. No axis
                            // certified periodic: neither deck copy is legal and
                            // the periodic deck resolver must not activate at
                            // all. Both cases resume the lift from the ordinary
                            // base sample the alternatives were derived from --
                            // the originating real boundary point -- exactly as
                            // the singular tie branch does, discarding the
                            // synthetic midpoint chain that could not shrink.
                            _ => {
                                if let Some((ou, ov, opt_pt, origin_tag)) = origin {
                                    vec.push((Point2::new(ou, ov), opt_pt).into());
                                    lifted_tags.push(origin_tag);
                                    previous = Some((ou, ov));
                                    previous_pt = Some(opt_pt);
                                }
                                break;
                            }
                        }
                    }
                }
                vec.push((Point2::new(u, v), pt).into());
                lifted_tags.push(tag);
                previous = Some((u, v));
                previous_pt = Some(pt);
            }
        }
        // PROJ-003 Stage A probe: one record per face with admitted points, so
        // the reconciliation can track each admission through the downstream
        // walk to its terminal outcome. Deliberately separate from the PROJ
        // record above â€” a face admitted and then lost later needs both lines.
        if (recovered_points > 0 || recovered_b_points > 0 || recovered_c_points > 0)
            && (proj_probe || std::env::var_os("TRUCK_PROBE_PROJ_RECOVERY").is_some())
        {
            let (source_face_id, declared_face_index, _) =
                PROBE_FACE_CONTEXT.with(std::cell::Cell::get);
            // The id prints in the ledger's format (`#110020` / `-`) so the
            // admission record joins cleanly to the census ledger, which keys
            // faces by `source_face_id=#...`.
            let sid = source_face_id
                .map(|id| format!("#{id}"))
                .unwrap_or_else(|| "-".to_string());
            if recovered_points > 0 {
                eprintln!(
                    "PROJ_RECOVER\tsource_face_id={sid}\t\
                     declared_face_index={declared_face_index}\tadmitted={recovered_points}\t\
                     residual_min={:.6e}\tresidual_max={:.6e}\ttol={tol:.6e}",
                    recovered_residual_min, recovered_residual_max,
                );
            }
            if recovered_b_points > 0 {
                eprintln!(
                    "PROJ_RECOVER_B\tsource_face_id={sid}\t\
                     declared_face_index={declared_face_index}\tadmitted={recovered_b_points}\t\
                     residual_min={:.6e}\tresidual_max={:.6e}\ttol={tol:.6e}",
                    recovered_b_residual_min, recovered_b_residual_max,
                );
            }
            if recovered_c_points > 0 {
                let mut classes: Vec<_> = domain_class_counts.iter().collect();
                classes.sort_by_key(|(c, _)| format!("{c:?}"));
                let classes: Vec<String> = classes
                    .into_iter()
                    .map(|(c, n)| format!("{c:?}={n}"))
                    .collect();
                eprintln!(
                    "PROJ_RECOVER_C\tsource_face_id={sid}\t\
                     declared_face_index={declared_face_index}\tadmitted={recovered_c_points}\t\
                     residual_min={:.6e}\tresidual_max={:.6e}\ttol={tol:.6e}\tclasses={}",
                    recovered_c_residual_min,
                    recovered_c_residual_max,
                    classes.join(","),
                );
            }
        }
        if proj_probe && failed_points > 0 {
            if !deep_point_verdicts.is_empty() {
                let verdicts: Vec<_> = deep_point_verdicts.iter().map(|(v, _)| *v).collect();
                let verdict = diagnosis::derive_face_verdict(&verdicts);
                // The route reported is the one belonging to the point that
                // decided the face, not a majority over points that do not
                // matter: the face is lost by its worst point.
                let winning_route = deep_point_verdicts
                    .iter()
                    .find(|(v, _)| *v == verdict)
                    .map(|(_, r)| *r)
                    .unwrap_or(diagnosis::NearestRoute::None);
                diagnosis::record_projection_witness(diagnosis::ProjectionWitness {
                    failed_points,
                    boundary_points: bdry3d.len(),
                    probed_points: deep_probed,
                    point_cap_hit: deep_point_cap_hit,
                    seed_cap_hit: deep_seed_cap_hit,
                    seeds_offered: deep_seeds_offered,
                    tolerance: tol,
                    best_residual: deep_best_ratio.is_finite().then(|| deep_best_ratio * tol),
                    best_residual_over_tol: deep_best_ratio.is_finite().then_some(deep_best_ratio),
                    worst_residual_over_tol: (deep_worst_ratio > 0.0).then_some(deep_worst_ratio),
                    winning_route,
                    point_verdicts: verdicts,
                    point_evidence: std::mem::take(&mut deep_point_evidence),
                    degenerate_hits: deep_degenerate,
                    nonconvergent: deep_nonconvergent,
                    verdict,
                });
            }
            let (source_face_id, declared_face_index, _) =
                PROBE_FACE_CONTEXT.with(std::cell::Cell::get);
            let p = first_failed_point.unwrap_or_else(Point3::origin);
            eprintln!(
                "PROJ\tsource_face_id={source_face_id:?}\t\
                 declared_face_index={declared_face_index}\t\
                 failed_points={failed_points}\tboundary_points={}\t\
                 ratio={:.6}\tlink1={}\tlink2={}\tlink3={}\tlink4={}\tlink5={}\t\
                 seeds={seeds_offered}\tbest_residual={}\ttol={tol:.6e}\t\
                 first_failed_xyz={:.9},{:.9},{:.9}",
                bdry3d.len(),
                failed_points as f64 / bdry3d.len() as f64,
                failed_links[1],
                failed_links[2],
                failed_links[3],
                failed_links[4],
                failed_links[5],
                if best_residual.is_finite() {
                    format!("{best_residual:.6e}")
                } else {
                    "none".into()
                },
                p.x,
                p.y,
                p.z,
            );
            // Probe mode walked the whole boundary and failed a subset; the
            // refusal is partial by construction.
            let p = first_failed_point.unwrap_or_else(Point3::origin);
            diagnosis::record_projection_refusal(diagnosis::ProjectionRefusalWitness {
                kind: diagnosis::ProjectionFailureKind::PartialProjection,
                attempted_samples: bdry3d.len(),
                successful_samples: bdry3d.len().saturating_sub(failed_points),
                failed_samples: failed_points,
                first_failed_sample: None,
                min_residual: best_residual.is_finite().then_some(best_residual),
                max_residual: None,
                acceptance_tolerance: Some(tol),
                source_parameter: None,
                candidate_uv: None,
                world_point: p.is_finite().then_some([p.x, p.y, p.z]),
                periodic_candidate_count: None,
            });
            return Err(TessellationFailureReason::BoundaryProjectionFailed);
        }
        if (bdry3d.len() <= 2 || bdry3d[0].distance(bdry3d[bdry3d.len() - 1]) < 1e-4)
            && piece_lengths.iter().all(|&l| l <= 2)
        {
            if let Some(up) = lattice.declared_u_period() {
                let p0 = bdry3d[0];
                if let Some((u0, v0)) = sp(surface, p0, None) {
                    let mut dense = Vec::new();
                    const STEPS: usize = 16;
                    for i in 0..=STEPS {
                        let frac = i as f64 / STEPS as f64;
                        let u = u0 + frac * up;
                        let pt = surface.subs(u, v0);
                        dense.push((Point2::new(u, v0), pt).into());
                    }
                    vec = dense;
                    // The reconstruction's samples are synthesized from the
                    // surface, not lifted from source trim: no source edge use
                    // describes them, so every provenance tag is cleared.
                    lifted_tags = vec![None; vec.len()];
                }
            } else if let Some(vp) = lattice.declared_v_period() {
                let p0 = bdry3d[0];
                if let Some((u0, v0)) = sp(surface, p0, None) {
                    let mut dense = Vec::new();
                    const STEPS: usize = 16;
                    for i in 0..=STEPS {
                        let frac = i as f64 / STEPS as f64;
                        let v = v0 + frac * vp;
                        let pt = surface.subs(u0, v);
                        dense.push((Point2::new(u0, v), pt).into());
                    }
                    vec = dense;
                    lifted_tags = vec![None; vec.len()];
                }
            }
        }
        let grav = vec.iter().fold(Point2::origin(), |g, p| g + p.uv.to_vec()) / vec.len() as f64;
        let mut quot_u = 0.0;
        let mut quot_v = 0.0;
        if let (Some(up), Some((u0, _))) = (up, urange) {
            quot_u = f64::floor((grav.x - u0) / up);
            vec.iter_mut().for_each(|p| p.x -= quot_u * up);
        }
        if let (Some(vp), Some((v0, _))) = (vp, vrange) {
            quot_v = f64::floor((grav.y - v0) / vp);
            vec.iter_mut().for_each(|p| p.y -= quot_v * vp);
        }
        if lift_probe {
            // Which period copy this bound was placed in, and where it ended
            // up. The shift is chosen from this bound's own centroid alone, and
            // `try_new` runs once per wire, so two bounds of the same face are
            // normalized independently and can be placed in different copies.
            // Comparing these lines across the bounds of one face is the test.
            let (mut u_lo, mut u_hi) = (f64::INFINITY, f64::NEG_INFINITY);
            let (mut v_lo, mut v_hi) = (f64::INFINITY, f64::NEG_INFINITY);
            for p in &vec {
                u_lo = u_lo.min(p.uv.x);
                u_hi = u_hi.max(p.uv.x);
                v_lo = v_lo.min(p.uv.y);
                v_hi = v_hi.max(p.uv.y);
            }
            let winding = |lo: f64, hi: f64, period: Option<f64>| {
                period.map_or(0.0, |period| (hi - lo) / period)
            };
            // Span conflates two different defects, so report the pair that
            // separates them. `k` is the net winding â€” how many periods the
            // boundary ends away from where it started â€” and `V` the total
            // variation, how far it travelled altogether. Circling once gives
            // |k| = 1 with V ~ 1. |k| = 1 with V ~ 2 means it went out and came
            // back, a branch chosen wrongly part way. |k| = 2 with V ~ 2 means
            // it genuinely went round twice, which is a duplicated wire or a
            // seam concatenated in both orientations.
            let (mut travel_u, mut travel_v) = (0.0, 0.0);
            for pair in vec.windows(2) {
                travel_u += f64::abs(pair[1].uv.x - pair[0].uv.x);
                travel_v += f64::abs(pair[1].uv.y - pair[0].uv.y);
            }
            let net = |period: Option<f64>, first: f64, last: f64| {
                period.map_or(0.0, |period| f64::round((last - first) / period))
            };
            let (first, last) = (vec[0].uv, vec[vec.len() - 1].uv);
            // Is the reported period a real period, and is the lift a valid
            // inverse at all? `e_p` and `e_2p` say whether `S` actually repeats
            // after one or two periods; `e_hp` catches a period that is not
            // fundamental. `e_inv` is the reconstruction residual, the distance
            // from each lifted parameter back to the 3D point it came from --
            // small residual with a doubled winding means the chart genuinely
            // takes two parameter circuits per geometric circuit.
            let anchor = vec[0].uv;
            let base = surface.subs(anchor.x, anchor.y);
            let shifted = |dv: f64| surface.subs(anchor.x, anchor.y + dv).distance(base);
            let (e_p, e_2p, e_hp) = match vp {
                Some(period) => (
                    shifted(period),
                    shifted(2.0 * period),
                    shifted(0.5 * period),
                ),
                None => (f64::NAN, f64::NAN, f64::NAN),
            };
            let e_inv = vec.iter().fold(0.0_f64, |worst, s| {
                worst.max(surface.subs(s.uv.x, s.uv.y).distance(s.point))
            });
            // Independent of `sp` entirely: brute-force the true distance from
            // the first boundary point to the surface. This separates the two
            // remaining explanations for a large residual. If the minimum is
            // also far, the point genuinely does not lie on this surface and
            // the edge has been paired with the wrong face. If the minimum is
            // near zero, a valid inverse existed and the projection search
            // failed to find it.
            let target = vec[0].point;
            let anchor_uv = vec[0].uv;
            let axis =
                |range: Option<(f64, f64)>, period: Option<f64>, centre: f64| match (range, period)
                {
                    (Some(r), _) => r,
                    (None, Some(p)) => (centre - p, centre + p),
                    (None, None) => (centre - 1.0, centre + 1.0),
                };
            let (ulo, uhi) = axis(urange, up, anchor_uv.x);
            let (vlo, vhi) = axis(vrange, vp, anchor_uv.y);
            const GRID: usize = 400;
            let mut d_min = f64::INFINITY;
            for i in 0..=GRID {
                let u = ulo + (uhi - ulo) * i as f64 / GRID as f64;
                for j in 0..=GRID {
                    let v = vlo + (vhi - vlo) * j as f64 / GRID as f64;
                    d_min = d_min.min(surface.subs(u, v).distance(target));
                }
            }
            // The structure of the residual, not just its size, says which
            // upstream error produced it. A constant world-space vector is a
            // missing translation. A constant magnitude aligned with the
            // surface normal is a radius or offset error. Neither pattern
            // holding means the entities are unrelated rather than
            // misplaced.
            let residuals: Vec<_> = vec
                .iter()
                .map(|s| s.point - surface.subs(s.uv.x, s.uv.y))
                .collect();
            let mean =
                residuals.iter().fold(Vector3::zero(), |acc, r| acc + r) / residuals.len() as f64;
            let spread = residuals
                .iter()
                .fold(0.0_f64, |worst, r| worst.max((r - mean).magnitude()));
            let (mut mag_lo, mut mag_hi) = (f64::INFINITY, 0.0_f64);
            let mut normal_alignment = 0.0;
            for (r, s) in residuals.iter().zip(&vec) {
                let magnitude = r.magnitude();
                mag_lo = mag_lo.min(magnitude);
                mag_hi = mag_hi.max(magnitude);
                let normal = surface
                    .uder(s.uv.x, s.uv.y)
                    .cross(surface.vder(s.uv.x, s.uv.y));
                if magnitude > 0.0 && normal.magnitude() > 0.0 {
                    normal_alignment += f64::abs(r.dot(normal.normalize()) / magnitude);
                }
            }
            normal_alignment /= residuals.len() as f64;
            eprintln!(
                "PERIOD e_p={e_p:.3e} e_2p={e_2p:.3e} e_hp={e_hp:.3e} e_inv={e_inv:.3e} \
                 d_min={d_min:.3e}"
            );
            eprintln!(
                "RESID |mean|={:.4e} spread={spread:.4e} |r|=[{mag_lo:.4e},{mag_hi:.4e}] \
                 normal_align={normal_alignment:.3}",
                mean.magnitude(),
            );
            // The same incidence certificate the source file satisfies exactly,
            // recomputed on the *converted* geometry. Both frames are recovered
            // from three sampled points, so this needs no knowledge of how the
            // conversion stores them and introduces no grid error. The source
            // gives e_angle = 0, e_axis <= 1e-14, e_radius = 0; whichever of
            // the three is non-zero here names the field the conversion
            // damaged.
            let circle_through = |a: Point3, b: Point3, c: Point3| {
                let (ab, ac) = (b - a, c - a);
                let normal = ab.cross(ac);
                let n2 = normal.magnitude2();
                if n2 < f64::EPSILON {
                    return None;
                }
                let centre = a
                    + (ac.magnitude2() * ab.cross(normal) - ab.magnitude2() * ac.cross(normal))
                        / (2.0 * n2);
                Some((centre, normal.normalize(), centre.distance(a)))
            };
            // Boundary curve: three points spread along it.
            let n = vec.len();
            let boundary_fit = if n >= 3 {
                circle_through(vec[0].point, vec[n / 3].point, vec[2 * n / 3].point)
            } else {
                None
            };
            // Surface cross-section: three points around one periodic axis.
            let anchor = vec[0].uv;
            let cross_fit = match (up, vp) {
                (Some(p), _) => circle_through(
                    surface.subs(anchor.x, anchor.y),
                    surface.subs(anchor.x + p / 3.0, anchor.y),
                    surface.subs(anchor.x + 2.0 * p / 3.0, anchor.y),
                ),
                (_, Some(p)) => circle_through(
                    surface.subs(anchor.x, anchor.y),
                    surface.subs(anchor.x, anchor.y + p / 3.0),
                    surface.subs(anchor.x, anchor.y + 2.0 * p / 3.0),
                ),
                _ => None,
            };
            if let (Some((cc, cn, cr)), Some((sc, sn, sr))) = (boundary_fit, cross_fit) {
                let e_angle = f64::acos(f64::min(1.0, f64::abs(cn.dot(sn)))).to_degrees();
                let offset = cc - sc;
                let e_axis = (offset - sn * offset.dot(sn)).magnitude();
                eprintln!(
                    "INCID e_angle={e_angle:.4}deg e_axis={e_axis:.4e} \
                     e_radius={:.4e} r_curve={cr:.5} r_surf={sr:.5}",
                    f64::abs(cr - sr),
                );
            }
            eprintln!(
                "BOUND pieces={piece_lengths:?} pts={} k=({:+.0},{:+.0}) V=({:.2},{:.2}) \
                 quot=({quot_u:+.0},{quot_v:+.0}) \
                 u=[{u_lo:.4},{u_hi:.4}] v=[{v_lo:.4},{v_hi:.4}] \
                 span/period=({:.3},{:.3})",
                vec.len(),
                net(up, first.x, last.x),
                net(vp, first.y, last.y),
                up.map_or(0.0, |p| travel_u / p),
                vp.map_or(0.0, |p| travel_v / p),
                winding(u_lo, u_hi, up),
                winding(v_lo, v_hi, vp),
            );
        }
        let last = *vec.last().unwrap();
        if !ctx.is_small_ratio(vec[0].uv.distance(last.uv)) {
            // BG-TOL-001: param
            let Point2 { x: u0, y: v0 } = last.uv;
            let u_der = surface.uder(u0, v0).magnitude();
            let v_der = surface.vder(u0, v0).magnitude();
            if ctx.is_small_len(u_der) || ctx.is_small_len(v_der) {
                // BG-TOL-001: model
                vec.push(vec[0]);
                lifted_tags.push(lifted_tags[0]);
            }
        }
        debug_assert_eq!(
            vec.len(),
            lifted_tags.len(),
            "every lifted boundary point must carry a provenance tag",
        );
        // One contributor set per cyclic segment: `sources[k]` names the source
        // edge uses of `points[k] -> points[k + 1]`. A source sample's tag is
        // its own use; a synthetic sample contributes nothing.
        let n = vec.len();
        let sources: Vec<SegmentSources> = (0..n)
            .map(|k| lifted_tags[k].iter().copied().collect())
            .collect();
        Ok(Self(vec, sources))
    }
}

/// FACE-VALIDITY Detector B: measure the constructed boundary pieces and return
/// a degenerate-trim certificate when one is defensible.
///
/// Runs after projection/lift/stitch, on the same pieces `PolyBoundary::new`
/// consumes. The certificate is the world-space numerical rank of the boundary:
/// a boundary whose points all lie on a point or a line (rank < 2, within a
/// floating-point conditioning bound) is rejected; every boundary with two real
/// world directions — however small or thin — survives. The meshing tolerance
/// is not a degeneracy threshold.
fn detect_degenerate_trim<S: PreMeshableSurface>(
    pieces: &[PolyBoundaryPiece],
    surface: &S,
) -> Option<FaceValidityCertificate> {
    let samples: Vec<Vec<validity::TrimSample>> = pieces
        .iter()
        .map(|PolyBoundaryPiece(points, _)| {
            points
                .iter()
                .map(|p| validity::TrimSample {
                    uv: p.uv,
                    world: p.point,
                })
                .collect()
        })
        .collect();
    let metric_scale = {
        // Cap the metric evaluation so a Nurbs heavy shell does not pay one
        // derivative pair per boundary sample. The scale is diagnostic
        // evidence; a bounded sample is enough.
        let mut probe: Vec<validity::TrimSample> = Vec::new();
        for piece in &samples {
            let step = (piece.len() / 24).max(1);
            probe.extend(piece.iter().step_by(step).take(24));
            if probe.len() >= 24 {
                break;
            }
        }
        validity::metric_scale_of(|u, v| surface.uder(u, v), |u, v| surface.vder(u, v), &probe)
    };
    let measurement = validity::measure_trim(&samples, metric_scale)?;
    validity::classify_trim_geometry(pieces.len(), &measurement)
}

/// Certified closed-loop boundary re-lift.
///
/// A topologically closed source edge (`edge.vertices.0 == edge.vertices.1`) is
/// traversed by the whole evaluator loop. When the exporter's closure sliver
/// extends the evaluator range beyond the portion of the curve that lies on the
/// owning surface (the "overshoot"), the boundary lift degenerates: the
/// off-surface samples project back onto the surface outside the trim domain,
/// the piece collapses to a two-point chart, no constraints reach the CDT, and
/// the face is lost as `NoOddParityRegion` even though the source trim is a
/// genuine rank-2 region.
///
/// This module certifies the closed on-surface sub-domain of the source
/// traversal and re-lifts the boundary over it. It is the *only* path that may
/// turn a Detector-B firing face into geometry, and it does so only when the
/// activation theorem holds:
///
/// 1. the wire is a single topologically closed edge;
/// 2. the physical source trim has world rank 2 (measured over the certified
///    interval, never over the overshoot);
/// 3. the initial boundary lift degenerated (Detector B fired);
/// 4. part of the evaluator range leaves the owning surface;
/// 5. a closed on-surface sub-domain can be independently certified (every
///    interior sample projects onto the owning surface at the boundary-lift
///    tolerance, and the interval endpoints coincide — a full loop).
///
/// Every helper returns either a certified result or `None`. There is no
/// heuristic fallback: an ambiguous interval, an un-certifiable closure, a
/// rank ≠ 2 source, or a re-lift that remains degenerate preserves the existing
/// failure. No interval is hard-coded and no evaluator range is clipped merely
/// because it is larger than a unit interval.
mod closed_loop_relift {
    use super::*;

    /// The minimum span of the on-surface run, as a fraction of the traversal
    /// range. Guards against certifying a sliver of on-surface samples as "the
    /// loop": a genuine closure overshoot is a small addition to a full loop,
    /// so the on-surface run dominates the traversal range.
    const MIN_ON_SURFACE_RUN_FRACTION: f64 = 0.5;

    /// The on-surface certification threshold, as a multiple of the chord
    /// tolerance. The boundary-lift tolerance is the chord tolerance; a small
    /// multiple absorbs chord-approximation and export residual without ever
    /// approaching the overshoot scale (the overshoot in the measured
    /// population projects at ~250× tolerance, the on-surface samples at
    /// ~1e-13×).
    const ON_SURFACE_TOL_FACTOR: f64 = 8.0;

    /// The loop-closure bound, as a multiple of the chord tolerance. A full
    /// loop returns to its seam point to floating-point accuracy; a partial arc
    /// ends a chord-length away. The floor keeps the bound from collapsing on a
    /// tiny-tolerance model.
    const CLOSURE_TOL_FACTOR: f64 = 0.01;

    /// Whether the certified closed-loop re-lift route is active.
    ///
    /// Nested under the master formal-recovery gate like every other route, so
    /// `TRUCK_FORMAL_RECOVERY=0` closes it and an explicit route disable
    /// (`TRUCK_FORMAL_RECOVERY_CLOSED_LOOP=0`) narrows it independently.
    pub(super) fn recovery_gate() -> bool {
        diagnosis::formal_recovery_enabled()
            && diagnosis::recovery_route_enabled("TRUCK_FORMAL_RECOVERY_CLOSED_LOOP")
    }

    /// Whether the re-lift prints its activation/refusal evidence.
    fn relift_probe() -> bool {
        std::env::var_os("TRUCK_PROBE_RELIFT").is_some()
    }

    /// The traversal range a topologically closed edge is sampled over.
    ///
    /// Replicates `tessellate_edge`'s range assembly exactly (evaluator range,
    /// the degenerate-range period fallback, then the partition-of-unity
    /// extension over the declared range tuple), so the interval certified here
    /// is the same domain the failing polyline was built over.
    fn closed_edge_traversal_range<C: PolylineableCurve>(curve: &C) -> (f64, f64) {
        let (lo, hi) = curve.evaluation_range();
        let mut range = (lo, hi);
        if (range.1 - range.0).abs() < 1e-4 {
            if let Some(period) = curve.period() {
                if period > 1e-4 {
                    range = (range.0, range.0 + period);
                }
            }
        }
        if let Some((rt0, rt1)) = curve.try_range_tuple() {
            if rt0 < range.0 - 1.0e-12 && curve.basis_is_partition_of_unity(rt0) {
                range.0 = rt0;
            }
            if rt1 > range.1 + 1.0e-12 && curve.basis_is_partition_of_unity(rt1) {
                range.1 = rt1;
            }
        }
        range
    }

    /// Whether a sample of the source curve lies on the owning surface at the
    /// boundary-lift tolerance.
    fn on_surface<C: PolylineableCurve, S: PreMeshableSurface, SP: super::SP<S>>(
        curve: &C,
        t: f64,
        surface: &S,
        sp: &SP,
        tol: f64,
    ) -> bool {
        let p = curve.subs(t);
        match sp(surface, p, None) {
            Some((u, v)) => surface.subs(u, v).distance(p) <= ON_SURFACE_TOL_FACTOR * tol,
            None => false,
        }
    }

    /// Certify the closed on-surface sub-domain of a source traversal.
    ///
    /// Returns `Some((a, b))` exactly when `(a, b) ⊆ (lo, hi)`, every sample of
    /// the curve over `(a, b)` lies on `surface` at the boundary-lift
    /// tolerance, the run is a substantial part of the traversal (an overshoot,
    /// not a sliver), and the interval endpoints coincide — the interval is a
    /// closed full loop. Returns `None` otherwise; there is no fallback result.
    fn certify_closed_on_surface_interval<C, S, SP>(
        curve: &C,
        range: (f64, f64),
        surface: &S,
        sp: &SP,
        tol: f64,
    ) -> Option<(f64, f64)>
    where
        C: PolylineableCurve,
        S: PreMeshableSurface,
        SP: super::SP<S>,
    {
        let (lo, hi) = range;
        if !lo.is_finite() || !hi.is_finite() || !(hi > lo) {
            return None;
        }
        const N: usize = 512;
        let mut first_on: Option<usize> = None;
        let mut last_on: Option<usize> = None;
        for i in 0..=N {
            let t = lo + (hi - lo) * i as f64 / N as f64;
            if on_surface(curve, t, surface, sp, tol) {
                if first_on.is_none() {
                    first_on = Some(i);
                }
                last_on = Some(i);
            }
        }
        let (i0, i1) = (first_on?, last_on?);
        // The on-surface run must dominate the traversal: a real overshoot is a
        // small addition to a full loop, so the certified loop covers most of
        // the range. A run below this is not a loop-plus-overshoot shape.
        let run_fraction = (i1 - i0) as f64 / N as f64;
        if run_fraction < MIN_ON_SURFACE_RUN_FRACTION {
            return None;
        }
        // Refine the low boundary between samples i0-1 (off) and i0 (on).
        let t_low = if i0 == 0 {
            lo
        } else {
            let mut a = lo + (hi - lo) * (i0 - 1) as f64 / N as f64;
            let mut b = lo + (hi - lo) * i0 as f64 / N as f64;
            for _ in 0..64 {
                let m = (a + b) / 2.0;
                if on_surface(curve, m, surface, sp, tol) {
                    b = m;
                } else {
                    a = m;
                }
            }
            b
        };
        let t_high = if i1 == N {
            hi
        } else {
            let mut a = lo + (hi - lo) * i1 as f64 / N as f64;
            let mut b = lo + (hi - lo) * (i1 + 1) as f64 / N as f64;
            for _ in 0..64 {
                let m = (a + b) / 2.0;
                if on_surface(curve, m, surface, sp, tol) {
                    a = m;
                } else {
                    b = m;
                }
            }
            a
        };
        if !(t_high > t_low) {
            return None;
        }
        // Loop closure: the interval endpoints represent the same seam point.
        // A full loop returns to the same point to floating-point accuracy; a
        // partial arc ends a chord-length away and is refused (this is the
        // guard that keeps a complementary-arc traversal from being treated as
        // a full loop).
        let pa = curve.subs(t_low);
        let pb = curve.subs(t_high);
        let scale = pa
            .to_vec()
            .magnitude()
            .max(pb.to_vec().magnitude())
            .max(1.0);
        let closure_tol = (tol * CLOSURE_TOL_FACTOR).max(validity::fp_rank_tolerance(scale));
        if pa.distance(pb) > closure_tol {
            return None;
        }
        Some((t_low, t_high))
    }

    /// The world rank of the source trim over an interval, sampled densely.
    ///
    /// Measured from the source curve over the *certified* interval — the real
    /// on-surface loop — never over the overshoot. A rank-2 source is the
    /// activation condition that separates a recoverable closed loop from a
    /// certified line/slit.
    fn source_world_rank<C: PolylineableCurve>(curve: &C, interval: (f64, f64)) -> u8 {
        const SAMPLES: usize = 24;
        let mut pts = Vec::with_capacity(SAMPLES + 1);
        for i in 0..=SAMPLES {
            let t = interval.0 + (interval.1 - interval.0) * i as f64 / SAMPLES as f64;
            pts.push(curve.subs(t));
        }
        validity::world_rank_of(&pts).0
    }

    /// Re-lift a single wire whose initial piece degenerated.
    ///
    /// The wire must be exactly one topologically closed edge whose source
    /// curve has a certifiable closed on-surface sub-domain and a rank-2
    /// physical trim. The source curve is re-sampled over the certified
    /// interval and presented as the wire's polyline, so the ordinary
    /// `PolyBoundaryPiece::try_new` lift runs on the on-surface loop only.
    fn relift_wire<C, S, SP>(
        shell: &CompressedShell<Point3, C, S>,
        surface: &S,
        bound_index: usize,
        wire: &[CompressedEdgeIndex],
        edges: &[EstablishedEdge],
        sp: &SP,
        tol: f64,
    ) -> Option<Vec<SourcePolyline>>
    where
        C: PolylineableCurve,
        S: PreMeshableSurface,
        SP: super::SP<S>,
    {
        // Activation condition 1: exactly one topologically closed edge.
        if wire.len() != 1 {
            return None;
        }
        let edge_use = &wire[0];
        let shell_edge = shell.edges.get(edge_use.index)?;
        if shell_edge.vertices.0 != shell_edge.vertices.1 {
            return None;
        }
        if !matches!(edges.get(edge_use.index), Some(EstablishedEdge::Mesh(_))) {
            return None;
        }
        let curve = &shell_edge.curve;
        let range = closed_edge_traversal_range(curve);
        let probe = relift_probe();
        let (source_face_id, ..) = PROBE_FACE_CONTEXT.with(std::cell::Cell::get);
        if probe {
            eprintln!(
                "RELIFT_WIRE source_face_id={:?} edge={} closed=1 range=({:.6},{:.6})",
                source_face_id, edge_use.index, range.0, range.1
            );
        }
        // Activation conditions 4 + 5: certify the closed on-surface interval.
        let interval = certify_closed_on_surface_interval(curve, range, surface, sp, tol);
        let Some(interval) = interval else {
            if probe {
                eprintln!(
                    "RELIFT_WIRE_REFUSE source_face_id={:?} reason=no_certified_interval",
                    source_face_id
                );
            }
            return None;
        };
        if probe {
            eprintln!(
                "RELIFT_WIRE_CERT source_face_id={:?} interval=({:.6},{:.6})",
                source_face_id, interval.0, interval.1
            );
        }
        // Activation condition 2: the physical source trim is rank 2.
        let rank = source_world_rank(curve, interval);
        if probe {
            eprintln!(
                "RELIFT_WIRE_RANK source_face_id={:?} rank={rank}",
                source_face_id
            );
        }
        if rank < 2 {
            return None;
        }
        // Re-sample the curve over the certified interval, in the wire's
        // orientation, and re-present the wire.
        //
        // The chord sampler (`from_curve`) is allowed to under-sample a loop
        // whose world extent is only a few times the chord tolerance: the
        // certified population sits there (the loop is small, which is part of
        // why the boundary lift was at all fragile), and a coarse polyline
        // misses the loop's two world directions entirely. The re-lift is a
        // recovery, not a re-mesh, so the certified interval is re-sampled at
        // a minimum density that captures the loop shape; every point is an
        // exact source-curve evaluation over the certified on-surface interval,
        // so no geometry is invented. A source whose certified interval is
        // large enough that the chord sampler is already dense keeps the
        // tolerance-driven sampling.
        const MIN_RELIFT_POINTS: usize = 24;
        let mut poly = PolylineCurve::from_curve(curve, interval, tol);
        if poly.len() < MIN_RELIFT_POINTS {
            let pts: Vec<Point3> = (0..=MIN_RELIFT_POINTS)
                .map(|i| {
                    curve.subs(
                        interval.0
                            + (interval.1 - interval.0) * i as f64 / MIN_RELIFT_POINTS as f64,
                    )
                })
                .collect();
            poly = PolylineCurve::from(pts);
        }
        if probe {
            let pts: Vec<String> = poly
                .iter()
                .map(|p| format!("({:.6},{:.6},{:.6})", p.x, p.y, p.z))
                .collect();
            eprintln!(
                "RELIFT_POLY source_face_id={:?} len={} pts={:?}",
                source_face_id,
                poly.len(),
                pts
            );
        }
        if poly.len() < 3 {
            return None;
        }
        let curve = match edge_use.orientation {
            true => poly,
            false => poly.inverse(),
        };
        let source = SourceEdgeUse {
            bound: BoundId(bound_index),
            index: 0,
            orientation: edge_use.orientation,
        };
        Some(vec![SourcePolyline { curve, source }])
    }

    /// Re-lift a face's boundaries over certified on-surface intervals.
    ///
    /// Only the degenerate-lift case qualifies (Detector B fired). Every wire
    /// must re-lift to a nondegenerate piece or the whole face keeps its
    /// existing failure; a partial re-lift would invent a boundary the source
    /// does not certify.
    pub(super) fn try_relift_face<C, S, SP>(
        shell: &CompressedShell<Point3, C, S>,
        surface: &S,
        boundaries: &[Vec<CompressedEdgeIndex>],
        edges: &[EstablishedEdge],
        sp: &SP,
        tol: f64,
        lattice: &CertifiedLattice,
        pieces: &[PolyBoundaryPiece],
    ) -> Option<Vec<PolyBoundaryPiece>>
    where
        C: PolylineableCurve,
        S: PreMeshableSurface,
        SP: super::SP<S>,
    {
        let probe = relift_probe();
        let (source_face_id, declared_face_index, _) =
            PROBE_FACE_CONTEXT.with(std::cell::Cell::get);
        if probe {
            eprintln!(
                "RELIFT_ENTER source_face_id={:?} pieces={} bounds={} piece_lens={:?}",
                source_face_id,
                pieces.len(),
                boundaries.len(),
                pieces.iter().map(|p| p.0.len()).collect::<Vec<_>>(),
            );
        }
        if pieces.len() != boundaries.len() {
            if probe {
                eprintln!("RELIFT_REFUSE reason=piece_bound_mismatch");
            }
            return None;
        }
        // Activation condition 3: the initial lift degenerated. The caller
        // reached here only because Detector B fired on these pieces (their
        // world rank is < 2), which is the degeneracy signal; no additional
        // point-count threshold is needed — a slit can carry many collinear
        // samples and still be a certified degeneracy, and the per-wire guards
        // below (single closed edge, certified interval, rank-2 source) are
        // what separate the recoverable closed loop from the rest.
        let mut rebuilt = Vec::with_capacity(boundaries.len());
        for (bound_index, wire) in boundaries.iter().enumerate() {
            let Some(relifted) = relift_wire(shell, surface, bound_index, wire, edges, sp, tol)
            else {
                if probe {
                    eprintln!(
                        "RELIFT_REFUSE source_face_id={:?} bound={bound_index} reason=wire_not_reliftable",
                        source_face_id
                    );
                }
                return None;
            };
            let piece =
                PolyBoundaryPiece::try_new(surface, relifted.into_iter(), sp, tol, lattice).ok()?;
            if piece.0.len() <= 2 {
                if probe {
                    eprintln!(
                        "RELIFT_REFUSE source_face_id={:?} bound={bound_index} reason=relift_still_degenerate",
                        source_face_id
                    );
                }
                return None;
            }
            if probe {
                let uvs: Vec<String> = piece
                    .0
                    .iter()
                    .map(|p| format!("({:.6},{:.6})", p.uv.x, p.uv.y))
                    .collect();
                eprintln!(
                    "RELIFT_PIECE source_face_id={:?} bound={bound_index} pts={} uv={:?}",
                    source_face_id,
                    piece.0.len(),
                    uvs
                );
            }
            rebuilt.push(piece);
        }
        if probe {
            eprintln!(
                "RELIFT_OK source_face_id={:?} declared_face_index={declared_face_index}",
                source_face_id
            );
        }
        Some(rebuilt)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use truck_geometry::prelude::Plane;

        fn plane() -> Plane {
            Plane::new(
                Point3::origin(),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
            )
        }

        fn square_loop() -> PolylineCurve {
            PolylineCurve::from(vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(1.0, 1.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
                Point3::new(0.0, 0.0, 0.0),
            ])
        }

        fn open_arc() -> PolylineCurve {
            PolylineCurve::from(vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(1.0, 1.0, 0.0),
            ])
        }

        fn line() -> PolylineCurve {
            PolylineCurve::from(vec![Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0)])
        }

        fn tol() -> f64 {
            1.0e-3
        }

        // T3 guard: a rank-1 slit (a straight out-and-back line) is never a
        // recoverable material source; a closed square loop is.
        #[test]
        fn source_world_rank_separates_slit_from_loop() {
            assert_eq!(source_world_rank(&line(), (0.0, 1.0)), 1);
            assert_eq!(source_world_rank(&square_loop(), (0.0, 4.0)), 2);
        }

        // T1: a closed on-surface loop over its whole traversal range is
        // certified — every sample lies on the plane and the endpoints close.
        #[test]
        fn certify_accepts_closed_full_loop() {
            let sp = by_search_nearest_parameter;
            let interval = certify_closed_on_surface_interval(
                &square_loop(),
                (0.0, 4.0),
                &plane(),
                &sp,
                tol(),
            );
            let (a, b) = interval.expect("a closed on-surface square is certifiable");
            assert!(b > a);
            assert!((a - 0.0).abs() < 1.0e-6);
            assert!((b - 4.0).abs() < 1.0e-6);
        }

        // T6 / T4: a complementary or partial arc does not close — its
        // endpoints are a chord-length apart — so no closed on-surface interval
        // is certified and the existing refusal is preserved.
        #[test]
        fn certify_refuses_open_arc() {
            let sp = by_search_nearest_parameter;
            let interval =
                certify_closed_on_surface_interval(&open_arc(), (0.0, 2.0), &plane(), &sp, tol());
            assert!(interval.is_none(), "an open arc must not close as a loop");
        }

        // Detector B: an out-and-back slit boundary is a certified
        // LineLikeTrim rejection, never a material source.
        #[test]
        fn detector_b_rejects_out_and_back_slit() {
            let corner = |x: f64, y: f64| -> SurfacePoint {
                (Point2::new(x, y), Point3::new(x, y, 0.0)).into()
            };
            let slit = vec![corner(0.0, 0.0), corner(1.0, 0.0), corner(0.0, 0.0)];
            let certificate =
                detect_degenerate_trim(&[PolyBoundaryPiece::untagged(slit)], &plane());
            let certificate = certificate.expect("an out-and-back slit is degenerate");
            assert_eq!(
                certificate.reason,
                DegenerateFaceReason::LineLikeTrim,
                "a rank-1 slit is a line-like trim"
            );
        }

        // Detector B negative: a finite rank-2 region is NOT rejected — the
        // world-rank certificate must never touch a real (even thin) region.
        #[test]
        fn detector_b_accepts_finite_rank_two_region() {
            let corner = |x: f64, y: f64| -> SurfacePoint {
                (Point2::new(x, y), Point3::new(x, y, 0.0)).into()
            };
            let rect = vec![
                corner(0.0, 0.0),
                corner(1.0, 0.0),
                corner(1.0, 1.0),
                corner(0.0, 1.0),
                corner(0.0, 0.0),
            ];
            let certificate =
                detect_degenerate_trim(&[PolyBoundaryPiece::untagged(rect)], &plane());
            assert!(certificate.is_none(), "a real rank-2 region must survive");
        }
    }
}

fn get_mindiff(u: f64, u0: f64, up: f64) -> f64 {
    // The nearest periodic copy outright, rather than the nearest among five.
    // The old search covered only two periods either side, so a boundary that
    // wrapped further was silently pulled back; rounding has no such bound and
    // is cheaper.
    u + f64::round((u0 - u) / up) * up
}

/// Filter periodic deck-candidate shifts by the certified periodicity of the
/// two parameter axes.
///
/// A deck shift `(ku, kv)` advances `ku` full periods along `u` and `kv` along
/// `v`. An advance along an axis is legal only when that axis is periodic, so a
/// shift is retained iff `ku != 0 ⇒ periodic_u` and `kv != 0 ⇒ periodic_v`.
///
/// This is the structural filter applied to a bisection-exhausted lift step
/// before any numerical dominance ([`get_mindiff`]) is attempted: an advance
/// along a certified-non-periodic axis is impossible however close the
/// numerical copies are. `periodic_u`/`periodic_v` are the same accessor facts
/// the diagnostic records as `periodic_axes`.
fn legal_deck_shifts(candidates: &[[i64; 2]], periodic_u: bool, periodic_v: bool) -> Vec<[i64; 2]> {
    candidates
        .iter()
        .copied()
        .filter(|&[ku, kv]| (ku == 0 || periodic_u) && (kv == 0 || periodic_v))
        .collect()
}

#[cfg(test)]
mod legal_deck_shift_tests {
    use super::legal_deck_shifts;

    /// U-periodic only: the `v` deck copy is structurally illegal, so
    /// `[1, 0], [0, 1]` reduces to `[1, 0]` and no numerical dominance
    /// (`get_mindiff`) is run between the candidates.
    #[test]
    fn u_periodic_only_retains_u_copy() {
        assert_eq!(
            legal_deck_shifts(&[[1, 0], [0, 1]], true, false),
            vec![[1, 0]]
        );
    }

    /// V-periodic only: `[1, 0], [0, 1]` reduces to `[0, 1]`.
    #[test]
    fn v_periodic_only_retains_v_copy() {
        assert_eq!(
            legal_deck_shifts(&[[1, 0], [0, 1]], false, true),
            vec![[0, 1]]
        );
    }

    /// Neither axis certified periodic: no deck candidate is legal, so the
    /// periodic deck resolver must not activate and the ordinary base lift is
    /// preserved.
    #[test]
    fn nonperiodic_retains_no_deck_candidate() {
        assert_eq!(
            legal_deck_shifts(&[[1, 0], [0, 1]], false, false),
            Vec::<[i64; 2]>::new()
        );
    }

    /// Both axes certified periodic: both deck copies may remain, and the
    /// numerical dominance decision (`get_mindiff`; `AmbiguousLift` when it
    /// cannot prove dominance) is the only arbiter left.
    #[test]
    fn doubly_periodic_retains_both_candidates() {
        assert_eq!(
            legal_deck_shifts(&[[1, 0], [0, 1]], true, true),
            vec![[1, 0], [0, 1]]
        );
    }
}

/// How far a boundary point may sit from its own surface, as a multiple of the
/// chord tolerance, before the pairing is refused.
///
/// A face's boundary is required to lie on that face's surface; this is the
/// slack allowed for the chord approximation and for imperfect exports, not a
/// licence to trim a surface with a curve belonging to something else.
///
/// **Off by default, deliberately.** The violation this detects is real â€”
/// swept on `00009190`, the rejected points sit at a median of 191x tolerance
/// and a maximum of 617x, and loosening the factor twentyfold removes only 62
/// of 315 rejections, so this is a population and not a threshold. But
/// rejecting them repairs nothing visible: the blob shells are byte-identical
/// with the gate on and off, while the gate costs 292 faces and 21,131
/// triangles, a tenth of the model. Deleting real geometry to fix nothing is
/// the wrong default. Set `TRUCK_COMPAT_FACTOR=5` to measure the population;
/// turn it on for real only once something downstream can use the refusal.
const COMPATIBILITY_FACTOR: f64 = f64::INFINITY;

/// The factor in force, overridable by `TRUCK_COMPAT_FACTOR`.
///
/// Sweeping the factor is what established that the gate names a real
/// population rather than a threshold, and a rebuild per sample would have made
/// that a five-build afternoon. Read once â€” this sits in the per-boundary-point
/// loop, and an env lookup there would be a measurable cost charged to every
/// model.
fn compatibility_factor() -> f64 {
    static FACTOR: std::sync::OnceLock<f64> = std::sync::OnceLock::new();
    *FACTOR.get_or_init(|| {
        std::env::var("TRUCK_COMPAT_FACTOR")
            .ok()
            .and_then(|raw| raw.trim().parse::<f64>().ok())
            .filter(|factor| *factor > 0.0 && !factor.is_nan())
            .unwrap_or(COMPATIBILITY_FACTOR)
    })
}

/// How far a step may advance, as a fraction of the period, before the periodic
/// representative it implies is treated as ambiguous.
///
/// [`get_mindiff`] takes the copy nearest the previous parameter, which is the
/// right answer only while the true step is under half a period. At exactly
/// half, the two candidates are equidistant and the tie is broken arbitrarily â€”
/// measured advancing `-0.5` of a period where the curve went `+0.5`, which
/// folds a full turn onto itself and makes a period-wrapping boundary look like
/// a closed loop. The margin below `0.5` keeps numerical noise clear of the tie.
const AMBIGUOUS_STEP_FRACTION: f64 = 0.45;

/// How many times a single step may be halved before refinement gives up.
const MAX_LIFT_REFINEMENTS: usize = 8;

/// Half-width of the band around the exact half-period tie at which the
/// singular-lift recovery (`TRUCK_LIFT_SINGULAR_RECOVERY`) admits an exhausted
/// step, as a fraction of the period.
///
/// The recovery fires only when bisection has exhausted *and* every ambiguous
/// periodic step lies within this band of exactly half a period. There the two
/// candidate deck copies are equidistant from the previous sample and differ by
/// exactly one full period -- they are deck-equivalent representations of the
/// same physical point, not two distinguishable branches -- so the nearest-copy
/// representative already chosen by `get_mindiff` is a continuous half-period
/// step rather than a full-period fold. The band admits only genuine ties
/// (cone apex / sphere pole crossings between generatrices half a period apart);
/// a step at, say, `0.6` of a period is a real branch ambiguity and remains
/// `AmbiguousLift`.
const SINGULAR_HALF_PERIOD_TOL: f64 = 0.02;

/// The outcome of a singular periodic-transition branch analysis (P2).
///
/// At an exhausted ambiguous lift step whose start sample is a rank-deficient
/// point of the *periodic* direction (a sphere pole, or the collapsed row of
/// any revolution), bisection cannot separate the two candidate period copies:
/// the chord midpoint is off the surface and projects onto one branch or the
/// other, so refinement never shrinks the step. The branch is not a numerical
/// fact but a source one -- a great circle through a pole has constant
/// longitude equal to the azimuth of its plane -- so the *leaving edge* fixes
/// the outgoing longitude regardless of the pole's own (undefined) coordinate.
/// Recovering the branch from the oriented incident geometry, never from the
/// pole's own parameter, is the L1/L2 gate:
///
/// - a uniquely determined continuation renders (recover and continue);
/// - a singular transition whose incident geometry does not determine the
///   branch leaves the lift unresolved -- the mechanism lacks positive
///   evidence of a continuation, which is not a source-level ambiguity
///   certificate (an ambiguity rejection would require constructing two
///   distinct source-consistent continuations);
/// - a transition that is not this mechanism leaves the lift unresolved.
enum SingularTransitionOutcome {
    /// The branch is uniquely determined by the leaving edge's plane. Resume
    /// the lift from `resume_point` after assigning the pole the corrected
    /// bookkeeping coordinate `pole_uv`.
    Continue {
        pole_uv: Point2,
        pole_point: Point3,
        resume_point: Point3,
    },
    /// The mechanism could not determine a continuation (the leaving edge's
    /// own first sample is a pole, or no usable leaving sample exists). This
    /// is negative evidence about this mechanism only: it proves "no
    /// continuation could be selected here", never "the source admits two
    /// distinct continuations". The lift stays unresolved.
    InsufficientEvidence,
    /// Not a recoverable singular periodic transition; leave the lift
    /// unresolved.
    NotApplicable,
}

/// Which parameter axis a chart's longitude lives on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LongitudeAxis {
    U,
    V,
}

impl LongitudeAxis {
    /// The periodic coordinate of a UV point on this axis.
    fn coordinate(self, uv: Point2) -> f64 {
        match self {
            Self::U => uv.x,
            Self::V => uv.y,
        }
    }
}

/// The P2 singular periodic-transition analysis.
///
/// `previous` is the accepted sample the ambiguous step departs from and
/// `origin` the real boundary sample that first triggered the ambiguity (the
/// leaving edge's first non-pole sample in the observed cases). `accepted` is
/// the lift built so far and `bdry3d` the flat boundary run, both used to
/// recover the *incoming* longitude: the branch is selected as the period copy
/// of the outgoing longitude nearest the already-lifted incoming longitude,
/// never from the pole's own (arbitrary) coordinate.
///
/// The gate never certifies a source-level ambiguity. A continuation is either
/// determined by the oriented incident geometry (`Continue`), or it is not;
/// in the latter case the lift stays unresolved (`InsufficientEvidence` /
/// `NotApplicable`). "This mechanism cannot select a continuation" is not a
/// certificate that the STEP source admits two distinct continuations, and it
/// must never become a `RejectedAmbiguous` rejection.
fn singular_transition_branch<S>(
    surface: &S,
    sp: &impl SP<S>,
    up: Option<f64>,
    vp: Option<f64>,
    previous: (f64, f64),
    previous_point: Point3,
    origin: (f64, f64, Point3),
    accepted: &[SurfacePoint],
    bdry3d: &[Point3],
) -> SingularTransitionOutcome
where
    S: PreMeshableSurface,
{
    let ctx = ToleranceCtx::unscaled_legacy();
    // The pole is a chart point where the *periodic* axis's partial collapses
    // (at a sphere pole the longitude is undefined, so moving in it moves
    // nothing). Detecting the collapse on the periodic axis is what separates
    // this from a regular half-period step: a cylinder never reaches here, a
    // cone apex and sphere pole do.
    let collapsed_axis = |u: f64, v: f64| -> Option<LongitudeAxis> {
        if vp.is_some() && ctx.is_small_len(surface.vder(u, v).magnitude()) {
            // BG-TOL-001: model
            Some(LongitudeAxis::V)
        } else if up.is_some() && ctx.is_small_len(surface.uder(u, v).magnitude()) {
            // BG-TOL-001: model
            Some(LongitudeAxis::U)
        } else {
            None
        }
    };
    // The accepted sample's longitude is the already-lifted incoming one.
    let incoming_longitude = |axis: LongitudeAxis| -> Option<f64> {
        accepted
            .iter()
            .rev()
            .find(|p| !collapsed_axis(p.uv.x, p.uv.y).is_some())
            .map(|p| axis.coordinate(p.uv))
    };
    // Resolve the branch from the leaving edge's own plane: `get_mindiff`
    // picks the copy of the outgoing longitude nearest the incoming one.
    let branch = |axis: LongitudeAxis, outgoing: f64, incoming: f64| match axis {
        LongitudeAxis::V => get_mindiff(outgoing, incoming, vp.unwrap()),
        LongitudeAxis::U => get_mindiff(outgoing, incoming, up.unwrap()),
    };

    let (ou, ov, opt_pt) = origin;
    match collapsed_axis(previous.0, previous.1) {
        // Case A: the step departs from the pole; the origin is the leaving
        // edge's first non-pole sample.
        Some(axis) => {
            if collapsed_axis(ou, ov).is_some() {
                // The leaving edge's own first sample is also a pole, so its
                // longitude is undefined and the plane does not determine a
                // continuation here. That is negative evidence about this
                // mechanism, not a proof that the source admits two distinct
                // continuations -- leave the lift unresolved.
                return SingularTransitionOutcome::InsufficientEvidence;
            }
            let outgoing = axis.coordinate(Point2::new(ou, ov));
            let Some(incoming) = incoming_longitude(axis) else {
                return SingularTransitionOutcome::NotApplicable;
            };
            let pole_uv = match axis {
                LongitudeAxis::V => Point2::new(previous.0, branch(axis, outgoing, incoming)),
                LongitudeAxis::U => Point2::new(branch(axis, outgoing, incoming), previous.1),
            };
            SingularTransitionOutcome::Continue {
                pole_uv,
                pole_point: previous_point,
                resume_point: opt_pt,
            }
        }
        // Case B: the step enters the pole (the origin is the pole). The
        // leaving edge must be found ahead on the flat boundary run.
        None => match collapsed_axis(origin.0, origin.1) {
            Some(axis) => {
                let pole_idx = bdry3d.iter().position(|p| p.distance(opt_pt) < 1.0e-3);
                let leaving = pole_idx.and_then(|i| bdry3d.get(i + 1).copied());
                let Some(leaving_pt) = leaving else {
                    // No usable leaving sample ahead; insufficient evidence for
                    // this mechanism to select a continuation. Unresolved, not
                    // certified ambiguous.
                    return SingularTransitionOutcome::InsufficientEvidence;
                };
                // The leaving edge's first non-pole sample; the closed-form
                // inverse fixes its longitude.
                let (lu, lv) = match sp(surface, leaving_pt, None) {
                    Some(uv) => uv,
                    None => return SingularTransitionOutcome::NotApplicable,
                };
                if collapsed_axis(lu, lv).is_some() {
                    // The leaving sample is itself a pole; its longitude is
                    // undefined, so no branch is determined. Insufficient
                    // evidence for this mechanism, not a source ambiguity.
                    return SingularTransitionOutcome::InsufficientEvidence;
                }
                let outgoing = axis.coordinate(Point2::new(lu, lv));
                // The step's start is the already-lifted incoming longitude.
                let incoming = axis.coordinate(Point2::new(previous.0, previous.1));
                let pole_uv = match axis {
                    LongitudeAxis::V => Point2::new(origin.0, branch(axis, outgoing, incoming)),
                    LongitudeAxis::U => Point2::new(branch(axis, outgoing, incoming), origin.1),
                };
                SingularTransitionOutcome::Continue {
                    pole_uv,
                    pole_point: opt_pt,
                    resume_point: leaving_pt,
                }
            }
            None => SingularTransitionOutcome::NotApplicable,
        },
    }
}

/// How many independent ray directions [`PolyBoundary::include`] may try before
/// reporting that containment is undecidable at a point.
///
/// Cost is confined to the abort path â€” `find_map` stops at the first ray that
/// decides. Measured on ABC `00009190`: 18% of the aborts left by a single cast
/// are resolved by alternate directions, and every one of those resolved to
/// *outside*, changing no output. Whatever still aborts after eight directions
/// is classified by [`PolyBoundary::on_boundary`], a direct predicate, rather
/// than by inferring a location from the rays having failed.
const INCLUSION_RAY_ATTEMPTS: u32 = 8;

/// Where a point lies relative to a trimmed domain.
///
/// `Boundary` and `Indeterminate` are deliberately distinct. The first is a
/// positive result from a direct point-on-segment test; the second means no
/// method established a location. Collapsing them would reintroduce, one level
/// up, the conflation this type exists to remove.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PointLocation {
    /// Strictly inside the material domain.
    Inside,
    /// Strictly outside it.
    Outside,
    /// On a boundary segment within `TOLERANCE`, by direct test.
    Boundary,
    /// Not established by any available method.
    Indeterminate,
}

/// Result of boundary loop closure evaluation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BoundaryClosure {
    /// Endpoints are within Euclidean UV tolerance.
    EuclideanClosed,
    /// Endpoints close modulo parameter period.
    PeriodicClosed {
        /// Integer winding displacement along [u, v].
        displacement: [i64; 2],
    },
    /// Boundary loop is un-closed (open).
    Open,
}

/// Evaluates periodic displacement winding modulo period if residual <= tolerance.
fn periodic_displacement(start: f64, end: f64, period: f64, tolerance: f64) -> Option<i64> {
    if period <= 1e-6 {
        return None;
    }
    let winding = ((end - start) / period).round() as i64;
    let residual = (end - start) - winding as f64 * period;
    if residual.abs() <= tolerance {
        Some(winding)
    } else {
        None
    }
}

/// Where one boundary segment came from.
///
/// **G6, phase 2A.** `PolyBoundary::new` stitches synthesised closure and seam
/// segments into the *same* point vectors as source-derived trim, after which
/// nothing distinguishes them â€” so `insert_to` tagged every segment
/// `PhysicalBoundary`, fabricated geometry included. The fix is not to
/// reclassify afterwards but to record the origin where the segment is created,
/// which is the only place it is known.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum SegmentOrigin {
    /// Carries source boundary evidence: a lifted source edge use.
    Source,
    /// Synthesised to close an open piece against the working extent. No source
    /// entity describes it (`DOM-ARTIFICIAL-CLOSURE-001`).
    SyntheticClosure,
    /// Synthesised to bridge a collapsed periodic pair â€” a seam across a
    /// degenerate direction rather than a trim boundary.
    Seam,
    /// **P3b.** Synthesised to close the planar chart of a periodic spherical
    /// cap: the meridian runs and the pole line that turn a |k|=1 latitude
    /// walk (whose UV image is a 1D line) into a contractible planar cell.
    ///
    /// Explicitly artificial: it carries no source edge use, and it is counted
    /// separately from `Seam`/`SyntheticClosure` so the census can say how much
    /// of the boundary is invented chart topology. It completes the *single*
    /// material boundary of the cap (the source latitude loop plus this
    /// closure form one closed toggling loop); it never creates a second,
    /// independent toggling region. `DOM-CHART-CLOSURE-001`.
    ChartClosure,
}

impl SegmentOrigin {
    /// The constraint role this origin justifies.
    ///
    /// Deliberately **behaviour-preserving**: the synthetic roles still toggle
    /// material parity, exactly as they did while masquerading as
    /// `PhysicalBoundary`. This makes the populations nameable and countable;
    /// deciding what a synthesised segment *should* do to material state is a
    /// separate change that must be measured on its own.
    ///
    /// A P3b chart closure toggles as well: the cap's source latitude walk is
    /// an *open* path in the chart (its periodic wrap is where the meridian
    /// seam replaces the deck), so a non-toggling closure would leave the
    /// toggling subgraph with two odd-degree endpoints and the parity flood
    /// would contradict itself. The closure completes the single boundary; it
    /// is artificial (no source identity) and never a separate parity region.
    fn role(self) -> ConstraintRole {
        match self {
            Self::Source => ConstraintRole::PhysicalBoundary,
            Self::SyntheticClosure | Self::Seam | Self::ChartClosure => {
                ConstraintRole::UnresolvedSyntheticClosure
            }
        }
    }
}

/// A closed boundary loop in parameter space, carrying each segment's origin
/// and source-use provenance.
///
/// `origins[i]` and `source_uses[i]` describe the segment from `points[i]` to
/// `points[i + 1]`, cyclically, so the three vectors have equal length by
/// construction. `source_uses[i]` is the contributor set of that segment:
/// empty for a synthetic seam/closure, the segment's own [`SourceEdgeUse`]
/// otherwise.
#[derive(Debug, Default, Clone)]
struct BoundaryLoop {
    points: Vec<SurfacePoint>,
    origins: Vec<SegmentOrigin>,
    source_uses: Vec<SegmentSources>,
}

impl BoundaryLoop {
    /// Build from parts that are known to chain end-to-start, closing back on
    /// the first part's start. Every join is a shared endpoint, so no segment
    /// is invented; this is the stitching case, where each run was constructed
    /// to begin where the previous one ended.
    ///
    /// Each part's `source_uses` runs parallel to its internal segments
    /// (`source_uses[i]` labels `part[i] -> part[i + 1]`), one entry fewer
    /// than the part's point count.
    fn chained(
        parts: impl IntoIterator<Item = (Vec<SurfacePoint>, Vec<SegmentSources>, SegmentOrigin)>,
    ) -> Self {
        let mut path = BoundaryPath::default();
        for (part, sources, origin) in parts {
            path.append(part, sources, origin, PartJoin::SharedEndpoint);
        }
        path.close(PartJoin::SharedEndpoint)
    }

    /// Cut the cyclic loop open at its wrap segment, yielding a path whose
    /// origins are retained.
    ///
    /// The wrap's own origin is dropped because that segment ceases to exist;
    /// every other segment keeps the label it was created with. This is what
    /// lets a loop be re-joined to something else without its provenance being
    /// rebuilt from scratch â€” taking `.points` and relabelling would, for
    /// instance, silently turn a periodic walk's deck seam back into `Source`.
    fn into_path_cutting_wrap(self) -> BoundaryPath {
        let Self {
            points,
            mut origins,
            mut source_uses,
        } = self;
        origins.pop();
        source_uses.pop();
        BoundaryPath {
            points,
            origins,
            source_uses,
        }
    }

    /// Checked constructor. The equal-length relation is the type's whole
    /// invariant, so it is enforced rather than documented.
    fn new(
        points: Vec<SurfacePoint>,
        origins: Vec<SegmentOrigin>,
        source_uses: Vec<SegmentSources>,
    ) -> Self {
        assert_eq!(
            points.len(),
            origins.len(),
            "every boundary segment must carry exactly one origin",
        );
        assert_eq!(
            points.len(),
            source_uses.len(),
            "every boundary segment must carry exactly one provenance entry",
        );
        Self {
            points,
            origins,
            source_uses,
        }
    }

    /// A loop whose duplicate endpoint has already been removed, so every
    /// cyclic segment â€” including the wrap from the last point back to the
    /// first â€” is source-derived.
    ///
    /// `source_uses` is the piece's provenance with the degenerate wrap entry
    /// dropped: it still has one entry per remaining point.
    fn euclidean_source_loop(points: Vec<SurfacePoint>, source_uses: Vec<SegmentSources>) -> Self {
        let origins = vec![SegmentOrigin::Source; points.len()];
        Self::new(points, origins, source_uses)
    }

    /// A lifted walk that closes only *modulo the lattice*: its last point is
    /// `first + LÎ´`, a distinct parameter point, and is retained.
    ///
    /// The wrap segment is therefore **not** another source trim segment â€” it
    /// is the deck closure, and labelling it `Source` would feed the material
    /// solve a boundary no source entity describes. Properly this should not be
    /// a geometric segment at all but a deck identification; until the quotient
    /// stage exists to hold that relation, it is marked `Seam`, which keeps the
    /// current toggling behaviour while naming what it is. Its provenance is
    /// cleared with the same intent: the closure has no source edge use.
    fn periodic_source_walk(
        points: Vec<SurfacePoint>,
        mut source_uses: Vec<SegmentSources>,
    ) -> Self {
        let mut origins = vec![SegmentOrigin::Source; points.len()];
        if let Some(wrap) = origins.last_mut() {
            *wrap = SegmentOrigin::Seam;
        }
        if let Some(wrap) = source_uses.last_mut() {
            wrap.clear();
        }
        Self::new(points, origins, source_uses)
    }
}

/// How one boundary part meets the next.
///
/// **Stated by the caller, never inferred.** An earlier version decided this by
/// testing `tail.uv.distance(next[0].uv) < TOLERANCE`, which is wrong twice
/// over. A UV epsilon cannot distinguish a retained shared endpoint from a deck
/// identification, a singular attachment, or an unresolved relation â€” they are
/// different facts that can present with the same coordinates â€” and its
/// tolerance has no fixed physical meaning across parameterisations. The
/// stitching site already knows which case it is building; the type now makes
/// it say so.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PartJoin {
    /// The next part begins at the point the previous one ended on. The
    /// duplicate is dropped and **no segment is created**.
    SharedEndpoint,
    /// The parts do not meet. A segment is created between them, carrying the
    /// given origin because neither part supplied it.
    Bridge(SegmentOrigin),
}

/// An *open* chain of boundary segments, before it is closed into a loop.
///
/// `origins[i]` and `source_uses[i]` label the segment `points[i] -> points[i + 1]`,
/// so there is exactly one fewer origin than point. Keeping the open case in
/// its own type is what makes the closing segment an explicit decision rather
/// than an accident of indexing.
#[derive(Debug, Default, Clone)]
struct BoundaryPath {
    points: Vec<SurfacePoint>,
    origins: Vec<SegmentOrigin>,
    source_uses: Vec<SegmentSources>,
}

impl BoundaryPath {
    fn start(
        points: Vec<SurfacePoint>,
        source_uses: Vec<SegmentSources>,
        origin: SegmentOrigin,
    ) -> Self {
        let origins = vec![origin; points.len().saturating_sub(1)];
        Self {
            points,
            origins,
            source_uses,
        }
    }

    /// Append a part, saying explicitly how it meets what is already here.
    ///
    /// A shared endpoint drops the duplicate point and creates no segment. A
    /// bridge keeps **both** endpoints and inserts one labelled segment between
    /// them â€” which is the case the previous implementation got wrong: it
    /// dropped every part's final point unconditionally, so a bridge silently
    /// replaced `a1 -> a2 -> b0` with the shortcut `a1 -> b0`, deleting a real
    /// source segment precisely when the distinction mattered most.
    ///
    /// `part_sources[i]` labels `part[i] -> part[i + 1]`, one entry fewer than
    /// the part's point count. On a shared endpoint the part's first entry
    /// becomes the join segment into the retained head; on a bridge the join is
    /// synthetic and carries no source.
    fn append(
        &mut self,
        mut part: Vec<SurfacePoint>,
        part_sources: Vec<SegmentSources>,
        origin: SegmentOrigin,
        join: PartJoin,
    ) {
        if part.is_empty() {
            return;
        }
        if self.points.is_empty() {
            *self = Self::start(part, part_sources, origin);
            return;
        }
        match join {
            PartJoin::SharedEndpoint => {
                part.remove(0);
            }
            PartJoin::Bridge(bridge) => {
                self.origins.push(bridge);
                self.source_uses.push(Vec::new());
            }
        }
        // After a shared-endpoint head drop, `part_sources` still names the
        // part's original segments; its first entry is the join into the
        // retained head, the rest are the part's own.
        let (join_source, own_sources): (SegmentSources, &[SegmentSources]) = match join {
            PartJoin::SharedEndpoint => (
                part_sources.first().cloned().unwrap_or_default(),
                &part_sources[1.min(part_sources.len())..],
            ),
            PartJoin::Bridge(_) => (Vec::new(), &part_sources[..]),
        };
        self.origins
            .extend(std::iter::repeat_n(origin, part.len().saturating_sub(1)));
        self.source_uses.extend(own_sources.iter().cloned());
        if !part.is_empty() {
            // The segment from the current tail into the first retained point
            // of `part` belongs to `part` when they shared an endpoint, and was
            // already labelled as the bridge otherwise.
            if matches!(join, PartJoin::SharedEndpoint) {
                self.origins.push(origin);
                self.source_uses.push(join_source);
            }
            self.points.extend(part);
        }
    }

    /// Append another path, preserving its per-segment origins.
    fn append_path(&mut self, other: BoundaryPath, join: PartJoin) {
        if other.points.is_empty() {
            return;
        }
        if self.points.is_empty() {
            *self = other;
            return;
        }
        let BoundaryPath {
            mut points,
            origins,
            source_uses,
        } = other;
        match join {
            PartJoin::SharedEndpoint => {
                points.remove(0);
            }
            PartJoin::Bridge(bridge) => {
                self.origins.push(bridge);
                self.source_uses.push(Vec::new());
            }
        }
        self.origins.extend(origins);
        self.source_uses.extend(source_uses);
        self.points.extend(points);
    }

    /// Reverse traversal.
    ///
    /// Sound on an open path precisely *because* it is open: with `origins[i]`
    /// labelling `points[i] -> points[i + 1]`, reversing both vectors maps
    /// segment `i` to old segment `n - 2 - i`, the same segment travelled
    /// backwards. The cyclic case is **not** this â€” reversing a loop's two
    /// vectors directly is off by one, because the wrap segment does not move
    /// with the rest. Cutting a loop into a path first removes the need to
    /// reason about where the cut went. Provenance reverses with the same
    /// segment transformation.
    fn reverse(&mut self) {
        self.points.reverse();
        self.origins.reverse();
        self.source_uses.reverse();
    }

    /// Close the path into a cyclic loop, saying what the closing segment is.
    ///
    /// `SharedEndpoint` means the path already returns to its start, so the
    /// duplicate final point is dropped and the existing last segment becomes
    /// the wrap. `Bridge` keeps every point and adds one labelled wrap segment,
    /// whose provenance is empty because neither part supplied it.
    fn close(mut self, join: PartJoin) -> BoundaryLoop {
        match join {
            PartJoin::SharedEndpoint => {
                self.points.pop();
            }
            PartJoin::Bridge(bridge) => {
                self.origins.push(bridge);
                self.source_uses.push(Vec::new());
            }
        }
        BoundaryLoop::new(self.points, self.origins, self.source_uses)
    }
}

impl BoundaryLoop {
    fn len(&self) -> usize {
        self.points.len()
    }
}

#[derive(Debug, Default, Clone)]
struct PolyBoundary(Vec<BoundaryLoop>);

/// Normalize an open boundary piece so it starts at the point crossing `u1`
/// (on axis `compidx`), carrying its per-segment provenance along.
///
/// **PLANAR-A invariant.** The rotation reorders segments, so `sources` must
/// follow its segments or the provenance silently attaches to the wrong
/// geometry. `sources[k]` labels `points[k] -> points[k + 1]`, one entry fewer
/// than the point count, so the transformation is derived on the *segments*:
///
/// - `i < n - 1`: the crossing is interior. The rotated chain's segments are a
///   cyclic shift of the open chain's, and every rotated segment starts at the
///   same point as its original, so `sources.rotate_left(i)` keeps each entry
///   on the segment it describes.
/// - `i == n - 1`: the curve's own last point is the crossing. There is no
///   interior run to rotate; the chain re-introduces the wrap segment back
///   onto its head (`points[n - 1] -> points[0]`). That segment's provenance
///   was dropped when the piece was classified open — as an open chain the
///   wrap did not exist — and it is synthetic, so an explicit empty entry is
///   inserted rather than shortening the vector. The point count grows by one
///   (the duplicate terminal) exactly as the geometry always did; the sources
///   now grow with it.
fn normalize_range(
    curve: &mut Vec<SurfacePoint>,
    sources: &mut Vec<SegmentSources>,
    compidx: usize,
    (u0, u1): (f64, f64),
) {
    let p = curve[0];
    let q = curve[curve.len() - 1];
    let tmp = f64::min(p[compidx], q[compidx]) + TOLERANCE;
    let del = f64::floor((tmp - u0) / (u1 - u0)) * (u1 - u0);
    curve.iter_mut().for_each(|p| p[compidx] -= del);
    let Some(i) = curve
        .iter()
        .position(|p| (curve[0][compidx] - u1) * (p[compidx] - u1) < 0.0)
    else {
        return;
    };
    if i == curve.len() - 1 {
        sources.insert(0, Vec::new());
    } else {
        sources.rotate_left(i);
    }
    let mut curve1 = curve.split_off(i + 1);
    curve1.pop();
    curve1.insert(0, curve[i]);
    match curve[0][compidx] < curve[curve.len() - 1][compidx] {
        true => curve1.iter_mut(),
        false => curve.iter_mut(),
    }
    .for_each(|p| p[compidx] -= u1 - u0);
    curve1.append(curve);
    *curve = curve1;
}

/// Twice the signed area of a closed `uv` loop, by the shoelace formula.
///
/// Diagnostic only. Its *sign* must not be used to decide what a face
/// contains: it negates under an orientation-reversing reparameterization,
/// which no observer of the solid can detect, so any predicate built on it
/// classifies the same face differently depending on how its surface happens
/// to be parameterized. Relative sign between loops is invariant; absolute
/// sign is not.
#[allow(dead_code)]
/// Record one boundary piece's deck evidence into the DIAG-001 sink.
///
/// The winding sign uses the *same* degeneracy threshold the two-closed-loop
/// branch tests against, so a piece recorded as sign `0` is exactly a piece
/// that branch would admit. Reporting a different threshold here would make
/// the record describe a decision nothing takes.
fn record_piece_deck(
    piece_index: usize,
    points: &[SurfacePoint],
    closure: ObservedClosure,
    ku: i64,
    kv: i64,
) {
    let area = signed_area(points);
    let uv = |p: &SurfacePoint| (p.uv.x, p.uv.y);
    let (start_uv, end_uv) = match (points.first(), points.last()) {
        (Some(first), Some(last)) => (uv(first), uv(last)),
        _ => ((f64::NAN, f64::NAN), (f64::NAN, f64::NAN)),
    };
    diagnosis::record_boundary_piece(diagnosis::BoundaryPieceDeck {
        piece_index,
        closure,
        ku,
        kv,
        winding_sign: match area {
            a if a.abs() < DEGENERATE_LOOP_AREA => 0,
            a if a > 0.0 => 1,
            _ => -1,
        },
        signed_area: area,
        representative: start_uv,
        start_uv,
        end_uv,
        point_count: points.len(),
    });
}

/// Record what the two-closed-loop join did to the deck sum.
///
/// `Σδᵢ = Δ_walk`, and `Δ_walk = 0` for a contractible regular boundary. The
/// join realises `δ₀ − δ₁` when it reverses loop 1 and `δ₀ + δ₁` when it
/// traverses loop 1 forward; the deck equation decides which direction closes.
/// `forward_would_close` is the discriminator: it is true exactly when the
/// reversal is what broke the equation and traversing forward would satisfy it.
fn record_two_loop_join(
    loop0_displacement: [i64; 2],
    loop1_displacement: [i64; 2],
    mean_translate: [i64; 2],
    loop1_reversed: bool,
    loop0_path: &BoundaryPath,
    loop1_path: &BoundaryPath,
) {
    let uv = |p: &SurfacePoint| (p.uv.x, p.uv.y);
    let nan = (f64::NAN, f64::NAN);
    let ends = |path: &BoundaryPath| match (path.points.first(), path.points.last()) {
        (Some(first), Some(last)) => (uv(first), uv(last)),
        _ => (nan, nan),
    };
    let (loop0_start, loop0_end) = ends(loop0_path);
    let (loop1_start, loop1_end) = ends(loop1_path);
    // The sum the chosen traversal realises, and the sum the other one would.
    let sign = if loop1_reversed { -1 } else { 1 };
    let deck_sum_u = loop0_displacement[0] + sign * loop1_displacement[0];
    let deck_sum_v = loop0_displacement[1] + sign * loop1_displacement[1];
    let other_u = loop0_displacement[0] - sign * loop1_displacement[0];
    let other_v = loop0_displacement[1] - sign * loop1_displacement[1];
    let deck_consistent = deck_sum_u == 0 && deck_sum_v == 0;
    diagnosis::record_two_loop_join(diagnosis::TwoLoopJoinRecord {
        loop0_displacement,
        loop1_displacement,
        loop1_reversed,
        mean_translate,
        deck_sum_u,
        deck_sum_v,
        deck_consistent,
        forward_would_close: !deck_consistent && other_u == 0 && other_v == 0,
        // The join appends loop 1's reversed path after loop 0's, then closes
        // back to loop 0's start: two bridges, and these are their endpoints.
        bridge0: [loop0_end, loop1_start],
        bridge1: [loop1_end, loop0_start],
    });
}

/// The number of distinct source edge uses a closed loop's segments reference.
///
/// A loop built from a single full-circle edge references exactly one source
/// edge use; a loop that contains a genuine source seam (arcs + seam lines)
/// references several. The count separates the two correspondence classes:
/// a single-source full-period loop's parameterization origin is arbitrary,
/// while a multi-source loop carries a source-established seam.
fn distinct_source_edge_uses(loop_: &BoundaryLoop) -> usize {
    let mut ids = rustc_hash::FxHashSet::default();
    for seg in &loop_.source_uses {
        for use_ in seg {
            ids.insert((use_.bound.0, use_.index, use_.orientation));
        }
    }
    ids.len()
}

/// Cyclically re-index loop 1 so its seam reference (the phase of its first
/// sample, modulo the period) matches loop 0's.
///
/// The two bounds of a full 360° band are each a single self-closing circle
/// edge whose parameterization origin is arbitrary: no source edge, vertex, or
/// seam connects a specific point of one circle to a specific point of the
/// other. The source therefore establishes no correspondence, and the correct
/// one is geometric — points on the two loops that share the same periodic
/// surface coordinate (v mod period) lie on a common generator (ruling) of the
/// cylinder, and the straight segment between them lies exactly on the surface.
/// Aligning the loops' seam references makes the two synthetic seam bridges
/// these generator lines.
///
/// The re-index is a cyclic rotation of the sample array plus a periodic
/// re-lift of the wrapped tail (each moved sample's uv is shifted by the
/// loop's own displacement, which maps to the same 3D point on the periodic
/// surface), so every realized boundary point is preserved exactly.
///
/// Gated to the single-source population: a multi-source loop's seam is
/// source-established and must not be moved. Returns true when loop 1 was
/// re-indexed.
fn align_two_loop_phase(
    loop0: &BoundaryLoop,
    loop1: &mut BoundaryLoop,
    loop1_displacement: [i64; 2],
    lattice: &CertifiedLattice,
) -> bool {
    if distinct_source_edge_uses(loop0) != 1 || distinct_source_edge_uses(loop1) != 1 {
        return false;
    }
    let (axis, period) = if loop1_displacement[1] != 0 {
        (1usize, lattice.declared_v_period())
    } else if loop1_displacement[0] != 0 {
        (0usize, lattice.declared_u_period())
    } else {
        return false;
    };
    let Some(period) = period else {
        return false;
    };
    let winding = if axis == 1 {
        loop1_displacement[1]
    } else {
        loop1_displacement[0]
    };
    if winding.unsigned_abs() != 1 {
        return false;
    }
    let n = loop1.points.len();
    if n < 2 {
        return false;
    }
    let distinct = n - 1;
    let phase = |uv: Point2| if axis == 1 { uv.y } else { uv.x };
    let target = phase(loop0.points[0].uv).rem_euclid(period);
    let circular = |a: f64, b: f64| {
        let d = (a - b).abs();
        d.min(period - d)
    };
    let mut best = 0usize;
    let mut best_d = f64::INFINITY;
    for i in 0..distinct {
        let d = circular(phase(loop1.points[i].uv).rem_euclid(period), target);
        if d < best_d {
            best_d = d;
            best = i;
        }
    }
    if best == 0 {
        return false;
    }
    let disp_axis = phase(loop1.points[n - 1].uv) - phase(loop1.points[0].uv);
    let mut points = Vec::with_capacity(n);
    let mut origins = Vec::with_capacity(n);
    let mut source_uses = Vec::with_capacity(n);
    for i in 0..distinct {
        let src = (best + i) % distinct;
        let mut p = loop1.points[src];
        if src < best {
            if axis == 1 {
                p.uv.y += disp_axis;
            } else {
                p.uv.x += disp_axis;
            }
        }
        points.push(p);
        origins.push(loop1.origins[src]);
        source_uses.push(loop1.source_uses[src].clone());
    }
    let mut wrap = points[0];
    if axis == 1 {
        wrap.uv.y += disp_axis;
    } else {
        wrap.uv.x += disp_axis;
    }
    points.push(wrap);
    origins.push(SegmentOrigin::Seam);
    source_uses.push(Vec::new());
    *loop1 = BoundaryLoop::new(points, origins, source_uses);
    true
}

/// The parameter-space area below which a closed loop is treated as degenerate
/// â€” a band's boundary circle, which encloses no area in the chart because it
/// *is* a chart-crossing line.
///
/// Named because the two-closed-loop branch and the DIAG-001 record must test
/// the same number: a record taken against a different threshold would describe
/// a population no code path acts on.
const DEGENERATE_LOOP_AREA: f64 = 1e-4;

fn signed_area(curve: &[SurfacePoint]) -> f64 {
    curve
        .iter()
        .circular_tuple_windows()
        .fold(0.0, |sum, (p, q)| sum + (q.x + p.x) * (q.y - p.y))
}

/// The working parameter rectangle of a trimmed face, derived from the face's
/// own bounds.
///
/// **`PAR-RANGE-INHERITANCE-001`.** A surface's declared `parameter_range` is a
/// property of the primitive it was constructed from, not of any face that
/// references it. `Line::parameter_range` is `[0, 1]` unconditionally and
/// `RevolutedCurve` inherits it, so a cone built as a revolved line declares
/// `[0, 1] x [0, 2pi)` â€” one unit of generatrix starting at the STEP reference
/// radius, chosen by the primitive and unrelated to the face. Stitching an open
/// boundary piece against the edge of that rectangle fabricates trim geometry
/// no source entity describes (`DOM-ARTIFICIAL-CLOSURE-001`), and when the
/// piece already lies on the edge the enclosed area is zero
/// (`DOM-ZERO-AREA-001`).
///
/// Measured: extending that range by a constant instead recovers 348 NIST faces
/// and destroys 268 others, in a disjoint set of models â€” one part in two
/// encodings loses 148 cone faces under whichever window excludes it. Any
/// fixed-size window trades one population for another, because whether a
/// face's material interval falls inside is decided by where its exporter put
/// the reference circle. The extent has to come from the face.
///
/// **A periodic axis is different in kind**: its extent *is* determined, by the
/// period, and the seam handling below relies on the wrap interval being one
/// full period. Only non-periodic axes are re-derived.
///
/// Returns `None` for an axis the bounds do not determine. A degenerate extent
/// means the material region is not recoverable from the boundary alone, and
/// the caller must refuse rather than invent one â€” a collapsed single-vertex
/// bound is exactly that case, marking a point the domain must reach while
/// contributing no trim segment (`QUO-005`, `SNG-COLLAPSED-DIRECTION-001`).
fn working_range(
    pieces: &[PolyBoundaryPiece],
    surface: &impl PreMeshableSurface,
) -> (Option<(f64, f64)>, Option<(f64, f64)>) {
    let ctx = ToleranceCtx::unscaled_legacy();
    let (udeclared, vdeclared) = surface.try_range_tuple();
    // A face with no boundary at all takes its domain from the surface — the
    // closure rectangle `new_with_join` synthesises is that declared domain,
    // never a piece-derived interval (there are no pieces). This is the ball:
    // a full closed sphere trimmed by a vertex loop, admitted only by the
    // converter's closed-surface gate. Returning `None` here would leave
    // `closed` empty and the face would reach the CDT with no boundary at
    // all, which the parity flood cannot turn into material.
    if pieces.is_empty() {
        return (udeclared, vdeclared);
    }
    let axis = |idx: usize, period: Option<f64>, declared: Option<(f64, f64)>| {
        // A period determines the extent; the bounds do not get a say.
        if period.is_some() {
            return declared;
        }
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        pieces.iter().for_each(|PolyBoundaryPiece(vec, _)| {
            vec.iter().for_each(|p| {
                lo = f64::min(lo, p[idx]);
                hi = f64::max(hi, p[idx]);
            })
        });
        (!ctx.is_small_ratio(hi - lo)).then_some((lo, hi)) // BG-TOL-001: param
    };
    (
        axis(0, surface.u_period(), udeclared),
        axis(1, surface.v_period(), vdeclared),
    )
}

/// How the two-closed-loop branch traverses the second loop.
///
/// The branch has always reversed loop 1 unconditionally. For a quotient-closed
/// boundary walk `Σδᵢ = Δ_walk`, with `Δ_walk = 0` for a contractible regular
/// boundary, so the reversal is only correct when the two loops wind the *same*
/// way. The two boundary circles of a band wind opposite — as they must, for
/// the face boundary to be coherently oriented — and there the reversal makes
/// `Σδ = ±2`, which is exactly the crossing the CDT then refuses.
///
/// [`PolyBoundary::new`] runs the two-loop join under [`Self::DeckConsistent`]:
/// the primary rendered-face path chooses the loop-1 traversal from the deck
/// equation. [`Self::Legacy`] is retained for the explicit legacy reference in
/// tests and as the fallback semantics inside `DeckConsistent`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TwoLoopJoinPolicy {
    /// Reverse loop 1 unconditionally.
    Legacy,
    /// Traverse loop 1 in whichever direction satisfies `Σδ = 0`, and fall back
    /// to [`Self::Legacy`] when no direction does or both do. The equation is
    /// decidable, so this guesses nothing: a direction is chosen only when it
    /// is the unique solution.
    DeckConsistent,
}

/// What the two-closed-loop branch concluded about its deck equation.
///
/// Reported rather than inferred, so a caller can tell "the correction changed
/// the boundary" from "there was nothing to correct" without rebuilding and
/// comparing meshes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TwoLoopJoinOutcome {
    /// The branch did not run: the face does not present two degenerate closed
    /// loops on a periodic chart.
    NotAttempted,
    /// `Î£Î´ = 0` already holds for the reversed traversal.
    LegacyDeckConsistent,
    /// `Î£Î´ â‰  0` reversed, and forward traversal is the unique solution.
    /// `applied` says whether the policy let it be taken.
    ForwardResolves {
        /// Whether the forward traversal was used.
        applied: bool,
    },
    /// Neither traversal satisfies the equation. Refused: the legacy join is
    /// retained and the face keeps whatever typed failure it had.
    Inconsistent,
    /// Both traversals satisfy it, so the deck equation does not select one.
    /// Refused for the same reason.
    Unresolved,
}

/// P3b: periodic spherical-cap chart closure.
///
/// A closed latitude-parallel boundary (|k| = 1 around the periodic axis) whose
/// UV image is a 1D line encloses a positive-area spherical region but cannot
/// itself bound a planar material cell. The chart closure builds the
/// contractible cell instead:
///
/// ```text
///      D --(pole line)--> C            r = r_pole  (the collapsed pole)
///      |                  |
///      seam-left          seam-right
///      |                  |
///      A --(source)--> ... --> B       r = r0      (the real latitude walk)
/// ```
///
/// `A -> B` is the source walk (real trim, carries STEP provenance, toggles
/// material parity). `B -> C`, `C -> D` and `D -> A` are synthesised chart
/// closure (origin [`SegmentOrigin::ChartClosure`], no source identity). The
/// source walk is an *open* path in the chart (its periodic wrap is replaced by
/// the meridian seam), so the closure must complete the single toggling
/// boundary: the whole rectangle toggles as one closed loop, and the parity
/// flood selects the interior (the cap). The closure is never a separate
/// material region. The pole is selected on the material side derived from the
/// source-loop orientation times the effective surface normal, so north/south
/// and small/large cap are decisions, never constants.
struct PeriodicCapClosure;

/// Which parameter axis carries the period of a periodic-cap boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PeriodicAxis {
    U,
    V,
}

impl PeriodicCapClosure {
    /// Classify `loop_` as a periodic cap and build its contractible cell, or
    /// return `None` if the loop is not the signature (single 1D |k|=1 periodic
    /// loop on a surface with a certified pole on the material side).
    fn try_build<S: PreMeshableSurface>(
        surface: &S,
        loop_: &BoundaryLoop,
        displacement: [i64; 2],
        tol: f64,
        lattice: &CertifiedLattice,
    ) -> Option<BoundaryLoop> {
        // The epistemic contract of the P3b theorem. Every hypothesis is
        // recorded with its evidence strength when the cap route runs, so a
        // census can answer *why* the route activated or declined and what
        // evidence each hypothesis rested on. Nothing here changes geometry;
        // the record is observational.
        let mut activation = diagnosis::CapActivationRecord {
            periodic_axis: diagnosis::PeriodicAxis::U,
            period: None,
            winding: None,
            cap_signature: diagnosis::CapHypothesisEvidence::NotEstablished,
            collapse: diagnosis::CapHypothesisEvidence::NotEstablished,
            material_side: None,
            activated: false,
            declined_reason: None,
        };
        let record = |activation: &mut diagnosis::CapActivationRecord| {
            if diagnosis::diag_enabled() {
                diagnosis::record_cap_activation(activation.clone());
            }
        };
        // 1. Exactly one periodic axis, winding exactly once.
        let (p_axis, k) = match displacement {
            [ku, 0] if ku.abs() == 1 => (PeriodicAxis::U, ku),
            [0, kv] if kv.abs() == 1 => (PeriodicAxis::V, kv),
            _ => {
                activation.declined_reason = Some("not a single |k|=1 periodic walk");
                record(&mut activation);
                return None;
            }
        };
        activation.periodic_axis = match p_axis {
            PeriodicAxis::U => diagnosis::PeriodicAxis::U,
            PeriodicAxis::V => diagnosis::PeriodicAxis::V,
        };
        // The cap theorem's H1 is a *genuine* period. Only a representation-
        // derived generator qualifies; a declared-but-uncertified accessor
        // value does not establish it, so such a surface does not get a
        // certified cap here (it stays on the candidate/legacy paths). The
        // winding is constructive: a certified period plus the bounded residual
        // the displacement classifier already checked makes the integer exact.
        let period = match p_axis {
            PeriodicAxis::U => match lattice.u_generator() {
                Some(period) => {
                    activation.period = Some(diagnosis::CapHypothesisEvidence::Certified);
                    activation.winding = Some(diagnosis::CapHypothesisEvidence::Constructive);
                    period
                }
                None => {
                    activation.declined_reason = Some("H1: no certified period");
                    record(&mut activation);
                    return None;
                }
            },
            PeriodicAxis::V => match lattice.v_generator() {
                Some(period) => {
                    activation.period = Some(diagnosis::CapHypothesisEvidence::Certified);
                    activation.winding = Some(diagnosis::CapHypothesisEvidence::Constructive);
                    period
                }
                None => {
                    activation.declined_reason = Some("H1: no certified period");
                    record(&mut activation);
                    return None;
                }
            },
        };
        // 2. The chart image must be a 1D line: zero signed area, and the
        //    non-periodic coordinate essentially constant across the loop.
        //    These are recognizer-level signature checks (H3), not certificates
        //    that the loop belongs to the theorem's source class.
        if signed_area(&loop_.points).abs() >= DEGENERATE_LOOP_AREA {
            activation.cap_signature = diagnosis::CapHypothesisEvidence::NotEstablished;
            activation.declined_reason = Some("H3: loop is not a 1D chart line (nonzero area)");
            record(&mut activation);
            return None;
        }
        let (n_min, n_max) = match p_axis {
            PeriodicAxis::U => loop_
                .points
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
                    (lo.min(p.uv.y), hi.max(p.uv.y))
                }),
            PeriodicAxis::V => loop_
                .points
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
                    (lo.min(p.uv.x), hi.max(p.uv.x))
                }),
        };
        if n_max - n_min > 0.1 * period {
            activation.cap_signature = diagnosis::CapHypothesisEvidence::NotEstablished;
            activation.declined_reason = Some("H3: non-periodic span not small");
            record(&mut activation);
            return None;
        }
        activation.cap_signature = diagnosis::CapHypothesisEvidence::Candidate;
        let r0 = 0.5 * (n_min + n_max);
        // 3. The pole on the material side; `None` when the material side has
        //    no orbit collapse, which is not a cap. The evidence is consumed
        //    below: the theorem's H4 (genuine collapse) is discharged by the
        //    certified sphere pole, or nominated by the candidate scan.
        let pole = match find_cap_pole(surface, loop_, p_axis, r0, period, lattice) {
            Some(pole) => pole,
            None => {
                activation.declined_reason = Some("H4/H5: no orbit collapse on material side");
                record(&mut activation);
                return None;
            }
        };
        let r_pole = pole.r_pole();
        match pole {
            CapPoleEvidence::CertifiedSpherePole { .. } => {
                activation.collapse = diagnosis::CapHypothesisEvidence::Certified;
            }
            CapPoleEvidence::Candidate { .. } => {
                activation.collapse = diagnosis::CapHypothesisEvidence::Candidate;
            }
        }
        activation.material_side = Some(diagnosis::CapHypothesisEvidence::Constructive);
        activation.activated = true;
        record(&mut activation);
        // 4. Build the contractible planar cell.
        Some(build_cap_cell(surface, loop_, p_axis, k, r0, r_pole, tol))
    }
}

/// The non-periodic coordinate of a point, on the axis that is *not* periodic.
fn non_periodic_comp(p: &SurfacePoint, p_axis: PeriodicAxis) -> f64 {
    match p_axis {
        PeriodicAxis::U => p.uv.y,
        PeriodicAxis::V => p.uv.x,
    }
}

/// The periodic coordinate of a point, on the periodic axis.
fn periodic_comp(p: &SurfacePoint, p_axis: PeriodicAxis) -> f64 {
    match p_axis {
        PeriodicAxis::U => p.uv.x,
        PeriodicAxis::V => p.uv.y,
    }
}

/// Twice the diameter of the angular orbit at non-periodic coordinate `r`:
/// the distance between two surface points half a period apart. This vanishes
/// exactly at a pole, where the periodic orbit collapses.
fn orbit_diameter<S: PreMeshableSurface>(
    surface: &S,
    p_axis: PeriodicAxis,
    r: f64,
    period: f64,
) -> f64 {
    let (pa, pb) = match p_axis {
        PeriodicAxis::U => {
            let v = r;
            (surface.subs(0.0, v), surface.subs(0.5 * period, v))
        }
        PeriodicAxis::V => {
            let u = r;
            (surface.subs(u, 0.0), surface.subs(u, 0.5 * period))
        }
    };
    pa.distance(pb)
}

/// The evidence behind a located cap pole.
///
/// The P3b construction theorem's H4 requires a *genuine* collapsed periodic
/// orbit: `q(r_pole, θ) = P` for every `θ`. Only a representation-derived
/// certificate establishes that. A numerically-shrunk orbit is a candidate
/// recognizer: it may nominate a pole location and let the cap be attempted,
/// but it is not a source-level certificate that the orbit truly collapses.
#[derive(Clone, Copy, Debug, PartialEq)]
enum CapPoleEvidence {
    /// The primitive's polar latitude collapses the orbit by construction.
    /// `r_pole` is the material-side extreme of the polar parameter range,
    /// where `Sphere::subs` is independent of the azimuth.
    CertifiedSpherePole {
        /// The non-periodic coordinate of the pole.
        r_pole: f64,
    },
    /// A numerical orbit-diameter scan found a collapse below a relative
    /// threshold. A candidate, not a certificate.
    Candidate {
        /// The non-periodic coordinate of the nominated pole.
        r_pole: f64,
    },
}

impl CapPoleEvidence {
    /// The non-periodic coordinate of the located pole, whatever its evidence
    /// strength.
    fn r_pole(self) -> f64 {
        match self {
            Self::CertifiedSpherePole { r_pole } | Self::Candidate { r_pole } => r_pole,
        }
    }
}

/// Locate the pole on the material side of the latitude walk.
///
/// The material side is decided by the STEP face orientation convention: the
/// material lies on the left of the directed boundary as seen from the side the
/// effective surface normal points toward, i.e. `n x t` points into the
/// material. `n` is the surface normal (which already carries
/// `FACE_SURFACE.same_sense` via `surface.invert()`); `t` is the walk tangent
/// (which already carries `FACE_BOUND x ORIENTED_EDGE x EDGE_CURVE`). The
/// normal's component along the non-periodic axis selects which pole bounds the
/// cap, so north/south and small/large are derived, never hard-coded.
///
/// **Evidence.** When `lattice.certified_collapse()` certifies a pole on this
/// loop's polar axis, the pole is the material-side extreme of the polar
/// parameter range, and the orbit collapses there by construction of the
/// primitive (a sphere: `subs` enters the azimuth only through `(cos v, sin v)`,
/// so at the polar latitudes the physical map is a single point). The scan is
/// then a consistency confirmation, not the certificate. Otherwise the
/// orbit-diameter scan nominates a candidate location, which the caller may
/// attempt but must not treat as a certified collapse.
///
/// **Search domain.** The scan evaluates the surface only at points of the
/// declared polar range. The cap path is gated on a *certified* deck generator,
/// which only elementary surfaces (sphere, revolution of a line) carry, and
/// those surfaces are fully evaluable over their declared range — so the
/// declared range *is* the basis-valid evaluation domain here, and no spline
/// closure-sliver exclusion is needed. The P1 `evaluation_range` distinction
/// bites on spline *curves*, which never reach this path.
fn find_cap_pole<S: PreMeshableSurface>(
    surface: &S,
    loop_: &BoundaryLoop,
    p_axis: PeriodicAxis,
    r0: f64,
    period: f64,
    lattice: &CertifiedLattice,
) -> Option<CapPoleEvidence> {
    let n = loop_.points.len();
    if n < 3 {
        return None;
    }
    let mid = n / 2;
    let (pa, pb, pc) = (
        loop_.points[(mid + n - 1) % n],
        loop_.points[mid],
        loop_.points[(mid + 1) % n],
    );
    let t = (pb.point - pa.point) + (pc.point - pb.point);
    if t.magnitude2() <= f64::EPSILON {
        return None;
    }
    let t = t.normalize();
    let nrm = surface.normal(pb.uv.x, pb.uv.y);
    if nrm.magnitude2() <= f64::EPSILON {
        return None;
    }
    let nrm = nrm.normalize();
    let into_material = nrm.cross(t);
    let ndir = match p_axis {
        PeriodicAxis::U => surface.vder(pb.uv.x, pb.uv.y),
        PeriodicAxis::V => surface.uder(pb.uv.x, pb.uv.y),
    };
    let n_range = match p_axis {
        PeriodicAxis::U => surface.try_range_tuple().1,
        PeriodicAxis::V => surface.try_range_tuple().0,
    };
    let dir = if into_material.dot(ndir) >= 0.0 {
        1.0
    } else {
        -1.0
    };
    let (n_lo, n_hi) = n_range?;
    let boundary = if dir > 0.0 { n_hi } else { n_lo };
    let span = (boundary - r0) * dir;
    if span <= 1e-9 {
        return None;
    }
    // The polar axis is the axis that is *not* periodic.
    let polar_axis = match p_axis {
        PeriodicAxis::U => Axis::V,
        PeriodicAxis::V => Axis::U,
    };
    // A representation-certified pole: the primitive's own polar latitude, on
    // the material side, with the orbit collapsing there by construction. The
    // relative-threshold scan below would find the same point; it is kept only
    // as a consistency confirmation for the certified path, never as its
    // evidence.
    if lattice
        .certified_collapse()
        .is_some_and(|c| c.polar == polar_axis && c.witness == CollapseWitness::ExactSpherePole)
    {
        return Some(CapPoleEvidence::CertifiedSpherePole { r_pole: boundary });
    }
    // No certificate: a numerical orbit-diameter scan, relative to the loop's
    // own orbit diameter so it scales with the surface. This nominates a pole;
    // it does not certify the collapse (H4 stays candidate for this route).
    let r_loop = orbit_diameter(surface, p_axis, r0, period);
    if r_loop <= 1e-12 || !r_loop.is_finite() {
        return None;
    }
    let threshold = 1e-4 * r_loop;
    let scan = |steps: usize| -> Option<f64> {
        let mut best_r = r0;
        let mut best_rr = r_loop;
        for i in 1..=steps {
            let frac = i as f64 / steps as f64;
            let r = r0 + dir * span * frac;
            let rr = orbit_diameter(surface, p_axis, r, period);
            if rr.is_finite() && rr < best_rr {
                best_rr = rr;
                best_r = r;
            }
        }
        (best_rr < threshold).then_some(best_r)
    };
    let coarse = scan(64)?;
    // Golden-section minimise the orbit diameter over the coarse confidence
    // window around the collapse.
    let (a, b) = (coarse - dir * span / 64.0, coarse + dir * span / 64.0);
    let mut lo = a.min(b);
    let mut hi = a.max(b);
    lo = lo.clamp(r0.min(boundary), r0.max(boundary));
    hi = hi.clamp(r0.min(boundary), r0.max(boundary));
    let mut refine = |f: &dyn Fn(f64) -> f64| -> f64 {
        const PHI: f64 = 1.618033988749895;
        let mut c = hi - (hi - lo) / PHI;
        let mut d = lo + (hi - lo) / PHI;
        let mut fc = f(c);
        let mut fd = f(d);
        while (hi - lo).abs() > 1e-9 {
            if fc < fd {
                hi = d;
                d = c;
                fd = fc;
                c = hi - (hi - lo) / PHI;
                fc = f(c);
            } else {
                lo = c;
                c = d;
                fc = fd;
                d = lo + (hi - lo) / PHI;
                fd = f(d);
            }
        }
        0.5 * (lo + hi)
    };
    let f = |r: f64| orbit_diameter(surface, p_axis, r, period);
    let refined = refine(&f);
    let rr = orbit_diameter(surface, p_axis, refined, period);
    (rr.is_finite() && rr < threshold).then(|| CapPoleEvidence::Candidate { r_pole: refined })
}

/// Build the contractible planar cell for a periodic cap.
///
/// The source latitude walk is kept verbatim (every real segment retains its
/// provenance); the meridian runs and the pole line are synthetic
/// ([`SegmentOrigin::ChartClosure`], empty contributor sets).
fn build_cap_cell<S: PreMeshableSurface>(
    surface: &S,
    loop_: &BoundaryLoop,
    p_axis: PeriodicAxis,
    _k: i64,
    _r0: f64,
    r_pole: f64,
    tol: f64,
) -> BoundaryLoop {
    // The source walk without its periodic wrap, as an open path A -> B.
    let mut path = loop_.clone().into_path_cutting_wrap();
    let a_uv = path.points.first().map(|p| p.uv).unwrap();
    let b_uv = path.points.last().map(|p| p.uv).unwrap();
    let p0 = periodic_comp(&loop_.points[0], p_axis);
    let p1 = periodic_comp(loop_.points.last().unwrap(), p_axis);
    // Corner points of the cell, evaluated from the surface.
    let corner = |n: f64, p: f64| -> SurfacePoint {
        let uv = match p_axis {
            PeriodicAxis::U => Point2::new(p, n),
            PeriodicAxis::V => Point2::new(n, p),
        };
        (uv, surface.subs(uv.x, uv.y)).into()
    };
    let c = corner(r_pole, p1);
    let d = corner(r_pole, p0);
    // The meridian runs and the degenerate pole line. Only the two pole-line
    // endpoints are kept: every additional point on it would be another vertex
    // at the collapsed pole and another degenerate triangle to filter.
    let seam_down = polyline_on_surface(surface, *path.points.last().unwrap(), c, tol);
    let seam_up = polyline_on_surface(surface, d, *path.points.first().unwrap(), tol);
    let pole_line = vec![c, d];
    let (n_down, n_pole, n_up) = (seam_down.len(), pole_line.len(), seam_up.len());
    path.append(
        seam_down,
        untagged_sources(n_down),
        SegmentOrigin::ChartClosure,
        PartJoin::SharedEndpoint,
    );
    path.append(
        pole_line,
        untagged_sources(n_pole),
        SegmentOrigin::ChartClosure,
        PartJoin::SharedEndpoint,
    );
    path.append(
        seam_up,
        untagged_sources(n_up),
        SegmentOrigin::ChartClosure,
        PartJoin::SharedEndpoint,
    );
    let _ = (a_uv, b_uv);
    path.close(PartJoin::SharedEndpoint)
}

/// The first-pass two-loop join policy for a face's boundary pieces.
///
/// Selects [`TwoLoopJoinPolicy::DeckConsistent`] only for the certified
/// structural deck-pair class: exactly two closed loops, both genuine full-period
/// deck walks (non-zero lattice displacement), whose displacements satisfy the
/// deck equation in exactly one traversal direction. Everything else keeps
/// [`TwoLoopJoinPolicy::Legacy`], so the legacy area gate and the recovery
/// DeckConsistent arm (which remains gated on a legacy
/// `ContradictoryDualParity` failure) stay byte-identical for every other face.
///
/// The classification reproduces exactly what `new_with_join` computes per
/// piece, so the primary path and this selector cannot disagree about which
/// loops are closed or by how much they wind.
fn primary_two_loop_join_policy(
    pieces: &[PolyBoundaryPiece],
    lattice: &CertifiedLattice,
) -> TwoLoopJoinPolicy {
    let u_period = lattice.declared_u_period();
    let v_period = lattice.declared_v_period();
    let mut displacements: Vec<[i64; 2]> = Vec::new();
    for PolyBoundaryPiece(vec, _) in pieces {
        let p0 = vec[0].uv;
        let p1 = vec[vec.len() - 1].uv;
        if p0.distance(p1) < 1.0e-3 {
            displacements.push([0, 0]);
            continue;
        }
        let ku = u_period
            .and_then(|up| periodic_displacement(p0.x, p1.x, up, 1e-3))
            .unwrap_or(0);
        let kv = v_period
            .and_then(|vp| periodic_displacement(p0.y, p1.y, vp, 1e-3))
            .unwrap_or(0);
        if (ku != 0 || kv != 0) && vec[0].point.distance(vec[vec.len() - 1].point) < 1e-3 {
            displacements.push([ku, kv]);
        }
    }
    if displacements.len() == 2 {
        let d0 = displacements[0];
        let d1 = displacements[1];
        // Both loops are genuine deck walks, and the deck equation decides a
        // traversal (`δ₀ = δ₁` reversed, or `δ₀ = −δ₁` forward, but not
        // both and not neither). A decided equation is what makes the join a
        // choice rather than a coin toss.
        if d0 != [0, 0] && d1 != [0, 0] && (d0 == d1) != (d0 == [-d1[0], -d1[1]]) {
            return TwoLoopJoinPolicy::DeckConsistent;
        }
    }
    TwoLoopJoinPolicy::Legacy
}

impl PolyBoundary {
    fn new(
        pieces: Vec<PolyBoundaryPiece>,
        surface: &impl PreMeshableSurface,
        tol: f64,
        lattice: &CertifiedLattice,
    ) -> Self {
        // The primary rendered-face path resolves the two-loop join against the
        // periodic deck equation rather than the legacy unconditional reversal.
        // Reversing loop 1 unconditionally realises `I'�,? �^' I'�,?`; for the two
        // boundary circles of a band that is `A�2` �?" the crossing seam bridge
        // that PLANAR-C then planarizes into a chart-centre pivot and the
        // radius-scale fan. The deck-consistent policy keeps the legacy
        // reversal when it closes (`I'�,? = I'�,?`) and takes forward traversal only
        // when that is the unique solution (`I'�,? = �^'I'�,?`). Only the
        // certified structural deck-pair class is routed through DeckConsistent
        // on the primary path (INV-W2-1 byte-identity for every other face);
        // the phase correspondence (`align_two_loop_phase`) still applies inside
        // the deck-consistent arm.
        let join_policy = primary_two_loop_join_policy(&pieces, lattice);
        Self::new_with_join(pieces, surface, tol, lattice, join_policy).0
    }

    fn new_with_join(
        pieces: Vec<PolyBoundaryPiece>,
        surface: &impl PreMeshableSurface,
        tol: f64,
        lattice: &CertifiedLattice,
        join_policy: TwoLoopJoinPolicy,
    ) -> (Self, TwoLoopJoinOutcome) {
        let ctx = ToleranceCtx::unscaled_legacy();
        let mut join_outcome = TwoLoopJoinOutcome::NotAttempted;
        let probe = std::env::var_os("TRUCK_PROBE_BOUNDARY").is_some();
        let had_source_pieces = !pieces.is_empty();
        // The synthetic closure rectangle comes from the face's own bounds,
        // never from the supporting primitive's declared parameter range. A
        // surface's declared range is a property of the primitive it was
        // constructed from, not of any face that references it (a
        // `RevolutedCurve` inherits `Line::parameter_range` = `[0, 1]`); a
        // trimmed face's material interval can sit far inside it, and closing
        // an open boundary piece against the full declared rectangle walks an
        // artificial parameter span unrelated to the face. That inflated
        // closure expands the interior sampling domain `insert_surface`
        // subdivides and hands the CDT pathological constraint geometry. The
        // face-derived `working_range` bounds synthetic closure by the boundary
        // evidence this face already carries.
        let range = working_range(&pieces, surface);
        let (mut closed, mut open) = (Vec::new(), Vec::new());
        // The lattice displacement of each closed loop, parallel to `closed`.
        // The `BoundaryLoop` the classification produces does not retain it,
        // and the two-closed-loop branch below needs it to say what its join
        // does to the deck sum â€” recovering it afterwards from normalised
        // points would re-derive an integer the classifier already decided.
        let mut closed_displacements: Vec<[i64; 2]> = Vec::new();
        let u_period = lattice.declared_u_period();
        let v_period = lattice.declared_v_period();
        // DIAG-001 deck evidence. Recorded where each piece is classified, so
        // the displacement written down is the one the pipeline acted on rather
        // than one recovered later from already-normalised points.
        let diag = diagnosis::diag_enabled();
        // DIAG-002 realized-extent accumulation. A small O(boundary) min/max
        // pass over samples the pipeline already materialized; nothing extra is
        // evaluated. `world_seen` distinguishes "a face with no extent" from
        // "no boundary at all".
        let mut world_lo = [f64::INFINITY; 3];
        let mut world_hi = [f64::NEG_INFINITY; 3];
        let mut uv_lo = [f64::INFINITY; 2];
        let mut uv_hi = [f64::NEG_INFINITY; 2];
        let mut world_seen = false;
        pieces.into_iter().enumerate().for_each(
            |(piece_index, PolyBoundaryPiece(mut vec, mut sources))| {
                if diag {
                    for p in &vec {
                        world_lo[0] = world_lo[0].min(p.point.x);
                        world_hi[0] = world_hi[0].max(p.point.x);
                        world_lo[1] = world_lo[1].min(p.point.y);
                        world_hi[1] = world_hi[1].max(p.point.y);
                        world_lo[2] = world_lo[2].min(p.point.z);
                        world_hi[2] = world_hi[2].max(p.point.z);
                        uv_lo[0] = uv_lo[0].min(p.uv.x);
                        uv_hi[0] = uv_hi[0].max(p.uv.x);
                        uv_lo[1] = uv_lo[1].min(p.uv.y);
                        uv_hi[1] = uv_hi[1].max(p.uv.y);
                        world_seen = true;
                    }
                }
                let p0 = vec[0].uv;
                let p1 = vec[vec.len() - 1].uv;

                let closure = if p0.distance(p1) < 1.0e-3 {
                    BoundaryClosure::EuclideanClosed
                } else {
                    let ku = u_period
                        .and_then(|up| periodic_displacement(p0.x, p1.x, up, 1e-3))
                        .unwrap_or(0);
                    let kv = v_period
                        .and_then(|vp| periodic_displacement(p0.y, p1.y, vp, 1e-3))
                        .unwrap_or(0);
                    if (ku != 0 || kv != 0)
                        && vec[0].point.distance(vec[vec.len() - 1].point) < 1e-3
                    {
                        BoundaryClosure::PeriodicClosed {
                            displacement: [ku, kv],
                        }
                    } else {
                        BoundaryClosure::Open
                    }
                };

                if probe {
                    let perimeter: f64 = vec
                        .windows(2)
                        .map(|w| w[0].uv.distance(w[1].uv))
                        .sum::<f64>();
                    eprintln!(
                        "PROBE piece pts={} gap={:.6e} perimeter={perimeter:.6e} \
                     closure={:?}",
                        vec.len(),
                        p0.distance(p1),
                        closure,
                    );
                }

                match closure {
                    BoundaryClosure::EuclideanClosed => {
                        vec.pop();
                        // The piece's provenance had one entry per cyclic
                        // segment including the degenerate wrap back onto the
                        // duplicate closing point; that entry dies with the
                        // point it described.
                        sources.pop();
                        if diag {
                            record_piece_deck(
                                piece_index,
                                &vec,
                                ObservedClosure::EuclideanClosed,
                                0,
                                0,
                            );
                        }
                        closed_displacements.push([0, 0]);
                        closed.push(BoundaryLoop::euclidean_source_loop(vec, sources));
                    }
                    BoundaryClosure::PeriodicClosed {
                        displacement: [ku, kv],
                    } => {
                        if let Some(up) = u_period {
                            if ku != 0 {
                                for p in &mut vec {
                                    p.uv.x -= (ku as f64) * up;
                                }
                                vec.last_mut().unwrap().uv.x = vec[0].uv.x + (ku as f64) * up;
                            }
                        }
                        if let Some(vp) = v_period {
                            if kv != 0 {
                                for p in &mut vec {
                                    p.uv.y -= (kv as f64) * vp;
                                }
                                vec.last_mut().unwrap().uv.y = vec[0].uv.y + (kv as f64) * vp;
                            }
                        }
                        if diag {
                            record_piece_deck(
                                piece_index,
                                &vec,
                                ObservedClosure::PeriodicClosed,
                                ku,
                                kv,
                            );
                        }
                        closed_displacements.push([ku, kv]);
                        closed.push(BoundaryLoop::periodic_source_walk(vec, sources));
                    }
                    BoundaryClosure::Open => {
                        if diag {
                            record_piece_deck(piece_index, &vec, ObservedClosure::Open, 0, 0);
                        }
                        // The piece's provenance is cyclic (one entry per point,
                        // the last labelling the wrap back onto the start). As an
                        // open chain that wrap segment does not exist — it is
                        // re-created by the closure join — so the chain's
                        // provenance has one fewer entry.
                        sources.pop();
                        open.push((vec, sources))
                    }
                }
            },
        );
        if diag && world_seen {
            diagnosis::record_world_uv_extents(
                diagnosis::DiagnosticBBox3 {
                    lo: world_lo,
                    hi: world_hi,
                },
                diagnosis::DiagnosticBBox2 {
                    lo: uv_lo,
                    hi: uv_hi,
                },
            );
        }
        if let Some(cap) = match closed.as_slice() {
            // P3b: a single 1D periodic walk that winds once around the
            // periodic axis is a spherical-cap boundary, not a degenerate
            // band. Build the contractible cell before any two-loop / apex
            // join has a chance to misread it. Any non-cap single loop falls
            // through unchanged.
            [loop_] if open.is_empty() => {
                PeriodicCapClosure::try_build(surface, loop_, closed_displacements[0], tol, lattice)
            }
            _ => None,
        } {
            closed = vec![cap];
        } else if closed.len() == 2
            && (lattice.declared_u_period().is_some() || lattice.declared_v_period().is_some())
        {
            let area0 = signed_area(&closed[0].points);
            let area1 = signed_area(&closed[1].points);
            // ARR-SEAM W2: a valid non-degenerate deck pair must reach this
            // join too. The legacy area gate admits only the collapsed cohort;
            // under `DeckConsistent` the semantic predicate is that both loops
            // are genuine deck walks (non-zero lattice displacement), which is
            // what `PolyBoundary::new` now passes on the primary rendered-face
            // path. The legacy area condition stays the first disjunct so the
            // collapsed cohort is admitted under both policies; the deck
            // equation and its `Unresolved`/`Inconsistent` refusal are already
            // computed below.
            let deck_pair = join_policy == TwoLoopJoinPolicy::DeckConsistent
                && closed_displacements[0] != [0, 0]
                && closed_displacements[1] != [0, 0];
            if (area0.abs() < DEGENERATE_LOOP_AREA && area1.abs() < DEGENERATE_LOOP_AREA)
                || deck_pair
            {
                let loop0_displacement = closed_displacements[0];
                let loop1_displacement = closed_displacements[1];
                let mut mean_translate = [0i64, 0i64];
                let loop0 = closed.remove(0);
                let mut loop1 = closed.remove(0);
                let u0_mean: f64 =
                    loop0.points.iter().map(|p| p.uv.x).sum::<f64>() / loop0.points.len() as f64;
                let u1_mean: f64 =
                    loop1.points.iter().map(|p| p.uv.x).sum::<f64>() / loop1.points.len() as f64;
                if let Some(up) = lattice.declared_u_period() {
                    let ku = ((u0_mean - u1_mean) / up).round();
                    mean_translate[0] = ku as i64;
                    if ku != 0.0 {
                        for p in &mut loop1.points {
                            p.uv.x += ku * up;
                        }
                    }
                }
                let v0_mean: f64 =
                    loop0.points.iter().map(|p| p.uv.y).sum::<f64>() / loop0.len() as f64;
                let v1_mean: f64 =
                    loop1.points.iter().map(|p| p.uv.y).sum::<f64>() / loop1.len() as f64;
                if let Some(vp) = lattice.declared_v_period() {
                    let kv = ((v0_mean - v1_mean) / vp).round();
                    mean_translate[1] = kv as i64;
                    if kv != 0.0 {
                        for p in &mut loop1.points {
                            p.uv.y += kv * vp;
                        }
                    }
                }
                // PHASE-CORRESPONDENCE. For a band whose two bounds are each a
                // single full-period circle edge, the source establishes no
                // seam correspondence: neither the circle edges nor the
                // placement ref directions connect a specific point of one
                // circle to a specific point of the other. The correct
                // correspondence is the surface's own generator structure —
                // points with equal periodic coordinate v (mod period) lie on
                // a common ruling, and the seam bridges must be rulings. The
                // integer mean-translate above preserves the fractional phase
                // residual (a π offset is irreducible by integer periods), so
                // cyclically re-index loop1's samples so both loops share the
                // same seam reference (mod period). This is a pure re-lift:
                // every realized 3D point is preserved.
                align_two_loop_phase(&loop0, &mut loop1, loop1_displacement, lattice);
                // Both halves are source-derived, but joining a loop to a
                // *reversed* loop introduces two segments that neither
                // supplied: the jump from `loop0`'s end to `loop1`'s reversed
                // start, and the closing wrap back to `loop0`'s start. Building
                // this by parts labels those bridges instead of letting them
                // inherit `Source`.
                // Solve the deck equation before choosing a traversal. Reversing
                // loop 1 realises `Î´â‚€ âˆ’ Î´â‚`; traversing it forward realises
                // `Î´â‚€ + Î´â‚`. `Î”_walk = 0`, so each direction is admissible
                // exactly when its sum vanishes, and the direction is *chosen*
                // only when precisely one does.
                let reversed_closes = loop0_displacement[0] == loop1_displacement[0]
                    && loop0_displacement[1] == loop1_displacement[1];
                let forward_closes = loop0_displacement[0] == -loop1_displacement[0]
                    && loop0_displacement[1] == -loop1_displacement[1];
                let take_forward = match (reversed_closes, forward_closes) {
                    (false, true) => {
                        let applied = join_policy == TwoLoopJoinPolicy::DeckConsistent;
                        join_outcome = TwoLoopJoinOutcome::ForwardResolves { applied };
                        applied
                    }
                    (true, false) => {
                        join_outcome = TwoLoopJoinOutcome::LegacyDeckConsistent;
                        false
                    }
                    // Both zero â€” the loops are Euclidean-closed on a periodic
                    // chart, so the equation says nothing about direction â€” or
                    // neither, which is a boundary the deck model does not
                    // describe. Refuse in both cases and keep the legacy
                    // traversal, so a face can only be recovered on a decided
                    // equation and never on a coin toss.
                    (true, true) => {
                        join_outcome = TwoLoopJoinOutcome::Unresolved;
                        false
                    }
                    (false, false) => {
                        join_outcome = TwoLoopJoinOutcome::Inconsistent;
                        false
                    }
                };
                let mut loop1_path = loop1.into_path_cutting_wrap();
                if !take_forward {
                    loop1_path.reverse();
                }
                let mut path = loop0.into_path_cutting_wrap();
                if diag {
                    record_two_loop_join(
                        loop0_displacement,
                        loop1_displacement,
                        mean_translate,
                        !take_forward,
                        &path,
                        &loop1_path,
                    );
                }
                // The two loops are disconnected: joining them creates a
                // segment neither supplied, and so does closing back to the
                // start. Both are declared, so no source point is dropped to
                // manufacture a shortcut.
                path.append_path(loop1_path, PartJoin::Bridge(SegmentOrigin::Seam));
                closed.push(path.close(PartJoin::Bridge(SegmentOrigin::Seam)));
            }
        } else if let Some(pair) = CollapsedPeriodicBoundaryPair::try_classify(
            surface,
            &closed.iter().map(|l| l.points.clone()).collect::<Vec<_>>(),
            &open.iter().map(|(pts, _)| pts.clone()).collect::<Vec<_>>(),
            range,
            lattice,
        ) {
            let closed_loop0 = closed.remove(0);
            let mut loop0 = closed_loop0.points;
            let mut loop0_sources = closed_loop0.source_uses;
            if loop0.len() > 1 && loop0[0].uv.distance(loop0.last().unwrap().uv) < 1e-3 {
                loop0.pop();
                loop0_sources.pop();
            }
            let is_v = lattice.declared_v_period().is_some_and(|p| p > 1e-6);
            let period = if is_v {
                lattice.declared_v_period().unwrap()
            } else {
                lattice.declared_u_period().unwrap()
            };

            let mut loop0_full = loop0.clone();
            let mut last_p = *loop0.last().unwrap();
            if is_v {
                last_p.uv.y = loop0[0].uv.y + period;
            } else {
                last_p.uv.x = loop0[0].uv.x + period;
            }
            loop0_full.push(last_p);
            // `loop0_full` runs the base loop then one appended period-wrap
            // point. Its provenance is the loop's, with the loop's own wrap
            // entry replaced by the synthetic period-wrap segment.
            let mut loop0_full_sources = loop0_sources;
            loop0_full_sources.pop();
            loop0_full_sources.push(Vec::new());

            let loop1_full: Vec<SurfacePoint> = loop0_full
                .iter()
                .map(|p| {
                    let uv = if is_v {
                        Point2::new(pair.apex_u, p.uv.y)
                    } else {
                        Point2::new(p.uv.x, pair.apex_u)
                    };
                    (uv, surface.subs(uv.x, uv.y)).into()
                })
                .collect();

            let end_loop0 = *loop0_full.last().unwrap();
            let loop1_rev: Vec<SurfacePoint> = loop1_full.into_iter().rev().collect();
            let start_loop1 = loop1_rev[0];
            let end_loop1 = *loop1_rev.last().unwrap();
            let start_loop0 = loop0_full[0];

            let seam_down = polyline_on_surface(surface, end_loop0, start_loop1, tol);
            let seam_up = polyline_on_surface(surface, end_loop1, start_loop0, tol);

            // Only `loop0_full` carries source evidence, and even it ends with
            // an appended period-wrap point rather than a source sample. The
            // apex branch `loop1_rev` is *evaluated from the surface* at
            // `pair.apex_u` â€” synthesised geometry that no source edge
            // describes â€” and the two joining runs are seams across the
            // collapsed direction. None of those three parts carry a source
            // edge use, so their provenance entries are empty.
            let (ns_down, ns_rev, ns_up) = (seam_down.len(), loop1_rev.len(), seam_up.len());
            closed.push(BoundaryLoop::chained([
                (loop0_full, loop0_full_sources, SegmentOrigin::Source),
                (seam_down, untagged_sources(ns_down), SegmentOrigin::Seam),
                (loop1_rev, untagged_sources(ns_rev), SegmentOrigin::Seam),
                (seam_up, untagged_sources(ns_up), SegmentOrigin::Seam),
            ]));
        }
        let (n_closed_in, n_open_in) = (closed.len(), open.len());
        // `connect_edges` used to live here. It dropped each part's last point
        // unconditionally, which is correct only when parts chain â€” the
        // assumption `BoundaryPath::append` now makes the caller state, so the
        // helper has no remaining callers.
        match open.len() {
            1 => {
                let (mut curve, mut curve_sources) = open.pop().unwrap();
                let p = curve[0];
                let q = curve[curve.len() - 1];
                if let (Some((u0, u1)), Some((v0, v1))) = range {
                    if p.x < q.x - ctx.ratio_margin() {
                        // BG-TOL-001: param
                        normalize_range(&mut curve, &mut curve_sources, 0, (u0, u1));
                        let p = curve[0];
                        let q = curve[curve.len() - 1];
                        let x = (Point2::new(u0, v1), surface.subs(u0, v1)).into();
                        let y = (Point2::new(u1, v1), surface.subs(u1, v1)).into();
                        let vec0 = polyline_on_surface(surface, q, y, tol);
                        let vec1 = polyline_on_surface(surface, y, x, tol);
                        let vec2 = polyline_on_surface(surface, x, p, tol);
                        let (n0, n1, n2) = (vec0.len(), vec1.len(), vec2.len());
                        closed.push(BoundaryLoop::chained([
                            (vec0, untagged_sources(n0), SegmentOrigin::SyntheticClosure),
                            (vec1, untagged_sources(n1), SegmentOrigin::SyntheticClosure),
                            (vec2, untagged_sources(n2), SegmentOrigin::SyntheticClosure),
                            (curve, curve_sources, SegmentOrigin::Source),
                        ]));
                    } else if q.x < p.x - ctx.ratio_margin() {
                        // BG-TOL-001: param
                        normalize_range(&mut curve, &mut curve_sources, 0, (u0, u1));
                        let p = curve[0];
                        let q = curve[curve.len() - 1];
                        let x = (Point2::new(u1, v0), surface.subs(u1, v0)).into();
                        let y = (Point2::new(u0, v0), surface.subs(u0, v0)).into();
                        let vec0 = polyline_on_surface(surface, q, y, tol);
                        let vec1 = polyline_on_surface(surface, y, x, tol);
                        let vec2 = polyline_on_surface(surface, x, p, tol);
                        let (n0, n1, n2) = (vec0.len(), vec1.len(), vec2.len());
                        closed.push(BoundaryLoop::chained([
                            (vec0, untagged_sources(n0), SegmentOrigin::SyntheticClosure),
                            (vec1, untagged_sources(n1), SegmentOrigin::SyntheticClosure),
                            (vec2, untagged_sources(n2), SegmentOrigin::SyntheticClosure),
                            (curve, curve_sources, SegmentOrigin::Source),
                        ]));
                    } else if p.y < q.y - ctx.ratio_margin() {
                        // BG-TOL-001: param
                        normalize_range(&mut curve, &mut curve_sources, 1, (v0, v1));
                        let p = curve[0];
                        let q = curve[curve.len() - 1];
                        let x = (Point2::new(u0, v0), surface.subs(u0, v0)).into();
                        let y = (Point2::new(u0, v1), surface.subs(u0, v1)).into();
                        let vec0 = polyline_on_surface(surface, q, y, tol);
                        let vec1 = polyline_on_surface(surface, y, x, tol);
                        let vec2 = polyline_on_surface(surface, x, p, tol);
                        let (n0, n1, n2) = (vec0.len(), vec1.len(), vec2.len());
                        closed.push(BoundaryLoop::chained([
                            (vec0, untagged_sources(n0), SegmentOrigin::SyntheticClosure),
                            (vec1, untagged_sources(n1), SegmentOrigin::SyntheticClosure),
                            (vec2, untagged_sources(n2), SegmentOrigin::SyntheticClosure),
                            (curve, curve_sources, SegmentOrigin::Source),
                        ]));
                    } else if q.y < p.y - ctx.ratio_margin() {
                        // BG-TOL-001: param
                        normalize_range(&mut curve, &mut curve_sources, 1, (v0, v1));
                        let p = curve[0];
                        let q = curve[curve.len() - 1];
                        let x = (Point2::new(u1, v1), surface.subs(u1, v1)).into();
                        let y = (Point2::new(u1, v0), surface.subs(u1, v0)).into();
                        let vec0 = polyline_on_surface(surface, q, y, tol);
                        let vec1 = polyline_on_surface(surface, y, x, tol);
                        let vec2 = polyline_on_surface(surface, x, p, tol);
                        let (n0, n1, n2) = (vec0.len(), vec1.len(), vec2.len());
                        closed.push(BoundaryLoop::chained([
                            (vec0, untagged_sources(n0), SegmentOrigin::SyntheticClosure),
                            (vec1, untagged_sources(n1), SegmentOrigin::SyntheticClosure),
                            (vec2, untagged_sources(n2), SegmentOrigin::SyntheticClosure),
                            (curve, curve_sources, SegmentOrigin::Source),
                        ]));
                    }
                }
            }
            2 => {
                let ctx = ToleranceCtx::unscaled_legacy();
                let (mut curve1, mut curve1_sources) = open.pop().unwrap();
                let (mut curve0, mut curve0_sources) = open.pop().unwrap();
                fn end_pts<T: Copy>(vec: &[T]) -> (T, T) {
                    (vec[0], vec[vec.len() - 1])
                }
                let ((p0, p1), (q0, q1)) = (end_pts(&curve0), end_pts(&curve1));
                if !ctx.is_small_ratio(p0.x - p1.x) && !ctx.is_small_ratio(q0.x - q1.x) {
                    // BG-TOL-001: param
                    if let (Some(urange), _) = range {
                        normalize_range(&mut curve0, &mut curve0_sources, 0, urange);
                        normalize_range(&mut curve1, &mut curve1_sources, 0, urange);
                    }
                } else if !ctx.is_small_ratio(p0.y - p1.y) && !ctx.is_small_ratio(q0.y - q1.y) {
                    // BG-TOL-001: param
                    if let (_, Some(vrange)) = range {
                        normalize_range(&mut curve0, &mut curve0_sources, 1, vrange);
                        normalize_range(&mut curve1, &mut curve1_sources, 1, vrange);
                    }
                }
                let ((p0, p1), (q0, q1)) = (end_pts(&curve0), end_pts(&curve1));
                let vec0 = polyline_on_surface(surface, p1, q0, tol);
                let vec1 = polyline_on_surface(surface, q1, p0, tol);
                let (n0, n1) = (vec0.len(), vec1.len());
                closed.push(BoundaryLoop::chained([
                    (curve0, curve0_sources, SegmentOrigin::Source),
                    (vec0, untagged_sources(n0), SegmentOrigin::SyntheticClosure),
                    (curve1, curve1_sources, SegmentOrigin::Source),
                    (vec1, untagged_sources(n1), SegmentOrigin::SyntheticClosure),
                ]));
            }
            _ => {}
        }
        if probe {
            let areas: Vec<String> = closed
                .iter()
                .map(|c| format!("{:+.4e}", signed_area(&c.points)))
                .collect();
            let range = surface.try_range_tuple();
            let has_rect = matches!(range, (Some(_), Some(_)));
            eprintln!(
                "PROBE in_closed={n_closed_in} in_open={n_open_in} loops={} \
                 areas=[{}] uperiod={:?} vperiod={:?} range={} rect={}",
                closed.len(),
                areas.join(","),
                surface.u_period(),
                surface.v_period(),
                has_rect,
                closed.is_empty() && has_rect,
            );
        }
        // Only a face with no enclosing loop takes its domain from the surface.
        if closed.is_empty() && !had_source_pieces {
            if let (Some((u0, u1)), Some((v0, v1))) = range {
                let p = [
                    (Point2::new(u0, v0), surface.subs(u0, v0)).into(),
                    (Point2::new(u1, v0), surface.subs(u1, v0)).into(),
                    (Point2::new(u1, v1), surface.subs(u1, v1)).into(),
                    (Point2::new(u0, v1), surface.subs(u0, v1)).into(),
                ];
                let vec0 = polyline_on_surface(surface, p[0], p[1], tol);
                let vec1 = polyline_on_surface(surface, p[1], p[2], tol);
                let vec2 = polyline_on_surface(surface, p[2], p[3], tol);
                let vec3 = polyline_on_surface(surface, p[3], p[0], tol);
                let (n0, n1, n2, n3) = (vec0.len(), vec1.len(), vec2.len(), vec3.len());
                closed.push(BoundaryLoop::chained([
                    (vec0, untagged_sources(n0), SegmentOrigin::SyntheticClosure),
                    (vec1, untagged_sources(n1), SegmentOrigin::SyntheticClosure),
                    (vec2, untagged_sources(n2), SegmentOrigin::SyntheticClosure),
                    (vec3, untagged_sources(n3), SegmentOrigin::SyntheticClosure),
                ]));
            }
        }
        (Self(closed), join_outcome)
    }

    /// Where `c` lies relative to the domain bounded by `self`.
    ///
    /// **G7a.** Previously this returned a `bool`, and a ray cast that aborted
    /// was reported as `false` â€” *outside* â€” which is an answer the computation
    /// did not have.
    ///
    /// The two failure modes are separated here rather than inferred from each
    /// other. `Boundary` is decided by a direct point-on-segment predicate, so
    /// it is a positive result about `c`. `Inside` and `Outside` come from ray
    /// casting. `Indeterminate` means every tried ray aborted *and* the direct
    /// predicate did not fire â€” the location is simply not established, and the
    /// type says so instead of naming a side.
    ///
    /// An earlier revision claimed the residue after eight rays *was* boundary
    /// membership. That was an inference from a negative result: an aborted
    /// cast in floating point can equally be near-boundary numerical
    /// degeneracy or an unlucky family of seeds, and "no ray decided" licenses
    /// neither conclusion. Measuring it directly happens to confirm the guess â€”
    /// on ABC `00009190`, 117,145 samples test as `Boundary` and **zero** come
    /// back `Indeterminate`, with triangle and failure counts unchanged â€” but
    /// it is now a positive result that would report `Indeterminate` the moment
    /// that stopped being true, rather than a conclusion drawn from silence.
    fn locate(&self, c: Point2) -> PointLocation {
        // A positive test first, so boundary membership is established rather
        // than inferred from rays failing to decide.
        if self.on_boundary(c) {
            return PointLocation::Boundary;
        }
        // Deterministic, so a face tessellates identically across runs.
        match (0..INCLUSION_RAY_ATTEMPTS).find_map(|attempt| self.include_along_ray(c, attempt)) {
            Some(true) => PointLocation::Inside,
            Some(false) => PointLocation::Outside,
            None => PointLocation::Indeterminate,
        }
    }

    /// Whether `c` lies on a boundary segment, within `TOLERANCE`.
    ///
    /// Direct and ray-independent: the nearest point of each segment is
    /// computed and compared to `c`. This is what entitles [`Self::locate`] to
    /// report `Boundary` as a fact rather than as "the rays gave up".
    fn on_boundary(&self, c: Point2) -> bool {
        let ctx = ToleranceCtx::unscaled_legacy();
        self.0
            .iter()
            .flat_map(|loop_| loop_.points.iter().circular_tuple_windows())
            .any(|(p0, p1)| {
                let (a, b) = (**p0, **p1);
                let ab = b - a;
                let len2 = ab.magnitude2();
                let t = match len2 <= f64::EPSILON {
                    true => 0.0,
                    false => ((c - a).dot(ab) / len2).clamp(0.0, 1.0),
                };
                ctx.is_small_ratio((a + ab * t).distance(c)) // BG-TOL-001: param
            })
    }

    /// One ray cast. `None` when this ray is degenerate against the boundary.
    fn include_along_ray(&self, c: Point2, attempt: u32) -> Option<bool> {
        let ctx = ToleranceCtx::unscaled_legacy();
        // Offsetting the seed per attempt keeps successive rays unrelated
        // rather than merely rotated by a fixed step, which a boundary with
        // regularly spaced vertices could otherwise defeat repeatedly.
        let seed = HashGen::hash1(c) + f64::from(attempt) * std::f64::consts::FRAC_1_PI;
        let t = 2.0 * std::f64::consts::PI * seed.fract();
        let r = Vector2::new(f64::cos(t), f64::sin(t));
        self.0
            .iter()
            .flat_map(|loop_| loop_.points.iter().circular_tuple_windows())
            .try_fold(0_i32, move |crossings, (p0, p1)| {
                let a = **p0 - c;
                let b = **p1 - c;
                let s0 = r.x * a.y - r.y * a.x; // v times a
                let s1 = r.x * b.y - r.y * b.x; // v times b
                let s2 = a.x * b.y - a.y * b.x; // a times b
                let x = s2 / (s1 - s0);
                if ctx.is_small_ratio(x) && s0 * s1 < 0.0 {
                    // BG-TOL-001: param
                    None
                } else if x > 0.0 && ((s0 <= 0.0 && s1 > 0.0) || (s0 >= 0.0 && s1 < 0.0)) {
                    Some(crossings + 1)
                } else {
                    Some(crossings)
                }
            })
            // `None` propagates: a degenerate ray decides nothing, and the
            // caller retries with another direction rather than reading the
            // abort as "outside".
            .map(|crossings| crossings % 2 == 1)
    }

    /// Inserts points and adds constraint
    fn insert_to(
        &self,
        triangulation: &mut Cdt,
        boundary_map: &mut HashMap<FixedVertexHandle, Point3>,
        roles: &mut ConstraintRoles,
        // PLANAR-A A6: per-vertex source-use candidates, fed by the piece's
        // per-segment provenance so `triangulation_into_polymesh_outcome` can
        // fill `VertexMetadata.source_edge_use` where attribution is
        // unambiguous.
        vertex_sources: &mut HashMap<FixedVertexHandle, Vec<SourceEdgeUse>>,
    ) -> std::result::Result<(), TessellationFailureReason> {
        // The first refusal, kept typed. The loop continues after recording it
        // so the probe counters below still see the whole face.
        let mut failure: Option<TessellationFailureReason> = None;
        let probe = std::env::var_os("TRUCK_PROBE_FAIL").is_some();
        let probe_face_id = if probe {
            std::env::var("TRUCK_PROBE_FACE_ID")
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
        } else {
            None
        };
        if let Some(target) = probe_face_id {
            let (source_face_id, declared_face_index, periodic_rank) =
                PROBE_FACE_CONTEXT.with(std::cell::Cell::get);
            if source_face_id == Some(target) {
                for (piece_index, piece) in self.0.iter().enumerate() {
                    for (point_index, point) in piece.points.iter().enumerate() {
                        eprintln!(
                            "WL\tsource_face_id={target}\tdeclared_face_index={declared_face_index}\t\
                             periodic_rank={periodic_rank}\tpost_stitch_piece={piece_index}\t\
                             point_index={point_index}\tuv={:.17e},{:.17e}",
                            point.uv.x, point.uv.y,
                        );
                    }
                }
            }
        }
        let mut probe_point_fail = 0usize;
        let mut probe_degenerate = 0usize;
        // These are post-stitch loop/segment proxies, not source-edge
        // provenance. Keep all direct contributors because duplicates exist.
        let mut installed_origins =
            probe.then(HashMap::<FixedUndirectedEdgeHandle, Vec<(usize, usize)>>::default);
        // DIAG-001: diagnostic capture. Gated on TRUCK_FACE_DIAG_JSONL. When
        // disabled, none of this code has any effect â€” the edge map is never
        // populated and the sink is never written. This instrumentation must
        // not alter insertion order or insertion behaviour.
        let diag = diagnosis::diag_enabled();
        let mut diag_edge_map: HashMap<FixedUndirectedEdgeHandle, u64> = HashMap::default();
        // The first three entries of the CDT stage vector. Counted
        // unconditionally â€” three `usize` increments on a path that already
        // does a triangulation query per segment â€” and emitted only under
        // `diag`.
        let mut stage_boundary_vertices = 0usize;
        let mut stage_constraints_presented = 0usize;
        let mut stage_constraints_inserted = 0usize;
        for (piece_index, piece) in self.0.iter().enumerate() {
            let poly2tri: Vec<Option<FixedVertexHandle>> = piece
                .points
                .iter()
                .map(|pt| {
                    let sp = SPoint2::new(spade_round(pt.uv.x), spade_round(pt.uv.y));
                    if let Some(idx) = triangulation
                        .vertices()
                        .find(|v| sp.distance_2(*v.as_ref()) < 1e-12)
                        .map(|v| v.fix())
                    {
                        boundary_map.insert(idx, pt.point);
                        Some(idx)
                    } else {
                        match triangulation.insert(sp) {
                            Ok(idx) => {
                                boundary_map.insert(idx, pt.point);
                                Some(idx)
                            }
                            Err(_) => None,
                        }
                    }
                })
                .collect();

            if poly2tri.iter().any(|v| v.is_none()) {
                probe_point_fail += 1;
                if diag {
                    diagnosis::set_vertex_insertion_failed();
                    // A failed vertex insertion means no boundary segment of
                    // this piece was ever presented to the CDT, so no segment
                    // pair exists to build a conflict witness from. Record a
                    // `VertexInsertionFailure` witness anyway, so the census
                    // can attribute the face instead of filing it as no
                    // evidence (the `gap == 0`, no-witness faces).
                    //
                    // This deliberately goes to the overlap witness vector,
                    // which `derive_loss_bucket` does not read, so the derived
                    // bucket stays `VertexInsertionFailure` â€” the record is
                    // diagnostic-only.
                    let origin = piece
                        .origins
                        .first()
                        .copied()
                        .unwrap_or(SegmentOrigin::Source);
                    let id = diagnosis::record_segment(origin, Some(piece_index), 0);
                    diagnosis::record_overlap_conflict(
                        id,
                        Some(id),
                        diagnosis::PresentedSegmentRelation::VertexInsertionFailure,
                        None,
                        None,
                    );
                }
                failure.get_or_insert(TessellationFailureReason::ConstraintInsertionIncomplete);
                continue;
            }
            let len = poly2tri.len();
            // Counted after the all-or-nothing point check above, so this is
            // the number of boundary points that actually became vertices.
            stage_boundary_vertices += len;
            // PLANAR-A A6: record the source uses incident at each vertex, for
            // the conservative `VertexMetadata.source_edge_use` attribution.
            // A point is incident to the segments `i` and `i - 1` (cyclic), so
            // a junction vertex shared by two edge uses accumulates both and is
            // left un-attributed downstream.
            for (point_index, maybe_vertex) in poly2tri.iter().enumerate() {
                let Some(idx) = maybe_vertex else { continue };
                let mut point_sources: Vec<SourceEdgeUse> = Vec::new();
                if let Some(sources) = piece.source_uses.get(point_index) {
                    point_sources.extend(sources.iter().copied());
                }
                if let Some(sources) = piece.source_uses.get((point_index + len - 1) % len) {
                    point_sources.extend(sources.iter().copied());
                }
                vertex_sources
                    .entry(*idx)
                    .or_default()
                    .extend(point_sources);
            }
            if len < 3 {
                continue;
            }
            for k in 0..len {
                let i = k;
                let j = (k + 1) % len;
                let vi = poly2tri[i].unwrap();
                let vj = poly2tri[j].unwrap();
                if vi == vj {
                    probe_degenerate += 1;
                    continue;
                }
                // A segment with two distinct endpoints is a real request; the
                // collapsed ones above are not presented to Spade at all.
                stage_constraints_presented += 1;
                // ARR-003: has *this face* already constrained this exact edge?
                //
                // A well-formed loop traverses each edge once. If the direct
                // edge is already a constraint that this face's own role table
                // claims, the boundary is traversing it a second time â€” a
                // duplicate or collinear-overlapping segment. ARR-SEAM W3 admits
                // these as additional traversals (counted, read mod 2 by the
                // flood) instead of refusing the whole face; see the arm below.
                //
                // This arm used to reject the case outright, which also refused
                // segments that were legitimately already fully represented â€”
                // 5 faces on `00009190`. Keeping the overlap witness but not the
                // failure is the separation that makes the census able to name
                // the population without losing the face.
                // G6: the role this segment is entitled to, decided by where
                // the segment came from rather than by which vector it ended up
                // in. `PolyBoundary::new` stitches synthesised closure and seam
                // segments into the same pieces as source trim, so before the
                // origin was recorded at creation every one of them arrived
                // here indistinguishable from a real boundary.
                let segment_origin = piece.origins.get(i).unwrap_or(SegmentOrigin::Source);
                let segment_role = segment_origin.role();
                // PLANAR-A: the source edge uses that contributed this presented
                // segment, carried from `try_new` through the piece. Empty for
                // synthetic segments.
                let segment_sources: Vec<SourceEdgeUse> =
                    piece.source_uses.get(i).cloned().unwrap_or_default();
                // PLANAR-B B3: one semantic identity per presented request. All
                // realized edges of this segment's chain share it.
                let semantic_id = roles.mint_semantic_constraint_id();
                let diag_seg_id = if diag {
                    diagnosis::record_segment(segment_origin, Some(piece_index), k as u32)
                } else {
                    0
                };
                let overlapping = triangulation
                    .get_edge_from_neighbors(vi, vj)
                    .filter(|e| e.is_constraint_edge())
                    .map(|e| e.as_undirected().fix())
                    .filter(|handle| ConstraintRoles::role_of(&triangulation, *handle).is_some());
                if let Some(handle) = overlapping {
                    // ARR-SEAM W3: admit the duplicate traversal instead of
                    // refusing it. The geometry is already present in the CDT
                    // and is valid — the refusal was Truck's own precondition,
                    // not Spade's, and `try_add_constraint` would have returned
                    // this exact edge in its chain. Represent the second
                    // declaration as a traversal count; the parity flood reads
                    // it mod 2, so a doubled edge separates nothing. First
                    // claim wins and no Spade API is called again.
                    //
                    // The diagnostic witness is kept exactly as it was: the
                    // census still needs to see the overlap, it just no longer
                    // implies a terminal failure.
                    if diag {
                        let incoming_a = triangulation.vertex(vi).position();
                        let incoming_b = triangulation.vertex(vj).position();
                        let blocking_edge = triangulation.undirected_edge(handle);
                        let blocking_positions = blocking_edge.positions();
                        let blocking_directed = blocking_edge.as_directed();
                        let blocking_vertices =
                            [blocking_directed.from().fix(), blocking_directed.to().fix()];
                        let relation = classify_presented_relation(
                            [vi, vj],
                            blocking_vertices,
                            incoming_a,
                            incoming_b,
                            blocking_positions[0],
                            blocking_positions[1],
                        );
                        diagnosis::record_overlap_conflict(
                            diag_seg_id,
                            diag_edge_map.get(&handle).copied(),
                            relation,
                            Some(spade_endpoints([incoming_a, incoming_b])),
                            Some(spade_endpoints(blocking_positions)),
                        );
                    }
                    let directed = triangulation.undirected_edge(handle).as_directed().fix();
                    roles.label_realized_chain(
                        triangulation,
                        &[directed],
                        semantic_id,
                        segment_role,
                        &segment_sources,
                        Some(segment_origin),
                    );
                    stage_constraints_inserted += 1;
                    // INV-W3-4: the single-role mod-2 simplification holds only
                    // while every multiplicity>1 edge is still material-toggling
                    // in its role. Read with the `Legacy` reading, which reports
                    // the role's raw property independent of multiplicity.
                    debug_assert!(
                        roles.toggles_material(&triangulation, handle, ParityReading::Legacy)
                            == Some(true),
                        "a multiplicity>1 edge must still be material-toggling for the \
                         single-role mod-2 simplification to hold",
                    );
                    continue;
                }
                // PLANAR-C C2: present the segment with explicit proper-crossing
                // planarization. `insert_with_split` calls Spade's splitting API
                // so a segment that properly crosses an existing constraint is
                // subdivided at the crossing vertex rather than refused: every
                // realized child carries the incoming claim, and every split
                // blocker's children inherit the blocker's payload and traversal
                // count. The returned chain is the authoritative realization and
                // is labelled here; `A -> B` is not assumed to exist as one edge.
                let report = match roles.insert_with_split(
                    triangulation,
                    vi,
                    vj,
                    semantic_id,
                    segment_role,
                    &segment_sources,
                    Some(segment_origin),
                ) {
                    Ok(report) => report,
                    // A crossing network Spade cannot planarize fails closed,
                    // exactly as the pre-PLANAR-C `try_add_constraint` refusal
                    // did. The face stays lost with a typed reason; the rest of
                    // the model is unaffected.
                    Err(reason) => {
                        failure.get_or_insert(reason);
                        continue;
                    }
                };
                stage_constraints_inserted += 1;
                if probe && report.blockers_crossed > 0 {
                    let (source_face_id, declared_face_index, periodic_rank) =
                        PROBE_FACE_CONTEXT.with(std::cell::Cell::get);
                    eprintln!(
                        "SPLIT\tsource_face_id={source_face_id:?}\tdeclared_face_index={declared_face_index}\t\
                         periodic_rank={periodic_rank}\tpost_stitch_piece={piece_index}\t\
                         post_stitch_segment={k}\tchain={}\tblockers_crossed={}\t\
                         blockers_split={}\tblockers_relocated={}\tsplit_vertices={}",
                        report.chain.len(),
                        report.blockers_crossed,
                        report.blockers_split,
                        report.blockers_relocated,
                        report.split_vertices,
                    );
                }
                for directed in &report.chain {
                    let handle = triangulation.directed_edge(*directed).as_undirected().fix();
                    if let Some(installed_origins) = installed_origins.as_mut() {
                        installed_origins
                            .entry(handle)
                            .or_default()
                            .push((piece_index, k));
                    }
                    if diag {
                        diag_edge_map.entry(handle).or_insert(diag_seg_id);
                        diagnosis::record_realized_edge(segment_role, diag_seg_id);
                    }
                }
            }
        }
        if probe && (failure.is_some() || probe_degenerate != 0) {
            eprintln!(
                "PF,{},{probe_point_fail},{probe_degenerate}",
                u8::from(failure.is_none()),
            );
        }
        if diag {
            diagnosis::record_insertion_counts(
                stage_boundary_vertices,
                stage_constraints_presented,
                stage_constraints_inserted,
            );
        }
        match failure {
            Some(reason) => Err(reason),
            None => {
                // PLANAR-C backstop: a boundary vertex inserted on top of an
                // earlier constraint edge can have split it into an unclaimed
                // child; repair those before the flood.
                roles.repair_unlabeled_constraint_edges(triangulation);
                Ok(())
            }
        }
    }
}

fn spade_round(x: f64) -> f64 {
    match f64::abs(x) < MIN_ALLOWED_VALUE {
        true => 0.0,
        false => x,
    }
}

/// The geometric relation between a presented segment and the constraint edge
/// that blocks it, measured on the coordinates Spade actually stores.
///
/// Both segments' endpoints come from the CDT â€” `triangulation.vertex(..)` and
/// `triangulation.undirected_edge(..).positions()` â€” so the two are in the same
/// coordinate space. Raw lifted UV (`piece.points[i].uv`) and Spade's snapped
/// positions differ by up to the 1e-6 vertex snap radius, which is exactly the
/// asymmetry that used to mislabel `no-intersection` witnesses.
///
/// Exact predicates only (`robust`, the same library Spade uses internally):
/// no tolerance, no snap-radius comparison, no grazing classifier. The relation
/// describes what Spade sees. Predicate order is significant:
///
/// 1. same undirected vertex pair -> the same geometric edge traversed again;
/// 2. all four collinear (both blocking endpoints on the incoming line) -> the
///    segments lie on one line and Spade said they conflict, so they overlap;
/// 3. any endpoint exactly on the other segment's supporting line;
/// 4. strict transversal sign crossing;
/// 5. otherwise `Unknown`.
fn classify_presented_relation(
    incoming: [FixedVertexHandle; 2],
    blocking: [FixedVertexHandle; 2],
    a: SPoint2,
    b: SPoint2,
    c: SPoint2,
    d: SPoint2,
) -> diagnosis::PresentedSegmentRelation {
    use diagnosis::PresentedSegmentRelation as R;
    let same_undirected_pair = (incoming[0] == blocking[0] && incoming[1] == blocking[1])
        || (incoming[0] == blocking[1] && incoming[1] == blocking[0]);
    if same_undirected_pair {
        return R::DuplicateTraversal;
    }
    let orient = |p: SPoint2, q: SPoint2, r: SPoint2| {
        robust::orient2d(
            robust::Coord { x: p.x, y: p.y },
            robust::Coord { x: q.x, y: q.y },
            robust::Coord { x: r.x, y: r.y },
        )
    };
    let o1 = orient(a, b, c);
    let o2 = orient(a, b, d);
    if o1 == 0.0 && o2 == 0.0 {
        return R::CollinearOverlap;
    }
    let o3 = orient(c, d, a);
    let o4 = orient(c, d, b);
    if o1 == 0.0 || o2 == 0.0 || o3 == 0.0 || o4 == 0.0 {
        return R::EndpointOnInterior;
    }
    if (o1 < 0.0) != (o2 < 0.0) && (o3 < 0.0) != (o4 < 0.0) {
        return R::ProperInteriorCrossing;
    }
    R::Unknown
}

/// What a Spade constraint edge *means*, so that material semantics can be
/// derived from it.
///
/// **Audit A1.** The dual-parity flood in [`triangulation_into_polymesh_outcome`]
/// asks `edge.is_constraint_edge()` and toggles material parity on every `true`.
/// That is one bit where the formal system requires a role: `insert_surface`
/// adds constraints across the *interior sampling grid*, wholly inside the
/// material region, and those toggle parity exactly as a trim segment does.
///
/// FORMAL_SYSTEM.md Â§IX distinguishes `Physical`, `ArtificialCut`,
/// `NativeBoundary` and `SingularLink`; Definition 20 gives them different
/// material constraints â€” a physical half-edge pins `Î¼_L = 1, Î¼_R = 0` while an
/// artificial cut requires `Î¼_L = Î¼_R`. This enum is that distinction, carried
/// far enough to make the A1 experiment causal.
///
/// It is deliberately *not* the general cell-constraint solver. Parity is still
/// the mechanism; the only thing that changes is which edges are entitled to
/// flip it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub enum ConstraintRole {
    /// A trim segment carrying source boundary evidence. Toggles material side.
    PhysicalBoundary,
    /// A chart cut or seam introduced to make the domain simply connected.
    /// Both sides are the same material state, so it must not toggle.
    ArtificialCut,
    /// The ambient parameter domain's own edge. Does not itself toggle;
    /// its interpretation is schema-dependent (FORMAL_SYSTEM Definition 20).
    NativeBoundary,
    /// An interior grid constraint inserted by [`insert_surface`] to control
    /// triangle shape. Carries no material meaning whatsoever.
    SurfaceSampling,
    /// A closure segment synthesised by [`PolyBoundary::new`] that no source
    /// entity justifies. Its role is genuinely unknown; it must not be silently
    /// assigned equality semantics (audit A6).
    UnresolvedSyntheticClosure,
}

/// The semantic identity of one presented boundary constraint, per face.
///
/// Mints from a per-face counter, one identity per presented semantic
/// constraint request. A request may realize as one CDT edge today, or as N
/// child edges under PLANAR-C; all those realized edges carry the same
/// [`SemanticConstraintId`]. It is deliberately **not** a diagnostic sink index,
/// a Spade edge handle, an array address, or a global counter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SemanticConstraintId(usize);

/// One semantic claim on a realized constraint edge.
///
/// [`Self::source_uses`] is the contributor set from PLANAR-A: the source edge
/// uses whose presented segment this claim realizes. It may be empty for
/// synthetic (seam/closure) and interior-sampling constraints.
#[derive(Clone, Debug)]
pub struct ConstraintClaim {
    /// Which presented semantic constraint this claim belongs to.
    pub semantic_id: SemanticConstraintId,
    /// The material role the segment is entitled to. The **first** claim on an
    /// edge decides its role; later claims are retained for the census.
    pub role: ConstraintRole,
    /// The source edge uses that contributed the presented segment.
    pub source_uses: Vec<SourceEdgeUse>,
}

/// The per-edge payload stored in Spade's `UE` slot.
///
/// The handle IS the identity: an edge that survives a split carries its claims
/// for free (`E0` keeps the payload; `E1` is default-constructed and must be
/// repaired by [`ConstraintRoles::repair_split`]). All realized edges of one
/// semantic constraint share that constraint's [`SemanticConstraintId`].
#[derive(Default, Clone, Debug)]
pub struct ConstraintEdgeData {
    /// Semantic claims on this edge, in attachment order. `role_of` reads the
    /// first; the census reads them all.
    pub claims: Vec<ConstraintClaim>,
}

/// Roles for the constraint edges of one face's triangulation.
///
/// Since ARR-PLANAR W5 the role/provenance source of truth lives in Spade's
/// `UE` slot ([`ConstraintEdgeData`]) rather than in this side table: the old
/// `roles: HashMap<FixedUndirectedEdgeHandle, ConstraintRole>` could not survive
/// CDT mutation, because splitting re-handles or relocates the blocking edge.
/// What remains here is exactly what the `UE` slot cannot hold: traversal
/// multiplicity (a per-handle count, deliberately kept handle-keyed during
/// INFRA), the census counts, the unresolved-at-flood counter, and the
/// per-face semantic-id counter.
#[derive(Debug, Default)]
struct ConstraintRoles {
    /// Constraint edges the flood met that no `record` call had claimed.
    /// Counted, not assumed: this is the size of the gap between what we asked
    /// Spade to constrain and what we can name (CDT-001, CDT-002).
    unresolved_at_flood: std::cell::Cell<usize>,
    /// How many semantic claims each role contributed, for the experiment's
    /// own report. A claim/contributor count: under duplicate traversals (and
    /// split children in PLANAR-C) an edge can carry several claims.
    recorded: HashMap<ConstraintRole, usize>,
    /// How many semantic claims each *origin* contributed. Distinct from
    /// `recorded`: several origins deliberately share one role while the
    /// material semantics of synthesised geometry stay unchanged, so without
    /// this the synthetic populations are indistinguishable in the census.
    origin_census: HashMap<SegmentOrigin, usize>,
    /// How many times the face's boundary traversed each constraint edge.
    ///
    /// Material parity is the boundary's winding number mod 2, so a
    /// twice-traversed edge must contribute *nothing* — counting is the only
    /// way to know that it should. In INFRA this map stays handle-keyed; the
    /// split contract (B7) propagates counts explicitly when a split happens.
    traversals: HashMap<FixedUndirectedEdgeHandle, usize>,
    /// The per-face counter minting [`SemanticConstraintId`]s, in presentation
    /// order. Deterministic because it is a plain local counter.
    next_semantic_id: usize,
}

/// Which reading of "this constraint edge separates material" the parity flood
/// is using.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ParityReading {
    /// Every edge a boundary segment realized toggles, exactly once â€” the set
    /// reading, which is what the flood has always used.
    Legacy,
    /// An edge toggles only if the boundary traversed it an *odd* number of
    /// times. Two traversals cancel mod 2 whether they run the same way (a
    /// doubled segment) or opposite ways (a slit), and in both cases the edge
    /// genuinely separates nothing.
    TraversalParity,
}

impl ConstraintRoles {
    /// Mint the next per-face semantic constraint identity, in presentation
    /// order. Deterministic: a plain local counter.
    fn mint_semantic_constraint_id(&mut self) -> SemanticConstraintId {
        let id = SemanticConstraintId(self.next_semantic_id);
        self.next_semantic_id += 1;
        id
    }

    /// The first semantic claim's role on an edge, read from the CDT's `UE`
    /// payload.
    ///
    /// First claim wins: a later, weaker claim must not overwrite a physical
    /// boundary — an interior grid segment that happens to land on an edge a
    /// trim segment already constrained is still a trim segment.
    fn role_of(cdt: &Cdt, edge: FixedUndirectedEdgeHandle) -> Option<ConstraintRole> {
        cdt.undirected_edge(edge)
            .data()
            .data()
            .claims
            .first()
            .map(|claim| claim.role)
    }

    /// Whether a constraint edge is entitled to flip material parity.
    ///
    /// `None` means the edge carries **no resolvable role**, which is not a
    /// material category and must not be answered with one.
    ///
    /// **G5b: fail closed.** This previously returned `true` for an unresolved
    /// edge â€” an unjustified material assertion. Answering `false` instead
    /// would have been the same mistake facing the other way: both invent a
    /// semantics for an edge the code cannot name. Since G5a labels the whole
    /// realized chain, every constraint edge this face requested has a role, and
    /// an unresolved one is an invariant violation rather than a legitimate
    /// category â€” so it is reported, not guessed.
    ///
    /// Measured after G5a: zero occurrences on ABC `00009190`, so this guard
    /// lands provably non-firing.
    ///
    /// Under [`ParityReading::TraversalParity`] a role that would toggle still
    /// only does so if the boundary crossed the edge an odd number of times;
    /// see [`Self::traversals`].
    fn toggles_material(
        &self,
        cdt: &Cdt,
        edge: FixedUndirectedEdgeHandle,
        reading: ParityReading,
    ) -> Option<bool> {
        let toggles = match Self::role_of(cdt, edge) {
            Some(ConstraintRole::SurfaceSampling) => Some(false),
            Some(ConstraintRole::ArtificialCut) => Some(false),
            Some(ConstraintRole::PhysicalBoundary) => Some(true),
            // FORMAL_SYSTEM Definition 20 says a native ambient boundary does
            // not itself toggle; its interpretation comes from incident
            // physical constraints. This case is never constructed today, so it
            // keeps its legacy answer and is decided when G6 first builds it.
            Some(ConstraintRole::NativeBoundary) => Some(true),
            // Definition 20's second bullet says an artificial cut generates
            // Î¼_L = Î¼_R, which would make this `false`. **Measured, and it
            // recovers nothing**: read as non-toggling, all 126 contradicting
            // faces on `00009190` still contradict, and the odd-vertex count
            // rises rather than falls. The synthesised segments sit inside a
            // closed cycle; the contradiction is elsewhere. Left at the legacy
            // answer rather than changed on the strength of the definition
            // alone, because the experiment says the definition is not what
            // this cell is about.
            Some(ConstraintRole::UnresolvedSyntheticClosure) => Some(true),
            None => {
                self.unresolved_at_flood
                    .set(self.unresolved_at_flood.get() + 1);
                None
            }
        };
        match (toggles, reading) {
            (Some(true), ParityReading::TraversalParity) => {
                // Absent from `traversals` means the edge was never traversed
                // by a boundary segment at all, which for a toggling role can
                // only be a bookkeeping gap â€” read it as one traversal, so this
                // reading is never *weaker* than the legacy one by accident.
                let crossings = self.traversals.get(&edge).copied().unwrap_or(1);
                Some(crossings % 2 == 1)
            }
            _ => toggles,
        }
    }

    /// Label a realized constraint chain with one semantic claim, mechanically.
    ///
    /// For every undirected edge of `chain` (in Spade's realized order) this
    /// attaches the given claim to the edge's `UE` payload, increments the
    /// traversal count exactly as the pre-W5 code did, and updates the role and
    /// origin censuses. First-role semantics come from `claims.first()`; the
    /// claim's identity is retained for the census. `origin` is `None` for
    /// interior-sampling constraints, which have no [`SegmentOrigin`].
    fn label_realized_chain(
        &mut self,
        cdt: &mut Cdt,
        chain: &[FixedDirectedEdgeHandle],
        semantic_id: SemanticConstraintId,
        role: ConstraintRole,
        source_uses: &[SourceEdgeUse],
        origin: Option<SegmentOrigin>,
    ) {
        for directed in chain {
            let handle = cdt.directed_edge(*directed).as_undirected().fix();
            let claim = ConstraintClaim {
                semantic_id,
                role,
                source_uses: source_uses.to_vec(),
            };
            cdt.undirected_edge_data_mut(handle)
                .data_mut()
                .claims
                .push(claim);
            *self.traversals.entry(handle).or_insert(0) += 1;
            *self.recorded.entry(role).or_insert(0) += 1;
            if let Some(origin) = origin {
                *self.origin_census.entry(origin).or_insert(0) += 1;
            }
        }
    }

    /// B7 split-repair primitive: propagate a parent edge's claim data onto its
    /// two split children.
    ///
    /// Spade 2.15.1 splits a blocking constraint `E` into `[E0, E1]` where `E0`
    /// is the original handle (it keeps the `UE` payload verbatim) and `E1` is
    /// a fresh default-constructed [`ConstraintEdgeData`]. Spade never clones
    /// the payload, so the child that kept its handle is already correct and
    /// only `E1` needs the parent's claims copied.
    ///
    /// Traversal rule: if the parent was traversed `t` times, every full-chord
    /// traversal now crosses both children, so `E0.traversals = t` and
    /// `E1.traversals = t`. This preserves mod-2 material parity per child.
    ///
    /// Not wired into the production path in INFRA; PLANAR-C calls it after each
    /// split.
    fn repair_split(
        &mut self,
        cdt: &mut Cdt,
        parent: FixedUndirectedEdgeHandle,
        child0: FixedUndirectedEdgeHandle,
        child1: FixedUndirectedEdgeHandle,
    ) {
        let parent_claims = cdt.undirected_edge_data_mut(parent).data().claims.clone();
        let parent_traversals = self.traversals.get(&parent).copied().unwrap_or(0);
        cdt.undirected_edge_data_mut(child1)
            .data_mut()
            .claims
            .extend(parent_claims);
        *self.traversals.entry(child0).or_insert(0) = parent_traversals;
        *self.traversals.entry(child1).or_insert(0) = parent_traversals;
    }

    /// B8 fallback-relocation repair primitive: move a lost edge's semantic
    /// claims onto its replacement edges.
    ///
    /// Spade's fallback transforms an old constraint `E` into a detour
    /// `prev + next` by unmaking `E` and making `prev`/`next`. The `UE` payloads
    /// themselves do not move, so the repair contract is explicit: snapshot the
    /// lost edge's claims and traversal count before the mutation, then append
    /// the claims onto each replacement edge (preserving any legitimate
    /// pre-existing claims there) and combine the traversal counts — each
    /// replacement edge now represents the same semantic traversals the lost
    /// edge did, so `replacement.traversals += lost.traversals` when both
    /// exist. First-role behavior stays deterministic because it is decided by
    /// claim order within each edge.
    ///
    /// Not wired into the production path in INFRA; PLANAR-C calls it from a
    /// before/after constraint-set snapshot. `lost` is the handle whose edge
    /// stopped being a constraint; `replacements` are the handles that gained
    /// the constraint bit in the same face.
    fn repair_relocation(
        &mut self,
        cdt: &mut Cdt,
        lost: FixedUndirectedEdgeHandle,
        replacements: &[FixedDirectedEdgeHandle],
    ) {
        let lost_claims = cdt.undirected_edge_data_mut(lost).data().claims.clone();
        let lost_traversals = self.traversals.get(&lost).copied().unwrap_or(0);
        for directed in replacements {
            let handle = cdt.directed_edge(*directed).as_undirected().fix();
            cdt.undirected_edge_data_mut(handle)
                .data_mut()
                .claims
                .extend(lost_claims.clone());
            *self.traversals.entry(handle).or_insert(0) += lost_traversals;
        }
    }

    /// PLANAR-C: present one boundary segment to the CDT with explicit
    /// proper-crossing planarization.
    ///
    /// This is the production replacement for the `try_add_constraint` refusal
    /// route. Spade's `add_constraint_and_split` subdivides the incoming
    /// segment at every proper interior crossing and splits the blocker edges
    /// it crosses, so `A --- B` crossing `C --- D` becomes four atomic
    /// constrained edges meeting at the new vertex `X`. In one call this:
    ///
    /// 1. snapshots the blockers the segment properly crosses;
    /// 2. calls `add_constraint_and_split` (new crossing vertices are rounded
    ///    exactly as boundary vertices are, so the vertex snaps like a
    ///    presented boundary point and gets `surface.subs` 3D positioning);
    /// 3. labels the authoritative returned chain with the incoming claim,
    ///    role, source uses, and one traversal per child
    ///    ([`Self::label_realized_chain`]);
    /// 4. repairs every split blocker via [`Self::repair_split`] (children
    ///    inherit the complete parent payload and traversal count) and every
    ///    fallback relocation via [`Self::repair_relocation`].
    ///
    /// The returned [`CrossingSplitReport`] carries the authoritative realized
    /// chain plus the census counters; the caller must not assume `vi -> vj`
    /// exists as one edge afterwards.
    fn insert_with_split(
        &mut self,
        cdt: &mut Cdt,
        vi: FixedVertexHandle,
        vj: FixedVertexHandle,
        semantic_id: SemanticConstraintId,
        role: ConstraintRole,
        source_uses: &[SourceEdgeUse],
        origin: Option<SegmentOrigin>,
    ) -> std::result::Result<CrossingSplitReport, TessellationFailureReason> {
        // The blockers this segment properly crosses, in Spade's walk order,
        // with each blocker's original endpoints recorded *before* the split.
        // Only these edges can be split, so the full constraint-set snapshot
        // below is skipped in the common no-crossing case.
        let blockers: Vec<(FixedUndirectedEdgeHandle, [FixedVertexHandle; 2])> = cdt
            .get_conflicting_edges_between_vertices(vi, vj)
            .map(|edge| {
                let handle = edge.as_undirected().fix();
                (handle, edge_vertices(cdt, handle))
            })
            .collect();
        let before_set = (!blockers.is_empty()).then(|| {
            let mut set = HashSet::default();
            for edge in cdt.undirected_edges() {
                if edge.is_constraint_edge() {
                    set.insert(edge.fix());
                }
            }
            set
        });
        let vertices_before = cdt.num_vertices();

        // Split. The vertex constructor receives Spade's computed intersection
        // point and rounds it with the same convention as boundary vertices, so
        // the inserted vertex participates in the CDT on an equal footing.
        //
        // Spade's splitting API has two internal assertions that can fire on a
        // dense crossing network: after inserting a split vertex it re-presents
        // `from -> final_vertex` and panics if that sub-segment still properly
        // crosses another constraint, and its split-position location guards
        // against an infinite loop with a panic. Both are aborts inside the
        // call, so this is caught here and the face fails closed with the same
        // refusal the old `try_add_constraint` route produced — a face whose
        // crossing network Spade cannot planarize is a typed failure, never a
        // model abort. The per-face `Cdt` is abandoned on the failure path, so
        // the partially-mutated triangulation is never read.
        let chain = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            cdt.add_constraint_and_split(vi, vj, |pt: spade::Point2<f64>| {
                SPoint2::new(spade_round(pt.x), spade_round(pt.y))
            })
        })) {
            Ok(chain) => chain,
            Err(_) => return Err(TessellationFailureReason::ConstraintInsertionIncomplete),
        };
        let split_vertices = cdt.num_vertices() - vertices_before;

        // The incoming claim, role, source uses, and traversal land on every
        // realized child; the returned chain is authoritative.
        self.label_realized_chain(cdt, &chain, semantic_id, role, source_uses, origin);

        let mut blockers_split = 0usize;
        let mut blockers_relocated = 0usize;
        if let Some(before_set) = before_set {
            let chain_set: HashSet<FixedUndirectedEdgeHandle> = chain
                .iter()
                .map(|directed| cdt.directed_edge(*directed).as_undirected().fix())
                .collect();
            // Newly created constraint edges: not present before this call and
            // not part of the incoming chain, so each is either a blocker's
            // split child or a relocation replacement.
            let mut new_constraints: Vec<FixedUndirectedEdgeHandle> = Vec::new();
            for edge in cdt.undirected_edges() {
                if !edge.is_constraint_edge() {
                    continue;
                }
                let handle = edge.fix();
                if before_set.contains(&handle) || chain_set.contains(&handle) {
                    continue;
                }
                new_constraints.push(handle);
            }
            for &(blocker, blocker_original) in &blockers {
                if !cdt.is_constraint_edge(blocker) {
                    // Fallback relocation: Spade un-made the blocker and routed
                    // the constraint through the two other sides of the face.
                    // The blocker handle survives (unconstrained) with its
                    // payload; move that payload onto the detour edges, which
                    // share exactly one endpoint with it.
                    let blocker_vertices = edge_vertices(cdt, blocker);
                    let replacements: Vec<FixedDirectedEdgeHandle> = new_constraints
                        .iter()
                        .filter(|handle| {
                            shares_exactly_one_endpoint(
                                blocker_vertices,
                                edge_vertices(cdt, **handle),
                            )
                        })
                        .map(|handle| cdt.undirected_edge(*handle).as_directed().fix())
                        .collect();
                    if !replacements.is_empty() {
                        self.repair_relocation(cdt, blocker, &replacements);
                        blockers_relocated += 1;
                    }
                } else {
                    // Split: the blocker kept its handle (`E0`) with its payload
                    // intact; the new child `E1` shares the crossing vertex with
                    // it, and the child plus the surviving half jointly span the
                    // blocker's *original* endpoints. This is an exact vertex-
                    // handle test: float collinearity cannot decide it, because
                    // the two halves of one straight edge are collinear in real
                    // arithmetic but differ by rounding in an exact predicate.
                    let blocker_vertices = edge_vertices(cdt, blocker);
                    let children: Vec<FixedUndirectedEdgeHandle> = new_constraints
                        .iter()
                        .copied()
                        .filter(|handle| {
                            let candidate = edge_vertices(cdt, *handle);
                            shares_exactly_one_endpoint(blocker_vertices, candidate)
                                && spans_original_endpoints(
                                    blocker_vertices,
                                    candidate,
                                    blocker_original,
                                )
                        })
                        .collect();
                    for child in children {
                        self.repair_split(cdt, blocker, blocker, child);
                        blockers_split += 1;
                    }
                }
            }
        }

        Ok(CrossingSplitReport {
            chain,
            blockers_crossed: blockers.len(),
            blockers_split,
            blockers_relocated,
            split_vertices,
        })
    }

    /// Backstop repair: no constraint edge may reach the parity flood without a
    /// claim, or the face fails closed with `ConstraintRoleMissing`.
    ///
    /// [`Self::insert_with_split`] repairs every blocker its own call splits,
    /// but Spade can create an unclaimed constraint edge in two places that
    /// call cannot see: when a vertex inserted by [`PolyBoundary::insert_to`]
    /// or [`insert_surface`] lands exactly on an existing constraint edge, the
    /// edge is split and the fresh child is `UE::default()`. This sweeps the
    /// whole CDT and attaches the claim of the labeled edge each unclaimed
    /// child continues: the surviving half shares the split vertex with it and
    /// lies on the same line.
    ///
    /// The same-line test here is a semantic-matching tolerance, not a
    /// tessellation tolerance: the two halves of one straight constraint are
    /// collinear in real arithmetic but differ by rounding in an exact
    /// predicate, so an exact orientation test cannot recognize them.
    fn repair_unlabeled_constraint_edges(&mut self, cdt: &mut Cdt) {
        // Iterate to fixpoint. One sweep cannot label a *chain* of split
        // children: an unlabeled child whose own parent is also unlabeled (the
        // same trim edge split at several grid/trim intersections) finds no
        // labeled parent within the same pass, so it must wait for its parent
        // to be labeled by the next pass. When a pass repairs nothing, the
        // remainder has no legitimate parent and stays unlabeled — the flood
        // will fail closed with `ConstraintRoleMissing` rather than invent a
        // role.
        loop {
            let unlabeled: Vec<FixedUndirectedEdgeHandle> = cdt
                .undirected_edges()
                .filter(|edge| edge.is_constraint_edge())
                .filter(|edge| edge.data().data().claims.is_empty())
                .map(|edge| edge.fix())
                .collect();
            if unlabeled.is_empty() {
                break;
            }
            let mut repaired = false;
            for handle in unlabeled {
                let candidate = edge_vertices(cdt, handle);
                let candidate_positions = cdt.undirected_edge(handle).positions();
                let mut parents: Vec<FixedUndirectedEdgeHandle> = Vec::new();
                for edge in cdt.undirected_edges() {
                    if !edge.is_constraint_edge() || edge.fix() == handle {
                        continue;
                    }
                    if edge.data().data().claims.is_empty() {
                        continue;
                    }
                    let parent = edge_vertices(cdt, edge.fix());
                    if !shares_exactly_one_endpoint(parent, candidate) {
                        continue;
                    }
                    if same_line(
                        candidate_positions,
                        cdt.undirected_edge(edge.fix()).positions(),
                    ) {
                        parents.push(edge.fix());
                    }
                }
                let Some(&parent) = parents.first() else {
                    continue;
                };
                self.repair_split(cdt, parent, parent, handle);
                repaired = true;
            }
            if !repaired {
                break;
            }
        }
    }
}

/// Whether two segments lie on one supporting line, within the float rounding
/// of a real collinearity.
///
/// This is a matching predicate for split-children repair only; it is not used
/// for any geometry decision and cannot change the tessellation.
fn same_line(a: [SPoint2; 2], b: [SPoint2; 2]) -> bool {
    let cross =
        |p: SPoint2, q: SPoint2, r: SPoint2| (q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x);
    let len2 = (a[1].x - a[0].x) * (a[1].x - a[0].x) + (a[1].y - a[0].y) * (a[1].y - a[0].y);
    let slack = 1e-9 * len2.max(1e-12);
    cross(a[0], a[1], b[0]).abs() <= slack && cross(a[0], a[1], b[1]).abs() <= slack
}

/// What one [`ConstraintRoles::insert_with_split`] call did to the CDT.
///
/// The realized chain is authoritative: the caller must label and iterate the
/// returned edges, never a presumed single `vi -> vj` edge. The counters feed
/// the probe census so a run can answer how much of the residual tail is
/// proper-crossing planarization.
#[derive(Debug, Default)]
pub(crate) struct CrossingSplitReport {
    /// The realized incoming chain, in Spade's order. Always non-empty for a
    /// distinct `vi != vj`.
    pub chain: Vec<FixedDirectedEdgeHandle>,
    /// Constraint edges the segment properly crossed.
    pub blockers_crossed: usize,
    /// Crossed blockers repaired through [`ConstraintRoles::repair_split`].
    pub blockers_split: usize,
    /// Crossed blockers repaired through [`ConstraintRoles::repair_relocation`].
    pub blockers_relocated: usize,
    /// New vertices introduced at crossing points.
    pub split_vertices: usize,
}

/// The two fixed vertex handles of an undirected edge, for endpoint matching
/// after a split without relying on float equality.
fn edge_vertices(cdt: &Cdt, handle: FixedUndirectedEdgeHandle) -> [FixedVertexHandle; 2] {
    let directed = cdt.undirected_edge(handle).as_directed();
    [directed.from().fix(), directed.to().fix()]
}

/// Whether `candidate` shares exactly one endpoint with `anchor`.
///
/// Vertex-handle identity, not coordinate equality: the crossing vertex a split
/// child shares with its parent is one fixed vertex, so handle comparison is
/// exact where a float comparison would be fragile.
fn shares_exactly_one_endpoint(
    anchor: [FixedVertexHandle; 2],
    candidate: [FixedVertexHandle; 2],
) -> bool {
    let shared = usize::from(anchor[0] == candidate[0] || anchor[0] == candidate[1])
        + usize::from(anchor[1] == candidate[0] || anchor[1] == candidate[1]);
    shared == 1
}

/// Whether a blocker's surviving half plus a candidate child jointly span the
/// blocker's original endpoints.
///
/// After Spade splits a blocker at the crossing vertex `X`, the surviving half
/// keeps its handle (spans `C-X` or `X-D`) and the new child spans the other
/// half. Their union is exactly the blocker's three distinct vertices `{C, X,
/// D}`. This is the exact, float-free test that a candidate is genuinely this
/// blocker's split child: it is not enough to share the crossing vertex, and
/// exact collinearity cannot decide it, because the two halves of one straight
/// edge are collinear in real arithmetic but differ by rounding in an exact
/// orientation predicate.
fn spans_original_endpoints(
    surviving_half: [FixedVertexHandle; 2],
    candidate: [FixedVertexHandle; 2],
    original: [FixedVertexHandle; 2],
) -> bool {
    let mut covered = surviving_half.to_vec();
    covered.extend(candidate);
    for &vertex in &original {
        if !covered.contains(&vertex) {
            return false;
        }
    }
    let mut distinct = covered;
    distinct.sort();
    distinct.dedup();
    distinct.len() == 3
}

/// Why one face could not be tessellated.
///
/// Seven of these variants are declared but never constructed, and are marked
/// as such below. They are retained rather than deleted because each names a
/// stage the formal system requires and this implementation does not yet have â€”
/// deleting them would erase the fact that the case is unhandled, which is the
/// opposite of what a typed outcome is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub enum TessellationFailureReason {
    /// No lifted boundary could be built, for a reason the lift does not name.
    ///
    /// Retained as the residual bucket now that the lift reports its own
    /// causes. A face arriving here means a refusal path was added without a
    /// reason to go with it.
    BoundaryConstructionFailed,
    /// A bound contributed no points at all, so it cannot bound anything.
    BoundaryWireEmpty,
    /// A boundary edge's source traversal could not be established, so the
    /// face has no renderable boundary.
    ///
    /// Distinct from an invalid edge: the curve and the source vertices are
    /// not known to contradict, only that no traversal is *certified*. The
    /// edge is deliberately not sampled over its evaluator domain, because a
    /// closed source crescent sampled as a whole loop would re-emit the
    /// malformed boundary this outcome exists to refuse.
    EdgeTraversalUnresolved,
    /// A boundary sample had no parameter on the face's own surface.
    BoundaryProjectionFailed,
    /// A boundary sample lay further from its own surface than the
    /// compatibility policy permits (GEO-005). Only reachable when
    /// `TRUCK_COMPAT_FACTOR` is set; the gate is off by default.
    BoundaryPointOffSurface,
    /// The periodic branch of a lift step could not be resolved.
    ///
    /// `get_mindiff` picks the period copy nearest the previous sample, which
    /// is correct only while the true step is under half a period. Beyond
    /// `AMBIGUOUS_STEP_FRACTION` the two candidates are not distinguishable
    /// that way, and the step is bisected to shorten it. When bisection is
    /// exhausted the branch is genuinely unresolved: the code previously
    /// accepted the ambiguous value silently, which is how a period-wrapping
    /// boundary could fold onto itself and read as closed (FS Def. 14).
    AmbiguousLift,
    /// The parity flood assigned a cell two different labels around a cycle.
    ///
    /// This is a *proved* inconsistency, not a heuristic giving up: the
    /// boundary as realized cannot carry a coherent material assignment.
    ContradictoryDualParity,
    /// The flood completed but selected no material cell, so there is nothing
    /// to mesh.
    NoOddParityRegion,
    /// The face was certified intrinsically degenerate by FACE-VALIDITY
    /// (Detector B) and rejected before tessellation. The geometric evidence
    /// travels in the face's diagnosis `validity_certificate`; this variant
    /// carries no payload so the reason enum stays `Copy`.
    ///
    /// A rejected face is not a rendered face and not a generic tessellation
    /// failure: the census must classify it as `rejected_intrinsic`, never as
    /// a mesh or as an unexplained loss.
    RejectedDegenerate,
    /// The face was certified singular-ambiguous: the oriented incident source
    /// geometry admits two *distinct source-consistent* continuations, and the
    /// certificate carries positive evidence for both. This is the only
    /// `Ambiguous` classification a face may carry: an ordinary
    /// [`Self::AmbiguousLift`] is a tessellation outcome, not an ambiguity
    /// certificate.
    ///
    /// **No production mechanism currently emits this.** The P2 singular
    /// transition analysis never constructs a second continuation: its
    /// negative evidence ("this mechanism could not select a continuation")
    /// proves nothing about the source, so it leaves the lift unresolved
    /// (`AmbiguousLift`). The variant is retained as the target type for a
    /// future certificate that *does* build two continuations. Like
    /// [`Self::RejectedDegenerate`] this is a terminal state no recovery route
    /// may touch; the certificate's supporting evidence travels in the face
    /// diagnosis.
    RejectedAmbiguous,
    /// A constraint chain did not close. **Never constructed.**
    ConstraintChainNotClosed,
    /// At least one boundary segment could not be represented as a constraint.
    ///
    /// Almost always a proper crossing of an earlier segment of this same
    /// face's boundary â€” which is what a folded lift produces.
    ConstraintInsertionIncomplete,
    /// A certified intersection was found that the envelope does not admit.
    /// **Never constructed** â€” no intersection classification stage exists yet.
    ConstraintIntersectionUnsupported,
    /// A collinear overlap was found that the envelope does not admit.
    /// **Never constructed** â€” no overlap normalization stage exists yet.
    ConstraintOverlapUnsupported,
    /// The triangulation could not be built at all. **Never constructed.**
    CdtConstructionFailed,
    /// A vertex evaluated to a non-finite 3D position.
    NonFinitePosition,
    /// A constraint edge carried no resolvable role. **Never constructed** â€”
    /// today an unresolved role silently keeps its legacy toggling behaviour.
    ConstraintRoleMissing,
    /// A constraint chain degenerated to a point. **Never constructed.**
    DegenerateConstraintChain,
    /// Parity selected cells but none yielded a finite triangle.
    /// **Never constructed** â€” the empty case reports [`Self::NoOddParityRegion`].
    NoFiniteTrianglesAfterParity,
}

/// A terminal failure of one face, carrying its finalized diagnostic record.
///
/// The single terminal representation of a face that did not render. The
/// `diagnostic` field is the production contract: a bare reason cannot become a
/// terminal failure without finalizing a [`FaceDiagnosticRecord`], so a future
/// refusal path cannot bypass diagnostic emission by forgetting to record.
///
/// Constructed exclusively through `diagnosis::fail` / `diagnosis::reject`.
#[derive(Clone, Debug)]
pub struct TessellationFailure {
    /// What went wrong.
    pub reason: TessellationFailureReason,
    /// The finalized machine-readable diagnostic record for this face.
    pub diagnostic: diagnosis::FaceDiagnosticRecord,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VertexGeneration {
    SurfaceEvaluation,
    SourceEdgeSample,
    ConstraintIntersection,
    SingularRealization,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct VertexRoles(pub u16);

impl VertexRoles {
    pub const PHYSICAL_BOUNDARY: u16 = 1 << 0;
    pub const ARTIFICIAL_SEAM: u16 = 1 << 1;
    pub const SINGULAR_COLLAPSE: u16 = 1 << 2;
    pub const CONSTRAINT_INTERSECT: u16 = 1 << 3;

    pub fn contains(&self, role: u16) -> bool {
        (self.0 & role) == role
    }
    pub fn insert(&mut self, role: u16) {
        self.0 |= role;
    }
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }
}

#[derive(Clone, Debug)]
pub struct SeamPair {
    pub first_chain: Vec<usize>,
    pub second_chain: Vec<usize>,
    pub correspondence: Vec<(usize, usize)>,
    pub orientation_reversed: bool,
    pub deck_displacement: [i64; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SingularKind {
    Apex,
    Pole,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SingularLinkKind {
    InteriorCycle,
    BoundaryInterval,
}

#[derive(Clone, Debug)]
pub struct SingularGroup {
    pub vertices: Vec<usize>,
    pub canonical_point: Point3,
    pub kind: SingularKind,
    pub link_kind: SingularLinkKind,
}

#[derive(Clone, Debug)]
pub struct VertexMetadata {
    pub uv: Point2,
    pub generation: VertexGeneration,
    pub roles: VertexRoles,
    pub source_edge_use: Option<usize>,
    pub seam_pair: Option<usize>,
    pub singular_group: Option<usize>,
}

#[derive(Clone, Debug, Default)]
pub struct TessellationDiagnostics {
    pub vertex_metadata: Vec<VertexMetadata>,
    pub seam_pairs: Vec<SeamPair>,
    pub singular_groups: Vec<SingularGroup>,
}

#[derive(Clone, Debug)]
pub struct FaceTessellation {
    pub mesh: PolygonMesh,
    pub diagnostics: TessellationDiagnostics,
}

#[derive(Clone, Debug)]
pub enum TessellationOutcome {
    Mesh(FaceTessellation),
    Failed(TessellationFailure),
}

/// Tessellates one surface trimmed by polyline, returning `TessellationOutcome`.
fn trimming_tessellation_with_diagnostics<S>(
    surface: &S,
    polyboundary: &PolyBoundary,
    tol: f64,
    lattice: &CertifiedLattice,
) -> TessellationOutcome
where
    S: PreMeshableSurface,
{
    let refine = std::env::var_os("TRUCK_SGC_REFINE").is_some();
    trimming_tessellation_with_refinement(surface, polyboundary, tol, lattice, refine)
}

/// The shared refinement body. `enable_refine` selects the CDT-aware refinement
/// path; tests drive it directly with an explicit flag so the acceptance rule
/// is exercised deterministically and in parallel, while production reads the
/// env gate in [`trimming_tessellation_with_diagnostics`].
fn trimming_tessellation_with_refinement<S>(
    surface: &S,
    polyboundary: &PolyBoundary,
    tol: f64,
    lattice: &CertifiedLattice,
    refine_enabled: bool,
) -> TessellationOutcome
where
    S: PreMeshableSurface,
{
    // CDT-aware refinement driven by the *maximum* sampled exact-surface
    // deviation E(mesh) = max over material triangles of the surface-vs-flat
    // deviation measured with exact S(uv) triangle corners.
    //
    // Acceptance theorem: a candidate mesh is retained only when it strictly
    // reduces E below the current best by a small numerical-progress tolerance;
    // refinement terminates as soon as E <= tolerance. `excess_sum`,
    // `unsafe_count` and triangle count are diagnostics only — they never
    // authorize a mesh. With `refine_enabled` false this is the ordinary single
    // pass, byte-identical to the pre-refinement behavior.
    const MAX_REFINE_PASSES: usize = 8;
    // Numerical progress epsilon, not a geometric tuning parameter: only guards
    // against accepting a pass whose max deviation did not actually move.
    const E_PROGRESS: f64 = 1e-9;
    let mut support_uvs: Vec<Point2> = Vec::new();
    let mut best_outcome: Option<TessellationOutcome> = None;
    let mut best_max_dev = f64::INFINITY;
    let mut best_supports = 0usize;
    let mut best_tris = 0usize;
    let mut termination: &'static str = "max_passes";
    for pass in 0..MAX_REFINE_PASSES {
        let mut triangulation = Cdt::new();
        let mut boundary_map = HashMap::<FixedVertexHandle, Point3>::default();
        let mut vertex_sources = HashMap::<FixedVertexHandle, Vec<SourceEdgeUse>>::default();
        let mut roles = ConstraintRoles::default();
        if let Err(reason) = polyboundary.insert_to(
            &mut triangulation,
            &mut boundary_map,
            &mut roles,
            &mut vertex_sources,
        ) {
            return TessellationOutcome::Failed(diagnosis::fail(
                reason,
                diagnosis::failure_stage_for_reason(reason),
            ));
        }
        REFINE_SUPPORT_CELL.with(|cell| cell.borrow_mut().clear());
        REFINE_TRAJECTORY.with(|cell| cell.borrow_mut().clear());
        let (samples_on_boundary, sampling_location_unresolved) = insert_surface(
            &mut triangulation,
            surface,
            polyboundary,
            tol,
            &mut roles,
            &support_uvs,
        );
        let outcome = triangulation_into_polymesh_outcome(
            &triangulation,
            surface,
            polyboundary,
            &boundary_map,
            &roles,
            &vertex_sources,
            lattice,
            tol,
            refine_enabled,
        );
        if !refine_enabled {
            if std::env::var_os("TRUCK_PROBE_ROLES").is_some() {
                // The population sizes the A1 comparison rests on. `unresolved`
                // is the honest gap: constraint edges the flood met that no
                // `record` call had claimed, which keep their legacy toggling
                // behaviour. A large number here would mean the experiment is
                // less causal than it looks.
                let count = |role| roles.recorded.get(&role).copied().unwrap_or(0);
                let by_origin = |o| roles.origin_census.get(&o).copied().unwrap_or(0);
                eprintln!(
                    "ROLES\tphysical={}\tsampling={}\tartificial={}\tnative={}\t\
                     unresolved_synth={}\tunresolved_at_flood={}\t\
                     samples_on_boundary={}\tsampling_location_unresolved={}\t\
                     origin_source={}\torigin_synthetic={}\torigin_seam={}",
                    count(ConstraintRole::PhysicalBoundary),
                    count(ConstraintRole::SurfaceSampling),
                    count(ConstraintRole::ArtificialCut),
                    count(ConstraintRole::NativeBoundary),
                    count(ConstraintRole::UnresolvedSyntheticClosure),
                    roles.unresolved_at_flood.get(),
                    samples_on_boundary,
                    sampling_location_unresolved,
                    by_origin(SegmentOrigin::Source),
                    by_origin(SegmentOrigin::SyntheticClosure),
                    by_origin(SegmentOrigin::Seam),
                );
            }
            return outcome;
        }
        // The exact-surface max deviation of this pass's mesh, read from the
        // trajectory the outcome pass recorded.
        let row = REFINE_TRAJECTORY.with(|cell| cell.borrow().first().cloned());
        let face = PROBE_FACE_CONTEXT.with(std::cell::Cell::get).0;
        let max_dev = row.as_ref().map(|r| r.max_dev).unwrap_or(f64::INFINITY);
        let tris = row.as_ref().map(|r| r.triangles).unwrap_or(0);
        let excess = row.as_ref().map(|r| r.excess_sum).unwrap_or(0.0);
        let unsafe_count = row.as_ref().map(|r| r.unsafe_count).unwrap_or(0);
        if std::env::var_os("TRUCK_SGC_REFINE_TRACE").is_some() {
            eprintln!(
                "REFINE_PASS\tface={face:?}\tpass={pass}\ttris={tris}\tunsafe={unsafe_count}\tmax_dev={max_dev:.6}\texcess_sum={excess:.6}\tsupports={}",
                support_uvs.len(),
            );
        }
        let candidate_acceptable = matches!(outcome, TessellationOutcome::Mesh(_))
            && max_dev.is_finite()
            && max_dev < best_max_dev - E_PROGRESS;
        if best_outcome.is_none() {
            // Pass 0 is the ordinary mesh; it is the incumbent regardless of
            // whether it satisfies tolerance (a face that cannot be improved
            // returns it unchanged).
            best_max_dev = max_dev;
            best_outcome = Some(outcome);
            best_supports = support_uvs.len();
            best_tris = tris;
            if !candidate_acceptable {
                termination = "no_mesh_at_pass0";
                break;
            }
            if max_dev <= tol {
                termination = "tolerance_met_at_pass0";
                break;
            }
        } else if candidate_acceptable {
            best_max_dev = max_dev;
            best_outcome = Some(outcome);
            best_supports = support_uvs.len();
            best_tris = tris;
            if max_dev <= tol {
                termination = "tolerance_met";
                if std::env::var_os("TRUCK_SGC_REFINE_TRACE").is_some() {
                    eprintln!(
                        "REFINE_DONE\tface={face:?}\tpass={pass}\tmax_dev={max_dev:.6}\ttol={tol:.6}"
                    );
                }
                break;
            }
        } else {
            termination = "no_strict_progress";
            if std::env::var_os("TRUCK_SGC_REFINE_TRACE").is_some() {
                eprintln!(
                    "REFINE_REJECT\tface={face:?}\tpass={pass}\tmax_dev={max_dev:.6}\tbest={best_max_dev:.6}\ttol={tol:.6}"
                );
            }
            break;
        }
        let new_supports = REFINE_SUPPORT_CELL.with(|cell| cell.borrow().clone());
        if new_supports.is_empty() {
            termination = "no_supports_offered";
            break;
        }
        if pass + 1 >= MAX_REFINE_PASSES {
            termination = "pass_cap";
            if std::env::var_os("TRUCK_SGC_REFINE_TRACE").is_some() {
                eprintln!(
                    "REFINE_CAP\tface={face:?}\tpasses={}\tsupports={}",
                    pass + 1,
                    support_uvs.len(),
                );
            }
            break;
        }
        let before = support_uvs.len();
        for sp in new_supports {
            if !support_uvs.iter().any(|x| (x - sp).magnitude() < 1e-9) {
                support_uvs.push(sp);
            }
        }
        if support_uvs.len() == before {
            termination = "support_stall";
            if std::env::var_os("TRUCK_SGC_REFINE_TRACE").is_some() {
                eprintln!(
                    "REFINE_STALL\tface={face:?}\tpass={pass}\tsupports={}\tno_new_supports",
                    support_uvs.len(),
                );
            }
            break;
        }
    }
    if std::env::var_os("TRUCK_SGC_REFINE_TRACE").is_some() {
        if let Some(outcome) = &best_outcome {
            if matches!(outcome, TessellationOutcome::Mesh(_)) {
                eprintln!(
                    "REFINE_ACCEPT\tface={:?}\tmax_dev={best_max_dev:.6}\ttol={tol:.6}\ttris={best_tris}\tsupports={best_supports}\ttermination={termination}",
                    PROBE_FACE_CONTEXT.with(std::cell::Cell::get).0,
                );
            }
        }
    }
    best_outcome.unwrap_or_else(|| {
        TessellationOutcome::Failed(diagnosis::fail(
            TessellationFailureReason::BoundaryConstructionFailed,
            diagnosis::FailureStage::BoundaryConstruction,
        ))
    })
}

/// Tessellates one surface trimmed by polyline, returning `TessellationOutcome`.
#[allow(dead_code)]
fn trimming_tessellation_with_outcome<S>(
    surface: &S,
    polyboundary: &PolyBoundary,
    tol: f64,
    lattice: &CertifiedLattice,
) -> TessellationOutcome
where
    S: PreMeshableSurface,
{
    trimming_tessellation_with_diagnostics(surface, polyboundary, tol, lattice)
}

/// Tessellates one surface trimmed by polyline, preserving why it failed.
///
/// **G8.** The mesh-only form below discards a fully-formed
/// [`TessellationFailure`] â€” including `ContradictoryDualParity`, which is a
/// *proved* inconsistency â€” and returns an empty mesh that the caller cannot
/// distinguish from a face that legitimately meshed to nothing. Detection was
/// never the missing part; the value was constructed and then destroyed one
/// line later. This form is the same computation with the result kept.
fn trimming_tessellation_result<S>(
    surface: &S,
    polyboundary: &PolyBoundary,
    tol: f64,
    lattice: &CertifiedLattice,
    // `Result` here is `truck_topology::Result<T>`, which fixes the error type, so
    // the standard two-parameter form must be named explicitly.
) -> std::result::Result<PolygonMesh, TessellationFailure>
where
    S: PreMeshableSurface,
{
    match trimming_tessellation_with_diagnostics(surface, polyboundary, tol, lattice) {
        TessellationOutcome::Mesh(ft) => {
            let mut mesh = ft.mesh;
            mesh.make_face_compatible_to_normal();
            Ok(mesh)
        }
        TessellationOutcome::Failed(f) => {
            if std::env::var_os("TRUCK_PROBE_FAIL").is_some() {
                eprintln!("PROBE_FAIL reason={:?}", f.reason);
            }
            Err(f)
        }
    }
}

/// Tessellates one surface trimmed by polyline, discarding why it failed.
///
/// Legacy shape, retained for the entry points that cannot carry an outcome.
/// Prefer [`trimming_tessellation_result`].
fn trimming_tessellation<S>(
    surface: &S,
    polyboundary: &PolyBoundary,
    tol: f64,
    lattice: &CertifiedLattice,
) -> PolygonMesh
where
    S: PreMeshableSurface,
{
    trimming_tessellation_result(surface, polyboundary, tol, lattice).unwrap_or_default()
}

/// Inserts parameter divisions into triangulation.
///
/// Returns how many grid samples lay on the boundary, and how many had no
/// established location at all (G7a).
fn insert_surface(
    triangulation: &mut Cdt,
    surface: impl PreMeshableSurface,
    polyline: &PolyBoundary,
    tol: f64,
    roles: &mut ConstraintRoles,
    extra_supports: &[Point2],
) -> (usize, usize) {
    // Grid samples on a boundary segment, by direct test.
    let mut on_boundary = 0usize;
    // Grid samples no method located. Not "outside", and not known to be on the
    // boundary either â€” simply unestablished.
    let mut location_unresolved = 0usize;
    // Audit A1: every constraint added below is an interior sampling edge. It
    // exists to control triangle shape and lies wholly inside the material
    // region â€” `polyline.include` gated the insertion of both its endpoints.
    // It carries no material meaning and must not toggle parity.
    // G5a, and the more consequential half of it.
    //
    // A trim segment that loses its role is still treated as a trim segment,
    // because the unresolved default toggles. A *sampling grid* segment that
    // loses its role is treated as a trim segment too â€” and that is exactly the
    // defect audit A1 removed, reappearing through the chain-splitting hole
    // rather than through the one-bit test A1 fixed. Labelling the whole
    // realized chain closes it.
    let bdb: BoundingBox<Point2> = polyline
        .0
        .iter()
        .flat_map(|loop_| loop_.points.iter())
        .map(std::ops::Deref::deref)
        .collect();
    let range = ((bdb.min()[0], bdb.max()[0]), (bdb.min()[1], bdb.max()[1]));
    let (udiv, vdiv) = surface.parameter_division(range, tol);
    let insert_res: Vec<Vec<Option<_>>> = udiv
        .iter()
        .copied()
        .map(|u| {
            vdiv.iter()
                // G7a. This call site asks "may I place an interior sampling
                // vertex here", not "is this point material". Only `Inside`
                // earns a vertex; the other three decline.
                //
                // Declining asserts nothing about material state â€” that is
                // decided later by constraint roles and the dual labelling,
                // never by this predicate â€” so skipping is the correct
                // conservative answer to the question actually being asked, and
                // refusing the whole face over a shape-control sample would
                // discard geometry for no semantic gain.
                //
                // What was wrong before was not the skip but the silence: an
                // aborted ray was folded into `false`, so a point the algorithm
                // could not classify was indistinguishable from one it had
                // classified as outside. Both residual populations are now
                // counted, and separately, because "on the boundary" and "not
                // established" are different facts.
                .map(|v| match polyline.locate(Point2::new(u, *v)) {
                    PointLocation::Inside => triangulation.insert(SPoint2::new(u, *v)).ok(),
                    PointLocation::Outside => None,
                    PointLocation::Boundary => {
                        on_boundary += 1;
                        None
                    }
                    PointLocation::Indeterminate => {
                        location_unresolved += 1;
                        None
                    }
                })
                .collect()
        })
        .collect();
    // Insert refinement support points as interior vertices, gated by `Inside`
    // exactly like the grid samples. Each support is the argmax-deviation
    // sample of a previously-unsafe material triangle; after insertion the
    // Delaunay triangulation reconnects to it, splitting the offending flat
    // span. No constraint is added for a support vertex; it is a pure Steiner
    // point that owns no parity role.
    let mut support_inserted = 0usize;
    for &sp in extra_supports {
        match polyline.locate(sp) {
            PointLocation::Inside => {
                if triangulation.insert(SPoint2::new(sp.x, sp.y)).is_ok() {
                    support_inserted += 1;
                }
            }
            PointLocation::Outside => {}
            PointLocation::Boundary => on_boundary += 1,
            PointLocation::Indeterminate => location_unresolved += 1,
        }
    }
    if support_inserted > 0 && std::env::var_os("TRUCK_SGC_REFINE_TRACE").is_some() {
        eprintln!(
            "REFINE\tface={:?}\tsupports_offered={}\tsupports_inserted={}",
            PROBE_FACE_CONTEXT.with(std::cell::Cell::get).0,
            extra_supports.len(),
            support_inserted,
        );
    }
    // A boundary carrying a [`SegmentOrigin::Seam`] is periodic/lifted deck
    // geometry: its seam and source edges are duplicate traversals of the same
    // curve, and the grid wiring's boundary vertices would split those edges,
    // leaving the seam's mod-2 traversal pairing broken and the parity flood
    // contradicting (odd toggling vertices). The seam machinery owns that
    // structure; the accuracy wiring applies to the non-seam generic surface
    // path and the windows wiring is restored there verbatim.
    let has_seam = polyline
        .0
        .iter()
        .any(|loop_| loop_.origins.iter().any(|o| *o == SegmentOrigin::Seam));
    if has_seam {
        wire_grid_constraints_windows(triangulation, roles, &insert_res);
    } else {
        wire_grid_constraints(triangulation, roles, polyline, &udiv, &vdiv, &insert_res);
    }
    // PLANAR-C backstop: a grid vertex inserted exactly on a planarized
    // boundary constraint splits it into an unclaimed child; repair those
    // before the flood.
    roles.repair_unlabeled_constraint_edges(triangulation);
    (on_boundary, location_unresolved)
}

/// The pre-accuracy grid wiring, restored for boundaries that carry a
/// [`SegmentOrigin::Seam`] (periodic/lifted deck geometry): constrain between
/// every consecutive *present* grid vertex, including the final u-column.
///
/// This is exactly the wiring the seam faces rendered with before the accuracy
/// work; the accuracy wiring's boundary vertices are unsafe there because the
/// seam/source duplicate traversals break under the mod-2 parity reading when
/// split.
fn wire_grid_constraints_windows(
    triangulation: &mut Cdt,
    roles: &mut ConstraintRoles,
    insert_res: &[Vec<Option<FixedVertexHandle>>],
) {
    insert_res.windows(2).for_each(|vec| {
        vec[0].windows(2).zip(&vec[1]).for_each(|(a, z)| {
            if let Some(x) = a[0] {
                if let Some(y) = a[1] {
                    constrain_grid_edge(triangulation, roles, x, y);
                }
                if let Some(z) = z {
                    constrain_grid_edge(triangulation, roles, x, *z);
                }
            }
        });
        let idx = vec[0].len() - 1;
        if let (Some(x), Some(y)) = (vec[0][idx], vec[1][idx]) {
            constrain_grid_edge(triangulation, roles, x, y);
        }
    });
    let last_column = insert_res.len().saturating_sub(1);
    for pair in insert_res[last_column].windows(2) {
        if let (Some(x), Some(y)) = (pair[0], pair[1]) {
            constrain_grid_edge(triangulation, roles, x, y);
        }
    }
}

/// Constrain the interior sampling grid so that every *material* sub-segment of
/// every grid line is a constrained edge.
///
/// **Audit A1 / G5a.** Every edge added here is an interior sampling edge: it
/// exists to control triangle shape, carries no source evidence, and must not
/// toggle material parity. Each realized chain is labelled
/// [`ConstraintRole::SurfaceSampling`] with its own semantic id.
///
/// **Accuracy contract (ACC-1).** The pre-accuracy wiring only constrained a
/// grid segment when *both* endpoints earned a vertex, so a grid point that was
/// not `Inside` silently deleted the adjacent grid-line constraints. Between
/// the last fully-interior grid line and the trim there was then no constraint
/// at all, and the CDT filled the band with triangles spanning several
/// subdivision cells whose interiors were never certified.
///
/// This wiring instead constrains every material sub-segment of every grid
/// line: each grid segment is cut at its real intersections with the trim
/// polyline, and each maximal inside sub-segment is constrained. Combined with
/// the (already constrained) trim boundary, every final triangle is then
/// confined to the clipped sub-region of one subdivision cell.
fn wire_grid_constraints(
    triangulation: &mut Cdt,
    roles: &mut ConstraintRoles,
    polyline: &PolyBoundary,
    udiv: &[f64],
    vdiv: &[f64],
    insert_res: &[Vec<Option<FixedVertexHandle>>],
) {
    // v-direction links: within each u-column, between consecutive v-rows.
    for (i, u) in udiv.iter().enumerate() {
        for j in 0..vdiv.len() - 1 {
            let a = Point2::new(*u, vdiv[j]);
            let b = Point2::new(*u, vdiv[j + 1]);
            wire_grid_segment(
                triangulation,
                roles,
                polyline,
                a,
                b,
                insert_res[i][j],
                insert_res[i][j + 1],
            );
        }
    }
    // u-direction links: within each v-row, between consecutive u-columns.
    for (j, v) in vdiv.iter().enumerate() {
        for i in 0..udiv.len() - 1 {
            let a = Point2::new(udiv[i], *v);
            let b = Point2::new(udiv[i + 1], *v);
            wire_grid_segment(
                triangulation,
                roles,
                polyline,
                a,
                b,
                insert_res[i][j],
                insert_res[i + 1][j],
            );
        }
    }
}

/// One grid segment, from `a` to `b` (endpoint vertices given or resolvable),
/// cut into its material sub-segments and constrained.
fn wire_grid_segment(
    triangulation: &mut Cdt,
    roles: &mut ConstraintRoles,
    polyline: &PolyBoundary,
    a: Point2,
    b: Point2,
    a_handle: Option<FixedVertexHandle>,
    b_handle: Option<FixedVertexHandle>,
) {
    let mut cuts = grid_segment_trim_intersections(polyline, a, b);
    cuts.push((0.0, a));
    cuts.push((1.0, b));
    cuts.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap_or(std::cmp::Ordering::Equal));
    cuts.dedup_by(|x, y| (x.0 - y.0).abs() < T_EPS);
    for w in cuts.windows(2) {
        let (t0, uv0) = w[0];
        let (t1, uv1) = w[1];
        if t1 - t0 < T_EPS {
            continue;
        }
        let mid_uv = a + (b - a) * (0.5 * (t0 + t1));
        // A material sub-interval, or conservatively an unestablished one (the
        // interval is bounded by real grid/trim geometry either way, so
        // constraining it is a bound, not a guess).
        if !matches!(
            polyline.locate(mid_uv),
            PointLocation::Inside | PointLocation::Indeterminate
        ) {
            continue;
        }
        let Some(x) = interval_endpoint_vertex(triangulation, a, b, a_handle, b_handle, t0, uv0)
        else {
            continue;
        };
        let Some(y) = interval_endpoint_vertex(triangulation, a, b, a_handle, b_handle, t1, uv1)
        else {
            continue;
        };
        if x != y {
            constrain_grid_edge(triangulation, roles, x, y);
        }
    }
}

/// The CDT vertex for the interval boundary at parameter `t`: the grid
/// endpoint's own vertex when the boundary is a grid point that earned one,
/// otherwise a vertex at the (trim-exact) boundary UV `uv`.
fn interval_endpoint_vertex(
    triangulation: &mut Cdt,
    a: Point2,
    b: Point2,
    a_handle: Option<FixedVertexHandle>,
    b_handle: Option<FixedVertexHandle>,
    t: f64,
    uv: Point2,
) -> Option<FixedVertexHandle> {
    if t <= T_EPS {
        if let Some(h) = a_handle {
            return Some(h);
        }
    } else if t >= 1.0 - T_EPS {
        if let Some(h) = b_handle {
            return Some(h);
        }
    }
    resolve_vertex(triangulation, uv)
}

/// Intersection parameters of the grid segment `a -> b` with every **source**
/// trim segment, as sorted candidate cut points in `[0, 1]`.
///
/// Proper crossings, trim vertices lying on the segment, and grid endpoints
/// lying on a trim segment all contribute. A trim segment **parallel** to the
/// grid segment contributes nothing: were it collinear and overlapping, the
/// trim constraint already covers that line, and constraining the grid edge
/// alongside it would give the split-child repair an ambiguous same-line
/// parent to mislabel the trim half with. Such degenerate bands keep the
/// pre-accuracy behaviour (the trim constrains them; the grid adds nothing).
///
/// The [`SegmentOrigin::Seam`] join bridges are deliberately excluded: they are
/// artificial deck-join geometry, not source trim, and a degenerate (legacy)
/// join can place them cutting across the material, where treating them as a
/// material boundary would insert spurious grid/trim cut vertices. The seam is
/// still a CDT constraint (added by [`PolyBoundary::insert_to`]), so the
/// material stays bounded by it; the grid just does not cut there.
fn grid_segment_trim_intersections(
    polyline: &PolyBoundary,
    a: Point2,
    b: Point2,
) -> Vec<(f64, Point2)> {
    let d = b - a;
    let d2 = d.magnitude2();
    let mut ts = Vec::new();
    // Safety margin for the bbox reject, scaled by the grid segment length
    // (so a long segment gets a proportional margin) but never below a floor.
    let eps = T_EPS * d2.sqrt().max(1.0);
    for loop_ in &polyline.0 {
        let n = loop_.points.len();
        for i in 0..n {
            if loop_.origins.get(i) == Some(SegmentOrigin::Seam) {
                continue;
            }
            let (p, q) = (&loop_.points[i], &loop_.points[(i + 1) % n]);
            let (c, e) = (**p, **q);
            // Bounding-box prefilter: no overlap, no intersection.
            if a.x.max(b.x) < c.x.min(e.x) - eps
                || a.x.min(b.x) > c.x.max(e.x) + eps
                || a.y.max(b.y) < c.y.min(e.y) - eps
                || a.y.min(b.y) > c.y.max(e.y) + eps
            {
                continue;
            }
            let seg = e - c;
            let denom = cross2(d, seg);
            if denom.abs() <= 1e-12 {
                // Parallel: no transverse crossing, and collinear overlap is
                // deliberately ignored (see the doc comment).
                continue;
            }
            let r = c - a;
            let t = cross2(r, seg) / denom;
            let s = cross2(r, d) / denom;
            if t >= -T_EPS && t <= 1.0 + T_EPS && s >= -T_EPS && s <= 1.0 + T_EPS {
                // The cut point is the projection onto the **trim** segment
                // (`c + seg * s`), so it lies exactly on the trim line. A grid
                // projection (`a + d * t`) carries fp error that can place the
                // vertex a hair outside the material; the constraint to it
                // would then cross the trim and be refused.
                ts.push((t.clamp(0.0, 1.0), c + seg * s));
            }
        }
    }
    ts
}

/// Whether a parameter point lies in the material region, decided for the
/// interior by the parity the flood will select and for the boundary exactly as
/// [`PolyBoundary::locate`] decides it.
///
/// - A midpoint on the trim (within the raw locate's boundary tolerance) is
///   **not** material: the trim already constrains there, and constraining the
///   adjacent sliver would insert vertices on the boundary that can break the
///   parity structure.
/// - Otherwise the point is material iff the face it lies in carries odd
///   parity under `early_parity`, computed on the boundary-only CDT before any
///   grid vertex or [`ConstraintRole::SurfaceSampling`] constraint was added —
///   exactly the material the final flood selects. This is what makes the
///   classification agree with the flood on degenerate (self-crossing legacy
///   join) boundaries, where the raw polyline ray-cast names a different
///   material than the planarized boundary does.
///
/// `None` parity (early flood failed, so the face will fail later) and
/// on-edge/on-vertex points classify conservatively as material.
/// An existing vertex at `uv` (within the same UV weld radius the boundary
/// uses), or a freshly inserted one.
fn resolve_vertex(triangulation: &mut Cdt, uv: Point2) -> Option<FixedVertexHandle> {
    let sp = SPoint2::new(spade_round(uv.x), spade_round(uv.y));
    if let Some(idx) = triangulation
        .vertices()
        .find(|v| sp.distance_2(*v.as_ref()) < 1e-12)
    {
        return Some(idx.fix());
    }
    triangulation.insert(sp).ok()
}

/// One interior sampling constraint, with its semantic label.
///
/// The material sub-interval it realizes may properly cross the trim; use
/// [`ConstraintRoles::insert_with_split`] so the crossing is planarized into a
/// constrained split rather than refused. A crossing network Spade cannot
/// planarize (or an existing duplicate constraint) is a best-effort skip: the
/// grid exists to shape the triangulation, never to decide material, so a
/// refused grid edge must not fail the face.
fn constrain_grid_edge(
    triangulation: &mut Cdt,
    roles: &mut ConstraintRoles,
    a: FixedVertexHandle,
    b: FixedVertexHandle,
) {
    if triangulation
        .get_edge_from_neighbors(a, b)
        .filter(|e| e.is_constraint_edge())
        .is_some()
    {
        return;
    }
    // A material sub-interval is bounded by real grid/trim geometry, so its
    // interior cannot cross the trim: every trim crossing of the host grid
    // segment was already computed as an interval boundary. `try_add_constraint`
    // therefore suffices and deliberately avoids `add_constraint_and_split`,
    // whose planarization can relocate an existing trim constraint and leave an
    // odd toggling vertex behind. An empty chain is a best-effort skip.
    let chain = triangulation.try_add_constraint(a, b);
    if chain.is_empty() {
        return;
    }
    // PLANAR-B B6: interior sampling constraints carry no source uses and
    // have no SegmentOrigin; each realized chain gets its own semantic id.
    let semantic_id = roles.mint_semantic_constraint_id();
    roles.label_realized_chain(
        triangulation,
        &chain,
        semantic_id,
        ConstraintRole::SurfaceSampling,
        &[],
        None,
    );
}

/// Two-dimensional cross product.
fn cross2(a: Vector2, b: Vector2) -> f64 {
    a.x * b.y - a.y * b.x
}

/// Relative tolerance for grid-segment cut points: two intersection parameters
/// closer than this are the same cut.
const T_EPS: f64 = 1e-9;

/// Labels every CDT face with material parity by flooding the dual graph from
/// the outer face, flipping across constraint edges that are entitled to toggle.
///
/// `reading` is handed straight to [`ConstraintRoles::toggles_material`]; see
/// there.
///
/// Errors carry the terminal reason rather than a bool so the caller can tell a
/// self-contradicting flood (retryable under a different reading) from a
/// missing role (an invariant violation that no reading repairs).
fn flood_parity(
    triangulation: &Cdt,
    roles: &ConstraintRoles,
    reading: ParityReading,
) -> std::result::Result<std::collections::HashMap<usize, u32>, TessellationFailureReason> {
    use std::collections::{HashMap as StdHashMap, VecDeque};

    let mut face_parity = StdHashMap::<usize, u32>::new();
    let mut queue = VecDeque::new();

    let outer = triangulation.outer_face();
    face_parity.insert(outer.index(), 0);
    queue.push_back((outer.fix(), 0));

    let mut contradictory_parity = false;
    while let Some((ffh, current_parity)) = queue.pop_front() {
        let face = triangulation.face(ffh);
        let edges = match face.as_inner() {
            Some(inner) => inner.adjacent_edges(),
            None => match face.adjacent_edge() {
                Some(e0) => {
                    let e1 = e0.next();
                    let e2 = e1.next();
                    [e0, e1, e2]
                }
                // The outer face has no adjacent edge, so the CDT holds fewer
                // than three distinct vertices: there are no inner faces and no
                // material region to select. This was an `unwrap`, and it
                // aborts the *whole model* rather than the face â€” `00005641`
                // panicked here the moment WAVE-4B started returning closed-form
                // parameters for boundaries that used to fail projection
                // outright, some of which collapse to a point in the chart.
                //
                // Stopping the flood is the honest answer: step 3 finds no face
                // at odd parity and reports `NoOddParityRegion`, which is
                // exactly what a degenerate chart image is.
                None => break,
            },
        };
        for e in edges {
            // Audit A1. This was `e.is_constraint_edge()` â€” one bit, so an
            // interior sampling constraint flipped material parity exactly as a
            // trim segment did. A constraint edge is now only a material
            // transition if its role says so.
            let is_domain_boundary = if e.is_constraint_edge() {
                // G5b: an edge with no resolvable role stops the face rather
                // than being assigned a material meaning it does not have.
                match roles.toggles_material(triangulation, e.as_undirected().fix(), reading) {
                    Some(toggles) => toggles,
                    None => return Err(TessellationFailureReason::ConstraintRoleMissing),
                }
            } else {
                false
            };
            let next_parity = if is_domain_boundary {
                current_parity ^ 1
            } else {
                current_parity
            };
            let adj_face = e.rev().face();
            let adj_idx = adj_face.index();
            if let Some(&existing_parity) = face_parity.get(&adj_idx) {
                if existing_parity != next_parity {
                    contradictory_parity = true;
                }
            } else {
                face_parity.insert(adj_idx, next_parity);
                queue.push_back((adj_face.fix(), next_parity));
            }
        }
    }

    if contradictory_parity {
        return Err(TessellationFailureReason::ContradictoryDualParity);
    }
    Ok(face_parity)
}

/// How many vertices carry an odd number of incident *toggling* constraint
/// edges.
///
/// This is the exact obstruction the flood trips over. Walking the faces around
/// one vertex returns to where it started, so parity is consistent there only
/// if the walk crosses an even number of toggling edges; a single odd vertex
/// anywhere makes [`flood_parity`] contradict itself no matter which order it
/// visits faces in. Zero odd vertices means the toggling subgraph is a cycle
/// mod 2 â€” a closed boundary â€” and the flood cannot fail.
///
/// So this separates "the material reading of some role is wrong" from "the
/// constraint set is not a closed boundary at all", which no count of failures
/// can distinguish. Diagnostic only.
fn odd_toggling_vertices(
    triangulation: &Cdt,
    roles: &ConstraintRoles,
    reading: ParityReading,
) -> usize {
    use std::collections::HashMap as StdHashMap;
    let mut degree = StdHashMap::<usize, usize>::new();
    for e in triangulation.undirected_edges() {
        if !e.is_constraint_edge() {
            continue;
        }
        if roles.toggles_material(triangulation, e.fix(), reading) != Some(true) {
            continue;
        }
        for v in e.vertices() {
            *degree.entry(v.fix().index()).or_insert(0) += 1;
        }
    }
    degree.values().filter(|d| **d % 2 == 1).count()
}

/// Converts triangulation into `TessellationOutcome`.
fn triangulation_into_polymesh_outcome<S: ParametricSurface3D>(
    triangulation: &Cdt,
    surface: &S,
    polyline: &PolyBoundary,
    boundary_map: &HashMap<FixedVertexHandle, Point3>,
    roles: &ConstraintRoles,
    vertex_sources: &HashMap<FixedVertexHandle, Vec<SourceEdgeUse>>,
    lattice: &CertifiedLattice,
    tol: f64,
    refine: bool,
) -> TessellationOutcome {
    let ctx = ToleranceCtx::unscaled_legacy();
    use std::collections::HashMap as StdHashMap;

    // 1. Parity-labeled CDT dual traversal across domain-boundary constraint
    //    edges. The set reading is asked first, so a face that renders today
    //    renders identically.
    let reading = PARITY_READING.with(std::cell::Cell::get);
    let flooded = flood_parity(triangulation, roles, reading);
    if std::env::var("TRUCK_PROBE_PARITY").is_ok() {
        let repeated = roles.traversals.values().filter(|n| **n > 1).count();
        let constraint_edges = triangulation
            .undirected_edges()
            .filter(|e| e.is_constraint_edge())
            .count();
        let semantic_claims = triangulation
            .undirected_edges()
            .map(|e| e.data().data().claims.len())
            .sum::<usize>();
        eprintln!(
            "PARITY\treading={reading:?}\tconstraint_edges={constraint_edges}\t\
             semantic_claims={semantic_claims}\trepeated_traversals={repeated}\t\
             odd_legacy={}\todd_winding={}\toutcome={}",
            odd_toggling_vertices(triangulation, roles, ParityReading::Legacy),
            odd_toggling_vertices(triangulation, roles, ParityReading::TraversalParity),
            match &flooded {
                Ok(_) => "ok".to_string(),
                Err(reason) => format!("{reason:?}"),
            },
        );
    }
    let face_parity = match flooded {
        Ok(parity) => parity,
        Err(reason) => {
            return TessellationOutcome::Failed(diagnosis::fail(
                reason,
                diagnosis::failure_stage_for_reason(reason),
            ))
        }
    };

    // 2. Vertex positions, parameter coordinates, and roles
    let mut positions = Vec::<Point3>::new();
    let mut uv_coords = Vec::<Vector2>::new();
    let mut normals = Vec::<Vector3>::new();
    let mut vertex_metadata = Vec::<VertexMetadata>::new();

    let u_period = lattice.declared_u_period();
    let v_period = lattice.declared_v_period();

    let vmap: HashMap<_, _> = triangulation
        .vertices()
        .enumerate()
        .map(|(i, v)| {
            let p = *v.as_ref();
            let idx = v.fix();
            let point = match boundary_map.get(&idx) {
                Some(point) => *point,
                None => surface.subs(p.x, p.y),
            };
            if !point.x.is_finite() || !point.y.is_finite() || !point.z.is_finite() {
                return (idx, usize::MAX);
            }
            positions.push(point);
            let uv = Vector2::new(p.x, p.y);
            uv_coords.push(uv);

            // Determine vertex roles (bitflags)
            let mut roles = VertexRoles::default();
            if boundary_map.contains_key(&idx) {
                let is_seam = (u_period.is_some()
                    && (p.x <= 1e-4 || (p.x - u_period.unwrap()).abs() <= 1e-4))
                    || (v_period.is_some()
                        && (p.y <= 1e-4 || (p.y - v_period.unwrap()).abs() <= 1e-4));
                if is_seam {
                    roles.insert(VertexRoles::ARTIFICIAL_SEAM);
                } else {
                    roles.insert(VertexRoles::PHYSICAL_BOUNDARY);
                }
            } else {
                // INTERIOR is derived downstream as the absence of any
                // boundary/seam/singular role; it is not stored as an
                // overlapping flag (PR 4A.1 design, sidecar diagnostics).
            }

            let n = surface.normal(p.x, p.y);
            let n_valid = if n.x.is_finite()
                && n.y.is_finite()
                && n.z.is_finite()
                && n.magnitude2() > 1e-12
            {
                n.normalize()
            } else {
                roles.insert(VertexRoles::SINGULAR_COLLAPSE);
                Vector3::zero()
            };
            normals.push(n_valid);

            // PLANAR-A A6: populate `source_edge_use` only where the vertex has
            // exactly one distinct contributing source edge use. A junction
            // vertex shared by two edges, a snapped duplicate, or a synthetic
            // vertex carries several or no uses and is left `None` rather than
            // given a fabricated attribution.
            let source_edge_use = {
                let mut distinct: Vec<SourceEdgeUse> = Vec::new();
                for &use_ in vertex_sources.get(&idx).into_iter().flatten() {
                    if !distinct.contains(&use_) {
                        distinct.push(use_);
                    }
                }
                match distinct.as_slice() {
                    [single] => Some(single.index),
                    _ => None,
                }
            };

            vertex_metadata.push(VertexMetadata {
                uv: Point2::new(p.x, p.y),
                generation: if boundary_map.contains_key(&idx) {
                    VertexGeneration::SourceEdgeSample
                } else {
                    VertexGeneration::SurfaceEvaluation
                },
                roles,
                source_edge_use,
                seam_pair: None,
                singular_group: None,
            });

            (idx, i)
        })
        .collect();

    if vmap.values().any(|&i| i == usize::MAX) {
        return TessellationOutcome::Failed(diagnosis::fail(
            TessellationFailureReason::NonFinitePosition,
            diagnosis::FailureStage::SurfaceEvaluation,
        ));
    }

    // 3. Material triangles selection (odd parity = 1)
    //
    // Selection and validation are two stages, and they are counted as two.
    // Fused into one chained iterator, as they were, a face where parity chose
    // no region and a face where parity chose a region that validation then
    // emptied both arrive at `NoOddParityRegion` indistinguishable â€” and that
    // reason carries 342 faces. The behaviour is unchanged: same predicates,
    // same order, same result.
    let stage_raw_cdt_triangles = triangulation.inner_faces().count();
    let material_selected: Vec<[usize; 3]> = triangulation
        .inner_faces()
        .filter(|face| face_parity.get(&face.index()) == Some(&1))
        .map(|tri| tri.vertices())
        .map(|tri| {
            [
                vmap[&tri[0].fix()],
                vmap[&tri[1].fix()],
                vmap[&tri[2].fix()],
            ]
        })
        .collect();
    let stage_material_selected = material_selected.len();
    // The realized world points and UVs of the selected region, kept so
    // Detector C can certify a region parity selected but validation emptied.
    // The `positions` vector is fully populated here — the `NonFinitePosition`
    // early return above already fired — so every index is a real vertex.
    let mut selected_world_points: Vec<Point3> = Vec::new();
    let mut selected_uvs: Vec<Vector2> = Vec::new();
    for &[i0, i1, i2] in &material_selected {
        selected_world_points.push(positions[i0]);
        selected_world_points.push(positions[i1]);
        selected_world_points.push(positions[i2]);
        selected_uvs.push(uv_coords[i0]);
        selected_uvs.push(uv_coords[i1]);
        selected_uvs.push(uv_coords[i2]);
    }
    let mut max_realized_area = 0.0f64;
    let tri_faces_raw: Vec<[usize; 3]> = material_selected
        .into_iter()
        .filter(|idcs| {
            if idcs[0] == idcs[1] || idcs[1] == idcs[2] || idcs[0] == idcs[2] {
                return false;
            }
            let p0 = positions[idcs[0]];
            let p1 = positions[idcs[1]];
            let p2 = positions[idcs[2]];
            let cross = (p1 - p0).cross(p2 - p0);
            let area = 0.5 * cross.magnitude();
            if area.is_finite() {
                max_realized_area = max_realized_area.max(area);
            }
            area > 1e-12 && area.is_finite()
        })
        .collect();

    // Collect the argmax-deviation sample UV of every material triangle whose
    // *exact-surface* sampled deviation exceeds the face tolerance. The
    // exact-surface estimator evaluates the flat triangle with its corners
    // taken as the true surface points `S(uv)`, so only genuine CDT
    // curvature-bypass activates refinement; boundary-realization error (coarse
    // trim polyline vertices) is invisible to it. The caller uses these as
    // interior support points for a subsequent CDT rebuild. No-op when `refine`
    // is false.
    if refine {
        let mut collected = 0usize;
        let mut max_dev_any = 0.0f64;
        let mut excess_sum = 0.0f64;
        let mut worst_prov: Option<(&VertexMetadata, &VertexMetadata, &VertexMetadata)> = None;
        const MAX_REFINE_COLLECT: usize = 1 << 10;
        for &[i0, i1, i2] in &tri_faces_raw {
            let uv_a = Point2::new(uv_coords[i0].x, uv_coords[i0].y);
            let uv_b = Point2::new(uv_coords[i1].x, uv_coords[i1].y);
            let uv_c = Point2::new(uv_coords[i2].x, uv_coords[i2].y);
            let (dev, argmax) = triangle_sampled_deviation_exact(surface, uv_a, uv_b, uv_c);
            if dev > max_dev_any {
                max_dev_any = dev;
                worst_prov = Some((
                    &vertex_metadata[i0],
                    &vertex_metadata[i1],
                    &vertex_metadata[i2],
                ));
            }
            if dev > tol {
                excess_sum += dev - tol;
                if collected < MAX_REFINE_COLLECT {
                    REFINE_SUPPORT_CELL.with(|cell| cell.borrow_mut().push(argmax));
                    collected += 1;
                }
            }
        }
        let (worst_seam, worst_singular, worst_boundary) = match worst_prov {
            Some((a, b, c)) => {
                let has = |r: &VertexMetadata, bit: u16| r.roles.contains(bit);
                (
                    has(a, VertexRoles::ARTIFICIAL_SEAM)
                        || has(b, VertexRoles::ARTIFICIAL_SEAM)
                        || has(c, VertexRoles::ARTIFICIAL_SEAM),
                    has(a, VertexRoles::SINGULAR_COLLAPSE)
                        || has(b, VertexRoles::SINGULAR_COLLAPSE)
                        || has(c, VertexRoles::SINGULAR_COLLAPSE),
                    has(a, VertexRoles::PHYSICAL_BOUNDARY)
                        || has(b, VertexRoles::PHYSICAL_BOUNDARY)
                        || has(c, VertexRoles::PHYSICAL_BOUNDARY),
                )
            }
            None => (false, false, false),
        };
        REFINE_TRAJECTORY.with(|cell| {
            cell.borrow_mut().push(RefineTrajectoryRow {
                triangles: tri_faces_raw.len(),
                unsafe_count: collected,
                max_dev: max_dev_any,
                excess_sum,
                worst_seam,
                worst_singular,
                worst_boundary,
            });
        });
        if std::env::var_os("TRUCK_SGC_REFINE_TRACE").is_some() {
            eprintln!(
                "REFINE_SCAN\tface={:?}\tunsafe={}\ttol={:.6}\ttris={}\tmax_dev={:.6}\texcess_sum={:.6}\tworst_seam={}\tworst_sing={}\tworst_bnd={}",
                PROBE_FACE_CONTEXT.with(std::cell::Cell::get).0,
                collected,
                tol,
                tri_faces_raw.len(),
                max_dev_any,
                excess_sum,
                worst_seam,
                worst_singular,
                worst_boundary,
            );
        }
    }

    if diagnosis::diag_enabled() {
        diagnosis::record_cdt_stages(
            stage_raw_cdt_triangles,
            stage_material_selected,
            tri_faces_raw.len(),
        );
    }

    if tri_faces_raw.is_empty() {
        // Detector C: parity selected a region (material_selected > 0) but the
        // world-area validator emptied it. Every realized triangle sits at or
        // below the `1e-12` world-area floor, which is genuine physical
        // degeneracy at the mesh resolution — a certified rejection, not an
        // unresolved parity outcome. `selected == 0` (parity chose nothing) is
        // the other terminal and stays `NoOddParityRegion` here; Detector B
        // certifies that class from the boundary before the CDT.
        if stage_material_selected > 0 && !selected_world_points.is_empty() {
            // The realized UV extents of the selected region.
            let (mut uv_lo, mut uv_hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
            for uv in &selected_uvs {
                uv_lo[0] = uv_lo[0].min(uv.x);
                uv_hi[0] = uv_hi[0].max(uv.x);
                uv_lo[1] = uv_lo[1].min(uv.y);
                uv_hi[1] = uv_hi[1].max(uv.y);
            }
            let uv_extents = (
                (uv_hi[0] - uv_lo[0]).max(0.0),
                (uv_hi[1] - uv_lo[1]).max(0.0),
            );
            // The realized world rank of the selected region.
            let (world_rank, rank_span, rank_max_perp, rank_tolerance) =
                validity::world_rank_of(&selected_world_points);
            let world_extents = {
                let mut lo = [f64::INFINITY; 3];
                let mut hi = [f64::NEG_INFINITY; 3];
                for p in &selected_world_points {
                    lo[0] = lo[0].min(p.x);
                    hi[0] = hi[0].max(p.x);
                    lo[1] = lo[1].min(p.y);
                    hi[1] = hi[1].max(p.y);
                    lo[2] = lo[2].min(p.z);
                    hi[2] = hi[2].max(p.z);
                }
                let mut ext = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
                ext.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
                (ext[0], ext[1], ext[2])
            };
            let certificate = FaceValidityCertificate::sub_tolerance_sliver(
                polyline.0.len(),
                polyline.0.len(),
                world_rank,
                rank_span,
                rank_max_perp,
                rank_tolerance,
                uv_extents,
                world_extents,
                stage_material_selected,
                max_realized_area,
            );
            return TessellationOutcome::Failed(diagnosis::reject(
                TessellationFailureReason::RejectedDegenerate,
                diagnosis::FailureStage::ValidityClassification,
                certificate,
            ));
        }
        return TessellationOutcome::Failed(diagnosis::fail(
            TessellationFailureReason::NoOddParityRegion,
            diagnosis::FailureStage::MaterialSelection,
        ));
    }

    // 4. Singular Vertex Normal Repair
    let mut vertex_incident_normals = vec![Vector3::zero(); positions.len()];
    for &[i0, i1, i2] in &tri_faces_raw {
        let p0 = positions[i0];
        let p1 = positions[i1];
        let p2 = positions[i2];
        let cross = (p1 - p0).cross(p2 - p0);
        let mag = cross.magnitude();
        if mag > 1e-12 {
            let fnorm = cross / mag;
            vertex_incident_normals[i0] += fnorm * mag;
            vertex_incident_normals[i1] += fnorm * mag;
            vertex_incident_normals[i2] += fnorm * mag;
        }
    }

    for (i, norm) in normals.iter_mut().enumerate() {
        if ctx.is_small_ratio(norm.magnitude()) || !norm.x.is_finite() {
            // BG-TOL-001: param
            let inc = vertex_incident_normals[i];
            if inc.magnitude2() > 1e-12 {
                *norm = inc.normalize();
            } else {
                *norm = Vector3::unit_z();
            }
        }
    }

    let tri_faces: Vec<[StandardVertex; 3]> = tri_faces_raw
        .into_iter()
        .map(|idcs| array![i => [idcs[i], idcs[i], idcs[i]].into(); 3])
        .collect();

    // PR 4A.1 invariant: sidecar metadata must stay aligned with mesh positions,
    // one record per vertex, through every transformation below. Enforced in the
    // test build; the census (release) relies on it holding by construction here.
    debug_assert_eq!(vertex_metadata.len(), positions.len());

    let mesh = PolygonMesh::debug_new(
        StandardAttributes {
            positions,
            uv_coords,
            normals,
        },
        Faces::from_tri_and_quad_faces(tri_faces, Vec::new()),
    );

    TessellationOutcome::Mesh(FaceTessellation {
        mesh,
        diagnostics: TessellationDiagnostics {
            vertex_metadata,
            seam_pairs: Vec::new(),
            singular_groups: Vec::new(),
        },
    })
}

/// Sampled surface-vs-flat deviation estimator for the material triangle
/// realized at `(p_a, p_b, p_c)` with UV corners `(uv_a, uv_b, uv_c)`.
///
/// Samples the surface at the three edge midpoints and the barycenter, and for
/// each sample measures the physical distance between the real surface point
/// `S(q)` and the linear point in the realized triangle at the same UV
/// barycentric weights. Returns `(max_dev, argmax_sample_uv)`.
///
/// This is a diagnostic estimator, not a general mathematical certificate.
fn triangle_sampled_deviation<S: ParametricSurface3D>(
    surface: &S,
    uv_a: Point2,
    uv_b: Point2,
    uv_c: Point2,
    p_a: Point3,
    p_b: Point3,
    p_c: Point3,
) -> (f64, Point2) {
    let corners = [(uv_a, p_a), (uv_b, p_b), (uv_c, p_c)];
    sampled_deviation_impl(surface, &corners)
}

/// Same estimator, but the triangle corners are taken as the *exact* surface
/// points `S(uv)` of the three UV corners rather than the realized mesh
/// vertices. This isolates the CDT curvature-bypass component of the deviation:
/// a triangle whose corners are exact surface points can only deviate if the
/// flat CDT triangle spans genuine surface curvature. Boundary-realization
/// error (coarse trim polyline vertices stored in `boundary_map`, which sit off
/// the analytic surface) is invisible to this estimator, so a face limited only
/// by boundary realization does not activate refinement.
fn triangle_sampled_deviation_exact<S: ParametricSurface3D>(
    surface: &S,
    uv_a: Point2,
    uv_b: Point2,
    uv_c: Point2,
) -> (f64, Point2) {
    let corners = [
        (uv_a, surface.subs(uv_a.x, uv_a.y)),
        (uv_b, surface.subs(uv_b.x, uv_b.y)),
        (uv_c, surface.subs(uv_c.x, uv_c.y)),
    ];
    sampled_deviation_impl(surface, &corners)
}

fn sampled_deviation_impl<S: ParametricSurface3D>(
    surface: &S,
    corners: &[(Point2, Point3); 3],
) -> (f64, Point2) {
    let (uv_a, p_a) = corners[0];
    let (uv_b, p_b) = corners[1];
    let (uv_c, p_c) = corners[2];
    // Barycentric weights of `q` in the UV triangle `(a, b, c)`, via 2D
    // cross-product areas. Returns `None` for a degenerate UV triangle.
    let cross2 = |p: Vector2, q: Vector2| p.x * q.y - p.y * q.x;
    let barycentric = |q: Point2, a: Point2, b: Point2, c: Point2| -> Option<(f64, f64, f64)> {
        let d = cross2(b - a, c - a);
        if d.abs() <= 1e-14 {
            return None;
        }
        let w_a = cross2(b - q, c - q) / d;
        let w_b = cross2(c - q, a - q) / d;
        let w_c = 1.0 - w_a - w_b;
        Some((w_a, w_b, w_c))
    };
    let samples = [
        uv_a + (uv_b - uv_a) * 0.5,
        uv_b + (uv_c - uv_b) * 0.5,
        uv_c + (uv_a - uv_c) * 0.5,
        uv_a + ((uv_b - uv_a) + (uv_c - uv_a)) / 3.0,
    ];
    let mut max_dev = 0.0f64;
    let mut argmax = samples[0];
    for q in samples {
        if let Some((w_a, w_b, w_c)) = barycentric(q, uv_a, uv_b, uv_c) {
            let linear = p_a + (p_b - p_a) * w_b + (p_c - p_a) * w_c;
            let real = surface.subs(q.x, q.y);
            let dev = (real - linear).magnitude();
            if dev.is_finite() && dev > max_dev {
                max_dev = dev;
                argmax = q;
            }
        }
    }
    (max_dev, argmax)
}

/// One per-pass record of the refinement trajectory for a face. The acceptance
/// rule is justified by the geometric error functional here — triangle count is
/// deliberately not the whole story.
#[derive(Clone, Debug)]
struct RefineTrajectoryRow {
    triangles: usize,
    unsafe_count: usize,
    max_dev: f64,
    excess_sum: f64,
    /// Whether the worst-offending triangle touches seam/singular/boundary
    /// structure (role bits from its three vertex metadata entries).
    worst_seam: bool,
    worst_singular: bool,
    worst_boundary: bool,
}

#[allow(dead_code)]
fn triangulation_into_polymesh<S: ParametricSurface3D>(
    triangulation: &Cdt,
    surface: &S,
    polyline: &PolyBoundary,
    boundary_map: &HashMap<FixedVertexHandle, Point3>,
    roles: &ConstraintRoles,
    lattice: &CertifiedLattice,
) -> PolygonMesh {
    let vertex_sources = HashMap::default();
    match triangulation_into_polymesh_outcome(
        triangulation,
        surface,
        polyline,
        boundary_map,
        roles,
        &vertex_sources,
        lattice,
        f64::INFINITY,
        false,
    ) {
        TessellationOutcome::Mesh(ft) => ft.mesh,
        TessellationOutcome::Failed(_) => PolygonMesh::default(),
    }
}

fn polyline_on_surface(
    surface: impl PreMeshableSurface,
    p: SurfacePoint,
    q: SurfacePoint,
    tol: f64,
) -> Vec<SurfacePoint> {
    use truck_geometry::prelude::*;
    let tol = tol.max(TOLERANCE);
    let line = Line(p.uv, q.uv);
    let pcurve = PCurve::new(line, &surface);
    let (vec, _) = pcurve.parameter_division(pcurve.range_tuple(), tol);
    vec.into_iter()
        .map(|t| {
            let uv = line.subs(t);
            (uv, surface.subs(uv.x, uv.y)).into()
        })
        .collect()
}

/// Explicit classification for a conical/revoluted face with one circular boundary and a collapsed apex.
#[allow(dead_code, unused)]
#[derive(Debug, Clone, Copy)]
pub struct CollapsedPeriodicBoundaryPair {
    pub base_u: f64,
    pub apex_u: f64,
}

impl CollapsedPeriodicBoundaryPair {
    /// Classify whether a face's boundary consists of a single regular periodic loop and a collapsed apex.
    pub fn try_classify<S: PreMeshableSurface>(
        surface: &S,
        closed: &[Vec<SurfacePoint>],
        open: &[Vec<SurfacePoint>],
        _range: (Option<(f64, f64)>, Option<(f64, f64)>),
        lattice: &CertifiedLattice,
    ) -> Option<Self> {
        // 1. Exactly one regular closed periodic boundary and no open generator/sector curves
        if closed.len() != 1 || !open.is_empty() {
            return None;
        }

        let (period, is_v) = match (lattice.declared_v_period(), lattice.declared_u_period()) {
            (Some(vp), _) if vp > 1e-6 => (vp, true),
            (_, Some(up)) if up > 1e-6 => (up, false),
            _ => return None,
        };
        let loop0 = &closed[0];

        // 2. Regular loop must span the periodic parameter (winding Â±1, span ~ period)
        let (p_min, p_max) =
            loop0
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(mn, mx), p| {
                    let val = if is_v { p.uv.y } else { p.uv.x };
                    (mn.min(val), mx.max(val))
                });
        if (p_max - p_min) < 0.75 * period {
            return None;
        }

        let base_r: f64 = loop0
            .iter()
            .map(|p| if is_v { p.uv.x } else { p.uv.y })
            .sum::<f64>()
            / loop0.len() as f64;

        // 3. Analytically compute exact apex parameter where angular orbit collapses in 3D.
        use cgmath::InnerSpace;
        let w = |r: f64| -> Vector3 {
            let (p0, p_half) = if is_v {
                (surface.subs(r, 0.0), surface.subs(r, 0.5 * period))
            } else {
                (surface.subs(0.0, r), surface.subs(0.5 * period, r))
            };
            p0 - p_half
        };

        let w0 = w(0.0);
        let w1 = w(1.0);
        let dw = w1 - w0;
        let dw2 = dw.magnitude2();

        if dw2 < 1e-12 {
            return None;
        }

        let apex_r = -w0.dot(dw) / dw2;

        // 4. Certificate guard: verify 3D point collapse at apex
        if w(apex_r).magnitude() > 1e-3 {
            return None;
        }

        // 5. Axial separation between base loop and apex must be nonzero
        if (base_r - apex_r).abs() < 1e-6 {
            return None;
        }

        Some(Self {
            base_u: base_r,
            apex_u: apex_r,
        })
    }
}

#[allow(dead_code, unused)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoundaryStratum {
    #[allow(dead_code, unused)]
    RegularCurve,
    #[allow(dead_code, unused)]
    CollapsedToPoint,
    #[allow(dead_code, unused)]
    PeriodicIdentification,
}

#[allow(dead_code, unused)]
#[derive(Clone, Debug)]
pub struct WireAssembledFace {
    pub loops: Vec<Vec<SurfacePoint>>,
    pub strata: Vec<BoundaryStratum>,
}

#[allow(dead_code, unused)]
#[derive(Clone, Debug)]
pub struct QuotientLiftCertificate {
    #[allow(dead_code, unused)]
    pub period_shifts: Vec<(i32, i32)>,
    #[allow(dead_code, unused)]
    pub max_junction_residual: f64,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct QuotientResolvedFace {
    pub resolved_loops: Vec<Vec<SurfacePoint>>,
    pub certificate: QuotientLiftCertificate,
}

#[allow(dead_code)]
pub fn solve_quotient_lift<S: PreMeshableSurface>(
    surface: &S,
    loops: &[Vec<SurfacePoint>],
) -> Option<QuotientResolvedFace> {
    let u_period = surface.u_period();
    let v_period = surface.v_period();

    let mut resolved_loops = Vec::with_capacity(loops.len());
    let mut period_shifts = Vec::with_capacity(loops.len());
    let mut max_junction_residual = 0.0f64;

    for loop_pts in loops {
        if loop_pts.len() < 2 {
            resolved_loops.push(loop_pts.clone());
            period_shifts.push((0, 0));
            continue;
        }

        let p0 = loop_pts[0].uv;
        let p1 = loop_pts[loop_pts.len() - 1].uv;
        let du = p1.x - p0.x;
        let dv = p1.y - p0.y;

        let ku = match u_period {
            Some(pu) if pu > 1e-6 => (du / pu).round() as i32,
            _ => 0,
        };
        let kv = match v_period {
            Some(pv) if pv > 1e-6 => (dv / pv).round() as i32,
            _ => 0,
        };

        period_shifts.push((ku, kv));

        let u_shift = match u_period {
            Some(pu) => (ku as f64) * pu,
            None => 0.0,
        };
        let v_shift = match v_period {
            Some(pv) => (kv as f64) * pv,
            None => 0.0,
        };

        let mut lifted_loop = loop_pts.clone();
        let n = lifted_loop.len();
        for (idx, pt) in lifted_loop.iter_mut().enumerate() {
            let frac = idx as f64 / (n - 1).max(1) as f64;
            pt.uv.x -= frac * u_shift;
            pt.uv.y -= frac * v_shift;
        }

        let residual = lifted_loop[0].uv.distance(lifted_loop[n - 1].uv);
        max_junction_residual = max_junction_residual.max(residual);
        resolved_loops.push(lifted_loop);
    }

    Some(QuotientResolvedFace {
        resolved_loops,
        certificate: QuotientLiftCertificate {
            period_shifts,
            max_junction_residual,
        },
    })
}

#[test]
fn test_global_quotient_lift_solver() {
    use truck_modeling::{BSplineSurface, KnotVec};
    let knot_vec = KnotVec::from(vec![0.0, 0.0, 1.0, 1.0]);
    let ctrl_pts = vec![
        vec![Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0)],
        vec![Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0)],
    ];
    let surface = BSplineSurface::new((knot_vec.clone(), knot_vec), ctrl_pts);
    let loop_pts = vec![
        (Point2::new(0.0, 0.0), Point3::new(0.0, 0.0, 0.0)).into(),
        (Point2::new(1.0, 1.0), Point3::new(1.0, 1.0, 0.0)).into(),
    ];
    let resolved = solve_quotient_lift(&surface, &[loop_pts]);
    assert!(resolved.is_some());
    let res = resolved.unwrap();
    assert_eq!(res.certificate.period_shifts, vec![(0, 0)]);
}

/// P1: a closed cubic B-spline with *unclamped* end knots (each end multiplicity
/// 2, not 4) extends its knot vector beyond the shape it draws. `range_tuple()`
/// reports the bare knot extremes `[-0.03125, 1.0625]`, where the basis is not a
/// partition of unity and `subs` returns the origin. The boundary polyline must
/// be sampled over the actually evaluable interior span `[0, 1]` instead.
#[test]
fn closed_spline_boundary_is_sampled_over_the_evaluable_knot_domain() {
    use truck_geometry::prelude::ParametricCurve;
    use truck_modeling::{BSplineCurve, KnotVec};

    let degree = 3;
    let distinct = [
        -0.03125, 0.0, 0.0625, 0.125, 0.1875, 0.25, 0.3125, 0.375, 0.4375, 0.5, 0.5625, 0.625,
        0.6875, 0.75, 0.8125, 0.875, 0.9375, 1.0, 1.0625,
    ];
    let mut knots = Vec::new();
    for k in distinct {
        knots.push(k);
        knots.push(k);
    }
    let knot_vec = KnotVec::from(knots);
    assert_eq!(knot_vec.len(), 2 * distinct.len());
    // A closed control net: the last `degree` points wrap the first `degree`,
    // as STEP's closed curve encoding does.
    let mut ctrl: Vec<Point3> = (0..=30)
        .map(|i| {
            let a = i as f64 / 30.0 * std::f64::consts::TAU;
            Point3::new(5.0 * a.cos(), 3.0 * a.sin(), 0.0)
        })
        .collect();
    let wrap = ctrl[..degree].to_vec();
    ctrl.extend(wrap);
    assert_eq!(knot_vec.len(), ctrl.len() + degree + 1);
    let curve = BSplineCurve::new(knot_vec, ctrl);

    // The representational range extends past the evaluable support.
    assert_eq!(curve.range_tuple(), (-0.03125, 1.0625));
    assert_eq!(curve.evaluation_range(), (0.0, 1.0));
    // The off-support extremes evaluate to the zero vector.
    assert!(curve.subs(-0.03125).to_vec().magnitude() < 1e-12);
    assert!(curve.subs(1.0625).to_vec().magnitude() < 1e-12);
    // The basis certificate is the predicate behind `evaluation_range()`: it
    // is a partition of unity on the interior span and a degenerate partial
    // basis (or all-zero window) in the exporter's closure sliver.
    assert!(curve.basis_is_partition_of_unity(0.0));
    assert!(curve.basis_is_partition_of_unity(0.5));
    assert!(curve.basis_is_partition_of_unity(1.0));
    assert!(!curve.basis_is_partition_of_unity(-0.03125));
    assert!(!curve.basis_is_partition_of_unity(1.0625));
    // Interior samples are genuinely on the curve.
    assert!(curve.subs(0.0).to_vec().magnitude() > 1.0);
    assert!(curve.subs(1.0).to_vec().magnitude() > 1.0);

    // The boundary polyline built over `evaluation_range()` carries the real
    // boundary only: no origin samples, and the two ends are the curve's own
    // values at the corrected domain ends -- the closure point is retained,
    // never dropped and never replaced by the synthetic origin.
    let (er0, er1) = curve.evaluation_range();
    let poly = PolylineCurve::from_curve(&curve, (er0, er1), 0.01);
    assert!(poly.len() > 4, "interior sampling must not collapse");
    assert!(
        poly.iter().all(|p| p.to_vec().magnitude() > 1e-3),
        "no boundary sample may be the synthetic origin"
    );
    assert!(
        poly[0].distance(curve.subs(er0)) < 1e-6
            && poly[poly.len() - 1].distance(curve.subs(er1)) < 1e-6,
        "the polyline ends must be the curve's own domain-end values"
    );
    // The curve's own start/end gap, whatever the exporter's closure precision,
    // is preserved by the sampling rather than amplified to the origin.
    assert!(
        poly[0].distance(poly[poly.len() - 1]) <= curve.subs(er0).distance(curve.subs(er1)) + 1e-9
    );

    // The bare `range_tuple()` extremes would inject the origin endpoints that
    // the projection stage then fails on -- the exact bug being fixed.
    let bad = PolylineCurve::from_curve(&curve, curve.range_tuple(), 0.01);
    assert!(
        bad.iter().any(|p| p.to_vec().magnitude() < 1e-9),
        "the pre-fix range must reproduce the off-support origin sample"
    );
}

/// A closed spline in the ABC `00007705` exporter convention: unclamped end
/// knots (end multiplicity 2, interior multiplicity 1) whose knot vector
/// extends symmetrically past the genuine loop, and a small closed control
/// net. This is the corpus family that once "rendered" a basis-degenerate
/// lens from the sliver. The basis certificate must keep the sampling domain
/// on the interior span where the genuine (tiny) loop lives, never on the
/// sliver.
#[test]
fn closed_spline_with_unclamped_ends_keeps_the_genuine_loop_and_rejects_the_sliver() {
    use truck_geometry::prelude::ParametricCurve;
    use truck_modeling::{BSplineCurve, KnotVec};
    let degree = 3;
    // 37 distinct knots, uniform 0.03125 spacing, from -0.0625 to 1.0625.
    let mut distinct = Vec::new();
    for i in 0..=36 {
        distinct.push(-0.0625 + i as f64 * 0.03125);
    }
    let mut knots = Vec::new();
    knots.push(distinct[0]);
    knots.push(distinct[0]);
    for k in distinct.iter().skip(1).take(35) {
        knots.push(*k);
    }
    knots.push(distinct[36]);
    knots.push(distinct[36]);
    assert_eq!(knots.len(), 39);
    let knot_vec = KnotVec::from(knots);
    // A small closed control net: 32 points on a tiny loop around the vertex
    // (radius ~0.06, in the xz-plane at y = 0.125), then the 3-point wrap.
    let mut ctrl: Vec<Point3> = (0..32)
        .map(|i| {
            let a = i as f64 / 32.0 * std::f64::consts::TAU;
            Point3::new(-1.54 + 0.06 * a.cos(), 0.125, -0.033 + 0.06 * a.sin())
        })
        .collect();
    let wrap = ctrl[..degree].to_vec();
    ctrl.extend_from_slice(&wrap);
    assert_eq!(ctrl.len(), 35);
    let curve = BSplineCurve::new(knot_vec, ctrl);

    // The declared extent runs into the sliver; the evaluable support is the
    // interior knot span where the basis is a partition of unity.
    assert_eq!(curve.range_tuple(), (-0.0625, 1.0625));
    assert_eq!(curve.evaluation_range(), (0.0, 1.0));
    assert!((curve.range_tuple().1 - curve.range_tuple().0 - 1.125).abs() < 1e-12);

    // The basis certificate: genuine on the interior span, degenerate in the
    // exporter's closure sliver.
    for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
        assert!(
            curve.basis_is_partition_of_unity(t),
            "interior {t} must be genuine"
        );
    }
    assert!(!curve.basis_is_partition_of_unity(-0.0625));
    assert!(!curve.basis_is_partition_of_unity(1.0625));
    assert!(!curve.basis_is_partition_of_unity(-0.03));

    // The sliver extremes evaluate to the origin (all-zero basis window).
    assert!(curve.subs(-0.0625).to_vec().magnitude() < 1e-12);
    assert!(curve.subs(1.0625).to_vec().magnitude() < 1e-12);

    // The genuine loop closes over the interior span: both ends are the
    // closure point on the small loop.
    let (er0, er1) = curve.evaluation_range();
    assert!(curve.subs(er0).distance(curve.subs(er1)) < 1e-9);

    // The boundary polyline over the derived (evaluable) domain carries only
    // the genuine tiny loop: every sample stays within the small control hull,
    // none is the synthetic origin, and the loop closes.
    let poly = PolylineCurve::from_curve(&curve, (er0, er1), 1e-3);
    assert!(poly.len() > 4, "the genuine loop must not collapse");
    assert!(
        poly.iter().all(|p| p.to_vec().magnitude() > 1e-3),
        "no boundary sample may be the synthetic origin"
    );
    assert!(
        poly.iter()
            .all(|p| p.distance(Point3::new(-1.54, 0.125, -0.033)) < 0.2),
        "every genuine sample stays on the small loop, not a lens through the origin"
    );

    // Sampling the declared extent injects the origin endpoints the sliver
    // produces -- the exact failure the sampling-domain policy must prevent.
    let bad = PolylineCurve::from_curve(&curve, curve.range_tuple(), 1e-3);
    assert!(
        bad.iter().any(|p| p.to_vec().magnitude() < 1e-9),
        "the declared range must reproduce the sliver origin sample"
    );

    // The derived sampling domain for a closed edge: extend the evaluable
    // core into the declared extent only while the basis certificate holds.
    // The sliver extremes fail the certificate, so the domain stays the
    // interior span.
    let mut derived = curve.evaluation_range();
    let (rt0, rt1) = curve.range_tuple();
    if rt0 < derived.0 - 1.0e-12 && curve.basis_is_partition_of_unity(rt0) {
        derived.0 = rt0;
    }
    if rt1 > derived.1 + 1.0e-12 && curve.basis_is_partition_of_unity(rt1) {
        derived.1 = rt1;
    }
    assert_eq!(derived, (er0, er1));
}

/*
#[test]
#[ignore]
#[cfg(not(target_arch = "wasm32"))]
fn par_bench() {
    use std::time::Instant;
    use truck_modeling::*;
    const JSON: &str = include_str!("../../resources/shape/bottle.json");
    let solid: Solid = serde_json::from_str(JSON).unwrap();
    let shell = solid.into_boundaries().pop().unwrap();

    let instant = Instant::now();
    (0..100).for_each(|_| {
        let _shell = shell_tessellation(&shell, 0.01, by_search_parameter);
    });
    println!("{}ms", instant.elapsed().as_millis());

    let instant = Instant::now();
    (0..100).for_each(|_| {
        let _shell = shell_tessellation_single_thread(&shell, 0.01, by_search_parameter);
    });
    println!("{}ms", instant.elapsed().as_millis());
}
*/

#[cfg(test)]
mod cone_topology_tests {
    use super::*;
    use std::f64::consts::PI;
    use truck_modeling::{Line, Point2, Point3, RevolutedCurve, Vector3};

    fn make_test_cone(r_base: f64, r_apex: f64, h: f64) -> RevolutedCurve<Line<Point3>> {
        let p0 = Point3::new(r_apex, 0.0, 0.0);
        let p1 = Point3::new(r_base, 0.0, h);
        RevolutedCurve::by_revolution(Line(p0, p1), Point3::origin(), Vector3::unit_z())
    }

    #[test]
    fn test_cone_lateral_face_forward_classified() {
        let cone = make_test_cone(10.0, 0.0, 10.0);
        let loop0: Vec<SurfacePoint> = (0..=10)
            .map(|i| {
                let v = (i as f64 / 10.0) * 2.0 * PI;
                let uv = Point2::new(1.0, v);
                (uv, cone.subs(uv.x, uv.y)).into()
            })
            .collect();
        let range = (Some((0.0, 1.0)), None);
        let res = CollapsedPeriodicBoundaryPair::try_classify(
            &cone,
            &[loop0],
            &[],
            range,
            &unevidenced_lattice(&cone),
        );
        assert!(res.is_some());
        let pair = res.unwrap();
        assert!((pair.apex_u - 0.0).abs() < 1e-6);
        assert!((pair.base_u - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_truncated_cone_two_loops_rejected() {
        let cone = make_test_cone(10.0, 5.0, 10.0);
        let loop0: Vec<SurfacePoint> = (0..=10)
            .map(|i| {
                let v = (i as f64 / 10.0) * 2.0 * PI;
                let uv = Point2::new(1.0, v);
                (uv, cone.subs(uv.x, uv.y)).into()
            })
            .collect();
        let loop1: Vec<SurfacePoint> = (0..=10)
            .map(|i| {
                let v = (i as f64 / 10.0) * 2.0 * PI;
                let uv = Point2::new(0.0, v);
                (uv, cone.subs(uv.x, uv.y)).into()
            })
            .collect();
        let range = (Some((0.0, 1.0)), None);
        let res = CollapsedPeriodicBoundaryPair::try_classify(
            &cone,
            &[loop0, loop1],
            &[],
            range,
            &unevidenced_lattice(&cone),
        );
        assert!(res.is_none());
    }

    #[test]
    fn test_cone_sector_with_open_generator_edges_rejected() {
        let cone = make_test_cone(10.0, 0.0, 10.0);
        let loop0: Vec<SurfacePoint> = (0..=10)
            .map(|i| {
                let v = (i as f64 / 10.0) * PI;
                let uv = Point2::new(1.0, v);
                (uv, cone.subs(uv.x, uv.y)).into()
            })
            .collect();
        let open_edge: Vec<SurfacePoint> = vec![
            (Point2::new(0.0, 0.0), cone.subs(0.0, 0.0)).into(),
            (Point2::new(1.0, 0.0), cone.subs(1.0, 0.0)).into(),
        ];
        let range = (Some((0.0, 1.0)), None);
        let res = CollapsedPeriodicBoundaryPair::try_classify(
            &cone,
            &[loop0],
            &[open_edge],
            range,
            &unevidenced_lattice(&cone),
        );
        assert!(res.is_none());
    }

    #[test]
    fn test_zero_area_non_cone_cylinder_rejected() {
        let cylinder = RevolutedCurve::by_revolution(
            Line(Point3::new(10.0, 0.0, 0.0), Point3::new(10.0, 0.0, 10.0)),
            Point3::origin(),
            Vector3::unit_z(),
        );
        let loop0: Vec<SurfacePoint> = (0..=10)
            .map(|i| {
                let v = (i as f64 / 10.0) * 2.0 * PI;
                let uv = Point2::new(1.0, v);
                (uv, cylinder.subs(uv.x, uv.y)).into()
            })
            .collect();
        let range = (Some((0.0, 1.0)), None);
        let res = CollapsedPeriodicBoundaryPair::try_classify(
            &cylinder,
            &[loop0],
            &[],
            range,
            &unevidenced_lattice(&cylinder),
        );
        assert!(res.is_none());
    }

    /// A cylindrical band presented the way STEP presents one: two boundary
    /// circles winding **opposite** ways, as they must for the face boundary to
    /// be coherently oriented.
    fn opposite_winding_band_pieces() -> (RevolutedCurve<Line<Point3>>, Vec<PolyBoundaryPiece>) {
        let cylinder = RevolutedCurve::by_revolution(
            Line(Point3::new(10.0, 0.0, 0.0), Point3::new(10.0, 0.0, 10.0)),
            Point3::origin(),
            Vector3::unit_z(),
        );
        let circle = |u: f64, sign: f64| -> PolyBoundaryPiece {
            PolyBoundaryPiece::untagged(
                (0..=32)
                    .map(|i| {
                        let v = sign * (i as f64 / 32.0) * 2.0 * PI;
                        let uv = Point2::new(u, v);
                        (uv, cylinder.subs(uv.x, uv.y)).into()
                    })
                    .collect(),
            )
        };
        (cylinder, vec![circle(0.2, 1.0), circle(0.8, -1.0)])
    }

    /// TEMP DEBUG: dump the opposite-winding band CDT for legacy vs corrected.

    /// The deck equation, on the geometry it was written for: reversing loop 1
    /// gives `Î£Î´ = Â±2`, so the legacy join is refused and forward traversal is
    /// named as the unique solution.
    #[test]
    fn opposite_winding_band_selects_forward_traversal() {
        let (cylinder, pieces) = opposite_winding_band_pieces();
        let lattice = unevidenced_lattice(&cylinder);
        let (_, legacy) = PolyBoundary::new_with_join(
            pieces.clone(),
            &cylinder,
            0.01,
            &lattice,
            TwoLoopJoinPolicy::Legacy,
        );
        assert_eq!(
            legacy,
            TwoLoopJoinOutcome::ForwardResolves { applied: false },
            "the legacy policy diagnoses the case but does not act on it",
        );
        let (_, corrected) = PolyBoundary::new_with_join(
            pieces,
            &cylinder,
            0.01,
            &lattice,
            TwoLoopJoinPolicy::DeckConsistent,
        );
        assert_eq!(
            corrected,
            TwoLoopJoinOutcome::ForwardResolves { applied: true },
            "the deck-consistent policy takes the solution",
        );
    }

    /// The corrected join makes the band tessellate under the deck-consistent
    /// policy, and PLANAR-C now recovers the *legacy* traversal the same way:
    /// its two crossing bridges are proper interior crossings, so
    /// `add_constraint_and_split` planarizes them instead of refusing, and the
    /// material region comes out identical to the corrected traversal.
    ///
    /// `PolyBoundary::new` now runs the join under `DeckConsistent` (the
    /// primary rendered-face path), so the explicit `Legacy` reference is used
    /// to demonstrate the planarized legacy traversal selects the same band.
    #[test]
    fn opposite_winding_band_tessellates_only_when_deck_consistent() {
        let (cylinder, pieces) = opposite_winding_band_pieces();
        let lattice = unevidenced_lattice(&cylinder);
        let (legacy, _) = PolyBoundary::new_with_join(
            pieces.clone(),
            &cylinder,
            0.01,
            &lattice,
            TwoLoopJoinPolicy::Legacy,
        );
        let legacy_mesh = trimming_tessellation_result(&cylinder, &legacy, 0.01, &lattice)
            .expect("PLANAR-C planarizes the crossing bridges instead of refusing");
        let (corrected, _) = PolyBoundary::new_with_join(
            pieces,
            &cylinder,
            0.01,
            &lattice,
            TwoLoopJoinPolicy::DeckConsistent,
        );
        let mesh = trimming_tessellation_result(&cylinder, &corrected, 0.01, &lattice)
            .expect("the deck-consistent boundary tessellates");
        assert!(!mesh.tri_faces().is_empty(), "and produces triangles");
        assert_eq!(
            legacy_mesh.tri_faces().len(),
            mesh.tri_faces().len(),
            "the planarized legacy traversal selects the same band region as the corrected one",
        );
    }

    /// Two loops winding the *same* way need the reversal, and must not be
    /// disturbed: the equation selects the legacy traversal there.
    #[test]
    fn same_winding_band_keeps_the_legacy_traversal() {
        let cylinder = RevolutedCurve::by_revolution(
            Line(Point3::new(10.0, 0.0, 0.0), Point3::new(10.0, 0.0, 10.0)),
            Point3::origin(),
            Vector3::unit_z(),
        );
        let circle = |u: f64| -> PolyBoundaryPiece {
            PolyBoundaryPiece::untagged(
                (0..=32)
                    .map(|i| {
                        let v = (i as f64 / 32.0) * 2.0 * PI;
                        let uv = Point2::new(u, v);
                        (uv, cylinder.subs(uv.x, uv.y)).into()
                    })
                    .collect(),
            )
        };
        let (_, outcome) = PolyBoundary::new_with_join(
            vec![circle(0.2), circle(0.8)],
            &cylinder,
            0.01,
            &unevidenced_lattice(&cylinder),
            TwoLoopJoinPolicy::DeckConsistent,
        );
        assert_eq!(outcome, TwoLoopJoinOutcome::LegacyDeckConsistent);
    }

    /// The synthetic closure of an open boundary piece is bounded by the
    /// face-local working range, never by the carrier surface's declared
    /// parameter range.
    ///
    /// A trimmed face can present one open piece plus one Euclidean-closed loop
    /// while the supporting surface's declared range is materially larger than
    /// the face's own extent. Closing the open piece against the declared
    /// rectangle walks synthetic segments to corners no face boundary point
    /// approaches; that inflated closure expands the interior sampling domain
    /// `insert_surface` subdivides and hands the CDT pathological constraint
    /// geometry (the UR10 `#88144`/`#89705` mechanism).
    #[test]
    fn open_piece_closure_uses_face_local_range_not_declared_range() {
        use truck_geometry::prelude::*;
        // Declared range is `[0, 1] × [0, 1]` (Plane::parameter_range), which is
        // materially larger than the face-local extent `[0.2, 0.8] × [0.2, 0.8]`
        // the two pieces below actually span.
        let plane = Plane::new(
            Point3::origin(),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        );
        let tol = 0.01;
        let lattice = unevidenced_lattice(&plane);
        // Open piece: a diagonal run whose UV endpoints do not coincide, so it
        // is classified `Open` and must be closed synthetically.
        let open_piece = PolyBoundaryPiece::untagged(vec![
            (Point2::new(0.2, 0.2), Point3::new(0.2, 0.2, 0.0)).into(),
            (Point2::new(0.5, 0.5), Point3::new(0.5, 0.5, 0.0)).into(),
            (Point2::new(0.8, 0.8), Point3::new(0.8, 0.8, 0.0)).into(),
        ]);
        // Euclidean-closed piece: a small loop whose first UV equals its last,
        // so it is classified `EuclideanClosed` and needs no synthetic closure.
        let closed_piece = PolyBoundaryPiece::untagged(vec![
            (Point2::new(0.3, 0.3), Point3::new(0.3, 0.3, 0.0)).into(),
            (Point2::new(0.7, 0.3), Point3::new(0.7, 0.3, 0.0)).into(),
            (Point2::new(0.7, 0.7), Point3::new(0.7, 0.7, 0.0)).into(),
            (Point2::new(0.3, 0.7), Point3::new(0.3, 0.7, 0.0)).into(),
            (Point2::new(0.3, 0.3), Point3::new(0.3, 0.3, 0.0)).into(),
        ]);
        let boundary = PolyBoundary::new(vec![open_piece, closed_piece], &plane, tol, &lattice);
        // The open piece must have been closed synthetically: exactly two loops
        // result, and the merged one carries SyntheticClosure segments.
        assert_eq!(
            boundary.0.len(),
            2,
            "the open piece is closed into a second loop",
        );
        let merged = boundary
            .0
            .iter()
            .find(|loop_| {
                loop_
                    .origins
                    .iter()
                    .any(|o| *o == SegmentOrigin::SyntheticClosure)
            })
            .expect("the open piece's closure is synthetic");
        assert!(
            merged.origins.iter().any(|o| *o == SegmentOrigin::Source),
            "the open piece's own segments keep their Source role",
        );
        // Every loop satisfies the BoundaryLoop equal-length invariant.
        for loop_ in &boundary.0 {
            assert_eq!(
                loop_.points.len(),
                loop_.origins.len(),
                "every boundary segment carries exactly one origin",
            );
            assert_eq!(
                loop_.points.len(),
                loop_.source_uses.len(),
                "every boundary segment carries exactly one provenance entry",
            );
        }
        // No synthetic closure vertex may leave the face-derived working extent
        // [0.2, 0.8] × [0.2, 0.8]. Walking the declared-range rectangle would
        // place vertices at the corners (0, 0), (0, 1), (1, 0), (1, 1) that no
        // face boundary point approaches.
        for loop_ in &boundary.0 {
            for p in &loop_.points {
                let (u, v) = (p.uv.x, p.uv.y);
                assert!(
                    u >= 0.1 && u <= 0.9 && v >= 0.1 && v <= 0.9,
                    "synthetic closure escapes the face-local extent: uv=({u}, {v})",
                );
            }
        }
    }

    #[test]
    fn test_traversal_semantics_periodic_circle() {
        use crate::tessellation::domain::projection::TraversalSemantics;
        use truck_geometry::prelude::*;
        let circle = UnitCircle::<Point3>::new();
        let cone = make_test_cone(10.0, 0.0, 10.0);
        let semantics = TraversalSemantics::resolve(&circle, &cone, 1e-4);
        assert!(matches!(semantics, TraversalSemantics::FullPeriod { .. }));
    }

    #[test]
    fn test_traversal_semantics_degenerate_point() {
        use crate::tessellation::domain::projection::TraversalSemantics;
        use truck_geometry::prelude::*;
        let line = Line(Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 0.0, 0.0));
        let cone = make_test_cone(10.0, 0.0, 10.0);
        let semantics = TraversalSemantics::resolve(&line, &cone, 1e-4);
        assert_eq!(semantics, TraversalSemantics::DegeneratePoint);
    }

    #[test]
    fn test_shared_boundary_projection_processor_wrapped_cone() {
        use crate::tessellation::domain::projection::{project_boundary_curve, TraversalSemantics};
        use truck_geometry::prelude::*;
        let cone = make_test_cone(10.0, 0.0, 10.0);
        let p_uv = cone.subs(0.5, 0.0);
        let tr = Matrix4::from_translation(Vector3::new(10.0, 20.0, 30.0));
        let proc_cone = Processor::with_transform(cone, tr);
        let circle = Processor::with_transform(TrimmedCurve::new(Line(p_uv, p_uv), (0.0, 1.0)), tr);
        let sem = TraversalSemantics::resolve(&circle, &proc_cone, 1e-4);
        let path = project_boundary_curve(&circle, &proc_cone, sem, 1.0).unwrap();
        assert!(!path.samples.is_empty());
    }

    #[test]
    fn test_periodic_closure_cone_tessellation() {
        let cone = make_test_cone(10.0, 0.0, 10.0);
        let tol = 0.1;
        let n = 32usize;
        let circle_pts: Vec<SurfacePoint> = (0..=n)
            .map(|i| {
                let v = (i as f64 / n as f64) * 2.0 * PI;
                let uv = Point2::new(1.0, v);
                (uv, cone.subs(uv.x, uv.y)).into()
            })
            .collect();
        let piece = PolyBoundaryPiece::untagged(circle_pts);
        let boundary = PolyBoundary::new(vec![piece], &cone, tol, &unevidenced_lattice(&cone));
        let mesh = trimming_tessellation(&cone, &boundary, tol, &unevidenced_lattice(&cone));
        assert!(
            !mesh.faces().is_empty(),
            "Cone C-APEX-DISK face must tessellate to non-empty mesh"
        );
    }

    #[test]
    fn test_parity_single_disk() {
        use truck_geometry::prelude::*;
        let plane = Plane::new(
            Point3::origin(),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        );
        let tol = 0.01;
        let loop0: Vec<SurfacePoint> = vec![
            (Point2::new(0.0, 0.0), Point3::new(0.0, 0.0, 0.0)).into(),
            (Point2::new(10.0, 0.0), Point3::new(10.0, 0.0, 0.0)).into(),
            (Point2::new(10.0, 10.0), Point3::new(10.0, 10.0, 0.0)).into(),
            (Point2::new(0.0, 10.0), Point3::new(0.0, 10.0, 0.0)).into(),
            (Point2::new(0.0, 0.0), Point3::new(0.0, 0.0, 0.0)).into(),
        ];
        let boundary = PolyBoundary::new(
            vec![PolyBoundaryPiece::untagged(loop0)],
            &plane,
            tol,
            &unevidenced_lattice(&plane),
        );
        let mesh = trimming_tessellation(&plane, &boundary, tol, &unevidenced_lattice(&plane));
        assert!(!mesh.faces().is_empty());
    }

    #[test]
    fn test_parity_disk_with_hole() {
        use truck_geometry::prelude::*;
        let plane = Plane::new(
            Point3::origin(),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        );
        let tol = 0.01;
        let outer: Vec<SurfacePoint> = vec![
            (Point2::new(0.0, 0.0), Point3::new(0.0, 0.0, 0.0)).into(),
            (Point2::new(10.0, 0.0), Point3::new(10.0, 0.0, 0.0)).into(),
            (Point2::new(10.0, 10.0), Point3::new(10.0, 10.0, 0.0)).into(),
            (Point2::new(0.0, 10.0), Point3::new(0.0, 10.0, 0.0)).into(),
            (Point2::new(0.0, 0.0), Point3::new(0.0, 0.0, 0.0)).into(),
        ];
        let hole: Vec<SurfacePoint> = vec![
            (Point2::new(3.0, 3.0), Point3::new(3.0, 3.0, 0.0)).into(),
            (Point2::new(7.0, 3.0), Point3::new(7.0, 3.0, 0.0)).into(),
            (Point2::new(7.0, 7.0), Point3::new(7.0, 7.0, 0.0)).into(),
            (Point2::new(3.0, 7.0), Point3::new(3.0, 7.0, 0.0)).into(),
            (Point2::new(3.0, 3.0), Point3::new(3.0, 3.0, 0.0)).into(),
        ];
        let boundary = PolyBoundary::new(
            vec![
                PolyBoundaryPiece::untagged(outer),
                PolyBoundaryPiece::untagged(hole),
            ],
            &plane,
            tol,
            &unevidenced_lattice(&plane),
        );
        let mesh = trimming_tessellation(&plane, &boundary, tol, &unevidenced_lattice(&plane));
        assert!(!mesh.faces().is_empty());
        // Verify hole centroid (5.0, 5.0) has no triangle containing it
        for tri in mesh.faces().tri_faces() {
            let p0 = mesh.uv_coords()[tri[0].pos];
            let p1 = mesh.uv_coords()[tri[1].pos];
            let p2 = mesh.uv_coords()[tri[2].pos];
            let center = (p0 + p1 + p2) / 3.0;
            assert!(
                !(center.x > 3.1 && center.x < 6.9 && center.y > 3.1 && center.y < 6.9),
                "Hole interior must not contain triangles"
            );
        }
    }

    #[test]
    fn test_parity_concave_outer_loop() {
        use truck_geometry::prelude::*;
        let plane = Plane::new(
            Point3::origin(),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        );
        let tol = 0.01;
        // L-shaped concave outer loop
        let loop0: Vec<SurfacePoint> = vec![
            (Point2::new(0.0, 0.0), Point3::new(0.0, 0.0, 0.0)).into(),
            (Point2::new(10.0, 0.0), Point3::new(10.0, 0.0, 0.0)).into(),
            (Point2::new(10.0, 5.0), Point3::new(10.0, 5.0, 0.0)).into(),
            (Point2::new(5.0, 5.0), Point3::new(5.0, 5.0, 0.0)).into(),
            (Point2::new(5.0, 10.0), Point3::new(5.0, 10.0, 0.0)).into(),
            (Point2::new(0.0, 10.0), Point3::new(0.0, 10.0, 0.0)).into(),
            (Point2::new(0.0, 0.0), Point3::new(0.0, 0.0, 0.0)).into(),
        ];
        let boundary = PolyBoundary::new(
            vec![PolyBoundaryPiece::untagged(loop0)],
            &plane,
            tol,
            &unevidenced_lattice(&plane),
        );
        let mesh = trimming_tessellation(&plane, &boundary, tol, &unevidenced_lattice(&plane));
        assert!(!mesh.faces().is_empty());
    }

    #[test]
    fn test_parity_intersecting_constraints_rejected() {
        use truck_geometry::prelude::*;
        let plane = Plane::new(
            Point3::origin(),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        );
        let tol = 0.01;
        // Self-overlapping loop traversing same segment (0,0)->(10,0) twice in forward direction
        let loop0: Vec<SurfacePoint> = vec![
            (Point2::new(0.0, 0.0), Point3::new(0.0, 0.0, 0.0)).into(),
            (Point2::new(10.0, 0.0), Point3::new(10.0, 0.0, 0.0)).into(),
            (Point2::new(0.0, 0.0), Point3::new(0.0, 0.0, 0.0)).into(),
            (Point2::new(10.0, 0.0), Point3::new(10.0, 0.0, 0.0)).into(),
            (Point2::new(10.0, 10.0), Point3::new(10.0, 10.0, 0.0)).into(),
            (Point2::new(0.0, 10.0), Point3::new(0.0, 10.0, 0.0)).into(),
            (Point2::new(0.0, 0.0), Point3::new(0.0, 0.0, 0.0)).into(),
        ];
        let boundary = PolyBoundary::new(
            vec![PolyBoundaryPiece::untagged(loop0)],
            &plane,
            tol,
            &unevidenced_lattice(&plane),
        );
        let mesh = trimming_tessellation(&plane, &boundary, tol, &unevidenced_lattice(&plane));
        assert!(
            mesh.faces().is_empty(),
            "Self-overlapping degenerate loop must fail or produce empty mesh"
        );
    }

    // -----------------------------------------------------------------------
    // ARR-SEAM W2 â€” the widened DeckConsistent two-loop join
    // -----------------------------------------------------------------------

    /// Two periodic walks whose loops enclose genuine UV area, unlike the
    /// degenerate fixed-`u` circles of [`opposite_winding_band_pieces`]. They
    /// wind opposite ways, so the deck equation resolves forward.
    ///
    /// The `u` coordinate bulges out and back along the `v` walk, so the lifted
    /// loop encloses `|signed_area| â‰ˆ 2 Â· amp Â· Ï€`, far above the
    /// `DEGENERATE_LOOP_AREA` gate, while still closing in 3D at `v = 0` and
    /// `v = 2Ï€`.
    fn non_degenerate_band_pieces() -> (RevolutedCurve<Line<Point3>>, Vec<PolyBoundaryPiece>) {
        let cylinder = RevolutedCurve::by_revolution(
            Line(Point3::new(10.0, 0.0, 0.0), Point3::new(10.0, 0.0, 10.0)),
            Point3::origin(),
            Vector3::unit_z(),
        );
        let bump_circle = |u0: f64, amp: f64, sign: f64| -> PolyBoundaryPiece {
            PolyBoundaryPiece::untagged(
                (0..=32)
                    .map(|i| {
                        let t = i as f64 / 32.0;
                        let v = sign * t * 2.0 * PI;
                        let u = u0 + amp * 2.0 * t.min(1.0 - t);
                        let uv = Point2::new(u, v);
                        (uv, cylinder.subs(uv.x, uv.y)).into()
                    })
                    .collect(),
            )
        };
        (
            cylinder,
            vec![bump_circle(0.2, 0.05, 1.0), bump_circle(0.8, 0.05, -1.0)],
        )
    }

    /// A non-degenerate deck pair is joined under `DeckConsistent`.
    #[test]
    fn non_degenerate_deck_pair_is_joined_under_deck_consistent() {
        let (cylinder, pieces) = non_degenerate_band_pieces();
        let lattice = unevidenced_lattice(&cylinder);
        let (_, outcome) = PolyBoundary::new_with_join(
            pieces,
            &cylinder,
            0.01,
            &lattice,
            TwoLoopJoinPolicy::DeckConsistent,
        );
        assert_eq!(
            outcome,
            TwoLoopJoinOutcome::ForwardResolves { applied: true },
            "a non-degenerate deck pair must be joined under DeckConsistent",
        );
    }

    /// The same pair is **not** joined under `Legacy` â€” INV-W2-1, the regression
    /// guard for the widened gate.
    #[test]
    fn non_degenerate_deck_pair_is_not_joined_under_legacy() {
        let (cylinder, pieces) = non_degenerate_band_pieces();
        let lattice = unevidenced_lattice(&cylinder);
        let (_, outcome) = PolyBoundary::new_with_join(
            pieces,
            &cylinder,
            0.01,
            &lattice,
            TwoLoopJoinPolicy::Legacy,
        );
        assert_eq!(
            outcome,
            TwoLoopJoinOutcome::NotAttempted,
            "the legacy policy must not admit a non-degenerate deck pair",
        );
    }

    /// The first-pass classifier routes the certified structural deck-pair class
    /// through `DeckConsistent` on `PolyBoundary::new` — the production entry —
    /// so a non-degenerate full-period deck pair is joined without waiting for a
    /// legacy failure to open the recovery arm. `Legacy` would leave it as two
    /// separate closed loops.
    #[test]
    fn non_degenerate_deck_pair_gets_deck_consistent_on_the_primary_path() {
        let (cylinder, pieces) = non_degenerate_band_pieces();
        let lattice = unevidenced_lattice(&cylinder);
        let boundary = PolyBoundary::new(pieces, &cylinder, 0.01, &lattice);
        assert_eq!(
            boundary.0.len(),
            1,
            "the primary path must join a certified deck pair (INV-W2-1 widened on first pass)",
        );
    }

    /// INV-W2-2: the joined loop contains no segment spanning a full period.
    #[test]
    fn joined_loop_has_no_period_spanning_segment() {
        let (cylinder, pieces) = non_degenerate_band_pieces();
        let lattice = unevidenced_lattice(&cylinder);
        let (boundary, outcome) = PolyBoundary::new_with_join(
            pieces,
            &cylinder,
            0.01,
            &lattice,
            TwoLoopJoinPolicy::DeckConsistent,
        );
        assert_eq!(
            outcome,
            TwoLoopJoinOutcome::ForwardResolves { applied: true }
        );
        assert_eq!(boundary.0.len(), 1, "the join yields one closed loop");
        let period = lattice.declared_v_period().unwrap_or(2.0 * PI);
        let loop_ = &boundary.0[0];
        for k in 0..loop_.points.len() {
            let a = loop_.points[k].uv;
            let b = loop_.points[(k + 1) % loop_.points.len()].uv;
            assert!(
                a.distance(b) < period,
                "no joined segment may span a full parameter period (INV-W2-2)",
            );
        }
    }

    /// INV-W2-3: exactly the two chart-edge bridges carry `Seam`.
    #[test]
    fn joined_loop_has_exactly_two_seam_origins() {
        let (cylinder, pieces) = non_degenerate_band_pieces();
        let lattice = unevidenced_lattice(&cylinder);
        let (boundary, outcome) = PolyBoundary::new_with_join(
            pieces,
            &cylinder,
            0.01,
            &lattice,
            TwoLoopJoinPolicy::DeckConsistent,
        );
        assert_eq!(
            outcome,
            TwoLoopJoinOutcome::ForwardResolves { applied: true }
        );
        assert_eq!(boundary.0.len(), 1);
        let loop_ = &boundary.0[0];
        let seams = loop_
            .origins
            .iter()
            .filter(|origin| **origin == SegmentOrigin::Seam)
            .count();
        assert_eq!(
            seams, 2,
            "exactly the two artificial bridges are Seam (INV-W2-3)",
        );
    }

    /// INV-W2-4: a zero-displacement (Euclidean-closed) pair is not admitted by
    /// the deck-pair disjunct.
    #[test]
    fn zero_displacement_pair_is_refused() {
        let cylinder = RevolutedCurve::by_revolution(
            Line(Point3::new(10.0, 0.0, 0.0), Point3::new(10.0, 0.0, 10.0)),
            Point3::origin(),
            Vector3::unit_z(),
        );
        // Two ordinary UV-closed circles on the periodic chart, each enclosing
        // area >> 1e-4. Their displacements are [0, 0], so neither the legacy
        // area gate nor the deck-pair disjunct fires.
        let circle = |u: f64| -> PolyBoundaryPiece {
            PolyBoundaryPiece::untagged(
                (0..=32)
                    .map(|i| {
                        let theta = (i as f64 / 32.0) * 2.0 * PI;
                        let uv = Point2::new(u + 0.2 * theta.cos(), PI + 0.2 * theta.sin());
                        (uv, cylinder.subs(uv.x, uv.y)).into()
                    })
                    .collect(),
            )
        };
        let (_, outcome) = PolyBoundary::new_with_join(
            vec![circle(0.4), circle(0.6)],
            &cylinder,
            0.01,
            &unevidenced_lattice(&cylinder),
            TwoLoopJoinPolicy::DeckConsistent,
        );
        assert_eq!(
            outcome,
            TwoLoopJoinOutcome::NotAttempted,
            "a zero-displacement pair must not be admitted (INV-W2-4)",
        );
    }

    /// INV-W2-4: an inconsistent deck equation is refused. Reproduces
    /// `00009190#34764` with `d0 = (0, 2)`, `d1 = (0, -1)`.
    #[test]
    fn inconsistent_displacements_are_refused() {
        let cylinder = RevolutedCurve::by_revolution(
            Line(Point3::new(10.0, 0.0, 0.0), Point3::new(10.0, 0.0, 10.0)),
            Point3::origin(),
            Vector3::unit_z(),
        );
        let winding_circle = |u0: f64, windings: i64| -> PolyBoundaryPiece {
            PolyBoundaryPiece::untagged(
                (0..=32)
                    .map(|i| {
                        let t = i as f64 / 32.0;
                        let v = (windings as f64) * t * 2.0 * PI;
                        let u = u0 + 0.05 * 2.0 * t.min(1.0 - t);
                        let uv = Point2::new(u, v);
                        (uv, cylinder.subs(uv.x, uv.y)).into()
                    })
                    .collect(),
            )
        };
        let (_, outcome) = PolyBoundary::new_with_join(
            vec![winding_circle(0.2, 2), winding_circle(0.8, -1)],
            &cylinder,
            0.01,
            &unevidenced_lattice(&cylinder),
            TwoLoopJoinPolicy::DeckConsistent,
        );
        assert_eq!(
            outcome,
            TwoLoopJoinOutcome::Inconsistent,
            "the (0,2)/(0,-1) deck is inconsistent and must be refused",
        );
    }

    /// End to end: the joined non-degenerate band tessellates to a non-empty
    /// mesh. This is the residual risk W2 accepts â€” the join is unchanged, the
    /// population is not.
    #[test]
    fn joined_band_tessellates_to_a_nonempty_mesh() {
        let (cylinder, pieces) = non_degenerate_band_pieces();
        let lattice = unevidenced_lattice(&cylinder);
        let (boundary, outcome) = PolyBoundary::new_with_join(
            pieces,
            &cylinder,
            0.01,
            &lattice,
            TwoLoopJoinPolicy::DeckConsistent,
        );
        assert_eq!(
            outcome,
            TwoLoopJoinOutcome::ForwardResolves { applied: true }
        );
        let mesh = trimming_tessellation_result(&cylinder, &boundary, 0.01, &lattice)
            .expect("the joined non-degenerate band tessellates");
        assert!(
            !mesh.faces().tri_faces().is_empty(),
            "and produces triangles",
        );
    }

    // -----------------------------------------------------------------------
    // BOW-TIE-ORIENTATION â€” the primary path resolves two-loop joins against
    // the deck equation, so the synthetic seam bridges are the two non-crossing
    // sides of the periodic chart.
    // -----------------------------------------------------------------------

    /// The two synthetic seam bridges of a joined loop, as UV segments.
    ///
    /// A two-loop join produces exactly two `Seam`-origin bridges (INV-W2-3);
    /// this extracts them so a test can assert the chart-closure invariant:
    /// the two bridges must not have a proper interior crossing.
    fn joined_seam_bridges(boundary: &PolyBoundary) -> Vec<[(f64, f64); 2]> {
        let mut bridges = Vec::new();
        for loop_ in &boundary.0 {
            let points = &loop_.points;
            assert_eq!(points.len(), loop_.origins.len());
            for (i, origin) in loop_.origins.iter().enumerate() {
                if *origin == SegmentOrigin::Seam {
                    let a = points[i].uv;
                    let b = points[(i + 1) % points.len()].uv;
                    bridges.push([(a.x, a.y), (b.x, b.y)]);
                }
            }
        }
        bridges
    }

    /// Whether two UV segments have a *proper* interior crossing: an
    /// intersection point strictly inside both open segments. Shared endpoints
    /// (the rectangle corners the two bridges legitimately share) and
    /// collinear overlaps are not proper crossings.
    fn segments_properly_cross(a: [(f64, f64); 2], b: [(f64, f64); 2]) -> bool {
        let orient = |p: (f64, f64), q: (f64, f64), r: (f64, f64)| {
            (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0)
        };
        let d1 = orient(a[0], a[1], b[0]);
        let d2 = orient(a[0], a[1], b[1]);
        let d3 = orient(b[0], b[1], a[0]);
        let d4 = orient(b[0], b[1], a[1]);
        (d1 * d2 < 0.0) && (d3 * d4 < 0.0)
    }

    /// The chart-closure invariant: the two synthetic seam bridges of a joined
    /// periodic band must be the two non-crossing sides of the chart, so no
    /// proper interior crossing exists and no synthetic chart-centre vertex is
    /// required. This is the assertion the bow-tie defect violates.
    fn assert_seam_bridges_do_not_cross(boundary: &PolyBoundary) {
        let bridges = joined_seam_bridges(boundary);
        assert_eq!(
            bridges.len(),
            2,
            "the join produces exactly two seam bridges"
        );
        assert!(
            !segments_properly_cross(bridges[0], bridges[1]),
            "the two synthetic seam bridges must not have a proper interior crossing \
             (the bow-tie invariant): {bridges:?}",
        );
    }

    /// Test A â€” the `(false, true)` / opposite-displacement population
    /// (`loop0_disp=[0,1]`, `loop1_disp=[0,-1]`, the `ftc_08 #5921` class).
    ///
    /// The primary path must select the forward traversal (the unique deck
    /// solution), so the bridges are the two rectangle sides rather than the
    /// crossing diagonals, and no chart-centre synthetic vertex is required.
    #[test]
    fn opposite_displacement_primary_path_joins_without_crossing() {
        let (cylinder, pieces) = opposite_winding_band_pieces();
        let lattice = unevidenced_lattice(&cylinder);
        let boundary = PolyBoundary::new(pieces, &cylinder, 0.01, &lattice);
        assert_eq!(boundary.0.len(), 1, "the join yields one closed loop");
        assert_seam_bridges_do_not_cross(&boundary);
        let mesh = trimming_tessellation_result(&cylinder, &boundary, 0.01, &lattice)
            .expect("the deck-consistent primary boundary tessellates");
        assert!(!mesh.tri_faces().is_empty(), "and produces triangles");
    }

    /// Test B â€“ the `(true, false)` / same-displacement population
    /// (`loop0_disp == loop1_disp`). The deck equation selects the reversed
    /// traversal there, so the primary path must keep the legacy reversal and
    /// the bridges must again be non-crossing.
    #[test]
    fn same_displacement_primary_path_joins_without_crossing() {
        let cylinder = RevolutedCurve::by_revolution(
            Line(Point3::new(10.0, 0.0, 0.0), Point3::new(10.0, 0.0, 10.0)),
            Point3::origin(),
            Vector3::unit_z(),
        );
        let circle = |u: f64| -> PolyBoundaryPiece {
            PolyBoundaryPiece::untagged(
                (0..=32)
                    .map(|i| {
                        let v = (i as f64 / 32.0) * 2.0 * PI;
                        let uv = Point2::new(u, v);
                        (uv, cylinder.subs(uv.x, uv.y)).into()
                    })
                    .collect(),
            )
        };
        let lattice = unevidenced_lattice(&cylinder);
        let boundary = PolyBoundary::new(vec![circle(0.2), circle(0.8)], &cylinder, 0.01, &lattice);
        assert_eq!(boundary.0.len(), 1, "the join yields one closed loop");
        assert_seam_bridges_do_not_cross(&boundary);
        let mesh = trimming_tessellation_result(&cylinder, &boundary, 0.01, &lattice)
            .expect("the same-displacement band tessellates");
        assert!(!mesh.tri_faces().is_empty(), "and produces triangles");
    }

    /// Test C â€” a representative previously-correct two-loop join (the
    /// non-degenerate deck pair) must keep its emitted boundary topology: the
    /// primary path still resolves forward and the band still tessellates to a
    /// non-empty mesh with exactly the two seam bridges.
    #[test]
    fn non_degenerate_band_primary_path_keeps_valid_topology() {
        let (cylinder, pieces) = non_degenerate_band_pieces();
        let lattice = unevidenced_lattice(&cylinder);
        let boundary = PolyBoundary::new(pieces, &cylinder, 0.01, &lattice);
        assert_eq!(boundary.0.len(), 1, "the join yields one closed loop");
        assert_seam_bridges_do_not_cross(&boundary);
        let mesh = trimming_tessellation_result(&cylinder, &boundary, 0.01, &lattice)
            .expect("the non-degenerate deck pair tessellates");
        assert!(!mesh.tri_faces().is_empty(), "and produces triangles");
    }

    /// Test D â€“ the invariant for a legitimate simple periodic two-loop band:
    /// the two synthetic closure segments never have a proper interior
    /// intersection, for both resolved orientation classes.
    #[test]
    fn seam_closure_segments_never_properly_cross() {
        let (cylinder, opposite) = opposite_winding_band_pieces();
        let boundary =
            PolyBoundary::new(opposite, &cylinder, 0.01, &unevidenced_lattice(&cylinder));
        assert_seam_bridges_do_not_cross(&boundary);
    }

    // -----------------------------------------------------------------------
    // ARR-SEAM W3 â€” duplicate traversal, multiplicity mod 2
    // -----------------------------------------------------------------------

    fn parity_plane() -> truck_geometry::prelude::Plane {
        use truck_geometry::prelude::*;
        Plane::new(
            Point3::origin(),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        )
    }

    fn square(visits: u32) -> Vec<SurfacePoint> {
        let corner =
            |x: f64, y: f64| -> SurfacePoint { (Point2::new(x, y), Point3::new(x, y, 0.0)).into() };
        (0..visits)
            .flat_map(|_| {
                [
                    corner(0.0, 0.0),
                    corner(10.0, 0.0),
                    corner(10.0, 10.0),
                    corner(0.0, 10.0),
                ]
            })
            .chain([corner(0.0, 0.0)])
            .collect()
    }

    /// C1's regression test: a single-traversal disk still bounds material under
    /// `TraversalParity` (identical to `Legacy` when every edge is traversed
    /// once).
    #[test]
    fn single_traversal_still_bounds_material() {
        let plane = parity_plane();
        let tol = 0.01;
        let boundary = PolyBoundary::new(
            vec![PolyBoundaryPiece::untagged(square(1))],
            &plane,
            tol,
            &unevidenced_lattice(&plane),
        );
        let mesh = trimming_tessellation(&plane, &boundary, tol, &unevidenced_lattice(&plane));
        assert!(!mesh.faces().is_empty());
    }

    /// A boundary declared twice cancels to no material and reports
    /// `NoOddParityRegion` â€” never `ConstraintOverlapUnsupported`.
    #[test]
    fn double_traversal_cancels() {
        let plane = parity_plane();
        let tol = 0.01;
        let boundary = PolyBoundary::new(
            vec![PolyBoundaryPiece::untagged(square(2))],
            &plane,
            tol,
            &unevidenced_lattice(&plane),
        );
        let reason =
            trimming_tessellation_result(&plane, &boundary, tol, &unevidenced_lattice(&plane))
                .err()
                .map(|failure| failure.reason);
        assert_eq!(
            reason,
            Some(TessellationFailureReason::NoOddParityRegion),
            "a fully-doubled boundary cancels to no material",
        );
    }

    /// Three traversals equal one: parity is multiplicity mod 2.
    #[test]
    fn triple_traversal_equals_single() {
        let plane = parity_plane();
        let tol = 0.01;
        let single = PolyBoundary::new(
            vec![PolyBoundaryPiece::untagged(square(1))],
            &plane,
            tol,
            &unevidenced_lattice(&plane),
        );
        let triple = PolyBoundary::new(
            vec![PolyBoundaryPiece::untagged(square(3))],
            &plane,
            tol,
            &unevidenced_lattice(&plane),
        );
        let single_mesh = trimming_tessellation(&plane, &single, tol, &unevidenced_lattice(&plane));
        let triple_mesh = trimming_tessellation(&plane, &triple, tol, &unevidenced_lattice(&plane));
        assert!(!single_mesh.faces().is_empty());
        assert!(!triple_mesh.faces().is_empty());
        assert_eq!(
            single_mesh.faces().tri_faces().len(),
            triple_mesh.faces().tri_faces().len(),
            "three traversals equal one: parity is multiplicity mod 2",
        );
    }

    /// INV-W3-1: a duplicate is never refused as `ConstraintOverlapUnsupported`.
    #[test]
    fn duplicate_edge_is_not_constraint_overlap_unsupported() {
        let plane = parity_plane();
        let tol = 0.01;
        let corner =
            |x: f64, y: f64| -> SurfacePoint { (Point2::new(x, y), Point3::new(x, y, 0.0)).into() };
        // The edge (0,0)-(10,0) is claimed three times; the loop otherwise
        // bounds the square (0,0)-(10,0)-(10,10)-(0,10).
        let loop0: Vec<SurfacePoint> = vec![
            corner(0.0, 0.0),
            corner(10.0, 0.0),
            corner(0.0, 0.0),
            corner(10.0, 0.0),
            corner(10.0, 10.0),
            corner(0.0, 10.0),
            corner(0.0, 0.0),
        ];
        let boundary = PolyBoundary::new(
            vec![PolyBoundaryPiece::untagged(loop0)],
            &plane,
            tol,
            &unevidenced_lattice(&plane),
        );
        let reason =
            trimming_tessellation_result(&plane, &boundary, tol, &unevidenced_lattice(&plane))
                .err()
                .map(|failure| failure.reason);
        assert_ne!(
            reason,
            Some(TessellationFailureReason::ConstraintOverlapUnsupported),
            "a duplicate traversal must not be refused as an unsupported overlap",
        );
    }

    /// INV-W3-2/3: a duplicated traversal creates no second CDT edge, and the
    /// traversal sum matches the inserted count.
    #[test]
    fn duplicate_edge_creates_no_second_cdt_edge() {
        let plane = parity_plane();
        let tol = 0.01;
        let corner =
            |x: f64, y: f64| -> SurfacePoint { (Point2::new(x, y), Point3::new(x, y, 0.0)).into() };
        let loop0: Vec<SurfacePoint> = vec![
            corner(0.0, 0.0),
            corner(10.0, 0.0),
            corner(0.0, 0.0),
            corner(10.0, 0.0),
            corner(5.0, 10.0),
        ];
        let boundary = PolyBoundary::new(
            vec![PolyBoundaryPiece::untagged(loop0)],
            &plane,
            tol,
            &unevidenced_lattice(&plane),
        );
        let mut triangulation = Cdt::new();
        let mut boundary_map = HashMap::<FixedVertexHandle, Point3>::default();
        let mut vertex_sources = HashMap::<FixedVertexHandle, Vec<SourceEdgeUse>>::default();
        let mut roles = ConstraintRoles::default();
        boundary
            .insert_to(
                &mut triangulation,
                &mut boundary_map,
                &mut roles,
                &mut vertex_sources,
            )
            .expect("duplicates are admitted, not rejected");
        assert_eq!(
            triangulation.num_constraints(),
            3,
            "a duplicated traversal creates no second CDT edge (INV-W3-2)",
        );
        let v0 = triangulation
            .vertices()
            .find(|v| v.as_ref() == &SPoint2::new(0.0, 0.0))
            .expect("vertex (0,0) exists")
            .fix();
        let v1 = triangulation
            .vertices()
            .find(|v| v.as_ref() == &SPoint2::new(10.0, 0.0))
            .expect("vertex (10,0) exists")
            .fix();
        let e_bottom = triangulation
            .get_edge_from_neighbors(v0, v1)
            .expect("edge (0,0)-(10,0) exists")
            .as_undirected()
            .fix();
        assert_eq!(
            roles.traversals.get(&e_bottom),
            Some(&3),
            "the duplicated edge is traversed three times",
        );
        assert_eq!(
            roles.traversals.values().sum::<usize>(),
            5,
            "sum of traversals equals the inserted semantic traversals (INV-W3-3)",
        );
    }

    /// The out-and-back slit cancels mod 2 but the disk survives: the 165-face
    /// recovery case, distinguished from the 675-face total cancellation.
    #[test]
    fn slit_cancels_but_disk_survives() {
        let plane = parity_plane();
        let tol = 0.01;
        let corner =
            |x: f64, y: f64| -> SurfacePoint { (Point2::new(x, y), Point3::new(x, y, 0.0)).into() };
        // A square with an interior spur across it: (0,5)->(10,5)->(0,5).
        let loop0: Vec<SurfacePoint> = vec![
            corner(0.0, 0.0),
            corner(10.0, 0.0),
            corner(10.0, 10.0),
            corner(0.0, 10.0),
            corner(0.0, 5.0),
            corner(10.0, 5.0),
            corner(0.0, 5.0),
            corner(0.0, 0.0),
        ];
        let boundary = PolyBoundary::new(
            vec![PolyBoundaryPiece::untagged(loop0)],
            &plane,
            tol,
            &unevidenced_lattice(&plane),
        );
        let mesh = trimming_tessellation(&plane, &boundary, tol, &unevidenced_lattice(&plane));
        assert!(
            !mesh.faces().is_empty(),
            "the out-and-back slit cancels but the disk must survive",
        );
    }

    /// INV-W3-4: a duplicate from a different role keeps the first claim, and
    /// the multiplicity stays counted.
    #[test]
    fn different_role_duplicate_keeps_first_claim() {
        let corner =
            |x: f64, y: f64| -> SurfacePoint { (Point2::new(x, y), Point3::new(x, y, 0.0)).into() };
        // Edge (0,0)-(10,0) claimed as Source (first), then SyntheticClosure,
        // then Source again.
        let loop_ = BoundaryLoop::new(
            vec![
                corner(0.0, 0.0),
                corner(10.0, 0.0),
                corner(0.0, 0.0),
                corner(10.0, 0.0),
                corner(5.0, 10.0),
            ],
            vec![
                SegmentOrigin::Source,
                SegmentOrigin::SyntheticClosure,
                SegmentOrigin::Source,
                SegmentOrigin::Source,
                SegmentOrigin::Source,
            ],
            vec![Vec::new(); 5],
        );
        let boundary = PolyBoundary(vec![loop_]);
        let mut triangulation = Cdt::new();
        let mut boundary_map = HashMap::<FixedVertexHandle, Point3>::default();
        let mut vertex_sources = HashMap::<FixedVertexHandle, Vec<SourceEdgeUse>>::default();
        let mut roles = ConstraintRoles::default();
        boundary
            .insert_to(
                &mut triangulation,
                &mut boundary_map,
                &mut roles,
                &mut vertex_sources,
            )
            .expect("the duplicate is admitted");
        let v0 = triangulation
            .vertices()
            .find(|v| v.as_ref() == &SPoint2::new(0.0, 0.0))
            .expect("vertex (0,0) exists")
            .fix();
        let v1 = triangulation
            .vertices()
            .find(|v| v.as_ref() == &SPoint2::new(10.0, 0.0))
            .expect("vertex (10,0) exists")
            .fix();
        let e = triangulation
            .get_edge_from_neighbors(v0, v1)
            .expect("edge (0,0)-(10,0) exists")
            .as_undirected()
            .fix();
        assert_eq!(
            ConstraintRoles::role_of(&triangulation, e),
            Some(ConstraintRole::PhysicalBoundary),
            "the first (Source) claim wins",
        );
        assert_eq!(
            roles.traversals.get(&e),
            Some(&3),
            "multiplicity is counted"
        );
    }
}

/// PHASE-ALIGNMENT tests — the two-loop seam-reference correspondence.
///
/// The two bounds of a full 360° band are each a single self-closing circle
/// edge whose parameterization origin is arbitrary: no source edge, vertex, or
/// seam connects a specific point of one circle to a specific point of the
/// other. The correct seam correspondence is the surface's own generator
/// structure — points with equal periodic coordinate (v mod period) lie on a
/// common ruling, and the synthetic seam bridges must be rulings. This module
/// exercises `align_two_loop_phase`, which cyclically re-indexes a single-source
/// full-period loop so both loops share the same seam reference.
#[cfg(test)]
mod phase_alignment_tests {
    use super::*;
    use std::f64::consts::PI;
    use truck_modeling::{Line, Point2, Point3, RevolutedCurve, Vector3};

    fn cylinder() -> RevolutedCurve<Line<Point3>> {
        RevolutedCurve::by_revolution(
            Line(Point3::new(10.0, 0.0, 0.0), Point3::new(10.0, 0.0, 10.0)),
            Point3::origin(),
            Vector3::unit_z(),
        )
    }

    fn lattice(cyl: &RevolutedCurve<Line<Point3>>) -> CertifiedLattice {
        unevidenced_lattice(cyl)
    }

    fn use_(bound: usize, index: usize) -> SourceEdgeUse {
        SourceEdgeUse {
            bound: BoundId(bound),
            index,
            orientation: true,
        }
    }

    /// A single-source full-period circle loop on the cylinder at height `u`,
    /// whose samples start at angular phase `phase0` (radians). `src` is the one
    /// source edge use every segment carries (single-source gate).
    fn circle_loop(
        cyl: &RevolutedCurve<Line<Point3>>,
        u: f64,
        phase0: f64,
        src: SourceEdgeUse,
    ) -> BoundaryLoop {
        let n = 32usize;
        let points: Vec<SurfacePoint> = (0..=n)
            .map(|i| {
                let v = phase0 + (i as f64 / n as f64) * 2.0 * PI;
                let uv = Point2::new(u, v);
                (uv, cyl.subs(uv.x, uv.y)).into()
            })
            .collect();
        let source_uses: Vec<SegmentSources> = (0..=n).map(|_| vec![src]).collect();
        BoundaryLoop::periodic_source_walk(points, source_uses)
    }

    /// A loop whose segments alternately reference two source edge uses, so the
    /// single-source gate must refuse to re-index it (the source seam is
    /// established; correspondence is not free).
    fn multi_source_loop(cyl: &RevolutedCurve<Line<Point3>>, u: f64, phase0: f64) -> BoundaryLoop {
        let n = 32usize;
        let points: Vec<SurfacePoint> = (0..=n)
            .map(|i| {
                let v = phase0 + (i as f64 / n as f64) * 2.0 * PI;
                let uv = Point2::new(u, v);
                (uv, cyl.subs(uv.x, uv.y)).into()
            })
            .collect();
        let source_uses: Vec<SegmentSources> = (0..=n)
            .map(|i| vec![if i % 2 == 0 { use_(0, 0) } else { use_(0, 1) }])
            .collect();
        BoundaryLoop::periodic_source_walk(points, source_uses)
    }

    fn v_phase(p: &SurfacePoint) -> f64 {
        p.uv.y.rem_euclid(2.0 * PI)
    }

    /// The aligned phase after the join: both loops' start samples carry the
    /// same phase mod 2π, so the seam bridges are generator lines.
    #[test]
    fn t2_half_period_loops_are_aligned() {
        let cyl = cylinder();
        let ltt = lattice(&cyl);
        let loop0 = circle_loop(&cyl, 0.2, 0.0, use_(0, 0));
        let mut loop1 = circle_loop(&cyl, 0.8, PI, use_(1, 0));
        assert!(
            align_two_loop_phase(&loop0, &mut loop1, [0, 1], &ltt),
            "a half-period offset must be re-indexed"
        );
        assert!(
            (v_phase(&loop1.points[0]) - v_phase(&loop0.points[0])).abs() < 1e-9,
            "both loops must share the seam reference mod period: l0={} l1={}",
            v_phase(&loop0.points[0]),
            v_phase(&loop1.points[0]),
        );
    }

    /// T1 — aligned full-period loops (phase offset 0) are not touched: the
    /// function reports no re-index and the loops keep their start phases.
    #[test]
    fn t1_aligned_loops_are_unchanged() {
        let cyl = cylinder();
        let ltt = lattice(&cyl);
        let loop0 = circle_loop(&cyl, 0.2, 0.0, use_(0, 0));
        let mut loop1 = circle_loop(&cyl, 0.8, 0.0, use_(1, 0));
        assert!(
            !align_two_loop_phase(&loop0, &mut loop1, [0, 1], &ltt),
            "an already-aligned loop must not be re-indexed"
        );
        assert_eq!(
            v_phase(&loop1.points[0]),
            0.0,
            "the loop start phase is unchanged"
        );
    }

    /// T3 — an arbitrary fractional start offset (period / 3) is aligned to the
    /// nearest sample, so the residual phase is bounded by half a sample step.
    #[test]
    fn t3_fractional_start_is_aligned() {
        let cyl = cylinder();
        let ltt = lattice(&cyl);
        let loop0 = circle_loop(&cyl, 0.2, 0.0, use_(0, 0));
        let mut loop1 = circle_loop(&cyl, 0.8, 2.0 * PI / 3.0, use_(1, 0));
        assert!(
            align_two_loop_phase(&loop0, &mut loop1, [0, 1], &ltt),
            "a fractional offset must be re-indexed"
        );
        let step = 2.0 * PI / 32.0;
        let raw = (v_phase(&loop1.points[0]) - v_phase(&loop0.points[0])).abs();
        let residual = raw.min(2.0 * PI - raw);
        assert!(
            residual < step / 2.0 + 1e-9,
            "residual phase bounded by half a sample step: {residual}"
        );
    }

    /// The re-index is a pure cyclic rotation plus a full-period re-lift, so the
    /// multiset of realized 3D boundary samples on loop1 is preserved exactly:
    /// no sample is added, dropped, or moved to a different 3D location. This is
    /// the preservation invariant — the alignment changes only the arbitrary
    /// cyclic starting index of the sampled full-period loop.
    #[test]
    fn reindexing_preserves_realized_samples() {
        let cyl = cylinder();
        let ltt = lattice(&cyl);
        let loop0 = circle_loop(&cyl, 0.2, 0.0, use_(0, 0));
        let mut loop1 = circle_loop(&cyl, 0.8, PI, use_(1, 0));
        let before: Vec<Point3> = loop1.points.iter().map(|p| p.point).collect();
        assert!(
            align_two_loop_phase(&loop0, &mut loop1, [0, 1], &ltt),
            "a half-period offset must be re-indexed"
        );
        let after: Vec<Point3> = loop1.points.iter().map(|p| p.point).collect();
        assert_eq!(
            after.len(),
            before.len(),
            "no realized sample is added or dropped"
        );
        for p in &after {
            assert!(
                before.iter().any(|q| q.distance(*p) < 1e-9),
                "every realized sample survives the re-index: {p:?}"
            );
        }
    }

    /// T5 — a multi-source loop carries a source-established seam and must not
    /// be re-indexed: the function reports refusal.
    #[test]
    fn t5_multi_source_loop_is_not_reindexed() {
        let cyl = cylinder();
        let ltt = lattice(&cyl);
        let loop0 = circle_loop(&cyl, 0.2, 0.0, use_(0, 0));
        let mut loop1 = multi_source_loop(&cyl, 0.8, PI);
        let before = loop1.points.clone();
        assert!(
            !align_two_loop_phase(&loop0, &mut loop1, [0, 1], &ltt),
            "a multi-source loop must refuse re-indexing"
        );
        assert!(
            loop1
                .points
                .iter()
                .zip(&before)
                .all(|(a, b)| a.uv == b.uv && a.point == b.point),
            "the loop is byte-identical"
        );
    }

    /// T4 — the alignment composes with the deck-consistent direction decision:
    /// re-indexing does not change the loop displacement, so the two-loop join
    /// still tessellates and the old bow-tie case stays fixed. The phase-aligned
    /// band must tessellate to a non-empty mesh.
    #[test]
    fn t4_alignment_composes_with_deck_consistent() {
        let cyl = cylinder();
        let ltt = lattice(&cyl);
        let pieces = vec![
            PolyBoundaryPiece::untagged(
                (0..=32)
                    .map(|i| {
                        let v = (i as f64 / 32.0) * 2.0 * PI;
                        let uv = Point2::new(0.2, v);
                        (uv, cyl.subs(uv.x, uv.y)).into()
                    })
                    .collect(),
            ),
            PolyBoundaryPiece::untagged(
                (0..=32)
                    .map(|i| {
                        let v = PI + (i as f64 / 32.0) * 2.0 * PI;
                        let uv = Point2::new(0.8, v);
                        (uv, cyl.subs(uv.x, uv.y)).into()
                    })
                    .collect(),
            ),
        ];
        let boundary = PolyBoundary::new(pieces, &cyl, 0.01, &ltt);
        assert_eq!(boundary.0.len(), 1, "the join yields one closed loop");
        let mesh = trimming_tessellation_result(&cyl, &boundary, 0.01, &ltt)
            .expect("the phase-aligned band tessellates");
        assert!(!mesh.tri_faces().is_empty(), "and produces triangles");
    }

    /// The mean-translate preserves the fractional phase residual (a π offset is
    /// irreducible by integer periods), so the alignment must act after it — the
    /// combined operation still aligns a half-period-offset loop.
    #[test]
    fn alignment_acts_after_integer_mean_translate() {
        let cyl = cylinder();
        let ltt = lattice(&cyl);
        let loop0 = circle_loop(&cyl, 0.2, 0.0, use_(0, 0));
        let mut loop1 = circle_loop(&cyl, 0.8, PI, use_(1, 0));
        // Simulate an integer period translate that cannot remove the π residual.
        for p in &mut loop1.points {
            p.uv.y += 2.0 * PI;
        }
        assert!(
            align_two_loop_phase(&loop0, &mut loop1, [0, 1], &ltt),
            "the fractional residual is still aligned"
        );
        assert!(
            (v_phase(&loop1.points[0]) - v_phase(&loop0.points[0])).abs() < 1e-9,
            "both loops share the seam reference mod period"
        );
    }
}

#[cfg(test)]
mod singular_transition_tests {
    use super::*;
    use truck_modeling::{Point2, Point3, Vector3};
    /// `v` collapses at `u = 1`: the whole `u = 1` row maps to the single
    /// point `(0, 1, 0)`, so every `(1, v)` is a legitimate UV representative
    /// of that one singular 3D point. This is the abstract shape of the
    /// representative face #54588 collapse.
    #[derive(Clone, Copy)]
    struct CollapsedPatch;

    impl ParametricSurface for CollapsedPatch {
        type Point = Point3;
        type Vector = Vector3;
        fn subs(&self, u: f64, v: f64) -> Point3 {
            Point3::new((1.0 - u) * v, u, 0.0)
        }
        fn uder(&self, _: f64, v: f64) -> Vector3 {
            Vector3::new(-v, 1.0, 0.0)
        }
        fn vder(&self, u: f64, _: f64) -> Vector3 {
            Vector3::new(1.0 - u, 0.0, 0.0)
        }
        fn uuder(&self, _: f64, _: f64) -> Vector3 {
            Vector3::zero()
        }
        fn uvder(&self, _: f64, _: f64) -> Vector3 {
            Vector3::new(-1.0, 0.0, 0.0)
        }
        fn vvder(&self, _: f64, _: f64) -> Vector3 {
            Vector3::zero()
        }
        fn der_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Vector3 {
            match (m, n) {
                (0, 0) => self.subs(u, v).to_vec(),
                (1, 0) => self.uder(u, v),
                (0, 1) => self.vder(u, v),
                (1, 1) => self.uvder(u, v),
                _ => Vector3::zero(),
            }
        }
    }

    impl ParametricSurface3D for CollapsedPatch {}

    impl ParameterDivision2D for CollapsedPatch {
        fn parameter_division(
            &self,
            ((u0, u1), (v0, v1)): ((f64, f64), (f64, f64)),
            _: f64,
        ) -> (Vec<f64>, Vec<f64>) {
            (vec![u0, u1], vec![v0, v1])
        }
    }

    /// Mirror of `CollapsedPatch` with the axes swapped, so `u` collapses at
    /// `v = 1`. Used to exercise the u-direction enter/leave branches.
    #[derive(Clone, Copy)]
    struct CollapsedPatchU;

    impl ParametricSurface for CollapsedPatchU {
        type Point = Point3;
        type Vector = Vector3;
        fn subs(&self, u: f64, v: f64) -> Point3 {
            Point3::new((1.0 - v) * u, v, 0.0)
        }
        fn uder(&self, _: f64, v: f64) -> Vector3 {
            Vector3::new(1.0 - v, 0.0, 0.0)
        }
        fn vder(&self, u: f64, _: f64) -> Vector3 {
            Vector3::new(-u, 1.0, 0.0)
        }
        fn uuder(&self, _: f64, _: f64) -> Vector3 {
            Vector3::zero()
        }
        fn uvder(&self, _: f64, _: f64) -> Vector3 {
            Vector3::new(-1.0, 0.0, 0.0)
        }
        fn vvder(&self, _: f64, _: f64) -> Vector3 {
            Vector3::zero()
        }
        fn der_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Vector3 {
            match (m, n) {
                (0, 0) => self.subs(u, v).to_vec(),
                (1, 0) => self.uder(u, v),
                (0, 1) => self.vder(u, v),
                (1, 1) => self.uvder(u, v),
                _ => Vector3::zero(),
            }
        }
    }

    impl ParametricSurface3D for CollapsedPatchU {}

    impl ParameterDivision2D for CollapsedPatchU {
        fn parameter_division(
            &self,
            ((u0, u1), (v0, v1)): ((f64, f64), (f64, f64)),
            _: f64,
        ) -> (Vec<f64>, Vec<f64>) {
            (vec![u0, u1], vec![v0, v1])
        }
    }

    /// Items 5-7: at least one triangle, no zero-area triangles, and a single
    /// consistent orientation sign. CollapsedPatch is planar (z = 0), so the
    /// orientation reduces to the sign of the z component of the cross product.
    fn assert_no_zero_area_or_orientation_flip(mesh: &PolygonMesh) {
        assert!(
            !mesh.faces().is_empty(),
            "tessellation produced no triangles"
        );
        let mut orientation = 0.0_f64;
        for tri in mesh.faces().tri_faces() {
            let [p0, p1, p2] = tri.map(|v| mesh.positions()[v.pos]);
            let cross = (p1 - p0).cross(p2 - p0);
            assert!(cross.magnitude2() > 1.0e-18, "zero-area triangle");
            if orientation == 0.0 {
                orientation = cross.z.signum();
            }
            assert_eq!(
                cross.z.signum(),
                orientation,
                "inconsistent triangle orientation"
            );
        }
    }

    /// Items 1-8: entering the collapsed row preserves the incoming UV branch,
    /// leaving it appends a bridge paired with the singular 3D point, every
    /// touched point re-evaluates within tolerance, the reconstructed boundary
    /// does not backtrack across the singular row, and the surface tessellates
    /// with consistent triangle orientation. The reverse-traversal shape is
    /// checked at the end.
    #[test]
    fn singular_transition_preserves_incidence_and_tessellates() {
        let surface = CollapsedPatch;
        let tolerance = 1.0e-9;
        let uv = |u, v| Point2::new(u, v);
        let sp = |p: Point2| (p, surface.subs(p.x, p.y)).into();
        // Approach the singular row along v = 0, leave it along v = 1.
        let mut points: Vec<SurfacePoint> =
            [uv(0.0, 1.0), uv(0.0, 0.5), uv(0.0, 0.0), uv(0.5, 0.0)]
                .into_iter()
                .map(sp)
                .collect();

        // Enter the collapse. The raw lift proposes (1, 1.1006...) but the
        // singular row u = 1 evaluates it to (0, 1, 0).
        let previous_uv = uv(0.5, 0.0);
        let previous_point = surface.subs(previous_uv.x, previous_uv.y);
        let mut entering_uv = uv(1.0, 1.100_620_084_301_750_6);
        let singular_point = surface.subs(entering_uv.x, entering_uv.y);
        assert!(singular_point.near(&Point3::new(0.0, 1.0, 0.0)));
        reconcile_singular_transition(
            &surface,
            previous_uv,
            previous_point,
            &mut entering_uv,
            singular_point,
            tolerance,
            &mut points,
        );
        // (1) The incoming v = 0 branch is preserved.
        assert!(entering_uv.near(&uv(1.0, 0.0)));
        points.push((entering_uv, singular_point).into());

        // Leave the collapse toward v = 1.
        let mut leaving_uv = uv(0.5, 1.0);
        let leaving_point = surface.subs(leaving_uv.x, leaving_uv.y);
        reconcile_singular_transition(
            &surface,
            entering_uv,
            singular_point,
            &mut leaving_uv,
            leaving_point,
            tolerance,
            &mut points,
        );
        // (2) A bridge was appended, paired with the singular 3D point.
        let bridge = *points.last().unwrap();
        assert!(bridge.uv.near(&uv(1.0, 1.0)));
        assert!(bridge.point.near(&singular_point));
        points.push((leaving_uv, leaving_point).into());

        // (3) Every modified or inserted point re-evaluates within tolerance.
        assert!(points
            .iter()
            .all(|p| surface.subs(p.uv.x, p.uv.y).distance(p.point) <= tolerance));

        // (4) No improper backtracking crossing. The original defect produced
        // the spike (0.5,0) -> (1,1.1006) -> (1,1); the repaired boundary
        // enters and leaves the singular row u = 1 exactly once each.
        let u_visits: Vec<f64> = points.iter().map(|p| p.uv.x).collect();
        let near_row = |x: f64| (x - 1.0).abs() < 1.0e-7;
        let crossings = u_visits
            .windows(2)
            .filter(|w| near_row(w[0]) ^ near_row(w[1]))
            .count();
        assert_eq!(
            crossings, 2,
            "boundary must enter and leave u = 1 exactly once each"
        );

        // Close the loop and tessellate.
        points.push(sp(uv(0.0, 1.0)));
        let boundary = PolyBoundary::new(
            vec![PolyBoundaryPiece::untagged(points)],
            &surface,
            tolerance,
            &unevidenced_lattice(&surface),
        );
        let mesh = trimming_tessellation(
            &surface,
            &boundary,
            tolerance,
            &unevidenced_lattice(&surface),
        );
        // (5),(6),(7) at least one triangle, no zero-area, consistent orientation.
        assert_no_zero_area_or_orientation_flip(&mesh);

        // (8) Reverse traversal: walk the same collapse the other way. The
        // singular 3D point must still be preserved on both enter and leave,
        // with a bridge inserted on the leave.
        let mut rev: Vec<SurfacePoint> = [uv(0.5, 1.0)].into_iter().map(sp).collect();
        let prev_uv = uv(0.5, 1.0);
        let prev_pt = surface.subs(prev_uv.x, prev_uv.y);
        let mut enter_uv = uv(1.0, 0.9);
        let singular_pt = surface.subs(enter_uv.x, enter_uv.y);
        assert!(singular_pt.near(&Point3::new(0.0, 1.0, 0.0)));
        reconcile_singular_transition(
            &surface,
            prev_uv,
            prev_pt,
            &mut enter_uv,
            singular_pt,
            tolerance,
            &mut rev,
        );
        assert!(
            enter_uv.near(&uv(1.0, 1.0)),
            "reverse enter preserves the singular row"
        );
        rev.push((enter_uv, singular_pt).into());
        let mut leave_uv = uv(0.5, 0.0);
        let leave_pt = surface.subs(leave_uv.x, leave_uv.y);
        reconcile_singular_transition(
            &surface,
            enter_uv,
            singular_pt,
            &mut leave_uv,
            leave_pt,
            tolerance,
            &mut rev,
        );
        let rev_bridge = *rev.last().unwrap();
        assert!(
            rev_bridge.point.near(&singular_pt),
            "reverse leave bridges with the singular 3D point"
        );
        assert!(
            rev.iter()
                .all(|p| surface.subs(p.uv.x, p.uv.y).distance(p.point) <= tolerance),
            "reverse traversal: every point re-evaluates within tolerance"
        );
    }

    /// Item 9: the u-direction enter/leave branches on a surface whose `u`
    /// collapses at `v = 1`.
    #[test]
    fn singular_transition_both_parameter_directions() {
        let surface = CollapsedPatchU;
        let tolerance = 1.0e-9;
        let uv = |u, v| Point2::new(u, v);
        let sp = |p: Point2| (p, surface.subs(p.x, p.y)).into();
        let mut points: Vec<SurfacePoint> =
            [uv(1.0, 0.0), uv(0.5, 0.0), uv(0.0, 0.0), uv(0.0, 0.5)]
                .into_iter()
                .map(sp)
                .collect();
        let previous_uv = uv(0.0, 0.5);
        let previous_point = surface.subs(previous_uv.x, previous_uv.y);
        let mut entering_uv = uv(1.100_620_084_301_750_6, 1.0);
        let singular_point = surface.subs(entering_uv.x, entering_uv.y);
        assert!(singular_point.near(&Point3::new(0.0, 1.0, 0.0)));
        reconcile_singular_transition(
            &surface,
            previous_uv,
            previous_point,
            &mut entering_uv,
            singular_point,
            tolerance,
            &mut points,
        );
        assert!(
            entering_uv.near(&uv(0.0, 1.0)),
            "u-collapse enter preserves the incoming u branch"
        );
        points.push((entering_uv, singular_point).into());

        let mut leaving_uv = uv(1.0, 0.5);
        let leaving_point = surface.subs(leaving_uv.x, leaving_uv.y);
        reconcile_singular_transition(
            &surface,
            entering_uv,
            singular_point,
            &mut leaving_uv,
            leaving_point,
            tolerance,
            &mut points,
        );
        let bridge = *points.last().unwrap();
        assert!(bridge.uv.near(&uv(1.0, 1.0)));
        assert!(bridge.point.near(&singular_point));
        points.push((leaving_uv, leaving_point).into());
        assert!(
            points
                .iter()
                .all(|p| surface.subs(p.uv.x, p.uv.y).distance(p.point) <= tolerance),
            "u-direction: every point re-evaluates within tolerance"
        );
    }

    /// Item 10: a derivative that is small but whose candidate UV does NOT
    /// reconstruct the associated 3D point must not be substituted.
    #[test]
    fn singular_transition_negative_case_rejects_non_reconstructing_candidate() {
        let surface = CollapsedPatch;
        let tolerance = 1.0e-9;
        // At u = 1 - delta the v-derivative has magnitude delta < TOLERANCE, so
        // `so_small()` proposes a substitution, but the point still genuinely
        // depends on v, so the residual check must reject it.
        let delta = 5.0e-7;
        let previous_uv = Point2::new(0.5, 0.0);
        let previous_point = surface.subs(previous_uv.x, previous_uv.y);
        let raw_uv = Point2::new(1.0 - delta, 0.6);
        let raw_point = surface.subs(raw_uv.x, raw_uv.y);
        // Sanity: the v-derivative really is small here, and the candidate
        // really would move the point beyond tolerance.
        assert!(surface.vder(raw_uv.x, raw_uv.y).so_small());
        let candidate = Point2::new(raw_uv.x, previous_uv.y);
        assert!(
            surface.subs(candidate.x, candidate.y).distance(raw_point) > tolerance,
            "test setup: candidate must fail the residual check"
        );

        let mut current_uv = raw_uv;
        let mut out: Vec<SurfacePoint> = Vec::new();
        reconcile_singular_transition(
            &surface,
            previous_uv,
            previous_point,
            &mut current_uv,
            raw_point,
            tolerance,
            &mut out,
        );
        // The candidate did not reconstruct the point: nothing changed.
        assert!(
            current_uv.near(&raw_uv),
            "helper must not substitute a non-reconstructing candidate"
        );
        assert!(
            out.is_empty(),
            "no bridge may be appended when the residual check fails"
        );
    }
}

#[cfg(test)]
mod segment_origin_tests {
    use super::*;

    fn pt(x: f64, y: f64) -> SurfacePoint {
        (Point2::new(x, y), Point3::new(x, y, 0.0)).into()
    }

    /// A distinct synthetic source edge use, so a test can assert *which* use
    /// labels a segment rather than only that a slot exists.
    fn use_(index: usize) -> SourceEdgeUse {
        SourceEdgeUse {
            bound: BoundId(0),
            index,
            orientation: true,
        }
    }

    /// One tagged provenance slot per segment of an `n`-point open chain.
    fn tagged_sources(point_count: usize) -> Vec<SegmentSources> {
        (0..point_count.saturating_sub(1))
            .map(|k| vec![use_(k)])
            .collect()
    }

    /// An interior crossing preserves the point count and rotates the
    /// provenance so each entry still labels the segment it described.
    ///
    /// The chain crosses the working range `u1` at index 3 of five points. The
    /// rotation starts the chain at the crossing; `sources` follow their
    /// segments under the start-point rule (`sources[k]` labels
    /// `points[k] -> points[k + 1]`).
    #[test]
    fn normalize_range_rotates_provenance_with_an_interior_crossing() {
        let mut curve = vec![
            pt(0.1, 0.0),
            pt(0.4, 0.0),
            pt(0.6, 0.0),
            pt(1.2, 0.0),
            pt(0.3, 0.0),
        ];
        let mut sources = tagged_sources(5);
        normalize_range(&mut curve, &mut sources, 0, (0.0, 1.0));
        assert_eq!(
            curve.len(),
            5,
            "an interior crossing preserves the point count"
        );
        assert_eq!(
            sources.len(),
            curve.len() - 1,
            "one provenance entry per resulting segment",
        );
        assert_eq!(
            sources,
            vec![vec![use_(3)], vec![use_(0)], vec![use_(1)], vec![use_(2)]],
            "the provenance rotates with its segments",
        );
    }

    /// A crossing at the chain's own last point re-introduces the wrap segment
    /// back onto the head. That segment is synthetic — its provenance was
    /// dropped when the piece was classified open — so the chain gains an
    /// explicit empty entry rather than short-circuiting the vector.
    #[test]
    fn normalize_range_reintroduces_an_empty_wrap_when_the_crossing_is_the_last_point() {
        let mut curve = vec![
            pt(0.1, 0.0),
            pt(0.4, 0.0),
            pt(0.6, 0.0),
            pt(0.9, 0.0),
            pt(1.2, 0.0),
        ];
        let mut sources = tagged_sources(5);
        normalize_range(&mut curve, &mut sources, 0, (0.0, 1.0));
        assert_eq!(
            curve.len(),
            6,
            "the last-point crossing duplicates the terminal"
        );
        assert_eq!(
            sources.len(),
            curve.len() - 1,
            "one provenance entry per resulting segment",
        );
        assert_eq!(
            sources,
            vec![
                Vec::new(),
                vec![use_(0)],
                vec![use_(1)],
                vec![use_(2)],
                vec![use_(3)],
            ],
            "the synthetic wrap leads, the original segments follow in order",
        );
    }

    /// The whole construction keeps the three vectors equal-length: every
    /// shared-endpoint join, bridge, and closure adds its provenance slots in
    /// lockstep with the segments it creates. The bridge is synthetic and
    /// carries an explicit empty entry; the closing wrap is the last source
    /// segment, which keeps its contributor.
    #[test]
    fn every_constructed_loop_keeps_one_provenance_entry_per_segment() {
        let mut path = BoundaryPath::start(
            vec![pt(0.0, 0.0), pt(1.0, 0.0), pt(2.0, 0.0)],
            vec![vec![use_(0)], vec![use_(1)]],
            SegmentOrigin::Source,
        );
        path.append(
            vec![pt(2.0, 0.0), pt(3.0, 0.0)],
            vec![vec![use_(2)]],
            SegmentOrigin::Source,
            PartJoin::SharedEndpoint,
        );
        path.append(
            vec![pt(3.0, 1.0), pt(0.0, 0.0)],
            vec![vec![use_(3)]],
            SegmentOrigin::SyntheticClosure,
            PartJoin::Bridge(SegmentOrigin::Seam),
        );
        let loop_ = path.close(PartJoin::SharedEndpoint);
        assert_eq!(
            loop_.points.len(),
            loop_.origins.len(),
            "one origin per segment",
        );
        assert_eq!(
            loop_.points.len(),
            loop_.source_uses.len(),
            "one provenance entry per segment",
        );
        assert_eq!(
            loop_.source_uses,
            vec![
                vec![use_(0)],
                vec![use_(1)],
                vec![use_(2)],
                Vec::new(),    // the bridge carries no source
                vec![use_(3)], // the closing wrap is the last source segment
            ],
        );
    }

    /// A shared endpoint drops the duplicate and creates no segment.
    #[test]
    fn shared_endpoint_creates_no_segment() {
        let mut path = BoundaryPath::start(
            vec![pt(0.0, 0.0), pt(1.0, 0.0)],
            untagged_sources(2),
            SegmentOrigin::Source,
        );
        path.append(
            vec![pt(1.0, 0.0), pt(1.0, 1.0)],
            untagged_sources(2),
            SegmentOrigin::SyntheticClosure,
            PartJoin::SharedEndpoint,
        );
        assert_eq!(path.points.len(), 3, "the duplicate join point is dropped");
        assert_eq!(
            path.origins,
            vec![SegmentOrigin::Source, SegmentOrigin::SyntheticClosure],
        );
    }

    /// A bridge keeps **both** endpoints and inserts exactly one labelled
    /// segment between them.
    ///
    /// This is the case an earlier implementation got wrong: it dropped every
    /// part's last point unconditionally, so `a1 -> a2 -> b0` silently became
    /// the shortcut `a1 -> b0`, deleting a source segment. The point count is
    /// the assertion that matters â€” metadata retention must not change the
    /// polygon.
    #[test]
    fn a_bridge_preserves_both_endpoints() {
        let mut path = BoundaryPath::start(
            vec![pt(0.0, 0.0), pt(1.0, 0.0), pt(2.0, 0.0)],
            untagged_sources(3),
            SegmentOrigin::Source,
        );
        path.append(
            vec![pt(5.0, 5.0), pt(6.0, 5.0), pt(7.0, 5.0)],
            untagged_sources(3),
            SegmentOrigin::Source,
            PartJoin::Bridge(SegmentOrigin::Seam),
        );
        assert_eq!(path.points.len(), 6, "no source point may be dropped");
        assert_eq!(
            path.points[2].uv,
            pt(2.0, 0.0).uv,
            "the first part keeps its tail"
        );
        assert_eq!(
            path.points[3].uv,
            pt(5.0, 5.0).uv,
            "the second keeps its head"
        );
        assert_eq!(
            path.origins,
            vec![
                SegmentOrigin::Source, // 0 -> 1
                SegmentOrigin::Source, // 1 -> 2
                SegmentOrigin::Seam,   // 2 -> 3, the bridge
                SegmentOrigin::Source, // 3 -> 4
                SegmentOrigin::Source, // 4 -> 5
            ],
        );
    }

    /// Closing on a shared endpoint drops the repeated point; the existing last
    /// segment becomes the cyclic wrap.
    #[test]
    fn closing_on_a_shared_endpoint_reuses_the_last_segment() {
        let mut path = BoundaryPath::start(
            vec![pt(0.0, 0.0), pt(1.0, 0.0)],
            untagged_sources(2),
            SegmentOrigin::Source,
        );
        path.append(
            vec![pt(1.0, 0.0), pt(0.0, 0.0)],
            untagged_sources(2),
            SegmentOrigin::SyntheticClosure,
            PartJoin::SharedEndpoint,
        );
        let loop_ = path.close(PartJoin::SharedEndpoint);
        assert_eq!(loop_.points.len(), 2);
        assert_eq!(
            loop_.points.len(),
            loop_.origins.len(),
            "one origin per segment"
        );
        assert_eq!(
            loop_.origins,
            vec![SegmentOrigin::Source, SegmentOrigin::SyntheticClosure],
        );
    }

    /// Closing across a gap adds one labelled wrap segment and keeps every point.
    #[test]
    fn closing_across_a_gap_adds_a_labelled_wrap() {
        let path = BoundaryPath::start(
            vec![pt(0.0, 0.0), pt(1.0, 0.0), pt(1.0, 1.0)],
            untagged_sources(3),
            SegmentOrigin::Source,
        );
        let loop_ = path.close(PartJoin::Bridge(SegmentOrigin::SyntheticClosure));
        assert_eq!(loop_.points.len(), 3);
        assert_eq!(loop_.points.len(), loop_.origins.len());
        assert_eq!(
            *loop_.origins.last().unwrap(),
            SegmentOrigin::SyntheticClosure,
            "the wrap is not another source segment",
        );
    }

    /// A periodically closed walk retains its endpoint at `first + LÂ·Î´`, so the
    /// cyclic wrap is the deck closure and must not be labelled `Source`.
    #[test]
    fn a_periodic_walk_does_not_call_its_wrap_a_source_segment() {
        let walk = BoundaryLoop::periodic_source_walk(
            vec![pt(0.0, 0.0), pt(0.0, 1.0), pt(0.0, 2.0)],
            vec![Vec::new(); 3],
        );
        assert_eq!(walk.points.len(), walk.origins.len());
        assert_eq!(
            walk.origins,
            vec![
                SegmentOrigin::Source,
                SegmentOrigin::Source,
                SegmentOrigin::Seam,
            ],
        );
    }

    /// A Euclidean loop has already had its duplicate endpoint removed, so
    /// every cyclic segment including the wrap is genuine source trim.
    #[test]
    fn a_euclidean_loop_wraps_on_a_source_segment() {
        let loop_ = BoundaryLoop::euclidean_source_loop(
            vec![pt(0.0, 0.0), pt(1.0, 0.0), pt(1.0, 1.0)],
            vec![Vec::new(); 3],
        );
        assert!(loop_.origins.iter().all(|o| *o == SegmentOrigin::Source));
        assert_eq!(loop_.points.len(), loop_.origins.len());
    }

    /// Every constructed loop must carry exactly one origin per segment.
    #[test]
    fn chained_parts_yield_one_origin_per_segment() {
        let loop_ = BoundaryLoop::chained([
            (
                vec![pt(0.0, 0.0), pt(1.0, 0.0)],
                untagged_sources(2),
                SegmentOrigin::Source,
            ),
            (
                vec![pt(1.0, 0.0), pt(1.0, 1.0)],
                untagged_sources(2),
                SegmentOrigin::SyntheticClosure,
            ),
            (
                vec![pt(1.0, 1.0), pt(0.0, 0.0)],
                untagged_sources(2),
                SegmentOrigin::Seam,
            ),
        ]);
        assert_eq!(loop_.points.len(), loop_.origins.len());
        assert_eq!(loop_.points.len(), 3, "join duplicates are dropped");
    }
}

/// Pure tests for [`classify_presented_relation`]: exact predicates only, no
/// CDT required.
///
/// The handle pairs are fabrications â€” `from_index` â€” because the classifier
/// reads nothing from the triangulation; the positions drive every predicate.
#[cfg(test)]
mod presented_relation_tests {
    use super::*;

    fn p(x: f64, y: f64) -> SPoint2 {
        SPoint2::new(x, y)
    }

    fn handles(a: usize, b: usize) -> [FixedVertexHandle; 2] {
        [
            FixedVertexHandle::from_index(a),
            FixedVertexHandle::from_index(b),
        ]
    }

    /// Same undirected vertex pair is a duplicate traversal, in either order.
    #[test]
    fn identical_vertex_pair_is_duplicate() {
        assert_eq!(
            classify_presented_relation(
                handles(0, 1),
                handles(1, 0),
                p(0.0, 0.0),
                p(1.0, 0.0),
                p(1.0, 0.0),
                p(0.0, 0.0),
            ),
            diagnosis::PresentedSegmentRelation::DuplicateTraversal,
        );
        assert_eq!(
            classify_presented_relation(
                handles(0, 1),
                handles(0, 1),
                p(0.0, 0.0),
                p(1.0, 0.0),
                p(0.0, 0.0),
                p(1.0, 0.0),
            ),
            diagnosis::PresentedSegmentRelation::DuplicateTraversal,
        );
    }

    /// `(0,0)-(2,0)` vs `(1,0)-(3,0)`: all four collinear, no shared endpoint.
    #[test]
    fn collinear_overlapping_is_collinear_overlap() {
        assert_eq!(
            classify_presented_relation(
                handles(0, 1),
                handles(2, 3),
                p(0.0, 0.0),
                p(2.0, 0.0),
                p(1.0, 0.0),
                p(3.0, 0.0),
            ),
            diagnosis::PresentedSegmentRelation::CollinearOverlap,
        );
    }

    /// `(0,0)-(2,0)` vs `(1,0)-(1,1)`: endpoint `(1,0)` lies exactly on the
    /// first segment's interior.
    #[test]
    fn endpoint_exactly_on_interior() {
        assert_eq!(
            classify_presented_relation(
                handles(0, 1),
                handles(2, 3),
                p(0.0, 0.0),
                p(2.0, 0.0),
                p(1.0, 0.0),
                p(1.0, 1.0),
            ),
            diagnosis::PresentedSegmentRelation::EndpointOnInterior,
        );
    }

    /// `(0,0)-(2,2)` vs `(0,2)-(2,0)`: a deep transversal crossing at `(1,1)`.
    #[test]
    fn transversal_is_proper_interior_crossing() {
        assert_eq!(
            classify_presented_relation(
                handles(0, 1),
                handles(2, 3),
                p(0.0, 0.0),
                p(2.0, 2.0),
                p(0.0, 2.0),
                p(2.0, 0.0),
            ),
            diagnosis::PresentedSegmentRelation::ProperInteriorCrossing,
        );
    }

    /// One ULP off the exact incidence is still a proper crossing. This pins
    /// the "exact predicates only" contract: no tolerance may fold a near-miss
    /// into `EndpointOnInterior`.
    #[test]
    fn near_miss_is_still_a_crossing() {
        let eps = f64::EPSILON;
        assert_eq!(
            classify_presented_relation(
                handles(0, 1),
                handles(2, 3),
                p(0.0, 0.0),
                p(2.0, 0.0),
                p(1.0, 0.0),
                p(1.0, 1.0),
            ),
            diagnosis::PresentedSegmentRelation::EndpointOnInterior,
            "sanity: the exact incidence is recognized",
        );
        // Shift the incident endpoint one ULP below the supporting line: the
        // vertical segment now genuinely straddles the horizontal one, and the
        // exact predicates see a proper crossing â€” never an incidence.
        assert_eq!(
            classify_presented_relation(
                handles(0, 1),
                handles(2, 3),
                p(0.0, 0.0),
                p(2.0, 0.0),
                p(1.0, -eps),
                p(1.0, 1.0),
            ),
            diagnosis::PresentedSegmentRelation::ProperInteriorCrossing,
            "1 ULP below the line must still read as a proper crossing",
        );
    }

    /// A self-crossing bowtie face now renders because PLANAR-C planarizes the
    /// proper crossing; the diagnostic classifier must not change that outcome.
    /// W1 must move no face.
    #[test]
    fn classifier_does_not_change_outcome() {
        use truck_geometry::prelude::*;
        let plane = Plane::new(
            Point3::origin(),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        );
        let tol = 0.01;
        let lattice = unevidenced_lattice(&plane);
        // Bowtie: the segments `(0,0)->(10,10)` and `(10,0)->(0,10)` cross at
        // `(5,5)`. Previously `try_add_constraint` refused the crossing and the
        // face failed with `ConstraintInsertionIncomplete`; PLANAR-C subdivides
        // it instead, and the face renders.
        let loop0: Vec<SurfacePoint> = vec![
            (Point2::new(0.0, 0.0), Point3::new(0.0, 0.0, 0.0)).into(),
            (Point2::new(10.0, 10.0), Point3::new(10.0, 10.0, 0.0)).into(),
            (Point2::new(10.0, 0.0), Point3::new(10.0, 0.0, 0.0)).into(),
            (Point2::new(0.0, 10.0), Point3::new(0.0, 10.0, 0.0)).into(),
            (Point2::new(0.0, 0.0), Point3::new(0.0, 0.0, 0.0)).into(),
        ];
        let boundary = PolyBoundary::new(
            vec![PolyBoundaryPiece::untagged(loop0)],
            &plane,
            tol,
            &lattice,
        );
        let triangles = |diag: bool| {
            if diag {
                std::env::set_var("TRUCK_FACE_DIAG_JSONL", "presented_relation.jsonl");
            } else {
                std::env::remove_var("TRUCK_FACE_DIAG_JSONL");
            }
            let count = trimming_tessellation_result(&plane, &boundary, tol, &lattice)
                .map(|mesh| mesh.tri_faces().len())
                .unwrap_or(0);
            std::env::remove_var("TRUCK_FACE_DIAG_JSONL");
            count
        };
        let without = triangles(false);
        let with = triangles(true);
        assert!(without > 0, "the bowtie crossing is planarized and renders");
        assert_eq!(
            without, with,
            "diagnostics must not change the tessellation outcome",
        );
    }
}

/// PLANAR-A provenance tests: the source `(bound, edge-use)` identity survives
/// the legacy boundary path without being fabricated for synthetic geometry.
#[cfg(test)]
mod provenance_tests {
    use super::*;
    use truck_geometry::prelude::*;

    fn pt(x: f64, y: f64) -> SurfacePoint {
        (Point2::new(x, y), Point3::new(x, y, 0.0)).into()
    }

    fn use_(bound: usize, index: usize, orientation: bool) -> SourceEdgeUse {
        SourceEdgeUse {
            bound: BoundId(bound),
            index,
            orientation,
        }
    }

    /// A-test 1: the same geometric curve used twice as distinct source edge
    /// uses stays two distinct identities through flattening.
    #[test]
    fn distinct_edge_uses_survive_flattening() {
        let plane = Plane::new(
            Point3::origin(),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        );
        let tol = 0.01;
        let lattice = unevidenced_lattice(&plane);
        let curve = PolylineCurve(vec![Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)]);
        let use0 = use_(0, 0, true);
        let use1 = use_(0, 1, false);
        let wire = [
            SourcePolyline {
                curve: curve.clone(),
                source: use0,
            },
            SourcePolyline {
                curve: curve.clone(),
                source: use1,
            },
        ];
        let piece = PolyBoundaryPiece::try_new(
            &plane,
            wire.into_iter(),
            by_search_parameter,
            tol,
            &lattice,
        )
        .expect("the wire flattens");
        let mut seen: Vec<SourceEdgeUse> = Vec::new();
        for sources in &piece.1 {
            for &u in sources {
                if !seen.contains(&u) {
                    seen.push(u);
                }
            }
        }
        assert!(
            seen.contains(&use0),
            "the first use survives with its identity",
        );
        assert!(
            seen.contains(&use1),
            "the second use survives with its identity",
        );
        assert_eq!(
            seen.len(),
            2,
            "identical geometry does not collapse distinct source uses",
        );
        assert_ne!(use0, use1, "the identities differ by index and orientation");
    }

    /// A-test 2: reversal keeps each provenance entry with the reversed segment
    /// it belongs to.
    #[test]
    fn reversal_preserves_provenance() {
        let mut path = BoundaryPath::start(
            vec![pt(0.0, 0.0), pt(1.0, 0.0), pt(1.0, 1.0), pt(0.0, 1.0)],
            vec![
                vec![use_(0, 0, true)],
                vec![use_(0, 1, false)],
                Vec::new(), // a synthetic segment in the middle
            ],
            SegmentOrigin::Source,
        );
        path.reverse();
        // Segment `i` becomes old segment `n - 2 - i`, so the provenance must
        // reverse with the segments, not with the points.
        assert_eq!(
            path.source_uses,
            vec![Vec::new(), vec![use_(0, 1, false)], vec![use_(0, 0, true)],],
            "provenance reverses with the segments",
        );
        assert_eq!(path.source_uses.len(), path.origins.len());
    }

    /// A-test 3: synthetic segments never receive an invented source identity.
    #[test]
    fn synthetic_segments_carry_no_invented_source() {
        // A periodic walk: two source segments, then the deck-closure wrap.
        let walk = BoundaryLoop::periodic_source_walk(
            vec![pt(0.0, 0.0), pt(0.0, 1.0), pt(0.0, 2.0)],
            vec![
                vec![use_(0, 0, true)],
                vec![use_(0, 1, true)],
                vec![use_(0, 2, true)],
            ],
        );
        assert_eq!(walk.origins[2], SegmentOrigin::Seam);
        assert_eq!(
            walk.source_uses[2],
            Vec::<SourceEdgeUse>::new(),
            "the deck wrap invents no source use",
        );
        assert_eq!(walk.source_uses[0], vec![use_(0, 0, true)]);

        // A bridge close adds a synthetic wrap with no source either.
        let path = BoundaryPath::start(
            vec![pt(0.0, 0.0), pt(1.0, 0.0), pt(1.0, 1.0)],
            vec![vec![use_(0, 0, true)], vec![use_(0, 1, true)]],
            SegmentOrigin::Source,
        );
        let loop_ = path.close(PartJoin::Bridge(SegmentOrigin::SyntheticClosure));
        assert_eq!(
            *loop_.source_uses.last().unwrap(),
            Vec::<SourceEdgeUse>::new(),
            "the closure wrap invents no source use",
        );
        assert_eq!(loop_.source_uses.len(), loop_.origins.len());
    }
}

/// The first tests in the repo that exercise Spade's *splitting* constraint API
/// (`add_constraint_and_split`) with semantic `UE` data. Production code still
/// does not call the splitting API; these prove the PLANAR-B repair primitives
/// against the verified Spade split contract (parent handle keeps its `UE`,
/// new child is `UE::default()`, constraint edges are never flipped).
#[cfg(test)]
mod planar_b_split_repair_tests {
    use super::*;

    fn point(x: f64, y: f64) -> SPoint2 {
        SPoint2::new(x, y)
    }

    fn insert_vertex(cdt: &mut Cdt, p: SPoint2) -> FixedVertexHandle {
        cdt.insert(p).expect("vertex insertion succeeds")
    }

    fn find_vertex(cdt: &Cdt, p: SPoint2) -> FixedVertexHandle {
        cdt.vertices()
            .find(|v| v.as_ref() == &p)
            .expect("vertex exists")
            .fix()
    }

    fn find_edge(cdt: &Cdt, a: SPoint2, b: SPoint2) -> FixedUndirectedEdgeHandle {
        let va = find_vertex(cdt, a);
        let vb = find_vertex(cdt, b);
        cdt.get_edge_from_neighbors(va, vb)
            .expect("edge exists")
            .as_undirected()
            .fix()
    }

    fn directed_of(cdt: &Cdt, e: FixedUndirectedEdgeHandle) -> FixedDirectedEdgeHandle {
        cdt.undirected_edge(e).as_directed().fix()
    }

    fn source_use(index: usize) -> SourceEdgeUse {
        SourceEdgeUse {
            bound: BoundId(0),
            index,
            orientation: true,
        }
    }

    /// The minimal crossing scenario: blocking constraint `(0,0)-(10,0)` on the
    /// x-axis, incoming chord `(2,-5)-(2,5)` crossing it at `(2,0)`. Returns the
    /// CDT, the blocking parent handle, and the realized incoming chain.
    fn crossing_scenario() -> (Cdt, FixedUndirectedEdgeHandle, Vec<FixedDirectedEdgeHandle>) {
        let mut cdt = Cdt::new();
        let a = insert_vertex(&mut cdt, point(2.0, -5.0));
        let b = insert_vertex(&mut cdt, point(2.0, 5.0));
        let p = insert_vertex(&mut cdt, point(0.0, 0.0));
        let q = insert_vertex(&mut cdt, point(10.0, 0.0));
        let blocking_chain = cdt.try_add_constraint(p, q);
        assert!(
            !blocking_chain.is_empty(),
            "the blocking constraint is added"
        );
        let parent = cdt
            .get_edge_from_neighbors(p, q)
            .expect("blocking edge exists")
            .as_undirected()
            .fix();
        let chain = cdt.add_constraint_and_split(a, b, |pt: spade::Point2<f64>| pt);
        assert!(
            chain.len() >= 2,
            "the incoming chord is split by the blocking edge"
        );
        (cdt, parent, chain)
    }

    /// B-test 1: the returned incoming chain is contiguous and receives exactly
    /// one semantic identity on every returned piece.
    #[test]
    fn incoming_chain_receives_one_semantic_identity() {
        let (mut cdt, _parent, chain) = crossing_scenario();
        let mut roles = ConstraintRoles::default();
        let id = roles.mint_semantic_constraint_id();
        let sources = vec![source_use(0)];
        roles.label_realized_chain(
            &mut cdt,
            &chain,
            id,
            ConstraintRole::PhysicalBoundary,
            &sources,
            Some(SegmentOrigin::Source),
        );

        // Chain ordering is contiguous: first.from == requested A, last.to == B,
        // piece[i].to == piece[i+1].from.
        let a = find_vertex(&cdt, point(2.0, -5.0));
        let b = find_vertex(&cdt, point(2.0, 5.0));
        let directed: Vec<_> = chain.iter().map(|h| cdt.directed_edge(*h)).collect();
        assert_eq!(
            directed[0].from().fix(),
            a,
            "the chain starts at the request"
        );
        assert_eq!(
            directed.last().unwrap().to().fix(),
            b,
            "the chain ends at the request"
        );
        for window in directed.windows(2) {
            assert_eq!(
                window[0].to().fix(),
                window[1].from().fix(),
                "the realized pieces chain head-to-tail",
            );
        }

        // Every returned piece carries the one semantic claim C, with the
        // PLANAR-A contributor attached.
        for h in &chain {
            let e = cdt.undirected_edge(cdt.directed_edge(*h).as_undirected().fix());
            let claims = e.data().data().claims.as_slice();
            assert_eq!(claims.len(), 1, "exactly the incoming claim");
            assert_eq!(claims[0].semantic_id, id, "one semantic identity");
            assert_eq!(claims[0].source_uses, sources, "the contributor is carried");
        }
    }

    /// B-test 2: after the split, the original blocking handle keeps the claim
    /// and the new child inherits it via `repair_split`.
    #[test]
    fn blocking_child_inherits_claim() {
        let (mut cdt, parent, _chain) = crossing_scenario();
        let mut roles = ConstraintRoles::default();
        let parent_id = roles.mint_semantic_constraint_id();
        let sources = vec![source_use(0)];
        let parent_directed = directed_of(&cdt, parent);
        roles.label_realized_chain(
            &mut cdt,
            &[parent_directed],
            parent_id,
            ConstraintRole::PhysicalBoundary,
            &sources,
            Some(SegmentOrigin::Source),
        );
        *roles.traversals.entry(parent).or_insert(0) = 1;

        // The split vertex and the child toward `q`.
        let split_v = find_vertex(&cdt, point(2.0, 0.0));
        let q = find_vertex(&cdt, point(10.0, 0.0));
        let child1 = cdt
            .get_edge_from_neighbors(split_v, q)
            .filter(|e| e.is_constraint_edge())
            .expect("child edge exists")
            .as_undirected()
            .fix();
        let child0 = parent;

        roles.repair_split(&mut cdt, parent, child0, child1);

        let claims0 = cdt.undirected_edge_data_mut(child0).data().claims.clone();
        let claims1 = cdt.undirected_edge_data_mut(child1).data().claims.clone();
        assert_eq!(claims0.len(), 1, "E0 keeps the parent payload");
        assert_eq!(claims0[0].semantic_id, parent_id);
        assert_eq!(claims1.len(), 1, "E1 inherits via repair");
        assert_eq!(claims1[0].semantic_id, parent_id);
        assert_eq!(claims1[0].source_uses, sources);
        assert_eq!(
            ConstraintRoles::role_of(&cdt, child0),
            Some(ConstraintRole::PhysicalBoundary),
            "role_of on E0",
        );
        assert_eq!(
            ConstraintRoles::role_of(&cdt, child1),
            Some(ConstraintRole::PhysicalBoundary),
            "role_of on E1 after repair",
        );
        assert_eq!(roles.traversals.get(&child0), Some(&1));
        assert_eq!(roles.traversals.get(&child1), Some(&1));
    }

    /// B-test 3: traversal multiplicity survives the split exactly, for several
    /// multiplicities — future material parity reads it mod 2.
    #[test]
    fn traversal_multiplicity_survives_split() {
        for t in [1usize, 2, 3] {
            let (mut cdt, parent, _chain) = crossing_scenario();
            let mut roles = ConstraintRoles::default();
            let id = roles.mint_semantic_constraint_id();
            let parent_directed = directed_of(&cdt, parent);
            roles.label_realized_chain(
                &mut cdt,
                &[parent_directed],
                id,
                ConstraintRole::PhysicalBoundary,
                &[],
                Some(SegmentOrigin::Source),
            );
            *roles.traversals.entry(parent).or_insert(0) = t;

            let split_v = find_vertex(&cdt, point(2.0, 0.0));
            let q = find_vertex(&cdt, point(10.0, 0.0));
            let child1 = cdt
                .get_edge_from_neighbors(split_v, q)
                .filter(|e| e.is_constraint_edge())
                .expect("child edge exists")
                .as_undirected()
                .fix();
            roles.repair_split(&mut cdt, parent, parent, child1);

            assert_eq!(
                roles.traversals.get(&parent),
                Some(&t),
                "E0 inherits the parent's {t} traversals",
            );
            assert_eq!(
                roles.traversals.get(&child1),
                Some(&t),
                "E1 inherits the parent's {t} traversals",
            );
        }
    }

    /// B-test 4: repair appends inherited claims rather than replacing a child's
    /// existing payload; first-role semantics stay deterministic.
    #[test]
    fn existing_claims_are_not_destroyed_by_repair() {
        let (mut cdt, parent, _chain) = crossing_scenario();
        let mut roles = ConstraintRoles::default();
        let parent_id = roles.mint_semantic_constraint_id();
        let parent_directed = directed_of(&cdt, parent);
        roles.label_realized_chain(
            &mut cdt,
            &[parent_directed],
            parent_id,
            ConstraintRole::PhysicalBoundary,
            &[],
            Some(SegmentOrigin::Source),
        );

        let split_v = find_vertex(&cdt, point(2.0, 0.0));
        let q = find_vertex(&cdt, point(10.0, 0.0));
        let child1 = cdt
            .get_edge_from_neighbors(split_v, q)
            .filter(|e| e.is_constraint_edge())
            .expect("child edge exists")
            .as_undirected()
            .fix();
        let child0 = parent;

        // A later, different semantic claim lands on child1 before repair.
        let later_id = roles.mint_semantic_constraint_id();
        cdt.undirected_edge_data_mut(child1)
            .data_mut()
            .claims
            .push(ConstraintClaim {
                semantic_id: later_id,
                role: ConstraintRole::UnresolvedSyntheticClosure,
                source_uses: Vec::new(),
            });

        roles.repair_split(&mut cdt, parent, child0, child1);

        let claims1 = cdt.undirected_edge_data_mut(child1).data().claims.clone();
        assert_eq!(
            claims1.len(),
            2,
            "existing claim preserved, inherited appended"
        );
        assert_eq!(
            claims1[0].semantic_id, later_id,
            "the existing claim stays first"
        );
        assert_eq!(
            claims1[1].semantic_id, parent_id,
            "the inherited claim is appended"
        );
        assert_eq!(
            ConstraintRoles::role_of(&cdt, child1),
            Some(ConstraintRole::UnresolvedSyntheticClosure),
            "first-role semantics remain deterministic",
        );
    }

    /// B-test 5: the fallback-relocation repair primitive moves a lost edge's
    /// claims and combines traversal counts onto replacement edges, preserving
    /// the replacements' own claims first.
    #[test]
    fn relocation_repair_moves_claims_and_traversals() {
        let mut cdt = Cdt::new();
        let v00 = insert_vertex(&mut cdt, point(0.0, 0.0));
        let v10 = insert_vertex(&mut cdt, point(10.0, 0.0));
        let v11 = insert_vertex(&mut cdt, point(10.0, 10.0));
        let v01 = insert_vertex(&mut cdt, point(0.0, 10.0));
        // E: bottom, P: left, N: right — three non-crossing constraints.
        assert!(!cdt.try_add_constraint(v00, v10).is_empty(), "E added");
        assert!(!cdt.try_add_constraint(v00, v01).is_empty(), "P added");
        assert!(!cdt.try_add_constraint(v10, v11).is_empty(), "N added");
        let e = find_edge(&cdt, point(0.0, 0.0), point(10.0, 0.0));
        let p = find_edge(&cdt, point(0.0, 0.0), point(0.0, 10.0));
        let n = find_edge(&cdt, point(10.0, 0.0), point(10.0, 10.0));

        let mut roles = ConstraintRoles::default();
        // E carries its own claim and two traversals.
        let e_id = roles.mint_semantic_constraint_id();
        let e_directed = directed_of(&cdt, e);
        roles.label_realized_chain(
            &mut cdt,
            &[e_directed],
            e_id,
            ConstraintRole::PhysicalBoundary,
            &[],
            Some(SegmentOrigin::Source),
        );
        *roles.traversals.entry(e).or_insert(0) = 2;
        // P carries a pre-existing claim and one traversal.
        let p_id = roles.mint_semantic_constraint_id();
        let p_directed = directed_of(&cdt, p);
        roles.label_realized_chain(
            &mut cdt,
            &[p_directed],
            p_id,
            ConstraintRole::SurfaceSampling,
            &[],
            None,
        );
        *roles.traversals.entry(p).or_insert(0) = 1;

        // The fallback state transition: E loses its constraint, P and N gain
        // it. The primitive moves the data; the constraint-bit bookkeeping is
        // PLANAR-C's call from a before/after snapshot.
        let replacements = [directed_of(&cdt, p), directed_of(&cdt, n)];
        roles.repair_relocation(&mut cdt, e, &replacements);

        let p_claims = cdt.undirected_edge_data_mut(p).data().claims.clone();
        assert_eq!(p_claims.len(), 2, "P keeps its own claim and receives E's");
        assert_eq!(p_claims[0].semantic_id, p_id, "P's own claim stays first");
        assert_eq!(p_claims[1].semantic_id, e_id, "E's claim is appended");
        let n_claims = cdt.undirected_edge_data_mut(n).data().claims.clone();
        assert_eq!(n_claims.len(), 1, "N receives E's claim");
        assert_eq!(n_claims[0].semantic_id, e_id);
        // Traversal counts combine rather than overwrite.
        assert_eq!(roles.traversals.get(&p), Some(&3), "P combines 1 + 2");
        assert_eq!(roles.traversals.get(&n), Some(&2), "N inherits E's 2");
        assert_eq!(
            ConstraintRoles::role_of(&cdt, p),
            Some(ConstraintRole::SurfaceSampling),
            "P's first role is its own",
        );
    }
}

/// PLANAR-C: the production crossing-splitting route.
///
/// These exercise [`ConstraintRoles::insert_with_split`] directly — the
/// semantic helper `insert_to` now uses for every non-duplicate boundary
/// segment — and assert the PLANAR-C invariants end to end: incoming claims
/// reach every realized child, blocker claims survive subdivision, traversal
/// multiplicity is preserved rather than divided, crossing vertices are not
/// forged source identities, and material parity is unchanged by subdivision.
#[cfg(test)]
mod planar_c_crossing_tests {
    use super::*;

    fn point(x: f64, y: f64) -> SPoint2 {
        SPoint2::new(x, y)
    }

    fn insert_vertex(cdt: &mut Cdt, p: SPoint2) -> FixedVertexHandle {
        cdt.insert(p).expect("vertex insertion succeeds")
    }

    fn find_vertex(cdt: &Cdt, p: SPoint2) -> FixedVertexHandle {
        cdt.vertices()
            .find(|v| v.as_ref() == &p)
            .expect("vertex exists")
            .fix()
    }

    fn find_edge(cdt: &Cdt, a: SPoint2, b: SPoint2) -> FixedUndirectedEdgeHandle {
        let va = find_vertex(cdt, a);
        let vb = find_vertex(cdt, b);
        cdt.get_edge_from_neighbors(va, vb)
            .expect("edge exists")
            .as_undirected()
            .fix()
    }

    fn directed_of(cdt: &Cdt, e: FixedUndirectedEdgeHandle) -> FixedDirectedEdgeHandle {
        cdt.undirected_edge(e).as_directed().fix()
    }

    fn source_use(index: usize) -> SourceEdgeUse {
        SourceEdgeUse {
            bound: BoundId(0),
            index,
            orientation: true,
        }
    }

    fn claims_of(cdt: &Cdt, e: FixedUndirectedEdgeHandle) -> Vec<ConstraintClaim> {
        cdt.undirected_edge(e).data().data().claims.clone()
    }

    fn label_blocker(
        cdt: &mut Cdt,
        roles: &mut ConstraintRoles,
        a: FixedVertexHandle,
        b: FixedVertexHandle,
    ) -> (FixedUndirectedEdgeHandle, SemanticConstraintId) {
        let id = roles.mint_semantic_constraint_id();
        let chain = cdt.try_add_constraint(a, b);
        assert!(!chain.is_empty(), "the blocking constraint is added");
        roles.label_realized_chain(
            cdt,
            &chain,
            id,
            ConstraintRole::PhysicalBoundary,
            &[source_use(7)],
            Some(SegmentOrigin::Source),
        );
        let handle = cdt
            .get_edge_from_neighbors(a, b)
            .unwrap()
            .as_undirected()
            .fix();
        *roles.traversals.entry(handle).or_insert(0) = 1;
        (handle, id)
    }

    /// The minimal crossing scenario, through the PLANAR-C helper: blocking
    /// constraint `(0,0)-(10,0)` on the x-axis, incoming chord `(2,-5)-(2,5)`
    /// crossing it at `(2,0)`.
    fn crossing_via_helper() -> (
        Cdt,
        ConstraintRoles,
        FixedUndirectedEdgeHandle,
        CrossingSplitReport,
        SemanticConstraintId,
    ) {
        let mut cdt = Cdt::new();
        let a = insert_vertex(&mut cdt, point(2.0, -5.0));
        let b = insert_vertex(&mut cdt, point(2.0, 5.0));
        let p = insert_vertex(&mut cdt, point(0.0, 0.0));
        let q = insert_vertex(&mut cdt, point(10.0, 0.0));
        let mut roles = ConstraintRoles::default();
        let (blocker, _blocker_id) = label_blocker(&mut cdt, &mut roles, p, q);
        let incoming_id = roles.mint_semantic_constraint_id();
        let report = roles
            .insert_with_split(
                &mut cdt,
                a,
                b,
                incoming_id,
                ConstraintRole::PhysicalBoundary,
                &[source_use(0)],
                Some(SegmentOrigin::Source),
            )
            .expect("planar split succeeds");
        (cdt, roles, blocker, report, incoming_id)
    }

    /// C-test 1: one proper crossing subdivides both the incoming segment and
    /// the blocker into two atomic constrained edges sharing the crossing
    /// vertex, and the report accounts for it.
    #[test]
    fn proper_crossing_splits_blocker_and_incoming() {
        let (cdt, _roles, blocker, report, _id) = crossing_via_helper();
        assert_eq!(report.blockers_crossed, 1);
        assert_eq!(report.blockers_split, 1);
        assert_eq!(report.blockers_relocated, 0);
        assert_eq!(report.split_vertices, 1);
        assert_eq!(report.chain.len(), 2, "the incoming chord is split in two");

        let a = find_vertex(&cdt, point(2.0, -5.0));
        let b = find_vertex(&cdt, point(2.0, 5.0));
        let x = find_vertex(&cdt, point(2.0, 0.0));
        let p = find_vertex(&cdt, point(0.0, 0.0));
        let q = find_vertex(&cdt, point(10.0, 0.0));
        let directed: Vec<_> = report.chain.iter().map(|h| cdt.directed_edge(*h)).collect();
        assert_eq!(
            directed[0].from().fix(),
            a,
            "the chain starts at the request"
        );
        assert_eq!(directed[0].to().fix(), x, "the chain passes through X");
        assert_eq!(directed[1].from().fix(), x);
        assert_eq!(directed[1].to().fix(), b, "the chain ends at the request");

        // The blocker is now two constraint edges through X.
        let child0 = find_edge(&cdt, point(0.0, 0.0), point(2.0, 0.0));
        let child1 = find_edge(&cdt, point(2.0, 0.0), point(10.0, 0.0));
        assert!(cdt.is_constraint_edge(child0));
        assert!(cdt.is_constraint_edge(child1));
        assert_eq!(child0, blocker, "E0 keeps the blocker handle");
        assert_ne!(child1, blocker, "E1 is a new child");
        // The crossing vertex is shared by four constraint edges.
        let mut incident = 0usize;
        for e in cdt.undirected_edges() {
            if !e.is_constraint_edge() {
                continue;
            }
            let d = e.as_directed();
            if d.from().fix() == x || d.to().fix() == x {
                incident += 1;
            }
        }
        assert_eq!(incident, 4, "four atomic constraint edges meet at X");
        let _ = (p, q);
    }

    /// C-test 2: every realized child of the incoming segment carries the one
    /// incoming semantic claim, role, and source uses.
    #[test]
    fn incoming_claim_reaches_every_child() {
        let (cdt, _roles, _blocker, report, incoming_id) = crossing_via_helper();
        for h in &report.chain {
            let handle = cdt.directed_edge(*h).as_undirected().fix();
            let claims = claims_of(&cdt, handle);
            assert_eq!(claims.len(), 1, "exactly the incoming claim");
            assert_eq!(claims[0].semantic_id, incoming_id);
            assert_eq!(claims[0].role, ConstraintRole::PhysicalBoundary);
            assert_eq!(claims[0].source_uses, vec![source_use(0)]);
            assert_eq!(
                ConstraintRoles::role_of(&cdt, handle),
                Some(ConstraintRole::PhysicalBoundary),
                "the role is readable on every child",
            );
        }
    }

    /// C-test 3: the blocker's claim survives the split on both children.
    #[test]
    fn blocker_claim_reaches_both_children() {
        let (cdt, _roles, blocker, _report, _incoming_id) = crossing_via_helper();
        let child0 = find_edge(&cdt, point(0.0, 0.0), point(2.0, 0.0));
        let child1 = find_edge(&cdt, point(2.0, 0.0), point(10.0, 0.0));
        assert_eq!(child0, blocker);
        for child in [child0, child1] {
            let claims = claims_of(&cdt, child);
            assert_eq!(claims.len(), 1, "one blocker claim on the child");
            assert_eq!(claims[0].role, ConstraintRole::PhysicalBoundary);
            assert_eq!(claims[0].source_uses, vec![source_use(7)]);
        }
        assert_eq!(
            ConstraintRoles::role_of(&cdt, child1),
            Some(ConstraintRole::PhysicalBoundary),
            "E1's inherited role is readable after repair",
        );
    }

    /// C-test 4: traversal multiplicity survives the crossing split exactly —
    /// both children inherit `t`, never `t / 2`.
    #[test]
    fn traversal_multiplicity_survives_crossing_split() {
        for t in [1usize, 2, 3] {
            let mut cdt = Cdt::new();
            let a = insert_vertex(&mut cdt, point(2.0, -5.0));
            let b = insert_vertex(&mut cdt, point(2.0, 5.0));
            let p = insert_vertex(&mut cdt, point(0.0, 0.0));
            let q = insert_vertex(&mut cdt, point(10.0, 0.0));
            let mut roles = ConstraintRoles::default();
            let (blocker, _id) = label_blocker(&mut cdt, &mut roles, p, q);
            *roles.traversals.entry(blocker).or_insert(0) = t;

            let incoming_id = roles.mint_semantic_constraint_id();
            roles
                .insert_with_split(
                    &mut cdt,
                    a,
                    b,
                    incoming_id,
                    ConstraintRole::PhysicalBoundary,
                    &[],
                    Some(SegmentOrigin::Source),
                )
                .expect("planar split succeeds");

            let child0 = find_edge(&cdt, point(0.0, 0.0), point(2.0, 0.0));
            let child1 = find_edge(&cdt, point(2.0, 0.0), point(10.0, 0.0));
            assert_eq!(
                roles.traversals.get(&child0),
                Some(&t),
                "E0 inherits the parent's {t} traversals",
            );
            assert_eq!(
                roles.traversals.get(&child1),
                Some(&t),
                "E1 inherits the parent's {t} traversals",
            );
        }
    }

    /// C-test 5: a blocker carrying several claims passes every one of them to
    /// its split child; first-role semantics stay deterministic.
    #[test]
    fn blocker_with_multiple_claims_copies_to_child() {
        let mut cdt = Cdt::new();
        let a = insert_vertex(&mut cdt, point(2.0, -5.0));
        let b = insert_vertex(&mut cdt, point(2.0, 5.0));
        let p = insert_vertex(&mut cdt, point(0.0, 0.0));
        let q = insert_vertex(&mut cdt, point(10.0, 0.0));
        let mut roles = ConstraintRoles::default();
        let chain = cdt.try_add_constraint(p, q);
        let first = roles.mint_semantic_constraint_id();
        roles.label_realized_chain(
            &mut cdt,
            &chain,
            first,
            ConstraintRole::PhysicalBoundary,
            &[],
            Some(SegmentOrigin::Source),
        );
        let blocker = cdt
            .get_edge_from_neighbors(p, q)
            .unwrap()
            .as_undirected()
            .fix();
        let second = roles.mint_semantic_constraint_id();
        let directed = directed_of(&cdt, blocker);
        roles.label_realized_chain(
            &mut cdt,
            &[directed],
            second,
            ConstraintRole::UnresolvedSyntheticClosure,
            &[],
            Some(SegmentOrigin::SyntheticClosure),
        );
        assert_eq!(
            claims_of(&cdt, blocker).len(),
            2,
            "two claims before the split"
        );

        let incoming_id = roles.mint_semantic_constraint_id();
        roles
            .insert_with_split(
                &mut cdt,
                a,
                b,
                incoming_id,
                ConstraintRole::PhysicalBoundary,
                &[],
                Some(SegmentOrigin::Source),
            )
            .expect("planar split succeeds");

        let child0 = find_edge(&cdt, point(0.0, 0.0), point(2.0, 0.0));
        let child1 = find_edge(&cdt, point(2.0, 0.0), point(10.0, 0.0));
        for child in [child0, child1] {
            let claims = claims_of(&cdt, child);
            assert_eq!(claims.len(), 2, "every blocker claim survives the split");
            assert_eq!(claims[0].semantic_id, first, "first-role stays first");
            assert_eq!(claims[1].semantic_id, second, "later claim is preserved");
        }
    }

    /// C-test 6: one incoming segment crossing several blockers splits each of
    /// them, and the realized chain covers the whole request.
    #[test]
    fn incoming_crossing_multiple_blockers() {
        let mut cdt = Cdt::new();
        let a = insert_vertex(&mut cdt, point(5.0, -5.0));
        let b = insert_vertex(&mut cdt, point(5.0, 15.0));
        let p0 = insert_vertex(&mut cdt, point(0.0, 0.0));
        let q0 = insert_vertex(&mut cdt, point(10.0, 0.0));
        let p1 = insert_vertex(&mut cdt, point(0.0, 10.0));
        let q1 = insert_vertex(&mut cdt, point(10.0, 10.0));
        let mut roles = ConstraintRoles::default();
        let (blocker0, _) = label_blocker(&mut cdt, &mut roles, p0, q0);
        let (blocker1, _) = label_blocker(&mut cdt, &mut roles, p1, q1);

        let incoming_id = roles.mint_semantic_constraint_id();
        let report = roles
            .insert_with_split(
                &mut cdt,
                a,
                b,
                incoming_id,
                ConstraintRole::PhysicalBoundary,
                &[source_use(3)],
                Some(SegmentOrigin::Source),
            )
            .expect("planar split succeeds");

        assert_eq!(report.blockers_crossed, 2);
        assert_eq!(report.blockers_split, 2);
        assert_eq!(report.split_vertices, 2);
        assert_eq!(
            report.chain.len(),
            3,
            "two splits divide the chord into three"
        );
        assert_eq!(
            roles.traversals.get(&blocker0),
            Some(&1),
            "blocker0 survives with its traversal",
        );
        assert_eq!(
            roles.traversals.get(&blocker1),
            Some(&1),
            "blocker1 survives with its traversal",
        );
        for split in [point(5.0, 0.0), point(5.0, 10.0)] {
            let x = find_vertex(&cdt, split);
            let mut incident = 0usize;
            for e in cdt.undirected_edges() {
                if !e.is_constraint_edge() {
                    continue;
                }
                let d = e.as_directed();
                if d.from().fix() == x || d.to().fix() == x {
                    incident += 1;
                }
            }
            assert_eq!(incident, 4, "each crossing vertex is a 4-way junction");
        }
        // Every realized child of the incoming request carries the incoming use.
        for h in &report.chain {
            let handle = cdt.directed_edge(*h).as_undirected().fix();
            let claims = claims_of(&cdt, handle);
            assert!(claims.iter().any(|c| c.semantic_id == incoming_id));
            assert!(claims.iter().any(|c| c.source_uses == vec![source_use(3)]));
        }
    }

    /// C-test 7: material meaning is attached to atomic constrained edges and
    /// is unchanged by crossing subdivision. A self-crossing "bow-tie" loop
    /// whose two diagonals cross at `(5,5)` — the H2 signature, a single bound
    /// folding over itself — selects both lobes as material: the crossing
    /// vertex is a 4-way junction of toggling edges, so the flood is consistent
    /// and the subdivided arrangement selects exactly the same region a proper
    /// planarization of the loop must.
    #[test]
    fn material_region_unchanged_by_crossing_subdivision() {
        let mut cdt = Cdt::new();
        let a = insert_vertex(&mut cdt, point(0.0, 0.0));
        let b = insert_vertex(&mut cdt, point(10.0, 0.0));
        let c = insert_vertex(&mut cdt, point(0.0, 10.0));
        let d = insert_vertex(&mut cdt, point(10.0, 10.0));
        let mut roles = ConstraintRoles::default();
        // The two non-crossing edges first, then the diagonals, so the second
        // diagonal properly crosses the first one.
        for (p, q) in [(a, b), (c, d)] {
            let id = roles.mint_semantic_constraint_id();
            let chain = cdt.try_add_constraint(p, q);
            roles.label_realized_chain(
                &mut cdt,
                &chain,
                id,
                ConstraintRole::PhysicalBoundary,
                &[],
                Some(SegmentOrigin::Source),
            );
        }
        let diag1_id = roles.mint_semantic_constraint_id();
        let chain = cdt.try_add_constraint(b, c);
        roles.label_realized_chain(
            &mut cdt,
            &chain,
            diag1_id,
            ConstraintRole::PhysicalBoundary,
            &[],
            Some(SegmentOrigin::Source),
        );
        let diag2_id = roles.mint_semantic_constraint_id();
        let report = roles
            .insert_with_split(
                &mut cdt,
                d,
                a,
                diag2_id,
                ConstraintRole::PhysicalBoundary,
                &[],
                Some(SegmentOrigin::Source),
            )
            .expect("planar split succeeds");
        assert_eq!(report.blockers_crossed, 1);
        assert_eq!(report.blockers_split, 1);
        assert_eq!(report.chain.len(), 2, "the second diagonal is subdivided");

        // The crossing vertex is a 4-way junction of toggling constraint edges,
        // so the dual flood cannot contradict itself there.
        let x = find_vertex(&cdt, point(5.0, 5.0));
        let mut incident = 0usize;
        for e in cdt.undirected_edges() {
            if !e.is_constraint_edge() {
                continue;
            }
            let d = e.as_directed();
            if d.from().fix() == x || d.to().fix() == x {
                incident += 1;
            }
        }
        assert_eq!(
            incident, 4,
            "the crossing is a subdivision, not a parity event"
        );

        // Every constraint edge carries a role, so the flood never trips
        // `ConstraintRoleMissing`.
        for e in cdt.undirected_edges() {
            if !e.is_constraint_edge() {
                continue;
            }
            assert!(
                ConstraintRoles::role_of(&cdt, e.fix()).is_some(),
                "every constraint edge carries a role",
            );
        }

        let parity = flood_parity(&cdt, &roles, ParityReading::TraversalParity)
            .expect("the subdivided arrangement floods consistently");
        let mut material_area = 0.0f64;
        for face in cdt.inner_faces() {
            if parity.get(&face.index()) != Some(&1) {
                continue;
            }
            let verts = face.vertices();
            let [a, b, c] = verts.map(|v| *v.as_ref());
            material_area += 0.5 * ((b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)).abs();
        }
        assert!(
            (material_area - 50.0).abs() < 1e-9,
            "both lobes of the bow-tie are material, got area {material_area}",
        );
    }

    /// C-test 8: a segment with no crossing realizes as one labeled edge and
    /// touches nothing else.
    #[test]
    fn non_crossing_segment_unchanged() {
        let mut cdt = Cdt::new();
        let a = insert_vertex(&mut cdt, point(2.0, -5.0));
        let b = insert_vertex(&mut cdt, point(2.0, 5.0));
        let mut roles = ConstraintRoles::default();
        let id = roles.mint_semantic_constraint_id();
        let report = roles
            .insert_with_split(
                &mut cdt,
                a,
                b,
                id,
                ConstraintRole::PhysicalBoundary,
                &[source_use(1)],
                Some(SegmentOrigin::Source),
            )
            .expect("planar split succeeds");
        assert_eq!(report.blockers_crossed, 0);
        assert_eq!(report.blockers_split, 0);
        assert_eq!(report.blockers_relocated, 0);
        assert_eq!(report.split_vertices, 0);
        assert_eq!(report.chain.len(), 1, "no subdivision for a clear segment");
        assert_eq!(cdt.num_constraints(), 1, "exactly one new constraint edge");
        let handle = cdt.directed_edge(report.chain[0]).as_undirected().fix();
        let claims = claims_of(&cdt, handle);
        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].semantic_id, id);
        assert_eq!(claims[0].source_uses, vec![source_use(1)]);
        assert_eq!(roles.traversals.get(&handle), Some(&1));
    }

    /// C-test 9: a full duplicate/retrace creates no second CDT edge — the
    /// ARR-SEAM admission is preserved through the splitting helper.
    #[test]
    fn duplicate_retrace_creates_no_second_edge() {
        let mut cdt = Cdt::new();
        let a = insert_vertex(&mut cdt, point(0.0, 0.0));
        let b = insert_vertex(&mut cdt, point(10.0, 0.0));
        let mut roles = ConstraintRoles::default();
        let first = roles.mint_semantic_constraint_id();
        let chain = cdt.try_add_constraint(a, b);
        roles.label_realized_chain(
            &mut cdt,
            &chain,
            first,
            ConstraintRole::PhysicalBoundary,
            &[],
            Some(SegmentOrigin::Source),
        );
        let before = cdt.num_constraints();
        let handle = cdt
            .get_edge_from_neighbors(a, b)
            .unwrap()
            .as_undirected()
            .fix();
        assert_eq!(roles.traversals.get(&handle), Some(&1));

        let second = roles.mint_semantic_constraint_id();
        let report = roles
            .insert_with_split(
                &mut cdt,
                a,
                b,
                second,
                ConstraintRole::PhysicalBoundary,
                &[source_use(9)],
                Some(SegmentOrigin::Source),
            )
            .expect("planar split succeeds");
        assert_eq!(
            cdt.num_constraints(),
            before,
            "a retrace makes no second CDT edge"
        );
        assert_eq!(
            report.chain.len(),
            1,
            "the retrace realizes as the existing edge"
        );
        assert_eq!(
            roles.traversals.get(&handle),
            Some(&2),
            "the retrace is a second traversal, not a new edge",
        );
        let claims = claims_of(&cdt, handle);
        assert_eq!(claims.len(), 2, "both claims are retained on the one edge");
        assert_eq!(claims[0].semantic_id, first);
        assert_eq!(claims[1].semantic_id, second);
        assert_eq!(
            ConstraintRoles::role_of(&cdt, handle),
            Some(ConstraintRole::PhysicalBoundary),
            "first-role semantics are unchanged by the retrace",
        );
    }

    /// C-test 10: the PLANAR-A source uses propagate exactly onto every
    /// realized incoming child.
    #[test]
    fn source_uses_propagate_to_incoming_children() {
        let (cdt, _roles, _blocker, report, _incoming_id) = crossing_via_helper();
        assert_eq!(report.chain.len(), 2);
        for h in &report.chain {
            let handle = cdt.directed_edge(*h).as_undirected().fix();
            let claims = claims_of(&cdt, handle);
            assert_eq!(
                claims[0].source_uses,
                vec![source_use(0)],
                "the exact contributor reaches every child",
            );
        }
    }

    /// C-test 11: after any number of crossings there are no anonymous
    /// constrained edges — every one carries a resolvable role, so the parity
    /// flood never trips `ConstraintRoleMissing`.
    #[test]
    fn no_anonymous_constraint_edges() {
        let mut cdt = Cdt::new();
        let a = insert_vertex(&mut cdt, point(5.0, -5.0));
        let b = insert_vertex(&mut cdt, point(5.0, 25.0));
        let p0 = insert_vertex(&mut cdt, point(0.0, 0.0));
        let q0 = insert_vertex(&mut cdt, point(10.0, 0.0));
        let p1 = insert_vertex(&mut cdt, point(0.0, 10.0));
        let q1 = insert_vertex(&mut cdt, point(10.0, 10.0));
        let p2 = insert_vertex(&mut cdt, point(0.0, 20.0));
        let q2 = insert_vertex(&mut cdt, point(10.0, 20.0));
        let mut roles = ConstraintRoles::default();
        for (p, q) in [(p0, q0), (p1, q1), (p2, q2)] {
            let id = roles.mint_semantic_constraint_id();
            let chain = cdt.try_add_constraint(p, q);
            roles.label_realized_chain(
                &mut cdt,
                &chain,
                id,
                ConstraintRole::PhysicalBoundary,
                &[],
                Some(SegmentOrigin::Source),
            );
        }
        let incoming_id = roles.mint_semantic_constraint_id();
        let report = roles
            .insert_with_split(
                &mut cdt,
                a,
                b,
                incoming_id,
                ConstraintRole::PhysicalBoundary,
                &[],
                Some(SegmentOrigin::Source),
            )
            .expect("planar split succeeds");
        assert_eq!(report.blockers_split, 3);

        for e in cdt.undirected_edges() {
            if !e.is_constraint_edge() {
                continue;
            }
            assert!(
                ConstraintRoles::role_of(&cdt, e.fix()).is_some(),
                "every realized constraint edge carries a role",
            );
        }
        // And the flood can walk the whole arrangement without a missing role.
        assert_eq!(roles.unresolved_at_flood.get(), 0);
    }

    /// C-test 12: an endpoint lying on an existing blocker's interior is
    /// accepted naturally — the blocker was realized as a chain through that
    /// vertex, and a segment departing from it adds no spurious split.
    #[test]
    fn endpoint_on_interior_vertex_is_accepted() {
        let mut cdt = Cdt::new();
        let a = insert_vertex(&mut cdt, point(0.0, 0.0));
        let b = insert_vertex(&mut cdt, point(10.0, 0.0));
        let x = insert_vertex(&mut cdt, point(5.0, 0.0));
        let p = insert_vertex(&mut cdt, point(5.0, 5.0));
        let q = insert_vertex(&mut cdt, point(5.0, -5.0));
        let mut roles = ConstraintRoles::default();
        // The blocker (0,0)-(10,0) is realized as a chain through the existing
        // interior vertex x.
        let blocker_id = roles.mint_semantic_constraint_id();
        let chain = cdt.try_add_constraint(a, b);
        assert_eq!(chain.len(), 2, "the blocker realizes as a chain through x");
        roles.label_realized_chain(
            &mut cdt,
            &chain,
            blocker_id,
            ConstraintRole::PhysicalBoundary,
            &[],
            Some(SegmentOrigin::Source),
        );

        // A segment from x (on the blocker's interior) out to p is accepted.
        let incoming_id = roles.mint_semantic_constraint_id();
        let report = roles
            .insert_with_split(
                &mut cdt,
                x,
                p,
                incoming_id,
                ConstraintRole::PhysicalBoundary,
                &[source_use(2)],
                Some(SegmentOrigin::Source),
            )
            .expect("planar split succeeds");
        assert_eq!(report.blockers_crossed, 0, "no blocker is crossed");
        assert_eq!(report.blockers_split, 0);
        assert_eq!(report.chain.len(), 1);
        // The blocker's own two children keep their claims.
        let child0 = find_edge(&cdt, point(0.0, 0.0), point(5.0, 0.0));
        let child1 = find_edge(&cdt, point(5.0, 0.0), point(10.0, 0.0));
        for child in [child0, child1] {
            let claims = claims_of(&cdt, child);
            assert_eq!(claims.len(), 1);
            assert_eq!(claims[0].semantic_id, blocker_id);
        }
        let _ = q;
    }

    /// C-test 13: a vertex inserted exactly on an existing constraint edge
    /// splits it, and Spade's fresh child arrives without a claim. The backstop
    /// repair attaches the surviving half's claim to it, so the flood never
    /// trips `ConstraintRoleMissing` for a split the insertion API performed
    /// outside `insert_with_split`.
    #[test]
    fn vertex_insertion_split_child_is_repaired() {
        let mut cdt = Cdt::new();
        let a = insert_vertex(&mut cdt, point(0.0, 0.0));
        let b = insert_vertex(&mut cdt, point(10.0, 0.0));
        let mut roles = ConstraintRoles::default();
        let id = roles.mint_semantic_constraint_id();
        let chain = cdt.try_add_constraint(a, b);
        roles.label_realized_chain(
            &mut cdt,
            &chain,
            id,
            ConstraintRole::PhysicalBoundary,
            &[],
            Some(SegmentOrigin::Source),
        );

        // Inserting a vertex on the constraint's interior splits it. Whether
        // Spade accepts the exact-on-edge point or lands one ULP beside it, the
        // survivor keeps the claim and any fresh child must be repaired.
        let _ = cdt.insert(point(5.0, 0.0));
        roles.repair_unlabeled_constraint_edges(&mut cdt);

        let mut constraint_count = 0usize;
        let mut unlabeled = 0usize;
        for e in cdt.undirected_edges() {
            if !e.is_constraint_edge() {
                continue;
            }
            constraint_count += 1;
            if e.data().data().claims.is_empty() {
                unlabeled += 1;
            }
        }
        assert_eq!(constraint_count, 2, "the split yields two constraint edges");
        assert_eq!(unlabeled, 0, "the backstop repaired the split child");
        assert_eq!(roles.unresolved_at_flood.get(), 0);
    }
}

#[cfg(test)]
mod proj003_stage_a_tests {
    use super::*;
    use truck_geotrait::algo::surface::SsnpVector;
    use truck_modeling::{BSplineSurface, KnotVec, Line, Point3, RevolutedCurve, Vector3};

    /// A 2x2 bilinear patch over `(u, v) in [0,1] x [0,1]` lying in the plane
    /// `z = 0`. Its parameter range is declared by its knot vector, so domain
    /// admission is actually testable on it.
    fn bilinear_patch() -> BSplineSurface<Point3> {
        let knot = KnotVec::from(vec![0.0, 0.0, 1.0, 1.0]);
        let ctrl = vec![
            vec![Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0)],
            vec![Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0)],
        ];
        BSplineSurface::new((knot.clone(), knot), ctrl)
    }

    fn outcome(uv: (f64, f64), residual: f64, in_domain: bool, converged: bool) -> NearestOutcome {
        NearestOutcome {
            uv,
            residual,
            converged,
            degenerate: false,
            in_domain,
            iterations: 0,
        }
    }

    fn attempt_with(prod_best: NearestOutcome) -> ProjectionAttempt {
        let mut attempt = ProjectionAttempt::default();
        attempt.prod_best = prod_best;
        attempt
    }

    /// ProductionMiss: Newton did not satisfy its `near2` gate (`converged:
    /// false`), but the best iterate from a production start is finite, inside
    /// the declared domain, and evaluates within the caller's tolerance. The
    /// tessellation recovery must admit it even though the legacy API reports
    /// the search as not converged.
    #[test]
    fn residual_certified_production_start_is_admitted() {
        let surface = bilinear_patch();
        let point = Point3::new(0.5, 0.5, 0.0);
        let attempt = attempt_with(outcome((0.5, 0.5), 0.0, true, false));
        let admitted = residual_certified_admission(&surface, point, 1.0e-3, attempt)
            .expect("a finite, in-domain, within-tol iterate must be admitted");
        assert_eq!(
            admitted.0,
            (0.5, 0.5),
            "the certified candidate's UV is kept"
        );
        assert!(
            admitted.1 <= 1.0e-3,
            "the re-evaluated residual is certified"
        );
    }

    /// Genuine miss: the best iterate is finite and in-domain, but its world
    /// residual exceeds the caller's tolerance. Must remain rejected.
    #[test]
    fn residual_beyond_tolerance_stays_rejected() {
        let surface = bilinear_patch();
        let point = Point3::new(5.0, 5.0, 0.0);
        let attempt = attempt_with(outcome((0.5, 0.5), 6.4, true, false));
        assert_eq!(
            residual_certified_admission(&surface, point, 1.0e-3, attempt),
            None,
            "a candidate that re-evaluates beyond tolerance must not be admitted"
        );
    }

    /// Domain: a candidate within tolerance but outside the declared nonperiodic
    /// parameter range must remain rejected. Stage A never normalises or clamps.
    #[test]
    fn in_tolerance_but_out_of_domain_stays_rejected() {
        let surface = bilinear_patch();
        let point = Point3::new(0.0, 0.0, 0.0);
        let attempt = attempt_with(outcome((2.0, 0.5), 0.0, false, false));
        assert_eq!(
            residual_certified_admission(&surface, point, 1.0e-3, attempt),
            None,
            "an out-of-domain parameter is a Stage C question, not a Stage A admission"
        );
    }

    /// Non-finite UV must be rejected regardless of the reported residual.
    #[test]
    fn non_finite_uv_stays_rejected() {
        let surface = bilinear_patch();
        let point = Point3::new(0.5, 0.5, 0.0);
        let attempt = attempt_with(outcome((f64::NAN, 0.5), 0.0, true, false));
        assert_eq!(
            residual_certified_admission(&surface, point, 1.0e-3, attempt),
            None
        );
    }

    /// A probe that never ran (no finite residual) admits nothing.
    #[test]
    fn no_ran_probe_stays_rejected() {
        let surface = bilinear_patch();
        assert_eq!(
            residual_certified_admission(
                &surface,
                Point3::origin(),
                1.0e-3,
                ProjectionAttempt::default()
            ),
            None,
            "with prod_best = NONE there is no candidate to certify"
        );
    }

    /// Legacy invariance of the shared solver: `search_nearest_parameter_outcome`
    /// must report exactly the converged answer the legacy
    /// `search_nearest_parameter` (and a direct `newton::solve`) do, for every
    /// input in the battery â€” converged, non-converged, and degenerate. The
    /// production chain sees the identical result it always did.
    #[test]
    fn shared_outcome_matches_legacy_newton() {
        use truck_base::newton;
        let surface = bilinear_patch();
        let cases = [
            ((0.3, 0.7, 0.05), (0.2, 0.6)),  // on/near the patch, converges
            ((0.9, 0.1, 0.0), (0.5, 0.5)),   // on the patch
            ((5.0, 5.0, 5.0), (0.4, 0.4)),   // far away, may not converge
            ((-3.0, 2.0, 1.0), (0.8, 0.2)),  // far corner, may not converge
            ((0.0, 0.0, 0.0), (0.1, 0.1)),   // at the patch corner
            ((0.5, 0.5, -0.05), (0.5, 0.5)), // mirrored side
        ];
        for (p, hint) in cases {
            let point = Point3::new(p.0, p.1, p.2);
            let outcome =
                algo::surface::search_nearest_parameter_outcome(&surface, point, hint, 100);
            let legacy = surface.search_nearest_parameter(point, hint, 100);
            assert_eq!(
                outcome.converged, legacy,
                "outcome.converged must equal legacy search_nearest_parameter"
            );
            let function = |q: Vector3| SsnpVector::subs(&surface, point, q);
            let direct = newton::solve(function, Vector3::new(hint.0, hint.1, 0.0), 100);
            assert_eq!(
                outcome.converged,
                direct.ok().map(|v| (v.x, v.y)),
                "outcome.converged must come from newton::solve itself"
            );
            if let Some(uv) = outcome.converged {
                let residual = (surface.subs(uv.0, uv.1) - point).magnitude();
                let nearest = |(a, b): (f64, f64)| (point - surface.subs(a, b)).magnitude();
                let min_res = [0.0f64, 0.25, 0.5, 0.75, 1.0]
                    .into_iter()
                    .flat_map(|a| {
                        [0.0f64, 0.25, 0.5, 0.75, 1.0]
                            .into_iter()
                            .map(move |b| nearest((a, b)))
                    })
                    .fold(f64::INFINITY, f64::min);
                assert!(
                    residual <= min_res + 1.0e-9,
                    "converged answer is a stationary nearest point (residual {residual})"
                );
            }
        }
    }

    /// A unit cylinder around the z axis: `subs(u, v)` applies
    /// `rotation_matrix(v)`, so the `v` axis is genuinely periodic with period
    /// `2Ï€` and its declared range is `[0, 2Ï€)`.
    fn unit_cylinder() -> RevolutedCurve<Line<Point3>> {
        RevolutedCurve::by_revolution(
            Line::from_origin_direction(Point3::new(1.0, 0.0, 0.0), Vector3::unit_z()),
            Point3::origin(),
            Vector3::unit_z(),
        )
    }

    fn attempt_with_seed(seed_best: NearestOutcome) -> ProjectionAttempt {
        let mut attempt = ProjectionAttempt::default();
        attempt.seed_best = seed_best;
        attempt
    }

    // ---- PROJ-003 Stage B ----

    /// A structural-seed candidate that is finite, in-domain, and within the
    /// caller tolerance is admitted, exactly like a Stage A production-start
    /// candidate.
    #[test]
    fn structural_seed_candidate_is_admitted() {
        let surface = bilinear_patch();
        let point = Point3::new(0.5, 0.5, 0.0);
        let attempt = attempt_with_seed(outcome((0.5, 0.5), 0.0, true, false));
        let admitted = residual_certified_seed_admission(&surface, point, 1.0e-3, attempt)
            .expect("a finite, in-domain, within-tol seed iterate must be admitted");
        assert_eq!(
            admitted.0,
            (0.5, 0.5),
            "the certified candidate's UV is kept"
        );
        assert!(
            admitted.1 <= 1.0e-3,
            "the re-evaluated residual is certified"
        );
    }

    /// A structural-seed candidate beyond the caller tolerance stays rejected.
    #[test]
    fn structural_seed_beyond_tolerance_stays_rejected() {
        let surface = bilinear_patch();
        let point = Point3::new(5.0, 5.0, 0.0);
        let attempt = attempt_with_seed(outcome((0.5, 0.5), 6.4, true, false));
        assert_eq!(
            residual_certified_seed_admission(&surface, point, 1.0e-3, attempt),
            None,
            "a candidate that re-evaluates beyond tolerance must not be admitted"
        );
    }

    /// A structural-seed candidate outside the declared nonperiodic range stays
    /// rejected: Stage B never normalises or clamps, that is a Stage C
    /// question.
    #[test]
    fn structural_seed_out_of_domain_stays_rejected() {
        let surface = bilinear_patch();
        let point = Point3::new(0.0, 0.0, 0.0);
        let attempt = attempt_with_seed(outcome((2.0, 0.5), 0.0, false, false));
        assert_eq!(
            residual_certified_seed_admission(&surface, point, 1.0e-3, attempt),
            None,
            "an out-of-domain seed iterate is a Stage C question, not a Stage B admission"
        );
    }

    /// A Stage A (production-start) success is not replaced: when the seed
    /// search never ran, Stage B admits nothing, so the ordering of the chain
    /// is what keeps Stage A wins intact.
    #[test]
    fn stage_a_success_is_not_replaced_by_seed_admission() {
        let surface = bilinear_patch();
        let point = Point3::new(0.5, 0.5, 0.0);
        let mut attempt = ProjectionAttempt::default();
        attempt.prod_best = outcome((0.5, 0.5), 0.0, true, false);
        attempt.seed_best = NearestOutcome::NONE;
        assert_eq!(
            residual_certified_seed_admission(&surface, point, 1.0e-3, attempt),
            None,
            "with no seed search there is no Stage B candidate to certify"
        );
    }

    /// A probe that never ran (no finite residual) admits nothing.
    #[test]
    fn no_ran_seed_probe_stays_rejected() {
        let surface = bilinear_patch();
        assert_eq!(
            residual_certified_seed_admission(
                &surface,
                Point3::origin(),
                1.0e-3,
                ProjectionAttempt::default(),
            ),
            None,
            "with seed_best = NONE there is no candidate to certify"
        );
    }

    // ---- PROJ-003 Stage C ----

    fn cylinder_lattice() -> CertifiedLattice {
        use super::super::domain::lattice::Axis;
        CertifiedLattice::revolution(Axis::V, AxisPeriodStatus::NonPeriodic)
    }

    /// A candidate on a genuinely periodic axis that differs from an in-range
    /// representative by one certified period is normalized and admitted. The
    /// point `(-1, 0, 0.5)` lies on the cylinder at `v = Ï€`; the candidate at
    /// `v = 3Ï€` is deck-equivalent to it.
    #[test]
    fn periodic_equivalent_candidate_is_normalized_and_admitted() {
        let surface = unit_cylinder();
        let point = Point3::new(-1.0, 0.0, 0.5);
        let attempt = attempt_with_seed(outcome(
            (0.5, 3.0 * std::f64::consts::PI),
            0.0,
            false,
            false,
        ));
        let admitted = residual_certified_domain_recovery(
            &surface,
            point,
            1.0e-3,
            attempt,
            &cylinder_lattice(),
        )
        .expect("a certified-periodic out-of-range candidate must be recoverable");
        assert_eq!(
            admitted.0,
            (0.5, std::f64::consts::PI),
            "one certified period is subtracted along the periodic axis"
        );
        assert_eq!(
            admitted.2,
            diagnosis::DomainRecoveryClass::PeriodicEquivalent,
            "the admission is justified by periodic equivalence"
        );
    }

    /// A periodic candidate on an axis the lattice does *not* certify must not
    /// be normalized: an uncertified period is not a generator. At more than a
    /// whole declared span outside, the class is the nontrivial range mismatch,
    /// not a recoverable periodic equivalence.
    #[test]
    fn uncertified_period_is_never_normalized() {
        let surface = unit_cylinder();
        let point = Point3::new(-1.0, 0.0, 0.5);
        let attempt = attempt_with_seed(outcome(
            (0.5, 5.0 * std::f64::consts::PI),
            0.0,
            false,
            false,
        ));
        let non_periodic = CertifiedLattice::NON_PERIODIC;
        assert_eq!(
            residual_certified_domain_recovery(&surface, point, 1.0e-3, attempt, &non_periodic),
            None,
            "without a certified generator the same candidate is not recoverable"
        );
        let class = classify_domain_point(
            attempt.seed_best,
            1.0e-3,
            Some((0.0, 1.0)),
            Some((0.0, 2.0 * std::f64::consts::PI)),
            &non_periodic,
        );
        assert_eq!(
            class,
            diagnosis::DomainRecoveryClass::RepresentationRangeMismatch,
            "a candidate a whole span outside with no period is a range mismatch"
        );
    }

    /// A candidate microscopically outside a closed boundary is classified as
    /// a boundary epsilon, and clamping it to the boundary still re-evaluates
    /// within tolerance.
    #[test]
    fn boundary_epsilon_is_classified_and_clamped() {
        let surface = bilinear_patch();
        // The point is inside the patch; the candidate sits a tiny epsilon
        // above the u=1 boundary.
        let point = Point3::new(1.0, 0.5, 0.0);
        let eps = 1.0e-9;
        let attempt = attempt_with_seed(outcome((1.0 + eps, 0.5), 0.0, false, false));
        let class = classify_domain_point(
            attempt.seed_best,
            1.0e-3,
            Some((0.0, 1.0)),
            Some((0.0, 1.0)),
            &CertifiedLattice::NON_PERIODIC,
        );
        assert_eq!(class, diagnosis::DomainRecoveryClass::BoundaryEpsilon);
        let admitted = residual_certified_domain_recovery(
            &surface,
            point,
            1.0e-3,
            attempt,
            &CertifiedLattice::NON_PERIODIC,
        )
        .expect("a microscopically-outside candidate clamps and re-certifies");
        assert_eq!(
            admitted.0,
            (1.0, 0.5),
            "the coordinate is clamped to the boundary"
        );
        assert_eq!(admitted.2, diagnosis::DomainRecoveryClass::BoundaryEpsilon);
    }

    /// A candidate genuinely far outside a nonperiodic range is a
    /// representation mismatch, and is *not* recovered by Stage C.
    #[test]
    fn representation_range_mismatch_is_not_admitted() {
        let surface = bilinear_patch();
        let point = Point3::new(0.0, 0.0, 0.0);
        let attempt = attempt_with_seed(outcome((5.0, 0.5), 0.0, false, false));
        assert_eq!(
            residual_certified_domain_recovery(
                &surface,
                point,
                1.0e-3,
                attempt,
                &CertifiedLattice::NON_PERIODIC
            ),
            None,
            "a far-outside nonperiodic candidate is diagnostic-only"
        );
    }

    /// A candidate whose residual exceeds tolerance is not a domain question
    /// and is not admitted by Stage C.
    #[test]
    fn domain_recovery_respects_tolerance() {
        let surface = unit_cylinder();
        let point = Point3::new(-1.0, 0.0, 0.5);
        let attempt = attempt_with_seed(outcome(
            (0.5, 3.0 * std::f64::consts::PI),
            5.0,
            false,
            false,
        ));
        assert_eq!(
            residual_certified_domain_recovery(
                &surface,
                point,
                1.0e-3,
                attempt,
                &cylinder_lattice()
            ),
            None,
            "Stage C never widens the caller tolerance"
        );
    }
}

#[cfg(test)]
mod sphere_pole_recovery_tests {
    use super::*;
    use std::f64::consts::FRAC_PI_4;
    use std::f64::consts::FRAC_PI_8;
    use truck_geometry::prelude::Sphere;

    const R: f64 = 10.0;

    fn sphere() -> Sphere {
        Sphere::new(Point3::origin(), R)
    }

    /// The certified lattice of the geometry sphere: azimuth (geometry-`v`) has
    /// period `2π` by construction of the primitive; the polar axis has none.
    /// The P2 continuation is a theorem path, so the tests drive it with the
    /// certified generator rather than an unevidenced accessor value.
    fn sphere_lattice() -> CertifiedLattice {
        CertifiedLattice::sphere_azimuth(Axis::V)
    }

    fn spt(u: f64, v: f64) -> Point3 {
        sphere().subs(u, v)
    }

    fn use_(bound: usize, index: usize, orientation: bool) -> SourceEdgeUse {
        SourceEdgeUse {
            bound: BoundId(bound),
            index,
            orientation,
        }
    }

    fn wire_edge(
        pts: Vec<Point3>,
        bound: usize,
        index: usize,
        orientation: bool,
    ) -> SourcePolyline {
        SourcePolyline {
            curve: PolylineCurve(pts),
            source: use_(bound, index, orientation),
        }
    }

    /// The canonical great-circle triangle through the north pole: vertices at
    /// the pole, (colatitude 45 deg, longitude 0) and (colatitude 45 deg,
    /// longitude 3.3). The two meridians are the pole edges; the third edge is
    /// the great circle between the two non-pole vertices. The pole is the
    /// shared vertex where the walk must cross the undefined longitude.
    fn north_pole_wire() -> Vec<SourcePolyline> {
        let p = spt(0.0, 0.0);
        let a = spt(FRAC_PI_4, 0.0);
        let b = spt(FRAC_PI_4, 3.3);
        let mid0 = spt(FRAC_PI_8, 0.0);
        let mid1 = spt(FRAC_PI_8, 3.3);
        vec![
            // meridian v = 0, A -> P; the pole is the wire's last point and is
            // dropped, so the walk enters the pole from longitude 0.
            wire_edge(vec![a, mid0, p], 0, 0, true),
            // meridian v = 3.3, P -> B; the pole is the polyline's first point.
            wire_edge(vec![p, mid1, b], 0, 1, true),
            // great circle B -> A, expanded to a chord by `try_new`.
            wire_edge(vec![b, a], 0, 2, true),
        ]
    }

    /// The same triangle reflected through the equator: the pole vertex is the
    /// south pole (u = pi).
    fn south_pole_wire() -> Vec<SourcePolyline> {
        let p = spt(std::f64::consts::PI, 0.0);
        let a = spt(std::f64::consts::PI - FRAC_PI_4, 0.0);
        let b = spt(std::f64::consts::PI - FRAC_PI_4, 3.3);
        let mid0 = spt(std::f64::consts::PI - FRAC_PI_8, 0.0);
        let mid1 = spt(std::f64::consts::PI - FRAC_PI_8, 3.3);
        vec![
            wire_edge(vec![a, mid0, p], 0, 0, true),
            wire_edge(vec![p, mid1, b], 0, 1, true),
            wire_edge(vec![b, a], 0, 2, true),
        ]
    }

    /// The same triangle, walked the other way around: the wire starts at the
    /// great-circle edge and crosses the pole at the end.
    fn reversed_pole_wire() -> Vec<SourcePolyline> {
        let p = spt(0.0, 0.0);
        let a = spt(FRAC_PI_4, 0.0);
        let b = spt(FRAC_PI_4, 3.3);
        let mid0 = spt(FRAC_PI_8, 0.0);
        let mid1 = spt(FRAC_PI_8, 3.3);
        vec![
            // great circle A -> B.
            wire_edge(vec![a, b], 0, 0, true),
            // meridian v = 3.3, B -> P.
            wire_edge(vec![b, mid1, p], 0, 1, true),
            // meridian v = 0, P -> A; the pole is the polyline's first point.
            wire_edge(vec![p, mid0, a], 0, 2, true),
        ]
    }

    fn lift(wire: Vec<SourcePolyline>) -> PolyBoundaryPiece {
        let s = sphere();
        let lattice = sphere_lattice();
        PolyBoundaryPiece::try_new(
            &s,
            wire.into_iter(),
            by_search_nearest_parameter,
            1.0e-6,
            &lattice,
        )
        .expect("the sphere-pole triangle must lift through the branch recovery")
    }

    fn assert_lift_is_closed_and_finite(piece: &PolyBoundaryPiece) {
        assert!(piece
            .0
            .iter()
            .all(|p| p.uv.x.is_finite() && p.uv.y.is_finite()));
        assert!(
            piece.0.len() >= 3,
            "the lifted boundary must be a non-degenerate loop",
        );
        // The walk closes back on its start in world space; the UV may return
        // to a different period copy, which the closure classifier resolves.
        assert!(
            piece.0[0].point.distance(piece.0.last().unwrap().point) <= 1.0e-6,
            "the lifted boundary must close in world space",
        );
    }

    /// The periodic axis must never make a half-period-or-larger step between
    /// consecutive *non-pole* samples. The pole's own longitude is bookkeeping
    /// (it is undefined at the chart singularity), so the single edge into it
    /// may legitimately span a wide longitude gap when the two incident
    /// longitudes are themselves far apart; the recovery's job is that the
    /// *continuation* out of the pole is clean and no regular step re-enters
    /// the ambiguity band.
    fn assert_continuous_longitude(piece: &PolyBoundaryPiece) {
        const V_PERIOD: f64 = 2.0 * std::f64::consts::PI;
        let is_pole =
            |p: &SurfacePoint| p.uv.x.so_small() || (p.uv.x - std::f64::consts::PI).abs() < 1.0e-6;
        for w in piece.0.windows(2) {
            if is_pole(&w[0]) || is_pole(&w[1]) {
                continue;
            }
            let step = f64::abs(w[1].uv.y - w[0].uv.y);
            assert!(
                step < AMBIGUOUS_STEP_FRACTION * V_PERIOD,
                "longitude step {} at [{:?}] -> [{:?}] must stay under the ambiguity band",
                step,
                w[0].uv,
                w[1].uv,
            );
        }
    }

    #[test]
    fn north_pole_great_circle_triangle_recovers() {
        let piece = lift(north_pole_wire());
        assert_lift_is_closed_and_finite(&piece);
        assert_continuous_longitude(&piece);
        let pole = piece.0.iter().find(|p| p.uv.x.so_small());
        assert!(
            pole.is_some(),
            "the pole vertex must be present in the lift"
        );
    }

    #[test]
    fn south_pole_great_circle_triangle_recovers() {
        let piece = lift(south_pole_wire());
        assert_lift_is_closed_and_finite(&piece);
        assert_continuous_longitude(&piece);
        let pole = piece
            .0
            .iter()
            .find(|p| (p.uv.x - std::f64::consts::PI).abs() < 1.0e-6);
        assert!(pole.is_some(), "the south pole vertex must be present");
    }

    #[test]
    fn reversed_pole_traversal_recovers() {
        let piece = lift(reversed_pole_wire());
        assert_lift_is_closed_and_finite(&piece);
        assert_continuous_longitude(&piece);
    }

    #[test]
    fn ordinary_sphere_triangle_away_from_pole_lifts() {
        // Three vertices all at colatitude 60 deg with moderate longitude
        // spans: no pole is touched, so the lift completes by projection.
        let a = spt(std::f64::consts::FRAC_PI_3, 0.0);
        let b = spt(std::f64::consts::FRAC_PI_3, 1.0);
        let c = spt(std::f64::consts::FRAC_PI_3, 2.0);
        let wire = vec![
            wire_edge(vec![a, b], 0, 0, true),
            wire_edge(vec![b, c], 0, 1, true),
            wire_edge(vec![c, a], 0, 2, true),
        ];
        let piece = lift(wire);
        assert_lift_is_closed_and_finite(&piece);
    }

    #[test]
    fn non_singular_step_is_not_recovered() {
        // A step whose start sample is not a chart singularity (the sphere at
        // colatitude 45 deg has full rank) must be refused by the branch
        // analysis: this is the near-pole / regular crossing gate.
        let s = sphere();
        let previous = (FRAC_PI_4, 0.0);
        let origin = (FRAC_PI_4, 3.3, spt(FRAC_PI_4, 3.3));
        let outcome = singular_transition_branch(
            &s,
            &by_search_nearest_parameter,
            None,
            Some(2.0 * std::f64::consts::PI),
            previous,
            spt(FRAC_PI_4, 0.0),
            origin,
            &[],
            &[],
        );
        assert!(
            matches!(outcome, SingularTransitionOutcome::NotApplicable),
            "a non-singular step must not be recovered",
        );
    }

    #[test]
    fn underdetermined_leaving_edge_returns_unresolved_not_rejected() {
        // The step departs from the pole, but the leaving edge's own first
        // sample is also the pole: its longitude is undefined, so this
        // mechanism cannot determine a continuation. That is negative evidence
        // about the mechanism only -- it does not prove the STEP source admits
        // two distinct continuations, so the outcome must be UNRESOLVED
        // (InsufficientEvidence), never a `RejectedAmbiguous` certificate.
        let s = sphere();
        let pole = spt(0.0, 0.0);
        let previous = (0.0, 0.0);
        let origin = (0.0, 3.3, pole);
        let outcome = singular_transition_branch(
            &s,
            &by_search_nearest_parameter,
            None,
            Some(2.0 * std::f64::consts::PI),
            previous,
            pole,
            origin,
            &[],
            &[],
        );
        assert!(
            matches!(outcome, SingularTransitionOutcome::InsufficientEvidence),
            "an underdetermined pole continuation is unresolved, never rejected"
        );
    }

    #[test]
    #[ignore]
    fn probe_equator_walk_current_behavior() {
        // Diagnostic (P3b): confirm the periodic-cap mechanism builds a
        // contractible cell for a single periodic latitude walk and that it
        // tessellates. Kept ignored; the graduating tests live in
        // `periodic_cap_closure_tests`.
        let s = sphere();
        let lattice = sphere_lattice();
        let pts: Vec<SurfacePoint> = (0..=32)
            .map(|i| {
                let v = (i as f64 / 32.0) * 2.0 * std::f64::consts::PI;
                let uv = Point2::new(std::f64::consts::FRAC_PI_2, v);
                (uv, s.subs(uv.x, uv.y)).into()
            })
            .collect();
        let piece = PolyBoundaryPiece::untagged(pts);
        let tol = 0.05;
        let boundary = PolyBoundary::new(vec![piece], &s, tol, &lattice);
        assert!(boundary.0.len() >= 1);
        let mesh = trimming_tessellation_result(&s, &boundary, tol, &lattice)
            .expect("the cap cell tessellates");
        assert!(!mesh.tri_faces().is_empty());
    }
}

/// P3b graduation: generic periodic spherical-cap chart closure.
///
/// A closed latitude-parallel loop with |k|=1 whose UV image is a 1D line must
/// become a contractible planar cell whose interior is the intended spherical
/// cap. The material side is derived from the source-loop orientation times the
/// effective surface normal; north/south and small/large are decisions, never
/// constants.
mod periodic_cap_closure_tests {
    use super::*;
    use std::f64::consts::{FRAC_PI_3, FRAC_PI_4, FRAC_PI_6, PI, TAU};

    const R: f64 = 10.0;

    fn sphere() -> truck_geometry::prelude::Sphere {
        truck_geometry::prelude::Sphere::new(Point3::origin(), R)
    }

    /// The certified lattice of the geometry sphere: azimuth (geometry-`v`) has
    /// period `2π` by construction of the primitive. The cap theorem's H1 is a
    /// genuine period, so the tests drive the cap with the certified generator.
    fn sphere_lattice() -> CertifiedLattice {
        CertifiedLattice::sphere_azimuth(Axis::V)
    }

    /// The certified lattice of an inverted sphere processor: the azimuth moves
    /// to the caller's `u` axis.
    fn sphere_lattice_inverted() -> CertifiedLattice {
        CertifiedLattice::sphere_azimuth(Axis::U)
    }

    fn use_(bound: usize, index: usize, orientation: bool) -> SourceEdgeUse {
        SourceEdgeUse {
            bound: BoundId(bound),
            index,
            orientation,
        }
    }

    /// A latitude-parallel boundary piece: constant colatitude `u0`, one full
    /// turn of longitude `v`, traversed in the given direction (`+1` =
    /// increasing longitude, `-1` = decreasing). Optional source tagging.
    fn latitude_piece<S: PreMeshableSurface>(
        surface: &S,
        u0: f64,
        dir: f64,
        tagged: bool,
    ) -> PolyBoundaryPiece {
        let n = 32;
        let pts: Vec<SurfacePoint> = (0..=n)
            .map(|i| {
                let v = (i as f64 / n as f64) * TAU;
                let v = if dir > 0.0 { v } else { TAU - v };
                let uv = Point2::new(u0, v);
                (uv, surface.subs(uv.x, uv.y)).into()
            })
            .collect();
        let sources = if tagged {
            (0..=n).map(|i| vec![use_(0, i, true)]).collect()
        } else {
            vec![Vec::new(); n + 1]
        };
        PolyBoundaryPiece(pts, sources)
    }

    fn cap_z_extent(mesh: &PolygonMesh) -> (f64, f64) {
        use truck_geometry::prelude::*;
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for p in mesh.positions() {
            lo = lo.min(p.z);
            hi = hi.max(p.z);
        }
        (lo, hi)
    }

    fn assert_cap_mesh(
        surface: &impl PreMeshableSurface,
        piece: PolyBoundaryPiece,
        lattice: CertifiedLattice,
        tol: f64,
        expect_floor: f64,
        expect_reach: f64,
    ) {
        let boundary = PolyBoundary::new(vec![piece], surface, tol, &lattice);
        assert_eq!(
            boundary.0.len(),
            1,
            "a periodic cap must close into exactly one contractible cell",
        );
        let mesh = trimming_tessellation_result(surface, &boundary, tol, &lattice)
            .expect("the cap cell must tessellate");
        assert!(
            !mesh.tri_faces().is_empty(),
            "the cap must produce triangles"
        );
        let (z_lo, z_hi) = cap_z_extent(&mesh);
        assert!(
            z_lo > expect_floor,
            "material must not extend below z={expect_floor}, got z_lo={z_lo}",
        );
        assert!(
            z_hi > expect_reach,
            "material must reach above z={expect_reach}, got z_hi={z_hi}",
        );
    }

    /// North spherical cap: a latitude walk at colatitude 60 deg traversed
    /// counterclockwise (increasing longitude). The material (left of travel
    /// viewed from the outward normal) is the north cap, z in [5, 10].
    #[test]
    fn north_spherical_cap_renders() {
        let s = sphere();
        let piece = latitude_piece(&s, FRAC_PI_3, 1.0, false);
        assert_cap_mesh(&s, piece, sphere_lattice(), 0.05, 4.0, 9.0);
    }

    /// South spherical cap: the same latitude circle traversed clockwise. The
    /// material is the region toward the south pole, z in [-10, -5].
    #[test]
    fn south_spherical_cap_renders() {
        let s = sphere();
        let piece = latitude_piece(&s, 2.0 * FRAC_PI_3, -1.0, false);
        assert_cap_mesh(&s, piece, sphere_lattice(), 0.05, -10.5, -6.0);
    }

    /// Hemisphere / equator boundary: the equator is a limiting cap whose
    /// material is a full hemisphere (radius 10, so the pole reaches z ~ 10).
    #[test]
    fn hemisphere_equator_boundary_renders() {
        let s = sphere();
        let piece = latitude_piece(&s, PI / 2.0, 1.0, false);
        assert_cap_mesh(&s, piece, sphere_lattice(), 0.05, -0.5, 9.0);
    }

    /// Reversed traversal: the north cap walked clockwise selects the opposite
    /// pole, so the mechanism must not be bound to any fixed side. A clockwise
    /// walk at colatitude 30 deg has material z in [-10, 8.66].
    #[test]
    fn reversed_traversal_selects_opposite_side() {
        let s = sphere();
        let piece = latitude_piece(&s, FRAC_PI_6, -1.0, false);
        assert_cap_mesh(&s, piece, sphere_lattice(), 0.05, -10.5, 8.0);
    }

    /// Reversed face/surface orientation: the same counterclockwise walk on a
    /// surface whose normal is inverted (a `Processor` with swapped axes and
    /// flipped normal, as `FACE_SURFACE.same_sense = .F.` produces) must select
    /// the opposite cap. This is the stepio `Surface::invert()` convention.
    #[test]
    fn inverted_surface_orientation_selects_opposite_side() {
        use truck_geometry::prelude::Processor;
        let mut processed = Processor::<truck_geometry::prelude::Sphere, Matrix4>::new(sphere());
        processed.invert();
        // Caller axes after inversion: caller u = longitude (periodic), caller
        // v = colatitude. A counterclockwise latitude walk at colatitude 60 deg
        // on the inverted surface has material toward the south pole, z in
        // [-10, 5].
        let n = 32;
        let pts: Vec<SurfacePoint> = (0..=n)
            .map(|i| {
                let u = (i as f64 / n as f64) * TAU;
                let uv = Point2::new(u, FRAC_PI_3);
                (uv, processed.subs(uv.x, uv.y)).into()
            })
            .collect();
        let piece = PolyBoundaryPiece::untagged(pts);
        assert_cap_mesh(
            &processed,
            piece,
            sphere_lattice_inverted(),
            0.05,
            -10.5,
            4.0,
        );
    }

    /// Double orientation flip: an inverted surface *and* a reversed traversal.
    ///
    /// The material side is `n x t`; flipping `n` (surface sense) and flipping
    /// `t` (walk direction) together restore the original product, so the cap
    /// must come back to the same hemisphere the upright CCW walk selects. This
    /// is the composition-law obligation of §7: a pipeline that applied either
    /// orientation twice (or folded `FACE_BOUND` into `ORIENTED_EDGE` twice)
    /// would render a perfectly good mesh of the wrong hemisphere, which render
    /// count alone cannot detect — only the hemisphere assertion can.
    #[test]
    fn double_orientation_flip_restores_the_base_cap() {
        use truck_geometry::prelude::Processor;
        let mut processed = Processor::<truck_geometry::prelude::Sphere, Matrix4>::new(sphere());
        processed.invert();
        // Reversed traversal (clockwise in caller longitude) at colatitude 60
        // deg on the inverted surface: material toward the north pole again.
        let n = 32;
        let pts: Vec<SurfacePoint> = (0..=n)
            .map(|i| {
                let u = TAU - (i as f64 / n as f64) * TAU;
                let uv = Point2::new(u, FRAC_PI_3);
                (uv, processed.subs(uv.x, uv.y)).into()
            })
            .collect();
        let piece = PolyBoundaryPiece::untagged(pts);
        assert_cap_mesh(&processed, piece, sphere_lattice_inverted(), 0.05, 4.0, 9.0);
    }

    /// Double orientation flip at the equator: the limiting hemisphere case.
    /// An inverted surface walked clockwise at the equator must still reach the
    /// north pole (z ~ 10), matching the upright CCW hemisphere, because both
    /// `n` and `t` have flipped.
    #[test]
    fn double_orientation_flip_at_equator_restores_the_north_hemisphere() {
        use truck_geometry::prelude::Processor;
        let mut processed = Processor::<truck_geometry::prelude::Sphere, Matrix4>::new(sphere());
        processed.invert();
        let n = 32;
        let pts: Vec<SurfacePoint> = (0..=n)
            .map(|i| {
                let u = TAU - (i as f64 / n as f64) * TAU;
                let uv = Point2::new(u, PI / 2.0);
                (uv, processed.subs(uv.x, uv.y)).into()
            })
            .collect();
        let piece = PolyBoundaryPiece::untagged(pts);
        assert_cap_mesh(
            &processed,
            piece,
            sphere_lattice_inverted(),
            0.05,
            -0.5,
            9.0,
        );
    }

    /// Face-bound orientation: `FACE_BOUND.orientation = .F.` reverses the wire
    /// before the tessellator sees it, so a clockwise walk on an upright sphere
    /// must select the south cap — the same physical composition as a reversed
    /// traversal (the walk carries the bound's reversal).
    ///
    /// This is deliberately the same assertion as
    /// [`south_spherical_cap_renders`] at a *different* colatitude, so the two
    /// mechanisms (edge-vs-bound orientation) that produce the same UV walk are
    /// shown to compose to the same hemisphere rather than one cancelling the
    /// other.
    #[test]
    fn face_bound_reversal_selects_the_opposite_cap() {
        let s = sphere();
        // Clockwise walk at colatitude 60 deg, as `FACE_BOUND .F.` would present
        // it after reversing the upright CCW wire.
        let piece = latitude_piece(&s, FRAC_PI_3, -1.0, false);
        assert_cap_mesh(&s, piece, sphere_lattice(), 0.05, -10.5, 4.0);
    }

    /// H4 evidence: a sphere cap's pole is located by the *certified* sphere
    /// pole, not by the numerical orbit-diameter scan. The two must agree on
    /// the pole latitude, but only the certified path establishes the collapse
    /// as a source-level fact.
    #[test]
    fn sphere_cap_pole_is_located_by_the_certified_collapse() {
        let s = sphere();
        let lattice = sphere_lattice();
        // The raw periodic source walk the gate consumes: one full turn of
        // longitude at colatitude 60 deg, wrapping back onto itself.
        let piece = latitude_piece(&s, FRAC_PI_3, 1.0, false);
        let sources = piece.1;
        let closed = BoundaryLoop::periodic_source_walk(piece.0, sources);
        let displacement = [0, 1];
        let built = PeriodicCapClosure::try_build(&s, &closed, displacement, 0.05, &lattice)
            .expect("a north sphere cap activates");
        // The cap's pole line is at the north pole (colatitude 0): the
        // chart-closure segments include the meridian runs (which walk from the
        // loop latitude down to the pole and back) *and* the degenerate pole
        // line, whose two endpoints both sit at the certified collapse latitude.
        let pole_line_exists = built
            .origins
            .iter()
            .enumerate()
            .filter(|(_, o)| **o == SegmentOrigin::ChartClosure)
            .any(|(i, _)| {
                let p = built.points[i];
                let q = built.points[(i + 1) % built.points.len()];
                p.uv.x.abs() < 1.0e-3 && q.uv.x.abs() < 1.0e-3
            });
        assert!(
            pole_line_exists,
            "the cap cell must contain a pole line at the certified north pole",
        );
        // And the meridian runs must terminate at that same certified latitude:
        // every chart-closure vertex at colatitude 0 is the collapsed pole.
        let reaches_pole = built.points.iter().any(|p| p.uv.x.abs() < 1.0e-3);
        assert!(reaches_pole, "the cap must reach the certified north pole");
    }

    /// The generic non-collapsing loop gets no pole: `find_cap_pole` declines
    /// with `None` rather than inventing a collapse. This keeps the candidate
    /// recognizer from silently becoming a certificate.
    #[test]
    fn a_non_collapsing_loop_gets_no_pole_evidence() {
        use truck_modeling::{Line, RevolutedCurve, Vector3};
        let cylinder = RevolutedCurve::by_revolution(
            Line(Point3::new(10.0, 0.0, 0.0), Point3::new(10.0, 0.0, 10.0)),
            Point3::origin(),
            Vector3::unit_z(),
        );
        let n = 32;
        let pts: Vec<SurfacePoint> = (0..=n)
            .map(|i| {
                let v = (i as f64 / n as f64) * TAU;
                let uv = Point2::new(0.5, v);
                (uv, cylinder.subs(uv.x, uv.y)).into()
            })
            .collect();
        let piece = PolyBoundaryPiece::untagged(pts);
        let lattice = unevidenced_lattice(&cylinder);
        let boundary = PolyBoundary::new(vec![piece], &cylinder, 0.05, &lattice);
        let closed = &boundary.0[0];
        let pole = find_cap_pole(&cylinder, closed, PeriodicAxis::V, 0.5, 2.0 * PI, &lattice);
        assert!(
            pole.is_none(),
            "a cylinder latitude loop must not nominate a pole: got {pole:?}",
        );
    }

    /// |k|=0 ordinary spherical loop is untouched: a small non-degenerate
    /// spherical triangle still tessellates through the normal arrangement, and
    /// the cap mechanism does not fire on it.
    #[test]
    fn ordinary_spherical_loop_is_unaffected() {
        let s = sphere();
        let lattice = sphere_lattice();
        // Three vertices spanning a genuine 2D chart region (varying both
        // colatitude and longitude), closed back on the start.
        let a = s.subs(FRAC_PI_3, 0.0);
        let b = s.subs(FRAC_PI_3, 1.0);
        let c = s.subs(FRAC_PI_4, 1.5);
        let pts = vec![
            (Point2::new(FRAC_PI_3, 0.0), a).into(),
            (Point2::new(FRAC_PI_3, 1.0), b).into(),
            (Point2::new(FRAC_PI_4, 1.5), c).into(),
            (Point2::new(FRAC_PI_3, 0.0), a).into(),
        ];
        let piece = PolyBoundaryPiece::untagged(pts);
        let boundary = PolyBoundary::new(vec![piece], &s, 0.05, &lattice);
        assert_eq!(boundary.0.len(), 1);
        assert!(
            boundary.0[0]
                .origins
                .iter()
                .all(|o| *o == SegmentOrigin::Source),
            "an ordinary closed loop must keep every segment as Source",
        );
        let mesh = trimming_tessellation_result(&s, &boundary, 0.05, &lattice)
            .expect("the ordinary triangle tessellates");
        assert!(!mesh.tri_faces().is_empty());
    }

    /// no orbit collapse, so the mechanism declines and the face keeps whatever
    /// the legacy path produced (no cap is invented).
    #[test]
    fn non_collapsing_cylinder_loop_is_not_a_cap() {
        use truck_modeling::{Line, RevolutedCurve, Vector3};
        let cylinder = RevolutedCurve::by_revolution(
            Line(Point3::new(10.0, 0.0, 0.0), Point3::new(10.0, 0.0, 10.0)),
            Point3::origin(),
            Vector3::unit_z(),
        );
        let n = 32;
        let pts: Vec<SurfacePoint> = (0..=n)
            .map(|i| {
                let v = (i as f64 / n as f64) * TAU;
                let uv = Point2::new(0.5, v);
                (uv, cylinder.subs(uv.x, uv.y)).into()
            })
            .collect();
        let piece = PolyBoundaryPiece::untagged(pts);
        let lattice = unevidenced_lattice(&cylinder);
        let boundary = PolyBoundary::new(vec![piece], &cylinder, 0.05, &lattice);
        // The cell, if built, would be a cap; a cylinder must decline. The
        // legacy path yields a degenerate loop that either stays a loop or is
        // closed by the existing machinery -- in no case a ChartClosure cell.
        assert!(
            !boundary
                .0
                .iter()
                .flat_map(|l| l.origins.iter())
                .any(|o| *o == SegmentOrigin::ChartClosure),
            "a cylinder latitude loop must not be turned into a cap",
        );
    }

    /// The chart closure carries no source identity and no physical parity
    /// role: every synthetic segment is labelled `ChartClosure` with an empty
    /// contributor set, and every real segment keeps exactly its source use.
    #[test]
    fn chart_closure_carries_no_source_identity_or_physical_role() {
        let s = sphere();
        let lattice = sphere_lattice();
        let piece = latitude_piece(&s, FRAC_PI_3, 1.0, true);
        let boundary = PolyBoundary::new(vec![piece], &s, 0.05, &lattice);
        let cell = &boundary.0[0];
        assert!(!cell.origins.is_empty());
        // Real source segments keep their provenance and physical role.
        let source_count = cell
            .origins
            .iter()
            .filter(|o| **o == SegmentOrigin::Source)
            .count();
        assert!(
            source_count > 0,
            "the source latitude walk must be retained"
        );
        // Synthetic closure segments are artificial and carry no source.
        let closure_count = cell
            .origins
            .iter()
            .filter(|o| **o == SegmentOrigin::ChartClosure)
            .count();
        assert!(closure_count > 0, "the chart closure must be present");
        for (i, origin) in cell.origins.iter().enumerate() {
            let sources = &cell.source_uses[i];
            match origin {
                SegmentOrigin::Source => {
                    assert_eq!(sources.len(), 1, "a source segment keeps its use");
                    assert!(
                        sources[0] == use_(0, i, true),
                        "the retained use is the segment's own"
                    );
                }
                SegmentOrigin::ChartClosure => {
                    assert!(
                        sources.is_empty(),
                        "a chart closure must never forge a source identity",
                    );
                }
                SegmentOrigin::Seam | SegmentOrigin::SyntheticClosure => {}
            }
        }
        // Every synthetic closure segment maps to a synthetic (non-physical)
        // constraint role; none is a physical trim.
        for origin in &cell.origins {
            match origin {
                SegmentOrigin::ChartClosure => assert_ne!(
                    origin.role(),
                    ConstraintRole::PhysicalBoundary,
                    "chart closure must not be a physical boundary",
                ),
                _ => {}
            }
        }
        let mesh = trimming_tessellation_result(&s, &boundary, 0.05, &lattice)
            .expect("the tagged cap cell tessellates");
        assert!(!mesh.tri_faces().is_empty());
    }
}

/// Constraint wiring of [`wire_grid_constraints`]: the interior sampling grid
/// must constrain every *material* sub-segment of every grid line, cutting each
/// grid segment at its real intersections with the trim so a final triangle
/// cannot cross a subdivision-cell boundary merely because a grid vertex was
/// outside the trimmed region.
#[cfg(test)]
mod grid_constraint_wiring_tests {
    use super::*;

    fn point(x: f64, y: f64) -> SPoint2 {
        SPoint2::new(x, y)
    }

    fn insert_vertex(cdt: &mut Cdt, p: SPoint2) -> FixedVertexHandle {
        cdt.insert(p).expect("vertex insertion succeeds")
    }

    fn find_vertex(cdt: &Cdt, p: SPoint2) -> FixedVertexHandle {
        cdt.vertices()
            .find(|v| v.as_ref().distance_2(p) < 1e-9)
            .expect("vertex exists")
            .fix()
    }

    /// A square trim `[lo, hi] × [lo, hi]` in the plane z = 0.
    fn square_trim(lo: f64, hi: f64) -> PolyBoundary {
        let pts: Vec<SurfacePoint> = [
            (Point2::new(lo, lo), Point3::new(lo, lo, 0.0)),
            (Point2::new(hi, lo), Point3::new(hi, lo, 0.0)),
            (Point2::new(hi, hi), Point3::new(hi, hi, 0.0)),
            (Point2::new(lo, hi), Point3::new(lo, hi, 0.0)),
        ]
        .into_iter()
        .map(Into::into)
        .collect();
        PolyBoundary(vec![BoundaryLoop {
            points: pts,
            origins: vec![SegmentOrigin::Source; 4],
            source_uses: vec![Vec::new(); 4],
        }])
    }

    /// A grid with every vertex present (`Some`), inserted into `cdt`.
    fn present_grid_into(
        cdt: &mut Cdt,
        udiv: &[f64],
        vdiv: &[f64],
    ) -> Vec<Vec<Option<FixedVertexHandle>>> {
        udiv.iter()
            .map(|&u| {
                vdiv.iter()
                    .map(|&v| Some(insert_vertex(cdt, point(u, v))))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    }

    /// A grid where only the grid points `inside` earn a vertex, inserted into
    /// `cdt`.
    fn classified_grid_into(
        cdt: &mut Cdt,
        udiv: &[f64],
        vdiv: &[f64],
        inside: &dyn Fn(f64, f64) -> bool,
    ) -> Vec<Vec<Option<FixedVertexHandle>>> {
        udiv.iter()
            .map(|&u| {
                vdiv.iter()
                    .map(|&v| {
                        if inside(u, v) {
                            Some(insert_vertex(cdt, point(u, v)))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    }

    /// Insert the trim as constraints (as the production `insert_to` does),
    /// insert the grid vertices, and wire.
    fn wire_with_boundary(
        boundary: &PolyBoundary,
        udiv: &[f64],
        vdiv: &[f64],
        grid: &[Vec<Option<FixedVertexHandle>>],
        cdt: &mut Cdt,
        roles: &mut ConstraintRoles,
    ) {
        wire_grid_constraints(cdt, roles, boundary, udiv, vdiv, grid);
    }

    /// The constraint edge between two vertices, if both exist and share one.
    fn edge_between(cdt: &Cdt, a: SPoint2, b: SPoint2) -> Option<FixedUndirectedEdgeHandle> {
        let va = cdt
            .vertices()
            .find(|v| v.as_ref().distance_2(a) < 1e-9)?
            .fix();
        let vb = cdt
            .vertices()
            .find(|v| v.as_ref().distance_2(b) < 1e-9)?
            .fix();
        cdt.get_edge_from_neighbors(va, vb)
            .map(|e| e.as_undirected().fix())
    }

    fn is_constraint_edge(cdt: &Cdt, a: SPoint2, b: SPoint2) -> bool {
        edge_between(cdt, a, b)
            .filter(|e| cdt.is_constraint_edge(*e))
            .is_some()
    }

    /// A fully-interior grid (trim larger than the grid, every grid point
    /// material): every grid line — including the final u-column — must be
    /// constrained between its consecutive vertices. This is the wiring that
    /// guarantees a triangle cannot straddle a certified cell in the interior.
    #[test]
    fn fully_interior_grid_is_fully_constrained_including_final_column() {
        let boundary = square_trim(-2.0, 2.0);
        let mut cdt = Cdt::new();
        let mut boundary_map = HashMap::<FixedVertexHandle, Point3>::default();
        let mut vertex_sources = HashMap::<FixedVertexHandle, Vec<SourceEdgeUse>>::default();
        let mut roles = ConstraintRoles::default();
        boundary
            .insert_to(&mut cdt, &mut boundary_map, &mut roles, &mut vertex_sources)
            .expect("insert_to succeeds");
        let grid = present_grid_into(&mut cdt, &[-1.0, 0.0, 1.0], &[-1.0, 0.0, 1.0]);
        wire_with_boundary(
            &boundary,
            &[-1.0, 0.0, 1.0],
            &[-1.0, 0.0, 1.0],
            &grid,
            &mut cdt,
            &mut roles,
        );

        for v in [-1.0, 0.0, 1.0] {
            assert!(
                is_constraint_edge(&cdt, point(-1.0, v), point(0.0, v)),
                "u-link (-1,{v})-(0,{v}) missing"
            );
            assert!(
                is_constraint_edge(&cdt, point(0.0, v), point(1.0, v)),
                "u-link (0,{v})-(1,{v}) missing"
            );
        }
        for u in [-1.0, 0.0, 1.0] {
            assert!(
                is_constraint_edge(&cdt, point(u, -1.0), point(u, 0.0)),
                "v-link ({u},-1)-({u},0) missing"
            );
            assert!(
                is_constraint_edge(&cdt, point(u, 0.0), point(u, 1.0)),
                "v-link ({u},0)-({u},1) missing"
            );
        }
    }

    /// A grid segment crossing the trim boundary: its material portion must be
    /// constrained up to a *real* grid/trim intersection vertex, and the
    /// outside portion must not be bridged by a constraint.
    #[test]
    fn boundary_crossing_segment_is_constrained_to_a_real_trim_intersection() {
        let boundary = square_trim(-0.4, 0.4);
        let mut cdt = Cdt::new();
        let mut boundary_map = HashMap::<FixedVertexHandle, Point3>::default();
        let mut vertex_sources = HashMap::<FixedVertexHandle, Vec<SourceEdgeUse>>::default();
        let mut roles = ConstraintRoles::default();
        boundary
            .insert_to(&mut cdt, &mut boundary_map, &mut roles, &mut vertex_sources)
            .expect("insert_to succeeds");
        // Grid over [-1, 1]²; only the center is material.
        let inside = |u: f64, v: f64| u.abs() < 0.49 && v.abs() < 0.49;
        let grid = classified_grid_into(&mut cdt, &[-1.0, 0.0, 1.0], &[-1.0, 0.0, 1.0], &inside);
        wire_with_boundary(
            &boundary,
            &[-1.0, 0.0, 1.0],
            &[-1.0, 0.0, 1.0],
            &grid,
            &mut cdt,
            &mut roles,
        );

        // The center connects to the four real trim intersections on the axes.
        let center = point(0.0, 0.0);
        for (ix, iy) in [(0.4, 0.0), (-0.4, 0.0), (0.0, 0.4), (0.0, -0.4)] {
            let target = point(ix, iy);
            // The intersection vertex must exist, exactly at the grid∩trim point.
            assert!(
                find_vertex(&cdt, target) != find_vertex(&cdt, center),
                "intersection vertex ({ix},{iy}) must be a distinct vertex"
            );
            assert!(
                is_constraint_edge(&cdt, center, target),
                "material spoke (0,0)-({ix},{iy}) missing"
            );
        }
        // The outside portion is not bridged: no constraint from the trim
        // intersection toward the outer grid point, and no vertex at (1,0).
        assert!(
            !is_constraint_edge(&cdt, point(0.0, 0.0), point(1.0, 0.0)),
            "the outside portion must not be constrained"
        );
        assert!(
            cdt.vertices()
                .find(|v| v.as_ref() == &point(1.0, 0.0))
                .is_none(),
            "no vertex may be placed on the outside portion of the grid line"
        );
    }
}

/// Structural tests for CDT refinement.
///
/// These assert the *invariant* — a refinement pass is retained only when it
/// strictly reduces the face's maximum sampled exact-surface deviation, and
/// refinement terminates when that deviation satisfies tolerance — rather than
/// any particular face population. They drive
/// [`trimming_tessellation_with_refinement`] directly with an explicit
/// `enable_refine` flag so the acceptance rule is exercised deterministically.
#[cfg(test)]
mod cdt_refinement_tests {
    use super::*;
    use std::f64::consts::{FRAC_PI_2, PI};
    use truck_modeling::{Line, Point2, Point3, RevolutedCurve, Vector3};

    fn cylinder() -> RevolutedCurve<Line<Point3>> {
        RevolutedCurve::by_revolution(
            Line(Point3::new(10.0, 0.0, 0.0), Point3::new(10.0, 0.0, 10.0)),
            Point3::origin(),
            Vector3::unit_z(),
        )
    }

    fn sphere() -> truck_geometry::prelude::Sphere {
        truck_geometry::prelude::Sphere::new(Point3::origin(), 10.0)
    }

    fn plane() -> truck_geometry::prelude::Plane {
        truck_geometry::prelude::Plane::new(
            Point3::origin(),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        )
    }

    /// A quad trim on a surface: four edges sampled as polylines, forming one
    /// closed loop. `(u0, u1)` spans the straight axis, `(v0, v1)` the curved
    /// axis; `arc_pts` samples the curved edges, `gen_pts` the straight
    /// generator edges. With coarse `arc_pts` (2 = just the corners) the CDT
    /// bridges the full curved span — the canonical interior-starved
    /// mechanism; with fine `arc_pts` the boundary chords already keep every
    /// triangle within tolerance.
    ///
    /// The boundary is constructed directly as a raw closed [`BoundaryLoop`]
    /// (origins all `Source`, empty provenance) rather than through
    /// [`PolyBoundary::new`], so the test controls the exact polyline and the
    /// periodic/join machinery does not re-lift or densify it.
    fn quad_boundary<S: PreMeshableSurface>(
        surface: &S,
        u0: f64,
        u1: f64,
        v0: f64,
        v1: f64,
        arc_pts: usize,
        gen_pts: usize,
    ) -> PolyBoundary {
        let mk = |u: f64, v: f64| -> SurfacePoint {
            let uv = Point2::new(u, v);
            (uv, surface.subs(u, v)).into()
        };
        let n = arc_pts;
        let m = gen_pts;
        let mut pts = Vec::new();
        // bottom generator: u0 -> u1 at v0 (straight edge)
        for i in 0..m {
            pts.push(mk(u0 + (u1 - u0) * (i as f64 / (m - 1) as f64), v0));
        }
        // right arc: v0 -> v1 at u1 (curved edge)
        for i in 1..n {
            pts.push(mk(u1, v0 + (v1 - v0) * (i as f64 / (n - 1) as f64)));
        }
        // top generator: u1 -> u0 at v1 (straight edge)
        for i in (0..m - 1).rev() {
            pts.push(mk(u0 + (u1 - u0) * (i as f64 / (m - 1) as f64), v1));
        }
        // left arc: v1 -> v0 at u0 (curved edge)
        for i in (1..n).rev() {
            pts.push(mk(u0, v0 + (v1 - v0) * (i as f64 / (n - 1) as f64)));
        }
        let k = pts.len();
        let origins = vec![SegmentOrigin::Source; k];
        let source_uses = vec![Vec::new(); k];
        PolyBoundary(vec![BoundaryLoop::new(pts, origins, source_uses)])
    }

    /// The maximum sampled exact-surface deviation of a mesh: corners taken as
    /// the true surface points `S(uv)`, so boundary realization is invisible.
    fn mesh_max_dev<S: ParametricSurface3D>(surface: &S, mesh: &PolygonMesh) -> f64 {
        let mut max = 0.0f64;
        for tri in mesh.faces().tri_faces() {
            let uv = [
                mesh.uv_coords()[tri[0].pos],
                mesh.uv_coords()[tri[1].pos],
                mesh.uv_coords()[tri[2].pos],
            ];
            let a = Point2::new(uv[0].x, uv[0].y);
            let b = Point2::new(uv[1].x, uv[1].y);
            let c = Point2::new(uv[2].x, uv[2].y);
            let (d, _) = triangle_sampled_deviation_exact(surface, a, b, c);
            if d > max {
                max = d;
            }
        }
        max
    }

    fn tess_mesh<S: PreMeshableSurface + ParametricSurface3D>(
        surface: &S,
        boundary: &PolyBoundary,
        tol: f64,
        refine: bool,
    ) -> PolygonMesh {
        let lattice = unevidenced_lattice(surface);
        match trimming_tessellation_with_refinement(surface, boundary, tol, &lattice, refine) {
            TessellationOutcome::Mesh(ft) => ft.mesh,
            TessellationOutcome::Failed(reason) => panic!("tessellation failed: {reason:?}"),
        }
    }

    /// T1 — unsafe curved-span triangle: a ruled surface whose ordinary CDT can
    /// bridge a tolerance-invalid curved span. Refinement must activate and
    /// reduce the maximum exact-surface deviation.
    #[test]
    fn t1_unsafe_curved_span_triggers_refinement() {
        // A sphere band at mid latitude (v in [0.8, 1.4]), spanning nearly the
        // full u azimuth. The flat CDT triangles across the longitude direction
        // deviate from the sphere by a mid-latitude chord sagitta; with a
        // tolerance below that sagitta the span is tolerance-invalid and the
        // argmax deviation is an interior point (not the pole), so refinement
        // must activate and reduce it.
        let sph = sphere();
        let tol = 0.02;
        let boundary = quad_boundary(&sph, 0.0, PI, 0.8, 1.4, 2, 2);
        let base = tess_mesh(&sph, &boundary, tol, false);
        let refined = tess_mesh(&sph, &boundary, tol, true);
        let base_dev = mesh_max_dev(&sph, &base);
        let refined_dev = mesh_max_dev(&sph, &refined);
        assert!(
            base_dev > tol,
            "baseline must exceed tolerance, got {base_dev:.4} vs tol {tol}"
        );
        assert!(
            refined_dev <= tol,
            "refinement must bring max deviation within tolerance, got {refined_dev:.4} vs tol {tol}"
        );
        assert!(
            refined_dev < base_dev,
            "refinement must strictly reduce max deviation: {base_dev:.4} -> {refined_dev:.4}"
        );
    }

    /// T2 — safe `inside=0` case: similar independent divisions, but the
    /// boundary/constraint topology already keeps every actual triangle within
    /// tolerance, so refinement must not activate.
    #[test]
    fn t2_safe_quad_within_tolerance_does_not_refine() {
        let cyl = cylinder();
        // A small angular patch (30 degrees), coarse arcs (corners only). The
        // CDT bridges the full 30-degree span, but on R = 10 that chord has
        // sagitta 10(1 - cos 15°) ≈ 0.34 < tol, so every actual triangle is
        // already safe even though the interior grid is empty (`inside = 0`).
        let tol = 0.93;
        let boundary = quad_boundary(&cyl, 0.0, 1.0, 0.0, FRAC_PI_2, 2, 4);
        let base = tess_mesh(&cyl, &boundary, tol, false);
        let refined = tess_mesh(&cyl, &boundary, tol, true);
        let base_dev = mesh_max_dev(&cyl, &base);
        let refined_dev = mesh_max_dev(&cyl, &refined);
        assert!(
            base_dev <= tol,
            "baseline must already be within tolerance, got {base_dev:.4} vs tol {tol}"
        );
        assert!(
            (refined_dev - base_dev).abs() < 1e-6,
            "safe face must be unchanged by refinement: {base_dev:.4} -> {refined_dev:.4}"
        );
    }

    /// T3 — long-axis complexity: greatly increase the straight-axis length
    /// while preserving the same local curved geometry. Refinement cost must
    /// not scale as `straight_length / tiny_orthogonal_interval`.
    #[test]
    fn t3_long_axis_does_not_scale_refinement() {
        let cyl = cylinder();
        let tol = 0.93;
        // Short patch: u in [0, 1].
        let short_b = quad_boundary(&cyl, 0.0, 1.0, 0.0, FRAC_PI_2, 2, 4);
        let short = tess_mesh(&cyl, &short_b, tol, true);
        // Long patch: u in [0, 100] — 100x the straight axis, same 90-degree
        // curved span.
        let long_b = quad_boundary(&cyl, 0.0, 100.0, 0.0, FRAC_PI_2, 2, 4);
        let long = tess_mesh(&cyl, &long_b, tol, true);
        let short_tris = short.tri_faces().len();
        let long_tris = long.tri_faces().len();
        assert!(
            long_tris < short_tris * 3,
            "refinement must not scale with straight-axis length: short {short_tris} tris, long {long_tris} tris"
        );
        assert!(
            mesh_max_dev(&cyl, &long) <= tol,
            "long axis must still satisfy tolerance after refinement"
        );
    }

    /// T4 — planar surface: no refinement, no deviation.
    #[test]
    fn t4_planar_never_refines() {
        let pl = plane();
        let tol = 0.93;
        let boundary = quad_boundary(&pl, 0.0, 10.0, 0.0, 10.0, 2, 4);
        let base = tess_mesh(&pl, &boundary, tol, false);
        let refined = tess_mesh(&pl, &boundary, tol, true);
        assert!(
            mesh_max_dev(&pl, &refined) < 1e-9,
            "planar mesh must have zero exact deviation"
        );
        assert_eq!(
            refined.tri_faces().len(),
            base.tri_faces().len(),
            "planar refinement must not add triangles"
        );
    }

    /// T5 — actual support realization: the chosen support point becomes part
    /// of the material CDT connectivity. The refined mesh must contain a vertex
    /// whose UV is strictly interior (not on the boundary) that was not present
    /// in the baseline mesh.
    #[test]
    fn t5_support_point_becomes_mesh_vertex() {
        // The same sphere band as T1: a tolerance-invalid curved span whose
        // argmax deviation is an interior point, so refinement inserts an
        // interior support that must appear as a material mesh vertex.
        let sph = sphere();
        let tol = 0.02;
        let boundary = quad_boundary(&sph, 0.0, PI, 0.8, 1.4, 2, 2);
        let base = tess_mesh(&sph, &boundary, tol, false);
        let refined = tess_mesh(&sph, &boundary, tol, true);
        // Interior criterion: u strictly inside the azimuth span, v strictly
        // inside the latitude band.
        let base_uvs: std::collections::HashSet<(i64, i64)> = base
            .uv_coords()
            .iter()
            .map(|v| ((v.x * 1e6).round() as i64, (v.y * 1e6).round() as i64))
            .collect();
        let mut found_interior = false;
        for uv in refined.uv_coords() {
            let key = ((uv.x * 1e6).round() as i64, (uv.y * 1e6).round() as i64);
            if !base_uvs.contains(&key)
                && uv.x > 1e-3
                && uv.x < PI - 1e-3
                && uv.y > 0.8 + 1e-3
                && uv.y < 1.4 - 1e-3
            {
                found_interior = true;
                break;
            }
        }
        assert!(
            found_interior,
            "refinement must insert a strictly-interior support vertex into the mesh"
        );
    }

    /// T6 — deterministic result: identical input produces identical
    /// support/refinement decisions and meshes.
    #[test]
    fn t6_refinement_is_deterministic() {
        let cyl = cylinder();
        let tol = 0.93;
        let boundary = quad_boundary(&cyl, 0.0, 1.0, 0.0, FRAC_PI_2, 2, 4);
        let m1 = tess_mesh(&cyl, &boundary, tol, true);
        let m2 = tess_mesh(&cyl, &boundary, tol, true);
        assert_eq!(
            m1.tri_faces().len(),
            m2.tri_faces().len(),
            "deterministic triangle count"
        );
        assert_eq!(
            m1.positions().len(),
            m2.positions().len(),
            "deterministic vertex count"
        );
        let d1 = mesh_max_dev(&cyl, &m1);
        let d2 = mesh_max_dev(&cyl, &m2);
        assert!(
            (d1 - d2).abs() < 1e-12,
            "deterministic max deviation: {d1:.12} vs {d2:.12}"
        );
    }

    /// T7 — defensive termination: a pathological face whose maximum cannot be
    /// reduced by interior support (boundary/pole-pinned) must terminate
    /// without unbounded support insertion or a blown-up triangle count.
    #[test]
    fn t7_pinned_maximum_terminates_without_explosion() {
        // Sphere pole patch: v near 0 is the pole where the sphere's u-axis
        // collapses; the flat triangles there carry large exact deviation that
        // interior support cannot reduce. The acceptance rule must reject every
        // candidate and return the baseline.
        let sph = sphere();
        let tol = 0.93;
        let boundary = quad_boundary(&sph, 0.0, PI, 0.0, 0.5, 8, 4);
        let base = tess_mesh(&sph, &boundary, tol, false);
        let refined = tess_mesh(&sph, &boundary, tol, true);
        let base_tris = base.tri_faces().len();
        let refined_tris = refined.tri_faces().len();
        let base_dev = mesh_max_dev(&sph, &base);
        let refined_dev = mesh_max_dev(&sph, &refined);
        // Either the baseline is already within tolerance (no activation), or
        // the maximum is pinned and every candidate is rejected, returning the
        // baseline. In the pinned case the refined mesh must not explode.
        assert!(
            refined_tris <= base_tris.max(64),
            "pinned maximum must not blow up triangles: base {base_tris}, refined {refined_tris}"
        );
        assert!(
            refined_dev <= base_dev + 1e-9,
            "refinement must never increase max deviation: {base_dev:.6} -> {refined_dev:.6}"
        );
    }
}

// ---------------------------------------------------------------------------
// DIAG-002: constitutive failure-diagnostic contract tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod diag002_contract_tests {
    use super::*;
    use std::path::{Path, PathBuf};

    /// Serialize with the other environment-sensitive diagnostic tests so the
    /// process-global `TRUCK_FACE_DIAG` / `TRUCK_FORMAL_RECOVERY` variables are
    /// not mutated concurrently. Poison-recovering: a panic in one test must
    /// not knock out the rest.
    fn env_guard() -> std::sync::MutexGuard<'static, ()> {
        diagnosis::DIAG_TEST_MUTEX
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn scratch(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "look_diag002_{}_{}.jsonl",
            std::process::id(),
            name
        ))
    }

    fn read_jsonl(path: &Path) -> Vec<serde_json::Value> {
        std::fs::read_to_string(path)
            .unwrap_or_default()
            .lines()
            .filter(|line| !line.is_empty())
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect()
    }

    fn clean_env() {
        std::env::remove_var("TRUCK_FACE_DIAG");
        std::env::remove_var("TRUCK_FACE_DIAG_JSONL");
        std::env::remove_var("TRUCK_FORMAL_RECOVERY");
        diagnosis::set_document_context(None);
        diagnosis::set_test_sink(None);
        diagnosis::reset_file_sink_for_tests();
        diagnosis::clear_emission_capture();
    }

    /// The diagnosis-level terminal cycle exactly as `tessellate_face` runs it:
    /// begin the face, let the pipeline body produce the terminal failure,
    /// finalize once.
    fn run_terminal(
        doc: &str,
        face_id: u64,
        terminal: impl FnOnce() -> TessellationFailure,
    ) -> TessellationFailure {
        diagnosis::set_document_context(Some(doc.to_string()));
        diagnosis::begin_face(
            diagnosis::document_context(),
            Some(face_id),
            None,
            diagnosis::PeriodicAxes { u: false, v: false },
            1,
            4,
            4,
            0,
            0.01,
            true,
            None,
        );
        diagnosis::finalize_and_emit(terminal())
    }

    /// Drive the real lift path until it refuses a boundary whose points sit at
    /// the centre of a sphere, where no nearest parameter exists.
    fn sphere_projection_refusal() -> TessellationFailureReason {
        let sphere = truck_geometry::prelude::Sphere::new(Point3::origin(), 10.0);
        let tol = 0.01;
        let lattice = unevidenced_lattice(&sphere);
        let use_ = SourceEdgeUse {
            bound: BoundId(0),
            index: 0,
            orientation: true,
        };
        let curve = PolylineCurve(vec![Point3::origin(), Point3::origin()]);
        let wire = [SourcePolyline {
            curve,
            source: use_,
        }];
        match PolyBoundaryPiece::try_new(
            &sphere,
            wire.into_iter(),
            by_search_parameter,
            tol,
            &lattice,
        ) {
            Err(reason) => reason,
            Ok(_) => TessellationFailureReason::BoundaryProjectionFailed,
        }
    }

    /// A deterministic terminal failure: a fully-doubled boundary cancels to no
    /// material (`NoOddParityRegion`).
    fn doubled_boundary_failure() -> TessellationFailureReason {
        let plane = plane_surface();
        let tol = 0.01;
        let lattice = unevidenced_lattice(&plane);
        let boundary = PolyBoundary::new(
            vec![PolyBoundaryPiece::untagged(square_loop(2))],
            &plane,
            tol,
            &lattice,
        );
        trimming_tessellation_result(&plane, &boundary, tol, &lattice)
            .err()
            .map(|failure| failure.reason)
            .expect("the doubled boundary must fail")
    }

    fn plane_surface() -> truck_geometry::prelude::Plane {
        truck_geometry::prelude::Plane::new(
            Point3::origin(),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        )
    }

    fn square_loop(visits: u32) -> Vec<SurfacePoint> {
        let corner =
            |x: f64, y: f64| -> SurfacePoint { (Point2::new(x, y), Point3::new(x, y, 0.0)).into() };
        (0..visits)
            .flat_map(|_| {
                [
                    corner(0.0, 0.0),
                    corner(10.0, 0.0),
                    corner(10.0, 10.0),
                    corner(0.0, 10.0),
                ]
            })
            .chain([corner(0.0, 0.0)])
            .collect()
    }

    /// D1: a terminal failure emits exactly one structured record by default.
    #[test]
    fn d1_every_failure_emits_exactly_one_record() {
        let _guard = env_guard();
        clean_env();
        let path = scratch("d1");
        let _ = std::fs::remove_file(&path);
        diagnosis::set_test_sink(Some(path.clone()));
        std::env::remove_var("TRUCK_FACE_DIAG");

        let failure = run_terminal("d1.step", 7, || {
            let reason = sphere_projection_refusal();
            diagnosis::fail(reason, diagnosis::failure_stage_for_reason(reason))
        });

        let rows = read_jsonl(&path);
        assert_eq!(rows.len(), 1, "exactly one record per terminal face");
        let row = &rows[0];
        assert_eq!(row["terminal_reason"], "BoundaryProjectionFailed");
        assert_eq!(row["disposition"], "Failed");
        assert_eq!(row["document_id"], "d1.step");
        assert_eq!(row["source_face_id"], 7);
        assert_eq!(row["schema_version"], 1);
        assert!(
            row["projection"].is_object(),
            "a projection refusal must carry its witness"
        );
        assert_eq!(
            failure.reason,
            TessellationFailureReason::BoundaryProjectionFailed
        );
        diagnosis::set_test_sink(None);
        let _ = std::fs::remove_file(&path);
    }

    /// D2: a certified intrinsic rejection emits exactly one record with
    /// `disposition = RejectedIntrinsic`.
    #[test]
    fn d2_rejection_emits_with_intrinsic_disposition() {
        let _guard = env_guard();
        clean_env();
        let path = scratch("d2");
        let _ = std::fs::remove_file(&path);
        diagnosis::set_test_sink(Some(path.clone()));
        std::env::remove_var("TRUCK_FACE_DIAG");

        let failure = run_terminal("d2.step", 3, || {
            diagnosis::reject(
                TessellationFailureReason::RejectedDegenerate,
                diagnosis::FailureStage::ValidityClassification,
                crate::tessellation::validity::FaceValidityCertificate::all_bounds_collapsed(0),
            )
        });

        let rows = read_jsonl(&path);
        assert_eq!(rows.len(), 1, "exactly one record per rejection");
        assert_eq!(rows[0]["terminal_reason"], "RejectedDegenerate");
        assert_eq!(rows[0]["disposition"], "RejectedIntrinsic");
        assert_eq!(rows[0]["failure_stage"], "ValidityClassification");
        assert_eq!(rows[0]["document_id"], "d2.step");
        assert_eq!(
            failure.reason,
            TessellationFailureReason::RejectedDegenerate
        );
        diagnosis::set_test_sink(None);
        let _ = std::fs::remove_file(&path);
    }

    /// D3: a face that tessellates emits no failure record.
    #[test]
    fn d3_success_emits_nothing() {
        let _guard = env_guard();
        clean_env();
        let path = scratch("d3");
        let _ = std::fs::remove_file(&path);
        diagnosis::set_test_sink(Some(path.clone()));
        std::env::remove_var("TRUCK_FACE_DIAG");

        let plane = plane_surface();
        let tol = 0.01;
        let lattice = unevidenced_lattice(&plane);
        let boundary = PolyBoundary::new(
            vec![PolyBoundaryPiece::untagged(square_loop(1))],
            &plane,
            tol,
            &lattice,
        );
        let mesh = trimming_tessellation_result(&plane, &boundary, tol, &lattice)
            .expect("the single square tessellates");
        assert!(!mesh.faces().is_empty());

        let rows = read_jsonl(&path);
        assert!(rows.is_empty(), "a successful face emits no failure record");
        diagnosis::set_test_sink(None);
        let _ = std::fs::remove_file(&path);
    }

    /// D4: a refusal that propagates through several layers emits exactly one
    /// terminal record — stage functions record witnesses, only the terminal
    /// finalizer emits.
    #[test]
    fn d4_exactly_once_through_multi_layer_refusal() {
        let _guard = env_guard();
        clean_env();
        let path = scratch("d4");
        let _ = std::fs::remove_file(&path);
        diagnosis::set_test_sink(Some(path.clone()));
        std::env::remove_var("TRUCK_FACE_DIAG");

        let failure = run_terminal("d4.step", 9, || {
            // Simulate an inner stage refusing, an intermediate layer
            // propagating it (as `trimming_tessellation_result` does), and a
            // second tessellation attempt that also refuses under suspension —
            // only the outer terminal finalizer emits.
            let inner = diagnosis::fail(
                TessellationFailureReason::BoundaryProjectionFailed,
                diagnosis::FailureStage::BoundaryProjection,
            );
            let _suspension = diagnosis::SinkSuspension::new();
            let _retried = diagnosis::fail(
                TessellationFailureReason::NoOddParityRegion,
                diagnosis::FailureStage::MaterialSelection,
            );
            drop(_suspension);
            inner
        });

        assert_eq!(
            failure.reason,
            TessellationFailureReason::BoundaryProjectionFailed
        );
        let rows = read_jsonl(&path);
        assert_eq!(
            rows.len(),
            1,
            "exactly one terminal record despite two attempted layers"
        );
        assert_eq!(rows[0]["terminal_reason"], "BoundaryProjectionFailed");
        diagnosis::set_test_sink(None);
        let _ = std::fs::remove_file(&path);
    }

    /// D5: with no diagnostic-related environment variables, a failure still
    /// emits (the default sink).
    #[test]
    fn d5_diagnostics_default_on() {
        let _guard = env_guard();
        clean_env();
        std::env::remove_var("TRUCK_FACE_DIAG");
        std::env::remove_var("TRUCK_FACE_DIAG_JSONL");
        std::env::remove_var("TRUCK_FORMAL_RECOVERY");
        let capture = diagnosis::capture_emissions();

        let failure = run_terminal("d5.step", 5, || {
            diagnosis::fail(
                TessellationFailureReason::ConstraintInsertionIncomplete,
                diagnosis::FailureStage::ConstraintInsertion,
            )
        });
        assert_eq!(
            failure.reason,
            TessellationFailureReason::ConstraintInsertionIncomplete
        );

        let emitted = capture.lock().unwrap().clone();
        assert_eq!(
            emitted.len(),
            1,
            "default emission is on without any env flag"
        );
        let row: serde_json::Value = serde_json::from_str(&emitted[0]).unwrap();
        assert_eq!(row["document_id"], "d5.step");
        diagnosis::clear_emission_capture();
    }

    /// D6: explicit opt-out suppresses external emission without changing the
    /// returned failure.
    #[test]
    fn d6_explicit_opt_out_suppresses_emission() {
        let _guard = env_guard();
        clean_env();
        let path = scratch("d6");
        let _ = std::fs::remove_file(&path);
        diagnosis::set_test_sink(Some(path.clone()));
        std::env::set_var("TRUCK_FACE_DIAG", "off");

        let failure = run_terminal("d6.step", 6, || {
            diagnosis::fail(
                TessellationFailureReason::NoOddParityRegion,
                diagnosis::FailureStage::MaterialSelection,
            )
        });
        assert_eq!(failure.reason, TessellationFailureReason::NoOddParityRegion);

        let rows = read_jsonl(&path);
        assert!(
            rows.is_empty(),
            "TRUCK_FACE_DIAG=off suppresses external emission"
        );
        diagnosis::set_test_sink(None);
        std::env::remove_var("TRUCK_FACE_DIAG");
        let _ = std::fs::remove_file(&path);
    }

    /// D7: a broken output sink is non-fatal and does not change the
    /// tessellation result.
    #[test]
    fn d7_sink_failure_is_non_fatal() {
        let _guard = env_guard();
        clean_env();
        std::env::remove_var("TRUCK_FACE_DIAG");
        // A path whose parent does not exist cannot be opened.
        std::env::set_var(
            "TRUCK_FACE_DIAG_JSONL",
            std::env::temp_dir()
                .join("look_diag002_no_such_parent_dir")
                .join("broken.jsonl"),
        );
        let capture = diagnosis::capture_emissions();

        let failure = run_terminal("d7.step", 4, || {
            diagnosis::fail(
                TessellationFailureReason::ConstraintInsertionIncomplete,
                diagnosis::FailureStage::ConstraintInsertion,
            )
        });
        assert_eq!(
            failure.reason,
            TessellationFailureReason::ConstraintInsertionIncomplete,
            "a broken sink must not change the failure",
        );
        // The fallback wrote the record to the default (captured) sink.
        let emitted = capture.lock().unwrap().clone();
        assert_eq!(
            emitted.len(),
            1,
            "record falls back to stderr and is not lost"
        );
        std::env::remove_var("TRUCK_FACE_DIAG_JSONL");
        diagnosis::clear_emission_capture();
    }

    /// D8: concurrent failed faces keep their own identity and witnesses.
    #[test]
    fn d8_parallel_faces_stay_isolated() {
        let _guard = env_guard();
        clean_env();
        let path = scratch("d8");
        let _ = std::fs::remove_file(&path);
        diagnosis::set_test_sink(Some(path.clone()));
        std::env::remove_var("TRUCK_FACE_DIAG");

        std::thread::scope(|scope| {
            for face_id in 0..8u64 {
                scope.spawn(move || {
                    let _ = run_terminal("d8.step", face_id, || {
                        diagnosis::fail(
                            TessellationFailureReason::BoundaryProjectionFailed,
                            diagnosis::FailureStage::BoundaryProjection,
                        )
                    });
                });
            }
        });

        let rows = read_jsonl(&path);
        assert_eq!(rows.len(), 8, "one record per concurrent failed face");
        let mut ids: Vec<u64> = rows
            .iter()
            .map(|row| row["source_face_id"].as_u64().unwrap())
            .collect();
        ids.sort_unstable();
        assert_eq!(
            ids,
            (0..8).collect::<Vec<u64>>(),
            "each face keeps its own id"
        );
        assert!(
            rows.iter()
                .all(|row| row["terminal_reason"] == "BoundaryProjectionFailed"),
            "each face keeps its own terminal reason"
        );
        diagnosis::set_test_sink(None);
        let _ = std::fs::remove_file(&path);
    }

    /// D10: diagnostics enabled vs explicitly disabled produce identical
    /// terminal outcomes and identical records.
    #[test]
    fn d10_opt_out_does_not_change_geometry() {
        let _guard = env_guard();
        clean_env();
        let path = scratch("d10");
        let _ = std::fs::remove_file(&path);

        // Enabled: record emitted to the file.
        diagnosis::set_test_sink(Some(path.clone()));
        std::env::remove_var("TRUCK_FACE_DIAG");
        let enabled = run_terminal("d10.step", 2, || {
            let reason = doubled_boundary_failure();
            diagnosis::fail(reason, diagnosis::failure_stage_for_reason(reason))
        });
        let enabled_json = serde_json::to_string(&enabled.diagnostic).unwrap();

        // Disabled: same pipeline, record built identically but not emitted.
        std::env::set_var("TRUCK_FACE_DIAG", "off");
        diagnosis::set_test_sink(None);
        let _ = std::fs::remove_file(&path);
        let disabled = run_terminal("d10.step", 2, || {
            let reason = doubled_boundary_failure();
            diagnosis::fail(reason, diagnosis::failure_stage_for_reason(reason))
        });
        let disabled_json = serde_json::to_string(&disabled.diagnostic).unwrap();

        assert_eq!(enabled.reason, disabled.reason);
        assert_eq!(
            enabled_json, disabled_json,
            "opt-out must not change the diagnostic content, only its emission"
        );
        assert!(
            read_jsonl(&path).is_empty(),
            "nothing emitted under opt-out"
        );
        std::env::remove_var("TRUCK_FACE_DIAG");
        let _ = std::fs::remove_file(&path);
    }
}
