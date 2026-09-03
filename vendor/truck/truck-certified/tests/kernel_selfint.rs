//! BG-KV2-404-S8 integration tests: the §13 R6 self-intersection machinery —
//! the deflated divided-difference residual on the Bézier net, the §7.1
//! numerator form for rational leaves, the Theorem 13.1 exact cover, the
//! Theorem 13.3 chart transitions, the Theorem 13.4 λ = 0 routing, and the
//! self-overlapping-sweep / valid-patch fixture exclusions.
//!
//! **H-1.** Like the module it tests, this file carries the crate's unwrap
//! discipline: no `unwrap`, no `expect`, no `panic!`, and no module-level
//! `allow`.

#![deny(clippy::unwrap_used)]

use truck_certified::kernel::evidence::{RefusalEvidence, RefusalKind, VerdictClass};
use truck_certified::kernel::graph::SegmentBreak;
use truck_certified::kernel::leaf::BezierLeaf;
use truck_certified::kernel::selfint::{
    bernstein_eval_2d, divided_difference_u, divided_difference_v, r6_admits, r6_lambda_zero,
    r6_lambda_zero_refusal, r6_transition_type1, r6_transition_type2, r6_witness,
    r6_witness_checked, shift_v, Chart, ChartChoice, LambdaZeroRoute, R6System,
};

/// The R6 fixture/detection agreement tolerance on the unit-domain leaves.
const GT_TOL: f64 = 1e-9; // H-3: dyadic R6 witness comparison tolerance
/// The R6 zero-detection threshold on the unit-domain leaves.
const DETECT_TOL: f64 = 1e-7; // H-3: R6 residual zero-detection threshold

/// Extract the `Ok` of any fallible construction; the fixture data is valid by
/// construction, so the refusal arm is a test-bug assertion (never an unwrap).
macro_rules! ok_or_fail {
    ($result:expr) => {{
        let result = $result;
        match result {
            Ok(value) => value,
            Err(_) => {
                assert!(false, "a construction that must succeed was refused");
                return;
            }
        }
    }};
}

/// Extract the `Some` of an evaluation; the fixture data always evaluates.
macro_rules! some_or_fail {
    ($option:expr) => {{
        let option = $option;
        match option {
            Some(value) => value,
            None => {
                assert!(false, "an evaluation that must succeed returned None");
                return;
            }
        }
    }};
}

fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() <= GT_TOL
}

fn approx3(a: [f64; 3], b: [f64; 3]) -> bool {
    approx(a[0], b[0]) && approx(a[1], b[1]) && approx(a[2], b[2])
}

fn approx2(a: [f64; 2], b: [f64; 2]) -> bool {
    approx(a[0], b[0]) && approx(a[1], b[1])
}

/// The `comp`-coordinate coefficient grid of a leaf, rows over `u` and columns
/// over `v` (the layout the divided-difference nets consume).
fn grid_of(leaf: &BezierLeaf, comp: usize) -> Vec<Vec<f64>> {
    let width = leaf.degree_v + 1;
    (0..=leaf.degree_u)
        .map(|i| {
            (0..=leaf.degree_v)
                .map(|j| leaf.control[i * width + j][comp])
                .collect()
        })
        .collect()
}

/// One homogeneous coordinate value of the leaf at `(u, v)`.
fn coord_value(leaf: &BezierLeaf, comp: usize, uv: [f64; 2]) -> Option<f64> {
    bernstein_eval_2d(&grid_of(leaf, comp), uv[0], uv[1])
}

/// The affine surface point `S(u, v) = P / w` of a (possibly rational) leaf.
fn point(leaf: &BezierLeaf, uv: [f64; 2]) -> Option<[f64; 3]> {
    let w = coord_value(leaf, 3, uv)?;
    let mut out = [0.0; 3];
    for comp in 0..3 {
        out[comp] = coord_value(leaf, comp, uv)? / w;
    }
    Some(out)
}

// ---------------------------------------------------------------------------
// Fixture leaves
// ---------------------------------------------------------------------------

/// The shared `v`-profile of the self-overlapping sweep leaves: a folded
/// extrusion profile `(y, z)(v)` with the exact self-crossing `y = 1/16, z = 0`
/// at `v = 1/4` and `v = 3/4`. The profile coefficients are the degree-3
/// Bernstein nets of `y = (v − 1/2)²` and `z = −(v − 1/2)/16 + (v − 1/2)³`.
fn profile_y() -> [f64; 4] {
    [0.25, -1.0 / 12.0, -1.0 / 12.0, 0.25]
}

