//! BG-CK-P2-TRACE integration suite over the crate's public path.
//!
//! The trace LOOP itself and its solver-private certifier seam are
//! `pub(crate)` (the wave-mode HULL precedent), so the six `tests_required`
//! are driven by synthetic certifier impl blocks inside `src/ssi_trace.rs`'s
//! own test module (same-crate access to the seam), exactly as the HULL packet
//! split its required tests. This suite exercises what the wave contract makes
//! reachable through `truck_certified::ssi_fixtures` and the re-exported
//! `ssi_types` shapes — the fixture ground truths the trace walks, the
//! `TraceStep`/`TraceOutcome`/`TraceRefusal` shapes the loop emits, and the
//! source-discipline (H-1) scan of the new module.

use truck_certified::contract::{CoordinateSwitch, IntervalEnclosure, Refusal};
use truck_certified::formal::contact::GenericUnresolved;
use truck_certified::formal::span::BranchGerm;
use truck_certified::hull::HullRefusal;
use truck_certified::ssi_fixtures as fx;
use truck_certified::ssi_types::{
    KrawczykCertificate3, SquareSystem3, TraceOutcome, TraceRefusal, TraceStep,
};

/// The fixture direct-evaluation tolerance (H-3).
const EVAL_EPSILON: f64 = 1e-9;

#[test]
fn ssi_trace_module_is_registered_in_lib() {
    let lib_source = include_str!("../src/lib.rs");
    assert!(
        lib_source.contains("pub mod ssi_trace;"),
        "lib.rs carries the one-line module registration"
    );
}

#[test]
fn ssi_trace_module_carries_no_panicking_or_extraction_calls() {
    // H-1 source discipline: the new module (tests included) must not breach
    // the crate-level deny and must carry no module-level opt-out.
    let source = include_str!("../src/ssi_trace.rs");
    assert!(!source.contains("panic!"), "ssi_trace.rs has no panic call");
    assert!(
        !source.contains(".unwrap("),
        "ssi_trace.rs has no unwrap call"
    );
    assert!(
        !source.contains(".expect("),
        "ssi_trace.rs has no expect call"
    );
    assert!(
        !source.contains("#![allow"),
        "ssi_trace.rs has no module-level allow"
    );
}

#[test]
fn germ_ladder_public_path_carries_documented_classes() {
    let ladder = fx::germ_ladder().expect("germ ladder fixture builds");
    assert_eq!(ladder.len(), 5, "one fixture per BranchGerm variant");
    let classes: Vec<BranchGerm> = ladder.iter().map(|fixture| fixture.germ).collect();
    assert_eq!(
        classes,
        vec![
            BranchGerm::Regular,
            BranchGerm::StationaryRegular {
                first_nonzero_order: 2
            },
            BranchGerm::CuspCandidate,
            BranchGerm::Singular,
            BranchGerm::Unresolved,
        ],
        "the ladder carries every BranchGerm variant in order"
    );
    assert!(
        ladder
            .iter()
            .take(4)
            .all(|fixture| fixture.event_is_interior()),
        "the four interior rungs have interior events"
    );
    assert!(
        !ladder[4].event_is_interior(),
        "the unresolved rung's event sits on the chart-box boundary"
    );
}

#[test]
fn closed_loop_fixture_ground_truth_holds_on_the_branch() {
    // The branch the closed-loop trace scenario walks: the diagonal lift of the
    // circle of radius 3/10 about (1/2, 1/2). F vanishes at both seeds and at
    // every sampled point of the parametrized loop.
    let pair = fx::closed_loop_pair().expect("closed loop fixture builds");
    let system = &pair.system;
    for (index, point) in [pair.first_seed, pair.second_seed].iter().enumerate() {
        let values = fx::eval_system(system, *point).expect("seed point evaluates");
        assert!(
            values.iter().all(|value| value.abs() < EVAL_EPSILON), // H-3
            "seed {index} lies on the fixture branch"
        );
    }
    for k in 0..128 {
        let theta = 2.0 * std::f64::consts::PI * (k as f64) / 128.0;
        let u = pair.center.0 + pair.radius * theta.cos();
        let v = pair.center.1 + pair.radius * theta.sin();
        let point = (u, v, u, v);
        let values = fx::eval_system(system, point).expect("loop point evaluates");
        assert!(
            values.iter().all(|value| value.abs() < EVAL_EPSILON), // H-3
            "the sampled loop point lies on the fixture branch"
        );
    }
}

