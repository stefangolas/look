#![cfg_attr(not(debug_assertions), deny(warnings))]
#![deny(clippy::all, rust_2018_idioms)]
#![deny(clippy::unwrap_used)]
#![warn(
    missing_docs,
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]

//! Segment gluing, deck identification, and graph assembly (BG-KV2-303-S9A).
//!
//! This module wires the landed identity Rules A/B/C (`kernel/identity.rs`,
//! consumed, never restated) and the landed deck/lattice arithmetic substrate
//! (`formal/deck.rs`, `domain/lattice.rs`) to arc chains, then assembles a
//! [`CertifiedGraph`] from certified arcs (§14.1–§14.2).
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module. The module carries no `unwrap`, no `expect`, and no `panic!`
//! calls, and adds no module-level `allow`.
//!
//! **N4.** No transcendental call appears in this module — no `sin`, `cos`,
//! `atan2`, `exp`, `ln`, `log`, `powf`, `sqrt`, and no `std::f64::consts`. The
//! classification source-scan test pins this.
//!
//! **§14.2 gluing (the condition set between arcs A, B at a SegmentBreak).**
//!
//! 1. **Tube overlap.** The shared point is the identity-Rule-A/B/C match of
//!    the two arcs' endpoint regions: [`regions_identify`] consumes the landed
//!    [`rule_a`]/[`rule_c`]; Rule B's transport ([`rule_b_transport`]) enters at
//!    the deck closure below. Float proposes (the nearer endpoint pair — the
//!    recorded Hermite model ends), intervals dispose (the certified C1
//!    enclosure, which decides whether the pair really is one point).
//! 2. **C1 agreement within [`crate::kernel::config::EPS_REP`].** The C1 bound
//!    is certified by the interval evaluation of both approximants'
//!    endpoints/derivatives at the junction ([`c1_bound_of`]): a plain
//!    outward-rounded enclosure comparison — no snapping, ever. A pair whose
//!    bound exceeds `EPS_REP` refuses.
//! 3. **Monotone reparameterization.** The concatenated pcurve reparameterizes
//!    to a single monotone parameter (the arclength of the model-space
//!    approximant), recorded as the ledger's parameter domain. The
//!    `EdgeSampleLedger` integration is C3's landed entry and is NOT reachable
//!    through this tree's public path yet — recorded as the **S9b seam**: this
//!    wave certifies the C1 data statement and books the ledger wiring for S9b.
//!    The deck winding is likewise carried as data on the emitted
//!    [`SegmentBreak::DeckStep`] breaks (the frozen [`CertifiedGraph`] shape
//!    has no per-arc winding field; adding one would be a frozen-shape
//!    amendment and is NOT made here).
//!
//! Tubes overlapping whose endpoints do NOT match under any rule of §4.2 ->
//! [`RefusalKind::SliverOrNearOverlap`] (Inconclusive): a near pair is refused,
//! never snapped.
//!
//! **Deck identification (§14.2).** An arc ending at `(chart, deck = k, u~)`
//! and one beginning at `(chart, deck = k + 1, u~ - P)` denote the same point.
//! [`deck_identify`] walks a closed chain of certified arcs on one periodic
//! chart and computes the total deck displacement as **exact integer sums** of
//! the carried deck indices — the chain's chart data carries the periods, so
//! the landed `formal/deck.rs` interval *solver* is not the right tool (it
//! decides a deck placement from real-valued enclosures when the integer is not
//! yet known); no adapter is missing. A chain whose first-start and last-end
//! endpoints differ by an exact integer deck translation AND whose endpoint
//! regions identify under Rule B closes as a loop; the displacement is recorded
//! as the winding, carried by the emitted deck-step breaks. `|deck| > DECK_MAX`
//! on one edge -> [`RefusalKind::DeckExhausted`] (Inconclusive).
//!
//! **Assembly (§16).** [`assemble`] validates that every [`ArcEnd::Topo`]
//! resolves to a [`Node`] whose [`NodeCert`] is Exact or AtTolerance (the
//! exhaustive two-variant match is the shape pin), every [`ArcEnd::Seg`]
//! resolves to a [`Break`] (which by frozen shape always grounds a
//! [`TubeOverlapCert`]), and assembles the [`CertifiedGraph`]. No [`TopoNode`]
//! variant is `Refuse` — the shim's enum makes that structural; the integration
//! test pins the exhaustive match. Identity is decided by Rules A/B/C only —
//! never by proximity.

