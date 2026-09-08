//! TOR-A conformance: the torus × plane exact circle loci of the Villarceau
//! factorization certificate (`truck-evidence::analytic::torus_section`),
//! exercised through the public API.
//!
//! The five required tests:
//!
//! - `axial_plane_emits_two_profile_circles` — the plane through the axis and
//!   the centre cuts two profile circles of radius `r` at `O ± R·(a × n̂)`,
//!   every sample on both carriers;
//! - `axis_perpendicular_plane_emits_coaxial_circles` — the horizontal plane
//!   cuts two coaxial circles `ρ± = R ± √(r² − d²)`, every sample on both
//!   carriers, the recovered radii matching the closed form;
//! - `villarceau_bitangent_plane_emits_two_circles_radius_R` — the classical
//!   radius-`R` corollary asserted FROM the emitted circle, not assumed;
//! - `general_plane_quartic_traces_not_approximated` — a residual (spiric)
//!   section routes to the tracer as a typed refusal, never an approximate
//!   circle;
//! - `empty_and_tangent_planes_typed` — the axis-perpendicular Empty arm and
//!   the double tangent circle are typed, exact classifications.
//!
//! Every witness is dyadic/rational so the exact predicates and the runtime
//! factorization certificate decide deterministically.

// Test-only allow: H-1 bans unwrap/expect on paths reachable from untrusted
// geometry. Unit-test assertions on hand-built dyadic witnesses are not such a
// path; the unwraps below cannot fire for the values constructed.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::f64::consts::PI;

use truck_base::cgmath64::{EuclideanSpace, InnerSpace, Point3};
use truck_base::evidence::{EnvelopeCase, Method, Prop, Refusal, Truth};
use truck_evidence::analytic::torus_section::plane_torus_section;
use truck_evidence::analytic::{AnalyticIntersection, ExactCurve, PlacedCircle};
use truck_geometry::specifieds::{Plane, Torus};
use truck_geotrait::ParametricCurve;

/// The on-carrier residual: the certification precision the analytic cell
/// achieves on the f64-emitted circles (unit-scale residuals, never a length).
const RESIDUAL: f64 = 1.0e-9; // H-3: unit-scale certified-point residual, not a length

/// The per-circle sample count of the on-carrier machine check.
const SAMPLES: usize = 64;

/// The fixture ring torus `R = 2, r = 1/2` at the origin (the torus_pairs
/// fixture).
fn torus_standard() -> Torus {
    Torus::new(Point3::origin(), 2.0, 0.5)
}

/// The Villarceau fixture ring torus `R = 5, r = 3` at the origin: the
/// bitangent plane of the Villarceau test has normal `(3/5, 0, 4/5)`, and the
/// Villarceau predicate `(R² − r²)·H = r²·s²` becomes `16·9/25 = 9·16/25`.
fn torus_villarceau() -> Torus {
    Torus::new(Point3::origin(), 5.0, 3.0)
}

/// A plane through three points.
fn plane(o: Point3, one: Point3, another: Point3) -> Plane {
    Plane::new(o, one, another)
}

/// The centre and radius recovered from a placed circle: `subs` takes the
/// angle, so `subs(0)` and `subs(π)` are antipodal and their midpoint is the
/// centre.
fn circle_geometry(c: &PlacedCircle) -> (Point3, f64) {
    let a = c.subs(0.0);
    let b = c.subs(PI);
    let center = Point3::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0, (a.z + b.z) / 2.0);
    let radius = (a - center).magnitude();
    (center, radius)
}

/// The torus implicit residual of `p` (axis-aligned torus through `center`).
fn torus_residual(p: Point3, center: Point3, major: f64, minor: f64) -> f64 {
    let q = p - center;
    let radial = (q.x * q.x + q.y * q.y).sqrt();
    (radial - major) * (radial - major) + q.z * q.z - minor * minor
}

/// Samples every circle of a two-circle arm and asserts each point sits on
/// both carriers to the certified residual.
fn assert_two_circles_on_torus_and_plane(
    circles: [PlacedCircle; 2],
    torus: &Torus,
    plane_normal: [f64; 3],
    plane_origin: Point3,
) {
    let major = torus.large_radius();
    let minor = torus.small_radius();
    let center = torus.center();
    let n = [plane_normal[0], plane_normal[1], plane_normal[2]];
    for circle in &circles {
        for i in 0..SAMPLES {
            let t = std::f64::consts::TAU * (i as f64) / (SAMPLES as f64 - 1.0);
            let p = circle.subs(t);
            let plane_res = (p.x - plane_origin.x) * n[0]
                + (p.y - plane_origin.y) * n[1]
                + (p.z - plane_origin.z) * n[2];
            let torus_res = torus_residual(p, center, major, minor);
            assert!(
                plane_res.abs() <= RESIDUAL,
                "point {p:?} leaves the plane (residual {plane_res})"
            );
            assert!(
                torus_res.abs() <= RESIDUAL,
                "point {p:?} leaves the torus (residual {torus_res})"
            );
        }
    }
}

