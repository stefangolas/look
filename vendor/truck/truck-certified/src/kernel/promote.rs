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

//! §14.3 promotion of an assembled arc to a model edge (BG-KV2-502-S9B).
//!
//! This module lands the census S9 second half: the assembled arc — a §14.2
//! [`ChainArc`] data statement over the frozen assemble output — is promoted to
//! a model-edge KERNEL RECORD, deliberately NOT a live `truck_topology::Edge`
//! handle. The landed topology constructors panic in debug on circle-carried
//! self-loops; the record avoids the whole class, and binding the record to
//! live handles is downstream integration, not this packet.
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module. The module carries no `unwrap`, no `expect`, and no `panic!`
//! calls, and adds no module-level `allow`.
//!
//! **D-record.** [`PromotedEdge`] is plain data (`Debug` + `Clone`), no
//! serialization is booked for this wave. Every geometry field is a data
//! statement over the certified assemble output — nothing here solves.
//!
//! **D-reuse.** The eight §14.3 promotion conditions are walked in order as one
//! refusing entry, [`promote`]. Each failing condition is a NAMED refusal
//! carrying the evidence the spec names for it. The conditions reuse the
//! LANDED seams only, never re-derive them:
//!
//! 1. **Run and deck ceiling.** The arc is one chart run with finite, canonical
//!    stored ends, a strictly positive developed run, a model-space approximant
//!    whose stored ends agree with the chain ends, and a deck magnitude at or
//!    below [`crate::kernel::config::DECK_MAX`] inside the promoted arc — the
//!    spec's termination bound holds at promotion even though §14.2's
//!    `deck_identify` already refuses at assembly. The over-ceiling refusal is
//!    `DeckExhausted`.
//! 2. **Endpoint identity.** An arc whose stored ends overlap in model space
//!    (within the representation gap) must certify as ONE shared C1 node under
//!    the LANDED A4.2 rules — 303's `regions_identify` (with the landed Rule B
//!    transport of `kernel::identity` when the two ends sit on different deck
//!    copies), never proximity. Tubes overlap but no A4.2 rule identifies the
//!    endpoints -> [`RefusalKind::SliverOrNearOverlap`], and the refusal
//!    carries both endpoints verbatim — a near pair is refused, never snapped.
//!    This is the single-arc self-loop closure case (a full circle wrap closes
//!    the edge on one shared node).
//! 3. **Trim events in one chart.** Every interior trim crossing routes through
//!    the LANDED one-chart R9 residual (`kernel::residuals_r89`): a
//!    [`KnotClass::Crossing`] event whose certificate is not the R9 residual is
//!    refused.
//! 4. **Interior order.** The interior events are finite, certified C1 points
//!    strictly inside the arc's developed run, in traversal order, distinct.
//! 5. **Endpoint C1 certificates.** The stored endpoint region of each end and
//!    the certificate of the shared graph node each end resolves to are C1
//!    certificates (finite, contraction rate at or below
//!    [`crate::kernel::config::RHO_MAX`]).
//! 6. **Knot multiplicities at crossings and cusps.** Each interior
//!    [`KnotClass::Crossing`] / [`KnotClass::Cusp`] event becomes an
//!    [`EdgeKnot`] whose multiplicity is set from its certified class (the
//!    §14.3 table: a transversal crossing joins two 1-complex branches — knot
//!    multiplicity 2; an A2 cusp degenerates the tangent — knot multiplicity 3).
//! 7. **Arclength parameterization.** The exported arclength parameterization
//!    carries its position table over the model-space approximant's vertices; a
//!    zero-total-length edge is not a model edge and is refused.
//! 8. **The tangency-at-tolerance gate.** A shared end node certified only at
//!    tolerance (a §10.3 rule-7 [`NodeCert::AtTolerance`] certificate — the
//!    tangency tag) refuses promotion UNLESS the [`PromoContext`] carries the
//!    explicit typed opt-in [`TangencyOptIn::Admit`]. The flag is a typed
//!    field, not a bool default true: [`promote`] never assumes admission.

