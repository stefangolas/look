//! Four result layers, kept apart on purpose.
//!
//! `FORMAL_SYSTEM.md` Definition 3 fixes the semantic algebra:
//!
//! ```text
//! SemanticOutcome ::= Valid(R) | Inconsistent(I) | Ambiguous(A)
//!                   | Unsupported(U) | Unresolved(N)
//! ```
//!
//! and Definition 4 immediately adds a *separate* judgment:
//!
//! ```text
//! RealizationOutcome ::= Realized(M) | RecognizedButUnsupported(R_u)
//!                      | RealizationFailure(R_f)
//! ```
//!
//! Neither describes a stage, and neither describes the machine. Collapsing
//! these into one type is the failure this module exists to prevent:
//!
//! - **A stage is not a face.** Resolving a face's ambient period says nothing
//!   about whether its region is valid; §XI is explicit that even a unique
//!   material labelling does not imply a valid region. A successful stage
//!   returns [`StageOutcome::Resolved`] carrying that stage's product.
//!
//! - **A failure of the machine is not a fact about the geometry.** An
//!   allocator refusal, an `i64` overflow or an exhausted budget are facts
//!   about *this run*. Reporting one as `Unsupported` would assert that the
//!   face lies outside the declared envelope, which is a claim about the face
//!   and is not what was observed. [`OperationalFailure`] is the `Err` of
//!   [`StageEvaluation`], never a variant of an outcome, and there is no
//!   `From<OperationalFailure>` for `StageOutcome`.
//!
//! - **A missing mesher is not an invalid face.** Definition 4's separation is
//!   why [`RealizationOutcome`] carries the [`ValidSemantic`] through all three
//!   of its variants: a realization failure leaves semantic validity intact.
//!
//! # Reports are constructed, not composed
//!
//! Every report has private fields and a checked constructor. Public fields
//! would allow an `UnsupportedReport` naming a pair-intersection clause while
//! carrying a lattice-rank witness — two halves of two different judgments,
//! typechecking fine and meaning nothing. Each report is built from a single
//! authoritative cause and derives whatever else it exposes.

use super::ambient::{AmbientAlternative, PeriodContradictionWitness};
use super::envelope::{
    EnvelopeViolation, FeatureExclusionWitness, NumericEnvelopeClause, PolicyInstanceId,
};
use super::evidence::{
    AnalyticRule, AtLeastTwo, NonEmptyVec, NumericalMethod, ParameterAxis, PredicateDescription,
    ResolutionAttempt, SemanticStage, SourceEntityKey, SourceFieldPath, SurfaceAccessor,
};

// ---------------------------------------------------------------------------
// Identity
// ---------------------------------------------------------------------------

/// Which document a face came from.
///
/// `look` currently converts one document per run and no document identity
/// survives conversion, so the honest representation is an enum rather than a
/// fabricated id: [`Self::SingleDocumentRun`] states that the run had one
/// document and that nothing named it, which is a fact, whereas a
/// `DocumentKey(0)` would be an invented one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DocumentScope {
    /// The document is identified.
    Identified(DocumentKey),
    /// The run processed a single document that carries no retained identity.
    SingleDocumentRun,
}

impl DocumentScope {
    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::Identified(_) => "identified",
            Self::SingleDocumentRun => "single_document_run",
        }
    }
}

/// A document identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DocumentKey(u64);

impl DocumentKey {
    /// Name a document.
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// The identifier.
    pub fn get(self) -> u64 {
        self.0
    }
}

/// Which shell within a document.
///
/// Required for uniqueness: `declared_face_index` is an index *within a shell*
/// and collides between shells, so a key without a shell is not a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ShellKey(u64);

impl ShellKey {
    /// Name a shell by its ordinal within the document.
    pub fn new(ordinal: u64) -> Self {
        Self(ordinal)
    }

    /// The ordinal.
    pub fn get(self) -> u64 {
        self.0
    }
}

/// Which face a judgment is about, uniquely.
///
/// All four components. `source_face_id` alone is insufficient because
/// conversion loses it for some faces; `declared_face_index` alone is
/// insufficient because it is per-shell; the pair alone is insufficient
/// because it is per-document. Ordering is derived so a census can sort
/// deterministically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FaceKey {
    /// Which document.
    pub document: DocumentScope,
    /// Which shell within it.
    pub shell: ShellKey,
    /// The STEP entity id, when conversion retained one.
    pub source_face_id: Option<SourceEntityKey>,
    /// The face's position in its shell.
    pub declared_face_index: usize,
}

impl PartialOrd for DocumentScope {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DocumentScope {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Self::Identified(a), Self::Identified(b)) => a.cmp(b),
            (Self::Identified(_), Self::SingleDocumentRun) => std::cmp::Ordering::Less,
            (Self::SingleDocumentRun, Self::Identified(_)) => std::cmp::Ordering::Greater,
            (Self::SingleDocumentRun, Self::SingleDocumentRun) => std::cmp::Ordering::Equal,
        }
    }
}

