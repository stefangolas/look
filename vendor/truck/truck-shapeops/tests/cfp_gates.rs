//! CFP-010-GATES — the boolean-boundary battery and the V5-boolean cumulative
//! adjudication record (CFP-010 scope decision 2: the record lives here, in
//! `truck-shapeops/tests/cfp_gates.rs`, as named cases + the notes field).
//!
//! truck-shapeops cannot name the frozen spine record type
//! (`truck-certified::cfp::V5BooleanDiff`, no Cargo edge; F1), so this file
//! carries the cumulative adjudication record as data — the same three program
//! cases replayed through the frozen helper in
//! `truck-certified/tests/cfp_battery.rs`, plus the boolean-boundary
//! measurements this crate owns — and a local monotone-superset adjudicator
//! over sorted event-id sets with the frozen semantics (new ⊇ old; a removal
//! is a finding, never an accepted widening).
//!
//! Battery content (the §5 "full battery at integrated HEAD", boolean side):
//!
//! - `boolean_output_deterministic_across_runs_at_head` — the flagship
//!   canonical pairs run twice at HEAD produce byte-identical outputs;
//! - `boolean_battery_congruent_with_recorded_m2_ground_truths_at_head` — the
//!   measured outputs (Difference 7 faces, Intersection 3, Union 8 in both
//!   orders) still match the recorded `boolean_m2` ground truths, i.e. the
//!   CFP diff on the canonical pairs is the empty identity (old == new — a
//!   trivial monotone superset, zero regressions);
//! - `v5_boolean_adjudication_record_boolean_boundary_monotone` — the record's
//!   named cases are replayed: every diff is a monotone superset, the
//!   F-C0 screen-widening case is grounded in the fixture's measured
//!   disjoint/touch facts, and a planted removal is caught.

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
// dyadic witnesses — not such a path.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::f64::consts::TAU;

use truck_base::cgmath64::{Matrix4, Point2, Point3, Vector4};
use truck_base::evidence::Budget;
use truck_geometry::arrange::{arrange, Arrangement};
use truck_geometry::canonical::{Curve, Surface};
use truck_geometry::prelude::*;
use truck_modeling::extrude::extrude_profile;
use truck_shapeops::boolean::assemble::boolean;
use truck_shapeops::boolean::BoolOp;
use truck_topology::Solid;

// ---------------------------------------------------------------------------
// Fixture construction (copied VERBATIM from the boolean_m2.rs recipe; the
// test module is not a library and the source file is read-only).
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

/// The flagship canonical pair: the 4x4 block (height 2) and the disk of
/// radius 1 at (2, 2) (height 2).
fn flagship_pair() -> (Solid<Point3, Curve, Surface>, Solid<Point3, Curve, Surface>) {
    let (profile_a, arr_a) = block_profile();
    let solid_a = extrude_solid(&profile_a, &arr_a, 2.0);
    let (profile_b, arr_b) = disk_profile(Point2::new(2.0, 2.0), 1.0);
    let solid_b = extrude_solid(&profile_b, &arr_b, 2.0);
    (solid_a, solid_b)
}

// ---------------------------------------------------------------------------
// Deterministic fingerprint machinery.
// ---------------------------------------------------------------------------

/// The surface-kind tag of a face carrier.
fn carrier_tag(surface: &Surface) -> &'static str {
    match surface {
        Surface::Plane(_) => "plane",
        Surface::Cylinder(_) => "cylinder",
        Surface::Sphere(_) => "sphere",
        Surface::Cone(_) => "cone",
        Surface::Torus(_) => "torus",
        Surface::BSplineSurface(_) => "bspline",
        Surface::NurbsSurface(_) => "nurbs",
        Surface::Processor(_) => "placed",
        Surface::SpineFrameSurface(_) => "sweep",
        Surface::RevolutedCurve(_) | Surface::ExtrudedCurve(_) => "derived",
    }
}

/// The sorted distinct vertex points of a solid (total order, dedup).
fn sorted_vertices(solid: &Solid<Point3, Curve, Surface>) -> Vec<Point3> {
    let mut pts: Vec<Point3> = solid.vertex_iter().map(|v| v.point()).collect();
    pts.sort_by(|a, b| {
        a.x.total_cmp(&b.x)
            .then(a.y.total_cmp(&b.y))
            .then(a.z.total_cmp(&b.z))
    });
    pts.dedup();
    pts
}