use crate::kernel::assemble::{regions_identify, ChainArc, ChainEnd};
use crate::kernel::certs::PointCert;
use crate::kernel::config::{DECK_MAX, EPS_REP, RHO_MAX};
use crate::kernel::evidence::{Construction, Refusal, RefusalEvidence, RefusalKind};
use crate::kernel::graph::{
    ArcId, ChartId, HermiteSegment, HermiteSpline, NodeCert, NodeId, Param, TopoNode,
};
use crate::kernel::identity::{rule_b_transport, IdentityVerdict};
use crate::kernel::residual::ResidualId;

/// The typed §14.3 condition-8 opt-in: whether an end node carrying the
/// tangency-at-tolerance tag is admitted into the promoted edge.
///
/// This is a typed field, deliberately NOT a bool default true — [`promote`]
/// never assumes an at-tolerance endpoint is admitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TangencyOptIn {
    /// The caller explicitly admits an endpoint certified only at tolerance.
    Admit,
    /// The tangency tag refuses the promotion.
    Refuse,
}

/// The shared graph node one end of the promoted edge resolves to (spec §16):
/// the topology node id, kind, and certificate.
///
/// The endpoint is a certified node of the assembled graph: [`NodeCert::Exact`]
/// carries the C1 point certificate, [`NodeCert::AtTolerance`] the §10.3
/// rule-7 tangency tag (§14.3 condition 8). Never a live `truck_topology`
/// handle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SharedNode {
    /// The certified topology node id.
    pub id: NodeId,
    /// The certified node kind.
    pub kind: TopoNode,
    /// The node certificate.
    pub cert: NodeCert,
}

/// One certified side of the promoted edge: the chain end (chart parameter,
/// recorded model-space point, certified endpoint region) fused with the shared
/// graph node it resolves to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SharedEnd {
    /// The chart parameter of the chain end.
    pub at: Param,
    /// The recorded model-space point of the chain end.
    pub point: [f64; 3],
    /// The certified endpoint region of the chain end (the C1 certificate).
    pub region: PointCert,
    /// The shared node id this end resolves to (the same id for a closed
    /// self-loop edge).
    pub node: NodeId,
    /// The node kind.
    pub kind: TopoNode,
    /// The node certificate.
    pub cert: NodeCert,
}

/// One pcurve of the promoted edge: the certified deck run of the edge in one
/// owning-face chart, in its lifted (decked) chart coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pcurve {
    /// The owning-face chart of this pcurve.
    pub chart: ChartId,
    /// The developed start parameter of the edge run in the chart.
    pub from: Param,
    /// The developed end parameter of the edge run in the chart.
    pub to: Param,
}

/// The certified class of an interior event of the arc (§14.3 condition 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnotClass {
    /// A transversal trim crossing: two 1-complex arcs cross in one chart,
    /// certified by the one-chart R9 residual.
    Crossing,
    /// An A2 cusp of the arc on the carrier.
    Cusp,
}

/// A certified interior event of the arc (§14.3 conditions 3/4/6): the chart
/// parameter of the event, its class, and the point certificate that certified
/// it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InteriorEvent {
    /// The chart parameter of the event.
    pub at: Param,
    /// The certified class of the event.
    pub class: KnotClass,
    /// The point certificate of the event.
    pub cert: PointCert,
}

/// An interior knot of the promoted edge: a certified crossing/cusp event with
/// its multiplicity set from its class (condition 6).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeKnot {
    /// The chart parameter of the knot.
    pub at: Param,
    /// The certified class of the knot.
    pub class: KnotClass,
    /// The point certificate of the knot.
    pub cert: PointCert,
    /// The knot multiplicity set from the certified class (crossing 2,
    /// cusp 3).
    pub multiplicity: usize,
}

/// One row of the exported arclength position table: a cumulative arclength
/// value `s` and the model-space position `p` at the polyline vertex it names.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArclengthRow {
    /// The cumulative arclength at the vertex.
    pub s: f64,
    /// The model-space position of the vertex.
    pub p: [f64; 3],
}

/// The exported arclength parameterization of the promoted edge (§14.3
/// condition 7): the total length and the position table over the model-space
/// approximant's vertices.
#[derive(Debug, Clone, PartialEq)]
pub struct ArclengthParam {
    /// The total model-space length of the edge.
    pub total: f64,
    /// The position table: one row per approximant vertex in traversal order,
    /// `table[0].s == 0.0` at the start and `table[last].p` at the end.
    pub table: Vec<ArclengthRow>,
}

