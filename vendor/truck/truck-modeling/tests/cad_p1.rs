//! BG-CAD-P1-UTILITY — the packet's ten required acceptance tests.
//!
//! The certified utility surface (`solid_bounding_box`, the similarity fold)
//! and planar face construction (`make_face`, `make_hull`), exercised on the
//! extrude.rs test pattern: the 4×4 rectangle flagship and the
//! rectangle-minus-circle profile.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]
// Test-only allow: H-1 bans unwrap/expect/panic on paths reachable from
// untrusted geometry; test assertions on hand-built witnesses are not such a
// path (the recognize.rs test-module precedent). The deny list above stays;
// `expect_ok` unwraps via `match` + `panic` so the deny lints stay satisfied.
#![allow(clippy::panic)]

use std::f64::consts::TAU;
use truck_base::evidence::{Budget, EnvelopeCase, Outcome, Refusal};
use truck_geometry::arrange::arrange;
use truck_geometry::recognize::{recognize_curve, recognize_surface, CanonicalCarrierWitness};
use truck_modeling::cad::{
    make_face, make_hull, mirror_solid, solid_bounding_box, translate_solid, uniform_scale_solid,
};
use truck_modeling::extrude::extrude_profile;
use truck_modeling::{
    Curve, Face, Line, Matrix4, Plane, Point3, Processor, Solid, Surface, TrimmedCurve, UnitCircle,
    Vector3, Vector4,
};

/// The height of the flagship extrude.
const FLAGSHIP_HEIGHT: f64 = 2.0;
/// The flagship rectangle's side.
const FLAGSHIP_SIDE: f64 = 4.0;
/// Sampling density for the cylinder-wall machine-check witness.
const WALL_THETA_SAMPLES: usize = 24;
const WALL_V_SAMPLES: usize = 8;

/// Unwraps an `Outcome` via `match` + `panic` so the deny lints stay
/// satisfied (the recognize.rs test-module precedent).
fn expect_ok<T>(r: Outcome<T>) -> T {
    match r {
        Ok(ok) => ok.value,
        Err(refusal) => panic!("expected a certified value, got {refusal:?}"),
    }
}

/// The 4×4 CCW rectangle on z = 0 (the extrude.rs test pattern).
fn rectangle_profile() -> Vec<Curve> {
    vec![
        Curve::Line(Line(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(FLAGSHIP_SIDE, 0.0, 0.0),
        )),
        Curve::Line(Line(
            Point3::new(FLAGSHIP_SIDE, 0.0, 0.0),
            Point3::new(FLAGSHIP_SIDE, FLAGSHIP_SIDE, 0.0),
        )),
        Curve::Line(Line(
            Point3::new(FLAGSHIP_SIDE, FLAGSHIP_SIDE, 0.0),
            Point3::new(0.0, FLAGSHIP_SIDE, 0.0),
        )),
        Curve::Line(Line(
            Point3::new(0.0, FLAGSHIP_SIDE, 0.0),
            Point3::new(0.0, 0.0, 0.0),
        )),
    ]
}

/// A placed full-range unit circle with the given center and radius: the
/// exact z-preserving uniform placement the recognizer's canonical form uses.
fn circle_at(center: Point3, radius: f64) -> Curve {
    Curve::Circle(Processor::with_transform(
        TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, TAU)),
        Matrix4 {
            x: Vector4::new(radius, 0.0, 0.0, 0.0),
            y: Vector4::new(0.0, radius, 0.0, 0.0),
            z: Vector4::new(0.0, 0.0, 1.0, 0.0),
            w: Vector4::new(center.x, center.y, center.z, 1.0),
        },
    ))
}

/// The flagship profile: the rectangle plus a full circle r = 1 at (2, 2).
fn plate_with_hole_profile() -> Vec<Curve> {
    let mut profile = rectangle_profile();
    profile.push(circle_at(Point3::new(2.0, 2.0, 0.0), 1.0));
    profile
}

/// The flagship solid: the rectangle profile extruded to height 2 — a
/// six-plane box spanning [0, 4] × [0, 4] × [0, 2].
fn flagship_box() -> Solid {
    let profile = rectangle_profile();
    let arrangement = expect_ok(arrange(&profile, None));
    expect_ok(extrude_profile(&profile, &arrangement, FLAGSHIP_HEIGHT))
}

