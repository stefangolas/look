//! CTE-008-GATES — the differential congruence battery against the landed
//! `boolean_m2` results.
//!
//! Scope decision 1 (pre-made): on the landed `boolean_m2` fixture family the
//! CTE path agrees face-for-face with the landed results (V5-equivalent: zero
//! regressions on canonical pairs). The fixture CONSTRUCTION code is copied
//! verbatim from `tests/boolean_m2.rs` (read-only; never imported — the test
//! module is not a library). Every canonical pair boolean_m2 recorded is
//! re-driven here through the landed `boolean()` entry and asserted
//! face-for-face against the same extrude ground truth boolean_m2 pinned:
//!
//! - Difference ≅ Extrude(P − Q) as a face-set bijection, wall flipped;
//! - Intersection ≅ Extrude(Q), wall unflipped;
//! - Union (both orders) with the recorded census `(2 annuli, 2 disks, 4
//!   sides, no wall)` and the two orders' face sets bijecting.
//!
//! If a congruence mismatch appears the DEFAULT is that the CTE path is wrong
//! and the landed path is right: the test reports the mismatch, it never
//! reconciles silently (scope decision 6).

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
use truck_base::cgmath64::{Matrix4, Point2, Point3, Vector3, Vector4};
use truck_base::evidence::Budget;
use truck_geometry::arrange::{arrange, Arrangement};
use truck_geometry::canonical::{Curve, Surface};
use truck_geometry::prelude::*;
use truck_modeling::extrude::extrude_profile;
use truck_shapeops::boolean::assemble::boolean;
use truck_shapeops::boolean::BoolOp;
use truck_topology::{Face, Shell, Solid, Wire};

/// The tolerance class of the dyadic insertion geometry (H-3: dimensionless
/// relative to the unit-scale witnesses).
const TOL: f64 = 1.0e-2; // H-3: tolerance class for insertion geometry

// ---------------------------------------------------------------------------
// fixture construction (copied VERBATIM from boolean_m2.rs)
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

// ---------------------------------------------------------------------------
// face-set bijection machinery (decision 3, copied from boolean_m2.rs)
// ---------------------------------------------------------------------------

/// The curve kind of one edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum CurveKind {
    Line,
    Circle,
}

/// The raw curve-kind sequence of one absolute boundary wire.
fn wire_kinds(wire: &Wire<Point3, Curve>) -> Vec<CurveKind> {
    wire.edge_iter()
        .map(|edge| match edge.curve() {
            Curve::Line(_) => CurveKind::Line,
            Curve::Circle(_) => CurveKind::Circle,
            _ => unreachable!("no non-canonical curve in the dyadic witnesses"),
        })
        .collect()
}

/// The DISTINCT curve kinds of a wire, one per kind (a subdivided full circle
/// and an unsubdivided one read identically).
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
/// sampled at the dyadic wall point (u=0, v=1) -> (3, 2, z).
fn wall_is_outward(face: &Face<Point3, Curve, Surface>, cyl: &Cylinder) -> bool {
    let p = face.surface().subs(0.0, 1.0);
    let radial = Vector3::new(p.x - cyl.center().x, p.y - cyl.center().y, 0.0).normalize();
    effective_normal(face, 0.0, 1.0).dot(radial) > 0.0
}

/// The carrier + wire structure + effective orientation discriminant of one
/// face (decision 3).
#[derive(Clone, Debug)]
enum FaceKey {
    Plane {
        axis: usize,
        coord: f64,
        sign: f64,
        wires: Vec<Vec<CurveKind>>,
    },
    ZCylinder {
        cx: f64,
        cy: f64,
        r: f64,
        outward: bool,
        wires: Vec<Vec<CurveKind>>,
    },
}

/// The face key of one face.
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

/// Whether two face keys denote the same face-set member.
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

/// Asserts the face-set bijection: every face of `actual` finds exactly one
/// face of `expected` with the same carrier + wire structure + effective
/// orientation, and no `expected` face is left over.
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

/// The dot of the wall's effective normal with the outward radial direction at
/// the dyadic wall point (u=0, v=1).
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

/// The measured face census of one shell, `(annuli, disks, sides, walls)`.
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

/// The face keys of the vertical (x- or y-constant) plane faces of a shell.
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
// the differential gate
// ---------------------------------------------------------------------------

