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

//! The CC seam stub types (CC-000-CONTRACT, spine seams S6/S9/S10/S11/S12).
//!
//! This module freezes the shared shapes the later CC packets type against:
//! the admissible v1 [`RadiusLaw`] (theory §5.3), the [`EventKind`] event
//! vocabulary (theory §5.2), the S9 seam records [`WireComplex`],
//! [`ShiftFunctional`], and [`Correspondence`], the [`BoundaryPlan`] and
//! [`BranchSeed`] seam types, and the S11 [`TripleContactNode`] output
//! record.
//!
//! **C7 stub posture.** Seam types whose production has not landed carry
//! refusing constructors that always return
//! `Err(ConstructRefusal::Unfrozen)`: their production belongs to the named
//! wave packet (CC-013 for [`WireComplex`]/[`ShiftFunctional`] and the
//! [`Correspondence`] record, CC-005 for [`BoundaryPlan`], CC-030 for
//! [`BranchSeed`], CC-020 for the triple-contact solve that produces
//! [`TripleContactNode`] values). No production logic lives anywhere in this
//! file.
//!
//! **H-1.** This module carries no `unwrap`, no `expect`, and no `panic!`, and
//! adds no module-level `allow`.

use crate::construct::refusal::ConstructRefusal;
use crate::construct::Interval;

/// An admissible v1 canal radius law (theory §5.3), consumed by the S10 canal
/// regularity seam and by the S11/S12 contact and trace machinery.
#[derive(Debug, Clone, PartialEq)]
pub enum RadiusLaw {
    /// A constant radius along the whole spine arc.
    Constant(f64),
    /// A linear radius interpolation between the arc-end radii.
    Linear {
        /// The radius at the arc start.
        r0: f64,
        /// The radius at the arc end.
        r1: f64,
    },
    /// A cubic Hermite radius profile over the arc.
    CubicHermite {
        /// The radius at the arc start.
        r0: f64,
        /// The radius at the arc end.
        r1: f64,
        /// The radius slope at the arc start.
        m0: f64,
        /// The radius slope at the arc end.
        m1: f64,
    },
    /// A monotone cubic radius law through explicit `(station, radius)` pairs.
    MonotoneCubic(Vec<(f64, f64)>),
    /// A vertex-driven radius law over the control-vertex stations.
    VertexControl(Vec<f64>),
}

/// The S12 blend-event vocabulary (theory §5.2): one tag per kind of event a
/// blend trace may record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    /// A trim event: the blend chain clips against a trim loop.
    Trim,
    /// A third-face event: the chain meets a third support face.
    ThirdFace,
    /// A focal event on the chain.
    Focal,
    /// A rank event on the chain (rank-deficient step).
    Rank,
    /// A collision event with a non-support face.
    Collision,
    /// A plain trace continuation point.
    Trace,
}

/// The S9 abstract oriented cyclic wire complex (seam S9; theory §2.2 L4).
///
/// **S11 posture (CC-013 amendment, accepted at the packet's fifth
/// dispatch).** The CC-000 stub was opaque (`_sealed`, refusing constructor
/// only), so CC-013's wire production and correspondence resolver could not
/// build or read a single wire; the accepted QUESTION amended the shape to
/// this frozen PUB-field form, landed here by CC-013. The refusing
/// [`Self::try_new`] marker is kept so the CC-000 contract test stays green.
///
/// Production meaning (CC-013-CORRESPONDENCE): an oriented cyclic sequence of
/// `arc_count` matched edges. `vertices[i]` is the certified position
/// enclosure of the vertex that starts arc `i` and ends arc `i - 1` (a
/// cycle), so the vertex count always equals `arc_count`. A valid complex has
/// `arc_count >= 2`. Values are built with
/// [`wire_complex_of`](crate::construct::correspondence::wire_complex_of);
/// this module never splits an edge (edge splitting happens upstream).
#[derive(Debug, Clone, PartialEq)]
pub struct WireComplex {
    /// The number of matched edges (arcs) of the closed wire; at least 2.
    pub arc_count: usize,
    /// The per-vertex position enclosures, in cyclic order. The vertex count
    /// equals `arc_count`: it is a cycle.
    pub vertices: Vec<[Interval; 3]>,
}

impl WireComplex {
    /// The refusing stub-constructor marker (C7): kept so CC-000's contract
    /// test stays green; production belongs to CC-013's
    /// `wire_complex_of`.
    pub fn try_new() -> Result<Self, ConstructRefusal> {
        Err(ConstructRefusal::Unfrozen)
    }
}

/// The declared geometric shift-functional discriminant (seam S9).
///
/// The v1 functional set is closed at [`ShiftFunctionalKind::VertexSumSq`];
/// any other functional is a later CC-000 amendment, never a wave-worker
/// choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShiftFunctionalKind {
    /// The v1 declared functional: the sum of squared distances between
    /// matched vertices, accumulated in index order over interval
    /// arithmetic.
    VertexSumSq,
}

/// A caller-supplied correspondence anchor (seam S9, theory §2.2 L4).
///
/// When a functional carries an anchor, the resolver returns immediately
/// with it (resolution step 1) — the anchor is never second-guessed by the
/// geometric functional.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShiftAnchor {
    /// The anchor vertex index (into the wire's cyclic vertex sequence) that
    /// each section is aligned to.
    pub index: usize,
    /// Explicit caller consent for the orientation-reversing match. The
    /// automatic path is orientation-preserving only; a reversing match is
    /// taken exclusively when the caller supplied it here.
    pub reversed: bool,
}

