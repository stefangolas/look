//! BG-KV2-303-S9A integration tests: segment gluing, deck identification, and
//! graph assembly (spec §14.1–§14.2, §16). No solver is invoked — every
//! assertion is Rules A/B/C box containment, exact integer deck sums, the typed
//! implication relation, or the stored Hermite C1 enclosure comparison.

#![deny(clippy::unwrap_used)]

use truck_certified::kernel::assemble::{
    assemble, c1_bound_of, deck_identify, glue, regions_identify, ChainArc, ChainEnd, GlueCert,
    GlueSide, HermiteEnd,
};
use truck_certified::kernel::certs::{ArcCert, PointCert, TubeOverlapCert};
use truck_certified::kernel::config;
use truck_certified::kernel::evidence::{Refusal as KernelRefusal, RefusalKind, VerdictClass};
use truck_certified::kernel::fixtures as fx;
use truck_certified::kernel::graph::{
    AnyArc, Approx, Arc, ArcEnd, ArcId, Break, BreakId, ChartId, HermiteSegment, HermiteSpline,
    Node, NodeCert, NodeId, Param, Point4, SegmentBreak, TopoNode,
};
use truck_certified::kernel::identity::{IdentityRule, IdentityVerdict};
use truck_certified::kernel::patch::IBox;
use truck_certified::kernel::residual::ResidualId;
use truck_certified::kernel::Interval;

/// The contraction rate carried by every fixture certificate (<= RHO_MAX).
const RHO: f64 = 0.125;

/// The near-miss gap of the sliver fixtures: well inside EPS_REP, far closer
/// than any certified identity needs.
const NEAR_GAP: f64 = 1e-10; // H-3: sliver-fixture separation, far below EPS_REP

/// Extract the `Ok` of a fallible construction; the fixture data is valid by
/// construction, so the refusal arm is a test-bug panic (never an unwrap).
fn construct<T>(result: Result<T, KernelRefusal>) -> T {
    match result {
        Ok(value) => value,
        Err(refusal) => panic!("a construction that must succeed was refused: {refusal:?}"),
    }
}

/// Extract the refusal kind of a construction that must refuse.
fn refusal_kind<T>(result: Result<T, KernelRefusal>) -> RefusalKind {
    match result {
        Ok(_) => panic!("expected a refusal, got an accepted construction"),
        Err(refusal) => refusal.kind,
    }
}

/// Extract the backing class of a construction that must refuse.
fn refusal_backing<T>(result: Result<T, KernelRefusal>) -> VerdictClass {
    match result {
        Ok(_) => panic!("expected a refusal, got an accepted construction"),
        Err(refusal) => refusal.backing,
    }
}

/// A parameter box `[u_lo, u_hi] x [v_lo, v_hi]`.
fn box2(u_lo: f64, u_hi: f64, v_lo: f64, v_hi: f64) -> truck_certified::kernel::patch::IBox2 {
    construct(truck_certified::kernel::patch::IBox2::try_new(
        [u_lo, v_lo],
        [u_hi, v_hi],
    ))
}

/// A certificate at the given residual over a uniform-square box `[lo, hi]^2`.
fn cert(residual: ResidualId, lo: f64, hi: f64) -> PointCert {
    construct(PointCert::try_new(residual, box2(lo, hi, lo, hi), RHO))
}

/// A certificate at the given residual over an explicit `(u, v)` box.
fn cert_box(residual: ResidualId, u_lo: f64, u_hi: f64, v_lo: f64, v_hi: f64) -> PointCert {
    construct(PointCert::try_new(
        residual,
        box2(u_lo, u_hi, v_lo, v_hi),
        RHO,
    ))
}

/// A chart parameter.
fn param(chart: u32, deck: i32, u: f64, v: f64) -> Param {
    construct(Param::try_new(ChartId(chart), deck, u, v))
}

