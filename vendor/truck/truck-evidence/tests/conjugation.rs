//! BG-CAD-P9 — relative-frame canonicalization of `Placed` cylinder face
//! pairs through the dispatcher.
//!
//! The dyadic witnesses exercise the two routes of the `(Placed, _)` arm of
//! `analytic_ff`: the W2 intersecting-axes witness (two world-frame ellipses
//! via `equal_radius_cylinders`), the W3 fold (a translation + z-rotation +
//! uniform-scale placed pair answered exactly like its bare counterpart), the
//! D6 metamorphic gate under a rigid motion, and the D2/D3 refusal
//! boundaries (skew axes, unequal radii, non-cylinder families, non-uniform
//! scale).
//!
//! The fixture placements are `rotY(90°)` and `rotX(−90°)` of a canonical
//! z-axis cylinder; in this tree's `cgmath` convention they map `ẑ` onto
//! world `x̂` and `ŷ` respectively (each carrying a `cos(π/2) ≈ 6e-17`
//! residue, recorded in RESULT.json). The world axis of a placed carrier is
//! extracted exactly as the production code does, `normalize(m·ẑ)`, and the
//! on-carrier machine checks use those extracted axes.

// Test-only allow: H-1 bans unwrap/expect on paths reachable from untrusted
// geometry. Unit-test assertions on hand-built dyadic witnesses are not such
// a path; the expects below cannot fire for the values constructed (the
// landed test modules use the same pattern).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::f64::consts::{FRAC_PI_2, TAU};

use truck_base::cgmath64::{
    EuclideanSpace, InnerSpace, Matrix4, Point3, Rad, SquareMatrix, Transform, Vector3,
};
use truck_base::contact::{ContactDimension, ContactEventKind};
use truck_base::evidence::{Budget, Certified, EnvelopeCase, Method, Prop, Refusal, Truth};
use truck_evidence::analytic::{AnalyticIntersection, ExactCurve, PlacedCircle};
use truck_evidence::contact::{contact, BoundedStratum, ContactComplex, ContactLocus};
use truck_geometry::decorators::Processor;
use truck_geometry::recognize::CanonicalSurface;
use truck_geometry::specifieds::{Cylinder, Plane, Sphere};
use truck_geotrait::ParametricCurve;

/// The on-carrier residual: the certification precision the analytic cells
/// achieve on the f64-emitted ellipses (unit-scale residuals, never a
/// length).
const RESIDUAL: f64 = 1.0e-9; // H-3: unit-scale certified-point residual, not a length

/// The per-ellipse sample count of the on-carrier machine check (the probe's
/// 26-point witness).
const SAMPLES: usize = 26;

/// `rotY(90°)`: the placement of the W1/W2 witness that puts the canonical
/// z-axis cylinder on world axis `−x̂` in this tree's `cgmath` convention.
fn rot_y_quarter() -> Matrix4 {
    Matrix4::from_angle_y(Rad(FRAC_PI_2))
}

/// `rotX(−90°)`: the placement that puts the canonical z-axis cylinder on
/// world axis `−ŷ`.
fn rot_x_minus_quarter() -> Matrix4 {
    Matrix4::from_angle_x(Rad(-FRAC_PI_2))
}

/// The world axis direction of a placed carrier: `normalize(m·ẑ)`, exactly
/// the production extraction.
fn world_axis(placement: Matrix4) -> Vector3 {
    Vector3::new(placement.z.x, placement.z.y, placement.z.z).normalize()
}

/// A placed cylinder face stratum: a canonical z-axis `Cylinder` at `center`
/// of the given radius, under the affine placement `placement`, on the
/// `(u, v)` box.
fn placed_cylinder_at(
    placement: Matrix4,
    center: Point3,
    radius: f64,
    u_range: (f64, f64),
    v_range: (f64, f64),
) -> BoundedStratum {
    let inner = CanonicalSurface::Cylinder(
        Cylinder::new(center, radius)
            .expect("a positive radius constructs a valid cylinder")
            .value,
    );
    BoundedStratum::Face {
        surface: CanonicalSurface::Placed(Processor::with_transform(Box::new(inner), placement)),
        u_range,
        v_range,
    }
}