use crate::kernel::certs::{PointCert, TubeOverlapCert};
use crate::kernel::config::DECK_MAX;
use crate::kernel::evidence::{Construction, Refusal, RefusalEvidence, RefusalKind};
use crate::kernel::graph::{
    AnyArc, Arc, ArcEnd, ArcId, Break, BreakId, CertifiedGraph, Node, NodeCert, Param, Point4,
    SegmentBreak,
};
use crate::kernel::identity::{rule_a, rule_b_transport, rule_c, IdentityRule, IdentityVerdict};
use crate::kernel::residual::ResidualId;

/// The stored model-space end of an arc side at a §14.2 junction: the position
/// and the derivative (tangent) recorded by the arc's stored Hermite
/// approximant (`approx.gamma`) at that end.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HermiteEnd {
    /// The model-space position of the recorded end.
    pub point: [f64; 3],
    /// The model-space derivative (tangent) recorded at that end.
    pub tangent: [f64; 3],
}

/// One side of a §14.2 gluing junction: the certified endpoint region of the
/// arc (the parameter box and residual that certified it) plus the arc's stored
/// Hermite end at the junction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlueSide {
    /// The certified endpoint region of the arc.
    pub region: PointCert,
    /// The stored Hermite end of the arc's approximant at the junction.
    pub end: HermiteEnd,
}

/// A certified §14.2 glue between two arcs meeting at a segment break: the
/// identity rule that matched the endpoint regions, the shared model point, and
/// the tube-overlap certificate whose `c1_bound` is the certified C1-agreement
/// bound at the junction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlueCert {
    /// The identity rule that certified the endpoint-region equality.
    pub rule: IdentityRule,
    /// The shared model point (the nearer-pair proposal, disposed by the
    /// certified C1 enclosure).
    pub shared_point: [f64; 3],
    /// The tube-overlap certificate; `c1_bound <= EPS_REP` by construction.
    pub overlap: TubeOverlapCert,
}

/// One certified end of a chain arc: the chart parameter (chart, deck, canonical
/// `(u, v)`), the recorded model-space point, and the certified endpoint region
/// (a `PointCert` stated in the developed coordinates of that deck copy).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChainEnd {
    /// The chart parameter of the end.
    pub at: Param,
    /// The recorded model-space point of the end.
    pub point: [f64; 3],
    /// The certified region of the end, stated in the deck copy's developed
    /// coordinates.
    pub region: PointCert,
}

/// One certified arc of a deck chain (a §14.2 data statement): the arc's two
/// certified chart ends. The chart period travels with the call.
#[derive(Debug, Clone, PartialEq)]
pub struct ChainArc {
    /// The arc id.
    pub id: ArcId,
    /// The arc's first end.
    pub start: ChainEnd,
    /// The arc's second end.
    pub end: ChainEnd,
}

/// A single deck-seam crossing inside a chain arc, in traversal order.
#[derive(Debug, Clone, Copy)]
struct Crossing {
    /// The index of the crossing chain arc.
    arc: usize,
    /// The deck index of the copy just left (before the seam).
    deck_left: i32,
    /// The deck index of the copy entered (after the seam).
    deck_entered: i32,
    /// The developed coordinate of the seam: an exact integer multiple of the
    /// period.
    seam_raw: f64,
}

/// §4.2 node identity for two certified endpoint regions, consuming the landed
/// Rules A/B/C ([`rule_a`], [`rule_c`]) — never proximity.
///
/// Rule A is tried against every caller union certificate (the same-residual
/// containment test); Rule C goes through the typed implication relation.
/// Anything ambiguous is [`IdentityVerdict::NotCertified`].
pub fn regions_identify(
    a: &PointCert,
    b: &PointCert,
    unions: &[(ResidualId, PointCert)],
) -> IdentityVerdict {
    for (_, union_cert) in unions {
        if let IdentityVerdict::CertifiedEqual { rule } = rule_a(a, b, union_cert) {
            return IdentityVerdict::CertifiedEqual { rule };
        }
    }
    rule_c(a, b, unions)
}

