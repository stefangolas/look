//! Rank-parametric deck labels for the quotient substrate.
//!
//! GEN-001A introduced the plain label types ([`DeckRank`], [`DeckLabel`]) that
//! every arrangement-facing record carries. GEN-001D lands the certified
//! rank/deck contract on top of them, reusing the authoritative ambient lattice
//! ([`super::ambient::CertifiedAmbientLattice`], whose rank is resolved by
//! [`super::ambient::resolve_ambient_periods`]) and the rank-1 axis-aligned deck
//! solver ([`super::deck::solve_axis_aligned`]) rather than inventing a parallel
//! arithmetic:
//!
//! - [`AmbientLatticeId`] and [`DeckContext`] — a deck vector is meaningful only
//!   relative to a *particular* certified ambient lattice, not merely its rank.
//!   The context binds every incidence of an event to one lattice via a stable
//!   [`AmbientLatticeId`] derived from the certified generators; it is a token,
//!   never a duplicated lattice certificate. The rank-0 lattice is unique, so a
//!   rank-0 context is constructible directly; a rank-1 or rank-2 context is
//!   minted only from a [`super::ambient::CertifiedAmbientLattice`].
//! - [`CertifiedDeckLabel`] — a rank-tagged, lattice-bound, validated deck
//!   vector with an explicit [`DeckLabelBasis`]. The constructors fix the rank
//!   structurally and the lattice binding (the fields are private; raw
//!   rank-1/rank-2 constructors do not exist publicly), and attaching a label to
//!   a context requires [`CertifiedDeckLabel::validate_for`], so a label from a
//!   different lattice — even of the same rank — is a typed
//!   [`DeckLabelError::LatticeMismatch`], never a silent truncation, padding or
//!   reinterpretation.
//! - [`DeckPlacementResult`] and [`adapt_axis_aligned_placement`] — the
//!   conservative adaptation of the existing four-way [`DeckSolveResult`]: a
//!   uniquely certified integer becomes one lattice-bound label, multiple
//!   compatible integers become `Ambiguous`, indeterminate arithmetic stays
//!   `Unresolved`, and overflow/resource failure is `OperationalFailure`.
//!   Epistemic ambiguity is never collapsed into operational failure, and no
//!   near-integer rounding ever mints a label. General rank-2 geometric
//!   placement is a typed [`DeckPlacementResult::Unsupported`] until a certified
//!   solver exists.
//! - [`CanonicalIncidenceId`], [`CanonicalBranchSide`] and [`DeckSignature`] —
//!   the canonical *relative* deck signature of an event. Absolute lifts are
//!   gauge-dependent (`k_i -> k_i + h` is the same quotient event), so event
//!   identity is established by a normalized relative label set in canonical
//!   incidence order, never by raw absolute labels. The canonical order is
//!   construction-based (source occurrence + canonical branch side), never
//!   parameter-enclosure bits.
//!
//! `FORMAL_SYSTEM.md` §VII–VIII state deck displacement `δ ∈ Z^r` and candidate
//! translation sets `K_ij` in terms of the ambient lattice `Λ = LZ^r`,
//! `0 ≤ r ≤ 2`. Rank 0 is the ordinary nonperiodic developed plane and carries
//! only the zero label. Periodicity is represented by deck labels and translated
//! lifts, never by wrapping coordinates modulo a period before the topology is
//! solved.

use super::deck::{DeckOperationalFailure, DeckSolveResult};
use super::evidence::ParameterAxis;
use super::outcome::ResourceOperation;
use super::span::SpanId;

/// The lattice rank of a developed chart: 0, 1 or 2.
///
/// Carried by the quotient domain (GEN-001D). A rank-0 chart has no deck
/// identifications; rank 1 has one periodic axis (cylinder/cone away from the
/// apex); rank 2 has two (torus).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DeckRank {
    /// The ordinary nonperiodic developed plane.
    Rank0,
    /// One periodic axis (e.g. an embedded cylinder).
    Rank1,
    /// Two periodic axes (e.g. a torus).
    Rank2,
}

impl DeckRank {
    /// A short stable tag, for diagnostics.
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Rank0 => "deck_rank0",
            Self::Rank1 => "deck_rank1",
            Self::Rank2 => "deck_rank2",
        }
    }
}

/// An integer deck displacement of at most two components.
///
/// The zero label identifies the base copy; a nonzero component translates the
/// piece by that multiple of the corresponding period generator. Rank-0 charts
/// carry only [`DeckLabel::ZERO`]. Identity is the integer pair, never a rounded
/// coordinate: two events identified by a certified deck translation carry the
/// label that translates one to the other.
///
/// This is the plain record label: it carries no rank tag and no lattice
/// binding. The *certified* rank/deck contract — where a label is meaningful
/// only relative to a particular certified ambient lattice — is
/// [`CertifiedDeckLabel`]; prefer it wherever a deck label is validated rather
/// than merely stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeckLabel {
    /// Displacement along the first period generator.
    pub u: i64,
    /// Displacement along the second period generator (0 for rank ≤ 1).
    pub v: i64,
}

impl DeckLabel {
    /// The zero displacement: the base copy. The only label a rank-0 chart
    /// carries.
    pub const ZERO: Self = Self { u: 0, v: 0 };

    /// A rank-0 label (always zero).
    pub const fn rank0() -> Self {
        Self::ZERO
    }

    /// A rank-1 label: displacement `u` along the single period generator.
    pub const fn rank1(u: i64) -> Self {
        Self { u, v: 0 }
    }

