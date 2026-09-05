//! CC-031-BLEND-VARRADIUS integration tests (spine S10 consumer; theory §5.3):
//! the foot-point uniqueness gate, the amended variable-radius walk closed by
//! the foot-point pair, and the certified exclusion of a guide's global branch
//! by the P5 clearance predicate. The test names are the contract.

#![deny(clippy::unwrap_used)]

use truck_base::evidence::Budget;
use truck_certified::certified_map::{
    admit_curve, admit_surface, CertifiedCurveMap, CertifiedSurfaceMap,
};
use truck_certified::construct::blend::{trace_blend_chain, BlendTrace, BranchSeed, SupportChart};
use truck_certified::construct::blend_varradius::{
    foot_point_gate, trace_blend_chain_variable, trace_branch_steps_variable, VariableBranch,
};
use truck_certified::construct::refusal::ConstructRefusal;
use truck_certified::construct::stubs::{EventKind, RadiusLaw};
use truck_certified::construct::Interval;
use truck_certified::formal::numeric::PositiveFinite;
use truck_certified::kernel::patch::IBox3;
use truck_evidence::clear::{ball_clearance, BallAdmissibility};
use truck_evidence::enclosure::{Box3, Interval as InariInterval};
use truck_geometry::prelude::{BSplineCurve, BSplineSurface, KnotVec, Point3};
use truck_geometry::specifieds::Plane;

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

/// Admit one curve fixture with a declared tau of `0.5`.
fn admitted_curve(curve: &BSplineCurve<Point3>) -> CertifiedCurveMap {
    admit_curve(curve, tau(0.5)).expect("the curve fixture admits")
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
        [centre[0] - 0.02, centre[1] - 0.02, centre[2] - 0.02],
        [centre[0] + 0.02, centre[1] + 0.02, centre[2] + 0.02],
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

/// The straight guide segment `p0 -> p1` over `[0, 1]`.
fn straight_guide(p0: [f64; 3], p1: [f64; 3]) -> BSplineCurve<Point3> {
    let knot = KnotVec::bezier_knot(1);
    let points = vec![
        Point3::new(p0[0], p0[1], p0[2]),
        Point3::new(p1[0], p1[1], p1[2]),
    ];
    BSplineCurve::new(knot, points)
}

/// The retracing guide over `[0, 2]`: the segment `(0..1)` out along the
/// branch line and the segment `(1..2)` back along the SAME line. Every point
/// of the constant-radius branch line is therefore met twice — the far
/// (return) trace is a GLOBAL branch of the foot-point equations that passes
/// near every centre of the local branch.
fn retrace_guide(radius: f64) -> BSplineCurve<Point3> {
    let knot = KnotVec::from(vec![0.0, 0.0, 1.0, 2.0, 2.0]);
    let points = vec![
        Point3::new(0.0, radius, radius),
        Point3::new(1.0, radius, radius),
        Point3::new(0.0, radius, radius),
    ];
    BSplineCurve::new(knot, points)
}

/// The constant-radius groove between the floor and the `y = 0` wall, seeded at
/// arc position `x0` with the given radius.
fn groove_seed(x0: f64, radius: f64) -> BranchSeed {
    let first = chart(face_a(), 1.0);
    let second = chart(face_b(), 1.0);
    BranchSeed::try_new(first, second, box_of([x0, radius, radius]), None, None)
        .expect("the groove seed builds")
}

#[test]
fn foot_point_uniqueness_gate_refuses_when_curvature_product_at_one() {
    // On the parabola guide `G(t) = (t, t², 0)` the curvature is `κ(0) = 2`. A
    // ball centre enclosure sitting `d = 0.6` from the curve at `t = 0` has a
    // curvature product `d·κ >= 1.2 > 1`, so the λ-derivative of the foot
    // residual cannot certify strictly negative over the region and the gate
    // refuses ConditioningBelowThreshold. A centre closer to the curve
    // (`d = 0.05`, product 0.1) leaves the derivative strictly negative and the
    // gate certifies the local foot-point uniqueness.
    let curve = BSplineCurve::new(
        KnotVec::from(vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0]),
        vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.5, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
        ],
    );
    let map = admitted_curve(&curve);
    let region = (0.0, 0.5);

    let near_box = [
        Interval {
            lo: -1e-3,
            hi: 1e-3,
        },
        Interval {
            lo: 0.599,
            hi: 0.601,
        },
        Interval {
            lo: -1e-3,
            hi: 1e-3,
        },
    ];
    let refusal = foot_point_gate(&map, &near_box, region);
    assert!(
        matches!(refusal, Err(ConstructRefusal::ConditioningBelowThreshold)),
        "a curvature product above one refuses the foot-point gate: {refusal:?}"
    );

    let far_box = [
        Interval {
            lo: -1e-3,
            hi: 1e-3,
        },
        Interval {
            lo: 0.049,
            hi: 0.051,
        },
        Interval {
            lo: -1e-3,
            hi: 1e-3,
        },
    ];
    let certified = foot_point_gate(&map, &far_box, region).expect("the local foot is unique");
    assert!(
        certified.hi < 0.0,
        "the certified λ-derivative is strictly negative: {certified:?}"
    );
}

