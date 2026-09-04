//! The CC-015-GORDON integration tests (spine S8 consumer): the Gordon
//! Boolean sum `S = S_u + S_v − S_uv` over a compatible profile/guide network.
//!
//! The L1 ground truth is built IN THE TEST as a consistent network of the
//! hyperbolic-paraboloid carrier `T(u, v) = (u, v, u·v)`: the profiles are its
//! `v`-sections at the `v` stations and the guides are its `u`-sections at the
//! `u` stations, all represented as quadratic Bézier curves over `[0, 1]`.
//! Because every construction object already exists from CC-010, a consistent
//! network reproduces the carrier exactly: the delivered Gordon surface must
//! interpolate both curve families at their stations and hit every network
//! point `(u_i, v_j)` with exactly one copy of the coincident network value —
//! not the two copies `S_u + S_v` alone would contribute (the correction
//! `S_uv` removes the double count).

#![deny(clippy::unwrap_used)]

use truck_certified::construct::banded::factor_banded_tp;
use truck_certified::construct::gordon::{gordon_surface, GordonInput};
use truck_certified::construct::loft::{
    loft_collocation_bands, loft_sections, make_compatible, LoftOutput,
};
use truck_certified::construct::refusal::ConstructRefusal;
use truck_certified::construct::Interval;
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

/// The quadratic Bézier knot vector over `[0, 1]` (full-clamped, no interior
/// knots) — the averaged `u`/`v` station knot of a three-station quadratic
/// loft.
fn quadratic_bezier_knot() -> KnotVec {
    KnotVec::from(vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0])
}

/// A quadratic Bézier curve over `[0, 1]` with the three given controls.
fn quadratic_bezier(controls: [Vector4; 3]) -> BSplineCurve<Vector4> {
    BSplineCurve::new(quadratic_bezier_knot(), controls.to_vec())
}

/// A consistent profile/guide network of the carrier `T(u, v) = (u, v, u·v)`.
struct NetworkFixture {
    profiles: Vec<BSplineCurve<Vector4>>,
    guides: Vec<BSplineCurve<Vector4>>,
    stations_u: Vec<f64>,
    stations_v: Vec<f64>,
}

impl NetworkFixture {
    /// The Gordon input built from this network.
    fn input(&self) -> GordonInput {
        GordonInput {
            profiles: self.profiles.clone(),
            guides: self.guides.clone(),
            stations_u: self.stations_u.clone(),
            stations_v: self.stations_v.clone(),
        }
    }
}

/// The consistent network: profiles `P_j(u) = (u, v_j, u·v_j)` and guides
/// `G_i(v) = (u_i, v, u_i·v)` over the station grids used by the tests.
///
/// The three quadratic profiles sit at the `v` stations and the three
/// quadratic guides at the `u` stations; every network point
/// `P_j(u_i) = G_i(v_j) = (u_i, v_j, u_i·v_j)` is a crossing of one profile and
/// one guide.
fn consistent_network() -> NetworkFixture {
    let stations_u = vec![0.0, 0.45, 1.0];
    let stations_v = vec![0.0, 0.6, 1.0];
    let profiles = stations_v
        .iter()
        .map(|&v| quadratic_bezier([p4(0.0, v, 0.0), p4(0.5, v, 0.5 * v), p4(1.0, v, v)]))
        .collect();
    let guides = stations_u
        .iter()
        .map(|&u| quadratic_bezier([p4(u, 0.0, 0.0), p4(u, 0.5, 0.5 * u), p4(u, 1.0, u)]))
        .collect();
    NetworkFixture {
        profiles,
        guides,
        stations_u,
        stations_v,
    }
}

/// The reference component lofts, rebuilt through the public CC-010 API with
/// the SAME stations and degrees `gordon_surface` derives (profile degree for
/// the `u` direction, guide degree for the `v` direction). Returns the profile
/// loft `S_u`, the transposed guide loft `S_v`, and their enclosure widths.
fn reference_lofts(fx: &NetworkFixture) -> (LoftOutput, LoftOutput) {
    let profiles = construct(make_compatible(&fx.profiles));
    let guides = construct(make_compatible(&fx.guides));
    let u_degree = profiles[0].degree();
    let v_degree = guides[0].degree();

    let factor_u = construct(factor_banded_tp(&construct(loft_collocation_bands(
        &fx.stations_u,
        u_degree,
    ))));
    let factor_v = construct(factor_banded_tp(&construct(loft_collocation_bands(
        &fx.stations_v,
        v_degree,
    ))));

    let su = construct(loft_sections(
        &profiles,
        &fx.stations_v,
        v_degree,
        &factor_v,
    ));
    let sv_raw = construct(loft_sections(&guides, &fx.stations_u, u_degree, &factor_u));
    let mut sv = sv_raw.surface;
    sv.swap_axes();
    (
        su,
        LoftOutput {
            surface: sv,
            epsilon: sv_raw.epsilon,
        },
    )
}

/// The `u` sample count of the section-reproduction grids.
const SAMPLE_N: usize = 32;

