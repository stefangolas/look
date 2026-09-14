//! FHC-G1-RATIONAL-FLUX required suite -- the rational-weight patch flux
//! through the certified boolean funnel.
//!
//! The suite drives the door probe (`FacadeOp::RationalFluxProbe`) through the
//! single native facade entry for the kernel-level acceptance tests, and the
//! compiled native module through a real interpreter for the end-to-end
//! canonical CYLINDER+CYLINDER union (exactly as `rg4_boolean_product.rs`
//! does). Every assertion exercises the path the door reaches.
//!
//! Tests 1-11:
//!
//! 1. `unit_weight_cell_equivalence` -- the rational arm with `W = 1` equals
//!    the exact polynomial arm summed over a closed cycle.
//! 2. `constant_weight_gauge` -- `(P, W)` and `(lambda P, lambda W)` give the
//!    identical certified flux.
//! 3. `polynomial_oracle` -- a polynomial patch carried as a homogeneous patch
//!    collapses to the exact result.
//! 4. `placement_restriction_homogeneous` -- homogeneous subdivision agrees
//!    with the direct whole-cell evaluation.
//! 5. `positive_cylinder_admitted` -- a rational cylinder side patch is
//!    admitted (no `unsupported_envelope`).
//! 6. `weight_zero_refines_or_refuses` -- a non-positive weight refuses typed
//!    before the reciprocal kernel.
//! 7. `fold_partition_contains_whole` -- child certificates contain the whole
//!    integral.
//! 8. `anchor_uniformity_enforced` -- a mixed-anchor fold refuses typed.
//! 9. `end_to_end_cylinder_union` -- a canonical CYLINDER+CYLINDER union
//!    passes the entire `boolean_product_volume_certified` route.
//! 10. `tail_monotone_shrinks` -- increasing the reciprocal order strictly
//!     shrinks the certified width.
//! 11. `subdivision_order` -- uniform subdivision drives the total width at
//!     slope `r + 1`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use truck123d::{FacadeTable, run_facade};

// ---------------------------------------------------------------------------
// The door probe helpers.
// ---------------------------------------------------------------------------

/// Runs one rational-flux probe row and returns the `rational_flux` outcome
/// object of the serialized report.
fn probe(row: serde_json::Value) -> serde_json::Value {
    let table = serde_json::json!({ "ops": [row] });
    let table: FacadeTable = serde_json::from_value(table).expect("the probe table deserializes");
    let report = run_facade(&table).expect("the facade report is produced");
    let value = serde_json::to_value(&report).expect("the report serializes");
    value
        .get("rational_flux")
        .cloned()
        .expect("the report carries the rational-flux outcome")
}

fn field(value: &serde_json::Value, key: &str) -> f64 {
    value
        .get(key)
        .and_then(serde_json::Value::as_f64)
        .unwrap_or_else(|| panic!("missing numeric field {key}: {value}"))
}

fn refusal_tag(value: &serde_json::Value) -> String {
    value
        .get("refusal")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_else(|| panic!("expected a refusal tag: {value}"))
        .to_string()
}

/// The degree-3 Bernstein representation of the linear basis functions
/// `1 - t` and `t` (the exact tensor-Bernstein elevation of a bilinear quad).
const E0: [f64; 4] = [1.0, 2.0 / 3.0, 1.0 / 3.0, 0.0];
const E1: [f64; 4] = [0.0, 1.0 / 3.0, 2.0 / 3.0, 1.0];

/// One planar quad as an exact bicubic tensor-Bernstein patch row, corner order
/// `(c00, c10, c11, c01)`, unit weights.
fn face(c00: [f64; 3], c10: [f64; 3], c11: [f64; 3], c01: [f64; 3]) -> serde_json::Value {
    let corners = [[c00, c01], [c10, c11]];
    let mut numerator = vec![vec![[0.0f64; 3]; 4]; 4];
    for (i, row) in numerator.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            let mut acc = [0.0f64; 3];
            for (a, pair) in corners.iter().enumerate() {
                for (b, corner) in pair.iter().enumerate() {
                    let wa = if a == 0 { E0[i] } else { E1[i] };
                    let wb = if b == 0 { E0[j] } else { E1[j] };
                    let coefficient = wa * wb;
                    for (k, value) in corner.iter().enumerate() {
                        acc[k] += coefficient * value;
                    }
                }
            }
            *cell = acc;
        }
    }
    serde_json::json!({
        "numerator": numerator,
        "weights": vec![vec![1.0f64; 4]; 4],
        "orientation": 1.0,
    })
}

