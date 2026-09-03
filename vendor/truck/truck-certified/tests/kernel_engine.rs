//! BG-KV2-201-S2A integration tests: Lemma 8.0's rho, the generic square C1,
//! the C2 tube over a nontrivial tau interval, and frame construction
//! (spec Â§7.1/Â§8.1â€“Â§8.3; packet Section 4).

#![deny(clippy::unwrap_used)]

use std::fmt::Debug;

use truck_certified::kernel::certs::Frame;
use truck_certified::kernel::config;
use truck_certified::kernel::engine::{
    build_frame4, c2_certify_tube4, krawczyk_c1, SquareResidualEval,
};
use truck_certified::kernel::evidence::{ClaimVerdict, RefusalKind};
use truck_certified::kernel::leaf::BezierLeaf;
use truck_certified::kernel::patch::{CertifiedPatch, CertifiedPositive, IBox2};
use truck_certified::kernel::Interval;
use truck_certified::SquareSystem3;

/// The fixture comparison tolerance (H-3).
const GT: f64 = 1e-9; // H-3: fixture ground-truth comparison tolerance

/// Extract the `Ok` of any fallible construction; the fixture data is valid by
/// construction, so the refusal arm is a test-bug panic (never an unwrap).
fn construct_ok<T, E: Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(err) => panic!("a construction that must succeed was refused: {err:?}"),
    }
}

/// A parameter box `[u_lo, u_hi] x [v_lo, v_hi]`.
fn box2(u_lo: f64, u_hi: f64, v_lo: f64, v_hi: f64) -> IBox2 {
    construct_ok(IBox2::try_new([u_lo, v_lo], [u_hi, v_hi]))
}

/// A 3-axis parameter box.
fn box3(lo: [f64; 3], hi: [f64; 3]) -> truck_certified::kernel::patch::IBox<3> {
    construct_ok(truck_certified::kernel::patch::IBox::<3>::try_new(lo, hi))
}

/// One certified positive unit weight.
fn positive_one() -> CertifiedPositive {
    construct_ok(CertifiedPositive::try_new(1.0))
}

/// `n` certified positive unit weights.
fn weights(n: usize) -> Vec<CertifiedPositive> {
    (0..n).map(|_| positive_one()).collect()
}

/// A point interval.
fn iv(x: f64) -> Interval {
    Interval::point(x)
}

// ---------------------------------------------------------------------------
// Square residual fixtures
// ---------------------------------------------------------------------------

/// A linear 2x2 residual `F(x) = JÂ·x âˆ’ b`.
struct Linear2 {
    /// The 2x2 matrix.
    j: [[f64; 2]; 2],
    /// The shift vector.
    b: [f64; 2],
}

impl Linear2 {
    fn at(&self, x: &[Interval]) -> [Interval; 2] {
        let f0 = iv(self.j[0][0])
            .mul(&x[0])
            .add(&iv(self.j[0][1]).mul(&x[1]))
            .sub(&iv(self.b[0]));
        let f1 = iv(self.j[1][0])
            .mul(&x[0])
            .add(&iv(self.j[1][1]).mul(&x[1]))
            .sub(&iv(self.b[1]));
        [f0, f1]
    }
}

impl SquareResidualEval for Linear2 {
    fn arity(&self) -> usize {
        2
    }
    fn eval(&self, x: &[Interval]) -> Vec<Interval> {
        self.at(x).to_vec()
    }
    fn jac_encl(&self, _b: &[Interval]) -> Vec<Vec<Interval>> {
        vec![
            vec![iv(self.j[0][0]), iv(self.j[0][1])],
            vec![iv(self.j[1][0]), iv(self.j[1][1])],
        ]
    }
}

/// The quadratic residual `F = (x^2 âˆ’ c, y)`.
struct Quadratic {
    /// The offset `c` (`x^2 âˆ’ c = 0`).
    c: f64,
}

impl SquareResidualEval for Quadratic {
    fn arity(&self) -> usize {
        2
    }
    fn eval(&self, x: &[Interval]) -> Vec<Interval> {
        vec![x[0].mul(&x[0]).sub(&iv(self.c)), x[1]]
    }
    fn jac_encl(&self, x: &[Interval]) -> Vec<Vec<Interval>> {
        vec![vec![iv(2.0).mul(&x[0]), iv(0.0)], vec![iv(0.0), iv(1.0)]]
    }
}

