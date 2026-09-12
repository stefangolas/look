//! FHC-G3 required suite — the drop-in data-row attribute surface the corpus's
//! client-layer algebra reads: `Face.faces`, the `Plane` frame attributes
//! (`origin` / `x_dir` / `y_dir` / `z_dir`), `Edge.edge`, and the corner
//! `Face` bounds probe (`surfaces.bbox`).
//!
//! Every answer is marshalling over landed facts, never new geometry: a
//! `Face.faces` iteration is the recorded profile face; the frame attributes
//! are the recorded orthonormal placement columns as Vectors; `Edge.edge` is
//! the recorded kernel edge itself; and the corner `Face` bounds probe replays
//! the recorded boundary into the installed OCC kernel so `BRepBndLib.Add_s`
//! bounds the B-spline control net (the documented control-net ENCLOSURE).
//!
//! Tests:
//!
//! 1. `face_faces_iteration_answers_the_profile_face` — the corpus
//!    `(plane * make_face(wire)).faces()[0]` idiom answers the section face and
//!    the native loft facts over it are exact.
//! 2. `plane_frame_attributes_answer_the_corpus_algebra` — `Plane.origin` and
//!    the frame columns are Vectors; the placed extrude answers world facts.
//! 3. `edge_edge_answers_the_recorded_kernel_edge` — `Spline(...).edge()`
//!    answers the recorded edge and a `Wire([...])` of it builds.
//! 4. `corner_face_bounds_probe_answers_control_net_enclosure` — the corpus
//!    `surfaces.bbox` on a drop-in `Face` answers a finite enclosure that
//!    contains the recorded samples.
//! 5. `claimed_rows_progress_past_the_data_row_attributes` — the four claimed
//!    rows no longer die on the named attribute: `hypercar/details` reaches a
//!    typed `Refused`, and the other rows advance to their next named gap.

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
/// the sibling integration suites: the crate cdylib is copied as
/// `truck123d.pyd` beside the interpreter runtime DLLs.
fn staged_native_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let exe = std::env::current_exe().expect("test binary path");
        let deps = exe.parent().expect("deps dir");
        let staging = std::env::temp_dir().join(format!("truck123d_fhc_g3_{}", std::process::id()));
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

/// Runs the corpus door with `--engine truck` over one family tree; returns
/// the parsed JSON record (ok true or a typed refusal).
fn run_truck_door(family: &str, module: &str, entry: &str, args_json: &str) -> serde_json::Value {
    let stl = std::env::temp_dir().join(format!("ttc_fhc_g3_{}_{}.stl", std::process::id(), entry));
    let _ = std::fs::remove_file(&stl);
    let output = python_command()
        .arg(corpus_ttc_dir().join("door.py"))
        .arg("--engine")
        .arg("truck")
        .arg(corpus_ttc_dir().join("trees").join(family).join("src"))
        .arg(module)
        .arg(entry)
        .arg(args_json)
        .arg(&stl)
        .output()
        .expect("spawn truck door");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let _ = std::fs::remove_file(&stl);
    serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("door json for {module}.{entry}: {e}\n{stdout}"))
}

// ---------------------------------------------------------------------------
// Test 1: Face.faces answers the profile face.
// ---------------------------------------------------------------------------

#[test]
fn face_faces_iteration_answers_the_profile_face() {
    // The corpus `hypercar/details` / `lighting` idiom is
    // `(plane * make_face(wire)).faces()[0]` fed to `loft`. The drop-in records
    // a planar region as one face, so `.faces()[0]` is the recorded face and
    // the native loft facts over the recorded sections are exact.
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

section = bd.Plane.XY * square(0.0)
selected = section.faces()
assert len(selected) == 1, selected
assert selected[0] is section, selected
assert selected[0].edges == section.edges
part = bd.loft([selected[0], bd.Plane.XY.offset(5.0) * square(0.0)])
facts = json.loads(truck123d.bd_facts(json.dumps(part._node())))
assert facts["volume"] == 5.0, facts
assert facts["bbox"][0] == [0.0, 0.0, 0.0], facts
assert facts["bbox"][1] == [1.0, 1.0, 5.0], facts
print(json.dumps({"ok": True, "volume": facts["volume"], "bbox": facts["bbox"]}))
"#;
    let stdout = run_python(script, &[corpus_ttc_dir().to_str().expect("corpus path")]);
    let record: serde_json::Value = serde_json::from_str(stdout.trim()).expect("faces json");
    assert_eq!(record["ok"], true);
    assert_eq!(record["volume"], 5.0);
}

// ---------------------------------------------------------------------------
// Test 2: the Plane frame attributes answer the corpus algebra.
// ---------------------------------------------------------------------------