fn profile_z() -> [f64; 4] {
    [-3.0 / 32.0, 13.0 / 96.0, -13.0 / 96.0, 3.0 / 32.0]
}

/// The extrusion coordinate of a sweep leaf: `x = u`, `x = u + v`, or
/// `x = u − v`. The offset between the two parameter points of a fold pair is
/// then vertical (chart B, `m = 0`), anti-diagonal (chart A, `m = −1`), or
/// diagonal (chart A, `m = +1`).
#[derive(Clone, Copy)]
enum FoldMode {
    /// `x = u`: fold pairs at offset `(0, 1/2)`, chart B.
    Vert,
    /// `x = u + v`: fold pairs at offset `(1/2, −1/2)`, chart A `m = −1`.
    Plus,
    /// `x = u − v`: fold pairs at offset `(1/2, 1/2)`, chart A `m = +1`.
    Minus,
}

/// A hand-built self-overlapping sweep leaf at bidegree `(1, 3)`: the unit
/// weight, `x` an extrusion coordinate linear in `u`, and the degree-3 profile
/// `(y, z)(v)` crossing itself at `v = 1/4` and `v = 3/4`.
///
/// The image is the x-extrusion of the self-crossing profile, so the leaf has
/// a genuine transversal self-intersection (the two profile tangents at the
/// crossing are independent, so the tangent planes span `R³`). Each unordered
/// self-pair sits at known parameter points with a known dyadic offset.
fn fold_leaf(mode: FoldMode) -> truck_certified::kernel::evidence::Construction<BezierLeaf> {
    let y = profile_y();
    let z = profile_z();
    // Degree-3 Bernstein coefficients of `v` (used to realize `x = u ± v`).
    let b = [0.0, 1.0 / 3.0, 2.0 / 3.0, 1.0];
    // Degree-1 Bernstein coefficients of `u`.
    let a = [0.0, 1.0];
    let mut control = Vec::with_capacity(8);
    for i in 0..=1 {
        for j in 0..=3 {
            let x = match mode {
                FoldMode::Vert => a[i],
                FoldMode::Plus => a[i] + b[j],
                FoldMode::Minus => a[i] - b[j],
            };
            control.push([x, y[j], z[j], 1.0]);
        }
    }
    BezierLeaf::try_new(1, 3, control)
}

/// A unit-weight bidegree `(3, 3)` polynomial net with dyadic control data,
/// used to machine-check the deflated divided differences (the leaf is
/// generic: the polynomial identities tested do not depend on its shape).
fn cubic_leaf() -> truck_certified::kernel::evidence::Construction<BezierLeaf> {
    let mut control = Vec::with_capacity(16);
    for i in 0..=3 {
        for j in 0..=3 {
            let x = i as f64 + 2.0 * j as f64 + 0.5 * (i as f64) * (j as f64);
            let y = 0.5 * (i as f64) * (i as f64) + 0.25 * (j as f64) * (j as f64) * (j as f64);
            let z = 0.125 * (i as f64) * (i as f64) * (j as f64)
                + 0.25 * (i as f64) * (j as f64) * (j as f64);
            control.push([x, y, z, 1.0]);
        }
    }
    BezierLeaf::try_new(3, 3, control)
}

/// The unit-weight bilinear plane `S(u, v) = (u, v, 0)` — a valid patch with
/// no self-intersections.
fn plane_leaf() -> truck_certified::kernel::evidence::Construction<BezierLeaf> {
    let control = vec![
        [0.0, 0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0, 1.0],
        [1.0, 0.0, 0.0, 1.0],
        [1.0, 1.0, 0.0, 1.0],
    ];
    BezierLeaf::try_new(1, 1, control)
}

/// A rational sphere-chart leaf: the stereographic cap
/// `(x, y, z) = (2u, 2v, 1 − u² − v²) / (1 + u² + v²)` at bidegree `(2, 2)`,
/// a regular, injective (hence self-intersection-free) spherical patch with
/// non-unit weights.
fn sphere_cap_leaf() -> truck_certified::kernel::evidence::Construction<BezierLeaf> {
    let x = [[0.0, 0.0, 0.0], [1.0, 1.0, 1.0], [2.0, 2.0, 2.0]];
    let y = [[0.0, 1.0, 2.0], [0.0, 1.0, 2.0], [0.0, 1.0, 2.0]];
    let z = [[1.0, 1.0, 0.0], [1.0, 1.0, 0.0], [0.0, 0.0, -1.0]];
    let w = [[1.0, 1.0, 2.0], [1.0, 1.0, 2.0], [2.0, 2.0, 3.0]];
    let mut control = Vec::with_capacity(9);
    for i in 0..=2 {
        for j in 0..=2 {
            control.push([x[i][j], y[i][j], z[i][j], w[i][j]]);
        }
    }
    BezierLeaf::try_new(2, 2, control)
}

