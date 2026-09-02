//! BG-CK-P2-RESIDUAL — the Phase-2 gate measurement harness (WAVE W3,
//! FLOOR shape), INTEGRATION AMENDMENT (session 49).
//!
//! The wave-phase tree measured the structural seeds only (every pair
//! `integration_pending`). The composed chain now exists in-tree — W1's
//! `ssi.rs` (square-system + 3×3 Krawczyk) and W2's `ssi_trace.rs`
//! (`certified_pair_trace`) — so this harness fills its single marked
//! integration seam ([`run_certified_pair_pair`]), extracts rational Bézier
//! patches from the corpus's spline-carried faces through the LANDED
//! decomposition (`certified_map::admit_surface`, the Phase-1 map's
//! row-then-column Bézier cut — never re-derived here), measures the full
//! patch-pair product per admitted FACE pair under the frozen seed grid, maps
//! `TraceOutcome`/`SsiRefusal` into the harness's disposition buckets, and
//! prints the measured certify-rate and refusal distribution. `integration_pending`
//! disappears from the aggregate: every dispositioned pair is certified,
//! refused with a named cause, or unresolved.
//!
//! This is a MEASUREMENT. No threshold assertion in-tree; the certify-rate and
//! the refusal distribution are OUTPUTS published in
//! `docs/CERTIFIED_PHASE2_FLOOR.md`, never thresholds. Fail-closed is not
//! passable by refusing everything: the doc shows the certify-rate AND the
//! admitted mass (the FLOOR anomaly-column discipline carries over). The run
//! is bounded by a certified-trace budget; what completes is published with
//! the wall time and completion fraction — never silently truncated. No
//! `unwrap` (the crate denies it).
//!
//! Corpus subset: the booking's spline-mass rows (spline~spline, plane~spline,
//! cylinder~spline, cone~spline, spline~torus). Seeds named per file from the
//! landed prevalence buckets (the prevalence-adjacency machinery is re-walked,
//! not re-derived). The FLOOR STOP finding's anomaly pairs (adjacent
//! `certified_disjoint`, 4,381 mass) are NOT folded in: that is the Phase-1
//! dispatch/census disagreement, an open owner decision
//! (`loop/results/BG-CK-P1-FLOOR.STOP.json`), out of scope here — the doc
//! cites the STOP filing and states the exclusion.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use truck_certified::certified_map::admit_surface;
use truck_certified::formal::numeric::PositiveFinite;
use truck_certified::ssi::{RationalBipatch, SsiRefusal};
use truck_certified::ssi_types::{TraceOutcome, TraceRefusal};
use truck_stepio::r#in::Table;
use truck_stepio::r#in::step_geometry::{Curve3D, ElementarySurface, Point3, Surface, SweptCurve};
use truck_topology::compress::{CompressedEdgeIndex, CompressedShell};

/// The corpus subset rows Phase 2 owns (the booking's spline-mass rows), in
/// the prevalence pair-key order (`min~max` by class tag, lexicographic).
const SUBSET_ROWS: [&str; 5] = [
    "cone~spline",
    "cylinder~spline",
    "plane~spline",
    "spline~spline",
    "spline~torus",
];

/// The declared Phase-1 map admission τ for the landed decomposition
/// (`certified_map::admit_surface`). The map module's D-tau discipline: a
/// declared threshold, never inferred, never auto-tuned. This is a
/// parameterization-degeneracy admission bound, not a certify-rate threshold.
/// H-3: declared constant; small enough to admit regular corpus splines while
/// still refusing exactly-degenerate domains.
const DECOMPOSE_TAU: f64 = 1e-6;

/// The frozen seed grid's half-step about the domain midpoint: the midpoint
/// `(0.5, 0.5, 0.5, 0.5)` plus every dyadic offset `±1/4` in all four chart
/// parameters (17 seeds). H-3: `1/4 = 2^-2` is the dyadic literal.
const SEED_HALF_STEP: f64 = 0.25;

/// The certified-trace run budget (calls of the seam's production entry).
/// The corpus's patch-pair products are astronomically larger than any
/// affordable trace budget; the run stops when the budget is spent and
/// publishes the completion fraction. Override with `PHASE2_TRACE_BUDGET`.
/// H-3: declared integer bound.
const MAX_TRACE_CALLS: usize = 400;

/// The seven classifier buckets, in the Phase-1 fast-path dispatch order
/// (the prevalence census's classifier, copied verbatim as provenance).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Class {
    Plane,
    Cylinder,
    Cone,
    Torus,
    Sphere,
    Spline,
    Other,
}

impl Class {
    fn tag(self) -> &'static str {
        match self {
            Self::Plane => "plane",
            Self::Cylinder => "cylinder",
            Self::Cone => "cone",
            Self::Torus => "torus",
            Self::Sphere => "sphere",
            Self::Spline => "spline",
            Self::Other => "other",
        }
    }
}

/// Classify one face's support surface with the Phase-1 fast-path dispatch
/// order (prevalence classifier, verbatim).
fn classify(surface: &Surface) -> (Class, &'static str) {
    if look::step_support_schema_of(surface).plane().is_some() {
        return (Class::Plane, "plane");
    }
    if look::step_cylinder_of(surface).is_ok() {
        return (Class::Cylinder, "cylinder");
    }
    if look::step_cone_of(surface).is_ok() {
        return (Class::Cone, "cone");
    }
    if look::step::torus_deck::identify_source_torus_opt(surface).is_ok() {
        return (Class::Torus, "torus");
    }
    if matches!(
        surface,
        Surface::ElementarySurface(ElementarySurface::Sphere(_))
    ) {
        return (Class::Sphere, "sphere");
    }
    if matches!(
        surface,
        Surface::BSplineSurface(_) | Surface::NurbsSurface(_)
    ) {
        return (Class::Spline, "spline");
    }
    (Class::Other, representation_name(surface))
}

