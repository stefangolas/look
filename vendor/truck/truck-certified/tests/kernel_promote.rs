//! BG-KV2-502-S9B integration tests: promotion of an assembled arc to a model
//! edge record (spec §14.3). The eight promotion conditions are walked by
//! `promote` over the stored assemble output; every assertion is Rules A/B/C
//! box containment, exact integer deck sums, the landed R9 residual id, the
//! typed condition-8 opt-in, or the exported record data. No solver is
//! invoked and no coordinate is ever moved (near pairs refuse, never snap).

#![deny(clippy::unwrap_used)]

use truck_certified::kernel::assemble::{regions_identify, ChainArc, ChainEnd};
use truck_certified::kernel::certs::{ContactCert, PointCert};
use truck_certified::kernel::config;
use truck_certified::kernel::evidence::Refusal as KernelRefusal;
use truck_certified::kernel::evidence::{RefusalEvidence, RefusalKind, VerdictClass};
use truck_certified::kernel::fixtures as fx;
use truck_certified::kernel::graph::{
    ArcId, ChartId, HermiteSegment, HermiteSpline, NodeCert, NodeId, Param, TopoNode,
};
use truck_certified::kernel::identity::{rule_b_transport, IdentityRule, IdentityVerdict};
use truck_certified::kernel::promote::{
    promote, InteriorEvent, KnotClass, PromoContext, SharedNode, TangencyOptIn,
};
use truck_certified::kernel::residual::ResidualId;
use truck_certified::kernel::SignCert;

/// The contraction rate carried by every fixture certificate (<= RHO_MAX).
const RHO: f64 = 0.125;

/// The near-miss separation of the sliver fixture: well inside EPS_REP, far
/// closer than any certified identity needs.
const NEAR_GAP: f64 = 1e-10; // H-3: sliver-fixture separation, far below EPS_REP

/// The fixture ground-truth agreement tolerance.
const GT_TOL: f64 = 1e-12; // H-3: dyadic fixture ground-truth comparison tolerance

/// The shared node id of the promoted self-loop edge.
const LOOP_NODE: usize = 7;

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

/// Extract the predicate name of a refusal that must carry predicate evidence.
fn refusal_name<T>(result: Result<T, KernelRefusal>) -> String {
    match result {
        Ok(_) => panic!("expected a refusal, got an accepted construction"),
        Err(refusal) => match refusal.evidence {
            RefusalEvidence::Predicate { name, .. } => name.to_string(),
            other => panic!("expected predicate evidence, got {other:?}"),
        },
    }
}

/// A parameter box `[u_lo, u_hi] x [v_lo, v_hi]`.
fn box2(u_lo: f64, u_hi: f64, v_lo: f64, v_hi: f64) -> truck_certified::kernel::patch::IBox2 {
    construct(truck_certified::kernel::patch::IBox2::try_new(
        [u_lo, v_lo],
        [u_hi, v_hi],
    ))
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

fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() <= GT_TOL
}

/// A one-segment straight Hermite approximant between two model points.
fn straight(p0: [f64; 3], p1: [f64; 3]) -> HermiteSpline {
    let d = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
    construct(HermiteSpline::try_new(vec![HermiteSegment {
        p0,
        p1,
        t0: d,
        t1: d,
    }]))
}

/// The model-space approximant of the closed full-wrap arc: a diamond polyline
/// that leaves and returns to `[1, 0, 0]` (four positive chords).
fn wrap_approx() -> HermiteSpline {
    let pts = [
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [-1.0, 0.0, 0.0],
        [0.0, -1.0, 0.0],
        [1.0, 0.0, 0.0],
    ];
    let mut segments = Vec::new();
    for w in pts.windows(2) {
        let p0 = w[0];
        let p1 = w[1];
        let d = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
        segments.push(HermiteSegment {
            p0,
            p1,
            t0: d,
            t1: d,
        });
    }
    construct(HermiteSpline::try_new(segments))
}

