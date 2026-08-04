//! Screen 2 — face_census: Order-of-magnitude loss breakdown with explicit failure instrumentation.

use std::collections::HashMap;
use std::env;

use serde::Serialize;
use truck_meshalgo::prelude::*;
use truck_meshalgo::tessellation::diagnosis as face_diag;
use truck_stepio::r#in::{Table, step_geometry::Surface};

const RELATIVE_TOLERANCE: f64 = 0.001;
const DEGENERATE_TOLERANCE: f64 = 1.0e-3;
const MINIMUM_TOLERANCE: f64 = 1.0e-6;
const EDGE_SAMPLES: u32 = 4;

/// DIAG-001: one JSONL row per failed face, with an optional conversion-reason
/// tag for faces lost before tessellation.
#[derive(Serialize)]
struct FaceDiagRow {
    #[serde(flatten)]
    diagnosis: face_diag::FailedFaceDiagnosis,
    conversion_reason: Option<&'static str>,
}

fn surface_family_from_kind(kind: &str) -> face_diag::SurfaceFamily {
    match kind {
        "plane" => face_diag::SurfaceFamily::Plane,
        "cylinder" => face_diag::SurfaceFamily::Cylinder,
        "cone" => face_diag::SurfaceFamily::Cone,
        "sphere" => face_diag::SurfaceFamily::Sphere,
        "torus" => face_diag::SurfaceFamily::Torus,
        "extruded" => face_diag::SurfaceFamily::Extruded,
        "revolved" => face_diag::SurfaceFamily::Revolved,
        "bspline" => face_diag::SurfaceFamily::Bspline,
        "nurbs" => face_diag::SurfaceFamily::Nurbs,
        "offset" => face_diag::SurfaceFamily::Offset,
        _ => face_diag::SurfaceFamily::Unknown,
    }
}

/// How a face ended up contributing nothing to the render.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Bucket {
    stage: &'static str,
    reason: String,
    surface_kind: &'static str,
}

fn surface_kind(surface: &Surface) -> &'static str {
    use truck_stepio::r#in::step_geometry::{ElementarySurface, SweptCurve};
    match surface {
        Surface::ElementarySurface(e) => match e {
            ElementarySurface::Plane(_) => "plane",
            ElementarySurface::Sphere(_) => "sphere",
            ElementarySurface::CylindricalSurface(_) => "cylinder",
            ElementarySurface::ToroidalSurface(_) => "torus",
            ElementarySurface::ConicalSurface(_) => "cone",
        },
        Surface::SweptCurve(s) => match s {
            SweptCurve::ExtrudedCurve(_) => "extruded",
            SweptCurve::RevolutedCurve(_) => "revolved",
        },
        Surface::BSplineSurface(_) => "bspline",
        Surface::NurbsSurface(_) => "nurbs",
        Surface::OffsetSurface(_) => "offset",
    }
}

struct Census {
    declared: usize,
    rendered: usize,
    counts: HashMap<Bucket, usize>,
    examples: HashMap<Bucket, Vec<String>>,
}

impl Census {
    fn new() -> Self {
        Self {
            declared: 0,
            rendered: 0,
            counts: HashMap::new(),
            examples: HashMap::new(),
        }
    }

    fn record(&mut self, bucket: Bucket, example: Option<String>) {
        *self.counts.entry(bucket.clone()).or_default() += 1;
        if let Some(example) = example {
            let slot = self.examples.entry(bucket).or_default();
            if slot.len() < 3 {
                slot.push(example);
            }
        }
    }

    fn lost(&self) -> usize {
        self.counts.values().sum()
    }
}

