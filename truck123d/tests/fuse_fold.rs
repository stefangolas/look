//! MONO-9-FUSE-FOLD required suite -- the certified multi-operand fold and the
//! chained composition it unblocks.
//!
//! The monocoque gap item 3 plus the depth-2 composition the census found
//! everywhere (21 of 27 f1 rows chain booleans on one body). The fold is
//! per-tool flux against a COMPOUND membership indicator, not a pairwise fold:
//!
//!   union      V = sum_k int_{dS_k} g * 1_{every other operand OUTSIDE}
//!   subtract   V = int_{dS_base} g * 1_{all tools OUTSIDE}
//!                  - sum_i int_{dS_i} g * 1_{base INSIDE, other tools OUTSIDE}
//!   intersect  V = sum_k int_{dS_k} g * 1_{every other operand INSIDE}
//!
//! No intermediate union is ever constructed; the MONO-5 membership primitive
//! evaluates every sample point against each other operand directly. The tests
//! prove the fold on boxes with known closed-form answers before any
//! spline-carrier composition, and prove the typed refusal of an uncertifiable
//! depth-2 operand.
//!
//! The suite runs the compiled native module from a real interpreter exactly
//! as `rg4_boolean_product.rs` / `mono_row_assembly.rs` do (the crate cdylib is
//! staged as `truck123d.pyd` beside the interpreter runtime DLLs). Geometry
//! expectations are independent closed forms, never re-derived through the
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
        let staging = std::env::temp_dir().join(format!("truck123d_fold_{}", std::process::id()));
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

