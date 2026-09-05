//! CL-005-EXACT-CONTACT — certified decisions for the MEASURED exact-contact
//! cases (the R2 rescope: a pure battery, no production behavior change).
//!
//! The funnel defers two MEASURED exact-contact classes (the loop traps): the
//! coplanar full-face butt-join union and the exact-footprint halfspace. The R2
//! amendment (2026-09-05, `CL-005-STOP-QUESTION.md` at commit `4d59d5b`)
//! rescoped this packet: neither class is derivable at the DECIDE/ASSEMBLE
//! boundary this packet's write set owns (the choke points live in
//! `split.rs`/`classify.rs`, both frozen here). So this file certifies what the
//! boundary CAN certify:
//!
//! - the families that ALREADY certify at HEAD (z-axis full-face union 10-face,
//!   P3 `(S−pad) ∪ (S∩pad)` recombination 10-face, padded over-box controls
//!   6-face, M2 flagship 7/8-face), pinned as a regression battery with the
//!   certified decision predicates documented, and
//! - the two RESCOPED classes carrying their TYPED refusals (never a silent
//!   wrong output): the x/y-axis full-face butt joins refuse inside the
//!   splitter (`split.rs::finish`, a `Region2 CoincidentInterval` between
//!   vertical side faces), and the exact-footprint halfspace refuses
//!   `Contradictory(FragmentInsideOther)` in the classifier.
//!
//! The certified decision predicates behind each certified case (all the §13.1
//! material-state primitive of `boolean::fragment_decision`, never a tolerance
//! comparison):
//!
//! - z-axis full-face butt join: the two coincident caps at the seam plane are
//!   a `CoincidentOrientation::Anti` pair; `fragment_decision(Union, anti)` is
//!   `Discard` on both members (each side of the seam is material to the
//!   union), so the interior caps vanish. Each solid's four side walls are
//!   exterior non-paired fragments; `fragment_decision(Union, exterior)` keeps
//!   each unflipped. The result is the merged box cosmetically split at the
//!   seam: 2 caps + 8 side walls = 10 faces.
//! - P3 recombination: each half assembles through the same exterior-keep /
//!   interior-discard rule; the recombination union is the same 10-face
//!   cosmetically-split merged box (the boundary emits no coplanar-face
//!   merging, so the 10-face count is the certified answer, not a 6-face box).
//! - padded over-box controls: a strictly-interior containment with NO
//!   coplanar wall pairs (the pad is offset from the solid's footprint, so
//!   every wall is transverse), Difference/Intersection each assemble the
//!   6-face half. The Region2 `Coincident` containment splits are load-bearing
//!   for this strictly-interior class (recorded evidence: deleting them changes
//!   the M2 flagship outputs — Difference 9 instead of 7, Union 6 instead of
//!   8); they are harmful only for the boundary-touching coplanar class, which
//!   is exactly what CTE-007 builds against.
//! - M2 flagship: the disk's caps are `CoincidentOrientation::Identical` with
//!   the block's; the union keeps the block's cap as an annulus + the disk's as
//!   a disk (8 faces), the Difference assembles the 7-face plate-with-hole.

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

use std::collections::BTreeMap;
use std::f64::consts::TAU;
use truck_base::cgmath64::{Point2, Point3, Vector3};
use truck_base::evidence::{Budget, EnvelopeCase, Outcome, Prop, Refusal};
use truck_geometry::arrange::{arrange, Arrangement};
use truck_geometry::canonical::{Curve, Surface};
use truck_geometry::prelude::*;
use truck_modeling::cad::{solid_bounding_box, translate_solid};
use truck_modeling::extrude::extrude_profile;
use truck_shapeops::boolean::assemble::boolean;
use truck_shapeops::boolean::BoolOp;
use truck_topology::Solid;

/// The insertion tolerance class for the fixture comparisons (H-3:
/// dimensionless relative to the unit-scale witnesses; dyadic geometry decides
/// exactly).
const TOL: f64 = 1.0e-2; // H-3: tolerance class for insertion geometry

// ---------------------------------------------------------------------------
// construction helpers (the boolean_m2 / resew / split_plane conventions:
// dyadic `extrude_profile` + `translate_solid` witnesses, in-crate and proven)
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

/// The flagship plate `[0, 4]^2 x [0, h]` from the 4x4 block profile.
fn flagship_plate(h: f64) -> Solid<Point3, Curve, Surface> {
    let (profile, arr) = block_profile();
    extrude_solid(&profile, &arr, h)
}

