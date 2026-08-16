//! Evidence authority: what is known, on what basis, and who may *make* the
//! claim.
//!
//! `FORMAL_SYSTEM.md` Definition 2 makes a geometric fact a pair `Fact(P, κ)`
//! with `κ ∈ {Declared, Analytic, CertifiedNumerical, Assumed, Unresolved}`,
//! and adds: *"A production implementation may rely on a fact only if its
//! status meets the requirement of the consuming stage."*
//!
//! Three consequences drive this module's shape.
//!
//! **`Unresolved` carries no value.** A `struct Fact<T> { value: T, status }`
//! is unsound for this definition, because `Unresolved` is precisely the state
//! in which no proposition value has been established — such a struct forces
//! the construction of a `T` that stands for nothing. [`Evidence`] is a sum
//! type whose `Unresolved` variant has no `T` field at all. The value is not
//! hidden from unauthorized readers; it does not exist.
//!
//! **Authority is not a scale.** Definition 2's five statuses are *kinds* of
//! justification, not degrees of confidence, and the consuming stage names its
//! requirement. There is deliberately no ordering such as
//! `Declared > Analytic > CertifiedNumerical`: a stage that must read what
//! STEP literally said ([`EvidenceRequirement::SourceDeclaration`]) is not
//! served by a derivation, and a stage that must not depend on an exporter's
//! word ([`EvidenceRequirement::AnalyticOnly`]) is not served by a
//! declaration.
//!
//! **Authority cannot be forged.** Restricting who may *read* a claim is only
//! half the problem; a `pub fn analytic(value, rule)` lets any call site mint
//! the authority it wants. The three authority-bearing constructors are
//! therefore `pub(super)`, reachable only from inside this `formal` subtree,
//! and every authoritative value a caller can obtain comes from a named
//! introduction rule that checks the rule's own premises — for ambient
//! periods, [`super::ambient::certify_plane_aperiodicity`] and its siblings.
//! [`Evidence::assumed`] and [`Evidence::unresolved`] stay public because
//! neither produces authority.

use super::numeric::{FiniteF64, NonNegativeFinite};

/// Which parameter axis a proposition concerns.
///
/// Defined here rather than in `ambient` because predicates naming an axis are
/// part of the evidence vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParameterAxis {
    /// The `u` axis, in the caller's convention.
    U,
    /// The `v` axis, in the caller's convention.
    V,
}

impl ParameterAxis {
    /// The other axis.
    pub fn other(self) -> Self {
        match self {
            Self::U => Self::V,
            Self::V => Self::U,
        }
    }

    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::U => "u",
            Self::V => "v",
        }
    }
}

/// Which stage of the pipeline a claim or judgment belongs to.
///
/// Only the stages that exist. `FORMAL_SYSTEM.md` §XVIII lists the full
/// pipeline; adding its later stages before they can be reached would make the
/// enum a plan rather than a record.
///
/// Lives in this module because a [`UseSite`] names one, and every other
/// module in the subtree already depends on this one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SemanticStage {
    /// Resolving the ambient deck lattice from period evidence. Step 1.
    AmbientPeriodResolution,
}

impl SemanticStage {
    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::AmbientPeriodResolution => "ambient_period_resolution",
        }
    }
}

// ---------------------------------------------------------------------------
// Nonempty witness structures
// ---------------------------------------------------------------------------

/// A vector proved nonempty at the type level.
///
/// A report whose witness list may be empty is a report that can be
/// constructed without a witness. No `Default`: there is no default witness.
#[derive(Debug, Clone, PartialEq)]
pub struct NonEmptyVec<T> {
    first: T,
    rest: Vec<T>,
}

impl<T> NonEmptyVec<T> {
    /// One element.
    pub fn one(first: T) -> Self {
        Self {
            first,
            rest: Vec::new(),
        }
    }

    /// One element and any number of further elements.
    pub fn new(first: T, rest: Vec<T>) -> Self {
        Self { first, rest }
    }

    /// The element that is always present.
    pub fn first(&self) -> &T {
        &self.first
    }

    /// Add an element.
    pub fn push(&mut self, value: T) {
        self.rest.push(value);
    }

    /// The number of elements, always at least one.
    pub fn len(&self) -> usize {
        self.rest.len() + 1
    }

    /// Always false. Present so the `len`-without-`is_empty` lint does not
    /// invite a fallible length elsewhere.
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Iterate in order.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        std::iter::once(&self.first).chain(self.rest.iter())
    }

    /// Map elementwise, preserving nonemptiness.
    pub fn map<U>(&self, f: impl Fn(&T) -> U) -> NonEmptyVec<U> {
        NonEmptyVec {
            first: f(&self.first),
            rest: self.rest.iter().map(f).collect(),
        }
    }
}

/// A collection proved to hold at least two elements.
///
/// `FORMAL_SYSTEM.md` Definition 21 makes `Ambiguous` mean `|M| > 1`. An
/// ambiguity report holding one alternative would be a category error, so the
/// type refuses to represent it. No `Default`.
#[derive(Debug, Clone, PartialEq)]
pub struct AtLeastTwo<T> {
    first: T,
    second: T,
    rest: Vec<T>,
}

impl<T> AtLeastTwo<T> {
    /// Exactly two elements.
    pub fn two(first: T, second: T) -> Self {
        Self {
            first,
            second,
            rest: Vec::new(),
        }
    }

    /// Two elements and any number of further elements.
    pub fn new(first: T, second: T, rest: Vec<T>) -> Self {
        Self {
            first,
            second,
            rest,
        }
    }

    /// The first two, which always exist.
    pub fn pair(&self) -> (&T, &T) {
        (&self.first, &self.second)
    }

    /// The number of elements, always at least two.
    pub fn len(&self) -> usize {
        self.rest.len() + 2
    }

    /// Always false.
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Iterate in order.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        std::iter::once(&self.first)
            .chain(std::iter::once(&self.second))
            .chain(self.rest.iter())
    }
}

// ---------------------------------------------------------------------------
// Source identity
// ---------------------------------------------------------------------------

/// A STEP entity id, as retained by conversion.
///
/// A newtype rather than a bare `u64` so an entity id cannot be interchanged
/// with a face index, a shell ordinal, or a count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SourceEntityKey(u64);

impl SourceEntityKey {
    /// From a retained document entity id.
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// The id.
    pub fn get(self) -> u64 {
        self.0
    }
}

