//! The pyo3 surface of the bridge core: the `truck123d` Python module's
//! marshaling helpers and the opaque kernel-solid handle class.
//!
//! This module is the boundary layer — H-1 permits `?` here — but the
//! GIL-released closures it runs are pure (`crate::gil`), so nothing panics
//! across the FFI.

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict};

use crate::gil::{self, ModulusSpec};
use crate::marshal::{ExceptionClass, Marshaled, RefusedPayload, UnresolvedPayload};
use crate::{Refused, Unresolved};
use truck_base::evidence::{Modulus, ModulusShape, Refusal};

// CG-BINDING: the certified-funnel binding module (the pyo3 translation over
// the stabilized facade). `lib.rs` is outside this packet's write set, so the
// module is declared from its sibling source file here rather than from the
// crate root; the module body and its three exports live in `binding.rs`.
#[path = "binding.rs"]
pub mod binding;

/// Opaque kernel-solid handle (PB-004 scope decision 2): no kernel type is
/// ever a `#[pyclass]`. This class owns the Rust-side value behind a closed
/// door; the concrete kernel `Solid` is injected by PB-006 (the assembly
/// packet) behind the same handle. This packet lands the shape and the
/// `kind` tag so sessions can carry named opaque results today.
#[pyclass(name = "TruckSolid", module = "truck123d")]
pub struct PyTruckSolid {
    kind: String,
}

#[pymethods]
impl PyTruckSolid {
    #[new]
    fn new(kind: &str) -> Self {
        Self {
            kind: kind.to_string(),
        }
    }

    /// The opaque handle's kind tag (e.g. the op that produced it).
    #[getter]
    fn kind(&self) -> &str {
        &self.kind
    }
}

/// Builds a `Refused` exception instance carrying the serde payload of a
/// `RefusedPayload` as `payload` plus flattened attributes. Used by PB-005 to
/// hand a marshaled kernel refusal to Python as a typed exception; nothing
/// degrades to a bare `Exception`.
#[pyfunction]
pub fn marshal_refused(py: Python<'_>, payload_json: &str) -> PyResult<Py<PyAny>> {
    let payload: RefusedPayload = serde_json::from_str(payload_json)
        .map_err(|e| PyValueError::new_err(format!("invalid Refused payload JSON: {e}")))?;
    let message = format!("kernel refusal: {}", payload.case);
    let value = serde_json::to_value(&payload)
        .map_err(|e| PyValueError::new_err(format!("Refused payload not serializable: {e}")))?;
    Ok(exception_instance(py, py.get_type::<Refused>().into_any(), &message, &value)?.unbind())
}

/// Builds an `Unresolved` exception instance carrying the serde payload of an
/// `UnresolvedPayload` (the κ ledger + the witness).
#[pyfunction]
pub fn marshal_unresolved(py: Python<'_>, payload_json: &str) -> PyResult<Py<PyAny>> {
    let payload: UnresolvedPayload = serde_json::from_str(payload_json)
        .map_err(|e| PyValueError::new_err(format!("invalid Unresolved payload JSON: {e}")))?;
    let message = format!("kernel unresolved: {}", payload.witness);
    let value = serde_json::to_value(&payload)
        .map_err(|e| PyValueError::new_err(format!("Unresolved payload not serializable: {e}")))?;
    Ok(exception_instance(py, py.get_type::<Unresolved>().into_any(), &message, &value)?.unbind())
}

/// The canonical kernel call-slot exposed to Python (PB-005 entries will
/// follow the same shape): parse two wire modulus specs, run the landed
/// evidence-composition kernel call with the GIL released, and marshal the
/// outcome. A refusal raises the typed `Refused`/`Unresolved` exception —
/// never a bare `Exception`.
#[pyfunction]
pub fn kernel_evidence_compose(py: Python<'_>, a_json: &str, b_json: &str) -> PyResult<String> {
    let a: ModulusSpec = serde_json::from_str(a_json)
        .map_err(|e| PyValueError::new_err(format!("invalid modulus spec a: {e}")))?;
    let b: ModulusSpec = serde_json::from_str(b_json)
        .map_err(|e| PyValueError::new_err(format!("invalid modulus spec b: {e}")))?;
    let outcome = gil::with_kernel_gil_released(py, move || gil::compose_kernel(a, b));
    match outcome {
        Ok(modulus) => {
            let value = modulus_to_value(&modulus);
            serde_json::to_string(&value)
                .map_err(|e| PyRuntimeError::new_err(format!("result serialization failed: {e}")))
        }
        Err(refusal) => Err(refusal_to_pyerr(py, &refusal)),
    }
}

