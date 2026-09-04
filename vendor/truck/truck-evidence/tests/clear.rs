//! CC-004-CLEAR Sections 2-3 — the `clear` module's P5 ball-clearance tests
//! against the landed `ImplicitField` carriers (plane/sphere/cylinder/cone/
//! torus are all implemented; the sphere carrier suffices for the ground
//! truths). No CC-000 fixtures are reachable from `truck-evidence` and none are
//! needed.
//!
//! Geometry (all dyadic): a unit sphere field at the origin with the exclusion
//! box `z >= 2` (the region to stay away from), a contact ball of radius 0.5,
//! and margin `mu = 0.1`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use truck_base::cgmath64::{EuclideanSpace, Point3};
use truck_base::evidence::{Refusal, UnresolvedWitness};
use truck_evidence::clear::{ball_clearance, BallAdmissibility};
use truck_evidence::enclosure::{Box3, Interval};
use truck_geometry::specifieds::Sphere;

/// A validated interval with `lo <= hi`.
fn iv(lo: f64, hi: f64) -> Interval {
    Interval::try_from((lo, hi)).expect("valid interval bounds")
}

/// The excluded half-space `z >= 2`: wide and finite in `x` and `y` (every
/// test ball stays well inside that lateral extent) and `z in [2, 100]`, so
/// the only relevant face is the lower `z = 2` boundary.
fn exclusion_above_z2() -> Box3 {
    Box3 {
        x: iv(-10.0, 10.0),
        y: iv(-10.0, 10.0),
        z: iv(2.0, 100.0),
    }
}

/// The contact-ball radius `0.5` as a degenerate interval.
fn radius_half() -> Interval {
    iv(0.5, 0.5)
}

/// The clearance margin for the ground truths.
const MU: f64 = 0.1; // H-3: the clearance margin, a model-space length on the unit-scale fixtures

/// The unit sphere carrier at the origin (negative inside).
fn unit_sphere() -> Sphere {
    Sphere::new(Point3::origin(), 1.0)
}

#[test]
fn ball_clearance_true_with_margin_at_known_separation() {
    // The unit sphere with the exclusion box z >= 2 and the ball (r = 0.5) at
    // the origin: the ball is 1.5 clear of the exclusion (its own box tops out
    // at z = 0.5, gap 1.5 > mu) and Round containment holds inside the sphere.
    let sphere = unit_sphere();
    let centre = Box3::point(Point3::origin());
    let out = ball_clearance(
        &sphere,
        &centre,
        &exclusion_above_z2(),
        radius_half(),
        MU,
        BallAdmissibility::Round,
    );
    assert!(matches!(out, Ok(true)), "expected Ok(true), got {out:?}");
}

#[test]
fn ball_clearance_false_when_ball_overlaps_exclusion() {
    // The packet's second ground truth: the same ball displaced to z = 1.6
    // reaches z = 2.1, so its own box overlaps the exclusion (gap 0 <= mu): the
    // separation side rejects.
    let sphere = unit_sphere();
    let centre = Box3::point(Point3::new(0.0, 0.0, 1.6));
    let out = ball_clearance(
        &sphere,
        &centre,
        &exclusion_above_z2(),
        radius_half(),
        MU,
        BallAdmissibility::Round,
    );
    assert!(matches!(out, Ok(false)), "expected Ok(false), got {out:?}");

    // The separation side ALONE drives the rejection: a sphere wide enough for
    // Round containment to hold everywhere on the ball (a radius-2 sphere
    // centred at the ball's own centre) is still inadmissible because the ball
    // overlaps the excluded region.
    let roomy = Sphere::new(Point3::new(0.0, 0.0, 1.6), 2.0);
    let out = ball_clearance(
        &roomy,
        &centre,
        &exclusion_above_z2(),
        radius_half(),
        MU,
        BallAdmissibility::Round,
    );
    assert!(
        matches!(out, Ok(false)),
        "a ball overlapping the exclusion is inadmissible even when containment holds, got {out:?}"
    );
}

#[test]
fn ball_clearance_refuses_when_interval_straddles_the_margin() {
    // A centre BOX spanning z in [1.3, 1.45] with r = 0.5: the ball's own box
    // tops out at z = 1.95, a gap of 0.05 from the exclusion — in (0, mu], so
    // the separation interval straddles the margin (the family is neither all
    // clear nor rejected). The unit-sphere containment over that box straddles
    // zero as well. The refusal is NumericallyUnresolved with the
    // UncertifiedContainment witness and a zero spent ledger.
    let sphere = unit_sphere();
    let centre = Box3 {
        x: iv(0.0, 0.0),
        y: iv(0.0, 0.0),
        z: iv(1.3, 1.45),
    };
    let out = ball_clearance(
        &sphere,
        &centre,
        &exclusion_above_z2(),
        radius_half(),
        MU,
        BallAdmissibility::Round,
    );
    match out {
        Err(Refusal::NumericallyUnresolved {
            spent,
            witness: UnresolvedWitness::UncertifiedContainment,
        }) => {
            assert_eq!(spent.subdiv, 0);
            assert_eq!(spent.newton, 0);
            assert_eq!(spent.depth, 0);
        }
        other => assert!(
            false,
            "expected a NumericallyUnresolved refusal, got {other:?}"
        ),
    }
}

#[test]
fn round_mode_requires_field_negative_on_ball() {
    // A ball fully inside the unit sphere (field <= 0 over its box) clears in
    // Round mode; the same configuration in Fillet mode (which needs field >=
    // 0) is rejected by the containment side.
    let sphere = unit_sphere();
    let centre = Box3::point(Point3::origin());
    let round = ball_clearance(
        &sphere,
        &centre,
        &exclusion_above_z2(),
        radius_half(),
        MU,
        BallAdmissibility::Round,
    );
    assert!(
        matches!(round, Ok(true)),
        "Round on an interior ball, got {round:?}"
    );
    let fillet = ball_clearance(
        &sphere,
        &centre,
        &exclusion_above_z2(),
        radius_half(),
        MU,
        BallAdmissibility::Fillet,
    );
    assert!(
        matches!(fillet, Ok(false)),
        "Fillet needs a non-negative field on the ball, got {fillet:?}"
    );
}

#[test]
fn fillet_mode_requires_field_positive_on_ball() {
    // A ball fully outside the unit sphere (field >= 0 over its box, centred at
    // (2.5, 0, 0) so it stays clear of the z >= 2 exclusion) clears in Fillet
    // mode; Round mode rejects it on the containment side.
    let sphere = unit_sphere();
    let centre = Box3::point(Point3::new(2.5, 0.0, 0.0));
    let fillet = ball_clearance(
        &sphere,
        &centre,
        &exclusion_above_z2(),
        radius_half(),
        MU,
        BallAdmissibility::Fillet,
    );
    assert!(
        matches!(fillet, Ok(true)),
        "Fillet on an exterior ball, got {fillet:?}"
    );
    let round = ball_clearance(
        &sphere,
        &centre,
        &exclusion_above_z2(),
        radius_half(),
        MU,
        BallAdmissibility::Round,
    );
    assert!(
        matches!(round, Ok(false)),
        "Round needs a non-positive field on the ball, got {round:?}"
    );
}
