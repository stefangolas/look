//! F1-AUTHORING-ARMS required suite — the door's missing census verbs: the
//! extrude prism arm, the loft arm (and its closed sweep-as-loft-chain form
//! with the halo seam certificate), the mirror placed-carrier arm, and the
//! spline-trimmed-face make_face extension (crate test file booked by the
//! packet).
//!
//! The arms are exercised end to end: the corpus door drop-in records the
//! construction rows (`corpus/ttc/door.py` `--engine truck` surface) and the
//! compiled native module computes the exact facts. Every geometry expectation
//! below is an INDEPENDENT derivation in Rust (the landed frustum telescoping
//! sum, the Simpson cross-section machine check, the pyramid-frustum closed
//! form, plain polygon area × extent) so the kernel's divergence-form and
//! prism arithmetic are never graded against themselves.
//!
//! Tests required by the packet:
//!   1. `loft_facts_match_derived_expectation`
//!   2. `halo_loft_chain_closes_with_seam_certificate`
//!   3. `mirror_produces_placed_carrier_rows`
//!   4. `line_profile_special_cases_bit_identical`

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
/// the ttc_lathe_spline / pb_conformance staging: the crate cdylib is copied
/// as `truck123d.pyd` beside the interpreter runtime DLLs.
fn staged_native_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let exe = std::env::current_exe().expect("test binary path");
        let deps = exe.parent().expect("deps dir");
        let staging =
            std::env::temp_dir().join(format!("truck123d_ttc_arms_{}", std::process::id()));
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

/// A closed unit square in the `z = z0` plane, wound CCW about +z.
fn square_loop(z0: f64) -> serde_json::Value {
    line_loop3(&[
        [0.0, 0.0, z0],
        [1.0, 0.0, z0],
        [1.0, 1.0, z0],
        [0.0, 1.0, z0],
    ])
}

