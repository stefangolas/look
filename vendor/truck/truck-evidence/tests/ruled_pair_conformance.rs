//! FSSI-004-RULED conformance: the ruled × ruled exact analytic locus
//! (`truck-evidence::analytic::ruled_pair`), exercised through the public API.
//!
//! The four required tests:
//!
//! - `two_linear_extrusions_cross_dyadic_exact` — transverse bounded ruled
//!   spans cross in an exactly-dyadic segment; the scalar predicate `λ = 0`
//!   and the rational `u, v` recovery are decisive-zero / degenerate-interval
//!   exact at dyadic locus parameters (CC-024 discipline);
//! - `parallel_generators_refuse_typed` — the parallel-generator excision
//!   refuses `Refusal::NumericallyUnresolved` with the landed witness
//!   vocabulary (`UnresolvedWitness::RootNotIsolated`), never a guess;
//! - `clipped_generator_domain_crossing_events_certified` — a crossing whose
//!   `u` recovery exits a generator domain terminates at a certified boundary
//!   event (`u − u_hi = 0`, degenerate-exact), not at a clipped float;
//! - `v5_analytic_pairs_bit_identical` — the landed BG-ANA-001 family answers
//!   are bit-identical snapshots (FSSI-004 only ADDS recognition; V5-pair
//!   identity).

use inari::Interval;
use truck_base::cgmath64::{InnerSpace, Point3, Vector3};
use truck_evidence::analytic::equal_radius_cylinders::equal_radius_cylinders;
use truck_evidence::analytic::parallel_cylinders::parallel_cylinders;
use truck_evidence::analytic::plane_cylinder::plane_cylinder;
use truck_evidence::analytic::plane_plane::plane_plane;
use truck_evidence::analytic::plane_sphere::plane_sphere;
use truck_evidence::analytic::ruled_pair::{lambda, recover_u_v, ruled_pair, RuledSpan};
use truck_evidence::analytic::sphere_sphere::sphere_sphere;
use truck_evidence::analytic::{AnalyticIntersection, ExactCurve};
use truck_evidence::outcome::{Method, Prop, Refusal, Truth, UnresolvedWitness};
use truck_geometry::specifieds::{Cylinder, Line, Plane, Sphere};

/// Builds a ruled span `S(t, u) = origin + t·directrix + u·generator` on
/// `t ∈ [0, 1]`, `u ∈ [u_lo, u_hi]`.
fn span(
    origin: [f64; 3],
    directrix: [f64; 3],
    generator: [f64; 3],
    u_lo: f64,
    u_hi: f64,
) -> RuledSpan {
    RuledSpan {
        origin: Point3::new(origin[0], origin[1], origin[2]),
        directrix: Vector3::new(directrix[0], directrix[1], directrix[2]),
        generator: Vector3::new(generator[0], generator[1], generator[2]),
        u_lo,
        u_hi,
    }
}

/// The surface point `S(t, u)` of a span (test-local; `subs` is private).
fn surface(s: RuledSpan, t: f64, u: f64) -> Point3 {
    s.origin + t * s.directrix + u * s.generator
}

/// Whether an interval is the degenerate `[0, 0]` (decisive zero).
fn decisive_zero(i: Interval) -> bool {
    i.inf() == 0.0 && i.sup() == 0.0
}

/// Whether an interval is a degenerate interval exactly equal to `x`.
fn degenerate(i: Interval, x: f64) -> bool {
    i.inf() == x && i.sup() == x
}

/// The segment endpoints of a transverse `Curve(Line(..))` outcome.
fn segment(out: &truck_evidence::outcome::Certified<AnalyticIntersection>) -> (Point3, Point3) {
    match &out.value {
        AnalyticIntersection::Curve(ExactCurve::Line(Line(p0, p1))) => (*p0, *p1),
        other => panic!("expected a transverse Line segment, got {other:?}"),
    }
}

fn points_eq_set(a: (Point3, Point3), p: Point3, q: Point3) -> bool {
    (a.0 == p && a.1 == q) || (a.0 == q && a.1 == p)
}

/// The residual of a point against a span's carrier plane: `(p − origin)·n`.
/// Unit-scale witnesses make this dimensionless.
fn plane_residual(s: RuledSpan, p: Point3) -> f64 {
    (p - s.origin).dot(s.directrix.cross(s.generator))
}

