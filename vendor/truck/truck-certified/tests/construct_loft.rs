//! The CC-010-LOFT-CORE integration tests (spine S8): exact compatibility,
//! chord-length stationing, de Boor-averaged station knot vectors, and the
//! L0/L1/L2 loft construction over the landed `truck_geometry::nurbs` types.
//!
//! The L1 ground truth is built IN THE TEST from the landed nurbs types: three
//! cubic clamped sections with known control points over a shared knot vector.
//! The loft must reproduce each section at its station up to the delivered L2
//! enclosure width `ε` (`LoftOutput::epsilon`).

#![deny(clippy::unwrap_used)]

use truck_certified::construct::banded::factor_banded_tp;
use truck_certified::construct::loft::{
    averaged_knot_vector, chord_length_stations, loft_collocation_bands, loft_sections,
    make_compatible,
};
use truck_certified::construct::refusal::ConstructRefusal;
use truck_geometry::prelude::{BSplineCurve, KnotVec, ParametricCurve, ParametricSurface, Vector4};

/// Extract the `Ok` of a fallible construction; the fixture data is valid by
/// construction, so the refusal arm is a test-bug panic (never an unwrap).
fn construct<T>(result: Result<T, ConstructRefusal>) -> T {
    match result {
        Ok(value) => value,
        Err(refusal) => panic!("a construction that must succeed was refused: {refusal:?}"),
    }
}

/// A homogeneous control point with unit weight (test helper).
fn p4(x: f64, y: f64, z: f64) -> Vector4 {
    Vector4::new(x, y, z, 1.0)
}

/// Project a homogeneous point to its Euclidean coordinate triple.
fn euclid4(homogeneous: Vector4) -> [f64; 3] {
    [
        homogeneous.x / homogeneous.w,
        homogeneous.y / homogeneous.w,
        homogeneous.z / homogeneous.w,
    ]
}

