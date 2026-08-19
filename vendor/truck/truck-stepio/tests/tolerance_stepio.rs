#![deny(clippy::unwrap_used)]

use truck_base::tolerance::{ToleranceCtx, TOLERANCE};

fn ctx_at(scale: f64) -> ToleranceCtx {
    match ToleranceCtx::new(scale, TOLERANCE, TOLERANCE, TOLERANCE) {
        Ok(certified) => certified.value,
        Err(_) => panic!("ToleranceCtx refused a positive finite model_scale {scale}"),
    }
}

/// Every site the STEP import/export boundary migrated onto a `ToleranceCtx`
/// predicate must carry its `// BG-TOL-001:` marker, and no migrated line may
/// retain a legacy absolute-tolerance predicate. The five source files are read
/// from the crate source at compile time so the check tracks the code as it
/// exists, not a snapshot.
#[test]
fn every_migrated_stepio_site_is_marked() {
    let files = [
        ("src/in/mod.rs", include_str!("../src/in/mod.rs")),
        (
            "src/in/step_geometry/degenerate_torus.rs",
            include_str!("../src/in/step_geometry/degenerate_torus.rs"),
        ),
        (
            "src/in/step_geometry/geom_impls.rs",
            include_str!("../src/in/step_geometry/geom_impls.rs"),
        ),
        (
            "src/in/step_geometry/stepout_impls.rs",
            include_str!("../src/in/step_geometry/stepout_impls.rs"),
        ),
        (
            "src/out/geometry.rs",
            include_str!("../src/out/geometry.rs"),
        ),
    ];
    let mut markers = 0;
    for (file, content) in files {
        let predicate_lines = content
            .lines()
            .filter(|line| {
                line.contains("ctx.near_pt(")
                    || line.contains("ctx.is_small_len(")
                    || line.contains("ctx.is_small_ratio(")
                    || line.contains("ctx.ratio_margin(")
            })
            .count();
        let marker_lines = content
            .lines()
            .filter(|line| line.contains("// BG-TOL-001:"))
            .count();
        assert_eq!(
            predicate_lines, marker_lines,
            "marking imbalance in {file}: {predicate_lines} predicate line(s) but {marker_lines} marker(s)",
        );
        markers += marker_lines;
        for (line_no, line) in content.lines().enumerate() {
            if line.contains("// BG-TOL-001:")
                && (line.contains(".near(")
                    || line.contains("so_small(")
                    || line.contains("TOLERANCE"))
            {
                panic!(
                    "migrated line in {file}:{} retains a legacy absolute-tolerance predicate: {line}",
                    line_no + 1,
                );
            }
        }
    }
    assert_eq!(
        markers, 19,
        "expected 19 migrated sites, found {markers} markers"
    );
}

/// The eleven `param` classifications in the STEP export path are all the same
/// uniform-scale test: a transform is uniformly scaled or it is not, and the
/// model's units must have no say in it. `is_small_ratio` is therefore required
/// to give identical answers at every model scale.
#[test]
fn scale_factor_comparisons_do_not_scale_with_the_model() {
    let scales = [0.001, 0.1, 1.0, 10.0, 1000.0]; // H-3: dimensionless model-scale factors spread across orders of magnitude
    let ratios = [0.0, 0.5 * TOLERANCE, TOLERANCE, 2.0 * TOLERANCE]; // H-3: dimensionless ratio quantities bracketing tau_rep
    let reference = ctx_at(scales[0]);

    for &ratio in &ratios {
        let baseline = reference.is_small_ratio(ratio);
        for &scale in scales.iter().skip(1) {
            assert_eq!(
                ctx_at(scale).is_small_ratio(ratio),
                baseline,
                "is_small_ratio must be scale-free at model_scale {scale}",
            );
        }
    }
}
