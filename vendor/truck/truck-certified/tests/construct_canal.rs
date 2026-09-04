//! CC-025-CANAL conformance tests (spine seam S10): the radius-law production
//! evaluators over the CC-000 `RadiusLaw` stub and the closed-form
//! arc-restricted canal regularity criterion over real certified spine maps
//! (`docs/CERTIFIED_LOFT_AND_SHELL_THEORY_SPEC.md` §6). The test names are the
//! contract.

#![deny(clippy::unwrap_used)]

use truck_certified::certified_map::{admit_curve, CertifiedCurveMap};
use truck_certified::construct::canal::{
    canal_regularity, canal_regularity_closed_pipe, radius_derivs, radius_eval,
};
use truck_certified::construct::refusal::ConstructRefusal;
use truck_certified::construct::stubs::RadiusLaw;
use truck_certified::construct::Interval;
use truck_certified::formal::numeric::PositiveFinite;
use truck_geometry::prelude::{BSplineCurve, KnotVec, Point3};

/// A declared positive tau for the fixtures.
fn tau(value: f64) -> PositiveFinite {
    PositiveFinite::new(value).expect("a positive declared tau")
}

/// Admit a curve fixture with the given declared tau, panicking only on a
/// test-bug refusal.
fn admitted_curve(curve: &BSplineCurve<Point3>, value: f64) -> CertifiedCurveMap {
    admit_curve(curve, tau(value)).expect("the curve fixture admits")
}

/// The unit-speed straight spine `C(t) = (t, 0, 0)` over `[0, 1]`: `|C'| = 1`
/// and `C'' = 0`, so any admissible radius law over the unit arc is regular
/// (the slope gate is the only possible refusal).
fn straight_line_spine() -> BSplineCurve<Point3> {
    let knot = KnotVec::bezier_knot(1);
    let points = vec![Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)];
    BSplineCurve::new(knot, points)
}

/// The unit-circle data spine (the CC-000 `curved_patch`/circle data, as used
/// by the CC-002 unit-circle test): the parabola `C(t) = (t, t²/2)` over
/// `[-1, 1]` reproduces the unit-circle invariants exactly — `σ* = min |C'| =
/// 1` (at `t = 0`) and `‖C″‖ = 1` on the whole arc — so the pipe condition for
/// a constant radius is `r·‖C″‖ = r < 1`.
fn unit_circle_data_spine() -> BSplineCurve<Point3> {
    let knot = KnotVec::from(vec![-1.0, -1.0, -1.0, 1.0, 1.0, 1.0]);
    let points = vec![
        Point3::new(-1.0, 0.5, 0.0),
        Point3::new(0.0, -0.5, 0.0),
        Point3::new(1.0, 0.5, 0.0),
    ];
    BSplineCurve::new(knot, points)
}

/// The cubic spine `C(t) = (t, t³)` over `[0, 1]`: `|C'| = √(1 + 9t⁴) ≥ 1` and
/// `‖C″‖ = 6t` varies from `0` at `t = 0` to `6` at `t = 1`, so the pipe
/// condition `r·‖C″‖ < 1` holds only on a sub-arc near `t = 0` for a radius
/// like `r = 0.5`. This is the fixture that observes the arc-restriction gap.
fn varying_curvature_spine() -> BSplineCurve<Point3> {
    let knot = KnotVec::from(vec![0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0]);
    let points = vec![
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0 / 3.0, 0.0, 0.0),
        Point3::new(2.0 / 3.0, 0.0, 0.0),
        Point3::new(1.0, 1.0, 0.0),
    ];
    BSplineCurve::new(knot, points)
}

