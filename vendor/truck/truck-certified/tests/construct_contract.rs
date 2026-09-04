//! The CC shim contract tests (CC-000-CONTRACT): the frozen construct shapes,
//! the refusing stub constructors, the C3 bridge, the C6 config constants, and
//! the fixture kit's machine-checked ground truths. No solver is implemented
//! or invoked here — every numerical fact is a direct evaluation of stored
//! data.

#![deny(clippy::unwrap_used)]

use std::collections::HashSet;

use truck_certified::construct::config as cfg;
use truck_certified::construct::convert::{box3_to_ibox, from_inari};
use truck_certified::construct::fixtures as fx;
use truck_certified::construct::refusal::ConstructRefusal;
use truck_certified::construct::stubs::{
    BoundaryPlan, BranchSeed, EventKind, RadiusLaw, ShiftFunctional, TripleContactNode, WireComplex,
};
use truck_certified::construct::Interval;
use truck_evidence::enclosure::{Box3, Interval as InariInterval};

/// Extract the `Ok` of a fallible construction; the fixture data is valid by
/// construction, so the refusal arm is a test-bug panic (never an unwrap).
fn construct<T>(result: Result<T, ConstructRefusal>) -> T {
    match result {
        Ok(value) => value,
        Err(refusal) => panic!("a construction that must succeed was refused: {refusal:?}"),
    }
}

/// Assert a refusing stub constructor refuses exactly `Unfrozen` (C7).
fn assert_stub_refuses<T>(result: Result<T, ConstructRefusal>) {
    match result {
        Ok(_) => panic!("a refusing stub constructor must refuse"),
        Err(refusal) => assert_eq!(refusal, ConstructRefusal::Unfrozen),
    }
}

/// A certified interval from explicit lo/hi bounds (test-local helper).
fn iv(lo: f64, hi: f64) -> Interval {
    Interval { lo, hi }
}

/// The 14 frozen C4 variants, in declaration order.
fn all_construct_refusals() -> Vec<ConstructRefusal> {
    use ConstructRefusal::*;
    vec![
        NonPositiveWeightField,
        SingularInterpolationSystem,
        AmbiguousCorrespondence,
        FocalDegeneracy,
        CanalSingular,
        RankDeficientContact,
        UnintendedContact,
        StarNotEmbedded,
        NoAdmissibleProjection,
        NonGenericThicknessEvent,
        AmbiguousEventOrdering,
        InvalidInput,
        ConditioningBelowThreshold,
        Unfrozen,
    ]
}

#[test]
fn construct_refusal_variants_are_distinct_and_tagged() {
    let variants = all_construct_refusals();
    assert_eq!(variants.len(), 14, "exactly the frozen C4 variant set");

    // The frozen variant names, in declaration order (CC-000 review text).
    const EXPECTED_TAGS: [&str; 14] = [
        "NonPositiveWeightField",
        "SingularInterpolationSystem",
        "AmbiguousCorrespondence",
        "FocalDegeneracy",
        "CanalSingular",
        "RankDeficientContact",
        "UnintendedContact",
        "StarNotEmbedded",
        "NoAdmissibleProjection",
        "NonGenericThicknessEvent",
        "AmbiguousEventOrdering",
        "InvalidInput",
        "ConditioningBelowThreshold",
        "Unfrozen",
    ];

    let mut seen_tags = HashSet::new();
    for (variant, expected) in variants.iter().zip(EXPECTED_TAGS.iter()) {
        assert_eq!(variant.tag(), *expected, "tag of {variant:?}");
        assert!(
            seen_tags.insert(variant.tag()),
            "duplicate tag for {variant:?}"
        );
    }
    assert_eq!(seen_tags.len(), 14, "all tags distinct");

    // Copy + Eq semantics of the derived set.
    let marker = ConstructRefusal::Unfrozen;
    let copied = marker;
    assert_eq!(copied, ConstructRefusal::Unfrozen);
    assert_ne!(
        ConstructRefusal::Unfrozen,
        ConstructRefusal::InvalidInput,
        "the C7 marker is distinct from the input rejection"
    );
}

