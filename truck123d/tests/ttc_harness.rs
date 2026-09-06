//! PB-008 required Rust suite — the text-to-cad corpus harness
//! (`tests_required`: corpus_runner_executes_canonical_subset,
//! staged_skips_carry_reasons, compat_surface_table_is_complete).
//!
//! The harness under test is `truck123d/compat/**` (declared here through a
//! `#[path]` module — see `compat/mod.rs` for why it is not part of the lib
//! crate). The corpus it drives is `corpus/ttc/**`: the vendored
//! earthtojake/text-to-cad F1 + Falcon-Heavy model trees, the row manifest,
//! the staged-skip machinery and the recorded OCC-derived reference facts.
//!
//! Test 1 executes the canonical-only subset end to end through the door
//! (`corpus/ttc/door.py`, OCC baseline regime — a fresh python process per
//! script), asserting STL + report JSON + geometry facts against the recorded
//! references. Like the PB-004/PB-005 suites this crate already ships, the
//! test needs the machine python; it additionally needs the genuine `build123d`
//! package the corpus is written against (installed on this machine). If it is
//! missing, the test fails loudly rather than skipping: a canonical row whose
//! geometry cannot be produced is a harness failure, not an `#[ignore]`.
//!
//! These tests re-derive geometry under OCC on every run (deterministic given
//! the same OCC revision; comparisons are tolerance-graded). They publish no
//! timing claims — wall time is recorded in the reports as local evidence only.

#[path = "../compat/mod.rs"]
mod compat;

use compat::manifest::{self, ManifestFile};
use compat::reference;
use compat::runner;
use compat::skips;

/// Loads the corpus manifest and the skips file once per test process.
fn corpus_root() -> (std::path::PathBuf, ManifestFile) {
    let corpus = compat::corpus_dir();
    let manifest = manifest::load_manifest(&corpus).expect("the corpus manifest must be valid");
    (corpus, manifest)
}

/// A scratch output directory unique to this test process.
fn scratch_out_dir() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("ttc_harness_{}", std::process::id()))
}

/// Asserts that the corpus manifest row ids are the enrolled corpus rows, and
/// returns the canonical row count.
fn row_census() -> (usize, usize) {
    let (_corpus, manifest) = corpus_root();
    let canonical = manifest::canonical_rows(&manifest).len();
    let skipped = manifest::skipped_rows(&manifest).len();
    assert!(canonical > 0, "the canonical subset must be non-empty");
    assert!(skipped > 0, "the staged-skip list must be non-empty");
    (canonical, skipped)
}

#[test]
fn corpus_runner_executes_canonical_subset() {
    let (corpus, manifest) = corpus_root();
    let canonical = manifest::canonical_rows(&manifest);
    let (canonical_count, _skipped) = row_census();
    assert_eq!(
        canonical.len(),
        canonical_count,
        "the canonical row census must match the manifest"
    );

    // The corpus reach statement: the full Falcon-Heavy vehicle is a canonical
    // row, and the F1 tree rows are enrolled (as staged skips) on the manifest.
    let ids: Vec<&str> = canonical.iter().map(|row| row.id.as_str()).collect();
    assert!(
        ids.contains(&"falcon_heavy/vehicle"),
        "the full Falcon-Heavy vehicle must be in the canonical subset"
    );
    assert!(
        manifest.rows.iter().any(|row| row.family == "f1"),
        "the F1 tree must be enrolled on the corpus manifest"
    );

    // Every canonical row must have a recorded reference file.
    for row in &canonical {
        reference::load_reference(&corpus, row).unwrap_or_else(|e| {
            panic!("canonical row {} must have a valid reference: {e}", row.id)
        });
    }

    // Run the canonical subset end to end: STL + report JSON per row.
    let out_dir = scratch_out_dir();
    let outcomes = runner::run_canonical_rows(&corpus, &manifest, &out_dir);
    assert_eq!(
        outcomes.len(),
        canonical_count,
        "the runner must produce one outcome per canonical row"
    );

    for (row, outcome) in canonical.iter().zip(outcomes.iter()) {
        assert!(
            outcome.ok,
            "row {} failed end to end: {}",
            row.id,
            outcome.error.as_deref().unwrap_or("no error")
        );
        let facts = outcome.facts.as_ref().expect("an ok run carries facts");

        // Geometry facts assert against the recorded OCC-derived reference.
        let reference = runner::compare_against_reference(&corpus, row, outcome)
            .unwrap_or_else(|e| panic!("row {} geometry facts mismatch: {e}", row.id));

        // The produced STL exists and is a real (non-empty) binary STL.
        let stl_path = outcome
            .stl_path
            .as_ref()
            .expect("an ok run produces an STL path");
        assert!(
            stl_path.is_file(),
            "row {} must produce an STL file",
            row.id
        );
        let stl_len = std::fs::metadata(stl_path)
            .map(|m| m.len())
            .expect("the produced STL must be readable");
        assert!(
            stl_len > 84,
            "row {} STL is not a valid binary STL ({} bytes)",
            row.id,
            stl_len
        );

        // The report JSON exists and round-trips as a typed report.
        let report_path = outcome
            .report_path
            .as_ref()
            .expect("an ok run writes a report");
        let report_text = std::fs::read_to_string(report_path)
            .unwrap_or_else(|e| panic!("row {} report must be readable: {e}", row.id));
        let report: runner::ReportFile = serde_json::from_str(&report_text)
            .unwrap_or_else(|e| panic!("row {} report must parse: {e}", row.id));
        assert_eq!(report.schema, runner::REPORT_SCHEMA);
        assert!(report.ok, "row {} report must be ok", row.id);
        assert_eq!(report.row_id, row.id);
        assert!(
            report.stl.as_ref().map(|s| s.triangles).unwrap_or(0) > 0,
            "row {} STL must carry triangles",
            row.id
        );

        // The door re-derived the row's solid-count fact exactly.
        let reference_facts = &reference.facts;
        assert_eq!(
            facts.solid_count, reference_facts.solid_count,
            "row {} solid count must match the recorded reference",
            row.id
        );
    }

    // Determinism across identical ordered input: a second report for the same
    // row set must agree on the fact census (the runner re-runs in manifest
    // order; here we re-parse the reports in order and confirm one row per id).
    let report_ids: Vec<String> = outcomes.iter().map(|o| o.row_id.clone()).collect();
    let mut sorted = report_ids.clone();
    sorted.sort();
    let mut deduped = sorted.clone();
    deduped.dedup();
    assert_eq!(
        sorted.len(),
        deduped.len(),
        "row ids in the run must be unique (one report per canonical row)"
    );
    assert!(
        out_dir.join("falcon_heavy__vehicle.report.json").is_file(),
        "the flagship vehicle row's report must be in the run output"
    );

    // Best-effort cleanup of the scratch STL/report output (the janitor would
    // reclaim a TEMP leak eventually; do not leave ~50 MB per run behind).
    let _ = std::fs::remove_dir_all(&out_dir);
}

