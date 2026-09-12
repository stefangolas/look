//! BD-EMIT-MESH-CACHE required suite — memoized tessellation for the GLB/STL
//! emit path.
//!
//! The construction tree is the authority; the GLB and STL artifacts are
//! derived. A tree that stamps the same part many times (the Falcon Heavy's 27
//! byte-identical engines, its identical port/starboard booster groups) must
//! tessellate each unique solid once and reuse it:
//!
//!   1. `mesh_cache_shares_accessors_for_identical_parts` — repeated
//!      `(spec, deflection, color)` parts share one mesh and one accessor pair;
//!      a distinct color gets a distinct mesh/accessor; the artifact's accessor
//!      count is below its node count.
//!   2. `mesh_cache_preserves_node_labels_colors_and_transforms` — one node per
//!      part is preserved with its label, color and per-node transform, and the
//!      material's linearized base color matches the recorded sRGB record (the
//!      `pb_glb_emit` round-trip assertions over a two-copy scene).
//!   3. `mesh_cache_emits_byte_identical_repeatedly` — the same ordered input
//!      produces byte-identical GLB bytes on repeated emits (cold vs warm
//!      cache; only `mesh_ms` may differ).
//!   4. `mesh_cache_none_deflection_fingerprint_and_vehicle_accessors` — the
//!      full-vehicle door run: `bd_stl(None)` reproduces the 4,461,816-triangle
//!      fingerprint bit-for-bit, and the shipped 0.4 mm GLB shares accessors
//!      across the repeated parts (accessor count below the part count).
//!
//! The suite runs the compiled native module from a real interpreter exactly
//! as `mono_row_assembly.rs` does (the crate cdylib is staged as
//! `truck123d.pyd` beside the interpreter runtime DLLs).

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
            std::env::temp_dir().join(format!("truck123d_mesh_cache_{}", std::process::id()));
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
    let dir = std::env::temp_dir().join(format!("truck123d_mesh_cache_out_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

// ---------------------------------------------------------------------------
// Fixture builders (the kernel-row vocabulary, independent of the door).
// ---------------------------------------------------------------------------

/// One placed part row around a solid, with optional label/color and a world
/// translation.
fn part_row(
    solid: serde_json::Value,
    x: f64,
    label: Option<&str>,
    color: Option<&str>,
) -> serde_json::Value {
    let mut node = serde_json::json!({
        "solid": solid,
        "x": x,
        "y": 0.0,
        "z": 0.0,
        "rz": 0.0,
    });
    if let Some(label) = label {
        node["label"] = serde_json::json!(label);
    }
    if let Some(color) = color {
        node["color"] = serde_json::json!(color);
    }
    serde_json::json!({ "part": node })
}

/// A canonical box row (cheap, deterministic, deflection-independent geometry).
fn box_solid() -> serde_json::Value {
    serde_json::json!({
        "kind": "box",
        "length": 2.0,
        "width": 3.0,
        "height": 4.0,
    })
}

// ---------------------------------------------------------------------------
// Native emission helpers.
// ---------------------------------------------------------------------------

/// Runs `bd_glb` (and optionally `bd_stl`) on a tree JSON through the staged
/// native module, returning the parsed result records. `glb`/`stl` are `None`
/// to skip that artifact.
fn emit_via_python(
    tree: &str,
    glb: Option<&Path>,
    stl: Option<&Path>,
    deflection: Option<f64>,
) -> serde_json::Value {
    let script = r#"
import json, sys
import truck123d
with open(sys.argv[1], encoding="utf-8") as fh:
    tree = fh.read()
glb_path, stl_path, deflection = sys.argv[2], sys.argv[3], sys.argv[4]
defl = None if deflection == "none" else float(deflection)
out = {}
if glb_path != "-":
    out["glb"] = json.loads(truck123d.bd_glb(tree, glb_path, defl))
if stl_path != "-":
    out["stl"] = json.loads(truck123d.bd_stl(tree, stl_path, defl))
print(json.dumps(out))
"#;
    let dir = scratch_dir();
    let tree_path = dir.join(format!("tree_{}.json", tree.len()));
    std::fs::write(&tree_path, tree).expect("write the tree JSON");
    let deflection = deflection.map_or_else(|| "none".to_string(), |d| d.to_string());
    let stdout = run_python(
        script,
        &[
            tree_path.to_str().expect("tree path"),
            glb.map_or("-", |p| p.to_str().expect("glb path")),
            stl.map_or("-", |p| p.to_str().expect("stl path")),
            &deflection,
        ],
    );
    serde_json::from_str(stdout.trim()).expect("native emission record")
}

/// Parses the JSON document chunk of a GLB file.
fn glb_document(path: &Path) -> serde_json::Value {
    let bytes = std::fs::read(path).expect("read the GLB");
    assert!(bytes.len() >= 20, "GLB too short");
    let json_length = u32::from_le_bytes(
        bytes
            .get(12..16)
            .expect("GLB json length")
            .try_into()
            .expect("four bytes"),
    ) as usize;
    let end = 20usize.checked_add(json_length).expect("json chunk end");
    let json = bytes.get(20..end).expect("json chunk range");
    serde_json::from_slice(json).expect("GLB JSON document")
}

fn as_array<'a>(value: &'a serde_json::Value, key: &str) -> &'a Vec<serde_json::Value> {
    value
        .get(key)
        .and_then(|v| v.as_array())
        .unwrap_or_else(|| panic!("GLB document is missing the `{key}` array"))
}

