//! Ambient period authority: what the deck lattice is, and who may use it.
//!
//! `FORMAL_SYSTEM.md` Definition 7 makes the ambient schema carry
//! `Λ = LZ^r` with `0 ≤ r ≤ r_max`, and Definition 8 makes the semantic
//! parameter object the quotient `(Ω/Λ)/Σ`. Everything downstream — the deck
//! displacement `δ` of Definition 9, the potential equations of §VII, the
//! candidate translation sets `K_ij` of Definition 16, the vertex
//! identification `~_Λ` of §IX — is stated in terms of `Λ`. If `Λ` is wrong,
//! every one of those is wrong in a way no later stage can detect.
//!
//! # The distinction this module exists for
//!
//! The corpus census over `00009190` reports:
//!
//! ```text
//!                 declared_rank    certified_rank
//!     rank 0         12,122           13,770
//!     rank 1         10,720           10,429
//!     rank 2          1,357                0
//! ```
//!
//! 1,357 faces declare rank 2 and certify rank 0; a further 291 declare rank 1
//! and certify rank 0. Reading `certified_rank() == 0` as `r = 0` would assign
//! all 1,648 of them a trivial deck group — asserting that no translation
//! identifies their parameter points — when what actually happened is that
//! *periodicity was declared and nothing certified it*. Those are different
//! propositions, and the corpus contains faces that distinguish them: a torus
//! is doubly periodic whatever `curve.period()` returns.
//!
//! So `certified_rank() == 0` supports no ambient conclusion at all. This
//! module gives the five states that are actually distinguishable —
//! authoritatively absent, certified, declared but uncertified, undetermined,
//! contradictory — and only the first two can feed a rank.
//!
//! # Introduction rules, not constructors
//!
//! Authority here is minted only by the named rules in the *introduction
//! rules* section — [`certify_plane_aperiodicity`],
//! [`certify_straight_generatrix_aperiodicity`],
//! [`certify_revolution_period`], [`certify_period_numerically`] — each of
//! which supplies the premises its rule requires. There is no public way to
//! hand a number and a rule name to [`CertifiedPeriodGenerator`] and get
//! authority back.
//!
//! # Two APIs, not one permissive one
//!
//! A single `period_for(use_case) -> Option<f64>` would put the decision at
//! the call site, where it is invisible in review and unchecked by the
//! compiler. Instead the authority is carried by the *type*:
//!
//! - [`AmbientPeriodEvidence::diagnostic_hints`] yields
//!   [`DeclaredPeriodHint`]s. They are named for what they are and convert
//!   into nothing.
//! - [`CertifiedAmbientLattice`] is the only type with deck operations on it,
//!   and it is obtainable only from [`resolve_ambient_periods`].

use super::envelope::{
    BoundObservation, CountingProcedure, ExactCount, FormalEnvelope, MeasurementSubject,
};
use super::evidence::{
    AnalyticCertificate, AnalyticPremise, AnalyticRule, AttemptOutcome, AuthoritativeBasis,
    AuthoritativeFact, CertificateConstructionError, Evidence, EvidenceCertificate,
    FormalPredicate, NonAuthoritativeOrigin, NonEmptyVec, NumericalCertificate, ParameterAxis,
    PredicateDescription, ResolutionAttempt, ResolutionMethod, SemanticStage,
    SourceDeclaredProvenance, SurfaceAccessor, UseSite, ANALYTIC_ONLY,
};
use super::numeric::{FiniteF64, NumericDomainError, PositiveFinite};
use super::outcome::{
    ContradictionWitness, FaceKey, InconsistencyReport, InvariantId, OperationalFailure,
    ProvenanceRecord, ProvenanceSet, ResourceOperation, StageEvaluation, StageOutcome,
    UnresolvedReason, UnresolvedReport, UnsupportedCause, UnsupportedReport,
};
use super::quotient::{AmbientLatticeId, DeckContext};
use super::support::{PlaneSchema, SupportSurfaceSchema};

// ---------------------------------------------------------------------------
// Declarations and hints
// ---------------------------------------------------------------------------

/// A period a named source entity declared.
///
/// A declaration is a *statement*, not a certificate. Definition 9's `δ` is the
/// unique integer with `γ(1) = γ(0) + Lδ`; uniqueness fails the moment `L` is
/// not the true period of the map, so a declaration cannot be `L`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeclaredPeriod {
    /// The declared value. Positive and finite by construction.
    pub value: PositiveFinite,
    /// Which entity declared it, in which field, under which interpretation.
    pub provenance: SourceDeclaredProvenance,
}

/// A period value that arrived through an accessor establishing nothing.
///
/// Kept apart from [`DeclaredPeriod`] because it *is* a different thing: there
/// is no source entity behind it and no field to go and read. `look`'s
/// `lattice_of` routes `Sphere`, `ToroidalSurface`, `SweptCurve` and
/// `OffsetSurface` through exactly this path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObservedPeriod {
    /// The value observed. Positive and finite by construction.
    pub value: PositiveFinite,
    /// Which accessor produced it.
    pub origin: NonAuthoritativeOrigin,
}

/// A period value exposed for diagnostics and for nothing else.
///
/// The name carries the marker. There is no `From<DeclaredPeriodHint>` for
/// [`CertifiedPeriodGenerator`], no `DeckVector` constructor taking one, no
/// quotient identification accepting one, and no method on this type that
/// returns any of them. A hint may be printed, counted, and used as a starting
/// guess by an explicitly nonauthoritative numerical procedure.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeclaredPeriodHint {
    /// Which axis it concerns.
    pub axis: ParameterAxis,
    /// The value.
    pub value: PositiveFinite,
    /// Where it came from — a source declaration or a bare accessor. Both are
    /// hints here; the distinction is retained because only one of them could
    /// ever be promoted, and then only for a proposition that admits
    /// declarations.
    pub source: PeriodHintSource,
}

/// Where a hint's value came from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PeriodHintSource {
    /// A named source entity declared it.
    SourceDeclaration(SourceDeclaredProvenance),
    /// An accessor returned it and nothing establishes it.
    UnevidencedObservation(NonAuthoritativeOrigin),
}

impl PeriodHintSource {
    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::SourceDeclaration(_) => "source_declaration",
            Self::UnevidencedObservation(_) => "unevidenced_observation",
        }
    }
}

/// The period hints of one face's ambient evidence.
///
/// Possibly empty — a face may declare no period at all — which is why this is
/// a plain collection and not a `NonEmptyVec`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PeriodHintSet {
    hints: Vec<DeclaredPeriodHint>,
}

impl PeriodHintSet {
    /// Iterate the hints.
    pub fn iter(&self) -> impl Iterator<Item = &DeclaredPeriodHint> {
        self.hints.iter()
    }

    /// How many hints.
    pub fn len(&self) -> usize {
        self.hints.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.hints.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Certified generators
// ---------------------------------------------------------------------------

/// A translation of the parameter plane, proved finite and nonzero.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CertifiedUvTranslation {
    du: FiniteF64,
    dv: FiniteF64,
}

/// Why a translation, generator or certificate could not be built. No
/// catch-all.
#[derive(Debug, Clone, PartialEq)]
pub enum GeneratorConstructionError {
    /// A component was `NaN` or infinite.
    NonFiniteComponent {
        /// Which component.
        component: TranslationComponent,
        /// Why it was refused.
        cause: NumericDomainError,
    },
    /// Both components are zero. The identity translation generates the
    /// trivial group and is not a deck generator: it would make Definition 9's
    /// `δ` non-unique for every arc.
    ZeroTranslation,
    /// The translation moves the axis it is not supposed to. The supported
    /// ambient basis schema is axis-aligned: a `u` generator has `dv = 0`.
    TranslationDoesNotLieOnClaimedAxis {
        /// The axis claimed.
        axis: ParameterAxis,
        /// The offending off-axis component.
        off_axis: f64,
    },
    /// The certificate offered is a bare declaration. `FORMAL_SYSTEM.md` §VII
    /// requires `L` to be the period of the map; an exporter's word is not
    /// that, so a declaration cannot authorize deck use.
    DeclarationCannotAuthorizeDeckUse {
        /// Where the declaration came from.
        provenance: SourceDeclaredProvenance,
    },
    /// The certificate itself was refused — a rule without its premises, or a
    /// numerical procedure that did not meet its tolerance.
    CertificateRefused(CertificateConstructionError),
}

impl From<CertificateConstructionError> for GeneratorConstructionError {
    fn from(error: CertificateConstructionError) -> Self {
        Self::CertificateRefused(error)
    }
}

/// Which component of a translation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranslationComponent {
    /// The `u` component.
    Du,
    /// The `v` component.
    Dv,
}

impl CertifiedUvTranslation {
    /// Build a translation, refusing non-finite components and the identity.
    pub fn new(du: f64, dv: f64) -> Result<Self, GeneratorConstructionError> {
        let du =
            FiniteF64::new(du).map_err(|cause| GeneratorConstructionError::NonFiniteComponent {
                component: TranslationComponent::Du,
                cause,
            })?;
        let dv =
            FiniteF64::new(dv).map_err(|cause| GeneratorConstructionError::NonFiniteComponent {
                component: TranslationComponent::Dv,
                cause,
            })?;
        match du.is_zero() && dv.is_zero() {
            true => Err(GeneratorConstructionError::ZeroTranslation),
            false => Ok(Self { du, dv }),
        }
    }

    /// A translation along one axis only.
    pub fn along_axis(
        axis: ParameterAxis,
        magnitude: PositiveFinite,
    ) -> Result<Self, GeneratorConstructionError> {
        match axis {
            ParameterAxis::U => Self::new(magnitude.get(), 0.0),
            ParameterAxis::V => Self::new(0.0, magnitude.get()),
        }
    }

    /// The `u` component.
    pub fn du(self) -> FiniteF64 {
        self.du
    }

    /// The `v` component.
    pub fn dv(self) -> FiniteF64 {
        self.dv
    }

    /// The component on the given axis.
    pub fn on_axis(self, axis: ParameterAxis) -> FiniteF64 {
        match axis {
            ParameterAxis::U => self.du,
            ParameterAxis::V => self.dv,
        }
    }

    /// Scale by an integer, with a finiteness check on the product.
    pub fn scaled(self, factor: i64) -> Result<Self, OperationalFailure> {
        let overflow = || OperationalFailure::ArithmeticOverflow {
            operation: ResourceOperation::DeckVectorScaling,
        };
        let du = FiniteF64::new(self.du.get() * factor as f64).map_err(|_| overflow())?;
        let dv = FiniteF64::new(self.dv.get() * factor as f64).map_err(|_| overflow())?;
        Ok(Self { du, dv })
    }

    /// Componentwise sum, with a finiteness check on the result.
    pub fn plus(self, other: Self) -> Result<Self, OperationalFailure> {
        let overflow = || OperationalFailure::ArithmeticOverflow {
            operation: ResourceOperation::DeckVectorAddition,
        };
        let du = FiniteF64::new(self.du.get() + other.du.get()).map_err(|_| overflow())?;
        let dv = FiniteF64::new(self.dv.get() + other.dv.get()).map_err(|_| overflow())?;
        Ok(Self { du, dv })
    }

    /// The zero translation. Private, and reachable only as the value of a
    /// rank-0 deck displacement.
    fn identity() -> Self {
        Self {
            du: FiniteF64::new(0.0).expect("0.0 is finite"),
            dv: FiniteF64::new(0.0).expect("0.0 is finite"),
        }
    }
}

/// A certificate that authorizes deck use.
///
/// Constructed only from an analytic or certified-numerical basis; the
/// constructor refuses a declaration. A `PeriodCertificate` in hand is
/// therefore itself the proof that
/// [`ANALYTIC_OR_CERTIFIED_NUMERICAL`] was met.
#[derive(Debug, Clone, PartialEq)]
pub struct PeriodCertificate {
    certificate: EvidenceCertificate,
}

impl PeriodCertificate {
    /// Build a deck-use certificate, refusing declarations.
    ///
    /// `pub(super)`: a deck-use authorization is a proof token, and the public
    /// route to one is an introduction rule that supplies the premises.
    pub(super) fn new(
        certificate: EvidenceCertificate,
    ) -> Result<Self, GeneratorConstructionError> {
        match certificate {
            EvidenceCertificate::Declared(provenance) => {
                Err(GeneratorConstructionError::DeclarationCannotAuthorizeDeckUse { provenance })
            }
            other => Ok(Self { certificate: other }),
        }
    }

    /// The certificate behind it.
    pub fn certificate(&self) -> &EvidenceCertificate {
        &self.certificate
    }

    /// The basis. Never [`AuthoritativeBasis::Declared`], by construction.
    pub fn basis(&self) -> AuthoritativeBasis {
        self.certificate.basis()
    }
}