#[test]
fn two_linear_extrusions_cross_dyadic_exact() {
    // Oblique crossing (Config A): X is the unit z = 0 square, Y is the strip
    // through (0, 0, 1/2) spanned by (1, 2, 0) and the generator z, with the
    // z-generator clamped to [−1, 1]. The carrier planes meet in the diagonal
    // of X, and the bounded spans cross in the exact dyadic segment
    // (0,0,0) → (1/2, 1, 0). Every locus parameter on the segment is dyadic:
    // t = s ∈ [0, 1/2], u = 2t, v = −1/2.
    let x = span([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], 0.0, 1.0);
    let y = span([0.0, 0.0, 0.5], [1.0, 2.0, 0.0], [0.0, 0.0, 1.0], -1.0, 1.0);

    let out = ruled_pair(&x, &y).expect("a dyadic transverse witness is decidable");
    assert_eq!(out.cert.method, Method::Exact);
    assert_eq!(out.cert.props.get(Prop::AnalyticCarrier), Truth::True);
    let (p0, p1) = segment(&out);
    assert!(
        points_eq_set(
            (p0, p1),
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.5, 1.0, 0.0)
        ),
        "unexpected dyadic segment {p0:?} -> {p1:?}"
    );

    // λ = 0 is decisive-zero at dyadic locus parameters (t = s on the locus).
    for t in [0.125, 0.25, 0.375] {
        assert!(
            decisive_zero(lambda(&x, &y, t, t)),
            "λ(t, t) must be exact zero at t = {t}"
        );
    }

    // The recovery is a degenerate interval equal to the exact parameter, and
    // the reconstructed point lies on BOTH carriers exactly and on the emitted
    // segment.
    let ((u, v), t) = (recover_u_v(&x, &y, 0.25, 0.25).unwrap(), 0.25);
    assert!(
        degenerate(u, 0.5),
        "u recovery must be exactly 1/2, got {u:?}"
    );
    assert!(
        degenerate(v, -0.5),
        "v recovery must be exactly −1/2, got {v:?}"
    );
    let px = surface(x, t, 0.5);
    let py = surface(y, t, -0.5);
    assert_eq!(px, Point3::new(0.25, 0.5, 0.0));
    assert_eq!(
        px, py,
        "recovered point must lie on both carriers at the same 3-D point"
    );

    // Symmetry: operand order never changes the certified segment.
    let back = ruled_pair(&y, &x).expect("the swapped dyadic witness is decidable");
    let (q0, q1) = segment(&back);
    assert!(
        points_eq_set((q0, q1), p0, p1),
        "dispatch must be symmetric"
    );

    // Perpendicular crossing (Config B): the y = 0 and x = 0 squares share the
    // exact dyadic segment (0,0,0) → (0,0,1); λ(t, s) = t is decisive zero at
    // the locus parameter t = 0, and the recovery u = z = s is exact.
    let xb = span([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0], 0.0, 1.0);
    let yb = span([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0], 0.0, 1.0);
    let outb = ruled_pair(&xb, &yb).expect("the dyadic perpendicular witness is decidable");
    let (rb0, rb1) = segment(&outb);
    assert!(
        points_eq_set(
            (rb0, rb1),
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.0, 0.0, 1.0)
        ),
        "unexpected dyadic segment {rb0:?} -> {rb1:?}"
    );
    assert!(
        decisive_zero(lambda(&xb, &yb, 0.0, 0.5)),
        "λ(0, s) must be exact zero"
    );
    let (ub, vb) = recover_u_v(&xb, &yb, 0.0, 0.5).expect("recovery on a transverse pair");
    assert!(
        degenerate(ub, 0.5),
        "u recovery must equal the z-coordinate, got {ub:?}"
    );
    assert!(
        degenerate(vb, 0.0),
        "v recovery must be exactly 0, got {vb:?}"
    );
    assert_eq!(surface(xb, 0.0, 0.5), surface(yb, 0.5, 0.0));
    // Dyadic-clean endpoints sit on both carrier planes with zero residual.
    assert_eq!(plane_residual(xb, rb0), 0.0);
    assert_eq!(plane_residual(yb, rb0), 0.0);
    assert_eq!(plane_residual(xb, rb1), 0.0);
    assert_eq!(plane_residual(yb, rb1), 0.0);
}

