//! FSSI-001-GATE conformance (r2, Theorem C mechanism): the separable
//! tangency-free admission gate certifies TRANSVERSALITY (`rank DF = 3` at
//! every point of `Σ ∩ B`) — never emptiness, which stays the separate
//! Bernstein exclusion of `F`. The battery pins the six required behaviours:
//! a transverse box certifies TangencyFree; a tangent-curve fixture refuses
//! the named `TangentCurveSuspected`; a coincident patch pair refuses the
//! named `CoincidentPatchSuspected`; a near-tangent rank-3 pair PASSES (the
//! small margin is the returned conditioning evidence, never a refusal); a
//! loft with a collapsed apex row passes (the per-box gate is unaffected by
//! the pole away from the box); and the green spline-pair battery keeps its
//! landed certified verdict class (V5-pair identity — the gate is monotone-
//! widening, it can only admit or refuse, never flip a certified verdict).
//!
//! House rules: H-1 (no `unwrap`/`expect` reachable from geometry) applies,
//! so every fallible construction unpacks through explicit `panic!` arms.
//! All fixture grids are unit-weight D2 spline-admissible rational patches.

#![deny(clippy::unwrap_used)]

use truck_certified::ssi::ssi_gate::{gate_square_system, GateAdmission, GateParams};
use truck_certified::ssi::{
    construct_square_system, krawczyk3_certificate, RationalBipatch, SsiParticipant, SsiRefusal,
};
use truck_certified::ssi_fixtures as fx;
use truck_certified::ssi_types::SquareSystem3;

/// The admission budget of the refusal fixtures: enough levels and cells that
/// a decidable box resolves and an undecidable one reaches the suspicion halt.
fn refusal_params() -> GateParams {
    GateParams {
        max_cells: 20000,
        max_level: 12,
    }
}

/// The admission budget of the passing fixtures (interior boxes certify at
/// shallow depth).
fn pass_params() -> GateParams {
    GateParams {
        max_cells: 20000,
        max_level: 12,
    }
}

/// Unpack a fallible fixture construction, panicking with a message (never an
/// `unwrap`).
fn admit<T>(result: Result<T, &'static str>) -> T {
    match result {
        Ok(value) => value,
        Err(what) => panic!("fixture construction failed: {what}"),
    }
}

/// Small exact binomial coefficient `C(n, k)`.
fn binom(n: usize, k: usize) -> f64 {
    let k = k.min(n - k);
    let mut acc = 1.0f64;
    for i in 0..k {
        acc = acc * (n - i) as f64 / (i + 1) as f64;
    }
    acc
}

/// The `(m + 1) × (n + 1)` Bernstein grid of `u^pu · v^pv` at bidegree `(m, n)`,
/// from the monomial-to-Bernstein coefficient conversion.
fn monomial_grid(m: usize, n: usize, terms: &[(usize, usize, f64)]) -> Vec<Vec<f64>> {
    let mut grid = vec![vec![0.0f64; n + 1]; m + 1];
    for &(pu, pv, coeff) in terms {
        let row_factors: Vec<f64> = (pu..=m).map(|a| binom(a, pu) / binom(m, pu)).collect();
        let col_factors: Vec<f64> = (pv..=n).map(|b| binom(b, pv) / binom(n, pv)).collect();
        for (a, fa) in (pu..=m).zip(row_factors.iter()) {
            for (b, fb) in (pv..=n).zip(col_factors.iter()) {
                grid[a][b] += coeff * fa * fb;
            }
        }
    }
    grid
}

/// The `(m + 1) × (n + 1)` Bernstein grid of the first (`which == 0`) or second
/// (`which == 1`) unit-chart coordinate.
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

/// The all-ones weight grid (the D2 unit-weight certificate).
fn ones(m: usize, n: usize) -> Vec<Vec<f64>> {
    vec![vec![1.0f64; n + 1]; m + 1]
}