/// A placed cylinder face stratum whose inner cylinder sits at the origin.
fn placed_cylinder(
    placement: Matrix4,
    radius: f64,
    u_range: (f64, f64),
    v_range: (f64, f64),
) -> BoundedStratum {
    placed_cylinder_at(placement, Point3::origin(), radius, u_range, v_range)
}

/// A bare cylinder face stratum at `center`, radius `radius`, on the
/// `(u, v)` box.
fn bare_cylinder(
    center: Point3,
    radius: f64,
    u_range: (f64, f64),
    v_range: (f64, f64),
) -> BoundedStratum {
    BoundedStratum::Face {
        surface: CanonicalSurface::Cylinder(
            Cylinder::new(center, radius)
                .expect("a positive radius constructs a valid cylinder")
                .value,
        ),
        u_range,
        v_range,
    }
}

/// A placed unit-sphere face stratum.
fn placed_sphere(placement: Matrix4, radius: f64) -> BoundedStratum {
    let inner = CanonicalSurface::Sphere(Sphere::new(Point3::origin(), radius));
    BoundedStratum::Face {
        surface: CanonicalSurface::Placed(Processor::with_transform(Box::new(inner), placement)),
        u_range: (0.0, TAU),
        v_range: (0.0, TAU),
    }
}

/// The two `Ellipse` loci of a certified intersecting-axes answer.
fn two_ellipses(out: &Certified<ContactComplex>) -> (PlacedCircle, PlacedCircle) {
    assert_eq!(out.value.contacts.len(), 1);
    let record = out.value.contacts.first().expect("one contact record");
    assert_eq!(record.dimension, ContactDimension::Arc1);
    assert_eq!(record.kind, ContactEventKind::Transverse);
    let ContactLocus::Analytic(AnalyticIntersection::TwoCurves(
        [ExactCurve::Ellipse(e0), ExactCurve::Ellipse(e1)],
    )) = &record.locus
    else {
        panic!("expected two world-frame ellipses, got {:?}", record.locus);
    };
    (*e0, *e1)
}

/// The maximum over the sampled ellipse points of the radial residuals
/// against both world-placed cylinder carriers (each `|(p − foot) × dir|`
/// against the shared radius 1).
fn on_carrier_max_residual(
    e0: &PlacedCircle,
    e1: &PlacedCircle,
    foot0: Point3,
    dir0: Vector3,
    foot1: Point3,
    dir1: Vector3,
) -> f64 {
    let mut max_res: f64 = 0.0;
    for k in 0..SAMPLES {
        let t = TAU * k as f64 / SAMPLES as f64;
        for e in [e0, e1] {
            let p = e.subs(t);
            let d0 = (p - foot0).cross(dir0).magnitude();
            let d1 = (p - foot1).cross(dir1).magnitude();
            max_res = max_res.max((d0 - 1.0).abs()).max((d1 - 1.0).abs());
        }
    }
    max_res
}

#[test]
fn conjugation_placed_intersecting_axes_two_ellipses() {
    // The W2 witness through the dispatcher: two placed cylinders (radius 1,
    // perpendicular world axes through the origin, u ∈ [0, TAU], v ∈ [0, 2])
    // answer records whose loci are the two Steinmetz ellipses; every sampled
    // ellipse point lies on BOTH world-placed carriers at axis distance 1.
    let u = (0.0, TAU);
    let v = (0.0, 2.0);
    let lhs = placed_cylinder(rot_y_quarter(), 1.0, u, v);
    let rhs = placed_cylinder(rot_x_minus_quarter(), 1.0, u, v);
    let mut budget = Budget::new(100, 100, 100);
    let out = contact(&lhs, &rhs, &mut budget)
        .expect("an intersecting-axes equal-radius placed pair is decidable");
    assert_eq!(out.cert.method, Method::Exact);
    assert_eq!(out.cert.props.get(Prop::AnalyticCarrier), Truth::True);
    let (e0, e1) = two_ellipses(&out);
    let d0 = world_axis(rot_y_quarter());
    let d1 = world_axis(rot_x_minus_quarter());
    // The recorded mapping of the fixture placements: `rotY(90°)` puts ẑ on
    // world `x̂` and `rotX(−90°)` on `ŷ` (each with a `cos(π/2) ≈ 6e-17`
    // residue), so the two world axes are perpendicular through the origin.
    assert!(
        (d0.x - 1.0).abs() < RESIDUAL && d0.y.abs() < RESIDUAL && d0.z.abs() < RESIDUAL,
        "rotY(90°) must map ẑ onto world x̂, got {d0:?}"
    );
    assert!(
        (d1.y - 1.0).abs() < RESIDUAL && d1.x.abs() < RESIDUAL && d1.z.abs() < RESIDUAL,
        "rotX(−90°) must map ẑ onto world ŷ, got {d1:?}"
    );
    assert!(
        d0.dot(d1).abs() < RESIDUAL,
        "the witness world axes must be perpendicular"
    );
    let max_res = on_carrier_max_residual(&e0, &e1, Point3::origin(), d0, Point3::origin(), d1);
    assert!(
        max_res <= RESIDUAL,
        "sampled ellipse point off a world-placed carrier by {max_res}"
    );
}