/// Which field of which entity a declaration was read from.
///
/// Only the fields Step 1 reads. This is the "source field path" of a
/// declaration: naming the *accessor category* is not enough, because two
/// different STEP fields reach the same accessor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceFieldPath {
    /// The support surface's declared periodicity on one axis, read from the
    /// surface schema rather than through a generic accessor.
    SupportSurfacePeriodicity(ParameterAxis),
}

impl SourceFieldPath {
    /// A short stable tag.
    pub fn tag(self) -> &'static str {
        match self {
            Self::SupportSurfacePeriodicity(ParameterAxis::U) => "support_surface_u_periodicity",
            Self::SupportSurfacePeriodicity(ParameterAxis::V) => "support_surface_v_periodicity",
        }
    }
}

/// The rule by which a source field was interpreted as a proposition.
///
/// `FORMAL_SYSTEM.md` §V makes the same point about orientation: *"The exact
/// mapping from STEP Boolean fields to ±1 is part of the STEP adapter
/// specification"*. A declaration is a field plus an interpretation, and
/// recording only the field would leave the interpretation unauditable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InterpretationRuleId {
    /// A `*_SURFACE` entity's declared periodicity is read as the period of
    /// the corresponding parameter axis of its own parameterization.
    StepPeriodicSurfaceDeclaration,
}

/// Where a *source declaration* came from.
///
/// Every field identifies something a reviewer can go and look at. Note what
/// cannot be spelled here: a bare `ParametricSurface::u_period()` result and a
/// `CertifiedLattice` with an erased constructor have no source entity and no
/// field path, so they cannot be dressed as declarations. They are
/// [`NonAuthoritativeOrigin`]s and produce `Assumed` or `Unresolved` evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceDeclaredProvenance {
    /// Which entity declared it.
    pub source_entity: SourceEntityKey,
    /// Which field of that entity.
    pub source_field: SourceFieldPath,
    /// How the field was interpreted.
    pub interpretation_rule: InterpretationRuleId,
}

/// A value's origin when that origin establishes nothing.
///
/// Kept separate from [`SourceDeclaredProvenance`] so the type system enforces
/// what the review calls the difference between a source declaration and an
/// observation: only the former can reach [`Evidence::declared`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NonAuthoritativeOrigin {
    /// A bare `ParametricSurface` accessor returned a value. For a
    /// `RevolutedCurve`'s generatrix axis this is `curve.period()` forwarded,
    /// and nothing establishes it.
    UnevidencedSurfaceAccessor {
        /// Which accessor.
        accessor: SurfaceAccessor,
    },
    /// A `domain::lattice::CertifiedLattice` reported it, and that type has
    /// already erased which constructor produced the state.
    LegacyLatticeWithErasedOrigin {
        /// Which accessor on the lattice.
        accessor: SurfaceAccessor,
    },
}

impl NonAuthoritativeOrigin {
    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::UnevidencedSurfaceAccessor { .. } => "unevidenced_surface_accessor",
            Self::LegacyLatticeWithErasedOrigin { .. } => "legacy_lattice_erased_origin",
        }
    }

    /// Which accessor.
    pub fn accessor(self) -> SurfaceAccessor {
        match self {
            Self::UnevidencedSurfaceAccessor { accessor }
            | Self::LegacyLatticeWithErasedOrigin { accessor } => accessor,
        }
    }
}

/// The accessors a nonauthoritative value can arrive through.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SurfaceAccessor {
    /// `ParametricSurface::u_period` or `CertifiedLattice::declared_u_period`.
    UPeriod,
    /// `ParametricSurface::v_period` or `CertifiedLattice::declared_v_period`.
    VPeriod,
}

impl SurfaceAccessor {
    /// The accessor for one axis.
    pub fn for_axis(axis: ParameterAxis) -> Self {
        match axis {
            ParameterAxis::U => Self::UPeriod,
            ParameterAxis::V => Self::VPeriod,
        }
    }

    /// A short stable tag.
    pub fn tag(self) -> &'static str {
        match self {
            Self::UPeriod => "u_period",
            Self::VPeriod => "v_period",
        }
    }
}

// ---------------------------------------------------------------------------
// Predicates
// ---------------------------------------------------------------------------

/// The proposition an [`Evidence`] is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PredicateDescription {
    /// The proposition.
    pub predicate: FormalPredicate,
}

impl PredicateDescription {
    /// Name a predicate.
    pub fn of(predicate: FormalPredicate) -> Self {
        Self { predicate }
    }

    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        self.predicate.tag()
    }
}

/// The propositions Step 1 states. Only these; later stages add their own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FormalPredicate {
    /// "The ambient quotient has a nonzero translation along this axis."
    AmbientAxisIsPeriodic(ParameterAxis),
    /// "No nonzero translation along this axis fixes the ambient surface."
    AmbientAxisIsAperiodic(ParameterAxis),
    /// "The declared period on this axis is a deck generator."
    DeclaredPeriodIsADeckGenerator(ParameterAxis),
    /// "The two certified generators are linearly independent."
    GeneratorsAreIndependent,
}

impl FormalPredicate {
    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::AmbientAxisIsPeriodic(ParameterAxis::U) => "u_periodic",
            Self::AmbientAxisIsPeriodic(ParameterAxis::V) => "v_periodic",
            Self::AmbientAxisIsAperiodic(ParameterAxis::U) => "u_aperiodic",
            Self::AmbientAxisIsAperiodic(ParameterAxis::V) => "v_aperiodic",
            Self::DeclaredPeriodIsADeckGenerator(ParameterAxis::U) => "u_declared_is_generator",
            Self::DeclaredPeriodIsADeckGenerator(ParameterAxis::V) => "v_declared_is_generator",
            Self::GeneratorsAreIndependent => "generators_independent",
        }
    }
}

/// Where a value is about to be consumed.
///
/// Recorded on every refusal so a rejected promotion says which stage wanted
/// what, not merely that something was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UseSite {
    /// The consuming stage.
    pub stage: SemanticStage,
    /// The proposition it needed.
    pub predicate: PredicateDescription,
}

// ---------------------------------------------------------------------------
// Certificates
// ---------------------------------------------------------------------------

/// A named analytic rule together with the premises it was applied under.
///
/// The premises are the correction that matters: `RevolutionAngularPeriodIs
/// TwoPi` is true *of a `RevolutedCurve`*, and a certificate naming only the
/// rule cannot be checked against the surface it was applied to.
#[derive(Debug, Clone, PartialEq)]
pub struct AnalyticCertificate {
    rule: AnalyticRule,
    premises: NonEmptyVec<AnalyticPremise>,
}

