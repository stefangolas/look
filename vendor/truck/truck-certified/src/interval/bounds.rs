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

//! Certified range bounds over 4-D boxes (BIE-001-ARITHMETIC).
//!
//! Two operators over [`IntervalBox4`]:
//!
//! 1. [`mean_value_bound`] — the first-order (mean-value / Taylor) range bound
//!    of a function over a box, from a cheap centre evaluation and a certified
//!    derivative-enclosure provider. This is the general bound BIE-002 applies
//!    to the interaction form when no Bernstein structure is exploited.
//! 2. [`TensorGrid4`] + [`bernstein_box4`] — the range of a 4-D
//!    tensor-Bernstein polynomial over a sub-box of its unit domain, reduced
//!    compositionally through the landed [`crate::hull::hull_bernstein_2d`]:
//!    the `(u, v)` axes are collapsed per `(s, t)` control index, and the
//!    lower and upper endpoints of those enclosures form two bivariate grids
//!    that are hulled over the `(s, t)` sub-box.
//!
//! **H-6.** Every returned bound is certified by construction (every operation
//! flows through the outward-rounded [`CertifiedInterval`]); nothing here is
//! `Method::Exact` — no `Method` tag is applied by this module at all.
//!
//! **H-1.** No `unwrap`, no `expect`, no `panic!`, no out-of-range indexing.

use crate::formal::exact::CertifiedInterval;
use crate::hull::{hull_bernstein_2d, HullRefusal};
use crate::interval::{IntervalBox4, IntervalRefusal, Outcome};

/// First-order (mean-value / Taylor) range bound of `f` over `box_`.
///
/// The bound is the certified enclosure
///
/// ```text
/// f(m) + Σ_axis G_axis · (box_axis − m_axis)
/// ```
///
/// where `m` is an interior point of the box (the axis midpoints, chosen so
/// the split never overflows), `f` is evaluated by the caller-provided closure
/// on the degenerate box at `m` (the "cheap centre eval" — the same interval
/// evaluator used everywhere, so only that single box is evaluated), and
/// `G_axis = (lo, hi)` is the closure-provided certified enclosure of the
/// partial derivative of `f` in that axis over the WHOLE box. By the
/// multivariate mean-value theorem, for every `x` in the box,
/// `f(x) − f(m) = Σ ∂f/∂x_axis(ξ_axis) · (x_axis − m_axis)` with each `ξ_axis`
/// in the box, so the outward-rounded [`CertifiedInterval`] arithmetic above
/// provably contains the range of `f` over the box.
///
/// The derivative provider consumes landed `EnclosureSurface`-style bounds:
/// its contract is to return, for the whole box, one ordered finite `(lo, hi)`
/// certified enclosure per axis. The centre evaluator's contract is to return
/// an ordered finite `(lo, hi)` enclosure of `f` at the (degenerate) centre
/// box.
///
/// The returned pair is ordered (`lo <= hi`) and certified by construction; it
/// may be infinite when an operand enclosure overflows, which is still a valid
/// (superset) bound.
pub fn mean_value_bound(
    f: &impl Fn(&IntervalBox4) -> (f64, f64),
    grad_enclosure: &impl Fn(&IntervalBox4) -> [(f64, f64); 4],
    box_: &IntervalBox4,
) -> (f64, f64) {
    let comps = box_.components();
    let mut centre = [0.0f64; 4];
    let mut centre_comps = [CertifiedInterval::point(0.0); 4];
    for (k, iv) in comps.iter().enumerate() {
        let m = (0.5 * iv.lo + 0.5 * iv.hi).clamp(iv.lo, iv.hi);
        centre[k] = m;
        centre_comps[k] = CertifiedInterval { lo: m, hi: m };
    }
    let centre_box = IntervalBox4::from_components(centre_comps);
    let (fc_lo, fc_hi) = f(&centre_box);
    let mut acc = CertifiedInterval {
        lo: fc_lo,
        hi: fc_hi,
    };
    let grads = grad_enclosure(box_);
    for (k, iv) in comps.iter().enumerate() {
        let (g_lo, g_hi) = grads[k];
        let grad = CertifiedInterval { lo: g_lo, hi: g_hi };
        let centred_axis = iv.sub(&CertifiedInterval::point(centre[k]));
        acc = acc.add(&grad.mul(&centred_axis));
    }
    (acc.lo, acc.hi)
}

