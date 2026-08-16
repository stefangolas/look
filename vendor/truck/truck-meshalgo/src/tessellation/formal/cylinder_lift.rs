//! Checkpoint 6: lift, deck placement, and terminal holonomy for a rank-1
//! cylinder-disk traversal.
//!
//! # The walk
//!
//! Step 2's traversal is already cyclically continuous by source vertex
//! identity ([`super::planar_slice::traverse_bound`]); this module does not
//! re-check that. What it adds is the developed side of the same claim: two
//! occurrences sharing a source vertex must develop to the *same* point in
//! the universal cover once each is placed on its own deck copy, and the
//! [`super::deck`] solver — never rounding, never re-derived — is what decides
//! which copy.
//!
//! For adjacent occurrences `i` and `i+1` with unplaced (raw) developed
//! endpoints `B_i` (the end of `i`) and `A_{i+1}` (the start of `i+1`), the
//! join is `B_i + n_i g = A_{i+1} + n_{i+1} g`, so `n_{i+1} - n_i` is exactly
//! the deck integer [`solve_axis_aligned`] returns for the point displacement
//! `B_i - A_{i+1}`. The walk fixes `n_0 = 0`, propagates every other `n_i`
//! from that equation, and — critically — does *not* fold the final join back
//! into `n_0`: the candidate `n_0'` it produces, compared against the fixed
//! `n_0 = 0`, *is* the terminal holonomy. Summing the individual developed
//! displacements around the loop would silently discard exactly the
//! information holonomy is: see `FORMAL_SYSTEM.md`'s ban on that shortcut,
//! repeated in this module's own tests.
//!
//! # `WitnessSpec` is a test adapter, not a certificate introduction rule
//!
//! [`super::curve_witness::identify_source_curve_witness`] is the
//! source-authoritative route: it derives the witness class from which
//! [`super::curve_witness::SourceCurveFamily`] variant the source curve
//! structurally is, and (for an arc) its declared sweep from the curve's own
//! parameter interval and Step 2's retained composed sense.
//! [`develop_traversal_from_source`] is the production entry point built on
//! it. [`WitnessSpec`] and [`develop_traversal`] exist only for synthetic
//! test fixtures that construct a witness directly from bare endpoints
//! without carrying a full source curve representation — `WitnessSpec`
//! grants no authority and a production caller must not use it as one.

use super::super::source_evidence::{EdgeUseId, SourceVertexKey};
use super::curve_witness::{
    axial_line_witness, circumferential_arc_witness, identify_source_curve_witness,
    CurveOnCylinderWitness, SourceCurveFamily, WitnessFailure,
};
use super::cylinder::CylinderSchema;
use super::deck::{DeckGenerator, DeckOperationalFailure};
use super::planar_slice::{RegularClosedTraversal, SliceCategory};
use super::rank1_annulus::{propagate_deck_placements, DeckJoinFailure, DeckPlacementWalk};
use truck_geometry::prelude::Point3;

/// Which curve family one traversal occurrence's edge use presents.
///
/// **Test-only.** This type is a caller *assertion*, not a certificate: it
/// lets a synthetic test build a witness from bare endpoints without
/// constructing a full [`super::curve_witness::SourceCurveFamily`] source
/// representation. Production code must use
/// [`develop_traversal_from_source`], which derives the class and sweep
/// structurally instead of trusting this enum's caller-chosen variant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WitnessSpec {
    /// An axial line edge use.
    AxialLine,
    /// A circumferential arc edge use, with its source curve's own declared
    /// signed angular sweep.
    CircumferentialArc {
        /// See [`circumferential_arc_witness`]'s `declared_sweep`.
        declared_sweep: f64,
    },
}

