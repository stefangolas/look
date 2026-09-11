//! DOOR-CIRCLE-FLIP required suite — the circle-section recording arm of the
//! drop-in executor binding.
//!
//! The corpus `bd.Circle(radius)` profile was previously refused at the door
//! (`a circle profile is not answered exactly by a kernel-engine row`). This
//! packet lands the exact analytic circle section carrier end to end:
//!
//! 1. `tube_with_circle_section_records_green_facts` — the Merlin `tube()`
//!    idiom (a spline spine with a closed `Circle` section) records a sweep
//!    loft whose sections are exact circles and answers deterministic green
//!    facts.
//! 2. `cockpit_disc_stack_records_green_facts` — the cockpit `_disc_stack`
//!    idiom (a ruled loft of coaxial circles) records and answers the exact
//!    frustum-stack volume and the analytic AABB.
//! 3. `circle_with_an_unanswerable_option_refuses_typed` — a `Circle` option
//!    outside the recorded exact vocabulary refuses typed, naming the option.
//! 4. `unknown_carrier_vocabulary_is_named_in_the_refusal` — the parse arm
//!    carries the serde diagnostic so a future vocabulary gap is named, never
//!    collapsed to the generic empty-domain refusal.

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
/// ttc_lathe_spline's staging: the crate cdylib is copied as `truck123d.pyd`
/// beside the interpreter runtime DLLs.
fn staged_native_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let exe = std::env::current_exe().expect("test binary path");
        let deps = exe.parent().expect("deps dir");
        let staging =
            std::env::temp_dir().join(format!("truck123d_door_circle_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&staging);
        std::fs::create_dir_all(&staging).expect("staging dir");
        copy_if_present(deps, "truck123d.dll", &staging, "truck123d.pyd");
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

/// Runs a python script (with the corpus `ttc` dir as `argv[1]`) and parses the
/// single JSON object it prints on stdout.
fn run_script(script: &str) -> serde_json::Value {
    let output = python_command()
        .arg("-c")
        .arg(script)
        .arg(corpus_ttc_dir())
        .output()
        .expect("spawn door python");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    assert!(
        output.status.success(),
        "door python failed:\nstdout:{stdout}\nstderr:{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_str(stdout.trim())
        .unwrap_or_else(|error| panic!("door python json: {error}\nstdout:{stdout}"))
}

/// The common preamble: import the real native module and the corpus door.
const PREAMBLE: &str = r#"
import json, math, sys
sys.path.insert(0, sys.argv[1])
import truck123d
import door
door._T123D = truck123d
bd = door._build_truck_module()
def v(x, y, z):
    return door.Vector(x, y, z)
"#;

/// Strips the wall-clock timing column (a diagnostic, never a gate).
fn strip_timing(value: &serde_json::Value) -> serde_json::Value {
    let mut value = value.clone();
    if let Some(map) = value.as_object_mut() {
        map.remove("timing");
    }
    value
}

#[test]
fn tube_with_circle_section_records_green_facts() {
    let script = String::from(PREAMBLE)
        + r#"
pts = [(0.0, 0.0, 0.0), (10.0, 0.0, 0.0), (20.0, 5.0, 0.0), (30.0, 10.0, 5.0)]
path = bd.Edge.make_spline([v(*p) for p in pts])
section = bd.Plane(origin=path.position_at(0), z_dir=path.tangent_at(0)) * bd.Circle(2.0)
part = bd.sweep(section, path=path)
node = part._node()
facts = json.loads(truck123d.bd_facts(json.dumps(node)))
again = json.loads(truck123d.bd_facts(json.dumps(node)))
print(json.dumps({"node": node, "facts": facts, "again": again}))
"#;
    let record = run_script(&script);
    let sections = record["node"]["part"]["solid"]["sections"]
        .as_array()
        .expect("sections");
    assert!(
        sections.len() >= 2,
        "the tube records a multi-station chain"
    );
    for section in sections {
        let edges = section.as_array().expect("section edges");
        assert_eq!(edges.len(), 1, "each tube station is a single circle edge");
        assert_eq!(edges[0]["kind"], "circle");
        assert_eq!(edges[0]["radius"].as_f64(), Some(2.0));
    }
    let facts = &record["facts"];
    assert_eq!(facts["solid_count"], 1);
    assert!(
        facts["volume"].as_f64().expect("volume") > 0.0,
        "the tube must answer a positive volume: {facts}"
    );
    let bbox = facts["bbox"].as_array().expect("bbox");
    for corner in bbox {
        for coordinate in corner.as_array().expect("bbox corner") {
            assert!(coordinate.as_f64().expect("bbox coordinate").is_finite());
        }
    }
    // Determinism: the facts (timing stripped) repeat byte for byte.
    assert_eq!(
        strip_timing(&record["facts"]),
        strip_timing(&record["again"]),
        "circle-section sweep facts must be deterministic"
    );
}

#[test]
fn cockpit_disc_stack_records_green_facts() {
    let script = String::from(PREAMBLE)
        + r#"
rings = [(0.0, 10.0), (5.0, 8.0), (10.0, 6.0), (15.0, 6.0)]
faces = [bd.Plane.XY.offset(o) * bd.Circle(r) for o, r in rings]
part = bd.loft(faces, ruled=True)
facts = json.loads(truck123d.bd_facts(json.dumps(part._node())))
expect = 0.0
for (o0, r0), (o1, r1) in zip(rings, rings[1:]):
    expect += math.pi * (o1 - o0) / 3.0 * (r0 * r0 + r0 * r1 + r1 * r1)
print(json.dumps({"facts": facts, "expect": expect}))
"#;
    let record = run_script(&script);
    let facts = &record["facts"];
    assert_eq!(facts["solid_count"], 1);
    let volume = facts["volume"].as_f64().expect("volume");
    let expect = record["expect"].as_f64().expect("expect");
    assert!(
        (volume - expect).abs() / expect < 1e-12,
        "disc-stack volume {volume} must equal the exact frustum stack {expect}"
    );
    assert_eq!(
        facts["bbox"],
        serde_json::json!([[-10.0, -10.0, 0.0], [10.0, 10.0, 15.0]])
    );
}

#[test]
fn circle_with_an_unanswerable_option_refuses_typed() {
    let script = String::from(PREAMBLE)
        + r#"
try:
    bd.Circle(3.0, arc_size=180.0)
    print(json.dumps({"refused": False}))
except truck123d.Refused as exc:
    print(json.dumps({"refused": True, "message": str(exc)}))
"#;
    let record = run_script(&script);
    assert_eq!(record["refused"], true, "a circle option must refuse typed");
    let message = record["message"].as_str().unwrap_or("");
    assert!(
        message.contains("arc_size"),
        "the refusal must name the unanswerable option: {message}"
    );
}

#[test]
fn unknown_carrier_vocabulary_is_named_in_the_refusal() {
    let script = String::from(PREAMBLE)
        + r#"
row = {"part": {"solid": {"kind": "not_a_recorded_carrier"},
                "x": 0.0, "y": 0.0, "z": 0.0, "rz": 0.0}}
try:
    truck123d.bd_facts(json.dumps(row))
    print(json.dumps({"refused": False}))
except truck123d.Refused as exc:
    print(json.dumps({"refused": True, "message": str(exc)}))
"#;
    let record = run_script(&script);
    assert_eq!(record["refused"], true);
    let message = record["message"].as_str().unwrap_or("");
    assert!(
        message.contains("vocabulary gap"),
        "a vocabulary gap must be named, not generic: {message}"
    );
}
