#![deny(clippy::unwrap_used)]

//! BG-CG-000-CONTRACT — the constructive geometry contract skeleton tests.
//! These PIN the contract shapes; later CG packets must not change them
//! without these failing.

use truck_base::tolerance::TOLERANCE;
use truck_geometry::base::*;
use truck_geometry::constructive::*;

#[test]
fn frame3_try_new_accepts_right_handed_basis() {
    let t = Vector3::new(1.0, 0.0, 0.0);
    let n = Vector3::new(0.0, 1.0, 0.0);
    let b = Vector3::new(0.0, 0.0, 1.0);
    assert!(matches!(
        Frame3::try_new(t, n, b),
        Ok(Frame3 { tangent, normal, binormal })
            if tangent == t && normal == n && binormal == b
    ));
}

#[test]
fn frame3_try_new_rejects_left_handed_basis() {
    let result = Frame3::try_new(
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(0.0, 0.0, -1.0),
    );
    assert!(matches!(result, Err(ConstructError::InvalidInput)));
}

#[test]
fn frame3_try_new_rejects_non_orthonormal_basis() {
    let non_orthogonal = Frame3::try_new(
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
    );
    assert!(matches!(non_orthogonal, Err(ConstructError::InvalidInput)));

    let non_unit = Frame3::try_new(
        Vector3::new(2.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
    );
    assert!(matches!(non_unit, Err(ConstructError::InvalidInput)));
}

#[test]
fn frame3_law_names_are_stable() {
    assert_eq!(
        FrameLaw::FixedPlane {
            normal: Vector3::unit_z()
        }
        .law_name(),
        "FixedPlane"
    );
    assert_eq!(
        FrameLaw::ArchitecturalUp {
            up: Vector3::unit_z()
        }
        .law_name(),
        "ArchitecturalUp"
    );
    assert_eq!(
        FrameLaw::ParallelTransport {
            initial_normal: Vector3::unit_z()
        }
        .law_name(),
        "ParallelTransport"
    );
    assert_eq!(
        FrameLaw::RadialAboutAxis {
            origin: Point3::new(0.0, 0.0, 0.0),
            axis: Vector3::unit_z(),
        }
        .law_name(),
        "RadialAboutAxis"
    );
}

#[test]
fn profile2d_try_closed_rejects_structurally_invalid() {
    let too_few = Profile2D::try_closed(vec![Point2::new(0.0, 0.0), Point2::new(1.0, 0.0)]);
    assert!(matches!(too_few, Err(ConstructError::InvalidInput)));

    let non_finite = Profile2D::try_closed(vec![
        Point2::new(0.0, 0.0),
        Point2::new(1.0, 0.0),
        Point2::new(f64::NAN, 1.0),
    ]);
    assert!(matches!(non_finite, Err(ConstructError::InvalidInput)));
}

#[test]
fn profile_law_linear_correspondence_rejects_count_mismatch() {
    let triangle = Profile2D {
        vertices: vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.0, 1.0),
        ],
    };
    let quad = Profile2D {
        vertices: vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ],
    };
    let result = ProfileLaw::try_linear_correspondence(triangle, quad);
    assert!(matches!(
        result,
        Err(ConstructError::ProfileCorrespondenceMismatch)
    ));
}

#[test]
fn scalar_law_linear_interpolates() {
    let law = ScalarLaw::Linear {
        start: 1.0,
        end: 3.0,
    };
    assert_eq!(law.at(0.5), 2.0);
    assert_eq!(law.at(0.0), 1.0);
    assert_eq!(law.at(1.0), 3.0);
}

#[test]
fn direct_tolerance_defaults_derive_from_truck_base() {
    let t = DirectTolerance::default();
    assert_eq!(t.position, TOLERANCE);
    assert_eq!(t.parameter, TOLERANCE);
    assert_eq!(t.jacobian, TOLERANCE);
    assert_eq!(t.intersection, TOLERANCE);
}

#[test]
fn construct_error_display_names_law_and_parameter() {
    let err = ConstructError::FrameSingular {
        at: 0.5,
        law: "ArchitecturalUp",
    };
    let display = err.to_string();
    assert!(display.contains("ArchitecturalUp"));
    assert!(display.contains("0.5"));
}

#[test]
fn recipe_evaluators_refuse_while_stub() {
    // BG-CG-002-FRAMES-ANALYTIC (r2): the frame step landed — asserted positively.
    let triangle = Profile2D {
        vertices: vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.0, 1.0),
        ],
    };
    let profile_law = ProfileLaw::Constant(triangle);
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
    let tol = DirectTolerance::default().position;
    assert_eq!(recipe.profile(0.5, 0.25), profile_law.evaluate(0.5, 0.25));
    let frame_ok = match recipe.frame(0.5) {
        Ok(f) => {
            (f.tangent.magnitude() - 1.0).abs() <= tol
                && (f.normal.magnitude() - 1.0).abs() <= tol
                && (f.binormal.magnitude() - 1.0).abs() <= tol
                && (f.tangent.cross(f.normal) - f.binormal).magnitude() <= tol
        }
        Err(_) => false,
    };
    assert!(frame_ok, "frame is not Ok, unit-length, and right-handed");
    let c = Point3::new(0.5, 0.0, 0.0);
    let t = Vector3::new(1.0, 0.0, 0.0);
    let n = Vector3::new(0.0, 1.0, 0.0);
    let p = Point2::new(0.75, 0.0);
    let expected = c + t * p.x + n * p.y;
    assert!(matches!(
        recipe.position(0.5, 0.25),
        Ok(x) if (x - expected).magnitude() <= tol
    ));
}

#[test]
fn sampling_policy_resolve_refuses_while_stub() {
    // BG-CG-001-RECIPE: in-place amendment — UniformCount and CustomParameters
    // now resolve; ChordTolerance/AngularTolerance still refuse in CG-001.
    let n = 4usize;
    let expected_uniform: Vec<f64> = (0..n)
        .map(|i| 0.0 + (1.0 - 0.0) * (i as f64) / ((n - 1) as f64))
        .collect();
    assert_eq!(
        SamplingPolicy::UniformCount { spine: 4 }.resolve(0.0, 1.0),
        Ok(expected_uniform)
    );
    assert_eq!(
        SamplingPolicy::CustomParameters(vec![0.0, 1.0]).resolve(0.0, 1.0),
        Ok(vec![0.0, 1.0])
    );
    assert!(matches!(
        SamplingPolicy::ChordTolerance(0.1).resolve(0.0, 1.0),
        Err(ConstructError::InvalidInput)
    ));
    assert!(matches!(
        SamplingPolicy::AngularTolerance(0.1).resolve(0.0, 1.0),
        Err(ConstructError::InvalidInput)
    ));
}
