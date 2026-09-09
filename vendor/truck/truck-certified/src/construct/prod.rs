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

//! The L2 tensor-Bernstein product lemma (ADM-L2-PRODUCT): the exact
//! degree-grown product of two extracted tensor-Bernstein patches and the
//! exact degree-elevation helper, computed over the frozen
//! [`TensorBernsteinPatch`](super::patches::TensorBernsteinPatch) type.
//!
//! **The lemma.** The product of two tensor-Bernstein patches is the
//! tensor-Bernstein patch of ADDED degree with exactly computable
//! coefficients: the coefficient convolution along each dimension. In the
//! tensor-Bernstein basis `B_i^m·B_j^n = [C(m,i)C(n,j)/C(m+n,i+j)]·B_{i+j}^{m+n}`
//! the product of the `(m,n)` net `a` and the `(p,q)` net `b` has bidegree
//! `(m+p, n+q)` and coefficients
//!
//! `c[k][l] = Σ_{i1+i2=k, j1+j2=l} a[i1][j1]·b[i2][j2] ·
//!            [C(m,i1)C(p,i2)/C(m+p,k)] · [C(n,j1)C(q,j2)/C(n+q,l)]`.
//!
//! `[`patch_product`]` multiplies BOTH fields of the two patches — the `R³`
//! numerator nets componentwise and the scalar weight nets the same way — so
//! the returned rational patch is the pointwise componentwise (Hadamard)
//! product of the two dehomogenized surfaces over their aligned unit square:
//! `patch_product(a, b)(u, v) = a(u, v) ⊙ b(u, v)` for every `(u, v)` in the
//! shared span, exactly. This is the clearing-denominator algebra Theorem A
//! needs: `F = Ŵ_Y·Â − Ŵ_X·B̂` is the difference of two such products (a weight
//! field replicated across the three coordinates times a numerator field), so
//! the L2 kernel is proved here in advance of the `F` assembly.
//!
//! **Exactness (H-6: pure exact algebra — nothing searches, nothing
//! samples).** The kernel never evaluates or samples a surface: it is pure
//! coefficient algebra. Every coefficient is accumulated as an exact
//! [`Expansion`](crate::formal::exact::Expansion) over the `f64` input
//! coefficients — the integer-scaled numerator is a sum of exact products —
//! and only the final division by the (integer) Bernstein denominator rounds
//! once into the representation. No truncated convolution is ever formed; the
//! full degree growth is recorded on the returned patch, never hidden.
//!
//! **Degree elevation.** `[`elevate_degree`]` re-encodes a patch at a target
//! bidegree without changing the field it represents — the elevation
//! identity, machine-checked in the conformance file. Elevation never rounds
//! when the target equals the current degree (the coefficients are copied
//! exactly); otherwise each raised coefficient is the exact integer-scaled
//! sum of the original coefficients divided once by its Bernstein
//! denominator, in [`Expansion`](crate::formal::exact::Expansion) arithmetic.
//!
//! **Zero and annihilator behavior.** Products whose numerator field is the
//! zero field are zero EXACTLY: the integer-scaled sums vanish termwise, so a
//! zero operand yields an exactly-zero product numerator at the recorded
//! grown degree (weights still multiply to a positive field) — the
//! `A·(A_u×A)`-type annihilation algebra of the `F`/`M` assemblies is a
//! coefficient fact here, not a cancellation that sampling could hide.
//!
//! **H-1.** This module carries no `unwrap`, no `expect`, and no `panic!`,
//! and adds no module-level `allow`.
//!
//! **H-3.** No bare absolute literals: every certified bound used here is
//! named and documented.
//!
//! **C9 determinism.** All coefficient reductions run in the fixed row-major
//! (over `u`, then `v`) order of the frozen patch grids — no reordering, no
//! hashing, no tolerance anywhere in the kernel.

use super::patches::TensorBernsteinPatch;
use crate::construct::refusal::ConstructRefusal;
use crate::formal::exact::{CertifiedInterval, Expansion};