    /// A rank-2 label: displacements along both period generators.
    pub const fn rank2(u: i64, v: i64) -> Self {
        Self { u, v }
    }

    /// Whether this is the zero (base-copy) label.
    pub const fn is_zero(self) -> bool {
        self.u == 0 && self.v == 0
    }
}

// ---------------------------------------------------------------------------
// Ambient lattice identity and the shared event context
// ---------------------------------------------------------------------------

/// A stable token identifying one certified ambient lattice.
///
/// A deck vector is meaningful only relative to a *particular* lattice: two
/// rank-1 lattices can have different generators, periods or orientation
/// conventions. The token is derived from the certified generators of
/// [`super::ambient::CertifiedAmbientLattice`] by
/// [`super::ambient::CertifiedAmbientLattice::deck_context`] — certified values,
/// never a bare rank or a caller-supplied name. It is a compact identity, not
/// the lattice certificate.
///
/// Generator magnitudes are stored as their `f64` bit patterns. This is a valid
/// equality encoding for the values that occur here: certified generators are
/// finite and nonzero, and for finite values `to_bits` equality agrees with
/// value equality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AmbientLatticeId {
    /// The unique rank-0 (trivial) lattice. There is exactly one.
    Rank0,
    /// A rank-1 lattice, identified by its certified generator: the periodic
    /// axis and the signed period magnitude.
    Rank1 {
        /// The periodic axis.
        periodic_axis: ParameterAxis,
        /// `magnitude().get().to_bits()` of the certified generator.
        signed_period_bits: u64,
    },
    /// A rank-2 lattice, identified by its two certified generator
    /// translations (each the `(du, dv)` pair, bit-encoded).
    Rank2 {
        /// The first generator's translation.
        first: [u64; 2],
        /// The second generator's translation.
        second: [u64; 2],
    },
}

impl AmbientLatticeId {
    /// The rank of the lattice this id names.
    pub const fn rank(self) -> DeckRank {
        match self {
            Self::Rank0 => DeckRank::Rank0,
            Self::Rank1 { .. } => DeckRank::Rank1,
            Self::Rank2 { .. } => DeckRank::Rank2,
        }
    }
}

/// The ambient lattice/rank context one event's deck labels are relative to.
///
/// Carried once per event, never duplicated per incidence. The context binds
/// every incidence of an event to one certified lattice via its
/// [`AmbientLatticeId`]; the full lattice certificate lives at the resolution
/// layer ([`super::ambient::CertifiedAmbientLattice`]) and is not copied here.
///
/// A rank-0 context is constructible directly because the rank-0 lattice is
/// unique. A rank-1 or rank-2 context can only be minted from a certified
/// lattice ([`super::ambient::CertifiedAmbientLattice::deck_context`]); there is
/// no `DeckContext::rank1()` that would silently leave the lattice unspecified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeckContext {
    lattice: AmbientLatticeId,
}

impl DeckContext {
    /// The rank-0 (nonperiodic) context: the unique trivial lattice.
    pub const fn rank0() -> Self {
        Self {
            lattice: AmbientLatticeId::Rank0,
        }
    }

    /// The context of a specific certified lattice.
    ///
    /// `pub(crate)`: the public route is
    /// [`super::ambient::CertifiedAmbientLattice::deck_context`], so a caller
    /// cannot bind a context to a lattice it did not certify.
    pub(crate) fn from_lattice_id(lattice: AmbientLatticeId) -> Self {
        Self { lattice }
    }

    /// The certified lattice this context is bound to.
    pub const fn lattice(self) -> AmbientLatticeId {
        self.lattice
    }

    /// The rank of the bound lattice.
    pub const fn rank(self) -> DeckRank {
        self.lattice.rank()
    }
}

// ---------------------------------------------------------------------------
// Certified deck labels
// ---------------------------------------------------------------------------

/// Why a certified deck label, placement or signature could not be produced.
///
/// Every variant is a distinct proposition. A lattice mismatch, a rank mismatch
/// and an arithmetic overflow are not the same failure, and none is a statement
/// about the surface: [`DeckLabelError::NonUniquePlacement`] preserves the
/// solver's own verdict so epistemic ambiguity (indeterminate evidence, several
/// compatible integers) is never collapsed into operational failure or
/// unsupportedness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeckLabelError {
    /// The label is bound to a different certified lattice. Even two rank-1
    /// lattices are distinct; never reinterpret a vector across them.
    LatticeMismatch {
        /// The lattice that was required.
        expected: AmbientLatticeId,
        /// The lattice the label is actually bound to.
        found: AmbientLatticeId,
    },
    /// A label's vector rank does not match the rank it was offered to. Never
    /// silently truncate, pad or reinterpret the offending components.
    RankMismatch {
        /// The rank that was required.
        expected: DeckRank,
        /// The rank the label actually carries.
        found: DeckRank,
    },
    /// Checked integer arithmetic refused. An `i64` deck coordinate
    /// overflowing is an implementation fact, not a mathematical judgment.
    ArithmeticOverflow(ResourceOperation),
    /// An isolated event carried no incidences, so no canonical anchor and no
    /// deck signature exist.
    EmptyEvent,
    /// A branch index named no branch of the event.
    NoSuchBranch {
        /// The requested index.
        index: usize,
    },
    /// A certified deck placement did not resolve to a unique label. The
    /// solver's verdict is carried verbatim.
    NonUniquePlacement(DeckPlacementResult),
}

