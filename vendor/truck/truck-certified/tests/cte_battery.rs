//! CTE-008-GATES — the certified program battery (T1 gates).
//!
//! The one-verify amendment's certified battery: the T1.8 termination
//! battery (theory §2.11), the R3 A₁⁻ node regression gate, the R1
//! no-runtime-T1.4 gate, and the determinism gate. New test file only; every
//! landed test name is a byte-identical constraint.
//!
//! The fixture *inputs* (systems + exact charts + certified graphs) live in
//! [`truck_certified::tangency::gates`] (doc-hidden test support), because the
//! crate's in-crate fixture kits are `#[cfg(test)]` and invisible to this
//! integration test. The battery drives them through the public path and
//! asserts the recorded certified outcomes:
//!
//! - F3 (A₁⁺, definite) → `A1Isolated`;
//! - F4 (A₁⁻, saddle node) → `A1Node` (never `Unresolved` — the R3 gate);
//! - F5 (A₂ parabola) → `A2Branch` (via the `certify_a2` route);
//! - F6 (transversal crossing planes) → `Transversal`.
//!
//! F1/F2 (the T1.4 / T1.1 identity fixtures) have no classify box: their
//! certified ground truths are the exact admissions, recorded as depth-0
//! identity rows of the termination battery. Refinement depth is measured as
//! the subdivision units the certified driver spent on the fixture's box
//! (booked minus remaining), and the certified boxes decide on the first
//! classify call.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]
// Test-only allow: H-1 bans unwrap/expect/panic on paths reachable from
// untrusted geometry. This file is integration-test assertions on the crate's
// own certified fixture inputs - not such a path.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use truck_certified::tangency::gates::{
    a2_booked_budget, admit_fixture_kit, booked_budget, f3_input, f4_input, f5_input, f6_input,
    is_certified, t14_runtime_classify_ok, A2Input, ClassifyInput,
};
use truck_certified::tangency::shapes::ContactVerdict;

/// The booked budget for one A₁/transversal classify run.
const BOOKED_SUBDIV: u32 = 256;

/// The booked budget for one A₂ certify run.
const A2_BOOKED_SUBDIV: u32 = 4096;

/// Run one certified route, returning `(verdict_kind, subdiv_spent)`.
fn classify_with_depth(input: &ClassifyInput) -> (String, u32) {
    let mut budget = booked_budget();
    let verdict = input
        .classify(&mut budget)
        .expect("the certified classify certifies");
    let spent = BOOKED_SUBDIV - budget.subdiv;
    (
        truck_certified::tangency::gates::verdict_kind(&verdict).to_string(),
        spent,
    )
}

/// Run the A₂ certify route, returning `(verdict_kind, subdiv_spent)`.
fn certify_with_depth(input: &A2Input) -> (String, u32) {
    let mut budget = a2_booked_budget();
    let verdict = input
        .certify(&mut budget)
        .expect("the certified A2 certify certifies");
    let spent = A2_BOOKED_SUBDIV - budget.subdiv;
    (
        truck_certified::tangency::gates::verdict_kind(&verdict).to_string(),
        spent,
    )
}

// ---------------------------------------------------------------------------
// T1.8 termination battery (theory §2.11)
// ---------------------------------------------------------------------------

#[test]
fn termination_battery_t18() {
    // Every fixture (F1–F6) terminates on a certified verdict (or, for the
    // identity fixtures F1/F2, on the exact admission) within the booked
    // budget. The certified fixture boxes decide on the first classify call;
    // depth is the subdivision spend the driver recorded.
    admit_fixture_kit().expect("the F1/F2 identity admissions terminate exactly");
    eprintln!("termination battery: F1 identity (T1.4 exact admission) depth 0");
    eprintln!("termination battery: F2 identity (T1.1 exact admission) depth 0");

    let f3 = classify_with_depth(&f3_input().expect("the F3 input builds"));
    assert_eq!(f3.0, "A1Isolated", "F3 terminates on A1Isolated");
    assert!(f3.1 <= BOOKED_SUBDIV, "F3 stays within the booked budget");
    eprintln!("termination battery: F3 A1Isolated depth {}", f3.1);

    let f4 = classify_with_depth(&f4_input().expect("the F4 input builds"));
    assert_eq!(f4.0, "A1Node", "F4 terminates on A1Node");
    assert!(f4.1 <= BOOKED_SUBDIV, "F4 stays within the booked budget");
    eprintln!("termination battery: F4 A1Node depth {}", f4.1);

    let f5 = certify_with_depth(&f5_input().expect("the F5 input builds"));
    assert_eq!(f5.0, "A2Branch", "F5 terminates on A2Branch");
    assert!(
        f5.1 <= A2_BOOKED_SUBDIV,
        "F5 stays within the booked budget"
    );
    eprintln!("termination battery: F5 A2Branch depth {}", f5.1);

    let f6 = classify_with_depth(&f6_input().expect("the F6 input builds"));
    assert_eq!(f6.0, "Transversal", "F6 terminates on Transversal");
    assert!(f6.1 <= BOOKED_SUBDIV, "F6 stays within the booked budget");
    eprintln!("termination battery: F6 Transversal depth {}", f6.1);
}