// ---------------------------------------------------------------------------
// Provenance
// ---------------------------------------------------------------------------

/// The derivation chain behind a judgment.
///
/// Ordered and nonempty. The document and face are on the report itself, so
/// they are not repeated per record; each record names one link in the chain
/// from source to conclusion.
#[derive(Debug, Clone, PartialEq)]
pub struct ProvenanceSet {
    chain: NonEmptyVec<ProvenanceRecord>,
}

impl ProvenanceSet {
    /// From at least one link.
    pub fn new(chain: NonEmptyVec<ProvenanceRecord>) -> Self {
        Self { chain }
    }

    /// From exactly one link.
    pub fn one(link: ProvenanceRecord) -> Self {
        Self {
            chain: NonEmptyVec::one(link),
        }
    }

    /// The chain, in derivation order.
    pub fn iter(&self) -> impl Iterator<Item = &ProvenanceRecord> {
        self.chain.iter()
    }

    /// How many links. Always at least one.
    pub fn len(&self) -> usize {
        self.chain.len()
    }

    /// Always false.
    pub fn is_empty(&self) -> bool {
        false
    }
}

/// One link in a derivation chain.
///
/// Each variant identifies the concrete thing consulted, not a category of
/// thing: which axis of which accessor, which entity and field, which analytic
/// rule, which numerical method, which policy instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProvenanceRecord {
    /// One axis was read from a `domain::lattice::CertifiedLattice` whose
    /// constructor is not recorded by that type.
    LegacyLatticeAxis {
        /// Which axis.
        axis: ParameterAxis,
        /// Through which accessor.
        accessor: SurfaceAccessor,
    },
    /// A field of a named source entity was read from the support-surface
    /// schema.
    SupportSurfaceSchema {
        /// Which entity.
        entity: SourceEntityKey,
        /// Which field.
        field: SourceFieldPath,
    },
    /// A named analytic rule was applied.
    AnalyticRuleApplied {
        /// Which rule.
        rule: AnalyticRule,
    },
    /// A numerical certificate was applied.
    NumericalCertificateApplied {
        /// By which method.
        method: NumericalMethod,
        /// About which proposition.
        predicate: PredicateDescription,
    },
    /// A policy instance was consulted.
    PolicyInstance {
        /// Which one.
        policy: PolicyInstanceId,
    },
}

impl ProvenanceRecord {
    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::LegacyLatticeAxis { .. } => "legacy_lattice_axis",
            Self::SupportSurfaceSchema { .. } => "support_surface_schema",
            Self::AnalyticRuleApplied { .. } => "analytic_rule_applied",
            Self::NumericalCertificateApplied { .. } => "numerical_certificate_applied",
            Self::PolicyInstance { .. } => "policy_instance",
        }
    }
}

// ---------------------------------------------------------------------------
// Intermediate stage result
// ---------------------------------------------------------------------------

/// What one formal stage concluded.
///
/// `Resolved(T)` carries *that stage's* product. It is not a claim that the
/// face is valid; see the module documentation.
#[derive(Debug, Clone, PartialEq)]
pub enum StageOutcome<T> {
    /// The stage established its product.
    Resolved(T),
    /// The evidence contradicts itself.
    Inconsistent(InconsistencyReport),
    /// Two or more inequivalent readings are each admissible.
    Ambiguous(AmbiguityReport),
    /// The face is proved to lie outside the declared envelope.
    Unsupported(UnsupportedReport),
    /// A required proposition was not established. This is a statement about
    /// the *evidence*, not about the face.
    Unresolved(UnresolvedReport),
}

impl<T> StageOutcome<T> {
    /// The resolved product, if any. Diagnostic; there is deliberately no
    /// `unwrap_or`-shaped accessor.
    pub fn resolved(&self) -> Option<&T> {
        match self {
            Self::Resolved(value) => Some(value),
            _ => None,
        }
    }

    /// A short stable tag, for probe records.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::Resolved(_) => "resolved",
            Self::Inconsistent(_) => "inconsistent",
            Self::Ambiguous(_) => "ambiguous",
            Self::Unsupported(_) => "unsupported",
            Self::Unresolved(_) => "unresolved",
        }
    }
}

/// A stage evaluation: a semantic judgment, or a failure of the machine.
///
/// Note what is *not* here: no `impl From<OperationalFailure> for
/// StageOutcome<_>`, so `?` cannot quietly turn an overflow into a verdict
/// about the geometry.
pub type StageEvaluation<T> = Result<StageOutcome<T>, OperationalFailure>;

// ---------------------------------------------------------------------------
// Final semantic outcome
// ---------------------------------------------------------------------------