/// The fixture leaf for the weight-value-argument test: the unit-weight plane
/// `z = 0` at bidegree `(1, 1)`.
fn unit_weight_plane_leaf() -> BezierLeaf {
    let control = vec![
        [0.0, 0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0, 1.0],
        [1.0, 0.0, 0.0, 1.0],
        [1.0, 1.0, 0.0, 1.0],
    ];
    construct_ok(BezierLeaf::try_new(1, 1, control))
}

// ---------------------------------------------------------------------------
// The engine test suite
// ---------------------------------------------------------------------------

/// A hand computation of Lemma 8.0's image and contraction rate over the same
/// op order the engine uses (M and r computed here, not read from the engine).
fn hand_c1(sys: &dyn SquareResidualEval, b: IBox2) -> Option<(bool, f64)> {
    let n = sys.arity();
    if n != 2 {
        return None;
    }
    let r = [(b.hi[0] - b.lo[0]) / 2.0, (b.hi[1] - b.lo[1]) / 2.0];
    if !r.iter().all(|c| c.is_finite() && *c > 0.0) {
        return None;
    }
    let z = [(b.lo[0] + b.hi[0]) / 2.0, (b.lo[1] + b.hi[1]) / 2.0];
    let ziv = [iv(z[0]), iv(z[1])];
    let box_iv = [
        Interval {
            lo: b.lo[0],
            hi: b.hi[0],
        },
        Interval {
            lo: b.lo[1],
            hi: b.hi[1],
        },
    ];
    let r0 = sys.eval(&ziv);
    if r0.len() != 2 {
        return None;
    }
    let j0_rows = sys.jac_encl(&ziv);
    let jb_rows = sys.jac_encl(&box_iv);
    if j0_rows.len() != 2 || j0_rows.iter().any(|row| row.len() != 2) {
        return None;
    }
    if jb_rows.len() != 2 || jb_rows.iter().any(|row| row.len() != 2) {
        return None;
    }

    // Inverse of the centre Jacobian: adjugate over determinant, row-major.
    let det = j0_rows[0][0]
        .mul(&j0_rows[1][1])
        .sub(&j0_rows[0][1].mul(&j0_rows[1][0]));
    if !det.is_finite() || (det.lo <= 0.0 && det.hi >= 0.0) {
        return None;
    }
    let a00 = j0_rows[1][1].div(&det)?;
    let a01 = j0_rows[0][1].neg().div(&det)?;
    let a10 = j0_rows[1][0].neg().div(&det)?;
    let a11 = j0_rows[0][0].div(&det)?;
    let a = [[a00, a01], [a10, a11]];

    // cj = AÂ·J(B), replicating the engine's matrix-product op order (an
    // accumulator seeded at point-zero, then one add per term).
    let mut cj = [[iv(0.0); 2]; 2];
    for r in 0..2 {
        for c in 0..2 {
            let mut acc = iv(0.0);
            for k in 0..2 {
                acc = acc.add(&a[r][k].mul(&jb_rows[k][c]));
            }
            cj[r][c] = acc;
        }
    }
    let id_minus = [
        [iv(1.0).sub(&cj[0][0]), cj[0][1].neg()],
        [cj[1][0].neg(), iv(1.0).sub(&cj[1][1])],
    ];

    // K = z_hat âˆ’ AÂ·R(z_hat) + (I âˆ’ AÂ·J(B))Â·(B âˆ’ z_hat).
    let mut dx = [iv(0.0); 2];
    for k in 0..2 {
        let d_lo = iv(b.lo[k]).sub(&ziv[k]);
        let d_hi = iv(b.hi[k]).sub(&ziv[k]);
        dx[k] = Interval {
            lo: d_lo.lo.min(d_hi.lo),
            hi: d_lo.hi.max(d_hi.hi),
        };
    }
    let ch = [
        a[0][0].mul(&r0[0]).add(&a[0][1].mul(&r0[1])),
        a[1][0].mul(&r0[0]).add(&a[1][1].mul(&r0[1])),
    ];
    let md = [
        id_minus[0][0].mul(&dx[0]).add(&id_minus[0][1].mul(&dx[1])),
        id_minus[1][0].mul(&dx[0]).add(&id_minus[1][1].mul(&dx[1])),
    ];
    let k = [
        ziv[0].sub(&ch[0]).add(&md[0]),
        ziv[1].sub(&ch[1]).add(&md[1]),
    ];

    let strict = (0..2).all(|i| b.lo[i] < k[i].lo && k[i].hi < b.hi[i]);
    if !strict {
        return Some((false, f64::NAN));
    }

    let mag = |v: &Interval| v.lo.abs().max(v.hi.abs());
    let mut rho = 0.0f64;
    for i in 0..2 {
        let mr = mag(&id_minus[i][0]) * r[0] + mag(&id_minus[i][1]) * r[1];
        let ratio = mr / r[i];
        if !ratio.is_finite() {
            return None;
        }
        rho = rho.max(ratio);
    }
    Some((true, rho))
}

