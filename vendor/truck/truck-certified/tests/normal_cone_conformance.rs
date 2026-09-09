//! ADM-L3-NORMALCONE conformance tests (lemma 3): the polynomial normal
//! numerator `M`, the Theorem B1 hemisphere certificate, and the per-box
//! normal-cone extraction, all as pure functions over the frozen
//! `TensorBernsteinPatch` representation. The test names are the contract:
//!
//! - `normal_numerator_identity_exact`: per fixture, re-verify the identity
//!   `Xᵤ×Xᵥ = M/W³` by exact evaluation against a DIRECT rational-derivative
//!   computation (an independent basis-derivative evaluation — never the
//!   module's own coefficient-transform kernels).
//! - `hemisphere_certificate_passes_regular_fixture`: the cheap B1 test
//!   certifies a genuinely regular fixture on the whole box.
//! - `hemisphere_certificate_refuses_singular_fixture`: a patch with a
//!   vanishing normal numerator (a genuine parameter singularity) is never
//!   certified — no sampled normal is ever used in the certificate.
//! - `cone_extraction_brackets_the_true_normals`: the certified cone
//!   (`anchor`, `s_up`) from `M`'s Bernstein hull brackets the TRUE normal
//!   directions over the box.
//!
//! Fixtures: the shared ADM-SHIM fixture kit (`tests/patch_fixtures.rs`).
//! No solver is called anywhere; every ground truth below is machine-checked
//! by direct evaluation.

#![deny(clippy::unwrap_used)]

#[path = "patch_fixtures.rs"]
mod patch_fixtures;

use truck_certified::construct::normal_cone::{
    assemble_normal_numerator, hemisphere_certificate, midpoint_normal_direction, normal_cone,
    NormalNumerator,
};
use truck_certified::construct::patches::{PatchParent, TensorBernsteinPatch};
use truck_certified::kernel::patch::IBox2;

/// The unit square `[0, 1]²` domain box of every local patch fixture.
const UNIT_DOMAIN: IBox2 = IBox2 {
    lo: [0.0, 0.0],
    hi: [1.0, 1.0],
};

/// The absolute agreement tolerance of the identity re-verification: the two
/// sides of `Xᵤ×Xᵥ = M/W³` are evaluated through independent float paths and
/// must agree to rounding (the fixture data is unit-scale and well
/// conditioned away from the singular samples). Named, never a bare literal.
const IDENTITY_TOL: f64 = 1.0e-8;

/// The angular slack of the sampled-normal bracket check: the certified cone
/// bound `s_up` must dominate every sampled `sin ∠(n, anchor)`. Named.
const ANGLE_SLACK: f64 = 1.0e-12;

/// The interior sample grid of the identity re-verification (away from the
/// intentional boundary collapses of the cone fixture and the pinch corner of
/// the singular fixture).
const IDENTITY_PARAMS: [f64; 4] = [0.2, 0.4, 0.6, 0.8];

/// The closed-box sample grid of the cone bracket check (corners included:
/// the certified cone must bracket the normal at the boundary too).
const CONE_PARAMS: [f64; 5] = [0.0, 0.25, 0.5, 0.75, 1.0];

/// The cone certificate is per BOX: the interval-hull enclosure tightens as
/// the box shrinks, so the certificate is exercised on the compact
/// sub-rectangle `[0.4, 0.6]²` of the bilinear patch (a box whose normals lie
/// in one tight hemisphere).
const CONE_BOX: ((f64, f64), (f64, f64)) = ((0.4, 0.6), (0.4, 0.6));

/// The sample lattice covering the cone box (its edges included).
const CONE_BOX_PARAMS: [f64; 5] = [0.4, 0.45, 0.5, 0.55, 0.6];

/// The exact integer binomial `C(n, k)` for the fixture degrees.
fn binomial(n: usize, k: usize) -> f64 {
    if k > n {
        return 0.0;
    }
    let k = k.min(n - k);
    let mut r = 1.0f64;
    for t in 1..=k {
        r = r * (n - k + t) as f64 / t as f64;
    }
    r
}