/// The final per-face algebra of `FORMAL_SYSTEM.md` Definition 3.
///
/// Defined now, and unreachable now. Its purpose in Step 1 is to fix the
/// target: a later stage that invented its own five-way result type would have
/// to be reconciled with this one afterwards, and Theorem 1's exhaustiveness
/// claim is about *this* algebra.
#[derive(Debug, Clone, PartialEq)]
pub enum SemanticOutcome<R> {
    /// A canonical valid region complex. Not constructible in Step 1.
    Valid(ValidSemantic<R>),
    /// The evidence contradicts itself.
    Inconsistent(InconsistencyReport),
    /// Two or more inequivalent readings are each admissible.
    Ambiguous(AmbiguityReport),
    /// The face lies outside the declared envelope.
    Unsupported(UnsupportedReport),
    /// A required proposition was not established.
    Unresolved(UnresolvedReport),
}

/// A region that has discharged every validity obligation of
/// `FORMAL_SYSTEM.md` Definition 25.
///
/// **There is no public constructor, and Step 1 does not construct one.** All
/// fields are private and no `pub fn new` exists, so the only code that could
/// produce this is code inside this module — which today is none. In
/// particular, a face the legacy tessellator meshed successfully is *not* a
/// `Valid`: nothing in the legacy path checks Def. 25.
///
/// The certificate field is the reason this cannot be relaxed later into a
/// newtype. Definition 25 lists seven obligations; the validity stage is the
/// only code able to discharge them, so it must be the only code able to
/// produce the token that says they were discharged.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidSemantic<R> {
    #[allow(dead_code)]
    region: R,
    #[allow(dead_code)]
    validity_certificate: ValidityCertificate,
    #[allow(dead_code)]
    provenance: ProvenanceSet,
}

impl<R> ValidSemantic<R> {
    /// The region.
    pub fn region(&self) -> &R {
        &self.region
    }

    /// The proof that Definition 25's obligations were discharged.
    pub fn validity_certificate(&self) -> &ValidityCertificate {
        &self.validity_certificate
    }

    /// The derivation chain.
    pub fn provenance(&self) -> &ProvenanceSet {
        &self.provenance
    }
}

/// The obligations of `FORMAL_SYSTEM.md` Definition 25, each discharged.
///
/// Private fields and no constructor reachable from outside this module. The
/// stage that checks these is not written; when it is, its constructor goes
/// here and nowhere else.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidityCertificate {
    #[allow(dead_code)]
    discharged: NonEmptyVec<ValidityObligation>,
}

impl ValidityCertificate {
    /// The obligations discharged.
    pub fn discharged(&self) -> &NonEmptyVec<ValidityObligation> {
        &self.discharged
    }
}

/// One obligation of Definition 25.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValidityObligation {
    /// Incidence is internally consistent.
    IncidenceConsistent,
    /// The selected regular part is an orientable two-manifold with boundary.
    SelectedPartIsOrientableTwoManifold,
    /// Every singular link is permitted by its stratum's schema.
    SingularLinksPermitted,
    /// The selected quotient region is compact.
    SelectedRegionCompact,
    /// Every physical boundary is represented.
    PhysicalBoundaryRepresented,
    /// Artificial boundaries are paired.
    ArtificialBoundariesPaired,
    /// No unresolved or unsupported relation remains.
    NoUnresolvedRelationRemains,
}

// ---------------------------------------------------------------------------
// Realization
// ---------------------------------------------------------------------------

/// The realization algebra of `FORMAL_SYSTEM.md` Definition 4.
///
/// Separate from [`SemanticOutcome`], and every variant carries the
/// [`ValidSemantic`] unchanged: *"A semantically valid region need not yet
/// have an implemented mesher"*, and a mesher that then fails has not made the
/// region invalid. Nothing in Step 1 produces one.
#[derive(Debug, Clone, PartialEq)]
pub enum RealizationOutcome<R, M> {
    /// A mesh was produced.
    Realized {
        /// The semantics, unchanged.
        semantic: ValidSemantic<R>,
        /// The mesh.
        mesh: M,
        /// The proof that the mesh realizes the region.
        certificate: RealizationCertificate,
    },
    /// The region is understood and no mesher covers it.
    RecognizedButUnsupported {
        /// The semantics, unchanged.
        semantic: ValidSemantic<R>,
        /// Which capability is missing.
        reason: RealizationUnsupported,
    },
    /// A mesher was applicable and did not succeed.
    Failure {
        /// The semantics, unchanged. A realization failure is not a semantic
        /// judgment and must not be reported as one.
        semantic: ValidSemantic<R>,
        /// What went wrong.
        failure: RealizationFailure,
    },
}

/// The proof that a mesh realizes its region. Not constructible in Step 1.
#[derive(Debug, Clone, PartialEq)]
pub struct RealizationCertificate {
    #[allow(dead_code)]
    obligations: NonEmptyVec<RealizationObligation>,
}