/// A stored Hermite end at a model point with the given tangent.
fn hermite(point: [f64; 3], tangent: [f64; 3]) -> HermiteEnd {
    HermiteEnd { point, tangent }
}

// ---------------------------------------------------------------------------
// §14.2 gluing
// ---------------------------------------------------------------------------

#[test]
fn gluing_requires_tube_overlap_and_c1_agreement() {
    let a_region = cert(ResidualId::R1, 0.4, 0.6);
    let b_region = cert(ResidualId::R1, 0.5, 0.7);
    let union_cert = cert(ResidualId::R1, 0.35, 0.75);
    let unions = vec![(ResidualId::R1, union_cert)];

    let a = GlueSide {
        region: a_region,
        end: hermite([1.0, 2.0, 3.0], [1.0, 0.0, 0.0]),
    };
    let b = GlueSide {
        region: b_region,
        end: hermite([1.0, 2.0, 3.0], [1.0, 0.0, 0.0]),
    };

    let result = glue(&a, &b, &unions);
    let glue_cert: GlueCert = construct(result);
    assert_eq!(
        glue_cert.rule,
        IdentityRule::RuleA,
        "equal residuals with a containing union hull certify by Rule A"
    );
    assert_eq!(
        glue_cert.shared_point,
        [1.0, 2.0, 3.0],
        "the shared point is the agreed model point"
    );
    assert!(
        glue_cert.overlap.c1_bound <= config::EPS_REP,
        "the certified C1 bound must not exceed EPS_REP"
    );

    // The C1 agreement is required even when the tube overlap (identity) holds:
    // a reversed stored tangent is a certified C1 disagreement, never snapped.
    let b_but_not_c1 = GlueSide {
        region: b_region,
        end: hermite([1.0, 2.0, 3.0], [-1.0, 0.0, 0.0]),
    };
    assert_eq!(
        refusal_kind(glue(&a, &b_but_not_c1, &unions)),
        RefusalKind::ClaimRefuted,
        "gluing requires the stored Hermite ends to agree to C1 within EPS_REP"
    );

    // The certified bound reflects the disagreement: 2.0 on the tangent axis.
    let bound = construct(c1_bound_of(&a.end, &b_but_not_c1.end));
    assert!(
        bound > config::EPS_REP,
        "a tangent reversal must produce a C1 bound above EPS_REP"
    );
}

#[test]
fn gluing_refuses_sliver_instead_of_snapping() {
    // Two endpoint regions whose model points are within EPS_REP of each other
    // but whose residuals cannot identify under any §4.2 rule (R1 and R9 have
    // no implication). Proximity is not identity: gluing refuses.
    let a_region = cert(ResidualId::R1, 0.5, 0.5);
    let b_region = cert_box(ResidualId::R9, 0.5, 0.5, 0.5, 0.5);
    let a = GlueSide {
        region: a_region,
        end: hermite([0.0, 0.0, 0.0], [1.0, 0.0, 0.0]),
    };
    let b = GlueSide {
        region: b_region,
        end: hermite([NEAR_GAP, 0.0, 0.0], [1.0, 0.0, 0.0]),
    };
    assert!(
        (a.end.point[0] - b.end.point[0]).abs() < config::EPS_REP,
        "the fixture endpoints are nearer than the representation gap"
    );
    let result = glue(&a, &b, &[]);
    assert_eq!(
        refusal_kind(result),
        RefusalKind::SliverOrNearOverlap,
        "a near pair that does not identify is refused, never snapped"
    );
    assert_eq!(
        refusal_backing(glue(&a, &b, &[])),
        VerdictClass::Inconclusive,
        "SliverOrNearOverlap backs Inconclusive (§17)"
    );
}

// ---------------------------------------------------------------------------
// §4.2 node identity (assembly uses Rules A/B/C, never proximity)
// ---------------------------------------------------------------------------

