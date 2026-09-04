//! CC-002-INJECTIVITY conformance tests (spine seam S4): the P2 injectivity
//! radius `δ = 2σ/L` over real certified curve/surface maps, the flat-region
//! and degenerate-region conventions, and the monotonicity-under-refinement
//! gate. The test names are the contract.

#![deny(clippy::unwrap_used)]

use truck_certified::certified_map::{
    admit_curve, admit_surface, CertifiedCurveMap, CertifiedSurfaceMap,
};
use truck_certified::construct::fixtures as fx;
use truck_certified::construct::injectivity::{curve_injectivity_radius, injectivity_radius};
use truck_certified::construct::refusal::ConstructRefusal;
use truck_certified::formal::numeric::PositiveFinite;
use truck_geometry::prelude::{BSplineCurve, BSplineSurface, KnotVec, Point3};

/// A declared positive tau for the fixtures.
fn tau(value: f64) -> PositiveFinite {
    PositiveFinite::new(value).expect("a positive declared tau")
}

/// The bilinear plane patch `S(u, v) = (u, 2v, 0)` over `[0, 1]^2`: a flat patch
/// with certified margin `|S_u × S_v| = 2` and identically zero second partials
/// (the CC-002 flat-patch success path, `L = 0`, `δ = +∞`).
fn plane_surface() -> BSplineSurface<Point3> {
    let uknot = KnotVec::bezier_knot(1);
    let vknot = KnotVec::bezier_knot(1);
    let ctrl_pts = vec![
        vec![Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 2.0, 0.0)],
        vec![Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 2.0, 0.0)],
    ];
    BSplineSurface::new((uknot, vknot), ctrl_pts)
}

/// The curved patch `S(u, v) = (2u, 2v, u^2)` over `[0, 1]^2`. Ground truth:
/// `S_u × S_v = (-4u, 0, 4)` so `σ* = min |S_u × S_v| = 4` (at `u = 0`), and the
/// only nonzero second partial is `S_uu = (0, 0, 2)` so `L* = sup ‖D²S‖ = 2`;
/// the ideal radius is `δ* = 2σ*/L* = 4` exactly — the CC-000 `curved_patch`
/// ground truth (`sigma = (4, 5)`, `curvature_l = 2`, `expected_delta = 4`).
fn curved_surface() -> BSplineSurface<Point3> {
    let uknot = KnotVec::bezier_knot(2);
    let vknot = KnotVec::bezier_knot(1);
    let ctrl_pts = vec![
        vec![Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 2.0, 0.0)],
        vec![Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 2.0, 0.0)],
        vec![Point3::new(2.0, 0.0, 1.0), Point3::new(2.0, 2.0, 1.0)],
    ];
    BSplineSurface::new((uknot, vknot), ctrl_pts)
}

/// The flat straight-line curve `C(t) = (t, 0, 0)` over `[0, 1]`: `|C'| = 1`,
/// `C'' = 0`, so the 1-D flat path (`L = 0`, `δ = +∞`).
fn flat_line_curve() -> BSplineCurve<Point3> {
    let knot = KnotVec::bezier_knot(1);
    let points = vec![Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)];
    BSplineCurve::new(knot, points)
}

/// The unit-circle data curve (CC-000 `curved_patch`/circle data): the parabola
/// `C(t) = (t, t²/2)` over `[-1, 1]` reproduces the unit-circle P2 invariants
/// exactly — `C' = (1, t)` so `σ* = min |C'| = 1` (at `t = 0`), and `C'' =
/// (0, 1)` so `L* = sup ‖C″‖ = 1`; the ideal radius is `δ* = 2σ*/L* = 2`.
fn unit_circle_data_curve() -> BSplineCurve<Point3> {
    let knot = KnotVec::from(vec![-1.0, -1.0, -1.0, 1.0, 1.0, 1.0]);
    let points = vec![
        Point3::new(-1.0, 0.5, 0.0),
        Point3::new(0.0, -0.5, 0.0),
        Point3::new(1.0, 0.5, 0.0),
    ];
    BSplineCurve::new(knot, points)
}

/// Admit a surface fixture with the given declared tau, panicking only on a
/// test-bug refusal.
fn admitted_surface(surface: &BSplineSurface<Point3>, value: f64) -> CertifiedSurfaceMap {
    admit_surface(surface, tau(value)).expect("the surface fixture admits")
}

/// Admit a curve fixture with the given declared tau, panicking only on a
/// test-bug refusal.
fn admitted_curve(curve: &BSplineCurve<Point3>, value: f64) -> CertifiedCurveMap {
    admit_curve(curve, tau(value)).expect("the curve fixture admits")
}

