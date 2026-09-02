//! The SSI wave shim's contract tests (BG-CK-P2-CONTRACT): the shared shapes,
//! their refusing constructors, and the fixture kit's machine-checked ground
//! truths. No solver is implemented or invoked here — every numerical fact is
//! a direct evaluation of a stored grid or an exact coefficient derivative.

#![deny(clippy::unwrap_used)]

use truck_certified::contract::{
    ContinuationCoordinate, CoordinateSwitch, IntervalEnclosure, Refusal,
};
use truck_certified::formal::contact::GenericUnresolved;
use truck_certified::formal::span::BranchGerm;
use truck_certified::hull::HullRefusal;
use truck_certified::ssi_fixtures as fx;
use truck_certified::ssi_types::{
    KrawczykCertificate3, SquareSystem3, TraceOutcome, TraceRefusal, TraceStep,
};

/// Assert two floats agree to the fixture kit's dyadic ground-truth tolerance.
fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9 // H-3: dyadic ground-truth comparison tolerance, fixture-only
}

fn assert_approx(a: f64, b: f64, what: &str) {
    assert!(approx(a, b), "{what}: expected {b}, got {a}");
}

/// Extract the `Ok` of a fallible construction; the fixture data is valid by
/// construction, so the refusal arm is a test-bug panic (never an unwrap).
fn construct<T>(result: Result<T, Refusal>) -> T {
    match result {
        Ok(value) => value,
        Err(_) => panic!("a fixture construction that must succeed was refused"),
    }
}

/// Direct system evaluation; shape mismatches are test bugs.
fn eval_sys(system: &SquareSystem3, uvst: (f64, f64, f64, f64)) -> [f64; 3] {
    match fx::eval_system(system, uvst) {
        Some(values) => values,
        None => panic!("grid/degrees shape mismatch in direct evaluation"),
    }
}

fn eval_z(
    grid: &[Vec<f64>],
    degrees: (usize, usize, usize, usize),
    uvst: (f64, f64, f64, f64),
) -> f64 {
    match fx::eval_grid4(grid, degrees, uvst) {
        Some(value) => value,
        None => panic!("grid/degrees shape mismatch in direct evaluation"),
    }
}

fn partial_z(
    grid: &[Vec<f64>],
    degrees: (usize, usize, usize, usize),
    axis: usize,
    uvst: (f64, f64, f64, f64),
) -> f64 {
    match fx::partial_grid4_axis(grid, degrees, axis, uvst) {
        Some(value) => value,
        None => panic!("axis partial unavailable for this fixture"),
    }
}

fn partial2_z(
    grid: &[Vec<f64>],
    degrees: (usize, usize, usize, usize),
    axis: usize,
    uvst: (f64, f64, f64, f64),
) -> f64 {
    match fx::second_partial_grid4_axis(grid, degrees, axis, uvst) {
        Some(value) => value,
        None => panic!("second partial unavailable for this fixture"),
    }
}

fn reduced_det(
    system: &SquareSystem3,
    continuation_axis: usize,
    uvst: (f64, f64, f64, f64),
) -> f64 {
    match fx::reduced_square_determinant(system, continuation_axis, uvst) {
        Some(value) => value,
        None => panic!("reduced determinant unavailable for this fixture"),
    }
}

fn reduced_diag(
    system: &SquareSystem3,
    continuation_axis: usize,
    uvst: (f64, f64, f64, f64),
) -> [f64; 3] {
    match fx::reduced_diagonal_entries(system, continuation_axis, uvst) {
        Some(values) => values,
        None => panic!("reduced diagonal unavailable for this fixture"),
    }
}

