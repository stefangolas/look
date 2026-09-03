//! BG-KV2-304-S3B integration tests: the additive arity-4 C1 carrier
//! (`PointCert4` + `krawczyk_c1_n4`, the N3CERT pattern's second application)
//! and the Tier-2 critical-point start set (§9.2 / Corollary 9.3: the R3
//! minor form `Psi_a = (F, a·m)`, the `a·m`-exclusion subdivision, and the
//! a-posteriori `k_a` direction-perturbation rule) — over rational, exact
//! ground truths.

#![deny(clippy::unwrap_used)]

use std::fmt::Debug;

use truck_certified::kernel::certs::{IBox4, PointCert3, PointCert4};
use truck_certified::kernel::config;
use truck_certified::kernel::engine::{krawczyk_c1_n3, krawczyk_c1_n4, SquareResidualEval};
use truck_certified::kernel::evidence::{ClaimVerdict, RefusalKind, VerdictClass};
use truck_certified::kernel::graph::ChartId;
use truck_certified::kernel::leaf::BezierLeaf;
use truck_certified::kernel::patch::CertifiedPositive;
use truck_certified::kernel::residual::ResidualId;
use truck_certified::kernel::residuals_r89::BezierLeaf1;
use truck_certified::kernel::tier1;
use truck_certified::kernel::tier2;
use truck_certified::kernel::Interval;
use truck_certified::SquareSystem3;

/// The root-containment slack when comparing a certified box against a known
/// float root (the fixture systems are built from rounded rational
/// coefficients, so the exact roots of the stored system are within rounding
/// of the ideal dyadic root).
const ROOT_TOL: f64 = 1e-6; // H-3: fixture root-containment slack

/// Extract the `Ok` of any fallible construction; the fixture data is valid by
/// construction, so the refusal arm is a test-bug panic (never an unwrap).
fn construct_ok<T, E: Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(err) => panic!("a construction that must succeed was refused: {err:?}"),
    }
}

/// A 4-axis parameter box.
fn box4(lo: [f64; 4], hi: [f64; 4]) -> IBox4 {
    construct_ok(IBox4::try_new(lo, hi))
}

/// A point interval.
fn iv(x: f64) -> Interval {
    Interval::point(x)
}

/// `n` certified positive unit weights.
fn weights(n: usize) -> Vec<CertifiedPositive> {
    (0..n)
        .map(|_| construct_ok(CertifiedPositive::try_new(1.0)))
        .collect()
}

/// Whether a box contains the 4-vector `p` up to the root slack.
fn contains_root(b: &IBox4, p: [f64; 4]) -> bool {
    for axis in 0..4 {
        if b.lo[axis] - ROOT_TOL > p[axis] || p[axis] > b.hi[axis] + ROOT_TOL {
            return false;
        }
    }
    true
}

// ---------------------------------------------------------------------------
// Raw arity-4 / arity-3 residual fixtures for the C1 entry gate tests
// ---------------------------------------------------------------------------

/// A linear 4x4 residual `F(x) = J·x − b`.
struct Linear4 {
    j: [[f64; 4]; 4],
    b: [f64; 4],
}

impl Linear4 {
    fn at(&self, x: &[Interval]) -> [Interval; 4] {
        let mut out = [iv(0.0); 4];
        for r in 0..4 {
            let mut acc = iv(0.0);
            for c in 0..4 {
                acc = acc.add(&iv(self.j[r][c]).mul(&x[c]));
            }
            out[r] = acc.sub(&iv(self.b[r]));
        }
        out
    }
}

impl SquareResidualEval for Linear4 {
    fn arity(&self) -> usize {
        4
    }
    fn eval(&self, x: &[Interval]) -> Vec<Interval> {
        self.at(x).to_vec()
    }
    fn jac_encl(&self, _b: &[Interval]) -> Vec<Vec<Interval>> {
        let mut rows = Vec::with_capacity(4);
        for r in 0..4 {
            rows.push((0..4).map(|c| iv(self.j[r][c])).collect());
        }
        rows
    }
}

/// A residual that reports arity 3 while evaluating four components: the
/// entry must refuse the arity mismatch before any evaluation.
struct WrongArity4;

impl SquareResidualEval for WrongArity4 {
    fn arity(&self) -> usize {
        3
    }
    fn eval(&self, x: &[Interval]) -> Vec<Interval> {
        vec![iv(0.0); x.len()]
    }
    fn jac_encl(&self, b: &[Interval]) -> Vec<Vec<Interval>> {
        vec![vec![iv(0.0); b.len()]; b.len()]
    }
}

