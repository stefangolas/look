//! Generic contact and event contracts: the surface ARR-003 consumes.
//!
//! ARR-002's [`super::intersection::PairIntersectionResult`] is pairwise —
//! exactly two curve incidences per result, isolated points only, overlap a
//! bare tag. GEN-001 lifts these into a family-independent model:
//!
//! - an isolated event carries an **arbitrary number** of incident branches
//!   ([`IsolatedEvent2`]), so a genuine triple event is one event, not three
//!   pairwise pseudo-events;
//! - contact is either an isolated event or a positive-dimensional common arc
//!   ([`ContactComponent2`]);
//! - identity is construction- and provenance-based ([`EventIdentity`]), never
//!   coordinate proximity.
//!
//! GEN-001A provides the types and a faithful adapter ([`lift_pair_result`])
//! from the existing pairwise result. The adapter turns each ARR-002 isolated
//! intersection into one two-branch isolated event under a stable identity; it
//! does **not** merge pairwise results into shared multi-branch events. That
//! merge is ARR-003's responsibility, and it uses the stable identities
//! introduced here.
//!
//! GEN-001D adds the rank/deck contract to the event layer: each
//! [`IsolatedEvent2`] carries one shared ambient [`DeckContext`] (bound to one
//! certified lattice) and each [`BranchIncidence`] carries a validated,
//! lattice-bound [`CertifiedDeckLabel`]. Event deck identity is canonical and
//! **relative**: [`IsolatedEvent2::deck_signature`] normalizes the gauge by
//! ordering the incidences canonically ([`CanonicalIncidenceId`] — source
//! occurrence + canonical branch side, never parameter-enclosure bits) and
//! subtracting the canonical anchor label. [`IsolatedEvent2::pair_contact_lift_key`]
//! pairs that with the pair/lift construction identity; it is explicitly **not**
//! a final aggregate vertex identity — ARR-003 certifies the complete event
//! equivalence class and produces an [`AggregatedQuotientEventKey`]. Absolute
//! lifts are gauge-dependent and never determine event identity; the existing
//! rank-1 solver's verdicts are adapted by `adapt_axis_aligned_placement` and
//! attached end to end by [`label_branch_from_placement`] without any
//! nearest-integer rounding. Rank-2 *labels* are carried and validated; rank-2
//! geometric *placement* stays a typed [`DeckPlacementResult::Unsupported`] (no
//! unreviewed closest-vector algorithm is invented).
//!
//! # A7 — closed in GEN-001E by certified parameter membership
//!
//! ARR-002's chord-side membership test read rounded evaluated arc endpoints
//! (`arc.start.point()` / `arc.end.point()`); see `GEN-001.md` assumption A7.
//! GEN-001E closes A7 in the analytic fast path: arc-piece membership is
//! decided by **certified parameter evidence** — the root's certified
//! arc-parameter enclosure, lifted onto the piece's authoritative unwrapped
//! axis by an enumeration of every compatible `2π` copy, and compared against
//! the piece's authoritative endpoint parameters (source trim values or
//! certified critical enclosures). No rounded endpoint coordinate and no
//! midpoint-selected `2π` copy decides topology. Every incident branch still
//! carries a certified [`super::span::BranchGerm`], and ARR-003 consumes
//! germs and parameter enclosures — never representative coordinates.

use super::super::source_evidence::{EdgeUseId, SourceVertexKey};
use super::curve2d::CurveOccurrenceProvenance;
use super::evidence::NonEmptyVec;
use super::intersection::{
    CertifiedIntersection2, ContactKind, IntersectionIdentity, PairIntersectionResult,
    PairUnresolved, PairUnsupported, ParameterEnclosure, ParameterLocation,
};
use super::quotient::{
    CanonicalBranchSide, CanonicalIncidenceId, CertifiedDeckLabel, DeckContext, DeckLabel,
    DeckLabelError, DeckPlacementResult, DeckSignature,
};
use super::span::{BranchGerm, CurveSpan2, SpanId};
// The certified CommonArc substrate (GEN-001E) lives in [`super::common_arc`];
// `contact` re-exports it so the arrangement-facing contact surface stays here.
pub use super::common_arc::{
    ArcParticipant, CommonArc2, CommonArcEnd, CommonSupportBasis, OrientationAlongSupport,
};
use truck_geometry::prelude::Point2;

/// Why a generic contact computation could not be certified.
///
/// Frozen in GEN-001A so the later phases (B: Bézier isolation, C: germs, E:
/// common arcs) populate the variants without changing the ARR-003-facing data
/// model. A tolerance may produce one of these; it may never create topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericUnresolved {
    /// Clustered roots the isolation cascade cannot separate at this precision.
    ClusteredRoots,
    /// An inseparable multiple (high-multiplicity) root.
    InseparableMultipleRoots,
    /// The isolation Jacobian is singular at the candidate root.
    SingularJacobian,
    /// A singular branch whose local topology is not yet classified.
    UnsupportedSingularBranch,
    /// A positive-dimensional common component beyond the minimal supported cases.
    GeneralCommonComponent,
    /// A Bézier root lying exactly on a parameter-domain boundary, where no
    /// endpoint certificate has been implemented yet (GEN-001C endpoint policy).
    UnresolvedBoundaryRoot,
    /// A certified root whose tangent-numerator enclosure could not certify a
    /// nonzero first derivative: a stationary (derivative-zero) candidate.
    /// Uncertainty here is never promoted to an ordinary tangent.
    UnresolvedStationaryBranch,
    /// A certified root whose transverse determinant could not be certified
    /// nonzero: a possible tangency or singularity, not a regular crossing.
    UnresolvedTangencyOrSingularity,
    /// The pair-local root ordinals could not be backed by a certified total
    /// order: two certified roots have overlapping source-parameter enclosures
    /// on both participants, so no interval-separation order exists. The roots
    /// are never ordered by traversal or discovery order.
    UnresolvedIdentityOrdering,
    /// A derived (subdivided) piece's root could not be matched to exactly one
    /// root of the parent (unsplit) pair isolation. A sub-span cannot acquire a
    /// geometric intersection its parent lacks, so an unmatched or ambiguous
    /// match is `Unresolved`, never a new root identity.
    UnresolvedIdentityReferBack,
    /// A reused ARR-002 pairwise cause.
    Pair(PairUnresolved),
    /// A positive-dimensional overlap whose certified CommonArc construction
    /// failed — the overlap extent or parameter correspondence could not be
    /// certified (GEN-001E). Never a guessed common arc.
    UnresolvedCommonArc,
}

impl GenericUnresolved {
    /// A short stable tag, for diagnostics.
    pub fn tag(self) -> &'static str {
        match self {
            Self::ClusteredRoots => "unresolved_clustered_roots",
            Self::InseparableMultipleRoots => "unresolved_inseparable_multiple_roots",
            Self::SingularJacobian => "unresolved_singular_jacobian",
            Self::UnsupportedSingularBranch => "unresolved_unsupported_singular_branch",
            Self::GeneralCommonComponent => "unresolved_general_common_component",
            Self::UnresolvedBoundaryRoot => "unresolved_boundary_root",
            Self::UnresolvedStationaryBranch => "unresolved_stationary_branch",
            Self::UnresolvedTangencyOrSingularity => "unresolved_tangency_or_singularity",
            Self::UnresolvedIdentityOrdering => "unresolved_identity_ordering",
            Self::UnresolvedIdentityReferBack => "unresolved_identity_refer_back",
            Self::UnresolvedCommonArc => "unresolved_common_arc",
            Self::Pair(_) => "unresolved_pair",
        }
    }
}

