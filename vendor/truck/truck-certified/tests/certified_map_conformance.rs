//! Conformance tests for BG-CK-P1-MAP: class-1 CertifiedMap admission, the
//! enclosure oracle, and the rank margin (curve and surface), plus the
//! refusal vocabulary. The test names are the contract.

#![deny(clippy::unwrap_used)]

use truck_certified::certified_map::{
    admit_curve, admit_curve_region, admit_surface, admit_surface_region, MapRefusal,
};
use truck_certified::formal::numeric::PositiveFinite;
use truck_geometry::base::{Cut, InnerSpace, ParametricCurve, ParametricSurface};
use truck_geometry::prelude::{BSplineCurve, BSplineSurface, KnotVec, Point3};

/// A declared positive tau for the fixtures.
fn tau(value: f64) -> PositiveFinite {
    PositiveFinite::new(value).expect("a positive declared tau")
}

/// A straight-line degree-1 B-spline over `[0, 1]` with two Bézier pieces of
/// width `1/2`: `C(t) = (2t, 2t, 0)`, so `|C'| = 2 sqrt(2)` constantly.
fn line_curve() -> BSplineCurve<Point3> {
    let knot_vec = KnotVec::from(vec![0.0, 0.0, 0.5, 1.0, 1.0]);
    let ctrl_pts = vec![
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 1.0, 0.0),
        Point3::new(2.0, 2.0, 0.0),
    ];
    BSplineCurve::new(knot_vec, ctrl_pts)
}

/// A degree-2 Bézier over `[0, 1]` whose derivative `2(2u - 1)` vanishes at
/// `u = 0.5`: `C(u) = ((1 - u)^2 + u^2, 0, 0)`.
fn vanish_curve() -> BSplineCurve<Point3> {
    let knot_vec = KnotVec::bezier_knot(2);
    let ctrl_pts = vec![
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
    ];
    BSplineCurve::new(knot_vec, ctrl_pts)
}

/// A bilinear plane patch over `[0, 1] x [0, 1]`: `S(u, v) = (u, 2v, 0)`, so
/// `Sᵤ = (1, 0, 0)`, `Sᵥ = (0, 2, 0)` and `|Sᵤ × Sᵥ| = 2` constantly.
fn plane_surface() -> BSplineSurface<Point3> {
    let uknot = KnotVec::bezier_knot(1);
    let vknot = KnotVec::bezier_knot(1);
    let ctrl_pts = vec![
        vec![Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 2.0, 0.0)],
        vec![Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 2.0, 0.0)],
    ];
    BSplineSurface::new((uknot, vknot), ctrl_pts)
}

/// A degree-1 surface over `u in [0, 2]` (two `u`-pieces, one `v`-piece):
/// `S(u, v) = (u, v, u·v)`. `Sᵤ × Sᵥ = (-v, -u, 1)`, so
/// `|Sᵤ × Sᵥ| = sqrt(u² + v² + 1)` with minimum 1 at `(u, v) = (0, 0)`.
fn two_piece_surface() -> BSplineSurface<Point3> {
    let uknot = KnotVec::from(vec![0.0, 0.0, 1.0, 2.0, 2.0]);
    let vknot = KnotVec::bezier_knot(1);
    let ctrl_pts = vec![
        vec![Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0)],
        vec![Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0)],
        vec![Point3::new(2.0, 0.0, 0.0), Point3::new(2.0, 1.0, 2.0)],
    ];
    BSplineSurface::new((uknot, vknot), ctrl_pts)
}

/// The same ruled surface as `two_piece_surface`, but as a SINGLE `u`-piece of
/// width 2 over `u in [0, 2]` — the non-unit width exercises the inverse-width
/// scaling of the first-derivative patches.
fn non_unit_width_surface() -> BSplineSurface<Point3> {
    let uknot = KnotVec::from(vec![0.0, 0.0, 2.0, 2.0]);
    let vknot = KnotVec::bezier_knot(1);
    let ctrl_pts = vec![
        vec![Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0)],
        vec![Point3::new(2.0, 0.0, 0.0), Point3::new(2.0, 1.0, 2.0)],
    ];
    BSplineSurface::new((uknot, vknot), ctrl_pts)
}

