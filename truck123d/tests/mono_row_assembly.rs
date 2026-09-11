//! MONO-7-ROW-ASSEMBLY required suite — the monocoque row end to end.
//!
//! The monocoque row is `{"group": [styled(shell), styled(hoop)]}`
//! (`corpus/ttc/trees/f1/src/lib/mono_tub.py:748-749`). This suite proves the
//! row-assembly vocabulary passes through exactly:
//!
//!   1. `labelled_group_emits_rows_and_timing` — a labelled group of three
//!      parts (a 42-station spline-section loft at the tub's scale, a blade
//!      member and a mirrored member) submits; the aggregate facts are
//!      unchanged and the group carries `rows` (one per immediate child) with
//!      the same arithmetic, the labels round-trip, and the `timing` columns
//!      are present and `>= 0.0`.
//!   2. `nested_group_is_one_row_with_its_aggregate` — a nested group appears
//!      as ONE row carrying its own immediate-child aggregate (the nested-group
//!      weights-zero rule is unchanged).
//!   3. `part_without_metadata_omits_keys` — a part with no metadata omits the
//!      keys entirely (absent, never null); the door's `_node()` records the
//!      metadata when set.
//!   4. `monocoque_boolean_refuses_typed_at_the_carrier_boundary` — the
//!      monocoque-shaped boolean (a loft cut by its cavity loft) refuses typed
//!      at the boolean boundary naming the carrier. This is the measured
//!      boundary, not a failure: the Swept x Swept cut wiring is the admission
//!      chain's cell.
//!
//! The suite runs the compiled native module from a real interpreter exactly as
//! `ttc_authoring_arms.rs` does (the crate cdylib is staged as `truck123d.pyd`
//! beside the interpreter runtime DLLs). Geometry expectations are independent
//! JSON fixtures, never re-derived through the door.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

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

/// One-time preparation of the native-module staging directory (python
/// subprocesses need `truck123d` importable from a real interpreter). Mirrors
/// the ttc_authoring_arms / ttc_lathe_spline staging: the crate cdylib is
/// copied as `truck123d.pyd` beside the interpreter runtime DLLs.
fn staged_native_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let exe = std::env::current_exe().expect("test binary path");
        let deps = exe.parent().expect("deps dir");
        let staging =
            std::env::temp_dir().join(format!("truck123d_mono_row_{}", std::process::id()));
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

// ---------------------------------------------------------------------------
// Fixture builders (the kernel-row vocabulary, independent of the door).
// ---------------------------------------------------------------------------

/// One placed part row around a solid, with optional metadata.
fn part_row(
    solid: serde_json::Value,
    label: Option<&str>,
    color: Option<&str>,
) -> serde_json::Value {
    let mut node = serde_json::json!({
        "solid": solid,
        "x": 0.0,
        "y": 0.0,
        "z": 0.0,
        "rz": 0.0,
    });
    if let Some(label) = label {
        node["label"] = serde_json::json!(label);
    }
    if let Some(color) = color {
        node["color"] = serde_json::json!(color);
    }
    serde_json::json!({ "part": node })
}

/// A placed part with the mirror placed-carrier transform.
fn mirrored_part_row(
    solid: serde_json::Value,
    axis: &str,
    label: Option<&str>,
    color: Option<&str>,
) -> serde_json::Value {
    let mut row = part_row(solid, label, color);
    row["part"]["mirror"] = serde_json::json!(axis);
    row
}

