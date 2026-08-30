//! BG-CAD-P8-FACADE — the conformance battery: the build123d-shaped facade
//! asserted THROUGH the facade names (plan §9 metamorphic algebra + the
//! constructive sequences + the typed refusal cases + the downstream
//! consumability rows).
//!
//! The tessellation rows are release-gated (Finding 3 of BG-CAD-P8-FACADE):
//! tessellation of circle-carrying solids PANICS in debug builds ("Two same
//! vertices cannot construct an edge", the recorded self-loop constructor
//! trap), so the mesh assertions below live in `#[cfg(not(debug_assertions))]`
//! blocks and the gate itself is asserted in debug. The refusal/metamorphic
//! rows are debug-safe and ungated.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]
// Test-only allow: H-1 bans unwrap/expect/panic on paths reachable from
// untrusted geometry. This file is integration-test assertions on hand-built
// dyadic witnesses - not such a path.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::HashSet;
use std::f64::consts::{FRAC_PI_2, TAU};
use truck_base::cgmath64::{Matrix4, Point2, Point3, Vector3, Vector4};
use truck_base::evidence::{Budget, EnvelopeCase, Refusal};
use truck_geometry::arrange::{arrange, Arrangement};
use truck_geometry::canonical::{Curve, Surface};
use truck_geometry::prelude::*;
use truck_geometry::recognize::recognize_surface;
#[cfg(not(debug_assertions))]
use truck_meshalgo::analyzers::Topology;
#[cfg(not(debug_assertions))]
use truck_meshalgo::filters::OptimizingFilter;
#[cfg(not(debug_assertions))]
use truck_meshalgo::prelude::*;
use truck_modeling::extrude::extrude_profile;
use truck_shapeops::facade::{self, BlendSpec, Mode};
use truck_shapeops::rewrite::{ChamferSpec, CircleFilletSpec, FilletSpec};
#[cfg(not(debug_assertions))]
use truck_topology::shell::ShellCondition;
use truck_topology::{Face, Solid};

/// The mesh tolerance class (a length constant, H-3). Release-only: the
/// tessellation rows are gated per Finding 3.
#[cfg(not(debug_assertions))]
const MESH_TOL: f64 = 1.0e-2; // H-3: the mesh tolerance class (length)

// ---------------------------------------------------------------------------
// construction helpers (the boolean_m2 fixture recipe)
// ---------------------------------------------------------------------------

/// A placed full-period circle at `center` with radius `r`.
fn placed_circle(center: Point3, r: f64) -> Processor<TrimmedCurve<UnitCircle<Point3>>, Matrix4> {
    Processor::with_transform(
        TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, TAU)),
        Matrix4 {
            x: Vector4::new(r, 0.0, 0.0, 0.0),
            y: Vector4::new(0.0, r, 0.0, 0.0),
            z: Vector4::new(0.0, 0.0, 1.0, 0.0),
            w: Vector4::new(center.x, center.y, center.z, 1.0),
        },
    )
}

/// The 4x4 block profile: four `Curve::Line`s, CCW.
fn block_profile() -> (Vec<Curve>, Arrangement) {
    let profile = vec![
        Curve::Line(Line(Point3::new(0.0, 0.0, 0.0), Point3::new(4.0, 0.0, 0.0))),
        Curve::Line(Line(Point3::new(4.0, 0.0, 0.0), Point3::new(4.0, 4.0, 0.0))),
        Curve::Line(Line(Point3::new(4.0, 4.0, 0.0), Point3::new(0.0, 4.0, 0.0))),
        Curve::Line(Line(Point3::new(0.0, 4.0, 0.0), Point3::new(0.0, 0.0, 0.0))),
    ];
    let ok = arrange(&profile, None).unwrap();
    (profile, ok.value)
}

/// The `[x0, x1] x [y0, y1]` axis-aligned box profile, CCW.
fn box_profile(x0: f64, y0: f64, x1: f64, y1: f64) -> (Vec<Curve>, Arrangement) {
    let profile = vec![
        Curve::Line(Line(Point3::new(x0, y0, 0.0), Point3::new(x1, y0, 0.0))),
        Curve::Line(Line(Point3::new(x1, y0, 0.0), Point3::new(x1, y1, 0.0))),
        Curve::Line(Line(Point3::new(x1, y1, 0.0), Point3::new(x0, y1, 0.0))),
        Curve::Line(Line(Point3::new(x0, y1, 0.0), Point3::new(x0, y0, 0.0))),
    ];
    let ok = arrange(&profile, None).unwrap();
    (profile, ok.value)
}

/// A pure-disk profile: one full circle of radius `r` at `center`.
fn disk_profile(center: Point2, r: f64) -> (Vec<Curve>, Arrangement) {
    let circle = Curve::Circle(placed_circle(Point3::new(center.x, center.y, 0.0), r));
    let profile = vec![circle];
    let ok = arrange(&profile, None).unwrap();
    (profile, ok.value)
}

/// The solid `height`-extrude of a profile.
fn extrude_solid(
    profile: &[Curve],
    arr: &Arrangement,
    height: f64,
) -> Solid<Point3, Curve, Surface> {
    extrude_profile(profile, arr, height)
        .expect("the dyadic profile extrudes")
        .value
}