/// Runs one boolean through the certified entry with a fresh budget.
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

/// Asserts `solid` assembles as exactly one closed shell and returns the shell
/// face count.
fn assert_single_shell(solid: &Solid<Point3, Curve, Surface>) -> usize {
    assert_eq!(solid.boundaries().len(), 1, "exactly one shell");
    solid
        .boundaries()
        .first()
        .expect("one shell")
        .face_iter()
        .count()
}

/// Asserts two solids share the same bounding box within `TOL`.
fn assert_same_box(a: &Solid<Point3, Curve, Surface>, b: &Solid<Point3, Curve, Surface>) {
    let (amin, amax) = bounding_box(a);
    let (bmin, bmax) = bounding_box(b);
    for (x, y) in [
        (amin.x, bmin.x),
        (amin.y, bmin.y),
        (amin.z, bmin.z),
        (amax.x, bmax.x),
        (amax.y, bmax.y),
        (amax.z, bmax.z),
    ] {
        assert!(
            (x - y).abs() < TOL,
            "bounding-box corners must match: ({amin:?}..{amax:?}) vs ({bmin:?}..{bmax:?})"
        );
    }
}

/// The `(axis, coord)` of a plane face: `axis` is the constant axis (0=x,
/// 1=y, 2=z) and `coord` its constant coordinate.
fn plane_axis_coord(face: &truck_topology::Face<Point3, Curve, Surface>) -> Option<(usize, f64)> {
    let Surface::Plane(plane) = face.surface() else {
        return None;
    };
    let n = plane.normal();
    let axis = if n.x.abs() > 0.5 {
        0
    } else if n.y.abs() > 0.5 {
        1
    } else {
        2
    };
    let coord = match axis {
        0 => plane.origin().x,
        1 => plane.origin().y,
        _ => plane.origin().z,
    };
    Some((axis, coord))
}

/// The face census of a shell's plane faces keyed by `(axis, coord)`; a
/// coordinate is quantized to its nearest 1e-6 (the dyadic witnesses sit on
/// exact small integers, and `-0.0` must land on `0.0`).
fn plane_census(
    shell: &truck_topology::Shell<Point3, Curve, Surface>,
) -> Vec<((usize, i64), usize)> {
    let mut counts: BTreeMap<(usize, i64), usize> = BTreeMap::new();
    for face in shell.face_iter() {
        if let Some((axis, coord)) = plane_axis_coord(&face) {
            let key = (axis, (coord * 1.0e6).round() as i64);
            *counts.entry(key).or_insert(0) += 1;
        }
    }
    counts.into_iter().collect()
}

/// The scaled integer coordinate of `v` (for census lookups).
fn q(v: f64) -> i64 {
    (v * 1.0e6).round() as i64
}

/// Asserts the recorded structure of the 10-face merged box `[0, 4]^2 x [0, 2]`
/// cosmetically split at the z = 1 seam: the two caps at z = 0 and z = 2 are
/// whole, each of the four side-wall planes carries exactly two faces, and
/// nothing remains on the interior seam plane z = 1 (the coincident `Anti`
/// caps were decided `Discard`).
fn assert_merged_10_face_box(shell: &truck_topology::Shell<Point3, Curve, Surface>, what: &str) {
    let census = plane_census(shell);
    assert_eq!(shell.face_iter().count(), 10, "{what}: 10 faces");
    let get = |axis: usize, c: i64| -> usize {
        census
            .iter()
            .find(|((a, cc), _)| *a == axis && *cc == c)
            .map(|(_, n)| *n)
            .unwrap_or(0)
    };
    assert_eq!(get(2, q(0.0)), 1, "{what}: one cap at z = 0");
    assert_eq!(get(2, q(2.0)), 1, "{what}: one cap at z = 2");
    assert_eq!(
        get(2, q(1.0)),
        0,
        "{what}: the seam caps at z = 1 were discarded"
    );
    for c in [0.0, 4.0] {
        assert_eq!(
            get(0, q(c)),
            2,
            "{what}: two faces on each x = {c} wall plane"
        );
        assert_eq!(
            get(1, q(c)),
            2,
            "{what}: two faces on each y = {c} wall plane"
        );
    }
    let total: usize = census.iter().map(|(_, n)| *n).sum();
    assert_eq!(total, 10, "{what}: every face is a plane face");
}

