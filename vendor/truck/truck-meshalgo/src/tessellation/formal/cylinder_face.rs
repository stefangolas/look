//! Checkpoint 4: the minimum rank-1 cylinder face record.
//!
//! # What this module adds
//!
//! Nothing beyond the pairing itself. Step 2's authoritative traversal
//! ([`super::planar_slice::regular_traversal`]) is purely combinatorial — it
//! reads [`SourceFaceInput`], an [`OuterBoundStanding`] and a curve accessor,
//! and never consults the support surface — so it is reused directly rather
//! than re-derived for the rank-1 case. [`CylinderFaceRecord`] is that
//! traversal plus the certified cylinder it is trimmed from and the face's
//! `source_face_id`, and nothing else.
//!
//! A general periodic-face record (rank-1 *or* rank-2, arbitrary bound count)
//! is deliberately not built here: the supported subset is one authoritative
//! outer bound, no holes, on a certified embedded cylinder, and
//! [`build_cylinder_face`] refuses everything else with [`SliceExit`] —
//! the identical taxonomy the planar slice already reports through.

use super::super::source_evidence::{EdgeUseId, SourceFaceInput};
use super::cylinder::CertifiedEmbeddedCylinder;
use super::planar_slice::{regular_traversal, RegularClosedTraversal, SliceExit};
use super::support::CurveSchema;
use truck_topology::compress::OuterBoundStanding;

/// The smallest rank-1 face record.
///
/// Carries exactly:
///
/// - `source_face_id`, for keying diagnostics and the recovery gate;
/// - the certified cylinder this face is trimmed from;
/// - Step 2's authoritative closed traversal of the one outer bound, with its
///   ordered [`EdgeUseId`]s, source vertex identities, selected edge
///   orientations and source curve representations untouched.
#[derive(Debug, Clone)]
pub struct CylinderFaceRecord {
    /// The document entity this face came from, when the importer retained it.
    pub source_face_id: Option<u64>,
    /// The certified embedded cylinder this face is trimmed from.
    pub cylinder: CertifiedEmbeddedCylinder,
    /// Step 2's authoritative closed traversal of the one outer bound.
    pub traversal: RegularClosedTraversal,
}

impl CylinderFaceRecord {
    /// The ordered edge-use identities, in source cyclic order.
    pub fn edge_use_ids(&self) -> Vec<EdgeUseId> {
        self.traversal
            .occurrences
            .iter()
            .map(|occurrence| occurrence.edge_use)
            .collect()
    }
}

/// Build the minimum rank-1 face record from a certified cylinder and one
/// face's source evidence.
///
/// Delegates traversal entirely to [`regular_traversal`]: this function does
/// not duplicate the outer-bound authority check, the hole-free check, the
/// cyclic-continuity check, or the per-use orientation/endpoint checks. A
/// face that fails any of them exits with the same [`SliceExit`] a planar
/// face would.
pub fn build_cylinder_face(
    source_face_id: Option<u64>,
    cylinder: CertifiedEmbeddedCylinder,
    input: &SourceFaceInput,
    outer_bound: OuterBoundStanding,
    curves: &mut impl FnMut(usize) -> CurveSchema,
) -> Result<CylinderFaceRecord, SliceExit> {
    let traversal = regular_traversal(input, outer_bound, curves)?;
    Ok(CylinderFaceRecord {
        source_face_id,
        cylinder,
        traversal,
    })
}

#[cfg(test)]
mod tests {
    use super::super::super::source_evidence::{
        BoundId, ErasedOrientationMechanism, OrientationEvidence, OrientationOrigin,
        SourceBoundInput, SourceEdgeOrientationEvidence, SourceEdgeUseInput,
        SourceFaceOrientationEvidence, SourceVertexKey,
    };
    use super::super::cylinder::identify_cylinder;
    use super::super::support::identify_line_segment;
    use super::*;
    use truck_geometry::prelude::{Line, Point3, RevolutedCurve, Vector3};

    fn z_cylinder(radius: f64, h: f64) -> CertifiedEmbeddedCylinder {
        let revo = RevolutedCurve::by_revolution(
            Line(Point3::new(radius, 0.0, 0.0), Point3::new(radius, 0.0, h)),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        );
        match identify_cylinder(&revo) {
            super::super::cylinder::CylinderIdentification::Cylinder(c) => c,
            other => panic!("expected a certified cylinder, got {other:?}"),
        }
    }

