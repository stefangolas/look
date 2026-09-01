//! The formal complexity envelope and the execution budget.
//!
//! `FORMAL_SYSTEM.md` Definition 6 gives the envelope
//! `β = (r_max, s_max, n_max, e_max, w_max, x_max, v_max, g_max)` and adds:
//! *"The exact numerical values are policy. The closure proof requires only
//! that they are finite."* Definition 11 then makes membership in `E_β` the
//! precondition for Theorem 1's closure claim.
//!
//! # Why there is no `Default`
//!
//! An envelope is a *claim about what has been proved to work*. A default one
//! would be a claim nobody made, silently attached to every call site that
//! omitted the argument — and because `Unsupported` is a judgment about the
//! face, an invented `s_max` would produce invented verdicts. The project
//! documents specify exactly one value, `r_max ≤ 2`; every other bound remains
//! an open policy decision, so callers pass one explicitly or do not proceed.
//! The same argument applies to [`ExecutionBudget`].
//!
//! **Zero is a policy, not a defect.** `max_pair_intersections = 0` says
//! "self-intersecting boundaries are not admitted"; `max_refinement_depth = 0`
//! says "no refinement is permitted". Both are coherent, so the constructors
//! accept them. The only value the constructor refuses is `r_max > 2`, which
//! Definition 6 forbids outright.
//!
//! # Why they are two types
//!
//! They answer different questions and their failures have different
//! classifications:
//!
//! - Exceeding a [`FormalEnvelope`] bound is a **proved property of the face**
//!   → `Unsupported`.
//! - Exhausting an [`ExecutionBudget`] is a **property of this run** → an
//!   `OperationalFailure`, or a named stage-specific `Unresolved`.
//!
//! Merging them would make "we did not look hard enough" indistinguishable
//! from "this face is outside the declared envelope". Only the second is a
//! statement about STEP.
//!
//! # Two things a measurement must carry
//!
//! The boundary convention is stated once, in [`check`], and applied nowhere
//! else: `observed <= maximum` is admitted, `observed > maximum` is outside,
//! and a certified lower bound rejects only when it *strictly* exceeds the
//! maximum. A lower bound equal to the maximum is consistent with the true
//! count being exactly the maximum, which is admitted, so it proves nothing.
//!
//! And every observation names the [`MeasurementSubject`] it counted, checked
//! against the clause it is tested under. Without that, nothing stops an arc
//! count being compared against `x_max`; the numbers would typecheck and the
//! verdict would be meaningless.

use super::evidence::NonEmptyVec;

// ---------------------------------------------------------------------------
// Clauses
// ---------------------------------------------------------------------------

/// A clause of Definition 11 that bounds a *number*.
///
/// Separate from [`FeatureExclusion`], which is categorical: applying the
/// numeric comparison to "surface sheet ambiguity" is not a type error today
/// and should be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NumericEnvelopeClause {
    /// `r_max` — the ambient lattice rank.
    LatticeRank,
    /// `s_max` — certified collapsed strata.
    CollapsedStratumCount,
    /// `n_max` — ordinary native-boundary strata.
    NativeBoundaryCount,
    /// `e_max` — normalized source arcs.
    NormalizedSourceArcCount,
    /// `w_max` — the norm of an arc's deck displacement.
    DeckDisplacementNorm,
    /// `x_max` — certified pairwise intersections.
    PairIntersectionCount,
    /// `v_max` — regular arrangement-vertex valence.
    RegularVertexValence,
    /// `g_max` — arrangement cells or graph elements.
    ArrangementElementCount,
}

impl NumericEnvelopeClause {
    /// The kind of thing this clause counts. A measurement of anything else
    /// may not be tested against it.
    pub fn subject(self) -> MeasurementSubject {
        match self {
            Self::LatticeRank => MeasurementSubject::LatticeRank,
            Self::CollapsedStratumCount => MeasurementSubject::CollapsedStrata,
            Self::NativeBoundaryCount => MeasurementSubject::NativeBoundaryStrata,
            Self::NormalizedSourceArcCount => MeasurementSubject::NormalizedSourceArcs,
            Self::DeckDisplacementNorm => MeasurementSubject::DeckDisplacementNorm,
            Self::PairIntersectionCount => MeasurementSubject::PairIntersections,
            Self::RegularVertexValence => MeasurementSubject::RegularVertexValence,
            Self::ArrangementElementCount => MeasurementSubject::ArrangementElements,
        }
    }

    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::LatticeRank => "lattice_rank",
            Self::CollapsedStratumCount => "collapsed_stratum_count",
            Self::NativeBoundaryCount => "native_boundary_count",
            Self::NormalizedSourceArcCount => "normalized_source_arc_count",
            Self::DeckDisplacementNorm => "deck_displacement_norm",
            Self::PairIntersectionCount => "pair_intersection_count",
            Self::RegularVertexValence => "regular_vertex_valence",
            Self::ArrangementElementCount => "arrangement_element_count",
        }
    }
}

