//! Step 3-arc and Step 7-arc: the developed-curve track for a planar rank-0
//! face whose boundary carries circular arcs.
//!
//! # Why this module exists
//!
//! [`super::planar_slice::certified_planar_curves`] discharges its
//! whole-interval curve-on-surface obligation by requiring the source
//! representation to be *exactly polygonal*, and everything downstream of it
//! inherits that: Step 7 classifies straight segment pairs, Step 8A's
//! approximation error is zero by construction, and Step 8B's battery checks a
//! polygon cycle. That chain is why the slice landed, and it is also why an
//! arc cannot enter it — `CurveSchema::polygonal()` is `None` for a circle, so
//! every arc-bounded planar face exits `UnsupportedCurveRepresentation`
//! whatever else is true of it.
//!
//! Measured over the corpus at the WAVE-3A pin, that is not a corner: of 3,251
//! planar faces the legacy tessellator loses, **2,131 exit on the curve family
//! and 1,120 before that on outer-bound standing, and none reach Step 7 at
//! all**. The arrangement work the handoff ranks sixth has an empty input
//! population until an arc can be developed, which is what this module does.
//!
//! # What is built here, and what is deliberately not
//!
//! This module carries a planar face's boundary from source evidence to
//! [`super::curve2d::DevelopedCurve2D`] occurrences — the analytic
//! line-and-arc representation the whole ARR-002 / GEN-001 substrate consumes
//! — and then certifies the arrangement of those occurrences pairwise through
//! [`super::xmonotone::make_x_monotone`] and
//! [`super::intersection::intersect_x_monotone`].
//!
//! It stops there. It produces **no mesh**, and nothing in it can replace a
//! legacy result: it is an observer whose whole output is a typed record. The
//! remaining stages — a certified polygonal approximation of an arc within the
//! caller's tolerance (Step 8A-arc), and face extraction plus §X parity
//! selection where the arrangement is *not* a simple Jordan curve (ARR-003) —
//! are separate builds, and both need the population this record measures
//! before they can be scoped honestly.
//!
//! # Why an arc's planar development needs no sampling
//!
//! A circle is
//!
//! ```text
//! P(t) = C + cos(t) * A + sin(t) * B
//! ```
//!
//! and the plane's chart map is affine. So the whole trimmed occurrence lies
//! on the support plane exactly when three vector conditions hold — `C` lies
//! on the plane, and `A` and `B` are both parallel to it — and under those
//! conditions the developed curve is the *same* parameterization with `C`,
//! `A`, `B` replaced by their chart images. There is no interval to bound and
//! no samples to check: three tests discharge the complete-interval
//! obligation, and the developed arc is exact.
//!
//! That is the same argument [`super::planar_slice`] makes for a polygonal
//! chain (an affine map of a convex combination), applied to the other family
//! whose image under an affine map stays in its own family.

use super::super::source_evidence::{EdgeUseId, SourceVertexKey};
use super::curve2d::{
    CurveOccurrenceProvenance, DevelopedCurve2D, DirectedCircularArc2, LineSegment2, SourceEdgeId,
    SourceEntityId, SourceFaceId,
};
use super::intersection::{intersect_x_monotone, IntersectionPolicy, PairIntersectionResult};
use super::planar_slice::{RegularClosedTraversal, SliceCategory};
use super::support::{CircularArcPlacement3, PlaneSchema};
use super::xmonotone::{make_x_monotone, NumericalPolicy, XMonotonePiece2};
use truck_geometry::prelude::{InnerSpace, Point2, Point3, Vector2, Vector3};

// ---------------------------------------------------------------------------
// Exits
// ---------------------------------------------------------------------------