/// A linear 3x3 residual `F(x) = J·x − b` (the arity-3 additivity witness).
struct Linear3 {
    j: [[f64; 3]; 3],
    b: [f64; 3],
}

impl SquareResidualEval for Linear3 {
    fn arity(&self) -> usize {
        3
    }
    fn eval(&self, x: &[Interval]) -> Vec<Interval> {
        let mut out = Vec::with_capacity(3);
        for r in 0..3 {
            let mut acc = iv(0.0);
            for c in 0..3 {
                acc = acc.add(&iv(self.j[r][c]).mul(&x[c]));
            }
            out.push(acc.sub(&iv(self.b[r])));
        }
        out
    }
    fn jac_encl(&self, _b: &[Interval]) -> Vec<Vec<Interval>> {
        (0..3)
            .map(|r| (0..3).map(|c| iv(self.j[r][c])).collect())
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Fixture geometry: graphs over the unit square against the plane z = 0
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

/// The graph leaf `(u, v) -> (u, v, h(u, v))` at bidegree `(m, n)` over the
/// tensor-Bernstein height grid `h[a][b]` (a over `u`, b over `v`).
fn graph_leaf(m: usize, n: usize, h: &[Vec<f64>]) -> BezierLeaf {
    let mut control = Vec::with_capacity((m + 1) * (n + 1));
    for a in 0..=m {
        for b in 0..=n {
            control.push([a as f64 / m as f64, b as f64 / n as f64, h[a][b], 1.0]);
        }
    }
    construct_ok(BezierLeaf::try_new(m, n, control))
}

/// A graph leaf `(u, v) -> (u, y(v), h(u, v))` at bidegree `(m, n)` whose
/// `y`-coordinate is the affine map of the `v`-index grid (`y[b]`), so its
/// boundary edges do NOT lie in the model planes `y = 0` or `y = 1`.
fn graph_xy_leaf(m: usize, n: usize, y: &[f64], h: &[Vec<f64>]) -> BezierLeaf {
    let mut control = Vec::with_capacity((m + 1) * (n + 1));
    for a in 0..=m {
        for b in 0..=n {
            control.push([a as f64 / m as f64, y[b], h[a][b], 1.0]);
        }
    }
    construct_ok(BezierLeaf::try_new(m, n, control))
}

/// The bidegree-`(2, 2)` height grid of the centred circle
/// `f(u, v) = (u − 1/3)² + (v − 1/3)² − r²` with `r = 1/4`. The circle lies
/// strictly inside the unit square (`u, v ∈ [1/12, 7/12]`), and its
/// `a = (1,0,0,0)` critical points (`v = 1/3`, `u = 1/3 ± 1/4`) have
/// NON-dyadic coordinates, so the dyadic subdivision can isolate them.
fn loop_heights() -> Vec<Vec<f64>> {
    let r2 = 1.0 / 16.0;
    let hu = [1.0 / 9.0, -2.0 / 9.0, 4.0 / 9.0];
    let mut out = vec![vec![0.0f64; 3]; 3];
    for a in 0..3 {
        for b in 0..3 {
            out[a][b] = hu[a] + hu[b] - r2;
        }
    }
    out
}

/// The bidegree-`(2, 1)` height grid of `f(u, v) = 16·(u − 1/3)(u − 2/3)·(1 +
/// v/10)`: two straight zero lines (`u = 1/3` and `u = 2/3`) crossing the full
/// `v`-range, with a deep negative band between them (so the subdivision
/// exclusion clears the band promptly) and strictly positive `f` away from
/// them.
fn two_line_heights() -> Vec<Vec<f64>> {
    let zu = [16.0 * 2.0 / 9.0, -16.0 * 5.0 / 18.0, 16.0 * 2.0 / 9.0];
    let w = [1.0, 11.0 / 10.0];
    let mut out = vec![vec![0.0f64; 2]; 3];
    for a in 0..3 {
        for b in 0..2 {
            out[a][b] = zu[a] * w[b];
        }
    }
    out
}

/// The stored square system `F(u,v,s,t) = P1(u,v) − P2(s,t)` of two leaves
/// over the shared unit chart (unit weights, the `construct_square_system`
/// layout): rows over P1's `(u,v)` grid, columns over P2's `(s,t)` grid.
fn system_from_leaves(p1: &BezierLeaf, p2: &BezierLeaf) -> SquareSystem3 {
    let (m1, n1) = (p1.degree_u, p1.degree_v);
    let (m2, n2) = (p2.degree_u, p2.degree_v);
    let rows = (m1 + 1) * (n1 + 1);
    let cols = (m2 + 1) * (n2 + 1);
    let mut grids: [Vec<Vec<f64>>; 3] = [
        vec![vec![0.0; cols]; rows],
        vec![vec![0.0; cols]; rows],
        vec![vec![0.0; cols]; rows],
    ];
    for a in 0..=m1 {
        for b in 0..=n1 {
            let row = a * (n1 + 1) + b;
            for i in 0..=m2 {
                for j in 0..=n2 {
                    let col = i * (n2 + 1) + j;
                    for k in 0..3 {
                        grids[k][row][col] = p1.control[row][k] - p2.control[col][k];
                    }
                }
            }
        }
    }
    construct_ok(SquareSystem3::new(
        grids,
        (m1, n1, m2, n2),
        (0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0),
    ))
}

/// The transversal loop fixture: the graph `z = (u − 1/3)² + (v − 1/3)² −
/// 1/16` (a centred circle of radius 1/4, strictly inside the unit square)
/// against the plane `z = 0`. Its product-space zero set is the closed loop
/// `{(u, v, u, v) : f = 0}`, which meets no boundary of `[0,1]⁴`. For
/// `a = (1,0,0,0)` the `a·m` component is `f_v = 2(v − 1/3)`, so the two
/// zeros of `Psi_a` are at the non-dyadic points
/// `(1/12, 1/3, 1/12, 1/3)` and `(7/12, 1/3, 7/12, 1/3)`.
fn loop_system() -> SquareSystem3 {
    let loop_leaf = graph_leaf(2, 2, &loop_heights());
    system_from_leaves(&loop_leaf, &plane_leaf())
}

/// The composition fixture (Corollary 9.3): the leaf
/// `P1(u, v) = (u, 2/5 + v/5, f(u, v))` (a bidegree-`(2,1)` graph whose
/// `y`-coordinate is offset, so its boundary edges avoid the model planes
/// `y = 0` and `y = 1`) against the plane `z = 0`, with
/// `f(u, v) = 16·(u − 1/3)(u − 2/3)·(1 + v/10)`. Its zero set `Z` has exactly
/// two connected components in the leaf product: the two straight arcs
/// `{u = 1/3, s = 1/3, t = 2/5 + v/5}` and `{u = 2/3, s = 2/3, t = 2/5 + v/5}`
/// (`v` free), each crossing the leaf's `v`-boundary twice at the NON-dyadic
/// R8 roots `(1/3, 1/3, 2/5)`, `(1/3, 1/3, 3/5)`, `(2/3, 2/3, 2/5)`,
/// `(2/3, 2/3, 3/5)`.
///
/// Every component meets `∂B`, so Theorem 9.2's boundary case applies: the
/// §9.3 boundary seeds cover both components, and `Psi_a` (with
/// `a = (1,1,0,0)`, for which `a·m = f_v − f_u` is nonzero on both arcs)
/// isolates no interior critical point — the combined start set covers every
/// oracle component.
fn composition_system() -> (BezierLeaf, SquareSystem3) {
    let y: Vec<f64> = vec![2.0 / 5.0, 3.0 / 5.0];
    let p = graph_xy_leaf(2, 1, &y, &two_line_heights());
    let system = system_from_leaves(&p, &plane_leaf());
    (p, system)
}

/// The straight-line fixture: the plane `z = x` (`P1(u,v) = (u, v, u)`)
/// against the horizontal plane `z = 1/2`. Their zero set is the straight
/// line `{(1/2, v, 1/2, v)}` across the domain, with the constant maximal
/// minor `m = (0, −1, 0, −1)`. For `a = (0,1,0,0)` the `a·m` component is the
/// constant `−1`, so exclusion clears the entire domain (the line carries no
/// critical point) and the start set is complete and empty.
fn straight_line_system() -> SquareSystem3 {
    let p1 = construct_ok(BezierLeaf::try_new(
        1,
        1,
        vec![
            [0.0, 0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0, 1.0],
            [1.0, 0.0, 1.0, 1.0],
            [1.0, 1.0, 1.0, 1.0],
        ],
    ));
    let p2 = construct_ok(BezierLeaf::try_new(
        1,
        1,
        vec![
            [0.0, 0.0, 0.5, 1.0],
            [0.0, 1.0, 0.5, 1.0],
            [1.0, 0.0, 0.5, 1.0],
            [1.0, 1.0, 0.5, 1.0],
        ],
    ));
    system_from_leaves(&p1, &p2)
}

/// The tangential fixture: the graph `z = u²` (the parabolic cylinder over
/// `(u,v)`, bidegree `(2,1)`) against the plane `z = 0`. Their intersection
/// is the straight line `{(0, v, 0, v)}` (the zero set of `F`), at which the
/// two surfaces are tangent: the maximal-minor vector `m = (0, −2u, 0, −2u)`
/// vanishes on the whole line, so `Psi_a`'s zero set contains the entire line
/// for EVERY direction `a` — a persistent positive-dimensional (curve) zero
/// set.
fn tangential_system() -> SquareSystem3 {
    let heights = vec![vec![0.0, 0.0], vec![0.0, 0.0], vec![1.0, 1.0]];
    let cyl = graph_leaf(2, 1, &heights);
    system_from_leaves(&cyl, &plane_leaf())
}

/// The crossing fixture: the graph `z = u·v` (the hyperbolic paraboloid over
/// `(u,v)`, bidegree `(1,1)`) against the plane `z = 0`. Its zero set is the
/// two straight branches `{(0, v, 0, v)}` and `{(u, 0, u, 0)}` crossing at the
/// origin — a singular point of `Z` (rank `DF` drops to 2 there, so `m = 0`
/// and `Sing(Z) ⊆ Psi_a⁻¹(0)` for every `a`). The crossing is an isolated
/// zero of `Psi_a` for every direction whose `a·m` is not identically zero,
/// but the local Jacobian is singular at it, so no Krawczyk box can isolate
/// it: the subdivision stalls on a bounded, isolated leaf set. This is the
/// §9.2 `IncompleteStartSet` signature (NOT a tangential curve).
fn crossing_system() -> SquareSystem3 {
    let heights = vec![vec![0.0, 0.0], vec![0.0, 1.0]];
    let saddle = graph_leaf(1, 1, &heights);
    system_from_leaves(&saddle, &plane_leaf())
}

/// Assert a certificate is R3-stamped with a finite box and an acceptable rho.
fn assert_point_cert4(cert: &PointCert4) {
    assert_eq!(cert.residual, ResidualId::R3, "start-set certs are R3");
    assert!(
        cert.rho <= config::RHO_MAX,
        "every certified start-set point must satisfy rho <= RHO_MAX: {:?}",
        cert
    );
    assert!(
        cert.box_
            .lo
            .iter()
            .chain(cert.box_.hi.iter())
            .all(|c| c.is_finite()),
        "every certified start-set box must be finite"
    );
}

// ---------------------------------------------------------------------------
// Test 1: the additive arity-4 carrier and its constructor gates
// ---------------------------------------------------------------------------

#[test]
fn point_cert4_and_n4_entry_additive_and_gated() {
    let box_ = box4([0.9, 0.9, 0.9, 0.9], [1.1, 1.1, 1.1, 1.1]);

    // The constructor gate, exactly PointCert3's: an acceptable rho carries,
    // an over-ceiling rho refuses Conditioning (Inconclusive), a non-finite
    // rho refuses NonFinite (Disproven), a non-finite box refuses.
    let ok = construct_ok(PointCert4::try_new(ResidualId::R3, box_, 0.3));
    assert_eq!(ok.rho, 0.3);
    assert_eq!(ok.box_, box_);
    assert_eq!(ok.residual, ResidualId::R3);
    match PointCert4::try_new(ResidualId::R3, box_, config::RHO_MAX + 0.01) {
        Err(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::Conditioning);
            assert_eq!(refusal.backing, VerdictClass::Inconclusive);
        }
        Ok(_) => panic!("rho above RHO_MAX must refuse the arity-4 point certificate"),
    }
    match PointCert4::try_new(ResidualId::R3, box_, f64::NAN) {
        Err(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::NonFinite);
            assert_eq!(refusal.backing, VerdictClass::Disproven);
        }
        Ok(_) => panic!("a non-finite rho must refuse the arity-4 point certificate"),
    }
    let bad_box = IBox4 {
        lo: [0.9, 0.9, 0.9, f64::NAN],
        hi: [1.1, 1.1, 1.1, 1.1],
    };
    assert!(
        PointCert4::try_new(ResidualId::R3, bad_box, 0.1).is_err(),
        "a non-finite box must refuse the arity-4 point certificate"
    );