/// A categorical exclusion of `FORMAL_SYSTEM.md` Definition 10.
///
/// **Deliberately short.** Definition 10's list also contains
/// "noncertified tangential contacts", "surface sheet ambiguity" and
/// "unresolved curve overlaps", and those are *epistemic*: whether such a face
/// is `Unsupported`, `Unresolved` or `Ambiguous` depends on predicates the
/// later stages have not yet defined. Classifying them as envelope exclusions
/// now would fix that answer before the question has been posed, so they are
/// omitted until the stage that decides them exists.
///
/// What remains are the exclusions that are categorical whatever the later
/// predicates turn out to be: an infinite or nonisolated population, or an
/// unbounded quantity, is outside a finite envelope by Definition 6's own
/// finiteness requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FeatureExclusion {
    /// Infinite intersection populations. Lemma 3's finiteness proof requires
    /// each candidate pair to have at most `x_max` intersections.
    InfiniteIntersectionPopulation,
    /// Nonisolated intersections, which have no finite certified parameter set
    /// to split an arc at.
    NonisolatedIntersection,
    /// Unbounded winding, which admits no finite deck displacement.
    UnboundedWinding,
    /// Unbounded curve enclosures, which break Definition 15's requirement of
    /// a *compact* enclosure and with it Lemma 1.
    UnboundedCurveEnclosure,
}

impl FeatureExclusion {
    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::InfiniteIntersectionPopulation => "infinite_intersection_population",
            Self::NonisolatedIntersection => "nonisolated_intersection",
            Self::UnboundedWinding => "unbounded_winding",
            Self::UnboundedCurveEnclosure => "unbounded_curve_enclosure",
        }
    }
}

/// A categorical exclusion together with the proof that it applies.
///
/// The witness is what distinguishes "we believe this face winds unboundedly"
/// from "here is the divergence". No Step 1 code constructs one; the type is
/// defined so the first stage that can prove such a thing has somewhere to put
/// the proof.
#[derive(Debug, Clone, PartialEq)]
pub struct FeatureExclusionWitness {
    exclusion: FeatureExclusion,
    grounds: NonEmptyVec<ExclusionGround>,
}

impl FeatureExclusionWitness {
    /// Build a witness.
    ///
    /// `pub(super)`: a caller able to write
    /// `ExclusionGround::EnclosureProvedUnbounded` without an enclosure proof
    /// could exclude any face it liked. Nonempty grounds are necessary and not
    /// sufficient — an exclusion with no grounds is an assertion, and an
    /// exclusion with forged grounds is a forged one. The certifier module
    /// that can actually prove these does not exist yet; when it does, it
    /// constructs them here. Unused until then.
    #[allow(dead_code)]
    pub(super) fn new(exclusion: FeatureExclusion, grounds: NonEmptyVec<ExclusionGround>) -> Self {
        Self { exclusion, grounds }
    }

    /// Which exclusion.
    pub fn exclusion(&self) -> FeatureExclusion {
        self.exclusion
    }

    /// The grounds.
    pub fn grounds(&self) -> &NonEmptyVec<ExclusionGround> {
        &self.grounds
    }
}

/// A ground on which a categorical exclusion was proved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExclusionGround {
    /// A certified enclosure was found to be unbounded.
    EnclosureProvedUnbounded,
    /// An intersection set was proved to contain a positive-length interval.
    IntersectionSetProvedNonisolated,
}

// ---------------------------------------------------------------------------
// Measurements
// ---------------------------------------------------------------------------

/// What a measurement counted.
///
/// Checked against a clause's own [`NumericEnvelopeClause::subject`] before
/// any comparison, so a normalized-arc count cannot be tested against
/// `x_max`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MeasurementSubject {
    /// The rank of the ambient deck lattice.
    LatticeRank,
    /// Certified collapsed strata.
    CollapsedStrata,
    /// Ordinary native-boundary strata.
    NativeBoundaryStrata,
    /// Normalized source arcs.
    NormalizedSourceArcs,
    /// The norm of a deck displacement.
    DeckDisplacementNorm,
    /// Certified pairwise intersections.
    PairIntersections,
    /// The valence of a regular arrangement vertex.
    RegularVertexValence,
    /// Arrangement cells or graph elements.
    ArrangementElements,
}

impl MeasurementSubject {
    /// A short stable tag.
    pub fn tag(self) -> &'static str {
        match self {
            Self::LatticeRank => "lattice_rank",
            Self::CollapsedStrata => "collapsed_strata",
            Self::NativeBoundaryStrata => "native_boundary_strata",
            Self::NormalizedSourceArcs => "normalized_source_arcs",
            Self::DeckDisplacementNorm => "deck_displacement_norm",
            Self::PairIntersections => "pair_intersections",
            Self::RegularVertexValence => "regular_vertex_valence",
            Self::ArrangementElements => "arrangement_elements",
        }
    }
}

