#![cfg_attr(not(debug_assertions), deny(warnings))]
#![deny(clippy::all, rust_2018_idioms)]
#![deny(clippy::unwrap_used)]
#![warn(
    missing_docs,
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]

//! The §13 R6 self-intersection residual: deflation, exact cover, and chart
//! transitions (BG-KV2-404-S8).
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module. The module carries no `unwrap`, no `expect`, and no `panic!`
//! calls, and adds no module-level `allow`.
//!
//! **What R6 is.** A self-intersection of one surface leaf is a pair of
//! parameter points `p` and `p + δ` with `S(p) = S(p + δ)`. The naive
//! difference residual over the offset `δ` is identically degenerate on the
//! trivial diagonal `δ = 0`, so §13 deflates it: the offset is split along the
//! two parameter axes and each axis quotient becomes an exact divided
//! difference of the Bézier net,
//!
//! ```text
//! D1(u, v; h, k) = [S(u + h, v + k) − S(u, v + k)] / h,
//! D2(u, v; h, k) = [S(u, v + k) − S(u, v)] / k,
//! S(u + h, v + k) − S(u, v) = h·D1 + k·D2.
//! ```
//!
//! Both divided differences are POLYNOMIAL on the Bézier net: each is realized
//! here as Bernstein coefficient arithmetic (shift the net, subtract, divide by
//! the scalar step, reduce the degree by one), never as a sampled finite
//! difference. Dyadic steps make the scalar divisions exact in binary floating
//! point, so the machine check reproduces the polynomial identity to the
//! rounding floor.
//!
//! For a rational leaf `S = P/w` the deflated QUOTIENT divided differences are
//! not polynomial; the residual is instead carried in the §7.1 NUMERATOR form
//! `P(p + δ)·w(p) − P(p)·w(p + δ)` (cross-multiplied, exactly the R8/R9
//! discipline of the S1A module: no weight-bearing division anywhere, the
//! positive weight bound is a VALUE argument, never a denominator).
//!
//! **Exact cover (Theorem 13.1).** The dominant step chooses the chart:
//! `|h| ≥ |k|` is chart A (offset `(λ, λ·m)`, `λ > 0`, `|m| ≤ 1`) and
//! `|k| > |h|` is chart B (offset `(λ·m, λ)`, `λ > 0`, `|m| < 1`). Because a
//! self-pair is an UNORDERED pair, `δ` and `−δ` describe the same witness; the
//! canonical representative (dominant step positive) makes exactly one of the
//! antipodal pair admissible, so counting witnesses never double-counts.
//! [`r6_witness`] implements the cover on the canonical representative.
//!
//! **Transitions (Theorem 13.3 / Corollary 13.2).** A chart-A step that reaches
//! `m = +1` re-parameterizes into chart B (Type I: `m_B = 1/m_A`,
//! `λ_B = λ_A·m_A`) on the same base and emits [`SegmentBreak::R6ChartSwitch`];
//! a step that reaches `m = −1` swaps the base point to the far member of the
//! unordered pair (Type II) and emits [`SegmentBreak::R6BaseSwap`]. Both
//! preserve the witness: the two sides carry the same unordered pair, so the
//! surface point of the witness is identical across the break.
//!
//! **λ = 0 stratum (Theorem 13.4).** At `λ = 0` chart A's residual reduces to
//! `S_u + m·S_v`, which is solvable only at a parametric degeneracy. The module
//! therefore routes a `λ = 0` datum back into the chart machinery (regular
//! leaf: no zero exists, the trace continues away from the diagonal) or to the
//! carrier/leaf level (parametric degeneracy detected) — and NEVER to the §10.3
//! isolated-contact classifier. A caller that insists on the §3.4 route is
//! refused by the typed refusal [`r6_lambda_zero_refusal`], which names the
//! route in its predicate.
//!
//! **N4.** This module performs no transcendental call — no `sin`, `cos`,
//! `atan2`, `exp`, `ln`, `log`, `powf`, and no `sqrt` anywhere. Degeneracy and
//! norm decisions are made on squared quantities.

use crate::hull::bernstein_derivative_2d;
use crate::kernel::config::EPS_REP;
use crate::kernel::evidence::{Construction, Refusal, RefusalEvidence, RefusalKind};
use crate::kernel::graph::SegmentBreak;
use crate::kernel::leaf::BezierLeaf;

/// A named predicate refusal: the kind names the violated class, the predicate
/// name the precise invariant.
fn refusal(kind: RefusalKind, name: &'static str, detail: String) -> Refusal {
    Refusal::new(kind, RefusalEvidence::Predicate { name, detail })
}