impl AnalyticCertificate {
    /// Build a certificate, checking that the premises are the ones the rule
    /// requires.
    ///
    /// This is the introduction rule for analytic authority: a rule name with
    /// the wrong premises is refused rather than recorded.
    pub fn new(
        rule: AnalyticRule,
        premises: NonEmptyVec<AnalyticPremise>,
    ) -> Result<Self, CertificateConstructionError> {
        for required in rule.required_premises() {
            if !premises.iter().any(|premise| premise == required) {
                return Err(CertificateConstructionError::MissingPremise {
                    rule,
                    premise: *required,
                });
            }
        }
        Ok(Self { rule, premises })
    }

    /// The rule.
    pub fn rule(&self) -> AnalyticRule {
        self.rule
    }

    /// The premises.
    pub fn premises(&self) -> &NonEmptyVec<AnalyticPremise> {
        &self.premises
    }
}

/// Why a certificate constructor refused. No catch-all.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CertificateConstructionError {
    /// A premise the rule requires was not supplied.
    MissingPremise {
        /// The rule.
        rule: AnalyticRule,
        /// The premise it needs.
        premise: AnalyticPremise,
    },
    /// The achieved bound is looser than the tolerance the predicate needed,
    /// so the procedure certified nothing.
    AchievedBoundExceedsRequiredTolerance {
        /// What was needed.
        required: f64,
        /// What was achieved.
        achieved: f64,
    },
    /// A certified domain's interval is empty or misordered.
    DegenerateCertifiedDomain,
}

/// The analytic rules Step 1 can appeal to. Each is a proposition about a
/// *representation*, provable without evaluating the surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnalyticRule {
    /// `RevolutedCurve::subs(u, v)` is `origin + rotation_matrix(v) · (…)`, so
    /// the revolved axis has period `2π` by construction of the map, for every
    /// generatrix. `domain/lattice.rs::PeriodWitness::ExactRevolutionAngle`.
    RevolutionAngularPeriodIsTwoPi,
    /// `Sphere::subs(u, v)` enters the azimuth only through `(cos v, sin v)` —
    /// a rotation about the polar axis — so the azimuth has period `2π` by
    /// construction of the primitive, for every latitude.
    /// `domain/lattice.rs::PeriodWitness::ExactSphereAzimuth`.
    SphereAzimuthPeriodIsTwoPi,
    /// A plane's parameterization is affine and injective on `ℝ²`; no nonzero
    /// translation fixes it. Establishes *absence*, not a period.
    PlaneHasNoPeriodicDirection,
    /// A straight generatrix is injective in its parameter, so the generatrix
    /// axis of a cylinder or cone carries no period. Establishes absence.
    StraightGeneratrixHasNoPeriod,
    /// Two nonzero axis-aligned translations on distinct axes are linearly
    /// independent in the represented parameter plane. Establishes generator
    /// independence for the axis-aligned basis schema, and only for it.
    AxisAlignedGeneratorsAreIndependent,
}

impl AnalyticRule {
    /// The premises this rule may not be applied without.
    pub fn required_premises(self) -> &'static [AnalyticPremise] {
        match self {
            Self::RevolutionAngularPeriodIsTwoPi => {
                &[AnalyticPremise::SupportSurfaceIsARevolvedCurve]
            }
            Self::SphereAzimuthPeriodIsTwoPi => &[AnalyticPremise::SupportSurfaceIsASphere],
            Self::PlaneHasNoPeriodicDirection => &[AnalyticPremise::SupportSurfaceIsAPlane],
            Self::StraightGeneratrixHasNoPeriod => &[
                AnalyticPremise::SupportSurfaceIsARevolvedCurve,
                AnalyticPremise::GeneratrixIsAStraightLine,
            ],
            Self::AxisAlignedGeneratorsAreIndependent => {
                &[AnalyticPremise::RepresentedBasisIsAxisAligned]
            }
        }
    }

    /// A short stable tag.
    pub fn tag(self) -> &'static str {
        match self {
            Self::RevolutionAngularPeriodIsTwoPi => "revolution_angular_period_two_pi",
            Self::SphereAzimuthPeriodIsTwoPi => "sphere_azimuth_period_two_pi",
            Self::PlaneHasNoPeriodicDirection => "plane_has_no_periodic_direction",
            Self::StraightGeneratrixHasNoPeriod => "straight_generatrix_has_no_period",
            Self::AxisAlignedGeneratorsAreIndependent => "axis_aligned_generators_independent",
        }
    }
}

/// A structural fact an analytic rule stands on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnalyticPremise {
    /// The support surface is a `RevolutedCurve`, possibly under a
    /// `Processor`.
    SupportSurfaceIsARevolvedCurve,
    /// The support surface is a sphere (`Processor<Sphere, Matrix4>`).
    SupportSurfaceIsASphere,
    /// The support surface is a plane.
    SupportSurfaceIsAPlane,
    /// The revolved generatrix is a straight line.
    GeneratrixIsAStraightLine,
    /// The represented parameter basis is axis aligned, so a `u` translation
    /// has zero `v` component and conversely.
    RepresentedBasisIsAxisAligned,
}

/// A closed parameter interval, proved ordered.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClosedInterval {
    lower: FiniteF64,
    upper: FiniteF64,
}

impl ClosedInterval {
    /// Build an interval, refusing a misordered pair.
    pub fn new(lower: f64, upper: f64) -> Result<Self, CertificateConstructionError> {
        let lower = FiniteF64::new(lower)
            .map_err(|_| CertificateConstructionError::DegenerateCertifiedDomain)?;
        let upper = FiniteF64::new(upper)
            .map_err(|_| CertificateConstructionError::DegenerateCertifiedDomain)?;
        match lower.get() <= upper.get() {
            true => Ok(Self { lower, upper }),
            false => Err(CertificateConstructionError::DegenerateCertifiedDomain),
        }
    }

    /// The lower endpoint.
    pub fn lower(self) -> FiniteF64 {
        self.lower
    }

    /// The upper endpoint.
    pub fn upper(self) -> FiniteF64 {
        self.upper
    }
}

