//! Certified pair intersection of x-monotone pieces.
//!
//! # What this module does
//!
//! [`intersect_x_monotone`] dispatches two [`XMonotonePiece2`] values to the
//! correct family-specific predicate (line–line, line–circle,
//! circle–circle) and returns a [`PairIntersectionResult`] whose variants
//! name every way the result can be nondeterminate.
//!
//! # Exactness policy
//!
//! Every decision that admits topology is an exact predicate over the `f64`
//! inputs, computed with the Shewchuk expansion arithmetic in
//! [`super::exact`] — never a tolerance, and never a rounded equality:
//!
//! - **line–line** uses [`super::planar_slice::classify_segments`]
//!   (`robust::orient2d`); the contact type is the exact sign of the
//!   direction cross product, so a single-point result is `Transverse`
//!   only for non-parallel tangents. Parallel single-point touches are
//!   collinear endpoint contacts (`Tangent`) and require a shared source
//!   vertex to be admitted at all.
//! - **line–circle** certifies the sign of the discriminant
//!   `D = 4(A·R² − O²)` where `A = |d|²`, `R² = |cos_basis|²` and
//!   `O = orient(start, end, center)`; every factor is an exact expansion
//!   and `O²` is an exact expansion product, so the sign decides the exact
//!   number and type of support-circle intersections. `d = end − start` is
//!   the **exact** coordinate difference (a `two_sum` expansion,
//!   [`Expansion::from_sum`]) — never a rounded `f64` vector — so the
//!   Lagrange identity `A·|w|² − (w·d)² = (d×w)²` (with `w = start − center`)
//!   sees the same exact direction in the norm, the dot and the cross.
//!   Rounded direction vectors appear only as representative/evaluation
//!   hints.
//! - **circle–circle** certifies the sign of the radical-axis discriminant
//!   `S = 2A·R1 + 2A·R2 − R1² − R2² − A² + 2R1R2` over the exact squared
//!   distance and exact squared radii (again by expansion products).
//!
//! The discriminant signs certify the *support-curve* intersection count,
//! **not** the finite-piece intersections. Each root is constructed as a
//! **certified parameter enclosure** — directed-rounding interval arithmetic
//! ([`super::exact::CertifiedInterval`]) over the exact expansions — and
//! membership on the finite pieces is decided by **exact predicates**:
//!
//! - a line piece by certified interval separation of the root parameter
//!   against `[0, 1]`, with exact endpoint identity admissions (a root at
//!   `0` or `1` is admitted only when the line's own endpoint is exactly on
//!   the support circle, decided by the exact on-circle predicate);
//! - a circular-arc piece by **certified parameter membership** (the A7
//!   repair, GEN-001E): the root's certified arc-parameter enclosure is
//!   compared against the piece's authoritative endpoint parameters (the
//!   source's declared trim values or the certified critical enclosures).
//!   No rounded evaluated endpoint coordinate decides membership.
//!
//! # Endpoint identity and the parameter-membership precondition
//!
//! An arc endpoint is admitted as the location of a root only on a
//! certificate, never from enclosure overlap alone, and never from an exact
//! predicate over a rounded `cos`/`sin` evaluation of the endpoint — a
//! rounded representative is not the semantic endpoint, so exact arithmetic
//! on it certifies neither incidence nor radical-axis side:
//!
//! 1. **Shared provenance identity.** A source-vertex endpoint shared with
//!    the other piece's endpoint is certified by the shared
//!    [`SourceVertexKey`]; the intersection coordinate is then the other
//!    piece's declared endpoint coordinate (an exact `f64` for a line), and
//!    circle incidence is an exact predicate on that declared geometry. The
//!    analogous artificial-vertex identity is the shared
//!    [`super::xmonotone::CriticalIdentity`] of a monotone split.
//! 2. **Certified attribution to an isolated root at the authoritative
//!    endpoint parameter.** The root's certified parameter enclosure on the
//!    arc must overlap the endpoint's authoritative parameter — the source's
//!    declared trim value, or the critical point's certified enclosure —
//!    and every other root's enclosure must be certified disjoint from it.
//!    This identifies *the* root at the endpoint without trusting a rounded
//!    evaluated point.
//!
//! The membership test rests on the x-monotone decomposition's guarantee: a
//! nondegenerate x-monotone circle piece covers exactly the certified
//! parameter range between its authoritative endpoints (the source's
//! unwrapped interval split at the certified x-critical enclosures). A root's
//! certified parameter enclosure is therefore compared against that range
//! ([`arc_parameter_membership`]) — Interior only when strictly between the
//! two endpoint parameters, Exterior only when certifiably outside, and
//! Boundary when it touches a piece boundary. This replaces the A7
//! chord-side orient test, which read rounded evaluated arc endpoints
//! (`arc.start.point()` / `arc.end.point()`); the conservative outcomes are
//! unchanged (a straddle goes to endpoint admission or `Undecidable`, never
//! an invented membership).
//!
//! No tolerance establishes topology. [`LocationOnPiece`] is exhaustive:
//! only a certified [`LocationOnPiece::Exterior`] is silently discarded. An
//! interior root is admitted; an identified endpoint is admitted with its
//! endpoint identity ([`IntersectionIdentity`]); anything undecidable is
//! [`PairUnresolved`]. The evaluation-seed
//! [`parameter_hint_interval`](super::xmonotone::PieceIdentity::parameter_hint_interval)
//! is never consulted.
//!
//! # The exact arithmetic is the canonical single copy
//!
//! The discriminant signs, the root enclosures and the orient coefficients
//! all flow through [`super::exact::Expansion`], which is the **one**
//! Shewchuk implementation in the workspace. `look` consumes it through the
//! `truck-meshalgo` patch and has retired its private copy, so there is no
//! second subtly different `Expansion`.
//!
//! # What this module does NOT handle
//!
//! - positive-length overlap → [`PairUnsupported::Overlap`]
//! - unrelated tangency → [`PairUnsupported::UnrelatedTangency`]
//! - coincident support circles → [`PairUnsupported::CoincidentCircles`]
//! - triple intersections (handled by the sweep, ARR-003)

use super::super::source_evidence::{EdgeUseId, SourceVertexKey};
use super::curve2d::{DirectedCircularArc2, LineSegment2};
use super::exact::{exact_dot2, exact_sq_dist, CertifiedInterval, Expansion};
use super::planar_slice::{classify_segments, SegmentIntersection};
use super::xmonotone::{ArcPieceEndpoint, XMonotoneCircularArc2, XMonotoneLine2, XMonotonePiece2};
use std::f64::consts::{PI, TAU};
use truck_geometry::prelude::{Point2, Vector2};

/// The certified-sign type of [`super::exact`], re-exported at this layer.
pub use super::exact::CertifiedSign;

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

/// Where on its piece a certified intersection lies.
///
/// The record form of a location: the exhaustive decision about where a
/// support-curve root lies is [`LocationOnPiece`]; an intersection that
/// reaches this record is always *on* the piece, so it is recorded as an
/// interior point or an identified endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterLocation {
    /// At the piece's source start endpoint.
    SourceStartEndpoint,
    /// At the piece's source end endpoint.
    SourceEndEndpoint,
    /// At an artificial monotone-split vertex.
    ArtificialPieceEndpoint,
    /// In the interior of the piece.
    PieceInterior,
}

/// How the curves meet at the intersection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactKind {
    /// The curves cross: the tangents are not parallel.
    Transverse,
    /// The curves are tangent (tangents parallel or anti-parallel).
    Tangent,
}

/// The stable identity of an intersection point for event deduplication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntersectionIdentity {
    /// A physical source vertex.
    SourceVertex(SourceVertexKey),
    /// An artificial split point from monotone decomposition.
    ArtificialMonotoneSplit {
        /// The edge use the split belongs to.
        edge_use_id: EdgeUseId,
        /// The split's critical index.
        critical_index: i64,
    },
    /// Two curves intersect.
    CurveIntersection {
        /// The identity of the first curve's occurrence.
        lhs_edge_use: EdgeUseId,
        /// The identity of the second curve's occurrence.
        rhs_edge_use: EdgeUseId,
        /// Which intersection (0-based) along the first curve's traversal.
        intersection_index: usize,
    },
}

/// Why a numerical predicate could not decide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericalCause {
    /// A computed value was not finite.
    NonFiniteComputedValue,
    /// A root's certified parameter enclosure overlaps a piece boundary (or
    /// the piece geometry degenerates), so its location cannot be certified.
    EnclosureOverlapsBoundary,
}

impl NumericalCause {
    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::NonFiniteComputedValue => "non_finite_computed_value",
            Self::EnclosureOverlapsBoundary => "enclosure_overlaps_boundary",
        }
    }
}

/// The exhaustive location of a support-curve root relative to a piece.
///
/// Every caller must account for all four states. Only
/// [`LocationOnPiece::Exterior`] may be silently discarded; an
/// [`LocationOnPiece::Undecidable`] root must propagate to `Unresolved`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocationOnPiece {
    /// Certifiably in the interior of the piece.
    Interior,
    /// At an identified semantic endpoint.
    IdentifiedEndpoint(ParameterLocation),
    /// Certifiably exterior to the piece.
    Exterior,
    /// The location cannot be certified.
    Undecidable(NumericalCause),
}

impl LocationOnPiece {
    /// Collapse to the record form. `None` for exterior or undecidable —
    /// those are never recorded.
    fn recorded(self) -> Option<ParameterLocation> {
        match self {
            LocationOnPiece::Interior => Some(ParameterLocation::PieceInterior),
            LocationOnPiece::IdentifiedEndpoint(location) => Some(location),
            LocationOnPiece::Exterior | LocationOnPiece::Undecidable(_) => None,
        }
    }
}

/// A certified enclosure of one piece's parameter at an intersection.
///
/// The enclosure conservatively bounds the exact parameter of the exact
/// intersection on that piece. It is the ordering key for the sweep
/// (ARR-003); ordering is decided by interval separation, never by the
/// rounded midpoint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParameterEnclosure {
    /// A certified lower bound.
    pub lo: f64,
    /// A certified upper bound.
    pub hi: f64,
}

impl ParameterEnclosure {
    /// The degenerate enclosure `[t, t]`.
    pub fn from_f64(t: f64) -> Self {
        ParameterEnclosure { lo: t, hi: t }
    }

    /// An enclosure from a precomputed pair.
    pub fn from_pair((lo, hi): (f64, f64)) -> Self {
        ParameterEnclosure { lo, hi }
    }

    /// Whether `t` lies within the enclosure (inclusive).
    pub fn contains(&self, t: f64) -> bool {
        self.lo <= t && t <= self.hi
    }

    /// The enclosure width `hi − lo`.
    pub fn width(&self) -> f64 {
        self.hi - self.lo
    }

    /// Whether the enclosure is a single exact point.
    pub fn is_degenerate(&self) -> bool {
        self.lo == self.hi
    }
}

/// One certified isolated pair intersection.
///
/// `point` is a representative: the analytic evaluation at the midpoint of
/// the certified parameter enclosures. It is a derivation aid for reporting
/// and visualization, never a topology decision — the certified object is
/// the pair of parameter enclosures together with the exhaustive
/// [`ParameterLocation`]s.
#[derive(Debug, Clone, PartialEq)]
pub struct CertifiedIntersection2 {
    /// A representative point (evaluation at the enclosure midpoints).
    pub point: Point2,
    /// A certified enclosure of the intersection parameter on the first
    /// piece.
    pub lhs_parameter: ParameterEnclosure,
    /// A certified enclosure of the intersection parameter on the second
    /// piece.
    pub rhs_parameter: ParameterEnclosure,
    /// Where the intersection lies on the first piece.
    pub lhs_location: ParameterLocation,
    /// Where the intersection lies on the second piece.
    pub rhs_location: ParameterLocation,
    /// How the curves meet.
    pub contact: ContactKind,
    /// The stable identity for event deduplication.
    pub identity: IntersectionIdentity,
}

// ---------------------------------------------------------------------------
// Pair intersection result
// ---------------------------------------------------------------------------

/// Why a pair intersection was refused as unsupported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairUnsupported {
    /// The two pieces share a positive-length collinear overlap.
    Overlap,
    /// The pieces touch tangentially without a certified source-declared
    /// join (the join authority is discharged in the sweep).
    UnrelatedTangency,
    /// The two circular arcs lie on the same support circle.
    CoincidentCircles,
}

impl PairUnsupported {
    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::Overlap => "pair_overlap",
            Self::UnrelatedTangency => "pair_unrelated_tangency",
            Self::CoincidentCircles => "pair_coincident_circles",
        }
    }
}

/// Why a pair intersection could not be resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairUnresolved {
    /// The exact discriminant sign is positive but the `f64` roots cannot be
    /// resolved into distinct real roots (a near-tangent).
    RootsBelowF64Resolution,
    /// A root's certified parameter enclosure overlaps a piece boundary and
    /// whether it lies on the piece cannot be certified.
    ParameterLocationUndecided,
    /// A computed value was not finite.
    NonFiniteComputedValue,
}

impl PairUnresolved {
    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::RootsBelowF64Resolution => "pair_roots_below_f64_resolution",
            Self::ParameterLocationUndecided => "pair_parameter_location_undecided",
            Self::NonFiniteComputedValue => "pair_non_finite",
        }
    }
}

/// The result of intersecting two x-monotone pieces.
#[derive(Debug, Clone)]
pub enum PairIntersectionResult {
    /// No intersection.
    Disjoint,
    /// One or more isolated certified intersections.
    Intersections(Vec<CertifiedIntersection2>),
    /// The intersection is valid geometry outside the admitted envelope.
    Unsupported(PairUnsupported),
    /// The intersection cannot be certified under the declared numerical
    /// policy.
    Unresolved(PairUnresolved),
}

impl PairIntersectionResult {
    /// A short stable tag, for probe records.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::Disjoint => "disjoint",
            Self::Intersections(_) => "intersections",
            Self::Unsupported(cause) => cause.tag(),
            Self::Unresolved(cause) => cause.tag(),
        }
    }
}

// ---------------------------------------------------------------------------
// Numerical policy
// ---------------------------------------------------------------------------

/// Declared numerical thresholds for intersection predicates.
#[derive(Debug, Clone, Copy)]
pub struct IntersectionPolicy {
    /// The maximum number of distinct intersections allowed per pair. A
    /// configured resource bound, not a geometric fact.
    pub max_intersections: usize,
}

impl IntersectionPolicy {
    /// The standard policy: a 4-intersection budget.
    pub const fn standard() -> Self {
        Self {
            max_intersections: 4,
        }
    }
}

// ---------------------------------------------------------------------------
// Dispatcher
// ---------------------------------------------------------------------------

/// Compute the certified intersections of two x-monotone pieces.
pub fn intersect_x_monotone(
    lhs: &XMonotonePiece2,
    rhs: &XMonotonePiece2,
    policy: &IntersectionPolicy,
) -> PairIntersectionResult {
    match (lhs, rhs) {
        (XMonotonePiece2::Line(l), XMonotonePiece2::Line(r)) => line_line(l, r, 0),
        (XMonotonePiece2::Line(l), XMonotonePiece2::CircularArc(r)) => {
            line_circle(l, r, 0, true, policy)
        }
        (XMonotonePiece2::CircularArc(l), XMonotonePiece2::Line(r)) => {
            line_circle(r, l, 0, false, policy)
        }
        (XMonotonePiece2::CircularArc(l), XMonotonePiece2::CircularArc(r)) => {
            circle_circle(l, r, 0, policy)
        }
    }
}

