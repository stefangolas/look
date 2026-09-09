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

//! The collapsed-edge deflation and seam-identification kernels (ADM-L4,
//! Theorem B1's computational layer over the ADM-SHIM frozen patch type).
//!
//! This is the L4 lemma packet of the swept-pair admission wave. It is pure
//! exact algebra over the frozen [`TensorBernsteinPatch`] representation
//! (`patches.rs`, ADM-SHIM) — no corpus contact, no boolean-boundary change,
//! no edit to the shim's landed types. It proves, in advance of the ADM-002
//! assembly, the two Theorem B1 computational lemmas:
//!
//! 1. **Exact factor division.** When a patch boundary is intentionally
//!    collapsed to a point `P`, the polynomial normal numerator
//!    `M = W(A_u×A_v) − W_v(A_u×A) − W_u(A×A_v)` vanishes on the whole
//!    collapsed slice, so the *known* linear boundary factor (`(1−v)`, `v`,
//!    `(1−u)` or `u` per side) divides `M` exactly. The kernel divides the
//!    factor out of the Bernstein coefficient net with a ZERO remainder that
//!    is proven, never assumed: each division step runs only while the
//!    boundary coefficient slice is *exactly* zero (`== 0.0`, the frozen
//!    coefficient arithmetic — never an epsilon), and iterates to the
//!    multiplicity `k` (`M = (boundary factor)^k · M*`). A boundary whose
//!    zero slice cannot be certified exactly refuses typed — a near-division
//!    is never reported as exact.
//! 2. **The deflated certificate.** The hemisphere certificate applied to the
//!    quotient `M*`: `c·M* > 0` over the closed box certifies a regular
//!    interior (`M = (1−v)^k M*`, so for `v < 1` the only rank defect is the
//!    intentional collapse). [`certify_deflated_interior`] returns the
//!    certified coefficient margin when the chosen direction certifies, and
//!    refuses typed otherwise.
//! 3. **Seam identification.** Two patches whose boundary curves coincide
//!    exactly are a paired-BRep-edge fact: after exact degree alignment the
//!    polynomial identity `A₀·W₁ − A₁·W₀ ≡ 0` is evaluated on the coefficient
//!    vector. An exact zero closes the seam ([`seam_identified`] returns
//!    `Ok(true)`); a seam-candidate pair that does not close exactly is a
//!    near-miss and REFUSES typed (near-miss is not identity); a pair that is
//!    not a seam candidate (no boundary edge) is `Ok(false)`.
//!
//! **Exactness discipline.** The deflation path recenters the homogeneous
//! numerator at the certified collapse point `P` (`Ã = A − W·P`, whose
//! collapsed slice is exactly zero), reduces the weight net to its exact
//! minimal degree (dropping bit-identical columns/rows — the Bernstein basis
//! sums to one), assembles `M` at the minimal degrees, and divides. On the
//! closed-form fixtures every exact-zero slice is structural (products of
//! exactly-zero operands, never a large cancellation), so the certified
//! columns are exactly zero in the shipped `f64` arithmetic. All reductions
//! are fixed-order; there is no `unwrap`, no `expect`, no `panic!`, and no
//! module-level `allow`.

use crate::construct::patches::{PatchSide, TensorBernsteinPatch};
use crate::construct::refusal::ConstructRefusal;

/// An `R³`-valued tensor-Bernstein net (row-major, rows over the first
/// parameter, width = second degree + 1).
type VecNet = Vec<Vec<[f64; 3]>>;

/// A scalar tensor-Bernstein net (same layout as [`VecNet`]).
type ScalarNet = Vec<Vec<f64>>;

/// The binomial coefficient `C(n, k)` for the small polynomial degrees of the
/// tensor-Bernstein algebra. Computed by the multiplicative recurrence, which
/// never overflows for the degrees this module carries.
fn binom(n: usize, k: usize) -> f64 {
    let k = k.min(n - k);
    let mut r = 1.0f64;
    for i in 1..=k {
        r *= (n - k + i) as f64 / i as f64;
    }
    r
}

/// Whether a vector-valued net's column `j` is the exact zero vector in every
/// row (the exact zero-slice predicate — never an epsilon).
fn vec_column_is_zero(net: &[Vec<[f64; 3]>], j: usize) -> bool {
    net.iter().all(|row| {
        let e = row[j];
        e[0] == 0.0 && e[1] == 0.0 && e[2] == 0.0
    })
}