/// Why the lift, placement or holonomy stage could not certify a zero-
/// holonomy disk.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CylinderLiftExit {
    /// An occurrence's start or end source vertex has no certified position.
    MissingVertexPosition {
        /// The occurrence whose endpoint has no certified position.
        edge_use: EdgeUseId,
    },
    /// A curve-on-cylinder witness could not be certified for an occurrence.
    WitnessConstruction {
        /// The occurrence whose witness failed.
        edge_use: EdgeUseId,
        /// Why.
        cause: WitnessFailure,
    },
    /// A join's developed displacement is not compatible with any deck
    /// integer: the shared source vertex is proved inconsistent with the
    /// certified cylinder period.
    JoinNoCompatibleInteger {
        /// Index of the join, between occurrence `join_index` and the next
        /// one in cyclic order.
        join_index: usize,
    },
    /// A join's evidence admits more than one deck integer. For a point
    /// displacement against a nonzero period this is a genuine ambiguity in
    /// the evidence, not a fact about the face.
    JoinMultipleCompatibleIntegers {
        /// See [`Self::JoinNoCompatibleInteger`].
        join_index: usize,
    },
    /// The period is too small, at the displacement's scale, for the
    /// arithmetic enclosure to resolve which deck integer applies.
    JoinIndeterminate {
        /// See [`Self::JoinNoCompatibleInteger`].
        join_index: usize,
    },
    /// The deck solver could not complete the join arithmetically.
    JoinOperationalFailure {
        /// See [`Self::JoinNoCompatibleInteger`].
        join_index: usize,
        /// Why.
        failure: DeckOperationalFailure,
    },
    /// Every join resolved uniquely, but the terminal holonomy is a nonzero
    /// certified deck integer. Outside the initial realization subset (see
    /// the module docs and `FORMAL_SYSTEM.md`); reported rather than
    /// silently reduced to zero.
    NonzeroHolonomy {
        /// The certified nonzero holonomy, in units of the deck generator.
        holonomy: i64,
    },
}

impl From<DeckJoinFailure> for CylinderLiftExit {
    /// Forward the chart-neutral deck-walk failure into this module's own
    /// vocabulary, one variant to one variant.
    ///
    /// The walk itself moved to [`super::rank1_annulus`] when the cone's
    /// essential band needed the identical arithmetic; the names, the
    /// categories and the tags a consumer sees are unchanged by that move.
    fn from(failure: DeckJoinFailure) -> Self {
        match failure {
            DeckJoinFailure::NoCompatibleInteger { join_index } => {
                Self::JoinNoCompatibleInteger { join_index }
            }
            DeckJoinFailure::MultipleCompatibleIntegers { join_index } => {
                Self::JoinMultipleCompatibleIntegers { join_index }
            }
            DeckJoinFailure::Indeterminate { join_index } => Self::JoinIndeterminate { join_index },
            DeckJoinFailure::OperationalFailure {
                join_index,
                failure,
            } => Self::JoinOperationalFailure {
                join_index,
                failure,
            },
        }
    }
}

impl CylinderLiftExit {
    /// Which semantic category this exit belongs to.
    pub fn category(self) -> SliceCategory {
        match self {
            Self::MissingVertexPosition { .. } => SliceCategory::Unresolved,
            Self::WitnessConstruction { cause, .. } => cause.category(),
            Self::JoinNoCompatibleInteger { .. } => SliceCategory::Inconsistent,
            Self::JoinMultipleCompatibleIntegers { .. } | Self::JoinIndeterminate { .. } => {
                SliceCategory::Unresolved
            }
            Self::JoinOperationalFailure { .. } => SliceCategory::OperationalFailure,
            Self::NonzeroHolonomy { .. } => SliceCategory::Unsupported,
        }
    }

    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::MissingVertexPosition { .. } => "lift_missing_vertex_position",
            // The nested `WitnessFailure` already names *which* constructor
            // condition refused the occurrence, and it already carries its own
            // stable tag. Forwarding it is a rendering of information this
            // variant has always held, not a second taxonomy: without it every
            // one of the eight distinguishable refusals collapses into a single
            // `lift_witness_construction_failed` bucket that cannot select a
            // correction.
            Self::WitnessConstruction { cause, .. } => cause.tag(),
            Self::JoinNoCompatibleInteger { .. } => "lift_join_no_compatible_integer",
            Self::JoinMultipleCompatibleIntegers { .. } => "lift_join_multiple_compatible_integers",
            Self::JoinIndeterminate { .. } => "lift_join_indeterminate",
            Self::JoinOperationalFailure { .. } => "lift_join_operational_failure",
            Self::NonzeroHolonomy { .. } => "lift_nonzero_holonomy",
        }
    }
}

/// Step 4's rank-1 product: one developed witness per source occurrence, in
/// source cyclic order, each still in its own unplaced (raw) developed frame.
#[derive(Debug, Clone)]
pub struct DevelopedCylinderBoundary {
    /// The edge use each witness represents, in source cyclic order.
    pub edge_uses: Vec<EdgeUseId>,
    /// The raw (unplaced) witnesses, in the same order.
    pub witnesses: Vec<CurveOnCylinderWitness>,
}

