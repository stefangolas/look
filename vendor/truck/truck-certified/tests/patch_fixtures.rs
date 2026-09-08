//! The ADM-SHIM synthetic fixture kit (`tests/patch_fixtures.rs`): the shared
//! extracted-patch fixtures the admission lemma wave parallelizes against.
//!
//! Every lemma packet tests against THESE — one fixture kit, six consumers, no
//! divergent fixtures. The kit ships patches with KNOWN closed forms and
//! invariants, all of which are machine-checked here without invoking any
//! lemma kernel body (the bodies refuse `ConstructRefusal::Unfrozen` until
//! their owning packets land):
//!
//! 1. **bilinear** ([`bilinear_patch`]) — the bidegree-`(1,1)` graph
//!    `X(u, v) = (u, v, u·v)` with all weights `1`.
//! 2. **rational quadratic volume** ([`rational_quadratic_volume_fixture`]) —
//!    the exact biquadratic NURBS octant of the unit sphere (a genuine
//!    rational patch, non-constant positive weights), whose enclosing-solid
//!    volume is the exactly-computable closed form `π/6`
//!    ([`SPHERE_OCTANT_VOLUME`]) and whose points all satisfy `|X|² = 1`.
//! 3. **collapsed edge** ([`collapsed_edge_fixture`]) — the quarter-cone
//!    `X(u, v) = P + (1−v)²·(B(u) − P)` whose `v = 1` edge collapses to the
//!    apex `P`; the normal numerator carries the known factor `(1−v)³`, so the
//!    recorded deflation multiplicity is `3`.
//! 4. **seam-identified pair** ([`seam_pair_fixture`]) — the up and down
//!    octants of the unit sphere sharing the same base quarter-arc: the shared
//!    boundary coefficient rows are exactly equal (aligned degrees and
//!    parameterizations) while the interiors differ.
//! 5. **weight-bracket violation** ([`weight_bracket_violating_patch_data`]) —
//!    a net whose weight field is genuinely non-positive on its domain; the
//!    refusing constructor must reject it with
//!    [`ConstructRefusal::InvalidInput`] — never a silent non-positive weight.
//!
//! The two ADM-SHIM required tests live here:
//! [`fixture_patches_satisfy_closed_form_invariants`] (the machine checks
//! above) and [`refusing_kernels_refuse`] (every lemma kernel refuses
//! `Unfrozen`, and the Theorem D signature alias [`CertifiedReciprocalPower`]
//! is nameable against the evidence layer's types).
//!
//! TEST SUPPORT ONLY: this is an integration-test fixture kit. The builders
//! are `pub` so later lemma test files can reuse them (the BIE/construct
//! `fixtures` precedent at the file level). No solver is called anywhere.

use truck_certified::construct::admission::EdgeId;
use truck_certified::construct::patches::{
    deflate_factor, extract_patches, normal_numerator, patch_product, seam_identified,
    CertifiedReciprocalPower, PatchParent, PatchSide, TensorBernsteinPatch,
};
use truck_certified::construct::refusal::ConstructRefusal;
use truck_certified::kernel::patch::IBox2;
use truck_evidence::contact::implicit2d::ScalarNet2;
use truck_geometry::prelude::{BSplineSurface, KnotVec, Vector4};

/// The quarter-circle middle weight `√2/2` of the exact NURBS arc nets (the
/// exact float value of `cos(π/4)`; H-3: named, never a bare literal).
const SQRT_HALF: f64 = std::f64::consts::FRAC_1_SQRT_2;

/// The exactly-computable volume of the unit-sphere octant region: the solid
/// bounded by the octant surface and the three coordinate planes has volume
/// `(4π/3)/8 = π/6` (closed form; the stored float is the correctly rounded
/// evaluation, H-6 `Method::Float`).
pub const SPHERE_OCTANT_VOLUME: f64 = std::f64::consts::PI / 6.0;

/// Unit-scale position tolerance for the closed-form machine checks (the
/// fixtures are unit-scale dyadic-rational geometry; H-3: named).
const POS_TOL: f64 = 1.0e-9;

/// The axis weights of every fixture's arcs: `1` at the arc ends, `√2/2` at
/// the middle (H-3: named constant, never a bare literal).
const ARC_WEIGHTS: [f64; 3] = [1.0, SQRT_HALF, 1.0];