/// De Casteljau evaluation of the Bernstein polynomial with coefficients
/// `coeffs` (degree `coeffs.len() - 1`) at the scalar `t`. `None` for an empty
/// list or a non-finite input. Pure `f64` arithmetic; the polynomials the nets
/// define are ordinary real polynomials on the whole line.
fn eval_1d(coeffs: &[f64], t: f64) -> Option<f64> {
    if coeffs.is_empty() || !t.is_finite() || coeffs.iter().any(|c| !c.is_finite()) {
        return None;
    }
    let mut level = coeffs.to_vec();
    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len() - 1);
        for w in level.windows(2) {
            next.push(w[0] + t * (w[1] - w[0]));
        }
        level = next;
    }
    Some(level[0])
}

/// Validate a tensor coefficient grid: non-empty, rectangular, finite.
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
fn check_grid(grid: &[Vec<f64>]) -> Result<(), Refusal> {
    if grid.is_empty() || grid[0].is_empty() {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "r6_empty_grid",
            "a divided-difference net needs a non-empty coefficient grid".to_string(),
        ));
    }
    let width = grid[0].len();
    for (i, row) in grid.iter().enumerate() {
        if row.len() != width {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "r6_ragged_grid",
                format!(
                    "grid row {i} has {} coefficients, expected {width}",
                    row.len()
                ),
            ));
        }
        for (j, c) in row.iter().enumerate() {
            if !c.is_finite() {
                return Err(refusal(
                    RefusalKind::NonFinite,
                    "r6_grid_coefficient_not_finite",
                    format!("grid coefficient ({i}, {j}) is not finite"),
                ));
            }
        }
    }
    Ok(())
}

/// De Casteljau evaluation of the tensor polynomial
/// `Σ_{i,j} grid[i][j]·B_i(u)·B_j(v)` (rows over `u`, columns over `v`) at the
/// point `(u, v)`. `None` for a malformed grid or non-finite inputs.
pub fn bernstein_eval_2d(grid: &[Vec<f64>], u: f64, v: f64) -> Option<f64> {
    check_grid(grid).ok()?;
    if !u.is_finite() || !v.is_finite() {
        return None;
    }
    let mut rows = Vec::with_capacity(grid.len());
    for row in grid {
        rows.push(eval_1d(row, v)?);
    }
    eval_1d(&rows, u)
}

/// The scalar blossom (polar form) of the degree-`coeffs.len() - 1` Bernstein
/// polynomial with coefficients `coeffs`, evaluated at the `degree` affine
/// arguments `xs` (multiaffine, symmetric). `None` when the argument count
/// does not match the degree or any input is non-finite.
fn blossom(coeffs: &[f64], xs: &[f64]) -> Option<f64> {
    let degree = coeffs.len().checked_sub(1)?;
    if xs.len() != degree {
        return None;
    }
    if coeffs.iter().any(|c| !c.is_finite()) || xs.iter().any(|x| !x.is_finite()) {
        return None;
    }
    let mut level: Vec<f64> = coeffs.to_vec();
    for &x in xs {
        let mut next = Vec::with_capacity(level.len() - 1);
        for w in level.windows(2) {
            next.push((1.0 - x) * w[0] + x * w[1]);
        }
        level = next;
    }
    level.first().copied()
}

/// Bernstein coefficients of the shifted polynomial `p(t + step)` in the SAME
/// degree-`n` basis over `[0, 1]`. The `i`-th coefficient is the blossom of
/// `p` at `(n − i)` copies of `step` and `i` copies of `1 + step`. A degree-0
/// coefficient list (a constant) shifts to itself.
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
pub fn shift_1d(coeffs: &[f64], step: f64) -> Construction<Vec<f64>> {
    if coeffs.is_empty() {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "r6_empty_coeffs",
            "a shift needs a non-empty coefficient list".to_string(),
        ));
    }
    if !step.is_finite() {
        return Err(refusal(
            RefusalKind::NonFinite,
            "r6_step_not_finite",
            format!("shift step {step} is not finite"),
        ));
    }
    if coeffs.iter().any(|c| !c.is_finite()) {
        return Err(refusal(
            RefusalKind::NonFinite,
            "r6_coefficient_not_finite",
            "a shift coefficient is not finite".to_string(),
        ));
    }
    let degree = coeffs.len() - 1;
    if degree == 0 {
        return Ok(coeffs.to_vec());
    }
    let step_hi = step + 1.0;
    let mut out = Vec::with_capacity(degree + 1);
    for i in 0..=degree {
        let mut xs = Vec::with_capacity(degree);
        xs.extend(std::iter::repeat_n(step, degree - i));
        xs.extend(std::iter::repeat_n(step_hi, i));
        match blossom(coeffs, &xs) {
            Some(q) => out.push(q),
            None => {
                return Err(refusal(
                    RefusalKind::ClaimRefuted,
                    "r6_shift_blossom_arity_mismatch",
                    "the shift blossom argument list does not match the degree".to_string(),
                ))
            }
        }
    }
    Ok(out)
}

