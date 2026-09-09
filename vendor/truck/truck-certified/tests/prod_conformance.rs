//! ADM-L2-PRODUCT conformance: the L2 tensor-Bernstein product lemma kernels
//! (`truck-certified::construct::prod::patch_product` and
//! `truck-certified::construct::prod::elevate_degree`), exercised through the
//! public API against the ADM-SHIM fixture kit (`tests/patch_fixtures.rs`).
//!
//! The three required tests:
//!
//! - `product_identity_exact_at_matched_parameters` — for every fixture pair
//!   (polynomial and rational-weight alike) the product patch `c =
//!   patch_product(a, b)` satisfies `c(u, v) = a(u, v) ⊙ b(u, v)` (the
//!   pointwise componentwise product of the dehomogenized surfaces) at every
//!   exact dyadic parameter of the shared unit square, machine-checked to a
//!   named unit-scale tolerance; the pure-dyadic bilinear square is checked to
//!   machine precision;
//! - `degree_growth_recorded_not_hidden` — the product bidegree is exactly
//!   `deg_a + deg_b`, recorded ON the returned patch and on its coefficient
//!   grids (the grown shape, never a truncated or sampled product);
//! - `zero_patch_product_is_zero` — a patch with the zero numerator field
//!   annihilates exactly: the product numerator grid is termwise exactly zero
//!   at the recorded grown degree, weights still multiply to a positive
//!   field, and the dehomogenized product surface is the zero point.
//!
//! Supporting tests (lemma item 2 and the refusing edges):
//!
//! - `degree_elevation_identity_exact` — raising a patch to a higher bidegree
//!   does not change any surface value (the elevation identity,
//!   machine-checked on the same exact grid);
//! - `degree_elevation_to_current_degree_is_an_exact_copy` — a target equal
//!   to the current bidegree copies the coefficients EXACTLY (no arithmetic,
//!   no rounding);
//! - `degree_elevation_below_current_degree_refuses` — a target lower than
//!   the current bidegree refuses typed `ConstructRefusal::InvalidInput`;
//! - `mismatched_spans_refuse_typed` — a product of patches over unequal
//!   source-domain boxes refuses `ConstructRefusal::InvalidInput`, never a
//!   silent reparameterization;
//! - `bilinear_square_coefficients_are_exact_dyadics` — the coefficient
//!   convolution is exactly representable on the integer-arithmetic fixture
//!   (the `(u, v, u·v)` bilinear graph squared), so the returned grids are
//!   checked BIT FOR BIT against the closed form `(u², v², u²v²)`.

use truck_certified::construct::patches::{PatchParent, TensorBernsteinPatch};
use truck_certified::construct::prod::{elevate_degree, patch_product};
use truck_certified::construct::refusal::ConstructRefusal;
use truck_certified::kernel::patch::IBox2;

#[path = "patch_fixtures.rs"]
mod patch_fixtures;

/// The dyadic step count of the exact-parameter grid: parameters `i/8` for
/// `i ∈ [0, 8]` (every grid parameter is an exact dyadic rational).
const GRID_STEPS: usize = 8;

/// Unit-scale identity tolerance for the machine-checked product/elevation
/// identities (the fixtures are unit-scale; the coefficient convolution is
/// exact up to one final storage rounding per coefficient, so a unit-scale
/// `1e-9` bound is orders of magnitude above the rounding noise — named, never
/// a bare literal).
const PRODUCT_TOLERANCE: f64 = 1.0e-9;

/// The machine-precision identity bound for the pure-dyadic (integer
/// coefficient) bilinear square, where every intermediate is exactly
/// representable (named, never a bare literal).
const EXACT_DYADIC_TOLERANCE: f64 = 1.0e-14;

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

/// The dehomogenized surface point `X(u, v) = Â/Ŵ` of a patch (the weight
/// field is certified strictly positive, so the division is safe).
fn surface_point(patch: &TensorBernsteinPatch, u: f64, v: f64) -> [f64; 3] {
    let num = eval_vector_net(patch.numerator(), u, v);
    let w = eval_scalar_net(patch.weights(), u, v);
    [num[0] / w, num[1] / w, num[2] / w]
}