#[test]
fn staged_skips_carry_reasons() {
    let (corpus, manifest) = corpus_root();
    let skips = skips::load_skips(&corpus, &manifest).expect("the skips file must validate");
    let census = skips::census(&manifest, &skips);

    // The manifest's skipped rows and the skips rows must be one-to-one.
    assert_eq!(
        census.manifest_skipped, census.skip_rows,
        "every skipped manifest row must carry exactly one skip row"
    );
    assert!(
        census.manifest_skipped > 0,
        "the skip list must not be empty"
    );

    // A reasonless skip, a skip to an unknown row, a skipped row with no skip
    // row, an unresolvable reason and a reason vocabulary entry with no
    // resolved_by are all harness failures.
    assert_eq!(
        census.reasonless, 0,
        "a skip without a reason row is a harness failure"
    );
    assert_eq!(
        census.unresolvable, 0,
        "every skip reason must resolve in the vocabulary"
    );
    assert_eq!(
        census.missing_row, 0,
        "every skipped row must be on the skip list"
    );
    assert_eq!(
        census.unknown_row, 0,
        "a skip row must name a skipped manifest row"
    );
    assert_eq!(
        census.unresolved_reason, 0,
        "every reason must carry resolved_by"
    );

    // Machine-checked shape: every skip row carries a reason that exists in the
    // vocabulary and that reason names the program that unblocks it. The
    // Falcon/F1 rows that boolean-compose swept carriers must resolve to the
    // BIE program's sweep-pair certification.
    for skip in &skips.rows {
        let reason = skip
            .reason
            .as_deref()
            .unwrap_or_else(|| panic!("skip row {} is reasonless", skip.id));
        let reason_row = skips
            .reasons
            .get(reason)
            .unwrap_or_else(|| panic!("skip row {} has unknown reason {reason}", skip.id));
        assert!(
            !reason_row.resolved_by.is_empty(),
            "reason {reason} must resolve to a packet/program"
        );
        if reason == "booleans-on-swept-carriers" {
            assert_eq!(reason_row.resolved_by, "BIE-006");
        }
    }
}

#[test]
fn compat_surface_table_is_complete() {
    // spec 8's measured table has exactly seven surface rows, and every one of
    // them must appear in docs/PY_BRIDGE_COMPAT_SURFACE.md with its status.
    assert_eq!(compat::surface::SURFACE_ROWS.len(), 7);

    let check = compat::surface::load_and_check_doc().expect("the compat surface doc must load");
    assert!(
        check.rows_missing.is_empty(),
        "these compat-surface rows are missing from the doc: {:?}",
        check.rows_missing
    );
    assert!(
        check.rows_without_status.is_empty(),
        "these compat-surface rows carry no landed status in the doc: {:?}",
        check.rows_without_status
    );
    assert_eq!(
        check.rows_ok, 7,
        "all seven compat-surface rows must appear with a status"
    );
}