// ---------------------------------------------------------------------------
// Exact geometric expansion helpers
// ---------------------------------------------------------------------------

/// Exact `(a − b)·v` over the `f64` coordinates.
fn dot_diff_exp(a: Point2, b: Point2, v: Vector2) -> Expansion {
    let mut acc = Expansion::from_product(a.x, v.x);
    acc = acc.merge(&Expansion::from_product(b.x, v.x).negate());
    acc = acc.merge(&Expansion::from_product(a.y, v.y));
    acc = acc.merge(&Expansion::from_product(b.y, v.y).negate());
    acc
}

/// Exact `orient(a, b, c) = (b − a) × (c − a)` over the `f64` coordinates.
///
/// The `±a.x·a.y` terms cancel exactly (f64 multiplication is commutative),
/// so the six-term expansion is exact over the coordinates.
///
/// `#[cfg(test)]`: the production membership path (A7, GEN-001E) decides arc
/// membership by certified parameter evidence, so this exact orient helper
/// survives only as a test oracle.
#[cfg(test)]
fn orient_exp(a: Point2, b: Point2, c: Point2) -> Expansion {
    let mut acc = Expansion::from_product(b.x, c.y);
    acc = acc.merge(&Expansion::from_product(b.x, a.y).negate());
    acc = acc.merge(&Expansion::from_product(a.x, c.y).negate());
    acc = acc.merge(&Expansion::from_product(b.y, c.x).negate());
    acc = acc.merge(&Expansion::from_product(b.y, a.x));
    acc = acc.merge(&Expansion::from_product(a.y, c.x));
    acc
}

/// The exact coordinate difference of two points, as `(dx, dy)` expansions.
///
/// Each `a − b` is split exactly by `two_sum` ([`Expansion::from_sum`]); a
/// rounded `f64` difference vector is never formed. Every certified quantity
/// in this module — squared norms, dot products, cross products, root
/// intervals, membership predicates — is built from these expansions, so that
/// the exact direction vector of a support line or center axis is the same in
/// every term. Rounded `f64` vectors appear only as representative/evaluation
/// hints.
fn point_diff_exp(a: Point2, b: Point2) -> (Expansion, Expansion) {
    (
        Expansion::from_sum(a.x, -b.x),
        Expansion::from_sum(a.y, -b.y),
    )
}

/// Exact `u × v = ux·vy − uy·vx` over two exact coordinate-difference pairs.
fn cross_exp2(ux: &Expansion, uy: &Expansion, vx: &Expansion, vy: &Expansion) -> Expansion {
    ux.mul_expansion(vy).merge(&uy.mul_expansion(vx).negate())
}

/// Exact `u · v = ux·vx + uy·vy` over two exact coordinate-difference pairs.
fn dot_exp2(ux: &Expansion, uy: &Expansion, vx: &Expansion, vy: &Expansion) -> Expansion {
    ux.mul_expansion(vx).merge(&uy.mul_expansion(vy))
}

/// Exact `d · v` where `d` is an exact coordinate-difference pair and `v` is a
/// fixed `f64` vector (a basis vector of a certified circle).
fn dot_vec_exp(dx: &Expansion, dy: &Expansion, vx: f64, vy: f64) -> Expansion {
    dx.mul_expansion(&Expansion::zero().grow(vx))
        .merge(&dy.mul_expansion(&Expansion::zero().grow(vy)))
}

/// Exact `(−dy, dx) · v = −dy·vx + dx·vy`: the dot of the 90°-rotated exact
/// difference with a fixed `f64` vector.
fn rot_dot_exp(dx: &Expansion, dy: &Expansion, vx: f64, vy: f64) -> Expansion {
    dy.mul_expansion(&Expansion::zero().grow(-vx))
        .merge(&dx.mul_expansion(&Expansion::zero().grow(vy)))
}

// ---------------------------------------------------------------------------
// Line–line intersection (exact orientation predicates)
// ---------------------------------------------------------------------------

/// The contact kind of a single-point line–line result, from the exact sign
/// of the direction cross product over the exact coordinate differences.
///
/// A single-point result is `Transverse` only when the support lines are not
/// parallel. A parallel single-point touch is a *collinear endpoint contact*
/// (`Tangent`): the two segments share exactly one endpoint on one line, and
/// their tangents coincide. Issue 4 of ARR-002: a one-point result is never
/// `Transverse` by default; the location test (interior vs. shared endpoint
/// vs. endpoint-on-interior) and this contact kind together name what it is.
fn line_line_contact(lhs: &XMonotoneLine2, rhs: &XMonotoneLine2) -> ContactKind {
    let (lx, ly) = point_diff_exp(lhs.source.end, lhs.source.start);
    let (rx, ry) = point_diff_exp(rhs.source.end, rhs.source.start);
    match cross_exp2(&lx, &ly, &rx, &ry).sign() {
        CertifiedSign::Zero => ContactKind::Tangent,
        _ => ContactKind::Transverse,
    }
}

/// Whether two source endpoints are the same physical vertex, by provenance.
fn shared_source_vertex(
    lhs_location: ParameterLocation,
    lhs_start: SourceVertexKey,
    lhs_end: SourceVertexKey,
    rhs_location: ParameterLocation,
    rhs_start: SourceVertexKey,
    rhs_end: SourceVertexKey,
) -> bool {
    let lhs_vertex = endpoint_vertex(lhs_location, lhs_start, lhs_end);
    let rhs_vertex = endpoint_vertex(rhs_location, rhs_start, rhs_end);
    match (lhs_vertex, rhs_vertex) {
        (Some(l), Some(r)) => l.is_identified() && l == r,
        _ => false,
    }
}

/// The vertex id at a piece location, if the location is a source endpoint.
fn endpoint_vertex(
    location: ParameterLocation,
    start: SourceVertexKey,
    end: SourceVertexKey,
) -> Option<SourceVertexKey> {
    match location {
        ParameterLocation::SourceStartEndpoint => Some(start),
        ParameterLocation::SourceEndEndpoint => Some(end),
        _ => None,
    }
}

fn line_line(
    lhs: &XMonotoneLine2,
    rhs: &XMonotoneLine2,
    intersection_index: usize,
) -> PairIntersectionResult {
    match classify_segments(
        lhs.source.start,
        lhs.source.end,
        rhs.source.start,
        rhs.source.end,
    ) {
        SegmentIntersection::Empty => PairIntersectionResult::Disjoint,
        SegmentIntersection::Overlap => {
            PairIntersectionResult::Unsupported(PairUnsupported::Overlap)
        }
        SegmentIntersection::Point(point) => {
            let lhs_loc = line_parameter_location(&lhs.source, point);
            let rhs_loc = line_parameter_location(&rhs.source, point);
            let contact = line_line_contact(lhs, rhs);
            if contact == ContactKind::Tangent
                && !shared_source_vertex(
                    lhs_loc,
                    lhs.source.provenance.start_vertex_id,
                    lhs.source.provenance.end_vertex_id,
                    rhs_loc,
                    rhs.source.provenance.start_vertex_id,
                    rhs.source.provenance.end_vertex_id,
                )
            {
                // A collinear endpoint touch that is not a source-declared
                // join: certified as tangency, but with no join authority it
                // is unrelated tangency, exactly as for the curve families.
                return PairIntersectionResult::Unsupported(PairUnsupported::UnrelatedTangency);
            }
            let (lhs_enc, rhs_enc) =
                match line_line_parameter_enclosures(lhs, rhs, lhs_loc, rhs_loc, point) {
                    Some(encs) => encs,
                    None => {
                        return PairIntersectionResult::Unresolved(
                            PairUnresolved::ParameterLocationUndecided,
                        )
                    }
                };
            let lhs_hint = line_endpoint_hint(&lhs.source, lhs_loc);
            let rhs_hint = line_endpoint_hint(&rhs.source, rhs_loc);
            let lhs_eu = lhs.source.provenance.edge_use_id;
            let rhs_eu = rhs.source.provenance.edge_use_id;
            PairIntersectionResult::Intersections(vec![build_record(
                lhs_enc,
                rhs_enc,
                lhs_loc,
                rhs_loc,
                lhs_hint,
                rhs_hint,
                lhs_eu,
                rhs_eu,
                point,
                contact,
                intersection_index,
            )])
        }
    }
}

/// The parameter location of a point already proved (by an exact
/// orientation predicate) to lie on the segment.
fn line_parameter_location(segment: &LineSegment2, point: Point2) -> ParameterLocation {
    if point == segment.start {
        ParameterLocation::SourceStartEndpoint
    } else if point == segment.end {
        ParameterLocation::SourceEndEndpoint
    } else {
        ParameterLocation::PieceInterior
    }
}

/// Certified enclosures of both line parameters at a single-point line–line
/// intersection, over the exact coordinate differences of both segments.
///
/// `point` is the intersection point [`classify_segments`](super::planar_slice::classify_segments)
/// returned: for an endpoint contact it is an exact declared endpoint
/// coordinate (on both support lines exactly), for a proper crossing it is a
/// rounded representative.
///
/// - **Both endpoints:** the parameters are exactly `0`/`1` (declared
///   endpoint coordinates, decided by exact `f64` equality).
/// - **Endpoint-on-interior:** `point` is the exact shared endpoint, so the
///   interior segment's parameter is [`certified_line_parameter`] of that
///   exact point. This stays certified when the supports are collinear
///   (`d1×d2 = 0`), where the cross-ratio denominator would vanish.
/// - **Proper crossing (both interior):** the cross-product identities
///   `s = (a2−a1)×d2 / (d1×d2)` and `t = (a2−a1)×d1 / (d1×d2)` over exact
///   expansions, widened by directed rounding — never a rounded ratio of the
///   rounded `point`.
///
/// Returns `None` when an interior parameter cannot be certified, so the
/// caller produces `Unresolved` rather than an uncertified ordering key.
fn line_line_parameter_enclosures(
    lhs: &XMonotoneLine2,
    rhs: &XMonotoneLine2,
    lhs_loc: ParameterLocation,
    rhs_loc: ParameterLocation,
    point: Point2,
) -> Option<(ParameterEnclosure, ParameterEnclosure)> {
    let exact_endpoint = |loc: ParameterLocation| match loc {
        ParameterLocation::SourceStartEndpoint => Some(ParameterEnclosure::from_f64(0.0)),
        ParameterLocation::SourceEndEndpoint => Some(ParameterEnclosure::from_f64(1.0)),
        _ => None,
    };
    let lhs_exact = exact_endpoint(lhs_loc);
    let rhs_exact = exact_endpoint(rhs_loc);
    match (lhs_exact, rhs_exact) {
        (Some(l), Some(r)) => Some((l, r)),
        (Some(l), None) => {
            // The shared point is exactly the other segment's endpoint; the
            // interior parameter is certified over that exact point. Valid
            // whether or not the supports are collinear.
            Some((l, certified_line_parameter(&rhs.source, &point)?))
        }
        (None, Some(r)) => Some((certified_line_parameter(&lhs.source, &point)?, r)),
        (None, None) => {
            let a1 = lhs.source.start;
            let b1 = lhs.source.end;
            let a2 = rhs.source.start;
            let b2 = rhs.source.end;
            let (d1x, d1y) = point_diff_exp(b1, a1);
            let (d2x, d2y) = point_diff_exp(b2, a2);
            let (wx, wy) = point_diff_exp(a2, a1);
            let den = cross_exp2(&d1x, &d1y, &d2x, &d2y);
            let den_iv = CertifiedInterval::from_expansion(&den);
            let s_num_iv = CertifiedInterval::from_expansion(&cross_exp2(&wx, &wy, &d2x, &d2y));
            let t_num_iv = CertifiedInterval::from_expansion(&cross_exp2(&wx, &wy, &d1x, &d1y));
            let s_iv = s_num_iv.div(&den_iv)?;
            let t_iv = t_num_iv.div(&den_iv)?;
            if !(s_iv.is_finite() && t_iv.is_finite()) {
                return None;
            }
            let s = ParameterEnclosure {
                lo: s_iv.lo,
                hi: s_iv.hi,
            };
            let t = ParameterEnclosure {
                lo: t_iv.lo,
                hi: t_iv.hi,
            };
            Some((s, t))
        }
    }
}