/// A unit-weight bidegree `(1, 2)` leaf whose `v`-profile has a stationary
/// point at `v = 1/2`: `S(u, v) = (u, v² − v, v² − v)`, so `S_v = 0` at
/// `(u, 1/2)` — a parametric degeneracy for the λ = 0 routing test.
fn degenerate_leaf() -> truck_certified::kernel::evidence::Construction<BezierLeaf> {
    let mut control = Vec::with_capacity(6);
    for i in 0..=1 {
        for j in 0..=2 {
            let b = match j {
                0 => 0.0,
                1 => -0.5,
                _ => 0.0,
            };
            control.push([i as f64, b, b, 1.0]);
        }
    }
    BezierLeaf::try_new(1, 2, control)
}

// ---------------------------------------------------------------------------
// Section 1: the deflated residual
// ---------------------------------------------------------------------------

#[test]
fn deflated_divided_differences_are_polynomial_on_bezier_net() {
    let leaf = ok_or_fail!(cubic_leaf());
    // Dyadic steps: the scalar divisions are exact in binary floating point.
    let steps = [
        [0.25, 0.25],
        [0.25, -0.125],
        [-0.125, 0.25],
        [0.0625, 0.125],
        [-0.25, -0.25],
    ];
    let samples = [[0.25, 0.25], [0.5, 0.375], [0.75, 0.625], [0.375, 0.75]];
    for [h, k] in steps {
        // The dimension fingerprint of the deflation: the u divided difference
        // drops the u degree by one, the v divided difference drops the v
        // degree by one — the nets are POLYNOMIAL, not sampled differences.
        let g0 = grid_of(&leaf, 0);
        let shifted_v = ok_or_fail!(shift_v(&g0, k));
        let d1_shape = ok_or_fail!(divided_difference_u(&shifted_v, h));
        let d2_shape = ok_or_fail!(divided_difference_v(&g0, k));
        assert_eq!(d1_shape.len(), leaf.degree_u, "D1 u-degree drops to m-1");
        assert_eq!(
            d1_shape[0].len(),
            leaf.degree_v + 1,
            "D1 v-degree unchanged"
        );
        assert_eq!(d2_shape.len(), leaf.degree_u + 1, "D2 u-degree unchanged");
        assert_eq!(d2_shape[0].len(), leaf.degree_v, "D2 v-degree drops to n-1");
        for [u, v] in samples {
            for comp in 0..3 {
                let grid = grid_of(&leaf, comp);
                let s_uv = some_or_fail!(coord_value(&leaf, comp, [u, v]));
                let s_u_vk = some_or_fail!(coord_value(&leaf, comp, [u, v + k]));
                let s_uh_vk = some_or_fail!(coord_value(&leaf, comp, [u + h, v + k]));
                // D1 = [S(u+h, v+k) - S(u, v+k)] / h, as a net over (u, v).
                let shifted_v = ok_or_fail!(shift_v(&grid, k));
                let d1 = ok_or_fail!(divided_difference_u(&shifted_v, h));
                let d1v = some_or_fail!(bernstein_eval_2d(&d1, u, v));
                let fd1 = (s_uh_vk - s_u_vk) / h;
                assert!(
                    approx(d1v, fd1),
                    "D1 is the polynomial divided difference: net {d1v} vs finite difference {fd1} \
                     at ({u}, {v}) with steps ({h}, {k}), coordinate {comp}"
                );
                // D2 = [S(u, v+k) - S(u, v)] / k, as a net over (u, v).
                let d2 = ok_or_fail!(divided_difference_v(&grid, k));
                let d2v = some_or_fail!(bernstein_eval_2d(&d2, u, v));
                let fd2 = (s_u_vk - s_uv) / k;
                assert!(
                    approx(d2v, fd2),
                    "D2 is the polynomial divided difference: net {d2v} vs finite difference {fd2} \
                     at ({u}, {v}) with steps ({h}, {k}), coordinate {comp}"
                );
                // Telescoping identity: h·D1 + k·D2 == S(u+h, v+k) - S(u, v).
                let identity = h * d1v + k * d2v;
                assert!(
                    approx(identity, s_uh_vk - s_uv),
                    "the deflated residual telescopes: {identity} vs {} at ({u}, {v}) with steps \
                     ({h}, {k}), coordinate {comp}",
                    s_uh_vk - s_uv
                );
            }
        }
    }
}

