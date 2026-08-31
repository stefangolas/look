#![deny(clippy::unwrap_used)]

//! BG-CG-001-RECIPE — the spine trait, profile evaluation, and the C¹
//! refusals: behavior tests for the filled evaluators and the spine surface.

use truck_geometry::base::*;
use truck_geometry::constructive::*;

#[test]
fn line_spine_domain_position_and_derivative() {
    let start = Point3::new(0.0, 0.0, 0.0);
    let end = Point3::new(1.0, 0.0, 0.0);
    let spine = LineSpine { start, end };
    assert_eq!(spine.domain(), (0.0, 1.0));
    assert_eq!(spine.position_at(0.25), Ok(start + (end - start) * 0.25));
    assert_eq!(spine.derivative_at(0.0), Ok(end - start));
    assert_eq!(spine.derivative_at(1.0), Ok(end - start));
}

#[test]
fn polyline_spine_derivative_refuses_at_corners() {
    let spine = PolylineSpine {
        vertices: vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
        ],
    };
    assert!(matches!(
        spine.derivative_at(1.0),
        Err(ConstructError::SpineNotC1 { at: 1.0 })
    ));
    let first = Point3::new(1.0, 0.0, 0.0) - Point3::new(0.0, 0.0, 0.0);
    let second = Point3::new(1.0, 1.0, 0.0) - Point3::new(1.0, 0.0, 0.0);
    assert_eq!(spine.derivative_at(0.5), Ok(first));
    assert_eq!(spine.derivative_at(1.5), Ok(second));
}

#[test]
fn polyline_spine_out_of_domain_refuses() {
    let spine = PolylineSpine {
        vertices: vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
        ],
    };
    assert!(matches!(
        spine.position_at(-0.5),
        Err(ConstructError::InvalidInput)
    ));
    assert!(matches!(
        spine.position_at(2.5),
        Err(ConstructError::InvalidInput)
    ));
    assert_eq!(spine.position_at(0.0), Ok(Point3::new(0.0, 0.0, 0.0)));
    assert_eq!(spine.position_at(2.0), Ok(Point3::new(1.0, 1.0, 0.0)));
}

#[test]
fn profile_constant_evaluates_vertices_and_edges() {
    let quad = Profile2D {
        vertices: vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ],
    };
    let law = ProfileLaw::Constant(quad);
    for s in [0.0, 0.5, 1.0] {
        assert_eq!(law.evaluate(s, 0.0), Ok(Point2::new(0.0, 0.0)));
        assert_eq!(law.evaluate(s, 0.25), Ok(Point2::new(1.0, 0.0)));
        assert_eq!(law.evaluate(s, 0.5), Ok(Point2::new(1.0, 1.0)));
        assert_eq!(law.evaluate(s, 0.75), Ok(Point2::new(0.0, 1.0)));
        assert_eq!(law.evaluate(s, 1.0), Ok(Point2::new(0.0, 0.0)));
        assert_eq!(law.evaluate(s, 0.125), Ok(Point2::new(0.5, 0.0)));
    }
}

#[test]
fn profile_scale_interpolates_and_collapses_through_zero() {
    let quad = Profile2D {
        vertices: vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ],
    };
    let scaled = ProfileLaw::Scale {
        profile: quad.clone(),
        scale: ScalarLaw::Linear {
            start: 1.0,
            end: 3.0,
        },
    };
    assert_eq!(scaled.evaluate(0.5, 0.25), Ok(Point2::new(2.0, 0.0)));

    let collapsing = ProfileLaw::Scale {
        profile: quad.clone(),
        scale: ScalarLaw::Linear {
            start: 1.0,
            end: -1.0,
        },
    };
    assert!(matches!(
        collapsing.evaluate(0.5, 0.25),
        Err(ConstructError::ProfileCollapse { at: 0.5 })
    ));

    let mirrored = ProfileLaw::Scale {
        profile: quad,
        scale: ScalarLaw::Constant(-1.0),
    };
    assert_eq!(mirrored.evaluate(0.5, 0.25), Ok(Point2::new(-1.0, 0.0)));
}

#[test]
fn profile_linear_correspondence_interpolates_vertexwise() {
    let triangle = Profile2D {
        vertices: vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.0, 1.0),
        ],
    };
    let translated = Profile2D {
        vertices: vec![
            Point2::new(1.0, 0.0),
            Point2::new(2.0, 0.0),
            Point2::new(1.0, 1.0),
        ],
    };
    let law = ProfileLaw::LinearCorrespondence {
        start: triangle,
        end: translated,
    };
    assert_eq!(law.evaluate(0.5, 0.0), Ok(Point2::new(0.5, 0.0)));
    assert_eq!(law.evaluate(0.5, 1.0 / 3.0), Ok(Point2::new(1.5, 0.0)));
    assert_eq!(law.evaluate(0.5, 2.0 / 3.0), Ok(Point2::new(0.5, 1.0)));
}