/// Replicates the correction's u-direction solve to observe its enclosure
/// width: each network row is interpolated through `factor_u` in profile order
/// (the order `gordon_surface` uses), so the factor's cache after the last row
/// is the width the shared-factor u-direction assertion observes.
fn correction_u_solve_width(fx: &NetworkFixture) -> f64 {
    let profiles = construct(make_compatible(&fx.profiles));
    let guides = construct(make_compatible(&fx.guides));
    let u_degree = profiles[0].degree();
    let factor_u = construct(factor_banded_tp(&construct(loft_collocation_bands(
        &fx.stations_u,
        u_degree,
    ))));

    // The guide loft and then the per-profile row solves share factor_u, in
    // the same order gordon_surface performs them.
    let _sv = construct(loft_sections(&guides, &fx.stations_u, u_degree, &factor_u));
    for profile in &profiles {
        let mut rhs = Vec::with_capacity(fx.stations_u.len());
        for &station in &fx.stations_u {
            let point = profile.subs(station);
            rhs.push([
                Interval::point(point.x),
                Interval::point(point.y),
                Interval::point(point.z),
                Interval::point(point.w),
            ]);
        }
        construct(factor_u.solve_homogeneous(&rhs));
    }
    factor_u.max_control_error()
}

#[test]
fn cardinal_functions_are_exactly_delta_at_stations() {
    // At every network point (u_i, v_j) the profile `P_j` and the guide `G_i`
    // coincide (the network is consistent), and the delivered Gordon surface
    // reproduces that coincident value: the u/v interpolation cardinals are
    // exactly δ at their stations, so only the intersecting pair's shared
    // network value shows through S_u + S_v − S_uv there.
    let fx = consistent_network();
    let input = fx.input();
    let output = construct(gordon_surface(&input));
    let epsilon = output.epsilon;

    for (j, profile) in fx.profiles.iter().enumerate() {
        for (i, guide) in fx.guides.iter().enumerate() {
            let u = fx.stations_u[i];
            let v = fx.stations_v[j];
            let on_profile = euclid4(profile.subs(u));
            let on_guide = euclid4(guide.subs(v));
            let on_surface = euclid4(output.surface.subs(u, v));

            // The two families agree at the crossing (fixture consistency).
            let network_gap = distance(on_profile, on_guide);
            assert!(
                network_gap <= epsilon,
                "inconsistent network at ({u}, {v}): profile and guide differ by {network_gap}"
            ); // H-3

            // The surface hits the crossing: the profile-derived value ...
            let profile_gap = distance(on_surface, on_profile);
            assert!(
                profile_gap <= epsilon,
                "surface misses the network point on profile {j} at ({u}, {v}) by {profile_gap}"
            ); // H-3

            // ... and the guide-derived value (the same network point).
            let guide_gap = distance(on_surface, on_guide);
            assert!(
                guide_gap <= epsilon,
                "surface misses the network point on guide {i} at ({u}, {v}) by {guide_gap}"
            ); // H-3
        }
    }
}

#[test]
fn correction_term_removes_double_counting_at_network_points() {
    // At a network point S_u and S_v each contribute a full copy of the
    // coincident value; without the correction the sum would carry two copies.
    // The SUM S_u + S_v − S_uv (the delivered surface) must instead equal the
    // expected cross-boundary value there up to ε — the weight channel is the
    // witness: a doubled contribution leaves w ≈ 2, the corrected surface has
    // w ≈ 1.
    let fx = consistent_network();
    let input = fx.input();
    let output = construct(gordon_surface(&input));
    let (su, sv) = reference_lofts(&fx);
    let epsilon = output.epsilon;

    let mut worst_surface = 0.0_f64;
    let mut worst_doubled = 0.0_f64;
    for (j, profile) in fx.profiles.iter().enumerate() {
        for (i, _guide) in fx.guides.iter().enumerate() {
            let u = fx.stations_u[i];
            let v = fx.stations_v[j];
            let expected = euclid4(profile.subs(u));
            let doubled = su.surface.subs(u, v) + sv.surface.subs(u, v);

            // Two lofts alone double count: the sum contributes two copies of
            // the network value (witness: weight channel ≈ 2, not ≈ 1).
            let expected_weight = 2.0_f64;
            let weight_gap = (doubled.w - expected_weight).abs();
            if weight_gap > worst_doubled {
                worst_doubled = weight_gap;
            }
            assert!(
                weight_gap <= 2.0 * (su.epsilon + sv.epsilon),
                "S_u + S_v does not double count the network value at ({u}, {v}): \
                 weight {0}",
                doubled.w
            ); // H-3

            // The corrected SUM S_u + S_v − S_uv equals the expected
            // cross-boundary value: unit weight and the projected point.
            let on_surface = output.surface.subs(u, v);
            let weight_gap = (on_surface.w - 1.0).abs();
            let point_gap = distance(euclid4(on_surface), expected);
            if point_gap > worst_surface {
                worst_surface = point_gap;
            }
            assert!(
                weight_gap <= epsilon,
                "corrected surface weight not single-counted at ({u}, {v}): {0}",
                on_surface.w
            ); // H-3
            assert!(
                point_gap <= epsilon,
                "S_u + S_v − S_uv misses the cross-boundary value at ({u}, {v}) by {point_gap}"
            ); // H-3
        }
    }

    eprintln!(
        "GORDON_EPS profile_loft={:.3e} guide_loft={:.3e} correction_u={:.3e} \
         correction_loft={:.3e} combined={:.3e} worst_doubled_w_gap={:.3e} \
         worst_surface_gap={:.3e}",
        su.epsilon,
        sv.epsilon,
        correction_u_solve_width(&fx),
        output.epsilon - su.epsilon - sv.epsilon,
        output.epsilon,
        worst_doubled,
        worst_surface
    );
}