#[test]
fn rational_deflated_residual_uses_numerator_form() {
    // A genuinely rational leaf (the sphere cap has non-unit weights), so the
    // numerator form and the naive weightless difference disagree.
    let leaf = ok_or_fail!(sphere_cap_leaf());
    let sys = ok_or_fail!(R6System::try_new(&leaf));
    let base = [0.25, 0.25];
    let delta = [0.125, 0.25];
    let near = [base[0] + delta[0], base[1] + delta[1]];
    let residual = ok_or_fail!(sys.residual(base, delta));

    let w0 = some_or_fail!(coord_value(&leaf, 3, base));
    let w1 = some_or_fail!(coord_value(&leaf, 3, near));
    for comp in 0..3 {
        let p0 = some_or_fail!(coord_value(&leaf, comp, base));
        let p1 = some_or_fail!(coord_value(&leaf, comp, near));
        // The residual is the §7.1 numerator form
        // P(base+δ)·w(base) − P(base)·w(base+δ), cross-multiplied on the
        // homogeneous nets — never a dehomogenized quotient.
        let numerator = p1 * w0 - p0 * w1;
        assert!(
            approx(residual[comp], numerator),
            "coordinate {comp}: the residual equals the numerator form (module {} vs {numerator})",
            residual[comp]
        );
        // The same value is w(base)·w(base+δ)·(S(base+δ) − S(base)): the
        // identity holds only because the module never divides by a weight.
        let s0 = p0 / w0;
        let s1 = p1 / w1;
        let weighted_gap = w0 * w1 * (s1 - s0);
        assert!(
            approx(residual[comp], weighted_gap),
            "coordinate {comp}: the numerator form carries the weight product \
             (module {} vs {weighted_gap})",
            residual[comp]
        );
        // The residual is NOT the weightless difference P(base+δ) − P(base).
        let weightless = p1 - p0;
        assert!(
            !approx(residual[comp], weightless),
            "coordinate {comp}: the module must cross-multiply, not subtract numerators \
             weightlessly"
        );
    }

    // The trivial diagonal δ = 0 is a zero of the numerator form.
    let diagonal = ok_or_fail!(sys.residual(base, [0.0, 0.0]));
    for comp in 0..3 {
        assert!(
            approx(diagonal[comp], 0.0),
            "the diagonal is the trivial zero"
        );
    }

    // A genuine non-pair offset is not a zero of the numerator form.
    let norm_sq = residual[0] * residual[0] + residual[1] * residual[1] + residual[2] * residual[2];
    assert!(
        norm_sq > DETECT_TOL * DETECT_TOL,
        "the non-pair offset residual must not vanish (norm^2 {norm_sq})"
    );
}

// ---------------------------------------------------------------------------
// Section 2: the exact cover and the transitions
// ---------------------------------------------------------------------------

