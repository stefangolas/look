#![deny(clippy::unwrap_used)]

use std::fs;
use std::path::Path;

/// Every line that migrates a tolerance site in the `decorators` module onto a
/// `ToleranceCtx` predicate must carry its `// BG-TOL-001:` marker, and every
/// marker must be on a predicate line, so the marker count tracks the site
/// count as the source evolves. The six migrated source files are read from
/// the crate manifest directory at runtime so the check follows the source as
/// it exists, not a snapshot.
#[test]
fn every_migrated_decorators_site_is_marked() {
    let files = [
        "src/decorators/intersection_curve.rs",
        "src/decorators/offset/curve.rs",
        "src/decorators/offset/surface.rs",
        "src/decorators/rbf_surface/algo.rs",
        "src/decorators/rbf_surface/contact_circle.rs",
        "src/decorators/revolved_curve.rs",
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

/// The deferred dimension site — the `debug_assert!(del.z.so_small(), ..)` in
/// `next_point`, where `del.z` carries dimension 1/length under a model
/// rescale — carries exactly one `FIXME(BG-TOL-001, DIMENSION)` marker and is
/// not migrated: the `next_point` function contains no `ToleranceCtx`.
/// `try_new` legitimately migrates one model site, so the file as a whole does
/// contain a context; the load-bearing half is that the deferred function
/// carries none.
#[test]
fn the_deferred_dimension_site_carries_a_fixme() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/decorators/rbf_surface/contact_circle.rs");
    let (fixme_lines, next_point) = match fs::read_to_string(&path) {
        Ok(content) => (
            content
                .lines()
                .filter(|line| line.contains("FIXME(BG-TOL-001, DIMENSION)"))
                .count(),
            content
                .split("fn next_point")
                .nth(1)
                .unwrap_or("")
                .to_string(),
        ),
        Err(_) => (0usize, String::new()),
    };
    assert_eq!(
        fixme_lines, 1,
        "expected exactly one FIXME(BG-TOL-001, DIMENSION) marker"
    );
    assert!(
        !next_point.contains("ToleranceCtx"),
        "the deferred next_point site must not be migrated"
    );
}