    // The arity-4 C1 entry proves a known 4-var root: `x − 1 = 0` at
    // `(1,1,1,1)` on the identity 4x4 system, box interior to the root.
    let sys = Linear4 {
        j: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
        b: [1.0, 1.0, 1.0, 1.0],
    };
    let w = weights(1);
    match krawczyk_c1_n4(&sys, box_, &w) {
        ClaimVerdict::Proven(cert) => {
            assert!(cert.rho <= config::RHO_MAX, "rho must satisfy the ceiling");
            assert_eq!(cert.box_, box_);
            assert_eq!(cert.residual, ResidualId::R1, "the engine stamps R1");
            for axis in 0..4 {
                assert!(
                    cert.box_.lo[axis] <= 1.0 && 1.0 <= cert.box_.hi[axis],
                    "certified box must contain the root on axis {axis}"
                );
            }
        }
        other => panic!("the identity root must certify Proven, got {other:?}"),
    }

    // A non-diagonal 4x4 linear system with a known interior root also
    // certifies (the adjugate/det path at n = 4).
    let block = Linear4 {
        j: [
            [3.0, 1.0, 0.0, 0.0],
            [1.0, 2.0, 0.0, 0.0],
            [0.0, 0.0, 4.0, 1.0],
            [0.0, 0.0, 1.0, 3.0],
        ],
        b: [4.0, 3.0, 5.0, 4.0],
    };
    match krawczyk_c1_n4(&block, box_, &w) {
        ClaimVerdict::Proven(cert) => {
            assert!(cert.rho <= config::RHO_MAX, "rho must satisfy the ceiling");
        }
        other => panic!("the non-diagonal root must certify Proven, got {other:?}"),
    }

