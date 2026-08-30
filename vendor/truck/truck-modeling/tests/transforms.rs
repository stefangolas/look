//! BG-CAD-P10-FRAMED â€” the packet's truck-modeling acceptance tests.
//!
//! The general transforms (`rotate_solid`, `mirror_about_plane`), the
//! oblique circle extrusion through the LANDED `extrude_profile_vector`, and
//! the T(extrude) metamorphic. Every fixture is dyadic; the fold is exact,
//! so the mirrored/oblique-wall subs points are machine-checked at f64
//! equality (the Ï€/2 rotations use the same Rodrigues matrix the fold
//! composes, so both sides of each equality run the same float operations).
//!
//! SPEC_GAP NOTE (see the worktree QUESTION.md): the packet's test 1
//! (`rotate_solid_rigid_carriers`, a full-circle disk-extrude cylinder
//! rotated) cannot be realized in debug builds â€” the LANDED `Mapped`
//! machinery's same-vertex edge-construction assertion aborts on the disk's self-loop
//! circle edges (proven empirically: even the landed `translate_solid`
//! aborts on the full-circle disk). Every cylinder-walled solid in
//! truck-modeling carries self-loop circle edges, so no self-loop-free
//! substitute exists. Test 1 is therefore omitted and reported as a SPEC_GAP.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]
// Test-only allow: H-1 bans unwrap/expect/panic on paths reachable from
// untrusted geometry; test assertions on hand-built dyadic witnesses are not
// such a path. The deny list above stays; `expect_ok`/`expect_err` unwrap via
// `match` + `panic` so the deny lints stay satisfied.
#![allow(clippy::panic)]

use std::collections::HashSet;
use std::f64::consts::TAU;
use truck_base::evidence::{Outcome, Refusal};
use truck_geometry::arrange::{arrange, Arrangement};
use truck_modeling::cad::{mirror_about_plane, mirror_solid, rotate_solid, translate_solid};
use truck_modeling::extrude::{extrude_profile, extrude_profile_vector};
use truck_modeling::{
    Curve, Line, Matrix4, ParametricCurve, ParametricSurface, Plane, Point3, Processor, Rad, Solid,
    Surface, Transformed, TrimmedCurve, UnitCircle, Vector3, Vector4,
};
use truck_topology::EdgeID;

/// The extrude height of the box fixtures.
const BOX_HEIGHT: f64 = 2.0;
/// The box fixtures' side length.
const BOX_SIDE: f64 = 4.0;
/// The wall-subs sampling density for the W3 junction machine check.
const JUNCTION_SAMPLES: usize = 9;

/// Unwraps an `Outcome` via `match` + `panic` so the deny lints stay
/// satisfied (the recognize.rs test-module precedent).
fn expect_ok<T>(r: Outcome<T>) -> T {
    match r {
        Ok(ok) => ok.value,
        Err(refusal) => panic!("expected a certified value, got {refusal:?}"),
    }
}

/// Unwraps a refusal via `match` + `panic`.
fn expect_err<T>(r: Outcome<T>) -> Refusal {
    match r {
        Ok(_) => panic!("expected a refusal, got a certified value"),
        Err(refusal) => refusal,
    }
}

/// The `s Ã— s` CCW rectangle on z = 0 with its arrangement.
fn rect_profile(s: f64) -> (Vec<Curve>, Arrangement) {
    let profile = vec![
        Curve::Line(Line(Point3::new(0.0, 0.0, 0.0), Point3::new(s, 0.0, 0.0))),
        Curve::Line(Line(Point3::new(s, 0.0, 0.0), Point3::new(s, s, 0.0))),
        Curve::Line(Line(Point3::new(s, s, 0.0), Point3::new(0.0, s, 0.0))),
        Curve::Line(Line(Point3::new(0.0, s, 0.0), Point3::new(0.0, 0.0, 0.0))),
    ];
    let arrangement = expect_ok(arrange(&profile, None));
    (profile, arrangement)
}

/// A placed full-range circle at `center` with radius `r`: the exact
/// z-preserving uniform placement the recognizer's canonical form uses.
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

/// The probe's disk profile: the full circle r = 2 at (0, 0, 0).
fn disk_profile() -> (Vec<Curve>, Arrangement) {
    let profile = vec![circle_at(Point3::new(0.0, 0.0, 0.0), 2.0)];
    let arrangement = expect_ok(arrange(&profile, None));
    (profile, arrangement)
}

/// The sorted multiset of distinct vertex points of a solid.
fn unique_sorted_points(solid: &Solid) -> Vec<Point3> {
    let mut pts: Vec<Point3> = solid.vertex_iter().map(|v| v.point()).collect();
    pts.sort_by(|a, b| {
        a.x.total_cmp(&b.x)
            .then(a.y.total_cmp(&b.y))
            .then(a.z.total_cmp(&b.z))
    });
    pts.dedup();
    pts
}

/// The number of DISTINCT edges of a solid, by `EdgeID`.
fn unique_edge_count(solid: &Solid) -> usize {
    let mut ids: HashSet<EdgeID<Curve>> = HashSet::new();
    for face in solid.face_iter() {
        for wire in face.boundaries() {
            for edge in wire.edge_iter() {
                ids.insert(edge.id());
            }
        }
    }
    ids.len()
}

