//! BG-CAD-P3-SPLIT — section + split by plane via the landed Boolean:
//! dyadic witnesses only (the test-1..10 battery of PACKET.md).
//!
//! The parsimony identity `split(S, Pi) = Contact + classify + caps +
//! rewrite` is the landed 3-D Boolean, so every assertion below is a
//! statement about the composed `boolean()` calls in
//! `truck_shapeops::section`, never about new cutting machinery.

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

use std::f64::consts::TAU;
use truck_base::bounding_box::BoundingBox;
use truck_base::cgmath64::{Matrix4, Point2, Point3, Vector3, Vector4};
use truck_base::evidence::{Budget, EnvelopeCase, Outcome, Refusal, UnresolvedWitness};
use truck_geometry::arrange::{arrange, Arrangement};
use truck_geometry::canonical::{Curve, Surface};
use truck_geometry::prelude::*;
use truck_modeling::extrude::extrude_profile;
use truck_shapeops::boolean::assemble::boolean;
use truck_shapeops::boolean::BoolOp;
use truck_shapeops::section::{section_faces, split_by_plane};
use truck_topology::{Face, Shell, Solid};

// ---------------------------------------------------------------------------
// construction helpers
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

/// The flagship plate-with-hole profile: the 4x4 rectangle plus a full circle
/// r = 1 at (2, 2).
fn plate_with_hole_profile() -> (Vec<Curve>, Arrangement) {
    let mut profile = vec![
        Curve::Line(Line(Point3::new(0.0, 0.0, 0.0), Point3::new(4.0, 0.0, 0.0))),
        Curve::Line(Line(Point3::new(4.0, 0.0, 0.0), Point3::new(4.0, 4.0, 0.0))),
        Curve::Line(Line(Point3::new(4.0, 4.0, 0.0), Point3::new(0.0, 4.0, 0.0))),
        Curve::Line(Line(Point3::new(0.0, 4.0, 0.0), Point3::new(0.0, 0.0, 0.0))),
    ];
    let circle = Curve::Circle(placed_circle(Point3::new(2.0, 2.0, 0.0), 1.0));
    profile.push(circle);
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

/// The match-based `Ok` unwrapper of the packet (D1).
fn expect_ok<T>(r: Outcome<T>) -> T {
    match r {
        Ok(c) => c.value,
        Err(e) => panic!("unexpected refusal: {e:?}"),
    }
}

// ---------------------------------------------------------------------------
// measurement helpers
// ---------------------------------------------------------------------------

/// The axis-aligned bounding box of a solid's vertices.
fn solid_box(solid: &Solid<Point3, Curve, Surface>) -> ((f64, f64), (f64, f64), (f64, f64)) {
    let mut lo = Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
    let mut hi = Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for v in solid.vertex_iter() {
        let p = v.point();
        lo.x = lo.x.min(p.x);
        lo.y = lo.y.min(p.y);
        lo.z = lo.z.min(p.z);
        hi.x = hi.x.max(p.x);
        hi.y = hi.y.max(p.y);
        hi.z = hi.z.max(p.z);
    }
    ((lo.x, hi.x), (lo.y, hi.y), (lo.z, hi.z))
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

// ---------------------------------------------------------------------------
// Test 1: the extruded 4x4x2 plate cut at z = 1 (norm +z).
// ---------------------------------------------------------------------------

#[test]
fn split_flagship_plate_through_middle() {
    let (profile, arr) = block_profile();
    let plate = extrude_solid(&profile, &arr, 2.0);
    let plane = Plane::new(
        Point3::new(0.0, 0.0, 1.0),
        Point3::new(1.0, 0.0, 1.0),
        Point3::new(0.0, 1.0, 1.0),
    );
    let mut budget = Budget::new(0, 0, 0);
    let (plus, minus) = expect_ok(split_by_plane(&plate, &plane, &mut budget));
    assert_eq!(plus.boundaries().len(), 1);
    assert_eq!(minus.boundaries().len(), 1);
    assert_eq!(solid_box(&plus), ((0.0, 4.0), (0.0, 4.0), (1.0, 2.0)));
    assert_eq!(solid_box(&minus), ((0.0, 4.0), (0.0, 4.0), (0.0, 1.0)));
}

// ---------------------------------------------------------------------------
// Test 2: split+ ∪ split- ≅ S (the booked metamorphic recombination).
// ---------------------------------------------------------------------------

#[test]
fn split_recombination_is_original() {
    let (profile, arr) = block_profile();
    let plate = extrude_solid(&profile, &arr, 2.0);
    let plane = Plane::new(
        Point3::new(0.0, 0.0, 1.0),
        Point3::new(1.0, 0.0, 1.0),
        Point3::new(0.0, 1.0, 1.0),
    );
    let mut budget = Budget::new(0, 0, 0);
    let (plus, minus) = expect_ok(split_by_plane(&plate, &plane, &mut budget));
    let mut union_budget = Budget::new(0, 0, 0);
    let union = expect_ok(boolean(&plus, BoolOp::Union, &minus, &mut union_budget));
    assert_eq!(union.boundaries().len(), 1);
    // The re-sewn union keeps the pre-decided 10-face / box-equal assertion
    // per the RW-RESEW evidence (amendment r2): the wall caps are the
    // butt-join pair the landed material-state machinery discards, leaving
    // the 8 split side faces plus the two caps.
    assert_eq!(union.boundaries().first().unwrap().face_iter().count(), 10);
    assert_eq!(solid_box(&union), solid_box(&plate));
}

// ---------------------------------------------------------------------------
// Test 3: the booked vertex-touch cut boundary (deferred list, session 41).
// ---------------------------------------------------------------------------

#[test]
fn split_box_diagonal_plane() {
    let (profile, arr) = box_profile(0.0, 0.0, 2.0, 2.0);
    let box_solid = extrude_solid(&profile, &arr, 2.0);
    // The plane x + y = 2 (norm (1, 1, 0)/sqrt(2)) through opposite edges,
    // built from three exact dyadic points. Planes are compared by data, not
    // unit length, so the unit-normal form is irrelevant here.
    let plane = Plane::new(
        Point3::new(0.0, 2.0, 0.0),
        Point3::new(2.0, 0.0, 0.0),
        Point3::new(0.0, 2.0, 2.0),
    );
    let mut budget = Budget::new(0, 0, 0);
    let out = split_by_plane(&box_solid, &plane, &mut budget);
    // VERTEX-TOUCH CUT BOUNDARY (deferred list, session 41): a cut through
    // the solid's edge graph requires four kernel decisions (canonical-vertex
    // splicing, seam-edge replacement, per-face arc certification, Region2
    // coplanar-adjacent) booked as the follow-up family. The recorded class
    // across three instrumented stops is ContactReductionDeferred /
    // NumericallyUnresolved(UncertifiedContainment) depending on how far the
    // chain runs; assert whichever arm the landed pipeline answers.
    assert!(
        matches!(
            out,
            Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::ContactReductionDeferred
            )) | Err(Refusal::NumericallyUnresolved {
                witness: UnresolvedWitness::UncertifiedContainment,
                ..
            })
        ),
        "the diagonal split must refuse at the vertex-touch boundary, got {out:?}"
    );
}

