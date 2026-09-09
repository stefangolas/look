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

//! The polynomial normal numerator and the hemisphere certificate
//! (ADM-L3-NORMALCONE, lemma 3): pure functions over the frozen patch type
//! [`TensorBernsteinPatch`](crate::construct::patches::TensorBernsteinPatch).
//!
//! This packet proves, IN ADVANCE of Theorem B1's computational layer, the
//! exact polynomial machinery that ADM-002-CERTIFICATES' production will drive:
//!
//! 1. **Derivative operators on tensor-Bernstein nets** — the Bernstein
//!    derivative: the coefficients transform by `c′ᵢ = d·(cᵢ₊₁ − cᵢ)` and the
//!    degree is retained one lower on the differentiated axis. Machine-checked
//!    against known-derivative fixtures in the in-module test kit.
//! 2. **The normal numerator assembly** — `M = W(Aᵤ×Aᵥ) − Wᵥ(Aᵤ×A) −
//!    Wᵤ(A×Aᵥ)` from the frozen patch data (the hand-verified identity
//!    `Xᵤ×Xᵥ = M/W³`; the conformance test re-verifies it per fixture by exact
//!    evaluation against a direct rational-derivative computation).
//! 3. **The hemisphere certificate (Theorem B1)** — a dyadic direction `c`
//!    taken from the float midpoint normal (SFC: the float choice searches;
//!    the certificate is the exact outward-rounded interval evaluation), with
//!    `min(bernstein coefficients of c·M) > 0` over the box ⇔ regular on ALL
//!    of the box (the convex-hull property of the Bernstein basis: the scalar
//!    `c·M` is represented by the coefficient-wise dots `c·Mᵢⱼ`, so a strictly
//!    positive coefficient min certifies the whole field strictly positive).
//!    Failure ⇒ subdivide — the caller owns subdivision; this module returns
//!    the verdict AND the coefficient net (the caller sees how far the margin
//!    fell and where the worst coefficient sits).
//! 4. **Normal-cone extraction** — per box, the anchored cone bracketing the
//!    normal directions over the box, computed from `M`'s Bernstein hull over
//!    that box (Theorem C's exact input, shared with FSSI-001): a certified
//!    upper bound `s_up` of `sin ∠(n, anchor)` for every normal direction `n`
//!    in the box, in the shape of the landed
//!    [`NormalCone`](crate::construct::admission::NormalCone) carrier. The
//!    anchor is the normalized `M`-value at the box centre (the float
//!    midpoint normal of that box, a pure search); the certificate is the
//!    outward-rounded hull computation — never a sampled normal. The hull over
//!    a box is an interval de Casteljau enclosure, so it tightens as the box
//!    shrinks: a box whose hull cannot certify a cone (`s_up ≥ 1`) is
//!    subdivided by the caller.
//!
//! **Boundary of the module.** The ADM-SHIM frozen kernels in
//! [`patches`](crate::construct::patches) remain REFUSING — their bodies land
//! with ADM-002. The frozen [`patches::MPolynomial`](crate::construct::patches::MPolynomial)
//! is not constructible outside its owning module (private fields, no
//! constructor), so the assembly here returns this module's own
//! [`NormalNumerator`] data record with the identical shape (`degree` plus a
//! row-major flat `R³` coefficient grid, width `dv + 1`): ADM-002 converts it
//! into the landed carrier without a re-derivation. No kernel body, no corpus
//! contact, no boolean-boundary change, and no editing of the shim's landed
//! types land here.
//!
//! **House rules.** **H-1.** This module carries no `unwrap`, no `expect`, and
//! no `panic!`, and adds no module-level `allow`. **H-3.** No bare absolute
//! literals: every certified bound in the shipped surface is named and
//! documented; the mid-square parameter is [`PATCH_MIDPOINT`]. The
//! subdivision-stall record (a dyadic `c` that cannot certify even after the
//! caller's subdivision) is the CALLER's decision (deeper budget or typed
//! refusal) — never silent here, and never fabricated by sampling: no sampled
//! normal appears in any certificate.

use crate::construct::admission::NormalCone;
use crate::construct::patches::TensorBernsteinPatch;
use crate::construct::refusal::ConstructRefusal;
use crate::construct::Interval;
use crate::hull::hull_bernstein_2d;

/// The mid-square parameter of a re-parameterized `[0, 1]²` patch span: every
/// float search in this module evaluates at the span's parameter midpoint.
const PATCH_MIDPOINT: f64 = 0.5;

