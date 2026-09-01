//! The two facts this slice exists to establish, and nothing else: a band
//! with two certified-distinct essential circles reaches a validated annular
//! mesh, and two coincident complete circles are classified as one carrier
//! and never reach the realizer.

use super::*;
use crate::formal::curve_witness::CompleteCirclePlacement;
use crate::formal::cylinder::{identify_cylinder, CylinderIdentification};
use crate::formal::support::identify_line_segment;
use crate::source_evidence::{
    ErasedOrientationMechanism, OrientationEvidence, OrientationOrigin,
    SourceEdgeOrientationEvidence, SourceEdgeUseInput, SourceFaceOrientationEvidence,
};
use truck_geometry::prelude::{InnerSpace, Line, RevolutedCurve, Vector3};

fn z_cylinder(radius: f64, height: f64) -> CertifiedEmbeddedCylinder {
    let revo = RevolutedCurve::by_revolution(
        Line(
            Point3::new(radius, 0.0, 0.0),
            Point3::new(radius, 0.0, height),
        ),
        Point3::new(0.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
    );
    match identify_cylinder(&revo) {
        CylinderIdentification::Cylinder(cylinder) => cylinder,
        other => panic!("expected a certified cylinder, got {other:?}"),
    }
}

fn on_cylinder(schema: &CylinderSchema, z: f64, theta: f64) -> Point3 {
    schema.origin()
        + z * schema.axis()
        + schema.radius().get() * theta.cos() * schema.radial_x()
        + schema.radius().get() * theta.sin() * schema.radial_y()
}

/// One edge use, running forward over its own curve, between two shell
/// vertices. Every fixture here declares the composed sense explicitly rather
/// than leaving it erased, so the traversal stage has the authority it
/// requires and the test exercises the band stages rather than that gate.
fn edge_use(
    bound: BoundId,
    index: usize,
    edge: usize,
    from: usize,
    to: usize,
) -> SourceEdgeUseInput {
    SourceEdgeUseInput {
        id: EdgeUseId::new(bound, index),
        source_edge_index: edge,
        source_vertices: (
            SourceVertexKey::ShellVertex(from),
            SourceVertexKey::ShellVertex(to),
        ),
        use_vertices: (
            SourceVertexKey::ShellVertex(from),
            SourceVertexKey::ShellVertex(to),
        ),
        orientation: SourceEdgeOrientationEvidence {
            bound_times_oriented_edge: OrientationEvidence::Retained {
                forward: true,
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

fn declared_outer() -> OuterBoundStanding {
    OuterBoundStanding::Declared {
        bound_index: 0,
        declared_count: 1,
    }
}

/// A two-bound face on a cylinder. Each bound is a complete circle cut into
/// two semicircular arcs; `sweeps` gives each edge's own signed parameter
/// interval, so a bound sweeping `+PI` twice winds one way and one sweeping
/// `-PI` twice winds the other.
struct Fixture {
    cylinder: CertifiedEmbeddedCylinder,
    input: SourceFaceInput,
    vertices: Vec<Point3>,
    sweeps: Vec<f64>,
}

impl Fixture {
    /// `levels` are the two circles' axial coordinates; `windings` their two
    /// signed turn directions.
    fn new(radius: f64, levels: [f64; 2], windings: [f64; 2]) -> Self {
        let cylinder = z_cylinder(radius, 5.0);
        let schema = cylinder.schema().clone();
        let mut vertices = Vec::new();
        let mut bounds = Vec::new();
        let mut sweeps = Vec::new();
        for (bound_index, (level, winding)) in levels.iter().zip(windings).enumerate() {
            let bound = BoundId(bound_index);
            let base = vertices.len();
            // Two vertices half a turn apart, in the winding's own direction.
            vertices.push(on_cylinder(&schema, *level, 0.0));
            vertices.push(on_cylinder(&schema, *level, winding * std::f64::consts::PI));
            let edges = [sweeps.len(), sweeps.len() + 1];
            sweeps.push(winding * std::f64::consts::PI);
            sweeps.push(winding * std::f64::consts::PI);
            bounds.push(SourceBoundInput::EdgeUses {
                id: bound,
                edge_uses: vec![
                    edge_use(bound, 0, edges[0], base, base + 1),
                    edge_use(bound, 1, edges[1], base + 1, base),
                ],
            });
        }
        let input = SourceFaceInput {
            source_face_id: Some(7),
            declared_face_index: 0,
            bounds,
            orientation: SourceFaceOrientationEvidence {
                face_use_orientation: OrientationEvidence::Missing,
                face_surface_same_sense: OrientationEvidence::Missing,
            },
        };
        Self {
            cylinder,
            input,
            vertices,
            sweeps,
        }
    }

    /// Two bounds sharing one circle, written over the *same* shell vertices.
    /// The duplicated representation is what the corroborating source-cycle
    /// check sees; the carrier verdict itself is decided on the two complete
    /// circles, and the test checks that separately against a fixture whose
    /// two coincident circles are written over distinct vertices.
    fn coincident(radius: f64, level: f64) -> Self {
        let cylinder = z_cylinder(radius, 5.0);
        let schema = cylinder.schema().clone();
        let vertices = vec![
            on_cylinder(&schema, level, 0.0),
            on_cylinder(&schema, level, std::f64::consts::PI),
        ];
        let sweeps = vec![
            std::f64::consts::PI,
            std::f64::consts::PI,
            -std::f64::consts::PI,
            -std::f64::consts::PI,
        ];
        let bounds = vec![
            SourceBoundInput::EdgeUses {
                id: BoundId(0),
                edge_uses: vec![
                    edge_use(BoundId(0), 0, 0, 0, 1),
                    edge_use(BoundId(0), 1, 1, 1, 0),
                ],
            },
            // The same circle, traversed the other way round, over the same
            // two vertices.
            SourceBoundInput::EdgeUses {
                id: BoundId(1),
                edge_uses: vec![
                    edge_use(BoundId(1), 0, 2, 0, 1),
                    edge_use(BoundId(1), 1, 3, 1, 0),
                ],
            },
        ];
        let input = SourceFaceInput {
            source_face_id: Some(8),
            declared_face_index: 0,
            bounds,
            orientation: SourceFaceOrientationEvidence {
                face_use_orientation: OrientationEvidence::Missing,
                face_surface_same_sense: OrientationEvidence::Missing,
            },
        };
        Self {
            cylinder,
            input,
            vertices,
            sweeps,
        }
    }

    fn family_of(&self) -> impl Fn(EdgeUseId) -> SourceCurveFamily + '_ {
        move |edge_use| {
            let edge = edge_use.bound.0 * 2 + edge_use.index;
            SourceCurveFamily::CircularArc {
                parameter_interval: (0.0, self.sweeps[edge]),
            }
        }
    }

    fn vertex_position(&self) -> impl Fn(SourceVertexKey) -> Option<Point3> + '_ {
        move |key| match key {
            SourceVertexKey::ShellVertex(index) => self.vertices.get(index).copied(),
            SourceVertexKey::Absent => None,
        }
    }
}

/// A structurally-identified curve schema for every edge. The band path never
/// reads it — [`develop_complete_parallel`] re-derives each occurrence's
/// family from its own source representation — but Step 2's traversal
/// requires one, so the fixture supplies a real identified schema rather than
/// an unidentified stub.
fn curve_schema() -> CurveSchema {
    identify_line_segment(&Line(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
    ))
}

/// The band vertical slice, end to end: two distinct essential circles on one
/// cylinder become a validated annular mesh, with no artificial cut edge left
/// in it.
#[test]
fn two_distinct_essential_circles_recover_an_annular_mesh() {
    let fixture = Fixture::new(2.0, [0.0, 3.0], [1.0, -1.0]);
    let schema = fixture.cylinder.schema().clone();

    let (band, mesh) = run_cylinder_band(
        fixture.input.source_face_id,
        fixture.cylinder.clone(),
        &fixture.input,
        declared_outer(),
        &mut |_| curve_schema(),
        &fixture.vertex_position(),
        &fixture.family_of(),
        1.0e-3,
    )
    .expect("two distinct oppositely oriented essential circles bound a band");

    // The band itself: primitive, opposite, strictly ordered, disjoint.
    assert_eq!(band.lower_boundary.homology.abs(), 1);
    assert_eq!(
        band.lower_boundary.homology + band.upper_boundary.homology,
        0,
        "the induced homologies are opposite"
    );
    assert!(
        band.lower_carrier.axial_high() < band.upper_carrier.axial_low(),
        "the carriers are strictly ordered and disjoint along the axis"
    );
    assert!(band.separation > 0.0);
    assert_eq!(band.lower_boundary.bound, BoundId(0));
    assert_eq!(band.upper_boundary.bound, BoundId(1));

    // The annulus.
    assert_eq!(mesh.validity.euler_characteristic, 0);
    assert_eq!(mesh.validity.boundary_components, 2);
    assert_eq!(mesh.validity.triangles, mesh.developed.triangles.len());
    assert_eq!(mesh.physical_vertices.len(), mesh.developed.vertices.len());
    assert!(
        mesh.validity.boundary_edges > 0 && mesh.validity.interior_edges > 0,
        "an annulus has both boundary and interior edges"
    );

    // No degenerate triangle survived the identification, and every lifted
    // vertex is genuinely on the certified cylinder.
    for triangle in &mesh.developed.triangles {
        let [a, b, c] = *triangle;
        assert!(a != b && b != c && a != c, "no collapsed triangle");
    }
    let radius = schema.radius().get();
    for vertex in &mesh.physical_vertices {
        let r = *vertex - schema.origin();
        let axial = r.dot(schema.axis());
        let radial = r - axial * schema.axis();
        assert!(
            (radial.magnitude() - radius).abs() < 1.0e-9,
            "lifted vertex {vertex:?} is not on the cylinder"
        );
        assert!(
            axial > -1.0e-9 && axial < 3.0 + 1.0e-9,
            "lifted vertex {vertex:?} left the band's axial extent"
        );
    }

    // The two physical boundaries are the two source circles: every boundary
    // vertex sits at one of the two certified axial levels, and both levels
    // are represented.
    let mut at_lower = 0;
    let mut at_upper = 0;
    for vertex in &mesh.developed.vertices {
        if vertex.x >= band.lower_carrier.axial_low() && vertex.x <= band.lower_carrier.axial_high()
        {
            at_lower += 1;
        } else if vertex.x >= band.upper_carrier.axial_low()
            && vertex.x <= band.upper_carrier.axial_high()
        {
            at_upper += 1;
        } else {
            panic!("a mesh vertex is on neither certified carrier: {vertex:?}");
        }
    }
    assert!(at_lower >= 3 && at_upper >= 3);
    assert_eq!(at_lower + at_upper, mesh.developed.vertices.len());
}

/// The corpus's own band representation: each bound is *one* closed circular
/// `edge_curve`, whose importer-recovered trim collapsed to a point because
/// both of its ends are the same source vertex. This is the representation ABC
/// `00009190`'s eligible cylinder bands actually carry, and the shape that
/// reached `WitnessConstruction` before the closed-edge rule was applied.
///
/// The obligation here is the one the packet names: the authoritative
/// representation constructs the expected occurrence and reaches the next
/// certified band stage. It does — each bound develops into a complete simple
/// parallel with primitive, opposite homology, which is exactly the input the
/// carrier and cut-open stages consume.
#[test]
fn two_bounds_of_one_closed_circular_edge_each_develop_into_opposite_parallels() {
    let cylinder = z_cylinder(2.0, 5.0);
    let schema = cylinder.schema().clone();
    let levels = [0.0f64, 3.0f64];
    // One vertex per bound: the closed edge starts and ends there.
    let vertices: Vec<Point3> = levels
        .iter()
        .map(|level| on_cylinder(&schema, *level, 0.0))
        .collect();
    // The two circles run opposite ways round the cylinder, which is what
    // makes them a band's two induced boundaries rather than two copies of
    // one orientation. Neither direction is recoverable from the collapsed
    // trim interval; both come from the circles' own parameter senses.
    let sweep_axes = [schema.axis(), -schema.axis()];
    let bounds: Vec<SourceBoundInput> = (0..2)
        .map(|index| SourceBoundInput::EdgeUses {
            id: BoundId(index),
            edge_uses: vec![edge_use(BoundId(index), 0, index, index, index)],
        })
        .collect();
    let input = SourceFaceInput {
        source_face_id: Some(27122),
        declared_face_index: 0,
        bounds,
        orientation: SourceFaceOrientationEvidence {
            face_use_orientation: OrientationEvidence::Missing,
            face_surface_same_sense: OrientationEvidence::Missing,
        },
    };
    let vertex_position = |key: SourceVertexKey| match key {
        SourceVertexKey::ShellVertex(index) => vertices.get(index).copied(),
        SourceVertexKey::Absent => None,
    };
    let family_of = |edge_use: EdgeUseId| SourceCurveFamily::CompleteCircle {
        placement: CompleteCirclePlacement {
            center: schema.origin() + levels[edge_use.bound.0] * schema.axis(),
            sweep_axis: sweep_axes[edge_use.bound.0],
            radius: schema.radius().get(),
            curve_orientation: true,
        },
    };

    let mut developed = Vec::new();
    for bound in &input.bounds {
        developed.push(
            develop_complete_parallel(
                bound,
                &schema,
                &mut |_| curve_schema(),
                &vertex_position,
                &family_of,
            )
            .expect("a closed circular edge develops into a complete simple parallel"),
        );
    }

    // One occurrence, closing on its own single source vertex, one full turn.
    for (parallel, level) in developed.iter().zip(levels) {
        assert_eq!(parallel.edge_uses.len(), 1);
        assert_eq!(parallel.homology.abs(), 1, "a primitive single turn");
        assert!((parallel.starts[0].x - level).abs() < 1.0e-12);
        assert!((parallel.terminal.x - level).abs() < 1.0e-12);
        let period = schema.deck_generator().signed_period().get();
        assert!(
            (parallel.terminal.y - parallel.starts[0].y).abs() - period.abs() < 1.0e-9,
            "the developed chain closes exactly one period away"
        );
    }
    assert_eq!(
        developed[0].homology + developed[1].homology,
        0,
        "the two circles' own parameter senses induce opposite boundary homologies"
    );

    // The next certified stage consumes them: two distinct, strictly ordered
    // carriers, which is precisely what the band admission asks of this input.
    let carriers = (
        carrier_of(&developed[0], &schema),
        carrier_of(&developed[1], &schema),
    );
    assert!(
        matches!(
            classify_carriers(&carriers.0, &carriers.1),
            CarrierRelation::DistinctCarrier {
                first_is_lower: true,
                separation
            } if separation > 0.0
        ),
        "two closed circular edges at different levels are two distinct, \
         strictly ordered circles"
    );
}

/// Two complete coincident cylinder circles are the same carrier, and the
/// band realizer is never reached.
#[test]
fn two_coincident_complete_circles_are_the_same_carrier() {
    let fixture = Fixture::coincident(2.0, 1.5);

    // The carriers, classified directly: coincident circles over the same
    // source vertices are certified equal, not merely unseparated.
    let schema = fixture.cylinder.schema().clone();
    let first = develop_complete_parallel(
        &fixture.input.bounds[0],
        &schema,
        &mut |_| curve_schema(),
        &fixture.vertex_position(),
        &fixture.family_of(),
    )
    .expect("the first bound is a complete simple parallel");
    let second = develop_complete_parallel(
        &fixture.input.bounds[1],
        &schema,
        &mut |_| curve_schema(),
        &fixture.vertex_position(),
        &fixture.family_of(),
    )
    .expect("the second bound is a complete simple parallel");
    assert_eq!(
        first.homology + second.homology,
        0,
        "the fixture's two traversals of one circle are opposite, so the pair \
         is refused on the carriers and not on the orientation"
    );

    // The verdict comes from the complete circles' own geometry: same
    // radius, same complete extent, and one certified enclosure covering
    // every axial coordinate either circle presents.
    let (first_carrier, second_carrier) =
        (carrier_of(&first, &schema), carrier_of(&second, &schema));
    assert_eq!(
        classify_carriers(&first_carrier, &second_carrier),
        CarrierRelation::SameCarrier
    );
    assert_eq!(first_carrier.radius, second_carrier.radius);
    assert!(
        first_carrier
            .observed_high
            .max(second_carrier.observed_high)
            - first_carrier.observed_low.min(second_carrier.observed_low)
            <= first_carrier.enclosure,
        "one certified enclosure covers both complete circles"
    );

    // The shared source cycle is corroboration, and is reported separately
    // from the physical verdict rather than standing in for it.
    assert!(carriers_share_source_cycle(&first, &second));

    // Two circles the source writes over *different* vertices, at the same
    // level, are still one carrier: the physical test does not depend on the
    // representation coinciding.
    let disjoint_representation = Fixture::new(2.0, [1.5, 1.5], [1.0, -1.0]);
    let disjoint_schema = disjoint_representation.cylinder.schema().clone();
    let a = develop_complete_parallel(
        &disjoint_representation.input.bounds[0],
        &disjoint_schema,
        &mut |_| curve_schema(),
        &disjoint_representation.vertex_position(),
        &disjoint_representation.family_of(),
    )
    .expect("a complete simple parallel");
    let b = develop_complete_parallel(
        &disjoint_representation.input.bounds[1],
        &disjoint_schema,
        &mut |_| curve_schema(),
        &disjoint_representation.vertex_position(),
        &disjoint_representation.family_of(),
    )
    .expect("a complete simple parallel");
    assert!(
        !carriers_share_source_cycle(&a, &b),
        "the two bounds are written over distinct source vertices"
    );
    assert_eq!(
        classify_carriers(
            &carrier_of(&a, &disjoint_schema),
            &carrier_of(&b, &disjoint_schema)
        ),
        CarrierRelation::SameCarrier,
        "complete circle equality is decided on the circles, not on the file"
    );

    // And the whole path refuses on exactly that, so nothing downstream —
    // no cut plan, no patch, no triangulation — ever sees this face.
    let exit = run_cylinder_band(
        fixture.input.source_face_id,
        fixture.cylinder.clone(),
        &fixture.input,
        declared_outer(),
        &mut |_| curve_schema(),
        &fixture.vertex_position(),
        &fixture.family_of(),
        1.0e-3,
    )
    .expect_err("two coincident circles do not bound an annulus");
    assert_eq!(exit, BandExit::SameCarrier);
    assert_eq!(exit.category(), SliceCategory::Unsupported);
    assert_eq!(exit.stage(), "carrier");
}

// ---------------------------------------------------------------------------
// Material authority
// ---------------------------------------------------------------------------

/// The nonconformant standing the ABC corpus actually presents: two
/// `FACE_OUTER_BOUND` entities on one face.
fn two_outer_bounds() -> OuterBoundStanding {
    OuterBoundStanding::Declared {
        bound_index: 0,
        declared_count: 2,
    }
}

/// The production-shaped case. A face declaring *two* outer bounds — which
/// ISO 10303-42 forbids, and which no complete essential parallel could
/// satisfy anyway — still reaches a validated annular mesh, and the mesh says
/// out loud that its source was malformed.
#[test]
fn two_outer_bounds_on_a_certified_band_recover_a_mesh_marked_malformed() {
    let fixture = Fixture::new(2.0, [0.0, 3.0], [1.0, -1.0]);

    let (_, mesh) = run_cylinder_band(
        fixture.input.source_face_id,
        fixture.cylinder.clone(),
        &fixture.input,
        two_outer_bounds(),
        &mut |_| curve_schema(),
        &fixture.vertex_position(),
        &fixture.family_of(),
        1.0e-3,
    )
    .expect("an unsatisfiable outer annotation on a proved band is repaired, not fatal");

    // A real annulus, held to the identical battery as the conforming path.
    assert_eq!(mesh.validity.euler_characteristic, 0);
    assert_eq!(mesh.validity.boundary_components, 2);
    // And the file is not thereby called clean.
    assert_eq!(
        mesh.conformance,
        SourceConformance::RecoveredFromMalformedSource(
            NonconformantRepair::TwoOuterBoundsOnCertifiedBand
        ),
    );
}

/// The repair changes the *annotation*, never the region. The declared and
/// intrinsic routes must agree on the physical annulus vertex for vertex.
#[test]
fn declared_and_intrinsic_authority_produce_the_same_physical_region() {
    let fixture = Fixture::new(2.0, [0.0, 3.0], [1.0, -1.0]);
    let run = |standing| {
        run_cylinder_band(
            fixture.input.source_face_id,
            fixture.cylinder.clone(),
            &fixture.input,
            standing,
            &mut |_| curve_schema(),
            &fixture.vertex_position(),
            &fixture.family_of(),
            1.0e-3,
        )
        .expect("the band is certified either way")
        .1
    };

    let declared = run(declared_outer());
    let intrinsic = run(two_outer_bounds());

    assert_eq!(declared.physical_vertices, intrinsic.physical_vertices);
    assert_eq!(declared.developed.triangles, intrinsic.developed.triangles);
    assert_eq!(declared.validity, intrinsic.validity);
    // The one intended difference.
    assert_eq!(declared.conformance, SourceConformance::Conforming);
    assert_ne!(declared.conformance, intrinsic.conformance);
}

/// Absent authority is not repaired, even on a perfect band. A fact nobody
/// stated cannot be normalized: `NotRetained` is a gap in this pipeline's
/// provenance and `NoneDeclared` is a legal `FACE_BOUND`-only face, and
/// neither is the unsatisfiable annotation the repair is scoped to.
#[test]
fn missing_authority_still_refuses_on_a_certified_band() {
    let fixture = Fixture::new(2.0, [0.0, 3.0], [1.0, -1.0]);
    for standing in [
        OuterBoundStanding::NotRetained,
        OuterBoundStanding::NoneDeclared,
    ] {
        let exit = run_cylinder_band(
            fixture.input.source_face_id,
            fixture.cylinder.clone(),
            &fixture.input,
            standing,
            &mut |_| curve_schema(),
            &fixture.vertex_position(),
            &fixture.family_of(),
            1.0e-3,
        )
        .expect_err("a band whose outer standing was never stated is not repaired");
        assert_eq!(exit, BandExit::Patch(SliceExit::MissingOuterBoundAuthority));
        assert_eq!(exit.category(), SliceCategory::Unresolved);
    }
}

/// Three or more declared outer bounds stay a typed inconsistent refusal. Two
/// is repaired because a two-bound band's annotation is *provably*
/// unsatisfiable; three says nothing this module can read, so it refuses.
#[test]
fn three_declared_outer_bounds_remain_a_source_contradiction() {
    let fixture = Fixture::new(2.0, [0.0, 3.0], [1.0, -1.0]);
    let exit = run_cylinder_band(
        fixture.input.source_face_id,
        fixture.cylinder.clone(),
        &fixture.input,
        OuterBoundStanding::Declared {
            bound_index: 0,
            declared_count: 3,
        },
        &mut |_| curve_schema(),
        &fixture.vertex_position(),
        &fixture.family_of(),
        1.0e-3,
    )
    .expect_err("three declared outer bounds are not the repaired pattern");
    assert_eq!(
        exit,
        BandExit::Patch(SliceExit::MultipleOuterBoundsDeclared)
    );
}

/// The repair is gated on the band certificate, not on the annotation. Two
/// same-signed circles never become a band, so the authority question is
/// never even reached — the orientation fact is what is reported.
#[test]
fn incompatible_orientation_refuses_before_authority_is_consulted() {
    let fixture = Fixture::new(2.0, [0.0, 3.0], [1.0, 1.0]);
    let exit = run_cylinder_band(
        fixture.input.source_face_id,
        fixture.cylinder.clone(),
        &fixture.input,
        two_outer_bounds(),
        &mut |_| curve_schema(),
        &fixture.vertex_position(),
        &fixture.family_of(),
        1.0e-3,
    )
    .expect_err("two same-signed parallels do not bound a band");
    assert!(
        matches!(exit, BandExit::OrientationIncompatible { .. }),
        "expected an orientation refusal, got {exit:?}",
    );
    assert_eq!(exit.category(), SliceCategory::Unsupported);
}

/// Coincident carriers are refused with the malformed annotation too, so the
/// repair cannot be read as permission for arbitrary two-bound faces.
#[test]
fn coincident_carriers_refuse_under_the_malformed_annotation() {
    let fixture = Fixture::new(2.0, [1.5, 1.5], [1.0, -1.0]);
    let exit = run_cylinder_band(
        fixture.input.source_face_id,
        fixture.cylinder.clone(),
        &fixture.input,
        two_outer_bounds(),
        &mut |_| curve_schema(),
        &fixture.vertex_position(),
        &fixture.family_of(),
        1.0e-3,
    )
    .expect_err("one carrier is not an annulus, whatever the annotation says");
    assert_eq!(exit, BandExit::SameCarrier);
}
