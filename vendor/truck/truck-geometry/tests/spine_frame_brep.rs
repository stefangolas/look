#![deny(clippy::unwrap_used)]

//! BG-CG-009-BREP â€” the parametric realization decorators and the closed-enum
//! ripple. These tests pin the two new `Curve`/`Surface` variants' forwarding
//! and the decorators' evaluation/search/transform discipline over the landed
//! recipe evaluators.

use truck_base::evidence::Refusal;
use truck_geometry::constructive::{
    ConstructError, DirectTolerance, FrameLaw, LineSpine, Profile2D, ProfileLaw, SpineFrameRecipe,
};
use truck_geometry::prelude::*;

/// The square profile (CCW about +z in the frame plane): vertices
/// (0,0), (1,0), (1,1), (0,1).
fn unit_square() -> Profile2D {
    Profile2D::try_closed(vec![
        Point2::new(0.0, 0.0),
        Point2::new(1.0, 0.0),
        Point2::new(1.0, 1.0),
        Point2::new(0.0, 1.0),
    ])
    .expect("a unit square is a valid closed profile")
}

/// The recipe over a `LineSpine` from the origin to (0, 0, 1): the frame plane
/// is pinned by `FixedPlane { normal: +x }`, so `b = x`, `n = x Ã— z = -y`,
/// and `X(s, v) = (py, -px, s)`.
fn line_spine_recipe() -> SpineFrameRecipe<LineSpine, ProfileLaw, FrameLaw> {
    let spine = LineSpine {
        start: Point3::origin(),
        end: Point3::new(0.0, 0.0, 1.0),
    };
    let profile = ProfileLaw::Constant(unit_square());
    let frame = FrameLaw::FixedPlane {
        normal: Vector3::unit_x(),
    };
    SpineFrameRecipe::new(spine, profile, frame)
}

/// The recipe over a `Box<Curve>` line spine â€” the storage form the closed
/// enums carry (the indirection that breaks the enum recursion).
fn curve_spine_recipe() -> SpineFrameRecipe<Box<Curve>, ProfileLaw, FrameLaw> {
    let spine = Box::new(Curve::Line(Line(
        Point3::origin(),
        Point3::new(0.0, 0.0, 1.0),
    )));
    let profile = ProfileLaw::Constant(unit_square());
    let frame = FrameLaw::FixedPlane {
        normal: Vector3::unit_x(),
    };
    SpineFrameRecipe::new(spine, profile, frame)
}

/// Edge 0 of the unit square: the window `[0, 1] Ã— [0, 1/4]` on the recipe.
fn edge_zero_surface() -> SpineFrameSurface<LineSpine> {
    SpineFrameSurface::try_new(line_spine_recipe(), 0.0, 1.0, 0.0, 0.25)
        .expect("edge zero is a valid surface window")
}

/// The stored (`Box<Curve>`-spine) edge-zero surface, as the canonical
/// `Surface` enum carries it.
fn edge_zero_stored_surface() -> Surface {
    let recipe = curve_spine_recipe();
    let surface =
        SpineFrameSurface::try_new(recipe, 0.0, 1.0, 0.0, 0.25).expect("a valid stored surface");
    Surface::SpineFrameSurface(surface)
}

#[test]
fn spine_frame_surface_evaluates_the_recipe() {
    let surface = edge_zero_surface();
    let recipe = line_spine_recipe();
    let tolerance = DirectTolerance::default().position;
    for i in 0..=8 {
        let s = i as f64 / 8.0;
        for j in 0..8 {
            // Sample strictly inside the edge (0, 1/4): the derivative at the
            // exact vertex v = 1/4 belongs to the NEXT edge by the floor rule.
            let v = 0.25 * (j as f64 + 0.5) / 8.0;
            let got = surface.subs(s, v);
            let expected = recipe
                .position(s, v)
                .expect("the recipe evaluates inside the window");
            assert!(
                (got - expected).magnitude() <= tolerance,
                "surface.subs({s}, {v}) diverged from the recipe"
            );
            // The v-direction is analytic: `S_v = frame Â· dP/dv` on the edge.
            let vder = surface.vder(s, v);
            assert!(vder.x == 0.0, "edge 0 keeps py = 0, so S_v.x == 0");
            assert!((vder.y + 4.0).abs() <= tolerance, "S_v = (0, -4, 0)");
            assert!(vder.z == 0.0, "S_v has no z component");
            // The s-direction is the spine derivative plus the frame.
            let uder = surface.uder(s, v);
            assert!((uder - Vector3::unit_z()).magnitude() <= tolerance);
        }
    }
}