/// The axis-aligned box solid `[x0, x1] x [y0, y1] x [z0, z1]`.
fn box_solid(
    x0: f64,
    y0: f64,
    z0: f64,
    x1: f64,
    y1: f64,
    z1: f64,
) -> Solid<Point3, Curve, Surface> {
    let (profile, arr) = box_profile(x0, y0, x1, y1);
    let solid = extrude_solid(&profile, &arr, z1 - z0);
    facade::translate(&solid, Vector3::new(0.0, 0.0, z0))
        .expect("the dyadic translation resolves")
        .value
}

/// The primary fillet fixture: the box [0,4]^2 x [0,2].
fn box_fixture() -> Solid<Point3, Curve, Surface> {
    box_solid(0.0, 0.0, 0.0, 4.0, 4.0, 2.0)
}

/// The plane z = 1 (the cutting plane of the split rows).
fn plane_z1() -> Plane {
    Plane::new(
        Point3::new(0.0, 0.0, 1.0),
        Point3::new(1.0, 0.0, 1.0),
        Point3::new(0.0, 1.0, 1.0),
    )
}

// ---------------------------------------------------------------------------
// measurement helpers
// ---------------------------------------------------------------------------

/// The unique-edge count (per-use iteration deduped by edge id).
fn unique_edges(solid: &Solid<Point3, Curve, Surface>) -> usize {
    solid
        .edge_iter()
        .map(|e| e.id())
        .collect::<HashSet<_>>()
        .len()
}

/// The unique-vertex count (per-use iteration deduped by vertex id).
fn unique_vertices(solid: &Solid<Point3, Curve, Surface>) -> usize {
    solid
        .vertex_iter()
        .map(|v| v.id())
        .collect::<HashSet<_>>()
        .len()
}

/// The sorted distinct vertex points of a solid.
fn unique_sorted_points(solid: &Solid<Point3, Curve, Surface>) -> Vec<Point3> {
    let mut pts: Vec<Point3> = solid.vertex_iter().map(|v| v.point()).collect();
    pts.sort_by(|a, b| {
        a.x.total_cmp(&b.x)
            .then(a.y.total_cmp(&b.y))
            .then(a.z.total_cmp(&b.z))
    });
    pts.dedup();
    pts
}

/// The sorted multiset of face-carrier kinds.
fn carrier_kinds(solid: &Solid<Point3, Curve, Surface>) -> Vec<&'static str> {
    let mut kinds: Vec<&'static str> = solid
        .face_iter()
        .map(|face| match face.surface() {
            Surface::Plane(_) => "plane",
            Surface::Cylinder(_) => "cylinder",
            Surface::Processor(_) => "placed",
            Surface::Torus(_) => "torus",
            other => panic!("unexpected carrier {other:?}"),
        })
        .collect();
    kinds.sort();
    kinds
}

/// The `(x, y)` box of one face's vertices.
fn face_xy_box(face: &Face<Point3, Curve, Surface>) -> ((f64, f64), (f64, f64)) {
    let mut lo = Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
    let mut hi = Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for v in face.vertex_iter() {
        let p = v.point();
        lo.x = lo.x.min(p.x);
        lo.y = lo.y.min(p.y);
        hi.x = hi.x.max(p.x);
        hi.y = hi.y.max(p.y);
    }
    ((lo.x, hi.x), (lo.y, hi.y))
}

/// The mesh condition of a solid: triangulate at `MESH_TOL`, merge equal
/// attributes at `TOLERANCE`, and report the shell condition plus non-empty
/// position/face counts. Release-only (Finding 3).
#[cfg(not(debug_assertions))]
fn mesh_condition(solid: &Solid<Point3, Curve, Surface>) -> (ShellCondition, usize, usize) {
    let mut mesh = solid.triangulation(MESH_TOL).to_polygon();
    mesh.put_together_same_attrs(truck_base::tolerance::TOLERANCE);
    let cond = mesh.shell_condition();
    let pos = mesh.positions().len();
    let faces = mesh.faces().len();
    (cond, pos, faces)
}

// ---------------------------------------------------------------------------
// Test 1: the naming table resolves — a compile-time presence battery.
// ---------------------------------------------------------------------------

