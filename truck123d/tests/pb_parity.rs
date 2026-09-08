//! PB-011 required Rust suite — the corpus-parity churn over the landed
//! certified funnel (`tests_required`: swept_carrier_booleans_route_to_certified_entry,
//! lifted_skip_rows_run_green, cutaway_partial_arc_revolve_coverage,
//! monocoque_row_lifts_green).
//!
//! PB-011 routes the facade's swept-carrier boolean rows into the landed
//! certified entry (audit G1): the facade's `Mode` row is the `boolean_op`
//! encoding, and a `Mode` row over a swept carrier (spline/swept/revolved
//! carrier class, the CFP-004 stage contract) now dispatches the carrier pair
//! through the facade's certified-entry dispatch instead of the pair being
//! refused pre-execution. Accepted pairs certify in-envelope and are recorded
//! on the facade report as routed swept-carrier boolean rows; a pair coupling
//! a swept carrier with a funnel-refused carrier class (torus) keeps the
//! typed, localized refusal. The loop-side facade cannot name the certified
//! crate (the dependency law PB-013/PB-014 record), so the certified outcome
//! is asserted at the report / refusal granularity this crate owns — exactly
//! as the landed PB-013/PB-014 suites assert live certification (in-envelope
//! report semantics). Canonical x canonical pairs are untouched (landed S1).
//!
//! PB-011B (the 2-D path wave) extends the lift test to the canonical-cutter
//! F1 rows (floor/diffuser/suspension x2/steering_rack/track rods x2/
//! corners x4/drs_actuator): the B lifted set is the SKIPS rows whose note
//! carries the `PB-011B LIFT EVIDENCE` marker, and each lifted row must run
//! green through the harness door AND reproduce the recorded OCC reference
//! facts within tolerance (a green-but-wrong build is a stop-and-file, never
//! a lift). The corner rows certify through the certified-entry dispatch
//! because their revolve carriers are spline/line-polyline profiles — no
//! revolved-circle (torus) carrier is in the op path, so no typed torus
//! refusal is recorded for them.
//!
//! The corpus-side lift tests run the lifted rows through the harness door
//! (`corpus/ttc/door.py`, OCC baseline regime — a fresh python process per
//! row) exactly as `compat::runner` runs canonical rows, asserting the row's
//! geometry facts + STL + report record (the lift evidence the skip discipline
//! keys on). They need the machine python and the genuine `build123d` package
//! the corpus is written against (installed on this machine, same as the
//! landed `ttc_harness` canonical-subset test).

#[path = "../compat/mod.rs"]
// This suite consumes the manifest + skips machinery only; the runner/
// reference/surface modules of the shared compat tree are exercised by
// ttc_harness.rs, so the dead-code lints are silenced here rather than
// replicated per item.
#[allow(dead_code)]
mod compat;

use std::path::{Path, PathBuf};
use std::process::Command;

use truck_base::evidence::{EnvelopeCase, Refusal};
use truck123d::{FacadeOp, FacadeReport, FacadeTable, ModeValue, run_facade};

/// The absolute path of the `corpus/ttc` directory (the harness corpus).
fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../corpus/ttc")
}

/// A scratch directory unique to this test process and call site (parallel
/// door runs must never share a scratch path).
fn scratch_dir(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let serial = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "pb_parity_{}_{}_{}",
        std::process::id(),
        serial,
        tag
    ))
}

/// The corpus-shaped spline carrier rows (S5 section authoring).
fn spline_carrier() -> FacadeOp {
    FacadeOp::Spline {
        points: vec![[0.0, 0.0, 0.0], [1.0, 2.0, 0.0], [0.0, 0.0, 3.0]],
        periodic: false,
    }
}

/// Runs a table to completion: `Ok(report)` or a typed refusal.
fn run(ops: Vec<FacadeOp>) -> Result<FacadeReport, Refusal> {
    run_facade(&FacadeTable { ops })
}

