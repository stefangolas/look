//! Integration tests for the SSI square-system engine (BG-CK-P2-SYSTEM +
//! KRAWCZYK3): the square-system constructor from two certified-admitted
//! rational Bézier patches, the F3 square reduction against the frozen
//! coordinate rule, and the 3×3 Krawczyk certificate over the fixture kit's
//! documented ground truths. The eight test names are the contract.

#![deny(clippy::unwrap_used)]

use truck_certified::contract::{
    select_continuation_coordinate as frozen_select, IntervalEnclosure, Refusal, SquareSystemInput,
};
use truck_certified::formal::intersection::PairUnsupported;
use truck_certified::formal::numeric::PositiveFinite;
use truck_certified::ssi::{
    construct_square_system, f3_diagonal_derivatives, krawczyk3_certificate,
    select_continuation_coordinate, RationalBipatch, SsiParticipant, SsiRefusal,
};
use truck_certified::ssi_fixtures as fx;
use truck_certified::ssi_types::SquareSystem3;
use truck_certified::KrawczykCertificate3;

/// Assert two floats agree to the fixture kit's dyadic ground-truth tolerance.
fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9 // H-3: dyadic ground-truth comparison tolerance, fixture-only
}

/// A unit-weight degree-(1,1) rational patch built from its three component
/// control grids (rows index the first parameter).
fn plane_grids(x: Vec<Vec<f64>>, y: Vec<Vec<f64>>, z: Vec<Vec<f64>>) -> RationalBipatch {
    let w = vec![vec![1.0, 1.0], vec![1.0, 1.0]];
    match RationalBipatch::new(1, 1, [x, y, z], w) {
        Ok(p) => p,
        Err(_) => panic!("a valid unit-weight patch was refused"),
    }
}

/// The two documented planes of the well-conditioned fixture pair: patch 1 =
/// `(u, v, v)`, patch 2 = `(s, t, 1/4 + s/2)`.
fn well_conditioned_patches() -> (SsiParticipant, SsiParticipant) {
    let p1 = plane_grids(
        vec![vec![0.0, 0.0], vec![1.0, 1.0]],
        vec![vec![0.0, 1.0], vec![0.0, 1.0]],
        vec![vec![0.0, 1.0], vec![0.0, 1.0]],
    );
    let p2 = plane_grids(
        vec![vec![0.0, 0.0], vec![1.0, 1.0]],
        vec![vec![0.0, 1.0], vec![0.0, 1.0]],
        vec![vec![0.25, 0.25], vec![0.75, 0.75]],
    );
    (
        SsiParticipant::RationalBipatch(p1),
        SsiParticipant::RationalBipatch(p2),
    )
}

/// A small root-centred box in the unit chart.
const ROOT_BOX: [(f64, f64); 4] = [(0.4, 0.6), (0.4, 0.6), (0.4, 0.6), (0.4, 0.6)];

/// An asymmetric root-box for coordinate selection: the retained `u` extent is
/// far smaller than the retained `v` extent, so the coordinate-0 margin
/// (`|1| / extent_u`) dominates deterministically over coordinate 1.
const SELECTION_BOX: [(f64, f64); 4] = [(0.45, 0.55), (0.25, 0.75), (0.45, 0.55), (0.45, 0.55)];