    // Backing-table parity with the arity-3 entry (mirrors the 2D/3D tests):
    // an empty weight slice refuses WeightDegenerate (Disproven), an arity
    // mismatch is Inconclusive, a disjoint image is Disproven
    // (ClaimRefuted), and a boundary-touching inclusion is Inconclusive.
    match krawczyk_c1_n4(&sys, box_, &[]) {
        ClaimVerdict::Disproven(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::WeightDegenerate);
            assert_eq!(refusal.backing, VerdictClass::Disproven);
        }
        other => panic!("an empty weight slice must refuse WeightDegenerate, got {other:?}"),
    }
    match krawczyk_c1_n4(&WrongArity4, box_, &w) {
        ClaimVerdict::Inconclusive(_) => {}
        other => panic!("an arity mismatch must be Inconclusive, got {other:?}"),
    }
    let disjoint_sys = Linear4 {
        j: [
            [4.0, 0.0, 0.0, 0.0],
            [0.0, 4.0, 0.0, 0.0],
            [0.0, 0.0, 4.0, 0.0],
            [0.0, 0.0, 0.0, 4.0],
        ],
        b: [10.0, 10.0, 10.0, 10.0],
    };
    match krawczyk_c1_n4(
        &disjoint_sys,
        box4([0.0, 0.0, 0.0, 0.0], [1.0, 1.0, 1.0, 1.0]),
        &w,
    ) {
        ClaimVerdict::Disproven(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::ClaimRefuted);
            assert_eq!(refusal.backing, VerdictClass::Disproven);
        }
        other => panic!("a disjoint 4D Krawczyk image must be Disproven, got {other:?}"),
    }
    match krawczyk_c1_n4(&sys, box4([1.0, 1.0, 1.0, 1.0], [2.0, 2.0, 2.0, 2.0]), &w) {
        ClaimVerdict::Inconclusive(_) => {}
        other => {
            panic!("a boundary-touching 4D inclusion must not certify, got {other:?}")
        }
    }

    // Additive: the arity-3 carrier and entry still work untouched.
    let sys3 = Linear3 {
        j: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        b: [1.0, 1.0, 1.0],
    };
    let box3_ = construct_ok(truck_certified::kernel::patch::IBox3::try_new(
        [0.9, 0.9, 0.9],
        [1.1, 1.1, 1.1],
    ));
    match krawczyk_c1_n3(&sys3, box3_, &w) {
        ClaimVerdict::Proven(cert) => {
            assert!(cert.rho <= config::RHO_MAX);
        }
        other => panic!("the arity-3 entry must still certify Proven, got {other:?}"),
    }
    let cert3 = construct_ok(PointCert3::try_new(ResidualId::R8, box3_, config::RHO_MAX));
    assert_eq!(cert3.residual, ResidualId::R8);
}

