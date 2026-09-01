//! Conformance tests for the Phase-1 D2 hull primitive (BG-CK-P1-HULL): the
//! public 1-D and tensor hull kernels, the pure `f64` derivative transforms,
//! and the refusal vocabulary. The test names are the contract.
//!
//! The curve form (`hull_curve_homogeneous`) needs a `RationalBezierSpan2`,
//! whose constructor is `pub(crate)`; there is no public construction path, so
//! its conformance lives in the in-module `#[cfg(test)]` suite inside
//! `src/hull.rs` (same-crate access, the crate's existing pattern).

#![deny(clippy::unwrap_used)]

use truck_certified::hull::{
    bernstein_derivative_1d, bernstein_derivative_2d, hull_bernstein_1d, hull_bernstein_2d,
    HullRefusal,
};

/// Plain `f64` de Casteljau point evaluation, the test's reference arithmetic.
fn eval_bernstein_1d(coeffs: &[f64], u: f64) -> f64 {
    let mut level: Vec<f64> = coeffs.to_vec();
    while level.len() > 1 {
        level = level
            .windows(2)
            .map(|w| (1.0 - u) * w[0] + u * w[1])
            .collect();
    }
    level[0]
}

/// Plain `f64` tensor-Bernstein point evaluation: the polynomial
/// `c[i][j] * B^i_m(s) B^j_n(t)`.
fn eval_bernstein_2d(grid: &[Vec<f64>], s: f64, t: f64) -> f64 {
    let row_evals: Vec<f64> = grid.iter().map(|row| eval_bernstein_1d(row, t)).collect();
    eval_bernstein_1d(&row_evals, s)
}

#[test]
fn hull_contains_point_evaluation_enclosures_on_degenerate_subinterval() {
    // A degree-3 polynomial; the degenerate subinterval `(u, u)` hull contains
    // the point evaluation at `u` (containment is the claim; a slightly-widened
    // match is expected and fine).
    let coeffs = [1.0, -2.0, 3.0, -4.0];
    for u in [0.0, 0.25, 0.5, 0.7, 1.0] {
        let hull = hull_bernstein_1d(&coeffs, (u, u)).expect("a degenerate subinterval is compact");
        let value = eval_bernstein_1d(&coeffs, u);
        assert!(
            hull.lo <= value && value <= hull.hi,
            "degenerate hull at u = {u} must contain the point evaluation"
        );
    }
}

#[test]
fn hull_contains_brute_force_samples_of_the_polynomial() {
    // 1-D: 1000 uniform samples of a degree-4 polynomial lie inside the hull
    // over the sample subbox.
    let coeffs = [1.5, -0.5, 2.0, 0.25, -1.0];
    let (lo, hi) = (0.1, 0.9);
    let hull = hull_bernstein_1d(&coeffs, (lo, hi)).expect("a valid subinterval");
    for i in 0..1000 {
        let u = lo + (hi - lo) * (i as f64) / 999.0;
        let value = eval_bernstein_1d(&coeffs, u);
        assert!(
            hull.lo <= value && value <= hull.hi,
            "1-D sample u = {u} must lie inside the hull"
        );
    }

    // 2-D: 1000 uniform samples of a degree-2 x degree-2 tensor polynomial lie
    // inside the hull over the sample rectangle.
    let grid = vec![
        vec![1.0, 2.0, 3.0],
        vec![2.0, -1.0, 4.0],
        vec![3.0, 0.5, 2.0],
    ];
    let (s_lo, s_hi, t_lo, t_hi) = (0.15, 0.85, 0.1, 0.9);
    let hull = hull_bernstein_2d(&grid, (s_lo, s_hi), (t_lo, t_hi)).expect("a valid rectangle");
    for i in 0..1000 {
        let s = s_lo + (s_hi - s_lo) * (i as f64) / 999.0;
        let t = t_lo + (t_hi - t_lo) * ((i * 7) % 1000) as f64 / 999.0;
        let value = eval_bernstein_2d(&grid, s, t);
        assert!(
            hull.lo <= value && value <= hull.hi,
            "2-D sample (s = {s}, t = {t}) must lie inside the hull"
        );
    }
}

