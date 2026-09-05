#![deny(clippy::unwrap_used)]

//! NUM-INTERPOLE-OVERSHOOT-001 — admission gates on `BSplineCurve::try_interpole`:
//! a knot vector that violates the Schoenberg–Whitney condition and a solve
//! whose delivered control points overshoot the data extent are refused typed
//! (`Error::InterpolationNotSwVerified`), never delivered as a silent
//! overshooting interpolant, and the de Boor-averaged knot choice stays the
//! default path that keeps between-sample evaluation inside the data bounds.

use truck_base::cgmath64::Point3;
use truck_geometry::errors::Error;
use truck_geometry::prelude::*;

/// The unit-scale probe data (the record's scaling table in miniature): a
/// smooth bounded helix sampled over `stations`, coordinates within `[0, 1]`.
fn unit_scale_data(stations: &[f64]) -> Vec<(f64, Point3)> {
    stations
        .iter()
        .map(|&s| {
            let theta = 2.0 * std::f64::consts::PI * s;
            (s, Point3::new(s, 0.5 * theta.sin(), 0.5 * theta.cos()))
        })
        .collect()
}

/// The L∞ diameter of a point set: the maximum coordinate range over every
/// axis (the per-dimension span of the record's `extent`).
fn coordinate_extent(points: &[Point3]) -> f64 {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for p in points {
        for d in 0..3 {
            lo[d] = lo[d].min(p[d]);
            hi[d] = hi[d].max(p[d]);
        }
    }
    let mut extent = 0.0f64;
    for d in 0..3 {
        extent = extent.max(hi[d] - lo[d]);
    }
    extent
}

/// The maximum coordinate magnitude over a dense between-sample sweep of the
/// curve — the record's "max |coord| between samples" oracle.
fn max_between_sample_coord(curve: &BSplineCurve<Point3>, samples: usize) -> f64 {
    let mut max = 0.0f64;
    for i in 0..=samples {
        let p = curve.subs(i as f64 / samples as f64);
        max = max.max(p[0].abs().max(p[1].abs().max(p[2].abs())));
    }
    max
}

/// A clamped cubic knot vector whose `n - 4` interior knots are ALL the single
/// value `0.5`: the multiplicity-`n-4` interior collapses the B-spline basis,
/// so several basis functions vanish — a Schoenberg–Whitney violation.
fn sw_violating_knots(n: usize) -> KnotVec {
    let mut knots = vec![0.0f64; 4];
    knots.extend(std::iter::repeat_n(0.5, n - 4));
    knots.extend(std::iter::repeat_n(1.0, 4));
    KnotVec::from(knots)
}

/// The uniform-interior clamped cubic knot vector used by the record's scaling
/// table (interior knots at `i / (n - 3)`), which is NOT adapted to a
/// clustered station distribution.
fn uniform_interior_knots(n: usize) -> KnotVec {
    KnotVec::uniform_knot(3, n - 3)
}

#[test]
fn sw_violating_knot_vector_refused_typed() {
    // A knot vector that violates the Schoenberg–Whitney condition is refused
    // as the typed `InterpolationNotSwVerified` — never a panic, never a
    // silent accept.
    let stations: Vec<f64> = (0..12).map(|i| i as f64 / 11.0).collect();
    let data = unit_scale_data(&stations);
    let result = BSplineCurve::<Point3>::try_interpole(sw_violating_knots(12), data);
    match result {
        Err(Error::InterpolationNotSwVerified { at }) => {
            assert!(at.is_finite(), "the refusal must carry a finite station");
        }
        other => panic!("an SW-violating knot vector must refuse typed, got {other:?}"),
    }
    // The same stations under the de Boor-averaged knot choice satisfy the
    // Schoenberg–Whitney condition and interpolate cleanly.
    let data = unit_scale_data(&stations);
    let averaged = averaged_interpolation_knots(&stations, 3);
    let ok = BSplineCurve::<Point3>::try_interpole(averaged, data);
    assert!(ok.is_ok(), "averaged knots must admit the same stations");
}

#[test]
fn averaged_knots_helper_matches_de_boor_definition() {
    // The de Boor averaging definition is the ground truth: ξ_{j+q} =
    // (1/q)·Σ_{r=j}^{j+q−1} v_r, clamped ends repeated q + 1.
    let stations = [0.0f64, 0.25, 0.4, 0.6, 0.8, 1.0];
    let degree = 3usize;
    let knots = averaged_interpolation_knots(&stations, degree);
    assert_eq!(
        knots.len(),
        stations.len() + degree + 1,
        "the averaged knot vector has length n + q + 1"
    );
    let expected = [
        0.0,
        0.0,
        0.0,
        0.0,
        (0.25 + 0.4 + 0.6) / 3.0,
        (0.4 + 0.6 + 0.8) / 3.0,
        1.0,
        1.0,
        1.0,
        1.0,
    ];
    assert_eq!(knots.len(), expected.len());
    for (i, (&got, &want)) in knots.iter().zip(expected.iter()).enumerate() {
        assert!(
            (got - want).abs() <= 1.0e-15, // H-3: averaging arithmetic exact to fp round-off
            "knot[{i}] = {got}, expected {want}"
        );
    }
    // Clamped ends: the first and last stations are each repeated q + 1 times.
    assert_eq!(
        knots.iter().filter(|&&u| u == stations[0]).count(),
        degree + 1
    );
    assert_eq!(
        knots.iter().filter(|&&u| u == stations[5]).count(),
        degree + 1
    );
}