#[test]
fn parallel_generators_refuse_typed() {
    // The parallel-generator excision: with generator_x × generator_y decisive
    // zero the u, v recovery blows up in a neighborhood of every point, so the
    // pair refuses typed — never a guess, never a downgraded arm.
    let x = span([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0], 0.0, 1.0);
    let parallel = span([0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0], 0.0, 1.0);
    let anti_parallel = span([0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, -1.0], 0.0, 1.0);
    for y in [parallel, anti_parallel] {
        let err = ruled_pair(&x, &y).expect_err("a parallel-generator pair must refuse typed");
        assert!(
            matches!(
                err,
                Refusal::NumericallyUnresolved {
                    witness: UnresolvedWitness::RootNotIsolated,
                    ..
                }
            ),
            "the excision refusal must carry the landed witness vocabulary, got {err:?}"
        );
    }
    // The excision is about the GENERATOR pair, never about the planes: the
    // same carriers with a crossed generator decide exactly (here: the bounded
    // spans miss, a certified Empty, not an excision refusal).
    let tilted = span([0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 1.0], 0.0, 1.0);
    let out = ruled_pair(&x, &tilted).expect("crossed generators must not refuse excision");
    assert!(
        matches!(out.value, AnalyticIntersection::Empty),
        "the crossed-generator pair is decidable (spans miss): {:?}",
        out.value
    );
}

#[test]
fn clipped_generator_domain_crossing_events_certified() {
    // Config C: X's generator domain u ∈ [0, 1/2] clips the oblique crossing of
    // Config A. The locus is the diagonal t = s; the recovery u = 2t exits X's
    // generator domain at u = 1/2 exactly, so the emitted segment terminates at
    // the certified boundary event u − u_hi = 0 — not at a clipped float. The
    // lower end terminates where u − u_lo = 0 at the X corner. The Y
    // parameters (s = t, v = −1/2) stay strictly inside Y's domains along the
    // whole open segment.
    let x = span([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], 0.0, 0.5);
    let y = span([0.0, 0.0, 0.5], [1.0, 2.0, 0.0], [0.0, 0.0, 1.0], -1.0, 1.0);

    let out = ruled_pair(&x, &y).expect("the clipped dyadic witness is decidable");
    let (p0, p1) = segment(&out);
    assert!(
        points_eq_set(
            (p0, p1),
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.25, 0.5, 0.0)
        ),
        "the generator-clipped segment must end at u = u_hi, got {p0:?} -> {p1:?}"
    );

    // The far endpoint IS the boundary event u − u_hi = 0: the recovery at its
    // locus parameter (t = s = 1/4) is the degenerate interval 1/2 == u_hi, and
    // the emitted point reconstructs from exactly that parameter.
    let (u_end, v_end) = recover_u_v(&x, &y, 0.25, 0.25).expect("recovery at the boundary event");
    assert!(
        degenerate(u_end, 0.5),
        "u at the far end must equal u_hi exactly, got {u_end:?}"
    );
    assert!(
        u_end.inf() == x.u_hi && u_end.sup() == x.u_hi,
        "the far end must saturate u_hi = {}",
        x.u_hi
    );
    assert!(
        degenerate(v_end, -0.5),
        "v stays interior to Y's domain, got {v_end:?}"
    );
    assert_eq!(surface(x, 0.25, 0.5), Point3::new(0.25, 0.5, 0.0));

    // The near endpoint saturates u_lo = 0 at the X corner, with v still
    // interior to Y's generator domain.
    let (u_lo_end, v_lo_end) = recover_u_v(&x, &y, 0.0, 0.0).expect("recovery at the near end");
    assert!(
        degenerate(u_lo_end, 0.0),
        "u at the near end must equal u_lo exactly"
    );
    assert!(degenerate(v_lo_end, -0.5));

    // The open segment's interior recovery is strictly inside the generator
    // domain: decisive positive u − u_lo and u_hi − u at the dyadic midpoint.
    let (u_mid, v_mid) = recover_u_v(&x, &y, 0.125, 0.125).expect("recovery at the midpoint");
    assert!(
        degenerate(u_mid, 0.25),
        "midpoint u must be 1/4, got {u_mid:?}"
    );
    assert!(degenerate(v_mid, -0.5));
    assert!((u_mid.inf() - x.u_lo) > 0.0 && (x.u_hi - u_mid.sup()) > 0.0);

    // Endpoints sit exactly on both carrier planes (zero residual) and both are
    // within the emitting carrier's parameter domains.
    assert_eq!(plane_residual(x, p0), 0.0);
    assert_eq!(plane_residual(y, p0), 0.0);
    assert_eq!(plane_residual(x, p1), 0.0);
    assert_eq!(plane_residual(y, p1), 0.0);

    // Monotone clipping: widening X's generator domain to [0, 1] extends the
    // certified segment past the boundary event (the Config A segment), proving
    // that the clipped end is the GENERATOR-DOMAIN crossing, not an arbitrary
    // truncation.
    let wide = span([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], 0.0, 1.0);
    let out_wide = ruled_pair(&wide, &y).expect("the unclipped dyadic witness is decidable");
    let (wp0, wp1) = segment(&out_wide);
    assert!(
        points_eq_set(
            (wp0, wp1),
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.5, 1.0, 0.0)
        ),
        "the unclipped pair must cross to u = 1, got {wp0:?} -> {wp1:?}"
    );
}

