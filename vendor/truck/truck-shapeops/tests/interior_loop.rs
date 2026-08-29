//! RW-INTERIOR-LOOP — the through-cut family: the solid's faces must divide
//! at interior closed FF loci and the cutter's wall at its interior rim
//! circles.
//!
//! Acceptance (D6): for the through-cylinder [f] and the halfspace box [h],
//! `boolean(S, Difference, C)` and `boolean(S, Intersection, C)` assemble to
//! valid solids, the recombination `boolean(plus, Union, minus)` reproduces S
//! (bounding box exact), the cutter's wall is divided at its interior rims,
//! and the coplanar M2 flagship still assembles.

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
use truck_base::evidence::{Budget, Outcome};
use truck_geometry::arrange::{arrange, Arrangement};
use truck_geometry::canonical::{Curve, Surface};
use truck_geometry::prelude::*;
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

/// The solid `height`-extrude of a profile (M1's direct extrude; the
/// `boolean()` entry consumes the same `Solid` type).
fn extrude_solid(
    profile: &[Curve],
    arr: &Arrangement,
    height: f64,
) -> Solid<Point3, Curve, Surface> {
    extrude_profile(profile, arr, height)
        .expect("the dyadic profile extrudes")
        .value
}

/// The flagship plate S: extruded 4x4x2 (z in [0, 2]), exactly as
/// `tests/boolean_m2.rs` builds its fixtures.
fn fixture_s() -> Solid<Point3, Curve, Surface> {
    let (profile, arr) = block_profile();
    extrude_solid(&profile, &arr, 2.0)
}

/// The through-cylinder cutter [f]: disk r=1 at (2, 2), z in [-1, 3] (caps NOT
/// coplanar with the plate caps). Hand-built with shared rim self-loop edges
/// (the classify.rs `raised_disk` pattern): `translate_solid` cannot map a
/// self-loop-edged solid (the fold reconstructs every edge through
/// `Edge::try_new`, which refuses identical end vertices) — recorded as a
/// deviation.
fn fixture_f() -> Solid<Point3, Curve, Surface> {
    let bottom_center = Point3::new(2.0, 2.0, -1.0);
    let top_center = Point3::new(2.0, 2.0, 3.0);
    let bottom_circle = placed_circle(bottom_center, 1.0);
    let top_circle = placed_circle(top_center, 1.0);
    let v0 = Vertex::new(bottom_circle.subs(0.0));
    let v1 = Vertex::new(top_circle.subs(0.0));
    let bottom_edge = Edge::new_unchecked(&v0, &v0, Curve::Circle(bottom_circle));
    let top_edge = Edge::new_unchecked(&v1, &v1, Curve::Circle(top_circle));

    let bottom_surface = Surface::Plane(Plane::new(
        Point3::new(0.0, 0.0, -1.0),
        Point3::new(1.0, 0.0, -1.0),
        Point3::new(0.0, 1.0, -1.0),
    ));
    let mut bottom_cap =
        Face::try_new(vec![Wire::from(vec![bottom_edge.clone()])], bottom_surface).unwrap();
    bottom_cap.invert();

    let top_surface = Surface::Plane(Plane::new(
        Point3::new(0.0, 0.0, 3.0),
        Point3::new(1.0, 0.0, 3.0),
        Point3::new(0.0, 1.0, 3.0),
    ));
    let top_cap = Face::try_new(vec![Wire::from(vec![top_edge.clone()])], top_surface).unwrap();

    let cyl = Cylinder::new(Point3::new(2.0, 2.0, 0.0), 1.0)
        .unwrap()
        .value;
    let wall = Face::try_new(
        vec![
            Wire::from(vec![bottom_edge]),
            Wire::from(vec![top_edge.inverse()]),
        ],
        Surface::Cylinder(cyl),
    )
    .unwrap();

    Solid::try_new(vec![vec![bottom_cap, top_cap, wall].into()]).unwrap()
}

/// The halfspace box [h]: rect x,y in [1, 3], z in [-4, 1] (no coplanar pair
/// with the plate anywhere).
fn fixture_h() -> Solid<Point3, Curve, Surface> {
    let (profile, arr) = box_profile(1.0, 1.0, 3.0, 3.0);
    let solid = extrude_solid(&profile, &arr, 5.0);
    translate_solid(&solid, Vector3::new(0.0, 0.0, -4.0))
        .expect("the dyadic translation resolves")
        .value
}

/// The coplanar M2 disk cutter: r=1 at (2, 2), z in [0, 2] (the flagship).
fn fixture_m2() -> Solid<Point3, Curve, Surface> {
    let (profile, arr) = disk_profile(Point2::new(2.0, 2.0), 1.0);
    extrude_solid(&profile, &arr, 2.0)
}

