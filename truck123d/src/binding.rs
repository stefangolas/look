//! CG-BINDING — the pyo3 translation over the stabilized facade (the
//! dependency-wall resolution).
//!
//! **The manifest-edge amendment (C2 precedent, recorded — do not
//! relitigate).** The recorded law "the loop-side crate cannot name
//! truck-certified" (STATE session-56, applied in PB-011) is AMENDED by this
//! packet: exactly one edge, `truck123d -> truck-certified`, is added to
//! `truck123d/Cargo.toml` and is added once. The loop-side crate may not name
//! truck-certified *except* through this one recorded edge, added once and
//! never extended without a spec amendment. The traits stay in truck-evidence;
//! the impls stay in truck-certified; this binding reaches the impls through
//! the stabilized facade's public entry points. No funnel code moves.
//!
//! **Why.** The door gap audit (`loop/audits/DOOR_GAP_AUDIT-2026-09-09.md`)
//! recorded the wall: the data-row executor can never call the certified
//! funnel in-process because the loop-side crate could not name it. This
//! module is the sanctioned resolution (AGENTS.md's booked "pyo3 binding
//! translation over the stabilized facade"): it exposes the funnel's runtime
//! twin to Python so the corpus door can dispatch against the landed
//! admission/boolean/volume machinery instead of its own analytic arms.
//!
//! **Three exports over the stabilized facade (minimal).**
//!
//! 1. [`binding_boolean_dispatch`] — a carrier pair + mode in, a
//!    `Routed`/`Refused` verdict out: the facade mirror's
//!    ([`facade::dispatch_swept_carrier_boolean`]) runtime twin. The facade
//!    mirror owns the carrier-class admission consult; when it routes the
//!    pair, this entry actually runs the certified Theorem C transversality
//!    certificate ([`certify_transverse_pair`]) over the row's recorded
//!    certified normal cones and returns the certified `‖n_X × n_Y‖` bracket.
//! 2. [`binding_volume_facts`] — an admitted construction's certified volume
//!    bracket out ([`certify_patch_form`] over the recorded patch row).
//! 3. [`binding_trim_facts`] — the algebraic-trim path's certified bracket out
//!    ([`certify_algebraic_trim_bracket`] over the recorded pullback nets).
//!
//! The exports consume EXACT data rows (the bridge's recorded carrier shapes);
//! no float crosses without an enclosing bracket. The rows are serde values,
//! exactly as the rest of the bridge marshals its tables.
//!
//! **Refusal marshaling is total (scope decision 3).** Every kernel refusal
//! kind maps to the door's typed vocabulary per
//! `docs/CERTIFICATE_MAPPING.md`; the binding never panics, never returns a
//! bare string, never approximates. An unmapped kind is itself a typed
//! [`UnmappedRefusal`] — a defect signal, never silent. The mapping lives in
//! [`crate::marshal`] beside the frozen §2 table.
//!
//! **Determinism (N4 carries).** Same row in → identical verdict and identical
//! bracket out, bit-for-bit, across reruns. Fixed order everywhere; no hash
//! iteration on any output path.
//!
//! **Reachability note.** `lib.rs` is outside this packet's write set, so the
//! module is declared from its sibling `python.rs` (the pyo3 surface module)
//! and the three `#[pyfunction]`s are exposed here; the follow-on consumer
//! packets flip the door shim against this landed surface.
//!
//! **H-1.** No `unwrap`, no `expect`, no `panic!`, no out-of-range indexing on
//! any shipped path; every failure is a typed value.

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
// The three exports are the follow-on door-shim consumers' surface. `lib.rs`
// is outside this packet's write set, so the pyo3 registration cannot be wired
// here yet; the module's items are therefore unreachable from the crate root
// and would read as dead code. Scoped to this one export module only.
#![allow(dead_code)]

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