#[test]
fn trajectory_curve_matches_surface_offset() {
    let surface = edge_zero_surface();
    let curve = SpineFrameCurve::try_new(line_spine_recipe(), 0.0, 1.0, 0.25)
        .expect("vertex 1 (ring v = 1/4) is a valid trajectory");
    let tolerance = DirectTolerance::default().position;
    for i in 0..=16 {
        let s = i as f64 / 16.0;
        // The trajectory of profile vertex 1 is the surface's v = 1/4 isocurve.
        let on_surface = surface.subs(s, 0.25);
        let on_curve = curve.subs(s);
        assert!(
            (on_surface - on_curve).magnitude() <= tolerance,
            "trajectory left the host surface at s = {s}"
        );
        // And it is exactly X(s, 1/4): the profile point (1, 0) maps to
        // (py, -px, s) = (0, -1, s).
        assert!((on_curve - Point3::new(0.0, -1.0, s)).magnitude() <= tolerance);
    }
}

#[test]
fn search_parameter_newton_recovers_station_and_vertex() {
    let surface = edge_zero_surface();
    let tolerance = DirectTolerance::default().position;
    for i in 0..=8 {
        let s = 0.2 + 0.6 * i as f64 / 8.0;
        for j in 0..=8 {
            let v = 0.05 + 0.15 * j as f64 / 8.0;
            let point = surface.subs(s, v);
            let Some((su, sv)) = surface.search_parameter(point, None, 100) else {
                panic!("Newton failed to recover ({s}, {v})");
            };
            // Newton-recovery epsilons in parameters, not model-space lengths.
            assert!((su - s).abs() <= 1.0e-6, "recovered station {su} != {s}"); // H-3
            assert!((sv - v).abs() <= 1.0e-6, "recovered vertex {sv} != {v}"); // H-3
            assert!((surface.subs(su, sv) - point).magnitude() <= tolerance);
        }
    }
}

#[test]
fn surface_variant_forwarding_all_landed_methods() {
    let surface = edge_zero_stored_surface();
    let tolerance = DirectTolerance::default().position;
    for i in 0..=4 {
        let s = i as f64 / 4.0;
        for j in 0..4 {
            let v = 0.25 * (j as f64 + 0.5) / 4.0;
            // `subs`, `normal`, and the range all forward to the decorator.
            assert!((surface.subs(s, v) - Point3::new(0.0, -4.0 * v, s)).magnitude() <= tolerance);
            let normal = surface.normal(s, v);
            assert!((normal - Vector3::unit_x()).magnitude() <= tolerance);
        }
    }
    // `parameter_range` forwards the stored window.
    let (u_range, v_range) = surface.try_range_tuple();
    let (u0, u1) = u_range.expect("bounded u range");
    let (v0, v1) = v_range.expect("bounded v range");
    assert_eq!((u0, u1), (0.0, 1.0));
    assert_eq!((v0, v1), (0.0, 0.25));
    // `invert` swaps the v-axis.
    let inverse = surface.inverse();
    let Surface::SpineFrameSurface(inverse) = inverse else {
        panic!("the inverse of a spine-frame variant stays a spine-frame variant");
    };
    assert_eq!(inverse.v0(), 0.25);
    assert_eq!(inverse.v1(), 0.0);
    // `SearchParameter<D2>` recovers the station and vertex on the wrapped
    // variant too.
    let point = surface.subs(0.5, 0.125);
    let Some((su, sv)) = surface.search_parameter(point, None, 100) else {
        panic!("forwarded search_parameter failed");
    };
    // Recovery epsilons are test Newton tolerances, not model-space lengths.
    assert!((su - 0.5).abs() <= 1.0e-6 && (sv - 0.125).abs() <= 1.0e-6); // H-3
    assert!(surface.search_nearest_parameter(point, None, 100).is_some()); // SearchNearestParameter<D2> forwards.
}