/// The pointwise componentwise (Hadamard) product of two surface points.
fn hadamard(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] * b[0], a[1] * b[1], a[2] * b[2]]
}

/// The largest per-coordinate surface deviation `|c(u,v) − a(u,v)⊙b(u,v)|`
/// over the exact dyadic grid, for one fixture pair.
fn worst_product_identity_error(a: &TensorBernsteinPatch, b: &TensorBernsteinPatch) -> f64 {
    let product = expect_product(a, b);
    assert_eq!(
        product.domain(),
        a.domain(),
        "the product patch inherits the shared span"
    );
    assert_eq!(
        product.parent(),
        a.parent(),
        "the product patch inherits the first operand's parent identity"
    );
    let mut worst = 0.0_f64;
    for i in 0..=GRID_STEPS {
        for j in 0..=GRID_STEPS {
            let u = i as f64 / GRID_STEPS as f64;
            let v = j as f64 / GRID_STEPS as f64;
            let left = surface_point(&product, u, v);
            let right = hadamard(surface_point(a, u, v), surface_point(b, u, v));
            for (l, r) in left.iter().zip(right.iter()) {
                let error = (l - r).abs();
                if error > worst {
                    worst = error;
                }
            }
        }
    }
    worst
}

/// The largest per-coordinate surface deviation between a patch and its
/// elevation at the target bidegree, over the exact dyadic grid.
fn worst_elevation_identity_error(patch: &TensorBernsteinPatch, target: (usize, usize)) -> f64 {
    let elevated = expect_elevation(patch, target);
    let mut worst = 0.0_f64;
    for i in 0..=GRID_STEPS {
        for j in 0..=GRID_STEPS {
            let u = i as f64 / GRID_STEPS as f64;
            let v = j as f64 / GRID_STEPS as f64;
            let original = surface_point(patch, u, v);
            let raised = surface_point(&elevated, u, v);
            for (o, r) in original.iter().zip(raised.iter()) {
                let error = (o - r).abs();
                if error > worst {
                    worst = error;
                }
            }
        }
    }
    worst
}

/// Extract the `Ok` of a product call; the refusal arm is a conformance-test
/// bug panic (the fixture pairs are valid aligned data).
fn expect_product(a: &TensorBernsteinPatch, b: &TensorBernsteinPatch) -> TensorBernsteinPatch {
    match patch_product(a, b) {
        Ok(product) => product,
        Err(refusal) => panic!("patch_product refused aligned fixtures: {refusal:?}"),
    }
}

/// Extract the `Ok` of an elevation call; the refusal arm is a
/// conformance-test bug panic (the targets are valid raises).
fn expect_elevation(patch: &TensorBernsteinPatch, target: (usize, usize)) -> TensorBernsteinPatch {
    match elevate_degree(patch, target) {
        Ok(elevated) => elevated,
        Err(refusal) => panic!("elevate_degree refused a valid raise: {refusal:?}"),
    }
}

/// Extract the `Ok` of a patch construction; the fixture data is valid by
/// construction.
fn expect_patch(
    result: Result<TensorBernsteinPatch, ConstructRefusal>,
    what: &str,
) -> TensorBernsteinPatch {
    match result {
        Ok(patch) => patch,
        Err(refusal) => panic!("{what} refused construction: {refusal:?}"),
    }
}

/// A unit-square patch with the ZERO numerator field (bidegree `(1, 1)`,
/// strictly positive unit weights): the annihilator fixture of the `F`
/// algebra.
fn zero_patch() -> TensorBernsteinPatch {
    let numerator: Vec<Vec<[f64; 3]>> = vec![vec![[0.0; 3]; 2]; 2];
    let weights: Vec<Vec<f64>> = vec![vec![1.0; 2]; 2];
    expect_patch(
        TensorBernsteinPatch::try_new(
            numerator,
            weights,
            patch_fixtures::unit_square_domain(),
            PatchParent::new(0, None),
        ),
        "the zero-field fixture patch",
    )
}

