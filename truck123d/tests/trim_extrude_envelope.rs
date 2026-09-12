//! FHC-TRIM-EXTRUDE-ENVELOPE required suite -- the trim-extrude constructor's
//! composition envelope.
//!
//! MONO-4 landed the constructor arm for the corpus's spline-profile extrude
//! idiom: `bd.extrude(plane * make_face(bd.Spline(*pts, periodic=True)),
//! amount, both=True)` (the rear-wing louvre cutters, the suspension `_plate`
//! profiles) records a `trim_prism` row with an EMPTY line base profile, a
//! closed spline `trim_curve` and no pullback net. This packet widens the
//! envelope from the constructor to the COMPOSITION path: the spline-profile
//! prism is now a swept carrier the landed boolean extraction consumes (its
//! exact patch 2-cycle is the two-station ruled sweep of the closed loop plus
//! the exact planar cap fans), so a `_plate` pocket/union reaches the landed
//! contact-cover solver.
//!
//! Tests:
//!
//! 1. `trim_shapes_round_trip_green` -- each of the four rows' trim shapes
//!    (the louvre cutter, the suspension `_plate`, the rocker lightening
//!    plate, the steering-rack plate) records and answers a certified bracket
//!    plus an emitted mesh.
//! 2. `over_extending_coaxial_pocket_refuses_typed` -- the rocker's
//!    `plate_t * 3` lightening cutter extends past the base along the sweep
//!    normal and is refused TYPED by the landed EXTREMES-SURVIVE bbox gate
//!    (the booked SPEC_GAP pair class), never approximated. This is the
//!    composition cell `f1/suspension_front` / `f1/suspension_rear` reach.
//! 3. `line_base_real_trim_refuses_typed` -- a line base profile with a
//!    recorded trim curve and no pullback net stays outside the admitted
//!    envelope and refuses typed.
//! 4. `rear_wing_and_steering_rack_rows_certify` -- the two rows whose
//!    composition paths are entirely admitted are green end to end under
//!    `door --engine truck`.

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

/// The absolute path of the F1 corpus tree.
fn f1_tree() -> PathBuf {
    corpus_ttc_dir().join("trees").join("f1").join("src")
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
/// subprocesses need `truck123d` importable from a real interpreter).
fn staged_native_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let exe = std::env::current_exe().expect("test binary path");
        let deps = exe.parent().expect("deps dir");
        let staging =
            std::env::temp_dir().join(format!("truck123d_fhc_trim_{}", std::process::id()));
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

/// The native `bd_facts` of a row that must refuse; returns the exception
/// class name (or `Answered`).
fn native_refusal_kind(tree_json: &str) -> String {
    let script = r#"
import sys
import truck123d
try:
    truck123d.bd_facts(sys.argv[1])
except truck123d.Refused:
    print("Refused")
except truck123d.Unresolved:
    print("Unresolved")
else:
    print("Answered")
"#;
    run_python(script, &[tree_json]).trim().to_string()
}

/// The native `bd_stl` of one construction tree JSON; returns the emitted
/// triangle count.
fn native_stl_triangles(tree_json: &str) -> u64 {
    let stl = std::env::temp_dir().join(format!("ttc_fhc_trim_{}.stl", std::process::id()));
    let _ = std::fs::remove_file(&stl);
    let script = r#"
import sys
import truck123d
truck123d.bd_stl(sys.argv[1], sys.argv[2], None)
"#;
    run_python(script, &[tree_json, stl.to_str().expect("stl path")]);
    let bytes = std::fs::read(&stl).expect("stl emitted");
    let _ = std::fs::remove_file(&stl);
    if bytes.len() < 84 {
        return 0;
    }
    u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]) as u64
}