#[test]
fn system3_constructor_matches_fixture_ground_truth() {
    let fixture = match fx::well_conditioned_root() {
        Ok(f) => f,
        Err(_) => panic!("fixture refused"),
    };
    let (p1, p2) = well_conditioned_patches();
    let system = match construct_square_system(&p1, &p2) {
        Ok(s) => s,
        Err(e) => panic!("constructor refused a spline-admissible pair: {}", e.tag()),
    };

    // The stored grids round-trip: feeding them back through the shim's
    // refusing constructor reproduces the system, and they equal the fixture
    // kit's own cross-multiplied system for the same two planes.
    let round_trip = match SquareSystem3::new(
        system.grids().clone(),
        system.degrees(),
        system.domain_maps(),
    ) {
        Ok(s) => s,
        Err(_) => panic!("stored grids refused on round trip"),
    };
    assert_eq!(round_trip, system);
    assert_eq!(
        system, fixture.system,
        "constructor reproduces the fixture system"
    );

    // Cross-multiplied F_k evaluate to the stated ground truth at the root and
    // its neighbours.
    let root = fixture.root;
    let vals = match fx::eval_system(&system, root) {
        Some(v) => v,
        None => panic!("eval failed"),
    };
    for v in vals {
        assert!(approx(v, 0.0), "F(root) = 0");
    }
    let (u, v, s, t) = root;
    let d = 0.05;
    let u_plus = match fx::eval_system(&system, (u + d, v, s, t)) {
        Some(x) => x,
        None => panic!("eval failed"),
    };
    assert!(approx(u_plus[0], d), "F_x across u");
    assert!(
        approx(u_plus[1], 0.0) && approx(u_plus[2], 0.0),
        "u crosses only F_x"
    );
    let t_plus = match fx::eval_system(&system, (u, v, s, t + d)) {
        Some(x) => x,
        None => panic!("eval failed"),
    };
    assert!(approx(t_plus[1], -d), "F_y across t");
    assert!(
        approx(t_plus[0], 0.0) && approx(t_plus[2], 0.0),
        "t crosses only F_y"
    );
}

#[test]
fn system3_refuses_non_spline_class_pairs() {
    let (p1, _) = well_conditioned_patches();
    let other = SsiParticipant::NonSpline;
    // A pair with any non-spline side refuses the DISPATCH widening variant.
    match construct_square_system(&other, &p1) {
        Err(SsiRefusal::PairClass(PairUnsupported::UnsupportedPairClass)) => {}
        Err(e) => panic!("wrong refusal: {}", e.tag()),
        Ok(_) => panic!("a non-spline pair must refuse"),
    }
    match construct_square_system(&p1, &other) {
        Err(SsiRefusal::PairClass(PairUnsupported::UnsupportedPairClass)) => {}
        Err(e) => panic!("wrong refusal: {}", e.tag()),
        Ok(_) => panic!("a non-spline pair must refuse"),
    }
    match construct_square_system(&other, &other) {
        Err(SsiRefusal::PairClass(PairUnsupported::UnsupportedPairClass)) => {}
        Err(e) => panic!("wrong refusal: {}", e.tag()),
        Ok(_) => panic!("a non-spline pair must refuse"),
    }
}

/// Run the frozen coordinate rule over a degenerate input built from the
/// fixture's directly evaluated identity-paired diagonal entries at a point and
/// the box extents. This is the oracle the certified enclosures must reproduce.
fn frozen_answer_on_fixture_diagonals(
    system: &SquareSystem3,
    continuation_axis: usize,
    box_: [(f64, f64); 4],
    point: (f64, f64, f64, f64),
) -> Result<usize, Refusal> {
    let diag = match fx::reduced_diagonal_entries(system, continuation_axis, point) {
        Some(d) => d,
        None => return Err(Refusal::InvalidInput),
    };
    let retained: Vec<usize> = (0..4).filter(|a| *a != continuation_axis).collect();
    let mut diagonal = Vec::with_capacity(3);
    for i in 0..3 {
        diagonal.push(IntervalEnclosure::new(diag[i], diag[i]).map_err(|_| Refusal::InvalidInput)?);
    }
    let mut extents = Vec::with_capacity(3);
    for i in 0..3 {
        let axis = retained[i];
        extents.push(
            PositiveFinite::new(box_[axis].1 - box_[axis].0).map_err(|_| Refusal::InvalidInput)?,
        );
    }
    let input = SquareSystemInput {
        diagonal_derivatives: [diagonal[0], diagonal[1], diagonal[2]],
        extents: [extents[0], extents[1], extents[2]],
    };
    frozen_select(&input).map(|c| c.index)
}

