//! RG-4-CANONICAL-BOOLEAN-PRODUCT required suite — boolean facts measure the
//! product, not the base operand.
//!
//! AUDIT RG-4 (VERTICAL_GAP): `top_volume` on a `TreeNode::Boolean` measured
//! `top_volume(&boolean.a)` — the BASE operand — with the tool's contribution
//! never applied, so `cut(canonical, canonical)` reported `V(A)` for `V(A-B)`
//! and every `fuse(swept, canonical)` row measured only its base. This suite
//! pins the landed fix:
//!
//!   1. `canonical_cut_measures_the_product` — `cut(box A, box B)` with
//!      A = `[0,2]^3` and B = `[0.5,1.5]^3` reports the PRODUCT volume 7, not
//!      the base's 8, through the landed MONO-6 contact-cover certificate.
//!   2. `canonical_fuse_measures_the_product` — `fuse(box A, box B)` with two
//!      disjoint canonical boxes reports the PRODUCT volume `V(A)+V(B)`, not
//!      the base's `V(A)`.
//!   3. `swept_tool_refuses_the_unmeasured_product_typed` — a boolean whose
//!      tool is a swept carrier has no product facts (the MONO-8 certified
//!      bracket is the open carrier); it refuses typed rather than reporting
//!      the base volume.
//!
//! The suite runs the compiled native module from a real interpreter exactly
//! as `mono_row_assembly.rs` / `ttc_authoring_arms.rs` do (the crate cdylib is
//! staged as `truck123d.pyd` beside the interpreter runtime DLLs). Geometry
//! expectations are independent JSON fixtures, never re-derived through the
//! door.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

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

/// One-time preparation of the native-module staging directory (python
/// subprocesses need `truck123d` importable from a real interpreter).
fn staged_native_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let exe = std::env::current_exe().expect("test binary path");
        let deps = exe.parent().expect("deps dir");
        let staging = std::env::temp_dir().join(format!("truck123d_rg4_{}", std::process::id()));
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
            copy_if_present(prefix, dll, &staging, dll);
        }
        if let Some(libunwind) = find_on_path("libunwind.dll") {
            let _ = std::fs::copy(libunwind, staging.join("libunwind.dll"));
        }
        staging
    })
}

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

/// The native `bd_facts` of one construction tree JSON.
fn native_facts(tree_json: &str) -> serde_json::Value {
    let script = r#"
import json, sys
import truck123d
facts = json.loads(truck123d.bd_facts(sys.argv[1]))
print(json.dumps(facts))
"#;
    let stdout = run_python(script, &[tree_json]);
    serde_json::from_str(stdout.trim()).expect("native facts json")
}

/// One placed part row around a solid at `(x, y, z)`.
fn part_row(solid: serde_json::Value, x: f64, y: f64, z: f64) -> serde_json::Value {
    serde_json::json!({
        "part": {
            "solid": solid,
            "x": x,
            "y": y,
            "z": z,
            "rz": 0.0,
        }
    })
}

/// One canonical box solid of the given extents.
fn box_solid(length: f64, width: f64, height: f64) -> serde_json::Value {
    serde_json::json!({
        "kind": "box",
        "length": length,
        "width": width,
        "height": height,
    })
}

/// One recorded boolean row over two operand nodes.
fn boolean_row(mode: &str, a: serde_json::Value, b: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "boolean": {
            "mode": mode,
            "a": a,
            "b": b,
        }
    })
}

/// A closed square section loop at station `z` (four line edges).
fn square_loop(z: f64) -> Vec<serde_json::Value> {
    let corners = [[0.0, 0.0, z], [1.0, 0.0, z], [1.0, 1.0, z], [0.0, 1.0, z]];
    (0..4)
        .map(|i| {
            serde_json::json!({
                "kind": "line",
                "a": corners[i],
                "b": corners[(i + 1) % 4],
            })
        })
        .collect()
}

/// A two-station ruled square-section loft: the `Swept` carrier class.
fn loft_solid() -> serde_json::Value {
    serde_json::json!({
        "kind": "loft",
        "closed": false,
        "sections": [square_loop(0.0), square_loop(5.0)],
    })
}

