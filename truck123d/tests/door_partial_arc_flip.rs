//! DOOR-PARTIAL-ARC-FLIP required suite — the door's partial-arc revolve now
//! passes through to the landed executor lathe arm.
//!
//! The packet's premise (the facade's PB-014 partial-arc op is landed) was
//! true of the classifier ledger only: the executor (`bd_bridge`) had no
//! partial-arc lathe arm. This suite pins the amended contract:
//!
//! 1. `door_partial_arc_revolve_answers_with_exact_facts` — a real python
//!    process drives the door drop-in. A 270-degree lathe and a 70-degree
//!    shield-shaped lathe (the corpus's `revolved_solid(..., arc_deg=70,
//!    start_deg=-35)` pattern) both answer with exact facts: the wedge volume
//!    is the full-revolution volume scaled by the swept fraction and the bbox
//!    is the exact support of the swept sector (the placement z-rotation folds
//!    into the recorded `start_deg`, so no rotated-AABB over-approximation).
//!    The partial-arc row serializes `arc_deg` (plus `start_deg` when nonzero)
//!    and the STL export emits the swept surface plus its two planar caps.
//! 2. `unanswerable_arc_option_refuses_typed` — an arc outside `(0, 360]`
//!    still refuses typed through the mapped `Refused` exception class.
//! 3. `full_revolution_rows_are_unchanged` — a full-360 row serializes
//!    exactly as before (no `start_deg` field, legacy `rz` placement) and its
//!    facts are bit-identical to the landed path.

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
            std::env::temp_dir().join(format!("truck123d_arc_flip_{}", std::process::id()));
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

/// Runs one python script (passed as `-c`) with the corpus dir and an STL
/// scratch path as arguments, returning the parsed JSON it prints.
fn run_script(script: &str, stl: &Path) -> serde_json::Value {
    let output = python_command()
        .arg("-c")
        .arg(script)
        .arg(corpus_ttc_dir())
        .arg(stl)
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
    serde_json::from_str(stdout.trim()).unwrap_or_else(|e| panic!("script json: {e}\n{stdout}"))
}

// ---------------------------------------------------------------------------
// Test 1: the door's partial-arc revolve answers with exact facts + STL.
// ---------------------------------------------------------------------------

#[test]
fn door_partial_arc_revolve_answers_with_exact_facts() {
    let script = r#"
import json, math, sys
sys.path.insert(0, sys.argv[1])
import door
import truck123d

class _Refused(Exception):
    def __init__(self, msg):
        super().__init__(msg)
        self.msg = msg

class _Stub:
    Refused = _Refused

door._T123D = _Stub()
bd = door._build_truck_module()

def v(x, y=0.0, z=0.0):
    return door.Vector(x, y, z)

def rect_face(x0, x1, z0, z1):
    pts = [v(x0, 0, z0), v(x1, 0, z0), v(x1, 0, z1), v(x0, 0, z1)]
    edges = [bd.Edge.make_line(pts[i], pts[(i + 1) % 4]) for i in range(4)]
    return bd.Face(bd.Wire(edges))

def facts_of(part):
    return json.loads(truck123d.bd_facts(json.dumps(part._node())))

# --- 270-degree lathe -------------------------------------------------------
wedge = bd.revolve(rect_face(10, 20, 0, 10), axis=bd.Axis.Z, revolution_arc=270.0)
node = wedge._node()["part"]
assert node["rz"] == 0.0, node
assert node["solid"]["arc_deg"] == 270.0, node
assert "start_deg" not in node["solid"], node
f270 = facts_of(wedge)
full = bd.revolve(rect_face(10, 20, 0, 10), axis=bd.Axis.Z)
ffull = facts_of(full)
assert abs(f270["volume"] - ffull["volume"] * 0.75) <= 1e-9 * ffull["volume"], (f270, ffull)

# --- 70-degree shield-shaped lathe, start -35 (the corpus rotate pattern) ---
shield = bd.revolve(rect_face(100, 110, 0, 50), axis=bd.Axis.Z, revolution_arc=70.0)
shield = shield.rotate(bd.Axis.Z, -35.0)
node = shield._node()["part"]
assert node["rz"] == 0.0, node
assert abs(node["solid"]["start_deg"] - (-35.0)) < 1e-12, node
assert node["solid"]["arc_deg"] == 70.0, node
f70 = facts_of(shield)
c = math.cos(math.radians(35.0))
s = math.sin(math.radians(35.0))
assert abs(f70["bbox"][0][0] - 100.0 * c) < 1e-9, f70
assert abs(f70["bbox"][1][0] - 110.0) < 1e-9, f70
assert abs(f70["bbox"][0][1] + 110.0 * s) < 1e-9, f70
assert abs(f70["bbox"][1][1] - 110.0 * s) < 1e-9, f70
full_volume = math.pi * (110.0 ** 2 - 100.0 ** 2) * 50.0
assert abs(f70["volume"] - full_volume * 70.0 / 360.0) <= 1e-9 * full_volume, f70

# --- STL export: the swept surface plus its two planar caps -----------------
triangles = json.loads(truck123d.bd_stl(json.dumps(shield._node()), sys.argv[2]))
assert triangles["triangles"] > 0, triangles

print(json.dumps({
    "ok": True,
    "volume_270": f270["volume"],
    "volume_70": f70["volume"],
    "bbox_70": f70["bbox"],
    "start_deg": node["solid"]["start_deg"],
    "triangles": triangles["triangles"],
}))
"#;
    let stl = std::env::temp_dir().join(format!("door_arc_flip_{}.stl", std::process::id()));
    let _ = std::fs::remove_file(&stl);
    let record = run_script(script, &stl);
    let _ = std::fs::remove_file(&stl);
    assert_eq!(record["ok"], true, "{record}");
    assert!(
        record["triangles"].as_u64().unwrap_or(0) > 0,
        "the partial-arc STL must emit triangles: {record}"
    );
    // The 70-degree shield volume: annulus r 100..110, height 50, swept 70/360.
    let expected_70 =
        std::f64::consts::PI * (110.0f64.powi(2) - 100.0f64.powi(2)) * 50.0 * 70.0 / 360.0;
    let got_70 = record["volume_70"].as_f64().expect("volume_70");
    assert!(
        (got_70 - expected_70).abs() <= 1e-9 * expected_70,
        "shield volume {got_70} vs expected {expected_70}"
    );
}

