//! FHC-MIRROR-FORM required suite — the mirror carrier form.
//!
//! The `f1/drs_actuator` row authors its left bodies with the corpus member
//! vocabulary and mirrors them with `surfaces.mirror_y` (`bd.mirror(shape,
//! Plane.XZ)`). The door records that as a placed-carrier reflection
//! (`PartSpec.mirror`): the local solid is untouched, the translation carries
//! the reflected origin and the recorded rotation stays the original frame.
//!
//! The production placement must therefore apply the rotation FIRST and the
//! reflection AFTER it (`translate(M o) ∘ mirror ∘ R`), which is exactly the
//! world reflection `M ∘ (translate(o) ∘ R)`. The landed order applied the
//! mirror to the local geometry before the rotation (`translate(M o) ∘ R ∘
//! mirror`), which is only an isometry when the rotation commutes with the
//! mirror plane; a rotated, asymmetric row mirrored to the wrong place. This
//! suite pins the exact form:
//!
//! 1. `mirrored_placed_solid_round_trips_green` — a placed solid carrying a
//!    recorded rotation frame mirrors to a certified bracket and an emitted
//!    STL.
//! 2. `mirrored_member_row_round_trips_green` — the kernel SOLID/member
//!    (loft) row the corpus member vocabulary records mirrors green end to
//!    end.
//! 3. `mirrored_rotation_frame_is_exact_isometry` — an asymmetric row with a
//!    rotation that does NOT commute with the mirror plane: the volume is
//!    invariant and the mirrored bbox is exactly the reflection of the
//!    original bbox.
//! 4. `mirrored_free_plane_solid_refuses_typed` — a mirror about a non-
//!    coordinate carrier is outside the admitted form and refuses typed.

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

/// The `python` interpreter prefix (mirrors the sibling integration suites).
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
/// subprocesses need `truck123d` importable from a real interpreter). Mirrors
/// door_circle_flip's staging: the crate cdylib is copied as `truck123d.pyd`
/// beside the interpreter runtime DLLs.
fn staged_native_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let exe = std::env::current_exe().expect("test binary path");
        let deps = exe.parent().expect("deps dir");
        let staging =
            std::env::temp_dir().join(format!("truck123d_mirror_form_{}", std::process::id()));
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