/// How a count was obtained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CountingProcedure {
    /// The quantity is determined by the resolved structure itself and needed
    /// no search — a certified lattice's rank, for instance.
    StructuralFromResolvedType,
    /// Every element was enumerated to completion.
    ExhaustiveEnumeration,
    /// Enumeration was halted before completion.
    HaltedEnumeration,
}

/// An exactly counted quantity, with its subject and procedure.
///
/// Private fields: an `ExactCount` asserts that a complete count was performed,
/// and a struct literal would let a caller assert it without having done so.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExactCount {
    value: u128,
    subject: MeasurementSubject,
    procedure: CountingProcedure,
}

impl ExactCount {
    /// Record an exact count.
    ///
    /// `pub(super)`: this is a **proof introduction rule**, not a data
    /// constructor. Calling it asserts that a complete count was performed,
    /// and a public version would let any caller assert that — producing a
    /// forged [`EnvelopeViolation`] and with it a false `Unsupported` verdict
    /// about a face. Only the counting routines inside the `formal` subtree
    /// may claim to have counted.
    ///
    /// The procedure is still checked: a halted enumeration by definition did
    /// not finish, so it cannot yield an exact count even from a trusted
    /// caller.
    pub(super) fn from_completed_count(
        value: u128,
        subject: MeasurementSubject,
        procedure: CountingProcedure,
    ) -> Result<Self, MeasurementError> {
        match procedure {
            CountingProcedure::HaltedEnumeration => {
                Err(MeasurementError::HaltedEnumerationIsNotExact { subject })
            }
            CountingProcedure::StructuralFromResolvedType
            | CountingProcedure::ExhaustiveEnumeration => Ok(Self {
                value,
                subject,
                procedure,
            }),
        }
    }

    /// The count.
    pub fn value(self) -> u128 {
        self.value
    }

    /// What was counted.
    pub fn subject(self) -> MeasurementSubject {
        self.subject
    }

    /// How.
    pub fn procedure(self) -> CountingProcedure {
        self.procedure
    }
}

/// A certified lower bound on a quantity, with the proof that it is one.
///
/// Private fields for the same reason as [`ExactCount`], and one extra check:
/// the certificate has to actually support the claimed bound. "At least 10,000"
/// backed by an enumeration that produced 3 elements is not a lower bound of
/// 10,000.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CertifiedLowerBoundCount {
    value: u128,
    subject: MeasurementSubject,
    certificate: LowerBoundCertificate,
}

impl CertifiedLowerBoundCount {
    /// Record a certified lower bound.
    ///
    /// `pub(super)` for the same reason as
    /// [`ExactCount::from_completed_count`]. The consistency check below —
    /// that the claimed bound does not exceed what the certificate states —
    /// is necessary and *not sufficient*: nothing in a `LowerBoundCertificate`
    /// value proves that the enumeration it describes actually ran. Only the
    /// routine that ran it may say so, which is what the visibility enforces.
    ///
    /// Unused: no Step 1 procedure halts an enumeration, because Step 1
    /// enumerates nothing. The rule is defined so the first stage that does
    /// has a checked way to report a partial count.
    #[allow(dead_code)]
    pub(super) fn from_certificate(
        value: u128,
        subject: MeasurementSubject,
        certificate: LowerBoundCertificate,
    ) -> Result<Self, MeasurementError> {
        let established = certificate.established_count();
        match established >= value {
            true => Ok(Self {
                value,
                subject,
                certificate,
            }),
            false => Err(MeasurementError::CertificateDoesNotSupportBound {
                subject,
                claimed: value,
                established,
            }),
        }
    }

    /// The bound.
    pub fn value(self) -> u128 {
        self.value
    }

    /// What was bounded.
    pub fn subject(self) -> MeasurementSubject {
        self.subject
    }

    /// The proof.
    pub fn certificate(self) -> LowerBoundCertificate {
        self.certificate
    }
}

/// How a lower bound was proved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LowerBoundCertificate {
    /// Enumeration was halted after producing this many distinct elements.
    /// Each one is a witness, so the true count is at least this.
    PartialEnumerationHalted {
        /// How many distinct elements were produced.
        enumerated: u128,
    },
    /// This many pairwise-distinct witnesses were exhibited by another means.
    DistinctWitnessesExhibited {
        /// How many.
        witnesses: u128,
    },
}

impl LowerBoundCertificate {
    /// The count the certificate actually establishes.
    pub fn established_count(self) -> u128 {
        match self {
            Self::PartialEnumerationHalted { enumerated } => enumerated,
            Self::DistinctWitnessesExhibited { witnesses } => witnesses,
        }
    }
}