/// The degree-`degree` Bernstein basis values `Bᵢ(degree)(t)`.
fn bernstein_basis(degree: usize, t: f64) -> Vec<f64> {
    let mut out = Vec::with_capacity(degree + 1);
    for i in 0..=degree {
        out.push(binomial(degree, i) * t.powi(i as i32) * (1.0 - t).powi((degree - i) as i32));
    }
    out
}

/// The derivative values `d/dt Bᵢ(degree)(t)` computed analytically from the
/// monomial form (an independent evaluation path from the module's nets).
fn bernstein_derivative_basis(degree: usize, t: f64) -> Vec<f64> {
    let mut out = Vec::with_capacity(degree + 1);
    for i in 0..=degree {
        let mut val = 0.0;
        if i >= 1 {
            val += i as f64 * t.powi(i as i32 - 1) * (1.0 - t).powi((degree - i) as i32);
        }
        if i < degree {
            val -= (degree - i) as f64 * t.powi(i as i32) * (1.0 - t).powi((degree - i) as i32 - 1);
        }
        out.push(binomial(degree, i) * val);
    }
    out
}

/// Scalar tensor-Bernstein evaluation of a row-major grid at `(u, v)`.
fn eval_scalar_net(grid: &[Vec<f64>], u: f64, v: f64) -> f64 {
    let bu = bernstein_basis(grid.len() - 1, u);
    let bv = bernstein_basis(grid[0].len() - 1, v);
    let mut acc = 0.0;
    for (bi, row) in bu.iter().zip(grid.iter()) {
        for (bj, c) in bv.iter().zip(row.iter()) {
            acc += bi * bj * c;
        }
    }
    acc
}

/// Vector tensor-Bernstein evaluation of a row-major `R³` grid at `(u, v)`.
fn eval_vector_net(grid: &[Vec<[f64; 3]>], u: f64, v: f64) -> [f64; 3] {
    let bu = bernstein_basis(grid.len() - 1, u);
    let bv = bernstein_basis(grid[0].len() - 1, v);
    let mut acc = [0.0; 3];
    for (bi, row) in bu.iter().zip(grid.iter()) {
        for (bj, c) in bv.iter().zip(row.iter()) {
            acc[0] += bi * bj * c[0];
            acc[1] += bi * bj * c[1];
            acc[2] += bi * bj * c[2];
        }
    }
    acc
}

/// Scalar u-partial evaluation of a row-major grid at `(u, v)`.
fn eval_scalar_partial_u(grid: &[Vec<f64>], u: f64, v: f64) -> f64 {
    let dbu = bernstein_derivative_basis(grid.len() - 1, u);
    let bv = bernstein_basis(grid[0].len() - 1, v);
    let mut acc = 0.0;
    for (di, row) in dbu.iter().zip(grid.iter()) {
        for (bj, c) in bv.iter().zip(row.iter()) {
            acc += di * bj * c;
        }
    }
    acc
}

/// Scalar v-partial evaluation of a row-major grid at `(u, v)`.
fn eval_scalar_partial_v(grid: &[Vec<f64>], u: f64, v: f64) -> f64 {
    let bu = bernstein_basis(grid.len() - 1, u);
    let dbv = bernstein_derivative_basis(grid[0].len() - 1, v);
    let mut acc = 0.0;
    for (bi, row) in bu.iter().zip(grid.iter()) {
        for (dj, c) in dbv.iter().zip(row.iter()) {
            acc += bi * dj * c;
        }
    }
    acc
}

/// Vector u-partial evaluation of a row-major `R³` grid at `(u, v)`.
fn eval_vector_partial_u(grid: &[Vec<[f64; 3]>], u: f64, v: f64) -> [f64; 3] {
    let dbu = bernstein_derivative_basis(grid.len() - 1, u);
    let bv = bernstein_basis(grid[0].len() - 1, v);
    let mut acc = [0.0; 3];
    for (di, row) in dbu.iter().zip(grid.iter()) {
        for (bj, c) in bv.iter().zip(row.iter()) {
            acc[0] += di * bj * c[0];
            acc[1] += di * bj * c[1];
            acc[2] += di * bj * c[2];
        }
    }
    acc
}