/// A raw patch input record: the `(numerator, weights, domain, parent)` data a
/// [`TensorBernsteinPatch::try_new`] call consumes. Used by the fixture kit
/// for the refusing weight-bracket-violating case, where no patch value can
/// exist.
#[derive(Debug, Clone, PartialEq)]
pub struct PatchData {
    /// The `(m+1)×(n+1)` numerator grid `Â` (rows over `u`).
    pub numerator: Vec<Vec<[f64; 3]>>,
    /// The `(m+1)×(n+1)` weight grid `Ŵ`.
    pub weights: Vec<Vec<f64>>,
    /// The span's source-domain box.
    pub domain: IBox2,
    /// The parent face (and optional boundary edge) identity.
    pub parent: PatchParent,
}

/// The unit square `[0, 1]²` as a domain box (every fixture's span domain).
pub fn unit_square_domain() -> IBox2 {
    IBox2 {
        lo: [0.0, 0.0],
        hi: [1.0, 1.0],
    }
}

/// The rational-quadratic volume fixture record: the exact biquadratic NURBS
/// octant of the unit sphere together with its exactly-computable closed-form
/// volume.
#[derive(Debug, Clone, PartialEq)]
pub struct RationalQuadraticVolumeFixture {
    /// The octant patch itself (bidegree `(2, 2)`, strictly positive weights).
    pub patch: TensorBernsteinPatch,
    /// The exactly-computable volume of the solid the octant bounds with the
    /// coordinate planes: [`SPHERE_OCTANT_VOLUME`].
    pub closed_form_volume: f64,
}

/// Fixture 2: the rational quadratic volume patch.
///
/// The biquadratic NURBS octant of the unit sphere (surface of revolution of
/// the exact rational-quadratic quarter arc by a quarter turn). It is a
/// genuine rational patch — the weight net is `w_i·ω_j` with `w = ω =
/// (1, √2/2, 1)`, so the weight field is non-constant — and every surface
/// point satisfies `x² + y² + z² = 1`. The solid between the octant and the
/// planes `x = 0`, `y = 0`, `z = 0` has the exactly computable volume `π/6`.
pub fn rational_quadratic_volume_fixture() -> RationalQuadraticVolumeFixture {
    let (numerator, weights) = sphere_octant_nets(1.0);
    RationalQuadraticVolumeFixture {
        patch: expect_patch(
            TensorBernsteinPatch::try_new(
                numerator,
                weights,
                unit_square_domain(),
                PatchParent::new(0, None),
            ),
            "the rational-quadratic volume fixture patch",
        ),
        closed_form_volume: SPHERE_OCTANT_VOLUME,
    }
}

/// The collapsed-edge fixture record: the quarter-cone patch whose `v = 1`
/// edge collapses to the apex `P`, with the known deflation multiplicity of
/// its boundary.
#[derive(Debug, Clone, PartialEq)]
pub struct CollapsedEdgeFixture {
    /// The collapsed-edge cone patch (bidegree `(2, 2)`).
    pub patch: TensorBernsteinPatch,
    /// The side of the patch whose whole edge maps to the apex.
    pub collapsed_side: PatchSide,
    /// The apex point `P` every `v = 1` span maps to.
    pub apex: [f64; 3],
    /// The multiplicity `k` of the collapsed-boundary factor `(1−v)^k` in the
    /// normal numerator.
    pub multiplicity: usize,
}

/// Fixture 3: the collapsed-edge patch.
///
/// The quarter cone over the base quarter-arc `B(u)` (radius 1 in the plane
/// `z = 0`) with apex `P = (0,0,1)` and the quadratic profile
/// `X(u, v) = P + (1−v)²·(B(u) − P)`. The `v = 1` edge maps the whole span to
/// the apex (the intentional collapse), while `X(u, 0)` is the base arc.
/// Because `X_u×X_v = −2(1−v)³·B′×(B−P)` and the weight field does not
/// vanish, the normal numerator `M = W³(X_u×X_v)` carries the known factor
/// `(1−v)³`: the recorded deflation multiplicity is `3`.
pub fn collapsed_edge_fixture() -> CollapsedEdgeFixture {
    let apex: [f64; 3] = [0.0, 0.0, 1.0];
    let (numerator, weights) = apex_cone_nets(&apex);
    CollapsedEdgeFixture {
        patch: expect_patch(
            TensorBernsteinPatch::try_new(
                numerator,
                weights,
                unit_square_domain(),
                PatchParent::new(0, Some(EdgeId(0))),
            ),
            "the collapsed-edge fixture patch",
        ),
        collapsed_side: PatchSide::SideVMax,
        apex,
        multiplicity: 3,
    }
}