/// The closed full-wrap arc (a circle-carried self-loop on the periodic chart):
/// the start is deck 0 at canonical `u = 0` and the end is deck 1 at canonical
/// `u = 0` (developed `period`), with the same model point `[1, 0, 0]`. Its two
/// ends are the SAME shared node — the case the live topology constructors
/// panic on in debug, which the record exists to represent.
fn wrap_arc(period: f64) -> ChainArc {
    ChainArc {
        id: ArcId(0),
        start: ChainEnd {
            at: param(0, 0, 0.0, 0.0),
            point: [1.0, 0.0, 0.0],
            region: cert_box(ResidualId::R1, 0.0, 0.0, 0.0, 0.0),
        },
        end: ChainEnd {
            at: param(0, 1, 0.0, 0.0),
            point: [1.0, 0.0, 0.0],
            region: cert_box(ResidualId::R1, period, period, 0.0, 0.0),
        },
    }
}

/// The A4.2 union certificate of the closed wrap: one R1 box spanning the
/// developed seam `period`, which contains the hull of the Rule-B transported
/// start region and the end region.
fn wrap_unions(period: f64) -> Vec<(ResidualId, PointCert)> {
    vec![(
        ResidualId::R1,
        cert_box(ResidualId::R1, period - 1e-6, period + 1e-6, 0.0, 0.0),
    )]
}

/// The promotion context of the closed full-wrap arc.
fn wrap_context(period: f64, node: SharedNode, admit: TangencyOptIn) -> PromoContext {
    PromoContext {
        period,
        unions: wrap_unions(period),
        approx: wrap_approx(),
        charts: [ChartId(0), ChartId(1)],
        end_nodes: [node, node],
        interiors: Vec::new(),
        admit_tangent_at_tolerance: admit,
    }
}

/// An open chain arc whose two ends are far apart (no closure to certify): the
/// run `u 1 -> u 5` on deck 0.
fn open_arc() -> ChainArc {
    ChainArc {
        id: ArcId(1),
        start: ChainEnd {
            at: param(0, 0, 1.0, 0.0),
            point: [0.0, 0.0, 0.0],
            region: cert_box(ResidualId::R1, 1.0, 1.0, 0.0, 0.0),
        },
        end: ChainEnd {
            at: param(0, 0, 5.0, 0.0),
            point: [10.0, 0.0, 0.0],
            region: cert_box(ResidualId::R1, 5.0, 5.0, 0.0, 0.0),
        },
    }
}

/// An exact shared C1 node: the node certificate is the `Exact` point
/// certificate of the given region.
fn exact_node(id: usize, region: PointCert) -> SharedNode {
    SharedNode {
        id: NodeId(id),
        kind: TopoNode::Boundary,
        cert: NodeCert::Exact(region),
    }
}

/// A tangency-tagged shared node: certified only at tolerance (a §10.3
/// contact certificate — the tag condition 8 gates on).
fn tagged_node(id: usize) -> SharedNode {
    let critical = cert_box(ResidualId::R1, 1.0, 1.0, 0.0, 0.0);
    let contact = construct(ContactCert::try_new(
        critical,
        truck_certified::kernel::Interval {
            lo: -1e-12,
            hi: 1e-12,
        },
        SignCert::Positive,
    ));
    SharedNode {
        id: NodeId(id),
        kind: TopoNode::TrimCrossing,
        cert: NodeCert::AtTolerance(contact),
    }
}

// ---------------------------------------------------------------------------
// §14.3 promotion
// ---------------------------------------------------------------------------