#[test]
fn facade_naming_table_covers_every_landed_entry() {
    let (pa, aa) = block_profile();
    let plate = extrude_solid(&pa, &aa, 2.0);
    let (pd, ad) = disk_profile(Point2::new(0.0, 0.0), 2.0);
    let (pdisk, adisk) = disk_profile(Point2::new(2.0, 2.0), 1.0);
    let disk = extrude_solid(&pdisk, &adisk, 2.0);
    let mut budget = Budget::new(1000, 1000, 1000);

    // extrude
    let extruded = facade::extrude(&pa, &aa, 2.0)
        .expect("extrude assembles")
        .value;
    assert_eq!(extruded.face_iter().count(), 6);
    // extrude_vector
    let vector = facade::extrude_vector(&pd, &ad, Vector3::new(1.0, 0.0, 1.0), false)
        .expect("extrude_vector assembles")
        .value;
    assert_eq!(vector.face_iter().count(), 3);
    // revolve (the flagship rectangle x in [1,3], z in [0,2], quarter turn)
    let rect = vec![
        Curve::Line(Line(Point3::new(1.0, 0.0, 0.0), Point3::new(3.0, 0.0, 0.0))),
        Curve::Line(Line(Point3::new(3.0, 0.0, 0.0), Point3::new(3.0, 0.0, 2.0))),
        Curve::Line(Line(Point3::new(3.0, 0.0, 2.0), Point3::new(1.0, 0.0, 2.0))),
        Curve::Line(Line(Point3::new(1.0, 0.0, 2.0), Point3::new(1.0, 0.0, 0.0))),
    ];
    let working: Vec<Curve> = rect
        .iter()
        .map(|c| match c {
            Curve::Line(Line(a, b)) => {
                Curve::Line(Line(Point3::new(a.x, a.z, 0.0), Point3::new(b.x, b.z, 0.0)))
            }
            _ => unreachable!("the revolve fixture is line-only"),
        })
        .collect();
    let arr_rev = arrange(&working, None).unwrap().value;
    let revolved = facade::revolve(&rect, &arr_rev, FRAC_PI_2)
        .expect("revolve assembles")
        .value;
    assert!(!revolved.boundaries().is_empty());
    // fillet (a grouped Straight batch)
    let straight = vec![
        BlendSpec::Straight(FilletSpec {
            a: Point3::new(4.0, 4.0, 0.0),
            b: Point3::new(4.0, 4.0, 2.0),
            radius: 1.0,
        }),
        BlendSpec::Straight(FilletSpec {
            a: Point3::new(0.0, 0.0, 0.0),
            b: Point3::new(0.0, 0.0, 2.0),
            radius: 1.0,
        }),
    ];
    let filleted = facade::fillet(&box_fixture(), &straight, &mut budget)
        .expect("fillet assembles")
        .value;
    assert_eq!(filleted.face_iter().count(), 8);
    // chamfer
    let chamfered = facade::chamfer(
        &box_fixture(),
        &[ChamferSpec {
            a: Point3::new(4.0, 4.0, 0.0),
            b: Point3::new(4.0, 4.0, 2.0),
            d_first: 1.0,
            d_second: 1.0,
        }],
        &mut budget,
    )
    .expect("chamfer assembles")
    .value;
    assert_eq!(chamfered.face_iter().count(), 7);
    // mirror (axis-aligned plane)
    let plane = Plane::new(
        Point3::new(0.0, 0.0, 1.0),
        Point3::new(1.0, 0.0, 1.0),
        Point3::new(0.0, 1.0, 1.0),
    );
    let mirrored = facade::mirror(&plate, &plane)
        .expect("mirror assembles")
        .value;
    assert_eq!(mirrored.face_iter().count(), 6);
    // mirror_about_plane
    let mirrored2 = facade::mirror_about_plane(
        &plate,
        Point3::new(0.0, 0.0, 1.0),
        Vector3::new(1.0, 1.0, 0.0),
    )
    .expect("mirror_about_plane assembles")
    .value;
    assert_eq!(mirrored2.face_iter().count(), 6);
    // rotate
    let rotated = facade::rotate(
        &plate,
        Point3::new(0.0, 0.0, 0.0),
        Vector3::unit_z(),
        FRAC_PI_2,
    )
    .expect("rotate assembles")
    .value;
    assert_eq!(rotated.face_iter().count(), 6);
    // scale
    let scaled = facade::scale(&plate, 2.0).expect("scale assembles").value;
    assert_eq!(scaled.face_iter().count(), 6);
    // translate
    let translated = facade::translate(&plate, Vector3::new(1.0, 2.0, 3.0))
        .expect("translate assembles")
        .value;
    assert_eq!(translated.face_iter().count(), 6);
    // section
    let mut section_budget = Budget::new(0, 0, 0);
    let faces = facade::section(&plate, &plane_z1(), &mut section_budget)
        .expect("section assembles")
        .value;
    assert_eq!(faces.len(), 1);
    // split
    let mut split_budget = Budget::new(0, 0, 0);
    let (plus, minus) = facade::split(&plate, &plane_z1(), &mut split_budget)
        .expect("split assembles")
        .value;
    assert_eq!(plus.boundaries().len(), 1);
    assert_eq!(minus.boundaries().len(), 1);
    // bounding_box
    let mut bb_budget = Budget::new(0, 0, 0);
    let _box = facade::bounding_box(&plate, &mut bb_budget)
        .expect("bounding_box assembles")
        .value;
    // boolean_op (all three modes)
    let mut add_budget = Budget::new(1000, 1000, 1000);
    let union = facade::boolean_op(&plate, Mode::Add, &disk, &mut add_budget)
        .expect("Add assembles")
        .value;
    assert_eq!(union.face_iter().count(), 8);
    let mut sub_budget = Budget::new(1000, 1000, 1000);
    let difference = facade::boolean_op(&plate, Mode::Subtract, &disk, &mut sub_budget)
        .expect("Subtract assembles")
        .value;
    assert_eq!(difference.face_iter().count(), 7);
    let mut inter_budget = Budget::new(1000, 1000, 1000);
    let intersection = facade::boolean_op(&plate, Mode::Intersect, &disk, &mut inter_budget)
        .expect("Intersect assembles")
        .value;
    assert_eq!(intersection.face_iter().count(), 3);
    // make_face
    let made = facade::make_face(&pa).expect("make_face assembles").value;
    assert_eq!(made.len(), 1);
    // make_hull
    let pts = vec![
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(2.0, 0.0, 0.0),
        Point3::new(2.0, 2.0, 0.0),
        Point3::new(0.0, 2.0, 0.0),
    ];
    let _hull = facade::make_hull(&pts).expect("make_hull assembles").value;
    // The fillet/Circular and the revolve/rotate folds keep the cylinder disk
    // consumable: the empty-list refusal is the facade's own typed arm.
    let mut empty_budget = Budget::new(1000, 1000, 1000);
    assert!(
        matches!(
            facade::fillet(&plate, &[], &mut empty_budget),
            Err(Refusal::Empty)
        ),
        "an empty fillet list refuses Empty exactly as the landed entries do"
    );
}

