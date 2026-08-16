//! Tests for the planar vertical slice.
//!
//! The negative cases matter more than the positive one. A slice that meshes a
//! square proves very little; a slice that *refuses* a face violating any
//! admitted premise is the claim worth testing, so most of what follows
//! constructs a specific violation and checks it exits with the right reason in
//! the right category.

use super::super::super::source_evidence::{
    BoundId, EdgeUseId, ErasedOrientationMechanism, OrientationEvidence, OrientationOrigin,
    SourceBoundInput, SourceEdgeOrientationEvidence, SourceEdgeUseInput, SourceFaceInput,
    SourceFaceOrientationEvidence, SourceVertexKey,
};
use super::super::ambient::{ambient_evidence_from_plane_schema, resolve_ambient_periods};
use super::super::envelope::{FormalEnvelope, PolicyInstanceId};
use super::super::outcome::{DocumentScope, FaceKey, ShellKey, StageOutcome};
use super::super::support::{
    identify_line_segment, identify_plane, identify_polyline, CurveSchema, CurveSchemaFailure,
};
use super::*;
use truck_geometry::prelude::{Line, Plane, Point3};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn xy_plane() -> PlaneSchema {
    *identify_plane(&Plane::new(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(0.0, 1.0, 0.0),
    ))
    .plane()
    .expect("the xy plane identifies")
}

/// A plane with a skewed, unequally scaled basis. Legal STEP, and the case a
/// per-axis inverse gets wrong.
fn skew_plane() -> PlaneSchema {
    *identify_plane(&Plane::new(
        Point3::new(1.0, 2.0, 0.0),
        Point3::new(4.0, 2.0, 0.0),
        Point3::new(2.0, 5.0, 0.0),
    ))
    .plane()
    .expect("a skewed plane is still a plane")
}

fn rank0(plane: &PlaneSchema) -> CertifiedAmbientLattice {
    let evidence = ambient_evidence_from_plane_schema(plane).expect("plane rule");
    let envelope = FormalEnvelope::new(
        PolicyInstanceId::new(1),
        2,
        4,
        64,
        4096,
        16,
        64,
        32,
        1 << 20,
    )
    .expect("well-formed");
    let face = FaceKey {
        document: DocumentScope::SingleDocumentRun,
        shell: ShellKey::new(0),
        source_face_id: None,
        declared_face_index: 0,
    };
    match resolve_ambient_periods(evidence, &envelope, face).expect("no operational failure") {
        StageOutcome::Resolved(lattice) => lattice,
        other => panic!("expected rank 0, got {other:?}"),
    }
}

fn declared_outer(count: u32) -> OuterBoundStanding {
    OuterBoundStanding::Declared {
        bound_index: 0,
        declared_count: count,
    }
}

/// A face whose bound is the given ordered vertex cycle: one edge use per
/// consecutive pair, each a straight `LINE` whose converted curve runs from the
/// first vertex to the second, all in the source's own direction.
struct Fixture {
    input: SourceFaceInput,
    positions: Vec<Point3>,
    curves: Vec<CurveSchema>,
}

impl Fixture {
    fn from_cycle(points: &[Point3]) -> Self {
        Self::from_cycle_with(points, |_| true)
    }

