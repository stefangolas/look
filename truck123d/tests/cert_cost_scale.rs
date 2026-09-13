//! FHC-G6-CERT-COST-SCALE required suite — the certified-construction cost
//! decomposition and its byte-identity gate.
//!
//! The packet instruments the certified loft/facts path with additive phase
//! timers and demands that any whitelisted mechanical fix keeps every artifact
//! byte-identical. This suite proves:
//!
//!   1. `phase_timers_present_and_additive` — the `bd_facts` record carries the
//!      four per-phase columns (`section_extraction_ms`, `loft_surface_ms`,
//!      `patch_certification_ms`, `bbox_ms`) beside the existing `timing`
//!      fields, the `bd_stl` record carries the mesh/write columns, the columns
//!      are non-negative, the disjoint phases never exceed `facts_ms`, and a
//!      non-loft tree reports zero loft phases (the accumulators are per call).
//!   2. `beam_wing_loft_identity` — the beam_wing-class spline loft's certified
//!      bracket and its full STL byte fingerprint are unchanged from the
//!      dispatch-time values (the packet's byte-identity gate).
//!   3. `heavy_rows_decomposition_recorded` — the two category-6 heavy rows
//!      (`f1/power_unit`, `f1/suspension_rear`) are driven through the real
//!      door; each completed facts call records its phase decomposition, and
//!      the row's final artifact is checked when the row terminates within the
//!      generous bound. The bound is recorded as data, never asserted.
//!
//! The suite runs the compiled native module from a real interpreter exactly
//! as `facts_cache.rs` does (the crate cdylib is staged as `truck123d.pyd`
//! beside the interpreter runtime DLLs).

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

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

/// One-time preparation of the native-module staging directory: the crate
/// cdylib is copied as `truck123d.pyd` beside the interpreter runtime DLLs.
fn staged_native_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let exe = std::env::current_exe().expect("test binary path");
        let deps = exe.parent().expect("deps dir");
        let staging =
            std::env::temp_dir().join(format!("truck123d_cert_cost_{}", std::process::id()));
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