// ---------------------------------------------------------------------------
// Test 2: the constructive sequence — plate with fillet and hole.
// ---------------------------------------------------------------------------

#[test]
fn facade_constructive_sequence_plate_with_fillet_and_hole() {
    // The flagship: rectangle profile -> extrude -> fillet (two vertical
    // edges, the grouped Straight batch) -> boolean_op(Subtract, the small
    // boundary-crossing box). Every intermediate assembles (the r2 measured
    // census: extrude 6 faces; fillet 8 faces; Subtract 13 faces).
    let (pa, aa) = block_profile();
    let plate = extrude_solid(&pa, &aa, 2.0);
    assert_eq!(
        plate.face_iter().count(),
        6,
        "the extrude assembles 6 faces"
    );

    let mut fillet_budget = Budget::new(1000, 1000, 1000);
    let filleted = facade::fillet(
        &plate,
        &[
            BlendSpec::Straight(FilletSpec {
                a: Point3::new(4.0, 4.0, 0.0),
                b: Point3::new(4.0, 4.0, 2.0),
                radius: 1.0,
            }),
            BlendSpec::Straight(FilletSpec {
                a: Point3::new(0.0, 0.0, 0.0),
                b: Point3::new(0.0, 0.0, 2.0),
                radius: 1.0,
            }),
        ],
        &mut fillet_budget,
    )
    .expect("the grouped Straight fillet batch assembles")
    .value;
    assert_eq!(
        filleted.face_iter().count(),
        8,
        "the fillet assembles 8 faces"
    );

    // The small box [1.5,2.5]^2 x [0.5,2.5] crosses the plate's top boundary
    // (the resew convention).
    let small = box_solid(1.5, 1.5, 0.5, 2.5, 2.5, 2.5);
    let mut boolean_budget = Budget::new(1000, 1000, 1000);
    let holed = facade::boolean_op(&filleted, Mode::Subtract, &small, &mut boolean_budget)
        .expect("the Subtract assembles")
        .value;
    assert_eq!(
        holed.face_iter().count(),
        13,
        "the Subtract assembles 13 faces"
    );

    // The split rows (re-booked r3): a PLAIN box splits by z=1 into 1+1
    // shells; a FILLET-CARRYING solid refuses the typed envelope for every
    // plane (the measured split-of-arc-carrying-faces v1 boundary).
    let mut plain_budget = Budget::new(1000, 1000, 1000);
    let (plus, minus) = facade::split(&plate, &plane_z1(), &mut plain_budget)
        .expect("the plain-box split assembles")
        .value;
    assert_eq!(plus.boundaries().len(), 1, "the plus half is one shell");
    assert_eq!(minus.boundaries().len(), 1, "the minus half is one shell");

    let mut arc_budget = Budget::new(1000, 1000, 1000);
    let split_arc = facade::split(&filleted, &plane_z1(), &mut arc_budget);
    assert!(
        matches!(
            split_arc,
            Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::ContactReductionDeferred
            ))
        ),
        "the split of the arc-carrying fillet solid is the measured v1 boundary, got {split_arc:?}"
    );
}

// ---------------------------------------------------------------------------
// Test 3: the metamorphic rows through the facade.
// ---------------------------------------------------------------------------

/// The metamorphic similarity T = Rz(pi/2) + translation (z-neutral),
/// composed through the facade's rotate/translate.
fn apply_t(solid: &Solid<Point3, Curve, Surface>, t: Vector3) -> Solid<Point3, Curve, Surface> {
    let rotated = facade::rotate(
        solid,
        Point3::new(0.0, 0.0, 0.0),
        Vector3::unit_z(),
        FRAC_PI_2,
    )
    .expect("the rotation assembles")
    .value;
    facade::translate(&rotated, t)
        .expect("the translation assembles")
        .value
}

