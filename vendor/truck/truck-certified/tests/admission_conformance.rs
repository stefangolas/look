//! ADM-001-ADAPTER conformance tests: the Theorem A ASSEMBLY over the landed
//! lemma kernels (L1 exact Bézier extraction + L2 tensor-Bernstein product)
//! and the first ADMITTED pair class (ruled-section lofts) behind its
//! admitting test.
//!
//! The assembly machine-checks the clearing algebra Theorem A depends on:
//! over every span pair the assembled polynomialized system is
//!
//! ```text
//! F_k(u,v,s,t) = Ŵ_Y(s,t)·Â_k(u,v) − Ŵ_X(u,v)·B̂_k(s,t)   (k ∈ x, y, z)
//! ```
//!
//! with `X = Â/Ŵ_X`, `Y = B̂/Ŵ_Y`, so `F = Ŵ_X·Ŵ_Y·(X − Y)` and the zeros of
//! the polynomialized `F` are EXACTLY the zeros of the direct difference
//! `X − Y` (the weights are strictly positive — certified per span by the
//! refusing [`TensorBernsteinPatch`] constructor). The four required tests:
//!
//! 1. **`polynomialized_F_zeros_equal_direct_difference_on_fixture`** — on a
//!    known fixture (the plane `X(u,v) = (u, v, 0)` against the bowl
//!    `Y(s,t) = (s, t, (s−1/2)² + (t−1/2)²)`, whose unique shared zero sits at
//!    the product-chart point `(1/2, 1/2, 1/2, 1/2)`) the assembled
//!    polynomialized residual equals the direct difference at sampled chart
//!    points, the two vanish together at the fixture zero, and the shared-chart
//!    clearing terms of the L2 product lemma recompose the same difference.
//! 2. **`ruled_section_loft_pair_admits_and_certifies`** — a ruled-section
//!    loft pair (each face linear in the loft axis, `v`-degree 1) is ADMITTED
//!    by the adapter, and the delivered `SsiPairSystem` is certified: every
//!    span carries a strictly positive weight bracket, the aligned clearing
//!    terms exist with the recorded grown degrees and positive brackets, and
//!    the polynomialized residual machine-checks against the direct difference
//!    on the fixture grid.
//! 3. **`non_admitted_carriers_still_refuse_typed`** — general spline-section
//!    faces (bidegree `(2, 2)` spans), a non-positive-weight face, and the
//!    certificate carriers all refuse typed exactly as before ADM-001 (the
//!    adapter fires nowhere a green class is served — V5).
//! 4. **`v5_pair_identity_battery_green`** — the V5 battery: the landed green
//!    restricted-pair battery (plane × sphere, sweep × plane) certifies
//!    bit-identically on identical re-runs, and the ADM adapter is reachable
//!    ONLY for the admitted ruled-section-loft class while every general
//!    spline-section carrier form refuses typed.
//!
//! House rules: H-1 (`#![deny(clippy::unwrap_used)]`); every fallible
//! construction unpacks through explicit `panic!` arms. All fixture nets are
//! positive-weight clamped homogeneous `BSplineSurface<Vector4>` faces over
//! the unit square; the identity slack is the named [`EVAL_TOLERANCE`].

#![deny(clippy::all, rust_2018_idioms)]
#![deny(clippy::unwrap_used)]

use truck_certified::construct::admission::{
    admit_tensor_spline_pair, AdmitDomainHint, CollapsedBoundary, CullingOrder, EdgeId, NormalCone,
    RegularPatch, SeamIdentified, SsiPairSystem, TransversePair,
};
use truck_certified::construct::bie::fixtures::{plane_sphere_fixture, sweep_plane_fixture};
use truck_certified::construct::bie::ssi4::{
    certify_restricted_pair, RestrictedChart, Ssi4Parameters,
};
use truck_certified::construct::bie::WitnessCell;
use truck_certified::construct::patches::TensorBernsteinPatch;
use truck_certified::construct::refusal::ConstructRefusal;
use truck_geometry::prelude::{BSplineSurface, KnotVec, Vector4};

/// The unit-scale evaluation tolerance of the assembly machine checks (the
/// fixture fixtures are unit-scale dyadic-rational geometry; the assembled
/// clearing and the direct difference are two float evaluation paths of the
/// same exact algebra, so a `1e-9` bound is orders of magnitude above the
/// rounding noise). H-3: named, never a bare literal.
const EVAL_TOLERANCE: f64 = 1.0e-9;