/// The representation's own name (prevalence classifier, verbatim).
fn representation_name(surface: &Surface) -> &'static str {
    match surface {
        Surface::ElementarySurface(e) => match e {
            ElementarySurface::Plane(_) => "plane",
            ElementarySurface::Sphere(_) => "sphere",
            ElementarySurface::CylindricalSurface(_) => "cylindrical_surface",
            ElementarySurface::ToroidalSurface(_) => "toroidal_surface",
            ElementarySurface::DegenerateToroidalSurface(_) => "degenerate_toroidal_surface",
            ElementarySurface::ConicalSurface(_) => "conical_surface",
        },
        Surface::SweptCurve(s) => match s {
            SweptCurve::ExtrudedCurve(_) => "surface_of_linear_extrusion",
            SweptCurve::RevolutedCurve(_) => "surface_of_revolution",
        },
        Surface::BSplineSurface(_) => "b_spline_surface",
        Surface::NurbsSurface(_) => "rational_b_spline_surface",
        Surface::OffsetSurface(_) => "offset_surface",
    }
}

/// Parse one STEP file into a table, through the exact reader the renderer
/// uses (`src/step.rs` `read_exchange` + `Table::from_owned_data_section`) —
/// the prevalence loader path, verbatim.
fn load_table(path: &Path) -> anyhow::Result<Table> {
    let bytes = std::fs::read(path)?;
    let text = std::str::from_utf8(&bytes)
        .map(std::borrow::Cow::Borrowed)
        .unwrap_or_else(|_| bytes.iter().map(|&b| b as char).collect::<String>().into());
    let mut exchange = match look::step::part21::parse(&text) {
        Ok(exchange) => exchange,
        Err(_) => ruststep::parser::parse(&text)
            .map_err(|error| anyhow::anyhow!("failed to parse STEP: {error}"))?,
    };
    if exchange.data.is_empty() {
        anyhow::bail!("STEP file contains no data section");
    }
    let section = exchange.data.swap_remove(0);
    Ok(Table::from_owned_data_section(section))
}

/// The Phase-2 disposition vocabulary: the five disposition buckets a measured
/// pair lands in, with the `refused:<cause>` bucket's named causes (no
/// catch-all).
///
/// The vocabulary holds BOTH the FLOOR pair-level dispositions (the landed
/// Phase-1 gate's `refused_unsupported` causes, carried over 1:1) AND the
/// Phase-2 trace-level named causes (the plan's own names). Every case is a
/// named variant — there is no `Other` arm, no catch-all. The trace-level
/// named causes are mapped from the shim's [`truck_certified::TraceRefusal`]
/// variants AT INTEGRATION (the doc carries the mapping table); the mapping
/// below is that documented integration mapping, applied to the composed
/// chain's `SsiRefusal`/`TraceRefusal` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RefusalCause {
    /// FLOOR pair-level cause: `PairUnsupported::Overlap`.
    Overlap,
    /// FLOOR pair-level cause: `PairUnsupported::CoincidentCircles`.
    CoincidentCircles,
    /// FLOOR pair-level cause: `PairUnsupported::UnrelatedTangency`.
    UnrelatedTangency,
    /// FLOOR pair-level cause: `PairUnsupported::UnsupportedPairClass`.
    UnsupportedPairClass,
    /// Phase-2 trace-level named cause (the plan's own name): the trace could
    /// not certify a transverse crossing.
    NonTransverse,
    /// Phase-2 trace-level named cause (the plan's own name): the frozen F3
    /// conditioning rule refused the box
    /// (`TraceRefusal::Conditioning`, `ConditioningBelowThreshold`).
    Conditioning,
    /// Phase-2 trace-level named cause (the plan's own name): a certified
    /// singular / collapsed-stratum branch reading.
    Singular,
}

impl RefusalCause {
    /// Every named refusal cause, in doc-table order: the FLOOR pair-level
    /// causes first, then the Phase-2 trace-level named causes.
    const ALL: [RefusalCause; 7] = [
        RefusalCause::Overlap,
        RefusalCause::CoincidentCircles,
        RefusalCause::UnrelatedTangency,
        RefusalCause::UnsupportedPairClass,
        RefusalCause::NonTransverse,
        RefusalCause::Conditioning,
        RefusalCause::Singular,
    ];

    /// The stable refusal-cause tag (the `refused:<cause>` bucket's cause).
    fn tag(self) -> &'static str {
        match self {
            Self::Overlap => "overlap",
            Self::CoincidentCircles => "coincident_circles",
            Self::UnrelatedTangency => "unrelated_tangency",
            Self::UnsupportedPairClass => "unsupported_pair_class",
            Self::NonTransverse => "non_transverse",
            Self::Conditioning => "conditioning",
            Self::Singular => "singular",
        }
    }
}

/// Every refusal-cause tag, for the exhaustive bucket tables.
fn all_refusal_cause_tags() -> Vec<&'static str> {
    RefusalCause::ALL.iter().map(|cause| cause.tag()).collect()
}

/// One measured pair's disposition: every pair lands in exactly one of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Disposition {
    CertifiedContact,
    CertifiedDisjoint,
    Refused(RefusalCause),
    Unresolved,
    IntegrationPending,
}

impl Disposition {
    /// The five top-level disposition bucket names.
    const FAMILIES: [&'static str; 5] = [
        "certified_contact",
        "certified_disjoint",
        "refused",
        "unresolved",
        "integration_pending",
    ];

    /// The top-level family this disposition lands in.
    fn family(self) -> &'static str {
        match self {
            Self::CertifiedContact => "certified_contact",
            Self::CertifiedDisjoint => "certified_disjoint",
            Self::Refused(_) => "refused",
            Self::Unresolved => "unresolved",
            Self::IntegrationPending => "integration_pending",
        }
    }
}

/// The disposition counters, one per bucket. The exhaustive match in
/// [`DispositionCounts::record`] is compile-time: adding a disposition variant
/// without a counter breaks the build (no catch-all).
#[derive(Debug, Default)]
struct DispositionCounts {
    certified_contact: usize,
    certified_disjoint: usize,
    refused: BTreeMap<&'static str, usize>,
    unresolved: usize,
    integration_pending: usize,
}