#[test]
fn coordinate_selection_follows_frozen_rule() {
    let fixture = match fx::well_conditioned_root() {
        Ok(f) => f,
        Err(_) => panic!("fixture refused"),
    };
    let system = &fixture.system;
    let continuation_axis = fixture.continuation_axis;
    let centre = (0.5, 0.5, 0.5, 0.5);

    // The certified input's enclosures contain the fixture's point diagonals
    // at the box centre.
    let certified = match f3_diagonal_derivatives(system, continuation_axis, SELECTION_BOX) {
        Ok(input) => input,
        Err(_) => panic!("certified F3 input refused on the fixture box"),
    };
    let point_diag = match fx::reduced_diagonal_entries(system, continuation_axis, centre) {
        Some(d) => d,
        None => panic!("fixture diagonal unavailable"),
    };
    for (i, value) in point_diag.iter().enumerate() {
        let enc = certified.diagonal_derivatives[i];
        assert!(
            enc.lower().get() <= *value && *value <= enc.upper().get(),
            "coordinate {i}: certified enclosure contains the fixture point value"
        );
    }

    // The module selection is exactly the frozen rule over the certified
    // input, and it reproduces the frozen rule's answer on the fixture point
    // diagonals (the certified enclosures certify the same coordinate).
    let module_selection =
        match select_continuation_coordinate(system, continuation_axis, SELECTION_BOX) {
            Ok(c) => c,
            Err(_) => panic!("selection refused on the fixture box"),
        };
    let oracle_index = match frozen_answer_on_fixture_diagonals(
        system,
        continuation_axis,
        SELECTION_BOX,
        centre,
    ) {
        Ok(i) => i,
        Err(_) => panic!("oracle refused"),
    };
    assert_eq!(module_selection.index, oracle_index, "frozen rule's answer");
    let frozen = match frozen_select(&certified) {
        Ok(c) => c,
        Err(_) => panic!("frozen select refused on the certified input"),
    };
    assert_eq!(module_selection, frozen, "the frozen rule applied verbatim");

    // Conditioning fixture: every coordinate margin fails for every candidate
    // continuation axis, so the box refuses ConditioningBelowThreshold.
    let cond = match fx::conditioning_below_threshold() {
        Ok(c) => c,
        Err(_) => panic!("fixture refused"),
    };
    for axis in 0..4 {
        match select_continuation_coordinate(&cond.system, axis, cond.box_) {
            Err(SsiRefusal::Conditioning(Refusal::ConditioningBelowThreshold)) => {}
            Err(e) => panic!("axis {axis} wrong refusal: {}", e.tag()),
            Ok(_) => panic!("axis {axis} must refuse ConditioningBelowThreshold"),
        }
    }
}

#[test]
fn krawczyk3_certifies_fixture_well_conditioned_root() {
    let fixture = match fx::well_conditioned_root() {
        Ok(f) => f,
        Err(_) => panic!("fixture refused"),
    };
    let system = &fixture.system;
    let axis = fixture.continuation_axis;

    let cert = match krawczyk3_certificate(system, axis, ROOT_BOX) {
        Ok(c) => c,
        Err(e) => panic!("well-conditioned root must certify: {}", e.tag()),
    };
    // Retained axes for continuation axis 2 are (u, v, t), ascending.
    let retained: Vec<usize> = (0..4).filter(|a| *a != axis).collect();
    let x_box = [
        ROOT_BOX[retained[0]],
        ROOT_BOX[retained[1]],
        ROOT_BOX[retained[2]],
    ];
    assert_eq!(cert.box_x(), x_box, "the box X is the retained box");
    // K(X) strictly inside X.
    for (b, k) in cert.box_x().iter().zip(cert.k_x().iter()) {
        assert!(b.0 < k.0 && k.1 < b.1, "K(X) strictly inside X");
    }
    // Determinant enclosure consistent with the fixture's reduced determinant
    // (+1 at the root): away from zero, positive, containing 1.
    let (d_lo, d_hi) = cert.det();
    assert!(d_lo > 0.0, "determinant away from zero, positive");
    assert!(d_lo <= fixture.reduced_determinant && fixture.reduced_determinant <= d_hi);
}