/// The certified C1-agreement bound of two stored Hermite ends (§14.2 condition
/// 2): the interval evaluation of both approximants' endpoints/derivatives over
/// the junction's shared-point box.
///
/// Each recorded end is enclosed as the degenerate interval at its stored
/// value; each per-component difference is pushed one ULP outward (the landed
/// outward-rounding discipline — a single ULP step is the only rounding
/// device). The certified bound is the maximum outward-rounded magnitude over
/// the position and tangent components. It is a plain enclosure comparison: the
/// two arcs either agree within [`EPS_REP`] or they do not — there is no
/// snapping, ever.
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
pub fn c1_bound_of(a: &HermiteEnd, b: &HermiteEnd) -> Result<f64, Refusal> {
    let mut bound = 0.0f64;
    for k in 0..3 {
        for diff in [a.point[k] - b.point[k], a.tangent[k] - b.tangent[k]] {
            if !diff.is_finite() {
                return Err(refusal(
                    RefusalKind::NonFinite,
                    "glue_c1_difference_not_finite",
                    format!("C1 difference component {k} is not finite"),
                ));
            }
            let widened = diff.abs().next_up();
            if !widened.is_finite() {
                return Err(refusal(
                    RefusalKind::NonFinite,
                    "glue_c1_difference_overflows",
                    format!("outward-rounded C1 difference component {k} is not finite"),
                ));
            }
            bound = bound.max(widened);
        }
    }
    Ok(bound)
}

/// §14.2 gluing between arc A (side `a`) and arc B (side `b`) meeting at a
/// segment break.
///
/// The tube-overlap condition (condition 1) and the C1-agreement condition
/// (condition 2) must BOTH certify: the endpoint regions must identify under a
/// §4.2 rule ([`regions_identify`]), and the stored Hermite ends must agree to
/// C1 within [`EPS_REP`] ([`c1_bound_of`], then the shim's refusing
/// [`TubeOverlapCert::try_new`]). Condition 3 (the monotone arclength
/// reparameterization recorded as the ledger's parameter domain) is the S9b
/// seam — not reachable in this tree, booked.
///
/// Tubes overlapping whose endpoints do NOT match under any rule of §4.2 are
/// refused [`RefusalKind::SliverOrNearOverlap`] (Inconclusive) — a near pair is
/// refused, never snapped.
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
pub fn glue(
    a: &GlueSide,
    b: &GlueSide,
    unions: &[(ResidualId, PointCert)],
) -> Construction<GlueCert> {
    let rule = match regions_identify(&a.region, &b.region, unions) {
        IdentityVerdict::CertifiedEqual { rule } => rule,
        IdentityVerdict::NotCertified => {
            return Err(refusal(
                RefusalKind::SliverOrNearOverlap,
                "endpoint_regions_not_certified_equal",
                format!(
                    "arc endpoint regions do not identify under any §4.2 rule \
                     (residuals {:?} and {:?})",
                    a.region.residual, b.region.residual
                ),
            ))
        }
    };
    let bound = c1_bound_of(&a.end, &b.end)?;
    let shared_point = mid3(a.end.point, b.end.point);
    let overlap = TubeOverlapCert::try_new(shared_point, bound)?;
    Ok(GlueCert {
        rule,
        shared_point,
        overlap,
    })
}