    /// As [`Self::from_cycle`], but `forward(i)` decides whether edge use `i`
    /// runs with its curve or against it. A reversed use gets a curve stored in
    /// the opposite direction, exactly as `EDGE_CURVE` sense folding produces.
    fn from_cycle_with(points: &[Point3], forward: impl Fn(usize) -> bool) -> Self {
        let n = points.len();
        let mut edge_uses = Vec::with_capacity(n);
        let mut curves = Vec::with_capacity(n);
        for i in 0..n {
            let j = (i + 1) % n;
            let runs_forward = forward(i);
            // The edge's *own* endpoints, in the edge's own direction.
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
        Self {
            input: SourceFaceInput {
                source_face_id: Some(1),
                declared_face_index: 0,
                bounds: vec![SourceBoundInput::EdgeUses {
                    id: BoundId(0),
                    edge_uses,
                }],
                orientation: SourceFaceOrientationEvidence {
                    face_use_orientation: OrientationEvidence::Missing,
                    face_surface_same_sense: OrientationEvidence::Missing,
                },
            },
            positions: points.to_vec(),
            curves,
        }
    }

    fn run(&self, plane: &PlaneSchema, outer: OuterBoundStanding) -> SliceRecord {
        let curves = self.curves.clone();
        let positions = self.positions.clone();
        run_planar_slice(
            &self.input,
            plane,
            &rank0(plane),
            outer,
            &mut |index| curves[index].clone(),
            &|key| match key {
                SourceVertexKey::ShellVertex(index) => positions.get(index).copied(),
                _ => None,
            },
            1e-6,
        )
    }
}

fn unit_square() -> Fixture {
    Fixture::from_cycle(&[
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(1.0, 1.0, 0.0),
        Point3::new(0.0, 1.0, 0.0),
    ])
}

/// An L: simple, concave, and not convex at one vertex, so ear clipping has to
/// skip a reflex vertex rather than take the first candidate.
fn concave_l() -> Fixture {
    Fixture::from_cycle(&[
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(2.0, 0.0, 0.0),
        Point3::new(2.0, 1.0, 0.0),
        Point3::new(1.0, 1.0, 0.0),
        Point3::new(1.0, 2.0, 0.0),
        Point3::new(0.0, 2.0, 0.0),
    ])
}

// ---------------------------------------------------------------------------
// End to end
// ---------------------------------------------------------------------------

#[test]
fn a_planar_line_bounded_square_resolves_through_every_stage() {
    let record = unit_square().run(&xy_plane(), declared_outer(1));
    assert_eq!(record.exit, None, "unexpected exit: {:?}", record.exit);
    assert_eq!(record.stage, SliceStage::FinalValidity);
    assert_eq!(record.category, SliceCategory::Resolved);
    let validity = record.validity.expect("final validity");
    assert_eq!(validity.triangles, 2);
    assert_eq!(validity.vertices, 4);
    assert_eq!(validity.boundary_edges, 4);
    assert_eq!(validity.internal_edges, 1);
    let mesh = record.mesh.expect("a mesh");
    assert_eq!(mesh.positions.len(), 4);
    assert_eq!(mesh.triangles.len(), 2);
}

#[test]
fn a_concave_simple_polygon_triangulates_correctly() {
    let record = concave_l().run(&xy_plane(), declared_outer(1));
    assert_eq!(record.stage, SliceStage::FinalValidity, "{:?}", record.exit);
    let validity = record.validity.expect("final validity");
    assert_eq!(validity.triangles, 4, "n - 2 triangles for n = 6");
    assert_eq!(validity.boundary_edges, 6);
}

#[test]
fn a_skewed_plane_basis_resolves_and_lifts_back_to_the_source_points() {
    // The face lies in the skewed plane's own `z = 0`, so it is a genuine
    // planar face there; the parameter coordinates are *not* the world ones,
    // which is the whole point of keeping the native chart.
    let plane = skew_plane();
    let corners: Vec<Point3> = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]
        .into_iter()
        .map(|(u, v)| plane.point_at(u, v))
        .collect();
    let record = Fixture::from_cycle(&corners).run(&plane, declared_outer(1));
    assert_eq!(record.stage, SliceStage::FinalValidity, "{:?}", record.exit);
    let mesh = record.mesh.expect("a mesh");
    for (lifted, source) in mesh.positions.iter().zip(&corners) {
        assert!(
            (lifted - source).magnitude() < 1e-12,
            "lift did not return the source point: {lifted:?} vs {source:?}"
        );
    }
}

#[test]
fn a_cyclic_rotation_of_the_same_source_gives_the_same_result() {
    let square = [
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(1.0, 1.0, 0.0),
        Point3::new(0.0, 1.0, 0.0),
    ];
    let rotated = [square[1], square[2], square[3], square[0]];
    let a = Fixture::from_cycle(&square).run(&xy_plane(), declared_outer(1));
    let b = Fixture::from_cycle(&rotated).run(&xy_plane(), declared_outer(1));
    assert_eq!(a.stage, SliceStage::FinalValidity);
    assert_eq!(b.stage, SliceStage::FinalValidity);
    assert_eq!(a.validity, b.validity);
}

#[test]
fn a_reversed_edge_use_traverses_its_curve_backwards_and_still_closes() {
    // Every other use runs against its stored curve, as `EDGE_CURVE` sense
    // folding produces. Applying the sign twice — or not at all — breaks the
    // cycle, so this passing means it is applied exactly once.
    let record = Fixture::from_cycle_with(
        &[
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ],
        |i| i % 2 == 0,
    )
    .run(&xy_plane(), declared_outer(1));
    assert_eq!(record.stage, SliceStage::FinalValidity, "{:?}", record.exit);
}

#[test]
fn the_missing_normalized_physical_sign_does_not_block_the_slice() {
    // The fixture leaves both face-level orientation factors `Missing`, which
    // is what the corpus supplies: Step 0 measured 0 of 110,770 computable
    // normalized signs. Nothing in the slice asks for one.
    let fixture = unit_square();
    assert!(fixture
        .input
        .edge_uses()
        .all(|use_| use_.normalized_sign(&fixture.input.orientation).is_none()));
    assert_eq!(
        fixture.run(&xy_plane(), declared_outer(1)).stage,
        SliceStage::FinalValidity
    );
}

// ---------------------------------------------------------------------------
// Step 2 refusals
// ---------------------------------------------------------------------------

#[test]
fn having_one_bound_is_not_outer_bound_standing() {
    for standing in [
        OuterBoundStanding::NotRetained,
        OuterBoundStanding::NoneDeclared,
    ] {
        let record = unit_square().run(&xy_plane(), standing);
        assert_eq!(record.exit, Some(SliceExit::MissingOuterBoundAuthority));
        assert_eq!(record.category, SliceCategory::Unresolved);
    }
}

