//! ADM-L1-EXTRACT conformance tests: the L1 lemma — exact Bézier extraction
//! over the landed spline faces (spec Theorem T3′).
//!
//! The lemma is proven by exact evaluation over synthetic spline faces with
//! KNOWN closed forms: on every knot rectangle, the extracted tensor-Bernstein
//! patch `Â/Ŵ` equals the original homogeneous tensor-spline face identically
//! (same rational surface, different basis — knot insertion is an exact local
//! change of basis, no approximation, no sampling). The tests are:
//!
//! 1. **`extraction_identity_exact_on_synthetic_splines`** — the machine
//!    proof: extract every face, then evaluate each extracted patch at matched
//!    exact parameters and compare with the face's own substitution AND the
//!    fixture's closed form. Covered fixtures (all positive-weight clamped
//!    homogeneous `BSplineSurface<Vector4>` faces):
//!    - [`bilinear_spline_face`]: a bilinear (1,1) spline over `[0,2]×[0,3]`
//!      with interior single knots in both axes (2×3 rectangles), closed form
//!      `X(u,v) = (u, v, u·v/6)`.
//!    - [`biquadratic_spline_face`]: a smooth biquadratic (2,2) spline over
//!      the same knot grid built from its monomial closed form
//!      `X(u,v) = (u, v, (u−1)² + (v−1.5)²)` by the B-spline blossom map
//!      (independent reference construction).
//!    - [`sphere_octant_face`]: the exact biquadratic NURBS octant of the unit
//!      sphere (a genuinely rational face with non-constant positive weights;
//!      closed-form invariant `|X| = 1`).
//!    - [`weighted_rational_spline_face`]: the rational spline
//!      `X = (w(u)·Γ(v))/w(u) = Γ(v)` with `Γ(v) = (v², v, v)` and the
//!      strictly positive weight spline `w(u) = (u−1)² + 1` over interior
//!      single knots in both axes — a multi-span rational face whose weights
//!      vary across the `u` spans.
//! 2. **`span_enumeration_covers_the_domain_exactly`** — the rectangles
//!    returned by [`extract_patches`] partition each face's declared domain
//!    exactly: the sorted domain boxes equal the unique-knot grid (no gap, no
//!    overlap, shared boundary), the counts match the rectangle product, and
//!    every patch's bidegree equals the face's degrees.
//! 3. **`weight_brackets_certified_positive_or_refused`** — every extracted
//!    patch of a positive-weight face carries a certified strictly positive
//!    weight bracket (the exact axis hull of its weight coefficients, with
//!    sampled field values inside it), and a face whose weight field cannot be
//!    certified positive (a negative homogeneous weight at a corner) REFUSES
//!    [`ConstructRefusal::InvalidInput`] — never a silent non-positive weight.
//!
//! The fixtures are plain `f64` reference data (tests are allowed H-3
//! opt-outs); the identity slack is a named constant
//! [`IDENTITY_REL_TOL`]. Nothing here touches a corpus, a boolean boundary, or
//! any evidence certificate (SFC — pure exact algebra over synthetic faces).

#![deny(clippy::all, rust_2018_idioms)]
#![deny(clippy::unwrap_used)]

use truck_certified::construct::extract::extract_patches;
use truck_certified::construct::refusal::ConstructRefusal;
use truck_geometry::base::ParametricSurface;
use truck_geometry::prelude::{BSplineSurface, KnotVec, Vector4};

/// The quarter-circle middle weight `√2/2` of the exact NURBS arc nets (the
/// exact float value of `cos(π/4)`; H-3: named, never a bare literal).
const SQRT_HALF: f64 = std::f64::consts::FRAC_1_SQRT_2;

/// The axis weights of the octant fixture arcs: `1` at the ends, `√2/2` in the
/// middle (H-3: named constant).
const ARC_WEIGHTS: [f64; 3] = [1.0, SQRT_HALF, 1.0];

/// The relative identity slack of the lemma machine check: an extracted patch
/// and the face's own substitution must agree to this RELATIVE bound at unit
/// scale (the extraction and the substitution are two different float
/// evaluation paths of the same exact change of basis; a real algebra error is
/// O(1) at this scale, so the bound separates exactness from error by many
/// orders of magnitude). H-3: named.
const IDENTITY_REL_TOL: f64 = 1.0e-9;