/// The §14.3 promotion context: everything the eight conditions need beyond the
/// two certified ends of the [`ChainArc`] data statement.
///
/// The [`ChainArc`] deliberately stores only its certified chart ends (§14.2);
/// the assembled-arc data the record restates — the model-space Hermite
/// approximant, the owning-face charts, the shared end nodes, and the certified
/// interior events — travels in the context.
#[derive(Debug, Clone)]
pub struct PromoContext {
    /// The chart period of the arc's chart family (deck arithmetic).
    pub period: f64,
    /// The §4.2 union certificates consumed by the landed A4.2
    /// [`regions_identify`].
    pub unions: Vec<(ResidualId, PointCert)>,
    /// The model-space Hermite approximant of the assembled arc; its first
    /// stored position equals `arc.start.point` and its last stored position
    /// equals `arc.end.point`.
    pub approx: HermiteSpline,
    /// The two owning-face chart ids; face `i` owns `pcurves[i]`.
    pub charts: [ChartId; 2],
    /// The shared C1 node each arc end resolves to, in start/end order.
    pub end_nodes: [SharedNode; 2],
    /// The certified interior events (crossings/cusps) of the arc, in
    /// developed order inside the run.
    pub interiors: Vec<InteriorEvent>,
    /// The typed §14.3 condition-8 opt-in (never a default-true bool).
    pub admit_tangent_at_tolerance: TangencyOptIn,
}

/// The §14.3 output record: a promoted model edge as a KERNEL RECORD, plain
/// data (`Debug` + `Clone`), deliberately NOT a live `truck_topology::Edge`
/// handle (the landed topology constructors panic in debug on circle-carried
/// self-loops; the record avoids the whole class).
#[derive(Debug, Clone, PartialEq)]
pub struct PromotedEdge {
    /// The id of the promoted chain arc.
    pub arc: ArcId,
    /// The model-space Hermite approximant of the edge.
    pub gamma: HermiteSpline,
    /// The owning-face chart ids; face `i` owns `pcurves[i]`.
    pub charts: [ChartId; 2],
    /// Both pcurves of the edge in their lifted charts.
    pub pcurves: [Pcurve; 2],
    /// The certified ends of the edge, each the shared C1 node its chain end
    /// resolves to.
    pub ends: [SharedEnd; 2],
    /// The interior knots of the edge (its crossings and cusps) with their
    /// set multiplicities.
    pub knots: Vec<EdgeKnot>,
    /// The exported arclength parameterization with its position table.
    pub arclength: ArclengthParam,
}

