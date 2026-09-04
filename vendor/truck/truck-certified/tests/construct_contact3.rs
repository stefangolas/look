//! CC-020-CONTACT-K3 integration tests (spine seam S11): the three-support
//! constrained contact system over real certified surface maps — the
//! hand-computed trihedral in-sphere ground truth, the structural rank-drop
//! refusal, the certified no-root outcome, the constant-radius-law ground
//! truth, and the depth-capped bisection termination. The test names are the
//! contract.

#![deny(clippy::unwrap_used)]

use truck_base::evidence::Budget;
use truck_certified::certified_map::{admit_surface, CertifiedSurfaceMap};
use truck_certified::construct::contact3::{solve_triple_node, ReducedSystem, TripleNodeOutcome};
use truck_certified::construct::refusal::ConstructRefusal;
use truck_certified::construct::stubs::{RadiusLaw, TripleContactNode};
use truck_certified::formal::numeric::PositiveFinite;
use truck_certified::kernel::certs::IBox4;
use truck_geometry::prelude::{BSplineSurface, KnotVec, Point3};

/// A declared positive tau for the fixtures.
fn tau(value: f64) -> PositiveFinite {
    PositiveFinite::new(value).expect("a positive declared tau")
}

/// The affine surface `S(u, v) = base + u*du + v*dv` over `[0, 1]^2`: a
/// degree-(1, 1) tensor Bézier patch whose control net interpolates the affine
/// values at the four corners.
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
fn admitted(base: [f64; 3], du: [f64; 3], dv: [f64; 3]) -> CertifiedSurfaceMap {
    let surface = affine_surface(base, du, dv);
    admit_surface(&surface, tau(0.5)).expect("the affine surface fixture admits")
}