#[test]
fn variable_law_chain_matches_constant_law_at_constant_radius() {
    // A constant radius law makes the foot-point pair degenerate: the amended
    // walk reduces EXACTLY to the CC-030 system, so the variable-radius chain
    // on the constant-radius groove (guide supplied or not) produces event
    // records IDENTICAL to the CC-030 walk — the dimensionality claim is made
    // observable. The two-plane chain walks to the certified trims at arc
    // `1 - 0.3 = 0.7` and `0 - 0.3 = -0.3`.
    let seed = groove_seed(0.3, 0.25);
    let guide = admitted_curve(&straight_guide([0.0, 0.25, 0.25], [1.0, 0.25, 0.25]));
    let law = RadiusLaw::Constant(0.25);

    let mut constant_budget = budget();
    let constant = expect_trace(trace_blend_chain(
        &[seed.clone()],
        &law,
        &mut constant_budget,
    ));

    let mut variable_budget = budget();
    let variable = expect_trace(trace_blend_chain_variable(
        &[seed],
        &guide,
        &law,
        &mut variable_budget,
    ));

    assert_eq!(
        variable, constant,
        "the constant-law amended walk reduces exactly to the CC-030 walk"
    );
    assert_eq!(constant.events.len(), 2, "exactly the two terminal events");
    let high = &constant.events[0];
    let low = &constant.events[1];
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
}

#[test]
fn linear_law_radius_follows_the_declared_law() {
    // A linear radius law `R(u) = 0.1 + 0.3·u` over the guide `G(λ) = (λ, R(λ),
    // R(λ))` puts every branch centre ON its guide, so the certified radius of
    // every accepted step must match the declared law value at the step's
    // certified guide foot. The guide domain `[0, 1]` is the branch arc, so
    // each certified foot maps to the law's unit coordinate 1:1.
    let r0 = 0.1_f64;
    let r1 = 0.4_f64;
    let law = RadiusLaw::Linear { r0, r1 };
    let guide = admitted_curve(&straight_guide([0.0, r0, r0], [1.0, r1, r1]));
    let x0 = 0.5_f64;
    let radius0 = r0 + (r1 - r0) * x0;
    let seed = groove_seed(x0, radius0);

    let mut branch_budget = budget();
    let branch = match trace_branch_steps_variable(&seed, &guide, &law, &mut branch_budget, None) {
        Ok(branch) => branch,
        Err(refusal) => panic!("the linear-law branch must walk: {refusal:?}"),
    };
    assert!(
        branch.steps.len() >= 5,
        "the walk certifies interior steps: {}",
        branch.steps.len()
    );
    let trims: Vec<_> = branch
        .events
        .iter()
        .filter(|event| event.kind == EventKind::Trim)
        .collect();
    assert_eq!(trims.len(), 2, "both directions terminate at a trim event");

    for step in &branch.steps {
        let foot_mid = (step.foot.lo + step.foot.hi) * 0.5;
        assert!(
            step.foot.lo >= 0.0 && step.foot.hi <= 1.0,
            "the certified foot stays inside the guide domain: {:?}",
            step.foot
        );
        let declared = r0 + (r1 - r0) * foot_mid;
        assert!(
            step.radius.lo <= declared && declared <= step.radius.hi, // H-3: the declared linear-law value at the certified foot
            "the certified radius follows the declared law at foot {foot_mid}: radius {:?} does not contain {declared}",
            step.radius
        );
        assert!(
            step.state.radius_admissible && step.state.feet_interior && step.state.clearance_clear,
            "every accepted step keeps the isolation state"
        );
    }

    // The chain-level entry (BlendTrace consumption) walks the same branch and
    // terminates at the two certified trims at arc `1 - 0.5 = 0.5` and
    // `0 - 0.5 = -0.5`.
    let mut chain_budget = budget();
    let trace = expect_trace(trace_blend_chain_variable(
        &[seed],
        &guide,
        &law,
        &mut chain_budget,
    ));
    assert_eq!(trace.events.len(), 2);
    let trims: Vec<_> = trace
        .events
        .iter()
        .filter(|event| event.kind == EventKind::Trim)
        .collect();
    assert_eq!(trims.len(), 2);
    let high = &trace.events[0];
    let low = &trace.events[1];
    assert!(
        high.at.contains(0.5),
        "the +x trim sits at the branch end: {:?}",
        high.at
    );
    assert!(
        low.at.contains(-0.5),
        "the -x trim sits at the branch start: {:?}",
        low.at
    );
}