/// Vector v-partial evaluation of a row-major `R³` grid at `(u, v)`.
fn eval_vector_partial_v(grid: &[Vec<[f64; 3]>], u: f64, v: f64) -> [f64; 3] {
    let bu = bernstein_basis(grid.len() - 1, u);
    let dbv = bernstein_derivative_basis(grid[0].len() - 1, v);
    let mut acc = [0.0; 3];
    for (bi, row) in bu.iter().zip(grid.iter()) {
        for (dj, c) in dbv.iter().zip(row.iter()) {
            acc[0] += bi * dj * c[0];
            acc[1] += bi * dj * c[1];
            acc[2] += bi * dj * c[2];
        }
    }
    acc
}

/// The vector cross product of two `R³` floats.
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// The DIRECT rational-derivative data of a patch at `(u, v)`: the numerator
/// `(AᵤW − AWᵤ) × (AᵥW − AWᵥ)` of `Xᵤ×Xᵥ = (·)/W⁴` together with the weight
/// value `W`. No module kernel is used: the partials come from an independent
/// analytic basis-derivative evaluation.
fn direct_rational_cross(patch: &TensorBernsteinPatch, u: f64, v: f64) -> ([f64; 3], f64) {
    let numerator = patch.numerator();
    let weights = patch.weights();
    let w = eval_scalar_net(weights, u, v);
    let a = eval_vector_net(numerator, u, v);
    let au = eval_vector_partial_u(numerator, u, v);
    let av = eval_vector_partial_v(numerator, u, v);
    let wu = eval_scalar_partial_u(weights, u, v);
    let wv = eval_scalar_partial_v(weights, u, v);
    let pu = [
        au[0] * w - a[0] * wu,
        au[1] * w - a[1] * wu,
        au[2] * w - a[2] * wu,
    ];
    let pv = [
        av[0] * w - a[0] * wv,
        av[1] * w - a[1] * wv,
        av[2] * w - a[2] * wv,
    ];
    (cross(pu, pv), w)
}

/// The dehomogenized unit normal direction `Xᵤ×Xᵥ / |Xᵤ×Xᵥ|` of a patch at
/// `(u, v)`, computed through the direct rational derivative (a test-time
/// ground truth, never a certificate).
fn true_normal_direction(patch: &TensorBernsteinPatch, u: f64, v: f64) -> [f64; 3] {
    let (raw, _) = direct_rational_cross(patch, u, v);
    let norm = (raw[0] * raw[0] + raw[1] * raw[1] + raw[2] * raw[2]).sqrt();
    [raw[0] / norm, raw[1] / norm, raw[2] / norm]
}

/// The value of the assembled polynomial numerator `M` at `(u, v)`.
fn eval_m(m: &NormalNumerator, u: f64, v: f64) -> [f64; 3] {
    let (du, dv) = m.degree();
    let width = dv + 1;
    let bu = bernstein_basis(du, u);
    let bv = bernstein_basis(dv, v);
    let mut acc = [0.0; 3];
    for i in 0..=du {
        for j in 0..=dv {
            let coeff = m.coeffs()[i * width + j];
            let factor = bu[i] * bv[j];
            acc[0] += factor * coeff[0];
            acc[1] += factor * coeff[1];
            acc[2] += factor * coeff[2];
        }
    }
    acc
}