/// A square-shaped valid grid family for `(1,1,1,1)` systems.
fn valid_grids() -> [Vec<Vec<f64>>; 3] {
    let mut grids = [
        vec![vec![0.0; 4]; 4],
        vec![vec![0.0; 4]; 4],
        vec![vec![0.0; 4]; 4],
    ];
    for (k, grid) in grids.iter_mut().enumerate() {
        for (r, row) in grid.iter_mut().enumerate() {
            for (c, value) in row.iter_mut().enumerate() {
                *value = 100.0 * k as f64 + 10.0 * r as f64 + c as f64;
            }
        }
    }
    grids
}

#[test]
fn square_system3_refuses_ragged_empty_or_nonfinite_grids() {
    let maps = (0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0);

    let valid = valid_grids();
    let system = construct(SquareSystem3::new(valid.clone(), (1, 1, 1, 1), maps));
    assert_eq!(system.degrees(), (1, 1, 1, 1));
    assert_eq!(system.domain_maps(), maps);
    assert_eq!(system.grids(), &valid);

    let mut empty = valid_grids();
    empty[0] = Vec::new();
    assert_eq!(
        SquareSystem3::new(empty, (1, 1, 1, 1), maps),
        Err(Refusal::InvalidInput),
        "an empty grid refuses"
    );

    let mut ragged = valid_grids();
    ragged[1][2].pop();
    assert_eq!(
        SquareSystem3::new(ragged, (1, 1, 1, 1), maps),
        Err(Refusal::InvalidInput),
        "a ragged (short-row) grid refuses"
    );

    let mut nonfinite = valid_grids();
    nonfinite[2][0][1] = f64::NAN;
    assert_eq!(
        SquareSystem3::new(nonfinite, (1, 1, 1, 1), maps),
        Err(Refusal::InvalidInput),
        "a NaN coefficient refuses"
    );
    let mut infinite = valid_grids();
    infinite[0][3][3] = f64::INFINITY;
    assert_eq!(
        SquareSystem3::new(infinite, (1, 1, 1, 1), maps),
        Err(Refusal::InvalidInput),
        "an infinite coefficient refuses"
    );

    assert_eq!(
        SquareSystem3::new(valid_grids(), (0, 1, 1, 1), maps),
        Err(Refusal::InvalidInput),
        "a degree-0 input refuses (m1)"
    );
    assert_eq!(
        SquareSystem3::new(valid_grids(), (1, 0, 1, 1), maps),
        Err(Refusal::InvalidInput),
        "a degree-0 input refuses (n1)"
    );
    assert_eq!(
        SquareSystem3::new(valid_grids(), (1, 1, 1, 0), maps),
        Err(Refusal::InvalidInput),
        "a degree-0 input refuses (n2)"
    );

    let mut too_few_rows = valid_grids();
    too_few_rows[2].pop();
    assert_eq!(
        SquareSystem3::new(too_few_rows, (1, 1, 1, 1), maps),
        Err(Refusal::InvalidInput),
        "a degree-mismatched row count refuses"
    );

    assert_eq!(
        SquareSystem3::new(
            valid_grids(),
            (1, 1, 1, 1),
            (0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0)
        ),
        Err(Refusal::InvalidInput),
        "a degenerate chart interval refuses"
    );
    assert_eq!(
        SquareSystem3::new(
            valid_grids(),
            (1, 1, 1, 1),
            (0.0, f64::NAN, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0)
        ),
        Err(Refusal::InvalidInput),
        "a non-finite chart bound refuses"
    );
}

