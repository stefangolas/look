//! FHC-G7 required suite — the self-describing `ttc_door_run.v2` refusal
//! record and its stable refusal codes.
//!
//! The door's refusal records carried their semantics in prose (`message`)
//! with only a two-field payload. This suite pins the enriched record: the
//! stable `refusal_code`, the mechanical `typed` marker, the
//! verb/carrier/phase metadata, the `client_site` block, and the
//! hand-maintained `known_gap` lookup — while `kind`/`message`/`payload` keep
//! their frozen v1 shapes and literals.
//!
//! Tests:
//!
//! 1. `v2_record_shape_for_a_typed_refusal` — a typed refusal carries every
//!    v2 field and the code from the table.
//! 2. `untyped_die_off_emits_typed_false` — an `AttributeError` reaching the
//!    record emits `typed: false` and no `refusal_code` (a register gap).
//! 3. `known_gap_present_only_for_registered_combinations` — a registered
//!    `(code, verb, carrier-prefix)` yields the block; an unknown one omits
//!    it.
//! 4. `v1_literals_are_preserved` — `kind`/`message`/`payload` are
//!    byte-identical to the v1 literals.
//! 5. `census_row_refusals_marshal_with_their_codes` — the three census-row
//!    spot-checks (airbox, brakes, corner_fl) marshal with their codes.

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

/// One-time preparation of the native-module staging directory (python
/// subprocesses need `truck123d` importable from a real interpreter). Mirrors
/// the sibling integration suites: the crate cdylib is copied as
/// `truck123d.pyd` beside the interpreter runtime DLLs.
fn staged_native_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let exe = std::env::current_exe().expect("test binary path");
        let deps = exe.parent().expect("deps dir");
        let staging = std::env::temp_dir().join(format!("truck123d_fhc_g7_{}", std::process::id()));
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

/// Runs a python `-c` script with the given positional arguments; returns its
/// stdout. Panics on a nonzero exit.
fn run_python(script: &str, args: &[&str]) -> String {
    let output = python_command()
        .arg("-c")
        .arg(script)
        .args(args)
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

/// Runs the corpus door with `--engine truck` over one family tree; returns
/// the parsed JSON record (ok true or a typed refusal).
fn run_truck_door(family: &str, module: &str, entry: &str, args_json: &str) -> serde_json::Value {
    let stl = std::env::temp_dir().join(format!("ttc_fhc_g7_{}_{}.stl", std::process::id(), entry));
    let _ = std::fs::remove_file(&stl);
    let output = python_command()
        .arg(corpus_ttc_dir().join("door.py"))
        .arg("--engine")
        .arg("truck")
        .arg(corpus_ttc_dir().join("trees").join(family).join("src"))
        .arg(module)
        .arg(entry)
        .arg(args_json)
        .arg(&stl)
        .output()
        .expect("spawn truck door");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let _ = std::fs::remove_file(&stl);
    serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("door json for {module}.{entry}: {e}\n{stdout}"))
}

/// The shared preamble: make `door` importable and expose `truck123d`.
const PREAMBLE: &str = r#"
import json, sys
sys.path.insert(0, sys.argv[1])
import door
import truck123d

def typed_refusal(payload, **meta):
    exc = truck123d.Refused("kernel refusal: unsupported_envelope")
    exc.payload = payload
    for key, value in meta.items():
        setattr(exc, key, value)
    return exc
"#;

// ---------------------------------------------------------------------------
// Test 1: the v2 shape of a typed refusal.
// ---------------------------------------------------------------------------

#[test]
fn v2_record_shape_for_a_typed_refusal() {
    let script = format!(
        r#"{PREAMBLE}
exc = typed_refusal(
    {{"case": "unsupported_envelope", "envelope": "non_canonical_carrier"}},
    refusal_code="E_UNSUPPORTED_ENVELOPE",
    typed=True,
    verb="fuse",
    carrier="spline_loft*spline_loft",
    phase="admission",
    client_site_via="surfaces.loft_solid",
)
record = door._error_record(exc, module="lib.suspension", entry="build")
print(json.dumps(record))
"#
    );
    let stdout = run_python(&script, &[corpus_ttc_dir().to_str().expect("corpus path")]);
    let error: serde_json::Value = serde_json::from_str(stdout.trim()).expect("v2 record json");

    assert_eq!(error["kind"], "Refused");
    assert_eq!(error["refusal_code"], "E_UNSUPPORTED_ENVELOPE");
    assert_eq!(error["typed"], true);
    assert_eq!(error["verb"], "fuse");
    assert_eq!(error["carrier"], "spline_loft*spline_loft");
    assert_eq!(error["phase"], "admission");
    assert_eq!(
        error["client_site"],
        serde_json::json!({"module": "lib.suspension", "via": "surfaces.loft_solid"})
    );
    assert_eq!(
        error["known_gap"]["register"],
        "docs/F1_HYPERCAR_GAP_REGISTER.md"
    );
    assert_eq!(error["known_gap"]["section"], "1");
    assert!(
        error["known_gap"]["hint"]
            .as_str()
            .is_some_and(|hint| hint.contains("rational-patch flux")),
        "the registered hint must be present: {error}"
    );
}

// ---------------------------------------------------------------------------
// Test 2: an untyped die-off emits typed: false.
// ---------------------------------------------------------------------------

