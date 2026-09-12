//! FHC-FACTS-CACHE required suite — content-hash memoization of the certified
//! facts evaluation across composition sites.
//!
//! The construction tree is canonical data and every measured fact is a
//! deterministic pure function of it, so the canonical node bytes are an exact
//! cache key: equal bytes imply equal evaluation. The process-wide memo pays
//! once for the door's repeated authoring probes and the final facts pass.
//!
//!   1. `identical_node_evaluated_once` — the same node through `bd_facts`
//!      twice: the second call reports the memo hit and returns the
//!      bit-identical facts record (timing stripped).
//!   2. `distinct_nodes_independent` — structurally near-identical subtrees do
//!      not collide; each keeps its own facts and a re-query hits its own memo.
//!   3. `growing_composition_reuses_children` — a three-deep fuse chain
//!      re-extracts only the operands it added after the child chain was
//!      evaluated (the child patch extraction is reused).
//!   4. `fingerprint_unchanged` — the full-vehicle `bd_stl(None)` fingerprint
//!      4,461,816 is bit-identical with the cache active.
//!   5. `cache_order_invisible` — populating the memo in different orders
//!      yields byte-identical (timing-stripped) outputs.
//!
//! The suite runs the compiled native module from a real interpreter exactly
//! as `mesh_cache.rs` does (the crate cdylib is staged as `truck123d.pyd`
//! beside the interpreter runtime DLLs).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn corpus_ttc_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("corpus")
        .join("ttc")
}

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

/// One-time preparation of the native-module staging directory: the crate
/// cdylib is copied as `truck123d.pyd` beside the interpreter runtime DLLs.
fn staged_native_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let exe = std::env::current_exe().expect("test binary path");
        let deps = exe.parent().expect("deps dir");
        let staging =
            std::env::temp_dir().join(format!("truck123d_facts_cache_{}", std::process::id()));
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

fn copy_if_present(from: &Path, name: &str, to: &Path, as_name: &str) {
    let src = from.join(name);
    if src.is_file() {
        let _ = std::fs::copy(&src, to.join(as_name));
    }
}