#[test]
fn constant_radius_unit_circle_satisfies_pipe_condition() {
    // The unit-circle spine data carries σ = 1 and ‖C″‖ = 1. A constant radius
    // r = 0.5 satisfies the pipe condition r·‖C″‖ = 0.5 < 1, so the
    // arc-restricted criterion must certify regularity with a strictly
    // positive margin. The closed form reduces to `min |a² − rq| − max r·a·‖C″‖
    // = 1 − 0.5 = 0.5` (constant radius: `p = q = 0`, `a = 1`), so the returned
    // enclosure's lower endpoint sits within certified enclosure width of 0.5.
    let spine = unit_circle_data_spine();
    let map = admitted_curve(&spine, 0.5);
    let enclosure = canal_regularity(&map, &RadiusLaw::Constant(0.5), (-1.0, 1.0))
        .expect("the constant-radius unit-circle pipe certifies regular");
    assert!(
        enclosure.lo > 0.0,
        "the certified regularity margin is strictly positive: {enclosure:?}"
    );
    assert!(
        enclosure.lo <= 0.5 + 1e-9, // H-3: certified margin never overclaims the ideal 1 - r = 0.5
        "the certified margin is at most the ideal 1 - r = 0.5: enclosure.lo = {}",
        enclosure.lo
    );
    assert!(
        0.5 - enclosure.lo < 1e-6, // H-3: certified margin tight to the dyadic ideal within enclosure width
        "the certified margin matches the ideal 1 - r = 0.5: enclosure.lo = {}",
        enclosure.lo
    );

    // The closed-pipe form agrees: a constant-radius pipe over the whole arc is
    // regular whenever the pipe condition holds on the whole loop.
    let closed = canal_regularity_closed_pipe(&map, &RadiusLaw::Constant(0.5))
        .expect("the constant-radius unit-circle closed pipe certifies regular");
    assert!(closed.lo > 0.0, "the closed-pipe margin is positive: {closed:?}");
}

#[test]
fn radius_law_slope_at_or_above_one_refuses_canal_singular() {
    // A radius law whose |r'| enclosure reaches 1 anywhere degenerates the
    // characteristic circle (theory §6.1 first gate: a = √(1 − r'²) = 0), so
    // both evaluators and the seam refuse CanalSingular immediately.
    let unit = Interval { lo: 0.0, hi: 1.0 };
    assert!(
        matches!(
            radius_eval(&RadiusLaw::Linear { r0: 0.0, r1: 1.0 }, unit),
            Err(ConstructRefusal::CanalSingular)
        ),
        "a slope of exactly one over the arc refuses at the first gate"
    );
    assert!(
        matches!(
            radius_derivs(&RadiusLaw::Linear { r0: 0.0, r1: 2.0 }, unit),
            Err(ConstructRefusal::CanalSingular)
        ),
        "a slope above one over the arc refuses at the first gate"
    );
    assert!(
        matches!(
            radius_eval(&RadiusLaw::Linear { r0: 0.5, r1: 1.5 }, Interval { lo: 0.0, hi: 0.5 }),
            Err(ConstructRefusal::CanalSingular)
        ),
        "the gate fires on any sub-interval where the constant slope reaches one"
    );
    // A cubic profile whose interior slope exceeds one is caught by the hull of
    // the derivative over the full arc.
    assert!(
        matches!(
            radius_derivs(
                &RadiusLaw::CubicHermite {
                    r0: 0.0,
                    r1: 2.0,
                    m0: 0.0,
                    m1: 0.0
                },
                unit
            ),
            Err(ConstructRefusal::CanalSingular)
        ),
        "a cubic with |r'| peaking above one over the arc refuses at the first gate"
    );

    // A slope strictly inside (-1, 1) is admissible: the gate is at one, not
    // before it.
    let ok = radius_eval(
        &RadiusLaw::Linear {
            r0: 0.25,
            r1: 0.75,
        },
        unit,
    )
    .expect("a slope of 0.5 is admissible");
    assert!(ok.contains(0.5), "the radius stays positive and evaluated");

    // The seam refuses the degenerate slope through the same gate.
    let map = admitted_curve(&straight_line_spine(), 0.5);
    let law = RadiusLaw::Linear { r0: 0.0, r1: 1.0 };
    assert!(
        matches!(
            canal_regularity(&map, &law, (0.0, 1.0)),
            Err(ConstructRefusal::CanalSingular)
        ),
        "canal_regularity refuses a slope-at-one radius law"
    );
    assert!(
        matches!(
            canal_regularity_closed_pipe(&map, &law),
            Err(ConstructRefusal::CanalSingular)
        ),
        "the closed-pipe form refuses the same degenerate law"
    );
}