#[test]
fn conjugation_identity_placement_equals_bare() {
    // D6's g = id special case: `Placed(identity) × bare` answers EXACTLY
    // what `bare × bare` answers for the same pair. The fold with θ = 0,
    // s = 1 reconstructs the identical bare carriers, so the whole certified
    // complexes are equal.
    let u = (0.0, TAU);
    // Certified-empty baseline: the parallel-offset pair (axis distance 3,
    // r = 1, v ∈ [0, 2]).
    let placed = placed_cylinder(Matrix4::identity(), 1.0, u, (0.0, 2.0));
    let bare_b = bare_cylinder(Point3::new(3.0, 0.0, 0.0), 1.0, u, (0.0, 2.0));
    let bare_a = bare_cylinder(Point3::origin(), 1.0, u, (0.0, 2.0));
    let mut budget = Budget::new(100, 100, 100);
    let placed_out = contact(&placed, &bare_b, &mut budget)
        .expect("an identity-placed offset pair is decidable");
    let mut budget = Budget::new(100, 100, 100);
    let bare_out = contact(&bare_a, &bare_b, &mut budget).expect("a bare offset pair is decidable");
    assert_eq!(format!("{placed_out:?}"), format!("{bare_out:?}"));
    assert!(
        placed_out.value.contacts.is_empty(),
        "the parallel-offset baseline is certified empty"
    );
    assert_eq!(placed_out.cert.method, Method::Exact);

    // The coaxial same-wall pair: the Region2/CoincidentInterval record, where
    // the mapped v boxes bound the same world patches.
    let placed_c = placed_cylinder(Matrix4::identity(), 1.0, u, (4.0, 6.0));
    let bare_c = bare_cylinder(Point3::origin(), 1.0, u, (4.0, 6.0));
    let bare_d = bare_cylinder(Point3::new(0.0, 0.0, 5.0), 1.0, u, (0.0, 1.0));
    let mut budget = Budget::new(100, 100, 100);
    let placed_out = contact(&placed_c, &bare_d, &mut budget)
        .expect("an identity-placed coaxial pair is decidable");
    let mut budget = Budget::new(100, 100, 100);
    let bare_out =
        contact(&bare_c, &bare_d, &mut budget).expect("a bare coaxial pair is decidable");
    assert_eq!(format!("{placed_out:?}"), format!("{bare_out:?}"));
    let record = placed_out
        .value
        .contacts
        .first()
        .expect("the overlapping coaxial pair emits one record");
    assert_eq!(record.dimension, ContactDimension::Region2);
    assert_eq!(record.kind, ContactEventKind::CoincidentInterval);
}