#[test]
fn promotion_emits_arclength_parameterized_edge() {
    let fx = construct(fx::deck_wrap());
    let period = fx.period;
    let arc = wrap_arc(period);
    let node = exact_node(
        LOOP_NODE,
        cert_box(ResidualId::R1, period, period, 0.0, 0.0),
    );
    let ctx = wrap_context(period, node, TangencyOptIn::Refuse);
    let edge = construct(promote(&arc, &ctx));

    assert_eq!(edge.arc, ArcId(0));
    assert_eq!(
        edge.gamma, ctx.approx,
        "the record carries the model-space Hermite approximant"
    );

    // The exported arclength parameterization: a position table over the
    // approximant's five vertices, monotone, ending at the recorded model
    // point with the polyline's total chord length.
    assert_eq!(
        edge.arclength.table.len(),
        5,
        "one row per approximant vertex"
    );
    assert!(
        approx(edge.arclength.table[0].s, 0.0),
        "the table starts at s = 0"
    );
    assert_eq!(
        edge.arclength.table[0].p, arc.start.point,
        "the first table row is the start model point"
    );
    for pair in edge.arclength.table.windows(2) {
        assert!(
            pair[0].s <= pair[1].s,
            "the arclength table is monotone non-decreasing"
        );
    }
    assert_eq!(
        edge.arclength.table[4].p, arc.end.point,
        "the last table row is the end model point"
    );
    assert!(
        edge.arclength.total > edge.arclength.table[0].s,
        "a full wrap has positive total length"
    );
    assert!(
        approx(edge.arclength.total, 4.0 * 2.0f64.sqrt()),
        "the diamond perimeter is four chords of length sqrt(2)"
    );
    assert_eq!(
        edge.arclength.table[4].s, edge.arclength.total,
        "the last row carries the total length"
    );

    // Both pcurves in their lifted charts, on the owning-face charts.
    assert_eq!(edge.charts, [ChartId(0), ChartId(1)]);
    assert_eq!(edge.pcurves[0].chart, ChartId(0));
    assert_eq!(edge.pcurves[1].chart, ChartId(1));
    assert_eq!(edge.pcurves[0].from.deck, 0);
    assert_eq!(edge.pcurves[0].to.deck, 1);
    assert!(approx(edge.pcurves[0].from.u, 0.0));
    assert!(approx(edge.pcurves[0].to.u, 0.0));
    assert_eq!(edge.knots.len(), 0, "no interior event on the plain wrap");
}

#[test]
fn promoted_endpoints_are_shared_c1_nodes() {
    let fx = construct(fx::deck_wrap());
    let period = fx.period;
    let arc = wrap_arc(period);
    let node = exact_node(
        LOOP_NODE,
        cert_box(ResidualId::R1, period, period, 0.0, 0.0),
    );
    let ctx = wrap_context(period, node, TangencyOptIn::Refuse);
    let edge = construct(promote(&arc, &ctx));

    // The promoted self-loop edge ends on ONE shared C1 node: both record ends
    // reference the same shared node, certified exactly.
    assert_eq!(
        edge.ends[0].node, edge.ends[1].node,
        "the closed full-wrap arc's two ends are the same shared node"
    );
    assert_eq!(edge.ends[0].node, NodeId(LOOP_NODE));
    for end in &edge.ends {
        assert_eq!(end.kind, TopoNode::Boundary);
        assert!(
            matches!(end.cert, NodeCert::Exact(_)),
            "the shared node carries an Exact C1 certificate"
        );
        match end.cert {
            NodeCert::Exact(point) => {
                assert!(
                    point.rho <= config::RHO_MAX,
                    "the endpoint is certified C1 (rho within the ceiling)"
                );
                assert!(
                    point.rho > 0.0,
                    "the endpoint carries a genuine certificate"
                );
            }
            NodeCert::AtTolerance(_) => panic!("the shared loop node is not tolerance-tagged"),
        }
        assert_eq!(
            end.point,
            [1.0, 0.0, 0.0],
            "the recorded model point is carried verbatim — never snapped"
        );
    }

    // The closure was decided by the LANDED A4.2 rules (Rule B transport then
    // regions_identify), exactly as promotion reuses them — never proximity.
    let transported = construct(rule_b_transport(
        &arc.start.region,
        (1, 0),
        (period, 0.0),
        None,
    ));
    assert_eq!(
        regions_identify(&transported, &arc.end.region, &ctx.unions),
        IdentityVerdict::CertifiedEqual {
            rule: IdentityRule::RuleA,
        },
        "the two deck copies of the shared node identify under the landed A4.2 rules"
    );
    assert_eq!(
        edge.ends[0].region, arc.start.region,
        "the record end region is the certified start region, verbatim"
    );
}