// ---------------------------------------------------------------------------
// the deterministic content digest (the V5 bit-identity record)
// ---------------------------------------------------------------------------

/// FNV-1a over the canonical byte record: deterministic (no randomized state),
/// so equal digests across two runs assert bit-identical output.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn push_f64(out: &mut Vec<u8>, v: f64) {
    out.extend_from_slice(&v.to_bits().to_le_bytes());
}

fn push_point(out: &mut Vec<u8>, p: Point3) {
    push_f64(out, p.x);
    push_f64(out, p.y);
    push_f64(out, p.z);
}

/// The deterministic content bytes of one solid: every face's surface carrier,
/// orientation and boundary geometry, in `face_iter()` order.
fn solid_bytes(solid: &Solid<Point3, Curve, Surface>) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    for shell in solid.boundaries() {
        for face in shell.face_iter() {
            out.push(if face.orientation() { 1 } else { 0 });
            match face.surface() {
                Surface::Plane(plane) => {
                    out.push(b'P');
                    push_point(&mut out, plane.origin());
                    let n = plane.normal();
                    push_f64(&mut out, n.x);
                    push_f64(&mut out, n.y);
                    push_f64(&mut out, n.z);
                }
                Surface::Cylinder(cyl) => {
                    out.push(b'C');
                    push_point(&mut out, cyl.center());
                    push_f64(&mut out, cyl.radius());
                }
                other => out.extend_from_slice(format!("{other:?}").as_bytes()),
            }
            for wire in face.absolute_boundaries() {
                out.push(b'w');
                for edge in wire.edge_iter() {
                    out.push(b'e');
                    let curve = edge.curve();
                    let (t0, t1) = curve.range_tuple();
                    push_point(&mut out, curve.subs(t0));
                    push_point(&mut out, curve.subs(t1));
                    out.extend_from_slice(format!("{curve:?}").as_bytes());
                }
            }
        }
    }
    out
}

/// The FNV-1a content digest of a solid.
fn digest_solid(solid: &Solid<Point3, Curve, Surface>) -> u64 {
    fnv1a(&solid_bytes(solid))
}

/// Runs the boolean twice through the certified entry and asserts both outputs
/// are bit-identical (equal content digests). Returns the digest.
fn run_bit_identical(
    a: &Solid<Point3, Curve, Surface>,
    op: BoolOp,
    b: &Solid<Point3, Curve, Surface>,
    what: &str,
) -> u64 {
    let first = certify(a, op, b, what);
    let second = certify(a, op, b, what);
    let d1 = digest_solid(&first);
    let d2 = digest_solid(&second);
    assert_eq!(d1, d2, "{what}: the two runs must be bit-identical");
    d1
}

/// Runs one boolean through the certified entry and returns the solid; a
/// refusal is a test failure naming the fixture.
fn certify(
    a: &Solid<Point3, Curve, Surface>,
    op: BoolOp,
    b: &Solid<Point3, Curve, Surface>,
    what: &str,
) -> Solid<Point3, Curve, Surface> {
    match run(a, op, b) {
        Ok(cert) => cert.value,
        Err(e) => panic!("{what}: must certify through the entry, got {e:?}"),
    }
}

// ---------------------------------------------------------------------------
// Test 1: the z-axis full-face butt-join family certifies (regression pin).
// ---------------------------------------------------------------------------