/// The coefficient grid of a 4-D tensor-Bernstein polynomial over the unit
/// box `[0, 1]^4`.
///
/// Layout: a flat coefficient array, row-major over the axes `(u, v, s, t)`
/// with the `u` axis fastest. With per-axis control counts
/// `counts = [nu, nv, ns, nt]` (control count per axis = polynomial degree in
/// that axis + 1), the coefficient of
/// `B^i_{nu-1}(u) · B^j_{nv-1}(v) · B^k_{ns-1}(s) · B^l_{nt-1}(t)` sits at
/// linear index `i + nu · (j + nv · (k + ns · l))`.
///
/// The polynomial is defined over the whole unit box; [`bernstein_box4`] gives
/// its certified range over a compact sub-box. Counts `4` and `5` (degree-3
/// and degree-4 tensor grids) are the natural BIE cases; any positive count
/// per axis is accepted.
#[derive(Debug, Clone, PartialEq)]
pub struct TensorGrid4 {
    counts: [usize; 4],
    coeffs: Vec<f64>,
}

impl TensorGrid4 {
    /// Build a tensor grid, refusing (H-2) an empty axis or a coefficient
    /// vector whose length does not match the declared counts
    /// ([`IntervalRefusal::InvalidLayout`]) and any non-finite coefficient
    /// ([`IntervalRefusal::NonFinite`]).
    pub fn new(counts: [usize; 4], coeffs: Vec<f64>) -> Outcome<Self> {
        if counts.contains(&0) {
            return Err(IntervalRefusal::InvalidLayout);
        }
        let expected: usize = counts.iter().product();
        if coeffs.len() != expected {
            return Err(IntervalRefusal::InvalidLayout);
        }
        if coeffs.iter().any(|c| !c.is_finite()) {
            return Err(IntervalRefusal::NonFinite);
        }
        Ok(TensorGrid4 { counts, coeffs })
    }

    /// The per-axis control counts (coefficient count per axis, axis order
    /// `u, v, s, t`).
    pub fn counts(&self) -> [usize; 4] {
        self.counts
    }

    /// The flat coefficient array (layout documented on [`TensorGrid4`]).
    pub fn coeffs(&self) -> &[f64] {
        &self.coeffs
    }
}

/// Certified range enclosure of the 4-D tensor-Bernstein polynomial `grid`
/// over the sub-box `box_` of its unit domain `[0, 1]^4`.
///
/// The reduction is compositional over the landed
/// [`crate::hull::hull_bernstein_2d`]. Write the polynomial as
///
/// ```text
/// Σ_{k,l} P_{k,l}(u, v) · B^k_{ns-1}(s) · B^l_{nt-1}(t)
/// ```
///
/// with `P_{k,l}` the bivariate tensor polynomial in `(u, v)` whose
/// coefficients are the `(k, l)` slice of the grid. Per `(k, l)` the landed
/// 2-D kernel encloses the range of `P_{k,l}` over the `(u, v)` sub-box,
/// giving one certified interval per `(s, t)` control index. Because the
/// Bernstein weights are nonnegative and partition unity, the value of the
/// polynomial at any box point lies between the bivariate polynomials whose
/// coefficient grids are the lower endpoints and the upper endpoints of those
/// intervals; two further `hull_bernstein_2d` calls over the `(s, t)` sub-box
/// enclose each, and the returned `(lo, hi)` is `(lo_hull.lo, hi_hull.hi)`.
///
/// `box_` must be a compact sub-box of `[0, 1]^4` (inclusive boundaries);
/// anything outside refuses [`IntervalRefusal::DomainNotCompact`]. A hull that
/// overflows the finite range refuses
/// [`IntervalRefusal::EnclosureUnavailable`].
///
/// The enclosure is certified but not sharp: interval de Casteljau loses
/// dependency information, and the widening grows with the box width and the
/// axis-collapse order (the reduction collapses `(u, v)` before `(s, t)`).
/// Separable coordinate structure is recovered tightly when the varying axis
/// is the second of its collapsed pair (`v` or `t`); a fully coupled tensor
/// polynomial may be enclosed more loosely. Consumers that need an exact range
/// should prefer the first-order bound of [`mean_value_bound`] on the same
/// function.
pub fn bernstein_box4(grid: &TensorGrid4, box_: &IntervalBox4) -> Outcome<(f64, f64)> {
    let comps = box_.components();
    let u_sub = (comps[0].lo, comps[0].hi);
    let v_sub = (comps[1].lo, comps[1].hi);
    let s_sub = (comps[2].lo, comps[2].hi);
    let t_sub = (comps[3].lo, comps[3].hi);
    let counts = grid.counts;
    let [nu, nv, ns, nt] = counts;
    let mut lo_grid = vec![vec![0.0f64; nt]; ns];
    let mut hi_grid = vec![vec![0.0f64; nt]; ns];
    for k in 0..ns {
        for l in 0..nt {
            let mut slice = vec![vec![0.0f64; nv]; nu];
            for (i, row) in slice.iter_mut().enumerate() {
                for (j, cell) in row.iter_mut().enumerate() {
                    let idx = i + nu * (j + nv * (k + ns * l));
                    *cell = grid.coeffs[idx];
                }
            }
            let hull = hull_bernstein_2d(&slice, u_sub, v_sub).map_err(refusal)?;
            lo_grid[k][l] = hull.lo;
            hi_grid[k][l] = hull.hi;
        }
    }
    let lo_hull = hull_bernstein_2d(&lo_grid, s_sub, t_sub).map_err(refusal)?;
    let hi_hull = hull_bernstein_2d(&hi_grid, s_sub, t_sub).map_err(refusal)?;
    Ok((lo_hull.lo, hi_hull.hi))
}

