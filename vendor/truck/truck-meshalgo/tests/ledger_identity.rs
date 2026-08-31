#![deny(clippy::unwrap_used)]

//! BG-CG-005-LEDGER — tests for the edge-sample ledger over a unit cube
//! compressed shell with line curves.

use truck_base::cgmath64::{Point3, Vector3};
use truck_base::tolerance::TOLERANCE;
use truck_meshalgo::prelude::*;
use truck_meshalgo::tessellation::domain::lattice::CertifiedLattice;
use truck_meshalgo::tessellation::formal::{
    CurveSchema, CurveSchemaFailure, SchemaIdentificationFailure, SupportSurfaceSchema,
};
use truck_modeling::builder;
use truck_modeling::{Curve, Solid, Surface};
use truck_topology::compress::CompressedShell;

type CubeShell = CompressedShell<Point3, Curve, Surface>;

fn lattice_of(surface: &Surface) -> CertifiedLattice {
    unevidenced_lattice(surface)
}

fn schema_of(_: &Surface) -> SupportSurfaceSchema {
    SupportSurfaceSchema::not_structurally_identified(
        SchemaIdentificationFailure::NoStructuralReader {
            representation: "ledger_identity_test",
        },
    )
}

fn curve_schema_of(_: &Curve) -> CurveSchema {
    CurveSchema::not_structurally_identified(CurveSchemaFailure::NoStructuralReader {
        representation: "ledger_identity_test",
    })
}

/// The unit cube as a `CompressedShell` with line curves, its fixture premises
/// machine-checked before anything is asserted against it.
fn cube_shell() -> CubeShell {
    let cube: Solid = {
        let v = builder::vertex(Point3::origin());
        let e = builder::tsweep(&v, Vector3::unit_x());
        let f = builder::tsweep(&e, Vector3::unit_y());
        builder::tsweep(&f, Vector3::unit_z())
    };
    let shell = cube.boundaries()[0].compress();
    assert_eq!(shell.vertices.len(), 8, "fixture premise: 8 vertices");
    assert_eq!(shell.edges.len(), 12, "fixture premise: 12 unique edges");
    assert_eq!(shell.faces.len(), 6, "fixture premise: 6 faces");
    assert!(
        shell.faces.iter().all(|face| face.boundaries.len() == 1),
        "fixture premise: one boundary per face"
    );
    assert!(
        shell.faces.iter().all(|face| face.boundaries[0].len() == 4),
        "fixture premise: four edge uses per boundary"
    );
    shell
}

fn run_ledger(shell: &CubeShell, tol: f64) -> EdgeSampleLedgerSet {
    let (ledger, _outcome) =
        triangulation_with_ledger(shell, tol, lattice_of, schema_of, curve_schema_of);
    ledger
}

fn run_both(
    shell: &CubeShell,
    tol: f64,
) -> (EdgeSampleLedgerSet, MeshedShellOutcome, MeshedShellOutcome) {
    let (ledger, via_ledger) =
        triangulation_with_ledger(shell, tol, lattice_of, schema_of, curve_schema_of);
    let direct =
        shell.robust_triangulation_with_schema_outcome(tol, lattice_of, schema_of, curve_schema_of);
    (ledger, via_ledger, direct)
}

#[test]
fn ledger_covers_every_unique_edge_once() {
    let shell = cube_shell();
    let ledger = run_ledger(&shell, TOLERANCE);
    assert_eq!(ledger.entries.len(), shell.edges.len());
    // One entry per unique edge, ordered by edge index ascending.
    assert!(
        ledger
            .entries
            .iter()
            .enumerate()
            .all(|(index, entry)| entry.edge == index),
        "one entry per unique edge, indices strictly ascending"
    );
    for entry in &ledger.entries {
        assert!(entry.parameters.len() >= 2, "at least two samples per edge");
        assert!(
            entry
                .parameters
                .windows(2)
                .all(|window| window[0] < window[1]),
            "parameters ascending"
        );
        assert_eq!(entry.parameters.len(), entry.position_indices.len());
        assert!(
            entry
                .position_indices
                .iter()
                .all(|&index| index < ledger.positions.len()),
            "every position index in bounds of the position table"
        );
    }
}