fn copy_from_dir(from: &Path, name: &str, to: &Path) {
    copy_if_present(from, name, to, name);
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

/// A scratch output directory unique to this test process.
fn scratch_dir() -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("truck123d_facts_cache_out_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

// ---------------------------------------------------------------------------
// Fixture builders (the kernel-row vocabulary, independent of the door).
// ---------------------------------------------------------------------------

/// One placed part row around a solid, at `(x, y, z)`.
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

/// One left-associated `fuse(base, *tools)` chain, exactly as the door records
/// a union run.
fn fuse_chain(base: serde_json::Value, tools: &[serde_json::Value]) -> serde_json::Value {
    tools
        .iter()
        .fold(base, |acc, tool| boolean_row("union", acc, tool.clone()))
}

/// Strips the wall-clock/diagnostic `timing` column from one facts record.
fn strip_timing(value: &serde_json::Value) -> serde_json::Value {
    let mut value = value.clone();
    if let Some(map) = value.as_object_mut() {
        map.remove("timing");
    }
    value
}

// ---------------------------------------------------------------------------
// Native evaluation helpers (one interpreter process per call).
// ---------------------------------------------------------------------------

/// Evaluates `bd_facts` on a sequence of trees in ONE interpreter process (the
/// process-wide memo persists across the calls) and returns the records in
/// evaluation order. A repeated index re-queries the same node.
fn facts_sequence(trees: &[serde_json::Value], order: &[usize]) -> Vec<serde_json::Value> {
    let script = r#"
import json, sys
import truck123d
trees = json.loads(sys.argv[1])
order = json.loads(sys.argv[2])
out = []
for i in order:
    out.append(json.loads(truck123d.bd_facts(json.dumps(trees[i]))))
print(json.dumps(out))
"#;
    let trees_json = serde_json::to_string(trees).expect("trees json");
    let order_json = serde_json::to_string(order).expect("order json");
    let stdout = run_python(script, &[&trees_json, &order_json]);
    serde_json::from_str(stdout.trim()).expect("facts sequence json")
}

/// Evaluates `bd_facts` on a sequence of trees in ONE interpreter process and
/// returns the records in NODE order (index 0..n), regardless of evaluation
/// order. Used to compare cache population orders.
fn facts_node_order(trees: &[serde_json::Value], order: &[usize]) -> Vec<serde_json::Value> {
    let script = r#"
import json, sys
import truck123d
trees = json.loads(sys.argv[1])
order = json.loads(sys.argv[2])
cache = {}
for i in order:
    cache[str(i)] = json.loads(truck123d.bd_facts(json.dumps(trees[i])))
print(json.dumps([cache[str(i)] for i in range(len(trees))]))
"#;
    let trees_json = serde_json::to_string(trees).expect("trees json");
    let order_json = serde_json::to_string(order).expect("order json");
    let stdout = run_python(script, &[&trees_json, &order_json]);
    serde_json::from_str(stdout.trim()).expect("facts node-order json")
}

fn timing_u64(record: &serde_json::Value, key: &str) -> u64 {
    record["timing"][key]
        .as_u64()
        .unwrap_or_else(|| panic!("timing.{key} must be a u64: {record}"))
}

fn timing_bool(record: &serde_json::Value, key: &str) -> bool {
    record["timing"][key]
        .as_bool()
        .unwrap_or_else(|| panic!("timing.{key} must be a bool: {record}"))
}

// ---------------------------------------------------------------------------
// Test 1: the same node evaluated twice pays once and is bit-identical.
// ---------------------------------------------------------------------------

#[test]
fn identical_node_evaluated_once() {
    let tree = fuse_chain(
        part_row(box_solid(1.0, 1.0, 1.0), 0.5, 0.5, 0.5),
        &[
            part_row(box_solid(1.0, 1.0, 1.0), 2.5, 0.5, 0.5),
            part_row(box_solid(1.0, 1.0, 1.0), 4.5, 0.5, 0.5),
        ],
    );
    let records = facts_sequence(&[tree], &[0, 0]);
    assert_eq!(records.len(), 2, "two evaluations of one node");
    let first = &records[0];
    let second = &records[1];
    assert!(
        !timing_bool(first, "cache_hit"),
        "the first evaluation must populate the memo: {first}"
    );
    assert!(
        timing_bool(second, "cache_hit"),
        "the second evaluation must hit the memo: {second}"
    );
    assert_eq!(
        strip_timing(first),
        strip_timing(second),
        "a cached evaluation must return the bit-identical record"
    );
}

// ---------------------------------------------------------------------------
// Test 2: structurally near-identical subtrees never collide.
// ---------------------------------------------------------------------------

#[test]
fn distinct_nodes_independent() {
    // The same left-associated chain shape; the only difference is the last
    // tool's extents. Near-identical canonical bytes, distinct evaluations.
    let base = || part_row(box_solid(1.0, 1.0, 1.0), 0.5, 0.5, 0.5);
    let tool = || part_row(box_solid(1.0, 1.0, 1.0), 2.5, 0.5, 0.5);
    let tree_a = fuse_chain(
        base(),
        &[tool(), part_row(box_solid(1.0, 1.0, 1.0), 4.5, 0.5, 0.5)],
    );
    let tree_b = fuse_chain(
        base(),
        &[tool(), part_row(box_solid(2.0, 1.0, 1.0), 5.0, 0.5, 0.5)],
    );

    let records = facts_sequence(&[tree_a.clone(), tree_b, tree_a], &[0, 1, 2]);
    let a_first = &records[0];
    let b = &records[1];
    let a_again = &records[2];

    let volume_a = a_first["volume"].as_f64().expect("volume a");
    let volume_b = b["volume"].as_f64().expect("volume b");
    assert!(
        (volume_a - 3.0).abs() < 1.0e-4,
        "the first chain must measure 3, got {volume_a}: {a_first}"
    );
    assert!(
        (volume_b - 4.0).abs() < 1.0e-4,
        "the second chain must measure 4, got {volume_b}: {b}"
    );
    assert_ne!(
        strip_timing(a_first),
        strip_timing(b),
        "distinct subtrees must not collide"
    );
    assert!(
        timing_bool(a_again, "cache_hit"),
        "re-querying the first node must hit its own memo: {a_again}"
    );
    assert_eq!(
        strip_timing(a_first),
        strip_timing(a_again),
        "the first node keeps its own facts after the second populated the memo"
    );
}

// ---------------------------------------------------------------------------
// Test 3: a growing composition reuses the child patch extraction.
// ---------------------------------------------------------------------------

#[test]
fn growing_composition_reuses_children() {
    // Disjoint unit boxes at x = 0.5, 2.5, 4.5, 6.5. The child chain fuses the
    // first three; the full chain adds the fourth. Pre-evaluating the child
    // extracts a, b and c; the full chain must reuse all three and extract only
    // the new tool d.
    let unit = || box_solid(1.0, 1.0, 1.0);
    let a = part_row(unit(), 0.5, 0.5, 0.5);
    let b = part_row(unit(), 2.5, 0.5, 0.5);
    let c = part_row(unit(), 4.5, 0.5, 0.5);
    let d = part_row(unit(), 6.5, 0.5, 0.5);
    let child = fuse_chain(a.clone(), &[b.clone(), c.clone()]);
    let full = fuse_chain(a, &[b, c, d]);

    let records = facts_sequence(&[child, full], &[0, 1]);
    let child_record = &records[0];
    let full_record = &records[1];
    assert_eq!(
        timing_u64(child_record, "patch_cache_hits"),
        0,
        "the child chain is evaluated cold: {child_record}"
    );
    assert!(
        timing_u64(full_record, "patch_cache_hits") >= 3,
        "the full chain must reuse a, b and c: {full_record}"
    );
    let volume = full_record["volume"].as_f64().expect("full volume");
    assert!(
        (volume - 4.0).abs() < 1.0e-4,
        "four disjoint unit boxes must union to 4, got {volume}: {full_record}"
    );
}

// ---------------------------------------------------------------------------
// Test 4: the None-deflection vehicle fingerprint is unchanged.
// ---------------------------------------------------------------------------

#[test]
fn fingerprint_unchanged() {
    let dir = scratch_dir();
    let stl = dir.join("facts_cache_vehicle.stl");
    let door = corpus_ttc_dir().join("door.py");
    let tree = corpus_ttc_dir()
        .join("trees")
        .join("falcon_heavy")
        .join("src");

    let output = python_command()
        .arg(&door)
        .args(["--engine", "truck"])
        .arg(&tree)
        .args([
            "lib.falcon_common",
            "build_vehicle",
            "[]",
            stl.to_str().expect("stl path"),
        ])
        .output()
        .expect("spawn the corpus door");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    assert!(
        output.status.success(),
        "vehicle door failed (rc {}):\nstdout:{stdout}\nstderr:{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let record: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("the door record must parse");
    assert_eq!(
        record["ok"],
        serde_json::json!(true),
        "door record: {record}"
    );
    assert_eq!(
        record["facts"]["solid_count"],
        serde_json::json!(2142),
        "the full vehicle carries 2,142 parts"
    );
    assert_eq!(
        record["stl"]["triangles"],
        serde_json::json!(4_461_816_u64),
        "the None-deflection vehicle fingerprint must be unchanged"
    );

    let _ = std::fs::remove_file(&stl);
}

// ---------------------------------------------------------------------------
// Test 5: the cache population order is invisible in every output.
// ---------------------------------------------------------------------------

#[test]
fn cache_order_invisible() {
    let trees = vec![
        fuse_chain(
            part_row(box_solid(1.0, 1.0, 1.0), 0.5, 0.5, 0.5),
            &[
                part_row(box_solid(1.0, 1.0, 1.0), 2.5, 0.5, 0.5),
                part_row(box_solid(1.0, 1.0, 1.0), 4.5, 0.5, 0.5),
            ],
        ),
        fuse_chain(
            part_row(box_solid(1.0, 1.0, 1.0), 0.5, 0.5, 0.5),
            &[
                part_row(box_solid(1.0, 1.0, 1.0), 2.5, 0.5, 0.5),
                part_row(box_solid(1.0, 1.0, 1.0), 4.5, 0.5, 0.5),
                part_row(box_solid(1.0, 1.0, 1.0), 6.5, 0.5, 0.5),
            ],
        ),
        serde_json::json!({
            "group": [
                part_row(box_solid(1.0, 1.0, 1.0), 0.5, 0.5, 0.5),
                part_row(box_solid(2.0, 2.0, 2.0), 5.0, 0.5, 0.5),
            ]
        }),
    ];

    let forward: Vec<serde_json::Value> = facts_node_order(&trees, &[0, 1, 2])
        .iter()
        .map(strip_timing)
        .collect();
    let shuffled: Vec<serde_json::Value> = facts_node_order(&trees, &[2, 0, 1])
        .iter()
        .map(strip_timing)
        .collect();
    assert_eq!(
        forward, shuffled,
        "the cache population order must never reach an output"
    );
}