/// Why a measurement could not be recorded. No catch-all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MeasurementError {
    /// An enumeration that stopped early cannot yield an exact count.
    HaltedEnumerationIsNotExact {
        /// What was being counted.
        subject: MeasurementSubject,
    },
    /// The certificate establishes fewer elements than the bound claims.
    CertificateDoesNotSupportBound {
        /// What was being counted.
        subject: MeasurementSubject,
        /// The bound claimed.
        claimed: u128,
        /// What the certificate proves.
        established: u128,
    },
}

/// What is known about a measured quantity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BoundObservation {
    /// The quantity was counted exactly.
    Exact(ExactCount),
    /// The quantity is at least this, and counting stopped.
    CertifiedLowerBound(CertifiedLowerBoundCount),
}

impl BoundObservation {
    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::Exact(_) => "exact",
            Self::CertifiedLowerBound(_) => "certified_lower_bound",
        }
    }

    /// The number observed.
    pub fn value(self) -> u128 {
        match self {
            Self::Exact(count) => count.value(),
            Self::CertifiedLowerBound(bound) => bound.value(),
        }
    }

    /// What was measured.
    pub fn subject(self) -> MeasurementSubject {
        match self {
            Self::Exact(count) => count.subject(),
            Self::CertifiedLowerBound(bound) => bound.subject(),
        }
    }
}

/// A clause of the envelope, proved exceeded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EnvelopeViolation {
    clause: NumericEnvelopeClause,
    maximum: u128,
    observation: BoundObservation,
}

impl EnvelopeViolation {
    /// Which clause.
    pub fn clause(self) -> NumericEnvelopeClause {
        self.clause
    }

    /// The permitted maximum.
    pub fn maximum(self) -> u128 {
        self.maximum
    }

    /// What was observed.
    pub fn observation(self) -> BoundObservation {
        self.observation
    }
}

/// Why a clause could not be checked at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClauseCheckError {
    /// The observation measured something the clause does not bound.
    SubjectDoesNotMatchClause {
        /// The clause applied.
        clause: NumericEnvelopeClause,
        /// What it bounds.
        expected: MeasurementSubject,
        /// What was measured.
        observed: MeasurementSubject,
    },
}

/// The single implementation of the boundary convention.
///
/// Private: every clause is reached through its own typed method on
/// [`FormalEnvelope`], so no call site chooses which maximum to compare a
/// number against.
fn check(
    clause: NumericEnvelopeClause,
    maximum: u128,
    observation: BoundObservation,
) -> Result<Result<(), EnvelopeViolation>, ClauseCheckError> {
    if observation.subject() != clause.subject() {
        return Err(ClauseCheckError::SubjectDoesNotMatchClause {
            clause,
            expected: clause.subject(),
            observed: observation.subject(),
        });
    }
    // Exact and lower-bound reject on the same strict comparison, for
    // different reasons: an exact count above the maximum *is* the excess, and
    // a lower bound above the maximum excludes every completion. Equality
    // rejects in neither case.
    let exceeds = observation.value() > maximum;
    Ok(match exceeds {
        true => Err(EnvelopeViolation {
            clause,
            maximum,
            observation,
        }),
        false => Ok(()),
    })
}

// ---------------------------------------------------------------------------
// Policies
// ---------------------------------------------------------------------------

/// Definition 6 fixes `r_max <= 2`. The one numeric value the documents state.
pub const DEFINITION_6_MAX_LATTICE_RANK: u8 = 2;

/// An identifier for one policy instance, so provenance can cite the exact
/// policy a judgment was made under rather than "a policy".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PolicyInstanceId(u64);

impl PolicyInstanceId {
    /// Name a policy instance.
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// The identifier.
    pub fn get(self) -> u64 {
        self.0
    }
}

/// The bounds `β` of `FORMAL_SYSTEM.md` Definition 6.
///
/// No `Default`, private fields, and one checked constructor; see the module
/// documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormalEnvelope {
    instance: PolicyInstanceId,
    max_lattice_rank: u8,
    max_collapsed_strata: usize,
    max_native_boundary_strata: usize,
    max_normalized_source_arcs: usize,
    max_deck_displacement_norm: u64,
    max_pair_intersections: usize,
    max_regular_vertex_valence: usize,
    max_arrangement_elements: usize,
}

/// Why an envelope constructor refused. No catch-all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyConstructionError {
    /// `r_max > 2`, which Definition 6 forbids.
    LatticeRankAboveDefinitionMaximum {
        /// What was requested.
        requested: u8,
        /// What Definition 6 permits.
        permitted: u8,
    },
}

impl std::fmt::Display for PolicyConstructionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LatticeRankAboveDefinitionMaximum {
                requested,
                permitted,
            } => write!(
                f,
                "max_lattice_rank {requested} exceeds Definition 6's r_max <= {permitted}"
            ),
        }
    }
}

impl std::error::Error for PolicyConstructionError {}