#[test]
fn krawczyk_certificate3_is_built_only_from_strict_inclusion() {
    let box_x = [(-1.0, 1.0), (0.0, 2.0), (0.0, 1.0)];
    let k_x = [(-0.5, 0.5), (0.25, 1.5), (0.1, 0.9)];
    let det = (2.0, 3.0);
    let certificate = construct(KrawczykCertificate3::new(box_x, k_x, det));
    assert_eq!(certificate.box_x(), box_x);
    assert_eq!(certificate.k_x(), k_x);
    assert_eq!(certificate.det(), det);

    let cases = [
        (
            "k touching the lower box bound is not strict",
            box_x,
            [(-1.0, 0.5), (0.25, 1.5), (0.1, 0.9)],
            det,
        ),
        (
            "k touching the upper box bound is not strict",
            box_x,
            [(-0.5, 1.0), (0.25, 1.5), (0.1, 0.9)],
            det,
        ),
        ("k equal to the box is not strict", box_x, box_x, det),
        (
            "k outside the box refuses",
            box_x,
            [(-0.5, 0.5), (0.25, 2.5), (0.1, 0.9)],
            det,
        ),
        (
            "a misordered k interval refuses",
            box_x,
            [(0.5, -0.5), (0.25, 1.5), (0.1, 0.9)],
            det,
        ),
        (
            "a non-finite k bound refuses",
            box_x,
            [(f64::NAN, 0.5), (0.25, 1.5), (0.1, 0.9)],
            det,
        ),
        (
            "a zero-containing determinant refuses",
            box_x,
            k_x,
            (-1.0, 1.0),
        ),
        (
            "a determinant touching zero from below refuses",
            box_x,
            k_x,
            (-1.0, 0.0),
        ),
        (
            "a determinant touching zero from above refuses",
            box_x,
            k_x,
            (0.0, 1.0),
        ),
        (
            "a degenerate zero determinant refuses",
            box_x,
            k_x,
            (0.0, 0.0),
        ),
        ("a misordered determinant refuses", box_x, k_x, (3.0, 2.0)),
    ];
    for (what, b, k, d) in cases {
        assert_eq!(
            KrawczykCertificate3::new(b, k, d),
            Err(Refusal::InvalidInput),
            "{what}"
        );
    }

    // A strictly-negative determinant is a valid orientation (0 excluded).
    let negative = construct(KrawczykCertificate3::new(box_x, k_x, (-3.0, -2.0)));
    assert_eq!(negative.det(), (-3.0, -2.0));
}

#[test]
fn trace_step_carries_box_germ_and_certificates() {
    let chart_box = [(0.2, 0.6), (0.3, 0.5), (0.2, 0.6), (0.3, 0.5)];
    let incidence = fx::sample_trace_incidence();
    let coordinate = ContinuationCoordinate {
        index: 2,
        relative_margin: construct(IntervalEnclosure::new(0.5, 1.0)),
    };
    let step = construct(TraceStep::new(
        chart_box,
        BranchGerm::Regular,
        incidence,
        coordinate,
    ));
    assert_eq!(step.chart_box(), chart_box);
    assert_eq!(step.germ(), BranchGerm::Regular);
    assert_eq!(step.incidence(), incidence);
    assert_eq!(step.coordinate(), coordinate);
    assert_eq!(step.coordinate().index, 2);
    assert_eq!(step.coordinate().relative_margin.lower().get(), 0.5);
    assert_eq!(step.coordinate().relative_margin.upper().get(), 1.0);

    // Construction from the fixture's own sample round-trips too.
    let sample = construct(fx::sample_trace_step());
    assert_eq!(sample.chart_box(), chart_box);
    assert_eq!(sample.germ(), BranchGerm::Regular);

    // A stationary germ round-trips as carried.
    let stationary = construct(TraceStep::new(
        chart_box,
        BranchGerm::StationaryRegular {
            first_nonzero_order: 2,
        },
        incidence,
        coordinate,
    ));
    assert_eq!(
        stationary.germ(),
        BranchGerm::StationaryRegular {
            first_nonzero_order: 2
        }
    );

    let invalid_boxes = [
        [(f64::NAN, 0.6), (0.3, 0.5), (0.2, 0.6), (0.3, 0.5)],
        [(0.6, 0.2), (0.3, 0.5), (0.2, 0.6), (0.3, 0.5)],
        [(0.2, f64::INFINITY), (0.3, 0.5), (0.2, 0.6), (0.3, 0.5)],
    ];
    for bad in invalid_boxes {
        assert_eq!(
            TraceStep::new(bad, BranchGerm::Regular, incidence, coordinate),
            Err(Refusal::InvalidInput)
        );
    }
}

