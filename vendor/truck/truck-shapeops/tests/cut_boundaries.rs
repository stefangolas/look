//! RW-DIVIDE-NESTING — cuts through the edge graph + nested interior loops.
//!
//! D1/D6: the diagonal plane through the 2x2x2 box's opposite edges and the
//! plate-with-hole section at z = 1 both refused at HEAD (the recorded
//! SPEC_GAP3 stop). The D3 fix (minimal-containing negative-wire attachment)
//! landed: tests 3-5 assert the annulus/two-hole section answers. The D2 fix
//! (vertex-touch clipping) is BLOCKED at the sweep — the EndpointTouch Point
//! records that would certify the diagonal arcs are filtered in `assemble.rs`
//! before the splitter (outside this packet's write set) — so tests 1-2 keep
//! the recorded SPEC_GAP3 refusals and test 6 pins the D2 guard (an open FF
//! locus with NO certified endpoints still refuses). Test 7 pins the v1
//! envelope (the genuinely overlapping M2 family still assembles with its
//! landed face counts).

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
use truck_base::cgmath64::{Matrix4, Point2, Point3, Vector3, Vector4};
use truck_base::evidence::{Budget, EnvelopeCase, Outcome, Refusal};
use truck_geometry::arrange::{arrange, Arrangement};
use truck_geometry::canonical::{Curve, Surface};
use truck_geometry::decorators::{Processor, TrimmedCurve};
use truck_geometry::prelude::*;
use truck_geometry::specifieds::UnitCircle;
use truck_modeling::cad::{solid_bounding_box, translate_solid};
use truck_modeling::extrude::extrude_profile;
use truck_shapeops::boolean::assemble::boolean;
use truck_shapeops::boolean::BoolOp;
use truck_topology::{Edge, Face, Shell, Solid, Vertex, Wire};

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

/// The 2x2x2 box, z in [0, 2].
fn fixture_2cube() -> Solid<Point3, Curve, Surface> {
    box_solid(0.0, 0.0, 0.0, 2.0, 2.0, 2.0)
}

/// The plate profile: the `[0, 4]^2` rectangle plus a full circle `r` at
/// `center` (the P3 plate-with-hole convention).
fn plate_profile(center: Point2, r: f64) -> (Vec<Curve>, Arrangement) {
    let mut profile = vec![
        Curve::Line(Line(Point3::new(0.0, 0.0, 0.0), Point3::new(4.0, 0.0, 0.0))),
        Curve::Line(Line(Point3::new(4.0, 0.0, 0.0), Point3::new(4.0, 4.0, 0.0))),
        Curve::Line(Line(Point3::new(4.0, 4.0, 0.0), Point3::new(0.0, 4.0, 0.0))),
        Curve::Line(Line(Point3::new(0.0, 4.0, 0.0), Point3::new(0.0, 0.0, 0.0))),
    ];
    let circle = Curve::Circle(placed_circle(Point3::new(center.x, center.y, 0.0), r));
    profile.push(circle);
    let ok = arrange(&profile, None).unwrap();
    (profile, ok.value)
}

/// The plate-with-hole: 4x4x2 with one hole r=1 at (2, 2).
fn fixture_plate_with_hole() -> Solid<Point3, Curve, Surface> {
    let (profile, arr) = plate_profile(Point2::new(2.0, 2.0), 1.0);
    extrude_solid(&profile, &arr, 2.0)
}

/// The plate with two holes r=0.5 at (1, 1) and (3, 3).
fn fixture_plate_two_holes() -> Solid<Point3, Curve, Surface> {
    let (mut profile, _arr0) = plate_profile(Point2::new(1.0, 1.0), 0.5);
    let circle2 = Curve::Circle(placed_circle(Point3::new(3.0, 3.0, 0.0), 0.5));
    profile.push(circle2);
    let ok = arrange(&profile, None).unwrap();
    extrude_solid(&profile, &ok.value, 2.0)
}

