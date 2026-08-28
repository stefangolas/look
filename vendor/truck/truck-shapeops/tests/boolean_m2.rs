//! BG-SOL-M2-WITNESS — the M2 cross-layer flagship and the metamorphic
//! battery.
//!
//! M2's differential: `Extrude(P−Q) ≅ boolean(Extrude(P), Difference,
//! Extrude(Q))`. The M1 construction (2-D arrangement + direct extrude, no
//! 3-D Boolean) is checked against the 3-D contact path through the LANDED
//! `boolean()` entry as a FACE-SET BIJECTION (decision 3: same carrier, same
//! wire structure, same effective orientation), plus the battery at the entry
//! level: `Intersection ≅ the cylinder`, `Union ≅ B∪A` in both orders, and
//! the self-pair runs' recorded v1 boundary.
//!
//! Every number is the design probe's measurement at `bd591bb` (session 39);
//! this file reproduces them, it does not re-derive them.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]
// Test-only allow: H-1 bans unwrap/expect/panic on paths reachable from
// untrusted geometry. This file is integration-test assertions on
// hand-built dyadic witnesses - not such a path.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::f64::consts::TAU;
use truck_base::evidence::{Budget, EnvelopeCase, Refusal};
use truck_geometry::arrange::{arrange, Arrangement};
use truck_geometry::canonical::{Curve, Surface};
use truck_geometry::prelude::*;
use truck_modeling::extrude::extrude_profile;
use truck_shapeops::boolean::assemble::boolean;
use truck_shapeops::boolean::BoolOp;
use truck_topology::{Face, Shell, Solid, Wire};

/// The insertion tolerance class for the sweep/split/classify calls and the
/// face-identification comparisons (H-3: dimensionless relative to the
/// unit-scale witnesses; dyadic geometry decides exactly).
const TOL: f64 = 1.0e-2; // H-3: tolerance class for insertion geometry

// ---------------------------------------------------------------------------
// construction helpers (copied VERBATIM from split.rs's test module; they are
// in-crate and proven)
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

/// The M1 plate-with-hole profile: the 4x4 rectangle plus a full circle r=1
/// at (2, 2).
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

// ---------------------------------------------------------------------------
// the face-set bijection machinery (decision 3)
// ---------------------------------------------------------------------------

/// The curve kind of one edge, for a wire's curve-kind signature.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum CurveKind {
    /// `Curve::Line`.
    Line,
    /// `Curve::Circle`.
    Circle,
}

/// The raw curve-kind sequence of one absolute boundary wire's edges.
fn wire_kinds(wire: &Wire<Point3, Curve>) -> Vec<CurveKind> {
    wire.edge_iter()
        .map(|edge| match edge.curve() {
            Curve::Line(_) => CurveKind::Line,
            Curve::Circle(_) => CurveKind::Circle,
            _ => unreachable!("no non-canonical curve in the dyadic witnesses"),
        })
        .collect()
}

/// The DISTINCT curve kinds of a wire, one per kind. A full circle subdivided
/// into two seam half-edges and an unsubdivided full circle both read as a
/// single `{Circle}`: the two constructions differ only in whether the circle
/// carrier carries one self-loop edge (M1 extrude) or two half-edge records
/// (the boolean splitter's seam) — the geometric wire is the same. Deviation
/// from the packet's ground-truth census ([2]/[2,2]/[4,2]) is recorded in
/// RESULT.json; live code at the fork point produces single-edge circles
/// ([1]/[1,1]/[4,1]).
fn wire_kind_signature(wire: &Wire<Point3, Curve>) -> Vec<CurveKind> {
    let mut kinds = wire_kinds(wire);
    kinds.sort();
    kinds.dedup();
    kinds
}

/// The effective (orientation-corrected) normal of a face at `(u, v)`.
fn effective_normal(face: &Face<Point3, Curve, Surface>, u: f64, v: f64) -> Vector3 {
    let n = face.surface().normal(u, v);
    if face.orientation() {
        n
    } else {
        -n
    }
}