#[test]
fn shared_edge_identity_as_integers() {
    let shell = cube_shell();
    let ledger = run_ledger(&shell, TOLERANCE);
    for (edge_index, _edge) in shell.edges.iter().enumerate() {
        let mut uses: Vec<(usize, bool, Vec<usize>)> = Vec::new();
        for (face_index, face) in shell.faces.iter().enumerate() {
            for wire in &face.boundaries {
                for edge_use in wire {
                    if edge_use.index != edge_index {
                        continue;
                    }
                    let effective = edge_use.orientation ^ face.orientation;
                    let natural = ledger.entries[edge_index].position_indices.clone();
                    let consumed = if effective {
                        natural.iter().rev().copied().collect()
                    } else {
                        natural
                    };
                    uses.push((face_index, effective, consumed));
                }
            }
        }
        assert_eq!(uses.len(), 2, "edge {edge_index}: exactly two face-uses");
        let (face_a, effective_a, sequence_a) = &uses[0];
        let (face_b, effective_b, sequence_b) = &uses[1];
        assert_ne!(face_a, face_b, "edge {edge_index}: two distinct faces");
        assert_ne!(
            effective_a, effective_b,
            "edge {edge_index}: effective traversals are opposite"
        );
        let mut reversed_b = sequence_b.clone();
        reversed_b.reverse();
        assert_eq!(
            sequence_a, &reversed_b,
            "edge {edge_index}: I(A, E) == reverse(I(B, E)) as integers"
        );
    }
}

#[test]
fn ledger_matches_production_sampling_bit_for_bit() {
    let shell = cube_shell();
    let (ledger, outcome, _direct) = run_both(&shell, TOLERANCE);
    for (edge_index, entry) in ledger.entries.iter().enumerate() {
        let production_polyline = &outcome.shell.edges[edge_index].curve;
        assert!(
            !production_polyline.is_empty(),
            "edge {edge_index}: production edge was sampled"
        );
        // Every ledger position equals the corresponding production polyline
        // position exactly (f64 `==`, component by component), in order.
        let mut search_from = 0;
        for &position_index in &entry.position_indices {
            let position = &ledger.positions[position_index];
            let mut matched = false;
            for (offset, sample) in production_polyline.iter().skip(search_from).enumerate() {
                if sample == position {
                    search_from += offset + 1;
                    matched = true;
                    break;
                }
            }
            assert!(
                matched,
                "edge {edge_index}: ledger position missing from production polyline"
            );
        }
    }
}

#[test]
fn closed_shell_has_no_boundary_edge_uses() {
    let shell = cube_shell();
    let _ledger = run_ledger(&shell, TOLERANCE);
    for (edge_index, _edge) in shell.edges.iter().enumerate() {
        let mut effective: Vec<bool> = Vec::new();
        for face in &shell.faces {
            for wire in &face.boundaries {
                for edge_use in wire {
                    if edge_use.index == edge_index {
                        effective.push(edge_use.orientation ^ face.orientation);
                    }
                }
            }
        }
        assert_eq!(
            effective.len(),
            2,
            "edge {edge_index}: appears in exactly two face-uses"
        );
        assert_ne!(
            effective[0], effective[1],
            "edge {edge_index}: one forward, one backward"
        );
    }
}

#[test]
fn ledger_outcome_equals_unchanged_entry_outcome() {
    let shell = cube_shell();
    let (_ledger, via_ledger, direct) = run_both(&shell, TOLERANCE);
    // The whole shell record, including every per-face polygon, is
    // bit-identical to the unchanged entry point's.
    assert_eq!(via_ledger.shell, direct.shell);
    // The failure/diagnosis vectors are structurally equal. The inner types do
    // not implement `PartialEq`; the fixture produces no failures, so equality
    // is decided by None-ness and, where both are present, by the typed
    // terminal reason.
    assert_eq!(via_ledger.face_failures.len(), direct.face_failures.len());
    assert!(via_ledger
        .face_failures
        .iter()
        .zip(&direct.face_failures)
        .all(|(left, right)| match (left, right) {
            (None, None) => true,
            (Some(left_failure), Some(right_failure)) => {
                left_failure.reason == right_failure.reason
            }
            _ => false,
        }));
    assert_eq!(via_ledger.face_diagnoses.len(), direct.face_diagnoses.len());
    assert!(via_ledger
        .face_diagnoses
        .iter()
        .zip(&direct.face_diagnoses)
        .all(|(left, right)| match (left, right) {
            (None, None) => true,
            (Some(left_record), Some(right_record)) => {
                left_record.terminal_reason == right_record.terminal_reason
            }
            _ => false,
        }));
    assert_eq!(via_ledger.band_attempts, direct.band_attempts);
    assert_eq!(via_ledger.cone_band_attempts, direct.cone_band_attempts);
    assert_eq!(via_ledger.torus_band_attempts, direct.torus_band_attempts);
}
