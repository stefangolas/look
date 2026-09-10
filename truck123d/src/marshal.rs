//! Frozen `Refusal` → typed-exception marshaling (PB-000 §2).
//!
//! Pure Rust: no pyo3, no Python. This module IS the mapping table's
//! executable form. Every landed `Refusal` variant converts to one of the two
//! typed payloads (`Refused` or `Unresolved`) — nothing ever degrades to a
//! bare `Exception` (refusal fidelity, spec §5) — and every payload is a serde
//! value that round-trips losslessly through JSON, which is exactly how the
//! Python exception instances build their dataclass payloads (PB-005 consumes
//! the same serde shape; nothing is ever pickled).
//!
//! H-1 applies: kernel-reachable paths here must not panic, unwrap, expect, or
//! index.

#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::todo,
        clippy::unimplemented,
        clippy::indexing_slicing
    )
)]

use serde::{Deserialize, Serialize};
use serde_json::json;
use truck_base::evidence::{
    Budget, Certificate, EnvelopeCase, Method, Modulus, ModulusShape, Prop, Refusal, Truth,
    UnresolvedWitness,
};
// CG-BINDING: the certified funnel's refusal vocabularies. Reaching them here
// is exactly the one sanctioned manifest edge (recorded in src/binding.rs);
// the construct/kernel refusal kinds are translated into the SAME typed door
// vocabulary the base `Refusal` mapping already produces, never a bare string.
use truck_certified::construct::refusal::ConstructRefusal;
use truck_certified::kernel::evidence::RefusalKind;

/// The two-class Python exception hierarchy (PB-000 §2):
/// `TruckError` → `Refused` | `Unresolved`. This enum is the class tag of a
/// marshaled refusal; there is no third ("bare exception") arm by
/// construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExceptionClass {
    /// `Refused` — a definitive kernel refusal.
    Refused,
    /// `Unresolved` — the kernel could not certify within budget.
    Unresolved,
}

/// The attributes a `Refused` exception instance carries, exactly as the
/// frozen §2 table lists them. Each payload field is optional because the
/// class is shared across refusal variants; the `case` discriminant says which
/// field is live. Absent fields are omitted from JSON (`skip_serializing_if`)
/// and read back as `None`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RefusedPayload {
    /// Which `Refusal` variant this refusal was, snake_case.
    pub case: String,
    /// `EnvelopeCase` name — `UnsupportedEnvelope` only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub envelope: Option<String>,
    /// The stage that exhausted the margin / gave up —
    /// `CompositionMarginExhausted` and `InputOutsideBackwardBudget`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    /// The property whose truth values conflicted — `Contradictory`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prop: Option<String>,
    /// One of the two conflicting truth values — `Contradictory`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub left: Option<String>,
    /// The other conflicting truth value — `Contradictory`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub right: Option<String>,
    /// Why the exact object collapsed — `Collapsed`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// The certificate of the collapse — `Collapsed`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub certificate: Option<serde_json::Value>,
    /// The bound that was computed — `ForwardToleranceExceeded`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bound: Option<f64>,
    /// The largest bound that would have been acceptable —
    /// `ForwardToleranceExceeded`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed: Option<f64>,
}

/// The κ ledger of an `Unresolved` exception: the budget the kernel published
/// when it gave up (`NumericallyUnresolved.spent` — the remaining ledger; the
/// per-call spend `κ = starting − remaining` is only computable by the caller
/// that owns the starting budget, PB-005, so this is the kernel-faithful
/// payload).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ledger {
    /// Subdivision counter.
    pub subdiv: u32,
    /// Newton iteration counter.
    pub newton: u32,
    /// Recursion depth counter.
    pub depth: u32,
}

/// The attributes an `Unresolved` exception instance carries (PB-000 §2):
/// the κ budget ledger plus the `UnresolvedWitness` that says why.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnresolvedPayload {
    /// κ: the kernel's budget ledger at give-up time.
    pub kappa: Ledger,
    /// Why the result could not be certified (the witness, snake_case).
    pub witness: String,
}

/// The marshaled form of one `Refusal`: the typed class tag, a human message,
/// and the payload the exception instance carries. Serde-round-trippable as a
/// whole.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Marshaled {
    /// Which of the two typed classes this raises as.
    pub class: ExceptionClass,
    /// Human-readable message for the exception's `args`.
    pub message: String,
    /// The typed payload.
    pub payload: MarshaledPayload,
}