/// A translation proved to generate the deck group along one axis.
///
/// This is the `L` of `FORMAL_SYSTEM.md` Definition 7. Fields are private and
/// the constructor is `pub(super)`, so a value of this type is a proof
/// obligation discharged rather than a struct someone filled in.
#[derive(Debug, Clone, PartialEq)]
pub struct CertifiedPeriodGenerator {
    axis: ParameterAxis,
    delta_uv: CertifiedUvTranslation,
    certificate: PeriodCertificate,
}

impl CertifiedPeriodGenerator {
    /// Build a generator.
    ///
    /// Verifies that the translation is finite and nonzero (by
    /// [`CertifiedUvTranslation`]'s own constructor), that it lies on the
    /// claimed axis under the supported axis-aligned basis schema, and — via
    /// [`PeriodCertificate`] — that the certificate authorizes deck use, which
    /// excludes a declaration.
    ///
    /// `pub(super)`; the public route is an introduction rule.
    pub(super) fn new(
        axis: ParameterAxis,
        delta_uv: CertifiedUvTranslation,
        certificate: PeriodCertificate,
    ) -> Result<Self, GeneratorConstructionError> {
        let off_axis = delta_uv.on_axis(axis.other());
        if !off_axis.is_zero() {
            return Err(
                GeneratorConstructionError::TranslationDoesNotLieOnClaimedAxis {
                    axis,
                    off_axis: off_axis.get(),
                },
            );
        }
        if delta_uv.on_axis(axis).is_zero() {
            // Unreachable through `CertifiedUvTranslation::new`, which rejects
            // the identity, but stated because "nonzero somewhere" and
            // "nonzero on this axis" are different claims.
            return Err(GeneratorConstructionError::ZeroTranslation);
        }
        Ok(Self {
            axis,
            delta_uv,
            certificate,
        })
    }

    /// The axis it generates.
    pub fn axis(&self) -> ParameterAxis {
        self.axis
    }

    /// The translation.
    pub fn translation(&self) -> CertifiedUvTranslation {
        self.delta_uv
    }

    /// The certificate.
    pub fn certificate(&self) -> &PeriodCertificate {
        &self.certificate
    }

    /// The signed magnitude along its own axis.
    pub fn magnitude(&self) -> FiniteF64 {
        self.delta_uv.on_axis(self.axis)
    }
}

/// A proposition: this axis has no period.
///
/// Absence is a claim requiring justification exactly as much as presence
/// does. Carried inside an [`AuthoritativeFact`] so it cannot be asserted
/// without a certificate naming *why*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeriodAbsence {
    /// The axis with no period.
    pub axis: ParameterAxis,
}

// ---------------------------------------------------------------------------
// Introduction rules
// ---------------------------------------------------------------------------

/// Why an introduction rule refused.
#[derive(Debug, Clone, PartialEq)]
pub enum IntroductionError {
    /// The certificate could not be built — a rule without its premises, or a
    /// numerical procedure that did not meet its tolerance.
    Certificate(CertificateConstructionError),
    /// The generator could not be built from the certified value.
    Generator(GeneratorConstructionError),
    /// The evidence was built and the proposition's policy refused its basis.
    /// Unreachable for the rules below, which each supply an admitted basis;
    /// reported rather than asserted away.
    NotAdmittedByPolicy,
}

impl From<CertificateConstructionError> for IntroductionError {
    fn from(error: CertificateConstructionError) -> Self {
        Self::Certificate(error)
    }
}

impl From<GeneratorConstructionError> for IntroductionError {
    fn from(error: GeneratorConstructionError) -> Self {
        Self::Generator(error)
    }
}

fn absence_use_site(axis: ParameterAxis) -> UseSite {
    UseSite {
        stage: SemanticStage::AmbientPeriodResolution,
        predicate: PredicateDescription::of(FormalPredicate::AmbientAxisIsAperiodic(axis)),
    }
}

/// Certify that a plane has no period on an axis.
///
/// The rule is `PlaneHasNoPeriodicDirection` and its premise is
/// `SupportSurfaceIsAPlane`. The premise is discharged by the *argument*: a
/// [`PlaneSchema`] has one constructor, [`super::support::identify_plane`],
/// which takes a `truck_geometry` plane representation and refuses a basis it
/// cannot separate. A caller therefore cannot assert plane-ness by choosing to
/// call this function; it has to present the plane.
///
/// The witness is taken by reference and not read. That is deliberate: this
/// rule is about the *schema*, not about any number in it — a plane is
/// aperiodic whatever its origin and axes are, once the basis is separated,
/// which is the one quantitative fact [`PlaneSchema`]'s existence already
/// carries.
pub fn certify_plane_aperiodicity(
    axis: ParameterAxis,
    _plane: &PlaneSchema,
) -> Result<AuthoritativeFact<PeriodAbsence>, IntroductionError> {
    let certificate = AnalyticCertificate::new(
        AnalyticRule::PlaneHasNoPeriodicDirection,
        NonEmptyVec::one(AnalyticPremise::SupportSurfaceIsAPlane),
    )?;
    let evidence = Evidence::analytic(PeriodAbsence { axis }, certificate);
    AuthoritativeFact::try_from_evidence(evidence, ANALYTIC_ONLY, absence_use_site(axis))
        .map_err(|_| IntroductionError::NotAdmittedByPolicy)
}

/// Certify that a revolved surface's *generatrix* axis has no period.
///
/// Applies only when the generatrix is a straight line, which is what makes a
/// cylinder's and a cone's non-angular axis aperiodic. The rule requires both
/// premises and [`AnalyticCertificate::new`] checks that both are present.
pub fn certify_straight_generatrix_aperiodicity(
    axis: ParameterAxis,
) -> Result<AuthoritativeFact<PeriodAbsence>, IntroductionError> {
    let certificate = AnalyticCertificate::new(
        AnalyticRule::StraightGeneratrixHasNoPeriod,
        NonEmptyVec::new(
            AnalyticPremise::SupportSurfaceIsARevolvedCurve,
            vec![AnalyticPremise::GeneratrixIsAStraightLine],
        ),
    )?;
    let evidence = Evidence::analytic(PeriodAbsence { axis }, certificate);
    AuthoritativeFact::try_from_evidence(evidence, ANALYTIC_ONLY, absence_use_site(axis))
        .map_err(|_| IntroductionError::NotAdmittedByPolicy)
}

/// Certify the `2π` angular generator of a surface of revolution.
///
/// `RevolutedCurve::subs(u, v)` applies `rotation_matrix(v)`, so this is a
/// property of the map rather than of the generatrix — the same rule
/// `domain/lattice.rs::PeriodWitness::ExactRevolutionAngle` records. The
/// premise `SupportSurfaceIsARevolvedCurve` is what this function's
/// applicability asserts.
pub fn certify_revolution_period(
    axis: ParameterAxis,
) -> Result<CertifiedPeriodGenerator, IntroductionError> {
    let magnitude =
        PositiveFinite::new(std::f64::consts::PI * 2.0).expect("2π is positive and finite");
    let translation = CertifiedUvTranslation::along_axis(axis, magnitude)?;
    let certificate = AnalyticCertificate::new(
        AnalyticRule::RevolutionAngularPeriodIsTwoPi,
        NonEmptyVec::one(AnalyticPremise::SupportSurfaceIsARevolvedCurve),
    )?;
    let period_certificate = PeriodCertificate::new(EvidenceCertificate::Analytic(certificate))?;
    Ok(CertifiedPeriodGenerator::new(
        axis,
        translation,
        period_certificate,
    )?)
}

/// Certify the azimuth period of a sphere.
///
/// `Sphere::subs(u, v)` enters the azimuth only through `(cos v, sin v)`, so
/// the azimuth has period `2π` by construction of the primitive, for every
/// latitude. The premise `SupportSurfaceIsASphere` is discharged by the
/// argument: this function's caller has already read the
/// `ExactSphereAzimuth` witness from a `Processor<Sphere, Matrix4>`.
pub fn certify_sphere_azimuth_period(
    axis: ParameterAxis,
) -> Result<CertifiedPeriodGenerator, IntroductionError> {
    let magnitude =
        PositiveFinite::new(std::f64::consts::PI * 2.0).expect("2π is positive and finite");
    let translation = CertifiedUvTranslation::along_axis(axis, magnitude)?;
    let certificate = AnalyticCertificate::new(
        AnalyticRule::SphereAzimuthPeriodIsTwoPi,
        NonEmptyVec::one(AnalyticPremise::SupportSurfaceIsASphere),
    )?;
    let period_certificate = PeriodCertificate::new(EvidenceCertificate::Analytic(certificate))?;
    Ok(CertifiedPeriodGenerator::new(
        axis,
        translation,
        period_certificate,
    )?)
}

/// Certify a period from a self-bounding numerical procedure.
///
/// The certificate states the predicate, the domain, the required tolerance,
/// the achieved bound, the termination and the premises;
/// [`NumericalCertificate::new`] refuses one whose achieved bound is looser
/// than its tolerance. Nothing in Step 1 calls this — no such procedure is
/// implemented — and it exists so that the first one has a typed way to
/// deliver its result rather than inventing one.
pub fn certify_period_numerically(
    axis: ParameterAxis,
    magnitude: PositiveFinite,
    certificate: NumericalCertificate,
) -> Result<CertifiedPeriodGenerator, IntroductionError> {
    let translation = CertifiedUvTranslation::along_axis(axis, magnitude)?;
    let period_certificate =
        PeriodCertificate::new(EvidenceCertificate::CertifiedNumerical(certificate))?;
    Ok(CertifiedPeriodGenerator::new(
        axis,
        translation,
        period_certificate,
    )?)
}

// ---------------------------------------------------------------------------
// Axis evidence
// ---------------------------------------------------------------------------

/// Why a period value was not certified as a generator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeriodCertificationFailure {
    /// The value rests on a bare `ParametricSurface` accessor, which forwards
    /// `curve.period()` and establishes nothing.
    ValueRestsOnUnevidencedAccessor,
    /// The representation is one whose periodicity would have to be read
    /// structurally, and that reading is not implemented. `Sphere`,
    /// `ToroidalSurface`, `SweptCurve` and `OffsetSurface` are in this state —
    /// `CORRECTNESS_GAP_REGISTER.md` C5.
    RepresentationNotStructurallyRead,
    /// This implementation has no certifying rule for that schema at all.
    NoCertifyingRuleForSchema,
}

impl PeriodCertificationFailure {
    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::ValueRestsOnUnevidencedAccessor => "unevidenced_accessor",
            Self::RepresentationNotStructurallyRead => "not_structurally_read",
            Self::NoCertifyingRuleForSchema => "no_certifying_rule",
        }
    }
}

/// One attempt to certify a period on one axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeriodCertificationAttempt {
    /// Which axis.
    pub axis: ParameterAxis,
    /// What was tried and how it ended.
    pub attempt: ResolutionAttempt,
}

/// The two facts that cannot both hold about an axis's period.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PeriodContradictionWitness {
    /// A certified generator exists and the value declared on the same axis
    /// differs from it. Both cannot describe the same deck group.
    DeclaredValueDiffersFromCertifiedGenerator {
        /// The declared magnitude.
        declared: PositiveFinite,
        /// The certified magnitude.
        certified: FiniteF64,
    },
    /// Absence is asserted authoritatively while a generator is certified.
    AbsenceAssertedWithCertifiedGenerator {
        /// The certified magnitude that contradicts the absence.
        certified: FiniteF64,
    },
    /// Two generators claimed to span the lattice lie on one axis, so they
    /// generate a rank-1 group and cannot be a rank-2 basis.
    GeneratorsShareAnAxis {
        /// The shared axis.
        axis: ParameterAxis,
    },
}

impl PeriodContradictionWitness {
    /// The named inconsistency this witness establishes.
    pub fn inconsistency(self) -> super::outcome::Inconsistency {
        use super::outcome::Inconsistency;
        match self {
            Self::DeclaredValueDiffersFromCertifiedGenerator { .. } => {
                Inconsistency::PeriodGeneratorContradiction
            }
            Self::AbsenceAssertedWithCertifiedGenerator { .. } => {
                Inconsistency::PeriodEvidenceContradiction
            }
            Self::GeneratorsShareAnAxis { .. } => {
                Inconsistency::PeriodGeneratorDependenceContradiction
            }
        }
    }
}

/// What is known about the period of one parameter axis.
///
/// Five states, because five are distinguishable and collapsing any two loses
/// a proposition the formal system needs. In particular
/// [`Self::DeclaredButUncertified`] and [`Self::Undetermined`] are **not**
/// [`Self::AuthoritativelyAbsent`]; the whole module exists to keep that
/// distinction.
#[derive(Debug, Clone, PartialEq)]
pub enum PeriodAxisEvidence {
    /// Proved to have no period.
    AuthoritativelyAbsent {
        /// Which axis.
        axis: ParameterAxis,
        /// The proof.
        evidence: AuthoritativeFact<PeriodAbsence>,
    },

    /// Proved to have a period, with a deck generator to show for it.
    Certified {
        /// Which axis.
        axis: ParameterAxis,
        /// What the source declared, if it declared anything. Retained so a
        /// declaration disagreeing with the certificate is detectable.
        declaration: Option<DeclaredPeriod>,
        /// The generator.
        generator: CertifiedPeriodGenerator,
    },