/// The routed swept-carrier boolean rows of a report as a JSON value (the
/// serialized report omits the field when no swept-carrier boolean row ran).
fn routed_boolean_rows(report: &FacadeReport) -> serde_json::Value {
    serde_json::to_value(report)
        .map(|value| {
            value
                .get("boolean_events")
                .cloned()
                .unwrap_or(serde_json::Value::Array(vec![]))
        })
        .expect("the facade report must serialize") // H-3: serde output of a fixed report
}

// ---------------------------------------------------------------------------
// Test 1 (G1 core): swept-carrier booleans route to the certified entry
// ---------------------------------------------------------------------------

/// PB-011 required test 1 (audit G1): a lofted/revolved carrier pair that
/// refused `NonCanonicalCarrier` pre-execution now reaches the certified
/// entry through the facade boundary and returns a certified verdict — or the
/// typed localized refusal — asserted WHICH, never a bare `Err`.
#[test]
fn swept_carrier_booleans_route_to_certified_entry() {
    // Live through the facade: the lofted-base x revolved-tool subtract pair
    // (surfaces.cut's target - tool over a lofted body and a revolved shell)
    // runs in-envelope, the routed swept-carrier boolean row rides the
    // deterministic report (mode + carrier-class pair), and STL exports.
    let cut_table = vec![
        FacadeOp::PushPart,
        spline_carrier(),
        FacadeOp::Loft,
        FacadeOp::Mode {
            value: ModeValue::Subtract,
        },
        spline_carrier(),
        FacadeOp::Revolve { angle_deg: 360.0 },
        FacadeOp::ExportStl {
            path: "cut_loft.stl".to_string(),
        },
        FacadeOp::Pop,
    ];
    let report = run(cut_table)
        .expect("a swept-carrier boolean routes into the certified entry in-envelope"); // H-3: the certified verdict is the assertion
    assert!(report.constructive, "a swept-carrier part is constructive");
    let rows = routed_boolean_rows(&report);
    let routed = rows
        .as_array()
        .expect("routed boolean rows serialize as an array"); // H-3: machine-produced report shape
    assert_eq!(
        routed.len(),
        1,
        "the lofted-base x revolved-tool pair is recorded as one routed swept-carrier boolean row"
    );
    let first = routed
        .first()
        .and_then(serde_json::Value::as_object)
        .expect("the routed row serializes as an object"); // H-3: machine-produced report shape
    assert_eq!(
        first.get("mode").and_then(serde_json::Value::as_str),
        Some("subtract"),
        "the routed row carries the subtract mode"
    );
    assert_eq!(
        first.get("base").and_then(serde_json::Value::as_str),
        Some("swept"),
        "the routed row carries the lofted base carrier class"
    );
    assert_eq!(
        first.get("tool").and_then(serde_json::Value::as_str),
        Some("revolved"),
        "the routed row carries the revolved tool carrier class"
    );
    assert_eq!(report.exports.len(), 1);
    assert_eq!(report.exports[0].op, "export_stl");

    // The funnel-refused class refuses typed through the facade too: a torus
    // tool subtracted from a lofted carrier is `ContactReductionDeferred`
    // (the typed, localized refusal), never a bare failure and never `Ok`.
    let torus_table = vec![
        FacadeOp::PushPart,
        spline_carrier(),
        FacadeOp::Loft,
        FacadeOp::Mode {
            value: ModeValue::Subtract,
        },
        FacadeOp::Torus {
            major_radius: 2.0,
            minor_radius: 0.5,
        },
    ];
    let refusal = run(torus_table).expect_err("a swept x torus boolean must refuse typed"); // H-3: the typed refusal is the assertion
    assert!(
        matches!(
            refusal,
            Refusal::UnsupportedEnvelope(EnvelopeCase::ContactReductionDeferred)
        ),
        "expected the typed localized ContactReductionDeferred refusal, got {refusal:?}"
    );

    // A canonical-carrier tool under a subtract mode over a swept base routes
    // too (the corpus's swept x canonical form, e.g. cutting a canonical
    // cylinder through a lofted shell): in-envelope, one routed row.
    let swept_canonical = vec![
        FacadeOp::PushPart,
        spline_carrier(),
        FacadeOp::Loft,
        FacadeOp::Mode {
            value: ModeValue::Subtract,
        },
        FacadeOp::Cylinder {
            radius: 1.0,
            height: 2.0,
        },
        FacadeOp::ExportStl {
            path: "cut_cylinder.stl".to_string(),
        },
        FacadeOp::Pop,
    ];
    let report = run(swept_canonical)
        .expect("a swept x canonical boolean routes into the certified entry in-envelope"); // H-3: the certified verdict is the assertion
    assert_eq!(
        routed_boolean_rows(&report)
            .as_array()
            .map(|rows| rows.len())
            .unwrap_or(0),
        1,
        "the swept x canonical pair is recorded as one routed row"
    );

    // Controls: canonical x canonical boolean and the TR-NRB-001 STEP boundary
    // are untouched by the routing (no routed row for a canonical pair; the
    // STEP-out refusal of a swept product still carries the NonCanonicalCarrier
    // envelope case).
    let canonical = vec![
        FacadeOp::PushPart,
        FacadeOp::Box {
            length: 2.0,
            width: 3.0,
            height: 1.0,
        },
        FacadeOp::Mode {
            value: ModeValue::Subtract,
        },
        FacadeOp::Box {
            length: 1.0,
            width: 1.0,
            height: 1.0,
        },
        FacadeOp::ExportStep {
            path: "canonical.step".to_string(),
        },
        FacadeOp::Pop,
    ];
    let report = run(canonical).expect("canonical x canonical boolean STEP is in envelope"); // H-3: the landed S1 control
    assert!(
        routed_boolean_rows(&report)
            .as_array()
            .is_none_or(|rows| rows.is_empty())
    );
    assert!(!report.constructive);

    let swept_step = vec![
        FacadeOp::PushPart,
        spline_carrier(),
        FacadeOp::Loft,
        FacadeOp::ExportStep {
            path: "swept.step".to_string(),
        },
        FacadeOp::Pop,
    ];
    let step_refusal = run(swept_step).expect_err("STEP out of a swept part must refuse typed"); // H-3: TR-NRB-001 control
    assert!(
        matches!(
            step_refusal,
            Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)
        ),
        "TR-NRB-001 STEP-out of constructive geometry must stay typed, got {step_refusal:?}"
    );
}

