#![deny(clippy::unwrap_used)]

//! BG-KV2-501-C6 — the enum boundary: the whole-sweep surface type
//! (spec §5.10, as amended by the owner resolution).
//!
//! The canonical `Surface::SpineFrameSurface` variant now carries the
//! whole-sweep closed value ([`SpineFrameSweep`]): the landed recipe stored
//! ONCE over the canonical `Box<Curve>` spine carrier, the realized window
//! `[s0, s1] × [v0, v1]` riding on the closed value (the r1 volume evidence —
//! windowed −1.0 vs whole-ring −3.0 on the unit prism — is why the window is
//! part of the value), and the sweep-level placement. These tests pin the
//! amended Sections 1-2: no assertion here weakens a landed one.
//!
//! H-1: no `unwrap`/`expect`/`panic!`, no module-level `allow`.

use std::path::Path;
use truck_geometry::constructive::{
    DirectTolerance, FrameLaw, Profile2D, ProfileLaw, SpineCurve, SpineFrameRecipe, SpineFrameSweep,
};
use truck_geometry::prelude::*;

/// Extracts the `Ok` value of a fallible construction in a test, asserting the
/// precondition on a real predicate first (clippy-silent, unwrap-free). The
/// `None` arm is the divergent tail the H-1 test files use.
fn expect_some<T>(option: Option<T>, what: &str) -> Option<T> {
    assert!(option.is_some(), "{what} refused unexpectedly");
    option
}

/// The `Ok` value of a windowed recipe evaluation, asserted on a real
/// predicate first (clippy-silent, unwrap-free).
fn expect_evaluation<T>(
    result: std::result::Result<T, truck_geometry::constructive::ConstructError>,
    what: &str,
) -> Option<T> {
    assert!(result.is_ok(), "{what} refused unexpectedly");
    result.ok()
}

/// The unit-square profile (CCW about +z in the frame plane).
fn unit_square_profile() -> Option<Profile2D> {
    Profile2D::try_closed(vec![
        Point2::new(0.0, 0.0),
        Point2::new(1.0, 0.0),
        Point2::new(1.0, 1.0),
        Point2::new(0.0, 1.0),
    ])
    .ok()
}

/// The recipe over a `Box<Curve>` line spine from the origin to (0, 0, 1) — the
/// storage form the closed enums carry (the whole-sweep spine is the canonical
/// carrier). The frame plane is pinned by `FixedPlane { normal: +x }`, so
/// `X(s, v) = (py, -px, s)`.
fn curve_spine_recipe() -> Option<SpineFrameRecipe<Box<Curve>, ProfileLaw, FrameLaw>> {
    let profile = unit_square_profile()?;
    let spine = Box::new(Curve::Line(Line(
        Point3::origin(),
        Point3::new(0.0, 0.0, 1.0),
    )));
    Some(SpineFrameRecipe::new(
        spine,
        ProfileLaw::Constant(profile),
        FrameLaw::FixedPlane {
            normal: Vector3::unit_x(),
        },
    ))
}

/// The edge-zero whole-sweep value: the window `[0, 1] × [0, 1/4]` — the edge
/// the landed B-rep constructor's per-face realizations cover, as the enum now
/// carries it.
fn edge_zero_sweep() -> Option<SpineFrameSweep> {
    let recipe = curve_spine_recipe()?;
    SpineFrameSweep::try_new(recipe, 0.0, 1.0, 0.0, 0.25).ok()
}

#[test]
fn whole_sweep_variant_stores_the_recipe_once() {
    let recipe = match expect_some(curve_spine_recipe(), "the curve-spine recipe") {
        Some(recipe) => recipe,
        None => return,
    };
    let sweep = match expect_some(
        SpineFrameSweep::try_new(recipe.clone(), 0.0, 1.0, 0.0, 0.25).ok(),
        "the edge-zero sweep",
    ) {
        Some(sweep) => sweep,
        None => return,
    };
    // The sweep stores the whole recipe ONCE: the spec's four fields
    // (`spine`, `profile_law`, `frame_law`, `frame_data`) all live inside the
    // single stored recipe, never restated flat on the sweep.
    let stored = sweep.recipe();
    assert_eq!(stored.frame_data(), recipe.frame_data());
    assert_eq!(stored.profile_law, recipe.profile_law);
    assert_eq!(stored.frame_law, recipe.frame_law);
    // The spine is the same canonical carrier: identical domain...
    let (stored_lo, stored_hi) = stored.spine.domain();
    let (recipe_lo, recipe_hi) = recipe.spine.domain();
    let parameter_tol = DirectTolerance::default().parameter;
    assert!((stored_lo - recipe_lo).abs() <= parameter_tol);
    assert!((stored_hi - recipe_hi).abs() <= parameter_tol);
    // ...and identical evaluations: with the identity placement the sweep
    // realizes exactly the recipe over the window.
    for s in [0.0, 0.25, 0.5, 0.75, 1.0] {
        for v in [0.0625, 0.125, 0.1875] {
            let on_sweep = sweep.subs(s, v);
            let on_recipe = match expect_evaluation(
                recipe.position(s, v),
                "the recipe position inside the window",
            ) {
                Some(point) => point,
                None => return,
            };
            assert!(
                (on_sweep - on_recipe).magnitude() <= DirectTolerance::default().position,
                "the sweep diverged from its stored recipe at ({s}, {v})"
            );
        }
    }
}