/// A unit-weight graph patch `(x, y, z) = (u, v, h(u, v))` at bidegree `(m, n)`.
fn graph_patch(m: usize, n: usize, h: Vec<Vec<f64>>) -> RationalBipatch {
    let patch = RationalBipatch::new(
        m,
        n,
        [coord_grid(m, n, 0), coord_grid(m, n, 1), h],
        ones(m, n),
    );
    admit(patch.map_err(|_| "a valid unit-weight graph patch was refused"))
}

/// A bidegree-`(1, 1)` unit-weight plane graph over the unit chart.
fn plane_graph(z: Vec<Vec<f64>>) -> RationalBipatch {
    graph_patch(1, 1, z)
}

/// The plane `z = c + a·s + b·t` over the second chart as a graph patch.
fn plane_graph_expr(a: f64, b: f64, c: f64) -> RationalBipatch {
    let m = 1usize;
    let n = 1usize;
    let mut z = vec![vec![0.0f64; n + 1]; m + 1];
    for (r, row) in z.iter_mut().enumerate() {
        for (col, cell) in row.iter_mut().enumerate() {
            let s = r as f64;
            let t = col as f64;
            *cell = c + a * s + b * t;
        }
    }
    plane_graph(z)
}

/// Build the square system of a unit-weight patch pair through the certified
/// constructor.
fn system_of(a: &RationalBipatch, b: &RationalBipatch) -> SquareSystem3 {
    let system = construct_square_system(
        &SsiParticipant::RationalBipatch(a.clone()),
        &SsiParticipant::RationalBipatch(b.clone()),
    );
    admit(system.map_err(|_| "the spline pair constructs a square system"))
}

/// The certified TangencyFree margin of a passing box, or a panic with the
/// refusal tag.
fn expect_tangency_free(
    system: &SquareSystem3,
    box_: [(f64, f64); 4],
    params: &GateParams,
) -> (f64, f64) {
    match gate_square_system(system, box_, params) {
        Ok(GateAdmission::TangencyFree { margin }) => margin,
        Ok(other) => panic!("expected TangencyFree, got {}", other.tag()),
        Err(refusal) => panic!("expected TangencyFree, got refusal {}", refusal.tag()),
    }
}

/// 1-D de Casteljau evaluation (the fixture tests' plain-`f64` reference).
fn bernstein_eval(coeffs: &[f64], x: f64) -> f64 {
    let mut level = coeffs.to_vec();
    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len() - 1);
        for pair in level.windows(2) {
            next.push(pair[0] + x * (pair[1] - pair[0]));
        }
        level = next;
    }
    level[0]
}