impl DeckLabelError {
    /// A short stable tag, for diagnostics.
    pub fn tag(self) -> &'static str {
        match self {
            Self::LatticeMismatch { .. } => "deck_label_lattice_mismatch",
            Self::RankMismatch { .. } => "deck_label_rank_mismatch",
            Self::ArithmeticOverflow(_) => "deck_label_arithmetic_overflow",
            Self::EmptyEvent => "deck_label_empty_event",
            Self::NoSuchBranch { .. } => "deck_label_no_such_branch",
            Self::NonUniquePlacement(_) => "deck_label_non_unique_placement",
        }
    }
}

/// Why a certified deck vector is certified.
///
/// Provenance, not arithmetic: every variant names the construction route that
/// discharged the obligation. The vector's value never depends on the basis;
/// the basis exists so a review can tell a certified placement from a reused
/// parent lift without guessing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeckLabelBasis {
    /// The zero deck vector of a rank: `0 ∈ Z^r` is certified for every lattice
    /// of that rank without a placement computation.
    ZeroByRank,
    /// A certified deck placement: the existing four-way solver's `Unique`
    /// verdict.
    CertifiedPlacement,
    /// An explicit, certified deck transport (a certified deck transition).
    ///
    /// Representability in GEN-001D; populated by the certified transport paths
    /// (e.g. a deck join) and by ARR-003.
    ExplicitTransport,
    /// Inherited from a parent occurrence's lift context: derived (subdivided)
    /// pieces carry their parent's label and provenance rather than minting a
    /// fresh label by observing coordinates.
    InheritedFromParent,
}

/// A rank-tagged, lattice-bound, validated deck vector: an element of `Z^rank`
/// of one certified ambient lattice.
///
/// Conceptually the deck groups are `Rank 0 = {0}`, `Rank 1 = Z`, `Rank 2 = Z²`,
/// and a deck vector is meaningful only relative to a particular certified
/// ambient lattice. The label therefore carries its lattice binding and rank
/// structurally (a rank-1 label carries `v == 0`, a rank-0 label carries zero),
/// and the fields are private. There is no unrestricted public constructor that
/// takes bare rank-1/rank-2 components; the public route to a label is
/// [`CertifiedDeckLabel::zero`] (certified by rank) or a certified placement or
/// transport inside the crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CertifiedDeckLabel {
    context: AmbientLatticeId,
    vector: DeckLabel,
    basis: DeckLabelBasis,
}

impl CertifiedDeckLabel {
    /// The zero vector of the given context's lattice.
    ///
    /// `0 ∈ Z^r` is certified for every lattice of rank `r` without a placement
    /// computation, so this is the one label that can be minted from a context
    /// alone. It is bound to that context and validated by
    /// [`CertifiedDeckLabel::validate_for`] like any other label.
    pub fn zero(context: DeckContext) -> Self {
        Self {
            context: context.lattice(),
            vector: DeckLabel::ZERO,
            basis: DeckLabelBasis::ZeroByRank,
        }
    }

    /// The label certified by a deck placement.
    ///
    /// The trusted proof path: only the adapter over the existing certified
    /// solver ([`adapt_axis_aligned_placement`]) and other in-crate certified
    /// placements construct labels this way.
    pub(crate) fn certified_placement(context: DeckContext, vector: DeckLabel) -> Self {
        Self {
            context: context.lattice(),
            vector,
            basis: DeckLabelBasis::CertifiedPlacement,
        }
    }

    /// The label carried by an explicit, certified deck transport.
    ///
    /// Representability for the certified transport laws; the value is explicit
    /// and certified by construction, never inferred by wrapping a coordinate.
    #[allow(dead_code)] // certified-transport representability; exercised by tests, populated by ARR-003
    pub(crate) fn explicit_transport(context: DeckContext, vector: DeckLabel) -> Self {
        Self {
            context: context.lattice(),
            vector,
            basis: DeckLabelBasis::ExplicitTransport,
        }
    }

    /// The same label, re-based as inherited from a parent lift context.
    ///
    /// Subdivision/derived pieces inherit their parent occurrence's lift
    /// context; this records that provenance without changing the vector.
    #[allow(dead_code)] // subdivision-transport representability; exercised by tests
    pub(crate) fn inherited(self) -> Self {
        Self {
            context: self.context,
            vector: self.vector,
            basis: DeckLabelBasis::InheritedFromParent,
        }
    }

    /// The lattice this label is bound to.
    pub const fn context(self) -> AmbientLatticeId {
        self.context
    }

    /// The rank of this label.
    pub const fn rank(self) -> DeckRank {
        self.context.rank()
    }

    /// The plain label pair.
    pub const fn get(self) -> DeckLabel {
        self.vector
    }

    /// The construction basis that certified this label.
    pub const fn basis(self) -> DeckLabelBasis {
        self.basis
    }

    /// Whether this is the zero (base-copy) label.
    pub const fn is_zero(self) -> bool {
        self.vector.is_zero()
    }

    /// Checked addition. Both labels must carry the same rank and be bound to
    /// the same certified lattice (a mismatch is a typed error, never a silent
    /// reinterpretation); overflow is a typed arithmetic failure.
    pub fn checked_add(self, other: Self) -> Result<Self, DeckLabelError> {
        self.ensure_compatible(other)?;
        let u =
            self.vector
                .u
                .checked_add(other.vector.u)
                .ok_or(DeckLabelError::ArithmeticOverflow(
                    ResourceOperation::DeckVectorAddition,
                ))?;
        let v =
            self.vector
                .v
                .checked_add(other.vector.v)
                .ok_or(DeckLabelError::ArithmeticOverflow(
                    ResourceOperation::DeckVectorAddition,
                ))?;
        Ok(Self {
            context: self.context,
            vector: DeckLabel { u, v },
            basis: DeckLabelBasis::ExplicitTransport,
        })
    }

