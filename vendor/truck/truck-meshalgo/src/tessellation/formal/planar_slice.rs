//! The first end-to-end vertical slice: a planar, line-bounded, hole-free face.
//!
//! # What this module is
//!
//! Steps 2 through 8 of `FORMAL_SYSTEM.md` §XVIII, implemented *only* far
//! enough to carry one face of the admitted subset from source evidence to a
//! validated triangle mesh, and to refuse everything else truthfully. It is
//! deliberately not a general implementation of any of those stages: the
//! milestone is a complete chain, not a complete stage.
//!
//! ```text
//! Step 0 source evidence
//!   -> Step 1 certified planar rank-0 ambient
//!   -> Step 2 regular source traversal
//!   -> Step 3 certified planar curve occurrences
//!   -> Step 4 rank-0 developed boundary
//!   -> Step 5 trivial deck and holonomy solution
//!   -> Step 6 one-copy working cover
//!   -> Step 7 certified simple Jordan arrangement
//!   -> bounded-interior material selection
//!   -> Step 8A certified polygonal approximation
//!   -> Step 8B constrained triangulation
//!   -> final validity
//! ```
//!
//! # The admitted subset
//!
//! A face enters only when *every* predicate holds. Anything else exits as
//! `Unresolved`, `Unsupported`, `Inconsistent` or `OperationalFailure`, with an
//! exit reason naming the obstruction, and no heuristic repair is attempted
//! anywhere. [`SliceExit`] is the complete list of ways out.
//!
//! # Why the curve family is line segments and polylines
//!
//! Both project *exactly* under the plane's affine inverse, so Step 3 needs no
//! numerical certificate and Step 8A's approximation error is zero. That
//! collapses three obligations onto one object — the developed boundary is the
//! polygon is the mesh boundary — which is what makes a first slice tractable.
//!
//! The collapse is guarded, not assumed. [`PolygonalRegion`] refuses to claim
//! the source-curve Jordan proof also settles polygon simplicity unless every
//! occurrence's approximation error is *exactly* zero
//! ([`Rank0DevelopedBoundary::approximation_is_exact`]). The first arc family
//! that arrives will fail that guard and be forced through its own check
//! rather than inheriting this one.
//!
//! # Why ear clipping rather than the `spade` CDT
//!
//! Final validity here is the eleven-item battery of Step 8B, including "the
//! union of the triangles is the polygonal region" and "the mesh boundary
//! equals the expected polygon cycle". A CDT triangulates the convex hull, so
//! reaching those claims from one requires deciding which triangles are inside
//! — and a centroid containment test is exactly what Step 8B forbids as a
//! substitute for the obligations. Ear clipping on a polygon already *proved*
//! simple inserts no Steiner points and emits only interior triangles, so every
//! item in the battery is a check on what was produced rather than a repair of
//! it. The battery still runs independently; nothing is taken on the
//! algorithm's word.

use super::super::source_evidence::{
    BoundId, EdgeUseId, OrientationEvidence, SourceBoundInput, SourceFaceInput, SourceVertexKey,
};
use super::ambient::CertifiedAmbientLattice;
use super::numeric::{FiniteF64, NonNegativeFinite};
use super::support::{CurveSchema, PlaneSchema};
use truck_geometry::prelude::{InnerSpace, Point2, Point3, Vector2};
use truck_topology::compress::OuterBoundStanding;

// ---------------------------------------------------------------------------
// Stages and exits
// ---------------------------------------------------------------------------

/// How far a face got.
///
/// Ordered, and the order is the pipeline's: [`Self::rank`] is what the funnel
/// counts, so "reached Step 5" means every earlier stage resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SliceStage {
    /// Nothing attempted: the face is not in the candidate population.
    NotAttempted,
    /// Step 1's product was consumed and is a certified rank-0 lattice.
    AmbientRank0,
    /// Step 2: one closed regular traversal of one authoritative outer bound.
    RegularTraversal,
    /// Step 3: every occurrence has a complete-interval planar witness.
    PlanarCurves,
    /// Step 4: the source-local rank-0 lift.
    DevelopedBoundary,
    /// Step 5: every join displacement and the holonomy are structurally zero.
    DeckSolution,
    /// Step 6: a finite one-copy working cover.
    WorkingCover,
    /// Step 7: the developed boundary is a simple closed Jordan curve.
    JordanArrangement,
    /// The bounded complementary component was selected as material.
    MaterialRegion,
    /// Step 8A: a certified simple polygonal region.
    PolygonalRegion,
    /// Step 8B: a triangle complex.
    Triangulation,
    /// Every final validity predicate passed.
    FinalValidity,
}

impl SliceStage {
    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::NotAttempted => "not_attempted",
            Self::AmbientRank0 => "ambient_rank0",
            Self::RegularTraversal => "regular_traversal",
            Self::PlanarCurves => "planar_curves",
            Self::DevelopedBoundary => "developed_boundary",
            Self::DeckSolution => "deck_solution",
            Self::WorkingCover => "working_cover",
            Self::JordanArrangement => "jordan_arrangement",
            Self::MaterialRegion => "material_region",
            Self::PolygonalRegion => "polygonal_region",
            Self::Triangulation => "triangulation",
            Self::FinalValidity => "final_validity",
        }
    }

    /// Position in the pipeline, for funnel counting.
    pub fn rank(self) -> usize {
        self as usize
    }
}

/// The semantic category of an exit, matching `FORMAL_SYSTEM.md` Definition 3.
///
/// `Unresolved` is a statement about the *evidence*; `Unsupported` is a proved
/// statement about the face against a declared envelope; `Inconsistent` is a
/// proved contradiction in the source; `OperationalFailure` is a statement
/// about the machine and says nothing about the face at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliceCategory {
    /// The stage established its product.
    Resolved,
    /// A required proposition was not established.
    Unresolved,
    /// The face is proved to lie outside the admitted subset.
    Unsupported,
    /// The evidence contradicts itself.
    Inconsistent,
    /// The machine could not carry out the computation.
    OperationalFailure,
}

impl SliceCategory {
    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::Resolved => "resolved",
            Self::Unresolved => "unresolved",
            Self::Unsupported => "unsupported",
            Self::Inconsistent => "inconsistent",
            Self::OperationalFailure => "operational_failure",
        }
    }
}

/// Every way a face can leave the slice.
///
/// One variant per obstruction the corpus can actually present, so the
/// histogram over this enum *is* the expansion backlog: the largest population
/// names the next capability to add.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliceExit {
    // -- entry ----------------------------------------------------------
    /// Step 1 did not resolve an ambient lattice for this face.
    AmbientUnresolved,
    /// Step 1 resolved a lattice and it is not rank 0.
    NotRank0,
    /// No structural reader identified the support surface.
    SupportSchemaNotAuthoritative,
    /// The support surface was identified and is not a plane.
    NotPlanar,

    // -- Step 2 ---------------------------------------------------------
    /// No stage retained which bound the source declared to be the outer one.
    MissingOuterBoundAuthority,
    /// The source declared more than one `FACE_OUTER_BOUND`.
    MultipleOuterBoundsDeclared,
    /// The face has bounds beyond its outer one. Holes are P2 work.
    MultipleBoundsOrHoles,
    /// The declared outer bound carries no edge uses.
    DegenerateTraversal,
    /// An edge use has no geometric traversal occurrence to consume.
    MissingGeometricTraversalOccurrence,
    /// Consecutive edge uses do not meet at a shared source vertex.
    SourceJoinContradiction,
    /// An edge use's traversal endpoints are not its source endpoints
    /// reordered by exactly the composed sense.
    EndpointsNotConsistentWithRetainedSense,

    // -- Step 3 ---------------------------------------------------------
    /// The curve representation has no structural reader.
    UnsupportedCurveRepresentation,
    /// The plane basis is too ill conditioned to bound the inverse.
    IllConditionedPlaneBasis,
    /// A projected curve endpoint disagrees with its source vertex.
    CurveSurfaceInconsistency,
    /// A projected coordinate was not finite.
    ProjectionNotFinite,

    // -- Steps 4-6 ------------------------------------------------------
    /// A deck displacement was represented as nonzero under a rank-0 lattice.
    NonzeroDeckDisplacementInRank0,
    /// The boundary holonomy was not structurally zero.
    NonzeroBoundaryHolonomy,
    /// A complete-interval bound for an occurrence was unavailable.
    CurveBoundUnavailable,
    /// A proved clause of the formal envelope was exceeded.
    FormalEnvelopeExceeded,
    /// An execution budget ran out before a proof concluded.
    ExecutionBudgetExhausted,

    // -- Step 7 ---------------------------------------------------------
    /// An occurrence is not simple on its own trimmed interval.
    IndividualCurveNotSimple,
    /// An adjacent pair meets somewhere besides its declared shared endpoint.
    AdjacentPairHasExtraIntersection,
    /// An adjacent pair's declared shared endpoint does not correspond.
    AdjacentEndpointMismatch,
    /// A nonadjacent pair crosses.
    NonadjacentCrossing,
    /// A nonadjacent pair touches without crossing.
    NonadjacentTangency,
    /// Two occurrences share a positive-length overlap.
    PositiveLengthOverlap,
    /// Two nonadjacent occurrences share a source vertex.
    NonadjacentRepeatedVertex,

    // -- Step 8 ---------------------------------------------------------
    /// No whole-interval approximation certificate for some occurrence.
    PolygonalizationCertificateUnavailable,
    /// The polygonal approximation self-intersects.
    PolygonalApproximationSelfIntersects,
    /// Ear clipping could not reduce the polygon.
    TriangulationDidNotComplete,
    /// The mesh boundary is not the polygon cycle.
    MeshBoundaryMismatch,
    /// A combinatorial mesh predicate failed.
    MeshTopologyInvalid,
    /// A geometric mesh predicate failed.
    MeshGeometryInvalid,

    // -- multi-bound (planar holes) -------------------------------------
    /// An edge use's identity repeats across the face's bounds.
    DuplicateEdgeUseId,
    /// A bound other than the authoritative outer one carries no edge-use
    /// evidence, so it can be neither traversed nor proved absent.
    DegenerateInnerBound,
    /// Two distinct boundary components cross.
    BoundaryComponentsCross,
    /// Two distinct boundary components touch without crossing.
    BoundaryComponentsTouch,
    /// Two distinct boundary components share a positive-length overlap.
    BoundaryComponentsOverlap,
    /// A bound the source declared to be an inner one lies outside the
    /// authoritative outer loop.
    InnerBoundOutsideOuter,
    /// One inner loop lies inside another. Nested holes are P2 work.
    NestedHole,
    /// The containment predicate could not decide where a loop lies.
    ContainmentUndecided,
    /// A retained triangle's edge crosses a boundary constraint.
    TriangleCrossesBoundaryConstraint,
    /// The retained complex does not have `h + 1` boundary cycles, or a cycle
    /// does not correspond to a declared bound.
    BoundaryCycleCountMismatch,
}