use crate::facade::{self, BooleanPairVerdict, CarrierClass, ModeValue};
use crate::marshal::{ExceptionClass, Marshaled, MarshaledPayload};

use truck_certified::construct::admission::{NormalCone, certify_transverse_pair};
use truck_certified::construct::patches::{PatchParent, TensorBernsteinPatch};
use truck_certified::construct::volume_facts::{
    VolumeOptions, certify_algebraic_trim_bracket, certify_patch_form,
};
use truck_certified::kernel::patch::IBox2;

/// A closed two-sided bracket `[lo, hi]`. The only numeric shape an export
/// ever returns: no float crosses the boundary without its enclosing bracket.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Bracket {
    /// The certified lower bound.
    pub lo: f64,
    /// The certified upper bound.
    pub hi: f64,
}

/// A malformed input row (a caller defect, distinct from a kernel refusal).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MalformedRow {
    /// What was wrong with the row.
    pub detail: String,
}

/// The two typed failure modes of an export: a marshaled kernel refusal, or a
/// malformed row. Both are typed; neither is a bare string. The refusal is
/// boxed so the `Result` error stays small on the hot path.
#[derive(Debug, Clone, PartialEq)]
pub enum BindingError {
    /// A kernel refusal, marshaled to the typed door vocabulary.
    Refusal(Box<Marshaled>),
    /// A malformed input row.
    Malformed(MalformedRow),
}

/// A certified normal-cone row: the anchor direction and the certified upper
/// bound on the sine of the cone half-angle (the `admission::NormalCone`
/// public shape).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConeRow {
    /// The anchor direction (the float midpoint normal, rounded to dyadic).
    pub anchor: [f64; 3],
    /// A certified upper bound of `sin` of the cone half-angle, `< 1`.
    pub s_up: f64,
}

impl ConeRow {
    /// The landed certified-funnel cone value this row records.
    pub fn to_cone(self) -> NormalCone {
        NormalCone {
            anchor: self.anchor,
            s_up: self.s_up,
        }
    }
}

/// The boolean-dispatch row: the carrier-class pair and mode of the facade
/// mirror, plus the two carriers' recorded certified normal cones (the
/// Theorem C transversality witnesses the runtime twin composes).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BooleanRow {
    /// The carrier class of the base solid.
    pub base: CarrierClass,
    /// The carrier class of the tool solid.
    pub tool: CarrierClass,
    /// The boolean mode of the pair.
    pub mode: ModeValue,
    /// The base carrier's recorded certified normal cone.
    pub base_cone: ConeRow,
    /// The tool carrier's recorded certified normal cone.
    pub tool_cone: ConeRow,
}

/// The observed verdict of one boolean pair through the runtime twin.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BooleanOutcome {
    /// Always `true` on the `Ok` path; present so the record is
    /// self-describing.
    pub ok: bool,
    /// `canonical_landed` | `routed` | `refused`.
    pub verdict: &'static str,
    /// The boolean mode of the pair.
    pub mode: ModeValue,
    /// The carrier class of the base solid.
    pub base: CarrierClass,
    /// The carrier class of the tool solid.
    pub tool: CarrierClass,
    /// The certified transversality bracket of a routed pair.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transverse: Option<Bracket>,
    /// The typed envelope case of a facade-level refusal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub envelope: Option<&'static str>,
}

/// The volume-facts row: an admitted construction's recorded patch shape (the
/// numerator and weight Bernstein grids over the unit square) plus its
/// outward-orientation sign.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VolumeRow {
    /// The row-major `R³` numerator grid.
    pub numerator: Vec<Vec<[f64; 3]>>,
    /// The same-shape scalar weight grid.
    pub weights: Vec<Vec<f64>>,
    /// The outward-orientation sign `±1`.
    pub orientation: f64,
}