#[test]
fn plane_frame_attributes_answer_the_corpus_algebra() {
    // The corpus suspension/frame algebra reads `pl.origin`, `pl.x_dir`,
    // `pl.y_dir`, `pl.z_dir` as vectors (`pl.origin + pl.z_dir * t`,
    // `d.dot(pl.x_dir)`, `Plane(origin=pl.origin, x_dir=pl.z_dir, ...)`). The
    // recorded frame answers them as Vectors; the placed extrude still records
    // the exact world prism row.
    let script = r#"
import json, sys
sys.path.insert(0, sys.argv[1])
import door
import truck123d
door._T123D = truck123d
bd = door._build_truck_module()

def v(x, y, z):
    return door.Vector(x, y, z)

wire = door.Wire([
    door.Edge.make_line(v(0, 0, 0), v(1, 0, 0)),
    door.Edge.make_line(v(1, 0, 0), v(1, 1, 0)),
    door.Edge.make_line(v(1, 1, 0), v(0, 1, 0)),
    door.Edge.make_line(v(0, 1, 0), v(0, 0, 0)),
])
plane = door.Plane(origin=(50.0, 0.0, 0.0), x_dir=(0.0, 1.0, 0.0), z_dir=(1.0, 0.0, 0.0))
assert isinstance(plane.origin, door.Vector), type(plane.origin)
assert plane.origin.to_tuple() == (50.0, 0.0, 0.0), plane.origin
assert (plane.origin + plane.z_dir * 3.0).to_tuple() == (53.0, 0.0, 0.0)
assert abs(plane.x_dir.dot(door.Vector(0.0, 1.0, 0.0)) - 1.0) < 1e-12
assert abs(plane.y_dir.dot(door.Vector(0.0, 0.0, 1.0)) - 1.0) < 1e-12
swapped = door.Plane(origin=plane.origin, x_dir=plane.z_dir, z_dir=plane.x_dir)
assert swapped.origin.to_tuple() == (50.0, 0.0, 0.0)
face = plane * bd.make_face(wire)
part = bd.extrude(face, amount=10.0, both=False)
facts = json.loads(truck123d.bd_facts(json.dumps(part._node())))
assert facts["bbox"][0] == [50.0, 0.0, 0.0], facts
assert facts["bbox"][1] == [60.0, 1.0, 1.0], facts
print(json.dumps({"ok": True, "origin": list(plane.origin), "bbox": facts["bbox"]}))
"#;
    let stdout = run_python(script, &[corpus_ttc_dir().to_str().expect("corpus path")]);
    let record: serde_json::Value = serde_json::from_str(stdout.trim()).expect("plane json");
    assert_eq!(record["ok"], true);
    assert_eq!(record["origin"], serde_json::json!([50.0, 0.0, 0.0]));
}

// ---------------------------------------------------------------------------
// Test 3: Edge.edge answers the recorded kernel edge.
// ---------------------------------------------------------------------------

