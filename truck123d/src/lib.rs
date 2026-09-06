//! truck123d — the pyo3 core of the Truck123d Python Bridge (work packet
//! PB-004-PYO3-CORE, extended by PB-005-PYTHON-FACADE).
//!
//! Scope (see `docs/TRUCK123D_PY_BRIDGE_SPEC.md` §5 and
//! `docs/PY_BRIDGE_CONTRACT.md`): module init, the two-class `Refusal` →
//! exception mapping (`Refused` / `Unresolved`, never a bare `Exception`),
//! serde round-trip of the v1 showcase tables, the GIL policy, and the
//! build123d-shaped Python facade (PB-005). Zero geometric content: the one
//! exercised kernel call (the evidence algebra's `Modulus::compose`) is a
//! landed `truck-base` entry; nothing geometric is computed here. The facade
//! module is a pure table layer — the Python side edits a session table and
//! submits once through the single native entry `run_facade`. No kernel
//! type crosses the boundary except behind opaque handles (`PyTruckSolid`);
//! no `#[pyclass]` sits on a kernel type.
//!
//! GIL policy: every kernel call runs with the GIL released through the
//! single choke point `gil::with_kernel_gil_released` (`Python::detach`).
//! The module is importable as `import truck123d` once built with maturin
//! (see `README.md`).

#![doc = include_str!("../README.md")]

mod exceptions;
mod facade;
mod gil;
mod marshal;
mod python;
mod tables;

#[cfg(test)]
mod test_bridge;

pub use exceptions::{Refused, TruckError, Unresolved};
pub use facade::{AxisValue, FacadeOp, FacadeReport, FacadeTable, ModeValue, run_facade};
pub use gil::ModulusSpec;
pub use marshal::{
    ExceptionClass, Ledger, Marshaled, MarshaledPayload, RefusedPayload, UnresolvedPayload,
    envelope_case_name, ledger_from_budget, prop_name, truth_name, witness_name,
};
pub use python::PyTruckSolid;
pub use tables::{AmphoraTable, TeapotTable, WaterslideTable};

use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::wrap_pyfunction;

/// The `truck123d` Python module: registers the typed exception hierarchy,
/// the opaque handle class, the marshaling helpers, the kernel call-slot, and
/// the facade submit entry (PB-005).
///
/// `pub` so the embedded-interpreter integration suite (`tests/pb_facade.rs`)
/// can register the module under its name in `sys.modules`.
#[pymodule]
pub fn truck123d(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyTruckSolid>()?;
    m.add("TruckError", m.py().get_type::<TruckError>())?;
    m.add("Refused", m.py().get_type::<Refused>())?;
    m.add("Unresolved", m.py().get_type::<Unresolved>())?;
    m.add_function(wrap_pyfunction!(python::marshal_refused, m)?)?;
    m.add_function(wrap_pyfunction!(python::marshal_unresolved, m)?)?;
    m.add_function(wrap_pyfunction!(python::kernel_evidence_compose, m)?)?;
    m.add_function(wrap_pyfunction!(facade::facade_submit, m)?)?;
    Ok(())
}