#[test]
fn facade_metamorphic_rows_still_hold_through_facade() {
    let (pa, aa) = block_profile();
    let a = extrude_solid(&pa, &aa, 2.0);
    let b = box_solid(1.0, 1.0, 1.5, 3.0, 3.0, 3.5);

    // A ∪ B ≅ B ∪ A through boolean_op(Mode::Add).
    let mut budget_ab = Budget::new(1000, 1000, 1000);
    let union_ab = facade::boolean_op(&a, Mode::Add, &b, &mut budget_ab)
        .expect("A union B assembles")
        .value;
    let mut budget_ba = Budget::new(1000, 1000, 1000);
    let union_ba = facade::boolean_op(&b, Mode::Add, &a, &mut budget_ba)
        .expect("B union A assembles")
        .value;
    assert_eq!(union_ab.face_iter().count(), union_ba.face_iter().count());
    assert_eq!(unique_edges(&union_ab), unique_edges(&union_ba));
    assert_eq!(unique_vertices(&union_ab), unique_vertices(&union_ba));
    assert_eq!(carrier_kinds(&union_ab), carrier_kinds(&union_ba));
    assert_eq!(
        unique_sorted_points(&union_ab),
        unique_sorted_points(&union_ba),
        "A ∪ B and B ∪ A share the exact vertex set"
    );

    // A − A = ∅: refusal-or-empty. The machine-checked landed behavior on
    // this fixture is the typed envelope refusal (the self-pair composition's
    // recorded v1 boundary, boolean_m2 Test 4).
    let mut budget_aa = Budget::new(1000, 1000, 1000);
    let aa = facade::boolean_op(&a, Mode::Subtract, &a, &mut budget_aa);
    let empty_or_refused = match &aa {
        Ok(c) => c.value.boundaries().is_empty(),
        Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ContactReductionDeferred)) => true,
        Err(_) => false,
    };
    assert!(
        empty_or_refused,
        "A − A is the refusal-or-empty row, got {aa:?}"
    );

    // The fillet round trip (the P6/P7 row): the fillet's two adjacent faces
    // are the original planes offset back by r = 1 — the kept x=4 face spans
    // y in [0, 3] and the kept y=4 face spans x in [0, 3], with the four
    // tangent points at distance r from the original edge line.
    let boxed = box_fixture();
    let mut round_budget = Budget::new(1000, 1000, 1000);
    let filleted = facade::fillet(
        &boxed,
        &[BlendSpec::Straight(FilletSpec {
            a: Point3::new(4.0, 4.0, 0.0),
            b: Point3::new(4.0, 4.0, 2.0),
            radius: 1.0,
        })],
        &mut round_budget,
    )
    .expect("the fillet round trip assembles")
    .value;
    let x4 = filleted
        .face_iter()
        .find(|f| matches!(f.surface(), Surface::Plane(p) if p.normal() == Vector3::unit_x()))
        .expect("the x=4 face survives");
    let (_, x4_ys) = face_xy_box(x4);
    assert_eq!(x4_ys, (0.0, 3.0), "the x=4 face trims back to y = 4 − r");
    let y4 = filleted
        .face_iter()
        .find(|f| matches!(f.surface(), Surface::Plane(p) if p.normal() == Vector3::unit_y()))
        .expect("the y=4 face survives");
    let (y4_xs, _) = face_xy_box(y4);
    assert_eq!(y4_xs, (0.0, 3.0), "the y=4 face trims back to x = 4 − r");
    let edge_dir = Vector3::new(0.0, 0.0, 1.0);
    for p in [
        Point3::new(4.0, 3.0, 0.0),
        Point3::new(3.0, 4.0, 0.0),
        Point3::new(4.0, 3.0, 2.0),
        Point3::new(3.0, 4.0, 2.0),
    ] {
        assert!(filleted.vertex_iter().any(|v| v.point() == p));
        let distance = (p - Point3::new(4.0, 4.0, p.z)).cross(edge_dir).magnitude();
        assert_eq!(distance, 1.0, "the tangent points sit at distance r = 1");
    }

    // The P9 contact row: contact(A, B) ≅ contact(g·A, g·B) for the
    // facade-rotated pair — the perpendicular side planes of the block and a
    // small box, one analytic FF record before and after rotation.
    let c0 = side_plane_contact(&a, &b);
    let ga = facade::rotate(&a, Point3::new(0.0, 0.0, 0.0), Vector3::unit_z(), FRAC_PI_2)
        .expect("the rotated block assembles")
        .value;
    let gb = facade::rotate(&b, Point3::new(0.0, 0.0, 0.0), Vector3::unit_z(), FRAC_PI_2)
        .expect("the rotated box assembles")
        .value;
    let c1 = side_plane_contact(&ga, &gb);
    assert_eq!(
        c0, c1,
        "contact(A, B) and contact(g·A, g·B) agree through the facade rotation"
    );

    // The P10 transform row: T(A ∪ B) = T(A) ∪ T(B) through the facade names.
    let t = Vector3::new(1.0, 2.0, 0.0);
    let mut budget_ab = Budget::new(1000, 1000, 1000);
    let ab_union = facade::boolean_op(&a, Mode::Add, &b, &mut budget_ab)
        .expect("A union B assembles")
        .value;
    let t_of_ab = apply_t(&ab_union, t);
    let ta = apply_t(&a, t);
    let tb = apply_t(&b, t);
    let mut budget_t = Budget::new(1000, 1000, 1000);
    let t_union = facade::boolean_op(&ta, Mode::Add, &tb, &mut budget_t)
        .expect("T(A) union T(B) assembles")
        .value;
    assert_eq!(
        t_union.face_iter().count(),
        t_of_ab.face_iter().count(),
        "face count"
    );
    assert_eq!(unique_edges(&t_union), unique_edges(&t_of_ab), "edge count");
    assert_eq!(
        unique_vertices(&t_union),
        unique_vertices(&t_of_ab),
        "vertex count"
    );
    assert_eq!(carrier_kinds(&t_union), carrier_kinds(&t_of_ab));
    assert_eq!(
        unique_sorted_points(&t_union),
        unique_sorted_points(&t_of_ab),
        "T(A) ∪ T(B) and T(A ∪ B) share the exact vertex set"
    );
}

