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

//! The certified banded solve (CC-001-BANDED, spine seam S3): the interval
//! fast path of the P1 certified solve.
//!
//! The fast path factorizes a **banded totally-positive collocation matrix**
//! by interval Gaussian elimination WITHOUT pivoting and solves the
//! homogeneous control rows of a loft against the shared factorization. The
//! factorization never sees geometry: the caller builds the row-major band
//! storage from B-spline basis values; this module only eliminates.
//!
//! # Class-specific stability (the de Boor–Pinkus justification)
//!
//! The fast path's stability is CLASS-SPECIFIC: banded totally-positive
//! matrices — all Schoenberg–Whitney collocation matrices — have growth factor
//! exactly 1 under no-pivot Gaussian elimination (de Boor–Pinkus). This is why
//! interval elimination without row exchanges is safe here and would not be in
//! general: for a totally-positive collocation matrix no pivot row exchange is
//! ever needed, the Schur complements stay nonnegative-scaled, and the
//! interval widths never experience pivot growth. The elimination therefore
//! NEVER swaps, never retries with a different order, and never widens: any
//! interval pivot containing `0` is a certified refusal
//! ([`ConstructRefusal::SingularInterpolationSystem`]) because that system has
//! no no-pivot factorization in the banded-TP class.
//!
//! # P1 exact rational path — pre-decided out of v1
//!
//! The exact rational path (theory §1 P1) is PRE-DECIDED OUT of v1:
//! `num-rational` stays out of the manifest. There is no rational-path code
//! and no rational-path test in this packet; the module doc records the
//! decision and the `CC_N_EXACT` constant (`construct::config`) remains
//! reserved for the later amendment that adds it.
//!
//! # Determinism
//!
//! Evaluation order is deterministic everywhere: row-major band iteration,
//! fixed ascending accumulation order in every sum, no parallel reductions.
//!
//! **H-1.** This module carries no `unwrap`, no `expect`, and no `panic!`, and
//! adds no module-level `allow`.

use std::cell::Cell;

use crate::construct::refusal::ConstructRefusal;
use crate::construct::Interval;

/// The certified interval no-pivot factorization of a banded
/// totally-positive collocation matrix (spine seam S3).
///
/// Private band storage of order `n` and half-bandwidth `q`: the no-pivot LU
/// factors live in row-major compact band layout with stride `2q + 1`, entry
/// `(i, j)` stored at offset `j - i + q`. The unit diagonal of `L` is implicit.
/// `last_max_control_width` is the interior cache behind
/// [`BandedFactor::max_control_error`]: it records the maximum enclosure width
/// among the control entries delivered by the most recent
/// [`BandedFactor::solve_homogeneous`] call on this factor (deterministic,
/// one-factor ownership, matching the S3 one-call-per-loft shape).
#[derive(Debug)]
pub struct BandedFactor {
    /// The matrix order `n`.
    order: usize,
    /// The half-bandwidth `q`: the largest structural `|i - j|` with a
    /// nonzero collocation coefficient.
    half_bandwidth: usize,
    /// The row stride `2q + 1` of `band_lu`.
    stride: usize,
    /// The compact row-major band storage of the no-pivot LU factors.
    band_lu: Vec<Interval>,
    /// The interior cache of the last solve's maximum control-entry width.
    last_max_control_width: Cell<f64>,
}

impl BandedFactor {
    /// Read the stored factor entry `(row, col)`.
    fn at(&self, row: usize, col: usize) -> Interval {
        let offset = col + self.half_bandwidth - row;
        self.band_lu[row * self.stride + offset]
    }

    /// Solve all homogeneous control rows of a loft in one call: every
    /// `[Interval; 4]` row of `rhs` is one homogeneous station's four
    /// coordinate channels, and all `n` right-hand sides share this factor.
    ///
    /// Each channel is solved by forward substitution through the implicit-unit
    /// lower factor followed by back-substitution through the upper factor,
    /// both in fixed ascending/descending order. A right-hand side of length
    /// other than the matrix order is [`ConstructRefusal::InvalidInput`]. The
    /// division refusal arm is unreachable after a successful factorization
    /// (every pivot was certified free of `0`); it is returned only as a
    /// defensive, never-panicking path.
    pub fn solve_homogeneous(
        &self,
        rhs: &[[Interval; 4]],
    ) -> Result<Vec<[Interval; 4]>, ConstructRefusal> {
        if rhs.len() != self.order {
            return Err(ConstructRefusal::InvalidInput);
        }
        let mut rows = Vec::with_capacity(self.order);
        for _ in 0..self.order {
            rows.push([Interval { lo: 0.0, hi: 0.0 }; 4]);
        }
        let mut max_width = 0.0_f64;
        for c in 0..4 {
            let mut y = Vec::with_capacity(self.order);
            for i in 0..self.order {
                let mut acc = rhs[i][c];
                let k_lo = i.saturating_sub(self.half_bandwidth);
                for k in k_lo..i {
                    acc = acc.sub(&self.at(i, k).mul(&y[k]));
                }
                y.push(acc);
            }
            for i in (0..self.order).rev() {
                let mut acc = y[i];
                let k_hi = (i + self.half_bandwidth).min(self.order - 1);
                for k in (i + 1)..=k_hi {
                    acc = acc.sub(&self.at(i, k).mul(&rows[k][c]));
                }
                let pivot = self.at(i, i);
                let x_i = acc
                    .div(&pivot)
                    .ok_or(ConstructRefusal::SingularInterpolationSystem)?;
                let w = width_up(&x_i);
                if w > max_width {
                    max_width = w;
                }
                rows[i][c] = x_i;
            }
        }
        self.last_max_control_width.replace(max_width);
        Ok(rows)
    }