#[test]
fn trace_step_shape_round_trips_through_the_public_types() {
    let step = fx::sample_trace_step().expect("sample trace step builds");
    assert_eq!(
        step.chart_box(),
        [(0.2, 0.6), (0.3, 0.5), (0.2, 0.6), (0.3, 0.5)],
        "the box round-trips verbatim"
    );
    assert_eq!(step.germ(), BranchGerm::Regular);
    assert_eq!(step.coordinate().index, 2, "the s continuation certificate");
    let incidence = step.incidence();
    assert_eq!(
        incidence.germ,
        BranchGerm::Regular,
        "germ travels on the record"
    );
    assert_eq!(
        incidence.span_id,
        truck_certified::formal::span::SpanId::from_occurrence(&incidence.provenance),
        "the record's span id is the provenance-derived identity"
    );

    // The refusing constructor is public and named.
    let bad = TraceStep::new(
        [(0.5, 0.2), (0.3, 0.5), (0.2, 0.6), (0.3, 0.5)], // reversed first axis
        BranchGerm::Regular,
        incidence,
        step.coordinate(),
    );
    assert_eq!(bad, Err(Refusal::InvalidInput), "a misordered box refuses");
}

#[test]
fn trace_outcome_vocabulary_is_closed_and_shaped() {
    let step = fx::sample_trace_step().expect("sample trace step builds");
    let margin = IntervalEnclosure::new(0.5, 1.0).expect("margin builds");
    let outgoing = truck_certified::contract::ContinuationCoordinate {
        index: 2,
        relative_margin: margin,
    };
    let incoming = truck_certified::contract::ContinuationCoordinate {
        index: 3,
        relative_margin: margin,
    };
    let switch = CoordinateSwitch { outgoing, incoming };
    let closed = TraceOutcome::ClosedLoop { steps: vec![step] };
    let terminated = TraceOutcome::Terminated { steps: vec![step] };
    let switched = TraceOutcome::Switched {
        steps: vec![step],
        switch,
    };
    let refused = TraceOutcome::Refused(TraceRefusal::Conditioning(
        Refusal::ConditioningBelowThreshold,
    ));

    // The exhaustive no-catch-all match compiles only because the vocabulary is
    // exactly these four named cases.
    let names: Vec<&str> = [closed, terminated, switched, refused]
        .into_iter()
        .map(|outcome| match outcome {
            TraceOutcome::ClosedLoop { steps } => {
                assert!(!steps.is_empty());
                "closed_loop"
            }
            TraceOutcome::Terminated { steps } => {
                assert!(!steps.is_empty());
                "terminated"
            }
            TraceOutcome::Switched { steps, switch } => {
                assert!(!steps.is_empty());
                assert_eq!(switch.outgoing.index, outgoing.index);
                assert_eq!(switch.incoming.index, incoming.index);
                "switched"
            }
            TraceOutcome::Refused(refusal) => refusal.tag(),
        })
        .collect();
    assert_eq!(
        names,
        vec![
            "closed_loop",
            "terminated",
            "switched",
            "trace_refused_conditioning"
        ]
    );
}

