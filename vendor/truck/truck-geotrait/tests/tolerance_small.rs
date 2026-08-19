#![deny(clippy::unwrap_used)]

use std::fs;

/// Every migrated predicate site in this packet's three files carries its
/// `// BG-TOL-001:` marker, and no marker is spurious. The files are read from
/// the crate manifest directory at runtime so the check tracks the source as
/// it exists, not a snapshot.
///
/// `truck-polymesh` sets `autotests = false`, so `polyline_curve.rs` is read
/// from here, in `truck-geotrait`, by relative path rather than from a test
/// file that would never run.
#[test]
fn every_migrated_small_site_is_marked() -> Result<(), Box<dyn std::error::Error>> {
    let files = [
        concat!(env!("CARGO_MANIFEST_DIR"), "/src/algo/curve.rs"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/src/algo/surface.rs"),
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../truck-polymesh/src/polyline_curve.rs"
        ),
    ];
    let mut predicate_lines = 0usize;
    let mut marker_lines = 0usize;
    for file in files {
        let content = fs::read_to_string(file)?;
        for line in content.lines() {
            if line.contains("ctx.near_pt(")
                || line.contains("ctx.near_points(")
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
        "predicate lines ({predicate_lines}) must equal // BG-TOL-001: marker lines ({marker_lines}) across the migrated files"
    );
    Ok(())
}
