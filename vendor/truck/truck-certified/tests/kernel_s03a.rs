//! BG-KV2-301-S03A integration tests: the §6.3 maximal-minor algebra
//! (Theorem 6.4 as CERTIFIED enclosure machinery), the Tier-1 loop-free
//! certificate (Theorem 9.1, the two-cone LP in cos-space), and the §9.3 R8
//! boundary-stratum seeds (every edge of P against Q is an R8 problem) — over
//! rational, exact ground truths.

#![deny(clippy::unwrap_used)]

use std::fmt::Debug;

use truck_certified::kernel::certs::PointCert3;
use truck_certified::kernel::config;
use truck_certified::kernel::evidence::ClaimVerdict;
use truck_certified::kernel::fixtures;
use truck_certified::kernel::graph::ChartId;
use truck_certified::kernel::leaf::BezierLeaf;
use truck_certified::kernel::minor_algebra;
use truck_certified::kernel::patch::{CertifiedPatch, IBox2, IBox3};
use truck_certified::kernel::residual::ResidualId;
use truck_certified::kernel::residuals_r89::BezierLeaf1;
use truck_certified::kernel::tier1;
use truck_certified::kernel::Interval;

/// The fixture comparison tolerance (dyadic model-space ground truths).
const GT_TOL: f64 = 1e-12; // H-3: dyadic fixture ground-truth comparison tolerance
/// The root-containment slack when comparing a certified box against the
/// known float root (the box bounds are certified, the float root is rounded).
const ROOT_TOL: f64 = 1e-6; // H-3: fixture root-containment slack

/// Extract the `Ok` of any fallible construction; the fixture data is valid by
/// construction, so the refusal arm is a test-bug panic (never an unwrap).
fn construct_ok<T, E: Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(err) => panic!("a construction that must succeed was refused: {err:?}"),
    }
}

/// A 2-axis parameter box.
fn box2(lo: [f64; 2], hi: [f64; 2]) -> IBox2 {
    construct_ok(IBox2::try_new(lo, hi))
}

/// The 3-vector norm (test-only float arithmetic).
fn norm3(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

/// Whether two floats agree to [`GT_TOL`].
fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() <= GT_TOL
}

// ---------------------------------------------------------------------------
// Fixture geometry shared by the minor-algebra tests
// ---------------------------------------------------------------------------

/// The transversal fixture from the shim kit: unit sphere + plane `z = 0`,
/// intersecting in the unit circle. The chart point `(u, v) = (1, 0)` of the
/// sphere maps to the model point `(1, 0, 0)` ON the plane — a transversal
/// crossing at which the exact maximal-minor vector is `m = (0, 1, 0, 1)` and
/// the kernel tangent is `w = (0, 1, 0)`.
fn sphere_plane() -> (
    truck_certified::kernel::leaf::RationalCarrier,
    truck_certified::kernel::leaf::RationalCarrier,
) {
    let fixture = construct_ok(fixtures::transversal_sphere_plane());
    (fixture.sphere, fixture.plane)
}

/// The one-axis interval of a derivative box.
fn axis_iv(b: IBox3, i: usize) -> Interval {
    Interval {
        lo: b.lo[i],
        hi: b.hi[i],
    }
}

/// The interval box of a derivative box as three intervals.
fn box3_ivs(b: IBox3) -> [Interval; 3] {
    [axis_iv(b, 0), axis_iv(b, 1), axis_iv(b, 2)]
}

/// The certified joint Jacobian enclosure `DF = [S¹_u  S¹_v  −S²_s  −S²_t]` of
/// two patches over their product box: rows are the three spatial components,
/// columns the four product-space directions.
fn patch_jac(
    p: &dyn CertifiedPatch,
    dp: IBox2,
    q: &dyn CertifiedPatch,
    dq: IBox2,
) -> [[Interval; 4]; 3] {
    let pde = p.derivs(dp);
    let qde = q.derivs(dq);
    let mut rows = [[Interval::point(0.0); 4]; 3];
    for r in 0..3 {
        rows[r] = [
            axis_iv(pde.su, r),
            axis_iv(pde.sv, r),
            axis_iv(qde.su, r).neg(),
            axis_iv(qde.sv, r).neg(),
        ];
    }
    rows
}

