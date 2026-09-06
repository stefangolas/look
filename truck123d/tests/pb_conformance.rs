//! PB-007 required Rust suite (three tests) over the conformance battery:
//! the three showcase facade sessions run in real python processes, byte-equal
//! to the Rust runs of the same tables.
//!
//! This is the gate for the whole bridge program (spec 3, PB-007). The three
//! showcase models (amphora, teapot, waterslide) are each expressed as their
//! canonical facade session: the ordered op table the bridge runs. The table
//! is executed through BOTH regimes and the resulting report JSON must be
//! byte-identical:
//!
//! 1. `python_runs_match_rust_runs_byte_equal` -- for each of the three
//!    showcase tables, the report JSON a fresh python process produces
//!    (facade sugar session -> `facade_submit`) is byte-equal to the report
//!    JSON Rust's `run_facade` produces for the same table, and the python
//!    session's table JSON is byte-equal to the Rust table serialization.
//! 2. `refusal_battery_mirrors_typed_refusals` -- each typed refusal the
//!    facade can express (empty export, blend without selection,
//!    NonCanonicalCarrier STEP) raises the mapped python exception
//!    (`Refused`/`Unresolved`) with the same named cause as the Rust-side
//!    marshaled refusal of the same table.
//! 3. `determinism_across_processes` -- the same showcase session run twice in
//!    two fresh python processes produces byte-identical output.
//!
//! Runtime: the battery drives the REAL python facade, so each test spawns
//! `python` subprocesses (like the PB-008 door). The native `truck123d`
//! module is staged into a per-process temp dir: the crate's cdylib (built
//! into the same target-profile deps dir as the test binary) is copied as
//! `truck123d.pyd` beside the interpreter's runtime DLLs (resolved from the
//! `python` prefix and PATH), mirroring pb_004's real-python smoke. The sugar
//! package is loaded from `truck123d/python` under the alias `facade`, exactly
//! as the embedded suites load it.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use truck_base::evidence::{EnvelopeCase, Refusal};
use truck123d::{FacadeOp, FacadeTable, run_facade};

/// The three showcase model tables under conformance (facade-session form).
const SHOWCASE_MODELS: [&str; 3] = ["amphora", "teapot", "waterslide"];

/// The absolute path of the conformance python directory.
fn conformance_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("conformance_py")
}

/// The absolute path of the crate's python sugar package directory.
fn sugar_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("python")
}

/// One-time preparation of the native-module staging directory (python
/// subprocesses need `truck123d` importable from a real interpreter).
fn staged_native_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let exe = std::env::current_exe().expect("test binary path");
        let deps = exe.parent().expect("deps dir");
        let staging = std::env::temp_dir().join(format!("truck123d_cfm_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&staging);
        std::fs::create_dir_all(&staging).expect("staging dir");
        // The crate's cdylib is built into the same deps dir as the test
        // binary; python needs it as `truck123d.pyd` with the interpreter DLL
        // resolvable beside it.
        copy_if_present(deps, "truck123d.dll", &staging, "truck123d.pyd");
        // Co-locate the runtime DLLs the pyd depends on (python runtime plus
        // the GNU toolchain's libunwind). Resolve the python prefix and the
        // libunwind location from the environment, like the embedded suites
        // bootstrap PYTHONHOME from `python`.
        let python_prefix = python_prefix();
        for dll in [
            "python3.dll",
            "python314.dll",
            "vcruntime140.dll",
            "vcruntime140_1.dll",
        ] {
            copy_from_dir(python_prefix, dll, &staging);
        }
        if let Some(libunwind) = find_on_path("libunwind.dll") {
            let _ = std::fs::copy(libunwind, staging.join("libunwind.dll"));
        }
        staging
    })
}

/// The `python` interpreter prefix (mirrors the embedded suites' bootstrap).
fn python_prefix() -> &'static Path {
    static PREFIX: OnceLock<PathBuf> = OnceLock::new();
    PREFIX.get_or_init(|| {
        if let Ok(home) = std::env::var("PYTHONHOME")
            && !home.is_empty()
        {
            return PathBuf::from(home);
        }
        let output = Command::new("python")
            .args(["-c", "import sys; print(sys.prefix)"])
            .output();
        if let Ok(output) = output
            && output.status.success()
        {
            let home = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !home.is_empty() {
                return PathBuf::from(home);
            }
        }
        // Last resort: the interpreter that spawned this test process's deps dir.
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .unwrap_or_default()
    })
}