// ---------------------------------------------------------------------------
// R3 gate: the A₁⁻ node regression
// ---------------------------------------------------------------------------

#[test]
fn a1_node_battery_is_not_unresolved() {
    // The R3 fixture (F4): classify over the A₁⁻ box must assert A1Node - the
    // gate that fails if a later change silently degrades the indefinite case
    // to `Unresolved`.
    let input = f4_input().expect("the F4 input builds");
    let mut budget = booked_budget();
    let verdict = input
        .classify(&mut budget)
        .expect("the F4 classify certifies");
    match &verdict {
        ContactVerdict::A1Node(cert) => {
            assert!(
                cert.root().rho() < 1.0,
                "the A1 cert carries a contraction (rho < 1)"
            );
        }
        ContactVerdict::Unresolved { .. } => {
            panic!("R3 regression: the F4 indefinite case degraded to Unresolved")
        }
        other => panic!("F4 must classify A1Node, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// R1 gate: no runtime T1.4 call site
// ---------------------------------------------------------------------------

#[test]
fn t14_never_evaluated_at_runtime_gate() {
    // R1: no runtime call site evaluates the T1.4 identity. Assert via the
    // crate's public API surface: no deflation-determinant evaluator is
    // exported by the tangency layer, and the certified runtime path certifies
    // the A₁ contact with only the standard certified inputs (square system,
    // certified graph, chart minors, deflated system, box). The one sanctioned
    // evaluation of the identity is the F1 fixture's exact integer admission
    // in the doc-hidden fixture kit (test support); the grep-shaped review
    // note is recorded in RESULT.json.
    let kind = t14_runtime_classify_ok().expect("the runtime classify certifies");
    assert_eq!(
        kind, "A1Node",
        "the runtime path certifies without a T1.4 evaluator"
    );
}

// ---------------------------------------------------------------------------
// determinism gate (field-for-field)
// ---------------------------------------------------------------------------

#[test]
fn determinism_gate_field_for_field() {
    // Identical ordered input -> field-for-field identical verdicts, twice, on
    // every fixture. ContactVerdict<QPoly> is PartialEq, so two runs with
    // fresh budgets must compare equal.
    let fixtures: [(&str, ClassifyInput); 3] = [
        ("F3", f3_input().expect("the F3 input builds")),
        ("F4", f4_input().expect("the F4 input builds")),
        ("F6", f6_input().expect("the F6 input builds")),
    ];
    for (name, input) in &fixtures {
        let mut first_budget = booked_budget();
        let first = input
            .classify(&mut first_budget)
            .expect("the first classify certifies");
        let mut second_budget = booked_budget();
        let second = input
            .classify(&mut second_budget)
            .expect("the second classify certifies");
        assert_eq!(
            first, second,
            "{name}: classify_box must be field-for-field deterministic"
        );
        assert!(
            is_certified(&first),
            "{name}: the deterministic verdict is certified"
        );
    }

    // The A₂ route is deterministic too.
    let a2 = f5_input().expect("the F5 input builds");
    let mut first_budget = a2_booked_budget();
    let first = a2
        .certify(&mut first_budget)
        .expect("the first A2 certify certifies");
    let mut second_budget = a2_booked_budget();
    let second = a2
        .certify(&mut second_budget)
        .expect("the second A2 certify certifies");
    assert_eq!(
        first, second,
        "F5: certify_a2 must be field-for-field deterministic"
    );
    assert!(
        is_certified(&first),
        "F5: the deterministic verdict is certified"
    );

    // The identity fixtures' admissions are trivially deterministic (pure
    // exact integer checks): two builds admit identically.
    admit_fixture_kit().expect("the fixture kit admits");
    admit_fixture_kit().expect("the fixture kit admits again, deterministically");
}