/// A closed section loop of four quadratic spline edges bulging outward from a
/// square of side `scale` centered on the section origin at station `z` (the
/// MONO-2 N-station fixture family).
fn nstation_lens(z: f64, scale: f64) -> serde_json::Value {
    let corners: [[f64; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let center: [f64; 2] = [0.5, 0.5];
    let mut edges = Vec::with_capacity(4);
    for i in 0..4 {
        let a = corners[i];
        let b = corners[(i + 1) % 4];
        let mid = [0.5 * (a[0] + b[0]), 0.5 * (a[1] + b[1])];
        let dx = mid[0] - center[0];
        let dy = mid[1] - center[1];
        let len = (dx * dx + dy * dy).sqrt();
        let outward = [mid[0] + 0.15 * dx / len, mid[1] + 0.15 * dy / len];
        let point = |q: [f64; 2]| [scale * (q[0] - 0.5), scale * (q[1] - 0.5), z];
        edges.push(serde_json::json!({
            "kind": "spline",
            "points": [point(a), point(outward), point(b)],
        }));
    }
    serde_json::json!(edges)
}

/// The synthetic N-station stack at the monocoque tub's scale.
fn nstation_sections(n: usize) -> Vec<serde_json::Value> {
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let scale = 1.0 + 0.06 * i as f64;
        let z = 2.0 * i as f64 + 0.05 * (i * i) as f64;
        out.push(nstation_lens(z, scale));
    }
    out
}

/// The 42-station spline-section loft row (the tub scale).
fn loft_solid() -> serde_json::Value {
    serde_json::json!({
        "kind": "loft",
        "closed": false,
        "sections": nstation_sections(42),
    })
}

/// A closed section loop of four spline edges through the unit-square corners
/// (a blade plate profile; geometrically a unit square).
fn spline_square(z0: f64) -> serde_json::Value {
    let corners = [
        [0.0, 0.0, z0],
        [1.0, 0.0, z0],
        [1.0, 1.0, z0],
        [0.0, 1.0, z0],
    ];
    let mut edges = Vec::with_capacity(4);
    for i in 0..4 {
        let a = corners[i];
        let b = corners[(i + 1) % 4];
        let mid = [
            0.5 * (a[0] + b[0]),
            0.5 * (a[1] + b[1]),
            0.5 * (a[2] + b[2]),
        ];
        edges.push(serde_json::json!({
            "kind": "spline",
            "points": [a, mid, b],
        }));
    }
    serde_json::json!(edges)
}

/// One identity station frame at `z`.
fn member_station(z: f64) -> serde_json::Value {
    serde_json::json!({
        "origin": [0.0, 0.0, z],
        "x_dir": [1.0, 0.0, 0.0],
        "y_dir": [0.0, 1.0, 0.0],
        "z_dir": [0.0, 0.0, 1.0],
    })
}

/// A three-station ruled plate member: the unit-square plate carried by spline
/// edges swept along +z (volume 5).
fn member_solid() -> serde_json::Value {
    serde_json::json!({
        "kind": "member",
        "profile": spline_square(0.0),
        "stations": [
            member_station(0.0),
            member_station(2.5),
            member_station(5.0),
        ],
        "ruled": true,
    })
}

/// A canonical box row (cheap aggregate arithmetic).
fn box_solid(extent: f64) -> serde_json::Value {
    serde_json::json!({
        "kind": "box",
        "length": extent,
        "width": extent,
        "height": extent,
    })
}

// ---------------------------------------------------------------------------
// Test 1: a labelled group of three parts carries rows, labels and timing.
// ---------------------------------------------------------------------------

#[test]
fn labelled_group_emits_rows_and_timing() {
    let group = serde_json::json!({
        "group": [
            part_row(loft_solid(), Some("survival_cell"), Some("#aabbcc")),
            part_row(member_solid(), Some("roll_structure"), Some("#ddeeff")),
            mirrored_part_row(member_solid(), "y", Some("roll_structure_mirror"), Some("#ddeeff")),
        ],
        "label": "monocoque_row",
        "color": "#123456",
    })
    .to_string();

    let facts = native_facts(&group);

    // The top node's metadata round-trips verbatim.
    assert_eq!(facts["solid_count"], serde_json::json!(3));
    assert_eq!(facts["label"], serde_json::json!("monocoque_row"));
    assert_eq!(facts["color"], serde_json::json!("#123456"));

    // The per-row breakdown: one entry per immediate child, in script order.
    let rows = facts["rows"].as_array().expect("a group carries rows");
    assert_eq!(rows.len(), 3, "one row per immediate child: {rows:?}");
    assert_eq!(rows[0]["label"], serde_json::json!("survival_cell"));
    assert_eq!(rows[1]["label"], serde_json::json!("roll_structure"));
    assert_eq!(rows[2]["label"], serde_json::json!("roll_structure_mirror"));
    for row in rows {
        assert_eq!(row["solid_count"], serde_json::json!(1));
        assert!(row["volume"].as_f64().expect("row volume") > 0.0);
        let bbox = row["bbox"].as_array().expect("row bbox");
        assert_eq!(bbox.len(), 2);
    }

    // The aggregate path is unchanged: the rows' volume sum and bbox union are
    // exactly the top-level facts for a group of three immediate parts.
    let top_volume = facts["volume"].as_f64().expect("aggregate volume");
    let row_volume: f64 = rows
        .iter()
        .map(|row| row["volume"].as_f64().expect("row volume"))
        .sum();
    assert!(
        (top_volume - row_volume).abs() <= 1e-9 * (1.0 + top_volume.abs()),
        "aggregate {top_volume} must equal the rows' sum {row_volume}"
    );

    let read_corner = |corner: &serde_json::Value| -> [f64; 3] {
        let c = corner.as_array().expect("corner");
        [
            c[0].as_f64().expect("x"),
            c[1].as_f64().expect("y"),
            c[2].as_f64().expect("z"),
        ]
    };
    let top_bbox = facts["bbox"].as_array().expect("aggregate bbox");
    let top_lo = read_corner(&top_bbox[0]);
    let top_hi = read_corner(&top_bbox[1]);
    let mut row_lo = [f64::INFINITY; 3];
    let mut row_hi = [f64::NEG_INFINITY; 3];
    for row in rows {
        let bbox = row["bbox"].as_array().expect("row bbox");
        let lo = read_corner(&bbox[0]);
        let hi = read_corner(&bbox[1]);
        for axis in 0..3 {
            row_lo[axis] = row_lo[axis].min(lo[axis]);
            row_hi[axis] = row_hi[axis].max(hi[axis]);
        }
    }
    assert_eq!(row_lo, top_lo, "the rows' bbox union must be the aggregate");
    assert_eq!(row_hi, top_hi, "the rows' bbox union must be the aggregate");

    // Timing columns are present and diagnostic only (`>= 0.0`).
    let timing = facts["timing"].as_object().expect("timing is emitted");
    let construct_ms = timing["construct_ms"].as_f64().expect("construct_ms");
    let facts_ms = timing["facts_ms"].as_f64().expect("facts_ms");
    assert!(construct_ms >= 0.0, "construct_ms {construct_ms}");
    assert!(facts_ms >= 0.0, "facts_ms {facts_ms}");
}

// ---------------------------------------------------------------------------
// Test 2: a nested group appears as one row carrying its own aggregate.
// ---------------------------------------------------------------------------

#[test]
fn nested_group_is_one_row_with_its_aggregate() {
    let inner = serde_json::json!({
        "group": [
            part_row(box_solid(2.0), Some("inner_a"), None),
            part_row(box_solid(3.0), Some("inner_b"), None),
        ],
        "label": "inner_group",
    });
    let group = serde_json::json!({
        "group": [
            part_row(box_solid(1.0), Some("outer"), None),
            inner,
        ],
        "label": "outer_group",
    })
    .to_string();

    let facts = native_facts(&group);
    assert_eq!(facts["solid_count"], serde_json::json!(3));
    let rows = facts["rows"].as_array().expect("rows");
    assert_eq!(rows.len(), 2, "the nested group is ONE row");
    assert_eq!(rows[0]["label"], serde_json::json!("outer"));
    assert_eq!(rows[0]["solid_count"], serde_json::json!(1));
    assert_eq!(rows[1]["label"], serde_json::json!("inner_group"));
    assert_eq!(rows[1]["solid_count"], serde_json::json!(2));

    // The nested row carries the nested group's own immediate-child aggregate
    // (2^3 + 3^3 = 35).
    let inner_volume = rows[1]["volume"].as_f64().expect("inner volume");
    assert!(
        (inner_volume - 35.0).abs() < 1e-9,
        "nested row volume {inner_volume} must be its aggregate 35"
    );

    // The top-level aggregate is unchanged by the nested-group weights-zero
    // rule: only the immediate part contributes (1^3 = 1).
    let top_volume = facts["volume"].as_f64().expect("aggregate volume");
    assert!(
        (top_volume - 1.0).abs() < 1e-9,
        "aggregate volume {top_volume} must weight the nested group zero"
    );
}

// ---------------------------------------------------------------------------
// Test 3: metadata is absent when unset and recorded verbatim by the door.
// ---------------------------------------------------------------------------

#[test]
fn part_without_metadata_omits_keys() {
    // A bare part: the label/color keys and the group-only rows key are all
    // ABSENT (never null).
    let bare = part_row(box_solid(1.0), None, None).to_string();
    let facts = native_facts(&bare);
    assert!(
        facts.get("label").is_none(),
        "an unset label must be absent, got {facts}"
    );
    assert!(
        facts.get("color").is_none(),
        "an unset color must be absent, got {facts}"
    );
    assert!(
        facts.get("rows").is_none(),
        "a part top node carries no rows, got {facts}"
    );

    // The door records the metadata when set and omits it when unset.
    let script = r#"
import json, sys
sys.path.insert(0, sys.argv[1])
import door
import truck123d
door._T123D = truck123d
bd = door._build_truck_module()

bare = bd.Box(1, 1, 1)
bare_node = bare._node()["part"]
assert "label" not in bare_node, bare_node
assert "color" not in bare_node, bare_node

styled = bd.Box(2, 2, 2)
styled.label = "labelled_box"
styled.color = (0.25, 0.5, 0.75)
styled_node = styled._node()["part"]
assert styled_node["label"] == "labelled_box", styled_node
assert "color" in styled_node, styled_node

group = bd.Compound(children=[styled])
group.label = "the_group"
group_node = group._node()
assert group_node["label"] == "the_group", group_node

facts = json.loads(truck123d.bd_facts(json.dumps(group_node)))
assert facts["label"] == "the_group", facts
assert facts["rows"][0]["label"] == "labelled_box", facts
print(json.dumps({"ok": True, "color": styled_node["color"], "rows": facts["rows"]}))
"#;
    let stdout = run_python(script, &[corpus_ttc_dir().to_str().expect("corpus path")]);
    let record: serde_json::Value = serde_json::from_str(stdout.trim()).expect("door metadata");
    assert_eq!(record["ok"], serde_json::json!(true));
    assert!(
        record["color"].is_string(),
        "the door records a color string, got {}",
        record["color"]
    );
}

// ---------------------------------------------------------------------------
// Test 4: the monocoque-shaped boolean refuses typed at the carrier boundary.
// ---------------------------------------------------------------------------

#[test]
fn monocoque_boolean_refuses_typed_at_the_carrier_boundary() {
    // The monocoque tub is `cut(swept_shell, swept_cavity)` — a Swept x Swept
    // pair. The landed certified funnel does not admit a boolean between two
    // spline-loft solids, so the pair answers the typed constructive-carrier
    // refusal at the boolean boundary. This is the recorded boundary, not a
    // failure: the Swept x Swept cut wiring is the admission chain's cell.
    let boolean = serde_json::json!({
        "boolean": {
            "mode": "subtract",
            "a": part_row(loft_solid(), None, None),
            "b": part_row(loft_solid(), None, None),
        }
    })
    .to_string();

    // The two-loft tree is larger than the Windows command line allows, so the
    // submitted JSON travels through a scratch file (distinct from the native
    // staging directory, which must survive the test).
    let dir =
        std::env::temp_dir().join(format!("truck123d_mono_row_scratch_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let path = dir.join("mono_boolean.json");
    std::fs::write(&path, &boolean).expect("write the boolean tree");

    let script = r#"
import json, sys
import truck123d
with open(sys.argv[1], "r", encoding="utf-8") as fh:
    tree = fh.read()
try:
    truck123d.bd_facts(tree)
except truck123d.Refused as exc:
    payload = getattr(exc, "payload", {}) or {}
    print(json.dumps({
        "refused": True,
        "class": "Refused",
        "case": payload.get("case"),
        "envelope": payload.get("envelope"),
    }))
except Exception as exc:  # noqa: BLE001 - a wrong class is the boundary failure
    print(json.dumps({"refused": False, "class": type(exc).__name__, "message": str(exc)}))
else:
    print(json.dumps({"refused": False, "class": "none"}))
"#;
    let stdout = run_python(script, &[path.to_str().expect("scratch path")]);
    let _ = std::fs::remove_dir_all(&dir);
    let record: serde_json::Value = serde_json::from_str(stdout.trim()).expect("refusal record");
    println!("MONO-7 boolean boundary: {record}");
    assert_eq!(
        record["refused"],
        serde_json::json!(true),
        "the Swept x Swept cut must refuse typed: {record}"
    );
    assert_eq!(record["class"], serde_json::json!("Refused"));
    assert_eq!(
        record["envelope"],
        serde_json::json!("non_canonical_carrier"),
        "the refusal must name the carrier at the boolean boundary: {record}"
    );
}