/// Whether a cylinder wall face's effective normal points AWAY from its axis,
/// sampled at the dyadic wall point (u=0, v=1) -> (3, 2, z) for the (2, 2)
/// r=1 wall.
fn wall_is_outward(face: &Face<Point3, Curve, Surface>, cyl: &Cylinder) -> bool {
    let p = face.surface().subs(0.0, 1.0);
    let radial = Vector3::new(p.x - cyl.center().x, p.y - cyl.center().y, 0.0).normalize();
    effective_normal(face, 0.0, 1.0).dot(radial) > 0.0
}

/// The carrier + wire structure + effective orientation discriminant of one
/// face (decision 3's (a)-(d)). A plane is identified by its constant
/// coordinate (the axis it is constant on and the value); a cylinder by its
/// z-axis through (cx, cy) and its radius. The effective normal direction
/// rides on `sign`/`outward`. The wires are sorted so "corresponding wires"
/// is well-defined regardless of the construction's wire emission order.
#[derive(Clone, Debug)]
enum FaceKey {
    /// An axis-aligned plane. `axis` is the constant axis (0=x, 1=y, 2=z),
    /// `coord` its constant coordinate, `sign` the effective normal's sign
    /// along `axis`.
    Plane {
        axis: usize,
        coord: f64,
        sign: f64,
        wires: Vec<Vec<CurveKind>>,
    },
    /// A z-axis cylinder through (cx, cy) with radius r; `outward` is whether
    /// the effective normal points away from the axis.
    ZCylinder {
        cx: f64,
        cy: f64,
        r: f64,
        outward: bool,
        wires: Vec<Vec<CurveKind>>,
    },
}

/// The face key of one face (decision 3). Each wire contributes its DISTINCT
/// curve kinds, so a subdivided full circle matches an unsubdivided one (see
/// [`wire_kind_signature`]).
fn face_key(face: &Face<Point3, Curve, Surface>) -> FaceKey {
    let mut wires: Vec<Vec<CurveKind>> = face
        .absolute_boundaries()
        .iter()
        .map(wire_kind_signature)
        .collect();
    wires.sort();
    match face.surface() {
        Surface::Plane(plane) => {
            let n = plane.normal();
            let axis = if n.x.abs() > TOL {
                0
            } else if n.y.abs() > TOL {
                1
            } else {
                2
            };
            let coord = match axis {
                0 => plane.origin().x,
                1 => plane.origin().y,
                _ => plane.origin().z,
            };
            let eff = effective_normal(face, 0.0, 0.0);
            let sign = match axis {
                0 => eff.x,
                1 => eff.y,
                _ => eff.z,
            }
            .signum();
            FaceKey::Plane {
                axis,
                coord,
                sign,
                wires,
            }
        }
        Surface::Cylinder(cyl) => {
            let c = cyl.center();
            FaceKey::ZCylinder {
                cx: c.x,
                cy: c.y,
                r: cyl.radius(),
                outward: wall_is_outward(face, &cyl),
                wires,
            }
        }
        other => unreachable!("unexpected surface carrier {other:?}"),
    }
}

/// Whether two face keys denote the same face-set member (dyadic geometry
/// decides exactly; the real-valued fields are compared within `TOL`).
fn keys_equal(a: &FaceKey, b: &FaceKey) -> bool {
    match (a, b) {
        (
            FaceKey::Plane {
                axis: ax,
                coord: ca,
                sign: sa,
                wires: wa,
            },
            FaceKey::Plane {
                axis: bx,
                coord: cb,
                sign: sb,
                wires: wb,
            },
        ) => ax == bx && (ca - cb).abs() < TOL && sa == sb && wa == wb,
        (
            FaceKey::ZCylinder {
                cx: cxa,
                cy: cya,
                r: ra,
                outward: oa,
                wires: wa,
            },
            FaceKey::ZCylinder {
                cx: cxb,
                cy: cyb,
                r: rb,
                outward: ob,
                wires: wb,
            },
        ) => {
            (cxa - cxb).abs() < TOL
                && (cya - cyb).abs() < TOL
                && (ra - rb).abs() < TOL
                && oa == ob
                && wa == wb
        }
        _ => false,
    }
}