/// The parameter region a numerical certificate is valid over.
///
/// A residual bound with no domain certifies nothing: `FORMAL_SYSTEM.md` §VI
/// requires the projection bound to hold *"for the certified interval"*, and a
/// bound established somewhere unspecified is a bound established nowhere.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CertifiedDomain {
    /// The `u` extent.
    pub u: ClosedInterval,
    /// The `v` extent.
    pub v: ClosedInterval,
}

/// How a numerical procedure ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerminationEvidence {
    /// The procedure converged inside its tolerance.
    ConvergedWithinTolerance,
    /// The subdivision was carried to completion over the certified domain.
    ExhaustiveSubdivisionComplete,
}

/// A certificate produced by a numerical procedure that bounds its own error.
///
/// Every field is required. A method plus a residual number does not certify a
/// proposition: the certificate must say *which* proposition, *over what
/// domain*, *to what tolerance*, *with what achieved bound*, *under what
/// termination*, and *on what structural premises*.
#[derive(Debug, Clone, PartialEq)]
pub struct NumericalCertificate {
    method: NumericalMethod,
    predicate: PredicateDescription,
    certified_domain: CertifiedDomain,
    required_tolerance: NonNegativeFinite,
    achieved_bound: NonNegativeFinite,
    termination: TerminationEvidence,
    premises: NonEmptyVec<AnalyticPremise>,
}

impl NumericalCertificate {
    /// Build a certificate, checking that the achieved bound actually meets
    /// the tolerance the predicate required.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        method: NumericalMethod,
        predicate: PredicateDescription,
        certified_domain: CertifiedDomain,
        required_tolerance: NonNegativeFinite,
        achieved_bound: NonNegativeFinite,
        termination: TerminationEvidence,
        premises: NonEmptyVec<AnalyticPremise>,
    ) -> Result<Self, CertificateConstructionError> {
        if achieved_bound.get() > required_tolerance.get() {
            return Err(
                CertificateConstructionError::AchievedBoundExceedsRequiredTolerance {
                    required: required_tolerance.get(),
                    achieved: achieved_bound.get(),
                },
            );
        }
        Ok(Self {
            method,
            predicate,
            certified_domain,
            required_tolerance,
            achieved_bound,
            termination,
            premises,
        })
    }

    /// The procedure.
    pub fn method(&self) -> NumericalMethod {
        self.method
    }

    /// The proposition certified.
    pub fn predicate(&self) -> PredicateDescription {
        self.predicate
    }

    /// The region it holds over.
    pub fn certified_domain(&self) -> CertifiedDomain {
        self.certified_domain
    }

    /// The tolerance required.
    pub fn required_tolerance(&self) -> NonNegativeFinite {
        self.required_tolerance
    }

    /// The bound achieved.
    pub fn achieved_bound(&self) -> NonNegativeFinite {
        self.achieved_bound
    }

    /// How it terminated.
    pub fn termination(&self) -> TerminationEvidence {
        self.termination
    }

    /// The structural premises.
    pub fn premises(&self) -> &NonEmptyVec<AnalyticPremise> {
        &self.premises
    }
}

/// Numerical procedures whose output may be treated as certified.
///
/// Sampling is not among them. `FORMAL_SYSTEM.md` §VI: *"Compatibility is not
/// established by isolated nearest-point samples alone."*
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NumericalMethod {
    /// Interval or affine arithmetic over the whole certified domain.
    IntervalArithmetic,
    /// Continuation with a certified branch-separation bound.
    CertifiedContinuation,
}

// ---------------------------------------------------------------------------
// Assumptions and attempts
// ---------------------------------------------------------------------------

/// Something taken on faith. Retained so it is auditable; it can never become
/// authoritative.
///
/// Carries what an auditor needs in order to go and discharge it: which
/// proposition, from what origin, over what scope, and under which recorded
/// rationale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssumptionRecord {
    /// What was assumed.
    pub predicate: PredicateDescription,
    /// Where the value came from.
    pub origin: NonAuthoritativeOrigin,
    /// How far the assumption reaches.
    pub scope: AssumptionScope,
    /// The recorded reason it is being made.
    pub rationale: AssumptionRationaleId,
}

/// How far an assumption reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssumptionScope {
    /// One axis of one face.
    SingleAxisOfOneFace,
    /// One whole face.
    OneFace,
}

/// The recorded rationales for an assumption.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssumptionRationaleId {
    /// A bare accessor returned a number and it was treated as a period.
    AccessorResultTakenAsPeriod,
    /// A bare accessor returned nothing and it was treated as proof that no
    /// period exists. The assumption Step 1 exists to stop making.
    MissingAccessorTakenAsAbsence,
    /// Made only to keep the legacy path's behaviour identical while the
    /// formal path is built beside it.
    LegacyBehaviourPreservation,
}

/// One recorded attempt to resolve a predicate, and how it ended.
///
/// A `NonEmptyVec<ResolutionAttempt>` on every `Unresolved` state means an
/// unresolved report is constructible only after at least one permitted
/// resolution method was considered or attempted — including the case where
/// the method was considered and found inapplicable, which
/// [`AttemptOutcome::NotImplementedAtThisStage`] records honestly rather than
/// pretending an attempt ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResolutionAttempt {
    /// What was considered or tried.
    pub method: ResolutionMethod,
    /// How it ended.
    pub outcome: AttemptOutcome,
}

/// What was tried in order to resolve a predicate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResolutionMethod {
    /// The legacy `CertifiedLattice` accessors were read.
    LegacyCertifiedLatticeAccessor,
    /// The concrete support-surface representation was inspected.
    SupportSurfaceSchemaInspection,
    /// A representation-derived witness was sought for the axis.
    RepresentationDerivedWitness,
}

/// How a resolution attempt ended. Every variant names a *reason*; there is no
/// catch-all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AttemptOutcome {
    /// The evidence needed had already been erased before this code ran. The
    /// `CertifiedLattice` reaching the adapter is in this state: it cannot say
    /// which constructor produced a `NonPeriodic` axis.
    EvidenceErasedBeforeThisStage,
    /// The representation was reachable, but this implementation has no rule
    /// that certifies the predicate for it.
    NoCertifyingRuleForRepresentation,
    /// The method was considered and is not implemented at this stage.
    NotImplementedAtThisStage,
}

impl AttemptOutcome {
    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::EvidenceErasedBeforeThisStage => "erased_before_stage",
            Self::NoCertifyingRuleForRepresentation => "no_certifying_rule",
            Self::NotImplementedAtThisStage => "not_implemented",
        }
    }
}

