//! CC-003-ARGMIN integration tests (P4 argmin-with-margin, spine seam S5):
//! the operator certifies an index `i*` only on strict separation
//! (`sup[i*] < inf[j]` for every `j != i*`), and refuses the typed
//! `AmbiguousEventOrdering` on overlap — never tie-breaking by value, never an
//! epsilon slack, never a "closest wins" fallback.

#![deny(clippy::unwrap_used)]

use truck_certified::construct::argmin::argmin_margin;
use truck_certified::construct::fixtures as fx;
use truck_certified::construct::refusal::ConstructRefusal;
use truck_certified::construct::Interval;

/// A certified interval from explicit lo/hi bounds (test-local helper).
fn iv(lo: f64, hi: f64) -> Interval {
    Interval { lo, hi }
}

/// Extract the `Ok` index of a certified argmin (the enclosures separate by
/// construction; the refusal arm is a test-bug panic, never an unwrap).
fn expect_argmin(enclosures: &[Interval]) -> usize {
    match argmin_margin(enclosures) {
        Ok(index) => index,
        Err(refusal) => panic!("expected a certified argmin, refused: {refusal:?}"),
    }
}

/// Assert `argmin_margin` refuses the exact expected typed outcome.
fn expect_refusal(enclosures: &[Interval], expected: ConstructRefusal) {
    match argmin_margin(enclosures) {
        Ok(index) => panic!("expected refusal {expected:?}, got a certified argmin {index}"),
        Err(refusal) => assert_eq!(refusal, expected),
    }
}

#[test]
fn strictly_separated_enclosures_select_the_unique_minimizer() {
    // CC-000 fixture 3 is the required input: [[0,1], [2,3], [4,5]].
    let separated = fx::argmin_separated().expect("the CC-000 argmin fixture is valid data");
    assert_eq!(separated.enclosures.len(), 3);
    assert_eq!(separated.argmin, 0, "the fixture ground truth is index 0");

    let index = expect_argmin(&separated.enclosures);
    assert_eq!(
        index, separated.argmin,
        "the operator returns the unique strict argmin"
    );

    // The certified index's supremum lies strictly below every other infimum.
    for (j, enclosure) in separated.enclosures.iter().enumerate() {
        if j != index {
            assert!(
                separated.enclosures[index].hi < enclosure.lo, // H-3: strict sup<inf margin
                "argmin supremum strictly below enclosure {j}"
            );
        }
    }

    // Index identity, not value order: a separated argmin at a non-zero index.
    let middle = [iv(5.0, 6.0), iv(0.0, 1.0), iv(7.0, 8.0)];
    assert_eq!(
        expect_argmin(&middle),
        1,
        "the argmin is index 1, not the smallest value"
    );
}

#[test]
fn overlapping_enclosures_refuse_ambiguous_event_ordering() {
    // CC-000 fixture 4 is the required input: [[0,3], [2,5], [4,7]] pairwise
    // overlap — no index separates, so the operator refuses.
    let overlapping = fx::argmin_overlapping().expect("the CC-000 argmin fixture is valid data");
    assert_eq!(overlapping.enclosures.len(), 3);
    expect_refusal(
        &overlapping.enclosures,
        ConstructRefusal::AmbiguousEventOrdering,
    );

    // A touching boundary (sup == inf) is not strict separation: no epsilon
    // slack admits it.
    let touching = [iv(0.0, 1.0), iv(1.0, 2.0)];
    expect_refusal(&touching, ConstructRefusal::AmbiguousEventOrdering);

    // A nested enclosure also fails the strict-separation condition.
    let nested = [iv(0.0, 5.0), iv(2.0, 3.0)];
    expect_refusal(&nested, ConstructRefusal::AmbiguousEventOrdering);
}

#[test]
fn empty_input_refuses_invalid_input() {
    expect_refusal(&[], ConstructRefusal::InvalidInput);
}

#[test]
fn single_element_returns_zero() {
    assert_eq!(expect_argmin(&[iv(2.0, 3.0)]), 0);
    assert_eq!(
        expect_argmin(&[iv(-100.0, 5.5)]),
        0,
        "a lone wide enclosure separates vacuously"
    );
}

#[test]
fn tie_is_refused_never_broken_by_value_comparison() {
    // Indices 0 and 1 share the minimum supremum (2.0): the lower infimum
    // (0.25 < 1.0), a different value shape, or a width comparison would each
    // "pick" index 0 — the operator refuses instead.
    let tied_sup = [iv(0.25, 2.0), iv(1.0, 2.0), iv(3.0, 4.0)];
    expect_refusal(&tied_sup, ConstructRefusal::AmbiguousEventOrdering);

    // Two identical enclosures tie on every value; the refusal is the same.
    let identical = [iv(1.0, 2.0), iv(1.0, 2.0)];
    expect_refusal(&identical, ConstructRefusal::AmbiguousEventOrdering);
}

#[test]
fn non_finite_enclosure_bounds_refuse_invalid_input() {
    expect_refusal(&[iv(f64::NAN, 1.0)], ConstructRefusal::InvalidInput);
    expect_refusal(&[iv(0.0, f64::NAN)], ConstructRefusal::InvalidInput);
    expect_refusal(&[iv(0.0, f64::INFINITY)], ConstructRefusal::InvalidInput);
    expect_refusal(
        &[iv(f64::NEG_INFINITY, 1.0)],
        ConstructRefusal::InvalidInput,
    );

    // A finite candidate cannot rescue a non-finite enclosure elsewhere.
    let poisoned = [iv(0.0, 1.0), iv(2.0, f64::INFINITY)];
    expect_refusal(&poisoned, ConstructRefusal::InvalidInput);
}