// ---------------------------------------------------------------------------
// Test 1: identical parts share one accessor; distinct colors do not.
// ---------------------------------------------------------------------------

#[test]
fn mesh_cache_shares_accessors_for_identical_parts() {
    let dir = scratch_dir();
    let glb = dir.join("shared.glb");
    let mut group = Vec::new();
    for i in 0..10 {
        group.push(part_row(
            box_solid(),
            i as f64 * 10.0,
            Some(&format!("copy_{i}")),
            Some("#3366cc"),
        ));
    }
    group.push(part_row(box_solid(), 1000.0, Some("odd"), Some("#cc3366")));
    let tree = serde_json::json!({ "group": group }).to_string();

    let record = emit_via_python(&tree, Some(&glb), None, Some(0.4));
    assert_eq!(
        record["glb"]["parts"],
        serde_json::json!(11),
        "one node per part is preserved: {record}"
    );
    let document = glb_document(&glb);
    let nodes = as_array(&document, "nodes");
    let meshes = as_array(&document, "meshes");
    let materials = as_array(&document, "materials");
    let accessors = as_array(&document, "accessors");
    assert_eq!(nodes.len(), 11);
    assert_eq!(meshes.len(), 2, "one shared mesh + one distinct-color mesh");
    assert_eq!(materials.len(), 2);
    assert!(
        accessors.len() < nodes.len(),
        "shared accessors: {} accessors for {} nodes",
        accessors.len(),
        nodes.len()
    );

    let shared_mesh = nodes[0]["mesh"].as_u64().expect("node mesh");
    for (i, node) in nodes.iter().take(10).enumerate() {
        assert_eq!(
            node["mesh"].as_u64(),
            Some(shared_mesh),
            "identical part {i} must reference the shared mesh"
        );
    }
    let distinct_mesh = nodes[10]["mesh"].as_u64().expect("distinct node mesh");
    assert_ne!(
        distinct_mesh, shared_mesh,
        "a distinct color must get a distinct mesh"
    );

    let shared_position = meshes[shared_mesh as usize]["primitives"][0]["attributes"]["POSITION"]
        .as_u64()
        .expect("shared POSITION accessor");
    let distinct_position =
        meshes[distinct_mesh as usize]["primitives"][0]["attributes"]["POSITION"]
            .as_u64()
            .expect("distinct POSITION accessor");
    assert_ne!(
        shared_position, distinct_position,
        "distinct colors must not share a POSITION accessor"
    );
}

// ---------------------------------------------------------------------------
// Test 2: node/label/color/transform round-trip over a multi-copy scene.
// ---------------------------------------------------------------------------