/// The exact degree-elevation of a vector-valued net one step in `v`: the
/// degree-`(m, n+1)` Bernstein coefficients of the same polynomial.
fn elevate_v_vec(net: &[Vec<[f64; 3]>]) -> Vec<Vec<[f64; 3]>> {
    let m = net.len();
    let n = net[0].len() - 1;
    let mut out = vec![vec![[0.0, 0.0, 0.0]; n + 2]; m];
    for i in 0..m {
        for j in 0..=n + 1 {
            let scale = j as f64 / (n + 1) as f64;
            if j == 0 {
                out[i][j] = net[i][0];
            } else if j == n + 1 {
                out[i][j] = net[i][n];
            } else {
                let lo = net[i][j - 1];
                let hi = net[i][j];
                out[i][j] = [
                    scale * lo[0] + (1.0 - scale) * hi[0],
                    scale * lo[1] + (1.0 - scale) * hi[1],
                    scale * lo[2] + (1.0 - scale) * hi[2],
                ];
            }
        }
    }
    out
}

/// The exact degree-elevation of a scalar net one step in `v`.
fn elevate_v_scalar(net: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let m = net.len();
    let n = net[0].len() - 1;
    let mut out = vec![vec![0.0; n + 2]; m];
    for i in 0..m {
        for j in 0..=n + 1 {
            let scale = j as f64 / (n + 1) as f64;
            if j == 0 {
                out[i][j] = net[i][0];
            } else if j == n + 1 {
                out[i][j] = net[i][n];
            } else {
                out[i][j] = scale * net[i][j - 1] + (1.0 - scale) * net[i][j];
            }
        }
    }
    out
}

/// Exact minimal-degree reduction of the weight net: while every column is
/// bit-identical the field is independent of `v` (the Bernstein basis sums to
/// one), so the single column is the exact degree-0-in-`v` representation;
/// while every row is bit-identical the field is independent of `u`.
fn reduce_weight_exact(weights: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let mut w = weights.to_vec();
    loop {
        let columns_equal =
            w[0].len() > 1 && (1..w[0].len()).all(|j| (0..w.len()).all(|i| w[i][j] == w[i][0]));
        if !columns_equal {
            break;
        }
        for row in &mut w {
            row.truncate(1);
        }
    }
    if w.len() > 1 && (1..w.len()).all(|i| w[i] == w[0]) {
        w.truncate(1);
    }
    w
}

