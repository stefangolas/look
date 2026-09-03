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

//! The §14/§16 certified topology: nodes, breaks, arcs, sheets, and the graph
//! containers (BG-KV2-000-CONTRACT).
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module. The module carries no `unwrap`, no `expect`, and no `panic!`
//! calls, and adds no module-level `allow`.
//!
//! **D-shim.** Topology shapes only; no tracing body. Per the §16 audit
//! "Refuse must not appear in `TopoNode`", NEITHER topology enum carries a
//! `Refuse` variant — a topology node/segment is a certified fact, never a
//! refusal (the integration contract test pins both exhaustive variant lists).
//!
//! **D-spelling.** `Arc` shadows `std::sync::Arc` module-locally (accepted,
//! per the spelling decision). [`NodeCert`] types §2 rule 7's never-unify: an
//! exact point certificate and an at-tolerance contact certificate are
//! different variants and never unify.
//!
//! **D6/§15.** `ClaimedGraph` and `CertifiedGraph` never unify: there is no
//! `From<ClaimedGraph> for CertifiedGraph`, ever.

use crate::kernel::certs::{ArcCert, ContactCert, PointCert, SheetCert, TubeOverlapCert};
use crate::kernel::evidence::{Refusal, RefusalEvidence, RefusalKind};
use crate::kernel::leaf::RationalCarrierKind;
use crate::kernel::patch::IBox2;

/// The identity of a chart in the certified graph (§16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChartId(pub u32);

/// A pcurve parameter: which chart, which deck (period crossing), and the
/// canonical `(u, v)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Param {
    /// The chart the parameter lives on.
    pub chart: ChartId,
    /// The deck (period-crossing index).
    pub deck: i32,
    /// The canonical `u` parameter.
    pub u: f64,
    /// The canonical `v` parameter.
    pub v: f64,
}

/// A point of the certified graph in the two-sheet parameter space: one
/// parameter on each of the two charts the graph ties together.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point4 {
    /// The parameter on the first chart.
    pub p1: Param,
    /// The parameter on the second chart.
    pub p2: Param,
}

/// The identity of a topology node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub usize);

/// The identity of a segment break.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BreakId(pub usize);

/// The identity of an arc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArcId(pub usize);

/// A certified topology node kind (§16): no `Refuse` variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopoNode {
    /// A boundary node (the graph meets a sheet boundary).
    Boundary,
    /// A transversal trim crossing.
    TrimCrossing,
    /// A Morse saddle.
    MorseSaddle,
    /// A Morse extremum.
    MorseExtremum,
    /// An A2 cusp.
    A2Cusp,
    /// An overlap-boundary node.
    OverlapBoundary,
    /// A fillet end.
    FilletEnd,
}

/// A certified segment-break kind (§16): no `Refuse` variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentBreak {
    /// The chart switches.
    ChartSwitch,
    /// The frame switches.
    FrameSwitch,
    /// A leaf boundary is crossed.
    LeafBoundary,
    /// The deck steps.
    DeckStep,
    /// An R6 chart switch.
    R6ChartSwitch,
    /// An R6 base swap.
    R6BaseSwap,
}

/// One endpoint of an arc: either a topology node or a segment break.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArcEnd {
    /// The arc ends at a topology node.
    Topo(NodeId),
    /// The arc ends at a segment break.
    Seg(BreakId),
}

/// The certificate of a node: §2 rule 7's never-unify, typed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeCert {
    /// The node is certified exactly (a certified point).
    Exact(PointCert),
    /// The node is certified only at tolerance (a contact certificate).
    AtTolerance(ContactCert),
}

/// A certified topology node: an id, a parameter point, a kind, and a
/// certificate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Node {
    /// The node id.
    pub id: NodeId,
    /// The node's parameter point.
    pub at: Point4,
    /// The node kind.
    pub kind: TopoNode,
    /// The node certificate.
    pub cert: NodeCert,
}

/// A certified segment break: an id, a parameter point, a kind, and the tube
/// overlap certificate that grounds the break.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Break {
    /// The break id.
    pub id: BreakId,
    /// The break's parameter point.
    pub at: Point4,
    /// The break kind.
    pub kind: SegmentBreak,
    /// The tube overlap certificate grounding the break.
    pub overlap: TubeOverlapCert,
}

/// One Hermite segment of a polyline approximation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HermiteSegment {
    /// The segment start point.
    pub p0: [f64; 3],
    /// The segment end point.
    pub p1: [f64; 3],
    /// The tangent at `p0`.
    pub t0: [f64; 3],
    /// The tangent at `p1`.
    pub t1: [f64; 3],
}

/// A Hermite polyline: consecutive segments.
///
/// Construct only through [`HermiteSpline::try_new`], which refuses an empty
/// segment list or any non-finite segment data.
#[derive(Debug, Clone, PartialEq)]
pub struct HermiteSpline {
    /// The consecutive Hermite segments.
    pub segments: Vec<HermiteSegment>,
}

