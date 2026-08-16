//! The three obligations this cell exists to discharge, and the realization
//! they gate: same nappe, apex exclusion, carrier order.
//!
//! Every fixture here is the diagnosed corpus shape in the small — a conical
//! face with two bounds, each one complete source `CIRCLE` closed on one source
//! vertex — varied one fact at a time, so a failing test names which obligation
//! stopped holding.

use super::*;
use crate::tessellation::formal::cone::{identify_cone, ConeIdentification};
use crate::tessellation::formal::support::identify_line_segment;
use crate::tessellation::source_evidence::{
    ErasedOrientationMechanism, OrientationEvidence, OrientationOrigin,
    SourceEdgeOrientationEvidence, SourceEdgeUseInput, SourceFaceOrientationEvidence,
};
use truck_geometry::prelude::{Line, RevolutedCurve, Vector3};

/// A cone about the z-axis with its apex at the origin and half-angle
/// `atan(slope)`, declared over the generator span `[1, 6]`.
fn z_cone(slope: f64) -> CertifiedEmbeddedCone {
    let revo = RevolutedCurve::by_revolution(
        Line(
            Point3::new(slope * 1.0, 0.0, 1.0),
            Point3::new(slope * 6.0, 0.0, 6.0),
        ),
        Point3::new(0.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
    );
    match identify_cone(&revo) {
        ConeIdentification::Cone(cone) => cone,
        other => panic!("expected a certified cone, got {other:?}"),
    }
}

/// One closed edge use: a single occurrence whose two ends are the *same*
/// source vertex, which is the source topology a complete circular
/// `edge_curve` presents and the only shape this cell admits.
fn closed_edge_use(bound: BoundId, edge: usize, vertex: usize) -> SourceEdgeUseInput {
    SourceEdgeUseInput {
        id: EdgeUseId::new(bound, 0),
        source_edge_index: edge,
        source_vertices: (
            SourceVertexKey::ShellVertex(vertex),
            SourceVertexKey::ShellVertex(vertex),
        ),
        use_vertices: (
            SourceVertexKey::ShellVertex(vertex),
            SourceVertexKey::ShellVertex(vertex),
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

/// A structurally-identified curve schema for every edge. The cone path never
/// reads its content — [`develop_complete_cone_parallel`] re-derives the family
/// from the source representation — but Step 2's traversal requires one.
fn curve_schema() -> CurveSchema {
    identify_line_segment(&Line(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
    ))
}

/// What each bound's single edge use presents to the family reader.
#[derive(Clone, Copy)]
enum Presented {
    /// A complete source circle at this generator coordinate, winding
    /// right-handedly about `sign · axis`.
    CompleteCircle { s: f64, sign: f64 },
    /// A complete source circle whose declared radius is overridden — for
    /// testing the level/radius agreement obligation directly.
    CircleWithRadius { s: f64, sign: f64, radius: f64 },
    /// A partial circular arc: the source declared an extent, so the trim did
    /// not collapse and this is not a complete circle.
    PartialArc { sweep: f64 },
    /// A representation no structural reader admits — a spline, or a genuine
    /// non-circular ellipse.
    Unreadable,
}

/// A two-bound conical face, one complete circle per bound.
struct Fixture {
    cone: CertifiedEmbeddedCone,
    input: SourceFaceInput,
    vertices: Vec<Point3>,
    presented: Vec<Presented>,
}

impl Fixture {
    fn new(slope: f64, bounds_presented: [Presented; 2]) -> Self {
        let cone = z_cone(slope);
        let schema = cone.schema().clone();
        let mut vertices = Vec::new();
        let mut bounds = Vec::new();
        for (index, presented) in bounds_presented.iter().enumerate() {
            let s = match presented {
                Presented::CompleteCircle { s, .. } | Presented::CircleWithRadius { s, .. } => *s,
                _ => 2.0 + index as f64,
            };
            // The vertex the closed edge starts and ends at, at angle zero.
            vertices.push(schema.point_at(s, 0.0));
            let bound = BoundId(index);
            bounds.push(SourceBoundInput::EdgeUses {
                id: bound,
                edge_uses: vec![closed_edge_use(bound, index, index)],
            });
        }
        Self {
            cone,
            input: SourceFaceInput {
                source_face_id: Some(35469),
                declared_face_index: 0,
                bounds,
                orientation: SourceFaceOrientationEvidence {
                    face_use_orientation: OrientationEvidence::Missing,
                    face_surface_same_sense: OrientationEvidence::Missing,
                },
            },
            vertices,
            presented: bounds_presented.to_vec(),
        }
    }

    /// Two bounds written over the *same* complete circle: one carrier, never
    /// an annulus.
    fn coincident(slope: f64, s: f64) -> Self {
        let mut fixture = Self::new(
            slope,
            [
                Presented::CompleteCircle { s, sign: 1.0 },
                Presented::CompleteCircle { s, sign: -1.0 },
            ],
        );
        // Both bounds' vertices are the same point on the same circle.
        let point = fixture.cone.schema().point_at(s, 0.0);
        fixture.vertices = vec![point, point];
        fixture
    }

    fn family_of(&self) -> impl Fn(EdgeUseId) -> Option<SourceCurveFamily> + '_ {
        let schema = self.cone.schema().clone();
        move |edge_use| {
            let placement = |s: f64, sign: f64, radius: f64| CompleteCirclePlacement {
                center: schema.apex() + s * schema.axis(),
                sweep_axis: sign * schema.axis(),
                radius,
                curve_orientation: true,
            };
            match self.presented[edge_use.bound.0] {
                Presented::CompleteCircle { s, sign } => Some(SourceCurveFamily::CompleteCircle {
                    placement: placement(s, sign, schema.radius_at(s)),
                }),
                Presented::CircleWithRadius { s, sign, radius } => {
                    Some(SourceCurveFamily::CompleteCircle {
                        placement: placement(s, sign, radius),
                    })
                }
                Presented::PartialArc { sweep } => Some(SourceCurveFamily::CircularArc {
                    parameter_interval: (0.0, sweep),
                }),
                Presented::Unreadable => None,
            }
        }
    }

    fn vertex_position(&self) -> impl Fn(SourceVertexKey) -> Option<Point3> + '_ {
        move |key| match key {
            SourceVertexKey::ShellVertex(index) => self.vertices.get(index).copied(),
            SourceVertexKey::Absent => None,
        }
    }

    fn run(
        &self,
        outer_bound: OuterBoundStanding,
    ) -> Result<(CertifiedConicalEssentialBand, CertifiedConicalBandMesh), ConicalBandExit> {
        run_conical_essential_band(
            self.input.source_face_id,
            self.cone.clone(),
            &self.input,
            outer_bound,
            &mut |_| curve_schema(),
            &self.vertex_position(),
            &self.family_of(),
            1.0e-3,
        )
    }

    fn refusal(&self) -> ConicalBandExit {
        self.run(declared_outer())
            .err()
            .expect("this fixture must be refused")
    }
}

/// Two complete circles on one nappe, apex excluded, oppositely wound: the
/// diagnosed shape, end to end, to a validated annular mesh.
#[test]
fn two_circles_on_one_nappe_recover_an_annular_mesh() {
    let fixture = Fixture::new(
        0.5,
        [
            Presented::CompleteCircle { s: 2.0, sign: 1.0 },
            Presented::CompleteCircle { s: 5.0, sign: -1.0 },
        ],
    );
    let (band, mesh) = fixture
        .run(declared_outer())
        .expect("two ordered essential circles on one nappe bound a band");

    // The certificate: one nappe, apex strictly outside, ordered and disjoint.
    assert_eq!(band.nappe, Nappe::Positive);
    assert!(band.apex_clearance > 0.0);
    assert!(band.separation > 0.0);
    assert_eq!(band.lower_boundary.bound, BoundId(0));
    assert_eq!(band.upper_boundary.bound, BoundId(1));
    assert_eq!(
        band.lower_boundary.homology + band.upper_boundary.homology,
        0
    );
    assert_eq!(band.lower_boundary.homology.abs(), 1);
    // The carriers keep their own radii: the fact a cylinder does not have.
    assert!((band.lower_carrier.radius - 1.0).abs() < 1e-9);
    assert!((band.upper_carrier.radius - 2.5).abs() < 1e-9);

    // The mesh: an annulus, with no artificial cut edge surviving.
    assert_eq!(mesh.validity.euler_characteristic, 0);
    assert_eq!(mesh.validity.boundary_components, 2);
    assert_eq!(mesh.validity.triangles, mesh.developed.triangles.len());
    assert!(mesh.validity.boundary_edges > 0 && mesh.validity.interior_edges > 0);
    assert_eq!(mesh.physical_vertices.len(), mesh.developed.vertices.len());
    assert_eq!(mesh.nappe, Nappe::Positive);
    assert_eq!(
        mesh.standing,
        ConicalSourceStanding::SingleOuterBoundDeclared { bound_index: 0 }
    );

    // Every lifted vertex is on the cone, on the certified nappe, and strictly
    // inside the closed carrier interval — the realization's own statement that
    // it stayed off the apex.
    let schema = band.cone.schema();
    for vertex in &mesh.physical_vertices {
        assert!(
            schema.radial_gap(*vertex) < 1e-6,
            "{vertex:?} is off the cone"
        );
        let s = schema.generator_coordinate(*vertex);
        assert_eq!(schema.nappe_of(s), Some(Nappe::Positive));
        assert!(
            s >= band.lower_carrier.generator_low() && s <= band.upper_carrier.generator_high(),
            "lifted vertex at s={s} left the carrier interval"
        );
    }
}

/// The mirror image: the same band on the negative nappe. The generator
/// coordinate is signed, so "lower" there is the *larger* circle, and the cell
/// must not have quietly assumed a radius order anywhere.
#[test]
fn a_band_on_the_negative_nappe_recovers_with_the_radius_order_reversed() {
    let fixture = Fixture::new(
        0.5,
        [
            Presented::CompleteCircle { s: -5.0, sign: 1.0 },
            Presented::CompleteCircle {
                s: -2.0,
                sign: -1.0,
            },
        ],
    );
    let (band, mesh) = fixture
        .run(declared_outer())
        .expect("a band on the negative nappe is still a band");
    assert_eq!(band.nappe, Nappe::Negative);
    assert_eq!(band.lower_boundary.bound, BoundId(0));
    // Lower in the generator coordinate is the bigger circle here.
    assert!(band.lower_carrier.radius > band.upper_carrier.radius);
    assert_eq!(mesh.validity.euler_characteristic, 0);
    assert_eq!(mesh.validity.boundary_components, 2);
    let schema = band.cone.schema();
    for vertex in &mesh.physical_vertices {
        assert_eq!(
            schema.nappe_of(schema.generator_coordinate(*vertex)),
            Some(Nappe::Negative)
        );
    }
}

/// The obligation with no cylinder counterpart. Two complete circles on
/// opposite nappes present this population's exact signature — two bounds, two
/// complete circles, opposite windings, well-separated levels — and do not
/// bound a strip. Refused as the proved fact it is, not as a generic
/// unsupported cone.
#[test]
fn two_circles_on_opposite_nappes_are_refused_by_name() {
    let fixture = Fixture::new(
        0.5,
        [
            Presented::CompleteCircle { s: -3.0, sign: 1.0 },
            Presented::CompleteCircle { s: 4.0, sign: -1.0 },
        ],
    );
    assert_eq!(
        fixture.refusal(),
        ConicalBandExit::OppositeNappes {
            first: Nappe::Negative,
            second: Nappe::Positive,
        }
    );
    assert_eq!(fixture.refusal().category(), SliceCategory::Unsupported);
    assert_eq!(fixture.refusal().stage(), "nappe");
}

/// A carrier *at* the apex has no parallel through it: the orbit is a point.
/// Named at the witness, before any chart is built on a degenerate level.
#[test]
fn a_carrier_at_the_apex_is_refused_at_the_witness() {
    let fixture = Fixture::new(
        0.5,
        [
            Presented::CompleteCircle { s: 0.0, sign: 1.0 },
            Presented::CompleteCircle { s: 4.0, sign: -1.0 },
        ],
    );
    assert_eq!(
        fixture.refusal(),
        ConicalBandExit::BoundWitness {
            bound: BoundId(0),
            cause: ConeWitnessFailure::StartAtApex,
        }
    );
}

/// The epistemic case the packet asks for by name: a carrier so near the apex
/// that its certified enclosure contains it. Whether the closed carrier
/// interval includes the apex is *not decided*, and the exit says so rather
/// than choosing a side.
#[test]
fn a_carrier_whose_enclosure_straddles_the_apex_is_unresolved_not_decided() {
    // The enclosure is `1e-9 · max(|s|, radius, 1)`, so at this level it is
    // `1e-9` and strictly wider than `|s|`: the carrier has no certified nappe.
    let fixture = Fixture::new(
        0.5,
        [
            Presented::CompleteCircle {
                s: 4.0e-10,
                sign: 1.0,
            },
            Presented::CompleteCircle { s: 4.0, sign: -1.0 },
        ],
    );
    assert_eq!(fixture.refusal(), ConicalBandExit::ApexContactUndecided);
    assert_eq!(fixture.refusal().category(), SliceCategory::Unresolved);
}

/// Two bounds over one and the same circle are one carrier, and never reach
/// the realizer.
#[test]
fn coincident_carriers_are_refused() {
    let fixture = Fixture::coincident(0.5, 3.0);
    assert_eq!(fixture.refusal(), ConicalBandExit::SameCarrier);
    assert_eq!(fixture.refusal().category(), SliceCategory::Unsupported);
}

/// Same-signed windings are refused, never repaired by reversing one of them.
#[test]
fn incompatible_windings_are_refused_not_reversed() {
    let fixture = Fixture::new(
        0.5,
        [
            Presented::CompleteCircle { s: 2.0, sign: 1.0 },
            Presented::CompleteCircle { s: 5.0, sign: 1.0 },
        ],
    );
    assert!(matches!(
        fixture.refusal(),
        ConicalBandExit::OrientationIncompatible { first, second } if first == second
    ));
}

/// A partial arc is not a complete circle, whatever its sweep. The admitted
/// class is read off the *source family*, so this is refused before any
/// geometry is consulted.
#[test]
fn a_partial_arc_boundary_is_refused() {
    let fixture = Fixture::new(
        0.5,
        [
            Presented::PartialArc {
                sweep: std::f64::consts::PI,
            },
            Presented::CompleteCircle { s: 5.0, sign: -1.0 },
        ],
    );
    assert_eq!(
        fixture.refusal(),
        ConicalBandExit::BoundNotACompleteSourceCircle { bound: BoundId(0) }
    );
}

/// A genuine ellipse, a spline, or any representation the structural readers
/// refuse, reaches the same exit — and reaches it by *absence of a certified
/// family*, never by being approximated into a circle.
#[test]
fn an_unreadable_or_elliptical_boundary_is_refused() {
    let fixture = Fixture::new(
        0.5,
        [
            Presented::CompleteCircle { s: 2.0, sign: 1.0 },
            Presented::Unreadable,
        ],
    );
    assert_eq!(
        fixture.refusal(),
        ConicalBandExit::BoundNotACompleteSourceCircle { bound: BoundId(1) }
    );
}

/// The obligation that replaces the cylinder's "radius is the support's
/// radius". A circle of a perfectly plausible radius, centred on the axis, in a
/// plane perpendicular to it — but not the radius the certified half-angle
/// predicts at its own level — is not this cone's parallel.
#[test]
fn a_circle_whose_radius_does_not_match_its_level_is_refused() {
    let fixture = Fixture::new(
        0.5,
        [
            Presented::CircleWithRadius {
                s: 2.0,
                sign: 1.0,
                // The radius the cone has at s = 5, presented at s = 2.
                radius: 2.5,
            },
            Presented::CompleteCircle { s: 5.0, sign: -1.0 },
        ],
    );
    assert_eq!(
        fixture.refusal(),
        ConicalBandExit::BoundWitness {
            bound: BoundId(0),
            cause: ConeWitnessFailure::CircleNotAConeParallel,
        }
    );
}

/// An absent outer-bound declaration is admitted, because no route here ever
/// consults one — and it is *recorded*, so a census can still tell a silent
/// file from a conformant one.
#[test]
fn an_absent_outer_bound_declaration_is_admitted_and_recorded() {
    let fixture = Fixture::new(
        0.5,
        [
            Presented::CompleteCircle { s: 2.0, sign: 1.0 },
            Presented::CompleteCircle { s: 5.0, sign: -1.0 },
        ],
    );
    let (_, mesh) = fixture
        .run(OuterBoundStanding::NoneDeclared)
        .expect("the material region never came from the declaration");
    assert_eq!(mesh.standing, ConicalSourceStanding::NoOuterBoundRetained);
    assert_eq!(mesh.validity.euler_characteristic, 0);
}

/// Two or more declared outer bounds is a malformed file, and this packet
/// reports that rather than extending the cylinder band's repair to it.
#[test]
fn two_declared_outer_bounds_are_reported_not_repaired() {
    let fixture = Fixture::new(
        0.5,
        [
            Presented::CompleteCircle { s: 2.0, sign: 1.0 },
            Presented::CompleteCircle { s: 5.0, sign: -1.0 },
        ],
    );
    let exit = fixture
        .run(OuterBoundStanding::Declared {
            bound_index: 0,
            declared_count: 2,
        })
        .err()
        .expect("a doubly declared outer bound is refused here");
    assert_eq!(
        exit,
        ConicalBandExit::MultipleOuterBoundsDeclared { declared: 2 }
    );
}

/// The material region does not depend on which loop the source called outer.
/// Declaring the *other* bound produces the identical mesh, which is what
/// "authority is intrinsic" means operationally.
#[test]
fn the_mesh_does_not_depend_on_which_bound_the_source_called_outer() {
    let fixture = Fixture::new(
        0.5,
        [
            Presented::CompleteCircle { s: 2.0, sign: 1.0 },
            Presented::CompleteCircle { s: 5.0, sign: -1.0 },
        ],
    );
    let (_, first) = fixture
        .run(declared_outer())
        .expect("bound 0 declared outer");
    let (_, second) = fixture
        .run(OuterBoundStanding::Declared {
            bound_index: 1,
            declared_count: 1,
        })
        .expect("bound 1 declared outer");
    assert_eq!(first.developed.triangles, second.developed.triangles);
    assert_eq!(first.physical_vertices, second.physical_vertices);
}

/// A face on a cone whose declared bound count is not two is not this cell,
/// and says so before developing anything.
#[test]
fn a_face_without_exactly_two_bounds_is_refused_first() {
    let mut fixture = Fixture::new(
        0.5,
        [
            Presented::CompleteCircle { s: 2.0, sign: 1.0 },
            Presented::CompleteCircle { s: 5.0, sign: -1.0 },
        ],
    );
    fixture.input.bounds.truncate(1);
    assert_eq!(
        fixture.refusal(),
        ConicalBandExit::NotTwoBounds { bounds: 1 }
    );
}

/// The nappe classifier decides on the *sign of certified enclosures*, and on
/// nothing else — not on how far apart the two circles are, and not on their
/// radii. Two circles a micron apart and two a kilometre apart on one nappe are
/// the same verdict.
#[test]
fn the_nappe_verdict_reads_signs_and_not_distances() {
    let carrier = |low: f64, high: f64| ConeCircleCarrier {
        bound: BoundId(0),
        observed_low: low,
        observed_high: high,
        enclosure: 1.0e-9,
        radius: 1.0,
        winding: 1,
    };
    assert!(matches!(
        classify_nappes(&carrier(1.0, 1.0), &carrier(1.000001, 1.000001)),
        NappeRelation::SameNappe {
            nappe: Nappe::Positive,
            ..
        }
    ));
    assert!(matches!(
        classify_nappes(&carrier(1.0, 1.0), &carrier(1000.0, 1000.0)),
        NappeRelation::SameNappe {
            nappe: Nappe::Positive,
            ..
        }
    ));
    assert!(matches!(
        classify_nappes(&carrier(-1.0, -1.0), &carrier(-1000.0, -1000.0)),
        NappeRelation::SameNappe {
            nappe: Nappe::Negative,
            ..
        }
    ));
    assert!(matches!(
        classify_nappes(&carrier(-1.0, -1.0), &carrier(1.0, 1.0)),
        NappeRelation::OppositeNappes { .. }
    ));
    // An enclosure containing the apex has no side, and is not given one.
    assert_eq!(
        classify_nappes(&carrier(-1.0e-12, 1.0e-12), &carrier(1.0, 1.0)),
        NappeRelation::Undecided
    );
}

/// The property that makes the enclosure width sound rather than merely small:
/// widening it is monotone toward refusal.
///
/// A wider enclosure can turn a nappe verdict into `Undecided` and can never do
/// the reverse, and it can never flip one nappe into the other. So the bound's
/// only power over admission is to withhold it — which is why "the tolerance
/// decided the nappe" is not a reading this cell admits. Checked by sweeping
/// the enclosure over sixteen decades against a fixed pair of carriers.
#[test]
fn widening_the_enclosure_only_ever_withholds_a_nappe_verdict() {
    let carrier = |s: f64, enclosure: f64| ConeCircleCarrier {
        bound: BoundId(0),
        observed_low: s,
        observed_high: s,
        enclosure,
        radius: 1.0,
        winding: 1,
    };
    let mut withheld = false;
    let mut previous_decided = true;
    for exponent in -16..=2 {
        let enclosure = 10.0_f64.powi(exponent);
        let relation = classify_nappes(&carrier(1.0, enclosure), &carrier(4.0, enclosure));
        match relation {
            NappeRelation::SameNappe { nappe, .. } => {
                // Never the wrong side, at any width.
                assert_eq!(nappe, Nappe::Positive, "enclosure 1e{exponent}");
                assert!(
                    previous_decided,
                    "a verdict reappeared after being withheld at a narrower enclosure"
                );
            }
            NappeRelation::Undecided => {
                withheld = true;
                previous_decided = false;
            }
            // Widening must never invent the opposite-nappe verdict, which is
            // a *positive* claim about the face.
            NappeRelation::OppositeNappes { .. } => {
                panic!("widening produced an opposite-nappe claim at 1e{exponent}")
            }
        }
    }
    assert!(
        withheld,
        "the sweep must reach a width wide enough to withhold, or it proves nothing"
    );

    // The same, for a genuinely opposite-nappe pair: widening withholds the
    // verdict rather than converting it into a same-nappe one.
    for exponent in -16..=2 {
        let enclosure = 10.0_f64.powi(exponent);
        match classify_nappes(&carrier(-2.0, enclosure), &carrier(3.0, enclosure)) {
            NappeRelation::OppositeNappes { first, second } => {
                assert_eq!((first, second), (Nappe::Negative, Nappe::Positive));
            }
            NappeRelation::Undecided => {}
            NappeRelation::SameNappe { .. } => {
                panic!("widening turned an opposite-nappe pair into one nappe at 1e{exponent}")
            }
        }
    }
}