/// The coordinate planes of the first octant corner, each with its certified
/// `S_u × S_v` normal pointing INTO the octant (`+x`, `+y`, `+z`), so
/// `ε = +1`: `x = 0`, `y = 0`, `z = 0`.
fn corner_maps() -> [CertifiedSurfaceMap; 3] {
    [
        admitted([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
        admitted([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
        admitted([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
    ]
}

/// The coordinate planes of the opposite corner at `x = y = z = 1`, each with
/// its certified `S_u × S_v` normal pointing OUT of the corner octant, so
/// `ε = −1`: the in-sphere centre sits at `(1 − r, 1 − r, 1 − r)`.
fn far_corner_maps() -> [CertifiedSurfaceMap; 3] {
    [
        admitted([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
        admitted([0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
        admitted([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
    ]
}

/// The full certified parameter region of the fixtures.
const REGION: ((f64, f64), (f64, f64)) = ((0.0, 1.0), (0.0, 1.0));

/// A fresh budget generous enough never to bind on these fixtures.
fn budget() -> Budget {
    Budget::new(1 << 20, 1 << 20, 64)
}

/// Build the reduced system over three owned maps and a four-variable seed.
fn build_system(
    maps: [CertifiedSurfaceMap; 3],
    eps: [f64; 3],
    radius: f64,
    lo: [f64; 4],
    hi: [f64; 4],
) -> ReducedSystem {
    let refs = [&maps[0], &maps[1], &maps[2]];
    let seed = IBox4::try_new(lo, hi).expect("the seed box is valid");
    ReducedSystem::try_new(refs, [REGION; 3], eps, &RadiusLaw::Constant(radius), seed)
        .expect("the corner system builds")
}

/// Extract the `Ok` node of a certified solve; a refusal or an `Empty` here is
/// a test-bug panic (the fixture is designed to certify a node).
fn expect_node(result: Result<TripleNodeOutcome, ConstructRefusal>) -> TripleContactNode {
    match result {
        Ok(TripleNodeOutcome::Node(node)) => node,
        Ok(TripleNodeOutcome::Empty) => {
            panic!("a node-carrying fixture certified an empty box")
        }
        Err(refusal) => panic!("a node-carrying fixture refused: {refusal:?}"),
    }
}

#[test]
fn three_plane_wedge_node_matches_hand_computed_centre() {
    // Three planes forming the first-octant trihedral corner at the origin,
    // all offsets into the octant (`ε = +1`), constant radius law `r = 0.5`.
    // The in-sphere centre is the hand value `(0.5, 0.5, 0.5)` with radius
    // `0.5`, and each contact sits at parameter `(0.5, 0.5)` on its plane.
    let maps = corner_maps();
    let lo = [0.4; 4];
    let hi = [0.6; 4];
    let system = build_system(maps, [1.0, 1.0, 1.0], 0.5, lo, hi);
    let mut budget = budget();
    let node = expect_node(solve_triple_node(&system, &mut budget));

    for axis in 0..3 {
        assert!(
            node.centre[axis].contains(0.5),
            "centre axis {axis} contains the hand value 0.5: {:?}",
            node.centre[axis]
        );
    }
    assert!(
        node.radius.contains(0.5),
        "radius contains the hand value 0.5: {:?}",
        node.radius
    );
    for (i, pair) in node.contacts.iter().enumerate() {
        for (j, parameter) in pair.iter().enumerate() {
            assert!(
                parameter.contains(0.5),
                "contact {i} parameter {j} contains 0.5: {parameter:?}"
            );
        }
    }
}

#[test]
fn submersion_margin_below_threshold_refuses_rank_deficient_contact() {
    // Two coincident support planes (x = 0 twice) plus a third plane: the rank
    // drop is structural (two identical offset rows), so the certified
    // submersion margin `η_F` is below the `CC_ETA_J`-class floor and the
    // solver must refuse `RankDeficientContact` before any iteration.
    let maps = [
        admitted([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
        admitted([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
        admitted([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
    ];
    let lo = [0.4; 4];
    let hi = [0.6; 4];
    let system = build_system(maps, [1.0, 1.0, 1.0], 0.5, lo, hi);
    let mut budget = budget();
    match solve_triple_node(&system, &mut budget) {
        Err(ConstructRefusal::RankDeficientContact) => {}
        Ok(outcome) => panic!("a rank-deficient triple must refuse, got {outcome:?}"),
        Err(refusal) => panic!("expected RankDeficientContact, got {refusal:?}"),
    }
}

#[test]
fn no_root_in_box_returns_no_root_not_refusal() {
    // The radius-law closure `r = 0.5` is disjoint from the seed's radius
    // window `[0.6, 0.9]`, so the residual excludes zero over the whole box: a
    // certified absence, returned as the typed `Empty` outcome — never a
    // refusal.
    let maps = corner_maps();
    let lo = [0.6, 0.6, 0.6, 0.6];
    let hi = [0.9, 0.9, 0.9, 0.9];
    let system = build_system(maps, [1.0, 1.0, 1.0], 0.5, lo, hi);
    let mut budget = budget();
    match solve_triple_node(&system, &mut budget) {
        Ok(TripleNodeOutcome::Empty) => {}
        Ok(TripleNodeOutcome::Node(node)) => {
            panic!("a root-free box certified a node: {node:?}")
        }
        Err(refusal) => panic!("a certified absence must be Empty, not a refusal: {refusal:?}"),
    }
}

#[test]
fn radius_law_node_matches_constant_radius_ground_truth() {
    // The opposite-corner trihedral at `x = y = z = 1` with the offsets into
    // the corner octant (`ε = −1`) and the constant radius law `r = 0.75`: the
    // in-sphere centre is the hand value `(1 − r, 1 − r, 1 − r) = (0.25, 0.25,
    // 0.25)` and each contact parameter is `0.25` on its plane.
    let maps = far_corner_maps();
    let lo = [0.15, 0.15, 0.15, 0.65];
    let hi = [0.35, 0.35, 0.35, 0.85];
    let system = build_system(maps, [-1.0, -1.0, -1.0], 0.75, lo, hi);
    let mut budget = budget();
    let node = expect_node(solve_triple_node(&system, &mut budget));

    for axis in 0..3 {
        assert!(
            node.centre[axis].contains(0.25),
            "centre axis {axis} contains the hand value 0.25: {:?}",
            node.centre[axis]
        );
    }
    assert!(
        node.radius.contains(0.75),
        "radius contains the law value 0.75: {:?}",
        node.radius
    );
    for (i, pair) in node.contacts.iter().enumerate() {
        for (j, parameter) in pair.iter().enumerate() {
            assert!(
                parameter.contains(0.25),
                "contact {i} parameter {j} contains 0.25: {parameter:?}"
            );
        }
    }
}

#[test]
fn seed_box_bisection_terminates_within_depth_cap() {
    // The certified root `(0.5, 0.5, 0.5, 0.5)` sits exactly on the lower
    // corner of every seed axis, so the Krawczyk image never lies strictly
    // inside a cell of the containing chain and the engine must subdivide
    // rather than certify on the first box. The subdivision is bounded by the
    // `CC_DEPTH_MAX` cap: an unresolved cell at the cap refuses
    // `RankDeficientContact` (termination), and a pre-cap resolution returns
    // the certified `Empty` — never a `Node`, and never a hang.
    let maps = corner_maps();
    let lo = [0.5; 4];
    let hi = [1.0; 4];
    let system = build_system(maps, [1.0, 1.0, 1.0], 0.5, lo, hi);
    let mut budget = budget();
    match solve_triple_node(&system, &mut budget) {
        Err(ConstructRefusal::RankDeficientContact) => {}
        Ok(TripleNodeOutcome::Empty) => {}
        Ok(TripleNodeOutcome::Node(node)) => {
            panic!("a boundary-root seed must not certify an interior node: {node:?}")
        }
        Err(refusal) => panic!("expected depth-cap termination, got {refusal:?}"),
    }
}
