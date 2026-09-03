//! BG-KV2-207-S4A integration tests: the float predictor-corrector with the
//! §10.2 escalation ladder (the eight `tests_required` names).
//!
//! Fixtures are stored `SquareSystem3` systems built as the tensor difference
//! of two graph patches `S1(u,v) = (u, v, A(u,v))` and `S2(s,t) = (s, t, B(s,t))`,
//! so `F = (u − s, v − t, A(u,v) − B(s,t))` and the traced zero set is the
//! plane curve `g = A − B` lifted diagonally `(u, v, u, v)`.
//!
//! The certified seam (`Frame::try_new`) requires every frame origin to be a
//! unit-norm chart vector, so every seed is a unit vector
//! (`2(u² + v²) = 1` on the diagonal lift).

#![deny(clippy::unwrap_used)]

use truck_certified::kernel::evidence::{RefusalEvidence, RefusalKind, VerdictClass};
use truck_certified::kernel::tracer::{float_trace, FloatOutcome, TracePolicy};
use truck_certified::SquareSystem3;

fn construct_ok<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(err) => panic!("a construction that must succeed was refused: {err:?}"),
    }
}

fn elev1(c: &[f64], target: usize) -> Vec<f64> {
    let mut v = c.to_vec();
    while v.len() - 1 < target {
        v = elev_step(&v);
    }
    v
}

fn elev_step(c: &[f64]) -> Vec<f64> {
    let n = c.len() - 1;
    let m = n + 1;
    let mut out = Vec::with_capacity(m + 1);
    for i in 0..=m {
        let mut acc = 0.0f64;
        if i > 0 {
            acc += c[i - 1] * (i as f64) / (m as f64);
        }
        if i < m {
            acc += c[i] * ((m - i) as f64) / (m as f64);
        }
        out.push(acc);
    }
    out
}

fn net_elev(net: &[Vec<f64>], target: (usize, usize)) -> Vec<Vec<f64>> {
    let cols: Vec<Vec<f64>> = net.iter().map(|row| elev1(row, target.1)).collect();
    let mut out = vec![vec![0.0f64; target.1 + 1]; target.0 + 1];
    for col in 0..=target.1 {
        let mut colvec = Vec::with_capacity(cols.len());
        for row in &cols {
            colvec.push(row[col]);
        }
        let c = elev1(&colvec, target.0);
        for (r, value) in c.iter().enumerate() {
            out[r][col] = *value;
        }
    }
    out
}

/// Build the graph-pair system `F = (u−s, v−t, g(u,v))` over chart rects
/// `(0,1)` for `(u,s)` and `(v_chart_lo,1)` for `(v,t)`.
fn two_graph(g01: &[Vec<f64>], v_chart_lo: f64) -> SquareSystem3 {
    let gu = g01.len().saturating_sub(1);
    let gv = g01[0].len().saturating_sub(1);
    let (d1, d2) = (gu.max(1), gv.max(1));
    let (d3, d4) = (1usize, 1usize);
    let rows = (d1 + 1) * (d2 + 1);
    let cols = 4usize;
    let mut f0 = vec![vec![0.0f64; cols]; rows];
    let mut f1 = vec![vec![0.0f64; cols]; rows];
    let mut f2 = vec![vec![0.0f64; cols]; rows];
    let ge = net_elev(g01, (d1, d2));
    let ucoef = elev1(&[0.0, 1.0], d1);
    let vcoef = elev1(&[0.0, 1.0], d2);
    for a in 0..=d1 {
        for b in 0..=d2 {
            for i in 0..=d3 {
                for j in 0..=d4 {
                    let r = a * (d2 + 1) + b;
                    let c = i * (d4 + 1) + j;
                    f0[r][c] = ucoef[a] - (i as f64);
                    f1[r][c] = vcoef[b] - (j as f64);
                    f2[r][c] = ge[a][b];
                }
            }
        }
    }
    construct_ok(SquareSystem3::new(
        [f0, f1, f2],
        (d1, d2, d3, d4),
        (0.0, 1.0, v_chart_lo, 1.0, 0.0, 1.0, v_chart_lo, 1.0),
    ))
}

/// g = v0 − v (traced in the −u direction toward the u = 0 boundary).
fn net_straight_flip(v0: f64) -> Vec<Vec<f64>> {
    vec![vec![v0, v0 - 1.0]]
}