/// The assembled `R³`-valued polynomial normal numerator `M` of one patch
/// (Lemma 2 data).
///
/// `M = W(Aᵤ×Aᵥ) − Wᵥ(Aᵤ×A) − Wᵤ(A×Aᵥ)` satisfies `Xᵤ×Xᵥ = M/W³`, so the
/// regularity of the rational patch is a POLYNOMIAL question about `M`. The
/// assembly data is one tensor-Bernstein polynomial of bidegree `degree` over
/// the patch's unit square: `coeffs` holds the `(du+1)×(dv+1)` grid of `R³`
/// coefficients in row-major order (rows over `u`, columns over `v`, width
/// `dv + 1`), the same basis convention as the frozen
/// [`TensorBernsteinPatch`](crate::construct::patches::TensorBernsteinPatch),
/// so the Bernstein coefficients of the scalar `c·M` are the component-wise
/// dots of `c` with the grid — exactly the data the Theorem B1 hemisphere
/// certificate needs, with no basis conversion.
#[derive(Debug, Clone, PartialEq)]
pub struct NormalNumerator {
    /// The tensor-Bernstein bidegree `(du, dv)` of the assembly data.
    degree: (usize, usize),
    /// The row-major `(du+1)×(dv+1)` coefficient grid of `M`.
    coeffs: Vec<[f64; 3]>,
}

impl NormalNumerator {
    /// The tensor-Bernstein bidegree of the assembly data.
    pub fn degree(&self) -> (usize, usize) {
        self.degree
    }

    /// The row-major coefficient grid of `M` (`width = dv + 1`).
    pub fn coeffs(&self) -> &[[f64; 3]] {
        &self.coeffs
    }
}

/// The Theorem B1 hemisphere certificate verdict over one box.
///
/// `direction` is the dyadic direction `c` that was certified against,
/// `coefficients` holds the outward-rounded enclosures of the scalar Bernstein
/// coefficients `c·Mᵢⱼ` in row-major order, and `margin` is a certified
/// enclosure of `min(c·Mᵢⱼ)` over the coefficient net (`margin.lo` a certified
/// lower bound, `margin.hi` a certified upper bound). The certificate is
/// `certified == true` exactly when every coefficient's lower bound is
/// strictly positive — `margin.lo > 0` — which by the convex-hull property
/// certifies `c·M > 0` over the whole box, i.e. regularity of the patch on ALL
/// of the box. A non-certified verdict carries the same data so the caller
/// (who owns subdivision) sees how far the worst coefficient fell.
#[derive(Debug, Clone, PartialEq)]
pub struct HemisphereCertificate {
    /// The dyadic direction `c` the certificate was attempted against.
    pub direction: [f64; 3],
    /// The outward-rounded enclosures of the coefficients `c·Mᵢⱼ`.
    pub coefficients: Vec<Interval>,
    /// A certified enclosure of the minimum Bernstein coefficient of `c·M`.
    pub margin: Interval,
    /// Whether `margin.lo > 0` (the box certifies regular on all of it).
    pub certified: bool,
}

/// Lemma 1: assemble the numerator data of one tensor-Bernstein patch and the
/// five machine-checked derivative/assembly kernels as PURE functions over the
/// frozen patch type.
///
/// The kernel body that ADM-002 lands (producing the frozen
/// [`patches::MPolynomial`](crate::construct::patches::MPolynomial) carrier)
/// is NOT here — this module ships the exact arithmetic the body will call,
/// plus the data record [`NormalNumerator`]. A patch whose bidegree has a zero
/// axis (a net that is constant in one parameter) refuses
/// [`ConstructRefusal::InvalidInput`]: such a patch has a vanishing partial
/// derivative, so no cross-product normal numerator can certify it regular.
pub fn assemble_normal_numerator(
    patch: &TensorBernsteinPatch,
) -> Result<NormalNumerator, ConstructRefusal> {
    let numerator = patch.numerator();
    let weights = patch.weights();
    let rows = numerator.len();
    if rows == 0 {
        return Err(ConstructRefusal::InvalidInput);
    }
    let cols = numerator[0].len();
    if rows < 2 || cols < 2 {
        return Err(ConstructRefusal::InvalidInput);
    }
    // Both partial derivatives must exist as non-empty nets (Lemma 1): the
    // u-derivative lowers the u-degree by one, the v-derivative the v-degree.
    let au = derivative_u_vector(numerator);
    let av = derivative_v_vector(numerator);
    let wu = derivative_u_scalar(weights);
    let wv = derivative_v_scalar(weights);

    // M = W(Aᵤ×Aᵥ) − Wᵥ(Aᵤ×A) − Wᵤ(A×Aᵥ). All three terms are exact
    // degree-grown Bernstein products of the same final bidegree
    // `(3m−1, 3n−1)`; no sampled value appears anywhere in the assembly.
    let term1 = mul_scalar_vector(weights, &cross_vector_nets(&au, &av));
    let term2 = mul_scalar_vector(&wv, &cross_vector_nets(&au, numerator));
    let term3 = mul_scalar_vector(&wu, &cross_vector_nets(numerator, &av));
    let m = subtract_vector_nets(&subtract_vector_nets(&term1, &term2), &term3);

    let m_rows = m.len();
    let m_cols = m[0].len();
    let mut coeffs = Vec::with_capacity(m_rows * m_cols);
    for row in &m {
        coeffs.extend_from_slice(row);
    }
    Ok(NormalNumerator {
        degree: (m_rows - 1, m_cols - 1),
        coeffs,
    })
}