    /// Checked subtraction. Rank and overflow rules as for
    /// [`CertifiedDeckLabel::checked_add`].
    pub fn checked_sub(self, other: Self) -> Result<Self, DeckLabelError> {
        self.ensure_compatible(other)?;
        let u =
            self.vector
                .u
                .checked_sub(other.vector.u)
                .ok_or(DeckLabelError::ArithmeticOverflow(
                    ResourceOperation::DeckVectorSubtraction,
                ))?;
        let v =
            self.vector
                .v
                .checked_sub(other.vector.v)
                .ok_or(DeckLabelError::ArithmeticOverflow(
                    ResourceOperation::DeckVectorSubtraction,
                ))?;
        Ok(Self {
            context: self.context,
            vector: DeckLabel { u, v },
            basis: DeckLabelBasis::ExplicitTransport,
        })
    }

    /// Both operands must have the same rank and the same lattice binding. A
    /// rank difference is reported as [`DeckLabelError::RankMismatch`]; a
    /// same-rank cross-lattice combination as [`DeckLabelError::LatticeMismatch`].
    fn ensure_compatible(self, other: Self) -> Result<(), DeckLabelError> {
        if self.rank() != other.rank() {
            return Err(DeckLabelError::RankMismatch {
                expected: self.rank(),
                found: other.rank(),
            });
        }
        if self.context != other.context {
            return Err(DeckLabelError::LatticeMismatch {
                expected: self.context,
                found: other.context,
            });
        }
        Ok(())
    }

    /// Checked negation.
    pub fn negated(self) -> Result<Self, DeckLabelError> {
        let u = self
            .vector
            .u
            .checked_neg()
            .ok_or(DeckLabelError::ArithmeticOverflow(
                ResourceOperation::DeckVectorScaling,
            ))?;
        let v = self
            .vector
            .v
            .checked_neg()
            .ok_or(DeckLabelError::ArithmeticOverflow(
                ResourceOperation::DeckVectorScaling,
            ))?;
        Ok(Self {
            context: self.context,
            vector: DeckLabel { u, v },
            basis: DeckLabelBasis::ExplicitTransport,
        })
    }

    /// Validate this label against an ambient context.
    ///
    /// The label's lattice binding must equal the context's; if the ranks
    /// already disagree the failure is a [`DeckLabelError::RankMismatch`],
    /// otherwise a [`DeckLabelError::LatticeMismatch`]. A vector is never
    /// truncated, padded or reinterpreted across lattices or ranks.
    pub fn validate_for(self, context: DeckContext) -> Result<Self, DeckLabelError> {
        if self.context == context.lattice() {
            Ok(self)
        } else if self.rank() != context.rank() {
            Err(DeckLabelError::RankMismatch {
                expected: context.rank(),
                found: self.rank(),
            })
        } else {
            Err(DeckLabelError::LatticeMismatch {
                expected: context.lattice(),
                found: self.context,
            })
        }
    }
}

// ---------------------------------------------------------------------------
// Canonical incidence identity
// ---------------------------------------------------------------------------

/// The canonical participant slot of one branch within its event's root.
///
/// For the two-participant roots the certified solvers produce, the sides are
/// the two slots of the canonical sorted participant pair
/// ([`super::contact::IsolatedRootKey`]): the branch whose span is the first
/// sorted participant is [`CanonicalBranchSide::First`]. The side is assigned by
/// the producer from that certified identity — never from traversal order,
/// insertion order or a coordinate — so it is stable under operand swap,
/// source traversal reversal and subdivision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CanonicalBranchSide {
    /// The first canonical participant slot.
    First,
    /// The second canonical participant slot.
    Second,
}

/// The canonical construction-based identity of one event incidence, for deck
/// gauge anchor selection and ordering.
///
/// Deliberately **not** parameter evidence: ordering and anchor selection use
/// the stable source occurrence and the canonical branch side, never
/// parameter-enclosure bits, representative points, insertion or discovery
/// order. A self-intersection's distinct roots are distinguished by their
/// certified pair-local ordinal — which the event's own
/// [`super::contact::EventIdentity`] carries — while the two participants of
/// one root are distinguished by [`CanonicalBranchSide`]. Parameter enclosures
/// remain evidence attached to the incidence, never the ordering key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CanonicalIncidenceId {
    /// The stable source occurrence (edge use + source edge).
    pub source_occurrence: SpanId,
    /// The canonical participant slot within the event's root.
    pub side: CanonicalBranchSide,
}

impl CanonicalIncidenceId {
    /// The canonical identity of one incidence.
    pub const fn new(source_occurrence: SpanId, side: CanonicalBranchSide) -> Self {
        Self {
            source_occurrence,
            side,
        }
    }
}

// ---------------------------------------------------------------------------
// Certified deck placement
// ---------------------------------------------------------------------------

/// Why a certified deck placement is not implemented (typed, never guessed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeckPlacementUnsupported {
    /// General rank-2 geometric placement — a certified solver for
    /// `d = m·g₁ + n·g₂` from a displacement enclosure — is deferred. The
    /// existing solver ([`super::deck::solve_axis_aligned`]) is rank-1 and
    /// axis-aligned.
    GeneralRank2PlacementNotImplemented,
}

