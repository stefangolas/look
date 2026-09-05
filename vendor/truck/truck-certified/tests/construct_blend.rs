//! CC-030-BLEND-SPINE integration tests (spine seam S12): the two-support
//! rolling-ball blend trace over real certified surface maps — the two-plane
//! chain that walks and terminates at trim events, the event-isolation
//! invariant of the discrete state, the certified collision stop, the P6
//! shared triple node across three incident branches, and the no-speculation
//! stop of an undecided step. The test names are the contract.

#![deny(clippy::unwrap_used)]

use truck_base::evidence::Budget;
use truck_certified::certified_map::{admit_surface, CertifiedSurfaceMap};
use truck_certified::construct::blend::{
    trace_blend_chain, trace_branch_steps, BlendTrace, BranchRefusal, BranchSeed,
    ClearanceBoundary, SupportChart,
};
use truck_certified::construct::refusal::ConstructRefusal;
use truck_certified::construct::stubs::{EventKind, RadiusLaw};
use truck_certified::formal::numeric::PositiveFinite;
use truck_certified::kernel::patch::IBox3;
use truck_evidence::clear::BallAdmissibility;
use truck_geometry::prelude::{BSplineSurface, KnotVec, Point3};

/// A declared positive tau for the fixtures.
fn tau(value: f64) -> PositiveFinite {
    PositiveFinite::new(value).expect("a positive declared tau")
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

/// Admit one affine surface fixture with a declared tau of `0.5`.
fn admitted(base: [f64; 3], du: [f64; 3], dv: [f64; 3]) -> CertifiedSurfaceMap {
    let surface = affine_surface(base, du, dv);
    admit_surface(&surface, tau(0.5)).expect("the affine surface fixture admits")
}

/// The full certified parameter region of the fixtures.
const REGION: ((f64, f64), (f64, f64)) = ((0.0, 1.0), (0.0, 1.0));

/// A support chart over one admitted face with the given side sign.
fn chart(map: CertifiedSurfaceMap, side: f64) -> SupportChart {
    SupportChart::try_new(map, REGION, side).expect("the affine face builds a chart")
}

/// The certified box of a centre with the standard half-width.
fn box_of(centre: [f64; 3]) -> IBox3 {
    IBox3::try_new(
        [
            centre[0] - 0.02,
            centre[1] - 0.02,
            centre[2] - 0.02,
        ],
        [
            centre[0] + 0.02,
            centre[1] + 0.02,
            centre[2] + 0.02,
        ],
    )
    .expect("the seed box is valid")
}

/// The floor face `z = 0` with its normal `+z` into the walk region.
fn face_a() -> CertifiedSurfaceMap {
    admitted([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0])
}

/// The wall face `y = 0` with its normal `+y` into the walk region.
fn face_b() -> CertifiedSurfaceMap {
    admitted([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0])
}

/// The far wall face `x = 1` whose normal `+x` points away from the ball
/// (`ε = −1`).
fn face_c() -> CertifiedSurfaceMap {
    admitted([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0])
}

/// A fresh budget generous enough never to bind on the completed fixtures.
fn budget() -> Budget {
    Budget::new(1 << 20, 1 << 20, 64)
}

/// Extract the `Ok` branch of a certified walk; a refusal is a test-bug panic.
fn expect_trace(result: Result<BlendTrace, ConstructRefusal>) -> BlendTrace {
    match result {
        Ok(trace) => trace,
        Err(refusal) => panic!("a walk that must succeed refused: {refusal:?}"),
    }
}

/// The constant-radius chain between the floor and the `y = 0` wall, seeded at
/// arc position `x0` with the given radius.
fn groove_seed(x0: f64, radius: f64, clearance: Option<ClearanceBoundary>) -> BranchSeed {
    let first = chart(face_a(), 1.0);
    let second = chart(face_b(), 1.0);
    BranchSeed::try_new(first, second, box_of([x0, radius, radius]), None, clearance)
        .expect("the groove seed builds")
}

#[test]
fn two_plane_chain_walks_and_terminates_at_trim_events() {
    // Two planar supports (the floor `z = 0` and the wall `y = 0`) with a
    // constant rolling radius `ρ = 0.25`: the branch centre line is the
    // straight line `c = (x, ρ, ρ)`. Seeded at `x = 0.3`, the walk advances in
    // the `+x` direction until the contact parameter on both supports reaches
    // the trim boundary `x = 1`, and in the `−x` direction until it reaches
    // the trim boundary `x = 0`. Both directions terminate at certified Trim
    // events whose arc enclosures contain the hand values `1 − 0.3` and
    // `0 − 0.3`.
    let seed = groove_seed(0.3, 0.25, None);
    let mut budget = budget();
    let trace = expect_trace(trace_blend_chain(&[seed], &RadiusLaw::Constant(0.25), &mut budget));

    let trims: Vec<_> = trace
        .events
        .iter()
        .filter(|event| event.kind == EventKind::Trim)
        .collect();
    assert_eq!(trims.len(), 2, "both directions terminate at a trim event");
    assert_eq!(trace.events.len(), 2, "exactly the two terminal events");

    let high = &trace.events[0];
    let low = &trace.events[1];
    assert_eq!(high.kind, EventKind::Trim);
    assert_eq!(low.kind, EventKind::Trim);
    assert!(
        high.at.contains(0.7),
        "the +x trim sits at arc 1 - 0.3 = 0.7: {:?}",
        high.at
    );
    assert!(
        low.at.contains(-0.3),
        "the -x trim sits at arc 0 - 0.3 = -0.3: {:?}",
        low.at
    );
    assert!(
        high.at.lo > low.at.hi,
        "the +x event lies after the -x event along the walk"
    );
}

#[test]
fn event_isolation_holds_between_certified_events() {
    // Between the two certified trim events the topology is FIXED: every
    // accepted certified step carries the IDENTICAL discrete state `Σ`
    // (both supports in contact, feet certified interior, the ball clear, the
    // rank and radius margins certified), and no accepted step crosses an
    // event boundary — every step's certified arc lies strictly between the
    // two terminal event arcs.
    let seed = groove_seed(0.3, 0.25, None);
    let mut budget = budget();
    let branch = trace_branch_steps(&seed, &RadiusLaw::Constant(0.25), &mut budget)
        .expect("the two-plane branch completes");
    assert!(
        branch.steps.len() >= 3,
        "the walk certifies interior steps between the events"
    );
    assert_eq!(branch.events.len(), 2, "exactly the two trim events");
    assert!(branch.events.iter().all(|e| e.kind == EventKind::Trim));

    let high_at = &branch.events[0].at;
    let low_at = &branch.events[1].at;
    let interior_state = branch.steps[0].state;
    assert!(interior_state.first_in_contact && interior_state.second_in_contact);
    assert!(interior_state.feet_interior);
    assert!(interior_state.clearance_clear);
    assert!(interior_state.rank_regular);
    assert!(interior_state.radius_admissible);
    for step in &branch.steps {
        assert_eq!(
            step.state, interior_state,
            "every accepted step between the events carries the identical state"
        );
        assert!(
            step.arc.lo > low_at.hi,
            "no accepted step crosses the -x event boundary"
        );
        assert!(
            step.arc.hi < high_at.lo,
            "no accepted step crosses the +x event boundary"
        );
    }
}

#[test]
fn clear_loss_stops_the_branch_as_collision_event() {
    // The same two-plane groove, but an excluded wall `x >= 0.7` is declared
    // ahead of the seed. Rolling toward the wall the P5 predicate
    // (`ball_clearance`) first certifies the ball clear of the wall and then
    // flips to Rejected: the walk stops at the certified Collision event at
    // the tangency `x = W − ρ = 0.7 − 0.25 = 0.45`, arc `0.45 − 0.15 = 0.3`.
    // The other direction still terminates at the trim `x = 0`, arc −0.15.
    let clearance = ClearanceBoundary {
        origin: [0.7, 0.0, 0.0],
        normal: [1.0, 0.0, 0.0],
        mode: BallAdmissibility::Round,
    };
    let seed = groove_seed(0.15, 0.25, Some(clearance));
    let mut budget = budget();
    let trace = expect_trace(trace_blend_chain(&[seed], &RadiusLaw::Constant(0.25), &mut budget));

    let collisions: Vec<_> = trace
        .events
        .iter()
        .filter(|event| event.kind == EventKind::Collision)
        .collect();
    let trims: Vec<_> = trace
        .events
        .iter()
        .filter(|event| event.kind == EventKind::Trim)
        .collect();
    assert_eq!(collisions.len(), 1, "exactly one certified collision event");
    assert_eq!(trims.len(), 1, "the other direction still trims");
    assert_eq!(trace.events.len(), 2);
    assert!(
        collisions[0].at.contains(0.3),
        "the collision tangency sits at arc W - rho - x0 = 0.3: {:?}",
        collisions[0].at
    );
    assert!(
        collisions[0].node.is_none(),
        "a pure collision carries no triple node"
    );
    assert!(
        trims[0].at.contains(-0.15),
        "the trim sits at arc 0 - 0.15 = -0.15: {:?}",
        trims[0].at
    );
}

#[test]
fn triple_node_joins_three_branches_exactly() {
    // The trihedral corner of the floor `z = 0`, the wall `y = 0`, and the far
    // wall `x = 1` carries a constant-radius rolling ball (`ρ = 0.5`) whose
    // centre at the corner node is the hand value `(1 − ρ, ρ, ρ) = (0.5, 0.5,
    // 0.5)`. The three pairwise branches (floor/wall, floor/far-wall,
    // wall/far-wall) each walk to that node, and each terminates at a
    // ThirdFace event referencing the SAME certified triple node (P6: solved
    // once, referenced).
    let a = chart(face_a(), 1.0);
    let b = chart(face_b(), 1.0);
    let c = chart(face_c(), -1.0);

    // Floor/wall branch along `x`, seeded inside `(0, 1 − ρ)`.
    let ab = BranchSeed::try_new(
        a.clone(),
        b.clone(),
        box_of([0.3, 0.5, 0.5]),
        Some(c.clone()),
        None,
    )
    .expect("the AB seed builds");
    // Floor/far-wall branch along `y`, seeded inside `(ρ, 1)`.
    let ac = BranchSeed::try_new(
        a.clone(),
        c.clone(),
        box_of([0.5, 0.8, 0.5]),
        Some(b.clone()),
        None,
    )
    .expect("the AC seed builds");
    // Wall/far-wall branch along `z`, seeded inside `(ρ, 1)`.
    let bc = BranchSeed::try_new(
        b,
        c,
        box_of([0.5, 0.5, 0.8]),
        Some(a),
        None,
    )
    .expect("the BC seed builds");

    let mut budget = budget();
    let trace = expect_trace(trace_blend_chain(
        &[ab, ac, bc],
        &RadiusLaw::Constant(0.5),
        &mut budget,
    ));

    let third_face: Vec<_> = trace
        .events
        .iter()
        .filter(|event| event.kind == EventKind::ThirdFace)
        .collect();
    assert_eq!(
        third_face.len(),
        3,
        "the three pairwise branches each join at the node"
    );
    assert_eq!(
        trace.events.len(),
        6,
        "three nodes and three trims terminate the three branches"
    );

    let reference = third_face[0]
        .node
        .as_ref()
        .expect("a ThirdFace event carries its node");
    for axis in 0..3 {
        assert!(
            reference.centre[axis].contains(0.5),
            "the node centre axis {axis} contains 0.5: {:?}",
            reference.centre[axis]
        );
    }
    assert!(
        reference.radius.contains(0.5),
        "the node radius contains 0.5: {:?}",
        reference.radius
    );
    for event in third_face.iter().skip(1) {
        let node = event.node.as_ref().expect("a ThirdFace event carries its node");
        assert_eq!(
            node, reference,
            "every incident branch references the SAME solved node (P6)"
        );
    }
}

#[test]
fn topology_between_events_is_never_speculated() {
    // When the corrector cannot certify another step the walk refuses with the
    // refusal family — it NEVER guesses a continuation past the last certified
    // step. A budget of three Newton spends lets the walk certify the seed step
    // and two continuation steps and then refuses on the third: the partial
    // walk carried by the refusal is exactly the three certified steps recorded
    // before the undecided step.
    let seed = groove_seed(0.3, 0.25, None);
    let mut budget = Budget::new(0, 3, 0);
    match trace_branch_steps(&seed, &RadiusLaw::Constant(0.25), &mut budget) {
        Err(BranchRefusal {
            refusal,
            partial,
        }) => {
            assert_eq!(refusal, ConstructRefusal::ConditioningBelowThreshold);
            assert_eq!(
                partial.steps.len(),
                3,
                "the walk stops exactly at the last certified step"
            );
            assert!(
                partial.events.is_empty(),
                "no event is fabricated before the undecided step"
            );
            let last = partial.steps.last().expect("three certified steps");
            assert!(
                last.centre.hi[0] < 0.5,
                "the last certified step is still inside the walk: {:?}",
                last.centre
            );
        }
        Ok(_) => panic!("an undecided step must refuse, never continue"),
    }
}