#[test]
fn linear_span_hull_is_the_exact_range() {
    // A linear Bernstein polynomial's hull over any subinterval equals the
    // exact endpoint range up to a few ulps: containment both ways.
    let coeffs = [0.3, 2.7];
    for sub in [
        (0.0, 1.0),
        (0.2, 0.8),
        (0.5, 0.5),
        (0.0, 0.0),
        (0.3, 0.3),
        (0.25, 0.75),
    ] {
        let hull = hull_bernstein_1d(&coeffs, sub).expect("a valid subinterval");
        let b_lo = (1.0 - sub.0) * coeffs[0] + sub.0 * coeffs[1];
        let b_hi = (1.0 - sub.1) * coeffs[0] + sub.1 * coeffs[1];
        let (rlo, rhi) = if b_lo <= b_hi {
            (b_lo, b_hi)
        } else {
            (b_hi, b_lo)
        };
        let slack = 8.0 * f64::EPSILON * (1.0 + rlo.abs().max(rhi.abs()));
        assert!(
            hull.lo <= rlo + slack,
            "hull lower sits at most the slack above the exact range lower"
        ); // H-3
        assert!(
            hull.lo >= rlo - slack,
            "hull lower sits at most the slack below the exact range lower"
        ); // H-3
        assert!(
            hull.hi <= rhi + slack,
            "hull upper sits at most the slack above the exact range upper"
        ); // H-3
        assert!(
            hull.hi >= rhi - slack,
            "hull upper sits at most the slack below the exact range upper"
        ); // H-3
    }
}

#[test]
fn non_compact_subinterval_refuses_domain_not_compact() {
    // Misordered, non-finite, outside-[0,1], empty and non-finite coefficients
    // each refuse the named case.
    let coeffs = [1.0, 2.0, 3.0];
    assert_eq!(
        hull_bernstein_1d(&coeffs, (0.5, 0.2)),
        Err(HullRefusal::DomainNotCompact),
        "misordered subinterval"
    );
    assert_eq!(
        hull_bernstein_1d(&coeffs, (f64::NAN, 0.5)),
        Err(HullRefusal::DomainNotCompact),
        "non-finite bound"
    );
    assert_eq!(
        hull_bernstein_1d(&coeffs, (-0.1, 0.5)),
        Err(HullRefusal::DomainNotCompact),
        "below the unit interval"
    );
    assert_eq!(
        hull_bernstein_1d(&coeffs, (0.5, 1.5)),
        Err(HullRefusal::DomainNotCompact),
        "above the unit interval"
    );
    assert_eq!(
        hull_bernstein_1d(&[], (0.0, 1.0)),
        Err(HullRefusal::DomainNotCompact),
        "empty coefficient list"
    );
    assert_eq!(
        hull_bernstein_1d(&[1.0, f64::INFINITY], (0.0, 1.0)),
        Err(HullRefusal::DomainNotCompact),
        "non-finite coefficient"
    );
    assert_eq!(
        hull_bernstein_2d(&[vec![1.0, 2.0, 3.0], vec![4.0]], (0.0, 1.0), (0.0, 1.0)),
        Err(HullRefusal::DomainNotCompact),
        "ragged grid"
    );
    assert_eq!(
        hull_bernstein_2d(&Vec::<Vec<f64>>::new(), (0.0, 1.0), (0.0, 1.0)),
        Err(HullRefusal::DomainNotCompact),
        "empty grid"
    );
    assert_eq!(
        hull_bernstein_2d(&[vec![1.0, 2.0], vec![3.0, 4.0]], (0.7, 0.3), (0.2, 0.8)),
        Err(HullRefusal::DomainNotCompact),
        "2-D misordered axis"
    );

    // The closed domain boundary is ACCEPTED (inclusive compactness).
    assert!(hull_bernstein_1d(&coeffs, (0.0, 1.0)).is_ok());
    assert!(hull_bernstein_1d(&coeffs, (0.0, 0.0)).is_ok());
    assert!(hull_bernstein_1d(&coeffs, (1.0, 1.0)).is_ok());
    assert!(hull_bernstein_2d(&[vec![1.0], vec![2.0]], (0.0, 1.0), (0.0, 1.0)).is_ok());
}

