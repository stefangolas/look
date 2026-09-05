//! CC-013-CORRESPONDENCE integration tests (theory §2.2 L4, spine seam S9):
//! wire production over the S9 `WireComplex` and the fixed-order cyclic
//! correspondence resolution — caller anchor → combinatorially forced unique
//! isomorphism (r = 2) → P4 separation-margin argmin over the r cyclic shifts
//! of the declared `VertexSumSq` functional → refuse. Twist minimization is
//! not an objective anywhere.

#![deny(clippy::unwrap_used)]

use truck_certified::construct::argmin::argmin_margin;
use truck_certified::construct::correspondence::{resolve_correspondence, wire_complex_of};
use truck_certified::construct::refusal::ConstructRefusal;
use truck_certified::construct::stubs::{
    Correspondence, ShiftAnchor, ShiftFunctional, ShiftFunctionalKind, WireComplex,
};
use truck_certified::construct::Interval;

/// A certified point interval from explicit x/y coordinates (z = 0).
fn pt(x: f64, y: f64) -> [Interval; 3] {
    [Interval::point(x), Interval::point(y), Interval::point(0.0)]
}

/// Build a test wire from a 2D point list through the production path.
fn wire_of(points: &[(f64, f64)]) -> WireComplex {
    let vertices: Vec<[Interval; 3]> = points.iter().map(|&(x, y)| pt(x, y)).collect();
    match wire_complex_of(vertices.len(), &vertices) {
        Ok(wire) => wire,
        Err(refusal) => panic!("a test fixture wire must build, refused: {refusal:?}"),
    }
}

/// The cyclic left rotation of a point list by `k` positions.
fn rotate(points: &[(f64, f64)], k: usize) -> Vec<(f64, f64)> {
    let n = points.len();
    (0..n).map(|j| points[(j + k) % n]).collect()
}

/// The reversal of a point list (an orientation-reversing section).
fn reverse(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    points.iter().rev().copied().collect()
}

/// The declared v1 functional with no anchor (`VertexSumSq`).
fn vertex_sum_sq() -> ShiftFunctional {
    ShiftFunctional {
        kind: ShiftFunctionalKind::VertexSumSq,
        anchor: None,
    }
}

/// The declared v1 functional with a caller-supplied anchor.
fn anchored(index: usize, reversed: bool) -> ShiftFunctional {
    ShiftFunctional {
        kind: ShiftFunctionalKind::VertexSumSq,
        anchor: Some(ShiftAnchor { index, reversed }),
    }
}

/// Expect the resolver's `Ok` correspondence to equal the ground truth.
fn expect_correspondence(
    result: Result<Correspondence, ConstructRefusal>,
    expected: Correspondence,
) {
    match result {
        Ok(correspondence) => assert_eq!(correspondence, expected),
        Err(refusal) => panic!("expected a correspondence, refused: {refusal:?}"),
    }
}

/// Expect a resolver/wire-production refusal of the exact typed outcome.
fn expect_refusal<T>(result: Result<T, ConstructRefusal>, expected: ConstructRefusal) {
    match result {
        Ok(_) => panic!("expected refusal {expected:?}, got Ok"),
        Err(refusal) => assert_eq!(refusal, expected),
    }
}

/// The test-local re-derivation of the declared v1 functional (the shipped
/// evaluation's direct mirror over the fixture data — never a solve).
fn local_vertex_sum_sq(wire: &WireComplex, section: &WireComplex, shift: usize) -> Interval {
    let r = wire.arc_count;
    let mut total = Interval::point(0.0);
    for i in 0..r {
        let j = (i + shift) % r;
        let dx = wire.vertices[i][0].sub(&section.vertices[j][0]);
        let dy = wire.vertices[i][1].sub(&section.vertices[j][1]);
        let dz = wire.vertices[i][2].sub(&section.vertices[j][2]);
        let distance = dx.mul(&dx).add(&dy.mul(&dy)).add(&dz.mul(&dz));
        total = total.add(&distance);
    }
    total
}