/// The Euclidean distance between two projected samples.
fn distance(p: [f64; 3], q: [f64; 3]) -> f64 {
    let dx = q[0] - p[0];
    let dy = q[1] - p[1];
    let dz = q[2] - p[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// The shared clamped cubic knot vector over `[0, 1]` with two interior knots.
fn cubic_knot() -> KnotVec {
    KnotVec::from(vec![
        0.0,
        0.0,
        0.0,
        0.0,
        1.0 / 3.0,
        2.0 / 3.0,
        1.0,
        1.0,
        1.0,
        1.0,
    ])
}

/// The L1 ground truth: three cubic clamped sections with known control
/// points, sharing `cubic_knot()`. Each section is a `Vector4` (homogeneous,
/// unit-weight) B-spline; the `z` coordinate distinguishes the stations.
fn loft_fixture_sections() -> Vec<BSplineCurve<Vector4>> {
    let section0 = [
        p4(0.0, 0.2, 0.0),
        p4(0.6, 0.8, 0.0),
        p4(1.2, 0.3, 0.0),
        p4(1.8, 0.9, 0.0),
        p4(2.4, 0.4, 0.0),
        p4(3.0, 0.7, 0.0),
    ];
    let section1 = [
        p4(0.0, 0.0, 1.0),
        p4(0.6, 1.0, 1.0),
        p4(1.2, 0.1, 1.0),
        p4(1.8, 1.1, 1.0),
        p4(2.4, 0.2, 1.0),
        p4(3.0, 0.9, 1.0),
    ];
    let section2 = [
        p4(0.0, 0.5, 2.0),
        p4(0.6, 0.3, 2.0),
        p4(1.2, 1.0, 2.0),
        p4(1.8, 0.6, 2.0),
        p4(2.4, 1.2, 2.0),
        p4(3.0, 0.8, 2.0),
    ];
    let knot = cubic_knot();
    [section0, section1, section2]
        .iter()
        .map(|controls| BSplineCurve::new(knot.clone(), controls.to_vec()))
        .collect()
}

/// A cubic clamped section over `[0, 1]` whose only interior knot is `x`
/// (test helper for the knot-union fixtures).
fn cubic_section_with_interior(x: f64) -> BSplineCurve<Vector4> {
    let knot = KnotVec::from(vec![0.0, 0.0, 0.0, 0.0, x, 1.0, 1.0, 1.0, 1.0]);
    let controls = [
        p4(0.0, 0.5, 0.0),
        p4(0.75, 1.5, 0.0),
        p4(1.5, 0.5, 0.0),
        p4(2.25, 1.5, 0.0),
        p4(3.0, 0.5, 0.0),
    ];
    BSplineCurve::new(knot, controls.to_vec())
}

/// A quadratic clamped section over `[0, 1]` whose only interior knot is `x`.
fn quadratic_section_with_interior(x: f64) -> BSplineCurve<Vector4> {
    let knot = KnotVec::from(vec![0.0, 0.0, 0.0, x, 1.0, 1.0, 1.0]);
    let controls = [
        p4(0.0, 0.5, 0.0),
        p4(1.0, 1.5, 0.0),
        p4(2.0, 0.5, 0.0),
        p4(3.0, 1.5, 0.0),
    ];
    BSplineCurve::new(knot, controls.to_vec())
}

/// The `v` interpolation degree of the loft fixtures (three sections admit a
/// quadratic station spline at most).
const LOFT_V_DEGREE: usize = 2;

/// The number of `u` sample intervals of the L1 gate grid.
const L1_U_SAMPLES: usize = 32;

#[test]
fn loft_reproduces_sections_identically_up_to_epsilon() {
    // L1 gate (H-3 opt-outs on every float comparison). The ground-truth
    // fixture is three cubic clamped sections with known control points.
    let sections = loft_fixture_sections();
    let compatible = construct(make_compatible(&sections));
    let stations = chord_length_stations(&compatible);
    assert_eq!(stations.len(), 3);

    let bands = construct(loft_collocation_bands(&stations, LOFT_V_DEGREE));
    let factor = construct(factor_banded_tp(&bands));
    let output = construct(loft_sections(
        &compatible,
        &stations,
        LOFT_V_DEGREE,
        &factor,
    ));
    let epsilon = output.epsilon;

    // The delivered surface must reproduce each section at its station over a
    // `u` sample grid, to within the delivered L2 enclosure width.
    for (k, station) in stations.iter().enumerate() {
        let section = &compatible[k];
        let mut max_deviation = 0.0_f64;
        for i in 0..=L1_U_SAMPLES {
            let u = (i as f64) / (L1_U_SAMPLES as f64);
            let on_surface = output.surface.subs(u, *station);
            let on_section = section.subs(u);
            let deviation = distance(euclid4(on_surface), euclid4(on_section));
            if deviation > max_deviation {
                max_deviation = deviation;
            }
        }
        assert!(
            max_deviation <= epsilon,
            "section {k} deviates {max_deviation} from its station curve, above the \
             delivered enclosure width {epsilon}"
        ); // H-3
    }
}

#[test]
fn delivered_epsilon_matches_max_control_error() {
    // The L2 contract: `LoftOutput::epsilon` is exactly the delivered control
    // enclosure width of the factor that solved the loft.
    let sections = loft_fixture_sections();
    let compatible = construct(make_compatible(&sections));
    let stations = chord_length_stations(&compatible);
    let bands = construct(loft_collocation_bands(&stations, LOFT_V_DEGREE));
    let factor = construct(factor_banded_tp(&bands));
    let output = construct(loft_sections(
        &compatible,
        &stations,
        LOFT_V_DEGREE,
        &factor,
    ));

    assert_eq!(output.epsilon, factor.max_control_error()); // H-3
    assert!(output.epsilon >= 0.0); // H-3
}

#[test]
fn averaged_knot_vector_satisfies_schoenberg_whitney_on_increasing_stations() {
    // Six strictly increasing stations, cubic station spline: the averaged
    // knot vector is clamped with `degree + 1` end multiplicities, and every
    // station lies in the support `[U_k, U_{k + q + 1}]` of its own `k`-th
    // basis function (the Schoenberg–Whitney sufficient condition).
    let degree = 3;
    let stations = [0.0, 0.1, 0.3, 0.7, 0.9, 1.0];
    let knot = averaged_knot_vector(&stations, degree);
    assert_eq!(knot.len(), stations.len() + degree + 1);

    // Clamped ends: the first and last `degree + 1` knots repeat the end
    // stations exactly.
    for k in 0..=degree {
        assert_eq!(knot[k], stations[0]); // H-3
        assert_eq!(
            knot[stations.len() + degree - k],
            stations[stations.len() - 1]
        ); // H-3
    }

    // Interior knots are strictly between the end stations.
    for k in (degree + 1)..(stations.len() + degree + 1 - (degree + 1)) {
        assert!(knot[k] > stations[0]); // H-3
        assert!(knot[k] < stations[stations.len() - 1]); // H-3
    }

    // Schoenberg–Whitney support inclusion: `v_k in [U_k, U_{k + q + 1}]`.
    for (k, &v) in stations.iter().enumerate() {
        assert!(knot[k] <= v && v <= knot[k + degree + 1]); // H-3
    }

    // The stationing is genuinely solvable: the banded-TP factor of the
    // collocation storage must admit the no-pivot factorization.
    let bands = construct(loft_collocation_bands(&stations, degree));
    construct(factor_banded_tp(&bands));
}

#[test]
fn chord_length_stations_are_deterministic_and_normalized() {
    let sections = loft_fixture_sections();
    let compatible = construct(make_compatible(&sections));

    let first = chord_length_stations(&compatible);
    let second = chord_length_stations(&compatible);
    assert_eq!(first.len(), compatible.len());
    assert_eq!(first, second); // H-3: deterministic bit-for-bit

    // Normalized: every station lies in `[0, 1]`, strictly increasing for
    // non-degenerate sections, and the last station is exactly `1.0`.
    for window in first.windows(2) {
        assert!(window[0] >= 0.0 && window[1] <= 1.0); // H-3
        assert!(window[0] < window[1]); // H-3
    }
    let last = first[first.len() - 1];
    assert!(last == 1.0); // H-3
}

#[test]
fn knot_union_is_exact_additive_never_tolerance_merged() {
    // Two cubic clamped sections whose interior knots differ by ONE ulp: far
    // below any legacy tolerance, so a tolerance-based merge would collapse
    // them. The exact union must keep both values, each with its own copy.
    let a = 0.3_f64;
    let b = f64::from_bits(a.to_bits() + 1);
    assert!(a < b); // H-3: the near-equal pair is genuinely distinct

    let section_a = cubic_section_with_interior(a);
    let section_b = cubic_section_with_interior(b);
    let compatible = construct(make_compatible(&[section_a, section_b]));
    assert_eq!(compatible.len(), 2);

    let knot0 = compatible[0].knot_vec();
    let knot1 = compatible[1].knot_vec();
    assert_eq!(knot0, knot1);
    assert_eq!(compatible[0].degree(), 3);
    assert_eq!(compatible[1].degree(), 3);

    // Exact additive union: `9 + 9 - 8` shared clamped-end knots = 10. The two
    // near-equal interior knots are both present, distinct, and never merged.
    assert_eq!(knot0.len(), 10);
    let count_a = knot0.iter().filter(|&&k| k == a).count();
    let count_b = knot0.iter().filter(|&&k| k == b).count();
    assert_eq!(count_a, 1);
    assert_eq!(count_b, 1);

    // The sections remain clamped after the exact union.
    assert!(compatible[0].is_clamped());
    assert!(compatible[1].is_clamped());
}

#[test]
fn make_compatible_elevates_degrees_and_unions_knots_exactly() {
    // A quadratic section with interior knot `a` and a cubic section with the
    // near-equal interior knot `b`: degree elevation must lift the quadratic to
    // cubic, and the exact union must carry both near-equal values — the
    // elevated knot copies of `a` and the distinct `b` are never tolerance
    // merged.
    let a = 0.3_f64;
    let b = f64::from_bits(a.to_bits() + 1);
    assert!(a < b); // H-3
    let quadratic = quadratic_section_with_interior(a);
    let cubic = cubic_section_with_interior(b);

    let compatible = construct(make_compatible(&[quadratic, cubic]));
    assert_eq!(compatible.len(), 2);
    assert_eq!(compatible[0].degree(), 3);
    assert_eq!(compatible[1].degree(), 3);
    assert_eq!(compatible[0].knot_vec(), compatible[1].knot_vec());

    // truck's degree elevation decomposes to Bézier pieces and rebuilds, so the
    // single interior knot of the quadratic lands at full cubic multiplicity
    // `p + 1 = 4` (the C0 seams of the rebuilt representation). The exact union
    // therefore carries four copies of `a` and one copy of `b`, plus the four
    // clamped ends on each side: 4 + 4 + 1 + 4.
    let knot = compatible[0].knot_vec();
    assert_eq!(knot.len(), 13);
    let count_a = knot.iter().filter(|&&k| k == a).count();
    let count_b = knot.iter().filter(|&&k| k == b).count();
    assert_eq!(count_a, 4);
    assert_eq!(count_b, 1);
}

#[test]
fn incompatible_sections_refuse() {
    // Empty input refuses `InvalidInput`.
    match make_compatible(&[]) {
        Err(ConstructRefusal::InvalidInput) => {}
        Ok(_) => panic!("empty input must refuse as InvalidInput"),
        Err(other) => panic!("wrong refusal for empty input: {other:?}"),
    }

    // An unclamped section's knot vector cannot be unioned by knot insertion.
    let unclamped_knot = KnotVec::from(vec![0.0, 0.0, 0.3, 0.7, 1.0, 1.0]);
    let controls = [p4(0.0, 0.5, 0.0), p4(1.5, 1.0, 0.0), p4(3.0, 0.5, 0.0)];
    let unclamped = BSplineCurve::new(unclamped_knot, controls.to_vec());
    assert!(!unclamped.is_clamped());
    match make_compatible(&[unclamped]) {
        Err(ConstructRefusal::InvalidInput) => {}
        Ok(_) => panic!("an unclamped section must refuse as InvalidInput"),
        Err(other) => panic!("wrong refusal for an unclamped section: {other:?}"),
    }

    // Non-increasing stations refuse the collocation band builder.
    match loft_collocation_bands(&[0.0, 0.5, 0.25], LOFT_V_DEGREE) {
        Err(ConstructRefusal::InvalidInput) => {}
        Ok(_) => panic!("non-increasing stations must refuse as InvalidInput"),
        Err(other) => panic!("wrong refusal for non-increasing stations: {other:?}"),
    }

    // A station count that does not match the section count refuses the solve.
    let sections = loft_fixture_sections();
    let compatible = construct(make_compatible(&sections));
    let stations = chord_length_stations(&compatible);
    let bands = construct(loft_collocation_bands(&stations, LOFT_V_DEGREE));
    let factor = construct(factor_banded_tp(&bands));
    let mismatched = [stations[0], stations[1]];
    match loft_sections(&compatible, &mismatched, LOFT_V_DEGREE, &factor) {
        Err(ConstructRefusal::InvalidInput) => {}
        Ok(_) => panic!("a station count different from the section count must refuse"),
        Err(other) => panic!("wrong refusal for a station-count mismatch: {other:?}"),
    }
}