    /// A period value exists and nothing certified it.
    ///
    /// The 1,357 declared-rank-2 faces and 291 of the declared-rank-1 faces
    /// are here. They are not aperiodic; nothing has been established either
    /// way.
    DeclaredButUncertified {
        /// Which axis.
        axis: ParameterAxis,
        /// The value, and whether it is a source declaration or a bare
        /// observation.
        value: UncertifiedPeriodValue,
        /// Why certification did not happen.
        reason: PeriodCertificationFailure,
        /// What was tried.
        attempts: NonEmptyVec<PeriodCertificationAttempt>,
    },

    /// Nothing was stated and nothing was established.
    ///
    /// Distinct from `DeclaredButUncertified` because there is no value to
    /// carry forward as a hint, and distinct from `AuthoritativelyAbsent`
    /// because silence is not a denial.
    Undetermined {
        /// Which axis.
        axis: ParameterAxis,
        /// The proposition left open.
        predicate: PredicateDescription,
        /// What was tried.
        attempts: NonEmptyVec<PeriodCertificationAttempt>,
    },

    /// Two established facts about this axis cannot both hold.
    Contradictory {
        /// Which axis.
        axis: ParameterAxis,
        /// The declaration involved, if any.
        declaration: Option<DeclaredPeriod>,
        /// The generator involved, if any.
        certified: Option<CertifiedPeriodGenerator>,
        /// The two facts that conflict.
        witness: PeriodContradictionWitness,
    },
}

/// An uncertified period value, with its standing retained.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UncertifiedPeriodValue {
    /// A named source entity declared it, and nothing certified it as a
    /// generator.
    Declared(DeclaredPeriod),
    /// A bare accessor returned it.
    Observed(ObservedPeriod),
}

impl UncertifiedPeriodValue {
    /// The value.
    pub fn value(self) -> PositiveFinite {
        match self {
            Self::Declared(declared) => declared.value,
            Self::Observed(observed) => observed.value,
        }
    }

    /// Its standing, as a hint source.
    pub fn hint_source(self) -> PeriodHintSource {
        match self {
            Self::Declared(declared) => PeriodHintSource::SourceDeclaration(declared.provenance),
            Self::Observed(observed) => PeriodHintSource::UnevidencedObservation(observed.origin),
        }
    }
}

impl PeriodAxisEvidence {
    /// Which axis this is about.
    pub fn axis(&self) -> ParameterAxis {
        match self {
            Self::AuthoritativelyAbsent { axis, .. }
            | Self::Certified { axis, .. }
            | Self::DeclaredButUncertified { axis, .. }
            | Self::Undetermined { axis, .. }
            | Self::Contradictory { axis, .. } => *axis,
        }
    }

    /// A short stable tag, for probe records. This is the axis-state census.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::AuthoritativelyAbsent { .. } => "authoritatively_absent",
            Self::Certified { .. } => "certified",
            Self::DeclaredButUncertified { .. } => "declared_but_uncertified",
            Self::Undetermined { .. } => "undetermined",
            Self::Contradictory { .. } => "contradictory",
        }
    }

    /// The period value, as an explicitly nonauthoritative hint.
    ///
    /// Present on every state that has a value, including `Certified` — a
    /// certified axis may still carry a declaration, and a diagnostic that
    /// dropped it could not report a disagreement.
    pub fn diagnostic_hint(&self) -> Option<DeclaredPeriodHint> {
        match self {
            Self::Certified {
                axis, declaration, ..
            }
            | Self::Contradictory {
                axis, declaration, ..
            } => declaration.map(|declared| DeclaredPeriodHint {
                axis: *axis,
                value: declared.value,
                source: PeriodHintSource::SourceDeclaration(declared.provenance),
            }),
            Self::DeclaredButUncertified { axis, value, .. } => Some(DeclaredPeriodHint {
                axis: *axis,
                value: value.value(),
                source: value.hint_source(),
            }),
            Self::AuthoritativelyAbsent { .. } | Self::Undetermined { .. } => None,
        }
    }

    /// The certified generator, if this axis has one.
    ///
    /// Deliberately *not* named `generator`, and deliberately not offered on
    /// [`AmbientPeriodEvidence`]: reading a generator off unresolved evidence
    /// is the operation this module forbids.
    pub fn certified_generator(&self) -> Option<&CertifiedPeriodGenerator> {
        match self {
            Self::Certified { generator, .. } => Some(generator),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Face-level evidence
// ---------------------------------------------------------------------------

/// The ambient period evidence for one face, both axes.
///
/// Note the absence of `generator(axis) -> Option<_>`. Definition 7's `Λ` is a
/// property of the *pair* of axes — a rank claim needs both — and offering a
/// per-axis authoritative generator here would let a caller build a lattice
/// out of one certified axis and one axis about which nothing is known.
#[derive(Debug, Clone, PartialEq)]
pub struct AmbientPeriodEvidence {
    /// The `u` axis.
    pub u: PeriodAxisEvidence,
    /// The `v` axis.
    pub v: PeriodAxisEvidence,
}

impl AmbientPeriodEvidence {
    /// The period values, as explicitly nonauthoritative hints.
    ///
    /// The only route by which a period value leaves this type.
    pub fn diagnostic_hints(&self) -> PeriodHintSet {
        let hints = [self.u.diagnostic_hint(), self.v.diagnostic_hint()]
            .into_iter()
            .flatten()
            .collect();
        PeriodHintSet { hints }
    }

    /// How many axes carry a certified generator. Diagnostic: this is a count,
    /// not a rank, and it is not the authority for any deck operation.
    pub fn authoritative_generator_count(&self) -> usize {
        usize::from(self.u.certified_generator().is_some())
            + usize::from(self.v.certified_generator().is_some())
    }

    /// The derivation chain behind this evidence.
    pub fn provenance(&self) -> ProvenanceSet {
        ProvenanceSet::new(NonEmptyVec::new(
            axis_provenance(ParameterAxis::U, &self.u),
            vec![axis_provenance(ParameterAxis::V, &self.v)],
        ))
    }
}

fn axis_provenance(axis: ParameterAxis, evidence: &PeriodAxisEvidence) -> ProvenanceRecord {
    match evidence {
        PeriodAxisEvidence::Certified { generator, .. } => {
            match generator.certificate().certificate() {
                EvidenceCertificate::Analytic(certificate) => {
                    ProvenanceRecord::AnalyticRuleApplied {
                        rule: certificate.rule(),
                    }
                }
                EvidenceCertificate::CertifiedNumerical(certificate) => {
                    ProvenanceRecord::NumericalCertificateApplied {
                        method: certificate.method(),
                        predicate: certificate.predicate(),
                    }
                }
                // Unreachable: `PeriodCertificate` refuses declarations.
                EvidenceCertificate::Declared(provenance) => {
                    ProvenanceRecord::SupportSurfaceSchema {
                        entity: provenance.source_entity,
                        field: provenance.source_field,
                    }
                }
            }
        }
        PeriodAxisEvidence::AuthoritativelyAbsent { evidence, .. } => {
            match evidence.certificate() {
                EvidenceCertificate::Analytic(certificate) => {
                    ProvenanceRecord::AnalyticRuleApplied {
                        rule: certificate.rule(),
                    }
                }
                EvidenceCertificate::CertifiedNumerical(certificate) => {
                    ProvenanceRecord::NumericalCertificateApplied {
                        method: certificate.method(),
                        predicate: certificate.predicate(),
                    }
                }
                EvidenceCertificate::Declared(provenance) => {
                    ProvenanceRecord::SupportSurfaceSchema {
                        entity: provenance.source_entity,
                        field: provenance.source_field,
                    }
                }
            }
        }
        PeriodAxisEvidence::DeclaredButUncertified { value, .. } => match value {
            UncertifiedPeriodValue::Declared(declared) => ProvenanceRecord::SupportSurfaceSchema {
                entity: declared.provenance.source_entity,
                field: declared.provenance.source_field,
            },
            UncertifiedPeriodValue::Observed(observed) => ProvenanceRecord::LegacyLatticeAxis {
                axis,
                accessor: observed.origin.accessor(),
            },
        },
        PeriodAxisEvidence::Undetermined { .. } | PeriodAxisEvidence::Contradictory { .. } => {
            ProvenanceRecord::LegacyLatticeAxis {
                axis,
                accessor: SurfaceAccessor::for_axis(axis),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Resolved lattices
// ---------------------------------------------------------------------------

/// A certificate that two generators are linearly independent.
///
/// `FORMAL_SYSTEM.md` Definition 7 requires `Λ = LZ^r` with `L` of full rank
/// on its image; Lemma 1's finiteness proof cites that rank directly. Two
/// dependent generators would make `Λ` rank 1 while the code treated it as
/// rank 2, and every `K_ij` computed from it would be wrong.
#[derive(Debug, Clone, PartialEq)]
pub struct GeneratorIndependenceCertificate {
    certificate: EvidenceCertificate,
    first_axis: ParameterAxis,
    second_axis: ParameterAxis,
}

impl GeneratorIndependenceCertificate {
    /// Certify independence from distinct axes under the axis-aligned schema.
    ///
    /// This is analytic *given the schema*, and the schema is the thing being
    /// relied on, so it is recorded as the premise
    /// `RepresentedBasisIsAxisAligned`: [`CertifiedPeriodGenerator::new`] has
    /// already verified that each generator's off-axis component is exactly
    /// zero, so a `u` generator is `(a, 0)` with `a ≠ 0` and a `v` generator
    /// is `(0, b)` with `b ≠ 0`, whose determinant `ab` is nonzero.
    ///
    /// The naming alone would not do it. Two generators are not independent
    /// *because one was called `u`*; they are independent because the schema
    /// makes them axis-aligned and the constructor checked it.
    pub fn from_distinct_axes(
        first: &CertifiedPeriodGenerator,
        second: &CertifiedPeriodGenerator,
    ) -> Result<Self, IndependenceFailure> {
        if first.axis() == second.axis() {
            return Err(IndependenceFailure::SharedAxis(
                PeriodContradictionWitness::GeneratorsShareAnAxis { axis: first.axis() },
            ));
        }
        let certificate = AnalyticCertificate::new(
            AnalyticRule::AxisAlignedGeneratorsAreIndependent,
            NonEmptyVec::one(AnalyticPremise::RepresentedBasisIsAxisAligned),
        )
        .map_err(IndependenceFailure::Certificate)?;
        Ok(Self {
            certificate: EvidenceCertificate::Analytic(certificate),
            first_axis: first.axis(),
            second_axis: second.axis(),
        })
    }

    /// The certificate.
    pub fn certificate(&self) -> &EvidenceCertificate {
        &self.certificate
    }

    /// The two axes.
    pub fn axes(&self) -> (ParameterAxis, ParameterAxis) {
        (self.first_axis, self.second_axis)
    }
}

/// Why independence was not certified.
#[derive(Debug, Clone, PartialEq)]
pub enum IndependenceFailure {
    /// The two generators lie on one axis. A proved contradiction, with its
    /// witness.
    SharedAxis(PeriodContradictionWitness),
    /// The certificate could not be built.
    Certificate(CertificateConstructionError),
}

/// `Λ` is trivial: both axes proved aperiodic.
#[derive(Debug, Clone, PartialEq)]
pub struct CertifiedRank0 {
    u_absent: AuthoritativeFact<PeriodAbsence>,
    v_absent: AuthoritativeFact<PeriodAbsence>,
}

impl CertifiedRank0 {
    /// The `u` absence proof.
    pub fn u_absent(&self) -> &AuthoritativeFact<PeriodAbsence> {
        &self.u_absent
    }

    /// The `v` absence proof.
    pub fn v_absent(&self) -> &AuthoritativeFact<PeriodAbsence> {
        &self.v_absent
    }
}

/// `Λ = LZ`: one axis periodic with a certified generator, the other proved
/// aperiodic.
///
/// Both halves are required. One certified generator plus an axis about which
/// nothing is known is not rank 1 — it is undetermined between rank 1 and
/// rank 2, and Definition 16's `K_ij` differs between them.
#[derive(Debug, Clone, PartialEq)]
pub struct CertifiedRank1 {
    periodic_axis: ParameterAxis,
    generator: CertifiedPeriodGenerator,
    other_axis_absent: AuthoritativeFact<PeriodAbsence>,
}

impl CertifiedRank1 {
    /// The periodic axis.
    pub fn periodic_axis(&self) -> ParameterAxis {
        self.periodic_axis
    }

    /// The generator.
    pub fn generator(&self) -> &CertifiedPeriodGenerator {
        &self.generator
    }

    /// The other axis's absence proof.
    pub fn other_axis_absent(&self) -> &AuthoritativeFact<PeriodAbsence> {
        &self.other_axis_absent
    }
}

/// `Λ = LZ²`: two certified generators with a certified independence proof.
#[derive(Debug, Clone, PartialEq)]
pub struct CertifiedRank2 {
    first: CertifiedPeriodGenerator,
    second: CertifiedPeriodGenerator,
    independence: GeneratorIndependenceCertificate,
}

impl CertifiedRank2 {
    /// The first generator.
    pub fn first(&self) -> &CertifiedPeriodGenerator {
        &self.first
    }

    /// The second generator.
    pub fn second(&self) -> &CertifiedPeriodGenerator {
        &self.second
    }

    /// The independence certificate.
    pub fn independence(&self) -> &GeneratorIndependenceCertificate {
        &self.independence
    }
}

/// The resolved ambient lattice, with its rank encoded structurally.
///
/// A `usize` rank beside an optional generator pair can represent "rank 2 with
/// one generator". This cannot.
#[derive(Debug, Clone, PartialEq)]
pub enum CertifiedAmbientLattice {
    /// `r = 0`.
    Rank0(CertifiedRank0),
    /// `r = 1`.
    Rank1(CertifiedRank1),
    /// `r = 2`.
    Rank2(CertifiedRank2),
}

impl CertifiedAmbientLattice {
    /// The rank. Authoritative, because the value came from
    /// [`resolve_ambient_periods`] rather than from counting non-`None`s.
    pub fn rank(&self) -> u8 {
        match self {
            Self::Rank0(_) => 0,
            Self::Rank1(_) => 1,
            Self::Rank2(_) => 2,
        }
    }

    /// A short stable tag, for probe records.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::Rank0(_) => "rank0",
            Self::Rank1(_) => "rank1",
            Self::Rank2(_) => "rank2",
        }
    }

    /// The shared ambient deck context this lattice authorizes.
    ///
    /// The event contract (GEN-001D) carries one [`DeckContext`] per event — a
    /// lattice-bound token, never a duplicated certificate — and every branch
    /// label must validate against it. The context's [`AmbientLatticeId`] is
    /// derived from the certified generators, so a label certified against a
    /// *different* lattice (even of the same rank) is rejected. The rank is the
    /// certified rank of [`resolve_ambient_periods`].
    pub fn deck_context(&self) -> DeckContext {
        let lattice = match self {
            Self::Rank0(_) => AmbientLatticeId::Rank0,
            Self::Rank1(rank1) => AmbientLatticeId::Rank1 {
                periodic_axis: rank1.generator().axis(),
                signed_period_bits: rank1.generator().magnitude().get().to_bits(),
            },
            Self::Rank2(rank2) => AmbientLatticeId::Rank2 {
                first: translation_bits(rank2.first()),
                second: translation_bits(rank2.second()),
            },
        };
        DeckContext::from_lattice_id(lattice)
    }

    /// The certified basis of `Λ`, for authoritative deck operations.
    pub fn authoritative_basis(&self) -> CertifiedPeriodBasisRef<'_> {
        match self {
            Self::Rank0(lattice) => CertifiedPeriodBasisRef::Rank0(lattice),
            Self::Rank1(lattice) => CertifiedPeriodBasisRef::Rank1(lattice),
            Self::Rank2(lattice) => CertifiedPeriodBasisRef::Rank2(lattice),
        }
    }

    /// Realize a deck displacement as a parameter-plane translation.
    ///
    /// Accepts only a [`DeckDisplacement`], which is only obtainable from the
    /// rank-specific vector types. A rank mismatch is an internal invariant
    /// violation — an operational failure, not a verdict about the face.
    pub fn deck_displacement(
        &self,
        displacement: &DeckDisplacement,
    ) -> Result<CertifiedUvTranslation, OperationalFailure> {
        match (self, displacement) {
            (Self::Rank0(_), DeckDisplacement::Rank0(DeckVector0)) => {
                Ok(CertifiedUvTranslation::identity())
            }
            (Self::Rank1(lattice), DeckDisplacement::Rank1(vector)) => {
                lattice.generator.translation().scaled(vector.get())
            }
            (Self::Rank2(lattice), DeckDisplacement::Rank2(vector)) => {
                let first = lattice.first.translation().scaled(vector.first())?;
                let second = lattice.second.translation().scaled(vector.second())?;
                first.plus(second)
            }
            _ => Err(OperationalFailure::InternalInvariantViolation {
                stage: SemanticStage::AmbientPeriodResolution,
                invariant: InvariantId::DeckVectorRankMatchesLattice,
            }),
        }
    }

    /// Authority to identify quotient vertices under `~_Λ`.
    ///
    /// `FORMAL_SYSTEM.md` §IX: `v ~_Λ w ⟺ x_w = x_v + Lk` **for a certified
    /// `k`**. The authority token exists so a call site's *type* records that
    /// the lattice was resolved, rather than a comment claiming it was.
    pub fn quotient_identification_authority(&self) -> QuotientIdentificationAuthority<'_> {
        QuotientIdentificationAuthority { lattice: self }
    }

    /// Authority to enumerate working-cover copies (Definitions 16-17).
    pub fn cover_enumeration_authority(&self) -> CoverEnumerationAuthority<'_> {
        CoverEnumerationAuthority { lattice: self }
    }
}

/// The `(du, dv)` translation of a certified generator, bit-encoded for the
/// [`AmbientLatticeId`]. Certified generators are finite and nonzero, so the
/// `f64::to_bits` encoding is a valid equality encoding.
fn translation_bits(generator: &CertifiedPeriodGenerator) -> [u64; 2] {
    let translation = generator.translation();
    [
        translation.du().get().to_bits(),
        translation.dv().get().to_bits(),
    ]
}

/// A borrowed view of the certified basis, discriminated by rank.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CertifiedPeriodBasisRef<'a> {
    /// No generators.
    Rank0(&'a CertifiedRank0),
    /// One generator.
    Rank1(&'a CertifiedRank1),
    /// Two independent generators.
    Rank2(&'a CertifiedRank2),
}

/// Permission to perform `~_Λ` vertex identification.
///
/// Obtainable only from a [`CertifiedAmbientLattice`]. There is no constructor
/// taking a [`DeclaredPeriodHint`], an [`AmbientPeriodEvidence`], or an
/// `Option<f64>`.
#[derive(Debug, Clone, Copy)]
pub struct QuotientIdentificationAuthority<'a> {
    lattice: &'a CertifiedAmbientLattice,
}

impl<'a> QuotientIdentificationAuthority<'a> {
    /// The lattice authorizing the identification.
    pub fn lattice(&self) -> &'a CertifiedAmbientLattice {
        self.lattice
    }
}

/// Permission to enumerate working-cover copies.
///
/// Same construction rule as [`QuotientIdentificationAuthority`].
#[derive(Debug, Clone, Copy)]
pub struct CoverEnumerationAuthority<'a> {
    lattice: &'a CertifiedAmbientLattice,
}

