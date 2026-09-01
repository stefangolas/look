//! BG-CK-P0-PREVALENCE — analytic-pair prevalence on the `LOOK_CORPUS` corpus.
//!
//! Certified-kernel Phase 0's exit gate needs a published prevalence table:
//! "analytic pairs are the majority" is a hypothesis, not a result. This
//! census loads every `.step`/`.stp` file under `LOOK_CORPUS` through the same
//! landed import path the renderer uses (`src/step.rs`: the part21 read with
//! the ruststep fallback, `Table::from_owned_data_section`, then
//! `to_compressed_shell_with_losses`), classifies every face's support surface
//! in the Phase-1 fast-path dispatch order, and prints JSON rows plus the
//! aggregate headline so `docs/CERTIFIED_PREVALENCE.md` numbers are copy-out
//! reproducible.
//!
//! This is a MEASUREMENT test, not a pass/fail gate. It asserts only
//! structural sanity — corpus found, faces > 0 per measured file, every face
//! in exactly one of the seven buckets. There is deliberately no threshold
//! assertion on the analytic fraction: a threshold would make the measurement
//! self-fulfilling.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use truck_stepio::r#in::Table;
use truck_stepio::r#in::step_geometry::{Curve3D, ElementarySurface, Point3, Surface, SweptCurve};
use truck_topology::compress::{CompressedEdgeIndex, CompressedShell};

/// The seven bucket tags, in the Phase-1 fast-path dispatch order.
const CLASS_TAGS: [&str; 7] = [
    "plane", "cylinder", "cone", "torus", "sphere", "spline", "other",
];

/// The seven classifier buckets, in the Phase-1 fast-path dispatch order.
///
/// `Other` carries a per-representation tag recorded separately for the doc;
/// for the histograms it is one bucket.
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

    /// The analytic set: the five carriers the Phase-1 class-2 fast path
    /// dispatches on.
    fn analytic(self) -> bool {
        matches!(
            self,
            Self::Plane | Self::Cylinder | Self::Cone | Self::Torus | Self::Sphere
        )
    }
}

/// Classify one face's support surface with the Phase-1 fast-path dispatch
/// order. Returns the class and the tag to record — the representation's own
/// name for the `Other` bucket, per the `NoStructuralReader` doctrine.
fn classify(surface: &Surface) -> (Class, &'static str) {
    // 1. Plane — the landed support schema (`identify_plane`) accepts it.
    if look::step_support_schema_of(surface).plane().is_some() {
        return (Class::Plane, "plane");
    }
    // 2. Cylinder — a `RevolutedCurve<Line<Point3>>`-shaped support that
    //    `identify_cylinder` certifies.
    if look::step_cylinder_of(surface).is_ok() {
        return (Class::Cylinder, "cylinder");
    }
    // 3. Cone — the same representation shape, `identify_cone` certifies it.
    if look::step_cone_of(surface).is_ok() {
        return (Class::Cone, "cone");
    }
    // 4. Torus — a `Torus` whose world-space parameters
    //    `identify_torus_world` certifies.
    if look::step::torus_deck::identify_source_torus_opt(surface).is_ok() {
        return (Class::Torus, "torus");
    }
    // 5. Sphere — no landed certified constructor exists; classified by the
    //    landed in-memory representation (a sphere-carried surface). Its
    //    evidence is representation-named, not certified-identified.
    if matches!(
        surface,
        Surface::ElementarySurface(ElementarySurface::Sphere(_))
    ) {
        return (Class::Sphere, "sphere");
    }
    // 6. Spline — B-spline / rational B-spline (Bézier) carried.
    if matches!(
        surface,
        Surface::BSplineSurface(_) | Surface::NurbsSurface(_)
    ) {
        return (Class::Spline, "spline");
    }
    // 7. Other(tag) — everything else, tagged by the representation's own name.
    (Class::Other, representation_name(surface))
}

/// The representation's own name, never a guess about what it "probably" is.
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
/// uses (`src/step.rs` `read_exchange` + `Table::from_owned_data_section`).
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

/// One measured file's row.
struct FileRow {
    file: String,
    /// How many shells the STEP table declared. A file with zero shells (the
    /// AP242 tessellated-surface variant) contributes zero faces legitimately.
    shells: usize,
    faces: usize,
    pairs: usize,
    analytic_pairs: usize,
    analytic_faces: usize,
    face_histogram: BTreeMap<&'static str, usize>,
    pair_histogram: BTreeMap<String, usize>,
    other_tags: BTreeMap<&'static str, usize>,
}

/// Classify every face and every adjacent face pair of one file.
fn census_file(path: &Path, root: &Path) -> anyhow::Result<FileRow> {
    let table = load_table(path)?;

    let mut row = FileRow {
        file: path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/"),
        shells: table.shell.len(),
        faces: 0,
        pairs: 0,
        analytic_pairs: 0,
        analytic_faces: 0,
        face_histogram: BTreeMap::new(),
        pair_histogram: BTreeMap::new(),
        other_tags: BTreeMap::new(),
    };

    // The same conversion the flat renderer path runs (`src/step.rs`
    // `parse_step_table`): every shell in the table, converted once with the
    // loss stream the renderer would emit. Tessellation is skipped — the
    // measurement reads the per-face support surfaces the conversion leaves on
    // each compressed face. The shell is censused in its compressed form, the
    // exact form production tessellates from; no editable-shell round trip
    // (`Shell::extract`) is introduced, because that extraction refuses
    // degenerate seam edges the renderer accepts on real corpus files.
    for (&shell_id, shell) in &table.shell {
        let (compressed, _losses) = table
            .to_compressed_shell_with_losses(shell_id, shell)
            .map_err(|error| anyhow::anyhow!("shell #{shell_id}: {error}"))?;
        census_shell(&compressed, &mut row);
    }
    Ok(row)
}