/// One obligation a realization certificate discharges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RealizationObligation {
    /// Every physical boundary edge of the region appears in the mesh.
    BoundaryEdgesRepresented,
    /// The mesh is within the declared approximation of the region.
    WithinDeclaredApproximation,
}

/// Why no mesher covers a recognized region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RealizationUnsupported {
    /// No cut-open plan is implemented for this region's normal form.
    NoCutOpenPlanImplemented,
    /// No mesher is implemented for this ambient schema.
    NoMesherForAmbientSchema,
}

/// Why an applicable mesher did not succeed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RealizationFailure {
    /// The constrained triangulation refused a required constraint.
    ConstraintInsertionRefused,
    /// The cut-open plan could not be executed.
    CutOpenPlanExecutionFailed,
}

// ---------------------------------------------------------------------------
// Operational failure
// ---------------------------------------------------------------------------

/// A failure of the implementation, not a judgment about the face.
///
/// Every variant names a resource or an invariant. None may be converted into
/// a [`StageOutcome`], and none may cause a fall back to legacy behaviour
/// inside the formal API: an entry point that answered "the legacy code got
/// this far" would be reporting a different proposition than the one asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationalFailure {
    /// Checked integer arithmetic refused. An `i64` deck coordinate
    /// overflowing says nothing about whether the face's deck displacement
    /// lies within `w_max` — that is a separate, provable question, and this
    /// is what happens when the machine could not carry out the count.
    ArithmeticOverflow {
        /// Which operation overflowed.
        operation: ResourceOperation,
    },

    /// A planned allocation was refused *by policy* before being attempted.
    ///
    /// Distinct from [`Self::AllocatorFailure`]: this is the resource policy
    /// working as designed, and the memory may well have been available.
    /// `FORMAL_SYSTEM.md` Definition 6 keeps `g_max` — an implementation
    /// resource bound — separate from the semantic bounds beside it, so this
    /// is not `Unsupported` either.
    PlannedAllocationExceedsBudget {
        /// Which operation asked.
        operation: ResourceOperation,
        /// How much it planned to allocate.
        requested: usize,
        /// How much the policy permits.
        permitted: usize,
    },

    /// The allocator itself failed. A fact about the machine, with no policy
    /// involved and nothing to tune.
    AllocatorFailure {
        /// Which operation asked.
        operation: ResourceOperation,
        /// How much it asked for.
        requested: usize,
    },

    /// An execution budget ran out before the stage could conclude.
    ///
    /// A low budget cannot prove anything about the face. Setting the budget
    /// to zero must not reclassify the corpus as `Unsupported`.
    ExecutionBudgetExhausted {
        /// Where it ran out.
        stage: SemanticStage,
        /// Which resource ran out.
        resource: ExecutionResource,
        /// How much was consumed.
        consumed: u128,
        /// The limit.
        limit: u128,
    },

    /// An internal invariant of this implementation did not hold.
    ///
    /// Not `Unresolved`: `Unresolved` means the *evidence* was insufficient,
    /// which is a defensible statement about the input. A broken invariant is
    /// a defect here and must not be laundered into a statement about STEP.
    InternalInvariantViolation {
        /// Where.
        stage: SemanticStage,
        /// Which invariant.
        invariant: InvariantId,
    },
}

impl OperationalFailure {
    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::ArithmeticOverflow { .. } => "arithmetic_overflow",
            Self::PlannedAllocationExceedsBudget { .. } => "planned_allocation_exceeds_budget",
            Self::AllocatorFailure { .. } => "allocator_failure",
            Self::ExecutionBudgetExhausted { .. } => "execution_budget_exhausted",
            Self::InternalInvariantViolation { .. } => "internal_invariant_violation",
        }
    }
}

/// The operations that can fail operationally in Step 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceOperation {
    /// Adding two deck displacement vectors.
    DeckVectorAddition,
    /// Subtracting two deck displacement vectors.
    DeckVectorSubtraction,
    /// Scaling a deck displacement vector by an integer.
    DeckVectorScaling,
    /// Computing the norm of a deck displacement.
    DeckDisplacementNorm,
}

/// Countable execution resources.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExecutionResource {
    /// Projection samples taken.
    ProjectionSamples,
    /// Refinement depth reached.
    RefinementDepth,
    /// Candidate pairs examined.
    CandidatePairsExamined,
    /// Working-cover copies allocated.
    WorkingCoverCopies,
    /// Solver iterations run.
    SolverIterations,
}

/// The internal invariants Step 1 asserts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InvariantId {
    /// A deck displacement was offered whose rank does not match the lattice
    /// it was offered to.
    DeckVectorRankMatchesLattice,
    /// A certified rank-1 lattice named a periodic axis that its generator
    /// does not lie on.
    Rank1GeneratorLiesOnPeriodicAxis,
    /// A certified rank-2 lattice's two generators lie on distinct axes.
    Rank2GeneratorsLieOnDistinctAxes,
    /// A measurement this code constructed itself was tested against a clause
    /// bounding a different subject. Unreachable unless the construction and
    /// the check disagree, which would be a defect here.
    MeasurementSubjectMatchesClause,
}