/// The exact bilinear `(1,1)` spline face over `u ∈ [0,2]`, `v ∈ [0,3]` with
/// interior single knots in both axes: `X(u, v) = (u, v, u·v/6)`.
///
/// Degree-1 clamped splines interpolate their net at the knots, so the control
/// net is the closed form sampled at the knot grid — an exact construction
/// that needs no blossom machinery. The surface has `2 × 3` knot rectangles
/// and unit weights.
fn bilinear_spline_face() -> BSplineSurface<Vector4> {
    let u_knots = KnotVec::from(vec![0.0, 0.0, 1.0, 2.0, 2.0]);
    let v_knots = KnotVec::from(vec![0.0, 0.0, 1.0, 2.0, 3.0, 3.0]);
    let u_nodes = [0.0, 1.0, 2.0];
    let v_nodes = [0.0, 1.0, 2.0, 3.0];
    let mut ctrl: Vec<Vec<Vector4>> = Vec::new();
    for &u in &u_nodes {
        let mut row: Vec<Vector4> = Vec::new();
        for &v in &v_nodes {
            row.push(Vector4::new(u, v, u * v / 6.0, 1.0));
        }
        ctrl.push(row);
    }
    BSplineSurface::new((u_knots, v_knots), ctrl)
}

/// The exact bilinear closed form `(u, v, u·v/6)`.
fn bilinear_closed(u: f64, v: f64) -> [f64; 3] {
    [u, v, u * v / 6.0]
}

/// The smooth biquadratic `(2,2)` spline face over `u ∈ [0,2]`, `v ∈ [0,3]`
/// with interior single knots in both axes: `X(u, v) = (u, v, (u−1)² +
/// (v−1.5)²)`.
///
/// The control net is the independent B-spline blossom construction: a
/// polynomial of bidegree ≤ (2,2) is representable exactly on the clamped
/// spline, and its control coefficient at net index `(i, j)` is the tensor
/// blossom evaluated at the consecutive knot windows `(κᵤ[i+1], κᵤ[i+2])` and
/// `(κᵥ[j+1], κᵥ[j+2])`. For the monomial `uᵖv^q` the blossom contribution is
/// `eₚ(windowᵤ)/C(2,p) · e_q(windowᵥ)/C(2,q)` with `e_r` the elementary
/// symmetric sums. This is reference arithmetic independent of the extraction
/// under test.
fn biquadratic_spline_face() -> BSplineSurface<Vector4> {
    let u_knots = KnotVec::from(vec![0.0, 0.0, 0.0, 1.0, 2.0, 2.0, 2.0]);
    let v_knots = KnotVec::from(vec![0.0, 0.0, 0.0, 1.0, 2.0, 3.0, 3.0, 3.0]);
    // The monomial coefficient grid of X(u, v) = (u, v, (u−1)² + (v−1.5)²):
    // monom[p][q] is the R³ coefficient of uᵖ v^q.
    //   x = u        -> monom[1][0] = (1, 0, 0)
    //   y = v        -> monom[0][1] = (0, 1, 0)
    //   z = u² − 2u + v² − 3v + 3.25 -> the remaining entries.
    let mut monom: Vec<Vec<[f64; 3]>> = vec![vec![[0.0; 3]; 3]; 3];
    monom[1][0][0] = 1.0;
    monom[0][1][1] = 1.0;
    monom[2][0][2] = 1.0;
    monom[1][0][2] = -2.0;
    monom[0][2][2] = 1.0;
    monom[0][1][2] = -3.0;
    monom[0][0][2] = 3.25;
    let net = tensor_blossom_net(2, 2, u_knots.as_slice(), v_knots.as_slice(), &monom);
    let weights = ones_weights(&net);
    homogeneous_face(u_knots, v_knots, net, weights)
}

/// The exact biquadratic closed form `(u, v, (u−1)² + (v−1.5)²)`.
fn biquadratic_closed(u: f64, v: f64) -> [f64; 3] {
    let z = (u - 1.0) * (u - 1.0) + (v - 1.5) * (v - 1.5);
    [u, v, z]
}

