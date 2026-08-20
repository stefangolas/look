//! BG-ENC-005: certified elementary functions for interval arguments.
//!
//! BG-ENC-002 tells every analytic carrier to "propagate intervals through the
//! parameterisation, being careful that `sin`/`cos` over an interval must
//! account for the extrema at kπ/2 **inside** the interval". It does not say
//! where those interval trig functions come from, and in this tree they did not
//! exist: `inari` puts `Interval::sin`/`::cos` in `src/elementary.rs` behind
//! `#[cfg(feature = "gmp")]`, and `truck-evidence` depends on
//! `inari = { version = "2.0", default-features = false }`. `plane.rs` never
//! noticed because a plane is affine. The first curved carrier did.
//!
//! Enabling `gmp` would pull `gmp-mpfr-sys` and `rug`, which build GMP and MPFR
//! from source through autotools. This machine has no `make` and no `m4`, and
//! the active toolchain is `x86_64-pc-windows-gnullvm`, which `gmp-mpfr-sys`
//! does not list as supported. So the functions are built here instead, on the
//! part of `inari` that is *not* feature-gated: outward-rounded arithmetic,
//! `sqr`, `floor`, and the correctly-rounded constants `PI` and `FRAC_PI_2`.
//!
//! # Why this is sound
//!
//! Three ingredients, each of which is a theorem rather than an estimate:
//!
//! 1. **Interval arithmetic is inclusion-monotonic and rounds outward.** Any
//!    expression evaluated on intervals encloses that expression's range over
//!    the inputs. This is `inari`'s contract and the whole reason it is here.
//! 2. **The Taylor series of sin is alternating with decreasing terms for
//!    |t| ≤ 1**, so the truncation error is bounded by the magnitude of the
//!    first omitted term — exactly, not asymptotically. That is what makes a
//!    finite sum into an enclosure.
//! 3. **Argument reduction by an integer multiple of π/2 is exact as an
//!    identity.** `sin(x) = ±sin(x − k·π/2)` or `±cos(x − k·π/2)` holds for
//!    *every* integer k, so k does not have to be the "right" one. The only
//!    thing k affects is how wide the reduced argument is, and the reduction's
//!    own rounding is captured because `x − k·π/2` is computed in interval
//!    arithmetic against an enclosure of π/2 rather than a float approximation.
//!
//! Point 3 is what makes this honest for large arguments. Reducing a big `x`
//! cancels catastrophically; here that cancellation shows up as a *wide*
//! reduced interval, and when it grows past the series' domain the functions
//! return `[-1, 1]`. Loose, never wrong — which is the direction this crate
//! exists to enforce.
//!
//! # What it is not
//!
//! Not competitive with a correctly-rounded MPFR implementation for width.
//! Near the reduction boundaries and for large arguments it gives up accuracy
//! and eventually gives up entirely. Tightening it is a later concern; the
//! contract here is BG-ENC-001 soundness, and every choice below resolves in
//! favour of a wider answer.

use inari::{const_interval, Interval};

/// `[-1, 1]`: the range of sin and cos over any argument whatsoever, and the
/// answer whenever a reduction stops being informative.
const FULL_RANGE: Interval = const_interval!(-1.0, 1.0);

/// A degenerate interval at a runtime `f64`. Non-finite input would make an
/// invalid interval, so it widens to everything instead of panicking — the
/// crate denies `unwrap`/`panic` and a NaN parameter is a caller's bug, not a
/// reason to abort a certified evaluation.
fn at(x: f64) -> Interval {
    if x.is_finite() {
        // inf <= sup and both finite: a valid singleton.
        Interval::try_from((x, x)).unwrap_or(Interval::ENTIRE)
    } else {
        Interval::ENTIRE
    }
}

/// Terms of the Taylor partial sum. At |t| ≤ π/4 the first omitted term is
/// below 1e-24, i.e. far under an f64 ulp of the result, so the truncation is
/// not what limits the width — the reduction is.
const TAYLOR_TERMS: usize = 10;