/// Plain `f64` 1-D de Casteljau evaluation, the test's reference arithmetic.
fn eval_bernstein_1d(coeffs: &[f64], u: f64) -> f64 {
    let mut level: Vec<f64> = coeffs.to_vec();
    while level.len() > 1 {
        level = level
            .windows(2)
            .map(|w| (1.0 - u) * w[0] + u * w[1])
            .collect();
    }
    level[0]
}

/// Plain `f64` tensor-Bernstein evaluation of `c[i][j] B^i_m(s) B^j_n(t)`.
fn eval_bernstein_2d(grid: &[Vec<f64>], s: f64, t: f64) -> f64 {
    let row_evals: Vec<f64> = grid.iter().map(|row| eval_bernstein_1d(row, t)).collect();
    eval_bernstein_1d(&row_evals, s)
}

#[test]
fn curve_map_admission_certifies_rank_above_tau() {
    let curve = line_curve();
    let tau = tau(1.0);
    let map = admit_curve(&curve, tau).expect("the straight-ish line admits");
    let margin = map
        .rank_margin((0.0, 1.0))
        .expect("whole-domain rank margin");
    assert!(
        margin.lo > tau.get(),
        "the certified margin must sit above the declared tau: margin.lo = {}, tau = {}",
        margin.lo,
        tau.get()
    );
    let analytic = 2.0 * std::f64::consts::SQRT_2;
    let slack = 16.0 * f64::EPSILON * (1.0 + analytic);
    assert!(
        (margin.lo - analytic).abs() <= slack,
        "the margin lower bound must be within a few ulps of the analytic |C'| = 2 sqrt(2): margin.lo = {}, analytic = {}",
        margin.lo,
        analytic
    ); // H-3
}

#[test]
fn surface_map_admission_certifies_rank_above_tau() {
    let surface = plane_surface();
    let tau = tau(1.0);
    let map = admit_surface(&surface, tau).expect("the plane patch admits");
    let margin = map
        .rank_margin(((0.0, 1.0), (0.0, 1.0)))
        .expect("whole-domain rank margin");
    assert!(
        margin.lo > tau.get(),
        "the certified margin must sit above the declared tau"
    );
    let analytic = 2.0;
    let slack = 16.0 * f64::EPSILON * (1.0 + analytic);
    assert!(
        (margin.lo - analytic).abs() <= slack,
        "the margin lower bound must be within a few ulps of the analytic |Sᵤ × Sᵥ| = 2: margin.lo = {}, analytic = {}",
        margin.lo,
        analytic
    ); // H-3
}

#[test]
fn degenerate_parameterization_refuses_named_case() {
    let tau = tau(0.3);
    let curve = vanish_curve();
    // The whole declared domain is a region containing the vanish at t = 0.5.
    assert!(
        matches!(
            admit_curve(&curve, tau),
            Err(MapRefusal::ParameterizationDegenerate)
        ),
        "admission over a region containing the vanish must refuse"
    );
    // Per-region remedy: split off sub-regions and admit each.
    let mut curve = curve;
    let right = curve.cut(0.4);
    assert!(
        matches!(
            admit_curve(&right, tau),
            Err(MapRefusal::ParameterizationDegenerate)
        ),
        "the [0.4, 1.0] sub-region still contains the vanish at t = 0.5"
    );
    let map =
        admit_curve(&curve, tau).expect("the [0.0, 0.4] sub-region excludes the vanish and admits");
    let region = admit_curve_region(&map, (0.0, 0.4)).expect("the remedy region admits");
    assert!(
        region.margin().lo > tau.get(),
        "the admitted region's margin is above tau"
    );
    assert_eq!(*region.region(), (0.0, 0.4));
}

