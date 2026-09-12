//! FHC-G2 required suite — the kernel answers for the corpus's OCC probe
//! idioms.
//!
//! The corpus's shared helpers (`corpus/ttc/trees/f1/src/lib/surfaces.py`)
//! probe a kernel-engine row through OCC: `surfaces.bbox` / `surfaces.obox`
//! bound `shape.wrapped`, `surfaces.is_valid_shape` runs `BRepCheck_Analyzer`
//! on it, and the row builders call `shape.is_valid`. A kernel-engine row has
//! no OCC geometry, so the door answers the probes from the row's own certified
//! facts row (`truck123d.bd_facts`):
//!
//!   * the validity probe answers from the certified volume bracket (present,
//!     ordered and finite) and a constructive `solid_count` — never an OCC
//!     `BRepCheck`;
//!   * the bounds probe answers the certified carrier-derived enclosure: the
//!     facts `bbox` is marshalled through the helper's `BRepBndLib` interface
//!     as an OCC fact carrier, so the answer is the certified bound, never an
//!     OCC geometry consultation;
//!   * a row whose certificate cannot answer the probe refuses TYPED naming the
//!     missing fact; an OCC probe of a genuinely-OCC row (an `Edge`/`Wire`
//!     curve carrier) still refuses TYPED with the anchored message.
//!
//! Tests:
//!
//! 1. `kernel_row_validity_probe_answers_from_a_bracket` — `is_valid` and
//!    `surfaces.is_valid_shape` answer from the certified bracket; a row the
//!    facts cannot measure is not valid.
//! 2. `bounds_probe_answers_carrier_derived` — `surfaces.bbox` / `surfaces.obox`
//!    on a kernel row equal the certified facts `bbox` (within OCC's shape
//!    tolerance).
//! 3. `occ_probe_of_an_occ_row_still_refuses_typed` — `Edge.wrapped` /
//!    `Wire.wrapped` still raise the anchored typed refusal.
//! 4. `claimed_rows_progress_past_the_probe_queries` — each claimed row's probe
//!    site passes and the row proceeds to its next honest verdict.

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