/// Compute the total deck displacement of a chain of certified arcs on one
/// periodic chart and emit the [`SegmentBreak::DeckStep`] breaks that record
/// each seam crossing (§14.2).
///
/// The displacement is exact integer arithmetic: the chain's chart data carries
/// the periods as deck indices on each [`ChainEnd`], so the per-arc displacement
/// is `end.deck - start.deck` — exact `i32`/`i64` sums, no float rounding. The
/// landed `formal/deck.rs` interval solver is not needed here: there is no
/// real-valued enclosure to decide, so no adapter is missing.
///
/// A chain whose first-start and last-end endpoints lie on the same chart with
/// equal canonical `(u, v)` differs by an exact integer deck translation; when
/// the two endpoint regions also identify under Rule B (transport by the deck
/// difference, then the containment test) the chain closes as a loop. The
/// displacement is the loop's winding and is carried by the emitted deck-step
/// breaks (each break advances one deck). `|deck| > DECK_MAX` on one edge (the
/// §0.4 per-edge ceiling) refuses [`RefusalKind::DeckExhausted`]
/// (Inconclusive).
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
pub fn deck_identify(
    chain: &[ChainArc],
    period: f64,
    unions: &[(ResidualId, PointCert)],
) -> Construction<Vec<Break>> {
    if !period.is_finite() {
        return Err(refusal(
            RefusalKind::NonFinite,
            "deck_period_not_finite",
            "the chart period must be finite".to_string(),
        ));
    }
    if period <= 0.0 {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "deck_period_nonpositive",
            format!("the chart period {period} must be strictly positive"),
        ));
    }
    if chain.is_empty() {
        return Ok(Vec::new());
    }
    for pair in chain.windows(2) {
        if pair[0].end.at != pair[1].start.at {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "chain_junction_disconnected",
                format!(
                    "chain junction between arcs {:?} and {:?} does not join",
                    pair[0].id, pair[1].id
                ),
            ));
        }
    }
    let mut total_deck: i64 = 0;
    let mut crossings: Vec<Crossing> = Vec::new();
    for (index, arc) in chain.iter().enumerate() {
        let start = &arc.start;
        let end = &arc.end;
        if start.at.chart != end.at.chart {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "deck_chain_crosses_charts",
                format!(
                    "arc {:?} spans two charts; deck identification runs on one chart",
                    arc.id
                ),
            ));
        }
        if !(0.0..period).contains(&start.at.u) || !(0.0..period).contains(&end.at.u) {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "deck_param_not_canonical",
                format!(
                    "arc {:?} has a non-canonical end parameter; canonical u must lie in [0, period)",
                    arc.id
                ),
            ));
        }
        let raw_start = raw_u(start.at.deck, start.at.u, period)?;
        let raw_end = raw_u(end.at.deck, end.at.u, period)?;
        if raw_end == raw_start {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "deck_arc_zero_run",
                format!("arc {:?} spans a zero developed run", arc.id),
            ));
        }
        let displacement = (end.at.deck as i64) - (start.at.deck as i64);
        total_deck += displacement;
        if displacement.unsigned_abs() > DECK_MAX as u64 {
            return Err(deck_exhausted(displacement, arc.id));
        }
        append_crossings(index, start.at.deck, end.at.deck, period, &mut crossings)?;
    }
    if crossings.is_empty() {
        return Ok(Vec::new());
    }
    let first_start = &chain[0].start;
    let last_end = &chain[chain.len() - 1].end;
    let closes = first_start.at.chart == last_end.at.chart
        && first_start.at.u == last_end.at.u
        && first_start.at.v == last_end.at.v;
    if closes {
        let deck_translation = (last_end.at.deck as i64) - (first_start.at.deck as i64);
        if deck_translation != total_deck {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "deck_loop_displacement_inconsistent",
                format!(
                    "closed chain displacement {total_deck} disagrees with its endpoint \
                     deck translation {deck_translation}"
                ),
            ));
        }
        // Rule B closure: transport the first-start region onto the last-end
        // deck copy and run the containment test (§4.2, Rule B then Rule A).
        let transported = rule_b_transport(
            &first_start.region,
            (deck_translation as i32, 0),
            (period, 0.0),
            None,
        )?;
        match regions_identify(&transported, &last_end.region, unions) {
            IdentityVerdict::CertifiedEqual { .. } => {}
            IdentityVerdict::NotCertified => {
                return Err(refusal(
                    RefusalKind::SliverOrNearOverlap,
                    "deck_loop_endpoints_not_rule_b_certified",
                    format!(
                        "closed chain endpoints differ by an exact deck translation but do not \
                         identify under Rule B (displacement {total_deck})"
                    ),
                ))
            }
        }
    }
    let mut breaks = Vec::new();
    for (index, crossing) in crossings.iter().enumerate() {
        breaks.push(build_deck_break(
            index,
            &chain[crossing.arc],
            crossing,
            period,
        )?);
    }
    Ok(breaks)
}