#[test]
fn exact_cover_thm_13_1_admits_exactly_one_of_delta_minus_delta() {
    let cases: [[f64; 2]; 20] = [
        [1.0, 0.0],
        [0.0, 1.0],
        [-1.0, 0.0],
        [0.0, -1.0],
        [1.0, 1.0],
        [-1.0, 1.0],
        [1.0, -1.0],
        [-1.0, -1.0],
        [3.0, 1.0],
        [-3.0, 1.0],
        [1.0, 3.0],
        [-1.0, -3.0],
        [0.5, 0.0],
        [-0.5, 0.0],
        [0.0, 0.5],
        [0.0, -0.5],
        [0.25, 0.75],
        [-0.25, 0.75],
        [0.125, 0.25],
        [0.375, -0.5],
    ];
    for delta in cases {
        let neg = [-delta[0], -delta[1]];
        let admit_delta = r6_admits(delta);
        let admit_neg = r6_admits(neg);
        assert!(
            admit_delta != admit_neg,
            "exactly one of {{delta, -delta}} = {delta:?} is admissible"
        );
        // Both antipodes map to the SAME canonical witness: no double counting.
        assert_eq!(
            r6_witness(delta),
            r6_witness(neg),
            "delta {delta:?} and -delta must map to the same canonical witness"
        );
        // The cover is decided by the magnitudes (Theorem 13.1).
        let admitted = if admit_delta { delta } else { neg };
        let choice = r6_witness_checked(admitted);
        let choice = ok_or_fail!(choice);
        let expected = if admitted[0].abs() >= admitted[1].abs() {
            Chart::A
        } else {
            Chart::B
        };
        assert_eq!(
            choice.chart(),
            expected,
            "cover for admitted offset {admitted:?}"
        );
        assert!(
            approx2(choice.offset(), admitted),
            "the canonical chart data reconstructs the admitted offset {admitted:?}: {:?}",
            choice.offset()
        );
    }
    // The zero offset is admitted by neither antipode and has no chart.
    assert!(!r6_admits([0.0, 0.0]));
    assert_eq!(r6_witness([0.0, 0.0]), r6_witness([0.0, 0.0]));
    match r6_witness_checked([0.0, 0.0]) {
        Err(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::ClaimRefuted);
            match &refusal.evidence {
                RefusalEvidence::Predicate { name, .. } => {
                    assert_eq!(*name, "r6_witness_zero_offset");
                }
                _ => assert!(
                    false,
                    "the zero-offset refusal must carry predicate evidence"
                ),
            }
        }
        Ok(_) => assert!(
            false,
            "the zero offset must be refused by r6_witness_checked"
        ),
    }
    // Non-finite offsets are never admissible.
    assert!(!r6_admits([f64::NAN, 0.0]));
    assert!(!r6_admits([f64::INFINITY, 1.0]));
}

#[test]
fn chart_transition_type_1_preserves_the_witness() {
    // The x = u − v sweep leaf has its fold pair at (1/4, 1/4) -> (3/4, 3/4),
    // an exact diagonal offset (1/2, 1/2): chart A at the m = +1 seam.
    let leaf = ok_or_fail!(fold_leaf(FoldMode::Minus));
    let sys = ok_or_fail!(R6System::try_new(&leaf));
    let base = [0.25, 0.25];
    let far = [0.75, 0.75];
    // Ground truth: the base is a genuine self-pair base.
    let p_base = some_or_fail!(point(&leaf, base));
    let p_far = some_or_fail!(point(&leaf, far));
    assert!(
        approx3(p_base, p_far),
        "the fixture fold maps {base:?} and {far:?} to the same surface point"
    );

    let chart_a = ChartChoice::A {
        lambda: 0.5,
        m: 1.0,
    };
    let transition = ok_or_fail!(r6_transition_type1(base, chart_a));
    assert_eq!(transition.break_kind, SegmentBreak::R6ChartSwitch);
    // Type I keeps the base and re-parameterizes into chart B.
    assert_eq!(transition.base, base, "Type I keeps the base point");
    let chart_b = transition.choice;
    assert_eq!(chart_b.chart(), Chart::B);
    assert!(
        approx(chart_b.m(), 1.0 / chart_a.m()),
        "Type I: m_B = 1/m_A"
    );
    assert!(
        approx(chart_b.lambda(), chart_a.lambda() * chart_a.m()),
        "Type I: lambda_B = lambda_A * m_A"
    );
    // The two sides encode the SAME ordered offset from the same base, so the
    // far point of the witness is identical across the break (Corollary 13.2).
    assert!(
        approx2(chart_a.offset(), chart_b.offset()),
        "chart A and chart B encode the same offset"
    );
    let far_b = [
        transition.base[0] + chart_b.offset()[0],
        transition.base[1] + chart_b.offset()[1],
    ];
    assert!(
        approx2(far_b, far),
        "the B-side far point is the A-side far point"
    );
    let p_far_b = some_or_fail!(point(&leaf, far_b));
    assert!(
        approx3(p_far_b, p_base),
        "witness preserved across the R6ChartSwitch break"
    );
    let zero_residual = ok_or_fail!(sys.residual(base, chart_b.offset()));
    for comp in 0..3 {
        assert!(
            approx(zero_residual[comp], 0.0),
            "the B-side offset is a genuine self-pair offset of the fixture"
        );
    }

    // The seam refuses non-seam inputs: chart B is not a Type I start, and a
    // chart-A slope away from +1 is not the seam.
    match r6_transition_type1(
        base,
        ChartChoice::B {
            lambda: 0.5,
            m: 1.0,
        },
    ) {
        Err(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::ClaimRefuted);
            match &refusal.evidence {
                RefusalEvidence::Predicate { name, .. } => {
                    assert_eq!(*name, "r6_type1_requires_chart_a");
                }
                _ => assert!(false, "the non-A start must carry predicate evidence"),
            }
        }
        Ok(_) => assert!(false, "Type I must refuse a chart-B start"),
    }
    match r6_transition_type1(
        base,
        ChartChoice::A {
            lambda: 0.5,
            m: 0.5,
        },
    ) {
        Err(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::ClaimRefuted);
            match &refusal.evidence {
                RefusalEvidence::Predicate { name, .. } => {
                    assert_eq!(*name, "r6_type1_requires_m_plus_one");
                }
                _ => assert!(false, "the non-seam slope must carry predicate evidence"),
            }
        }
        Ok(_) => assert!(false, "Type I must refuse a slope away from m = +1"),
    }
}