/// Bernstein coefficients of the degree-`n − 1` polynomial
/// `(p(t + step) − p(t)) / step` from the degree-`n` coefficients `coeffs`.
///
/// The construction is Bernstein coefficient arithmetic only: re-express
/// `p(t + step)` in the same basis (a shift), subtract the input net, divide
/// the coefficient difference by the scalar `step` (exact for dyadic steps),
/// and degree-reduce by one through the exact elevation inverse. The result is
/// a polynomial — this is what makes the deflated residual a polynomial on the
/// Bézier net rather than a sampled finite difference.
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
pub fn divided_difference_1d(coeffs: &[f64], step: f64) -> Construction<Vec<f64>> {
    if coeffs.len() <= 1 {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "r6_divided_difference_constant",
            "a divided difference needs a positive degree coefficient list".to_string(),
        ));
    }
    if !step.is_finite() {
        return Err(refusal(
            RefusalKind::NonFinite,
            "r6_step_not_finite",
            format!("divided-difference step {step} is not finite"),
        ));
    }
    if step == 0.0 {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "r6_zero_step",
            "a zero divided-difference step is degenerate".to_string(),
        ));
    }
    if coeffs.iter().any(|c| !c.is_finite()) {
        return Err(refusal(
            RefusalKind::NonFinite,
            "r6_coefficient_not_finite",
            "a divided-difference coefficient is not finite".to_string(),
        ));
    }
    let shifted = shift_1d(coeffs, step)?;
    let degree = coeffs.len() - 1;
    let g: Vec<f64> = shifted
        .iter()
        .zip(coeffs.iter())
        .map(|(q, p)| (q - p) / step)
        .collect();
    // Degree-reduce the degree-n coefficient list g (which represents a
    // polynomial of degree at most n − 1) to degree n − 1 by inverting the
    // Bernstein elevation recurrence g_i = (i/n)·d_{i-1} + ((n - i)/n)·d_i.
    let n = degree as f64;
    let mut d = Vec::with_capacity(degree);
    let mut d_prev = 0.0;
    for (i, &gi) in g.iter().take(degree).enumerate() {
        let di = (n * gi - i as f64 * d_prev) / (n - i as f64);
        d.push(di);
        d_prev = di;
    }
    Ok(d)
}

/// Shift a tensor net along its `u` axis by `step`: the coefficients of
/// `S(u + step, v)` in the same basis (rows over `u` unchanged in count).
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
pub fn shift_u(grid: &[Vec<f64>], step: f64) -> Construction<Vec<Vec<f64>>> {
    check_grid(grid)?;
    if !step.is_finite() {
        return Err(refusal(
            RefusalKind::NonFinite,
            "r6_step_not_finite",
            format!("shift step {step} is not finite"),
        ));
    }
    let width = grid[0].len();
    let mut out = Vec::with_capacity(grid.len());
    for _ in grid {
        out.push(vec![0.0; width]);
    }
    for j in 0..width {
        let col: Vec<f64> = grid.iter().map(|row| row[j]).collect();
        let shifted = shift_1d(&col, step)?;
        for (i, value) in shifted.iter().enumerate() {
            out[i][j] = *value;
        }
    }
    Ok(out)
}

/// Shift a tensor net along its `v` axis by `step`: the coefficients of
/// `S(u, v + step)` in the same basis (columns over `v` unchanged in count).
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
pub fn shift_v(grid: &[Vec<f64>], step: f64) -> Construction<Vec<Vec<f64>>> {
    check_grid(grid)?;
    if !step.is_finite() {
        return Err(refusal(
            RefusalKind::NonFinite,
            "r6_step_not_finite",
            format!("shift step {step} is not finite"),
        ));
    }
    let mut out = Vec::with_capacity(grid.len());
    for row in grid {
        out.push(shift_1d(row, step)?);
    }
    Ok(out)
}

