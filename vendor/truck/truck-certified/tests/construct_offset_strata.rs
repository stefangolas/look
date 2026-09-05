//! CC-021-OFFSET-STRATA integration tests (spine seams S10/S11 consumers): the
//! rounded-offset contact-complex strata — the k=1 face stratum with its
//! certified `J_t = 1 − 2Ht + Kt²` lower bound over real certified surface
//! maps, the k=2 edge stratum routed through the S10 canal-regularity seam,
//! the k=3 corner stratum routed through the S11 triple-node solve, and the
//! certified reach bounds `ρ_A` (`|t|` exactly for the ball strata, the
//! certified centre-to-source bound for the corner). The test names are the
//! contract.

#![deny(clippy::unwrap_used)]

use truck_base::evidence::Budget;
use truck_certified::certified_map::{
    admit_curve, admit_surface, CertifiedCurveMap, CertifiedSurfaceMap,
};
use truck_certified::construct::canal::canal_regularity;
use truck_certified::construct::offset_strata::{
    corner_stratum, edge_stratum, face_stratum, OffsetStratum, StratumId, StratumRefusal,
};
use truck_certified::construct::refusal::ConstructRefusal;
use truck_certified::construct::stubs::RadiusLaw;
use truck_certified::construct::Interval;
use truck_certified::formal::numeric::PositiveFinite;
use truck_certified::kernel::certs::IBox4;
use truck_geometry::prelude::{BSplineCurve, BSplineSurface, KnotVec, Point3};

/// A declared positive tau for the fixtures.
fn tau(value: f64) -> PositiveFinite {
    PositiveFinite::new(value).expect("a positive declared tau")
}

/// The bilinear plane face `S(u, v) = (u, 2v, 0)` over `[0, 1]^2`: flat
/// (`‖D²S‖ ≡ 0`), with certified margin `|S_u × S_v| = 2`. A flat face has
/// `J_t = 1` exactly for every offset — never focal.
fn plane_face() -> BSplineSurface<Point3> {
    let uknot = KnotVec::bezier_knot(1);
    let vknot = KnotVec::bezier_knot(1);
    let ctrl_pts = vec![
        vec![Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 2.0, 0.0)],
        vec![Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 2.0, 0.0)],
    ];
    BSplineSurface::new((uknot, vknot), ctrl_pts)
}

/// The parabolic face `S(u, v) = (2u, 2v, u²)` over `[0, 1]^2` (the CC-002
/// `curved_patch` data): `S_u × S_v = (−4u, 0, 4)` so `σ* = 4` at `u = 0`, the
/// only nonzero second partial is `S_uu = (0, 0, 2)`, and the parabola
/// `z = x²/4` carries max principal curvature magnitude `κ_max = 1/2` at the
/// apex (`u = 0`). The exact `J_t` infimum over the face for an offset toward
/// the curvature centre is `1 − |t|·κ_max`; the certified `J_t` lower bound
/// must certify strictly positive for small `|t|` and refuse FocalDegeneracy
/// once the certified focal margin `1/c` is at or below `|t|`.
fn parabolic_face() -> BSplineSurface<Point3> {
    let uknot = KnotVec::bezier_knot(2);
    let vknot = KnotVec::bezier_knot(1);
    let ctrl_pts = vec![
        vec![Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 2.0, 0.0)],
        vec![Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 2.0, 0.0)],
        vec![Point3::new(2.0, 0.0, 1.0), Point3::new(2.0, 2.0, 1.0)],
    ];
    BSplineSurface::new((uknot, vknot), ctrl_pts)
}

/// Admit a surface fixture with the given declared tau, panicking only on a
/// test-bug refusal.
fn admitted_surface(surface: &BSplineSurface<Point3>, value: f64) -> CertifiedSurfaceMap {
    admit_surface(surface, tau(value)).expect("the surface fixture admits")
}

/// The unit-speed straight spine `C(t) = (t, 0, 0)` over `[0, 1]`: `C'' = 0`,
/// so any constant-radius canal over it certifies regular.
fn straight_line_spine() -> BSplineCurve<Point3> {
    let knot = KnotVec::bezier_knot(1);
    let points = vec![Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)];
    BSplineCurve::new(knot, points)
}