/// Every way the developed-curve track can leave.
///
/// Kept as its own enum rather than folded into
/// [`super::planar_slice::SliceExit`]: that enum's histogram is the frozen
/// funnel the corpus sweeps compare against across pins, and this track is an
/// observer that must not perturb it. The two are joined on
/// `source_face_id` by the sweep, not by sharing a type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevelopedExit {
    /// No occurrence carried a curve family this track reads. The face is the
    /// polygonal slice's population and this track claims nothing about it.
    NoDevelopableCurve,
    /// An occurrence's curve representation is neither polygonal nor a
    /// certified circle.
    UnsupportedCurveRepresentation,
    /// The plane basis is too ill conditioned to bound the chart inverse.
    IllConditionedPlaneBasis,
    /// A chart coordinate was not finite.
    ProjectionNotFinite,
    /// A polygonal occurrence's chain has fewer than two vertices, so it spans
    /// no segment.
    DegenerateChain,
    /// An arc's circle does not lie in the support plane: its center is off
    /// the plane, or a basis vector is not parallel to it, by more than the
    /// caller's tolerance.
    ///
    /// A *proved* fact about the face against the declared support surface,
    /// not a numerical shortfall — the residual is a distance in model units
    /// compared against the tolerance the pipeline validates at.
    ArcSupportOffPlane,
    /// A projected curve endpoint disagrees with its source vertex.
    CurveSurfaceInconsistency,
    /// An occurrence could not be decomposed into certified x-monotone pieces.
    ///
    /// Carries the decomposition's own failure rather than collapsing it: the
    /// six causes have three different categories between them, and "the
    /// interval is degenerate" (a closed circular edge whose trim collapsed at
    /// import) is a completely different piece of work from "the critical
    /// classification is undecided".
    MonotoneDecompositionFailed(super::xmonotone::MonotoneDecompositionFailure),
    /// A nonadjacent pair of occurrences crosses. **The arrangement is real**:
    /// the crossing is certified on the analytic curves, not observed on a
    /// polyline approximation of them.
    NonadjacentCrossing,
    /// An adjacent pair meets somewhere besides its declared shared endpoint.
    AdjacentPairHasExtraIntersection,
    /// A pair's intersection is valid geometry outside ARR-002's admitted
    /// envelope.
    PairUnsupported,
    /// A pair's intersection could not be certified under the declared
    /// numerical policy. Says nothing about the face.
    PairUnresolved,
}

impl DevelopedExit {
    /// Which semantic category this exit belongs to, on the same reading
    /// [`super::planar_slice::SliceExit::category`] applies.
    pub fn category(self) -> SliceCategory {
        match self {
            Self::NoDevelopableCurve
            | Self::MonotoneDecompositionFailed(_)
            | Self::PairUnresolved => SliceCategory::Unresolved,
            Self::UnsupportedCurveRepresentation
            | Self::ArcSupportOffPlane
            | Self::DegenerateChain
            | Self::NonadjacentCrossing
            | Self::PairUnsupported => SliceCategory::Unsupported,
            Self::CurveSurfaceInconsistency | Self::AdjacentPairHasExtraIntersection => {
                SliceCategory::Inconsistent
            }
            Self::IllConditionedPlaneBasis | Self::ProjectionNotFinite => {
                SliceCategory::OperationalFailure
            }
        }
    }

    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::NoDevelopableCurve => "no_developable_curve",
            Self::UnsupportedCurveRepresentation => "unsupported_curve_representation",
            Self::IllConditionedPlaneBasis => "ill_conditioned_plane_basis",
            Self::ProjectionNotFinite => "projection_not_finite",
            Self::DegenerateChain => "degenerate_chain",
            Self::ArcSupportOffPlane => "arc_support_off_plane",
            Self::CurveSurfaceInconsistency => "curve_surface_inconsistency",
            Self::MonotoneDecompositionFailed(cause) => cause.tag(),
            Self::NonadjacentCrossing => "nonadjacent_crossing",
            Self::AdjacentPairHasExtraIntersection => "adjacent_pair_has_extra_intersection",
            Self::PairUnsupported => "pair_unsupported",
            Self::PairUnresolved => "pair_unresolved",
        }
    }
}

type DevelopedResult<T> = Result<T, DevelopedExit>;

// ---------------------------------------------------------------------------
// Step 3-arc — develop the boundary into analytic planar occurrences
// ---------------------------------------------------------------------------