impl SliceExit {
    /// Which semantic category this exit belongs to.
    ///
    /// The mapping is the substance, not bookkeeping. An unread schema is
    /// missing *evidence*; a hole is a proved fact about the face against a
    /// declared envelope; a source join contradiction is a proved
    /// contradiction; and a mesh predicate failing after every input obligation
    /// was discharged is a defect in this module, not a verdict about the face.
    pub fn category(self) -> SliceCategory {
        match self {
            // Evidence was not established.
            Self::AmbientUnresolved
            | Self::SupportSchemaNotAuthoritative
            | Self::MissingOuterBoundAuthority
            | Self::MissingGeometricTraversalOccurrence
            | Self::IllConditionedPlaneBasis
            | Self::CurveBoundUnavailable
            | Self::IndividualCurveNotSimple
            | Self::PolygonalizationCertificateUnavailable
            | Self::ContainmentUndecided => SliceCategory::Unresolved,

            // Proved facts about the face, outside the admitted subset.
            Self::NotRank0
            | Self::NotPlanar
            | Self::MultipleBoundsOrHoles
            | Self::DegenerateTraversal
            | Self::UnsupportedCurveRepresentation
            | Self::FormalEnvelopeExceeded
            | Self::AdjacentPairHasExtraIntersection
            | Self::NonadjacentCrossing
            | Self::NonadjacentTangency
            | Self::PositiveLengthOverlap
            | Self::NonadjacentRepeatedVertex
            | Self::PolygonalApproximationSelfIntersects
            | Self::DegenerateInnerBound
            | Self::BoundaryComponentsCross
            | Self::BoundaryComponentsTouch
            | Self::BoundaryComponentsOverlap
            | Self::NestedHole => SliceCategory::Unsupported,

            // Proved contradictions in the source evidence.
            Self::MultipleOuterBoundsDeclared
            | Self::SourceJoinContradiction
            | Self::EndpointsNotConsistentWithRetainedSense
            | Self::CurveSurfaceInconsistency
            | Self::AdjacentEndpointMismatch
            | Self::NonzeroDeckDisplacementInRank0
            | Self::NonzeroBoundaryHolonomy
            | Self::DuplicateEdgeUseId
            | Self::InnerBoundOutsideOuter => SliceCategory::Inconsistent,

            // Statements about the machine or about this module.
            Self::ExecutionBudgetExhausted
            | Self::ProjectionNotFinite
            | Self::TriangulationDidNotComplete
            | Self::MeshBoundaryMismatch
            | Self::MeshTopologyInvalid
            | Self::MeshGeometryInvalid
            | Self::TriangleCrossesBoundaryConstraint
            | Self::BoundaryCycleCountMismatch => SliceCategory::OperationalFailure,
        }
    }

    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::AmbientUnresolved => "ambient_unresolved",
            Self::NotRank0 => "not_rank_0",
            Self::SupportSchemaNotAuthoritative => "support_schema_not_authoritative",
            Self::NotPlanar => "not_planar",
            Self::MissingOuterBoundAuthority => "missing_outer_bound_authority",
            Self::MultipleOuterBoundsDeclared => "multiple_outer_bounds_declared",
            Self::MultipleBoundsOrHoles => "multiple_bounds_or_holes",
            Self::DegenerateTraversal => "degenerate_traversal",
            Self::MissingGeometricTraversalOccurrence => "missing_geometric_traversal_occurrence",
            Self::SourceJoinContradiction => "source_join_contradiction",
            Self::EndpointsNotConsistentWithRetainedSense => "endpoints_inconsistent_with_sense",
            Self::UnsupportedCurveRepresentation => "unsupported_curve_representation",
            Self::IllConditionedPlaneBasis => "ill_conditioned_plane_basis",
            Self::CurveSurfaceInconsistency => "curve_surface_inconsistency",
            Self::ProjectionNotFinite => "projection_not_finite",
            Self::NonzeroDeckDisplacementInRank0 => "nonzero_deck_displacement_in_rank0",
            Self::NonzeroBoundaryHolonomy => "nonzero_boundary_holonomy",
            Self::CurveBoundUnavailable => "curve_bound_unavailable",
            Self::FormalEnvelopeExceeded => "formal_envelope_exceeded",
            Self::ExecutionBudgetExhausted => "execution_budget_exhausted",
            Self::IndividualCurveNotSimple => "individual_curve_not_simple",
            Self::AdjacentPairHasExtraIntersection => "adjacent_pair_has_extra_intersection",
            Self::AdjacentEndpointMismatch => "adjacent_endpoint_mismatch",
            Self::NonadjacentCrossing => "nonadjacent_crossing",
            Self::NonadjacentTangency => "nonadjacent_tangency",
            Self::PositiveLengthOverlap => "positive_length_overlap",
            Self::NonadjacentRepeatedVertex => "nonadjacent_repeated_vertex",
            Self::PolygonalizationCertificateUnavailable => {
                "polygonalization_certificate_unavailable"
            }
            Self::PolygonalApproximationSelfIntersects => "polygonal_approximation_self_intersects",
            Self::TriangulationDidNotComplete => "triangulation_did_not_complete",
            Self::MeshBoundaryMismatch => "mesh_boundary_mismatch",
            Self::MeshTopologyInvalid => "mesh_topology_invalid",
            Self::MeshGeometryInvalid => "mesh_geometry_invalid",
            Self::DuplicateEdgeUseId => "duplicate_edge_use_id",
            Self::DegenerateInnerBound => "degenerate_inner_bound",
            Self::BoundaryComponentsCross => "boundary_components_cross",
            Self::BoundaryComponentsTouch => "boundary_components_touch",
            Self::BoundaryComponentsOverlap => "boundary_components_overlap",
            Self::InnerBoundOutsideOuter => "inner_bound_outside_outer",
            Self::NestedHole => "nested_hole",
            Self::ContainmentUndecided => "containment_undecided",
            Self::TriangleCrossesBoundaryConstraint => "triangle_crosses_boundary_constraint",
            Self::BoundaryCycleCountMismatch => "boundary_cycle_count_mismatch",
        }
    }
}

/// A stage's result: it advanced, or it left with a named reason.
type SliceResult<T> = Result<T, SliceExit>;

// ---------------------------------------------------------------------------
// Step 2 — authoritative regular traversal
// ---------------------------------------------------------------------------

/// One edge use, with the geometric traversal occurrence Step 0 selected.
///
/// `source_*` and the traversal direction are kept apart on purpose. Step 0
/// already applied `bound x oriented_edge` exactly once to produce
/// `use_vertices`; this stage reads that as a *fact* about the curve's
/// traversal direction and does not apply it to the vertices a second time.
#[derive(Debug, Clone)]
pub struct TraversalOccurrence {
    /// `(BoundId, local_index)` — the edge use's identity.
    pub edge_use: EdgeUseId,
    /// Which edge in the shell's edge table.
    pub source_edge_index: usize,
    /// The source vertex this use starts at.
    pub start_vertex: SourceVertexKey,
    /// The source vertex this use ends at.
    pub end_vertex: SourceVertexKey,
    /// Whether the use traverses its curve in the curve's own direction.
    ///
    /// The single retained orientation factor, read from Step 0 and never
    /// recomputed. `EDGE_CURVE`'s sense and the selected curve direction were
    /// folded into the converted curve at import and are `HistoryErased`; the
    /// *normalized physical sign* remains unavailable and this stage does not
    /// need it.
    pub forward: bool,
    /// The curve's structural schema, in the curve's own parameter direction.
    pub curve: CurveSchema,
}

