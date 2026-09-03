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

//! The §6.3 maximal-minor algebra (BG-KV2-301-S03A): Theorem 6.4 as enclosure
//! machinery over a per-box 3x4 Jacobian enclosure.
//!
//! For `DF : R⁴ → R³`, the kernel direction is `m ∈ R⁴` with
//! `m_j = (−1)^j det(DF with column j deleted)` (Theorem 6.4). This module
//! encloses `m` from an interval Jacobian enclosure and exposes the two
//! checkables of Theorem 6.4 — `DF·m = 0` (i) and `a·m = det[DF; aᵀ]` (iii) —
//! plus the Theorem 6.5 route `a = (d·S¹_u, d·S¹_v, 0, 0) ⇒ a·m = d·w`.
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module. The module carries no `unwrap`, no `expect`, and no `panic!`
//! calls, and adds no module-level `allow`.
//!
//! **N4 / bit-reproducibility.** No transcendental call appears anywhere: the
//! arithmetic is deterministic `CertifiedInterval` sequences only.
//!
//! **det3 discipline.** The 3x3 determinant is the S2A cofactor expansion with
//! the exact engine op order ([`crate::kernel::engine`]'s private `det3_iv` /
//! `det3_f64` and the landed trace's `det3`): `a·(b·c − b'·c') − ...`. No new
//! interval linear algebra is forked; the row/column layout and evaluation
//! order are reused verbatim so an enclosure here is comparable to one the
//! engine would produce.
//!
//! **N7 two-stage rule.** The minor enclosure is the CHEAP interval form; a
//! Bernstein-net form is not built here (recorded in the packet: no predicate
//! in the fixture corpus needed the tight form; escalate per N7 later if a
//! cell stays inconclusive).

use crate::kernel::Interval;

/// Determinant of a 3x3 interval matrix under directed rounding: the same
/// cofactor expansion as the engine's private `det3_iv` (and the landed
/// `det3_f64`), mirrored verbatim so no new interval linear algebra appears.
fn det3_iv(m: &[[Interval; 3]; 3]) -> Interval {
    let a = m[0][0].mul(&m[1][1].mul(&m[2][2]).sub(&m[1][2].mul(&m[2][1])));
    let b = m[0][1].mul(&m[1][0].mul(&m[2][2]).sub(&m[1][2].mul(&m[2][0])));
    let c = m[0][2].mul(&m[1][0].mul(&m[2][1]).sub(&m[1][1].mul(&m[2][0])));
    a.sub(&b).add(&c)
}

/// The certified determinant of the 3x3 submatrix of `jac` on the three given
/// columns (the interval cofactor of one column deletion).
fn minor_of_cols(jac: &[[Interval; 4]; 3], cols: [usize; 3]) -> Interval {
    let m = [
        [jac[0][cols[0]], jac[0][cols[1]], jac[0][cols[2]]],
        [jac[1][cols[0]], jac[1][cols[1]], jac[1][cols[2]]],
        [jac[2][cols[0]], jac[2][cols[1]], jac[2][cols[2]]],
    ];
    det3_iv(&m)
}

/// The maximal-minor enclosure of Theorem 6.4: `m_j = (−1)^j det(DF with
/// column j deleted)` as intervals over the Jacobian enclosure `jac`.
///
/// The Jacobian layout is the product-space one of §6.2: rows are the three
/// spatial components, columns the four product-space directions
/// `(u, v, s, t)` in order. The sign pattern matches the landed float code
/// (`ssi_trace.rs` null_direction and the engine's `kernel_minors`) exactly.
///
/// The result is a single-row grid carrying the four component intervals,
/// `m[0] = (m₀, m₁, m₂, m₃)`.
pub fn minor_vector_encl(jac: &[[Interval; 4]; 3]) -> [[Interval; 4]; 1] {
    // m0 = det(DF with column 0 deleted), m1 = −det(... column 1 deleted), ...
    let d0 = minor_of_cols(jac, [1, 2, 3]);
    let d1 = minor_of_cols(jac, [0, 2, 3]);
    let d2 = minor_of_cols(jac, [0, 1, 3]);
    let d3 = minor_of_cols(jac, [0, 1, 2]);
    // The (−1)^j signs flip the odd-index minors, exactly the landed sign
    // pattern (`ssi_trace.rs` null_direction and the engine's `kernel_minors`).
    [[d0, d1.neg(), d2, d3.neg()]]
}

/// Theorem 6.4(i) as a checkable: the certified component enclosure of
/// `DF·m`. Each of the three output components contains `0` on every box —
/// `DF(x) m(x) = 0` identically in exact arithmetic, and the interval product
/// of the `DF` and `m` enclosures contains that exact value (the
/// `minor_vector_satisfies_df_times_m_is_zero` test asserts exactly this on a
/// grid).
///
/// The result is a single-row grid `out[0] = (r₀, r₁, r₂)` for the three
/// spatial components.
pub fn df_times_m(jac: &[[Interval; 4]; 3], m: &[[Interval; 4]; 1]) -> [[Interval; 3]; 1] {
    let mut out = [[Interval::point(0.0); 3]; 1];
    for (r, out_r) in out[0].iter_mut().enumerate() {
        let mut acc = Interval::point(0.0);
        for k in 0..4 {
            acc = acc.add(&jac[r][k].mul(&m[0][k]));
        }
        *out_r = acc;
    }
    out
}

/// Theorem 6.4(iii) as a checkable: the certified enclosure of `a·m`, which
/// equals `det[DF; aᵀ]` wherever both are evaluated at the same point. With
/// `a = (d·S¹_u, d·S¹_v, 0, 0)` this is the Theorem 6.5 route `d·w` (the
/// `a_dot_m_matches_d_times_w` test).
pub fn a_dot_m(a: [Interval; 4], m: &[[Interval; 4]; 1]) -> Interval {
    let mut acc = Interval::point(0.0);
    for k in 0..4 {
        acc = acc.add(&a[k].mul(&m[0][k]));
    }
    acc
}
