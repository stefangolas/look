//! FHC-C-AUTHORING-FIDELITY required suite — the three authoring-surface gaps
//! (`truck123d/tests/authoring_fidelity.rs`, the crate test file booked by the
//! packet).
//!
//! 1. `Solid.make_loft` is answered (the corpus structural members build a
//!    loft from placed section Wires) and routes to the landed N-station loft.
//! 2. `FilletPolyline` decomposes a polyline with arc-filleted corners EXACTLY
//!    into its line-arc-line chain (every piece a landed carrier); the chain
//!    is asserted against hand-computed tangent points and extrudes green
//!    through the landed line+arc `profile_loop`.
//! 3. A path spline's interpolation options dispatch to the landed
//!    interpolants: no tangents -> the Lagrange clamped cubic, `tangents=(t0,
//!    t1)` -> the Hermite clamped cubic, any other option refuses TYPED naming
//!    the option (never an approximation).
//!
//! The corpus rows are spot-checked: `hypercar/suspension_rear` clears the
//! untyped `make_loft` `AttributeError` and `hypercar/wheels` clears the
//! `FilletPolyline` refusal, each reaching its NEXT honest carrier with a
//! typed verdict.

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
        let staging = std::env::temp_dir().join(format!("truck123d_fhc_c_{}", std::process::id()));
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

/// Runs a python `-c` script with the corpus `ttc` dir as `argv[1]`; parses the
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
    let stl = std::env::temp_dir().join(format!("ttc_fhc_c_{}_{}.stl", std::process::id(), entry));
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

/// The common preamble: import the real native module and the corpus door.
const PREAMBLE: &str = r#"
import json, math, os, sys, tempfile
sys.path.insert(0, sys.argv[1])
import truck123d
import door
door._T123D = truck123d
bd = door._build_truck_module()
"#;

/// Asserts two floats agree to a tight relative tolerance.
fn close(got: f64, expect: f64, label: &str) {
    assert!(
        (got - expect).abs() <= 1e-9 * expect.abs().max(1.0),
        "{label}: got {got}, expected {expect}"
    );
}

/// The chord-length parameters of a planar path.
fn chord_params(points: &[[f64; 2]]) -> Vec<f64> {
    let mut params = vec![0.0];
    for pair in points.windows(2) {
        let dx = pair[1][0] - pair[0][0];
        let dy = pair[1][1] - pair[0][1];
        let next = params.last().copied().unwrap_or(0.0) + (dx * dx + dy * dy).sqrt();
        params.push(next);
    }
    params
}

/// The derivative at the first (or last) node of the degree-3 Lagrange
/// polynomial through four planar samples at their chord parameters (the
/// landed Lagrange end slope the door's default spline uses).
fn lagrange_end_slope(points: &[[f64; 2]], params: &[f64], first: bool) -> [f64; 2] {
    let p = params;
    let factors = if first {
        [
            1.0 / (p[0] - p[1]) + 1.0 / (p[0] - p[2]) + 1.0 / (p[0] - p[3]),
            1.0 / (p[1] - p[0]) * ((p[0] - p[2]) / (p[1] - p[2])) * ((p[0] - p[3]) / (p[1] - p[3])),
            1.0 / (p[2] - p[0]) * ((p[0] - p[1]) / (p[2] - p[1])) * ((p[0] - p[3]) / (p[2] - p[3])),
            1.0 / (p[3] - p[0]) * ((p[0] - p[1]) / (p[3] - p[1])) * ((p[0] - p[2]) / (p[3] - p[2])),
        ]
    } else {
        [
            1.0 / (p[0] - p[3]) * ((p[3] - p[1]) / (p[0] - p[1])) * ((p[3] - p[2]) / (p[0] - p[2])),
            1.0 / (p[1] - p[3]) * ((p[3] - p[0]) / (p[1] - p[0])) * ((p[3] - p[2]) / (p[1] - p[2])),
            1.0 / (p[2] - p[3]) * ((p[3] - p[0]) / (p[2] - p[0])) * ((p[3] - p[1]) / (p[2] - p[1])),
            1.0 / (p[3] - p[0]) + 1.0 / (p[3] - p[1]) + 1.0 / (p[3] - p[2]),
        ]
    };
    let mut out = [0.0, 0.0];
    for i in 0..4 {
        out[0] += points[i][0] * factors[i];
        out[1] += points[i][1] * factors[i];
    }
    out
}

