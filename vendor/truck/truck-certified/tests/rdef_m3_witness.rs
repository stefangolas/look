//! RDEF-M3-WITNESS-TIER: exact degeneracy classification for 2x2 carriers.
//!
//! The witness tier W1–W3 of `docs/RANK_DEFICIENT_CONTACT_SPEC.md` §5.1–5.2,
//! graded against the T1–T5 topology fixtures (§10). Every assertion here is
//! exact (rational arithmetic); no tolerance and no numeric classification is
//! exercised.

use truck_certified::formal::curve2d::SourceEntityId;
use truck_certified::tangency::fixtures::{
    T1PlaneSphereWitness, T2CylinderCylinderWitness, T3CoaxialCylinderWitness,
    T4FilletContactWitness, T5SphereConeWitness, WitnessFixtureKit,
};
use truck_certified::tangency::witness::{
    carrier_identity, quadric_pencil, quadric_pencil_with_provenance, same_carrier,
    CarrierIdentity, ConstructionWitness, ExactCarrier, ExactContactClass, ExactContactCurve,
    WitnessTierRefusal,
};

#[test]
fn w1_plane_plane_classifies_exactly() {
    let z0 = ExactCarrier::plane([0, 0, 1], 0).expect("valid plane");
    let z1 = ExactCarrier::plane([0, 0, 1], 1).expect("valid plane");
    assert_eq!(
        quadric_pencil(&z0, &z1).expect("classified"),
        ExactContactClass::Empty
    );

    let x0 = ExactCarrier::plane([1, 0, 0], 0).expect("valid plane");
    assert_eq!(
        quadric_pencil(&z0, &x0).expect("classified"),
        ExactContactClass::Regular
    );

    // A scaled normal with the matching scaled offset is the same plane.
    let z0_scaled = ExactCarrier::plane([0, 0, 2], 0).expect("valid plane");
    assert_eq!(
        quadric_pencil(&z0, &z0_scaled).expect("classified"),
        ExactContactClass::Coincident
    );
}

#[test]
fn w1_plane_sphere_classifies_exactly() {
    let sphere = ExactCarrier::sphere([0, 0, 0], 1).expect("valid sphere");

    let tangent = ExactCarrier::plane([0, 1, 0], 1).expect("valid plane");
    assert_eq!(
        quadric_pencil(&tangent, &sphere).expect("classified"),
        ExactContactClass::TangentPoint
    );

    let through = ExactCarrier::plane([0, 1, 0], 0).expect("valid plane");
    assert_eq!(
        quadric_pencil(&through, &sphere).expect("classified"),
        ExactContactClass::Regular
    );

    let away = ExactCarrier::plane([0, 1, 0], 2).expect("valid plane");
    assert_eq!(
        quadric_pencil(&away, &sphere).expect("classified"),
        ExactContactClass::Empty
    );
}

#[test]
fn w1_plane_cylinder_classifies_exactly() {
    let cylinder = ExactCarrier::cylinder([0, 0, 0], [0, 0, 1], 1).expect("valid cylinder");

    // Plane parallel to the axis at distance one: a double line (tangent).
    let tangent = ExactCarrier::plane([1, 0, 0], 1).expect("valid plane");
    assert_eq!(
        quadric_pencil(&tangent, &cylinder).expect("classified"),
        ExactContactClass::TangentCurve
    );

    // Plane parallel to the axis at distance zero: two parallel lines.
    let through = ExactCarrier::plane([1, 0, 0], 0).expect("valid plane");
    assert_eq!(
        quadric_pencil(&through, &cylinder).expect("classified"),
        ExactContactClass::Regular
    );

    // Plane parallel to the axis beyond the radius: empty.
    let away = ExactCarrier::plane([1, 0, 0], 2).expect("valid plane");
    assert_eq!(
        quadric_pencil(&away, &cylinder).expect("classified"),
        ExactContactClass::Empty
    );

    // Plane crossing the axis: an ellipse (regular).
    let oblique = ExactCarrier::plane([0, 0, 1], 0).expect("valid plane");
    assert_eq!(
        quadric_pencil(&oblique, &cylinder).expect("classified"),
        ExactContactClass::Regular
    );
}

