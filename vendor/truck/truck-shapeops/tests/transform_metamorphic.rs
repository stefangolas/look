//! BG-CAD-P10-FRAMED — the packet's truck-shapeops metamorphic battery.
//!
//! The plan's §9 gate `T(A op B) = T(A) op T(B)` for a similarity T,
//! realized end-to-end through the LANDED `boolean()` entry. T is restricted
//! to transformations that keep every consumed carrier in its landed cell:
//! rotation about z + translation (planes and lines stay BARE) and uniform
//! dyadic scale.
//!
//! SPEC_GAP NOTE (see the worktree QUESTION.md): the packet's test 10
//! (`transform_oblique_extrude_metamorphic`) requires the fold to process
//! the oblique-extruded full-circle disk, whose self-loop circle edges abort
//! the landed `Mapped` machinery's debug assertion (`Edge::debug_new`).
//! Every cylinder-walled solid in the tree carries such self-loops, so test
//! 10 is omitted and reported as a SPEC_GAP.

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

use std::f64::consts::FRAC_PI_2;
use truck_base::evidence::Budget;
use truck_geometry::arrange::{arrange, Arrangement};
use truck_geometry::canonical::{Curve, Surface};
use truck_geometry::prelude::*;
use truck_modeling::cad::{rotate_solid, translate_solid, uniform_scale_solid};
use truck_modeling::extrude::extrude_profile;
use truck_shapeops::boolean::assemble::boolean;
use truck_shapeops::boolean::BoolOp;
use truck_topology::Solid;

/// The metamorphic similarity T = Rz(π/2) + translation (z-neutral).
fn apply_t(solid: &Solid<Point3, Curve, Surface>, t: Vector3) -> Solid<Point3, Curve, Surface> {
    let rotated = rotate_solid(
        solid,
        Point3::new(0.0, 0.0, 0.0),
        Vector3::unit_z(),
        FRAC_PI_2,
    )
    .expect("the rotation assembles")
    .value;
    translate_solid(&rotated, t)
        .expect("the translation assembles")
        .value
}

/// The 4x4 block profile.
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

/// The `[x0, x1] x [y0, y1]` box profile.
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

/// The `height`-extrude of a profile.
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
            other => panic!("unexpected carrier {other:?}"),
        })
        .collect();
    kinds.sort();
    kinds
}

/// 7. The Union metamorphic: T = Rz(π/2) + translation on two boxes (the
///    small box straddling the big one's top boundary, the resew convention):
///    `boolean(T(A), T(B), Union)` vs `T(boolean(A, B, Union))` — census,
///    carrier kinds, and vertex points agree.
#[test]
fn transform_union_metamorphic() {
    let t = Vector3::new(1.0, 2.0, 0.0);
    let (pa, aa) = block_profile();
    let a = extrude_solid(&pa, &aa, 2.0);
    let b = box_solid(1.0, 1.0, 1.5, 3.0, 3.0, 3.5);

    let mut budget_ab = Budget::new(1000, 1000, 1000);
    let ab_union = boolean(&a, BoolOp::Union, &b, &mut budget_ab)
        .expect("A union B assembles")
        .value;
    let t_of_ab = apply_t(&ab_union, t);

    let ta = apply_t(&a, t);
    let tb = apply_t(&b, t);
    let mut budget_t = Budget::new(1000, 1000, 1000);
    let t_union = boolean(&ta, BoolOp::Union, &tb, &mut budget_t)
        .expect("T(A) union T(B) assembles")
        .value;

    assert_eq!(
        t_union.face_iter().count(),
        t_of_ab.face_iter().count(),
        "face count"
    );
    assert_eq!(
        t_union.edge_iter().count(),
        t_of_ab.edge_iter().count(),
        "edge count"
    );
    assert_eq!(
        t_union.vertex_iter().count(),
        t_of_ab.vertex_iter().count(),
        "vertex count"
    );
    assert_eq!(
        carrier_kinds(&t_union),
        carrier_kinds(&t_of_ab),
        "carrier kinds"
    );
    // Measured f64-exact (achieved precision 0): the boolean's vertex
    // arithmetic is exactly equivariant under this dyadic fixture + T.
    assert_eq!(
        unique_sorted_points(&t_union),
        unique_sorted_points(&t_of_ab),
        "T(A) ∪ T(B) and T(A ∪ B) share the exact vertex set"
    );
}