// ---------------------------------------------------------------------------
// 1. make_loft
// ---------------------------------------------------------------------------

#[test]
fn make_loft_routes_to_the_landed_nstation_loft() {
    // `Solid.make_loft([wires])` builds the corpus structural-member carrier:
    // a unit square swept five units is an exact 5.0 loft with the degenerate
    // certificate bracket and a deterministic STL.
    let script = String::from(PREAMBLE)
        + r#"
def v(x, y, z):
    return door.Vector(x, y, z)

def square(z):
    pts = [(0.0, 0.0, z), (1.0, 0.0, z), (1.0, 1.0, z), (0.0, 1.0, z)]
    return door.Wire([
        door.Edge.make_line(v(*pts[i]), v(*pts[(i + 1) % 4])) for i in range(4)
    ])

part = bd.Solid.make_loft([square(0.0), square(5.0)])
facts = json.loads(truck123d.bd_facts(json.dumps(part._node())))
path = os.path.join(tempfile.gettempdir(), "ttc_fhc_c_loft.stl")
stl = json.loads(truck123d.bd_stl(json.dumps(part._node()), path, None))
print(json.dumps({
    "kind": part._node()["part"]["solid"]["kind"],
    "facts": facts,
    "triangles": stl["triangles"],
    "exists": os.path.exists(path),
}))
"#;
    let record = run_script(&script);
    assert_eq!(
        record["kind"], "loft",
        "make_loft must record the landed Loft carrier: {record}"
    );
    let facts = &record["facts"];
    assert_eq!(facts["solid_count"], 1, "make_loft solid count: {facts}");
    close(
        facts["volume"].as_f64().expect("volume"),
        5.0,
        "make_loft volume",
    );
    let bracket = facts["volume_bracket"].as_array().expect("bracket");
    assert_eq!(bracket[0].as_f64(), bracket[1].as_f64(), "exact bracket");
    assert_eq!(facts["bbox"][0], serde_json::json!([0.0, 0.0, 0.0]));
    assert_eq!(facts["bbox"][1], serde_json::json!([1.0, 1.0, 5.0]));
    assert_eq!(record["exists"], true, "make_loft STL exists");
    assert!(
        record["triangles"].as_u64().unwrap_or(0) > 0,
        "make_loft STL carries triangles: {record}"
    );
}

// ---------------------------------------------------------------------------
// 2. FilletPolyline
// ---------------------------------------------------------------------------

/// The corpus `_blade_section` call site: a 100 x 10 chamfered hexagon with
/// 2 mm corner fillets (`radius = thick * 0.20`).
const FILLET_SCRIPT: &str = r#"
chord, thick, flat = 100.0, 10.0, 0.56
hc, ht = chord / 2.0, thick / 2.0
radius = thick * 0.20
pts = [
    (-hc, -0.10 * thick),
    (-flat * hc, ht),
    (flat * hc, ht),
    (hc, -0.10 * thick),
    (min(flat + 0.12, 0.86) * hc, -ht),
    (-min(flat + 0.12, 0.86) * hc, -ht),
]
"#;