#[test]
fn contraction_rho_matches_hand_computed_weighted_norm() {
    // F(x, y) = JÂ·(x,y) âˆ’ b with the root at exactly (1, 2).
    let sys = Linear2 {
        j: [[3.0, 1.0], [1.0, 2.0]],
        b: [5.0, 5.0],
    };
    let b = box2(0.9, 1.1, 1.9, 2.1);
    let w = weights(1);

    let (strict, hand_rho) = match hand_c1(&sys, b) {
        Some(hand) => hand,
        None => panic!("the hand computation must run on the linear fixture"),
    };
    assert!(
        strict,
        "the linear fixture must contract on the hand computation"
    );

    match krawczyk_c1(&sys, b, &w) {
        ClaimVerdict::Proven(cert) => {
            assert!(cert.rho <= config::RHO_MAX);
            // Lemma 8.0's rate must equal the hand computation bit for bit
            // (same op order over the same intervals).
            assert_eq!(cert.rho, hand_rho, "rho must equal the hand-computed rate");
            assert_eq!(cert.box_, b);
        }
        ClaimVerdict::Disproven(refusal) => {
            panic!("the linear fixture must certify Proven, refused: {refusal:?}")
        }
        ClaimVerdict::Inconclusive(reason) => {
            panic!("the linear fixture must certify Proven, inconclusive: {reason}")
        }
    }
}

#[test]
fn c1_proves_unique_root_on_a_known_quadratic() {
    // F = (x^2 âˆ’ 2, y) has the root (âˆš2, 0); box around the positive root.
    let sys = Quadratic { c: 2.0 };
    let b = box2(1.4, 1.5, -0.05, 0.05);
    let w = weights(1);
    match krawczyk_c1(&sys, b, &w) {
        ClaimVerdict::Proven(cert) => {
            assert!(cert.rho <= config::RHO_MAX, "rho must satisfy the ceiling");
            assert_eq!(cert.box_, b);
            // The box of the emitted certificate contains the known root.
            let root_x = std::f64::consts::SQRT_2;
            assert!(cert.box_.lo[0] <= root_x && root_x <= cert.box_.hi[0]);
            assert!(cert.box_.lo[1] <= 0.0 && 0.0 <= cert.box_.hi[1]);
        }
        ClaimVerdict::Disproven(refusal) => {
            panic!("a box around the known quadratic root must certify Proven: {refusal:?}")
        }
        ClaimVerdict::Inconclusive(reason) => {
            panic!("a box around the known quadratic root must certify Proven: {reason}")
        }
    }
}

#[test]
fn c1_refuses_when_krawczyk_image_exits_the_box() {
    // F = (4x âˆ’ 10, 4y âˆ’ 10): root (2.5, 2.5) far outside the box [0,1]^2.
    let sys = Linear2 {
        j: [[4.0, 0.0], [0.0, 4.0]],
        b: [10.0, 10.0],
    };
    let b = box2(0.0, 1.0, 0.0, 1.0);
    let w = weights(1);
    match krawczyk_c1(&sys, b, &w) {
        ClaimVerdict::Disproven(_) => {}
        other => {
            panic!("a disjoint Krawczyk image must be a Disproven-backed refusal, got {other:?}")
        }
    }
}

#[test]
fn c1_inconclusive_backing_when_inclusion_is_not_strict() {
    // F = (x âˆ’ 1, y âˆ’ 2): the root (1, 2) sits exactly at the box corner
    // [1,2] x [1,2]; K touches the boundary, so the inclusion is not strict.
    let sys = Linear2 {
        j: [[1.0, 0.0], [0.0, 1.0]],
        b: [1.0, 2.0],
    };
    let b = box2(1.0, 2.0, 1.0, 2.0);
    let w = weights(1);
    match krawczyk_c1(&sys, b, &w) {
        ClaimVerdict::Inconclusive(_) => {}
        ClaimVerdict::Proven(cert) => {
            panic!("a boundary-touching inclusion must not certify Proven: {cert:?}")
        }
        ClaimVerdict::Disproven(refusal) => {
            panic!("a boundary-touching (non-strict) inclusion is Inconclusive, got a Disproven refusal: {refusal:?}")
        }
    }
}