/// Converts a landed `Refusal` into the typed Python exception, payload and
/// all. The conversion itself cannot fail; every failure mode here falls back
/// to a runtime error carrying the marshaled message (H-1: no unwinding).
fn refusal_to_pyerr(py: Python<'_>, refusal: &Refusal) -> PyErr {
    let marshaled = Marshaled::from_refusal(refusal);
    let payload = serde_json::to_value(&marshaled.payload).unwrap_or(serde_json::Value::Null);
    let class = match marshaled.class {
        ExceptionClass::Refused => py.get_type::<Refused>().into_any(),
        ExceptionClass::Unresolved => py.get_type::<Unresolved>().into_any(),
    };
    match exception_instance(py, class, &marshaled.message, &payload) {
        Ok(instance) => PyErr::from_value(instance),
        Err(e) => e,
    }
}

/// Creates an exception instance of `class`, sets `args` to `message`, and
/// attaches the serde payload: a `payload` attribute holding the full JSON
/// dict plus each top-level non-null field as a flattened attribute. The
/// dict is materialized through `json.loads` of the serde rendering — the
/// witness payload is always built from serde, never pickled internals.
fn exception_instance<'py>(
    py: Python<'py>,
    class: Bound<'py, PyAny>,
    message: &str,
    payload: &serde_json::Value,
) -> PyResult<Bound<'py, PyAny>> {
    let instance = class.call1((message,))?;
    let dict = payload_to_dict(py, payload)?;
    instance.setattr("payload", dict.clone().into_any())?;
    if let Some(map) = payload.as_object() {
        for (key, value) in map {
            if !value.is_null()
                && let Some(item) = dict.get_item(key.as_str())?
            {
                instance.setattr(key.as_str(), item)?;
            }
        }
    }
    Ok(instance)
}

/// Renders a `serde_json::Value` as a Python `dict` via `json.loads`.
fn payload_to_dict<'py>(
    py: Python<'py>,
    payload: &serde_json::Value,
) -> PyResult<Bound<'py, PyDict>> {
    let json_str = serde_json::to_string(payload)
        .map_err(|e| PyRuntimeError::new_err(format!("payload serialization failed: {e}")))?;
    let json_mod = py.import("json")?;
    let loads = json_mod.getattr("loads")?;
    let obj = loads.call1((json_str,))?;
    obj.cast_into::<PyDict>()
        .map_err(|e| PyRuntimeError::new_err(format!("payload is not a dict: {e}")))
}

/// Renders a composed `Modulus` (the landed result) as JSON for the Python
/// caller. Reporting only: the values are read off the kernel result, never
/// recomputed here. Non-finite numbers (a global/unbounded domain) become
/// `null`, since JSON cannot carry them.
fn modulus_to_value(modulus: &Modulus) -> serde_json::Value {
    let shape = match modulus.shape {
        ModulusShape::Lipschitz(k) => {
            serde_json::json!({ "kind": "lipschitz", "k": finite_or_null(k) })
        }
        ModulusShape::Holder { k, exponent } => serde_json::json!({
            "kind": "holder",
            "k": finite_or_null(k),
            "exponent": finite_or_null(exponent),
        }),
        ModulusShape::Pole { k } => serde_json::json!({ "kind": "pole", "k": finite_or_null(k) }),
        ModulusShape::Unbounded => serde_json::json!({ "kind": "unbounded" }),
    };
    serde_json::json!({
        "ok": true,
        "domain": finite_or_null(modulus.domain),
        "shape": shape,
    })
}

fn finite_or_null(value: f64) -> serde_json::Value {
    if value.is_finite() {
        serde_json::Value::from(value)
    } else {
        serde_json::Value::Null
    }
}
