//! AUTHOR-EXT-FILLET-HALO required suite — the fillet recording arm and the
//! closed-loop loft certificate.
//!
//! The arms are exercised end to end through the compiled native module
//! (`truck123d.bd_facts`), exactly as the corpus door drives them: the
//! recorded construction tree JSON goes in and the exact facts (or the typed
//! `Refused` exception) come out. The closed-loop arm is the landed
//! halo seam certificate (`A0 W1 - A1 W0 == 0` on aligned meeting edges); the
//! fillet arm resolves the recorded edge selectors against the base's own
//! recorded edge vocabulary and certifies the blend through the landed blend
//! profile machinery. A blend that cannot close, an edge the base cannot
//! resolve, and a base outside the depth-1 rule all refuse typed.
//!
//! Tests:
//!   1. `closed_halo_loop_certifies_zero_seam`
//!   2. `closed_halo_loop_with_nudged_section_refuses_seam`
//!   3. `fillet_of_box_edge_set_certifies_blend`
//!   4. `fillet_with_unresolvable_edge_refuses_carrier`
//!   5. `fillet_of_group_refuses_depth_one`
//!   6. `fillet_door_arm_records_base_and_edges`
//!   7. `halo_door_closed_loft_records_closed_flag`

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

/// One-time preparation of the native-module staging directory (python
/// subprocesses need `truck123d` importable from a real interpreter). Mirrors
/// the ttc_authoring_arms / ttc_lathe_spline staging: the crate cdylib is
/// copied as `truck123d.pyd` beside the interpreter runtime DLLs.
fn staged_native_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let exe = std::env::current_exe().expect("test binary path");
        let deps = exe.parent().expect("deps dir");
        let staging =
            std::env::temp_dir().join(format!("truck123d_fillet_halo_{}", std::process::id()));
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

/// Whether the native `bd_facts` of one tree raises the typed `Refused`.
fn native_refuses(tree_json: &str) -> bool {
    let script = r#"
import sys
import truck123d
try:
    truck123d.bd_facts(sys.argv[1])
except truck123d.Refused:
    print("refused")
else:
    print("ok")
"#;
    run_python(script, &[tree_json]).trim() == "refused"
}

// ---------------------------------------------------------------------------
// Fixture builders
// ---------------------------------------------------------------------------

/// One placed part row around a solid.
fn part_row(solid: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "part": { "solid": solid, "x": 0.0, "y": 0.0, "z": 0.0, "rz": 0.0 }
    })
}

/// A closed 3-D line-loop profile from `(x, y, z)` vertices.
fn line_loop3(pts: &[[f64; 3]]) -> serde_json::Value {
    let mut edges = Vec::new();
    for i in 0..pts.len() {
        edges.push(serde_json::json!({
            "kind": "line",
            "a": pts[i],
            "b": pts[(i + 1) % pts.len()],
        }));
    }
    serde_json::json!(edges)
}

/// A halo-ring station: a vertical square profile (radial extent 4..5, height
/// -1..1) at azimuth `deg` around the z axis.
fn halo_station(deg: f64) -> serde_json::Value {
    let th = deg.to_radians();
    let (c, s) = (th.cos(), th.sin());
    let mut pts = Vec::new();
    for (r, h) in [(4.0, -1.0), (5.0, -1.0), (5.0, 1.0), (4.0, 1.0)] {
        pts.push([r * c, r * s, h]);
    }
    line_loop3(&pts)
}

/// One recorded fillet row over a box base with the given edge selectors.
fn fillet_row(radius: f64, edges: &[[[f64; 3]; 2]]) -> serde_json::Value {
    let selectors: Vec<serde_json::Value> = edges
        .iter()
        .map(|[a, b]| serde_json::json!({ "a": a, "b": b }))
        .collect();
    serde_json::json!({
        "fillet": {
            "base": part_row(serde_json::json!({
                "kind": "box",
                "length": 2.0,
                "width": 2.0,
                "height": 2.0,
            })),
            "radius": radius,
            "edges": selectors,
        }
    })
}