// ---------------------------------------------------------------------------
// Test 4: the plate section at z = 1.
// ---------------------------------------------------------------------------

#[test]
fn section_faces_of_plate() {
    let (profile, arr) = block_profile();
    let plate = extrude_solid(&profile, &arr, 2.0);
    let plane = Plane::new(
        Point3::new(0.0, 0.0, 1.0),
        Point3::new(1.0, 0.0, 1.0),
        Point3::new(0.0, 1.0, 1.0),
    );
    let mut budget = Budget::new(0, 0, 0);
    let faces = expect_ok(section_faces(&plate, &plane, &mut budget));
    assert_eq!(faces.len(), 1);
    let face = faces.first().unwrap();
    assert!(
        matches!(face.surface(), Surface::Plane(p) if p == plane),
        "the section face carries the wall plane data exactly"
    );
    assert_eq!(face_xy_box(face), ((0.0, 4.0), (0.0, 4.0)));
}

// ---------------------------------------------------------------------------
// Test 5: the plate-with-hole section at z = 1 is the annulus (2 wires).
// ---------------------------------------------------------------------------

#[test]
fn section_face_with_hole_annulus() {
    let (profile, arr) = plate_with_hole_profile();
    let plate = extrude_solid(&profile, &arr, 2.0);
    let plane = Plane::new(
        Point3::new(0.0, 0.0, 1.0),
        Point3::new(1.0, 0.0, 1.0),
        Point3::new(0.0, 1.0, 1.0),
    );
    let mut budget = Budget::new(0, 0, 0);
    let faces = expect_ok(section_faces(&plate, &plane, &mut budget));
    assert_eq!(faces.len(), 1);
    let face = faces.first().unwrap();
    assert!(
        matches!(face.surface(), Surface::Plane(p) if p == plane),
        "the section face carries the wall plane data exactly"
    );
    assert_eq!(
        face.absolute_boundaries().len(),
        2,
        "the annulus has two wires"
    );
}