#[test]
fn fillet_polyline_decomposes_to_the_line_arc_chain() {
    // The corpus hexagon decomposes into six lines and six exact arcs; the
    // tangent points and arc centres below are hand-computed from the
    // `fillet_2d` call-site convention (tangent length `r tan(turn/2)`, centre
    // on the interior bisector at `r / cos(turn/2)`) and independently
    // reproduced by the OCC reference.
    let script = String::from(PREAMBLE)
        + FILLET_SCRIPT
        + r#"
wire = bd.FilletPolyline(*pts, radius=radius, close=True)
edges = []
for edge in wire.edges:
    if edge.kind == "arc":
        edges.append({
            "kind": "arc",
            "center": list(edge.center.to_tuple()),
            "radius": edge.radius,
            "start": edge.start_angle,
            "end": edge.end_angle,
            "normal": list(edge.normal.to_tuple()),
        })
    else:
        edges.append({
            "kind": "line",
            "a": list(edge.p0.to_tuple()),
            "b": list(edge.p1.to_tuple()),
        })
print(json.dumps({"edges": edges}))
"#;
    let record = run_script(&script);
    let edges = record["edges"].as_array().expect("edges");
    assert_eq!(edges.len(), 12, "six lines + six arcs");
    let kinds: Vec<&str> = edges
        .iter()
        .map(|edge| edge["kind"].as_str().expect("kind"))
        .collect();
    assert_eq!(
        kinds,
        [
            "line", "arc", "line", "arc", "line", "arc", "line", "arc", "line", "arc", "line",
            "arc"
        ],
        "the chain alternates line and arc"
    );

    // Corner P1 = (-28, 5): incoming edge P0->P1 (a 15.255 deg turn).
    let line_in = &edges[2];
    let a = line_in["a"].as_array().expect("a");
    let b = line_in["b"].as_array().expect("b");
    close(a[0].as_f64().expect("x"), -42.616567082388, "P1 T_in x");
    close(a[1].as_f64().expect("y"), 1.013663522985, "P1 T_in y");
    close(b[0].as_f64().expect("x"), -28.258398644257, "P1 T_out x");
    close(b[1].as_f64().expect("y"), 4.929527642475, "P1 T_out y");
    let arc_p1 = &edges[3];
    let center = arc_p1["center"].as_array().expect("center");
    close(center[0].as_f64().expect("x"), -27.732163832672, "P1 arc x");
    close(center[1].as_f64().expect("y"), 3.0, "P1 arc y");
    close(
        arc_p1["radius"].as_f64().expect("radius"),
        2.0,
        "P1 arc radius",
    );

    // Corner P0 = (-50, -1): a 150.745 deg sharp corner.
    let arc_p0 = &edges[1];
    let center = arc_p0["center"].as_array().expect("center");
    close(center[0].as_f64().expect("x"), -42.090332270804, "P0 arc x");
    close(center[1].as_f64().expect("y"), -0.915864119490, "P0 arc y");
    close(
        arc_p0["radius"].as_f64().expect("radius"),
        2.0,
        "P0 arc radius",
    );

    // Every arc joins its neighbouring straight edges: its start tangent point
    // is the previous line's end and its end tangent point is the next line's
    // start (the `normal = -z` conic frame maps angle theta to
    // `center + r (cos theta, -sin theta)`).
    let point = |edge: &serde_json::Value, theta: f64| -> [f64; 2] {
        let center = edge["center"].as_array().expect("center");
        let cx = center[0].as_f64().expect("cx");
        let cy = center[1].as_f64().expect("cy");
        let r = edge["radius"].as_f64().expect("r");
        [cx + r * theta.cos(), cy - r * theta.sin()]
    };
    for k in (1..12).step_by(2) {
        let arc = &edges[k];
        let start = point(arc, arc["start"].as_f64().expect("start").to_radians());
        let end = point(arc, arc["end"].as_f64().expect("end").to_radians());
        let prev = &edges[k - 1];
        let prev_b = prev["b"].as_array().expect("prev b");
        close(start[0], prev_b[0].as_f64().expect("x"), "arc start x");
        close(start[1], prev_b[1].as_f64().expect("y"), "arc start y");
        let next = &edges[(k + 1) % 12];
        let next_a = next["a"].as_array().expect("next a");
        close(end[0], next_a[0].as_f64().expect("x"), "arc end x");
        close(end[1], next_a[1].as_f64().expect("y"), "arc end y");
    }
}

#[test]
fn fillet_polyline_prism_round_trip_is_green() {
    // The decomposed chain feeds the landed line+arc `profile_loop`: an exact
    // prism (facts bracket + deterministic STL), never a chord flattening.
    let script = String::from(PREAMBLE)
        + FILLET_SCRIPT
        + r#"
face = bd.make_face(bd.FilletPolyline(*pts, radius=radius, close=True))
part = bd.extrude(face, amount=20.0)
facts = json.loads(truck123d.bd_facts(json.dumps(part._node())))
path = os.path.join(tempfile.gettempdir(), "ttc_fhc_c_fillet.stl")
stl = json.loads(truck123d.bd_stl(json.dumps(part._node()), path, None))
print(json.dumps({
    "kind": part._node()["part"]["solid"]["kind"],
    "facts": facts,
    "triangles": stl["triangles"],
    "exists": os.path.exists(path),
}))
"#;
    let record = run_script(&script);
    assert_eq!(record["kind"], "prism", "the fillet profile extrudes");
    let facts = &record["facts"];
    assert_eq!(facts["solid_count"], 1, "prism solid count: {facts}");
    assert!(
        facts["volume"].as_f64().expect("volume") > 0.0,
        "prism volume is positive: {facts}"
    );
    let bracket = facts["volume_bracket"].as_array().expect("bracket");
    assert_eq!(
        bracket[0].as_f64(),
        bracket[1].as_f64(),
        "the line+arc prism bracket is exact: {facts}"
    );
    assert_eq!(record["exists"], true, "prism STL exists");
    assert!(
        record["triangles"].as_u64().unwrap_or(0) > 0,
        "prism STL carries triangles: {record}"
    );
}