#[test]
fn base_point_swap_type_2_preserves_the_witness() {
    // The x = u + v sweep leaf has its fold pair at (1/4, 3/4) -> (3/4, 1/4),
    // an exact anti-diagonal offset (1/2, -1/2): chart A at the m = -1 seam.
    let leaf = ok_or_fail!(fold_leaf(FoldMode::Plus));
    let sys = ok_or_fail!(R6System::try_new(&leaf));
    let base = [0.25, 0.75];
    let far = [0.75, 0.25];
    let p_base = some_or_fail!(point(&leaf, base));
    let p_far = some_or_fail!(point(&leaf, far));
    assert!(
        approx3(p_base, p_far),
        "the fixture fold maps {base:?} and {far:?} to the same surface point"
    );

    let chart_a = ChartChoice::A {
        lambda: 0.5,
        m: -1.0,
    };
    assert!(approx2(chart_a.offset(), [0.5, -0.5]));
    let transition = ok_or_fail!(r6_transition_type2(base, chart_a));
    assert_eq!(transition.break_kind, SegmentBreak::R6BaseSwap);
    // Type II swaps the base to the far member of the unordered pair.
    assert!(
        approx2(transition.base, far),
        "the base point moves to the far member {:?}, got {:?}",
        far,
        transition.base
    );
    // The far-side chart data points back from the new base to the old base,
    // carrying the SAME unordered pair.
    let back = [
        transition.base[0] + transition.choice.offset()[0],
        transition.base[1] + transition.choice.offset()[1],
    ];
    assert!(
        approx2(back, base),
        "the far-side data points back to the old base"
    );
    // The witness is preserved: the surface point at the swapped base equals
    // the surface point at the original base (both are the fold's image).
    let p_swapped = some_or_fail!(point(&leaf, transition.base));
    assert!(
        approx3(p_swapped, p_base),
        "witness preserved across the R6BaseSwap break"
    );
    let zero_residual = ok_or_fail!(sys.residual(transition.base, transition.choice.offset()));
    for comp in 0..3 {
        assert!(
            approx(zero_residual[comp], 0.0),
            "the far-side offset is a genuine self-pair offset of the fixture"
        );
    }

    // The seam refuses non-seam inputs.
    match r6_transition_type2(
        base,
        ChartChoice::A {
            lambda: 0.5,
            m: 1.0,
        },
    ) {
        Err(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::ClaimRefuted);
            match &refusal.evidence {
                RefusalEvidence::Predicate { name, .. } => {
                    assert_eq!(*name, "r6_type2_requires_m_minus_one");
                }
                _ => assert!(false, "the non-seam slope must carry predicate evidence"),
            }
        }
        Ok(_) => assert!(false, "Type II must refuse a slope away from m = -1"),
    }
}

// ---------------------------------------------------------------------------
// Section 3: the λ = 0 stratum and the fixtures
// ---------------------------------------------------------------------------

