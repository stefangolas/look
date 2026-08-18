#![deny(clippy::unwrap_used)]

//! Bookkeeping tests for the BG-TOL-001 tolerance migration in
//! `truck-meshalgo`.
//!
//! These read the crate's own source from disk at runtime so a later edit that
//! relocates a migrated site, drops its `// BG-TOL-001:` marker, or "finishes"
//! a deferred area site fails here rather than at review time.

use std::fs;
use std::path::PathBuf;

/// The three source files whose migrated sites must each carry a marker.
const MIGRATED: [&str; 3] = [
    "src/filters/normal_filters.rs",
    "src/tessellation/source_edge.rs",
    "src/tessellation/triangulation.rs",
];

/// The four deferred area files and the exact FIXME count each must carry.
const DEFERRED: [(&str, usize); 4] = [
    ("src/analyzers/collision.rs", 3),
    ("src/analyzers/in_out_judge.rs", 1),
    ("src/analyzers/point_cloud/mod.rs", 1),
    ("src/analyzers/point_cloud/sort_end_points.rs", 1),
];

fn read_source(relative: &str) -> Result<String, Box<dyn std::error::Error>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative);
    fs::read_to_string(path).map_err(Into::into)
}

/// Every line that carries a `ctx.` tolerance predicate in the migrated files
/// is also marked, and every marker sits on such a line.
#[test]
fn every_migrated_meshalgo_site_is_marked() -> Result<(), Box<dyn std::error::Error>> {
    let mut predicate_lines = 0usize;
    let mut marker_lines = 0usize;
    for file in MIGRATED {
        let source = read_source(file)?;
        for line in source.lines() {
            if line.contains("ctx.near_pt(")
                || line.contains("ctx.is_small_len(")
                || line.contains("ctx.is_small_ratio(")
                || line.contains("ctx.ratio_margin(")
            {
                predicate_lines += 1;
            }
            if line.contains("// BG-TOL-001:") {
                marker_lines += 1;
            }
        }
    }
    assert_eq!(
        predicate_lines, marker_lines,
        "predicate lines ({predicate_lines}) and marker lines ({marker_lines}) must match",
    );
    assert!(
        predicate_lines >= 20,
        "expected at least the 20 BG-TOL-001 migrated sites, saw {predicate_lines}",
    );
    Ok(())
}

/// Every deferred area site carries its FIXME, and none of the deferred files
/// introduce a `ToleranceCtx` — the load-bearing half that keeps the area
/// sites off the ratchet.
#[test]
fn deferred_area_sites_carry_a_fixme() -> Result<(), Box<dyn std::error::Error>> {
    for (file, expected) in DEFERRED {
        let source = read_source(file)?;
        let fixme_count = source
            .lines()
            .filter(|line| line.contains("FIXME(BG-TOL-001)"))
            .count();
        assert_eq!(
            fixme_count, expected,
            "{file} must carry exactly {expected} FIXME(BG-TOL-001) comments, saw {fixme_count}",
        );
        assert!(
            !source.contains("ToleranceCtx"),
            "{file} must not introduce a ToleranceCtx into an area comparison",
        );
    }
    Ok(())
}