#[test]
fn curve_variant_forwarding_all_landed_methods() {
    let recipe = curve_spine_recipe();
    let trajectory = SpineFrameCurve::try_new(recipe, 0.0, 1.0, 0.0)
        .expect("vertex 0 (ring v = 0) is a valid trajectory");
    let curve = Curve::SpineFrameCurve(trajectory);
    let tolerance = DirectTolerance::default().position;
    for i in 0..=8 {
        let s = i as f64 / 8.0;
        // `subs`, `der`, `range_tuple` forward to the decorator.
        assert!((curve.subs(s) - Point3::new(0.0, 0.0, s)).magnitude() <= tolerance);
        assert!((curve.der(s) - Vector3::unit_z()).magnitude() <= tolerance);
    }
    assert_eq!(curve.range_tuple(), (0.0, 1.0));
    // `invert` swaps the s-axis.
    let inverse = curve.inverse();
    let Curve::SpineFrameCurve(inverse) = inverse else {
        panic!("the inverse of a spine-frame curve stays a spine-frame curve");
    };
    assert_eq!(inverse.s0(), 1.0);
    assert_eq!(inverse.s1(), 0.0);
    // `transformed` composes the stored placement.
    let moved = curve.transformed(Matrix4::from_translation(Vector3::new(3.0, 0.0, 0.0)));
    assert!((moved.subs(0.5) - Point3::new(3.0, 0.0, 0.5)).magnitude() <= tolerance);
    // `SearchParameter<D1>` delegates to the host surface's search on the
    // vertex line.
    let point = curve.subs(0.5);
    let Some(recovered) = curve.search_parameter(point, None, 100) else {
        panic!("forwarded curve search_parameter failed");
    };
    // Recovery epsilon is a test Newton tolerance, not a model-space length.
    assert!((recovered - 0.5).abs() <= 1.0e-6); // H-3
    assert!(curve.search_nearest_parameter(point, None, 100).is_some()); // SearchNearestParameter<D1> forwards.
    let (params, _) = curve.parameter_division((0.0, 1.0), TOLERANCE);
    assert!(params.len() >= 2);
    // `cut` splits the s-window.
    let mut left = curve.clone();
    let right = left.cut(0.5);
    assert_eq!(left.range_tuple(), (0.0, 0.5));
    assert_eq!(right.range_tuple(), (0.5, 1.0));
}

#[test]
fn transform_of_surface_refuses_or_composes_typed() {
    let surface = edge_zero_stored_surface();
    let tolerance = DirectTolerance::default().position;

    // A translation composes exactly into the stored matrix.
    let translation = Matrix4::from_translation(Vector3::new(2.0, -3.0, 4.0));
    let placed = surface.transformed(translation);
    for i in 0..=4 {
        let s = i as f64 / 4.0;
        for j in 0..=4 {
            let v = 0.25 * j as f64 / 4.0;
            let expected = translation.transform_point(surface.subs(s, v));
            assert!((placed.subs(s, v) - expected).magnitude() <= tolerance);
        }
    }

    // A trajectory-containment question cannot be certified on the placed
    // surface (the canonical `Curve` has no equality, and the placed
    // trajectory is not even representable as a canonical `SpineFrameCurve`);
    // it refuses typed â€” never approximates.
    let trajectory =
        SpineFrameCurve::try_new(curve_spine_recipe(), 0.0, 1.0, 0.25).expect("a valid trajectory");
    let query = Curve::SpineFrameCurve(trajectory);
    let outcome = placed.include(&query);
    assert!(
        matches!(outcome, Err(Refusal::NumericallyUnresolved { .. })),
        "trajectory containment on the placed surface must refuse typed"
    );

    // A singular placement still composes exactly for `subs` (no inverse is
    // needed for evaluation) â€” the refusal above is the honest boundary.
    let projection = Matrix4 {
        x: Vector4::new(1.0, 0.0, 0.0, 0.0),
        y: Vector4::new(0.0, 1.0, 0.0, 0.0),
        z: Vector4::new(0.0, 0.0, 0.0, 0.0),
        w: Vector4::new(0.0, 0.0, 0.0, 1.0),
    };
    let collapsed = surface.transformed(projection);
    let expected = projection.transform_point(surface.subs(0.5, 0.125));
    assert!((collapsed.subs(0.5, 0.125) - expected).magnitude() <= tolerance);
}

#[test]
fn surface_constructor_refuses_invalid_windows_typed() {
    // A window that does not cover exactly one profile edge refuses
    // `InvalidInput` â€” the profile-edge window contract.
    let recipe = line_spine_recipe();
    let cross_edge = SpineFrameSurface::try_new(recipe.clone(), 0.0, 1.0, 0.0, 0.5);
    assert!(matches!(cross_edge, Err(ConstructError::InvalidInput)));
    // A reversed window refuses.
    let reversed = SpineFrameSurface::try_new(recipe.clone(), 1.0, 0.0, 0.0, 0.25);
    assert!(matches!(reversed, Err(ConstructError::InvalidInput)));
    // A station outside the spine domain refuses.
    let out_of_domain = SpineFrameSurface::try_new(recipe, 0.0, 2.0, 0.0, 0.25);
    assert!(matches!(out_of_domain, Err(ConstructError::InvalidInput)));
}