#[test]
fn trace_outcome_refusals_are_named_cases() {
    fn refusal_class(refusal: TraceRefusal) -> &'static str {
        match refusal {
            TraceRefusal::Conditioning(_) => "conditioning",
            TraceRefusal::Hull(_) => "hull",
            TraceRefusal::Unresolved(_) => "unresolved",
        }
    }

    let conditioning = TraceRefusal::Conditioning(Refusal::ConditioningBelowThreshold);
    let hull = TraceRefusal::Hull(HullRefusal::EnclosureUnavailable);
    let unresolved = TraceRefusal::Unresolved(GenericUnresolved::SingularJacobian);

    assert_eq!(refusal_class(conditioning), "conditioning");
    assert_eq!(
        conditioning,
        TraceRefusal::Conditioning(Refusal::ConditioningBelowThreshold)
    );
    assert_eq!(refusal_class(hull), "hull");
    assert_eq!(hull, TraceRefusal::Hull(HullRefusal::EnclosureUnavailable));
    assert_eq!(refusal_class(unresolved), "unresolved");
    assert_eq!(
        unresolved,
        TraceRefusal::Unresolved(GenericUnresolved::SingularJacobian)
    );

    // No catch-all on the outcome either.
    fn outcome_class(outcome: &TraceOutcome) -> &'static str {
        match outcome {
            TraceOutcome::ClosedLoop { .. } => "closed_loop",
            TraceOutcome::Terminated { .. } => "terminated",
            TraceOutcome::Switched { .. } => "switched",
            TraceOutcome::Refused(_) => "refused",
        }
    }

    let step = construct(fx::sample_trace_step());
    assert_eq!(
        outcome_class(&TraceOutcome::ClosedLoop { steps: vec![step] }),
        "closed_loop"
    );
    assert_eq!(
        outcome_class(&TraceOutcome::Terminated { steps: vec![step] }),
        "terminated"
    );
    let switch = CoordinateSwitch {
        outgoing: ContinuationCoordinate {
            index: 0,
            relative_margin: construct(IntervalEnclosure::new(0.25, 0.5)),
        },
        incoming: ContinuationCoordinate {
            index: 2,
            relative_margin: construct(IntervalEnclosure::new(0.5, 0.75)),
        },
    };
    assert_eq!(
        outcome_class(&TraceOutcome::Switched {
            steps: vec![step],
            switch,
        }),
        "switched"
    );
    assert_eq!(
        outcome_class(&TraceOutcome::Refused(conditioning)),
        "refused"
    );
}