// ---------------------------------------------------------------------------
// Test 2 (corpus lift): every lifted skip row runs green through the door
// ---------------------------------------------------------------------------

/// Spawns the harness door for one corpus row and returns its stdout JSON
/// record (the runner's `run_one` shape, replicated here so a lifted row —
/// not yet on the canonical runner set — can be proven green).
fn door_run(
    corpus: &Path,
    row: &compat::manifest::ManifestRow,
    stl_path: &Path,
) -> serde_json::Value {
    let tree = corpus.join(&row.tree);
    let args_json = serde_json::to_string(&row.args).expect("the manifest args serialize"); // H-3: machine-produced manifest data
    let output = Command::new("python")
        .arg(corpus.join("door.py"))
        .arg(&tree)
        .arg(&row.module)
        .arg(&row.entry)
        .arg(&args_json)
        .arg(stl_path)
        .output()
        .unwrap_or_else(|e| panic!("cannot spawn the door for {}: {e}", row.id));
    assert!(
        output.status.success(),
        "the door process must exit cleanly for {}:\n{}",
        row.id,
        String::from_utf8_lossy(&output.stderr)
    );
    let record: serde_json::Value = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|e| panic!("the door must emit a JSON record for {}: {e}", row.id));
    assert!(
        record
            .get("ok")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        "the door run of {} must be ok: {}",
        row.id,
        record.get("error").cloned().unwrap_or_default()
    );
    record
}