/// 2. The T(extrude) metamorphic on a rect profile with T = Rz(Ï€/2) +
///    translation (z-neutral): `T(extrude_profile(P, h))` equals
///    `extrude_profile(T(P), h)` in census, carrier kinds, and vertex points
///    â€” EXACTLY (both sides compose the same Rodrigues matrix and the same
///    dyadic translation on the same dyadic profile corners).
#[test]
fn rotate_about_z_extrude_metamorphic() {
    // The 2 x 2 rect is arrange-able under the Rodrigues rotation: its
    // opposite edges stay exactly anti-parallel (2c is exactly representable,
    // unlike 4c), so the exact-parallelogram gate of `arrange` holds.
    let (profile, arrangement) = rect_profile(2.0);
    let t = Vector3::new(1.0, 2.0, 0.0);
    let rz = Matrix4::from_axis_angle(
        Vector3::new(0.0, 0.0, 1.0),
        Rad(std::f64::consts::FRAC_PI_2),
    );
    let m = Matrix4::from_translation(t) * rz;

    let solid = expect_ok(extrude_profile(&profile, &arrangement, BOX_HEIGHT));

    // T(A): rotate about z through the origin, then translate, through the
    // landed fold entries.
    let t_of_a = expect_ok(translate_solid(
        &expect_ok(rotate_solid(
            &solid,
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
            std::f64::consts::FRAC_PI_2,
        )),
        t,
    ));
    // T(P): rotate + translate each profile curve, re-arrange, extrude.
    let t_profile: Vec<Curve> = profile.iter().map(|c| c.transformed(m)).collect();
    let t_arrangement = expect_ok(arrange(&t_profile, None));
    let t_of_p = expect_ok(extrude_profile(&t_profile, &t_arrangement, BOX_HEIGHT));

    // Census equal, and the box shape is exactly the rotated box.
    assert_eq!(t_of_a.face_iter().count(), t_of_p.face_iter().count());
    assert_eq!(t_of_a.face_iter().count(), 6);
    assert_eq!(t_of_a.edge_iter().count(), t_of_p.edge_iter().count());
    assert_eq!(t_of_a.vertex_iter().count(), t_of_p.vertex_iter().count());
    // Carrier kinds: every face stays a BARE plane on both sides.
    for side in [&t_of_a, &t_of_p] {
        for face in side.face_iter() {
            assert!(
                matches!(face.surface(), Surface::Plane(_)),
                "the metamorphic keeps every carrier bare"
            );
        }
    }
    // Vertex points equal EXACTLY.
    assert_eq!(
        unique_sorted_points(&t_of_a),
        unique_sorted_points(&t_of_p),
        "T(extrude(P)) and extrude(T(P)) share the exact vertex set"
    );
}

/// 3. A box mirrored about the plane through (1, 1, 0) with normal (1, 1, 0)
///    assembles; the mirrored vertices equal the hand-computed reflection
///    images EXACTLY. The Householder `I - 2nn^T/(nÂ·n)` for n = (1, 1, 0)
///    needs only nÂ·n = 2, so the whole matrix is exactly dyadic and the
///    reflection (x, y, z) â†¦ (2 âˆ’ y, 2 âˆ’ x, z) is f64-exact.
#[test]
fn mirror_general_plane_assembles() {
    let (profile, arrangement) = rect_profile(BOX_SIDE);
    let solid = expect_ok(extrude_profile(&profile, &arrangement, BOX_HEIGHT));
    let mirrored = expect_ok(mirror_about_plane(
        &solid,
        Point3::new(1.0, 1.0, 0.0),
        Vector3::new(1.0, 1.0, 0.0),
    ));
    assert!(Solid::try_new(mirrored.boundaries().clone()).is_ok());

    let hand: Vec<Point3> = solid
        .vertex_iter()
        .map(|v| {
            let p = v.point();
            Point3::new(2.0 - p.y, 2.0 - p.x, p.z)
        })
        .collect();
    let mut expected = hand.clone();
    expected.sort_by(|a, b| {
        a.x.total_cmp(&b.x)
            .then(a.y.total_cmp(&b.y))
            .then(a.z.total_cmp(&b.z))
    });
    expected.dedup();
    assert_eq!(
        unique_sorted_points(&mirrored),
        expected,
        "the mirrored vertices are the exact reflection images"
    );
}