/// Step 4. Develop every traversal occurrence into a curve-on-cylinder
/// witness, using the caller's certified vertex positions and witness
/// classification.
///
/// The traversal's own endpoint pairing is reused unchanged:
/// `occurrence.start_vertex` / `occurrence.end_vertex` are exactly what Step
/// 2 already proved cyclically continuous by source identity. This function
/// substitutes one canonical position per source vertex — never each
/// occurrence's own curve endpoint — so that two occurrences sharing a
/// vertex are guaranteed to develop from bit-identical input, the same
/// discipline [`super::planar_slice::certified_planar_curves`] applies to the
/// planar case.
pub fn develop_traversal(
    traversal: &RegularClosedTraversal,
    schema: &CylinderSchema,
    vertex_position: &impl Fn(SourceVertexKey) -> Option<Point3>,
    witness_spec: &impl Fn(EdgeUseId) -> WitnessSpec,
) -> Result<DevelopedCylinderBoundary, CylinderLiftExit> {
    let mut edge_uses = Vec::with_capacity(traversal.occurrences.len());
    let mut witnesses = Vec::with_capacity(traversal.occurrences.len());

    for occurrence in &traversal.occurrences {
        let start = vertex_position(occurrence.start_vertex).ok_or(
            CylinderLiftExit::MissingVertexPosition {
                edge_use: occurrence.edge_use,
            },
        )?;
        let end = vertex_position(occurrence.end_vertex).ok_or(
            CylinderLiftExit::MissingVertexPosition {
                edge_use: occurrence.edge_use,
            },
        )?;

        let witness = match witness_spec(occurrence.edge_use) {
            WitnessSpec::AxialLine => axial_line_witness(schema, start, end),
            WitnessSpec::CircumferentialArc { declared_sweep } => {
                circumferential_arc_witness(schema, start, end, declared_sweep)
            }
        }
        .map_err(|cause| CylinderLiftExit::WitnessConstruction {
            edge_use: occurrence.edge_use,
            cause,
        })?;

        edge_uses.push(occurrence.edge_use);
        witnesses.push(witness);
    }

    Ok(DevelopedCylinderBoundary {
        edge_uses,
        witnesses,
    })
}

/// Step 4, production route. Develop every traversal occurrence into a
/// curve-on-cylinder witness, deriving each occurrence's witness class and
/// declared sweep from its own source curve family rather than from a
/// caller assertion.
///
/// `family_of` names which [`SourceCurveFamily`] the edge use's own curve
/// representation structurally is (and, for an arc, its trimmed parameter
/// interval) — the same kind of structural fact
/// [`super::support::identify_line_segment`] reads from a representation,
/// not a classification a caller invents. `occurrence.forward`, already
/// retained by Step 2 as the composed sense, is passed through unchanged as
/// [`identify_source_curve_witness`]'s selected orientation.
pub fn develop_traversal_from_source(
    traversal: &RegularClosedTraversal,
    schema: &CylinderSchema,
    vertex_position: &impl Fn(SourceVertexKey) -> Option<Point3>,
    family_of: &impl Fn(EdgeUseId) -> SourceCurveFamily,
) -> Result<DevelopedCylinderBoundary, CylinderLiftExit> {
    let mut edge_uses = Vec::with_capacity(traversal.occurrences.len());
    let mut witnesses = Vec::with_capacity(traversal.occurrences.len());

    for occurrence in &traversal.occurrences {
        let start = vertex_position(occurrence.start_vertex).ok_or(
            CylinderLiftExit::MissingVertexPosition {
                edge_use: occurrence.edge_use,
            },
        )?;
        let end = vertex_position(occurrence.end_vertex).ok_or(
            CylinderLiftExit::MissingVertexPosition {
                edge_use: occurrence.edge_use,
            },
        )?;

        let family = family_of(occurrence.edge_use);
        // Whether this occurrence closes on itself, by source vertex identity
        // and nothing else — the fact a complete circle needs and its
        // collapsed trim interval no longer carries. Read from the same
        // traversal record Step 2 already proved cyclically continuous, so it
        // costs no new evidence.
        let closed_occurrence = occurrence.start_vertex == occurrence.end_vertex;
        let witness = identify_source_curve_witness(
            schema,
            family,
            start,
            end,
            occurrence.forward,
            closed_occurrence,
        )
        .map_err(|cause| CylinderLiftExit::WitnessConstruction {
            edge_use: occurrence.edge_use,
            cause,
        })?;

        edge_uses.push(occurrence.edge_use);
        witnesses.push(witness);
    }

    Ok(DevelopedCylinderBoundary {
        edge_uses,
        witnesses,
    })
}