/// The SFC float search: the normalized direction of `Xᵤ×Xᵥ` at the patch's
/// parameter midpoint, computed in plain `f64` from the frozen net data.
///
/// The search is deliberately a float computation (never a certificate): the
/// candidate `c` it returns is later certified by the exact interval
/// evaluation of [`hemisphere_certificate`]. `None` when the midpoint normal
/// is degenerate (the float search found no usable direction) or not finite.
pub fn midpoint_normal_direction(patch: &TensorBernsteinPatch) -> Option<[f64; 3]> {
    let numerator = patch.numerator();
    let weights = patch.weights();
    if numerator.is_empty() || weights.is_empty() || numerator[0].is_empty() {
        return None;
    }
    let du = numerator.len() - 1;
    let dv = numerator[0].len() - 1;
    let bu = bernstein_weights(du, PATCH_MIDPOINT);
    let bv = bernstein_weights(dv, PATCH_MIDPOINT);
    let dbu = bernstein_derivative_weights(du, PATCH_MIDPOINT);
    let dbv = bernstein_derivative_weights(dv, PATCH_MIDPOINT);

    // Xᵤ = (AᵤW − AWᵤ)/W² and Xᵥ = (AᵥW − AWᵥ)/W², so the normal direction
    // is that of (AᵤW − AWᵤ) × (AᵥW − AWᵥ) up to the positive scale W⁴ — the
    // search drops the (positive, certified) weight division.
    let a = eval_vector_net(numerator, &bu, &bv);
    let w = eval_scalar_net(weights, &bu, &bv);
    let au = eval_vector_partial_u(numerator, &dbu, &bv);
    let av = eval_vector_partial_v(numerator, &bu, &dbv);
    let wu = eval_scalar_partial_u(weights, &dbu, &bv);
    let wv = eval_scalar_partial_v(weights, &bu, &dbv);
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
    let normal = cross(pu, pv);
    let norm = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
    if !norm.is_finite() || norm <= 0.0 {
        return None;
    }
    Some([normal[0] / norm, normal[1] / norm, normal[2] / norm])
}

/// The Theorem B1 hemisphere certificate over a box: certify (or refuse to
/// certify) that the patch is regular on ALL of the box by the strict
/// positivity of `min(bernstein coefficients of c·M)`.
///
/// `direction` is the dyadic float direction `c` (typically the output of
/// [`midpoint_normal_direction`]). The scalar `c·M` shares the Bernstein basis
/// of `M`, so its Bernstein coefficients are exactly the component-wise dots
/// `c·Mᵢⱼ`. Each dot is evaluated as an OUTWARD-ROUNDED
/// [`Interval`](crate::construct::Interval) (the exact interval evaluation —
/// the certificate never samples a normal). If every coefficient's lower bound
/// is strictly positive, the coefficient min is positive and the convex-hull
/// property of the non-negative Bernstein basis certifies `c·M > 0` — and with
/// it `M ≠ 0`, i.e. `Xᵤ×Xᵥ ≠ 0` — over the whole box. Otherwise the box is
/// NOT certified and the caller subdivides (the returned coefficient net shows
/// exactly how far the worst coefficient fell).
pub fn hemisphere_certificate(
    m: &NormalNumerator,
    direction: [f64; 3],
) -> Result<HemisphereCertificate, ConstructRefusal> {
    if m.coeffs.is_empty()
        || direction.iter().any(|c| !c.is_finite())
        || m.coeffs.iter().any(|c| c.iter().any(|k| !k.is_finite()))
    {
        return Err(ConstructRefusal::InvalidInput);
    }
    let mut coefficients = Vec::with_capacity(m.coeffs.len());
    for coeff in &m.coeffs {
        let dot_x = Interval::point(direction[0]).mul(&Interval::point(coeff[0]));
        let dot_y = Interval::point(direction[1]).mul(&Interval::point(coeff[1]));
        let dot_z = Interval::point(direction[2]).mul(&Interval::point(coeff[2]));
        coefficients.push(dot_x.add(&dot_y).add(&dot_z));
    }
    // The certified enclosure of the minimum coefficient: the true minimum of
    // the coefficients is at least the minimum of their lower bounds and at
    // most the minimum of their upper bounds.
    let mut margin = Interval {
        lo: f64::INFINITY,
        hi: f64::INFINITY,
    };
    for coeff in &coefficients {
        if coeff.lo < margin.lo {
            margin.lo = coeff.lo;
        }
        if coeff.hi < margin.hi {
            margin.hi = coeff.hi;
        }
    }
    let certified = margin.lo > 0.0;
    Ok(HemisphereCertificate {
        direction,
        coefficients,
        margin,
        certified,
    })
}