#[test]
fn trace_refusal_tags_are_stable_named_cases() {
    let tag = |refusal: TraceRefusal| refusal.tag();
    assert_eq!(
        tag(TraceRefusal::Conditioning(
            Refusal::ConditioningBelowThreshold
        )),
        "trace_refused_conditioning"
    );
    assert_eq!(
        tag(TraceRefusal::Conditioning(Refusal::InvalidInput)),
        "trace_refused_invalid_input"
    );
    assert_eq!(
        tag(TraceRefusal::Conditioning(Refusal::Unfrozen)),
        "trace_refused_unfrozen"
    );
    assert_eq!(
        tag(TraceRefusal::Hull(HullRefusal::EnclosureUnavailable)),
        "trace_refused_hull_enclosure_unavailable"
    );
    assert_eq!(
        tag(TraceRefusal::Hull(HullRefusal::DomainNotCompact)),
        "trace_refused_hull_domain_not_compact"
    );
    assert_eq!(
        tag(TraceRefusal::Unresolved(GenericUnresolved::ClusteredRoots)),
        "unresolved_clustered_roots"
    );

    // No catch-all arm: every refusal family the trace can emit wraps a landed
    // named cause.
    let named = |refusal: TraceRefusal| match refusal {
        TraceRefusal::Conditioning(cause) => match cause {
            Refusal::ConditioningBelowThreshold => "conditioning",
            Refusal::InvalidInput => "invalid_input",
            Refusal::Unfrozen => "unfrozen",
        },
        TraceRefusal::Hull(cause) => match cause {
            HullRefusal::EnclosureUnavailable => "enclosure_unavailable",
            HullRefusal::DomainNotCompact => "domain_not_compact",
        },
        TraceRefusal::Unresolved(cause) => cause.tag(),
    };
    assert_eq!(
        named(TraceRefusal::Conditioning(Refusal::InvalidInput)),
        "invalid_input"
    );
    assert_eq!(
        named(TraceRefusal::Hull(HullRefusal::DomainNotCompact)),
        "domain_not_compact"
    );
    assert_eq!(
        named(TraceRefusal::Unresolved(GenericUnresolved::ClusteredRoots)),
        "unresolved_clustered_roots"
    );
}

/// Re-exported shim types resolve through the crate root (the reachability
/// fact the wave relies on for the fixture-driven suites).
#[test]
fn shim_shapes_resolve_at_the_crate_root() {
    let _: Option<KrawczykCertificate3> = None;
    let _: Option<SquareSystem3> = None;
    let _: Option<TraceStep> = None;
    let _: Option<TraceOutcome> = None;
    let _: Option<TraceRefusal> = None;
}

// ---------------------------------------------------------------------------
// Certified production seam (integration amendment): certified_pair_trace over
// the fixture pairs reconstructed as certified-admitted rational patches.
// ---------------------------------------------------------------------------

use truck_certified::ssi::{construct_square_system, RationalBipatch, SsiParticipant, SsiRefusal};
use truck_certified::ssi_trace::certified_pair_trace;

fn binom(n: usize, k: usize) -> f64 {
    let mut numerator = 1u64;
    let mut denominator = 1u64;
    for i in 0..k {
        numerator *= (n - i) as u64;
        denominator *= (i + 1) as u64;
    }
    numerator as f64 / denominator as f64
}

/// Add `coeff * u^pu * v^pv` (monomial basis) onto a Bernstein grid.
fn add_monomial_term(grid: &mut [Vec<f64>], m: usize, n: usize, pu: usize, pv: usize, coeff: f64) {
    for a in pu..=m {
        for b in pv..=n {
            let fa = binom(a, pu) / binom(m, pu);
            let fb = binom(b, pv) / binom(n, pv);
            grid[a][b] += coeff * fa * fb;
        }
    }
}

fn monomial_grid(m: usize, n: usize, terms: &[(usize, usize, f64)]) -> Vec<Vec<f64>> {
    let mut grid = vec![vec![0.0; n + 1]; m + 1];
    for &(pu, pv, coeff) in terms {
        add_monomial_term(&mut grid, m, n, pu, pv, coeff);
    }
    grid
}

/// The first-parameter (`which == 0`) or second-parameter chart coordinate grid.
fn chart_grid(m: usize, n: usize, which: usize) -> Vec<Vec<f64>> {
    let mut grid = Vec::with_capacity(m + 1);
    for a in 0..=m {
        let mut row = Vec::with_capacity(n + 1);
        for b in 0..=n {
            let value = if which == 0 {
                a as f64 / m as f64
            } else {
                b as f64 / n as f64
            };
            row.push(value);
        }
        grid.push(row);
    }
    grid
}

