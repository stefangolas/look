//! CC-012-LOFT-STRIPS integration tests (theory §2.2 L3): the closed-wire loft
//! as r strips over matched edges. Adjacent strips share their split vertex
//! data by construction identity (P6), so their common boundary is the SAME
//! computation and the seam agreement is BITWISE (f64 bit patterns, no
//! epsilon anywhere); one shared banded factorization serves every strip; the
//! split registry recomputes each value exactly once; CC-011 weight
//! certification runs once per strip and its refinements are applied to every
//! shipped strip net; and r = 1 (no splits) degenerates cleanly to a single
//! open-style strip. The test names are the contract.

#![deny(clippy::unwrap_used)]

use truck_base::evidence::Budget;
use truck_certified::construct::banded::factor_banded_tp;
use truck_certified::construct::loft::{loft_collocation_bands, loft_sections, make_compatible};
use truck_certified::construct::loft_strips::loft_closed_wire;
use truck_certified::construct::loft_weights::certify_weight_field;
use truck_certified::construct::refusal::ConstructRefusal;
use truck_geometry::prelude::{
    BSplineCurve, BSplineSurface, KnotVec, ParametricCurve, ParametricSurface, Vector4,
};
use truck_topology::{EntityId, Op, OpKind, OpParams, Selector};

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