/// The seam-identified pair fixture record: two octants of the unit sphere
/// (up and down) sharing the same base quarter-arc as their common boundary.
#[derive(Debug, Clone, PartialEq)]
pub struct SeamPairFixture {
    /// The up octant (z ≥ 0), carrying one seam edge id.
    pub a: TensorBernsteinPatch,
    /// The down octant (z ≤ 0), carrying the paired seam edge id.
    pub b: TensorBernsteinPatch,
    /// The paired BRep edges of the certified seam (the ADM-000
    /// [`SeamIdentified`](truck_certified::construct::admission::SeamIdentified)
    /// shape).
    pub paired: (EdgeId, EdgeId),
    /// The shared side of `a` that carries the seam.
    pub seam_side_a: PatchSide,
    /// The shared side of `b` that carries the seam.
    pub seam_side_b: PatchSide,
}

/// Fixture 4: the seam-identified pair.
///
/// `a` is the up octant (z ≥ 0) and `b` the down octant (z ≤ 0) of the unit
/// sphere. Both share the same base quarter-arc in the plane `z = 0` as their
/// `u = 0` boundary row: the numerator and weight coefficient rows are EXACTLY
/// equal (aligned bidegrees and parameterizations), so
/// `A_a·W_b − A_b·W_a ≡ 0` along the seam — while the interiors are opposite
/// octants. This is the exact shared-boundary shape the L4
/// [`seam_identified`] verdict certifies.
pub fn seam_pair_fixture() -> SeamPairFixture {
    let (num_up, w_up) = sphere_octant_nets(1.0);
    let (num_down, w_down) = sphere_octant_nets(-1.0);
    SeamPairFixture {
        a: expect_patch(
            TensorBernsteinPatch::try_new(
                num_up,
                w_up,
                unit_square_domain(),
                PatchParent::new(0, Some(EdgeId(0))),
            ),
            "the seam pair up-octant patch",
        ),
        b: expect_patch(
            TensorBernsteinPatch::try_new(
                num_down,
                w_down,
                unit_square_domain(),
                PatchParent::new(1, Some(EdgeId(1))),
            ),
            "the seam pair down-octant patch",
        ),
        paired: (EdgeId(0), EdgeId(1)),
        seam_side_a: PatchSide::SideUMin,
        seam_side_b: PatchSide::SideUMin,
    }
}