/// The exact degree-grown product of two extracted tensor-Bernstein patches
/// (ADM-L2-PRODUCT kernel, L2).
///
/// The product multiplies the two patches' numerator and weight fields on
/// their shared span: the returned [`TensorBernsteinPatch`] carries the
/// componentwise (Hadamard) product of the two `R³` numerator nets and the
/// product of the two scalar weight nets, each at the EXACT grown bidegree
/// `(m_a + m_b, n_a + n_b)` — the coefficient convolution along each
/// dimension, never a sampled or truncated product. The degree growth is
/// recorded ON the returned patch (`[`TensorBernsteinPatch::degree`]`), never
/// hidden: a coefficient grid of shape `(m_a + m_b + 1) × (n_a + n_b + 1)`
/// always comes back. The returned patch's dehomogenized surface is the
/// pointwise componentwise product of the two inputs' surfaces over the whole
/// span: `patch_product(a, b)(u, v) = a(u, v) ⊙ b(u, v)` (exact in the
/// coefficient algebra; the conformance file machine-checks the identity at
/// exact parameters per fixture).
///
/// The two patches must be defined over the SAME span: mismatched
/// source-domain boxes refuse typed, never a silent reparameterization. The
/// product patch inherits the first operand's domain and parent identity (the
/// result is an intermediate over `a`'s span lineage, not a face of its own).
///
/// # Errors
///
/// * Patches whose source-domain boxes are not exactly equal refuse
///   [`ConstructRefusal::InvalidInput`] (mismatched spans refuse typed).
/// * Any shape the refusing constructor [`TensorBernsteinPatch::try_new`]
///   rejects (a non-positive product weight field, non-finite data) is
///   propagated as its [`ConstructRefusal`]. The product of two strictly
///   positive weight fields is a strictly positive weight field, so this arm
///   cannot fire on certified inputs.
pub fn patch_product(
    a: &TensorBernsteinPatch,
    b: &TensorBernsteinPatch,
) -> Result<TensorBernsteinPatch, ConstructRefusal> {
    if a.domain() != b.domain() {
        return Err(ConstructRefusal::InvalidInput);
    }

    let an = a.numerator();
    let bn = b.numerator();
    let rows = an.len() + bn.len() - 1;
    let cols = an[0].len() + bn[0].len() - 1;
    let mut numerator: Vec<Vec<[f64; 3]>> = vec![vec![[0.0; 3]; cols]; rows];
    for axis in [0usize, 1, 2] {
        let ga = component_scalar_grid(an, axis);
        let gb = component_scalar_grid(bn, axis);
        let product = bernstein_product_grid_scalar(&ga, &gb);
        for (i, row) in product.iter().enumerate() {
            for (j, value) in row.iter().enumerate() {
                numerator[i][j][axis] = *value;
            }
        }
    }
    let weights = bernstein_product_grid_scalar(a.weights(), b.weights());
    TensorBernsteinPatch::try_new(numerator, weights, a.domain(), a.parent())
}

/// The exact degree-elevation helper (ADM-L2-PRODUCT, L2): re-encode a patch
/// at a target bidegree WITHOUT changing the field it represents.
///
/// Raising a tensor-Bernstein net from bidegree `(m, n)` to a target bidegree
/// `(M, N)` with `M ≥ m`, `N ≥ n` leaves every surface value fixed — the
/// elevation identity, machine-checked in the conformance file. Each raised
/// coefficient is the exact integer-scaled sum
/// `c_k = Σ_i a_i·[C(m,i)C(M−m,k−i)] / C(M,k)` over the fixed index order,
/// accumulated in [`Expansion`](crate::formal::exact::Expansion) arithmetic
/// with a single final rounding into the representation; a target axis equal
/// to the current degree copies that axis's coefficients EXACTLY (no
/// arithmetic, no rounding). The domain and parent identity are preserved, and
/// the raised weight field stays strictly positive, so the result always
/// re-certifies through [`TensorBernsteinPatch::try_new`].
///
/// # Errors
///
/// * A target bidegree below the current bidegree on either axis refuses
///   [`ConstructRefusal::InvalidInput`] (elevation only ever raises).
/// * Any shape the refusing constructor rejects is propagated as its
///   [`ConstructRefusal`].
pub fn elevate_degree(
    patch: &TensorBernsteinPatch,
    target: (usize, usize),
) -> Result<TensorBernsteinPatch, ConstructRefusal> {
    let (m, n) = patch.degree();
    if target.0 < m || target.1 < n {
        return Err(ConstructRefusal::InvalidInput);
    }

    let mut numerator: Vec<Vec<[f64; 3]>> = vec![vec![[0.0; 3]; target.1 + 1]; target.0 + 1];
    for axis in [0usize, 1, 2] {
        let scalar = component_scalar_grid(patch.numerator(), axis);
        let raised = elevate_grid_scalar(&scalar, target.0, target.1);
        for (i, row) in raised.iter().enumerate() {
            for (j, value) in row.iter().enumerate() {
                numerator[i][j][axis] = *value;
            }
        }
    }
    let weights = elevate_grid_scalar(patch.weights(), target.0, target.1);
    TensorBernsteinPatch::try_new(numerator, weights, patch.domain(), patch.parent())
}

// ---------------------------------------------------------------------------
// Exact tensor-Bernstein coefficient algebra (private kernels; C9 fixed order)
// ---------------------------------------------------------------------------

/// The integer binomial `C(n, k)`, for the small degrees the certified kernel
/// admits (exact `usize` arithmetic; the atlas degree bounds keep `C` far
/// inside `usize` range — a blow-up past those bounds is a representation
/// bug, never a silent wrap).
fn binom(n: usize, k: usize) -> usize {
    let k = k.min(n - k);
    let mut result = 1usize;
    for i in 1..=k {
        result = result * (n - k + i) / i;
    }
    result
}

/// An exact single-component [`Expansion`] of the small integer `n`.
fn int_expansion(n: usize) -> Expansion {
    Expansion::zero().grow(n as f64)
}