impl DeckPlacementUnsupported {
    /// A short stable tag, for diagnostics.
    pub fn tag(self) -> &'static str {
        match self {
            Self::GeneralRank2PlacementNotImplemented => "deck_placement_rank2_not_implemented",
        }
    }
}

/// The six-way certified verdict for a deck placement, adapted from the
/// existing four-way [`DeckSolveResult`].
///
/// Nothing here is a nearest-integer round, and no epistemic ambiguity is
/// collapsed into operational failure: `Ambiguous`, `Unresolved` and
/// `OperationalFailure` are three different findings with three different
/// meanings. A [`DeckPlacementResult::Unique`] label is bound to the lattice
/// the placement was certified against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeckPlacementResult {
    /// Exactly one certified deck vector is compatible.
    Unique(CertifiedDeckLabel),
    /// No compatible placement exists.
    Incompatible,
    /// Several compatible placements exist; no arbitrary one is chosen.
    Ambiguous,
    /// The arithmetic evidence cannot decide between the above.
    Unresolved,
    /// The placement is valid geometry outside the admitted machinery.
    Unsupported(DeckPlacementUnsupported),
    /// The computation failed operationally (overflow/resource).
    OperationalFailure(DeckOperationalFailure),
}

impl DeckPlacementResult {
    /// A short stable tag, for diagnostics.
    pub fn tag(self) -> &'static str {
        match self {
            Self::Unique(_) => "deck_placement_unique",
            Self::Incompatible => "deck_placement_incompatible",
            Self::Ambiguous => "deck_placement_ambiguous",
            Self::Unresolved => "deck_placement_unresolved",
            Self::Unsupported(_) => "deck_placement_unsupported",
            Self::OperationalFailure(_) => "deck_placement_operational_failure",
        }
    }
}

/// Adapt the existing four-way deck solver's verdict to the generic placement
/// contract, in the given certified lattice context.
///
/// The mapping is one-to-one: `Unique(k)` becomes one lattice-bound rank-1
/// label, `NoCompatibleInteger` becomes [`DeckPlacementResult::Incompatible`],
/// `MultipleCompatibleIntegers` becomes [`DeckPlacementResult::Ambiguous`]
/// (never an arbitrary label), `Indeterminate` stays
/// [`DeckPlacementResult::Unresolved`], and an operational failure is
/// `OperationalFailure`. This is an adapter over [`solve_axis_aligned`], not a
/// new solver; the caller supplies the certified lattice context the solver
/// ran against, so the unique label is bound to it.
pub fn adapt_axis_aligned_placement(
    context: DeckContext,
    result: Result<DeckSolveResult, DeckOperationalFailure>,
) -> DeckPlacementResult {
    match result {
        Ok(DeckSolveResult::Unique(k)) => DeckPlacementResult::Unique(
            CertifiedDeckLabel::certified_placement(context, DeckLabel::rank1(k)),
        ),
        Ok(DeckSolveResult::NoCompatibleInteger) => DeckPlacementResult::Incompatible,
        Ok(DeckSolveResult::MultipleCompatibleIntegers) => DeckPlacementResult::Ambiguous,
        Ok(DeckSolveResult::Indeterminate) => DeckPlacementResult::Unresolved,
        Err(failure) => DeckPlacementResult::OperationalFailure(failure),
    }
}

/// Certified rank-2 geometric placement: deferred.
///
/// There is no certified solver for `d = m·g₁ + n·g₂` from a displacement
/// enclosure yet — [`super::deck::solve_axis_aligned`] is rank-1 and
/// axis-aligned. Rather than inventing an unreviewed lattice-reduction or
/// closest-vector algorithm, this returns a typed
/// [`DeckPlacementResult::Unsupported`]. A future solver replaces the body
/// without changing the contract.
pub fn certify_rank2_placement() -> DeckPlacementResult {
    DeckPlacementResult::Unsupported(DeckPlacementUnsupported::GeneralRank2PlacementNotImplemented)
}

// ---------------------------------------------------------------------------
// Canonical relative deck signature
// ---------------------------------------------------------------------------

/// The canonical *relative* deck signature of an isolated event.
///
/// Absolute lifts are gauge-dependent: adding one deck vector `h` to every
/// incidence of an event is the same quotient event (`k_i -> k_i + h`), so raw
/// absolute labels must not determine event identity. The canonical signature
/// is the normalized relative label set (each incidence's label minus the
/// anchor's label) in canonical incidence order, invariant under input
/// permutation, operand swap, source traversal reversal, deterministic
/// repetition and common deck translation.
///
/// Constructed only by [`DeckSignature::normalize`], which performs the
/// canonical ordering and gauge normalization itself: a `DeckSignature` in hand
/// is necessarily nonempty, rank-consistent, canonically ordered, and anchored
/// (its first relative label is zero). There is no public constructor that
/// accepts an arbitrary normalized-or-not vector.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeckSignature {
    rank: DeckRank,
    relative: Vec<DeckLabel>,
}

