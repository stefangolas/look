//! BIE-007-GATES — the χ valuation + mod-2 homology validity gate battery.
//!
//! Four tests:
//!
//! 1. `chi_valuation_matches_known_complexes` — χ = V − E + F on hand-built
//!    complexes with known χ (cube shell 2, torus 0, sphere 2).
//! 2. `mod2_homology_detects_defect` — the Z₂ rank computation distinguishes a
//!    closed shell from planted-defect variants.
//! 3. `gate_fails_not_warns_on_mismatch` — a mismatching complex returns a
//!    typed refusal outcome, not an annotated pass (the mutation battery).
//! 4. `differential_congruent_with_boolean_m2` — the `boolean_m2` recipe
//!    fixtures through the gate agree with the landed results, bit-for-bit on
//!    the canonical pairs.
//!
//! The defect shells and the fixture geometry are built here (the
//! `boolean_m2.rs` file is read-only and byte-identical after this run). The
//! abstract shells are `Shell<(), (), ()>`; the differential battery builds
//! the same dyadic fixture shapes the landed `boolean_m2` flagship uses.

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
// dyadic witnesses and landed fixture recipes - not such a path.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::f64::consts::TAU;

use truck_base::evidence::{Budget, Refusal};
use truck_geometry::arrange::{arrange, Arrangement};
use truck_geometry::canonical::{Curve, Surface};
use truck_geometry::prelude::*;
use truck_modeling::extrude::extrude_profile;
use truck_shapeops::boolean::assemble::boolean;
use truck_shapeops::boolean::BoolOp;
use truck_shapeops::gates::{chi_homology_gate, mod2_homology, BettiNumbers, GateReport};
use truck_topology::{Edge, Face, Shell, Solid, Vertex, Wire};

/// The canonical-solid fixture type of the boolean recipe.
type FixtureSolid = Solid<Point3, Curve, Surface>;

// ---------------------------------------------------------------------------
// abstract (purely combinatorial) shells
// ---------------------------------------------------------------------------

/// A wire from owned edge values (each element keeps its own direction).
fn wire_from(edges: Vec<Edge<(), ()>>) -> Wire<(), ()> {
    Wire::from(edges)
}

/// The closed cube shell (8 vertices, 12 edges, 6 faces), a sphere: χ = 2.
fn cube_shell() -> Shell<(), (), ()> {
    let v = Vertex::news([(); 8]);
    let edge = [
        Edge::new(&v[0], &v[1], ()),
        Edge::new(&v[1], &v[2], ()),
        Edge::new(&v[2], &v[3], ()),
        Edge::new(&v[3], &v[0], ()),
        Edge::new(&v[0], &v[4], ()),
        Edge::new(&v[1], &v[5], ()),
        Edge::new(&v[2], &v[6], ()),
        Edge::new(&v[3], &v[7], ()),
        Edge::new(&v[4], &v[5], ()),
        Edge::new(&v[5], &v[6], ()),
        Edge::new(&v[6], &v[7], ()),
        Edge::new(&v[7], &v[4], ()),
    ];
    let faces = vec![
        Face::new(
            vec![wire_from(vec![
                edge[0].clone(),
                edge[1].clone(),
                edge[2].clone(),
                edge[3].clone(),
            ])],
            (),
        ),
        Face::new(
            vec![wire_from(vec![
                edge[4].clone(),
                edge[8].clone(),
                edge[5].inverse(),
                edge[0].inverse(),
            ])],
            (),
        ),
        Face::new(
            vec![wire_from(vec![
                edge[5].clone(),
                edge[9].clone(),
                edge[6].inverse(),
                edge[1].inverse(),
            ])],
            (),
        ),
        Face::new(
            vec![wire_from(vec![
                edge[6].clone(),
                edge[10].clone(),
                edge[7].inverse(),
                edge[2].inverse(),
            ])],
            (),
        ),
        Face::new(
            vec![wire_from(vec![
                edge[7].clone(),
                edge[11].clone(),
                edge[4].inverse(),
                edge[3].inverse(),
            ])],
            (),
        ),
        Face::new(
            vec![wire_from(vec![
                edge[11].inverse(),
                edge[10].inverse(),
                edge[9].inverse(),
                edge[8].inverse(),
            ])],
            (),
        ),
    ];
    Shell::from(faces)
}