/// Unwraps a two-circle arm.
fn two_circles(
    out: &truck_evidence::outcome::Certified<AnalyticIntersection>,
) -> [PlacedCircle; 2] {
    match &out.value {
        AnalyticIntersection::TwoCurves([ExactCurve::Circle(c0), ExactCurve::Circle(c1)]) => {
            [*c0, *c1]
        }
        other => panic!("expected two circle loci, got {other:?}"),
    }
}

#[test]
fn axial_plane_emits_two_profile_circles() {
    // The plane x = 0 contains the axis and the centre: two profile circles
    // of radius r = 1/2, centred at (0, ±R, 0) = (0, ±2, 0).
    let torus = torus_standard();
    let cutting = plane(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(0.0, 1.0, 0.0),
        Point3::new(0.0, 0.0, 1.0),
    );
    let out = plane_torus_section(&cutting, &torus).unwrap();
    assert_eq!(out.cert.method, Method::Exact);
    assert_eq!(out.cert.props.get(Prop::AnalyticCarrier), Truth::True);
    let circles = two_circles(&out);
    for circle in &circles {
        let (c, radius) = circle_geometry(circle);
        assert!(
            (radius - 0.5).abs() <= RESIDUAL,
            "a profile circle has radius r = 1/2, got {radius}"
        );
        // Centres at O ± R·ŷ.
        assert!(
            c.x.abs() <= RESIDUAL && (c.y.abs() - 2.0).abs() <= RESIDUAL && c.z.abs() <= RESIDUAL,
            "centre {c:?} must sit at (0, ±2, 0)"
        );
    }
    // The two centres are opposite across the torus centre.
    let (c0, _) = circle_geometry(&circles[0]);
    let (c1, _) = circle_geometry(&circles[1]);
    assert!(
        (c0.x + c1.x).abs() <= RESIDUAL
            && (c0.y + c1.y).abs() <= RESIDUAL
            && (c0.z + c1.z).abs() <= RESIDUAL,
        "the two profile centres are opposite across the torus centre"
    );
    assert_two_circles_on_torus_and_plane(circles, &torus, [1.0, 0.0, 0.0], Point3::origin());
}

#[test]
fn axis_perpendicular_plane_emits_coaxial_circles() {
    // The horizontal plane z = 1/4 cuts the torus R = 2, r = 1/2 in two
    // coaxial circles of closed-form radii R ± √(r² − d²) = 2 ± √(3/16),
    // both centred on the axis at the plane height. The dyadic offset keeps
    // the certificate's exact arithmetic inside range.
    let torus = torus_standard();
    let z: f64 = 0.25;
    let root = 0.1875f64.sqrt();
    let expected = [2.0 - root, 2.0 + root];
    let cutting = plane(
        Point3::new(-2.0, -2.0, z),
        Point3::new(2.0, -2.0, z),
        Point3::new(-2.0, 2.0, z),
    );
    let out = plane_torus_section(&cutting, &torus).unwrap();
    assert_eq!(out.cert.method, Method::Exact);
    let circles = two_circles(&out);
    for circle in &circles {
        let (c, radius) = circle_geometry(circle);
        assert!(
            c.y.abs() <= RESIDUAL && (c.z - z).abs() <= RESIDUAL,
            "a coaxial circle centre {c:?} must sit on the axis at z = {z}"
        );
        assert!(
            (radius - expected[0]).abs() <= RESIDUAL || (radius - expected[1]).abs() <= RESIDUAL,
            "recovered radius {radius} differs from the closed forms 2 ± sqrt(3/16)"
        );
    }
    let (_, rad0) = circle_geometry(&circles[0]);
    let (_, rad1) = circle_geometry(&circles[1]);
    assert!(
        (rad0 - rad1).abs() > 1.0e-9, // H-3: dimensionless separation of the two distinct coaxial radii, not a length
        "the two coaxial circles have distinct radii"
    );
    assert_two_circles_on_torus_and_plane(
        circles,
        &torus,
        [0.0, 0.0, 1.0],
        Point3::new(-2.0, -2.0, z),
    );
}