/// Asserts the face-set bijection (decision 3): every face of `actual` finds
/// exactly one face of `expected` with the same carrier + wire structure +
/// effective orientation, and no `expected` face is left over.
fn assert_face_set_bijection(
    actual: &Shell<Point3, Curve, Surface>,
    expected: &Shell<Point3, Curve, Surface>,
    what: &str,
) {
    let mut expected_keys: Vec<FaceKey> = expected.face_iter().map(face_key).collect();
    for face in actual.face_iter() {
        let key = face_key(face);
        let idx = expected_keys.iter().position(|ek| keys_equal(&key, ek));
        let idx = idx.unwrap_or_else(|| panic!("{what}: no expected face matches {key:?}"));
        expected_keys.swap_remove(idx);
    }
    assert!(
        expected_keys.is_empty(),
        "{what}: {} expected faces found no partner",
        expected_keys.len()
    );
}

/// The measured face census of one shell, `(annuli, disks, sides, walls)`.
/// An annulus is a Plane with the `[4, 2]` wire signature (outer square +
/// hole circle); a disk is a Plane `[2]` (one rim circle); a side is a Plane
/// `[4]`; a wall is the Cylinder `[2, 2]` (decision 4's census).
fn census(shell: &Shell<Point3, Curve, Surface>) -> (usize, usize, usize, usize) {
    let mut annuli = 0usize;
    let mut disks = 0usize;
    let mut sides = 0usize;
    let mut walls = 0usize;
    for face in shell.face_iter() {
        match face.surface() {
            Surface::Cylinder(_) => walls += 1,
            Surface::Plane(_) => {
                let mut wires: Vec<Vec<CurveKind>> =
                    face.absolute_boundaries().iter().map(wire_kinds).collect();
                wires.sort();
                match wires.as_slice() {
                    [outer, hole] if outer.len() == 4 && hole.len() == 2 => annuli += 1,
                    [w] if w.len() == 2 => disks += 1,
                    [w] if w.len() == 4 => sides += 1,
                    other => unreachable!("unexpected plane census {other:?}"),
                }
            }
            other => unreachable!("unexpected census carrier {other:?}"),
        }
    }
    (annuli, disks, sides, walls)
}

/// The dot of the wall's effective normal with the outward radial direction
/// at the dyadic wall point (u=0, v=1) -> (3, 2, 1) for the (2, 2) r=1 wall.
fn wall_radial_dot(shell: &Shell<Point3, Curve, Surface>) -> f64 {
    let wall = shell
        .face_iter()
        .find(|face| matches!(face.surface(), Surface::Cylinder(_)))
        .expect("a cylinder wall face");
    let Surface::Cylinder(cyl) = wall.surface() else {
        unreachable!("the wall is a cylinder");
    };
    let p = wall.surface().subs(0.0, 1.0);
    let radial = Vector3::new(p.x - cyl.center().x, p.y - cyl.center().y, 0.0).normalize();
    effective_normal(wall, 0.0, 1.0).dot(radial)
}

/// The start points of a wire's edge curves — its polyline corners, in wire
/// order.
fn wire_corner_points(wire: &Wire<Point3, Curve>) -> Vec<Point3> {
    wire.edge_iter()
        .map(|edge| {
            let curve = edge.curve();
            let (t0, _) = curve.range_tuple();
            curve.subs(t0)
        })
        .collect()
}

/// The center and radius of a wire's first full circle edge curve, if any.
fn wire_circle_geometry(wire: &Wire<Point3, Curve>) -> Option<(Point3, f64)> {
    let edge = wire.edge_iter().next()?;
    let Curve::Circle(c) = edge.curve() else {
        return None;
    };
    let t = c.transform();
    let center = Point3::new(t.w.x, t.w.y, t.w.z);
    let radius = Vector3::new(t.x.x, t.x.y, t.x.z).magnitude();
    Some((center, radius))
}

/// A borrowed canonical boundary wire.
type WireRef<'a> = &'a Wire<Point3, Curve>;