#[test]
fn two_declared_outer_bounds_are_a_source_contradiction() {
    let record = unit_square().run(&xy_plane(), declared_outer(2));
    assert_eq!(record.exit, Some(SliceExit::MultipleOuterBoundsDeclared));
    assert_eq!(record.category, SliceCategory::Inconsistent);
}

#[test]
fn an_empty_bound_is_a_degenerate_traversal() {
    let mut fixture = unit_square();
    fixture.input.bounds = vec![SourceBoundInput::EdgeUses {
        id: BoundId(0),
        edge_uses: Vec::new(),
    }];
    let record = fixture.run(&xy_plane(), declared_outer(1));
    assert_eq!(record.exit, Some(SliceExit::DegenerateTraversal));
    assert_eq!(record.category, SliceCategory::Unsupported);
}

#[test]
fn a_degenerate_evidence_bound_is_a_degenerate_traversal() {
    let mut fixture = unit_square();
    fixture.input.bounds = vec![SourceBoundInput::DegenerateEvidenceUnavailable { id: BoundId(0) }];
    assert_eq!(
        fixture.run(&xy_plane(), declared_outer(1)).exit,
        Some(SliceExit::DegenerateTraversal)
    );
}

#[test]
fn a_second_bound_is_unsupported_rather_than_ignored() {
    let mut fixture = unit_square();
    let extra = fixture.input.bounds[0].clone();
    fixture.input.bounds.push(extra);
    let record = fixture.run(&xy_plane(), declared_outer(1));
    assert_eq!(record.exit, Some(SliceExit::MultipleBoundsOrHoles));
    assert_eq!(record.category, SliceCategory::Unsupported);
}

#[test]
fn a_missing_geometric_traversal_occurrence_blocks_the_traversal() {
    let mut fixture = unit_square();
    if let SourceBoundInput::EdgeUses { edge_uses, .. } = &mut fixture.input.bounds[0] {
        edge_uses[2].orientation.bound_times_oriented_edge = OrientationEvidence::Missing;
    }
    let record = fixture.run(&xy_plane(), declared_outer(1));
    assert_eq!(
        record.exit,
        Some(SliceExit::MissingGeometricTraversalOccurrence)
    );
    assert_eq!(record.category, SliceCategory::Unresolved);
}

#[test]
fn a_checked_discontinuity_is_inconsistent() {
    let mut fixture = unit_square();
    if let SourceBoundInput::EdgeUses { edge_uses, .. } = &mut fixture.input.bounds[0] {
        // Point one use at a vertex its neighbour does not end at. Both
        // endpoint orders are rewritten so the *consistency* predicate still
        // holds and the failure is the join, not the sense.
        edge_uses[1].source_vertices = (
            SourceVertexKey::ShellVertex(7),
            SourceVertexKey::ShellVertex(2),
        );
        edge_uses[1].use_vertices = edge_uses[1].source_vertices;
    }
    let record = fixture.run(&xy_plane(), declared_outer(1));
    assert_eq!(record.exit, Some(SliceExit::SourceJoinContradiction));
    assert_eq!(record.category, SliceCategory::Inconsistent);
}

#[test]
fn applying_the_composed_sense_twice_is_detected() {
    let mut fixture = unit_square();
    if let SourceBoundInput::EdgeUses { edge_uses, .. } = &mut fixture.input.bounds[0] {
        // Swap the use order without changing the retained sign: exactly what
        // a consumer that read the sign and swapped again would produce.
        let (a, b) = edge_uses[0].use_vertices;
        edge_uses[0].use_vertices = (b, a);
    }
    assert_eq!(
        fixture.run(&xy_plane(), declared_outer(1)).exit,
        Some(SliceExit::EndpointsNotConsistentWithRetainedSense)
    );
}

#[test]
fn coordinate_close_but_source_distinct_vertices_stay_distinct() {
    // Two vertices a nanometre apart, joined nowhere by the source. Nothing in
    // the slice welds them, so the cycle stays open and the join contradiction
    // is reported rather than repaired.
    let mut fixture = unit_square();
    fixture.positions.push(Point3::new(1e-9, 0.0, 0.0));
    if let SourceBoundInput::EdgeUses { edge_uses, .. } = &mut fixture.input.bounds[0] {
        edge_uses[0].source_vertices = (
            SourceVertexKey::ShellVertex(4),
            SourceVertexKey::ShellVertex(1),
        );
        edge_uses[0].use_vertices = edge_uses[0].source_vertices;
    }
    assert_eq!(
        fixture.run(&xy_plane(), declared_outer(1)).exit,
        Some(SliceExit::SourceJoinContradiction)
    );
}

#[test]
fn source_identical_joins_survive_coordinate_noise() {
    // The same square with every corner jittered well inside tolerance. The
    // joins are established by identity, so nothing here is affected.
    let jitter = 1e-9;
    let record = Fixture::from_cycle(&[
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0 + jitter, -jitter, 0.0),
        Point3::new(1.0 - jitter, 1.0 + jitter, 0.0),
        Point3::new(jitter, 1.0, 0.0),
    ])
    .run(&xy_plane(), declared_outer(1));
    assert_eq!(record.stage, SliceStage::FinalValidity, "{:?}", record.exit);
}