fn load(path: &str) -> anyhow::Result<Table> {
    let bytes = std::fs::read(path)?;
    let text = std::str::from_utf8(&bytes)
        .map(std::borrow::Cow::Borrowed)
        .unwrap_or_else(|_| bytes.iter().map(|&b| b as char).collect::<String>().into());
    let mut exchange = match look::step::part21::parse(&text) {
        Ok(exchange) => exchange,
        Err(_) => ruststep::parser::parse(&text)
            .map_err(|error| anyhow::anyhow!("failed to parse STEP: {error}"))?,
    };
    let section = exchange.data.swap_remove(0);
    Ok(Table::from_owned_data_section(section))
}

fn census(table: &Table, into: &mut Census, ledger: bool, model_id: &str, diag_rows: &mut Vec<FaceDiagRow>) {
    let diag_enabled = face_diag::diag_enabled();
    let mut converted = Vec::new();
    for (_, shell) in table.shell.iter() {
        into.declared += shell.cfs_faces.len();
        if let Ok((cshell, losses)) = table.to_compressed_shell_with_losses(shell) {
            for loss in &losses {
                let example = loss.provenance.best_id().map(|id| id.to_string());
                if diag_enabled {
                    diag_rows.push(FaceDiagRow {
                        diagnosis: face_diag::FailedFaceDiagnosis {
                            model_id: model_id.into(),
                            source_face_id: loss.provenance.best_id().map(|id| id.get()),
                            terminal_reason: TessellationFailureReason::BoundaryConstructionFailed,
                            surface_family: face_diag::SurfaceFamily::Unknown,
                            chart_rank: 0,
                            periodic_axes: face_diag::PeriodicAxes { u: false, v: false },
                            bound_count: 0,
                            source_segment_count: 0,
                            synthetic_segment_count: 0,
                            lift_status: face_diag::ObservedLiftStatus::NotPeriodic,
                            deck_status: face_diag::ObservedDeckStatus::Rank0,
                            projection_status: face_diag::ObservedProjectionStatus::Unavailable,
                            insertion_conflicts: Vec::new(),
                            derived_bucket: face_diag::LossBucket::OtherTypedFailure,
                        },
                        conversion_reason: Some(loss.reason.tag()),
                    });
                }
                if ledger {
                    let id = loss
                        .provenance
                        .best_id()
                        .map(|id| id.to_string())
                        .unwrap_or_else(|| "-".into());
                    eprintln!(
                        "FACE\tdeclared_face_index=-\tsource_face_id={id}\t\
                         surface_kind=-\trendered=0\ttriangles=0\tstage=convert\treason={}",
                        loss.reason.tag()
                    );
                }
                into.record(
                    Bucket {
                        stage: "convert",
                        reason: loss.reason.tag().to_string(),
                        surface_kind: "-",
                    },
                    example,
                );
            }
            converted.push(cshell);
        }
    }

    let mut model = BoundingBox::<Point3>::new();
    for shell in &converted {
        for v in &shell.vertices {
            model.push(*v);
        }
        for edge in &shell.edges {
            let (a, b) = edge.curve.range_tuple();
            for i in 0..=EDGE_SAMPLES {
                model.push(
                    edge.curve
                        .subs(a + (b - a) * f64::from(i) / f64::from(EDGE_SAMPLES)),
                );
            }
        }
    }
    let scaled = model.diameter() * RELATIVE_TOLERANCE;
    let tolerance = if scaled.is_finite() && scaled > 0.0 {
        scaled.max(MINIMUM_TOLERANCE)
    } else {
        DEGENERATE_TOLERANCE
    };

    for shell in &converted {
        let kinds: Vec<&'static str> = shell
            .faces
            .iter()
            .map(|f| surface_kind(&f.surface))
            .collect();
        // Must exercise the same path production does, or it measures a
        // build nobody ships (REFINEMENT_AUDIT.md section 6).
        let outcome = shell.robust_triangulation_with_cylinder_outcome(
            tolerance,
            look::step_lattice_of,
            look::step_support_schema_of,
            look::step_curve_schema_of,
            look::step_cylinder_of,
            look::step_cylinder_curve_schema_of,
            look::step_cylinder_curve_family_of,
        );
        let meshed = &outcome.shell;
        for (i, face) in meshed.faces.iter().enumerate() {
            let kind = if i < kinds.len() { kinds[i] } else { "?" };
            let example = face.provenance.best_id().map(|id| id.to_string());
            let (rendered, reason): (u8, &str) = match &face.surface {
                None => (0, "NoSurfaceProduced"),
                Some(mesh) if mesh.faces().is_empty() => (0, "MeshedToNothing"),
                Some(_) => (1, "-"),
            };
            // Audit A1 requires a per-face triangle delta, not only a
            // rendered/lost flag: the material-parity defect is expected to
            // change *which* triangles a face keeps far more often than it
            // changes whether the face survives at all. A face that renders
            // both before and after while retaining a different triangle count
            // is invisible to `rendered`.
            let triangles = face
                .surface
                .as_ref()
                .map_or(0, |mesh| mesh.tri_faces().len());
            if rendered == 0 {
                into.record(
                    Bucket {
                        stage: "tessellate",
                        reason: reason.into(),
                        surface_kind: kind,
                    },
                    example,
                );
            } else {
                into.rendered += 1;
            }
            if ledger {
                let id = face
                    .provenance
                    .best_id()
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "-".into());
                eprintln!(
                    "FACE\tdeclared_face_index={i}\tsource_face_id={id}\t\
                     surface_kind={kind}\trendered={rendered}\ttriangles={triangles}\t\
                     stage=tessellate\treason={reason}"
                );
            }
            // DIAG-001: collect structured diagnostic records for failed faces.
            if diag_enabled && rendered == 0 {
                if let Some(diag) = outcome.face_diagnoses.get(i).and_then(|d| d.as_ref()) {
                    let mut d = diag.clone();
                    d.model_id = model_id.into();
                    d.surface_family = surface_family_from_kind(kind);
                    diag_rows.push(FaceDiagRow {
                        diagnosis: d,
                        conversion_reason: None,
                    });
                } else if let Some(failure) =
                    outcome.face_failures.get(i).cloned().flatten()
                {
                    let orig = &shell.faces[i].surface;
                    let mut d = face_diag::FailedFaceDiagnosis {
                        model_id: model_id.into(),
                        source_face_id: face.provenance.best_id().map(|id| id.get()),
                        terminal_reason: failure.reason,
                        surface_family: surface_family_from_kind(kind),
                        chart_rank: u8::from(orig.u_period().is_some())
                            + u8::from(orig.v_period().is_some()),
                        periodic_axes: face_diag::PeriodicAxes {
                            u: orig.u_period().is_some(),
                            v: orig.v_period().is_some(),
                        },
                        bound_count: face.boundaries.len(),
                        source_segment_count: 0,
                        synthetic_segment_count: 0,
                        lift_status: face_diag::ObservedLiftStatus::Unavailable,
                        deck_status: face_diag::ObservedDeckStatus::Unavailable,
                        projection_status: face_diag::ObservedProjectionStatus::Unavailable,
                        insertion_conflicts: Vec::new(),
                        derived_bucket: face_diag::LossBucket::InsertionUnknown,
                    };
                    diag_rows.push(FaceDiagRow {
                        diagnosis: d,
                        conversion_reason: None,
                    });
                }
            }
        }
    }
}