/// The six outward-oriented faces of the box `[lo, hi]`.
fn box_patches(lo: [f64; 3], hi: [f64; 3]) -> Vec<serde_json::Value> {
    let [x0, y0, z0] = lo;
    let [x1, y1, z1] = hi;
    vec![
        face([x0, y0, z0], [x0, y1, z0], [x1, y1, z0], [x1, y0, z0]),
        face([x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]),
        face([x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]),
        face([x0, y1, z0], [x0, y1, z1], [x1, y1, z1], [x1, y1, z0]),
        face([x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]),
        face([x1, y0, z0], [x1, y1, z0], [x1, y1, z1], [x1, y0, z1]),
    ]
}

/// The same quad as [`face`] with both the numerator and the weight scaled by
/// `lambda` (the exact gauge `(P, W) ~ (lambda P, lambda W)`).
fn scaled_face(
    c00: [f64; 3],
    c10: [f64; 3],
    c11: [f64; 3],
    c01: [f64; 3],
    lambda: f64,
) -> serde_json::Value {
    let mut value = face(c00, c10, c11, c01);
    if let Some(numerator) = value.get_mut("numerator").and_then(|v| v.as_array_mut()) {
        for row in numerator.iter_mut() {
            if let Some(cells) = row.as_array_mut() {
                for cell in cells.iter_mut() {
                    if let Some(coords) = cell.as_array_mut() {
                        for c in coords.iter_mut() {
                            if let Some(x) = c.as_f64() {
                                *c = serde_json::json!(lambda * x);
                            }
                        }
                    }
                }
            }
        }
    }
    let weights = value
        .get_mut("weights")
        .and_then(|v| v.as_array_mut())
        .expect("weights array");
    for row in weights.iter_mut() {
        if let Some(cells) = row.as_array_mut() {
            for cell in cells.iter_mut() {
                *cell = serde_json::json!(lambda);
            }
        }
    }
    value
}

/// One exact 90-degree rational-quadratic cylinder side patch: `u` runs the arc
/// (degree 2), `v` the axis (degree 1), weights `1, sqrt(2)/2, 1`.
fn cylinder_side_patch(r: f64, z0: f64, z1: f64, theta0: f64) -> serde_json::Value {
    let mut numerator = vec![vec![[0.0f64; 3]; 2]; 3];
    let mut weights = vec![vec![0.0f64; 2]; 3];
    for i in 0..3 {
        let angle = theta0 + (i as f64) * std::f64::consts::FRAC_PI_4;
        let scale = if i == 1 {
            std::f64::consts::SQRT_2
        } else {
            1.0
        };
        let w = if i == 1 {
            std::f64::consts::FRAC_1_SQRT_2
        } else {
            1.0
        };
        for (j, z) in [z0, z1].into_iter().enumerate() {
            numerator[i][j] = [
                w * r * scale * angle.cos(),
                w * r * scale * angle.sin(),
                w * z,
            ];
            weights[i][j] = w;
        }
    }
    serde_json::json!({
        "numerator": numerator,
        "weights": weights,
        "orientation": 1.0,
    })
}

/// Runs one `cell_flux` probe and returns its bracket list.
fn cell_flux(patches: Vec<serde_json::Value>, cells: Vec<[f64; 4]>) -> serde_json::Value {
    probe(serde_json::json!({
        "op": "rational_flux_probe",
        "kind": "cell_flux",
        "patches": patches,
        "cells": cells,
    }))
}

/// The brackets array of a `cell_flux` outcome.
fn brackets(value: &serde_json::Value) -> Vec<[f64; 2]> {
    value
        .get("brackets")
        .and_then(serde_json::Value::as_array)
        .expect("brackets")
        .iter()
        .map(|pair| {
            let array = pair.as_array().expect("bracket pair");
            [
                array[0].as_f64().expect("lo"),
                array[1].as_f64().expect("hi"),
            ]
        })
        .collect()
}

fn sum_brackets(value: &serde_json::Value) -> [f64; 2] {
    brackets(value)
        .into_iter()
        .fold([0.0, 0.0], |acc, b| [acc[0] + b[0], acc[1] + b[1]])
}

// ---------------------------------------------------------------------------
// Test 1: the rational arm with W = 1 equals the exact polynomial arm.
// ---------------------------------------------------------------------------