#[test]
fn config_constants_match_the_spine_document() {
    // Spine decision C6 values, verbatim (kernel/config.rs pattern).
    assert_eq!(cfg::CC_N_EXACT, 64);
    assert_eq!(cfg::CC_ETA_J, 1e-12); // H-3: normative C6 default (regularity margin floor)
    assert_eq!(cfg::CC_ETA_PI, 1e-12); // H-3: normative C6 default (projection determinant margin)
    assert_eq!(cfg::CC_MU_CLEAR, 1e-9); // H-3: normative C6 default (clearance margin)
    assert_eq!(cfg::CC_DEPTH_MAX, 40);

    // The declared types are the normative ones.
    let _: usize = cfg::CC_N_EXACT;
    let _: f64 = cfg::CC_ETA_J;
    let _: f64 = cfg::CC_ETA_PI;
    let _: f64 = cfg::CC_MU_CLEAR;
    let _: u32 = cfg::CC_DEPTH_MAX;
}

#[test]
fn inari_conversion_is_an_exact_order_preserving_copy() {
    // Both universes are outward-rounded; the bridge copies endpoints
    // verbatim, so lo = inf, hi = sup, order preserved, no width added.
    let source = InariInterval::try_from((-0.25, 0.5)).expect("valid inari interval");
    let converted = from_inari(source);
    assert_eq!(converted.lo, source.inf()); // H-3: exact endpoint copy
    assert_eq!(converted.hi, source.sup()); // H-3: exact endpoint copy
    assert!(converted.lo <= converted.hi, "order preserved");
    assert_eq!(converted.hi - converted.lo, source.sup() - source.inf()); // H-3: exact width copy

    // A second, non-symmetric sample keeps the property.
    let wide = InariInterval::try_from((1.5, 2.75)).expect("valid inari interval");
    let converted = from_inari(wide);
    assert_eq!(converted.lo, wide.inf()); // H-3: exact endpoint copy
    assert_eq!(converted.hi, wide.sup()); // H-3: exact endpoint copy
}

#[test]
fn box3_to_ibox_preserves_bounds() {
    let x = InariInterval::try_from((-1.5, 2.0)).expect("valid x");
    let y = InariInterval::try_from((0.0, 0.25)).expect("valid y");
    let z = InariInterval::try_from((3.0, 3.0)).expect("valid z");
    let box3 = Box3 { x, y, z };
    let ibox = box3_to_ibox(&box3);

    assert_eq!(ibox.lo[0], box3.x.inf()); // H-3: exact per-axis endpoint copy
    assert_eq!(ibox.hi[0], box3.x.sup()); // H-3: exact per-axis endpoint copy
    assert_eq!(ibox.lo[1], box3.y.inf()); // H-3: exact per-axis endpoint copy
    assert_eq!(ibox.hi[1], box3.y.sup()); // H-3: exact per-axis endpoint copy
    assert_eq!(ibox.lo[2], box3.z.inf()); // H-3: exact per-axis endpoint copy
    assert_eq!(ibox.hi[2], box3.z.sup()); // H-3: exact per-axis endpoint copy

    // Order is preserved on every axis.
    assert!(ibox.lo[0] <= ibox.hi[0] && ibox.lo[1] <= ibox.hi[1] && ibox.lo[2] <= ibox.hi[2]);
}

