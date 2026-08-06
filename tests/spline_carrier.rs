//! Integration tests for the public spline-on-carrier certification entry
//! point, demonstrating the exact call signature the central-dispatch
//! integration owner will add to the curve-witness routing.
//!
//! These mirror `src/step/spline_carrier.rs`'s unit tests but go through the
//! crate's public API (`look::step::spline_carrier::certify_spline_carrier`)
//! from outside the crate, the way the integration owner will call it.

use look::step::spline_carrier::{
    CarrierQuery, CarrierWitness, CirclePlacement, InconsistencyWitness, LinearCarrierCoordinate,
    SplineCarrierCertification, UnresolvedSplineReason, certify_spline_carrier,
};
use truck_meshalgo::prelude::{Point3, Vector3};
use truck_stepio::r#in::step_geometry::{BSplineCurve, Curve3D, KnotVec, NurbsCurve};

/// The integration call signature: a `&Curve3D` (the source edge's 3D
/// representation), a `CarrierQuery` (the carrier relation to test, built
/// from the certified surface's axis/origin or a candidate circle), and the
/// edge's own parameter `trim`. Returns the stage-specific certification.
fn certify(curve: &Curve3D, query: CarrierQuery, trim: (f64, f64)) -> SplineCarrierCertification {
    certify_spline_carrier(curve, query, trim)
}

fn unit_circle() -> CirclePlacement {
    CirclePlacement {
        center: Point3::new(0.0, 0.0, 0.0),
        radius: 1.0,
        normal: Vector3::new(0.0, 0.0, 1.0),
    }
}

fn axial() -> LinearCarrierCoordinate {
    LinearCarrierCoordinate {
        axis: Vector3::new(0.0, 0.0, 1.0),
        origin: Point3::new(0.0, 0.0, 0.0),
        name: "axial",
    }
}

/// The end-to-end shape the band routing cares about: a non-rational B-spline
/// at a constant axial coordinate certifies as a circumferential-parallel
/// carrier over the whole trim, carrying the constant value and endpoints.
#[test]
fn b_spline_constant_axial_coordinate_certifies_end_to_end() {
    let knots = KnotVec::bezier_knot(1);
    let cps = vec![Point3::new(0.0, 0.0, 7.0), Point3::new(2.0, 0.0, 7.0)];
    let curve = Curve3D::BSplineCurve(BSplineCurve::new(knots, cps));
    match certify(
        &curve,
        CarrierQuery::ConstantLinearCoordinate(axial()),
        (0.0, 1.0),
    ) {
        SplineCarrierCertification::Certified(CarrierWitness::ConstantCoordinate {
            value,
            spans_examined,
            denominator_sign: None,
            ..
        }) => {
            assert!((value - 7.0).abs() < 1e-12);
            assert_eq!(spans_examined, 1);
        }
        other => panic!("expected Certified ConstantCoordinate, got {other:?}"),
    }
}

/// A NURBS with mixed-sign weights is a proved denominator inconsistency,
/// reported distinctly from a non-carrier — the integration owner routes it
/// to the typed-exit ledger, never to recovery.
#[test]
fn nurbs_mixed_sign_weights_reported_as_inconsistent_denominator() {
    let knots = KnotVec::bezier_knot(2);
    let cps = vec![
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 1.0, 0.0),
        Point3::new(2.0, 0.0, 0.0),
    ];
    let nurbs = NurbsCurve::try_from_bspline_and_weights(
        BSplineCurve::new(knots, cps),
        vec![1.0, -1.0, 1.0],
    )
    .expect("weights match");
    let curve = Curve3D::NurbsCurve(nurbs);
    assert_eq!(
        certify(
            &curve,
            CarrierQuery::CircularArcOn(unit_circle()),
            (0.0, 1.0)
        ),
        SplineCarrierCertification::Inconsistent(InconsistencyWitness::DenominatorSignIndefinite)
    );
}

/// A non-rational B-spline asked about circularity is a proved `Unsupported`:
/// the integration owner can record this as a definitive negative rather than
/// an open failure.
#[test]
fn non_rational_spline_circularity_is_proved_unsupported() {
    let knots = KnotVec::bezier_knot(1);
    let cps = vec![Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)];
    let curve = Curve3D::BSplineCurve(BSplineCurve::new(knots, cps));
    assert!(matches!(
        certify(
            &curve,
            CarrierQuery::CircularArcOn(unit_circle()),
            (0.0, 1.0)
        ),
        SplineCarrierCertification::Unsupported(_)
    ));
}

/// A NURBS circle with rounded irrational weights (the standard 1/√2 case)
/// lands in `Unresolved(CircleMembershipWithinRounding)` — honest that f64
/// cannot prove exact membership, rather than overclaiming `Certified`.
#[test]
fn nurbs_quarter_circle_is_unresolved_within_rounding() {
    let knots = KnotVec::bezier_knot(2);
    let s2 = std::f64::consts::FRAC_1_SQRT_2;
    let cps = vec![
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(1.0, 1.0, 0.0),
        Point3::new(0.0, 1.0, 0.0),
    ];
    let nurbs =
        NurbsCurve::try_from_bspline_and_weights(BSplineCurve::new(knots, cps), vec![1.0, s2, 1.0])
            .expect("weights match");
    let curve = Curve3D::NurbsCurve(nurbs);
    assert!(matches!(
        certify(
            &curve,
            CarrierQuery::CircularArcOn(unit_circle()),
            (0.0, 1.0)
        ),
        SplineCarrierCertification::Unresolved(
            UnresolvedSplineReason::CircleMembershipWithinRounding
        )
    ));
}