#[test]
fn fixture_well_conditioned_root_matches_ground_truth() {
    let fixture = construct(fx::well_conditioned_root());
    let system = &fixture.system;
    assert_eq!(system.degrees(), (1, 1, 1, 1));
    assert_eq!(
        system.domain_maps(),
        (0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0)
    );
    assert_eq!(fixture.continuation_axis, 2);
    assert_eq!(fixture.reduced_determinant, 1.0);

    // The root lies on the branch: F(root) == 0 by direct evaluation.
    for value in eval_sys(system, fixture.root) {
        assert_approx(value, 0.0, "F at the root");
    }

    // The reduced square system (unknowns u, v, t at the s-slice) has one
    // transverse root: crossing each reduced unknown through the root flips
    // the corresponding F components.
    let (u, v, s, t) = fixture.root;
    let delta = 0.05;

    let u_plus = eval_sys(system, (u + delta, v, s, t));
    let u_minus = eval_sys(system, (u - delta, v, s, t));
    assert_approx(u_plus[0], delta, "F_x across the u direction");
    assert_approx(
        u_minus[0],
        -delta,
        "F_x across the u direction (other side)",
    );
    assert_approx(u_plus[1], 0.0, "F_y constant across u");
    assert_approx(u_plus[2], 0.0, "F_z constant across u");

    let v_plus = eval_sys(system, (u, v + delta, s, t));
    let v_minus = eval_sys(system, (u, v - delta, s, t));
    assert_approx(v_plus[1], delta, "F_y across the v direction");
    assert_approx(
        v_minus[1],
        -delta,
        "F_y across the v direction (other side)",
    );
    assert_approx(v_plus[2], delta, "F_z across the v direction");
    assert_approx(
        v_minus[2],
        -delta,
        "F_z across the v direction (other side)",
    );

    let t_plus = eval_sys(system, (u, v, s, t + delta));
    let t_minus = eval_sys(system, (u, v, s, t - delta));
    assert_approx(t_plus[1], -delta, "F_y across the t direction");
    assert_approx(t_minus[1], delta, "F_y across the t direction (other side)");

    // The whole branch is the single transverse line, machine-checked along it.
    for uu in [0.3, 0.5, 0.7] {
        let vv = 0.25 + 0.5 * uu;
        for value in eval_sys(system, (uu, vv, uu, vv)) {
            assert_approx(value, 0.0, "F on the documented branch");
        }
    }

    // Orientation ground truth: the reduced determinant is +1 at the root.
    assert_approx(
        reduced_det(system, fixture.continuation_axis, fixture.root),
        fixture.reduced_determinant,
        "well-conditioned reduced determinant",
    );

    // An off-branch point at the slice is not a root.
    let off = eval_sys(system, (0.5, 0.3, 0.5, 0.3));
    assert!(
        off.iter().any(|value| !approx(*value, 0.0)),
        "off-branch point is not a root"
    );
}

#[test]
fn fixture_negative_orientation_flips_the_reduced_determinant_sign() {
    let positive = construct(fx::well_conditioned_root());
    let negative = construct(fx::negative_orientation_root());
    assert_eq!(positive.reduced_determinant, 1.0);
    assert_eq!(negative.reduced_determinant, -1.0);

    let system = &negative.system;
    for value in eval_sys(system, negative.root) {
        assert_approx(value, 0.0, "F at the flipped-orientation root");
    }
    assert_approx(
        reduced_det(system, negative.continuation_axis, negative.root),
        negative.reduced_determinant,
        "negative reduced determinant",
    );
}

#[test]
fn fixture_determinant_spans_zero_is_constructible() {
    let fixture = construct(fx::determinant_spans_zero());
    let system = &fixture.system;

    // The witness is a root on the (2D diagonal) zero set.
    for value in eval_sys(system, fixture.witness) {
        assert_approx(value, 0.0, "F at the diagonal witness");
    }

    // Every reduced determinant vanishes at the witness, so any sound
    // enclosure over the box contains zero and the certificate must refuse.
    for axis in 0..4 {
        assert_approx(
            reduced_det(system, axis, fixture.witness),
            0.0,
            "reduced determinant spans zero at the witness",
        );
    }

    // Off the diagonal the determinant is genuinely nonzero (a real system,
    // not a flat zero matrix).
    let off_diagonal = (0.6, 0.5, 0.4, 0.5);
    assert_approx(
        reduced_det(system, 3, off_diagonal),
        0.4,
        "off-diagonal reduced determinant",
    );
    let off_root = eval_sys(system, off_diagonal);
    assert!(
        off_root.iter().any(|value| !approx(*value, 0.0)),
        "off-diagonal point is not a root"
    );

    // The certificate MUST refuse a det enclosure containing zero over the
    // fixture box: the box straddles the diagonal (witness interior), so a
    // determinant enclosure for any reduced system contains zero.
    assert_eq!(
        KrawczykCertificate3::new(
            [(0.3, 0.7), (0.3, 0.7), (0.3, 0.7)],
            [(0.4, 0.6), (0.4, 0.6), (0.4, 0.6)],
            (-0.1, 0.1),
        ),
        Err(Refusal::InvalidInput),
        "the determinant-spans-zero fixture forces a certificate refusal"
    );
}