/// The coefficient net of the divided difference of a tensor net along its `u`
/// axis with step `step`: rows drop by one (the `u` degree lowers by one),
/// columns are unchanged.
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
pub fn divided_difference_u(grid: &[Vec<f64>], step: f64) -> Construction<Vec<Vec<f64>>> {
    check_grid(grid)?;
    if !step.is_finite() {
        return Err(refusal(
            RefusalKind::NonFinite,
            "r6_step_not_finite",
            format!("divided-difference step {step} is not finite"),
        ));
    }
    if step == 0.0 {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "r6_zero_step",
            "a zero divided-difference step is degenerate".to_string(),
        ));
    }
    if grid.len() <= 1 {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "r6_divided_difference_constant",
            "a u-axis divided difference needs a positive u degree".to_string(),
        ));
    }
    let width = grid[0].len();
    let rows_out = grid.len() - 1;
    let mut out = Vec::with_capacity(rows_out);
    for _ in 0..rows_out {
        out.push(vec![0.0; width]);
    }
    for j in 0..width {
        let col: Vec<f64> = grid.iter().map(|row| row[j]).collect();
        let dd = divided_difference_1d(&col, step)?;
        for (i, value) in dd.iter().enumerate() {
            out[i][j] = *value;
        }
    }
    Ok(out)
}

/// The coefficient net of the divided difference of a tensor net along its `v`
/// axis with step `step`: columns drop by one, rows are unchanged.
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
pub fn divided_difference_v(grid: &[Vec<f64>], step: f64) -> Construction<Vec<Vec<f64>>> {
    check_grid(grid)?;
    if !step.is_finite() {
        return Err(refusal(
            RefusalKind::NonFinite,
            "r6_step_not_finite",
            format!("divided-difference step {step} is not finite"),
        ));
    }
    if step == 0.0 {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "r6_zero_step",
            "a zero divided-difference step is degenerate".to_string(),
        ));
    }
    if grid[0].len() <= 1 {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "r6_divided_difference_constant",
            "a v-axis divided difference needs a positive v degree".to_string(),
        ));
    }
    let mut out = Vec::with_capacity(grid.len());
    for row in grid {
        out.push(divided_difference_1d(row, step)?);
    }
    Ok(out)
}

/// Validate a surface leaf for admission to the R6 residual: re-runs the
/// landed `BezierLeaf::try_new` structural checks so a raw leaf cannot reach
/// the residual.
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
fn validate_leaf(leaf: &BezierLeaf) -> Result<(), Refusal> {
    if leaf.degree_u == 0 || leaf.degree_v == 0 {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "bezier_zero_degree",
            format!(
                "leaf degrees ({}, {}) must be positive",
                leaf.degree_u, leaf.degree_v
            ),
        ));
    }
    let expected = (leaf.degree_u + 1) * (leaf.degree_v + 1);
    if leaf.control.len() != expected {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "bezier_control_count_mismatch",
            format!(
                "control net has {} points, degrees ({}, {}) require {expected}",
                leaf.control.len(),
                leaf.degree_u,
                leaf.degree_v
            ),
        ));
    }
    for (i, p) in leaf.control.iter().enumerate() {
        for c in p {
            if !c.is_finite() {
                return Err(refusal(
                    RefusalKind::NonFinite,
                    "bezier_coordinate_not_finite",
                    format!("control point {i} has a non-finite coordinate: {p:?}"),
                ));
            }
        }
        if p[3] <= 0.0 {
            return Err(refusal(
                RefusalKind::WeightDegenerate,
                "bezier_control_weight_not_positive",
                format!("control point {i} has weight {} which is not > 0", p[3]),
            ));
        }
    }
    Ok(())
}

/// The `comp`-coordinate coefficient grid of a surface leaf, rows over `u` and
/// columns over `v` (the layout the divided-difference nets consume). The leaf
/// is assumed already validated.
fn leaf_grid(leaf: &BezierLeaf, comp: usize) -> Vec<Vec<f64>> {
    let width = leaf.degree_v + 1;
    (0..=leaf.degree_u)
        .map(|i| {
            (0..=leaf.degree_v)
                .map(|j| leaf.control[i * width + j][comp])
                .collect()
        })
        .collect()
}

/// Validate a `(base, offset)` pair of parameter points as finite scalars.
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
fn check_uv(base: [f64; 2], offset: [f64; 2]) -> Result<(), Refusal> {
    for (i, value) in base.iter().enumerate() {
        if !value.is_finite() {
            return Err(refusal(
                RefusalKind::NonFinite,
                "r6_base_not_finite",
                format!("base coordinate {i} = {value} is not finite"),
            ));
        }
    }
    for (i, value) in offset.iter().enumerate() {
        if !value.is_finite() {
            return Err(refusal(
                RefusalKind::NonFinite,
                "r6_offset_not_finite",
                format!("offset coordinate {i} = {value} is not finite"),
            ));
        }
    }
    Ok(())
}