/// Promote an assembled chain arc to a model edge record, walking the eight
/// §14.3 conditions in order (§14.3 conditions 2, 3, 6, 7 and 8 are the
/// spec-numbered conditions; the run/deck ceiling, interior order, and endpoint
/// C1-certificate checks fill out the walk).
///
/// Each failing condition is a NAMED refusal carrying the evidence the spec
/// names for it. A near pair whose endpoints do not identify under any §4.2
/// rule is refused [`RefusalKind::SliverOrNearOverlap`] with both endpoints
/// carried verbatim — never snapped. A deck magnitude above
/// [`crate::kernel::config::DECK_MAX`] inside the promoted arc refuses
/// [`RefusalKind::DeckExhausted`]. An end node carrying the §10.3
/// tangency-at-tolerance tag refuses unless the context carries the explicit
/// typed opt-in.
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
pub fn promote(arc: &ChainArc, ctx: &PromoContext) -> Construction<PromotedEdge> {
    let start = arc.start;
    let end = arc.end;

    // Condition 1: run and deck ceiling.
    condition_run(arc, ctx)?;

    // Condition 2 (spec §14.3 condition 2): endpoint identity, by the landed
    // A4.2 rules — never proximity.
    condition_endpoint_identity(start, end, ctx)?;

    // Condition 3 (spec §14.3 condition 3): trim events in one chart route
    // through the landed R9 residual.
    condition_trim_events(ctx)?;

    // Condition 4: interior events are finite, certified C1 points strictly
    // inside the run, in order, distinct.
    condition_interior_order(arc, ctx)?;

    // Condition 5: the endpoint C1 certificates (the stored regions and the
    // shared end nodes).
    condition_endpoint_c1(start, end, ctx)?;

    // Condition 6 (spec §14.3 condition 6): knot multiplicities at crossings
    // and cusps, set from the certified class.
    let knots = build_knots(ctx);

    // Condition 7 (spec §14.3 condition 7): the exported arclength
    // parameterization and its position table.
    let arclength = build_arclength(&ctx.approx.segments)?;

    // Condition 8 (spec §14.3 condition 8): the tangency-at-tolerance gate.
    condition_tangency_gate(ctx)?;

    let charts = ctx.charts;
    let pcurves = [
        Pcurve {
            chart: charts[0],
            from: rebase(&start.at, charts[0]),
            to: rebase(&end.at, charts[0]),
        },
        Pcurve {
            chart: charts[1],
            from: rebase(&start.at, charts[1]),
            to: rebase(&end.at, charts[1]),
        },
    ];
    let ends = [
        SharedEnd {
            at: start.at,
            point: start.point,
            region: start.region,
            node: ctx.end_nodes[0].id,
            kind: ctx.end_nodes[0].kind,
            cert: ctx.end_nodes[0].cert,
        },
        SharedEnd {
            at: end.at,
            point: end.point,
            region: end.region,
            node: ctx.end_nodes[1].id,
            kind: ctx.end_nodes[1].kind,
            cert: ctx.end_nodes[1].cert,
        },
    ];
    Ok(PromotedEdge {
        arc: arc.id,
        gamma: ctx.approx.clone(),
        charts,
        pcurves,
        ends,
        knots,
        arclength,
    })
}

/// Condition 1: the arc is a single chart run with finite, canonical stored
/// ends, a deck magnitude inside [`DECK_MAX`] (the spec's termination bound
/// holds at promotion), a strictly positive developed run, and a model-space
/// approximant whose stored ends agree with the chain ends.
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
fn condition_run(arc: &ChainArc, ctx: &PromoContext) -> Result<(), Refusal> {
    let start = arc.start;
    let end = arc.end;
    if start.at.chart != end.at.chart {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "promote_arc_crosses_charts",
            format!(
                "arc {:?} spans charts {:?} and {:?}; a promoted arc is one chart run",
                arc.id, start.at.chart, end.at.chart
            ),
        ));
    }
    for (side, at) in [("start", &start.at), ("end", &end.at)] {
        if !at.u.is_finite() || !at.v.is_finite() {
            return Err(non_finite(
                "promote_end_param_not_finite",
                format!(
                    "the {side} chart parameter ({}, {}) is not finite",
                    at.u, at.v
                ),
            ));
        }
    }
    for (side, point) in [("start", start.point), ("end", end.point)] {
        if !point.iter().all(|c| c.is_finite()) {
            return Err(non_finite(
                "promote_end_point_not_finite",
                format!("the {side} recorded model point {point:?} is not finite"),
            ));
        }
    }
    if !ctx.period.is_finite() {
        return Err(non_finite(
            "promote_period_not_finite",
            format!("the chart period {} is not finite", ctx.period),
        ));
    }
    if ctx.period <= 0.0 && (start.at.deck != 0 || end.at.deck != 0) {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "promote_period_missing_for_deck",
            format!(
                "deck copies {:?}/{:?} need a positive chart period for the developed run",
                start.at.deck, end.at.deck
            ),
        ));
    }
    if ctx.period > 0.0 {
        for (side, at) in [("start", &start.at), ("end", &end.at)] {
            if !(0.0..ctx.period).contains(&at.u) {
                return Err(refusal(
                    RefusalKind::ClaimRefuted,
                    "promote_end_param_not_canonical",
                    format!(
                        "the {side} parameter u {} is not canonical in [0, {})",
                        at.u, ctx.period
                    ),
                ));
            }
        }
    }
    let start_mag = start.at.deck.unsigned_abs();
    let end_mag = end.at.deck.unsigned_abs();
    let span = (end.at.deck as i64 - start.at.deck as i64).unsigned_abs();
    let ceiling = DECK_MAX as u64;
    if span > ceiling || start_mag as u64 > ceiling || end_mag as u64 > ceiling {
        return Err(deck_exhausted(
            arc.id,
            span.max(start_mag as u64).max(end_mag as u64),
        ));
    }
    let raw_start = raw_u(start.at.deck, start.at.u, ctx.period)?;
    let raw_end = raw_u(end.at.deck, end.at.u, ctx.period)?;
    if raw_end == raw_start {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "promote_arc_zero_run",
            format!("arc {:?} spans a zero developed run", arc.id),
        ));
    }
    let segments = &ctx.approx.segments;
    if segments.is_empty() {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "promote_approx_empty",
            format!("the model-space approximant of arc {:?} is empty", arc.id),
        ));
    }
    for (index, segment) in segments.iter().enumerate() {
        for v in [segment.p0, segment.p1, segment.t0, segment.t1] {
            if !v.iter().all(|c| c.is_finite()) {
                return Err(non_finite(
                    "promote_approx_not_finite",
                    format!("approximant segment {index} {v:?} is not finite"),
                ));
            }
        }
    }
    let first = segments[0].p0;
    let last = segments[segments.len() - 1].p1;
    if first != start.point || last != end.point {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "promote_approx_endpoint_mismatch",
            format!(
                "the approximant ends {} -> {} disagree with the stored chain ends {} -> {}",
                fmt3(first),
                fmt3(last),
                fmt3(start.point),
                fmt3(end.point)
            ),
        ));
    }
    Ok(())
}