// ---------------------------------------------------------------------------
// Test 2: Psi_a zeros isolate on a transversal fixture
// ---------------------------------------------------------------------------

#[test]
fn psi_a_zeros_isolated_on_a_transversal_fixture() {
    let system = loop_system();
    let a = [1.0, 0.0, 0.0, 0.0];
    let domain = box4([0.0, 0.0, 0.0, 0.0], [1.0, 1.0, 1.0, 1.0]);

    let outcome = tier2::tier2_start_set(&system, a, domain);
    let start_set = match &outcome {
        tier2::TierTwoOutcome::Complete { start_set } => start_set.clone(),
        tier2::TierTwoOutcome::Refused(refusal) => {
            panic!("the transversal loop must yield a complete start set, refused: {refusal:?}")
        }
    };
    assert_eq!(
        start_set.len(),
        2,
        "the closed loop carries exactly two a.m-critical points: {start_set:?}"
    );
    for cert in &start_set {
        assert_point_cert4(cert);
    }
    // The certified boxes contain the two known loop critical points
    // (u = 1/3 ± 1/4 at v = 1/3, on the diagonal s = u, t = v).
    let known = [
        [1.0 / 12.0, 1.0 / 3.0, 1.0 / 12.0, 1.0 / 3.0],
        [7.0 / 12.0, 1.0 / 3.0, 7.0 / 12.0, 1.0 / 3.0],
    ];
    let mut hit = [false; 2];
    for cert in &start_set {
        for (k, root) in known.iter().enumerate() {
            if contains_root(&cert.box_, *root) {
                hit[k] = true;
            }
        }
    }
    assert!(hit[0], "the left critical point must be isolated");
    assert!(hit[1], "the right critical point must be isolated");
}