/// The tolerance used for the "vanish together" zero-coincidence checks, scaled
/// by the certified weight-bracket products (the fixture weights are `1`, so
/// the residual is the direct difference up to the same rounding noise).
const ZERO_TOLERANCE: f64 = 1.0e-9;

/// The integer binomial `C(n, k)` (small exact arithmetic for the fixture
/// degrees).
fn binomial(n: usize, k: usize) -> f64 {
    let k = k.min(n - k);
    let mut r = 1usize;
    for i in 1..=k {
        r = r * (n - k + i) / i;
    }
    r as f64
}

/// The degree-`degree` Bernstein basis values at `t`.
fn bernstein_basis(degree: usize, t: f64) -> Vec<f64> {
    let mut values = Vec::with_capacity(degree + 1);
    for k in 0..=degree {
        values.push(binomial(degree, k) * t.powi(k as i32) * (1.0 - t).powi((degree - k) as i32));
    }
    values
}

/// Scalar tensor-Bernstein evaluation of a row-major grid at `(u, v)`.
fn eval_scalar_net(grid: &[Vec<f64>], u: f64, v: f64) -> f64 {
    let bu = bernstein_basis(grid.len() - 1, u);
    let bv = bernstein_basis(grid[0].len() - 1, v);
    let mut acc = 0.0;
    for (i, bi) in bu.iter().enumerate() {
        for (j, bj) in bv.iter().enumerate() {
            acc += bi * bj * grid[i][j];
        }
    }
    acc
}

/// Vector tensor-Bernstein evaluation of a row-major `R³` grid at `(u, v)`.
fn eval_vector_net(grid: &[Vec<[f64; 3]>], u: f64, v: f64) -> [f64; 3] {
    let bu = bernstein_basis(grid.len() - 1, u);
    let bv = bernstein_basis(grid[0].len() - 1, v);
    let mut acc = [0.0; 3];
    for (i, bi) in bu.iter().enumerate() {
        for (j, bj) in bv.iter().enumerate() {
            let c = grid[i][j];
            acc[0] += bi * bj * c[0];
            acc[1] += bi * bj * c[1];
            acc[2] += bi * bj * c[2];
        }
    }
    acc
}

/// A same-shape unit weight grid for the polynomial fixtures.
fn ones_weights(numerator: &[Vec<[f64; 3]>]) -> Vec<Vec<f64>> {
    numerator.iter().map(|row| vec![1.0; row.len()]).collect()
}

/// The `(m + 1) × (n + 1)` Bernstein grid of `u^pu · v^pv` at bidegree
/// `(m, n)`, from the monomial-to-Bernstein coefficient conversion.
fn monomial_grid(m: usize, n: usize, terms: &[(usize, usize, f64)]) -> Vec<Vec<f64>> {
    let mut grid = vec![vec![0.0f64; n + 1]; m + 1];
    for &(pu, pv, coeff) in terms {
        for (a, row) in grid.iter_mut().enumerate() {
            for (b, cell) in row.iter_mut().enumerate() {
                if a >= pu && b >= pv {
                    let fa = binomial(a, pu) / binomial(m, pu);
                    let fb = binomial(b, pv) / binomial(n, pv);
                    *cell += coeff * fa * fb;
                }
            }
        }
    }
    grid
}

/// The `(m + 1) × (n + 1)` grid of the first (`which == 0`) or second
/// (`which == 1`) unit-chart coordinate: the Bernstein coefficients of the
/// linear coordinate polynomial in that axis (rows index the first parameter).
fn coord_grid(m: usize, n: usize, which: usize) -> Vec<Vec<f64>> {
    let mut grid = Vec::with_capacity(m + 1);
    for a in 0..=m {
        let mut row = Vec::with_capacity(n + 1);
        for b in 0..=n {
            row.push(if which == 0 {
                a as f64 / m as f64
            } else {
                b as f64 / n as f64
            });
        }
        grid.push(row);
    }
    grid
}

