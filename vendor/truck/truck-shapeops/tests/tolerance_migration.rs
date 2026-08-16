#![deny(clippy::unwrap_used)]

use std::fs;
use std::path::Path;

use truck_base::tolerance::{ToleranceCtx, TOLERANCE};

fn ctx_at(scale: f64) -> ToleranceCtx {
    match ToleranceCtx::new(scale, TOLERANCE, TOLERANCE, TOLERANCE) {
        Ok(certified) => certified.value,
        Err(_) => ToleranceCtx::unscaled_legacy(),
    }
}

/// Every site that `truck-shapeops` migrated onto a `ToleranceCtx` predicate
/// must carry its `// BG-TOL-001:` marker, and no marker may be spurious. The
/// five files are read from the crate manifest directory at runtime so the
/// check tracks the source as it exists, not a snapshot.
#[test]
fn every_migrated_shapeops_site_is_marked() {
    let files = [
        "src/fillet/mod.rs",
        "src/healing/split_closed_faces.rs",
        "src/transversal/loops_store/mod.rs",
        "src/transversal/intersection_curve/mod.rs",
        "src/transversal/polyline_construction/mod.rs",
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
                            || line.contains("ctx.is_small_len(")
                            || line.contains("ctx.is_small_ratio(")
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

/// `is_small_ratio` compares a dimensionless quantity and must be identical at
/// every model scale; `is_small_len` compares a model-space length and must
/// change with the scale. This is the invariant every `param` classification
/// in the BG-TOL-001 site table depends on.
#[test]
fn param_sites_are_unaffected_by_model_scale() {
    let scales = [0.001, 0.1, 1.0, 10.0, 1000.0]; // H-3: dimensionless model-scale factors spread across orders of magnitude
    let ratios = [0.0, 0.5 * TOLERANCE, TOLERANCE, 2.0 * TOLERANCE]; // H-3: dimensionless ratio quantities bracketing tau_rep
    let lengths = [0.5 * TOLERANCE, TOLERANCE, 2.0 * TOLERANCE]; // H-3: model-space length quantities bracketing tau_rep * model_scale
    let reference = ctx_at(scales[0]);

    for &ratio in &ratios {
        let baseline = reference.is_small_ratio(ratio);
        for &scale in scales.iter().skip(1) {
            assert_eq!(
                ctx_at(scale).is_small_ratio(ratio),
                baseline,
                "is_small_ratio must be scale-free at model_scale {scale}"
            );
        }
    }

    let differs = lengths.iter().any(|&length| {
        let baseline = reference.is_small_len(length);
        scales
            .iter()
            .skip(1)
            .any(|&scale| ctx_at(scale).is_small_len(length) != baseline)
    });
    assert!(differs, "is_small_len must scale with the model");
}
