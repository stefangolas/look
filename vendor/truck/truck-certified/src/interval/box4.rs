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

//! Outward-rounded 4-D parameter boxes over `(u, v, s, t)` (BIE-001-ARITHMETIC).
//!
//! A box is four [`CertifiedInterval`]s (landed, outward-rounded, untouched);
//! this module adds the box shape, the refusing constructor, bisection, the
//! component-wise `add`/`sub`/`mul` composed from the scalar operations, and
//! the width used to drive bisection termination. All arithmetic flows through
//! `CertifiedInterval`, so every box result carries the same outward rounding
//! as the scalar engine.
//!
//! **H-1.** No `unwrap`, no `expect`, no `panic!`; `bisect` refuses an
//! out-of-range axis by returning `None` and never indexes out of range.

use crate::formal::exact::CertifiedInterval;
use crate::interval::{IntervalRefusal, Outcome};

/// A 4-D parameter box over `(u, v, s, t)`: one outward-rounded
/// [`CertifiedInterval`] per axis.
///
/// Construction is refusing (H-2): inverted bounds (`lo > hi`) refuse
/// [`IntervalRefusal::InvertedBounds`] and non-finite endpoints refuse
/// [`IntervalRefusal::NonFinite`]; both are refusals, never panics. Degenerate
/// boxes (`lo == hi`) and partially degenerate axes are valid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IntervalBox4 {
    comps: [CertifiedInterval; 4],
}

impl IntervalBox4 {
    /// Build a box from four axis bounds `[(u_lo, u_hi), (v_lo, v_hi),
    /// (s_lo, s_hi), (t_lo, t_hi)]`, refusing inverted and non-finite bounds.
    pub fn new(bounds: [(f64, f64); 4]) -> Outcome<Self> {
        let mut comps = [CertifiedInterval::point(0.0); 4];
        for (k, &(lo, hi)) in bounds.iter().enumerate() {
            if !lo.is_finite() || !hi.is_finite() {
                return Err(IntervalRefusal::NonFinite);
            }
            if lo > hi {
                return Err(IntervalRefusal::InvertedBounds);
            }
            comps[k] = CertifiedInterval { lo, hi };
        }
        Ok(IntervalBox4 { comps })
    }

    /// Build a box from already-certified component intervals.
    ///
    /// The crate-internal construction path (bisection halves, the centre
    /// evaluation box of the mean-value bound); callers must supply ordered,
    /// finite components.
    pub(crate) fn from_components(comps: [CertifiedInterval; 4]) -> Self {
        IntervalBox4 { comps }
    }

    /// The `u`-axis interval.
    pub fn u(&self) -> CertifiedInterval {
        self.comps[0]
    }

    /// The `v`-axis interval.
    pub fn v(&self) -> CertifiedInterval {
        self.comps[1]
    }

    /// The `s`-axis interval.
    pub fn s(&self) -> CertifiedInterval {
        self.comps[2]
    }

    /// The `t`-axis interval.
    pub fn t(&self) -> CertifiedInterval {
        self.comps[3]
    }

    /// The four component intervals, in axis order `u, v, s, t`.
    pub fn components(&self) -> [CertifiedInterval; 4] {
        self.comps
    }

    /// Bisect the box on `axis` (`0..=3`, mapping `u`/`v`/`s`/`t`) into the
    /// two boxes whose `axis` intervals partition the original: `[lo, m]` and
    /// `[m, hi]`, with `m` a point of the original axis chosen so the split
    /// never overflows and always stays inside `[lo, hi]`. Every other axis is
    /// copied unchanged. An out-of-range axis returns `None` (H-1: the box is
    /// never indexed out of range).
    pub fn bisect(&self, axis: usize) -> Option<(Self, Self)> {
        if axis >= 4 {
            return None;
        }
        let iv = self.comps[axis];
        let mid = (0.5 * iv.lo + 0.5 * iv.hi).clamp(iv.lo, iv.hi);
        let mut lower = self.comps;
        let mut upper = self.comps;
        lower[axis] = CertifiedInterval { lo: iv.lo, hi: mid };
        upper[axis] = CertifiedInterval { lo: mid, hi: iv.hi };
        Some((IntervalBox4 { comps: lower }, IntervalBox4 { comps: upper }))
    }

    /// Component-wise outward-rounded addition (each axis is the scalar
    /// [`CertifiedInterval::add`]).
    pub fn add(&self, other: &Self) -> Self {
        let mut out = [CertifiedInterval::point(0.0); 4];
        for (k, (a, b)) in self.comps.iter().zip(other.comps.iter()).enumerate() {
            out[k] = a.add(b);
        }
        IntervalBox4 { comps: out }
    }

    /// Component-wise outward-rounded subtraction (each axis is the scalar
    /// [`CertifiedInterval::sub`]).
    pub fn sub(&self, other: &Self) -> Self {
        let mut out = [CertifiedInterval::point(0.0); 4];
        for (k, (a, b)) in self.comps.iter().zip(other.comps.iter()).enumerate() {
            out[k] = a.sub(b);
        }
        IntervalBox4 { comps: out }
    }

    /// Component-wise outward-rounded multiplication (each axis is the scalar
    /// [`CertifiedInterval::mul`]).
    pub fn mul(&self, other: &Self) -> Self {
        let mut out = [CertifiedInterval::point(0.0); 4];
        for (k, (a, b)) in self.comps.iter().zip(other.comps.iter()).enumerate() {
            out[k] = a.mul(b);
        }
        IntervalBox4 { comps: out }
    }