/// Asserts a green door-run evidence triple for one row: geometry facts
/// (solid_count/volume/bbox) + a non-empty binary STL (the report record is
/// the stdout JSON itself).
fn assert_green_door_evidence(corpus: &Path, row: &compat::manifest::ManifestRow) {
    let out_dir = scratch_dir(&format!("lift_{}", row.id.replace('/', "_")));
    std::fs::create_dir_all(&out_dir)
        .unwrap_or_else(|e| panic!("cannot create scratch dir {}: {e}", out_dir.display()));
    let stl_path = out_dir.join(format!("{}.stl", row.id.replace('/', "__")));
    let record = door_run(corpus, row, &stl_path);

    let facts = record
        .get("facts")
        .and_then(serde_json::Value::as_object)
        .unwrap_or_else(|| panic!("row {} must carry geometry facts", row.id));
    assert!(
        facts
            .get("solid_count")
            .and_then(serde_json::Value::as_i64)
            .is_some_and(|n| n > 0),
        "row {} must report a positive solid count",
        row.id
    );
    assert!(
        facts
            .get("volume")
            .and_then(serde_json::Value::as_f64)
            .is_some(),
        "row {} must report a volume fact",
        row.id
    );
    assert!(
        facts
            .get("bbox")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|corners| corners.len() == 2),
        "row {} must report a two-corner bounding box",
        row.id
    );
    let triangles = record
        .get("stl")
        .and_then(|stl| stl.get("triangles"))
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0);
    assert!(triangles > 0, "row {} STL must carry triangles", row.id);
    let stl_len = std::fs::metadata(&stl_path)
        .map(|m| m.len())
        .unwrap_or_else(|e| panic!("row {} must produce an STL file: {e}", row.id));
    assert!(
        stl_len > 84,
        "row {} STL is not a valid binary STL ({} bytes)",
        row.id,
        stl_len
    );
    assert_facts_match_recorded_reference(corpus, row, &parse_run_facts(&record, &row.id));
    let _ = std::fs::remove_dir_all(&out_dir);
}

/// Parses a door record's geometry facts into the typed runner facts.
fn parse_run_facts(record: &serde_json::Value, row_id: &str) -> compat::runner::RunFacts {
    let facts = record
        .get("facts")
        .and_then(serde_json::Value::as_object)
        .unwrap_or_else(|| panic!("row {row_id} must carry geometry facts"));
    let solid_count = facts
        .get("solid_count")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or_else(|| panic!("row {row_id} solid_count must be an integer"));
    let volume = facts
        .get("volume")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or_else(|| panic!("row {row_id} volume must be a number"));
    let corners = facts
        .get("bbox")
        .and_then(serde_json::Value::as_array)
        .unwrap_or_else(|| panic!("row {row_id} must carry a two-corner bbox"));
    let mut bbox = [[0.0f64; 3]; 2];
    for (corner_idx, corner) in corners.iter().enumerate() {
        let coords = corner
            .as_array()
            .unwrap_or_else(|| panic!("row {row_id} bbox corner {corner_idx} must be an array"));
        for (axis, value) in coords.iter().enumerate() {
            bbox[corner_idx][axis] = value
                .as_f64()
                .unwrap_or_else(|| panic!("row {row_id} bbox coordinate must be a number"));
        }
    }
    compat::runner::RunFacts {
        solid_count,
        volume,
        bbox,
    }
}

/// The recorded-reference file for a row id (`<base>.json` under
/// `corpus/ttc/reference/`, keyed by the row-id basename). The staged rows
/// carry no manifest `reference` field until the orchestrator's manifest
/// movement, so the id basename keys the recorded file.
fn recorded_reference_path(
    corpus: &Path,
    row: &compat::manifest::ManifestRow,
) -> std::path::PathBuf {
    let base = row
        .id
        .rsplit('/')
        .next()
        .unwrap_or_else(|| panic!("row id {} must carry a slash", row.id));
    corpus.join("reference").join(format!("{base}.json"))
}