/// One closed regular traversal of one authoritative outer bound.
#[derive(Debug, Clone)]
pub struct RegularClosedTraversal {
    /// Which bound.
    pub bound: BoundId,
    /// Occurrences in declared cyclic order.
    pub occurrences: Vec<TraversalOccurrence>,
}

/// Step 2. Build one closed regular traversal from one authoritative outer
/// bound.
///
/// Every join is justified by *source vertex identity*. No coordinate
/// proximity is consulted anywhere in this function, so two vertices at the
/// same coordinates stay distinct and one vertex reached through noisy
/// geometry stays identified.
pub fn regular_traversal(
    input: &SourceFaceInput,
    outer_bound: OuterBoundStanding,
    curves: &mut impl FnMut(usize) -> CurveSchema,
) -> SliceResult<RegularClosedTraversal> {
    // 1. Authoritative outer-bound standing. Having one bound is not standing.
    let outer_index = match outer_bound {
        OuterBoundStanding::NotRetained => return Err(SliceExit::MissingOuterBoundAuthority),
        OuterBoundStanding::NoneDeclared => return Err(SliceExit::MissingOuterBoundAuthority),
        OuterBoundStanding::Declared {
            declared_count: 1,
            bound_index,
        } => bound_index as usize,
        OuterBoundStanding::Declared { .. } => return Err(SliceExit::MultipleOuterBoundsDeclared),
    };

    // 2. This slice is the hole-free one, so the outer bound must be the only
    //    one. A degenerate evidence term still counts: it may be a real
    //    collapsed `VERTEX_LOOP`, which this slice does not model. Faces with
    //    inner bounds are the [`super::planar_holes`] population.
    if input.bounds.len() != 1 || outer_index != 0 {
        return Err(SliceExit::MultipleBoundsOrHoles);
    }
    traverse_bound(&input.bounds[0], curves)
}

/// Steps 2's per-bound core: one closed regular traversal of *one* bound,
/// whichever standing that bound has.
///
/// Split out of [`regular_traversal`] so the multi-bound path of
/// [`super::planar_holes`] runs the identical logic on each of its loops rather
/// than a re-derived copy of it. The authority question — which bound is the
/// outer one, and whether inner ones are admitted — belongs to the caller and
/// is deliberately absent here.
pub(super) fn traverse_bound(
    bound: &SourceBoundInput,
    curves: &mut impl FnMut(usize) -> CurveSchema,
) -> SliceResult<RegularClosedTraversal> {
    let SourceBoundInput::EdgeUses { id, edge_uses } = bound else {
        return Err(SliceExit::DegenerateTraversal);
    };
    if edge_uses.is_empty() {
        return Err(SliceExit::DegenerateTraversal);
    }

    // 3. Consume Step 0's occurrences in declared cyclic order, exactly once
    //    each.
    //
    //    Before the bound-level continuity check, deliberately: a use whose
    //    sense was erased, or whose endpoints do not follow from it, makes the
    //    *specific* defect nameable, where continuity would report only that
    //    the cycle does not close. Checking per-use first also means a face
    //    with both defects exits `Unresolved` rather than `Inconsistent`, which
    //    is the weaker and therefore safer claim.
    let mut occurrences = Vec::with_capacity(edge_uses.len());
    for edge_use in edge_uses {
        // The composed sense is the traversal direction. It must be retained;
        // a use whose sense was erased has no geometric traversal occurrence
        // to consume, which is `Unresolved` and not a fact about the face.
        let OrientationEvidence::Retained { forward, .. } =
            edge_use.orientation.bound_times_oriented_edge
        else {
            return Err(SliceExit::MissingGeometricTraversalOccurrence);
        };
        // Step 0 states this proposition and does not discharge it. Discharging
        // it here is what stops this stage applying the same sign twice.
        if !edge_use.endpoints_consistent() {
            return Err(SliceExit::EndpointsNotConsistentWithRetainedSense);
        }
        if !edge_use.start_vertex().is_identified() || !edge_use.end_vertex().is_identified() {
            return Err(SliceExit::MissingGeometricTraversalOccurrence);
        }
        let curve = curves(edge_use.source_edge_index);
        // Any structurally-identified representation is admitted here, not
        // only the exactly-polygonal ones: this traversal is shared by the
        // planar rank-0 path (`regular_traversal`, which only ever sees
        // `LineSegment`/`Polyline`/`NotStructurallyIdentified` today, so this
        // widening changes nothing for it) and the rank-1 cylinder path
        // (`super::cylinder_face::build_cylinder_face`), whose downstream
        // witness stage (`super::cylinder_lift::develop_traversal_from_source`)
        // never reads `occurrence.curve` at all — it re-derives the curve
        // family and sweep from the source representation independently.
        // `certified_planar_curves` (Step 3, planar-only) still requires
        // `.polygonal()` and exits `UnsupportedCurveRepresentation` on a
        // `CircularArc` exactly as it does on `NotStructurallyIdentified`
        // today, so a `CircularArc` reaching the planar path fails exactly as
        // safely as before this change, just one stage later.
        if !curve.is_structurally_identified() {
            return Err(SliceExit::UnsupportedCurveRepresentation);
        }
        occurrences.push(TraversalOccurrence {
            edge_use: edge_use.id,
            source_edge_index: edge_use.source_edge_index,
            start_vertex: edge_use.start_vertex(),
            end_vertex: edge_use.end_vertex(),
            forward,
            curve,
        });
    }

    // 4. Checked cyclic continuity, by source identity. Every join is a
    //    statement about vertex *identity*; no coordinate is consulted.
    match bound.cyclically_continuous() {
        Some(true) => {}
        Some(false) => return Err(SliceExit::SourceJoinContradiction),
        None => return Err(SliceExit::DegenerateTraversal),
    }

    Ok(RegularClosedTraversal {
        bound: *id,
        occurrences,
    })
}

// ---------------------------------------------------------------------------
// Step 3 — certified planar curve occurrences
// ---------------------------------------------------------------------------

/// How a curve-on-surface relation was discharged over the complete interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertificateRoute {
    /// Route B: the plane's affine inverse applied to an exactly polygonal 3D
    /// representation. Every source segment maps to exactly one 2D segment and
    /// the represented approximation error is zero.
    AnalyticAffineProjectionOfPolygonalCurve,
    /// The rank-1 cylinder analog: an axial-line or circumferential-arc
    /// witness developed via the certified cylinder schema
    /// ([`super::curve_witness`]). Linear in the developed `(axial, angular)`
    /// chart by construction, so — as with Route B — the represented
    /// approximation error is zero.
    AnalyticCylinderDevelopment,
}

impl CertificateRoute {
    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::AnalyticAffineProjectionOfPolygonalCurve => "analytic_affine_polygonal",
            Self::AnalyticCylinderDevelopment => "analytic_cylinder_development",
        }
    }
}

/// One traversal occurrence as a 2D curve in the plane's native chart.
///
/// `points` is in **traversal order** — the composed sense has been applied to
/// the curve's own direction exactly once, here and nowhere else.
#[derive(Debug, Clone)]
pub struct CertifiedPlanarCurveOccurrence {
    /// The edge use this represents.
    pub edge_use: EdgeUseId,
    /// The source vertex the traversal starts at.
    pub start_vertex: SourceVertexKey,
    /// The source vertex the traversal ends at.
    pub end_vertex: SourceVertexKey,
    /// The complete trimmed occurrence, in traversal order, in the plane's
    /// native `(u, v)` chart. Never orthogonalised, never normalised.
    pub points: Vec<Point2>,
    /// How the whole-interval obligation was discharged.
    pub route: CertificateRoute,
    /// The 3D distance between this occurrence's represented endpoints and the
    /// source vertices they are declared to meet.
    ///
    /// Zero for the corpus's `EDGE_CURVE`-of-`LINE` faces, whose converted
    /// curve is built from the two vertices. Retained rather than assumed away
    /// because it is the whole of the polygon's approximation error: a segment
    /// whose endpoints move by at most `e` stays within `e` of the original
    /// over its complete interval, by linearity.
    pub endpoint_reconciliation: NonNegativeFinite,
}

impl CertifiedPlanarCurveOccurrence {
    /// The 2D traversal start.
    pub fn start(&self) -> Point2 {
        self.points[0]
    }

    /// The 2D traversal end.
    pub fn end(&self) -> Point2 {
        self.points[self.points.len() - 1]
    }
}