/// Lemma 4: extract the certified anchored normal cone of a box from `M`'s
/// Bernstein hull over that box.
///
/// `box` is a compact sub-rectangle `((u_lo, u_hi), (v_lo, v_hi))` of the
/// patch's unit square. The anchor is the normalized `M`-value at the box
/// centre — the box's float midpoint normal (`M` is a positive scalar multiple
/// of `Xᵤ×Xᵥ`, so its direction IS the normal direction; a pure float search,
/// never a certificate). The per-axis hulls of the assembled `M` over the box
/// (certified by `hull_bernstein_2d`) give a box that provably contains every
/// normal vector `M(u, v)` over the box. The certificate bounds
/// `sin ∠(n, anchor)` over every such `n`:
///
/// - `w_max` is a certified upper bound of `|n × anchor|` over the hull box
///   (each cross-product component is an outward-rounded interval product of
///   the hull axis with the anchor axis);
/// - `n_min` is a certified lower bound of `|n|` over the box (the axis
///   mignitude bound);
/// - `a_lo` is a certified lower bound of `|anchor|`.
///
/// Then `sin ∠(n, anchor) ≤ w_max / (n_min · a_lo)`, and the cone certifies
/// exactly when that certified bound is strictly below `1` (all normals in one
/// open hemisphere around `anchor`). `None` when the box cannot certify such a
/// cone (a box whose normals span past a hemisphere, or a hull too coarse for
/// the box — the caller subdivides, which tightens the hull). The returned
/// shape is the landed [`NormalCone`](crate::construct::admission::NormalCone)
/// carrier (`anchor`, `s_up`). No sampled normal appears in the certificate.
pub fn normal_cone(m: &NormalNumerator, box_: ((f64, f64), (f64, f64))) -> Option<NormalCone> {
    if m.coeffs.is_empty() || !valid_box(box_) {
        return None;
    }
    let ((u_lo, u_hi), (v_lo, v_hi)) = box_;
    // The anchor: the float midpoint normal direction of the box, read off the
    // assembled M at the box centre (M ∥ Xᵤ×Xᵥ up to a positive scalar).
    let anchor = normalize(eval_m_at(m, 0.5 * (u_lo + u_hi), 0.5 * (v_lo + v_hi)))?;
    let hull = hull_box(m, box_).ok()?;
    // |n × anchor| per axis, outward-rounded over the hull box.
    let cx = Interval::point(anchor[1])
        .mul(&hull[2])
        .sub(&Interval::point(anchor[2]).mul(&hull[1]));
    let cy = Interval::point(anchor[2])
        .mul(&hull[0])
        .sub(&Interval::point(anchor[0]).mul(&hull[2]));
    let cz = Interval::point(anchor[0])
        .mul(&hull[1])
        .sub(&Interval::point(anchor[1]).mul(&hull[0]));
    let mut w2 = Interval::point(0.0);
    for iv in [cx, cy, cz] {
        let far = abs_upper(iv);
        w2 = w2.add(&Interval::point(far).mul(&Interval::point(far)));
    }
    let w_max = w2.sqrt()?.hi;
    // The certified lower bound of |n| over the box (the axis mignitude).
    let n_min = mignitude(&hull);
    // The certified lower bound of |anchor|.
    let a_iv = Interval::point(anchor[0]).mul(&Interval::point(anchor[0]));
    let a_iv = a_iv.add(&Interval::point(anchor[1]).mul(&Interval::point(anchor[1])));
    let a_iv = a_iv.add(&Interval::point(anchor[2]).mul(&Interval::point(anchor[2])));
    let a_lo = a_iv.sqrt()?.lo;
    if !w_max.is_finite() || !n_min.is_finite() || !a_lo.is_finite() {
        return None;
    }
    let denom = Interval::point(n_min).mul(&Interval::point(a_lo));
    let s_up = Interval::point(w_max).div(&denom)?.hi;
    if !s_up.is_finite() || s_up >= 1.0 {
        return None;
    }
    Some(NormalCone { anchor, s_up })
}