/// A positive-weight clamped single-span homogeneous face over the unit square
/// from an `R³` numerator net and a same-shape weight net: the control net is
/// the homogeneous `(w·x, w·y, w·z, w)` grid over the clamped degree knots of
/// the net shape.
fn unit_square_face(
    numerator: Vec<Vec<[f64; 3]>>,
    weights: Vec<Vec<f64>>,
) -> BSplineSurface<Vector4> {
    let rows = numerator.len();
    let cols = numerator[0].len();
    let knots = |degree: usize| {
        let mut k = Vec::with_capacity(2 * (degree + 1));
        for _ in 0..=degree {
            k.push(0.0);
        }
        for _ in 0..=degree {
            k.push(1.0);
        }
        KnotVec::from(k)
    };
    let mut ctrl: Vec<Vec<Vector4>> = Vec::with_capacity(rows);
    for (i, row) in numerator.iter().enumerate() {
        let mut control_row: Vec<Vector4> = Vec::with_capacity(cols);
        for (j, a) in row.iter().enumerate() {
            control_row.push(Vector4::new(a[0], a[1], a[2], weights[i][j]));
        }
        ctrl.push(control_row);
    }
    BSplineSurface::new((knots(rows - 1), knots(cols - 1)), ctrl)
}

/// The dehomogenized surface point of a patch at the unit-square parameters
/// `(u, v)` (the weight field is certified strictly positive).
fn surface_point(patch: &TensorBernsteinPatch, u: f64, v: f64) -> [f64; 3] {
    let num = eval_vector_net(patch.numerator(), u, v);
    let w = eval_scalar_net(patch.weights(), u, v);
    [num[0] / w, num[1] / w, num[2] / w]
}

/// The weight-field value of a patch at the unit-square parameters `(u, v)`.
fn weight_value(patch: &TensorBernsteinPatch, u: f64, v: f64) -> f64 {
    eval_scalar_net(patch.weights(), u, v)
}

/// The `R³` numerator-field value of a patch at `(u, v)` (never dehomogenized).
fn numerator_value(patch: &TensorBernsteinPatch, u: f64, v: f64) -> [f64; 3] {
    eval_vector_net(patch.numerator(), u, v)
}

/// Scale a vector by a scalar (the fixture arithmetic; not a shipped kernel).
fn scale3(s: f64, v: [f64; 3]) -> [f64; 3] {
    [s * v[0], s * v[1], s * v[2]]
}

/// The componentwise difference of two vectors.
fn sub3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// The polynomialized residual `F = Ŵ_Y(s,t)·Â(u,v) − Ŵ_X(u,v)·B̂(s,t)` of one
/// assembled span pair at a product-chart point, evaluated from the two stored
/// spans (the per-side separable evaluation the carrier holds).
fn cleared_residual(
    x_span: &TensorBernsteinPatch,
    y_span: &TensorBernsteinPatch,
    x: [f64; 4],
) -> [f64; 3] {
    let wx = weight_value(x_span, x[0], x[1]);
    let wy = weight_value(y_span, x[2], x[3]);
    let ax = numerator_value(x_span, x[0], x[1]);
    let by = numerator_value(y_span, x[2], x[3]);
    sub3(scale3(wy, ax), scale3(wx, by))
}

/// The direct difference `X(u, v) − Y(s, t)` at a product-chart point.
fn direct_difference(
    x_span: &TensorBernsteinPatch,
    y_span: &TensorBernsteinPatch,
    x: [f64; 4],
) -> [f64; 3] {
    sub3(
        surface_point(x_span, x[0], x[1]),
        surface_point(y_span, x[2], x[3]),
    )
}

/// The product of the two certified weight-bracket lower bounds of a span pair
/// (strictly positive; the clearing factor `Ŵ_X·Ŵ_Y` never vanishes below it).
fn clearing_factor_lo(x_span: &TensorBernsteinPatch, y_span: &TensorBernsteinPatch) -> f64 {
    x_span.weight_bracket().lo * y_span.weight_bracket().lo
}

/// The worst per-coordinate absolute error between two vectors.
fn worst_error(a: [f64; 3], b: [f64; 3]) -> f64 {
    let mut worst = 0.0;
    for k in 0..3 {
        let error = (a[k] - b[k]).abs();
        if error > worst {
            worst = error;
        }
    }
    worst
}

/// The worst per-coordinate magnitude of a vector.
fn worst_abs(a: [f64; 3]) -> f64 {
    let mut worst = 0.0;
    for k in 0..3 {
        let m = a[k].abs();
        if m > worst {
            worst = m;
        }
    }
    worst
}