/// Fixture 1: the bilinear patch `X(u, v) = (u, v, u·v)`.
///
/// Bidegree `(1, 1)`, all weights `1` (the polynomial special case of the
/// rational patch type). Closed form: the four corners are `(0,0,0)`,
/// `(1,0,0)`, `(0,1,0)`, `(1,1,1)` and the surface is the bilinear graph
/// `z = u·v`.
pub fn bilinear_patch() -> TensorBernsteinPatch {
    let numerator: Vec<Vec<[f64; 3]>> = vec![
        vec![[0.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        vec![[1.0, 0.0, 0.0], [1.0, 1.0, 1.0]],
    ];
    let weights: Vec<Vec<f64>> = vec![vec![1.0, 1.0], vec![1.0, 1.0]];
    expect_patch(
        TensorBernsteinPatch::try_new(
            numerator,
            weights,
            unit_square_domain(),
            PatchParent::new(0, None),
        ),
        "the bilinear fixture patch",
    )
}

/// Fixture 5: the weight-bracket-violating patch DATA.
///
/// The weight grid `[[1, 1], [1, −1/2]]` has a strictly negative corner
/// coefficient: the bilinear weight field takes the value `−1/2` at `(u, v) =
/// (1, 1)`, so its Bernstein hull cannot certify a strictly positive bracket.
/// [`TensorBernsteinPatch::try_new`] must refuse this input with
/// [`ConstructRefusal::InvalidInput`] — no patch value exists for it.
pub fn weight_bracket_violating_patch_data() -> PatchData {
    PatchData {
        numerator: vec![
            vec![[0.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            vec![[1.0, 0.0, 0.0], [1.0, 1.0, 1.0]],
        ],
        weights: vec![vec![1.0, 1.0], vec![1.0, -0.5]],
        domain: unit_square_domain(),
        parent: PatchParent::new(0, None),
    }
}

/// The exact biquadratic NURBS nets of one octant of the unit sphere.
///
/// `z_sign = +1` yields the up octant (apex `(0,0,1)`), `z_sign = −1` the down
/// octant (apex `(0,0,−1)`); both share the base quarter-arc in `z = 0`. The
/// surface is the surface of revolution of the exact rational-quadratic
/// quarter arc by a quarter turn: the `u` direction is the profile (rows), the
/// `v` direction the revolution (columns). `A[i][j] = w_i·ω_j·Q_ij` and
/// `W[i][j] = w_i·ω_j` with `w = ω = (1, √2/2, 1)`.
fn sphere_octant_nets(z_sign: f64) -> (Vec<Vec<[f64; 3]>>, Vec<Vec<f64>>) {
    // The profile quarter arc in the plane y = 0 (control points, y-coordinate
    // zero) and the unit quarter-turn circle control points.
    let profile: [[f64; 3]; 3] = [[1.0, 0.0, 0.0], [1.0, 0.0, z_sign], [0.0, 0.0, z_sign]];
    let turn: [[f64; 2]; 3] = [[1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    #[allow(clippy::needless_range_loop)]
    // fixed 3x3 NURBS net assembly; the indices are the grid axes
    // Rows run over the profile (u), columns over the revolution (v).
    let mut numerator: Vec<Vec<[f64; 3]>> = Vec::new();
    let mut weights: Vec<Vec<f64>> = Vec::new();
    for i in 0..3 {
        let mut row: Vec<[f64; 3]> = Vec::new();
        let mut wrow: Vec<f64> = Vec::new();
        for j in 0..3 {
            let x = profile[i][0] * turn[j][0];
            let y = profile[i][0] * turn[j][1];
            let z = profile[i][2];
            let w = ARC_WEIGHTS[i] * ARC_WEIGHTS[j];
            row.push([w * x, w * y, w * z]);
            wrow.push(w);
        }
        numerator.push(row);
        weights.push(wrow);
    }
    (numerator, weights)
}

/// The exact biquadratic nets of the apex-collapsed quarter cone
/// `X(u, v) = P + (1−v)²·(B(u) − P)`.
///
/// `B(u)` is the base quarter-arc of radius 1 in `z = 0` (the same arc net as
/// the octant base), `P` the apex. In the degree-2 `v` basis the numerator
/// coefficient columns are `B(u)`'s homogeneous controls at `v = 0` and the
/// apex control `w_i·P` at both higher `v` columns; the weight field is the
/// base arc's weight field (constant in `v`).
fn apex_cone_nets(apex: &[f64; 3]) -> (Vec<Vec<[f64; 3]>>, Vec<Vec<f64>>) {
    let base: [[f64; 3]; 3] = [[1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0]];
    #[allow(clippy::needless_range_loop)]
    // fixed 3x3 NURBS net assembly; the indices are the grid axes
    let mut numerator: Vec<Vec<[f64; 3]>> = Vec::new();
    let mut weights: Vec<Vec<f64>> = Vec::new();
    for i in 0..3 {
        let w = ARC_WEIGHTS[i];
        let base_col = [w * base[i][0], w * base[i][1], w * base[i][2]];
        let apex_col = [w * apex[0], w * apex[1], w * apex[2]];
        numerator.push(vec![base_col, apex_col, apex_col]);
        weights.push(vec![w, w, w]);
    }
    (numerator, weights)
}

/// Extract the `Ok` of a fixture patch construction; the fixture data is valid
/// by construction, so the refusal arm is a kit-bug panic (never an unwrap).
fn expect_patch(
    result: Result<TensorBernsteinPatch, ConstructRefusal>,
    what: &str,
) -> TensorBernsteinPatch {
    match result {
        Ok(patch) => patch,
        Err(refusal) => panic!("{what} refused construction: {refusal:?}"),
    }
}

// ---------------------------------------------------------------------------
// Test-only evaluation helpers (plain f64 Bernstein arithmetic; never a
// shipped kernel — the fixture kit machine-checks closed forms without
// invoking any lemma body).
// ---------------------------------------------------------------------------

/// The integer binomial `C(n, k)` (small exact arithmetic for the fixture
/// degrees).
fn binomial(n: usize, k: usize) -> usize {
    let mut r = 1usize;
    for i in 1..=k {
        r = r * (n - k + i) / i;
    }
    r
}

/// The degree-`degree` Bernstein basis values at `t`.
fn bernstein_basis(degree: usize, t: f64) -> Vec<f64> {
    let mut values = Vec::with_capacity(degree + 1);
    for k in 0..=degree {
        let c = binomial(degree, k) as f64;
        values.push(c * t.powi(k as i32) * (1.0 - t).powi((degree - k) as i32));
    }
    values
}

/// Scalar tensor-Bernstein evaluation of a row-major grid at `(u, v)`.
fn eval_scalar_net(grid: &[Vec<f64>], u: f64, v: f64) -> f64 {
    let rows = grid.len();
    let cols = grid[0].len();
    let bu = bernstein_basis(rows - 1, u);
    let bv = bernstein_basis(cols - 1, v);
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
    let rows = grid.len();
    let cols = grid[0].len();
    let bu = bernstein_basis(rows - 1, u);
    let bv = bernstein_basis(cols - 1, v);
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

/// Machine-check that two scalars agree within the fixture tolerance.
fn assert_close(a: f64, b: f64, what: &str) {
    assert!(
        (a - b).abs() <= POS_TOL,
        "{what}: {a} diverged from {b} by {}",
        (a - b).abs()
    );
}

/// Machine-check that two points agree within the fixture tolerance.
fn assert_point_close(a: [f64; 3], b: [f64; 3], what: &str) {
    for k in 0..3 {
        assert_close(a[k], b[k], &format!("{what} coordinate {k}"));
    }
}

/// The shared structural invariants every ACCEPTED fixture patch must satisfy:
/// the bidegree matches the grid shapes, the certified weight bracket is
/// strictly positive and encloses sampled values of the weight field, and the
/// numerator/weight grids share one rectangular shape.
fn check_patch_invariants(patch: &TensorBernsteinPatch, what: &str) {
    let (m, n) = patch.degree();
    let rows = m + 1;
    let cols = n + 1;
    assert_eq!(patch.numerator().len(), rows, "{what}: numerator row count");
    assert_eq!(patch.weights().len(), rows, "{what}: weight row count");
    for row in 0..rows {
        assert_eq!(
            patch.numerator()[row].len(),
            cols,
            "{what}: numerator row {row} width"
        );
        assert_eq!(
            patch.weights()[row].len(),
            cols,
            "{what}: weight row {row} width"
        );
    }
    let bracket = patch.weight_bracket();
    assert!(
        bracket.lo > 0.0 && bracket.lo <= bracket.hi,
        "{what}: the certified weight bracket must be strictly positive, got [{}, {}]",
        bracket.lo,
        bracket.hi
    );
    // The bracket is the exact Bernstein hull of the weight coefficients
    // (the convex-hull certificate): recomputing the axis hull must reproduce
    // it exactly.
    let mut w_lo = f64::INFINITY;
    let mut w_hi = f64::NEG_INFINITY;
    for row in patch.weights() {
        for &w in row {
            if w < w_lo {
                w_lo = w;
            }
            if w > w_hi {
                w_hi = w;
            }
        }
    }
    assert_eq!(
        bracket.lo, w_lo,
        "{what}: bracket lower bound equals the hull minimum"
    );
    assert_eq!(
        bracket.hi, w_hi,
        "{what}: bracket upper bound equals the hull maximum"
    );
    for u in [0.0, 0.25, 0.5, 0.75, 1.0] {
        for v in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let sample = eval_scalar_net(patch.weights(), u, v);
            assert!(
                bracket.lo <= sample && sample <= bracket.hi,
                "{what}: weight sample {sample} at ({u}, {v}) escaped the certified \
                 bracket [{}, {}]",
                bracket.lo,
                bracket.hi
            );
        }
    }
}

#[test]
fn fixture_patches_satisfy_closed_form_invariants() {
    // Fixture 1: the bilinear graph X(u, v) = (u, v, u·v).
    let bilinear = bilinear_patch();
    check_patch_invariants(&bilinear, "bilinear");
    assert_eq!(bilinear.degree(), (1, 1), "bilinear bidegree");
    for (u, v) in [
        (0.0, 0.0),
        (1.0, 0.0),
        (0.0, 1.0),
        (1.0, 1.0),
        (0.25, 0.5),
        (0.5, 0.5),
        (0.7, 0.3),
        (1.0, 1.0),
    ] {
        let p = surface_point(&bilinear, u, v);
        assert_point_close(p, [u, v, u * v], &format!("bilinear graph at ({u}, {v})"));
    }

    // Fixture 2: the rational quadratic volume patch (unit-sphere octant).
    let volume = rational_quadratic_volume_fixture();
    check_patch_invariants(&volume.patch, "volume octant");
    assert_eq!(volume.patch.degree(), (2, 2), "octant bidegree");
    assert_eq!(
        volume.closed_form_volume, SPHERE_OCTANT_VOLUME,
        "the recorded closed-form volume is the exactly-computable octant volume"
    );
    for (u, v) in [
        (0.0, 0.0),
        (0.0, 1.0),
        (0.25, 0.75),
        (0.5, 0.5),
        (0.75, 0.25),
        (0.5, 0.0),
        (0.0, 0.5),
    ] {
        let p = surface_point(&volume.patch, u, v);
        let radius_sq = p[0] * p[0] + p[1] * p[1] + p[2] * p[2];
        assert!(
            (radius_sq - 1.0).abs() <= POS_TOL,
            "octant point at ({u}, {v}) left the unit sphere: |X|² = {radius_sq}"
        );
    }
    // The stored closed form is exactly the analytic octant volume π/6.
    assert_close(
        volume.closed_form_volume,
        std::f64::consts::PI / 6.0,
        "octant volume closed form",
    );

    // Fixture 3: the collapsed-edge patch (apex at v = 1).
    let collapsed = collapsed_edge_fixture();
    check_patch_invariants(&collapsed.patch, "collapsed cone");
    assert_eq!(collapsed.collapsed_side, PatchSide::SideVMax);
    assert_eq!(
        collapsed.multiplicity, 3,
        "the (1−v)³ deflation multiplicity"
    );
    assert_point_close(collapsed.apex, [0.0, 0.0, 1.0], "recorded apex");
    for u in [0.0, 0.25, 0.5, 0.75, 1.0] {
        // v = 1: the whole span collapses to the apex.
        let apex_point = surface_point(&collapsed.patch, u, 1.0);
        assert_point_close(
            apex_point,
            collapsed.apex,
            &format!("apex collapse at u = {u}"),
        );
        // v = 0: the base quarter arc of radius 1 in z = 0.
        let base_point = surface_point(&collapsed.patch, u, 0.0);
        assert_close(base_point[2], 0.0, &format!("base plane at u = {u}"));
        let radial_sq = base_point[0] * base_point[0] + base_point[1] * base_point[1];
        assert!(
            (radial_sq - 1.0).abs() <= POS_TOL,
            "base point at u = {u} left the unit circle: x² + y² = {radial_sq}"
        );
        // The intermediate spans follow the closed form
        // X(u, v) = P + (1−v)²·(B(u) − P) with B(u) = X(u, 0).
        for v in [0.25, 0.5, 0.75] {
            let p = surface_point(&collapsed.patch, u, v);
            let scale = (1.0 - v) * (1.0 - v);
            let expected = [
                collapsed.apex[0] + scale * (base_point[0] - collapsed.apex[0]),
                collapsed.apex[1] + scale * (base_point[1] - collapsed.apex[1]),
                collapsed.apex[2] + scale * (base_point[2] - collapsed.apex[2]),
            ];
            assert_point_close(p, expected, &format!("cone closed form at ({u}, {v})"));
        }
    }

    // Fixture 4: the seam-identified pair (shared base arc, distinct octants).
    let seam = seam_pair_fixture();
    check_patch_invariants(&seam.a, "seam up octant");
    check_patch_invariants(&seam.b, "seam down octant");
    assert_eq!(seam.seam_side_a, PatchSide::SideUMin);
    assert_eq!(seam.seam_side_b, PatchSide::SideUMin);
    // The shared u = 0 boundary rows are EXACTLY equal (aligned degrees and
    // parameterizations): A_a·W_b − A_b·W_a ≡ 0 along the seam.
    assert_eq!(
        seam.a.numerator()[0],
        seam.b.numerator()[0],
        "shared seam numerator row"
    );
    assert_eq!(
        seam.a.weights()[0],
        seam.b.weights()[0],
        "shared seam weight row"
    );
    for v in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let pa = surface_point(&seam.a, 0.0, v);
        let pb = surface_point(&seam.b, 0.0, v);
        assert_point_close(pa, pb, &format!("shared seam point at v = {v}"));
        let radial_sq = pa[0] * pa[0] + pa[1] * pa[1];
        assert!(
            (radial_sq - 1.0).abs() <= POS_TOL && pa[2].abs() <= POS_TOL,
            "seam point at v = {v} left the shared base arc"
        );
    }
    // The interiors are genuinely distinct patches: the up octant has z ≥ 0
    // and the down octant z ≤ 0 at the same interior parameter.
    let interior = (0.5, 0.5);
    let up = surface_point(&seam.a, interior.0, interior.1);
    let down = surface_point(&seam.b, interior.0, interior.1);
    assert!(
        up[2] > POS_TOL && down[2] < -POS_TOL,
        "the seam pair interiors must be opposite octants, got z = {} and {}",
        up[2],
        down[2]
    );

    // Fixture 5: the weight-bracket-violating patch data refuses.
    let violating = weight_bracket_violating_patch_data();
    // The violating weight field is genuinely non-positive on its domain: at
    // (u, v) = (1, 1) the bilinear weight field equals the −1/2 coefficient.
    assert!(eval_scalar_net(&violating.weights, 1.0, 1.0) < 0.0);
    match TensorBernsteinPatch::try_new(
        violating.numerator,
        violating.weights,
        violating.domain,
        violating.parent,
    ) {
        Ok(_) => panic!("a weight-bracket-violating patch must refuse construction"),
        Err(refusal) => assert_eq!(
            refusal,
            ConstructRefusal::InvalidInput,
            "a non-certifiable weight bracket refuses InvalidInput, never a silent \
             non-positive weight"
        ),
    }
}

#[test]
fn refusing_kernels_refuse() {
    let bilinear = bilinear_patch();
    let volume = rational_quadratic_volume_fixture();
    let collapsed = collapsed_edge_fixture();
    let seam = seam_pair_fixture();

    // A positive-weight homogeneous tensor-spline face for the L1 kernel: the
    // octant patch as a single-span clamped biquadratic BSplineSurface<Vector4>.
    let octant = volume.patch;
    let mut controls: Vec<Vec<Vector4>> = Vec::new();
    for (i, row) in octant.numerator().iter().enumerate() {
        let mut control_row: Vec<Vector4> = Vec::new();
        for (j, a) in row.iter().enumerate() {
            control_row.push(Vector4::new(a[0], a[1], a[2], octant.weights()[i][j]));
        }
        controls.push(control_row);
    }
    let knots = (
        KnotVec::from(vec![0.0f64, 0.0, 0.0, 1.0, 1.0, 1.0]),
        KnotVec::from(vec![0.0f64, 0.0, 0.0, 1.0, 1.0, 1.0]),
    );
    let face = BSplineSurface::new(knots, controls);

    assert_eq!(
        extract_patches(&face),
        Err(ConstructRefusal::Unfrozen),
        "extract_patches (L1) refuses until its lemma packet lands"
    );
    assert_eq!(
        patch_product(&bilinear, &bilinear),
        Err(ConstructRefusal::Unfrozen),
        "patch_product (L2) refuses until its lemma packet lands"
    );
    assert_eq!(
        normal_numerator(&bilinear),
        Err(ConstructRefusal::Unfrozen),
        "normal_numerator (L3) refuses until its lemma packet lands"
    );
    assert_eq!(
        deflate_factor(&collapsed.patch, PatchSide::SideVMax),
        Err(ConstructRefusal::Unfrozen),
        "deflate_factor (L4) refuses until its lemma packet lands"
    );
    assert_eq!(
        seam_identified(&seam.a, &seam.b),
        Err(ConstructRefusal::Unfrozen),
        "seam_identified (L4) refuses until its lemma packet lands"
    );
}

/// A local stub matching the frozen Theorem D signature shape, used to
/// compile-check that the [`CertifiedReciprocalPower`] alias is nameable
/// against the evidence layer's types (the L5 body lands in truck-evidence).
fn certified_reciprocal_power_stub(
    _w: &ScalarNet2,
    _p: u32,
    _target_error: f64,
) -> Result<(ScalarNet2, f64), truck_base::evidence::Refusal> {
    Err(truck_base::evidence::Refusal::Empty)
}

#[test]
fn certified_reciprocal_power_signature_is_nameable() {
    // The type-alias freeze (Theorem D, L5) must be a well-formed function
    // signature over the evidence layer's nameable types.
    let _signature: CertifiedReciprocalPower = certified_reciprocal_power_stub;
}