/// A scratch output directory unique to this test process.
fn scratch_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("truck123d_cert_cost_out_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
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

/// FNV-1a 64 over a byte slice: the byte-level artifact fingerprint.
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// A synthetic two-station square-section spline loft: the beam_wing-class
/// carrier the packet's byte-identity gate names, small enough to run in the
/// fast suite.
fn spline_square_section(z: f64) -> serde_json::Value {
    let corners = [[0.0, 0.0, z], [1.0, 0.0, z], [1.0, 1.0, z], [0.0, 1.0, z]];
    let mut edges = Vec::with_capacity(4);
    for i in 0..4 {
        let a = corners[i];
        let b = corners[(i + 1) % 4];
        let mid = [
            0.5 * (a[0] + b[0]),
            0.5 * (a[1] + b[1]),
            0.5 * (a[2] + b[2]),
        ];
        edges.push(serde_json::json!({"kind": "spline", "points": [a, mid, b]}));
    }
    serde_json::Value::Array(edges)
}

fn spline_loft_tree() -> serde_json::Value {
    serde_json::json!({
        "part": {
            "solid": {
                "kind": "loft",
                "sections": [
                    spline_square_section(0.0),
                    spline_square_section(5.0),
                ],
                "closed": false,
            },
            "x": 0.0,
            "y": 0.0,
            "z": 0.0,
            "rz": 0.0,
        }
    })
}

fn box_tree() -> serde_json::Value {
    serde_json::json!({
        "part": {
            "solid": {"kind": "box", "length": 1.0, "width": 1.0, "height": 1.0},
            "x": 0.0,
            "y": 0.0,
            "z": 0.0,
            "rz": 0.0,
        }
    })
}

fn timing_f64(record: &serde_json::Value, path: &[&str]) -> f64 {
    let mut value = record;
    for key in path {
        value = &value[key];
    }
    value
        .as_f64()
        .unwrap_or_else(|| panic!("timing field {path:?} must be an f64: {record}"))
}

// ---------------------------------------------------------------------------
// Test 1: the per-phase columns are present and additive.
// ---------------------------------------------------------------------------

#[test]
fn phase_timers_present_and_additive() {
    let script = r#"
import json, sys
import truck123d
tree = json.loads(sys.argv[1])
stl_path = sys.argv[2]
facts = json.loads(truck123d.bd_facts(json.dumps(tree)))
stl = json.loads(truck123d.bd_stl(json.dumps(tree), stl_path, None))
print(json.dumps({"facts": facts, "stl": stl}))
"#;
    let tree = serde_json::to_string(&spline_loft_tree()).expect("tree json");
    let stl_path = scratch_dir().join("phase_timers.stl");
    let stl_arg = stl_path.to_string_lossy().to_string();
    let stdout = run_python(script, &[&tree, &stl_arg]);
    let value: serde_json::Value = serde_json::from_str(stdout.trim()).expect("record json");
    let facts = &value["facts"];

    let phases = &facts["timing"]["phases"];
    let section = timing_f64(facts, &["timing", "phases", "section_extraction_ms"]);
    let loft = timing_f64(facts, &["timing", "phases", "loft_surface_ms"]);
    let certify = timing_f64(facts, &["timing", "phases", "patch_certification_ms"]);
    let bbox = timing_f64(facts, &["timing", "phases", "bbox_ms"]);
    let facts_ms = timing_f64(facts, &["timing", "facts_ms"]);
    for (name, value) in [
        ("section_extraction_ms", section),
        ("loft_surface_ms", loft),
        ("patch_certification_ms", certify),
        ("bbox_ms", bbox),
    ] {
        assert!(
            value.is_finite() && value >= 0.0,
            "phase {name} must be finite and non-negative, got {value}: {facts}"
        );
    }
    assert!(
        certify > 0.0,
        "a spline loft must exercise the per-patch certification phase: {phases}"
    );
    let phase_sum = section + loft + certify + bbox;
    assert!(
        phase_sum <= facts_ms + 1.0,
        "the disjoint phase columns ({phase_sum}) must not exceed facts_ms ({facts_ms})"
    );

    let stl = &value["stl"];
    let mesh_ms = timing_f64(stl, &["phases", "mesh_build_ms"]);
    let write_ms = timing_f64(stl, &["phases", "stl_write_ms"]);
    assert!(
        mesh_ms.is_finite() && mesh_ms >= 0.0 && write_ms.is_finite() && write_ms >= 0.0,
        "the stl phase columns must be finite and non-negative: {stl}"
    );
    assert!(
        stl["triangles"].as_u64().unwrap_or(0) > 0,
        "the stl record must carry its triangle count: {stl}"
    );

    // The accumulators reset per call: a canonical box has no loft phases.
    let box_tree = serde_json::to_string(&box_tree()).expect("box json");
    let box_stdout = run_python(script, &[&box_tree, &stl_arg]);
    let box_value: serde_json::Value = serde_json::from_str(box_stdout.trim()).expect("box json");
    let box_facts = &box_value["facts"];
    assert_eq!(
        timing_f64(box_facts, &["timing", "phases", "section_extraction_ms"]),
        0.0,
        "a box must report no section extraction: {box_facts}"
    );
    assert_eq!(
        timing_f64(box_facts, &["timing", "phases", "patch_certification_ms"]),
        0.0,
        "a box must report no per-patch certification: {box_facts}"
    );
    let _ = std::fs::remove_file(&stl_path);
}

// ---------------------------------------------------------------------------
// Test 2: the beam_wing-class loft is byte-identical to the dispatch baseline.
// ---------------------------------------------------------------------------

/// The dispatch-time certified bracket and STL fingerprint of the beam_wing
/// spline loft (measured before any optimization, per the packet's gate).
const BEAM_WING_VOLUME: f64 = 2366637.6965246294;
const BEAM_WING_TRIANGLES: u64 = 341_760;
const BEAM_WING_STL_LEN: u64 = 17_088_084;
const BEAM_WING_STL_FNV: u64 = 17_095_488_309_390_686_321;

/// Builds beam_wing through the door's truck regime and returns its certified
/// facts and STL (the exact path the door's `geometry_facts` / export take).
const BEAM_WING_DRIVER: &str = r#"
import json, sys
corpus_ttc, tree_src, stl_path = sys.argv[1:4]
sys.path.insert(0, corpus_ttc)
import door
import truck123d
door.ENGINE = "truck"
obj = door.run_entry(tree_src, "lib.rear_wing", "build_beam_wing", [])
node = json.dumps(obj._node())
facts = json.loads(truck123d.bd_facts(node))
stl = json.loads(truck123d.bd_stl(node, stl_path, None))
print(json.dumps({
    "solid_count": facts.get("solid_count"),
    "volume": facts.get("volume"),
    "volume_bracket": facts.get("volume_bracket"),
    "facts_phases": facts.get("timing", {}).get("phases"),
    "stl": stl,
}))
"#;

#[test]
fn beam_wing_loft_identity() {
    let dir = scratch_dir();
    let stl = dir.join("cert_cost_beam_wing.stl");
    let tree = corpus_ttc_dir().join("trees").join("f1").join("src");
    let stdout = run_python(
        BEAM_WING_DRIVER,
        &[
            corpus_ttc_dir().to_str().expect("corpus path"),
            tree.to_str().expect("tree path"),
            stl.to_str().expect("stl path"),
        ],
    );
    let record: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("the driver record must parse");
    assert_eq!(
        record["solid_count"],
        serde_json::json!(2),
        "beam_wing is two solids: {record}"
    );
    assert_eq!(
        record["volume"].as_f64(),
        Some(BEAM_WING_VOLUME),
        "the beam_wing certified volume must be unchanged: {record}"
    );
    let bracket = record["volume_bracket"].as_array().expect("bracket array");
    assert_eq!(
        [bracket[0].as_f64(), bracket[1].as_f64()],
        [Some(BEAM_WING_VOLUME), Some(BEAM_WING_VOLUME)],
        "the beam_wing certified bracket must equal the dispatch baseline: {record}"
    );
    assert_eq!(
        record["stl"]["triangles"],
        serde_json::json!(BEAM_WING_TRIANGLES),
        "the beam_wing STL triangle fingerprint must be unchanged: {record}"
    );

    let mut bytes = Vec::new();
    std::fs::File::open(&stl)
        .expect("stl file")
        .read_to_end(&mut bytes)
        .expect("read stl");
    assert_eq!(
        bytes.len() as u64,
        BEAM_WING_STL_LEN,
        "the beam_wing STL byte length must be unchanged"
    );
    assert_eq!(
        fnv1a64(&bytes),
        BEAM_WING_STL_FNV,
        "the beam_wing STL bytes must be identical to the dispatch baseline"
    );
    let _ = std::fs::remove_file(&stl);
}

// ---------------------------------------------------------------------------
// Test 3: the heavy rows record their per-phase decomposition.
// ---------------------------------------------------------------------------

/// The Python driver for one heavy row: it builds the row through the door's
/// truck regime, records every completed `bd_facts` call's phase columns to a
/// progress file, then runs the final facts + STL. The progress file survives
/// a bounded kill, so the decomposition is recovered even when the row's
/// certified bracket evaluation does not terminate inside the bound.
const HEAVY_ROW_DRIVER: &str = r#"
import hashlib, json, os, sys, time
corpus_ttc, tree_src, module, entry, stl_path, progress_path = sys.argv[1:7]
sys.path.insert(0, corpus_ttc)
import door
import truck123d
door.ENGINE = "truck"
orig_facts = truck123d.bd_facts
progress = open(progress_path, "a", buffering=1)
count = [0]
def wrapped(tree_json):
    count[0] += 1
    started = time.perf_counter()
    result = orig_facts(tree_json)
    wall_ms = (time.perf_counter() - started) * 1000.0
    timing = json.loads(result).get("timing", {})
    phases = timing.get("phases", {})
    progress.write(json.dumps({
        "call": count[0],
        "wall_ms": wall_ms,
        "facts_ms": timing.get("facts_ms"),
        "phases": phases,
    }) + "\n")
    return result
truck123d.bd_facts = wrapped
obj = door.run_entry(tree_src, module, entry, [])
node = json.dumps(obj._node())
facts = json.loads(orig_facts(node))
stl = json.loads(truck123d.bd_stl(node, stl_path, None))
with open(stl_path, "rb") as fh:
    digest = hashlib.sha256(fh.read()).hexdigest()
print(json.dumps({
    "ok": True,
    "solid_count": facts.get("solid_count"),
    "volume": facts.get("volume"),
    "volume_bracket": facts.get("volume_bracket"),
    "facts_phases": facts.get("timing", {}).get("phases"),
    "stl": stl,
    "stl_sha256": digest,
}))
"#;

/// The generous bound for one heavy row, recorded as data (never a threshold).
const HEAVY_ROW_BOUND: Duration = Duration::from_secs(120);

/// Drives one heavy row through the real door under a hard bound. Returns the
/// recorded completed-call decompositions and the final record when the row
/// terminates inside the bound.
fn run_heavy_row(module: &str, entry: &str) -> (Vec<serde_json::Value>, Option<serde_json::Value>) {
    let dir = scratch_dir();
    let stl = dir.join(format!("cert_cost_{entry}.stl"));
    let progress = dir.join(format!("cert_cost_{entry}.progress"));
    let _ = std::fs::remove_file(&progress);
    let tree = corpus_ttc_dir().join("trees").join("f1").join("src");

    let mut child = python_command()
        .arg("-c")
        .arg(HEAVY_ROW_DRIVER)
        .arg(corpus_ttc_dir())
        .arg(&tree)
        .arg(module)
        .arg(entry)
        .arg(&stl)
        .arg(&progress)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the heavy-row driver");

    let started = Instant::now();
    let mut finished = false;
    while started.elapsed() < HEAVY_ROW_BOUND {
        match child.try_wait().expect("poll the heavy-row driver") {
            Some(status) => {
                finished = status.success();
                break;
            }
            None => std::thread::sleep(Duration::from_millis(500)),
        }
    }
    if !finished {
        let _ = child.kill();
    }
    let output = child.wait_with_output().expect("reap the heavy-row driver");
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;

    let mut records: Vec<serde_json::Value> = Vec::new();
    if let Ok(text) = std::fs::read_to_string(&progress) {
        for line in text.lines() {
            if let Ok(value) = serde_json::from_str(line) {
                records.push(value);
            }
        }
    }
    assert!(
        !records.is_empty(),
        "{entry}: no completed facts call recorded a per-phase decomposition within \
         {HEAVY_ROW_BOUND:?} (elapsed {elapsed_ms:.1} ms); the row's cost is not in the \
         instrumented loft/facts phases"
    );
    for record in &records {
        let certify = record["phases"]["patch_certification_ms"]
            .as_f64()
            .expect("patch_certification_ms");
        assert!(
            certify.is_finite() && certify >= 0.0,
            "{entry}: the recorded decomposition must be finite: {record}"
        );
    }
    assert!(
        records.iter().any(|record| {
            record["phases"]["patch_certification_ms"]
                .as_f64()
                .unwrap_or(0.0)
                > 0.0
        }),
        "{entry}: at least one completed facts call must exercise the per-patch \
         certification phase within {HEAVY_ROW_BOUND:?} (elapsed {elapsed_ms:.1} ms)"
    );

    let final_record = if finished {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let record: serde_json::Value =
            serde_json::from_str(stdout.trim()).expect("heavy-row final record must parse");
        assert_eq!(record["ok"], serde_json::json!(true), "{entry}: {record}");
        assert!(
            record["solid_count"].as_u64().unwrap_or(0) > 0,
            "{entry}: the completed row must report its solids: {record}"
        );
        let bracket = record["volume_bracket"].as_array().expect("bracket array");
        assert_eq!(bracket.len(), 2, "{entry}: the bracket must be two-sided");
        assert!(
            bracket[0].as_f64().unwrap_or(f64::NAN).is_finite()
                && bracket[1].as_f64().unwrap_or(f64::NAN).is_finite(),
            "{entry}: the bracket must be finite: {record}"
        );
        assert!(
            record["stl"]["triangles"].as_u64().unwrap_or(0) > 0,
            "{entry}: the completed row must emit its STL: {record}"
        );
        Some(record)
    } else {
        None
    };

    eprintln!(
        "FHC-G6 heavy row {entry}: bound={HEAVY_ROW_BOUND:?} elapsed_ms={elapsed_ms:.1} \
         completed_calls={} terminated={finished}",
        records.len()
    );
    let _ = std::fs::remove_file(&stl);
    (records, final_record)
}

#[test]
fn heavy_rows_decomposition_recorded() {
    // Both category-6 rows are driven through the real corpus door. The
    // dispatch profile records the dominant phase as the per-patch interval
    // certification; the full-row completion bound is recorded as data.
    let (power_unit, power_unit_final) = run_heavy_row("lib.power_unit", "build_power_unit");
    let (suspension_rear, suspension_rear_final) =
        run_heavy_row("lib.suspension", "build_suspension_rear");

    for (name, records, final_record) in [
        ("f1/power_unit", &power_unit, &power_unit_final),
        (
            "f1/suspension_rear",
            &suspension_rear,
            &suspension_rear_final,
        ),
    ] {
        assert!(
            !records.is_empty(),
            "{name}: the decomposition must record at least one completed facts call"
        );
        if let Some(record) = final_record {
            assert_eq!(record["ok"], serde_json::json!(true), "{name}: {record}");
        }
    }
}