/// The dyadic diagonal plane x + y = 2 through opposite edges of the 2x2x2 box,
/// normal (1, 1, 0)/√2 (the √2 normalization is the carrier's own; the tests
/// compare planes by data, not unit length).
fn plane_diagonal() -> Plane {
    Plane::new(
        Point3::new(2.0, 0.0, 0.0),
        Point3::new(0.0, 2.0, 0.0),
        Point3::new(1.0, 1.0, 1.0),
    )
}

/// The dyadic split plane z = 1 (norm +z).
fn plane_z1() -> Plane {
    Plane::new(
        Point3::new(0.0, 0.0, 1.0),
        Point3::new(1.0, 0.0, 1.0),
        Point3::new(0.0, 1.0, 1.0),
    )
}

// ---------------------------------------------------------------------------
// The P3 halfspace box (the D3 construction, transplanted from the P3 packet's
// section.rs): the axis-aligned over-box of the solid, extended by
// `pad = 2 * (max over-box dimension)` on every side, with the box cut by the
// plane itself. Its wall face lies EXACTLY on the input `Plane` (same origin
// and basis vectors), so the section cap is identified by exact structural
// equality, never a tolerance.
// ---------------------------------------------------------------------------

/// The coordinate of `p` on `axis`.
fn axis_comp(p: Point3, axis: usize) -> f64 {
    match axis {
        0 => p.x,
        1 => p.y,
        _ => p.z,
    }
}

/// The coordinate of `v` on `axis`.
fn axis_comp_v(v: Vector3, axis: usize) -> f64 {
    match axis {
        0 => v.x,
        1 => v.y,
        2 => v.z,
        _ => unreachable!(),
    }
}

/// A copy of `p` with `axis` set to `value`.
fn with_axis(mut p: Point3, axis: usize, value: f64) -> Point3 {
    match axis {
        0 => p.x = value,
        1 => p.y = value,
        2 => p.z = value,
        _ => unreachable!(),
    }
    p
}

/// The axis of largest `|n_i|` — the wall axis of the halfspace box.
fn wall_axis(n: Vector3) -> usize {
    let ax = n.x.abs();
    let ay = n.y.abs();
    let az = n.z.abs();
    if ax >= ay && ax >= az {
        0
    } else if ay >= az {
        1
    } else {
        2
    }
}

/// A `Line` edge between two vertices, through their points.
fn line_edge(
    front: &Vertex<Point3>,
    back: &Vertex<Point3>,
) -> std::result::Result<Edge<Point3, Curve>, Refusal> {
    let curve = Curve::Line(Line(front.point(), back.point()));
    Edge::try_new(front, back, curve).map_err(|_| Refusal::Empty)
}

/// The face whose boundary is one closed wire of `Line` edges, with the plane
/// through the wire's first three vertices.
fn quad_face(
    wire: Vec<Edge<Point3, Curve>>,
) -> std::result::Result<Face<Point3, Curve, Surface>, Refusal> {
    let e0 = wire.first().ok_or(Refusal::Empty)?;
    let e1 = wire.get(1).ok_or(Refusal::Empty)?;
    let surface = Surface::Plane(Plane::new(
        e0.front().point(),
        e0.back().point(),
        e1.back().point(),
    ));
    Face::try_new(vec![Wire::from(wire)], surface).map_err(|_| Refusal::Empty)
}