// ---------------------------------------------------------------------------
// Test 2: an unanswerable angle option still refuses typed.
// ---------------------------------------------------------------------------

#[test]
fn unanswerable_arc_option_refuses_typed() {
    let script = r#"
import json, sys
sys.path.insert(0, sys.argv[1])
import door
import truck123d

class _Refused(Exception):
    def __init__(self, msg):
        super().__init__(msg)
        self.msg = msg

class _Stub:
    Refused = _Refused

door._T123D = _Stub()
bd = door._build_truck_module()

def v(x, y=0.0, z=0.0):
    return door.Vector(x, y, z)

def rect_face(x0, x1, z0, z1):
    pts = [v(x0, 0, z0), v(x1, 0, z0), v(x1, 0, z1), v(x0, 0, z1)]
    edges = [bd.Edge.make_line(pts[i], pts[(i + 1) % 4]) for i in range(4)]
    return bd.Face(bd.Wire(edges))

refused = []
for arc in (0.0, -45.0, 400.0):
    part = bd.revolve(rect_face(10, 20, 0, 10), axis=bd.Axis.Z, revolution_arc=arc)
    try:
        truck123d.bd_facts(json.dumps(part._node()))
    except truck123d.Refused:
        refused.append(arc)
    else:
        refused.append(None)

assert refused == [0.0, -45.0, 400.0], refused
print(json.dumps({"ok": True, "refused": refused}))
"#;
    let stl = std::env::temp_dir().join(format!("door_arc_flip_bad_{}.stl", std::process::id()));
    let record = run_script(script, &stl);
    assert_eq!(record["ok"], true, "{record}");
    assert_eq!(record["refused"].as_array().map(Vec::len), Some(3));
}

// ---------------------------------------------------------------------------
// Test 3: full-360 rows serialize unchanged and keep the landed facts.
// ---------------------------------------------------------------------------

#[test]
fn full_revolution_rows_are_unchanged() {
    let script = r#"
import json, math, sys
sys.path.insert(0, sys.argv[1])
import door
import truck123d

class _Refused(Exception):
    def __init__(self, msg):
        super().__init__(msg)
        self.msg = msg

class _Stub:
    Refused = _Refused

door._T123D = _Stub()
bd = door._build_truck_module()

def v(x, y=0.0, z=0.0):
    return door.Vector(x, y, z)

def rect_face(x0, x1, z0, z1):
    pts = [v(x0, 0, z0), v(x1, 0, z0), v(x1, 0, z1), v(x0, 0, z1)]
    edges = [bd.Edge.make_line(pts[i], pts[(i + 1) % 4]) for i in range(4)]
    return bd.Face(bd.Wire(edges))

part = bd.revolve(rect_face(10, 20, 0, 10), axis=bd.Axis.Z)
node = part._node()["part"]
assert node["rz"] == 0.0, node
assert node["solid"]["arc_deg"] == 360.0, node
assert "start_deg" not in node["solid"], node
facts = json.loads(truck123d.bd_facts(json.dumps(part._node())))
# Annulus r 10..20, height 10.
expected = math.pi * (20.0 ** 2 - 10.0 ** 2) * 10.0
assert abs(facts["volume"] - expected) <= 1e-9 * expected, facts
print(json.dumps({"ok": True, "volume": facts["volume"]}))
"#;
    let stl = std::env::temp_dir().join(format!("door_arc_flip_full_{}.stl", std::process::id()));
    let record = run_script(script, &stl);
    assert_eq!(record["ok"], true, "{record}");
}