#[test]
fn weight_bound_is_a_value_argument_not_an_assumption() {
    let sys = Quadratic { c: 2.0 };
    let b = box2(1.4, 1.5, -0.05, 0.05);

    // Empty w refuses WeightDegenerate (Disproven).
    match krawczyk_c1(&sys, b, &[]) {
        ClaimVerdict::Disproven(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::WeightDegenerate);
            assert_eq!(
                refusal.backing,
                truck_certified::kernel::evidence::VerdictClass::Disproven
            );
        }
        other => panic!("an empty weight slice must refuse WeightDegenerate, got {other:?}"),
    }

    // A fixture leaf's weight_bound output feeds straight through as the value
    // argument: the same certificate runs Proven with the leaf's bound.
    let leaf = unit_weight_plane_leaf();
    let sub = box2(0.2, 0.4, 0.2, 0.4);
    let bound = match CertifiedPatch::weight_bound(&leaf, sub) {
        Some(ClaimVerdict::Proven(positive)) => positive,
        Some(other) => {
            panic!("the unit-weight plane leaf must yield a Proven weight bound: {other:?}")
        }
        None => panic!("BezierLeaf::weight_bound never returns None"),
    };
    let from_leaf = vec![bound];
    match krawczyk_c1(&sys, b, &from_leaf) {
        ClaimVerdict::Proven(cert) => {
            assert!(cert.rho <= config::RHO_MAX);
        }
        other => {
            panic!("the leaf weight bound must flow through as a value argument, got {other:?}")
        }
    }
}

// ---------------------------------------------------------------------------
// The straight-line (diagonal plane) SquareSystem3 fixture
// ---------------------------------------------------------------------------

/// The `u`/`s` coordinate grid at bidegree (1, 1): value `a`.
fn grid_a() -> Vec<Vec<f64>> {
    vec![vec![0.0, 0.0], vec![1.0, 1.0]]
}

/// The `v`/`t` coordinate grid at bidegree (1, 1): value `b`.
fn grid_b() -> Vec<Vec<f64>> {
    vec![vec![0.0, 1.0], vec![0.0, 1.0]]
}

/// A constant grid of bidegree (1, 1).
fn grid_const(c: f64) -> Vec<Vec<f64>> {
    vec![vec![c, c], vec![c, c]]
}

/// The straight-line fixture: `S1(u,v) = (u, v, u)` (the plane `z = x`)
/// against the horizontal plane `S2(s,t) = (s, t, 0.5)`. Their product-space
/// zero set is the straight vertical line
/// `{u = 0.5, s = 0.5, v = t}` through `(0.5, 0.5, 0.5, 0.5)`, tangent to the
/// `(v, t)` chart plane (the "diagonal lift" family of ssi_fixtures, reframed:
/// `s = u`, `t = v` on the zero set).
fn diagonal_plane_system() -> SquareSystem3 {
    let p1 = [grid_a(), grid_b(), grid_a()];
    let p2 = [grid_a(), grid_b(), grid_const(0.5)];
    let rows = 4;
    let cols = 4;
    let mut grids: [Vec<Vec<f64>>; 3] = [
        vec![vec![0.0; cols]; rows],
        vec![vec![0.0; cols]; rows],
        vec![vec![0.0; cols]; rows],
    ];
    for (k, grid_k) in grids.iter_mut().enumerate() {
        for a_i in 0..=1 {
            for b_i in 0..=1 {
                let row = a_i * 2 + b_i;
                for i in 0..=1 {
                    for j in 0..=1 {
                        grid_k[row][i * 2 + j] = p1[k][a_i][b_i] - p2[k][i][j];
                    }
                }
            }
        }
    }
    construct_ok(SquareSystem3::new(
        grids,
        (1, 1, 1, 1),
        (0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0),
    ))
}