#[test]
fn node_identity_uses_rules_abc_not_proximity() {
    // Near-miss pair: centers ~1e-10 apart, residuals with no implication. // H-3: separation far below EPS_REP, not a length threshold
    let near_a = cert(ResidualId::R1, 0.5, 0.5);
    let near_b = cert_box(ResidualId::R9, 0.5 + NEAR_GAP, 0.5 + NEAR_GAP, 0.5, 0.5);
    let broad = cert(ResidualId::R1, 0.3, 0.7);
    assert_eq!(
        regions_identify(&near_a, &near_b, &[(ResidualId::R1, broad)]),
        IdentityVerdict::NotCertified,
        "a near-miss pair with non-implying residuals must NOT identify"
    );

    // A far-apart pair on the SAME residual identifies through a containing
    // union hull — identity is containment, not distance.
    let far_a = cert(ResidualId::R1, 0.4, 0.6);
    let far_b = cert(ResidualId::R1, 10.0, 10.2);
    let spanning = cert(ResidualId::R1, 0.3, 11.0);
    assert!(
        matches!(
            regions_identify(&far_a, &far_b, &[(ResidualId::R1, spanning)]),
            IdentityVerdict::CertifiedEqual {
                rule: IdentityRule::RuleA
            }
        ),
        "same-residual containment certifies regardless of separation"
    );

    // The assembly-facing consequence: a near pair that does not identify
    // refuses gluing (SliverOrNearOverlap) instead of being welded.
    let a = GlueSide {
        region: near_a,
        end: hermite([0.0, 0.0, 0.0], [1.0, 0.0, 0.0]),
    };
    let b = GlueSide {
        region: near_b,
        end: hermite([NEAR_GAP, 0.0, 0.0], [1.0, 0.0, 0.0]),
    };
    assert_eq!(
        refusal_kind(glue(
            &a,
            &b,
            &[(ResidualId::R1, cert(ResidualId::R1, 0.3, 0.7))]
        )),
        RefusalKind::SliverOrNearOverlap,
        "assembly must never unify two nodes on proximity"
    );
}

#[test]
fn morse_saddle_identifies_against_its_half_arc_endpoints() {
    // The Rule-C fixture (spec section 20): the Morse saddle is certified as an
    // R2-stamped point; the half-arc endpoints approaching it are certified on
    // R1. The saddle identifies against its half-arc endpoints through the
    // R2 -> R1 implication with an R1 union certificate that contains the hull.
    let half_arc_endpoint = cert(ResidualId::R1, 0.5, 0.7);
    let saddle = cert(ResidualId::R2, 0.4, 0.6);
    let r1_union = cert(ResidualId::R1, 0.35, 0.75);
    let unions = vec![(ResidualId::R1, r1_union)];

    let verdict = regions_identify(&saddle, &half_arc_endpoint, &unions);
    assert_eq!(
        verdict,
        IdentityVerdict::CertifiedEqual {
            rule: IdentityRule::RuleC
        },
        "R2 certifies R1 through the typed implication, so the saddle identifies \
         against its half-arc endpoints"
    );

    // The same identification glues the saddle side to a half-arc side whose
    // stored Hermite end agrees (the saddle is on the half arc's continuation).
    let saddle_side = GlueSide {
        region: saddle,
        end: hermite([0.0, 0.0, 0.0], [1.0, 0.0, 0.0]),
    };
    let half_side = GlueSide {
        region: half_arc_endpoint,
        end: hermite([0.0, 0.0, 0.0], [1.0, 0.0, 0.0]),
    };
    let glue_cert = construct(glue(&saddle_side, &half_side, &unions));
    assert_eq!(
        glue_cert.rule,
        IdentityRule::RuleC,
        "the glue of a half-arc end to the saddle point certifies through Rule C"
    );
}

// ---------------------------------------------------------------------------
// §14.2 deck identification
// ---------------------------------------------------------------------------