impl<'a> CoverEnumerationAuthority<'a> {
    /// The lattice authorizing the enumeration.
    pub fn lattice(&self) -> &'a CertifiedAmbientLattice {
        self.lattice
    }
}

/// One admissible reading of a face's ambient lattice.
///
/// Structural, not a digest: an ambiguity report has to support asking *how*
/// two readings differ, and a `u64` cannot answer that.
#[derive(Debug, Clone, PartialEq)]
pub enum AmbientAlternative {
    /// The reading assigns rank 0.
    Rank0,
    /// The reading assigns rank 1, periodic on this axis.
    Rank1 {
        /// The periodic axis.
        axis: ParameterAxis,
    },
    /// The reading assigns rank 2.
    Rank2 {
        /// The magnitude on `u`.
        u_magnitude: PositiveFinite,
        /// The magnitude on `v`.
        v_magnitude: PositiveFinite,
    },
}

impl AmbientAlternative {
    /// The rank this reading assigns.
    pub fn rank(&self) -> u8 {
        match self {
            Self::Rank0 => 0,
            Self::Rank1 { .. } => 1,
            Self::Rank2 { .. } => 2,
        }
    }
}

// ---------------------------------------------------------------------------
// Deck vectors
// ---------------------------------------------------------------------------

/// The single element of `Z^0`.
///
/// There is exactly one rank-zero deck displacement and it is the identity.
/// No constructor takes components: a `DeckVector0::new(3)` would be a
/// displacement in a group with one element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeckVector0;

/// An element of `Z^1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeckVector1(i64);

impl DeckVector1 {
    /// From an integer copy index.
    pub fn new(value: i64) -> Self {
        Self(value)
    }

    /// The copy index.
    pub fn get(self) -> i64 {
        self.0
    }

    /// Checked addition. Overflow is an [`OperationalFailure`], never a
    /// saturation and never a wrap: a silently saturated deck index would make
    /// `ψ(w) − ψ(v) = d(h)` false while the solver believed it.
    pub fn checked_add(self, other: Self) -> Result<Self, OperationalFailure> {
        self.0
            .checked_add(other.0)
            .map(Self)
            .ok_or(OperationalFailure::ArithmeticOverflow {
                operation: ResourceOperation::DeckVectorAddition,
            })
    }

    /// Checked subtraction.
    pub fn checked_sub(self, other: Self) -> Result<Self, OperationalFailure> {
        self.0
            .checked_sub(other.0)
            .map(Self)
            .ok_or(OperationalFailure::ArithmeticOverflow {
                operation: ResourceOperation::DeckVectorSubtraction,
            })
    }

    /// The `w_max` norm of Definition 6, checked.
    pub fn checked_norm(self) -> Result<u64, OperationalFailure> {
        self.0
            .checked_abs()
            .map(i64::unsigned_abs)
            .ok_or(OperationalFailure::ArithmeticOverflow {
                operation: ResourceOperation::DeckDisplacementNorm,
            })
    }
}

/// An element of `Z^2`, in the basis of a [`CertifiedRank2`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeckVector2 {
    first: i64,
    second: i64,
}

impl DeckVector2 {
    /// From two copy indices.
    pub fn new(first: i64, second: i64) -> Self {
        Self { first, second }
    }

    /// The first coordinate.
    pub fn first(self) -> i64 {
        self.first
    }

    /// The second coordinate.
    pub fn second(self) -> i64 {
        self.second
    }

    /// Checked componentwise addition.
    pub fn checked_add(self, other: Self) -> Result<Self, OperationalFailure> {
        let overflow = OperationalFailure::ArithmeticOverflow {
            operation: ResourceOperation::DeckVectorAddition,
        };
        let first = self.first.checked_add(other.first).ok_or(overflow)?;
        let second = self.second.checked_add(other.second).ok_or(overflow)?;
        Ok(Self { first, second })
    }

    /// Checked componentwise subtraction.
    pub fn checked_sub(self, other: Self) -> Result<Self, OperationalFailure> {
        let overflow = OperationalFailure::ArithmeticOverflow {
            operation: ResourceOperation::DeckVectorSubtraction,
        };
        let first = self.first.checked_sub(other.first).ok_or(overflow)?;
        let second = self.second.checked_sub(other.second).ok_or(overflow)?;
        Ok(Self { first, second })
    }

    /// The supremum norm, checked.
    pub fn checked_norm(self) -> Result<u64, OperationalFailure> {
        let overflow = OperationalFailure::ArithmeticOverflow {
            operation: ResourceOperation::DeckDisplacementNorm,
        };
        let first = self.first.checked_abs().ok_or(overflow)?.unsigned_abs();
        let second = self.second.checked_abs().ok_or(overflow)?.unsigned_abs();
        Ok(first.max(second))
    }
}

/// A deck displacement of any rank.
///
/// The rank tag is what [`CertifiedAmbientLattice::deck_displacement`] matches
/// against, so a rank-1 vector cannot be silently read as the first coordinate
/// of a rank-2 one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeckDisplacement {
    /// In `Z^0`.
    Rank0(DeckVector0),
    /// In `Z^1`.
    Rank1(DeckVector1),
    /// In `Z^2`.
    Rank2(DeckVector2),
}

impl DeckDisplacement {
    /// The `w_max` norm of Definition 6, checked. Zero for rank 0.
    pub fn checked_norm(self) -> Result<u64, OperationalFailure> {
        match self {
            Self::Rank0(DeckVector0) => Ok(0),
            Self::Rank1(vector) => vector.checked_norm(),
            Self::Rank2(vector) => vector.checked_norm(),
        }
    }
}

// ---------------------------------------------------------------------------
// Resolution
// ---------------------------------------------------------------------------

/// What one axis contributes to a rank determination.
enum AxisContribution {
    Absent(AuthoritativeFact<PeriodAbsence>),
    Generator(CertifiedPeriodGenerator),
}