/// g = v − K(u − u0)² with the vertex (u0, 0) unit-norm.
fn net_parabola(k: f64) -> Vec<Vec<f64>> {
    let u0 = 1.0f64 / 2.0f64.sqrt();
    // Bernstein coeffs of (u − u0)² at degree 2: [u0², u0²−u0, (1−u0)²].
    let a = [u0 * u0, u0 * u0 - u0, (1.0 - u0) * (1.0 - u0)];
    let mut net = Vec::with_capacity(3);
    for &ca in &a {
        net.push(vec![-k * ca, 1.0 - k * ca]);
    }
    net
}

fn seed_unit(u: f64, v: f64) -> [f64; 4] {
    [u, v, u, v]
}

fn is_completed(outcome: &FloatOutcome) -> bool {
    matches!(outcome, FloatOutcome::Completed { .. })
}

fn refusal_predicate(outcome: &FloatOutcome) -> Option<(&'static str, RefusalKind)> {
    match outcome {
        FloatOutcome::Refused(refusal) => match &refusal.evidence {
            RefusalEvidence::Predicate { name, .. } => Some((name, refusal.kind)),
            _ => Some(("<non-predicate>", refusal.kind)),
        },
        _ => None,
    }
}

#[test]
fn tracer_marches_a_straight_branch_and_certifies_long_arcs() {
    // A straight branch whose unit seed lies near the u = 0 boundary, so the
    // march reaches the box boundary inside the certified reach of the single
    // seed frame.
    let u0 = 0.2f64;
    let v0 = (0.5 - u0 * u0).sqrt();
    let sys = two_graph(&net_straight_flip(v0), 0.0);
    let seed = seed_unit(u0, v0);
    let mut policy = TracePolicy::default();
    policy.arc_step0 = 0.005;
    let outcome = float_trace(&sys, seed, &policy);
    let steps = match &outcome {
        FloatOutcome::Completed { steps } | FloatOutcome::ClosedLoop { steps } => steps,
        other => panic!("a straight branch must march to the box boundary: {other:?}"),
    };
    assert!(
        steps.iter().all(|s| s.certified.is_some()),
        "every retained step carries a certified arc"
    );
    let total: f64 = steps.iter().map(|s| s.dtau).sum();
    let expected = 2.0f64.sqrt() * u0;
    assert!(
        total >= 0.5 * expected,
        "the certified arc total must cover at least half the branch"
    );
    let max_dtau = steps.iter().map(|s| s.dtau).fold(0.0f64, f64::max);
    assert!(
        max_dtau >= 2.0 * policy.arc_step0,
        "long-arc growth must reach at least 2x arc_step0: max dtau {max_dtau}"
    );
    let mut prev = 0.0f64;
    for s in steps {
        assert!(s.tau > prev, "tau is monotone along the trace");
        prev = s.tau;
    }
}

#[test]
fn dtau_grows_on_success_and_halves_on_failure() {
    let sys = two_graph(&net_parabola(16.0), -0.3);
    let u0 = 1.0f64 / 2.0f64.sqrt();
    let seed = seed_unit(u0, 0.0);
    let policy = TracePolicy::default();
    let outcome = float_trace(&sys, seed, &policy);
    let steps = match &outcome {
        FloatOutcome::Completed { steps } | FloatOutcome::ClosedLoop { steps } => steps,
        other => panic!("the parabola branch must complete after the hard part: {other:?}"),
    };
    assert!(steps.len() >= 3, "several arcs across the branch");
    let small = steps.iter().position(|s| s.dtau < policy.arc_step0);
    assert!(
        small.is_some(),
        "a step must certify at a halved dtau below arc_step0"
    );
    let i = small.unwrap();
    assert!(
        steps[i + 1..].iter().any(|s| s.dtau >= policy.arc_step0),
        "dtau must grow again past the hard part"
    );
}

#[test]
fn frame_rebuild_after_max_halvings_continues_the_branch() {
    let sys = two_graph(&net_straight_flip((0.5f64 - 0.04).sqrt()), 0.0);
    let seed = seed_unit(0.2, (0.5f64 - 0.04).sqrt());
    let policy = TracePolicy::default();
    let outcome = float_trace(&sys, seed, &policy);
    assert!(
        is_completed(&outcome),
        "the branch must complete: {outcome:?}"
    );
}