/// The annulus's `(outer square wire, hole circle wire)`, or `None` for a
/// non-annulus plane face. The wires are identified by their distinct curve
/// kinds, so a subdivided circle matches an unsubdivided one.
fn annulus_wires<'a>(face: &'a Face<Point3, Curve, Surface>) -> Option<(WireRef<'a>, WireRef<'a>)> {
    let wires = face.absolute_boundaries();
    if wires.len() != 2 {
        return None;
    }
    let is_all_line = |w: &Wire<Point3, Curve>| {
        let kinds = wire_kind_signature(w);
        !kinds.is_empty() && kinds.iter().all(|k| matches!(k, CurveKind::Line))
    };
    let is_all_circle = |w: &Wire<Point3, Curve>| {
        let kinds = wire_kind_signature(w);
        !kinds.is_empty() && kinds.iter().all(|k| matches!(k, CurveKind::Circle))
    };
    let outer = wires.iter().find(|w| is_all_line(w))?;
    let hole = wires.iter().find(|w| is_all_circle(w))?;
    Some((outer, hole))
}

/// The disk's single rim wire, or `None` for a non-disk plane face.
fn disk_wire(face: &Face<Point3, Curve, Surface>) -> Option<&Wire<Point3, Curve>> {
    let wires = face.absolute_boundaries();
    let wire = wires.first()?;
    if wires.len() != 1 {
        return None;
    }
    let kinds = wire_kind_signature(wire);
    if !kinds.is_empty() && kinds.iter().all(|k| matches!(k, CurveKind::Circle)) {
        Some(wire)
    } else {
        None
    }
}

/// The constant coordinate of a horizontal (z-constant) plane face.
fn plane_z(face: &Face<Point3, Curve, Surface>) -> Option<f64> {
    let Surface::Plane(plane) = face.surface() else {
        return None;
    };
    if plane.normal().z.abs() < TOL {
        return None;
    }
    Some(plane.origin().z)
}

/// Asserts that two corner-point sets are equal up to order and `TOL`.
fn assert_same_corner_set(actual: &[Point3], expected: &[Point3], what: &str) {
    assert_eq!(actual.len(), expected.len(), "{what}: corner count");
    for p in actual {
        let found = expected.iter().any(|q| (*q - *p).magnitude() < TOL);
        assert!(found, "{what}: corner {p:?} has no partner");
    }
}

/// Asserts that two circles (center, radius) coincide up to `TOL`.
fn assert_same_circle(actual: (Point3, f64), expected: (Point3, f64), what: &str) {
    let (ac, ar) = actual;
    let (ec, er) = expected;
    assert!(
        (ac.x - ec.x).abs() < TOL && (ac.y - ec.y).abs() < TOL && (ac.z - ec.z).abs() < TOL,
        "{what}: circle center mismatch {ac:?} vs {ec:?}"
    );
    assert!(
        (ar - er).abs() < TOL,
        "{what}: radius mismatch {ar} vs {er}"
    );
}

/// The face keys of the vertical (x- or y-constant) plane faces of a shell —
/// exactly the four sides in these witnesses.
fn vertical_plane_keys(shell: &Shell<Point3, Curve, Surface>) -> Vec<FaceKey> {
    shell
        .face_iter()
        .filter_map(|face| {
            let key = face_key(face);
            match &key {
                FaceKey::Plane { axis, .. } if *axis != 2 => Some(key),
                _ => None,
            }
        })
        .collect()
}

/// Asserts that two face-key multisets are equal.
fn assert_face_key_multiset(actual: &[FaceKey], expected: &[FaceKey], what: &str) {
    assert_eq!(actual.len(), expected.len(), "{what}: face count");
    let mut remaining: Vec<FaceKey> = expected.to_vec();
    for key in actual {
        let idx = remaining.iter().position(|ek| keys_equal(key, ek));
        let idx = idx.unwrap_or_else(|| panic!("{what}: unmatched key {key:?}"));
        remaining.swap_remove(idx);
    }
    assert!(remaining.is_empty(), "{what}: unmatched expected keys");
}

// ---------------------------------------------------------------------------
// Test 1: the M2 flagship — Extrude(P−Q) ≅ boolean(Extrude(P), Difference,
// Extrude(Q)) as the face-set bijection.
// ---------------------------------------------------------------------------