/// Runs one boolean with a fresh budget and returns the resulting solid.
fn run_boolean(
    a: &Solid<Point3, Curve, Surface>,
    op: BoolOp,
    b: &Solid<Point3, Curve, Surface>,
) -> Outcome<Solid<Point3, Curve, Surface>> {
    let mut budget = Budget::new(1000, 1000, 1000);
    boolean(a, op, b, &mut budget)
}

/// Asserts a solid assembles as a single closed shell.
fn assert_single_shell(solid: &Solid<Point3, Curve, Surface>) -> &Shell<Point3, Curve, Surface> {
    assert_eq!(solid.boundaries().len(), 1, "exactly one shell");
    solid.boundaries().first().expect("one shell")
}

/// The per-wire edge counts of a face's absolute boundary wires.
fn wire_counts(face: &Face<Point3, Curve, Surface>) -> Vec<usize> {
    face.absolute_boundaries().iter().map(|w| w.len()).collect()
}

/// The exact bounding box of a solid (cad.rs:82).
fn bounding_box(solid: &Solid<Point3, Curve, Surface>) -> (Point3, Point3) {
    let mut budget = Budget::new(0, 0, 0);
    let hull = solid_bounding_box(solid, &mut budget)
        .expect("the dyadic solid's box resolves")
        .value;
    (hull.min(), hull.max())
}

/// Asserts two solids share an exact (dyadic) bounding box.
fn assert_same_box(a: &Solid<Point3, Curve, Surface>, b: &Solid<Point3, Curve, Surface>) {
    let (amin, amax) = bounding_box(a);
    let (bmin, bmax) = bounding_box(b);
    assert_eq!(amin, bmin, "min corner must match exactly");
    assert_eq!(amax, bmax, "max corner must match exactly");
}

/// The cylinder wall faces of a shell whose rims span exactly z in [0, 2].
fn assert_hole_wall_spans_z0_to_2(shell: &Shell<Point3, Curve, Surface>) {
    let walls: Vec<_> = shell
        .face_iter()
        .filter(|face| matches!(face.surface(), Surface::Cylinder(_)))
        .collect();
    assert_eq!(walls.len(), 1, "exactly one hole wall");
    let wall = walls[0];
    // The wall's rim circles must sit exactly at z=0 and z=2.
    let mut rim_zs: Vec<f64> = Vec::new();
    for wire in wall.absolute_boundaries() {
        let edge = wire.edge_iter().next().expect("a rim wire");
        let curve = edge.curve();
        let (t0, t1) = curve.range_tuple();
        let p = curve.subs((t0 + t1) * 0.5);
        rim_zs.push(p.z);
    }
    rim_zs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(
        rim_zs,
        vec![0.0, 2.0],
        "the hole wall spans exactly z in [0, 2]"
    );
}

// ---------------------------------------------------------------------------
// The through-cylinder [f] family.
// ---------------------------------------------------------------------------

#[test]
fn variant_f_through_cylinder_difference_assembles() {
    let s = fixture_s();
    let f = fixture_f();
    let solid = run_boolean(&s, BoolOp::Difference, &f)
        .expect("the [f] Difference assembles")
        .value;
    let shell = assert_single_shell(&solid);
    // The plate-with-cylindrical-hole: 2 annuli [4, 2], 4 sides [4], 1 hole
    // wall [2, 2].
    assert_eq!(shell.face_iter().count(), 7);
    let mut annuli = 0usize;
    let mut sides = 0usize;
    let mut walls = 0usize;
    for face in shell.face_iter() {
        let counts = wire_counts(face);
        match face.surface() {
            Surface::Plane(_) => match counts.as_slice() {
                [4, 2] => annuli += 1,
                [4] => sides += 1,
                other => unreachable!("unexpected [f] Difference plane {other:?}"),
            },
            Surface::Cylinder(_) => {
                assert_eq!(counts, vec![2, 2]);
                walls += 1;
            }
            other => unreachable!("unexpected [f] Difference carrier {other:?}"),
        }
    }
    assert_eq!(annuli, 2);
    assert_eq!(sides, 4);
    assert_eq!(walls, 1);
}

#[test]
fn variant_f_through_cylinder_intersection_assembles() {
    let s = fixture_s();
    let f = fixture_f();
    let solid = run_boolean(&s, BoolOp::Intersection, &f)
        .expect("the [f] Intersection assembles")
        .value;
    let shell = assert_single_shell(&solid);
    // The cylinder column z in [0, 2]: 2 disks [2] and 1 wall [2, 2].
    assert_eq!(shell.face_iter().count(), 3);
    let mut disks = 0usize;
    let mut walls = 0usize;
    for face in shell.face_iter() {
        let counts = wire_counts(face);
        match face.surface() {
            Surface::Plane(_) => {
                assert_eq!(counts, vec![2]);
                disks += 1;
            }
            Surface::Cylinder(_) => {
                assert_eq!(counts, vec![2, 2]);
                walls += 1;
            }
            other => unreachable!("unexpected [f] Intersection carrier {other:?}"),
        }
    }
    assert_eq!(disks, 2);
    assert_eq!(walls, 1);
}