    /// The box width: the widest per-axis interval width (a diameter in the
    /// axis-wise max metric). Zero exactly when the box is degenerate.
    pub fn width(&self) -> f64 {
        self.comps.iter().fold(0.0, |acc, c| acc.max(c.width()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit() -> [(f64, f64); 4] {
        [(0.0, 1.0), (0.0, 1.0), (0.0, 1.0), (0.0, 1.0)]
    }

    #[test]
    fn interval_box4_refuses_inverted_and_nonfinite() {
        let mut inverted = unit();
        inverted[0] = (1.0, 0.0);
        assert_eq!(
            IntervalBox4::new(inverted),
            Err(IntervalRefusal::InvertedBounds),
            "lo > hi on the u axis refuses InvertedBounds"
        );
        let mut inverted = unit();
        inverted[2] = (0.5, 0.25);
        assert_eq!(
            IntervalBox4::new(inverted),
            Err(IntervalRefusal::InvertedBounds),
            "lo > hi on the s axis refuses InvertedBounds"
        );

        let mut nan_lo = unit();
        nan_lo[1] = (f64::NAN, 0.5);
        assert_eq!(
            IntervalBox4::new(nan_lo),
            Err(IntervalRefusal::NonFinite),
            "a NaN lower bound refuses NonFinite"
        );
        let mut nan_hi = unit();
        nan_hi[3] = (0.0, f64::NAN);
        assert_eq!(
            IntervalBox4::new(nan_hi),
            Err(IntervalRefusal::NonFinite),
            "a NaN upper bound refuses NonFinite"
        );
        let mut pos_inf = unit();
        pos_inf[0] = (0.0, f64::INFINITY);
        assert_eq!(
            IntervalBox4::new(pos_inf),
            Err(IntervalRefusal::NonFinite),
            "a +inf bound refuses NonFinite"
        );
        let mut neg_inf = unit();
        neg_inf[1] = (f64::NEG_INFINITY, 1.0);
        assert_eq!(
            IntervalBox4::new(neg_inf),
            Err(IntervalRefusal::NonFinite),
            "a -inf bound refuses NonFinite"
        );

        assert!(IntervalBox4::new(unit()).is_ok());
        assert!(
            IntervalBox4::new([(0.5, 0.5); 4]).is_ok(),
            "degenerate boxes are valid"
        );
        assert!(
            IntervalBox4::new([(-1.0, 2.0), (0.0, 0.0), (0.0, 1.0), (3.0, 7.0)]).is_ok(),
            "finite ordered bounds including a degenerate axis are valid"
        );
    }

    #[test]
    fn interval_box_ops_are_outward_rounded() {
        let a_vals = [1.0, 2.0, -3.0, 0.5];
        let b_vals = [0.25, -0.5, 2.0, 1.0];
        let a = IntervalBox4::from_components([
            CertifiedInterval::point(a_vals[0]),
            CertifiedInterval::point(a_vals[1]),
            CertifiedInterval::point(a_vals[2]),
            CertifiedInterval::point(a_vals[3]),
        ]);
        let b = IntervalBox4::from_components([
            CertifiedInterval::point(b_vals[0]),
            CertifiedInterval::point(b_vals[1]),
            CertifiedInterval::point(b_vals[2]),
            CertifiedInterval::point(b_vals[3]),
        ]);
        let sum = a.add(&b);
        let diff = a.sub(&b);
        let prod = a.mul(&b);
        for k in 0..4 {
            let exact_sum = a_vals[k] + b_vals[k];
            let exact_diff = a_vals[k] - b_vals[k];
            let exact_prod = a_vals[k] * b_vals[k];
            assert!(
                sum.components()[k].contains(exact_sum),
                "axis {k}: the exact sum {exact_sum} lies in the outward box"
            );
            assert!(
                diff.components()[k].contains(exact_diff),
                "axis {k}: the exact difference {exact_diff} lies in the outward box"
            );
            assert!(
                prod.components()[k].contains(exact_prod),
                "axis {k}: the exact product {exact_prod} lies in the outward box"
            );
        }

        let box_ = match IntervalBox4::new([(0.1, 0.9), (-0.5, 0.5), (0.0, 2.0), (0.25, 0.75)]) {
            Ok(b) => b,
            Err(_) => return,
        };
        let whole_width = box_.width();
        for axis in 0..4 {
            let (lower, upper) = match box_.bisect(axis) {
                Some(parts) => parts,
                None => return,
            };
            assert!(
                lower.width() <= whole_width,
                "axis {axis}: the lower half is no wider than the box"
            );
            assert!(
                upper.width() <= whole_width,
                "axis {axis}: the upper half is no wider than the box"
            );
            for k in 0..4 {
                assert!(
                    box_.components()[k].lo <= lower.components()[k].lo
                        && lower.components()[k].hi <= box_.components()[k].hi,
                    "axis {axis}: the lower half lies inside the box on axis {k}"
                );
                assert!(
                    box_.components()[k].lo <= upper.components()[k].lo
                        && upper.components()[k].hi <= box_.components()[k].hi,
                    "axis {axis}: the upper half lies inside the box on axis {k}"
                );
            }
            let joined = match IntervalBox4::new([
                (lower.components()[0].lo, upper.components()[0].hi),
                (lower.components()[1].lo, upper.components()[1].hi),
                (lower.components()[2].lo, upper.components()[2].hi),
                (lower.components()[3].lo, upper.components()[3].hi),
            ]) {
                Ok(j) => j,
                Err(_) => return,
            };
            assert!(
                whole_width <= joined.width() && joined.width() <= whole_width,
                "axis {axis}: re-joining the halves reproduces the original box width"
            );
        }
        assert!(
            box_.bisect(4).is_none() && box_.bisect(usize::MAX).is_none(),
            "out-of-range bisect axes return None, never indexing out of range"
        );
    }
}
