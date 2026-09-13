//! FHC-B-MULTI-CONTOUR-SECTIONS required suite — profiles with holes
//! (annular sections).
//!
//! The landed section representation is single-ring (`orient_profile_ring` /
//! `profile_loop`). This suite lands the multi-contour section: one or more
//! oriented rings, outer CCW and holes CW, so the hole contributes negative
//! flux through the SAME per-patch certified machinery. There is no new facts
//! path and no 2D boolean machinery: the corpus `Circle(a) - Circle(b)` annular
//! sketch is recorded as a two-contour section, and its cap is the planar quad
//! strip between corresponding rings.
//!
//! Tests:
//!
//! 1. `analytic_annulus_prism_matches_closed_form` — the corpus `_ring` idiom
//!    (extrude of `Circle(R) - Circle(r)`) answers the exact annular-prism
//!    volume `pi (R^2 - r^2) h` and the outer AABB.
//! 2. `analytic_annulus_revolve_matches_closed_form` — a two-contour meridian
//!    section revolved about z answers the exact bracket
//!    `pi (r1^2 - r0^2) h` (lo == hi).
//! 3. `crossing_contours_refuse_typed_nesting` — a tool ring that crosses the
//!    outer ring refuses `E_NESTING_INVALID`.
//! 4. `ring_outside_ring_refuses_typed_nesting` — a tool ring outside the
//!    outer ring refuses `E_NESTING_INVALID`.
//! 5. `annular_prism_mesh_is_the_matched_annular_grid` — the mesh is the outer
//!    wall + inward hole wall + the two matched annular cap strips.
//! 6. `brakes_and_suspension_front_clear_the_two_contour_boolean` — the two
//!    census rows are green or typed-refusing at their NEXT honest carrier,
//!    never at the old `face_boolean` refusal.

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
        let staging = std::env::temp_dir().join(format!("truck123d_mcs_{}", std::process::id()));
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

/// Runs a python `-c` script with the corpus `ttc` dir as `argv[1]`; parses the
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