#[test]
fn non_compact_region_refuses_named_case() {
    let map = admit_curve(&line_curve(), tau(1.0)).expect("the line admits");
    for sub in [
        (0.5, 0.2),
        (f64::NAN, 0.5),
        (0.5, f64::INFINITY),
        (-1.0, 0.5),
        (0.5, 2.0),
    ] {
        assert_eq!(
            map.enclosure(sub),
            Err(MapRefusal::DomainNotCompact),
            "enclosure over the non-compact subregion {sub:?}"
        );
        assert_eq!(
            map.rank_margin(sub),
            Err(MapRefusal::DomainNotCompact),
            "margin over the non-compact subregion {sub:?}"
        );
        assert!(
            matches!(
                admit_curve_region(&map, sub),
                Err(MapRefusal::DomainNotCompact)
            ),
            "region admission over the non-compact subregion {sub:?}"
        );
    }
    // The closed domain boundary is ACCEPTED (inclusive compactness).
    assert!(admit_curve_region(&map, (0.0, 0.0)).is_ok());
    assert!(admit_curve_region(&map, (1.0, 1.0)).is_ok());
    assert!(admit_curve_region(&map, (0.0, 1.0)).is_ok());
    assert!(map.enclosure((0.0, 0.0)).is_ok());
    assert!(map.rank_margin((1.0, 1.0)).is_ok());

    let smap = admit_surface(&two_piece_surface(), tau(0.5)).expect("the surface admits");
    for sub in [
        ((0.5, 0.2), (0.0, 0.5)),
        ((f64::NAN, 1.0), (0.0, 1.0)),
        ((0.0, 3.0), (0.0, 1.0)),
        ((0.5, 1.5), (-1.0, 0.5)),
        ((0.5, 1.5), (0.5, 0.4)),
    ] {
        assert_eq!(
            smap.rank_margin(sub),
            Err(MapRefusal::DomainNotCompact),
            "surface margin over the non-compact subregion {sub:?}"
        );
    }
    assert!(smap.enclosure(((0.0, 0.0), (0.0, 0.0))).is_ok());
    assert!(admit_surface_region(&smap, ((0.0, 2.0), (0.0, 1.0))).is_ok());
}

#[test]
fn enclosure_contains_brute_force_samples() {
    // Curve: 1000 samples of C over the subinterval lie inside the enclosure.
    let curve = line_curve();
    let map = admit_curve(&curve, tau(1.0)).expect("the line admits");
    let sub = (0.2, 0.8);
    let enclosure = map.enclosure(sub).expect("a compact subinterval");
    for i in 0..1000 {
        let t = sub.0 + (sub.1 - sub.0) * (i as f64) / 999.0;
        let point = curve.subs(t);
        assert!(
            enclosure[0].lo <= point.x && point.x <= enclosure[0].hi,
            "x sample at t = {t}"
        );
        assert!(
            enclosure[1].lo <= point.y && point.y <= enclosure[1].hi,
            "y sample at t = {t}"
        );
        assert!(
            enclosure[2].lo <= point.z && point.z <= enclosure[2].hi,
            "z sample at t = {t}"
        );
    }

    // Surface: 1000 samples of S over a piece-spanning rectangle lie inside.
    let surface = two_piece_surface();
    let smap = admit_surface(&surface, tau(0.5)).expect("the surface admits");
    let ssub = ((0.25, 1.75), (0.2, 0.8));
    let senclosure = smap.enclosure(ssub).expect("a compact rectangle");
    for i in 0..1000 {
        let u = ssub.0 .0 + (ssub.0 .1 - ssub.0 .0) * (i as f64) / 999.0;
        let v = ssub.1 .0 + (ssub.1 .1 - ssub.1 .0) * (i as f64) / 999.0;
        let point = surface.subs(u, v);
        assert!(
            senclosure[0].lo <= point.x && point.x <= senclosure[0].hi,
            "x sample at (u = {u}, v = {v})"
        );
        assert!(
            senclosure[1].lo <= point.y && point.y <= senclosure[1].hi,
            "y sample at (u = {u}, v = {v})"
        );
        assert!(
            senclosure[2].lo <= point.z && point.z <= senclosure[2].hi,
            "z sample at (u = {u}, v = {v})"
        );
    }
}