#[test]
fn guide_global_branch_excluded_by_clearance() {
    // The retracing guide meets every constant-radius branch centre TWICE: the
    // local out-trace at `λ = x` and the distant return trace at `λ = 2 - x`
    // that passes near the same centre — a GLOBAL branch of the foot-point
    // equations. The seed centre sits exactly on both; the local foot-point
    // gate certifies the LOCAL branch, and the P5 clearance predicate
    // (`ball_clearance` through the C2 manifest edge), which the walk runs on
    // every accepted step regardless of the guide, certifies the ball clear of
    // the excluded wall at `x = 0.7` and stops the walk at the certified
    // collision tangency `x = 0.7 - 0.25 = 0.45` — the global continuation of
    // the rolling ball beyond the wall is excluded.
    let radius = 0.25_f64;
    let x0 = 0.15_f64;
    let guide = admitted_curve(&retrace_guide(radius));
    let seed = groove_seed(x0, radius);
    let law = RadiusLaw::Constant(radius);
    let seed_iv = [
        Interval::point(x0),
        Interval::point(radius),
        Interval::point(radius),
    ];

    // The local out-trace foot is locally unique...
    let local_gate = foot_point_gate(&guide, &seed_iv, (0.0, 0.5))
        .expect("the local out-trace foot certifies unique");
    assert!(local_gate.hi < 0.0);
    // ... and the distant return-trace foot near the same centre is locally
    // unique too: the seed centre has a global second foot on the guide.
    let far_gate = foot_point_gate(&guide, &seed_iv, (1.6, 2.0))
        .expect("the distant return-trace foot certifies unique");
    assert!(far_gate.hi < 0.0);

    // The declared clearance boundary: the ball must stay on the `x <= 0.7`
    // side of the wall.
    let boundary = truck_certified::construct::blend::ClearanceBoundary {
        origin: [0.7, 0.0, 0.0],
        normal: [1.0, 0.0, 0.0],
        mode: BallAdmissibility::Round,
    };
    let mut budget = budget();
    let branch: VariableBranch =
        match trace_branch_steps_variable(&seed, &guide, &law, &mut budget, Some(boundary)) {
            Ok(branch) => branch,
            Err(refusal) => panic!("the clearance walk must complete: {refusal:?}"),
        };

    // P5 ran on every accepted step: all are clear of the wall and stop at the
    // certified collision tangency before the wall.
    let collisions: Vec<_> = branch
        .events
        .iter()
        .filter(|event| event.kind == EventKind::Collision)
        .collect();
    let trims: Vec<_> = branch
        .events
        .iter()
        .filter(|event| event.kind == EventKind::Trim)
        .collect();
    assert_eq!(collisions.len(), 1, "exactly one certified collision event");
    assert_eq!(trims.len(), 1, "the other direction still trims");
    assert!(
        collisions[0].at.contains(0.3),
        "the collision tangency sits at arc 0.7 - 0.25 - 0.15 = 0.3: {:?}",
        collisions[0].at
    );
    assert!(
        trims[0].at.contains(-0.15),
        "the trim sits at arc 0 - 0.15 = -0.15: {:?}",
        trims[0].at
    );
    assert!(
        branch.steps.iter().all(|step| step.state.clearance_clear),
        "P5 runs on every accepted step regardless of the guide"
    );
    for step in &branch.steps {
        assert!(
            step.arc.hi < 0.3,
            "no accepted step crosses the certified collision tangency"
        );
    }

    // Direct P5 evidence on the manifest edge: a ball near the wall is
    // certified clear, and a ball whose centre reaches the tangency `0.45` is
    // certified NOT clear (the Rejected side). This is the predicate whose
    // flip terminates the walk at the collision event.
    let plane = Plane::new(
        Point3::new(0.7, 0.0, 0.0),
        Point3::new(0.7, -1.0, 0.0),
        Point3::new(0.7, 0.0, -1.0),
    );
    let exclusion = Box3 {
        x: inari(0.7, f64::INFINITY),
        y: inari(f64::NEG_INFINITY, f64::INFINITY),
        z: inari(f64::NEG_INFINITY, f64::INFINITY),
    };
    let r = inari(radius, radius);
    let mu = 1e-9;
    let clear_centre = Box3 {
        x: inari(0.199, 0.201),
        y: inari(radius - 0.001, radius + 0.001),
        z: inari(radius - 0.001, radius + 0.001),
    };
    match ball_clearance(
        &plane,
        &clear_centre,
        &exclusion,
        r,
        mu,
        BallAdmissibility::Round,
    ) {
        Ok(true) => {}
        Ok(false) => panic!("an interior ball must certify clear of the wall"),
        Err(_) => panic!("an interior ball must certify clear of the wall"),
    }
    let tangency_centre = Box3 {
        x: inari(0.449, 0.451),
        y: inari(radius - 0.001, radius + 0.001),
        z: inari(radius - 0.001, radius + 0.001),
    };
    match ball_clearance(
        &plane,
        &tangency_centre,
        &exclusion,
        r,
        mu,
        BallAdmissibility::Round,
    ) {
        Ok(true) => panic!("a ball reaching the wall tangency is NOT clear"),
        Ok(false) => {}
        Err(_) => {}
    }
}

/// A certified `truck-evidence` interval for the direct P5 evidence.
fn inari(lo: f64, hi: f64) -> InariInterval {
    InariInterval::try_from((lo, hi)).expect("the inari interval is valid")
}