#[test]
fn profile_evaluation_refuses_nonfinite_parameters() {
    let quad = Profile2D {
        vertices: vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ],
    };
    let law = ProfileLaw::Constant(quad);
    assert!(matches!(
        law.evaluate(f64::NAN, 0.5),
        Err(ConstructError::NonFinite { .. })
    ));
    assert!(matches!(
        law.evaluate(0.5, -0.5),
        Err(ConstructError::InvalidInput)
    ));
    assert!(matches!(
        law.evaluate(0.5, 1.5),
        Err(ConstructError::InvalidInput)
    ));
}

#[test]
fn recipe_profile_evaluation_matches_profile_law() {
    let quad = Profile2D {
        vertices: vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ],
    };
    let profile_law = ProfileLaw::Constant(quad);
    let recipe = SpineFrameRecipe::new(
        LineSpine {
            start: Point3::new(0.0, 0.0, 0.0),
            end: Point3::new(1.0, 0.0, 0.0),
        },
        profile_law.clone(),
        FrameLaw::FixedPlane {
            normal: Vector3::unit_z(),
        },
    );
    for (s, v) in [(0.0, 0.0), (0.5, 0.25), (1.0, 0.5), (0.25, 1.0)] {
        assert_eq!(recipe.profile(s, v), profile_law.evaluate(s, v));
    }
}

#[test]
fn recipe_position_refuses_until_frames_land() {
    // CG-002 amends this test in place when FixedPlane makes the frame step
    // succeed.
    let quad = Profile2D {
        vertices: vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ],
    };
    let recipe = SpineFrameRecipe::new(
        LineSpine {
            start: Point3::new(0.0, 0.0, 0.0),
            end: Point3::new(1.0, 0.0, 0.0),
        },
        ProfileLaw::Constant(quad),
        FrameLaw::FixedPlane {
            normal: Vector3::unit_z(),
        },
    );
    assert!(recipe.position(0.5, 0.25).is_err());
}

#[test]
fn recipe_position_evaluates_profile_before_frame() {
    let quad = Profile2D {
        vertices: vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ],
    };
    let recipe = SpineFrameRecipe::new(
        LineSpine {
            start: Point3::new(0.0, 0.0, 0.0),
            end: Point3::new(1.0, 0.0, 0.0),
        },
        ProfileLaw::Scale {
            profile: quad,
            scale: ScalarLaw::Constant(0.0),
        },
        FrameLaw::FixedPlane {
            normal: Vector3::unit_z(),
        },
    );
    assert!(matches!(
        recipe.position(0.5, 0.25),
        Err(ConstructError::ProfileCollapse { .. })
    ));
}

#[test]
fn sampling_uniform_count_resolves_inclusive_endpoints() {
    let n = 4usize;
    let expected: Vec<f64> = (0..n)
        .map(|i| 0.0 + (1.0 - 0.0) * (i as f64) / ((n - 1) as f64))
        .collect();
    assert_eq!(
        SamplingPolicy::UniformCount { spine: 4 }.resolve(0.0, 1.0),
        Ok(expected)
    );
    assert!(matches!(
        SamplingPolicy::UniformCount { spine: 1 }.resolve(0.0, 1.0),
        Err(ConstructError::InvalidInput)
    ));
    assert!(matches!(
        SamplingPolicy::UniformCount { spine: 4 }.resolve(1.0, 0.0),
        Err(ConstructError::InvalidInput)
    ));
}

#[test]
fn sampling_custom_parameters_sorts_and_dedupes() {
    assert_eq!(
        SamplingPolicy::CustomParameters(vec![1.0, 0.0, 0.5, 0.0]).resolve(7.0, 9.0),
        Ok(vec![0.0, 0.5, 1.0])
    );
}

#[test]
fn sampling_tolerance_variants_still_refuse_in_cg001() {
    assert!(matches!(
        SamplingPolicy::ChordTolerance(0.1).resolve(0.0, 1.0),
        Err(ConstructError::InvalidInput)
    ));
    assert!(matches!(
        SamplingPolicy::AngularTolerance(0.1).resolve(0.0, 1.0),
        Err(ConstructError::InvalidInput)
    ));
}