impl FormalEnvelope {
    /// The only constructor. Every bound is stated by the caller; nothing is
    /// defaulted, and zero is accepted wherever it denotes a coherent policy.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instance: PolicyInstanceId,
        max_lattice_rank: u8,
        max_collapsed_strata: usize,
        max_native_boundary_strata: usize,
        max_normalized_source_arcs: usize,
        max_deck_displacement_norm: u64,
        max_pair_intersections: usize,
        max_regular_vertex_valence: usize,
        max_arrangement_elements: usize,
    ) -> Result<Self, PolicyConstructionError> {
        if max_lattice_rank > DEFINITION_6_MAX_LATTICE_RANK {
            return Err(PolicyConstructionError::LatticeRankAboveDefinitionMaximum {
                requested: max_lattice_rank,
                permitted: DEFINITION_6_MAX_LATTICE_RANK,
            });
        }
        Ok(Self {
            instance,
            max_lattice_rank,
            max_collapsed_strata,
            max_native_boundary_strata,
            max_normalized_source_arcs,
            max_deck_displacement_norm,
            max_pair_intersections,
            max_regular_vertex_valence,
            max_arrangement_elements,
        })
    }

    /// Which policy instance this is.
    pub fn instance(&self) -> PolicyInstanceId {
        self.instance
    }

    /// `r_max`.
    pub fn max_lattice_rank(&self) -> u8 {
        self.max_lattice_rank
    }

    /// `s_max`.
    pub fn max_collapsed_strata(&self) -> usize {
        self.max_collapsed_strata
    }

    /// `n_max`.
    pub fn max_native_boundary_strata(&self) -> usize {
        self.max_native_boundary_strata
    }

    /// `e_max`.
    pub fn max_normalized_source_arcs(&self) -> usize {
        self.max_normalized_source_arcs
    }

    /// `w_max`.
    pub fn max_deck_displacement_norm(&self) -> u64 {
        self.max_deck_displacement_norm
    }

    /// `x_max`.
    pub fn max_pair_intersections(&self) -> usize {
        self.max_pair_intersections
    }

    /// `v_max`.
    pub fn max_regular_vertex_valence(&self) -> usize {
        self.max_regular_vertex_valence
    }

    /// `g_max`.
    pub fn max_arrangement_elements(&self) -> usize {
        self.max_arrangement_elements
    }

    /// Test a proved lattice rank against `r_max`.
    pub fn check_lattice_rank(
        &self,
        observation: BoundObservation,
    ) -> Result<Result<(), EnvelopeViolation>, ClauseCheckError> {
        check(
            NumericEnvelopeClause::LatticeRank,
            u128::from(self.max_lattice_rank),
            observation,
        )
    }

    /// Test a deck displacement norm against `w_max`.
    pub fn check_deck_displacement_norm(
        &self,
        observation: BoundObservation,
    ) -> Result<Result<(), EnvelopeViolation>, ClauseCheckError> {
        check(
            NumericEnvelopeClause::DeckDisplacementNorm,
            u128::from(self.max_deck_displacement_norm),
            observation,
        )
    }

    /// Test a normalized source arc count against `e_max`.
    pub fn check_normalized_source_arcs(
        &self,
        observation: BoundObservation,
    ) -> Result<Result<(), EnvelopeViolation>, ClauseCheckError> {
        check(
            NumericEnvelopeClause::NormalizedSourceArcCount,
            self.max_normalized_source_arcs as u128,
            observation,
        )
    }

    /// Test a certified pairwise intersection count against `x_max`.
    pub fn check_pair_intersections(
        &self,
        observation: BoundObservation,
    ) -> Result<Result<(), EnvelopeViolation>, ClauseCheckError> {
        check(
            NumericEnvelopeClause::PairIntersectionCount,
            self.max_pair_intersections as u128,
            observation,
        )
    }

    /// Test a collapsed stratum count against `s_max`.
    pub fn check_collapsed_strata(
        &self,
        observation: BoundObservation,
    ) -> Result<Result<(), EnvelopeViolation>, ClauseCheckError> {
        check(
            NumericEnvelopeClause::CollapsedStratumCount,
            self.max_collapsed_strata as u128,
            observation,
        )
    }

    /// Test a native-boundary stratum count against `n_max`.
    pub fn check_native_boundary_strata(
        &self,
        observation: BoundObservation,
    ) -> Result<Result<(), EnvelopeViolation>, ClauseCheckError> {
        check(
            NumericEnvelopeClause::NativeBoundaryCount,
            self.max_native_boundary_strata as u128,
            observation,
        )
    }

    /// Test a regular vertex valence against `v_max`.
    pub fn check_regular_vertex_valence(
        &self,
        observation: BoundObservation,
    ) -> Result<Result<(), EnvelopeViolation>, ClauseCheckError> {
        check(
            NumericEnvelopeClause::RegularVertexValence,
            self.max_regular_vertex_valence as u128,
            observation,
        )
    }

    /// Test an arrangement element count against `g_max`.
    pub fn check_arrangement_elements(
        &self,
        observation: BoundObservation,
    ) -> Result<Result<(), EnvelopeViolation>, ClauseCheckError> {
        check(
            NumericEnvelopeClause::ArrangementElementCount,
            self.max_arrangement_elements as u128,
            observation,
        )
    }
}