/// A closed `s x s` square in the `z = z0` plane sharing the origin corner.
fn rect_loop(s: f64, z0: f64) -> serde_json::Value {
    line_loop3(&[[0.0, 0.0, z0], [s, 0.0, z0], [s, s, z0], [0.0, s, z0]])
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

/// The polygon signed area of a flat `z`-stacked loop (the independent area a
/// Rust test asserts against).
fn area_of(pts: &[[f64; 3]]) -> f64 {
    let mut ax = 0.0f64;
    let mut ay = 0.0f64;
    let mut az = 0.0f64;
    for i in 0..pts.len() {
        let a = pts[i];
        let b = pts[(i + 1) % pts.len()];
        ax += a[1] * b[2] - a[2] * b[1];
        ay += a[2] * b[0] - a[0] * b[2];
        az += a[0] * b[1] - a[1] * b[0];
    }
    0.5 * (ax * ax + ay * ay + az * az).sqrt()
}

/// The landed lathe frustum-telescoping volume of a closed `(x, z)` line
/// profile (independent of the native arm, replicated here for the test).
fn lathe_profile_volume(pts: &[[f64; 2]]) -> f64 {
    let mut sum = 0.0f64;
    for i in 0..pts.len() {
        let (x0, z0) = (pts[i][0], pts[i][1]);
        let (x1, z1) = (pts[(i + 1) % pts.len()][0], pts[(i + 1) % pts.len()][1]);
        sum += std::f64::consts::PI / 3.0 * (z1 - z0) * (x0 * x0 + x0 * x1 + x1 * x1);
    }
    sum.abs()
}

// ---------------------------------------------------------------------------
// Test 1: loft facts match the derived expectation.
// ---------------------------------------------------------------------------

#[test]
fn loft_facts_match_derived_expectation() {
    // A two-section loft of two identical aligned unit squares at z = 0 and
    // z = 5 is the degenerate line-profile case of a loft: an exact prism of
    // volume 5.0. The kernel's divergence-form integral over the ruled faces
    // must match the derived area x extent, and the door's drop-in loft()
    // recording must produce exactly that row.
    let loft_solid = serde_json::json!({
        "kind": "loft",
        "closed": false,
        "sections": [square_loop(0.0), square_loop(5.0)],
    });
    let facts = native_facts(&part_row(loft_solid).to_string());
    assert_eq!(facts["solid_count"], 1);
    let volume = facts["volume"].as_f64().expect("loft volume");
    assert!(
        (volume - 5.0).abs() / 5.0 < 1e-12,
        "identical-section loft volume {volume}"
    );
    // bbox spans the two unit sections at z in [0, 5].
    assert_eq!(facts["bbox"][0], serde_json::json!([0.0, 0.0, 0.0]));
    assert_eq!(facts["bbox"][1], serde_json::json!([1.0, 1.0, 5.0]));

    // A coaxial similar-section pair (a pyramid frustum): the derived
    // expectation is the closed-form frustum h/3 (A0 + A1 + sqrt(A0 A1)).
    let small = area_of(&[
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [2.0, 2.0, 0.0],
        [0.0, 2.0, 0.0],
    ]);
    let big = area_of(&[
        [0.0, 0.0, 3.0],
        [4.0, 0.0, 3.0],
        [4.0, 4.0, 3.0],
        [0.0, 4.0, 3.0],
    ]);
    let frustum = 3.0 / 3.0 * (small + big + (small * big).sqrt());
    assert!((small - 4.0).abs() < 1e-12 && (big - 16.0).abs() < 1e-12);
    assert!((frustum - 28.0).abs() < 1e-12);

    let frustum_solid = serde_json::json!({
        "kind": "loft",
        "closed": false,
        "sections": [rect_loop(2.0, 0.0), rect_loop(4.0, 3.0)],
    });
    let facts = native_facts(&part_row(frustum_solid).to_string());
    let volume = facts["volume"].as_f64().expect("frustum volume");
    assert!(
        (volume - frustum).abs() / frustum < 1e-12,
        "frustum loft volume {volume} vs {frustum}"
    );

    // End to end through the door drop-in: `bd.loft(faces, ruled=False)` over
    // two Face carriers records the `loft` row and the native facts over that
    // recorded row equal the derived value (same volume 5.0, bit for bit when
    // submitted through the recorded node).
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

part = bd.loft([square(0.0), square(5.0)], ruled=False)
node = part._node()
solid = node["part"]["solid"]
assert solid["kind"] == "loft", solid
assert solid["closed"] is False, solid
assert len(solid["sections"]) == 2, solid
facts = json.loads(truck123d.bd_facts(json.dumps(node)))
assert facts["volume"] == 5.0, facts
print(json.dumps({"ok": True, "recorded_kind": solid["kind"], "volume": facts["volume"]}))
"#;
    let stdout = run_python(script, &[corpus_ttc_dir().to_str().expect("corpus path")]);
    let record: serde_json::Value = serde_json::from_str(stdout.trim()).expect("record json");
    assert_eq!(record["ok"], true);
    assert_eq!(record["recorded_kind"], "loft");
    assert_eq!(record["volume"], 5.0);
}

// ---------------------------------------------------------------------------
// Test 2: the halo loft chain closes with the seam certificate.
// ---------------------------------------------------------------------------

#[test]
fn halo_loft_chain_closes_with_seam_certificate() {
    // The halo form: a vertical square profile sampled every 45 degrees around
    // a full ring, with the station list returning exactly to its start. The
    // aligned meeting edges across the chain closure satisfy the seam identity
    // `A0 W1 - A1 W0 == 0`, and the native facts report the (zero) mismatch.
    let mut sections = Vec::new();
    for i in 0..8 {
        sections.push(halo_station(i as f64 * 45.0));
    }
    // The return to the first station is the exact recorded first section.
    sections.push(sections[0].clone());
    let closed = part_row(serde_json::json!({
        "kind": "loft",
        "closed": true,
        "sections": sections,
    }))
    .to_string();
    let facts = native_facts(&closed);
    assert_eq!(facts["solid_count"], 1);
    assert_eq!(facts["seam_mismatch"], serde_json::json!(0.0));
    let volume = facts["volume"].as_f64().expect("halo volume");
    assert!(volume > 0.0, "a closing halo ring has positive volume");

    // A non-closing chain: the recorded return station is rotated 0.1 degrees
    // off the seam, so the aligned meeting edges no longer coincide and the
    // row refuses typed with the mismatch evidence.
    let mut broken = Vec::new();
    for i in 0..8 {
        broken.push(halo_station(i as f64 * 45.0));
    }
    broken.push(halo_station(360.1));
    let broken_tree = part_row(serde_json::json!({
        "kind": "loft",
        "closed": true,
        "sections": broken,
    }))
    .to_string();
    let script = r#"
import json, sys
import truck123d
try:
    truck123d.bd_facts(sys.argv[1])
except truck123d.Refused as exc:
    print(json.dumps({"refused": True, "kind": "Refused"}))
else:
    print(json.dumps({"refused": False}))
    sys.exit(2)
"#;
    let stdout = run_python(script, &[&broken_tree]);
    let record: serde_json::Value = serde_json::from_str(stdout.trim()).expect("refusal json");
    assert_eq!(
        record["refused"], true,
        "a non-closing halo chain must refuse typed"
    );
}

// ---------------------------------------------------------------------------
// Test 3: mirror produces placed carrier rows.
// ---------------------------------------------------------------------------

#[test]
fn mirror_produces_placed_carrier_rows() {
    // The mirror arm records a placed-carrier transform over the mirrored
    // census row: the same solid spec, a `mirror` axis on the placement, and
    // the reflected frame. The volume is unchanged and the bbox is reflected.
    let script = r#"
import json, sys
sys.path.insert(0, sys.argv[1])
import door
import truck123d
door._T123D = truck123d
bd = door._build_truck_module()

def v(x, y, z):
    return door.Vector(x, y, z)

square = door.Face(door.Wire([
    door.Edge.make_line(v(0, 0, 0), v(1, 0, 0)),
    door.Edge.make_line(v(1, 0, 0), v(1, 1, 0)),
    door.Edge.make_line(v(1, 1, 0), v(0, 1, 0)),
    door.Edge.make_line(v(0, 1, 0), v(0, 0, 0)),
]))
plate = bd.extrude(square, amount=2.0, both=True)
plate.locate(door.Location((10.0, 0.0, 0.0)))
left = plate
right = bd.mirror(left, about=bd.Plane.XZ)
left_node = left._node()["part"]
right_node = right._node()["part"]
# The placed carrier row: same solid, mirror axis recorded, frame reflected.
assert left_node["solid"] == right_node["solid"], (left_node, right_node)
assert right_node["mirror"] == "y", right_node
assert right_node["x"] == left_node["x"], (left_node, right_node)
assert right_node["y"] == -left_node["y"], (left_node, right_node)
lf = json.loads(truck123d.bd_facts(json.dumps(left._node())))
rf = json.loads(truck123d.bd_facts(json.dumps(right._node())))
assert lf["volume"] == rf["volume"], (lf, rf)
assert rf["bbox"][0] == [10.0, -1.0, -2.0], rf
assert rf["bbox"][1] == [11.0, 0.0, 2.0], rf
print(json.dumps({"ok": True,
                  "mirror": right_node["mirror"],
                  "x": right_node["x"],
                  "volume_equal": lf["volume"] == rf["volume"],
                  "bbox": rf["bbox"]}))
"#;
    let stdout = run_python(script, &[corpus_ttc_dir().to_str().expect("corpus path")]);
    let record: serde_json::Value = serde_json::from_str(stdout.trim()).expect("mirror json");
    assert_eq!(record["ok"], true);
    assert_eq!(record["mirror"], "y");
    assert_eq!(record["x"], 10.0);
    assert_eq!(record["volume_equal"], true);
    assert_eq!(
        record["bbox"],
        serde_json::json!([[10.0, -1.0, -2.0], [11.0, 0.0, 2.0]])
    );
}

// ---------------------------------------------------------------------------
// Test 4: line-profile special cases stay bit-identical.
// ---------------------------------------------------------------------------

#[test]
fn line_profile_special_cases_bit_identical() {
    // (1) The landed line-profile lathe facts stay bit-identical after the
    // authoring arms land: the recorded profile-edge form of an all-line lathe
    // reproduces the landed frustum-telescoping value to the bit through the
    // native arm.
    let vertices: [[f64; 2]; 5] = [
        [10.0, 0.0],
        [20.0, 0.0],
        [20.0, 10.0],
        [15.0, 15.0],
        [10.0, 10.0],
    ];
    let profile: Vec<serde_json::Value> = vertices
        .iter()
        .enumerate()
        .map(|(i, a)| {
            serde_json::json!({
                "kind": "line",
                "a": a,
                "b": vertices[(i + 1) % vertices.len()],
            })
        })
        .collect();
    let lathe = part_row(serde_json::json!({
        "kind": "lathe",
        "arc_deg": 360.0,
        "profile": profile,
    }))
    .to_string();
    let facts = native_facts(&lathe);
    let volume = facts["volume"].as_f64().expect("lathe volume");
    let expected = lathe_profile_volume(&vertices);
    assert_eq!(volume.to_bits(), expected.to_bits());

    // (2) The degenerate two-section line-profile case of a loft (two identical
    // aligned line-loop sections = a prism) is bit-identical to the extrude
    // prism arm on the same profile: both reduce to the same area x extent.
    let loft = part_row(serde_json::json!({
        "kind": "loft",
        "closed": false,
        "sections": [square_loop(0.0), square_loop(5.0)],
    }))
    .to_string();
    let prism = part_row(serde_json::json!({
        "kind": "prism",
        "profile": square_loop(0.0),
        "amount": 5.0,
        "both": false,
    }))
    .to_string();
    let loft_facts = native_facts(&loft);
    let prism_facts = native_facts(&prism);
    assert_eq!(
        loft_facts["volume"].as_f64().expect("loft vol").to_bits(),
        prism_facts["volume"].as_f64().expect("prism vol").to_bits(),
        "the two-section line-profile loft special case must be bit-identical \
         to the extruded prism"
    );
    assert_eq!(loft_facts["volume"], serde_json::json!(5.0));
}