#[test]
fn region_enclosure_contained_in_whole_domain_enclosure() {
    let map = admit_curve(&line_curve(), tau(1.0)).expect("the line admits");
    let whole = map.enclosure((0.0, 1.0)).expect("the whole domain");
    let part = map.enclosure((0.2, 0.8)).expect("a subinterval");
    for k in 0..3 {
        assert!(whole[k].lo <= part[k].lo, "lower bound monotone, k = {k}");
        assert!(part[k].hi <= whole[k].hi, "upper bound monotone, k = {k}");
    }

    let smap = admit_surface(&two_piece_surface(), tau(0.5)).expect("the surface admits");
    let swhole = smap
        .enclosure(((0.0, 2.0), (0.0, 1.0)))
        .expect("the whole domain");
    let spart = smap
        .enclosure(((0.25, 1.75), (0.2, 0.8)))
        .expect("a subrectangle");
    for k in 0..3 {
        assert!(
            swhole[k].lo <= spart[k].lo,
            "surface lower bound monotone, k = {k}"
        );
        assert!(
            spart[k].hi <= swhole[k].hi,
            "surface upper bound monotone, k = {k}"
        );
    }
}

#[test]
fn bspline_curve_admission_matches_direct_bezier_admission() {
    let curve = line_curve();
    let tau = tau(1.0);
    let map = admit_curve(&curve, tau).expect("the B-spline admits");
    let intervals = map.piece_intervals();
    assert_eq!(intervals, vec![(0.0, 0.5), (0.5, 1.0)]);
    let pieces = curve.bezier_decomposition();
    for (piece, interval) in pieces.iter().zip(&intervals) {
        let single = BSplineCurve::new(piece.knot_vec().clone(), piece.control_points().clone());
        let single_map = admit_curve(&single, tau).expect("a single Bézier piece admits");
        assert_eq!(single_map.piece_intervals(), vec![*interval]);
        let width = interval.1 - interval.0;
        let overlap = (interval.0 + 0.05 * width, interval.1 - 0.05 * width);
        let a = map.enclosure(overlap).expect("the multi-piece enclosure");
        let b = single_map
            .enclosure(overlap)
            .expect("the single-piece enclosure");
        for k in 0..3 {
            assert_eq!(a[k].lo, b[k].lo, "lower bound, k = {k}, piece {interval:?}");
            assert_eq!(a[k].hi, b[k].hi, "upper bound, k = {k}, piece {interval:?}");
        }
    }
}