#[test]
fn singular_spine_fixture_refuses_canal_singular() {
    // A spine whose pipe condition fails — r·‖C″‖ >= 1 somewhere on the arc —
    // is a singular canal: the criterion value is at or below zero and the seam
    // refuses CanalSingular (no interval-Jacobian fallback tier exists). On the
    // unit-circle data ‖C″‖ = 1, so r = 1 is the borderline and r = 1.5 is
    // clearly singular.
    let spine = unit_circle_data_spine();
    let map = admitted_curve(&spine, 0.5);
    for radius in [1.0, 1.5] {
        assert!(
            matches!(
                canal_regularity(&map, &RadiusLaw::Constant(radius), (-1.0, 1.0)),
                Err(ConstructRefusal::CanalSingular)
            ),
            "constant radius {radius} with r*||C''|| >= 1 refuses CanalSingular"
        );
        assert!(
            matches!(
                canal_regularity_closed_pipe(&map, &RadiusLaw::Constant(radius)),
                Err(ConstructRefusal::CanalSingular)
            ),
            "the closed-pipe form refuses constant radius {radius} on the singular spine"
        );
    }

    // A locally-constructed singular fixture: r·‖C″‖ = 0.5 · 6 = 3 >= 1 at the
    // t = 1 end of the (t, t³) spine.
    let cubic = varying_curvature_spine();
    let cmap = admitted_curve(&cubic, 0.5);
    assert!(
        matches!(
            canal_regularity_closed_pipe(&cmap, &RadiusLaw::Constant(0.5)),
            Err(ConstructRefusal::CanalSingular)
        ),
        "the full (t, t^3) pipe with 0.5 * sup||C''|| = 3 refuses CanalSingular"
    );
}

#[test]
fn arc_restriction_is_strictly_more_permissive_than_all_theta() {
    // The (t, t³) spine has ‖C″‖ = 6t rising from 0 to 6 over [0, 1]. A
    // constant radius r = 0.5 fails the pipe condition on the whole loop
    // (0.5 · 6 = 3 >= 1), so the closed pipe refuses. The arc-restricted form
    // certifies only the spine sub-arc the patch actually occupies: over
    // [0, 0.1] the curvature bound is ~0.6 and r·‖C″‖ < 1, so a blend patch
    // restricted to that arc certifies regular. The permissiveness gap is
    // observed, not assumed.
    let spine = varying_curvature_spine();
    let map = admitted_curve(&spine, 0.5);
    let law = RadiusLaw::Constant(0.5);

    let arc_enclosure = canal_regularity(&map, &law, (0.0, 0.1))
        .expect("the sub-arc near t = 0 certifies regular");
    assert!(
        arc_enclosure.lo > 0.0,
        "the arc-restricted margin is strictly positive: {arc_enclosure:?}"
    );

    let whole = canal_regularity(&map, &law, (0.0, 1.0));
    assert!(
        matches!(whole, Err(ConstructRefusal::CanalSingular)),
        "the same radius over the whole arc is singular and refuses"
    );
    let closed = canal_regularity_closed_pipe(&map, &law);
    assert!(
        matches!(closed, Err(ConstructRefusal::CanalSingular)),
        "the all-theta closed pipe refuses where the arc-restricted patch certifies"
    );
}