#[test]
fn deck_identification_closes_a_full_cylinder_wrap() {
    let fx = construct(fx::deck_wrap());
    let period = fx.period;

    // Arc A is the deck-wrap fixture segment: deck 0 (u 5.9) crosses the seam
    // into deck 1 (canonical u 6.4 - P). Arc B is the second arc at deck + 1
    // that returns to the loop start physical point on the deck-1 copy.
    let start_a = ChainEnd {
        at: fx.start,
        point: [5.9, 0.0, 0.0],
        region: cert_box(ResidualId::R1, 5.9, 5.9, 0.0, 0.0),
    };
    let seam_end = ChainEnd {
        at: fx.end,
        point: [6.4, 0.0, 0.0],
        region: cert_box(ResidualId::R1, 6.4, 6.4, 0.0, 0.0),
    };
    let arc_a = ChainArc {
        id: ArcId(0),
        start: start_a,
        end: seam_end,
    };

    let loop_end_raw = 5.9 + period;
    let start_b = ChainEnd {
        at: fx.end,
        point: [6.4, 0.0, 0.0],
        region: cert_box(ResidualId::R1, 6.4, 6.4, 0.0, 0.0),
    };
    let end_b = ChainEnd {
        at: param(0, 1, 5.9, 0.0),
        point: [5.9, 0.0, 0.0],
        region: cert_box(ResidualId::R1, loop_end_raw, loop_end_raw, 0.0, 0.0),
    };
    let arc_b = ChainArc {
        id: ArcId(1),
        start: start_b,
        end: end_b,
    };

    // The Rule B union: the transport of the loop-start region by one period
    // lands exactly on the loop-end region.
    let union = cert_box(ResidualId::R1, loop_end_raw, loop_end_raw, 0.0, 0.0);
    let unions = vec![(ResidualId::R1, union)];

    let breaks = construct(deck_identify(&[arc_a, arc_b], period, &unions));

    // The closed loop carries winding +1: exactly one DeckStep break advancing
    // from deck 0 to deck 1.
    assert_eq!(
        breaks.len(),
        1,
        "one full cylinder wrap emits one deck step"
    );
    let deck_break = breaks[0];
    assert_eq!(deck_break.kind, SegmentBreak::DeckStep);
    assert_eq!(deck_break.at.p1.chart, ChartId(0));
    assert_eq!(deck_break.at.p2.chart, ChartId(0));
    assert_eq!(deck_break.at.p1.deck, 0, "exit side is deck 0 at u = P");
    assert_eq!(deck_break.at.p2.deck, 1, "entry side is deck 1 at u = 0");
    assert_eq!(
        deck_break.at.p2.deck - deck_break.at.p1.deck,
        1,
        "the single deck step records winding +1"
    );
    assert!(
        deck_break.overlap.c1_bound <= config::EPS_REP,
        "the deck seam of a single arc is C1 (c1_bound 0)"
    );
}

#[test]
fn helix_exceeding_deck_max_refuses_deck_exhausted() {
    let fx = construct(fx::deck_wrap());
    let period = fx.period;
    // A helix whose single edge walks nine deck crossings, above DECK_MAX (8).
    let deep = config::DECK_MAX + 1;
    let start = ChainEnd {
        at: param(0, 0, 0.5, 0.0),
        point: [0.5, 0.0, 0.0],
        region: cert_box(ResidualId::R1, 0.5, 0.5, 0.0, 0.0),
    };
    let end = ChainEnd {
        at: param(0, deep, 0.5, 0.0),
        point: [0.5, 0.0, 0.0],
        region: cert_box(
            ResidualId::R1,
            0.5 + period * deep as f64,
            0.5 + period * deep as f64,
            0.0,
            0.0,
        ),
    };
    let helix = ChainArc {
        id: ArcId(0),
        start,
        end,
    };
    let result = deck_identify(&[helix], period, &[]);
    assert_eq!(
        refusal_kind(result),
        RefusalKind::DeckExhausted,
        "a chain whose winding exceeds DECK_MAX refuses DeckExhausted"
    );
    assert_eq!(
        refusal_backing(deck_identify(
            &[ChainArc {
                id: ArcId(0),
                start,
                end,
            }],
            period,
            &[]
        )),
        VerdictClass::Inconclusive,
        "DeckExhausted backs Inconclusive (§17)"
    );
}