#[test]
fn non_finite_hull_refuses_enclosure_unavailable() {
    // Coefficients near `f64::MAX` whose de Casteljau intermediates overflow
    // refuse the named case. The packet's `[f64::MAX, f64::MAX]` example sums
    // past overflow under the `(1-u)*a + u*b` node arrangement; the
    // subdivision arrangement (`a + u*(b - a)`, mandated by the exact-linear
    // range conformance test) instead overflows the coefficient difference:
    // `-MAX - MAX` underflows past `f64::MIN` in interval arithmetic, so a
    // strict-interior subinterval yields a non-finite intermediate.
    assert_eq!(
        hull_bernstein_1d(&[f64::MAX, -f64::MAX], (0.4, 0.6)),
        Err(HullRefusal::EnclosureUnavailable),
        "1-D coefficient-difference overflow refuses the enclosure"
    );
    let grid = vec![vec![f64::MAX, -f64::MAX], vec![f64::MAX, -f64::MAX]];
    assert_eq!(
        hull_bernstein_2d(&grid, (0.4, 0.6), (0.4, 0.6)),
        Err(HullRefusal::EnclosureUnavailable),
        "2-D coefficient-difference overflow refuses the enclosure"
    );
}

#[test]
fn derivative_coefficients_match_analytic_quadratic() {
    // p(u) = u^2 + 2u + 3: Bernstein degree-2 coefficients are
    // [c, b/2 + c, a + b + c] = [3, 4, 6]; the derivative coefficients
    // [2*(4-3), 2*(6-4)] = [2, 4] equal the Bernstein coefficients of
    // 2a*u + b = 2u + 2, which are [b, 2a + b] = [2, 4]. Exact in `f64`.
    assert_eq!(bernstein_derivative_1d(&[3.0, 4.0, 6.0]), vec![2.0, 4.0]);
    // Degree-0 input yields the zero polynomial.
    assert_eq!(bernstein_derivative_1d(&[5.0]), vec![0.0]);
    // Degree-1: d * (c1 - c0) with d = 1.
    assert_eq!(bernstein_derivative_1d(&[1.0, 3.0]), vec![2.0]);

    // A case that rounds in `f64`: p(u) = 0.1u^2 + 0.2u + 0.3, whose Bernstein
    // coefficients are not exactly representable.
    let a = 0.1;
    let b = 0.2;
    let c = 0.3;
    let coeffs = [c, b / 2.0 + c, a + b + c];
    let derived = bernstein_derivative_1d(&coeffs);
    let expected = [b, 2.0 * a + b];
    let slack = 8.0 * f64::EPSILON * (1.0 + expected[0].abs().max(expected[1].abs()));
    assert!(
        (derived[0] - expected[0]).abs() <= slack,
        "derivative coefficient 0"
    ); // H-3
    assert!(
        (derived[1] - expected[1]).abs() <= slack,
        "derivative coefficient 1"
    ); // H-3

    // 2-D derivative axes on a bilinear grid: axis 0 differentiates in `s`
    // (rows), axis 1 in `t` (columns).
    let grid = vec![vec![1.0, 3.0], vec![5.0, 7.0]];
    assert_eq!(bernstein_derivative_2d(&grid, 0), vec![vec![4.0, 4.0]]);
    assert_eq!(
        bernstein_derivative_2d(&grid, 1),
        vec![vec![2.0], vec![2.0]]
    );
    // Degree-0 along an axis yields the zero polynomial of the other axis.
    assert_eq!(
        bernstein_derivative_2d(&[vec![1.0, 2.0, 3.0]], 0),
        vec![vec![0.0, 0.0, 0.0]]
    );
    assert_eq!(
        bernstein_derivative_2d(&vec![vec![1.0], vec![2.0], vec![3.0]], 1),
        vec![vec![0.0], vec![0.0], vec![0.0]]
    );
}