#[test]
// The packet's required test name ends in the symbol `R` (the classical
// corollary); it is a mandated identifier, not a naming regression.
#[allow(non_snake_case)]
fn villarceau_bitangent_plane_emits_two_circles_radius_R() {
    // The Villarceau bitangent plane through the centre with exact normal
    // (3, 0, 4) (raw; unit (3/5, 0, 4/5)) cuts the torus R = 5, r = 3 in two
    // circles. The classical corollary — each recovered circle has radius
    // exactly R = 5 — is asserted FROM the emitted locus, never assumed.
    let torus = torus_villarceau();
    let cutting = plane(
        Point3::origin(),
        Point3::new(0.0, 5.0, 0.0),
        Point3::new(-4.0, 0.0, 3.0),
    );
    let out = plane_torus_section(&cutting, &torus).unwrap();
    assert_eq!(out.cert.method, Method::Exact);
    assert_eq!(out.cert.props.get(Prop::AnalyticCarrier), Truth::True);
    let circles = two_circles(&out);
    for circle in &circles {
        let (c, radius) = circle_geometry(circle);
        assert!(
            (radius - 5.0).abs() <= RESIDUAL,
            "the Villarceau corollary fails: recovered radius {radius} != R = 5"
        );
        // Centres at O ± r·(a × n̂) = (0, ±3, 0).
        assert!(
            c.x.abs() <= RESIDUAL && (c.y.abs() - 3.0).abs() <= RESIDUAL && c.z.abs() <= RESIDUAL,
            "a Villarceau centre {c:?} must sit at (0, ±3, 0)"
        );
    }
    // Every sampled point of each circle sits on the torus and on the plane
    // (raw normal (15, 0, 20), through the origin).
    assert_two_circles_on_torus_and_plane(circles, &torus, [15.0, 0.0, 20.0], Point3::origin());
}

#[test]
fn general_plane_quartic_traces_not_approximated() {
    // The central 45° plane x + z = 0 (raw normal (1, 0, 1)) is not axial,
    // not axis-perpendicular, and not a Villarceau bitangent plane for this
    // torus: the section is a genuine spiric quartic. The cell must not
    // approximate it — it routes to the tracer as a typed refusal.
    let torus = torus_standard();
    let cutting = plane(
        Point3::origin(),
        Point3::new(1.0, 0.0, -1.0),
        Point3::new(0.0, 1.0, 0.0),
    );
    let out = plane_torus_section(&cutting, &torus);
    assert!(
        matches!(
            out,
            Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::ContactReductionDeferred
            ))
        ),
        "a residual quartic must trace, never approximate: {out:?}"
    );

    // A second, offset residual witness: the same section family through a
    // plane that provably meets the torus (it contains the outer-equator
    // point (0, R + r, 0)).
    let offset = plane(
        Point3::new(1.0, 0.0, -1.0),
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(0.0, 2.5, 0.0),
    );
    let out2 = plane_torus_section(&offset, &torus);
    assert!(
        matches!(
            out2,
            Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::ContactReductionDeferred
            ))
        ),
        "an intersecting residual quartic must trace: {out2:?}"
    );
}

#[test]
fn empty_and_tangent_planes_typed() {
    // Axis-perpendicular tangent: the plane z = r = 1/2 touches the torus
    // along the whole circle radius R = 2 — a single double circle, typed.
    let torus = torus_standard();
    let tangent = plane(
        Point3::new(-2.0, -2.0, 0.5),
        Point3::new(2.0, -2.0, 0.5),
        Point3::new(-2.0, 2.0, 0.5),
    );
    let out = plane_torus_section(&tangent, &torus).unwrap();
    assert_eq!(out.cert.method, Method::Exact);
    let AnalyticIntersection::TangentCircle(circle) = &out.value else {
        panic!("a z = r plane is a tangent circle, got {:?}", out.value);
    };
    let (c, radius) = circle_geometry(circle);
    assert!(
        c.x.abs() <= RESIDUAL && c.y.abs() <= RESIDUAL && (c.z - 0.5).abs() <= RESIDUAL,
        "the double circle centre {c:?} must sit on the axis at z = 1/2"
    );
    assert!(
        (radius - 2.0).abs() <= RESIDUAL,
        "the double circle has radius R = 2, got {radius}"
    );
    for i in 0..SAMPLES {
        let t = std::f64::consts::TAU * (i as f64) / (SAMPLES as f64 - 1.0);
        let p = circle.subs(t);
        let torus_res = torus_residual(p, torus.center(), 2.0, 0.5);
        assert!(
            torus_res.abs() <= RESIDUAL,
            "point {p:?} leaves the torus (residual {torus_res})"
        );
        assert!(
            (p.z - 0.5).abs() <= RESIDUAL,
            "point {p:?} leaves the tangent plane"
        );
    }

    // Axis-perpendicular empty: the plane z = 7/10 clears the tube.
    let miss = plane(
        Point3::new(-2.0, -2.0, 0.7),
        Point3::new(2.0, -2.0, 0.7),
        Point3::new(-2.0, 2.0, 0.7),
    );
    let out = plane_torus_section(&miss, &torus).unwrap();
    assert_eq!(out.cert.method, Method::Exact);
    assert!(
        matches!(out.value, AnalyticIntersection::Empty),
        "a plane above the tube is Empty, got {:?}",
        out.value
    );

    // A separated residual plane (oblique, beyond R + r of the centre) is
    // also an exact Empty, not a trace.
    let far = plane(
        Point3::new(3.0, 0.0, 0.0),
        Point3::new(3.0, 1.0, 0.0),
        Point3::new(3.0, 0.0, 1.0),
    );
    let out = plane_torus_section(&far, &torus).unwrap();
    assert!(
        matches!(out.value, AnalyticIntersection::Empty),
        "a separated plane is Empty, got {:?}",
        out.value
    );
}