#[test]
fn radius_law_stubs_carry_no_default_construction() {
    // No Default anywhere in the stub file: no #[derive(Default)], no
    // `impl Default`, no `default` spelling — nothing is implicitly
    // default-constructed.
    let source = include_str!("../src/construct/stubs.rs");
    assert!(
        !source.contains("Default"),
        "stubs.rs must not implement Default on any stub type"
    );
    assert!(
        !source.contains("default"),
        "stubs.rs must not spell a default path"
    );

    // The admissible v1 radius laws (theory §5.3) are plain data enums.
    let _constant = RadiusLaw::Constant(1.0);
    let _linear = RadiusLaw::Linear { r0: 1.0, r1: 2.0 };
    let _cubic = RadiusLaw::CubicHermite {
        r0: 1.0,
        r1: 2.0,
        m0: 0.0,
        m1: 0.0,
    };
    let _monotone = RadiusLaw::MonotoneCubic(vec![(0.0, 1.0), (1.0, 2.0)]);
    let _vertex = RadiusLaw::VertexControl(vec![1.0, 2.0, 3.0]);

    // The S12 event vocabulary (theory §5.2) is six distinct unit tags.
    let events = [
        EventKind::Trim,
        EventKind::ThirdFace,
        EventKind::Focal,
        EventKind::Rank,
        EventKind::Collision,
        EventKind::Trace,
    ];
    let mut seen_events = HashSet::new();
    for event in events {
        assert!(
            seen_events.insert(format!("{event:?}")),
            "duplicate event kind {event:?}"
        );
    }
    assert_eq!(seen_events.len(), 6, "the six event kinds are distinct");

    // The opaque seam stubs only carry refusing constructors (C7): production
    // belongs to CC-013 / CC-005 / CC-030 respectively.
    assert_stub_refuses(WireComplex::try_new());
    assert_stub_refuses(ShiftFunctional::try_new());
    assert_stub_refuses(BoundaryPlan::try_new());
    assert_stub_refuses(BranchSeed::try_new());

    // The S11 output record's refusing constructor (CC-020 owns production);
    // the field shape itself is frozen and public.
    let centre = [iv(0.0, 0.0); 3];
    let radius = iv(1.0, 1.0);
    let contacts = [[iv(0.0, 0.0), iv(1.0, 1.0)]; 3];
    assert_stub_refuses(TripleContactNode::try_new(centre, radius, contacts));
}

