//! BG-KV2-503-S10 integration tests: authored-topology verification
//! (`claims.rs`, spec §15). The six `tests_required` names.
//!
//! Fixtures are shared-chart graph-arrangement leaf pairs (identity charts):
//! `S1(u,v) = (u, v, h1(u,v))` against the plane `z = 0`, so the R1 zero set is
//! the diagonal lift of the plane curve `g = h1`. The `single_line` fixture
//! (`g = 0.5 - v`) has one transversal component at `v = 0.5`. The
//! `line_and_parabola` fixture
//! (`g = (v - 0.2) * (v - (u - 1/2)^2 - 0.45)`) has the straight component at
//! `v = 0.2` plus a second, curved component (a parabola) whose vertex is an
//! isolated critical point of the Tier-2 exclusion direction.

#![deny(clippy::unwrap_used)]

use truck_certified::kernel::claims::{
    certify_claimed, claim_claimed, ClaimRefutation, ClaimedComponent, ComponentKind, LeafPair,
    TopologyClaim,
};
use truck_certified::kernel::evidence::Refusal as KernelRefusal;
use truck_certified::kernel::graph::{
    CertifiedGraph, ChartId, ClaimedGraph, Param, Point4, Provenance,
};
use truck_certified::kernel::leaf::BezierLeaf;
use truck_certified::kernel::patch::IBox;

/// The first chart id of every fixture.
const CHART_A: ChartId = ChartId(0);
/// The second chart id of every fixture.
const CHART_B: ChartId = ChartId(1);

/// Extract the `Ok` of a fallible construction; the fixture data is valid by
/// construction, so the refusal arm is a test-bug panic (never an unwrap).
fn construct<T>(result: Result<T, KernelRefusal>) -> T {
    match result {
        Ok(value) => value,
        Err(refusal) => panic!("a construction that must succeed was refused: {refusal:?}"),
    }
}

/// A canonical unit-weight graph leaf `(u, v, z(u, v))` over the identity
/// chart at bidegree `(du, dv)`. `z` is the row-major control height net over
/// `(u, v)` (index `a * (dv + 1) + b`).
fn graph_leaf(du: usize, dv: usize, z: &[f64]) -> BezierLeaf {
    let mut control = Vec::with_capacity((du + 1) * (dv + 1));
    for a in 0..=du {
        for b in 0..=dv {
            let r = a * (dv + 1) + b;
            control.push([a as f64 / du as f64, b as f64 / dv as f64, z[r], 1.0]);
        }
    }
    construct(BezierLeaf::try_new(du, dv, control))
}

/// The plane `z = 0` leaf at bidegree `(du, dv)`.
fn plane_leaf(du: usize, dv: usize) -> BezierLeaf {
    let z = vec![0.0f64; (du + 1) * (dv + 1)];
    graph_leaf(du, dv, &z)
}

/// The claim domain box: `u1` and `u2` share `[u_lo, u_hi]`, `v1` and `v2`
/// share `[v_lo, v_hi]`.
fn claim_domain(u_lo: f64, u_hi: f64, v_lo: f64, v_hi: f64) -> IBox<4> {
    construct(IBox::<4>::try_new(
        [u_lo, v_lo, u_lo, v_lo],
        [u_hi, v_hi, u_hi, v_hi],
    ))
}

/// A chart parameter on the shared chart at deck 0.
fn param(chart: ChartId, u: f64, v: f64) -> Param {
    construct(Param::try_new(chart, 0, u, v))
}

/// A diagonal seed `(u, v)` on the two charts.
fn diagonal_seed(u: f64, v: f64) -> Point4 {
    Point4 {
        p1: param(CHART_A, u, v),
        p2: param(CHART_B, u, v),
    }
}