// ---------------------------------------------------------------------------
// 3. Path spline interpolation options
// ---------------------------------------------------------------------------

/// The planar path used by the interpolation-option tests. It lies in the
/// `y = 0` plane, so its `(x, z)` samples are the 2-D reference path.
const SPLINE_POINTS: [[f64; 2]; 4] = [[0.0, 0.0], [1.0, 2.0], [3.0, 1.0], [4.0, 3.0]];

#[test]
fn lagrange_default_spline_selects_the_lagrange_interpolant() {
    // No tangents -> the landed Lagrange interpolant: the end derivative is
    // the degree-3 Lagrange end slope of the first four samples, exactly as
    // the bridge's `spline_spans` reconstructs.
    let script = String::from(PREAMBLE)
        + r#"
def v(x, y, z):
    return door.Vector(x, y, z)

pts = [v(0.0, 0.0, 0.0), v(1.0, 0.0, 2.0), v(3.0, 0.0, 1.0), v(4.0, 0.0, 3.0)]
edge = door.Edge.make_spline(pts)
rows = []
for t in (0.0, 0.25, 0.5, 0.75, 1.0):
    p = edge.position_at(t)
    d = edge.tangent_at(t)
    rows.append({"t": t, "p": list(p.to_tuple()), "d": list(d.to_tuple())})
print(json.dumps(rows))
"#;
    let rows: Vec<serde_json::Value> = run_script(&script)
        .as_array()
        .expect("lagrange rows")
        .clone();
    let params = chord_params(&SPLINE_POINTS);
    let total = *params.last().expect("total");
    let slope = lagrange_end_slope(&SPLINE_POINTS, &params, true);
    // Endpoint exactness.
    let first = rows[0]["p"].as_array().expect("first p");
    let last = rows[4]["p"].as_array().expect("last p");
    assert_eq!(first[0].as_f64(), Some(0.0));
    assert_eq!(first[2].as_f64(), Some(0.0));
    assert_eq!(last[0].as_f64(), Some(4.0));
    assert_eq!(last[2].as_f64(), Some(3.0));
    // The start tangent is `total * lagrange_end_slope` in the `(x, z)` plane.
    let d0 = rows[0]["d"].as_array().expect("start tangent");
    close(
        d0[0].as_f64().expect("dx"),
        total * slope[0],
        "lagrange start tangent x",
    );
    close(
        d0[2].as_f64().expect("dz"),
        total * slope[1],
        "lagrange start tangent z",
    );
    assert_eq!(d0[1].as_f64(), Some(0.0), "the path lies in y = 0");
}