/// Whether a 2-D box is a compact sub-rectangle of `[0, 1]²`.
fn valid_box(box_: ((f64, f64), (f64, f64))) -> bool {
    let ((u_lo, u_hi), (v_lo, v_hi)) = box_;
    [u_lo, u_hi, v_lo, v_hi].iter().all(|b| b.is_finite())
        && u_lo >= 0.0
        && u_lo <= u_hi
        && u_hi <= 1.0
        && v_lo >= 0.0
        && v_lo <= v_hi
        && v_hi <= 1.0
}

/// Normalize a nonzero vector (the SFC float search; `None` on a degenerate
/// or non-finite vector).
fn normalize(v: [f64; 3]) -> Option<[f64; 3]> {
    if v.iter().any(|c| !c.is_finite()) {
        return None;
    }
    let norm = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if !norm.is_finite() || norm <= 0.0 {
        return None;
    }
    Some([v[0] / norm, v[1] / norm, v[2] / norm])
}

/// A certified upper bound of `|iv|` over the interval (an endpoint bound).
fn abs_upper(iv: Interval) -> f64 {
    iv.lo.abs().max(iv.hi.abs())
}

/// A certified lower bound of the norm of every vector in the box `[iv; 3]`
/// (the mignitude: the axis mignitudes sum in quadrature, outward-rounded).
fn mignitude(box_: &[Interval; 3]) -> f64 {
    let mut acc = Interval::point(0.0);
    for iv in box_ {
        let mig = if iv.lo > 0.0 {
            iv.lo
        } else if iv.hi < 0.0 {
            -iv.hi
        } else {
            0.0
        };
        acc = acc.add(&Interval::point(mig).mul(&Interval::point(mig)));
    }
    match acc.sqrt() {
        Some(norm) => norm.lo,
        None => 0.0,
    }
}

/// The certified Bernstein-hull box of `M` over `box_`: the per-axis hull of
/// the assembled polynomial net (rows over `u`, columns over `v`) restricted
/// to the sub-rectangle.
fn hull_box(
    m: &NormalNumerator,
    box_: ((f64, f64), (f64, f64)),
) -> Result<[Interval; 3], ConstructRefusal> {
    let (du, dv) = m.degree;
    let width = dv + 1;
    let mut axes: [Vec<Vec<f64>>; 3] = [
        vec![vec![0.0; width]; du + 1],
        vec![vec![0.0; width]; du + 1],
        vec![vec![0.0; width]; du + 1],
    ];
    for (row, coeff) in m.coeffs.iter().enumerate() {
        let i = row / width;
        let j = row % width;
        axes[0][i][j] = coeff[0];
        axes[1][i][j] = coeff[1];
        axes[2][i][j] = coeff[2];
    }
    let ((u_lo, u_hi), (v_lo, v_hi)) = box_;
    let mut out = [Interval::point(0.0); 3];
    for (k, grid) in axes.iter().enumerate() {
        let hull = hull_bernstein_2d(grid, (u_lo, u_hi), (v_lo, v_hi))
            .map_err(|_| ConstructRefusal::InvalidInput)?;
        out[k] = hull;
    }
    Ok(out)
}

/// The value of the assembled polynomial numerator `M` at `(u, v)` (plain
/// float evaluation used only by the SFC direction searches).
fn eval_m_at(m: &NormalNumerator, u: f64, v: f64) -> [f64; 3] {
    let (du, dv) = m.degree;
    let width = dv + 1;
    let bu = bernstein_weights(du, u);
    let bv = bernstein_weights(dv, v);
    let mut acc = [0.0; 3];
    for (i, bi) in bu.iter().enumerate() {
        for (j, bj) in bv.iter().enumerate() {
            let coeff = m.coeffs[i * width + j];
            let factor = bi * bj;
            acc[0] += factor * coeff[0];
            acc[1] += factor * coeff[1];
            acc[2] += factor * coeff[2];
        }
    }
    acc
}

/// The degree-`degree` Bernstein basis values `Bᵢ(degree)(t)`.
fn bernstein_weights(degree: usize, t: f64) -> Vec<f64> {
    let mut out = Vec::with_capacity(degree + 1);
    for i in 0..=degree {
        out.push(binomial(degree, i) * t.powi(i as i32) * (1.0 - t).powi((degree - i) as i32));
    }
    out
}

