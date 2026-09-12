//! FHC-G5 required suite — surface the swallowed refusals, then classify
//! (`truck123d/tests/swallowed_refusal_diagnosis.rs`, the crate test file
//! booked by the packet).
//!
//! Gap-register category 5: four rows whose blockers were UNIDENTIFIED because
//! a corpus builder swallowed the drop-in refusal. The corpus trees are
//! untouchable, so the door makes the swallowed case visible from its own
//! probe path: every typed refusal a door-side probe catches is recorded, and
//! `_error_record` surfaces it in place of the untyped `RuntimeError` wrapper
//! (`floor._loft_stack`'s `is_valid_shape` -> `False` ->
//! `RuntimeError("floor loft failed: None")`), preserving the wrapper verbatim
//! under `swallowed_wrapper`.
//!
//! Tests:
//!
//! 1. `floor_swallowed_refusal_is_surfaced_typed` — `f1/floor` no longer
//!    produces an untyped `RuntimeError`: the door surfaces the typed
//!    `unsupported_envelope` (the open smooth spline-loft carrier) and keeps
//!    the swallowed wrapper verbatim.
//! 2. `diffuser_swallowed_refusal_is_surfaced_typed` — the same builder shape
//!    for `f1/diffuser`.
//! 3. `mismatched_station_span_loft_refuses_typed_at_the_door_arm` — the
//!    door-side record behind the swallow: a two-station periodic-spline loft
//!    whose sections carry different sample counts refuses TYPED at the
//!    landed EX-A smooth arm (never an untyped failure).
//! 4. `aero_band_composition_names_its_open_extrude_carrier` — `hypercar/aero`
//!    is no longer the diagnosis-pending `case empty`: the band's negative
//!    extrude is a named carrier refusal at `extrude`/`trim_prism`.
//! 5. `powertrain_refusal_names_its_next_carrier` — `hypercar/powertrain`
//!    progresses to a typed `spline_loft*spline_loft` fuse refusal.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// The absolute path of the corpus `ttc` directory.
fn corpus_ttc_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("corpus")
        .join("ttc")
}

/// The absolute path of the corpus door script.
fn door_path() -> PathBuf {
    corpus_ttc_dir().join("door.py")
}