// ---------------------------------------------------------------------------
// Reports
// ---------------------------------------------------------------------------

/// The evidence contradicts itself.
///
/// Private fields. The `reason` is *derived* from the witness rather than
/// supplied beside it, so a report cannot name one contradiction and exhibit
/// another.
#[derive(Debug, Clone, PartialEq)]
pub struct InconsistencyReport {
    face: FaceKey,
    stage: SemanticStage,
    reason: Inconsistency,
    witness: ContradictionWitness,
    provenance: ProvenanceSet,
}

impl InconsistencyReport {
    /// Build a report from its witness. The named reason is whatever the
    /// witness establishes; there is no way to disagree with it.
    pub fn from_witness(
        face: FaceKey,
        stage: SemanticStage,
        witness: ContradictionWitness,
        provenance: ProvenanceSet,
    ) -> Self {
        Self {
            face,
            stage,
            reason: witness.established_inconsistency(),
            witness,
            provenance,
        }
    }

    /// Which face.
    pub fn face(&self) -> FaceKey {
        self.face
    }

    /// Which stage.
    pub fn stage(&self) -> SemanticStage {
        self.stage
    }

    /// The named contradiction, derived from the witness.
    pub fn reason(&self) -> Inconsistency {
        self.reason
    }

    /// The two facts that cannot both hold.
    pub fn witness(&self) -> ContradictionWitness {
        self.witness
    }

    /// The derivation chain.
    pub fn provenance(&self) -> &ProvenanceSet {
        &self.provenance
    }
}

/// Two or more inequivalent readings are each admissible.
///
/// Private fields, and the constructor checks the thing that makes an
/// ambiguity report meaningful: that the inequivalence witness actually
/// separates the alternatives it is attached to.
#[derive(Debug, Clone, PartialEq)]
pub struct AmbiguityReport {
    face: FaceKey,
    stage: SemanticStage,
    reason: Ambiguity,
    alternatives: AtLeastTwo<SemanticAlternative>,
    inequivalence: InequivalenceWitness,
    provenance: ProvenanceSet,
}

impl AmbiguityReport {
    /// Build a report, checking that the witness separates the first two
    /// alternatives.
    ///
    /// Without this check an "ambiguity" may be two encodings of one answer,
    /// which `FORMAL_SYSTEM.md` Definition 27's gauge transformations make a
    /// live possibility rather than a hypothetical one.
    pub fn new(
        face: FaceKey,
        stage: SemanticStage,
        reason: Ambiguity,
        alternatives: AtLeastTwo<SemanticAlternative>,
        inequivalence: InequivalenceWitness,
        provenance: ProvenanceSet,
    ) -> Result<Self, ReportConstructionError> {
        let (first, second) = alternatives.pair();
        if !inequivalence.separates(first, second) {
            return Err(ReportConstructionError::WitnessDoesNotSeparateAlternatives);
        }
        Ok(Self {
            face,
            stage,
            reason,
            alternatives,
            inequivalence,
            provenance,
        })
    }

    /// Which face.
    pub fn face(&self) -> FaceKey {
        self.face
    }

    /// Which stage.
    pub fn stage(&self) -> SemanticStage {
        self.stage
    }

    /// The named ambiguity.
    pub fn reason(&self) -> Ambiguity {
        self.reason
    }

    /// The alternatives, at least two.
    pub fn alternatives(&self) -> &AtLeastTwo<SemanticAlternative> {
        &self.alternatives
    }

    /// Why they are genuinely different readings.
    pub fn inequivalence(&self) -> &InequivalenceWitness {
        &self.inequivalence
    }

    /// The derivation chain.
    pub fn provenance(&self) -> &ProvenanceSet {
        &self.provenance
    }
}

/// The face is proved to lie outside the declared envelope.
///
/// One authoritative cause. The clause, the maximum and the observation all
/// live inside the [`EnvelopeViolation`] the cause carries, so the report
/// cannot name a clause its witness does not concern.
#[derive(Debug, Clone, PartialEq)]
pub struct UnsupportedReport {
    face: FaceKey,
    stage: SemanticStage,
    cause: UnsupportedCause,
    provenance: ProvenanceSet,
}

impl UnsupportedReport {
    /// Build a report from its single authoritative cause.
    pub fn new(
        face: FaceKey,
        stage: SemanticStage,
        cause: UnsupportedCause,
        provenance: ProvenanceSet,
    ) -> Self {
        Self {
            face,
            stage,
            cause,
            provenance,
        }
    }

    /// Which face.
    pub fn face(&self) -> FaceKey {
        self.face
    }

    /// Which stage.
    pub fn stage(&self) -> SemanticStage {
        self.stage
    }

    /// The cause.
    pub fn cause(&self) -> &UnsupportedCause {
        &self.cause
    }