/// Step 5's rank-1 product: a certified deck placement for every occurrence,
/// with terminal holonomy proved structurally zero.
#[derive(Debug, Clone)]
pub struct ZeroHolonomyLift {
    /// The certified deck integer for each occurrence, in traversal order,
    /// with `placements[0] == 0` by the walk's fixed starting placement.
    pub placements: Vec<i64>,
    /// How many joins were checked, including the final-to-initial wrap.
    pub joins_checked: usize,
}

/// Step 5's walk. Propagate deck placements around the developed boundary
/// from a fixed initial placement and report the terminal holonomy, taking no
/// verdict on its value.
///
/// The arithmetic is [`propagate_deck_placements`]'s, which is chart-neutral:
/// it reads only each occurrence's two developed endpoints and the certified
/// generator, and nothing about which surface produced them. It moved there
/// when the cone's essential band needed the identical walk; this function is
/// the cylinder's entry to it and its behaviour is unchanged, including every
/// exit name, category and tag.
///
/// [`propagate_and_classify_holonomy`] is this walk plus the contractible
/// subset's `h = 0` requirement. A consumer that admits an *essential*
/// boundary — [`super::cylinder_band`], for which `h = ±1` is precisely the
/// admission criterion rather than a failure — reads the holonomy from here
/// rather than recovering it from an error variant.
pub fn propagate_placements(
    boundary: &DevelopedCylinderBoundary,
    generator: DeckGenerator,
) -> Result<DeckPlacementWalk, CylinderLiftExit> {
    let chain: Vec<_> = boundary
        .witnesses
        .iter()
        .map(|witness| (witness.start, witness.end))
        .collect();
    propagate_deck_placements(&chain, generator).map_err(CylinderLiftExit::from)
}

/// Step 5. Propagate deck placements around the developed boundary and
/// certify the terminal holonomy structurally zero.
///
/// [`propagate_placements`] does the walk; this function adds the one fact
/// the contractible-disk subset needs on top of it. A certified nonzero
/// holonomy is reported as [`CylinderLiftExit::NonzeroHolonomy`] and refused
/// rather than silently reduced to zero.
pub fn propagate_and_classify_holonomy(
    boundary: &DevelopedCylinderBoundary,
    generator: DeckGenerator,
) -> Result<ZeroHolonomyLift, CylinderLiftExit> {
    let walk = propagate_placements(boundary, generator)?;
    if walk.holonomy != 0 {
        return Err(CylinderLiftExit::NonzeroHolonomy {
            holonomy: walk.holonomy,
        });
    }
    Ok(ZeroHolonomyLift {
        placements: walk.placements,
        joins_checked: walk.joins_checked,
    })
}

#[cfg(test)]
mod tests {
    use super::super::super::source_evidence::{
        BoundId, ErasedOrientationMechanism, OrientationEvidence, OrientationOrigin,
        SourceBoundInput, SourceEdgeOrientationEvidence, SourceEdgeUseInput, SourceFaceInput,
        SourceFaceOrientationEvidence,
    };
    use super::super::cylinder::{identify_cylinder, CylinderIdentification};
    use super::super::cylinder_arrangement::{certify_cylinder_disk, placed_occurrences};
    use super::super::cylinder_cover::build_working_cover;
    use super::super::cylinder_face::build_cylinder_face;
    use super::super::cylinder_mesh::certify_cylinder_mesh;
    use super::super::deck::DeckBudget;
    use super::super::support::identify_line_segment;
    use super::*;
    use truck_geometry::prelude::{InnerSpace, Line, RevolutedCurve, Vector3};
    use truck_topology::compress::OuterBoundStanding;