#[test]
fn bspline_surface_decomposition_covers_the_declared_domain() {
    let surface = two_piece_surface();
    let map = admit_surface(&surface, tau(0.5)).expect("the surface admits");
    let boxes = map.patch_boxes();
    let grids = map.patch_grids();
    assert_eq!(boxes.len(), 2);

    // The patch subboxes tile the declared domain exactly: sorted adjacency
    // equality against the unique-knot grid, in f64, exact.
    let (u_knots, _) = surface.uknot_vec().to_single_multi();
    let (v_knots, _) = surface.vknot_vec().to_single_multi();
    let mut expected: Vec<((f64, f64), (f64, f64))> = Vec::new();
    for i in 0..(u_knots.len() - 1) {
        for j in 0..(v_knots.len() - 1) {
            expected.push(((u_knots[i], u_knots[i + 1]), (v_knots[j], v_knots[j + 1])));
        }
    }
    let mut actual = boxes.clone();
    actual.sort_by(|a, b| {
        a.0 .0
            .partial_cmp(&b.0 .0)
            .expect("ordered u bounds")
            .then(a.1 .0.partial_cmp(&b.1 .0).expect("ordered v bounds"))
    });
    expected.sort_by(|a, b| {
        a.0 .0
            .partial_cmp(&b.0 .0)
            .expect("ordered u bounds")
            .then(a.1 .0.partial_cmp(&b.1 .0).expect("ordered v bounds"))
    });
    assert_eq!(
        actual, expected,
        "the patch subboxes tile the declared domain"
    );

    // Tensor commutation: each patch grid, evaluated directly, agrees with the
    // surface's own substitution beyond ulp noise (the row-wise then
    // column-wise cuts must reproduce the surface on each patch box).
    for (patch_box, grid) in boxes.iter().zip(&grids) {
        for i in 0..8 {
            for j in 0..8 {
                let u = patch_box.0 .0 + (patch_box.0 .1 - patch_box.0 .0) * (i as f64) / 8.0;
                let v = patch_box.1 .0 + (patch_box.1 .1 - patch_box.1 .0) * (j as f64) / 8.0;
                let s = (u - patch_box.0 .0) / (patch_box.0 .1 - patch_box.0 .0);
                let t = (v - patch_box.1 .0) / (patch_box.1 .1 - patch_box.1 .0);
                let point = surface.subs(u, v);
                for k in 0..3 {
                    let reference = match k {
                        0 => point.x,
                        1 => point.y,
                        _ => point.z,
                    };
                    let ev = eval_bernstein_2d(&grid[k], s, t);
                    let slack = 16.0 * f64::EPSILON * (1.0 + reference.abs().max(ev.abs()));
                    assert!(
                        (ev - reference).abs() <= slack,
                        "patch grid at (u = {u}, v = {v}) coordinate {k}: grid eval {ev} vs surface subs {reference}"
                    );
                }
            }
        }
    }
}

#[test]
fn rank_margin_lower_bound_bounded_by_brute_force_min() {
    // Curve: the certified lower bound of |C'| must never overclaim the
    // minimum finite-difference |C'| over a sample grid. The step is coarse
    // (`h = 0.05`) because the line's central difference is exact at any step
    // (zero truncation error), and a coarse step keeps the catastrophic-
    // cancellation error of the finite difference inside a few ulps.
    let curve = line_curve();
    let map = admit_curve(&curve, tau(1.0)).expect("the line admits");
    let margin = map.rank_margin((0.0, 1.0)).expect("whole-domain margin");
    let h = 0.05;
    let mut brute_min = f64::INFINITY;
    for i in 0..=20 {
        let t = 0.05 + 0.9 * (i as f64) / 20.0;
        let derivative = (curve.subs(t + h) - curve.subs(t - h)).map(|c| c / (2.0 * h));
        brute_min = brute_min.min(derivative.magnitude());
    }
    // The finite-difference estimate itself carries cancellation error on the
    // order of `eps * |C| / h`; the certified bound must never overclaim the
    // TRUE minimum beyond that estimation slack.
    let slack = 256.0 * f64::EPSILON * brute_min;
    assert!(
        margin.lo <= brute_min + slack,
        "the certified curve margin must not overclaim: margin.lo = {}, brute-force min = {}",
        margin.lo,
        brute_min
    );

    // Surface: same containment over the non-unit-width patch (this fixture
    // would expose a missing inverse-width scaling as an overclaim). Both
    // central differences are exact for the bilinear `S(u, v) = (u, v, uv)`.
    let surface = non_unit_width_surface();
    let smap = admit_surface(&surface, tau(0.5)).expect("the surface admits");
    let smargin = smap
        .rank_margin(((0.0, 2.0), (0.0, 1.0)))
        .expect("whole-domain margin");
    let mut s_brute_min = f64::INFINITY;
    for i in 1..=19 {
        for j in 1..=9 {
            let u = 0.1 * (i as f64);
            let v = 0.1 * (j as f64);
            let su = (surface.subs(u + h, v) - surface.subs(u - h, v)).map(|c| c / (2.0 * h));
            let sv = (surface.subs(u, v + h) - surface.subs(u, v - h)).map(|c| c / (2.0 * h));
            s_brute_min = s_brute_min.min(su.cross(sv).magnitude());
        }
    }
    assert!(
        smargin.lo <= s_brute_min,
        "the certified surface margin must not overclaim |Sᵤ × Sᵥ|: margin.lo = {}, brute-force min = {}",
        smargin.lo,
        s_brute_min
    );
}