/// The mirror across x = 0: the plane with normal −x through the origin.
fn mirror_plane_x0() -> Plane {
    Plane::new(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(0.0, 0.0, 1.0),
        Point3::new(0.0, 1.0, 0.0),
    )
}

/// Same face/edge/wire counts and the same per-face wire shape: the topology
/// STRUCTURE of the similarity fold is identical (same shared-edge identity
/// pattern `Mapped` already preserves).
fn assert_structure_congruent(a: &Solid, b: &Solid) {
    let faces0: Vec<Face> = a.face_iter().cloned().collect();
    let faces1: Vec<Face> = b.face_iter().cloned().collect();
    assert_eq!(faces0.len(), faces1.len(), "face count");
    for (f0, f1) in faces0.iter().zip(faces1.iter()) {
        let w0 = f0.absolute_boundaries();
        let w1 = f1.absolute_boundaries();
        assert_eq!(w0.len(), w1.len(), "wire count per face");
        for (wire0, wire1) in w0.iter().zip(w1.iter()) {
            assert_eq!(wire0.len(), wire1.len(), "edge count per wire");
        }
    }
}

/// Every vertex point, as a sorted multiset (the traversal order of the fold's
/// output is the input's, but the multiset comparison is order-independent).
fn sorted_vertex_points(solid: &Solid) -> Vec<Point3> {
    let mut pts: Vec<Point3> = solid.vertex_iter().map(|v| v.point()).collect();
    pts.sort_by(|a, b| {
        let by_x = a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal);
        by_x.then(a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal))
    });
    pts
}

/// Every face surface and edge curve of the solid carries a recognized
/// canonical carrier — the fold's downstream-consumability contract.
fn all_carriers_recognized(solid: &Solid) -> bool {
    solid.face_iter().all(|face| {
        let surface = face.surface();
        !matches!(
            recognize_surface(&surface),
            CanonicalCarrierWitness::Unrecognized
        )
    }) && solid.edge_iter().all(|edge| {
        let curve = edge.curve();
        !matches!(
            recognize_curve(&curve),
            CanonicalCarrierWitness::Unrecognized
        )
    })
}

/// 1. The box derivation is closed-form on planes: the flagship's box is the
///    exact dyadic box [0,4]×[0,4]×[0,2] — min/max corners exact.
///
///    The D2 cylinder machine-check witness rides in this test's family: the
///    extruded bare circle (the disk). The wall's extreme xy is achieved on
///    the rims (the radius is constant in v) and its z-extent is bracketed by
///    the rim circles, so the derived box must be exactly the rims' hull —
///    and every sampled interior-wall point must lie inside it. A falsified
///    assert here is a stop-and-report SPEC_GAP, not a widened rule.
#[test]
fn bounding_box_of_flagship_extrude_is_exact() {
    let solid = flagship_box();
    let mut budget = Budget::new(0, 0, 0);
    let hull = expect_ok(solid_bounding_box(&solid, &mut budget));
    assert_eq!(hull.min(), Point3::new(0.0, 0.0, 0.0));
    assert_eq!(
        hull.max(),
        Point3::new(FLAGSHIP_SIDE, FLAGSHIP_SIDE, FLAGSHIP_HEIGHT)
    );

    // The cylinder-wall witness: extrude the bare circle and machine-check
    // the rim-hull rule on the wall.
    let profile = vec![circle_at(Point3::new(2.0, 2.0, 0.0), 1.0)];
    let arrangement = expect_ok(arrange(&profile, None));
    let disk = expect_ok(extrude_profile(&profile, &arrangement, FLAGSHIP_HEIGHT));
    let disk_hull = expect_ok(solid_bounding_box(&disk, &mut budget));
    // Exactly the rims' hull: [2−1, 2+1] × [2−1, 2+1] × [0, 2].
    assert_eq!(disk_hull.min(), Point3::new(1.0, 1.0, 0.0));
    assert_eq!(disk_hull.max(), Point3::new(3.0, 3.0, FLAGSHIP_HEIGHT));
    // Sample the wall strictly between the rims: every point of the wall's
    // interior must lie inside the derived box.
    for k in 0..WALL_THETA_SAMPLES {
        let theta = TAU * (k as f64) / (WALL_THETA_SAMPLES as f64);
        for j in 1..WALL_V_SAMPLES {
            let z = FLAGSHIP_HEIGHT * (j as f64) / (WALL_V_SAMPLES as f64);
            let p = Point3::new(2.0 + theta.cos(), 2.0 + theta.sin(), z);
            assert!(
                disk_hull.contains(p),
                "cylinder wall sample {p:?} escaped the derived box"
            );
        }
    }
}