// ---------------------------------------------------------------------------
// Test 1: a closing halo loop certifies its seam.
// ---------------------------------------------------------------------------

#[test]
fn closed_halo_loop_certifies_zero_seam() {
    // The halo form: a vertical square profile sampled every 45 degrees around
    // a full ring, with the station list returning exactly to its start. The
    // aligned meeting edges across the chain closure satisfy the seam identity
    // and the native facts report the (zero) mismatch.
    let mut sections = Vec::new();
    for i in 0..8 {
        sections.push(halo_station(i as f64 * 45.0));
    }
    // The return to the first station is the exact recorded first section.
    sections.push(sections[0].clone());
    let tree = part_row(serde_json::json!({
        "kind": "loft",
        "closed": true,
        "sections": sections,
    }))
    .to_string();
    let facts = native_facts(&tree);
    assert_eq!(facts["solid_count"], 1);
    assert_eq!(facts["seam_mismatch"], serde_json::json!(0.0));
    let volume = facts["volume"].as_f64().expect("halo volume");
    assert!(volume > 0.0, "a closing halo ring has positive volume");
}

// ---------------------------------------------------------------------------
// Test 2: a nudged section refuses the seam typed.
// ---------------------------------------------------------------------------

#[test]
fn closed_halo_loop_with_nudged_section_refuses_seam() {
    // The recorded return station is rotated 0.1 degrees off the seam, so the
    // aligned meeting edges no longer coincide and the row refuses typed with
    // the mismatch evidence (never a tolerance pass).
    let mut broken = Vec::new();
    for i in 0..8 {
        broken.push(halo_station(i as f64 * 45.0));
    }
    broken.push(halo_station(360.1));
    let tree = part_row(serde_json::json!({
        "kind": "loft",
        "closed": true,
        "sections": broken,
    }))
    .to_string();
    assert!(
        native_refuses(&tree),
        "a non-closing halo chain must refuse typed naming the seam"
    );
}

// ---------------------------------------------------------------------------
// Test 3: a resolvable box edge set certifies the blend.
// ---------------------------------------------------------------------------

#[test]
fn fillet_of_box_edge_set_certifies_blend() {
    // A 2x2x2 box with one recorded edge along x at (y, z) = (-1, -1). The
    // edge is part of the box's recorded edge vocabulary, so the selector
    // resolves and the blend profile machinery certifies the cross field. The
    // measured volume is the base part's (the blend is a local certificate).
    let tree = fillet_row(0.25, &[[[-1.0, -1.0, -1.0], [1.0, -1.0, -1.0]]]).to_string();
    let facts = native_facts(&tree);
    assert_eq!(facts["solid_count"], 1);
    assert_eq!(facts["volume"], serde_json::json!(8.0));
    let blend = &facts["blend"];
    assert_eq!(blend["kind"], "fillet");
    assert_eq!(blend["edges"], 1);
    assert_eq!(blend["radius"], 0.25);
    let width = blend["max_profile_width"]
        .as_f64()
        .expect("a certified profile width");
    assert!(width >= 0.0 && width < 1e-9, "constant field width {width}");

    // Two edges of the same box certify as one blend row over two edges.
    let two = fillet_row(
        0.25,
        &[
            [[-1.0, -1.0, -1.0], [1.0, -1.0, -1.0]],
            [[-1.0, 1.0, -1.0], [1.0, 1.0, -1.0]],
        ],
    )
    .to_string();
    let facts = native_facts(&two);
    assert_eq!(facts["blend"]["edges"], 2);
}

// ---------------------------------------------------------------------------
// Test 4: an unresolvable edge refuses the carrier typed.
// ---------------------------------------------------------------------------