/// The derivative values `d/dt Bᵢ(degree)(t)` of the Bernstein basis: the
/// closed form `degree·(Bᵢ₋₁(degree−1) − Bᵢ(degree−1))` with the two
/// out-of-range terms read as zero.
fn bernstein_derivative_weights(degree: usize, t: f64) -> Vec<f64> {
    let lower = if degree == 0 {
        Vec::new()
    } else {
        bernstein_weights(degree - 1, t)
    };
    let mut out = Vec::with_capacity(degree + 1);
    for i in 0..=degree {
        let mut val = 0.0;
        if i >= 1 {
            val += lower[i - 1];
        }
        if i < degree {
            val -= lower[i];
        }
        out.push(degree as f64 * val);
    }
    out
}

/// The exact integer binomial `C(n, k)` as a float (the fixture degrees stay
/// small, so the product loop is exact-safe).
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

/// The univariate Bernstein degree-elevation-style weight of one term of the
/// exact product `Bᵢ(d1)·Bⱼ(d2) = w·Bᵢ₊ⱼ(d1+d2)`: `C(d1,i)·C(d2,j)/C(d1+d2,i+j)`.
fn bernstein_product_weight(d1: usize, i: usize, d2: usize, j: usize) -> f64 {
    let top = binomial(d1, i) * binomial(d2, j);
    let bottom = binomial(d1 + d2, i + j);
    if bottom == 0.0 {
        0.0
    } else {
        top / bottom
    }
}

/// Extract one component of a vector-valued row-major net as a scalar net.
fn component(g: &[Vec<[f64; 3]>], k: usize) -> Vec<Vec<f64>> {
    g.iter()
        .map(|row| row.iter().map(|c| c[k]).collect())
        .collect()
}

/// The exact degree-grown Bernstein product of two scalar nets.
fn bernstein_mul_scalar(a: &[Vec<f64>], b: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let ra = a.len();
    let ca = a[0].len();
    let rb = b.len();
    let cb = b[0].len();
    let (da, db) = (ra - 1, rb - 1);
    let (dc, dd) = (ca - 1, cb - 1);
    let mut out = vec![vec![0.0; dc + dd + 1]; da + db + 1];
    for i in 0..ra {
        for p in 0..rb {
            let wu = bernstein_product_weight(da, i, db, p);
            for j in 0..ca {
                for q in 0..cb {
                    let wv = bernstein_product_weight(dc, j, dd, q);
                    out[i + p][j + q] += a[i][j] * b[p][q] * wu * wv;
                }
            }
        }
    }
    out
}

/// Element-wise difference of two same-shaped scalar nets.
fn bernstein_sub_scalar(a: &[Vec<f64>], b: &[Vec<f64>]) -> Vec<Vec<f64>> {
    a.iter()
        .zip(b.iter())
        .map(|(ra, rb)| ra.iter().zip(rb.iter()).map(|(x, y)| x - y).collect())
        .collect()
}

/// The exact Bernstein product of a scalar net with each component of a
/// vector net.
fn mul_scalar_vector(w: &[Vec<f64>], v: &[Vec<[f64; 3]>]) -> Vec<Vec<[f64; 3]>> {
    let x = bernstein_mul_scalar(w, &component(v, 0));
    let y = bernstein_mul_scalar(w, &component(v, 1));
    let z = bernstein_mul_scalar(w, &component(v, 2));
    vector_net_from_axes(&x, &y, &z)
}

/// The exact Bernstein cross product `a × b` of two vector nets.
fn cross_vector_nets(a: &[Vec<[f64; 3]>], b: &[Vec<[f64; 3]>]) -> Vec<Vec<[f64; 3]>> {
    let a0 = component(a, 0);
    let a1 = component(a, 1);
    let a2 = component(a, 2);
    let b0 = component(b, 0);
    let b1 = component(b, 1);
    let b2 = component(b, 2);
    let x = bernstein_sub_scalar(
        &bernstein_mul_scalar(&a1, &b2),
        &bernstein_mul_scalar(&a2, &b1),
    );
    let y = bernstein_sub_scalar(
        &bernstein_mul_scalar(&a2, &b0),
        &bernstein_mul_scalar(&a0, &b2),
    );
    let z = bernstein_sub_scalar(
        &bernstein_mul_scalar(&a0, &b1),
        &bernstein_mul_scalar(&a1, &b0),
    );
    vector_net_from_axes(&x, &y, &z)
}

/// Element-wise difference of two same-shaped vector nets.
fn subtract_vector_nets(a: &[Vec<[f64; 3]>], b: &[Vec<[f64; 3]>]) -> Vec<Vec<[f64; 3]>> {
    a.iter()
        .zip(b.iter())
        .map(|(ra, rb)| {
            ra.iter()
                .zip(rb.iter())
                .map(|(x, y)| [x[0] - y[0], x[1] - y[1], x[2] - y[2]])
                .collect()
        })
        .collect()
}

