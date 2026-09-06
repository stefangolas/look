//! PB-005 required Rust suite (four tests) over the build123d-shaped Python
//! facade.
//!
//! Tests 1â€“3 embed the CPython interpreter (pyo3's `auto-initialize` dev
//! feature) and drive the real Python sugar package under
//! `truck123d/python/truck123d/`; test 4 is pure Rust against the single
//! native facade entry. Unlike `test_bridge.rs` (which registers the module
//! through `append_to_inittab` before interpreter init), this suite registers
//! the native `truck123d` module post-init by building it with
//! `PyModule::new` + the `pub` module-init fn and inserting it into
//! `sys.modules`, which is safe with several tests sharing one interpreter.
//!
//! All interpreter access is serialized by one mutex; the PYTHONHOME
//! bootstrap runs once before the first interpreter attach.

use std::ffi::CString;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, Once};

use pyo3::prelude::*;
use pyo3::types::PyDict;
use truck_base::evidence::{EnvelopeCase, Refusal};
use truck123d::{AxisValue, FacadeOp, FacadeTable, run_facade};

/// Serializes every embedded-interpreter block: one Python operation at a
/// time across the four tests sharing one process interpreter.
static PY_LOCK: Mutex<()> = Mutex::new(());

/// Whether the native `truck123d` module is registered in `sys.modules`.
static NATIVE_REGISTERED: AtomicBool = AtomicBool::new(false);

/// Whether the sugar package is loaded as the `facade` module.
static SUGAR_REGISTERED: AtomicBool = AtomicBool::new(false);

/// Whether PYTHONHOME was bootstrapped from the `python` prefix.
static BOOTSTRAPPED: Once = Once::new();

fn bootstrap_python_home() {
    BOOTSTRAPPED.call_once(|| {
        if std::env::var_os("PYTHONHOME").is_none() {
            let probe = std::process::Command::new("python")
                .args(["-c", "import sys; print(sys.prefix)"])
                .output();
            if let Ok(output) = probe
                && output.status.success()
            {
                let home = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !home.is_empty() {
                    // SAFETY: single-threaded bootstrap point, before any
                    // interpreter attach; no other env access is racing.
                    unsafe { std::env::set_var("PYTHONHOME", &home) };
                }
            }
        }
    });
}

/// Registers the native `truck123d` module in `sys.modules` once. Post-init
/// registration avoids the `append_to_inittab` "interpreter already running"
/// panic that several python tests in one binary would trip.
fn register_native_module(py: Python<'_>) -> PyResult<()> {
    if NATIVE_REGISTERED.load(Ordering::Relaxed) {
        return Ok(());
    }
    let module = PyModule::new(py, "truck123d")?;
    truck123d::truck123d(&module)?;
    let modules = py.import("sys")?.getattr("modules")?;
    modules.set_item("truck123d", module)?;
    NATIVE_REGISTERED.store(true, Ordering::Relaxed);
    Ok(())
}

/// Loads the pure-Python sugar package (`truck123d/python/truck123d`) as the
/// module `facade` once. Its internal `import truck123d` resolves to the
/// native module registered above.
fn register_sugar_module(py: Python<'_>) -> PyResult<()> {
    if SUGAR_REGISTERED.load(Ordering::Relaxed) {
        return Ok(());
    }
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/python/truck123d/__init__.py");
    let text = std::fs::read_to_string(path).map_err(|e| {
        pyo3::exceptions::PyRuntimeError::new_err(format!("cannot read {path}: {e}"))
    })?;
    let code = CString::new(text)
        .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("sugar source contains NUL"))?;
    PyModule::from_code(py, &code, c"truck123d/python/__init__.py", c"facade")?;
    SUGAR_REGISTERED.store(true, Ordering::Relaxed);
    Ok(())
}

/// Runs `scenario` (which must set the global `OUT` to a `str`) under the
/// embedded interpreter and returns `OUT`.
fn run_scenario(scenario: &str) -> String {
    let _guard = PY_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    bootstrap_python_home();
    Python::attach(|py| {
        if let Err(e) = register_native_module(py).and_then(|()| register_sugar_module(py)) {
            e.print(py);
            panic!("facade module registration failed");
        }
        let globals = PyDict::new(py);
        let code = CString::new(scenario).expect("scenario contains no NUL byte");
        py.run(&code, Some(&globals), None).unwrap_or_else(|e| {
            e.print(py);
            panic!("embedded python scenario failed");
        });
        let out = globals
            .get_item("OUT")
            .expect("scenario must bind OUT")
            .expect("OUT is missing");
        out.extract::<String>().expect("OUT must be a str")
    })
}