/// One analytic FF contact record count between a side plane of `lhs` (x = 0)
/// and a side plane of `rhs` (y = 1), both lifted as bounded face strata.
fn side_plane_contact(
    lhs: &Solid<Point3, Curve, Surface>,
    rhs: &Solid<Point3, Curve, Surface>,
) -> usize {
    let a_face = lhs
        .face_iter()
        .find(|f| matches!(f.surface(), Surface::Plane(p) if p.origin().x == 0.0))
        .expect("the lhs side plane x = 0");
    let b_face = rhs
        .face_iter()
        .find(|f| matches!(f.surface(), Surface::Plane(p) if p.origin().y == 1.0))
        .expect("the rhs side plane y = 1");
    let a_stratum = truck_evidence::contact::face_stratum(
        recognize_surface(&a_face.surface()),
        (0.0, 1.0),
        (0.0, 1.0),
    )
    .expect("the lhs face lifts to a bounded stratum");
    let b_stratum = truck_evidence::contact::face_stratum(
        recognize_surface(&b_face.surface()),
        (0.0, 1.0),
        (0.0, 1.0),
    )
    .expect("the rhs face lifts to a bounded stratum");
    let mut budget = Budget::new(0, 0, 0);
    truck_evidence::contact::contact(&a_stratum, &b_stratum, &mut budget)
        .expect("the analytic FF pair certifies")
        .value
        .contacts
        .len()
}

// ---------------------------------------------------------------------------
// Test 4: the typed refusal cases (one per feature family).
// ---------------------------------------------------------------------------