/// The single-component fixture: `g = 0.5 - v`, one transversal branch at
/// `v = 0.5`. The Tier-2 direction is off the (constant) kernel direction, so
/// the complement exclusion prunes immediately and an exhaustive claim
/// discharges.
fn single_line_pair() -> LeafPair {
    let first = graph_leaf(1, 1, &[0.5, -0.5, 0.5, -0.5]);
    let second = plane_leaf(1, 1);
    let a = [1.0, 0.0, 0.0, 0.0];
    construct(LeafPair::try_new(
        first,
        second,
        CHART_A,
        CHART_B,
        claim_domain(0.3, 0.7, 0.0, 1.0),
        a,
    ))
}

/// One ordinary claimed component at the diagonal seed `(u, v)`.
fn ordinary_component(u: f64, v: f64) -> ClaimedComponent {
    ClaimedComponent {
        seed: diagonal_seed(u, v),
        expected: ComponentKind::Ordinary,
    }
}

/// The claim shape used by the tests.
fn make_claim(
    components: Vec<ClaimedComponent>,
    exhaustive: bool,
    provenance: Provenance,
) -> TopologyClaim {
    TopologyClaim {
        components,
        exhaustive,
        provenance,
    }
}

/// The three-valued claim verdict of the kernel.
use truck_certified::kernel::evidence::ClaimVerdict as Verdict;

// ---------------------------------------------------------------------------
// The six tests_required
// ---------------------------------------------------------------------------

#[test]
fn certify_claimed_accepts_a_true_component() {
    let pair = single_line_pair();
    let claim = make_claim(
        vec![ordinary_component(0.5, 0.5)],
        true,
        Provenance::Claimed,
    );
    let verdict = certify_claimed(&pair, &claim);
    let graph: CertifiedGraph = match verdict {
        Verdict::Proven(graph) => graph,
        other => panic!("a true single-component claim must certify: {other:?}"),
    };
    assert_eq!(
        graph.nodes.len(),
        2,
        "one certified component contributes its two boundary nodes"
    );
    assert_eq!(
        graph.arcs.len(),
        1,
        "one certified component contributes one arc"
    );
    assert!(
        graph.exhaustive,
        "an exhaustive claim that discharges targeted completeness certifies as exhaustive"
    );
}

#[test]
fn refuted_component_names_component_and_predicate() {
    let pair = single_line_pair();
    // Component 0 is true (the branch at v = 0.5); component 1 is claimed at
    // v = 0.8, provably off the intersection.
    let claim = make_claim(
        vec![ordinary_component(0.5, 0.5), ordinary_component(0.5, 0.8)],
        true,
        Provenance::Claimed,
    );
    let verdict = certify_claimed(&pair, &claim);
    let refutation: ClaimRefutation = match verdict {
        Verdict::Disproven(refutation) => refutation,
        other => panic!("a wrong component must refute, never repair: {other:?}"),
    };
    assert_eq!(
        refutation.component, 1,
        "the refutation names the wrong component's index"
    );
    assert_eq!(
        refutation.predicate, "tube-chain-via-C2",
        "an off-intersection seed refutes the tube-chain predicate"
    );
}

#[test]
fn non_exhaustive_claim_yields_claimed_graph() {
    let pair = single_line_pair();
    let claim = make_claim(
        vec![ordinary_component(0.5, 0.5)],
        false,
        Provenance::Client,
    );
    let verdict = claim_claimed(&pair, &claim);
    let claimed: ClaimedGraph = match verdict {
        Verdict::Proven(claimed) => claimed,
        other => panic!("a true non-exhaustive claim must yield a claimed graph: {other:?}"),
    };
    assert!(
        !claimed.graph.exhaustive,
        "a non-exhaustive claim's graph is not certified exhaustive"
    );
    assert_eq!(
        claimed.provenance,
        Provenance::Client,
        "the claimed graph carries the claim's provenance"
    );
}

