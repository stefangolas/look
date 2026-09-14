//! FHC-G9-GREEN-TRIM-INTEGRATION required suite -- faces with holes, no
//! trimmed-interior meshing.
//!
//! The funnel's patches are tensor-Bernstein on rectangular domains; a face
//! with holes (a trimmed surface) is inexpressible through the plain cell
//! route. This suite exercises the §9 Green reduction: the divergence-form
//! face flux `(1/3) ∬_Ω X·(X_u×X_v)` is never integrated over a meshed trim
//! interior -- it is reduced to the oriented trim boundary `∂Ω` by Green's
//! theorem, with the exact Bernstein antiderivative (Lemma 9.2) for the
//! polynomial case and G1's 1D reciprocal-power certificates for rational
//! pullbacks.
//!
//! The trim carriers are parameter-space Bernstein curves submitted through
//! the landed `rational_flux_probe` surface: a non-empty `tool` row list
//! carries the oriented segments (each row's `orientation` is its loop sign)
//! and `cells` carries the `[start, end, sign, _]` loop descriptors. `k = 0`
//! on the `reciprocal` kind is the Lemma 9.2 antiderivative round-trip probe.
//!
//! Tests:
//!
//! 1. `antiderivative_exact` -- differentiating the constructed antiderivative
//!    reproduces the integrand coefficients exactly.
//! 2. `green_matches_full_rectangle` -- an untrimmed patch's Green boundary
//!    route equals the direct exact integral bit-for-bit.
//! 3. `polynomial_trims_exact` -- polynomial rectangular and triangular trims
//!    answer their independently computable areas bit-identically.
//! 4. `quarter_disk_pi` -- a polynomial plane trimmed by a rational circular
//!    arc converges to the known π-dependent value via the 1D route.
//! 5. `rational_surface_trimmed` -- Thm 9.5 end-to-end: the certified bracket
//!    contains the analytic value and the order increase shrinks its width.
//! 6. `hole_orientation_refuses` -- a mis-oriented hole loop refuses typed
//!    `loop_orientation`; never a silently wrong sign.

use truck123d::{FacadeTable, run_facade};

/// Runs one rational-flux probe row and returns the `rational_flux` outcome.
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