#[test]
fn hermite_tangents_spline_selects_the_hermite_interpolant() {
    // `tangents=(t0, t1)` -> the landed Hermite interpolant: the end
    // derivative is the recorded tangent (scaled by the chord-length total),
    // and the curve differs from the Lagrange interpolant at the interior.
    let script = String::from(PREAMBLE)
        + r#"
def v(x, y, z):
    return door.Vector(x, y, z)

pts = [v(0.0, 0.0, 0.0), v(1.0, 0.0, 2.0), v(3.0, 0.0, 1.0), v(4.0, 0.0, 3.0)]
lag = door.Edge.make_spline(pts)
herm = door.Edge.make_spline(pts, tangents=((1.0, 0.0), (-1.0, 0.0)))
rows = []
for t in (0.0, 0.25, 0.5, 0.75, 1.0):
    rows.append({
        "t": t,
        "lag": list(lag.position_at(t).to_tuple()),
        "herm": list(herm.position_at(t).to_tuple()),
    })
print(json.dumps({
    "rows": rows,
    "d0": list(herm.tangent_at(0.0).to_tuple()),
    "d1": list(herm.tangent_at(1.0).to_tuple()),
}))
"#;
    let record = run_script(&script);
    let params = chord_params(&SPLINE_POINTS);
    let total = *params.last().expect("total");
    let d0 = record["d0"].as_array().expect("d0");
    let d1 = record["d1"].as_array().expect("d1");
    close(
        d0[0].as_f64().expect("d0x"),
        total,
        "hermite start tangent x",
    );
    close(d0[2].as_f64().expect("d0z"), 0.0, "hermite start tangent z");
    close(
        d1[0].as_f64().expect("d1x"),
        -total,
        "hermite end tangent x",
    );
    close(d1[2].as_f64().expect("d1z"), 0.0, "hermite end tangent z");
    // The two interpolants select different curves at the interior.
    let rows = record["rows"].as_array().expect("rows");
    let mid = &rows[2];
    let lag = mid["lag"].as_array().expect("lag");
    let herm = mid["herm"].as_array().expect("herm");
    assert!(
        (lag[0].as_f64().unwrap_or(0.0) - herm[0].as_f64().unwrap_or(0.0)).abs() > 1e-6
            || (lag[2].as_f64().unwrap_or(0.0) - herm[2].as_f64().unwrap_or(0.0)).abs() > 1e-6,
        "the tangents option must change the curve: {record}"
    );
    // Both interpolate the recorded samples at the endpoints.
    for (sample, t) in [(0usize, 0usize), (3usize, 4usize)] {
        let row = &rows[t];
        let herm = row["herm"].as_array().expect("herm endpoint");
        let want = SPLINE_POINTS[sample];
        close(herm[0].as_f64().expect("x"), want[0], "hermite endpoint x");
        close(herm[2].as_f64().expect("z"), want[1], "hermite endpoint z");
    }
}

#[test]
fn spline_option_outside_the_landed_set_refuses_typed() {
    // A parameter list selects a parameterization no landed interpolant
    // reproduces exactly: refuse TYPED naming the option, never approximate.
    let script = String::from(PREAMBLE)
        + r#"
def v(x, y, z):
    return door.Vector(x, y, z)

pts = [v(0.0, 0.0, 0.0), v(1.0, 0.0, 2.0), v(3.0, 0.0, 1.0), v(4.0, 0.0, 3.0)]
edge = door.Edge.make_spline(pts, parameters=(0.0, 1.0, 2.0, 3.0))
try:
    edge.position_at(0.5)
    print(json.dumps({"refused": False}))
except truck123d.Refused as exc:
    print(json.dumps({
        "refused": True,
        "typed": True,
        "carrier": getattr(exc, "carrier", None),
        "message": str(exc),
    }))
"#;
    let record = run_script(&script);
    assert_eq!(
        record["refused"], true,
        "an unsupported spline option must refuse typed: {record}"
    );
    let message = record["message"].as_str().unwrap_or("");
    assert!(
        message.contains("parameters"),
        "the refusal must name the option: {message}"
    );
    assert_eq!(
        record["carrier"], "path_spline",
        "the refusal must name the path-spline carrier: {record}"
    );
}

// ---------------------------------------------------------------------------
// Corpus spot-checks
// ---------------------------------------------------------------------------

#[test]
fn corpus_rows_clear_the_authoring_surface() {
    // suspension_rear clears the untyped `make_loft` AttributeError;
    // wheels clears the `FilletPolyline` refusal. Each reaches its next honest
    // carrier with a typed verdict (category-1 booleans / the line+arc loft).
    for (module, cleared) in [
        ("lib.suspension_rear", "make_loft"),
        ("lib.wheels", "FilletPolyline"),
    ] {
        let record = run_truck_door("hypercar", module, "build", "[]");
        assert_eq!(
            record["error"]["kind"], "Refused",
            "{module} must reach a typed verdict: {record}"
        );
        assert_eq!(
            record["error"]["typed"], true,
            "{module} must refuse typed: {record}"
        );
        let message = record["error"]["message"].as_str().unwrap_or("");
        let carrier = record["error"]["carrier"].as_str().unwrap_or("");
        assert!(
            !message.contains(cleared) && !carrier.contains(cleared),
            "{module} must clear the old {cleared} refusal: {record}"
        );
    }
}
