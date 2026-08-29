//! BG-CAD-P11 — torus FF pairs through the landed validated-FF stage.
//!
//! The fixture torus is `Torus::new(Point3::origin(), 2.0, 0.5)`; the planes
//! are constructed from three exact points. The certified points' machine
//! checks use a unit-scale residual (the certification precision the probe
//! achieved — its certified points landed at machine epsilon; recorded in the
//! packet's RESULT.json notes). Dyadic witnesses throughout.

// Test-only allow: H-1 bans unwrap/expect on paths reachable from untrusted
// geometry. Unit-test assertions on hand-built dyadic witnesses are not such a
// path; the expects below cannot fire for the values constructed (the landed
// test modules use the same pattern).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::f64::consts::TAU;

use truck_base::cgmath64::{EuclideanSpace, Point3};
use truck_base::contact::{ContactDimension, ContactEventKind};
use truck_base::evidence::{Budget, EnvelopeCase, Method, Refusal};
use truck_evidence::contact::{contact, BoundedStratum, ContactLocus};
use truck_geometry::recognize::CanonicalSurface;
use truck_geometry::specifieds::{Cone, Cylinder, Plane, Torus};

/// The certified-point residual: the certification precision the probe
/// achieved (unit-scale residuals on f values and radii, never a length).
const RESIDUAL: f64 = 1.0e-9; // H-3: unit-scale certified-point residual, not a length

/// The closed-form outer contact radius of the fixture: `2 + sqrt(3/16)`.
fn outer_radius() -> f64 {
    2.0 + 0.1875f64.sqrt()
}

/// The closed-form inner contact radius of the fixture: `2 - sqrt(3/16)`.
fn inner_radius() -> f64 {
    2.0 - 0.1875f64.sqrt()
}

/// The fixture torus, R = 2, r = 0.5, at the origin.
fn fixture_torus() -> Torus {
    Torus::new(Point3::origin(), 2.0, 0.5)
}

/// The horizontal plane z = 0.25, a patch covering both contact circles.
fn z025_plane() -> Plane {
    Plane::new(
        Point3::new(-3.0, -3.0, 0.25),
        Point3::new(3.0, -3.0, 0.25),
        Point3::new(-3.0, 3.0, 0.25),
    )
}

/// A face stratum on a canonical surface with a custom `(u, v)` box.
fn face_with_bounds(
    surface: CanonicalSurface,
    u_range: (f64, f64),
    v_range: (f64, f64),
) -> BoundedStratum {
    BoundedStratum::Face {
        surface,
        u_range,
        v_range,
    }
}

/// The torus patch spanning both contact circles at z = 0.25 (v = PI/6 and
/// 5PI/6, i.e. the full u sweep and a v band around them).
fn full_ring_torus_patch() -> BoundedStratum {
    face_with_bounds(
        CanonicalSurface::Torus(fixture_torus()),
        (0.0, TAU),
        (0.4, 2.8),
    )
}

/// The `ValidatedBranchCover` points of a certified complex, discovery order.
fn cover_points(
    out: &truck_evidence::outcome::Certified<truck_evidence::contact::ContactComplex>,
) -> Vec<Point3> {
    let mut points = Vec::new();
    for record in &out.value.contacts {
        if let ContactLocus::ValidatedBranchCover(cover) = &record.locus {
            points.extend(cover.points.iter().copied());
        }
    }
    points
}

#[test]
fn torus_plane_axial_two_circles() {
    // The plane z = 0.25 cuts the torus R = 2, r = 0.5 in two circles of
    // closed-form radii 2 +- sqrt(0.1875) at z = 0.25 exactly. Through the
    // dispatcher the answer carries certified points; every point sits at
    // z = 0.25 on the torus exactly, at one of the two closed-form radii, and
    // both families are present.
    let torus_patch = full_ring_torus_patch();
    let plane_patch = face_with_bounds(
        CanonicalSurface::Plane(z025_plane()),
        (0.0, 1.0),
        (0.0, 1.0),
    );
    let mut budget = Budget::new(20000, 0, 0);
    let out = contact(&torus_patch, &plane_patch, &mut budget)
        .expect("a regular axial torus/plane pair certifies under healthy budget");
    assert_eq!(out.cert.method, Method::Interval);
    assert_eq!(out.value.contacts.len(), 1);
    let record = out.value.contacts.first().expect("one contact record");
    assert_eq!(record.dimension, ContactDimension::Arc1);
    assert_eq!(record.kind, ContactEventKind::Transverse);
    let points = cover_points(&out);
    assert!(!points.is_empty(), "the axial plane certifies crossings");
    let (outer, inner) = (outer_radius(), inner_radius());
    let (mut saw_outer, mut saw_inner) = (false, false);
    for p in &points {
        let rhat = (p.x * p.x + p.y * p.y).sqrt();
        let torus_res = (rhat - 2.0) * (rhat - 2.0) + p.z * p.z - 0.25;
        assert!(
            (p.z - 0.25).abs() <= RESIDUAL,
            "point {p:?} must sit at z = 0.25 exactly"
        );
        assert!(
            torus_res.abs() <= RESIDUAL,
            "point {p:?} must lie on the torus exactly (residual {torus_res})"
        );
        assert!(
            (rhat - outer).abs() <= RESIDUAL || (rhat - inner).abs() <= RESIDUAL,
            "point {p:?} must sit at a closed-form radius, rhat = {rhat}"
        );
        saw_outer |= (rhat - outer).abs() <= RESIDUAL;
        saw_inner |= (rhat - inner).abs() <= RESIDUAL;
    }
    assert!(saw_outer, "the outer circle family is present");
    assert!(saw_inner, "the inner circle family is present");
}