#[test]
fn fixture_ground_truths_hold() {
    // 1. banded_cubic_uniform(4): order-5 uniform cubic collocation.
    let banded = construct(fx::banded_cubic_uniform(4));
    assert_eq!(banded.n, 4);
    assert_eq!(banded.size, 5);
    assert_eq!(banded.stations.len(), 5, "n + 1 uniform stations");
    for (i, station) in banded.stations.iter().enumerate() {
        assert_eq!(*station, i as f64); // H-3: exact uniform station value (integer lattice)
    }
    assert_eq!(
        banded.bands.len(),
        banded.size * banded.size,
        "row-major dense"
    );
    for row in 0..banded.size {
        let mut off_diag_sum = 0.0;
        for col in 0..banded.size {
            let entry = banded.bands[row * banded.size + col];
            if row == col {
                assert_eq!(entry.lo, 4.0); // H-3: exact diagonal coefficient
                assert_eq!(entry.hi, 4.0); // H-3: exact diagonal coefficient
            } else if (row as isize - col as isize).abs() == 1 {
                assert_eq!(entry.lo, 1.0); // H-3: exact off-diagonal coefficient
                assert_eq!(entry.hi, 1.0); // H-3: exact off-diagonal coefficient
                off_diag_sum += 1.0;
            } else {
                assert_eq!(entry.lo, 0.0); // H-3: exact zero band gap
                assert_eq!(entry.hi, 0.0); // H-3: exact zero band gap
            }
        }
        assert!(
            off_diag_sum < 4.0,
            "strict diagonal dominance guarantees the positive determinant"
        );
    }
    assert!(
        banded.bands[0].lo > 0.0,
        "the first pivot is away from zero"
    );
    assert!(banded.det_exact > 0, "det sign known: positive");
    // The exact integer determinant of the order-5 matrix is 780, recomputed
    // by the tridiagonal recurrence D_k = 4 D_{k-1} - D_{k-2}.
    let mut d_km2 = 1i64; // D_0
    let mut d_km1 = 4i64; // D_1
    let mut det = d_km1;
    for _ in 2..=banded.size {
        det = 4 * d_km1 - d_km2;
        d_km2 = d_km1;
        d_km1 = det;
    }
    assert_eq!(banded.det_exact, det, "stored det matches the recurrence");
    assert_eq!(det, 780);

    // 2. banded_pivot_spans_zero: the first diagonal pivot strictly contains 0.
    let pivot_fx = construct(fx::banded_pivot_spans_zero());
    assert_eq!(pivot_fx.size, 2);
    assert_eq!(pivot_fx.bands.len(), 4);
    let pivot = pivot_fx.bands[0];
    assert!(
        pivot.lo < 0.0 && pivot.hi > 0.0,
        "pivot strictly contains 0"
    );
    assert!(pivot.contains(0.0));

    // 3/4. argmin_separated / argmin_overlapping.
    let separated = construct(fx::argmin_separated());
    assert_eq!(separated.enclosures.len(), 3);
    assert_eq!(separated.argmin, 0);
    for (j, enclosure) in separated.enclosures.iter().enumerate() {
        if j != separated.argmin {
            assert!(
                separated.enclosures[separated.argmin].hi < enclosure.lo, // H-3: strict sup<inf margin
                "argmin supremum strictly below enclosure {j}"
            );
        }
    }
    for i in 0..separated.enclosures.len() {
        if i != separated.argmin {
            // A non-argmin never separates: some other enclosure fails the
            // strict sup < inf test against it.
            let mut separates = true;
            for j in 0..separated.enclosures.len() {
                if i != j && separated.enclosures[i].hi >= separated.enclosures[j].lo {
                    separates = false; // H-3: a non-separated witness pair
                }
            }
            assert!(!separates, "only index {} separates", separated.argmin);
        }
    }
    let overlapping = construct(fx::argmin_overlapping());
    assert_eq!(overlapping.enclosures.len(), 3);
    for i in 0..overlapping.enclosures.len() {
        let mut separates = true;
        for j in 0..overlapping.enclosures.len() {
            if i != j && overlapping.enclosures[i].hi >= overlapping.enclosures[j].lo {
                separates = false; // H-3: a non-separated witness pair
            }
        }
        assert!(
            !separates,
            "no index separates when the enclosures overlap (index {i})"
        );
    }

    // 5. flat_patch: sigma > 0, L = 0 => delta infinite.
    let flat = construct(fx::flat_patch());
    assert!(flat.sigma.0 > 0.0, "sigma positive");
    assert_eq!(flat.curvature_l, 0.0);
    assert!(flat.expected_delta.is_infinite(), "2 sigma / 0 diverges");

    // 6. curved_patch: delta = 2 sigma_lo / L exactly (dyadic data).
    let curved = construct(fx::curved_patch());
    assert!(curved.sigma.0 > 0.0 && curved.curvature_l > 0.0);
    assert_eq!(
        curved.expected_delta,
        2.0 * curved.sigma.0 / curved.curvature_l
    ); // H-3: exact dyadic delta

    // 7. degenerate_patch: the sigma enclosure contains 0.
    let degenerate = construct(fx::degenerate_patch());
    assert!(
        degenerate.sigma.0 <= 0.0 && 0.0 <= degenerate.sigma.1,
        "sigma contains 0"
    );
    assert!(degenerate.expected_delta <= 0.0, "no admissible radius");

    // 8. genuine_star: every piece positively embedded, seams glued.
    let star = construct(fx::genuine_star());
    assert_eq!(star.pieces.len(), 2);
    assert!(
        !star.sign_change,
        "no determinant sign change in a genuine star"
    );
    for piece in &star.pieces {
        assert!(
            piece.det_lower.lo > 0.0,
            "piece determinant strictly positive"
        );
        assert!(piece.seam_glued, "seams glued");
        assert!(piece.boundary_simple, "boundary simple");
    }

    // 9. folded_corner: determinant sign change, non-simple fold boundary.
    let fold = construct(fx::folded_corner());
    assert_eq!(fold.pieces.len(), 2);
    assert!(fold.sign_change, "determinant sign change across the fold");
    let mut saw_positive = false;
    let mut saw_negative = false;
    let mut saw_not_simple = false;
    for piece in &fold.pieces {
        if piece.det_lower.lo > 0.0 {
            saw_positive = true;
        }
        if piece.det_lower.hi < 0.0 {
            saw_negative = true;
        }
        if !piece.boundary_simple {
            saw_not_simple = true;
        }
    }
    assert!(
        saw_positive && saw_negative,
        "opposite-signed determinant bounds"
    );
    assert!(saw_not_simple, "the folded piece boundary is not simple");
}
