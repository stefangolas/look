//! FHC-D required suite — the last uncovered admission surface.
//!
//! The admission-surface audit (`scratch/admission_surface_audit.py`) left
//! exactly one residual: the four build123d class names the hypercar rows use
//! as anchors (`Part` / `Rectangle` / `Solid` / `Shape`), the six OCC-probe
//! methods of the hypercar OCP block (`IsIdentity` / `Located` / `Mass` /
//! `ShapeType` / `Transformation` / `VolumeProperties_s`) and the data-row
//! attribute `vertices`. Every one is now answered by the drop-in surface from
//! landed facts — never an OCC geometry consultation, never an untyped
//! `AttributeError`/`TypeError`:
//!
//!   * `Rectangle` is the unrounded sibling of the landed `RectangleRounded`
//!     vocabulary: four exact line edges (the same closed line loop the
//!     polygon/prism handlers already certify);
//!   * `Part` / `Solid` / `Shape` are constructor/anchor rows: a recorded row
//!     anchors to itself, an OCC shape refuses TYPED (a kernel-engine row has
//!     no OCC geometry);
//!   * `IsIdentity` / `Transformation` answer from the recorded placement
//!     algebra (the frame's own origin and orthonormal columns, MONO-3);
//!   * `Located` composes the landed placement algebra over a row;
//!   * `ShapeType` answers the recorded carrier kind and `Mass` /
//!     `VolumeProperties_s` the already-computed certified volume;
//!   * `vertices` answers the recorded section/edge samples (G3-adjacent);
//!   * a fillet over a vertex selection refuses TYPED (a 2-D vertex blend is
//!     not a landed kernel carrier), never an untyped attribute failure.
//!
//! Tests:
//!
//! 1. `rectangle_profile_is_four_line_edges_and_extrudes_exactly`
//! 2. `part_and_solid_anchor_recorded_rows_and_refuse_typed`
//! 3. `shape_cast_anchors_rows_and_refuses_typed`
//! 4. `frame_identity_and_transformation_answer_the_placement_algebra`
//! 5. `located_composes_the_landed_placement`
//! 6. `shape_type_and_mass_answer_the_recorded_carrier_facts`
//! 7. `volume_properties_s_fills_the_certified_volume`
//! 8. `face_vertices_answer_the_recorded_samples`
//! 9. `fillet_over_a_vertex_selection_refuses_typed`
//! 10. `hypercar_owning_rows_progress_past_the_residue_to_typed_verdicts`

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
        let staging = std::env::temp_dir().join(format!("truck123d_fhc_d_{}", std::process::id()));
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

/// Runs the corpus door with `--engine truck` over one tree/module/entry.
fn run_truck_door(tree: &str, module: &str, entry: &str, args_json: &str) -> serde_json::Value {
    let stl = std::env::temp_dir().join(format!("ttc_fhc_d_{}_{}.stl", std::process::id(), entry));
    let _ = std::fs::remove_file(&stl);
    let output = python_command()
        .arg(corpus_ttc_dir().join("door.py"))
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
    let _ = std::fs::remove_file(&stl);
    serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("door json for {module}.{entry}: {e}\n{stdout}"))
}

/// The common preamble: import the real native module and the corpus door.
const PREAMBLE: &str = r#"
import json, sys
sys.path.insert(0, sys.argv[1])
import truck123d
import door
door._T123D = truck123d
bd = door._build_truck_module()
def facts(node):
    return json.loads(truck123d.bd_facts(json.dumps(node)))
"#;

/// Asserts two floats agree to the landed relative tolerance.
fn assert_relative_close(got: f64, expect: f64, label: &str) {
    assert!(
        (got - expect).abs() <= 1e-9 * expect.abs().max(1.0),
        "{label}: got {got}, expected {expect}"
    );
}

// ---------------------------------------------------------------------------
// Test 1: Rectangle is four line edges and extrudes exactly.
// ---------------------------------------------------------------------------

#[test]
fn rectangle_profile_is_four_line_edges_and_extrudes_exactly() {
    let script = String::from(PREAMBLE)
        + r#"
w, h, amount = 74.0, 206.0, 4.0
face = bd.Rectangle(w, h)
kinds = [edge.kind for edge in face.edges]
part = bd.extrude(face, amount)
print(json.dumps({
    "kinds": kinds,
    "facts": facts(part._node()),
    "expect": w * h * amount,
}))
"#;
    let record = run_script(&script);
    let kinds = record["kinds"].as_array().expect("edge kinds");
    assert_eq!(kinds.len(), 4, "the unrounded rectangle is four edges");
    for kind in kinds {
        assert_eq!(kind.as_str(), Some("line"), "every edge is a line");
    }
    let facts = &record["facts"];
    assert_eq!(facts["solid_count"], 1, "the rectangle extrudes: {facts}");
    let volume = facts["volume"].as_f64().expect("volume");
    let expect = record["expect"].as_f64().expect("expect");
    assert_relative_close(volume, expect, "rectangle prism volume");
    assert_eq!(
        facts["bbox"],
        serde_json::json!([[-37.0, -103.0, 0.0], [37.0, 103.0, 4.0]]),
        "the rectangle is centered on its local origin"
    );
}

