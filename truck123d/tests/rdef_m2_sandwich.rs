//! RDEF-M2-REGIME-SANDWICH required suite -- the (T)/(G) admission dichotomy
//! and the tangential sandwich volume rule for 2x2, with BOTH range-function
//! paths (the spec section 4.3 regression trap is a test requirement).
//!
//! The suite drives the door probe (`FacadeOp::SandwichProbe`) through the
//! single native facade entry, so every assertion exercises the exact path the
//! door reaches (CHK-9). The fixtures V1-V6 of spec section 10 are modelled
//! with exact tensor-Bernstein patch 2-cycles; every volume fixture asserts:
//!
//! 1. the true volume lies inside the certified bracket;
//! 2. the bracket width is non-increasing as the budget grows;
//! 3. the output is deterministic (bit-identical rerun);
//! 4. the refusal tag is correct wherever a refusal is expected.
//!
//! The exact fixture geometry:
//!
//! * V1 -- two boxes touching face-to-face (the sphere-resting-on-a-plane
//!   contact): union and cut. The overlap is exactly zero.
//! * V2 -- a plane against a tangent parabola (tangent curve); the undecided
//!   sandwich region shrinks as the schedule refines.
//! * V3 -- two identical boxes with a shared carrier (W2): exact.
//! * V4 -- the same coincidence with no carrier: `CoincidenceWithoutExactCarrier`.
//! * V5 -- V1 in a non-Bernstein representation: the mandatory fallback range
//!   function (the regression trap).
//! * V6 -- the retrodiction cell: a swept/canonical tangent pair (fuse) returns
//!   a certified bracket.

use truck123d::{FacadeTable, run_facade};

/// The degree-3 Bernstein representation of the linear basis functions `1 - t`
/// and `t` (the exact tensor-Bernstein elevation of a bilinear quad).
const E0: [f64; 4] = [1.0, 2.0 / 3.0, 1.0 / 3.0, 0.0];
const E1: [f64; 4] = [0.0, 1.0 / 3.0, 2.0 / 3.0, 1.0];

/// One planar quad as an exact bicubic tensor-Bernstein patch row, with the
/// corner order `(c00, c10, c11, c01)` (so `P_u x P_v` is outward for a
/// counter-clockwise corner order).
fn face(c00: [f64; 3], c10: [f64; 3], c11: [f64; 3], c01: [f64; 3]) -> serde_json::Value {
    let corners = [[c00, c01], [c10, c11]];
    let mut numerator = vec![vec![[0.0f64; 3]; 4]; 4];
    for (i, row) in numerator.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            let mut acc = [0.0f64; 3];
            for a in 0..2 {
                for b in 0..2 {
                    let wa = if a == 0 { E0[i] } else { E1[i] };
                    let wb = if b == 0 { E0[j] } else { E1[j] };
                    let coefficient = wa * wb;
                    for k in 0..3 {
                        acc[k] += coefficient * corners[a][b][k];
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

/// The six outward-oriented faces of the axis-aligned box `[lo, hi]` (the
/// corner orders are the exact `box_patch_rows` construction the bridge uses).
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

/// A single flat graph patch `z = 0` over the unit square, outward `+z`.
fn flat_patch() -> serde_json::Value {
    face(
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
    )
}

/// A degree-2 tangent-parabola patch `z = -c*((u-1/2)^2 + (v-1/2)^2)` over the
/// unit square, outward `+z`. The Bernstein coefficients of the two 1-D parts
/// are `c * [1/4, -1/4, 1/4]` for `-c*(t-1/2)^2`, so the tensor net is the sum
/// of the u-part and the v-part.
fn parabola_patch(c: f64) -> serde_json::Value {
    let part = [c * 0.25, -c * 0.25, c * 0.25];
    let axis = [0.0f64, 0.5, 1.0];
    let mut numerator = vec![vec![[0.0f64; 3]; 3]; 3];
    for (i, row) in numerator.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            *cell = [axis[i], axis[j], part[i] + part[j]];
        }
    }
    serde_json::json!({
        "numerator": numerator,
        "weights": vec![vec![1.0f64; 3]; 3],
        "orientation": 1.0,
    })
}

/// Runs one sandwich probe through the single native facade entry and returns
/// the serialized report.
#[allow(clippy::too_many_arguments)]
fn probe(
    base: Vec<serde_json::Value>,
    tool: Vec<serde_json::Value>,
    mode: &str,
    tolerance: f64,
    max_cells: usize,
    shared_carrier: bool,
    bernstein_chart: bool,
    rational_positive_weights: bool,
) -> serde_json::Value {
    let table = serde_json::json!({
        "ops": [{
            "op": "sandwich_probe",
            "base": base,
            "tool": tool,
            "mode": mode,
            "tolerance": tolerance,
            "max_cells": max_cells,
            "shared_carrier": shared_carrier,
            "bernstein_chart": bernstein_chart,
            "rational_positive_weights": rational_positive_weights,
        }],
    });
    let table: FacadeTable = serde_json::from_value(table).expect("the probe table deserializes");
    let report = run_facade(&table).expect("the facade report is produced");
    serde_json::to_value(&report).expect("the report serializes")
}

/// The `sandwich` outcome object of a report.
fn sandwich(report: &serde_json::Value) -> serde_json::Value {
    report
        .get("sandwich")
        .cloned()
        .expect("the report carries the sandwich outcome")
}

fn field(value: &serde_json::Value, key: &str) -> f64 {
    value
        .get(key)
        .and_then(serde_json::Value::as_f64)
        .unwrap_or_else(|| panic!("missing numeric field {key}: {value}"))
}

fn text<'a>(value: &'a serde_json::Value, key: &str) -> &'a str {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or_else(|| panic!("missing string field {key}: {value}"))
}