/// Asserts a green run's facts match the row's recorded OCC reference within
/// its recorded tolerances (the facts-vs-reference gate a lift keys on: a
/// build that is green but wrong geometry is a stop-and-file, never a lift).
///
/// The staged-row references carry the door's full facts record (including
/// `diag`, which the strict `ReferenceFile` deserialization rejects), so the
/// recorded values are read from the JSON value into the typed structs first
/// and compared with [`compat::reference::compare_facts`].
fn assert_facts_match_recorded_reference(
    corpus: &Path,
    row: &compat::manifest::ManifestRow,
    run: &compat::runner::RunFacts,
) {
    let path = recorded_reference_path(corpus, row);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "row {} has no recorded reference {}: {e}",
            row.id,
            path.display()
        )
    });
    let value: serde_json::Value = serde_json::from_str(&text).unwrap_or_else(|e| {
        panic!(
            "row {} recorded reference {} is invalid: {e}",
            row.id,
            path.display()
        )
    });
    let reference = reference_from_value(&value).unwrap_or_else(|e| {
        panic!(
            "row {} recorded reference {} is not usable: {e}",
            row.id,
            path.display()
        )
    });
    compat::reference::compare_facts(&compat::runner::as_geometry_facts(run), &reference)
        .unwrap_or_else(|e| {
            panic!(
                "row {} door-run facts differ from the recorded reference: {e}",
                row.id
            )
        });
}

/// Builds a typed reference file from a reference JSON value, reading only the
/// fields the comparison needs (the recorded value may carry extra fields the
/// door emitted, e.g. `diag`, that the strict struct deserialization rejects).
fn reference_from_value(
    value: &serde_json::Value,
) -> Result<compat::reference::ReferenceFile, String> {
    let obj = value
        .as_object()
        .ok_or_else(|| "reference is not an object".to_string())?;
    let get = |key: &str| -> Result<&serde_json::Value, String> {
        obj.get(key)
            .ok_or_else(|| format!("reference has no {key}"))
    };
    let facts = get("facts")?
        .as_object()
        .ok_or_else(|| "reference facts is not an object".to_string())?;
    let fnum = |key: &str| -> Result<f64, String> {
        facts
            .get(key)
            .and_then(serde_json::Value::as_f64)
            .ok_or_else(|| format!("reference facts.{key} is not a number"))
    };
    let bbox_value = facts
        .get("bbox")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "reference facts.bbox is not an array".to_string())?;
    let mut bbox = [[0.0f64; 3]; 2];
    for (corner_idx, corner) in bbox_value.iter().enumerate() {
        let coords = corner
            .as_array()
            .ok_or_else(|| "reference bbox corner is not an array".to_string())?;
        for (axis, coord) in coords.iter().enumerate() {
            bbox[corner_idx][axis] = coord
                .as_f64()
                .ok_or_else(|| "reference bbox coordinate is not a number".to_string())?;
        }
    }
    let tolerances = get("tolerances")?
        .as_object()
        .ok_or_else(|| "reference tolerances is not an object".to_string())?;
    let tnum = |key: &str| -> Result<f64, String> {
        tolerances
            .get(key)
            .and_then(serde_json::Value::as_f64)
            .ok_or_else(|| format!("reference tolerances.{key} is not a number"))
    };
    Ok(compat::reference::ReferenceFile {
        schema: get("schema")
            .ok()
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string(),
        row_id: get("row_id")
            .ok()
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string(),
        door_version: get("door_version")
            .ok()
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string(),
        facts: compat::reference::GeometryFacts {
            solid_count: facts
                .get("solid_count")
                .and_then(serde_json::Value::as_i64)
                .ok_or_else(|| "reference facts.solid_count is not an integer".to_string())?,
            volume: fnum("volume")?,
            bbox,
        },
        tolerances: compat::reference::Tolerances {
            solid_count: tolerances
                .get("solid_count")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string(),
            volume_rel: tnum("volume_rel")?,
            volume_abs: tnum("volume_abs")?,
            bbox_abs: tnum("bbox_abs")?,
        },
    })
}

