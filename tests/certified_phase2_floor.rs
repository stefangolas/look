//! BG-CK-P2-RESIDUAL — the Phase-2 gate measurement harness (WAVE W3,
//! FLOOR shape).
//!
//! This is a MEASUREMENT packet, wave-phase scope. The production chain this
//! harness measures (dispatch admission → Bézier decomposition →
//! `SquareSystem3` → 3×3 Krawczyk → branch trace) does NOT exist in this tree:
//! W1's and W2's modules land at integration. The wave-phase harness compiles
//! against the SHIM ONLY ([`truck_certified::SquareSystem3`] etc. through the
//! landed dev-dependency edge) plus the fixture kit
//! ([`truck_certified::ssi_fixtures`]), and every measured pair is counted
//! `integration_pending` WITHOUT pretending to certify. Wave-phase numbers are
//! structural: pair counts, bucket totals, and named seeds. The certify-rate
//! table fills at integration.
//!
//! The corpus subset is the booking's spline-mass rows
//! (`docs/CERTIFIED_PHASE2_BOOKING.md`): spline~spline, plane~spline,
//! cylinder~spline, cone~spline, spline~torus (~60k pairs). The seeds are
//! named per file from the landed prevalence buckets — the adjacency machinery
//! of `tests/certified_prevalence.rs` is re-walked here, not re-derived. The
//! FLOOR STOP finding's anomaly pairs (adjacent `certified_disjoint`, 4,381
//! mass) are NOT folded in: that is the Phase-1 dispatch/census disagreement,
//! an open owner decision (`loop/results/BG-CK-P1-FLOOR.STOP.json`), out of
//! scope here — the doc cites the STOP filing and states the exclusion.
//!
//! Structural assertions only. No threshold assertion on any rate; the
//! certify-rate and the refusal distribution are OUTPUTS published in
//! `docs/CERTIFIED_PHASE2_FLOOR.md`, never thresholds. No `unwrap` (the crate
//! denies it).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use truck_certified::SquareSystem3;
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
/// variants AT INTEGRATION (the doc carries the mapping table); in the
/// wave-phase tree they exist here with the mapping documented and receive no
/// counts, because no pair is traced yet.
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
///
/// Wave-phase note: the only dispositions the wave-phase walk emits is
/// `IntegrationPending` — the production chain is not in this tree, so a pair
/// is never certified, refused, or left unresolved here. The full vocabulary
/// is what the FLOOR gate will publish at integration.
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

    /// Total pairs in the `refused:<cause>` bucket.
    fn refused_total(&self) -> usize {
        self.refused.values().sum()
    }
}

/// One measured file's Phase-2 seed row: which spline-mass pair mass this file
/// carries, per class pair.
struct Phase2FileRow {
    file: String,
    shells: usize,
    faces: usize,
    seeds: usize,
    seed_rows: BTreeMap<&'static str, usize>,
}

/// The Phase-2 subset row an unordered class pair belongs to, when it is one
/// (the prevalence pair-key order, min/max by class tag so plane/cylinder and
/// cylinder/plane are one bucket). Pairs outside the five spline-mass rows are
/// not this gate's seeds.
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

/// Measure one file's Phase-2 seed mass (prevalence-adjacency re-walk, same
/// loader path, measurement only).
fn measure_file(path: &Path, root: &Path) -> anyhow::Result<Phase2FileRow> {
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
    };
    for (&shell_id, shell) in &table.shell {
        let (compressed, _losses) = table
            .to_compressed_shell_with_losses(shell_id, shell)
            .map_err(|error| anyhow::anyhow!("shell #{shell_id}: {error}"))?;
        measure_shell(&compressed, &mut row);
    }
    Ok(row)
}