/// Develop one loop's occurrences into analytic planar curves.
///
/// A polygonal occurrence contributes one [`LineSegment2`] per source segment,
/// so the developed boundary is *finer* than the source occurrence list and
/// each piece keeps the occurrence's provenance verbatim. A circular
/// occurrence contributes exactly one [`DirectedCircularArc2`] over its whole
/// authoritative interval — the arc is never split into chords here, which is
/// the entire point of the track.
///
/// `tolerance` is the caller's, in model units, and is what the on-plane
/// obligations are discharged against; nothing here consults a global epsilon.
pub fn develop_planar_curves(
    traversal: &RegularClosedTraversal,
    plane: &PlaneSchema,
    vertex_position: &impl Fn(SourceVertexKey) -> Option<Point3>,
    source_face_id: Option<u64>,
    tolerance: f64,
) -> DevelopedResult<Vec<DevelopedCurve2D>> {
    if !plane_inverse_is_conditioned(plane, tolerance) {
        return Err(DevelopedExit::IllConditionedPlaneBasis);
    }
    let gram = plane.gram();
    // The chart image of a *point*: solve the Gram system for the offset from
    // the plane origin. The chart image of a *vector* is the same solve
    // without the origin subtraction — a vector has no origin, and applying
    // one would translate every basis of every arc by the chart image of the
    // plane's own origin.
    let project_point = |point: Point3| -> DevelopedResult<Point2> {
        let offset = point - plane.origin();
        finite_point(gram.solve(offset.dot(plane.u_axis()), offset.dot(plane.v_axis())))
    };
    let project_vector = |vector: Vector3| -> DevelopedResult<Vector2> {
        let (x, y) = gram.solve(vector.dot(plane.u_axis()), vector.dot(plane.v_axis()));
        let point = finite_point((x, y))?;
        Ok(Vector2::new(point.x, point.y))
    };

    let normal = plane.u_axis().cross(plane.v_axis());
    let normal_magnitude = normal.magnitude();
    if !(normal_magnitude > 0.0) || !normal_magnitude.is_finite() {
        return Err(DevelopedExit::IllConditionedPlaneBasis);
    }
    // Distance from the plane, measured along the *unit* normal, so the value
    // is in model units and comparable with the caller's tolerance however the
    // retained basis happens to be scaled.
    let off_plane = |v: Vector3| f64::abs(v.dot(normal)) / normal_magnitude;

    let mut developed: Vec<DevelopedCurve2D> = Vec::new();
    let mut saw_developable = false;

    for occurrence in &traversal.occurrences {
        let provenance = provenance_of(occurrence, source_face_id);
        let start = vertex_position(occurrence.start_vertex)
            .ok_or(DevelopedExit::CurveSurfaceInconsistency)?;
        let end = vertex_position(occurrence.end_vertex)
            .ok_or(DevelopedExit::CurveSurfaceInconsistency)?;

        if let Some(polygonal) = occurrence.curve.polygonal() {
            saw_developable = true;
            let mut chain: Vec<Point3> = polygonal.vertices().to_vec();
            if !occurrence.forward {
                chain.reverse();
            }
            if chain.len() < 2 {
                return Err(DevelopedExit::DegenerateChain);
            }
            // The same endpoint reconciliation the polygonal Step 3 performs,
            // and for the same reason: the source vertices are the
            // authoritative shared endpoint occurrences, so adjacent
            // occurrences meet bit-exactly rather than nearly.
            let last = chain.len() - 1;
            if (chain[0] - start).magnitude() > tolerance
                || (chain[last] - end).magnitude() > tolerance
            {
                return Err(DevelopedExit::CurveSurfaceInconsistency);
            }
            chain[0] = start;
            chain[last] = end;
            for point in &chain {
                if off_plane(*point - plane.origin()) > tolerance {
                    return Err(DevelopedExit::CurveSurfaceInconsistency);
                }
            }
            for window in chain.windows(2) {
                developed.push(DevelopedCurve2D::Line(LineSegment2 {
                    start: project_point(window[0])?,
                    end: project_point(window[1])?,
                    provenance,
                }));
            }
            continue;
        }

        if let Some(placement) = occurrence.curve.circular_arc() {
            saw_developable = true;
            developed.push(DevelopedCurve2D::CircularArc(develop_arc(
                placement,
                occurrence.forward,
                occurrence.start_vertex == occurrence.end_vertex,
                provenance,
                start,
                end,
                &project_point,
                &project_vector,
                &off_plane,
                plane,
                tolerance,
            )?));
            continue;
        }

        return Err(DevelopedExit::UnsupportedCurveRepresentation);
    }

    match saw_developable {
        true => Ok(developed),
        false => Err(DevelopedExit::NoDevelopableCurve),
    }
}