/// How two regular branches cross at an isolated event.
///
/// For a two-branch event this is the ARR-002 `ContactKind` lifted intact. For
/// a merged high-multiplicity event ARR-003 derives the classification from the
/// [`BranchGerm`] configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossingClassification {
    /// The vertical order of the two branches swaps across the event.
    Transverse,
    /// The vertical order is preserved (a tangential contact).
    Tangent,
    /// A stationary/singular contact not reducible to transverse/tangent.
    Stationary,
}

impl CrossingClassification {
    /// Lift an ARR-002 pairwise contact kind.
    pub fn from_pair(contact: ContactKind) -> Self {
        match contact {
            ContactKind::Transverse => Self::Transverse,
            ContactKind::Tangent => Self::Tangent,
        }
    }
}

/// One participant's occurrence identity within an isolated-root key.
///
/// Reversal-stable: [`CurveOccurrenceProvenance::reversed`] swaps only the
/// traversal start/end vertices and preserves the edge-use and source-edge ids,
/// so reparameterizing the same occurrence keeps this identity while reversing
/// only the branch-incidence orientation. A different twin edge use is a
/// distinct B-rep incidence with a distinct span id and is never conflated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IsolatedRootParticipant {
    /// The stable span/occurrence identity.
    pub span_id: SpanId,
}

/// The canonical identity of one isolated root of a curve pair.
///
/// Construction- and provenance-based, never coordinate proximity: the two
/// participant occurrences (sorted) plus a deterministic pair-local root
/// ordinal. See [`EventIdentity::IsolatedRoot`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IsolatedRootKey {
    /// The two participant occurrences, sorted canonically.
    pub participants: [IsolatedRootParticipant; 2],
    /// The deterministic pair-local root ordinal.
    pub ordinal: u32,
}

impl IsolatedRootKey {
    /// The canonical key for the given pair of span occurrences and ordinal.
    ///
    /// The participants are sorted, so an operand swap re-sorts to the same
    /// pair. Reversal does not change the span ids ([`CurveOccurrenceProvenance::reversed`]
    /// preserves them), so reparameterization reversal keeps the same pair.
    pub fn new(first_span_id: SpanId, second_span_id: SpanId, ordinal: u32) -> Self {
        let mut participants = [
            IsolatedRootParticipant {
                span_id: first_span_id,
            },
            IsolatedRootParticipant {
                span_id: second_span_id,
            },
        ];
        participants.sort();
        IsolatedRootKey {
            participants,
            ordinal,
        }
    }
}

/// The stable identity of one arrangement event, for deduplication and merging.
///
/// Based on construction and provenance, never on coordinate proximity. Two
/// pairwise intersections that share a source vertex produce the same
/// [`EventIdentity::SharedSourceVertex`], so ARR-003 can merge them into one
/// multi-branch event. A representative `Point2<f64>` may be stored elsewhere
/// for rendering; it must not define equality or ordering identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EventIdentity {
    /// A physical source vertex shared by incident branches.
    SharedSourceVertex(SourceVertexKey),
    /// An artificial monotone-critical split shared by incident branches.
    SharedMonotoneCritical {
        /// The edge use the split belongs to.
        edge_use_id: EdgeUseId,
        /// The split's critical index.
        critical_index: i64,
    },
    /// One isolated certified root of a curve pair.
    ///
    /// **Canonical (GEN-001C).** The key is construction data, never
    /// coordinates: the two participant occurrences sorted canonically plus a
    /// deterministic pair-local root ordinal. The ordinal is assigned by one
    /// canonical isolation run, ordering the pair's certified roots by interval
    /// separation of their orientation-normalized authoritative source
    /// parameter boxes. It is stable under operand swap, reversal of either
    /// span, reversal of both spans, and deterministic repetition. Certified
    /// parameter boxes are evidence carried on the incident branches, never part
    /// of this identity.
    IsolatedRoot(IsolatedRootKey),
    /// A merged high-multiplicity event (≥3 branches), keyed by a stable
    /// construction id assigned by ARR-003 when it merges.
    MergedHighMultiplicity {
        /// A stable id for the merged event.
        construction_id: u64,
    },
    /// An endpoint of a common arc.
    CommonArcEndpoint {
        /// The common arc's stable id.
        arc_id: u64,
        /// Which end.
        end: CommonArcEnd,
    },
    /// An event identified with another by a certified deck translation.
    DeckTranslated {
        /// A stable id for the identified event.
        construction_id: u64,
        /// The deck displacement identifying the copy.
        deck: DeckLabel,
    },
    /// A collapsed quotient stratum (pole/apex). Representable in GEN-001A;
    /// populated only once the evidence seam certifies strata.
    CollapsedStratum {
        /// The collapsed stratum's stable id.
        stratum_id: u64,
    },
}

impl EventIdentity {
    /// Map an ARR-002 pairwise intersection identity to the generic identity.
    ///
    /// Two pairwise intersections sharing a source vertex map to the same
    /// [`EventIdentity::SharedSourceVertex`]; that is the handle ARR-003 merges
    /// on. An isolated curve-curve intersection maps to the canonical
    /// [`EventIdentity::IsolatedRoot`] keyed by the two span occurrences and
    /// the pair-local ordinal assigned by the caller (see [`lift_pair_result`]:
    /// the pair's roots are ordered by interval separation of their certified
    /// source-parameter boxes, never by discovery order or a per-curve index).
    pub fn from_pair(
        identity: &IntersectionIdentity,
        lhs_span_id: SpanId,
        rhs_span_id: SpanId,
        ordinal: u32,
    ) -> Self {
        match identity {
            IntersectionIdentity::SourceVertex(vertex) => Self::SharedSourceVertex(*vertex),
            IntersectionIdentity::ArtificialMonotoneSplit {
                edge_use_id,
                critical_index,
            } => Self::SharedMonotoneCritical {
                edge_use_id: *edge_use_id,
                critical_index: *critical_index,
            },
            IntersectionIdentity::CurveIntersection { .. } => {
                Self::IsolatedRoot(IsolatedRootKey::new(lhs_span_id, rhs_span_id, ordinal))
            }
        }
    }
}