/// Aggregates the step-0 `TRUCK_PROBE_EVIDENCE` records.
///
/// The probe writes one tab-separated line per face to stderr from inside a
/// parallel tessellation, so records arrive interleaved and in no fixed order.
/// This reads them back from a captured log and counts populations. Two runs'
/// logs are not comparable byte-for-byte; their aggregates are.
///
/// ```console
/// TRUCK_PROBE_EVIDENCE=1 face_census model.step 2> probe.log
/// face_census --evidence-report probe.log
/// ```
///
/// Remove with the probe once `SourceFaceInput` becomes the production input.
fn evidence_report(path: &str) -> anyhow::Result<()> {
    let text = std::fs::read_to_string(path)?;
    let mut faces = 0usize;
    let mut by_field: HashMap<(&'static str, String), usize> = HashMap::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut duplicates = 0usize;

    for line in text.lines() {
        let Some(rest) = line.strip_prefix("EVIDENCE\t") else {
            continue;
        };
        let fields: HashMap<&str, &str> = rest
            .split('\t')
            .filter_map(|field| field.split_once('='))
            .collect();
        faces += 1;
        // A face is identified by its document id when it has one, and only
        // then by position — the same rule the ledger uses. A repeat means the
        // log mixes runs, which would silently double every population.
        let key = match fields.get("source_face_id") {
            Some(&id) if id != "none" => format!("id:{id}"),
            _ => format!("idx:{}", fields.get("declared_face_index").unwrap_or(&"-")),
        };
        if !seen.insert(key) {
            duplicates += 1;
        }
        for name in [
            "endpoint_ids_complete",
            "endpoints_consistent",
            "continuous_regular_bounds",
            "computable_normalized_signs",
            "bounds_degenerate",
            "face_use_orientation",
            "face_surface_history",
            "declared_rank",
            "certified_rank",
            "adapter_error",
        ] {
            if let Some(value) = fields.get(name) {
                let label: &'static str = name;
                *by_field.entry((label, (*value).to_string())).or_default() += 1;
            }
        }
    }

    if faces == 0 {
        anyhow::bail!("no EVIDENCE records in {path} — was TRUCK_PROBE_EVIDENCE set?");
    }
    // The two ratio fields are the quantitative result, so they are also
    // totalled. Their per-face distribution stays in the table below, because
    // "every face is 1/1" and "half are 2/2 and half are 0/2" have the same
    // total and are not the same finding.
    let mut totals: HashMap<&'static str, (u64, u64)> = HashMap::new();
    for line in text.lines() {
        let Some(rest) = line.strip_prefix("EVIDENCE\t") else {
            continue;
        };
        let fields: HashMap<&str, &str> = rest
            .split('\t')
            .filter_map(|field| field.split_once('='))
            .collect();
        for (name, value) in &fields {
            let (name, value) = (*name, *value);
            let label: &'static str = match name {
                "continuous_regular_bounds" => "continuous_regular_bounds",
                "computable_normalized_signs" => "computable_normalized_signs",
                // Per-factor counts, totalled against the face's edge-use
                // count so an incremental repair is visible as a fraction.
                "edge_curve_retained" => "edge_curve_retained",
                "edge_curve_history_erased" => "edge_curve_history_erased",
                "selected_curve_retained" => "selected_curve_retained",
                "selected_curve_history_erased" => "selected_curve_history_erased",
                _ => continue,
            };
            let uses: u64 = fields
                .get("edge_uses")
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
            let (n, d) = match value.split_once('/') {
                Some((numerator, denominator)) => (
                    numerator.parse::<u64>().ok(),
                    denominator.parse::<u64>().ok(),
                ),
                None => (value.parse::<u64>().ok(), Some(uses)),
            };
            if let (Some(n), Some(d)) = (n, d) {
                let slot = totals.entry(label).or_default();
                slot.0 += n;
                slot.1 += d;
            }
        }
    }
    let mut total_rows: Vec<(&str, (u64, u64))> = totals.into_iter().collect();
    total_rows.sort();
    for (name, (n, d)) in &total_rows {
        let share = match d {
            0 => 0.0,
            _ => *n as f64 / *d as f64 * 100.0,
        };
        println!("  total {name:36} {n:>9} / {d:<9} {share:5.1}%");
    }
    if !total_rows.is_empty() {
        println!();
    }
    println!("{faces} face records from {path}");
    if duplicates > 0 {
        println!(
            "  WARNING: {duplicates} duplicate face keys — this log mixes runs, \
             so every population below is inflated"
        );
    }
    println!();
    let mut rows: Vec<((&str, String), usize)> = by_field.into_iter().collect();
    rows.sort_by(|a, b| a.0.0.cmp(b.0.0).then_with(|| b.1.cmp(&a.1)));
    println!(
        "  {:38} {:32} {:>7}  {:>6}",
        "field", "value", "faces", "share"
    );
    for ((field, value), count) in &rows {
        let share = *count as f64 / faces as f64 * 100.0;
        println!("  {field:38} {value:32} {count:>7}  {share:5.1}%");
    }
    Ok(())
}