/// 2. Translate by (1, 2, 3): same face/edge/wire counts, every vertex point
///    shifted exactly, box shifted exactly, `Solid::try_new` ok, all carriers
///    still recognized.
#[test]
fn translated_solid_is_congruent() {
    let solid = flagship_box();
    let t = Vector3::new(1.0, 2.0, 3.0);
    let translated = expect_ok(translate_solid(&solid, t));
    assert_structure_congruent(&solid, &translated);
    for (a, b) in sorted_vertex_points(&solid)
        .iter()
        .zip(sorted_vertex_points(&translated).iter())
    {
        assert_eq!(*b, *a + t, "vertex point must shift exactly");
    }
    let mut budget = Budget::new(0, 0, 0);
    let hull = expect_ok(solid_bounding_box(&translated, &mut budget));
    assert_eq!(hull.min(), Point3::new(1.0, 2.0, 3.0));
    assert_eq!(hull.max(), Point3::new(5.0, 6.0, 5.0));
    assert!(Solid::try_new(translated.boundaries().clone()).is_ok());
    assert!(all_carriers_recognized(&translated));
}

/// 3. Scale 2.0 about the origin: same structure, box doubled, carriers
///    canonical.
#[test]
fn uniform_scaled_solid_is_congruent() {
    let solid = flagship_box();
    let scaled = expect_ok(uniform_scale_solid(&solid, 2.0));
    assert_structure_congruent(&solid, &scaled);
    let mut budget = Budget::new(0, 0, 0);
    let hull = expect_ok(solid_bounding_box(&scaled, &mut budget));
    assert_eq!(hull.min(), Point3::new(0.0, 0.0, 0.0));
    assert_eq!(hull.max(), Point3::new(8.0, 8.0, 4.0));
    assert!(Solid::try_new(scaled.boundaries().clone()).is_ok());
    assert!(all_carriers_recognized(&scaled));
}

/// 4. Mirror across x = 0 (plane normal −x through the origin): same
///    structure, try_new ok, carriers canonical.
#[test]
fn mirrored_solid_is_congruent() {
    let solid = flagship_box();
    let mirrored = expect_ok(mirror_solid(&solid, &mirror_plane_x0()));
    assert_structure_congruent(&solid, &mirrored);
    assert!(Solid::try_new(mirrored.boundaries().clone()).is_ok());
    assert!(all_carriers_recognized(&mirrored));
}

/// 5. The mirrored flagship's box is the exact reflection of test 1's box:
///    [0,4] × [0,4] × [0,2] mirrored in x = 0 is [−4,0] × [0,4] × [0,2].
#[test]
fn mirrored_flagship_box_is_reflected() {
    let solid = flagship_box();
    let mirrored = expect_ok(mirror_solid(&solid, &mirror_plane_x0()));
    let mut budget = Budget::new(0, 0, 0);
    let hull = expect_ok(solid_bounding_box(&mirrored, &mut budget));
    assert_eq!(hull.min(), Point3::new(-4.0, 0.0, 0.0));
    assert_eq!(hull.max(), Point3::new(0.0, FLAGSHIP_SIDE, FLAGSHIP_HEIGHT));
}