/// The typed payload union: exactly the two classes of the frozen mapping.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MarshaledPayload {
    /// Payload of the `Refused` class.
    Refused(RefusedPayload),
    /// Payload of the `Unresolved` class.
    Unresolved(UnresolvedPayload),
}

impl Marshaled {
    /// Marshals a landed `Refusal` against the frozen mapping. The match is
    /// exhaustive: adding a `Refusal` variant later breaks this function on
    /// purpose (mirroring `pb_refusal_mapping_covers_landed_refusal_variants`).
    pub fn from_refusal(refusal: &Refusal) -> Marshaled {
        match refusal {
            Refusal::Empty => Marshaled {
                class: ExceptionClass::Refused,
                message: "kernel refusal: empty operation domain".to_string(),
                payload: MarshaledPayload::Refused(RefusedPayload {
                    case: "empty".to_string(),
                    envelope: None,
                    stage: None,
                    prop: None,
                    left: None,
                    right: None,
                    reason: None,
                    certificate: None,
                    bound: None,
                    allowed: None,
                }),
            },
            Refusal::UnsupportedEnvelope(case) => Marshaled {
                class: ExceptionClass::Refused,
                message: format!(
                    "kernel refusal: unsupported envelope: {}",
                    envelope_case_name(*case)
                ),
                payload: MarshaledPayload::Refused(RefusedPayload {
                    case: "unsupported_envelope".to_string(),
                    envelope: Some(envelope_case_name(*case).to_string()),
                    stage: None,
                    prop: None,
                    left: None,
                    right: None,
                    reason: None,
                    certificate: None,
                    bound: None,
                    allowed: None,
                }),
            },
            Refusal::NumericallyUnresolved { spent, witness } => Marshaled {
                class: ExceptionClass::Unresolved,
                message: format!(
                    "kernel unresolved: budget exhausted, witness: {}",
                    witness_name(*witness)
                ),
                payload: MarshaledPayload::Unresolved(UnresolvedPayload {
                    kappa: Ledger {
                        subdiv: spent.subdiv,
                        newton: spent.newton,
                        depth: spent.depth,
                    },
                    witness: witness_name(*witness).to_string(),
                }),
            },
            Refusal::CompositionMarginExhausted(witness) => Marshaled {
                class: ExceptionClass::Refused,
                message: format!(
                    "kernel refusal: composition margin exhausted at stage: {}",
                    witness.stage
                ),
                payload: MarshaledPayload::Refused(RefusedPayload {
                    case: "composition_margin_exhausted".to_string(),
                    envelope: None,
                    stage: Some(witness.stage.to_string()),
                    prop: None,
                    left: None,
                    right: None,
                    reason: None,
                    certificate: None,
                    bound: None,
                    allowed: None,
                }),
            },
            Refusal::InputOutsideBackwardBudget(witness) => Marshaled {
                class: ExceptionClass::Refused,
                message: format!(
                    "kernel refusal: input outside backward budget at stage: {}",
                    witness.stage
                ),
                payload: MarshaledPayload::Refused(RefusedPayload {
                    case: "input_outside_backward_budget".to_string(),
                    envelope: None,
                    stage: Some(witness.stage.to_string()),
                    prop: None,
                    left: None,
                    right: None,
                    reason: None,
                    certificate: None,
                    bound: None,
                    allowed: None,
                }),
            },
            Refusal::Contradictory(witness) => Marshaled {
                class: ExceptionClass::Refused,
                message: format!(
                    "kernel refusal: contradictory evidence on {} ({} vs {})",
                    prop_name(witness.prop),
                    truth_name(witness.left),
                    truth_name(witness.right)
                ),
                payload: MarshaledPayload::Refused(RefusedPayload {
                    case: "contradictory".to_string(),
                    envelope: None,
                    stage: None,
                    prop: Some(prop_name(witness.prop).to_string()),
                    left: Some(truth_name(witness.left).to_string()),
                    right: Some(truth_name(witness.right).to_string()),
                    reason: None,
                    certificate: None,
                    bound: None,
                    allowed: None,
                }),
            },
            Refusal::Collapsed(collapse, certificate) => Marshaled {
                class: ExceptionClass::Refused,
                message: format!(
                    "kernel refusal: exact object collapsed ({})",
                    collapse_reason_name(collapse.reason)
                ),
                payload: MarshaledPayload::Refused(RefusedPayload {
                    case: "collapsed".to_string(),
                    envelope: None,
                    stage: None,
                    prop: None,
                    left: None,
                    right: None,
                    reason: Some(collapse_reason_name(collapse.reason).to_string()),
                    certificate: Some(certificate_to_value(certificate)),
                    bound: None,
                    allowed: None,
                }),
            },
            Refusal::ForwardToleranceExceeded { bound, allowed } => Marshaled {
                class: ExceptionClass::Refused,
                message: format!(
                    "kernel refusal: forward tolerance exceeded (bound {bound}, allowed {allowed})"
                ),
                payload: MarshaledPayload::Refused(RefusedPayload {
                    case: "forward_tolerance_exceeded".to_string(),
                    envelope: None,
                    stage: None,
                    prop: None,
                    left: None,
                    right: None,
                    reason: None,
                    certificate: None,
                    bound: Some(*bound),
                    allowed: Some(*allowed),
                }),
            },
        }
    }
}

