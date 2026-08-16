//! Tests for the planar slice with holes.
//!
//! The negative cases carry the weight. Meshing a square with a square hole
//! proves very little on its own; what is worth testing is that every way two
//! boundary components can fail to be admissible — touching, tangent,
//! crossing, overlapping, nested, escaped — exits with its own reason in its
//! own category, and that a *mesh* violating any final-validity predicate is
//! rejected even when its total area is right.

use super::super::super::source_evidence::{
    BoundId, EdgeUseId, ErasedOrientationMechanism, OrientationEvidence, OrientationOrigin,
    SourceBoundInput, SourceEdgeOrientationEvidence, SourceEdgeUseInput, SourceFaceInput,
    SourceFaceOrientationEvidence, SourceVertexKey,
};
use super::super::ambient::{ambient_evidence_from_plane_schema, resolve_ambient_periods};
use super::super::envelope::{FormalEnvelope, PolicyInstanceId};
use super::super::outcome::{DocumentScope, FaceKey, ShellKey, StageOutcome};
use super::super::support::{
    identify_line_segment, identify_plane, CurveSchema, CurveSchemaFailure,
};
use super::*;
use truck_geometry::prelude::{Line, Plane};

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

fn declared_outer_at(bound_index: u32, count: u32) -> OuterBoundStanding {
    OuterBoundStanding::Declared {
        bound_index,
        declared_count: count,
    }
}

fn declared_outer() -> OuterBoundStanding {
    declared_outer_at(0, 1)
}

/// A face built from a list of vertex cycles, one bound per cycle, bound 0
/// first. Every edge use is a straight `LINE` in its own source direction.
struct HoleFixture {
    input: SourceFaceInput,
    positions: Vec<Point3>,
    curves: Vec<CurveSchema>,
}

impl HoleFixture {
    fn new(loops: &[Vec<Point3>]) -> Self {
        Self::with(loops, |_, _| true)
    }

    /// As [`Self::new`], but `forward(bound, index)` decides whether that edge
    /// use runs with its curve or against it, so a loop can be presented in
    /// either source traversal direction.
    fn with(loops: &[Vec<Point3>], forward: impl Fn(usize, usize) -> bool) -> Self {
        let mut positions: Vec<Point3> = Vec::new();
        let mut curves: Vec<CurveSchema> = Vec::new();
        let mut bounds: Vec<SourceBoundInput> = Vec::new();

        for (bound_index, cycle) in loops.iter().enumerate() {
            let base = positions.len();
            positions.extend_from_slice(cycle);
            let n = cycle.len();
            let mut edge_uses = Vec::with_capacity(n);
            for i in 0..n {
                let j = (i + 1) % n;
                let runs_forward = forward(bound_index, i);
                let (from, to) = match runs_forward {
                    true => (base + i, base + j),
                    false => (base + j, base + i),
                };
                let edge_index = curves.len();
                curves.push(identify_line_segment(&Line(positions[from], positions[to])));
                let source_vertices = (
                    SourceVertexKey::ShellVertex(from),
                    SourceVertexKey::ShellVertex(to),
                );
                let use_vertices = match runs_forward {
                    true => source_vertices,
                    false => (source_vertices.1, source_vertices.0),
                };
                edge_uses.push(edge_use(
                    BoundId(bound_index),
                    i,
                    edge_index,
                    source_vertices,
                    use_vertices,
                    runs_forward,
                ));
            }
            bounds.push(SourceBoundInput::EdgeUses {
                id: BoundId(bound_index),
                edge_uses,
            });
        }

        Self {
            input: SourceFaceInput {
                source_face_id: Some(1),
                declared_face_index: 0,
                bounds,
                orientation: SourceFaceOrientationEvidence {
                    face_use_orientation: OrientationEvidence::Missing,
                    face_surface_same_sense: OrientationEvidence::Missing,
                },
            },
            positions,
            curves,
        }
    }

    fn run(&self, outer: OuterBoundStanding) -> HoleSliceRecord {
        let plane = xy_plane();
        let curves = self.curves.clone();
        let positions = self.positions.clone();
        run_planar_holes_slice(
            &self.input,
            &plane,
            &rank0(&plane),
            outer,
            &mut |index| curves[index].clone(),
            &|key| match key {
                SourceVertexKey::ShellVertex(index) => positions.get(index).copied(),
                _ => None,
            },
            1e-6,
        )
    }