#[test]
fn mesh_cache_preserves_node_labels_colors_and_transforms() {
    let dir = scratch_dir();
    let glb = dir.join("roundtrip.glb");
    let group = vec![
        part_row(box_solid(), 0.0, Some("alpha"), Some("#ff0000")),
        part_row(box_solid(), 100.0, Some("beta"), Some("#ff0000")),
        part_row(box_solid(), 200.0, Some("gamma"), Some("#00ff00")),
    ];
    let tree = serde_json::json!({ "group": group }).to_string();
    let record = emit_via_python(&tree, Some(&glb), None, Some(0.4));
    assert_eq!(record["glb"]["parts"], serde_json::json!(3));

    let document = glb_document(&glb);
    let nodes = as_array(&document, "nodes");
    let materials = as_array(&document, "materials");

    // Names ride in insertion order and every node is a scene root.
    let names: Vec<&str> = nodes
        .iter()
        .map(|node| node["name"].as_str().expect("node name"))
        .collect();
    assert_eq!(names, ["alpha", "beta", "gamma"]);
    let scene_nodes: Vec<u64> = document["scenes"][0]["nodes"]
        .as_array()
        .expect("scene nodes")
        .iter()
        .map(|v| v.as_u64().expect("scene node"))
        .collect();
    assert_eq!(scene_nodes, vec![0, 1, 2]);

    // Identical (spec, color) parts share one mesh; the distinct color differs.
    assert_eq!(
        nodes[0]["mesh"], nodes[1]["mesh"],
        "alpha/beta share a mesh"
    );
    assert_ne!(nodes[0]["mesh"], nodes[2]["mesh"], "gamma has its own mesh");

    // Each node carries its own recorded translation (column-major index 12).
    let translations: Vec<f64> = nodes
        .iter()
        .map(|node| node["matrix"][12].as_f64().expect("translation x"))
        .collect();
    assert_eq!(translations, [0.0, 100.0, 200.0]);

    // The material base color is the sRGB -> linear conversion of the record.
    let linear = |srgb: f64| {
        if srgb <= 0.04045 {
            srgb / 12.92
        } else {
            ((srgb + 0.055) / 1.055).powf(2.4)
        }
    };
    let factor = |index: usize| -> Vec<f64> {
        materials[index]["pbrMetallicRoughness"]["baseColorFactor"]
            .as_array()
            .expect("baseColorFactor")
            .iter()
            .map(|v| v.as_f64().expect("component"))
            .collect()
    };
    let red = factor(0);
    assert!((red[0] - linear(1.0)).abs() < 1e-6);
    assert!((red[1] - linear(0.0)).abs() < 1e-6);
    let green = factor(1);
    assert!((green[0] - linear(0.0)).abs() < 1e-6);
    assert!((green[1] - linear(1.0)).abs() < 1e-6);
    assert_eq!(
        nodes[0]["mesh"], nodes[1]["mesh"],
        "the shared mesh references one material"
    );
}

// ---------------------------------------------------------------------------
// Test 3: repeated emit is byte-identical (cold vs warm cache).
// ---------------------------------------------------------------------------

#[test]
fn mesh_cache_emits_byte_identical_repeatedly() {
    let dir = scratch_dir();
    let mut group = Vec::new();
    for i in 0..6 {
        group.push(part_row(
            box_solid(),
            i as f64 * 5.0,
            Some("stamped"),
            Some("#123456"),
        ));
    }
    let tree = serde_json::json!({ "group": group }).to_string();

    let first = dir.join("first.glb");
    let second = dir.join("second.glb");
    let record = emit_via_python(&tree, Some(&first), None, Some(0.4));
    assert_eq!(record["glb"]["parts"], serde_json::json!(6));
    emit_via_python(&tree, Some(&second), None, Some(0.4));

    let first_bytes = std::fs::read(&first).expect("first GLB");
    let second_bytes = std::fs::read(&second).expect("second GLB");
    assert_eq!(
        first_bytes, second_bytes,
        "the same ordered input must emit byte-identical GLBs"
    );
}

// ---------------------------------------------------------------------------
// Test 4: full-vehicle door run — fingerprint + shared accessors.
// ---------------------------------------------------------------------------

#[test]
fn mesh_cache_none_deflection_fingerprint_and_vehicle_accessors() {
    let dir = scratch_dir();
    let stl = dir.join("vehicle.stl");
    let glb = dir.join("vehicle.glb");
    let door = corpus_ttc_dir().join("door.py");
    let tree = corpus_ttc_dir()
        .join("trees")
        .join("falcon_heavy")
        .join("src");

    // The door's truck regime: STL at None (the landed fixed-resolution
    // fingerprint) and the shipped 0.4 mm colored GLB in one run.
    let output = python_command()
        .arg(&door)
        .args(["--engine", "truck"])
        .arg(&tree)
        .args([
            "lib.falcon_common",
            "build_vehicle",
            "[]",
            stl.to_str().expect("stl path"),
            "--glb",
            glb.to_str().expect("glb path"),
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

    let document = glb_document(&glb);
    let nodes = as_array(&document, "nodes");
    let meshes = as_array(&document, "meshes");
    let accessors = as_array(&document, "accessors");
    assert_eq!(nodes.len(), 2142, "one node per placed part is preserved");
    assert!(
        meshes.len() < nodes.len(),
        "the cache must fire on the Falcon Heavy tree: {} meshes for {} nodes",
        meshes.len(),
        nodes.len()
    );
    assert!(
        accessors.len() < nodes.len(),
        "shared accessors: {} accessors for {} nodes",
        accessors.len(),
        nodes.len()
    );

    let _ = std::fs::remove_dir_all(&dir);
}