impl DispositionCounts {
    /// Record one pair's disposition into exactly one bucket.
    fn record(&mut self, disposition: Disposition) {
        match disposition {
            Disposition::CertifiedContact => self.certified_contact += 1,
            Disposition::CertifiedDisjoint => self.certified_disjoint += 1,
            Disposition::Refused(cause) => {
                *self.refused.entry(cause.tag()).or_insert(0) += 1;
            }
            Disposition::Unresolved => self.unresolved += 1,
            Disposition::IntegrationPending => self.integration_pending += 1,
        }
    }

    /// Total pairs counted across every bucket.
    fn total(&self) -> usize {
        self.certified_contact
            + self.certified_disjoint
            + self.refused.values().sum::<usize>()
            + self.unresolved
            + self.integration_pending
    }

    /// Merge another counter set into this one (per-file -> aggregate).
    fn merge(&mut self, other: &DispositionCounts) {
        self.certified_contact += other.certified_contact;
        self.certified_disjoint += other.certified_disjoint;
        self.unresolved += other.unresolved;
        self.integration_pending += other.integration_pending;
        for (cause, count) in &other.refused {
            *self.refused.entry(*cause).or_insert(0) += *count;
        }
    }
}

/// One measured file's Phase-2 census and measurement row.
struct Phase2FileRow {
    file: String,
    shells: usize,
    faces: usize,
    seeds: usize,
    seed_rows: BTreeMap<&'static str, usize>,
    admitted_pairs: usize,
    admitted_rows: BTreeMap<&'static str, usize>,
    not_admitted_reasons: BTreeMap<&'static str, usize>,
    unit_pairs_total: usize,
    completed_pairs: usize,
    truncated_pairs: usize,
    dispositions: DispositionCounts,
}

/// The corpus-wide run state shared across files.
struct RunState {
    /// The budget the run started with.
    initial_budget: usize,
    /// The remaining certified-trace budget (seam calls of
    /// `certified_pair_trace`). Zero ends the measurement; the remaining
    /// admitted pairs are reported truncated, never silently dropped.
    budget: usize,
    /// Set once the budget is exhausted; every later admitted pair is
    /// truncated (counted in the totals, not dispositioned).
    budget_spent: bool,
    /// Wall clock of the run.
    started: std::time::Instant,
}

impl RunState {
    fn new(budget: usize) -> Self {
        Self {
            initial_budget: budget,
            budget,
            budget_spent: false,
            started: std::time::Instant::now(),
        }
    }

    fn trace_calls_used(&self) -> usize {
        self.initial_budget - self.budget
    }
}

/// One spline-carried face's extraction: the rational Bézier patches the
/// LANDED decomposition produced, or the reason it could not be reached.
enum FacePatches {
    /// The face's rational Bézier patch list (landed `certified_map` cut).
    Patches(Vec<RationalBipatch>),
    /// The face's carrier cannot reach a landed decomposition, and why.
    ///
    /// `rational_nurbs`: a rational (NURBS) spline surface — the landed
    /// surface decomposition (`certified_map`, D-map) is non-rational only.
    /// `admission_refused`: the landed whole-domain admission refused the
    /// face (`MapRefusal` — degenerate or cannot-decide parameterization).
    NoPath(&'static str),
}

/// Decompose one spline surface into rational Bézier patches through the
/// LANDED decomposition (`certified_map::admit_surface`, the Phase-1 map's
/// row-then-column Bézier cut), reading the map's patch grids verbatim. Each
/// non-rational Bézier piece is a unit-weight [`RationalBipatch`] over its own
/// unit chart; the image of the piece (the surface's world-space geometry over
/// the span) is unchanged by the unit reparametrization.
fn spline_face_patches(surface: &Surface) -> FacePatches {
    let bsp = match surface {
        Surface::BSplineSurface(bsp) => bsp,
        // Rational NURBS surfaces cannot reach the landed (non-rational)
        // decomposition: the D-map's declared scope excludes them.
        Surface::NurbsSurface(_) => {
            return FacePatches::NoPath("rational_nurbs");
        }
        // Not spline-carried (the callers only route spline-classed faces).
        _ => return FacePatches::NoPath("non_spline_carrier"),
    };
    let tau = match PositiveFinite::new(DECOMPOSE_TAU) {
        Ok(tau) => tau,
        Err(_) => return FacePatches::NoPath("admission_refused"),
    };
    let map = match admit_surface(bsp, tau) {
        Ok(map) => map,
        Err(_) => return FacePatches::NoPath("admission_refused"),
    };
    let mut patches = Vec::new();
    for grid in map.patch_grids() {
        // grid[k][a][b]: `grid[k]` has `m + 1` rows (first axis `a`), each of
        // length `n + 1` (second axis `b`). Non-rational pieces carry the
        // constant positive unit weight certificate.
        let rows = grid[0].len();
        let cols = grid[0][0].len();
        let (m, n) = (rows - 1, cols - 1);
        if m == 0 || n == 0 {
            return FacePatches::NoPath("admission_refused");
        }
        let w: Vec<Vec<f64>> = (0..=m).map(|_| vec![1.0; n + 1]).collect();
        match RationalBipatch::new(m, n, grid, w) {
            Ok(patch) => patches.push(patch),
            Err(_) => return FacePatches::NoPath("admission_refused"),
        }
    }
    if patches.is_empty() {
        return FacePatches::NoPath("admission_refused");
    }
    FacePatches::Patches(patches)
}

/// The cached extraction of both faces of a subset pair (decomposing spline
/// carriers once per shell). Returns borrows of the two cache slots.
fn extract_pair<'a>(
    cache: &'a mut [Option<FacePatches>],
    surface_a: &Surface,
    surface_b: &Surface,
    index_a: usize,
    index_b: usize,
) -> (&'a FacePatches, &'a FacePatches) {
    if cache[index_a].is_none() {
        cache[index_a] = Some(spline_face_patches(surface_a));
    }
    if cache[index_b].is_none() {
        cache[index_b] = Some(spline_face_patches(surface_b));
    }
    let a = match cache[index_a].as_ref() {
        Some(patches) => patches,
        None => panic!("cache slot filled above"),
    };
    let b = match cache[index_b].as_ref() {
        Some(patches) => patches,
        None => panic!("cache slot filled above"),
    };
    (a, b)
}