/// The unit-circle data spine (the CC-000 `curved_patch`/circle data): the
/// parabola `C(t) = (t, t²/2)` over `[-1, 1]` with `‖C″‖ = 1`, so the pipe
/// condition for a constant radius is `r·‖C″‖ = r < 1`.
fn unit_circle_data_spine() -> BSplineCurve<Point3> {
    let knot = KnotVec::from(vec![-1.0, -1.0, -1.0, 1.0, 1.0, 1.0]);
    let points = vec![
        Point3::new(-1.0, 0.5, 0.0),
        Point3::new(0.0, -0.5, 0.0),
        Point3::new(1.0, 0.5, 0.0),
    ];
    BSplineCurve::new(knot, points)
}

/// Admit a curve fixture with the given declared tau, panicking only on a
/// test-bug refusal.
fn admitted_curve(curve: &BSplineCurve<Point3>, value: f64) -> CertifiedCurveMap {
    admit_curve(curve, tau(value)).expect("the curve fixture admits")
}

/// The affine surface `S(u, v) = base + u*du + v*dv` over `[0, 1]^2`.
fn affine_surface(base: [f64; 3], du: [f64; 3], dv: [f64; 3]) -> BSplineSurface<Point3> {
    let uknot = KnotVec::bezier_knot(1);
    let vknot = KnotVec::bezier_knot(1);
    let add = |a: [f64; 3], b: [f64; 3]| [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
    let p = |v: [f64; 3]| Point3::new(v[0], v[1], v[2]);
    let ctrl = vec![
        vec![p(base), p(add(base, dv))],
        vec![p(add(base, du)), p(add(add(base, du), dv))],
    ];
    BSplineSurface::new((uknot, vknot), ctrl)
}

/// Admit one affine surface fixture with a declared tau of `0.5` (the unit
/// tangent-frame fixtures certify a rank margin of `1`).
fn admitted_affine(base: [f64; 3], du: [f64; 3], dv: [f64; 3]) -> CertifiedSurfaceMap {
    let surface = affine_surface(base, du, dv);
    admit_surface(&surface, tau(0.5)).expect("the affine surface fixture admits")
}

/// The three coordinate planes of the first-octant trihedral corner at the
/// origin, each with its certified `S_u × S_v` normal pointing INTO the octant
/// (`+x`, `+y`, `+z`), so `ε = +1`.
fn corner_maps() -> [CertifiedSurfaceMap; 3] {
    [
        admitted_affine([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
        admitted_affine([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
        admitted_affine([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
    ]
}

/// The full certified parameter region of the fixtures.
const REGION: ((f64, f64), (f64, f64)) = ((0.0, 1.0), (0.0, 1.0));

/// A fresh budget generous enough never to bind on these fixtures.
fn budget() -> Budget {
    Budget::new(1 << 20, 1 << 20, 64)
}

/// The exact Euclidean distance between two axis-aligned boxes (the axis gaps
/// are independent, so this is the exact minimum point-pair distance).
fn box_distance(a: &[Interval; 3], b: &[Interval; 3]) -> f64 {
    let mut sum = 0.0_f64;
    for k in 0..3 {
        let gap = axis_gap(&a[k], &b[k]);
        sum += gap * gap;
    }
    sum.sqrt()
}

/// The gap of two closed intervals on one axis (`0` when they touch).
fn axis_gap(a: &Interval, b: &Interval) -> f64 {
    if a.hi < b.lo {
        b.lo - a.hi
    } else if b.hi < a.lo {
        a.lo - b.hi
    } else {
        0.0
    }
}

/// The carried record of a certified face stratum.
struct FaceFields {
    offset: f64,
    j_t_lower: Interval,
}

/// The carried record of a certified edge stratum.
struct EdgeFields {
    radius: f64,
    canal: Interval,
}

/// The carried record of a certified corner stratum.
struct CornerFields {
    centre: [Interval; 3],
    radius: Interval,
    contacts: [[Interval; 2]; 3],
}

/// Destructure a `Face` stratum; a different variant here is a test-bug panic.
fn face_fields(stratum: OffsetStratum) -> FaceFields {
    match stratum {
        OffsetStratum::Face {
            offset, j_t_lower, ..
        } => FaceFields { offset, j_t_lower },
        other => panic!("expected a Face stratum, got {other:?}"),
    }
}

/// Destructure an `Edge` stratum; a different variant here is a test-bug panic.
fn edge_fields(stratum: OffsetStratum) -> EdgeFields {
    match stratum {
        OffsetStratum::Edge { radius, canal, .. } => EdgeFields { radius, canal },
        other => panic!("expected an Edge stratum, got {other:?}"),
    }
}

/// Destructure a `Corner` stratum; a different variant here is a test-bug panic.
fn corner_fields(stratum: OffsetStratum) -> CornerFields {
    match stratum {
        OffsetStratum::Corner { node } => CornerFields {
            centre: node.centre,
            radius: node.radius,
            contacts: node.contacts,
        },
        other => panic!("expected a Corner stratum, got {other:?}"),
    }
}

#[test]
fn face_stratum_focal_margin_bounds_j_t_from_below() {
    // The parabolic face has max principal curvature magnitude 1/2 at the
    // apex, so the exact J_t infimum over the face for the offset t = 0.25
    // (toward the curvature centre) is 1 - 0.25 * 1/2 = 0.875, and the focal
    // distance is 2. The certified lower bound must be strictly positive (the
    // offset sits well inside the certified focal margin) and must never
    // overclaim the exact infimum: a certified bound "from below" stays at or
    // under 0.875.
    let surface = parabolic_face();
    let map = admitted_surface(&surface, 1.0);
    let stratum = face_stratum(&map, 0.25).expect("the offset 0.25 face certifies");
    assert_eq!(stratum.reach_bound(), 0.25); // H-3: exact ball-stratum reach |t|
    let fields = face_fields(stratum);
    assert_eq!(fields.offset, 0.25);
    assert!(
        fields.j_t_lower.lo > 0.0,
        "the certified J_t lower bound is strictly positive: {:?}",
        fields.j_t_lower
    );
    assert!(
        fields.j_t_lower.lo <= 0.875, // H-3: certified lower bound never overclaims the exact infimum 1 - t*1/2
        "the certified J_t lower bound stays at or below the exact infimum 0.875: {}",
        fields.j_t_lower.lo
    );
    assert!(
        fields.j_t_lower.lo <= fields.j_t_lower.hi,
        "the certified J_t enclosure is ordered: {:?}",
        fields.j_t_lower
    );

    // The opposite offset sign is symmetric under the certified bound (the v1
    // composition is sign-agnostic): -0.25 also certifies with the same
    // strictly positive margin.
    let neg = face_stratum(&map, -0.25).expect("the offset -0.25 face certifies");
    let neg_fields = face_fields(neg);
    assert!(
        neg_fields.j_t_lower.lo > 0.0,
        "the negative-offset margin is positive: {:?}",
        neg_fields.j_t_lower
    );
    assert_eq!(neg_fields.j_t_lower.lo, fields.j_t_lower.lo); // H-3: identical |t| certified margin

    // A flat face certifies J_t = 1 exactly for ANY offset (no curvature, no
    // focal set), so the composition never mistakes an unbounded focal margin
    // for degeneracy.
    let flat_map = admitted_surface(&plane_face(), 0.5);
    let flat = face_stratum(&flat_map, 1.25).expect("the flat face certifies");
    let flat_fields = face_fields(flat);
    assert!(
        flat_fields.j_t_lower.lo > 0.0,
        "the flat-face J_t lower bound is strictly positive: {:?}",
        flat_fields.j_t_lower
    );
    assert_eq!(flat_fields.j_t_lower.lo, 1.0); // H-3: J_t = 1 exactly for a plane (c = 0)
    assert_eq!(flat_fields.j_t_lower.hi, 1.0); // H-3: J_t = 1 exactly for a plane (c = 0)
}

#[test]
fn face_stratum_refuses_focal_degeneracy_when_margin_straddles_zero() {
    // On the parabolic face the certified curvature bound c (the composition's
    // conservative value) gives |t|·c >= 1 for |t| = 1.0: the certified J_t
    // margin straddles the focal threshold and the face stratum must refuse
    // FocalDegeneracy naming the face — for both offset signs.
    let surface = parabolic_face();
    let map = admitted_surface(&surface, 1.0);
    for offset in [1.0, -1.0] {
        match face_stratum(&map, offset) {
            Err(StratumRefusal {
                stratum: StratumId::Face,
                refusal: ConstructRefusal::FocalDegeneracy,
            }) => {}
            Ok(stratum) => panic!("a focal face certified at offset {offset}: {stratum:?}"),
            Err(refusal) => panic!("expected FocalDegeneracy on the face, got {refusal:?}"),
        }
    }

    // The refusal is curvature-driven, never a flat rejection of large
    // offsets: a flat face certifies far beyond the curved face's refusal
    // point, because its certified curvature bound is exactly zero.
    let flat_map = admitted_surface(&plane_face(), 0.5);
    let flat = face_stratum(&flat_map, 5.0).expect("the flat face certifies any offset");
    assert_eq!(flat.reach_bound(), 5.0); // H-3: flat face reach is |t| exactly
}

#[test]
fn edge_stratum_routes_through_canal_regularity() {
    // The unit-circle data spine (‖C″‖ = 1) with constant radius 0.5 certifies
    // the canal over the whole arc: the edge stratum's carried criterion value
    // is EXACTLY the S10 seam's own answer for the same spine, law, and arc —
    // the routing is a pass-through, not a re-derivation.
    let curve = unit_circle_data_spine();
    let map = admitted_curve(&curve, 0.5);
    let stratum = edge_stratum(&map, 0.5).expect("the radius-0.5 edge certifies");
    assert_eq!(stratum.reach_bound(), 0.5); // H-3: edge reach is |t| exactly
    let fields = edge_fields(stratum);
    assert_eq!(fields.radius, 0.5);
    let direct = canal_regularity(&map, &RadiusLaw::Constant(0.5), (-1.0, 1.0))
        .expect("the direct S10 call certifies the same edge");
    assert!(
        fields.canal.lo > 0.0,
        "the edge canal criterion is strictly positive: {:?}",
        fields.canal
    );
    assert_eq!(
        fields.canal.lo, direct.lo,
        "the edge stratum carries the S10 criterion value"
    ); // H-3: same seam computation
    assert_eq!(
        fields.canal.hi, direct.hi,
        "the edge stratum carries the S10 criterion value"
    ); // H-3: same seam computation

    // A radius whose pipe condition fails on the spine (radius * sup ||C''|| =
    // 1.5 >= 1) refuses through the S10 seam, propagated as a CanalSingular
    // naming the edge stratum.
    match edge_stratum(&map, 1.5) {
        Err(StratumRefusal {
            stratum: StratumId::Edge,
            refusal: ConstructRefusal::CanalSingular,
        }) => {}
        Ok(stratum) => panic!("a singular edge certified: {stratum:?}"),
        Err(refusal) => panic!("expected CanalSingular on the edge, got {refusal:?}"),
    }

    // A straight spine (C'' = 0) has no pipe-condition bound: any positive
    // radius certifies. A zero offset magnitude, though, is an invalid edge
    // request (a zero-radius canal degenerates to the spine itself).
    let line = straight_line_spine();
    let cmap = admitted_curve(&line, 0.5);
    let straight = edge_stratum(&cmap, 0.5).expect("the straight edge certifies");
    assert_eq!(straight.reach_bound(), 0.5); // H-3: edge reach is the constant radius
    match edge_stratum(&cmap, 0.0) {
        Err(StratumRefusal {
            stratum: StratumId::Edge,
            refusal: ConstructRefusal::InvalidInput,
        }) => {}
        Ok(stratum) => panic!("a zero-radius edge certified: {stratum:?}"),
        Err(refusal) => panic!("expected InvalidInput for a zero-radius edge, got {refusal:?}"),
    }
}

#[test]
fn corner_stratum_matches_triple_node_centre() {
    // Three planes forming the first-octant trihedral corner at the origin,
    // all offsets into the octant (ε = +1), offset magnitude 0.5. The
    // rolling-ball centre is the hand value (0.5, 0.5, 0.5) with radius 0.5,
    // and each contact sits at parameter (0.5, 0.5) on its plane — the S11
    // ground truth the contact3 packet certifies, reached here through the
    // offset corner stratum.
    let maps = corner_maps();
    let refs = [&maps[0], &maps[1], &maps[2]];
    let seed = IBox4::try_new([0.4; 4], [0.6; 4]).expect("the corner seed box is valid");
    let mut budget = budget();
    let stratum = corner_stratum(refs, [REGION; 3], [1.0, 1.0, 1.0], 0.5, seed, &mut budget)
        .expect("the corner stratum certifies");
    let fields = corner_fields(stratum);
    for axis in 0..3 {
        assert!(
            fields.centre[axis].contains(0.5),
            "corner centre axis {axis} contains the hand value 0.5: {:?}",
            fields.centre[axis]
        );
    }
    assert!(
        fields.radius.contains(0.5),
        "corner radius contains the hand value 0.5: {:?}",
        fields.radius
    );
    for (i, pair) in fields.contacts.iter().enumerate() {
        for (j, parameter) in pair.iter().enumerate() {
            assert!(
                parameter.contains(0.5),
                "corner contact {i} parameter {j} contains 0.5: {parameter:?}"
            );
        }
    }
}

#[test]
fn reach_bound_is_exact_for_ball_strata() {
    // Face strata are ball strata: reach_bound() is |t| EXACTLY (asserted by
    // equality, not by a tolerance), for both offset signs, on the flat face
    // and on the curved face alike.
    let flat_map = admitted_surface(&plane_face(), 0.5);
    for (offset, expected) in [(1.25, 1.25), (-0.75, 0.75), (0.0, 0.0)] {
        let face = face_stratum(&flat_map, offset).expect("the flat face certifies");
        assert_eq!(face.reach_bound(), expected); // H-3: face reach is |t| exactly
    }
    let curved_map = admitted_surface(&parabolic_face(), 1.0);
    let curved_face =
        face_stratum(&curved_map, 0.25).expect("the curved face certifies at t = 0.25");
    assert_eq!(curved_face.reach_bound(), 0.25); // H-3: face reach is |t| exactly

    // Edge strata are ball strata too: reach_bound() is the constant canal
    // radius |t| EXACTLY.
    let line = straight_line_spine();
    let cmap = admitted_curve(&line, 0.5);
    for (offset, expected) in [(0.5, 0.5), (-0.5, 0.5)] {
        let edge = edge_stratum(&cmap, offset).expect("the straight edge certifies");
        assert_eq!(edge.reach_bound(), expected); // H-3: edge reach is |t| exactly
    }

    // Corner strata carry the certified centre-to-source bound: the node
    // radius upper endpoint. The reach gate asserts it dominates the distance
    // from the node-centre enclosure to EACH support's bounding box — a sound
    // lower-bound sanity check (recorded in the module doc as not the exact
    // supremum). On the symmetric corner the centre sits at distance |t| from
    // every support bbox, so the bound is at least that distance.
    let maps = corner_maps();
    let refs = [&maps[0], &maps[1], &maps[2]];
    let seed = IBox4::try_new([0.4; 4], [0.6; 4]).expect("the corner seed box is valid");
    let mut budget = budget();
    let corner = corner_stratum(refs, [REGION; 3], [1.0, 1.0, 1.0], 0.5, seed, &mut budget)
        .expect("the corner stratum certifies");
    let reach = corner.reach_bound();
    let fields = corner_fields(corner);
    for map in &maps {
        let support_box = map
            .enclosure(REGION)
            .expect("the support map encloses its bounding box");
        let distance = box_distance(&fields.centre, &support_box);
        assert!(
            reach >= distance,
            "the corner reach {reach} dominates the centre-to-support-bbox distance {distance}"
        );
    }
}