/// Assert that the r shift enclosures separate with the expected argmin —
/// the mechanism CC-013 must certify, never a proximity pick.
fn assert_separated_shift(wire: &WireComplex, section: &WireComplex, expected: usize) {
    let r = wire.arc_count;
    let mut enclosures = Vec::with_capacity(r);
    for shift in 0..r {
        enclosures.push(local_vertex_sum_sq(wire, section, shift));
    }
    let winner = match argmin_margin(&enclosures) {
        Ok(index) => index,
        Err(refusal) => panic!("fixture enclosures must separate, refused: {refusal:?}"),
    };
    assert_eq!(winner, expected, "the separated shift is the ground truth");
    for (j, enclosure) in enclosures.iter().enumerate() {
        if j != winner {
            assert!(
                enclosures[winner].hi < enclosure.lo, // H-3: strict sup<inf separation margin
                "shift {winner} must be strictly separated from shift {j}"
            );
        }
    }
}

#[test]
fn anchor_supplied_wins() {
    // A 3x1 rectangle reference wire; the sections are cyclic re-indexings.
    let points = [(0.0, 0.0), (3.0, 0.0), (3.0, 1.0), (0.0, 1.0)];
    let wire = wire_of(&points);
    let sec_a = wire_of(&rotate(&points, 1));
    let sec_b = wire_of(&rotate(&points, 2));

    // Control: with no anchor the geometric functional unambiguously resolves
    // sec_a to shift 3 (the section is the wire re-indexed by one).
    let plain = resolve_correspondence(&wire, &[sec_a.clone()], &vertex_sum_sq());
    expect_correspondence(
        plain,
        Correspondence {
            orientation: true,
            anchor: None,
            shifts: vec![3],
        },
    );

    // The anchor pins shift 1 for EVERY section and is never second-guessed,
    // even though the unanchored functional would pick 3 (sec_a) and 2 (sec_b).
    let anchored_result = resolve_correspondence(&wire, &[sec_a, sec_b], &anchored(1, false));
    expect_correspondence(
        anchored_result,
        Correspondence {
            orientation: true,
            anchor: Some(1),
            shifts: vec![1, 1],
        },
    );
}

#[test]
fn unique_isomorphism_resolves_without_argmin() {
    // r = 2 (a digon): the resolver must resolve in step 2 without invoking
    // the geometric argmin. The section is the wire re-indexed by one, so a
    // geometric functional would strictly separate shift 1 (value 0); the
    // index-preserving shift 0 the resolver returns proves the argmin was
    // never consulted.
    let points = [(0.0, 0.0), (1.0, 0.0)];
    let wire = wire_of(&points);
    let swapped = wire_of(&rotate(&points, 1));

    let result = resolve_correspondence(&wire, &[swapped], &vertex_sum_sq());
    expect_correspondence(
        result,
        Correspondence {
            orientation: true,
            anchor: None,
            shifts: vec![0],
        },
    );
}

#[test]
fn four_arc_two_circle_shift_resolves_by_separated_argmin() {
    // Two four-arc closed wires (a 2x1 rectangle reference and two cyclic
    // re-indexings of it). The r = 4 shift enclosures of each section are
    // strictly separated and the argmin resolves the unique shift: 3 for the
    // rotate-by-one section, 2 for the rotate-by-two section.
    let points = [(0.0, 0.0), (2.0, 0.0), (2.0, 1.0), (0.0, 1.0)];
    let wire = wire_of(&points);
    let sec_a = wire_of(&rotate(&points, 1));
    let sec_b = wire_of(&rotate(&points, 2));

    assert_separated_shift(&wire, &sec_a, 3);
    assert_separated_shift(&wire, &sec_b, 2);

    let result = resolve_correspondence(&wire, &[sec_a, sec_b], &vertex_sum_sq());
    expect_correspondence(
        result,
        Correspondence {
            orientation: true,
            anchor: None,
            shifts: vec![3, 2],
        },
    );
}

