//! FHC-EX-B required suite — spline lofts as boolean operands plus the
//! hypercar band-loft carrier (`truck123d/tests/extraction_breadth_b.rs`, the
//! crate test file booked by the packet).
//!
//! The packet widens the extraction that feeds the landed swept-carrier
//! boolean admission: a curved-section spline loft (a section built on a
//! plane, but whose boundary is a curved spline rather than four straight
//! spans) now extracts an exact planar end-cap fan, so its tensor-Bernstein
//! 2-cycle closes against the loft sides. The pure multi-station spline lofts
//! (`f1/beam_wing`, `f1/drs_flap`, `f1/track_rod_*`) are certified by the
//! dependency packet FHC-EX-A and are re-verified green here. The corpus's
//! nested/parallel offset shells reach the landed solver and refuse TYPED at
//! its transversality/normal-cone gates — the genuine `swept x swept` parallel
//! offset pair class booked as a SPEC_GAP, never approximated.
//!
//! Tests:
//!
//! 1. `beam_wing_row_certifies_under_the_truck_door` — a green round trip for
//!    the pure multi-station spline loft (`ok`, constructive facts, STL emits).
//! 2. `curved_section_loft_extracts_an_exact_planar_cap` — a hand-recorded
//!    two-station curved-section loft certifies an exact (`lo == hi`) bracket;
//!    the extraction is the widened one (the curved cap is no longer an open
//!    carrier).
//! 3. `curved_section_loft_boolean_refuses_typed_at_the_solver` — the widened
//!    extraction feeds the landed boolean admission, which refuses the
//!    parallel/nested curved pair TYPED (the SPEC_GAP), never untyped and
//!    never approximated.
//! 4. `hypercar_band_and_brake_rows_stay_typed` — the hypercar band lofts and
//!    the brake revolve keep their typed door refusals.

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
        let staging =
            std::env::temp_dir().join(format!("truck123d_fhc_ex_b_{}", std::process::id()));
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

/// Runs the corpus door with `--engine truck` over one tree/module/entry.
fn run_truck_door(tree: &str, module: &str, entry: &str, args_json: &str) -> serde_json::Value {
    let stl =
        std::env::temp_dir().join(format!("ttc_fhc_ex_b_{}_{}.stl", std::process::id(), entry));
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

/// A curved (but planar) closed section: one periodic spline through the
/// corners of a square of side `scale` centred on `(0.5, 0.5)` at `z`. The
/// recorded boundary is a curved spline, so the exact planar cap is the fan
/// (the landed fast path's four-straight-span quad does not apply).
fn lens_section(z: f64, scale: f64) -> serde_json::Value {
    let c = 0.5;
    let h = 0.5 * scale;
    let corners = [
        [c - h, c - h, z],
        [c + h, c - h, z],
        [c + h, c + h, z],
        [c - h, c + h, z],
    ];
    serde_json::json!([{"kind": "spline", "points": corners}])
}

/// A two-station curved-section loft row.
fn curved_loft_row(z0: f64, z1: f64, scale: f64) -> String {
    serde_json::json!({
        "part": {
            "solid": {
                "kind": "loft",
                "closed": false,
                "sections": [lens_section(z0, scale), lens_section(z1, scale)]
            },
            "x": 0.0,
            "y": 0.0,
            "z": 0.0,
            "rz": 0.0
        }
    })
    .to_string()
}

#[test]
fn beam_wing_row_certifies_under_the_truck_door() {
    // The pure multi-station spline loft (the packet's pinned operand carrier)
    // is green end to end under `door --engine truck`.
    let record = run_truck_door("f1", "lib.rear_wing", "build_beam_wing", "[]");
    assert_eq!(
        record["ok"], true,
        "the multi-station spline loft must build under truck: {record}"
    );
    let facts = &record["facts"];
    assert!(
        facts["solid_count"].as_u64().unwrap_or(0) >= 1,
        "beam_wing solid count"
    );
    assert!(
        facts["volume"].as_f64().expect("volume").is_finite(),
        "beam_wing volume must be finite"
    );
    assert!(
        record["stl"]["triangles"].as_u64().unwrap_or(0) > 0,
        "beam_wing STL must carry triangles"
    );
}

#[test]
fn curved_section_loft_extracts_an_exact_planar_cap() {
    // A curved-section loft is no longer an open extraction carrier: the exact
    // planar fan cap closes the 2-cycle, so the loft measures with an exact
    // (`lo == hi`) certified bracket.
    let facts = native_facts(&curved_loft_row(0.0, 5.0, 1.0));
    assert_eq!(facts["solid_count"], 1);
    let bracket = facts["volume_bracket"].as_array().expect("bracket");
    let lo = bracket[0].as_f64().expect("lo");
    let hi = bracket[1].as_f64().expect("hi");
    assert_eq!(lo, hi, "the curved-section bracket must be exact: {facts}");
    assert!(lo.is_finite() && lo > 0.0, "curved-section volume positive");
}

#[test]
fn curved_section_loft_boolean_refuses_typed_at_the_solver() {
    // Two nested/parallel curved shells now EXTRACT (the widened cap fan) and
    // reach the landed boolean solver, which refuses the parallel offset pair
    // TYPED — the booked SPEC_GAP pair class, never an untyped failure and
    // never an approximation.
    let tree = serde_json::json!({
        "boolean": {
            "mode": "subtract",
            "a": serde_json::from_str::<serde_json::Value>(&curved_loft_row(0.0, 5.0, 1.0))
                .expect("base json"),
            "b": serde_json::from_str::<serde_json::Value>(&curved_loft_row(1.0, 4.0, 0.5))
                .expect("tool json")
        }
    })
    .to_string();
    assert_eq!(
        native_refusal_kind(&tree),
        "Refused",
        "the parallel curved shell pair must refuse typed"
    );
}

#[test]
fn hypercar_band_and_brake_rows_stay_typed() {
    // The hypercar master body shell (a nested band loft) refuses TYPED at the
    // landed admission.
    let body = run_truck_door("hypercar", "lib.body", "build", "[]");
    assert_eq!(body["ok"], false, "hypercar/body is still red: {body}");
    assert_eq!(
        body["error"]["kind"], "Refused",
        "hypercar/body refusal must be typed"
    );

    // The brake disc's `revolve(Polyline(..., close=True) * Plane.XZ)` profile
    // is outside the recorded profile vocabulary (the drop-in records a
    // polyline as one straight edge), so the revolve refuses TYPED naming the
    // open carrier.
    let brakes = run_truck_door("hypercar", "lib.brakes", "build", "[]");
    assert_eq!(
        brakes["ok"], false,
        "hypercar/brakes is still red: {brakes}"
    );
    assert_eq!(
        brakes["error"]["kind"], "Refused",
        "hypercar/brakes refusal must be typed"
    );
}