/// One curve's involvement in one contact event.
///
/// The arrangement-facing atom: a span, the certified parameter enclosure at
/// which it meets the event, its certified local germ, its canonical branch
/// side, its validated deck label, and its provenance. The representative point
/// is an evaluation hint, never identity (A7: ARR-003 consumes the germ and the
/// parameter enclosure, not the representative coordinate).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BranchIncidence {
    /// The span this branch is a piece of.
    pub span_id: SpanId,
    /// The source occurrence provenance.
    pub provenance: CurveOccurrenceProvenance,
    /// Certified enclosure of the meeting parameter on this branch's
    /// authoritative domain.
    pub parameter: ParameterEnclosure,
    /// Where on the piece the meeting lies.
    pub location: ParameterLocation,
    /// The certified local branch behavior at the meeting parameter.
    pub germ: BranchGerm,
    /// The canonical participant slot of this branch within its event's root.
    ///
    /// Assigned by the producer from the canonical sorted participant pair
    /// ([`IsolatedRootKey`]), never from insertion order, discovery order or a
    /// coordinate. It is part of the canonical incidence identity
    /// ([`CanonicalIncidenceId`]) used for deck gauge anchor selection, so the
    /// anchor is stable under operand swap, source traversal reversal and
    /// subdivision. Parameter enclosures are evidence, not the ordering key.
    pub side: CanonicalBranchSide,
    /// The validated deck label of this branch's chart copy, bound to the
    /// event's shared ambient context ([`IsolatedEvent2::deck_context`]).
    pub deck: CertifiedDeckLabel,
    /// A representative point (evaluation hint), never identity.
    pub representative: Point2,
}

/// An isolated contact event with an arbitrary number of incident branches.
#[derive(Debug, Clone, PartialEq)]
pub struct IsolatedEvent2 {
    /// The stable identity for deduplication and merging.
    pub identity: EventIdentity,
    /// How the branches cross. For a two-branch GEN-001A lift this is the
    /// ARR-002 contact kind; for a merged event ARR-003 derives it from the
    /// [`BranchGerm`] configuration.
    pub crossing: CrossingClassification,
    /// The incident branches. Two for an ARR-002 pairwise lift; ≥3 after
    /// ARR-003 merges by identity.
    pub branches: Vec<BranchIncidence>,
    /// The shared ambient lattice/rank context for this event's incidences.
    ///
    /// Carried once per event, never duplicated per branch (GEN-001D): the
    /// context is bound to one certified lattice, and every branch label must
    /// validate against it. A mismatch is a typed [`DeckLabelError`] from
    /// [`IsolatedEvent2::deck_signature`].
    pub deck_context: DeckContext,
    /// A representative point (evaluation hint), never identity.
    pub representative: Point2,
}

impl IsolatedEvent2 {
    /// The canonical *relative* deck signature of this isolated event.
    ///
    /// Gauge normalization (GEN-001D): every branch label is first validated
    /// against the event's shared ambient context, the incidences are ordered
    /// canonically by construction-based incidence identity
    /// ([`CanonicalIncidenceId`]: source occurrence + canonical branch side —
    /// never parameter-enclosure bits, the representative point, insertion
    /// order, discovery order or a rounded coordinate), the anchor is the
    /// canonical minimum, and the anchor label is subtracted from every label.
    /// The resulting normalized relative labels are returned in canonical
    /// incidence order as a [`DeckSignature`].
    ///
    /// The signature is invariant under input permutation, operand swap, source
    /// traversal reversal, deterministic repetition, and adding one deck vector
    /// to every incidence (a common deck translation represents the same
    /// quotient event). Absolute lifts are gauge-dependent, so they never
    /// determine event identity — the normalized signature does.
    pub fn deck_signature(&self) -> Result<DeckSignature, DeckLabelError> {
        let entries: Vec<(CanonicalIncidenceId, CertifiedDeckLabel)> = self
            .branches
            .iter()
            .map(|branch| {
                (
                    CanonicalIncidenceId::new(branch.span_id, branch.side),
                    branch.deck,
                )
            })
            .collect();
        DeckSignature::normalize(self.deck_context, &entries)
    }

    /// The pair/lift-level quotient-event key: the construction identity of the
    /// pair contact plus the normalized relative deck signature.
    ///
    /// **Not a final aggregate vertex identity.** The [`EventIdentity`] is the
    /// pre-ARR-003 construction identity of one pair contact/root, so at a
    /// genuine three-way event the three pairwise records have three distinct
    /// keys even though they belong to one arrangement vertex. ARR-003 certifies
    /// the complete event equivalence class and produces the
    /// [`AggregatedQuotientEventKey`]. Raw absolute deck labels never appear in
    /// either key, because they are gauge-dependent.
    pub fn pair_contact_lift_key(&self) -> Result<PairContactLiftKey, DeckLabelError> {
        Ok(PairContactLiftKey {
            pair_contact: self.identity.clone(),
            deck_signature: self.deck_signature()?,
        })
    }
}

/// The key of one pair/lift contact record: its construction identity plus its
/// normalized relative deck signature.
///
/// This is what a single pairwise root contributes to the arrangement. It is
/// deliberately not called the event identity: at an N-way event the pairwise
/// records carry distinct [`EventIdentity`] values, and only ARR-003's
/// aggregation produces the vertex-level key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PairContactLiftKey {
    /// The construction identity of the pair contact.
    pub pair_contact: EventIdentity,
    /// The normalized relative deck signature.
    pub deck_signature: DeckSignature,
}

/// The canonical quotient-event key of an aggregated arrangement vertex.
///
/// Defined in GEN-001D for representability; **populated only by ARR-003**,
/// after it has certified the complete event equivalence class (all incidences
/// of a genuine high-multiplicity event under one vertex identity). The
/// incidences are the canonical incidence identities of every participating
/// branch, and the relative deck signature is the gauge-normalized label set of
/// the whole vertex.
#[derive(Debug, Clone, PartialEq)]
pub struct AggregatedQuotientEventKey {
    /// The canonical incidence identities of every branch in the equivalence
    /// class. Non-empty by construction (ARR-003 aggregates over ≥1 incidence).
    pub incidences: NonEmptyVec<CanonicalIncidenceId>,
    /// The gauge-normalized relative deck signature of the whole vertex.
    pub relative_deck_signature: DeckSignature,
}

/// One contact component: an isolated event or a common arc.
#[derive(Debug, Clone, PartialEq)]
pub enum ContactComponent2 {
    /// An isolated (0-dimensional) event.
    IsolatedEvent(IsolatedEvent2),
    /// A positive-dimensional common arc.
    CommonArc(CommonArc2),
}

/// The generic pair-contact result.
///
/// Mirrors ARR-002's [`PairIntersectionResult`] in generic terms. `Unsupported`
/// is reused verbatim so no formal diagnostic is lost; `Unresolved` is widened
/// to [`GenericUnresolved`] so the Bézier isolation phases can report their own
/// typed non-results without changing the data model.
#[derive(Debug, Clone, PartialEq)]
pub enum PairContactResult {
    /// No contact.
    Disjoint,
    /// One or more contact components.
    Components(Vec<ContactComponent2>),
    /// The contact is valid geometry outside the admitted envelope.
    Unsupported(PairUnsupported),
    /// The contact cannot be certified under the declared numerical policy.
    Unresolved(GenericUnresolved),
}

impl PairContactResult {
    /// A short stable tag, for diagnostics.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::Disjoint => "contact_disjoint",
            Self::Components(_) => "contact_components",
            Self::Unsupported(_) => "contact_unsupported",
            Self::Unresolved(_) => "contact_unresolved",
        }
    }
}

/// The canonical sides of a two-participant pair, from the sorted participant
/// identity.
///
/// Mirrors [`IsolatedRootKey::new`]: the branch whose span is the first sorted
/// participant is [`CanonicalBranchSide::First`]. Reversal preserves span ids,
/// so the sides are stable under operand swap and traversal reversal.
fn canonical_sides(
    first_span: SpanId,
    second_span: SpanId,
) -> (CanonicalBranchSide, CanonicalBranchSide) {
    if first_span <= second_span {
        (CanonicalBranchSide::First, CanonicalBranchSide::Second)
    } else {
        (CanonicalBranchSide::Second, CanonicalBranchSide::First)
    }
}