#[test]
fn fillet_with_unresolvable_edge_refuses_carrier() {
    // The selector is not an edge of the recorded box vocabulary, so the base
    // cannot resolve it and the row refuses typed naming the open carrier.
    let tree = fillet_row(0.25, &[[[5.0, 5.0, 5.0], [6.0, 5.0, 5.0]]]).to_string();
    assert!(
        native_refuses(&tree),
        "an edge the recorded base cannot resolve must refuse typed"
    );
}

// ---------------------------------------------------------------------------
// Test 5: a fillet of a group refuses the depth-1 rule.
// ---------------------------------------------------------------------------

#[test]
fn fillet_of_group_refuses_depth_one() {
    // The depth-1 rule: the base must be a part. A group base is the recorded
    // open composition cell and refuses typed naming the carrier.
    let tree = serde_json::json!({
        "fillet": {
            "base": {
                "group": [part_row(serde_json::json!({
                    "kind": "box",
                    "length": 2.0,
                    "width": 2.0,
                    "height": 2.0,
                }))],
            },
            "radius": 0.25,
            "edges": [{ "a": [-1.0, -1.0, -1.0], "b": [1.0, -1.0, -1.0] }],
        }
    })
    .to_string();
    assert!(
        native_refuses(&tree),
        "a fillet of a group must refuse typed (depth-1 only)"
    );
}

// ---------------------------------------------------------------------------
// Test 6: the door's fillet arm records the base and its edge selectors.
// ---------------------------------------------------------------------------

#[test]
fn fillet_door_arm_records_base_and_edges() {
    let script = r#"
import json, sys
sys.path.insert(0, sys.argv[1])
import door
import truck123d
door._T123D = truck123d
bd = door._build_truck_module()

box = bd.Box(2.0, 2.0, 2.0)
edges = box.edges().filter_by("x")
assert len(edges) == 4, len(edges)
out = bd.fillet(edges, 0.25)
node = out._node()
fillet = node["fillet"]
assert fillet["base"]["part"]["solid"]["kind"] == "box", node
assert len(fillet["edges"]) == 4, fillet
facts = json.loads(truck123d.bd_facts(json.dumps(node)))
assert facts["blend"]["edges"] == 4, facts
print(json.dumps({"ok": True, "edges": facts["blend"]["edges"]}))
"#;
    let stdout = run_python(script, &[corpus_ttc_dir().to_str().expect("corpus path")]);
    let record: serde_json::Value = serde_json::from_str(stdout.trim()).expect("record json");
    assert_eq!(record["ok"], true);
    assert_eq!(record["edges"], 4);
}

// ---------------------------------------------------------------------------
// Test 7: the door's closed loft flag honors the loop-closure request.
// ---------------------------------------------------------------------------

#[test]
fn halo_door_closed_loft_records_closed_flag() {
    let script = r#"
import json, sys
sys.path.insert(0, sys.argv[1])
import door
import truck123d
door._T123D = truck123d
bd = door._build_truck_module()

def v(x, y, z):
    return door.Vector(x, y, z)

def square(z):
    pts = [(0.0, 0.0, z), (1.0, 0.0, z), (1.0, 1.0, z), (0.0, 1.0, z)]
    return door.Face(door.Wire([
        door.Edge.make_line(v(*pts[i]), v(*pts[(i + 1) % 4])) for i in range(4)
    ]))

closed = bd.loft([square(0.0), square(5.0), square(0.0)], closed=True)
node = closed._node()
solid = node["part"]["solid"]
assert solid["kind"] == "loft", solid
assert solid["closed"] is True, solid
try:
    bd.loft([square(0.0), square(5.0)], closed=True)
except Exception:
    open_refused = True
else:
    open_refused = False
assert open_refused, "an open chain with closed=True must refuse typed"
print(json.dumps({"ok": True, "closed": solid["closed"]}))
"#;
    let stdout = run_python(script, &[corpus_ttc_dir().to_str().expect("corpus path")]);
    let record: serde_json::Value = serde_json::from_str(stdout.trim()).expect("record json");
    assert_eq!(record["ok"], true);
    assert_eq!(record["closed"], true);
}
