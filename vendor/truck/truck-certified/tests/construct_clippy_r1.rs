//! CC-CLIPPY-R1 smoke artifact: the battery blocker is clippy debt in the
//! landed construct modules (banded.rs, residual_solve.rs, loft.rs, canal.rs).
//! This test file constructs nothing new; it exists so the packet has a test
//! artifact — one smoke test asserting the four modules still expose their
//! seam functions, including the seams whose internals carry the A1/A2
//! anchors (the fixed-order band recurrence of `solve_homogeneous` and the
//! radius-positivity gate of `canal_regularity`).

#![deny(clippy::unwrap_used)]

use truck_certified::certified_map::admit_curve;
use truck_certified::construct::banded::factor_banded_tp;
use truck_certified::construct::canal::canal_regularity;
use truck_certified::construct::fixtures as fx;
use truck_certified::construct::loft::averaged_knot_vector;
use truck_certified::construct::refusal::ConstructRefusal;
use truck_certified::construct::residual_solve::residual_solve_dense;
use truck_certified::construct::stubs::RadiusLaw;
use truck_certified::construct::Interval;
use truck_certified::formal::numeric::PositiveFinite;
use truck_geometry::prelude::{BSplineCurve, KnotVec, Point3};

/// Extract the `Ok` of a fallible construction; the smoke fixtures are valid
/// by construction, so the refusal arm is a test-bug panic (never an unwrap).
fn into_ok<T>(result: Result<T, ConstructRefusal>) -> T {
    match result {
        Ok(value) => value,
        Err(refusal) => panic!("a smoke construction that must succeed was refused: {refusal:?}"),
    }
}

/// The unit-speed straight spine `C(t) = (t, 0, 0)` over `[0, 1]`: any
/// constant radius law is regular over it (curvature is exactly zero).
fn straight_line_spine() -> BSplineCurve<Point3> {
    let knot = KnotVec::bezier_knot(1);
    let points = vec![Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)];
    BSplineCurve::new(knot, points)
}

#[test]
fn clippy_r1_modules_still_expose_their_seam_functions() {
    // The A1 seam (banded.rs): the fixed-order band recurrence retained in
    // `solve_homogeneous`, reached through `factor_banded_tp`.
    let fixture = into_ok(fx::banded_cubic_uniform(4));
    assert_eq!(fixture.size, 5);
    let factor = into_ok(factor_banded_tp(&fixture.bands));
    let rhs: Vec<[Interval; 4]> = (0..fixture.size)
        .map(|_| [Interval::point(1.0); 4])
        .collect();
    let rows = into_ok(factor.solve_homogeneous(&rhs));
    assert_eq!(rows.len(), fixture.size);

    // The residual_solve.rs seam: the dense fallback still certifies.
    let a = [
        [Interval::point(2.0), Interval::point(1.0)],
        [Interval::point(1.0), Interval::point(2.0)],
    ];
    let r_inv = [[2.0 / 3.0, -1.0 / 3.0], [-1.0 / 3.0, 2.0 / 3.0]];
    let x_hat = [1.0_f64, 2.0];
    let b = [Interval::point(4.0), Interval::point(5.0)];
    let enclosure = into_ok(residual_solve_dense(&a, &r_inv, &x_hat, &b));
    assert!(enclosure[0].contains(1.0));
    assert!(enclosure[1].contains(2.0));

    // The loft.rs seam: the de Boor-averaged station knot vector still runs
    // its interior accumulation in station order. Four stations at degree two
    // give one interior knot `(0.25 + 0.5) / 2 = 0.375`, exactly.
    let stations = [0.0_f64, 0.25, 0.5, 1.0];
    let knots = averaged_knot_vector(&stations, 2);
    assert_eq!(knots.len(), stations.len() + 2 + 1);
    assert_eq!(knots[3], 0.375); // H-3
    assert_eq!(knots[knots.len() - 1], 1.0); // H-3

    // The A2 seam (canal.rs): the radius-positivity gate of the regularity
    // criterion still certifies a strictly positive margin on a straight
    // spine with a constant radius law.
    let map = admit_curve(
        &straight_line_spine(),
        PositiveFinite::new(0.5).expect("a positive tau"),
    )
    .expect("the straight spine admits");
    let enclosure = canal_regularity(&map, &RadiusLaw::Constant(0.5), (0.0, 1.0))
        .expect("the constant-radius straight pipe certifies regular");
    assert!(
        enclosure.lo > 0.0,
        "the certified margin is positive: {enclosure:?}"
    );
}