/// 4. The identity guard: the landed `mirror_solid` on the box across x = 0
///    answers exactly what it answered before the packet â€” the box
///    [0,4]Ã—[0,4]Ã—[0,2] mirrored is [âˆ’4,0]Ã—[0,4]Ã—[0,2], vertex-exact, with
///    the same census.
#[test]
fn mirror_axis_aligned_still_green() {
    let (profile, arrangement) = rect_profile(BOX_SIDE);
    let solid = expect_ok(extrude_profile(&profile, &arrangement, BOX_HEIGHT));
    let plane = Plane::new(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(0.0, 0.0, 1.0),
        Point3::new(0.0, 1.0, 0.0),
    );
    let mirrored = expect_ok(mirror_solid(&solid, &plane));
    assert_eq!(mirrored.face_iter().count(), 6);
    assert!(Solid::try_new(mirrored.boundaries().clone()).is_ok());

    let mut expected = vec![
        Point3::new(-4.0, 0.0, 0.0),
        Point3::new(-4.0, 4.0, 0.0),
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(0.0, 4.0, 0.0),
        Point3::new(-4.0, 0.0, 2.0),
        Point3::new(-4.0, 4.0, 2.0),
        Point3::new(0.0, 0.0, 2.0),
        Point3::new(0.0, 4.0, 2.0),
    ];
    expected.sort_by(|a, b| {
        a.x.total_cmp(&b.x)
            .then(a.y.total_cmp(&b.y))
            .then(a.z.total_cmp(&b.z))
    });
    assert_eq!(
        unique_sorted_points(&mirrored),
        expected,
        "the axis-aligned mirror keeps its pre-packet vertex answers"
    );
}

/// 5. The probe's W2 THROUGH THE LANDED `extrude_profile_vector`: disk
///    r = 2 at z = 0, dir (1, 0, 1): 3 faces, 2 unique edges, 2 unique
///    vertices; the wall is the placed affine cylinder whose subs points
///    interpolate the junction circles exactly (W3's check); the caps sit at
///    z = 0 and z = 1.
#[test]
fn oblique_extrude_circle_assembles() {
    let (profile, arrangement) = disk_profile();
    let solid = expect_ok(extrude_profile_vector(
        &profile,
        &arrangement,
        Vector3::new(1.0, 0.0, 1.0),
        false,
    ));

    // W2 census: 3 faces, 2 unique edges, 2 unique vertices.
    assert_eq!(solid.face_iter().count(), 3);
    assert_eq!(unique_edge_count(&solid), 2);
    let mut vertex_points: Vec<Point3> = Vec::new();
    for face in solid.face_iter() {
        for wire in face.boundaries() {
            for edge in wire.edge_iter() {
                vertex_points.push(edge.front().point());
                vertex_points.push(edge.back().point());
            }
        }
    }
    vertex_points.sort_by(|a, b| {
        a.x.total_cmp(&b.x)
            .then(a.y.total_cmp(&b.y))
            .then(a.z.total_cmp(&b.z))
    });
    vertex_points.dedup();
    assert_eq!(vertex_points.len(), 2, "two unique self-loop vertices");

    // Caps at z = 0 and z = 1: two bare plane faces, one on each cap plane.
    let mut cap_zs: Vec<f64> = Vec::new();
    let mut wall: Option<Surface> = None;
    for face in solid.face_iter() {
        match face.surface() {
            Surface::Plane(plane) => {
                assert!(plane.origin().z == 0.0 || plane.origin().z == 1.0);
                cap_zs.push(plane.origin().z);
            }
            Surface::Processor(_) => wall = Some(face.surface()),
            other => panic!("unexpected oblique-extrude carrier {other:?}"),
        }
    }
    cap_zs.sort_by(|a, b| a.total_cmp(b));
    assert_eq!(cap_zs, vec![0.0, 1.0]);
    let placed = match wall {
        Some(Surface::Processor(placed)) => placed,
        other => panic!("expected a placed affine wall, got {other:?}"),
    };
    let Surface::Cylinder(inner) = &**placed.entity() else {
        panic!("the placed wall's inner carrier is the bare right cylinder");
    };
    assert_eq!(inner.center(), Point3::new(0.0, 0.0, 0.0));
    assert_eq!(inner.radius(), 2.0);

    // W3's machine check: the wall's subs points at v = 0 and v = 1 are the
    // junction circles exactly â€” `(2 cos u, 2 sin u, 0)` on the bottom cap
    // and `(2 cos u + 1, 2 sin u, 1)` on the top cap (dir = (1, 0, 1)).
    let bottom = circle_at(Point3::new(0.0, 0.0, 0.0), 2.0);
    let top = circle_at(Point3::new(1.0, 0.0, 1.0), 2.0);
    for k in 0..JUNCTION_SAMPLES {
        let u = TAU * (k as f64) / (JUNCTION_SAMPLES as f64);
        assert_eq!(
            placed.subs(u, 0.0),
            bottom.subs(u),
            "bottom junction at u = {u}"
        );
        assert_eq!(placed.subs(u, 1.0), top.subs(u), "top junction at u = {u}");
    }
}

/// 6. `dir` with dz == 0 refuses `Refusal::Empty` (the landed zero-volume
///    arm, machine-checked).
#[test]
fn oblique_extrude_refuses_dz0() {
    let (profile, arrangement) = disk_profile();
    let err = expect_err(extrude_profile_vector(
        &profile,
        &arrangement,
        Vector3::new(1.0, 0.0, 0.0),
        false,
    ));
    assert!(
        matches!(err, Refusal::Empty),
        "a z-neutral oblique dir must refuse Empty, got {err:?}"
    );
}