/// The deterministic reason a subset face pair could not be built from two
/// patch lists (never the empty slot: both sides were extracted).
fn block_reason(left: &FacePatches, right: &FacePatches) -> &'static str {
    let reason_of = |r: &FacePatches| match r {
        FacePatches::NoPath(reason) => Some(*reason),
        FacePatches::Patches(_) => None,
    };
    match (reason_of(left), reason_of(right)) {
        (Some(a), Some(b)) if a != b => {
            if a <= b {
                a
            } else {
                b
            }
        }
        (Some(a), _) => a,
        (_, Some(b)) => b,
        _ => "non_spline_carrier",
    }
}

/// The Phase-2 subset row an unordered class pair belongs to, when it is one.
fn subset_row(a: Class, b: Class) -> Option<&'static str> {
    let (a, b) = if a.tag() <= b.tag() { (a, b) } else { (b, a) };
    match (a, b) {
        (Class::Cone, Class::Spline) => Some("cone~spline"),
        (Class::Cylinder, Class::Spline) => Some("cylinder~spline"),
        (Class::Plane, Class::Spline) => Some("plane~spline"),
        (Class::Spline, Class::Spline) => Some("spline~spline"),
        (Class::Spline, Class::Torus) => Some("spline~torus"),
        _ => None,
    }
}

/// Measure one file: the prevalence-adjacency re-walk (structural seeds) plus
/// the integration measurement over the admitted patch-pair products.
fn measure_file(path: &Path, root: &Path, state: &mut RunState) -> anyhow::Result<Phase2FileRow> {
    let table = load_table(path)?;
    let mut row = Phase2FileRow {
        file: path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/"),
        shells: table.shell.len(),
        faces: 0,
        seeds: 0,
        seed_rows: BTreeMap::new(),
        admitted_pairs: 0,
        admitted_rows: BTreeMap::new(),
        not_admitted_reasons: BTreeMap::new(),
        unit_pairs_total: 0,
        completed_pairs: 0,
        truncated_pairs: 0,
        dispositions: DispositionCounts::default(),
    };
    for (&shell_id, shell) in &table.shell {
        let (compressed, _losses) = table
            .to_compressed_shell_with_losses(shell_id, shell)
            .map_err(|error| anyhow::anyhow!("shell #{shell_id}: {error}"))?;
        measure_shell(&compressed, &mut row, state);
    }
    Ok(row)
}

/// Count one shell's faces and Phase-2 subset adjacent pairs, and measure the
/// admitted patch-pair products under the run budget.
fn measure_shell(
    shell: &CompressedShell<Point3, Curve3D, Surface>,
    row: &mut Phase2FileRow,
    state: &mut RunState,
) {
    let faces = &shell.faces;
    let classes: Vec<Class> = faces.iter().map(|face| classify(&face.surface).0).collect();
    row.faces += faces.len();

    // Per-face extraction cache (only spline-classed faces are routed, but the
    // slots are per shell face so the index math stays trivial).
    let mut patch_cache: Vec<Option<FacePatches>> = (0..faces.len()).map(|_| None).collect();

    let mut edge_faces: HashMap<usize, Vec<usize>> = HashMap::new();
    for (face_index, face) in faces.iter().enumerate() {
        for wire in &face.boundaries {
            for edge in wire {
                edge_faces
                    .entry(edge_index(edge))
                    .or_default()
                    .push(face_index);
            }
        }
    }

    let mut seen: HashSet<(usize, usize)> = HashSet::new();
    for mut adjacents in edge_faces.into_values() {
        adjacents.sort_unstable();
        adjacents.dedup();
        for (k, &index0) in adjacents.iter().enumerate() {
            for &index1 in &adjacents[k + 1..] {
                if !seen.insert((index0, index1)) {
                    continue;
                }
                let class_a = classes[index0];
                let class_b = classes[index1];
                let Some(key) = subset_row(class_a, class_b) else {
                    continue;
                };
                row.seeds += 1;
                *row.seed_rows.entry(key).or_insert(0) += 1;

                let (left, right) = extract_pair(
                    &mut patch_cache,
                    &faces[index0].surface,
                    &faces[index1].surface,
                    index0,
                    index1,
                );
                let (FacePatches::Patches(lhs_patches), FacePatches::Patches(rhs_patches)) =
                    (left, right)
                else {
                    *row.not_admitted_reasons
                        .entry(block_reason(left, right))
                        .or_insert(0) += 1;
                    continue;
                };
                // Admitted face pair: both sides decompose. Its full patch-pair
                // product is the unit-pair mass this gate certifies.
                row.admitted_pairs += 1;
                *row.admitted_rows.entry(key).or_insert(0) += 1;
                row.unit_pairs_total += lhs_patches.len() * rhs_patches.len();
                if state.budget_spent {
                    row.truncated_pairs += 1;
                    continue;
                }
                match trace_pair_product(lhs_patches, rhs_patches, &mut state.budget) {
                    Some(disposition) => {
                        row.dispositions.record(disposition);
                        row.completed_pairs += 1;
                    }
                    None => {
                        row.truncated_pairs += 1;
                        state.budget_spent = true;
                    }
                }
            }
        }
    }
}