/// Develop one certified circular occurrence into the plane's chart.
///
/// The three on-plane obligations are discharged here and nowhere else, and
/// they are the *whole* complete-interval obligation for this family — see the
/// module docs for why three vector tests suffice where a spline would need a
/// bound.
#[allow(clippy::too_many_arguments)]
fn develop_arc(
    placement: &CircularArcPlacement3,
    forward: bool,
    closed_occurrence: bool,
    provenance: CurveOccurrenceProvenance,
    start: Point3,
    end: Point3,
    project_point: &impl Fn(Point3) -> DevelopedResult<Point2>,
    project_vector: &impl Fn(Vector3) -> DevelopedResult<Vector2>,
    off_plane: &impl Fn(Vector3) -> f64,
    plane: &PlaneSchema,
    tolerance: f64,
) -> DevelopedResult<DirectedCircularArc2> {
    let (t0, t1) = placement.parameter_interval;
    if !t0.is_finite() || !t1.is_finite() {
        return Err(DevelopedExit::ProjectionNotFinite);
    }

    // The closed-edge rule, restated from [`super::curve_witness`] because
    // this is the second route that needs it:
    //
    // > An `edge_curve` whose two ends are the *same* source vertex, carried
    // > on a circle, represents exactly one complete traversal of that
    // > circle — one full period, once, in the curve's own parameter
    // > direction.
    //
    // An importer recovers an edge's trim by solving each of its two vertex
    // points onto the curve. When the edge is closed those are the *same*
    // solve, so the interval degenerates to `(u, u)` — not because the source
    // declared a zero sweep, but because coincident endpoints cannot carry an
    // extent. Measured on `00009190`, this is 172 of the 207 planar lost faces
    // the developed track reaches, so it is the difference between the track
    // seeing this corpus and not.
    //
    // Two bounds, both enforced rather than assumed:
    //
    // - *once*, not `k` times. The rule licenses exactly one period; a
    //   multiply-wound occurrence would need an authoritative turn count that
    //   no closed `edge_curve` carries, and none is invented here.
    // - *closed by identity*, not by coincidence. Two distinct source vertices
    //   at one point are a zero-length edge, not a closed one; that case keeps
    //   the degenerate interval and is refused by the decomposition below.
    let (t0, t1) = match t0 == t1 && closed_occurrence {
        // The period is added in the curve's *own* parameter direction, which
        // is where `parameter_interval` already lives — the traversal fold is
        // applied once, below, to the completed interval.
        true => (t0, t0 + std::f64::consts::TAU),
        false => (t0, t1),
    };

    // 1. The circle's center lies on the plane.
    if off_plane(placement.center - plane.origin()) > tolerance {
        return Err(DevelopedExit::ArcSupportOffPlane);
    }
    // 2 and 3. Both basis vectors are parallel to the plane. The residual is
    //          measured at the circle's own scale — a basis vector *is* a
    //          radius — so the test is "the furthest the circle strays from
    //          the plane", in model units, exactly like the center's.
    if off_plane(placement.cos_basis) > tolerance || off_plane(placement.sin_basis) > tolerance {
        return Err(DevelopedExit::ArcSupportOffPlane);
    }

    // The traversal fold, applied exactly once. Reversing the *occurrence*
    // swaps the interval ends; it never negates the basis, because the basis
    // belongs to the source curve and not to this use of it.
    let (t0, t1) = match forward {
        true => (t0, t1),
        false => (t1, t0),
    };

    let arc = DirectedCircularArc2 {
        center: project_point(placement.center)?,
        cos_basis: project_vector(placement.cos_basis)?,
        sin_basis: project_vector(placement.sin_basis)?,
        t0,
        t1,
        provenance,
    };

    // Endpoint correspondence, in the chart. The developed arc's own ends must
    // be the source vertices this edge use declares it meets. This is the
    // check that would catch a mis-folded interval or a placement whose
    // parameter origin does not match the importer's, and it is asked in 2D
    // because that is where the arc will be consumed.
    //
    // The tolerance is the caller's, carried through the chart: a chart unit
    // is a model unit only when the plane basis is unit-length, so the
    // comparison is made in 3D by evaluating the *source* circle rather than
    // by inventing a chart-space epsilon.
    let evaluate3 =
        |t: f64| placement.center + t.cos() * placement.cos_basis + t.sin() * placement.sin_basis;
    if (evaluate3(arc.t0) - start).magnitude() > tolerance
        || (evaluate3(arc.t1) - end).magnitude() > tolerance
    {
        return Err(DevelopedExit::CurveSurfaceInconsistency);
    }

    Ok(arc)
}