/// The bilinear fixture nets re-encoded over a DIFFERENT source-domain box
/// (`[0, 2]²`): equal grids, unequal spans — the mismatched-spans refusal
/// fixture.
fn bilinear_over_shifted_domain() -> TensorBernsteinPatch {
    let base = patch_fixtures::bilinear_patch();
    expect_patch(
        TensorBernsteinPatch::try_new(
            base.numerator().to_vec(),
            base.weights().to_vec(),
            IBox2 {
                lo: [0.0, 0.0],
                hi: [2.0, 2.0],
            },
            PatchParent::new(0, None),
        ),
        "the shifted-domain bilinear fixture patch",
    )
}

/// The coordinate-axis field of an `R³` grid, as a scalar grid.
fn axis_grid(net: &[Vec<[f64; 3]>], axis: usize) -> Vec<Vec<f64>> {
    net.iter()
        .map(|row| row.iter().map(|point| point[axis]).collect())
        .collect()
}

#[test]
fn product_identity_exact_at_matched_parameters() {
    let bilinear = patch_fixtures::bilinear_patch();
    let octant = patch_fixtures::rational_quadratic_volume_fixture().patch;
    let down_octant = patch_fixtures::seam_pair_fixture().b;
    let cone = patch_fixtures::collapsed_edge_fixture().patch;

    let pairs: [(&str, &TensorBernsteinPatch, &TensorBernsteinPatch); 5] = [
        ("bilinear x bilinear", &bilinear, &bilinear),
        ("octant x octant", &octant, &octant),
        ("octant x bilinear", &octant, &bilinear),
        ("octant x down octant", &octant, &down_octant),
        ("cone x bilinear", &cone, &bilinear),
    ];

    let mut dyadic_worst = 0.0_f64;
    for (name, a, b) in pairs {
        let worst = worst_product_identity_error(a, b);
        assert!(
            worst <= PRODUCT_TOLERANCE,
            "{name}: the product identity fails by {worst} at a matched exact parameter"
        );
        let (ma, na) = a.degree();
        let (mb, nb) = b.degree();
        let product = expect_product(a, b);
        assert_eq!(
            product.degree(),
            (ma + mb, na + nb),
            "{name}: the product degree is recorded, never hidden"
        );
        if name == "bilinear x bilinear" {
            dyadic_worst = worst;
        }
    }
    assert!(
        dyadic_worst <= EXACT_DYADIC_TOLERANCE,
        "the pure-dyadic bilinear square must agree to machine precision, \
         got a worst deviation of {dyadic_worst}"
    );
}

#[test]
fn degree_growth_recorded_not_hidden() {
    let bilinear = patch_fixtures::bilinear_patch();
    let octant = patch_fixtures::rational_quadratic_volume_fixture().patch;
    let zero = zero_patch();

    let cases: [(&str, &TensorBernsteinPatch, &TensorBernsteinPatch); 4] = [
        ("bilinear x bilinear", &bilinear, &bilinear),
        ("octant x octant", &octant, &octant),
        ("octant x bilinear", &octant, &bilinear),
        ("zero x octant", &zero, &octant),
    ];

    for (name, a, b) in cases {
        let (ma, na) = a.degree();
        let (mb, nb) = b.degree();
        let expected = (ma + mb, na + nb);
        let product = expect_product(a, b);
        assert_eq!(
            product.degree(),
            expected,
            "{name}: the grown bidegree is recorded on the patch"
        );
        assert_eq!(
            product.numerator().len(),
            expected.0 + 1,
            "{name}: the numerator row count is the grown degree plus one"
        );
        for row in product.numerator() {
            assert_eq!(
                row.len(),
                expected.1 + 1,
                "{name}: every numerator row has the grown width"
            );
        }
        assert_eq!(
            product.weights().len(),
            expected.0 + 1,
            "{name}: the weight row count is the grown degree plus one"
        );
        for row in product.weights() {
            assert_eq!(
                row.len(),
                expected.1 + 1,
                "{name}: every weight row has the grown width"
            );
        }
        let bracket = product.weight_bracket();
        assert!(
            bracket.lo > 0.0,
            "{name}: the grown weight field still certifies a positive bracket"
        );
        assert_eq!(
            product.domain(),
            a.domain(),
            "{name}: the product patch carries the shared span"
        );
    }

    // A product never shrinks a degree: an axis that grows on one side is
    // still recorded at the full sum.
    let mixed = expect_product(&octant, &bilinear);
    assert_eq!(
        mixed.degree(),
        (3, 3),
        "a (2, 2) x (1, 1) product is recorded at (3, 3), not at the lower degree"
    );
}

