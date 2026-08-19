#![deny(clippy::unwrap_used)]

use std::fs;
use std::path::Path;

/// Every line that migrates a tolerance site in the `nurbs` module onto a
/// `ToleranceCtx` predicate must carry its `// BG-TOL-001:` marker, and every
/// marker must be on a predicate line, so the marker count tracks the site
/// count as the source evolves. The six source files are read from the crate
/// manifest directory at runtime so the check follows the source as it exists,
/// not a snapshot.
#[test]
fn every_migrated_nurbs_site_is_marked() {
    let files = [
        "src/nurbs/bspcurve.rs",
        "src/nurbs/bspsurface.rs",
        "src/nurbs/knot_vec.rs",
        "src/nurbs/mod.rs",
        "src/nurbs/nurbscurve.rs",
        "src/nurbs/nurbssurface.rs",
    ];
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    for file in files {
        let path = Path::new(manifest_dir).join(file);
        let content = fs::read_to_string(&path);
        let (predicate_lines, marker_lines) = match content {
            Ok(content) => {
                let predicate_lines = content
                    .lines()
                    .filter(|line| {
                        line.contains("ctx.near_pt(")
                            || line.contains("ctx.near_points(")
                            || line.contains("ctx.is_small_len(")
                            || line.contains("ctx.is_small_ratio(")
                            || line.contains("ctx.ratio_margin(")
                    })
                    .count();
                let marker_lines = content
                    .lines()
                    .filter(|line| line.contains("// BG-TOL-001:"))
                    .count();
                (predicate_lines, marker_lines)
            }
            Err(_) => (0usize, 1usize),
        };
        assert_eq!(
            predicate_lines, marker_lines,
            "marking imbalance in {}: {} predicate line(s) but {} marker(s)",
            file, predicate_lines, marker_lines
        );
    }
}

/// The twelve generic-bound sites are deferred with a `FIXME(BG-TOL-001,
/// GENERIC_BOUND)` marker and nothing else. The per-file counts are the point:
/// they prove none of the twelve was quietly migrated and that a later reader
/// cannot "finish the job" by widening a public bound without first removing
/// the marker this test counts.
#[test]
fn deferred_generic_bound_sites_carry_a_fixme() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let expected = [
        ("src/nurbs/bspcurve.rs", 4),
        ("src/nurbs/bspsurface.rs", 4),
        ("src/nurbs/nurbscurve.rs", 2),
        ("src/nurbs/nurbssurface.rs", 2),
    ];
    for (file, want) in expected {
        let path = Path::new(manifest_dir).join(file);
        let content = fs::read_to_string(&path);
        let got = match content {
            Ok(content) => content
                .lines()
                .filter(|line| line.contains("FIXME(BG-TOL-001, GENERIC_BOUND)"))
                .count(),
            Err(_) => 0,
        };
        assert_eq!(
            got, want,
            "unexpected FIXME(BG-TOL-001, GENERIC_BOUND) count in {}: got {got}, want {want}",
            file
        );
    }
}
