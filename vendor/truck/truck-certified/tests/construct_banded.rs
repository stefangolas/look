//! The CC-001-BANDED integration tests (seam S3): the certified interval
//! no-pivot banded solve against the CC-000 fixtures, and the Rump / Ogita /
//! Oishi residual fallback.
//!
//! Fixture inputs come from `construct::fixtures`
//! (`banded_cubic_uniform`, `banded_pivot_spans_zero`). Their ground truths
//! are the CC-000 contract and are not bent here: the cubic collocation
//! matrix is the order-`(n+1)` tridiagonal Toeplitz matrix with diagonal `4`
//! and unit off-diagonals, and the refusal fixture's first pivot strictly
//! contains `0`.

#![deny(clippy::unwrap_used)]

use truck_certified::construct::banded::{factor_banded_tp, BandedFactor};
use truck_certified::construct::fixtures as fx;
use truck_certified::construct::refusal::ConstructRefusal;
use truck_certified::construct::residual_solve::residual_solve_dense;
use truck_certified::construct::Interval;

/// Extract the `Ok` of a fallible construction; the fixture data is valid by
/// construction, so the refusal arm is a test-bug panic (never an unwrap).
fn construct<T>(result: Result<T, ConstructRefusal>) -> T {
    match result {
        Ok(value) => value,
        Err(refusal) => panic!("a construction that must succeed was refused: {refusal:?}"),
    }
}

/// Assert a banded factorization refuses exactly `SingularInterpolationSystem`.
fn assert_refuses_singular(result: Result<BandedFactor, ConstructRefusal>) {
    match result {
        Ok(_) => panic!("a singular no-pivot banded system must refuse"),
        Err(refusal) => assert_eq!(refusal, ConstructRefusal::SingularInterpolationSystem),
    }
}

/// A certified interval from explicit lo/hi bounds (test-local helper).
fn iv(lo: f64, hi: f64) -> Interval {
    Interval { lo, hi }
}

/// A certified enclosure of the rational `numerator / 3` (test helper).
fn third_of(numerator: i64) -> Interval {
    match Interval::point(numerator as f64).div(&Interval::point(3.0)) {
        Some(v) => v,
        None => panic!("the certified enclosure of an integer over 3 must divide"),
    }
}

/// The RHS rows used by the enclosure-width test: station values of the
/// quadratic `i²`, the line `i`, the affine `2i + 1`, and the constant `4`
/// (one channel each), scaled by the matrix's implicit `6`.
fn cubic_rhs(size: usize) -> Vec<[Interval; 4]> {
    (0..size)
        .map(|i| {
            let f = i as f64;
            [
                Interval::point(6.0 * f * f),
                Interval::point(6.0 * f),
                Interval::point(6.0 * (2.0 * f + 1.0)),
                Interval::point(24.0),
            ]
        })
        .collect()
}