#[test]
fn edge_edge_answers_the_recorded_kernel_edge() {
    // `hypercar/suspension_rear` builds `Wire([Spline(*pts, periodic=True)
    // .edge()])`. The recorded row IS the kernel edge, so `.edge()` answers
    // the row and a wire of it feeds the extrude arm exactly.
    let script = r#"
import json, sys
sys.path.insert(0, sys.argv[1])
import door
import truck123d
door._T123D = truck123d
bd = door._build_truck_module()

def v(x, y, z):
    return door.Vector(x, y, z)

edge = bd.Spline(
    (0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (1.0, 1.0, 0.0), (0.0, 1.0, 0.0),
    periodic=True,
)
assert edge.edge() is edge, edge.edge()
assert edge.edge().kind == "spline"
assert len(bd.Wire([edge.edge()])) == 1

# A line edge answers the same accessor and feeds a closed line-loop extrude.
def line(a, b):
    return door.Edge.make_line(v(*a), v(*b)).edge()
loop = [
    line((0.0, 0.0, 0.0), (1.0, 0.0, 0.0)),
    line((1.0, 0.0, 0.0), (1.0, 1.0, 0.0)),
    line((1.0, 1.0, 0.0), (0.0, 1.0, 0.0)),
    line((0.0, 1.0, 0.0), (0.0, 0.0, 0.0)),
]
part = bd.extrude(bd.make_face(door.Wire(loop)), amount=2.0, both=True)
facts = json.loads(truck123d.bd_facts(json.dumps(part._node())))
assert facts["volume"] == 4.0, facts
print(json.dumps({"ok": True, "volume": facts["volume"], "kind": edge.kind}))
"#;
    let stdout = run_python(script, &[corpus_ttc_dir().to_str().expect("corpus path")]);
    let record: serde_json::Value = serde_json::from_str(stdout.trim()).expect("edge json");
    assert_eq!(record["ok"], true);
    assert_eq!(record["volume"], 4.0);
    assert_eq!(record["kind"], "spline");
}

// ---------------------------------------------------------------------------
// Test 4: the corner Face bounds probe answers the control-net enclosure.
// ---------------------------------------------------------------------------

#[test]
fn corner_face_bounds_probe_answers_control_net_enclosure() {
    // `f1/wheels._loft` and `f1/cockpit._prism_estimate` call
    // `surfaces.bbox(f)` on a drop-in section `Face`. The recorded boundary is
    // replayed into OCC so `BRepBndLib.Add_s` bounds the B-spline control net;
    // the enclosure must contain every recorded sample (OCC parity with the
    // executor enclosure is a diagnostic, not a gate).
    let script = r#"
import json, sys
sys.path.insert(0, sys.argv[1])
sys.path.insert(0, sys.argv[2])
import door
door.ENGINE = "truck"
door.install_truck_alias()
import lib.surfaces as surfaces

samples = [(1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (-1.0, 0.0, 0.0), (0.0, -1.0, 0.0)]
face = door.Face(door.Wire([door.Edge.make_spline(samples, periodic=True)]))
lo, hi = surfaces.bbox(face)
for point in samples:
    for axis in range(3):
        assert lo[axis] <= point[axis] <= hi[axis], (point, lo, hi)
assert lo[0] < -1.0 and hi[0] > 1.0, (lo, hi)
assert lo[1] < -1.0 and hi[1] > 1.0, (lo, hi)
box = face.bounding_box()
assert box.min.x <= -1.0 and box.max.x >= 1.0, (box.min, box.max)
assert box.min.y <= -1.0 and box.max.y >= 1.0, (box.min, box.max)
print(json.dumps({"ok": True, "lo": list(lo), "hi": list(hi)}))
"#;
    let f1_tree = corpus_ttc_dir().join("trees").join("f1").join("src");
    let stdout = run_python(
        script,
        &[
            corpus_ttc_dir().to_str().expect("corpus path"),
            f1_tree.to_str().expect("f1 tree path"),
        ],
    );
    let record: serde_json::Value = serde_json::from_str(stdout.trim()).expect("bbox json");
    assert_eq!(record["ok"], true);
}

// ---------------------------------------------------------------------------
// Test 5: the claimed rows progress past the data-row attributes.
// ---------------------------------------------------------------------------

#[test]
fn claimed_rows_progress_past_the_data_row_attributes() {
    // `hypercar/details` and `lighting` died on `'Face' object has no
    // attribute 'faces'`; with the attribute answered they reach the next
    // carrier. `details` reaches a TYPED kernel refusal (the native boolean/
    // loft admission), never an untyped attribute error.
    let details = run_truck_door("hypercar", "lib.details", "build", "[]");
    assert_eq!(details["ok"], false, "details is still red: {details}");
    assert_eq!(
        details["error"]["kind"], "Refused",
        "details must reach a typed refusal: {details}"
    );

    // `suspension_front` died on `'Plane' object has no attribute 'origin'`;
    // it now reaches the next named drop-in gap (`bd.Color`), not the old
    // attribute error.
    let front = run_truck_door("hypercar", "lib.suspension_front", "build", "[]");
    assert_eq!(front["ok"], false, "suspension_front is still red: {front}");
    assert_ne!(
        front["error"]["message"], "'Plane' object has no attribute 'origin'",
        "the Plane frame attribute must be answered: {front}"
    );

    // `suspension_rear` died on `'Edge' object has no attribute 'edge'`; it now
    // reaches the next named drop-in gap (`Solid.make_loft`), not the old
    // attribute error.
    let rear = run_truck_door("hypercar", "lib.suspension_rear", "build", "[]");
    assert_eq!(rear["ok"], false, "suspension_rear is still red: {rear}");
    assert_ne!(
        rear["error"]["message"], "'Edge' object has no attribute 'edge'",
        "the Edge accessor must be answered: {rear}"
    );

    // `lighting` shares the `Face.faces` carrier with `details`.
    let lighting = run_truck_door("hypercar", "lib.lighting", "build", "[]");
    assert_eq!(lighting["ok"], false, "lighting is still red: {lighting}");
    assert_ne!(
        lighting["error"]["message"], "'Face' object has no attribute 'faces'",
        "the Face.faces iteration must be answered: {lighting}"
    );
}
