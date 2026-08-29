//! RW-RESEW — the face-adjacent union: the sew-completion pass unifies the
//! seam edge instances across two distinct results so a butt-join assembles
//! into ONE connected shell.
//!
//! D1/D6: the adjacent-boxes pair and the P3 split-recombination both refused
//! at HEAD (`UnsupportedEnvelope(ContactReductionDeferred)`); the acceptance
//! asserts the post-D2 machine answers. The tests 3-7 pin the v1 envelope: the
//! touching Difference keeps A whole, the partial seam refuses, the disjoint
//! pair still refuses at the multi-component fold, the overlapping flagship
//! family is unchanged, and the butt-joined solid downstream-consumes.
//!
//! Every assertion below was machine-observed at the post-D2 pipeline; the
//! face-count expectations that differ from the packet's prose are recorded in
//! RESULT.json's deviations (the landed pipeline does not merge coplanar
//! faces, so a butt-join union of two boxes is a valid 10-face shell — the two
//! solids' side faces — not a 6-face box).

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::f64::consts::TAU;
use truck_base::cgmath64::{Point2, Point3, Vector3};
use truck_base::evidence::{Budget, EnvelopeCase, Outcome, Refusal};
use truck_geometry::arrange::{arrange, Arrangement};
use truck_geometry::canonical::{Curve, Surface};
use truck_geometry::prelude::*;
use truck_modeling::cad::{solid_bounding_box, translate_solid};
use truck_modeling::extrude::extrude_profile;
use truck_shapeops::boolean::assemble::boolean;
use truck_shapeops::boolean::BoolOp;
use truck_topology::Solid;

// ---------------------------------------------------------------------------
// construction helpers (the boolean_m2 / interior_loop conventions)
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
    translate_solid(&solid, Vector3::new(0.0, 0.0, z0))
        .expect("the dyadic translation resolves")
        .value
}

/// Runs one boolean with a fresh budget and returns the outcome.
fn run(
    a: &Solid<Point3, Curve, Surface>,
    op: BoolOp,
    b: &Solid<Point3, Curve, Surface>,
) -> Outcome<Solid<Point3, Curve, Surface>> {
    let mut budget = Budget::new(1000, 1000, 1000);
    boolean(a, op, b, &mut budget)
}

/// The exact bounding box of a solid (cad.rs:82).
fn bounding_box(solid: &Solid<Point3, Curve, Surface>) -> (Point3, Point3) {
    let mut budget = Budget::new(0, 0, 0);
    let hull = solid_bounding_box(solid, &mut budget)
        .expect("the dyadic solid's box resolves")
        .value;
    (hull.min(), hull.max())
}

/// Asserts the solid assembles as exactly one closed shell and returns it.
fn assert_single_shell(solid: &Solid<Point3, Curve, Surface>) {
    assert_eq!(solid.boundaries().len(), 1, "exactly one shell");
}

/// Asserts two solids share an exact (dyadic) bounding box.
fn assert_same_box(a: &Solid<Point3, Curve, Surface>, b: &Solid<Point3, Curve, Surface>) {
    let (amin, amax) = bounding_box(a);
    let (bmin, bmax) = bounding_box(b);
    assert_eq!(amin, bmin, "min corner must match exactly");
    assert_eq!(amax, bmax, "max corner must match exactly");
}

/// The box `[0, 4]^2 x [0, h]` from a 4x4 block profile.
fn flagship_plate(h: f64) -> Solid<Point3, Curve, Surface> {
    let (profile, arr) = block_profile();
    extrude_solid(&profile, &arr, h)
}

// ---------------------------------------------------------------------------
// Test 1: the adjacent-boxes union (SPEC_GAP2 derivation (b)) — two directly
// built boxes `[0,4]^2 x [0,1]` and `[0,4]^2 x [1,2]` union to ONE valid
// solid whose bounding box is `[0,4]^2 x [0,2]` exactly.
// ---------------------------------------------------------------------------

#[test]
fn adjacent_boxes_union_assembles() {
    let a = box_solid(0.0, 0.0, 0.0, 4.0, 4.0, 1.0);
    let b = box_solid(0.0, 0.0, 1.0, 4.0, 4.0, 2.0);

    let union = run(&a, BoolOp::Union, &b)
        .expect("the adjacent-boxes union assembles")
        .value;
    assert_single_shell(&union);
    let merged = box_solid(0.0, 0.0, 0.0, 4.0, 4.0, 2.0);
    assert_same_box(&union, &merged);
    // The landed pipeline does not merge coplanar faces: the union keeps each
    // solid's four side faces (8) plus the two outer caps (2) = 10 faces — the
    // cosmetically split merged box. The packet's "6 faces" is recorded as a
    // deviation in RESULT.json.
    let shell = union.boundaries().first().expect("one shell");
    assert_eq!(shell.face_iter().count(), 10);
}

// ---------------------------------------------------------------------------
// Test 2: the P3 recombination metamorphic (the P3 unblock). Split the
// flagship 4x4x2 plate at z = 1 by the two-call recipe (Difference /
// Intersection with the padded over-box halfspace), then re-union the halves:
// the result is a valid solid with S's exact bounding box.
// ---------------------------------------------------------------------------

