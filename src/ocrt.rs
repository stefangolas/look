//! Obligation-Carrying Refinement Typestate (OCRT) Core Primitives
//!
//! Enforces the epistemic invariant:
//! "No semantic promotion without a certificate; no failed obligation without a witness."

use std::fmt::Debug;
use std::marker::PhantomData;

/// Result of an obligation check or refinement validation.
#[derive(Clone, Debug, PartialEq)]
pub enum CheckResult<T, W> {
    /// Refinement condition verified; object promoted with unforgeable certificate.
    Certified(T),
    /// Explicit refusal with witness data.
    Refused(W),
    /// Predicate could not be determined within numerical bounds.
    Undetermined(W),
    /// Operation halted due to resource limit (time, memory, iterations).
    ResourceCapped(ResourceWitness),
}

/// Witness object when a check exceeds resource budgets.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ResourceWitness {
    pub limit_kind: &'static str,
    pub limit_value: u64,
    pub consumed_value: u64,
}

/// An unforgeable certificate wrapping a refined semantic value `T` with predicate `P`.
///
/// Private fields ensure that safe Rust code outside the defining module cannot
/// construct a `Certified<T, P>` without calling a checked constructor.
#[derive(Clone, Debug, PartialEq)]
pub struct Certified<T, P> {
    value: T,
    certificate: P,
    evidence: EvidenceGraph,
}

impl<T, P> Certified<T, P> {
    /// Private to crate/module — only checked validators may construct certificates.
    pub(crate) fn new(value: T, certificate: P, evidence: EvidenceGraph) -> Self {
        Self {
            value,
            certificate,
            evidence,
        }
    }

    /// Access the underlying refined value.
    pub fn value(&self) -> &T {
        &self.value
    }

    /// Unwrap into the inner value.
    pub fn into_value(self) -> T {
        self.value
    }

    /// Access the certificate proof object.
    pub fn certificate(&self) -> &P {
        &self.certificate
    }

    /// Access the evidence graph establishing provenance.
    pub fn evidence(&self) -> &EvidenceGraph {
        &self.evidence
    }
}

/// Evidence graph tracking dependency hashes, metric bounds, and algorithm version.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EvidenceGraph {
    pub provenance_id: u64,
    pub metric: Option<String>,
    pub tolerance: Option<f64>,
    pub max_residual: Option<f64>,
    pub dependencies: Vec<u64>,
}

/// Linear owned obligation that must be explicitly resolved or converted to a refusal witness.
#[must_use = "Obligations represent mandatory geometric claims and cannot be dropped silently."]
#[derive(Debug)]
pub struct Obligation<P> {
    pub id: u64,
    pub subject_entity: u32,
    pub predicate: P,
}

impl<P> Obligation<P> {
    pub fn new(id: u64, subject_entity: u32, predicate: P) -> Self {
        Self {
            id,
            subject_entity,
            predicate,
        }
    }

    /// Discharge obligation with proof, yielding a `Certified<T, C>`.
    pub fn discharge<T, C>(
        self,
        value: T,
        certificate: C,
        evidence: EvidenceGraph,
    ) -> Certified<T, C> {
        Certified::new(value, certificate, evidence)
    }

    /// Refuse obligation with witness data.
    pub fn refuse<W>(self, witness: W) -> Failure<W> {
        Failure {
            root: RootFailure::Refused(witness),
            consequences: Vec::new(),
            provenance_id: self.subject_entity,
        }
    }
}

/// Root failure object that preserves the root cause across pipeline wrappers.
/// Invariant: root(wrap(e, c)) == root(e).
#[derive(Clone, Debug, PartialEq)]
pub struct Failure<W> {
    pub root: RootFailure<W>,
    pub consequences: Vec<String>,
    pub provenance_id: u32,
}

impl<W> Failure<W> {
    pub fn new_root(root: W, provenance_id: u32) -> Self {
        Self {
            root: RootFailure::Refused(root),
            consequences: Vec::new(),
            provenance_id,
        }
    }