/// Resolve two axes of period evidence into a certified ambient lattice.
///
/// The rules, restated from `FORMAL_SYSTEM.md` Definitions 6-8:
///
/// | evidence | outcome |
/// |---|---|
/// | both axes authoritatively absent | `Resolved(Rank0)` |
/// | one certified, other authoritatively absent | `Resolved(Rank1)` |
/// | two certified independent generators | `Resolved(Rank2)` |
/// | any axis declared-but-uncertified | `Unresolved` |
/// | any axis undetermined | `Unresolved` |
/// | contradictory evidence | `Inconsistent` |
/// | proved rank above `r_max` | `Unsupported` |
///
/// Both axes are always required to determine the rank — the rank is the count
/// of periodic axes, and an axis in an unresolved state could contribute
/// either 0 or 1 — so there is no case in which an unresolved axis can be
/// ignored as irrelevant.
///
/// **A declared-but-uncertified axis is never converted into absence.** That
/// conversion is what would classify the corpus's 1,357 declared-rank-2 and
/// 291 declared-rank-1 faces as `Rank0`.
pub fn resolve_ambient_periods(
    evidence: AmbientPeriodEvidence,
    envelope: &FormalEnvelope,
    face: FaceKey,
) -> StageEvaluation<CertifiedAmbientLattice> {
    let stage = SemanticStage::AmbientPeriodResolution;
    let provenance = evidence.provenance();

    // 1. Contradiction first. A contradictory axis is a proved fact about the
    //    evidence, and it is not made less true by the other axis's state.
    for axis_evidence in [&evidence.u, &evidence.v] {
        if let PeriodAxisEvidence::Contradictory { witness, .. } = axis_evidence {
            return Ok(StageOutcome::Inconsistent(
                InconsistencyReport::from_witness(
                    face,
                    stage,
                    ContradictionWitness::AmbientPeriod(*witness),
                    provenance,
                ),
            ));
        }
    }

    // 2. Each axis must contribute either an absence proof or a generator.
    //    Anything else leaves the rank undetermined.
    let mut contributions = Vec::with_capacity(2);
    for axis_evidence in [evidence.u, evidence.v] {
        match axis_evidence {
            PeriodAxisEvidence::AuthoritativelyAbsent { evidence, .. } => {
                contributions.push(AxisContribution::Absent(evidence));
            }
            PeriodAxisEvidence::Certified { generator, .. } => {
                contributions.push(AxisContribution::Generator(generator));
            }
            PeriodAxisEvidence::DeclaredButUncertified { axis, attempts, .. } => {
                return Ok(StageOutcome::Unresolved(UnresolvedReport::new(
                    face,
                    stage,
                    PredicateDescription::of(FormalPredicate::DeclaredPeriodIsADeckGenerator(axis)),
                    UnresolvedReason::DeclaredPeriodNotCertified,
                    certification_attempts(&attempts),
                    provenance,
                )));
            }
            PeriodAxisEvidence::Undetermined {
                predicate,
                attempts,
                ..
            } => {
                return Ok(StageOutcome::Unresolved(UnresolvedReport::new(
                    face,
                    stage,
                    predicate,
                    UnresolvedReason::PeriodAbsenceNotEstablished,
                    certification_attempts(&attempts),
                    provenance,
                )));
            }
            // Handled above; repeated so the match stays exhaustive without a
            // wildcard that would swallow a variant added later.
            PeriodAxisEvidence::Contradictory { witness, .. } => {
                return Ok(StageOutcome::Inconsistent(
                    InconsistencyReport::from_witness(
                        face,
                        stage,
                        ContradictionWitness::AmbientPeriod(witness),
                        provenance,
                    ),
                ));
            }
        }
    }

    // 3. The rank is now proved: it is read off the resolved contributions and
    //    needed no search, which is what `StructuralFromResolvedType` records.
    //    Test it against `r_max` before building anything — `Unsupported` is a
    //    claim about the face, and this is the point at which it is earned.
    let rank = contributions
        .iter()
        .filter(|contribution| matches!(contribution, AxisContribution::Generator(_)))
        .count() as u8;
    let observation = BoundObservation::Exact(
        ExactCount::from_completed_count(
            u128::from(rank),
            MeasurementSubject::LatticeRank,
            CountingProcedure::StructuralFromResolvedType,
        )
        .map_err(|_| OperationalFailure::InternalInvariantViolation {
            stage,
            invariant: InvariantId::MeasurementSubjectMatchesClause,
        })?,
    );
    let admitted = envelope.check_lattice_rank(observation).map_err(|_| {
        OperationalFailure::InternalInvariantViolation {
            stage,
            invariant: InvariantId::MeasurementSubjectMatchesClause,
        }
    })?;
    if let Err(violation) = admitted {
        return Ok(StageOutcome::Unsupported(UnsupportedReport::new(
            face,
            stage,
            UnsupportedCause::EnvelopeExceeded(violation),
            provenance,
        )));
    }

    let mut contributions = contributions.into_iter();
    let u = contributions.next().expect("two axes were pushed");
    let v = contributions.next().expect("two axes were pushed");

    let lattice = match (u, v) {
        (AxisContribution::Absent(u_absent), AxisContribution::Absent(v_absent)) => {
            CertifiedAmbientLattice::Rank0(CertifiedRank0 { u_absent, v_absent })
        }
        (AxisContribution::Generator(generator), AxisContribution::Absent(other_axis_absent))
        | (AxisContribution::Absent(other_axis_absent), AxisContribution::Generator(generator)) => {
            CertifiedAmbientLattice::Rank1(CertifiedRank1 {
                periodic_axis: generator.axis(),
                generator,
                other_axis_absent,
            })
        }
        (AxisContribution::Generator(first), AxisContribution::Generator(second)) => {
            match GeneratorIndependenceCertificate::from_distinct_axes(&first, &second) {
                Ok(independence) => CertifiedAmbientLattice::Rank2(CertifiedRank2 {
                    first,
                    second,
                    independence,
                }),
                // Two generators on one axis is a proved contradiction, not an
                // unresolved question: the pair is exhibited.
                Err(IndependenceFailure::SharedAxis(witness)) => {
                    return Ok(StageOutcome::Inconsistent(
                        InconsistencyReport::from_witness(
                            face,
                            stage,
                            ContradictionWitness::AmbientPeriod(witness),
                            provenance,
                        ),
                    ));
                }
                // The certificate's own premises failed, which is a defect
                // here rather than a fact about the face.
                Err(IndependenceFailure::Certificate(_)) => {
                    return Err(OperationalFailure::InternalInvariantViolation {
                        stage,
                        invariant: InvariantId::Rank2GeneratorsLieOnDistinctAxes,
                    });
                }
            }
        }
    };

    // The rank-1 invariant the type cannot express: the generator must lie on
    // the axis the lattice names as periodic. Violating it is a defect here,
    // so it is an operational failure rather than a verdict.
    if let CertifiedAmbientLattice::Rank1(rank1) = &lattice {
        if rank1.generator.axis() != rank1.periodic_axis {
            return Err(OperationalFailure::InternalInvariantViolation {
                stage,
                invariant: InvariantId::Rank1GeneratorLiesOnPeriodicAxis,
            });
        }
    }

    Ok(StageOutcome::Resolved(lattice))
}

/// Project period-certification attempts onto the generic attempt vocabulary
/// the reports use.
fn certification_attempts(
    attempts: &NonEmptyVec<PeriodCertificationAttempt>,
) -> NonEmptyVec<ResolutionAttempt> {
    attempts.map(|attempt| attempt.attempt)
}

// ---------------------------------------------------------------------------
// Adapter from the legacy lattice representation
// ---------------------------------------------------------------------------

/// Which representation an [`AmbientPeriodEvidence`] was adapted from.
///
/// This is the field that decides whether an absent axis can be authoritative.
/// It exists because [`super::super::domain::lattice::CertifiedLattice`] does
/// not carry it: that type's `AxisPeriodStatus::NonPeriodic` is produced both
/// by `CertifiedLattice::NON_PERIODIC` — an analytic claim about a plane — and
/// by `from_unevidenced_accessor(None)`, where a bare accessor returned
/// nothing. The two are indistinguishable once constructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LatticeOrigin {
    /// A `CertifiedLattice` whose constructor is not recorded.
    ///
    /// Under this origin `NonPeriodic` maps to
    /// [`PeriodAxisEvidence::Undetermined`], never to
    /// [`PeriodAxisEvidence::AuthoritativelyAbsent`]. This is not
    /// conservatism for its own sake: `look`'s `lattice_of` routes
    /// `ToroidalSurface` through `from_unevidenced_accessors`, and a torus
    /// whose accessor returns `None` on an axis is doubly periodic all the
    /// same. An absence rule here would be false on real corpus faces.
    UnattributedLegacyLattice,
}

impl LatticeOrigin {
    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::UnattributedLegacyLattice => "unattributed_legacy_lattice",
        }
    }
}

/// Why the adapter could not build evidence for an axis.
#[derive(Debug, Clone, PartialEq)]
pub enum AdapterError {
    /// A period value was not a positive finite number. The legacy constructor
    /// already filters these, so this should not occur; it is reported rather
    /// than assumed away.
    PeriodValueOutOfDomain {
        /// Which axis.
        axis: ParameterAxis,
        /// Why it was refused.
        cause: NumericDomainError,
    },
    /// A certified generator could not be rebuilt from the legacy witness.
    GeneratorNotReconstructible {
        /// Which axis.
        axis: ParameterAxis,
        /// Why.
        cause: IntroductionError,
    },
}

impl AdapterError {
    /// A short stable tag, for probe records.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::PeriodValueOutOfDomain { .. } => "period_value_out_of_domain",
            Self::GeneratorNotReconstructible { .. } => "generator_not_reconstructible",
        }
    }
}

/// Adapt one axis of the legacy representation.
///
/// The mapping, and the reasoning behind each arm:
///
/// | legacy `AxisPeriodStatus` | formal state | why |
/// |---|---|---|
/// | `Exact { period, witness }` | `Certified` | the witness names an analytic rule about the map, so the generator is *re-derived* from that rule rather than trusted as a number |
/// | `Uncertified { declared }` | `DeclaredButUncertified` (observed) | a value exists; nothing established it, and it has no source entity behind it |
/// | `NonPeriodic` | `Undetermined` | the constructor that produced it is not recorded |
///
/// The third row is the whole point. `NonPeriodic` *may* have come from a
/// plane, in which case absence is analytic — but it may equally have come
/// from an accessor returning `None` on a torus. Choosing
/// `AuthoritativelyAbsent` would fabricate a basis; choosing `Undetermined`
/// records exactly what is known, which is nothing.
///
/// The second row is why [`ObservedPeriod`] exists: an accessor result has no
/// source entity and no field path, so it cannot be spelled as a
/// [`SourceDeclaredProvenance`] and cannot become `Declared` evidence.
pub fn ambient_axis_evidence_from_legacy(
    axis: ParameterAxis,
    status: super::super::domain::lattice::AxisPeriodStatus,
    origin: LatticeOrigin,
) -> Result<PeriodAxisEvidence, AdapterError> {
    use super::super::domain::lattice::AxisPeriodStatus;

    let accessor = SurfaceAccessor::for_axis(axis);
    let non_authoritative = match origin {
        LatticeOrigin::UnattributedLegacyLattice => {
            NonAuthoritativeOrigin::LegacyLatticeWithErasedOrigin { accessor }
        }
    };
    let attempt = |outcome: AttemptOutcome, method: ResolutionMethod| {
        NonEmptyVec::one(PeriodCertificationAttempt {
            axis,
            attempt: ResolutionAttempt { method, outcome },
        })
    };

    match status {
        // The legacy witnesses name the analytic rule they were read from, so
        // this arm re-derives a real certificate through the introduction rule
        // rather than trusting the stored number.
        AxisPeriodStatus::Exact { period, witness } => {
            let generator = match witness {
                super::super::domain::lattice::PeriodWitness::ExactRevolutionAngle => {
                    certify_revolution_period(axis).map_err(|cause| {
                        AdapterError::GeneratorNotReconstructible { axis, cause }
                    })?
                }
                super::super::domain::lattice::PeriodWitness::ExactSphereAzimuth => {
                    certify_sphere_azimuth_period(axis).map_err(|cause| {
                        AdapterError::GeneratorNotReconstructible { axis, cause }
                    })?
                }
                // The composition layer certified a source-declared spline
                // closure by checking the converted evaluator's seam
                // identification (`S(u,a)==S(u,b)` and `Sv(u,a)==Sv(u,b)` over
                // the active interval `[a,b]`). That is not an analytic rule
                // about the parameterisation — a spline is not a rotation — so
                // the formal Step-1 model has no rule that can *re-derive* the
                // generator from the representation. The honest mapping is
                // `DeclaredButUncertified`: the period value is carried, and
                // this model records that it has no certifying rule for a
                // spline. It never treats the value as a certified generator
                // of its own.
                super::super::domain::lattice::PeriodWitness::SourceDeclaredClosedSplineAxis => {
                    let value = PositiveFinite::new(period)
                        .map_err(|cause| AdapterError::PeriodValueOutOfDomain { axis, cause })?;
                    return Ok(PeriodAxisEvidence::DeclaredButUncertified {
                        axis,
                        value: UncertifiedPeriodValue::Observed(ObservedPeriod {
                            value,
                            origin: non_authoritative,
                        }),
                        reason: PeriodCertificationFailure::NoCertifyingRuleForSchema,
                        attempts: attempt(
                            AttemptOutcome::NoCertifyingRuleForRepresentation,
                            ResolutionMethod::RepresentationDerivedWitness,
                        ),
                    });
                }
            };
            // If the legacy number disagrees with the re-derived 2π, that is a
            // contradiction between two established facts, not a reason to
            // silently prefer one.
            if let Ok(value) = PositiveFinite::new(period) {
                let certified = generator.magnitude();
                if (value.get() - certified.get()).abs() > f64::EPSILON * 8.0 {
                    return Ok(PeriodAxisEvidence::Contradictory {
                        axis,
                        declaration: None,
                        certified: Some(generator),
                        witness:
                            PeriodContradictionWitness::DeclaredValueDiffersFromCertifiedGenerator {
                                declared: value,
                                certified,
                            },
                    });
                }
            }
            Ok(PeriodAxisEvidence::Certified {
                axis,
                // No `DeclaredPeriod`: the legacy type has no source entity to
                // attribute the value to, so there is nothing to record here
                // that would not be invented.
                declaration: None,
                generator,
            })
        }

        AxisPeriodStatus::Uncertified { declared } => {
            let value = PositiveFinite::new(declared)
                .map_err(|cause| AdapterError::PeriodValueOutOfDomain { axis, cause })?;
            Ok(PeriodAxisEvidence::DeclaredButUncertified {
                axis,
                value: UncertifiedPeriodValue::Observed(ObservedPeriod {
                    value,
                    origin: non_authoritative,
                }),
                reason: PeriodCertificationFailure::ValueRestsOnUnevidencedAccessor,
                attempts: attempt(
                    AttemptOutcome::NoCertifyingRuleForRepresentation,
                    ResolutionMethod::RepresentationDerivedWitness,
                ),
            })
        }

        AxisPeriodStatus::NonPeriodic => Ok(PeriodAxisEvidence::Undetermined {
            axis,
            predicate: PredicateDescription::of(FormalPredicate::AmbientAxisIsAperiodic(axis)),
            attempts: attempt(
                AttemptOutcome::EvidenceErasedBeforeThisStage,
                ResolutionMethod::LegacyCertifiedLatticeAccessor,
            ),
        }),
    }
}