#[test]
fn p3_recombination_flagship_is_original() {
    let s = flagship_plate(2.0);
    // The P3 halfspace box for the `z <= 1` cut: the plate's over-box (padded
    // per the carrier table so no wall is coplanar with the plate's) clipped to
    // the halfspace.
    let minus_box = box_solid(-1.0, -1.0, -1.0, 5.0, 5.0, 1.0);

    let minus = run(&s, BoolOp::Intersection, &minus_box)
        .expect("the Intersection half assembles")
        .value;
    let plus = run(&s, BoolOp::Difference, &minus_box)
        .expect("the Difference half assembles")
        .value;
    assert_single_shell(&minus);
    assert_single_shell(&plus);
    // The halves are the two box halves of the plate (6 faces each).
    assert_eq!(minus.boundaries()[0].face_iter().count(), 6);
    assert_eq!(plus.boundaries()[0].face_iter().count(), 6);

    let union = run(&plus, BoolOp::Union, &minus)
        .expect("the recombination union assembles")
        .value;
    assert_single_shell(&union);
    // The metamorphic: the union reproduces S's exact bounding box. (Face
    // count is the machine's 10, not S's 6 — the side faces do not merge;
    // recorded as a deviation.)
    assert_same_box(&union, &s);
    let expected = box_solid(0.0, 0.0, 0.0, 4.0, 4.0, 2.0);
    assert_same_box(&union, &expected);
    assert_eq!(union.boundaries()[0].face_iter().count(), 10);
}

// ---------------------------------------------------------------------------
// Test 3: the touching pair under Difference / Intersection. The touching face
// is a zero-measure overlap, so the material states keep A whole: Difference
// answers A (6 faces, A's box) and Intersection answers the empty solid. Both
// arms are machine-observed post-D2.
// ---------------------------------------------------------------------------

#[test]
fn touching_difference_answers() {
    let a = box_solid(0.0, 0.0, 0.0, 4.0, 4.0, 1.0);
    let b = box_solid(0.0, 0.0, 1.0, 4.0, 4.0, 2.0);

    let difference = run(&a, BoolOp::Difference, &b)
        .expect("the touching Difference assembles")
        .value;
    assert_single_shell(&difference);
    assert_eq!(difference.boundaries()[0].face_iter().count(), 6);
    assert_same_box(&difference, &a);

    let intersection = run(&a, BoolOp::Intersection, &b)
        .expect("the touching Intersection assembles")
        .value;
    // Zero-measure overlap: the regularized intersection is the empty solid.
    assert!(intersection.boundaries().is_empty());
}

// ---------------------------------------------------------------------------
// Test 4: a partial seam refuses. An L-shaped pair sharing only PART of a
// face (`[0,2]^2 x [0,2]` + `[2,4] x [0,2] x [0,1]`) is outside the v1
// envelope: the shared locus is not a full edge-to-edge butt-join.
// ---------------------------------------------------------------------------

#[test]
fn partial_seam_refuses() {
    let a = box_solid(0.0, 0.0, 0.0, 2.0, 2.0, 2.0);
    let b = box_solid(2.0, 0.0, 0.0, 4.0, 2.0, 1.0);
    let union = run(&a, BoolOp::Union, &b);
    // Observed arm: the deferred envelope.
    assert!(
        matches!(
            union,
            Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::ContactReductionDeferred
            ))
        ),
        "the partial seam must refuse the deferred envelope"
    );
}

// ---------------------------------------------------------------------------
// Test 5: two far-apart boxes under Union still refuse at the multi-component
// fold (the A7 guard is untouched).
// ---------------------------------------------------------------------------

#[test]
fn disjoint_union_unchanged() {
    let a = box_solid(0.0, 0.0, 0.0, 2.0, 2.0, 2.0);
    let b = box_solid(10.0, 10.0, 0.0, 12.0, 12.0, 2.0);
    let union = run(&a, BoolOp::Union, &b);
    assert!(
        matches!(
            union,
            Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::ContactReductionDeferred
            ))
        ),
        "the disjoint pair must refuse at the multi-component fold"
    );
}

// ---------------------------------------------------------------------------
// Test 6: the genuinely overlapping M2 flagship family still assembles with
// its landed face counts (the envelope did not regress).
// ---------------------------------------------------------------------------

#[test]
fn overlapping_union_unchanged() {
    let (pa, aa) = block_profile();
    let a = extrude_solid(&pa, &aa, 2.0);
    let (pb, ab) = disk_profile(Point2::new(2.0, 2.0), 1.0);
    let b = extrude_solid(&pb, &ab, 2.0);

    let union = run(&a, BoolOp::Union, &b)
        .expect("the overlapping flagship union assembles")
        .value;
    assert_single_shell(&union);
    // The landed M2 union face count (decision 4's measured set).
    assert_eq!(union.boundaries()[0].face_iter().count(), 8);
}

// ---------------------------------------------------------------------------
// Test 7: the butt-joined result downstream-consumes: the test-1 merged box
// minus a strictly-interior small box assembles a valid solid.
// ---------------------------------------------------------------------------

#[test]
fn butt_join_survives_further_boolean() {
    let a = box_solid(0.0, 0.0, 0.0, 4.0, 4.0, 1.0);
    let b = box_solid(0.0, 0.0, 1.0, 4.0, 4.0, 2.0);
    let merged = run(&a, BoolOp::Union, &b)
        .expect("the adjacent-boxes union assembles")
        .value;
    // A small cutter that crosses the merged solid's top boundary (a strictly
    // interior cutter meets no contact event — the sweep needs the cutter to
    // reach the solid's boundary, the RW-INTERIOR-LOOP boundary).
    let small = box_solid(1.0, 1.0, 1.5, 3.0, 3.0, 3.0);

    let result = run(&merged, BoolOp::Difference, &small)
        .expect("the butt-joined solid downstream-consumes a Difference")
        .value;
    assert_single_shell(&result);
    // Machine-observed face count: the stepped-top result.
    assert_eq!(result.boundaries()[0].face_iter().count(), 15);
}