/// 8. The Difference metamorphic: the same T, Difference.
#[test]
fn transform_difference_metamorphic() {
    let t = Vector3::new(1.0, 2.0, 0.0);
    let (pa, aa) = block_profile();
    let a = extrude_solid(&pa, &aa, 2.0);
    let b = box_solid(1.0, 1.0, 1.5, 3.0, 3.0, 3.5);

    let mut budget_ab = Budget::new(1000, 1000, 1000);
    let ab_diff = boolean(&a, BoolOp::Difference, &b, &mut budget_ab)
        .expect("A minus B assembles")
        .value;
    let t_of_ab = apply_t(&ab_diff, t);

    let ta = apply_t(&a, t);
    let tb = apply_t(&b, t);
    let mut budget_t = Budget::new(1000, 1000, 1000);
    let t_diff = boolean(&ta, BoolOp::Difference, &tb, &mut budget_t)
        .expect("T(A) minus T(B) assembles")
        .value;

    assert_eq!(
        t_diff.face_iter().count(),
        t_of_ab.face_iter().count(),
        "face count"
    );
    assert_eq!(
        t_diff.edge_iter().count(),
        t_of_ab.edge_iter().count(),
        "edge count"
    );
    assert_eq!(
        t_diff.vertex_iter().count(),
        t_of_ab.vertex_iter().count(),
        "vertex count"
    );
    assert_eq!(
        carrier_kinds(&t_diff),
        carrier_kinds(&t_of_ab),
        "carrier kinds"
    );
    // Measured f64-exact (achieved precision 0): the boolean's vertex
    // arithmetic is exactly equivariant under this dyadic fixture + T.
    assert_eq!(
        unique_sorted_points(&t_diff),
        unique_sorted_points(&t_of_ab),
        "T(A) − T(B) and T(A − B) share the exact vertex set"
    );
}

/// 9. The scale metamorphic: uniform scale 2 — `T(A − B)` vs `T(A) − T(B)`
///    (dyadic scale keeps everything dyadic, so the vertex agreement is
///    f64-exact).
#[test]
fn transform_scale_metamorphic() {
    let (pa, aa) = block_profile();
    let a = extrude_solid(&pa, &aa, 2.0);
    let b = box_solid(1.0, 1.0, 1.5, 3.0, 3.0, 3.5);

    let mut budget_ab = Budget::new(1000, 1000, 1000);
    let ab_diff = boolean(&a, BoolOp::Difference, &b, &mut budget_ab)
        .expect("A minus B assembles")
        .value;
    let t_of_ab = uniform_scale_solid(&ab_diff, 2.0)
        .expect("scale assembles")
        .value;

    let ta = uniform_scale_solid(&a, 2.0).expect("scale assembles").value;
    let tb = uniform_scale_solid(&b, 2.0).expect("scale assembles").value;
    let mut budget_t = Budget::new(1000, 1000, 1000);
    let t_diff = boolean(&ta, BoolOp::Difference, &tb, &mut budget_t)
        .expect("T(A) minus T(B) assembles")
        .value;

    assert_eq!(
        t_diff.face_iter().count(),
        t_of_ab.face_iter().count(),
        "face count"
    );
    assert_eq!(
        t_diff.edge_iter().count(),
        t_of_ab.edge_iter().count(),
        "edge count"
    );
    assert_eq!(
        t_diff.vertex_iter().count(),
        t_of_ab.vertex_iter().count(),
        "vertex count"
    );
    assert_eq!(
        carrier_kinds(&t_diff),
        carrier_kinds(&t_of_ab),
        "carrier kinds"
    );
    assert_eq!(
        unique_sorted_points(&t_diff),
        unique_sorted_points(&t_of_ab),
        "the dyadic scale metamorphic is f64-exact"
    );
}