    /// The L2 enclosure width `ε`: the maximum enclosure width over the
    /// control entries delivered by the most recent
    /// [`BandedFactor::solve_homogeneous`] call on this factor. Pure and
    /// deterministic for a fixed (factor, input) pair. Returns `0.0` before
    /// any solve has been delivered.
    pub fn max_control_error(&self) -> f64 {
        self.last_max_control_width.get()
    }
}

/// Factor the row-major band storage of a banded totally-positive collocation
/// matrix (spine seam S3).
///
/// `bands` is the row-major storage of a square collocation matrix of order
/// `n` (`bands.len()` must be a perfect square); the caller builds it from
/// B-spline basis values. The structural half-bandwidth `q` is the largest
/// `|i - j|` over coefficients that are not the exact zero point interval;
/// the compact band is extracted and eliminated by interval Gaussian
/// elimination WITHOUT pivoting:
///
/// - any interval pivot containing `0` is a refusal
///   ([`ConstructRefusal::SingularInterpolationSystem`]) — never a swap, never
///   a retry with a different order, never a widening;
/// - the division refusal arm below is unreachable after the pivot check (the
///   only denominators are pivots), returned only as a defensive,
///   never-panicking path;
/// - iteration is row-major with fixed ascending accumulation order.
///
/// A non-square length is [`ConstructRefusal::InvalidInput`].
pub fn factor_banded_tp(bands: &[Interval]) -> Result<BandedFactor, ConstructRefusal> {
    let order = exact_isqrt(bands.len()).ok_or(ConstructRefusal::InvalidInput)?;
    if order == 0 {
        return Err(ConstructRefusal::InvalidInput);
    }

    // The structural half-bandwidth: the largest |i - j| carrying a
    // coefficient that is not the exact-zero point interval. B-spline basis
    // coefficients outside the band are exact zeros by construction.
    let mut q = 0usize;
    for row in 0..order {
        for col in 0..order {
            let entry = bands[row * order + col];
            if !(entry.lo == 0.0 && entry.hi == 0.0) {
                let d = row.abs_diff(col);
                if d > q {
                    q = d;
                }
            }
        }
    }

    let stride = 2 * q + 1;
    let mut band_lu = vec![Interval { lo: 0.0, hi: 0.0 }; order * stride];
    for row in 0..order {
        let col_lo = row.saturating_sub(q);
        let col_hi = (row + q).min(order - 1);
        for col in col_lo..=col_hi {
            band_lu[row * stride + (col + q - row)] = bands[row * order + col];
        }
    }

    // Interval Gaussian elimination WITHOUT pivoting, restricted to the band.
    for k in 0..order {
        let pivot = band_lu[k * stride + q];
        if pivot.lo <= 0.0 && pivot.hi >= 0.0 {
            return Err(ConstructRefusal::SingularInterpolationSystem);
        }
        let i_hi = (k + q).min(order - 1);
        let j_hi = (k + q).min(order - 1);
        for i in (k + 1)..=i_hi {
            let mult = band_lu[i * stride + (q + k - i)]
                .div(&pivot)
                .ok_or(ConstructRefusal::SingularInterpolationSystem)?;
            band_lu[i * stride + (q + k - i)] = mult;
            for j in (k + 1)..=j_hi {
                let a_ij = band_lu[i * stride + (q + j - i)];
                let a_kj = band_lu[k * stride + (q + j - k)];
                band_lu[i * stride + (q + j - i)] = a_ij.sub(&mult.mul(&a_kj));
            }
        }
    }

    Ok(BandedFactor {
        order,
        half_bandwidth: q,
        stride,
        band_lu,
        last_max_control_width: Cell::new(0.0),
    })
}

/// An outward-rounded upper bound on the enclosure width `hi - lo`.
#[inline]
fn width_up(value: &Interval) -> f64 {
    (value.hi - value.lo).next_up()
}

/// The exact integer square root of `len`, or `None` when `len` is not a
/// perfect square. Deterministic integer binary search; no float dependence.
fn exact_isqrt(len: usize) -> Option<usize> {
    if len == 0 {
        return Some(0);
    }
    let mut lo = 0usize;
    let mut hi = 1usize << (usize::BITS / 2);
    while lo + 1 < hi {
        let mid = lo + (hi - lo) / 2;
        match mid.checked_mul(mid) {
            Some(sq) if sq <= len => lo = mid,
            _ => hi = mid,
        }
    }
    let sq = lo.checked_mul(lo)?;
    if sq == len {
        Some(lo)
    } else {
        None
    }
}