/// 6. Rectangle profile → `Vec` of exactly 1 face, 1 boundary wire, on the
///    z = 0 plane, orientation such that the face normal is +z.
#[test]
fn make_face_rectangle() {
    let faces = expect_ok(make_face(&rectangle_profile()));
    assert_eq!(faces.len(), 1);
    let face = match faces.first() {
        Some(face) => face,
        None => panic!("expected exactly one face"),
    };
    let wires = face.boundaries();
    assert_eq!(wires.len(), 1);
    let Surface::Plane(plane) = face.surface() else {
        panic!("the face must be planar");
    };
    assert_eq!(plane.origin().z, 0.0);
    assert_eq!(plane.normal(), Vector3::new(0.0, 0.0, 1.0));
    assert!(face.orientation(), "the face normal must be +z");
}

/// 7. Rectangle-minus-circle profile (the flagship profile) → 1 face with 2
///    boundary wires, NO seam edges: the boundary carries exactly the
///    profile's edges — the outer rectangle's four line edges and the hole's
///    single circle self-loop — and every edge endpoint lies on z = 0 (a
///    vertical seam edge would not).
#[test]
fn make_face_with_hole() {
    let faces = expect_ok(make_face(&plate_with_hole_profile()));
    assert_eq!(faces.len(), 1);
    let face = match faces.first() {
        Some(face) => face,
        None => panic!("expected exactly one face"),
    };
    let wires = face.boundaries();
    assert_eq!(wires.len(), 2, "the annulus carries two boundary wires");
    let mut wire_lens: Vec<usize> = Vec::new();
    let mut edge_count = 0usize;
    for wire in &wires {
        wire_lens.push(wire.len());
        for edge in wire.edge_iter() {
            edge_count += 1;
            assert_eq!(edge.front().point().z, 0.0, "no seam edges: z = 0 only");
            assert_eq!(edge.back().point().z, 0.0, "no seam edges: z = 0 only");
        }
    }
    assert_eq!(edge_count, 5);
    wire_lens.sort();
    assert_eq!(wire_lens, vec![1, 4], "outer 4-edge wire, 1-edge hole wire");
    let Surface::Plane(plane) = face.surface() else {
        panic!("the face must be planar");
    };
    assert_eq!(plane.origin().z, 0.0);
}

/// 8. A point set whose hull is the unit square → 1 face, 4 edges, +z normal.
#[test]
fn make_hull_square() {
    let points = vec![
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(1.0, 1.0, 0.0),
        Point3::new(0.0, 1.0, 0.0),
        Point3::new(0.5, 0.5, 0.0),
    ];
    let face = expect_ok(make_hull(&points));
    let wires = face.boundaries();
    assert_eq!(wires.len(), 1);
    let wire = match wires.first() {
        Some(wire) => wire,
        None => panic!("expected one boundary wire"),
    };
    assert_eq!(wire.len(), 4);
    let Surface::Plane(plane) = face.surface() else {
        panic!("the face must be planar");
    };
    assert_eq!(plane.normal(), Vector3::new(0.0, 0.0, 1.0));
    assert!(face.orientation(), "the face normal must be +z");
}

/// 9. Three collinear points → the `Collapsed` refusal.
#[test]
fn make_hull_degenerate_collapses() {
    let points = vec![
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 1.0, 0.0),
        Point3::new(2.0, 2.0, 0.0),
    ];
    match make_hull(&points) {
        Err(Refusal::Collapsed(..)) => {}
        other => panic!("expected the Collapsed refusal, got {other:?}"),
    }
}

/// 10. `make_face` on a profile with one vertex at z = 1 →
///     `UnsupportedEnvelope`, machine-checked as `NonCanonicalCarrier`.
#[test]
fn profile_off_plane_refuses() {
    let profile = vec![
        Curve::Line(Line(Point3::new(0.0, 0.0, 0.0), Point3::new(4.0, 0.0, 0.0))),
        Curve::Line(Line(Point3::new(4.0, 0.0, 0.0), Point3::new(4.0, 4.0, 0.0))),
        Curve::Line(Line(Point3::new(4.0, 4.0, 0.0), Point3::new(0.0, 4.0, 1.0))),
        Curve::Line(Line(Point3::new(0.0, 4.0, 1.0), Point3::new(0.0, 0.0, 0.0))),
    ];
    match make_face(&profile) {
        Err(Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)) => {}
        other => panic!("expected UnsupportedEnvelope(NonCanonicalCarrier), got {other:?}"),
    }
}