/// The endpoint identity of a line-piece location, if it is an endpoint.
fn line_endpoint_hint(
    segment: &LineSegment2,
    loc: ParameterLocation,
) -> Option<IntersectionIdentity> {
    match loc {
        ParameterLocation::SourceStartEndpoint => Some(IntersectionIdentity::SourceVertex(
            segment.provenance.start_vertex_id,
        )),
        ParameterLocation::SourceEndEndpoint => Some(IntersectionIdentity::SourceVertex(
            segment.provenance.end_vertex_id,
        )),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Piece locations and record assembly
// ---------------------------------------------------------------------------

/// The location of one support-curve root on one piece, with everything
/// needed to build the record when the root is on the piece.
struct PieceLocation {
    location: LocationOnPiece,
    /// Meaningful when `location` is `Interior` or `IdentifiedEndpoint`.
    parameter: ParameterEnclosure,
    /// The endpoint identity, when `location` is `IdentifiedEndpoint`.
    identity_hint: Option<IntersectionIdentity>,
}

impl PieceLocation {
    fn exterior() -> Self {
        PieceLocation {
            location: LocationOnPiece::Exterior,
            parameter: ParameterEnclosure::from_f64(0.0),
            identity_hint: None,
        }
    }

    fn undecided(cause: NumericalCause) -> Self {
        PieceLocation {
            location: LocationOnPiece::Undecidable(cause),
            parameter: ParameterEnclosure::from_f64(0.0),
            identity_hint: None,
        }
    }

    fn interior(parameter: ParameterEnclosure) -> Self {
        PieceLocation {
            location: LocationOnPiece::Interior,
            parameter,
            identity_hint: None,
        }
    }
}

/// A certified admission of a root at a semantic endpoint of a piece.
struct EndpointAdmission {
    location: ParameterLocation,
    identity: IntersectionIdentity,
    parameter: ParameterEnclosure,
}

impl EndpointAdmission {
    fn into_piece_location(self) -> PieceLocation {
        PieceLocation {
            location: LocationOnPiece::IdentifiedEndpoint(self.location),
            parameter: self.parameter,
            identity_hint: Some(self.identity),
        }
    }
}

/// The per-root outcome of a piece pair.
enum RootOutcome {
    /// The root is certifiably exterior to at least one piece — silently
    /// discarded.
    Skip,
    /// The root is a certified finite-piece intersection.
    Record(CertifiedIntersection2),
    /// The contact is tangent but not at a certified source-declared join.
    UnrelatedTangency,
}

/// Which endpoint of a piece is being tested.
#[derive(Debug, Clone, Copy)]
enum EndpointRole {
    Start,
    End,
}

/// Assemble one certified intersection record.
#[allow(clippy::too_many_arguments)]
fn build_record(
    lhs_parameter: ParameterEnclosure,
    rhs_parameter: ParameterEnclosure,
    lhs_location: ParameterLocation,
    rhs_location: ParameterLocation,
    lhs_identity_hint: Option<IntersectionIdentity>,
    rhs_identity_hint: Option<IntersectionIdentity>,
    lhs_edge_use: EdgeUseId,
    rhs_edge_use: EdgeUseId,
    point: Point2,
    contact: ContactKind,
    index: usize,
) -> CertifiedIntersection2 {
    CertifiedIntersection2 {
        point,
        lhs_parameter,
        rhs_parameter,
        lhs_location,
        rhs_location,
        contact,
        identity: pair_identity(
            lhs_identity_hint,
            rhs_identity_hint,
            lhs_edge_use,
            rhs_edge_use,
            index,
        ),
    }
}

/// The deduplication identity of a pair intersection.
///
/// A shared source vertex is the strongest identity; then any single
/// endpoint identity; then the plain curve-pair intersection.
fn pair_identity(
    lhs_hint: Option<IntersectionIdentity>,
    rhs_hint: Option<IntersectionIdentity>,
    lhs_edge_use: EdgeUseId,
    rhs_edge_use: EdgeUseId,
    index: usize,
) -> IntersectionIdentity {
    match (lhs_hint, rhs_hint) {
        (
            Some(IntersectionIdentity::SourceVertex(l)),
            Some(IntersectionIdentity::SourceVertex(r)),
        ) if l == r => IntersectionIdentity::SourceVertex(l),
        (Some(h), _) => h,
        (None, Some(h)) => h,
        (None, None) => IntersectionIdentity::CurveIntersection {
            lhs_edge_use,
            rhs_edge_use,
            intersection_index: index,
        },
    }
}

/// Whether a tangent line–arc contact is a source-declared join: the contact
/// is at both pieces' source endpoints and the two relevant vertex
/// identities match.
fn line_arc_source_join(
    line: &XMonotoneLine2,
    arc: &XMonotoneCircularArc2,
    line_loc: ParameterLocation,
    arc_loc: ParameterLocation,
) -> bool {
    use ParameterLocation::*;
    let line_vertex = match line_loc {
        SourceStartEndpoint => line.source.provenance.start_vertex_id,
        SourceEndEndpoint => line.source.provenance.end_vertex_id,
        _ => return false,
    };
    let arc_vertex = match arc_loc {
        SourceStartEndpoint => arc.source.provenance.start_vertex_id,
        SourceEndEndpoint => arc.source.provenance.end_vertex_id,
        _ => return false,
    };
    line_vertex.is_identified() && line_vertex == arc_vertex
}

/// Whether a tangent arc–arc contact is a source-declared join.
fn arc_arc_source_join(
    lhs: &XMonotoneCircularArc2,
    rhs: &XMonotoneCircularArc2,
    lhs_loc: ParameterLocation,
    rhs_loc: ParameterLocation,
) -> bool {
    use ParameterLocation::*;
    let lhs_vertex = match lhs_loc {
        SourceStartEndpoint => lhs.source.provenance.start_vertex_id,
        SourceEndEndpoint => lhs.source.provenance.end_vertex_id,
        _ => return false,
    };
    let rhs_vertex = match rhs_loc {
        SourceStartEndpoint => rhs.source.provenance.start_vertex_id,
        SourceEndEndpoint => rhs.source.provenance.end_vertex_id,
        _ => return false,
    };
    lhs_vertex.is_identified() && lhs_vertex == rhs_vertex
}

// ---------------------------------------------------------------------------
// Exact arc membership: the certified parameter test (A7, GEN-001E)
// ---------------------------------------------------------------------------

/// The three decisive outcomes of the membership test over an enclosure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OrientLocation {
    Interior,
    Exterior,
    Boundary,
}

// ---------------------------------------------------------------------------
// Arc endpoint identity
// ---------------------------------------------------------------------------

/// The exact squared radius `|cos_basis|²` of a support circle.
fn radius_squared_exp(arc: &DirectedCircularArc2) -> Expansion {
    let cb = arc.cos_basis;
    exact_dot2([cb.x, cb.y], [cb.x, cb.y])
}

/// Whether a point is exactly on a support circle, as an exact predicate
/// over the `f64` coordinates: `|P − C|² == |cos_basis|²` exactly.
///
/// Both sides are exact expansions ([`exact_sq_dist`] and [`exact_dot2`]),
/// so the equality is decided by the exact sign of their difference — never
/// by `distance_squared == radius_squared` in rounded `f64`.
fn point_is_on_circle_exact(point: &Point2, arc: &DirectedCircularArc2) -> bool {
    exact_sq_dist([point.x, point.y], [arc.center.x, arc.center.y])
        .merge(&radius_squared_exp(arc).negate())
        .sign()
        == CertifiedSign::Zero
}

/// Whether this root's enclosure is certified separated from the other root's
/// enclosure (or there is no other root).
///
/// Enclosure overlap is only *not separated*, never equality. When both roots'
/// boxes overlap a piece boundary, neither can be identified as *the* root at
/// that boundary — the caller must produce `Undecidable`.
fn separated_from_other_root(
    s_iv: &ParameterEnclosure,
    other_root: Option<&ParameterEnclosure>,
) -> bool {
    match other_root {
        None => true,
        Some(other) => s_iv.hi < other.lo || s_iv.lo > other.hi,
    }
}

/// A certified enclosure of the exact line parameter of a point certified to
/// lie on the exact support line (`orient == 0` exactly).
///
/// The parameter is the ratio of exact coordinate differences — numerator
/// from the point, denominator from the line's declared direction — each
/// expanded exactly by `two_sum` and divided with directed rounding. A rounded
/// `line_parameter` result is never a certificate. `None` for a bitwise
/// degenerate segment (no axis has a nonzero exact difference), which the
/// callers have already refused.
fn certified_line_parameter(line: &LineSegment2, point: &Point2) -> Option<ParameterEnclosure> {
    let p = line.start;
    let q = line.end;
    // Pick an axis with an exactly nonzero direction component; the point is
    // on the line, so the ratio is the same on either axis.
    let (num, den) = if q.x != p.x {
        (
            Expansion::from_sum(point.x, -p.x),
            Expansion::from_sum(q.x, -p.x),
        )
    } else if q.y != p.y {
        (
            Expansion::from_sum(point.y, -p.y),
            Expansion::from_sum(q.y, -p.y),
        )
    } else {
        return None;
    };
    let num_iv = CertifiedInterval::from_expansion(&num);
    let den_iv = CertifiedInterval::from_expansion(&den);
    let s = num_iv.div(&den_iv)?;
    if !s.is_finite() {
        return None;
    }
    Some(ParameterEnclosure { lo: s.lo, hi: s.hi })
}

/// Whether the endpoint at `point` (an exact `f64` coordinate, certified on
/// the line by the caller) uniquely attributes the support root with
/// enclosure `s_iv`, given the other root's enclosure.
///
/// Two conditions, both over certified enclosures: (1) the endpoint's exact
/// line parameter overlaps **this** root's enclosure, and (2) the endpoint's
/// exact line parameter is certified disjoint from every other root's
/// enclosure. Root-box separation alone does not identify the root — the
/// endpoint's parameter enclosure may overlap both separated root boxes while
/// the exact parameter lies in the other one. The endpoint being exactly on
/// the circle makes its line parameter an exact root value, so attribution
/// identifies the root exactly. Anything unproven returns `false`, and the
/// caller produces `Undecidable`.
fn attributed_to_this_line_root(
    line: &LineSegment2,
    point: &Point2,
    s_iv: &ParameterEnclosure,
    other_root: Option<&ParameterEnclosure>,
) -> bool {
    let s_p = match certified_line_parameter(line, point) {
        Some(s_p) => s_p,
        None => return false,
    };
    // The endpoint's exact parameter must overlap this root's enclosure.
    if s_p.hi < s_iv.lo || s_p.lo > s_iv.hi {
        return false;
    }
    // ... and be certified disjoint from every other root's enclosure.
    match other_root {
        None => true,
        Some(other) => s_p.hi < other.lo || s_p.lo > other.hi,
    }
}

/// The admission of an arc endpoint as a line–circle support-curve
/// intersection, if the endpoint is certifiably the location of the root
/// under consideration.
///
/// The certificate is never enclosure overlap alone, and never an exact
/// predicate over a rounded `cos`/`sin` evaluation of the endpoint — a
/// rounded representative is not the semantic endpoint. In order of
/// preference:
///
/// 1. **Shared source-vertex identity.** A source-vertex endpoint shared
///    with the line's own endpoint is certified by the matching vertex id;
///    the intersection coordinate is then the line's declared endpoint
///    coordinate (an exact `f64`), circle incidence is an exact predicate
///    on that declared geometry, and the root is attributed to it.
/// 2. **Certified attribution to an isolated root at the authoritative
///    endpoint parameter.** The root's certified arc-parameter enclosure
///    must overlap the endpoint's authoritative parameter (the source's
///    declared trim value, or the critical point's certified enclosure),
///    and the other root's arc-parameter enclosure must be certified
///    disjoint from it. This identifies *this* root as the one at the
///    endpoint without trusting a rounded evaluated point.
///
/// Anything that cannot be certified returns `None`, and the caller
/// produces `Undecidable`.
#[allow(clippy::too_many_arguments)]
fn arc_endpoint_identity_for_line(
    arc: &XMonotoneCircularArc2,
    role: EndpointRole,
    endpoint: &ArcPieceEndpoint,
    line: &XMonotoneLine2,
    dx: &Expansion,
    dy: &Expansion,
    s_iv: &ParameterEnclosure,
    other_root: Option<&ParameterEnclosure>,
) -> Option<EndpointAdmission> {
    let seg = &line.source;

    // Route 1: shared source-vertex identity. The shared vertex's coordinate
    // is the line's declared endpoint coordinate (an exact `f64`).
    if let ArcPieceEndpoint::SourceVertex { vertex_id, .. } = endpoint {
        let shared = if *vertex_id == seg.provenance.start_vertex_id {
            Some(seg.start)
        } else if *vertex_id == seg.provenance.end_vertex_id {
            Some(seg.end)
        } else {
            None
        };
        if let Some(shared_point) = shared {
            if point_is_on_circle_exact(&shared_point, &arc.source)
                && attributed_to_this_line_root(seg, &shared_point, s_iv, other_root)
            {
                return Some(admission_from_endpoint(arc, role, endpoint));
            }
        }
    }

    // Route 2: certified attribution to an isolated root at the authoritative
    // endpoint parameter. The endpoint's rounded evaluated point is never
    // consulted; the root's certified arc parameter decides.
    let t_end = arc_endpoint_parameter(arc, role, endpoint);
    let t_root = match arc_parameter_for_line(arc, seg.start, dx, dy, s_iv) {
        Ok(ArcParameterLift::OnPiece(t)) => t,
        // Unresolved lift or certifiably off the piece: no endpoint admission.
        _ => return None,
    };
    let t_other =
        other_root.and_then(
            |o| match arc_parameter_for_line(arc, seg.start, dx, dy, o) {
                Ok(ArcParameterLift::OnPiece(t)) => Some(t),
                _ => None,
            },
        );
    if attributed_to_this_arc_root(&t_root, t_other.as_ref(), &t_end) {
        Some(admission_from_endpoint(arc, role, endpoint))
    } else {
        None
    }
}

/// The admission of an arc endpoint as a circle–circle support-curve
/// intersection, if the endpoint is certifiably the location of the root
/// under consideration.
///
/// The certificate is never enclosure overlap alone, and never an exact
/// predicate over a rounded `cos`/`sin` evaluation of the endpoint — a
/// rounded representative is not the semantic endpoint, so an evaluated
/// point's radical-axis side cannot certify side incidence. The certificate
/// is **attribution of an isolated root at the authoritative endpoint
/// parameter**: this root's certified arc-parameter enclosure must overlap
/// the endpoint's authoritative parameter (the source's declared trim value,
/// or the critical point's certified enclosure), and the other root's
/// arc-parameter enclosure must be certified disjoint from it. The root's own
/// certified `side` branch carries the radical-axis placement; when the
/// endpoint is a shared source vertex or shared artificial split, the two
/// pieces' records combine to the shared identity at record assembly.
///
/// Anything that cannot be certified returns `None`, and the caller
/// produces `Undecidable`.
#[allow(clippy::too_many_arguments)]
fn arc_endpoint_identity_for_circle(
    arc: &XMonotoneCircularArc2,
    role: EndpointRole,
    endpoint: &ArcPieceEndpoint,
    c1: Point2,
    dcx: &Expansion,
    dcy: &Expansion,
    dist_iv: &CertifiedInterval,
    a_iv: &CertifiedInterval,
    h_iv: &CertifiedInterval,
    side: i64,
) -> Option<EndpointAdmission> {
    let t_end = arc_endpoint_parameter(arc, role, endpoint);
    // The other root of the pair (the tangent double root has no other).
    let t_other = if side == 0 {
        None
    } else {
        match arc_parameter_for_circle(arc, c1, dcx, dcy, dist_iv, a_iv, h_iv, -side) {
            Ok(ArcParameterLift::OnPiece(t)) => Some(t),
            _ => None,
        }
    };
    let t_root = match arc_parameter_for_circle(arc, c1, dcx, dcy, dist_iv, a_iv, h_iv, side) {
        Ok(ArcParameterLift::OnPiece(t)) => t,
        _ => return None,
    };
    if !attributed_to_this_arc_root(&t_root, t_other.as_ref(), &t_end) {
        return None;
    }
    Some(admission_from_endpoint(arc, role, endpoint))
}

/// Build the admission record for an endpoint that has been certified as the
/// root's location.
fn admission_from_endpoint(
    arc: &XMonotoneCircularArc2,
    role: EndpointRole,
    endpoint: &ArcPieceEndpoint,
) -> EndpointAdmission {
    match endpoint {
        ArcPieceEndpoint::SourceVertex { vertex_id, .. } => {
            let (location, parameter) = match role {
                EndpointRole::Start => (ParameterLocation::SourceStartEndpoint, arc.source.t0),
                EndpointRole::End => (ParameterLocation::SourceEndEndpoint, arc.source.t1),
            };
            EndpointAdmission {
                location,
                identity: IntersectionIdentity::SourceVertex(*vertex_id),
                parameter: ParameterEnclosure::from_f64(parameter),
            }
        }
        ArcPieceEndpoint::Critical(c) => EndpointAdmission {
            location: ParameterLocation::ArtificialPieceEndpoint,
            identity: IntersectionIdentity::ArtificialMonotoneSplit {
                edge_use_id: c.identity.edge_use_id,
                critical_index: c.identity.critical_index,
            },
            parameter: ParameterEnclosure::from_pair(c.parameter_enclosure),
        },
    }
}

/// The **authoritative** parameter enclosure of an arc piece endpoint: the
/// source's declared trim value for a source-vertex endpoint, or the
/// certified critical enclosure for an artificial split endpoint.
///
/// This is the semantic endpoint parameter — never a rounded `cos`/`sin`
/// evaluation of `endpoint.point()`, and never the
/// `parameter_hint_interval` evaluation seed.
fn arc_endpoint_parameter(
    arc: &XMonotoneCircularArc2,
    role: EndpointRole,
    endpoint: &ArcPieceEndpoint,
) -> ParameterEnclosure {
    match endpoint {
        ArcPieceEndpoint::SourceVertex { .. } => {
            let t = match role {
                EndpointRole::Start => arc.source.t0,
                EndpointRole::End => arc.source.t1,
            };
            ParameterEnclosure::from_f64(t)
        }
        ArcPieceEndpoint::Critical(c) => ParameterEnclosure::from_pair(c.parameter_enclosure),
    }
}

/// Whether the root with certified arc parameter `t_root` — with every other
/// root at `t_other`, when there is one — is certified to be the root at the
/// endpoint whose authoritative parameter is `t_end`.
///
/// Two conditions, both over certified enclosures: this root's parameter
/// overlaps the endpoint's authoritative parameter, and every other root's
/// parameter is certified disjoint from it. An endpoint parameter inside both
/// root boxes is never attributed to either; the caller produces
/// `Undecidable`.
fn attributed_to_this_arc_root(
    t_root: &ParameterEnclosure,
    t_other: Option<&ParameterEnclosure>,
    t_end: &ParameterEnclosure,
) -> bool {
    if t_root.hi < t_end.lo || t_root.lo > t_end.hi {
        return false;
    }
    match t_other {
        None => true,
        Some(other) => other.hi < t_end.lo || other.lo > t_end.hi,
    }
}

// ---------------------------------------------------------------------------
// Arc parameter enclosures
// ---------------------------------------------------------------------------

/// The certified parameter span of an x-monotone arc piece, on the source's
/// authoritative unwrapped axis.
///
/// The hull of the two authoritative endpoint parameters ([`arc_endpoint_parameter`]:
/// the source's declared trim values or the certified critical enclosures).
/// This is a *superset* of the piece's true parameter range and is the same
/// interval the membership test compares a lifted root against, so a lift
/// selected against it is on the same axis as the piece endpoints.
fn arc_piece_span(arc: &XMonotoneCircularArc2) -> (f64, f64) {
    let start = arc_endpoint_parameter(arc, EndpointRole::Start, &arc.start);
    let end = arc_endpoint_parameter(arc, EndpointRole::End, &arc.end);
    (start.lo.min(end.lo), start.hi.max(end.hi))
}

/// The certified angular range of a box `(u, v) = (cos t, sin t)`, in a
/// common unwrapped copy.
///
/// The angle extrema of a rectangle (not containing the origin) are attained
/// at its corners; widening each corner's correctly-rounded `atan2` by one
/// ulp makes the enclosure sound. A box subtending `≥ π` from the origin is
/// rejected (`None`).
fn interval_atan2(v: CertifiedInterval, u: CertifiedInterval) -> Option<(f64, f64)> {
    if !(u.is_finite() && v.is_finite()) {
        return None;
    }
    let corners = [[u.lo, v.lo], [u.hi, v.lo], [u.lo, v.hi], [u.hi, v.hi]];
    let mut angles: Vec<f64> = corners.iter().map(|c| c[1].atan2(c[0])).collect();
    let ref0 = angles[0];
    for a in angles.iter_mut().skip(1) {
        let mut d = *a - ref0;
        while d > PI {
            d -= TAU;
        }
        while d < -PI {
            d += TAU;
        }
        *a = ref0 + d;
    }
    let lo = angles.iter().cloned().fold(f64::INFINITY, f64::min);
    let hi = angles.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    if hi - lo >= PI {
        return None;
    }
    Some((lo.next_down(), hi.next_up()))
}

/// The certified lift of a root's angle box onto the piece's unwrapped axis.
///
/// Either the root is certifiably off the piece, or its parameter is certified
/// on exactly one `2π` copy of the piece's authoritative axis.
#[derive(Debug, Clone, PartialEq)]
enum ArcParameterLift {
    /// Certifiably off the piece: no `2π` copy of the principal angle
    /// enclosure intersects the piece's certified parameter span.
    OffPiece,
    /// The uniquely-certified lifted parameter enclosure on the piece's axis.
    OnPiece(ParameterEnclosure),
}

/// The maximum number of `2π`-copy candidates inspected before a lift is
/// declared indeterminate. An x-monotone piece has span `< π`, so a sound
/// candidate range holds at most two integers; the padded enumeration below
/// is always small.
const MAX_LIFT_CANDIDATES: i64 = 8;

/// Lift a root's certified principal angle enclosure `Θ = [a_lo, a_hi]`
/// (mod `2π`, from [`interval_atan2`]) onto the piece's authoritative
/// unwrapped axis.
///
/// **Certified branch selection (GEN-001E, A7 closure).** The previous lift
/// picked the `2π` copy nearest the *midpoint* of a reference window — a
/// representative round that could select a copy off the piece and, once that
/// lifted parameter fed the membership decision, could create topology. The
/// selection here is an **enumeration of every integer `k` such that
/// `Θ + 2πk` may intersect the piece's certified parameter span `[lo, hi]`**
/// ([`arc_piece_span`], the same axis as the piece endpoints):
///
/// ```text
/// zero compatible copies    -> OffPiece   (certifiably exterior)
/// exactly one compatible k  -> OnPiece(Θ + 2πk)   (uniquely certified lift)
/// two or more compatible k  -> Unresolved (ambiguous copy)
/// non-finite / unbounded    -> Unresolved (indeterminate arithmetic)
/// ```
///
/// A midpoint may seed nothing here: the final lift is chosen only by the
/// certified compatibility count. Soundness: the true parameter of a root on
/// the piece satisfies `a_true + 2π·k_true ∈ [L, H] ⊆ [lo, hi]` with
/// `a_true ∈ Θ`, so `k_true` is among the enumerated candidates and is the
/// only compatible copy whenever the root's copy is unique.
fn lift_arc_parameter(
    arc: &XMonotoneCircularArc2,
    u: CertifiedInterval,
    v: CertifiedInterval,
) -> Result<ArcParameterLift, PairUnresolved> {
    let (a_lo, a_hi) = interval_atan2(v, u).ok_or(PairUnresolved::ParameterLocationUndecided)?;
    if !a_lo.is_finite() || !a_hi.is_finite() {
        return Err(PairUnresolved::NonFiniteComputedValue);
    }
    let (lo, hi) = arc_piece_span(arc);
    // Padded candidate range: rounding in the f64 bounds must never drop a
    // compatible copy, so pad by one integer on each side and test each one.
    let k_min = ((lo - a_hi) / TAU).floor() as i64 - 1;
    let k_max = ((hi - a_lo) / TAU).ceil() as i64 + 1;
    if k_max - k_min > MAX_LIFT_CANDIDATES {
        return Err(PairUnresolved::ParameterLocationUndecided);
    }
    let mut compatible: Vec<i64> = Vec::new();
    for k in k_min..=k_max {
        let shift = TAU * k as f64;
        let shift_lo = (a_lo + shift).next_down();
        let shift_hi = (a_hi + shift).next_up();
        if !shift_lo.is_finite() || !shift_hi.is_finite() {
            return Err(PairUnresolved::NonFiniteComputedValue);
        }
        if shift_lo <= hi && lo <= shift_hi {
            compatible.push(k);
        }
    }
    match compatible.len() {
        0 => Ok(ArcParameterLift::OffPiece),
        1 => {
            let k = compatible[0];
            let shift = TAU * k as f64;
            let lift_lo = (a_lo + shift).next_down();
            let lift_hi = (a_hi + shift).next_up();
            if !lift_lo.is_finite() || !lift_hi.is_finite() {
                return Err(PairUnresolved::NonFiniteComputedValue);
            }
            Ok(ArcParameterLift::OnPiece(ParameterEnclosure {
                lo: lift_lo,
                hi: lift_hi,
            }))
        }
        _ => Err(PairUnresolved::ParameterLocationUndecided),
    }
}

/// The certified lift of a line–circle root onto the arc piece's unwrapped
/// axis, via the certified `(cos t, sin t)` intervals over the exact line
/// direction.
///
/// The returned [`ArcParameterLift`] is on the same authoritative unwrapped
/// axis as the piece endpoints: the `2π` copy is selected by the certified
/// enumeration in [`lift_arc_parameter`], never by a midpoint round.
#[allow(clippy::too_many_arguments)]
fn arc_parameter_for_line(
    arc: &XMonotoneCircularArc2,
    p: Point2,
    dx: &Expansion,
    dy: &Expansion,
    s_iv: &ParameterEnclosure,
) -> Result<ArcParameterLift, PairUnresolved> {
    let cb = arc.source.cos_basis;
    let sb = arc.source.sin_basis;
    // The denominator enclosure comes from the exact squared radius, never
    // from the rounded `radius_squared()` scalar.
    let r2_exp = radius_squared_exp(&arc.source);
    if r2_exp.sign() == CertifiedSign::Zero {
        return Err(PairUnresolved::ParameterLocationUndecided);
    }
    let r_iv = CertifiedInterval::from_expansion(&r2_exp);
    let s_ci = CertifiedInterval {
        lo: s_iv.lo,
        hi: s_iv.hi,
    };
    let off_cos = CertifiedInterval::from_expansion(&dot_diff_exp(p, arc.source.center, cb));
    let off_sin = CertifiedInterval::from_expansion(&dot_diff_exp(p, arc.source.center, sb));
    let d_cos = CertifiedInterval::from_expansion(&dot_vec_exp(dx, dy, cb.x, cb.y));
    let d_sin = CertifiedInterval::from_expansion(&dot_vec_exp(dx, dy, sb.x, sb.y));
    let u = off_cos
        .add(&d_cos.mul(&s_ci))
        .div(&r_iv)
        .ok_or(PairUnresolved::NonFiniteComputedValue)?;
    let v = off_sin
        .add(&d_sin.mul(&s_ci))
        .div(&r_iv)
        .ok_or(PairUnresolved::NonFiniteComputedValue)?;
    lift_arc_parameter(arc, u, v)
}

/// The certified lift of a circle–circle root on one arc onto the arc piece's
/// unwrapped axis, via the certified `(cos t, sin t)` intervals.
///
/// The returned [`ArcParameterLift`] is on the same authoritative unwrapped
/// axis as the piece endpoints (see [`lift_arc_parameter`]).
#[allow(clippy::too_many_arguments)]
fn arc_parameter_for_circle(
    arc: &XMonotoneCircularArc2,
    root_center: Point2,
    dcx: &Expansion,
    dcy: &Expansion,
    dist_iv: &CertifiedInterval,
    a_iv: &CertifiedInterval,
    h_iv: &CertifiedInterval,
    side: i64,
) -> Result<ArcParameterLift, PairUnresolved> {
    // The denominator enclosure comes from the exact squared radius, never
    // from the rounded `radius_squared()` scalar.
    let r2_exp = radius_squared_exp(&arc.source);
    if r2_exp.sign() == CertifiedSign::Zero {
        return Err(PairUnresolved::ParameterLocationUndecided);
    }
    let r_iv = CertifiedInterval::from_expansion(&r2_exp);
    let cb = arc.source.cos_basis;
    let sb = arc.source.sin_basis;
    let a_over_d = a_iv
        .div(dist_iv)
        .ok_or(PairUnresolved::NonFiniteComputedValue)?;
    let h_over_d = h_iv
        .div(dist_iv)
        .ok_or(PairUnresolved::NonFiniteComputedValue)?;
    let sign_h = if side >= 0 { 1.0 } else { -1.0 };
    let sh = CertifiedInterval::point(sign_h);
    let off_cos =
        CertifiedInterval::from_expansion(&dot_diff_exp(root_center, arc.source.center, cb));
    let off_sin =
        CertifiedInterval::from_expansion(&dot_diff_exp(root_center, arc.source.center, sb));
    let dc_cos = CertifiedInterval::from_expansion(&dot_vec_exp(dcx, dcy, cb.x, cb.y));
    let dc_sin = CertifiedInterval::from_expansion(&dot_vec_exp(dcx, dcy, sb.x, sb.y));
    let rot_cos = CertifiedInterval::from_expansion(&rot_dot_exp(dcx, dcy, cb.x, cb.y));
    let rot_sin = CertifiedInterval::from_expansion(&rot_dot_exp(dcx, dcy, sb.x, sb.y));
    let mut u = off_cos.add(&a_over_d.mul(&dc_cos));
    u = u
        .add(&h_over_d.mul(&rot_cos).mul(&sh))
        .div(&r_iv)
        .ok_or(PairUnresolved::NonFiniteComputedValue)?;
    let mut v = off_sin.add(&a_over_d.mul(&dc_sin));
    v = v
        .add(&h_over_d.mul(&rot_sin).mul(&sh))
        .div(&r_iv)
        .ok_or(PairUnresolved::NonFiniteComputedValue)?;
    lift_arc_parameter(arc, u, v)
}

// ---------------------------------------------------------------------------
// Piece location for the family-specific roots
// ---------------------------------------------------------------------------

/// The location of a line–circle root on the line piece, by certified
/// interval separation against `[0, 1]` with exact endpoint-identity checks.
///
/// A root whose enclosure overlaps `0` or `1` is admitted as that endpoint
/// exactly when **two** certificates hold: the line's **declared** endpoint
/// coordinate is exactly on the support circle (an exact predicate on the
/// declared geometry, replacing the rounded quadratic tests `C == 0.0` /
/// `A + B + C == 0.0`), and this root's enclosure is separated from the other
/// root's enclosure — a root box overlapping the other's cannot be identified
/// as *the* root at the boundary. Otherwise the location is undecidable; it is
/// never silently interior or exterior.
fn line_piece_location(
    line: &XMonotoneLine2,
    arc: &XMonotoneCircularArc2,
    s_iv: &ParameterEnclosure,
    other_root: Option<&ParameterEnclosure>,
) -> PieceLocation {
    if s_iv.hi < 0.0 || s_iv.lo > 1.0 {
        return PieceLocation::exterior();
    }
    if s_iv.lo <= 0.0 && 0.0 <= s_iv.hi {
        if point_is_on_circle_exact(&line.source.start, &arc.source)
            && separated_from_other_root(s_iv, other_root)
        {
            return PieceLocation {
                location: LocationOnPiece::IdentifiedEndpoint(
                    ParameterLocation::SourceStartEndpoint,
                ),
                parameter: ParameterEnclosure::from_f64(0.0),
                identity_hint: Some(IntersectionIdentity::SourceVertex(
                    line.source.provenance.start_vertex_id,
                )),
            };
        }
        return PieceLocation::undecided(NumericalCause::EnclosureOverlapsBoundary);
    }
    if s_iv.lo <= 1.0 && 1.0 <= s_iv.hi {
        if point_is_on_circle_exact(&line.source.end, &arc.source)
            && separated_from_other_root(s_iv, other_root)
        {
            return PieceLocation {
                location: LocationOnPiece::IdentifiedEndpoint(ParameterLocation::SourceEndEndpoint),
                parameter: ParameterEnclosure::from_f64(1.0),
                identity_hint: Some(IntersectionIdentity::SourceVertex(
                    line.source.provenance.end_vertex_id,
                )),
            };
        }
        return PieceLocation::undecided(NumericalCause::EnclosureOverlapsBoundary);
    }
    if s_iv.lo > 0.0 && s_iv.hi < 1.0 {
        return PieceLocation::interior(s_iv.clone());
    }
    PieceLocation::undecided(NumericalCause::EnclosureOverlapsBoundary)
}

/// The membership of a root on an x-monotone arc piece, decided by certified
/// parameter evidence (the A7 repair, GEN-001E).
///
/// A7: the previous chord-side orient test read rounded evaluated arc
/// endpoints (`arc.start.point()` / `arc.end.point()`) and decided
/// interior/exterior against a sign of the exact orient over those
/// *representatives*. That rounded geometry could decide topology. The
/// certified replacement decides membership by interval separation on the
/// **authoritative parameter axis**: an x-monotone piece covers exactly the
/// certified parameter range between its authoritative endpoints — the source's
/// declared trim values or the certified critical enclosures, never a rounded
/// `cos`/`sin` evaluation — and the root's certified parameter enclosure is
/// compared against it.
///
/// - `Exterior` when the root is certifiably below or above the piece's
///   parameter range (the piece endpoints' enclosures are upper/lower bounds on
///   the true endpoint parameters, so strict interval separation is certified);
/// - `Interior` when the root is certifiably strictly between the two endpoint
///   parameters (its enclosure is separated from both);
/// - `Boundary` when the root's enclosure touches a piece boundary — the caller
///   attempts an endpoint admission and otherwise reports `Undecidable`.
///
/// The conservative outcomes are unchanged from the orient test: a straddle
/// never invents membership. Only the deciding evidence is now certified
/// parameter evidence instead of rounded endpoint representatives.
fn arc_parameter_membership(
    arc: &XMonotoneCircularArc2,
    t_iv: &ParameterEnclosure,
) -> OrientLocation {
    let start = arc_endpoint_parameter(arc, EndpointRole::Start, &arc.start);
    let end = arc_endpoint_parameter(arc, EndpointRole::End, &arc.end);
    if arc.source.t1 == arc.source.t0 {
        return OrientLocation::Boundary;
    }
    // The piece preserves the source traversal direction, so the parameter
    // order of its endpoints is the source's: ascending when t1 > t0, else
    // descending. `lower`/`upper` are the certified endpoint enclosures at the
    // low and high parameter ends.
    let (lower, upper) = if arc.source.t1 > arc.source.t0 {
        (start, end)
    } else {
        (end, start)
    };
    if t_iv.hi < lower.lo {
        return OrientLocation::Exterior;
    }
    if t_iv.lo > upper.hi {
        return OrientLocation::Exterior;
    }
    if t_iv.lo > lower.hi && t_iv.hi < upper.lo {
        return OrientLocation::Interior;
    }
    OrientLocation::Boundary
}

/// The location of a line–circle root on the arc piece, by certified
/// parameter membership against the piece's authoritative endpoint parameters
/// (A7: no rounded endpoint representative decides the location).
#[allow(clippy::too_many_arguments)]
fn arc_location_for_line_root(
    arc: &XMonotoneCircularArc2,
    line: &XMonotoneLine2,
    dx: &Expansion,
    dy: &Expansion,
    s_iv: &ParameterEnclosure,
    other_root: Option<&ParameterEnclosure>,
) -> Result<PieceLocation, PairUnresolved> {
    let p = line.source.start;
    let lift = arc_parameter_for_line(arc, p, dx, dy, s_iv)?;
    let t_iv = match lift {
        ArcParameterLift::OffPiece => return Ok(PieceLocation::exterior()),
        ArcParameterLift::OnPiece(t) => t,
    };
    match arc_parameter_membership(arc, &t_iv) {
        OrientLocation::Interior => Ok(PieceLocation::interior(t_iv)),
        OrientLocation::Exterior => Ok(PieceLocation::exterior()),
        OrientLocation::Boundary => {
            if let Some(ad) = arc_endpoint_identity_for_line(
                arc,
                EndpointRole::Start,
                &arc.start,
                line,
                dx,
                dy,
                s_iv,
                other_root,
            ) {
                Ok(ad.into_piece_location())
            } else if let Some(ad) = arc_endpoint_identity_for_line(
                arc,
                EndpointRole::End,
                &arc.end,
                line,
                dx,
                dy,
                s_iv,
                other_root,
            ) {
                Ok(ad.into_piece_location())
            } else {
                Ok(PieceLocation::undecided(
                    NumericalCause::EnclosureOverlapsBoundary,
                ))
            }
        }
    }
}

/// The location of a circle–circle root on one arc piece, by certified
/// parameter membership against the piece's authoritative endpoint parameters
/// (A7: no rounded endpoint representative decides the location).
#[allow(clippy::too_many_arguments)]
fn arc_location_for_circle_root(
    arc: &XMonotoneCircularArc2,
    c1: Point2,
    dcx: &Expansion,
    dcy: &Expansion,
    dist_iv: &CertifiedInterval,
    a_iv: &CertifiedInterval,
    h_iv: &CertifiedInterval,
    side: i64,
) -> Result<PieceLocation, PairUnresolved> {
    let lift = arc_parameter_for_circle(arc, c1, dcx, dcy, dist_iv, a_iv, h_iv, side)?;
    let t_iv = match lift {
        ArcParameterLift::OffPiece => return Ok(PieceLocation::exterior()),
        ArcParameterLift::OnPiece(t) => t,
    };
    match arc_parameter_membership(arc, &t_iv) {
        OrientLocation::Interior => Ok(PieceLocation::interior(t_iv)),
        OrientLocation::Exterior => Ok(PieceLocation::exterior()),
        OrientLocation::Boundary => {
            if let Some(ad) = arc_endpoint_identity_for_circle(
                arc,
                EndpointRole::Start,
                &arc.start,
                c1,
                dcx,
                dcy,
                dist_iv,
                a_iv,
                h_iv,
                side,
            ) {
                Ok(ad.into_piece_location())
            } else if let Some(ad) = arc_endpoint_identity_for_circle(
                arc,
                EndpointRole::End,
                &arc.end,
                c1,
                dcx,
                dcy,
                dist_iv,
                a_iv,
                h_iv,
                side,
            ) {
                Ok(ad.into_piece_location())
            } else {
                Ok(PieceLocation::undecided(
                    NumericalCause::EnclosureOverlapsBoundary,
                ))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Representative points (evaluation seeds)
// ---------------------------------------------------------------------------

/// A representative point for a line–circle root.
fn representative_point_for_line(line: &LineSegment2, s_iv: &ParameterEnclosure) -> Point2 {
    line.point_at((s_iv.lo + s_iv.hi) * 0.5)
}

/// A representative point for a circle–circle root.
fn representative_point_for_circle(
    root_center: Point2,
    dc: Vector2,
    dist_iv: &CertifiedInterval,
    a_iv: &CertifiedInterval,
    h_iv: &CertifiedInterval,
    side: i64,
) -> Point2 {
    let a_mid = (a_iv.lo + a_iv.hi) * 0.5;
    let h_mid = (h_iv.lo + h_iv.hi) * 0.5;
    let dist_mid = (dist_iv.lo + dist_iv.hi) * 0.5;
    if dist_mid == 0.0 {
        return root_center;
    }
    let ux = dc.x / dist_mid;
    let uy = dc.y / dist_mid;
    let sign_h = if side >= 0 { 1.0 } else { -1.0 };
    Point2::new(
        root_center.x + a_mid * ux - sign_h * h_mid * uy,
        root_center.y + a_mid * uy + sign_h * h_mid * ux,
    )
}

// ---------------------------------------------------------------------------
// Line–circle intersection
// ---------------------------------------------------------------------------

/// The exact sign of the line–circle discriminant, over the exact expansions
/// `A = |d|²`, `R² = |cos_basis|²` and the exact cross `o = d×w` where
/// `w = start − center` (so `o² = O²` for `O = orient(start, end, center)`).
///
/// The quadratic in the line parameter is `A·s² + 2(w·d)s + (|w|² − R²) = 0`;
/// its discriminant is `4(A·R² − O²) = 4(A·R² − (d×w)²)` by the Lagrange
/// identity `A·|w|² − (w·d)² = (d×w)²`. The `4` factor is positive, so the
/// sign of `A·R² − (d×w)²` is the sign of the whole discriminant. Each factor
/// is an exact expansion and both products are exact expansion products
/// ([`Expansion::mul_expansion`]); `d` and `w` are exact coordinate
/// differences, so the identity holds term for term — never rounded
/// coefficients.
fn line_circle_discriminant_exp(a: &Expansion, r2: &Expansion, o: &Expansion) -> CertifiedSign {
    a.mul_expansion(r2)
        .merge(&o.mul_expansion(o).negate())
        .sign()
}

/// Combine the two piece locations of one line–circle root into a record,
/// or into the skip / unrelated-tangency / unresolved outcomes.
#[allow(clippy::too_many_arguments)]
fn combine_line_arc(
    line: &XMonotoneLine2,
    arc: &XMonotoneCircularArc2,
    line_loc: PieceLocation,
    arc_loc: PieceLocation,
    line_first: bool,
    point: Point2,
    contact: ContactKind,
    index: usize,
) -> Result<RootOutcome, PairUnresolved> {
    use LocationOnPiece::*;
    match (&line_loc.location, &arc_loc.location) {
        (Undecidable(_), _) | (_, Undecidable(_)) => {
            Err(PairUnresolved::ParameterLocationUndecided)
        }
        (Exterior, _) | (_, Exterior) => Ok(RootOutcome::Skip),
        _ => {
            let line_pl = line_loc.location.recorded().unwrap();
            let arc_pl = arc_loc.location.recorded().unwrap();
            if contact == ContactKind::Tangent && !line_arc_source_join(line, arc, line_pl, arc_pl)
            {
                return Ok(RootOutcome::UnrelatedTangency);
            }
            let line_eu = line.identity.source_occurrence.edge_use_id;
            let arc_eu = arc.identity.source_occurrence.edge_use_id;
            let (l_param, r_param, l_loc, r_loc, l_hint, r_hint, l_eu, r_eu) = if line_first {
                (
                    line_loc.parameter.clone(),
                    arc_loc.parameter.clone(),
                    line_pl,
                    arc_pl,
                    line_loc.identity_hint,
                    arc_loc.identity_hint,
                    line_eu,
                    arc_eu,
                )
            } else {
                (
                    arc_loc.parameter.clone(),
                    line_loc.parameter.clone(),
                    arc_pl,
                    line_pl,
                    arc_loc.identity_hint,
                    line_loc.identity_hint,
                    arc_eu,
                    line_eu,
                )
            };
            Ok(RootOutcome::Record(build_record(
                l_param, r_param, l_loc, r_loc, l_hint, r_hint, l_eu, r_eu, point, contact, index,
            )))
        }
    }
}

/// Process one line–circle root through both pieces' locations.
#[allow(clippy::too_many_arguments)]
fn process_line_root(
    line: &XMonotoneLine2,
    arc: &XMonotoneCircularArc2,
    dx: &Expansion,
    dy: &Expansion,
    s_iv: &ParameterEnclosure,
    other_root: Option<&ParameterEnclosure>,
    line_first: bool,
    index: usize,
    contact: ContactKind,
) -> Result<RootOutcome, PairUnresolved> {
    let line_loc = line_piece_location(line, arc, s_iv, other_root);
    let arc_loc = arc_location_for_line_root(arc, line, dx, dy, s_iv, other_root)?;
    let point = representative_point_for_line(&line.source, s_iv);
    combine_line_arc(
        line, arc, line_loc, arc_loc, line_first, point, contact, index,
    )
}

fn line_circle(
    line: &XMonotoneLine2,
    arc: &XMonotoneCircularArc2,
    intersection_index: usize,
    line_first: bool,
    policy: &IntersectionPolicy,
) -> PairIntersectionResult {
    let seg = &line.source;
    let p = seg.start;
    let q = seg.end;
    if seg.is_degenerate() {
        return PairIntersectionResult::Unsupported(PairUnsupported::Overlap);
    }
    let c = arc.source.center;

    // The exact expansions behind every decision, all built from the same
    // exact coordinate differences: d = q − p and w = p − c are `two_sum`
    // expansions ([`Expansion::from_sum`]), never rounded `f64` vectors, so
    // A = |d|², the cross d×w and the dot w·d see the identical direction.
    // The roots are certified enclosures s = (−w·d ± √(A·R² − (d×w)²)) / A,
    // and the tangent contact is the double root s = −w·d / A.
    let (dx, dy) = point_diff_exp(q, p);
    let (wx, wy) = point_diff_exp(p, c);
    let a_exp = dot_exp2(&dx, &dy, &dx, &dy);
    let r2_exp = radius_squared_exp(&arc.source);
    let dcrossw = cross_exp2(&dx, &dy, &wx, &wy);
    let dot_exp = dot_exp2(&wx, &wy, &dx, &dy);
    let a_iv = CertifiedInterval::from_expansion(&a_exp);
    let dot_iv = CertifiedInterval::from_expansion(&dot_exp);
    if !a_iv.is_finite() {
        return PairIntersectionResult::Unresolved(PairUnresolved::NonFiniteComputedValue);
    }

    match line_circle_discriminant_exp(&a_exp, &r2_exp, &dcrossw) {
        CertifiedSign::Negative => PairIntersectionResult::Disjoint,
        CertifiedSign::Zero => {
            // Tangent to the support circle: one contact at the double root
            // s = −w·d / |d|², a certified enclosure of the foot.
            let q = match dot_iv.neg().div(&a_iv) {
                Some(q) => q,
                None => {
                    return PairIntersectionResult::Unresolved(
                        PairUnresolved::NonFiniteComputedValue,
                    )
                }
            };
            if !q.is_finite() {
                return PairIntersectionResult::Unresolved(PairUnresolved::NonFiniteComputedValue);
            }
            let s_iv = ParameterEnclosure { lo: q.lo, hi: q.hi };
            match process_line_root(
                line,
                arc,
                &dx,
                &dy,
                &s_iv,
                None,
                line_first,
                intersection_index,
                ContactKind::Tangent,
            ) {
                Ok(RootOutcome::Record(rec)) => PairIntersectionResult::Intersections(vec![rec]),
                Ok(RootOutcome::Skip) => PairIntersectionResult::Disjoint,
                Ok(RootOutcome::UnrelatedTangency) => {
                    PairIntersectionResult::Unsupported(PairUnsupported::UnrelatedTangency)
                }
                Err(u) => PairIntersectionResult::Unresolved(u),
            }
        }
        CertifiedSign::Positive => {
            // Two distinct support-circle intersections, certified by the
            // exact discriminant sign; each root is a certified enclosure.
            let d4 = a_exp
                .mul_expansion(&r2_exp)
                .merge(&dcrossw.mul_expansion(&dcrossw).negate());
            let d4_iv = CertifiedInterval::from_expansion(&d4);
            if d4_iv.lo <= 0.0 {
                return PairIntersectionResult::Unresolved(PairUnresolved::RootsBelowF64Resolution);
            }
            let sqrt_d = match d4_iv.sqrt() {
                Some(s) => s,
                None => {
                    return PairIntersectionResult::Unresolved(
                        PairUnresolved::RootsBelowF64Resolution,
                    )
                }
            };
            let n1 = dot_iv.neg().sub(&sqrt_d);
            let n2 = dot_iv.neg().add(&sqrt_d);
            let s0 = match n1.div(&a_iv) {
                Some(x) => x,
                None => {
                    return PairIntersectionResult::Unresolved(
                        PairUnresolved::NonFiniteComputedValue,
                    )
                }
            };
            let s1 = match n2.div(&a_iv) {
                Some(x) => x,
                None => {
                    return PairIntersectionResult::Unresolved(
                        PairUnresolved::NonFiniteComputedValue,
                    )
                }
            };
            let s0_enc = ParameterEnclosure {
                lo: s0.lo,
                hi: s0.hi,
            };
            let s1_enc = ParameterEnclosure {
                lo: s1.lo,
                hi: s1.hi,
            };

            let mut intersections = Vec::new();
            for (root, other) in [(&s0_enc, &s1_enc), (&s1_enc, &s0_enc)] {
                match process_line_root(
                    line,
                    arc,
                    &dx,
                    &dy,
                    root,
                    Some(other),
                    line_first,
                    intersection_index,
                    ContactKind::Transverse,
                ) {
                    Ok(RootOutcome::Record(rec)) => {
                        intersections.push(rec);
                        if intersections.len() >= policy.max_intersections {
                            break;
                        }
                    }
                    Ok(RootOutcome::Skip) => {}
                    Ok(RootOutcome::UnrelatedTangency) => {
                        return PairIntersectionResult::Unsupported(
                            PairUnsupported::UnrelatedTangency,
                        );
                    }
                    Err(u) => return PairIntersectionResult::Unresolved(u),
                }
            }
            if intersections.is_empty() {
                PairIntersectionResult::Disjoint
            } else {
                PairIntersectionResult::Intersections(intersections)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Circle–circle intersection
// ---------------------------------------------------------------------------

/// The exact sign of the circle–circle radical-axis discriminant
/// `S = 2A(R1+R2) − A² − (R1−R2)²`, expanded to the exact polynomial
/// `2A·R1 + 2A·R2 − R1² − R2² − A² + 2R1R2`. `A`, `R1` and `R2` are exact
/// expansions (exact squared distance / exact squared radii), and every term
/// is an exact expansion product ([`Expansion::mul_expansion`]) — never a
/// rounded `f64` coefficient.
fn circle_circle_discriminant(a: &Expansion, r1: &Expansion, r2: &Expansion) -> CertifiedSign {
    let mut s = a.mul_expansion(r1).scale_expansion_by_pow2(1); // 2A·R1
    s = s.merge(&a.mul_expansion(r2).scale_expansion_by_pow2(1)); // + 2A·R2
    s = s.merge(&r1.mul_expansion(r1).negate()); // − R1²
    s = s.merge(&r2.mul_expansion(r2).negate()); // − R2²
    s = s.merge(&a.mul_expansion(a).negate()); // − A²
    s = s.merge(&r1.mul_expansion(r2).scale_expansion_by_pow2(1)); // + 2R1R2
    s.sign()
}

/// Combine the two piece locations of one circle–circle root into a record,
/// or into the skip / unrelated-tangency / unresolved outcomes.
fn combine_arc_arc(
    lhs: &XMonotoneCircularArc2,
    rhs: &XMonotoneCircularArc2,
    lhs_loc: PieceLocation,
    rhs_loc: PieceLocation,
    point: Point2,
    contact: ContactKind,
    index: usize,
) -> Result<RootOutcome, PairUnresolved> {
    use LocationOnPiece::*;
    match (&lhs_loc.location, &rhs_loc.location) {
        (Undecidable(_), _) | (_, Undecidable(_)) => {
            Err(PairUnresolved::ParameterLocationUndecided)
        }
        (Exterior, _) | (_, Exterior) => Ok(RootOutcome::Skip),
        _ => {
            let lhs_pl = lhs_loc.location.recorded().unwrap();
            let rhs_pl = rhs_loc.location.recorded().unwrap();
            if contact == ContactKind::Tangent && !arc_arc_source_join(lhs, rhs, lhs_pl, rhs_pl) {
                return Ok(RootOutcome::UnrelatedTangency);
            }
            let lhs_eu = lhs.identity.source_occurrence.edge_use_id;
            let rhs_eu = rhs.identity.source_occurrence.edge_use_id;
            Ok(RootOutcome::Record(build_record(
                lhs_loc.parameter.clone(),
                rhs_loc.parameter.clone(),
                lhs_pl,
                rhs_pl,
                lhs_loc.identity_hint,
                rhs_loc.identity_hint,
                lhs_eu,
                rhs_eu,
                point,
                contact,
                index,
            )))
        }
    }
}

/// Process one circle–circle root through both pieces' locations.
#[allow(clippy::too_many_arguments)]
fn process_circle_root(
    lhs: &XMonotoneCircularArc2,
    rhs: &XMonotoneCircularArc2,
    c1: Point2,
    dcx: &Expansion,
    dcy: &Expansion,
    dc: Vector2,
    dist_iv: &CertifiedInterval,
    a_iv: &CertifiedInterval,
    h_iv: &CertifiedInterval,
    side: i64,
    index: usize,
    contact: ContactKind,
) -> Result<RootOutcome, PairUnresolved> {
    let lhs_loc = arc_location_for_circle_root(lhs, c1, dcx, dcy, dist_iv, a_iv, h_iv, side)?;
    let rhs_loc = arc_location_for_circle_root(rhs, c1, dcx, dcy, dist_iv, a_iv, h_iv, side)?;
    // The rounded `dc` is a representative/evaluation hint only.
    let point = representative_point_for_circle(c1, dc, dist_iv, a_iv, h_iv, side);
    combine_arc_arc(lhs, rhs, lhs_loc, rhs_loc, point, contact, index)
}

fn circle_circle(
    lhs: &XMonotoneCircularArc2,
    rhs: &XMonotoneCircularArc2,
    intersection_index: usize,
    policy: &IntersectionPolicy,
) -> PairIntersectionResult {
    let c1 = lhs.source.center;
    let c2 = rhs.source.center;

    // The exact expansions behind every decision: A = |c2 − c1|² is the exact
    // squared center distance, and R1², R2² are the exact squared radii. The
    // center difference `dc = c2 − c1` is an exact coordinate-difference
    // expansion (`two_sum`), used consistently for the radical-axis foot, the
    // orient test and the arc-parameter enclosures — exactly as in the
    // line–circle path. A rounded `Vector2` appears only as a
    // representative/evaluation hint.
    let (dcx, dcy) = point_diff_exp(c2, c1);
    let dc = Vector2::new(c2.x - c1.x, c2.y - c1.y);
    let a_exp = exact_sq_dist([c1.x, c1.y], [c2.x, c2.y]);
    let r1_exp = radius_squared_exp(&lhs.source);
    let r2_exp = radius_squared_exp(&rhs.source);

    // Concentric: the exact statement is decisive.
    if a_exp.sign() == CertifiedSign::Zero {
        if r1_exp.merge(&r2_exp.negate()).sign() == CertifiedSign::Zero {
            return PairIntersectionResult::Unsupported(PairUnsupported::CoincidentCircles);
        }
        return PairIntersectionResult::Disjoint;
    }

    let dist_sq_iv = CertifiedInterval::from_expansion(&a_exp);
    let r1_iv = CertifiedInterval::from_expansion(&r1_exp);
    let r2_iv = CertifiedInterval::from_expansion(&r2_exp);
    let dist_iv = match dist_sq_iv.sqrt() {
        Some(d) => d,
        None => return PairIntersectionResult::Unresolved(PairUnresolved::NonFiniteComputedValue),
    };
    if dist_iv.lo <= 0.0 {
        return PairIntersectionResult::Unresolved(PairUnresolved::NonFiniteComputedValue);
    }

    // The radical-axis foot along the center axis, as a certified interval
    // over the exact expansions (never rounded `f64` squared quantities).
    let two_dist = dist_iv.scale_pow2(1);
    let num = r1_iv.sub(&r2_iv).add(&dist_sq_iv);
    let a_iv = match num.div(&two_dist) {
        Some(x) => x,
        None => return PairIntersectionResult::Unresolved(PairUnresolved::NonFiniteComputedValue),
    };

    match circle_circle_discriminant(&a_exp, &r1_exp, &r2_exp) {
        CertifiedSign::Negative => PairIntersectionResult::Disjoint,
        CertifiedSign::Zero => {
            // Tangent: one contact at the radical-line foot.
            let h_iv = CertifiedInterval::point(0.0);
            match process_circle_root(
                lhs,
                rhs,
                c1,
                &dcx,
                &dcy,
                dc,
                &dist_iv,
                &a_iv,
                &h_iv,
                0,
                intersection_index,
                ContactKind::Tangent,
            ) {
                Ok(RootOutcome::Record(rec)) => PairIntersectionResult::Intersections(vec![rec]),
                Ok(RootOutcome::Skip) => PairIntersectionResult::Disjoint,
                Ok(RootOutcome::UnrelatedTangency) => {
                    PairIntersectionResult::Unsupported(PairUnsupported::UnrelatedTangency)
                }
                Err(u) => PairIntersectionResult::Unresolved(u),
            }
        }
        CertifiedSign::Positive => {
            // Two distinct intersections; the chord half-length is certified
            // real by the discriminant sign, over the exact intervals.
            let h_sq_iv = r1_iv.sub(&a_iv.mul(&a_iv));
            if h_sq_iv.lo <= 0.0 {
                return PairIntersectionResult::Unresolved(PairUnresolved::RootsBelowF64Resolution);
            }
            let h_iv = match h_sq_iv.sqrt() {
                Some(h) => h,
                None => {
                    return PairIntersectionResult::Unresolved(
                        PairUnresolved::RootsBelowF64Resolution,
                    )
                }
            };

            let mut intersections = Vec::new();
            for (idx, side) in [1i64, -1i64].iter().enumerate() {
                match process_circle_root(
                    lhs,
                    rhs,
                    c1,
                    &dcx,
                    &dcy,
                    dc,
                    &dist_iv,
                    &a_iv,
                    &h_iv,
                    *side,
                    intersection_index + idx,
                    ContactKind::Transverse,
                ) {
                    Ok(RootOutcome::Record(rec)) => {
                        intersections.push(rec);
                        if intersections.len() >= policy.max_intersections {
                            break;
                        }
                    }
                    Ok(RootOutcome::Skip) => {}
                    Ok(RootOutcome::UnrelatedTangency) => {
                        return PairIntersectionResult::Unsupported(
                            PairUnsupported::UnrelatedTangency,
                        );
                    }
                    Err(u) => return PairIntersectionResult::Unresolved(u),
                }
            }
            if intersections.is_empty() {
                PairIntersectionResult::Disjoint
            } else {
                PairIntersectionResult::Intersections(intersections)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::curve2d::{
        CurveOccurrenceProvenance, DevelopedCurve2D, SourceEdgeId, SourceEntityId, SourceFaceId,
    };
    use super::super::xmonotone::{
        make_x_monotone, ClosedInterval, DecompositionKind, MonotoneKind, NumericalPolicy,
        PieceIdentity,
    };
    use super::*;
    use crate::source_evidence::{BoundId, EdgeUseId, SourceVertexKey};

    const PI: f64 = std::f64::consts::PI;

    fn provenance_with(
        index: usize,
        start: SourceVertexKey,
        end: SourceVertexKey,
    ) -> CurveOccurrenceProvenance {
        CurveOccurrenceProvenance {
            source_face_id: Some(SourceFaceId(42)),
            bound_id: BoundId(1),
            edge_use_id: EdgeUseId::new(BoundId(1), index),
            source_edge_id: SourceEdgeId(7 + index),
            start_vertex_id: start,
            end_vertex_id: end,
            source_curve_entity_id: Some(SourceEntityId(99)),
        }
    }

    fn provenance() -> CurveOccurrenceProvenance {
        provenance_with(
            0,
            SourceVertexKey::ShellVertex(3),
            SourceVertexKey::ShellVertex(4),
        )
    }

    fn line_piece(start: Point2, end: Point2) -> XMonotonePiece2 {
        let curve = DevelopedCurve2D::Line(LineSegment2 {
            start,
            end,
            provenance: provenance(),
        });
        make_x_monotone(&curve, &NumericalPolicy::standard())
            .unwrap()
            .remove(0)
    }

    /// A circle arc of radius `r` at `center`, with canonical basis, over
    /// `[t0, t1]`. The returned pieces are all of the circle's x-monotone
    /// decomposition.
    fn arc_pieces(
        center: Point2,
        r: f64,
        t0: f64,
        t1: f64,
        edge_use: usize,
    ) -> Vec<XMonotonePiece2> {
        let arc = DirectedCircularArc2 {
            center,
            cos_basis: Vector2::new(r, 0.0),
            sin_basis: Vector2::new(0.0, r),
            t0,
            t1,
            provenance: provenance_with(
                edge_use,
                SourceVertexKey::ShellVertex(10 + edge_use),
                SourceVertexKey::ShellVertex(20 + edge_use),
            ),
        };
        make_x_monotone(
            &DevelopedCurve2D::CircularArc(arc),
            &NumericalPolicy::standard(),
        )
        .unwrap()
    }

    fn intersect(lhs: &XMonotonePiece2, rhs: &XMonotonePiece2) -> PairIntersectionResult {
        intersect_x_monotone(lhs, rhs, &IntersectionPolicy::standard())
    }

    /// Count total intersections of a curve against all pieces of a
    /// decomposed circle.
    fn total_intersections(
        pieces: &[XMonotonePiece2],
        other: &XMonotonePiece2,
        lhs_side: bool,
    ) -> Vec<CertifiedIntersection2> {
        let mut out = Vec::new();
        for piece in pieces {
            let result = if lhs_side {
                intersect(piece, other)
            } else {
                intersect(other, piece)
            };
            match result {
                PairIntersectionResult::Intersections(pts) => out.extend(pts),
                _ => {}
            }
        }
        out
    }

    // -- exact discriminant signs -------------------------------------------

    #[test]
    fn line_circle_discriminant_sign_is_exact() {
        // d = (6, 0): A = 36, radius² = 4, O = orient((−3,1),(3,1),(0,0)) = −6,
        // so A·R² − O² = 144 − 36 > 0: two real support intersections.
        let a = Expansion::from_product(6.0, 6.0);
        let r2 = Expansion::from_product(2.0, 2.0);
        let o = orient_exp(
            Point2::new(-3.0, 1.0),
            Point2::new(3.0, 1.0),
            Point2::new(0.0, 0.0),
        );
        assert_eq!(
            line_circle_discriminant_exp(&a, &r2, &o),
            CertifiedSign::Positive
        );

        // Tangent line x = 2 through the radius-2 circle: A = 4, O = 4, so
        // A·R² − O² = 16 − 16 = 0 exactly.
        let a = Expansion::from_product(2.0, 2.0);
        let o = orient_exp(
            Point2::new(2.0, -1.0),
            Point2::new(2.0, 1.0),
            Point2::new(0.0, 0.0),
        );
        assert_eq!(
            line_circle_discriminant_exp(&a, &r2, &o),
            CertifiedSign::Zero
        );

        // Line x = 3: A = 4, O = 6, so 16 − 36 < 0: no real intersections.
        let o = orient_exp(
            Point2::new(3.0, -1.0),
            Point2::new(3.0, 1.0),
            Point2::new(0.0, 0.0),
        );
        assert_eq!(
            line_circle_discriminant_exp(&a, &r2, &o),
            CertifiedSign::Negative
        );
    }

    #[test]
    fn circle_circle_discriminant_sign_is_exact() {
        // R1 = R2 = 4 (radius²), A = 4 (distance²):
        // S = 2·4·4 + 2·4·4 − 16 − 16 − 16 + 2·4·4 = 48 > 0.
        let a = Expansion::from_product(2.0, 2.0);
        let r = Expansion::from_product(2.0, 2.0);
        assert_eq!(
            circle_circle_discriminant(&a, &r, &r),
            CertifiedSign::Positive
        );

        // Concentric equal: A = 0 → S = −16 − 16 + 32 = 0.
        let a0 = Expansion::zero();
        assert_eq!(circle_circle_discriminant(&a0, &r, &r), CertifiedSign::Zero);

        // A = 100, R1 = R2 = 4: S = 1600 − 10000 < 0.
        let a100 = Expansion::from_product(10.0, 10.0);
        assert_eq!(
            circle_circle_discriminant(&a100, &r, &r),
            CertifiedSign::Negative
        );

        // A = 16, R1 = R2 = 4: S = 256 − 256 = 0 (internal tangency).
        let a16 = Expansion::from_product(4.0, 4.0);
        assert_eq!(
            circle_circle_discriminant(&a16, &r, &r),
            CertifiedSign::Zero
        );
    }

    // -- line–line ---------------------------------------------------------

    #[test]
    fn line_line_crossing() {
        let a = line_piece(Point2::new(0.0, 0.0), Point2::new(2.0, 2.0));
        let b = line_piece(Point2::new(0.0, 2.0), Point2::new(2.0, 0.0));
        match intersect(&a, &b) {
            PairIntersectionResult::Intersections(pts) => {
                assert_eq!(pts.len(), 1);
                assert!((pts[0].point.x - 1.0).abs() < 1e-10);
                assert!((pts[0].point.y - 1.0).abs() < 1e-10);
                assert_eq!(pts[0].contact, ContactKind::Transverse);
                assert_eq!(pts[0].lhs_location, ParameterLocation::PieceInterior);
                assert_eq!(pts[0].rhs_location, ParameterLocation::PieceInterior);
                assert!(pts[0].lhs_parameter.contains(0.5));
                assert!(pts[0].rhs_parameter.contains(0.5));
            }
            other => panic!("expected crossing, got {:?}", other.tag()),
        }
    }

    #[test]
    fn line_line_disjoint() {
        let a = line_piece(Point2::new(0.0, 0.0), Point2::new(1.0, 0.0));
        let b = line_piece(Point2::new(0.0, 1.0), Point2::new(1.0, 1.0));
        match intersect(&a, &b) {
            PairIntersectionResult::Disjoint => {}
            other => panic!("expected disjoint, got {:?}", other.tag()),
        }
    }

    #[test]
    fn line_line_overlap() {
        let a = line_piece(Point2::new(0.0, 0.0), Point2::new(2.0, 0.0));
        let b = line_piece(Point2::new(1.0, 0.0), Point2::new(3.0, 0.0));
        match intersect(&a, &b) {
            PairIntersectionResult::Unsupported(PairUnsupported::Overlap) => {}
            other => panic!("expected overlap, got {:?}", other.tag()),
        }
    }

    #[test]
    fn line_line_shared_endpoint() {
        // b starts at a's end: the shared point is certified by the exact
        // orientation predicate, and its parameter location is b's source
        // start endpoint.
        let a = line_piece(Point2::new(0.0, 0.0), Point2::new(1.0, 0.0));
        let b = line_piece(Point2::new(1.0, 0.0), Point2::new(2.0, 1.0));
        match intersect(&a, &b) {
            PairIntersectionResult::Intersections(pts) => {
                assert_eq!(pts.len(), 1);
                assert_eq!(pts[0].point, Point2::new(1.0, 0.0));
                assert_eq!(pts[0].rhs_location, ParameterLocation::SourceStartEndpoint);
                assert_eq!(pts[0].lhs_location, ParameterLocation::SourceEndEndpoint);
            }
            other => panic!("expected endpoint intersection, got {:?}", other.tag()),
        }
    }

    #[test]
    fn line_endpoint_on_segment_interior_is_a_resolved_contact() {
        // The mixed contact: a's end (1, 0) lies in the interior of b. The
        // supports are not collinear, which is the realizable form of this
        // location pair (two *closed* collinear segments that share an
        // interior point always overlap in a positive interval, so
        // classify_segments yields Overlap, never this Point). The result
        // must be a classified contact — one exact 0/1 parameter, one
        // certified interior parameter — never Unresolved.
        let a = line_piece(Point2::new(1.0, 1.0), Point2::new(1.0, 0.0));
        let b = line_piece(Point2::new(0.0, 0.0), Point2::new(2.0, 0.0));
        match intersect(&a, &b) {
            PairIntersectionResult::Intersections(pts) => {
                assert_eq!(pts.len(), 1);
                assert_eq!(pts[0].point, Point2::new(1.0, 0.0));
                assert_eq!(pts[0].lhs_location, ParameterLocation::SourceEndEndpoint);
                assert_eq!(pts[0].rhs_location, ParameterLocation::PieceInterior);
                assert_eq!(
                    pts[0].lhs_parameter,
                    ParameterEnclosure::from_f64(1.0),
                    "endpoint parameter is exact 1"
                );
                assert!(
                    pts[0].rhs_parameter.contains(0.5),
                    "interior parameter certified from the exact endpoint"
                );
                assert_eq!(pts[0].contact, ContactKind::Transverse);
            }
            PairIntersectionResult::Unresolved(_) => {
                panic!("endpoint-on-interior contact must resolve, not be Unresolved")
            }
            other => panic!("expected a contact, got {:?}", other.tag()),
        }
    }

    #[test]
    fn collinear_endpoint_on_interior_uses_the_exact_endpoint_parameter() {
        // The denominator-vanishing sub-case: one curve's endpoint lies on the
        // other curve's interior with *collinear* supports, so d1 × d2 = 0 and
        // the cross-ratio denominator vanishes. The interior parameter must
        // come from certified_line_parameter over the known endpoint, not from
        // the cross ratio. classify_segments can never produce this exact
        // location pair from two closed segments, so this regression drives
        // the parameter helper directly with the fabricated pair to pin the
        // collinear path.
        let lhs = match line_piece(Point2::new(0.0, 0.0), Point2::new(2.0, 0.0)) {
            XMonotonePiece2::Line(l) => l,
            _ => unreachable!("line_piece yields a line"),
        };
        let rhs = match line_piece(Point2::new(1.0, 0.0), Point2::new(3.0, 0.0)) {
            XMonotonePiece2::Line(l) => l,
            _ => unreachable!("line_piece yields a line"),
        };
        let (lhs_enc, rhs_enc) = line_line_parameter_enclosures(
            &lhs,
            &rhs,
            ParameterLocation::SourceEndEndpoint,
            ParameterLocation::PieceInterior,
            Point2::new(2.0, 0.0),
        )
        .expect("the exact shared endpoint certifies both parameters");
        assert_eq!(
            lhs_enc,
            ParameterEnclosure::from_f64(1.0),
            "endpoint parameter is exact 1"
        );
        assert!(
            rhs_enc.contains(0.5),
            "interior parameter computed from the exact endpoint, not the vanishing cross-ratio"
        );
    }

    // -- line–circle -------------------------------------------------------

    #[test]
    fn line_crosses_upper_semicircle_twice() {
        // Horizontal line y = 1 through the radius-2 upper semicircle: both
        // roots of the support circle lie on the upper arc [0, π].
        let line = line_piece(Point2::new(-3.0, 1.0), Point2::new(3.0, 1.0));
        let arc = arc_pieces(Point2::new(0.0, 0.0), 2.0, 0.0, PI, 0);
        assert_eq!(arc.len(), 1);
        match intersect(&line, &arc[0]) {
            PairIntersectionResult::Intersections(pts) => {
                assert_eq!(pts.len(), 2, "line crosses the arc twice");
                for pt in &pts {
                    let r = (pt.point.x * pt.point.x + pt.point.y * pt.point.y).sqrt();
                    assert!((r - 2.0).abs() < 1e-8, "point not on circle: r={r}");
                }
                assert_eq!(pts[0].contact, ContactKind::Transverse);
                assert!(pts
                    .iter()
                    .all(|p| p.lhs_location == ParameterLocation::PieceInterior));
                assert!(pts
                    .iter()
                    .all(|p| p.rhs_location == ParameterLocation::PieceInterior));
            }
            other => panic!("expected two intersections, got {:?}", other.tag()),
        }
    }

    #[test]
    fn line_circle_disjoint() {
        let line = line_piece(Point2::new(10.0, 10.0), Point2::new(12.0, 12.0));
        let arc = arc_pieces(Point2::new(0.0, 0.0), 2.0, 0.0, PI, 0);
        match intersect(&line, &arc[0]) {
            PairIntersectionResult::Disjoint => {}
            other => panic!("expected disjoint, got {:?}", other.tag()),
        }
    }

    #[test]
    fn line_below_the_upper_semicircle_hits_only_the_lower_half() {
        // Horizontal line y = -1: the support-circle roots are at
        // (±√3, -1), both below the upper arc [0, π], so a single-piece
        // upper arc has no intersection; across the full circle both are
        // found.
        let line = line_piece(Point2::new(-3.0, -1.0), Point2::new(3.0, -1.0));
        let upper = arc_pieces(Point2::new(0.0, 0.0), 2.0, 0.0, PI, 0);
        match intersect(&line, &upper[0]) {
            PairIntersectionResult::Disjoint => {}
            other => panic!("expected disjoint from upper arc, got {:?}", other.tag()),
        }
        let full = arc_pieces(Point2::new(0.0, 0.0), 2.0, 0.0, 2.0 * PI, 0);
        let hits = total_intersections(&full, &line, false);
        assert_eq!(hits.len(), 2, "full circle should yield both intersections");
    }

    // -- circle–circle -----------------------------------------------------

    #[test]
    fn two_upper_semicircles_intersect_once() {
        // Circle 1: origin, radius 2, upper semicircle [0, π].
        // Circle 2: (2, 0), radius 2, upper semicircle [0, π].
        // Support-circle intersections at (1, ±√3); only (1, √3) lies on
        // both upper arcs.
        let c1 = arc_pieces(Point2::new(0.0, 0.0), 2.0, 0.0, PI, 0);
        let c2 = arc_pieces(Point2::new(2.0, 0.0), 2.0, 0.0, PI, 1);
        assert_eq!(c1.len(), 1);
        assert_eq!(c2.len(), 1);
        match intersect(&c1[0], &c2[0]) {
            PairIntersectionResult::Intersections(pts) => {
                assert_eq!(pts.len(), 1, "upper semicircles meet exactly once");
                let p = pts[0].point;
                let r1 = (p.x * p.x + p.y * p.y).sqrt();
                let dx = p.x - 2.0;
                let r2 = (dx * dx + p.y * p.y).sqrt();
                assert!((r1 - 2.0).abs() < 1e-8);
                assert!((r2 - 2.0).abs() < 1e-8);
                assert!(p.y > 0.0, "intersection on the upper halves");
                assert_eq!(pts[0].contact, ContactKind::Transverse);
            }
            other => panic!("expected one intersection, got {:?}", other.tag()),
        }
    }

    #[test]
    fn two_disjoint_circles() {
        let c1 = arc_pieces(Point2::new(0.0, 0.0), 1.0, 0.0, 2.0 * PI, 0);
        let c2 = arc_pieces(Point2::new(10.0, 0.0), 1.0, 0.0, 2.0 * PI, 1);
        let any_hit = c1
            .iter()
            .zip(c2.iter())
            .any(|(a, b)| matches!(intersect(a, b), PairIntersectionResult::Intersections(_)));
        assert!(!any_hit, "far circles never intersect");
    }

    #[test]
    fn two_concentric_unequal_circles_are_disjoint() {
        let c1 = arc_pieces(Point2::new(0.0, 0.0), 1.0, 0.0, 2.0 * PI, 0);
        let c2 = arc_pieces(Point2::new(0.0, 0.0), 2.0, 0.0, 2.0 * PI, 1);
        match intersect(&c1[0], &c2[0]) {
            PairIntersectionResult::Disjoint => {}
            other => panic!("expected disjoint, got {:?}", other.tag()),
        }
    }

    #[test]
    fn two_coincident_circles_are_unsupported() {
        let c1 = arc_pieces(Point2::new(0.0, 0.0), 1.0, 0.0, 2.0 * PI, 0);
        let c2 = arc_pieces(Point2::new(0.0, 0.0), 1.0, 0.0, 2.0 * PI, 1);
        match intersect(&c1[0], &c2[0]) {
            PairIntersectionResult::Unsupported(PairUnsupported::CoincidentCircles) => {}
            other => panic!(
                "expected coincident-circles unsupported, got {:?}",
                other.tag()
            ),
        }
    }

    #[test]
    fn external_tangent_circles_are_unrelated_tangency() {
        // Two radius-2 circles whose centers are 4 apart touch externally
        // at (2, 0); no shared source vertex, so it is unrelated tangency.
        let c1 = arc_pieces(Point2::new(0.0, 0.0), 2.0, 0.0, 2.0 * PI, 0);
        let c2 = arc_pieces(Point2::new(4.0, 0.0), 2.0, 0.0, 2.0 * PI, 1);
        // The tangent point (2,0) is on both full circles (all pieces).
        let mut saw_tangency = false;
        for a in &c1 {
            for b in &c2 {
                match intersect(a, b) {
                    PairIntersectionResult::Unsupported(PairUnsupported::UnrelatedTangency) => {
                        saw_tangency = true;
                    }
                    PairIntersectionResult::Unresolved(_) => {}
                    _ => {}
                }
            }
        }
        assert!(
            saw_tangency,
            "external tangency must be unsupported tangency"
        );
    }

    // -- rotated basis through the certified decomposition ------------------

    #[test]
    fn rotated_basis_line_circle_intersects() {
        // Circle radius 2 rotated by 0.7 rad; a horizontal line y = 1
        // crossing it at two interior points (not at the x-extrema). The
        // decomposition and intersection must agree with the canonical
        // result (the rotation is in ARR-001's envelope).
        let theta: f64 = 0.7;
        let arc = DirectedCircularArc2 {
            center: Point2::new(0.0, 0.0),
            cos_basis: Vector2::new(2.0 * theta.cos(), 2.0 * theta.sin()),
            sin_basis: Vector2::new(-2.0 * theta.sin(), 2.0 * theta.cos()),
            t0: 0.0,
            t1: 2.0 * PI,
            provenance: provenance_with(
                0,
                SourceVertexKey::ShellVertex(3),
                SourceVertexKey::ShellVertex(4),
            ),
        };
        let curve = DevelopedCurve2D::CircularArc(arc);
        let pieces = make_x_monotone(&curve, &NumericalPolicy::standard())
            .expect("rotated basis decomposes");
        let line = line_piece(Point2::new(-3.0, 1.0), Point2::new(3.0, 1.0));
        let hits = total_intersections(&pieces, &line, false);
        assert_eq!(hits.len(), 2, "a line at y=1 crosses the circle twice");
        for hit in &hits {
            let r = (hit.point.x * hit.point.x + hit.point.y * hit.point.y).sqrt();
            assert!((r - 2.0).abs() < 1e-8);
            assert!(hit.lhs_location == ParameterLocation::PieceInterior);
        }
    }

    #[test]
    fn diameter_through_rotated_extrema_is_never_an_interior_intersection() {
        // A line through the rotated circle's x-extrema lands its
        // intersections exactly at the artificial monotone-split criticals.
        // The invariant: such an intersection is never certified as an
        // *interior* point of an x-monotone piece. It is either certified
        // as the artificial monotone-split endpoint (identity match with
        // the certified critical construction — the case here, since
        // cos²(0.7)+sin²(0.7) rounds to exactly 1 so r = 2 exactly) or it
        // is left Unresolved (when no exact endpoint admission exists).
        let theta: f64 = 0.7;
        let arc = DirectedCircularArc2 {
            center: Point2::new(0.0, 0.0),
            cos_basis: Vector2::new(2.0 * theta.cos(), 2.0 * theta.sin()),
            sin_basis: Vector2::new(-2.0 * theta.sin(), 2.0 * theta.cos()),
            t0: 0.0,
            t1: 2.0 * PI,
            provenance: provenance_with(
                0,
                SourceVertexKey::ShellVertex(3),
                SourceVertexKey::ShellVertex(4),
            ),
        };
        let curve = DevelopedCurve2D::CircularArc(arc);
        let pieces = make_x_monotone(&curve, &NumericalPolicy::standard())
            .expect("rotated basis decomposes");
        let line = line_piece(Point2::new(-3.0, 0.0), Point2::new(3.0, 0.0));
        let mut saw_resolution = false;
        for piece in &pieces {
            match intersect(&line, piece) {
                PairIntersectionResult::Intersections(pts) => {
                    for p in &pts {
                        assert_eq!(
                            p.rhs_location,
                            ParameterLocation::ArtificialPieceEndpoint,
                            "an intersection at an extremum must not be certified interior"
                        );
                        saw_resolution = true;
                    }
                }
                PairIntersectionResult::Unresolved(_) => saw_resolution = true,
                PairIntersectionResult::Disjoint | PairIntersectionResult::Unsupported(_) => {}
            }
        }
        assert!(
            saw_resolution,
            "the diameter line must resolve or be Unresolved, never silently drop"
        );
    }

    #[test]
    fn rotated_basis_circle_circle_intersects() {
        let theta: f64 = 0.7;
        let arc1 = DirectedCircularArc2 {
            center: Point2::new(0.0, 0.0),
            cos_basis: Vector2::new(2.0 * theta.cos(), 2.0 * theta.sin()),
            sin_basis: Vector2::new(-2.0 * theta.sin(), 2.0 * theta.cos()),
            t0: 0.0,
            t1: 2.0 * PI,
            provenance: provenance_with(
                0,
                SourceVertexKey::ShellVertex(3),
                SourceVertexKey::ShellVertex(4),
            ),
        };
        let arc2 = DirectedCircularArc2 {
            center: Point2::new(2.0, 0.0),
            cos_basis: Vector2::new(2.0, 0.0),
            sin_basis: Vector2::new(0.0, 2.0),
            t0: 0.0,
            t1: 2.0 * PI,
            provenance: provenance_with(
                1,
                SourceVertexKey::ShellVertex(5),
                SourceVertexKey::ShellVertex(6),
            ),
        };
        let pieces1 = make_x_monotone(
            &DevelopedCurve2D::CircularArc(arc1),
            &NumericalPolicy::standard(),
        )
        .expect("rotated circle decomposes");
        let pieces2 = make_x_monotone(
            &DevelopedCurve2D::CircularArc(arc2),
            &NumericalPolicy::standard(),
        )
        .expect("canonical circle decomposes");

        let mut count = 0;
        for a in &pieces1 {
            for b in &pieces2 {
                match intersect(a, b) {
                    PairIntersectionResult::Intersections(pts) => count += pts.len(),
                    PairIntersectionResult::Unresolved(_)
                    | PairIntersectionResult::Unsupported(_) => {}
                    PairIntersectionResult::Disjoint => {}
                }
            }
        }
        assert_eq!(
            count, 2,
            "two radius-2 circles at distance 2 intersect twice"
        );
    }

    // -- exhaustive location: certified exterior is discarded, nothing else --

    #[test]
    fn near_start_root_is_represented_or_the_pair_is_unresolved() {
        // A line whose support line crosses the unit circle just after its
        // source start: the near-start root's certified enclosure is within
        // ulps of s = 0. This regression is satisfied only by the *near* root
        // itself — a far-root-only resolution does not count. Either the near
        // root is represented (a recorded intersection with line parameter
        // ~0) or the curve pair returns `Unresolved` for it; it must never be
        // silently discarded, and it must never be admitted as
        // `SourceStartEndpoint` because the line start is not exactly on the
        // circle.
        //
        // Construct: unit circle at origin, line from (0, next_down(-1.0)) to
        // (1, 1). `next_down(-1.0)` is the largest f64 strictly below -1, so
        // the line start is certifiably off the circle; an offset below half
        // an ulp of 1.0 would round to exactly -1.0 and silently place the
        // start on the circle, defeating the test. The near root sits at
        // s ~ 0.5 ulp (arc parameter 3π/2, a piece boundary), the far root at
        // s ~ 0.8 (interior of both pieces).
        let line = line_piece(
            Point2::new(0.0, (-1.0_f64).next_down()),
            Point2::new(1.0, 1.0),
        );
        let arc = arc_pieces(Point2::new(0.0, 0.0), 1.0, 0.0, 2.0 * PI, 0);
        let mut near_root_represented = false;
        let mut near_root_unresolved = false;
        for piece in &arc {
            match intersect(&line, piece) {
                PairIntersectionResult::Intersections(pts) => {
                    for rec in &pts {
                        assert_ne!(
                            rec.lhs_location,
                            ParameterLocation::SourceStartEndpoint,
                            "a line start not exactly on the circle is never \
                             admitted as SourceStartEndpoint"
                        );
                        if rec.lhs_parameter.lo <= 0.05 {
                            near_root_represented = true;
                        }
                    }
                }
                PairIntersectionResult::Unresolved(_) => near_root_unresolved = true,
                PairIntersectionResult::Disjoint | PairIntersectionResult::Unsupported(_) => {}
            }
        }
        assert!(
            near_root_represented || near_root_unresolved,
            "the near-start root (s ~ 0.5 ulp) must be represented or the curve \
             pair Unresolved — a far-root-only resolution must not satisfy this"
        );
    }

    // -- certified 2π-copy lift and parameter membership (A7, GEN-001E) -------

    /// A fabricated x-monotone arc piece over the source interval `[t0, t1]`
    /// with authoritative source-vertex endpoints. Test-only: the piece is not
    /// produced by `make_x_monotone`; it exists to pin the certified lift and
    /// membership semantics directly.
    fn fabricated_piece(t0: f64, t1: f64) -> XMonotoneCircularArc2 {
        let source = DirectedCircularArc2 {
            center: Point2::new(0.0, 0.0),
            cos_basis: Vector2::new(1.0, 0.0),
            sin_basis: Vector2::new(0.0, 1.0),
            t0,
            t1,
            provenance: provenance(),
        };
        let start = ArcPieceEndpoint::SourceVertex {
            vertex_id: SourceVertexKey::ShellVertex(1),
            point: Point2::new(0.0, 0.0),
        };
        let end = ArcPieceEndpoint::SourceVertex {
            vertex_id: SourceVertexKey::ShellVertex(2),
            point: Point2::new(0.0, 0.0),
        };
        XMonotoneCircularArc2 {
            source,
            start,
            end,
            kind: MonotoneKind::StrictlyIncreasingX,
            identity: PieceIdentity {
                source_occurrence: provenance(),
                source_piece_index: 0,
                parameter_hint_interval: ClosedInterval { t0, t1 },
                decomposition_kind: DecompositionKind::WholeOccurrence,
            },
        }
    }

    fn lift(
        piece: &XMonotoneCircularArc2,
        u: CertifiedInterval,
        v: CertifiedInterval,
    ) -> Result<ArcParameterLift, PairUnresolved> {
        lift_arc_parameter(piece, u, v)
    }

    fn on_piece(result: Result<ArcParameterLift, PairUnresolved>) -> ParameterEnclosure {
        match result {
            Ok(ArcParameterLift::OnPiece(t)) => t,
            other => panic!("expected OnPiece, got {other:?}"),
        }
    }

    #[test]
    fn ascending_interior_root_is_lifted_and_membership_is_interior() {
        // The upper semicircle piece [0, π], ascending. A root at angle ≈ π/2.
        let piece = fabricated_piece(0.0, PI);
        let u = CertifiedInterval { lo: -0.1, hi: 0.1 };
        let v = CertifiedInterval { lo: 0.9, hi: 1.1 };
        let t = on_piece(lift(&piece, u, v));
        // The lift is the k = 0 copy, on the piece's [0, π] axis.
        assert!(t.lo > 1.0 && t.hi < 2.0, "lift ≈ π/2, got {t:?}");
        assert_eq!(
            arc_parameter_membership(&piece, &t),
            OrientLocation::Interior
        );
    }

    #[test]
    fn descending_interior_root_is_lifted_and_membership_is_interior() {
        // The same geometric piece traversed descending: source t1 < t0. The
        // parameter evidence is unchanged and the membership stays Interior —
        // direction is read from the authoritative source, not from a rounded
        // endpoint.
        let piece = fabricated_piece(PI, 0.0);
        let u = CertifiedInterval { lo: -0.1, hi: 0.1 };
        let v = CertifiedInterval { lo: 0.9, hi: 1.1 };
        let t = on_piece(lift(&piece, u, v));
        assert_eq!(
            arc_parameter_membership(&piece, &t),
            OrientLocation::Interior
        );
    }

    #[test]
    fn exterior_root_has_no_compatible_copy() {
        // Piece [0, π]; a root at angle ≈ 4.08 rad (1.3π) is off the piece on
        // every 2π copy.
        let piece = fabricated_piece(0.0, PI);
        let u = CertifiedInterval {
            lo: -0.55,
            hi: -0.45,
        };
        let v = CertifiedInterval {
            lo: -0.85,
            hi: -0.75,
        };
        match lift(&piece, u, v) {
            Ok(ArcParameterLift::OffPiece) => {}
            other => panic!("expected OffPiece, got {other:?}"),
        }
    }

    #[test]
    fn critical_endpoint_root_is_boundary_not_interior() {
        // A root whose certified enclosure overlaps the piece's start critical
        // (angle 0 on [0, π]): Boundary, so the caller attempts an endpoint
        // admission and otherwise reports Undecidable — never Interior.
        let piece = fabricated_piece(0.0, PI);
        let u = CertifiedInterval { lo: 0.9, hi: 1.0 };
        let v = CertifiedInterval { lo: -0.2, hi: 0.2 };
        let t = on_piece(lift(&piece, u, v));
        assert_eq!(
            arc_parameter_membership(&piece, &t),
            OrientLocation::Boundary
        );
    }

    #[test]
    fn boundary_overlap_root_is_boundary_not_interior() {
        // A root enclosure straddling the piece start: Boundary. The straddle
        // never invents membership.
        let piece = fabricated_piece(0.0, PI);
        let u = CertifiedInterval { lo: 0.9, hi: 1.0 };
        let v = CertifiedInterval { lo: -0.3, hi: 0.3 };
        let t = on_piece(lift(&piece, u, v));
        assert_eq!(
            arc_parameter_membership(&piece, &t),
            OrientLocation::Boundary
        );
    }

    #[test]
    fn near_boundary_root_that_cannot_certify_interior_is_boundary_or_unresolved() {
        // The unit-level counterpart of `near_start_root_...`: a root enclosure
        // that just touches the start boundary cannot certify Interior and is
        // Boundary (the pair-level lift then reports Undecidable/Unresolved).
        let piece = fabricated_piece(0.0, PI);
        let u = CertifiedInterval { lo: 0.999, hi: 1.0 };
        let v = CertifiedInterval {
            lo: -0.001,
            hi: 0.001,
        };
        let t = on_piece(lift(&piece, u, v));
        assert_eq!(
            arc_parameter_membership(&piece, &t),
            OrientLocation::Boundary
        );
    }

    #[test]
    fn seam_crossing_root_lifts_onto_the_source_axis() {
        // A seam-crossing piece spanning [2π, 3π]. A root at principal angle
        // ≈ 0.6 rad is lifted to the k = 1 copy ≈ 2π + 0.6, on the piece's
        // unwrapped axis — never wrapped back to the principal angle.
        let piece = fabricated_piece(TAU, 3.0 * PI);
        let u = CertifiedInterval { lo: 0.8, hi: 0.9 };
        let v = CertifiedInterval { lo: 0.5, hi: 0.7 };
        let t = on_piece(lift(&piece, u, v));
        assert!(
            t.lo >= TAU - 1e-9 && t.hi <= 3.0 * PI + 1e-9,
            "the lift must lie on the [2π, 3π] source axis, got {t:?}"
        );
        assert!(
            t.contains(TAU + 0.6),
            "lift must contain the unwrapped 2π + 0.6"
        );
        // And membership on that axis is Interior for this interior root.
        assert_eq!(
            arc_parameter_membership(&piece, &t),
            OrientLocation::Interior
        );
    }

    #[test]
    fn midpoint_nearest_copy_is_not_accepted_unless_uniquely_certified() {
        // The A7 regression: the previous lift rounded `(center - a_lo)/2π`
        // to the nearest integer, i.e. selected the copy nearest the *midpoint*
        // of a reference window. Here a wide authoritative piece [0, 5π/2] is
        // compatible with **two** 2π copies of the root's angle enclosure
        // (k = 0 and k = 1 both intersect it), so no copy is uniquely
        // certified: the certified lift is `Unresolved`, never a midpoint pick.
        // The old midpoint would have selected k = round(1.25π/2π) = 1 and
        // silently minted a lift.
        let piece = fabricated_piece(0.0, 5.0 * PI / 2.0);
        let u = CertifiedInterval { lo: 0.9, hi: 1.0 };
        let v = CertifiedInterval { lo: -0.1, hi: 0.1 };
        match lift(&piece, u, v) {
            Err(PairUnresolved::ParameterLocationUndecided) => {}
            other => panic!("ambiguous copy must be Unresolved, got {other:?}"),
        }
    }

    #[test]
    fn unique_copy_is_lifted_even_when_a_midpoint_would_not_be_nearest() {
        // A seam-crossing piece [2π, 3π] with a root whose angle enclosure
        // extends below the piece start: the certified lift selects the unique
        // compatible copy (k = 1, containing 2π) on the piece axis. The
        // parameter fed to the membership test is therefore on the same
        // unwrapped axis as the piece endpoints.
        let piece = fabricated_piece(TAU, 3.0 * PI);
        // Principal angle enclosure straddling angle 0 (i.e. the piece's 2π
        // boundary in unwrapped terms).
        let u = CertifiedInterval { lo: 0.9, hi: 1.0 };
        let v = CertifiedInterval { lo: -0.2, hi: 0.2 };
        let t = on_piece(lift(&piece, u, v));
        assert!(
            t.lo <= TAU && TAU <= t.hi,
            "the unique lift must contain the piece start 2π, got {t:?}"
        );
        // It is Boundary at that endpoint (never Interior), and the enclosure
        // is on the [2π, 3π] axis.
        assert_eq!(
            arc_parameter_membership(&piece, &t),
            OrientLocation::Boundary
        );
    }

    #[test]
    fn seam_crossing_arc_root_parameter_is_recorded_on_the_source_axis() {
        // Integration: a full-turn arc [0, 2π] crossed by a line at
        // y = sin(2π − 0.2). One root is at the unwrapped parameter 2π − 0.2,
        // near the 2π seam. Its recorded parameter enclosure must lie on the
        // source axis (π, 2π] — never wrapped to −0.2.
        let line = line_piece(
            Point2::new(-2.0, -0.2_f64.sin()),
            Point2::new(2.0, -0.2_f64.sin()),
        );
        let arc = arc_pieces(Point2::new(0.0, 0.0), 1.0, 0.0, 2.0 * PI, 0);
        let mut saw_seam_root = false;
        for piece in &arc {
            match intersect(&line, piece) {
                PairIntersectionResult::Intersections(pts) => {
                    for rec in &pts {
                        // The root at (cos(-0.2), sin(-0.2)) ≈ (0.98, -0.20).
                        if rec.point.x > 0.9 && rec.point.y < -0.19 {
                            assert!(
                                rec.rhs_parameter.lo > PI,
                                "the seam root parameter must be on (π, 2π], got {:?}",
                                rec.rhs_parameter
                            );
                            assert!(
                                rec.rhs_parameter.contains(2.0 * PI - 0.2),
                                "the seam root parameter must contain 2π − 0.2, got {:?}",
                                rec.rhs_parameter
                            );
                            saw_seam_root = true;
                        }
                    }
                }
                _ => {}
            }
        }
        assert!(
            saw_seam_root,
            "the seam-crossing root at 2π − 0.2 must be recorded"
        );
    }
}
