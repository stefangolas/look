//! Integration tests for BG-KV2-102-LEAF: knot-span extraction of homogeneous
//! Bézier leaves and the `CertifiedPatch`/`CertifiedPatchC2` implementation on
//! `BezierLeaf` via interval Bernstein hulls.
//!
//! Ground truths are machine-checked locally (fixed seed, recorded below); the
//! §7.4 straddle fixture data is reused from `kernel::fixtures`.

use std::path::PathBuf;

use truck_certified::kernel::evidence::ClaimVerdict;
use truck_certified::kernel::fixtures;
use truck_certified::kernel::leaf::BezierLeaf;
use truck_certified::kernel::leaf_extract::{extract_bezier_leaves, leaf_from_control};
use truck_certified::kernel::patch::IBox2;
use truck_geometry::prelude::{BSplineSurface, KnotVec, NurbsSurface, ParametricSurface, Vector4};

/// Seed of the fixed random grid of the containment test (recorded; the test
/// must stay deterministic).
const ENCLOSE_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

/// Homogeneous control points in `xyzw`, row-major over `(u, v)`.
fn q(p: [f64; 3], w: f64) -> [f64; 4] {
    [p[0] * w, p[1] * w, p[2] * w, w]
}

fn box2(lo: [f64; 2], hi: [f64; 2]) -> IBox2 {
    IBox2::try_new(lo, hi).expect("test box is well-formed")
}

fn point_box(u: f64, v: f64) -> IBox2 {
    box2([u, v], [u, v])
}

/// Scalar de Casteljau in `f64` (the test's reference evaluator).
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

/// Evaluate a homogeneous `[f64; 4]` control net as a tensor-Bernstein
/// polynomial at `(u, v)` (row-major over `(u, v)`), then dehomogenize.
fn eval_leaf(leaf: &BezierLeaf, u: f64, v: f64) -> [f64; 3] {
    let width = leaf.degree_v + 1;
    let value = |comp: usize| -> f64 {
        let mut cols = Vec::with_capacity(width);
        for j in 0..width {
            let coeffs: Vec<f64> = (0..=leaf.degree_u)
                .map(|i| leaf.control[i * width + j][comp])
                .collect();
            cols.push(eval_bernstein_1d(&coeffs, u));
        }
        eval_bernstein_1d(&cols, v)
    };
    let x = value(0);
    let y = value(1);
    let z = value(2);
    let w = value(3);
    [x / w, y / w, z / w]
}

fn box_contains(p: [f64; 3], b: &truck_certified::kernel::patch::IBox3, slack: f64) -> bool {
    for c in 0..3 {
        if p[c] < b.lo[c] - slack || p[c] > b.hi[c] + slack {
            return false;
        }
    }
    true
}

/// A small deterministic LCG over `(0, 1)`.
struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    fn next(&mut self) -> f64 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let bits = (self.state >> 11) as f64 / (1u64 << 53) as f64;
        if bits <= 0.0 {
            0.01
        } else if bits >= 1.0 {
            0.99
        } else {
            bits
        }
    }
}

/// The parabola-graph degree-`(2, 1)` clamped B-spline surface fixture of the
/// splitting test: `S(u, v) = (u, v, u^2)` reproduced on the unit square by a
/// degree-2 spline in `u` with an interior knot at `0.5`.
fn parabola_surface() -> NurbsSurface<Vector4> {
    let uknots = KnotVec::from(vec![0.0, 0.0, 0.0, 0.5, 1.0, 1.0, 1.0]);
    let vknots = KnotVec::from(vec![0.0, 0.0, 1.0, 1.0]);
    // Greville abscissae of the clamped u knot vector (degree 2).
    let u_greville = [0.0, 0.25, 0.75, 1.0];
    let v_greville = [0.0, 1.0];
    let mut control = Vec::with_capacity(8);
    for &u in &u_greville {
        let mut row = Vec::with_capacity(2);
        for &v in &v_greville {
            row.push(Vector4::new(u, v, u * u, 1.0));
        }
        control.push(row);
    }
    let bsp = BSplineSurface::<Vector4>::new((uknots, vknots), control);
    NurbsSurface::new(bsp)
}