/// The determinant of a 3x3 float matrix.
fn det3(m: [[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

/// The hand-computed Theorem 6.4 maximal-minor vector of the system Jacobian
/// at a chart point (the sign pattern as landed in ssi_trace).
fn hand_kernel_minors(system: &SquareSystem3, at: (f64, f64, f64, f64)) -> [f64; 4] {
    let degrees = system.degrees();
    let grids = system.grids();
    let mut rows = [[0.0f64; 4]; 3];
    for r in 0..3 {
        for c in 0..4 {
            rows[r][c] = match truck_certified::ssi_fixtures::partial_grid4_axis(
                &grids[r], degrees, c, at,
            ) {
                Some(v) => v,
                None => panic!("the straight-line fixture partials are well defined"),
            };
        }
    }
    let minor = |cols: [usize; 3]| -> f64 {
        let mut m = [[0.0f64; 3]; 3];
        for (r, row) in rows.iter().enumerate() {
            for (k, &c) in cols.iter().enumerate() {
                m[r][k] = row[c];
            }
        }
        det3(m)
    };
    let d0 = minor([1, 2, 3]);
    let d1 = -minor([0, 2, 3]);
    let d2 = minor([0, 1, 3]);
    let d3 = -minor([0, 1, 2]);
    [d0, d1, d2, d3]
}

fn assert_orthonormal(q: &[[f64; 4]; 4]) {
    let dot = |a: &[f64; 4], b: &[f64; 4]| a.iter().zip(b.iter()).map(|(x, y)| x * y).sum::<f64>();
    for c in 0..4 {
        let n = dot(&q[c], &q[c]).sqrt();
        assert!(
            (n - 1.0).abs() <= config::TOL_JACOBIAN,
            "frame column {c} is not unit: norm {n}"
        );
        for d in (c + 1)..4 {
            let d_ = dot(&q[c], &q[d]);
            assert!(
                d_.abs() <= config::TOL_JACOBIAN,
                "frame columns {c} and {d} are not orthogonal: dot {d_}"
            );
        }
    }
}

#[test]
fn frame_is_orthonormal_and_q_tau_is_the_normalized_kernel_direction() {
    let system = diagonal_plane_system();
    let z_hat = [0.5, 0.5, 0.5, 0.5];
    let (frame, m) = construct_ok(build_frame4(&system, z_hat));

    // The frame columns are orthonormal within the shim's gate.
    assert_orthonormal(&frame.q);
    assert!(
        frame.q_perp[3]
            .iter()
            .zip(frame.q_tau.iter())
            .all(|(a, b)| (a - b).abs() <= config::TOL_JACOBIAN),
        "q_perp must re-store q_tau as its final column"
    );

    // q_tau is the normalized hand-computed minor vector.
    let hand = hand_kernel_minors(&system, (0.5, 0.5, 0.5, 0.5));
    let hand_norm =
        (hand[0] * hand[0] + hand[1] * hand[1] + hand[2] * hand[2] + hand[3] * hand[3]).sqrt();
    assert!(hand_norm > 0.0, "the straight-line fixture has full rank");
    for i in 0..4 {
        let expected = hand[i] / hand_norm;
        assert!(
            (frame.q_tau[i] - expected).abs() <= GT,
            "q_tau[{i}] = {}, expected {expected}",
            frame.q_tau[i]
        );
        assert!(
            (m[i] - hand[i]).abs() <= GT,
            "returned kernel direction m[{i}] = {}, expected {}",
            m[i],
            hand[i]
        );
    }
}

#[test]
fn c2_tube_certifies_over_a_nontrivial_tau_interval() {
    let system = diagonal_plane_system();
    let z_hat = [0.5, 0.5, 0.5, 0.5];
    let (frame, _m) = construct_ok(build_frame4(&system, z_hat));

    // A ~0.1-wide tangent interval about the branch point and a perpendicular
    // box comfortably containing the straight branch's (zero) deviation.
    let i_tau = Interval {
        lo: -0.05,
        hi: 0.05,
    };
    let b_perp = box3([-0.3, -0.3, -0.3], [0.3, 0.3, 0.3]);
    let w = weights(2);

    match c2_certify_tube4(&system, &frame, i_tau, b_perp, &w) {
        ClaimVerdict::Proven(cert) => {
            assert_eq!(cert.i_tau.lo, i_tau.lo);
            assert_eq!(cert.i_tau.hi, i_tau.hi);
            assert!(
                (cert.i_tau.hi - cert.i_tau.lo - 0.1).abs() <= GT,
                "i_tau must carry the ~0.1 interval"
            );
            assert!(
                cert.rho <= config::RHO_MAX,
                "the tube's contraction rate must satisfy the ceiling: {}",
                cert.rho
            );
            assert_eq!(
                cert.residual,
                truck_certified::kernel::residual::ResidualId::R1
            );
            // The q_perp-aligned box convention: axes 0..=2 perpendicular,
            // axis 3 carries the tangent interval.
            assert_eq!(cert.b_perp.lo[3], i_tau.lo);
            assert_eq!(cert.b_perp.hi[3], i_tau.hi);
        }
        ClaimVerdict::Disproven(refusal) => {
            panic!("the straight branch must certify a tube, refused: {refusal:?}")
        }
        ClaimVerdict::Inconclusive(reason) => {
            panic!("the straight branch must certify a tube, inconclusive: {reason}")
        }
    }
}

#[test]
fn c2_tube_refuses_when_perpendicular_contraction_fails() {
    let system = diagonal_plane_system();
    let z_hat = [0.5, 0.5, 0.5, 0.5];
    let (frame, _m) = construct_ok(build_frame4(&system, z_hat));
    let w = weights(2);

    // A far-too-wide tangent interval sweeps the joint box out of the chart
    // rectangle: the perpendicular image cannot be enclosed, so the tube is
    // Inconclusive (Conditioning) â€” never a wrong Proven.
    let too_wide = Interval { lo: -1.5, hi: 1.5 };
    let b_perp = box3([-0.2, -0.2, -0.2], [0.2, 0.2, 0.2]);
    match c2_certify_tube4(&system, &frame, too_wide, b_perp, &w) {
        ClaimVerdict::Proven(cert) => {
            panic!("a too-wide tau interval must never certify a tube: {cert:?}")
        }
        ClaimVerdict::Inconclusive(_) => {}
        ClaimVerdict::Disproven(refusal) => {
            panic!("a tube enclosure failure is Inconclusive, not Disproven: {refusal:?}")
        }
    }

    // A deliberately tilted frame (identity frame in chart coordinates) with a
    // too-small perpendicular box: the perpendicular image cannot be strict
    // inside B_perp, so the outcome is Inconclusive.
    let tilted_q = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let tilted_q_tau = [1.0, 0.0, 0.0, 0.0];
    let tilted_q_perp = [
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
        [1.0, 0.0, 0.0, 0.0],
    ];
    let tilted_a = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let tilted = construct_ok(Frame::try_new(
        z_hat,
        tilted_q,
        tilted_q_tau,
        tilted_q_perp,
        tilted_a,
    ));
    let narrow = box3([-0.01, -0.01, -0.01], [0.01, 0.01, 0.01]);
    let i_tau = Interval {
        lo: -0.05,
        hi: 0.05,
    };
    match c2_certify_tube4(&system, &tilted, i_tau, narrow, &w) {
        ClaimVerdict::Proven(cert) => {
            panic!("a tilted frame with a too-small perpendicular box must not certify: {cert:?}")
        }
        ClaimVerdict::Inconclusive(_) => {}
        ClaimVerdict::Disproven(refusal) => {
            panic!("a tube contraction failure is Inconclusive, not Disproven: {refusal:?}")
        }
    }
}

#[test]
fn no_transcendental_call_in_engine_module() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/kernel/engine.rs");
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(err) => panic!("engine.rs must be readable: {err}"),
    };
    let code: Vec<&str> = source
        .lines()
        .map(|line| {
            // Strip // comments.
            match line.find("//") {
                Some(i) => &line[..i],
                None => line,
            }
        })
        .collect();
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
            .iter()
            .any(|line| contains_word(line, needle) || line.contains("std::f64::consts"));
        assert!(
            !present,
            "no transcendental call may appear outside comments in engine.rs (found {needle})"
        );
    }
    // sqrt appears only in frame normalization contexts.
    let sqrt_lines: Vec<&str> = code
        .iter()
        .copied()
        .filter(|line| line.contains("sqrt"))
        .collect();
    assert!(
        sqrt_lines
            .iter()
            .all(|l| l.contains("norm") || l.contains("norm_sq")),
        "sqrt must appear only in frame normalization: {sqrt_lines:?}"
    );
    assert!(!sqrt_lines.is_empty(), "frame normalization uses sqrt");
}