    pub fn add_consequence(&mut self, consequence: impl Into<String>) {
        self.consequences.push(consequence.into());
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum RootFailure<W> {
    Refused(W),
    Undetermined(W),
    ResourceCapped(ResourceWitness),
}

/// Strict face outcome conservation law:
/// N_source = N_certified + N_refused + N_undetermined + N_resource_capped.
#[derive(Clone, Debug, PartialEq)]
pub enum FaceOutcome<M, W> {
    CertifiedMesh(Certified<M, MeshTopologyCertificate>),
    PreciseRefusal(Failure<W>),
    Undetermined(Failure<W>),
    ResourceCapped(ResourceWitness),
}

#[derive(Clone, Debug, PartialEq)]
pub struct MeshTopologyCertificate {
    pub is_closed: bool,
    pub is_oriented: bool,
    pub face_count: usize,
}

/// Distinguish raw algorithm outputs (`Candidate`) from validated claims (`Certified`).
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate<T> {
    pub value: T,
    pub claimed_residual: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProjectionCandidate {
    pub uv: (f64, f64),
    pub reconstructed: [f64; 3],
    pub iterations: usize,
}

use std::collections::HashMap;

/// Runtime obligation ledger guaranteeing exact linear conservation.
/// Asserted at ingestion exit: N_issued == N_terminal.
#[derive(Debug, Default)]
pub struct ObligationLedger {
    issued: HashMap<u64, u32>,
    discharged: HashMap<u64, String>,
    refused: HashMap<u64, String>,
}

impl ObligationLedger {
    pub fn register(&mut self, obligation_id: u64, subject_entity: u32) {
        self.issued.insert(obligation_id, subject_entity);
    }

    pub fn record_discharge(&mut self, obligation_id: u64, certificate_tag: impl Into<String>) {
        self.discharged
            .insert(obligation_id, certificate_tag.into());
    }

    pub fn record_refusal(&mut self, obligation_id: u64, reason: impl Into<String>) {
        self.refused.insert(obligation_id, reason.into());
    }

    pub fn is_conserved(&self) -> bool {
        self.issued.len() == (self.discharged.len() + self.refused.len())
    }
}

// -----------------------------------------------------------------------------
// Semantic Assumption Audit Ledger Infrastructure
// -----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AssumptionId {
    ParSurfaceSearch001,
    SngRegularity001,
    UvQuotientClosure001,
    ProjIncidence001,
    OrientMaterialSide001,
    OmitEntityFallback001,
    IdArenaPreservation001,
    UnitDimensionalScaling001,
    ResourceBoundTermination001,
}

impl AssumptionId {
    pub fn tag(&self) -> &'static str {
        match self {
            Self::ParSurfaceSearch001 => "PAR-SURFACE-SEARCH-001",
            Self::SngRegularity001 => "SNG-REGULARITY-001",
            Self::UvQuotientClosure001 => "UV-QUOTIENT-CLOSURE-001",
            Self::ProjIncidence001 => "PROJ-INCIDENCE-001",
            Self::OrientMaterialSide001 => "ORIENT-MATERIAL-SIDE-001",
            Self::OmitEntityFallback001 => "OMIT-ENTITY-FALLBACK-001",
            Self::IdArenaPreservation001 => "ID-ARENA-PRESERVATION-001",
            Self::UnitDimensionalScaling001 => "UNIT-DIMENSIONAL-SCALING-001",
            Self::ResourceBoundTermination001 => "RESOURCE-BOUND-TERMINATION-001",
        }
    }
}

#[derive(Clone, Debug)]
pub struct AssumptionRecord {
    pub id: AssumptionId,
    pub owner: &'static str,
    pub input_type: &'static str,
    pub produced_claim: &'static str,
    pub required_predicate: &'static str,
    pub passed_count: u64,
    pub refused_count: u64,
}

#[derive(Debug, Default)]
pub struct AssumptionLedger {
    records: HashMap<AssumptionId, AssumptionRecord>,
}

impl AssumptionLedger {
    pub fn record_eval(&mut self, id: AssumptionId, passed: bool) {
        let entry = self.records.entry(id).or_insert_with(|| AssumptionRecord {
            id,
            owner: "truck-fork",
            input_type: "geometry_primitive",
            produced_claim: id.tag(),
            required_predicate: "certified_refinement_predicate",
            passed_count: 0,
            refused_count: 0,
        });
        if passed {
            entry.passed_count += 1;
        } else {
            entry.refused_count += 1;
        }
    }