/// Runs the corpus door with `--engine truck` over the F1 tree.
fn run_truck_door(module: &str, entry: &str, args_json: &str) -> serde_json::Value {
    let stl = std::env::temp_dir().join(format!(
        "ttc_fhc_trim_door_{}_{}.stl",
        std::process::id(),
        entry
    ));
    let _ = std::fs::remove_file(&stl);
    let output = python_command()
        .arg(door_path())
        .arg("--engine")
        .arg("truck")
        .arg(f1_tree())
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

/// The corpus `rounded_plate_pts` outline (`surfaces.py:656`) as the door
/// records it: a periodic spline through the corner arc samples, closed on the
/// first point by `extrude` when the carrier does not already return to it.
fn rounded_plate_curve(length: f64, height: f64, corner: f64, samples: usize) -> Vec<[f64; 3]> {
    let (hl, hh, r) = (length / 2.0, height / 2.0, corner);
    let corners = [
        (hl - r, hh - r, 0.0f64),
        (-hl + r, hh - r, std::f64::consts::FRAC_PI_2),
        (-hl + r, -hh + r, std::f64::consts::PI),
        (hl - r, -hh + r, 3.0 * std::f64::consts::FRAC_PI_2),
    ];
    let mut pts = Vec::new();
    for (cu, cv, a0) in corners {
        for i in 0..=samples {
            let a = a0 + std::f64::consts::FRAC_PI_2 * i as f64 / samples as f64;
            pts.push([cu + r * a.cos(), cv + r * a.sin(), 0.0]);
        }
    }
    pts.push(pts[0]);
    pts
}

/// The door's `trim_prism` row for `extrude(spline_face, amount, both)`.
fn spline_prism_row(curve: &[[f64; 3]], amount: f64, both: bool) -> serde_json::Value {
    serde_json::json!({
        "part": {
            "solid": {
                "kind": "trim_prism",
                "profile": [],
                "amount": amount,
                "both": both,
                "trim_curve": curve,
                "trim_net": [],
                "tolerance": 1.0e-3
            },
            "x": 0.0,
            "y": 0.0,
            "z": 0.0,
            "rz": 0.0
        }
    })
}

/// A subtract of two recorded trim-prism parts.
fn subtract_row(a: serde_json::Value, b: serde_json::Value) -> String {
    serde_json::json!({
        "boolean": { "mode": "subtract", "a": a, "b": b }
    })
    .to_string()
}

#[test]
fn trim_shapes_round_trip_green() {
    // The four rows' trim shapes: the rear-wing louvre cutter, the suspension
    // `_plate` outline, the rocker lightening plate and the steering-rack
    // plate. Each records a closed spline base profile and answers a certified
    // (exact) bracket plus an emitted mesh.
    let shapes = [
        ("louvre", rounded_plate_curve(86.0, 6.8, 3.2, 5), 40.0, true),
        (
            "suspension_plate",
            rounded_plate_curve(30.0, 58.0, 13.0, 6),
            18.0,
            true,
        ),
        (
            "rocker_plate",
            rounded_plate_curve(120.0, 70.0, 12.0, 6),
            18.0,
            true,
        ),
        (
            "rack_plate",
            rounded_plate_curve(140.0, 24.0, 8.0, 6),
            9.0,
            true,
        ),
    ];
    for (name, curve, amount, both) in shapes {
        let tree = spline_prism_row(&curve, amount, both).to_string();
        let facts = native_facts(&tree);
        assert_eq!(facts["solid_count"], 1, "{name} solid count");
        let bracket = facts["volume_bracket"].as_array().expect("bracket");
        let lo = bracket[0].as_f64().expect("lo");
        let hi = bracket[1].as_f64().expect("hi");
        assert_eq!(lo, hi, "{name} bracket must be exact: {facts}");
        assert!(lo.is_finite() && lo > 0.0, "{name} volume positive");
        assert!(
            native_stl_triangles(&tree) > 0,
            "{name} must emit a non-empty STL"
        );
    }
}

#[test]
fn over_extending_coaxial_pocket_refuses_typed() {
    // The rocker's `plate_t * 3` lightening cutter extends past the base along
    // the sweep normal. The landed EXTREMES-SURVIVE bbox gate refuses the pair
    // TYPED: the reported boolean bbox cannot be certified equal to the base's
    // (the booked SPEC_GAP pair class), never approximated and never untyped.
    let base = spline_prism_row(&rounded_plate_curve(30.0, 58.0, 13.0, 6), 18.0, true);
    let tool = spline_prism_row(&rounded_plate_curve(10.0, 10.0, 3.0, 6), 54.0, true);
    assert_eq!(
        native_refusal_kind(&subtract_row(base, tool)),
        "Refused",
        "the over-extending coaxial pocket must refuse typed"
    );
}

#[test]
fn line_base_real_trim_refuses_typed() {
    // A line base profile with a recorded spline trim and no pullback net is
    // the landed constructor's real-trim carrier: outside the admitted
    // spline-profile envelope, it refuses typed rather than being read as a
    // spline base profile.
    let tree = serde_json::json!({
        "part": {
            "solid": {
                "kind": "trim_prism",
                "profile": [
                    {"kind": "line", "a": [0.0, 0.0, 0.0], "b": [1.0, 0.0, 0.0]},
                    {"kind": "line", "a": [1.0, 0.0, 0.0], "b": [1.0, 1.0, 0.0]},
                    {"kind": "line", "a": [1.0, 1.0, 0.0], "b": [0.0, 1.0, 0.0]},
                    {"kind": "line", "a": [0.0, 1.0, 0.0], "b": [0.0, 0.0, 0.0]}
                ],
                "amount": 1.0,
                "both": false,
                "trim_curve": [
                    [0.25, 0.25, 0.0],
                    [0.75, 0.25, 0.0],
                    [0.75, 0.75, 0.0],
                    [0.25, 0.75, 0.0],
                    [0.25, 0.25, 0.0]
                ],
                "trim_net": [],
                "tolerance": 1.0e-3
            },
            "x": 0.0,
            "y": 0.0,
            "z": 0.0,
            "rz": 0.0
        }
    })
    .to_string();
    assert_eq!(native_refusal_kind(&tree), "Refused");
}

#[test]
fn rear_wing_and_steering_rack_rows_certify() {
    // The louvre cutter composition is caught by the corpus endplate builder
    // and the trim shapes are admitted, so both rows are green end to end.
    for (module, entry) in [
        ("lib.rear_wing", "build_rear_wing"),
        ("lib.suspension", "build_steering_rack"),
    ] {
        let record = run_truck_door(module, entry, "[]");
        assert_eq!(
            record["ok"], true,
            "{entry} must build under truck: {record}"
        );
        let facts = &record["facts"];
        assert!(
            facts["solid_count"].as_u64().unwrap_or(0) >= 1,
            "{entry} solid count"
        );
        assert!(
            record["stl"]["triangles"].as_u64().unwrap_or(0) > 0,
            "{entry} STL must carry triangles"
        );
    }
}