#[test]
fn flat_patch_yields_infinite_radius() {
    // Surface variant: the bilinear plane has zero second partials, so L = 0
    // and the certified radius is the +infinity Interval (a flat patch has no
    // curvature-driven self-contact). Matches the CC-000 `flat_patch` ground
    // truth (`sigma > 0`, `L = 0`, `expected_delta = +infinity`).
    let surface = plane_surface();
    let map = admitted_surface(&surface, 0.5);
    let delta = injectivity_radius(&map, ((0.0, 1.0), (0.0, 1.0)))
        .expect("the flat patch certifies an infinite radius");
    assert!(
        delta.lo.is_infinite() && delta.hi.is_infinite(),
        "the flat-patch radius is the +inf Interval: {delta:?}"
    );
    assert!(delta.lo > 0.0, "the infinite radius is positively infinite");

    // Curve variant: the straight unit-speed line has C'' = 0, so L = 0 and the
    // 1-D radius is +infinity too.
    let curve = flat_line_curve();
    let cmap = admitted_curve(&curve, 0.5);
    let cdelta = curve_injectivity_radius(&cmap, (0.0, 1.0))
        .expect("the flat line certifies an infinite radius");
    assert!(
        cdelta.lo.is_infinite() && cdelta.hi.is_infinite(),
        "the flat-line radius is the +inf Interval: {cdelta:?}"
    );

    // The §6 flat_patch fixture agrees with the certified convention.
    let flat = fx::flat_patch().expect("the flat-patch fixture data is valid");
    assert_eq!(flat.curvature_l, 0.0);
    assert!(flat.expected_delta.is_infinite(), "2 sigma / 0 diverges");
}

#[test]
fn curved_patch_radius_is_a_certified_lower_bound() {
    // The curved patch has ideal values sigma* = 4, L* = 2, delta* = 4. The
    // certified computation must return an interval whose lower endpoint is a
    // certified lower bound of delta* (never above 4 -- a sup/inf swap in the
    // margin or the curvature bound would overclaim) yet close to it.
    let surface = curved_surface();
    let map = admitted_surface(&surface, 1.0);
    let margin = map
        .rank_margin(((0.0, 1.0), (0.0, 1.0)))
        .expect("the curved patch certifies its rank margin");
    assert!(
        margin.lo > 1.0,
        "the certified margin is comfortably above the declared tau"
    );

    let delta = injectivity_radius(&map, ((0.0, 1.0), (0.0, 1.0)))
        .expect("the curved patch certifies a finite radius");
    assert!(
        delta.lo.is_finite() && delta.hi.is_finite(),
        "the curved-patch radius is finite: {delta:?}"
    );
    assert!(delta.lo > 0.0, "a certified radius is strictly positive");
    assert!(
        delta.lo <= 4.0,
        "the certified lower bound never exceeds the ideal 2 sigma* / L* = 4: delta.lo = {}",
        delta.lo
    );
    assert!(
        4.0 - delta.lo < 1e-6, // H-3: certified bound tight to the dyadic ideal within enclosure width
        "the certified lower bound is tight: delta.lo = {}, ideal = 4",
        delta.lo
    );

    // The §6 curved_patch fixture records the same dyadic ground truth.
    let curved = fx::curved_patch().expect("the curved-patch fixture data is valid");
    assert_eq!(curved.sigma.0, 4.0);
    assert_eq!(curved.curvature_l, 2.0);
    assert_eq!(curved.expected_delta, 4.0);
}

#[test]
fn degenerate_patch_refuses_invalid_input() {
    // A non-compact or misordered region is a degenerate request: the certified
    // radius must refuse InvalidInput, never return a delta of 0 as a success
    // and never conflate the input defect with the map's own admit-time
    // ParameterizationDegenerate refusal.
    let surface = plane_surface();
    let map = admitted_surface(&surface, 0.5);
    for sub in [
        ((-1.0, 1.5), (0.0, 1.0)),
        ((0.5, 0.4), (0.0, 1.0)),
        ((f64::NAN, 0.5), (0.0, 1.0)),
        ((0.0, 1.0), (2.0, 1.0)),
    ] {
        assert!(
            matches!(
                injectivity_radius(&map, sub),
                Err(ConstructRefusal::InvalidInput)
            ),
            "surface radius over the degenerate region {sub:?} refuses InvalidInput"
        );
    }

    // Curve variant: the same refusal vocabulary over a degenerate subinterval.
    let curve = unit_circle_data_curve();
    let cmap = admitted_curve(&curve, 0.5);
    for sub in [(-2.0, 0.5), (0.5, 0.2), (f64::NAN, 0.5)] {
        assert!(
            matches!(
                curve_injectivity_radius(&cmap, sub),
                Err(ConstructRefusal::InvalidInput)
            ),
            "curve radius over the degenerate subinterval {sub:?} refuses InvalidInput"
        );
    }

    // The §6 degenerate_patch fixture records the refusal ground truth: a sigma
    // enclosure strictly containing 0 (no admissible radius, never delta = 0).
    let degenerate = fx::degenerate_patch().expect("the degenerate fixture data is valid");
    assert!(
        degenerate.sigma.0 <= 0.0 && degenerate.sigma.1 >= 0.0,
        "sigma contains 0"
    );
    assert!(degenerate.expected_delta <= 0.0, "no admissible radius");
}