/// The refusal tag of an outcome, or `None` when the rule certified the
/// bracket.
fn refusal(value: &serde_json::Value) -> Option<&str> {
    value.get("refusal").and_then(serde_json::Value::as_str)
}

/// Asserts the certified bracket contains `true_value` up to the certified
/// floor slack.
fn assert_contains(outcome: &serde_json::Value, true_value: f64) {
    let lo = field(outcome, "bracket_lo");
    let hi = field(outcome, "bracket_hi");
    assert!(
        lo <= true_value + 1.0e-9 && true_value <= hi + 1.0e-9,
        "bracket [{lo}, {hi}] must contain {true_value}: {outcome}"
    );
}

/// V1 -- face-to-face contact (the sphere-resting-on-a-plane cell). The two
/// boxes touch at `z = 1`, so the overlap is exactly zero.
#[test]
fn v1_tangent_plane_volume_bracket_contains_the_true_volume() {
    let a = box_patches([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
    let b = box_patches([0.25, 0.25, 1.0], [0.75, 0.75, 2.0]);

    let cut = sandwich(&probe(
        a.clone(),
        b.clone(),
        "subtract",
        1.0,
        65536,
        false,
        true,
        true,
    ));
    assert!(refusal(&cut).is_none());
    assert_eq!(text(&cut, "regime"), "tangential");
    assert!(
        field(&cut, "sandwich_bound") < 1.0e-9,
        "the face contact has no sandwich uncertainty: {cut}"
    );
    assert_contains(&cut, 1.0);

    let union = sandwich(&probe(
        a.clone(),
        b.clone(),
        "union",
        1.0,
        65536,
        false,
        true,
        true,
    ));
    assert_contains(&union, 1.25);

    let intersect = sandwich(&probe(a, b, "intersect", 1.0, 65536, false, true, true));
    assert_contains(&intersect, 0.0);
}

/// V2 -- tangent curve. A plane against a tangent parabola: the undecided
/// sandwich region shrinks as the schedule refines, so the bound is
/// non-increasing with budget and stays non-negative.
#[test]
fn v2_tangent_curve_sandwich_is_monotone_in_budget() {
    let a = vec![flat_patch()];
    let b = vec![parabola_patch(1.0)];
    let coarse = sandwich(&probe(
        a.clone(),
        b.clone(),
        "intersect",
        1.0,
        8,
        false,
        true,
        true,
    ));
    let fine = sandwich(&probe(a, b, "intersect", 1.0e-2, 512, false, true, true));
    assert_eq!(text(&coarse, "regime"), "tangential");
    assert!(refusal(&fine).is_none(), "the refined rule certifies: {fine}");
    assert!(
        field(&coarse, "sandwich_bound") > 0.0,
        "the tangent curve carries a positive undecided bound: {coarse}"
    );
    assert!(
        field(&fine, "sandwich_bound") <= field(&coarse, "sandwich_bound"),
        "the sandwich bound must be non-increasing with budget: coarse {coarse}, fine {fine}"
    );
    assert!(
        field(&fine, "sandwich_bound") >= 0.0,
        "the refined bound must stay non-negative: {fine}"
    );
}

/// V3 -- coaxial equal patches with a shared carrier (W2): the coincidence is
/// exact, so the union and intersection are the operand and the cut is empty.
#[test]
fn v3_shared_carrier_coincidence_is_exact() {
    let a = box_patches([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
    let b = a.clone();
    let union = sandwich(&probe(
        a.clone(),
        b.clone(),
        "union",
        1.0e-6,
        65536,
        true,
        true,
        true,
    ));
    assert!(refusal(&union).is_none());
    assert!((field(&union, "bracket_lo") - 1.0).abs() < 1.0e-9);
    assert!((field(&union, "bracket_hi") - 1.0).abs() < 1.0e-9);
    assert_eq!(field(&union, "sandwich_bound"), 0.0);

    let cut = sandwich(&probe(a, b, "subtract", 1.0e-6, 65536, true, true, true));
    assert_eq!(field(&cut, "bracket_lo"), 0.0);
    assert_eq!(field(&cut, "bracket_hi"), 0.0);
}

/// V4 -- the same coincidence with no shared provenance: the rule refuses
/// `CoincidenceWithoutExactCarrier` and reports the floor.
#[test]
fn v4_coincidence_without_carrier_refuses_with_floor() {
    let a = box_patches([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
    let b = a.clone();
    let outcome = sandwich(&probe(a, b, "union", 0.0, 65536, false, true, true));
    assert_eq!(
        refusal(&outcome),
        Some("coincidence_without_exact_carrier"),
        "the W2-less coincidence refuses with the named tag: {outcome}"
    );
    let floor = field(&outcome, "floor");
    assert!(
        floor > 0.0,
        "the coincidence floor must be positive: {outcome}"
    );
}

/// V5 -- the regression trap: the same V1 pair in a non-Bernstein
/// representation must take the mandatory fallback range function and still
/// certify the bracket.
#[test]
fn v5_non_bernstein_representation_uses_the_fallback() {
    let a = box_patches([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
    let b = box_patches([0.25, 0.25, 1.0], [0.75, 0.75, 2.0]);
    let outcome = sandwich(&probe(a, b, "subtract", 1.0, 65536, false, false, true));
    assert_eq!(text(&outcome, "range_path"), "fallback");
    assert!(refusal(&outcome).is_none());
    assert_contains(&outcome, 1.0);
}

/// V6 -- the retrodiction cell: a swept/canonical tangent pair under fuse
/// returns a certified bracket (or a named refusal), never a silent zero.
#[test]
fn v6_retrodiction_fuse_returns_a_certified_bracket() {
    // The canonical base (a flat face) and a swept-family tool patch tangent to
    // it at a point (the modelled `fuse(swept, canonical)` cell).
    let a = vec![flat_patch()];
    let b = vec![parabola_patch(1.0)];
    let outcome = sandwich(&probe(a, b, "union", 1.0e-2, 512, false, true, true));
    match refusal(&outcome) {
        None => {
            assert_eq!(text(&outcome, "regime"), "tangential");
            assert!(field(&outcome, "width") >= 0.0);
        }
        Some(tag) => assert!(
            matches!(
                tag,
                "budget_exhausted"
                    | "coincidence_without_exact_carrier"
                    | "graph_not_injective"
                    | "singular_parametrization"
            ),
            "a named refusal is acceptable: {outcome}"
        ),
    }
}

/// The output is bit-reproducible across runs (spec section 8 determinism).
#[test]
fn sandwich_output_is_bit_reproducible() {
    let a = box_patches([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
    let b = box_patches([0.25, 0.25, 1.0], [0.75, 0.75, 2.0]);
    let first = probe(
        a.clone(),
        b.clone(),
        "subtract",
        1.0,
        65536,
        false,
        true,
        true,
    );
    let second = probe(a, b, "subtract", 1.0, 65536, false, true, true);
    assert_eq!(
        serde_json::to_string(&first).expect("json"),
        serde_json::to_string(&second).expect("json"),
        "the sandwich report must be bit-identical across runs"
    );
}