/// A polyline approximation of an arc.
#[derive(Debug, Clone, PartialEq)]
pub struct Approx {
    /// The approximating Hermite spline.
    pub gamma: HermiteSpline,
}

/// A certified arc (tube certificate at dimension `N`).
#[derive(Debug, Clone, PartialEq)]
pub struct Arc<const N: usize> {
    /// The arc id.
    pub id: ArcId,
    /// The polyline approximation.
    pub approx: Approx,
    /// The tube certificate.
    pub cert: ArcCert<N>,
    /// The two certified endpoints.
    pub ends: (ArcEnd, ArcEnd),
}

/// A carrier arc: an arc that lies on a rational carrier.
#[derive(Debug, Clone, PartialEq)]
pub struct CarrierArc {
    /// The carrier arc id.
    pub id: ArcId,
    /// Which carrier family the arc lies on.
    pub carrier: RationalCarrierKind,
    /// The polyline approximation.
    pub approx: Approx,
}

/// Any arc of the certified graph: ordinary, difference, self-intersection,
/// spine, or carrier.
#[derive(Debug, Clone, PartialEq)]
pub enum AnyArc {
    /// An ordinary surface-intersection arc.
    Ordinary(Arc<4>),
    /// A difference arc (in the plane).
    Difference(Arc<2>),
    /// A self-intersection arc.
    SelfInt(Arc<4>),
    /// A spine arc.
    Spine(Arc<7>),
    /// A carrier arc.
    Carrier(CarrierArc),
}

/// A sheet of the certified graph (an exact-surface overlap region).
#[derive(Debug, Clone, PartialEq)]
pub struct Sheet {
    /// The sheet domain box.
    pub domain: IBox2,
    /// The parameter-map kind (§16's recorded spelling deviation).
    pub psi_kind: crate::kernel::certs::PsiMapKind,
    /// The sheet certificate.
    pub cert: SheetCert,
    /// The arcs on the sheet boundary.
    pub boundary: Vec<ArcId>,
}

/// The provenance of a graph (D6): a provenance tag is not a certificate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provenance {
    /// The graph was produced by this kernel's own certification.
    Claimed,
    /// The graph was imported from an external representation.
    Imported,
    /// The graph was supplied by the client.
    Client,
}

/// A fully certified graph: nodes, breaks, arcs, sheets, and the exhaustiveness
/// flag.
#[derive(Debug, Clone, PartialEq)]
pub struct CertifiedGraph {
    /// The certified nodes.
    pub nodes: Vec<Node>,
    /// The certified segment breaks.
    pub breaks: Vec<Break>,
    /// The certified arcs.
    pub arcs: Vec<AnyArc>,
    /// The certified sheets.
    pub sheets: Vec<Sheet>,
    /// Whether the graph is certified exhaustive.
    pub exhaustive: bool,
}

/// A claimed (not yet certified) graph and its provenance.
///
/// Never unifies with [`CertifiedGraph`] (D6/§15): there is no
/// `From<ClaimedGraph> for CertifiedGraph`.
#[derive(Debug, Clone, PartialEq)]
pub struct ClaimedGraph {
    /// The claimed graph contents.
    pub graph: CertifiedGraph,
    /// The provenance of the claim.
    pub provenance: Provenance,
}

/// The partial graph a mid-build [`crate::kernel::evidence::Refusal`] carries:
/// the graph assembled so far plus the open frontier of parameter points.
#[derive(Debug, Clone, PartialEq)]
pub struct PartialGraph {
    /// The graph assembled before the refusal.
    pub graph: CertifiedGraph,
    /// The frontier of yet-unprocessed points.
    pub frontier: Vec<Point4>,
}

impl Param {
    /// Build a parameter, refusing non-finite `u`/`v`.
    pub fn try_new(chart: ChartId, deck: i32, u: f64, v: f64) -> Result<Self, Refusal> {
        if !u.is_finite() || !v.is_finite() {
            return Err(refusal(
                RefusalKind::NonFinite,
                "param_not_finite",
                format!("param (u {u}, v {v}) is not finite"),
            ));
        }
        Ok(Self { chart, deck, u, v })
    }
}

impl HermiteSpline {
    /// Build a spline, refusing an empty segment list or non-finite segment
    /// data.
    pub fn try_new(segments: Vec<HermiteSegment>) -> Result<Self, Refusal> {
        if segments.is_empty() {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "hermite_spline_empty",
                "a Hermite spline needs at least one segment".to_string(),
            ));
        }
        for (i, s) in segments.iter().enumerate() {
            for v in [s.p0, s.p1, s.t0, s.t1] {
                if !v.iter().all(|c| c.is_finite()) {
                    return Err(refusal(
                        RefusalKind::NonFinite,
                        "hermite_segment_not_finite",
                        format!("segment {i} {v:?} is not finite"),
                    ));
                }
            }
        }
        Ok(Self { segments })
    }
}

/// A named predicate refusal: the kind names the violated class, the predicate
/// name the precise invariant.
fn refusal(kind: RefusalKind, name: &'static str, detail: String) -> Refusal {
    Refusal::new(kind, RefusalEvidence::Predicate { name, detail })
}