/// The frozen seed grid of one unit-pair: the domain midpoint plus the dyadic
/// offsets `±1/4` in all four parameters (17 seeds). First certified box wins.
fn seed_grid() -> Vec<[f64; 4]> {
    let mut seeds = vec![[0.5; 4]];
    for d0 in [-SEED_HALF_STEP, SEED_HALF_STEP] {
        for d1 in [-SEED_HALF_STEP, SEED_HALF_STEP] {
            for d2 in [-SEED_HALF_STEP, SEED_HALF_STEP] {
                for d3 in [-SEED_HALF_STEP, SEED_HALF_STEP] {
                    seeds.push([0.5 + d0, 0.5 + d1, 0.5 + d2, 0.5 + d3]);
                }
            }
        }
    }
    seeds
}

/// Attempt the full patch-pair product of one face pair under the frozen seed
/// grid. First certified box wins; a pair whose whole product was attempted
/// without a certified box dispositions as its first named failure. `None`
/// when the run budget was exhausted mid-product (the pair is truncated, not
/// dispositioned).
fn trace_pair_product(
    lhs_patches: &[RationalBipatch],
    rhs_patches: &[RationalBipatch],
    budget: &mut usize,
) -> Option<Disposition> {
    let seeds = seed_grid();
    let mut first_failure: Option<Disposition> = None;
    for lhs in lhs_patches {
        for rhs in rhs_patches {
            for seed in &seeds {
                if *budget == 0 {
                    return None;
                }
                *budget -= 1;
                match run_certified_pair_pair(lhs, rhs, *seed) {
                    Ok(TraceOutcome::ClosedLoop { .. })
                    | Ok(TraceOutcome::Terminated { .. })
                    | Ok(TraceOutcome::Switched { .. }) => {
                        // A certified branch: the pair has a certified contact.
                        return Some(Disposition::CertifiedContact);
                    }
                    Ok(TraceOutcome::Refused(refusal)) => {
                        if first_failure.is_none() {
                            first_failure = Some(disposition_of_trace_refusal(refusal));
                        }
                    }
                    Err(refusal) => {
                        if first_failure.is_none() {
                            first_failure = Some(disposition_of_ssi_refusal(refusal));
                        }
                    }
                }
            }
        }
    }
    Some(first_failure.unwrap_or(Disposition::Unresolved))
}

/// Map a composed-chain trace refusal into exactly one disposition bucket.
fn disposition_of_trace_refusal(refusal: TraceRefusal) -> Disposition {
    match refusal {
        TraceRefusal::Conditioning(_) => Disposition::Refused(RefusalCause::Conditioning),
        TraceRefusal::Hull(_) => Disposition::Refused(RefusalCause::NonTransverse),
        TraceRefusal::Unresolved(_) => Disposition::Unresolved,
    }
}

/// Map a composed-chain square-system refusal into exactly one disposition
/// bucket (the doc's refusal-cause mapping table).
fn disposition_of_ssi_refusal(refusal: SsiRefusal) -> Disposition {
    match refusal {
        SsiRefusal::Conditioning(_) => Disposition::Refused(RefusalCause::Conditioning),
        SsiRefusal::PairClass(_) => Disposition::Refused(RefusalCause::UnsupportedPairClass),
        SsiRefusal::Hull(_) => Disposition::Refused(RefusalCause::NonTransverse),
        SsiRefusal::DeterminantSpansZero => Disposition::Refused(RefusalCause::Singular),
        SsiRefusal::InclusionNotStrict => Disposition::Refused(RefusalCause::NonTransverse),
        SsiRefusal::InvalidInput => Disposition::Refused(RefusalCause::UnsupportedPairClass),
    }
}

/// The shared edge identity of one compressed boundary entry.
fn edge_index(edge: &CompressedEdgeIndex) -> usize {
    edge.index
}

/// The aggregate JSON for the corpus run: the structural headline (the seed
/// files and their pair masses) plus the MEASURED gate columns — the admitted
/// mass beside the certify-rate (the FLOOR anomaly-column discipline) and the
/// refusal distribution by named cause.
fn aggregate_json(
    files: usize,
    seed_rows: &BTreeMap<&'static str, usize>,
    admitted_rows: &BTreeMap<&'static str, usize>,
    not_admitted_reasons: &BTreeMap<&'static str, usize>,
    unit_pairs_total: usize,
    state: &RunState,
    dispositions: &DispositionCounts,
) -> serde_json::Value {
    let refused = serde_json::json!({
        "overlap": dispositions.refused.get("overlap").copied().unwrap_or(0),
        "coincident_circles": dispositions.refused.get("coincident_circles").copied().unwrap_or(0),
        "unrelated_tangency": dispositions.refused.get("unrelated_tangency").copied().unwrap_or(0),
        "unsupported_pair_class": dispositions.refused.get("unsupported_pair_class").copied().unwrap_or(0),
        "non_transverse": dispositions.refused.get("non_transverse").copied().unwrap_or(0),
        "conditioning": dispositions.refused.get("conditioning").copied().unwrap_or(0),
        "singular": dispositions.refused.get("singular").copied().unwrap_or(0),
    });
    let seeds: usize = seed_rows.values().sum();
    let admitted: usize = admitted_rows.values().sum();
    let completed = dispositions.total();
    let certify_rate = if completed > 0 {
        serde_json::Value::from(dispositions.certified_contact as f64 / completed as f64)
    } else {
        serde_json::Value::Null
    };
    let completion = if admitted > 0 {
        serde_json::Value::from(completed as f64 / admitted as f64)
    } else {
        serde_json::Value::Null
    };
    serde_json::json!({
        "files": files,
        "seeds": seeds,
        "seed_rows": seed_rows,
        "admitted_pairs": admitted,
        "admitted_rows": admitted_rows,
        "not_admitted_reasons": not_admitted_reasons,
        "unit_pairs_total": unit_pairs_total,
        "unit_pairs_traced": state.trace_calls_used(),
        "completed_pairs": completed,
        "truncated_pairs": admitted.saturating_sub(completed),
        "completion_fraction": completion,
        "wall_seconds": state.started.elapsed().as_secs_f64(),
        "certified_contact": dispositions.certified_contact,
        "certified_disjoint": dispositions.certified_disjoint,
        "refused": refused,
        "unresolved": dispositions.unresolved,
        "admitted_mass": admitted,
        "certify_rate": certify_rate,
    })
}

