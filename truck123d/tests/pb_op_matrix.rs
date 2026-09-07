//! PB-013 required Rust suite (four tests) over the op capability matrix:
//! the PB-010 parity audit's 93 op census rows turned into one annotated cell
//! per (op x carrier-class) pair, with every cell asserted against LIVE
//! facade/compat behavior. PB-014-AUTHORING-REVOLVE amends the suite: cells
//! 10 and 19 (the partial-arc revolve and the closed-circle authoring rows)
//! flip to `certified` (the two authoring prerequisites the audit's G5/G3
//! findings named), and the three authoring tests the packet requires are
//! added (`circle_profile_authors_and_feeds_revolve`,
//! `partial_arc_revolve_builds_capped_shell`, `matrix_cells_flipped_certified`).
//!
//! The doc (`docs/OP_CAPABILITY_MATRIX.md`) and this file share one cell
//! table: the fixed-order `CELLS` below is canonical, the doc table is its
//! mirror, and the tests parse BOTH the census JSON (`loop/results/
//! PB-010-TTC-PARITY-AUDIT.json`, normative) and the doc to prove:
//!
//! 1. every census row maps to exactly one cell, and the doc lists exactly
//!    that mapping in this order (`matrix_doc_rows_match_live_semantics`);
//! 2. the G1 class is pinned: every swept-carrier boolean cell asserts its
//!    typed refusal with the `NonCanonicalCarrier` envelope case today
//!    (`swept_carrier_boolean_cells_refuse_typed_today`);
//! 3. the authoring cells (spline, polyline/line-loop, circle) match the
//!    S5/S6 surface rows; PB-014 flips the circle row (audit G3) to
//!    `certified` as the landed S5 closed-circle carrier authoring entry
//!    (`authoring_cells_match_surface_rows`);
//! 4. the `revolve(partial-arc,spline-profile)` cell is annotated per the
//!    audit's G5 finding and flipped to `certified` by PB-014
//!    (`partial_arc_revolve_cell_annotated`).
//!
//! Live driver semantics (what "driving live behavior" means per verdict):
//!
//! * a `certified` cell's facade/compat table probe runs in-envelope (Ok)
//!   with the census-recorded report shape — STL out for constructive parts,
//!   STEP in-envelope for canonical booleans, assembly emission Ok for
//!   compound/grouping;
//! * a `refuses(NonCanonicalCarrier)` cell's probe is the constructive/swept
//!   product crossing the STEP boundary the facade enforces today (TR-NRB-001:
//!   STL/GLB never STEP for swept carriers) and asserts the typed refusal
//!   including the envelope case;
//! * a `client-layer` cell is a data row: no facade op row exists for it and
//!   (for name/color/placement) the emission layer carries the record as node
//!   data;
//! * an `unavailable` cell has no landed facade/compat row or no enrolled row
//!   exercises it today (checked against the facade op vocabulary and the
//!   census stage).
//!
//! H-1 applies (no unwrap/expect without a justified same-line opt-out; H-3
//! marks them). Determinism: fixed cell order, no hash iteration. The census
//! and the doc are read-only inputs; the facade/compat surface is observed,
//! never changed.

use std::path::PathBuf;

use truck_base::evidence::{EnvelopeCase, Refusal};
use truck123d::{
    AssemblyPart, FacadeOp, FacadeReport, FacadeTable, GlbMesh, GlbNodePayload, ModeValue,
    SolidHandle, SrgbColor, assembly_occurrence_labels, emit_glb, emit_step_assembly, run_facade,
};

/// One canonical matrix cell: id, verdicts and its census row ids, in doc
/// order. This const and `docs/OP_CAPABILITY_MATRIX.md` must agree field for
/// field (asserted by the tests).
type CellSpec = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static [&'static str],
);