/// 2-D Bernstein evaluation of a grid at `(p, q)`.
fn bernstein_eval_2d(grid: &[Vec<f64>], p: f64, q: f64) -> f64 {
    let rows: Vec<f64> = grid.iter().map(|row| bernstein_eval(row, q)).collect();
    bernstein_eval(&rows, p)
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// The apex-loft patch `X(u, v) = ((1 − v)·u, (1 − v)·u², v)`: the top
/// (`v = 1`) row collapses to the apex point `(0, 0, 1)` for every `u`, so the
/// parametric normal `n = (1 − v)·(2u, −1, u²)` vanishes on the whole collapsed
/// row. Bidegree `(2, 2)`.
fn apex_loft_patch() -> RationalBipatch {
    let x = monomial_grid(2, 2, &[(1, 0, 1.0), (1, 1, -1.0)]);
    let y = monomial_grid(2, 2, &[(2, 0, 1.0), (2, 1, -1.0)]);
    let z = monomial_grid(2, 2, &[(0, 1, 1.0)]);
    admit(
        RationalBipatch::new(2, 2, [x, y, z], ones(2, 2))
            .map_err(|_| "the apex-loft patch is a valid unit-weight patch"),
    )
}

/// The transverse pair: the plane `z = v` (graph `(u, v, v)`) against the plane
/// `z = 1/4 + s/2`. Normals `(0, −1, 1)` and `(−1/2, 0, 1)` are never parallel,
/// so any product box certifies.
fn transverse_pair() -> SquareSystem3 {
    let m = 1usize;
    let n = 1usize;
    let a = graph_patch(m, n, coord_grid(m, n, 1));
    let b = plane_graph_expr(0.5, 0.0, 0.25);
    system_of(&a, &b)
}

/// The near-tangent rank-3 pair: the plane `z = 0` against the plane
/// `z = ε·s` with the small dihedral angle `ε`. The intersection is genuinely
/// transverse (rank 3) everywhere — the minimal `‖n_X × n_Y‖` over any box is
/// `~ ε > 0` — so the gate must PASS it, with the small margin as the
/// conditioning evidence (never a refusal).
fn near_tangent_pair(eps: f64) -> SquareSystem3 {
    let m = 1usize;
    let n = 1usize;
    let a = plane_graph(vec![vec![0.0f64; n + 1]; m + 1]);
    let mut z = vec![vec![0.0f64; n + 1]; m + 1];
    for (r, row) in z.iter_mut().enumerate() {
        for (col, cell) in row.iter_mut().enumerate() {
            let s = r as f64;
            let _ = col;
            *cell = eps * s;
        }
    }
    let b = plane_graph(z);
    system_of(&a, &b)
}

/// The tangent-curve fixture: the plane `z = 0` against the extruded parabola
/// `z = (s − 1/2)²`. The zero set of the difference is the curve
/// `{u = s = 1/2, v = t}` in the chart, and the two surfaces share their
/// tangent plane along it (the normals are parallel at `s = 1/2`), so the box
/// can never certify tangency-free and the gate must refuse
/// `TangentCurveSuspected` (a sub-linear undecided shrink, never area-scale).
fn tangent_curve_pair() -> SquareSystem3 {
    let m = 2usize;
    let n = 1usize;
    let h = monomial_grid(m, n, &[(2, 0, 1.0), (1, 0, -1.0), (0, 0, 0.25)]);
    let a = plane_graph(vec![vec![0.0f64; 2]; 2]);
    let b = graph_patch(m, n, h);
    system_of(&a, &b)
}

/// The coincident-patch fixture: the SAME bowl `(u, v, (u − 1/2)² + (v − 1/2)²)`
/// on both sides, so the two surfaces coincide over the box and the normals
/// are parallel everywhere. The undecided measure never shrinks (area-scale
/// stagnation), so the gate must refuse `CoincidentPatchSuspected`.
fn coincident_pair() -> SquareSystem3 {
    let m = 2usize;
    let n = 2usize;
    let h = monomial_grid(
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
    let patch = graph_patch(m, n, h);
    system_of(&patch, &patch)
}

/// The box that contains the whole interior of the chart on both sides.
fn full_box() -> [(f64, f64); 4] {
    [(0.1, 0.9), (0.1, 0.9), (0.1, 0.9), (0.1, 0.9)]
}

// ---------------------------------------------------------------------------
// Required tests
// ---------------------------------------------------------------------------

#[test]
fn gate_admit_certifies_transverse_box() {
    // Two transverse planes: the gate certifies rank DF = 3 on Σ ∩ B over the
    // whole box at the top level, with a strictly positive certified margin on
    // the minimal ‖n_X × n_Y‖.
    let system = transverse_pair();
    let margin = expect_tangency_free(&system, full_box(), &pass_params());
    assert!(
        margin.0 > 0.0 && margin.0 <= margin.1,
        "the certified margin is a positive enclosure, got {margin:?}"
    );
    assert!(margin.1.is_finite(), "the margin is finite");

    // Deterministic: an identical second run returns the identical margin.
    let again = expect_tangency_free(&system, full_box(), &pass_params());
    assert_eq!(margin, again, "the gate verdict is deterministic");
}

#[test]
fn tangent_curve_fixture_refuses_named_case() {
    // The tangent-curve pair refuses the typed TangentCurveSuspected halt
    // (sub-linear undecided shrink at budget exhaustion), never a wrong
    // acceptance.
    let system = tangent_curve_pair();
    let box_ = full_box();
    match gate_square_system(&system, box_, &refusal_params()) {
        Err(SsiRefusal::TangentCurveSuspected { margin }) => {
            assert!(
                margin.0.is_finite() && margin.0 > 0.0,
                "the undecided measure at halt is positive and finite, got {margin:?}"
            );
            assert!(
                margin.1.is_finite() && margin.1 >= 0.0,
                "the shrink exponent is finite and nonnegative, got {margin:?}"
            );
            assert!(
                margin.1 < 4.0,
                "a tangent curve shrinks slower than the full-dimensional 2^-4k \
                 rate, got exponent {}",
                margin.1
            );
        }
        Err(refusal) => panic!("expected TangentCurveSuspected, got {}", refusal.tag()),
        Ok(verdict) => panic!("expected a refusal, got {}", verdict.tag()),
    }
}

#[test]
fn coincident_patch_fixture_refuses_named_case() {
    // The coincident bowl pair refuses the typed CoincidentPatchSuspected halt
    // (area-scale stagnation: the undecided measure never shrank).
    let system = coincident_pair();
    let box_ = full_box();
    match gate_square_system(&system, box_, &refusal_params()) {
        Err(SsiRefusal::CoincidentPatchSuspected { margin }) => {
            assert!(
                margin.0.is_finite() && margin.0 > 0.0,
                "the undecided measure at halt is positive and finite, got {margin:?}"
            );
            assert!(
                margin.1.is_finite() && margin.1 >= 0.0,
                "the shrink exponent is finite and nonnegative, got {margin:?}"
            );
        }
        Err(refusal) => panic!("expected CoincidentPatchSuspected, got {}", refusal.tag()),
        Ok(verdict) => panic!("expected a refusal, got {}", verdict.tag()),
    }
}

#[test]
fn near_tangent_rank3_pair_passes_gate() {
    // Near-tangency with rank 3 PASSES (theory §1.3): the pair is genuinely
    // transverse, so the gate admits it. The conditioning cost is visible in
    // the returned margin — small (on the order of the dihedral angle ε), never
    // a refusal.
    let eps = 1.0e-3;
    let system = near_tangent_pair(eps);
    let margin = expect_tangency_free(&system, full_box(), &pass_params());
    assert!(
        margin.0 > 0.0,
        "the near-tangent pair certifies with margin > 0"
    );
    assert!(
        margin.0 < 5.0e-3,
        "the near-tangent margin is small (the conditioning cost is visible in \
         the returned margin, not a refusal), got {}",
        margin.0
    );
}

#[test]
fn loft_apex_pole_fixture_passes_gate() {
    // A loft with a collapsed apex row PASSES the gate (theory §4.1 regression
    // that kills the naive global-q_t design): the normal net vanishes on the
    // collapsed apex row, but the gate decides PER BOX over the regular part
    // (the F3 selector's surviving-coordinate region), so an interior box over
    // the loft certifies tangency-free against the transverse plane.
    let loft = apex_loft_patch();
    // Machine-check the collapsed apex row: X(u, 1) = (0, 0, 1) for every u.
    let x = &loft.numerator()[0];
    let y = &loft.numerator()[1];
    let z = &loft.numerator()[2];
    for u in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let px = bernstein_eval_2d(x, u, 1.0);
        let py = bernstein_eval_2d(y, u, 1.0);
        let pz = bernstein_eval_2d(z, u, 1.0);
        assert!(
            px.abs() < 1e-12 && py.abs() < 1e-12 && (pz - 1.0).abs() < 1e-12,
            "the v = 1 row must collapse to the apex (0, 0, 1), got ({px}, {py}, {pz}) at u = {u}"
        );
    }
    let plane = plane_graph_expr(2.0, 0.0, 0.5);
    let system = system_of(&loft, &plane);
    // An interior box over the regular part of the loft (v away from the
    // collapsed apex row at v = 1). The composed normal net over a box this
    // size is a sound but loose enclosure, so the separable cones need a few
    // subdivision levels before they separate; the admission budget below lets
    // that resolve (the gate certifies, never refuses).
    let box_: [(f64, f64); 4] = [(0.2, 0.5), (0.1, 0.4), (0.2, 0.5), (0.1, 0.4)];
    let params = GateParams {
        max_cells: 300_000,
        max_level: 24,
    };
    let margin = expect_tangency_free(&system, box_, &params);
    assert!(
        margin.0 > 0.0,
        "the apex-loft interior box certifies tangency-free, margin {margin:?}"
    );
}

#[test]
fn v5_pair_identity_on_green_spline_pairs() {
    // The V5-pair identity: the gate is monotone-widening — it can only admit
    // or refuse, never flip a landed certified verdict. Every box the landed
    // SSI engine certifies (a GREEN spline pair) the gate admits
    // (TangencyFree), with a deterministic margin; and a box the landed engine
    // refuses as a genuine degeneracy the gate refuses with a suspicion (never
    // a wrong acceptance).
    //
    // Green spline-pair battery (landed fixtures + a further transverse pair).
    let well = match fx::well_conditioned_root() {
        Ok(fixture) => fixture,
        Err(_) => panic!("the well-conditioned fixture refused"),
    };
    let flipped = match fx::negative_orientation_root() {
        Ok(fixture) => fixture,
        Err(_) => panic!("the flipped-orientation fixture refused"),
    };

    let root_box: [(f64, f64); 4] = [(0.4, 0.6), (0.4, 0.6), (0.4, 0.6), (0.4, 0.6)];
    let green: Vec<(SquareSystem3, [(f64, f64); 4])> = vec![
        (well.system.clone(), root_box),
        (flipped.system.clone(), root_box),
        (transverse_pair(), root_box),
    ];
    for (system, box_) in &green {
        // The landed engine certifies the box on at least one continuation
        // axis (the pair is GREEN); scan the axes deterministically.
        let mut certified = false;
        for axis in 0..4 {
            if let Ok(certificate) = krawczyk3_certificate(system, axis, *box_) {
                let (d_lo, d_hi) = certificate.det();
                assert!(
                    d_lo > 0.0 || d_hi < 0.0,
                    "a green pair certifies with a determinant away from zero"
                );
                certified = true;
                break;
            }
        }
        assert!(certified, "the green pair must certify on some landed axis");
        // The gate admits the same box: TangencyFree, deterministic.
        let margin = expect_tangency_free(system, *box_, &pass_params());
        assert!(
            margin.0 > 0.0,
            "the green box admits with a positive margin"
        );
        let again = expect_tangency_free(system, *box_, &pass_params());
        assert_eq!(margin, again, "the gate verdict is deterministic per box");
    }

    // The degenerate side of the identity: the landed determinant-spans-zero
    // fixture is a coincident parabolic cylinder pair (identical surfaces);
    // the gate refuses it with the coincident suspicion — a landed refusal is
    // never flipped into an admission.
    let degenerate = match fx::determinant_spans_zero() {
        Ok(fixture) => fixture,
        Err(_) => panic!("the determinant-spans-zero fixture refused"),
    };
    match krawczyk3_certificate(&degenerate.system, 2, degenerate.box_) {
        Err(SsiRefusal::DeterminantSpansZero) => {}
        Err(refusal) => panic!("wrong landed refusal: {}", refusal.tag()),
        Ok(_) => panic!("the degenerate fixture must refuse on the landed engine"),
    }
    match gate_square_system(&degenerate.system, degenerate.box_, &refusal_params()) {
        Err(SsiRefusal::CoincidentPatchSuspected { margin }) => {
            assert!(
                margin.0.is_finite() && margin.0 > 0.0,
                "the coincident suspicion carries a positive undecided measure"
            );
        }
        Err(refusal) => panic!("expected CoincidentPatchSuspected, got {}", refusal.tag()),
        Ok(verdict) => panic!("the degenerate box must refuse, got {}", verdict.tag()),
    }
}