    fn z_cylinder(radius: f64, h: f64) -> super::super::cylinder::CertifiedEmbeddedCylinder {
        let revo = RevolutedCurve::by_revolution(
            Line(Point3::new(radius, 0.0, 0.0), Point3::new(radius, 0.0, h)),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        );
        match identify_cylinder(&revo) {
            CylinderIdentification::Cylinder(c) => c,
            other => panic!("expected a certified cylinder, got {other:?}"),
        }
    }

    fn on_cylinder(schema: &CylinderSchema, z: f64, theta: f64) -> Point3 {
        schema.origin()
            + z * schema.axis()
            + schema.radius().get() * theta.cos() * schema.radial_x()
            + schema.radius().get() * theta.sin() * schema.radial_y()
    }

    /// A closed quad face on the cylinder: `points[i] -> points[i+1]`
    /// cyclically. Every edge use runs forward; the reversal hazard is
    /// exercised separately in `curve_witness` and `cylinder_face`. Which
    /// witness class each edge use gets is supplied separately by the caller
    /// to `develop_traversal`.
    fn quad_face(points: &[Point3]) -> (SourceFaceInput, Vec<Point3>) {
        let n = points.len();
        let mut edge_uses = Vec::with_capacity(n);
        for i in 0..n {
            let j = (i + 1) % n;
            edge_uses.push(SourceEdgeUseInput {
                id: EdgeUseId::new(BoundId(0), i),
                source_edge_index: i,
                source_vertices: (
                    SourceVertexKey::ShellVertex(i),
                    SourceVertexKey::ShellVertex(j),
                ),
                use_vertices: (
                    SourceVertexKey::ShellVertex(i),
                    SourceVertexKey::ShellVertex(j),
                ),
                orientation: SourceEdgeOrientationEvidence {
                    bound_times_oriented_edge: OrientationEvidence::Retained {
                        forward: true,
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
            source_face_id: Some(99),
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
        (input, points.to_vec())
    }

    fn declared_outer() -> OuterBoundStanding {
        OuterBoundStanding::Declared {
            bound_index: 0,
            declared_count: 1,
        }
    }

    /// The first required end-to-end milestone: traversal -> witnesses ->
    /// unique placements -> h = 0, on a simple non-wrapping quad patch.
    #[test]
    fn a_simple_quad_reaches_zero_holonomy() {
        let cylinder = z_cylinder(2.0, 5.0);
        let schema = cylinder.schema().clone();
        let points = [
            on_cylinder(&schema, 0.0, 0.2),
            on_cylinder(&schema, 0.0, 1.4),
            on_cylinder(&schema, 3.0, 1.4),
            on_cylinder(&schema, 3.0, 0.2),
        ];
        let specs = vec![
            WitnessSpec::CircumferentialArc {
                declared_sweep: 1.2,
            },
            WitnessSpec::AxialLine,
            WitnessSpec::CircumferentialArc {
                declared_sweep: -1.2,
            },
            WitnessSpec::AxialLine,
        ];
        let (input, positions) = quad_face(&points);
        let curves: Vec<_> = (0..4)
            .map(|i| identify_line_segment(&Line(points[i], points[(i + 1) % 4])))
            .collect();

        let record = build_cylinder_face(
            input.source_face_id,
            cylinder,
            &input,
            declared_outer(),
            &mut |index| curves[index].clone(),
        )
        .expect("regular traversal resolves");

        let vertex_position = |key: SourceVertexKey| match key {
            SourceVertexKey::ShellVertex(index) => positions.get(index).copied(),
            SourceVertexKey::Absent => None,
        };
        let spec_by_use = |edge_use: EdgeUseId| specs[edge_use.index];

        let boundary =
            develop_traversal(&record.traversal, &schema, &vertex_position, &spec_by_use)
                .expect("every occurrence develops to a witness");
        assert_eq!(boundary.witnesses.len(), 4);

        let lift = propagate_and_classify_holonomy(&boundary, schema.deck_generator())
            .expect("zero holonomy on a non-wrapping quad");
        assert_eq!(lift.placements[0], 0);
        assert_eq!(lift.joins_checked, 4);
    }

    /// The same milestone, but with a seam crossing in the middle of the
    /// walk: two joins solve to nonzero deck integers (`+1` then `-1`) and
    /// still cancel to `h = 0`. This is what exercises the deck solver
    /// nontrivially rather than trivially placing every occurrence at `n =
    /// 0`.
    #[test]
    fn a_seam_crossing_quad_reaches_zero_holonomy_through_nontrivial_placements() {
        let cylinder = z_cylinder(2.0, 5.0);
        let schema = cylinder.schema().clone();
        // theta = 3.0 is inside (-PI, PI]; theta = 3.5 is not, and its
        // physical direction's principal branch is 3.5 - 2*PI.
        let points = [
            on_cylinder(&schema, 0.0, 3.0),
            on_cylinder(&schema, 0.0, 3.5),
            on_cylinder(&schema, 3.0, 3.5),
            on_cylinder(&schema, 3.0, 3.0),
        ];
        let specs = vec![
            WitnessSpec::CircumferentialArc {
                declared_sweep: 0.5,
            },
            WitnessSpec::AxialLine,
            WitnessSpec::CircumferentialArc {
                declared_sweep: -0.5,
            },
            WitnessSpec::AxialLine,
        ];
        let (input, positions) = quad_face(&points);
        let curves: Vec<_> = (0..4)
            .map(|i| identify_line_segment(&Line(points[i], points[(i + 1) % 4])))
            .collect();

        let record = build_cylinder_face(
            input.source_face_id,
            cylinder,
            &input,
            declared_outer(),
            &mut |index| curves[index].clone(),
        )
        .expect("regular traversal resolves");

        let vertex_position = |key: SourceVertexKey| match key {
            SourceVertexKey::ShellVertex(index) => positions.get(index).copied(),
            SourceVertexKey::Absent => None,
        };
        let spec_by_use = |edge_use: EdgeUseId| specs[edge_use.index];

        let boundary =
            develop_traversal(&record.traversal, &schema, &vertex_position, &spec_by_use)
                .expect("every occurrence develops to a witness");

        let lift = propagate_and_classify_holonomy(&boundary, schema.deck_generator())
            .expect("zero holonomy: the seam crossings cancel");
        assert_eq!(lift.placements, vec![0, 1, 1, 0]);
    }

    /// The §13/§16 milestone: a seam-crossing contractible cylinder disk is
    /// recovered through every formal stage — authoritative traversal,
    /// certified curve-on-surface witnesses, certified integer deck placement,
    /// finite working cover, quotient-aware disk, polygonalization, and a
    /// topology-blind triangulation — with **no seam segment and no
    /// `PolyBoundary`**.
    ///
    /// The seam crossing is ordinary in the continuous lift: the deck solve
    /// places occurrences at `[0, 1, 1, 0]`, the nonzero joins cancel to zero
    /// holonomy, and the developed boundary closes as a small quad entirely
    /// inside one fundamental tile. The triangulator therefore receives an
    /// already-certified planar disk and creates no synthetic closure, no
    /// `SegmentOrigin::Seam`, and no parity guess.
    #[test]
    fn a_seam_crossing_contractible_disk_recovers_a_valid_mesh_end_to_end() {
        let cylinder = z_cylinder(2.0, 5.0);
        let schema = cylinder.schema().clone();
        // theta 3.0 -> 3.5 straddles the principal-branch seam (π / -π): the
        // developed arc is continuous, but the axial-line witnesses at theta
        // 3.5 read its principal branch (-2.78…), so the deck solve must place
        // them one period over to recover continuity.
        let points = [
            on_cylinder(&schema, 0.0, 3.0),
            on_cylinder(&schema, 0.0, 3.5),
            on_cylinder(&schema, 3.0, 3.5),
            on_cylinder(&schema, 3.0, 3.0),
        ];
        let specs = vec![
            WitnessSpec::CircumferentialArc {
                declared_sweep: 0.5,
            },
            WitnessSpec::AxialLine,
            WitnessSpec::CircumferentialArc {
                declared_sweep: -0.5,
            },
            WitnessSpec::AxialLine,
        ];
        let (input, positions) = quad_face(&points);
        let curves: Vec<_> = (0..4)
            .map(|i| identify_line_segment(&Line(points[i], points[(i + 1) % 4])))
            .collect();
        let record = build_cylinder_face(
            input.source_face_id,
            cylinder,
            &input,
            declared_outer(),
            &mut |index| curves[index].clone(),
        )
        .expect("regular traversal resolves");

        let vertex_position = |key: SourceVertexKey| match key {
            SourceVertexKey::ShellVertex(index) => positions.get(index).copied(),
            SourceVertexKey::Absent => None,
        };
        let spec_by_use = |edge_use: EdgeUseId| specs[edge_use.index];
        let boundary =
            develop_traversal(&record.traversal, &schema, &vertex_position, &spec_by_use)
                .expect("every occurrence develops to a witness");

        let lift = propagate_and_classify_holonomy(&boundary, schema.deck_generator())
            .expect("zero holonomy: the seam crossings cancel");
        assert_eq!(lift.placements, vec![0, 1, 1, 0]);

        // Stage F: finite working cover, derived from certified enclosures,
        // not a fixed window. A small quad needs only the copy it lives in.
        let cover = build_working_cover(
            &boundary.witnesses,
            &lift.placements,
            schema.deck_generator(),
            DeckBudget {
                deck_width_cap: 4096,
            },
        )
        .expect("a narrow seam-crossing quad needs a finite cover within budget");

        // Stage G + H + I: quotient-aware disk. Periodic closure is the deck
        // identification already solved; no seam arc is created.
        let disk = certify_cylinder_disk(
            &boundary.edge_uses,
            &boundary.witnesses,
            &lift.placements,
            schema.deck_generator(),
            declared_outer(),
            &cover.materialized_copies,
        )
        .expect("the seam-crossing disk is simple and translate-disjoint");

        // Stages K + L: polygonalization + topology-blind triangulation of an
        // already-certified planar patch.
        let occurrences = placed_occurrences(
            &boundary.edge_uses,
            &boundary.witnesses,
            &lift.placements,
            &schema.deck_generator(),
        );
        let mesh = certify_cylinder_mesh(&disk, &occurrences, &schema, 1e-9)
            .expect("the certified disk triangulates and validates");

        // Stage M / §13 realization invariants. The formal path emits a disk
        // triangulation; every lifted vertex lies on the cylinder; the
        // developed patch is a disk (Euler characteristic 1).
        assert_eq!(mesh.validity.triangles, mesh.developed.triangles.len());
        assert_eq!(
            mesh.developed.triangles.len() + 2,
            mesh.developed.vertices.len(),
            "a quad disk triangulates to triangles + 2 == vertices"
        );
        assert_eq!(mesh.physical_vertices.len(), mesh.developed.vertices.len());
        // §13 "no seam constraint exists in the emitted patch": the only
        // boundary edges are the four certified source-traversal edges. A
        // synthetic seam would appear as a fifth boundary edge or an extra
        // boundary cycle, and `final_validity` would have rejected it; this
        // makes the count explicit.
        assert_eq!(
            mesh.validity.boundary_edges, 4,
            "the disk boundary is exactly the four source edges — no seam edge"
        );
        assert_eq!(
            mesh.validity.internal_edges, 1,
            "a quad disk has exactly one internal diagonal"
        );
        let radius = schema.radius().get();
        for vertex in &mesh.physical_vertices {
            let r = *vertex - schema.origin();
            let axial = r.dot(schema.axis());
            let radial = r - axial * schema.axis();
            assert!(
                (radial.magnitude() - radius).abs() < 1e-9,
                "lifted vertex {vertex:?} is not on the cylinder"
            );
        }
    }

    /// A single-occurrence full-turn arc is outside the supported subset —
    /// it is exactly a nonzero-holonomy realization, the case the module docs
    /// and `FORMAL_SYSTEM.md` require to be reported rather than silently
    /// accepted or reduced to zero.
    #[test]
    fn a_full_wrap_is_reported_as_nonzero_holonomy() {
        let cylinder = z_cylinder(2.0, 5.0);
        let schema = cylinder.schema().clone();
        let point = on_cylinder(&schema, 0.0, 0.0);
        let witness = circumferential_arc_witness(&schema, point, point, std::f64::consts::TAU)
            .expect("a full turn from a point back to itself is a valid arc witness");
        let boundary = DevelopedCylinderBoundary {
            edge_uses: vec![EdgeUseId::new(BoundId(0), 0)],
            witnesses: vec![witness],
        };

        let exit = propagate_and_classify_holonomy(&boundary, schema.deck_generator())
            .expect_err("a full turn must not certify as zero holonomy");
        assert_eq!(exit, CylinderLiftExit::NonzeroHolonomy { holonomy: 1 });
        assert_eq!(exit.category(), SliceCategory::Unsupported);
    }
}