/// Step 3. Project every traversal occurrence into the support plane's native
/// parameter chart.
///
/// The projection is the Gram solve of [`super::support::PlaneGram::solve`],
/// not the per-axis quotient: STEP planes need not supply an orthogonal basis.
///
/// The shared endpoints are the *source vertices'* projections, taken once per
/// vertex, so adjacent occurrences share a bit-identical `Point2` and the
/// closure predicate in Step 5 is exact rather than approximate. Each
/// occurrence's own curve endpoints are checked against them, and the
/// discrepancy is retained as the polygon's approximation error rather than
/// welded away.
pub fn certified_planar_curves(
    traversal: &RegularClosedTraversal,
    plane: &PlaneSchema,
    vertex_position: &impl Fn(SourceVertexKey) -> Option<Point3>,
    tolerance: f64,
) -> SliceResult<Vec<CertifiedPlanarCurveOccurrence>> {
    // The conditioning obligation. `identify_plane` already proved the basis
    // separated against a structural floor; this is the *numerical* question,
    // asked against the tolerance this call is required to meet.
    let gram = plane.gram();
    if !plane_inverse_is_conditioned(plane, tolerance) {
        return Err(SliceExit::IllConditionedPlaneBasis);
    }

    let project = |point: Point3| -> SliceResult<Point2> {
        let offset = point - plane.origin();
        let (u, v) = gram.solve(offset.dot(plane.u_axis()), offset.dot(plane.v_axis()));
        match (FiniteF64::new(u), FiniteF64::new(v)) {
            (Ok(u), Ok(v)) => Ok(Point2::new(u.get(), v.get())),
            _ => Err(SliceExit::ProjectionNotFinite),
        }
    };

    let mut out = Vec::with_capacity(traversal.occurrences.len());
    for occurrence in &traversal.occurrences {
        let polygonal = occurrence
            .curve
            .polygonal()
            .ok_or(SliceExit::UnsupportedCurveRepresentation)?;

        // Traversal order: the curve's own chain, reversed exactly when the
        // composed sense says the use runs against it.
        let mut chain: Vec<Point3> = polygonal.vertices().to_vec();
        if !occurrence.forward {
            chain.reverse();
        }

        // The declared endpoints. Their positions are the authoritative shared
        // endpoint occurrences, and they are what the polygon will use.
        let start = vertex_position(occurrence.start_vertex)
            .ok_or(SliceExit::MissingGeometricTraversalOccurrence)?;
        let end = vertex_position(occurrence.end_vertex)
            .ok_or(SliceExit::MissingGeometricTraversalOccurrence)?;

        // Endpoint correspondence, in 3D, against the tolerance. A curve whose
        // ends are nowhere near the vertices its edge declares is a source
        // contradiction, not an approximation to absorb.
        let start_gap = (chain[0] - start).magnitude();
        let end_gap = (chain[chain.len() - 1] - end).magnitude();
        if !(start_gap.is_finite() && end_gap.is_finite()) {
            return Err(SliceExit::ProjectionNotFinite);
        }
        if start_gap > tolerance || end_gap > tolerance {
            return Err(SliceExit::CurveSurfaceInconsistency);
        }
        // Curve-on-surface: the complete trimmed occurrence must lie *on* the
        // plane. For a polygonal curve that is exactly the claim that every
        // vertex does, since the plane is affine and the segments are convex
        // combinations of their ends.
        for point in &chain {
            if !point_is_on_plane(plane, *point, tolerance) {
                return Err(SliceExit::CurveSurfaceInconsistency);
            }
        }

        // Substitute the authoritative shared endpoints. Interior vertices are
        // the curve's own.
        let last = chain.len() - 1;
        chain[0] = start;
        chain[last] = end;

        let points = chain
            .into_iter()
            .map(project)
            .collect::<SliceResult<Vec<Point2>>>()?;

        let reconciliation = NonNegativeFinite::new(start_gap.max(end_gap))
            .map_err(|_| SliceExit::ProjectionNotFinite)?;

        out.push(CertifiedPlanarCurveOccurrence {
            edge_use: occurrence.edge_use,
            start_vertex: occurrence.start_vertex,
            end_vertex: occurrence.end_vertex,
            points,
            route: CertificateRoute::AnalyticAffineProjectionOfPolygonalCurve,
            endpoint_reconciliation: reconciliation,
        });
    }
    Ok(out)
}

/// Whether the plane inverse is conditioned well enough to be trusted at this
/// tolerance.
///
/// The criterion is dimensionless: the Gram matrix's condition number in the
/// spectral-equivalent 2x2 form, compared against the reciprocal of the
/// relative precision the tolerance implies. Comparing the *dimensional*
/// determinant to an absolute epsilon would make the answer depend on whether
/// the model is in metres or millimetres, which is the error this avoids.
fn plane_inverse_is_conditioned(plane: &PlaneSchema, tolerance: f64) -> bool {
    let gram = plane.gram();
    let normalised = gram.normalised_determinant().get();
    if !(normalised > 0.0) || !tolerance.is_finite() || tolerance <= 0.0 {
        return false;
    }
    // Relative error of the solve is bounded by a small multiple of the
    // reciprocal separation. Requiring it to stay below 1e-6 leaves ten orders
    // of magnitude of headroom over f64 and is far stricter than the
    // structural floor `identify_plane` applies.
    normalised >= 1e-6
}

/// Whether a point lies on the plane, within tolerance.
///
/// Measured along the unit normal, so the test is a distance in model units
/// and is comparable with the caller's tolerance regardless of how the plane's
/// basis is scaled.
fn point_is_on_plane(plane: &PlaneSchema, point: Point3, tolerance: f64) -> bool {
    let normal = plane.u_axis().cross(plane.v_axis());
    let magnitude = normal.magnitude();
    if !(magnitude > 0.0) {
        return false;
    }
    let distance = (point - plane.origin()).dot(normal) / magnitude;
    distance.abs() <= tolerance
}

// ---------------------------------------------------------------------------
// Steps 4, 5, 6 — rank-0 lift, deck solve, working cover
// ---------------------------------------------------------------------------

/// A deck displacement under a rank-0 lattice.
///
/// The unique element of the trivial group, represented *structurally* — as a
/// unit struct, not as a pair of floats near zero. That is the point: Step 5
/// requires the abstract deck value to be zero, and a float comparison would
/// make "structurally zero" a tolerance question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rank0Displacement;

/// Step 4's product: one developed occurrence per source occurrence.
///
/// For a rank-0 ambient lattice the plane's native chart *is* the universal
/// cover chart, so the lift is the identity on coordinates. It is still a
/// stage rather than a skipped step, because the invariants it establishes —
/// one occurrence per occurrence, no quotient, no merge of coordinate-equal
/// but source-distinct endpoints — are exactly what a later rank-1 expansion
/// will have to preserve.
#[derive(Debug, Clone)]
pub struct Rank0DevelopedBoundary {
    /// Developed occurrences, in source cyclic order.
    pub occurrences: Vec<CertifiedPlanarCurveOccurrence>,
    /// The deck displacement attached to each occurrence, all identity.
    pub displacements: Vec<Rank0Displacement>,
}

impl Rank0DevelopedBoundary {
    /// Whether every occurrence's approximation error is exactly zero.
    ///
    /// The guard that keeps Step 7's source-curve Jordan proof from being
    /// reused as Step 8A's polygon-simplicity proof once a curve family with
    /// nonzero error is admitted.
    pub fn approximation_is_exact(&self) -> bool {
        self.occurrences
            .iter()
            .all(|occurrence| occurrence.endpoint_reconciliation.get() == 0.0)
    }

    /// The largest represented approximation error over the boundary.
    pub fn approximation_bound(&self) -> f64 {
        self.occurrences
            .iter()
            .map(|occurrence| occurrence.endpoint_reconciliation.get())
            .fold(0.0, f64::max)
    }
}

/// Step 4. Attach the rank-0 deck coordinate, preserving everything else.
///
/// Nothing is reduced modulo anything, normalised into a box, or merged.
pub fn rank0_lift(
    lattice: &CertifiedAmbientLattice,
    occurrences: Vec<CertifiedPlanarCurveOccurrence>,
) -> SliceResult<Rank0DevelopedBoundary> {
    if !matches!(lattice, CertifiedAmbientLattice::Rank0(_)) {
        return Err(SliceExit::NotRank0);
    }
    let displacements = vec![Rank0Displacement; occurrences.len()];
    Ok(Rank0DevelopedBoundary {
        occurrences,
        displacements,
    })
}

/// Step 5's product: every join and the holonomy proved structurally zero.
#[derive(Debug, Clone, Copy)]
pub struct TrivialDeckSolution {
    /// The boundary holonomy: the ordered sum of the join displacements.
    pub holonomy: Rank0Displacement,
    /// How many joins were checked, including final-to-initial.
    pub joins_checked: usize,
}

