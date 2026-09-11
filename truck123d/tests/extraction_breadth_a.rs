//! FHC-EX-A required suite — the closed-loop loft section representation of
//! the drop-in executor binding (`truck123d/tests/extraction_breadth_a.rs`,
//! the crate test file booked by the packet).
//!
//! The packet's new representation is the SECTION of an N-station loft: a
//! closed ring of mixed line/spline edges whose recorded traversal is not
//! necessarily connected (the corpus authors `top + tail + bottom`), and the
//! single periodic spline outline (the corpus `Spline(*pts, periodic=True)`
//! section). The landed MONO-2 canonical loft arm consumes the oriented ring
//! unchanged; a stack whose edges cannot be connected into a ring still
//! refuses typed naming the open carrier.
//!
//! Tests:
//!
//! 1. `halo_row_certifies_under_the_truck_door` — the staged `f1/halo` row
//!    (the closed-loop loft chain) runs end to end under `door --engine
//!    truck`: `ok`, solid count 14, the recorded OCC reference bbox, and a
//!    non-empty binary STL.
//! 2. `closed_loop_ring_section_certifies_exact_bracket` — a hand-recorded
//!    two-station loft whose section is the corpus's non-connected
//!    `spline + line + spline` ring certifies an exact (`lo == hi`) bracket.
//! 3. `single_spline_section_certifies_exact_bracket` — a section recorded as
//!    one periodic spline certifies an exact bracket.
//! 4. `unconnectable_section_refuses_typed` — a section whose recorded edges
//!    cannot form a ring keeps the typed `unsupported_envelope` refusal.
//! 5. `still_open_rows_keep_their_typed_refusals` — the rows this packet does
//!    not flip (nose: a canonical cylinder union the landed canonical boolean
//!    product does not carry; cockpit: a corpus OCC probe) refuse TYPED, never
//!    untyped.

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