/// The occurrence's complete provenance, carried verbatim into every piece the
/// development produces.
fn provenance_of(
    occurrence: &super::planar_slice::TraversalOccurrence,
    source_face_id: Option<u64>,
) -> CurveOccurrenceProvenance {
    let EdgeUseId { bound, .. } = occurrence.edge_use;
    CurveOccurrenceProvenance {
        source_face_id: source_face_id.map(SourceFaceId),
        bound_id: bound,
        edge_use_id: occurrence.edge_use,
        source_edge_id: SourceEdgeId(occurrence.source_edge_index),
        start_vertex_id: occurrence.start_vertex,
        end_vertex_id: occurrence.end_vertex,
        // The importer seam does not retain a curve entity id at this stage.
        // Absence is preserved rather than filled with the edge's id, which
        // would claim an identity the source never supplied.
        source_curve_entity_id: None::<SourceEntityId>,
    }
}

fn finite_point((x, y): (f64, f64)) -> DevelopedResult<Point2> {
    match x.is_finite() && y.is_finite() {
        true => Ok(Point2::new(x, y)),
        false => Err(DevelopedExit::ProjectionNotFinite),
    }
}

/// The same dimensionless conditioning criterion
/// [`super::planar_slice`] applies, restated here so this track does not
/// depend on that module's private helper.
fn plane_inverse_is_conditioned(plane: &PlaneSchema, tolerance: f64) -> bool {
    let normalised = plane.gram().normalised_determinant().get();
    normalised > 0.0 && tolerance.is_finite() && tolerance > 0.0 && normalised >= 1e-6
}

// ---------------------------------------------------------------------------
// Step 7-arc — certify the arrangement on the analytic curves
// ---------------------------------------------------------------------------

/// What the pairwise arrangement certification established.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArrangementSurvey {
    /// How many developed occurrences the loop carries.
    pub occurrences: usize,
    /// How many are circular arcs.
    pub arcs: usize,
    /// How many x-monotone pieces they decompose into.
    pub pieces: usize,
    /// How many pairs were skipped as intra-occurrence.
    ///
    /// Reported rather than silently dropped: it is the size of the obligation
    /// this survey does *not* discharge (see [`survey_arrangement`]).
    pub intra_occurrence_pairs: usize,
    /// How many nonadjacent piece pairs were certified to cross.
    ///
    /// **The number package 6 turns on.** Zero over a face means the legacy
    /// tessellator's `ConstraintInsertionIncomplete` on it was an artefact of
    /// approximating arcs by chords, and the face needs no arrangement — only
    /// an exact Step 8A. Nonzero means the crossing is real and face
    /// extraction plus parity selection is genuinely required.
    pub certified_crossings: usize,
}