/// `EnvelopeCase` → snake_case name. Exhaustive; the six payloads of §2.
pub fn envelope_case_name(case: EnvelopeCase) -> &'static str {
    match case {
        EnvelopeCase::ChartDegenerate => "chart_degenerate",
        EnvelopeCase::ReachTooSmall => "reach_too_small",
        EnvelopeCase::NonCanonicalCarrier => "non_canonical_carrier",
        EnvelopeCase::NonPositiveNurbsWeight => "non_positive_nurbs_weight",
        EnvelopeCase::ContactReductionDeferred => "contact_reduction_deferred",
        EnvelopeCase::ConstructRefused => "construct_refused",
    }
}

/// `UnresolvedWitness` → snake_case name. Exhaustive; the five witnesses of §2.
pub fn witness_name(witness: UnresolvedWitness) -> &'static str {
    match witness {
        UnresolvedWitness::UncertifiedContainment => "uncertified_containment",
        UnresolvedWitness::RootNotIsolated => "root_not_isolated",
        UnresolvedWitness::KrawczykIndeterminate => "krawczyk_indeterminate",
        UnresolvedWitness::ContactCurveNotFound => "contact_curve_not_found",
        UnresolvedWitness::DeviationUncertified => "deviation_uncertified",
    }
}

/// `Prop` → snake_case name. Exhaustive over the landed property set.
pub fn prop_name(prop: Prop) -> &'static str {
    match prop {
        Prop::AnalyticCarrier => "analytic_carrier",
        Prop::SoundEnclosure => "sound_enclosure",
        Prop::Provisional => "provisional",
        Prop::AnalyticPreserved => "analytic_preserved",
        Prop::CoedgePairing => "coedge_pairing",
        Prop::VertexLink => "vertex_link",
        Prop::EulerPoincare => "euler_poincare",
        Prop::SameParameter => "same_parameter",
        Prop::DomainBoundary => "domain_boundary",
        Prop::Representation => "representation",
        Prop::ToleranceMonotonicity => "tolerance_monotonicity",
        Prop::ShellNesting => "shell_nesting",
        Prop::WedgeNonDegeneracy => "wedge_non_degeneracy",
        Prop::FragmentInsideOther => "fragment_inside_other",
    }
}

/// `Truth` → snake_case name.
pub fn truth_name(truth: Truth) -> &'static str {
    match truth {
        Truth::Unknown => "unknown",
        Truth::True => "true",
        Truth::False => "false",
        Truth::Both => "both",
    }
}

/// `CollapseReason` → snake_case name.
pub fn collapse_reason_name(reason: truck_base::evidence::CollapseReason) -> &'static str {
    match reason {
        truck_base::evidence::CollapseReason::KnifeEdge => "knife_edge",
        truck_base::evidence::CollapseReason::ApexVanishing => "apex_vanishing",
    }
}

/// `Method` → snake_case name.
pub fn method_name(method: Method) -> &'static str {
    match method {
        Method::Exact => "exact",
        Method::Interval => "interval",
        Method::Float => "float",
        Method::None => "none",
    }
}