#[test]
fn hull_over_subinterval_is_contained_in_hull_over_whole() {
    // Monotonicity of the enclosure under subbox inclusion (containment, not
    // width).
    let coeffs = [1.5, -0.5, 2.0, 0.25, -1.0];
    let whole = hull_bernstein_1d(&coeffs, (0.0, 1.0)).expect("the full unit interval");
    for (lo, hi) in [(0.2, 0.8), (0.0, 0.5), (0.3, 0.3), (0.5, 1.0)] {
        let part = hull_bernstein_1d(&coeffs, (lo, hi)).expect("a valid subinterval");
        assert!(
            whole.lo <= part.lo && part.hi <= whole.hi,
            "1-D containment under subbox inclusion for ({lo}, {hi})"
        );
    }
    let grid = vec![
        vec![1.0, 2.0, 3.0],
        vec![2.0, -1.0, 4.0],
        vec![3.0, 0.5, 2.0],
    ];
    let whole = hull_bernstein_2d(&grid, (0.0, 1.0), (0.0, 1.0)).expect("the full unit square");
    let part = hull_bernstein_2d(&grid, (0.2, 0.8), (0.3, 0.7)).expect("a valid sub-rectangle");
    assert!(
        whole.lo <= part.lo && part.hi <= whole.hi,
        "2-D containment under subbox inclusion"
    );
}

#[test]
fn tensor_patch_hull_contains_corner_and_midpoint_samples() {
    // A bilinear tensor patch's hull over a rectangle contains the four corner
    // values and the center value (containment).
    let grid = vec![vec![1.0, 3.0], vec![5.0, 7.0]];
    let (s_lo, s_hi, t_lo, t_hi) = (0.2, 0.8, 0.1, 0.9);
    let hull = hull_bernstein_2d(&grid, (s_lo, s_hi), (t_lo, t_hi)).expect("a valid rectangle");
    for s in [s_lo, 0.5, s_hi] {
        for t in [t_lo, 0.5, t_hi] {
            let value = eval_bernstein_2d(&grid, s, t);
            assert!(
                hull.lo <= value && value <= hull.hi,
                "tensor sample at (s = {s}, t = {t})"
            );
        }
    }
}

#[test]
fn hull_never_panics_and_never_divides() {
    // The crate-level `#![deny(clippy::unwrap_used)]` mechanically enforces the
    // unwrap half; this source scan pins the same facts in the module text:
    // no `unwrap`, no `expect`, no `panic!`, and no `/` operator in the
    // module's OWN code (the hull never divides — polynomial-only;
    // dehomogenization is the consumer's named F2 composition, never this
    // module's). The in-module test code divides `f64` values only (slopes and
    // coefficient literals), never `CertifiedInterval` values.
    let source = include_str!("../src/hull.rs");
    let stripped: Vec<&str> = source
        .lines()
        .map(|line| line.split("//").next().expect("a line prefix"))
        .collect();
    let module_code: String = stripped
        .iter()
        .take_while(|line| !line.trim().starts_with("#[cfg(test)]"))
        .cloned()
        .collect::<Vec<&str>>()
        .join("\n");
    let code = stripped.join("\n");
    assert!(
        !module_code.contains('/'),
        "hull.rs module code has no `/` operator on any value"
    );
    assert!(!code.contains("unwrap"), "hull.rs text has no unwrap call");
    assert!(!code.contains("expect"), "hull.rs text has no expect call");
    assert!(!code.contains("panic!"), "hull.rs text has no panic");
}