// ---------------------------------------------------------------------------
// Test 3: exclusion clears the remainder
// ---------------------------------------------------------------------------

#[test]
fn exclusion_clears_the_remainder() {
    // (a) The straight-line fixture with the constant covector `a = (0,1,0,0)`
    // (a.m = −1): the whole domain contains the curve Z but no critical
    // point, and the a.m exclusion clears every cell — the start set is
    // complete and empty.
    let line_system = straight_line_system();
    let domain = box4([0.0, 0.0, 0.0, 0.0], [1.0, 1.0, 1.0, 1.0]);
    match tier2::tier2_start_set(&line_system, [0.0, 1.0, 0.0, 0.0], domain) {
        tier2::TierTwoOutcome::Complete { start_set } => {
            assert!(
                start_set.is_empty(),
                "a straight curve with no critical point must clear entirely by exclusion: {start_set:?}"
            );
        }
        tier2::TierTwoOutcome::Refused(refusal) => {
            panic!("the straight line must clear by exclusion, refused: {refusal:?}")
        }
    }

    let system = loop_system();
    let a = [1.0, 0.0, 0.0, 0.0];

    // (b) A box that contains parts of the loop but NO critical point (the
    // v-axis is bounded away from 1/3, so the a.m exclusion clears every
    // cell): the loop arcs in the box carry no critical point and are
    // excluded, never stalling.
    let top_cap = box4([0.08, 0.45, 0.08, 0.45], [0.6, 0.65, 0.6, 0.65]);
    match tier2::tier2_start_set(&system, a, top_cap) {
        tier2::TierTwoOutcome::Complete { start_set } => {
            assert!(
                start_set.is_empty(),
                "a critical-point-free arc box must yield an empty start set: {start_set:?}"
            );
        }
        tier2::TierTwoOutcome::Refused(refusal) => {
            panic!("the critical-point-free arc box must clear by exclusion, refused: {refusal:?}")
        }
    }

    // (c) A box with no zero set at all, straddling the a.m exclusion
    // hypersurface `v = 1/3`: the F-component exclusion clears the cells the
    // cheap a.m form cannot, and nothing stalls.
    let away = box4([0.2, 0.2, 0.2, 0.2], [0.5, 0.45, 0.5, 0.45]);
    match tier2::tier2_start_set(&system, a, away) {
        tier2::TierTwoOutcome::Complete { start_set } => {
            assert!(
                start_set.is_empty(),
                "a zero-free box must yield an empty start set: {start_set:?}"
            );
        }
        tier2::TierTwoOutcome::Refused(refusal) => {
            panic!("the zero-free box must clear by exclusion, refused: {refusal:?}")
        }
    }

    // (d) The full loop: exactly the two certified zeros are isolated and
    // every remaining cell of the domain (the whole remainder, including the
    // non-critical loop arcs) is excluded — no spurious certificates, no
    // stall.
    match tier2::tier2_start_set(&system, a, domain) {
        tier2::TierTwoOutcome::Complete { start_set } => {
            assert_eq!(
                start_set.len(),
                2,
                "the full loop must isolate exactly the two known zeros: {start_set:?}"
            );
        }
        tier2::TierTwoOutcome::Refused(refusal) => {
            panic!("the full loop must complete, refused: {refusal:?}")
        }
    }
}

