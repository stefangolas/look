//! AUTHOR-CENSUS-NAMES required suite — the door's census vocabulary is
//! completed with the eight names the hypercar corpus referenced but the truck
//! drop-in never answered.
//!
//! SWARM FINDING: `RectangleRounded`, `Align`, `Cone`, `Helix`, `Ellipse`,
//! `RegularPolygon`, `FilletPolyline` and `make_hull` were absent from
//! `_TRUCK_NAMES`, so corpus rows died with UNTYPED `AttributeError`s before
//! any typed verdict. This suite pins the cure:
//!
//! TIER A (recorded exactly): `Align` (placement metadata), `Cone` (canonical
//! `SolidSpec::Cone`), `Ellipse` (exact analytic `ProfileEdge::Ellipse`),
//! `RegularPolygon` (the existing polygon profile handler) and
//! `RectangleRounded` (four lines + four exact quarter-arc
//! `ProfileEdge::Arc` segments).
//!
//! TIER B (typed refusals naming the open carrier): `Helix`,
//! `FilletPolyline` and `make_hull`.

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
/// door_circle_flip's staging: the crate cdylib is copied as `truck123d.pyd`
/// beside the interpreter runtime DLLs.
fn staged_native_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let exe = std::env::current_exe().expect("test binary path");
        let deps = exe.parent().expect("deps dir");
        let staging =
            std::env::temp_dir().join(format!("truck123d_census_names_{}", std::process::id()));
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

/// The common preamble: import the real native module and the corpus door.
const PREAMBLE: &str = r#"
import json, math, sys
sys.path.insert(0, sys.argv[1])
import truck123d
import door
door._T123D = truck123d
bd = door._build_truck_module()
def facts(node):
    return json.loads(truck123d.bd_facts(json.dumps(node)))
"#;

fn assert_relative_close(got: f64, expect: f64, label: &str) {
    assert!(
        (got - expect).abs() <= 1e-9 * expect.abs().max(1.0),
        "{label}: got {got}, expected {expect}"
    );
}

/// Strips the wall-clock timing column (a diagnostic, never a gate).
fn strip_timing(value: &serde_json::Value) -> serde_json::Value {
    let mut value = value.clone();
    if let Some(map) = value.as_object_mut() {
        map.remove("timing");
    }
    value
}

#[test]
fn the_eight_census_names_are_answered_name_for_name() {
    let script = String::from(PREAMBLE)
        + r#"
names = ["RectangleRounded", "Align", "Cone", "Helix", "Ellipse",
         "RegularPolygon", "FilletPolyline", "make_hull"]
print(json.dumps({
    "answered": [n for n in names if hasattr(bd, n)],
    "in_list": [n for n in names if n in door._TRUCK_NAMES],
}))
"#;
    let record = run_script(&script);
    let names: Vec<&str> = vec![
        "RectangleRounded",
        "Align",
        "Cone",
        "Helix",
        "Ellipse",
        "RegularPolygon",
        "FilletPolyline",
        "make_hull",
    ];
    let answered: Vec<String> = record["answered"]
        .as_array()
        .expect("answered")
        .iter()
        .map(|v| v.as_str().unwrap_or("").to_string())
        .collect();
    let in_list: Vec<String> = record["in_list"]
        .as_array()
        .expect("in_list")
        .iter()
        .map(|v| v.as_str().unwrap_or("").to_string())
        .collect();
    for name in names {
        assert!(
            answered.iter().any(|n| n == name),
            "bd.{name} is not answered"
        );
        assert!(
            in_list.iter().any(|n| n == name),
            "{name} is absent from _TRUCK_NAMES"
        );
    }
}

#[test]
fn align_is_answered_as_placement_metadata() {
    let script = String::from(PREAMBLE)
        + r#"
plain = bd.Cone(20.0, 10.0, 6.0)
aligned = bd.Cone(20.0, 10.0, 6.0,
                  align=(bd.Align.CENTER, bd.Align.CENTER, bd.Align.MIN))
print(json.dumps({
    "plain": facts(plain._node()),
    "aligned": facts(aligned._node()),
    "constants": [bd.Align.MIN, bd.Align.CENTER, bd.Align.MAX],
}))
"#;
    let record = run_script(&script);
    assert_eq!(
        strip_timing(&record["plain"]),
        strip_timing(&record["aligned"]),
        "Align is placement metadata: it must not change the measured facts"
    );
    let constants = record["constants"].as_array().expect("align constants");
    assert_eq!(constants.len(), 3, "Align answers MIN/CENTER/MAX");
}

#[test]
fn cone_round_trips_exact_facts() {
    let script = String::from(PREAMBLE)
        + r#"
r0, r1, h = 20.0, 10.0, 6.0
part = bd.Cone(bottom_radius=r0, top_radius=r1, height=h)
print(json.dumps({"facts": facts(part._node())}))
"#;
    let record = run_script(&script);
    let facts = &record["facts"];
    assert_eq!(facts["solid_count"], 1);
    let volume = facts["volume"].as_f64().expect("volume");
    let expect = std::f64::consts::PI / 3.0 * 6.0 * (20.0 * 20.0 + 20.0 * 10.0 + 10.0 * 10.0);
    assert_relative_close(volume, expect, "cone volume");
    assert_eq!(
        facts["bbox"],
        serde_json::json!([[-20.0, -20.0, -3.0], [20.0, 20.0, 3.0]])
    );
}