/// Count one shell's faces and Phase-2 subset adjacent pairs into the row.
///
/// Adjacency is `shell.face_adjacency()`'s relation — two faces sharing an
/// edge — computed on the compressed form exactly as the prevalence census
/// does (the adjacency machinery is copied, its semantics are not re-derived).
/// Only the Phase-2 subset rows are counted here; the spline-mass rows are
/// this gate's seeds.
fn measure_shell(shell: &CompressedShell<Point3, Curve3D, Surface>, row: &mut Phase2FileRow) {
    let faces = &shell.faces;
    let classes: Vec<Class> = faces.iter().map(|face| classify(&face.surface).0).collect();
    row.faces += faces.len();

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
                if seen.insert((index0, index1)) {
                    let class_a = classes[index0];
                    let class_b = classes[index1];
                    if let Some(key) = subset_row(class_a, class_b) {
                        row.seeds += 1;
                        *row.seed_rows.entry(key).or_insert(0) += 1;
                    }
                }
            }
        }
    }
}

/// The shared edge identity of one compressed boundary entry.
fn edge_index(edge: &CompressedEdgeIndex) -> usize {
    edge.index
}

/// The wave-phase per-pair decision. The production chain is absent in this
/// tree, so every measured subset pair is `integration_pending` by
/// construction — the harness reports the structural counts without
/// pretending to certify. At integration this decision is replaced by a call
/// to the single marked seam [`run_certified_pair_pair`], and this
/// pending-only answer becomes the seam's real disposition.
fn wave_phase_disposition() -> Disposition {
    Disposition::IntegrationPending
}

/// The aggregate JSON for the corpus walk: the structural headline plus the
/// disposition counts, the admitted mass column, and the (wave-phase: empty)
/// certify-rate field beside it.
///
/// `admitted_mass` is the number of subset pairs the gate would hand the
/// seam; `certify_rate` is `null` in the wave phase because no pair has been
/// certified yet (the doc's certify-rate table fills at integration). The
/// refusal distribution bucket table is seeded with every named cause so the
/// bucket is visibly exhaustive even at zero counts.
fn aggregate_json(
    files: usize,
    seed_rows: &BTreeMap<&'static str, usize>,
    dispositions: &DispositionCounts,
) -> serde_json::Value {
    let seeds: usize = seed_rows.values().sum();
    let refused = serde_json::json!({
        "overlap": dispositions.refused.get("overlap").copied().unwrap_or(0),
        "coincident_circles": dispositions.refused.get("coincident_circles").copied().unwrap_or(0),
        "unrelated_tangency": dispositions.refused.get("unrelated_tangency").copied().unwrap_or(0),
        "unsupported_pair_class": dispositions.refused.get("unsupported_pair_class").copied().unwrap_or(0),
        "non_transverse": dispositions.refused.get("non_transverse").copied().unwrap_or(0),
        "conditioning": dispositions.refused.get("conditioning").copied().unwrap_or(0),
        "singular": dispositions.refused.get("singular").copied().unwrap_or(0),
    });
    serde_json::json!({
        "files": files,
        "seeds": seeds,
        "seed_rows": seed_rows,
        "certified_contact": dispositions.certified_contact,
        "certified_disjoint": dispositions.certified_disjoint,
        "refused": refused,
        "unresolved": dispositions.unresolved,
        "integration_pending": dispositions.integration_pending,
        "admitted_mass": seeds,
        "certify_rate": serde_json::Value::Null,
    })
}

/// One excluded file: it could not be measured, and why (loader finding).
struct ExcludedFile {
    file: String,
    error: String,
}

