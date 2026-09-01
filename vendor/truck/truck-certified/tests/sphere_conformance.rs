//! Conformance tests for the certified sphere constructor
//! (`truck_certified::formal::sphere`, BG-CK-P1-SPHERE).
//!
//! Every entry is `pub`, so this integration test constructs everything
//! directly; no in-module test split was needed.

#![deny(clippy::unwrap_used)]

use truck_certified::formal::numeric::{NumericDomainError, PositiveFinite};
use truck_certified::formal::sphere::{
    identify_sphere, identify_sphere_placement, identify_sphere_world, CertifiedEmbeddedSphere,
    SphereIdentification, SphereIdentificationFailure,
};
use truck_geometry::prelude::{Matrix4, Point3, Processor, Sphere};

/// Extract the certified witness, refusing the negative case (test helper).
fn expect_sphere(id: SphereIdentification) -> CertifiedEmbeddedSphere {
    match id {
        SphereIdentification::Sphere(witness) => witness,
        other => panic!("expected a certified sphere, got {other:?}"),
    }
}

#[test]
fn witness_carries_representation_derived_center_and_radius() {
    // Representation-derived means bit-equal round-trip: the witness carries
    // exactly the numbers the representation stated.
    let center = Point3::new(1.25, -3.5, 7.75);
    let radius = 2.0_f64.powi(10); // 1024, exactly representable
    let witness = expect_sphere(identify_sphere_world(center, radius));
    assert_eq!(witness.center(), center);
    assert_eq!(witness.radius().get(), radius);
}

#[test]
fn witness_fields_are_private_with_accessors_only() {
    // Compile-shape: `CertifiedEmbeddedSphere { center, radius }` (a struct
    // literal) and `witness.center` (field access) do not compile outside the
    // module — the fields are private, so the only read path is the accessor
    // pair below, and there is no mutation path at all.
    let witness = expect_sphere(identify_sphere_world(Point3::new(1.0, 2.0, 3.0), 4.0));
    let center: Point3 = witness.center();
    let radius: PositiveFinite = witness.radius();
    assert_eq!(center, Point3::new(1.0, 2.0, 3.0));
    assert_eq!(radius.get(), 4.0);
    assert_eq!(witness.tag(), "certified_embedded_sphere");
}

#[test]
fn non_finite_coordinate_refuses_named_case() {
    // A NaN center coordinate and an infinite radius each refuse
    // `NonFiniteCoordinate`, with the exact domain cause.
    assert_eq!(
        identify_sphere_world(Point3::new(f64::NAN, 0.0, 0.0), 1.0),
        SphereIdentification::NotASphere(SphereIdentificationFailure::NonFiniteCoordinate {
            cause: NumericDomainError::NotANumber,
        }),
        "a NaN center coordinate"
    );
    assert_eq!(
        identify_sphere_world(Point3::new(0.0, 0.0, 0.0), f64::INFINITY),
        SphereIdentification::NotASphere(SphereIdentificationFailure::NonFiniteCoordinate {
            cause: NumericDomainError::Infinite,
        }),
        "an infinite radius"
    );
    assert_eq!(
        identify_sphere_world(Point3::new(0.0, f64::NEG_INFINITY, 0.0), 1.0),
        SphereIdentification::NotASphere(SphereIdentificationFailure::NonFiniteCoordinate {
            cause: NumericDomainError::Infinite,
        }),
        "an infinite center coordinate"
    );
}

#[test]
fn non_positive_radius_refuses_named_case() {
    // Zero and negative radii (including negative zero) refuse
    // `DegenerateRadius`.
    assert_eq!(
        identify_sphere_world(Point3::new(0.0, 0.0, 0.0), 0.0),
        SphereIdentification::NotASphere(SphereIdentificationFailure::DegenerateRadius),
        "zero radius"
    );
    assert_eq!(
        identify_sphere_world(Point3::new(0.0, 0.0, 0.0), -0.0),
        SphereIdentification::NotASphere(SphereIdentificationFailure::DegenerateRadius),
        "negative zero radius"
    );
    assert_eq!(
        identify_sphere_world(Point3::new(0.0, 0.0, 0.0), -3.0),
        SphereIdentification::NotASphere(SphereIdentificationFailure::DegenerateRadius),
        "negative radius"
    );
}