/// Adapt a whole legacy lattice.
///
/// Diagnostic. Nothing in production consumes the result.
pub fn ambient_evidence_from_legacy(
    lattice: &super::super::domain::lattice::CertifiedLattice,
    origin: LatticeOrigin,
) -> Result<AmbientPeriodEvidence, AdapterError> {
    Ok(AmbientPeriodEvidence {
        u: ambient_axis_evidence_from_legacy(ParameterAxis::U, lattice.u, origin)?,
        v: ambient_axis_evidence_from_legacy(ParameterAxis::V, lattice.v, origin)?,
    })
}

// ---------------------------------------------------------------------------
// Bridge from the authoritative support-surface schema
// ---------------------------------------------------------------------------

/// Both axes of a plane, proved absent.
///
/// The narrow evidence-ingestion path Step 1's census called for. It reads the
/// *representation* — through [`PlaneSchema`], whose only constructor inspects
/// a real `truck_geometry` plane — rather than the legacy lattice, which has
/// already erased which of its two producers said `NonPeriodic`.
///
/// This is the only function in the subtree that can produce an
/// `AuthoritativelyAbsent` pair, and it can only be called with a plane in
/// hand.
pub fn ambient_evidence_from_plane_schema(
    plane: &PlaneSchema,
) -> Result<AmbientPeriodEvidence, IntroductionError> {
    Ok(AmbientPeriodEvidence {
        u: PeriodAxisEvidence::AuthoritativelyAbsent {
            axis: ParameterAxis::U,
            evidence: certify_plane_aperiodicity(ParameterAxis::U, plane)?,
        },
        v: PeriodAxisEvidence::AuthoritativelyAbsent {
            axis: ParameterAxis::V,
            evidence: certify_plane_aperiodicity(ParameterAxis::V, plane)?,
        },
    })
}

/// Build ambient evidence from whatever the composition layer established,
/// falling back to the conservative legacy adapter.
///
/// The dispatch is the point of the whole bridge, so it is written once here
/// rather than at each call site:
///
/// - a structurally identified plane takes the analytic route and can resolve
///   rank 0;
/// - **everything else** — including a surface whose legacy lattice says
///   `NonPeriodic` — takes the legacy adapter, which maps `NonPeriodic` to
///   `Undetermined` and therefore cannot resolve a rank at all.
///
/// There is deliberately no arm that consults the legacy lattice *first*. A
/// torus reaching `look::lattice_of` goes through `from_unevidenced_accessors`,
/// and its accessors returning `None` would look exactly like a plane's
/// analytic absence at that layer.
pub fn ambient_evidence_from_schema(
    schema: &SupportSurfaceSchema,
    lattice: &super::super::domain::lattice::CertifiedLattice,
    origin: LatticeOrigin,
) -> Result<AmbientPeriodEvidence, AmbientEvidenceError> {
    match schema {
        SupportSurfaceSchema::Plane(plane) => {
            ambient_evidence_from_plane_schema(plane).map_err(AmbientEvidenceError::Introduction)
        }
        SupportSurfaceSchema::NotStructurallyIdentified(_) => {
            ambient_evidence_from_legacy(lattice, origin).map_err(AmbientEvidenceError::Adapter)
        }
    }
}

/// Why ambient evidence could not be built for a face.
#[derive(Debug, Clone, PartialEq)]
pub enum AmbientEvidenceError {
    /// The legacy adapter refused. See [`AdapterError`].
    Adapter(AdapterError),
    /// An analytic introduction rule refused. A defect here rather than a fact
    /// about the face: the plane rule's premises are supplied by this module.
    Introduction(IntroductionError),
}

impl AmbientEvidenceError {
    /// A short stable tag, for probe records.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::Adapter(error) => error.tag(),
            Self::Introduction(_) => "introduction_rule_refused",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::envelope::PolicyInstanceId;
    use super::super::outcome::{DocumentScope, Inconsistency, ShellKey};
    use super::*;
    use crate::tessellation::domain::lattice::{Axis, AxisPeriodStatus, CertifiedLattice};

    fn a_face() -> FaceKey {
        FaceKey {
            document: DocumentScope::SingleDocumentRun,
            shell: ShellKey::new(0),
            source_face_id: None,
            declared_face_index: 0,
        }
    }