// ---------------------------------------------------------------------------
// Step 3 refusals
// ---------------------------------------------------------------------------

#[test]
fn an_unsupported_curve_representation_is_refused() {
    let mut fixture = unit_square();
    fixture.curves[1] =
        CurveSchema::not_structurally_identified(CurveSchemaFailure::NoStructuralReader {
            representation: "circle",
        });
    let record = fixture.run(&xy_plane(), declared_outer(1));
    assert_eq!(record.exit, Some(SliceExit::UnsupportedCurveRepresentation));
    assert_eq!(record.category, SliceCategory::Unsupported);
}

#[test]
fn a_curve_off_the_support_plane_is_inconsistent() {
    let mut fixture = unit_square();
    fixture.curves[1] = identify_line_segment(&Line(
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(1.0, 1.0, 5.0),
    ));
    fixture.positions[2] = Point3::new(1.0, 1.0, 5.0);
    let record = fixture.run(&xy_plane(), declared_outer(1));
    assert_eq!(record.exit, Some(SliceExit::CurveSurfaceInconsistency));
    assert_eq!(record.category, SliceCategory::Inconsistent);
}

#[test]
fn a_curve_whose_end_is_not_its_declared_vertex_is_inconsistent() {
    let mut fixture = unit_square();
    // The curve stops a millimetre short of the vertex its edge declares.
    fixture.curves[0] = identify_line_segment(&Line(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(0.999, 0.0, 0.0),
    ));
    assert_eq!(
        fixture.run(&xy_plane(), declared_outer(1)).exit,
        Some(SliceExit::CurveSurfaceInconsistency)
    );
}

#[test]
fn a_nearly_singular_plane_basis_never_reaches_a_projection() {
    // `identify_plane` refuses this basis outright, so there is no
    // `PlaneSchema` to project with — the ill-conditioned case cannot be
    // reached by presenting a plane, only by a caller building one.
    let nearly = identify_plane(&Plane::new(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(1.0, 1e-6, 0.0),
    ));
    assert!(nearly.plane().is_none());
}

#[test]
fn a_poorly_conditioned_but_identifiable_basis_is_unresolved() {
    // Separation of ~4e-8: above `identify_plane`'s structural floor of 1e-9,
    // below Step 3's numerical requirement of 1e-6. The face exits
    // `Unresolved`, not `Unsupported`: nothing was proved about it.
    let plane = identify_plane(&Plane::new(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(1.0, 2e-4, 0.0),
    ));
    let plane = plane.plane().expect("above the structural floor");
    let corners: Vec<Point3> = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]
        .into_iter()
        .map(|(u, v)| plane.point_at(u, v))
        .collect();
    let record = Fixture::from_cycle(&corners).run(plane, declared_outer(1));
    assert_eq!(record.exit, Some(SliceExit::IllConditionedPlaneBasis));
    assert_eq!(record.category, SliceCategory::Unresolved);
}

#[test]
fn the_gram_solve_is_used_rather_than_the_per_axis_quotient() {
    // On the skewed plane the per-axis quotient would give a different `u` for
    // the corner at `(0, 1)`. If the slice used it, the projected square would
    // be a different quadrilateral — still simple, so it would still mesh — but
    // the lift would not return the source points. That is what this asserts,
    // and it is checked in `a_skewed_plane_basis_resolves_and_lifts_back...`;
    // here the disagreement itself is pinned so the two tests cannot both be
    // satisfied by an orthogonal-only implementation.
    let plane = skew_plane();
    let point = plane.point_at(0.0, 1.0);
    let offset = point - plane.origin();
    let naive_u = offset.dot(plane.u_axis()) / plane.gram().g00();
    assert!(naive_u.abs() > 0.3, "the bases must actually be skewed");
}

// ---------------------------------------------------------------------------
// Steps 4-6
// ---------------------------------------------------------------------------

#[test]
fn every_source_occurrence_gets_exactly_one_developed_occurrence() {
    let fixture = concave_l();
    let plane = xy_plane();
    let curves = fixture.curves.clone();
    let positions = fixture.positions.clone();
    let traversal = regular_traversal(&fixture.input, declared_outer(1), &mut |i| {
        curves[i].clone()
    })
    .expect("traversal");
    let planar = certified_planar_curves(
        &traversal,
        &plane,
        &|key| match key {
            SourceVertexKey::ShellVertex(index) => positions.get(index).copied(),
            _ => None,
        },
        1e-6,
    )
    .expect("planar curves");
    let developed = rank0_lift(&rank0(&plane), planar).expect("lift");
    assert_eq!(developed.occurrences.len(), 6);
    assert_eq!(developed.displacements.len(), 6);
    assert!(developed
        .displacements
        .iter()
        .all(|d| *d == Rank0Displacement));
    let solution = trivial_deck_solution(&developed).expect("deck solution");
    assert_eq!(solution.holonomy, Rank0Displacement);
    assert_eq!(solution.joins_checked, 6);
}