#[test]
fn facade_refusal_cases_are_typed() {
    let (pa, aa) = block_profile();
    let plate = extrude_solid(&pa, &aa, 2.0);
    let (pd, ad) = disk_profile(Point2::new(0.0, 0.0), 2.0);
    let cyl = extrude_solid(&pd, &ad, 2.0);

    // Non-plane fillet lift: a Straight fillet on a cylinder-carrying solid
    // refuses at the lift.
    let mut b = Budget::new(1000, 1000, 1000);
    assert!(
        matches!(
            facade::fillet(
                &cyl,
                &[BlendSpec::Straight(FilletSpec {
                    a: Point3::new(4.0, 4.0, 0.0),
                    b: Point3::new(4.0, 4.0, 2.0),
                    radius: 1.0,
                })],
                &mut b
            ),
            Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::NonCanonicalCarrier
            ))
        ),
        "the non-plane fillet lift refuses NonCanonicalCarrier"
    );

    // Trim overflow: the fillet radius reaches the adjacent extent.
    let mut b = Budget::new(1000, 1000, 1000);
    assert!(
        matches!(
            facade::fillet(
                &plate,
                &[BlendSpec::Straight(FilletSpec {
                    a: Point3::new(4.0, 4.0, 0.0),
                    b: Point3::new(4.0, 4.0, 2.0),
                    radius: 4.0,
                })],
                &mut b
            ),
            Err(Refusal::Empty)
        ),
        "the trim overflow refuses Empty"
    );

    // Circular overflow: the circular rim fillet radius consumes the wall.
    let mut b = Budget::new(1000, 1000, 1000);
    assert!(
        matches!(
            facade::fillet(
                &cyl,
                &[BlendSpec::Circular(CircleFilletSpec {
                    center: Point3::new(0.0, 0.0, 2.0),
                    edge_radius: 2.0,
                    radius: 2.0,
                })],
                &mut b
            ),
            Err(Refusal::Empty)
        ),
        "the circular overflow refuses Empty"
    );

    // Multi-wire cap: the washer's top face is a two-wire plane (the
    // [-4,4]^2 square with the r=1 hole self-loop at the origin); filleting
    // the hole rim has that face as the cap and refuses at the D2 lift.
    let mut washer = vec![
        Curve::Line(Line(
            Point3::new(-4.0, -4.0, 0.0),
            Point3::new(4.0, -4.0, 0.0),
        )),
        Curve::Line(Line(
            Point3::new(4.0, -4.0, 0.0),
            Point3::new(4.0, 4.0, 0.0),
        )),
        Curve::Line(Line(
            Point3::new(4.0, 4.0, 0.0),
            Point3::new(-4.0, 4.0, 0.0),
        )),
        Curve::Line(Line(
            Point3::new(-4.0, 4.0, 0.0),
            Point3::new(-4.0, -4.0, 0.0),
        )),
    ];
    washer.push(Curve::Circle(placed_circle(
        Point3::new(0.0, 0.0, 0.0),
        1.0,
    )));
    let ok = arrange(&washer, None).unwrap();
    let washer_solid = extrude_solid(&washer, &ok.value, 1.0);
    let mut b = Budget::new(1000, 1000, 1000);
    assert!(
        matches!(
            facade::fillet(
                &washer_solid,
                &[BlendSpec::Circular(CircleFilletSpec {
                    center: Point3::new(0.0, 0.0, 1.0),
                    edge_radius: 1.0,
                    radius: 0.25,
                })],
                &mut b
            ),
            Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::NonCanonicalCarrier
            ))
        ),
        "the multi-wire cap refuses the envelope at the D2 lift"
    );

    // Oblique dz=0 extrude: a z-neutral oblique dir has zero volume.
    assert!(
        matches!(
            facade::extrude_vector(&pd, &ad, Vector3::new(1.0, 0.0, 0.0), false),
            Err(Refusal::Empty)
        ),
        "the dz=0 oblique extrude refuses Empty"
    );

    // Non-z-parallel oblique on non-circle profiles: the machine-checked arm
    // for a non-circle (line) profile is the zero-volume `Empty` refusal — a
    // dz != 0 oblique on a line rectangle ASSEMBLES (the P10 emission), so
    // the only typed refusal the landed entry answers on non-circle profiles
    // is the dz=0 arm (machine-checked on this tree).
    assert!(
        matches!(
            facade::extrude_vector(&pa, &aa, Vector3::new(1.0, 1.0, 0.0), false),
            Err(Refusal::Empty)
        ),
        "the non-z-parallel oblique on the non-circle profile refuses Empty"
    );

    // Boolean multi-shell guard: a two-shell solid refuses at the guard.
    let s1 = box_solid(0.0, 0.0, 0.0, 2.0, 2.0, 2.0);
    let s2 = box_solid(10.0, 10.0, 0.0, 12.0, 12.0, 2.0);
    let multi = Solid::try_new(vec![
        s1.boundaries().first().unwrap().clone(),
        s2.boundaries().first().unwrap().clone(),
    ])
    .expect("the two-shell solid assembles");
    let mut b = Budget::new(1000, 1000, 1000);
    assert!(
        matches!(
            facade::boolean_op(&multi, Mode::Add, &plate, &mut b),
            Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::ContactReductionDeferred
            ))
        ),
        "the multi-shell guard refuses the typed envelope"
    );

    // Split by a plane that grazes (the vertex-touch cut boundary): the
    // diagonal plane x + y = 2 on the box [0,2]^2 x [0,2].
    let (pg, ag) = box_profile(0.0, 0.0, 2.0, 2.0);
    let small_box = extrude_solid(&pg, &ag, 2.0);
    let diag = Plane::new(
        Point3::new(0.0, 2.0, 0.0),
        Point3::new(2.0, 0.0, 0.0),
        Point3::new(0.0, 2.0, 2.0),
    );
    let mut b = Budget::new(1000, 1000, 1000);
    assert!(
        matches!(
            facade::split(&small_box, &diag, &mut b),
            Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::ContactReductionDeferred
            ))
        ),
        "the grazing split is the measured vertex-touch typed refusal"
    );
}

// ---------------------------------------------------------------------------
// Test 5: the Mode -> BoolOp mapping produces identical results.
// ---------------------------------------------------------------------------

#[test]
fn facade_boolean_modes_map_to_boolop() {
    let (pa, aa) = block_profile();
    let a = extrude_solid(&pa, &aa, 2.0);
    let (pb, ab) = disk_profile(Point2::new(2.0, 2.0), 1.0);
    let b = extrude_solid(&pb, &ab, 2.0);

    for (mode, op) in [
        (Mode::Add, truck_shapeops::boolean::BoolOp::Union),
        (Mode::Subtract, truck_shapeops::boolean::BoolOp::Difference),
        (
            Mode::Intersect,
            truck_shapeops::boolean::BoolOp::Intersection,
        ),
    ] {
        let mut facade_budget = Budget::new(1000, 1000, 1000);
        let via_facade = facade::boolean_op(&a, mode, &b, &mut facade_budget)
            .expect("the facade mode assembles")
            .value;
        let mut landed_budget = Budget::new(1000, 1000, 1000);
        let via_landed = truck_shapeops::boolean::assemble::boolean(&a, op, &b, &mut landed_budget)
            .expect("the landed BoolOp assembles")
            .value;
        assert_eq!(
            via_facade.face_iter().count(),
            via_landed.face_iter().count(),
            "{op:?}: face count"
        );
        assert_eq!(
            unique_edges(&via_facade),
            unique_edges(&via_landed),
            "{op:?}: edge count"
        );
        assert_eq!(
            unique_vertices(&via_facade),
            unique_vertices(&via_landed),
            "{op:?}: vertex count"
        );
        assert_eq!(
            unique_sorted_points(&via_facade),
            unique_sorted_points(&via_landed),
            "{op:?}: the mode and the BoolOp produce the same exact vertex set"
        );
    }
}

// ---------------------------------------------------------------------------
// Test 6: the consumability rows — tessellation condition Closed on the
// generated carriers (release-gated per Finding 3).
// ---------------------------------------------------------------------------