/// The developed (unwrapped) coordinate `deck * period + u`.
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
fn raw_u(deck: i32, u: f64, period: f64) -> Result<f64, Refusal> {
    let raw = deck as f64 * period + u;
    if !raw.is_finite() {
        return Err(refusal(
            RefusalKind::NonFinite,
            "deck_raw_not_finite",
            format!("developed coordinate for deck {deck} and u {u} is not finite"),
        ));
    }
    Ok(raw)
}

/// Record every seam crossing of one chain arc, in traversal order.
///
/// Each deck boundary `m` between deck `m - 1` and deck `m` is crossed once
/// when the developed run passes it; `m` runs over `min_deck + 1 ..= max_deck`
/// in traversal order.
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
fn append_crossings(
    index: usize,
    start_deck: i32,
    end_deck: i32,
    period: f64,
    out: &mut Vec<Crossing>,
) -> Result<(), Refusal> {
    let lo = start_deck.min(end_deck);
    let hi = start_deck.max(end_deck);
    for m in (lo + 1)..=hi {
        let seam_raw = m as f64 * period;
        if !seam_raw.is_finite() {
            return Err(refusal(
                RefusalKind::NonFinite,
                "deck_seam_not_finite",
                format!("seam coordinate for deck {m} is not finite"),
            ));
        }
        let (deck_left, deck_entered) = if end_deck >= start_deck {
            (m - 1, m)
        } else {
            (m, m - 1)
        };
        out.push(Crossing {
            arc: index,
            deck_left,
            deck_entered,
            seam_raw,
        });
    }
    Ok(())
}

/// Build one [`Break`] for a recorded seam crossing, carrying the exact
/// deck-boundary parameter pair and the trivial same-arc tube overlap at the
/// seam.
///
/// The two params of the seam point denote the same point per §14.2:
/// `(chart, deck = k, u = P)` and `(chart, deck = k + 1, u = 0)`. The seam's
/// model position is the linear proposal along the crossing arc's recorded
/// ends (the float proposal); the exact crossing is the ledger wave's (S9b)
/// entry. Both recorded ends carry equal `v` for the fixture families.
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
fn build_deck_break(
    index: usize,
    arc: &ChainArc,
    crossing: &Crossing,
    period: f64,
) -> Result<Break, Refusal> {
    let chart = arc.start.at.chart;
    let ascending = crossing.deck_entered > crossing.deck_left;
    let (u_left, u_entered) = if ascending {
        (period, 0.0)
    } else {
        (0.0, period)
    };
    let seam_v = (arc.start.at.v + arc.end.at.v) * 0.5;
    let seam_point = seam_model_point(arc, crossing.seam_raw, period)?;
    Ok(Break {
        id: BreakId(index),
        at: Point4 {
            p1: Param::try_new(chart, crossing.deck_left, u_left, seam_v)?,
            p2: Param::try_new(chart, crossing.deck_entered, u_entered, seam_v)?,
        },
        kind: SegmentBreak::DeckStep,
        overlap: TubeOverlapCert::try_new(seam_point, 0.0)?,
    })
}

/// The model-space point of a deck seam, linearly proposed along the crossing
/// arc's recorded ends (the float proposal; no transcendental arithmetic).
///
/// The fraction is taken over the developed run: `t = (seam_raw - raw_start) /
/// (raw_end - raw_start)`. The exact crossing is the ledger wave's (S9b) entry.
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
fn seam_model_point(arc: &ChainArc, seam_raw: f64, period: f64) -> Result<[f64; 3], Refusal> {
    let raw_start = raw_u(arc.start.at.deck, arc.start.at.u, period)?;
    let raw_end = raw_u(arc.end.at.deck, arc.end.at.u, period)?;
    if raw_end == raw_start {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "deck_arc_zero_run",
            format!("arc {:?} spans a zero developed run", arc.id),
        ));
    }
    let t = (seam_raw - raw_start) / (raw_end - raw_start);
    let mut point = [0.0f64; 3];
    for (out, (from, to)) in point
        .iter_mut()
        .zip(arc.start.point.iter().zip(arc.end.point.iter()))
    {
        *out = *from + (*to - *from) * t;
    }
    if !point.iter().all(|c| c.is_finite()) {
        return Err(refusal(
            RefusalKind::NonFinite,
            "deck_seam_model_point_not_finite",
            format!("seam model point {point:?} is not finite"),
        ));
    }
    Ok(point)
}