#[test]
fn curve_radius_on_unit_circle_matches_two() {
    // Unit-circle data: sigma* = 1, ||C''|| = 1, so the ideal radius is
    // delta* = 2 sigma* / L* = 2. The certified interval must sit below 2 (a
    // certified lower bound) yet within certified enclosure width of it.
    let curve = unit_circle_data_curve();
    let map = admitted_curve(&curve, 0.5);
    let delta =
        curve_injectivity_radius(&map, (-1.0, 1.0)).expect("the circle-data curve certifies");
    assert!(
        delta.lo.is_finite() && delta.hi.is_finite(),
        "the circle-data radius is finite: {delta:?}"
    );
    assert!(delta.lo > 0.0, "a certified radius is strictly positive");
    assert!(
        delta.lo <= 2.0,
        "the certified lower bound never exceeds the ideal 2 sigma* / L* = 2: delta.lo = {}",
        delta.lo
    );
    assert!(
        2.0 - delta.lo < 1e-6, // H-3: certified bound within enclosure width of the ideal 2
        "the certified radius matches the unit-circle delta of 2: delta.lo = {}",
        delta.lo
    );
    assert!(delta.hi >= delta.lo, "the returned interval is ordered");
}

#[test]
fn radius_shrinks_monotonically_under_region_refinement() {
    // Splitting a region in half must never increase the certified delta over
    // either half beyond the parent's delta (up to certified enclosure width).
    // The fixtures below keep the certified (sigma, L) data constant across the
    // tested splits -- the circle-data curve split at its sigma-min locus t = 0,
    // and the curved patch split along v (both margin and curvature are
    // v-independent, and every tested half retains the u = 0 minimum-margin
    // line) -- so a sound implementation returns parent and child deltas equal
    // up to rounding, while an unsound sup/inf swap would push a child above
    // the parent.

    // Curve: parent [-1, 1] split at t = 0; each half retains the sigma-min
    // point, so the certified delta over either half must not exceed the
    // parent's.
    let curve = unit_circle_data_curve();
    let cmap = admitted_curve(&curve, 0.5);
    let parent =
        curve_injectivity_radius(&cmap, (-1.0, 1.0)).expect("the parent curve region certifies");
    for half in [(-1.0, 0.0), (0.0, 1.0)] {
        let child = curve_injectivity_radius(&cmap, half).expect("a curve half certifies a radius");
        assert!(
            child.lo <= parent.lo + 1e-6, // H-3: never increase beyond the parent's delta (enclosure width)
            "curve half {half:?} delta {} exceeds parent delta {} by more than enclosure width",
            child.lo,
            parent.lo
        );
    }

    // Surface: the curved patch split along v into halves and quarters; every
    // subregion keeps the full u-range, so the certified (sigma, L) data is
    // unchanged and each child delta must not exceed the parent's.
    let surface = curved_surface();
    let smap = admitted_surface(&surface, 1.0);
    let sparent = injectivity_radius(&smap, ((0.0, 1.0), (0.0, 1.0)))
        .expect("the parent surface region certifies");
    for sub in [
        ((0.0, 1.0), (0.0, 0.5)),
        ((0.0, 1.0), (0.5, 1.0)),
        ((0.0, 1.0), (0.0, 0.25)),
        ((0.0, 1.0), (0.25, 0.5)),
        ((0.0, 1.0), (0.5, 0.75)),
        ((0.0, 1.0), (0.75, 1.0)),
        ((0.0, 0.5), (0.0, 1.0)),
    ] {
        let child = injectivity_radius(&smap, sub).expect("a surface subregion certifies a radius");
        assert!(
            child.lo <= sparent.lo + 1e-6, // H-3: never increase beyond the parent's delta (enclosure width)
            "surface subregion {sub:?} delta {} exceeds parent delta {} by more than enclosure width",
            child.lo,
            sparent.lo
        );
    }
}