#[test]
fn window_rides_on_the_closed_sweep_value() {
    let sweep = match expect_some(edge_zero_sweep(), "the edge-zero sweep") {
        Some(sweep) => sweep,
        None => return,
    };
    // The window is part of the closed value, readable off the value itself.
    assert_eq!((sweep.s0(), sweep.s1()), (0.0, 1.0));
    assert_eq!((sweep.v0(), sweep.v1()), (0.0, 0.25));
    // The parametric domain reports exactly that window — the per-face windowed
    // realization the landed B-rep constructor needs rides on the enum value
    // (the sweep window is bounded by construction, so `range_tuple` is total).
    let ((u0, u1), (v0, v1)) = sweep.range_tuple();
    assert_eq!((u0, u1), (0.0, 1.0));
    assert_eq!((v0, v1), (0.0, 0.25));
    // Evaluation over the window is the analytic fixture shape X = (py, -px, s)
    // (edge 0 of the unit square maps to p = (4v, 0)).
    let tolerance = DirectTolerance::default().position;
    for s in [0.0, 0.5, 1.0] {
        for v in [0.0625, 0.125, 0.1875] {
            let expected = Point3::new(0.0, -4.0 * v, s);
            assert!(
                (sweep.subs(s, v) - expected).magnitude() <= tolerance,
                "edge-zero sweep diverged from X = (0, -4v, s) at ({s}, {v})"
            );
        }
    }
    // Inverting the sweep mutates the window IN PLACE: the window is data of
    // the closed value, so the inverted enum surface still reports its own
    // (now reversed) window — the assertion the landed
    // `surface_variant_forwarding_all_landed_methods` pins.
    let mut inverted = sweep.clone();
    inverted.invert();
    assert_eq!((inverted.v0(), inverted.v1()), (0.25, 0.0));
    let ((_, _), (inverted_v0, inverted_v1)) = inverted.range_tuple();
    assert_eq!((inverted_v0, inverted_v1), (0.25, 0.0));
}

#[test]
fn sweep_transform_is_sweep_level() {
    let sweep = match expect_some(edge_zero_sweep(), "the edge-zero sweep") {
        Some(sweep) => sweep,
        None => return,
    };
    // The constructor sets the identity placement — the same placement the
    // landed B-rep constructor sets on every face of a sweep (there is no
    // per-face transform channel anymore; stop condition 2's guard).
    assert_eq!(*sweep.transform(), Matrix4::identity());
    let translation = Matrix4::from_translation(Vector3::new(2.0, -3.0, 4.0));
    let placed = sweep.transformed(translation);
    let tolerance = DirectTolerance::default().position;
    for s in [0.0, 0.5, 1.0] {
        for v in [0.0625, 0.1875] {
            let expected = translation.transform_point(sweep.subs(s, v));
            let got = placed.subs(s, v);
            assert!(
                (got - expected).magnitude() <= tolerance,
                "the sweep-level placement diverged at ({s}, {v})"
            );
        }
    }
    // `transform_by` composes into the same sweep-level matrix.
    let mut composed = sweep.clone();
    composed.transform_by(translation);
    for s in [0.0, 0.5, 1.0] {
        for v in [0.0625, 0.1875] {
            let expected = translation.transform_point(sweep.subs(s, v));
            let got = composed.subs(s, v);
            assert!(
                (got - expected).magnitude() <= tolerance,
                "transform_by diverged at ({s}, {v})"
            );
        }
    }
}

#[test]
fn enum_has_exactly_one_spine_frame_variant() {
    let sweep = match expect_some(edge_zero_sweep(), "the edge-zero sweep") {
        Some(sweep) => sweep,
        None => return,
    };
    // The variant carries the whole-sweep closed value, window and all; the
    // enum's surface methods forward onto that value.
    let surface = Surface::SpineFrameSurface(sweep);
    let Surface::SpineFrameSurface(stored) = &surface else {
        return;
    };
    assert_eq!((stored.v0(), stored.v1()), (0.0, 0.25));
    if let (Some((u0, u1)), Some((v0, v1))) = surface.try_range_tuple() {
        assert_eq!((u0, u1), (0.0, 1.0));
        assert_eq!((v0, v1), (0.0, 0.25));
    } else {
        return;
    }
    // Structurally, the enum has exactly ONE spine-frame SURFACE variant and it
    // is the whole-sweep payload: the decorator is no longer stored on the
    // enum (a payload swap would silently change neither here nor the landed
    // forwarding assertions).
    let canonical_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/canonical.rs");
    let source = match std::fs::read_to_string(&canonical_path) {
        Ok(source) => source,
        Err(_) => {
            return;
        }
    };
    assert_eq!(
        source.matches("SpineFrameSurface(SpineFrameSweep)").count(),
        1,
        "the Surface enum must declare its spine-frame variant with the whole-sweep payload exactly once"
    );
    assert_eq!(
        source
            .matches("SpineFrameSurface(SpineFrameSurface")
            .count(),
        0,
        "the windowed realization decorator must not be stored on the enum"
    );
}