/// One excluded file: it could not be measured, and why (loader finding).
struct ExcludedFile {
    file: String,
    error: String,
}

/// The corpus run: name the seeds, admit the decomposable face pairs, measure
/// the patch-pair products under the certified-trace budget, and print the
/// per-file rows plus the aggregate. Structural assertions only.
fn run_floor_measurement(root: &Path, state: &mut RunState) {
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).expect("corpus directory should be readable") {
            let path = entry.expect("corpus entry should be readable").path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            let extension = path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(str::to_ascii_lowercase);
            if matches!(extension.as_deref(), Some("step" | "stp")) {
                files.push(path);
            }
        }
    }
    files.sort();
    assert!(!files.is_empty(), "LOOK_CORPUS held no STEP files");

    let mut rows = Vec::new();
    let mut excluded = Vec::new();
    for path in &files {
        match measure_file(path, root, state) {
            Ok(row) => {
                let json = serde_json::json!({
                    "file": row.file,
                    "shells": row.shells,
                    "faces": row.faces,
                    "seeds": row.seeds,
                    "seed_rows": row.seed_rows,
                    "admitted_pairs": row.admitted_pairs,
                    "admitted_rows": row.admitted_rows,
                    "not_admitted_reasons": row.not_admitted_reasons,
                    "unit_pairs_total": row.unit_pairs_total,
                    "completed_pairs": row.completed_pairs,
                    "truncated_pairs": row.truncated_pairs,
                    "certified_contact": row.dispositions.certified_contact,
                });
                println!("{json}");
                rows.push(row);
            }
            Err(error) => excluded.push(ExcludedFile {
                file: path
                    .strip_prefix(root)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .replace('\\', "/"),
                error: error.to_string(),
            }),
        }
    }

    for file in &excluded {
        eprintln!("EXCLUDED {}: {}", file.file, file.error);
    }
    assert!(
        excluded.is_empty(),
        "{} corpus file(s) could not be measured; see RESULT for the loader finding",
        excluded.len()
    );

    let mut seed_rows: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut admitted_rows: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut not_admitted_reasons: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut dispositions = DispositionCounts::default();
    let mut unit_pairs_total = 0usize;
    for row in &rows {
        if row.shells > 0 {
            assert!(row.faces > 0, "{}: measured faces must be > 0", row.file);
        }
        let counted: usize = row.seed_rows.values().sum();
        assert_eq!(
            counted, row.seeds,
            "{}: every subset pair must be counted exactly once",
            row.file
        );
        for (key, count) in &row.seed_rows {
            assert!(
                SUBSET_ROWS.contains(key),
                "{}: {key} is not a Phase-2 subset row",
                row.file
            );
            *seed_rows.entry(*key).or_insert(0) += *count;
        }
        for (key, count) in &row.admitted_rows {
            *admitted_rows.entry(*key).or_insert(0) += *count;
        }
        for (reason, count) in &row.not_admitted_reasons {
            *not_admitted_reasons.entry(*reason).or_insert(0) += *count;
        }
        unit_pairs_total += row.unit_pairs_total;
        dispositions.merge(&row.dispositions);
    }

    for key in SUBSET_ROWS {
        assert!(
            seed_rows.contains_key(key),
            "Phase-2 subset row {key} carries no mass in the measured corpus"
        );
    }
    let seeds: usize = seed_rows.values().sum();
    let admitted: usize = admitted_rows.values().sum();
    assert_eq!(
        not_admitted_reasons.values().sum::<usize>() + admitted,
        seeds,
        "every subset pair is admitted or blocked by a named reason"
    );
    assert_eq!(
        dispositions.total(),
        admitted - rows.iter().map(|r| r.truncated_pairs).sum::<usize>(),
        "every completed pair landed in exactly one bucket"
    );
    let completed = dispositions.total();
    let truncated = admitted.saturating_sub(completed);
    let _ = truncated;

    let aggregate = aggregate_json(
        rows.len(),
        &seed_rows,
        &admitted_rows,
        &not_admitted_reasons,
        unit_pairs_total,
        state,
        &dispositions,
    );
    println!("CERTIFIED_PHASE2_FLOOR_AGGREGATE {aggregate}");
    if state.budget_spent {
        println!(
            "CERTIFIED_PHASE2_FLOOR_BUDGET_EXHAUSTED completed={completed} admitted={admitted} \
             unit_pairs_traced={} wall_seconds={:.1}",
            state.trace_calls_used(),
            state.started.elapsed().as_secs_f64()
        );
    }
}

#[test]
fn floor_harness_skips_cleanly_without_look_corpus() {
    let Some(root) = std::env::var_os("LOOK_CORPUS") else {
        eprintln!(
            "LOOK_CORPUS is unset: skipping the Phase-2 floor harness. \
             Point it at the look-corpus checkout to measure the Phase-2 gate."
        );
        return;
    };
    let root = PathBuf::from(root);
    assert!(root.is_dir(), "LOOK_CORPUS {root:?} is not a directory");
    let budget = std::env::var("PHASE2_TRACE_BUDGET")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(MAX_TRACE_CALLS);
    let mut state = RunState::new(budget);
    run_floor_measurement(&root, &mut state);
}