#[test]
fn fixture_germ_ladder_covers_all_branch_germ_variants() {
    let ladder = construct(fx::germ_ladder());
    assert_eq!(ladder.len(), 5, "one fixture per BranchGerm variant");

    for fixture in &ladder {
        let system = &fixture.system;
        let z_grid = &system.grids()[2];
        let degrees = system.degrees();
        let diagonal = |u: f64, v: f64| (u, v, u, v);

        match fixture.germ {
            BranchGerm::Regular => {
                assert!(fixture.event_is_interior(), "regular event is interior");
                assert_approx(eval_z(z_grid, degrees, fixture.event), 0.0, "on-branch");
                assert_approx(
                    partial_z(z_grid, degrees, 0, fixture.event),
                    -0.5,
                    "regular branch slope (h_u = -1/2)",
                );
            }
            BranchGerm::StationaryRegular {
                first_nonzero_order,
            } => {
                assert_eq!(first_nonzero_order, 2);
                assert!(fixture.event_is_interior(), "stationary event is interior");
                assert_approx(eval_z(z_grid, degrees, fixture.event), 0.0, "on-branch");
                assert_approx(
                    partial_z(z_grid, degrees, 0, fixture.event),
                    0.0,
                    "stationary: h_u = 0",
                );
                assert_approx(
                    partial_z(z_grid, degrees, 1, fixture.event),
                    1.0,
                    "stationary: h_v = 1",
                );
                assert_approx(
                    partial2_z(z_grid, degrees, 0, fixture.event),
                    -2.0,
                    "stationary order 2: h_uu = -2 (q'' = 2)",
                );
            }
            BranchGerm::CuspCandidate => {
                assert!(fixture.event_is_interior(), "cusp event is interior");
                assert_approx(eval_z(z_grid, degrees, fixture.event), 0.0, "on-branch");
                assert_approx(
                    partial_z(z_grid, degrees, 0, fixture.event),
                    0.0,
                    "cusp: h_u = 0 at the event",
                );
                assert_approx(
                    partial_z(z_grid, degrees, 1, fixture.event),
                    0.0,
                    "cusp: h_v = 0 at the event",
                );
                // Two half-branches meet at the event (the cuspidal curve
                // (tau^2, tau^3) about (1/4, 1/2)); sample one on each side.
                let (u0, v0) = (0.25, 0.5);
                for tau in [-0.5, 0.5] {
                    let point = diagonal(u0 + tau * tau, v0 + tau * tau * tau);
                    for value in eval_sys(system, point) {
                        assert_approx(value, 0.0, "F on the cusp half-branch");
                    }
                }
            }
            BranchGerm::Singular => {
                assert!(fixture.event_is_interior(), "singular event is interior");
                // The zero set is a 2D diagonal (coincident patches): every
                // sampled diagonal point is a root.
                for (uu, vv) in [(0.4, 0.6), (0.6, 0.3), (0.2, 0.7), (0.7, 0.2)] {
                    for value in eval_sys(system, diagonal(uu, vv)) {
                        assert_approx(value, 0.0, "F on the 2D diagonal zero set");
                    }
                }
                // Local topology is not that of a regular branch: every
                // reduced determinant vanishes at the event.
                for axis in 0..4 {
                    assert_approx(
                        reduced_det(system, axis, fixture.event),
                        0.0,
                        "singular reduced determinant",
                    );
                }
            }
            BranchGerm::Unresolved => {
                // The event lies exactly on the documented box's lower-u face:
                // no endpoint germ certificate is implemented at the declared
                // policy, so the germ classification is Unresolved.
                assert!(
                    !fixture.event_is_interior(),
                    "unresolved event is on the box boundary"
                );
                assert_eq!(fixture.event.0, fixture.chart_box[0].0);
                assert_eq!(fixture.event.2, fixture.chart_box[2].0);
                for value in eval_sys(system, fixture.event) {
                    assert_approx(value, 0.0, "F at the boundary event");
                }
            }
        }
    }

    let tags: Vec<&'static str> = ladder.iter().map(|f| f.germ.tag()).collect();
    assert_eq!(
        tags,
        vec![
            "germ_regular",
            "germ_stationary_regular",
            "germ_cusp_candidate",
            "germ_singular",
            "germ_unresolved",
        ]
    );
}