/// The rational Bézier leaf used by the round-trip fixture: degree `(2, 1)`
/// over Bezier knots with weights `[1, 2, 1]` along `u` and `[1, 2]` along
/// `v`; the affine control grid sweeps a saddle `(x, y, z) = (u, v, u v)`.
fn rational_leaf_net() -> Vec<[f64; 4]> {
    let w_uv = [
        [1.0, 2.0], // u row 0
        [2.0, 4.0], // u row 1
        [1.0, 2.0], // u row 2
    ];
    let mut net = Vec::with_capacity(6);
    for i in 0..3 {
        for j in 0..2 {
            let u = i as f64 / 2.0;
            let v = j as f64;
            net.push(q([u, v, u * v], w_uv[i][j]));
        }
    }
    net
}

#[test]
fn leaf_extraction_round_trips_a_bezier_patch_exactly() {
    let net = rational_leaf_net();
    let uknots = KnotVec::from(vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
    let vknots = KnotVec::from(vec![0.0, 0.0, 1.0, 1.0]);
    let control: Vec<Vec<Vector4>> = (0..3)
        .map(|i| {
            (0..2)
                .map(|j| {
                    let p = net[i * 2 + j];
                    Vector4::new(p[0], p[1], p[2], p[3])
                })
                .collect()
        })
        .collect();
    let bsp = BSplineSurface::<Vector4>::new((uknots, vknots), control);
    let surface = NurbsSurface::new(bsp);

    let leaves = extract_bezier_leaves(&surface).expect("a Bezier patch extracts");
    assert_eq!(
        leaves.len(),
        1,
        "a single-span Bezier patch yields one leaf"
    );
    let leaf = &leaves[0];
    assert_eq!(leaf.degree_u, 2);
    assert_eq!(leaf.degree_v, 1);
    // Extraction of an already-Bezier patch performs no knot insertion, so the
    // refined net equals the input exactly.
    for (got, expected) in leaf.control.iter().zip(net.iter()) {
        assert_eq!(got, expected, "control nets equal exactly");
    }
}

#[test]
fn leaf_extraction_splits_a_bspline_span_into_bezier_leaves() {
    let surface = parabola_surface();
    let leaves = extract_bezier_leaves(&surface).expect("the clamped B-spline extracts");
    assert_eq!(
        leaves.len(),
        2,
        "one interior u-knot splits into two leaves"
    );
    for leaf in &leaves {
        assert_eq!(leaf.degree_u, 2);
        assert_eq!(leaf.degree_v, 1);
        assert!(
            leaf.control.iter().all(|p| p[3] == 1.0),
            "a non-rational B-spline lifts to unit weights"
        );
    }

    // Sample the source domain; each sample must be contained in the enclose of
    // the leaf that covers its knot-span cell.
    let u_spans = [0.0, 0.5, 1.0];
    let v_spans = [0.0, 1.0];
    for i in 0..=10 {
        let u = 0.05 + 0.9 * (i as f64) / 10.0;
        for j in 0..=5 {
            let v = 0.1 + 0.8 * (j as f64) / 5.0;
            let pt = surface.subs(u, v);
            let sample = [pt.x, pt.y, pt.z];
            let (ucell, s) = if u < 0.5 {
                (0usize, (u - u_spans[0]) / (u_spans[1] - u_spans[0]))
            } else {
                (1usize, (u - u_spans[1]) / (u_spans[2] - u_spans[1]))
            };
            let t = (v - v_spans[0]) / (v_spans[1] - v_spans[0]);
            let leaf = &leaves[ucell];
            let enc =
                truck_certified::kernel::patch::CertifiedPatch::enclose(leaf, point_box(s, t));
            let scale = 1.0 + sample.iter().map(|c| c.abs()).fold(0.0, f64::max);
            assert!(
                box_contains(sample, &enc, 256.0 * f64::EPSILON * scale),
                "sample {sample:?} at (u={u}, v={v}) escapes leaf {ucell} enclosure {enc:?}"
            );
        }
    }
}

#[test]
fn enclose_contains_every_sampled_point() {
    // Rational leaf with weights `[1,2,1] x [1,2]` (the round-trip net).
    let net = rational_leaf_net();
    let leaf = leaf_from_control(2, 1, net).expect("the rational leaf constructs");
    let mut rng = Lcg::new(ENCLOSE_SEED);
    let mut evaluated = 0usize;
    for _ in 0..512 {
        let u = 0.02 + 0.96 * rng.next();
        let v = 0.02 + 0.96 * rng.next();
        let sample = eval_leaf(&leaf, u, v);
        let enc = truck_certified::kernel::patch::CertifiedPatch::enclose(&leaf, point_box(u, v));
        let scale = 1.0 + sample.iter().map(|c| c.abs()).fold(0.0, f64::max);
        assert!(
            box_contains(sample, &enc, 256.0 * f64::EPSILON * scale),
            "random sample {sample:?} at ({u}, {v}) escapes {enc:?}"
        );
        evaluated += 1;
    }
    assert_eq!(evaluated, 512, "the full recorded grid was evaluated");
}

/// A rational leaf whose surface has a genuinely non-polynomial weight field:
/// degree `(2, 2)`, weights `w(i, j) = 1 + 0.2 * (i + j)` (all positive), and a
/// ruled paraboloid affine grid.
fn curved_rational_leaf() -> BezierLeaf {
    let mut net = Vec::with_capacity(9);
    for i in 0..3 {
        for j in 0..3 {
            let u = i as f64 / 2.0;
            let v = j as f64 / 2.0;
            let w = 1.0 + 0.2 * ((i + j) as f64);
            net.push(q([u, v, 0.25 * (u * u + v * v)], w));
        }
    }
    debug_assert_eq!(net.len(), 9);
    leaf_from_control(2, 2, net).expect("the curved rational leaf constructs")
}

#[test]
fn derivs_enclose_finite_difference_derivatives() {
    let leaf = curved_rational_leaf();
    let h = 0.02;
    let probes = [(0.25, 0.35), (0.4, 0.6), (0.6, 0.25)];
    for (u0, v0) in probes {
        // Forward difference along u at fixed v0.
        let a = eval_leaf(&leaf, u0, v0);
        let b = eval_leaf(&leaf, u0 + h, v0);
        let slope_u = [(b[0] - a[0]) / h, (b[1] - a[1]) / h, (b[2] - a[2]) / h];
        let d = box2([u0, v0], [u0 + h, v0]);
        let de = truck_certified::kernel::patch::CertifiedPatch::derivs(&leaf, d);
        let scale = 1.0 + slope_u.iter().map(|c| c.abs()).fold(0.0, f64::max);
        for c in 0..3 {
            assert!(
                slope_u[c] >= de.su.lo[c] - 512.0 * f64::EPSILON * scale
                    && slope_u[c] <= de.su.hi[c] + 512.0 * f64::EPSILON * scale,
                "u-slope component {c} = {} outside su {:?}",
                slope_u[c],
                de.su
            );
        }

        // Forward difference along v at fixed u0.
        let c = eval_leaf(&leaf, u0, v0 + h);
        let slope_v = [(c[0] - a[0]) / h, (c[1] - a[1]) / h, (c[2] - a[2]) / h];
        let d = box2([u0, v0], [u0, v0 + h]);
        let de = truck_certified::kernel::patch::CertifiedPatch::derivs(&leaf, d);
        let scale = 1.0 + slope_v.iter().map(|x| x.abs()).fold(0.0, f64::max);
        for c in 0..3 {
            assert!(
                slope_v[c] >= de.sv.lo[c] - 512.0 * f64::EPSILON * scale
                    && slope_v[c] <= de.sv.hi[c] + 512.0 * f64::EPSILON * scale,
                "v-slope component {c} = {} outside sv {:?}",
                slope_v[c],
                de.sv
            );
        }
    }
}

#[test]
fn regularity_proven_on_regular_patch_and_disproven_on_degenerate() {
    use truck_certified::kernel::patch::CertifiedPatch;

    // A plane leaf S(u, v) = (u, v, 0): EG - F^2 == 1 everywhere.
    let plane_net = vec![
        [0.0, 0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0, 1.0],
        [1.0, 0.0, 0.0, 1.0],
        [1.0, 1.0, 0.0, 1.0],
    ];
    let plane = leaf_from_control(1, 1, plane_net).expect("the plane leaf constructs");
    let unit = box2([0.0, 0.0], [1.0, 1.0]);
    match CertifiedPatch::regularity(&plane, unit) {
        ClaimVerdict::Proven(positive) => {
            assert!(
                positive.value() > 0.0,
                "a plane is regular with a strictly positive EG - F^2 floor"
            );
        }
        other => panic!("plane regularity must be Proven, got {other:?}"),
    }

    // A collapsed leaf: the two u-rows are identical, so the surface does not
    // vary in u, the cross product vanishes, and EG - F^2 == 0.
    let degenerate_net = vec![
        [0.0, 0.0, 0.0, 1.0],
        [0.0, 1.0, 1.0, 1.0],
        [0.0, 0.0, 0.0, 1.0],
        [0.0, 1.0, 1.0, 1.0],
    ];
    let degenerate =
        leaf_from_control(1, 1, degenerate_net).expect("the collapsed leaf constructs");
    match CertifiedPatch::regularity(&degenerate, unit) {
        ClaimVerdict::Disproven(witness) => {
            assert_eq!(witness.box_, unit);
            // The collapsed edge has EG - F^2 == 0; the interval witness sits at
            // or below the §0.4 singular floor TOL_JACOBIAN.
            assert!(
                witness.egf2.1 < 1e-12,
                "the degenerate enclosure {:?} is below the singular floor",
                witness.egf2
            );
        }
        other => panic!("collapsed-edge regularity must be Disproven, got {other:?}"),
    }
}

/// A gentle positive-weight rational leaf (degree `(2, 1)`, weights
/// `1 + 0.4 * i * j >= 1`) whose weight hull over the whole unit square is
/// certifiably positive under the landed hull kernel.
fn positive_weight_leaf() -> BezierLeaf {
    let mut net = Vec::with_capacity(6);
    for i in 0..3 {
        for j in 0..2 {
            let u = i as f64 / 2.0;
            let v = j as f64;
            let wgt = 1.0 + 0.4 * (i as f64) * (j as f64);
            net.push(q([u, v, 0.2 * u * u], wgt));
        }
    }
    leaf_from_control(2, 1, net).expect("the positive-weight leaf constructs")
}

#[test]
fn weight_bound_proven_on_positive_leaf_and_refuses_straddle() {
    use truck_certified::kernel::patch::CertifiedPatch;

    let unit = box2([0.0, 0.0], [1.0, 1.0]);

    // All-positive weights: the whole-domain weight bound is Proven. (The
    // positive leaf keeps its weight field gentle so the landed hull kernel
    // certifies a positive lower bound over the whole box; heavily sloped
    // weight fields need subdivision, exactly like every other enclosure.)
    let positive = positive_weight_leaf();
    match CertifiedPatch::weight_bound(&positive, unit) {
        Some(ClaimVerdict::Proven(positive_bound)) => {
            assert!(positive_bound.value() > 0.0);
        }
        other => panic!("positive-weight leaf must be Proven, got {other:?}"),
    }

    // §7.4 straddle data (1, -1, 1): the weight net crosses zero at u = 1/2, so
    // over the full box the bound is Inconclusive; the kit's shifted sub-box
    // [0.6, 1] keeps the weight strictly positive.
    let kit = fixtures::weight_straddles_zero().expect("the §7.4 fixture constructs");
    let straddle_net = vec![
        [0.0, 0.0, 0.0, kit.weights[0]],
        [0.0, 0.0, 0.0, kit.weights[0]],
        [0.0, 0.0, 0.0, kit.weights[1]],
        [0.0, 0.0, 0.0, kit.weights[1]],
        [0.0, 0.0, 0.0, kit.weights[2]],
        [0.0, 0.0, 0.0, kit.weights[2]],
    ];
    let straddle = leaf_from_control(2, 1, straddle_net)
        .expect("the pass-through constructor admits the straddle fixture net");
    match CertifiedPatch::weight_bound(&straddle, unit) {
        Some(ClaimVerdict::Inconclusive(_)) => {}
        Some(other) => panic!("the straddle leaf must be Inconclusive over [0,1]^2, got {other:?}"),
        None => panic!("BezierLeaf::weight_bound never returns None"),
    }
    // The straddle fixture's own interval enclosure contains zero.
    assert!(kit.hull_lo <= 0.0 && 0.0 <= kit.hull_hi);
    // On a degenerate sub-box where the weight polynomial is strictly positive
    // (u = 0.8: w = (1 - 2*0.8)^2 = 0.36) the same leaf certifies Proven —
    // the Inconclusive arm above is only the sign-undecidable straddle.
    let positive_spot = point_box(0.8, 0.5);
    match CertifiedPatch::weight_bound(&straddle, positive_spot) {
        Some(ClaimVerdict::Proven(_)) => {}
        Some(other) => panic!("the straddle leaf must be Proven at a positive spot, got {other:?}"),
        None => panic!("BezierLeaf::weight_bound never returns None"),
    }
}

#[test]
fn c2_second_derivs_enclose_second_finite_differences() {
    let leaf = curved_rational_leaf();
    let h = 0.05;
    let probes = [(0.3, 0.3), (0.5, 0.4), (0.45, 0.65)];
    for (u0, v0) in probes {
        // Second central difference along u at fixed v0.
        let lo = eval_leaf(&leaf, u0 - h, v0);
        let mid = eval_leaf(&leaf, u0, v0);
        let hi = eval_leaf(&leaf, u0 + h, v0);
        let d2 = [
            (hi[0] - 2.0 * mid[0] + lo[0]) / (h * h),
            (hi[1] - 2.0 * mid[1] + lo[1]) / (h * h),
            (hi[2] - 2.0 * mid[2] + lo[2]) / (h * h),
        ];
        let d = box2([u0 - h, v0], [u0 + h, v0]);
        let sec = truck_certified::kernel::patch::CertifiedPatchC2::second_derivs(&leaf, d);
        let scale = 1.0 + d2.iter().map(|x| x.abs()).fold(0.0, f64::max);
        for c in 0..3 {
            assert!(
                d2[c] >= sec.suu.lo[c] - 4096.0 * f64::EPSILON * scale
                    && d2[c] <= sec.suu.hi[c] + 4096.0 * f64::EPSILON * scale,
                "second u-difference component {c} = {} outside suu {:?}",
                d2[c],
                sec.suu
            );
        }
    }
}

/// The banned-transcendental token list (N4): `sin | cos | atan2 | exp | ln |
/// log | powf | sqrt`, matched as whole words outside comments.
const BANNED: [&str; 8] = ["sin", "cos", "atan2", "exp", "ln", "log", "powf", "sqrt"];

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

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn contains_banned(code: &str) -> Option<&'static str> {
    let chars: Vec<char> = code.chars().collect();
    for banned in BANNED {
        let token: Vec<char> = banned.chars().collect();
        for (i, &c) in chars.iter().enumerate() {
            if c == token[0] && i + token.len() <= chars.len() {
                let window = &chars[i..i + token.len()];
                if window.iter().collect::<String>() == *banned {
                    let before_ok = i == 0 || !is_word_char(chars[i - 1]);
                    let after_ok =
                        i + token.len() == chars.len() || !is_word_char(chars[i + token.len()]);
                    if before_ok && after_ok {
                        return Some(banned);
                    }
                }
            }
        }
    }
    None
}

#[test]
fn no_transcendental_call_in_leaf_module() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for rel in ["src/kernel/leaf.rs", "src/kernel/leaf_extract.rs"] {
        let path = dir.join(rel);
        let src = std::fs::read_to_string(&path).expect("the leaf module source is readable");
        let code = strip_comments(&src);
        if let Some(token) = contains_banned(&code) {
            panic!("{rel} contains banned transcendental token `{token}` outside comments");
        }
    }
}