// ---------------------------------------------------------------------------
// Tests 4 & 5: the a-posteriori routing (tangential curve vs incomplete)
// ---------------------------------------------------------------------------

/// Assert the refusal carries the named kind with the Inconclusive backing.
fn assert_inconclusive_refusal(outcome: &tier2::TierTwoOutcome, kind: RefusalKind) {
    match outcome {
        tier2::TierTwoOutcome::Complete { .. } => {
            panic!("expected a refused outcome, got Complete")
        }
        tier2::TierTwoOutcome::Refused(refusal) => {
            assert_eq!(refusal.kind, kind);
            assert_eq!(refusal.backing, VerdictClass::Inconclusive);
        }
    }
}

#[test]
fn persistent_positive_dimensional_psi_a_routes_to_tangential() {
    // The tangential line: m = 0 along the whole zero set, so Psi_a's zero
    // set is a curve for EVERY direction. The bounded subdivision cannot
    // isolate it and every cell of the shrinking family still carries zero:
    // this routes to §10.4 as TangentialCurve, never IncompleteStartSet.
    let system = tangential_system();
    let a = [1.0, 1.0, 1.0, 1.0];
    let domain = box4([0.0, 0.0, 0.0, 0.0], [1.0, 1.0, 1.0, 1.0]);
    let outcome = tier2::tier2_start_set(&system, a, domain);
    assert_inconclusive_refusal(&outcome, RefusalKind::TangentialCurve);
    match &outcome {
        tier2::TierTwoOutcome::Refused(refusal) => {
            let name = match &refusal.evidence {
                truck_certified::kernel::evidence::RefusalEvidence::Predicate { name, .. } => *name,
                _ => "",
            };
            assert_eq!(name, "tier2_persistent_positive_dimensional");
        }
        _ => {}
    }
}