// ---------------------------------------------------------------------------
// Test 2: Part / Solid anchor recorded rows and refuse OCC typed.
// ---------------------------------------------------------------------------

#[test]
fn part_and_solid_anchor_recorded_rows_and_refuse_typed() {
    let script = String::from(PREAMBLE)
        + r#"
box = bd.Box(2.0, 3.0, 4.0)
out = {
    "part_anchor": bd.Part(box) is box,
    "solid_anchor": bd.Solid(box) is box,
    "solid_make_loft": hasattr(bd.Solid, "make_loft"),
    "in_list": [n for n in ("Part", "Solid") if n in door._TRUCK_NAMES],
}
for name, ctor in (("Part", bd.Part), ("Solid", bd.Solid)):
    try:
        ctor(object())
        out[name] = {"refused": False}
    except truck123d.Refused as exc:
        out[name] = {"refused": True, "message": str(exc)}
    except Exception as exc:
        out[name] = {"refused": False, "untyped": type(exc).__name__}
print(json.dumps(out))
"#;
    let record = run_script(&script);
    assert_eq!(record["part_anchor"], true, "Part anchors a recorded row");
    assert_eq!(record["solid_anchor"], true, "Solid anchors a recorded row");
    assert_eq!(record["solid_make_loft"], true, "Solid keeps make_loft");
    assert_eq!(record["in_list"], serde_json::json!(["Part", "Solid"]));
    for name in ["Part", "Solid"] {
        assert_eq!(
            record[name]["refused"], true,
            "{name} over an OCC shape must refuse TYPED: {record}"
        );
    }
}

// ---------------------------------------------------------------------------
// Test 3: Shape.cast anchors recorded rows and refuses OCC typed.
// ---------------------------------------------------------------------------

#[test]
fn shape_cast_anchors_rows_and_refuses_typed() {
    let script = String::from(PREAMBLE)
        + r#"
box = bd.Box(1.0, 1.0, 1.0)
out = {"anchor": bd.Shape.cast(box) is box}
try:
    bd.Shape.cast(object())
    out["refused"] = False
except truck123d.Refused as exc:
    out["refused"] = True
    out["message"] = str(exc)
except Exception as exc:
    out["refused"] = False
    out["untyped"] = type(exc).__name__
print(json.dumps(out))
"#;
    let record = run_script(&script);
    assert_eq!(record["anchor"], true, "Shape.cast anchors a recorded row");
    assert_eq!(
        record["refused"], true,
        "Shape.cast over an OCC shape must refuse TYPED: {record}"
    );
}

// ---------------------------------------------------------------------------
// Test 4: IsIdentity / Transformation answer the placement algebra.
// ---------------------------------------------------------------------------

#[test]
fn frame_identity_and_transformation_answer_the_placement_algebra() {
    let script = String::from(PREAMBLE)
        + r#"
ident = door.Pos(0.0, 0.0, 0.0)
moved = door.Pos(1.0, 2.0, 3.0)
rot = door.Rotation(0.0, 0.0, 90.0)
print(json.dumps({
    "ident": ident.IsIdentity(),
    "moved": moved.IsIdentity(),
    "trans_origin": rot.Transformation()["origin"],
    "trans_z": rot.Transformation()["z_dir"],
    "trans_x": rot.Transformation()["x_dir"],
}))
"#;
    let record = run_script(&script);
    assert_eq!(record["ident"], true, "the identity frame is identity");
    assert_eq!(record["moved"], false, "a translated frame is not identity");
    assert_eq!(record["trans_origin"], serde_json::json!([0.0, 0.0, 0.0]));
    assert_eq!(record["trans_z"], serde_json::json!([0.0, 0.0, 1.0]));
    let x = record["trans_x"].as_array().expect("x dir");
    assert_relative_close(x[0].as_f64().expect("x"), 0.0, "rotated x.x");
    assert_relative_close(x[1].as_f64().expect("x"), 1.0, "rotated x.y");
}

// ---------------------------------------------------------------------------
// Test 5: Located composes the landed placement.
// ---------------------------------------------------------------------------

#[test]
fn located_composes_the_landed_placement() {
    let script = String::from(PREAMBLE)
        + r#"
box = bd.Box(2.0, 3.0, 4.0)
placed = box.Located(bd.Pos(10.0, 0.0, 0.0))
out = {
    "base": facts(box._node())["bbox"],
    "placed": facts(placed._node())["bbox"],
    "base_volume": facts(box._node())["volume"],
    "placed_volume": facts(placed._node())["volume"],
}
try:
    box.Located(object())
    out["refused"] = False
except truck123d.Refused as exc:
    out["refused"] = True
except Exception as exc:
    out["refused"] = False
    out["untyped"] = type(exc).__name__
print(json.dumps(out))
"#;
    let record = run_script(&script);
    assert_eq!(
        record["placed"],
        serde_json::json!([[9.0, -1.5, -2.0], [11.0, 1.5, 2.0]]),
        "Located shifts the world enclosure"
    );
    assert_eq!(record["base_volume"], record["placed_volume"]);
    assert_eq!(record["refused"], true, "Located refuses a non-frame typed");
}