#[test]
fn map_never_panics_and_tau_is_declared_not_inferred() {
    // Every entry returns `Result` (a compile-time fact); admission signatures
    // take `PositiveFinite`, so τ is declared at the boundary and never
    // inferred from a raw f64. Exercise every entry point.
    let curve = line_curve();
    let surface = two_piece_surface();
    let tau = PositiveFinite::new(0.5).expect("a positive declared tau");
    assert!(
        PositiveFinite::new(0.0).is_err(),
        "zero is not a declared tau"
    );

    let map = match admit_curve(&curve, tau) {
        Ok(map) => map,
        Err(refusal) => panic!("the line curve must admit, got {refusal:?}"),
    };
    let region = match admit_curve_region(&map, (0.1, 0.9)) {
        Ok(region) => region,
        Err(refusal) => panic!("the sub-region must admit, got {refusal:?}"),
    };
    assert!(region.margin().lo > 0.0);
    assert_eq!(*region.region(), (0.1, 0.9));
    match map.enclosure((0.2, 0.8)) {
        Ok(_) => {}
        Err(refusal) => panic!("the enclosure must certify, got {refusal:?}"),
    }
    match map.rank_margin((0.2, 0.8)) {
        Ok(_) => {}
        Err(refusal) => panic!("the rank margin must certify, got {refusal:?}"),
    }

    let smap = match admit_surface(&surface, tau) {
        Ok(map) => map,
        Err(refusal) => panic!("the two-piece surface must admit, got {refusal:?}"),
    };
    let sregion = match admit_surface_region(&smap, ((0.1, 1.9), (0.1, 0.9))) {
        Ok(region) => region,
        Err(refusal) => panic!("the surface sub-region must admit, got {refusal:?}"),
    };
    assert!(sregion.margin().lo > 0.0);
    assert_eq!(*sregion.region(), ((0.1, 1.9), (0.1, 0.9)));
    match smap.enclosure(((0.2, 1.8), (0.2, 0.8))) {
        Ok(_) => {}
        Err(refusal) => panic!("the surface enclosure must certify, got {refusal:?}"),
    }
    match smap.rank_margin(((0.2, 1.8), (0.2, 0.8))) {
        Ok(_) => {}
        Err(refusal) => panic!("the surface rank margin must certify, got {refusal:?}"),
    }

    // The refusal vocabulary is exactly the three named cases, each with a
    // stable tag.
    assert_eq!(
        MapRefusal::ParameterizationDegenerate.tag(),
        "map_parameterization_degenerate"
    );
    assert_eq!(
        MapRefusal::EnclosureUnavailable.tag(),
        "map_enclosure_unavailable"
    );
    assert_eq!(MapRefusal::DomainNotCompact.tag(), "map_domain_not_compact");

    // H-1 source scan: the module text carries no unwrap, expect, or panic
    // (the crate-level deny mechanically enforces the unwrap half; this pins
    // the same facts in the module text, comment-stripped).
    let source = include_str!("../src/certified_map.rs");
    let stripped: Vec<&str> = source
        .lines()
        .map(|line| line.split("//").next().expect("a line prefix"))
        .collect();
    let code = stripped.join("\n");
    assert!(
        !code.contains("unwrap"),
        "certified_map.rs has no unwrap call"
    );
    assert!(
        !code.contains("expect"),
        "certified_map.rs has no expect call"
    );
    assert!(!code.contains("panic!"), "certified_map.rs has no panic");
}