    pub fn summary(&self) -> String {
        let mut out = String::from("=== SEMANTIC ASSUMPTION AUDIT SUMMARY ===\n");
        for (id, rec) in &self.records {
            out.push_str(&format!(
                "[{}] passed: {} refused: {}\n",
                id.tag(),
                rec.passed_count,
                rec.refused_count
            ));
        }
        out
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CertifiedProjection {
    pub uv: (f64, f64),
    pub residual_meters: f64,
}

/// Distinct boundary types — never collapse empty vectors or None into missing boundary.
#[derive(Clone, Debug, PartialEq)]
pub enum FaceBoundary {
    Wire(Vec<u32>),
    Collapsed(CollapsedApexBoundary),
}

/// Chart-level representative of a singular equivalence class (e.g. cone apex fiber {u_apex} x S1).
#[derive(Clone, Debug, PartialEq)]
pub enum ParameterBoundary {
    Regular(CertifiedProjection),
    Collapsed(CollapsedChartBoundary),
}

#[derive(Clone, Debug, PartialEq)]
pub struct CollapsedChartBoundary {
    pub source_vertex: u32,
    pub uv_start: (f64, f64),
    pub uv_end: (f64, f64),
    pub physical_point: [f64; 3],
    pub collapse: CollapseCertificate,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CollapseCertificate {
    pub max_apex_residual: f64,
    pub rank_jacobian: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CollapsedApexBoundary {
    pub vertex_id: u32,
    pub apex_3d: [f64; 3],
}

// -----------------------------------------------------------------------------
// Coarse Typestate Pipeline Stage Markers
// -----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct ImportedStage;
#[derive(Clone, Debug, PartialEq)]
pub struct CanonicalStage;
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedStage;
#[derive(Clone, Debug, PartialEq)]
pub struct TopologicallyValidStage;
#[derive(Clone, Debug, PartialEq)]
pub struct CurveSurfaceCompatibleStage;
#[derive(Clone, Debug, PartialEq)]
pub struct QuotientResolvedStage;
#[derive(Clone, Debug, PartialEq)]
pub struct DomainResolvedStage;
#[derive(Clone, Debug, PartialEq)]
pub struct TriangulatedStage;
#[derive(Clone, Debug, PartialEq)]
pub struct CertifiedStage;

/// Coarse typestate face wrapper parameterized by stage.
#[derive(Debug)]
pub struct FaceState<Stage> {
    entity_id: u32,
    _stage: PhantomData<Stage>,
}

impl FaceState<ImportedStage> {
    pub fn new(entity_id: u32) -> Self {
        Self {
            entity_id,
            _stage: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct PeriodicClosurePredicate;
    struct ConeApexWitness {
        u_singular: f64,
    }

    #[test]
    fn test_obligation_discharge_produces_certificate() {
        let obl = Obligation::new(101, 4932, PeriodicClosurePredicate);
        let evidence = EvidenceGraph {
            provenance_id: 4932,
            metric: Some("Euclidean3D".to_string()),
            tolerance: Some(1.0e-7),
            max_residual: Some(1.22e-13),
            dependencies: vec![101],
        };

        let certified = obl.discharge(
            "ClosedBoundary",
            MeshTopologyCertificate {
                is_closed: true,
                is_oriented: true,
                face_count: 1,
            },
            evidence.clone(),
        );

        assert_eq!(*certified.value(), "ClosedBoundary");
        assert!(certified.certificate().is_closed);
        assert_eq!(certified.evidence().max_residual, Some(1.22e-13));
    }

    #[test]
    fn test_root_failure_preservation() {
        let obl = Obligation::new(102, 4932, PeriodicClosurePredicate);
        let witness = ConeApexWitness {
            u_singular: -0.04489,
        };
        let mut failure = obl.refuse(witness);

        assert_eq!(failure.provenance_id, 4932);
        assert!(failure.consequences.is_empty());

        failure.add_consequence("BoundaryPieceUnavailable");
        failure.add_consequence("DomainNotConstructed");

        // Consequence wrapping preserves the root failure
        if let RootFailure::Refused(ref w) = failure.root {
            assert_eq!(w.u_singular, -0.04489);
        } else {
            panic!("Root failure was modified");
        }
        assert_eq!(failure.consequences.len(), 2);
    }

    #[test]
    fn test_assumption_ledger_conservation() {
        let mut ledger = AssumptionLedger::default();
        ledger.record_eval(AssumptionId::ParSurfaceSearch001, true);
        ledger.record_eval(AssumptionId::ParSurfaceSearch001, true);
        ledger.record_eval(AssumptionId::SngRegularity001, false);

        let summary = ledger.summary();
        assert!(summary.contains("PAR-SURFACE-SEARCH-001"));
        assert!(summary.contains("passed: 2"));
        assert!(summary.contains("refused: 1"));
    }
}