    /// A test policy. Not a production value; none is specified.
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
        .expect("well-formed")
    }

    /// A structurally identified plane, for the rules that require the
    /// premise to be witnessed rather than asserted.
    fn a_test_plane() -> PlaneSchema {
        let plane = truck_geometry::prelude::Plane::new(
            truck_geometry::prelude::Point3::new(0.0, 0.0, 0.0),
            truck_geometry::prelude::Point3::new(1.0, 0.0, 0.0),
            truck_geometry::prelude::Point3::new(0.0, 1.0, 0.0),
        );
        *super::super::support::identify_plane(&plane)
            .plane()
            .expect("a unit plane identifies")
    }

    fn absent(axis: ParameterAxis) -> PeriodAxisEvidence {
        PeriodAxisEvidence::AuthoritativelyAbsent {
            axis,
            evidence: certify_plane_aperiodicity(axis, &a_test_plane())
                .expect("a plane's absence is analytic"),
        }
    }

    fn certified(axis: ParameterAxis) -> PeriodAxisEvidence {
        PeriodAxisEvidence::Certified {
            axis,
            declaration: None,
            generator: certify_revolution_period(axis).expect("2π"),
        }
    }

    fn an_attempt(axis: ParameterAxis) -> NonEmptyVec<PeriodCertificationAttempt> {
        NonEmptyVec::one(PeriodCertificationAttempt {
            axis,
            attempt: ResolutionAttempt {
                method: ResolutionMethod::LegacyCertifiedLatticeAccessor,
                outcome: AttemptOutcome::EvidenceErasedBeforeThisStage,
            },
        })
    }

    fn declared_but_uncertified(axis: ParameterAxis, value: f64) -> PeriodAxisEvidence {
        PeriodAxisEvidence::DeclaredButUncertified {
            axis,
            value: UncertifiedPeriodValue::Observed(ObservedPeriod {
                value: PositiveFinite::new(value).expect("positive"),
                origin: NonAuthoritativeOrigin::UnevidencedSurfaceAccessor {
                    accessor: SurfaceAccessor::for_axis(axis),
                },
            }),
            reason: PeriodCertificationFailure::ValueRestsOnUnevidencedAccessor,
            attempts: an_attempt(axis),
        }
    }

    fn undetermined(axis: ParameterAxis) -> PeriodAxisEvidence {
        PeriodAxisEvidence::Undetermined {
            axis,
            predicate: PredicateDescription::of(FormalPredicate::AmbientAxisIsAperiodic(axis)),
            attempts: an_attempt(axis),
        }
    }

    // -- 19.2 Ambient-axis semantics ---------------------------------------

    #[test]
    fn declared_but_uncertified_is_not_absent() {
        let evidence = AmbientPeriodEvidence {
            u: declared_but_uncertified(ParameterAxis::U, 1.5),
            v: absent(ParameterAxis::V),
        };
        match resolve_ambient_periods(evidence, &a_test_envelope(), a_face()).unwrap() {
            StageOutcome::Unresolved(report) => {
                assert_eq!(
                    report.reason(),
                    UnresolvedReason::DeclaredPeriodNotCertified
                );
            }
            other => panic!(
                "a declared uncertified axis must not resolve: {}",
                other.tag()
            ),
        }
    }

    #[test]
    fn undetermined_is_not_absent() {
        let evidence = AmbientPeriodEvidence {
            u: undetermined(ParameterAxis::U),
            v: absent(ParameterAxis::V),
        };
        match resolve_ambient_periods(evidence, &a_test_envelope(), a_face()).unwrap() {
            StageOutcome::Unresolved(report) => {
                assert_eq!(
                    report.reason(),
                    UnresolvedReason::PeriodAbsenceNotEstablished
                );
            }
            other => panic!("an undetermined axis must not resolve: {}", other.tag()),
        }
    }

    #[test]
    fn certified_period_contains_a_nonzero_generator() {
        let generator = certify_revolution_period(ParameterAxis::V).expect("2π");
        assert_eq!(generator.axis(), ParameterAxis::V);
        assert!(!generator.magnitude().is_zero());
        assert_eq!(generator.magnitude().get(), std::f64::consts::PI * 2.0);
        assert!(
            generator.translation().on_axis(ParameterAxis::U).is_zero(),
            "a v generator does not move u"
        );
        // The identity is refused outright, so a "certified" zero period is
        // unrepresentable rather than merely discouraged.
        assert_eq!(
            CertifiedUvTranslation::new(0.0, 0.0),
            Err(GeneratorConstructionError::ZeroTranslation)
        );
    }

    #[test]
    fn declared_period_hint_is_not_a_certified_generator() {
        let axis_evidence = declared_but_uncertified(ParameterAxis::U, 1.5);
        let hint = axis_evidence
            .diagnostic_hint()
            .expect("an uncertified axis has a hint");
        assert_eq!(hint.value.get(), 1.5);
        assert_eq!(hint.source.tag(), "unevidenced_observation");
        // The generator accessor answers `None` for the same axis: the hint
        // and the generator are different propositions and only one holds.
        assert!(axis_evidence.certified_generator().is_none());
    }

    #[test]
    fn contradictory_axis_evidence_yields_inconsistent() {
        let generator = certify_revolution_period(ParameterAxis::U).expect("2π");
        let declared = PositiveFinite::new(4.0).expect("positive");
        let magnitude = generator.magnitude();
        let evidence = AmbientPeriodEvidence {
            u: PeriodAxisEvidence::Contradictory {
                axis: ParameterAxis::U,
                declaration: None,
                certified: Some(generator),
                witness: PeriodContradictionWitness::DeclaredValueDiffersFromCertifiedGenerator {
                    declared,
                    certified: magnitude,
                },
            },
            v: absent(ParameterAxis::V),
        };
        match resolve_ambient_periods(evidence, &a_test_envelope(), a_face()).unwrap() {
            StageOutcome::Inconsistent(report) => {
                assert_eq!(report.reason(), Inconsistency::PeriodGeneratorContradiction);
            }
            other => panic!("expected Inconsistent, got {}", other.tag()),
        }
    }

    // -- 19.3 Rank semantics -----------------------------------------------

    #[test]
    fn rank_zero_requires_two_authoritative_absence_facts() {
        let evidence = AmbientPeriodEvidence {
            u: absent(ParameterAxis::U),
            v: absent(ParameterAxis::V),
        };
        match resolve_ambient_periods(evidence, &a_test_envelope(), a_face()).unwrap() {
            StageOutcome::Resolved(CertifiedAmbientLattice::Rank0(rank0)) => {
                assert_eq!(rank0.u_absent().get().axis, ParameterAxis::U);
                assert_eq!(rank0.v_absent().get().axis, ParameterAxis::V);
                assert_eq!(rank0.u_absent().basis(), AuthoritativeBasis::Analytic);
            }
            other => panic!("expected Rank0, got {}", other.tag()),
        }
        // One absence and one silence is not rank zero.
        let half = AmbientPeriodEvidence {
            u: absent(ParameterAxis::U),
            v: undetermined(ParameterAxis::V),
        };
        assert!(matches!(
            resolve_ambient_periods(half, &a_test_envelope(), a_face()).unwrap(),
            StageOutcome::Unresolved(_)
        ));
    }

    #[test]
    fn rank_one_requires_one_certified_generator_and_one_authoritative_absence() {
        let evidence = AmbientPeriodEvidence {
            u: absent(ParameterAxis::U),
            v: certified(ParameterAxis::V),
        };
        match resolve_ambient_periods(evidence, &a_test_envelope(), a_face()).unwrap() {
            StageOutcome::Resolved(CertifiedAmbientLattice::Rank1(rank1)) => {
                assert_eq!(rank1.periodic_axis(), ParameterAxis::V);
                assert_eq!(rank1.generator().axis(), ParameterAxis::V);
                assert_eq!(rank1.other_axis_absent().get().axis, ParameterAxis::U);
            }
            other => panic!("expected Rank1, got {}", other.tag()),
        }
        // A certified axis beside an *uncertified value* is not rank 1: the
        // other axis may yet be periodic, so the rank is not determined.
        let evidence = AmbientPeriodEvidence {
            u: declared_but_uncertified(ParameterAxis::U, 3.0),
            v: certified(ParameterAxis::V),
        };
        assert!(matches!(
            resolve_ambient_periods(evidence, &a_test_envelope(), a_face()).unwrap(),
            StageOutcome::Unresolved(_)
        ));
    }

    #[test]
    fn rank_two_requires_two_certified_generators() {
        let evidence = AmbientPeriodEvidence {
            u: certified(ParameterAxis::U),
            v: certified(ParameterAxis::V),
        };
        match resolve_ambient_periods(evidence, &a_test_envelope(), a_face()).unwrap() {
            StageOutcome::Resolved(CertifiedAmbientLattice::Rank2(rank2)) => {
                assert_eq!(rank2.first().axis(), ParameterAxis::U);
                assert_eq!(rank2.second().axis(), ParameterAxis::V);
            }
            other => panic!("expected Rank2, got {}", other.tag()),
        }
        // One generator and one uncertified value is not rank 2.
        let evidence = AmbientPeriodEvidence {
            u: certified(ParameterAxis::U),
            v: declared_but_uncertified(ParameterAxis::V, 6.28),
        };
        assert!(matches!(
            resolve_ambient_periods(evidence, &a_test_envelope(), a_face()).unwrap(),
            StageOutcome::Unresolved(_)
        ));
    }

    #[test]
    fn rank_two_requires_generator_independence() {
        let u_generator = certify_revolution_period(ParameterAxis::U).unwrap();
        // Two generators on the same axis span a rank-1 group. The certificate
        // constructor refuses, and the refusal is a *witnessed* contradiction.
        let failure =
            GeneratorIndependenceCertificate::from_distinct_axes(&u_generator, &u_generator)
                .expect_err("same-axis generators are dependent");
        let IndependenceFailure::SharedAxis(witness) = failure else {
            panic!("expected a shared-axis witness");
        };
        assert_eq!(
            witness,
            PeriodContradictionWitness::GeneratorsShareAnAxis {
                axis: ParameterAxis::U
            }
        );
        assert_eq!(
            witness.inconsistency(),
            Inconsistency::PeriodGeneratorDependenceContradiction
        );
        // And it is certified for distinct axes, with the rule and its premise
        // recorded.
        let v_generator = certify_revolution_period(ParameterAxis::V).unwrap();
        let certificate =
            GeneratorIndependenceCertificate::from_distinct_axes(&u_generator, &v_generator)
                .expect("distinct axes are independent under the axis-aligned schema");
        match certificate.certificate() {
            EvidenceCertificate::Analytic(analytic) => {
                assert_eq!(
                    analytic.rule(),
                    AnalyticRule::AxisAlignedGeneratorsAreIndependent
                );
                assert_eq!(
                    *analytic.premises().first(),
                    AnalyticPremise::RepresentedBasisIsAxisAligned
                );
            }
            other => panic!("expected an analytic certificate: {other:?}"),
        }
    }

    #[test]
    fn declared_rank_two_certified_rank_zero_is_unresolved_not_rank_zero() {
        // The 1,357-face population, reproduced exactly: both axes declare a
        // period, the legacy type certifies neither.
        let legacy = CertifiedLattice::from_unevidenced_accessors(Some(6.28), Some(2.0));
        assert_eq!(legacy.certified_rank(), 0, "the legacy census reading");
        assert_eq!(
            usize::from(legacy.declared_u_period().is_some())
                + usize::from(legacy.declared_v_period().is_some()),
            2,
            "declared rank two"
        );

        let evidence =
            ambient_evidence_from_legacy(&legacy, LatticeOrigin::UnattributedLegacyLattice)
                .expect("adapter succeeds");
        assert_eq!(evidence.u.tag(), "declared_but_uncertified");
        assert_eq!(evidence.v.tag(), "declared_but_uncertified");

        match resolve_ambient_periods(evidence, &a_test_envelope(), a_face()).unwrap() {
            StageOutcome::Unresolved(report) => {
                assert_eq!(
                    report.reason(),
                    UnresolvedReason::DeclaredPeriodNotCertified
                );
            }
            other => panic!(
                "certified_rank()==0 must not become formal Rank0; got {}",
                other.tag()
            ),
        }
    }

    #[test]
    fn declared_rank_one_certified_rank_zero_is_unresolved_not_rank_zero() {
        // The 291-face population: one axis declares, the other is silent.
        // Note that *both* axes are unresolved here — the silent one because
        // nothing established its absence either.
        let legacy = CertifiedLattice::from_unevidenced_accessors(Some(6.28), None);
        assert_eq!(legacy.certified_rank(), 0);

        let evidence =
            ambient_evidence_from_legacy(&legacy, LatticeOrigin::UnattributedLegacyLattice)
                .expect("adapter succeeds");
        assert_eq!(evidence.u.tag(), "declared_but_uncertified");
        assert_eq!(
            evidence.v.tag(),
            "undetermined",
            "an accessor returning None proves nothing"
        );

        match resolve_ambient_periods(evidence, &a_test_envelope(), a_face()).unwrap() {
            StageOutcome::Unresolved(report) => {
                assert_eq!(
                    report.reason(),
                    UnresolvedReason::DeclaredPeriodNotCertified
                );
            }
            other => panic!("expected Unresolved, got {}", other.tag()),
        }
    }

    #[test]
    fn rank_above_envelope_is_unsupported() {
        // A policy admitting rank 1 at most. A genuine rank-2 face is then
        // proved outside the envelope — an `Unsupported`, because the rank was
        // established before the bound was applied.
        let envelope = FormalEnvelope::new(
            PolicyInstanceId::new(2),
            1,
            4,
            64,
            4096,
            16,
            64,
            32,
            1 << 20,
        )
        .unwrap();
        let evidence = AmbientPeriodEvidence {
            u: certified(ParameterAxis::U),
            v: certified(ParameterAxis::V),
        };
        match resolve_ambient_periods(evidence, &envelope, a_face()).unwrap() {
            StageOutcome::Unsupported(report) => {
                let UnsupportedCause::EnvelopeExceeded(violation) = report.cause() else {
                    panic!("expected a numeric envelope cause");
                };
                assert_eq!(violation.maximum(), 1);
                assert_eq!(violation.observation().value(), 2);
                assert_eq!(
                    violation.observation().subject(),
                    MeasurementSubject::LatticeRank
                );
            }
            other => panic!("expected Unsupported, got {}", other.tag()),
        }
    }

    // -- 19.4 Deck-vector structure ----------------------------------------

    #[test]
    fn rank_zero_has_only_the_zero_deck_vector() {
        let evidence = AmbientPeriodEvidence {
            u: absent(ParameterAxis::U),
            v: absent(ParameterAxis::V),
        };
        let StageOutcome::Resolved(lattice) =
            resolve_ambient_periods(evidence, &a_test_envelope(), a_face()).unwrap()
        else {
            panic!("expected Rank0");
        };
        // `DeckVector0` is a unit struct: there is one value and no
        // constructor taking components, so the group is `{0}` by type.
        let displacement = DeckDisplacement::Rank0(DeckVector0);
        assert_eq!(displacement.checked_norm().unwrap(), 0);
        let translation = lattice.deck_displacement(&displacement).unwrap();
        assert!(translation.du().is_zero() && translation.dv().is_zero());
    }

    #[test]
    fn rank_one_vector_addition_is_checked() {
        let a = DeckVector1::new(3);
        let b = DeckVector1::new(-5);
        assert_eq!(a.checked_add(b).unwrap().get(), -2);
        assert_eq!(a.checked_sub(b).unwrap().get(), 8);
        assert_eq!(DeckVector1::new(-7).checked_norm().unwrap(), 7);
    }

    #[test]
    fn rank_two_vector_addition_is_checked() {
        let a = DeckVector2::new(3, -4);
        let b = DeckVector2::new(-5, 6);
        assert_eq!(a.checked_add(b).unwrap(), DeckVector2::new(-2, 2));
        assert_eq!(a.checked_sub(b).unwrap(), DeckVector2::new(8, -10));
        assert_eq!(a.checked_norm().unwrap(), 4);
    }

    #[test]
    fn deck_vector_overflow_is_operational_failure() {
        // Not a saturation, not a wrap, and not a semantic judgment.
        assert_eq!(
            DeckVector1::new(i64::MAX).checked_add(DeckVector1::new(1)),
            Err(OperationalFailure::ArithmeticOverflow {
                operation: ResourceOperation::DeckVectorAddition,
            })
        );
        assert_eq!(
            DeckVector1::new(i64::MIN).checked_sub(DeckVector1::new(1)),
            Err(OperationalFailure::ArithmeticOverflow {
                operation: ResourceOperation::DeckVectorSubtraction,
            })
        );
        assert_eq!(
            DeckVector1::new(i64::MIN).checked_norm(),
            Err(OperationalFailure::ArithmeticOverflow {
                operation: ResourceOperation::DeckDisplacementNorm,
            }),
            "|i64::MIN| does not fit in i64"
        );
        assert_eq!(
            DeckVector2::new(i64::MAX, 0).checked_add(DeckVector2::new(1, 0)),
            Err(OperationalFailure::ArithmeticOverflow {
                operation: ResourceOperation::DeckVectorAddition,
            })
        );
    }

    #[test]
    fn a_rank_mismatched_displacement_is_an_internal_invariant_violation() {
        let evidence = AmbientPeriodEvidence {
            u: absent(ParameterAxis::U),
            v: certified(ParameterAxis::V),
        };
        let StageOutcome::Resolved(lattice) =
            resolve_ambient_periods(evidence, &a_test_envelope(), a_face()).unwrap()
        else {
            panic!("expected Rank1");
        };
        assert_eq!(
            lattice.deck_displacement(&DeckDisplacement::Rank2(DeckVector2::new(1, 1))),
            Err(OperationalFailure::InternalInvariantViolation {
                stage: SemanticStage::AmbientPeriodResolution,
                invariant: InvariantId::DeckVectorRankMatchesLattice,
            })
        );
    }

    // -- 19.5 Period-use permissions ---------------------------------------

    #[test]
    fn declared_period_can_be_exposed_as_diagnostic_hint() {
        let evidence = AmbientPeriodEvidence {
            u: declared_but_uncertified(ParameterAxis::U, 1.5),
            v: undetermined(ParameterAxis::V),
        };
        let hints = evidence.diagnostic_hints();
        assert_eq!(hints.len(), 1);
        let hint = hints.iter().next().unwrap();
        assert_eq!(hint.axis, ParameterAxis::U);
        assert_eq!(hint.value.get(), 1.5);
    }

    #[test]
    fn declared_period_cannot_authorize_deck_displacement() {
        // The value exists and is readable as a hint. Turning it into a
        // displacement requires a `CertifiedPeriodGenerator`, and resolution
        // refuses to produce a lattice at all — so there is no
        // `CertifiedAmbientLattice` on which to call `deck_displacement`.
        let evidence = AmbientPeriodEvidence {
            u: declared_but_uncertified(ParameterAxis::U, 1.5),
            v: undetermined(ParameterAxis::V),
        };
        assert_eq!(evidence.diagnostic_hints().len(), 1);
        let outcome = resolve_ambient_periods(evidence, &a_test_envelope(), a_face()).unwrap();
        assert!(matches!(outcome, StageOutcome::Unresolved(_)));
        assert!(outcome.resolved().is_none());
    }

    #[test]
    fn declared_period_cannot_authorize_quotient_identification() {
        let evidence = AmbientPeriodEvidence {
            u: declared_but_uncertified(ParameterAxis::U, 1.5),
            v: declared_but_uncertified(ParameterAxis::V, 2.5),
        };
        // `quotient_identification_authority` is a method on
        // `CertifiedAmbientLattice` alone. This evidence yields none.
        match resolve_ambient_periods(evidence, &a_test_envelope(), a_face()).unwrap() {
            StageOutcome::Unresolved(report) => {
                assert_eq!(
                    report.reason(),
                    UnresolvedReason::DeclaredPeriodNotCertified
                );
            }
            other => panic!("expected Unresolved, got {}", other.tag()),
        }
    }

    #[test]
    fn declared_period_cannot_authorize_cover_enumeration() {
        let evidence = AmbientPeriodEvidence {
            u: declared_but_uncertified(ParameterAxis::U, 1.5),
            v: absent(ParameterAxis::V),
        };
        let outcome = resolve_ambient_periods(evidence, &a_test_envelope(), a_face()).unwrap();
        assert!(
            outcome.resolved().is_none(),
            "no lattice, so no `cover_enumeration_authority` exists to call"
        );
    }

    #[test]
    fn certified_lattice_can_authorize_deck_operations() {
        let evidence = AmbientPeriodEvidence {
            u: absent(ParameterAxis::U),
            v: certified(ParameterAxis::V),
        };
        let StageOutcome::Resolved(lattice) =
            resolve_ambient_periods(evidence, &a_test_envelope(), a_face()).unwrap()
        else {
            panic!("expected Rank1");
        };
        assert_eq!(
            lattice.quotient_identification_authority().lattice().rank(),
            1
        );
        assert_eq!(lattice.cover_enumeration_authority().lattice().rank(), 1);
        assert!(matches!(
            lattice.authoritative_basis(),
            CertifiedPeriodBasisRef::Rank1(_)
        ));

        // Two copies along the certified generator: 2 · 2π.
        let translation = lattice
            .deck_displacement(&DeckDisplacement::Rank1(DeckVector1::new(2)))
            .unwrap();
        assert!(translation.du().is_zero());
        assert_eq!(translation.dv().get(), 4.0 * std::f64::consts::PI);
    }

    // -- introduction rules -------------------------------------------------

    #[test]
    fn an_absence_rule_carries_its_premise() {
        let plane = certify_plane_aperiodicity(ParameterAxis::U, &a_test_plane()).unwrap();
        match plane.certificate() {
            EvidenceCertificate::Analytic(certificate) => {
                assert_eq!(
                    certificate.rule(),
                    AnalyticRule::PlaneHasNoPeriodicDirection
                );
                assert_eq!(
                    *certificate.premises().first(),
                    AnalyticPremise::SupportSurfaceIsAPlane
                );
            }
            other => panic!("expected analytic: {other:?}"),
        }
        // The generatrix rule needs both of its premises, which the
        // introduction rule supplies and `AnalyticCertificate::new` checks.
        let generatrix = certify_straight_generatrix_aperiodicity(ParameterAxis::U).unwrap();
        match generatrix.certificate() {
            EvidenceCertificate::Analytic(certificate) => {
                assert_eq!(certificate.premises().len(), 2);
            }
            other => panic!("expected analytic: {other:?}"),
        }
        assert_eq!(
            plane.use_site().predicate,
            PredicateDescription::of(FormalPredicate::AmbientAxisIsAperiodic(ParameterAxis::U))
        );
    }

    #[test]
    fn a_generator_must_lie_on_its_claimed_axis() {
        // Reachable only from inside `formal`; the point is that the schema
        // check is what makes independence-by-axis sound, so it is enforced
        // rather than assumed.
        let skew = CertifiedUvTranslation::new(1.0, 1.0).unwrap();
        let certificate = PeriodCertificate::new(EvidenceCertificate::Analytic(
            AnalyticCertificate::new(
                AnalyticRule::RevolutionAngularPeriodIsTwoPi,
                NonEmptyVec::one(AnalyticPremise::SupportSurfaceIsARevolvedCurve),
            )
            .unwrap(),
        ))
        .unwrap();
        assert_eq!(
            CertifiedPeriodGenerator::new(ParameterAxis::U, skew, certificate),
            Err(
                GeneratorConstructionError::TranslationDoesNotLieOnClaimedAxis {
                    axis: ParameterAxis::U,
                    off_axis: 1.0,
                }
            )
        );
    }

    // -- adapter ------------------------------------------------------------

    #[test]
    fn the_adapter_never_infers_absence_from_a_missing_accessor() {
        // `NON_PERIODIC` and `from_unevidenced_accessors(None, None)` are the
        // same value once constructed. The adapter therefore cannot tell a
        // plane from a torus whose accessor said nothing, and reports both as
        // undetermined rather than picking one.
        let from_const = CertifiedLattice::NON_PERIODIC;
        let from_accessors = CertifiedLattice::from_unevidenced_accessors(None, None);
        assert_eq!(
            from_const, from_accessors,
            "the legacy type has already erased the distinction"
        );
        for legacy in [from_const, from_accessors] {
            let evidence =
                ambient_evidence_from_legacy(&legacy, LatticeOrigin::UnattributedLegacyLattice)
                    .unwrap();
            assert_eq!(evidence.u.tag(), "undetermined");
            assert_eq!(evidence.v.tag(), "undetermined");
        }
    }

    #[test]
    fn the_adapter_reconstructs_a_revolution_generator() {
        let legacy = CertifiedLattice::revolution(Axis::V, AxisPeriodStatus::NonPeriodic);
        let evidence =
            ambient_evidence_from_legacy(&legacy, LatticeOrigin::UnattributedLegacyLattice)
                .unwrap();
        assert_eq!(evidence.v.tag(), "certified");
        assert_eq!(
            evidence.u.tag(),
            "undetermined",
            "the generatrix axis arrives as NonPeriodic, whose origin is erased"
        );
        let generator = evidence.v.certified_generator().expect("certified");
        assert_eq!(generator.axis(), ParameterAxis::V);
        assert_eq!(
            generator.certificate().basis(),
            AuthoritativeBasis::Analytic
        );
        assert_eq!(evidence.authoritative_generator_count(), 1);

        // Even a certified axis does not make the *face* rank 1: the `u` axis
        // is undetermined, so the rank is not established. This is the
        // measurable difference from the legacy `certified_rank() == 1`.
        assert!(matches!(
            resolve_ambient_periods(evidence, &a_test_envelope(), a_face()).unwrap(),
            StageOutcome::Unresolved(_)
        ));
    }

    #[test]
    fn the_adapter_records_provenance_for_every_axis() {
        let legacy = CertifiedLattice::from_unevidenced_accessors(Some(6.28), None);
        let evidence =
            ambient_evidence_from_legacy(&legacy, LatticeOrigin::UnattributedLegacyLattice)
                .unwrap();
        let provenance = evidence.provenance();
        assert_eq!(provenance.len(), 2, "one link per axis");
        for record in provenance.iter() {
            assert_eq!(record.tag(), "legacy_lattice_axis");
        }
    }

    // -- the structural support-surface bridge ------------------------------
    //
    // Step 1's census resolved 0 of 24,199 faces because the legacy lattice
    // had already erased which producer said `NonPeriodic`. These tests fix
    // the shape of the replacement: authority comes from the *representation*,
    // and every route that does not start there still resolves nothing.

    #[test]
    fn a_structurally_identified_plane_resolves_rank_zero() {
        let evidence = ambient_evidence_from_plane_schema(&a_test_plane())
            .expect("the plane rule supplies its own premise");
        let outcome = resolve_ambient_periods(evidence, &a_test_envelope(), a_face())
            .expect("no operational failure");
        match outcome {
            StageOutcome::Resolved(CertifiedAmbientLattice::Rank0(_)) => {}
            other => panic!("expected a certified rank 0 lattice, got {other:?}"),
        }
    }

    #[test]
    fn both_planar_axes_carry_analytic_absence_evidence() {
        let evidence = ambient_evidence_from_plane_schema(&a_test_plane()).unwrap();
        for (axis, side) in [
            (ParameterAxis::U, &evidence.u),
            (ParameterAxis::V, &evidence.v),
        ] {
            match side {
                PeriodAxisEvidence::AuthoritativelyAbsent {
                    axis: got,
                    evidence,
                } => {
                    assert_eq!(*got, axis);
                    assert_eq!(evidence.basis(), AuthoritativeBasis::Analytic);
                    match evidence.certificate() {
                        EvidenceCertificate::Analytic(certificate) => assert_eq!(
                            certificate.rule(),
                            AnalyticRule::PlaneHasNoPeriodicDirection
                        ),
                        other => panic!("expected an analytic certificate, got {other:?}"),
                    }
                }
                other => panic!("expected authoritative absence on {axis:?}, got {other:?}"),
            }
        }
    }

    /// The measured corpus case the bridge must not break. `look::lattice_of`
    /// routes a torus through `from_unevidenced_accessors`, and a torus is
    /// doubly periodic whatever those accessors return — so `None` on both
    /// axes must not become absence.
    #[test]
    fn a_torus_with_absent_accessor_periods_does_not_resolve_rank_zero() {
        let torus = CertifiedLattice::from_unevidenced_accessors(None, None);
        let schema = SupportSurfaceSchema::not_structurally_identified(
            super::super::support::SchemaIdentificationFailure::NoStructuralReader {
                representation: "toroidal_surface",
            },
        );
        let evidence =
            ambient_evidence_from_schema(&schema, &torus, LatticeOrigin::UnattributedLegacyLattice)
                .expect("the legacy adapter accepts an all-`None` lattice");
        let outcome = resolve_ambient_periods(evidence, &a_test_envelope(), a_face()).unwrap();
        match outcome {
            StageOutcome::Unresolved(report) => {
                assert_eq!(
                    report.reason(),
                    UnresolvedReason::PeriodAbsenceNotEstablished
                );
            }
            other => panic!("a torus must not resolve, got {other:?}"),
        }
    }

    /// The forbidden inference, stated as a test: `NON_PERIODIC` on a surface
    /// whose schema was never read resolves nothing. Only presenting the plane
    /// changes the answer.
    #[test]
    fn an_unknown_schema_with_legacy_non_periodic_stays_unresolved() {
        let schema = SupportSurfaceSchema::not_structurally_identified(
            super::super::support::SchemaIdentificationFailure::NoStructuralReader {
                representation: "b_spline_surface",
            },
        );
        let evidence = ambient_evidence_from_schema(
            &schema,
            &CertifiedLattice::NON_PERIODIC,
            LatticeOrigin::UnattributedLegacyLattice,
        )
        .unwrap();
        let outcome = resolve_ambient_periods(evidence, &a_test_envelope(), a_face()).unwrap();
        match outcome {
            StageOutcome::Unresolved(report) => {
                assert_eq!(
                    report.reason(),
                    UnresolvedReason::PeriodAbsenceNotEstablished
                );
            }
            other => panic!("legacy NON_PERIODIC must not resolve, got {other:?}"),
        }
    }

    /// The same lattice value, the same origin, one difference: the schema.
    /// This is the whole content of the bridge.
    #[test]
    fn only_the_schema_distinguishes_the_two_producers_of_non_periodic() {
        let unknown = SupportSurfaceSchema::not_structurally_identified(
            super::super::support::SchemaIdentificationFailure::NoStructuralReader {
                representation: "b_spline_surface",
            },
        );
        let plane = SupportSurfaceSchema::Plane(a_test_plane());
        let resolve = |schema: &SupportSurfaceSchema| {
            let evidence = ambient_evidence_from_schema(
                schema,
                &CertifiedLattice::NON_PERIODIC,
                LatticeOrigin::UnattributedLegacyLattice,
            )
            .unwrap();
            resolve_ambient_periods(evidence, &a_test_envelope(), a_face()).unwrap()
        };
        assert!(matches!(resolve(&unknown), StageOutcome::Unresolved(_)));
        assert!(matches!(
            resolve(&plane),
            StageOutcome::Resolved(CertifiedAmbientLattice::Rank0(_))
        ));
    }

    /// A degenerate plane never becomes a `PlaneSchema`, so the rank-0 route is
    /// closed to it at the type level — there is no value to call the bridge
    /// with. Restated here because it is an ambient-authority fact, not only a
    /// schema-reader fact.
    #[test]
    fn a_degenerate_plane_cannot_reach_the_rank_zero_route() {
        let degenerate = truck_geometry::prelude::Plane::new(
            truck_geometry::prelude::Point3::new(0.0, 0.0, 0.0),
            truck_geometry::prelude::Point3::new(1.0, 0.0, 0.0),
            truck_geometry::prelude::Point3::new(2.0, 0.0, 0.0),
        );
        let schema = super::super::support::identify_plane(&degenerate);
        assert!(schema.plane().is_none());
        let evidence = ambient_evidence_from_schema(
            &schema,
            &CertifiedLattice::NON_PERIODIC,
            LatticeOrigin::UnattributedLegacyLattice,
        )
        .unwrap();
        assert!(matches!(
            resolve_ambient_periods(evidence, &a_test_envelope(), a_face()).unwrap(),
            StageOutcome::Unresolved(_)
        ));
    }
}