#[test]
fn ka_perturbation_retries_then_incomplete_start_set() {
    // The branch crossing: Z is two straight branches crossing at the origin,
    // a singular point that lies in Psi_a⁻¹(0) for every direction but that
    // no Krawczyk box can isolate (the local Jacobian is singular). The
    // caller's direction stalls on a bounded leaf set; all KA deterministic
    // perturbations stall the same way, and the start set is refused
    // IncompleteStartSet — after KA direction attempts.
    let system = crossing_system();
    let a = [1.0, 1.0, 0.0, 0.0];
    let domain = box4([0.0, 0.0, 0.0, 0.0], [0.25, 0.25, 0.25, 0.25]);
    let outcome = tier2::tier2_start_set(&system, a, domain);
    assert_inconclusive_refusal(&outcome, RefusalKind::IncompleteStartSet);
    // The refusal names the count: the caller direction plus KA perturbations
    // were attempted before giving up.
    match &outcome {
        tier2::TierTwoOutcome::Refused(refusal) => {
            let detail = match &refusal.evidence {
                truck_certified::kernel::evidence::RefusalEvidence::Predicate {
                    detail, ..
                } => detail,
                _ => panic!("the incomplete refusal must carry predicate evidence"),
            };
            assert!(
                detail.contains(&format!("ka={}", config::KA)),
                "the incomplete refusal must pin the KA perturbation count: {detail}"
            );
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Test 6: Corollary 9.3 — boundary seeds + Psi_a zeros cover every component
// ---------------------------------------------------------------------------

/// The four boundary curve leaves of a surface leaf (the u = 0, u = 1, v = 0,
/// v = 1 net boundary rows/columns), homogeneous `xyzw` preserved.
fn boundary_curves(leaf: &BezierLeaf, chart: ChartId) -> Vec<BezierLeaf1> {
    let width = leaf.degree_v + 1;
    let mut out = Vec::with_capacity(4);
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

#[test]
fn boundary_seeds_plus_psi_a_cover_every_oracle_component() {
    // The composition fixture's zero set has exactly two connected components
    // in the leaf product: the two straight arcs `u = 1/3` and `u = 2/3`
    // (each with `s = u`, `t = 2/5 + v/5`), crossing the full v-range. Every
    // component meets the leaf boundary, so Theorem 9.2's boundary case
    // applies: the §9.3 boundary seeds (R8 hits of each arc's two non-dyadic
    // endpoints against the plane) cover both components, and Psi_a — run over
    // the same leaf product — certifies that no interior critical point is
    // missed. Together (Corollary 9.3) they cover every oracle component.
    let (p, system) = composition_system();
    let q = plane_leaf();
    let chart = ChartId(0);
    let p_edges = boundary_curves(&p, chart);

    // The four boundary seeds: each of the two arcs pierces the plane on the
    // leaf's v = 0 and v = 1 edges, at the non-dyadic R8 roots (over the
    // curve parameter and the plane's own parameters) (1/3, 1/3, 2/5),
    // (1/3, 1/3, 3/5), (2/3, 2/3, 2/5), and (2/3, 2/3, 3/5).
    let seeds = construct_ok(tier1::boundary_seeds(&p, &p_edges, &q, &[]));
    let arc_roots: [[f64; 3]; 4] = [
        [1.0 / 3.0, 1.0 / 3.0, 2.0 / 5.0],
        [1.0 / 3.0, 1.0 / 3.0, 3.0 / 5.0],
        [2.0 / 3.0, 2.0 / 3.0, 2.0 / 5.0],
        [2.0 / 3.0, 2.0 / 3.0, 3.0 / 5.0],
    ];
    let mut arc_hit = [false; 4];
    for seed in &seeds {
        assert_eq!(seed.residual, ResidualId::R8);
        for (k, root) in arc_roots.iter().enumerate() {
            let on = (0..3).all(|axis| {
                seed.box_.lo[axis] - ROOT_TOL <= root[axis]
                    && root[axis] <= seed.box_.hi[axis] + ROOT_TOL
            });
            if on {
                arc_hit[k] = true;
            }
        }
    }
    assert!(
        arc_hit.iter().all(|h| *h),
        "every arc boundary crossing must be a boundary seed: {seeds:?}"
    );

    // The Tier-2 Psi_a run over the leaf product: both components meet the
    // boundary, so the start set isolates no interior critical point and is
    // complete and empty (a = (1,1,0,0) has a.m = f_v − f_u, which is nonzero
    // on both arcs). The domain is a band around the u = 1/3 arc, whose arc
    // pieces meet the band boundary.
    let a = [1.0, 1.0, 0.0, 0.0];
    let domain = box4([0.2, 0.05, 0.2, 0.05], [0.45, 0.95, 0.45, 0.95]);
    let outcome = tier2::tier2_start_set(&system, a, domain);
    match &outcome {
        tier2::TierTwoOutcome::Complete { start_set } => {
            assert!(
                start_set.is_empty(),
                "no interior critical point exists on boundary-meeting arcs: {start_set:?}"
            );
        }
        tier2::TierTwoOutcome::Refused(refusal) => {
            panic!("the composition fixture must complete, refused: {refusal:?}")
        }
    }

    // Corollary 9.3: the boundary seeds cover the arc component at u = 1/3
    // (its two crossings) and the arc component at u = 2/3 (its two
    // crossings) — every one of the two oracle components of Z is hit by the
    // combined start set, none is missed.
    assert!(
        arc_hit[0] && arc_hit[1],
        "the u = 1/3 arc component is covered by boundary seeds"
    );
    assert!(
        arc_hit[2] && arc_hit[3],
        "the u = 2/3 arc component is covered by boundary seeds"
    );
}

// ---------------------------------------------------------------------------
// Test 7: N4 source scan over the Tier-2 module
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
fn no_transcendental_call_in_tier2_module() {
    // N4 discipline: the tier2 module may not call sin, cos, atan2, exp, ln,
    // log, or powf outside comments (there is no sqrt normalization in this
    // module, so sqrt is scanned too).
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/kernel/tier2.rs");
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(err) => panic!("tier2.rs must be readable: {err}"),
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
            "no transcendental call may appear outside comments in tier2.rs (found {needle})"
        );
    }
}