/// Renders the certificate (π, μ, β, 𝔪, ω) as a JSON value for the `Collapsed`
/// payload. Total over the public surface of the certificate; every counter,
/// the method, the margin exponent, the modulus (shape constants + domain) and
/// the property map ride along. An unbounded margin or modulus domain is
/// `null` (JSON cannot carry `f64::INFINITY`).
fn certificate_to_value(certificate: &Certificate) -> serde_json::Value {
    let margin = certificate.margin.log2();
    json!({
        "method": method_name(certificate.method),
        "budget_left": {
            "subdiv": certificate.budget_left.subdiv,
            "newton": certificate.budget_left.newton,
            "depth": certificate.budget_left.depth,
        },
        "margin_log2": finite_or_null(margin),
        "modulus": {
            "domain": finite_or_null(certificate.modulus.domain),
            "shape": modulus_shape_to_value(&certificate.modulus),
        },
        "props": props_to_value(certificate),
    })
}

fn modulus_shape_to_value(modulus: &Modulus) -> serde_json::Value {
    match modulus.shape {
        ModulusShape::Lipschitz(k) => json!({ "kind": "lipschitz", "k": finite_or_null(k) }),
        ModulusShape::Holder { k, exponent } => json!({
            "kind": "holder",
            "k": finite_or_null(k),
            "exponent": finite_or_null(exponent),
        }),
        ModulusShape::Pole { k } => json!({ "kind": "pole", "k": finite_or_null(k) }),
        ModulusShape::Unbounded => json!({ "kind": "unbounded" }),
    }
}

/// The set properties of the certificate's property map, as an object of
/// `{prop_name: truth_name}`. Unknown (unset) properties are omitted.
fn props_to_value(certificate: &Certificate) -> serde_json::Value {
    let mut props = serde_json::Map::new();
    for prop in all_props() {
        let truth = certificate.props.get(prop);
        if truth != Truth::Unknown {
            props.insert(
                prop_name(prop).to_string(),
                serde_json::Value::String(truth_name(truth).to_string()),
            );
        }
    }
    serde_json::Value::Object(props)
}

/// Every landed `Prop`, for rendering a `PropMap` without owning one (the map
/// has no public iterator; the accessor is `get`).
fn all_props() -> [Prop; 14] {
    [
        Prop::AnalyticCarrier,
        Prop::SoundEnclosure,
        Prop::Provisional,
        Prop::AnalyticPreserved,
        Prop::CoedgePairing,
        Prop::VertexLink,
        Prop::EulerPoincare,
        Prop::SameParameter,
        Prop::DomainBoundary,
        Prop::Representation,
        Prop::ToleranceMonotonicity,
        Prop::ShellNesting,
        Prop::WedgeNonDegeneracy,
        Prop::FragmentInsideOther,
    ]
}

fn finite_or_null(value: f64) -> serde_json::Value {
    if value.is_finite() {
        serde_json::Value::from(value)
    } else {
        serde_json::Value::Null
    }
}

/// Convenience: the full `Ledger` of a kernel `Budget`.
pub fn ledger_from_budget(budget: &Budget) -> Ledger {
    Ledger {
        subdiv: budget.subdiv,
        newton: budget.newton,
        depth: budget.depth,
    }
}

// ---------------------------------------------------------------------------
// CG-BINDING — the total certified-refusal -> typed-door-vocabulary mapping.
//
// The base `Refusal` mapping above is the PB-000 §2 table. The certified
// funnel (`truck-certified`) refuses through two OTHER vocabularies — the
// construct-layer `ConstructRefusal` (CC-000-C4) and the kernel-v2
// `RefusalKind` (§17) — and those refusals must reach the corpus door as the
// same typed cases, never as a bare string and never by approximation. Per
// `docs/CERTIFICATE_MAPPING.md` §C row 1, every construct-stage refusal maps
// onto the single booked `construct_refused` door case (the detailed variant
// rides the payload). A kind the table does not name is itself a typed
// `UnmappedRefusal` — a defect signal, never silent.
// ---------------------------------------------------------------------------

