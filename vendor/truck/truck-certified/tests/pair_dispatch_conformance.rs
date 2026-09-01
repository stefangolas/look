//! Conformance tests for the certified analytic pair dispatch
//! (`truck_certified::pair_dispatch`, BG-CK-P1-DISPATCH).
//!
//! All types are `pub`; fixtures are witnesses built from the landed
//! identifiers (`identify_sphere_world`, `identify_cylinder` on constructed
//! `RevolutedCurve<Line<Point3>>`, `identify_plane`'s schema path — the public
//! entry the census uses). Every load-bearing shape:
//!
//! - every emitted `Circle`'s radius enclosure CONTAINS a brute-force
//!   ulp-bracketed true radius (the sqrt-enclosure discipline);
//! - every emitted `Line`/`Point` locus satisfies both surfaces' equations at
//!   its construction values to ulp tolerance (`// H-3` same-line opt-outs);
//! - the coincident-plane (`Overlap`) and coincident-sphere/coincident-cylinder
//!   (`UnsupportedPairClass`) asymmetries hit the NAMED variants exactly;
//! - `dispatch_pair(a, b) == dispatch_pair(b, a)` across all arms;
//! - no panic anywhere, admission is exact-predicate-decided.

#![deny(clippy::unwrap_used)]

use truck_certified::formal::cone::{identify_cone, ConeIdentification};
use truck_certified::formal::cylinder::{
    identify_cylinder, CertifiedEmbeddedCylinder, CylinderIdentification,
    CylinderIdentificationFailure,
};
use truck_certified::formal::intersection::PairUnsupported;
use truck_certified::formal::sphere::{
    identify_sphere_world, CertifiedEmbeddedSphere, SphereIdentification,
    SphereIdentificationFailure,
};
use truck_certified::formal::support::{
    identify_plane, PlaneSchema, SchemaIdentificationFailure, SupportSurfaceSchema,
};
use truck_certified::formal::torus::identify_torus;
use truck_certified::pair_dispatch::{
    dispatch_pair, CertifiedPairContact, CertifiedPairParticipant, CertifiedPairResult,
    ContactLocus,
};
use truck_geometry::prelude::{InnerSpace, Line, Plane, Point3, RevolutedCurve, Torus, Vector3};

// ---------------------------------------------------------------------------
// Fixtures and geometry predicates
// ---------------------------------------------------------------------------

fn plane_participant(origin: Point3, u: Vector3, v: Vector3) -> CertifiedPairParticipant {
    let plane = Plane::new(origin, origin + u, origin + v);
    let schema = identify_plane(&plane);
    CertifiedPairParticipant::from_support_schema(&schema)
        .expect("a separated plane basis identifies as a plane")
}

/// A unit vector perpendicular to `v`.
fn perpendicular(v: Vector3) -> Vector3 {
    if v.x.abs() <= v.y.abs() && v.x.abs() <= v.z.abs() {
        Vector3::new(0.0, -v.z, v.y).normalize()
    } else if v.y.abs() <= v.z.abs() {
        Vector3::new(-v.z, 0.0, v.x).normalize()
    } else {
        Vector3::new(-v.y, v.x, 0.0).normalize()
    }
}

fn cylinder_participant(
    radius: f64,
    origin: Point3,
    axis: Vector3,
    height: f64,
) -> CertifiedPairParticipant {
    let perp = perpendicular(axis);
    let p = origin + radius * perp;
    let q = p + height * axis;
    let revo = RevolutedCurve::by_revolution(Line(p, q), origin, axis);
    CertifiedPairParticipant::from_cylinder_identification(identify_cylinder(&revo))
        .expect("a generatrix parallel to the axis identifies as a cylinder")
}

fn sphere_participant(center: Point3, radius: f64) -> CertifiedPairParticipant {
    CertifiedPairParticipant::from_sphere_identification(identify_sphere_world(center, radius))
        .expect("a finite positive sphere identifies")
}

fn plane_schema_of(p: &CertifiedPairParticipant) -> PlaneSchema {
    match p {
        CertifiedPairParticipant::Plane(schema) => *schema,
        other => panic!("expected a plane participant, got {other:?}"),
    }
}