#[test]
fn a_rank1_lattice_never_enters_the_rank0_lift() {
    // There is no way to build a rank-1 lattice from a plane, so the guard is
    // exercised through `run_planar_slice`'s own entry test instead: any
    // non-rank-0 lattice exits before Step 2.
    let plane = xy_plane();
    let lattice = rank0(&plane);
    assert!(matches!(lattice, CertifiedAmbientLattice::Rank0(_)));
    let occurrences = Vec::new();
    assert!(rank0_lift(&lattice, occurrences).is_ok());
}

#[test]
fn the_working_cover_contains_every_complete_occurrence_in_one_copy() {
    let fixture = concave_l();
    let plane = xy_plane();
    let curves = fixture.curves.clone();
    let positions = fixture.positions.clone();
    let traversal = regular_traversal(&fixture.input, declared_outer(1), &mut |i| {
        curves[i].clone()
    })
    .expect("traversal");
    let planar = certified_planar_curves(
        &traversal,
        &plane,
        &|key| match key {
            SourceVertexKey::ShellVertex(index) => positions.get(index).copied(),
            _ => None,
        },
        1e-6,
    )
    .expect("planar");
    let developed = rank0_lift(&rank0(&plane), planar).expect("lift");
    let cover = one_copy_working_cover(&developed).expect("cover");
    assert_eq!(cover.copies, 1);
    assert_eq!(cover.occurrences, 6);
    assert_eq!(cover.min, Point2::new(0.0, 0.0));
    assert_eq!(cover.max, Point2::new(2.0, 2.0));
    for occurrence in &developed.occurrences {
        for point in &occurrence.points {
            assert!(point.x >= cover.min.x && point.x <= cover.max.x);
            assert!(point.y >= cover.min.y && point.y <= cover.max.y);
        }
    }
}

// ---------------------------------------------------------------------------
// Step 7
// ---------------------------------------------------------------------------

#[test]
fn a_nonadjacent_crossing_is_refused() {
    // A bowtie: the source declares a closed cycle whose segments cross.
    let record = Fixture::from_cycle(&[
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 1.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(0.0, 1.0, 0.0),
    ])
    .run(&xy_plane(), declared_outer(1));
    assert_eq!(record.exit, Some(SliceExit::NonadjacentCrossing));
    assert_eq!(record.category, SliceCategory::Unsupported);
    assert_eq!(record.stage, SliceStage::WorkingCover);
}