/// How much work this run is allowed to do.
///
/// Not part of [`FormalEnvelope`], no `Default`, private fields. Nothing in
/// Step 1 consumes a budget — resolving ambient periods is a finite match over
/// evidence already in hand — but the type is defined here so that the first
/// stage that does need one cannot invent a different resource vocabulary.
///
/// Zero is accepted throughout: `max_refinement_depth = 0` is the coherent
/// policy "do not refine".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionBudget {
    instance: PolicyInstanceId,
    max_projection_samples: usize,
    max_refinement_depth: u32,
    max_candidate_pairs_examined: usize,
    max_working_cover_copies_allocated: usize,
    max_solver_iterations: usize,
}

impl ExecutionBudget {
    /// The only constructor.
    pub fn new(
        instance: PolicyInstanceId,
        max_projection_samples: usize,
        max_refinement_depth: u32,
        max_candidate_pairs_examined: usize,
        max_working_cover_copies_allocated: usize,
        max_solver_iterations: usize,
    ) -> Self {
        Self {
            instance,
            max_projection_samples,
            max_refinement_depth,
            max_candidate_pairs_examined,
            max_working_cover_copies_allocated,
            max_solver_iterations,
        }
    }

    /// Which policy instance this is.
    pub fn instance(&self) -> PolicyInstanceId {
        self.instance
    }

    /// Projection samples permitted.
    pub fn max_projection_samples(&self) -> usize {
        self.max_projection_samples
    }

    /// Refinement depth permitted.
    pub fn max_refinement_depth(&self) -> u32 {
        self.max_refinement_depth
    }

    /// Candidate pairs permitted to be examined.
    pub fn max_candidate_pairs_examined(&self) -> usize {
        self.max_candidate_pairs_examined
    }

    /// Working-cover copies permitted to be allocated.
    pub fn max_working_cover_copies_allocated(&self) -> usize {
        self.max_working_cover_copies_allocated
    }

    /// Solver iterations permitted.
    pub fn max_solver_iterations(&self) -> usize {
        self.max_solver_iterations
    }
}

#[cfg(test)]
mod tests {
    use super::super::evidence::SemanticStage;
    use super::super::outcome::{ExecutionResource, OperationalFailure, ResourceOperation};
    use super::*;

    /// A policy for tests only. It is *not* a production value: no project
    /// document specifies these, and Step 1 deliberately does not invent them.
    fn a_test_envelope() -> FormalEnvelope {
        FormalEnvelope::new(
            PolicyInstanceId::new(1),
            2,
            4,
            64,
            4096,
            16,
            64,
            32,
            1 << 20,
        )
        .expect("test policy is well-formed")
    }

    fn exact(value: u128, subject: MeasurementSubject) -> BoundObservation {
        BoundObservation::Exact(
            ExactCount::from_completed_count(
                value,
                subject,
                CountingProcedure::ExhaustiveEnumeration,
            )
            .expect("an exhaustive enumeration yields an exact count"),
        )
    }

    fn lower_bound(value: u128, subject: MeasurementSubject) -> BoundObservation {
        BoundObservation::CertifiedLowerBound(
            CertifiedLowerBoundCount::from_certificate(
                value,
                subject,
                LowerBoundCertificate::PartialEnumerationHalted { enumerated: value },
            )
            .expect("the certificate supports the bound"),
        )
    }

    #[test]
    fn the_envelope_has_no_default_and_caps_the_lattice_rank() {
        assert_eq!(a_test_envelope().max_lattice_rank(), 2);
        assert_eq!(
            FormalEnvelope::new(
                PolicyInstanceId::new(1),
                3,
                4,
                64,
                4096,
                16,
                64,
                32,
                1 << 20
            ),
            Err(PolicyConstructionError::LatticeRankAboveDefinitionMaximum {
                requested: 3,
                permitted: 2,
            }),
            "Definition 6 caps r_max at 2"
        );
    }

    #[test]
    fn zero_is_a_coherent_policy() {
        // "No self-intersections admitted", "no refinement permitted", "no
        // collapsed strata admitted" are all policies a caller may mean.
        let envelope =
            FormalEnvelope::new(PolicyInstanceId::new(2), 0, 0, 0, 0, 0, 0, 0, 0).unwrap();
        assert_eq!(envelope.max_pair_intersections(), 0);
        assert_eq!(
            envelope
                .check_pair_intersections(exact(0, MeasurementSubject::PairIntersections))
                .unwrap(),
            Ok(()),
            "zero observed against a zero bound is admitted"
        );
        assert!(envelope
            .check_pair_intersections(exact(1, MeasurementSubject::PairIntersections))
            .unwrap()
            .is_err());

        let budget = ExecutionBudget::new(PolicyInstanceId::new(2), 0, 0, 0, 0, 0);
        assert_eq!(budget.max_refinement_depth(), 0);
    }