#[test]
fn ellipse_round_trips_exact_facts() {
    let script = String::from(PREAMBLE)
        + r#"
a, b, amount = 72.0, 56.0, 5.0
part = bd.extrude(bd.Ellipse(a, b), amount)
print(json.dumps({"facts": facts(part._node()), "expect": math.pi * a * b * amount}))
"#;
    let record = run_script(&script);
    let facts = &record["facts"];
    assert_eq!(facts["solid_count"], 1);
    let volume = facts["volume"].as_f64().expect("volume");
    let expect = record["expect"].as_f64().expect("expect");
    assert_relative_close(volume, expect, "ellipse prism volume");
    assert_eq!(
        facts["bbox"],
        serde_json::json!([[-72.0, -56.0, 0.0], [72.0, 56.0, 5.0]])
    );
}

#[test]
fn regular_polygon_routes_through_the_polygon_profile() {
    let script = String::from(PREAMBLE)
        + r#"
r, n, amount = 50.0, 6, 4.0
part = bd.extrude(bd.RegularPolygon(r, n), amount)
area = 0.5 * n * r * r * math.sin(2.0 * math.pi / n)
print(json.dumps({"facts": facts(part._node()), "expect": area * amount}))
"#;
    let record = run_script(&script);
    let facts = &record["facts"];
    assert_eq!(facts["solid_count"], 1);
    let volume = facts["volume"].as_f64().expect("volume");
    let expect = record["expect"].as_f64().expect("expect");
    assert_relative_close(volume, expect, "regular polygon prism volume");
    let bbox = facts["bbox"].as_array().expect("bbox");
    let x_lo = bbox[0][0].as_f64().expect("x lo");
    let x_hi = bbox[1][0].as_f64().expect("x hi");
    assert_relative_close(x_lo, -50.0, "hexagon x lo");
    assert_relative_close(x_hi, 50.0, "hexagon x hi");
}

#[test]
fn rectangle_rounded_round_trips_exact_facts() {
    let script = String::from(PREAMBLE)
        + r#"
w, h, r, amount = 74.0, 206.0, 30.0, 4.0
part = bd.extrude(bd.RectangleRounded(w, h, r), amount)
area = w * h - (4.0 - math.pi) * r * r
print(json.dumps({"facts": facts(part._node()), "expect": area * amount}))
"#;
    let record = run_script(&script);
    let facts = &record["facts"];
    assert_eq!(facts["solid_count"], 1);
    let volume = facts["volume"].as_f64().expect("volume");
    let expect = record["expect"].as_f64().expect("expect");
    assert_relative_close(volume, expect, "rounded-rectangle prism volume");
    assert_eq!(
        facts["bbox"],
        serde_json::json!([[-37.0, -103.0, 0.0], [37.0, 103.0, 4.0]])
    );
}

#[test]
fn helix_fillet_polyline_and_make_hull_refuse_typed() {
    let script = String::from(PREAMBLE)
        + r#"
out = {}
for name, call in (
    ("Helix", lambda: bd.Helix(pitch=1.0, height=10.0, radius=3.0)),
    ("FilletPolyline", lambda: bd.FilletPolyline((0, 0), (1, 0), (1, 1), radius=0.1)),
    ("make_hull", lambda: bd.make_hull([])),
):
    try:
        call()
        out[name] = {"refused": False}
    except truck123d.Refused as exc:
        out[name] = {"refused": True, "message": str(exc)}
print(json.dumps(out))
"#;
    let record = run_script(&script);
    for name in ["Helix", "FilletPolyline", "make_hull"] {
        assert_eq!(
            record[name]["refused"], true,
            "{name} must refuse TYPED (never AttributeError)"
        );
        let message = record[name]["message"].as_str().unwrap_or("");
        assert!(
            message.contains(name),
            "the refusal must name the open carrier {name}: {message}"
        );
    }
}

#[test]
fn hypercar_shaped_composition_is_green() {
    let script = String::from(PREAMBLE)
        + r#"
face = bd.RectangleRounded(74.0, 206.0, 30.0)
pad = bd.Pos(0.0, 0.0, 10.0) * bd.extrude(face, amount=4.0)
cone = bd.Cone(bottom_radius=20.0, top_radius=10.0, height=6.0)
group = bd.Compound(obj=[pad, cone], children=[pad, cone], label="hypercar_pad")
pad_area = 74.0 * 206.0 - (4.0 - math.pi) * 30.0 * 30.0
cone_volume = math.pi / 3.0 * 6.0 * (20.0**2 + 20.0 * 10.0 + 10.0**2)
print(json.dumps({
    "facts": facts(group._node()),
    "expect": pad_area * 4.0 + cone_volume,
}))
"#;
    let record = run_script(&script);
    let facts = &record["facts"];
    assert_eq!(facts["solid_count"], 2);
    let volume = facts["volume"].as_f64().expect("volume");
    let expect = record["expect"].as_f64().expect("expect");
    assert_relative_close(volume, expect, "hypercar composition volume");
    assert_eq!(
        facts["bbox"],
        serde_json::json!([[-37.0, -103.0, -3.0], [37.0, 103.0, 14.0]])
    );
}