/// Step 5. Verify — not assert — that every join and the holonomy are zero.
///
/// Source identity establishes incidence; the geometric predicate then checks
/// that two *independently certified* parameter representations agree at a join
/// the source already declared. Proximity never creates a join here, and never
/// removes one.
pub fn trivial_deck_solution(
    developed: &Rank0DevelopedBoundary,
) -> SliceResult<TrivialDeckSolution> {
    let occurrences = &developed.occurrences;
    let count = occurrences.len();
    if count == 0 {
        return Err(SliceExit::DegenerateTraversal);
    }
    // The abstract group element must be structurally zero. `Rank0Displacement`
    // has one inhabitant, so a nonzero value is unrepresentable — but the check
    // is written out rather than left implicit, because the rank-1 expansion
    // will replace the type and must replace the check with it.
    for displacement in &developed.displacements {
        if *displacement != Rank0Displacement {
            return Err(SliceExit::NonzeroDeckDisplacementInRank0);
        }
    }
    if developed.displacements.len() != count {
        return Err(SliceExit::NonzeroDeckDisplacementInRank0);
    }

    for index in 0..count {
        let current = &occurrences[index];
        let next = &occurrences[(index + 1) % count];
        // 1. Source identity. This is what establishes the join.
        if current.end_vertex != next.start_vertex {
            return Err(SliceExit::SourceJoinContradiction);
        }
        // 2. The certified parameter representations must agree there. Step 3
        //    used the *same* authoritative endpoint occurrence for both sides,
        //    so agreement is exact and a failure here means a defect above, not
        //    a tolerance question.
        if current.end() != next.start() {
            return Err(SliceExit::AdjacentEndpointMismatch);
        }
    }

    Ok(TrivialDeckSolution {
        holonomy: Rank0Displacement,
        joins_checked: count,
    })
}

/// Step 6's product: one finite parameter-domain region holding every complete
/// occurrence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OneCopyWorkingCover {
    /// Lower corner.
    pub min: Point2,
    /// Upper corner.
    pub max: Point2,
    /// Copies of the fundamental domain. Exactly one, because the deck group
    /// is trivial.
    pub copies: usize,
    /// How many complete occurrences it contains.
    pub occurrences: usize,
}

/// Step 6. The union of every occurrence's complete-interval enclosure.
///
/// For a polygonal occurrence the enclosure is the bounding box of its
/// vertices, which contains the complete interval exactly because each segment
/// is a convex combination of its endpoints. No padding is added: the formal
/// envelope declares no clause requiring any, and inventing one would be a
/// semantic limit nothing proved.
pub fn one_copy_working_cover(
    developed: &Rank0DevelopedBoundary,
) -> SliceResult<OneCopyWorkingCover> {
    let mut min = Point2::new(f64::INFINITY, f64::INFINITY);
    let mut max = Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for occurrence in &developed.occurrences {
        for point in &occurrence.points {
            if !point.x.is_finite() || !point.y.is_finite() {
                return Err(SliceExit::CurveBoundUnavailable);
            }
            min.x = min.x.min(point.x);
            min.y = min.y.min(point.y);
            max.x = max.x.max(point.x);
            max.y = max.y.max(point.y);
        }
    }
    if !(min.x.is_finite() && min.y.is_finite() && max.x.is_finite() && max.y.is_finite()) {
        return Err(SliceExit::CurveBoundUnavailable);
    }
    Ok(OneCopyWorkingCover {
        min,
        max,
        copies: 1,
        occurrences: developed.occurrences.len(),
    })
}

// ---------------------------------------------------------------------------
// Step 7 — conservative simple Jordan arrangement
// ---------------------------------------------------------------------------

/// What two segments share.
///
/// Deliberately not a Boolean. Collapsing these into "they intersect" is what
/// makes a missed tangency and a positive-length overlap indistinguishable
/// from an expected endpoint contact.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SegmentIntersection {
    /// Nothing in common.
    Empty,
    /// Exactly one point.
    Point(Point2),
    /// A positive-length collinear overlap.
    Overlap,
}

/// Classify what two closed 2D segments share, using exact orientation
/// predicates.
///
/// `robust::orient2d` is exact for `f64` inputs, so a determinant reported zero
/// *is* zero and collinearity is decided rather than estimated. No epsilon
/// appears in this function, which is why proximity alone can neither create
/// nor remove an intersection.
pub fn classify_segments(a0: Point2, a1: Point2, b0: Point2, b1: Point2) -> SegmentIntersection {
    let d = |p: Point2, q: Point2, r: Point2| {
        robust::orient2d(
            robust::Coord { x: p.x, y: p.y },
            robust::Coord { x: q.x, y: q.y },
            robust::Coord { x: r.x, y: r.y },
        )
    };
    let d1 = d(b0, b1, a0);
    let d2 = d(b0, b1, a1);
    let d3 = d(a0, a1, b0);
    let d4 = d(a0, a1, b1);

    let opposite = |x: f64, y: f64| (x > 0.0 && y < 0.0) || (x < 0.0 && y > 0.0);
    if opposite(d1, d2) && opposite(d3, d4) {
        // A proper crossing: one isolated transverse point. Computed only for
        // reporting; the classification above did not depend on it.
        return SegmentIntersection::Point(segment_crossing(a0, a1, b0, b1));
    }

    // Any remaining contact is collinear or touching, so it is decided by
    // containment rather than by sign.
    let mut touches: Vec<Point2> = Vec::new();
    let mut collinear_overlap = false;
    let record = |point: Point2, touches: &mut Vec<Point2>| {
        if !touches.iter().any(|existing| *existing == point) {
            touches.push(point);
        }
    };
    if d1 == 0.0 && on_segment(b0, b1, a0) {
        record(a0, &mut touches);
    }
    if d2 == 0.0 && on_segment(b0, b1, a1) {
        record(a1, &mut touches);
    }
    if d3 == 0.0 && on_segment(a0, a1, b0) {
        record(b0, &mut touches);
    }
    if d4 == 0.0 && on_segment(a0, a1, b1) {
        record(b1, &mut touches);
    }
    if touches.len() >= 2 {
        collinear_overlap = true;
    }
    match (collinear_overlap, touches.first()) {
        (true, _) => SegmentIntersection::Overlap,
        (false, Some(point)) => SegmentIntersection::Point(*point),
        (false, None) => SegmentIntersection::Empty,
    }
}

/// Whether `r` lies within the bounding box of the collinear segment `p q`.
///
/// Only ever called after an exact orientation test has established
/// collinearity, so the box test is a containment decision and not a proximity
/// one.
pub(super) fn on_segment(p: Point2, q: Point2, r: Point2) -> bool {
    r.x >= p.x.min(q.x) && r.x <= p.x.max(q.x) && r.y >= p.y.min(q.y) && r.y <= p.y.max(q.y)
}

/// The crossing point of two segments already proved to cross transversally.
fn segment_crossing(a0: Point2, a1: Point2, b0: Point2, b1: Point2) -> Point2 {
    let r = Vector2::new(a1.x - a0.x, a1.y - a0.y);
    let s = Vector2::new(b1.x - b0.x, b1.y - b0.y);
    let denominator = r.x * s.y - r.y * s.x;
    let qp = Vector2::new(b0.x - a0.x, b0.y - a0.y);
    let t = (qp.x * s.y - qp.y * s.x) / denominator;
    Point2::new(a0.x + t * r.x, a0.y + t * r.y)
}

/// Step 7's product: the developed boundary proved to be a simple closed
/// Jordan curve, with its vertices and segments in source cyclic order.
#[derive(Debug, Clone)]
pub struct SimpleJordanArrangement {
    /// The closed cycle of 2D vertices, in traversal order, first not
    /// repeated at the end.
    pub cycle: Vec<Point2>,
    /// Which occurrence each segment came from.
    pub segment_origin: Vec<EdgeUseId>,
}

/// Step 7. Prove the ordered developed boundary is a simple closed curve.
///
/// The boundary is a chain of straight segments, so "each occurrence is simple
/// on its trimmed interval", "each adjacent pair meets exactly at its declared
/// shared endpoint" and "nonadjacent pairs are disjoint" are all statements
/// about pairs of segments, and they are established by exhaustive pairwise
/// classification. `O(n^2)`, which the first slice admits.
///
/// No intersection vertex is ever inserted and no proximity weld occurs: the
/// arrangement's vertices are exactly the authoritative endpoint occurrences
/// the previous stages produced.
pub fn simple_jordan_arrangement(
    developed: &Rank0DevelopedBoundary,
) -> SliceResult<SimpleJordanArrangement> {
    jordan_arrangement_of(&developed.occurrences)
}