/// Runs the corpus door with `--engine truck` over one tree/module/entry.
fn run_truck_door(tree: &str, module: &str, entry: &str, args_json: &str) -> serde_json::Value {
    let stl = std::env::temp_dir().join(format!("ttc_fhc_b_{}_{}.stl", std::process::id(), entry));
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
import json, math, os, sys, tempfile
sys.path.insert(0, sys.argv[1])
import truck123d
import door
door._T123D = truck123d
bd = door._build_truck_module()
"#;

/// Relative error helper.
fn rel_error(value: f64, expect: f64) -> f64 {
    (value - expect).abs() / expect.abs()
}

#[test]
fn analytic_annulus_prism_matches_closed_form() {
    let script = String::from(PREAMBLE)
        + r#"
r_out, r_in, height = 5.0, 2.0, 3.0
annulus = bd.Circle(r_out) - bd.Circle(r_in)
part = bd.extrude(annulus, amount=height)
node = part._node()
facts = json.loads(truck123d.bd_facts(json.dumps(node)))
expect = math.pi * (r_out * r_out - r_in * r_in) * height
print(json.dumps({
    "node": node,
    "facts": facts,
    "expect": expect,
    "holes": len(node["part"]["solid"].get("holes", [])),
}))
"#;
    let record = run_script(&script);
    assert_eq!(
        record["holes"], 1,
        "the annular sketch records one hole ring"
    );
    let facts = &record["facts"];
    let volume = facts["volume"].as_f64().expect("volume");
    let expect = record["expect"].as_f64().expect("expect");
    assert!(
        rel_error(volume, expect) < 1e-12,
        "annular prism volume {volume} must equal pi (R^2 - r^2) h = {expect}"
    );
    // The bracket is the degenerate scalar interval [V, V].
    let bracket = facts["volume_bracket"].as_array().expect("bracket");
    assert_eq!(bracket[0].as_f64(), Some(volume));
    assert_eq!(bracket[1].as_f64(), Some(volume));
    // The hole is inside the outer ring, so the AABB is the outer ring's.
    assert_eq!(
        facts["bbox"],
        serde_json::json!([[-5.0, -5.0, 0.0], [5.0, 5.0, 3.0]])
    );
}

#[test]
fn analytic_annulus_revolve_matches_closed_form() {
    let script = String::from(PREAMBLE)
        + r#"
r0, r1, z0, z1 = 2.0, 5.0, 0.0, 4.0
outer = bd.Polyline((0.0, z0), (r1, z0), (r1, z1), (0.0, z1), close=True)
inner = bd.Polyline((0.0, z0), (r0, z0), (r0, z1), (0.0, z1), close=True)
face = bd.make_face(outer, inner)
part = bd.revolve(bd.Plane.XZ * face, axis=bd.Axis.Z, revolution_arc=360.0)
facts = json.loads(truck123d.bd_facts(json.dumps(part._node())))
expect = math.pi * (r1 * r1 - r0 * r0) * (z1 - z0)
print(json.dumps({"facts": facts, "expect": expect}))
"#;
    let record = run_script(&script);
    let facts = &record["facts"];
    let expect = record["expect"].as_f64().expect("expect");
    let bracket = facts["volume_bracket"].as_array().expect("bracket");
    let lo = bracket[0].as_f64().expect("lo");
    let hi = bracket[1].as_f64().expect("hi");
    assert_eq!(lo, hi, "the analytic annulus bracket is degenerate");
    assert!(
        rel_error(lo, expect) < 1e-12,
        "revolved annulus bracket {lo} must equal pi (r1^2 - r0^2) h = {expect}"
    );
}

#[test]
fn crossing_contours_refuse_typed_nesting() {
    let script = String::from(PREAMBLE)
        + r#"
try:
    bd.Circle(5.0) - (bd.Pos(4.0, 0.0, 0.0) * bd.Circle(3.0))
    print(json.dumps({"refused": False}))
except truck123d.Refused as exc:
    print(json.dumps({"refused": True, "code": exc.refusal_code, "message": str(exc)}))
"#;
    let record = run_script(&script);
    assert_eq!(record["refused"], true, "crossing rings must refuse typed");
    assert_eq!(
        record["code"], "E_NESTING_INVALID",
        "crossing rings must name E_NESTING_INVALID: {record}"
    );
}

#[test]
fn ring_outside_ring_refuses_typed_nesting() {
    let script = String::from(PREAMBLE)
        + r#"
try:
    bd.Circle(5.0) - (bd.Pos(10.0, 0.0, 0.0) * bd.Circle(2.0))
    print(json.dumps({"refused": False}))
except truck123d.Refused as exc:
    print(json.dumps({"refused": True, "code": exc.refusal_code}))
"#;
    let record = run_script(&script);
    assert_eq!(record["refused"], true, "an outside ring must refuse typed");
    assert_eq!(
        record["code"], "E_NESTING_INVALID",
        "an outside ring must name E_NESTING_INVALID: {record}"
    );
}

#[test]
fn annular_prism_mesh_is_the_matched_annular_grid() {
    // 64 outer-wall quads + 64 inward hole-wall quads + 64 bottom + 64 top
    // matched cap quads, two triangles per quad.
    let script = String::from(PREAMBLE)
        + r#"
annulus = bd.Circle(5.0) - bd.Circle(2.0)
part = bd.extrude(annulus, amount=3.0)
path = os.path.join(tempfile.gettempdir(), "ttc_fhc_b_annulus.stl")
out = json.loads(truck123d.bd_stl(json.dumps(part._node()), path, None))
print(json.dumps({"triangles": out["triangles"], "exists": os.path.exists(path)}))
"#;
    let record = run_script(&script);
    assert_eq!(record["exists"], true, "the annulus mesh writes an STL");
    assert_eq!(
        record["triangles"], 512,
        "the annular grid is 64 matched segments of walls and caps"
    );
}

#[test]
fn brakes_and_suspension_front_clear_the_two_contour_boolean() {
    for (tree, module, entry) in [
        ("hypercar", "lib.brakes", "build"),
        ("f1", "lib.suspension", "build_suspension_front"),
    ] {
        let record = run_truck_door(tree, module, entry, "[]");
        if record["ok"] == serde_json::json!(true) {
            continue;
        }
        let error = &record["error"];
        assert_eq!(
            error["kind"], "Refused",
            "{entry} must be green or refuse typed: {record}"
        );
        assert_eq!(
            error["typed"], true,
            "{entry} must refuse typed, never untyped: {record}"
        );
        assert_ne!(
            error["carrier"], "face_boolean",
            "{entry} must clear the old planar-profile boolean refusal: {record}"
        );
    }
}