// ---------------------------------------------------------------------------
// Test 6: ShapeType / Mass answer the recorded carrier facts.
// ---------------------------------------------------------------------------

#[test]
fn shape_type_and_mass_answer_the_recorded_carrier_facts() {
    let script = String::from(PREAMBLE)
        + r#"
box = bd.Box(2.0, 3.0, 4.0)
group = bd.Compound(children=[box])
print(json.dumps({
    "solid_type": box.ShapeType(),
    "compound_type": group.ShapeType(),
    "mass": box.Mass(),
    "volume": facts(box._node())["volume"],
}))
"#;
    let record = run_script(&script);
    assert_eq!(record["solid_type"], "solid");
    assert_eq!(record["compound_type"], "compound");
    assert_eq!(record["mass"], 24.0, "Mass is the certified volume");
    assert_eq!(record["mass"], record["volume"]);
}

// ---------------------------------------------------------------------------
// Test 7: VolumeProperties_s fills the certified volume.
// ---------------------------------------------------------------------------

#[test]
fn volume_properties_s_fills_the_certified_volume() {
    let script = String::from(PREAMBLE)
        + r#"
box = bd.Box(2.0, 3.0, 4.0)
class Props:
    pass
props = Props()
box.VolumeProperties_s(props)
print(json.dumps({"volume": props._volume, "expect": facts(box._node())["volume"]}))
"#;
    let record = run_script(&script);
    assert_eq!(record["volume"], 24.0);
    assert_eq!(record["volume"], record["expect"]);
}

// ---------------------------------------------------------------------------
// Test 8: Face.vertices answers the recorded samples.
// ---------------------------------------------------------------------------

#[test]
fn face_vertices_answer_the_recorded_samples() {
    let script = String::from(PREAMBLE)
        + r#"
face = bd.Rectangle(2.0, 4.0)
verts = face.vertices()
print(json.dumps({
    "count": len(verts),
    "coords": sorted([[v.x, v.y] for v in verts]),
    "vectors": all(isinstance(v, door.Vector) for v in verts),
}))
"#;
    let record = run_script(&script);
    assert_eq!(
        record["count"], 4,
        "a rectangle records four unique vertices"
    );
    assert_eq!(record["vectors"], true);
    assert_eq!(
        record["coords"],
        serde_json::json!([[-1.0, -2.0], [-1.0, 2.0], [1.0, -2.0], [1.0, 2.0]])
    );
}

// ---------------------------------------------------------------------------
// Test 9: a fillet over a vertex selection refuses TYPED.
// ---------------------------------------------------------------------------

#[test]
fn fillet_over_a_vertex_selection_refuses_typed() {
    let script = String::from(PREAMBLE)
        + r#"
face = bd.Rectangle(2.0, 4.0)
try:
    bd.fillet(face.vertices(), 0.5)
    out = {"refused": False}
except truck123d.Refused as exc:
    out = {"refused": True, "message": str(exc)}
except Exception as exc:
    out = {"refused": False, "untyped": type(exc).__name__}
print(json.dumps(out))
"#;
    let record = run_script(&script);
    assert_eq!(
        record["refused"], true,
        "a 2-D vertex blend is not a landed carrier: {record}"
    );
}

// ---------------------------------------------------------------------------
// Test 10: the owning hypercar rows progress past the residue to typed verdicts.
// ---------------------------------------------------------------------------

#[test]
fn hypercar_owning_rows_progress_past_the_residue_to_typed_verdicts() {
    // Every owning row of the residue is now past it: no untyped `Pos expects a
    // position`, `'Part'`/`'Solid'`/`'Rectangle'` name failure or `vertices`
    // attribute failure -- each reaches its next honest verdict, a TYPED kernel
    // refusal.
    for module in [
        "lib.wheels",
        "lib.chassis",
        "lib.suspension_front",
        "lib.brakes",
    ] {
        let record = run_truck_door("hypercar", module, "build", "[]");
        assert_eq!(
            record["ok"], false,
            "{module} is not green under the landed machinery: {record}"
        );
        assert_eq!(
            record["error"]["kind"], "Refused",
            "{module} must refuse TYPED: {record}"
        );
        assert_eq!(
            record["error"]["typed"], true,
            "{module} refusal must be typed: {record}"
        );
        let message = record["error"]["message"].as_str().unwrap_or("");
        assert!(
            !message.contains("Pos expects a position")
                && !message.contains("has no attribute")
                && !message.contains("takes no arguments"),
            "{module} must not die on an untyped residue failure: {record}"
        );
    }
}