    #[test]
    fn value_equal_to_limit_is_admitted() {
        let envelope = a_test_envelope();
        assert_eq!(
            envelope
                .check_pair_intersections(exact(64, MeasurementSubject::PairIntersections))
                .unwrap(),
            Ok(())
        );
        assert_eq!(
            envelope
                .check_lattice_rank(exact(2, MeasurementSubject::LatticeRank))
                .unwrap(),
            Ok(())
        );
        assert_eq!(
            envelope
                .check_deck_displacement_norm(exact(16, MeasurementSubject::DeckDisplacementNorm))
                .unwrap(),
            Ok(())
        );
        assert_eq!(
            envelope
                .check_normalized_source_arcs(exact(4096, MeasurementSubject::NormalizedSourceArcs))
                .unwrap(),
            Ok(())
        );
        assert_eq!(
            envelope
                .check_collapsed_strata(exact(4, MeasurementSubject::CollapsedStrata))
                .unwrap(),
            Ok(())
        );
        assert_eq!(
            envelope
                .check_native_boundary_strata(exact(64, MeasurementSubject::NativeBoundaryStrata))
                .unwrap(),
            Ok(())
        );
        assert_eq!(
            envelope
                .check_regular_vertex_valence(exact(32, MeasurementSubject::RegularVertexValence))
                .unwrap(),
            Ok(())
        );
        assert_eq!(
            envelope
                .check_arrangement_elements(exact(1 << 20, MeasurementSubject::ArrangementElements))
                .unwrap(),
            Ok(())
        );
    }

    #[test]
    fn value_one_above_limit_is_unsupported() {
        let envelope = a_test_envelope();
        let violation = envelope
            .check_pair_intersections(exact(65, MeasurementSubject::PairIntersections))
            .unwrap()
            .expect_err("65 > 64");
        assert_eq!(
            violation.clause(),
            NumericEnvelopeClause::PairIntersectionCount
        );
        assert_eq!(violation.maximum(), 64);
        assert_eq!(violation.observation().value(), 65);

        assert!(envelope
            .check_lattice_rank(exact(3, MeasurementSubject::LatticeRank))
            .unwrap()
            .is_err());
        assert!(envelope
            .check_deck_displacement_norm(exact(17, MeasurementSubject::DeckDisplacementNorm))
            .unwrap()
            .is_err());
        assert!(envelope
            .check_normalized_source_arcs(exact(4097, MeasurementSubject::NormalizedSourceArcs))
            .unwrap()
            .is_err());
    }

    #[test]
    fn lower_bound_equal_to_limit_does_not_reject() {
        // "At least 64" is consistent with "exactly 64", which is admitted, so
        // the partial count proves nothing and must not exclude the face.
        assert_eq!(
            a_test_envelope()
                .check_pair_intersections(lower_bound(64, MeasurementSubject::PairIntersections))
                .unwrap(),
            Ok(())
        );
    }

    #[test]
    fn lower_bound_one_above_limit_rejects() {
        // "At least 65" is inconsistent with "at most 64" for every
        // completion, so the partial count is a proof.
        let violation = a_test_envelope()
            .check_pair_intersections(lower_bound(65, MeasurementSubject::PairIntersections))
            .unwrap()
            .expect_err("65 > 64 for every completion");
        assert_eq!(violation.observation().tag(), "certified_lower_bound");
    }

    #[test]
    fn a_measurement_cannot_be_tested_against_the_wrong_clause() {
        // An arc count compared against `x_max` would typecheck as a number
        // and mean nothing. The subject check makes it an error.
        assert_eq!(
            a_test_envelope()
                .check_pair_intersections(exact(1, MeasurementSubject::NormalizedSourceArcs))
                .expect_err("wrong subject"),
            ClauseCheckError::SubjectDoesNotMatchClause {
                clause: NumericEnvelopeClause::PairIntersectionCount,
                expected: MeasurementSubject::PairIntersections,
                observed: MeasurementSubject::NormalizedSourceArcs,
            }
        );
    }