// ---------------------------------------------------------------------------
// Evidence
// ---------------------------------------------------------------------------

/// What is known about a proposition of type `T`, and on what basis.
///
/// The inner representation is private, the `Unresolved` state holds no `T`,
/// and the three authority-bearing constructors are `pub(super)`. There is no
/// `Deref`, no `AsRef<T>`, no `Borrow<T>`, no `value()`, no `unwrap_or` and no
/// `unwrap_or_default`. The only route to a readable value is
/// [`AuthoritativeFact::try_from_evidence`] with a named
/// [`EvidenceRequirement`] and a [`UseSite`].
#[derive(Debug, Clone, PartialEq)]
pub struct Evidence<T> {
    inner: EvidenceInner<T>,
}

/// Private. The variants are the authority model; exposing them would let a
/// consumer pattern-match past the requirement check.
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
enum EvidenceInner<T> {
    // `Declared` and `CertifiedNumerical` are constructible and not yet
    // constructed. Step 1's only authoritative propositions are ambient period
    // presence and absence, and both are established analytically. A source
    // declaration becomes reachable when a stage needs a proposition *about
    // the document*; a numerical certificate becomes reachable with the first
    // self-bounding procedure. Neither is deleted, because removing a status of
    // Definition 2 and re-adding it later is how five statuses become three.
    Declared {
        value: T,
        provenance: SourceDeclaredProvenance,
    },
    Analytic {
        value: T,
        certificate: AnalyticCertificate,
    },
    CertifiedNumerical {
        value: T,
        certificate: NumericalCertificate,
    },
    Assumed {
        value: T,
        assumption: AssumptionRecord,
    },
    /// No `value` field. This is the point of the whole type.
    Unresolved {
        predicate: PredicateDescription,
        attempts: NonEmptyVec<ResolutionAttempt>,
    },
}

impl<T> Evidence<T> {
    /// A value a named source entity declared in a named field.
    ///
    /// `pub(super)`: only introduction rules inside the `formal` subtree may
    /// mint declared authority, and [`SourceDeclaredProvenance`] makes an
    /// accessor result unable to reach it in the first place.
    #[allow(dead_code)]
    pub(super) fn declared(value: T, provenance: SourceDeclaredProvenance) -> Self {
        Self {
            inner: EvidenceInner::Declared { value, provenance },
        }
    }

    /// A value derived by a named analytic rule under checked premises.
    ///
    /// `pub(super)`; see [`Self::declared`].
    pub(super) fn analytic(value: T, certificate: AnalyticCertificate) -> Self {
        Self {
            inner: EvidenceInner::Analytic { value, certificate },
        }
    }

    /// A value produced by a self-bounding numerical procedure.
    ///
    /// `pub(super)`; see [`Self::declared`].
    #[allow(dead_code)]
    pub(super) fn certified_numerical(value: T, certificate: NumericalCertificate) -> Self {
        Self {
            inner: EvidenceInner::CertifiedNumerical { value, certificate },
        }
    }

    /// A value taken on faith. Auditable, never authoritative.
    ///
    /// Public, because it grants nothing: no requirement admits it.
    pub fn assumed(value: T, assumption: AssumptionRecord) -> Self {
        Self {
            inner: EvidenceInner::Assumed { value, assumption },
        }
    }

    /// No value was established. The attempts are the record of why.
    ///
    /// Public, because it grants nothing.
    pub fn unresolved(
        predicate: PredicateDescription,
        attempts: NonEmptyVec<ResolutionAttempt>,
    ) -> Self {
        Self {
            inner: EvidenceInner::Unresolved {
                predicate,
                attempts,
            },
        }
    }

    /// Which of the five statuses this is. Diagnostic only: knowing the status
    /// does not yield the value.
    pub fn status(&self) -> EvidenceStatus {
        match &self.inner {
            EvidenceInner::Declared { .. } => EvidenceStatus::Declared,
            EvidenceInner::Analytic { .. } => EvidenceStatus::Analytic,
            EvidenceInner::CertifiedNumerical { .. } => EvidenceStatus::CertifiedNumerical,
            EvidenceInner::Assumed { .. } => EvidenceStatus::Assumed,
            EvidenceInner::Unresolved { .. } => EvidenceStatus::Unresolved,
        }
    }

    /// The predicate, when unresolved.
    pub fn unresolved_predicate(&self) -> Option<PredicateDescription> {
        match &self.inner {
            EvidenceInner::Unresolved { predicate, .. } => Some(*predicate),
            _ => None,
        }
    }

    /// The recorded attempts, when unresolved.
    pub fn unresolved_attempts(&self) -> Option<&NonEmptyVec<ResolutionAttempt>> {
        match &self.inner {
            EvidenceInner::Unresolved { attempts, .. } => Some(attempts),
            _ => None,
        }
    }
}

/// The five statuses of `FORMAL_SYSTEM.md` Definition 2, as a diagnostic
/// label. Deliberately not `Ord`: see the module documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EvidenceStatus {
    /// The source states it.
    Declared,
    /// A named rule derives it.
    Analytic,
    /// A self-bounding numerical procedure established it.
    CertifiedNumerical,
    /// It was taken on faith.
    Assumed,
    /// Nothing established it, and no value exists.
    Unresolved,
}

impl EvidenceStatus {
    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::Declared => "declared",
            Self::Analytic => "analytic",
            Self::CertifiedNumerical => "certified_numerical",
            Self::Assumed => "assumed",
            Self::Unresolved => "unresolved",
        }
    }
}

// ---------------------------------------------------------------------------
// Authority
// ---------------------------------------------------------------------------

/// The three bases on which a value may be read authoritatively.
///
/// `Assumed` and `Unresolved` are absent by construction, not by convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuthoritativeBasis {
    /// A named source entity declared it.
    Declared,
    /// A named rule derives it.
    Analytic,
    /// A self-bounding numerical procedure established it.
    CertifiedNumerical,
}

impl AuthoritativeBasis {
    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::Declared => "declared",
            Self::Analytic => "analytic",
            Self::CertifiedNumerical => "certified_numerical",
        }
    }
}

/// The certificate behind an [`AuthoritativeFact`], matched to its basis.
#[derive(Debug, Clone, PartialEq)]
pub enum EvidenceCertificate {
    /// Declared, with its source entity, field and interpretation rule.
    Declared(SourceDeclaredProvenance),
    /// Analytic, with the rule and its premises.
    Analytic(AnalyticCertificate),
    /// Certified numerically, with predicate, domain, tolerance, achieved
    /// bound, termination and premises.
    CertifiedNumerical(NumericalCertificate),
}