#[test]
fn exhaustive_claim_discharges_targeted_completeness() {
    let pair = single_line_pair();
    let claim = make_claim(
        vec![ordinary_component(0.5, 0.5)],
        true,
        Provenance::Claimed,
    );
    let verdict = certify_claimed(&pair, &claim);
    let graph: CertifiedGraph = match verdict {
        Verdict::Proven(graph) => graph,
        other => panic!("an exhaustive claim over the single component must discharge: {other:?}"),
    };
    assert!(
        graph.exhaustive,
        "the discharged graph is certified exhaustive"
    );
    assert_eq!(graph.nodes.len(), 2);
    assert_eq!(graph.arcs.len(), 1);

    // A non-exhaustive claim cannot be certified through certify_claimed: the
    // CertifiedGraph result type may never carry an undischarged claim (D6).
    let nonexhaustive = make_claim(
        vec![ordinary_component(0.5, 0.5)],
        false,
        Provenance::Claimed,
    );
    match certify_claimed(&pair, &nonexhaustive) {
        Verdict::Inconclusive(refusal) => {
            assert_eq!(
                refusal.kind,
                truck_certified::kernel::evidence::RefusalKind::ClaimRefuted,
                "the non-exhaustive gate refuses with ClaimRefuted"
            );
        }
        other => panic!("a non-exhaustive claim must not be certified: {other:?}"),
    }
}

#[test]
fn provenance_does_not_discharge_completeness() {
    let pair = single_line_pair();
    // Provenance is data, never a certificate (D6): an exhaustive claim with
    // each of the three provenances discharges identically — the item-3
    // complement exclusion is what discharges, and the claim's provenance does
    // NOT skip it. A skip would leave the claim undischarged (the only
    // provenance-adjacent skip, the trusted path through claim_claimed, yields
    // a ClaimedGraph — see trusted_provenance_yields_claimed_graph_not_certified).
    for provenance in [
        Provenance::Claimed,
        Provenance::Imported,
        Provenance::Client,
    ] {
        let claim = make_claim(vec![ordinary_component(0.5, 0.5)], true, provenance);
        match certify_claimed(&pair, &claim) {
            Verdict::Proven(graph) => {
                assert!(
                    graph.exhaustive,
                    "provenance {provenance:?} never skips the complement exclusion: the \
                     discharged graph is exhaustive"
                );
            }
            other => {
                panic!(
                    "provenance {provenance:?} must not discharge completeness (only the item-3 \
                     exclusion does): {other:?}"
                )
            }
        }
    }
}

#[test]
fn trusted_provenance_yields_claimed_graph_not_certified() {
    let pair = single_line_pair();
    // The trusted-provenance opt-in: the caller routes the exhaustive claim
    // through claim_claimed, which skips item 3 and yields a ClaimedGraph —
    // the type, not a flag, is what a CertifiedGraph signature must reject.
    let claim = make_claim(
        vec![ordinary_component(0.5, 0.5)],
        true,
        Provenance::Claimed,
    );
    let verdict = claim_claimed(&pair, &claim);
    let claimed: ClaimedGraph = match verdict {
        Verdict::Proven(claimed) => claimed,
        other => panic!("the trusted path must yield a claimed graph: {other:?}"),
    };
    assert!(
        claimed.graph.exhaustive,
        "the claimed graph records the claim's exhaustive assertion"
    );
    assert_eq!(claimed.graph.nodes.len(), 2);
    assert_eq!(claimed.graph.arcs.len(), 1);

    // The verification entry on the SAME exhaustive claim produces a
    // CertifiedGraph — a type distinct from the trusted path's ClaimedGraph
    // (D6: no From<ClaimedGraph> for CertifiedGraph), so a Boolean requiring
    // closure rejects the trusted result by type, never by a runtime flag.
    match certify_claimed(&pair, &claim) {
        Verdict::Proven(graph) => {
            let _certified: CertifiedGraph = graph;
            assert!(_certified.exhaustive);
        }
        other => panic!("the verification entry must certify the same claim: {other:?}"),
    }
}