#[test]
fn floor_refusal_distribution_buckets_are_exhaustive() {
    let tags = all_refusal_cause_tags();
    let distinct: HashSet<&str> = tags.iter().copied().collect();
    assert_eq!(
        distinct.len(),
        tags.len(),
        "refusal cause tags must be distinct"
    );
    assert!(
        tags.iter().all(|tag| !tag.is_empty()),
        "no refusal cause may be unnamed"
    );
    assert!(
        !tags.contains(&"other") && !tags.contains(&"unknown") && !tags.contains(&"catch_all"),
        "the refusal vocabulary must have no catch-all bucket"
    );
    for floor_cause in [
        "overlap",
        "coincident_circles",
        "unrelated_tangency",
        "unsupported_pair_class",
    ] {
        assert!(
            tags.contains(&floor_cause),
            "FLOOR pair-level refusal cause {floor_cause} missing from the vocabulary"
        );
    }
    for trace_cause in ["non_transverse", "conditioning", "singular"] {
        assert!(
            tags.contains(&trace_cause),
            "Phase-2 trace-level cause {trace_cause} missing from the vocabulary"
        );
    }
    for disposition in [
        Disposition::CertifiedContact,
        Disposition::CertifiedDisjoint,
        Disposition::Refused(RefusalCause::Conditioning),
        Disposition::Unresolved,
        Disposition::IntegrationPending,
    ] {
        assert!(
            Disposition::FAMILIES.contains(&disposition.family()),
            "{} is not a top-level disposition bucket",
            disposition.family()
        );
    }
    // Every composed-chain refusal maps to exactly one bucket (exhaustive
    // matches, no catch-all).
    let ssi_cases = [
        (
            SsiRefusal::Conditioning(
                truck_certified::contract::Refusal::ConditioningBelowThreshold,
            ),
            "conditioning",
        ),
        (
            SsiRefusal::Hull(truck_certified::hull::HullRefusal::EnclosureUnavailable),
            "non_transverse",
        ),
        (SsiRefusal::DeterminantSpansZero, "singular"),
        (SsiRefusal::InclusionNotStrict, "non_transverse"),
    ];
    for (refusal, expected) in ssi_cases {
        match disposition_of_ssi_refusal(refusal) {
            Disposition::Refused(cause) => assert_eq!(cause.tag(), expected),
            other => panic!("refusal must map to a refused bucket, got {other:?}"),
        }
    }
    let trace_cases = [
        (
            TraceRefusal::Conditioning(
                truck_certified::contract::Refusal::ConditioningBelowThreshold,
            ),
            "conditioning",
        ),
        (
            TraceRefusal::Hull(truck_certified::hull::HullRefusal::DomainNotCompact),
            "non_transverse",
        ),
    ];
    for (refusal, expected) in trace_cases {
        match disposition_of_trace_refusal(refusal) {
            Disposition::Refused(cause) => assert_eq!(cause.tag(), expected),
            other => panic!("refusal must map to a refused bucket, got {other:?}"),
        }
    }
    match disposition_of_trace_refusal(TraceRefusal::Unresolved(
        truck_certified::formal::contact::GenericUnresolved::ClusteredRoots,
    )) {
        Disposition::Unresolved => {}
        other => panic!("a trace unresolved must map to unresolved, got {other:?}"),
    }
    let mut counts = DispositionCounts::default();
    for disposition in [
        Disposition::CertifiedContact,
        Disposition::CertifiedDisjoint,
        Disposition::Refused(RefusalCause::Conditioning),
        Disposition::Unresolved,
        Disposition::IntegrationPending,
        Disposition::IntegrationPending,
    ] {
        counts.record(disposition);
    }
    assert_eq!(counts.total(), 6, "six recorded pairs must sum to six");
    assert_eq!(counts.certified_contact, 1);
    assert_eq!(counts.certified_disjoint, 1);
    assert_eq!(counts.refused.get("conditioning"), Some(&1));
    assert_eq!(counts.unresolved, 1);
    assert_eq!(counts.integration_pending, 2);
}

#[test]
fn floor_admitted_mass_is_published_not_asserted() {
    // The FLOOR anomaly-column discipline carries over: the doc must show the
    // certify-rate AND the admitted mass side by side. The aggregate printer
    // emits both columns. This test checks the columns are PUBLISHED; it
    // asserts nothing numeric about any rate.
    let seed_rows: BTreeMap<&'static str, usize> = BTreeMap::new();
    let admitted_rows: BTreeMap<&'static str, usize> = BTreeMap::new();
    let not_admitted: BTreeMap<&'static str, usize> = BTreeMap::new();
    let state = RunState::new(0);
    let dispositions = DispositionCounts::default();
    let aggregate = aggregate_json(
        0,
        &seed_rows,
        &admitted_rows,
        &not_admitted,
        0,
        &state,
        &dispositions,
    );
    let has_admitted_mass = aggregate.get("admitted_mass").is_some();
    let has_rate_field = aggregate.get("certify_rate").is_some();
    let has_seeds = aggregate.get("seeds").is_some();
    let has_completed = aggregate.get("completed_pairs").is_some();
    let has_admitted = aggregate.get("admitted_pairs").is_some();
    assert!(has_admitted_mass, "aggregate must publish admitted_mass");
    assert!(
        has_rate_field,
        "aggregate must publish the certify-rate column"
    );
    assert!(has_seeds, "aggregate must publish the seed mass");
    assert!(has_completed, "aggregate must publish completed pairs");
    assert!(has_admitted, "aggregate must publish admitted pairs");
    // No threshold assertion in-tree: no `assert!`/`assert_eq!` line may
    // reference a rate or a floor numeric (source-scan discipline).
    let source = include_str!("certified_phase2_floor.rs");
    let threshold_tokens = [
        "certify_rate",
        ">= 0.8",
        ">= 0.95",
        "> 0.8",
        "> 0.95",
        ">=0.8",
        ">=0.95",
        "0.80",
        "0.95",
    ];
    for line in source.lines() {
        let is_assert_line =
            line.contains("assert!") || line.contains("assert_eq!") || line.contains("panic!");
        if !is_assert_line {
            continue;
        }
        let tripped = threshold_tokens.iter().any(|token| line.contains(token));
        assert!(!tripped, "rate threshold assertion found in-tree: {line}");
    }
}