/// The absolute path of the corpus reference directory.
fn reference_dir() -> PathBuf {
    corpus_ttc_dir().join("reference")
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
            std::env::temp_dir().join(format!("truck123d_fhc_ex_a_{}", std::process::id()));
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

/// Runs the corpus door with `--engine truck` over the F1 tree.
fn run_truck_door(module: &str, entry: &str, args_json: &str) -> serde_json::Value {
    let stl =
        std::env::temp_dir().join(format!("ttc_fhc_ex_a_{}_{}.stl", std::process::id(), entry));
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

/// The corpus's non-connected `spline + line + spline` section band at `z`.
fn ring_section(z: f64) -> serde_json::Value {
    serde_json::json!([
        {"kind": "spline", "points": [[0.0, 0.0, z], [0.5, 0.6, z], [1.0, 0.0, z]]},
        {"kind": "line", "a": [1.0, 0.0, z], "b": [1.0, -1.0, z]},
        {"kind": "spline", "points": [[0.0, 0.0, z], [0.5, -0.6, z], [1.0, -1.0, z]]}
    ])
}

/// A two-station loft over the non-connected ring band.
fn ring_row() -> String {
    serde_json::json!({
        "part": {
            "solid": {
                "kind": "loft",
                "closed": false,
                "sections": [ring_section(0.0), ring_section(2.0)]
            },
            "x": 0.0,
            "y": 0.0,
            "z": 0.0,
            "rz": 0.0
        }
    })
    .to_string()
}

/// A two-station loft whose section is recorded as ONE periodic spline.
fn single_spline_row() -> String {
    serde_json::json!({
        "part": {
            "solid": {
                "kind": "loft",
                "closed": false,
                "sections": [
                    [{"kind": "spline", "points": [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0]]}],
                    [{"kind": "spline", "points": [[0.0, 0.0, 2.0], [1.0, 0.0, 2.0], [1.0, 1.0, 2.0], [0.0, 1.0, 2.0]]}]
                ]
            },
            "x": 0.0,
            "y": 0.0,
            "z": 0.0,
            "rz": 0.0
        }
    })
    .to_string()
}

/// A two-station loft whose recorded section edges cannot form a ring.
fn unconnectable_row() -> String {
    serde_json::json!({
        "part": {
            "solid": {
                "kind": "loft",
                "closed": false,
                "sections": [
                    [
                        {"kind": "line", "a": [0.0, 0.0, 0.0], "b": [1.0, 0.0, 0.0]},
                        {"kind": "line", "a": [5.0, 5.0, 0.0], "b": [6.0, 5.0, 0.0]},
                        {"kind": "spline", "points": [[10.0, 0.0, 0.0], [11.0, 1.0, 0.0], [12.0, 0.0, 0.0]]}
                    ],
                    [
                        {"kind": "line", "a": [0.0, 0.0, 2.0], "b": [1.0, 0.0, 2.0]},
                        {"kind": "line", "a": [5.0, 5.0, 2.0], "b": [6.0, 5.0, 2.0]},
                        {"kind": "spline", "points": [[10.0, 0.0, 2.0], [11.0, 1.0, 2.0], [12.0, 0.0, 2.0]]}
                    ]
                ]
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
fn halo_row_certifies_under_the_truck_door() {
    let record = run_truck_door("lib.mono_halo", "build_halo_bodies", "[]");
    assert_eq!(
        record["ok"], true,
        "the closed-loop halo row must build under truck: {record}"
    );
    let facts = &record["facts"];
    assert_eq!(facts["solid_count"], 14, "halo solid count");
    assert!(
        facts["volume"].as_f64().expect("volume").is_finite(),
        "halo volume must be finite"
    );

    // The recorded OCC reference is a diagnostic (annex C): the kernel bbox is
    // a carrier-derived certified ENCLOSURE (the N-station control-net hull
    // over-reports on outward-bulging sections), so it must contain the
    // reference bound, not match it to the retired band.
    let reference: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(reference_dir().join("halo.json")).expect("halo reference"),
    )
    .expect("halo reference json");
    let got = facts["bbox"].as_array().expect("bbox");
    let want = reference["facts"]["bbox"].as_array().expect("ref bbox");
    for axis in 0..3 {
        let got_lo = got[0][axis].as_f64().expect("bbox lo");
        let got_hi = got[1][axis].as_f64().expect("bbox hi");
        let want_lo = want[0][axis].as_f64().expect("reference lo");
        let want_hi = want[1][axis].as_f64().expect("reference hi");
        assert!(
            got_lo <= want_lo + 1e-3 && got_hi >= want_hi - 1e-3,
            "the kernel bbox must enclose the recorded reference on axis {axis}: \
             got [{got_lo}, {got_hi}] want [{want_lo}, {want_hi}]"
        );
    }

    // The mesh emits (the certification artifact).
    assert!(
        record["stl"]["triangles"].as_u64().unwrap_or(0) > 0,
        "halo STL must carry triangles"
    );
}

#[test]
fn closed_loop_ring_section_certifies_exact_bracket() {
    let facts = native_facts(&ring_row());
    assert_eq!(facts["solid_count"], 1);
    let bracket = facts["volume_bracket"].as_array().expect("bracket");
    let lo = bracket[0].as_f64().expect("lo");
    let hi = bracket[1].as_f64().expect("hi");
    assert_eq!(lo, hi, "the closed-ring bracket must be exact: {facts}");
    assert!(lo.is_finite() && lo > 0.0, "closed-ring volume positive");
}

#[test]
fn single_spline_section_certifies_exact_bracket() {
    let facts = native_facts(&single_spline_row());
    assert_eq!(facts["solid_count"], 1);
    let bracket = facts["volume_bracket"].as_array().expect("bracket");
    assert_eq!(
        bracket[0].as_f64().expect("lo"),
        bracket[1].as_f64().expect("hi"),
        "the periodic-spline section bracket must be exact: {facts}"
    );
}

#[test]
fn unconnectable_section_refuses_typed() {
    assert_eq!(native_refusal_kind(&unconnectable_row()), "Refused");
}

#[test]
fn still_open_rows_keep_their_typed_refusals() {
    // nose: the section band now certifies, but the row's canonical
    // cylinder-union fasteners are outside the landed canonical boolean
    // product (boxes only), so the row still refuses TYPED.
    let nose = run_truck_door("lib.nose", "build_nose", "[]");
    assert_eq!(nose["ok"], false, "nose is still red: {nose}");
    assert_eq!(
        nose["error"]["kind"], "Refused",
        "nose refusal must be typed"
    );

    // cockpit: the loft now certifies, but the corpus's own OCC bounds probe
    // (`_prism_estimate` -> `surfaces.bbox`) cannot be served on a
    // kernel-engine row, so the door records the typed refusal rather than an
    // untyped OCP `TypeError`.
    let cockpit = run_truck_door("lib.cockpit", "build_cockpit", "[]");
    assert_eq!(cockpit["ok"], false, "cockpit is still red: {cockpit}");
    assert_eq!(
        cockpit["error"]["kind"], "Refused",
        "cockpit refusal must be typed"
    );
}