#[test]
fn unit_weight_cell_equivalence() {
    // A unit-weight box carried through the rational entry: `cell_flux_bracket`
    // parses it with the positive-weight admission and takes the exact
    // `cell_flux_exact` fast path (W = 1). The summed closed-cycle flux is the
    // box volume 2 * 3 * 4 = 24.
    let patches = box_patches([0.0, 0.0, 0.0], [2.0, 3.0, 4.0]);
    let outcome = cell_flux(patches, vec![[0.0, 1.0, 0.0, 1.0]]);
    assert_eq!(outcome["ok"], serde_json::json!(true), "{outcome}");
    let [lo, hi] = sum_brackets(&outcome);
    assert!(
        (lo - 24.0).abs() < 1.0e-6 && (hi - 24.0).abs() < 1.0e-6,
        "the rational arm with W = 1 must answer the exact 24, got [{lo}, {hi}]"
    );
}

// ---------------------------------------------------------------------------
// Test 2: constant-weight gauge.
// ---------------------------------------------------------------------------

#[test]
fn constant_weight_gauge() {
    let quad = (
        [0.0, 0.0, 1.0],
        [1.0, 0.0, 1.0],
        [1.0, 1.0, 1.0],
        [0.0, 1.0, 1.0],
    );
    let unit = face(quad.0, quad.1, quad.2, quad.3);
    let scaled = scaled_face(quad.0, quad.1, quad.2, quad.3, 3.0);
    let outcome = cell_flux(vec![unit, scaled], vec![[0.0, 1.0, 0.0, 1.0]]);
    let brackets = brackets(&outcome);
    assert_eq!(brackets.len(), 2, "{outcome}");
    assert!(
        (brackets[0][0] - brackets[1][0]).abs() < 1.0e-9
            && (brackets[0][1] - brackets[1][1]).abs() < 1.0e-9,
        "(P, W) and (3P, 3W) must certify identically, got {brackets:?}"
    );
}

// ---------------------------------------------------------------------------
// Test 3: the polynomial oracle.
// ---------------------------------------------------------------------------

#[test]
fn polynomial_oracle() {
    // The unit square at z = 1, outward +z: the divergence-form flux is
    // (1/3) * 1 * area = 1/3. The homogeneous representation (W = 1) must
    // collapse to that exact polynomial result.
    let patch = face(
        [0.0, 0.0, 1.0],
        [1.0, 0.0, 1.0],
        [1.0, 1.0, 1.0],
        [0.0, 1.0, 1.0],
    );
    let outcome = cell_flux(vec![patch], vec![[0.0, 1.0, 0.0, 1.0]]);
    let [lo, hi] = sum_brackets(&outcome);
    assert!(
        lo <= 1.0 / 3.0 && 1.0 / 3.0 <= hi,
        "the polynomial oracle 1/3 must lie in [{lo}, {hi}]"
    );
    assert!(
        (hi - lo).abs() < 1.0e-9,
        "a polynomial patch certifies exactly, got width {}",
        hi - lo
    );
}

// ---------------------------------------------------------------------------
// Test 4: homogeneous placement/restriction.
// ---------------------------------------------------------------------------

#[test]
fn placement_restriction_homogeneous() {
    // A genuinely rational patch (the cylinder side quarter). The whole-cell
    // certificate and the four homogeneous child certificates certify the same
    // integral: the child sum contains the whole.
    let patch = cylinder_side_patch(1.0, 0.0, 1.0, 0.0);
    let whole = cell_flux(vec![patch.clone()], vec![[0.0, 1.0, 0.0, 1.0]]);
    let children = cell_flux(
        vec![patch],
        vec![
            [0.0, 0.5, 0.0, 0.5],
            [0.5, 1.0, 0.0, 0.5],
            [0.0, 0.5, 0.5, 1.0],
            [0.5, 1.0, 0.5, 1.0],
        ],
    );
    let [wlo, whi] = sum_brackets(&whole);
    let [clo, chi] = sum_brackets(&children);
    assert!(
        clo <= wlo + 1.0e-9 && whi <= chi + 1.0e-9,
        "the child partition [{clo}, {chi}] must contain the whole [{wlo}, {whi}]"
    );
}

// ---------------------------------------------------------------------------
// Test 5: a positive rational cylinder patch is admitted.
// ---------------------------------------------------------------------------

#[test]
fn positive_cylinder_admitted() {
    let patch = cylinder_side_patch(2.0, -1.0, 1.0, 0.0);
    let outcome = cell_flux(vec![patch], vec![[0.0, 1.0, 0.0, 1.0]]);
    assert_eq!(outcome["ok"], serde_json::json!(true), "{outcome}");
    assert!(
        outcome.get("refusal").is_none(),
        "a positive rational cylinder patch must not refuse: {outcome}"
    );
    let [lo, hi] = sum_brackets(&outcome);
    assert!(
        lo.is_finite() && hi.is_finite() && lo <= hi,
        "the cylinder patch must certify a finite bracket, got [{lo}, {hi}]"
    );
}