/// Step 7-arc. Certify the pairwise arrangement of a developed loop.
///
/// The same obligation [`super::planar_slice::jordan_arrangement_of`]
/// discharges over segment pairs, discharged over x-monotone piece pairs by
/// ARR-002's certified solvers instead. Adjacency is inherited from the
/// developed order: consecutive pieces share a declared endpoint, and every
/// other pair must be disjoint.
///
/// Returns the survey rather than a boundary: this track builds no mesh, and a
/// crossing is reported as a *count* so a corpus sweep can rank the population
/// instead of losing it to the first refusal.
///
/// # Pairs from one occurrence are skipped, and why that is not a gap here
///
/// A complete circle decomposes into two x-monotone pieces that share the same
/// support circle by construction, and consecutive segments of one polyline
/// share their occurrence's identity. Asking ARR-002's pairwise solver about
/// such a pair asks it about a curve and itself: the circle case returns
/// `Unsupported` (coincident support circles), which is a correct answer to
/// the wrong question. Before this exclusion those self-pairs were 5,090 of
/// `00009190`'s bound records and hid every real result behind them.
///
/// The obligation they *would* have discharged is that each occurrence is
/// simple on its own interval — [`super::planar_slice::SliceExit::IndividualCurveNotSimple`],
/// a per-curve question with a per-family answer, not a pairwise one. This
/// survey does not discharge it and does not claim to; the skipped count is
/// reported so the size of what is deferred is visible.
pub fn survey_arrangement(developed: &[DevelopedCurve2D]) -> DevelopedResult<ArrangementSurvey> {
    let monotone_policy = NumericalPolicy::standard();
    let intersection_policy = IntersectionPolicy::standard();

    let arcs = developed
        .iter()
        .filter(|curve| matches!(curve, DevelopedCurve2D::CircularArc(_)))
        .count();

    let mut pieces: Vec<XMonotonePiece2> = Vec::new();
    for curve in developed {
        let decomposed = make_x_monotone(curve, &monotone_policy)
            .map_err(DevelopedExit::MonotoneDecompositionFailed)?;
        pieces.extend(decomposed);
    }

    let n = pieces.len();
    if n < 2 {
        return Ok(ArrangementSurvey {
            occurrences: developed.len(),
            arcs,
            pieces: n,
            intra_occurrence_pairs: 0,
            certified_crossings: 0,
        });
    }

    // Which source occurrence each piece came from. Identity, read from the
    // provenance the decomposition preserved verbatim — never from
    // coordinates, which is the inference the arrangement must not make.
    let origin: Vec<EdgeUseId> = pieces
        .iter()
        .map(|piece| piece.source_curve_copy().provenance().edge_use_id)
        .collect();

    let mut certified_crossings = 0usize;
    let mut intra_occurrence_pairs = 0usize;
    for i in 0..n {
        for j in (i + 1)..n {
            if origin[i] == origin[j] {
                intra_occurrence_pairs += 1;
                continue;
            }
            // Cyclic adjacency: the loop closes, so the last piece and the
            // first are adjacent too. Adjacent pieces are *expected* to meet,
            // at their shared endpoint and nowhere else.
            let adjacent = j == i + 1 || (i == 0 && j == n - 1);
            match intersect_x_monotone(&pieces[i], &pieces[j], &intersection_policy) {
                PairIntersectionResult::Disjoint => {}
                PairIntersectionResult::Intersections(found) => match adjacent {
                    // One intersection between adjacent pieces is the join the
                    // source declared. More than one means the pair meets
                    // somewhere besides it, which contradicts the source.
                    true => {
                        if found.len() > 1 {
                            return Err(DevelopedExit::AdjacentPairHasExtraIntersection);
                        }
                    }
                    false => certified_crossings += found.len(),
                },
                PairIntersectionResult::Unsupported(_) => {
                    return Err(DevelopedExit::PairUnsupported)
                }
                PairIntersectionResult::Unresolved(_) => return Err(DevelopedExit::PairUnresolved),
            }
        }
    }

    Ok(ArrangementSurvey {
        occurrences: developed.len(),
        arcs,
        pieces: n,
        intra_occurrence_pairs,
        certified_crossings,
    })
}