/// Searches PATH for `name`, returning the first match's path.
fn find_on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn copy_if_present(from: &Path, name: &str, to: &Path, as_name: &str) {
    let src = from.join(name);
    if src.is_file() {
        let _ = std::fs::copy(&src, to.join(as_name));
    }
}

fn copy_from_dir(from: &Path, name: &str, to: &Path) {
    copy_if_present(from, name, to, name);
}

/// Runs `python <script> <args...>` with the staged native module and the
/// sugar dir wired, returning stdout bytes. The child's PATH is prepended with
/// the staging dir (co-located DLLs) and the python prefix so the pyd import's
/// runtime dependencies resolve.
fn run_python(script: &str, args: &[&str]) -> Vec<u8> {
    let script_path = conformance_dir().join(script);
    let staging = staged_native_dir();
    let prefix = python_prefix();
    let mut path_parts = vec![staging.to_path_buf(), prefix.to_path_buf()];
    if let Some(current) = std::env::var_os("PATH") {
        path_parts.extend(std::env::split_paths(&current));
    }
    let path = std::env::join_paths(path_parts).expect("join child PATH");
    let output = Command::new("python")
        .arg(&script_path)
        .args(args)
        .env("T123D_NATIVE_DIR", staging)
        .env("T123D_SUGAR_DIR", sugar_dir())
        .env("PYTHONPATH", staging)
        .env("PATH", path)
        .output()
        .expect("spawn conformance python");
    if !output.status.success() {
        panic!(
            "conformance python {script} failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    output.stdout
}

/// The canonical facade table for one showcase model, mirrored from the
/// conformance python sessions (same ordered rows, same numbers).
fn showcase_table(model: &str) -> FacadeTable {
    let ops = match model {
        "amphora" => vec![
            FacadeOp::PushPart,
            FacadeOp::Cylinder {
                radius: 0.26,
                height: 0.06,
            },
            FacadeOp::ExportStl {
                path: "amphora_foot.stl".to_string(),
            },
            FacadeOp::Pop,
        ],
        "teapot" => vec![
            FacadeOp::PushPart,
            FacadeOp::Cylinder {
                radius: 2.5,
                height: 4.0,
            },
            FacadeOp::ExportStl {
                path: "teapot_body.stl".to_string(),
            },
            FacadeOp::Pop,
        ],
        "waterslide" => vec![
            FacadeOp::PushPart,
            FacadeOp::Cylinder {
                radius: 3.0,
                height: 1.45,
            },
            FacadeOp::ExportStl {
                path: "waterslide_pool.stl".to_string(),
            },
            FacadeOp::Pop,
        ],
        _ => panic!("unknown showcase {model}"),
    };
    FacadeTable { ops }
}

/// Parses the first stdout line (the python session's submitted table JSON).
fn first_line(bytes: &[u8]) -> &str {
    let text = std::str::from_utf8(bytes).expect("utf8 stdout");
    text.lines().next().expect("at least one line")
}

/// Parses the second stdout line (the python facade report JSON).
fn second_line(bytes: &[u8]) -> &str {
    let text = std::str::from_utf8(bytes).expect("utf8 stdout");
    text.lines().nth(1).expect("at least two lines")
}

// ---------------------------------------------------------------------------
// Test 1: python runs match Rust runs byte for byte.
// ---------------------------------------------------------------------------

#[test]
fn python_runs_match_rust_runs_byte_equal() {
    for model in SHOWCASE_MODELS {
        let table = showcase_table(model);
        // Rust regime: serialize the table and run the native facade.
        let rust_table_json = serde_json::to_string(&table).expect("rust table serializes");
        let report = run_facade(&table).expect("showcase table is in envelope");
        let rust_report_json = serde_json::to_string(&report).expect("rust report serializes");

        // Python regime: a fresh python process builds the same session,
        // renders its table JSON and submits it.
        let out = run_python("showcase_sessions.py", &[model]);

        assert_eq!(
            first_line(&out),
            rust_table_json,
            "{model}: python session table JSON is not byte-equal to the Rust table"
        );
        assert_eq!(
            second_line(&out),
            rust_report_json,
            "{model}: python facade report JSON is not byte-equal to the Rust run"
        );
    }
}

// ---------------------------------------------------------------------------
// Test 2: the typed-refusal battery mirrors as python raises.
// ---------------------------------------------------------------------------

/// One typed-refusal fixture: a submitted table that the facade refuses, plus
/// the Rust-side expectation (the marshaled class tag and named cause).
fn refusal_fixtures() -> Vec<(&'static str, FacadeTable, Refusal)> {
    // (name, table, expected refusal)
    vec![
        (
            "empty_export",
            FacadeTable {
                ops: vec![
                    FacadeOp::PushPart,
                    FacadeOp::ExportStl {
                        path: "empty.stl".to_string(),
                    },
                ],
            },
            Refusal::Empty,
        ),
        (
            "blend_without_selection",
            FacadeTable {
                ops: vec![
                    FacadeOp::PushPart,
                    FacadeOp::Box {
                        length: 2.0,
                        width: 3.0,
                        height: 1.0,
                    },
                    FacadeOp::Fillet { radius: 0.1 },
                ],
            },
            Refusal::Empty,
        ),
        (
            "step_of_constructive",
            FacadeTable {
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
            },
            Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier),
        ),
    ]
}

#[test]
fn refusal_battery_mirrors_typed_refusals() {
    for (name, table, expected) in refusal_fixtures() {
        // Rust side: run_facade refuses typed; marshal the named cause.
        let rust_refusal =
            run_facade(&table).expect_err("the fixture table must refuse on the Rust side");
        assert!(
            std::mem::discriminant(&rust_refusal) == std::mem::discriminant(&expected),
            "{name}: Rust refusal variant differs from the fixture"
        );

        // Python side: the same table submitted through the facade raises the
        // mapped exception. The door prints the named cause the exception
        // carries.
        let table_json = serde_json::to_string(&table).expect("table serializes");
        let out = run_python("refusal_battery.py", &[&table_json]);
        let line = second_line(&out);
        let record: serde_json::Value = serde_json::from_str(line).expect("refusal door json");

        // The named cause on the python side must be the marshaled Rust cause.
        let (case_name, envelope_name) = marshaled_cause(&expected);
        let py_class = record["class"].as_str().expect("class tag");
        assert_eq!(
            py_class,
            class_name_for(&expected),
            "{name}: python raised the wrong exception class"
        );
        let py_case = record["case"].as_str().unwrap_or("");
        assert_eq!(
            py_case, case_name,
            "{name}: python named cause differs from the marshaled Rust cause"
        );
        if let Some(envelope) = envelope_name {
            assert_eq!(
                record["envelope"].as_str().unwrap_or(""),
                envelope,
                "{name}: envelope differs"
            );
        }
    }
}

/// The marshaled snake_case cause of a refusal (pb_004 vocabulary).
fn marshaled_cause(refusal: &Refusal) -> (&'static str, Option<&'static str>) {
    match refusal {
        Refusal::Empty => ("empty", None),
        Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier) => {
            ("unsupported_envelope", Some("non_canonical_carrier"))
        }
        _ => panic!("fixture refusal not marshaled in this battery"),
    }
}

/// The python exception class name the facade maps the refusal to.
fn class_name_for(refusal: &Refusal) -> &'static str {
    match refusal {
        Refusal::Empty | Refusal::UnsupportedEnvelope(_) => "Refused",
        _ => panic!("fixture refusal class not mapped"),
    }
}

// ---------------------------------------------------------------------------
// Test 3: determinism across two fresh processes.
// ---------------------------------------------------------------------------

#[test]
fn determinism_across_processes() {
    // The battery IS the determinism gate: two fresh python processes running
    // the same showcase session must produce byte-identical output.
    for model in SHOWCASE_MODELS {
        let first = run_python("showcase_sessions.py", &[model]);
        let second = run_python("showcase_sessions.py", &[model]);
        assert_eq!(
            first, second,
            "{model}: two fresh python processes produced different bytes"
        );
    }
}