#[test]
fn conjugation_parallel_placed_pair_folds() {
    // The W3 witness: a translation + rotation-about-z + uniform-scale placed
    // pair answers exactly what the corresponding bare pair answers (the
    // fold path with mapped boxes). Both the certified-empty baseline and the
    // coaxial Region2 pair agree record-for-record.
    let fold = Matrix4::from_translation(Vector3::new(1.0, 2.0, 3.0))
        * Matrix4::from_angle_z(Rad(0.5))
        * Matrix4::from_nonuniform_scale(2.0, 2.0, 2.0);
    let u = (0.0, TAU);
    let placed_a = placed_cylinder_at(fold, Point3::origin(), 1.0, u, (0.0, 2.0));
    let placed_b = placed_cylinder_at(fold, Point3::new(3.0, 0.0, 0.0), 1.0, u, (0.0, 2.0));
    let bare_a = bare_cylinder(Point3::origin(), 1.0, u, (0.0, 2.0));
    let bare_b = bare_cylinder(Point3::new(3.0, 0.0, 0.0), 1.0, u, (0.0, 2.0));
    let mut budget = Budget::new(100, 100, 100);
    let placed_out = contact(&placed_a, &placed_b, &mut budget)
        .expect("a folded parallel placed pair is decidable");
    let mut budget = Budget::new(100, 100, 100);
    let bare_out =
        contact(&bare_a, &bare_b, &mut budget).expect("the bare parallel pair is decidable");
    assert_eq!(format!("{placed_out:?}"), format!("{bare_out:?}"));
    assert!(
        placed_out.value.contacts.is_empty(),
        "the certified-empty baseline folds to empty"
    );

    let placed_c = placed_cylinder_at(fold, Point3::origin(), 1.0, u, (4.0, 6.0));
    let placed_d = placed_cylinder_at(fold, Point3::new(0.0, 0.0, 5.0), 1.0, u, (0.0, 1.0));
    let bare_c = bare_cylinder(Point3::origin(), 1.0, u, (4.0, 6.0));
    let bare_d = bare_cylinder(Point3::new(0.0, 0.0, 5.0), 1.0, u, (0.0, 1.0));
    let mut budget = Budget::new(100, 100, 100);
    let placed_out = contact(&placed_c, &placed_d, &mut budget)
        .expect("a folded coaxial placed pair is decidable");
    let mut budget = Budget::new(100, 100, 100);
    let bare_out =
        contact(&bare_c, &bare_d, &mut budget).expect("the bare coaxial pair is decidable");
    assert_eq!(format!("{placed_out:?}"), format!("{bare_out:?}"));
    let record = placed_out
        .value
        .contacts
        .first()
        .expect("the folded coaxial pair emits one record");
    assert_eq!(record.dimension, ContactDimension::Region2);
    assert_eq!(record.kind, ContactEventKind::CoincidentInterval);
}

#[test]
fn conjugation_metamorphic_rigid() {
    // The D6 gate on the intersecting-axes pair under a rigid g (a rotation
    // about z + a translation): record count and kinds equal, and the
    // g-image of every first-answer ellipse sample lies on the second
    // answer's loci (on both transformed carriers).
    let u = (0.0, TAU);
    let v = (0.0, 2.0);
    let lhs = placed_cylinder(rot_y_quarter(), 1.0, u, v);
    let rhs = placed_cylinder(rot_x_minus_quarter(), 1.0, u, v);
    let mut budget = Budget::new(100, 100, 100);
    let out =
        contact(&lhs, &rhs, &mut budget).expect("the bare intersecting-axes pair is decidable");
    let (e0, e1) = two_ellipses(&out);

    let g =
        Matrix4::from_translation(Vector3::new(1.0, 2.0, 3.0)) * Matrix4::from_angle_z(Rad(0.7));
    let glhs = placed_cylinder(g * rot_y_quarter(), 1.0, u, v);
    let grhs = placed_cylinder(g * rot_x_minus_quarter(), 1.0, u, v);
    let mut budget = Budget::new(100, 100, 100);
    let gout = contact(&glhs, &grhs, &mut budget)
        .expect("the transformed intersecting-axes pair is decidable");
    assert_eq!(gout.value.contacts.len(), out.value.contacts.len());
    let grecord = gout.value.contacts.first().expect("one record");
    assert_eq!(grecord.dimension, ContactDimension::Arc1);
    assert_eq!(grecord.kind, ContactEventKind::Transverse);

    let gd0 = g.transform_vector(world_axis(rot_y_quarter())).normalize();
    let gd1 = g
        .transform_vector(world_axis(rot_x_minus_quarter()))
        .normalize();
    let gfoot = g.transform_point(Point3::origin());
    for k in 0..SAMPLES {
        let t = TAU * k as f64 / SAMPLES as f64;
        for e in [&e0, &e1] {
            let gp = g.transform_point(e.subs(t));
            let d0 = (gp - gfoot).cross(gd0).magnitude();
            let d1 = (gp - gfoot).cross(gd1).magnitude();
            assert!(
                (d0 - 1.0).abs() <= RESIDUAL,
                "g-image point off the transformed first carrier by {}",
                (d0 - 1.0).abs()
            );
            assert!(
                (d1 - 1.0).abs() <= RESIDUAL,
                "g-image point off the transformed second carrier by {}",
                (d1 - 1.0).abs()
            );
        }
    }
}