#[test]
fn krawczyk3_refuses_determinant_spans_zero_fixture() {
    let fixture = match fx::determinant_spans_zero() {
        Ok(f) => f,
        Err(_) => panic!("fixture refused"),
    };
    let system = &fixture.system;
    // The witness is interior to the fixture box and every reduced determinant
    // is exactly zero there, so a sound det enclosure over the retained box
    // contains zero and the certificate refuses at construction.
    for axis in 2..=3 {
        match krawczyk3_certificate(system, axis, fixture.box_) {
            Err(SsiRefusal::DeterminantSpansZero) => {}
            Err(e) => panic!("axis {axis} wrong refusal: {}", e.tag()),
            Ok(_) => panic!("axis {axis} must refuse: determinant spans zero"),
        }
    }
}

#[test]
fn krawczyk3_refuses_conditioning_fixture() {
    let fixture = match fx::conditioning_below_threshold() {
        Ok(f) => f,
        Err(_) => panic!("fixture refused"),
    };
    let system = &fixture.system;
    // Every coordinate margin fails over the fixture box, so the certificate
    // refuses through the frozen coordinate rule before any Krawczyk work.
    for axis in 0..4 {
        match krawczyk3_certificate(system, axis, fixture.box_) {
            Err(SsiRefusal::Conditioning(Refusal::ConditioningBelowThreshold)) => {}
            Err(e) => panic!("axis {axis} wrong refusal: {}", e.tag()),
            Ok(_) => panic!("axis {axis} must refuse ConditioningBelowThreshold"),
        }
    }
}

#[test]
fn krawczyk3_negative_orientation_fixture_flips_det_sign() {
    let neg = match fx::negative_orientation_root() {
        Ok(f) => f,
        Err(_) => panic!("fixture refused"),
    };
    let system = &neg.system;
    assert!(
        neg.reduced_determinant < 0.0,
        "fixture states a negative reduced determinant for the flipped orientation"
    );
    // The flipped parameter order flips the identity-paired diagonal margins,
    // so the certifying axes differ from the positive orientation; scan the
    // candidate continuation axes and require that a certificate constructs on
    // the branch whose determinant sign is flipped (strictly negative,
    // consistent with the fixture's stated reduced determinant).
    let mut flipped_certified = false;
    let mut certifying_axes = Vec::new();
    for axis in 0..4 {
        match krawczyk3_certificate(system, axis, ROOT_BOX) {
            Ok(cert) => {
                certifying_axes.push((axis, cert.det()));
                let (d_lo, d_hi) = cert.det();
                assert!(d_hi < 0.0 || d_lo > 0.0, "axis {axis}: det excludes zero");
                for (b, k) in cert.box_x().iter().zip(cert.k_x().iter()) {
                    assert!(b.0 < k.0 && k.1 < b.1, "K(X) strictly inside X");
                }
                if d_hi < 0.0 {
                    flipped_certified = true;
                }
            }
            Err(SsiRefusal::Conditioning(_)) => {
                // Not a certifiable axis for this orientation; a flipped one
                // must exist below.
            }
            Err(_) => {}
        }
    }
    assert!(
        !certifying_axes.is_empty(),
        "negative orientation root must certify on some axis"
    );
    assert!(
        flipped_certified,
        "a certifying axis must carry the flipped (negative) determinant sign; got {certifying_axes:?}"
    );
}

#[test]
fn krawczyk3_strict_inclusion_only() {
    // A hand-built non-strict inclusion (K(X) touching the boundary) refuses
    // through the shim's strict-inclusion-only constructor. The boundary case
    // is constructed here, never found by search.
    let box_x = [(0.0, 1.0), (0.0, 1.0), (0.0, 1.0)];
    let k_touching_lower = [(0.0, 1.0), (0.2, 0.8), (0.2, 0.8)];
    let det = (2.0, 3.0);
    assert!(matches!(
        KrawczykCertificate3::new(box_x, k_touching_lower, det),
        Err(Refusal::InvalidInput)
    ));
    let k_touching_upper = [(0.0, 1.0), (0.2, 1.0), (0.2, 0.8)];
    assert!(matches!(
        KrawczykCertificate3::new(box_x, k_touching_upper, det),
        Err(Refusal::InvalidInput)
    ));

    // A genuinely strict inclusion is accepted.
    let k_strict = [(0.1, 0.9), (0.2, 0.8), (0.2, 0.8)];
    assert!(KrawczykCertificate3::new(box_x, k_strict, det).is_ok());
}