/// Build the two branch incidences of one ARR-002 pairwise intersection.
///
/// Every analytic branch is certified [`BranchGerm::Regular`]: a line/circle has
/// a nonzero first derivative everywhere on a nondegenerate piece. The deck
/// label is the validated rank-0 zero label of the rank-0 context: the analytic
/// pairwise path has no certified ambient context, and the event carries the
/// shared rank-0 context ([`IsolatedEvent2::deck_context`]).
fn branches_from_pair(
    lhs_span: &CurveSpan2,
    rhs_span: &CurveSpan2,
    intersection: &CertifiedIntersection2,
) -> [BranchIncidence; 2] {
    let (lhs_side, rhs_side) = canonical_sides(lhs_span.span_id(), rhs_span.span_id());
    let lhs = BranchIncidence {
        span_id: lhs_span.span_id(),
        provenance: *lhs_span.provenance(),
        parameter: intersection.lhs_parameter,
        location: intersection.lhs_location,
        germ: BranchGerm::Regular,
        side: lhs_side,
        deck: CertifiedDeckLabel::zero(DeckContext::rank0()),
        representative: intersection.point,
    };
    let rhs = BranchIncidence {
        span_id: rhs_span.span_id(),
        provenance: *rhs_span.provenance(),
        parameter: intersection.rhs_parameter,
        location: intersection.rhs_location,
        germ: BranchGerm::Regular,
        side: rhs_side,
        deck: CertifiedDeckLabel::zero(DeckContext::rank0()),
        representative: intersection.point,
    };
    [lhs, rhs]
}

/// Order a pair's intersections canonically for ordinal assignment.
///
/// The pair-local ordinal must be stable under operand swap and must not depend
/// on discovery order. Order by interval separation of the certified
/// source-parameter enclosures (lexicographic on the interval endpoints, which
/// agrees with separation order for disjoint certified roots). The rounded
/// midpoint is never used as an ordering key.
fn sort_intersections_canonically(intersections: &mut [CertifiedIntersection2]) {
    intersections.sort_by(|a, b| {
        let ka = (
            a.lhs_parameter.lo,
            a.lhs_parameter.hi,
            a.rhs_parameter.lo,
            a.rhs_parameter.hi,
        );
        let kb = (
            b.lhs_parameter.lo,
            b.lhs_parameter.hi,
            b.rhs_parameter.lo,
            b.rhs_parameter.hi,
        );
        ka.partial_cmp(&kb).unwrap_or(std::cmp::Ordering::Equal)
    });
}

/// Lift an ARR-002 pairwise result into the generic contact contracts.
///
/// Faithful: each isolated intersection becomes one [`IsolatedEvent2`] with two
/// [`BranchIncidence`] under the [`EventIdentity`] mapped from its
/// [`IntersectionIdentity`]. The pair's intersections are ordered canonically
/// before ordinals are assigned, so the identity of each isolated root is
/// stable under operand swap and does not depend on discovery order. `Disjoint`,
/// `Unsupported` and `Unresolved` pass through (the last wrapped in
/// [`GenericUnresolved::Pair`]). This does not merge pairwise results into
/// shared multi-branch events — that is ARR-003's job, using the stable
/// identities introduced here.
///
/// **GEN-001E (CommonArc).** A pairwise positive-length overlap (the analytic
/// `Overlap` / `CoincidentCircles` tags) is no longer a bare `Unsupported`: it
/// is first certified by [`super::common_arc::common_arc_for_pair`]. A
/// certified positive-dimensional overlap becomes one
/// [`ContactComponent2::CommonArc`]; a certified-empty overlap becomes
/// `Disjoint`; a point-only or uncertifiable overlap keeps the original typed
/// `Unsupported`. The germ and the certified parameter evidence — never a
/// representative point — are what ARR-003 reads.
pub fn lift_pair_result(
    lhs_span: &CurveSpan2,
    rhs_span: &CurveSpan2,
    result: &PairIntersectionResult,
) -> PairContactResult {
    match result {
        PairIntersectionResult::Disjoint => PairContactResult::Disjoint,
        PairIntersectionResult::Unsupported(
            cause @ (PairUnsupported::Overlap | PairUnsupported::CoincidentCircles),
        ) => match super::common_arc::common_arc_for_pair(lhs_span, rhs_span) {
            Ok(arc) => PairContactResult::Components(vec![ContactComponent2::CommonArc(arc)]),
            // A certified-empty overlap is no contact component.
            Err(super::common_arc::CommonArcError::EmptyOverlap) => PairContactResult::Disjoint,
            // The overlap is not certifiable as positive-dimensional (a
            // point-only meeting, or an uncertified support identity such as a
            // periodic seam overlap): keep the original typed Unsupported.
            Err(
                super::common_arc::CommonArcError::PointOnlyOverlap
                | super::common_arc::CommonArcError::UnsupportedSupportIdentity,
            ) => PairContactResult::Unsupported(*cause),
            // Any certification/operational failure of the CommonArc attempt is
            // a typed Unresolved, never an Unsupported and never a guess.
            Err(_) => PairContactResult::Unresolved(GenericUnresolved::UnresolvedCommonArc),
        },
        PairIntersectionResult::Unsupported(cause) => PairContactResult::Unsupported(*cause),
        PairIntersectionResult::Unresolved(cause) => {
            PairContactResult::Unresolved(GenericUnresolved::Pair(*cause))
        }
        PairIntersectionResult::Intersections(intersections) => {
            let mut intersections = intersections.clone();
            sort_intersections_canonically(&mut intersections);
            let lhs_span_id = lhs_span.span_id();
            let rhs_span_id = rhs_span.span_id();
            let components: Vec<ContactComponent2> = intersections
                .iter()
                .enumerate()
                .map(|(ordinal, intersection)| {
                    let [lhs_branch, rhs_branch] =
                        branches_from_pair(lhs_span, rhs_span, intersection);
                    ContactComponent2::IsolatedEvent(IsolatedEvent2 {
                        identity: EventIdentity::from_pair(
                            &intersection.identity,
                            lhs_span_id,
                            rhs_span_id,
                            ordinal as u32,
                        ),
                        crossing: CrossingClassification::from_pair(intersection.contact),
                        branches: vec![lhs_branch, rhs_branch],
                        deck_context: DeckContext::rank0(),
                        representative: intersection.point,
                    })
                })
                .collect();
            if components.is_empty() {
                PairContactResult::Disjoint
            } else {
                PairContactResult::Components(components)
            }
        }
    }
}