#[test]
fn sliver_near_overlap_refuses_never_snaps() {
    let fx = construct(fx::deck_wrap());
    let period = fx.period;

    // A near closure: the two stored ends agree in model space within the
    // representation gap (the c1/glue agreement machinery would certify the
    // Hermite ends within the tube radius), but the end regions carry R1 and
    // R9 — residuals with no §4.2 implication — and no union is offered. The
    // tubes overlap; no A4.2 rule identifies the endpoints.
    let start = ChainEnd {
        at: param(0, 0, 0.5, 0.0),
        point: [0.0, 0.0, 0.0],
        region: cert_box(ResidualId::R1, 0.5, 0.5, 0.0, 0.0),
    };
    let end = ChainEnd {
        at: param(0, 1, 0.5, 0.0),
        point: [NEAR_GAP, 0.0, 0.0],
        region: cert_box(ResidualId::R9, period + 0.5, period + 0.5, 0.0, 0.0),
    };
    assert!(
        (start.point[0] - end.point[0]).abs() <= config::EPS_REP,
        "the fixture ends are nearer than the representation gap"
    );
    let arc = ChainArc {
        id: ArcId(2),
        start,
        end,
    };
    let ctx = PromoContext {
        period,
        unions: Vec::new(),
        approx: straight([0.0, 0.0, 0.0], [NEAR_GAP, 0.0, 0.0]),
        charts: [ChartId(0), ChartId(1)],
        end_nodes: [
            exact_node(0, cert_box(ResidualId::R1, 0.5, 0.5, 0.0, 0.0)),
            exact_node(
                1,
                cert_box(ResidualId::R9, period + 0.5, period + 0.5, 0.0, 0.0),
            ),
        ],
        interiors: Vec::new(),
        admit_tangent_at_tolerance: TangencyOptIn::Refuse,
    };

    let result = promote(&arc, &ctx);
    assert_eq!(
        refusal_kind(result),
        RefusalKind::SliverOrNearOverlap,
        "a near pair that does not identify is refused, never snapped"
    );
    assert_eq!(
        refusal_backing(promote(&arc, &ctx)),
        VerdictClass::Inconclusive,
        "SliverOrNearOverlap backs Inconclusive (§17)"
    );
    assert_eq!(
        refusal_name(promote(&arc, &ctx)),
        "sliver_near_overlap_refuses_never_snap"
    );

    // The refusal carries both endpoints verbatim: no coordinate was moved and
    // no midpoint was invented.
    let detail = match promote(&arc, &ctx) {
        Ok(_) => panic!("expected a sliver refusal"),
        Err(refusal) => match refusal.evidence {
            RefusalEvidence::Predicate { detail, .. } => detail,
            other => panic!("expected predicate evidence, got {other:?}"),
        },
    };
    assert!(
        detail.contains(&format!("{start:?}")),
        "the start endpoint is carried verbatim in the refusal"
    );
    assert!(
        detail.contains(&format!("{end:?}")),
        "the end endpoint is carried verbatim in the refusal"
    );
    assert_eq!(arc.start.point, start.point, "nothing was moved");
    assert_eq!(arc.end.point, end.point, "nothing was moved");
}

#[test]
fn deck_exhausted_routes_through_promotion() {
    let fx = construct(fx::deck_wrap());
    let period = fx.period;

    // A helix whose single promoted arc walks nine deck copies, above DECK_MAX
    // (8) — driven through promote directly with a forced deck-overflow
    // fixture, exactly as §14.2's deck_identify already refuses at assembly.
    let deep = config::DECK_MAX + 1;
    let arc = ChainArc {
        id: ArcId(3),
        start: ChainEnd {
            at: param(0, 0, 0.5, 0.0),
            point: [0.5, 0.0, 0.0],
            region: cert_box(ResidualId::R1, 0.5, 0.5, 0.0, 0.0),
        },
        end: ChainEnd {
            at: param(0, deep, 0.5, 0.0),
            point: [50.0, 0.0, 0.0],
            region: cert_box(
                ResidualId::R1,
                0.5 + period * deep as f64,
                0.5 + period * deep as f64,
                0.0,
                0.0,
            ),
        },
    };
    let ctx = PromoContext {
        period,
        unions: Vec::new(),
        approx: straight([0.5, 0.0, 0.0], [50.0, 0.0, 0.0]),
        charts: [ChartId(0), ChartId(1)],
        end_nodes: [
            exact_node(0, cert_box(ResidualId::R1, 0.5, 0.5, 0.0, 0.0)),
            exact_node(
                1,
                cert_box(
                    ResidualId::R1,
                    0.5 + period * deep as f64,
                    0.5 + period * deep as f64,
                    0.0,
                    0.0,
                ),
            ),
        ],
        interiors: Vec::new(),
        admit_tangent_at_tolerance: TangencyOptIn::Refuse,
    };

    assert_eq!(
        refusal_kind(promote(&arc, &ctx)),
        RefusalKind::DeckExhausted,
        "the §0.4 deck ceiling holds inside a promoted arc"
    );
    assert_eq!(
        refusal_backing(promote(&arc, &ctx)),
        VerdictClass::Inconclusive,
        "DeckExhausted backs Inconclusive (§17)"
    );
    assert_eq!(
        refusal_name(promote(&arc, &ctx)),
        "promote_deck_max_exceeded"
    );
}