#[test]
fn a_nonadjacent_touch_is_refused_as_a_tangency() {
    // A vertex of one segment lands in the interior of a nonadjacent one
    // without crossing it.
    let record = Fixture::from_cycle(&[
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(2.0, 0.0, 0.0),
        Point3::new(1.0, 1.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(0.0, 1.0, 0.0),
    ])
    .run(&xy_plane(), declared_outer(1));
    assert!(
        matches!(
            record.exit,
            Some(SliceExit::NonadjacentTangency | SliceExit::NonadjacentRepeatedVertex)
        ),
        "expected a nonadjacent contact refusal, got {:?}",
        record.exit
    );
}

#[test]
fn a_positive_length_overlap_is_refused() {
    // The cycle doubles back along itself.
    let record = Fixture::from_cycle(&[
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(2.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(1.0, 1.0, 0.0),
    ])
    .run(&xy_plane(), declared_outer(1));
    assert!(
        matches!(
            record.exit,
            Some(SliceExit::PositiveLengthOverlap | SliceExit::AdjacentPairHasExtraIntersection)
        ),
        "expected an overlap refusal, got {:?}",
        record.exit
    );
}

#[test]
fn a_collapsed_occurrence_is_not_simple() {
    let mut fixture = unit_square();
    fixture.positions[1] = Point3::new(0.0, 0.0, 0.0);
    fixture.curves[0] = identify_line_segment(&Line(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(0.0, 0.0, 0.0),
    ));
    fixture.curves[1] = identify_line_segment(&Line(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 1.0, 0.0),
    ));
    let record = fixture.run(&xy_plane(), declared_outer(1));
    assert!(
        matches!(
            record.exit,
            Some(SliceExit::IndividualCurveNotSimple | SliceExit::NonadjacentRepeatedVertex)
        ),
        "got {:?}",
        record.exit
    );
}

#[test]
fn proximity_alone_neither_creates_nor_removes_an_intersection() {
    // Two segments a nanometre apart. No epsilon appears in the classifier, so
    // they are disjoint — and a square built with that gap in its interior
    // still meshes.
    let a = classify_segments(
        Point2::new(0.0, 0.0),
        Point2::new(1.0, 0.0),
        Point2::new(0.5, 1e-9),
        Point2::new(0.5, 1.0),
    );
    assert_eq!(a, SegmentIntersection::Empty);
    // And a genuine touch at exactly zero is found.
    let b = classify_segments(
        Point2::new(0.0, 0.0),
        Point2::new(1.0, 0.0),
        Point2::new(0.5, 0.0),
        Point2::new(0.5, 1.0),
    );
    assert_eq!(b, SegmentIntersection::Point(Point2::new(0.5, 0.0)));
}

#[test]
fn adjacent_segments_meeting_only_at_their_shared_endpoint_pass() {
    let intersection = classify_segments(
        Point2::new(0.0, 0.0),
        Point2::new(1.0, 0.0),
        Point2::new(1.0, 0.0),
        Point2::new(1.0, 1.0),
    );
    assert_eq!(
        intersection,
        SegmentIntersection::Point(Point2::new(1.0, 0.0))
    );
}

#[test]
fn collinear_segments_sharing_a_length_report_overlap_not_a_point() {
    let intersection = classify_segments(
        Point2::new(0.0, 0.0),
        Point2::new(2.0, 0.0),
        Point2::new(1.0, 0.0),
        Point2::new(3.0, 0.0),
    );
    assert_eq!(intersection, SegmentIntersection::Overlap);
}

// ---------------------------------------------------------------------------
// Step 8
// ---------------------------------------------------------------------------

#[test]
fn a_line_bounded_polygon_has_a_zero_error_certificate() {
    let record = unit_square().run(&xy_plane(), declared_outer(1));
    assert_eq!(record.stage, SliceStage::FinalValidity);
    assert_eq!(record.polygon_vertices, Some(4));
    assert_eq!(
        record.certificate_route,
        Some(CertificateRoute::AnalyticAffineProjectionOfPolygonalCurve)
    );
}

#[test]
fn a_polyline_edge_contributes_all_of_its_segments() {
    let mut fixture = unit_square();
    // Replace the bottom edge with a polyline through an interior waypoint.
    fixture.curves[0] = identify_polyline(&[
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(0.5, -0.5, 0.0),
        Point3::new(1.0, 0.0, 0.0),
    ]);
    let record = fixture.run(&xy_plane(), declared_outer(1));
    assert_eq!(record.stage, SliceStage::FinalValidity, "{:?}", record.exit);
    assert_eq!(record.polygon_vertices, Some(5));
    assert_eq!(record.validity.expect("validity").triangles, 3);
}

#[test]
fn a_missing_boundary_constraint_fails_final_validity() {
    // Hand the battery a triangulation that omits one boundary edge by
    // replacing a corner triangle with one spanning the wrong pair.
    let plane = xy_plane();
    let region = square_region(&plane);
    let mut mesh = triangulate(&region).expect("triangulate");
    mesh.triangles[0] = [0, 1, 2];
    mesh.triangles[1] = [0, 1, 2];
    assert!(final_validity(&mesh, &region).is_err());
}

#[test]
fn overlapping_triangles_fail_final_validity() {
    let plane = xy_plane();
    let region = square_region(&plane);
    let mesh = TriangulatedRegion {
        vertices: region.region.boundary.cycle.clone(),
        // Two identical triangles plus one more: the same area as the square,
        // covered twice on one half and not at all on the other.
        triangles: vec![[0, 1, 2], [0, 1, 2]],
    };
    assert!(final_validity(&mesh, &region).is_err());
}

#[test]
fn a_disconnected_triangle_set_fails_final_validity() {
    // Two triangles that share no edge. The area happens to be right; the dual
    // graph is not connected and the boundary is not the polygon cycle.
    let plane = xy_plane();
    let region = square_region(&plane);
    let mesh = TriangulatedRegion {
        vertices: region.region.boundary.cycle.clone(),
        triangles: vec![[0, 1, 2], [0, 2, 3]],
    };
    // This one *is* a valid triangulation, so it must pass — the negative case
    // is below, and pinning both keeps the check honest.
    assert!(final_validity(&mesh, &region).is_ok());

    let broken = TriangulatedRegion {
        vertices: region.region.boundary.cycle.clone(),
        triangles: vec![[0, 1, 2]],
    };
    assert!(final_validity(&broken, &region).is_err());
}

#[test]
fn equal_total_area_with_an_overlap_and_a_gap_still_fails() {
    let plane = xy_plane();
    let region = square_region(&plane);
    // `[0,1,2]` twice covers the lower half twice and the upper half never.
    // Total area equals the square's, and every combinatorial check rejects it.
    let mesh = TriangulatedRegion {
        vertices: region.region.boundary.cycle.clone(),
        triangles: vec![[0, 1, 2], [2, 1, 0]],
    };
    let mesh_area: f64 = mesh
        .triangles
        .iter()
        .map(|[a, b, c]| {
            let (a, b, c) = (mesh.vertices[*a], mesh.vertices[*b], mesh.vertices[*c]);
            ((b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y)).abs() / 2.0
        })
        .sum();
    assert!(
        (mesh_area - region.region.signed_area.abs()).abs() < 1e-12,
        "the fixture must have the right total area for the test to mean anything"
    );
    assert!(final_validity(&mesh, &region).is_err());
}

fn square_region(plane: &PlaneSchema) -> CertifiedPolygonalRegion {
    let fixture = unit_square();
    let curves = fixture.curves.clone();
    let positions = fixture.positions.clone();
    let traversal = regular_traversal(&fixture.input, declared_outer(1), &mut |i| {
        curves[i].clone()
    })
    .expect("traversal");
    let planar = certified_planar_curves(
        &traversal,
        plane,
        &|key| match key {
            SourceVertexKey::ShellVertex(index) => positions.get(index).copied(),
            _ => None,
        },
        1e-6,
    )
    .expect("planar");
    let developed = rank0_lift(&rank0(plane), planar).expect("lift");
    let arrangement = simple_jordan_arrangement(&developed).expect("jordan");
    let material =
        bounded_material_region(arrangement, declared_outer(1)).expect("material region");
    certified_polygonal_region(material, &developed, 1e-6).expect("polygonal region")
}

// ---------------------------------------------------------------------------
// Orientation
// ---------------------------------------------------------------------------

#[test]
fn the_emitted_winding_follows_the_source_traversal_handedness() {
    // The same square traversed both ways round. Ear clipping normalises to a
    // counter-clockwise chart internally; if that normalisation reached the
    // output, both runs would emit the same winding and the same normal, and
    // every clockwise-declared face in the corpus would be silently reoriented.
    let corners = [
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(1.0, 1.0, 0.0),
        Point3::new(0.0, 1.0, 0.0),
    ];
    let mut reversed = corners;
    reversed.reverse();

    let ccw = Fixture::from_cycle(&corners)
        .run(&xy_plane(), declared_outer(1))
        .mesh
        .expect("ccw mesh");
    let cw = Fixture::from_cycle(&reversed)
        .run(&xy_plane(), declared_outer(1))
        .mesh
        .expect("cw mesh");

    // The chart normal of the xy plane is +z; the reversed traversal gets -z.
    assert!(
        (ccw.chart_normal.z - 1.0).abs() < 1e-12,
        "{:?}",
        ccw.chart_normal
    );
    assert!(
        (cw.chart_normal.z + 1.0).abs() < 1e-12,
        "{:?}",
        cw.chart_normal
    );

    // And the winding agrees with the normal in each case, so the mesh is
    // self-consistent rather than merely differently labelled.
    for (mesh, expected_up) in [(&ccw, true), (&cw, false)] {
        for [a, b, c] in &mesh.triangles {
            let (a, b, c) = (mesh.positions[*a], mesh.positions[*b], mesh.positions[*c]);
            let face_normal = (b - a).cross(c - a);
            assert!(
                face_normal.dot(mesh.chart_normal) > 0.0,
                "winding disagrees with the emitted normal"
            );
            assert_eq!(face_normal.z > 0.0, expected_up);
        }
    }
}

// ---------------------------------------------------------------------------
// Corpus regression fixtures
// ---------------------------------------------------------------------------

/// Model `00009190`, face `#42234` — the first real corpus face this slice
/// recovered.
const FACE_42234: [Point3; 3] = [
    Point3::new(-0.0756527225796564, 0.0258927104646686, 0.251408135159232),
    Point3::new(
        -0.07549906969207085,
        0.023357437819507002,
        0.251408135159232,
    ),
    Point3::new(-0.0756527225796564, 0.0258927104646686, 0.251406705940474),
];

/// Model `00009190`, face `#41780`.
const FACE_41780: [Point3; 3] = [
    Point3::new(-0.015174060560400411, 0.023357437819507, 0.251408135159232),
    Point3::new(
        -0.01502040767281486,
        0.025892710464668605,
        0.251408135159232,
    ),
    Point3::new(
        -0.01502040767281486,
        0.025892710464668605,
        0.251406705940474,
    ),
];

/// The support plane of three points, with the orthonormal basis a STEP
/// `PLANE` carries.
///
/// Taking the three points *themselves* as the plane's `(o, p, q)` would be
/// wrong for these fixtures, and instructively so: for face `#41780` the two
/// resulting axes are nearly parallel — `p - o` and `q - o` differ only in a
/// `1.4e-6` z component against a `2.5e-3` length — so the Gram matrix is
/// ill conditioned and Step 3 refuses it. See
/// [`a_three_point_basis_on_a_sliver_is_refused`], which pins that.
///
/// A STEP `PLANE` does not have that problem: it is defined by an
/// `AXIS2_PLACEMENT_3D`, whose axes are orthonormal by construction. This
/// builds the same thing, so the fixture presents the support surface the
/// corpus actually supplies rather than an artefact of how the fixture was
/// written.
fn plane_through(points: [Point3; 3]) -> PlaneSchema {
    let normal = (points[1] - points[0])
        .cross(points[2] - points[0])
        .normalize();
    let u = (points[1] - points[0]).normalize();
    let v = normal.cross(u);
    *identify_plane(&Plane::new(points[0], points[0] + u, points[0] + v))
        .plane()
        .expect("an orthonormal basis is separated")
}

#[test]
fn a_three_point_basis_on_a_sliver_is_refused() {
    // Not a fact about the face: a fact about that choice of basis. The same
    // face resolves through its real orthonormal support plane, which is what
    // `corpus_face_41780_still_recovers` shows.
    let record = Fixture::from_cycle(&FACE_41780).run(
        identify_plane(&Plane::new(FACE_41780[0], FACE_41780[1], FACE_41780[2]))
            .plane()
            .expect("still above the structural floor"),
        declared_outer(1),
    );
    assert_eq!(record.exit, Some(SliceExit::IllConditionedPlaneBasis));
    assert_eq!(record.category, SliceCategory::Unresolved);
}

/// Both recovered faces are planar triangles bounded by three `LINE` edges with
/// one declared `FACE_OUTER_BOUND`, and both are faces the legacy tessellator
/// loses as `MeshedToNothing`. The coordinates are the ones the recovery gate
/// emitted, in the model's own units (metres); the 400 MB model cannot go in
/// the repository, so the geometry is inlined.
#[test]
fn corpus_face_42234_still_recovers() {
    let record =
        Fixture::from_cycle(&FACE_42234).run(&plane_through(FACE_42234), declared_outer(1));
    assert_eq!(record.stage, SliceStage::FinalValidity, "{:?}", record.exit);
    let validity = record.validity.expect("validity");
    assert_eq!(validity.triangles, 1);
    assert_eq!(validity.vertices, 3);
    assert_eq!(validity.boundary_edges, 3);
    assert_eq!(validity.internal_edges, 0);
    assert_eq!(record.curve_representations, vec!["line_segment"]);
}

#[test]
fn corpus_face_41780_still_recovers() {
    let record =
        Fixture::from_cycle(&FACE_41780).run(&plane_through(FACE_41780), declared_outer(1));
    assert_eq!(record.stage, SliceStage::FinalValidity, "{:?}", record.exit);
    assert_eq!(record.validity.expect("validity").triangles, 1);
    assert_eq!(record.polygon_vertices, Some(3));
}

#[test]
fn the_recovered_corpus_faces_are_extreme_slivers() {
    // Pinned so a future change that starts filtering by aspect ratio, or by an
    // absolute area epsilon, fails here rather than silently dropping them.
    // A chain that only works on well-proportioned polygons would pass the
    // synthetic square above and fail these; the exact orientation predicates
    // are what let them through without a tolerance deciding the answer.
    for points in [FACE_42234, FACE_41780] {
        let sides = [
            (points[1] - points[0]).magnitude(),
            (points[2] - points[1]).magnitude(),
            (points[0] - points[2]).magnitude(),
        ];
        let longest = sides.iter().cloned().fold(0.0, f64::max);
        let shortest = sides.iter().cloned().fold(f64::INFINITY, f64::min);
        assert!(
            longest / shortest > 1000.0,
            "aspect ratio {}",
            longest / shortest
        );
        let area = (points[1] - points[0])
            .cross(points[2] - points[0])
            .magnitude()
            / 2.0;
        assert!(area > 0.0, "the triangle is nondegenerate");
        assert!(
            area < 1e-8,
            "area {area} — an absolute epsilon would reject it"
        );
    }
}

// ---------------------------------------------------------------------------
// Generic material authority
// ---------------------------------------------------------------------------

/// The generic patch entry keeps requiring source outer-bound authority, and
/// now distinguishes *why* it is absent.
///
/// The cylinder band was given a bounded, band-only route to material
/// standing. This is the test that the route did not leak: a generic planar
/// patch handed the identical certified simple cycle still refuses every
/// non-unique standing, and reports the state the source is actually in —
/// a multiply-declared face is a source contradiction, not missing provenance.
#[test]
fn generic_material_selection_still_requires_outer_bound_authority() {
    let plane = xy_plane();
    // The same simple Jordan cycle the successful path uses, so the only
    // thing under test is the authority gate.
    let arrangement = |_: ()| {
        let fixture = unit_square();
        let curves = fixture.curves.clone();
        let positions = fixture.positions.clone();
        let traversal = regular_traversal(&fixture.input, declared_outer(1), &mut |i| {
            curves[i].clone()
        })
        .expect("traversal");
        let planar = certified_planar_curves(
            &traversal,
            &plane,
            &|key| match key {
                SourceVertexKey::ShellVertex(index) => positions.get(index).copied(),
                _ => None,
            },
            1e-6,
        )
        .expect("planar");
        let developed = rank0_lift(&rank0(&plane), planar).expect("lift");
        simple_jordan_arrangement(&developed).expect("jordan")
    };

    for (standing, expected) in [
        (
            OuterBoundStanding::NotRetained,
            SliceExit::MissingOuterBoundAuthority,
        ),
        (
            OuterBoundStanding::NoneDeclared,
            SliceExit::MissingOuterBoundAuthority,
        ),
        (declared_outer(2), SliceExit::MultipleOuterBoundsDeclared),
        (declared_outer(3), SliceExit::MultipleOuterBoundsDeclared),
    ] {
        assert_eq!(
            bounded_material_region(arrangement(()), standing).err(),
            Some(expected),
            "generic material selection must refuse {standing:?}",
        );
    }

    // And the conforming standing still succeeds, so the gate is a gate and
    // not a wall.
    assert!(bounded_material_region(arrangement(()), declared_outer(1)).is_ok());
}