/// Attach a certified deck placement's unique label to one branch of an event.
///
/// The end-to-end rank-1 label path: the existing
/// [`super::deck::solve_axis_aligned`] verdict is adapted by
/// `adapt_axis_aligned_placement`, the unique label is validated against the
/// event's shared ambient context, and it is written onto `branch_index`. A
/// non-unique verdict is returned verbatim as
/// [`DeckLabelError::NonUniquePlacement`], so `Incompatible`, `Ambiguous`,
/// `Unresolved`, `Unsupported` and `OperationalFailure` stay distinct and no
/// near-integer rounding ever mints a label. An out-of-range `branch_index` is
/// a typed [`DeckLabelError::NoSuchBranch`], never a panic.
pub fn label_branch_from_placement(
    event: &mut IsolatedEvent2,
    branch_index: usize,
    placement: DeckPlacementResult,
) -> Result<(), DeckLabelError> {
    let branch = event
        .branches
        .get_mut(branch_index)
        .ok_or(DeckLabelError::NoSuchBranch {
            index: branch_index,
        })?;
    match placement {
        DeckPlacementResult::Unique(label) => {
            branch.deck = label.validate_for(event.deck_context)?;
            Ok(())
        }
        other => Err(DeckLabelError::NonUniquePlacement(other)),
    }
}

#[cfg(test)]
mod tests {
    use super::super::curve2d::{
        DevelopedCurve2D, LineSegment2, SourceEdgeId, SourceEntityId, SourceFaceId,
    };
    use super::super::deck::{
        solve_axis_aligned, DeckGenerator, DeckInterval, DevelopedAxis, DevelopedBox,
    };
    use super::super::evidence::ParameterAxis;
    use super::super::intersection::{intersect_x_monotone, IntersectionPolicy};
    use super::super::numeric::FiniteF64;
    use super::super::quotient::{
        adapt_axis_aligned_placement, certify_rank2_placement, AmbientLatticeId, DeckRank,
    };
    use super::super::xmonotone::{make_x_monotone, NumericalPolicy, XMonotonePiece2};
    use super::*;
    use crate::source_evidence::{BoundId, EdgeUseId, SourceVertexKey};
    use truck_geometry::prelude::Point2;

    fn provenance_with(
        edge_index: usize,
        start: SourceVertexKey,
        end: SourceVertexKey,
    ) -> CurveOccurrenceProvenance {
        CurveOccurrenceProvenance {
            source_face_id: Some(SourceFaceId(1)),
            bound_id: BoundId(0),
            edge_use_id: EdgeUseId::new(BoundId(0), edge_index),
            source_edge_id: SourceEdgeId(edge_index),
            start_vertex_id: start,
            end_vertex_id: end,
            source_curve_entity_id: Some(SourceEntityId(100 + edge_index as u64)),
        }
    }

    /// A line occurrence and its single x-monotone piece, with a span over the
    /// same occurrence.
    fn line_span_and_piece(
        start: Point2,
        end: Point2,
        edge_index: usize,
        sv: SourceVertexKey,
        ev: SourceVertexKey,
    ) -> (CurveSpan2, XMonotonePiece2) {
        let segment = LineSegment2 {
            start,
            end,
            provenance: provenance_with(edge_index, sv, ev),
        };
        let occ = DevelopedCurve2D::Line(segment);
        let piece = make_x_monotone(&occ, &NumericalPolicy::standard())
            .unwrap()
            .remove(0);
        (CurveSpan2::from_line(segment), piece)
    }

    fn intersect(
        lhs: &(CurveSpan2, XMonotonePiece2),
        rhs: &(CurveSpan2, XMonotonePiece2),
    ) -> PairContactResult {
        let result = intersect_x_monotone(&lhs.1, &rhs.1, &IntersectionPolicy::standard());
        lift_pair_result(&lhs.0, &rhs.0, &result)
    }

    #[test]
    fn disjoint_lifts_to_disjoint() {
        let a = line_span_and_piece(
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            0,
            SourceVertexKey::ShellVertex(1),
            SourceVertexKey::ShellVertex(2),
        );
        let b = line_span_and_piece(
            Point2::new(0.0, 5.0),
            Point2::new(1.0, 5.0),
            1,
            SourceVertexKey::ShellVertex(3),
            SourceVertexKey::ShellVertex(4),
        );
        assert_eq!(intersect(&a, &b), PairContactResult::Disjoint);
    }

    #[test]
    fn transverse_interior_crossing_lifts_to_two_branch_event() {
        let a = line_span_and_piece(
            Point2::new(0.0, 0.5),
            Point2::new(1.0, 0.5),
            0,
            SourceVertexKey::ShellVertex(1),
            SourceVertexKey::ShellVertex(2),
        );
        let b = line_span_and_piece(
            Point2::new(0.5, 0.0),
            Point2::new(0.5, 1.0),
            1,
            SourceVertexKey::ShellVertex(3),
            SourceVertexKey::ShellVertex(4),
        );
        let lifted = intersect(&a, &b);
        let PairContactResult::Components(comps) = lifted else {
            panic!("expected components, got {lifted:?}");
        };
        assert_eq!(comps.len(), 1, "one transverse crossing");
        let ContactComponent2::IsolatedEvent(event) = &comps[0] else {
            panic!("expected an isolated event");
        };
        assert_eq!(event.crossing, CrossingClassification::Transverse);
        assert_eq!(event.branches.len(), 2, "two incident branches");
        assert!(
            event
                .branches
                .iter()
                .all(|br| br.germ == BranchGerm::Regular),
            "analytic branches are regular"
        );
        assert!(
            event.branches.iter().all(|br| br.deck.is_zero()),
            "rank-0 deck label"
        );
        // Interior crossing: neither branch location is a source endpoint.
        assert!(event
            .branches
            .iter()
            .all(|br| br.location == ParameterLocation::PieceInterior));
        // Identity is a canonical isolated root (no shared source vertex here),
        // never a point.
        assert!(matches!(event.identity, EventIdentity::IsolatedRoot(_)));
    }

    #[test]
    fn shared_vertex_lifts_to_shared_source_vertex_identity() {
        let shared = SourceVertexKey::ShellVertex(9);
        let a = line_span_and_piece(
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            0,
            SourceVertexKey::ShellVertex(1),
            shared,
        );
        // A straight continuation through the shared vertex: collinear tangent
        // contact admitted as a source-declared join.
        let b = line_span_and_piece(
            Point2::new(1.0, 0.0),
            Point2::new(2.0, 0.0),
            1,
            shared,
            SourceVertexKey::ShellVertex(2),
        );
        let lifted = intersect(&a, &b);
        let PairContactResult::Components(comps) = lifted else {
            panic!("expected components, got {lifted:?}");
        };
        let ContactComponent2::IsolatedEvent(event) = &comps[0] else {
            panic!("expected an isolated event");
        };
        assert_eq!(event.identity, EventIdentity::SharedSourceVertex(shared));
        assert_eq!(event.crossing, CrossingClassification::Tangent);
    }