/// Runs a python script (with the corpus `ttc` dir as `argv[1]` and the given
/// extra arguments after it) and parses the single JSON object it prints on
/// stdout.
fn run_script(script: &str, extra: &[&str]) -> serde_json::Value {
    let output = python_command()
        .arg("-c")
        .arg(script)
        .arg(corpus_ttc_dir())
        .args(extra)
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

/// A unique temp STL path for one test.
fn stl_path(tag: &str) -> String {
    std::env::temp_dir()
        .join(format!(
            "truck123d_mirror_form_{}_{tag}.stl",
            std::process::id()
        ))
        .to_string_lossy()
        .into_owned()
}

/// The common preamble: import the real native module and the corpus door,
/// install the truck drop-in surface and expose the vector helper.
const PREAMBLE: &str = r#"
import json, sys
sys.path.insert(0, sys.argv[1])
import truck123d
import door
door.ENGINE = "truck"
bd = door.install_truck_alias()
def v(x, y, z):
    return bd.Vector(x, y, z)
def facts(shape):
    return json.loads(truck123d.bd_facts(json.dumps(shape._node())))
def emit(shape, path):
    return json.loads(truck123d.bd_stl(json.dumps(shape._node()), path, None))
"#;

#[test]
fn mirrored_placed_solid_round_trips_green() {
    let stl = stl_path("placed_solid");
    let script = String::from(PREAMBLE)
        + r#"
part = bd.Location((1.0, 2.0, 3.0)) * bd.Rotation(0.0, 0.0, 30.0) * bd.Box(10.0, 20.0, 30.0)
mirrored = bd.mirror(part, bd.Plane.XZ)
f = facts(mirrored)
stl = emit(mirrored, sys.argv[2])
print(json.dumps({"facts": f, "triangles": stl["triangles"]}))
"#;
    let record = run_script(&script, &[&stl]);
    let facts = &record["facts"];
    assert_eq!(
        facts["solid_count"], 1,
        "the mirrored placed solid must count one solid: {facts}"
    );
    let bracket = facts["volume_bracket"].as_array().expect("bracket");
    let lo = bracket[0].as_f64().expect("lo");
    let hi = bracket[1].as_f64().expect("hi");
    assert_eq!(
        lo, hi,
        "the mirrored solid's bracket must be exact: {facts}"
    );
    assert!(lo.is_finite() && lo > 0.0, "the volume must be positive");
    assert!(
        record["triangles"].as_u64().unwrap_or(0) > 0,
        "the mirrored solid must emit a non-empty STL: {record}"
    );
}

#[test]
fn mirrored_member_row_round_trips_green() {
    let stl = stl_path("member_row");
    let script = String::from(PREAMBLE)
        + r#"
section = bd.make_face(bd.Polygon(v(0.0, 0.0, 0.0), v(10.0, 0.0, 0.0), v(0.0, 4.0, 0.0)))
far = bd.Plane.XY.offset(20.0) * section
member = bd.loft([section, far], ruled=True)
mirrored = bd.mirror(member, bd.Plane.XZ)
f = facts(mirrored)
stl = emit(mirrored, sys.argv[2])
print(json.dumps({"facts": f, "triangles": stl["triangles"]}))
"#;
    let record = run_script(&script, &[&stl]);
    let facts = &record["facts"];
    assert_eq!(
        facts["solid_count"], 1,
        "the mirrored member row must count one solid: {facts}"
    );
    let bracket = facts["volume_bracket"].as_array().expect("bracket");
    assert_eq!(
        bracket[0].as_f64(),
        bracket[1].as_f64(),
        "the mirrored member row's bracket must be exact: {facts}"
    );
    assert!(
        record["triangles"].as_u64().unwrap_or(0) > 0,
        "the mirrored member row must emit a non-empty STL: {record}"
    );
}

#[test]
fn mirrored_rotation_frame_is_exact_isometry() {
    // The asymmetric prism is rotated about z, which does NOT commute with the
    // y = 0 mirror plane: the reflection of the placed solid must land exactly
    // on the reflected world bbox. The landed pre-rotation mirror order landed
    // it elsewhere.
    let script = String::from(PREAMBLE)
        + r#"
tri = bd.Polygon(v(0.0, 0.0, 0.0), v(10.0, 0.0, 0.0), v(0.0, 3.0, 0.0))
prism = bd.extrude(bd.make_face(tri), 4.0)
part = bd.Location((1.0, 2.0, 3.0)) * bd.Rotation(0.0, 0.0, 30.0) * prism
base = facts(part)
mirrored = bd.mirror(part, bd.Plane.XZ)
image = facts(mirrored)
lo, hi = base["bbox"]
true = [[lo[0], -hi[1], lo[2]], [hi[0], -lo[1], hi[2]]]
print(json.dumps({"base": base, "mirrored": image, "true_bbox": true}))
"#;
    let record = run_script(&script, &[]);
    let base = &record["base"];
    let mirrored = &record["mirrored"];
    // Volume is invariant under the exact isometry (the same bits).
    assert_eq!(
        base["volume"].as_f64(),
        mirrored["volume"].as_f64(),
        "the mirror must preserve the volume bit-for-bit"
    );
    let base_bracket = base["volume_bracket"].as_array().expect("base bracket");
    let mirrored_bracket = mirrored["volume_bracket"]
        .as_array()
        .expect("mirrored bracket");
    assert_eq!(base_bracket, mirrored_bracket, "the bracket is invariant");
    // The mirrored bbox is the reflection of the original bbox.
    assert_eq!(
        &mirrored["bbox"], &record["true_bbox"],
        "the mirrored bbox must be the exact reflection of the original bbox"
    );
}

#[test]
fn mirrored_free_plane_solid_refuses_typed() {
    let script = String::from(PREAMBLE)
        + r#"
part = bd.Box(1.0, 1.0, 1.0)
free = bd.Plane(origin=v(0.0, 0.0, 0.0), x_dir=v(1.0, 0.0, 0.0), z_dir=v(1.0, 1.0, 0.0))
try:
    bd.mirror(part, free)
    print(json.dumps({"refused": False}))
except truck123d.Refused as exc:
    print(json.dumps({"refused": True, "message": str(exc)}))
"#;
    let record = run_script(&script, &[]);
    assert_eq!(
        record["refused"], true,
        "a mirror about a non-coordinate carrier must refuse typed: {record}"
    );
}