#[test]
fn fixture_conditioning_below_threshold_refuses_every_coordinate() {
    let fixture = construct(fx::conditioning_below_threshold());
    let system = &fixture.system;

    // A genuine branch passes through the box.
    for value in eval_sys(system, fixture.root) {
        assert_approx(value, 0.0, "F at the interior root");
    }

    // For every candidate continuation axis, the reduced square system's
    // identity-paired diagonal derivatives are identically zero over the box,
    // so the frozen relative-margin rule cannot certify any coordinate and the
    // trace must refuse ConditioningBelowThreshold.
    for axis in 0..4 {
        for point in [(0.5, 0.5, 0.5, 0.5), (0.25, 0.75, 0.25, 0.75)] {
            for entry in reduced_diag(system, axis, point) {
                assert_approx(entry, 0.0, "coordinate margin derivative is zero");
            }
        }
    }
}

#[test]
fn fixture_closed_loop_pair_seeds_share_the_same_loop() {
    let fixture = construct(fx::closed_loop_pair());
    let system = &fixture.system;

    // Both seeds are roots on the same closed branch.
    for seed in [fixture.first_seed, fixture.second_seed] {
        for value in eval_sys(system, seed) {
            assert_approx(value, 0.0, "F at a closed-loop seed");
        }
    }

    // The two seeds sit on the documented circle, as diagonal quadruples.
    for seed in [fixture.first_seed, fixture.second_seed] {
        let (u, v, s, t) = seed;
        assert_eq!(u, s, "loop seeds are diagonal-chart quadruples");
        assert_eq!(v, t, "loop seeds are diagonal-chart quadruples");
        let radial = ((u - fixture.center.0).powi(2) + (v - fixture.center.1).powi(2)).sqrt();
        assert_approx(radial, fixture.radius, "seed radius");
    }

    // The whole circle is the branch: sampling the parametrized loop, every
    // point is a root (the zero set is one closed curve, not two isolated
    // roots), so a trace from either seed closes on itself.
    let (cx, cy) = fixture.center;
    for step in 0..24 {
        let theta = 2.0 * std::f64::consts::PI * step as f64 / 24.0;
        let u = cx + fixture.radius * theta.cos();
        let v = cy + fixture.radius * theta.sin();
        assert!(
            u > 0.0 && u < 1.0 && v > 0.0 && v < 1.0,
            "the loop stays interior"
        );
        for value in eval_sys(system, (u, v, u, v)) {
            assert_approx(value, 0.0, "F on the sampled closed loop");
        }
    }
}

#[test]
fn shim_never_implements_a_solver() {
    let types_source = include_str!("../src/ssi_types.rs");
    let fixtures_source = include_str!("../src/ssi_fixtures.rs");

    assert!(
        !types_source.contains("hull_bernstein"),
        "ssi_types.rs must not call the hull kernels (the solvers own those)"
    );
    assert!(
        !types_source.contains("CertifiedInterval"),
        "ssi_types.rs must carry no CertifiedInterval arithmetic chains"
    );
    assert!(
        !fixtures_source.contains("hull_bernstein"),
        "ssi_fixtures.rs must not call the hull kernels (the solvers own those)"
    );
    assert!(
        !fixtures_source.contains("CertifiedInterval"),
        "ssi_fixtures.rs must carry no CertifiedInterval arithmetic"
    );
}