/// Aggregates the step-1 `TRUCK_PROBE_AMBIENT` records.
///
/// Same reading discipline as [`evidence_report`]: the probe writes one
/// tab-separated line per successfully converted face to stderr from inside a
/// parallel tessellation, so records interleave and arrive in no fixed order.
///
/// ```console
/// TRUCK_PROBE_AMBIENT=1 face_census model.step 2> ambient.log
/// face_census --ambient-report ambient.log
/// ```
///
/// **The formal-resolution histogram is not expected to equal the legacy
/// `certified_rank` histogram, and is deliberately not forced to.** The legacy
/// rank counts certified generators; a face with zero of them may be genuinely
/// aperiodic *or* may have declared a period nothing certified. Separating
/// those two populations is the entire point of the step, so a divergence here
/// is the result, not a discrepancy to reconcile away.
///
/// The cross-tabulation at the end is the acceptance check: it shows what the
/// formal model concludes for each legacy `(declared_rank, certified_rank)`
/// cell, and in particular that no face in the `declared 2 / certified 0` or
/// `declared 1 / certified 0` cells resolves as formal rank 0.
/// The planar-holes obstruction funnel, from a `TRUCK_PROBE_SLICE` log.
///
/// The `SLICE` funnel reports every planar rank-0 candidate and collapses each
/// multi-bound face into one `multiple_bounds_or_holes` bucket. This reads the
/// `HOLES` records the same run emits and splits that bucket by the obstruction
/// that actually stopped the face, which is what the next expansion is planned
/// from. Representative face ids are printed for every bucket, because a
/// population without an example cannot be investigated.
fn holes_report(path: &str) -> anyhow::Result<()> {
    let text = std::fs::read_to_string(path)?;
    let mut faces = 0usize;
    let mut by_exit: HashMap<(String, String), (usize, Vec<String>)> = HashMap::new();
    let mut by_stage: HashMap<String, usize> = HashMap::new();
    let mut by_shape: HashMap<String, (usize, Vec<String>)> = HashMap::new();
    let mut recovered = 0usize;
    let mut recovered_triangles = 0usize;
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut duplicates = 0usize;

    for line in text.lines() {
        let Some(rest) = line.strip_prefix("HOLES\t") else {
            continue;
        };
        let fields: HashMap<&str, &str> = rest
            .split('\t')
            .filter_map(|field| field.split_once('='))
            .collect();
        faces += 1;
        let id = match fields.get("source_face_id") {
            Some(&id) if id != "none" => format!("#{id}"),
            _ => format!(
                "shell:{}/idx:{}",
                fields.get("shell_ordinal").unwrap_or(&"-"),
                fields.get("declared_face_index").unwrap_or(&"-")
            ),
        };
        if !seen.insert(id.clone()) {
            duplicates += 1;
        }
        let get = |name: &str| (*fields.get(name).unwrap_or(&"-")).to_string();
        let (stage, category, exit) = (get("stage"), get("category"), get("exit"));
        *by_stage.entry(stage.clone()).or_default() += 1;

        // The bucket is the exit reason, kept beside its category so an
        // `Unsupported` refusal is never read as a defect and vice versa.
        // The exit alone under-reports: "an unsupported curve" is different
        // work depending on whether it sits on the authoritative outer bound or
        // on a hole, so the attributed bound is part of the bucket key.
        let role = get("obstruction_bound");
        let bucket = match exit.as_str() {
            "none" => ("resolved_and_recovered".to_string(), category.clone()),
            other if role == "none" || role == "-" => (other.to_string(), category.clone()),
            other => (format!("{other} [{role}]"), category.clone()),
        };
        let entry = by_exit.entry(bucket).or_default();
        entry.0 += 1;
        if entry.1.len() < 5 {
            entry.1.push(id.clone());
        }

        // The bound shape, which says how much of the population one outer plus
        // one inner would cover.
        let inner = get("inner_bounds");
        let shape = match inner.as_str() {
            "1" => "one_outer_one_inner".to_string(),
            "-" => "unknown".to_string(),
            many => format!("one_outer_{many}_inner"),
        };
        let shape_entry = by_shape.entry(shape).or_default();
        shape_entry.0 += 1;
        if shape_entry.1.len() < 5 {
            shape_entry.1.push(id.clone());
        }

        if exit == "none" && stage == "final_validity" {
            recovered += 1;
            recovered_triangles += get("triangles").parse::<usize>().unwrap_or(0);
        }
    }

    if faces == 0 {
        anyhow::bail!("no HOLES records in {path} — was TRUCK_PROBE_SLICE set?");
    }
    println!("{faces} multi-bound planar rank-0 candidates from {path}");
    if duplicates > 0 {
        println!(
            "  WARNING: {duplicates} duplicate face keys — this log mixes runs, \
             so every population below is inflated"
        );
    }
    println!("  resolved and meshed: {recovered} ({recovered_triangles} triangles)");
    println!();

    println!("  bound shape");
    let mut shapes: Vec<(String, (usize, Vec<String>))> = by_shape.into_iter().collect();
    shapes.sort_by(|a, b| b.1 .0.cmp(&a.1 .0).then_with(|| a.0.cmp(&b.0)));
    println!("  {:28} {:>7}  {:>6}  {}", "shape", "faces", "share", "examples");
    for (shape, (count, examples)) in &shapes {
        let share = *count as f64 / faces as f64 * 100.0;
        println!(
            "  {shape:28} {count:>7}  {share:5.1}%  {}",
            examples.join(" ")
        );
    }
    println!();

    println!("  furthest stage reached");
    let mut stages: Vec<(String, usize)> = by_stage.into_iter().collect();
    stages.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    for (stage, count) in &stages {
        let share = *count as f64 / faces as f64 * 100.0;
        println!("  {stage:40} {count:>7}  {share:5.1}%");
    }
    println!();

    println!("  obstruction funnel");
    let mut rows: Vec<((String, String), (usize, Vec<String>))> = by_exit.into_iter().collect();
    rows.sort_by(|a, b| b.1 .0.cmp(&a.1 .0).then_with(|| a.0.cmp(&b.0)));
    println!(
        "  {:40} {:18} {:>7}  {:>6}  {}",
        "exit", "category", "faces", "share", "examples"
    );
    for ((exit, category), (count, examples)) in &rows {
        let share = *count as f64 / faces as f64 * 100.0;
        println!(
            "  {exit:40} {category:18} {count:>7}  {share:5.1}%  {}",
            examples.join(" ")
        );
    }
    Ok(())
}

