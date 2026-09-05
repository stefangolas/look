//! CC-011-LOFT-WEIGHTS integration tests (theory §2.2 L1r): the certified
//! positive weight field of a delivered loft. The fast path admits an
//! all-positive control net without subdivision; a genuinely straddling weight
//! field (the kernel `weight_straddles_zero` ground-truth pattern) refuses
//! `NonPositiveWeightField`; a positive field whose certification needs more
//! subdivisions than the budget holds refuses the same way; and a certificate
//! produced under refinement is valid ONLY on the refined net — the test
//! applies the returned refinements to the shipped surface and re-checks
//! positivity there. The test names are the contract.

#![deny(clippy::unwrap_used)]

use truck_base::evidence::Budget;
use truck_certified::construct::loft_weights::certify_weight_field;
use truck_certified::construct::refusal::ConstructRefusal;
use truck_geometry::prelude::{BSplineSurface, KnotVec, Vector4};

/// Extract the `Ok` of a fallible certification; the fixture is valid by
/// construction, so the refusal arm is a test-bug panic (never an unwrap).
fn construct<T>(result: Result<T, ConstructRefusal>) -> T {
    match result {
        Ok(value) => value,
        Err(refusal) => panic!("a certification that must succeed refused: {refusal:?}"),
    }
}

/// A homogeneous control point with the given weight (the geometry channels
/// are irrelevant to the weight-field certification).
fn p4(w: f64) -> Vector4 {
    Vector4::new(0.0, 0.0, 0.0, w)
}

/// A degree-(2, 0) clamped surface whose `u` knot vector is `uknot` and whose
/// homogeneous `w`-channel control net is `weights` (one row per `u` control,
/// a single degree-0 `v` control). The weight field is the tensor product of
/// the quadratic `u` field with a constant `v` field.
fn weight_surface(uknot: KnotVec, weights: &[f64]) -> BSplineSurface<Vector4> {
    let vknot = KnotVec::bezier_knot(0);
    let rows: Vec<Vec<Vector4>> = weights.iter().map(|&w| vec![p4(w)]).collect();
    BSplineSurface::try_new((uknot, vknot), rows).expect("the weight-field fixture surface is valid")
}

/// The minimum `w`-control value of a surface net, row-major.
fn weight_net_min(surface: &BSplineSurface<Vector4>) -> f64 {
    let mut min = f64::INFINITY;
    for row in surface.control_points() {
        for point in row {
            if point.w < min {
                min = point.w;
            }
        }
    }
    min
}

/// The single-span quadratic `u` knot vector over `[0, 1]`.
fn quadratic_u_knot() -> KnotVec {
    KnotVec::bezier_knot(2)
}

/// The two-span quadratic `u` knot vector over `[0, 1]` with the interior knot
/// `1/2` at its full (C0) multiplicity.
fn two_span_u_knot() -> KnotVec {
    KnotVec::from(vec![0.0, 0.0, 0.0, 0.5, 0.5, 1.0, 1.0, 1.0])
}

#[test]
fn all_positive_control_weights_admit_without_subdivision() {
    // Fast path: a strictly positive control net certifies the weight field by
    // the convex-hull property, with no subdivision and no refinements.
    let surface = weight_surface(two_span_u_knot(), &[1.0, 0.5, 1.0, 0.5, 1.0]);
    let mut budget = Budget::new(1024, 1024, 64);
    let cert = construct(certify_weight_field(&surface, &mut budget));

    assert!(!cert.refined, "an all-positive net admits on the fast path");
    assert!(cert.refinements.is_empty(), "the fast path inserts no knots");
    assert_eq!(cert.min_control_weight, 0.5); // H-3: the net minimum, exact
    assert_eq!(budget.subdiv, 1024, "the free fast path spends no budget");
}