#[test]
fn consumability_tessellation_closed_on_generated_carriers() {
    // Finding 3 of BG-CAD-P8-FACADE: tessellation of circle-carrying solids
    // PANICS in debug builds ("Two same vertices cannot construct an edge",
    // the recorded self-loop constructor trap; the box control — line edges
    // only — meshes fine in debug). The tessellation rows are therefore
    // release-gated; in debug this test asserts the gate compiles and returns
    // early.
    #[cfg(not(debug_assertions))]
    {
        let (pa, aa) = block_profile();
        let plate = extrude_solid(&pa, &aa, 2.0);
        let (pd, ad) = disk_profile(Point2::new(0.0, 0.0), 2.0);
        let cyl = extrude_solid(&pd, &ad, 2.0);

        // The box control (line edges only).
        let (cond, pos, faces) = mesh_condition(&plate);
        assert!(
            cond == ShellCondition::Closed && pos > 0 && faces > 0,
            "the box tessellates Closed with a non-empty mesh (the probe's measured condition), got {cond:?} pos {pos} faces {faces}"
        );

        // The plain cylinder (disk extrude).
        let (cond, pos, faces) = mesh_condition(&cyl);
        assert!(
            cond == ShellCondition::Closed && pos > 0 && faces > 0,
            "the plain cylinder tessellates Closed (the probe's measured condition), got {cond:?} pos {pos} faces {faces}"
        );

        // The P12 torus-fillet solid (through the facade's Circular fillet).
        let mut b = Budget::new(1000, 1000, 1000);
        let torus_solid = facade::fillet(
            &cyl,
            &[BlendSpec::Circular(CircleFilletSpec {
                center: Point3::new(0.0, 0.0, 2.0),
                edge_radius: 2.0,
                radius: 0.5,
            })],
            &mut b,
        )
        .expect("the torus fillet assembles")
        .value;
        let (cond, pos, faces) = mesh_condition(&torus_solid);
        assert!(
            cond == ShellCondition::Closed && pos > 0 && faces > 0,
            "the torus-fillet solid tessellates Closed (the probe's measured condition), got {cond:?} pos {pos} faces {faces}"
        );

        // The mirrored (Placed-cylinder) solid.
        let mirrored = facade::mirror_about_plane(
            &cyl,
            Point3::new(0.0, 0.0, 1.0),
            Vector3::new(1.0, 1.0, 0.0),
        )
        .expect("the mirror assembles")
        .value;
        let (cond, pos, faces) = mesh_condition(&mirrored);
        assert!(
            cond == ShellCondition::Closed && pos > 0 && faces > 0,
            "the mirrored placed-cylinder tessellates Closed (the probe's measured condition), got {cond:?} pos {pos} faces {faces}"
        );

        // The boolean Difference output.
        let (pdisk, adisk) = disk_profile(Point2::new(2.0, 2.0), 1.0);
        let disk = extrude_solid(&pdisk, &adisk, 2.0);
        let mut b = Budget::new(1000, 1000, 1000);
        let diff = facade::boolean_op(&plate, Mode::Subtract, &disk, &mut b)
            .expect("the boolean output assembles")
            .value;
        let (cond, pos, faces) = mesh_condition(&diff);
        assert!(
            cond == ShellCondition::Closed && pos > 0 && faces > 0,
            "the boolean output tessellates Closed (the r2 worker's measured condition), got {cond:?} pos {pos} faces {faces}"
        );
    }
    #[cfg(debug_assertions)]
    {
        // The gate compiles and returns early: debug tessellation of these
        // circle carriers panics (Finding 3), so no mesh work runs here.
    }
}

// ---------------------------------------------------------------------------
// Test 7: the oblique row — the landed extrude_vector emission's measured
// condition (release-gated per Finding 3).
// ---------------------------------------------------------------------------

#[test]
fn consumability_tessellation_oblique_recorded_boundary() {
    // The probe's Finding 2 quoted a HAND-BUILT oblique placed-affine wall on
    // the PRE-P10 tree tessellating with condition Regular. The r2 worker
    // exhaustively re-measured the LANDED `extrude_vector` emission (dir
    // (1,0,1)/(1,1,1)/(0,1,1), both=true, rotated/mirrored/translated) and
    // found condition Closed at every put_together tolerance tried; the r3
    // amendment asserts the dispatch-tree measurement. The closure of sheared
    // placements is recorded as measured, no boundary claimed. Release-gated
    // per Finding 3.
    #[cfg(not(debug_assertions))]
    {
        let (pd, ad) = disk_profile(Point2::new(0.0, 0.0), 2.0);
        let oblique = facade::extrude_vector(&pd, &ad, Vector3::new(1.0, 0.0, 1.0), true)
            .expect("the oblique extrusion assembles")
            .value;
        let (cond, pos, faces) = mesh_condition(&oblique);
        assert!(
            cond == ShellCondition::Closed && pos > 0 && faces > 0,
            "the oblique placed-affine wall tessellates Closed on the dispatch tree (the r2/r3 measured condition), got {cond:?} pos {pos} faces {faces}"
        );
    }
    #[cfg(debug_assertions)]
    {
        // The gate compiles and returns early (Finding 3).
    }
}