#[test]
fn lambda_zero_routes_to_chart_or_carrier_not_classifier() {
    // Regular leaves: at λ = 0 the residual reduces to S_u + m S_v, which has
    // no zero on a regular leaf — the routing is back into the charts.
    let leaf = ok_or_fail!(fold_leaf(FoldMode::Vert));
    let base = [0.5, 0.25];
    for m in [-1.0, 0.0, 1.0] {
        let route = ok_or_fail!(r6_lambda_zero(&leaf, base, m));
        match route {
            LambdaZeroRoute::Chart => {}
            LambdaZeroRoute::Carrier => {
                assert!(
                    false,
                    "a regular leaf must route the λ = 0 stratum to the charts"
                )
            }
        }
    }
    // A slope outside chart A's |m| <= 1 refuses.
    match r6_lambda_zero(&leaf, base, 2.0) {
        Err(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::ClaimRefuted);
            match &refusal.evidence {
                RefusalEvidence::Predicate { name, .. } => {
                    assert_eq!(*name, "r6_lambda_zero_m_outside_chart_a");
                }
                _ => assert!(
                    false,
                    "the out-of-chart slope must carry predicate evidence"
                ),
            }
        }
        Ok(_) => assert!(false, "a chart-B slope is not a chart-A λ = 0 datum"),
    }

    // A leaf with a parametric degeneracy at the base routes to the CARRIER:
    // the stratum S_u + m S_v = 0 is only solvable at that degeneracy, and the
    // leaf owns it — never the contact classifier.
    let degenerate = ok_or_fail!(degenerate_leaf());
    let route = ok_or_fail!(r6_lambda_zero(&degenerate, [0.5, 0.5], 0.0));
    match route {
        LambdaZeroRoute::Carrier => {}
        LambdaZeroRoute::Chart => {
            assert!(
                false,
                "a parametric degeneracy must route the stratum to the carrier"
            )
        }
    }

    // A caller that insists on the §3.4 (contact-classifier) route is refused
    // by a typed refusal that names the route and backs Inconclusive.
    let refusal = r6_lambda_zero_refusal(base, 0.0);
    assert_eq!(refusal.kind, RefusalKind::Conditioning);
    assert_eq!(refusal.backing, VerdictClass::Inconclusive);
    match &refusal.evidence {
        RefusalEvidence::Predicate { name, .. } => {
            assert_eq!(*name, "r6_lambda_zero_section_3_4_route");
        }
        _ => assert!(
            false,
            "the λ = 0 route refusal must carry predicate evidence"
        ),
    }
}

/// Scan the dyadic `(base × offset)` probe grid of a leaf and return the
/// offsets from `base` that are R6 zeros (self-pair witnesses) — the trivial
/// zero offset excluded. Offsets whose far endpoint leaves `[0, 1]²` are
/// skipped (the leaf is certified over its unit domain).
fn scan_zero_offsets(
    sys: &R6System,
    base: [f64; 2],
    h_vals: &[f64],
    k_vals: &[f64],
) -> Vec<[f64; 2]> {
    let mut hits = Vec::new();
    for &h in h_vals {
        for &k in k_vals {
            if h == 0.0 && k == 0.0 {
                continue;
            }
            let near = [base[0] + h, base[1] + k];
            if near[0] < 0.0 || near[0] > 1.0 || near[1] < 0.0 || near[1] > 1.0 {
                continue;
            }
            match sys.residual(base, [h, k]) {
                Ok(residual) => {
                    let norm_sq = residual[0] * residual[0]
                        + residual[1] * residual[1]
                        + residual[2] * residual[2];
                    if norm_sq <= DETECT_TOL * DETECT_TOL {
                        hits.push([h, k]);
                    }
                }
                Err(_) => assert!(false, "residual evaluation on the probe grid must succeed"),
            }
        }
    }
    hits
}

#[test]
fn self_overlapping_sweep_detected_on_fixture() {
    // The vertical sweep leaf: fold pairs at (u, 1/4) <-> (u, 3/4), offset
    // (0, 1/2) — a chart-B witness (the v step dominates).
    let leaf = ok_or_fail!(fold_leaf(FoldMode::Vert));
    let sys = ok_or_fail!(R6System::try_new(&leaf));
    let base = [0.5, 0.25];
    let far = [0.5, 0.75];
    let p_base = some_or_fail!(point(&leaf, base));
    let p_far = some_or_fail!(point(&leaf, far));
    assert!(
        approx3(p_base, p_far),
        "the fixture sweep maps {base:?} and {far:?} to the same surface point"
    );

    // From the base, the dyadic offset scan finds EXACTLY ONE zero: the fold
    // offset (0, 1/2).
    let h_vals = [-0.5, -0.25, 0.0, 0.25, 0.5];
    let k_vals = [-0.25, 0.0, 0.25, 0.5, 0.75];
    let hits = scan_zero_offsets(&sys, base, &h_vals, &k_vals);
    assert_eq!(hits.len(), 1, "the fixture fold is detected exactly once");
    assert!(
        approx2(hits[0], [0.0, 0.5]),
        "the detected witness offset is the fold offset, got {:?}",
        hits[0]
    );

    // Double-count regression: δ and −δ (the two ordered directions of the SAME
    // unordered pair) map to the SAME canonical witness.
    let delta = hits[0];
    let witness = r6_witness(delta);
    let witness_neg = r6_witness([-delta[0], -delta[1]]);
    assert_eq!(
        witness, witness_neg,
        "delta and -delta must map to the same canonical witness"
    );
    assert!(r6_admits(delta));
    assert!(!r6_admits([-delta[0], -delta[1]]));

    // Scanning the same fold from the FAR base finds exactly −δ, and that
    // witness canonicalizes to the SAME unordered pair: one witness per
    // self-pair, never two.
    let far_h = [-0.5, -0.25, 0.0, 0.25, 0.5];
    let far_k = [-0.5, -0.25, 0.0, 0.25];
    let far_hits = scan_zero_offsets(&sys, far, &far_h, &far_k);
    assert_eq!(
        far_hits.len(),
        1,
        "the far base detects the fold exactly once"
    );
    assert!(
        approx2(far_hits[0], [0.0, -0.5]),
        "the far-base witness offset is -delta, got {:?}",
        far_hits[0]
    );
    assert_eq!(
        r6_witness(far_hits[0]),
        witness,
        "both bases canonicalize to the same unordered-pair witness"
    );
}