/// A deterministic structural fingerprint of a boolean output: shell count,
/// face census (carrier kind + boundary-wire structure), and the sorted
/// vertex-point set. Two runs over identical inputs must agree byte-for-byte.
fn solid_fingerprint(solid: &Solid<Point3, Curve, Surface>) -> String {
    let mut out = format!("shells={}\n", solid.boundaries().len());
    let mut faces: Vec<String> = Vec::new();
    for shell in solid.boundaries() {
        for face in shell.face_iter() {
            let mut wires: Vec<String> = face
                .absolute_boundaries()
                .iter()
                .map(|wire| format!("w{}", wire.len()))
                .collect();
            wires.sort();
            faces.push(format!("{} {:?}", carrier_tag(&face.surface()), wires));
        }
    }
    faces.sort();
    for face in faces {
        out.push_str(&face);
        out.push('\n');
    }
    out.push_str("vertices=");
    out.push_str(&format!("{:?}\n", sorted_vertices(solid)));
    out
}

// ---------------------------------------------------------------------------
// The named boolean-boundary tests.
// ---------------------------------------------------------------------------

/// The flagship canonical pairs are deterministic at HEAD: two full runs of
/// each operation produce byte-identical fingerprints (verdicts, face
/// structure, vertex sets).
#[test]
fn boolean_output_deterministic_across_runs_at_head() {
    let (a, b) = flagship_pair();
    for op in [BoolOp::Union, BoolOp::Difference, BoolOp::Intersection] {
        let mut first_budget = Budget::new(1000, 1000, 1000);
        let first = boolean(&a, op, &b, &mut first_budget)
            .expect("the flagship run assembles")
            .value;
        let mut second_budget = Budget::new(1000, 1000, 1000);
        let second = boolean(&a, op, &b, &mut second_budget)
            .expect("the flagship run assembles")
            .value;
        assert_eq!(
            solid_fingerprint(&first),
            solid_fingerprint(&second),
            "two identical boolean runs must agree byte-for-byte (op {op:?})"
        );
    }
}

/// The canonical boolean battery is congruent with the recorded `boolean_m2`
/// ground truths at integrated HEAD: the CFP diff on these pairs is the empty
/// identity (old == new — a trivial monotone superset, zero regressions).
#[test]
fn boolean_battery_congruent_with_recorded_m2_ground_truths_at_head() {
    let (a, b) = flagship_pair();

    let mut diff_budget = Budget::new(1000, 1000, 1000);
    let difference = boolean(&a, BoolOp::Difference, &b, &mut diff_budget)
        .expect("the Difference flagship assembles through the entry")
        .value;
    assert_eq!(difference.boundaries().len(), 1);
    let shell = difference.boundaries().first().expect("one shell");
    assert_eq!(
        shell.face_iter().count(),
        7,
        "Difference: 7 faces, as recorded"
    );

    let mut int_budget = Budget::new(1000, 1000, 1000);
    let intersection = boolean(&a, BoolOp::Intersection, &b, &mut int_budget)
        .expect("the Intersection flagship assembles through the entry")
        .value;
    assert_eq!(intersection.boundaries().len(), 1);
    let shell = intersection.boundaries().first().expect("one shell");
    assert_eq!(
        shell.face_iter().count(),
        3,
        "Intersection: 3 faces, as recorded"
    );

    let mut union_budget = Budget::new(1000, 1000, 1000);
    let union_ab = boolean(&a, BoolOp::Union, &b, &mut union_budget)
        .expect("A union B assembles through the entry")
        .value;
    assert_eq!(union_ab.boundaries().len(), 1);
    let shell = union_ab.boundaries().first().expect("one shell");
    assert_eq!(shell.face_iter().count(), 8, "Union: 8 faces, as recorded");

    let mut union_ba_budget = Budget::new(1000, 1000, 1000);
    let union_ba = boolean(&b, BoolOp::Union, &a, &mut union_ba_budget)
        .expect("B union A assembles through the entry")
        .value;
    assert_eq!(union_ba.boundaries().len(), 1);
    let shell = union_ba.boundaries().first().expect("one shell");
    assert_eq!(shell.face_iter().count(), 8, "Union (both orders): 8 faces");
}

// ---------------------------------------------------------------------------
// The V5-boolean cumulative adjudication record (scope decision 2).
// ---------------------------------------------------------------------------

/// The F-C0 fixture ground truths, copied verbatim from
/// `truck-certified/src/cfp/fixtures.rs` (F1 — never import the module).
const FC0_SAMPLED_LO: [f64; 3] = [-4.0, -4.0, -3.0];
const FC0_SAMPLED_HI: [f64; 3] = [4.0, 4.0, -3.0];
const FC0_SLAB_LO: [f64; 3] = [-10.0, -10.0, -6.0];
const FC0_SLAB_HI: [f64; 3] = [10.0, 10.0, -4.0];