#[test]
fn floor_integration_seam_is_single_and_marked() {
    let source = include_str!("certified_phase2_floor.rs");
    let seam_def = concat!("fn run_certified_", "pair_pair(");
    let seam_call = concat!("run_certified_", "pair_pair(");
    let seam_marker = concat!("BG-CK-P2-", "RESIDUAL integration seam");
    let definitions = count_occurrences(source, seam_def);
    assert_eq!(definitions, 1, "the integration seam must be single");
    // Exactly three lines carry the call syntax: the definition line itself,
    // the measurement's per-unit seam call (trace_pair_product), and the
    // seam's own structural test below. The measurement must route every unit
    // pair through this one seam.
    let call_syntax = count_occurrences(source, seam_call);
    assert_eq!(
        call_syntax, 3,
        "the single seam is the measurement's only production-call site"
    );
    let markers = count_occurrences(source, seam_marker);
    assert_eq!(markers, 1, "the seam must be marked exactly once");
    // Wired-phase discipline: no panic-stub macros remain. The needles are
    // assembled from pieces so the scanner's own text never matches.
    let unimplemented_needle = concat!("unimplement", "ed!");
    let todo_needle = concat!("todo", "!()");
    assert_eq!(
        count_occurrences(source, unimplemented_needle),
        0,
        "the seam must not carry a panic-stub body"
    );
    assert_eq!(
        count_occurrences(source, todo_needle),
        0,
        "the seam must not carry a panic-stub body"
    );
    // Wired honesty: the seam over a well-conditioned analytic pair certifies a
    // real branch at the midpoint seed (the pair's documented ground truth).
    let (p1, p2) = well_conditioned_patch_pair();
    match run_certified_pair_pair(&p1, &p2, [0.5, 0.5, 0.5, 0.5]) {
        Ok(outcome) => match outcome {
            TraceOutcome::ClosedLoop { .. }
            | TraceOutcome::Terminated { .. }
            | TraceOutcome::Switched { .. } => {}
            TraceOutcome::Refused(_) => panic!("midpoint seed must certify the fixture branch"),
        },
        Err(_) => panic!("midpoint seed must certify the fixture branch"),
    }
}

/// Count non-overlapping occurrences of `needle` in `haystack`.
fn count_occurrences(haystack: &str, needle: &str) -> usize {
    haystack.matches(needle).count()
}

/// The integration seam's well-conditioned exercise pair (the fixture kit's
/// documented ground truth): patch 1 `(u, v, v)`, patch 2 `(s, t, 1/4 + s/2)`.
fn well_conditioned_patch_pair() -> (RationalBipatch, RationalBipatch) {
    let u = chart_grid(1, 1, 0);
    let v = chart_grid(1, 1, 1);
    let p1 = unit_weight_patch([u.clone(), v.clone(), v.clone()]);
    let z2 = monomial_grid(1, 1, &[(0, 0, 0.25), (1, 0, 0.5)]);
    let p2 = unit_weight_patch([u, v, z2]);
    (p1, p2)
}

/// The first-parameter (`which == 0`) or second-parameter chart coordinate grid.
fn chart_grid(m: usize, n: usize, which: usize) -> Vec<Vec<f64>> {
    let mut grid = Vec::with_capacity(m + 1);
    for a in 0..=m {
        let mut row = Vec::with_capacity(n + 1);
        for b in 0..=n {
            let value = if which == 0 {
                a as f64 / m as f64
            } else {
                b as f64 / n as f64
            };
            row.push(value);
        }
        grid.push(row);
    }
    grid
}

fn binom(n: usize, k: usize) -> f64 {
    let mut numerator = 1u64;
    let mut denominator = 1u64;
    for i in 0..k {
        numerator *= (n - i) as u64;
        denominator *= (i + 1) as u64;
    }
    numerator as f64 / denominator as f64
}

fn add_monomial_term(grid: &mut [Vec<f64>], m: usize, n: usize, pu: usize, pv: usize, coeff: f64) {
    for (a, row) in grid.iter_mut().enumerate().skip(pu) {
        let fa = binom(a, pu) / binom(m, pu);
        for (b, cell) in row.iter_mut().enumerate().skip(pv) {
            let fb = binom(b, pv) / binom(n, pv);
            *cell += coeff * fa * fb;
        }
    }
}

fn monomial_grid(m: usize, n: usize, terms: &[(usize, usize, f64)]) -> Vec<Vec<f64>> {
    let mut grid = vec![vec![0.0; n + 1]; m + 1];
    for &(pu, pv, coeff) in terms {
        add_monomial_term(&mut grid, m, n, pu, pv, coeff);
    }
    grid
}

/// A unit-weight rational patch from explicit component grids.
fn unit_weight_patch(num: [Vec<Vec<f64>>; 3]) -> RationalBipatch {
    let m = num[0].len() - 1;
    let n = num[0][0].len() - 1;
    let w: Vec<Vec<f64>> = (0..=m).map(|_| vec![1.0; n + 1]).collect();
    match RationalBipatch::new(m, n, num, w) {
        Ok(patch) => patch,
        Err(_) => panic!("a valid unit-weight patch was refused"),
    }
}

/// BG-CK-P2-RESIDUAL integration seam — single and marked.
///
/// The ONLY site that calls the composed Phase-2 production entry
/// (`truck_certified::ssi_trace::certified_pair_trace`, W2's branch tracing
/// over W1's square-system + 3×3 Krawczyk pipeline). The wave-phase tree had
/// no chain, so the seam was compile-only and every pair was counted
/// `integration_pending`; at integration the chain landed and the seam was
/// wired in — the measurement's [`trace_pair_product`] routes every unit-pair
/// through this one function under the frozen seed grid. The parameters are
/// the two certified-admitted rational Bézier patches and one chart seed
/// `(u, v, s, t)`; the result is the chain's raw [`TraceOutcome`] or named
/// [`SsiRefusal`], which the harness maps into exactly one disposition bucket
/// (see [`disposition_of_trace_refusal`] / [`disposition_of_ssi_refusal`]).
fn run_certified_pair_pair(
    lhs: &RationalBipatch,
    rhs: &RationalBipatch,
    seed: [f64; 4],
) -> Result<TraceOutcome, SsiRefusal> {
    truck_certified::ssi_trace::certified_pair_trace(lhs, rhs, seed)
}