/// Condition 2 (spec §14.3 condition 2): an arc whose stored ends overlap in
/// model space within the representation gap must certify as one shared C1
/// node under the landed A4.2 rules ([`regions_identify`], with the landed Rule
/// B transport across deck copies) — never by proximity. A near pair no rule
/// identifies is refused [`RefusalKind::SliverOrNearOverlap`] with both
/// endpoints carried verbatim: refused, never snapped.
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
fn condition_endpoint_identity(
    start: ChainEnd,
    end: ChainEnd,
    ctx: &PromoContext,
) -> Result<(), Refusal> {
    let near = (0..3).all(|k| (start.point[k] - end.point[k]).abs() <= EPS_REP);
    if !near {
        return Ok(());
    }
    let transported = if start.at.deck == end.at.deck {
        start.region
    } else {
        let shift = end.at.deck - start.at.deck;
        rule_b_transport(&start.region, (shift, 0), (ctx.period, 0.0), None)?
    };
    match regions_identify(&transported, &end.region, &ctx.unions) {
        IdentityVerdict::CertifiedEqual { .. } => Ok(()),
        IdentityVerdict::NotCertified => Err(sliver_refusal(start, end)),
    }
}

/// Condition 3 (spec §14.3 condition 3): trim events in one chart route
/// through the landed R9 residual (`kernel::residuals_r89`). An interior trim
/// crossing whose certificate is not the one-chart R9 residual is refused.
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
fn condition_trim_events(ctx: &PromoContext) -> Result<(), Refusal> {
    for event in &ctx.interiors {
        if event.class == KnotClass::Crossing && event.cert.residual != ResidualId::R9 {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "promote_trim_crossing_not_r9_certified",
                format!(
                    "interior trim crossing at {:?} is certified at residual {:?}; §9.4 trim \
                     events route through the landed one-chart R9 residual",
                    event.at, event.cert.residual
                ),
            ));
        }
    }
    Ok(())
}

/// Condition 4: the interior events are finite, certified C1 points strictly
/// inside the arc's developed run, in traversal order and distinct.
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
fn condition_interior_order(arc: &ChainArc, ctx: &PromoContext) -> Result<(), Refusal> {
    if ctx.interiors.is_empty() {
        return Ok(());
    }
    let raw_start = raw_u(arc.start.at.deck, arc.start.at.u, ctx.period)?;
    let raw_end = raw_u(arc.end.at.deck, arc.end.at.u, ctx.period)?;
    let lo = raw_start.min(raw_end);
    let hi = raw_start.max(raw_end);
    let mut previous: Option<f64> = None;
    for (index, event) in ctx.interiors.iter().enumerate() {
        let raw = raw_u(event.at.deck, event.at.u, ctx.period)?;
        if raw <= lo || raw >= hi {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "promote_interior_outside_run",
                format!(
                    "interior event {index} at developed {} is outside the run ({}, {})",
                    raw, lo, hi
                ),
            ));
        }
        if let Some(prior) = previous {
            if raw <= prior {
                return Err(refusal(
                    RefusalKind::ClaimRefuted,
                    "promote_interior_not_ordered",
                    format!(
                        "interior event {index} at developed {raw} is not strictly after {prior}"
                    ),
                ));
            }
        }
        previous = Some(raw);
        c1_rho(&event.cert, &format!("interior event {index}"))?;
    }
    Ok(())
}