fn constant_grid(m: usize, n: usize, value: f64) -> Vec<Vec<f64>> {
    vec![vec![value; n + 1]; m + 1]
}

fn unit_weight(m: usize, n: usize) -> Vec<Vec<f64>> {
    constant_grid(m, n, 1.0)
}

fn rational(m: usize, n: usize, num: [Vec<Vec<f64>>; 3]) -> RationalBipatch {
    RationalBipatch::new(m, n, num, unit_weight(m, n))
        .expect("a valid unit-weight rational patch was refused")
}

/// The `well_conditioned_root()` pair: patch 1 `(u, v, v)`, patch 2
/// `(s, t, 1/4 + s/2)`.
fn well_conditioned_pair() -> (RationalBipatch, RationalBipatch) {
    let u = chart_grid(1, 1, 0);
    let v = chart_grid(1, 1, 1);
    let p1 = rational(1, 1, [u.clone(), v.clone(), v.clone()]);
    let z2 = monomial_grid(1, 1, &[(0, 0, 0.25), (1, 0, 0.5)]);
    let p2 = rational(1, 1, [u, v, z2]);
    (p1, p2)
}

/// The `closed_loop_pair()` pair: the bowl graph against the plane `z = 0`.
fn closed_loop_pair() -> (RationalBipatch, RationalBipatch) {
    let h = monomial_grid(
        2,
        2,
        &[
            (2, 0, 1.0),
            (1, 0, -1.0),
            (0, 2, 1.0),
            (0, 1, -1.0),
            (0, 0, 0.41),
        ],
    );
    let p1 = rational(2, 2, [chart_grid(2, 2, 0), chart_grid(2, 2, 1), h]);
    let p2 = rational(
        1,
        1,
        [
            chart_grid(1, 1, 0),
            chart_grid(1, 1, 1),
            constant_grid(1, 1, 0.0),
        ],
    );
    (p1, p2)
}

/// The `conditioning_below_threshold()` pair: patch 1 `(0, u, u+v)`, patch 2
/// `(s+t-1, t, 1)`.
fn conditioning_pair() -> (RationalBipatch, RationalBipatch) {
    let p1 = rational(
        1,
        1,
        [
            constant_grid(1, 1, 0.0),
            monomial_grid(1, 1, &[(1, 0, 1.0)]),
            monomial_grid(1, 1, &[(1, 0, 1.0), (0, 1, 1.0)]),
        ],
    );
    let p2 = rational(
        1,
        1,
        [
            monomial_grid(1, 1, &[(1, 0, 1.0), (0, 1, 1.0), (0, 0, -1.0)]),
            monomial_grid(1, 1, &[(0, 1, 1.0)]),
            constant_grid(1, 1, 1.0),
        ],
    );
    (p1, p2)
}

/// The reconstructed square system matches the stored fixture ground truth:
/// `F` vanishes at the documented branch points.
#[test]
fn reconstructed_pairs_reproduce_the_fixture_ground_truth() {
    let fixture = fx::well_conditioned_root().expect("fixture builds");
    let (p1, p2) = well_conditioned_pair();
    let system = construct_square_system(
        &SsiParticipant::RationalBipatch(p1),
        &SsiParticipant::RationalBipatch(p2),
    )
    .expect("spline-admissible pair constructs");
    assert_eq!(system, fixture.system, "well-conditioned system reproduced");
    let values = fx::eval_system(&system, fixture.root).expect("root evaluates");
    assert!(
        values.iter().all(|value| value.abs() < EVAL_EPSILON), // H-3
        "F(root) = 0"
    );

    let fixture = fx::closed_loop_pair().expect("fixture builds");
    let (p1, p2) = closed_loop_pair();
    let system = construct_square_system(
        &SsiParticipant::RationalBipatch(p1),
        &SsiParticipant::RationalBipatch(p2),
    )
    .expect("spline-admissible pair constructs");
    assert_eq!(system, fixture.system, "closed-loop system reproduced");
    for point in [fixture.first_seed, fixture.second_seed] {
        let values = fx::eval_system(&system, point).expect("seed evaluates");
        assert!(
            values.iter().all(|value| value.abs() < EVAL_EPSILON), // H-3
            "F(seed) = 0"
        );
    }

    let fixture = fx::conditioning_below_threshold().expect("fixture builds");
    let (p1, p2) = conditioning_pair();
    let system = construct_square_system(
        &SsiParticipant::RationalBipatch(p1),
        &SsiParticipant::RationalBipatch(p2),
    )
    .expect("spline-admissible pair constructs");
    assert_eq!(system, fixture.system, "conditioning system reproduced");
}