/// The certified volume fact of one admitted patch.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VolumeOutcome {
    /// Always `true` on the `Ok` path.
    pub ok: bool,
    /// The certified two-sided volume bracket.
    pub bracket: Bracket,
    /// The certified volume value (the bracket midpoint).
    pub value: f64,
    /// The certified absolute error added by the rational/trim routes.
    pub certified_error: f64,
    /// The number of boundary faces contributing.
    pub faces: usize,
    /// The number of extracted patches contributing.
    pub patches: usize,
}

/// The trim-facts row: the polynomial density net and the algebraic trim net
/// (whose kept region is `trim ≥ 0`) over the unit square, plus the requested
/// bracket width.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrimRow {
    /// The polynomial integrand net (row-major).
    pub density: Vec<Vec<f64>>,
    /// The trim polynomial net (row-major).
    pub trim: Vec<Vec<f64>>,
    /// The requested residual bracket width.
    pub tolerance: f64,
}

/// The certified bracket of one algebraically trimmed domain.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TrimOutcome {
    /// Always `true` on the `Ok` path.
    pub ok: bool,
    /// The certified two-sided integral bracket.
    pub bracket: Bracket,
}

/// The unit-square domain every extracted patch row is re-parameterized onto.
fn unit_domain() -> IBox2 {
    IBox2 {
        lo: [0.0, 0.0],
        hi: [1.0, 1.0],
    }
}

/// Serializes an outcome, mapping a serde failure to a typed malformed row
/// (never an unwrap, never a bare string).
fn to_json<T: Serialize>(value: &T) -> Result<String, BindingError> {
    serde_json::to_string(value).map_err(|e| {
        BindingError::Malformed(MalformedRow {
            detail: format!("outcome serialization failed: {e}"),
        })
    })
}

/// Registers the certified funnel engines into the `truck-evidence`
/// trait-object registries (CL-006-SOLVER-ENTRY's restricted-pair engine and
/// CL-001-SPLINE-LIFT's spline×analytic SSI engine). This is the concrete
/// "call the impls through the trait objects" step: the certified impls land
/// in the registry slots the funnel dispatches through, so the door's funnel
/// path reaches the landed machinery in-process. Registration is set-once
/// (`#[ctor]`-free, explicit); a redundant call is a no-op for the binding's
/// verdict.
pub fn register_certified_entries() {
    let _ = truck_certified::construct::bie::ssi4::register_restricted_pair_solver();
    let _ = truck_certified::ssi_admit::register_ssi_admit_solver();
}

/// The certified boolean dispatch: the facade mirror's runtime twin. The
/// facade's carrier-class admission consult decides canonical/refused/routed;
/// a routed pair actually runs the certified Theorem C transversality
/// certificate over the row's recorded cones and returns the certified
/// bracket. A certified refusal marshals to the typed door vocabulary.
pub fn boolean_dispatch(row: &BooleanRow) -> Result<String, BindingError> {
    register_certified_entries();
    match facade::dispatch_swept_carrier_boolean(row.base, row.tool, row.mode) {
        BooleanPairVerdict::CanonicalLanded => to_json(&BooleanOutcome {
            ok: true,
            verdict: "canonical_landed",
            mode: row.mode,
            base: row.base,
            tool: row.tool,
            transverse: None,
            envelope: None,
        }),
        BooleanPairVerdict::Refused(refusal) => to_json(&BooleanOutcome {
            ok: true,
            verdict: "refused",
            mode: row.mode,
            base: row.base,
            tool: row.tool,
            transverse: None,
            envelope: Some(crate::marshal::envelope_case_name(refusal.case)),
        }),
        BooleanPairVerdict::Routed(route) => {
            let base_cone = row.base_cone.to_cone();
            let tool_cone = row.tool_cone.to_cone();
            match certify_transverse_pair(&base_cone, &tool_cone) {
                Ok(transverse) => to_json(&BooleanOutcome {
                    ok: true,
                    verdict: "routed",
                    mode: route.mode,
                    base: route.base,
                    tool: route.tool,
                    transverse: Some(Bracket {
                        lo: transverse.delta.0,
                        hi: transverse.delta.1,
                    }),
                    envelope: None,
                }),
                Err(refusal) => Err(BindingError::Refusal(Box::new(
                    Marshaled::from_construct_refusal(refusal),
                ))),
            }
        }
    }
}