#[test]
fn torus_plane_oblique_loop() {
    // The plane x + z = 1.35 cuts the torus in a closed loop whose maximum
    // radial coordinate clears the equator band (max rhat ~= 1.82 < R = 2).
    // Certified points satisfy BOTH surface equations at certification
    // precision; the answer is non-empty.
    let torus_patch = full_ring_torus_patch();
    let plane_patch = face_with_bounds(
        CanonicalSurface::Plane(Plane::new(
            Point3::new(0.0, -1.7, 1.35),
            Point3::new(2.0, -1.7, -0.65),
            Point3::new(0.0, 1.7, 1.35),
        )),
        (0.0, 1.0),
        (0.0, 1.0),
    );
    let mut budget = Budget::new(20000, 0, 0);
    let out = contact(&torus_patch, &plane_patch, &mut budget)
        .expect("a regular oblique torus/plane pair certifies under healthy budget");
    assert_eq!(out.cert.method, Method::Interval);
    let points = cover_points(&out);
    assert!(!points.is_empty(), "the oblique plane certifies crossings");
    for p in &points {
        let rhat = (p.x * p.x + p.y * p.y).sqrt();
        let torus_res = (rhat - 2.0) * (rhat - 2.0) + p.z * p.z - 0.25;
        assert!(
            torus_res.abs() <= RESIDUAL,
            "point {p:?} must lie on the torus exactly (residual {torus_res})"
        );
        assert!(
            (p.x + p.z - 1.35).abs() <= RESIDUAL,
            "point {p:?} must lie on the plane x+z=1.35 exactly"
        );
    }
}

#[test]
fn torus_plane_miss_proves_empty() {
    // The plane z = 0.65 is strictly above the torus patch's z extent, so the
    // certified AABBs separate on z and the pair proves empty contact with no
    // unresolved remainder.
    let torus_patch = full_ring_torus_patch();
    let plane_patch = face_with_bounds(
        CanonicalSurface::Plane(Plane::new(
            Point3::new(-3.0, -3.0, 0.65),
            Point3::new(3.0, -3.0, 0.65),
            Point3::new(-3.0, 3.0, 0.65),
        )),
        (0.0, 1.0),
        (0.0, 1.0),
    );
    let entry = Budget::new(128, 0, 0);
    let mut budget = entry;
    let out = contact(&torus_patch, &plane_patch, &mut budget)
        .expect("a separated AABB torus/plane pair proves empty contact");
    assert!(
        out.value.contacts.is_empty(),
        "a plane above the torus patch meets it nowhere"
    );
    assert_eq!(out.cert.method, Method::Interval);
    assert_eq!(
        out.cert.budget_left, entry,
        "the early separation spends nothing"
    );
}

#[test]
fn torus_degenerate_family_lift_refuses() {
    // The D4 lift (BG-CAD-P11): a horn torus (r = R) has a cusp on the
    // surface with grad f = 0, so no certified contact work is possible; the
    // typed refusal fires BEFORE any certified work and the budget is
    // untouched.
    let horn = Torus::new(Point3::origin(), 2.0, 2.0);
    let torus_patch = face_with_bounds(CanonicalSurface::Torus(horn), (0.0, TAU), (0.0, TAU));
    let plane_patch = face_with_bounds(
        CanonicalSurface::Plane(z025_plane()),
        (0.0, 1.0),
        (0.0, 1.0),
    );
    let entry = Budget::new(1024, 0, 0);
    let mut budget = entry;
    let out = contact(&torus_patch, &plane_patch, &mut budget);
    assert!(
        matches!(
            out,
            Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::NonCanonicalCarrier
            ))
        ),
        "a degenerate torus carrier refuses at the lift: {out:?}"
    );
    assert_eq!(budget, entry, "the lift refuses before any certified work");
}