/// Condition 5: the endpoint C1 certificates — each stored endpoint region of
/// the arc and the certificate of each shared end node — are certified C1
/// (finite, contraction rate at or below [`RHO_MAX`]).
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
fn condition_endpoint_c1(
    start: ChainEnd,
    end: ChainEnd,
    ctx: &PromoContext,
) -> Result<(), Refusal> {
    for (label, region) in [("start", start.region), ("end", end.region)] {
        c1_rho(&region, &format!("{label} endpoint region"))?;
    }
    for (index, node) in ctx.end_nodes.iter().enumerate() {
        match node.cert {
            NodeCert::Exact(cert) => {
                c1_rho(&cert, &format!("shared end node {index}"))?;
            }
            NodeCert::AtTolerance(contact) => {
                c1_rho(
                    &contact.critical_point,
                    &format!("shared end node {index} (at tolerance)"),
                )?;
            }
        }
    }
    Ok(())
}

/// Condition 6 (spec §14.3 condition 6): knot multiplicities set at crossings
/// and cusps from the certified class (crossing 2, cusp 3).
fn build_knots(ctx: &PromoContext) -> Vec<EdgeKnot> {
    ctx.interiors
        .iter()
        .map(|event| EdgeKnot {
            at: event.at,
            class: event.class,
            cert: event.cert,
            multiplicity: knot_multiplicity(event.class),
        })
        .collect()
}

/// Condition 8 (spec §14.3 condition 8): an end node carrying the §10.3
/// tangency-at-tolerance tag (a [`NodeCert::AtTolerance`] certificate) refuses
/// the promotion UNLESS the context carries the explicit typed opt-in
/// [`TangencyOptIn::Admit`].
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
fn condition_tangency_gate(ctx: &PromoContext) -> Result<(), Refusal> {
    for (index, node) in ctx.end_nodes.iter().enumerate() {
        if matches!(node.cert, NodeCert::AtTolerance(_))
            && ctx.admit_tangent_at_tolerance != TangencyOptIn::Admit
        {
            return Err(refusal(
                RefusalKind::TangentialCurve,
                "promote_tangency_at_tolerance_requires_opt_in",
                format!(
                    "endpoint {index} (shared node {:?}) carries the §10.3 \
                     tangency-at-tolerance tag; promotion to a model edge requires the explicit \
                     PromoContext opt-in, not a default true",
                    node.id
                ),
            ));
        }
    }
    Ok(())
}

/// The §14.3 knot-multiplicity table (condition 6): a transversal trim crossing
/// joins two 1-complex branches in the chart (knot multiplicity 2); an A2 cusp
/// degenerates the tangent (knot multiplicity 3).
fn knot_multiplicity(class: KnotClass) -> usize {
    match class {
        KnotClass::Crossing => 2,
        KnotClass::Cusp => 3,
    }
}

/// Build the exported arclength parameterization and its position table over
/// the model-space approximant's vertices (condition 7). The table names each
/// vertex by its cumulative chord length; a zero-total-length edge is not a
/// model edge and is refused.
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
fn build_arclength(segments: &[HermiteSegment]) -> Result<ArclengthParam, Refusal> {
    let mut table = Vec::with_capacity(segments.len() + 1);
    table.push(ArclengthRow {
        s: 0.0,
        p: segments[0].p0,
    });
    let mut total = 0.0;
    for (index, segment) in segments.iter().enumerate() {
        let chord = chord_length(segment.p1, segment.p0);
        if !chord.is_finite() {
            return Err(non_finite(
                "promote_arclength_not_finite",
                format!("the chord of approximant segment {index} is not finite"),
            ));
        }
        total += chord;
        if !total.is_finite() {
            return Err(non_finite(
                "promote_arclength_not_finite",
                format!("the cumulative arclength at segment {index} is not finite"),
            ));
        }
        table.push(ArclengthRow {
            s: total,
            p: segment.p1,
        });
    }
    if total <= 0.0 {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "promote_edge_zero_length",
            format!("the model-space edge has zero total length {total}"),
        ));
    }
    Ok(ArclengthParam { total, table })
}