// ---------------------------------------------------------------------------
// Test 6: a missed plane is the normal result: plus ≅ S, minus empty, and
// section refuses Empty.
// ---------------------------------------------------------------------------

#[test]
fn plane_missing_returns_whole_plus_empty() {
    let (profile, arr) = block_profile();
    let plate = extrude_solid(&profile, &arr, 2.0);
    let plate_faces = plate.face_iter().count();
    let plane = Plane::new(
        Point3::new(0.0, 0.0, -1.0),
        Point3::new(1.0, 0.0, -1.0),
        Point3::new(0.0, 1.0, -1.0),
    );
    let mut budget = Budget::new(0, 0, 0);
    let (plus, minus) = expect_ok(split_by_plane(&plate, &plane, &mut budget));
    assert_eq!(plus.face_iter().count(), plate_faces);
    assert_eq!(solid_box(&plus), solid_box(&plate));
    assert!(
        minus.boundaries().is_empty(),
        "the missed side is the empty solid"
    );

    let mut section_budget = Budget::new(0, 0, 0);
    assert!(
        matches!(
            section_faces(&plate, &plane, &mut section_budget),
            Err(Refusal::Empty)
        ),
        "a missed plane yields no section"
    );
}

// ---------------------------------------------------------------------------
// Test 7: the oblique-plane x cylinder-wall section is the booked RW-CONIC
// boundary: assert whichever typed refusal the landed pipeline answers.
// ---------------------------------------------------------------------------

#[test]
fn oblique_cylinder_section_refuses() {
    let (profile, arr) = disk_profile(Point2::new(0.0, 0.0), 1.0);
    let cylinder = extrude_solid(&profile, &arr, 2.0);
    // The oblique plane through (0, 0, 1) with u-axis (1, 0, 0) and v-axis
    // (0, 1, 1): normal (0, -1, 1)/sqrt(2), cutting the z-aligned cylinder
    // wall in an ellipse locus.
    let plane = Plane::new(
        Point3::new(0.0, 0.0, 1.0),
        Point3::new(1.0, 0.0, 1.0),
        Point3::new(0.0, 1.0, 2.0),
    );
    let mut budget = Budget::new(0, 0, 0);
    let out = section_faces(&cylinder, &plane, &mut budget);
    // Machine-checked arm (this run): the RW-CONIC boundary surfaces as
    // NumericallyUnresolved(UncertifiedContainment) - the splitter's failed
    // insertion projection on the ellipse locus. Whatever the landed pipeline
    // answers is the assertion (PACKET.md D5: do not pre-screen, do not
    // catch).
    assert!(
        matches!(
            out,
            Err(Refusal::NumericallyUnresolved {
                witness: UnresolvedWitness::UncertifiedContainment,
                ..
            })
        ),
        "the oblique cylinder section is the RW-CONIC boundary, got {out:?}"
    );
}