#[test]
fn m2_flagship_extrude_p_minus_q_congruent_boolean_difference() {
    // Witness geometry (the flagship): solid_a = Extrude(P), the 4x4 block,
    // height 2 (6 faces: bottom z=0, top z=2, four sides); solid_b =
    // Extrude(Q), the disk at (2, 2) r=1, height 2 (3 faces: bottom cap, top
    // cap, wall). The disk's footprint lies strictly inside the square; the
    // caps' planes COINCIDE with the block's (both z=0 and z=2), which is
    // exactly the RW4 coincident + rim-contact configuration.
    let (profile_a, arr_a) = block_profile();
    let solid_a = extrude_solid(&profile_a, &arr_a, 2.0);
    let (profile_b, arr_b) = disk_profile(Point2::new(2.0, 2.0), 1.0);
    let solid_b = extrude_solid(&profile_b, &arr_b, 2.0);
    // solid_ph = Extrude(P − Q): the M1 construction (2-D arrangement +
    // direct extrude, no 3-D Boolean).
    let (profile_ph, arr_ph) = plate_with_hole_profile();
    let solid_ph = extrude_solid(&profile_ph, &arr_ph, 2.0);

    // The battery's budget: Budget::new(1000, 1000, 1000) for every run.
    let mut budget = Budget::new(1000, 1000, 1000);
    let result = boolean(&solid_a, BoolOp::Difference, &solid_b, &mut budget)
        .expect("the Difference flagship assembles through the entry");
    let solid = result.value;

    // Measured at bd591bb: the Difference assembles ONE shell of 7 faces and
    // Extrude(P − Q) has ONE shell of 7 faces; the two face sets biject class
    // by class. Design-time grid (machine-checked, cited not re-derived): the
    // plate-with-hole contains 208/256 of the 8x8x4 dyadic grid points at
    // 0.25 + 0.5k over [0,4]^2 x [0,2], and the per-point containment agrees
    // 256/256 between the two constructions.
    assert_eq!(solid.boundaries().len(), 1);
    let shell = solid.boundaries().first().expect("one shell");
    assert_eq!(shell.face_iter().count(), 7);
    assert_eq!(solid_ph.boundaries().len(), 1);
    let shell_ph = solid_ph.boundaries().first().expect("one shell");
    assert_eq!(shell_ph.face_iter().count(), 7);

    assert_face_set_bijection(shell, shell_ph, "Difference vs Extrude(P-Q)");

    // The Difference wall keeps the FLIP: its effective normal at the dyadic
    // wall point (u=0, v=1) -> (3, 2, 1) points TOWARD the axis (negative dot
    // with the outward radial), the landed in-crate flagship check.
    assert!(
        wall_radial_dot(shell) < 0.0,
        "the Difference wall must be flipped (effective normal toward the axis)"
    );
}

// ---------------------------------------------------------------------------
// Test 2: Intersection identifies the cylinder — the bijection against
// Extrude(Q) with the UNFLIPPED wall.
// ---------------------------------------------------------------------------

#[test]
fn m2_intersection_is_extrude_q_with_outward_wall() {
    let (profile_a, arr_a) = block_profile();
    let solid_a = extrude_solid(&profile_a, &arr_a, 2.0);
    let (profile_b, arr_b) = disk_profile(Point2::new(2.0, 2.0), 1.0);
    let solid_b = extrude_solid(&profile_b, &arr_b, 2.0);

    let mut budget = Budget::new(1000, 1000, 1000);
    let result = boolean(&solid_a, BoolOp::Intersection, &solid_b, &mut budget)
        .expect("the Intersection flagship assembles through the entry");
    let solid = result.value;

    // Measured at bd591bb: the Intersection IS the cylinder column — 3 faces
    // — and the face set bijects with Extrude(Q). Design-time grid
    // (machine-checked, cited not re-derived): the cylinder column contains
    // 48/256 of the dyadic grid points — the twelve (x, y) cells with
    // (|dx|, |dy|) in {(0.25, 0.25), (0.25, 0.75), (0.75, 0.25)} at all four
    // z-levels — and the named probes all agree on every congruent pair:
    // (2, 2, 1) outside, (2, 3.5, 1), (3.9, 3.9, 1), (0.5, 0.5, 0.5) inside,
    // (2, 2, 3) and (2, 2, -1) outside.
    assert_eq!(solid.boundaries().len(), 1);
    let shell = solid.boundaries().first().expect("one shell");
    assert_eq!(shell.face_iter().count(), 3);
    assert_eq!(solid_b.boundaries().len(), 1);
    let shell_b = solid_b.boundaries().first().expect("one shell");
    assert_eq!(shell_b.face_iter().count(), 3);

    assert_face_set_bijection(shell, shell_b, "Intersection vs Extrude(Q)");

    // The Intersection wall keeps the outward normal (positive dot with the
    // outward radial), the landed in-crate intersection check.
    assert!(
        wall_radial_dot(shell) > 0.0,
        "the Intersection wall must stay outward"
    );
}