/// The certified chart of a nonzero offset direction (Theorem 13.1 exact
/// cover).
///
/// A self-pair is an unordered pair, so `δ` and `−δ` describe the same
/// witness. [`r6_witness`] returns the canonical representative — the member of
/// `{δ, −δ}` whose dominant step is positive — so `r6_witness(δ) ==
/// r6_witness(−δ)` always, and witnesses are never double-counted. The chart is
/// decided on the canonical magnitudes exactly as Theorem 13.1 states:
/// `|h| ≥ |k|` is chart A (unique there), `|k| > |h|` is chart B.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Chart {
    /// Chart A: `|h| ≥ |k|`, offset `(λ, λ·m)`, `|m| ≤ 1`.
    A,
    /// Chart B: `|k| > |h|`, offset `(λ·m, λ)`, `|m| < 1`.
    B,
}

/// The chart data of an R6 witness: one of the two exact-cover charts with the
/// step magnitude `λ` and the signed slope `m`.
///
/// Chart A encodes the offset `(λ, λ·m)`; chart B encodes the offset
/// `(λ·m, λ)`. The degenerate (zero) offset is carried as chart A with
/// `λ = m = 0` by [`r6_witness`] and refused by [`r6_witness_checked`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChartChoice {
    /// Chart A data: the offset is `(λ, λ·m)`.
    A {
        /// The dominant step magnitude along `u`.
        lambda: f64,
        /// The signed slope `k/h`, `|m| ≤ 1`.
        m: f64,
    },
    /// Chart B data: the offset is `(λ·m, λ)`.
    B {
        /// The dominant step magnitude along `v`.
        lambda: f64,
        /// The signed slope `h/k`, `|m| < 1`.
        m: f64,
    },
}

impl ChartChoice {
    /// The chart this data belongs to (Theorem 13.1).
    pub fn chart(&self) -> Chart {
        match self {
            ChartChoice::A { .. } => Chart::A,
            ChartChoice::B { .. } => Chart::B,
        }
    }

    /// The step magnitude `λ` carried by this chart data.
    pub fn lambda(&self) -> f64 {
        match self {
            ChartChoice::A { lambda, .. } | ChartChoice::B { lambda, .. } => *lambda,
        }
    }

    /// The signed slope `m` carried by this chart data.
    pub fn m(&self) -> f64 {
        match self {
            ChartChoice::A { m, .. } | ChartChoice::B { m, .. } => *m,
        }
    }

    /// The offset `δ = (h, k)` this chart data encodes.
    pub fn offset(&self) -> [f64; 2] {
        match self {
            ChartChoice::A { lambda, m } => [*lambda, lambda * m],
            ChartChoice::B { lambda, m } => [lambda * m, *lambda],
        }
    }
}

/// The canonical chart of an offset (Theorem 13.1 exact cover), on the
/// canonical member of `{δ, −δ}` whose dominant step is positive.
///
/// Precondition: `δ` is a finite nonzero offset — a genuine self-pair witness
/// never has a zero offset. The degenerate zero offset (and any non-finite
/// input) maps to the diagonal marker `A { λ: 0, m: 0 }`; callers that need a
/// refusal instead use [`r6_witness_checked`].
pub fn r6_witness(delta: [f64; 2]) -> ChartChoice {
    let [h, k] = delta;
    if !h.is_finite() || !k.is_finite() {
        return ChartChoice::A {
            lambda: 0.0,
            m: 0.0,
        };
    }
    if h.abs() >= k.abs() {
        if h < 0.0 {
            return r6_witness([-h, -k]);
        }
        if h == 0.0 {
            return ChartChoice::A {
                lambda: 0.0,
                m: 0.0,
            };
        }
        ChartChoice::A {
            lambda: h,
            m: k / h,
        }
    } else {
        if k < 0.0 {
            return r6_witness([-h, -k]);
        }
        ChartChoice::B {
            lambda: k,
            m: h / k,
        }
    }
}

/// The refusing chart selector: like [`r6_witness`], but refuses a zero or
/// non-finite offset with a named predicate instead of returning the diagonal
/// marker.
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
pub fn r6_witness_checked(delta: [f64; 2]) -> Construction<ChartChoice> {
    let [h, k] = delta;
    if !h.is_finite() || !k.is_finite() {
        return Err(refusal(
            RefusalKind::NonFinite,
            "r6_witness_offset_not_finite",
            format!("witness offset {delta:?} is not finite"),
        ));
    }
    if h == 0.0 && k == 0.0 {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "r6_witness_zero_offset",
            "the zero offset has no chart: a self-pair needs two distinct points".to_string(),
        ));
    }
    Ok(r6_witness(delta))
}