    /// The numeric clause exceeded, when the cause is a numeric one. Derived
    /// from the cause rather than stored beside it.
    pub fn numeric_clause(&self) -> Option<NumericEnvelopeClause> {
        match &self.cause {
            UnsupportedCause::EnvelopeExceeded(violation) => Some(violation.clause()),
            UnsupportedCause::ExplicitFeatureExcluded(_) => None,
        }
    }

    /// The derivation chain.
    pub fn provenance(&self) -> &ProvenanceSet {
        &self.provenance
    }
}

/// The single authoritative representation of why a face is unsupported.
#[derive(Debug, Clone, PartialEq)]
pub enum UnsupportedCause {
    /// A numeric clause of Definition 11 was proved exceeded.
    EnvelopeExceeded(EnvelopeViolation),
    /// A categorical exclusion of Definition 10 was proved to apply.
    ExplicitFeatureExcluded(FeatureExclusionWitness),
}

impl UnsupportedCause {
    /// A short stable tag, for probe records.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::EnvelopeExceeded(violation) => violation.clause().tag(),
            Self::ExplicitFeatureExcluded(witness) => witness.exclusion().tag(),
        }
    }
}

/// A required proposition was not established.
///
/// Private fields, and the attempts are nonempty: an `UnresolvedReport` is
/// constructible only after at least one permitted resolution method was
/// considered or attempted. That is weaker than "somebody tried hard" and it
/// is what the type can honestly enforce — the *outcome* of each attempt
/// distinguishes an erased-evidence dead end from an unimplemented method.
#[derive(Debug, Clone, PartialEq)]
pub struct UnresolvedReport {
    face: FaceKey,
    stage: SemanticStage,
    predicate: PredicateDescription,
    reason: UnresolvedReason,
    attempts: NonEmptyVec<ResolutionAttempt>,
    provenance: ProvenanceSet,
}

impl UnresolvedReport {
    /// Build a report.
    pub fn new(
        face: FaceKey,
        stage: SemanticStage,
        predicate: PredicateDescription,
        reason: UnresolvedReason,
        attempts: NonEmptyVec<ResolutionAttempt>,
        provenance: ProvenanceSet,
    ) -> Self {
        Self {
            face,
            stage,
            predicate,
            reason,
            attempts,
            provenance,
        }
    }

    /// Which face.
    pub fn face(&self) -> FaceKey {
        self.face
    }

    /// Which stage.
    pub fn stage(&self) -> SemanticStage {
        self.stage
    }

    /// Which proposition.
    pub fn predicate(&self) -> PredicateDescription {
        self.predicate
    }

    /// The named reason.
    pub fn reason(&self) -> UnresolvedReason {
        self.reason
    }

    /// What was tried.
    pub fn attempts(&self) -> &NonEmptyVec<ResolutionAttempt> {
        &self.attempts
    }

    /// The derivation chain.
    pub fn provenance(&self) -> &ProvenanceSet {
        &self.provenance
    }
}

/// Why a report could not be built. No catch-all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReportConstructionError {
    /// The inequivalence witness does not separate the alternatives it was
    /// attached to, so the "ambiguity" may be one answer twice.
    WitnessDoesNotSeparateAlternatives,
}

// ---------------------------------------------------------------------------
// Reason enums
// ---------------------------------------------------------------------------

/// The contradictions Step 1 can name.
///
/// No `Other`, `Unknown`, `Misc`, `Unexpected` or `Custom(String)`. A
/// contradiction this stage cannot name is a contradiction it has not proved,
/// and the honest answer for that is `Unresolved`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Inconsistency {
    /// One axis is asserted both periodic and aperiodic.
    PeriodEvidenceContradiction,
    /// A certified generator disagrees with the declared period on its axis.
    PeriodGeneratorContradiction,
    /// Two generators claimed independent are not.
    PeriodGeneratorDependenceContradiction,
}

impl Inconsistency {
    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::PeriodEvidenceContradiction => "period_evidence_contradiction",
            Self::PeriodGeneratorContradiction => "period_generator_contradiction",
            Self::PeriodGeneratorDependenceContradiction => {
                "period_generator_dependence_contradiction"
            }
        }
    }
}

/// The ambiguities Step 1 can name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ambiguity {
    /// The period evidence admits two inequivalent ambient readings.
    AmbientPeriodInterpretation,
}

impl Ambiguity {
    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::AmbientPeriodInterpretation => "ambient_period_interpretation",
        }
    }
}

/// The unresolved propositions Step 1 can name.
///
/// Every one is a statement about *missing evidence*. None asserts anything
/// about the surface. [`Self::PeriodAbsenceNotEstablished`] is the honest
/// reading of the state this codebase currently calls `NonPeriodic` when it
/// arrived from a bare accessor returning `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnresolvedReason {
    /// A period was declared and nothing certified it as a deck generator.
    DeclaredPeriodNotCertified,
    /// No evidence establishes that the axis has no period.
    PeriodAbsenceNotEstablished,
    /// A generator was needed and none is certified.
    PeriodGeneratorNotCertified,
    /// Two generators exist and their independence is not certified.
    GeneratorIndependenceNotCertified,
}