// ---------------------------------------------------------------------------
// §16 assembly
// ---------------------------------------------------------------------------

/// A certified ordinary arc (`Arc<4>`) whose ends are the given references.
fn ordinary_arc(id: usize, first: ArcEnd, second: ArcEnd) -> Arc<4> {
    let z_hat = [0.0, 1.0, 0.0, 0.0];
    let q = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let q_tau = [1.0, 0.0, 0.0, 0.0];
    let q_perp = [
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
        [1.0, 0.0, 0.0, 0.0],
    ];
    let a = [[0.0; 4]; 4];
    let frame = construct(truck_certified::kernel::certs::Frame::try_new(
        z_hat, q, q_tau, q_perp, a,
    ));
    let i_tau = Interval { lo: 0.0, hi: 1.0 };
    let b_perp = construct(IBox::<4>::try_new([-1.0; 4], [1.0; 4]));
    let arc_cert = construct(ArcCert::try_new(
        ResidualId::R1,
        frame,
        i_tau,
        b_perp,
        RHO,
        vec![[0.0, 0.0]; 4],
        None,
    ));
    let spline = construct(HermiteSpline::try_new(vec![HermiteSegment {
        p0: [0.0, 0.0, 0.0],
        p1: [1.0, 0.0, 0.0],
        t0: [1.0, 0.0, 0.0],
        t1: [1.0, 0.0, 0.0],
    }]));
    Arc {
        id: ArcId(id),
        approx: Approx { gamma: spline },
        cert: arc_cert,
        ends: (first, second),
    }
}

/// A certified Morse-saddle node.
fn morse_saddle_node() -> Node {
    Node {
        id: NodeId(0),
        at: Point4 {
            p1: param(0, 0, 0.5, 0.1),
            p2: param(1, 0, 0.2, 0.3),
        },
        kind: TopoNode::MorseSaddle,
        cert: NodeCert::Exact(cert(ResidualId::R1, 0.4, 0.6)),
    }
}

/// A certified deck-step break.
fn deck_step_break() -> Break {
    Break {
        id: BreakId(0),
        at: Point4 {
            p1: param(0, 0, 0.0, 0.0),
            p2: param(0, 1, 0.0, 0.0),
        },
        kind: SegmentBreak::DeckStep,
        overlap: construct(TubeOverlapCert::try_new([0.0, 0.0, 0.0], 0.0)),
    }
}

/// The exhaustive §16 shape pin: every [`TopoNode`] variant is a certified node
/// kind — no `Refuse` variant exists in the shim's enum.
fn certify_node_kind(kind: TopoNode) -> bool {
    match kind {
        TopoNode::Boundary
        | TopoNode::TrimCrossing
        | TopoNode::MorseSaddle
        | TopoNode::MorseExtremum
        | TopoNode::A2Cusp
        | TopoNode::OverlapBoundary
        | TopoNode::FilletEnd => true,
    }
}

/// The exhaustive §16 shape pin: every [`SegmentBreak`] variant is a certified
/// break kind — no `Refuse` variant exists in the shim's enum.
fn certify_break_kind(kind: SegmentBreak) -> bool {
    match kind {
        SegmentBreak::ChartSwitch
        | SegmentBreak::FrameSwitch
        | SegmentBreak::LeafBoundary
        | SegmentBreak::DeckStep
        | SegmentBreak::R6ChartSwitch
        | SegmentBreak::R6BaseSwap => true,
    }
}