/// The vector net `∂/∂v` of a bidegree-`(m, n)` net (a bidegree-`(m, n−1)`
/// net; returns an empty-column net when `n = 0`).
fn diff_v_vec(net: &[Vec<[f64; 3]>]) -> Vec<Vec<[f64; 3]>> {
    let n = net[0].len() - 1;
    if n == 0 {
        return Vec::new();
    }
    let scale = n as f64;
    net.iter()
        .map(|row| {
            (0..n)
                .map(|j| {
                    let lo = row[j];
                    let hi = row[j + 1];
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

/// The vector net `∂/∂u` of a bidegree-`(m, n)` net (a bidegree-`(m−1, n)`
/// net; returns an empty net when `m = 0`).
fn diff_u_vec(net: &[Vec<[f64; 3]>]) -> Vec<Vec<[f64; 3]>> {
    let m = net.len() - 1;
    if m == 0 {
        return Vec::new();
    }
    let scale = m as f64;
    (0..m)
        .map(|i| {
            net[i]
                .iter()
                .zip(net[i + 1].iter())
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

/// The scalar net `∂/∂v` of a bidegree-`(m, n)` weight net (an empty net when
/// the field is independent of `v`).
fn diff_v_scalar(weights: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = weights[0].len() - 1;
    if n == 0 {
        return Vec::new();
    }
    let scale = n as f64;
    weights
        .iter()
        .map(|row| (0..n).map(|j| scale * (row[j + 1] - row[j])).collect())
        .collect()
}

/// The scalar net `∂/∂u` of a bidegree-`(m, n)` weight net (an empty net when
/// the field is independent of `u`).
fn diff_u_scalar(weights: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let m = weights.len() - 1;
    if m == 0 {
        return Vec::new();
    }
    let scale = m as f64;
    (0..m)
        .map(|i| {
            weights[i]
                .iter()
                .zip(weights[i + 1].iter())
                .map(|(lo, hi)| scale * (hi - lo))
                .collect()
        })
        .collect()
}

/// The exact Bernstein product of two `R³`-valued nets (the convolution of
/// coefficient grids with the binomial product weights, evaluated in a fixed
/// order — the same polynomial the factors multiply to).
fn vec_times_vec_cross(a: &[Vec<[f64; 3]>], b: &[Vec<[f64; 3]>]) -> Vec<Vec<[f64; 3]>> {
    let (ma, na) = (a.len() - 1, a[0].len() - 1);
    let (mb, nb) = (b.len() - 1, b[0].len() - 1);
    let (m, n) = (ma + mb, na + nb);
    let mut out = vec![vec![[0.0, 0.0, 0.0]; n + 1]; m + 1];
    for i1 in 0..=ma {
        for i2 in 0..=mb {
            let wu = binom(ma, i1) * binom(mb, i2) / binom(m, i1 + i2);
            for j1 in 0..=na {
                for j2 in 0..=nb {
                    let wv = binom(na, j1) * binom(nb, j2) / binom(n, j1 + j2);
                    let w = wu * wv;
                    let x = a[i1][j1];
                    let y = b[i2][j2];
                    let o = &mut out[i1 + i2][j1 + j2];
                    o[0] += w * (x[1] * y[2] - x[2] * y[1]);
                    o[1] += w * (x[2] * y[0] - x[0] * y[2]);
                    o[2] += w * (x[0] * y[1] - x[1] * y[0]);
                }
            }
        }
    }
    out
}

/// The exact Bernstein product of a scalar net and an `R³`-valued net.
fn scalar_times_vec(a: &[Vec<f64>], b: &[Vec<[f64; 3]>]) -> Vec<Vec<[f64; 3]>> {
    let (ma, na) = (a.len() - 1, a[0].len() - 1);
    let (mb, nb) = (b.len() - 1, b[0].len() - 1);
    let (m, n) = (ma + mb, na + nb);
    let mut out = vec![vec![[0.0, 0.0, 0.0]; n + 1]; m + 1];
    for i1 in 0..=ma {
        for i2 in 0..=mb {
            let wu = binom(ma, i1) * binom(mb, i2) / binom(m, i1 + i2);
            for j1 in 0..=na {
                for j2 in 0..=nb {
                    let wv = binom(na, j1) * binom(nb, j2) / binom(n, j1 + j2);
                    let w = wu * wv * a[i1][j1];
                    let e = b[i2][j2];
                    let o = &mut out[i1 + i2][j1 + j2];
                    o[0] += w * e[0];
                    o[1] += w * e[1];
                    o[2] += w * e[2];
                }
            }
        }
    }
    out
}

/// Component-wise subtraction `a − b` of two equal-degree vector nets.
fn vec_net_sub(a: &[Vec<[f64; 3]>], b: &[Vec<[f64; 3]>]) -> Vec<Vec<[f64; 3]>> {
    a.iter()
        .zip(b.iter())
        .map(|(ra, rb)| {
            ra.iter()
                .zip(rb.iter())
                .map(|(ea, eb)| [ea[0] - eb[0], ea[1] - eb[1], ea[2] - eb[2]])
                .collect()
        })
        .collect()
}

/// Component-wise scalar-vector subtraction for the recentered numerator:
/// `Ã[i][j] = A[i][j] − W[i][j]·P`.
fn recenter_numerator(
    numerator: &[Vec<[f64; 3]>],
    weights: &[Vec<f64>],
    p: [f64; 3],
) -> Vec<Vec<[f64; 3]>> {
    numerator
        .iter()
        .zip(weights.iter())
        .map(|(row_a, row_w)| {
            row_a
                .iter()
                .zip(row_w.iter())
                .map(|(a, w)| [a[0] - w * p[0], a[1] - w * p[1], a[2] - w * p[2]])
                .collect()
        })
        .collect()
}

/// The polynomial normal numerator `M = W(A_u×A_v) − W_v(A_u×A) − W_u(A×A_v)`
/// of the rational patch data `(A, W)`, assembled at the exact minimal degrees
/// of its operands (`weights` must already be exactly degree-reduced). A term
/// whose multiplier is identically zero (`W` independent of the differentiated
/// variable) is skipped exactly, never assembled as a rounding-zero net.
fn assemble_normal_numerator(
    numerator: &[Vec<[f64; 3]>],
    weights: &[Vec<f64>],
) -> Vec<Vec<[f64; 3]>> {
    let au = diff_u_vec(numerator);
    let av = diff_v_vec(numerator);
    let cross_u_v = vec_times_vec_cross(&au, &av);
    let mut m = scalar_times_vec(weights, &cross_u_v);
    if weights.len() > 1 {
        let wu = diff_u_scalar(weights);
        let cross_a_v = vec_times_vec_cross(numerator, &av);
        let term = scalar_times_vec(&wu, &cross_a_v);
        m = vec_net_sub(&m, &term);
    }
    if weights[0].len() > 1 {
        let wv = diff_v_scalar(weights);
        let cross_u_a = vec_times_vec_cross(&au, numerator);
        let term = scalar_times_vec(&wv, &cross_u_a);
        m = vec_net_sub(&m, &term);
    }
    m
}

/// Divide the linear `v`-boundary factor out of the vector net exactly,
/// iterating while the boundary column is the exact zero slice. `at_max`
/// selects the `v = 1` side (factor `(1−v)`, quotient scaling `n/(n−k)`) or
/// the `v = 0` side (factor `v`, quotient scaling `n/(k+1)`). Returns the
/// multiplicity and the deflated quotient net.
fn deflate_v_boundary(m: &[Vec<[f64; 3]>], at_max: bool) -> (usize, Vec<Vec<[f64; 3]>>) {
    let mut g = m.to_vec();
    let mut multiplicity = 0usize;
    while g[0].len() > 1 {
        let n = g[0].len() - 1;
        let slice = if at_max { n } else { 0 };
        if !vec_column_is_zero(&g, slice) {
            break;
        }
        let scale = n as f64;
        g = g
            .iter()
            .map(|row| {
                if at_max {
                    (0..n)
                        .map(|k| {
                            let c = row[k];
                            let s = scale / (n - k) as f64;
                            [s * c[0], s * c[1], s * c[2]]
                        })
                        .collect()
                } else {
                    (0..n)
                        .map(|k| {
                            let c = row[k + 1];
                            let s = scale / (k + 1) as f64;
                            [s * c[0], s * c[1], s * c[2]]
                        })
                        .collect()
                }
            })
            .collect();
        multiplicity += 1;
    }
    (multiplicity, g)
}

/// The deflation certificate of one boundary side of an extracted patch
/// (Theorem B1, lemma 4): the multiplicity `k` of the divided-out boundary
/// factor and the exact quotient `M*` of the polynomial normal numerator.
#[derive(Debug, Clone, PartialEq)]
pub struct DeflationCertificate {
    /// The boundary side whose collapse was deflated.
    boundary: PatchSide,
    /// The multiplicity `k` of the divided-out boundary factor.
    multiplicity: usize,
    /// The quotient `M*` the hemisphere certificate certifies over the
    /// deflated side.
    quotient: DeflatedNumerator,
}

impl DeflationCertificate {
    /// The boundary side whose collapse was deflated.
    pub fn boundary(&self) -> PatchSide {
        self.boundary
    }

    /// The multiplicity `k` of the divided-out boundary factor.
    pub fn multiplicity(&self) -> usize {
        self.multiplicity
    }

    /// The quotient normal numerator `M*` of the deflation.
    pub fn quotient(&self) -> &DeflatedNumerator {
        &self.quotient
    }
}

/// The deflated quotient normal numerator `M*` as a tensor-Bernstein net:
/// `M = (boundary factor)^k · M*`. The coefficient grid is row-major
/// (`rows` over the first parameter, `width = second degree + 1`), the SAME
/// layout the ADM-SHIM [`MPolynomial`](crate::construct::patches::MPolynomial)
/// freezes, so the ADM-002 assembly can lift the quotient into the frozen
/// carrier without a basis conversion.
#[derive(Debug, Clone, PartialEq)]
pub struct DeflatedNumerator {
    /// The tensor-Bernstein bidegree of the quotient net.
    degree: (usize, usize),
    /// The row-major coefficient grid (`width = dv + 1`).
    coeffs: Vec<[f64; 3]>,
}

impl DeflatedNumerator {
    /// The tensor-Bernstein bidegree of the quotient net.
    pub fn degree(&self) -> (usize, usize) {
        self.degree
    }

    /// The row-major coefficient grid of the quotient net (`width = dv + 1`).
    pub fn coeffs(&self) -> &[[f64; 3]] {
        &self.coeffs
    }

    /// The certified lower margin `min_i (c · coeff_i)` of the scalar
    /// `c·M*`: by the Bernstein convex-hull property this lower bound holds
    /// over the whole closed box. Returns `None` when the direction does not
    /// certify strict positivity.
    pub fn direction_margin(&self, c: [f64; 3]) -> Option<f64> {
        let mut min = f64::INFINITY;
        for e in &self.coeffs {
            let dot = c[0] * e[0] + c[1] * e[1] + c[2] * e[2];
            if dot < min {
                min = dot;
            }
        }
        if min.is_finite() && min > 0.0 {
            Some(min)
        } else {
            None
        }
    }
}

/// Build the certificate for a collapse deflated along the `v` axis of the
/// (row-major, rows over `u`) numerator/weight grids, at the `v = 1` side
/// (`at_max`) or the `v = 0` side. The boundary must collapse to a single
/// point exactly (the recentered slice is the exact zero vector in every row);
/// a boundary that cannot be certified collapsed refuses typed
/// [`ConstructRefusal::InvalidInput`].
fn deflate_along_v(
    numerator: &VecNet,
    weights: &ScalarNet,
    at_max: bool,
) -> Result<(usize, VecNet), ConstructRefusal> {
    let cols = numerator[0].len();
    let slice = if at_max { cols - 1 } else { 0 };
    // The certified collapse point: any control of the collapsed slice
    // dehomogenized (the weight field is strictly positive, so the division
    // is safe); the per-row exactness check below is the certificate.
    let p = {
        let a0 = numerator[0][slice];
        let w0 = weights[0][slice];
        [a0[0] / w0, a0[1] / w0, a0[2] / w0]
    };
    let recentered = recenter_numerator(numerator, weights, p);
    if !vec_column_is_zero(&recentered, slice) {
        return Err(ConstructRefusal::InvalidInput);
    }
    let weights_minimal = reduce_weight_exact(weights);
    let m = assemble_normal_numerator(&recentered, &weights_minimal);
    let (multiplicity, quotient) = deflate_v_boundary(&m, at_max);
    if multiplicity == 0 {
        // An exactly collapsed boundary must carry at least one boundary
        // factor in the normal numerator (the zero slice of M vanishes); a
        // zero multiplicity here is a substrate inconsistency, refused loudly
        // rather than reported as an exact deflation.
        return Err(ConstructRefusal::InvalidInput);
    }
    Ok((multiplicity, quotient))
}

/// Transpose a vector net (rows over `u` become columns over `v`), routing the
/// `u`-axis deflation through the `v`-axis kernel.
fn transpose_vec(net: &[Vec<[f64; 3]>]) -> Vec<Vec<[f64; 3]>> {
    let rows = net.len();
    let cols = net[0].len();
    let mut out = vec![vec![[0.0, 0.0, 0.0]; rows]; cols];
    for i in 0..rows {
        for j in 0..cols {
            out[j][i] = net[i][j];
        }
    }
    out
}

/// Transpose a scalar (weight) net.
fn transpose_scalar(net: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let rows = net.len();
    let cols = net[0].len();
    let mut out = vec![vec![0.0; rows]; cols];
    for i in 0..rows {
        for j in 0..cols {
            out[j][i] = net[i][j];
        }
    }
    out
}

/// Flatten a quotient grid into the row-major [`DeflatedNumerator`] layout.
fn quotient_to_numerator(net: Vec<Vec<[f64; 3]>>) -> DeflatedNumerator {
    let degree = (net.len() - 1, net[0].len() - 1);
    let width = net[0].len();
    let mut coeffs = Vec::with_capacity(net.len() * width);
    for row in &net {
        coeffs.extend_from_slice(row);
    }
    DeflatedNumerator { degree, coeffs }
}

/// The L4 deflation kernel: divide the known collapsed-boundary factor out of
/// a patch's polynomial normal numerator exactly.
///
/// For a boundary side collapsed to a point the normal numerator carries the
/// known linear boundary factor (`(1−v)` at `v = 1`, `v` at `v = 0`, and the
/// `u` analogues); the kernel recenters the homogeneous numerator at the
/// certified collapse point, assembles the normal numerator at minimal degree,
/// and divides the factor out with a PROVEN zero remainder, iterating to the
/// multiplicity `k`. A boundary that does not collapse exactly (the recentered
/// slice is not the exact zero vector) refuses typed
/// [`ConstructRefusal::InvalidInput`] — a near-collapse is never divided as if
/// exact.
pub fn deflate_factor(
    patch: &TensorBernsteinPatch,
    boundary: PatchSide,
) -> Result<DeflationCertificate, ConstructRefusal> {
    let numerator = patch.numerator().to_vec();
    let weights = patch.weights().to_vec();
    let (multiplicity, quotient) = match boundary {
        PatchSide::SideVMax => deflate_along_v(&numerator, &weights, true)?,
        PatchSide::SideVMin => deflate_along_v(&numerator, &weights, false)?,
        PatchSide::SideUMax | PatchSide::SideUMin => {
            let at_max = boundary == PatchSide::SideUMax;
            let (multiplicity, quotient) = deflate_along_v(
                &transpose_vec(&numerator),
                &transpose_scalar(&weights),
                at_max,
            )?;
            (multiplicity, transpose_vec(&quotient))
        }
    };
    Ok(DeflationCertificate {
        boundary,
        multiplicity,
        quotient: quotient_to_numerator(quotient),
    })
}

/// The deflated certificate (Theorem B1, lemma 4): certify the quotient `M*`
/// of a deflation by the hemisphere test — `c·M* > 0` over the closed box
/// certifies a regular interior for `v < 1`, the only rank defect being the
/// intentional collapse.
///
/// Returns the certified lower margin `min_i (c·coeff_i(M*))` when the chosen
/// direction certifies strict positivity (the Bernstein convex-hull property
/// makes the coefficient minimum a certified lower bound over the whole box).
/// A direction that does not certify — a coefficient dot at or below zero —
/// refuses typed [`ConstructRefusal::ConditioningBelowThreshold`]: the caller
/// may retry another direction, and a genuinely singular interior is never
/// reported regular.
pub fn certify_deflated_interior(
    certificate: &DeflationCertificate,
    direction: [f64; 3],
) -> Result<f64, ConstructRefusal> {
    match certificate.quotient().direction_margin(direction) {
        Some(margin) => Ok(margin),
        None => Err(ConstructRefusal::ConditioningBelowThreshold),
    }
}

/// A boundary curve of a patch, read off one side of the rectangular domain:
/// the (vector) numerator controls and the (scalar) weight controls along the
/// boundary parameter.
struct BoundaryCurve {
    /// The numerator control array along the boundary.
    numerator: Vec<[f64; 3]>,
    /// The weight control array along the boundary.
    weights: Vec<f64>,
}

/// Read the boundary curve of a patch side (rows are over `u`, columns over
/// `v`; a `u` side is a row read across `v`, a `v` side is a column read
/// across `u`).
fn boundary_curve(patch: &TensorBernsteinPatch, side: PatchSide) -> BoundaryCurve {
    let numerator = patch.numerator();
    let weights = patch.weights();
    match side {
        PatchSide::SideUMin => BoundaryCurve {
            numerator: numerator[0].clone(),
            weights: weights[0].clone(),
        },
        PatchSide::SideUMax => {
            let i = numerator.len() - 1;
            BoundaryCurve {
                numerator: numerator[i].clone(),
                weights: weights[i].clone(),
            }
        }
        PatchSide::SideVMin => BoundaryCurve {
            numerator: (0..numerator.len()).map(|i| numerator[i][0]).collect(),
            weights: (0..weights.len()).map(|i| weights[i][0]).collect(),
        },
        PatchSide::SideVMax => {
            let j = numerator[0].len() - 1;
            BoundaryCurve {
                numerator: (0..numerator.len()).map(|i| numerator[i][j]).collect(),
                weights: (0..weights.len()).map(|i| weights[i][j]).collect(),
            }
        }
    }
}

/// The exact product `A_a·W_b` of two aligned boundary curves (a vector
/// Bernstein curve and a scalar Bernstein curve of the same degree), in the
/// coefficient vector of the product curve.
fn curve_product(num: &[[f64; 3]], weights: &[f64]) -> Vec<[f64; 3]> {
    let d = num.len() - 1;
    let mut out = vec![[0.0, 0.0, 0.0]; 2 * d + 1];
    for i in 0..=d {
        for j in 0..=d {
            let w = binom(d, i) * binom(d, j) / binom(2 * d, i + j);
            let e = num[i];
            let s = w * weights[j];
            let o = &mut out[i + j];
            o[0] += s * e[0];
            o[1] += s * e[1];
            o[2] += s * e[2];
        }
    }
    out
}

/// Elevate a boundary curve (vector) to a target degree, returning it
/// unchanged when already at the target.
fn elevate_curve_to(
    num: &[Vec<[f64; 3]>],
    weights: &[Vec<f64>],
    target: usize,
) -> (Vec<Vec<[f64; 3]>>, Vec<Vec<f64>>) {
    let mut n = num.to_vec();
    let mut w = weights.to_vec();
    while n[0].len() - 1 < target {
        n = elevate_v_vec(&n);
        w = elevate_v_scalar(&w);
    }
    (n, w)
}

/// The exact seam identity test over one aligned side-pair: `A₀·W₁ − A₁·W₀ ≡
/// 0` on the coefficient vector (the two boundary rational curves coincide).
/// The identity is exact or it is not an identity: the residual coefficients
/// must be the exact zero vector.
fn boundary_identity_is_exact(
    a_num: &[[f64; 3]],
    a_w: &[f64],
    b_num: &[[f64; 3]],
    b_w: &[f64],
) -> bool {
    // Align the two boundary curves to the common degree by exact degree
    // elevation (knot insertion is vacuous for a single extracted span).
    let d_a = a_num.len() - 1;
    let d_b = b_num.len() - 1;
    let target = d_a.max(d_b);
    let (a_num, a_w) = elevate_curve_to(&[a_num.to_vec()], &[a_w.to_vec()], target);
    let (b_num, b_w) = elevate_curve_to(&[b_num.to_vec()], &[b_w.to_vec()], target);
    let p1 = curve_product(&a_num[0], &b_w[0]);
    let p2 = curve_product(&b_num[0], &a_w[0]);
    p1.iter()
        .zip(p2.iter())
        .all(|(x, y)| x[0] == y[0] && x[1] == y[1] && x[2] == y[2])
}

/// Reverse a boundary curve (re-parameterize `s ↦ 1 − s` by reversing the
/// Bernstein coefficient arrays, exact).
fn reverse_curve(curve: &BoundaryCurve) -> BoundaryCurve {
    BoundaryCurve {
        numerator: curve.numerator.iter().rev().copied().collect(),
        weights: curve.weights.iter().rev().copied().collect(),
    }
}

/// The L4 seam-identification kernel: the verdict over an aligned patch pair.
///
/// Two patches whose shared boundary curves coincide EXACTLY
/// (`A₀·W₁ − A₁·W₀ ≡ 0` over aligned degrees) are seam-identified: the shared
/// boundary is a paired-BRep-edge fact, not a singularity, and the kernel
/// returns `Ok(true)`. The identity is exact or it is refused — a seam
/// candidate pair (both patches carry a boundary BRep edge) whose boundaries
/// do not close exactly is a misaligned near-seam and REFUSES typed
/// [`ConstructRefusal::InvalidInput`]; near-miss is never reported as
/// identity. A pair that is not a seam candidate (at least one patch carries
/// no boundary edge) returns `Ok(false)`: there is no seam fact to close.
pub fn seam_identified(
    patch_a: &TensorBernsteinPatch,
    patch_b: &TensorBernsteinPatch,
) -> Result<bool, ConstructRefusal> {
    if patch_a.parent().edge.is_none() || patch_b.parent().edge.is_none() {
        return Ok(false);
    }
    let sides = [
        PatchSide::SideUMin,
        PatchSide::SideUMax,
        PatchSide::SideVMin,
        PatchSide::SideVMax,
    ];
    for side_a in sides {
        let curve_a = boundary_curve(patch_a, side_a);
        for side_b in sides {
            for curve_b in [
                boundary_curve(patch_b, side_b),
                reverse_curve(&boundary_curve(patch_b, side_b)),
            ] {
                if boundary_identity_is_exact(
                    &curve_a.numerator,
                    &curve_a.weights,
                    &curve_b.numerator,
                    &curve_b.weights,
                ) {
                    return Ok(true);
                }
            }
        }
    }
    // A claimed seam (both patches bound the shared edge) that does not close
    // exactly: near-miss is not identity.
    Err(ConstructRefusal::InvalidInput)
}