/// Whether `δ` is the admissible member of the antipodal pair `{δ, −δ}`.
///
/// Theorem 13.1's cover is stated on the canonical representative of the
/// unordered direction: exactly one of `δ` and `−δ` is admissible (the member
/// whose dominant step is positive). The zero offset is admissible by neither.
pub fn r6_admits(delta: [f64; 2]) -> bool {
    let [h, k] = delta;
    if !h.is_finite() || !k.is_finite() {
        return false;
    }
    if h.abs() >= k.abs() {
        h > 0.0
    } else {
        k > 0.0
    }
}

/// The outcome of an R6 chart transition at a break point: which segment break
/// was emitted, the base point on the far side of the break, and the far-side
/// chart data.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct R6Transition {
    /// The emitted segment break kind: `R6ChartSwitch` (Type I) or
    /// `R6BaseSwap` (Type II).
    pub break_kind: SegmentBreak,
    /// The base point after the transition.
    pub base: [f64; 2],
    /// The chart data on the far side of the transition.
    pub choice: ChartChoice,
}

/// Type I transition (Theorem 13.3): chart A at `m = +1` re-parameterizes the
/// SAME base point into chart B with `m_B = 1/m_A` and `λ_B = λ_A·m_A`, and
/// emits [`SegmentBreak::R6ChartSwitch`].
///
/// The two sides encode the same ordered offset from the same base, so the far
/// point of the witness — and therefore the witness surface point — is
/// identical across the break (Corollary 13.2's unordered-pair identity).
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
pub fn r6_transition_type1(base: [f64; 2], choice: ChartChoice) -> Construction<R6Transition> {
    if !base[0].is_finite() || !base[1].is_finite() {
        return Err(refusal(
            RefusalKind::NonFinite,
            "r6_transition_base_not_finite",
            format!("transition base {base:?} is not finite"),
        ));
    }
    let (lambda_a, m_a) = match choice {
        ChartChoice::A { lambda, m } => (lambda, m),
        ChartChoice::B { .. } => {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "r6_type1_requires_chart_a",
                "the Type I transition starts on chart A at m = +1".to_string(),
            ))
        }
    };
    if !lambda_a.is_finite() || !m_a.is_finite() {
        return Err(refusal(
            RefusalKind::NonFinite,
            "r6_transition_choice_not_finite",
            format!("Type I chart data ({lambda_a}, {m_a}) is not finite"),
        ));
    }
    if (m_a - 1.0).abs() > EPS_REP {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "r6_type1_requires_m_plus_one",
            format!("Type I fires at m_A = +1, received m = {m_a}"),
        ));
    }
    let lambda_b = lambda_a * m_a;
    let m_b = 1.0 / m_a;
    Ok(R6Transition {
        break_kind: SegmentBreak::R6ChartSwitch,
        base,
        choice: ChartChoice::B {
            lambda: lambda_b,
            m: m_b,
        },
    })
}

/// Type II transition (Theorem 13.3): chart A at `m = −1` swaps the base point
/// to the far member of the unordered pair (the base moves by the encoded
/// offset `(λ_A, λ_A·m_A)`), and emits [`SegmentBreak::R6BaseSwap`].
///
/// The far-side chart data points back from the new base to the old base, so
/// the two bases carry the SAME unordered pair and the witness surface point
/// `S(base) = S(new base)` is preserved across the break.
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
pub fn r6_transition_type2(base: [f64; 2], choice: ChartChoice) -> Construction<R6Transition> {
    if !base[0].is_finite() || !base[1].is_finite() {
        return Err(refusal(
            RefusalKind::NonFinite,
            "r6_transition_base_not_finite",
            format!("transition base {base:?} is not finite"),
        ));
    }
    let (lambda_a, m_a) = match choice {
        ChartChoice::A { lambda, m } => (lambda, m),
        ChartChoice::B { .. } => {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "r6_type2_requires_chart_a",
                "the Type II transition starts on chart A at m = -1".to_string(),
            ))
        }
    };
    if !lambda_a.is_finite() || !m_a.is_finite() {
        return Err(refusal(
            RefusalKind::NonFinite,
            "r6_transition_choice_not_finite",
            format!("Type II chart data ({lambda_a}, {m_a}) is not finite"),
        ));
    }
    if (m_a + 1.0).abs() > EPS_REP {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "r6_type2_requires_m_minus_one",
            format!("Type II fires at m_A = -1, received m = {m_a}"),
        ));
    }
    if lambda_a == 0.0 {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "r6_type2_zero_lambda",
            "a Type II base swap needs a nonzero step magnitude".to_string(),
        ));
    }
    // The offset this chart data encodes, from the current base to the far
    // member of the unordered pair.
    let offset = choice.offset();
    let new_base = [base[0] + offset[0], base[1] + offset[1]];
    // Far-side data: point from the new base back to the old base. For
    // m_A = -1 the offset is (λ_A, -λ_A), its negative is (-λ_A, λ_A), and
    // chart A data with λ = -λ_A, m = -1 encodes exactly (-λ_A, λ_A).
    let lambda2 = -offset[0];
    let m2 = offset[1] / offset[0];
    Ok(R6Transition {
        break_kind: SegmentBreak::R6BaseSwap,
        base: new_base,
        choice: ChartChoice::A {
            lambda: lambda2,
            m: m2,
        },
    })
}

