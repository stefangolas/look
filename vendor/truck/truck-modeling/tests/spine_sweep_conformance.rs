#![deny(clippy::unwrap_used)]

//! BG-CG-009-BREP — the authored-topology sweep constructor conformance:
//! the closed prism solid, shared trajectory edges by identity, topology that
//! is independent of station density, the typed nonplanar-cap refusal, and
//! the analytic volume match.

use std::collections::HashMap;

use truck_base::evidence::{Certified, Refusal};
use truck_geometry::constructive::{FrameLaw, LineSpine, Profile2D, ProfileLaw, SpineFrameRecipe};
use truck_modeling::*;
use truck_modeling::{builder, errors::Error};

/// The unit-square profile (CCW about +z in the frame plane).
fn unit_square() -> Profile2D {
    Profile2D::try_closed(vec![
        Point2::new(0.0, 0.0),
        Point2::new(1.0, 0.0),
        Point2::new(1.0, 1.0),
        Point2::new(0.0, 1.0),
    ])
    .expect("a unit square is a valid closed profile")
}

/// The recipe of a unit prism: `LineSpine` of height 1 along +z, the unit
/// square profile, and a `FixedPlane` frame pinned to +x, so the sweep is a
/// 1×1×1 box (the profile maps to `(py, -px)` in the xy-plane).
fn unit_prism_recipe() -> SpineFrameRecipe<LineSpine, ProfileLaw, FrameLaw> {
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

/// The swept unit prism over `stations`.
fn unit_prism(stations: &[f64]) -> Solid {
    let recipe = unit_prism_recipe();
    match spine_sweep::spine_sweep::<LineSpine>(&recipe, stations) {
        Ok(Certified { value, .. }) => value,
        Err(refusal) => panic!("unit prism refused: {refusal:?}"),
    }
}

/// The per-edge use census of a solid's shell: every edge id with its use
/// count and the sum of its signed orientations.
fn edge_census(solid: &Solid) -> HashMap<truck_topology::EdgeID<Curve>, (usize, i32)> {
    let mut census: HashMap<truck_topology::EdgeID<Curve>, (usize, i32)> = HashMap::new();
    for face in solid.face_iter() {
        for wire in face.absolute_boundaries() {
            for edge in wire.iter() {
                let entry = census.entry(edge.id()).or_insert((0, 0));
                entry.0 += 1;
                entry.1 += if edge.orientation() { 1 } else { -1 };
            }
        }
    }
    census
}

/// The signed volume of a solid, computed from its parametric boundary faces:
/// V = (1/6) Σ a·(b×c) over the fan triangulation of every face, sampled on a
/// fixed grid per face.
fn signed_volume(solid: &Solid) -> f64 {
    let mut sum = 0.0;
    for face in solid.face_iter() {
        let surface = face.surface();
        let (u_range, v_range) = surface.try_range_tuple();
        let (u0, u1) = u_range.expect("bounded u range");
        let (v0, v1) = v_range.expect("bounded v range");
        const N: usize = 12;
        for i in 0..N {
            for j in 0..N {
                let a = surface.subs(
                    u0 + (u1 - u0) * (i as f64 / N as f64),
                    v0 + (v1 - v0) * (j as f64 / N as f64),
                );
                let b = surface.subs(
                    u0 + (u1 - u0) * ((i + 1) as f64 / N as f64),
                    v0 + (v1 - v0) * (j as f64 / N as f64),
                );
                let c = surface.subs(
                    u0 + (u1 - u0) * ((i + 1) as f64 / N as f64),
                    v0 + (v1 - v0) * ((j + 1) as f64 / N as f64),
                );
                let d = surface.subs(
                    u0 + (u1 - u0) * (i as f64 / N as f64),
                    v0 + (v1 - v0) * ((j + 1) as f64 / N as f64),
                );
                let origin = Point3::origin();
                sum += (a - origin).dot((b - origin).cross(c - origin));
                sum += (a - origin).dot((c - origin).cross(d - origin));
            }
        }
    }
    sum / 6.0
}

#[test]
fn prism_sweep_assembles_closed_solid() {
    let solid = unit_prism(&[0.0, 1.0]);
    // The authored topology: 4 side faces (one per profile edge) + 2 caps,
    // regardless of the station count.
    assert_eq!(solid.boundaries()[0].len(), 6);
    // The shell is closed, connected, and manifold (Solid::try_new validated
    // all three); the face/edge census confirms the pairing by identity.
    let census = edge_census(&solid);
    assert!(
        census
            .values()
            .all(|&(uses, direction)| uses == 2 && direction == 0),
        "every shared edge must appear exactly twice with opposite orientations"
    );
}

#[test]
fn side_faces_share_trajectory_edges_by_identity() {
    let solid = unit_prism(&[0.0, 1.0]);
    // Exactly k = 4 distinct trajectory edges exist (one per profile vertex),
    // each a `SpineFrameCurve`, and each is shared by exactly two faces.
    let census = edge_census(&solid);
    let mut trajectory_ids = std::collections::HashSet::new();
    for edge in solid.edge_iter() {
        if matches!(edge.curve(), Curve::SpineFrameCurve(_)) {
            trajectory_ids.insert(edge.id());
        }
    }
    assert_eq!(trajectory_ids.len(), 4, "one trajectory per profile vertex");
    for id in &trajectory_ids {
        let &(uses, direction) = census.get(id).expect("the trajectory is in the census");
        assert_eq!(uses, 2, "a trajectory edge is shared by exactly two faces");
        assert_eq!(direction, 0, "the two uses are opposite");
    }
    // The two adjacent side faces reference the SAME trajectory edge handle
    // (identity, not coordinates): side face j uses trajectory[j+1] reversed
    // as its v = v1 boundary, and side face j+1 uses trajectory[j+1] as its
    // v = v0 boundary.
    let faces = solid.boundaries()[0].iter().collect::<Vec<_>>();
    let side_face_edges = |face: &Face| -> Vec<Edge> {
        face.boundaries()[0]
            .edge_iter()
            .cloned()
            .collect::<Vec<_>>()
    };
    for j in 0..4 {
        let current = side_face_edges(faces[j]);
        let next = side_face_edges(faces[(j + 1) % 4]);
        // The shared trajectory is the THIRD edge of the current face (its
        // v = v1 boundary, trajectory[j+1] reversed) and the FIRST edge of the
        // next face (its v = v0 boundary, trajectory[j+1]).
        let shared_current = &current[2];
        let shared_next = &next[0];
        assert!(
            shared_current.is_same(shared_next),
            "adjacent side faces must share the trajectory edge by identity"
        );
        assert_ne!(
            shared_current.orientation(),
            shared_next.orientation(),
            "the shared trajectory is traversed oppositely"
        );
    }
}

#[test]
fn tessellation_density_does_not_change_topology() {
    // The station list is the tessellation density along the spine; the BREP
    // topology (side faces per profile edge + caps) must not depend on it.
    let coarse = unit_prism(&[0.0, 1.0]);
    let fine = unit_prism(&[0.0, 0.25, 0.5, 0.75, 1.0]);
    let coarse_census = edge_census(&coarse);
    let fine_census = edge_census(&fine);
    assert_eq!(coarse.boundaries()[0].len(), 6);
    assert_eq!(fine.boundaries()[0].len(), 6);
    assert_eq!(coarse_census.len(), fine_census.len());
    assert!(
        coarse_census
            .values()
            .all(|&(uses, direction)| uses == 2 && direction == 0),
        "coarse sweep is a closed shell"
    );
    assert!(
        fine_census
            .values()
            .all(|&(uses, direction)| uses == 2 && direction == 0),
        "fine sweep is a closed shell"
    );
}

#[test]
fn nonplanar_cap_refuses_typed() {
    // A cap ring whose vertices are not coplanar (a bent quad) refuses at the
    // cap attachment with the typed `WireNotInOnePlane` — the refusal
    // `spine_sweep` maps into its `Refusal` at the cap step. A valid sweep
    // never produces one (every station ring lies in its frame plane), so the
    // refusal is defensive, never silent clamping.
    let v0 = builder::vertex(Point3::new(0.0, 0.0, 0.0));
    let v1 = builder::vertex(Point3::new(1.0, 0.0, 0.0));
    let v2 = builder::vertex(Point3::new(1.0, 1.0, 0.0));
    let v3 = builder::vertex(Point3::new(0.0, 1.0, 1.0));
    let wire: Wire = vec![
        builder::line(&v0, &v1),
        builder::line(&v1, &v2),
        builder::line(&v2, &v3),
        builder::line(&v3, &v0),
    ]
    .into();
    assert!(matches!(
        builder::try_attach_plane::<_, Surface>(vec![wire]),
        Err(Error::WireNotInOnePlane)
    ));
}

#[test]
fn convex_prism_volume_matches_analytic() {
    let solid = unit_prism(&[0.0, 1.0]);
    // The unit square swept by height 1 is a unit box: volume 1.0.
    let volume = signed_volume(&solid).abs();
    assert!(
        (volume - 1.0).abs() <= 1.0e-4,
        "unit prism volume {volume} must match the analytic 1.0"
    );
}

#[test]
fn through_zero_scale_refuses_typed() {
    // A `Scale` law whose scalar crosses zero between the first and last
    // stations collapses the profile; the sweep entry refuses typed.
    let spine = LineSpine {
        start: Point3::origin(),
        end: Point3::new(0.0, 0.0, 1.0),
    };
    let profile = ProfileLaw::Scale {
        profile: unit_square(),
        scale: truck_geometry::constructive::ScalarLaw::Linear {
            start: 1.0,
            end: -1.0,
        },
    };
    let frame = FrameLaw::FixedPlane {
        normal: Vector3::unit_x(),
    };
    let recipe = SpineFrameRecipe::new(spine, profile, frame);
    assert!(matches!(
        spine_sweep::spine_sweep::<LineSpine>(&recipe, &[0.0, 1.0]),
        Err(Refusal::UnsupportedEnvelope(_))
    ));
}
