//! AUTHOR-WIRE-MIRROR-ARM required suite — the door's wire/edge mirror arm.
//!
//! The swarm finding: `bd.mirror(wire, Plane.YZ)` was refused at the door
//! ("a mirror of this carrier is not a kernel-engine row") because the mirror
//! arm handled placed solids (`_Part`/`Compound`) only. Every hypercar master
//! shell mirrors a half-section wire before lofting
//! (`surfaces.py` `_mirrored_face`), so the refusal gated the whole family.
//!
//! This suite pins the amended contract:
//!
//! 1. `mirrored_spline_wire_records_reflected_control_data` — a spline wire's
//!    recorded control samples reflect exactly (x -> -x about `Plane.YZ`), with
//!    no re-interpolation and no resampling.
//! 2. `mirrored_line_records_reflected_endpoints` — a line edge's endpoints
//!    reflect exactly about `Plane.XZ`.
//! 3. `mirrored_free_plane_wire_refuses_typed` — a free-plane mirror refuses
//!    through the mapped `truck123d.Refused` class, same as the part arm.
//! 4. `mirrored_wire_lofts_green` — the mirrored wire feeds the existing
//!    `make_face`/`loft` section paths end to end and answers green facts.

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
/// door_circle_flip's staging: the crate cdylib is copied as `truck123d.pyd`
/// beside the interpreter runtime DLLs.
fn staged_native_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let exe = std::env::current_exe().expect("test binary path");
        let deps = exe.parent().expect("deps dir");
        let staging =
            std::env::temp_dir().join(format!("truck123d_wire_mirror_{}", std::process::id()));
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
import json, sys
sys.path.insert(0, sys.argv[1])
import truck123d
import door
door._T123D = truck123d
bd = door._build_truck_module()
def v(x, y, z):
    return door.Vector(x, y, z)
"#;

#[test]
fn mirrored_spline_wire_records_reflected_control_data() {
    let script = String::from(PREAMBLE)
        + r#"
pts = [(1.0, 2.0, 3.0), (4.0, -5.0, 6.0), (7.0, 8.0, -9.0)]
wire = bd.Spline(*[v(*p) for p in pts]) + bd.Line(v(*pts[-1]), v(*pts[0]))
mirrored = bd.mirror(wire, bd.Plane.YZ)
original = [list(p.to_tuple()) for e in wire.edges for p in e.points]
reflected = [list(p.to_tuple()) for e in mirrored.edges for p in e.points]
print(json.dumps({
    "kinds": [e.kind for e in mirrored.edges],
    "original": original,
    "reflected": reflected,
}))
"#;
    let record = run_script(&script);
    let kinds = record["kinds"].as_array().expect("edge kinds");
    assert_eq!(
        kinds[0], "spline",
        "the mirrored wire keeps the spline carrier"
    );
    let original = record["original"].as_array().expect("original points");
    let reflected = record["reflected"].as_array().expect("reflected points");
    assert_eq!(
        original.len(),
        reflected.len(),
        "the reflected wire keeps the recorded sample count"
    );
    for (before, after) in original.iter().zip(reflected.iter()) {
        let before = before.as_array().expect("original point");
        let after = after.as_array().expect("reflected point");
        let x = before[0].as_f64().expect("x");
        let y = before[1].as_f64().expect("y");
        let z = before[2].as_f64().expect("z");
        // Plane.YZ reflects x -> -x; y and z are untouched exact arithmetic.
        assert_eq!(after[0].as_f64(), Some(-x), "x must reflect about Plane.YZ");
        assert_eq!(after[1].as_f64(), Some(y), "y must be unchanged");
        assert_eq!(after[2].as_f64(), Some(z), "z must be unchanged");
    }
}

#[test]
fn mirrored_line_records_reflected_endpoints() {
    let script = String::from(PREAMBLE)
        + r#"
line = bd.Line(v(1.0, 2.0, 3.0), v(4.0, 5.0, 6.0))
mirrored = bd.mirror(line, bd.Plane.XZ)
print(json.dumps({
    "kind": mirrored.kind,
    "p0": list(mirrored.p0.to_tuple()),
    "p1": list(mirrored.p1.to_tuple()),
}))
"#;
    let record = run_script(&script);
    assert_eq!(record["kind"], "line");
    // Plane.XZ reflects y -> -y; x and z are untouched exact arithmetic.
    assert_eq!(record["p0"], serde_json::json!([1.0, -2.0, 3.0]));
    assert_eq!(record["p1"], serde_json::json!([4.0, -5.0, 6.0]));
}

#[test]
fn mirrored_free_plane_wire_refuses_typed() {
    let script = String::from(PREAMBLE)
        + r#"
wire = bd.Spline(v(1.0, 2.0, 3.0), v(4.0, -5.0, 6.0)) + bd.Line(v(4.0, -5.0, 6.0), v(1.0, 2.0, 3.0))
free = bd.Plane(origin=v(0.0, 0.0, 0.0), x_dir=v(1.0, 0.0, 0.0), z_dir=v(1.0, 1.0, 0.0))
try:
    bd.mirror(wire, free)
    print(json.dumps({"refused": False}))
except truck123d.Refused as exc:
    print(json.dumps({"refused": True, "message": str(exc)}))
"#;
    let record = run_script(&script);
    assert_eq!(
        record["refused"], true,
        "a free-plane wire mirror must refuse typed"
    );
}

#[test]
fn mirrored_wire_lofts_green() {
    let script = String::from(PREAMBLE)
        + r#"
half = bd.Polygon(v(0.0, 0.0, 0.0), v(2.0, 1.0, 0.0), v(0.0, 2.0, 0.0))
mirrored = bd.mirror(half, bd.Plane.XZ)
section = bd.make_face(mirrored)
far = bd.Plane.XY.offset(10.0) * section
part = bd.loft([section, far], ruled=True)
facts = json.loads(truck123d.bd_facts(json.dumps(part._node())))
print(json.dumps({"facts": facts}))
"#;
    let record = run_script(&script);
    let facts = &record["facts"];
    assert_eq!(
        facts["solid_count"], 1,
        "the mirrored wire must loft into one solid: {facts}"
    );
    assert!(
        facts["volume"].as_f64().expect("volume") > 0.0,
        "the mirrored-wire loft must answer a positive volume: {facts}"
    );
    let bbox = facts["bbox"].as_array().expect("bbox");
    for corner in bbox {
        for coordinate in corner.as_array().expect("bbox corner") {
            assert!(coordinate.as_f64().expect("bbox coordinate").is_finite());
        }
    }
}