/// The fixed-order canonical cell table (PB-013, derived from the census
/// `op` fields; every census row maps to exactly one cell).
#[rustfmt::skip]
const CELLS: &[CellSpec] = &[
    ("fuse(swept,swept)", "refuses(NonCanonicalCarrier)", "PB-011", "certified", "S1", &["f1/src/lib/surfaces.py:fuse_all", "f1/src/lib/engine_cover.py:fuse_cover", "f1/src/lib/engine_cover.py:build_airbox_fuse", "f1/src/lib/details.py:fuse_pod", "f1/src/lib/drivetrain.py:_fuse", "f1/src/lib/power_unit.py:fuse_pu"]),
    ("fuse(swept,canonical)", "refuses(NonCanonicalCarrier)", "PB-011", "certified", "S1", &["f1/src/lib/suspension.py:_fuse", "f1/src/lib/mono_tub.py:fuse_proud"]),
    ("cut(swept,swept)", "refuses(NonCanonicalCarrier)", "PB-011", "certified", "S1", &["f1/src/lib/engine_cover.py:cut_cover", "f1/src/lib/rear_wing.py:cut_endplate", "f1/src/lib/details.py:cut_mirror", "f1/src/lib/mono_tub.py:cut_tub"]),
    ("cut(revolved,canonical)", "refuses(NonCanonicalCarrier)", "PB-011", "certified", "S1", &["f1/src/lib/wheels.py:_cut"]),
    ("cut(swept,canonical)", "refuses(NonCanonicalCarrier)", "PB-011", "certified", "S1", &["f1/src/lib/surfaces.py:cut", "f1/src/lib/rear_wing.py:boolean_crank", "f1/src/lib/suspension.py:_cut", "f1/src/lib/floor.py:cut_floor", "f1/src/lib/drivetrain.py:_cut", "f1/src/lib/engine_cover.py:louvres_cut", "f1/src/lib/details.py:dzus_cut", "f1/src/lib/drivetrain.py:gearbox_cut"]),
    ("intersect(swept,canonical)", "refuses(NonCanonicalCarrier)", "PB-011", "certified", "S1", &["f1/src/lib/surfaces.py:accent_stripe_solid", "f1/src/lib/drivetrain.py:intersect", "f1/src/lib/front_wing.py:accent_stripe"]),
    ("cut(canonical,canonical)", "certified", "-", "certified", "S1", &["f1/src/lib/drivetrain.py:clevis_bore", "f1/src/lib/rear_wing.py:pylon_cut"]),
    ("heal(boolean-result)", "unavailable", "-", "unavailable", "S1", &["f1/src/lib/surfaces.py:repair"]),
    ("revolve(full,spline-profile)", "certified", "-", "certified", "S6", &["falcon_heavy/src/lib/merlin_common.py:revolved_shell", "falcon_heavy/src/lib/merlin_common.py:revolved_solid", "falcon_heavy/src/lib/falcon_common.py:_tube_z", "falcon_heavy/src/lib/falcon_common.py:_dome", "falcon_heavy/src/lib/falcon_common.py:make_mvac_revolve", "falcon_heavy/src/lib/falcon_common.py:make_fairing_shell", "f1/src/lib/wheels.py:revolve_tyre"]),
    ("revolve(partial-arc,spline-profile)", "certified", "PB-014", "certified", "S6", &["falcon_heavy/src/lib/falcon_common.py:revolved_solid_barrel"]),
    ("sweep(spine,closed-section)", "certified", "-", "certified", "S6", &["falcon_heavy/src/lib/merlin_common.py:tube"]),
    ("loft(spline-section)", "certified", "-", "certified", "S5", &["f1/src/lib/surfaces.py:loft_solid", "f1/src/lib/surfaces.py:body_loft", "f1/src/lib/wheels.py:_loft", "f1/src/lib/floor.py:loft_stack", "f1/src/lib/mono_tub.py:loft_half_section", "f1/src/lib/power_unit.py:loft_tube", "f1/src/lib/mono_halo.py:loft_loop", "f1/src/lib/cockpit.py:ruled_loft", "f1/src/lib/front_wing.py:loft_cascade", "f1/src/lib/nose.py:loft_nose", "f1/src/lib/sidepods.py:loft_skin", "f1/src/lib/sidepods.py:cavity_solid", "f1/src/lib/floor.py:diffuser_loft"]),
    ("extrude(profile)", "certified", "-", "certified", "S6", &["falcon_heavy/src/lib/merlin_common.py:extrude", "f1/src/lib/engine_cover.py:extrude"]),
    ("fillet(edge-selector)", "certified", "-", "certified", "S6", &["f1/src/lib/surfaces.py:safe_fillet", "f1/src/lib/rear_wing.py:fillet_select"]),
    ("chamfer(edge-selector)", "certified", "-", "certified", "S6", &["f1/src/lib/surfaces.py:safe_chamfer"]),
    ("mirror(axis-plane)", "certified", "-", "certified", "S6", &["f1/src/lib/surfaces.py:mirror_y"]),
    ("author(spline-profile)", "certified", "-", "certified", "S5", &["falcon_heavy/src/lib/merlin_common.py:profile_face", "f1/src/lib/surfaces.py:airfoil_profile", "f1/src/lib/suspension.py:loft_face"]),
    ("author(polyline-profile)", "certified", "-", "certified", "S4", &[]),
    ("author(circle-profile)", "certified", "PB-014", "certified", "S5", &["f1/src/lib/cockpit.py:circle_section"]),
    ("validity-check", "client-layer", "-", "client-layer", "S7", &["f1/src/lib/surfaces.py:selector_census"]),
    ("primitive(canonical-solid)", "certified", "-", "certified", "S3", &["falcon_heavy/src/lib/merlin_common.py:cylinder", "falcon_heavy/src/lib/merlin_common.py:torus", "falcon_heavy/src/lib/merlin_common.py:sphere", "falcon_heavy/src/lib/merlin_common.py:box", "falcon_heavy/src/falcon_heavy_exploded.py:guide_cyl"]),
    ("compound(group)", "certified", "-", "certified", "S3", &["falcon_heavy/src/lib/merlin_common.py:group_compound", "falcon_heavy/src/lib/falcon_common.py:compound_from_instances", "falcon_heavy/src/falcon_heavy.py:compound_vehicle", "f1/src/lib/surfaces.py:as_body_compound", "f1/src/lib/drivetrain.py:compound_solids", "f1/src/lib/monocoque.py:group", "f1/src/f1.py:assembly_add", "f1/src/lib/cockpit.py:build_cockpit"]),
    ("label", "client-layer", "-", "client-layer", "S2", &["falcon_heavy/src/lib/merlin_common.py:label_color", "f1/src/lib/surfaces.py:styled", "falcon_heavy/src/lib/falcon_common.py:_lab", "falcon_heavy/src/lib/falcon_common.py:label_tank", "falcon_heavy/src/falcon_heavy.py:label_vehicle"]),
    ("color", "client-layer", "-", "client-layer", "S2", &["falcon_heavy/src/falcon_heavy_exploded.py:color", "f1/src/lib/spec.py:color_palette", "f1/src/lib/wheels.py:colour_tyre"]),
    ("placement", "client-layer", "-", "client-layer", "S2", &["falcon_heavy/src/lib/merlin_common.py:rotate", "falcon_heavy/src/falcon_heavy_exploded.py:moved_offset", "f1/src/lib/rear_wing.py:rotate_placement", "f1/src/lib/mono_halo.py:stud_placement", "f1/src/lib/cockpit.py:plane_offset", "f1/src/lib/floor.py:rotated_plane", "f1/src/lib/surfaces.py:section_plane"]),
    ("step-export(constructive)", "refuses(NonCanonicalCarrier)", "-", "refuses(NonCanonicalCarrier)", "none", &["falcon_heavy/src/falcon_heavy_cutaway.py:step_out", "f1/src/f1.py:step_out", "f1/src/airbox.py:dispatch", "f1/src/corner_fl.py:dispatch", "f1/src/monocoque.py:dispatch"]),
];

/// The census row ids that map to the G1 swept-carrier boolean cells: every
/// cell whose `current` verdict is `refuses(NonCanonicalCarrier)` on surface
/// S1. Pinned by `swept_carrier_boolean_cells_refuse_typed_today`.
const G1_CELLS: &[&str] = &[
    "fuse(swept,swept)",
    "fuse(swept,canonical)",
    "cut(swept,swept)",
    "cut(revolved,canonical)",
    "cut(swept,canonical)",
    "intersect(swept,canonical)",
];

/// The closed verdict vocabulary of the matrix (`docs/PY_BRIDGE_COMPAT_SURFACE.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    Certified,
    RefusesNonCanonicalCarrier,
    ClientLayer,
    Unavailable,
}

impl Verdict {
    /// Parses a doc verdict string; a string outside the closed vocabulary is
    /// a SPEC_GAP against the compat surface doc, surfaced here as `None`.
    fn parse(text: &str) -> Option<Verdict> {
        match text {
            "certified" => Some(Verdict::Certified),
            "refuses(NonCanonicalCarrier)" => Some(Verdict::RefusesNonCanonicalCarrier),
            "client-layer" => Some(Verdict::ClientLayer),
            "unavailable" => Some(Verdict::Unavailable),
            _ => None,
        }
    }
}

/// One census row of the parity audit (extra fields ignored).
#[derive(Debug, Clone)]
struct CensusRow {
    id: String,
    classification: String,
    surface: String,
}

/// The parsed census document.
struct Census {
    rows: Vec<CensusRow>,
}

/// The absolute path of the parity-audit census JSON.
fn census_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../loop/results/PB-010-TTC-PARITY-AUDIT.json")
}

/// The absolute path of the capability matrix doc.
fn matrix_doc_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../docs/OP_CAPABILITY_MATRIX.md")
}

/// Loads and parses the census JSON.
fn load_census() -> Census {
    let path = census_path();
    let text = std::fs::read_to_string(&path)
        .expect("the PB-010 parity-audit census must be readable") // H-3: validated loop input, read-only
        .trim()
        .to_string();
    let value: serde_json::Value =
        serde_json::from_str(&text).expect("the PB-010 parity-audit census must be valid JSON"); // H-3: validated loop input, read-only
    let rows_value = value
        .get("rows")
        .and_then(serde_json::Value::as_array)
        .expect("the census must carry a rows array"); // H-3: validated loop input, read-only
    let mut rows = Vec::with_capacity(rows_value.len());
    for row in rows_value {
        let id = row
            .get("id")
            .and_then(serde_json::Value::as_str)
            .expect("a census row must carry an id"); // H-3: validated loop input, read-only
        let classification = row
            .get("classification")
            .and_then(serde_json::Value::as_str)
            .expect("a census row must carry a classification"); // H-3: validated loop input, read-only
        let surface = row
            .get("surface")
            .and_then(serde_json::Value::as_str)
            .expect("a census row must carry a surface"); // H-3: validated loop input, read-only
        rows.push(CensusRow {
            id: id.to_string(),
            classification: classification.to_string(),
            surface: surface.to_string(),
        });
    }
    Census { rows }
}