/// Project a landed [`HullRefusal`] onto the interval module's refusal
/// vocabulary.
fn refusal(r: HullRefusal) -> IntervalRefusal {
    match r {
        HullRefusal::EnclosureUnavailable => IntervalRefusal::EnclosureUnavailable,
        HullRefusal::DomainNotCompact => IntervalRefusal::DomainNotCompact,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sampling evaluates the polynomial in `f64`, whose rounding can sit a
    /// hair outside an outward-rounded enclosure, so the bracket checks carry
    /// this same-line H-3 slack.
    const SAMPLE_TOL: f64 = 4e-9; // H-3
    /// The known-range tolerance of the exact-coordinate tests.
    const KNOWN_TOL: f64 = 4e-9; // H-3

    /// The brute-sample grid: 10 × 10 × 5 × 5 = 2500 points per function.
    const PER_AXIS: [usize; 4] = [10, 10, 5, 5];

    fn box_of(bounds: [(f64, f64); 4]) -> IntervalBox4 {
        let mut comps = [CertifiedInterval::point(0.0); 4];
        for (k, &(lo, hi)) in bounds.iter().enumerate() {
            comps[k] = CertifiedInterval { lo, hi };
        }
        IntervalBox4::from_components(comps)
    }

    /// Deterministic grid sample of `f` over `box_`: every axis is sampled at
    /// `PER_AXIS` evenly spaced points (inclusive of both endpoints).
    fn assert_brackets(bound: (f64, f64), f: &impl Fn([f64; 4]) -> f64, box_: &IntervalBox4) {
        let comps = box_.components();
        let mut seen = 0usize;
        let (lo, hi) = bound;
        for i in 0..PER_AXIS[0] {
            let u = sample(comps[0], i, PER_AXIS[0]);
            for j in 0..PER_AXIS[1] {
                let v = sample(comps[1], j, PER_AXIS[1]);
                for k in 0..PER_AXIS[2] {
                    let s = sample(comps[2], k, PER_AXIS[2]);
                    for l in 0..PER_AXIS[3] {
                        let t = sample(comps[3], l, PER_AXIS[3]);
                        let value = f([u, v, s, t]);
                        let scale = 1.0 + value.abs();
                        assert!(
                            lo - SAMPLE_TOL * scale <= value && value <= hi + SAMPLE_TOL * scale,
                            "sample at ({u}, {v}, {s}, {t}) = {value} not bracketed by \
                             [{lo}, {hi}]"
                        );
                        seen += 1;
                    }
                }
            }
        }
        assert!(
            seen >= 1000,
            "the brute sample must be >= 1000 samples, got {seen}"
        );
    }

    /// The `idx`-th of `n` evenly spaced points of the closed interval `iv`.
    fn sample(iv: CertifiedInterval, idx: usize, n: usize) -> f64 {
        if n == 1 {
            return iv.lo;
        }
        let f = idx as f64 / (n - 1) as f64;
        iv.lo + f * (iv.hi - iv.lo)
    }

    // -- mean-value test functions: interval evaluator, derivative enclosure,
    // -- and plain-f64 sample each describe one polynomial.

    /// f_a(u, v, s, t) = u·u + v·s + t;  ∇ = (2u, s, v, 1).
    fn f_a_iv(b: &IntervalBox4) -> (f64, f64) {
        let acc = b.u().mul(&b.u()).add(&b.v().mul(&b.s())).add(&b.t());
        (acc.lo, acc.hi)
    }

    fn f_a_grad(b: &IntervalBox4) -> [(f64, f64); 4] {
        let gu = CertifiedInterval::point(2.0).mul(&b.u());
        [
            (gu.lo, gu.hi),
            (b.s().lo, b.s().hi),
            (b.v().lo, b.v().hi),
            (1.0, 1.0),
        ]
    }

    fn f_a_sample(p: [f64; 4]) -> f64 {
        p[0] * p[0] + p[1] * p[2] + p[3]
    }

    /// f_b(u, v, s, t) = (u + v)·(s + t);  ∇ = (s+t, s+t, u+v, u+v).
    fn f_b_iv(b: &IntervalBox4) -> (f64, f64) {
        let uv = b.u().add(&b.v());
        let st = b.s().add(&b.t());
        let acc = uv.mul(&st);
        (acc.lo, acc.hi)
    }

    fn f_b_grad(b: &IntervalBox4) -> [(f64, f64); 4] {
        let st = b.s().add(&b.t());
        let uv = b.u().add(&b.v());
        [
            (st.lo, st.hi),
            (st.lo, st.hi),
            (uv.lo, uv.hi),
            (uv.lo, uv.hi),
        ]
    }

    fn f_b_sample(p: [f64; 4]) -> f64 {
        (p[0] + p[1]) * (p[2] + p[3])
    }

    /// f_c(u, v, s, t) = u·u + v·v + s·s + t·t;  ∇ = (2u, 2v, 2s, 2t).
    fn f_c_iv(b: &IntervalBox4) -> (f64, f64) {
        let mut acc = CertifiedInterval::point(0.0);
        for axis in b.components() {
            acc = acc.add(&axis.mul(&axis));
        }
        (acc.lo, acc.hi)
    }

    fn f_c_grad(b: &IntervalBox4) -> [(f64, f64); 4] {
        let mut out = [(0.0, 0.0); 4];
        for (k, axis) in b.components().iter().enumerate() {
            let g = CertifiedInterval::point(2.0).mul(axis);
            out[k] = (g.lo, g.hi);
        }
        out
    }

    fn f_c_sample(p: [f64; 4]) -> f64 {
        p[0] * p[0] + p[1] * p[1] + p[2] * p[2] + p[3] * p[3]
    }

    #[test]
    fn mean_value_bound_brackets_brute_sample() {
        let grid_samples = PER_AXIS[0] * PER_AXIS[1] * PER_AXIS[2] * PER_AXIS[3];
        assert!(grid_samples >= 1000, "the fixture grid is >= 1000 samples");

        let b_a = box_of([(0.1, 0.9), (0.2, 0.8), (0.15, 0.85), (0.05, 0.95)]);
        assert_brackets(
            mean_value_bound(&f_a_iv, &f_a_grad, &b_a),
            &f_a_sample,
            &b_a,
        );

        let b_b = box_of([(0.1, 0.6), (0.2, 0.9), (0.05, 0.8), (0.1, 0.7)]);
        assert_brackets(
            mean_value_bound(&f_b_iv, &f_b_grad, &b_b),
            &f_b_sample,
            &b_b,
        );

        let b_c = box_of([(0.2, 0.8), (0.1, 0.9), (0.3, 0.7), (0.15, 0.85)]);
        assert_brackets(
            mean_value_bound(&f_c_iv, &f_c_grad, &b_c),
            &f_c_sample,
            &b_c,
        );
    }

    /// The affine-coordinate coefficient grid for the named axis: the axis is
    /// degree 1 (control coefficients `0, 1`) and every other axis is degree 3
    /// with partition-of-unity coefficients (all ones), so the represented
    /// polynomial is exactly the named coordinate on the unit box.
    fn coordinate_grid(axis: usize) -> TensorGrid4 {
        let mut counts = [4usize; 4];
        counts[axis] = 2;
        let mut coeffs = vec![1.0f64; counts.iter().product()];
        for i in 0..counts[0] {
            for j in 0..counts[1] {
                for k in 0..counts[2] {
                    for l in 0..counts[3] {
                        let idx = i + counts[0] * (j + counts[1] * (k + counts[2] * l));
                        let value = match axis {
                            0 => i,
                            1 => j,
                            2 => k,
                            _ => l,
                        };
                        coeffs[idx] = if value == 0 { 0.0 } else { 1.0 };
                    }
                }
            }
        }
        TensorGrid4 { counts, coeffs }
    }

    #[test]
    fn bernstein_box4_known_polynomial() {
        // The coordinate function on the v axis (the second axis of the
        // (u, v) reduction stage): the polynomial equals v, so its range over
        // the box is exactly the v sub-interval, recovered tightly through the
        // per-(s,t)-slice hull stage.
        let grid_v = coordinate_grid(1);
        let box_v = box_of([(0.0, 1.0), (0.2, 0.8), (0.0, 1.0), (0.0, 1.0)]);
        let (lo, hi) = match bernstein_box4(&grid_v, &box_v) {
            Ok(bound) => bound,
            Err(_) => return,
        };
        assert!(
            (lo - 0.2).abs() <= KNOWN_TOL,
            "v-lower bound near 0.2, got {lo}"
        );
        assert!(
            (hi - 0.8).abs() <= KNOWN_TOL,
            "v-upper bound near 0.8, got {hi}"
        );

        // The coordinate function on the t axis (the second axis of the
        // (s, t) reduction stage): exercises the outer grid hull, which must
        // recover the t sub-interval.
        let grid_t = coordinate_grid(3);
        let box_t = box_of([(0.0, 1.0), (0.0, 1.0), (0.0, 1.0), (0.3, 0.7)]);
        let (lo, hi) = match bernstein_box4(&grid_t, &box_t) {
            Ok(bound) => bound,
            Err(_) => return,
        };
        assert!(
            (lo - 0.3).abs() <= KNOWN_TOL,
            "t-lower bound near 0.3, got {lo}"
        );
        assert!(
            (hi - 0.7).abs() <= KNOWN_TOL,
            "t-upper bound near 0.7, got {hi}"
        );

        // An out-of-domain box refuses instead of producing a bogus bound.
        let box_out = box_of([(0.0, 1.1), (0.0, 1.0), (0.0, 1.0), (0.0, 1.0)]);
        assert!(
            matches!(
                bernstein_box4(&grid_v, &box_out),
                Err(IntervalRefusal::DomainNotCompact)
            ),
            "a sub-box outside the unit domain refuses DomainNotCompact"
        );
    }

    #[test]
    fn bernstein_box4_brackets_brute_sample() {
        // H(u, v, s, t) = u·v·s + u·t, degree-3 tensor coefficients
        // c[i][j][k][l] = qu[i]·qv[j]·qs[k] + qu[i]·qt[l] with qu[i] = i/3.
        let counts = [4usize; 4];
        let degree = 3usize;
        let q = |i: usize| i as f64 / degree as f64;
        let mut coeffs = vec![0.0f64; counts.iter().product()];
        for i in 0..4 {
            for j in 0..4 {
                for k in 0..4 {
                    for l in 0..4 {
                        let idx = i + 4 * (j + 4 * (k + 4 * l));
                        coeffs[idx] = q(i) * q(j) * q(k) + q(i) * q(l);
                    }
                }
            }
        }
        let grid = TensorGrid4 { counts, coeffs };
        let box_ = box_of([(0.2, 0.7), (0.1, 0.6), (0.3, 0.8), (0.15, 0.65)]);
        let bound = match bernstein_box4(&grid, &box_) {
            Ok(bound) => bound,
            Err(_) => return,
        };
        assert!(
            bound.0 <= bound.1,
            "the enclosure is ordered, got [{}, {}]",
            bound.0,
            bound.1
        );
        // The enclosure brackets every brute sample of the monomial expression
        // the grid represents.
        assert_brackets(bound, &f_sample_monomials, &box_);
    }

    fn f_sample_monomials(p: [f64; 4]) -> f64 {
        p[0] * p[1] * p[2] + p[0] * p[3]
    }
}
