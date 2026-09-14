//! FHC-G8-PLANARITY-ROUTER required suite -- the exact planar-face flux router.
//!
//! The router detects a planar patch by the coefficient test `nu . A_ij =
//! c w_ij` over the homogeneous control net (Thm 8.1, parametrization
//! independent) and certifies its divergence-form flux through the boundary
//! area-vector form `(1/3) c (nu . A_vec) / (nu . nu)` with `A_vec = (1/2)
//! oint X x dX`. A polynomial boundary integrates exactly; a rational boundary
//! routes to the 1D `k = 2` reciprocal-power certificate. A non-planar patch
//! falls through unchanged to the generic 2D certification.
//!
//! Tests:
//!
//! 1. `coefficient_plane_detector` -- planar polynomial and rational patches
//!    are detected exactly (the test plane is not axis-aligned); a genuinely
//!    non-planar rational patch is refused to the generic path.
//! 2. `planar_polynomial_exact` -- a planar polynomial face's boundary-form
//!    flux equals the generic exact value bit-for-bit.
//! 3. `planar_rational_k2_route` -- a rational planar quarter-disk fan routes
//!    to the 1D `k = 2` certificate and its enclosure contains the
//!    `pi`-dependent value (never a rational-valued exact claim).
//! 4. `fan_cap_through_cut` -- the suspension `_rocker` through-cut cap (a
//!    coaxial equal-profile plate subtraction) clears the degenerate-cap
//!    `SingularParametrization` refusal: the planar cone is certified exactly
//!    instead of refusing at the fan centroid.
//! 5. `non_planar_falls_through` -- a genuinely curved rational face reaches
//!    the same generic verdict as before the router.

use truck123d::{FacadeTable, run_facade};

// ---------------------------------------------------------------------------
// The door probe helpers.
// ---------------------------------------------------------------------------

/// Runs one facade op and returns the serialized report value.
fn report(row: serde_json::Value) -> serde_json::Value {
    let table = serde_json::json!({ "ops": [row] });
    let table: FacadeTable = serde_json::from_value(table).expect("the probe table deserializes");
    let report = run_facade(&table).expect("the facade report is produced");
    serde_json::to_value(&report).expect("the report serializes")
}

/// Runs one rational-flux probe row and returns the `rational_flux` outcome.
fn probe(row: serde_json::Value) -> serde_json::Value {
    report(row)
        .get("rational_flux")
        .cloned()
        .expect("the report carries the rational-flux outcome")
}

/// Runs one RDEF-M2 sandwich probe row and returns the `sandwich` outcome.
fn sandwich(row: serde_json::Value) -> serde_json::Value {
    report(row)
        .get("sandwich")
        .cloned()
        .expect("the report carries the sandwich outcome")
}

/// One `cell_flux` probe over the unit cell.
fn cell_flux(patches: Vec<serde_json::Value>) -> serde_json::Value {
    probe(serde_json::json!({
        "op": "rational_flux_probe",
        "kind": "cell_flux",
        "patches": patches,
        "cells": [[0.0, 1.0, 0.0, 1.0]],
    }))
}

/// The bracket of a single-patch `cell_flux` outcome.
fn bracket(value: &serde_json::Value) -> [f64; 2] {
    let list = value
        .get("brackets")
        .and_then(serde_json::Value::as_array)
        .expect("brackets");
    let pair = list[0].as_array().expect("bracket pair");
    [pair[0].as_f64().expect("lo"), pair[1].as_f64().expect("hi")]
}

fn refusal_tag(value: &serde_json::Value) -> String {
    value
        .get("refusal")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_else(|| panic!("expected a refusal tag: {value}"))
        .to_string()
}

/// One bilinear patch with the corner order `(p00, p10, p11, p01)` and the
/// row-major weight grid `[w00, w01, w10, w11]`.
fn bilinear(
    c00: [f64; 3],
    c10: [f64; 3],
    c11: [f64; 3],
    c01: [f64; 3],
    weights: [f64; 4],
) -> serde_json::Value {
    let corners = [[c00, c01], [c10, c11]];
    let mut numerator = vec![vec![[0.0f64; 3]; 2]; 2];
    let mut weight_grid = vec![vec![0.0f64; 2]; 2];
    for (i, row) in numerator.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            let w = weights[i * 2 + j];
            let corner = corners[i][j];
            *cell = [w * corner[0], w * corner[1], w * corner[2]];
            weight_grid[i][j] = w;
        }
    }
    serde_json::json!({
        "numerator": numerator,
        "weights": weight_grid,
        "orientation": 1.0,
    })
}