#[test]
fn torus_equator_tangency_refuses() {
    // The plane x + z = 1.5 grazes the equator band rhat = R = 2 tangentially
    // at (2, 0, -0.5), and crosses the band transversally at (1, +-sqrt(3),
    // 0.5): those leaves straddle the band with a genuine contact point
    // inside, so they are never proven empty and never clean. The arm's
    // pre-split hits the resolution floor on them and returns the honest typed
    // outcome `ContactReductionDeferred` (the singular-class contact the v1
    // envelope excludes; the band-grazing family is a booked follow-up).
    let torus_patch = face_with_bounds(
        CanonicalSurface::Torus(fixture_torus()),
        (0.0, TAU),
        (0.0, TAU),
    );
    let plane_patch = face_with_bounds(
        CanonicalSurface::Plane(Plane::new(
            Point3::new(0.0, -1.8, 1.5),
            Point3::new(2.0, -1.8, -0.5),
            Point3::new(0.0, 1.8, 1.5),
        )),
        (0.0, 1.0),
        (0.0, 1.0),
    );
    let mut budget = Budget::new(20000, 0, 0);
    let out = contact(&torus_patch, &plane_patch, &mut budget);
    assert!(
        matches!(
            out,
            Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::ContactReductionDeferred
            ))
        ),
        "a band-grazing torus/plane pair refuses via the pre-split floor: {out:?}"
    );
}

#[test]
fn torus_torus_identical_carrier_screen() {
    // Two struct-equal torus faces with overlapping parameter boxes ride the
    // landed C0-C2 identity screen: the same-carrier torus row is periodic on
    // both u and v, so overlapping boxes emit the Region2/IdenticalCarrier
    // Coincident record on Method::Exact.
    let surface = CanonicalSurface::Torus(fixture_torus());
    let lhs = face_with_bounds(surface.clone(), (0.0, TAU), (0.4, 2.8));
    let rhs = face_with_bounds(surface, (0.0, TAU), (0.4, 2.8));
    let mut budget = Budget::new(100, 100, 100);
    let out = contact(&lhs, &rhs, &mut budget)
        .expect("equal dyadic torus carriers decide at the identity stage");
    assert_eq!(out.cert.method, Method::Exact);
    assert_eq!(out.value.contacts.len(), 1);
    let record = out.value.contacts.first().expect("one record");
    assert_eq!(record.dimension, ContactDimension::Region2);
    assert_eq!(record.kind, ContactEventKind::IdenticalCarrier);
    assert!(matches!(record.locus, ContactLocus::Coincident));
}

#[test]
fn torus_quadric_offset_pair_still_green() {
    // The landed offset mixed-quadric cell (cylinder x cone, the analytic_ff
    // shape) still answers its landed record through contact(): one
    // Arc1/Transverse validated branch cover with non-empty certified points
    // and empty singular/unresolved lists. Guards the D1 (torus field) and D2
    // (torus dispatch) changes against the quadric regression.
    let cyl = face_with_bounds(
        CanonicalSurface::Cylinder(
            Cylinder::new(Point3::origin(), 1.0)
                .expect("a unit cylinder is a valid carrier")
                .value,
        ),
        (0.8, 1.3),
        (0.8, 1.2),
    );
    let cone = face_with_bounds(
        CanonicalSurface::Cone(
            Cone::new(Point3::new(1.0, 0.0, 0.0), std::f64::consts::FRAC_PI_4)
                .expect("a dyadic cone is a valid carrier")
                .value,
        ),
        (0.0, std::f64::consts::PI),
        (0.8, 1.2),
    );
    let mut budget = Budget::new(4096, 0, 0);
    let out = contact(&cyl, &cone, &mut budget)
        .expect("a regular offset mixed-quadric pair certifies under healthy budget");
    assert_eq!(out.cert.method, Method::Interval);
    assert_eq!(out.value.contacts.len(), 1);
    let record = out.value.contacts.first().expect("one record");
    assert_eq!(record.dimension, ContactDimension::Arc1);
    assert_eq!(record.kind, ContactEventKind::Transverse);
    let ContactLocus::ValidatedBranchCover(cover) = &record.locus else {
        panic!("an offset mixed-quadric pair emits a validated branch cover");
    };
    assert!(!cover.points.is_empty(), "the cover certifies crossings");
    assert!(
        cover.singular_boxes.is_empty(),
        "a regular pair proves no singular cells"
    );
    assert!(
        cover.unresolved_boxes.is_empty(),
        "a regular pair proves no unresolved cells"
    );
}