/// A homogeneous control point with the given weight (test helper).
fn p4w(x: f64, y: f64, z: f64, w: f64) -> Vector4 {
    Vector4::new(x, y, z, w)
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

/// The minimum `w`-control value of a surface net, row-major (fixed order).
fn weight_net_min(surface: &BSplineSurface<Vector4>) -> f64 {
    let mut min = f64::INFINITY;
    for row in surface.control_points() {
        for point in row {
            if point.w < min {
                min = point.w;
            }
        }
    }
    min
}

/// Assert byte equality of two control rows (the L3 bit-pattern gate).
fn assert_rows_bits_equal(left: &[Vector4], right: &[Vector4]) {
    assert_eq!(
        left.len(),
        right.len(),
        "adjacent strip boundary rows must have the same v-control count"
    );
    for (a, b) in left.iter().zip(right.iter()) {
        assert_eq!(a.x.to_bits(), b.x.to_bits(), "x channel not bitwise equal");
        assert_eq!(a.y.to_bits(), b.y.to_bits(), "y channel not bitwise equal");
        assert_eq!(a.z.to_bits(), b.z.to_bits(), "z channel not bitwise equal");
        assert_eq!(a.w.to_bits(), b.w.to_bits(), "w channel not bitwise equal");
    }
}

/// Assert byte equality of two whole strip nets (row-major).
fn assert_nets_bits_equal(left: &BSplineSurface<Vector4>, right: &BSplineSurface<Vector4>) {
    let left_pts = left.control_points();
    let right_pts = right.control_points();
    assert_eq!(left_pts.len(), right_pts.len());
    for (row_l, row_r) in left_pts.iter().zip(right_pts.iter()) {
        assert_rows_bits_equal(row_l, row_r);
    }
}

/// The shared clamped cubic knot vector of the closed-wire fixtures: interior
/// knots at `1/3` and `2/3`, so two splits give three arcs.
fn loop_knot() -> KnotVec {
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

/// The knot-vector indices selecting the two matched split parameters `1/3`
/// and `2/3` of [`loop_knot`].
fn split_indices() -> Vec<usize> {
    vec![4, 5]
}

/// One closed cubic section (six controls, last == first) lifted to the given
/// `z` station, with a unit-weight carrier.
fn closed_loop_section(z: f64) -> BSplineCurve<Vector4> {
    let base = [(0.0, 0.0), (3.0, 1.0), (2.5, 3.5), (-0.5, 4.0), (-2.0, 1.5)];
    let mut controls: Vec<Vector4> = base.iter().map(|&(x, y)| p4(x, y, z)).collect();
    controls.push(p4(0.0, 0.0, z));
    BSplineCurve::new(loop_knot(), controls)
}

/// The L3 gate fixture: three closed cubic sections over [`loop_knot`] at the
/// stations `z = 0, 1, 2`, all unit weight.
fn closed_wire_sections() -> Vec<BSplineCurve<Vector4>> {
    vec![
        closed_loop_section(0.0),
        closed_loop_section(1.0),
        closed_loop_section(2.0),
    ]
}

/// The strip-interpolation stationing of the fixtures (three stations admit a
/// quadratic station spline).
const STATIONS: [f64; 3] = [0.0, 0.5, 1.0];
/// The shared `v` interpolation degree of the fixtures.
const V_DEGREE: usize = 2;

#[test]
fn adjacent_strip_boundaries_agree_bitwise() {
    // L3 gate: the u-endpoint control row of strip i and the u-start row of
    // strip i + 1 are BYTE equal — under P6 the seam agreement is not a
    // numerical fact, so a tolerance comparison here would be a test failure
    // even when the values agree to 1 ulp.
    let sections = closed_wire_sections();
    let splits = split_indices();
    let built = construct(loft_closed_wire(&sections, &splits, &STATIONS, V_DEGREE));
    assert_eq!(built.strips.len(), 3);
    assert_eq!(built.seam_ids.len(), 2);

    for pair in 0..(built.strips.len() - 1) {
        let left = built.strips[pair].surface.control_points();
        let right = built.strips[pair + 1].surface.control_points();
        let end_row = match left.last() {
            Some(row) => row,
            None => panic!("a strip surface must carry a u-endpoint control row"),
        };
        let start_row = match right.first() {
            Some(row) => row,
            None => panic!("a strip surface must carry a u-start control row"),
        };
        assert_rows_bits_equal(end_row, start_row);
    }
}

#[test]
fn one_factorization_shared_across_all_strips() {
    // Every strip is solved through the SAME banded factorization of the
    // shared collocation storage, so the L2 enclosure width ε delivered on
    // every strip is identical (distinct factorizations of the same matrix
    // can deliver different max-error enclosures; identical ε on every strip
    // is the observable).
    //
    // The fixture is three closed sections that are constant along their own
    // u axis (unit-weight point sections at distinct stations), so every
    // solved control column of every strip is the SAME right-hand side and
    // the identical ε is exact rather than a coincidence of unrelated column
    // magnitudes.
    let constant_knot = loop_knot();
    let section0 = BSplineCurve::new(constant_knot.clone(), vec![p4(0.0, 0.0, 0.0); 6]);
    let section1 = BSplineCurve::new(constant_knot.clone(), vec![p4(1.0, 0.0, 1.0); 6]);
    let section2 = BSplineCurve::new(constant_knot, vec![p4(2.0, 0.0, 2.0); 6]);
    let sections = vec![section0, section1, section2];
    let splits = split_indices();
    let built = construct(loft_closed_wire(&sections, &splits, &STATIONS, V_DEGREE));
    assert_eq!(built.strips.len(), 3);

    let first = built.strips[0].epsilon;
    for strip in built.strips.iter().skip(1) {
        assert_eq!(strip.epsilon, first); // H-3: the shared factor reports one ε
        assert!(strip.epsilon >= 0.0); // H-3
    }
}

#[test]
fn split_vertex_identity_surfaces_recomputation() {
    // The P6 split values are keyed by DAG identities derived with the landed
    // algebra (Op/OpParams, slot = the strip-pair index). Recomputation of the
    // whole construction must therefore surface the SAME identities and the
    // SAME bitwise strips: two evaluations of a split point never meet.
    let sections = closed_wire_sections();
    let splits = split_indices();
    let first = construct(loft_closed_wire(&sections, &splits, &STATIONS, V_DEGREE));
    let second = construct(loft_closed_wire(&sections, &splits, &STATIONS, V_DEGREE));

    assert_eq!(first.seam_ids.len(), 2);
    assert_eq!(first.strips.len(), second.strips.len());

    // The seam identity is the DAG node `Op { Loft, List(section ids) }`
    // applied to the section imports with slot = the strip-pair index.
    for (pair, seam) in first.seam_ids.iter().enumerate() {
        let params = OpParams::List(
            (0..sections.len() as u32)
                .map(OpParams::Index)
                .collect::<Vec<OpParams>>(),
        );
        let op = Op {
            kind: OpKind::Loft,
            params,
        };
        let inputs: Vec<EntityId> = (0..sections.len() as u64).map(EntityId::src).collect();
        assert_eq!(*seam, op.output(&inputs, pair as u32));
        assert_eq!(*seam, second.seam_ids[pair]);
    }
    assert_ne!(
        first.seam_ids[0], first.seam_ids[1],
        "the slot distinguishes strip pairs"
    );
    let pole0 = EntityId::sel(first.seam_ids[0].clone(), Selector::Pole(0));
    let pole1 = EntityId::sel(first.seam_ids[0].clone(), Selector::Pole(1));
    assert_ne!(
        pole0, pole1,
        "the station selector distinguishes split vertices"
    );

    // Recomputation is bitwise-identical across every shipped strip net.
    for (a, b) in first.strips.iter().zip(second.strips.iter()) {
        assert_nets_bits_equal(&a.surface, &b.surface);
        assert_eq!(a.epsilon, b.epsilon); // H-3
    }
}

#[test]
fn weight_refinements_applied_to_shipped_net() {
    // A closed fixture whose sections carry constant per-station weights
    // `(1, 1/20, 1)`: the interpolation across the three stations inverts a
    // totally-positive collocation matrix, whose net is NOT all-positive (the
    // raw loft net minimum is below zero), while the weight field itself is
    // strictly positive (minimum `1/20` at the middle station). CC-011 must
    // therefore certify the strip UNDER refinement, and CC-012 applies those
    // refinements to the shipped net — the shipped net admits on the free
    // fast path with an empty budget, the raw net never does.
    let weight_knot = loop_knot();
    let section0 = BSplineCurve::new(weight_knot.clone(), vec![p4w(0.0, 0.0, 0.0, 1.0); 6]);
    let section1 = BSplineCurve::new(weight_knot.clone(), vec![p4w(0.0, 0.0, 1.0, 0.05); 6]);
    let section2 = BSplineCurve::new(weight_knot, vec![p4w(0.0, 0.0, 2.0, 1.0); 6]);
    let sections = vec![section0, section1, section2];

    // The raw (pre-certification) strip loft net is NOT all-positive: this is
    // the fixture that forces the refinement path.
    let compatible = construct(make_compatible(&sections));
    let bands = construct(loft_collocation_bands(&STATIONS, V_DEGREE));
    let factor = construct(factor_banded_tp(&bands));
    let raw = construct(loft_sections(&compatible, &STATIONS, V_DEGREE, &factor));
    assert!(weight_net_min(&raw.surface) < 0.0); // H-3: the raw net forces refinement

    // The shipped strip (r = 1, no splits) carries the certified refined net.
    let built = construct(loft_closed_wire(&sections, &[], &STATIONS, V_DEGREE));
    assert_eq!(built.strips.len(), 1);
    assert!(built.seam_ids.is_empty());
    let shipped = &built.strips[0].surface;
    assert!(weight_net_min(shipped) > 0.0); // H-3

    // The shipped net admits on the free fast path with an EMPTY budget: the
    // refinements were applied to the shipped net. Had CC-012 shipped the raw
    // net, the empty-budget certification would refuse.
    let mut empty = Budget::new(0, 0, 0);
    let cert = construct(certify_weight_field(shipped, &mut empty));
    assert!(
        !cert.refined,
        "the shipped net admits without further subdivision"
    );
    assert!(
        cert.refinements.is_empty(),
        "the shipped net needs no further insertion"
    );
}

#[test]
fn open_loft_degenerates_to_single_strip() {
    // r = 1 (no splits): the strip machinery degenerates cleanly to a single
    // open-style strip — an ordinary loft of the full (compatible) sections
    // over the shared u basis, with no seam ids.
    let sections = closed_wire_sections();
    let built = construct(loft_closed_wire(&sections, &[], &STATIONS, V_DEGREE));
    assert_eq!(built.strips.len(), 1);
    assert!(built.seam_ids.is_empty());

    let compatible = construct(make_compatible(&sections));
    let strip = &built.strips[0];
    assert_eq!(strip.surface.uknot_vec(), compatible[0].knot_vec());

    // The single strip reproduces every section at its station up to the
    // delivered L2 enclosure width ε over a u sample grid.
    const SAMPLES: usize = 24;
    for (k, &station) in STATIONS.iter().enumerate() {
        let section = &compatible[k];
        let mut max_deviation = 0.0_f64;
        for i in 0..=SAMPLES {
            let u = (i as f64) / (SAMPLES as f64);
            let on_surface = strip.surface.subs(u, station);
            let on_section = section.subs(u);
            let deviation = distance(euclid4(on_surface), euclid4(on_section));
            if deviation > max_deviation {
                max_deviation = deviation;
            }
        }
        assert!(
            max_deviation <= strip.epsilon,
            "section {k} deviates {max_deviation} from its station curve, above the \
             delivered enclosure width {}",
            strip.epsilon
        ); // H-3
    }
}

#[test]
fn loft_closed_wire_refuses_invalid_splits_and_input() {
    // Invalid split selections refuse InvalidInput: an out-of-range index, an
    // index on the clamped boundary (a seam split would create a zero-length
    // arc), non-increasing indices, and a repeated knot value.
    let sections = closed_wire_sections();
    match loft_closed_wire(&sections, &[99], &STATIONS, V_DEGREE) {
        Err(ConstructRefusal::InvalidInput) => {}
        Ok(_) => panic!("an out-of-range split index must refuse as InvalidInput"),
        Err(other) => panic!("wrong refusal for an out-of-range split index: {other:?}"),
    }
    match loft_closed_wire(&sections, &[0], &STATIONS, V_DEGREE) {
        Err(ConstructRefusal::InvalidInput) => {}
        Ok(_) => panic!("a seam-boundary split must refuse as InvalidInput"),
        Err(other) => panic!("wrong refusal for a seam-boundary split: {other:?}"),
    }
    match loft_closed_wire(&sections, &[5, 4], &STATIONS, V_DEGREE) {
        Err(ConstructRefusal::InvalidInput) => {}
        Ok(_) => panic!("non-increasing split indices must refuse as InvalidInput"),
        Err(other) => panic!("wrong refusal for non-increasing split indices: {other:?}"),
    }
    match loft_closed_wire(&sections, &[4, 4], &STATIONS, V_DEGREE) {
        Err(ConstructRefusal::InvalidInput) => {}
        Ok(_) => panic!("a repeated split index must refuse as InvalidInput"),
        Err(other) => panic!("wrong refusal for a repeated split index: {other:?}"),
    }

    // Empty sections refuse InvalidInput (make_compatible's gate).
    match loft_closed_wire(&[], &[], &STATIONS, V_DEGREE) {
        Err(ConstructRefusal::InvalidInput) => {}
        Ok(_) => panic!("empty sections must refuse as InvalidInput"),
        Err(other) => panic!("wrong refusal for empty sections: {other:?}"),
    }
}