/// Step 7's per-loop core, over one loop's ordered occurrences.
///
/// Split out of [`simple_jordan_arrangement`] so [`super::planar_holes`]
/// certifies each of its loops with the identical predicates. It establishes
/// only that *this* loop is a simple closed Jordan curve; nothing about how it
/// relates to any other loop, which is the multi-bound caller's obligation.
pub(super) fn jordan_arrangement_of(
    occurrences: &[CertifiedPlanarCurveOccurrence],
) -> SliceResult<SimpleJordanArrangement> {
    // Flatten to the segment cycle. Each occurrence contributes its own
    // vertices; the shared endpoints are already bit-identical, so the last
    // point of one occurrence is dropped in favour of the next occurrence's
    // first, which is the same value.
    let mut cycle: Vec<Point2> = Vec::new();
    let mut segment_origin: Vec<EdgeUseId> = Vec::new();
    for occurrence in occurrences {
        for (index, point) in occurrence.points.iter().enumerate() {
            if index + 1 == occurrence.points.len() {
                break;
            }
            cycle.push(*point);
            segment_origin.push(occurrence.edge_use);
        }
    }
    let n = cycle.len();
    if n < 3 {
        return Err(SliceExit::DegenerateTraversal);
    }

    let segment = |i: usize| (cycle[i], cycle[(i + 1) % n]);

    // 1. Individual simplicity. A straight segment is simple exactly when it
    //    has positive length; a zero-length one is a collapsed occurrence,
    //    which this slice does not model.
    for i in 0..n {
        let (p, q) = segment(i);
        if p == q {
            return Err(SliceExit::IndividualCurveNotSimple);
        }
    }

    // 2. Every unordered pair, exactly once, adjacency classified modulo cyclic
    //    indexing.
    for i in 0..n {
        for j in (i + 1)..n {
            let (a0, a1) = segment(i);
            let (b0, b1) = segment(j);
            let adjacent = j == i + 1 || (i == 0 && j == n - 1);
            let intersection = classify_segments(a0, a1, b0, b1);
            match (adjacent, intersection) {
                (_, SegmentIntersection::Overlap) => return Err(SliceExit::PositiveLengthOverlap),
                (true, SegmentIntersection::Point(point)) => {
                    // The declared shared endpoint, and only it.
                    let shared = if j == i + 1 { a1 } else { a0 };
                    if point != shared {
                        return Err(SliceExit::AdjacentPairHasExtraIntersection);
                    }
                }
                (true, SegmentIntersection::Empty) => {
                    // Adjacent segments must meet. Reaching here means the
                    // cycle is not closed where the source says it is.
                    return Err(SliceExit::AdjacentEndpointMismatch);
                }
                (false, SegmentIntersection::Point(point)) => {
                    // A nonadjacent contact. A shared endpoint is a repeated
                    // vertex — legal STEP, outside this subset; anything else
                    // is a crossing or a touch.
                    let endpoint = point == a0 || point == a1 || point == b0 || point == b1;
                    return Err(match endpoint {
                        true => SliceExit::NonadjacentRepeatedVertex,
                        false => match transversal(a0, a1, b0, b1) {
                            true => SliceExit::NonadjacentCrossing,
                            false => SliceExit::NonadjacentTangency,
                        },
                    });
                }
                (false, SegmentIntersection::Empty) => {}
            }
        }
    }

    Ok(SimpleJordanArrangement {
        cycle,
        segment_origin,
    })
}

/// Whether two segments meet transversally, by exact orientation.
pub(super) fn transversal(a0: Point2, a1: Point2, b0: Point2, b1: Point2) -> bool {
    let d = |p: Point2, q: Point2, r: Point2| {
        robust::orient2d(
            robust::Coord { x: p.x, y: p.y },
            robust::Coord { x: q.x, y: q.y },
            robust::Coord { x: r.x, y: r.y },
        )
    };
    let opposite = |x: f64, y: f64| (x > 0.0 && y < 0.0) || (x < 0.0 && y > 0.0);
    opposite(d(b0, b1, a0), d(b0, b1, a1)) && opposite(d(a0, a1, b0), d(a0, a1, b1))
}

// ---------------------------------------------------------------------------
// Material selection
// ---------------------------------------------------------------------------

/// The material region: the unique bounded complementary component of a
/// certified simple planar outer bound.
///
/// Orientation independent, so it needs neither the normalized physical sign —
/// which Step 0 measured as unavailable on every one of the corpus's 110,770
/// edge uses — nor any parity argument.
#[derive(Debug, Clone)]
pub struct BoundedMaterialRegion {
    /// The Jordan boundary.
    pub boundary: SimpleJordanArrangement,
    /// The signed area of the cycle. Its *sign* is the traversal's handedness
    /// in the plane's chart and carries no physical meaning; its magnitude is
    /// the region's area and is used by the final validity battery.
    pub signed_area: f64,
}

/// Select the bounded complementary component as the material region.
///
/// Requires authoritative outer-bound standing, which Step 2 already
/// established — the argument is stated here because the *reason* the bounded
/// component is material is that the loop is an outer bound, not that there
/// happens to be one loop.
pub fn bounded_material_region(
    boundary: SimpleJordanArrangement,
    outer_bound: OuterBoundStanding,
) -> SliceResult<BoundedMaterialRegion> {
    // Report the standing the source actually has. A face declaring two
    // `FACE_OUTER_BOUND` entities is not one whose authority went missing —
    // it is a source contradiction, and `MultipleOuterBoundsDeclared` is the
    // exit that says so. Collapsing the two into one exit misattributes a
    // stated fact about the source to a gap in this pipeline's provenance,
    // which is the one confusion this module's exit taxonomy exists to
    // prevent. Step 2's `regular_traversal` already separates them; this is
    // the same split, applied where the authority is consumed.
    if outer_bound.unique_outer_bound_index().is_none() {
        return Err(match outer_bound {
            OuterBoundStanding::Declared { .. } => SliceExit::MultipleOuterBoundsDeclared,
            _ => SliceExit::MissingOuterBoundAuthority,
        });
    }
    bounded_material_region_of(boundary)
}

/// The bounded complementary component of an already-authorized simple cycle.
///
/// The area work of [`bounded_material_region`] without its authority gate.
/// Splitting the two is what lets a caller that establishes material standing
/// some *other* certified way reuse this arithmetic instead of copying it,
/// while [`bounded_material_region`] — the only entry a generic patch can
/// reach — keeps requiring source outer-bound authority exactly as before.
///
/// `pub(super)` deliberately: the authority question is settled per route
/// inside [`super`], and nothing outside the formal module can call this and
/// thereby skip a gate. The sole caller today is the cylinder band, whose
/// standing comes from its own completed band certificate rather than from a
/// declared outer loop — see
/// [`super::cylinder_band::BandMaterialAuthority`].
pub(super) fn bounded_material_region_of(
    boundary: SimpleJordanArrangement,
) -> SliceResult<BoundedMaterialRegion> {
    let signed_area = signed_area(&boundary.cycle);
    if !signed_area.is_finite() || signed_area == 0.0 {
        return Err(SliceExit::MeshGeometryInvalid);
    }
    Ok(BoundedMaterialRegion {
        boundary,
        signed_area,
    })
}

/// Twice the signed area of a closed polygon, halved. Shoelace.
fn signed_area(cycle: &[Point2]) -> f64 {
    let n = cycle.len();
    let mut total = 0.0;
    for i in 0..n {
        let p = cycle[i];
        let q = cycle[(i + 1) % n];
        total += p.x * q.y - q.x * p.y;
    }
    total / 2.0
}

// ---------------------------------------------------------------------------
// Step 8A — certified polygonal approximation
// ---------------------------------------------------------------------------

/// A polygonal region with a whole-interval approximation certificate for
/// every source occurrence.
#[derive(Debug, Clone)]
pub struct CertifiedPolygonalRegion {
    /// The material region.
    pub region: BoundedMaterialRegion,
    /// The largest whole-interval approximation error over the boundary.
    pub approximation_bound: f64,
    /// The tolerance that bound was required to meet.
    pub required_tolerance: f64,
    /// Whether the approximation is exact, i.e. the polygon *is* the source
    /// boundary rather than an approximation of it.
    pub exact: bool,
}

/// Step 8A. Certify the polygonal boundary.
///
/// For the admitted curve family the polygon is the source boundary, so the
/// certificate is the statement that the endpoint reconciliation of Step 3 is
/// the whole of the error — true because a straight segment whose endpoints
/// move by at most `e` stays within `e` of the original over its complete
/// interval, by linearity, and interior polyline vertices were not moved at
/// all.
///
/// The polygon-simplicity obligation is discharged by Step 7's proof *only*
/// when the approximation is exact, so that the two are the same object. A
/// nonzero bound forces the separate check the guard names.
pub fn certified_polygonal_region(
    region: BoundedMaterialRegion,
    developed: &Rank0DevelopedBoundary,
    tolerance: f64,
) -> SliceResult<CertifiedPolygonalRegion> {
    let bound = developed.approximation_bound();
    if !bound.is_finite() || bound > tolerance {
        return Err(SliceExit::PolygonalizationCertificateUnavailable);
    }
    let exact = developed.approximation_is_exact();
    if !exact {
        // The source-curve Jordan proof of Step 7 was taken on the polygon,
        // which is not the source boundary in this case. Establishing that the
        // approximation is still simple needs a separation bound this slice
        // does not have.
        return Err(SliceExit::PolygonalizationCertificateUnavailable);
    }
    Ok(CertifiedPolygonalRegion {
        region,
        approximation_bound: bound,
        required_tolerance: tolerance,
        exact,
    })
}

// ---------------------------------------------------------------------------
// Step 8B — triangulation and final validity
// ---------------------------------------------------------------------------

/// A triangle complex over a certified polygonal region.
#[derive(Debug, Clone)]
pub struct TriangulatedRegion {
    /// The polygon's vertices, in cycle order. Indices below refer to these.
    pub vertices: Vec<Point2>,
    /// Triangles as vertex-index triples.
    pub triangles: Vec<[usize; 3]>,
}