#[test]
fn w1_plane_cone_classifies_exactly() {
    let cone = ExactCarrier::cone([0, 0, 0], [0, 0, 1], 1).expect("valid cone");

    // A plane through the apex, not tangent: two generators crossing there.
    let apex_plane = ExactCarrier::plane([0, 1, 0], 0).expect("valid plane");
    assert_eq!(
        quadric_pencil(&apex_plane, &cone).expect("classified"),
        ExactContactClass::TangentCrossing
    );

    // The tangent plane z = x touches along one generator (a double line).
    let tangent = ExactCarrier::plane([-1, 0, 1], 0).expect("valid plane");
    assert_eq!(
        quadric_pencil(&tangent, &cone).expect("classified"),
        ExactContactClass::TangentCurve
    );

    // A transverse plane z = 1 cuts a circle.
    let cut = ExactCarrier::plane([0, 0, 1], 1).expect("valid plane");
    assert_eq!(
        quadric_pencil(&cut, &cone).expect("classified"),
        ExactContactClass::Regular
    );
}

#[test]
fn w1_cylinder_cylinder_classifies_exactly() {
    let inner = ExactCarrier::cylinder([0, 0, 0], [0, 0, 1], 1).expect("valid cylinder");

    // Internal tangency: distance one, radii one and two.
    let internal = ExactCarrier::cylinder([1, 0, 0], [0, 0, 1], 4).expect("valid cylinder");
    assert_eq!(
        quadric_pencil(&inner, &internal).expect("classified"),
        ExactContactClass::TangentCurve
    );

    // External tangency: distance three, radii one and two.
    let external = ExactCarrier::cylinder([3, 0, 0], [0, 0, 1], 4).expect("valid cylinder");
    assert_eq!(
        quadric_pencil(&inner, &external).expect("classified"),
        ExactContactClass::TangentCurve
    );

    // Crossing: distance two -> two lines.
    let crossing = ExactCarrier::cylinder([2, 0, 0], [0, 0, 1], 4).expect("valid cylinder");
    assert_eq!(
        quadric_pencil(&inner, &crossing).expect("classified"),
        ExactContactClass::Regular
    );

    // Disjoint: distance five.
    let far = ExactCarrier::cylinder([5, 0, 0], [0, 0, 1], 4).expect("valid cylinder");
    assert_eq!(
        quadric_pencil(&inner, &far).expect("classified"),
        ExactContactClass::Empty
    );
}

#[test]
fn w1_sphere_sphere_and_sphere_cylinder_classify_exactly() {
    let unit = ExactCarrier::sphere([0, 0, 0], 1).expect("valid sphere");
    let tangent = ExactCarrier::sphere([2, 0, 0], 1).expect("valid sphere");
    assert_eq!(
        quadric_pencil(&unit, &tangent).expect("classified"),
        ExactContactClass::TangentPoint
    );
    let crossing = ExactCarrier::sphere([1, 0, 0], 1).expect("valid sphere");
    assert_eq!(
        quadric_pencil(&unit, &crossing).expect("classified"),
        ExactContactClass::Regular
    );
    let disjoint = ExactCarrier::sphere([5, 0, 0], 1).expect("valid sphere");
    assert_eq!(
        quadric_pencil(&unit, &disjoint).expect("classified"),
        ExactContactClass::Empty
    );

    let cylinder = ExactCarrier::cylinder([0, 0, 0], [0, 0, 1], 1).expect("valid cylinder");
    let tangent_cyl = ExactCarrier::sphere([2, 0, 0], 1).expect("valid sphere");
    assert_eq!(
        quadric_pencil(&tangent_cyl, &cylinder).expect("classified"),
        ExactContactClass::TangentPoint
    );
    let through_cyl = ExactCarrier::sphere([0, 0, 0], 1).expect("valid sphere");
    assert_eq!(
        quadric_pencil(&through_cyl, &cylinder).expect("classified"),
        ExactContactClass::Regular
    );
}