/// The rational quadratic quarter-circle arc fan of radius `r` in the plane
/// `z = 1`, fanned from the centre: a `3 x 2` rational tensor patch over
/// `(u, v)` with `u` the arc parameter and `v` the radial parameter. The image
/// is the planar quarter disk of area `pi r^2 / 4`.
fn quarter_disk_fan(r: f64) -> serde_json::Value {
    // The exact rational quadratic quarter arc: control points
    // `(r, 0), (r, r), (0, r)` with weights `1, sqrt(2)/2, 1`.
    let arc = [
        ([r, 0.0, 1.0], 1.0f64),
        ([r, r, 1.0], std::f64::consts::FRAC_1_SQRT_2),
        ([0.0, r, 1.0], 1.0f64),
    ];
    let centre = [0.0f64, 0.0, 1.0];
    let mut numerator = vec![vec![[0.0f64; 3]; 2]; 3];
    let mut weights = vec![vec![0.0f64; 2]; 3];
    for (i, (point, w)) in arc.iter().enumerate() {
        let w = *w;
        let arc_num = [w * point[0], w * point[1], w * point[2]];
        // v = 0: the centre (A = W * centre); v = 1: the arc control point.
        numerator[i][0] = [w * centre[0], w * centre[1], w * centre[2]];
        numerator[i][1] = arc_num;
        weights[i][0] = w;
        weights[i][1] = w;
    }
    serde_json::json!({
        "numerator": numerator,
        "weights": weights,
        "orientation": 1.0,
    })
}

// ---------------------------------------------------------------------------
// Test 1: the coefficient plane detector.
// ---------------------------------------------------------------------------

#[test]
fn coefficient_plane_detector() {
    // A planar parallelogram on the tilted plane `x + y + z = 1` (the plane is
    // not axis-aligned). Its flux magnitude is `(1/3) * area * distance`
    // = 1/3: the polynomial boundary integrates exactly.
    let p0 = [1.0, 0.0, 0.0];
    let a = [0.0, 1.0, -1.0];
    let b = [1.0, -1.0, 0.0];
    let add = |p: [f64; 3], q: [f64; 3]| [p[0] + q[0], p[1] + q[1], p[2] + q[2]];
    let c00 = p0;
    let c10 = add(p0, a);
    let c11 = add(add(p0, a), b);
    let c01 = add(p0, b);
    let polynomial = bilinear(c00, c10, c11, c01, [1.0; 4]);
    let outcome = cell_flux(vec![polynomial]);
    assert_eq!(outcome["ok"], serde_json::json!(true), "{outcome}");
    let [lo, hi] = bracket(&outcome);
    assert!(
        lo == hi && (lo.abs() - 1.0 / 3.0).abs() < 1.0e-12,
        "the tilted planar polynomial face must route to the exact 1/3, got [{lo}, {hi}]"
    );

    // The same parallelogram with a non-factorable positive weight net: the
    // coefficient test detects planarity and the 1D `k = 2` route certifies it.
    // The generic weight-factorization path would refuse this net.
    let rational = bilinear(c00, c10, c11, c01, [1.0, 2.0, 3.0, 4.0]);
    let outcome = cell_flux(vec![rational]);
    assert_eq!(outcome["ok"], serde_json::json!(true), "{outcome}");
    let [lo, hi] = bracket(&outcome);
    assert!(
        lo.is_finite() && hi.is_finite() && lo < hi,
        "the planar rational face must certify a genuine enclosure, got [{lo}, {hi}]"
    );

    // Perturb one control point off the plane: no plane fits, the router
    // refuses, and the generic path refuses the non-factorable weight net.
    let off = [c11[0], c11[1], c11[2] + 0.25];
    let perturbed = bilinear(c00, c10, off, c01, [1.0, 2.0, 3.0, 4.0]);
    let outcome = cell_flux(vec![perturbed]);
    assert_eq!(outcome["ok"], serde_json::json!(false), "{outcome}");
    assert_eq!(
        refusal_tag(&outcome),
        "non_regular_patch",
        "a near-planar non-planar patch must fall through to the generic path: {outcome}"
    );
}

// ---------------------------------------------------------------------------
// Test 2: the planar polynomial boundary form is exact.
// ---------------------------------------------------------------------------

#[test]
fn planar_polynomial_exact() {
    // A 3 x 1 rectangle in the plane z = 1: area 3, plane constant 1, so the
    // divergence-form flux is (1/3) * 1 * 3 = 1, exactly representable. The
    // boundary form and the generic exact 2D path both terminate at 1.0, and
    // the routed bracket is the bit-identical `[1, 1]`.
    let patch = bilinear(
        [0.0, 0.0, 1.0],
        [3.0, 0.0, 1.0],
        [3.0, 1.0, 1.0],
        [0.0, 1.0, 1.0],
        [1.0; 4],
    );
    let outcome = cell_flux(vec![patch]);
    assert_eq!(outcome["ok"], serde_json::json!(true), "{outcome}");
    let [lo, hi] = bracket(&outcome);
    assert_eq!(lo, 1.0, "the planar boundary form is exact: {outcome}");
    assert_eq!(hi, 1.0, "the planar boundary form is exact: {outcome}");
}