/// The largest reduced argument the series is used for. The alternating-series
/// remainder bound needs the terms to decrease, which holds for |t| ≤ 1; a
/// correct reduction gives |t| ≲ π/4, so anything above this means the
/// reduction has lost its meaning and the caller gets `[-1, 1]`.
const SERIES_DOMAIN: f64 = 1.0;

/// sin on an already-reduced argument, `|t| ≲ π/4`.
///
/// `sin t = Σ (−1)^k t^(2k+1)/(2k+1)!`, truncated after `TAYLOR_TERMS` terms
/// and inflated by the magnitude of the first omitted term.
fn sin_series(t: Interval) -> Interval {
    let t2 = t.sqr();
    // term_k = t^(2k+1) / (2k+1)!, carried multiplicatively so no factorial is
    // ever formed (they overflow long before k = 10 would need them to).
    let mut term = t;
    let mut sum = term;
    let mut positive = false;
    for k in 1..TAYLOR_TERMS {
        term = term * t2 / at(((2 * k) * (2 * k + 1)) as f64);
        sum = if positive { sum + term } else { sum - term };
        positive = !positive;
    }
    let next = term * t2 / at(((2 * TAYLOR_TERMS) * (2 * TAYLOR_TERMS + 1)) as f64);
    inflate(sum, next.mag())
}

/// cos on an already-reduced argument, `|t| ≲ π/4`.
///
/// `cos t = Σ (−1)^k t^(2k)/(2k)!`, same construction as [`sin_series`].
fn cos_series(t: Interval) -> Interval {
    let t2 = t.sqr();
    let mut term = at(1.0);
    let mut sum = term;
    let mut positive = false;
    for k in 1..TAYLOR_TERMS {
        term = term * t2 / at(((2 * k - 1) * (2 * k)) as f64);
        sum = if positive { sum + term } else { sum - term };
        positive = !positive;
    }
    let next = term * t2 / at(((2 * TAYLOR_TERMS - 1) * (2 * TAYLOR_TERMS)) as f64);
    inflate(sum, next.mag())
}

/// Widen `iv` by `eps` on both sides. `eps` is a truncation bound, so this is
/// the step that turns a partial sum into an enclosure.
fn inflate(iv: Interval, eps: f64) -> Interval {
    if !eps.is_finite() {
        return Interval::ENTIRE;
    }
    let pad = at(eps.abs());
    iv + const_interval!(-1.0, 1.0) * pad
}

/// The reduced argument `x − k·π/2` and `k mod 4`, or `None` when the
/// reduction is not usable.
///
/// `k` is chosen as the nearest integer to `x/(π/2)` in plain `f64`, and that
/// choice needs no justification: the reduction identity holds for every
/// integer, so a `k` that is off by one only costs width. What must be exact is
/// the *subtraction*, which is why it runs in interval arithmetic against
/// `Interval::FRAC_PI_2` rather than against a float π/2.
fn reduce(x: f64) -> Option<(Interval, u8)> {
    if !x.is_finite() {
        return None;
    }
    let k = (x / core::f64::consts::FRAC_PI_2).round();
    // Beyond 2^53 the quadrant of a float argument is not determined by the
    // float itself, so there is nothing to reduce to.
    if !k.is_finite() || k.abs() >= 9.007_199_254_740_992e15 {
        return None;
    }
    let r = at(x) - Interval::FRAC_PI_2 * at(k);
    // Written as the positive test so a NaN magnitude (an empty interval) also
    // falls through to `None` rather than through the series.
    let mag = r.mag();
    if mag.is_nan() || mag > SERIES_DOMAIN {
        return None;
    }
    // rem_euclid on the f64 keeps the quadrant in 0..4 for negative k too.
    let quadrant = (k.rem_euclid(4.0)) as u8;
    Some((r, quadrant))
}