/// Classify one shell's faces and adjacent face pairs into the row.
///
/// Adjacency is `shell.face_adjacency()`'s relation — two faces sharing an
/// edge — computed on the compressed form: each face's boundaries name edge
/// indices, and two faces are adjacent exactly when they name a common edge.
/// Each unordered pair contributes one row; a pair sharing more than one edge
/// still contributes one row.
fn census_shell(shell: &CompressedShell<Point3, Curve3D, Surface>, row: &mut FileRow) {
    let faces = &shell.faces;
    let classes: Vec<(Class, &'static str)> =
        faces.iter().map(|face| classify(&face.surface)).collect();
    row.faces += faces.len();
    for &(class, tag) in &classes {
        *row.face_histogram.entry(class.tag()).or_insert(0) += 1;
        if class.analytic() {
            row.analytic_faces += 1;
        }
        if class == Class::Other {
            *row.other_tags.entry(tag).or_insert(0) += 1;
        }
    }

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
                    row.pairs += 1;
                    let (class_a, _) = classes[index0];
                    let (class_b, _) = classes[index1];
                    let key = pair_key(class_a, class_b);
                    *row.pair_histogram.entry(key).or_insert(0) += 1;
                    if class_a.analytic() && class_b.analytic() {
                        row.analytic_pairs += 1;
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

/// The unordered pair key, min/max by class tag so plane/cylinder and
/// cylinder/plane are one bucket.
fn pair_key(a: Class, b: Class) -> String {
    let (a, b) = if a.tag() <= b.tag() { (a, b) } else { (b, a) };
    format!("{}~{}", a.tag(), b.tag())
}

/// One excluded file: it could not be measured, and why (loader finding).
struct ExcludedFile {
    file: String,
    error: String,
}

#[test]
#[ignore = "corpus census: needs LOOK_CORPUS; run explicitly"]
fn certified_prevalence_census() {
    let Some(root) = std::env::var_os("LOOK_CORPUS") else {
        eprintln!(
            "LOOK_CORPUS is unset: skipping the certified-prevalence census. \
             Point it at the look-corpus checkout to measure."
        );
        return;
    };
    let root = PathBuf::from(root);
    assert!(root.is_dir(), "LOOK_CORPUS {root:?} is not a directory");

    let mut files = Vec::new();
    let mut pending = vec![root.clone()];
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
        match census_file(path, &root) {
            Ok(row) => {
                let json = serde_json::json!({
                    "file": row.file,
                    "shells": row.shells,
                    "faces": row.faces,
                    "pairs": row.pairs,
                    "analytic_pairs": row.analytic_pairs,
                    "analytic_faces": row.analytic_faces,
                    "face_histogram": row.face_histogram,
                    "pair_histogram": row.pair_histogram,
                    "other_tags": row.other_tags,
                });
                println!("{json}");
                rows.push(row);
            }
            Err(error) => excluded.push(ExcludedFile {
                file: path
                    .strip_prefix(&root)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .replace('\\', "/"),
                error: error.to_string(),
            }),
        }
    }

    // Structural sanity only. The measurement is the printed output, not a
    // threshold: the analytic fraction is reported, never asserted.
    for file in &excluded {
        eprintln!("EXCLUDED {}: {}", file.file, file.error);
    }
    assert!(
        excluded.is_empty(),
        "{} corpus file(s) could not be measured; see RESULT for the loader finding",
        excluded.len()
    );
    for row in &rows {
        // A file with shells must have faces: a silent zero would mean the
        // census stopped reading. A file with no shells (the AP242
        // tessellated-surface variant) legitimately has zero faces.
        if row.shells > 0 {
            assert!(row.faces > 0, "{}: measured faces must be > 0", row.file);
        }
        let classified: usize = row.face_histogram.values().sum();
        assert_eq!(
            classified, row.faces,
            "{}: every face must land in exactly one bucket",
            row.file
        );
        // Every bucket that appears is one of the seven — the classifier never
        // invents a class. A file is free not to contain a class.
        for &bucket in row.face_histogram.keys() {
            assert!(
                CLASS_TAGS.contains(&bucket),
                "{}: {bucket} is not one of the seven buckets",
                row.file
            );
        }
    }

    let files_total = files.len();
    let faces_total: usize = rows.iter().map(|row| row.faces).sum();
    let analytic_faces_total: usize = rows.iter().map(|row| row.analytic_faces).sum();
    let pairs_total: usize = rows.iter().map(|row| row.pairs).sum();
    let analytic_pairs_total: usize = rows.iter().map(|row| row.analytic_pairs).sum();
    let analytic_pair_fraction = analytic_pairs_total as f64 / pairs_total.max(1) as f64;
    let analytic_face_fraction = analytic_faces_total as f64 / faces_total.max(1) as f64;

    let aggregate = serde_json::json!({
        "files": files_total,
        "measured": rows.len(),
        "faces": faces_total,
        "analytic_faces": analytic_faces_total,
        "analytic_face_fraction": analytic_face_fraction,
        "pairs": pairs_total,
        "analytic_pairs": analytic_pairs_total,
        "analytic_pair_fraction": analytic_pair_fraction,
        "excluded": excluded
            .iter()
            .map(|e| serde_json::json!({"file": e.file, "error": e.error}))
            .collect::<Vec<_>>(),
    });
    println!("CERTIFIED_PREVALENCE_AGGREGATE {aggregate}");
}