impl UnresolvedReason {
    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::DeclaredPeriodNotCertified => "declared_period_not_certified",
            Self::PeriodAbsenceNotEstablished => "period_absence_not_established",
            Self::PeriodGeneratorNotCertified => "period_generator_not_certified",
            Self::GeneratorIndependenceNotCertified => "generator_independence_not_certified",
        }
    }
}

// ---------------------------------------------------------------------------
// Witnesses and alternatives
// ---------------------------------------------------------------------------

/// The two facts that cannot both hold.
///
/// A witness, not a message: `FORMAL_SYSTEM.md` §XVI separates what is proved
/// from what is assumed, and a proved contradiction must exhibit its two sides.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ContradictionWitness {
    /// The contradiction lies in one axis's period evidence.
    AmbientPeriod(PeriodContradictionWitness),
}

impl ContradictionWitness {
    /// The [`Inconsistency`] this witness establishes. An
    /// [`InconsistencyReport`]'s reason is always this value.
    pub fn established_inconsistency(self) -> Inconsistency {
        match self {
            Self::AmbientPeriod(witness) => witness.inconsistency(),
        }
    }
}

/// One admissible reading of a face, structurally.
///
/// Not a hash. A `u64` digest cannot prove that two readings differ — equal
/// digests may collide and unequal digests say nothing about *how* the
/// readings differ, which is what the inequivalence witness has to establish.
/// A digest is acceptable as diagnostic metadata beside the structure and not
/// in place of it.
#[derive(Debug, Clone, PartialEq)]
pub enum SemanticAlternative {
    /// A reading of the ambient lattice.
    Ambient(AmbientAlternative),
}

impl SemanticAlternative {
    /// The lattice rank this reading would give the face.
    pub fn lattice_rank(&self) -> u8 {
        match self {
            Self::Ambient(alternative) => alternative.rank(),
        }
    }
}

/// Why two alternatives are genuinely different readings.
///
/// Each variant is checked by [`Self::separates`] against the alternatives it
/// is attached to, so the witness cannot be asserted about a pair it does not
/// actually separate.
#[derive(Debug, Clone, PartialEq)]
pub enum InequivalenceWitness {
    /// They assign the face different ambient lattice ranks, which no admitted
    /// gauge transformation of Definition 27 can reconcile.
    DistinctRank {
        /// One reading's rank.
        first: u8,
        /// The other's.
        second: u8,
    },
    /// Same rank, different certified generators.
    DistinctCertifiedGenerator {
        /// The axis on which they differ.
        axis: ParameterAxis,
    },
    /// Same generators, different subgroup of the deck group.
    DistinctGeneratorSubgroup {
        /// The index of one subgroup in the other, where finite.
        index: u64,
    },
}