#[test]
fn straddling_weight_field_refuses_non_positive_weight_field() {
    // The kernel `weight_straddles_zero` ground-truth pattern (fixtures.rs:
    // weights `(1, -1, 1)` give `w(u) = 1 - 4u(1 - u)`, which reaches exactly
    // zero at `u = 1/2`): a pole inside the domain. No refinement can certify
    // a field that touches zero, so certification refuses — even with a
    // generous budget, the refusal fires at the depth cap.
    let surface = weight_surface(quadratic_u_knot(), &[1.0, -1.0, 1.0]);
    let mut budget = Budget::new(1 << 20, 1 << 20, 64);
    match certify_weight_field(&surface, &mut budget) {
        Err(ConstructRefusal::NonPositiveWeightField) => {}
        Ok(_) => panic!("a weight field that reaches zero must refuse"),
        Err(other) => panic!("a straddling weight field must refuse as NonPositiveWeightField, not {other:?}"),
    }
}

#[test]
fn refinement_budget_exhaustion_refuses_non_positive_weight_field() {
    // Two quadratic spans with control weights `(1, -1/2, 1)` each: the field
    // is strictly positive (minimum `1/4` over every span), but the coarse net
    // is not, so certification needs one dyadic split per straddling span.
    let surface = weight_surface(two_span_u_knot(), &[1.0, -0.5, 1.0, -0.5, 1.0]);

    // One subdivision is not enough for two straddling spans: the refinement
    // budget runs out and the admissible field is refused.
    let mut exhausted = Budget::new(1, 0, 0);
    match certify_weight_field(&surface, &mut exhausted) {
        Err(ConstructRefusal::NonPositiveWeightField) => {}
        Ok(_) => panic!("an under-budgeted refinement must refuse as NonPositiveWeightField"),
        Err(other) => panic!("budget exhaustion must refuse as NonPositiveWeightField, not {other:?}"),
    }

    // With a sufficient budget the same surface certifies: the refusal above
    // is a budget refusal of an admissible field, not a field defect.
    let mut sufficient = Budget::new(4, 0, 0);
    let cert = construct(certify_weight_field(&surface, &mut sufficient));
    assert!(cert.refined, "the admissible field needs refinement");
    assert!(cert.min_control_weight > 0.0); // H-3
}

#[test]
fn certified_net_is_the_refined_net_never_the_coarse_one() {
    // The storage rule (theory D4-clause-(a) lineage): a certificate produced
    // under refinement is valid ONLY if the identical knot insertions are
    // applied to the shipped surface. Apply `refinements` and re-check
    // positivity on the refined net; the coarse net was not positive.
    let surface = weight_surface(two_span_u_knot(), &[1.0, -0.5, 1.0, -0.5, 1.0]);
    assert!(weight_net_min(&surface) < 0.0); // H-3: the coarse net is not positive

    let mut budget = Budget::new(8, 0, 0);
    let cert = construct(certify_weight_field(&surface, &mut budget));
    assert!(cert.refined, "the certificate is produced under refinement");
    assert!(!cert.refinements.is_empty(), "refinement must insert knots");

    // Replay the recorded insertions onto a fresh copy of the shipped surface.
    let mut refined = surface.clone();
    for &(axis, _, knot) in &cert.refinements {
        if axis {
            refined.add_vknot(knot);
        } else {
            refined.add_uknot(knot);
        }
    }

    // The refined net is strictly positive and its minimum is the certificate's
    // minimum; the coarse net never was.
    let refined_min = weight_net_min(&refined);
    assert!(refined_min > 0.0); // H-3
    assert_eq!(cert.min_control_weight, refined_min); // H-3
    assert!(
        refined.uknot_vec().len() > surface.uknot_vec().len(),
        "the refined net carries the inserted knots"
    );

    // The refined net now admits on the free fast path: the certificate is
    // exactly the refined net's positivity.
    let mut fresh = Budget::new(0, 0, 0);
    let again = construct(certify_weight_field(&refined, &mut fresh));
    assert!(!again.refined, "the refined net admits without further subdivision");
    assert_eq!(again.min_control_weight, refined_min); // H-3
}