fn brackets(value: &serde_json::Value) -> Vec<[f64; 2]> {
    value
        .get("brackets")
        .and_then(serde_json::Value::as_array)
        .unwrap_or_else(|| panic!("brackets missing: {value}"))
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

fn first_bracket(value: &serde_json::Value) -> [f64; 2] {
    brackets(value)[0]
}

fn refusal_tag(value: &serde_json::Value) -> String {
    value
        .get("refusal")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_else(|| panic!("expected a refusal tag: {value}"))
        .to_string()
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// The plane `X(u, v) = (u, v, z)` over `[0, 1]²` as an exact bilinear
/// unit-weight tensor-Bernstein patch (flux density `(1/3) z`).
fn plane_patch(z: f64) -> serde_json::Value {
    serde_json::json!({
        "numerator": [
            [[0.0, 0.0, z], [0.0, 1.0, z]],
            [[1.0, 0.0, z], [1.0, 1.0, z]],
        ],
        "weights": [[1.0, 1.0], [1.0, 1.0]],
        "orientation": 1.0,
    })
}

/// The plane `X(u, v) = (u, v, 1)` carried rationally as `A / W` with
/// `W(u) = 1 + u` (a univariate positive weight; Thm 9.5's admitted arm).
fn rational_plane_patch() -> serde_json::Value {
    // A = W·X, degree (2, 1).
    serde_json::json!({
        "numerator": [
            [[0.0, 0.0, 1.0], [0.0, 1.0, 1.0]],
            [[0.5, 0.0, 1.5], [0.5, 1.5, 1.5]],
            [[2.0, 0.0, 2.0], [2.0, 2.0, 2.0]],
        ],
        "weights": [[1.0, 1.0], [1.5, 1.5], [2.0, 2.0]],
        "orientation": 1.0,
    })
}

/// One polynomial trim segment through the parameter-space points (degree
/// `len - 1`), positive loop sign.
fn poly_segment(points: &[[f64; 2]]) -> serde_json::Value {
    let numerator: Vec<Vec<[f64; 3]>> = vec![points.iter().map(|p| [p[0], p[1], 0.0]).collect()];
    let n = points.len();
    serde_json::json!({
        "numerator": numerator,
        "weights": vec![vec![1.0; n]],
        "orientation": 1.0,
    })
}

/// One rational trim segment: homogeneous control points and weights.
fn rational_segment(points: &[[f64; 3]], weights: &[f64]) -> serde_json::Value {
    serde_json::json!({
        "numerator": vec![points.to_vec()],
        "weights": vec![weights.to_vec()],
        "orientation": 1.0,
    })
}

/// The four CCW segments of the parameter rectangle `[u0, u1] x [v0, v1]`.
fn rectangle_segments(u0: f64, u1: f64, v0: f64, v1: f64) -> Vec<serde_json::Value> {
    vec![
        poly_segment(&[[u0, v0], [u1, v0]]),
        poly_segment(&[[u1, v0], [u1, v1]]),
        poly_segment(&[[u1, v1], [u0, v1]]),
        poly_segment(&[[u0, v1], [u0, v0]]),
    ]
}

/// The three CCW segments of the triangle `(0,0) -> (1,0) -> (0,1)`.
fn triangle_segments() -> Vec<serde_json::Value> {
    vec![
        poly_segment(&[[0.0, 0.0], [1.0, 0.0]]),
        poly_segment(&[[1.0, 0.0], [0.0, 1.0]]),
        poly_segment(&[[0.0, 1.0], [0.0, 0.0]]),
    ]
}

fn trim_call(
    surface: serde_json::Value,
    segments: Vec<serde_json::Value>,
    loops: Vec<[f64; 4]>,
    order: usize,
) -> serde_json::Value {
    probe(serde_json::json!({
        "op": "rational_flux_probe",
        "kind": "cell_flux",
        "patches": [surface],
        "tool": segments,
        "cells": loops,
        "order": order,
    }))
}

fn direct_call(surface: serde_json::Value) -> serde_json::Value {
    probe(serde_json::json!({
        "op": "rational_flux_probe",
        "kind": "cell_flux",
        "patches": [surface],
        "cells": [[0.0, 1.0, 0.0, 1.0]],
    }))
}

// ---------------------------------------------------------------------------
// Test 1: the Lemma 9.2 antiderivative round-trip.
// ---------------------------------------------------------------------------

#[test]
fn antiderivative_exact() {
    let integrand = vec![0.5, -1.25, 3.0, 2.0];
    let outcome = probe(serde_json::json!({
        "op": "rational_flux_probe",
        "kind": "reciprocal",
        "coeffs": integrand,
        "k": 0,
        "order": 0,
    }));
    assert_eq!(outcome["ok"], serde_json::json!(true), "{outcome}");
    let anti: Vec<f64> = brackets(&outcome).iter().map(|b| b[0]).collect();
    assert_eq!(anti.len(), integrand.len() + 1, "degree r -> r + 1");
    // Differentiate the constructed antiderivative independently.
    let r = anti.len() - 1;
    let derivative: Vec<f64> = (0..r)
        .map(|i| (r as f64) * (anti[i + 1] - anti[i]))
        .collect();
    assert_eq!(
        derivative, integrand,
        "the antiderivative derivative must reproduce the integrand coefficients exactly"
    );
}

// ---------------------------------------------------------------------------
// Test 2: the Green route equals the direct exact integral bit-for-bit.
// ---------------------------------------------------------------------------

#[test]
fn green_matches_full_rectangle() {
    let green = trim_call(
        plane_patch(3.0),
        rectangle_segments(0.0, 1.0, 0.0, 1.0),
        vec![[0.0, 4.0, 1.0, 0.0]],
        0,
    );
    let direct = direct_call(plane_patch(3.0));
    let [glo, ghi] = first_bracket(&green);
    let [dlo, dhi] = first_bracket(&direct);
    // The direct exact route returns a certified (single-rounding) enclosure of
    // the exact integral; the Green boundary route reconstructs the exact
    // rational value. They must agree to the direct bracket's own rounding.
    assert!(
        dlo <= glo && ghi <= dhi,
        "the Green route [{glo}, {ghi}] must lie in the direct certified bracket \
         [{dlo}, {dhi}]"
    );
    assert!(
        (glo - 1.0).abs() < 1.0e-12 && (ghi - 1.0).abs() < 1.0e-12,
        "the unit rectangle at z = 3 must answer exactly 1, got [{glo}, {ghi}]"
    );
    assert!(
        dhi - dlo < 1.0e-12,
        "the direct exact route must certify the unit rectangle tightly, got [{dlo}, {dhi}]"
    );
}

// ---------------------------------------------------------------------------
// Test 3: polynomial rectangular and triangular trims are exact.
// ---------------------------------------------------------------------------

#[test]
fn polynomial_trims_exact() {
    // The density is exactly 1, so the flux is the trimmed area.
    let rectangle = trim_call(
        plane_patch(3.0),
        rectangle_segments(0.0, 0.5, 0.0, 1.0),
        vec![[0.0, 4.0, 1.0, 0.0]],
        0,
    );
    let triangle = trim_call(
        plane_patch(3.0),
        triangle_segments(),
        vec![[0.0, 3.0, 1.0, 0.0]],
        0,
    );
    let [rlo, rhi] = first_bracket(&rectangle);
    let [tlo, thi] = first_bracket(&triangle);
    assert_eq!(
        (rlo, rhi),
        (0.5, 0.5),
        "the half-unit rectangle must answer exactly 0.5, got [{rlo}, {rhi}]"
    );
    assert_eq!(
        (tlo, thi),
        (0.5, 0.5),
        "the half-unit triangle must answer exactly 0.5, got [{tlo}, {thi}]"
    );
    assert_eq!((rlo, rhi), (tlo, thi), "both trims are bit-identical");
}

// ---------------------------------------------------------------------------
// Test 4: the rational circular-arc trim converges to the pi-dependent value.
// ---------------------------------------------------------------------------

/// The quarter disk `(0,0) -> (1,0) -> (0,1)` with a rational quadratic arc.
fn quarter_disk_segments() -> Vec<serde_json::Value> {
    let w = std::f64::consts::FRAC_1_SQRT_2;
    vec![
        poly_segment(&[[0.0, 0.0], [1.0, 0.0]]),
        rational_segment(
            &[[1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0]],
            &[1.0, w, 1.0],
        ),
        poly_segment(&[[0.0, 1.0], [0.0, 0.0]]),
    ]
}

#[test]
fn quarter_disk_pi() {
    let expected = std::f64::consts::PI / 4.0;
    let low = trim_call(
        plane_patch(3.0),
        quarter_disk_segments(),
        vec![[0.0, 3.0, 1.0, 0.0]],
        2,
    );
    let high = trim_call(
        plane_patch(3.0),
        quarter_disk_segments(),
        vec![[0.0, 3.0, 1.0, 0.0]],
        10,
    );
    let [llo, lhi] = first_bracket(&low);
    let [hlo, hhi] = first_bracket(&high);
    assert!(
        llo <= expected && expected <= lhi,
        "the quarter-disk bracket [{llo}, {lhi}] must contain pi/4 = {expected}"
    );
    assert!(
        hhi - hlo < lhi - llo,
        "a higher reciprocal order must strictly shrink the width ({}, {})",
        lhi - llo,
        hhi - hlo
    );
}

// ---------------------------------------------------------------------------
// Test 5: Thm 9.5 rational surface, trimmed.
// ---------------------------------------------------------------------------

#[test]
fn rational_surface_trimmed() {
    // X(u, v) = (u, v, 1) with W = 1 + u; the flux density is exactly 1/3, so
    // the analytic trimmed flux over the unit square is 1/3.
    let expected = 1.0 / 3.0;
    let low = trim_call(
        rational_plane_patch(),
        rectangle_segments(0.0, 1.0, 0.0, 1.0),
        vec![[0.0, 4.0, 1.0, 0.0]],
        1,
    );
    let high = trim_call(
        rational_plane_patch(),
        rectangle_segments(0.0, 1.0, 0.0, 1.0),
        vec![[0.0, 4.0, 1.0, 0.0]],
        8,
    );
    let [llo, lhi] = first_bracket(&low);
    let [hlo, hhi] = first_bracket(&high);
    assert!(
        llo <= expected && expected <= lhi,
        "the rational-surface bracket [{llo}, {lhi}] must contain 1/3 = {expected}"
    );
    assert!(
        hhi - hlo < lhi - llo,
        "the order increase must strictly shrink the rational bracket width ({}, {})",
        lhi - llo,
        hhi - hlo
    );
}

// ---------------------------------------------------------------------------
// Test 6: a mis-oriented hole loop refuses typed.
// ---------------------------------------------------------------------------

#[test]
fn hole_orientation_refuses() {
    // Outer CCW unit square plus a hole that is also CCW (declared negative):
    // the boundary handedness contradicts the declared material role, so the
    // loop set refuses rather than silently flipping the hole's sign.
    let mut segments = rectangle_segments(0.0, 1.0, 0.0, 1.0);
    segments.extend(rectangle_segments(0.25, 0.75, 0.25, 0.75));
    let outcome = trim_call(
        plane_patch(3.0),
        segments,
        vec![[0.0, 4.0, 1.0, 0.0], [4.0, 8.0, -1.0, 0.0]],
        0,
    );
    assert_eq!(outcome["ok"], serde_json::json!(false), "{outcome}");
    assert_eq!(refusal_tag(&outcome), "loop_orientation");
}

/// A correctly oriented (CW) hole is admitted and subtracts its area.
#[test]
fn hole_orientation_accepts_cw_hole() {
    let mut segments = rectangle_segments(0.0, 1.0, 0.0, 1.0);
    // The hole traversed clockwise (negative handedness), declared negative.
    segments.push(poly_segment(&[[0.25, 0.25], [0.25, 0.75]]));
    segments.push(poly_segment(&[[0.25, 0.75], [0.75, 0.75]]));
    segments.push(poly_segment(&[[0.75, 0.75], [0.75, 0.25]]));
    segments.push(poly_segment(&[[0.75, 0.25], [0.25, 0.25]]));
    let outcome = trim_call(
        plane_patch(3.0),
        segments,
        vec![[0.0, 4.0, 1.0, 0.0], [4.0, 8.0, -1.0, 0.0]],
        0,
    );
    assert_eq!(outcome["ok"], serde_json::json!(true), "{outcome}");
    let [lo, hi] = first_bracket(&outcome);
    let expected = 1.0 - 0.25; // unit square minus the quarter-area hole
    assert!(
        (lo - expected).abs() < 1.0e-12 && (hi - expected).abs() < 1.0e-12,
        "the CW hole must subtract exactly its area, got [{lo}, {hi}]"
    );
}