impl InequivalenceWitness {
    /// Whether this witness actually separates the two alternatives.
    pub fn separates(&self, first: &SemanticAlternative, second: &SemanticAlternative) -> bool {
        match self {
            Self::DistinctRank {
                first: a,
                second: b,
            } => a != b && first.lattice_rank() == *a && second.lattice_rank() == *b,
            // A generator or subgroup difference is a claim about readings of
            // the same rank; a rank difference would be the stronger witness
            // and should have been used instead.
            Self::DistinctCertifiedGenerator { .. } | Self::DistinctGeneratorSubgroup { .. } => {
                first.lattice_rank() == second.lattice_rank() && first != second
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::ambient::AmbientAlternative;
    use super::super::evidence::{AttemptOutcome, FormalPredicate, ResolutionMethod};
    use super::*;

    fn a_face() -> FaceKey {
        FaceKey {
            document: DocumentScope::SingleDocumentRun,
            shell: ShellKey::new(0),
            source_face_id: Some(SourceEntityKey::new(42)),
            declared_face_index: 7,
        }
    }

    #[test]
    fn a_face_key_separates_shells() {
        // The defect a per-shell index alone would have: two faces at index 7
        // in different shells are different faces.
        let first = a_face();
        let second = FaceKey {
            shell: ShellKey::new(1),
            source_face_id: None,
            ..first
        };
        assert_ne!(first, second);
        let third = FaceKey {
            source_face_id: None,
            ..first
        };
        assert_ne!(second, third, "same index, different shell");
    }

    #[test]
    fn an_unresolved_report_names_a_predicate_and_at_least_one_attempt() {
        let report = UnresolvedReport::new(
            a_face(),
            SemanticStage::AmbientPeriodResolution,
            PredicateDescription::of(FormalPredicate::AmbientAxisIsAperiodic(ParameterAxis::U)),
            UnresolvedReason::PeriodAbsenceNotEstablished,
            NonEmptyVec::one(ResolutionAttempt {
                method: ResolutionMethod::LegacyCertifiedLatticeAccessor,
                outcome: AttemptOutcome::EvidenceErasedBeforeThisStage,
            }),
            ProvenanceSet::one(ProvenanceRecord::LegacyLatticeAxis {
                axis: ParameterAxis::U,
                accessor: SurfaceAccessor::UPeriod,
            }),
        );
        assert_eq!(report.attempts().len(), 1);
        assert_eq!(report.provenance().len(), 1);
        assert_eq!(report.reason().tag(), "period_absence_not_established");
    }

    #[test]
    fn an_ambiguity_witness_must_separate_its_alternatives() {
        let provenance = ProvenanceSet::one(ProvenanceRecord::PolicyInstance {
            policy: PolicyInstanceId::new(1),
        });
        let rank0 = SemanticAlternative::Ambient(AmbientAlternative::Rank0);
        let rank1 = SemanticAlternative::Ambient(AmbientAlternative::Rank1 {
            axis: ParameterAxis::V,
        });

        // A rank witness that matches the alternatives is accepted.
        assert!(AmbiguityReport::new(
            a_face(),
            SemanticStage::AmbientPeriodResolution,
            Ambiguity::AmbientPeriodInterpretation,
            AtLeastTwo::two(rank0.clone(), rank1.clone()),
            InequivalenceWitness::DistinctRank {
                first: 0,
                second: 1
            },
            provenance.clone(),
        )
        .is_ok());

        // One that does not is refused: this pair is not separated by a claim
        // about ranks 1 and 2.
        assert_eq!(
            AmbiguityReport::new(
                a_face(),
                SemanticStage::AmbientPeriodResolution,
                Ambiguity::AmbientPeriodInterpretation,
                AtLeastTwo::two(rank0.clone(), rank1),
                InequivalenceWitness::DistinctRank {
                    first: 1,
                    second: 2
                },
                provenance.clone(),
            )
            .expect_err("the witness concerns other ranks"),
            ReportConstructionError::WitnessDoesNotSeparateAlternatives
        );

        // And two identical alternatives are never separated.
        assert_eq!(
            AmbiguityReport::new(
                a_face(),
                SemanticStage::AmbientPeriodResolution,
                Ambiguity::AmbientPeriodInterpretation,
                AtLeastTwo::two(rank0.clone(), rank0),
                InequivalenceWitness::DistinctRank {
                    first: 0,
                    second: 0
                },
                provenance,
            )
            .expect_err("one answer twice is not an ambiguity"),
            ReportConstructionError::WitnessDoesNotSeparateAlternatives
        );
    }

    #[test]
    fn an_unsupported_report_has_one_cause() {
        // The clause is derived from the cause, so the report cannot name a
        // clause its witness does not concern.
        let witness = FeatureExclusionWitness::new(
            super::super::envelope::FeatureExclusion::UnboundedCurveEnclosure,
            NonEmptyVec::one(super::super::envelope::ExclusionGround::EnclosureProvedUnbounded),
        );
        let report = UnsupportedReport::new(
            a_face(),
            SemanticStage::AmbientPeriodResolution,
            UnsupportedCause::ExplicitFeatureExcluded(witness),
            ProvenanceSet::one(ProvenanceRecord::PolicyInstance {
                policy: PolicyInstanceId::new(1),
            }),
        );
        assert_eq!(report.numeric_clause(), None);
        assert_eq!(report.cause().tag(), "unbounded_curve_enclosure");
    }

    /// The classification rule, stated as a test so a later refactor that adds
    /// a `From` impl breaks here first.
    #[test]
    fn an_operational_failure_is_not_a_stage_outcome() {
        let failure = OperationalFailure::ExecutionBudgetExhausted {
            stage: SemanticStage::AmbientPeriodResolution,
            resource: ExecutionResource::SolverIterations,
            consumed: 10,
            limit: 10,
        };
        let evaluation: StageEvaluation<u8> = Err(failure);
        assert!(evaluation.is_err());
        assert_eq!(failure.tag(), "execution_budget_exhausted");
    }

    #[test]
    fn allocation_failures_are_split_by_cause() {
        // Policy enforcement and a genuine allocator failure are different
        // facts and lead to different remedies.
        let by_policy = OperationalFailure::PlannedAllocationExceedsBudget {
            operation: ResourceOperation::DeckVectorScaling,
            requested: 1 << 30,
            permitted: 1 << 20,
        };
        let by_allocator = OperationalFailure::AllocatorFailure {
            operation: ResourceOperation::DeckVectorScaling,
            requested: 1 << 30,
        };
        assert_ne!(by_policy, by_allocator);
        assert_eq!(by_policy.tag(), "planned_allocation_exceeds_budget");
        assert_eq!(by_allocator.tag(), "allocator_failure");
    }
}