#[test]
fn facade_round_trips_through_tables() {
    // The direct Rust call: the same BuildPart-shaped session as a table.
    let expected = FacadeTable {
        ops: vec![
            FacadeOp::PushPart,
            FacadeOp::PushSketch,
            FacadeOp::Polygon {
                points: vec![[0.0, 0.0], [2.0, 0.0], [2.0, 1.0], [0.0, 1.0]],
            },
            FacadeOp::Pop,
            FacadeOp::Extrude { amount: 1.0 },
            FacadeOp::SelectEdges,
            FacadeOp::SelectFilter { axis: AxisValue::Z },
            FacadeOp::Fillet { radius: 0.05 },
            FacadeOp::Pop,
        ],
    };
    let expected_json = serde_json::to_string(&expected).expect("expected table serializes");

    let scenario = r#"
import facade
with facade.BuildPart() as part:
    with facade.BuildSketch():
        facade.Polygon((0, 0), (2, 0), (2, 1), (0, 1))
    facade.extrude(amount=1)
    facade.fillet(part.edges().filter_by("z"), radius=0.05)
OUT = part.to_table_json()
"#;
    let got = run_scenario(scenario);
    assert_eq!(
        got, expected_json,
        "the Python session's table is not byte-equal to the direct Rust call"
    );

    // The submitted table is in-envelope: the facade run certifies it.
    let report = run_facade(&expected).expect("primitive + extrude + fillet is in envelope");
    assert_eq!(report.op_count, 9);
    assert!(!report.constructive);
    assert!(report.exports.is_empty());
}

#[test]
fn builder_mode_is_python_side_only() {
    let scenario = r#"
import facade, truck123d

calls = []
real = truck123d.facade_submit
def counting(table_json):
    calls.append(table_json)
    return real(table_json)
truck123d.facade_submit = counting

with facade.BuildPart() as part:
    facade.Box(2.0, 3.0, 4.0)

success_submitted = part.submitted and len(calls) == 1

raised = False
bad = None
try:
    with facade.BuildPart() as bad:
        facade.Cylinder(radius=1.0, height=2.0)
        raise RuntimeError("boom")
except RuntimeError:
    raised = True

truck123d.facade_submit = real

# A mid-build exception submits nothing: exactly one native submit happened
# (the success path), and the aborted session never crossed the boundary.
ok = (
    success_submitted
    and raised
    and len(calls) == 1
    and bad is not None
    and bad.submitted is False
    and len(bad._ops) >= 2
)
OUT = "ok" if ok else (
    "failed: success_submitted=%r raised=%r calls=%d bad.submitted=%r partial_rows=%d"
    % (success_submitted, raised, len(calls),
       None if bad is None else bad.submitted,
       0 if bad is None else len(bad._ops))
)
"#;
    let out = run_scenario(scenario);
    assert_eq!(out, "ok", "builder-mode statefulness is Python-side only");
}

#[test]
fn facade_selectors_fluent() {
    // The documented four-expression selector vocabulary: faces, edges,
    // filter_by, take. The session records a faces+filter selection and an
    // edges+take selection, then fillets the edges of a trailing
    // edges+filter+take selection.
    let expected = FacadeTable {
        ops: vec![
            FacadeOp::PushPart,
            FacadeOp::Box {
                length: 2.0,
                width: 3.0,
                height: 1.0,
            },
            FacadeOp::SelectFaces,
            FacadeOp::SelectFilter { axis: AxisValue::Z },
            FacadeOp::SelectEdges,
            FacadeOp::SelectTake { count: 4 },
            FacadeOp::SelectEdges,
            FacadeOp::SelectFilter { axis: AxisValue::X },
            FacadeOp::SelectTake { count: 2 },
            FacadeOp::Fillet { radius: 0.1 },
            FacadeOp::Pop,
        ],
    };
    let expected_json = serde_json::to_string(&expected).expect("expected table serializes");

    let scenario = r#"
import facade
with facade.BuildPart() as part:
    facade.Box(2.0, 3.0, 1.0)
    part.faces().filter_by("z")
    part.edges().take(4)
    facade.fillet(part.edges().filter_by("x").take(2), radius=0.1)
OUT = part.to_table_json()
"#;
    let got = run_scenario(scenario);
    assert_eq!(
        got, expected_json,
        "the fluent selector vocabulary must produce exactly the documented rows"
    );

    let report = run_facade(&expected).expect("selector rows stay in envelope");
    assert_eq!(report.op_count, 11);
    assert!(!report.constructive);
    assert!(report.exports.is_empty());
}