    #[test]
    fn a_measurement_is_internally_consistent() {
        // A halted enumeration cannot yield an exact count...
        assert_eq!(
            ExactCount::from_completed_count(
                10,
                MeasurementSubject::PairIntersections,
                CountingProcedure::HaltedEnumeration
            ),
            Err(MeasurementError::HaltedEnumerationIsNotExact {
                subject: MeasurementSubject::PairIntersections,
            })
        );
        // ...and a lower bound cannot exceed what its certificate states.
        assert_eq!(
            CertifiedLowerBoundCount::from_certificate(
                10_000,
                MeasurementSubject::PairIntersections,
                LowerBoundCertificate::PartialEnumerationHalted { enumerated: 3 },
            ),
            Err(MeasurementError::CertificateDoesNotSupportBound {
                subject: MeasurementSubject::PairIntersections,
                claimed: 10_000,
                established: 3,
            })
        );
        // A weaker claim than the certificate supports is fine.
        assert!(CertifiedLowerBoundCount::from_certificate(
            3,
            MeasurementSubject::PairIntersections,
            LowerBoundCertificate::PartialEnumerationHalted { enumerated: 7 },
        )
        .is_ok());
    }

    /// The consistency checks above are necessary and **not sufficient**: a
    /// `LowerBoundCertificate` value does not prove its enumeration ran, so
    /// the remaining guarantee has to come from visibility.
    ///
    /// This test is a statement of the property, verified by the compiler
    /// rather than at runtime. Every proof-introducing constructor here is
    /// `pub(super)`, so the equivalent call from outside the `formal` subtree
    /// — from `triangulation.rs`, from `look`, from a downstream crate — does
    /// not compile. `tessellation/formal/mod.rs` re-exports the *types* and
    /// none of these constructors, which is what makes the restriction hold at
    /// the crate boundary too.
    ///
    /// The corresponding negative case belongs in a `trybuild` compile-fail
    /// fixture; adding that harness is a dev-dependency decision this step
    /// does not take, and it is recorded as the one unproven half.
    #[test]
    fn certified_observations_are_not_publicly_forgeable() {
        // Reachable here, because this module is inside `formal`.
        let inside = ExactCount::from_completed_count(
            1_000_000,
            MeasurementSubject::PairIntersections,
            CountingProcedure::ExhaustiveEnumeration,
        );
        assert!(inside.is_ok());

        // The three proof-introduction rules, named so a refactor that widens
        // any of them to `pub` has to come through this test.
        //
        //   ExactCount::from_completed_count        pub(super)
        //   CertifiedLowerBoundCount::from_certificate  pub(super)
        //   FeatureExclusionWitness::new            pub(super)
        //
        // and the types they gate, which have no other constructor:
        //   EnvelopeViolation  — built only by the private `check`
        //   BoundObservation   — inhabited only by the two above
        let witness = FeatureExclusionWitness::new(
            FeatureExclusion::UnboundedCurveEnclosure,
            NonEmptyVec::one(ExclusionGround::EnclosureProvedUnbounded),
        );
        assert_eq!(
            witness.exclusion(),
            FeatureExclusion::UnboundedCurveEnclosure
        );
    }

    #[test]
    fn arithmetic_overflow_is_operational_not_unsupported() {
        // An `i64` that cannot hold a sum is a fact about the machine's word
        // size. It is not evidence that the face's deck displacement exceeds
        // `w_max`, and the envelope is not consulted.
        let failure = OperationalFailure::ArithmeticOverflow {
            operation: ResourceOperation::DeckVectorAddition,
        };
        assert_eq!(failure.tag(), "arithmetic_overflow");
        assert!(!matches!(
            failure,
            OperationalFailure::ExecutionBudgetExhausted { .. }
        ));
        assert_eq!(
            a_test_envelope()
                .check_deck_displacement_norm(exact(1, MeasurementSubject::DeckDisplacementNorm))
                .unwrap(),
            Ok(())
        );
    }

    #[test]
    fn execution_budget_exhaustion_does_not_prove_unsupported() {
        // A budget of one iteration is legal, and running out of it must not
        // reclassify the geometry: the observation the envelope would need was
        // never obtained, and there is no constructor turning exhaustion into
        // an `EnvelopeViolation`.
        let budget = ExecutionBudget::new(PolicyInstanceId::new(3), 1, 1, 1, 1, 1);
        assert_eq!(budget.max_solver_iterations(), 1);
        let failure = OperationalFailure::ExecutionBudgetExhausted {
            stage: SemanticStage::AmbientPeriodResolution,
            resource: ExecutionResource::SolverIterations,
            consumed: 1,
            limit: 1,
        };
        assert_eq!(failure.tag(), "execution_budget_exhausted");
    }

    #[test]
    fn a_feature_exclusion_needs_grounds_and_is_not_numeric() {
        // Categorical exclusions have no maximum to compare against, which is
        // why they are a separate type from `NumericEnvelopeClause` and cannot
        // be passed to a numeric check at all.
        let witness = FeatureExclusionWitness::new(
            FeatureExclusion::UnboundedCurveEnclosure,
            NonEmptyVec::one(ExclusionGround::EnclosureProvedUnbounded),
        );
        assert_eq!(witness.exclusion().tag(), "unbounded_curve_enclosure");
        assert_eq!(witness.grounds().len(), 1);
    }
}