/// The typed door-refusal vocabulary: the base `Refusal` cases of the frozen
/// §2 table plus the two certified-funnel cases. A `DoorCase` is a named
/// cause, never a bare string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DoorCase {
    /// `Refusal::Empty` — an empty operation domain.
    Empty,
    /// `Refusal::UnsupportedEnvelope` — a carrier/envelope the funnel refuses.
    UnsupportedEnvelope,
    /// `Refusal::NumericallyUnresolved` — the budget was exhausted.
    NumericallyUnresolved,
    /// `Refusal::CompositionMarginExhausted`.
    CompositionMarginExhausted,
    /// `Refusal::InputOutsideBackwardBudget`.
    InputOutsideBackwardBudget,
    /// `Refusal::Contradictory` — conflicting evidence.
    Contradictory,
    /// `Refusal::Collapsed` — an exact object collapsed.
    Collapsed,
    /// `Refusal::ForwardToleranceExceeded`.
    ForwardToleranceExceeded,
    /// A construct-stage refusal (CERTIFICATE_MAPPING §C row 1).
    ConstructRefused,
    /// A certified refusal kind with no booked door mapping: the defect
    /// signal. Never silent, never approximated.
    UnmappedRefusal,
}

/// The marshaled form of an unmapped certified refusal kind (the defect
/// signal). Carries the offending kind name so the defect is traceable; it is
/// raised as a `Refused` exception like every other typed refusal, never as a
/// bare `Exception`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnmappedRefusal {
    /// The certified refusal kind with no booked mapping.
    pub kind: String,
}

impl UnmappedRefusal {
    /// Names the unmapped kind.
    pub fn new(kind: impl Into<String>) -> Self {
        Self { kind: kind.into() }
    }

    /// The typed door case of this defect signal.
    pub fn door_case(&self) -> DoorCase {
        DoorCase::UnmappedRefusal
    }

    /// The marshaled `Refused` form of the defect signal.
    pub fn marshaled(&self) -> Marshaled {
        Marshaled::from_unmapped_refusal(self)
    }
}

/// The `RefusedPayload` skeleton of one named door case (every optional field
/// absent). Shared by the certified-refusal marshaling helpers.
fn refused_payload(case: &str) -> RefusedPayload {
    RefusedPayload {
        case: case.to_string(),
        envelope: None,
        stage: None,
        prop: None,
        left: None,
        right: None,
        reason: None,
        certificate: None,
        bound: None,
        allowed: None,
    }
}

impl Marshaled {
    /// Marshals a construct-stage `ConstructRefusal` into the typed door
    /// vocabulary (CERTIFICATE_MAPPING §C row 1): the single booked
    /// `construct_refused` case, with the construct variant tag carried on the
    /// payload's `stage` field. Total over the frozen 14-variant set.
    pub fn from_construct_refusal(refusal: ConstructRefusal) -> Marshaled {
        let mut payload = refused_payload("construct_refused");
        payload.envelope = Some("construct_refused".to_string());
        payload.stage = Some(refusal.tag().to_string());
        Marshaled {
            class: ExceptionClass::Refused,
            message: format!("kernel construct refusal: {}", refusal.tag()),
            payload: MarshaledPayload::Refused(payload),
        }
    }

    /// Marshals an unmapped certified refusal kind as the typed
    /// [`UnmappedRefusal`] defect signal: `Refused` class, `unmapped_refusal`
    /// case, the offending kind on the payload. Never a bare string, never a
    /// silent approximation.
    pub fn from_unmapped_refusal(refusal: &UnmappedRefusal) -> Marshaled {
        let mut payload = refused_payload("unmapped_refusal");
        payload.stage = Some(refusal.kind.clone());
        Marshaled {
            class: ExceptionClass::Refused,
            message: format!("kernel refusal kind has no door mapping: {}", refusal.kind),
            payload: MarshaledPayload::Refused(payload),
        }
    }
}