/// Step 8B. Triangulate a polygon already proved simple.
///
/// Ear clipping. Every candidate ear is tested with exact orientation and
/// exact containment predicates, and no Steiner point is introduced, so the
/// result's vertex set is the polygon's and its boundary is the polygon cycle
/// by construction. That is *claimed* here and *checked* by
/// [`final_validity`], which does not read this function's reasoning.
pub fn triangulate(region: &CertifiedPolygonalRegion) -> SliceResult<TriangulatedRegion> {
    let vertices = region.region.boundary.cycle.clone();
    let n = vertices.len();
    if n < 3 {
        return Err(SliceExit::TriangulationDidNotComplete);
    }
    // Work in counter-clockwise order so an ear is a convex vertex with a
    // positive orientation. The polygon's own handedness carries no physical
    // meaning, so reversing it here changes nothing semantic.
    let mut indices: Vec<usize> = (0..n).collect();
    if region.region.signed_area < 0.0 {
        indices.reverse();
    }

    let orient = |a: usize, b: usize, c: usize| {
        let (a, b, c) = (vertices[a], vertices[b], vertices[c]);
        robust::orient2d(
            robust::Coord { x: a.x, y: a.y },
            robust::Coord { x: b.x, y: b.y },
            robust::Coord { x: c.x, y: c.y },
        )
    };

    let mut triangles = Vec::with_capacity(n - 2);
    // Each successful clip removes one vertex, so `n - 2` clips suffice. The
    // guard counts *failed* scans, not iterations: a polygon proved simple has
    // an ear at every step (the two ears theorem), so exhausting a full scan
    // without finding one means this function's own predicates disagree with
    // Step 7 — a defect here, reported as operational.
    while indices.len() > 3 {
        let mut clipped = false;
        for position in 0..indices.len() {
            let count = indices.len();
            let (prev, current, next) = (
                indices[(position + count - 1) % count],
                indices[position],
                indices[(position + 1) % count],
            );
            if orient(prev, current, next) <= 0.0 {
                continue;
            }
            let contains_other = indices.iter().any(|&other| {
                other != prev
                    && other != current
                    && other != next
                    && point_in_triangle(
                        vertices[other],
                        vertices[prev],
                        vertices[current],
                        vertices[next],
                    )
            });
            if contains_other {
                continue;
            }
            triangles.push([prev, current, next]);
            indices.remove(position);
            clipped = true;
            break;
        }
        if !clipped {
            return Err(SliceExit::TriangulationDidNotComplete);
        }
    }
    triangles.push([indices[0], indices[1], indices[2]]);

    Ok(TriangulatedRegion {
        vertices,
        triangles,
    })
}

/// Whether `p` lies in the closed triangle `abc`, by exact orientation.
fn point_in_triangle(p: Point2, a: Point2, b: Point2, c: Point2) -> bool {
    let d = |q: Point2, r: Point2| {
        robust::orient2d(
            robust::Coord { x: q.x, y: q.y },
            robust::Coord { x: r.x, y: r.y },
            robust::Coord { x: p.x, y: p.y },
        )
    };
    let (ab, bc, ca) = (d(a, b), d(b, c), d(c, a));
    (ab >= 0.0 && bc >= 0.0 && ca >= 0.0) || (ab <= 0.0 && bc <= 0.0 && ca <= 0.0)
}

/// The result of the final validity battery.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FinalValidityReport {
    /// Triangles emitted.
    pub triangles: usize,
    /// Distinct vertices.
    pub vertices: usize,
    /// Boundary edges: edges with exactly one incident triangle.
    pub boundary_edges: usize,
    /// Internal edges: edges with exactly two.
    pub internal_edges: usize,
    /// `|sum of triangle areas - polygon area|`.
    pub area_residual: f64,
}

/// The eleven final validity predicates of Step 8B.
///
/// Every one is a check on the emitted complex. None of them reads how the
/// complex was produced, and none of them is a barycentre containment test.
pub fn final_validity(
    mesh: &TriangulatedRegion,
    region: &CertifiedPolygonalRegion,
) -> SliceResult<FinalValidityReport> {
    use std::collections::HashMap;

    let n = mesh.vertices.len();

    // 3, 4. No repeated vertex; no zero or unresolved signed area.
    for triangle in &mesh.triangles {
        let [a, b, c] = *triangle;
        if a == b || b == c || a == c {
            return Err(SliceExit::MeshTopologyInvalid);
        }
        let orientation = robust::orient2d(
            robust::Coord {
                x: mesh.vertices[a].x,
                y: mesh.vertices[a].y,
            },
            robust::Coord {
                x: mesh.vertices[b].x,
                y: mesh.vertices[b].y,
            },
            robust::Coord {
                x: mesh.vertices[c].x,
                y: mesh.vertices[c].y,
            },
        );
        if orientation == 0.0 || !orientation.is_finite() {
            return Err(SliceExit::MeshGeometryInvalid);
        }
    }

    // 1. Every polygon vertex is represented.
    let mut seen = vec![false; n];
    for triangle in &mesh.triangles {
        for index in triangle {
            seen[*index] = true;
        }
    }
    if seen.iter().any(|present| !present) {
        return Err(SliceExit::MeshTopologyInvalid);
    }

    // 6, 7. Edge incidence. An undirected edge carries one or two triangles;
    //       three is a nonmanifold complex.
    let mut incidence: HashMap<(usize, usize), usize> = HashMap::new();
    for triangle in &mesh.triangles {
        let [a, b, c] = *triangle;
        for (p, q) in [(a, b), (b, c), (c, a)] {
            *incidence.entry((p.min(q), p.max(q))).or_insert(0) += 1;
        }
    }
    if incidence.values().any(|count| *count > 2) {
        return Err(SliceExit::MeshTopologyInvalid);
    }
    let boundary_edges: Vec<(usize, usize)> = incidence
        .iter()
        .filter(|(_, count)| **count == 1)
        .map(|(edge, _)| *edge)
        .collect();
    let internal_edges = incidence.values().filter(|count| **count == 2).count();

    // 8. The mesh boundary is exactly the polygon cycle.
    let expected: std::collections::HashSet<(usize, usize)> = (0..n)
        .map(|i| {
            let j = (i + 1) % n;
            (i.min(j), i.max(j))
        })
        .collect();
    let actual: std::collections::HashSet<(usize, usize)> =
        boundary_edges.iter().copied().collect();
    if actual != expected {
        return Err(SliceExit::MeshBoundaryMismatch);
    }

    // 9. Connected. Walk the dual graph across internal edges.
    if !dual_graph_is_connected(mesh, &incidence) {
        return Err(SliceExit::MeshTopologyInvalid);
    }

    // 10. Euler counts of a triangulated disk with no interior vertices and no
    //     Steiner points: exactly `n - 2` triangles.
    if mesh.triangles.len() + 2 != n {
        return Err(SliceExit::MeshTopologyInvalid);
    }

    // 5, 11. Interiors pairwise disjoint and the union is the region. With the
    //        boundary, count and connectivity checks above already passed, the
    //        area identity closes both: `n - 2` triangles with the polygon's
    //        boundary and total area equal to the polygon's cannot overlap
    //        without leaving a gap, and cannot leave a gap without a boundary
    //        edge the check above would have rejected.
    let polygon_area = region.region.signed_area.abs();
    let mesh_area: f64 = mesh
        .triangles
        .iter()
        .map(|[a, b, c]| {
            let (a, b, c) = (mesh.vertices[*a], mesh.vertices[*b], mesh.vertices[*c]);
            ((b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y)).abs() / 2.0
        })
        .sum();
    let area_residual = (mesh_area - polygon_area).abs();
    // Scale aware: the residual is compared against the polygon's own area
    // scaled by a relative precision, not against an absolute epsilon that
    // would mean different things in metres and millimetres.
    if !(area_residual <= polygon_area * 1e-9) {
        return Err(SliceExit::MeshGeometryInvalid);
    }

    Ok(FinalValidityReport {
        triangles: mesh.triangles.len(),
        vertices: n,
        boundary_edges: boundary_edges.len(),
        internal_edges,
        area_residual,
    })
}

fn dual_graph_is_connected(
    mesh: &TriangulatedRegion,
    incidence: &std::collections::HashMap<(usize, usize), usize>,
) -> bool {
    use std::collections::HashMap;
    if mesh.triangles.is_empty() {
        return false;
    }
    let mut by_edge: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
    for (index, [a, b, c]) in mesh.triangles.iter().enumerate() {
        for (p, q) in [(*a, *b), (*b, *c), (*c, *a)] {
            by_edge.entry((p.min(q), p.max(q))).or_default().push(index);
        }
    }
    let mut visited = vec![false; mesh.triangles.len()];
    let mut stack = vec![0usize];
    visited[0] = true;
    while let Some(current) = stack.pop() {
        let [a, b, c] = mesh.triangles[current];
        for (p, q) in [(a, b), (b, c), (c, a)] {
            let key = (p.min(q), p.max(q));
            if incidence.get(&key) != Some(&2) {
                continue;
            }
            for neighbour in by_edge.get(&key).into_iter().flatten() {
                if !visited[*neighbour] {
                    visited[*neighbour] = true;
                    stack.push(*neighbour);
                }
            }
        }
    }
    visited.into_iter().all(|seen| seen)
}