// ---------------------------------------------------------------------------
// Test 3: the rational planar boundary routes to the 1D k = 2 certificate.
// ---------------------------------------------------------------------------

#[test]
fn planar_rational_k2_route() {
    // The quarter-disk fan of radius 1 in the plane z = 1 has area pi/4, so
    // its divergence-form flux is pi/12. The enclosure must contain it and
    // must NOT be exact: a rational planar face is not forced through the
    // exact integer path.
    let patch = quarter_disk_fan(1.0);
    let outcome = cell_flux(vec![patch]);
    assert_eq!(outcome["ok"], serde_json::json!(true), "{outcome}");
    let [lo, hi] = bracket(&outcome);
    let expected = std::f64::consts::PI / 12.0;
    assert!(
        (lo <= expected && expected <= hi) || (lo <= -expected && -expected <= hi),
        "the quarter-disk enclosure [{lo}, {hi}] must contain pi/12 = {expected}"
    );
    assert!(
        lo < hi,
        "a rational planar face must carry a genuine enclosure, got [{lo}, {hi}]"
    );
    assert!(
        ((0.5 * (lo + hi)).abs() - expected).abs() < 1.0e-6,
        "the quarter-disk value must converge to pi/12, got [{lo}, {hi}]"
    );
}

// ---------------------------------------------------------------------------
// Test 5: a genuinely curved face falls through unchanged.
// ---------------------------------------------------------------------------

#[test]
fn non_planar_falls_through() {
    // The exact rational quadratic cylinder side patch (radius 1, height 1)
    // is genuinely curved; the router adds nothing and the generic certified
    // bracket is returned (a finite enclosure, not a planar point value).
    let mut numerator = vec![vec![[0.0f64; 3]; 2]; 3];
    let mut weights = vec![vec![0.0f64; 2]; 3];
    for i in 0..3 {
        let angle = (i as f64) * std::f64::consts::FRAC_PI_4;
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
        for (j, z) in [0.0, 1.0].into_iter().enumerate() {
            numerator[i][j] = [w * scale * angle.cos(), w * scale * angle.sin(), w * z];
            weights[i][j] = w;
        }
    }
    let patch = serde_json::json!({
        "numerator": numerator,
        "weights": weights,
        "orientation": 1.0,
    });
    let outcome = cell_flux(vec![patch]);
    assert_eq!(outcome["ok"], serde_json::json!(true), "{outcome}");
    let [lo, hi] = bracket(&outcome);
    assert!(
        lo.is_finite() && hi.is_finite() && lo <= hi,
        "the curved face must certify through the generic path, got [{lo}, {hi}]"
    );
    assert!(
        lo < hi,
        "a curved rational face is not a planar point value, got [{lo}, {hi}]"
    );
}

// ---------------------------------------------------------------------------
// Test 4: the `_rocker` through-cut fan cap clears the degenerate-cap refusal.
// ---------------------------------------------------------------------------

#[test]
fn fan_cap_through_cut() {
    // The suspension `_rocker` lightening cut is a coaxial equal-profile plate
    // through-cut. Its exact planar cap is a fan whose normal cone degenerates
    // at the loop centroid: the whole-box normal hull reaches zero there, so
    // the landed RDEF-M2 admission used to refuse typed
    // `SingularParametrization`. The router detects the planar cap first and
    // certifies its cone as the single plane normal (`s_up = 0`), so the
    // admission no longer refuses at the fan centroid.
    let base = quarter_disk_fan(1.0);
    let tool = quarter_disk_fan(0.5);
    let outcome = sandwich(serde_json::json!({
        "op": "sandwich_probe",
        "base": [base],
        "tool": [tool],
        "mode": "subtract",
        "tolerance": 1.0e-4,
        "max_cells": 65536,
        "shared_carrier": false,
        "bernstein_chart": true,
        "rational_positive_weights": true,
    }));
    let refusal = outcome
        .get("refusal")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    assert_ne!(
        refusal, "singular_parametrization",
        "the planar fan cap must not refuse the degenerate normal cone: {outcome}"
    );
    // The fan cap's planar flux must also certify: a refused planar flux would
    // surface as a sandwich `malformed_patch`.
    assert_ne!(
        refusal, "malformed_patch",
        "the planar fan cap flux must certify through the boundary form: {outcome}"
    );
}
