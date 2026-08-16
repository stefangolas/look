//! The formal path, built beside the legacy tessellator.
//!
//! The entire subtree is staged prototype work (see the per-stage notes below):
//! each module lands ahead of the production caller that will consume it, so its
//! public surface is intentionally unused until the corresponding priority in
//! `HANDOFF.md` wires it in. Mirror `triangulation.rs`'s top-of-file allow
//! rather than denying the build on work that is deliberately not yet reached.
#![allow(dead_code, unused)]
//!
//! `MATHEMATICAL_FOUNDATION.md` §0 states the design rule this subtree exists
//! to obey:
//!
//! > Every pipeline stage is a fallible constructor. Its output type
//! > represents a stronger state and carries evidence for the obligations that
//! > were discharged.
//!
//! **Steps 1 through 8, on a declared subset.** The ambient-period authority
//! model, the judgment algebra and the bounded evaluation policy are general;
//! [`planar_slice`] carries Steps 2 to 8 only as far as one *planar,
//! line-bounded, hole-free* face needs, and refuses everything else with a
//! named reason. `FORMAL_SYSTEM.md` §XVIII lists the stages in full; each
//! expansion arrives with the measured obstruction population it targets.
//!
//! **Production geometry is unchanged unless the recovery gate is opened.**
//! With `TRUCK_FORMAL_RECOVERY` unset the formal chain either does not run or
//! runs in shadow and its result is dropped, so the measured baseline —
//! 1,358,543 triangles, 4,486 faces lost — holds by construction. With the gate
//! open, a *validated* formal mesh may replace a face the legacy path **lost**,
//! and nothing else; on `00009190` that is 2 faces and 2 triangles.
//!
//! # The layout
//!
//! - [`numeric`] — checked finite wrappers. No formal certificate holds a raw
//!   `f64` bound.
//! - [`evidence`] — `Fact(P, κ)` of Definition 2, as a sum type whose
//!   `Unresolved` state carries no value, with proposition-specific
//!   admissibility policies rather than a confidence ranking.
//! - [`outcome`] — the four result layers: stage result, final semantic
//!   outcome, realization outcome, operational failure.
//! - [`envelope`] — the bounds `β` of Definition 6 and the separate execution
//!   budget.
//! - [`ambient`] — `Λ` of Definition 7: the five distinguishable states of an
//!   axis's period evidence, and the rank-structured lattice they can resolve
//!   to.
//! - [`deck`] — certified integer deck arithmetic for the rank-1
//!   contractible-disk slice: the four-way deck solver and the outward-rounded
//!   cover intervals. Self-contained: no STEP parsing.
//! - [`support`] — structural identification of the support surface and of the
//!   edge curves, read from the representation *before* the legacy types erase
//!   it. The only route by which the premise `SupportSurfaceIsAPlane` enters.
//! - [`planar_slice`] — Steps 2 to 8 for the admitted planar subset, with one
//!   [`SliceExit`] per obstruction the corpus can present.
//!
//! # What the subtree will not let you do
//!
//! Authority is minted only by named introduction rules. `Evidence`'s three
//! authority-bearing constructors, `PeriodCertificate::new`,
//! `CertifiedPeriodGenerator::new`, `ExactCount::from_completed_count`,
//! `CertifiedLowerBoundCount::from_certificate` and
//! `FeatureExclusionWitness::new` are all `pub(super)` and are *not*
//! re-exported below. Outside this subtree the only way to obtain a certified
//! ambient generator is [`ambient::certify_revolution_period`] or
//! [`ambient::certify_period_numerically`], and the only way to obtain a
//! certified lattice is [`ambient::resolve_ambient_periods`].

pub mod ambient;
pub mod cone;
pub mod cone_band;
pub mod curve2d;
pub mod curve_witness;
pub mod cylinder;
pub mod cylinder_arrangement;
pub mod cylinder_band;
pub mod cylinder_cover;
pub mod cylinder_face;
pub mod cylinder_lift;
pub mod cylinder_mesh;
pub mod torus;
pub mod torus_cell;
pub mod torus_circle;
pub mod torus_outcome;
pub mod torus_realize;
// GEN-001 generic arrangement substrate.
pub mod bezier;

pub mod bezier_isect;
pub mod common_arc;
pub mod contact;
pub mod deck;
pub mod envelope;
pub mod evidence;
pub mod exact;
pub mod intersection;
pub mod numeric;
pub mod outcome;
pub mod planar_developed;
pub mod planar_holes;
pub mod planar_slice;
pub mod quotient;
pub mod rank1_annulus;
pub mod span;
pub mod support;
pub mod xmonotone;