/// The S9 declared geometric shift functional (seam S9; theory §2.2 L4).
///
/// **S11 posture (CC-013 amendment, accepted at the packet's fifth
/// dispatch).** The CC-000 stub was opaque (`_sealed`, refusing constructor
/// only); CC-013 lands this frozen PUB-field production shape. The refusing
/// [`Self::try_new`] marker is kept so the CC-000 contract test stays green.
///
/// A functional declares which geometric functional the step-3 argmin
/// evaluates over the `r` cyclic shifts ([`ShiftFunctionalKind::VertexSumSq`]
/// is the closed v1 set) together with an optional caller-supplied
/// [`ShiftAnchor`].
#[derive(Debug, Clone, PartialEq)]
pub struct ShiftFunctional {
    /// The declared geometric functional discriminant (closed v1 set).
    pub kind: ShiftFunctionalKind,
    /// The optional caller-supplied anchor (resolution step 1). When set, the
    /// resolver returns immediately with this anchor and never runs the
    /// declared functional.
    pub anchor: Option<ShiftAnchor>,
}

impl ShiftFunctional {
    /// The refusing stub-constructor marker (C7): kept so CC-000's contract
    /// test stays green; production values are assembled through the pub
    /// fields above.
    pub fn try_new() -> Result<Self, ConstructRefusal> {
        Err(ConstructRefusal::Unfrozen)
    }
}

/// The S9 correspondence record (seam S9; theory §2.2 L4).
///
/// A correspondence is an orientation, an anchor, and a cyclic edge matching:
/// `shifts[k]` is the cyclic shift that aligns section `k` to the wire. When
/// the caller supplied an anchor, `anchor` records it (`None` on the
/// automatic argmin path).
///
/// This carrier was missing from the crate entirely (accepted CC-013
/// QUESTION, fifth dispatch); its frozen shape is the S9 seam shape from the
/// construction-contracts document, landed here by CC-013. Values are
/// produced by
/// [`resolve_correspondence`](crate::construct::correspondence::resolve_correspondence).
#[derive(Debug, Clone, PartialEq)]
pub struct Correspondence {
    /// Whether the matching is orientation-preserving (`true`) or
    /// orientation-reversing (`false`). The automatic path is forward only;
    /// reversal is taken solely on explicit caller consent in the anchor.
    pub orientation: bool,
    /// The caller-supplied anchor vertex index, when one was supplied.
    pub anchor: Option<usize>,
    /// The per-section cyclic shift of the resolved matching, in `sections`
    /// order.
    pub shifts: Vec<usize>,
}

/// The S6 boundary-simplicity input plan.
///
/// **S11 posture (CC-005 dispatch amendment, session 51).** The original
/// CC-000 stub was uninhabitable (private `_sealed`, refusing constructor
/// only), so CC-005 could not consume a plan verdict across the seam. The
/// accepted QUESTION amended the shape to the S11 `TripleContactNode`
/// posture: frozen PUB fields landed here by CC-005 plus a refusing
/// `try_new()` (the contract test keeps asserting the refusing constructor).
/// Consumers — `certify_graph_disk` in `construct/graphdisk.rs` and the
/// projection search — read the verdict through these public fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundaryPlan {
    /// Whether the projected region boundary is certified simple.
    pub boundary_simple: bool,
    /// Whether every seam of the glued region is certified glued.
    pub seams_glued: bool,
}

impl BoundaryPlan {
    /// The refusing stub constructor (C7): the frozen shape is constructible
    /// through its public fields; `try_new` stays refusing as the C7 posture
    /// marker.
    pub fn try_new() -> Result<Self, ConstructRefusal> {
        Err(ConstructRefusal::Unfrozen)
    }
}

/// The S12 blend branch seed (stub posture C7).
///
/// Opaque: private fields only, constructible exclusively through the refusing
/// constructor until the CC-030 blend-trace packet lands its production.
#[derive(Debug, Clone)]
pub struct BranchSeed {
    /// Sealed. Production data is CC-030's design.
    _sealed: (),
}

impl BranchSeed {
    /// The refusing stub constructor (C7): production belongs to CC-030.
    pub fn try_new() -> Result<Self, ConstructRefusal> {
        Err(ConstructRefusal::Unfrozen)
    }
}

/// The S11 k=3 contact output record: one three-face contact node.
///
/// The field shape is frozen here (seam S11); values are produced by
/// `solve_triple_node` when CC-020 lands. The refusing constructor below is
/// the C7 stub posture until then.
#[derive(Debug, Clone, PartialEq)]
pub struct TripleContactNode {
    /// The certified centre enclosure of the contact node.
    pub centre: [Interval; 3],
    /// The certified radius enclosure of the contact node.
    pub radius: Interval,
    /// The certified per-contact parameter enclosures, one pair per support.
    pub contacts: [[Interval; 2]; 3],
}

impl TripleContactNode {
    /// The refusing stub constructor (C7): production belongs to CC-020.
    pub fn try_new(
        _centre: [Interval; 3],
        _radius: Interval,
        _contacts: [[Interval; 2]; 3],
    ) -> Result<Self, ConstructRefusal> {
        Err(ConstructRefusal::Unfrozen)
    }
}
