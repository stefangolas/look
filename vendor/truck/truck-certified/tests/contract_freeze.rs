//! Contract-pinning tests for the Phase-0 freeze (BG-CK-P0-FREEZE): the F1
//! witness-edge shape, the F2 bound policy table, and the F3 continuation
//! coordinate contract. The test names are the contract.

use truck_certified::contract::{
    certified_bound, select_continuation_coordinate, BoundMechanism, BoundPolicy, BoundPolicyRow,
    BoundedSurfaceInput, ContinuationCoordinate, CoordinateSwitch, IntervalEnclosure, Quantity,
    Refusal, SquareSystemInput, WitnessEdge,
};
use truck_certified::formal::numeric::PositiveFinite;

#[test]
fn witness_edge_carries_pcurve_pair_and_surfaces_and_enclosures() {
    #[derive(Debug, PartialEq)]
    struct ToySurface(u8);
    #[derive(Debug, PartialEq)]
    struct ToyPcurve(u8);

    let edge = WitnessEdge {
        pcurve_a: ToyPcurve(1),
        pcurve_b: ToyPcurve(2),
        surface_a: ToySurface(10),
        surface_b: ToySurface(20),
        enclosure_a: IntervalEnclosure::new(0.0, 1.0).expect("a valid interval"),
        enclosure_b: IntervalEnclosure::new(-2.0, -1.0).expect("a valid interval"),
    };

    // The six fields of the fiber-product witness: two pcurves, both surface
    // handles, and the two enclosures.
    assert_eq!(edge.pcurve_a, ToyPcurve(1));
    assert_eq!(edge.pcurve_b, ToyPcurve(2));
    assert_eq!(edge.surface_a, ToySurface(10));
    assert_eq!(edge.surface_b, ToySurface(20));
    assert_eq!(edge.enclosure_a.lower().get(), 0.0);
    assert_eq!(edge.enclosure_a.upper().get(), 1.0);
    assert_eq!(edge.enclosure_b.lower().get(), -2.0);
    assert_eq!(edge.enclosure_b.upper().get(), -1.0);
}

#[test]
fn witness_edge_has_no_spline_field() {
    // F1 guard: the certified Edge is the fiber-product witness and is NEVER a
    // spline carrier; the export view is a future type. The negative is
    // compile-level: `edge.spline()` (or `.bezier()`) does not compile — see
    // the `compile_fail` doctest on `WitnessEdge`. Reading the six fields here
    // pins the exact field set, so a regression that bolted a spline carrier
    // onto the witness would surface as a changed struct.
    #[derive(Debug, PartialEq)]
    struct ToySurface(u8);
    #[derive(Debug, PartialEq)]
    struct ToyPcurve(u8);

    let edge = WitnessEdge {
        pcurve_a: ToyPcurve(1),
        pcurve_b: ToyPcurve(2),
        surface_a: ToySurface(3),
        surface_b: ToySurface(4),
        enclosure_a: IntervalEnclosure::new(0.0, 1.0).expect("a valid interval"),
        enclosure_b: IntervalEnclosure::new(0.0, 1.0).expect("a valid interval"),
    };
    let pcurve_a: &ToyPcurve = &edge.pcurve_a;
    let pcurve_b: &ToyPcurve = &edge.pcurve_b;
    let surface_a: &ToySurface = &edge.surface_a;
    let surface_b: &ToySurface = &edge.surface_b;
    let enclosure_a = edge.enclosure_a;
    let enclosure_b = edge.enclosure_b;
    assert_eq!(pcurve_a.0, 1);
    assert_eq!(pcurve_b.0, 2);
    assert_eq!(surface_a.0, 3);
    assert_eq!(surface_b.0, 4);
    // Both enclosures carry the same `Method` tag (H-6: interval work only).
    assert_eq!(enclosure_a.method(), enclosure_b.method());
}

#[test]
fn bound_policy_table_names_all_five_quantities() {
    let policy = BoundPolicy::frozen();
    let rows = policy.rows();
    assert_eq!(rows.len(), 5);

    let expected = [
        (
            Quantity::NormalAdmissibility,
            BoundMechanism::IntervalComposition,
        ),
        (
            Quantity::Curvature,
            BoundMechanism::IntervalCompositionWithRootIsolationGuard,
        ),
        (
            Quantity::RationalNumerator,
            BoundMechanism::IntervalComposition,
        ),
        (
            Quantity::RationalDenominator,
            BoundMechanism::IntervalComposition,
        ),
        (
            Quantity::RationalQuotient,
            BoundMechanism::IntervalComposition,
        ),
    ];
    for (row, (quantity, mechanism)) in rows.iter().zip(expected.iter()) {
        assert_eq!(row.quantity(), *quantity);
        assert_eq!(row.mechanism(), *mechanism);
    }

    // The frozen table never records an `Unfrozen` row: the spec-gap state is
    // reserved for quantities outside the table (F2).
    assert!(
        rows.iter()
            .all(|row| row.mechanism() != BoundMechanism::Unfrozen),
        "no frozen row is Unfrozen"
    );
}