/// The absolute path of the F1 corpus tree's source directory.
fn f1_tree_src() -> PathBuf {
    corpus_ttc_dir().join("trees").join("f1").join("src")
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
        let staging = std::env::temp_dir().join(format!("truck123d_fhc_g2_{}", std::process::id()));
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

/// The two positional arguments every probe script consumes: the corpus `ttc`
/// directory and the F1 tree source directory.
fn probe_args() -> [String; 2] {
    [
        corpus_ttc_dir().to_str().expect("corpus path").to_string(),
        f1_tree_src().to_str().expect("f1 tree path").to_string(),
    ]
}

/// The standard probe preamble: install the truck drop-in over `cadgen`,
/// expose `door`, and import the shared `surfaces` helper.
const PREAMBLE: &str = r#"
import json, sys
sys.path.insert(0, sys.argv[1])
sys.path.insert(0, sys.argv[2])
import door
door.ENGINE = "truck"
bd = door.install_truck_alias()
import lib.surfaces as surfaces
import truck123d

def close(a, b, tol=1e-5):
    return all(abs(a[i] - b[i]) < tol for i in range(3))
"#;

// ---------------------------------------------------------------------------
// Test 1: the validity probe answers from the certified bracket.
// ---------------------------------------------------------------------------

#[test]
fn kernel_row_validity_probe_answers_from_a_bracket() {
    let script = format!(
        r#"{PREAMBLE}
part = bd.Box(20.0, 10.0, 30.0)
facts = part._facts()
assert facts["solid_count"] == 1, facts
lo, hi = facts["volume_bracket"]
assert lo <= hi and lo == lo and hi == hi, facts
assert part.is_valid is True, facts
assert surfaces.is_valid_shape(part) is True, facts

# A boolean row carries the solver's certified bracket.
cut = bd.Box(20.0, 10.0, 30.0) - bd.Box(5.0, 5.0, 5.0)
cb = cut._facts()["volume_bracket"]
assert cb[0] <= cb[1], cb
assert cut.is_valid is True
assert surfaces.is_valid_shape(cut) is True

# A row the facts cannot measure is not valid.
empty = bd.Compound(children=[])
assert surfaces.is_valid_shape(empty) is False
assert empty.is_valid is False
print(json.dumps({{"ok": True, "bracket": facts["volume_bracket"]}}))
"#
    );
    let args = probe_args();
    let stdout = run_python(&script, &[args[0].as_str(), args[1].as_str()]);
    let record: serde_json::Value = serde_json::from_str(stdout.trim()).expect("validity json");
    assert_eq!(record["ok"], true);
    assert_eq!(record["bracket"], serde_json::json!([6000.0, 6000.0]));
}

// ---------------------------------------------------------------------------
// Test 2: the bounds probe answers the carrier-derived enclosure.
// ---------------------------------------------------------------------------

#[test]
fn bounds_probe_answers_carrier_derived() {
    let script = format!(
        r#"{PREAMBLE}
part = bd.Box(20.0, 10.0, 30.0)
facts = part._facts()
lo, hi = surfaces.bbox(part)
assert close(lo, facts["bbox"][0]) and close(hi, facts["bbox"][1]), (lo, hi, facts["bbox"])
olo, ohi = surfaces.obox(part)
assert close(olo, facts["bbox"][0]) and close(ohi, facts["bbox"][1]), (olo, ohi, facts["bbox"])

# A placed row answers its world enclosure.
placed = bd.Pos(7.0, -3.0, 2.0) * bd.Box(20.0, 10.0, 30.0)
pf = placed._facts()
plo, phi = surfaces.bbox(placed)
assert close(plo, pf["bbox"][0]) and close(phi, pf["bbox"][1]), (plo, phi, pf["bbox"])
print(json.dumps({{"ok": True, "bbox": facts["bbox"]}}))
"#
    );
    let args = probe_args();
    let stdout = run_python(&script, &[args[0].as_str(), args[1].as_str()]);
    let record: serde_json::Value = serde_json::from_str(stdout.trim()).expect("bounds json");
    assert_eq!(record["ok"], true);
    assert_eq!(
        record["bbox"],
        serde_json::json!([[-10.0, -5.0, -15.0], [10.0, 5.0, 15.0]])
    );
}

// ---------------------------------------------------------------------------
// Test 3: an OCC probe of an OCC (curve) row still refuses typed.
// ---------------------------------------------------------------------------

#[test]
fn occ_probe_of_an_occ_row_still_refuses_typed() {
    let script = format!(
        r#"{PREAMBLE}
edge = door.Edge.make_line(door.Vector(0.0, 0.0, 0.0), door.Vector(1.0, 0.0, 0.0))
for row in (edge, door.Wire([edge])):
    try:
        row.wrapped
        raise AssertionError("the OCC probe must refuse")
    except truck123d.Refused as exc:
        assert "an OCC probe of a kernel-engine row is not a kernel-engine row" in str(exc), exc
print(json.dumps({{"ok": True}}))
"#
    );
    let args = probe_args();
    let stdout = run_python(&script, &[args[0].as_str(), args[1].as_str()]);
    let record: serde_json::Value = serde_json::from_str(stdout.trim()).expect("occ json");
    assert_eq!(record["ok"], true);
}

// ---------------------------------------------------------------------------
// Test 4: the claimed rows progress past the probe queries.
// ---------------------------------------------------------------------------

#[test]
fn claimed_rows_progress_past_the_probe_queries() {
    // Each claimed row's probe site is exercised end-to-end through the door's
    // drop-in: the bounds/validity probe passes and the row proceeds to its
    // next honest verdict (a green result or a DIFFERENT typed refusal).
    let script = format!(
        r#"{PREAMBLE}
import lib.cockpit as cockpit
import lib.engine_cover as engine_cover
import lib.mono_tub as mono_tub

# f1/cockpit: `_solid`'s `surfaces.bbox(base)` on the lofted seat shell.
def square(z, s=10.0):
    pts = [(-s, -s, z), (s, -s, z), (s, s, z), (-s, s, z)]
    return door.Face(door.Wire([
        door.Edge.make_line(door.Vector(*pts[i]), door.Vector(*pts[(i + 1) % 4]))
        for i in range(4)
    ]))
seat = cockpit._solid([square(0.0), square(5.0), square(10.0)], smooth=True)
assert seat._facts()["volume"] > 0.0

# f1/engine_cover: `_bounded`'s `surfaces.bbox(solid)` on the lofted skin.
box = bd.Box(1.0, 1.0, 1.0)
outlines = [[(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)]]
engine_cover._bounded(box, outlines, [-0.5, 0.5], 0.01, "probe")

# f1/monocoque: `_tub_solid`'s `surfaces.obox(skin)` and
# `surfaces.is_valid_shape(tub)`; the row now proceeds to its next honest
# verdict -- a typed kernel refusal, never the OCC probe message.
verdict = "green"
try:
    mono_tub._tub_solid()
except truck123d.Refused as exc:
    verdict = "refused"
    assert "an OCC probe of a kernel-engine row" not in str(exc), exc
print(json.dumps({{"ok": True, "cockpit": "green", "engine_cover": "green", "monocoque": verdict}}))
"#
    );
    let args = probe_args();
    let stdout = run_python(&script, &[args[0].as_str(), args[1].as_str()]);
    let record: serde_json::Value = serde_json::from_str(stdout.trim()).expect("rows json");
    assert_eq!(record["ok"], true);
    assert_eq!(record["cockpit"], "green");
    assert_eq!(record["engine_cover"], "green");
    assert_eq!(record["monocoque"], "refused");
}