/// The native `bd_facts` of one construction tree JSON, or the typed refusal
/// record `{refused, case, envelope}`.
fn native_outcome(tree_json: &str) -> serde_json::Value {
    let script = r#"
import json, sys
import truck123d
try:
    facts = json.loads(truck123d.bd_facts(sys.argv[1]))
except truck123d.Refused as exc:
    payload = getattr(exc, "payload", {}) or {}
    print(json.dumps({"refused": True, "case": payload.get("case"),
                      "envelope": payload.get("envelope")}))
except Exception as exc:  # noqa: BLE001 - a wrong class is the boundary failure
    print(json.dumps({"refused": False, "class": type(exc).__name__}))
else:
    print(json.dumps({"refused": False, "class": "green", "facts": facts}))
"#;
    let stdout = run_python(script, &[tree_json]);
    serde_json::from_str(stdout.trim()).expect("native outcome json")
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

/// One `fuse(base, *tools)` chain as the door records it: a left-associated
/// run of union rows over the recorded operand order.
fn fuse_chain(base: serde_json::Value, tools: &[serde_json::Value]) -> serde_json::Value {
    tools
        .iter()
        .fold(base, |acc, tool| boolean_row("union", acc, tool.clone()))
}

/// One `cut(base, *tools)` chain (left-associated subtract).
fn cut_chain(base: serde_json::Value, tools: &[serde_json::Value]) -> serde_json::Value {
    tools
        .iter()
        .fold(base, |acc, tool| boolean_row("subtract", acc, tool.clone()))
}

/// One `intersect(base, *tools)` chain (left-associated intersect).
fn intersect_chain(base: serde_json::Value, tools: &[serde_json::Value]) -> serde_json::Value {
    tools.iter().fold(base, |acc, tool| {
        boolean_row("intersect", acc, tool.clone())
    })
}

/// A straight spline edge (three collinear defining samples, the corpus
/// `Edge.make_spline` recording) between `a` and `b`.
fn spline_edge(a: [f64; 3], b: [f64; 3]) -> serde_json::Value {
    let mid = [
        0.5 * (a[0] + b[0]),
        0.5 * (a[1] + b[1]),
        0.5 * (a[2] + b[2]),
    ];
    serde_json::json!({"kind": "spline", "points": [a, mid, b]})
}

/// A closed square section loop at station `z` (four straight spline edges).
fn spline_square_loop(z: f64, size: f64) -> Vec<serde_json::Value> {
    let corners = [
        [0.0, 0.0, z],
        [size, 0.0, z],
        [size, size, z],
        [0.0, size, z],
    ];
    (0..4)
        .map(|i| spline_edge(corners[i], corners[(i + 1) % 4]))
        .collect()
}

/// A two-station straight spline-section loft: the `Swept` carrier class.
fn spline_loft_solid(size: f64, height: f64) -> serde_json::Value {
    serde_json::json!({
        "kind": "loft",
        "closed": false,
        "sections": [
            spline_square_loop(0.0, size),
            spline_square_loop(height, size),
        ],
    })
}

// ---------------------------------------------------------------------------
// Test 1: a three-tool union of boxes, exact.
// ---------------------------------------------------------------------------

#[test]
fn three_tool_union_of_boxes_is_exact() {
    // A = [0,1]^3 (V = 1); B1 = [2,3]x[0,1]^2 (V = 1); B2 = [4,5]x[0,1]^2
    // (V = 1); B3 = [4.25,4.75]^3 (V = 0.125) is strictly inside B2, so the
    // union excludes it. The fold is A u B1 u B2 u B3 = 3, never 3.125.
    let a = part_row(box_solid(1.0, 1.0, 1.0), 0.5, 0.5, 0.5);
    let b1 = part_row(box_solid(1.0, 1.0, 1.0), 2.5, 0.5, 0.5);
    let b2 = part_row(box_solid(1.0, 1.0, 1.0), 4.5, 0.5, 0.5);
    let b3 = part_row(box_solid(0.5, 0.5, 0.5), 4.5, 0.5, 0.5);
    let tree = fuse_chain(a, &[b1, b2, b3]).to_string();
    let facts = native_facts(&tree);
    let volume = facts["volume"].as_f64().expect("union volume");
    assert!(
        (volume - 3.0).abs() < 1.0e-4,
        "the three-tool union must measure 3, got {volume}: {facts}"
    );
    assert!(
        (volume - 3.125).abs() > 1.0e-4,
        "the nested fourth operand must be excluded: {facts}"
    );
    let bracket = facts["volume_bracket"]
        .as_array()
        .expect("the fold reports a bracket");
    assert_eq!(bracket.len(), 2, "the bracket is two-sided");
}

// ---------------------------------------------------------------------------
// Test 2: a chained subtract, exact.
// ---------------------------------------------------------------------------

#[test]
fn chained_subtract_is_exact() {
    // A = [0,3]^3 (V = 27); the three tools are strictly inside A and
    // pairwise disjoint: V = 27 - 1 - 0.125 - 0.125 = 25.75.
    let a = part_row(box_solid(3.0, 3.0, 3.0), 1.5, 1.5, 1.5);
    let t1 = part_row(box_solid(1.0, 1.0, 1.0), 1.0, 1.0, 1.0);
    let t2 = part_row(box_solid(0.5, 0.5, 0.5), 2.25, 2.25, 2.25);
    let t3 = part_row(box_solid(0.5, 0.5, 0.5), 2.25, 1.0, 1.0);
    let tree = cut_chain(a, &[t1, t2, t3]).to_string();
    let facts = native_facts(&tree);
    let volume = facts["volume"].as_f64().expect("cut volume");
    assert!(
        (volume - 25.75).abs() < 1.0e-4,
        "the chained subtract must measure 25.75, got {volume}: {facts}"
    );
}

// ---------------------------------------------------------------------------
// Test 3: one chained intersect, exact.
// ---------------------------------------------------------------------------

#[test]
fn chained_intersect_is_exact() {
    // A = [0,3]^3, B1 = [0.5,2.5]^3, B2 = [1,2]^3, all concentric and nested.
    // The intersection is B2, V = 1.
    let a = part_row(box_solid(3.0, 3.0, 3.0), 1.5, 1.5, 1.5);
    let b1 = part_row(box_solid(2.0, 2.0, 2.0), 1.5, 1.5, 1.5);
    let b2 = part_row(box_solid(1.0, 1.0, 1.0), 1.5, 1.5, 1.5);
    let tree = intersect_chain(a, &[b1, b2]).to_string();
    let facts = native_facts(&tree);
    let volume = facts["volume"].as_f64().expect("intersect volume");
    assert!(
        (volume - 1.0).abs() < 1.0e-4,
        "the chained intersect must measure 1, got {volume}: {facts}"
    );
}

// ---------------------------------------------------------------------------
// Test 4: one spline-carrier three-tool fuse closing within budget.
// ---------------------------------------------------------------------------

#[test]
fn spline_carrier_three_tool_fuse_closes_within_budget() {
    // Three disjoint straight spline-section lofts (the `Swept` carrier the
    // executor extracts as a patch 2-cycle). The union of disjoint solids is
    // the exact sum of the single-part volumes; the fold must contain it and
    // close within the 1e-4 relative band.
    let base = part_row(spline_loft_solid(2.0, 5.0), 0.0, 0.0, 0.0);
    let t1 = part_row(spline_loft_solid(2.0, 5.0), 10.0, 0.0, 0.0);
    let t2 = part_row(spline_loft_solid(2.0, 5.0), 20.0, 0.0, 0.0);
    let expected = [base.clone(), t1.clone(), t2.clone()]
        .iter()
        .map(|part| {
            native_facts(&part.to_string())["volume"]
                .as_f64()
                .expect("single-loft volume")
        })
        .sum::<f64>();
    let tree = fuse_chain(base, &[t1, t2]).to_string();
    let facts = native_facts(&tree);
    let bracket = facts["volume_bracket"]
        .as_array()
        .expect("the fold reports a bracket");
    let lo = bracket[0].as_f64().expect("bracket lo");
    let hi = bracket[1].as_f64().expect("bracket hi");
    assert!(
        lo <= expected && expected <= hi,
        "the fold bracket [{lo}, {hi}] must contain {expected}: {facts}"
    );
    assert!(
        hi - lo < 1.0e-4 * expected.abs(),
        "the fold must close within the 1e-4 band, width {}: {facts}",
        hi - lo
    );
}

// ---------------------------------------------------------------------------
// Test 5: a depth-2 operand with an uncertifiable sub-result refuses typed.
// ---------------------------------------------------------------------------

#[test]
fn depth_two_operand_with_uncertifiable_sub_result_refuses_typed() {
    // (A u Cylinder) u B: the flattened fold carries a cylinder operand, whose
    // carrier is outside the extracted patch vocabulary. The fold must refuse
    // typed naming the open carrier -- never approximate the cylinder away and
    // never report the base operand's volume.
    let a = part_row(box_solid(1.0, 1.0, 1.0), 0.5, 0.5, 0.5);
    let cylinder = part_row(
        serde_json::json!({"kind": "cylinder", "radius": 0.5, "height": 1.0, "axis": "z"}),
        4.0,
        0.5,
        0.5,
    );
    let b = part_row(box_solid(1.0, 1.0, 1.0), 8.0, 0.5, 0.5);
    let tree = fuse_chain(a, &[cylinder, b]).to_string();
    let record = native_outcome(&tree);
    assert_eq!(
        record["refused"],
        serde_json::json!(true),
        "an uncertifiable depth-2 operand must refuse typed, got {record}"
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