/// Rebuild a vector net from three equal-shaped scalar nets.
fn vector_net_from_axes(x: &[Vec<f64>], y: &[Vec<f64>], z: &[Vec<f64>]) -> Vec<Vec<[f64; 3]>> {
    x.iter()
        .zip(y.iter())
        .zip(z.iter())
        .map(|((rx, ry), rz)| {
            rx.iter()
                .zip(ry.iter())
                .zip(rz.iter())
                .map(|((&cx, &cy), &cz)| [cx, cy, cz])
                .collect()
        })
        .collect()
}

/// Lemma 1 kernel: the Bernstein u-derivative of a scalar net (degree lowered
/// by one on the row axis; caller guarantees at least two rows).
fn derivative_u_scalar(g: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let degree = g.len() - 1;
    let scale = degree as f64;
    g.windows(2)
        .map(|pair| {
            pair[0]
                .iter()
                .zip(pair[1].iter())
                .map(|(lo, hi)| scale * (hi - lo))
                .collect()
        })
        .collect()
}

/// Lemma 1 kernel: the Bernstein v-derivative of a scalar net (degree lowered
/// by one on the column axis).
fn derivative_v_scalar(g: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let degree = g[0].len() - 1;
    let mut out = Vec::with_capacity(g.len());
    for row in g {
        let mut r = Vec::with_capacity(degree);
        for j in 0..degree {
            r.push(degree as f64 * (row[j + 1] - row[j]));
        }
        out.push(r);
    }
    out
}

/// Lemma 1 kernel: the Bernstein u-derivative of a vector net.
fn derivative_u_vector(g: &[Vec<[f64; 3]>]) -> Vec<Vec<[f64; 3]>> {
    let degree = g.len() - 1;
    let scale = degree as f64;
    g.windows(2)
        .map(|pair| {
            pair[0]
                .iter()
                .zip(pair[1].iter())
                .map(|(lo, hi)| {
                    [
                        scale * (hi[0] - lo[0]),
                        scale * (hi[1] - lo[1]),
                        scale * (hi[2] - lo[2]),
                    ]
                })
                .collect()
        })
        .collect()
}

/// Lemma 1 kernel: the Bernstein v-derivative of a vector net.
fn derivative_v_vector(g: &[Vec<[f64; 3]>]) -> Vec<Vec<[f64; 3]>> {
    let degree = g[0].len() - 1;
    let mut out = Vec::with_capacity(g.len());
    for row in g {
        let mut r = Vec::with_capacity(degree);
        for j in 0..degree {
            let hi = row[j + 1];
            let lo = row[j];
            let scale = degree as f64;
            r.push([
                scale * (hi[0] - lo[0]),
                scale * (hi[1] - lo[1]),
                scale * (hi[2] - lo[2]),
            ]);
        }
        out.push(r);
    }
    out
}

/// The vector cross product of two `R³` floats (the plain arithmetic of the
/// SFC float search; never a certificate).
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Scalar tensor-Bernstein evaluation of a row-major net at a point.
fn eval_scalar_net(g: &[Vec<f64>], bu: &[f64], bv: &[f64]) -> f64 {
    let mut acc = 0.0;
    for (bi, row) in bu.iter().zip(g.iter()) {
        for (bj, c) in bv.iter().zip(row.iter()) {
            acc += bi * bj * c;
        }
    }
    acc
}

/// Vector tensor-Bernstein evaluation of a row-major net at a point.
fn eval_vector_net(g: &[Vec<[f64; 3]>], bu: &[f64], bv: &[f64]) -> [f64; 3] {
    let mut acc = [0.0; 3];
    for (bi, row) in bu.iter().zip(g.iter()) {
        for (bj, c) in bv.iter().zip(row.iter()) {
            acc[0] += bi * bj * c[0];
            acc[1] += bi * bj * c[1];
            acc[2] += bi * bj * c[2];
        }
    }
    acc
}

/// Scalar u-partial tensor-Bernstein evaluation (the u-basis is replaced by
/// its derivative weights).
fn eval_scalar_partial_u(g: &[Vec<f64>], dbu: &[f64], bv: &[f64]) -> f64 {
    let mut acc = 0.0;
    for (di, row) in dbu.iter().zip(g.iter()) {
        for (bj, c) in bv.iter().zip(row.iter()) {
            acc += di * bj * c;
        }
    }
    acc
}

/// Scalar v-partial tensor-Bernstein evaluation (the v-basis is replaced by
/// its derivative weights).
fn eval_scalar_partial_v(g: &[Vec<f64>], bu: &[f64], dbv: &[f64]) -> f64 {
    let mut acc = 0.0;
    for (bi, row) in bu.iter().zip(g.iter()) {
        for (dj, c) in dbv.iter().zip(row.iter()) {
            acc += bi * dj * c;
        }
    }
    acc
}