    /// A face whose one bound is the given ordered vertex cycle, with the
    /// `i`-th edge use running forward exactly when `forward(i)` says so — the
    /// same construction the planar slice tests use, so this test exercises
    /// only the pairing this module adds, not a re-derived traversal.
    fn cycle_input(
        points: &[Point3],
        forward: impl Fn(usize) -> bool,
    ) -> (SourceFaceInput, Vec<CurveSchema>) {
        let n = points.len();
        let mut edge_uses = Vec::with_capacity(n);
        let mut curves = Vec::with_capacity(n);
        for i in 0..n {
            let j = (i + 1) % n;
            let runs_forward = forward(i);
            let (edge_from, edge_to) = match runs_forward {
                true => (i, j),
                false => (j, i),
            };
            curves.push(identify_line_segment(&Line(
                points[edge_from],
                points[edge_to],
            )));
            let source_vertices = (
                SourceVertexKey::ShellVertex(edge_from),
                SourceVertexKey::ShellVertex(edge_to),
            );
            let use_vertices = match runs_forward {
                true => source_vertices,
                false => (source_vertices.1, source_vertices.0),
            };
            edge_uses.push(SourceEdgeUseInput {
                id: EdgeUseId::new(BoundId(0), i),
                source_edge_index: i,
                source_vertices,
                use_vertices,
                orientation: SourceEdgeOrientationEvidence {
                    bound_times_oriented_edge: OrientationEvidence::Retained {
                        forward: runs_forward,
                        origin: OrientationOrigin::BoundTimesOrientedEdge,
                    },
                    edge_curve_same_sense: OrientationEvidence::HistoryErased {
                        mechanism:
                            ErasedOrientationMechanism::EdgeCurveSenseFoldedIntoConvertedCurve,
                    },
                    selected_curve_direction: OrientationEvidence::HistoryErased {
                        mechanism:
                            ErasedOrientationMechanism::SelectedCurveDirectionFoldedIntoConvertedCurve,
                    },
                },
            });
        }
        let input = SourceFaceInput {
            source_face_id: Some(42),
            declared_face_index: 0,
            bounds: vec![SourceBoundInput::EdgeUses {
                id: BoundId(0),
                edge_uses,
            }],
            orientation: SourceFaceOrientationEvidence {
                face_use_orientation: OrientationEvidence::Missing,
                face_surface_same_sense: OrientationEvidence::Missing,
            },
        };
        (input, curves)
    }

    fn declared_outer() -> OuterBoundStanding {
        OuterBoundStanding::Declared {
            bound_index: 0,
            declared_count: 1,
        }
    }

    #[test]
    fn a_cylinder_face_preserves_source_traversal_exactly() {
        // Four vertices, one edge use reversed against its curve — the same
        // hazard the planar-slice fixtures probe — to prove the record does
        // not silently re-apply or drop the retained sense.
        let points = [
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 1.0),
            Point3::new(0.0, 1.0, 1.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        let (input, curves) = cycle_input(&points, |i| i != 1);

        let record = build_cylinder_face(
            input.source_face_id,
            z_cylinder(2.0, 5.0),
            &input,
            declared_outer(),
            &mut |index| curves[index].clone(),
        )
        .expect("a regular closed traversal on one outer bound resolves");

        assert_eq!(record.source_face_id, Some(42));

        // Ordered EdgeUseIds: exactly the source cyclic order, one per bound
        // position, nothing merged or reordered.
        let ids = record.edge_use_ids();
        assert_eq!(
            ids,
            (0..4)
                .map(|i| EdgeUseId::new(BoundId(0), i))
                .collect::<Vec<_>>()
        );

        // Source vertex identities and the cyclic join they establish.
        let occurrences = &record.traversal.occurrences;
        assert_eq!(occurrences[0].start_vertex, SourceVertexKey::ShellVertex(0));
        assert_eq!(occurrences[0].end_vertex, SourceVertexKey::ShellVertex(1));
        // Edge use 1 was declared reversed: its curve runs 2 -> 1 but the
        // traversal direction is still 1 -> 2, matching edge use 0's end.
        assert_eq!(occurrences[1].start_vertex, SourceVertexKey::ShellVertex(1));
        assert_eq!(occurrences[1].end_vertex, SourceVertexKey::ShellVertex(2));
        assert!(
            !occurrences[1].forward,
            "the declared reversal is retained, not erased"
        );
        assert_eq!(occurrences[2].start_vertex, SourceVertexKey::ShellVertex(2));
        assert_eq!(occurrences[2].end_vertex, SourceVertexKey::ShellVertex(3));
        assert_eq!(occurrences[3].start_vertex, SourceVertexKey::ShellVertex(3));
        assert_eq!(occurrences[3].end_vertex, SourceVertexKey::ShellVertex(0));

        // The source curve representation is carried, not discarded.
        for occurrence in occurrences {
            assert!(occurrence.curve.polygonal().is_some());
        }

        // The certified cylinder travels with the record untouched.
        assert!((record.cylinder.schema().radius().get() - 2.0).abs() < 1e-12);
    }

    #[test]
    fn a_second_declared_outer_bound_is_a_source_contradiction() {
        let points = [
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 1.0),
            Point3::new(0.0, 1.0, 1.0),
        ];
        let (input, curves) = cycle_input(&points, |_| true);
        let outer = OuterBoundStanding::Declared {
            bound_index: 0,
            declared_count: 2,
        };
        let result = build_cylinder_face(
            input.source_face_id,
            z_cylinder(1.0, 1.0),
            &input,
            outer,
            &mut |index| curves[index].clone(),
        );
        assert_eq!(result.unwrap_err(), SliceExit::MultipleOuterBoundsDeclared);
    }
}