/// The developed (unwrapped) coordinate `deck * period + u`.
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
fn raw_u(deck: i32, u: f64, period: f64) -> Result<f64, Refusal> {
    let raw = deck as f64 * period + u;
    if !raw.is_finite() {
        return Err(non_finite(
            "promote_raw_not_finite",
            format!("developed coordinate for deck {deck} and u {u} is not finite"),
        ));
    }
    Ok(raw)
}

/// Validate that a point certificate is finite and certified C1 (contraction
/// rate at or below [`RHO_MAX`]).
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
fn c1_rho(cert: &PointCert, what: &str) -> Result<(), Refusal> {
    for (axis, bound) in cert.box_.lo.iter().chain(cert.box_.hi.iter()).enumerate() {
        if !bound.is_finite() {
            return Err(non_finite(
                "promote_cert_box_not_finite",
                format!("{what} certificate box bound {axis} is not finite"),
            ));
        }
    }
    if !cert.rho.is_finite() {
        return Err(non_finite(
            "promote_cert_rho_not_finite",
            format!("{what} certificate rho {} is not finite", cert.rho),
        ));
    }
    if cert.rho > RHO_MAX {
        return Err(refusal(
            RefusalKind::Conditioning,
            "promote_endpoint_not_c1",
            format!(
                "{what} certificate rho {} exceeds RHO_MAX {RHO_MAX}",
                cert.rho
            ),
        ));
    }
    Ok(())
}

/// The Euclidean chord length of a stored model segment.
fn chord_length(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d0 = a[0] - b[0];
    let d1 = a[1] - b[1];
    let d2 = a[2] - b[2];
    (d0 * d0 + d1 * d1 + d2 * d2).sqrt()
}

/// Re-state a parameter on a given owning-face chart (the decked coordinates
/// are chart-relative data; the run is recorded on each owning face).
fn rebase(at: &Param, chart: ChartId) -> Param {
    Param {
        chart,
        deck: at.deck,
        u: at.u,
        v: at.v,
    }
}

/// The SliverOrNearOverlap refusal for a near pair no §4.2 rule identifies:
/// both endpoints are carried verbatim — refused, never snapped.
fn sliver_refusal(start: ChainEnd, end: ChainEnd) -> Refusal {
    refusal(
        RefusalKind::SliverOrNearOverlap,
        "sliver_near_overlap_refuses_never_snap",
        format!(
            "the arc's stored ends overlap within the representation gap but do not identify \
             under any §4.2 rule; refused, never snapped. start endpoint carried verbatim: \
             {start:?}; end endpoint carried verbatim: {end:?}"
        ),
    )
}

/// The DeckExhausted refusal for a deck magnitude over the §0.4 ceiling inside
/// a promoted arc.
fn deck_exhausted(arc: ArcId, magnitude: u64) -> Refusal {
    refusal(
        RefusalKind::DeckExhausted,
        "promote_deck_max_exceeded",
        format!("|deck| {magnitude} exceeds DECK_MAX {DECK_MAX} inside the promoted arc {arc:?}"),
    )
}

/// A compact `[x, y, z]` spelling for refusal details.
fn fmt3(point: [f64; 3]) -> String {
    format!("[{}, {}, {}]", point[0], point[1], point[2])
}

/// A named predicate refusal: the kind names the violated class, the predicate
/// name the precise invariant.
fn refusal(kind: RefusalKind, name: &'static str, detail: String) -> Refusal {
    Refusal::new(kind, RefusalEvidence::Predicate { name, detail })
}

/// A named non-finite refusal.
fn non_finite(name: &'static str, detail: String) -> Refusal {
    refusal(RefusalKind::NonFinite, name, detail)
}