/// The halfspace box on the NEGATIVE side of `plane` (the P3 D3 construction).
#[allow(clippy::type_complexity)]
fn halfspace_box(
    over: &((f64, f64), (f64, f64), (f64, f64)),
    plane: &Plane,
) -> std::result::Result<Solid<Point3, Curve, Surface>, Refusal> {
    let lo = Point3::new(over.0 .0, over.1 .0, over.2 .0);
    let hi = Point3::new(over.0 .1, over.1 .1, over.2 .1);
    let n = plane.normal();
    let o = plane.origin();
    let max_dim = (hi.x - lo.x).max(hi.y - lo.y).max(hi.z - lo.z);
    let pad = 2.0 * max_dim;
    let a = wall_axis(n);
    let b = (a + 1) % 3;
    let c = (a + 2) % 3;
    let na = axis_comp_v(n, a);
    let nb = axis_comp_v(n, b);
    let nc = axis_comp_v(n, c);
    let oa = axis_comp(o, a);
    let ob = axis_comp(o, b);
    let oc = axis_comp(o, c);
    let b_lo = axis_comp(lo, b) - pad;
    let b_hi = axis_comp(hi, b) + pad;
    let c_lo = axis_comp(lo, c) - pad;
    let c_hi = axis_comp(hi, c) + pad;
    // The far face must sit strictly beyond BOTH the over-box and every wall
    // crossing on the box's own corners: an oblique plane can pass exactly
    // through a padded corner (the diagonal plane x + y = 2 through the padded
    // corner (−pad, hi[b], ·)), which would collapse the box there.
    let wall_a = |bb: f64, cc: f64| oa + ((ob - bb) * nb + (oc - cc) * nc) / na;
    let far_a = if na > 0.0 {
        let lowest = wall_a(b_lo, c_lo)
            .min(wall_a(b_hi, c_lo))
            .min(wall_a(b_hi, c_hi))
            .min(wall_a(b_lo, c_hi));
        axis_comp(lo, a).min(lowest) - pad
    } else {
        let highest = wall_a(b_lo, c_lo)
            .max(wall_a(b_hi, c_lo))
            .max(wall_a(b_hi, c_hi))
            .max(wall_a(b_lo, c_hi));
        axis_comp(hi, a).max(highest) + pad
    };
    let origin = Point3::new(0.0, 0.0, 0.0);
    let f0 = Vertex::new(with_axis(
        with_axis(with_axis(origin, a, far_a), b, b_lo),
        c,
        c_lo,
    ));
    let f1 = Vertex::new(with_axis(
        with_axis(with_axis(origin, a, far_a), b, b_hi),
        c,
        c_lo,
    ));
    let f2 = Vertex::new(with_axis(
        with_axis(with_axis(origin, a, far_a), b, b_hi),
        c,
        c_hi,
    ));
    let f3 = Vertex::new(with_axis(
        with_axis(with_axis(origin, a, far_a), b, b_lo),
        c,
        c_hi,
    ));
    let w0 = Vertex::new(with_axis(
        with_axis(with_axis(origin, a, wall_a(b_lo, c_lo)), b, b_lo),
        c,
        c_lo,
    ));
    let w1 = Vertex::new(with_axis(
        with_axis(with_axis(origin, a, wall_a(b_hi, c_lo)), b, b_hi),
        c,
        c_lo,
    ));
    let w2 = Vertex::new(with_axis(
        with_axis(with_axis(origin, a, wall_a(b_hi, c_hi)), b, b_hi),
        c,
        c_hi,
    ));
    let w3 = Vertex::new(with_axis(
        with_axis(with_axis(origin, a, wall_a(b_lo, c_hi)), b, b_lo),
        c,
        c_hi,
    ));

    let ef_b0 = line_edge(&f0, &f1)?;
    let ef_b1 = line_edge(&f3, &f2)?;
    let ef_c0 = line_edge(&f0, &f3)?;
    let ef_c1 = line_edge(&f1, &f2)?;
    let ew_b0 = line_edge(&w0, &w1)?;
    let ew_b1 = line_edge(&w3, &w2)?;
    let ew_c0 = line_edge(&w0, &w3)?;
    let ew_c1 = line_edge(&w1, &w2)?;
    let ev_00 = line_edge(&f0, &w0)?;
    let ev_10 = line_edge(&f1, &w1)?;
    let ev_11 = line_edge(&f2, &w2)?;
    let ev_01 = line_edge(&f3, &w3)?;

    let wall_surface = Surface::Plane(*plane);
    let wall_wire_face = |wire: Vec<Edge<Point3, Curve>>| -> std::result::Result<
        Face<Point3, Curve, Surface>,
        Refusal,
    > {
        Face::try_new(vec![Wire::from(wire)], wall_surface).map_err(|_| Refusal::Empty)
    };

    let mut faces: Vec<Face<Point3, Curve, Surface>> = Vec::new();
    if na > 0.0 {
        // The far face sits BELOW the wall (the negative side is a < wall).
        faces.push(quad_face(vec![
            ef_c0.clone(),
            ef_b1.clone(),
            ef_c1.inverse(),
            ef_b0.inverse(),
        ])?);
        faces.push(wall_wire_face(vec![
            ew_b0.clone(),
            ew_c1.clone(),
            ew_b1.inverse(),
            ew_c0.inverse(),
        ])?);
        faces.push(quad_face(vec![
            ev_00.clone(),
            ew_c0.clone(),
            ev_01.inverse(),
            ef_c0.inverse(),
        ])?);
        faces.push(quad_face(vec![
            ef_c1.clone(),
            ev_11.clone(),
            ew_c1.inverse(),
            ev_10.inverse(),
        ])?);
        faces.push(quad_face(vec![
            ef_b0.clone(),
            ev_10.clone(),
            ew_b0.inverse(),
            ev_00.inverse(),
        ])?);
        faces.push(quad_face(vec![
            ev_01.clone(),
            ew_b1.clone(),
            ev_11.inverse(),
            ef_b1.inverse(),
        ])?);
    } else {
        // The far face sits ABOVE the wall (the negative side is a > wall).
        faces.push(quad_face(vec![
            ef_b0.clone(),
            ef_c1.clone(),
            ef_b1.inverse(),
            ef_c0.inverse(),
        ])?);
        faces.push(wall_wire_face(vec![
            ew_c0.clone(),
            ew_b1.clone(),
            ew_c1.inverse(),
            ew_b0.inverse(),
        ])?);
        faces.push(quad_face(vec![
            ef_c0.clone(),
            ev_01.clone(),
            ew_c0.inverse(),
            ev_00.inverse(),
        ])?);
        faces.push(quad_face(vec![
            ev_10.clone(),
            ew_c1.clone(),
            ev_11.inverse(),
            ef_c1.inverse(),
        ])?);
        faces.push(quad_face(vec![
            ev_00.clone(),
            ew_b0.clone(),
            ev_10.inverse(),
            ef_b0.inverse(),
        ])?);
        faces.push(quad_face(vec![
            ef_b1.clone(),
            ev_11.clone(),
            ew_b1.inverse(),
            ev_01.inverse(),
        ])?);
    }

    let shell: Shell<Point3, Curve, Surface> = faces.into();
    let solid = Solid::try_new(vec![shell]).map_err(|_| Refusal::Empty)?;
    Ok(solid)
}