// ---------------------------------------------------------------------------
// One face's record
// ---------------------------------------------------------------------------

/// The developed-curve track's verdict on one loop.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DevelopedRecord {
    /// The survey, when both stages resolved.
    pub survey: Option<ArrangementSurvey>,
    /// The exit, when one did not.
    pub exit: Option<DevelopedExit>,
}

impl DevelopedRecord {
    /// A short stable tag naming the outcome.
    pub fn tag(&self) -> &'static str {
        match self.exit {
            Some(exit) => exit.tag(),
            None => "resolved",
        }
    }
}

/// One face's verdict: every bound surveyed, independently.
#[derive(Debug, Clone, PartialEq)]
pub struct DevelopedFaceRecord {
    /// How many bounds the face declares.
    pub bound_count: usize,
    /// One record per bound, in source order.
    pub bounds: Vec<DevelopedRecord>,
}

impl DevelopedFaceRecord {
    /// The face's aggregate crossing count, when *every* bound resolved.
    ///
    /// `None` when any bound exited: a partial count would understate the
    /// arrangement, and understating it is the failure mode that would make
    /// the ARR-003 scoping decision wrong.
    pub fn certified_crossings(&self) -> Option<usize> {
        self.bounds
            .iter()
            .map(|record| record.survey.map(|survey| survey.certified_crossings))
            .sum()
    }

    /// The first bound's exit, or `None` when every bound resolved.
    pub fn first_exit(&self) -> Option<DevelopedExit> {
        self.bounds.iter().find_map(|record| record.exit)
    }
}

/// Run the track over every bound of one face.
///
/// Deliberately independent of [`truck_topology::compress::OuterBoundStanding`]:
/// 1,120 of the corpus's lost planar faces declare no outer bound, and a
/// diagnostic that needs the standing before it can look at the geometry
/// cannot say anything about exactly the population that most needs
/// explaining. Which loop is outer matters to material selection, which this
/// track does not perform.
pub fn run_developed_face(
    input: &super::super::source_evidence::SourceFaceInput,
    plane: &PlaneSchema,
    curve_of: &mut impl FnMut(usize) -> super::support::CurveSchema,
    vertex_position: &impl Fn(SourceVertexKey) -> Option<Point3>,
    tolerance: f64,
) -> DevelopedFaceRecord {
    let mut bounds = Vec::with_capacity(input.bounds.len());
    for bound in &input.bounds {
        let record = match super::planar_slice::traverse_bound(bound, curve_of) {
            Ok(traversal) => run_developed_track(
                &traversal,
                plane,
                vertex_position,
                input.source_face_id,
                tolerance,
            ),
            // A bound Step 2 cannot traverse carries no developable loop. The
            // track reports that as its own exit rather than borrowing the
            // traversal's, which belongs to the frozen funnel.
            Err(_) => DevelopedRecord {
                survey: None,
                exit: Some(DevelopedExit::NoDevelopableCurve),
            },
        };
        bounds.push(record);
    }
    DevelopedFaceRecord {
        bound_count: input.bounds.len(),
        bounds,
    }
}