/// The exact biquadratic NURBS octant of the unit sphere (a genuinely rational
/// face: non-constant strictly positive weights, all surface points on the
/// unit sphere). Single span `[0,1]²`, bidegree `(2,2)`.
fn sphere_octant_face() -> BSplineSurface<Vector4> {
    let knots = KnotVec::from(vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
    let (numerator, weights) = sphere_octant_nets(1.0);
    homogeneous_face(knots.clone(), knots, numerator, weights)
}

/// The exact net grids of one octant of the unit sphere (the ADM-SHIM octant
/// fixture nets): `z_sign = +1` is the up octant. The net is the tensor of the
/// exact rational-quadratic quarter-arc nets `(w·Q, w)` with
/// `w = (1, √2/2, 1)`.
fn sphere_octant_nets(z_sign: f64) -> (Vec<Vec<[f64; 3]>>, Vec<Vec<f64>>) {
    let profile: [[f64; 3]; 3] = [[1.0, 0.0, 0.0], [1.0, 0.0, z_sign], [0.0, 0.0, z_sign]];
    let turn: [[f64; 2]; 3] = [[1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let mut numerator: Vec<Vec<[f64; 3]>> = Vec::new();
    let mut weights: Vec<Vec<f64>> = Vec::new();
    for i in 0..3 {
        let mut num_row: Vec<[f64; 3]> = Vec::new();
        let mut w_row: Vec<f64> = Vec::new();
        for j in 0..3 {
            let x = profile[i][0] * turn[j][0];
            let y = profile[i][0] * turn[j][1];
            let z = profile[i][2];
            let w = ARC_WEIGHTS[i] * ARC_WEIGHTS[j];
            num_row.push([w * x, w * y, w * z]);
            w_row.push(w);
        }
        numerator.push(num_row);
        weights.push(w_row);
    }
    (numerator, weights)
}

/// A multi-span RATIONAL spline face whose weight field varies across the `u`
/// spans while the geometry is the closed-form curve
/// `X(u, v) = Γ(v) = (v², v, v)`.
///
/// `X = A/W` with `A(u, v) = w(u)·Γ(v)` and `W(u, v) = w(u)`, where the weight
/// spline `w(u) = (u−1)² + 1` (strictly positive, blossom coefficients in
/// `[1, 2]`) is a quadratic spline over `u` knots `[0,0,0,1,2,2,2]` and `Γ` is
/// a quadratic spline curve over `v` knots `[0,0,0,1,2,3,3,3]`. The
/// homogeneous control net of the tensor representation is
/// `P[i][j] = (wᵢ·Γⱼ, wᵢ)` with `wᵢ`, `Γⱼ` the u/v blossom coefficients —
/// exact by the same blossom map, in both axes at once because the product
/// factorizes. Bidegree `(2,2)`, `2 × 3` knot rectangles.
fn weighted_rational_spline_face() -> BSplineSurface<Vector4> {
    let u_knots = KnotVec::from(vec![0.0, 0.0, 0.0, 1.0, 2.0, 2.0, 2.0]);
    let v_knots = KnotVec::from(vec![0.0, 0.0, 0.0, 1.0, 2.0, 3.0, 3.0, 3.0]);
    // w(u) = u² − 2u + 2, monomial coefficients ascending in u.
    let w_monom = [2.0, -2.0, 1.0];
    let w_coeffs = scalar_blossom(2, u_knots.as_slice(), &w_monom);
    // Gamma(v) = (v², v, v): R³ monomial coefficients ascending in v.
    let gamma_monom: [[f64; 3]; 3] = [[0.0, 0.0, 0.0], [0.0, 1.0, 1.0], [1.0, 0.0, 0.0]];
    let gamma_coeffs = vector_blossom(2, v_knots.as_slice(), &gamma_monom);
    let mut numerator: Vec<Vec<[f64; 3]>> = Vec::new();
    let mut weights: Vec<Vec<f64>> = Vec::new();
    for &w in &w_coeffs {
        let mut num_row: Vec<[f64; 3]> = Vec::new();
        let mut w_row: Vec<f64> = Vec::new();
        for g in &gamma_coeffs {
            num_row.push([w * g[0], w * g[1], w * g[2]]);
            w_row.push(w);
        }
        numerator.push(num_row);
        weights.push(w_row);
    }
    homogeneous_face(u_knots, v_knots, numerator, weights)
}

/// The weighted-rational closed form `(v², v, v)` (independent of `u`).
fn weighted_rational_closed(_u: f64, v: f64) -> [f64; 3] {
    [v * v, v, v]
}

/// A face whose weight field is NOT certifiably positive: a single-span
/// bilinear face whose `(u, v) = (0, 0)` corner carries the homogeneous weight
/// `−1/2`. The corner span's weight bracket cannot certify strictly positive,
/// so extraction must refuse [`ConstructRefusal::InvalidInput`].
fn non_positive_weight_face() -> BSplineSurface<Vector4> {
    let knots = KnotVec::from(vec![0.0, 0.0, 1.0, 1.0]);
    let ctrl = vec![
        vec![
            Vector4::new(0.0, 0.0, 0.0, -0.5),
            Vector4::new(0.0, 1.0, 0.0, 1.0),
        ],
        vec![
            Vector4::new(1.0, 0.0, 0.0, 1.0),
            Vector4::new(1.0, 1.0, 1.0, 1.0),
        ],
    ];
    BSplineSurface::new((knots.clone(), knots), ctrl)
}

/// Build the homogeneous carrier face from an `R³` numerator net and a same
/// shape scalar weight net: `control[i][j] = (Â[i][j], Ŵ[i][j])`.
fn homogeneous_face(
    u_knots: KnotVec,
    v_knots: KnotVec,
    numerator: Vec<Vec<[f64; 3]>>,
    weights: Vec<Vec<f64>>,
) -> BSplineSurface<Vector4> {
    let mut ctrl: Vec<Vec<Vector4>> = Vec::new();
    for (i, row) in numerator.iter().enumerate() {
        let mut ctrl_row: Vec<Vector4> = Vec::new();
        for (j, a) in row.iter().enumerate() {
            ctrl_row.push(Vector4::new(a[0], a[1], a[2], weights[i][j]));
        }
        ctrl.push(ctrl_row);
    }
    BSplineSurface::new((u_knots, v_knots), ctrl)
}

/// A same-shape unit weight net for the polynomial fixtures.
fn ones_weights(numerator: &[Vec<[f64; 3]>]) -> Vec<Vec<f64>> {
    numerator.iter().map(|row| vec![1.0; row.len()]).collect()
}

/// The `r`-th elementary symmetric sum of a window, for `r = 0..=window.len()`:
/// `e[0] = 1`, `e[r]` = the sum of the products of all `r`-subsets.
fn elementary_sums(window: &[f64]) -> Vec<f64> {
    let mut e = vec![0.0; window.len() + 1];
    e[0] = 1.0;
    for &w in window {
        for k in (1..e.len()).rev() {
            e[k] += e[k - 1] * w;
        }
    }
    e
}

/// The integer binomial `C(n, k)` for the small blossom degrees.
fn binomial(n: usize, k: usize) -> usize {
    let mut r = 1usize;
    for i in 1..=k {
        r = r * (n - k + i) / i;
    }
    r
}

/// The univariate blossom factors of a knot window of length `degree`:
/// `factor[r] = e_r(window) / C(degree, r)` for `r = 0..=degree`, so that the
/// B-spline coefficient of the monomial `uᵖ` at a net index is exactly
/// `factor[p]` of its consecutive knot window.
fn blossom_factors(degree: usize, window: &[f64]) -> Vec<f64> {
    debug_assert_eq!(window.len(), degree);
    let e = elementary_sums(window);
    let mut factors = Vec::with_capacity(degree + 1);
    for r in 0..=degree {
        factors.push(e[r] / binomial(degree, r) as f64);
    }
    factors
}

/// The univariate scalar B-spline blossom: control coefficients of the clamped
/// degree-`degree` spline over `knots` that reproduces the scalar polynomial
/// with the ascending monomial coefficients `monom` (length `degree + 1`).
fn scalar_blossom(degree: usize, knots: &[f64], monom: &[f64]) -> Vec<f64> {
    let nctrl = knots.len() - degree - 1;
    let mut out = Vec::with_capacity(nctrl);
    for i in 0..nctrl {
        let window = &knots[i + 1..i + 1 + degree];
        let factors = blossom_factors(degree, window);
        let mut acc = 0.0;
        for (p, &m) in monom.iter().enumerate() {
            acc += m * factors[p];
        }
        out.push(acc);
    }
    out
}

/// The univariate `R³`-valued B-spline blossom (same map as
/// [`scalar_blossom`], component-wise).
fn vector_blossom(degree: usize, knots: &[f64], monom: &[[f64; 3]]) -> Vec<[f64; 3]> {
    let nctrl = knots.len() - degree - 1;
    let mut out = Vec::with_capacity(nctrl);
    for i in 0..nctrl {
        let window = &knots[i + 1..i + 1 + degree];
        let factors = blossom_factors(degree, window);
        let mut acc = [0.0; 3];
        for (p, m) in monom.iter().enumerate() {
            let f = factors[p];
            acc[0] += m[0] * f;
            acc[1] += m[1] * f;
            acc[2] += m[2] * f;
        }
        out.push(acc);
    }
    out
}

/// The tensor B-spline blossom of an `R³`-valued bivariate polynomial of
/// bidegree `(du, dv)`: `monom[p][q]` is the coefficient of `uᵖv^q`, and the
/// net index `(i, j)` coefficient is the tensor blossom over the consecutive
/// knot windows. Rows run over `u`, columns over `v`.
fn tensor_blossom_net(
    du: usize,
    dv: usize,
    u_knots: &[f64],
    v_knots: &[f64],
    monom: &[Vec<[f64; 3]>],
) -> Vec<Vec<[f64; 3]>> {
    let nu_ctrl = u_knots.len() - du - 1;
    let nv_ctrl = v_knots.len() - dv - 1;
    let mut u_factors: Vec<Vec<f64>> = Vec::with_capacity(nu_ctrl);
    for i in 0..nu_ctrl {
        let window = &u_knots[i + 1..i + 1 + du];
        u_factors.push(blossom_factors(du, window));
    }
    let mut v_factors: Vec<Vec<f64>> = Vec::with_capacity(nv_ctrl);
    for j in 0..nv_ctrl {
        let window = &v_knots[j + 1..j + 1 + dv];
        v_factors.push(blossom_factors(dv, window));
    }
    let mut net: Vec<Vec<[f64; 3]>> = Vec::with_capacity(nu_ctrl);
    for i in 0..nu_ctrl {
        let mut row: Vec<[f64; 3]> = Vec::with_capacity(nv_ctrl);
        for j in 0..nv_ctrl {
            let mut acc = [0.0; 3];
            for (p, u_row) in monom.iter().enumerate() {
                let fu = u_factors[i][p];
                for (q, m) in u_row.iter().enumerate() {
                    let f = fu * v_factors[j][q];
                    acc[0] += m[0] * f;
                    acc[1] += m[1] * f;
                    acc[2] += m[2] * f;
                }
            }
            row.push(acc);
        }
        net.push(row);
    }
    net
}

/// Plain `f64` 1-D de Casteljau evaluation, the tests' reference arithmetic.
fn eval_bernstein_1d(coeffs: &[f64], t: f64) -> f64 {
    let mut level: Vec<f64> = coeffs.to_vec();
    while level.len() > 1 {
        level = level
            .windows(2)
            .map(|w| (1.0 - t) * w[0] + t * w[1])
            .collect();
    }
    level[0]
}

/// Scalar tensor-Bernstein evaluation of a row-major grid at `(s, t)` (rows
/// over `u`, columns over `v`).
fn eval_scalar_net(grid: &[Vec<f64>], s: f64, t: f64) -> f64 {
    let row_evals: Vec<f64> = grid.iter().map(|row| eval_bernstein_1d(row, t)).collect();
    eval_bernstein_1d(&row_evals, s)
}

/// Vector tensor-Bernstein evaluation of a row-major `R³` grid at `(s, t)`.
fn eval_vector_net(grid: &[Vec<[f64; 3]>], s: f64, t: f64) -> [f64; 3] {
    let mut acc = [0.0; 3];
    let rows = grid.len();
    let cols = grid[0].len();
    for (i, row) in grid.iter().enumerate() {
        let mut row_evals = [0.0; 3];
        for (j, c) in row.iter().enumerate() {
            let buv = bernstein_pair(rows - 1, i, cols - 1, j, s, t);
            row_evals[0] += buv * c[0];
            row_evals[1] += buv * c[1];
            row_evals[2] += buv * c[2];
        }
        acc[0] += row_evals[0];
        acc[1] += row_evals[1];
        acc[2] += row_evals[2];
    }
    acc
}

/// The basis product `Bᵢ(u)(s)·Bⱼ(v)(t)` of the tensor net.
fn bernstein_pair(mu: usize, i: usize, nv: usize, j: usize, s: f64, t: f64) -> f64 {
    let bu = bernstein_1d(mu, i, s);
    let bv = bernstein_1d(nv, j, t);
    bu * bv
}

/// The degree-`degree` Bernstein basis value at `t`.
fn bernstein_1d(degree: usize, k: usize, t: f64) -> f64 {
    let c = binomial(degree, k) as f64;
    c * t.powi(k as i32) * (1.0 - t).powi((degree - k) as i32)
}

/// The dehomogenized surface point `X(u, v) = (x/w, y/w, z/w)` of the face's
/// own substitution at the SOURCE parameters.
fn surface_point(face: &BSplineSurface<Vector4>, u: f64, v: f64) -> [f64; 3] {
    let p = face.subs(u, v);
    [p.x / p.w, p.y / p.w, p.z / p.w]
}

/// The dehomogenized surface point of an extracted patch at the SOURCE
/// parameters `(u, v)` (mapped onto the patch's unit square from its domain
/// box).
fn patch_point(
    patch: &truck_certified::construct::patches::TensorBernsteinPatch,
    u: f64,
    v: f64,
) -> [f64; 3] {
    let dom = patch.domain();
    let s = (u - dom.lo[0]) / (dom.hi[0] - dom.lo[0]);
    let t = (v - dom.lo[1]) / (dom.hi[1] - dom.lo[1]);
    let num = eval_vector_net(patch.numerator(), s, t);
    let w = eval_scalar_net(patch.weights(), s, t);
    [num[0] / w, num[1] / w, num[2] / w]
}

/// Assert two points agree within the relative identity slack.
fn assert_point_close(a: [f64; 3], b: [f64; 3], what: &str) {
    for k in 0..3 {
        let scale = 1.0 + a[k].abs().max(b[k].abs());
        let bound = IDENTITY_REL_TOL * scale;
        assert!(
            (a[k] - b[k]).abs() <= bound,
            "{what}: coordinate {k} diverged: {a:?} vs {b:?}"
        );
    }
}

/// The shared identity proof over one fixture: the fixture's face reproduces
/// its known closed form (where one is supplied), [`extract_patches`] extracts
/// exactly `(u_breaks.len()-1)·(v_breaks.len()-1)` patches whose domains tile
/// the declared domain, and every patch evaluated at matched exact parameters
/// equals the face's own substitution AND the closed form.
fn assert_extraction_identity(
    face: &BSplineSurface<Vector4>,
    u_breaks: &[f64],
    v_breaks: &[f64],
    closed: Option<&dyn Fn(f64, f64) -> [f64; 3]>,
    label: &str,
) {
    // Fixture sanity: the face reproduces the closed form at every rectangle
    // centre (this validates the fixture construction, including the blossom
    // maps, independently of the extraction under test).
    if let Some(closed_form) = closed {
        for iu in 0..u_breaks.len() - 1 {
            for iv in 0..v_breaks.len() - 1 {
                let u = 0.5 * (u_breaks[iu] + u_breaks[iu + 1]);
                let v = 0.5 * (v_breaks[iv] + v_breaks[iv + 1]);
                let from_face = surface_point(face, u, v);
                let expected = closed_form(u, v);
                assert_point_close(
                    from_face,
                    expected,
                    &format!("{label}: face vs closed form at rectangle ({iu}, {iv})"),
                );
            }
        }
    }

    let patches = match extract_patches(face) {
        Ok(patches) => patches,
        Err(refusal) => panic!("{label}: extraction refused: {refusal:?}"),
    };
    let expected_count = (u_breaks.len() - 1) * (v_breaks.len() - 1);
    assert_eq!(
        patches.len(),
        expected_count,
        "{label}: the extraction must return one patch per knot rectangle"
    );

    let fractions = [0.0f64, 0.25, 0.5, 0.75, 1.0];
    for (idx, patch) in patches.iter().enumerate() {
        let dom = patch.domain();
        for &s in &fractions {
            for &t in &fractions {
                let u = dom.lo[0] + s * (dom.hi[0] - dom.lo[0]);
                let v = dom.lo[1] + t * (dom.hi[1] - dom.lo[1]);
                let from_face = surface_point(face, u, v);
                let from_patch = patch_point(patch, u, v);
                assert_point_close(
                    from_face,
                    from_patch,
                    &format!("{label}: identity at (u = {u}, v = {v}) on patch {idx}"),
                );
                if let Some(closed_form) = closed {
                    let expected = closed_form(u, v);
                    assert_point_close(
                        from_patch,
                        expected,
                        &format!("{label}: patch {idx} vs closed form at (u = {u}, v = {v})"),
                    );
                }
            }
        }
    }
}

#[test]
fn extraction_identity_exact_on_synthetic_splines() {
    // Bilinear multi-span polynomial spline (degree (1,1), 2×3 rectangles).
    let bilinear = bilinear_spline_face();
    assert_extraction_identity(
        &bilinear,
        &[0.0, 1.0, 2.0],
        &[0.0, 1.0, 2.0, 3.0],
        Some(&bilinear_closed),
        "bilinear spline face",
    );

    // Smooth biquadratic multi-span polynomial spline (blossom-built).
    let biquadratic = biquadratic_spline_face();
    assert_extraction_identity(
        &biquadratic,
        &[0.0, 1.0, 2.0],
        &[0.0, 1.0, 2.0, 3.0],
        Some(&biquadratic_closed),
        "biquadratic spline face",
    );

    // The rational NURBS unit-sphere octant (non-constant positive weights,
    // single span): every extracted point must stay on the unit sphere.
    let octant = sphere_octant_face();
    assert_extraction_identity(
        &octant,
        &[0.0, 1.0],
        &[0.0, 1.0],
        None,
        "sphere octant face",
    );
    let octant_patches = extract_patches(&octant)
        .unwrap_or_else(|refusal| panic!("octant extraction refused: {refusal:?}"));
    for patch in &octant_patches {
        let fractions = [0.0f64, 0.2, 0.4, 0.6, 0.8, 1.0];
        for &s in &fractions {
            for &t in &fractions {
                let p = patch_point(patch, s, t);
                let radius_sq = p[0] * p[0] + p[1] * p[1] + p[2] * p[2];
                let bound = IDENTITY_REL_TOL * (1.0 + radius_sq);
                assert!(
                    (radius_sq - 1.0).abs() <= bound,
                    "octant patch point left the unit sphere: |X|² = {radius_sq}"
                );
            }
        }
    }

    // Rational multi-span spline face (weights vary across the u spans).
    let weighted = weighted_rational_spline_face();
    assert_extraction_identity(
        &weighted,
        &[0.0, 1.0, 2.0],
        &[0.0, 1.0, 2.0, 3.0],
        Some(&weighted_rational_closed),
        "weighted rational spline face",
    );
}

#[test]
fn span_enumeration_covers_the_domain_exactly() {
    // Each fixture's declared domain must be partitioned EXACTLY by the
    // extracted patch domains: the sorted domain boxes equal the Cartesian
    // product of the unique-knot grids (adjacent rectangles share their
    // boundary exactly — no gap, no overlap), the count matches the rectangle
    // product, every patch records the expected bidegree, and the parent
    // identity names face ordinal 0.
    let fixtures: Vec<(
        BSplineSurface<Vector4>,
        Vec<f64>,
        Vec<f64>,
        (usize, usize),
        &str,
    )> = vec![
        (
            bilinear_spline_face(),
            vec![0.0, 1.0, 2.0],
            vec![0.0, 1.0, 2.0, 3.0],
            (1, 1),
            "bilinear",
        ),
        (
            biquadratic_spline_face(),
            vec![0.0, 1.0, 2.0],
            vec![0.0, 1.0, 2.0, 3.0],
            (2, 2),
            "biquadratic",
        ),
        (
            sphere_octant_face(),
            vec![0.0, 1.0],
            vec![0.0, 1.0],
            (2, 2),
            "octant",
        ),
        (
            weighted_rational_spline_face(),
            vec![0.0, 1.0, 2.0],
            vec![0.0, 1.0, 2.0, 3.0],
            (2, 2),
            "weighted rational",
        ),
    ];

    for (face, u_breaks, v_breaks, expected_degree, label) in fixtures {
        let patches = match extract_patches(&face) {
            Ok(patches) => patches,
            Err(refusal) => panic!("{label}: extraction refused: {refusal:?}"),
        };
        let nu = u_breaks.len() - 1;
        let nv = v_breaks.len() - 1;
        assert_eq!(
            patches.len(),
            nu * nv,
            "{label}: one patch per knot rectangle"
        );

        let mut actual: Vec<([f64; 2], [f64; 2])> = patches
            .iter()
            .map(|p| {
                let dom = p.domain();
                (dom.lo, dom.hi)
            })
            .collect();
        actual.sort_by(|a, b| {
            a.0[0]
                .partial_cmp(&b.0[0])
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(
                    a.0[1]
                        .partial_cmp(&b.0[1])
                        .unwrap_or(std::cmp::Ordering::Equal),
                )
        });

        let mut expected: Vec<([f64; 2], [f64; 2])> = Vec::new();
        for i in 0..nu {
            for j in 0..nv {
                expected.push((
                    [u_breaks[i], v_breaks[j]],
                    [u_breaks[i + 1], v_breaks[j + 1]],
                ));
            }
        }
        expected.sort_by(|a, b| {
            a.0[0]
                .partial_cmp(&b.0[0])
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(
                    a.0[1]
                        .partial_cmp(&b.0[1])
                        .unwrap_or(std::cmp::Ordering::Equal),
                )
        });

        assert_eq!(
            actual, expected,
            "{label}: the extracted domains must tile the unique-knot grid exactly"
        );

        for patch in &patches {
            assert_eq!(
                patch.degree(),
                expected_degree,
                "{label}: the per-span bidegree is the face's bidegree"
            );
            let dom = patch.domain();
            assert!(
                dom.lo[0] <= dom.hi[0] && dom.lo[1] <= dom.hi[1],
                "{label}: an ordered source-domain box"
            );
            assert_eq!(
                patch.parent().face,
                0,
                "{label}: single-face extraction names face ordinal 0"
            );
        }
    }
}

#[test]
fn weight_brackets_certified_positive_or_refused() {
    // Positive-weight faces: every extracted patch carries a certified strictly
    // positive bracket that is the exact axis hull of its weight coefficients
    // and that encloses sampled values of the weight field over the span.
    let positive_fixtures: Vec<(BSplineSurface<Vector4>, &str)> = vec![
        (bilinear_spline_face(), "bilinear"),
        (biquadratic_spline_face(), "biquadratic"),
        (sphere_octant_face(), "octant"),
        (weighted_rational_spline_face(), "weighted rational"),
    ];
    for (face, label) in positive_fixtures {
        let patches = match extract_patches(&face) {
            Ok(patches) => patches,
            Err(refusal) => panic!("{label}: extraction refused: {refusal:?}"),
        };
        assert!(!patches.is_empty(), "{label}: at least one span");
        for (idx, patch) in patches.iter().enumerate() {
            let bracket = patch.weight_bracket();
            assert!(
                bracket.lo > 0.0 && bracket.lo <= bracket.hi,
                "{label} patch {idx}: the certified bracket must be strictly positive, \
                 got [{}, {}]",
                bracket.lo,
                bracket.hi
            );
            // The bracket is the exact Bernstein hull (axis hull) of the
            // weight coefficients: recomputing it reproduces the bracket.
            let mut w_lo = f64::INFINITY;
            let mut w_hi = f64::NEG_INFINITY;
            for row in patch.weights() {
                for &w in row {
                    if w < w_lo {
                        w_lo = w;
                    }
                    if w > w_hi {
                        w_hi = w;
                    }
                }
            }
            assert_eq!(
                bracket.lo, w_lo,
                "{label} patch {idx}: bracket lower bound equals the hull minimum"
            );
            assert_eq!(
                bracket.hi, w_hi,
                "{label} patch {idx}: bracket upper bound equals the hull maximum"
            );
            for s in [0.0f64, 0.25, 0.5, 0.75, 1.0] {
                for t in [0.0f64, 0.25, 0.5, 0.75, 1.0] {
                    let sample = eval_scalar_net(patch.weights(), s, t);
                    assert!(
                        bracket.lo <= sample && sample <= bracket.hi,
                        "{label} patch {idx}: weight sample {sample} at ({s}, {t}) escaped \
                         the certified bracket [{}, {}]",
                        bracket.lo,
                        bracket.hi
                    );
                }
            }
        }
    }

    // A face whose weight field cannot be certified positive (a negative
    // homogeneous weight at a corner) REFUSES extraction typed — never a
    // silent non-positive weight in a delivered patch.
    let refusing = non_positive_weight_face();
    match extract_patches(&refusing) {
        Ok(patches) => panic!(
            "a non-positive weight face must refuse extraction, got {} patches",
            patches.len()
        ),
        Err(refusal) => assert_eq!(
            refusal,
            ConstructRefusal::InvalidInput,
            "a non-certifiable weight bracket refuses InvalidInput, never a silent \
             non-positive weight"
        ),
    }
}