#[test]
fn z_axis_full_face_butt_join_certified() {
    // Direct z-stacked full-face union (the resew test-1 class): the two boxes
    // `[0,4]^2 x [0,1]` and `[0,4]^2 x [1,2]` share the ENTIRE z = 1 cap. The
    // seam is a Region2 `CoincidentInterval` between two caps and splits fine
    // (the z-adjacent twin of the x/y-axis case, which refuses — Test 2). The
    // coincident cap pair is `Anti`-oriented; `fragment_decision(Union, anti)`
    // discards both interior caps, and the four exterior walls of each box are
    // kept unflipped. Recorded 10-face result: 2 caps + 8 side walls.
    let a = box_solid(0.0, 0.0, 0.0, 4.0, 4.0, 1.0);
    let b = box_solid(0.0, 0.0, 1.0, 4.0, 4.0, 2.0);
    let union = certify(&a, BoolOp::Union, &b, "the z-axis full-face union");
    let faces = assert_single_shell(&union);
    assert_eq!(
        faces, 10,
        "the z-axis butt-join union keeps its 10-face record"
    );
    let merged = box_solid(0.0, 0.0, 0.0, 4.0, 4.0, 2.0);
    assert_same_box(&union, &merged);
    assert_merged_10_face_box(
        union.boundaries().first().expect("one shell"),
        "z-axis full-face union",
    );

    // P3 recombination `(S − pad) ∪ (S ∩ pad)`: split the flagship plate
    // `[0,4]^2 x [0,2]` at z = 1 by the padded over-box halfspace (P3's D3/D4
    // recipe, walls OFFSET from the plate's so no wall is coplanar), then
    // re-union the halves. Each half assembles through the same certified
    // decision path (exterior-keep / interior-discard); the recombination is
    // the same cosmetically-split 10-face merged box with S's exact bounding
    // box.
    let s = flagship_plate(2.0);
    let minus_box = box_solid(-1.0, -1.0, -1.0, 5.0, 5.0, 1.0);

    let minus = certify(&s, BoolOp::Intersection, &minus_box, "the S ∩ pad half");
    let plus = certify(&s, BoolOp::Difference, &minus_box, "the S − pad half");
    assert_eq!(
        assert_single_shell(&minus),
        6,
        "the S ∩ pad half is 6 faces"
    );
    assert_eq!(assert_single_shell(&plus), 6, "the S − pad half is 6 faces");

    let recombined = certify(
        &plus,
        BoolOp::Union,
        &minus,
        "the (S−pad) ∪ (S∩pad) recombination",
    );
    assert_eq!(
        assert_single_shell(&recombined),
        10,
        "the P3 recombination keeps its recorded 10-face structure"
    );
    assert_same_box(&recombined, &s);
    assert_merged_10_face_box(
        recombined.boundaries().first().expect("one shell"),
        "P3 recombination union",
    );
}

// ---------------------------------------------------------------------------
// Test 2: the two rescoped classes carry their typed refusals (never a silent
// wrong output).
// ---------------------------------------------------------------------------