#[test]
fn banded_uniform_cubic_recovers_known_rational_solution() {
    // The CC-000 fixture: order-5 uniform cubic collocation (tridiagonal
    // Toeplitz, diagonal 4, unit off-diagonals).
    let fixture = construct(fx::banded_cubic_uniform(4));
    let factor = construct(factor_banded_tp(&fixture.bands));
    let size = fixture.size;
    assert_eq!(size, 5);

    // A KNOWN rational solution of the finite order-5 system: the control
    // rows `x_j = k_c * (j + 1) / 3` for per-channel integer scales
    // `k_c = [1, 2, 3, 4]`. Applying the band `M` by exact rational arithmetic
    // gives the integer right-hand side `b = k_c * [2, 4, 6, 8, 8]`, checked
    // below. The denominators of 3 are not dyadic, so the interval solve must
    // enclose genuinely rational values.
    let scales = [1.0_f64, 2.0, 3.0, 4.0];
    let rhs_cols = [2.0_f64, 4.0, 6.0, 8.0, 8.0];
    let rhs: Vec<[Interval; 4]> = (0..size)
        .map(|i| {
            [
                Interval::point(scales[0] * rhs_cols[i]),
                Interval::point(scales[1] * rhs_cols[i]),
                Interval::point(scales[2] * rhs_cols[i]),
                Interval::point(scales[3] * rhs_cols[i]),
            ]
        })
        .collect();

    // Verify the fixture matrix really maps the known solution to `rhs`
    // (exact rational check in integers: the collocation band is 4/1/1).
    for i in 0..size {
        let mut expected = [0_i64; 4];
        for c in 0..4 {
            let mut num = 0_i64;
            for j in 0..size {
                let band = if i == j {
                    4
                } else if i.abs_diff(j) == 1 {
                    1
                } else {
                    0
                };
                num += band * scales[c] as i64 * (j as i64 + 1);
            }
            expected[c] = num / 3;
        }
        assert_eq!(expected[0], rhs_cols[i] as i64);
        assert_eq!(expected[1], 2 * rhs_cols[i] as i64);
        assert_eq!(expected[2], 3 * rhs_cols[i] as i64);
        assert_eq!(expected[3], 4 * rhs_cols[i] as i64);
    }

    let rows = construct(factor.solve_homogeneous(&rhs));
    assert_eq!(rows.len(), size);

    for j in 0..size {
        for c in 0..4 {
            // The exact rational control value k_c * (j + 1) / 3 must lie in
            // the delivered enclosure. vx is a certified enclosure of it.
            let numerator = scales[c] as i64 * (j as i64 + 1);
            let vx = third_of(numerator);
            assert!(
                rows[j][c].lo <= vx.lo && vx.hi <= rows[j][c].hi,
                "control {j} channel {c} must enclose the rational \
                 k_c*(j+1)/3 = {numerator}/3: enclosure [{}, {}], \
                 certified value [{}, {}]",
                rows[j][c].lo,
                rows[j][c].hi,
                vx.lo,
                vx.hi
            );
        }
    }

    // The 1/3 divisions are not dyadic: every enclosure is genuinely open to
    // rounding, so the delivered L2 enclosure width is positive.
    assert!(factor.max_control_error() > 0.0);
}

#[test]
fn pivot_containing_zero_refuses_singular_interpolation_system() {
    // The CC-000 refusal fixture: a 2×2 banded system whose first diagonal
    // pivot strictly contains 0. The no-pivot elimination must refuse on it,
    // never swapping, never retrying, never widening.
    let fixture = construct(fx::banded_pivot_spans_zero());
    assert_refuses_singular(factor_banded_tp(&fixture.bands));
}

#[test]
fn ill_conditioned_non_tp_fixture_refuses_never_pivots() {
    // The matrix [[1,1,0],[1,1,1],[0,1,1]] is nonsingular (det −1) but is NOT
    // totally positive: its no-pivot Schur pivot at (1,1) collapses to an
    // interval containing 0. A row exchange (or a retry in another order)
    // could continue — the class-specific guarantee is exactly that the
    // algorithm NEVER pivots. It must refuse rather than deviate from the
    // no-pivot order.
    let bands = [
        iv(1.0, 1.0),
        iv(1.0, 1.0),
        iv(0.0, 0.0),
        iv(1.0, 1.0),
        iv(1.0, 1.0),
        iv(1.0, 1.0),
        iv(0.0, 0.0),
        iv(1.0, 1.0),
        iv(1.0, 1.0),
    ];
    assert_refuses_singular(factor_banded_tp(&bands));
}