#[test]
fn knot_multiplicity_set_at_crossings_and_cusps() {
    let fx = construct(fx::deck_wrap());
    let period = fx.period;
    let arc = open_arc();

    // Two certified interior events in the run: a trim crossing certified by
    // the landed one-chart R9 residual (condition 3) and an A2 cusp.
    let crossing = InteriorEvent {
        at: param(0, 0, 2.0, 0.0),
        class: KnotClass::Crossing,
        cert: cert_box(ResidualId::R9, 2.0, 2.0, 0.0, 0.0),
    };
    let cusp = InteriorEvent {
        at: param(0, 0, 3.5, 0.0),
        class: KnotClass::Cusp,
        cert: cert_box(ResidualId::R4Prime, 3.5, 3.5, 0.0, 0.0),
    };
    let ctx = PromoContext {
        period,
        unions: Vec::new(),
        approx: straight([0.0, 0.0, 0.0], [10.0, 0.0, 0.0]),
        charts: [ChartId(0), ChartId(1)],
        end_nodes: [
            exact_node(0, cert_box(ResidualId::R1, 1.0, 1.0, 0.0, 0.0)),
            exact_node(1, cert_box(ResidualId::R1, 5.0, 5.0, 0.0, 0.0)),
        ],
        interiors: vec![crossing, cusp],
        admit_tangent_at_tolerance: TangencyOptIn::Refuse,
    };

    let edge = construct(promote(&arc, &ctx));
    assert_eq!(
        edge.knots.len(),
        2,
        "each interior crossing/cusp becomes an interior knot"
    );
    assert_eq!(edge.knots[0].class, KnotClass::Crossing);
    assert_eq!(
        edge.knots[0].multiplicity, 2,
        "a transversal trim crossing sets knot multiplicity 2"
    );
    assert_eq!(edge.knots[0].at, crossing.at);
    assert_eq!(edge.knots[0].cert, crossing.cert);
    assert_eq!(edge.knots[1].class, KnotClass::Cusp);
    assert_eq!(
        edge.knots[1].multiplicity, 3,
        "an A2 cusp sets knot multiplicity 3"
    );
    assert_eq!(edge.knots[1].at, cusp.at);
}

#[test]
fn tangency_tag_requires_explicit_opt_in() {
    let fx = construct(fx::deck_wrap());
    let period = fx.period;
    let arc = open_arc();

    // Both shared end nodes carry the §10.3 tangency-at-tolerance tag.
    let tags = [tagged_node(0), tagged_node(1)];
    let refused_ctx = PromoContext {
        period,
        unions: Vec::new(),
        approx: straight([0.0, 0.0, 0.0], [10.0, 0.0, 0.0]),
        charts: [ChartId(0), ChartId(1)],
        end_nodes: tags,
        interiors: Vec::new(),
        admit_tangent_at_tolerance: TangencyOptIn::Refuse,
    };

    let result = promote(&arc, &refused_ctx);
    assert_eq!(
        refusal_kind(result),
        RefusalKind::TangentialCurve,
        "a tagged endpoint refuses promotion without the opt-in"
    );
    assert_eq!(
        refusal_backing(promote(&arc, &refused_ctx)),
        VerdictClass::Inconclusive,
        "TangentialCurve backs Inconclusive (§17)"
    );
    assert_eq!(
        refusal_name(promote(&arc, &refused_ctx)),
        "promote_tangency_at_tolerance_requires_opt_in"
    );

    // The typed opt-in admits the tagged endpoints (never a default true).
    let admitted_ctx = PromoContext {
        admit_tangent_at_tolerance: TangencyOptIn::Admit,
        ..refused_ctx
    };
    let edge = construct(promote(&arc, &admitted_ctx));
    for end in &edge.ends {
        assert!(
            matches!(end.cert, NodeCert::AtTolerance(_)),
            "the admitted edge keeps its at-tolerance endpoint certificates"
        );
    }
}