/// The certified float partial matrix of the joint Jacobian at a chart point
/// (the midpoint of the certified enclosure over the degenerate point box).
fn float_jac_at(
    p: &dyn CertifiedPatch,
    up: f64,
    vp: f64,
    q: &dyn CertifiedPatch,
    uq: f64,
    vq: f64,
) -> [[f64; 4]; 3] {
    let d1 = box2([up, vp], [up, vp]);
    let d2 = box2([uq, vq], [uq, vq]);
    let pde = p.derivs(d1);
    let qde = q.derivs(d2);
    let mid = |b: IBox3, i: usize| 0.5 * (b.lo[i] + b.hi[i]);
    let mut rows = [[0.0f64; 4]; 3];
    for r in 0..3 {
        rows[r] = [
            mid(pde.su, r),
            mid(pde.sv, r),
            -mid(qde.su, r),
            -mid(qde.sv, r),
        ];
    }
    rows
}

/// Determinant of a 3x3 float matrix (the landed cofactor order).
fn det3_f(m: [[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

/// The float maximal-minor vector of a 3x4 matrix, with Theorem 6.4's sign
/// pattern (as landed in `ssi_trace.rs` / `engine.rs`).
fn float_kernel_minors(rows: [[f64; 4]; 3]) -> [f64; 4] {
    let minor = |cols: [usize; 3]| -> f64 {
        let mut m = [[0.0f64; 3]; 3];
        for (r, row) in rows.iter().enumerate() {
            for (k, &c) in cols.iter().enumerate() {
                m[r][k] = row[c];
            }
        }
        det3_f(m)
    };
    [
        minor([1, 2, 3]),
        -minor([0, 2, 3]),
        minor([0, 1, 3]),
        -minor([0, 1, 2]),
    ]
}

/// The certified scalar enclosure of `d·b` for a float direction and an
/// interval box.
fn dot_iv_box(d: [f64; 3], b: IBox3) -> Interval {
    let ivs = box3_ivs(b);
    let mut acc = Interval::point(0.0);
    for i in 0..3 {
        acc = acc.add(&Interval::point(d[i]).mul(&ivs[i]));
    }
    acc
}

/// The coordinate-wise `a·x + b·y` of two interval boxes scaled by intervals.
fn scale_add_boxes(a: Interval, x: IBox3, b: Interval, y: IBox3) -> IBox3 {
    let mut lo = [0.0f64; 3];
    let mut hi = [0.0f64; 3];
    for i in 0..3 {
        let t = a.mul(&axis_iv(x, i)).add(&b.mul(&axis_iv(y, i)));
        lo[i] = t.lo;
        hi[i] = t.hi;
    }
    IBox3 { lo, hi }
}

/// An interval midpoint.
fn mid(i: &Interval) -> f64 {
    0.5 * (i.lo + i.hi)
}

// ---------------------------------------------------------------------------
// §1 minimal-minor algebra tests
// ---------------------------------------------------------------------------

#[test]
fn minor_vector_satisfies_df_times_m_is_zero_on_grid() {
    let (sphere, plane) = sphere_plane();
    // A grid of 4x4 cells over a chart neighbourhood of both carriers around
    // the transversal crossing region; on EVERY cell the Theorem 6.4(i)
    // identity must hold: each component of DF·m contains 0.
    let cells = |lo: f64, hi: f64| -> Vec<(f64, f64)> {
        let n = 4usize;
        let step = (hi - lo) / n as f64;
        (0..n)
            .map(|i| {
                let a = lo + i as f64 * step;
                (a, a + step)
            })
            .collect()
    };
    let sphere_cells = cells(0.8, 1.2);
    let v_cells = cells(-0.2, 0.2);
    for (u0, u1) in &sphere_cells {
        for (v0, v1) in &v_cells {
            let dp = box2([*u0, *v0], [*u1, *v1]);
            for (s0, s1) in &sphere_cells {
                for (t0, t1) in &v_cells {
                    let dq = box2([*s0, *t0], [*s1, *t1]);
                    let jac = patch_jac(&sphere, dp, &plane, dq);
                    let m = minor_algebra::minor_vector_encl(&jac);
                    let dfm = minor_algebra::df_times_m(&jac, &m);
                    for r in 0..3 {
                        assert!(
                            dfm[0][r].contains(0.0),
                            "Theorem 6.4(i): (DF·m)[{r}] must contain 0 on every grid cell \
                             (DF·m = 0 identically); cell u:[{u0},{u1}] v:[{v0},{v1}] got {:?}",
                            dfm[0][r]
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn minor_enclosure_brackets_the_float_minor() {
    let (sphere, plane) = sphere_plane();
    // Small product box around the transversal crossing chart point (1, 0).
    let h = 0.0005f64;
    let d1 = box2([1.0 - h, -h], [1.0 + h, h]);
    let d2 = box2([1.0 - h, -h], [1.0 + h, h]);
    let jac = patch_jac(&sphere, d1, &plane, d2);
    let m = minor_algebra::minor_vector_encl(&jac)[0];

    // The certified float maximal-minor vector at the box centre must be
    // bracketed componentwise by the interval enclosure over the whole box.
    let fmat = float_jac_at(&sphere, 1.0, 0.0, &plane, 1.0, 0.0);
    let fm = float_kernel_minors(fmat);
    for j in 0..4 {
        assert!(
            m[j].lo <= fm[j] && fm[j] <= m[j].hi,
            "minor {0}: the float minor {1:.12e} must be bracketed by the enclosure {2:?}",
            j,
            fm[j],
            m[j]
        );
    }
    // Ground truth at the crossing: m = (0, 1, 0, 1), so the enclosure is rank
    // 3 — at least the second component is certified well away from zero, and
    // every enclosure is finite.
    for j in 0..4 {
        assert!(m[j].is_finite(), "minor {j} enclosure must be finite");
    }
    assert!(
        fm[1] > 0.5,
        "the exact minor m1 at the crossing is 1: got {fm:?}"
    );
}

#[test]
fn a_dot_m_matches_d_times_w_theorem_6_5() {
    let (sphere, plane) = sphere_plane();
    let h = 0.0005f64;
    let d1 = box2([1.0 - h, -h], [1.0 + h, h]);
    let d2 = box2([1.0 - h, -h], [1.0 + h, h]);
    let pde = sphere.derivs(d1);
    let jac = patch_jac(&sphere, d1, &plane, d2);
    let m = minor_algebra::minor_vector_encl(&jac);

    // d = (0, 1, 0): at the crossing the kernel tangent is w = S¹_v = (0,1,0),
    // so d·w = 1 exactly.
    let d = [0.0f64, 1.0, 0.0];

    // 4D route: a = (d·S¹_u, d·S¹_v, 0, 0), a·m = d·w (Theorem 6.5).
    let a = [
        dot_iv_box(d, pde.su),
        dot_iv_box(d, pde.sv),
        Interval::point(0.0),
        Interval::point(0.0),
    ];
    let am = minor_algebra::a_dot_m(a, &m);

    // Direct route: w = m0·S¹_u + m1·S¹_v (Theorem 6.5's construction), d·w.
    let w = scale_add_boxes(m[0][0], pde.su, m[0][1], pde.sv);
    let dw = dot_iv_box(d, w);

    // Both routes are certified enclosures of the same exact scalar, which is
    // exactly 1 at the transversal crossing point inside the box.
    assert!(
        am.contains(1.0),
        "a·m must contain the exact value 1: {am:?}"
    );
    assert!(
        dw.contains(1.0),
        "d·w must contain the exact value 1: {dw:?}"
    );
    assert!(am.lo > 0.0, "a·m must be certified positive: {am:?}");
    assert!(dw.lo > 0.0, "d·w must be certified positive: {dw:?}");
    assert!(
        (mid(&am) - mid(&dw)).abs() < 1e-3, // H-3: interval-midpoint agreement tolerance
        "the two routes must agree: a·m mid {} vs d·w mid {}",
        mid(&am),
        mid(&dw)
    );

    // Consistency with Theorem 6.5: w is parallel to n1 × n2 (the certified
    // 3D enclosure of the normal cross product), so d·w has the sign of
    // d·(n1×n2)·c. The geometric cross route must be bounded away from zero.
    let qde = plane.derivs(d2);
    let n1_box = cross_box3(pde.su, pde.sv);
    let n2_box = cross_box3(qde.su, qde.sv);
    let cross_box = cross_box3(n1_box, n2_box);
    let d_cross = dot_iv_box(d, cross_box);
    assert!(
        !d_cross.contains(0.0),
        "the 3D normal-cross enclosure must certify non-parallel normals at the crossing: {d_cross:?}"
    );
}

/// The interval cross product of two interval boxes.
fn cross_box3(a: IBox3, b: IBox3) -> IBox3 {
    let cross = |x: [Interval; 3], y: [Interval; 3]| -> [Interval; 3] {
        [
            x[1].mul(&y[2]).sub(&x[2].mul(&y[1])),
            x[2].mul(&y[0]).sub(&x[0].mul(&y[2])),
            x[0].mul(&y[1]).sub(&x[1].mul(&y[0])),
        ]
    };
    let c = cross(box3_ivs(a), box3_ivs(b));
    IBox3 {
        lo: [c[0].lo, c[1].lo, c[2].lo],
        hi: [c[0].hi, c[1].hi, c[2].hi],
    }
}

// ---------------------------------------------------------------------------
// §2 Tier-1 (Theorem 9.1) tests
// ---------------------------------------------------------------------------

#[test]
fn tier1_lp_feasible_on_transversal_fixture_and_excludes_contact() {
    let fixture = construct_ok(fixtures::transversal_sphere_plane());
    // Narrow sphere normal cone about the +x radial (chart box around (1,0))
    // and the point plane normal cone (0,0,1): the certified cos-space
    // separation must prove a feasible d with a positive lower bound.
    let d1 = box2([0.9, -0.1], [1.1, 0.1]);
    let d2 = box2([0.9, -0.1], [1.1, 0.1]);
    let c1 = fixture.sphere.normal_cone(d1);
    let c2 = fixture.plane.normal_cone(d2);
    match tier1::tier1_loop_free(&c1, &c2) {
        ClaimVerdict::Proven(cert) => {
            assert!(cert.min_dot > 0.0, "min_dot must be certified positive");
            assert!(
                approx(norm3(cert.d), 1.0),
                "the transversal direction must be unit: {:?}",
                cert.d
            );
        }
        ClaimVerdict::Inconclusive(reason) => {
            panic!("the transversal sphere/plane cone pair must certify Proven, inconclusive: {reason}")
        }
        ClaimVerdict::Disproven(_) => panic!("the transversal pair must never be Disproven"),
    }
    // The cone axes are far from parallel; over the whole cone product the
    // normals are certified non-parallel, i.e. contact is excluded.
    let axis_dot = c1.axis[0] * c2.axis[0] + c1.axis[1] * c2.axis[1] + c1.axis[2] * c2.axis[2];
    assert!(
        axis_dot.abs() < 1e-9, // H-3: fixture axis-orthogonality slack
        "the sphere and plane cone axes are orthogonal at this crossing, got {axis_dot}"
    );
}

#[test]
fn tier1_infeasible_on_tangential_fixture() {
    let fixture = construct_ok(fixtures::coaxial_cylinders());
    // Two coincident unit cylinders on the z-axis over the SAME angular box:
    // their normal cones coincide (radial), every cone pair contains parallel
    // normals, and no transversal direction exists.
    let d = box2([0.95, 0.0], [1.05, 0.1]);
    let c1 = fixture.first.normal_cone(d);
    let c2 = fixture.second.normal_cone(d);
    match tier1::tier1_loop_free(&c1, &c2) {
        ClaimVerdict::Proven(cert) => {
            panic!(
                "coaxial coincident cylinders must never certify a transversal direction: {cert:?}"
            )
        }
        ClaimVerdict::Inconclusive(reason) => {
            assert!(
                !reason.is_empty(),
                "an Inconclusive Tier-1 refusal must carry a reason"
            );
        }
        ClaimVerdict::Disproven(_) => {
            // Also an honest infeasible arm; the assertion is that no
            // transversal certificate was issued.
        }
    }
}

// ---------------------------------------------------------------------------
// §9.3 R8 boundary-stratum seed fixtures
// ---------------------------------------------------------------------------

/// The unit-weight plane leaf `S(u, v) = (u, v, 0)` at bidegree `(1, 1)`.
fn plane_leaf() -> BezierLeaf {
    let control = vec![
        [0.0, 0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0, 1.0],
        [1.0, 0.0, 0.0, 1.0],
        [1.0, 1.0, 0.0, 1.0],
    ];
    construct_ok(BezierLeaf::try_new(1, 1, control))
}

/// A degree-1 curve leaf from its two homogeneous endpoints.
fn line_leaf(chart: ChartId, p0: [f64; 4], p1: [f64; 4]) -> BezierLeaf1 {
    construct_ok(BezierLeaf1::try_new(1, vec![p0, p1], chart))
}

/// The four boundary curve leaves of a surface leaf (the u = 0, u = 1, v = 0,
/// v = 1 net boundary rows/columns), homogeneous `xyzw` preserved.
fn boundary_curves(leaf: &BezierLeaf, chart: ChartId) -> Vec<BezierLeaf1> {
    let width = leaf.degree_v + 1;
    let mut out = Vec::with_capacity(4);
    // v = 0 and v = 1 edges: rows over u at fixed column j.
    for j in [0usize, leaf.degree_v] {
        let control: Vec<[f64; 4]> = (0..=leaf.degree_u)
            .map(|i| leaf.control[i * width + j])
            .collect();
        out.push(construct_ok(BezierLeaf1::try_new(
            leaf.degree_u,
            control,
            chart,
        )));
    }
    // u = 0 and u = 1 edges: columns over v at fixed row i.
    for i in [0usize, leaf.degree_u] {
        let control: Vec<[f64; 4]> = (0..=leaf.degree_v)
            .map(|j| leaf.control[i * width + j])
            .collect();
        out.push(construct_ok(BezierLeaf1::try_new(
            leaf.degree_v,
            control,
            chart,
        )));
    }
    out
}

/// The wavy "cylinder" leaf `P(u, v) = (u, y(v), w(u))` at bidegree `(2, 1)`
/// with `y(v) = 3/10 + 2v/5` and `w(u) = (u − 1/3)(u − 2/3)` (control heights
/// `0.2, −0.25, 0.2`): a shallow arch whose `v`-boundary curves pierce the
/// plane `z = 0` exactly twice each (at `u = 1/3` and `u = 2/3`), while its
/// `u`-boundary curves (constant height `0.2`) never do. Every crossing lands
/// at NON-dyadic product-space coordinates (`v = 3/10`, `u = t ≈ 1/3, 2/3`),
/// so the R8 subdivision can certify each one. Ground truth: 4 edge crossings
/// total against the plane, all transverse.
fn wavy_leaf() -> BezierLeaf {
    let x = [0.0, 0.5, 1.0];
    let y = [0.3, 0.7];
    let z = [0.2, -0.25, 0.2];
    let mut control = Vec::with_capacity(6);
    for i in 0..3 {
        for j in 0..2 {
            control.push([x[i], y[j], z[i], 1.0]);
        }
    }
    construct_ok(BezierLeaf::try_new(2, 1, control))
}

/// Assert every collected seed is an R8 certificate with `rho` at the ceiling.
fn assert_r8_seeds(seeds: &[PointCert3]) {
    for seed in seeds {
        assert_eq!(
            seed.residual,
            ResidualId::R8,
            "seeds must be R8 certificates"
        );
        assert!(
            seed.rho <= config::RHO_MAX,
            "every certified seed must satisfy rho <= RHO_MAX: {:?}",
            seed
        );
        assert!(
            seed.box_
                .lo
                .iter()
                .chain(seed.box_.hi.iter())
                .all(|c| c.is_finite()),
            "every certified seed box must be finite"
        );
    }
}

#[test]
fn boundary_seeds_r8_hits_on_known_edge_crossings() {
    // A single transverse edge: the line C(t) = (1/5 + 7t/10, 1/5 + t/2,
    // −1 + 3t) pierces the plane z = 0 exactly once, at t = 1/3, at the model
    // point (13/30, 11/30, 0) — every product-space coordinate is interior and
    // non-dyadic, so the R8 subdivision must certify exactly one seed.
    let chart = ChartId(0);
    let edge = line_leaf(chart, [0.2, 0.2, -1.0, 1.0], [0.9, 0.7, 2.0, 1.0]);
    let plane = plane_leaf();
    let seeds = construct_ok(tier1::boundary_seeds(&plane, &[edge], &plane, &[]));
    assert_r8_seeds(&seeds);
    assert_eq!(
        seeds.len(),
        1,
        "the single crossing must produce exactly one seed"
    );
    // The certified box contains the known root (t, u, v) = (1/3, 13/30, 11/30).
    let root = [1.0 / 3.0, 0.2 + 0.7 / 3.0, 0.2 + 0.5 / 3.0];
    let box_ = seeds[0].box_;
    for axis in 0..3 {
        assert!(
            box_.lo[axis] - ROOT_TOL <= root[axis] && root[axis] <= box_.hi[axis] + ROOT_TOL,
            "certified box must contain the known crossing on axis {axis}: root {root:?} box {box_:?}"
        );
    }
}

#[test]
fn boundary_seed_completeness_matches_oracle_on_fixture() {
    // Plane patch + wavy arch leaf: the arch's boundary curves pierce the
    // plane a KNOWN number of times. Every edge of the arch against the plane
    // (and of the plane against the arch, which has no hits) is solved as R8;
    // the seed count must equal the oracle count of 4, each certified.
    let arch = wavy_leaf();
    let plane = plane_leaf();
    let chart = ChartId(0);
    let arch_edges = boundary_curves(&arch, chart);
    let plane_edges = boundary_curves(&plane, chart);
    assert_eq!(arch_edges.len(), 4);
    assert_eq!(plane_edges.len(), 4);

    let seeds = construct_ok(tier1::boundary_seeds(
        &arch,
        &arch_edges,
        &plane,
        &plane_edges,
    ));
    assert_r8_seeds(&seeds);

    let oracle = 4usize;
    assert_eq!(
        seeds.len(),
        oracle,
        "seeds found must equal the oracle edge-crossing count: got {} seeds {seeds:?}",
        seeds.len()
    );
}

// ---------------------------------------------------------------------------
// N4 source scan over the two new modules
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
fn no_transcendental_call_in_s03a_modules() {
    // N4 / cos-space discipline: neither new module may call sin, cos, atan2,
    // exp, ln, log, or powf outside comments. sqrt is the permitted
    // normalization carve-out (unit direction of the Tier-1 candidate d), so
    // it is deliberately not scanned.
    for path in [
        concat!(env!("CARGO_MANIFEST_DIR"), "/src/kernel/minor_algebra.rs"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/src/kernel/tier1.rs"),
    ] {
        let source = match std::fs::read_to_string(path) {
            Ok(source) => source,
            Err(err) => panic!("{path} must be readable: {err}"),
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
        for needle in ["sin", "cos", "atan2", "exp", "ln", "log", "powf"] {
            let present = code
                .lines()
                .any(|line| contains_word(line, needle) || line.contains("std::f64::consts"));
            assert!(
                !present,
                "no transcendental call may appear outside comments in {path} (found {needle})"
            );
        }
    }
}