#[test]
fn enclosure_width_shrinks_with_input_width() {
    // One shared factorization; the delivered enclosure width is a monotone
    // function of the right-hand-side width. A widened RHS (each coordinate
    // spread to ±0.5 around the same centre) must deliver a strictly wider
    // control enclosure than the exact point RHS.
    let fixture = construct(fx::banded_cubic_uniform(4));
    let factor = construct(factor_banded_tp(&fixture.bands));
    let size = fixture.size;

    let tight = cubic_rhs(size);
    let wide: Vec<[Interval; 4]> = tight
        .iter()
        .map(|row| {
            [
                iv(row[0].lo - 0.5, row[0].hi + 0.5),
                iv(row[1].lo - 0.5, row[1].hi + 0.5),
                iv(row[2].lo - 0.5, row[2].hi + 0.5),
                iv(row[3].lo - 0.5, row[3].hi + 0.5),
            ]
        })
        .collect();

    let _tight_rows = construct(factor.solve_homogeneous(&tight));
    let tight_error = factor.max_control_error();
    let _wide_rows = construct(factor.solve_homogeneous(&wide));
    let wide_error = factor.max_control_error();

    assert!(
        wide_error > tight_error,
        "wider input enclosures must deliver a wider control enclosure: \
         tight eps = {tight_error}, wide eps = {wide_error}"
    );
}

#[test]
fn rump_residual_certifies_when_eta_below_one() {
    // A well-conditioned 2×2 system A = [[2,1],[1,2]] with the exact adjugate
    // preconditioner R = A⁻¹ (float-rounded). η = ‖I − R·A‖_∞ is ~1e-16,
    // far below 1, so the residual solve certifies and the returned enclosure
    // must contain the exact integer solution [1, 2] of b = A·[1,2].
    let a = [
        [Interval::point(2.0), Interval::point(1.0)],
        [Interval::point(1.0), Interval::point(2.0)],
    ];
    let r_inv = [[2.0 / 3.0, -1.0 / 3.0], [-1.0 / 3.0, 2.0 / 3.0]];
    let x_hat = [
        r_inv[0][0] * 4.0 + r_inv[0][1] * 5.0,
        r_inv[1][0] * 4.0 + r_inv[1][1] * 5.0,
    ];
    let b = [Interval::point(4.0), Interval::point(5.0)];

    let enclosure = construct(residual_solve_dense(&a, &r_inv, &x_hat, &b));
    assert!(
        enclosure[0].contains(1.0),
        "channel 0 must enclose the exact solution 1: [{}, {}]",
        enclosure[0].lo,
        enclosure[0].hi
    );
    assert!(
        enclosure[1].contains(2.0),
        "channel 1 must enclose the exact solution 2: [{}, {}]",
        enclosure[1].lo,
        enclosure[1].hi
    );
}

#[test]
fn rump_refuses_conditioning_below_threshold_when_eta_at_or_above_one() {
    // η ≥ 1 refuses. The zero preconditioner leaves E = I − 0 = I, so
    // η = ‖I‖_∞ = 1 exactly ("at" one).
    let a = [
        [Interval::point(2.0), Interval::point(1.0)],
        [Interval::point(1.0), Interval::point(2.0)],
    ];
    let zero_r = [[0.0_f64, 0.0], [0.0, 0.0]];
    let x_hat = [0.0_f64, 0.0];
    let b = [Interval::point(4.0), Interval::point(5.0)];
    match residual_solve_dense(&a, &zero_r, &x_hat, &b) {
        Err(ConstructRefusal::ConditioningBelowThreshold) => {}
        Ok(_) => panic!("η = 1 must refuse the residual solve"),
        Err(other) => panic!("wrong refusal for η = 1: {other:?}"),
    }

    // A grossly wrong preconditioner drives η strictly above one ("above").
    let bad_r = [[3.0_f64, 0.0], [0.0, 3.0]];
    match residual_solve_dense(&a, &bad_r, &x_hat, &b) {
        Err(ConstructRefusal::ConditioningBelowThreshold) => {}
        Ok(_) => panic!("η > 1 must refuse the residual solve"),
        Err(other) => panic!("wrong refusal for η > 1: {other:?}"),
    }
}