fn ambient_report(path: &str) -> anyhow::Result<()> {
    let text = std::fs::read_to_string(path)?;
    let mut faces = 0usize;
    let mut by_field: HashMap<(&'static str, String), usize> = HashMap::new();
    // Legacy `(declared_rank, certified_rank)` against formal resolution and
    // rank. Keyed as strings so a missing field is visible rather than zeroed.
    let mut cross: HashMap<(String, String, String, String), usize> = HashMap::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut duplicates = 0usize;
    let mut adapter_errors = 0usize;

    for line in text.lines() {
        let Some(rest) = line.strip_prefix("AMBIENT\t") else {
            continue;
        };
        let fields: HashMap<&str, &str> = rest
            .split('\t')
            .filter_map(|field| field.split_once('='))
            .collect();
        faces += 1;
        // A face is identified by its document id when it has one, and only
        // then by shell and position — `declared_face_index` is per-shell and
        // collides between shells, which is why the probe emits the ordinal.
        let key = match fields.get("source_face_id") {
            Some(&id) if id != "none" => format!("id:{id}"),
            _ => format!(
                "shell:{}/idx:{}",
                fields.get("shell_ordinal").unwrap_or(&"-"),
                fields.get("declared_face_index").unwrap_or(&"-")
            ),
        };
        if !seen.insert(key) {
            duplicates += 1;
        }
        if fields
            .get("adapter_error")
            .is_some_and(|value| *value != "none")
        {
            adapter_errors += 1;
        }
        for name in [
            "u_state",
            "v_state",
            "declared_rank",
            "legacy_certified_rank",
            "formal_resolution",
            "formal_rank",
            "unresolved_reason",
            "inconsistency_reason",
            "unsupported_clause",
            "authoritative_generator_count",
            "diagnostic_hint_count",
            "adapter_error",
        ] {
            if let Some(value) = fields.get(name) {
                let label: &'static str = name;
                *by_field.entry((label, (*value).to_string())).or_default() += 1;
            }
        }
        let get = |name: &str| (*fields.get(name).unwrap_or(&"-")).to_string();
        *cross
            .entry((
                get("declared_rank"),
                get("legacy_certified_rank"),
                get("formal_resolution"),
                get("formal_rank"),
            ))
            .or_default() += 1;
    }

    if faces == 0 {
        anyhow::bail!("no AMBIENT records in {path} — was TRUCK_PROBE_AMBIENT set?");
    }
    println!("{faces} face records from {path}");
    if duplicates > 0 {
        println!(
            "  WARNING: {duplicates} duplicate face keys — this log mixes runs, \
             so every population below is inflated"
        );
    }
    println!("  adapter errors: {adapter_errors}");
    println!();

    let mut rows: Vec<((&str, String), usize)> = by_field.into_iter().collect();
    rows.sort_by(|a, b| a.0.0.cmp(b.0.0).then_with(|| b.1.cmp(&a.1)));
    println!(
        "  {:34} {:34} {:>7}  {:>6}",
        "field", "value", "faces", "share"
    );
    for ((field, value), count) in &rows {
        let share = *count as f64 / faces as f64 * 100.0;
        println!("  {field:34} {value:34} {count:>7}  {share:5.1}%");
    }

    println!();
    println!("  legacy lattice census against the formal resolution");
    println!(
        "  {:>8} {:>9}  {:20} {:>10} {:>7}",
        "declared", "certified", "formal_resolution", "formal_rank", "faces"
    );
    let mut cross_rows: Vec<((String, String, String, String), usize)> =
        cross.into_iter().collect();
    cross_rows.sort_by(|a, b| a.0.cmp(&b.0));
    for ((declared, certified, resolution, rank), count) in &cross_rows {
        println!("  {declared:>8} {certified:>9}  {resolution:20} {rank:>10} {count:>7}");
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    let csv = args.iter().any(|a| a == "--csv");
    let ledger = args.iter().any(|a| a == "--ledger");
    if let Some(position) = args.iter().position(|a| a == "--ambient-report") {
        let Some(path) = args.get(position + 1) else {
            anyhow::bail!("usage: face_census --ambient-report PROBE.log");
        };
        return ambient_report(path);
    }
    if let Some(position) = args.iter().position(|a| a == "--evidence-report") {
        let Some(path) = args.get(position + 1) else {
            anyhow::bail!("usage: face_census --evidence-report PROBE.log");
        };
        return evidence_report(path);
    }
    if let Some(position) = args.iter().position(|a| a == "--holes-report") {
        let Some(path) = args.get(position + 1) else {
            anyhow::bail!("usage: face_census --holes-report PROBE.log");
        };
        return holes_report(path);
    }
    let models: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    if models.is_empty() {
        eprintln!(
            "usage: face_census [--csv] [--ledger] MODEL.step [MORE.step ...]\n       \
             face_census --evidence-report PROBE.log\n       \
             face_census --ambient-report PROBE.log
                    face_census --holes-report PROBE.log"
        );
        return Ok(());
    }

    let mut overall = Census::new();
    let mut diag_rows: Vec<FaceDiagRow> = Vec::new();
    for path in &models {
        let table = load(path)?;
        census(&table, &mut overall, ledger, path, &mut diag_rows);
    }

    // DIAG-001: when TRUCK_FACE_DIAG_JSONL is set, sort and write one JSONL
    // row per failed face. Deterministic ordering independent of thread
    // completion order: sort by model_id, source_face_id, terminal_reason.
    if let Some(out_path) = env::var_os("TRUCK_FACE_DIAG_JSONL") {
        diag_rows.sort_by(|a, b| {
            a.diagnosis
                .model_id
                .cmp(&b.diagnosis.model_id)
                .then_with(|| {
                    a.diagnosis
                        .source_face_id
                        .cmp(&b.diagnosis.source_face_id)
                })
                .then_with(|| {
                    format!("{:?}", a.diagnosis.terminal_reason)
                        .cmp(&format!("{:?}", b.diagnosis.terminal_reason))
                })
        });
        use std::io::Write;
        let mut file = std::fs::File::create(&out_path)?;
        for row in &diag_rows {
            serde_json::to_writer(&mut file, row)?;
            writeln!(file)?;
        }
        // Also write a small meta file for the aggregator's reconciliation.
        if let Some(meta_path) = out_path.to_str().map(|s| format!("{s}.meta.json")) {
            let meta = serde_json::json!({
                "declared": overall.declared,
                "rendered": overall.rendered,
                "failed": overall.lost(),
                "models": models.len(),
            });
            std::fs::write(&meta_path, serde_json::to_string_pretty(&meta)?)?;
        }
        eprintln!(
            "DIAG-001: wrote {} diagnostic rows to {}",
            diag_rows.len(),
            out_path.to_string_lossy()
        );
    }

    let mut rows: Vec<(Bucket, usize)> = overall.counts.clone().into_iter().collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    if csv {
        println!("stage,reason,surface,count,share_pct,examples");
        for (bucket, count) in &rows {
            let share = *count as f64 / overall.declared as f64 * 100.0;
            let ex = overall
                .examples
                .get(bucket)
                .map(|v| v.join(" "))
                .unwrap_or_default();
            println!(
                "{},{},{},{},{:.1},{}",
                bucket.stage, bucket.reason, bucket.surface_kind, count, share, ex
            );
        }
    } else {
        println!(
            "{} models, {} faces declared, {} rendered, {} lost ({:.2}%)\n",
            models.len(),
            overall.declared,
            overall.rendered,
            overall.lost(),
            overall.lost() as f64 / overall.declared as f64 * 100.0
        );
        println!(
            "  {:11} {:28} {:10} {:5}   {:5}  examples",
            "stage", "reason", "surface", "count", "share"
        );
        for (bucket, count) in &rows {
            let share = *count as f64 / overall.declared as f64 * 100.0;
            let ex = overall
                .examples
                .get(bucket)
                .map(|v| v.join(" "))
                .unwrap_or_default();
            println!(
                "  {:11} {:28} {:10} {:5}  {:4.1}%  {}",
                bucket.stage, bucket.reason, bucket.surface_kind, count, share, ex
            );
        }
    }

    Ok(())
}