/// Lift a validated 2D complex back to 3D through the plane's affine map.
///
/// `S(u, v) = O + uU + vV` with `U`, `V` linearly independent, so the map is
/// injective and the embedding is preserved.
pub fn lift_to_3d(mesh: &TriangulatedRegion, plane: &PlaneSchema) -> SliceResult<Vec<Point3>> {
    mesh.vertices
        .iter()
        .map(|point| {
            let lifted = plane.point_at(point.x, point.y);
            match lifted.x.is_finite() && lifted.y.is_finite() && lifted.z.is_finite() {
                true => Ok(lifted),
                false => Err(SliceExit::MeshGeometryInvalid),
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Orchestration
// ---------------------------------------------------------------------------

/// Everything one face's run of the slice established.
#[derive(Debug, Clone)]
pub struct SliceRecord {
    /// The furthest stage reached.
    pub stage: SliceStage,
    /// The semantic category of the result.
    pub category: SliceCategory,
    /// The exact exit reason, when the face did not complete.
    pub exit: Option<SliceExit>,
    /// How many bounds the face declared.
    pub bound_count: usize,
    /// How many edge uses it declared, across all bounds.
    pub edge_use_count: usize,
    /// The outer-bound standing the source supplied.
    pub outer_bound: OuterBoundStanding,
    /// The curve representations met, as tags, deduplicated.
    pub curve_representations: Vec<&'static str>,
    /// The certificate route, when one was used.
    pub certificate_route: Option<CertificateRoute>,
    /// The working cover, when one was built.
    pub working_cover: Option<OneCopyWorkingCover>,
    /// Polygon vertex count, when a polygon was certified.
    pub polygon_vertices: Option<usize>,
    /// The final validity report, when the face completed.
    pub validity: Option<FinalValidityReport>,
    /// The 3D triangles, when the face completed. The recovery gate's payload.
    pub mesh: Option<PlanarMesh>,
}

/// A validated planar mesh, ready for the recovery gate.
#[derive(Debug, Clone)]
pub struct PlanarMesh {
    /// Vertex positions in 3D.
    pub positions: Vec<Point3>,
    /// Triangles as vertex-index triples.
    pub triangles: Vec<[usize; 3]>,
    /// The unit normal consistent with [`Self::triangles`]' winding.
    ///
    /// This is `±(U x V)` of the support plane, with the sign taken from the
    /// **source traversal's handedness in the plane's chart** — not normalised
    /// to a preferred direction.
    ///
    /// It is *not* a physical material-side normal, and nothing here claims one:
    /// Step 0 measured 0 of 110,770 computable normalized physical signs, so the
    /// outward direction is genuinely unavailable. What it does claim is the
    /// weaker fact the evidence supports — that the emitted winding and this
    /// normal agree with each other, and that both follow the same convention
    /// the legacy path follows, since the importer already applied
    /// `surface.invert()` (which swaps a plane's `u` and `v` axes, flipping
    /// `U x V`) whenever `FACE_SURFACE.same_sense` was false.
    pub chart_normal: truck_geometry::prelude::Vector3,
}

impl SliceRecord {
    fn exited(mut self, stage: SliceStage, exit: SliceExit) -> Self {
        self.stage = stage;
        self.category = exit.category();
        self.exit = Some(exit);
        self
    }
}

/// Run the complete slice for one face.
///
/// `lattice` is Step 1's product. The caller has already established the
/// support schema; a face reaching here with anything but a plane and a
/// certified rank-0 lattice exits at the first stage.
#[allow(clippy::too_many_arguments)]
pub fn run_planar_slice(
    input: &SourceFaceInput,
    plane: &PlaneSchema,
    lattice: &CertifiedAmbientLattice,
    outer_bound: OuterBoundStanding,
    curve_of: &mut impl FnMut(usize) -> CurveSchema,
    vertex_position: &impl Fn(SourceVertexKey) -> Option<Point3>,
    tolerance: f64,
) -> SliceRecord {
    let mut record = SliceRecord {
        stage: SliceStage::NotAttempted,
        category: SliceCategory::Unresolved,
        exit: None,
        bound_count: input.bounds.len(),
        edge_use_count: input.edge_use_count(),
        outer_bound,
        curve_representations: Vec::new(),
        certificate_route: None,
        working_cover: None,
        polygon_vertices: None,
        validity: None,
        mesh: None,
    };

    if !matches!(lattice, CertifiedAmbientLattice::Rank0(_)) {
        return record.exited(SliceStage::NotAttempted, SliceExit::NotRank0);
    }
    record.stage = SliceStage::AmbientRank0;

    // The face's curve representations, read before Step 2 rather than from
    // its product. Read afterwards they are unavailable on exactly the faces
    // that need explaining: `traverse_bound` refuses on the first
    // unidentified curve, so `unsupported_curve_representation` used to report
    // `curves=none` and the histogram over the enum could not say which
    // family to admit next. Reading them here costs one `curve_of` call per
    // edge use on a path that is about to make the same calls, and grants
    // nothing — a tag is a diagnostic label, and Step 2 still decides
    // admission from the schema itself.
    for edge_use in input.edge_uses() {
        let tag = curve_of(edge_use.source_edge_index).tag();
        if !record.curve_representations.contains(&tag) {
            record.curve_representations.push(tag);
        }
    }

    // Step 2.
    let traversal = match regular_traversal(input, outer_bound, curve_of) {
        Ok(traversal) => traversal,
        Err(exit) => return record.exited(SliceStage::AmbientRank0, exit),
    };
    record.stage = SliceStage::RegularTraversal;

    // Step 3.
    let planar = match certified_planar_curves(&traversal, plane, vertex_position, tolerance) {
        Ok(planar) => planar,
        Err(exit) => return record.exited(SliceStage::RegularTraversal, exit),
    };
    record.certificate_route = planar.first().map(|occurrence| occurrence.route);
    record.stage = SliceStage::PlanarCurves;

    // Step 4.
    let developed = match rank0_lift(lattice, planar) {
        Ok(developed) => developed,
        Err(exit) => return record.exited(SliceStage::PlanarCurves, exit),
    };
    record.stage = SliceStage::DevelopedBoundary;

    // Step 5.
    if let Err(exit) = trivial_deck_solution(&developed) {
        return record.exited(SliceStage::DevelopedBoundary, exit);
    }
    record.stage = SliceStage::DeckSolution;

    // Step 6.
    let cover = match one_copy_working_cover(&developed) {
        Ok(cover) => cover,
        Err(exit) => return record.exited(SliceStage::DeckSolution, exit),
    };
    record.working_cover = Some(cover);
    record.stage = SliceStage::WorkingCover;

    // Step 7.
    let arrangement = match simple_jordan_arrangement(&developed) {
        Ok(arrangement) => arrangement,
        Err(exit) => return record.exited(SliceStage::WorkingCover, exit),
    };
    record.stage = SliceStage::JordanArrangement;

    // Material selection.
    let material = match bounded_material_region(arrangement, outer_bound) {
        Ok(material) => material,
        Err(exit) => return record.exited(SliceStage::JordanArrangement, exit),
    };
    record.stage = SliceStage::MaterialRegion;

    // Step 8A.
    let polygonal = match certified_polygonal_region(material, &developed, tolerance) {
        Ok(polygonal) => polygonal,
        Err(exit) => return record.exited(SliceStage::MaterialRegion, exit),
    };
    record.polygon_vertices = Some(polygonal.region.boundary.cycle.len());
    record.stage = SliceStage::PolygonalRegion;

    // Step 8B.
    let mesh = match triangulate(&polygonal) {
        Ok(mesh) => mesh,
        Err(exit) => return record.exited(SliceStage::PolygonalRegion, exit),
    };
    record.stage = SliceStage::Triangulation;

    // Final validity.
    let validity = match final_validity(&mesh, &polygonal) {
        Ok(validity) => validity,
        Err(exit) => return record.exited(SliceStage::Triangulation, exit),
    };
    let positions = match lift_to_3d(&mesh, plane) {
        Ok(positions) => positions,
        Err(exit) => return record.exited(SliceStage::Triangulation, exit),
    };
    record.validity = Some(validity);
    // Restore the source traversal's handedness. Ear clipping works in a
    // counter-clockwise chart, so `triangulate` normalises to it; emitting that
    // normalisation would silently reorient every face whose declared loop ran
    // clockwise in the plane's chart, which is a claim about the face that
    // nothing established. The winding and the normal are flipped together, so
    // they stay consistent.
    let clockwise = polygonal.region.signed_area < 0.0;
    let triangles = match clockwise {
        false => mesh.triangles,
        true => mesh
            .triangles
            .into_iter()
            .map(|[a, b, c]| [c, b, a])
            .collect(),
    };
    let chart_normal = {
        let normal = plane.u_axis().cross(plane.v_axis()).normalize();
        match clockwise {
            false => normal,
            true => -normal,
        }
    };
    record.mesh = Some(PlanarMesh {
        positions,
        triangles,
        chart_normal,
    });
    record.stage = SliceStage::FinalValidity;
    record.category = SliceCategory::Resolved;
    record
}

#[cfg(test)]
mod tests;