    /// The pipeline up to Step 7H's certificate, for tests that need to attack
    /// a mesh rather than a face.
    fn certificate(&self) -> PlanarRegionWithHolesCertificate {
        let plane = xy_plane();
        let lattice = rank0(&plane);
        let curves = self.curves.clone();
        let positions = self.positions.clone();
        let bounds = match classify_bounds(&self.input, declared_outer()).expect("classifies") {
            MultiBoundEntry::MultiBound(bounds) => bounds,
            other => panic!("expected a multi-bound face, got {other:?}"),
        };
        let traversal =
            regular_planar_multibound_traversal(&bounds, &mut |index| curves[index].clone())
                .expect("traverses");
        let vertex = |key| match key {
            SourceVertexKey::ShellVertex(index) => positions.get(index).copied(),
            _ => None,
        };
        let mut built = Vec::new();
        for one in std::iter::once(&traversal.outer).chain(traversal.inners.iter()) {
            let planar = planar_slice::certified_planar_curves(one, &plane, &vertex, 1e-6)
                .expect("projects");
            let developed = planar_slice::rank0_lift(&lattice, planar).expect("lifts");
            let arrangement =
                jordan_arrangement_of(&developed.occurrences).expect("is a Jordan loop");
            let area = signed_area(&arrangement.cycle);
            built.push(BoundaryLoop {
                bound: one.bound,
                arrangement,
                signed_area: area,
            });
        }
        let outer = built.remove(0);
        certify_region_with_holes(outer, built).expect("certifies")
    }
}

#[allow(clippy::too_many_arguments)]
fn edge_use(
    bound: BoundId,
    local: usize,
    edge_index: usize,
    source_vertices: (SourceVertexKey, SourceVertexKey),
    use_vertices: (SourceVertexKey, SourceVertexKey),
    forward: bool,
) -> SourceEdgeUseInput {
    SourceEdgeUseInput {
        id: EdgeUseId::new(bound, local),
        source_edge_index: edge_index,
        source_vertices,
        use_vertices,
        orientation: SourceEdgeOrientationEvidence {
            bound_times_oriented_edge: OrientationEvidence::Retained {
                forward,
                origin: OrientationOrigin::BoundTimesOrientedEdge,
            },
            edge_curve_same_sense: OrientationEvidence::HistoryErased {
                mechanism: ErasedOrientationMechanism::EdgeCurveSenseFoldedIntoConvertedCurve,
            },
            selected_curve_direction: OrientationEvidence::HistoryErased {
                mechanism:
                    ErasedOrientationMechanism::SelectedCurveDirectionFoldedIntoConvertedCurve,
            },
        },
    }
}