#[test]
fn overlapping_shift_values_refuse_ambiguous_correspondence() {
    // The four-fold ambiguous case: two four-arc closed wires whose vertex
    // patterns are fully 4-fold symmetric, so every cyclic shift yields the
    // same functional enclosure. The resolver must refuse
    // AmbiguousCorrespondence — never a proximity tie-break.
    let points = [(0.0, 0.0), (0.0, 0.0), (0.0, 0.0), (0.0, 0.0)];
    let wire = wire_of(&points);
    let section = wire_of(&[(1.0, 1.0); 4]);

    let result = resolve_correspondence(&wire, &[section], &vertex_sum_sq());
    expect_refusal(result, ConstructRefusal::AmbiguousCorrespondence);
}

#[test]
fn orientation_reversal_requires_explicit_caller_consent() {
    // The section is the wire traversed in reverse: a reversed matching (with
    // parameter 3, section vertex `3 - i`) would align every matched pair
    // perfectly (value 0). The automatic path is orientation-preserving only,
    // so it searches just the forward shifts — whose best is shift 0 at value
    // 4 — and NEVER takes the reversal; the reversal is taken only when the
    // caller supplied it explicitly in the anchor.
    let points = [(0.0, 0.0), (3.0, 0.0), (3.0, 1.0), (0.0, 1.0)];
    let wire = wire_of(&points);
    let rev = wire_of(&reverse(&points));

    let automatic = resolve_correspondence(&wire, &[rev.clone()], &vertex_sum_sq());
    expect_correspondence(
        automatic,
        Correspondence {
            orientation: true,
            anchor: None,
            shifts: vec![0],
        },
    );
    assert_separated_shift(&wire, &rev, 0);

    let reversed_consent = resolve_correspondence(&wire, &[rev.clone()], &anchored(3, true));
    expect_correspondence(
        reversed_consent,
        Correspondence {
            orientation: false,
            anchor: Some(3),
            shifts: vec![3],
        },
    );

    let forward_consent = resolve_correspondence(&wire, &[rev], &anchored(0, false));
    expect_correspondence(
        forward_consent,
        Correspondence {
            orientation: true,
            anchor: Some(0),
            shifts: vec![0],
        },
    );
}

#[test]
fn wire_production_and_resolution_refuse_malformed_input() {
    // arc_count below 2.
    let single = [pt(0.0, 0.0)];
    expect_refusal(wire_complex_of(1, &single), ConstructRefusal::InvalidInput);

    // Vertex count unequal to arc_count (a cycle needs exactly arc_count).
    let two = [pt(0.0, 0.0), pt(1.0, 0.0)];
    expect_refusal(wire_complex_of(3, &two), ConstructRefusal::InvalidInput);

    // A non-finite enclosure is not admissible vertex data.
    let non_finite = [
        [
            Interval::point(0.0),
            Interval::point(f64::NAN),
            Interval::point(0.0),
        ],
        [
            Interval::point(1.0),
            Interval::point(0.0),
            Interval::point(0.0),
        ],
    ];
    expect_refusal(
        wire_complex_of(2, &non_finite),
        ConstructRefusal::InvalidInput,
    );

    // A section that is not isomorphic (arc count differs) cannot correspond.
    let wire = wire_of(&[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]);
    let triangle = wire_of(&[(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)]);
    let mismatch = resolve_correspondence(&wire, &[triangle], &vertex_sum_sq());
    expect_refusal(mismatch, ConstructRefusal::InvalidInput);

    // An anchor index outside the wire's vertex range is invalid input.
    let square = wire_of(&[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]);
    let out_of_range = resolve_correspondence(&wire, &[square], &anchored(4, false));
    expect_refusal(out_of_range, ConstructRefusal::InvalidInput);
}

#[test]
fn empty_section_list_resolves_to_an_empty_correspondence() {
    let wire = wire_of(&[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]);
    let result = resolve_correspondence(&wire, &[], &vertex_sum_sq());
    expect_correspondence(
        result,
        Correspondence {
            orientation: true,
            anchor: None,
            shifts: Vec::new(),
        },
    );
}