/// The one `f64` a certified coefficient is stored in: the midpoint of the
/// certified enclosure of the exact accumulated value (the enclosure is
/// degenerate for an exactly-representable value up to the directed-rounding
/// widening; the midpoint carries no accumulation error beyond it).
fn expansion_value(e: &Expansion) -> f64 {
    if e.is_zero() {
        return 0.0;
    }
    let iv = CertifiedInterval::from_expansion(e);
    0.5 * (iv.lo + iv.hi)
}

/// The exact tensor-Bernstein product of two scalar coefficient grids
/// (`rows` over `u`, `columns` over `v`), returned at the grown shape.
///
/// Each output coefficient is accumulated as the exact integer-scaled sum
/// `Σ a[i1][j1]·b[i2][j2]·C(m,i1)C(p,i2)C(n,j1)C(q,j2)` over the fixed
/// row-major index order, then divided once by the axis Bernstein
/// denominators `C(m+p,k)·C(n+q,l)`. A zero factor field yields an exactly
/// zero result grid termwise.
fn bernstein_product_grid_scalar(a: &[Vec<f64>], b: &[Vec<f64>]) -> Vec<Vec<f64>> {
    debug_assert!(a.iter().all(|row| row.len() == a[0].len()));
    debug_assert!(b.iter().all(|row| row.len() == b[0].len()));
    let ma = a.len() - 1;
    let na = a[0].len() - 1;
    let mb = b.len() - 1;
    let nb = b[0].len() - 1;
    let rows = ma + mb + 1;
    let cols = na + nb + 1;

    let mut acc: Vec<Vec<Expansion>> = vec![vec![Expansion::zero(); cols]; rows];
    for i1 in 0..=ma {
        for j1 in 0..=na {
            for i2 in 0..=mb {
                for j2 in 0..=nb {
                    let scale = binom(ma, i1) * binom(mb, i2) * binom(na, j1) * binom(nb, j2);
                    let product = Expansion::from_product(a[i1][j1], b[i2][j2]);
                    let term = product.mul_expansion(&int_expansion(scale));
                    acc[i1 + i2][j1 + j2] = acc[i1 + i2][j1 + j2].merge(&term);
                }
            }
        }
    }

    let mut out = vec![vec![0.0; cols]; rows];
    for (k, row) in out.iter_mut().enumerate() {
        for (l, cell) in row.iter_mut().enumerate() {
            let denominator = (binom(ma + mb, k) * binom(na + nb, l)) as f64;
            *cell = expansion_value(&acc[k][l]) / denominator;
        }
    }
    out
}

/// Raise one scalar coefficient sequence from degree `len − 1` to
/// `to_degree ≥ len − 1`, keeping the represented polynomial identical.
///
/// A target equal to the current degree copies the coefficients EXACTLY (the
/// elevation identity at zero growth: no arithmetic, no rounding).
fn elevate_1d_scalar(coeffs: &[f64], to_degree: usize) -> Vec<f64> {
    let from_degree = coeffs.len() - 1;
    if to_degree == from_degree {
        return coeffs.to_vec();
    }
    let mut out = Vec::with_capacity(to_degree + 1);
    for k in 0..=to_degree {
        let i_lo = k.saturating_sub(to_degree - from_degree);
        let i_hi = from_degree.min(k);
        let mut acc = Expansion::zero();
        for (offset, &coefficient) in coeffs[i_lo..=i_hi].iter().enumerate() {
            let i = i_lo + offset;
            let scale = binom(from_degree, i) * binom(to_degree - from_degree, k - i);
            acc = acc.merge(&Expansion::from_product(coefficient, scale as f64));
        }
        let denominator = binom(to_degree, k) as f64;
        out.push(expansion_value(&acc) / denominator);
    }
    out
}

/// Raise every axis of a scalar coefficient grid to the target bidegree
/// (`u` direction first, then `v` — the fixed C9 order).
fn elevate_grid_scalar(grid: &[Vec<f64>], target_u: usize, target_v: usize) -> Vec<Vec<f64>> {
    let cur_u = grid.len() - 1;
    let cur_v = grid[0].len() - 1;
    let mut after_u: Vec<Vec<f64>> = vec![vec![0.0; cur_v + 1]; target_u + 1];
    for j in 0..=cur_v {
        let column: Vec<f64> = (0..=cur_u).map(|i| grid[i][j]).collect();
        let raised = elevate_1d_scalar(&column, target_u);
        for (i, value) in raised.iter().enumerate() {
            after_u[i][j] = *value;
        }
    }
    let mut out: Vec<Vec<f64>> = Vec::with_capacity(target_u + 1);
    for row in &after_u {
        out.push(elevate_1d_scalar(row, target_v));
    }
    out
}

/// The `axis`-coordinate field of an `R³` coefficient grid, as a scalar grid
/// of the same shape (rows over `u`, columns over `v`).
fn component_scalar_grid(net: &[Vec<[f64; 3]>], axis: usize) -> Vec<Vec<f64>> {
    net.iter()
        .map(|row| row.iter().map(|point| point[axis]).collect())
        .collect()
}