/// One-time preparation of the native-module staging directory (python
/// subprocesses need `truck123d` importable from a real interpreter). Mirrors
/// the sibling integration suites: the crate cdylib is copied as
/// `truck123d.pyd` beside the interpreter runtime DLLs.
fn staged_native_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let exe = std::env::current_exe().expect("test binary path");
        let deps = exe.parent().expect("deps dir");
        let staging = std::env::temp_dir().join(format!("truck123d_fhc_g5_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&staging);
        std::fs::create_dir_all(&staging).expect("staging dir");
        copy_if_present(deps, "truck123d.dll", &staging, "truck123d.pyd");
        let prefix = python_prefix();
        for dll in [
            "python3.dll",
            "python314.dll",
            "vcruntime140.dll",
            "vcruntime140_1.dll",
        ] {
            copy_from_dir(prefix, dll, &staging);
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

/// The staged python environment: PATH is prepended with the staging dir and
/// the python prefix so the pyd's runtime dependencies resolve.
fn python_command() -> Command {
    let staging = staged_native_dir();
    let prefix = python_prefix();
    let mut path_parts = vec![staging.to_path_buf(), prefix.to_path_buf()];
    if let Some(current) = std::env::var_os("PATH") {
        path_parts.extend(std::env::split_paths(&current));
    }
    let path = std::env::join_paths(path_parts).expect("join child PATH");
    let mut command = Command::new("python");
    command
        .env("PYTHONPATH", staging)
        .env("PATH", path)
        .current_dir(env!("CARGO_MANIFEST_DIR"));
    command
}

/// Runs a python `-c` script with the corpus `ttc` dir as `argv[1]`; returns
/// its stdout. Panics on a nonzero exit.
fn run_python(script: &str) -> String {
    let output = python_command()
        .arg("-c")
        .arg(script)
        .arg(corpus_ttc_dir())
        .output()
        .expect("spawn python");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    assert!(
        output.status.success(),
        "python failed (rc {}):\nstdout:{}\nstderr:{}",
        output.status,
        stdout,
        String::from_utf8_lossy(&output.stderr)
    );
    stdout
}

/// Runs the corpus door with `--engine truck` over one tree/module/entry.
fn run_truck_door(tree: &str, module: &str, entry: &str, args_json: &str) -> serde_json::Value {
    let stl = std::env::temp_dir().join(format!("ttc_fhc_g5_{}_{}.stl", std::process::id(), entry));
    let _ = std::fs::remove_file(&stl);
    let output = python_command()
        .arg(door_path())
        .arg("--engine")
        .arg("truck")
        .arg(corpus_ttc_dir().join("trees").join(tree).join("src"))
        .arg(module)
        .arg(entry)
        .arg(args_json)
        .arg(&stl)
        .output()
        .expect("spawn truck door");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let record: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("door json for {entry}: {e}\n{stdout}"));
    let _ = std::fs::remove_file(&stl);
    record
}

/// The common preamble: import the real native module and the corpus door.
const PREAMBLE: &str = r#"
import json, sys
sys.path.insert(0, sys.argv[1])
import door
import truck123d
"#;

/// The typed refusal of a row surfaced through a swallowed untyped wrapper.
fn assert_surfaced_typed_refusal(record: &serde_json::Value, label: &str, wrapper: &str) {
    assert_eq!(record["ok"], false, "{label} must be red: {record}");
    let error = &record["error"];
    assert_eq!(
        error["kind"], "Refused",
        "{label} must surface a typed refusal, not the untyped wrapper: {record}"
    );
    assert_eq!(error["typed"], true, "{label} must be typed: {record}");
    assert_eq!(
        error["refusal_code"], "E_UNSUPPORTED_ENVELOPE",
        "{label} refusal code: {record}"
    );
    assert_eq!(
        error["payload"],
        serde_json::json!({
            "case": "unsupported_envelope",
            "envelope": "non_canonical_carrier",
        }),
        "{label} payload: {record}"
    );
    assert_eq!(
        error["swallowed_wrapper"]["kind"], "RuntimeError",
        "{label} must preserve the swallowed wrapper verbatim: {record}"
    );
    assert!(
        error["swallowed_wrapper"]["message"]
            .as_str()
            .unwrap_or("")
            .contains(wrapper),
        "{label} wrapper message: {record}"
    );
}

// ---------------------------------------------------------------------------
// Test 1: floor's swallowed refusal is surfaced typed.
// ---------------------------------------------------------------------------

#[test]
fn floor_swallowed_refusal_is_surfaced_typed() {
    let record = run_truck_door("f1", "lib.floor", "build_floor", "[]");
    assert_surfaced_typed_refusal(&record, "f1/floor", "floor loft failed");
    // The surfaced refusal names the open smooth loft carrier the door-side
    // probe actually hit.
    assert_eq!(
        record["error"]["carrier"], "spline_loft",
        "f1/floor carrier: {record}"
    );
}

// ---------------------------------------------------------------------------
// Test 2: diffuser's swallowed refusal is surfaced typed.
// ---------------------------------------------------------------------------

#[test]
fn diffuser_swallowed_refusal_is_surfaced_typed() {
    let record = run_truck_door("f1", "lib.floor", "build_diffuser", "[]");
    assert_surfaced_typed_refusal(&record, "f1/diffuser", "floor loft failed");
    assert_eq!(
        record["error"]["carrier"], "spline_loft",
        "f1/diffuser carrier: {record}"
    );
}

// ---------------------------------------------------------------------------
// Test 3: the door-side record — the mismatched-span spline loft refuses typed.
// ---------------------------------------------------------------------------

#[test]
fn mismatched_station_span_loft_refuses_typed_at_the_door_arm() {
    let script = format!(
        r#"{PREAMBLE}
import math
bd = door._build_truck_module()
door._T123D = truck123d

def section(z, n):
    pts = [
        (math.cos(2.0 * math.pi * i / n), math.sin(2.0 * math.pi * i / n), z)
        for i in range(n)
    ]
    return bd.make_face(bd.Spline(*pts, periodic=True))

part = bd.loft([section(0.0, 8), section(10.0, 12)])
try:
    truck123d.bd_facts(json.dumps(part._node()))
    print(json.dumps({{"refused": False}}))
except truck123d.Refused as exc:
    print(json.dumps({{"refused": True, "payload": exc.payload}}))
"#
    );
    let stdout = run_python(&script);
    let record: serde_json::Value = serde_json::from_str(stdout.trim()).expect("arm json");
    assert_eq!(
        record["refused"], true,
        "a mismatched station-span spline loft must refuse typed: {record}"
    );
    assert_eq!(
        record["payload"],
        serde_json::json!({
            "case": "unsupported_envelope",
            "envelope": "non_canonical_carrier",
        }),
        "the arm's refusal payload: {record}"
    );
}

// ---------------------------------------------------------------------------
// Test 4: aero's `case empty` is reclassified to the named open carrier.
// ---------------------------------------------------------------------------

#[test]
fn aero_band_composition_names_its_open_extrude_carrier() {
    let record = run_truck_door("hypercar", "lib.aero", "build", "[]");
    assert_eq!(record["ok"], false, "aero is not green: {record}");
    let error = &record["error"];
    assert_eq!(error["kind"], "Refused", "aero must refuse typed: {record}");
    assert_eq!(error["typed"], true, "aero must refuse typed: {record}");
    assert_eq!(
        error["refusal_code"], "E_UNSUPPORTED_ENVELOPE",
        "aero must name the open carrier, not the diagnosis-pending case empty: {record}"
    );
    assert_eq!(error["verb"], "extrude", "aero verb: {record}");
    assert_eq!(error["carrier"], "trim_prism", "aero carrier: {record}");
    assert_ne!(
        error["payload"]["case"], "empty",
        "the case-empty diagnosis is resolved: {record}"
    );
}

// ---------------------------------------------------------------------------
// Test 5: powertrain's next refusing verb past its landed name-gap.
// ---------------------------------------------------------------------------

#[test]
fn powertrain_refusal_names_its_next_carrier() {
    let record = run_truck_door("hypercar", "lib.powertrain", "build", "[]");
    assert_eq!(record["ok"], false, "powertrain is not green: {record}");
    let error = &record["error"];
    assert_eq!(
        error["kind"], "Refused",
        "powertrain must refuse typed: {record}"
    );
    assert_eq!(
        error["typed"], true,
        "powertrain must refuse typed: {record}"
    );
    assert_eq!(
        error["refusal_code"], "E_UNSUPPORTED_ENVELOPE",
        "powertrain refusal code: {record}"
    );
    assert_eq!(error["verb"], "fuse", "powertrain verb: {record}");
    assert_eq!(
        error["carrier"], "spline_loft*spline_loft",
        "powertrain carrier: {record}"
    );
}