// ---------------------------------------------------------------------------
// Test 6: a non-positive weight refuses typed before the kernel.
// ---------------------------------------------------------------------------

#[test]
fn weight_zero_refines_or_refuses() {
    let mut patch = cylinder_side_patch(1.0, 0.0, 1.0, 0.0);
    if let Some(weights) = patch.get_mut("weights").and_then(|v| v.as_array_mut()) {
        if let Some(cell) = weights[0].as_array_mut().and_then(|row| row.get_mut(0)) {
            *cell = serde_json::json!(0.0);
        }
    }
    let outcome = cell_flux(vec![patch], vec![[0.0, 1.0, 0.0, 1.0]]);
    assert_eq!(outcome["ok"], serde_json::json!(false), "{outcome}");
    assert_eq!(refusal_tag(&outcome), "rational_weights");
}

// ---------------------------------------------------------------------------
// Test 7: the fold partition contains the whole.
// ---------------------------------------------------------------------------

#[test]
fn fold_partition_contains_whole() {
    let patch = cylinder_side_patch(1.5, 0.0, 2.0, std::f64::consts::FRAC_PI_2);
    let whole = cell_flux(vec![patch.clone()], vec![[0.0, 1.0, 0.0, 1.0]]);
    let children = cell_flux(
        vec![patch],
        vec![
            [0.0, 0.5, 0.0, 0.5],
            [0.5, 1.0, 0.0, 0.5],
            [0.0, 0.5, 0.5, 1.0],
            [0.5, 1.0, 0.5, 1.0],
        ],
    );
    let [wlo, whi] = sum_brackets(&whole);
    let [clo, chi] = sum_brackets(&children);
    assert!(
        clo - 1.0e-9 <= wlo && whi <= chi + 1.0e-9,
        "the partitioned fold [{clo}, {chi}] must contain the whole [{wlo}, {whi}]"
    );
}

// ---------------------------------------------------------------------------
// Test 8: anchor uniformity is enforced by construction.
// ---------------------------------------------------------------------------

#[test]
fn anchor_uniformity_enforced() {
    let mut a = cylinder_side_patch(1.0, 0.0, 1.0, 0.0);
    a["orientation"] = serde_json::json!(1.0);
    let mut b = cylinder_side_patch(1.0, 0.0, 1.0, 0.0);
    b["orientation"] = serde_json::json!(-1.0);
    let outcome = probe(serde_json::json!({
        "op": "rational_flux_probe",
        "kind": "fold",
        "patches": [a, b],
    }));
    assert_eq!(outcome["ok"], serde_json::json!(false), "{outcome}");
    assert_eq!(refusal_tag(&outcome), "anchor_mismatch");
}

// ---------------------------------------------------------------------------
// Test 10: the reciprocal tail shrinks with the order.
// ---------------------------------------------------------------------------

#[test]
fn tail_monotone_shrinks() {
    let coeffs = serde_json::json!([1.0, 3.0, 2.0, 1.5]);
    let first = probe(serde_json::json!({
        "op": "rational_flux_probe",
        "kind": "reciprocal",
        "coeffs": coeffs,
        "k": 3,
        "order": 1,
    }));
    let second = probe(serde_json::json!({
        "op": "rational_flux_probe",
        "kind": "reciprocal",
        "coeffs": coeffs,
        "k": 3,
        "order": 2,
    }));
    let error_first = field(&first, "error");
    let error_second = field(&second, "error");
    assert!(
        error_second < error_first,
        "order 2 ({error_second}) must strictly shrink order 1 ({error_first})"
    );
}

// ---------------------------------------------------------------------------
// Test 11: the subdivision-order law.
// ---------------------------------------------------------------------------

/// The exact de Casteljau split of a scalar Bernstein coefficient list.
fn split(coeffs: &[f64], t: f64) -> (Vec<f64>, Vec<f64>) {
    let n = coeffs.len();
    let mut work = coeffs.to_vec();
    let mut left = Vec::with_capacity(n);
    let mut right = vec![0.0f64; n];
    left.push(work[0]);
    right[n - 1] = work[n - 1];
    for k in 1..n {
        for i in 0..(n - k) {
            work[i] = (1.0 - t) * work[i] + t * work[i + 1];
        }
        left.push(work[0]);
        right[n - 1 - k] = work[n - 1 - k];
    }
    (left, right)
}