// ---------------------------------------------------------------------------
// Test 1: the canonical cut measures the PRODUCT (V(A) - V(A n B) = 7).
// ---------------------------------------------------------------------------

#[test]
fn canonical_cut_measures_the_product() {
    // A = [0, 2]^3 (a 2-cube placed at its centre (1, 1, 1)); B = [0.5, 1.5]^3
    // (a unit cube at the same centre, strictly inside A). V(A) = 8,
    // V(A - B) = 7.
    let a = part_row(box_solid(2.0, 2.0, 2.0), 1.0, 1.0, 1.0);
    let b = part_row(box_solid(1.0, 1.0, 1.0), 1.0, 1.0, 1.0);
    let tree = boolean_row("subtract", a, b).to_string();
    let facts = native_facts(&tree);
    let volume = facts["volume"].as_f64().expect("cut volume");
    assert!(
        (volume - 7.0).abs() < 1.0e-4,
        "cut(canonical, canonical) must measure the product 7, got {volume}: {facts}"
    );
    assert!(
        (volume - 8.0).abs() > 1.0e-4,
        "the base operand's volume 8 must not be reported: {facts}"
    );
}

// ---------------------------------------------------------------------------
// Test 2: the canonical fuse measures the PRODUCT (V(A) + V(B) = 9).
// ---------------------------------------------------------------------------

#[test]
fn canonical_fuse_measures_the_product() {
    // A = [0, 2]^3 (V = 8); B = [2.5, 3.5] x [0.5, 1.5]^2 (V = 1), disjoint.
    // V(A u B) = 9; the base alone is 8.
    let a = part_row(box_solid(2.0, 2.0, 2.0), 1.0, 1.0, 1.0);
    let b = part_row(box_solid(1.0, 1.0, 1.0), 3.0, 1.0, 1.0);
    let tree = boolean_row("union", a, b).to_string();
    let facts = native_facts(&tree);
    let volume = facts["volume"].as_f64().expect("fuse volume");
    assert!(
        (volume - 9.0).abs() < 1.0e-4,
        "fuse(canonical, canonical) must measure the product 9, got {volume}: {facts}"
    );
}

// ---------------------------------------------------------------------------
// Test 3: a swept tool refuses the unmeasured product typed.
// ---------------------------------------------------------------------------

#[test]
fn swept_tool_refuses_the_unmeasured_product_typed() {
    // A canonical base cut by a swept (lofted) tool: the pair routes, but the
    // product volume has no certified facts (the swept carrier is the open
    // MONO-8 carrier). The facts must refuse typed, never report the base.
    let a = part_row(box_solid(2.0, 2.0, 2.0), 1.0, 1.0, 1.0);
    let b = part_row(loft_solid(), 0.0, 0.0, 0.0);
    let tree = boolean_row("subtract", a, b).to_string();
    let script = r#"
import json, sys
import truck123d
try:
    facts = truck123d.bd_facts(sys.argv[1])
except truck123d.Refused as exc:
    payload = getattr(exc, "payload", {}) or {}
    print(json.dumps({"refused": True, "case": payload.get("case"),
                      "envelope": payload.get("envelope")}))
except Exception as exc:  # noqa: BLE001 - a wrong class is the boundary failure
    print(json.dumps({"refused": False, "class": type(exc).__name__}))
else:
    print(json.dumps({"refused": False, "class": "green", "facts": facts}))
"#;
    let stdout = run_python(script, &[&tree]);
    let record: serde_json::Value = serde_json::from_str(stdout.trim()).expect("refusal record");
    assert_eq!(
        record["refused"],
        serde_json::json!(true),
        "a swept-tool product must refuse typed, got {record}"
    );
    assert_eq!(
        record["case"],
        serde_json::json!("unsupported_envelope"),
        "the refusal must be the typed envelope refusal, got {record}"
    );
    assert_eq!(
        record["envelope"],
        serde_json::json!("non_canonical_carrier"),
        "the refusal must name the open carrier, got {record}"
    );
}