// ---------------------------------------------------------------------------
// The arity-3 C1 entry tests (BG-KV2-206-N3CERT)
// ---------------------------------------------------------------------------

use truck_certified::kernel::certs::PointCert3;
use truck_certified::kernel::engine::krawczyk_c1_n3;
use truck_certified::kernel::patch::IBox3;

/// A linear 3x3 residual `F(x) = J·x − b`.
struct Linear3 {
    /// The 3x3 matrix.
    j: [[f64; 3]; 3],
    /// The shift vector.
    b: [f64; 3],
}

impl Linear3 {
    fn at(&self, x: &[Interval]) -> [Interval; 3] {
        let f0 = iv(self.j[0][0])
            .mul(&x[0])
            .add(&iv(self.j[0][1]).mul(&x[1]))
            .add(&iv(self.j[0][2]).mul(&x[2]))
            .sub(&iv(self.b[0]));
        let f1 = iv(self.j[1][0])
            .mul(&x[0])
            .add(&iv(self.j[1][1]).mul(&x[1]))
            .add(&iv(self.j[1][2]).mul(&x[2]))
            .sub(&iv(self.b[1]));
        let f2 = iv(self.j[2][0])
            .mul(&x[0])
            .add(&iv(self.j[2][1]).mul(&x[1]))
            .add(&iv(self.j[2][2]).mul(&x[2]))
            .sub(&iv(self.b[2]));
        [f0, f1, f2]
    }
}