/// Assemble a certified graph from certified arcs, segment breaks, and nodes
/// (§16).
///
/// Validation: every [`ArcEnd::Topo`] resolves to a [`Node`] whose [`NodeCert`]
/// is Exact or AtTolerance (the exhaustive two-variant match below is the shape
/// pin — a `Refuse` certificate would not compile); every [`ArcEnd::Seg`]
/// resolves to a [`Break`], which by frozen shape grounds a
/// [`TubeOverlapCert`]. Carrier arcs carry no ends and are admitted as-is. A
/// reference that does not resolve refuses [`RefusalKind::ClaimRefuted`].
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
pub fn assemble(
    arcs: Vec<AnyArc>,
    breaks: Vec<Break>,
    nodes: Vec<Node>,
) -> Construction<CertifiedGraph> {
    for arc in &arcs {
        let ends = arc_ends(arc);
        for (position, end) in ends.iter().enumerate() {
            match *end {
                ArcEnd::Topo(node_id) => {
                    let node = match nodes.iter().find(|node| node.id == node_id) {
                        Some(node) => node,
                        None => {
                            return Err(refusal(
                                RefusalKind::ClaimRefuted,
                                "assembled_end_resolves_to_missing_node",
                                format!(
                                    "an arc end ({position}) references node {node_id:?} which \
                                     is not in the node set"
                                ),
                            ))
                        }
                    };
                    // The exhaustive two-variant match: every node certificate is
                    // Exact or AtTolerance — never a refusal.
                    match node.cert {
                        NodeCert::Exact(_) => {}
                        NodeCert::AtTolerance(_) => {}
                    }
                }
                ArcEnd::Seg(break_id) => {
                    if !breaks.iter().any(|b| b.id == break_id) {
                        return Err(refusal(
                            RefusalKind::ClaimRefuted,
                            "assembled_end_resolves_to_missing_break",
                            format!(
                                "an arc end ({position}) references break {break_id:?} which \
                                 is not in the break set"
                            ),
                        ));
                    }
                }
            }
        }
    }
    Ok(CertifiedGraph {
        nodes,
        breaks,
        arcs,
        sheets: Vec::new(),
        exhaustive: false,
    })
}

/// The two ends of an [`AnyArc`], in order; a carrier arc has none.
fn arc_ends(arc: &AnyArc) -> Vec<ArcEnd> {
    match arc {
        AnyArc::Ordinary(Arc { ends, .. }) => vec![ends.0, ends.1],
        AnyArc::Difference(Arc { ends, .. }) => vec![ends.0, ends.1],
        AnyArc::SelfInt(Arc { ends, .. }) => vec![ends.0, ends.1],
        AnyArc::Spine(Arc { ends, .. }) => vec![ends.0, ends.1],
        AnyArc::Carrier(_) => Vec::new(),
    }
}

/// The DeckExhausted refusal for a displacement magnitude over the ceiling.
fn deck_exhausted(displacement: i64, arc: ArcId) -> Refusal {
    refusal(
        RefusalKind::DeckExhausted,
        "deck_max_exceeded",
        format!(
            "|deck displacement| {} exceeds DECK_MAX {DECK_MAX} on the edge {arc:?}",
            displacement.unsigned_abs()
        ),
    )
}

/// The model-space midpoint of two points (the float proposal; the certified
/// enclosure disposes).
fn mid3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        (a[0] + b[0]) * 0.5,
        (a[1] + b[1]) * 0.5,
        (a[2] + b[2]) * 0.5,
    ]
}

/// A named predicate refusal: the kind names the violated class, the predicate
/// name the precise invariant.
fn refusal(kind: RefusalKind, name: &'static str, detail: String) -> Refusal {
    Refusal::new(kind, RefusalEvidence::Predicate { name, detail })
}