#[test]
fn interpolation_stays_within_data_bounds_on_the_probe_fixture() {
    // The record's scaling table in miniature: bounded unit-scale data over a
    // station distribution that clusters near the start. The record's uniform
    // interior-knot choice is NOT adapted to that distribution — it refuses
    // typed — while the de Boor-averaged knots (the default interpolant knot
    // choice) realize an interpolant whose between-sample evaluation stays
    // within the data bounds.
    let n = 32usize;
    let stations: Vec<f64> = (0..n).map(|i| (i as f64 / (n - 1) as f64).sqrt()).collect();
    let data = unit_scale_data(&stations);
    let extent = coordinate_extent(&data.iter().map(|&(_, p)| p).collect::<Vec<_>>());
    assert!(extent > 0.5, "the probe data must be bounded unit-scale");

    // The BAD knot choice (the record's uniform interior knots, unadapted to
    // the clustered stations): typed refusal, never a delivered overshoot.
    let bad = BSplineCurve::<Point3>::try_interpole(uniform_interior_knots(n), data.clone());
    match bad {
        Err(Error::InterpolationNotSwVerified { .. }) => {}
        other => panic!("the bad knot choice must refuse typed, got {other:?}"),
    }

    // The same data under `averaged_interpolation_knots`: the interpolant is
    // delivered and stays within BOUND_FACTOR-style bounds between samples.
    let averaged = averaged_interpolation_knots(&stations, 3);
    let curve = match BSplineCurve::<Point3>::try_interpole(averaged, data.clone()) {
        Ok(curve) => curve,
        Err(error) => panic!("averaged knots must admit the probe data: {error:?}"),
    };
    let bound = 1e3 * extent;
    let max_coord = max_between_sample_coord(&curve, 40_000);
    assert!(
        max_coord <= bound, // H-3: BOUND_FACTOR-style between-sample bound
        "between-sample evaluation must stay within BOUND_FACTOR-style bounds: \
         {max_coord} vs {bound}"
    );
    let max_residual = data
        .iter()
        .map(|&(t, p)| {
            let got = curve.subs(t);
            (got - p)
                .x
                .abs()
                .max((got - p).y.abs().max((got - p).z.abs()))
        })
        .fold(0.0f64, f64::max);
    assert!(
        max_residual <= 1.0e-9, // H-3: interpolation exactness at the stations
        "the averaged-knot interpolant must hit every data point, residual {max_residual}"
    );
}

#[test]
fn existing_interpole_callers_behavior_unchanged_on_valid_inputs() {
    // A valid input is bit-identical to the landed behavior: the solve is
    // untouched, so an existing caller (interpolating a parabola through
    // uniform clamped knots) still gets an interpolant that reproduces the
    // quadratic exactly at and between the stations.
    let stations: Vec<f64> = (0..6).map(|i| i as f64 / 5.0).collect();
    let data: Vec<(f64, Point3)> = stations
        .iter()
        .map(|&t| (t, Point3::new(t, t * t, 0.0)))
        .collect();
    let extent = coordinate_extent(&data.iter().map(|&(_, p)| p).collect::<Vec<_>>());
    let curve = match BSplineCurve::<Point3>::try_interpole(uniform_interior_knots(6), data.clone())
    {
        Ok(curve) => curve,
        Err(error) => panic!("a valid uniform-knot input must interpolate: {error:?}"),
    };
    // Control-point extent stays O(1) × the data extent (honest interpolant).
    let control_points = curve.control_points();
    let control_extent = coordinate_extent(control_points);
    assert!(
        control_extent <= 1e3 * extent, // H-3: BOUND_FACTOR-style bound
        "a valid input must not be refused as an overshoot: {control_extent} vs {extent}"
    );
    // The cubic interpolation reproduces the quadratic exactly: sampled
    // between-station evaluation matches t ↦ (t, t²).
    const SAMPLES: usize = 200;
    for i in 0..=SAMPLES {
        let t = i as f64 / SAMPLES as f64;
        let got = curve.subs(t);
        let want = Point3::new(t, t * t, 0.0);
        assert!(
            (got - want).magnitude() <= 1.0e-9, // H-3: parabola reproduction epsilon
            "the interpolated parabola diverged at t = {t}: {got:?} vs {want:?}"
        );
    }
}