    #[test]
    fn collinear_overlap_certifies_a_common_arc() {
        // GEN-001E: a positive-length collinear overlap between distinct line
        // occurrences is no longer a bare `Unsupported(Overlap)` — the analytic
        // support identity is certified by exact predicates and the overlap
        // becomes one CommonArc component with two participants.
        let a = line_span_and_piece(
            Point2::new(0.0, 0.0),
            Point2::new(2.0, 0.0),
            0,
            SourceVertexKey::ShellVertex(1),
            SourceVertexKey::ShellVertex(2),
        );
        let b = line_span_and_piece(
            Point2::new(1.0, 0.0),
            Point2::new(3.0, 0.0),
            1,
            SourceVertexKey::ShellVertex(3),
            SourceVertexKey::ShellVertex(4),
        );
        let PairContactResult::Components(components) = intersect(&a, &b) else {
            panic!("a certified collinear overlap must lift to components");
        };
        assert_eq!(components.len(), 1);
        let ContactComponent2::CommonArc(arc) = &components[0] else {
            panic!("a positive-length overlap must be a CommonArc");
        };
        assert_eq!(arc.participants.len(), 2);
        assert_eq!(
            arc.support.basis,
            CommonSupportBasis::IdenticalAnalyticSupport
        );
        assert!(
            arc.participants
                .iter()
                .all(|p| p.parameter_interval.lo < p.parameter_interval.hi),
            "both participant intervals have positive certified extent"
        );
        assert!(
            arc.certificate.boundaries_certified_distinct,
            "boundaries are certified distinct"
        );
        arc.validate().expect("the certified CommonArc validates");
    }

    #[test]
    fn identity_invariant_under_curve_order_swap_single_root() {
        // SINGLE-ROOT pair only. The canonical participants and ordinal 0 are
        // the same either way. A two-root operand-swap test -- proving each
        // geometric root keeps its identity -- is exercised in GEN-001C on the
        // generic Bézier path, where identities are keyed by canonical pair
        // ordinals rather than a per-curve index.
        let a = line_span_and_piece(
            Point2::new(0.0, 0.5),
            Point2::new(1.0, 0.5),
            0,
            SourceVertexKey::ShellVertex(1),
            SourceVertexKey::ShellVertex(2),
        );
        let b = line_span_and_piece(
            Point2::new(0.5, 0.0),
            Point2::new(0.5, 1.0),
            1,
            SourceVertexKey::ShellVertex(3),
            SourceVertexKey::ShellVertex(4),
        );
        let PairContactResult::Components(ab) = intersect(&a, &b) else {
            unreachable!()
        };
        let PairContactResult::Components(ba) = intersect(&b, &a) else {
            unreachable!()
        };
        let id_ab = match &ab[0] {
            ContactComponent2::IsolatedEvent(e) => e.identity.clone(),
            _ => unreachable!(),
        };
        let id_ba = match &ba[0] {
            ContactComponent2::IsolatedEvent(e) => e.identity.clone(),
            _ => unreachable!(),
        };
        // Single crossing: index 0 either way, so the identities are equal.
        // (Multi-root invariance is the GEN-001C test.)
        assert_eq!(
            id_ab, id_ba,
            "single-root event identity is invariant under swap"
        );
    }

    // ----- GEN-001D: canonical deck gauge normalization --------------------

    fn rank1_context() -> DeckContext {
        DeckContext::from_lattice_id(AmbientLatticeId::Rank1 {
            periodic_axis: ParameterAxis::V,
            signed_period_bits: std::f64::consts::TAU.to_bits(),
        })
    }

    fn rank2_context() -> DeckContext {
        DeckContext::from_lattice_id(AmbientLatticeId::Rank2 {
            first: [1.0_f64.to_bits(), 0.0_f64.to_bits()],
            second: [0.0_f64.to_bits(), 1.0_f64.to_bits()],
        })
    }

    fn placement_label(context: DeckContext, u: i64, v: i64) -> CertifiedDeckLabel {
        CertifiedDeckLabel::certified_placement(context, DeckLabel { u, v })
    }

    fn synthetic_branch(
        edge_index: usize,
        parameter: (f64, f64),
        side: CanonicalBranchSide,
        deck: CertifiedDeckLabel,
    ) -> BranchIncidence {
        let provenance = provenance_with(
            edge_index,
            SourceVertexKey::ShellVertex(1),
            SourceVertexKey::ShellVertex(2),
        );
        BranchIncidence {
            span_id: SpanId::from_occurrence(&provenance),
            provenance,
            parameter: ParameterEnclosure::from_pair(parameter),
            location: ParameterLocation::PieceInterior,
            germ: BranchGerm::Regular,
            side,
            deck,
            representative: Point2::new(0.0, 0.0),
        }
    }

    /// A reversal-stable synthetic branch: same span occurrence, swapped
    /// traversal endpoints, parameter orientation reversed.
    fn synthetic_branch_reversed(
        edge_index: usize,
        parameter: (f64, f64),
        side: CanonicalBranchSide,
        deck: CertifiedDeckLabel,
    ) -> BranchIncidence {
        let mut branch = synthetic_branch(edge_index, parameter, side, deck);
        branch.provenance = branch.provenance.reversed();
        branch
    }

    fn synthetic_event(
        identity: EventIdentity,
        context: DeckContext,
        branches: Vec<BranchIncidence>,
    ) -> IsolatedEvent2 {
        IsolatedEvent2 {
            identity,
            crossing: CrossingClassification::Transverse,
            branches,
            deck_context: context,
            representative: Point2::new(0.0, 0.0),
        }
    }

    fn isolated_root_identity() -> EventIdentity {
        EventIdentity::IsolatedRoot(IsolatedRootKey::new(
            SpanId::from_occurrence(&provenance_with(
                0,
                SourceVertexKey::ShellVertex(1),
                SourceVertexKey::ShellVertex(2),
            )),
            SpanId::from_occurrence(&provenance_with(
                1,
                SourceVertexKey::ShellVertex(3),
                SourceVertexKey::ShellVertex(4),
            )),
            0,
        ))
    }

    #[test]
    fn rank1_gauge_signature_is_invariant_under_translation_and_permutation() {
        let context = rank1_context();
        let identity = isolated_root_identity();
        let first = CanonicalBranchSide::First;
        // [3,5,8] -> [0,2,5] anchored at the canonical minimum (span 0).
        let base = synthetic_event(
            identity.clone(),
            context,
            vec![
                synthetic_branch(0, (0.1, 0.2), first, placement_label(context, 3, 0)),
                synthetic_branch(1, (0.5, 0.6), first, placement_label(context, 5, 0)),
                synthetic_branch(2, (0.8, 0.9), first, placement_label(context, 8, 0)),
            ],
        );
        // [-7,-5,-2] is [3,5,8] translated by -10: same quotient event.
        let translated = synthetic_event(
            identity.clone(),
            context,
            vec![
                synthetic_branch(0, (0.1, 0.2), first, placement_label(context, -7, 0)),
                synthetic_branch(1, (0.5, 0.6), first, placement_label(context, -5, 0)),
                synthetic_branch(2, (0.8, 0.9), first, placement_label(context, -2, 0)),
            ],
        );
        // Insertion order is irrelevant: the anchor and order are canonical.
        let permuted = synthetic_event(
            identity.clone(),
            context,
            vec![
                synthetic_branch(2, (0.8, 0.9), first, placement_label(context, 8, 0)),
                synthetic_branch(0, (0.1, 0.2), first, placement_label(context, 3, 0)),
                synthetic_branch(1, (0.5, 0.6), first, placement_label(context, 5, 0)),
            ],
        );
        let signature = base.deck_signature().unwrap();
        assert_eq!(signature, translated.deck_signature().unwrap());
        assert_eq!(signature, permuted.deck_signature().unwrap());
        // Deterministic repetition: recomputing the same event yields the same
        // signature.
        assert_eq!(signature, base.deck_signature().unwrap());
        assert_eq!(
            signature.relative(),
            &[
                DeckLabel::rank1(0),
                DeckLabel::rank1(2),
                DeckLabel::rank1(5)
            ]
        );
    }