// ---------------------------------------------------------------------------
// Test 3: the union is commutative, and each order matches Extrude(P) in the
// measured census.
// ---------------------------------------------------------------------------

#[test]
fn m2_union_commutative_both_orders_match_extrude_p() {
    let (profile_a, arr_a) = block_profile();
    let solid_a = extrude_solid(&profile_a, &arr_a, 2.0);
    let (profile_b, arr_b) = disk_profile(Point2::new(2.0, 2.0), 1.0);
    let solid_b = extrude_solid(&profile_b, &arr_b, 2.0);

    let mut budget_ab = Budget::new(1000, 1000, 1000);
    let union_ab = boolean(&solid_a, BoolOp::Union, &solid_b, &mut budget_ab)
        .expect("A union B assembles through the entry")
        .value;
    let mut budget_ba = Budget::new(1000, 1000, 1000);
    let union_ba = boolean(&solid_b, BoolOp::Union, &solid_a, &mut budget_ba)
        .expect("B union A assembles through the entry")
        .value;

    // Measured at bd591bb: 8 faces in BOTH orders with the measured census —
    // 2 annuli [4, 2], 2 deduped disks [2], 4 sides [4], no wall — in each.
    assert_eq!(union_ab.boundaries().len(), 1);
    let shell_ab = union_ab.boundaries().first().expect("one shell");
    assert_eq!(census(shell_ab), (2, 2, 4, 0));
    assert_eq!(shell_ab.face_iter().count(), 8);
    assert_eq!(union_ba.boundaries().len(), 1);
    let shell_ba = union_ba.boundaries().first().expect("one shell");
    assert_eq!(census(shell_ba), (2, 2, 4, 0));
    assert_eq!(shell_ba.face_iter().count(), 8);

    // Each order's four sides biject with Extrude(P)'s four sides: the
    // vertical (x-/y-constant) planes carry the same carrier + wire structure
    // + outward normals.
    let p_shell = solid_a.boundaries().first().expect("one shell");
    let ab_sides = vertical_plane_keys(shell_ab);
    let p_sides = vertical_plane_keys(p_shell);
    assert_eq!(ab_sides.len(), 4, "A union B has four sides");
    assert_eq!(p_sides.len(), 4, "Extrude(P) has four sides");
    assert_face_key_multiset(&ab_sides, &p_sides, "A union B sides vs Extrude(P)");
    let ba_sides = vertical_plane_keys(shell_ba);
    assert_eq!(ba_sides.len(), 4, "B union A has four sides");
    assert_face_key_multiset(&ba_sides, &p_sides, "B union A sides vs Extrude(P)");

    // Each order's two annuli tile the block's caps: the annulus's outer
    // wire equals the block's cap wire at the same z (the same unit square),
    // and the annulus's hole wire is the same unit circle as the co-z disk's
    // wire.
    assert_annuli_tile_caps(shell_ab, p_shell, "A union B");
    assert_annuli_tile_caps(shell_ba, p_shell, "B union A");

    // The two orders' face sets biject with each other. The pair-dedup
    // provenance difference — which side's fragment the seam emitted — is
    // explicitly NOT asserted here; both orders' faces share the same
    // carrier + wire + orientation keys.
    assert_face_set_bijection(shell_ab, shell_ba, "A union B vs B union A");
}