/// The §13.4 λ = 0 routing decision for a chart-A datum whose step magnitude
/// collapsed to zero.
///
/// At `λ = 0` the deflated chart-A residual reduces to `S_u + m·S_v`, which is
/// solvable only at a parametric degeneracy — never at a genuine transversal
/// self-intersection. The isolated-contact classifier (§10.3) is deliberately
/// NOT a route:
///
/// * [`LambdaZeroRoute::Chart`] — the leaf is regular at the base and
///   `S_u + m·S_v` does not vanish: no λ = 0 self-pair zero exists, the trace
///   continues in the charts away from the diagonal;
/// * [`LambdaZeroRoute::Carrier`] — the partials are (certifiably)
///   degenerate at the base: the stratum is a leaf/carrier-level parametric
///   degeneracy, owned by the carrier, never by the contact classifier.
///
/// A caller that insists on the §3.4 (contact-classifier) route instead
/// receives the typed refusal [`r6_lambda_zero_refusal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LambdaZeroRoute {
    /// Route back into the chart machinery: the λ = 0 stratum has no zero on a
    /// regular leaf.
    Chart,
    /// Route to the carrier/leaf level: the stratum is a parametric
    /// degeneracy of the leaf itself.
    Carrier,
}

/// The partial derivative of the affine surface `S = P/w` with respect to
/// `axis` (`0` for `u`, `1` for `v`) at `base`, as a three-vector. The
/// quotient is evaluated once per coordinate over the leaf's polynomial nets
/// (N5: the plain `f64` weight value at the point, never an interval
/// division).
fn affine_partial(leaf: &BezierLeaf, base: [f64; 2], axis: usize) -> Option<[f64; 3]> {
    let [u, v] = base;
    if !u.is_finite() || !v.is_finite() {
        return None;
    }
    let w0 = bernstein_eval_2d(&leaf_grid(leaf, 3), u, v)?;
    let wd = bernstein_eval_2d(&bernstein_derivative_2d(&leaf_grid(leaf, 3), axis), u, v)?;
    let den = w0 * w0;
    if den == 0.0 {
        return None;
    }
    let mut out = [0.0; 3];
    for (comp, slot) in out.iter_mut().enumerate() {
        let p0 = bernstein_eval_2d(&leaf_grid(leaf, comp), u, v)?;
        let pd = bernstein_eval_2d(&bernstein_derivative_2d(&leaf_grid(leaf, comp), axis), u, v)?;
        *slot = (pd * w0 - p0 * wd) / den;
    }
    Some(out)
}

/// Decide the §13.4 λ = 0 routing for a chart-A slope `m` at the base point on
/// `leaf`.
///
/// The decision is made from the certified-signable geometry of the partials
/// at the base (evaluated on the leaf's nets, squared-norm comparisons only):
/// a degenerate pair of partials routes to the carrier, a regular pair with no
/// `S_u + m·S_v` zero routes back into the charts. The contact classifier is
/// never a route.
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
pub fn r6_lambda_zero(leaf: &BezierLeaf, base: [f64; 2], m: f64) -> Construction<LambdaZeroRoute> {
    validate_leaf(leaf)?;
    if !m.is_finite() {
        return Err(refusal(
            RefusalKind::NonFinite,
            "r6_lambda_zero_m_not_finite",
            format!("lambda-zero slope m = {m} is not finite"),
        ));
    }
    if m.abs() > 1.0 {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "r6_lambda_zero_m_outside_chart_a",
            format!("lambda-zero slope m = {m} is outside chart A's |m| <= 1"),
        ));
    }
    let su = match affine_partial(leaf, base, 0) {
        Some(su) => su,
        None => {
            return Err(refusal(
                RefusalKind::NonFinite,
                "r6_lambda_zero_partial_unavailable",
                format!("partials at {base:?} could not be evaluated"),
            ))
        }
    };
    let sv = match affine_partial(leaf, base, 1) {
        Some(sv) => sv,
        None => {
            return Err(refusal(
                RefusalKind::NonFinite,
                "r6_lambda_zero_partial_unavailable",
                format!("partials at {base:?} could not be evaluated"),
            ))
        }
    };
    // Squared-norm degeneracy of the parametrization at the base:
    // |S_u × S_v|^2 == 0 (up to the representation floor) means the partials
    // are parallel, i.e. a parametric degeneracy owns the stratum.
    let cross = [
        su[1] * sv[2] - su[2] * sv[1],
        su[2] * sv[0] - su[0] * sv[2],
        su[0] * sv[1] - su[1] * sv[0],
    ];
    let cross_sq = cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2];
    let degenerate_sq = EPS_REP * EPS_REP;
    if cross_sq <= degenerate_sq {
        return Ok(LambdaZeroRoute::Carrier);
    }
    Ok(LambdaZeroRoute::Chart)
}