/// The coefficients of the `2^depth` uniform sub-cells of `coeffs`.
fn uniform_cells(coeffs: &[f64], depth: u32) -> Vec<(Vec<f64>, f64)> {
    let mut cells = vec![(coeffs.to_vec(), 1.0f64)];
    for _ in 0..depth {
        let mut next = Vec::with_capacity(cells.len() * 2);
        for (list, length) in cells {
            let (left, right) = split(&list, 0.5);
            next.push((left, 0.5 * length));
            next.push((right, 0.5 * length));
        }
        cells = next;
    }
    cells
}

/// The total certified integral width of the uniform subdivision.
fn subdivision_width(coeffs: &[f64], k: usize, order: usize, depth: u32) -> f64 {
    uniform_cells(coeffs, depth)
        .into_iter()
        .map(|(list, length)| {
            let outcome = probe(serde_json::json!({
                "op": "rational_flux_probe",
                "kind": "reciprocal",
                "coeffs": list,
                "k": k,
                "order": order,
            }));
            length * field(&outcome, "error")
        })
        .sum()
}

#[test]
fn subdivision_order() {
    let coeffs = [1.0, 2.0, 1.0];
    let order = 2usize;
    let k = 3usize;
    let w4 = subdivision_width(&coeffs, k, order, 2);
    let w16 = subdivision_width(&coeffs, k, order, 4);
    let slope = (w16 / w4).ln() / (16.0f64 / 4.0).ln();
    let expected = -((order + 1) as f64);
    assert!(
        (slope - expected).abs() < 0.6,
        "the subdivision slope {slope} must be near {expected} (widths {w4}, {w16})"
    );
}

// ---------------------------------------------------------------------------
// Test 9: the end-to-end canonical CYLINDER+CYLINDER union (native module).
// ---------------------------------------------------------------------------

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

/// One-time preparation of the native-module staging directory.
fn staged_native_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let exe = std::env::current_exe().expect("test binary path");
        let deps = exe.parent().expect("deps dir");
        let staging =
            std::env::temp_dir().join(format!("truck123d_rational_flux_{}", std::process::id()));
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

fn part_row(solid: serde_json::Value, x: f64, y: f64, z: f64) -> serde_json::Value {
    serde_json::json!({
        "part": {
            "solid": solid,
            "x": x,
            "y": y,
            "z": z,
            "rz": 0.0,
        }
    })
}

fn cylinder_solid(radius: f64, height: f64) -> serde_json::Value {
    serde_json::json!({
        "kind": "cylinder",
        "radius": radius,
        "height": height,
        "axis": "z",
    })
}

fn boolean_row(mode: &str, a: serde_json::Value, b: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "boolean": {
            "mode": mode,
            "a": a,
            "b": b,
        }
    })
}

#[test]
fn end_to_end_cylinder_union() {
    // Two disjoint canonical z-cylinders, r = 1, h = 2, centred at the origin
    // and at (4, 4, 4): the union volume is 2 * (pi * 1^2 * 2) = 4 pi. The two
    // bounding boxes are fully disjoint, so the EXTREMES-SURVIVE gate admits
    // the pair. The whole route -- node_box_patches (MONO-8 extraction) ->
    // rational flux -> MONO-5 membership -> the certified product -- must run
    // end to end.
    let a = part_row(cylinder_solid(1.0, 2.0), 0.0, 0.0, 0.0);
    let b = part_row(cylinder_solid(1.0, 2.0), 4.0, 4.0, 4.0);
    let tree = boolean_row("union", a, b).to_string();
    let script = r#"
import json, sys
import truck123d
try:
    facts = json.loads(truck123d.bd_facts(sys.argv[1]))
    print(json.dumps({"refused": False, "volume": facts.get("volume")}))
except truck123d.Refused as exc:
    payload = getattr(exc, "payload", {}) or {}
    print(json.dumps({"refused": True, "case": payload.get("case"),
                      "envelope": payload.get("envelope")}))
except Exception as exc:  # noqa: BLE001 - a wrong class is the boundary failure
    print(json.dumps({"refused": False, "class": type(exc).__name__}))
"#;
    let stdout = run_python(script, &[&tree]);
    let record: serde_json::Value = serde_json::from_str(stdout.trim()).expect("union record");
    assert_eq!(
        record["refused"],
        serde_json::json!(false),
        "the canonical cylinder union must pass the certified route, got {record}"
    );
    let volume = record["volume"].as_f64().expect("union volume");
    let expected = 4.0 * std::f64::consts::PI;
    assert!(
        (volume - expected).abs() < 1.0e-3,
        "the union volume {volume} must be near {expected}"
    );
}