#[test]
fn zero_patch_product_is_zero() {
    let zero = zero_patch();
    let bilinear = patch_fixtures::bilinear_patch();
    let octant = patch_fixtures::rational_quadratic_volume_fixture().patch;

    for (name, other) in [("zero x bilinear", &bilinear), ("zero x octant", &octant)] {
        let product = expect_product(&zero, other);
        let (mz, nz) = zero.degree();
        let (mo, no) = other.degree();
        assert_eq!(
            product.degree(),
            (mz + mo, nz + no),
            "{name}: the zero product still records the full degree growth"
        );
        for row in product.numerator() {
            for point in row {
                for (axis, coordinate) in point.iter().enumerate() {
                    assert_eq!(
                        *coordinate, 0.0,
                        "{name}: a zero numerator field annihilates exactly \
                         (coefficient {axis} is {coordinate})"
                    );
                }
            }
        }
        let bracket = product.weight_bracket();
        assert!(
            bracket.lo > 0.0,
            "{name}: the weight field of the zero product stays strictly positive"
        );
        for i in [0, 2, 4, 6, 8] {
            for j in [0, 2, 4, 6, 8] {
                let u = i as f64 / GRID_STEPS as f64;
                let v = j as f64 / GRID_STEPS as f64;
                let p = surface_point(&product, u, v);
                assert_eq!(p, [0.0, 0.0, 0.0], "{name}: the product surface is zero");
            }
        }
    }

    let zero_square = expect_product(&zero, &zero);
    assert_eq!(
        zero_square.degree(),
        (2, 2),
        "zero x zero records the growth"
    );
    for row in zero_square.numerator() {
        for point in row {
            assert_eq!(
                *point,
                [0.0, 0.0, 0.0],
                "zero x zero numerator is exactly zero"
            );
        }
    }
}

#[test]
fn bilinear_square_coefficients_are_exact_dyadics() {
    // The bilinear graph `X(u, v) = (u, v, u·v)` squared: the product surface
    // is the closed form `(u², v², u²v²)` with unit weights, whose Bernstein
    // grids over bidegree `(2, 2)` are the exact dyadic arrays below. On this
    // integer-arithmetic fixture the coefficient convolution is exactly
    // representable, so the returned grids are checked bit for bit.
    let bilinear = patch_fixtures::bilinear_patch();
    let square = expect_product(&bilinear, &bilinear);
    assert_eq!(square.degree(), (2, 2));

    let x_grid = axis_grid(square.numerator(), 0);
    let expected_x: Vec<Vec<f64>> = vec![
        vec![0.0, 0.0, 0.0],
        vec![0.0, 0.0, 0.0],
        vec![1.0, 1.0, 1.0],
    ];
    assert_eq!(
        x_grid, expected_x,
        "the x field of the square is exactly u²"
    );

    let y_grid = axis_grid(square.numerator(), 1);
    let expected_y: Vec<Vec<f64>> = vec![
        vec![0.0, 0.0, 1.0],
        vec![0.0, 0.0, 1.0],
        vec![0.0, 0.0, 1.0],
    ];
    assert_eq!(
        y_grid, expected_y,
        "the y field of the square is exactly v²"
    );

    let z_grid = axis_grid(square.numerator(), 2);
    let expected_z: Vec<Vec<f64>> = vec![
        vec![0.0, 0.0, 0.0],
        vec![0.0, 0.0, 0.0],
        vec![0.0, 0.0, 1.0],
    ];
    assert_eq!(
        z_grid, expected_z,
        "the z field of the square is exactly u²v²"
    );

    let weights = square.weights();
    for row in weights {
        for &w in row {
            assert_eq!(w, 1.0, "the unit weights square to unit weights exactly");
        }
    }
}