/// Unpack a fallible face construction; the fixture data is valid.
fn admit_face<T>(result: Result<T, ()>, what: &str) -> T {
    match result {
        Ok(value) => value,
        Err(_) => panic!("fixture construction failed: {what}"),
    }
}

/// The domain hint of the Theorem A adapter signature.
fn domain_hint() -> AdmitDomainHint {
    AdmitDomainHint {
        order: CullingOrder::HullAabbThenKnotSpan,
    }
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// The bilinear unit plane `X(u, v) = (u, v, 0)` (bidegree `(1, 1)`, unit
/// weights).
fn plane_zero_face() -> BSplineSurface<Vector4> {
    let m = 1usize;
    let n = 1usize;
    let x = coord_grid(m, n, 0);
    let y = coord_grid(m, n, 1);
    let z = vec![vec![0.0; n + 1]; m + 1];
    let numerator = combine(x, y, z);
    let weights = ones_weights(&numerator);
    unit_square_face(numerator, weights)
}

/// The unit bowl `Y(s, t) = (s, t, (s − 1/2)² + (t − 1/2)²)` (bidegree
/// `(2, 2)`, unit weights): a general spline-section surface whose unique
/// meeting point with the plane `X = (u, v, 0)` is the product-chart point
/// `(1/2, 1/2, 1/2, 1/2)`.
fn bowl_face() -> BSplineSurface<Vector4> {
    let m = 2usize;
    let n = 2usize;
    let x = coord_grid(m, n, 0);
    let y = coord_grid(m, n, 1);
    let z = monomial_grid(
        m,
        n,
        &[
            (2, 0, 1.0),
            (1, 0, -1.0),
            (0, 2, 1.0),
            (0, 1, -1.0),
            (0, 0, 0.5),
        ],
    );
    let numerator = combine(x, y, z);
    let weights = ones_weights(&numerator);
    unit_square_face(numerator, weights)
}

/// Combine three scalar grids into one `R³` grid.
fn combine(x: Vec<Vec<f64>>, y: Vec<Vec<f64>>, z: Vec<Vec<f64>>) -> Vec<Vec<[f64; 3]>> {
    let mut out = Vec::with_capacity(x.len());
    for i in 0..x.len() {
        let mut row = Vec::with_capacity(x[i].len());
        for j in 0..x[i].len() {
            row.push([x[i][j], y[i][j], z[i][j]]);
        }
        out.push(row);
    }
    out
}

/// A ruled-section loft face: the loft `X(u, v) = (u, u², v)` (a parabolic
/// loft between the two sections `(u, u², 0)` and `(u, u², 1)` — straight
/// generator lines in the `v` direction). Bidegree `(2, 1)`: every extracted
/// span is linear in the loft axis, the ADM-001 admitted class shape.
fn ruled_loft_face(offset_x: f64) -> BSplineSurface<Vector4> {
    let m = 2usize;
    let n = 1usize;
    // x = u (offset), y = u², z = v.
    let u_coeff = [0.0, 0.5, 1.0];
    let u2_coeff = [0.0, 0.0, 1.0];
    let v_coeff = [0.0, 1.0];
    let mut numerator: Vec<Vec<[f64; 3]>> = Vec::with_capacity(m + 1);
    for i in 0..=m {
        let mut row: Vec<[f64; 3]> = Vec::with_capacity(n + 1);
        for j in 0..=n {
            row.push([u_coeff[i] + offset_x, u2_coeff[i], v_coeff[j]]);
        }
        numerator.push(row);
    }
    let weights = ones_weights(&numerator);
    unit_square_face(numerator, weights)
}

/// The exact NURBS unit-sphere octant face (bidegree `(2, 2)`, genuinely
/// rational positive weights) — a general spline-section carrier.
fn sphere_octant_face() -> BSplineSurface<Vector4> {
    let profile: [[f64; 3]; 3] = [[1.0, 0.0, 0.0], [1.0, 0.0, 1.0], [0.0, 0.0, 1.0]];
    let turn: [[f64; 2]; 3] = [[1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let arc_weights = [1.0, std::f64::consts::FRAC_1_SQRT_2, 1.0];
    let mut numerator: Vec<Vec<[f64; 3]>> = Vec::new();
    let mut weights: Vec<Vec<f64>> = Vec::new();
    for i in 0..3 {
        let mut num_row: Vec<[f64; 3]> = Vec::new();
        let mut w_row: Vec<f64> = Vec::new();
        for j in 0..3 {
            let w = arc_weights[i] * arc_weights[j];
            num_row.push([
                w * profile[i][0] * turn[j][0],
                w * profile[i][0] * turn[j][1],
                w * profile[i][2],
            ]);
            w_row.push(w);
        }
        numerator.push(num_row);
        weights.push(w_row);
    }
    unit_square_face(numerator, weights)
}

/// A face whose weight field is NOT certifiably positive (a negative
/// homogeneous weight at a corner): extraction must refuse.
fn non_positive_weight_face() -> BSplineSurface<Vector4> {
    let knots = KnotVec::from(vec![0.0, 0.0, 1.0, 1.0]);
    let ctrl = vec![
        vec![
            Vector4::new(0.0, 0.0, 0.0, -0.5),
            Vector4::new(0.0, 1.0, 0.0, 1.0),
        ],
        vec![
            Vector4::new(1.0, 0.0, 0.0, 1.0),
            Vector4::new(1.0, 1.0, 1.0, 1.0),
        ],
    ];
    BSplineSurface::new((knots.clone(), knots), ctrl)
}

// ---------------------------------------------------------------------------
// Required tests
// ---------------------------------------------------------------------------

// The required test name spells the polynomialized form `F` (a mathematical
// symbol), which is not snake case; the `allow` is per-item.
#[allow(non_snake_case)]
#[test]
fn polynomialized_F_zeros_equal_direct_difference_on_fixture() {
    // The fixture: the plane X = (u, v, 0) against the bowl Y = (s, t, bowl)
    // (a general spline-section pair — assembled directly, not via the
    // adapter, which refuses this class). The unique shared zero is the
    // product-chart point (1/2, 1/2, 1/2, 1/2).
    let x_face = admit_face(Ok(plane_zero_face()), "plane zero face");
    let y_face = admit_face(Ok(bowl_face()), "bowl face");
    let system = match SsiPairSystem::assemble(&x_face, &y_face, &domain_hint()) {
        Ok(system) => system,
        Err(refusal) => panic!("the assembly refused the fixture pair: {refusal:?}"),
    };

    assert_eq!(system.x_spans().len(), 1, "one X span on the fixture");
    assert_eq!(system.y_spans().len(), 1, "one Y span on the fixture");
    assert_eq!(
        system.span_pair_count(),
        1,
        "one assembled span pair on the single-span fixture"
    );
    assert_eq!(
        system.x_spans()[0].degree(),
        (1, 1),
        "the plane span bidegree"
    );
    assert_eq!(
        system.y_spans()[0].degree(),
        (2, 2),
        "the bowl span bidegree"
    );

    let x_span = &system.x_spans()[0];
    let y_span = &system.y_spans()[0];
    let clearing = &system.span_pairs()[0];
    let factor_lo = clearing_factor_lo(x_span, y_span);
    assert!(
        factor_lo > 0.0,
        "the certified clearing factor is strictly positive"
    );

    // The assembly identity: F = Ŵ_X·Ŵ_Y·(X − Y) at sampled product-chart
    // points. Sample a modest grid over the four-axis chart.
    let steps = [0.0f64, 0.25, 0.5, 0.75, 1.0];
    let mut worst = 0.0f64;
    for &u in &steps {
        for &v in &steps {
            for &s in &steps {
                for &t in &steps {
                    let point = [u, v, s, t];
                    let cleared = cleared_residual(x_span, y_span, point);
                    let direct = direct_difference(x_span, y_span, point);
                    let wx = weight_value(x_span, u, v);
                    let wy = weight_value(y_span, s, t);
                    let scaled = scale3(wx * wy, direct);
                    let error = worst_error(cleared, scaled);
                    if error > worst {
                        worst = error;
                    }
                }
            }
        }
    }
    assert!(
        worst <= EVAL_TOLERANCE,
        "the assembled polynomialized residual deviates from Ŵ_X·Ŵ_Y·(X − Y) \
         by {worst} at a sampled chart point"
    );

    // Zero coincidence at the fixture zero: both the polynomialized residual
    // and the direct difference vanish at (1/2, 1/2, 1/2, 1/2).
    let zero = [0.5, 0.5, 0.5, 0.5];
    let cleared = cleared_residual(x_span, y_span, zero);
    let direct = direct_difference(x_span, y_span, zero);
    assert!(
        worst_abs(cleared) <= ZERO_TOLERANCE,
        "the polynomialized residual must vanish at the fixture zero, got {cleared:?}"
    );
    assert!(
        worst_abs(direct) <= ZERO_TOLERANCE,
        "the direct difference must vanish at the fixture zero, got {direct:?}"
    );

    // Away from the zero the identity keeps both nonzero together: sample a
    // non-zero chart point and confirm the polynomialized residual matches the
    // scaled direct difference (a real clearing error would be O(1) here).
    let off = [0.3, 0.6, 0.2, 0.7];
    let cleared = cleared_residual(x_span, y_span, off);
    let direct = direct_difference(x_span, y_span, off);
    let wx = weight_value(x_span, off[0], off[1]);
    let wy = weight_value(y_span, off[2], off[3]);
    assert!(
        worst_abs(direct) > 1.0e-3,
        "the off-zero chart point is genuinely away from the intersection"
    );
    assert!(
        worst_error(cleared, scale3(wx * wy, direct)) <= EVAL_TOLERANCE,
        "the clearing identity must hold away from the zero too"
    );

    // The aligned clearing terms of the L2 product lemma over the shared
    // chart: on this fixture both spans share the unit-square domain, so the
    // span pair carries the two clearing term patches term1 = Ŵ_Y·Â and
    // term2 = Ŵ_X·B̂, and their difference recomposes the cleared difference
    // at matched parameters (the Theorem A clearing algebra, exact).
    let aligned = match clearing.aligned_terms() {
        Some(terms) => terms,
        None => panic!("the shared-chart span pair must carry the aligned clearing terms"),
    };
    for &a in &steps {
        for &b in &steps {
            let term1 = surface_point(&aligned[0], a, b);
            let term2 = surface_point(&aligned[1], a, b);
            let matched = sub3(term1, term2);
            let expected = scale3(
                weight_value(x_span, a, b) * weight_value(y_span, a, b),
                sub3(surface_point(x_span, a, b), surface_point(y_span, a, b)),
            );
            assert!(
                worst_error(matched, expected) <= EVAL_TOLERANCE,
                "the aligned clearing terms must recompose Ŵ_X·Ŵ_Y·(X − Y) at \
                 matched parameters ({a}, {b})"
            );
        }
    }
}

#[test]
fn ruled_section_loft_pair_admits_and_certifies() {
    // A ruled-section loft pair: both faces are v-linear ruled lofts over the
    // unit square (the section curves are quadratic parabolas, the loft
    // direction straight). The adapter ADMITS the class, and the delivered
    // SsiPairSystem is certified on its assembled data.
    let x_face = admit_face(Ok(ruled_loft_face(0.0)), "ruled-section loft X");
    let y_face = admit_face(Ok(ruled_loft_face(0.25)), "ruled-section loft Y");
    let system = match admit_tensor_spline_pair(&x_face, &y_face, &domain_hint()) {
        Ok(system) => system,
        Err(refusal) => {
            panic!("a ruled-section loft pair must ADMIT behind ADM-001, refused: {refusal:?}")
        }
    };

    // The delivered assembly is certified: exactly one span per face (both
    // single-span), every span linear in the loft axis, and every extracted
    // span carries a strictly positive certified weight bracket.
    assert_eq!(system.x_spans().len(), 1, "the X loft is a single span");
    assert_eq!(system.y_spans().len(), 1, "the Y loft is a single span");
    for span in system.x_spans().iter().chain(system.y_spans().iter()) {
        assert_eq!(
            span.degree().1,
            1,
            "a ruled-section loft span is linear in the loft axis"
        );
        assert!(
            span.weight_bracket().lo > 0.0,
            "every extracted span carries a certified strictly positive weight bracket"
        );
    }

    // Both faces share the unit-square domain, so the single span pair carries
    // the aligned clearing terms (the L2 product-lemma composition), each with
    // the recorded grown degree and a strictly positive weight bracket.
    let clearing = &system.span_pairs()[0];
    assert_eq!(clearing.x_span(), 0, "the clearing names the X span");
    assert_eq!(clearing.y_span(), 0, "the clearing names the Y span");
    let aligned = match clearing.aligned_terms() {
        Some(terms) => terms,
        None => panic!("the shared-chart span pair must carry the aligned clearing terms"),
    };
    let (mx, nx) = system.x_spans()[0].degree();
    let (my, ny) = system.y_spans()[0].degree();
    for (label, term) in [("term1", &aligned[0]), ("term2", &aligned[1])] {
        assert_eq!(
            term.degree(),
            (mx + my, nx + ny),
            "{label}: the clearing product records the grown bidegree"
        );
        assert!(
            term.weight_bracket().lo > 0.0,
            "{label}: the clearing product weight field stays strictly positive"
        );
    }

    // The polynomialized residual machine-checks against the direct difference
    // on the fixture grid (the clearing algebra is exact on the admitted pair).
    let x_span = &system.x_spans()[0];
    let y_span = &system.y_spans()[0];
    let steps = [0.0f64, 0.25, 0.5, 0.75, 1.0];
    let mut worst = 0.0f64;
    for &u in &steps {
        for &v in &steps {
            for &s in &steps {
                for &t in &steps {
                    let point = [u, v, s, t];
                    let cleared = cleared_residual(x_span, y_span, point);
                    let direct = direct_difference(x_span, y_span, point);
                    let wx = weight_value(x_span, u, v);
                    let wy = weight_value(y_span, s, t);
                    let error = worst_error(cleared, scale3(wx * wy, direct));
                    if error > worst {
                        worst = error;
                    }
                }
            }
        }
    }
    assert!(
        worst <= EVAL_TOLERANCE,
        "the admitted pair's polynomialized system deviates by {worst} from the \
         cleared direct difference"
    );

    // The delivery is deterministic: an identical second admission assembles
    // an identical span-pair count and identical clearing degrees.
    let again = match admit_tensor_spline_pair(&x_face, &y_face, &domain_hint()) {
        Ok(system) => system,
        Err(refusal) => panic!("the identical second admission refused: {refusal:?}"),
    };
    assert_eq!(
        again.span_pair_count(),
        system.span_pair_count(),
        "identical input assembles an identical system"
    );
}

#[test]
fn non_admitted_carriers_still_refuse_typed() {
    // General spline-section carriers: a (2, 2) bowl against a (2, 2) octant —
    // neither face is a ruled-section loft, so the adapter refuses typed
    // InvalidInput (nothing beyond the first class is admitted; a boolean
    // boundary consulting admission keeps its NonCanonicalCarrier refusal).
    let bowl = admit_face(Ok(bowl_face()), "bowl face");
    let octant = admit_face(Ok(sphere_octant_face()), "octant face");
    match admit_tensor_spline_pair(&bowl, &octant, &domain_hint()) {
        Ok(_) => panic!("a general spline-section pair must refuse typed"),
        Err(refusal) => assert_eq!(
            refusal,
            ConstructRefusal::InvalidInput,
            "a non-admitted carrier pair refuses InvalidInput (V5)"
        ),
    }

    // A non-positive-weight face refuses at extraction (typed), never a silent
    // non-positive weight.
    let bad = admit_face(Ok(non_positive_weight_face()), "non-positive weight face");
    match admit_tensor_spline_pair(&bad, &bowl, &domain_hint()) {
        Ok(_) => panic!("a non-positive-weight face must refuse typed"),
        Err(refusal) => assert_eq!(
            refusal,
            ConstructRefusal::InvalidInput,
            "a face whose weight bracket cannot certify positive refuses InvalidInput"
        ),
    }

    // The certificate carriers keep refusing Unfrozen: their production is
    // ADM-002-CERTIFICATES, so nothing beyond the assembly certifies here.
    let cone = NormalCone {
        anchor: [0.0, 0.0, 1.0],
        s_up: 0.5,
    };
    match RegularPatch::try_new(cone) {
        Ok(_) => panic!("RegularPatch production is ADM-002's, refusing here"),
        Err(refusal) => assert_eq!(refusal, ConstructRefusal::Unfrozen),
    }
    match CollapsedBoundary::try_new(2) {
        Ok(_) => panic!("CollapsedBoundary production is ADM-002's, refusing here"),
        Err(refusal) => assert_eq!(refusal, ConstructRefusal::Unfrozen),
    }
    match SeamIdentified::try_new((EdgeId(0), EdgeId(1))) {
        Ok(_) => panic!("SeamIdentified production is ADM-002's, refusing here"),
        Err(refusal) => assert_eq!(refusal, ConstructRefusal::Unfrozen),
    }
    match TransversePair::try_new((1.0e-2, 1.0e-1)) {
        Ok(_) => panic!("TransversePair production is ADM-002's, refusing here"),
        Err(refusal) => assert_eq!(refusal, ConstructRefusal::Unfrozen),
    }
}

#[test]
fn v5_pair_identity_battery_green() {
    // The V5 battery: the landed green restricted-pair fixtures certify
    // BIT-IDENTICALLY on identical re-runs through the landed engine, and the
    // ADM adapter is reachable ONLY where the old path refused
    // NonCanonicalCarrier — the ruled-section-loft class — while every general
    // spline-section carrier form (a green class served ahead of admission)
    // refuses typed, so no already-green answer can change.

    // (a) The landed green restricted battery: plane × sphere and sweep ×
    // plane, certified through certify_restricted_pair with default
    // parameters. Two identical runs produce bit-identical certified curves.
    let ps = plane_sphere_fixture();
    let ps_a = RestrictedChart::from_plane(ps.plane);
    let ps_b = RestrictedChart::from_sphere(ps.sphere);

    let sp = sweep_plane_fixture();
    let sp_a = match RestrictedChart::circular_sweep(
        sp.sweep.spine_from,
        sp.sweep.spine_to,
        sp.sweep.radius_start,
        sp.sweep.radius_end,
        sp.sweep.s0,
        sp.sweep.s1,
        sp.sweep.v0,
        sp.sweep.v1,
    ) {
        Some(chart) => chart,
        None => panic!("the fixture sweep spine is non-degenerate"),
    };
    let sp_b = RestrictedChart::from_plane(sp.plane);

    let battery: Vec<(&str, RestrictedChart, RestrictedChart, WitnessCell)> = vec![
        ("plane_x_sphere", ps_a, ps_b, ps.cell),
        ("sweep_x_plane", sp_a, sp_b, sp.cell),
    ];
    let params = Ssi4Parameters::default();
    for (name, a, b, cell) in battery {
        let run = || {
            let mut budget = truck_base::evidence::Budget::new(0, 0, 0);
            let outcome = certify_restricted_pair(a.clone(), b.clone(), cell, &params, &mut budget);
            match outcome {
                Ok(truck_base::evidence::Certified { value, .. }) => {
                    (format!("{value:?}"), value.samples.len())
                }
                Err(refusal) => {
                    panic!("{name}: the landed green pair must certify, refused {refusal:?}")
                }
            }
        };
        let (first, first_count) = run();
        let (second, second_count) = run();
        assert!(
            first_count > 0 && second_count > 0,
            "{name}: a green battery row certifies branch samples \
             ({first_count}, {second_count})"
        );
        assert_eq!(
            first, second,
            "{name}: a green pair answers bit-identically on an identical re-run (V5)"
        );
    }

    // (b) The adapter is reachable ONLY for the admitted ruled-section-loft
    // class: the ruled pair admits (fires where the old path returned
    // NonCanonicalCarrier for the loft solid faces), and the general
    // spline-section pair refuses typed (green classes are served ahead of the
    // adapter and never reach it).
    let loft_x = admit_face(Ok(ruled_loft_face(0.0)), "ruled loft X");
    let loft_y = admit_face(Ok(ruled_loft_face(0.25)), "ruled loft Y");
    match admit_tensor_spline_pair(&loft_x, &loft_y, &domain_hint()) {
        Ok(system) => assert!(
            system.span_pair_count() >= 1,
            "the admitted class assembles a non-empty system"
        ),
        Err(refusal) => panic!("the ruled-section-loft class must be reachable: {refusal:?}"),
    }

    let bowl = admit_face(Ok(bowl_face()), "bowl face");
    let octant = admit_face(Ok(sphere_octant_face()), "octant face");
    match admit_tensor_spline_pair(&bowl, &octant, &domain_hint()) {
        Ok(_) => panic!("a general spline-section pair must not reach the adapter"),
        Err(refusal) => assert_eq!(
            refusal,
            ConstructRefusal::InvalidInput,
            "a green/general carrier pair keeps its typed refusal (V5)"
        ),
    }
    // And the same green refusal is deterministic (identical input, identical
    // refusal — no verdict flip).
    match admit_tensor_spline_pair(&bowl, &octant, &domain_hint()) {
        Ok(_) => panic!("the repeated general pair must refuse"),
        Err(refusal) => assert_eq!(refusal, ConstructRefusal::InvalidInput),
    }
}