#[test]
fn conjugation_skew_pair_defers() {
    // The W4 mapping: skew (non-coplanar) equal-radius placed cylinders — the
    // eqrcyl cell's own `NonCanonicalCarrier` refusal, mapped by the
    // dispatcher to the deferred funnel.
    let u = (0.0, TAU);
    let v = (0.0, 2.0);
    let skew = Matrix4::from_translation(Vector3::new(0.0, 0.0, 1.0)) * rot_x_minus_quarter();
    let lhs = placed_cylinder(rot_y_quarter(), 1.0, u, v);
    let rhs = placed_cylinder(skew, 1.0, u, v);
    let mut budget = Budget::new(100, 100, 100);
    let out = contact(&lhs, &rhs, &mut budget);
    assert!(
        matches!(
            out,
            Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::ContactReductionDeferred
            ))
        ),
        "skew equal-radius placed cylinders must defer: {out:?}"
    );
}

#[test]
fn conjugation_unequal_radii_defers() {
    // D4.3: non-parallel placed cylinders with radii 1 and 2 belong to the
    // general solver's cell.
    let u = (0.0, TAU);
    let v = (0.0, 2.0);
    let lhs = placed_cylinder(rot_y_quarter(), 1.0, u, v);
    let rhs = placed_cylinder(rot_x_minus_quarter(), 2.0, u, v);
    let mut budget = Budget::new(100, 100, 100);
    let out = contact(&lhs, &rhs, &mut budget);
    assert!(
        matches!(
            out,
            Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::ContactReductionDeferred
            ))
        ),
        "non-parallel unequal-radius placed cylinders must defer: {out:?}"
    );
}

#[test]
fn conjugation_noncylinder_placed_defers() {
    // D2's family boundary: a `Placed` sphere × bare cylinder, and a placed
    // cylinder × bare plane, both refuse `ContactReductionDeferred`.
    let u = (0.0, TAU);
    let v = (0.0, 2.0);
    let sphere = placed_sphere(rot_y_quarter(), 1.0);
    let cyl = bare_cylinder(Point3::origin(), 1.0, u, v);
    let mut budget = Budget::new(100, 100, 100);
    let out = contact(&sphere, &cyl, &mut budget);
    assert!(
        matches!(
            out,
            Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::ContactReductionDeferred
            ))
        ),
        "a placed sphere is outside the cylinder family: {out:?}"
    );

    let placed_cyl = placed_cylinder(rot_y_quarter(), 1.0, u, v);
    let plane = BoundedStratum::Face {
        surface: CanonicalSurface::Plane(Plane::new(
            Point3::origin(),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        )),
        u_range: (0.0, 1.0),
        v_range: (0.0, 1.0),
    };
    let mut budget = Budget::new(100, 100, 100);
    let out = contact(&placed_cyl, &plane, &mut budget);
    assert!(
        matches!(
            out,
            Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::ContactReductionDeferred
            ))
        ),
        "a bare plane side is outside the cylinder family: {out:?}"
    );
}

#[test]
fn conjugation_nonuniform_scale_refuses() {
    // D3's similarity screen: a placed cylinder with |m·x̂| ≠ |m·ẑ| (scale
    // (2, 2, 3)) is an elliptical cross-section, a non-canonical carrier.
    let u = (0.0, TAU);
    let v = (0.0, 2.0);
    let nonuniform = Matrix4::from_nonuniform_scale(2.0, 2.0, 3.0);
    let lhs = placed_cylinder(nonuniform, 1.0, u, v);
    let rhs = bare_cylinder(Point3::origin(), 1.0, u, v);
    let mut budget = Budget::new(100, 100, 100);
    let out = contact(&lhs, &rhs, &mut budget);
    assert!(
        matches!(
            out,
            Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::NonCanonicalCarrier
            ))
        ),
        "a non-uniform-scaled placed cylinder must refuse NonCanonicalCarrier: {out:?}"
    );
}