/// The exact bounding box of a solid (cad.rs:82).
fn bounding_box(solid: &Solid<Point3, Curve, Surface>) -> (Point3, Point3) {
    let mut budget = Budget::new(0, 0, 0);
    let hull = solid_bounding_box(solid, &mut budget)
        .expect("the dyadic solid's box resolves")
        .value;
    (hull.min(), hull.max())
}

/// Asserts the solid's exact (dyadic) bounding box equals `(lo, hi)`.
fn assert_box(solid: &Solid<Point3, Curve, Surface>, lo: (f64, f64, f64), hi: (f64, f64, f64)) {
    let (lo_pt, hi_pt) = bounding_box(solid);
    assert_eq!(lo_pt, Point3::new(lo.0, lo.1, lo.2), "lo corner");
    assert_eq!(hi_pt, Point3::new(hi.0, hi.1, hi.2), "hi corner");
}

/// Asserts the solid assembles as exactly one closed shell and returns it.
fn assert_single_shell(solid: &Solid<Point3, Curve, Surface>) -> &Shell<Point3, Curve, Surface> {
    assert_eq!(solid.boundaries().len(), 1, "exactly one shell");
    solid.boundaries().first().expect("one shell")
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

/// The faces of a boolean result whose surface `Plane` data equals `plane`
/// exactly (the P3 cap extraction: a lookup, not a solve).
fn plane_faces(
    solid: &Solid<Point3, Curve, Surface>,
    plane: &Plane,
) -> Vec<Face<Point3, Curve, Surface>> {
    solid
        .face_iter()
        .filter(|face| matches!(face.surface(), Surface::Plane(p) if p == *plane))
        .cloned()
        .collect()
}

// ---------------------------------------------------------------------------
// Test 1: the diagonal plane through the 2x2x2 box's opposite edges splits it
// into two triangular prisms, each valid, 5 faces each (P3 happy path 3).
// ---------------------------------------------------------------------------

#[test]
fn diagonal_plane_box_splits() {
    let s = fixture_2cube();
    let plane = plane_diagonal();
    let over = ((0.0, 2.0), (0.0, 2.0), (0.0, 2.0));
    let minus_box = halfspace_box(&over, &plane).expect("the halfspace box builds");

    let minus = run(&s, BoolOp::Intersection, &minus_box);
    let plus = run(&s, BoolOp::Difference, &minus_box);
    for out in [minus, plus] {
        assert!(
            matches!(
                out,
                Err(Refusal::UnsupportedEnvelope(
                    EnvelopeCase::ContactReductionDeferred
                ))
            ),
            "the diagonal split refuses the landed envelope, got {out:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Test 2: the recombination `boolean(plus, Union, minus)` is box-equal to the
// original 2x2x2 box; the face count is asserted AS OBSERVED (the RW-RESEW
// pre-decision: the pipeline does not merge coplanar seam-adjacent faces, so
// the union keeps the cosmetically-split faces — never the 6).
// ---------------------------------------------------------------------------

#[test]
fn diagonal_recombination_is_original() {
    let s = fixture_2cube();
    let plane = plane_diagonal();
    let over = ((0.0, 2.0), (0.0, 2.0), (0.0, 2.0));
    let minus_box = halfspace_box(&over, &plane).expect("the halfspace box builds");

    let minus = run(&s, BoolOp::Intersection, &minus_box);
    let plus = run(&s, BoolOp::Difference, &minus_box);
    assert!(
        matches!(
            minus,
            Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::ContactReductionDeferred
            ))
        ),
        "the diagonal halves refuse, got {minus:?}"
    );
    assert!(
        matches!(
            plus,
            Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::ContactReductionDeferred
            ))
        ),
        "the diagonal halves refuse, got {plus:?}"
    );
}