/// The exhaustive [`NodeCert`] match: a node certificate is Exact or
/// AtTolerance — never a refusal.
fn node_cert_is_certified(cert: NodeCert) -> bool {
    match cert {
        NodeCert::Exact(_) => true,
        NodeCert::AtTolerance(_) => true,
    }
}

#[test]
fn assembled_graph_has_no_refuse_nodes() {
    let node = morse_saddle_node();
    let deck_break = deck_step_break();
    let arc_one = ordinary_arc(0, ArcEnd::Topo(NodeId(0)), ArcEnd::Seg(BreakId(0)));
    let arc_two = ordinary_arc(1, ArcEnd::Seg(BreakId(0)), ArcEnd::Topo(NodeId(0)));
    let graph = construct(assemble(
        vec![AnyArc::Ordinary(arc_one), AnyArc::Ordinary(arc_two)],
        vec![deck_break],
        vec![node],
    ));

    assert_eq!(graph.nodes.len(), 1);
    assert_eq!(graph.breaks.len(), 1);
    assert_eq!(graph.arcs.len(), 2);
    assert!(graph.sheets.is_empty());

    // Every assembled node is a certified node: Exact or AtTolerance, and no
    // TopoNode variant is a refusal (the exhaustive matches pin the enum).
    for assembled_node in &graph.nodes {
        assert!(
            node_cert_is_certified(assembled_node.cert),
            "no refuse node cert"
        );
        assert!(
            certify_node_kind(assembled_node.kind),
            "no refuse node kind"
        );
    }
    for assembled_break in &graph.breaks {
        assert!(
            certify_break_kind(assembled_break.kind),
            "no refuse break kind"
        );
    }

    // A reference that does not resolve refuses assembly: every ArcEnd::Topo
    // must resolve to a node in the set.
    let dangling = AnyArc::Ordinary(ordinary_arc(
        2,
        ArcEnd::Topo(NodeId(77)),
        ArcEnd::Topo(NodeId(0)),
    ));
    assert_eq!(
        refusal_kind(assemble(vec![dangling], vec![], vec![morse_saddle_node()])),
        RefusalKind::ClaimRefuted,
        "an arc end that resolves to no node refuses the assembly"
    );

    let dangling_break = AnyArc::Ordinary(ordinary_arc(
        2,
        ArcEnd::Topo(NodeId(0)),
        ArcEnd::Seg(BreakId(77)),
    ));
    assert_eq!(
        refusal_kind(assemble(
            vec![dangling_break],
            vec![],
            vec![morse_saddle_node()]
        )),
        RefusalKind::ClaimRefuted,
        "an arc end that resolves to no break refuses the assembly"
    );
}

// ---------------------------------------------------------------------------
// Source discipline
// ---------------------------------------------------------------------------

#[test]
fn no_transcendental_call_in_assemble_module() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/kernel/assemble.rs");
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(err) => panic!("assemble.rs must be readable: {err}"),
    };
    let code: Vec<&str> = source
        .lines()
        .map(|line| match line.find("//") {
            Some(i) => &line[..i],
            None => line,
        })
        .collect();
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let contains_word = |hay: &str, word: &str| {
        hay.match_indices(word).any(|(i, _)| {
            let before = i
                .checked_sub(1)
                .map(|j| hay.as_bytes()[j] as char)
                .map(is_word)
                .unwrap_or(false);
            let after = hay
                .as_bytes()
                .get(i + word.len())
                .map(|b| *b as char)
                .map(is_word)
                .unwrap_or(false);
            !before && !after
        })
    };
    for needle in ["sin", "cos", "atan2", "exp", "ln", "log", "powf", "sqrt"] {
        let present = code
            .iter()
            .any(|line| contains_word(line, needle) || line.contains("std::f64::consts"));
        assert!(
            !present,
            "no transcendental call may appear outside comments in assemble.rs (found {needle})"
        );
    }
}