/// Run both stages over one loop.
pub fn run_developed_track(
    traversal: &RegularClosedTraversal,
    plane: &PlaneSchema,
    vertex_position: &impl Fn(SourceVertexKey) -> Option<Point3>,
    source_face_id: Option<u64>,
    tolerance: f64,
) -> DevelopedRecord {
    let developed =
        match develop_planar_curves(traversal, plane, vertex_position, source_face_id, tolerance) {
            Ok(developed) => developed,
            Err(exit) => {
                return DevelopedRecord {
                    survey: None,
                    exit: Some(exit),
                }
            }
        };
    match survey_arrangement(&developed) {
        Ok(survey) => DevelopedRecord {
            survey: Some(survey),
            exit: None,
        },
        Err(exit) => DevelopedRecord {
            survey: None,
            exit: Some(exit),
        },
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source_evidence::BoundId;

    const TAU: f64 = std::f64::consts::TAU;

    /// Distinct occurrences need distinct edge-use identities, because
    /// [`survey_arrangement`] skips intra-occurrence pairs by exactly that
    /// identity. A fixture that reused one id would silently skip every pair
    /// and report zero crossings for anything.
    fn provenance(local: usize) -> CurveOccurrenceProvenance {
        CurveOccurrenceProvenance {
            source_face_id: Some(SourceFaceId(1)),
            bound_id: BoundId(0),
            edge_use_id: EdgeUseId::new(BoundId(0), local),
            source_edge_id: SourceEdgeId(local),
            start_vertex_id: SourceVertexKey::ShellVertex(local),
            end_vertex_id: SourceVertexKey::ShellVertex(local + 1),
            source_curve_entity_id: None,
        }
    }

    fn segment(local: usize, start: (f64, f64), end: (f64, f64)) -> DevelopedCurve2D {
        DevelopedCurve2D::Line(LineSegment2 {
            start: Point2::new(start.0, start.1),
            end: Point2::new(end.0, end.1),
            provenance: provenance(local),
        })
    }

    /// A unit square, traversed once. Simple, so no pair may cross.
    fn square() -> Vec<DevelopedCurve2D> {
        vec![
            segment(0, (0.0, 0.0), (1.0, 0.0)),
            segment(1, (1.0, 0.0), (1.0, 1.0)),
            segment(2, (1.0, 1.0), (0.0, 1.0)),
            segment(3, (0.0, 1.0), (0.0, 0.0)),
        ]
    }

    #[test]
    fn a_simple_square_certifies_no_crossing() {
        let survey = survey_arrangement(&square()).expect("a square surveys");
        assert_eq!(survey.certified_crossings, 0);
        assert_eq!(survey.occurrences, 4);
        assert_eq!(survey.arcs, 0);
    }

    /// The positive control, and the reason the corpus result means anything.
    ///
    /// "Zero certified crossings everywhere" is only evidence that boundaries
    /// do not cross if this survey is *able* to report one. A crossed
    /// quadrilateral — the same four corners wired into a figure eight — must
    /// come back with a crossing.
    #[test]
    fn a_self_crossing_boundary_certifies_the_crossing() {
        let crossed = vec![
            segment(0, (0.0, 0.0), (1.0, 1.0)),
            segment(1, (1.0, 1.0), (1.0, 0.0)),
            segment(2, (1.0, 0.0), (0.0, 1.0)),
            segment(3, (0.0, 1.0), (0.0, 0.0)),
        ];
        let survey = survey_arrangement(&crossed).expect("the crossed loop surveys");
        assert!(
            survey.certified_crossings > 0,
            "a figure-eight boundary must certify at least one crossing, \
             otherwise a zero elsewhere proves nothing"
        );
    }

    /// A full circle decomposes into two x-monotone pieces that share one
    /// support circle. Asking the pairwise solver about that pair asks it
    /// about a curve and itself; the survey must skip it and say how many it
    /// skipped, not refuse the face.
    #[test]
    fn the_two_pieces_of_one_circle_are_not_an_arrangement_question() {
        let circle = vec![DevelopedCurve2D::CircularArc(DirectedCircularArc2 {
            center: Point2::new(0.0, 0.0),
            cos_basis: Vector2::new(1.0, 0.0),
            sin_basis: Vector2::new(0.0, 1.0),
            t0: 0.0,
            t1: TAU,
            provenance: provenance(0),
        })];
        let survey = survey_arrangement(&circle).expect("a full circle surveys");
        assert_eq!(survey.arcs, 1);
        assert!(survey.pieces >= 2, "a full circle is not x-monotone");
        assert_eq!(
            survey.intra_occurrence_pairs,
            survey.pieces * (survey.pieces - 1) / 2
        );
        assert_eq!(survey.certified_crossings, 0);
    }
}