#[test]
fn denominator_well_definedness_uses_root_isolation_not_composition() {
    let policy = BoundPolicy::frozen();
    let curvature = policy
        .row_for(Quantity::Curvature)
        .expect("curvature is one of the five frozen rows");
    assert_eq!(
        curvature.mechanism(),
        BoundMechanism::IntervalCompositionWithRootIsolationGuard
    );

    // A policy-row construction that attempts composition-only for the
    // curvature guard refuses: the division's well-definedness is certified by
    // AUXILIARY ROOT ISOLATION on the denominator polynomial, never by
    // interval sign-testing/composition alone (F2).
    assert_eq!(
        BoundPolicyRow::new(Quantity::Curvature, BoundMechanism::IntervalComposition),
        Err(Refusal::InvalidInput)
    );
    // The sanctioned curvature construction succeeds.
    assert!(
        BoundPolicyRow::new(
            Quantity::Curvature,
            BoundMechanism::IntervalCompositionWithRootIsolationGuard
        )
        .is_ok(),
        "root-isolation-guarded curvature is the frozen construction"
    );
}

#[test]
fn continuation_coordinate_selection_is_deterministic_lowest_index_on_ties() {
    // Margins = |lower bound| / box extent:
    //   coordinate 0: |2.0| / 2.0 = 1.0
    //   coordinate 1: |4.0| / 2.0 = 2.0
    //   coordinate 2: |6.0| / 3.0 = 2.0  -> ties coordinate 1
    // The frozen rule breaks the tie to the LOWEST index (deterministic, no
    // hash order).
    let system = SquareSystemInput {
        diagonal_derivatives: [
            IntervalEnclosure::new(2.0, 3.0).expect("a valid interval"),
            IntervalEnclosure::new(4.0, 5.0).expect("a valid interval"),
            IntervalEnclosure::new(6.0, 7.0).expect("a valid interval"),
        ],
        extents: [
            PositiveFinite::new(2.0).expect("a positive extent"),
            PositiveFinite::new(2.0).expect("a positive extent"),
            PositiveFinite::new(3.0).expect("a positive extent"),
        ],
    };
    let selected =
        select_continuation_coordinate(&system).expect("coordinate 1 certifies away-from-zero");
    assert_eq!(selected.index, 1);
}

#[test]
fn coordinate_switch_requires_both_certificates() {
    let outgoing = ContinuationCoordinate {
        index: 0,
        relative_margin: IntervalEnclosure::new(1.0, 1.0).expect("a valid interval"),
    };
    let incoming = ContinuationCoordinate {
        index: 1,
        relative_margin: IntervalEnclosure::new(2.0, 2.0).expect("a valid interval"),
    };

    // `CoordinateSwitch` carries two REQUIRED certificates. There is no
    // `Option`, no default, and no reseed path (F3) — both fields are
    // inhabited in every value, so a one-sided switch is unrepresentable.
    let switch = CoordinateSwitch { outgoing, incoming };
    assert_eq!(switch.outgoing, outgoing);
    assert_eq!(switch.incoming, incoming);
    assert_eq!(switch.outgoing.index, 0);
    assert_eq!(switch.incoming.index, 1);
    assert_ne!(
        switch.outgoing.index, switch.incoming.index,
        "a switch changes coordinate"
    );
}

#[test]
fn no_coordinate_certified_refuses_with_named_case() {
    // Every diagonal-derivative enclosure contains (or touches) zero, so no
    // coordinate is strictly away from zero over the box.
    let system = SquareSystemInput {
        diagonal_derivatives: [
            IntervalEnclosure::new(-1.0, 1.0).expect("a valid interval"),
            IntervalEnclosure::new(-2.0, 0.0).expect("a valid interval"),
            IntervalEnclosure::new(0.0, 2.0).expect("a valid interval"),
        ],
        extents: [
            PositiveFinite::new(1.0).expect("a positive extent"),
            PositiveFinite::new(1.0).expect("a positive extent"),
            PositiveFinite::new(1.0).expect("a positive extent"),
        ],
    };
    let refusal = select_continuation_coordinate(&system)
        .expect_err("no coordinate certifies away-from-zero");
    assert_eq!(
        refusal,
        Refusal::ConditioningBelowThreshold,
        "the box refuses the named ConditioningBelowThreshold case and is never retried"
    );
}

#[test]
fn freeze_types_refuse_construction_outside_their_rules() {
    // `certified_bound` refuses for every frozen quantity: the freeze performs
    // no numerics (the mechanism is pinned, the evaluation is Phase-1).
    let patch = BoundedSurfaceInput { patch_index: 0 };
    for quantity in [
        Quantity::NormalAdmissibility,
        Quantity::Curvature,
        Quantity::RationalNumerator,
        Quantity::RationalDenominator,
        Quantity::RationalQuotient,
    ] {
        let refusal = certified_bound(quantity, &patch).expect_err("the freeze refuses");
        assert!(
            matches!(refusal, Refusal::InvalidInput),
            "a frozen quantity's bound is a construction outside the frozen rules"
        );
    }

    // `select_continuation_coordinate` refuses when no coordinate certifies and
    // is deterministic: identical input, identical refusal.
    let none_certified = SquareSystemInput {
        diagonal_derivatives: [
            IntervalEnclosure::new(-1.0, 1.0).expect("a valid interval"),
            IntervalEnclosure::new(-1.0, 1.0).expect("a valid interval"),
            IntervalEnclosure::new(-1.0, 1.0).expect("a valid interval"),
        ],
        extents: [
            PositiveFinite::new(1.0).expect("a positive extent"),
            PositiveFinite::new(1.0).expect("a positive extent"),
            PositiveFinite::new(1.0).expect("a positive extent"),
        ],
    };
    assert_eq!(
        select_continuation_coordinate(&none_certified),
        select_continuation_coordinate(&none_certified),
        "deterministic refusal — no hash order"
    );

    // No evaluator panics: every path above refused cleanly.
}