#[test]
fn output_passes_the_same_validity_postcondition() {
    // The CC-010 L1 gate applied to the Gordon output: the delivered surface
    // must reproduce every profile at its v station over a u sample grid and
    // every guide at its u station over a v sample grid, to within the
    // delivered enclosure width ε — the same validity postcondition CC-010
    // asserts for a loft's sections.
    let fx = consistent_network();
    let input = fx.input();
    let output = construct(gordon_surface(&input));
    let epsilon = output.epsilon;
    assert!(epsilon >= 0.0); // H-3

    for (j, profile) in fx.profiles.iter().enumerate() {
        let station = fx.stations_v[j];
        let mut worst = 0.0_f64;
        for k in 0..=SAMPLE_N {
            let u = (k as f64) / (SAMPLE_N as f64);
            let gap = distance(
                euclid4(output.surface.subs(u, station)),
                euclid4(profile.subs(u)),
            );
            if gap > worst {
                worst = gap;
            }
        }
        assert!(
            worst <= epsilon,
            "profile {j} deviates {worst} from its station curve, above the delivered \
             enclosure width {epsilon}"
        ); // H-3
    }

    for (i, guide) in fx.guides.iter().enumerate() {
        let station = fx.stations_u[i];
        let mut worst = 0.0_f64;
        for k in 0..=SAMPLE_N {
            let v = (k as f64) / (SAMPLE_N as f64);
            let gap = distance(
                euclid4(output.surface.subs(station, v)),
                euclid4(guide.subs(v)),
            );
            if gap > worst {
                worst = gap;
            }
        }
        assert!(
            worst <= epsilon,
            "guide {i} deviates {worst} from its station curve, above the delivered \
             enclosure width {epsilon}"
        ); // H-3
    }
}

#[test]
fn incompatible_component_bases_refuse() {
    // A family whose shared basis is not the averaged knot vector of its
    // station grid can never share one (u, v) basis pair with the other two
    // components. An un-clamped curve already refuses make_compatible; here
    // the refusal is the basis gate itself: a clamped quadratic profile family
    // carrying an interior knot 0.5 is not the averaged knot vector of any
    // three-station quadratic grid, so the components' u bases differ and the
    // construction refuses InvalidInput — never a silent re-basis.
    let fx = consistent_network();
    let interior_knot = KnotVec::from(vec![0.0, 0.0, 0.0, 0.5, 1.0, 1.0, 1.0]);
    let profiles_with_foreign_knot = vec![
        BSplineCurve::new(
            interior_knot.clone(),
            vec![
                p4(0.0, 0.0, 0.0),
                p4(0.5, 0.5, 0.1),
                p4(1.0, 1.0, 0.2),
                p4(2.0, 2.0, 0.0),
            ],
        ),
        BSplineCurve::new(
            interior_knot,
            vec![
                p4(0.0, 1.0, 0.0),
                p4(0.5, 1.5, 0.1),
                p4(1.0, 2.0, 0.2),
                p4(2.0, 2.5, 0.0),
            ],
        ),
        BSplineCurve::new(
            quadratic_bezier_knot(),
            vec![p4(0.0, 2.0, 0.0), p4(1.0, 2.5, 0.1), p4(2.0, 3.0, 0.0)],
        ),
    ];
    let input = GordonInput {
        profiles: profiles_with_foreign_knot,
        guides: fx.guides.clone(),
        stations_u: fx.stations_u.clone(),
        stations_v: fx.stations_v.clone(),
    };
    match gordon_surface(&input) {
        Err(ConstructRefusal::InvalidInput) => {}
        Ok(_) => panic!("a foreign profile basis must refuse as InvalidInput"),
        Err(other) => panic!("wrong refusal for a foreign profile basis: {other:?}"),
    }

    // A station count different from the family count is invalid input too.
    let consistent = consistent_network();
    let input = GordonInput {
        profiles: consistent.profiles.clone(),
        guides: consistent.guides.clone(),
        stations_u: consistent.stations_u.clone(),
        stations_v: vec![0.0, 1.0],
    };
    match gordon_surface(&input) {
        Err(ConstructRefusal::InvalidInput) => {}
        Ok(_) => panic!("a family / station-count mismatch must refuse as InvalidInput"),
        Err(other) => panic!("wrong refusal for a station-count mismatch: {other:?}"),
    }
}