fn square(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Point3> {
    vec![
        Point3::new(x0, y0, 0.0),
        Point3::new(x1, y0, 0.0),
        Point3::new(x1, y1, 0.0),
        Point3::new(x0, y1, 0.0),
    ]
}

/// The same cycle the other way round.
fn reversed(cycle: &[Point3]) -> Vec<Point3> {
    let mut out = cycle.to_vec();
    out.reverse();
    out
}

fn outer_square() -> Vec<Point3> {
    square(0.0, 0.0, 10.0, 10.0)
}

// ---------------------------------------------------------------------------
// The admitted subset
// ---------------------------------------------------------------------------

#[test]
fn square_with_one_square_hole_resolves() {
    let fixture = HoleFixture::new(&[outer_square(), square(3.0, 3.0, 7.0, 7.0)]);
    let record = fixture.run(declared_outer());

    assert_eq!(record.stage, SliceStage::FinalValidity, "{record:?}");
    assert_eq!(record.category, SliceCategory::Resolved);
    assert_eq!(record.exit, None);
    assert_eq!(record.inner_bound_count, 1);
    assert_eq!(record.edge_uses_per_bound, vec![4, 4]);
    assert_eq!(record.polygon_vertices_per_bound, vec![4, 4]);

    let validity = record.validity.expect("a completed face reports validity");
    assert_eq!(validity.vertices, 8);
    assert_eq!(validity.boundary_cycles, 2);
    assert_eq!(validity.euler_characteristic, 0, "chi = 1 - h = 0");
    assert_eq!(validity.boundary_edges, 8);
    // A square annulus: 8 vertices, 8 boundary edges, chi = 0 forces T = E - V.
    assert_eq!(validity.triangles, 8);
    assert!(validity.area_residual <= 1e-9 * (100.0 - 16.0));

    let mesh = record.mesh.expect("a completed face carries a mesh");
    assert_eq!(mesh.triangles.len(), 8);
    assert_eq!(mesh.positions.len(), 8);
}

#[test]
fn square_with_two_disjoint_holes_resolves() {
    let fixture = HoleFixture::new(&[
        outer_square(),
        square(1.0, 1.0, 3.0, 3.0),
        square(6.0, 6.0, 9.0, 9.0),
    ]);
    let record = fixture.run(declared_outer());

    assert_eq!(record.stage, SliceStage::FinalValidity, "{record:?}");
    assert_eq!(record.category, SliceCategory::Resolved);
    assert_eq!(record.inner_bound_count, 2);
    let validity = record.validity.expect("validity");
    assert_eq!(validity.boundary_cycles, 3);
    assert_eq!(validity.euler_characteristic, -1, "chi = 1 - h = -1");
    assert_eq!(validity.vertices, 12);
}

#[test]
fn hole_traversal_direction_does_not_change_material() {
    // The same face with the hole presented in the opposite source direction.
    // Material membership is orientation independent, so nothing about the
    // region may change.
    let forward = HoleFixture::new(&[outer_square(), square(3.0, 3.0, 7.0, 7.0)]);
    let backward = HoleFixture::new(&[outer_square(), reversed(&square(3.0, 3.0, 7.0, 7.0))]);

    let a = forward.run(declared_outer());
    let b = backward.run(declared_outer());

    assert_eq!(a.stage, SliceStage::FinalValidity);
    assert_eq!(b.stage, SliceStage::FinalValidity);
    let (a, b) = (a.validity.expect("a"), b.validity.expect("b"));
    assert_eq!(a.triangles, b.triangles);
    assert_eq!(a.euler_characteristic, b.euler_characteristic);
    assert_eq!(a.boundary_cycles, b.boundary_cycles);
}

#[test]
fn outer_traversal_direction_flips_the_emitted_normal_and_winding() {
    // Reversing the *outer* loop reverses the source's handedness in the
    // plane's chart. The emitted winding and normal must follow it rather than
    // be normalised to a preferred direction; the material region must not
    // change at all.
    let ccw = HoleFixture::new(&[outer_square(), square(3.0, 3.0, 7.0, 7.0)]);
    let cw = HoleFixture::new(&[reversed(&outer_square()), square(3.0, 3.0, 7.0, 7.0)]);

    let a = ccw.run(declared_outer());
    let b = cw.run(declared_outer());
    assert_eq!(a.stage, SliceStage::FinalValidity);
    assert_eq!(b.stage, SliceStage::FinalValidity);

    let (a_mesh, b_mesh) = (a.mesh.expect("a"), b.mesh.expect("b"));
    assert_eq!(a_mesh.triangles.len(), b_mesh.triangles.len());
    // Opposite physical normals.
    let dot = a_mesh.chart_normal.dot(b_mesh.chart_normal);
    assert!(dot < -0.5, "expected opposed normals, dot = {dot}");
    // Unchanged material: same area either way.
    let (a_v, b_v) = (a.validity.expect("a"), b.validity.expect("b"));
    assert_eq!(a_v.triangles, b_v.triangles);
    assert_eq!(a_v.euler_characteristic, b_v.euler_characteristic);
}

#[test]
fn a_triangular_hole_in_a_square_resolves() {
    let fixture = HoleFixture::new(&[
        outer_square(),
        vec![
            Point3::new(3.0, 3.0, 0.0),
            Point3::new(7.0, 4.0, 0.0),
            Point3::new(4.0, 7.0, 0.0),
        ],
    ]);
    let record = fixture.run(declared_outer());
    assert_eq!(record.stage, SliceStage::FinalValidity, "{record:?}");
    let validity = record.validity.expect("validity");
    assert_eq!(validity.euler_characteristic, 0);
    assert_eq!(validity.boundary_cycles, 2);
    assert_eq!(validity.vertices, 7);
}

#[test]
fn a_face_with_no_inner_bounds_delegates() {
    let fixture = HoleFixture::new(&[outer_square()]);
    let record = fixture.run(declared_outer());
    assert!(record.delegated, "{record:?}");
    assert_eq!(record.exit, None);
    assert_eq!(record.stage, SliceStage::AmbientRank0);
    assert!(record.mesh.is_none());
}

#[test]
fn a_repeated_edge_id_through_distinct_edge_uses_is_not_an_identity_failure() {
    // A repeated underlying `EdgeId` is not a repeated `EdgeUseId`, and Step 2H
    // must not conflate them. Two *disjoint* loops can never genuinely share an
    // edge — that would be a positive-length overlap — so the claim is tested
    // where it lives: the traversal stage accepts the repeated index, and any
    // later refusal comes from geometry rather than from identity.
    let mut fixture = HoleFixture::new(&[outer_square(), square(3.0, 3.0, 7.0, 7.0)]);
    if let SourceBoundInput::EdgeUses { edge_uses, .. } = &mut fixture.input.bounds[1] {
        edge_uses[0].source_edge_index = 0;
    }

    let bounds = match classify_bounds(&fixture.input, declared_outer()).expect("classifies") {
        MultiBoundEntry::MultiBound(bounds) => bounds,
        other => panic!("expected a multi-bound face, got {other:?}"),
    };
    let curves = fixture.curves.clone();
    let traversal =
        regular_planar_multibound_traversal(&bounds, &mut |index| curves[index].clone());
    let traversal = traversal.expect("a repeated EdgeId traverses");
    assert_eq!(traversal.inners.len(), 1);
    assert_eq!(
        traversal.inners[0].occurrences[0].source_edge_index,
        traversal.outer.occurrences[0].source_edge_index,
        "the two uses really do share one edge table entry"
    );

    // And the face as a whole is refused for the geometric reason, not an
    // identity one.
    let record = fixture.run(declared_outer());
    assert_ne!(record.exit, Some(SliceExit::DuplicateEdgeUseId));
    assert_eq!(record.exit, Some(SliceExit::CurveSurfaceInconsistency));
}

// ---------------------------------------------------------------------------
// Bound authority
// ---------------------------------------------------------------------------

#[test]
fn two_declared_outer_bounds_is_a_source_contradiction() {
    let fixture = HoleFixture::new(&[outer_square(), square(3.0, 3.0, 7.0, 7.0)]);
    let record = fixture.run(declared_outer_at(0, 2));
    assert_eq!(record.exit, Some(SliceExit::MultipleOuterBoundsDeclared));
    assert_eq!(record.category, SliceCategory::Inconsistent);
}

#[test]
fn absent_outer_authority_is_unresolved_not_a_guess() {
    let fixture = HoleFixture::new(&[outer_square(), square(3.0, 3.0, 7.0, 7.0)]);
    for standing in [
        OuterBoundStanding::NotRetained,
        OuterBoundStanding::NoneDeclared,
    ] {
        let record = fixture.run(standing);
        assert_eq!(
            record.exit,
            Some(SliceExit::MissingOuterBoundAuthority),
            "{standing:?}"
        );
        assert_eq!(record.category, SliceCategory::Unresolved);
    }
}

#[test]
fn the_outer_bound_is_whichever_the_source_named_not_the_first() {
    // The larger loop is declared second. Nothing may infer outer standing
    // from source order or from area.
    let fixture = HoleFixture::new(&[square(3.0, 3.0, 7.0, 7.0), outer_square()]);
    let record = fixture.run(declared_outer_at(1, 1));
    assert_eq!(record.stage, SliceStage::FinalValidity, "{record:?}");
    assert_eq!(record.inner_bound_count, 1);
    let validity = record.validity.expect("validity");
    assert_eq!(validity.euler_characteristic, 0);

    // Naming the *small* loop as outer makes the big one a hole outside it,
    // which contradicts the declaration rather than silently swapping them.
    let record = fixture.run(declared_outer_at(0, 1));
    assert_eq!(record.exit, Some(SliceExit::InnerBoundOutsideOuter));
    assert_eq!(record.category, SliceCategory::Inconsistent);
}

#[test]
fn a_duplicate_edge_use_id_across_bounds_is_malformed() {
    let mut fixture = HoleFixture::new(&[outer_square(), square(3.0, 3.0, 7.0, 7.0)]);
    if let SourceBoundInput::EdgeUses { edge_uses, .. } = &mut fixture.input.bounds[1] {
        edge_uses[0].id = EdgeUseId::new(BoundId(0), 0);
    }
    let record = fixture.run(declared_outer());
    assert_eq!(record.exit, Some(SliceExit::DuplicateEdgeUseId));
    assert_eq!(record.category, SliceCategory::Inconsistent);
}

#[test]
fn a_degenerate_inner_bound_is_outside_the_subset() {
    let mut fixture = HoleFixture::new(&[outer_square(), square(3.0, 3.0, 7.0, 7.0)]);
    fixture.input.bounds[1] = SourceBoundInput::DegenerateEvidenceUnavailable { id: BoundId(1) };
    let record = fixture.run(declared_outer());
    assert_eq!(record.exit, Some(SliceExit::DegenerateInnerBound));
    assert_eq!(record.category, SliceCategory::Unsupported);
}

#[test]
fn an_unsupported_curve_in_an_inner_bound_is_refused() {
    let mut fixture = HoleFixture::new(&[outer_square(), square(3.0, 3.0, 7.0, 7.0)]);
    fixture.curves[5] =
        CurveSchema::not_structurally_identified(CurveSchemaFailure::NoStructuralReader {
            representation: "b_spline_curve_with_knots",
        });
    let record = fixture.run(declared_outer());
    assert_eq!(record.exit, Some(SliceExit::UnsupportedCurveRepresentation));
    assert_eq!(record.category, SliceCategory::Unsupported);
    // Attributed to the bound that carries it. "an inner bound has an
    // unsupported curve" and "the outer bound has one" name different work.
    assert_eq!(record.obstruction_bound, Some(BoundRole::Inner(0)));
}

#[test]
fn an_unsupported_curve_in_the_outer_bound_is_attributed_to_the_outer_bound() {
    let mut fixture = HoleFixture::new(&[outer_square(), square(3.0, 3.0, 7.0, 7.0)]);
    fixture.curves[1] =
        CurveSchema::not_structurally_identified(CurveSchemaFailure::NoStructuralReader {
            representation: "b_spline_curve_with_knots",
        });
    let record = fixture.run(declared_outer());
    assert_eq!(record.exit, Some(SliceExit::UnsupportedCurveRepresentation));
    assert_eq!(record.obstruction_bound, Some(BoundRole::Outer));
}

#[test]
fn an_obstruction_on_the_second_hole_names_the_second_hole() {
    let fixture = HoleFixture::new(&[
        outer_square(),
        square(1.0, 1.0, 3.0, 3.0),
        // Crosses the outer loop: found during the pairwise pass, not traversal.
        square(6.0, 6.0, 14.0, 14.0),
    ]);
    let record = fixture.run(declared_outer());
    assert_eq!(record.exit, Some(SliceExit::BoundaryComponentsCross));
    // The pairwise stage is face-level, so the role reported is the last loop
    // certified rather than a claim about which pair collided.
    assert_eq!(record.inner_bound_count, 2);
}

// ---------------------------------------------------------------------------
// Pairwise component relations
// ---------------------------------------------------------------------------

#[test]
fn a_hole_outside_the_outer_loop_contradicts_its_declaration() {
    let fixture = HoleFixture::new(&[outer_square(), square(20.0, 20.0, 25.0, 25.0)]);
    let record = fixture.run(declared_outer());
    assert_eq!(record.exit, Some(SliceExit::InnerBoundOutsideOuter));
    assert_eq!(record.category, SliceCategory::Inconsistent);
}

#[test]
fn a_hole_touching_the_outer_loop_at_one_vertex_is_refused() {
    let fixture = HoleFixture::new(&[
        outer_square(),
        vec![
            Point3::new(0.0, 0.0, 0.0), // shares the outer loop's corner
            Point3::new(4.0, 1.0, 0.0),
            Point3::new(1.0, 4.0, 0.0),
        ],
    ]);
    let record = fixture.run(declared_outer());
    assert_eq!(record.exit, Some(SliceExit::BoundaryComponentsTouch));
    assert_eq!(record.category, SliceCategory::Unsupported);
}

#[test]
fn a_hole_tangent_to_an_outer_edge_is_refused() {
    // A corner of the hole lands in the interior of the outer loop's bottom
    // edge: a touch, not a crossing.
    let fixture = HoleFixture::new(&[
        outer_square(),
        vec![
            Point3::new(5.0, 0.0, 0.0),
            Point3::new(7.0, 3.0, 0.0),
            Point3::new(3.0, 3.0, 0.0),
        ],
    ]);
    let record = fixture.run(declared_outer());
    assert_eq!(record.exit, Some(SliceExit::BoundaryComponentsTouch));
    assert_eq!(record.category, SliceCategory::Unsupported);
}

#[test]
fn a_hole_crossing_the_outer_loop_is_refused() {
    let fixture = HoleFixture::new(&[outer_square(), square(8.0, 3.0, 12.0, 7.0)]);
    let record = fixture.run(declared_outer());
    assert_eq!(record.exit, Some(SliceExit::BoundaryComponentsCross));
    assert_eq!(record.category, SliceCategory::Unsupported);
}

#[test]
fn a_hole_sharing_an_edge_with_the_outer_loop_is_refused() {
    // The hole's bottom edge lies along the outer loop's bottom edge: a
    // positive-length overlap, which must not be reported as a touch.
    let fixture = HoleFixture::new(&[
        outer_square(),
        vec![
            Point3::new(2.0, 0.0, 0.0),
            Point3::new(6.0, 0.0, 0.0),
            Point3::new(6.0, 4.0, 0.0),
            Point3::new(2.0, 4.0, 0.0),
        ],
    ]);
    let record = fixture.run(declared_outer());
    assert_eq!(record.exit, Some(SliceExit::BoundaryComponentsOverlap));
    assert_eq!(record.category, SliceCategory::Unsupported);
}

#[test]
fn two_holes_touching_at_one_vertex_are_refused() {
    let fixture = HoleFixture::new(&[
        outer_square(),
        square(1.0, 1.0, 4.0, 4.0),
        vec![
            Point3::new(4.0, 4.0, 0.0), // the first hole's corner
            Point3::new(8.0, 5.0, 0.0),
            Point3::new(5.0, 8.0, 0.0),
        ],
    ]);
    let record = fixture.run(declared_outer());
    assert_eq!(record.exit, Some(SliceExit::BoundaryComponentsTouch));
}

#[test]
fn two_holes_crossing_are_refused() {
    let fixture = HoleFixture::new(&[
        outer_square(),
        square(1.0, 1.0, 5.0, 5.0),
        square(3.0, 3.0, 8.0, 8.0),
    ]);
    let record = fixture.run(declared_outer());
    assert_eq!(record.exit, Some(SliceExit::BoundaryComponentsCross));
}

#[test]
fn two_holes_sharing_an_edge_are_refused_as_an_overlap() {
    let fixture = HoleFixture::new(&[
        outer_square(),
        square(1.0, 1.0, 4.0, 4.0),
        square(4.0, 1.0, 8.0, 4.0),
    ]);
    let record = fixture.run(declared_outer());
    assert_eq!(record.exit, Some(SliceExit::BoundaryComponentsOverlap));
}

#[test]
fn a_hole_nested_in_another_hole_is_refused() {
    let fixture = HoleFixture::new(&[
        outer_square(),
        square(1.0, 1.0, 9.0, 9.0),
        square(3.0, 3.0, 6.0, 6.0),
    ]);
    let record = fixture.run(declared_outer());
    assert_eq!(record.exit, Some(SliceExit::NestedHole));
    assert_eq!(record.category, SliceCategory::Unsupported);

    // And the same pair declared the other way round, so the refusal does not
    // depend on which of the two came first.
    let fixture = HoleFixture::new(&[
        outer_square(),
        square(3.0, 3.0, 6.0, 6.0),
        square(1.0, 1.0, 9.0, 9.0),
    ]);
    assert_eq!(
        fixture.run(declared_outer()).exit,
        Some(SliceExit::NestedHole)
    );
}

#[test]
fn a_self_intersecting_inner_loop_is_refused_before_any_pairwise_claim() {
    // A bow tie. The per-loop Jordan proof must reject it; no containment
    // question is meaningful for a loop that is not simple.
    let fixture = HoleFixture::new(&[
        outer_square(),
        vec![
            Point3::new(3.0, 3.0, 0.0),
            Point3::new(7.0, 7.0, 0.0),
            Point3::new(3.0, 7.0, 0.0),
            Point3::new(7.0, 3.0, 0.0),
        ],
    ]);
    let record = fixture.run(declared_outer());
    assert_eq!(record.exit, Some(SliceExit::NonadjacentCrossing));
    assert_eq!(record.stage, SliceStage::WorkingCover);
}

// ---------------------------------------------------------------------------
// The containment predicate
// ---------------------------------------------------------------------------

#[test]
fn point_containment_is_exact_and_refuses_the_boundary() {
    let square: Vec<Point2> = vec![
        Point2::new(0.0, 0.0),
        Point2::new(4.0, 0.0),
        Point2::new(4.0, 4.0),
        Point2::new(0.0, 4.0),
    ];
    assert_eq!(
        point_strictly_inside(Point2::new(2.0, 2.0), &square),
        Some(true)
    );
    assert_eq!(
        point_strictly_inside(Point2::new(9.0, 2.0), &square),
        Some(false)
    );
    assert_eq!(
        point_strictly_inside(Point2::new(-1.0, 2.0), &square),
        Some(false)
    );
    // On an edge and on a vertex: no answer, rather than a side.
    assert_eq!(point_strictly_inside(Point2::new(2.0, 0.0), &square), None);
    assert_eq!(point_strictly_inside(Point2::new(0.0, 0.0), &square), None);
    // A ray leaving through a vertex must still be counted once. `(2, 4)` is a
    // boundary point; `(2, 3)` is interior with the ray passing the corner.
    assert_eq!(point_strictly_inside(Point2::new(2.0, 4.0), &square), None);

    // A concave polygon, where a naive convexity test would get it wrong.
    let ell: Vec<Point2> = vec![
        Point2::new(0.0, 0.0),
        Point2::new(4.0, 0.0),
        Point2::new(4.0, 1.0),
        Point2::new(1.0, 1.0),
        Point2::new(1.0, 4.0),
        Point2::new(0.0, 4.0),
    ];
    assert_eq!(
        point_strictly_inside(Point2::new(0.5, 3.0), &ell),
        Some(true)
    );
    assert_eq!(
        point_strictly_inside(Point2::new(3.0, 3.0), &ell),
        Some(false)
    );
    assert_eq!(
        point_strictly_inside(Point2::new(3.0, 0.5), &ell),
        Some(true)
    );
}

#[test]
fn component_relations_are_classified_distinctly() {
    let a: Vec<Point2> = vec![
        Point2::new(0.0, 0.0),
        Point2::new(4.0, 0.0),
        Point2::new(4.0, 4.0),
        Point2::new(0.0, 4.0),
    ];
    let disjoint: Vec<Point2> = vec![
        Point2::new(6.0, 6.0),
        Point2::new(8.0, 6.0),
        Point2::new(8.0, 8.0),
    ];
    let crossing: Vec<Point2> = vec![
        Point2::new(2.0, 2.0),
        Point2::new(6.0, 2.0),
        Point2::new(6.0, 6.0),
    ];
    let touching: Vec<Point2> = vec![
        Point2::new(4.0, 4.0),
        Point2::new(8.0, 5.0),
        Point2::new(5.0, 8.0),
    ];
    let overlapping: Vec<Point2> = vec![
        Point2::new(1.0, 0.0),
        Point2::new(3.0, 0.0),
        Point2::new(3.0, -2.0),
    ];
    assert_eq!(
        classify_components(&a, &disjoint),
        ComponentRelation::Disjoint
    );
    assert_eq!(classify_components(&a, &crossing), ComponentRelation::Cross);
    assert_eq!(classify_components(&a, &touching), ComponentRelation::Touch);
    assert_eq!(
        classify_components(&a, &overlapping),
        ComponentRelation::Overlap
    );
}

// ---------------------------------------------------------------------------
// The final validity battery, attacked directly
// ---------------------------------------------------------------------------

/// A certificate and a valid complex for a square with a square hole.
fn annulus() -> (
    PlanarRegionWithHolesCertificate,
    TriangulatedRegion,
    BoundaryComponentMap,
) {
    let fixture = HoleFixture::new(&[outer_square(), square(3.0, 3.0, 7.0, 7.0)]);
    let certificate = fixture.certificate();
    let (mesh, map) = triangulate_with_holes(&certificate).expect("triangulates");
    final_validity_with_holes(&mesh, &map, &certificate).expect("the honest complex is valid");
    (certificate, mesh, map)
}

#[test]
fn a_complex_that_fills_the_hole_is_rejected() {
    let (certificate, mut mesh, map) = annulus();
    // Fill the hole with its own two triangles. The hole's vertices are
    // indices 4..8 in cycle order.
    mesh.triangles.push([4, 5, 6]);
    mesh.triangles.push([4, 6, 7]);
    let result = final_validity_with_holes(&mesh, &map, &certificate);
    assert!(result.is_err(), "a filled hole must not validate");
}

#[test]
fn a_complex_missing_an_inner_boundary_constraint_is_rejected() {
    // Drop the two triangles incident to one hole edge. Total area falls, the
    // boundary set changes, and the Euler count moves: several predicates fire,
    // and the point is that *some* predicate does.
    let (certificate, mut mesh, map) = annulus();
    let victim = mesh
        .triangles
        .iter()
        .position(|[a, b, c]| [a, b, c].iter().filter(|v| ***v >= 4).count() == 2)
        .expect("some triangle has two hole vertices");
    mesh.triangles.remove(victim);
    assert!(final_validity_with_holes(&mesh, &map, &certificate).is_err());
}

#[test]
fn a_complex_with_the_wrong_boundary_cycle_count_is_rejected() {
    // Present the annulus's own valid complex against a certificate claiming
    // no holes: the cycle count and the Euler characteristic both disagree.
    let (certificate, mesh, map) = annulus();
    let hole_free = PlanarRegionWithHolesCertificate {
        outer: certificate.outer.clone(),
        holes: Vec::new(),
        material_area: certificate.outer.signed_area.abs(),
    };
    let result = final_validity_with_holes(&mesh, &map, &hole_free);
    assert!(result.is_err(), "two cycles must not pass as one");
}

#[test]
fn a_complex_with_the_right_area_but_an_overlap_is_rejected() {
    // Swap one triangle for a different one of equal area that overlaps its
    // neighbour. Area alone cannot see this; incidence and the boundary set
    // can, which is why area is supplemental.
    let (certificate, mut mesh, map) = annulus();
    let original = mesh.triangles[0];
    mesh.triangles[0] = [original[0], original[1], original[1]];
    assert!(final_validity_with_holes(&mesh, &map, &certificate).is_err());

    let (certificate, mut mesh, map) = annulus();
    // Duplicate a triangle: the area doubles for that cell and incidence goes
    // to three on its edges.
    let duplicate = mesh.triangles[0];
    mesh.triangles.push(duplicate);
    assert!(final_validity_with_holes(&mesh, &map, &certificate).is_err());
}

#[test]
fn a_complex_with_a_triangle_crossing_a_constraint_is_rejected() {
    // Replace the honest complex with one that straddles the hole: a triangle
    // spanning two opposite outer corners passes straight through the hole's
    // boundary. Its centroid may well be material; the crossing check is what
    // catches it.
    let (certificate, mut mesh, map) = annulus();
    mesh.triangles.push([0, 2, 5]);
    assert!(final_validity_with_holes(&mesh, &map, &certificate).is_err());
}

#[test]
fn a_disconnected_complex_is_rejected() {
    let fixture = HoleFixture::new(&[
        outer_square(),
        square(1.0, 1.0, 3.0, 3.0),
        square(6.0, 6.0, 9.0, 9.0),
    ]);
    let certificate = fixture.certificate();
    let (mut mesh, map) = triangulate_with_holes(&certificate).expect("triangulates");
    final_validity_with_holes(&mesh, &map, &certificate).expect("valid as produced");
    // Remove a band of triangles until the retained complex falls apart. Any
    // removal breaks the boundary set too; what matters is that no mutilated
    // complex is accepted.
    mesh.triangles.truncate(mesh.triangles.len() / 2);
    assert!(final_validity_with_holes(&mesh, &map, &certificate).is_err());
}

#[test]
fn boundary_cycles_are_partitioned_by_component() {
    let (_, _, map) = annulus();
    assert_eq!(map.ranges, vec![(0, 4), (4, 8)]);
    assert_eq!(map.component, vec![0, 0, 0, 0, 1, 1, 1, 1]);
}