#[test]
fn valid_patch_has_zero_self_intersections() {
    // A plane and a sphere chart are regular and injective: over the small
    // dyadic fixture domain the R6 residual has NO nonzero zero (the trivial
    // diagonal excluded), so no self-intersection is detected.
    let h_vals = [-0.5, -0.25, 0.0, 0.25, 0.5];
    let k_vals = [-0.5, -0.25, 0.0, 0.25, 0.5];
    let bases = [
        [0.25, 0.25],
        [0.25, 0.5],
        [0.25, 0.75],
        [0.5, 0.25],
        [0.5, 0.5],
        [0.5, 0.75],
        [0.75, 0.25],
        [0.75, 0.5],
        [0.75, 0.75],
    ];

    let plane = ok_or_fail!(plane_leaf());
    let sys_plane = ok_or_fail!(R6System::try_new(&plane));
    let mut plane_hits = 0usize;
    for base in bases {
        plane_hits += scan_zero_offsets(&sys_plane, base, &h_vals, &k_vals).len();
    }
    assert_eq!(
        plane_hits, 0,
        "the plane patch has ZERO R6 zeros on the fixture domain"
    );

    let sphere = ok_or_fail!(sphere_cap_leaf());
    let sys_sphere = ok_or_fail!(R6System::try_new(&sphere));
    let mut sphere_hits = 0usize;
    for base in bases {
        sphere_hits += scan_zero_offsets(&sys_sphere, base, &h_vals, &k_vals).len();
    }
    assert_eq!(
        sphere_hits, 0,
        "the sphere chart has ZERO R6 zeros on the fixture domain"
    );
}

// ---------------------------------------------------------------------------
// N4 discipline scan
// ---------------------------------------------------------------------------

/// Strip `//` line comments, `///`/`//!` doc comments, and `/* ... */` blocks.
fn strip_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
            let mut depth = 1usize;
            i += 2;
            while i < chars.len() && depth > 0 {
                if chars[i] == '*' && i + 1 < chars.len() && chars[i + 1] == '/' {
                    depth -= 1;
                    i += 2;
                } else if chars[i] == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
                    depth += 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
        } else if chars[i] == '/' && (i + 1 >= chars.len() || chars[i + 1] == '/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

#[test]
fn no_transcendental_call_in_selfint_module() {
    // N4: the module performs no transcendental call — no sin, cos, atan2,
    // exp, ln, log, powf, and no sqrt anywhere (whole words, comments
    // stripped).
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/kernel/selfint.rs");
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(err) => {
            assert!(false, "selfint.rs must be readable: {err}");
            return;
        }
    };
    let code = strip_comments(&source);
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let contains_word = |hay: &str, word: &str| {
        hay.match_indices(word).any(|(i, _)| {
            let before = i
                .checked_sub(1)
                .map(|j| hay.as_bytes()[j] as char)
                .map(is_word)
                .unwrap_or(false);
            let after = hay
                .as_bytes()
                .get(i + word.len())
                .map(|b| *b as char)
                .map(is_word)
                .unwrap_or(false);
            !before && !after
        })
    };
    for needle in ["sin", "cos", "atan2", "exp", "ln", "log", "powf", "sqrt"] {
        let present = code
            .lines()
            .any(|line| contains_word(line, needle) || line.contains("std::f64::consts"));
        assert!(
            !present,
            "no transcendental call may appear outside comments in selfint.rs (found {needle})"
        );
    }
}