impl EvidenceCertificate {
    /// The basis this certificate establishes.
    pub fn basis(&self) -> AuthoritativeBasis {
        match self {
            Self::Declared(_) => AuthoritativeBasis::Declared,
            Self::Analytic(_) => AuthoritativeBasis::Analytic,
            Self::CertifiedNumerical(_) => AuthoritativeBasis::CertifiedNumerical,
        }
    }
}

/// A value a consuming stage is entitled to read, together with why.
///
/// Obtainable only through [`Self::try_from_evidence`], so a fact's presence at
/// a call site is itself the record that the stage's requirement was met.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthoritativeFact<T> {
    value: T,
    basis: AuthoritativeBasis,
    certificate: EvidenceCertificate,
    use_site: UseSite,
}

impl<T> AuthoritativeFact<T> {
    /// Promote evidence to a fact, if the requirement admits its basis.
    ///
    /// Consumes the evidence: a fact and its evidence are the same knowledge,
    /// and leaving both in scope invites a second read under a weaker policy.
    ///
    /// On refusal **nothing is discarded**. The returned [`AuthorityRefusal`]
    /// carries the use site, the requirement, and the complete rejected state
    /// — attempts for unresolved evidence, the assumption record for assumed
    /// evidence, the certificate for an inadmissible basis — so a caller can
    /// build a report from the refusal alone.
    pub fn try_from_evidence(
        evidence: Evidence<T>,
        requirement: EvidenceRequirement,
        use_site: UseSite,
    ) -> Result<Self, AuthorityRefusal> {
        let refuse = |reason| AuthorityRefusal {
            use_site,
            requirement,
            reason,
        };
        let (value, certificate) = match evidence.inner {
            EvidenceInner::Declared { value, provenance } => {
                (value, EvidenceCertificate::Declared(provenance))
            }
            EvidenceInner::Analytic { value, certificate } => {
                (value, EvidenceCertificate::Analytic(certificate))
            }
            EvidenceInner::CertifiedNumerical { value, certificate } => {
                (value, EvidenceCertificate::CertifiedNumerical(certificate))
            }
            EvidenceInner::Assumed { assumption, .. } => {
                return Err(refuse(RefusalReason::AssumedIsNeverAuthoritative {
                    assumption,
                }));
            }
            EvidenceInner::Unresolved {
                predicate,
                attempts,
            } => {
                return Err(refuse(RefusalReason::UnresolvedHasNoValue {
                    predicate,
                    attempts,
                }));
            }
        };
        let basis = certificate.basis();
        match requirement.admits(basis) {
            true => Ok(Self {
                value,
                basis,
                certificate,
                use_site,
            }),
            false => Err(refuse(RefusalReason::BasisNotAdmitted {
                basis,
                certificate,
            })),
        }
    }

    /// Read the value.
    pub fn get(&self) -> &T {
        &self.value
    }

    /// Consume the fact for its value.
    pub fn into_value(self) -> T {
        self.value
    }

    /// The basis on which it may be read.
    pub fn basis(&self) -> AuthoritativeBasis {
        self.basis
    }

    /// The certificate behind it.
    pub fn certificate(&self) -> &EvidenceCertificate {
        &self.certificate
    }

    /// The site that promoted it.
    pub fn use_site(&self) -> UseSite {
        self.use_site
    }
}

/// A refused promotion, with everything the promotion had to work with.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthorityRefusal {
    /// Where the value was wanted.
    pub use_site: UseSite,
    /// The policy applied.
    pub requirement: EvidenceRequirement,
    /// Why it was refused, with the rejected state attached.
    pub reason: RefusalReason,
}

/// Why a promotion to [`AuthoritativeFact`] was refused.
#[derive(Debug, Clone, PartialEq)]
pub enum RefusalReason {
    /// The basis is real but this proposition's policy does not admit it. The
    /// rejected certificate is retained, not dropped.
    BasisNotAdmitted {
        /// The basis offered.
        basis: AuthoritativeBasis,
        /// The certificate offered.
        certificate: EvidenceCertificate,
    },
    /// Assumed evidence has a value and no justification.
    AssumedIsNeverAuthoritative {
        /// The full assumption record.
        assumption: AssumptionRecord,
    },
    /// Unresolved evidence has no value at all. The attempt history survives
    /// the refusal, so the report built from it can say what was tried.
    UnresolvedHasNoValue {
        /// The predicate that was not resolved.
        predicate: PredicateDescription,
        /// What was tried.
        attempts: NonEmptyVec<ResolutionAttempt>,
    },
}

/// Which bases a particular proposition admits.
///
/// A closed enum rather than three booleans: an unnamed combination is
/// unrepresentable, so there is no "policy" a reviewer has to reconstruct from
/// its bits and no fallback tag for one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EvidenceRequirement {
    /// Only what a named source entity declared will do.
    ///
    /// For propositions *about the source document* — "which entity did this
    /// face name" — where a derivation, however correct about the geometry,
    /// answers a different question.
    SourceDeclaration,

    /// Only a named analytic rule will do.
    ///
    /// For propositions the implementation must not take an exporter's word
    /// on. Ambient period *absence* is one: `CORRECTNESS_GAP_REGISTER.md` C5
    /// records four surface classes whose declared periodicity is carried
    /// uncertified, and a torus that fails to declare its second period is the
    /// case that makes a declaration-based absence rule unsound.
    AnalyticOnly,

    /// A derivation or a self-bounding numerical certificate; not a
    /// declaration.
    ///
    /// For deck generators: `FORMAL_SYSTEM.md` §VII makes `δ` the unique
    /// integer solving `γ(1) = γ(0) + Lδ`, so `L` must be the period of the
    /// map and not the number an exporter wrote down.
    AnalyticOrCertifiedNumerical,

    /// Any of the three authoritative bases. The weakest admissible policy,
    /// and still strictly stronger than reading a value out of `Assumed` or
    /// `Unresolved` evidence.
    AnyAuthoritative,
}