// The Step 1 surface, re-exported for the probe and for later stages. Types
// only: every proof-introduction rule stays behind its module's visibility.
pub use ambient::{
    ambient_axis_evidence_from_legacy, ambient_evidence_from_legacy,
    ambient_evidence_from_plane_schema, ambient_evidence_from_schema, certify_period_numerically,
    certify_plane_aperiodicity, certify_revolution_period,
    certify_straight_generatrix_aperiodicity, resolve_ambient_periods, AdapterError,
    AmbientAlternative, AmbientEvidenceError, AmbientPeriodEvidence, CertifiedAmbientLattice,
    CertifiedPeriodBasisRef, CertifiedPeriodGenerator, CertifiedRank0, CertifiedRank1,
    CertifiedRank2, CertifiedUvTranslation, CoverEnumerationAuthority, DeckDisplacement,
    DeckVector0, DeckVector1, DeckVector2, DeclaredPeriod, DeclaredPeriodHint,
    GeneratorIndependenceCertificate, LatticeOrigin, ObservedPeriod, PeriodAbsence,
    PeriodAxisEvidence, PeriodCertificate, PeriodCertificationAttempt, PeriodCertificationFailure,
    PeriodContradictionWitness, PeriodHintSet, PeriodHintSource, QuotientIdentificationAuthority,
    UncertifiedPeriodValue,
};
pub use bezier::{BezierSpanError, RationalBezierSpan2};
pub use bezier_isect::intersect_bezier_pair;
pub use common_arc::{
    common_arc_for_pair, AnalyticSupportClass, AnalyticSupportCorrespondenceCertificate,
    ArcParticipant, AuthoritativeParameterKey, AxisSide, CanonicalSourceAxis, CertifiedAffineMap,
    CertifiedIntervalOverlap, CertifiedParameterCorrespondence, CommonArc2, CommonArcBoundaries,
    CommonArcBoundary, CommonArcBoundaryKey, CommonArcCertificate, CommonArcEnd, CommonArcError,
    CommonArcIdentity, CommonSupportBasis, CommonSupportFragment, CommonSupportIdentity,
    OrientationAlongSupport, SharedParentCertificate, SupportIdentityCertificate,
};
pub use cone::{
    identify_cone, CertifiedEmbeddedCone, ConeIdentification, ConeIdentificationFailure,
    ConeSchema, ConeValidityCertificate, Nappe, MINIMUM_CONE_AXIAL_COMPONENT,
    MINIMUM_CONE_GENERATRIX_TILT,
};
pub use cone_band::{
    carrier_of as cone_carrier_of, certify_conical_essential_band, classify_cone_carriers,
    classify_nappes, complete_cone_circle_witness, conical_band_material_authority,
    develop_complete_cone_parallel, run_conical_essential_band, CertifiedConicalBandMesh,
    CertifiedConicalEssentialBand, ConeCarrierRelation, ConeCircleCarrier, ConeParallelWitness,
    ConeWitnessFailure, ConicalBandExit, ConicalBandMaterialAuthority, ConicalSourceStanding,
    NappeRelation,
};
pub use contact::{
    label_branch_from_placement, lift_pair_result, AggregatedQuotientEventKey, BranchIncidence,
    ContactComponent2, CrossingClassification, EventIdentity, GenericUnresolved, IsolatedEvent2,
    IsolatedRootKey, IsolatedRootParticipant, PairContactLiftKey, PairContactResult,
};
pub use curve2d::{
    CurveOccurrenceProvenance, DevelopedCurve2D, DirectedCircularArc2, LineSegment2, SourceEdgeId,
    SourceEntityId, SourceFaceId,
};
pub use curve_witness::{
    axial_line_witness, circumferential_arc_witness, complete_circle_witness,
    identify_source_curve_witness, CompleteCirclePlacement, CurveOnCylinderWitness,
    SourceCurveFamily, WitnessClass, WitnessFailure,
};
pub use cylinder::{
    identify_cylinder, CertifiedEmbeddedCylinder, CylinderIdentification,
    CylinderIdentificationFailure, CylinderSchema, CylinderValidityCertificate, PeriodicParameter,
    MINIMUM_CYLINDER_LINE_AXIS_PARALLELISM,
};
pub use cylinder_arrangement::{
    certify_cylinder_disk, placed_occurrences, CertifiedCylinderDisk, CylinderArrangementExit,
};
pub use cylinder_cover::{build_working_cover, CylinderCoverExit, WorkingCoverResult};
pub use cylinder_face::{build_cylinder_face, CylinderFaceRecord};
pub use cylinder_lift::{
    develop_traversal, develop_traversal_from_source, propagate_and_classify_holonomy,
    CylinderLiftExit, DevelopedCylinderBoundary, WitnessSpec, ZeroHolonomyLift,
};
pub use cylinder_mesh::{
    certify_cylinder_mesh, certify_cylinder_polygon, lift_to_cylinder, CertifiedCylinderMesh,
};
pub use deck::{
    deck_cover_interval, solve_axis_aligned, CertifiedDeckInterval, DeckBudget,
    DeckConstructorFailure, DeckGenerator, DeckInterval, DeckNumericFailure,
    DeckOperationalFailure, DeckSolveResult, DevelopedAxis, DevelopedBox,
};
pub use envelope::{
    BoundObservation, CertifiedLowerBoundCount, ExactCount, ExecutionBudget, FeatureExclusion,
    FeatureExclusionWitness, FormalEnvelope, NumericEnvelopeClause, PolicyInstanceId,
};
pub use evidence::{
    AnalyticCertificate, AnalyticPremise, AnalyticRule, AtLeastTwo, AuthoritativeBasis,
    AuthoritativeFact, Evidence, EvidenceCertificate, EvidenceRequirement, EvidenceStatus,
    FormalPredicate, NonEmptyVec, NumericalCertificate, ParameterAxis, PredicateDescription,
    SemanticStage, SourceEntityKey, ANALYTIC_ONLY, ANALYTIC_OR_CERTIFIED_NUMERICAL,
    ANY_AUTHORITATIVE, SOURCE_DECLARATION,
};
pub use exact::{cross_exp, exact_dot2, CertifiedInterval, CertifiedSign, Expansion};
pub use intersection::{
    intersect_x_monotone, CertifiedIntersection2, ContactKind, IntersectionIdentity,
    IntersectionPolicy, LocationOnPiece, NumericalCause, PairIntersectionResult, PairUnresolved,
    PairUnsupported, ParameterEnclosure, ParameterLocation,
};
pub use outcome::{
    Ambiguity, AmbiguityReport, ContradictionWitness, DocumentKey, DocumentScope, FaceKey,
    Inconsistency, InconsistencyReport, OperationalFailure, RealizationOutcome, SemanticOutcome,
    ShellKey, StageEvaluation, StageOutcome, UnresolvedReason, UnresolvedReport, UnsupportedCause,
    UnsupportedReport, ValidSemantic,
};
pub use planar_developed::{
    develop_planar_curves, run_developed_face, run_developed_track, survey_arrangement,
    ArrangementSurvey, DevelopedExit, DevelopedFaceRecord, DevelopedRecord,
};
pub use planar_holes::{
    run_planar_holes_slice, BoundRole, BoundaryComponentMap, BoundaryLoop, ComponentRelation,
    HoleFinalValidityReport, HoleSliceRecord, MultiBoundEntry, PlanarMultiBoundInput,
    PlanarRegionWithHolesCertificate,
};
pub use planar_slice::{
    run_planar_slice, CertificateRoute, FinalValidityReport, PlanarMesh, SliceCategory, SliceExit,
    SliceRecord, SliceStage,
};
pub use quotient::{
    adapt_axis_aligned_placement, certify_rank2_placement, AmbientLatticeId, CanonicalBranchSide,
    CanonicalIncidenceId, CertifiedDeckLabel, DeckContext, DeckLabel, DeckLabelBasis,
    DeckLabelError, DeckPlacementResult, DeckPlacementUnsupported, DeckRank, DeckSignature,
};
pub use rank1_annulus::{
    realize as realize_rank_one_annulus, AnnulusBoundary, AnnulusCell, AnnulusExit,
    AnnulusObligation, AnnulusValidityReport, CarrierOrder, CompleteParallel, FreeDeckAction,
    RankOnePeriodicAnnulus, RealizedAnnulus,
};
pub use span::{BranchGerm, CurveSpan2, FastPath, SpanId};
pub use support::{
    identify_line_segment, identify_plane, identify_polyline, CircularArcPlacement3, CurveSchema,
    CurveSchemaFailure, PlaneGram, PlaneSchema, SchemaIdentificationFailure, SupportSurfaceSchema,
    MINIMUM_NORMALISED_GRAM_DETERMINANT,
};
pub use torus::{
    identify_torus, identify_torus_world, CertifiedEmbeddedTorus, CertifiedRankTwoDeck,
    TorusIdentification, TorusIdentificationFailure, TorusSchema, TorusValidityCertificate,
    MINIMUM_TORUS_PERIOD_RESIDUAL,
};
pub use torus_cell::{
    certify_torus_annular_cell, certify_torus_annular_cell_with_witnesses, BoundaryLoopPlacement,
    CertifiedEssentialLoop, CertifiedMaterialAuthority, CertifiedTorusAnnularCell, ConformanceTag,
    PrimitiveWinding, SourceBoundaryComposition, TorusCellFailure, TwoOuterBoundMalformation,
};
pub use torus_circle::{
    certify_circle_on_torus, lift_circle_winding, normalize_to_canonical, CertifiedWinding,
    CircleFamily, CircleOnTorusStatus, Gl2zNormalization, OnTorusWitness, WindingResult,
};
pub use torus_outcome::TorusAnnulusExit;
pub use torus_realize::{
    develop_torus_annulus, realize_torus_annulus, DevelopedSide, DevelopedTorusAnnulus,
    Gl2zBasisChange, RealizationFailure as TorusRealizationFailure, RealizeFailure,
    RealizedTorusAnnulus,
};
pub use xmonotone::{
    make_x_monotone, ClosedInterval, CriticalIdentity, DecompositionKind,
    MonotoneDecompositionFailure, MonotoneKind, NumericalPolicy, PieceIdentity,
    XMonotoneCircularArc2, XMonotoneLine2, XMonotonePiece2,
};