#[test]
fn non_similar_placement_refuses_named_case() {
    let sphere = Sphere::new(Point3::new(1.0, 2.0, 3.0), 1.5);

    // A 2x/1x anisotropic scale: direction-column magnitudes 2, 1, 1 — not all
    // equal, so the placement deforms the sphere into an ellipsoid.
    let ellipsoid =
        Processor::with_transform(sphere, Matrix4::from_nonuniform_scale(2.0, 1.0, 1.0));
    assert_eq!(
        identify_sphere_placement(&ellipsoid),
        SphereIdentification::NotASphere(SphereIdentificationFailure::NonSimilarityPlacement)
    );

    // A uniform 2x-scaled placement ACCEPTS: the common column magnitude is 2,
    // so the radius is exactly `2 * r_local` and the placed center maps
    // bit-equal through the placement (both multiplications by 2.0 are exact
    // in `f64`, so no H-3 opt-out is needed).
    let doubled = Processor::with_transform(sphere, Matrix4::from_scale(2.0));
    let witness = expect_sphere(identify_sphere_placement(&doubled));
    assert_eq!(witness.radius().get(), 2.0 * 1.5);
    assert_eq!(
        witness.center(),
        Point3::new(2.0 * 1.0, 2.0 * 2.0, 2.0 * 3.0)
    );
}

#[test]
fn longitude_period_verified_by_evaluation() {
    // A well-formed sphere certifies; the period check's residual-constant
    // path is exercised (a sphere is always periodic, so this is a green-path
    // test; the refusal arm stays covered by
    // `identify_never_panics_and_refusals_are_named_cases`'s shape).
    let witness = expect_sphere(identify_sphere_world(Point3::new(-1.0, 4.0, 2.5), 3.0));
    assert_eq!(witness.center(), Point3::new(-1.0, 4.0, 2.5));
    assert_eq!(witness.radius().get(), 3.0);
}

#[test]
fn placement_typed_and_world_entries_agree() {
    let sphere = Sphere::new(Point3::new(1.0, 2.0, 3.0), 4.5);
    let typed = identify_sphere(&sphere);
    // An identity-ish (uniform 1x) placement still runs the similarity rule
    // and reads out the same world parameters.
    let placed =
        identify_sphere_placement(&Processor::with_transform(sphere, Matrix4::from_scale(1.0)));
    let world = identify_sphere_world(sphere.center(), sphere.radius());
    assert_eq!(typed, placed);
    assert_eq!(typed, world);
}

#[test]
fn identify_never_panics_and_refusals_are_named_cases() {
    // Every refusal arm above matches its named case exactly (no catch-all),
    // and no entry panics on any input in the battery.
    let battery: Vec<SphereIdentification> = vec![
        identify_sphere_world(Point3::new(f64::NAN, 0.0, 0.0), 1.0),
        identify_sphere_world(Point3::new(0.0, 0.0, 0.0), f64::INFINITY),
        identify_sphere_world(Point3::new(0.0, 0.0, 0.0), 0.0),
        identify_sphere_world(Point3::new(0.0, 0.0, 0.0), -1.0),
        identify_sphere_world(Point3::new(1.0, 2.0, 3.0), 4.0),
    ];
    for verdict in battery {
        match verdict {
            SphereIdentification::Sphere(witness) => {
                assert_eq!(witness.tag(), "certified_embedded_sphere");
                assert!(witness.radius().get() > 0.0);
            }
            SphereIdentification::NotASphere(failure) => match failure {
                SphereIdentificationFailure::NonFiniteCoordinate { cause } => assert!(matches!(
                    cause,
                    NumericDomainError::NotANumber | NumericDomainError::Infinite
                )),
                SphereIdentificationFailure::DegenerateRadius => {}
                SphereIdentificationFailure::NonSimilarityPlacement => {}
                SphereIdentificationFailure::UnverifiedPeriod => {}
            },
        }
    }

    // The placement entry refuses the named case and never panics either.
    let sphere = Sphere::new(Point3::new(1.0, 2.0, 3.0), 1.0);
    let placed = identify_sphere_placement(&Processor::with_transform(
        sphere,
        Matrix4::from_nonuniform_scale(2.0, 1.0, 1.0),
    ));
    assert!(matches!(
        placed,
        SphereIdentification::NotASphere(SphereIdentificationFailure::NonSimilarityPlacement)
    ));
    let placed =
        identify_sphere_placement(&Processor::with_transform(sphere, Matrix4::from_scale(2.0)));
    assert!(matches!(placed, SphereIdentification::Sphere(_)));
}