#[test]
fn differential_congruence_with_boolean_m2() {
    // Canonical pair 1 (Difference): the CTE-path boolean() Difference on the
    // block ∖ disk pair agrees face-for-face with the landed Extrude(P − Q)
    // construction — the same carrier + wire structure + effective
    // orientation, wall flipped.
    let (profile_a, arr_a) = block_profile();
    let solid_a = extrude_solid(&profile_a, &arr_a, 2.0);
    let (profile_b, arr_b) = disk_profile(Point2::new(2.0, 2.0), 1.0);
    let solid_b = extrude_solid(&profile_b, &arr_b, 2.0);
    let (profile_ph, arr_ph) = plate_with_hole_profile();
    let solid_ph = extrude_solid(&profile_ph, &arr_ph, 2.0);

    let mut budget = Budget::new(1000, 1000, 1000);
    let result = boolean(&solid_a, BoolOp::Difference, &solid_b, &mut budget)
        .expect("the Difference flagship assembles through the entry");
    let solid = result.value;
    assert_eq!(solid.boundaries().len(), 1);
    let shell = solid.boundaries().first().expect("one shell");
    assert_eq!(shell.face_iter().count(), 7);
    let shell_ph = solid_ph.boundaries().first().expect("one shell");
    assert_eq!(shell_ph.face_iter().count(), 7);
    assert_face_set_bijection(shell, shell_ph, "Difference vs Extrude(P-Q)");
    assert!(
        wall_radial_dot(shell) < 0.0,
        "the Difference wall must be flipped (effective normal toward the axis)"
    );

    // Canonical pair 2 (Intersection): the boolean() Intersection is the
    // cylinder column, bijecting Extrude(Q) with the UNFLIPPED wall.
    let mut inter_budget = Budget::new(1000, 1000, 1000);
    let inter = boolean(&solid_a, BoolOp::Intersection, &solid_b, &mut inter_budget)
        .expect("the Intersection flagship assembles through the entry")
        .value;
    let shell_inter = inter.boundaries().first().expect("one shell");
    assert_eq!(shell_inter.face_iter().count(), 3);
    let shell_b = solid_b.boundaries().first().expect("one shell");
    assert_face_set_bijection(shell_inter, shell_b, "Intersection vs Extrude(Q)");
    assert!(
        wall_radial_dot(shell_inter) > 0.0,
        "the Intersection wall must stay outward"
    );

    // Canonical pair 3 (Union): commutative in both orders, with the recorded
    // census (2 annuli, 2 disks, 4 sides, no wall) and each order's sides
    // bijecting Extrude(P)'s sides.
    let mut budget_ab = Budget::new(1000, 1000, 1000);
    let union_ab = boolean(&solid_a, BoolOp::Union, &solid_b, &mut budget_ab)
        .expect("A union B assembles through the entry")
        .value;
    let mut budget_ba = Budget::new(1000, 1000, 1000);
    let union_ba = boolean(&solid_b, BoolOp::Union, &solid_a, &mut budget_ba)
        .expect("B union A assembles through the entry")
        .value;
    let shell_ab = union_ab.boundaries().first().expect("one shell");
    let shell_ba = union_ba.boundaries().first().expect("one shell");
    assert_eq!(census(shell_ab), (2, 2, 4, 0));
    assert_eq!(census(shell_ba), (2, 2, 4, 0));
    assert_eq!(shell_ab.face_iter().count(), 8);
    assert_eq!(shell_ba.face_iter().count(), 8);
    let p_shell = solid_a.boundaries().first().expect("one shell");
    let ab_sides = vertical_plane_keys(shell_ab);
    let p_sides = vertical_plane_keys(p_shell);
    assert_eq!(ab_sides.len(), 4, "A union B has four sides");
    assert_eq!(p_sides.len(), 4, "Extrude(P) has four sides");
    assert_face_key_multiset(&ab_sides, &p_sides, "A union B sides vs Extrude(P)");
    let ba_sides = vertical_plane_keys(shell_ba);
    assert_eq!(ba_sides.len(), 4, "B union A has four sides");
    assert_face_key_multiset(&ba_sides, &p_sides, "B union A sides vs Extrude(P)");
    assert_face_set_bijection(shell_ab, shell_ba, "A union B vs B union A");
}