/// PB-011 required test 2 (PB-011B lift set): every row this packet lifts
/// from `corpus/ttc/SKIPS.json` runs green through the harness door with
/// geometry facts + report record + STL, and the green door-run facts match
/// the row's recorded OCC reference within its recorded tolerances. The B
/// (canonical-cutter, 2-D path) lifted set is read from the machine-checked
/// evidence this packet wrote into the SKIPS rows (`PB-011B LIFT EVIDENCE`
/// note marker); an empty B lifted set fails the test — lifting is the point.
#[test]
fn lifted_skip_rows_run_green() {
    let corpus = corpus_dir();
    let manifest = compat::manifest::load_manifest(&corpus).expect("the corpus manifest must load"); // H-3: validated corpus input, read-only
    let skips =
        compat::skips::load_skips(&corpus, &manifest).expect("the skips file must validate"); // H-3: validated corpus input, read-only

    let lifted: Vec<String> = skips
        .rows
        .iter()
        .filter(|skip| skip.note.contains("PB-011B LIFT EVIDENCE"))
        .map(|skip| skip.id.clone())
        .collect();
    assert!(
        !lifted.is_empty(),
        "the B (canonical-cutter) lifted set must be non-empty — lifting is the point of PB-011B"
    );

    for id in &lifted {
        let row = manifest
            .rows
            .iter()
            .find(|row| &row.id == id)
            .unwrap_or_else(|| panic!("lifted row {id} must be enrolled on the manifest")); // H-3: skips/manifest 1:1 is machine-checked
        assert_green_door_evidence(&corpus, row);
    }
}

// ---------------------------------------------------------------------------
// Test 3 (G5 cutaway): the partial-arc revolve path coverage
// ---------------------------------------------------------------------------

/// PB-011 required test 3 (audit G5, decision 4): the falcon cutaway's
/// partial-arc revolve path — the sectioned center-core tank barrel and
/// interstage shells driven by `arc`/`start` on `eng.revolved_solid`/`shell`
/// over `CUT_START=135`/`CUT_ARC=270` — runs through the harness door green on
/// top of PB-014's partial-arc revolve capability. The coverage claim is
/// asserted against the vendored code path, then executed.
#[test]
fn cutaway_partial_arc_revolve_coverage() {
    let corpus = corpus_dir();
    let manifest = compat::manifest::load_manifest(&corpus).expect("the corpus manifest must load"); // H-3: validated corpus input, read-only
    let cutaway = manifest
        .rows
        .iter()
        .find(|row| row.id == "falcon_heavy/cutaway")
        .expect("the cutaway row must be enrolled on the manifest"); // H-3: manifest row set is fixed
    assert_eq!(cutaway.entry, "build_vehicle");
    assert_eq!(
        cutaway.args,
        vec![serde_json::Value::Bool(true)],
        "the cutaway row runs build_vehicle(cutaway=True)"
    );

    // Coverage: the vendored code path is partial-arc revolves (revolution_arc
    // + start), not boolean sectioning. Assert the driving constants and the
    // cutaway arc/start route exist in the vendored lib module.
    let falcon_common = corpus.join("trees/falcon_heavy/src/lib/falcon_common.py");
    let source = std::fs::read_to_string(&falcon_common)
        .unwrap_or_else(|e| panic!("the vendored falcon_common.py must be readable: {e}")); // H-3: read-only vendored corpus
    assert!(
        source.contains("CUT_START, CUT_ARC = 135.0, 270.0"),
        "the cutaway opening constants must drive the partial-arc revolves"
    );
    assert!(
        source.contains("arc, start = (CUT_ARC, CUT_START) if cutaway else (360.0, 0.0)"),
        "build_vehicle(cutaway=True) must select the partial arc"
    );
    assert!(
        source.contains("revolved_solid("),
        "the sectioned barrel must be built by the partial-arc revolve helper"
    );
    assert!(
        source.contains("sectioned=cutaway"),
        "the cutaway flag must reach the sectioned tank/interstage builders"
    );

    // The row's skip note records the same classification (PB-014 corrected
    // the mis-description; PB-011 records the lift evidence).
    let skips =
        compat::skips::load_skips(&corpus, &manifest).expect("the skips file must validate"); // H-3: validated corpus input, read-only
    let cutaway_skip = skips
        .rows
        .iter()
        .find(|skip| skip.id == "falcon_heavy/cutaway")
        .expect("the cutaway skip row must exist"); // H-3: skips/manifest 1:1 is machine-checked
    assert!(
        cutaway_skip.note.contains("partial-arc revolves")
            && cutaway_skip.note.contains("PB-011 LIFT EVIDENCE"),
        "the cutaway skip note must record the partial-arc revolve path and the PB-011 lift evidence"
    );

    // Executed: build_vehicle(cutaway=True) runs green through the harness
    // door with geometry facts + STL (the partial-arc revolve coverage run).
    assert_green_door_evidence(&corpus, cutaway);
}