/// A certified enclosure of `sin(x)` for a single `f64`.
fn sin_at(x: f64) -> Interval {
    match reduce(x) {
        None => FULL_RANGE,
        Some((r, quadrant)) => match quadrant {
            0 => sin_series(r),
            1 => cos_series(r),
            2 => -sin_series(r),
            _ => -cos_series(r),
        },
    }
}

/// An enclosure of `{ sin(x) : x ∈ xx }` (BG-ENC-001 for the sine).
///
/// The endpoints are enclosed by [`sin_at`]; the interior extrema at
/// `π/2 + kπ` are then added wherever the cell can reach one. "Can reach" is
/// deliberately generous: an extremum is included whenever its own enclosure
/// merely *intersects* the cell, so a critical point that is within a rounding
/// step of the boundary is counted as inside. Including an extremum that is
/// not attained widens the answer; missing one that is attained is the classic
/// interval-trigonometry bug, and it under-estimates.
pub fn sin(xx: Interval) -> Interval {
    if xx.is_empty() {
        return Interval::EMPTY;
    }
    let (lo, hi) = (xx.inf(), xx.sup());
    if !lo.is_finite() || !hi.is_finite() {
        return FULL_RANGE;
    }
    // A cell at least a full period wide attains everything. Compare against
    // the *lower* bound of 2π so the test never claims a full period early.
    if xx.wid() >= (Interval::PI * at(2.0)).inf() {
        return FULL_RANGE;
    }

    let ends = sin_at(lo).convex_hull(sin_at(hi));
    let mut result_lo = ends.inf();
    let mut result_hi = ends.sup();

    // Critical points of sin are π/2 + kπ, with sin = +1 for even k and −1 for
    // odd k. The cell is under a period wide, so at most two of them are in
    // range; the window below is bounded and its bounds are checked rather
    // than assumed.
    let first = ((at(lo) - Interval::FRAC_PI_2) / Interval::PI)
        .inf()
        .floor();
    let last = ((at(hi) - Interval::FRAC_PI_2) / Interval::PI).sup().ceil();
    if !first.is_finite() || !last.is_finite() || last - first > 8.0 {
        return FULL_RANGE;
    }
    let mut k = first;
    while k <= last {
        let critical = Interval::FRAC_PI_2 + Interval::PI * at(k);
        if !critical.intersection(xx).is_empty() {
            if (k.rem_euclid(2.0)) == 0.0 {
                result_hi = 1.0;
            } else {
                result_lo = -1.0;
            }
        }
        k += 1.0;
    }

    Interval::try_from((result_lo.max(-1.0), result_hi.min(1.0))).unwrap_or(FULL_RANGE)
}