#[test]
fn w1_sphere_cone_classifies_on_axis_exactly() {
    let cone = ExactCarrier::cone([0, 0, 0], [0, 0, 1], 1).expect("valid cone");

    // Inscribed at height two: tangent along a circle.
    let tangent = ExactCarrier::sphere([0, 0, 2], 2).expect("valid sphere");
    assert_eq!(
        quadric_pencil(&tangent, &cone).expect("classified"),
        ExactContactClass::TangentCurve
    );

    // Strictly inside the cone: empty.
    let inside = ExactCarrier::sphere([0, 0, 2], 1).expect("valid sphere");
    assert_eq!(
        quadric_pencil(&inside, &cone).expect("classified"),
        ExactContactClass::Empty
    );

    // Cutting the cone: a circle.
    let cutting = ExactCarrier::sphere([0, 0, 2], 9).expect("valid sphere");
    assert_eq!(
        quadric_pencil(&cutting, &cone).expect("classified"),
        ExactContactClass::Regular
    );

    // Off-axis sphere x cone is booked open (a named refusal, never sampled).
    let off_axis = ExactCarrier::sphere([1, 0, 2], 2).expect("valid sphere");
    assert_eq!(
        quadric_pencil(&off_axis, &cone).expect_err("off-axis sphere x cone"),
        WitnessTierRefusal::UnsupportedCarrierPair
    );
}

#[test]
fn w1_unsupported_pair_refuses_named() {
    let first = ExactCarrier::cone([0, 0, 0], [0, 0, 1], 1).expect("valid cone");
    let second = ExactCarrier::cone([0, 0, 3], [0, 0, 1], 1).expect("valid cone");
    assert_eq!(
        quadric_pencil(&first, &second).expect_err("cone x cone is booked open"),
        WitnessTierRefusal::UnsupportedCarrierPair
    );
}

#[test]
fn w2_coincident_carrier_routes() {
    // Equal definitions up to an exact scale: the same axis line and radius.
    let a = ExactCarrier::cylinder([0, 0, 0], [0, 0, 1], 1).expect("valid cylinder");
    let b = ExactCarrier::cylinder([0, 0, 5], [0, 0, 2], 1).expect("valid cylinder");
    assert!(same_carrier(&a, &b).expect("exact equality"));
    assert_eq!(
        quadric_pencil(&a, &b).expect("classified"),
        ExactContactClass::Coincident
    );

    // Provenance alone: distinct definitions, one shared SourceEntityId.
    let distinct = ExactCarrier::cylinder([0, 0, 0], [0, 0, 1], 4).expect("valid cylinder");
    let id = SourceEntityId(11);
    assert_eq!(
        carrier_identity(&a, &distinct, Some((id, id))).expect("exact identity"),
        CarrierIdentity::ProvenanceShared(id)
    );
    assert_eq!(
        quadric_pencil_with_provenance(&a, &distinct, Some((id, id))).expect("classified"),
        ExactContactClass::Coincident
    );

    // Without provenance the distinct pair falls through to W1: concentric
    // unequal-radius cylinders are empty.
    assert_eq!(
        quadric_pencil_with_provenance(&a, &distinct, None).expect("classified"),
        ExactContactClass::Empty
    );
}

#[test]
fn w3_construction_witness_emits_tangent_curve() {
    let plane = ExactCarrier::plane([0, 0, 1], 0).expect("valid plane");
    let line = ExactContactCurve::line([0, 0, 0], [1, 0, 0]).expect("valid line");
    let witness = ConstructionWitness::new(plane.clone(), line).expect("line lies on plane");
    assert_eq!(witness.class(), ExactContactClass::TangentCurve);
    assert_eq!(witness.carrier(), &plane);

    // A curve that is not on the carrier is not a certificate.
    let off = ExactContactCurve::line([0, 0, 1], [1, 0, 0]).expect("valid line");
    assert_eq!(
        ConstructionWitness::new(plane, off).expect_err("off-carrier line"),
        WitnessTierRefusal::CurveOffCarrier
    );
}