impl DeckSignature {
    /// The canonical signature of a set of incidence-label pairs in one
    /// certified lattice context.
    ///
    /// The single construction route. It validates every label against the
    /// context (a cross-lattice or cross-rank label is a typed error), orders
    /// the incidences canonically by [`CanonicalIncidenceId`], selects the
    /// canonical minimum as the anchor, subtracts the anchor label from every
    /// label, and returns the normalized relative labels in canonical incidence
    /// order. An empty incidence set is [`DeckLabelError::EmptyEvent`]; the
    /// anchor's own relative label is zero by construction.
    pub fn normalize(
        context: DeckContext,
        entries: &[(CanonicalIncidenceId, CertifiedDeckLabel)],
    ) -> Result<Self, DeckLabelError> {
        if entries.is_empty() {
            return Err(DeckLabelError::EmptyEvent);
        }
        for (_, label) in entries {
            label.validate_for(context)?;
        }
        let mut order: Vec<usize> = (0..entries.len()).collect();
        order.sort_by(|&a, &b| entries[a].0.cmp(&entries[b].0));
        let anchor = entries[order[0]].1;
        let mut relative = Vec::with_capacity(entries.len());
        for &index in &order {
            relative.push(entries[index].1.checked_sub(anchor)?.get());
        }
        debug_assert!(
            relative[0].is_zero(),
            "the anchor's relative label is zero by construction"
        );
        Ok(Self {
            rank: context.rank(),
            relative,
        })
    }

    /// The rank of the signature.
    pub fn rank(&self) -> DeckRank {
        self.rank
    }