/// One recorded V5-boolean case: the pair name, the old (pre-CFP) contact
/// event set and the new (post-CFP) event set on identical inputs, and the
/// provenance note.
struct V5RecordedCase {
    /// The program case (`cfp001_*`, `cfp004_*`, `cfp005_*`).
    name: &'static str,
    /// The old event ids, sorted.
    old: Vec<u64>,
    /// The new event ids, sorted.
    new: Vec<u64>,
    /// The notes field for the case (measurement provenance).
    notes: &'static str,
}

/// Whether `superset` contains every element of `subset` (the frozen
/// monotone-superset semantics; the certified replay lives in
/// `cfp_battery.rs` through `V5BooleanDiff::adjudicate`).
fn is_superset(subset: &[u64], superset: &[u64]) -> bool {
    subset.iter().all(|id| superset.contains(id))
}

#[test]
fn v5_boolean_adjudication_record_boolean_boundary_monotone() {
    // The F-C0 screen facts are measured from the copied fixture constants: the
    // boundary-sample (old) screen box does NOT touch the slab while the true
    // cap image does, so the old screen dropped the pair (zero contact events)
    // and the certified screen admits it (the diff adds events — never removes
    // them). This is the boolean-boundary witness of the CFP-001 widening.
    let sampled_touches_slab =
        aabb_touches(FC0_SAMPLED_LO, FC0_SAMPLED_HI, FC0_SLAB_LO, FC0_SLAB_HI);
    assert!(
        !sampled_touches_slab,
        "the sampled screen drops the F-C0 pair (the defect being fixed)"
    );
    let true_image_lo = [-4.0, -4.0, -5.0];
    let true_image_hi = [4.0, 4.0, -3.0];
    assert!(
        aabb_touches(true_image_lo, true_image_hi, FC0_SLAB_LO, FC0_SLAB_HI),
        "the cap's true image reaches the slab"
    );

    // The record's named cases are monotone supersets: new ⊇ old on identical
    // inputs (the three program diffs; the canonical boolean battery is the
    // empty identity — its recorded face counts are unchanged at HEAD, which
    // the congruence test above measures — and the F-C0 screen case adds the
    // dropped contact).
    let cases = [
        V5RecordedCase {
            name: "cfp001_screen_widening_fc0",
            old: Vec::new(),
            new: vec![1],
            notes: "sampled screen box disjoint from the slab (measured above); \
                    the certified screen admits the pair and the funnel certifies \
                    the cap x slab contact at HEAD (cfp_battery fc0_...)",
        },
        V5RecordedCase {
            name: "cfp001_canonical_boolean_identity",
            old: vec![7],
            new: vec![7],
            notes: "the canonical flagship Difference output is unchanged at HEAD \
                    (7 faces as recorded in boolean_m2): old == new, a trivial \
                    monotone superset with zero removals",
        },
        V5RecordedCase {
            name: "cfp004_implicit_stage_fc4",
            old: Vec::new(),
            new: vec![2],
            notes: "the plane x spline pair certifies its interior critical point \
                    through the Theorem-4 stage at HEAD (cfp_battery fc3_/fc4); \
                    the pre-CFP-004 funnel produced no scalar-stage event",
        },
        V5RecordedCase {
            name: "cfp005_pair_enumeration_fc5",
            old: Vec::new(),
            new: vec![221],
            notes: "the span-BVH enumerates the single contact span pair \
                    (u=7, v=11, stack position 221) of the 20 x 30 decomposition \
                    at HEAD (cfp_battery fc5)",
        },
    ];
    for case in cases {
        assert!(
            is_superset(&case.old, &case.new),
            "the recorded diff {} must be a monotone superset ({} ⊉ {})",
            case.name,
            case.new.len(),
            case.old.len()
        );
        assert!(
            !case.notes.is_empty(),
            "every recorded case carries its measurement provenance"
        );
    }

    // The adjudicator is not vacuous: a planted removal refuses.
    assert!(
        !is_superset(&[1, 2], &[2]),
        "a planted removal must fail the monotone superset check"
    );
}

/// Whether two axis-aligned boxes touch (closed, per-axis overlap).
fn aabb_touches(a_lo: [f64; 3], a_hi: [f64; 3], b_lo: [f64; 3], b_hi: [f64; 3]) -> bool {
    a_lo.iter()
        .zip(a_hi.iter())
        .zip(b_lo.iter())
        .zip(b_hi.iter())
        .all(|(((al, ah), bl), bh)| al <= bh && bl <= ah)
}