#[test]
fn rescoped_classes_carry_typed_refusals() {
    // Case (a), REBOOKED to CTE-007: the x-/y-axis full-face butt joins. Two
    // 2-cubes sharing an exact vertical face refuse INSIDE the splitter
    // (`split.rs::finish`, a Region2 `CoincidentInterval` between vertical side
    // faces) — the refusal never reaches DECIDE, so no certified answer exists
    // at this boundary. A silent `Ok` on either is a failure.
    let cube = box_solid(0.0, 0.0, 0.0, 2.0, 2.0, 2.0);
    let xfull = box_solid(2.0, 0.0, 0.0, 4.0, 2.0, 2.0);
    let yfull = box_solid(0.0, 2.0, 0.0, 2.0, 4.0, 2.0);
    for (other, what) in [
        (&xfull, "x-axis full-face butt join"),
        (&yfull, "y-axis full-face butt join"),
    ] {
        let out = run(&cube, BoolOp::Union, other);
        assert!(
            matches!(
                out,
                Err(Refusal::UnsupportedEnvelope(
                    EnvelopeCase::ContactReductionDeferred
                ))
            ),
            "{what} must refuse the deferred envelope inside the splitter, got {out:?}"
        );
    }

    // Case (b), DROPPED per §5.9: the exact-footprint halfspace box. C is the
    // plate's exact-footprint lower half `[0,4]^2 x [0,1]` — its walls are
    // coplanar with the plate's over z ∈ [0, 1]. The splitter succeeds (a
    // 20-fragment mesh, 5 coincident pairs) but the mesh is not a proper
    // tiling of the coplanar container faces, and the classifier refuses
    // `Contradictory(FragmentInsideOther)`. A typed and recorded refusal is
    // SUCCESS under §5.9; a silent `Ok` or a `NumericallyUnresolved` on any op
    // is a failure.
    let s = flagship_plate(2.0);
    let c = box_solid(0.0, 0.0, 0.0, 4.0, 4.0, 1.0);
    for (op, what) in [
        (BoolOp::Difference, "exact-footprint halfspace Difference"),
        (
            BoolOp::Intersection,
            "exact-footprint halfspace Intersection",
        ),
        (BoolOp::Union, "exact-footprint halfspace Union"),
    ] {
        let out = run(&s, op, &c);
        let contradictory = matches!(
            out,
            Err(Refusal::Contradictory(ref witness)) if witness.prop == Prop::FragmentInsideOther
        );
        assert!(
            contradictory,
            "{what} must refuse Contradictory(FragmentInsideOther), got {out:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Test 3: the canonical controls certify bit-identically (V5 identity guard).
// ---------------------------------------------------------------------------

#[test]
fn canonical_controls_bit_identical() {
    // boolean_m2's fixture set through the same certified entry: the 4x4 block
    // and the r = 1 disk at (2, 2), both height 2. The recorded face counts
    // (Difference 7, Intersection 3, Union 8, Xor 7) and the bit-identity of
    // the outputs across two runs.
    let (pa, aa) = block_profile();
    let a = extrude_solid(&pa, &aa, 2.0);
    let (pb, ab) = disk_profile(Point2::new(2.0, 2.0), 1.0);
    let b = extrude_solid(&pb, &ab, 2.0);

    let mut digests: Vec<u64> = Vec::new();

    let diff = run_bit_identical(&a, BoolOp::Difference, &b, "M2 Difference");
    assert_eq!(
        assert_single_shell(&certify(&a, BoolOp::Difference, &b, "M2 Difference")),
        7
    );
    digests.push(diff);

    let inter = run_bit_identical(&a, BoolOp::Intersection, &b, "M2 Intersection");
    assert_eq!(
        assert_single_shell(&certify(&a, BoolOp::Intersection, &b, "M2 Intersection")),
        3
    );
    digests.push(inter);

    let union = run_bit_identical(&a, BoolOp::Union, &b, "M2 Union");
    assert_eq!(
        assert_single_shell(&certify(&a, BoolOp::Union, &b, "M2 Union")),
        8
    );
    digests.push(union);

    // The disk is strictly inside the block, so the M2 Xor result is the same
    // 7-face plate-with-hole as the Difference (A △ B = A − B when B ⊂ A); it
    // is pinned for bit-identity and face count, not for digest distinctness.
    let _xor = run_bit_identical(&a, BoolOp::Xor, &b, "M2 Xor");
    assert_eq!(
        assert_single_shell(&certify(&a, BoolOp::Xor, &b, "M2 Xor")),
        7
    );

    // The padded over-box controls (P3 D3 recipe): S minus / intersect the
    // pad `[-1,5]^2 x [-1,1]`. No wall is coplanar with S's, so the
    // strictly-interior containment certifies; each half is the 6-face box
    // `[0,4]^2 x [1,2]` (Difference) / `[0,4]^2 x [0,1]` (Intersection).
    let s = flagship_plate(2.0);
    let pad = box_solid(-1.0, -1.0, -1.0, 5.0, 5.0, 1.0);

    let pad_diff = run_bit_identical(&s, BoolOp::Difference, &pad, "pad Difference control");
    let diff_solid = certify(&s, BoolOp::Difference, &pad, "pad Difference control");
    assert_eq!(
        assert_single_shell(&diff_solid),
        6,
        "pad Difference is the 6-face upper half"
    );
    let pad_diff_digest = pad_diff;

    let pad_inter = run_bit_identical(&s, BoolOp::Intersection, &pad, "pad Intersection control");
    let inter_solid = certify(&s, BoolOp::Intersection, &pad, "pad Intersection control");
    assert_eq!(
        assert_single_shell(&inter_solid),
        6,
        "pad Intersection is the 6-face lower half"
    );
    let pad_inter_digest = pad_inter;

    // The z-axis full-face union (Test 1's direct fixture) also certifies
    // bit-identically through the entry.
    let z_lo = box_solid(0.0, 0.0, 0.0, 4.0, 4.0, 1.0);
    let z_hi = box_solid(0.0, 0.0, 1.0, 4.0, 4.0, 2.0);
    let z_union = run_bit_identical(&z_lo, BoolOp::Union, &z_hi, "z-axis full-face union");
    assert_eq!(
        assert_single_shell(&certify(
            &z_lo,
            BoolOp::Union,
            &z_hi,
            "z-axis full-face union"
        )),
        10
    );

    // The digest discriminates: no two of these certified controls collapse to
    // one record, and the two 6-face pad halves (upper vs lower) are distinct.
    for (i, d) in digests.iter().enumerate() {
        for (j, other) in digests.iter().enumerate() {
            if i < j {
                assert_ne!(d, other, "M2 control digests must be pairwise distinct");
            }
        }
    }
    assert_ne!(
        pad_diff_digest, pad_inter_digest,
        "the two pad halves must be distinct"
    );
    assert_ne!(
        z_union, union,
        "the z-axis union and the M2 union must be distinct"
    );
}