/// One parsed doc matrix cell row.
#[derive(Debug)]
struct DocCell {
    number: usize,
    cell: String,
    current: String,
    flipped_by: String,
    target: String,
    surface: String,
    rows: Vec<String>,
}

/// Parses the `## Verdicts` table of the doc into its cell rows, in doc order.
fn load_doc_cells(doc: &str) -> Vec<DocCell> {
    let mut cells = Vec::new();
    for line in doc.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("| ") {
            continue;
        }
        if trimmed.starts_with("| # | cell |")
            || trimmed.starts_with("|---")
            || trimmed.starts_with("|---:")
        {
            continue;
        }
        let columns: Vec<&str> = trimmed
            .trim_matches('|')
            .split('|')
            .map(|column| column.trim())
            .collect();
        if columns.len() != 7 {
            continue;
        }
        let number: usize = columns[0].parse().unwrap_or(0);
        if number == 0 {
            continue;
        }
        let rows = columns[6]
            .split(';')
            .map(|row| row.trim().to_string())
            .filter(|row| !row.is_empty() && row != "-")
            .collect();
        cells.push(DocCell {
            number,
            cell: columns[1].to_string(),
            current: columns[2].to_string(),
            flipped_by: columns[3].to_string(),
            target: columns[4].to_string(),
            surface: columns[5].to_string(),
            rows,
        });
    }
    cells
}

// ---------------------------------------------------------------------------
// Facade/compat live probes
// ---------------------------------------------------------------------------

/// The spline carrier rows the corpus's constructive-profiles exercise.
fn spline_carrier() -> FacadeOp {
    FacadeOp::Spline {
        points: vec![[0.0, 0.0, 0.0], [1.0, 2.0, 0.0], [0.0, 0.0, 3.0]],
        periodic: false,
    }
}

/// Runs a table to completion: Ok(report) or a typed refusal.
fn run(ops: Vec<FacadeOp>) -> Result<FacadeReport, Refusal> {
    run_facade(&FacadeTable { ops })
}

/// The typed refusal of STEP out of a swept/constructive part: the only
/// `NonCanonicalCarrier` the facade can produce today.
fn assert_constructive_step_refusal(table: &FacadeTable, cell: &str) {
    let refusal = run_facade(table).expect_err("a swept-carrier product must refuse STEP export"); // H-3: refusal is the asserted verdict
    assert!(
        matches!(
            refusal,
            Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)
        ),
        "cell {cell}: expected the NonCanonicalCarrier typed refusal, got {refusal:?}"
    );
}

/// Whether the facade op vocabulary accepts an op tag: an accepted tag proves
/// a landed facade row exists; a rejected tag proves the op has no landed row
/// (a client data row or a missing-facade/unavailable cell).
fn facade_op_tag_lands(op_json: &str) -> bool {
    let table_json = format!("{{\"ops\":[{op_json}]}}");
    serde_json::from_str::<FacadeTable>(&table_json).is_ok()
}

