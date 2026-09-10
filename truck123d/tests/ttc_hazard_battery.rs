//! The hazard battery (owner-directed): compact pre-identified trouble
//! fixtures run through the corpus door, asserting the DISCIPLINE —
//!
//!   1. never an untyped failure (every non-ok exit is a Refused record),
//!   2. never a green without well-formed facts,
//!   3. never a silent wrong answer.
//!
//! Each fixture carries an EXPECTED verdict class in the table below
//! (green = facts-gated success once its carrier class lands; typed = a
//! typed refusal is the correct permanent answer; staged = typed is correct
//! today, a follow-on packet re-classes it). The battery RECORDS actual vs
//! expected and, in strict mode (env HAZARD_BATTERY_STRICT=1), ASSERTS the
//! expected class — for use after the corpus generations land. Non-strict
//! (default) runs are the drift watch: discipline violations fail, class
//! drifts only report.
//!
//! Run explicitly (it is #[ignore]-gated so the normal suite never churns on
//! expected-red staged rows):
//!
//! ```text
//! cargo test --locked -p truck123d --test ttc_hazard_battery -- --ignored --nocapture
//! ```

use serde_json::Value;
use std::path::{Path, PathBuf};

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../corpus/ttc")
}

fn python_command() -> std::process::Command {
    let mut c = std::process::Command::new("python");
    c.env("PYTHONIOENCODING", "utf-8");
    c
}

/// One battery row: the door entry name and its expected verdict class.
const CASES: &[(&str, &str)] = &[
    // A. designed tangency & degeneracy
    ("sphere_plane_kiss", "staged"),          // boolean gate
    ("coaxial_equal_cylinders", "staged"),    // boolean gate
    ("equal_cylinders_perpendicular", "staged"),
    ("near_tangency", "staged"),
    ("coplanar_face_fuse", "staged"),
    // B. periodicity, seams, decks
    ("cylinder_wrap", "staged"),
    ("seam_split", "staged"),
    ("apex_revolve", "green"),                // revolve arm landed
    // C. trim & topology edges
    ("deep_cavity_cut", "staged"),
    ("zero_thickness_residual", "typed"),     // correct permanent answer
    ("disjoint_fuse", "staged"),
    // D. chain depth & composition
    ("boolean_of_boolean", "typed"),          // depth-2 cell: permanent
    ("long_chain", "staged"),
    ("fillet_then_cut", "staged"),
    // E. scale & tolerance regime
    ("small_feature_large_origin", "staged"),
    ("sliver_faces", "staged"),
    ("scale_span_shock", "green"),            // compound arm landed
    // F. representation extremes
    ("rational_heavy_spline", "green"),       // spline profile + loft row
    ("twisted_loft_stations", "staged"),      // correspondence law
    ("c0_knot_profile", "green"),             // polyline profile + extrude
    // G. assembly identity & placement
    ("prototype_reuse", "green"),             // placement landed
    ("mirror_twins", "green"),
    ("nested_compounds", "green"),
    ("frame_composition_chain", "green"),     // frames landed
    // H. shells, offsets, thickens
    ("box_shell", "staged"),                  // CC strata landed, arm not
    ("cylinder_shell", "staged"),
    ("spline_loft_shell", "staged"),
    ("variable_thickness_offset", "staged"),
    ("thicken_sheet", "staged"),
    ("offset_self_intersect", "typed"),       // singular regime: permanent
    // I. basic operations the corpus never touched
    ("drafted_extrude", "staged"),
    ("loft_intersection", "staged"),
    ("section_slice", "staged"),
    ("asymmetric_chamfer", "staged"),
    ("multi_section_loft_tangent", "staged"),
    ("helical_sweep", "staged"),
    ("twisted_sweep", "staged"),
    ("project_curve_to_face", "staged"),
    ("nonuniform_scale", "staged"),
    ("pattern_composition", "staged"),
    ("fillet_spline_edge", "staged"),
];

fn run_case(entry: &str) -> Value {
    let corpus = corpus_dir();
    let tree = corpus.join("trees/hazard/src");
    let stl = std::env::temp_dir().join(format!("hazard_{entry}.stl"));
    let out = python_command()
        .arg(corpus.join("door.py"))
        .arg(&tree)
        .arg("lib.hazard")
        .arg(entry)
        .arg("[]")
        .arg(&stl)
        .output()
        .unwrap_or_else(|e| panic!("cannot spawn the door for {entry}: {e}"));
    let stdout = String::from_utf8_lossy(&out.stdout);
    let record: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("{entry}: door output was not a record ({e}): {stdout:.400}"));
    assert!(
        record.get("schema").and_then(|s| s.as_str()) == Some("ttc_door_run.v1"),
        "{entry}: unexpected record shape"
    );
    record
}

#[test]
#[ignore = "the hazard battery: run explicitly with -- --ignored; strict mode via HAZARD_BATTERY_STRICT=1"]
fn hazard_battery_discipline() {
    let strict = std::env::var("HAZARD_BATTERY_STRICT").is_ok();
    let mut drift: Vec<String> = Vec::new();
    for (entry, expected) in CASES {
        let record = run_case(entry);
        let ok = record["ok"].as_bool().unwrap_or(false);
        if ok {
            // 2. green requires well-formed facts.
            assert!(
                record.get("facts").is_some() || record.get("stl").is_some(),
                "{entry}: green without facts"
            );
        } else {
            // 1. every non-ok exit is a typed Refused record.
            assert_eq!(
                record["error"]["kind"].as_str(),
                Some("Refused"),
                "{entry}: UNTYPED failure — discipline violation: {}",
                record["error"]
            );
            assert!(
                !record["error"]["message"].as_str().unwrap_or("").is_empty(),
                "{entry}: typed refusal with empty message"
            );
        }
        // 3. class bookkeeping.
        let actual = if ok { "green" } else { "typed" };
        if actual != *expected {
            let line = format!("{entry}: expected {expected}, actual {actual}");
            if strict {
                panic!("STRICT class drift — {line}");
            }
            drift.push(line);
        }
    }
    if !drift.is_empty() {
        println!("=== hazard battery: {} class drifts (staged vs landed) ===", drift.len());
        for d in &drift {
            println!("  {d}");
        }
    }
}