#[test]
fn mismatched_spans_refuse_typed() {
    let bilinear = patch_fixtures::bilinear_patch();
    let shifted = bilinear_over_shifted_domain();

    for (a, b) in [(&bilinear, &shifted), (&shifted, &bilinear)] {
        match patch_product(a, b) {
            Ok(_) => panic!("a product over mismatched spans must refuse typed"),
            Err(refusal) => assert_eq!(
                refusal,
                ConstructRefusal::InvalidInput,
                "mismatched spans refuse InvalidInput, never a silent reparameterization"
            ),
        }
    }
}

#[test]
fn degree_elevation_identity_exact() {
    let bilinear = patch_fixtures::bilinear_patch();
    let octant = patch_fixtures::rational_quadratic_volume_fixture().patch;
    let cone = patch_fixtures::collapsed_edge_fixture().patch;

    let cases: [(&str, &TensorBernsteinPatch, (usize, usize)); 4] = [
        ("bilinear to (2, 2)", &bilinear, (2, 2)),
        ("bilinear to (3, 4)", &bilinear, (3, 4)),
        ("octant to (3, 3)", &octant, (3, 3)),
        ("cone to (4, 3)", &cone, (4, 3)),
    ];
    for (name, patch, target) in cases {
        let elevated = expect_elevation(patch, target);
        assert_eq!(
            elevated.degree(),
            target,
            "{name}: the elevation records the target bidegree"
        );
        let bracket = elevated.weight_bracket();
        assert!(
            bracket.lo > 0.0,
            "{name}: the elevated weight field stays strictly positive"
        );
        let worst = worst_elevation_identity_error(patch, target);
        assert!(
            worst <= PRODUCT_TOLERANCE,
            "{name}: the elevation identity fails by {worst} at an exact parameter"
        );
    }
}

#[test]
fn degree_elevation_to_current_degree_is_an_exact_copy() {
    let bilinear = patch_fixtures::bilinear_patch();
    let octant = patch_fixtures::rational_quadratic_volume_fixture().patch;

    for patch in [&bilinear, &octant] {
        let current = patch.degree();
        let elevated = expect_elevation(patch, current);
        assert_eq!(
            elevated.numerator(),
            patch.numerator(),
            "a target at the current degree copies the numerator EXACTLY"
        );
        assert_eq!(
            elevated.weights(),
            patch.weights(),
            "a target at the current degree copies the weights EXACTLY"
        );
        assert_eq!(elevated.degree(), current);
        assert_eq!(elevated.domain(), patch.domain());
        assert_eq!(elevated.parent(), patch.parent());
    }
}

#[test]
fn degree_elevation_below_current_degree_refuses() {
    let bilinear = patch_fixtures::bilinear_patch();
    // The bilinear patch has bidegree (1, 1): any target with an axis below
    // its current degree on that axis refuses typed.
    for target in [(0, 1), (1, 0), (0, 0)] {
        match elevate_degree(&bilinear, target) {
            Ok(_) => panic!("elevation below the current bidegree must refuse typed"),
            Err(refusal) => assert_eq!(
                refusal,
                ConstructRefusal::InvalidInput,
                "elevation below the current bidegree refuses InvalidInput"
            ),
        }
    }
}