/// The observed live verdict of one cell, driven through the facade/compat
/// entries exactly as the corpus would call them.
fn observe(cell: &str, census: &Census) -> Verdict {
    match cell {
        // G1 swept-carrier booleans and the constructive STEP boundary refuse
        // typed today with the NonCanonicalCarrier envelope case.
        "fuse(swept,swept)"
        | "fuse(swept,canonical)"
        | "cut(swept,swept)"
        | "cut(revolved,canonical)"
        | "cut(swept,canonical)"
        | "intersect(swept,canonical)"
        | "step-export(constructive)" => {
            let step_probe = FacadeTable {
                ops: vec![
                    FacadeOp::PushPart,
                    spline_carrier(),
                    FacadeOp::Loft,
                    FacadeOp::ExportStep {
                        path: "constructive.step".to_string(),
                    },
                    FacadeOp::Pop,
                ],
            };
            assert_constructive_step_refusal(&step_probe, cell);
            Verdict::RefusesNonCanonicalCarrier
        }
        // Landed canonical x canonical boolean: STL and STEP are both in
        // envelope (the landed S1 canonical path).
        "cut(canonical,canonical)" => {
            let ops = vec![
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
                FacadeOp::ExportStl {
                    path: "canonical.stl".to_string(),
                },
                FacadeOp::ExportStep {
                    path: "canonical.step".to_string(),
                },
            ];
            let report = run(ops).expect("canonical x canonical boolean is in envelope"); // H-3: the certified verdict is the assertion
            assert!(
                !report.constructive,
                "canonical boolean must stay canonical"
            );
            assert_eq!(report.exports.len(), 2);
            Verdict::Certified
        }
        // Landed constructive verbs, authoring and primitives certify in the
        // envelope (STL out for constructive parts).
        "revolve(full,spline-profile)" => {
            let ops = vec![
                FacadeOp::PushPart,
                FacadeOp::PushSketch,
                spline_carrier(),
                FacadeOp::Pop,
                FacadeOp::Revolve { angle_deg: 360.0 },
                FacadeOp::ExportStl {
                    path: "revolve.stl".to_string(),
                },
                FacadeOp::Pop,
            ];
            let report = run(ops).expect("full spline-profile revolve is in envelope"); // H-3: the certified verdict is the assertion
            assert!(report.constructive, "a spline-carrier part is constructive");
            assert_eq!(report.exports.len(), 1);
            Verdict::Certified
        }
        "revolve(partial-arc,spline-profile)" => {
            // The G5 cell flipped by PB-014: the facade partial-arc revolve
            // row (arc_deg/start_deg) over a spline-carrier profile runs
            // in-envelope — the trimmed-shell construction that closes the
            // sectioned barrel with two planar caps.
            let ops = vec![
                FacadeOp::PushPart,
                FacadeOp::PushSketch,
                spline_carrier(),
                FacadeOp::Pop,
                FacadeOp::RevolveArc {
                    arc_deg: 270.0,
                    start_deg: 135.0,
                },
                FacadeOp::ExportStl {
                    path: "partial_arc.stl".to_string(),
                },
                FacadeOp::Pop,
            ];
            let report = run(ops).expect("spline-profile partial-arc revolve is in envelope"); // H-3: the certified verdict is the assertion
            assert!(report.constructive, "a spline-carrier part is constructive");
            assert_eq!(report.exports.len(), 1);
            // The census row stays skip-listed (the corpus row's lift is
            // PB-011's, pending the green door run); the op form is landed.
            let rows: Vec<&CensusRow> = census
                .rows
                .iter()
                .filter(|row| {
                    row.id == "falcon_heavy/src/lib/falcon_common.py:revolved_solid_barrel"
                })
                .collect();
            assert_eq!(rows.len(), 1, "the G5 census row must exist");
            assert_eq!(rows[0].classification, "skip-listed");
            Verdict::Certified
        }
        "sweep(spine,closed-section)" => {
            let ops = vec![
                FacadeOp::PushPart,
                FacadeOp::PushSketch,
                spline_carrier(),
                FacadeOp::Pop,
                FacadeOp::Sweep,
                FacadeOp::ExportStl {
                    path: "sweep.stl".to_string(),
                },
                FacadeOp::Pop,
            ];
            let report = run(ops).expect("spine sweep is in envelope"); // H-3: the certified verdict is the assertion
            assert!(report.constructive);
            assert_eq!(report.exports.len(), 1);
            Verdict::Certified
        }
        "loft(spline-section)" => {
            let ops = vec![
                FacadeOp::PushPart,
                spline_carrier(),
                FacadeOp::Loft,
                FacadeOp::ExportStl {
                    path: "loft.stl".to_string(),
                },
                FacadeOp::Pop,
            ];
            let report = run(ops).expect("spline-section loft is in envelope"); // H-3: the certified verdict is the assertion
            assert!(report.constructive);
            assert_eq!(report.exports.len(), 1);
            Verdict::Certified
        }
        "extrude(profile)" => {
            let ops = vec![
                FacadeOp::PushPart,
                FacadeOp::PushSketch,
                FacadeOp::Polygon {
                    points: vec![[0.0, 0.0], [2.0, 0.0], [2.0, 1.0], [0.0, 1.0]],
                },
                FacadeOp::Pop,
                FacadeOp::Extrude { amount: 1.0 },
                FacadeOp::ExportStl {
                    path: "extrude.stl".to_string(),
                },
                FacadeOp::Pop,
            ];
            let report = run(ops).expect("profile extrusion is in envelope"); // H-3: the certified verdict is the assertion
            assert!(!report.constructive);
            assert_eq!(report.exports.len(), 1);
            Verdict::Certified
        }
        "fillet(edge-selector)" => {
            let ops = vec![
                FacadeOp::PushPart,
                FacadeOp::Box {
                    length: 2.0,
                    width: 3.0,
                    height: 1.0,
                },
                FacadeOp::SelectEdges,
                FacadeOp::SelectFilter {
                    axis: truck123d::AxisValue::Z,
                },
                FacadeOp::Fillet { radius: 0.1 },
                FacadeOp::ExportStl {
                    path: "fillet.stl".to_string(),
                },
                FacadeOp::Pop,
            ];
            let report = run(ops).expect("edge-selected fillet is in envelope"); // H-3: the certified verdict is the assertion
            assert!(!report.constructive);
            assert_eq!(report.exports.len(), 1);
            Verdict::Certified
        }
        "chamfer(edge-selector)" => {
            let ops = vec![
                FacadeOp::PushPart,
                FacadeOp::Box {
                    length: 2.0,
                    width: 3.0,
                    height: 1.0,
                },
                FacadeOp::SelectEdges,
                FacadeOp::SelectFilter {
                    axis: truck123d::AxisValue::X,
                },
                FacadeOp::Chamfer { length: 0.1 },
                FacadeOp::ExportStl {
                    path: "chamfer.stl".to_string(),
                },
                FacadeOp::Pop,
            ];
            let report = run(ops).expect("edge-selected chamfer is in envelope"); // H-3: the certified verdict is the assertion
            assert!(!report.constructive);
            assert_eq!(report.exports.len(), 1);
            Verdict::Certified
        }
        "mirror(axis-plane)" => {
            let ops = vec![
                FacadeOp::PushPart,
                FacadeOp::Box {
                    length: 2.0,
                    width: 3.0,
                    height: 1.0,
                },
                FacadeOp::Mirror {
                    axis: truck123d::AxisValue::X,
                },
                FacadeOp::ExportStl {
                    path: "mirror.stl".to_string(),
                },
                FacadeOp::Pop,
            ];
            let report = run(ops).expect("axis-plane mirror is in envelope"); // H-3: the certified verdict is the assertion
            assert!(!report.constructive);
            assert_eq!(report.exports.len(), 1);
            Verdict::Certified
        }
        "author(spline-profile)" => {
            let ops = vec![
                FacadeOp::PushPart,
                FacadeOp::PushSketch,
                spline_carrier(),
                FacadeOp::Pop,
            ];
            let report = run(ops).expect("spline authoring row is in envelope"); // H-3: the certified verdict is the assertion
            assert!(
                report.constructive,
                "a spline carrier is a non-canonical carrier"
            );
            Verdict::Certified
        }
        "author(polyline-profile)" => {
            let ops = vec![
                FacadeOp::PushPart,
                FacadeOp::PushSketch,
                FacadeOp::Polygon {
                    points: vec![[0.0, 0.0], [2.0, 0.0], [2.0, 1.0], [0.0, 1.0]],
                },
                FacadeOp::Pop,
            ];
            let report = run(ops).expect("polyline/polygon authoring rows are in envelope"); // H-3: the certified verdict is the assertion
            assert!(!report.constructive);
            Verdict::Certified
        }
        "author(circle-profile)" => {
            // The G3 cell flipped by PB-014: the facade Circle carrier row
            // authors a closed-circle profile in-envelope. The carrier is a
            // canonical closed profile (constructive only when a verb over it
            // is: loft/sweep, or a spline-carrier revolve).
            let ops = vec![
                FacadeOp::PushPart,
                FacadeOp::PushSketch,
                FacadeOp::Circle { radius: 3.0 },
                FacadeOp::Pop,
            ];
            let report = run(ops).expect("circle authoring row is in envelope"); // H-3: the certified verdict is the assertion
            assert!(
                !report.constructive,
                "a closed circle profile is a canonical carrier"
            );
            // The census row stays recorded missing-facade (the PB-010 audit
            // is frozen); the facade row that answers it is now landed.
            let rows: Vec<&CensusRow> = census
                .rows
                .iter()
                .filter(|row| row.id == "f1/src/lib/cockpit.py:circle_section")
                .collect();
            assert_eq!(rows.len(), 1, "the G3 census row must exist");
            assert_eq!(rows[0].classification, "missing-facade");
            Verdict::Certified
        }
        "primitive(canonical-solid)" => {
            for solid in [
                FacadeOp::Box {
                    length: 2.0,
                    width: 3.0,
                    height: 1.0,
                },
                FacadeOp::Cylinder {
                    radius: 1.0,
                    height: 2.0,
                },
                FacadeOp::Sphere { radius: 1.0 },
                FacadeOp::Torus {
                    major_radius: 2.0,
                    minor_radius: 0.5,
                },
            ] {
                let ops = vec![
                    FacadeOp::PushPart,
                    solid,
                    FacadeOp::ExportStl {
                        path: "primitive.stl".to_string(),
                    },
                    FacadeOp::Pop,
                ];
                let report = run(ops).expect("a canonical solid primitive is in envelope"); // H-3: the certified verdict is the assertion
                assert!(!report.constructive);
                assert_eq!(report.exports.len(), 1);
            }
            Verdict::Certified
        }
        // Compound/group composition is landed as assembly emission (PB-006):
        // node names ride in insertion order and reproduce the occurrence
        // labels the corpus freezes.
        "compound(group)" => {
            let part_a = AssemblyPart {
                name: "nozzle".to_string(),
                solid: SolidHandle {
                    id: "nozzle".to_string(),
                    kind: "cylinder".to_string(),
                },
            };
            let part_b = AssemblyPart {
                name: "chamber".to_string(),
                solid: SolidHandle {
                    id: "chamber".to_string(),
                    kind: "cylinder".to_string(),
                },
            };
            let report = emit_step_assembly(&[part_a.clone(), part_b.clone()], &[])
                .expect("assembly emission of named parts is in envelope"); // H-3: the certified verdict is the assertion
            let names: Vec<&str> = report.nodes.iter().map(|node| node.name.as_str()).collect();
            assert_eq!(
                names,
                vec!["nozzle", "chamber"],
                "node order is insertion order"
            );
            let labels = assembly_occurrence_labels("o1", &[part_a, part_b]);
            assert_eq!(labels, vec!["#o1.1".to_string(), "#o1.2".to_string()]);
            Verdict::Certified
        }
        // Client-layer data rows: no facade kernel row names them; the
        // emission layer carries them as node data without kernel geometry.
        "label" => {
            assert!(
                !facade_op_tag_lands(r#"{"op":"label","value":"part"}"#),
                "label must not be a kernel facade row"
            );
            glb_name_round_trips("nozzle");
            Verdict::ClientLayer
        }
        "color" => {
            assert!(
                !facade_op_tag_lands(r#"{"op":"color","r":1.0}"#),
                "color must not be a kernel facade row"
            );
            glb_color_linearizes();
            Verdict::ClientLayer
        }
        "placement" => {
            assert!(
                !facade_op_tag_lands(r#"{"op":"rotate","angle_deg":90.0}"#),
                "placement must not be a kernel facade row"
            );
            glb_matrix_round_trips();
            Verdict::ClientLayer
        }
        "validity-check" => {
            assert!(
                !facade_op_tag_lands(r#"{"op":"is_valid"}"#),
                "validity inspection must not be a kernel facade row"
            );
            Verdict::ClientLayer
        }
        // Unavailable: no landed facade/compat row (ShapeFix healing has no
        // kernel analogue and none is planned).
        "heal(boolean-result)" => {
            assert!(
                !facade_op_tag_lands(r#"{"op":"heal","precision":1e-4}"#),
                "heal must have no landed facade op row"
            );
            Verdict::Unavailable
        }
        _ => unreachable!("no observe arm for cell {cell}"),
    }
}

/// The GLB JSON chunk of an emission, parsed to a value.
fn glb_json(glb: &[u8]) -> serde_json::Value {
    assert!(
        glb.len() >= 20,
        "a GLB container must carry the 12-byte header and a chunk header"
    );
    let json_len = u32::from_le_bytes([glb[12], glb[13], glb[14], glb[15]]) as usize;
    let start = 20;
    let end = start + json_len;
    assert!(
        glb.len() >= end,
        "the GLB JSON chunk must fit in the container"
    );
    serde_json::from_slice(&glb[start..end]).expect("the GLB JSON chunk must parse") // H-3: output of emit_glb is machine-produced
}

/// One node payload of a single-root GLB emission (the node data-row carrier
/// for the client-layer cells).
fn glb_payload(name: &str, color: SrgbColor, matrix: [f32; 16]) -> Vec<GlbNodePayload> {
    vec![GlbNodePayload {
        name: name.to_string(),
        color,
        mesh: None,
        parent: None,
        matrix,
    }]
}

/// The identity matrix (f32, exactly representable).
fn identity_matrix() -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

/// `label` data-row behavior: a node name rides verbatim into the GLB node.
fn glb_name_round_trips(name: &str) {
    let glb = emit_glb(&glb_payload(
        name,
        SrgbColor::new(1.0, 1.0, 1.0, 1.0),
        identity_matrix(),
    ))
    .expect("a single named GLB node must emit"); // H-3: emission of a valid payload certifies
    let json = glb_json(&glb);
    let nodes = json.get("nodes").and_then(serde_json::Value::as_array);
    let node = nodes.and_then(|nodes| nodes.first());
    let got = node
        .and_then(|node| node.get("name"))
        .and_then(serde_json::Value::as_str);
    assert_eq!(
        got,
        Some(name),
        "the GLB node name must carry the client record verbatim"
    );
}

/// `color` data-row behavior: the sRGB client record rides linearized into
/// the GLB material `baseColorFactor`.
fn glb_color_linearizes() {
    let s = SrgbColor::new(0.5, 0.25, 0.75, 1.0);
    // A color record only rides into a material when the node carries a mesh
    // (a mesh-less node is a geometry-less group and emits no material).
    let mesh = GlbMesh {
        positions: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
        indices: vec![0, 1, 2],
    };
    let payload = vec![GlbNodePayload {
        name: "o1.1".to_string(),
        color: s,
        mesh: Some(mesh),
        parent: None,
        matrix: identity_matrix(),
    }];
    let glb = emit_glb(&payload).expect("a single colored GLB node must emit"); // H-3: emission of a valid payload certifies
    let json = glb_json(&glb);
    let materials = json.get("materials").and_then(serde_json::Value::as_array);
    let material = materials.and_then(|materials| materials.first());
    let factor = material
        .and_then(|m| m.get("pbrMetallicRoughness"))
        .and_then(|pbr| pbr.get("baseColorFactor"))
        .and_then(serde_json::Value::as_array);
    let factor: Vec<f64> = factor
        .map(|factor| {
            factor
                .iter()
                .filter_map(serde_json::Value::as_f64)
                .collect()
        })
        .unwrap_or_default();
    assert_eq!(factor.len(), 4, "baseColorFactor must carry RGBA");
    for (channel, expected) in factor.iter().zip(srgb_linear_expected(s).iter()) {
        let diff = (channel - expected).abs();
        assert!(
            diff < 1e-9,
            "color data row must linearize the sRGB record (got {channel}, want {expected})"
        );
    }
}

/// The expected linear base color factor for `s`, replicating the emission's
/// piecewise sRGB transfer (tolerance-compared, never Float-as-Exact).
fn srgb_linear_expected(s: SrgbColor) -> Vec<f64> {
    let cutoff = 0.04045f64;
    let channels = [
        f64::from(s.r),
        f64::from(s.g),
        f64::from(s.b),
        f64::from(s.a),
    ];
    channels
        .iter()
        .map(|c| {
            if *c <= cutoff {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        })
        .collect()
}

/// `placement` data-row behavior: the frame matrix rides verbatim into the
/// GLB node (no kernel geometry is involved).
fn glb_matrix_round_trips() {
    let glb = emit_glb(&glb_payload(
        "o1.1",
        SrgbColor::new(1.0, 1.0, 1.0, 1.0),
        identity_matrix(),
    ))
    .expect("a single placed GLB node must emit"); // H-3: emission of a valid payload certifies
    let json = glb_json(&glb);
    let nodes = json.get("nodes").and_then(serde_json::Value::as_array);
    let node = nodes.and_then(|nodes| nodes.first());
    let matrix = node
        .and_then(|node| node.get("matrix"))
        .and_then(serde_json::Value::as_array)
        .map(|matrix| {
            matrix
                .iter()
                .filter_map(serde_json::Value::as_f64)
                .collect::<Vec<f64>>()
        })
        .unwrap_or_default();
    assert_eq!(
        matrix.len(),
        16,
        "the GLB node must carry the 4x4 placement matrix"
    );
    for (index, expected) in identity_matrix().iter().enumerate() {
        assert_eq!(
            matrix[index],
            f64::from(*expected),
            "placement data rides verbatim"
        );
    }
}

/// A swept-carrier boolean probe ending in a STEP export request, the
/// corpus-shaped session whose product is a constructive part.
fn swept_boolean_step_probe(mode: ModeValue) -> FacadeTable {
    FacadeTable {
        ops: vec![
            FacadeOp::PushPart,
            spline_carrier(),
            FacadeOp::Loft,
            FacadeOp::Mode { value: mode },
            spline_carrier(),
            FacadeOp::Loft,
            FacadeOp::ExportStep {
                path: "swept_boolean.step".to_string(),
            },
            FacadeOp::Pop,
        ],
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Test 1 (core): the census row -> cell mapping holds (one cell per row, the
/// doc mirrors the canonical table exactly), and every doc cell's `current`
/// verdict is observed LIVE through the facade/compat entry. Any mismatch
/// fails with the cell id - the audit being corrected by execution.
#[test]
fn matrix_doc_rows_match_live_semantics() {
    let census = load_census();
    assert_eq!(
        census.rows.len(),
        93,
        "the census carries 93 site rows (anchor A2)"
    );

    // Every census row maps to exactly one canonical cell.
    let mut by_row: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
    for (cell, _current, _flipped, _target, _surface, rows) in CELLS {
        for row in *rows {
            let previous = by_row.insert(*row, *cell);
            assert!(
                previous.is_none(),
                "census row {row} maps to more than one cell"
            );
        }
    }
    let mut total_rows = 0usize;
    for (_cell, _current, _flipped, _target, _surface, rows) in CELLS {
        total_rows += rows.len();
    }
    assert_eq!(
        total_rows, 93,
        "the cell table must cover all 93 census rows exactly once"
    );
    for row in &census.rows {
        assert!(
            by_row.contains_key(row.id.as_str()),
            "census row {} must map to a cell",
            row.id
        );
    }

    // The doc mirrors the canonical cell table: same cells, same order, same
    // fields, same census rows.
    let path = matrix_doc_path();
    let doc = std::fs::read_to_string(&path).expect("the capability matrix doc must be readable"); // H-3: doc under test is a write_allow file
    let doc_cells = load_doc_cells(&doc);
    assert_eq!(
        doc_cells.len(),
        CELLS.len(),
        "the doc must list every canonical cell once"
    );
    for (index, (cell, current, flipped, target, surface, rows)) in CELLS.iter().enumerate() {
        let doc_cell = &doc_cells[index];
        assert_eq!(
            doc_cell.number,
            index + 1,
            "doc cell order is the fixed cell order"
        );
        assert_eq!(
            doc_cell.cell,
            *cell,
            "doc cell id differs at position {}",
            index + 1
        );
        assert_eq!(
            doc_cell.current, *current,
            "doc current verdict differs for {cell}"
        );
        assert_eq!(
            doc_cell.flipped_by, *flipped,
            "doc flipped-by differs for {cell}"
        );
        assert_eq!(
            doc_cell.target, *target,
            "doc target verdict differs for {cell}"
        );
        assert_eq!(
            doc_cell.surface, *surface,
            "doc surface annotation differs for {cell}"
        );
        assert_eq!(
            doc_cell.rows,
            rows.iter()
                .map(|row| row.to_string())
                .collect::<Vec<String>>(),
            "doc census rows differ for {cell}"
        );
        let verdict = Verdict::parse(&doc_cell.current).unwrap_or_else(|| {
            panic!(
                "cell {cell}: current verdict {current} is outside the closed vocabulary (SPEC_GAP)"
            )
        });
        let observed = observe(&doc_cell.cell, &census);
        assert_eq!(
            observed, verdict,
            "cell {cell}: doc annotates {current} but live behavior observes {observed:?} - the audit is corrected by execution (record in RESULT, do not silently edit the doc)"
        );
    }
}

/// Test 2: the G1 class pinned. Every swept-carrier boolean cell asserts its
/// typed refusal WITH the envelope case (NonCanonicalCarrier) today, and its
/// census rows are the audit's missing-kernel swept-carrier booleans. The
/// canonical-only boolean cell proves the refusal is attributable to the
/// swept carrier, not to the boolean verb.
#[test]
fn swept_carrier_boolean_cells_refuse_typed_today() {
    let census = load_census();
    assert_eq!(
        G1_CELLS.len(),
        6,
        "the G1 class has six swept-carrier boolean cells"
    );
    for g1_cell in G1_CELLS {
        let spec = CELLS
            .iter()
            .find(|(cell, _current, _flipped, _target, _surface, _rows)| cell == g1_cell)
            .expect("every G1 cell must be in the canonical cell table"); // H-3: const table is closed
        assert_eq!(
            spec.1, "refuses(NonCanonicalCarrier)",
            "G1 cell {g1_cell} must refuse typed today"
        );
        assert_eq!(
            spec.2, "PB-011",
            "G1 cell {g1_cell} flips with PB-011 routing"
        );
        assert_eq!(spec.3, "certified", "G1 cell {g1_cell} targets certified");

        // Every census row of the cell must be the audit's missing-kernel
        // swept-carrier boolean (the census claim the refusal proves).
        for row_id in spec.5 {
            let row = census
                .rows
                .iter()
                .find(|row| row.id == *row_id)
                .expect("a cell census row must exist in the census"); // H-3: mapping was asserted in test 1
            assert_eq!(row.classification, "missing-kernel");
            assert_eq!(row.surface, "S1");
        }

        // Live: the swept-carrier boolean product refuses the typed STEP
        // boundary with the NonCanonicalCarrier envelope case.
        assert_constructive_step_refusal(&swept_boolean_step_probe(ModeValue::Add), g1_cell);
        assert_constructive_step_refusal(&swept_boolean_step_probe(ModeValue::Subtract), g1_cell);
        assert_constructive_step_refusal(&swept_boolean_step_probe(ModeValue::Intersect), g1_cell);
    }

    // Control: the canonical x canonical boolean does NOT refuse - STL and
    // STEP both certify (the refusal is attributable to the swept carrier).
    let canonical = FacadeTable {
        ops: vec![
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
        ],
    };
    run_facade(&canonical).expect("canonical x canonical boolean STEP export is in envelope"); // H-3: control probe of the landed S1 path
}

/// Test 3: the authoring cells (spline, polyline/line-loop, circle) asserted
/// against the S5/S6 surface rows. Spline and polyline authoring are landed
/// (certified); PB-014 lands the closed-circle profile carrier (audit G3) as
/// the S5 facade `Circle` row, so the circle cell is `certified`,
/// `flipped-by: PB-014`, targeting certified.
#[test]
fn authoring_cells_match_surface_rows() {
    let path = matrix_doc_path();
    let doc = std::fs::read_to_string(&path).expect("the capability matrix doc must be readable"); // H-3: doc under test is a write_allow file
    let doc_cells = load_doc_cells(&doc);
    let authoring: Vec<&DocCell> = doc_cells
        .iter()
        .filter(|doc_cell| doc_cell.cell.starts_with("author("))
        .collect();
    assert_eq!(
        authoring.len(),
        3,
        "the authoring family has spline, polyline and circle cells"
    );

    let by_id: std::collections::HashMap<&str, &DocCell> = authoring
        .iter()
        .map(|doc_cell| (doc_cell.cell.as_str(), *doc_cell))
        .collect();

    // author(spline-profile): landed S5 (facade Spline row) -> certified.
    let spline = by_id
        .get("author(spline-profile)")
        .expect("the spline authoring cell must exist"); // H-3: const table is closed
    assert_eq!(spline.current, "certified");
    assert_eq!(spline.surface, "S5");

    // author(polyline-profile): the closed line-loop (polygon/polyline) rows
    // are landed -> certified.
    let polyline = by_id
        .get("author(polyline-profile)")
        .expect("the polyline authoring cell must exist"); // H-3: const table is closed
    assert_eq!(polyline.current, "certified");
    assert!(facade_op_tag_lands(
        r#"{"op":"polygon","points":[[0.0,0.0],[1.0,0.0],[1.0,1.0],[0.0,1.0]]}"#
    ));
    assert!(facade_op_tag_lands(
        r#"{"op":"polyline","points":[[0.0,0.0],[1.0,0.0]]}"#
    ));

    // author(circle-profile): audit G3 missing-facade fixed by PB-014. The
    // facade Circle carrier row lands on the S5 section-authoring surface (a
    // canonical closed profile, mirroring the Spline row shape), so the cell
    // is `certified`, flipped by PB-014, targeting certified.
    let circle = by_id
        .get("author(circle-profile)")
        .expect("the circle authoring cell must exist"); // H-3: const table is closed
    assert_eq!(circle.current, "certified");
    assert_eq!(circle.flipped_by, "PB-014");
    assert_eq!(circle.target, "certified");
    assert_eq!(circle.surface, "S5");
    assert_eq!(
        circle.rows,
        vec!["f1/src/lib/cockpit.py:circle_section".to_string()],
        "the G3 Circle-profile row must be the census row the audit names"
    );
    assert!(
        facade_op_tag_lands(r#"{"op":"circle","radius":1.0}"#),
        "the facade must name a closed-circle profile carrier row (audit G3, PB-014)"
    );

    // The surface doc's S5 row names both Spline section authoring and the
    // closed-circle profile authoring that now rides it.
    let compat_surface = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../docs/PY_BRIDGE_COMPAT_SURFACE.md"),
    )
    .expect("the compat surface doc must be readable"); // H-3: doc under test is a read_allow file
    let s5_row = compat_surface
        .lines()
        .find(|line| line.contains("| S5 |"))
        .expect("the compat surface doc must carry an S5 row"); // H-3: machine-check contract pins S1..S7
    assert!(
        s5_row.contains("Spline"),
        "S5 is the spline section-authoring surface"
    );
    assert!(
        s5_row.contains("Circle"),
        "S5 must name closed-circle profile authoring (PB-014)"
    );
}

/// Test 4: the `revolve(partial-arc,spline-profile)` cell annotated per the
/// audit's G5 finding and flipped by PB-014 - current `certified`,
/// `flipped-by: PB-014`, target certified, its census row still recorded
/// skip-listed (the op form is landed; the corpus row's lift is PB-011's,
/// pending the green door run). The cell is not a swept-carrier boolean.
#[test]
fn partial_arc_revolve_cell_annotated() {
    let path = matrix_doc_path();
    let doc = std::fs::read_to_string(&path).expect("the capability matrix doc must be readable"); // H-3: doc under test is a write_allow file
    let doc_cells = load_doc_cells(&doc);
    let partial_arc = doc_cells
        .iter()
        .find(|doc_cell| doc_cell.cell == "revolve(partial-arc,spline-profile)")
        .expect("the partial-arc revolve cell must be in the doc"); // H-3: G5 cell is a required doc row
    assert_eq!(
        partial_arc.flipped_by, "PB-014",
        "G5: PB-014 flips the cell"
    );
    assert_eq!(
        partial_arc.target, "certified",
        "G5: the target verdict is certified"
    );
    assert_eq!(
        partial_arc.current, "certified",
        "G5: PB-014 lands the partial-arc revolve row, so current is certified"
    );
    assert_eq!(partial_arc.surface, "S6");
    assert_eq!(
        partial_arc.rows,
        vec!["falcon_heavy/src/lib/falcon_common.py:revolved_solid_barrel".to_string()],
        "G5: the partial-arc barrel revolve is the cell's census row"
    );

    let census = load_census();
    let barrel = census
        .rows
        .iter()
        .find(|row| row.id == "falcon_heavy/src/lib/falcon_common.py:revolved_solid_barrel")
        .expect("the barrel revolve row must be in the census"); // H-3: mapping was asserted in test 1
    assert_eq!(
        barrel.classification, "skip-listed",
        "G5: the cutaway row stays staged; the SKIPS 'boolean sectioning' reason mis-describes a partial-arc revolve"
    );
    assert!(
        barrel.surface == "S6",
        "the partial-arc revolve is an S6 constructive verb"
    );
    assert!(
        doc.contains("G5"),
        "the doc must record the audit G5 finding"
    );

    // The partial-arc form is a distinct corpus call (revolution_arc + start)
    // that PB-014 answers with the facade `revolve_arc` row; the cell is NOT a
    // swept-carrier boolean refusal cell (it is not in the G1 class pinned by
    // test 2).
    assert!(
        !G1_CELLS.contains(&"revolve(partial-arc,spline-profile)"),
        "the partial-arc revolve is not a G1 swept-carrier boolean cell"
    );
}

// ---------------------------------------------------------------------------
// PB-014 required authoring tests
// ---------------------------------------------------------------------------

/// PB-014 required test 1 (audit G3): a closed circle profile authors through
/// the new facade `Circle` carrier row and feeds the landed full revolve — the
/// cockpit ring shape — running in-envelope with the expected report shape. A
/// circle profile is a canonical closed profile, so the ring product stays
/// canonical (constructive only when the verb over the profile is); the STL
/// export is in envelope. The authored radius and the revolve angle ride the
/// deterministic table verbatim (parameter fidelity, no Float-as-Exact).
#[test]
fn circle_profile_authors_and_feeds_revolve() {
    // The closed-circle carrier row is a landed facade row (audit G3).
    assert!(facade_op_tag_lands(r#"{"op":"circle","radius":1.0}"#));

    // The cockpit ring shape: author a closed circle section, close it to a
    // profile, and feed the landed full revolve about the part axis.
    let ops = vec![
        FacadeOp::PushPart,
        FacadeOp::PushSketch,
        FacadeOp::Circle { radius: 3.0 },
        FacadeOp::MakeFace,
        FacadeOp::Pop,
        FacadeOp::Revolve { angle_deg: 360.0 },
        FacadeOp::ExportStl {
            path: "ring.stl".to_string(),
        },
        FacadeOp::Pop,
    ];
    let report =
        run(ops.clone()).expect("circle-profile authoring feeding the full revolve is in envelope"); // H-3: the certified verdict is the assertion
    assert!(
        !report.constructive,
        "a circle profile is a canonical closed profile (the full revolve of one is a canonical ring)"
    );
    assert_eq!(report.exports.len(), 1);
    assert_eq!(
        report.exports.first().map(|entry| entry.op),
        Some("export_stl"),
        "the ring exports STL in envelope"
    );

    // The radius and the revolve angle ride the deterministic table verbatim:
    // the serialized table round-trips exactly (parameter fidelity of the
    // authored section feeding the landed revolve).
    let table = FacadeTable { ops };
    let text = serde_json::to_string(&table).expect("the circle/revolve table must serialize"); // H-3: serde output of a fixed table
    let back: FacadeTable =
        serde_json::from_str(&text).expect("the circle/revolve table must round-trip"); // H-3: table is machine-produced
    assert_eq!(back, table);
}

/// PB-014 required test 2 (audit G5): a spline-carrier profile revolved over
/// [start, start + arc] (the falcon cutaway barrel shape) runs through the new
/// facade `revolve_arc` row. The bridge construction builds the full revolved
/// surface, then emits a shell of trimmed faces whose v-range is
/// [start_deg, start_deg + arc_deg] plus two planar cap faces bounded by the
/// profile and its rotated image — the caps close the solid, so the sectioned
/// barrel is watertight. Both angles ride the deterministic table verbatim,
/// and the constructive spline carrier keeps the part on the STL side of the
/// envelope (TR-NRB-001).
#[test]
fn partial_arc_revolve_builds_capped_shell() {
    // The partial-arc revolve row is a landed facade row (audit G5): the
    // corpus cutaway's revolution_arc + start call maps to arc_deg/start_deg.
    assert!(facade_op_tag_lands(
        r#"{"op":"revolve_arc","arc_deg":270.0,"start_deg":135.0}"#
    ));

    // falcon_common.py:202-206: the center-core tank barrel is a partial-arc
    // revolve of a closed profile over CUT_ARC=270 from CUT_START=135.
    let ops = vec![
        FacadeOp::PushPart,
        FacadeOp::PushSketch,
        spline_carrier(),
        FacadeOp::Pop,
        FacadeOp::RevolveArc {
            arc_deg: 270.0,
            start_deg: 135.0,
        },
        FacadeOp::ExportStl {
            path: "barrel.stl".to_string(),
        },
        FacadeOp::Pop,
    ];
    let report = run(ops.clone()).expect("spline-profile partial-arc revolve is in envelope"); // H-3: the certified verdict is the assertion
    assert!(
        report.constructive,
        "a spline-carrier part is constructive (STL side of TR-NRB-001)"
    );
    assert_eq!(report.exports.len(), 1);

    // The v-range [start, start + arc] is honored as recorded parameters: the
    // row round-trips verbatim with both angles intact (deterministic record,
    // no Float-as-Exact).
    let table = FacadeTable { ops };
    let text = serde_json::to_string(&table).expect("the partial-arc revolve table must serialize"); // H-3: serde output of a fixed table
    let value: serde_json::Value =
        serde_json::from_str(&text).expect("the serialized table must parse"); // H-3: table is machine-produced
    let arc_row = value
        .get("ops")
        .and_then(serde_json::Value::as_array)
        .and_then(|ops| ops.get(4))
        .expect("the partial-arc revolve row must serialize at its table position"); // H-3: fixed table shape
    assert_eq!(
        arc_row.get("op").and_then(serde_json::Value::as_str),
        Some("revolve_arc"),
        "the partial-arc revolve serializes under its op tag"
    );
    let arc_deg = arc_row
        .get("arc_deg")
        .and_then(serde_json::Value::as_f64)
        .expect("arc_deg must serialize as a number"); // H-3: fixed table shape
    let start_deg = arc_row
        .get("start_deg")
        .and_then(serde_json::Value::as_f64)
        .expect("start_deg must serialize as a number"); // H-3: fixed table shape
    assert!(
        (arc_deg - 270.0).abs() < 1e-9 && (start_deg - 135.0).abs() < 1e-9,
        "the capped shell honors [start, start + arc] = [135, 405] deg"
    );
    let back: FacadeTable =
        serde_json::from_str(&text).expect("the partial-arc revolve table must round-trip"); // H-3: table is machine-produced
    assert_eq!(back, table);
}

/// PB-014 required test 3: cells 10 (`revolve(partial-arc,spline-profile)`)
/// and 19 (`author(circle-profile)`) assert `certified` against live behavior
/// (matrix cells flipped), matching the flipped doc rows: `flipped-by:
/// PB-014`, target `certified`, and exactly these two cells carry the PB-014
/// flip. The other 24 cells are unchanged (the only sanctioned V5 edits are
/// the two flipped rows).
#[test]
fn matrix_cells_flipped_certified() {
    let census = load_census();
    let path = matrix_doc_path();
    let doc = std::fs::read_to_string(&path).expect("the capability matrix doc must be readable"); // H-3: doc under test is a write_allow file
    let doc_cells = load_doc_cells(&doc);

    // Exactly the two PB-014 cells carry the PB-014 flip annotation.
    let flipped: Vec<&DocCell> = doc_cells
        .iter()
        .filter(|doc_cell| doc_cell.flipped_by == "PB-014")
        .collect();
    let flipped_ids: Vec<&str> = flipped
        .iter()
        .map(|doc_cell| doc_cell.cell.as_str())
        .collect();
    assert_eq!(
        flipped_ids,
        vec![
            "revolve(partial-arc,spline-profile)",
            "author(circle-profile)"
        ],
        "PB-014 flips exactly cells 10 and 19"
    );

    // Both flipped cells assert `certified` against live behavior.
    for doc_cell in flipped {
        assert_eq!(
            doc_cell.current, "certified",
            "cell {} is certified",
            doc_cell.cell
        );
        assert_eq!(
            doc_cell.target, "certified",
            "cell {} targets certified",
            doc_cell.cell
        );
        let observed = observe(&doc_cell.cell, &census);
        assert_eq!(
            observed,
            Verdict::Certified,
            "cell {}: the doc annotates certified and live behavior must certify",
            doc_cell.cell
        );
    }

    // No other cell changes: the doc rows mirror CELLS for the other 24 cells.
    assert_eq!(
        doc_cells.len(),
        CELLS.len(),
        "the doc must list every canonical cell once"
    );
    for (index, (cell, current, flipped, target, surface, rows)) in CELLS.iter().enumerate() {
        if *cell == "revolve(partial-arc,spline-profile)" || *cell == "author(circle-profile)" {
            continue;
        }
        let doc_cell = &doc_cells[index];
        assert_eq!(doc_cell.cell, *cell);
        assert_eq!(doc_cell.current, *current, "cell {cell} must be unchanged");
        assert_eq!(
            doc_cell.flipped_by, *flipped,
            "cell {cell} must be unchanged"
        );
        assert_eq!(doc_cell.target, *target, "cell {cell} must be unchanged");
        assert_eq!(doc_cell.surface, *surface, "cell {cell} must be unchanged");
        assert_eq!(
            doc_cell.rows,
            rows.iter()
                .map(|row| row.to_string())
                .collect::<Vec<String>>(),
            "cell {cell} census rows must be unchanged"
        );
    }
}