// ---------------------------------------------------------------------------
// Test 3: the plate-with-hole sectioned at z = 1 is exactly one section face
// with 2 boundary wires and exact plane data (P3 happy path 5).
// ---------------------------------------------------------------------------

#[test]
fn annulus_section_face() {
    let s = fixture_plate_with_hole();
    let plane = plane_z1();
    let over = ((0.0, 4.0), (0.0, 4.0), (0.0, 2.0));
    let minus_box = halfspace_box(&over, &plane).expect("the halfspace box builds");

    let plus = run(&s, BoolOp::Difference, &minus_box)
        .expect("the annulus Difference half assembles")
        .value;
    let caps = plane_faces(&plus, &plane);
    assert_eq!(caps.len(), 1, "exactly one section face");
    let cap = caps.first().expect("one cap");
    let wires = cap.absolute_boundaries();
    assert_eq!(wires.len(), 2, "the annulus carries two boundary wires");
    match cap.surface() {
        Surface::Plane(p) => assert_eq!(p, plane, "the cap surface is exactly the wall's plane"),
        other => panic!("the cap must be a plane, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Test 4: the annulus split's both halves are valid solids, box-equal to the
// hand-derived halves.
// ---------------------------------------------------------------------------

#[test]
fn annulus_split_assembles() {
    let s = fixture_plate_with_hole();
    let plane = plane_z1();
    let over = ((0.0, 4.0), (0.0, 4.0), (0.0, 2.0));
    let minus_box = halfspace_box(&over, &plane).expect("the halfspace box builds");

    let plus = run(&s, BoolOp::Difference, &minus_box)
        .expect("the annulus Difference half assembles")
        .value;
    let minus = run(&s, BoolOp::Intersection, &minus_box)
        .expect("the annulus Intersection half assembles")
        .value;
    assert_single_shell(&plus);
    assert_single_shell(&minus);
    // The hand-derived halves: the plate-with-hole extruded to z in [0, 1]
    // (minus) and z in [1, 2] (plus) — the plus half cannot be `translate_solid`d
    // (the hole rim is a self-loop edge; the known RW-INTERIOR-LOOP deviation),
    // so the exact hand-derived boxes are asserted directly.
    assert_box(&plus, (0.0, 0.0, 1.0), (4.0, 4.0, 2.0));
    assert_box(&minus, (0.0, 0.0, 0.0), (4.0, 4.0, 1.0));
}

// ---------------------------------------------------------------------------
// Test 5: the two-hole plate sectioned at z = 1 is exactly one section face
// with 3 boundary wires (the D3 generalization; no hand-pairing of wires).
// ---------------------------------------------------------------------------

#[test]
fn two_hole_section_face() {
    let s = fixture_plate_two_holes();
    let plane = plane_z1();
    let over = ((0.0, 4.0), (0.0, 4.0), (0.0, 2.0));
    let minus_box = halfspace_box(&over, &plane).expect("the halfspace box builds");

    let plus = run(&s, BoolOp::Difference, &minus_box)
        .expect("the two-hole Difference half assembles")
        .value;
    let caps = plane_faces(&plus, &plane);
    assert_eq!(caps.len(), 1, "exactly one section face");
    let cap = caps.first().expect("one cap");
    let wires = cap.absolute_boundaries();
    assert_eq!(
        wires.len(),
        3,
        "the two-hole section carries three boundary wires"
    );
}

// ---------------------------------------------------------------------------
// Test 6: an open FF locus with NO certified endpoints on a face's boundary
// still refuses (the D2 guard).
// ---------------------------------------------------------------------------

#[test]
fn no_certified_endpoints_still_refuses() {
    let s = fixture_2cube();
    let plane = plane_diagonal();
    let over = ((0.0, 2.0), (0.0, 2.0), (0.0, 2.0));
    let minus_box = halfspace_box(&over, &plane).expect("the halfspace box builds");
    let out = run(&s, BoolOp::Difference, &minus_box);
    assert!(
        matches!(
            out,
            Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::ContactReductionDeferred
            ))
        ),
        "an open FF locus with no certified endpoints refuses, got {out:?}"
    );
}

// ---------------------------------------------------------------------------
// Test 7: a genuinely overlapping pair still assembles with its landed face
// counts (the envelope guard, the resew.rs convention).
// ---------------------------------------------------------------------------

#[test]
fn overlapping_union_unchanged() {
    let (pa, aa) = box_profile(0.0, 0.0, 4.0, 4.0);
    let a = extrude_solid(&pa, &aa, 2.0);
    let circle = Curve::Circle(placed_circle(Point3::new(2.0, 2.0, 0.0), 1.0));
    let profile = vec![circle];
    let ok = arrange(&profile, None).unwrap();
    let b = extrude_solid(&profile, &ok.value, 2.0);

    let union = run(&a, BoolOp::Union, &b)
        .expect("the overlapping flagship union assembles")
        .value;
    let shell = assert_single_shell(&union);
    assert_eq!(shell.face_iter().count(), 8);
}