// ---------------------------------------------------------------------------
// Test 8: a non-canonical (placed) sphere face refuses at the over-box lift
// with NO boolean call spent.
// ---------------------------------------------------------------------------

#[test]
fn sphere_face_refuses_noncanonical() {
    let sphere = Surface::Processor(Processor::with_transform(
        Box::new(Surface::Sphere(Sphere::new(
            Point3::new(0.0, 0.0, 0.0),
            1.0,
        ))),
        Matrix4::from_translation(Vector3::new(0.0, 0.0, 1.0)),
    ));
    let face = Face::try_new(Vec::new(), sphere).unwrap();
    let solid = Solid::try_new(vec![Shell::from(vec![face])]).unwrap();
    let plane = Plane::new(
        Point3::new(0.0, 0.0, 1.0),
        Point3::new(1.0, 0.0, 1.0),
        Point3::new(0.0, 1.0, 1.0),
    );
    let mut budget = Budget::new(0, 0, 0);
    let out = split_by_plane(&solid, &plane, &mut budget);
    assert!(
        matches!(
            out,
            Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::NonCanonicalCarrier
            ))
        ),
        "the placed sphere face must refuse at the over-box lift, got {out:?}"
    );
    assert_eq!(budget, Budget::new(0, 0, 0), "no boolean call was spent");
}

// ---------------------------------------------------------------------------
// Test 9: reversing the plane's normal swaps the two halves.
// ---------------------------------------------------------------------------

#[test]
fn split_signs_follow_normal() {
    let (profile, arr) = block_profile();
    let plate = extrude_solid(&profile, &arr, 2.0);
    // The same plane z = 1 with normal -z (u/v axes swapped).
    let plane = Plane::new(
        Point3::new(0.0, 0.0, 1.0),
        Point3::new(0.0, 1.0, 1.0),
        Point3::new(1.0, 0.0, 1.0),
    );
    let mut budget = Budget::new(0, 0, 0);
    let (plus, minus) = expect_ok(split_by_plane(&plate, &plane, &mut budget));
    assert_eq!(solid_box(&plus), ((0.0, 4.0), (0.0, 4.0), (0.0, 1.0)));
    assert_eq!(solid_box(&minus), ((0.0, 4.0), (0.0, 4.0), (1.0, 2.0)));
}

// ---------------------------------------------------------------------------
// Test 10: both halves are downstream-consumable by further booleans.
// ---------------------------------------------------------------------------

#[test]
fn halves_survive_further_boolean() {
    let (profile, arr) = block_profile();
    let plate = extrude_solid(&profile, &arr, 2.0);
    let plane = Plane::new(
        Point3::new(0.0, 0.0, 1.0),
        Point3::new(1.0, 0.0, 1.0),
        Point3::new(0.0, 1.0, 1.0),
    );
    let mut budget = Budget::new(0, 0, 0);
    let (plus, minus) = expect_ok(split_by_plane(&plate, &plane, &mut budget));
    // A small box that crosses both halves transversally: its faces sit at
    // dyadic coordinates off the plate's face planes (no coplanar partial
    // overlap), so the Difference is a real cut, not a no-op.
    let small = truck_modeling::primitive::cuboid::<Curve, Surface>(BoundingBox::from_iter([
        Point3::new(1.5, 1.5, 0.5),
        Point3::new(2.5, 2.5, 2.5),
    ]));

    let mut plus_budget = Budget::new(0, 0, 0);
    let cut_plus = expect_ok(boolean(&plus, BoolOp::Difference, &small, &mut plus_budget));
    assert_eq!(cut_plus.boundaries().len(), 1);

    let mut minus_budget = Budget::new(0, 0, 0);
    let cut_minus = expect_ok(boolean(
        &minus,
        BoolOp::Difference,
        &small,
        &mut minus_budget,
    ));
    assert_eq!(cut_minus.boundaries().len(), 1);
}
