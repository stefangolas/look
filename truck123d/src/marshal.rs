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