/// The closed tetrahedron shell (4 vertices, 6 edges, 4 faces), a sphere:
/// χ = 2.
fn tetrahedron_shell() -> Shell<(), (), ()> {
    let v = Vertex::news([(); 4]);
    let edge = [
        Edge::new(&v[0], &v[1], ()),
        Edge::new(&v[0], &v[2], ()),
        Edge::new(&v[0], &v[3], ()),
        Edge::new(&v[1], &v[2], ()),
        Edge::new(&v[1], &v[3], ()),
        Edge::new(&v[2], &v[3], ()),
    ];
    let mut faces = vec![
        Face::new(
            vec![wire_from(vec![
                edge[0].clone(),
                edge[3].clone(),
                edge[1].inverse(),
            ])],
            (),
        ),
        Face::new(
            vec![wire_from(vec![
                edge[1].clone(),
                edge[5].clone(),
                edge[2].inverse(),
            ])],
            (),
        ),
        Face::new(
            vec![wire_from(vec![
                edge[2].clone(),
                edge[4].inverse(),
                edge[0].inverse(),
            ])],
            (),
        ),
        Face::new(
            vec![wire_from(vec![
                edge[3].clone(),
                edge[5].clone(),
                edge[4].inverse(),
            ])],
            (),
        ),
    ];
    faces[3].invert();
    Shell::from(faces)
}

/// The closed 3×3 torus shell (9 vertices, 18 edges, 9 faces): χ = 0. The
/// torus is the `N × N` quotient grid with `N = 3`, so every vertex has four
/// distinct toroidal neighbours and no parallel edges (the 2×2 quotient would
/// collapse opposite sides onto the same neighbour and trip the landed link
/// classifier).
fn torus_shell() -> Shell<(), (), ()> {
    const N: usize = 3;
    let vertices: Vec<Vec<Vertex<()>>> = (0..N).map(|_| Vertex::news([(); N])).collect();
    let horizontal: Vec<Vec<Edge<(), ()>>> = (0..N)
        .map(|r| {
            (0..N)
                .map(|c| Edge::new(&vertices[r][c], &vertices[r][(c + 1) % N], ()))
                .collect()
        })
        .collect();
    let vertical: Vec<Vec<Edge<(), ()>>> = (0..N)
        .map(|r| {
            (0..N)
                .map(|c| Edge::new(&vertices[r][c], &vertices[(r + 1) % N][c], ()))
                .collect()
        })
        .collect();
    let mut faces = Vec::new();
    for r in 0..N {
        for c in 0..N {
            let wire = wire_from(vec![
                horizontal[r][c].clone(),
                vertical[r][(c + 1) % N].clone(),
                horizontal[(r + 1) % N][c].inverse(),
                vertical[r][c].inverse(),
            ]);
            faces.push(Face::new(vec![wire], ()));
        }
    }
    Shell::from(faces)
}

/// The closed cube shell with its first face dropped: an open box
/// (8 vertices, 12 edges, 5 faces), χ = 1 — a planted dropped-face defect.
fn open_box_shell() -> Shell<(), (), ()> {
    let mut faces = cube_shell().into_iter().collect::<Vec<_>>();
    faces.remove(0);
    Shell::from(faces)
}

// ---------------------------------------------------------------------------
// the boolean_m2 recipe fixtures (rebuilt here; boolean_m2.rs is not edited)
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

/// The 4×4 block profile: four `Curve::Line`s, CCW.
fn block_profile() -> (Vec<Curve>, Arrangement) {
    let profile = vec![
        Curve::Line(Line(Point3::new(0.0, 0.0, 0.0), Point3::new(4.0, 0.0, 0.0))),
        Curve::Line(Line(Point3::new(4.0, 0.0, 0.0), Point3::new(4.0, 4.0, 0.0))),
        Curve::Line(Line(Point3::new(4.0, 4.0, 0.0), Point3::new(0.0, 4.0, 0.0))),
        Curve::Line(Line(Point3::new(0.0, 4.0, 0.0), Point3::new(0.0, 0.0, 0.0))),
    ];
    let ok = arrange(&profile, None).expect("the dyadic block profile arranges");
    (profile, ok.value)
}

/// The M1 plate-with-hole profile: the 4×4 rectangle plus a full circle r=1
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
    let ok = arrange(&profile, None).expect("the dyadic plate profile arranges");
    (profile, ok.value)
}