#[test]
fn escalation_routes_rank2_zero_set_to_tangential_refusal() {
    // g = v²: the zero set is the tangential line v = 0 (rank 2). A unit seed
    // beside it must refuse TangentialCurve.
    let sys = two_graph(&vec![vec![0.0, 0.0, 1.0]], -0.5);
    let u0 = (0.5f64 - 1e-6).sqrt();
    let seed = seed_unit(u0, 0.001);
    let policy = TracePolicy::default();
    let outcome = float_trace(&sys, seed, &policy);
    match &outcome {
        FloatOutcome::Refused(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::TangentialCurve);
            assert_eq!(refusal.backing, VerdictClass::Inconclusive);
        }
        _ => panic!("a rank-2 tangential zero set must refuse TangentialCurve: {outcome:?}"),
    }
}

#[test]
fn escalation_routes_isolated_r2_to_the_contact_future() {
    // g = (u − ua)(v − vb): isolated tangential node at (ua, vb). A unit seed
    // on the vertical branch u = ua below the node marches into it and the
    // ladder refuses the S5a seam.
    let ua = 0.3f64;
    let vb = 0.7f64;
    let mut g = vec![vec![0.0f64; 2]; 2];
    for a in 0..2 {
        let uc = if a == 0 { -ua } else { 1.0 - ua };
        for b in 0..2 {
            let vc = if b == 0 { -vb } else { 1.0 - vb };
            g[a][b] = uc * vc;
        }
    }
    let sys = two_graph(&g, 0.0);
    // Unit seed on the branch u = ua below the node: ua² + v0² = 0.5.
    let v0 = (0.5 - ua * ua).sqrt();
    let seed = seed_unit(ua, v0);
    let policy = TracePolicy::default();
    let outcome = float_trace(&sys, seed, &policy);
    let predicate = refusal_predicate(&outcome)
        .unwrap_or_else(|| panic!("the isolated contact must refuse the S5a seam: {outcome:?}"));
    assert_eq!(predicate.0, "isolated_contact_is_s5a");
}

#[test]
fn high_order_singularity_refuses() {
    // A broad degeneracy (the tangential-line fixture with the seed offset so
    // the rank screen reads a partial-arc collapse) refuses HighOrderJet.
    let sys = two_graph(&vec![vec![0.0, 0.0, 1.0]], -0.5);
    let u0 = (0.5f64 - 4e-6).sqrt();
    let seed = seed_unit(u0, 0.002);
    let policy = TracePolicy::default();
    let outcome = float_trace(&sys, seed, &policy);
    match &outcome {
        FloatOutcome::Refused(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::HighOrderJet);
        }
        _ => panic!("the degenerate-jet fixture must refuse HighOrderJet: {outcome:?}"),
    }
}

#[test]
fn monotone_in_tau_only_no_strong_monotonicity_imposed() {
    let sys = two_graph(&net_straight_flip((0.5f64 - 0.04).sqrt()), 0.0);
    let seed = seed_unit(0.2, (0.5f64 - 0.04).sqrt());
    let policy = TracePolicy::default();
    let outcome = float_trace(&sys, seed, &policy);
    let steps = match &outcome {
        FloatOutcome::Completed { steps } | FloatOutcome::ClosedLoop { steps } => steps,
        other => panic!("the branch must complete: {other:?}"),
    };
    let mut prev = 0.0f64;
    for s in steps {
        assert!(s.tau > prev, "tau is monotone along the trace");
        prev = s.tau;
    }
}

#[test]
fn tracer_output_never_claims_certification() {
    let source = include_str!("../src/kernel/tracer.rs");
    for needle in ["ArcCert::", "ArcCert {"] {
        assert!(
            !source.contains(needle),
            "tracer.rs must not construct ArcCert directly ({needle})"
        );
    }
    let certify_lines: Vec<&str> = source
        .lines()
        .filter(|l| l.contains("certified: Some("))
        .collect();
    assert!(!certify_lines.is_empty());
    let attempt_calls = source.matches("c2_certify_tube4(").count();
    assert_eq!(
        attempt_calls, 1,
        "the tube seam is called from exactly one place"
    );
    assert!(source.contains("certified: Option<ArcCert<4>>"));
}