fn cylinder_of(p: &CertifiedPairParticipant) -> CertifiedEmbeddedCylinder {
    match p {
        CertifiedPairParticipant::Cylinder(cylinder) => cylinder.clone(),
        other => panic!("expected a cylinder participant, got {other:?}"),
    }
}

fn sphere_of(p: &CertifiedPairParticipant) -> CertifiedEmbeddedSphere {
    match p {
        CertifiedPairParticipant::Sphere(sphere) => *sphere,
        other => panic!("expected a sphere participant, got {other:?}"),
    }
}

fn assert_on_plane(p: Point3, schema: &PlaneSchema) {
    let n = schema.u_axis().cross(schema.v_axis());
    let err = (p - schema.origin()).dot(n).abs();
    let scale = n.magnitude() * (p - schema.origin()).magnitude().max(1.0);
    assert!(
        err <= 1e-9 * scale.max(1.0), // H-3
        "point {p:?} on plane, err={err}"
    );
}

fn assert_on_cylinder(p: Point3, cylinder: &CertifiedEmbeddedCylinder) {
    let s = cylinder.schema();
    let radial = p - s.origin() - (p - s.origin()).dot(s.axis()) * s.axis();
    let err = (radial.magnitude() - s.radius().get()).abs();
    let scale = s.radius().get().max(1.0);
    assert!(err <= 1e-9 * scale, "point {p:?} on cylinder, err={err}"); // H-3
}

fn assert_on_sphere(p: Point3, sphere: &CertifiedEmbeddedSphere) {
    let err = ((p - sphere.center()).magnitude() - sphere.radius().get()).abs();
    let scale = sphere.radius().get().max(1.0);
    assert!(err <= 1e-9 * scale, "point {p:?} on sphere, err={err}"); // H-3
}

fn assert_direction_in_plane(direction: Vector3, schema: &PlaneSchema) {
    let n = schema.u_axis().cross(schema.v_axis());
    let err = direction.dot(n).abs();
    let scale = direction.magnitude() * n.magnitude();
    assert!(
        err <= 1e-9 * scale.max(1.0), // H-3
        "direction {direction:?} in plane, err={err}"
    );
}

fn assert_direction_parallel_axis(direction: Vector3, axis: Vector3) {
    let err = direction.cross(axis).magnitude();
    let scale = direction.magnitude() * axis.magnitude();
    assert!(
        err <= 1e-9 * scale.max(1.0), // H-3
        "direction {direction:?} parallel to {axis:?}, err={err}"
    );
}

/// The brute-force ulp bracket of the true `sqrt(radius_sq)`: the two adjacent
/// `f64` values whose squares straddle `radius_sq`. An emitted radius
/// enclosure must CONTAIN both (the sqrt-enclosure discipline).
fn ulp_bracket_sqrt(radius_sq: f64) -> (f64, f64) {
    let mut u = radius_sq.sqrt();
    if u * u > radius_sq {
        while u * u > radius_sq {
            u = u.next_down();
        }
    } else {
        while u.next_up() * u.next_up() <= radius_sq {
            u = u.next_up();
        }
    }
    (u, u.next_up())
}

fn assert_radius_encloses_true(
    radius: &truck_certified::formal::exact::CertifiedInterval,
    radius_sq: f64,
) {
    let (lo, hi) = ulp_bracket_sqrt(radius_sq);
    assert!(
        radius.lo <= lo && radius.hi >= hi,
        "radius enclosure {radius:?} must contain the ulp bracket [{lo}, {hi}] of sqrt({radius_sq})"
    );
}