    /// The normalized relative labels, in canonical incidence order.
    pub fn relative(&self) -> &[DeckLabel] {
        &self.relative
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::source_evidence::{BoundId, EdgeUseId};
    use super::super::curve2d::SourceEdgeId;
    use super::super::deck::{
        solve_axis_aligned, DeckGenerator, DeckInterval, DevelopedAxis, DevelopedBox,
    };
    use super::super::numeric::FiniteF64;
    use super::*;

    /// A synthetic rank-1 lattice id, for tests. Distinct generators get
    /// distinct ids.
    fn rank1_lattice(period: f64) -> AmbientLatticeId {
        AmbientLatticeId::Rank1 {
            periodic_axis: ParameterAxis::V,
            signed_period_bits: period.to_bits(),
        }
    }

    fn rank1_context(period: f64) -> DeckContext {
        DeckContext::from_lattice_id(rank1_lattice(period))
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

    // ----- Deck algebra -----------------------------------------------------

    #[test]
    fn zero_labels_exist_for_every_rank() {
        assert!(CertifiedDeckLabel::zero(DeckContext::rank0()).is_zero());
        assert!(CertifiedDeckLabel::zero(rank1_context(6.28)).is_zero());
        assert!(CertifiedDeckLabel::zero(rank2_context()).is_zero());
        assert_eq!(
            CertifiedDeckLabel::zero(DeckContext::rank0()).get(),
            DeckLabel::ZERO
        );
    }

    #[test]
    fn rank1_addition_subtraction_negation() {
        let context = rank1_context(6.28);
        let a = placement_label(context, 3, 0);
        let b = placement_label(context, -5, 0);
        assert_eq!(a.checked_add(b).unwrap().get(), DeckLabel::rank1(-2));
        assert_eq!(a.checked_sub(b).unwrap().get(), DeckLabel::rank1(8));
        assert_eq!(a.negated().unwrap().get(), DeckLabel::rank1(-3));
    }

    #[test]
    fn rank2_addition_subtraction_negation() {
        let context = rank2_context();
        let a = placement_label(context, 3, -4);
        let b = placement_label(context, -5, 6);
        assert_eq!(a.checked_add(b).unwrap().get(), DeckLabel::rank2(-2, 2));
        assert_eq!(a.checked_sub(b).unwrap().get(), DeckLabel::rank2(8, -10));
        assert_eq!(a.negated().unwrap().get(), DeckLabel::rank2(-3, 4));
    }

    #[test]
    fn checked_overflow_is_a_typed_arithmetic_failure() {
        let context = rank1_context(6.28);
        assert_eq!(
            placement_label(context, i64::MAX, 0).checked_add(placement_label(context, 1, 0)),
            Err(DeckLabelError::ArithmeticOverflow(
                ResourceOperation::DeckVectorAddition
            ))
        );
        assert_eq!(
            placement_label(context, i64::MIN, 0).checked_sub(placement_label(context, 1, 0)),
            Err(DeckLabelError::ArithmeticOverflow(
                ResourceOperation::DeckVectorSubtraction
            ))
        );
        assert_eq!(
            placement_label(context, i64::MIN, 0).negated(),
            Err(DeckLabelError::ArithmeticOverflow(
                ResourceOperation::DeckVectorScaling
            ))
        );
        assert_eq!(
            placement_label(rank2_context(), i64::MAX, 0).checked_add(placement_label(
                rank2_context(),
                1,
                0
            )),
            Err(DeckLabelError::ArithmeticOverflow(
                ResourceOperation::DeckVectorAddition
            ))
        );
    }

    #[test]
    fn cross_rank_algebra_is_rejected() {
        assert!(matches!(
            placement_label(rank1_context(6.28), 1, 0).checked_add(placement_label(
                rank2_context(),
                1,
                1
            )),
            Err(DeckLabelError::RankMismatch { .. })
        ));
    }

    #[test]
    fn cross_lattice_algebra_is_rejected() {
        // Two different rank-1 lattices (different periods): adding their labels
        // is a LatticeMismatch, never a silent reinterpretation.
        assert!(matches!(
            placement_label(rank1_context(6.28), 1, 0).checked_add(placement_label(
                rank1_context(3.14),
                2,
                0
            )),
            Err(DeckLabelError::LatticeMismatch { .. })
        ));
    }

    #[test]
    fn rank_structure_is_structural_not_inferred() {
        let context = rank1_context(6.28);
        // A rank-1 label structurally carries v == 0; a rank-0 label structurally
        // carries zero. There is no constructor that pads, truncates or
        // reinterprets components across ranks.
        assert_eq!(placement_label(context, 7, 0).get().v, 0);
        assert_eq!(CertifiedDeckLabel::zero(context).rank(), DeckRank::Rank1);
        assert!(CertifiedDeckLabel::zero(DeckContext::rank0())
            .get()
            .is_zero());
    }

    #[test]
    fn rank0_label_is_zero_by_type() {
        let label = CertifiedDeckLabel::zero(DeckContext::rank0());
        assert_eq!(label.rank(), DeckRank::Rank0);
        assert_eq!(label.context(), AmbientLatticeId::Rank0);
        assert_eq!(label.get(), DeckLabel::ZERO);
    }

    #[test]
    fn label_rejected_by_a_different_lattice_of_any_rank() {
        let a = placement_label(rank1_context(6.28), 3, 0);
        // Different rank: RankMismatch.
        assert!(matches!(
            a.validate_for(DeckContext::rank0()),
            Err(DeckLabelError::RankMismatch { .. })
        ));
        // Same rank, different lattice: LatticeMismatch.
        assert!(matches!(
            a.validate_for(rank1_context(3.14)),
            Err(DeckLabelError::LatticeMismatch { .. })
        ));
        // Its own lattice: accepted.
        assert_eq!(a.validate_for(rank1_context(6.28)).unwrap().get(), a.get());
    }

    #[test]
    fn rank2_label_validated_only_by_its_own_lattice() {
        let label = placement_label(rank2_context(), 1, 2);
        assert!(matches!(
            label.validate_for(rank1_context(6.28)),
            Err(DeckLabelError::RankMismatch { .. })
        ));
        assert_eq!(label.validate_for(rank2_context()).unwrap(), label);
    }

    #[test]
    fn zero_is_bound_to_its_context() {
        let zero = CertifiedDeckLabel::zero(rank1_context(6.28));
        assert_eq!(zero.basis(), DeckLabelBasis::ZeroByRank);
        assert_eq!(zero.context(), rank1_lattice(6.28));
        // The same zero value in another lattice is a different label.
        assert!(matches!(
            zero.validate_for(rank1_context(3.14)),
            Err(DeckLabelError::LatticeMismatch { .. })
        ));
    }

    #[test]
    fn placement_and_inheritance_record_their_basis() {
        let context = rank1_context(6.28);
        let placed = placement_label(context, 2, 0);
        assert_eq!(placed.basis(), DeckLabelBasis::CertifiedPlacement);
        let inherited = placed.inherited();
        assert_eq!(inherited.basis(), DeckLabelBasis::InheritedFromParent);
        assert_eq!(inherited.get(), placed.get());
        assert_eq!(inherited.context(), placed.context());
        let transported = CertifiedDeckLabel::explicit_transport(context, DeckLabel::rank1(4));
        assert_eq!(transported.basis(), DeckLabelBasis::ExplicitTransport);
        assert_eq!(transported.get(), DeckLabel::rank1(4));
    }

    // ----- Rank-1 adapter ---------------------------------------------------

    #[test]
    fn unique_solver_verdict_becomes_one_bound_label() {
        let context = rank1_context(6.28);
        let result = adapt_axis_aligned_placement(context, Ok(DeckSolveResult::Unique(3)));
        assert_eq!(
            result,
            DeckPlacementResult::Unique(placement_label(context, 3, 0))
        );
        assert_eq!(result.tag(), "deck_placement_unique");
        if let DeckPlacementResult::Unique(label) = result {
            assert_eq!(label.context(), context.lattice());
            assert_eq!(label.basis(), DeckLabelBasis::CertifiedPlacement);
        }
    }

    #[test]
    fn multiple_compatible_integers_never_become_an_arbitrary_label() {
        let result = adapt_axis_aligned_placement(
            rank1_context(6.28),
            Ok(DeckSolveResult::MultipleCompatibleIntegers),
        );
        assert_eq!(result, DeckPlacementResult::Ambiguous);
        assert_eq!(result.tag(), "deck_placement_ambiguous");
    }

    #[test]
    fn indeterminate_evidence_remains_unresolved() {
        let result =
            adapt_axis_aligned_placement(rank1_context(6.28), Ok(DeckSolveResult::Indeterminate));
        assert_eq!(result, DeckPlacementResult::Unresolved);
        assert_eq!(result.tag(), "deck_placement_unresolved");
    }

    #[test]
    fn no_compatible_integer_is_a_certified_incompatibility() {
        let result = adapt_axis_aligned_placement(
            rank1_context(6.28),
            Ok(DeckSolveResult::NoCompatibleInteger),
        );
        assert_eq!(result, DeckPlacementResult::Incompatible);
        assert_eq!(result.tag(), "deck_placement_incompatible");
    }

    #[test]
    fn arithmetic_overflow_is_operational_failure_not_unsupported() {
        let result = adapt_axis_aligned_placement(
            rank1_context(6.28),
            Err(DeckOperationalFailure::ArithmeticOverflow),
        );
        assert_eq!(
            result,
            DeckPlacementResult::OperationalFailure(DeckOperationalFailure::ArithmeticOverflow)
        );
        assert_eq!(result.tag(), "deck_placement_operational_failure");
    }

    #[test]
    fn adapter_consumes_the_actual_axis_aligned_solver() {
        // The adapter sits on the real solver: a certified displacement of one
        // full period is Unique(1); a broad enclosure spanning several periods
        // is Ambiguous, never an arbitrary pick.
        let context = rank1_context(std::f64::consts::TAU);
        let generator = DeckGenerator::new(
            DevelopedAxis::First,
            FiniteF64::new(std::f64::consts::TAU).unwrap(),
        )
        .unwrap();
        let one_period = DevelopedBox {
            first: DeckInterval::from_f64(std::f64::consts::TAU, std::f64::consts::TAU).unwrap(),
            second: DeckInterval::from_f64(0.0, 0.0).unwrap(),
        };
        assert_eq!(
            adapt_axis_aligned_placement(context, solve_axis_aligned(&generator, &one_period)),
            DeckPlacementResult::Unique(placement_label(context, 1, 0))
        );
        let broad = DevelopedBox {
            first: DeckInterval::from_f64(0.0, 3.0 * std::f64::consts::TAU).unwrap(),
            second: DeckInterval::from_f64(0.0, 0.0).unwrap(),
        };
        assert_eq!(
            adapt_axis_aligned_placement(context, solve_axis_aligned(&generator, &broad)),
            DeckPlacementResult::Ambiguous
        );
    }

    // ----- Rank-2 contract --------------------------------------------------

    #[test]
    fn rank2_labels_normalize_like_vectors() {
        // The task's canonical example: [(2,-1),(5,4)] and [(12,9),(15,14)]
        // normalize to the same relative signature under a common translation.
        // The relative computation is exercised end to end on events in
        // `contact::tests`; here the label algebra is checked directly.
        let context = rank2_context();
        let a1 = placement_label(context, 2, -1);
        let a2 = placement_label(context, 5, 4);
        let b1 = placement_label(context, 12, 9);
        let b2 = placement_label(context, 15, 14);
        let rel_a = a2.checked_sub(a1).unwrap();
        let rel_b = b2.checked_sub(b1).unwrap();
        assert_eq!(rel_a, rel_b);
        assert_eq!(rel_a.get(), DeckLabel::rank2(3, 5));
    }

    #[test]
    fn rank2_geometric_placement_is_typed_unsupported_not_guessed() {
        let result = certify_rank2_placement();
        assert_eq!(
            result,
            DeckPlacementResult::Unsupported(
                DeckPlacementUnsupported::GeneralRank2PlacementNotImplemented
            )
        );
        assert_eq!(result.tag(), "deck_placement_unsupported");
        assert!(
            !matches!(result, DeckPlacementResult::Unique(_)),
            "rank-2 placement must never mint a label by guessing"
        );
    }

    // ----- Deck signature ---------------------------------------------------

    fn an_id(span: usize, side: CanonicalBranchSide) -> CanonicalIncidenceId {
        CanonicalIncidenceId::new(
            SpanId {
                edge_use_id: EdgeUseId::new(BoundId(0), span),
                source_edge_id: SourceEdgeId(span),
            },
            side,
        )
    }

    #[test]
    fn signature_rejects_an_empty_event() {
        assert_eq!(
            DeckSignature::normalize(rank1_context(6.28), &[]),
            Err(DeckLabelError::EmptyEvent)
        );
    }

    #[test]
    fn signature_rejects_a_cross_lattice_label() {
        let entries = [
            (
                an_id(0, CanonicalBranchSide::First),
                placement_label(rank1_context(6.28), 0, 0),
            ),
            (
                an_id(1, CanonicalBranchSide::First),
                placement_label(rank1_context(3.14), 1, 0),
            ),
        ];
        assert!(matches!(
            DeckSignature::normalize(rank1_context(6.28), &entries),
            Err(DeckLabelError::LatticeMismatch { .. })
        ));
    }

    #[test]
    fn signature_is_canonically_ordered_and_anchored() {
        let context = rank1_context(6.28);
        // Insertion order is not the canonical order: span 0 is the anchor
        // wherever it appears.
        let entries = [
            (
                an_id(1, CanonicalBranchSide::First),
                placement_label(context, 5, 0),
            ),
            (
                an_id(0, CanonicalBranchSide::First),
                placement_label(context, 3, 0),
            ),
            (
                an_id(2, CanonicalBranchSide::First),
                placement_label(context, 8, 0),
            ),
        ];
        let signature = DeckSignature::normalize(context, &entries).unwrap();
        assert_eq!(signature.rank(), DeckRank::Rank1);
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
    fn signature_is_translation_invariant() {
        let context = rank1_context(6.28);
        let base = [
            (
                an_id(0, CanonicalBranchSide::First),
                placement_label(context, 3, 0),
            ),
            (
                an_id(1, CanonicalBranchSide::First),
                placement_label(context, 5, 0),
            ),
            (
                an_id(2, CanonicalBranchSide::First),
                placement_label(context, 8, 0),
            ),
        ];
        let translated = [
            (
                an_id(0, CanonicalBranchSide::First),
                placement_label(context, -7, 0),
            ),
            (
                an_id(1, CanonicalBranchSide::First),
                placement_label(context, -5, 0),
            ),
            (
                an_id(2, CanonicalBranchSide::First),
                placement_label(context, -2, 0),
            ),
        ];
        assert_eq!(
            DeckSignature::normalize(context, &base).unwrap(),
            DeckSignature::normalize(context, &translated).unwrap()
        );
    }
}