#[test]
fn export_stl_and_step_entries() {
    // Prismatic fixture: STL and STEP exports are both in envelope.
    let prismatic_stl = FacadeTable {
        ops: vec![
            FacadeOp::PushPart,
            FacadeOp::Box {
                length: 2.0,
                width: 3.0,
                height: 1.0,
            },
            FacadeOp::ExportStl {
                path: "prismatic.stl".to_string(),
            },
        ],
    };
    let report = run_facade(&prismatic_stl).expect("STL export of a prismatic part is in envelope");
    assert!(!report.constructive);
    assert_eq!(report.exports.len(), 1);
    assert_eq!(report.exports[0].op, "export_stl");
    assert_eq!(report.exports[0].path, "prismatic.stl");

    let prismatic_step = FacadeTable {
        ops: vec![
            FacadeOp::PushPart,
            FacadeOp::Box {
                length: 2.0,
                width: 3.0,
                height: 1.0,
            },
            FacadeOp::ExportStep {
                path: "prismatic.step".to_string(),
            },
        ],
    };
    let report =
        run_facade(&prismatic_step).expect("STEP export of a prismatic part is in envelope");
    assert!(!report.constructive);
    assert_eq!(report.exports.len(), 1);
    assert_eq!(report.exports[0].op, "export_step");
    assert_eq!(report.exports[0].path, "prismatic.step");

    // Swept fixture (spline section lofted): STL is in envelope...
    let swept_stl = FacadeTable {
        ops: vec![
            FacadeOp::PushPart,
            FacadeOp::Spline {
                points: vec![[0.0, 0.0, 0.0], [1.0, 2.0, 0.0], [0.0, 0.0, 3.0]],
                periodic: false,
            },
            FacadeOp::Loft,
            FacadeOp::ExportStl {
                path: "swept.stl".to_string(),
            },
        ],
    };
    let report = run_facade(&swept_stl).expect("STL export of a swept part is in envelope");
    assert!(report.constructive);
    assert_eq!(report.exports.len(), 1);
    assert_eq!(report.exports[0].op, "export_stl");
    assert_eq!(report.exports[0].path, "swept.stl");

    // ...but STEP out of a swept part refuses typed (TR-NRB-001: STL only).
    let swept_step = FacadeTable {
        ops: vec![
            FacadeOp::PushPart,
            FacadeOp::Spline {
                points: vec![[0.0, 0.0, 0.0], [1.0, 2.0, 0.0], [0.0, 0.0, 3.0]],
                periodic: false,
            },
            FacadeOp::Loft,
            FacadeOp::ExportStep {
                path: "swept.step".to_string(),
            },
        ],
    };
    let refusal = run_facade(&swept_step).expect_err("STEP of a swept part must refuse typed");
    assert!(
        matches!(
            refusal,
            Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)
        ),
        "expected the TR-NRB-001 NonCanonicalCarrier refusal, got {refusal:?}"
    );

    // Exporting nothing is an empty domain.
    let empty_export = FacadeTable {
        ops: vec![
            FacadeOp::PushPart,
            FacadeOp::ExportStep {
                path: "empty.step".to_string(),
            },
        ],
    };
    let refusal = run_facade(&empty_export).expect_err("exporting no solid must refuse");
    assert!(matches!(refusal, Refusal::Empty));

    // A blend row with no recorded selection is an empty edge domain.
    let blend_without_selection = FacadeTable {
        ops: vec![
            FacadeOp::PushPart,
            FacadeOp::Box {
                length: 2.0,
                width: 3.0,
                height: 1.0,
            },
            FacadeOp::Fillet { radius: 0.1 },
        ],
    };
    let refusal = run_facade(&blend_without_selection)
        .expect_err("a blend without a recorded selection must refuse");
    assert!(matches!(refusal, Refusal::Empty));
}