/// The snake_case name of a landed kernel-v2 `RefusalKind` (the §17
/// taxonomy). Exhaustive over the frozen 25-variant set; a new variant breaks
/// this function on purpose (the mapping table must be extended by spec edit,
/// not by a silent wildcard).
// The door-vocabulary surface is consumed by `binding.rs` (and the follow-on
// door-shim flip); `lib.rs` is outside this packet's write set, so the items
// cannot be added to the crate-root re-export list yet.
#[allow(dead_code)]
pub fn kernel_refusal_kind_name(kind: RefusalKind) -> &'static str {
    match kind {
        RefusalKind::SpineNotC1 => "spine_not_c1",
        RefusalKind::FrameSingular => "frame_singular",
        RefusalKind::ProfileCollapse => "profile_collapse",
        RefusalKind::ProfileCorrespondenceMismatch => "profile_correspondence_mismatch",
        RefusalKind::NonFinite => "non_finite",
        RefusalKind::WindingAuditFailed => "winding_audit_failed",
        RefusalKind::NonDyadicSharedRequest => "non_dyadic_shared_request",
        RefusalKind::CarrierSingularity => "carrier_singularity",
        RefusalKind::ChartExhausted => "chart_exhausted",
        RefusalKind::TranscendentalCarrier => "transcendental_carrier",
        RefusalKind::WeightDegenerate => "weight_degenerate",
        RefusalKind::DeckExhausted => "deck_exhausted",
        RefusalKind::Conditioning => "conditioning",
        RefusalKind::TangentialCurve => "tangential_curve",
        RefusalKind::HighOrderJet => "high_order_jet",
        RefusalKind::IncompleteStartSet => "incomplete_start_set",
        RefusalKind::R5EnclosureFailed => "r5_enclosure_failed",
        RefusalKind::TrimClipFailed => "trim_clip_failed",
        RefusalKind::NearOverlap => "near_overlap",
        RefusalKind::OffsetDegenerate => "offset_degenerate",
        RefusalKind::OffsetSwallowtail => "offset_swallowtail",
        RefusalKind::CornerUnsolved => "corner_unsolved",
        RefusalKind::SliverOrNearOverlap => "sliver_or_near_overlap",
        RefusalKind::ClaimRefuted => "claim_refuted",
        RefusalKind::Budget => "budget",
    }
}

/// The door case of a landed kernel-v2 `RefusalKind` name. Every §17 variant
/// is booked here; a name the table does not hold is the typed
/// [`DoorCase::UnmappedRefusal`] defect signal — never silent, never a guess.
#[allow(dead_code)]
pub fn door_case_of_kernel_refusal_name(name: &str) -> DoorCase {
    match name {
        // The construct/carrier classes (CERTIFICATE_MAPPING §C row 1).
        "spine_not_c1"
        | "frame_singular"
        | "profile_collapse"
        | "profile_correspondence_mismatch" => DoorCase::ConstructRefused,
        // The unsupported-envelope classes.
        "non_dyadic_shared_request" | "carrier_singularity" | "transcendental_carrier" => {
            DoorCase::UnsupportedEnvelope
        }
        // The contradictory-evidence classes.
        "winding_audit_failed" | "weight_degenerate" | "near_overlap" | "claim_refuted" => {
            DoorCase::Contradictory
        }
        // The collapse classes.
        "offset_degenerate" | "offset_swallowtail" => DoorCase::Collapsed,
        // The forward-tolerance class.
        "non_finite" => DoorCase::ForwardToleranceExceeded,
        // Every inconclusive-class §17 variant is the budget/conditioning
        // unresolved door case.
        "chart_exhausted"
        | "deck_exhausted"
        | "conditioning"
        | "tangential_curve"
        | "high_order_jet"
        | "incomplete_start_set"
        | "r5_enclosure_failed"
        | "trim_clip_failed"
        | "corner_unsolved"
        | "sliver_or_near_overlap"
        | "budget" => DoorCase::NumericallyUnresolved,
        _ => DoorCase::UnmappedRefusal,
    }
}

/// The door case of a landed kernel-v2 `RefusalKind`. Total over the frozen
/// 25-variant set; the name table is the single mapping source.
#[allow(dead_code)]
pub fn door_case_of_kernel_refusal_kind(kind: RefusalKind) -> DoorCase {
    door_case_of_kernel_refusal_name(kernel_refusal_kind_name(kind))
}

/// The door case of a construct-stage `ConstructRefusal`. Per
/// CERTIFICATE_MAPPING §C row 1 every construct-stage failure maps onto the
/// single booked `construct_refused` case; the variant detail rides the
/// payload (`Marshaled::from_construct_refusal`). Total over the frozen
/// 14-variant set.
#[allow(dead_code)]
pub fn door_case_of_construct_refusal(refusal: ConstructRefusal) -> DoorCase {
    let _ = refusal.tag();
    DoorCase::ConstructRefused
}