impl EvidenceRequirement {
    /// Whether this policy admits a basis.
    pub fn admits(self, basis: AuthoritativeBasis) -> bool {
        match (self, basis) {
            (Self::SourceDeclaration, AuthoritativeBasis::Declared) => true,
            (Self::SourceDeclaration, _) => false,
            (Self::AnalyticOnly, AuthoritativeBasis::Analytic) => true,
            (Self::AnalyticOnly, _) => false,
            (Self::AnalyticOrCertifiedNumerical, AuthoritativeBasis::Declared) => false,
            (Self::AnalyticOrCertifiedNumerical, _) => true,
            (Self::AnyAuthoritative, _) => true,
        }
    }

    /// A short stable tag, for probe records and refusal messages.
    pub fn tag(self) -> &'static str {
        match self {
            Self::SourceDeclaration => "source_declaration",
            Self::AnalyticOnly => "analytic_only",
            Self::AnalyticOrCertifiedNumerical => "analytic_or_certified_numerical",
            Self::AnyAuthoritative => "any_authoritative",
        }
    }
}

/// Only what a named source entity declared. See
/// [`EvidenceRequirement::SourceDeclaration`].
pub const SOURCE_DECLARATION: EvidenceRequirement = EvidenceRequirement::SourceDeclaration;
/// Only a named analytic rule. See [`EvidenceRequirement::AnalyticOnly`].
pub const ANALYTIC_ONLY: EvidenceRequirement = EvidenceRequirement::AnalyticOnly;
/// A derivation or a numerical certificate, not a declaration. See
/// [`EvidenceRequirement::AnalyticOrCertifiedNumerical`].
pub const ANALYTIC_OR_CERTIFIED_NUMERICAL: EvidenceRequirement =
    EvidenceRequirement::AnalyticOrCertifiedNumerical;
/// Any authoritative basis. See [`EvidenceRequirement::AnyAuthoritative`].
pub const ANY_AUTHORITATIVE: EvidenceRequirement = EvidenceRequirement::AnyAuthoritative;

#[cfg(test)]
mod tests {
    use super::*;

    fn a_use_site() -> UseSite {
        UseSite {
            stage: SemanticStage::AmbientPeriodResolution,
            predicate: PredicateDescription::of(FormalPredicate::AmbientAxisIsPeriodic(
                ParameterAxis::U,
            )),
        }
    }

    fn a_source_provenance() -> SourceDeclaredProvenance {
        SourceDeclaredProvenance {
            source_entity: SourceEntityKey::new(1234),
            source_field: SourceFieldPath::SupportSurfacePeriodicity(ParameterAxis::U),
            interpretation_rule: InterpretationRuleId::StepPeriodicSurfaceDeclaration,
        }
    }

    fn an_analytic_certificate() -> AnalyticCertificate {
        AnalyticCertificate::new(
            AnalyticRule::RevolutionAngularPeriodIsTwoPi,
            NonEmptyVec::one(AnalyticPremise::SupportSurfaceIsARevolvedCurve),
        )
        .expect("premises match the rule")
    }

    fn an_attempt() -> NonEmptyVec<ResolutionAttempt> {
        NonEmptyVec::one(ResolutionAttempt {
            method: ResolutionMethod::LegacyCertifiedLatticeAccessor,
            outcome: AttemptOutcome::EvidenceErasedBeforeThisStage,
        })
    }

    #[test]
    fn declared_fact_can_satisfy_declared_requirement() {
        let evidence = Evidence::declared(7u32, a_source_provenance());
        let fact = AuthoritativeFact::try_from_evidence(evidence, SOURCE_DECLARATION, a_use_site())
            .expect("a source declaration satisfies a declaration policy");
        assert_eq!(*fact.get(), 7);
        assert_eq!(fact.basis(), AuthoritativeBasis::Declared);
        assert_eq!(fact.use_site(), a_use_site());
    }

    #[test]
    fn analytic_fact_cannot_satisfy_declared_only_requirement() {
        // The point of refusing an ordinal ranking: analytic evidence is not
        // "better declared evidence". It answers a different question.
        let evidence = Evidence::analytic(7u32, an_analytic_certificate());
        let refusal =
            AuthoritativeFact::try_from_evidence(evidence, SOURCE_DECLARATION, a_use_site())
                .expect_err("analytic evidence does not satisfy SOURCE_DECLARATION");
        assert_eq!(refusal.requirement, SOURCE_DECLARATION);
        assert_eq!(refusal.use_site, a_use_site());
        match refusal.reason {
            // The rejected certificate survives the refusal.
            RefusalReason::BasisNotAdmitted { basis, certificate } => {
                assert_eq!(basis, AuthoritativeBasis::Analytic);
                assert_eq!(
                    certificate,
                    EvidenceCertificate::Analytic(an_analytic_certificate())
                );
            }
            other => panic!("wrong refusal: {other:?}"),
        }
    }

    #[test]
    fn assumed_fact_cannot_become_authoritative() {
        // Refused under the *weakest* policy, so the refusal is a property of
        // the basis and not of any particular requirement.
        let assumption = AssumptionRecord {
            predicate: PredicateDescription::of(FormalPredicate::AmbientAxisIsAperiodic(
                ParameterAxis::U,
            )),
            origin: NonAuthoritativeOrigin::UnevidencedSurfaceAccessor {
                accessor: SurfaceAccessor::UPeriod,
            },
            scope: AssumptionScope::SingleAxisOfOneFace,
            rationale: AssumptionRationaleId::MissingAccessorTakenAsAbsence,
        };
        let evidence = Evidence::assumed(7u32, assumption);
        let refusal =
            AuthoritativeFact::try_from_evidence(evidence, ANY_AUTHORITATIVE, a_use_site())
                .expect_err("assumed evidence is never authoritative");
        assert_eq!(
            refusal.reason,
            RefusalReason::AssumedIsNeverAuthoritative { assumption },
            "the whole assumption record survives the refusal"
        );
    }

    #[test]
    fn unresolved_evidence_contains_no_value() {
        let predicate =
            PredicateDescription::of(FormalPredicate::AmbientAxisIsPeriodic(ParameterAxis::U));
        let evidence: Evidence<u32> = Evidence::unresolved(predicate, an_attempt());

        // There is no accessor that could return a `u32` here, under any
        // policy, because the variant has no such field.
        let refusal =
            AuthoritativeFact::try_from_evidence(evidence, ANY_AUTHORITATIVE, a_use_site())
                .expect_err("unresolved evidence has no value to promote");
        assert_eq!(
            refusal.reason,
            RefusalReason::UnresolvedHasNoValue {
                predicate,
                attempts: an_attempt(),
            },
            "the attempt history survives the refusal"
        );

        let evidence: Evidence<u32> = Evidence::unresolved(predicate, an_attempt());
        assert_eq!(evidence.status(), EvidenceStatus::Unresolved);
        assert_eq!(evidence.unresolved_predicate(), Some(predicate));
        assert_eq!(
            evidence.unresolved_attempts().map(NonEmptyVec::len),
            Some(1)
        );
    }