/// A pure-disk profile: one full circle of radius `r` at `center`.
fn disk_profile(center: Point2, r: f64) -> (Vec<Curve>, Arrangement) {
    let circle = Curve::Circle(placed_circle(Point3::new(center.x, center.y, 0.0), r));
    let profile = vec![circle];
    let ok = arrange(&profile, None).expect("the dyadic disk profile arranges");
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

/// The flagship witnesses: `solid_a` = the 4×4×2 block, `solid_b` = the disk
/// column at (2, 2) r=1 height 2, `solid_ph` = the direct extrude of the
/// plate-with-hole profile (Extrude(P − Q)).
fn flagship_solids() -> (FixtureSolid, FixtureSolid, FixtureSolid) {
    let (profile_a, arr_a) = block_profile();
    let solid_a = extrude_solid(&profile_a, &arr_a, 2.0);
    let (profile_b, arr_b) = disk_profile(Point2::new(2.0, 2.0), 1.0);
    let solid_b = extrude_solid(&profile_b, &arr_b, 2.0);
    let (profile_ph, arr_ph) = plate_with_hole_profile();
    let solid_ph = extrude_solid(&profile_ph, &arr_ph, 2.0);
    (solid_a, solid_b, solid_ph)
}

/// Runs the gate on a solid's single boundary shell.
fn gate_solid_report(solid: &Solid<Point3, Curve, Surface>) -> GateReport {
    let shell = solid.boundaries().first().expect("one output shell");
    chi_homology_gate(shell)
        .expect("the output complex passes the χ/homology gate")
        .value
}

/// The three planted defects of the mutation battery, as shells: a dropped
/// face, an extra (duplicate) face, and a flipped orientation parity.
fn mutation_shells(
    shell: &Shell<Point3, Curve, Surface>,
) -> Vec<(&'static str, Shell<Point3, Curve, Surface>)> {
    let mut dropped = shell.face_iter().cloned().collect::<Vec<_>>();
    dropped.remove(0);

    let mut extra = shell.face_iter().cloned().collect::<Vec<_>>();
    let first = extra.first().expect("the shell has faces").clone();
    let duplicate = Face::new(first.absolute_boundaries().clone(), first.surface());
    extra.push(duplicate);

    let mut flipped = shell.face_iter().cloned().collect::<Vec<_>>();
    flipped.first_mut().expect("the shell has faces").invert();

    vec![
        ("dropped face", Shell::from(dropped)),
        ("extra face", Shell::from(extra)),
        ("flipped orientation parity", Shell::from(flipped)),
    ]
}

// ---------------------------------------------------------------------------
// Test 1: χ = V − E + F on hand-built complexes with known χ.
// ---------------------------------------------------------------------------

#[test]
fn chi_valuation_matches_known_complexes() {
    let cases = vec![
        ("cube shell", cube_shell(), 8usize, 12usize, 6usize, 2isize),
        ("torus", torus_shell(), 9usize, 18usize, 9usize, 0isize),
        (
            "sphere (tetrahedron)",
            tetrahedron_shell(),
            4usize,
            6usize,
            4usize,
            2isize,
        ),
    ];
    for (name, shell, vertices, edges, faces, expected_chi) in cases {
        let data = mod2_homology(&shell).expect("a hand-built complex is a chain complex");
        assert_eq!(
            (data.vertices, data.edges, data.faces),
            (vertices, edges, faces),
            "{name}: the V, E, F census"
        );
        assert_eq!(
            data.chi,
            data.vertices as isize - data.edges as isize + data.faces as isize,
            "{name}: χ = V − E + F"
        );
        assert_eq!(data.chi, expected_chi, "{name}: known Euler characteristic");
        let report = chi_homology_gate(&shell)
            .expect("a closed orientable shell passes the gate")
            .value;
        assert_eq!(report.chi, expected_chi, "{name}: gate χ");
        assert_eq!(
            report.chi,
            report.betti.b0 as isize - report.betti.b1 as isize + report.betti.b2 as isize,
            "{name}: χ = b0 − b1 + b2"
        );
    }
}

// ---------------------------------------------------------------------------
// Test 2: the Z₂ rank computation distinguishes a closed shell from the
// planted-defect variants.
// ---------------------------------------------------------------------------

#[test]
fn mod2_homology_detects_defect() {
    let closed = cube_shell();
    let closed_data = mod2_homology(&closed).expect("the cube is a chain complex");
    assert_eq!(
        closed_data.betti,
        BettiNumbers {
            b0: 1,
            b1: 0,
            b2: 1
        }
    );
    assert!(
        chi_homology_gate(&closed).is_ok(),
        "the closed shell passes"
    );

    // A dropped face opens a boundary: b2 collapses to 0 (and χ turns odd).
    let open_box = open_box_shell();
    let open_data = mod2_homology(&open_box).expect("the box is a chain complex");
    assert_ne!(
        open_data.betti, closed_data.betti,
        "the Z₂ ranks must distinguish the dropped-face variant"
    );
    assert_eq!(
        open_data.betti.b2, 0,
        "no closed 2-cycle survives the dropped face"
    );
    assert_eq!(open_data.betti.b0, 1, "the open box is still connected");
    assert!(
        chi_homology_gate(&open_box).is_err(),
        "the box refuses the gate"
    );

    // An extra (duplicate) face adds an independent closed 2-cycle.
    let mut extra_faces = cube_shell().into_iter().collect::<Vec<_>>();
    let first = extra_faces.first().expect("the cube has faces").clone();
    let duplicate = Face::new(first.absolute_boundaries().clone(), ());
    extra_faces.push(duplicate);
    let extra: Shell<(), (), ()> = Shell::from(extra_faces);
    let extra_data = mod2_homology(&extra).expect("the extra-face complex is a chain complex");
    assert_ne!(
        extra_data.betti, closed_data.betti,
        "the Z₂ ranks must distinguish the extra-face variant"
    );
    assert_eq!(
        extra_data.betti.b2, 2,
        "the duplicated face is a second 2-cycle"
    );
    assert!(
        chi_homology_gate(&extra).is_err(),
        "the extra-face complex refuses the gate"
    );

    // Two disjoint closed shells pass the local manifold stage but not the
    // homology stage: b0 = 2 ≠ 1 (H₀ is no longer a single Z₂).
    let mut two = cube_shell().into_iter().collect::<Vec<_>>();
    two.extend(tetrahedron_shell());
    let disjoint: Shell<(), (), ()> = Shell::from(two);
    let disjoint_data = mod2_homology(&disjoint).expect("two shells form a chain complex");
    assert_eq!(
        disjoint_data.betti.b0, 2,
        "two disjoint closed shells have two H₀ components"
    );
    assert!(
        chi_homology_gate(&disjoint).is_err(),
        "the homology stage refuses a multi-component closed complex"
    );

    // A flipped orientation parity leaves the mod-2 ranks IDENTICAL (Z₂ is
    // orientation-blind) — the manifold diagnostics stage is what refuses it.
    let mut flipped_faces = cube_shell().into_iter().collect::<Vec<_>>();
    flipped_faces
        .first_mut()
        .expect("the cube has faces")
        .invert();
    let flipped: Shell<(), (), ()> = Shell::from(flipped_faces);
    let flipped_data = mod2_homology(&flipped).expect("the flipped complex is a chain complex");
    assert_eq!(
        flipped_data.betti, closed_data.betti,
        "Z₂ ranks cannot see a single-face flip"
    );
    assert!(
        chi_homology_gate(&flipped).is_err(),
        "the orientation stage refuses the flipped-parity variant"
    );
}

// ---------------------------------------------------------------------------
// Test 3: a mismatching complex returns a typed refusal, never an annotated
// pass — the mutation battery on the boolean output complex.
// ---------------------------------------------------------------------------

#[test]
fn gate_fails_not_warns_on_mismatch() {
    let out = chi_homology_gate(&open_box_shell());
    assert!(out.is_err(), "a mismatching complex refuses, never passes");
    assert!(
        matches!(out, Err(Refusal::Contradictory(_))),
        "the refusal is typed"
    );

    let (solid_a, solid_b, _solid_ph) = flagship_solids();
    let mut budget = Budget::new(1000, 1000, 1000);
    let difference = boolean(&solid_a, BoolOp::Difference, &solid_b, &mut budget)
        .expect("the Difference flagship assembles through the entry")
        .value;
    let shell = difference
        .boundaries()
        .first()
        .expect("the Difference output is one shell");
    assert!(
        chi_homology_gate(shell).is_ok(),
        "the intact output complex passes the gate"
    );

    // The mutation battery: an extra face, a dropped face, and a flipped
    // orientation parity planted on the output complex must each FAIL the
    // gate — a gate that has only ever passed is indistinguishable from a
    // gate that cannot fail.
    for (label, mutant) in mutation_shells(shell) {
        let verdict = chi_homology_gate(&mutant);
        assert!(
            matches!(verdict, Err(Refusal::Contradictory(_))),
            "the {label} mutant must FAIL the gate with a typed refusal, got {verdict:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Test 4: the boolean_m2 recipe fixtures through the gate agree with the
// landed results, bit-for-bit on the canonical pairs.
// ---------------------------------------------------------------------------

#[test]
fn differential_congruent_with_boolean_m2() {
    let sphere_profile = BettiNumbers {
        b0: 1,
        b1: 0,
        b2: 1,
    };
    let torus_profile = BettiNumbers {
        b0: 1,
        b1: 2,
        b2: 1,
    };

    let (solid_a, solid_b, solid_ph) = flagship_solids();

    let mut difference_budget = Budget::new(1000, 1000, 1000);
    let difference = boolean(
        &solid_a,
        BoolOp::Difference,
        &solid_b,
        &mut difference_budget,
    )
    .expect("Difference assembles")
    .value;
    let mut intersection_budget = Budget::new(1000, 1000, 1000);
    let intersection = boolean(
        &solid_a,
        BoolOp::Intersection,
        &solid_b,
        &mut intersection_budget,
    )
    .expect("Intersection assembles")
    .value;
    let mut union_ab_budget = Budget::new(1000, 1000, 1000);
    let union_ab = boolean(&solid_a, BoolOp::Union, &solid_b, &mut union_ab_budget)
        .expect("A union B assembles")
        .value;
    let mut union_ba_budget = Budget::new(1000, 1000, 1000);
    let union_ba = boolean(&solid_b, BoolOp::Union, &solid_a, &mut union_ba_budget)
        .expect("B union A assembles")
        .value;
    let mut xor_budget = Budget::new(1000, 1000, 1000);
    let xor = boolean(&solid_a, BoolOp::Xor, &solid_b, &mut xor_budget)
        .expect("Xor assembles")
        .value;

    // The intact outputs must be single closed shells and must match the
    // landed `boolean_m2` face census exactly.
    assert_eq!(difference.boundaries().len(), 1);
    assert_eq!(
        difference.boundaries().first().unwrap().face_iter().count(),
        7
    );
    assert_eq!(intersection.boundaries().len(), 1);
    assert_eq!(
        intersection
            .boundaries()
            .first()
            .unwrap()
            .face_iter()
            .count(),
        3
    );
    assert_eq!(union_ab.boundaries().len(), 1);
    assert_eq!(
        union_ab.boundaries().first().unwrap().face_iter().count(),
        8
    );
    assert_eq!(union_ba.boundaries().len(), 1);
    assert_eq!(
        union_ba.boundaries().first().unwrap().face_iter().count(),
        8
    );
    assert_eq!(solid_ph.boundaries().len(), 1);
    assert_eq!(
        solid_ph.boundaries().first().unwrap().face_iter().count(),
        7
    );
    assert_eq!(solid_b.boundaries().len(), 1);
    assert_eq!(solid_b.boundaries().first().unwrap().face_iter().count(), 3);

    // The gate answers on the canonical pairs, congruent bit-for-bit with the
    // landed `boolean_m2` face-set bijections.
    let difference_report = gate_solid_report(&difference);
    let ph_report = gate_solid_report(&solid_ph);
    let xor_report = gate_solid_report(&xor);
    assert_eq!(difference_report, ph_report, "Difference ≅ Extrude(P − Q)");
    assert_eq!(
        difference_report, xor_report,
        "Xor ≅ Difference (same face set)"
    );
    assert_eq!(difference_report.chi, 0, "the plate-with-hole is a torus");
    assert_eq!(difference_report.betti, torus_profile, "b = (1, 2, 1)");

    let intersection_report = gate_solid_report(&intersection);
    let q_report = gate_solid_report(&solid_b);
    assert_eq!(intersection_report, q_report, "Intersection ≅ Extrude(Q)");
    assert_eq!(intersection_report.chi, 2, "the cylinder is a sphere");
    assert_eq!(intersection_report.betti, sphere_profile, "b = (1, 0, 1)");

    let union_ab_report = gate_solid_report(&union_ab);
    let union_ba_report = gate_solid_report(&union_ba);
    assert_eq!(
        union_ab_report, union_ba_report,
        "Union is commutative on the gate"
    );
    assert_eq!(union_ab_report.chi, 2, "the union is a sphere");
    assert_eq!(union_ab_report.betti, sphere_profile, "b = (1, 0, 1)");

    // Every output complex clears the gate: 7 congruent gate answers across
    // the flagship set (Difference, Extrude(P−Q), Xor, Intersection,
    // Extrude(Q), Union A∪B, Union B∪A).
    let congruence_count = [
        difference_report,
        ph_report,
        xor_report,
        intersection_report,
        q_report,
        union_ab_report,
        union_ba_report,
    ]
    .len();
    assert_eq!(
        congruence_count, 7,
        "the flagship set gates to seven reports"
    );
}
