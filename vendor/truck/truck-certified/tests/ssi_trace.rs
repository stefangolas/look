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