#[test]
fn w3_circle_on_sphere_and_generator_on_cylinder() {
    let sphere = ExactCarrier::sphere([0, 0, 0], 9).expect("valid sphere");
    let great_circle = ExactContactCurve::circle([0, 0, 0], [0, 0, 1], 9).expect("valid circle");
    assert!(ConstructionWitness::new(sphere, great_circle).is_ok());

    let cylinder = ExactCarrier::cylinder([0, 0, 0], [0, 0, 1], 4).expect("valid cylinder");
    let generator = ExactContactCurve::line([2, 0, 0], [0, 0, 1]).expect("valid line");
    assert!(ConstructionWitness::new(cylinder.clone(), generator).is_ok());

    let off_cylinder = ExactContactCurve::line([3, 0, 0], [0, 0, 1]).expect("valid line");
    assert!(ConstructionWitness::new(cylinder, off_cylinder).is_err());
}

#[test]
fn t1_t5_witness_kit_admits_with_exact_classes() {
    let kit = WitnessFixtureKit::build().expect("the T1-T5 kit admits");

    assert_eq!(
        kit.t1.admit().expect("T1").class,
        ExactContactClass::TangentPoint
    );
    assert_eq!(
        kit.t2.admit().expect("T2").class,
        ExactContactClass::TangentCurve
    );
    assert_eq!(
        kit.t3.admit().expect("T3").class,
        ExactContactClass::Coincident
    );
    assert_eq!(
        kit.t4.admit().expect("T4").class,
        ExactContactClass::TangentCurve
    );
    assert_eq!(
        kit.t5.admit().expect("T5").class,
        ExactContactClass::TangentCurve
    );

    // The valid-B-rep local dimensions fixed by the classes.
    assert_eq!(kit.t1.admit().expect("T1").local_dim, 0);
    assert_eq!(kit.t2.admit().expect("T2").local_dim, 1);
    assert_eq!(kit.t3.admit().expect("T3").local_dim, 2);
    assert_eq!(kit.t4.admit().expect("T4").local_dim, 1);
    assert_eq!(kit.t5.admit().expect("T5").local_dim, 1);

    // The bare fixture constructors agree with the kit.
    assert!(T1PlaneSphereWitness::new().admit().is_ok());
    assert!(T2CylinderCylinderWitness::new().admit().is_ok());
    assert!(T3CoaxialCylinderWitness::new().admit().is_ok());
    assert!(T4FilletContactWitness::new().admit().is_ok());
    assert!(T5SphereConeWitness::new().admit().is_ok());
}

#[test]
fn classification_is_deterministic() {
    let plane = ExactCarrier::plane([0, 1, 0], 1).expect("valid plane");
    let sphere = ExactCarrier::sphere([0, 0, 0], 1).expect("valid sphere");
    let first = quadric_pencil(&plane, &sphere).expect("classified");
    for _ in 0..8 {
        assert_eq!(quadric_pencil(&plane, &sphere).expect("classified"), first);
    }
}

#[test]
fn degenerate_carrier_constructors_refuse() {
    assert_eq!(
        ExactCarrier::plane([0, 0, 0], 1).expect_err("zero normal"),
        WitnessTierRefusal::DegenerateCarrier
    );
    assert_eq!(
        ExactCarrier::sphere([0, 0, 0], 0).expect_err("zero radius"),
        WitnessTierRefusal::DegenerateCarrier
    );
    assert_eq!(
        ExactCarrier::cylinder([0, 0, 0], [0, 0, 0], 1).expect_err("zero axis"),
        WitnessTierRefusal::DegenerateCarrier
    );
    assert_eq!(
        ExactCarrier::cone([0, 0, 0], [0, 0, 0], 1).expect_err("zero axis"),
        WitnessTierRefusal::DegenerateCarrier
    );
}