impl SquareResidualEval for Linear3 {
    fn arity(&self) -> usize {
        3
    }
    fn eval(&self, x: &[Interval]) -> Vec<Interval> {
        self.at(x).to_vec()
    }
    fn jac_encl(&self, _b: &[Interval]) -> Vec<Vec<Interval>> {
        vec![
            vec![iv(self.j[0][0]), iv(self.j[0][1]), iv(self.j[0][2])],
            vec![iv(self.j[1][0]), iv(self.j[1][1]), iv(self.j[1][2])],
            vec![iv(self.j[2][0]), iv(self.j[2][1]), iv(self.j[2][2])],
        ]
    }
}

/// The R8-shaped line-pierce-plane residual as a raw [`SquareResidualEval`]
/// (the fixture does not need `R8System`; that is S1A's). The line
/// `C(t) = (t, t, t − 1)` pierces the plane `S(u, v) = (u, v, 0)` exactly
/// where `(t, u, v) = (1, 1, 1)`:
/// `H(t, u, v) = C(t) − S(u, v) = (t − u, t − v, t − 1)`.
struct R8LinePiercePlane;

impl SquareResidualEval for R8LinePiercePlane {
    fn arity(&self) -> usize {
        3
    }
    fn eval(&self, x: &[Interval]) -> Vec<Interval> {
        vec![x[0].sub(&x[1]), x[0].sub(&x[2]), x[0].sub(&iv(1.0))]
    }
    fn jac_encl(&self, _b: &[Interval]) -> Vec<Vec<Interval>> {
        // DH = [ C'(t)  −S_u  −S_v ] = [[1, −1, 0], [1, 0, −1], [1, 0, 0]]
        vec![
            vec![iv(1.0), iv(-1.0), iv(0.0)],
            vec![iv(1.0), iv(0.0), iv(-1.0)],
            vec![iv(1.0), iv(0.0), iv(0.0)],
        ]
    }
}

#[test]
fn point_cert3_try_new_enforces_rho_and_finite_box() {
    let box_ = box3([0.9, 0.9, 0.9], [1.1, 1.1, 1.1]);
    let ok = construct_ok(PointCert3::try_new(
        truck_certified::kernel::residual::ResidualId::R1,
        box_,
        0.3,
    ));
    assert_eq!(ok.rho, 0.3);
    assert_eq!(ok.box_, box_);
    assert_eq!(
        ok.residual,
        truck_certified::kernel::residual::ResidualId::R1
    );

    // rho above the Lemma 8.0 ceiling refuses Conditioning (Inconclusive),
    // exactly as PointCert::try_new does.
    match PointCert3::try_new(
        truck_certified::kernel::residual::ResidualId::R1,
        box_,
        config::RHO_MAX + 0.01,
    ) {
        Err(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::Conditioning);
            assert_eq!(
                refusal.backing,
                truck_certified::kernel::evidence::VerdictClass::Inconclusive
            );
        }
        Ok(_) => panic!("rho above RHO_MAX must refuse the arity-3 point certificate"),
    }
    assert!(
        PointCert3::try_new(truck_certified::kernel::residual::ResidualId::R1, box_, 0.9).is_err(),
        "a clearly-over-ceiling rho must refuse"
    );

    // A non-finite rho refuses NonFinite (Disproven).
    match PointCert3::try_new(
        truck_certified::kernel::residual::ResidualId::R1,
        box_,
        f64::NAN,
    ) {
        Err(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::NonFinite);
            assert_eq!(
                refusal.backing,
                truck_certified::kernel::evidence::VerdictClass::Disproven
            );
        }
        Ok(_) => panic!("a non-finite rho must refuse the arity-3 point certificate"),
    }

    // A non-finite box refuses even at an acceptable rho.
    let bad_box = IBox3 {
        lo: [0.9, 0.9, f64::NAN],
        hi: [1.1, 1.1, 1.1],
    };
    assert!(
        PointCert3::try_new(
            truck_certified::kernel::residual::ResidualId::R1,
            bad_box,
            0.1
        )
        .is_err(),
        "a non-finite box must refuse the arity-3 point certificate"
    );
}