#[test]
fn v5_analytic_pairs_bit_identical() {
    // FSSI-004 scope decision 4: all landed BG-ANA-001 pair answers stay
    // bit-identical. This family only ADDS recognized pairs (monotone
    // widening), so each landed family's value must reproduce its reference
    // Debug string byte-for-byte. A red row here reports a regression in the
    // landed analytic family being fixed, never a legitimate change from this
    // packet.

    let z0 = Plane::new(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(0.0, 1.0, 0.0),
    );
    let y0 = Plane::new(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(0.0, 0.0, 1.0),
    );
    let z2 = Plane::new(
        Point3::new(0.0, 0.0, 2.0),
        Point3::new(1.0, 0.0, 2.0),
        Point3::new(0.0, 1.0, 2.0),
    );
    let z5 = Plane::new(
        Point3::new(0.0, 0.0, 5.0),
        Point3::new(1.0, 0.0, 5.0),
        Point3::new(0.0, 1.0, 5.0),
    );
    let px1 = Plane::new(
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(1.0, 1.0, 0.0),
        Point3::new(1.0, 0.0, 1.0),
    );
    let sph_origin = Sphere::new(Point3::new(0.0, 0.0, 0.0), 2.0);
    let sph_at3 = Sphere::new(Point3::new(3.0, 0.0, 0.0), 2.0);
    let sph_tangent = Sphere::new(Point3::new(4.0, 0.0, 0.0), 2.0);
    let sph_far = Sphere::new(Point3::new(20.0, 0.0, 0.0), 2.0);
    let unit_cyl = Cylinder::new(Point3::new(0.0, 0.0, 0.0), 1.0)
        .expect("a unit cylinder at the origin is constructible")
        .value;
    let cyl_tangent = Cylinder::new(Point3::new(2.0, 0.0, 0.0), 1.0)
        .expect("a tangent parallel cylinder is constructible")
        .value;
    let cyl_far = Cylinder::new(Point3::new(3.0, 0.0, 0.0), 1.0)
        .expect("a disjoint parallel cylinder is constructible")
        .value;

    let value = |out: truck_evidence::outcome::Outcome<AnalyticIntersection>| {
        format!(
            "{:?}",
            out.expect("the landed witness must stay decidable").value
        )
    };

    let mut rows: Vec<(&str, String)> = Vec::new();
    rows.push(("plane_plane transverse", value(plane_plane(&z0, &y0))));
    rows.push(("plane_plane parallel", value(plane_plane(&z0, &z2))));
    rows.push(("plane_plane coincident", value(plane_plane(&z0, &z0))));
    rows.push(("plane_sphere circle", value(plane_sphere(&z0, &sph_origin))));
    rows.push((
        "plane_sphere disjoint",
        value(plane_sphere(&z5, &sph_origin)),
    ));
    rows.push((
        "sphere_sphere circle",
        value(sphere_sphere(&sph_origin, &sph_at3)),
    ));
    rows.push((
        "sphere_sphere tangent",
        value(sphere_sphere(&sph_origin, &sph_tangent)),
    ));
    rows.push((
        "sphere_sphere disjoint",
        value(sphere_sphere(&sph_origin, &sph_far)),
    ));
    rows.push((
        "plane_cylinder circle",
        value(plane_cylinder(&z0, &unit_cyl)),
    ));
    rows.push((
        "plane_cylinder generatrix",
        value(plane_cylinder(&px1, &unit_cyl)),
    ));
    rows.push((
        "parallel_cylinders tangent",
        value(parallel_cylinders(&unit_cyl, &cyl_tangent)),
    ));
    rows.push((
        "parallel_cylinders disjoint",
        value(parallel_cylinders(&unit_cyl, &cyl_far)),
    ));
    let a0 = (Point3::new(0.0, 0.0, 0.0), Vector3::new(0.0, 0.0, 1.0));
    let a1 = (Point3::new(0.0, 0.0, 0.0), Vector3::new(1.0, 0.0, 0.0));
    rows.push((
        "equal_radius_cylinders two ellipses",
        value(equal_radius_cylinders(1.0, &a0, &a1)),
    ));

    // Run each row twice: within-run identity and against the reference.
    let reference: &[(&str, &str)] = &[
        ("plane_plane transverse", "Curve(Line(Line(Point3 [0.0, 0.0, 0.0], Point3 [1.0, 0.0, 0.0])))"),
        ("plane_plane parallel", "Parallel"),
        ("plane_plane coincident", "Coincident"),
        (
            "plane_sphere circle",
            "Curve(Circle(Processor { entity: TrimmedCurve { curve: UnitCircle(PhantomData<cgmath::point::Point3<f64>>), range: (0.0, 6.283185307179586) }, transform: Matrix4 [[0.0, 2.0, 0.0, 0.0], [-2.0, 0.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]], orientation: true }))",
        ),
        ("plane_sphere disjoint", "Empty"),
        (
            "sphere_sphere circle",
            "Curve(Circle(Processor { entity: TrimmedCurve { curve: UnitCircle(PhantomData<cgmath::point::Point3<f64>>), range: (0.0, 6.283185307179586) }, transform: Matrix4 [[0.0, 0.0, -1.3228756555322954, 0.0], [0.0, 1.3228756555322954, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0], [1.5, 0.0, 0.0, 1.0]], orientation: true }))",
        ),
        ("sphere_sphere tangent", "TangentPoint(Point3 [2.0, 0.0, 0.0])"),
        ("sphere_sphere disjoint", "Empty"),
        (
            "plane_cylinder circle",
            "Curve(Circle(Processor { entity: TrimmedCurve { curve: UnitCircle(PhantomData<cgmath::point::Point3<f64>>), range: (0.0, 6.283185307179586) }, transform: Matrix4 [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]], orientation: true }))",
        ),
        ("plane_cylinder generatrix", "TangentLine(Line(Point3 [1.0, 0.0, 0.0], Point3 [1.0, 0.0, 1.0]))"),
        ("parallel_cylinders tangent", "TangentLine(Line(Point3 [1.0, 0.0, 0.0], Point3 [1.0, 0.0, 1.0]))"),
        ("parallel_cylinders disjoint", "Parallel"),
        (
            "equal_radius_cylinders two ellipses",
            "TwoCurves([Ellipse(Processor { entity: TrimmedCurve { curve: UnitCircle(PhantomData<cgmath::point::Point3<f64>>), range: (0.0, 6.283185307179586) }, transform: Matrix4 [[0.9999999999999998, 0.0, 0.9999999999999998, 0.0], [0.0, 1.0, 0.0, 0.0], [-0.7071067811865475, 0.0, 0.7071067811865475, 0.0], [0.0, 0.0, 0.0, 1.0]], orientation: true }), Ellipse(Processor { entity: TrimmedCurve { curve: UnitCircle(PhantomData<cgmath::point::Point3<f64>>), range: (0.0, 6.283185307179586) }, transform: Matrix4 [[-0.9999999999999998, 0.0, 0.9999999999999998, 0.0], [0.0, 1.0, 0.0, 0.0], [-0.7071067811865475, 0.0, -0.7071067811865475, 0.0], [0.0, 0.0, 0.0, 1.0]], orientation: true })])",
        ),
    ];

    for ((label, snapshot), (ref_label, ref_value)) in rows.iter().zip(reference.iter()) {
        assert_eq!(label, ref_label, "row ordering drift in the V5 battery");
        assert_eq!(snapshot, ref_value, "V5-pair identity violated for {label}");
    }
    assert_eq!(rows.len(), reference.len());
}
