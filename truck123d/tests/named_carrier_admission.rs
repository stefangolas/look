//! FHC-G4 required suite — named-carrier admission for the hypercar walls
//! (`truck123d/tests/named_carrier_admission.rs`, the crate test file booked by
//! the packet).
//!
//! The category-4 gap: hypercar rows that progressed past their R3 name gaps
//! and stopped at new, NAMED carriers. This suite pins the admissions the
//! packet lands and the typed refusals that remain:
//!
//! 1. `color_name_is_answered_and_records_the_client_payload` — the missing
//!    `bd.Color` name surface is answered (a colour data row), and the recorded
//!    payload reaches the landed GLB/facts colour path.
//! 2. `closed_polyline_profile_revolves_to_an_exact_annulus` — the closed
//!    polyline composition the brake/hinge rows author now arrives at the
//!    exact lathe arm as a closed loop and certifies an exact bracket.
//! 3. `mirrored_band_section_loft_certifies` — a section built from a half-band
//!    MIRRORED about a plane composes with the landed loft and certifies an
//!    exact bracket (the `surfaces._mirrored_face` carrier).
//! 4. `hypercar_body_master_band_loft_is_green` — the corpus's 26-station
//!    mirrored band loft round-trips green under the truck door (facts bracket
//!    plus STL).
//! 5. `mirrored_band_loft_facts_are_equal_and_opposite` — mirroring the lofted
//!    solid reflects the carrier-derived facts exactly (volume preserved, the
//!    mirror axis negated).
//! 6. `open_revolve_profile_refuses_typed` — a profile that genuinely does not
//!    close refuses TYPED naming the open carrier, never untyped.
//! 7. `hypercar_named_carrier_rows_progress_to_typed_verdicts` — the
//!    brake/wheel/hinge rows progressed past the admitted carriers and now
//!    refuse TYPED at their next named carrier (no untyped `AttributeError` /
//!    `TypeError`).

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
        let staging = std::env::temp_dir().join(format!("truck123d_fhc_g4_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&staging);
        std::fs::create_dir_all(&staging).expect("staging dir");
        copy_if_present(deps, "truck123d.dll", &staging, "truck123d.pyd");
        let python_prefix = python_prefix();
        for dll in [
            "python3.dll",
            "python314.dll",
            "vcruntime140.dll",
            "vcruntime140_1.dll",
        ] {
            copy_from_dir(python_prefix, dll, &staging);
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

/// Runs a python `-c` script with the corpus `ttc` dir as `argv[1]`; returns
/// its stdout. Panics on a nonzero exit.
fn run_python(script: &str) -> String {
    let output = python_command()
        .arg("-c")
        .arg(script)
        .arg(corpus_ttc_dir())
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

/// Runs the corpus door with `--engine truck` over one tree/module/entry.
fn run_truck_door(tree: &str, module: &str, entry: &str, args_json: &str) -> serde_json::Value {
    let stl = std::env::temp_dir().join(format!("ttc_fhc_g4_{}_{}.stl", std::process::id(), entry));
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

/// Asserts two floats agree to the landed relative tolerance.
fn assert_relative_close(got: f64, expect: f64, label: &str) {
    assert!(
        (got - expect).abs() <= 1e-9 * expect.abs().max(1.0),
        "{label}: got {got}, expected {expect}"
    );
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

#[test]
fn color_name_is_answered_and_records_the_client_payload() {
    let script = String::from(PREAMBLE)
        + r#"
part = bd.Box(2.0, 3.0, 4.0)
part.color = bd.Color(0.1, 0.2, 0.3, 0.4)
print(json.dumps({
    "answered": hasattr(bd, "Color"),
    "in_list": "Color" in door._TRUCK_NAMES,
    "recorded": door._color_record(part.color),
    "facts": facts(part._node()),
}))
"#;
    let record: serde_json::Value =
        serde_json::from_str(run_python(&script).trim()).expect("colour json");
    assert_eq!(record["answered"], true, "bd.Color must be answered");
    assert_eq!(record["in_list"], true, "Color must be in _TRUCK_NAMES");
    assert_eq!(
        record["recorded"], "[0.1, 0.2, 0.3, 0.4]",
        "the colour data row records its channels verbatim"
    );
    assert_eq!(
        record["facts"]["color"], "[0.1, 0.2, 0.3, 0.4]",
        "the recorded colour rides the landed facts payload"
    );
}

#[test]
fn closed_polyline_profile_revolves_to_an_exact_annulus() {
    let script = String::from(PREAMBLE)
        + r#"
import math
r0, r1, h = 5.0, 8.0, 4.0
face = bd.make_face(bd.Polyline((r0, 0.0), (r1, 0.0), (r1, h), (r0, h), close=True))
part = bd.revolve(bd.Plane.XZ * face, bd.Axis.Z)
print(json.dumps({
    "facts": facts(part._node()),
    "expect": math.pi * (r1 * r1 - r0 * r0) * h,
}))
"#;
    let record: serde_json::Value =
        serde_json::from_str(run_python(&script).trim()).expect("annulus json");
    let facts = &record["facts"];
    assert_eq!(
        facts["solid_count"], 1,
        "the closed profile revolves: {facts}"
    );
    let bracket = facts["volume_bracket"].as_array().expect("bracket");
    assert_eq!(
        bracket[0].as_f64(),
        bracket[1].as_f64(),
        "the closed-profile lathe bracket must be exact: {facts}"
    );
    let volume = facts["volume"].as_f64().expect("volume");
    let expect = record["expect"].as_f64().expect("expect");
    assert_relative_close(volume, expect, "revolved annulus volume");
}

/// A half-band section that starts and ends on the mirror plane, closed into a
/// full section by the door's exact wire mirror (the `_mirrored_face` carrier).
const BAND_SECTION: &str = r#"
def _half():
    return bd.Spline(
        (0.0, 0.0, 0.0), (0.5, 1.0, 0.0), (1.0, 1.5, 0.0),
        (0.5, 2.0, 0.0), (0.0, 3.0, 0.0),
    )
def _section(x):
    half = _half()
    return bd.Plane.YZ.offset(x) * bd.make_face(half + bd.mirror(half, bd.Plane.YZ))
"#;

#[test]
fn mirrored_band_section_loft_certifies() {
    let script = String::from(PREAMBLE)
        + BAND_SECTION
        + r#"
part = bd.loft([_section(0.0), _section(10.0), _section(20.0)])
print(json.dumps({"facts": facts(part._node())}))
"#;
    let record: serde_json::Value =
        serde_json::from_str(run_python(&script).trim()).expect("band json");
    let facts = &record["facts"];
    assert_eq!(facts["solid_count"], 1, "the mirrored band lofts: {facts}");
    let bracket = facts["volume_bracket"].as_array().expect("bracket");
    assert_eq!(
        bracket[0].as_f64(),
        bracket[1].as_f64(),
        "the mirrored-band loft bracket must be exact: {facts}"
    );
    assert!(
        facts["volume"].as_f64().expect("volume") > 0.0,
        "the mirrored-band loft volume is positive: {facts}"
    );
}

#[test]
fn hypercar_body_master_band_loft_is_green() {
    // The corpus's 26-station mirrored band loft (the real `_mirrored_face`
    // carrier) round-trips green: constructive facts plus a deterministic STL.
    let record = run_truck_door("hypercar", "lib.surfaces", "body_master", "[]");
    assert_eq!(
        record["ok"], true,
        "the mirrored band loft must build under truck: {record}"
    );
    let facts = &record["facts"];
    assert_eq!(facts["solid_count"], 1, "body_master solid count: {facts}");
    assert!(
        facts["volume"].as_f64().expect("volume").is_finite(),
        "body_master volume must be finite: {facts}"
    );
    assert!(
        facts["volume"].as_f64().expect("volume") > 0.0,
        "body_master volume must be positive: {facts}"
    );
    assert!(
        record["stl"]["triangles"].as_u64().unwrap_or(0) > 0,
        "body_master STL must carry triangles"
    );
}

#[test]
fn mirrored_band_loft_facts_are_equal_and_opposite() {
    let script = String::from(PREAMBLE)
        + BAND_SECTION
        + r#"
part = bd.loft([_section(0.0), _section(10.0), _section(20.0)])
mirrored = bd.mirror(part, bd.Plane.YZ)
print(json.dumps({
    "facts": facts(part._node()),
    "mirrored": facts(mirrored._node()),
}))
"#;
    let record: serde_json::Value =
        serde_json::from_str(run_python(&script).trim()).expect("mirror json");
    let facts = &record["facts"];
    let mirrored = &record["mirrored"];
    assert_relative_close(
        mirrored["volume"].as_f64().expect("mirrored volume"),
        facts["volume"].as_f64().expect("volume"),
        "mirror preserves volume",
    );
    let bbox = facts["bbox"].as_array().expect("bbox");
    let mirror_bbox = mirrored["bbox"].as_array().expect("mirror bbox");
    // Plane.YZ reflects x -> -x; the band spans x in [0, 20], so the mirror
    // spans x in [-20, 0] with the same y/z extents.
    assert_relative_close(
        mirror_bbox[0][0].as_f64().expect("x lo"),
        -bbox[1][0].as_f64().expect("x hi"),
        "mirrored x lo",
    );
    assert_relative_close(
        mirror_bbox[1][0].as_f64().expect("x hi"),
        -bbox[0][0].as_f64().expect("x lo"),
        "mirrored x hi",
    );
    for axis in 1..3 {
        assert_relative_close(
            mirror_bbox[0][axis].as_f64().expect("y/z lo"),
            bbox[0][axis].as_f64().expect("y/z lo"),
            "mirror preserves the transverse lo",
        );
        assert_relative_close(
            mirror_bbox[1][axis].as_f64().expect("y/z hi"),
            bbox[1][axis].as_f64().expect("y/z hi"),
            "mirror preserves the transverse hi",
        );
    }
}

#[test]
fn open_revolve_profile_refuses_typed() {
    let script = String::from(PREAMBLE)
        + r#"
out = {"refused": False}
try:
    bd.revolve(
        bd.Plane.XZ * bd.make_face(bd.Spline((1.0, 0.0), (2.0, 0.0), (3.0, 0.0))),
        bd.Axis.Z,
    )
except truck123d.Refused as exc:
    out = {"refused": True, "message": str(exc)}
except Exception as exc:
    out = {"refused": False, "untyped": type(exc).__name__}
print(json.dumps(out))
"#;
    let record: serde_json::Value =
        serde_json::from_str(run_python(&script).trim()).expect("open profile json");
    assert_eq!(
        record["refused"], true,
        "a genuinely open profile must refuse typed: {record}"
    );
    let message = record["message"].as_str().unwrap_or("");
    assert!(
        message.contains("closed profile"),
        "the refusal must name the open profile: {message}"
    );
}

#[test]
fn hypercar_named_carrier_rows_progress_to_typed_verdicts() {
    // The rows progressed past the admitted named carriers (`bd.Color`, the
    // closed polyline profile) and now stop at their next NAMED carrier with a
    // TYPED verdict — never an untyped `AttributeError`/`TypeError`.
    for (module, carrier) in [
        ("lib.brakes", "face_boolean"),
        ("lib.wheels", "FilletPolyline"),
        ("lib.hinge", "spline_loft*cylinder"),
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
        let observed = record["error"]["carrier"].as_str().unwrap_or("");
        assert!(
            observed.contains(carrier)
                || record["error"]["message"]
                    .as_str()
                    .unwrap_or("")
                    .contains(carrier),
            "{module} must name its next carrier {carrier}: {record}"
        );
    }
}