#[test]
fn krawczyk_c1_n3_proves_a_known_3var_root() {
    let sys = R8LinePiercePlane;
    // The unique root (t, u, v) = (1, 1, 1) is interior to the box.
    let root = [1.0, 1.0, 1.0];
    let b = box3([0.9, 0.9, 0.9], [1.1, 1.1, 1.1]);
    let w = weights(1);

    // Ground the fixture: the raw residual vanishes at the claimed root.
    let at_root = <R8LinePiercePlane as SquareResidualEval>::eval(
        &sys,
        &[iv(root[0]), iv(root[1]), iv(root[2])],
    );
    for (k, component) in at_root.iter().enumerate() {
        assert!(
            component.contains(0.0),
            "the residual must vanish at the known root: component {k} = {component:?}"
        );
    }

    match krawczyk_c1_n3(&sys, b, &w) {
        ClaimVerdict::Proven(cert) => {
            assert!(cert.rho <= config::RHO_MAX, "rho must satisfy the ceiling");
            assert_eq!(cert.box_, b);
            assert_eq!(
                cert.residual,
                truck_certified::kernel::residual::ResidualId::R1
            );
            // The box of the emitted certificate contains the known root.
            for axis in 0..3 {
                assert!(
                    cert.box_.lo[axis] <= root[axis] && root[axis] <= cert.box_.hi[axis],
                    "certified box must contain the root on axis {axis}"
                );
            }
        }
        ClaimVerdict::Disproven(refusal) => {
            panic!("the line-pierce-plane root must certify Proven, refused: {refusal:?}")
        }
        ClaimVerdict::Inconclusive(reason) => {
            panic!("the line-pierce-plane root must certify Proven, inconclusive: {reason}")
        }
    }
}

#[test]
fn krawczyk_c1_n3_backing_matches_the_2d_table() {
    let w = weights(1);

    // A disjoint Krawczyk image is Disproven-backed (mirror of the 2D
    // c1_refuses_when_krawczyk_image_exits_the_box test): the root (2.5,
    // 2.5, 2.5) of `4x − 10` sits far outside the box [0, 1]^3.
    let sys = Linear3 {
        j: [[4.0, 0.0, 0.0], [0.0, 4.0, 0.0], [0.0, 0.0, 4.0]],
        b: [10.0, 10.0, 10.0],
    };
    let b = box3([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
    match krawczyk_c1_n3(&sys, b, &w) {
        ClaimVerdict::Disproven(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::ClaimRefuted);
            assert_eq!(
                refusal.backing,
                truck_certified::kernel::evidence::VerdictClass::Disproven
            );
        }
        other => panic!("a disjoint 3D Krawczyk image must be Disproven, got {other:?}"),
    }

    // A non-strict (boundary-touching) inclusion is Inconclusive (mirror of
    // the 2D c1_inconclusive_backing_when_inclusion_is_not_strict test): the
    // root (1, 1, 1) of `x − 1` sits exactly at the box corner [1, 2]^3.
    let sys = Linear3 {
        j: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        b: [1.0, 1.0, 1.0],
    };
    let b = box3([1.0, 1.0, 1.0], [2.0, 2.0, 2.0]);
    match krawczyk_c1_n3(&sys, b, &w) {
        ClaimVerdict::Inconclusive(_) => {}
        ClaimVerdict::Proven(cert) => {
            panic!("a boundary-touching 3D inclusion must not certify Proven: {cert:?}")
        }
        ClaimVerdict::Disproven(refusal) => {
            panic!("a boundary-touching (non-strict) 3D inclusion is Inconclusive, got a Disproven refusal: {refusal:?}")
        }
    }
}