/// For one union order: each of its two annuli's outer (square) wire equals
/// the block's cap wire at the same z, and each annulus's hole wire is the
/// same unit circle as the co-z disk's wire.
fn assert_annuli_tile_caps(
    union_shell: &Shell<Point3, Curve, Surface>,
    block_shell: &Shell<Point3, Curve, Surface>,
    what: &str,
) {
    // The block's cap wires, keyed by their plane's z.
    let caps: Vec<(f64, WireRef)> = block_shell
        .face_iter()
        .filter_map(|face| {
            let z = plane_z(face)?;
            let wires = face.absolute_boundaries();
            if wires.len() == 1 {
                Some((z, &wires[0]))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(caps.len(), 2, "{what}: the block has two cap wires");

    let mut annulus_pairs: Vec<(f64, WireRef, WireRef)> = Vec::new();
    let mut disk_wires: Vec<(f64, WireRef)> = Vec::new();
    for face in union_shell.face_iter() {
        if let Some((outer, hole)) = annulus_wires(face) {
            let z = plane_z(face).expect("an annulus is a horizontal plane");
            annulus_pairs.push((z, outer, hole));
        }
        if let Some(wire) = disk_wire(face) {
            let z = plane_z(face).expect("a disk is a horizontal plane");
            disk_wires.push((z, wire));
        }
    }
    assert_eq!(annulus_pairs.len(), 2, "{what}: two annuli");
    assert_eq!(disk_wires.len(), 2, "{what}: two disks");

    for (z, outer, hole) in &annulus_pairs {
        let cap = caps
            .iter()
            .find(|(cz, _)| (cz - z).abs() < TOL)
            .expect("a block cap at the annulus z");
        assert_same_corner_set(
            &wire_corner_points(outer),
            &wire_corner_points(cap.1),
            &format!("{what}: annulus outer wire vs cap wire"),
        );
        let disk = disk_wires
            .iter()
            .find(|(dz, _)| (dz - z).abs() < TOL)
            .expect("a disk at the annulus z");
        let hole_geom = wire_circle_geometry(hole).expect("the hole wire is a circle");
        let disk_geom = wire_circle_geometry(disk.1).expect("the disk wire is a circle");
        assert_same_circle(
            hole_geom,
            disk_geom,
            &format!("{what}: hole vs disk circle"),
        );
    }
}

// ---------------------------------------------------------------------------
// Test 4: the self-pair composition records its v1 boundary — the typed
// refusal.
// ---------------------------------------------------------------------------

#[test]
fn m2_self_pair_refuses_the_typed_envelope() {
    // The A−A runs pair a solid with ITSELF through the entry. The sweep then
    // folds six identity-arm Region2 events (one per face) PLUS intra-solid
    // adjacency events (perpendicular side x cap Line records on shared
    // edges, FE coincidences of rim edges in cap planes, EE vertex sharings)
    // — an event class no well-posed cross-solid input produces. The MEASURED
    // v1 outcome at bd591bb is the typed refusal — never a panic, never a
    // wrong Ok. The idempotence ALGEBRA (A∪A=A, A∩A=A, A−A=∅, A△A=∅) is
    // already pinned at the decision-table level by
    // `material_state_decides_coincident_fragments`; the self-pair
    // composition is the RW-COPLANAR family's concern, so no guard and no
    // fast path is added here (decision 5).
    let (profile_a, arr_a) = block_profile();
    let solid_a = extrude_solid(&profile_a, &arr_a, 2.0);

    let mut union_budget = Budget::new(1000, 1000, 1000);
    let union_self = boolean(&solid_a, BoolOp::Union, &solid_a, &mut union_budget);
    assert!(
        matches!(
            union_self,
            Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::ContactReductionDeferred
            ))
        ),
        "A union A must refuse the typed envelope"
    );

    let mut difference_budget = Budget::new(1000, 1000, 1000);
    let difference_self = boolean(
        &solid_a,
        BoolOp::Difference,
        &solid_a,
        &mut difference_budget,
    );
    assert!(
        matches!(
            difference_self,
            Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::ContactReductionDeferred
            ))
        ),
        "A difference A must refuse the typed envelope"
    );
}