// ---------------------------------------------------------------------------
// Test 4 (monocoque lift): the f1/monocoque row lifts with facts matching the
// recorded reference
// ---------------------------------------------------------------------------

/// Loads the recorded reference for a skipped-but-lifted row by id (the
/// manifest row carries no `reference` field while it is still staged, so the
/// reference file is addressed directly — same file the orchestrator's
/// corpus-output step will attach when the row moves to the runnable set).
fn load_skip_reference(corpus: &Path, id: &str) -> serde_json::Value {
    let name = id.split('/').next_back().unwrap_or(id); // H-3: row ids are `<family>/<part>`
    let path = corpus.join("reference").join(format!("{name}.json"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("the recorded reference for {id} must exist: {e}")); // H-3: orchestrator-recorded reference
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("the recorded reference for {id} must parse: {e}")) // H-3: orchestrator-recorded reference
}

/// Asserts a door run's geometry facts match the recorded reference within the
/// recorded tolerances (solid_count exact; volume relative/absolute; bbox
/// absolute) — the lift evidence for a skipped-but-lifted row.
fn assert_facts_match_reference(
    corpus: &Path,
    row: &compat::manifest::ManifestRow,
    record: &serde_json::Value,
) {
    let reference = load_skip_reference(corpus, &row.id);
    let expected_facts = reference
        .get("facts")
        .and_then(serde_json::Value::as_object)
        .unwrap_or_else(|| panic!("reference {} must carry facts", row.id)); // H-3: machine-recorded reference shape
    let tolerances = reference
        .get("tolerances")
        .and_then(serde_json::Value::as_object)
        .unwrap_or_else(|| panic!("reference {} must carry tolerances", row.id)); // H-3: machine-recorded reference shape
    let facts = record
        .get("facts")
        .and_then(serde_json::Value::as_object)
        .unwrap_or_else(|| panic!("row {} must carry geometry facts", row.id)); // H-3: the door emits the record

    let got_solids = facts
        .get("solid_count")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or_else(|| panic!("row {} solid_count is missing", row.id)); // H-3: the door emits the record
    let want_solids = expected_facts
        .get("solid_count")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or_else(|| panic!("reference {} solid_count is missing", row.id)); // H-3: machine-recorded reference shape
    assert_eq!(
        got_solids, want_solids,
        "row {} solid_count must match the recorded reference",
        row.id
    );

    let got_volume = facts
        .get("volume")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or_else(|| panic!("row {} volume is missing", row.id)); // H-3: the door emits the record
    let want_volume = expected_facts
        .get("volume")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or_else(|| panic!("reference {} volume is missing", row.id)); // H-3: machine-recorded reference shape
    let volume_rel = tolerances
        .get("volume_rel")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let volume_abs = tolerances
        .get("volume_abs")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let volume_ok = if want_volume == 0.0 {
        (got_volume - want_volume).abs() <= volume_abs
    } else {
        ((got_volume - want_volume) / want_volume).abs() <= volume_rel
    };
    assert!(
        volume_ok,
        "row {} volume {got_volume} must match recorded reference {want_volume} within tolerance",
        row.id
    );

    let bbox_abs = tolerances
        .get("bbox_abs")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let got_bbox = facts
        .get("bbox")
        .and_then(serde_json::Value::as_array)
        .unwrap_or_else(|| panic!("row {} bbox is missing", row.id)); // H-3: the door emits the record
    let want_bbox = expected_facts
        .get("bbox")
        .and_then(serde_json::Value::as_array)
        .unwrap_or_else(|| panic!("reference {} bbox is missing", row.id)); // H-3: machine-recorded reference shape
    for corner in 0..2 {
        for axis in 0..3 {
            let want = want_bbox
                .get(corner)
                .and_then(serde_json::Value::as_array)
                .and_then(|c| c.get(axis))
                .and_then(serde_json::Value::as_f64)
                .unwrap_or_else(|| panic!("reference {} bbox coordinate is missing", row.id)); // H-3: machine-recorded reference shape
            let got = got_bbox
                .get(corner)
                .and_then(serde_json::Value::as_array)
                .and_then(|c| c.get(axis))
                .and_then(serde_json::Value::as_f64)
                .unwrap_or_else(|| panic!("row {} bbox coordinate is missing", row.id)); // H-3: the door emits the record
            assert!(
                (got - want).abs() <= bbox_abs,
                "row {} bbox corner {corner} axis {axis} = {got} must match reference {want} within tolerance",
                row.id
            );
        }
    }
}