/// The genuine-parameter-singularity fixture: the polynomial bidegree-`(2,2)`
/// patch `X(u, v) = (u², v², u·v)` with all weights `1`. Its normal numerator
/// vanishes at the `(u, v) = (0, 0)` corner (`Aᵤ = Aᵥ = 0` there), so no
/// dyadic hemisphere direction can certify the whole box: `X(0,0)` is a pinch,
/// not an intentional collapsed edge.
fn singular_pinch_patch() -> TensorBernsteinPatch {
    // The numerator net (rows over u, columns over v) of X = (u², v², u·v):
    // u² carries the u-coefficients (0, 0, 1), v² the v-coefficients (0, 0, 1)
    // and u·v the rank-one product (0, 1/2, 1)⊗(0, 1/2, 1).
    let numerator: Vec<Vec<[f64; 3]>> = vec![
        vec![[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        vec![[0.0, 0.0, 0.0], [0.0, 0.0, 0.25], [0.0, 1.0, 0.5]],
        vec![[1.0, 0.0, 0.0], [1.0, 0.0, 0.5], [1.0, 1.0, 1.0]],
    ];
    let weights: Vec<Vec<f64>> = vec![vec![1.0; 3]; 3];
    match TensorBernsteinPatch::try_new(numerator, weights, UNIT_DOMAIN, PatchParent::new(0, None))
    {
        Ok(patch) => patch,
        Err(refusal) => panic!("the singular pinch fixture refused construction: {refusal:?}"),
    }
}

#[test]
fn normal_numerator_identity_exact() {
    // Per fixture, re-verify the hand-verified identity Xᵤ×Xᵥ = M/W³ by exact
    // evaluation against a DIRECT rational-derivative computation. Writing
    // Xᵤ = (AᵤW − AWᵤ)/W² and Xᵥ = (AᵥW − AWᵥ)/W² turns the identity into the
    // polynomial equation (AᵤW − AWᵤ) × (AᵥW − AWᵥ) = M·W, whose two sides
    // are evaluated independently at every sample point.
    let bilinear = patch_fixtures::bilinear_patch();
    let octant = patch_fixtures::rational_quadratic_volume_fixture();
    let collapsed = patch_fixtures::collapsed_edge_fixture();
    let patches: [(&str, TensorBernsteinPatch); 4] = [
        ("bilinear graph", bilinear),
        ("rational sphere octant", octant.patch),
        ("collapsed-edge quarter cone", collapsed.patch),
        ("singular pinch", singular_pinch_patch()),
    ];
    for (name, patch) in patches {
        let m = assemble_normal_numerator(&patch)
            .expect("the fixture patch assembles a normal numerator");
        for &u in &IDENTITY_PARAMS {
            for &v in &IDENTITY_PARAMS {
                let (direct, w) = direct_rational_cross(&patch, u, v);
                let m_val = eval_m(&m, u, v);
                for k in 0..3 {
                    let expected = m_val[k] * w;
                    let scale = 1.0 + direct[k].abs();
                    assert!(
                        (direct[k] - expected).abs() <= IDENTITY_TOL * scale,
                        "{name} at ({u}, {v}): (AᵤW−AWᵤ)×(AᵥW−AWᵥ)[{k}] = {} diverged \
                         from (M·W)[{k}] = {} by {}",
                        direct[k],
                        expected,
                        (direct[k] - expected).abs()
                    );
                }
            }
        }
    }
}

#[test]
fn hemisphere_certificate_passes_regular_fixture() {
    // The bilinear graph X(u, v) = (u, v, u·v) is regular on ALL of its box:
    // Xᵤ×Xᵥ = (−v, −u, 1) never vanishes. The dyadic float direction from its
    // midpoint normal must certify the whole box — min(bernstein coefficients
    // of c·M) strictly positive — and the coefficient net is returned with the
    // verdict.
    let patch = patch_fixtures::bilinear_patch();
    let m = assemble_normal_numerator(&patch).expect("the bilinear patch assembles");
    let direction =
        midpoint_normal_direction(&patch).expect("the bilinear patch has a finite midpoint normal");
    let certificate = hemisphere_certificate(&m, direction).expect("the certificate builds");
    assert!(
        certificate.certified,
        "the bilinear whole-box certificate must certify, margin = {:?}",
        certificate.margin
    );
    assert!(
        certificate.margin.lo > 0.0,
        "the certified margin is positive"
    );
    for (i, coeff) in certificate.coefficients.iter().enumerate() {
        assert!(
            coeff.lo > 0.0,
            "coefficient {i} of c·M has certified lower bound {} (must be strictly \
             positive for the convex-hull certificate)",
            coeff.lo
        );
    }
    // The convex-hull consequence is observable: c·M > 0 over the whole box,
    // so the sampled true normals never leave the open hemisphere about c.
    for &u in &CONE_PARAMS {
        for &v in &CONE_PARAMS {
            let n = true_normal_direction(&patch, u, v);
            let dot = n[0] * direction[0] + n[1] * direction[1] + n[2] * direction[2];
            assert!(
                dot > 0.0,
                "true normal at ({u}, {v}) stays in the hemisphere about c"
            );
        }
    }
}

#[test]
fn hemisphere_certificate_refuses_singular_fixture() {
    // The pinch patch X(u, v) = (u², v², u·v) has a genuine parameter
    // singularity at (0, 0) — M(0, 0) = 0 is the corner coefficient of the
    // assembled net. No dyadic direction can certify min(bernstein
    // coefficients of c·M) > 0 over a box that contains the pinch, so the
    // certificate must refuse even though the float midpoint normal exists and
    // the patch is regular away from the corner. The refusal is data (a
    // non-positive margin), never a sampled-normal fallback.
    let patch = singular_pinch_patch();
    let m = assemble_normal_numerator(&patch).expect("the pinch patch assembles");
    let direction = midpoint_normal_direction(&patch)
        .expect("the pinch patch has a finite midpoint normal away from the corner");
    let certificate = hemisphere_certificate(&m, direction).expect("the certificate builds");
    assert!(
        !certificate.certified,
        "a box containing the pinch must never certify, margin = {:?}",
        certificate.margin
    );
    assert!(
        certificate.margin.lo <= 0.0,
        "the pinch corner coefficient keeps the certified margin at or below zero"
    );
    // The corner coefficient c·M₀₀ = c·M(0,0) = 0 is the obstruction: it is
    // the first coefficient of the row-major net.
    assert!(
        certificate.coefficients[0].lo <= 0.0 && certificate.coefficients[0].hi >= 0.0,
        "the corner coefficient enclosure straddles zero: {:?}",
        certificate.coefficients[0]
    );
}

#[test]
fn cone_extraction_brackets_the_true_normals() {
    // The per-box normal cone from M's Bernstein hull over the box: on the
    // compact box [0.4, 0.6]² of the bilinear graph the cone certifies, and
    // every TRUE normal direction over the box (sampled at the closed lattice,
    // box edges included) must satisfy sin ∠(n, anchor) ≤ s_up — the exact
    // property Theorem C's transversality gate consumes. The sampling is a
    // TEST-time ground truth; the cone bound itself never samples a normal.
    let patch = patch_fixtures::bilinear_patch();
    let m = assemble_normal_numerator(&patch).expect("the bilinear patch assembles");
    let cone =
        normal_cone(&m, CONE_BOX).expect("the bilinear cone box certifies an anchored normal cone");
    assert!(
        cone.s_up < 1.0,
        "the certified sine bound is strictly below one: {}",
        cone.s_up
    );
    let anchor = cone.anchor;
    let anchor_norm =
        (anchor[0] * anchor[0] + anchor[1] * anchor[1] + anchor[2] * anchor[2]).sqrt();
    assert!(
        (anchor_norm - 1.0).abs() <= 1.0e-12, // H-3: the anchor direction is unit-normalized
        "the cone anchor is a unit direction"
    );
    for &u in &CONE_BOX_PARAMS {
        for &v in &CONE_BOX_PARAMS {
            let n = true_normal_direction(&patch, u, v);
            let sin_angle = (cross(n, anchor)[0].powi(2)
                + cross(n, anchor)[1].powi(2)
                + cross(n, anchor)[2].powi(2))
            .sqrt();
            assert!(
                sin_angle <= cone.s_up + ANGLE_SLACK,
                "true normal at ({u}, {v}) escapes the certified cone: sin ∠ = {sin_angle} \
                 exceeds s_up = {}",
                cone.s_up
            );
            let dot = n[0] * anchor[0] + n[1] * anchor[1] + n[2] * anchor[2];
            assert!(
                dot > 0.0,
                "the normal at ({u}, {v}) stays in the anchored hemisphere"
            );
        }
    }
}