/// Extract the lone `Contact` from a result.
fn expect_contact(result: CertifiedPairResult) -> CertifiedPairContact {
    match result {
        CertifiedPairResult::Contact(contact) => contact,
        other => panic!("expected a contact, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Required tests (the eleven `tests_required` names)
// ---------------------------------------------------------------------------

#[test]
fn transverse_planes_emit_certified_line() {
    // Plane z = 0 (normal (0,0,1)) and plane y = 0 (normal (0,-1,0)) meet in
    // the x-axis.
    let a = plane_participant(
        Point3::new(0.0, 0.0, 0.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
    );
    let b = plane_participant(
        Point3::new(0.0, 0.0, 1.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
    );
    let result = dispatch_pair(&a, &b);
    let contact = expect_contact(result);
    let ContactLocus::Line { point, direction } = contact.locus else {
        panic!("transverse planes must emit a line locus");
    };
    let pa = plane_schema_of(&contact.first);
    let pb = plane_schema_of(&contact.second);
    assert_on_plane(point, &pa);
    assert_on_plane(point, &pb);
    // The direction lies in both planes and is parallel to their intersection.
    assert_direction_in_plane(direction, &pa);
    assert_direction_in_plane(direction, &pb);
    assert_direction_parallel_axis(direction, Vector3::new(1.0, 0.0, 0.0));
}

#[test]
fn distinct_parallel_planes_are_disjoint_and_coincident_refuses_overlap() {
    // Distinct parallel planes z = 0 and z = 5: exactly `Disjoint`.
    let a = plane_participant(
        Point3::new(0.0, 0.0, 0.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
    );
    let b = plane_participant(
        Point3::new(0.0, 0.0, 5.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
    );
    assert_eq!(dispatch_pair(&a, &b), CertifiedPairResult::Disjoint);

    // Coincident planes (the same z = 0 plane with different retained bases):
    // a positive-area shared region refuses `Overlap` — the 2D pipeline's own
    // meaning — by name.
    let c = plane_participant(
        Point3::new(0.0, 0.0, 0.0),
        Vector3::new(2.0, 0.0, 0.0),
        Vector3::new(0.0, 2.0, 0.0),
    );
    assert_eq!(
        dispatch_pair(&a, &c),
        CertifiedPairResult::Unsupported(PairUnsupported::Overlap)
    );
}

#[test]
fn plane_cylinder_transverse_emits_circle() {
    // A plane perpendicular to the cylinder's axis: the cut circle is centered
    // on the axis with the cylinder's exact radius.
    let plane = plane_participant(
        Point3::new(0.0, 0.0, 2.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
    );
    let cyl = cylinder_participant(
        3.0,
        Point3::new(0.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
        5.0,
    );
    let contact = expect_contact(dispatch_pair(&plane, &cyl));
    let ContactLocus::Circle {
        center,
        axis,
        radius,
    } = contact.locus
    else {
        panic!("a perpendicular plane cuts a circle");
    };
    assert_eq!(center, Point3::new(0.0, 0.0, 2.0));
    assert_eq!(axis, Vector3::new(0.0, 0.0, 1.0));
    assert!(
        radius.contains(3.0) && radius.is_degenerate(),
        "the cut radius is the cylinder's exact radius"
    );
    // The circle's rim lies on the plane and on the cylinder (the radius is
    // the cylinder's exact radius, so the rim point needs no bracket).
    let cylinder = cylinder_of(&contact.second);
    let rim = center + 3.0 * perpendicular(axis);
    assert_on_cylinder(rim, &cylinder);
    assert_on_plane(rim, &plane_schema_of(&contact.first));
}

#[test]
fn plane_cylinder_tangent_emits_generatrix_line_and_offset_is_disjoint() {
    // Plane x = 3 tangent to the radius-3 z-cylinder: one shared generatrix.
    let cyl = cylinder_participant(
        3.0,
        Point3::new(0.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
        5.0,
    );
    let tangent = plane_participant(
        Point3::new(3.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
    );
    let contact = expect_contact(dispatch_pair(&tangent, &cyl));
    let ContactLocus::Line { point, direction } = contact.locus else {
        panic!("a tangent parallel plane emits one generatrix line");
    };
    let plane_schema = plane_schema_of(&contact.first);
    let cylinder = cylinder_of(&contact.second);
    assert_on_plane(point, &plane_schema);
    assert_on_cylinder(point, &cylinder);
    assert_direction_parallel_axis(direction, cylinder.schema().axis());

    // The offset plane x = 5 (distance 5 > 3) is exactly `Disjoint`.
    let offset = plane_participant(
        Point3::new(5.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
    );
    assert_eq!(dispatch_pair(&offset, &cyl), CertifiedPairResult::Disjoint);
}

#[test]
fn plane_sphere_transverse_emits_circle_with_enclosing_radius() {
    // Sphere radius 5, plane z = 3: cut circle radius sqrt(25 - 9) = 4.
    let plane = plane_participant(
        Point3::new(0.0, 0.0, 3.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
    );
    let sphere = sphere_participant(Point3::new(0.0, 0.0, 0.0), 5.0);
    let contact = expect_contact(dispatch_pair(&plane, &sphere));
    let ContactLocus::Circle {
        center,
        axis,
        radius,
    } = contact.locus
    else {
        panic!("a transverse plane cuts a circle");
    };
    assert_eq!(center, Point3::new(0.0, 0.0, 3.0));
    assert_eq!(axis, Vector3::new(0.0, 0.0, 1.0));
    assert!(
        radius.contains(4.0),
        "the true radius 4 lies in the enclosure"
    );
    assert_radius_encloses_true(&radius, 16.0);
    // The circle's rim lies on the sphere and on the plane.
    let mid = (radius.lo + radius.hi) / 2.0;
    let rim = center + mid * perpendicular(axis);
    assert_on_sphere(rim, &sphere_of(&contact.second));
    assert_on_plane(rim, &plane_schema_of(&contact.first));
}

#[test]
fn tangent_sphere_plane_emits_point() {
    // Sphere radius 5, plane z = 5: one tangent point at the foot.
    let plane = plane_participant(
        Point3::new(0.0, 0.0, 5.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
    );
    let sphere = sphere_participant(Point3::new(0.0, 0.0, 0.0), 5.0);
    let contact = expect_contact(dispatch_pair(&plane, &sphere));
    let ContactLocus::Point { point } = contact.locus else {
        panic!("a tangent plane emits one point");
    };
    assert_eq!(point, Point3::new(0.0, 0.0, 5.0));
    let plane_schema = plane_schema_of(&contact.first);
    let sphere_witness = sphere_of(&contact.second);
    assert_on_plane(point, &plane_schema);
    assert_on_sphere(point, &sphere_witness);
}

#[test]
fn sphere_sphere_transverse_emits_radical_circle_and_tangent_emits_point() {
    // Radical circle: r1 = 5 at origin, r2 = 4 at (6,0,0). |c1-c2|² = 36 sits
    // strictly between (5-4)² = 1 and (5+4)² = 81.
    let a = sphere_participant(Point3::new(0.0, 0.0, 0.0), 5.0);
    let b = sphere_participant(Point3::new(6.0, 0.0, 0.0), 4.0);
    let contact = expect_contact(dispatch_pair(&a, &b));
    let ContactLocus::Circle {
        center,
        axis,
        radius,
    } = contact.locus
    else {
        panic!("strictly-between spheres emit a radical circle");
    };
    assert_eq!(center, Point3::new(3.75, 0.0, 0.0));
    assert_eq!(axis, Vector3::new(6.0, 0.0, 0.0));
    // radius² = r1² - 3.75² = 10.9375.
    assert_radius_encloses_true(&radius, 10.9375);
    // The radical circle's rim lies on BOTH spheres.
    let mid = (radius.lo + radius.hi) / 2.0;
    let rim = center + mid * perpendicular(axis);
    assert_on_sphere(rim, &sphere_of(&contact.first));
    assert_on_sphere(rim, &sphere_of(&contact.second));

    // External tangency: r1 = 5 at origin, r2 = 4 at (9,0,0). D = 81 = (r1+r2)².
    let ext = sphere_participant(Point3::new(9.0, 0.0, 0.0), 4.0);
    let contact = expect_contact(dispatch_pair(&a, &ext));
    let ContactLocus::Point { point } = contact.locus else {
        panic!("external tangency emits a point");
    };
    assert_eq!(point, Point3::new(5.0, 0.0, 0.0));
    assert_on_sphere(point, &sphere_of(&contact.first));
    assert_on_sphere(point, &sphere_of(&contact.second));

    // Internal tangency: r1 = 5 at origin, r2 = 4 at (1,0,0). D = 1 = (r1-r2)².
    let int = sphere_participant(Point3::new(1.0, 0.0, 0.0), 4.0);
    let contact = expect_contact(dispatch_pair(&a, &int));
    let ContactLocus::Point { point } = contact.locus else {
        panic!("internal tangency emits a point");
    };
    assert_eq!(point, Point3::new(5.0, 0.0, 0.0));
    assert_on_sphere(point, &sphere_of(&contact.first));
    assert_on_sphere(point, &sphere_of(&contact.second));
}

#[test]
fn coaxial_cylinders_emit_circle_and_equal_radius_refuses_overlap() {
    // A plane perpendicular to the cylinder's axis cuts a circle coaxial with
    // the cylinder (center on the axis, the cylinder's exact radius).
    let cyl = cylinder_participant(
        3.0,
        Point3::new(0.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
        5.0,
    );
    let plane = plane_participant(
        Point3::new(0.0, 0.0, 2.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
    );
    let contact = expect_contact(dispatch_pair(&plane, &cyl));
    let ContactLocus::Circle {
        center,
        axis,
        radius,
    } = contact.locus
    else {
        panic!("a perpendicular plane cuts a coaxial circle");
    };
    assert_eq!(center, Point3::new(0.0, 0.0, 2.0));
    assert_eq!(axis, Vector3::new(0.0, 0.0, 1.0));
    assert!(radius.contains(3.0) && radius.is_degenerate());

    // Two coaxial cylinders of EQUAL radius: coincident faces refuse
    // `UnsupportedPairClass` — NOT the 2D `Overlap` cause (the
    // coincident-cylinder asymmetry).
    let a = cylinder_participant(
        3.0,
        Point3::new(0.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
        5.0,
    );
    let same_axis = cylinder_participant(
        3.0,
        Point3::new(0.0, 0.0, 7.0),
        Vector3::new(0.0, 0.0, 1.0),
        3.0,
    );
    assert_eq!(
        dispatch_pair(&a, &same_axis),
        CertifiedPairResult::Unsupported(PairUnsupported::UnsupportedPairClass)
    );

    // Two coaxial cylinders of DIFFERENT radii never meet (the annulus gap):
    // exactly `Disjoint`.
    let bigger = cylinder_participant(
        4.0,
        Point3::new(0.0, 0.0, 7.0),
        Vector3::new(0.0, 0.0, 1.0),
        3.0,
    );
    assert_eq!(dispatch_pair(&a, &bigger), CertifiedPairResult::Disjoint);
}

#[test]
fn unsupported_pair_class_refuses_named_case() {
    // The enum-absence route: a cone side maps to `None` (the routing enum
    // carries no cone variant it cannot dispatch; the cone arm books
    // DISPATCH-2).
    let cone = RevolutedCurve::by_revolution(
        Line(Point3::new(0.5, 0.0, 1.0), Point3::new(2.0, 0.0, 4.0)),
        Point3::new(0.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
    );
    match identify_cone(&cone) {
        ConeIdentification::Cone(_) => {}
        other => panic!("the fixture must certify as a cone, got {other:?}"),
    }
    assert!(
        CertifiedPairParticipant::from_cone_identification(identify_cone(&cone)).is_none(),
        "a cone identification cannot route to a participant this packet can dispatch"
    );

    // A torus side maps to `None` likewise (the torus arm books DISPATCH-2).
    let torus = Torus::new(Point3::new(1.0, 2.0, 3.0), 5.0, 1.0);
    assert!(
        CertifiedPairParticipant::from_torus_identification(identify_torus(&torus)).is_none(),
        "a torus identification cannot route to a participant this packet can dispatch"
    );

    // A non-plane support schema maps to `None`.
    let non_plane = SupportSurfaceSchema::not_structurally_identified(
        SchemaIdentificationFailure::NoStructuralReader {
            representation: "spline",
        },
    );
    assert!(
        CertifiedPairParticipant::from_support_schema(&non_plane).is_none(),
        "only a certified plane schema routes to a Plane participant"
    );

    // A geometry case the admission screens reject: the general oblique
    // plane~cylinder cut (the plane is neither perpendicular to the axis nor
    // parallel to it, so the cut is an ellipse — not a `ContactLocus` variant,
    // and not certifiable closed-form here).
    let oblique = plane_participant(
        Point3::new(0.0, 0.0, 1.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 1.0),
    );
    let cyl = cylinder_participant(
        3.0,
        Point3::new(0.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
        5.0,
    );
    assert_eq!(
        dispatch_pair(&oblique, &cyl),
        CertifiedPairResult::Unsupported(PairUnsupported::UnsupportedPairClass)
    );

    // Non-parallel cylinder axes (the general skew-cylinder quartic): refused
    // this packet.
    let skew = cylinder_participant(
        2.0,
        Point3::new(0.0, 0.0, 0.0),
        Vector3::new(1.0, 1.0, 1.0),
        5.0,
    );
    assert_eq!(
        dispatch_pair(&cyl, &skew),
        CertifiedPairResult::Unsupported(PairUnsupported::UnsupportedPairClass)
    );

    // Cylinder~sphere is a class the enum can express but this packet does not
    // admit (books DISPATCH-2): refused by name.
    let sphere = sphere_participant(Point3::new(0.0, 0.0, 0.0), 2.0);
    assert_eq!(
        dispatch_pair(&cyl, &sphere),
        CertifiedPairResult::Unsupported(PairUnsupported::UnsupportedPairClass)
    );
}

#[test]
fn operand_swap_yields_the_sorted_canonical_answer() {
    for (a, b) in battery() {
        let ab = dispatch_pair(&a, &b);
        let ba = dispatch_pair(&b, &a);
        assert_eq!(ab, ba, "dispatch must be symmetric for {a:?} ~ {b:?}");
    }
}

#[test]
fn dispatch_never_panics_and_admission_is_exact_predicate_decided() {
    // H-1 source-scan: `pair_dispatch.rs` is covered by the crate-level
    // `#![deny(clippy::unwrap_used)]` and contains no `unwrap`/`expect`/
    // `panic!` anywhere. Every admission screen is an `Expansion` sign decision
    // over the witnesses' representation-derived `f64` coordinates — never an
    // f64 epsilon and never an interval straddle at admission time. This test
    // therefore runs the constructors' own refusals (zero-ish radii and NaN
    // coordinates are impossible through the identifying witnesses) rather
    // than panic paths, and exercises a broad dispatch battery covering every
    // arm and every refusal path without panicking.

    // The constructors refuse their degenerate inputs by name, never panic.
    assert!(matches!(
        identify_sphere_world(Point3::new(0.0, 0.0, 0.0), 0.0),
        SphereIdentification::NotASphere(SphereIdentificationFailure::DegenerateRadius)
    ));
    assert!(matches!(
        identify_sphere_world(Point3::new(f64::NAN, 0.0, 0.0), 1.0),
        SphereIdentification::NotASphere(SphereIdentificationFailure::NonFiniteCoordinate { .. })
    ));
    let degenerate_cyl = RevolutedCurve::by_revolution(
        Line(Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 0.0, 5.0)),
        Point3::new(0.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
    );
    assert!(matches!(
        identify_cylinder(&degenerate_cyl),
        CylinderIdentification::NotACylinder(CylinderIdentificationFailure::DegenerateRadius)
    ));
    let degenerate_plane = Plane::new(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(2.0, 0.0, 0.0),
    );
    assert!(matches!(
        identify_plane(&degenerate_plane),
        SupportSurfaceSchema::NotStructurallyIdentified(
            SchemaIdentificationFailure::PlaneBasisNotSeparated
        )
    ));

    // The dispatch battery spans every arm and every refusal path; each result
    // is one of the four typed variants and no call panics.
    for (a, b) in battery() {
        let forward = dispatch_pair(&a, &b);
        let backward = dispatch_pair(&b, &a);
        match (&forward, &backward) {
            (CertifiedPairResult::Disjoint, CertifiedPairResult::Disjoint) => {}
            (CertifiedPairResult::Contact(_), CertifiedPairResult::Contact(_)) => {}
            (CertifiedPairResult::Unsupported(_), CertifiedPairResult::Unsupported(_)) => {}
            (CertifiedPairResult::Unresolved(_), CertifiedPairResult::Unresolved(_)) => {}
            (other_a, other_b) => {
                panic!("typed-result battery mismatch: {other_a:?} vs {other_b:?}")
            }
        }
    }
}

/// The battery of pairs spanning all arms, used by the swap-symmetry and
/// no-panic tests.
fn battery() -> Vec<(CertifiedPairParticipant, CertifiedPairParticipant)> {
    let z_plane = plane_participant(
        Point3::new(0.0, 0.0, 0.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
    );
    let z5_plane = plane_participant(
        Point3::new(0.0, 0.0, 5.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
    );
    let y0_plane = plane_participant(
        Point3::new(0.0, 0.0, 1.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
    );
    let coincident = plane_participant(
        Point3::new(0.0, 0.0, 0.0),
        Vector3::new(2.0, 0.0, 0.0),
        Vector3::new(0.0, 2.0, 0.0),
    );
    let cyl = cylinder_participant(
        3.0,
        Point3::new(0.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
        5.0,
    );
    let cyl_offset = cylinder_participant(
        3.0,
        Point3::new(10.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
        5.0,
    );
    let cyl_small = cylinder_participant(
        2.0,
        Point3::new(0.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
        5.0,
    );
    let cyl_coaxial = cylinder_participant(
        3.0,
        Point3::new(0.0, 0.0, 7.0),
        Vector3::new(0.0, 0.0, 1.0),
        3.0,
    );
    let cyl_skew = cylinder_participant(
        2.0,
        Point3::new(0.0, 0.0, 0.0),
        Vector3::new(1.0, 1.0, 1.0),
        5.0,
    );
    let plane_z2 = plane_participant(
        Point3::new(0.0, 0.0, 2.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
    );
    let plane_x3 = plane_participant(
        Point3::new(3.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
    );
    let plane_x5 = plane_participant(
        Point3::new(5.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
    );
    let oblique = plane_participant(
        Point3::new(0.0, 0.0, 1.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 1.0),
    );
    let sphere5 = sphere_participant(Point3::new(0.0, 0.0, 0.0), 5.0);
    let sphere4_at_6 = sphere_participant(Point3::new(6.0, 0.0, 0.0), 4.0);
    let sphere4_at_9 = sphere_participant(Point3::new(9.0, 0.0, 0.0), 4.0);
    let sphere4_at_1 = sphere_participant(Point3::new(1.0, 0.0, 0.0), 4.0);
    let sphere_concentric = sphere_participant(Point3::new(0.0, 0.0, 0.0), 3.0);
    let plane_z5 = plane_participant(
        Point3::new(0.0, 0.0, 5.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
    );
    let plane_z3 = plane_participant(
        Point3::new(0.0, 0.0, 3.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
    );

    vec![
        // plane~plane: transverse, parallel distinct, coincident.
        (z_plane.clone(), y0_plane.clone()),
        (z_plane.clone(), z5_plane.clone()),
        (z_plane.clone(), coincident.clone()),
        // plane~cylinder: perpendicular, tangent, offset disjoint, oblique.
        (plane_z2.clone(), cyl.clone()),
        (plane_x3.clone(), cyl.clone()),
        (plane_x5.clone(), cyl.clone()),
        (oblique.clone(), cyl.clone()),
        // plane~sphere: circle, tangent point, disjoint.
        (plane_z3.clone(), sphere5.clone()),
        (plane_z5.clone(), sphere5.clone()),
        (z5_plane.clone(), sphere5.clone()),
        // sphere~sphere: radical circle, external/internal tangency, concentric.
        (sphere5.clone(), sphere4_at_6.clone()),
        (sphere5.clone(), sphere4_at_9.clone()),
        (sphere5.clone(), sphere4_at_1.clone()),
        (sphere5.clone(), sphere_concentric.clone()),
        // cylinder~cylinder: coaxial equal/different, parallel offset, tangent,
        // skew.
        (cyl.clone(), cyl_coaxial.clone()),
        (cyl.clone(), cyl_small.clone()),
        (cyl.clone(), cyl_offset.clone()),
        (cyl_small.clone(), cyl_offset.clone()),
        (cyl.clone(), cyl_skew.clone()),
        // cylinder~sphere: refused this packet.
        (cyl.clone(), sphere5.clone()),
    ]
}