    #[test]
    fn rank2_gauge_signature_is_invariant_under_translation() {
        let context = rank2_context();
        let identity = isolated_root_identity();
        let first = CanonicalBranchSide::First;
        // [(2,-1),(5,4)] -> [(0,0),(3,5)].
        let a = synthetic_event(
            identity.clone(),
            context,
            vec![
                synthetic_branch(0, (0.1, 0.2), first, placement_label(context, 2, -1)),
                synthetic_branch(1, (0.5, 0.6), first, placement_label(context, 5, 4)),
            ],
        );
        // [(12,9),(15,14)] is [(2,-1),(5,4)] translated by (10,10).
        let b = synthetic_event(
            identity.clone(),
            context,
            vec![
                synthetic_branch(0, (0.1, 0.2), first, placement_label(context, 12, 9)),
                synthetic_branch(1, (0.5, 0.6), first, placement_label(context, 15, 14)),
            ],
        );
        let signature = a.deck_signature().unwrap();
        assert_eq!(signature, b.deck_signature().unwrap());
        assert_eq!(
            signature.relative(),
            &[DeckLabel::rank2(0, 0), DeckLabel::rank2(3, 5)]
        );
    }

    #[test]
    fn operand_swap_leaves_the_signature_unchanged() {
        let context = rank1_context();
        let identity = isolated_root_identity();
        // Canonical sides are derived from the sorted span pair, so reversing
        // the branch list still normalizes to the same anchor and order.
        let ab = synthetic_event(
            identity.clone(),
            context,
            vec![
                synthetic_branch(
                    0,
                    (0.1, 0.2),
                    CanonicalBranchSide::First,
                    placement_label(context, 4, 0),
                ),
                synthetic_branch(
                    1,
                    (0.5, 0.6),
                    CanonicalBranchSide::Second,
                    placement_label(context, 7, 0),
                ),
            ],
        );
        let ba = synthetic_event(
            identity.clone(),
            context,
            vec![
                synthetic_branch(
                    1,
                    (0.5, 0.6),
                    CanonicalBranchSide::Second,
                    placement_label(context, 7, 0),
                ),
                synthetic_branch(
                    0,
                    (0.1, 0.2),
                    CanonicalBranchSide::First,
                    placement_label(context, 4, 0),
                ),
            ],
        );
        assert_eq!(ab.deck_signature().unwrap(), ba.deck_signature().unwrap());
        assert_eq!(
            ab.pair_contact_lift_key().unwrap(),
            ba.pair_contact_lift_key().unwrap()
        );
    }

    #[test]
    fn source_traversal_reversal_does_not_change_lift_identity() {
        let context = rank1_context();
        let identity = isolated_root_identity();
        let normal = synthetic_event(
            identity.clone(),
            context,
            vec![
                synthetic_branch(
                    0,
                    (0.1, 0.2),
                    CanonicalBranchSide::First,
                    placement_label(context, 3, 0),
                ),
                synthetic_branch(
                    1,
                    (0.5, 0.6),
                    CanonicalBranchSide::Second,
                    placement_label(context, 9, 0),
                ),
            ],
        );
        // Reversal changes traversal roles and parameter orientation, not the
        // physical lift: same span occurrences, same canonical sides, same
        // labels, reversed parameter enclosures.
        let reversed = synthetic_event(
            identity.clone(),
            context,
            vec![
                synthetic_branch_reversed(
                    0,
                    (0.8, 0.9),
                    CanonicalBranchSide::First,
                    placement_label(context, 3, 0),
                ),
                synthetic_branch_reversed(
                    1,
                    (0.4, 0.5),
                    CanonicalBranchSide::Second,
                    placement_label(context, 9, 0),
                ),
            ],
        );
        assert_eq!(
            normal.deck_signature().unwrap(),
            reversed.deck_signature().unwrap()
        );
    }

    #[test]
    fn subdivision_inherits_the_parent_lift_context() {
        let context = rank1_context();
        let identity = isolated_root_identity();
        let parent = synthetic_event(
            identity.clone(),
            context,
            vec![
                synthetic_branch(
                    0,
                    (0.1, 0.2),
                    CanonicalBranchSide::First,
                    placement_label(context, 3, 0),
                ),
                synthetic_branch(
                    1,
                    (0.5, 0.6),
                    CanonicalBranchSide::Second,
                    placement_label(context, 9, 0),
                ),
            ],
        );
        // A derived (subdivided) piece inherits its parent occurrence's span
        // identity, canonical side and deck label, with a narrower certified
        // parameter enclosure. No fresh label is minted by observing
        // coordinates, and the narrower enclosure does not move the anchor.
        let derived = synthetic_event(
            identity.clone(),
            context,
            vec![
                synthetic_branch(
                    0,
                    (0.15, 0.18),
                    CanonicalBranchSide::First,
                    placement_label(context, 3, 0).inherited(),
                ),
                synthetic_branch(
                    1,
                    (0.55, 0.56),
                    CanonicalBranchSide::Second,
                    placement_label(context, 9, 0).inherited(),
                ),
            ],
        );
        assert_eq!(
            parent.deck_signature().unwrap(),
            derived.deck_signature().unwrap()
        );
    }

    #[test]
    fn rank_mismatch_fails_clearly() {
        let identity = isolated_root_identity();
        // A rank-2 label in a rank-1 event: typed RankMismatch, never a silent
        // truncation to (1,0).
        let bad = synthetic_event(
            identity.clone(),
            rank1_context(),
            vec![
                synthetic_branch(
                    0,
                    (0.1, 0.2),
                    CanonicalBranchSide::First,
                    placement_label(rank2_context(), 1, 1),
                ),
                synthetic_branch(
                    1,
                    (0.5, 0.6),
                    CanonicalBranchSide::Second,
                    placement_label(rank1_context(), 4, 0),
                ),
            ],
        );
        assert!(matches!(
            bad.deck_signature(),
            Err(DeckLabelError::RankMismatch { .. })
        ));
        // A rank-1 label in a rank-2 event: typed RankMismatch.
        let bad2 = synthetic_event(
            identity.clone(),
            rank2_context(),
            vec![synthetic_branch(
                0,
                (0.1, 0.2),
                CanonicalBranchSide::First,
                placement_label(rank1_context(), 1, 0),
            )],
        );
        assert!(matches!(
            bad2.deck_signature(),
            Err(DeckLabelError::RankMismatch { .. })
        ));
    }

    #[test]
    fn lattice_mismatch_fails_clearly() {
        let identity = isolated_root_identity();
        // Two different rank-1 lattices: same rank, different generators. A
        // label bound to the wrong lattice is a typed LatticeMismatch, never a
        // reinterpretation across generators or orientation conventions.
        let context_a = rank1_context();
        let context_b = DeckContext::from_lattice_id(AmbientLatticeId::Rank1 {
            periodic_axis: ParameterAxis::V,
            signed_period_bits: 3.14_f64.to_bits(),
        });
        let bad = synthetic_event(
            identity.clone(),
            context_a,
            vec![
                synthetic_branch(
                    0,
                    (0.1, 0.2),
                    CanonicalBranchSide::First,
                    placement_label(context_a, 0, 0),
                ),
                synthetic_branch(
                    1,
                    (0.5, 0.6),
                    CanonicalBranchSide::Second,
                    placement_label(context_b, 1, 0),
                ),
            ],
        );
        assert!(matches!(
            bad.deck_signature(),
            Err(DeckLabelError::LatticeMismatch { .. })
        ));
    }

