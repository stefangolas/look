//! BG-TOL-001-GEOM-SPECIFIEDS tolerance migration tests: the two invariants
//! every `param` and `model` classification in the specifieds site table rests
//! on.
//!
//! `circle.rs`, `hyperbola.rs` and `parabola.rs` compare canonical-frame
//! quantities: the `PhantomData` primitives carry no geometry and live in a
//! frame whose characteristic radius is 1 by construction, so every distance
//! there is a dimensionless multiple of that unit and must NOT move with the
//! model. `line.rs`, `plane.rs`, `sphere.rs` and `torus.rs` compare model-space
//! lengths that MUST move with the model.

#![deny(clippy::unwrap_used)]

use truck_geometry::prelude::*;

fn ctx_at(scale: f64) -> ToleranceCtx {
    match ToleranceCtx::new(scale, TOLERANCE, TOLERANCE, TOLERANCE) {
        Ok(certified) => certified.value,
        Err(_) => ToleranceCtx::unscaled_legacy(),
    }
}

/// The invariant every `param` classification in `circle.rs`, `hyperbola.rs`
/// and `parabola.rs` rests on: a quantity that lives in the canonical frame is
/// dimensionless, so `ratio_margin` and `is_small_ratio` give identical
/// answers at every `model_scale`. If this ever fails, someone has scaled a
/// canonical quantity.
#[test]
fn canonical_sites_do_not_scale_with_the_model() {
    let scales = [0.001, 0.1, 1.0, 10.0, 1000.0]; // H-3: dimensionless model-scale factors spread across orders of magnitude
    let ratios = [0.0, 0.5 * TOLERANCE, TOLERANCE, 2.0 * TOLERANCE]; // H-3: dimensionless ratio quantities bracketing tau_rep
    let reference = ctx_at(1.0);

    for &scale in &scales {
        let c = ctx_at(scale);
        assert_eq!(
            c.ratio_margin(),
            reference.ratio_margin(),
            "ratio_margin must be scale-free at model_scale {scale}"
        );
        for &ratio in &ratios {
            assert_eq!(
                c.is_small_ratio(ratio),
                reference.is_small_ratio(ratio),
                "is_small_ratio must be scale-free at model_scale {scale}, ratio {ratio}"
            );
        }
    }
}

/// The converse for the model-space primitives `line.rs`, `plane.rs`,
/// `sphere.rs` and `torus.rs`: `length_margin` and `is_small_len` change with
/// `model_scale`, and a fixed separation that is "small" at a large scale is
/// not small at a small one.
#[test]
fn model_space_sites_do_scale_with_the_model() {
    let scales = [0.001, 0.1, 1.0, 10.0, 1000.0]; // H-3: dimensionless model-scale factors spread across orders of magnitude
    let lengths = [0.5 * TOLERANCE, TOLERANCE, 2.0 * TOLERANCE]; // H-3: model-space length quantities bracketing tau_rep * model_scale
    let reference = ctx_at(1.0);

    for &scale in &scales {
        let c = ctx_at(scale);
        assert_eq!(
            c.length_margin(),
            scale * reference.length_margin(),
            "length_margin must scale with the model at model_scale {scale}"
        );
        for &length in &lengths {
            assert_eq!(
                c.is_small_len(scale * length),
                reference.is_small_len(length),
                "is_small_len must scale with the model at scale {scale}, length {length}"
            );
        }
    }

    let fixed = TOLERANCE; // H-3: a fixed model-space separation equal to the legacy epsilon
    assert!(ctx_at(1000.0).is_small_len(fixed));
    assert!(!ctx_at(0.001).is_small_len(fixed));
}