/// PB-011 required test 4 (monocoque lift): the `f1/monocoque` row is in the
/// lifted set (its SKIPS note carries the `PB-011 LIFT EVIDENCE` marker this
/// packet wrote) and runs green through the harness door with geometry facts
/// matching the recorded reference (`corpus/ttc/reference/monocoque.json`).
/// An empty lifted set fails the test — lifting is the point.
#[test]
fn monocoque_row_lifts_green() {
    let corpus = corpus_dir();
    let manifest = compat::manifest::load_manifest(&corpus).expect("the corpus manifest must load"); // H-3: validated corpus input, read-only
    let skips =
        compat::skips::load_skips(&corpus, &manifest).expect("the skips file must validate"); // H-3: validated corpus input, read-only

    let lifted: Vec<String> = skips
        .rows
        .iter()
        .filter(|skip| skip.note.contains("PB-011 LIFT EVIDENCE"))
        .map(|skip| skip.id.clone())
        .collect();
    assert!(
        !lifted.is_empty(),
        "the lifted set must be non-empty — lifting is the point of PB-011"
    );
    assert!(
        lifted.iter().any(|id| id == "f1/monocoque"),
        "the monocoque row must be in the lifted set"
    );

    let monocoque = manifest
        .rows
        .iter()
        .find(|row| row.id == "f1/monocoque")
        .expect("the monocoque row must be enrolled on the manifest"); // H-3: skips/manifest 1:1 is machine-checked

    let out_dir = scratch_dir("monocoque_lift");
    std::fs::create_dir_all(&out_dir)
        .unwrap_or_else(|e| panic!("cannot create scratch dir {}: {e}", out_dir.display()));
    let stl_path = out_dir.join("f1__monocoque.stl");
    let record = door_run(&corpus, monocoque, &stl_path);

    assert_facts_match_reference(&corpus, monocoque, &record);

    let triangles = record
        .get("stl")
        .and_then(|stl| stl.get("triangles"))
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0);
    assert!(triangles > 0, "the monocoque STL must carry triangles");
    let stl_len = std::fs::metadata(&stl_path)
        .map(|m| m.len())
        .unwrap_or_else(|e| panic!("the monocoque run must produce an STL file: {e}"));
    assert!(
        stl_len > 84,
        "the monocoque STL is not a valid binary STL ({} bytes)",
        stl_len
    );
    let _ = std::fs::remove_dir_all(&out_dir);
}