/// Vector u-partial tensor-Bernstein evaluation.
fn eval_vector_partial_u(g: &[Vec<[f64; 3]>], dbu: &[f64], bv: &[f64]) -> [f64; 3] {
    let mut acc = [0.0; 3];
    for (di, row) in dbu.iter().zip(g.iter()) {
        for (bj, c) in bv.iter().zip(row.iter()) {
            acc[0] += di * bj * c[0];
            acc[1] += di * bj * c[1];
            acc[2] += di * bj * c[2];
        }
    }
    acc
}

/// Vector v-partial tensor-Bernstein evaluation.
fn eval_vector_partial_v(g: &[Vec<[f64; 3]>], bu: &[f64], dbv: &[f64]) -> [f64; 3] {
    let mut acc = [0.0; 3];
    for (bi, row) in bu.iter().zip(g.iter()) {
        for (dj, c) in dbv.iter().zip(row.iter()) {
            acc[0] += bi * dj * c[0];
            acc[1] += bi * dj * c[1];
            acc[2] += bi * dj * c[2];
        }
    }
    acc
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bilinear vector net `(u, v, u·v)` (the fixture-kit closed form).
    fn bilinear_net() -> Vec<Vec<[f64; 3]>> {
        vec![
            vec![[0.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            vec![[1.0, 0.0, 0.0], [1.0, 1.0, 1.0]],
        ]
    }

    #[test]
    fn scalar_u_partial_transforms_like_the_bernstein_derivative() {
        // f(u, v) = u²·v carries the degree-(2, 1) coefficient grid with rows
        // (over u) built from the u² coefficients (0, 0, 1) times the v
        // coefficients (0, 1). Its u-derivative 2u·v has the exact grid
        // [[0,0],[0,2]] in bidegree (1, 1).
        let g = vec![vec![0.0, 0.0], vec![0.0, 0.0], vec![0.0, 1.0]];
        let du = derivative_u_scalar(&g);
        assert_eq!(
            du,
            vec![vec![0.0, 0.0], vec![0.0, 2.0]],
            "d/du(u²v) = 2uv has the exact net [[0,0],[0,2]]"
        );
    }

    #[test]
    fn scalar_v_partial_transforms_like_the_bernstein_derivative() {
        // The same f(u, v) = u²·v: the v-derivative u² has the exact column
        // grid [[0],[0],[1]] in bidegree (2, 0).
        let g = vec![vec![0.0, 0.0], vec![0.0, 0.0], vec![0.0, 1.0]];
        let dv = derivative_v_scalar(&g);
        assert_eq!(
            dv,
            vec![vec![0.0], vec![0.0], vec![1.0]],
            "d/dv(u²v) = u² has the exact grid [[0],[0],[1]]"
        );
    }

    #[test]
    fn bilinear_vector_partials_are_the_known_derivatives() {
        // X(u, v) = (u, v, u·v): Xᵤ = (1, 0, v) and Xᵥ = (0, 1, u). In the
        // retained-degree Bernstein nets those are exactly
        // Aᵤ = [[(1,0,0),(1,0,1)]] (bidegree (0,1)) and
        // Aᵥ = [[(0,1,0)],[(0,1,1)]] (bidegree (1,0)).
        let a = bilinear_net();
        let au = derivative_u_vector(&a);
        assert_eq!(
            au,
            vec![vec![[1.0, 0.0, 0.0], [1.0, 0.0, 1.0]]],
            "the u-partial net of the bilinear patch"
        );
        let av = derivative_v_vector(&a);
        assert_eq!(
            av,
            vec![vec![[0.0, 1.0, 0.0]], vec![[0.0, 1.0, 1.0]]],
            "the v-partial net of the bilinear patch"
        );
    }

    #[test]
    fn bernstein_product_of_nets_evaluates_like_the_function_product() {
        // u as a degree-2 net is (0, 1/2, 1); its square u² as a degree-4 net
        // must evaluate to u² at every point (the degree-grown product is a
        // re-representation of the product function).
        let u: Vec<Vec<f64>> = vec![vec![0.0], vec![0.5], vec![1.0]];
        let square = bernstein_mul_scalar(&u, &u);
        for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let b = bernstein_weights(4, t);
            let value: f64 = b
                .iter()
                .zip(square.iter())
                .map(|(bb, row)| bb * row[0])
                .sum();
            assert!(
                (value - t * t).abs() <= 1.0e-12, // H-3: degree-grown product tracks u² to rounding
                "product net evaluated {value} at u = {t}, expected {}",
                t * t
            );
        }
    }
}