/// An enclosure of `{ cos(x) : x ∈ xx }` (BG-ENC-001 for the cosine).
///
/// `cos x = sin(x + π/2)`, evaluated against the interval π/2 so the shift's
/// rounding is carried rather than dropped. The identity is exact, so this
/// inherits [`sin`]'s soundness; the cost is the width of `FRAC_PI_2`, about
/// 2e-16, which is far below the widths BG-NUM subdivides down to.
pub fn cos(xx: Interval) -> Interval {
    if xx.is_empty() {
        return Interval::EMPTY;
    }
    sin(xx + Interval::FRAC_PI_2)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sampling density for the containment properties. These are the direct
    /// BG-ENC-001 tests for this module.
    const SAMPLES: usize = 401;

    /// The crate denies `expect`/`unwrap` even in tests, so a malformed test
    /// interval degrades to EMPTY and fails the assertion that uses it rather
    /// than panicking here.
    fn iv(lo: f64, hi: f64) -> Interval {
        Interval::try_from((lo, hi)).unwrap_or(Interval::EMPTY)
    }

    /// `f64::sin` is not the definition of sin, but every libm worth using is
    /// within an ulp or so of it, so a certified enclosure that does not
    /// contain it is certainly wrong. The slack absorbs the libm error without
    /// absorbing a real defect.
    const LIBM_SLACK: f64 = 1.0e-12;

    fn assert_contains(iv: Interval, truth: f64, what: &str) {
        assert!(
            truth >= iv.inf() - LIBM_SLACK && truth <= iv.sup() + LIBM_SLACK,
            "{what}: {truth} escaped {iv:?}"
        );
    }

    #[test]
    fn sin_encloses_sampled_values() {
        let cells = [
            iv(0.0, 0.1),
            iv(-0.3, 0.3),
            iv(1.0, 2.0),
            iv(3.0, 3.5),
            iv(-7.0, -6.0),
            iv(10.0, 10.2),
            iv(0.4 * core::f64::consts::PI, 0.6 * core::f64::consts::PI),
        ];
        for cell in cells {
            let enclosure = sin(cell);
            let step = cell.wid() / (SAMPLES as f64 - 1.0);
            for i in 0..SAMPLES {
                let x = cell.inf() + step * (i as f64);
                assert_contains(enclosure, x.sin(), "sin");
            }
        }
    }

    #[test]
    fn cos_encloses_sampled_values() {
        let cells = [
            iv(0.0, 0.1),
            iv(-0.3, 0.3),
            iv(1.0, 2.0),
            iv(3.0, 3.5),
            iv(-7.0, -6.0),
            iv(10.0, 10.2),
        ];
        for cell in cells {
            let enclosure = cos(cell);
            let step = cell.wid() / (SAMPLES as f64 - 1.0);
            for i in 0..SAMPLES {
                let x = cell.inf() + step * (i as f64);
                assert_contains(enclosure, x.cos(), "cos");
            }
        }
    }

    /// The bug this whole module exists to prevent: an interval spanning an
    /// interior extremum must reach it, which endpoint-only evaluation does
    /// not. `sin` over `[0.4π, 0.6π]` peaks at 1 in the interior while both
    /// endpoints are below 0.952.
    #[test]
    fn interval_spanning_a_peak_reaches_it() {
        let cell = iv(0.4 * core::f64::consts::PI, 0.6 * core::f64::consts::PI);
        let enclosure = sin(cell);
        let endpoints_only = cell.inf().sin().max(cell.sup().sin());
        assert!(
            endpoints_only < 0.96,
            "the test cell must not peak at an end"
        );
        assert!(
            enclosure.sup() >= 1.0 - LIBM_SLACK,
            "sin over {cell:?} must reach 1, got {enclosure:?}"
        );
        assert!(enclosure.sup() > endpoints_only);
    }

    /// The same for a trough, and for cos, whose critical points sit where
    /// sin's do not.
    #[test]
    fn interval_spanning_a_trough_reaches_it() {
        let cell = iv(1.4 * core::f64::consts::PI, 1.6 * core::f64::consts::PI);
        assert!(sin(cell).inf() <= -1.0 + LIBM_SLACK);
        let cos_cell = iv(0.9 * core::f64::consts::PI, 1.1 * core::f64::consts::PI);
        assert!(cos(cos_cell).inf() <= -1.0 + LIBM_SLACK);
    }

    #[test]
    fn a_full_period_is_the_whole_range() {
        let cell = iv(0.0, 7.0);
        assert_eq!(sin(cell).inf(), -1.0);
        assert_eq!(sin(cell).sup(), 1.0);
        assert_eq!(cos(cell).inf(), -1.0);
        assert_eq!(cos(cell).sup(), 1.0);
    }

    /// Never wider than the range of the functions, whatever the argument.
    #[test]
    fn results_stay_within_minus_one_to_one() {
        for &x in &[0.0, 1.0, -1.0, 1e3, -1e3, 1e17, f64::MAX] {
            let cell = iv(x, x);
            assert!(sin(cell).inf() >= -1.0 && sin(cell).sup() <= 1.0);
            assert!(cos(cell).inf() >= -1.0 && cos(cell).sup() <= 1.0);
        }
    }

    /// A degenerate cell must give a *narrow* answer, not a lazy `[-1, 1]` —
    /// otherwise every soundness test above would pass on a function that
    /// returns the full range unconditionally.
    #[test]
    fn a_point_argument_is_tight() {
        for &x in &[0.0, 0.5, 1.0, -2.0, 3.0, core::f64::consts::TAU] {
            let s = sin(iv(x, x));
            let c = cos(iv(x, x));
            assert!(s.wid() < 1.0e-9, "sin({x}) too wide: {s:?}");
            assert!(c.wid() < 1.0e-9, "cos({x}) too wide: {c:?}");
            assert_contains(s, x.sin(), "sin point");
            assert_contains(c, x.cos(), "cos point");
        }
    }

    /// BG-ENC-002 convergence, stated as the property rather than as a
    /// threshold: sin is 1-Lipschitz, so the enclosure of a cell can be no
    /// wider than the cell itself, up to the rounding the arithmetic adds.
    /// A fixed target width would only have measured how many times the test
    /// bisected — the first version of this test asserted `< 1e-9` after 20
    /// halvings of a 0.2-wide cell, which cannot reach below 1.9e-7 however
    /// good the implementation is.
    #[test]
    fn enclosure_converges_under_bisection() {
        /// 1-Lipschitz plus room for the reduction's own rounding.
        const LIPSCHITZ_SLACK: f64 = 1.0e-14;
        let mut cell = iv(0.2, 0.4);
        let mut width = sin(cell).wid();
        for _ in 0..30 {
            cell = iv(cell.inf(), cell.mid());
            let next = sin(cell).wid();
            assert!(
                next <= width + LIPSCHITZ_SLACK,
                "width grew: {next} > {width}"
            );
            assert!(
                next <= cell.wid() + LIPSCHITZ_SLACK,
                "enclosure {next} wider than its 1-Lipschitz cell {}",
                cell.wid()
            );
            width = next;
        }
        assert!(width < 1.0e-9, "did not converge: {width}");
    }

    /// The Pythagorean identity is an independent check on both functions at
    /// once: it fails if either is wrong, and it does not go through `f64::sin`.
    #[test]
    fn pythagorean_identity_holds_on_points() {
        for i in -50..50 {
            let x = (i as f64) * 0.37;
            let s = sin(iv(x, x));
            let c = cos(iv(x, x));
            let one = s.sqr() + c.sqr();
            assert!(
                one.inf() <= 1.0 && one.sup() >= 1.0,
                "sin^2 + cos^2 = {one:?} does not contain 1 at x = {x}"
            );
        }
    }

    /// A dense sweep across many periods, deliberately including the
    /// quadrant boundaries where the reduction changes which series is used
    /// and which sign it takes. A sign error in the quadrant table, or an
    /// off-by-one in the reduction, survives every hand-picked cell above and
    /// dies here.
    #[test]
    fn dense_sweep_over_quadrant_boundaries() {
        const STEPS: i32 = 4000;
        for i in -STEPS..STEPS {
            // Step by a fraction of pi/2 so the samples land on, and just
            // either side of, every reduction boundary in the range.
            let x = (i as f64) * (core::f64::consts::FRAC_PI_2 / 7.0);
            let s = sin(iv(x, x));
            let c = cos(iv(x, x));
            assert_contains(s, x.sin(), "sin sweep");
            assert_contains(c, x.cos(), "cos sweep");
            assert!(s.wid() < 1.0e-9, "sin({x}) too wide: {s:?}");
            assert!(c.wid() < 1.0e-9, "cos({x}) too wide: {c:?}");
        }
    }

    #[test]
    fn empty_in_empty_out() {
        assert!(sin(Interval::EMPTY).is_empty());
        assert!(cos(Interval::EMPTY).is_empty());
    }
}