/// The certified volume-facts entry: assemble the recorded patch row and run
/// the landed certified face-form volume fact, returning its two-sided
/// bracket. A patch or volume refusal marshals to the typed door vocabulary.
pub fn volume_facts(row: &VolumeRow) -> Result<String, BindingError> {
    let patch = TensorBernsteinPatch::try_new(
        row.numerator.clone(),
        row.weights.clone(),
        unit_domain(),
        PatchParent::new(0, None),
    )
    .map_err(|refusal| {
        BindingError::Refusal(Box::new(Marshaled::from_construct_refusal(refusal)))
    })?;
    let fact = certify_patch_form(&patch, row.orientation, &VolumeOptions::default()).map_err(
        |refusal| BindingError::Refusal(Box::new(Marshaled::from_construct_refusal(refusal))),
    )?;
    to_json(&VolumeOutcome {
        ok: true,
        bracket: Bracket {
            lo: fact.bracket.lo,
            hi: fact.bracket.hi,
        },
        value: fact.value,
        certified_error: fact.certified_error,
        faces: fact.faces,
        patches: fact.patches,
    })
}

/// The certified trim-facts entry for the algebraic-trim path: run the landed
/// certified algebraic-trim bracket engine over the recorded pullback nets and
/// return the two-sided integral bracket. A refusal marshals to the typed door
/// vocabulary.
pub fn trim_facts(row: &TrimRow) -> Result<String, BindingError> {
    let bracket = certify_algebraic_trim_bracket(&row.density, &row.trim, row.tolerance).map_err(
        |refusal| BindingError::Refusal(Box::new(Marshaled::from_construct_refusal(refusal))),
    )?;
    to_json(&TrimOutcome {
        ok: true,
        bracket: Bracket {
            lo: bracket.lo,
            hi: bracket.hi,
        },
    })
}

/// Converts a [`BindingError`] into the typed Python exception: a kernel
/// refusal becomes the marshaled `Refused`/`Unresolved` instance (never a bare
/// `Exception`), a malformed row a `ValueError`.
fn to_pyerr(py: Python<'_>, error: BindingError) -> PyErr {
    match error {
        BindingError::Malformed(malformed) => PyValueError::new_err(malformed.detail),
        BindingError::Refusal(marshaled) => {
            let payload_json = match &marshaled.payload {
                MarshaledPayload::Refused(payload) => serde_json::to_string(payload),
                MarshaledPayload::Unresolved(payload) => serde_json::to_string(payload),
            };
            let payload_json = match payload_json {
                Ok(json) => json,
                Err(e) => {
                    return PyRuntimeError::new_err(format!(
                        "refusal payload serialization failed: {e}"
                    ));
                }
            };
            let instance = match marshaled.class {
                ExceptionClass::Refused => crate::python::marshal_refused(py, &payload_json),
                ExceptionClass::Unresolved => crate::python::marshal_unresolved(py, &payload_json),
            };
            match instance {
                Ok(value) => PyErr::from_value(value.bind(py).clone()),
                Err(e) => e,
            }
        }
    }
}

/// The pyo3 export of [`boolean_dispatch`].
#[pyfunction]
pub fn binding_boolean_dispatch(py: Python<'_>, row_json: &str) -> PyResult<String> {
    let row: BooleanRow = serde_json::from_str(row_json)
        .map_err(|e| PyValueError::new_err(format!("invalid boolean row JSON: {e}")))?;
    boolean_dispatch(&row).map_err(|e| to_pyerr(py, e))
}