    #[test]
    fn evidence_bases_have_no_total_order() {
        // `AuthoritativeBasis` implements no ordering, so no call site can
        // express "at least as good as". Behaviourally: admissibility has no
        // consistent linear order, because each of two bases is admitted by a
        // policy that refuses the other.
        assert!(SOURCE_DECLARATION.admits(AuthoritativeBasis::Declared));
        assert!(!SOURCE_DECLARATION.admits(AuthoritativeBasis::Analytic));
        assert!(ANALYTIC_ONLY.admits(AuthoritativeBasis::Analytic));
        assert!(!ANALYTIC_ONLY.admits(AuthoritativeBasis::Declared));
        assert!(!ANALYTIC_ONLY.admits(AuthoritativeBasis::CertifiedNumerical));
        assert!(ANALYTIC_OR_CERTIFIED_NUMERICAL.admits(AuthoritativeBasis::CertifiedNumerical));
        assert!(!ANALYTIC_OR_CERTIFIED_NUMERICAL.admits(AuthoritativeBasis::Declared));

        // Were there a total order with "admits" upward-closed, SOURCE_DECLARATION
        // would force Declared ≺ Analytic and ANALYTIC_ONLY would force
        // Analytic ≺ Declared. Contradiction.
        assert_ne!(
            SOURCE_DECLARATION.admits(AuthoritativeBasis::Analytic),
            ANALYTIC_ONLY.admits(AuthoritativeBasis::Analytic),
        );
        assert_ne!(
            SOURCE_DECLARATION.admits(AuthoritativeBasis::Declared),
            ANALYTIC_ONLY.admits(AuthoritativeBasis::Declared),
        );
    }

    #[test]
    fn every_requirement_is_named() {
        // The closed enum makes an unnamed policy unrepresentable, so this is
        // exhaustive by construction rather than by inspection.
        for requirement in [
            EvidenceRequirement::SourceDeclaration,
            EvidenceRequirement::AnalyticOnly,
            EvidenceRequirement::AnalyticOrCertifiedNumerical,
            EvidenceRequirement::AnyAuthoritative,
        ] {
            assert_ne!(requirement.tag(), "");
            assert!(!requirement.tag().contains("unnamed"));
        }
    }

    #[test]
    fn an_analytic_certificate_retains_its_premises() {
        let certificate = an_analytic_certificate();
        assert_eq!(
            certificate.rule(),
            AnalyticRule::RevolutionAngularPeriodIsTwoPi
        );
        assert_eq!(
            *certificate.premises().first(),
            AnalyticPremise::SupportSurfaceIsARevolvedCurve
        );
        // And a rule cannot be asserted without them: `2π` is a property of a
        // revolved parameterization, so claiming it for a plane is refused.
        assert_eq!(
            AnalyticCertificate::new(
                AnalyticRule::RevolutionAngularPeriodIsTwoPi,
                NonEmptyVec::one(AnalyticPremise::SupportSurfaceIsAPlane),
            ),
            Err(CertificateConstructionError::MissingPremise {
                rule: AnalyticRule::RevolutionAngularPeriodIsTwoPi,
                premise: AnalyticPremise::SupportSurfaceIsARevolvedCurve,
            })
        );
        // A multi-premise rule needs all of them.
        assert_eq!(
            AnalyticCertificate::new(
                AnalyticRule::StraightGeneratrixHasNoPeriod,
                NonEmptyVec::one(AnalyticPremise::SupportSurfaceIsARevolvedCurve),
            ),
            Err(CertificateConstructionError::MissingPremise {
                rule: AnalyticRule::StraightGeneratrixHasNoPeriod,
                premise: AnalyticPremise::GeneratrixIsAStraightLine,
            })
        );
    }

    #[test]
    fn a_numerical_certificate_states_its_whole_claim() {
        let domain = CertifiedDomain {
            u: ClosedInterval::new(0.0, 1.0).unwrap(),
            v: ClosedInterval::new(0.0, std::f64::consts::TAU).unwrap(),
        };
        let certificate = NumericalCertificate::new(
            NumericalMethod::IntervalArithmetic,
            PredicateDescription::of(FormalPredicate::AmbientAxisIsPeriodic(ParameterAxis::V)),
            domain,
            NonNegativeFinite::new(1e-9).unwrap(),
            NonNegativeFinite::new(1e-12).unwrap(),
            TerminationEvidence::ExhaustiveSubdivisionComplete,
            NonEmptyVec::one(AnalyticPremise::RepresentedBasisIsAxisAligned),
        )
        .expect("the achieved bound meets the tolerance");
        assert_eq!(certificate.achieved_bound().get(), 1e-12);
        assert_eq!(
            certificate.termination(),
            TerminationEvidence::ExhaustiveSubdivisionComplete
        );

        // A procedure that did not reach its tolerance certified nothing.
        assert_eq!(
            NumericalCertificate::new(
                NumericalMethod::IntervalArithmetic,
                PredicateDescription::of(FormalPredicate::AmbientAxisIsPeriodic(ParameterAxis::V)),
                domain,
                NonNegativeFinite::new(1e-12).unwrap(),
                NonNegativeFinite::new(1e-9).unwrap(),
                TerminationEvidence::ConvergedWithinTolerance,
                NonEmptyVec::one(AnalyticPremise::RepresentedBasisIsAxisAligned),
            ),
            Err(
                CertificateConstructionError::AchievedBoundExceedsRequiredTolerance {
                    required: 1e-12,
                    achieved: 1e-9,
                }
            )
        );
        assert_eq!(
            ClosedInterval::new(1.0, 0.0),
            Err(CertificateConstructionError::DegenerateCertifiedDomain)
        );
    }

    #[test]
    fn nonempty_structures_cannot_be_empty() {
        let one = NonEmptyVec::one(1u8);
        assert_eq!(one.len(), 1);
        assert_eq!(*one.first(), 1);
        let two = AtLeastTwo::two(1u8, 2u8);
        assert_eq!(two.len(), 2);
        assert_eq!(two.iter().copied().collect::<Vec<_>>(), vec![1, 2]);
    }
}