#[test]
fn radius_law_evaluation_matches_declared_law() {
    // Constant(c) → (c, 0) over any sub-interval.
    let constant = RadiusLaw::Constant(0.75);
    let r = radius_eval(&constant, Interval { lo: 0.0, hi: 1.0 })
        .expect("the constant law evaluates");
    assert!(r.contains(0.75), "constant radius evaluates to c");
    let (rv, rp) = radius_derivs(&constant, Interval { lo: 0.0, hi: 1.0 })
        .expect("the constant law derives");
    assert!(rv.contains(0.75), "constant radius value");
    assert!(rp.contains(0.0), "constant radius slope is zero");

    // Linear { r0, r1 } → r(u) = r0 + (r1 - r0)·u with constant slope
    // r1 - r0 over the declared arc; on [0.25, 0.5] the value spans [0.6, 0.7].
    let linear = RadiusLaw::Linear {
        r0: 0.5,
        r1: 0.9,
    };
    let band = Interval {
        lo: 0.25,
        hi: 0.5,
    };
    let r = radius_eval(&linear, band).expect("the linear law evaluates");
    assert!(r.contains(0.6) && r.contains(0.7), "linear interpolation band");
    let (rv, rp) = radius_derivs(&linear, band).expect("the linear law derives");
    assert!(rv.contains(0.6) && rv.contains(0.7), "linear value band");
    assert!(rp.contains(0.4), "linear slope is r1 - r0 = 0.4");

    // CubicHermite: with r0 = r1 = 0.5, m0 = 0.3, m1 = -0.3 the profile is
    // r(u) = 0.5 + 0.3·(u - u²); at u = 0.25 the value is 0.55625 and the slope
    // is 0.3·(1 - 2u) = 0.15 (hand-computed).
    let cubic = RadiusLaw::CubicHermite {
        r0: 0.5,
        r1: 0.5,
        m0: 0.3,
        m1: -0.3,
    };
    let (rv, rp) = radius_derivs(&cubic, Interval::point(0.25)).expect("the cubic law derives");
    assert!(
        rv.lo <= 0.55625 && 0.55625 <= rv.hi, // H-3: hand-computed Hermite value
        "the cubic value at u = 0.25 is 0.55625: {rv:?}"
    );
    assert!(
        rp.lo <= 0.15 && 0.15 <= rp.hi, // H-3: hand-computed Hermite slope
        "the cubic slope at u = 0.25 is 0.15: {rp:?}"
    );

    // MonotoneCubic / VertexControl: collinear dyadic data (radii 0.5, 0.7, 0.9
    // at the uniform stations 0, 0.5, 1) has chord slope 0.4 throughout, so the
    // monotone cubic reproduces the line exactly: at u = 0.25 the value is 0.6
    // and the slope is 0.4, and the station radii are reproduced exactly.
    let mono = RadiusLaw::MonotoneCubic(vec![(0.0, 0.5), (0.5, 0.7), (1.0, 0.9)]);
    let vertex = RadiusLaw::VertexControl(vec![0.5, 0.7, 0.9]);
    for law in [&mono, &vertex] {
        let (rv, rp) = radius_derivs(law, Interval::point(0.25)).expect("the monotone cubic derives");
        assert!(
            rv.lo <= 0.6 && 0.6 <= rv.hi, // H-3: collinear monotone cubic tracks the line
            "the monotone cubic value at u = 0.25 is 0.6: {rv:?}"
        );
        assert!(
            rp.lo <= 0.4 && 0.4 <= rp.hi, // H-3: collinear monotone cubic slope is the chord slope
            "the monotone cubic slope at u = 0.25 is 0.4: {rp:?}"
        );
        for (u, station_radius) in [(0.0, 0.5), (0.5, 0.7), (1.0, 0.9)] {
            let at_station =
                radius_eval(law, Interval::point(u)).expect("the monotone cubic evaluates");
            assert!(
                at_station.lo <= station_radius && station_radius <= at_station.hi, // H-3: station reproduction
                "the monotone cubic reproduces the station radius {station_radius} at u = {u}: {at_station:?}"
            );
        }
    }
}