/// The pyo3 export of [`volume_facts`].
#[pyfunction]
pub fn binding_volume_facts(py: Python<'_>, row_json: &str) -> PyResult<String> {
    let row: VolumeRow = serde_json::from_str(row_json)
        .map_err(|e| PyValueError::new_err(format!("invalid volume row JSON: {e}")))?;
    volume_facts(&row).map_err(|e| to_pyerr(py, e))
}

/// The pyo3 export of [`trim_facts`].
#[pyfunction]
pub fn binding_trim_facts(py: Python<'_>, row_json: &str) -> PyResult<String> {
    let row: TrimRow = serde_json::from_str(row_json)
        .map_err(|e| PyValueError::new_err(format!("invalid trim row JSON: {e}")))?;
    trim_facts(&row).map_err(|e| to_pyerr(py, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::marshal::{
        DoorCase, UnmappedRefusal, door_case_of_construct_refusal,
        door_case_of_kernel_refusal_kind, door_case_of_kernel_refusal_name,
        kernel_refusal_kind_name,
    };
    use truck_certified::construct::refusal::ConstructRefusal;
    use truck_certified::kernel::evidence::RefusalKind;

    /// Every landed §17 kernel refusal kind (the frozen 25-variant set).
    const KERNEL_KINDS: [RefusalKind; 25] = [
        RefusalKind::SpineNotC1,
        RefusalKind::FrameSingular,
        RefusalKind::ProfileCollapse,
        RefusalKind::ProfileCorrespondenceMismatch,
        RefusalKind::NonFinite,
        RefusalKind::WindingAuditFailed,
        RefusalKind::NonDyadicSharedRequest,
        RefusalKind::CarrierSingularity,
        RefusalKind::ChartExhausted,
        RefusalKind::TranscendentalCarrier,
        RefusalKind::WeightDegenerate,
        RefusalKind::DeckExhausted,
        RefusalKind::Conditioning,
        RefusalKind::TangentialCurve,
        RefusalKind::HighOrderJet,
        RefusalKind::IncompleteStartSet,
        RefusalKind::R5EnclosureFailed,
        RefusalKind::TrimClipFailed,
        RefusalKind::NearOverlap,
        RefusalKind::OffsetDegenerate,
        RefusalKind::OffsetSwallowtail,
        RefusalKind::CornerUnsolved,
        RefusalKind::SliverOrNearOverlap,
        RefusalKind::ClaimRefuted,
        RefusalKind::Budget,
    ];

    /// Every landed construct refusal (the frozen 14-variant set).
    const CONSTRUCT_REFUSALS: [ConstructRefusal; 14] = [
        ConstructRefusal::NonPositiveWeightField,
        ConstructRefusal::SingularInterpolationSystem,
        ConstructRefusal::AmbiguousCorrespondence,
        ConstructRefusal::FocalDegeneracy,
        ConstructRefusal::CanalSingular,
        ConstructRefusal::RankDeficientContact,
        ConstructRefusal::UnintendedContact,
        ConstructRefusal::StarNotEmbedded,
        ConstructRefusal::NoAdmissibleProjection,
        ConstructRefusal::NonGenericThicknessEvent,
        ConstructRefusal::AmbiguousEventOrdering,
        ConstructRefusal::InvalidInput,
        ConstructRefusal::ConditioningBelowThreshold,
        ConstructRefusal::Unfrozen,
    ];

    fn cone(anchor: [f64; 3], s_up: f64) -> ConeRow {
        ConeRow { anchor, s_up }
    }

    /// The canonical admitted boolean row: a spline base against a canonical
    /// tool with two well-separated certified cones.
    fn canonical_row() -> BooleanRow {
        BooleanRow {
            base: CarrierClass::Spline,
            tool: CarrierClass::Canonical,
            mode: ModeValue::Add,
            base_cone: cone([0.0, 0.0, 1.0], 0.1),
            tool_cone: cone([1.0, 0.0, 0.0], 0.1),
        }
    }

    /// A non-degenerate admitted patch row (`X(u, v) = (u, v, 1)`, unit
    /// weights) whose exact face-form volume is `1/3`.
    fn canonical_volume_row() -> VolumeRow {
        VolumeRow {
            numerator: vec![
                vec![[0.0, 0.0, 1.0], [0.0, 1.0, 1.0]],
                vec![[1.0, 0.0, 1.0], [1.0, 1.0, 1.0]],
            ],
            weights: vec![vec![1.0, 1.0], vec![1.0, 1.0]],
            orientation: 1.0,
        }
    }

    /// An axis-aligned trim row: the constant density `1` over the whole unit
    /// square (the trim `1` is strictly positive everywhere).
    fn canonical_trim_row() -> TrimRow {
        TrimRow {
            density: vec![vec![1.0]],
            trim: vec![vec![1.0]],
            tolerance: 1.0e-3,
        }
    }

    fn verdict(json: &str) -> String {
        let value: serde_json::Value = serde_json::from_str(json).expect("outcome json");
        value["verdict"].as_str().unwrap_or_default().to_string()
    }

    #[test]
    fn binding_exports_the_certified_boolean_entry() {
        // A routed pair returns the certified transversality bracket.
        let routed = boolean_dispatch(&canonical_row()).expect("routed pair");
        assert_eq!(verdict(&routed), "routed");
        let value: serde_json::Value = serde_json::from_str(&routed).expect("routed json");
        let lo = value["transverse"]["lo"].as_f64().expect("certified lo");
        let hi = value["transverse"]["hi"].as_f64().expect("certified hi");
        assert!(lo > 0.0, "the certified margin must be strictly positive");
        assert!(lo <= hi, "the certified bracket must be ordered");

        // A canonical x canonical pair lands on the S1 path, never dispatched.
        let landed = BooleanRow {
            base: CarrierClass::Canonical,
            tool: CarrierClass::Canonical,
            ..canonical_row()
        };
        assert_eq!(
            verdict(&boolean_dispatch(&landed).expect("landed pair")),
            "canonical_landed"
        );

        // A torus carrier is refused typed at the facade boundary.
        let torus = BooleanRow {
            tool: CarrierClass::Torus,
            ..canonical_row()
        };
        let refused = boolean_dispatch(&torus).expect("facade refusal verdict");
        assert_eq!(verdict(&refused), "refused");
        assert!(refused.contains("contact_reduction_deferred"));

        // A pair whose cones do not certify separation marshals the kernel
        // refusal to the typed door vocabulary (no panic, no bare string).
        let overlapping = BooleanRow {
            base_cone: cone([0.0, 0.0, 1.0], 0.99),
            tool_cone: cone([0.0, 0.0, 1.0], 0.99),
            ..canonical_row()
        };
        match boolean_dispatch(&overlapping) {
            Err(BindingError::Refusal(marshaled)) => {
                assert_eq!(marshaled.class, ExceptionClass::Refused);
                match marshaled.payload {
                    MarshaledPayload::Refused(payload) => {
                        assert_eq!(payload.case, "construct_refused");
                        assert_eq!(payload.stage.as_deref(), Some("ConditioningBelowThreshold"));
                    }
                    MarshaledPayload::Unresolved(_) => {
                        panic!("a construct refusal must not marshal as Unresolved")
                    }
                }
            }
            other => panic!("expected a typed refusal, got {other:?}"),
        }
    }

    #[test]
    fn kernel_refusals_marshall_to_typed_door_vocabulary() {
        // Every landed §17 kind maps to a typed door case, none unmapped.
        for kind in KERNEL_KINDS {
            let case = door_case_of_kernel_refusal_kind(kind);
            assert_ne!(
                case,
                DoorCase::UnmappedRefusal,
                "kind {} must have a booked door mapping",
                kernel_refusal_kind_name(kind)
            );
        }
        // Every landed construct refusal maps to the booked construct case and
        // marshals as the Refused class with the variant tag on the payload.
        for refusal in CONSTRUCT_REFUSALS {
            assert_eq!(
                door_case_of_construct_refusal(refusal),
                DoorCase::ConstructRefused
            );
            let marshaled = Marshaled::from_construct_refusal(refusal);
            assert_eq!(marshaled.class, ExceptionClass::Refused);
            match marshaled.payload {
                MarshaledPayload::Refused(payload) => {
                    assert_eq!(payload.case, "construct_refused");
                    assert_eq!(payload.stage.as_deref(), Some(refusal.tag()));
                }
                MarshaledPayload::Unresolved(_) => {
                    panic!("construct refusals never marshal as Unresolved")
                }
            }
        }
        // A kind the table does not name is the typed UnmappedRefusal defect
        // signal, never silent.
        assert_eq!(
            door_case_of_kernel_refusal_name("a_future_refusal_kind"),
            DoorCase::UnmappedRefusal
        );
        let unmapped = UnmappedRefusal::new("a_future_refusal_kind");
        assert_eq!(unmapped.door_case(), DoorCase::UnmappedRefusal);
        let marshaled = unmapped.marshaled();
        assert_eq!(marshaled.class, ExceptionClass::Refused);
        match marshaled.payload {
            MarshaledPayload::Refused(payload) => assert_eq!(payload.case, "unmapped_refusal"),
            MarshaledPayload::Unresolved(_) => {
                panic!("an unmapped refusal never marshals as Unresolved")
            }
        }
    }

    #[test]
    fn canonical_pair_end_to_end_through_the_export() {
        // The canonical admitted pair runs the full path: facade admission →
        // certified transversality → a certified bracket out.
        let routed = boolean_dispatch(&canonical_row()).expect("canonical pair routed");
        assert_eq!(verdict(&routed), "routed");
        let value: serde_json::Value = serde_json::from_str(&routed).expect("routed json");
        assert_eq!(value["ok"], serde_json::Value::Bool(true));
        assert!(value["transverse"]["lo"].as_f64().expect("lo") > 0.0);

        // The canonical admitted patch runs the certified volume fact.
        let volume = volume_facts(&canonical_volume_row()).expect("canonical volume");
        let value: serde_json::Value = serde_json::from_str(&volume).expect("volume json");
        let lo = value["bracket"]["lo"].as_f64().expect("volume lo");
        let hi = value["bracket"]["hi"].as_f64().expect("volume hi");
        assert!((lo - 1.0 / 3.0).abs() < 1.0e-12, "exact face form is 1/3");
        assert!(lo <= hi);

        // The canonical algebraic-trim row runs the certified trim bracket.
        let trim = trim_facts(&canonical_trim_row()).expect("canonical trim");
        let value: serde_json::Value = serde_json::from_str(&trim).expect("trim json");
        let lo = value["bracket"]["lo"].as_f64().expect("trim lo");
        let hi = value["bracket"]["hi"].as_f64().expect("trim hi");
        assert!(
            lo <= 1.0 && 1.0 <= hi,
            "the constant density integrates to 1"
        );
    }

    #[test]
    fn binding_is_deterministic_bit_identical_rerun() {
        let row = canonical_row();
        let first = boolean_dispatch(&row).expect("first run");
        let second = boolean_dispatch(&row).expect("second run");
        assert_eq!(first, second, "boolean dispatch must be bit-identical");

        let volume = canonical_volume_row();
        assert_eq!(
            volume_facts(&volume).expect("first volume"),
            volume_facts(&volume).expect("second volume"),
            "volume facts must be bit-identical"
        );

        let trim = canonical_trim_row();
        assert_eq!(
            trim_facts(&trim).expect("first trim"),
            trim_facts(&trim).expect("second trim"),
            "trim facts must be bit-identical"
        );
    }
}