#[test]
fn untyped_die_off_emits_typed_false() {
    let script = format!(
        r#"{PREAMBLE}
record = door._error_record(
    AttributeError("'Compound' object has no attribute 'moved'"),
    module="lib.falcon_common",
    entry="build_vehicle",
)
print(json.dumps(record))
"#
    );
    let stdout = run_python(&script, &[corpus_ttc_dir().to_str().expect("corpus path")]);
    let error: serde_json::Value = serde_json::from_str(stdout.trim()).expect("untyped json");

    assert_eq!(error["kind"], "AttributeError");
    assert_eq!(error["typed"], false);
    assert!(
        error.get("refusal_code").is_none(),
        "an untyped failure has no machine code: {error}"
    );
    assert!(
        error.get("known_gap").is_none(),
        "an untyped failure is not a known-gap lookup: {error}"
    );
    assert!(
        error["message"]
            .as_str()
            .is_some_and(|m| m.contains("has no attribute")),
        "the untyped message is preserved: {error}"
    );
}

// ---------------------------------------------------------------------------
// Test 3: known_gap is present only for registered combinations.
// ---------------------------------------------------------------------------

#[test]
fn known_gap_present_only_for_registered_combinations() {
    let script = format!(
        r#"{PREAMBLE}
registered = door._error_record(typed_refusal(
    {{"case": "unsupported_envelope", "envelope": "non_canonical_carrier"}},
    refusal_code="E_UNSUPPORTED_ENVELOPE",
    typed=True,
    verb="fuse",
    carrier="spline_loft*spline_loft",
    phase="admission",
), module="lib.suspension", entry="build")
unknown = door._error_record(typed_refusal(
    {{"case": "unsupported_envelope", "envelope": "non_canonical_carrier"}},
    refusal_code="E_UNSUPPORTED_ENVELOPE",
    typed=True,
    verb="fuse",
    carrier="mystery_carrier",
    phase="admission",
), module="lib.suspension", entry="build")
print(json.dumps({{"registered": registered, "unknown": unknown}}))
"#
    );
    let stdout = run_python(&script, &[corpus_ttc_dir().to_str().expect("corpus path")]);
    let record: serde_json::Value = serde_json::from_str(stdout.trim()).expect("known-gap json");

    assert_eq!(
        record["registered"]["known_gap"]["register"],
        "docs/F1_HYPERCAR_GAP_REGISTER.md"
    );
    assert!(
        record["unknown"].get("known_gap").is_none(),
        "an unknown combination must omit the block: {}",
        record["unknown"]
    );
}

// ---------------------------------------------------------------------------
// Test 4: the v1 literals are byte-identical.
// ---------------------------------------------------------------------------

#[test]
fn v1_literals_are_preserved() {
    let script = format!(
        r#"{PREAMBLE}
exc = typed_refusal(
    {{"case": "unsupported_envelope", "envelope": "non_canonical_carrier"}},
    refusal_code="E_UNSUPPORTED_ENVELOPE",
    typed=True,
)
record = door._error_record(exc, module="lib.suspension", entry="build")
print(json.dumps(record))
"#
    );
    let stdout = run_python(&script, &[corpus_ttc_dir().to_str().expect("corpus path")]);
    let error: serde_json::Value = serde_json::from_str(stdout.trim()).expect("v1 json");

    assert_eq!(error["kind"], "Refused");
    assert_eq!(error["message"], "kernel refusal: unsupported_envelope");
    assert_eq!(
        error["payload"],
        serde_json::json!({
            "case": "unsupported_envelope",
            "envelope": "non_canonical_carrier",
        })
    );
}

// ---------------------------------------------------------------------------
// Test 5: the three census-row spot-checks marshal with their codes.
// ---------------------------------------------------------------------------

#[test]
fn census_row_refusals_marshal_with_their_codes() {
    let airbox = run_truck_door("f1", "lib.engine_cover", "build_airbox", "[]");
    assert_eq!(airbox["schema"], "ttc_door_run.v2", "airbox: {airbox}");
    assert_eq!(airbox["ok"], false, "airbox is still red: {airbox}");
    assert_eq!(airbox["error"]["kind"], "Refused", "airbox: {airbox}");
    assert_eq!(
        airbox["error"]["refusal_code"], "E_UNSUPPORTED_ENVELOPE",
        "airbox: {airbox}"
    );
    assert_eq!(airbox["error"]["typed"], true, "airbox: {airbox}");
    assert_eq!(
        airbox["error"]["payload"],
        serde_json::json!({
            "case": "unsupported_envelope",
            "envelope": "non_canonical_carrier",
        }),
        "airbox payload: {airbox}"
    );

    let brakes = run_truck_door("hypercar", "lib.brakes", "build", "[]");
    assert_eq!(brakes["schema"], "ttc_door_run.v2", "brakes: {brakes}");
    assert_eq!(brakes["error"]["kind"], "Refused", "brakes: {brakes}");
    assert_eq!(
        brakes["error"]["refusal_code"], "E_UNSUPPORTED_ENVELOPE",
        "brakes: {brakes}"
    );
    assert_eq!(brakes["error"]["verb"], "cut", "brakes: {brakes}");
    assert_eq!(
        brakes["error"]["carrier"], "face_boolean",
        "brakes: {brakes}"
    );
    assert_eq!(brakes["error"]["phase"], "admission", "brakes: {brakes}");

    let corner = run_truck_door("f1", "lib.wheels", "build_corner", "[\"fl\"]");
    assert_eq!(corner["schema"], "ttc_door_run.v2", "corner_fl: {corner}");
    assert_eq!(corner["error"]["kind"], "Refused", "corner_fl: {corner}");
    assert_eq!(
        corner["error"]["refusal_code"], "E_UNSUPPORTED_ENVELOPE",
        "corner_fl: {corner}"
    );
    assert_eq!(corner["error"]["typed"], true, "corner_fl: {corner}");
}