/// The wave-phase corpus walk: name the seeds (per-file spline-mass pair
/// counts), count every subset pair into exactly one disposition bucket, print
/// the per-file rows and the aggregate headline, and assert structural sanity.
///
/// Prints one JSON row per file plus the `CERTIFIED_PHASE2_FLOOR_AGGREGATE`
/// line (census format discipline) so the doc's numbers are copy-out
/// reproducible.
fn run_floor_measurement(root: &Path) {
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
        match measure_file(path, root) {
            Ok(row) => {
                let json = serde_json::json!({
                    "file": row.file,
                    "shells": row.shells,
                    "faces": row.faces,
                    "seeds": row.seeds,
                    "seed_rows": row.seed_rows,
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
    let mut dispositions = DispositionCounts::default();
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
            // The per-pair disposition decision is recorded for every subset
            // pair (exactly one bucket each).
            for _ in 0..*count {
                dispositions.record(wave_phase_disposition());
            }
        }
    }

    // Seeds must be NAMED with nonzero pair mass: every spline-mass row the
    // booking assigns to Phase 2 is present in the aggregate. These are
    // structural facts (the pair rows exist), never rate thresholds.
    for key in SUBSET_ROWS {
        assert!(
            seed_rows.contains_key(key),
            "Phase-2 subset row {key} carries no mass in the measured corpus"
        );
    }

    // Every pair lands in exactly one disposition bucket. Wave phase: all
    // subset pairs are pending (the chain is absent), so the certified /
    // disjoint / refused / unresolved buckets are structurally zero — the
    // FLOOR anomaly's `certified_disjoint` mass is not produced here because
    // no dispatch runs in this tree (and it is out of scope per the STOP).
    let seeds: usize = seed_rows.values().sum();
    assert_eq!(dispositions.total(), seeds, "pairs outside all buckets");
    assert_eq!(
        dispositions.integration_pending, seeds,
        "wave phase must report every subset pair integration_pending"
    );
    assert_eq!(dispositions.certified_contact, 0, "no contact certified");
    assert_eq!(dispositions.certified_disjoint, 0, "no disjoint certified");
    assert_eq!(dispositions.refused_total(), 0, "no refusal produced");
    assert_eq!(dispositions.unresolved, 0, "no unresolved produced");

    let aggregate = aggregate_json(rows.len(), &seed_rows, &dispositions);
    println!("CERTIFIED_PHASE2_FLOOR_AGGREGATE {aggregate}");
}

#[test]
fn floor_harness_skips_cleanly_without_look_corpus() {
    let Some(root) = std::env::var_os("LOOK_CORPUS") else {
        eprintln!(
            "LOOK_CORPUS is unset: skipping the Phase-2 floor harness. \
             Point it at the look-corpus checkout to measure the wave-phase \
             structural run."
        );
        return;
    };
    let root = PathBuf::from(root);
    assert!(root.is_dir(), "LOOK_CORPUS {root:?} is not a directory");
    run_floor_measurement(&root);
}

#[test]
fn floor_refusal_distribution_buckets_are_exhaustive() {
    let tags = all_refusal_cause_tags();
    // Named cases only: every refusal cause has a distinct, non-empty tag and
    // there is no catch-all arm in the vocabulary.
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
    // The vocabulary holds the FLOOR pair-level causes (the landed
    // `PairUnsupported` variants of the Phase-1 gate, carried over 1:1)...
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
    // ... AND the Phase-2 trace-level named causes (the plan's own names,
    // mapped from the shim's `TraceRefusal` variants at integration).
    for trace_cause in ["non_transverse", "conditioning", "singular"] {
        assert!(
            tags.contains(&trace_cause),
            "Phase-2 trace-level cause {trace_cause} missing from the vocabulary"
        );
    }
    // Every disposition lands in exactly one of the five top-level buckets.
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
    // Counting through the record function places each disposition in exactly
    // one bucket (the exhaustive match has no catch-all arm).
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
    // certify-rate AND the admitted mass side by side, so "refuse everything"
    // cannot masquerade as the gate passing. The aggregate printer emits both
    // columns. This test checks the columns are PUBLISHED; it asserts nothing
    // numeric about any rate.
    let seed_rows: BTreeMap<&'static str, usize> = BTreeMap::new();
    let dispositions = DispositionCounts::default();
    let aggregate = aggregate_json(0, &seed_rows, &dispositions);
    let has_admitted_mass = aggregate.get("admitted_mass").is_some();
    let has_rate_field = aggregate.get("certify_rate").is_some();
    let has_integration_pending = aggregate.get("integration_pending").is_some();
    let has_seeds = aggregate.get("seeds").is_some();
    assert!(has_admitted_mass, "aggregate must publish admitted_mass");
    assert!(
        has_rate_field,
        "aggregate must publish the certify-rate column"
    );
    assert!(
        has_integration_pending,
        "aggregate must publish integration_pending"
    );
    assert!(has_seeds, "aggregate must publish the seed mass");
    // No threshold assertion in-tree: no `assert!`/`assert_eq!` line may
    // reference a rate or a floor numeric. The certify-rate is published, never
    // asserted (source-scan discipline, per the FLOOR packet's house rule).
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
    // The search needles are assembled from pieces so the needle text never
    // appears contiguously in the scanner's own source (the file is self-
    // included); only the seam's real definition and doc lines can match.
    let seam_def = concat!("fn run_certified_", "pair_pair(");
    let seam_call = concat!("run_certified_", "pair_pair(");
    let seam_marker = concat!("BG-CK-P2-", "RESIDUAL integration seam");
    // Single: the seam is defined exactly once.
    let definitions = count_occurrences(source, seam_def);
    assert_eq!(definitions, 1, "the integration seam must be single");
    // Compile-only: the corpus walk never calls the seam. Exactly two lines
    // carry the call syntax — the definition line itself and the seam's own
    // structural test below (one non-definition call).
    let call_syntax = count_occurrences(source, seam_call);
    assert_eq!(
        call_syntax, 2,
        "the compile-only seam must not be called by the measurement"
    );
    // Marked: the seam's doc comment carries the marker text (also quoted into
    // docs/CERTIFIED_PHASE2_FLOOR.md).
    let markers = count_occurrences(source, seam_marker);
    assert_eq!(markers, 1, "the seam must be marked exactly once");
    // Compile-only: the seam is free of the panic-stub macros. The needles are
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
    // Wave-phase honesty: exercised on a shim fixture square system the seam
    // answers `integration_pending`, never a certification (a compile-only
    // seam returning data would fake a measurement).
    match truck_certified::ssi_fixtures::well_conditioned_root() {
        Ok(fixture) => {
            let pending = run_certified_pair_pair(&fixture.system);
            assert_eq!(
                pending,
                Disposition::IntegrationPending,
                "the wave-phase seam must not certify"
            );
        }
        Err(error) => panic!("the well-conditioned fixture must construct: {error:?}"),
    }
}

/// Count non-overlapping occurrences of `needle` in `haystack`.
fn count_occurrences(haystack: &str, needle: &str) -> usize {
    haystack.matches(needle).count()
}

/// BG-CK-P2-RESIDUAL integration seam — single and marked.
///
/// The ONLY site that will call the Phase-2 production chain (dispatch
/// admission → Bézier decomposition → `SquareSystem3` → 3×3 Krawczyk →
/// branch trace) at integration. The wave-phase tree has no such chain — W1's
/// and W2's modules land at integration — so this seam is compile-only: the
/// corpus walk never calls it, and every measured subset pair is counted
/// `integration_pending` without pretending to certify (see
/// [`wave_phase_disposition`]). At integration the orchestrator amends this
/// function to the production chain's real inputs (the pair's certified-
/// admitted Bézier patches) and routes each measured pair through it.
///
/// The parameter is the shim's frozen square-system carrier
/// ([`truck_certified::SquareSystem3`], BG-CK-P2-CONTRACT); naming it here
/// pins the dev-dependency re-export reachability the integration relies on
/// (the packet's stop condition 2 load-bearing premise). Returning
/// `integration_pending` is the honest wave-phase answer, and it is never
/// reached by the measurement.
fn run_certified_pair_pair(system: &SquareSystem3) -> Disposition {
    let _ = system;
    Disposition::IntegrationPending
}