#[test]
fn certified_trace_well_conditioned_root_terminates_at_domain_boundary() {
    let (p1, p2) = well_conditioned_pair();
    let system = construct_square_system(
        &SsiParticipant::RationalBipatch(p1.clone()),
        &SsiParticipant::RationalBipatch(p2.clone()),
    )
    .expect("pair constructs");
    let outcome = certified_pair_trace(&p1, &p2, [0.5, 0.5, 0.5, 0.5])
        .expect("the well-conditioned seed certifies");
    let steps = match outcome {
        TraceOutcome::Terminated { steps } => steps,
        other => {
            assert!(
                false,
                "expected Terminated at the domain boundary, got {other:?}"
            );
            return;
        }
    };
    assert!(!steps.is_empty(), "a domain exit traces in-domain steps");
    for step in &steps {
        let box_ = step.chart_box();
        assert!(
            box_.iter().all(|(lo, hi)| *lo >= 0.0 && *hi <= 1.0),
            "every certified step box lies inside the unit chart"
        );
        let centre = (
            0.5 * (box_[0].0 + box_[0].1),
            0.5 * (box_[1].0 + box_[1].1),
            0.5 * (box_[2].0 + box_[2].1),
            0.5 * (box_[3].0 + box_[3].1),
        );
        let values = fx::eval_system(&system, centre).expect("centre evaluates");
        assert!(
            values.iter().all(|value| value.abs() < EVAL_EPSILON), // H-3
            "the certified step centre lies on the fixture branch"
        );
    }
    // The walk left the seed region along the branch: the final step sits past
    // the interior seed toward the chart boundary.
    let last = &steps[steps.len() - 1];
    let last_centre = last.chart_box();
    let s_last = 0.5 * (last_centre[2].0 + last_centre[2].1);
    assert!(
        s_last > 0.9,
        "the terminated branch reached the chart boundary"
    );
}

#[test]
fn certified_trace_closed_loop_closes_with_identity_recurrence() {
    let (p1, p2) = closed_loop_pair();
    let outcome = certified_pair_trace(&p1, &p2, [0.5, 0.8, 0.5, 0.8])
        .expect("the closed-loop seed certifies");
    let steps = match outcome {
        TraceOutcome::ClosedLoop { steps } => steps,
        other => {
            assert!(
                false,
                "expected ClosedLoop from the closed fixture, got {other:?}"
            );
            return;
        }
    };
    assert!(!steps.is_empty(), "a closed loop traces steps");
    let first = &steps[0];
    let closing = &steps[steps.len() - 1];
    assert_eq!(
        closing.chart_box(),
        first.chart_box(),
        "identity recurrence: closing box id equals the first box id"
    );
    assert!(
        steps.len() > 50,
        "a full revolution of the loop needs many certified steps, got {}",
        steps.len()
    );
}

#[test]
fn certified_trace_conditioning_fixture_refuses_the_named_way() {
    let (p1, p2) = conditioning_pair();
    match certified_pair_trace(&p1, &p2, [0.5, 0.5, 0.5, 0.5]) {
        Err(SsiRefusal::Conditioning(Refusal::ConditioningBelowThreshold)) => {}
        other => assert!(
            false,
            "the conditioning fixture must refuse ConditioningBelowThreshold, got {other:?}"
        ),
    }
}