    #[test]
    fn rank0_signature_is_trivial_zero() {
        let context = DeckContext::rank0();
        let identity = isolated_root_identity();
        let event = synthetic_event(
            identity,
            context,
            vec![
                synthetic_branch(
                    0,
                    (0.1, 0.2),
                    CanonicalBranchSide::First,
                    CertifiedDeckLabel::zero(context),
                ),
                synthetic_branch(
                    1,
                    (0.5, 0.6),
                    CanonicalBranchSide::Second,
                    CertifiedDeckLabel::zero(context),
                ),
            ],
        );
        let signature = event.deck_signature().unwrap();
        assert_eq!(signature.rank(), DeckRank::Rank0);
        assert!(signature.relative().iter().all(|label| label.is_zero()));
    }

    #[test]
    fn pair_contact_key_is_relative_not_absolute() {
        let context = rank1_context();
        let identity = isolated_root_identity();
        // Same identity, same relative signature, different absolute labels:
        // the same quotient event.
        let ev_a = synthetic_event(
            identity.clone(),
            context,
            vec![
                synthetic_branch(
                    0,
                    (0.1, 0.2),
                    CanonicalBranchSide::First,
                    placement_label(context, 5, 0),
                ),
                synthetic_branch(
                    1,
                    (0.5, 0.6),
                    CanonicalBranchSide::Second,
                    placement_label(context, 8, 0),
                ),
            ],
        );
        let ev_b = synthetic_event(
            identity.clone(),
            context,
            vec![
                synthetic_branch(
                    0,
                    (0.1, 0.2),
                    CanonicalBranchSide::First,
                    placement_label(context, 2, 0),
                ),
                synthetic_branch(
                    1,
                    (0.5, 0.6),
                    CanonicalBranchSide::Second,
                    placement_label(context, 5, 0),
                ),
            ],
        );
        assert_eq!(
            ev_a.pair_contact_lift_key().unwrap(),
            ev_b.pair_contact_lift_key().unwrap()
        );
        // A genuinely different relative signature stays distinct even when the
        // representative coordinates coincide.
        let ev_c = synthetic_event(
            identity.clone(),
            context,
            vec![
                synthetic_branch(
                    0,
                    (0.1, 0.2),
                    CanonicalBranchSide::First,
                    placement_label(context, 5, 0),
                ),
                synthetic_branch(
                    1,
                    (0.5, 0.6),
                    CanonicalBranchSide::Second,
                    placement_label(context, 9, 0),
                ),
            ],
        );
        assert_ne!(
            ev_a.pair_contact_lift_key().unwrap(),
            ev_c.pair_contact_lift_key().unwrap()
        );
    }

    #[test]
    fn rank1_label_path_carries_the_label_end_to_end() {
        let context = rank1_context();
        let identity = isolated_root_identity();
        let mut event = synthetic_event(
            identity,
            context,
            vec![
                synthetic_branch(
                    0,
                    (0.1, 0.2),
                    CanonicalBranchSide::First,
                    CertifiedDeckLabel::zero(context),
                ),
                synthetic_branch(
                    1,
                    (0.5, 0.6),
                    CanonicalBranchSide::Second,
                    CertifiedDeckLabel::zero(context),
                ),
            ],
        );
        let generator = DeckGenerator::new(
            DevelopedAxis::First,
            FiniteF64::new(std::f64::consts::TAU).unwrap(),
        )
        .unwrap();
        // One full period: the certified solver says k = 1, the adapter turns it
        // into one lattice-bound rank-1 label, and the label is attached to
        // branch 1.
        let one_period = DevelopedBox {
            first: DeckInterval::from_f64(std::f64::consts::TAU, std::f64::consts::TAU).unwrap(),
            second: DeckInterval::from_f64(0.0, 0.0).unwrap(),
        };
        let placement =
            adapt_axis_aligned_placement(context, solve_axis_aligned(&generator, &one_period));
        label_branch_from_placement(&mut event, 1, placement).unwrap();
        assert_eq!(event.branches[1].deck.get(), DeckLabel::rank1(1));
        assert_eq!(
            event.deck_signature().unwrap().relative(),
            &[DeckLabel::rank1(0), DeckLabel::rank1(1)]
        );
        // A broad enclosure gives several compatible integers: the attachment is
        // a typed NonUniquePlacement(Ambiguous) and the branch label is left
        // untouched — no near-integer rounding and no arbitrary pick.
        let broad = DevelopedBox {
            first: DeckInterval::from_f64(0.0, 3.0 * std::f64::consts::TAU).unwrap(),
            second: DeckInterval::from_f64(0.0, 0.0).unwrap(),
        };
        let placement =
            adapt_axis_aligned_placement(context, solve_axis_aligned(&generator, &broad));
        assert!(matches!(
            label_branch_from_placement(&mut event, 1, placement),
            Err(DeckLabelError::NonUniquePlacement(
                DeckPlacementResult::Ambiguous
            ))
        ));
        assert_eq!(event.branches[1].deck.get(), DeckLabel::rank1(1));
    }

    #[test]
    fn label_branch_from_placement_rejects_an_out_of_range_index() {
        let context = rank1_context();
        let identity = isolated_root_identity();
        let mut event = synthetic_event(
            identity,
            context,
            vec![synthetic_branch(
                0,
                (0.1, 0.2),
                CanonicalBranchSide::First,
                CertifiedDeckLabel::zero(context),
            )],
        );
        let placement = adapt_axis_aligned_placement(
            context,
            Ok(super::super::deck::DeckSolveResult::Unique(1)),
        );
        assert!(matches!(
            label_branch_from_placement(&mut event, 7, placement),
            Err(DeckLabelError::NoSuchBranch { index: 7 })
        ));
    }

    #[test]
    fn rank2_geometric_placement_is_typed_unsupported_end_to_end() {
        let context = rank2_context();
        let identity = isolated_root_identity();
        let mut event = synthetic_event(
            identity,
            context,
            vec![
                synthetic_branch(
                    0,
                    (0.1, 0.2),
                    CanonicalBranchSide::First,
                    CertifiedDeckLabel::zero(context),
                ),
                synthetic_branch(
                    1,
                    (0.5, 0.6),
                    CanonicalBranchSide::Second,
                    CertifiedDeckLabel::zero(context),
                ),
            ],
        );
        // There is no certified rank-2 placement solver yet: the adapter returns
        // typed Unsupported, and label attachment propagates it without
        // guessing a closest-vector answer.
        let placement = certify_rank2_placement();
        assert!(matches!(
            label_branch_from_placement(&mut event, 1, placement),
            Err(DeckLabelError::NonUniquePlacement(
                DeckPlacementResult::Unsupported(_)
            ))
        ));
        assert!(event.branches[1].deck.is_zero());
    }
}