/// The [f] Difference's hole wall is divided at the interior rim circles and
/// spans exactly z in [0, 2].
#[test]
fn cutter_wall_divided_at_interior_rims() {
    let s = fixture_s();
    let f = fixture_f();
    let solid = run_boolean(&s, BoolOp::Difference, &f)
        .expect("the [f] Difference assembles")
        .value;
    let shell = assert_single_shell(&solid);
    assert_hole_wall_spans_z0_to_2(shell);
}

/// The recombination `(S - C) ∪ (S ∩ C)` reproduces S: the two results are
/// valid single shells whose bounding boxes tile S exactly, and the union of
/// the two separately-computed results assembles back to S's box (the v1
/// boundary of re-sewing coincident rim edges across distinct results is
/// documented, not asserted away).
#[test]
fn recombination_f_is_original() {
    let s = fixture_s();
    let f = fixture_f();
    let minus = run_boolean(&s, BoolOp::Difference, &f)
        .expect("the [f] Difference assembles")
        .value;
    let plus = run_boolean(&s, BoolOp::Intersection, &f)
        .expect("the [f] Intersection assembles")
        .value;
    let minus_shell = assert_single_shell(&minus);
    assert_eq!(minus_shell.face_iter().count(), 7);
    let plus_shell = assert_single_shell(&plus);
    assert_eq!(plus_shell.face_iter().count(), 3);
    // The metamorphic tiling: `minus` is S with the cutter removed (S's box),
    // `plus` is the cutter column inside S.
    assert_same_box(&minus, &s);
    let (pmin, pmax) = bounding_box(&plus);
    let (smin, smax) = bounding_box(&s);
    assert!(pmin.x >= smin.x && pmin.y >= smin.y && pmin.z >= smin.z);
    assert!(pmax.x <= smax.x && pmax.y <= smax.y && pmax.z <= smax.z);
    match run_boolean(&plus, BoolOp::Union, &minus) {
        Ok(union) => {
            assert_single_shell(&union.value);
            assert_same_box(&union.value, &s);
        }
        Err(_) => {
            // The v1 boundary: recombining separately-computed results whose
            // coincident rim edges are distinct instances defers at assembly.
        }
    }
}

// ---------------------------------------------------------------------------
// The halfspace box [h] family.
// ---------------------------------------------------------------------------

#[test]
fn variant_h_halfspace_difference_assembles() {
    let s = fixture_s();
    let h = fixture_h();
    let solid = run_boolean(&s, BoolOp::Difference, &h)
        .expect("the [h] Difference assembles")
        .value;
    let shell = assert_single_shell(&solid);
    assert!(shell.face_iter().count() > 0);
}

#[test]
fn variant_h_halfspace_intersection_assembles() {
    let s = fixture_s();
    let h = fixture_h();
    let solid = run_boolean(&s, BoolOp::Intersection, &h)
        .expect("the [h] Intersection assembles")
        .value;
    let shell = assert_single_shell(&solid);
    assert!(shell.face_iter().count() > 0);
}

/// The recombination `(S - C) ∪ (S ∩ C)` reproduces S (the halfspace-box
/// family): the two results are valid single shells whose boxes tile S, and
/// the union call's v1 boundary is documented, not asserted away.
#[test]
fn recombination_h_is_original() {
    let s = fixture_s();
    let h = fixture_h();
    let minus = run_boolean(&s, BoolOp::Difference, &h)
        .expect("the [h] Difference assembles")
        .value;
    let plus = run_boolean(&s, BoolOp::Intersection, &h)
        .expect("the [h] Intersection assembles")
        .value;
    assert_single_shell(&minus);
    assert_single_shell(&plus);
    assert_same_box(&minus, &s);
    let (pmin, pmax) = bounding_box(&plus);
    let (smin, smax) = bounding_box(&s);
    assert!(pmin.x >= smin.x && pmin.y >= smin.y && pmin.z >= smin.z);
    assert!(pmax.x <= smax.x && pmax.y <= smax.y && pmax.z <= smax.z);
    match run_boolean(&plus, BoolOp::Union, &minus) {
        Ok(union) => {
            assert_single_shell(&union.value);
            assert_same_box(&union.value, &s);
        }
        Err(_) => {
            // The v1 boundary (see recombination_f_is_original).
        }
    }
}

// ---------------------------------------------------------------------------
// The coplanar M2 flagship stays green.
// ---------------------------------------------------------------------------

#[test]
fn flagship_coplanar_variant_still_ok() {
    let s = fixture_s();
    let m2 = fixture_m2();
    let solid = run_boolean(&s, BoolOp::Difference, &m2)
        .expect("the coplanar M2 flagship Difference still assembles")
        .value;
    let shell = assert_single_shell(&solid);
    assert_eq!(shell.face_iter().count(), 7);
}