/// The typed refusal of the §3.4 route for a λ = 0 datum.
///
/// Theorem 13.4's stratum reduces chart A at `λ = 0` to `S_u + m·S_v = 0`,
/// solvable only at a parametric degeneracy; feeding such a datum to the
/// §10.3 isolated-contact classifier is never done. This refusal names the
/// §3.4 route in its predicate so the caller can re-route (chart or carrier)
/// instead.
pub fn r6_lambda_zero_refusal(base: [f64; 2], m: f64) -> Refusal {
    refusal(
        RefusalKind::Conditioning,
        "r6_lambda_zero_section_3_4_route",
        format!(
            "lambda-zero stratum at base {base:?} with slope m = {m}: R6_A reduces to \
             S_u + m S_v = 0, solvable only at parametric degeneracy; the section 3.4 \
             (contact-classifier) route is refused, never taken"
        ),
    )
}

/// The R6 self-intersection system over one validated surface leaf: the
/// numerator-form residual and the deflated divided-difference nets the packet
/// exercises.
#[derive(Debug, Clone, PartialEq)]
pub struct R6System {
    /// The surface leaf whose self-intersections the system certifies over.
    leaf: BezierLeaf,
}

// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
impl R6System {
    /// Build the R6 system, refusing a leaf that is not a certified rational
    /// homogeneous net (non-finite, zero-degree, or non-positive-weight data).
    pub fn try_new(leaf: &BezierLeaf) -> Construction<R6System> {
        validate_leaf(leaf)?;
        Ok(R6System { leaf: leaf.clone() })
    }

    /// The surface leaf the system certifies over.
    pub fn leaf(&self) -> &BezierLeaf {
        &self.leaf
    }

    /// The R6 self-intersection residual of the leaf at `base` with `offset`,
    /// in the D-homogeneous NUMERATOR form: coordinate `c` is
    /// `P_c(base + offset)·w(base) − P_c(base)·w(base + offset)`.
    ///
    /// This is the R8/R9 cross-multiplied discipline (§7.1): no division by
    /// any weight ever occurs here; for a unit-weight (polynomial) leaf it is
    /// exactly `P_c(base + offset) − P_c(base)`. The zero set (with certified
    /// positive weights as the VALUE argument) is the self-pair locus.
    pub fn residual(&self, base: [f64; 2], offset: [f64; 2]) -> Construction<[f64; 3]> {
        check_uv(base, offset)?;
        let near = [base[0] + offset[0], base[1] + offset[1]];
        let mut out = [0.0; 3];
        let w0 = match bernstein_eval_2d(&leaf_grid(&self.leaf, 3), base[0], base[1]) {
            Some(w0) => w0,
            None => {
                return Err(refusal(
                    RefusalKind::NonFinite,
                    "r6_eval_unavailable",
                    format!("weight evaluation at {base:?} is unavailable"),
                ))
            }
        };
        let w1 = match bernstein_eval_2d(&leaf_grid(&self.leaf, 3), near[0], near[1]) {
            Some(w1) => w1,
            None => {
                return Err(refusal(
                    RefusalKind::NonFinite,
                    "r6_eval_unavailable",
                    format!("weight evaluation at {near:?} is unavailable"),
                ))
            }
        };
        for (comp, value) in out.iter_mut().enumerate() {
            let p0 = match bernstein_eval_2d(&leaf_grid(&self.leaf, comp), base[0], base[1]) {
                Some(p0) => p0,
                None => {
                    return Err(refusal(
                        RefusalKind::NonFinite,
                        "r6_eval_unavailable",
                        format!("coordinate {comp} evaluation at {base:?} is unavailable"),
                    ))
                }
            };
            let p1 = match bernstein_eval_2d(&leaf_grid(&self.leaf, comp), near[0], near[1]) {
                Some(p1) => p1,
                None => {
                    return Err(refusal(
                        RefusalKind::NonFinite,
                        "r6_eval_unavailable",
                        format!("coordinate {comp} evaluation at {near:?} is unavailable"),
                    ))
                }
            };
            *value = p1 * w0 - p0 * w1;
        }
        Ok(out)
    }
}
