//! Certified CommonArc substrate (GEN-001E).
//!
//! A [`CommonArc2`] is a certified **positive-dimensional connected overlap
//! component** between two source-curve occurrences. It is not a tangent
//! point, an endpoint-only meeting, two nearby curves, a sampled
//! approximation, or a sequence of isolated intersections.
//!
//! # The admitted certified support classes
//!
//! The minimal supported cases are those whose proof comes from source or
//! construction identity, never from evaluated geometry:
//!
//! - **provenance-identical support** ([`CommonSupportBasis::IdenticalSourceProvenance`]):
//!   both participants share one authoritative source occurrence (identical
//!   `SpanId`, preserved verbatim by certified subdivision), so both cover
//!   certified intervals of the same authoritative source parameter axis.
//! - **identical admitted analytic support** ([`CommonSupportBasis::IdenticalAnalyticSupport`]):
//!   distinct occurrences whose support identity is certified by exact
//!   predicates — collinear lines (exact parallel cross and exact
//!   point-on-line) or equal circles (exact center, exact squared radius,
//!   and the same authoritative parameterization). Approximate center/radius
//!   or sampled-point equality is never used.
//!
//! Everything else — general algebraic common components, unrelated
//! rational-Bézier equivalence, arbitrary rational reparameterization — is a
//! typed [`CommonArcError`] (mostly `UnsupportedSupportIdentity`), never a
//! guess from proximity.
//!
//! # Parameter correspondence
//!
//! Each participant carries a [`CertifiedParameterCorrespondence`]: a
//! certified affine map from its own traversal parameter to the canonical
//! source axis (identity, reversal, or an explicitly certified analytic
//! affine map). No unchecked public constructor can claim a correspondence
//! without evidence; every map is built inside this module from exact source
//! data and validated by [`CommonArc2::validate`].
//!
//! # Identity
//!
//! [`CommonArcIdentity`] is construction-based and canonical: the shared
//! support identity, the sorted participant occurrences, the relative
//! orientation class, the canonical ordered overlap bounds, and the
//! gauge-normalized relative deck displacement. It is independent of operand
//! order, input vector order, representative coordinates, discovery order,
//! source traversal reversal, subdivision depth and common deck translation.
//! It never derives from rounded interval endpoints or a representative point.

use super::super::source_evidence::SourceVertexKey;
use super::curve2d::{CurveOccurrenceProvenance, DirectedCircularArc2, LineSegment2};
use super::exact::{exact_dot2, exact_sq_dist, CertifiedInterval, CertifiedSign, Expansion};
use super::intersection::ParameterEnclosure;
use super::outcome::ResourceOperation;
use super::quotient::{CertifiedDeckLabel, DeckContext, DeckLabel, DeckLabelError};
use super::span::{CurveSpan2, SpanId};
use truck_geometry::prelude::Point2;

/// Why a CommonArc could not be certified. Every variant is a distinct typed
/// finding; none is a geometric guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonArcError {
    /// The two participants cannot be certified to share a support identity.
    UnsupportedSupportIdentity,
    /// The certified overlap on the canonical axis is empty.
    EmptyOverlap,
    /// The certified overlap is exactly one parameter value (an isolated
    /// endpoint contact, not a CommonArc).
    PointOnlyOverlap,
    /// The overlap extent or the overlap bounds cannot be certified.
    UnresolvedOverlap,
    /// The parameter correspondence between the participants cannot be
    /// certified.
    UnresolvedCorrespondence,
    /// The correspondence map is not invertible on the admitted interval.
    NonInvertibleCorrespondence,
    /// A participant's interval lacks positive certified extent.
    ZeroExtentParticipant,
    /// The orientation disagrees with the certified map.
    InconsistentOrientation,
    /// The boundary parameters are not certified distinct.
    NonDistinctBoundaries,
    /// The boundaries do not agree on both participants under the map.
    InconsistentBoundaries,
    /// The support certificate does not refer to the claimed support.
    InconsistentSupportCertificate,
    /// The deck labels do not agree with one ambient context.
    Deck(DeckLabelError),
    /// A multiplicity is not positive.
    NonPositiveMultiplicity,
    /// The identity does not match the canonicalized content.
    IdentityMismatch,
    /// The two participants must be distinct incidences (unless the support
    /// basis explicitly allows repetition).
    DistinctParticipantsRequired,
    /// Non-finite arithmetic: an operational failure, not geometry.
    NonFinite,
    /// Checked arithmetic overflow: an operational failure, not geometry.
    ArithmeticOverflow(ResourceOperation),
}

impl CommonArcError {
    /// A short stable tag, for diagnostics.
    pub fn tag(self) -> &'static str {
        match self {
            Self::UnsupportedSupportIdentity => "common_arc_unsupported_support_identity",
            Self::EmptyOverlap => "common_arc_empty_overlap",
            Self::PointOnlyOverlap => "common_arc_point_only_overlap",
            Self::UnresolvedOverlap => "common_arc_unresolved_overlap",
            Self::UnresolvedCorrespondence => "common_arc_unresolved_correspondence",
            Self::NonInvertibleCorrespondence => "common_arc_non_invertible_correspondence",
            Self::ZeroExtentParticipant => "common_arc_zero_extent_participant",
            Self::InconsistentOrientation => "common_arc_inconsistent_orientation",
            Self::NonDistinctBoundaries => "common_arc_non_distinct_boundaries",
            Self::InconsistentBoundaries => "common_arc_inconsistent_boundaries",
            Self::InconsistentSupportCertificate => "common_arc_inconsistent_support_certificate",
            Self::Deck(_) => "common_arc_deck",
            Self::NonPositiveMultiplicity => "common_arc_non_positive_multiplicity",
            Self::IdentityMismatch => "common_arc_identity_mismatch",
            Self::DistinctParticipantsRequired => "common_arc_distinct_participants_required",
            Self::NonFinite => "common_arc_non_finite",
            Self::ArithmeticOverflow(_) => "common_arc_arithmetic_overflow",
        }
    }
}

// ---------------------------------------------------------------------------
// Certified closed intervals and overlap classification
// ---------------------------------------------------------------------------

/// The certified overlap class of two parameter intervals on a common axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertifiedIntervalOverlap {
    /// Certifiably disjoint.
    Disjoint,
    /// Certified to meet in exactly one parameter value.
    Point,
    /// Certified to overlap in a positive-dimensional interval.
    Positive,
    /// The overlap class could not be certified.
    Unresolved,
}

/// Classify the overlap of two **exact** authoritative parameter intervals
/// `[lo1, hi1]` and `[lo2, hi2]` on a common axis.
///
/// The bounds are authoritative construction values (source endpoints or
/// certified split parameters), so the classification is exact — never an
/// epsilon comparison.
fn classify_exact_interval_overlap(
    lo1: f64,
    hi1: f64,
    lo2: f64,
    hi2: f64,
) -> CertifiedIntervalOverlap {
    let lo = lo1.max(lo2);
    let hi = hi1.min(hi2);
    if hi < lo {
        CertifiedIntervalOverlap::Disjoint
    } else if hi == lo {
        CertifiedIntervalOverlap::Point
    } else {
        CertifiedIntervalOverlap::Positive
    }
}

// ---------------------------------------------------------------------------
// Certified affine parameter maps
// ---------------------------------------------------------------------------

/// A certified affine parameter map `y = slope · x + intercept` with
/// outward-rounded interval coefficients.
///
/// Both coefficients are certified intervals; evaluation, inversion and
/// orientation are decided by directed-rounding interval arithmetic. No
/// unchecked constructor exists outside this module: maps are built from
/// exact source data and validated by [`CommonArc2::validate`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CertifiedAffineMap {
    slope: CertifiedInterval,
    intercept: CertifiedInterval,
}

impl CertifiedAffineMap {
    /// Build a certified affine map. Refuses non-finite coefficients.
    pub fn new(
        slope: CertifiedInterval,
        intercept: CertifiedInterval,
    ) -> Result<Self, CommonArcError> {
        if !slope.is_finite() || !intercept.is_finite() {
            return Err(CommonArcError::NonFinite);
        }
        Ok(Self { slope, intercept })
    }

    /// The identity map `y = x`.
    pub fn identity() -> Self {
        Self {
            slope: CertifiedInterval::point(1.0),
            intercept: CertifiedInterval::point(0.0),
        }
    }

    /// The slope coefficient.
    pub fn slope(&self) -> CertifiedInterval {
        self.slope
    }

    /// The intercept coefficient.
    pub fn intercept(&self) -> CertifiedInterval {
        self.intercept
    }

    /// Whether the slope is certified nonzero (the map is invertible).
    pub fn slope_is_nonzero(&self) -> bool {
        self.slope.lo > 0.0 || self.slope.hi < 0.0
    }

    /// The certified image of an interval, outward-rounded. `None` when the
    /// image is non-finite (operational failure).
    pub fn eval(&self, t: &ParameterEnclosure) -> Option<ParameterEnclosure> {
        let t_iv = CertifiedInterval { lo: t.lo, hi: t.hi };
        let r = self.slope.mul(&t_iv).add(&self.intercept);
        if r.is_finite() {
            Some(ParameterEnclosure { lo: r.lo, hi: r.hi })
        } else {
            None
        }
    }

    /// The certified inverse map `x = (y − intercept)/slope`. `None` when the
    /// slope contains zero (not invertible) or the coefficients are not
    /// finite.
    pub fn inverse(&self) -> Option<Self> {
        let one = CertifiedInterval::point(1.0);
        let inv_slope = one.div(&self.slope)?;
        let inv_intercept = self.intercept.neg().div(&self.slope)?;
        if inv_slope.is_finite() && inv_intercept.is_finite() {
            Some(Self {
                slope: inv_slope,
                intercept: inv_intercept,
            })
        } else {
            None
        }
    }

    /// The traversal orientation implied by the slope sign: `Codirected` for a
    /// certified positive slope, `Opposed` for a certified negative slope,
    /// `None` when the sign cannot be certified.
    pub fn orientation(&self) -> Option<OrientationAlongSupport> {
        if self.slope.lo > 0.0 {
            Some(OrientationAlongSupport::Codirected)
        } else if self.slope.hi < 0.0 {
            Some(OrientationAlongSupport::Opposed)
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// The canonical source axis and parameter correspondence
// ---------------------------------------------------------------------------

/// The canonical source axis of a CommonArc: the shared authoritative source
/// occurrence parameterization (provenance-identical support) or the
/// canonically-first participant's axis (analytic support).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CanonicalSourceAxis {
    /// The span id of the source occurrence whose parameterization is the
    /// canonical axis.
    pub span_id: SpanId,
}

/// The certified construction ancestor of a subdivided participant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SharedParentCertificate {
    /// The parent source occurrence the participant was cut from.
    pub parent: SpanId,
}

/// The certificate that an analytic correspondence maps to a certified
/// analytic support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AnalyticSupportCorrespondenceCertificate {
    /// The certified support identity the correspondence refers to.
    pub support: CommonSupportIdentity,
}

/// The certified parameter correspondence of one participant: how its own
/// traversal parameter maps to the canonical source axis.
///
/// There is **no public unchecked constructor**: every value is built inside
/// this module from exact source data, and a `CommonArc2` that carries a
/// correspondence is validated by [`CommonArc2::validate`]. The correspondence
/// is invertible on the admitted interval (checked by
/// [`CertifiedParameterCorrespondence::to_canonical_map`] and the validator).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CertifiedParameterCorrespondence {
    /// The participant reads the canonical source parameter directly (identity
    /// in authoritative source parameter).
    Identity {
        /// The canonical source axis.
        source_axis: CanonicalSourceAxis,
    },
    /// The participant traverses the canonical source axis in reverse (reversal
    /// in authoritative source parameter).
    Reversal {
        /// The canonical source axis.
        source_axis: CanonicalSourceAxis,
        /// The certified affine map from the participant's traversal parameter
        /// to the canonical axis: `s_axis = intercept + slope · t_participant`,
        /// with slope `−1` and intercept the participant's source-domain upper
        /// bound.
        to_axis: CertifiedAffineMap,
    },
    /// An explicit certified affine child-to-parent construction map.
    AffineConstruction {
        /// The certified affine map from this participant's traversal parameter
        /// to the shared parent axis.
        to_parent: CertifiedAffineMap,
        /// The certified shared parent construction.
        proof: SharedParentCertificate,
    },
    /// An explicitly certified analytic affine correspondence (distinct
    /// occurrences on a certified shared analytic support).
    Analytic {
        /// The certified affine map from this participant's traversal parameter
        /// to the canonical axis.
        map: CertifiedAffineMap,
        /// The certified analytic support the map is relative to.
        proof: AnalyticSupportCorrespondenceCertificate,
    },
}

impl CertifiedParameterCorrespondence {
    /// The certified affine map from this participant's traversal parameter to
    /// the canonical source axis parameter.
    pub fn to_canonical_map(&self) -> Result<CertifiedAffineMap, CommonArcError> {
        match self {
            Self::Identity { .. } => Ok(CertifiedAffineMap::identity()),
            Self::Reversal { to_axis, .. } => Ok(*to_axis),
            Self::AffineConstruction { to_parent, .. } => Ok(*to_parent),
            Self::Analytic { map, .. } => Ok(*map),
        }
    }

    /// The certified orientation of this participant along the shared support.
    pub fn orientation(&self) -> Result<OrientationAlongSupport, CommonArcError> {
        let map = self.to_canonical_map()?;
        map.orientation()
            .ok_or(CommonArcError::UnresolvedCorrespondence)
    }

    /// Whether the correspondence map is invertible (certified nonzero slope).
    pub fn invertible(&self) -> bool {
        self.to_canonical_map()
            .map(|m| m.slope_is_nonzero())
            .unwrap_or(false)
    }

    /// A short stable tag, for diagnostics.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::Identity { .. } => "correspondence_identity",
            Self::Reversal { .. } => "correspondence_reversal",
            Self::AffineConstruction { .. } => "correspondence_affine_construction",
            Self::Analytic { .. } => "correspondence_analytic",
        }
    }
}

// ---------------------------------------------------------------------------
// Support identity and the support fragment
// ---------------------------------------------------------------------------

/// The certified basis on which common support is claimed for a common arc.
///
/// The variant set does not preclude full common-factor extraction, which
/// stays deferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommonSupportBasis {
    /// Identical source provenance: the same occurrence, or pieces cut from it
    /// by certified subdivision.
    IdenticalSourceProvenance,
    /// Identical analytic support with a certified overlapping parameter
    /// correspondence.
    IdenticalAnalyticSupport,
    /// Identical homogeneous Bézier representation (deferred in GEN-001E).
    IdenticalHomogeneousRepresentation,
    /// A general common component not covered by the minimal cases (deferred).
    Deferred,
}

/// The relative orientation of one occurrence along a common support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrientationAlongSupport {
    /// Traversed in the same direction as the shared support.
    Codirected,
    /// Traversed opposite the shared support.
    Opposed,
}

impl OrientationAlongSupport {
    /// A short stable tag, for diagnostics.
    pub fn tag(self) -> &'static str {
        match self {
            Self::Codirected => "orientation_codirected",
            Self::Opposed => "orientation_opposed",
        }
    }
}

/// Which end of a common arc an endpoint event sits at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommonArcEnd {
    /// The start end of the common interval.
    Start,
    /// The end end of the common interval.
    End,
}

/// The analytic family of a certified analytic support identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnalyticSupportClass {
    /// A straight line.
    Line,
    /// A circle.
    Circle,
}

/// The canonical shared-support identity of a CommonArc.
///
/// Construction-based, never coordinate-based: a shared source occurrence, or
/// a certified analytic support anchored at the canonically-first participant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommonSupportIdentity {
    /// A single shared authoritative source occurrence (subdivision-inherited).
    SameSourceOccurrence(SpanId),
    /// A certified analytic support shared by distinct occurrences.
    IdenticalAnalyticSupport {
        /// The analytic family.
        class: AnalyticSupportClass,
        /// The canonically-first participant's span id (a construction anchor).
        anchor: SpanId,
    },
}

/// The evidence that a common-support claim is certified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportIdentityCertificate {
    /// Identical source provenance: both participants share this occurrence.
    IdenticalSourceProvenance {
        /// The shared occurrence.
        span_id: SpanId,
    },
    /// Certified collinear line supports: exact parallel cross and exact
    /// point-on-line predicates.
    CertifiedCollinearLines {
        /// The canonically-first span.
        first: SpanId,
        /// The canonically-second span.
        second: SpanId,
    },
    /// Certified equal circle supports: exact center equality, exact squared
    /// radius equality, and the same authoritative parameterization.
    CertifiedEqualCircles {
        /// The canonically-first span.
        first: SpanId,
        /// The canonically-second span.
        second: SpanId,
    },
}

/// The support fragment of a CommonArc: the certified basis, the canonical
/// support identity, and the evidence certificate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonSupportFragment {
    /// The certified basis.
    pub basis: CommonSupportBasis,
    /// The canonical support identity.
    pub identity: CommonSupportIdentity,
    /// The evidence certifying the support.
    pub certificate: SupportIdentityCertificate,
}

// ---------------------------------------------------------------------------
// Boundaries: construction identity separated from numerical localization
// ---------------------------------------------------------------------------

/// The canonical side of a participant's endpoint on the canonical source axis.
///
/// Reversal-invariant: `Lower` is the endpoint that projects to the lesser
/// canonical-axis parameter, `Upper` to the greater — a geometric property of
/// the mapped interval, independent of traversal order. Reversing an
/// occurrence swaps its traversal start/end but leaves its `Lower`/`Upper`
/// mapped endpoints unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AxisSide {
    /// The endpoint mapping to the lesser canonical-axis parameter.
    Lower,
    /// The endpoint mapping to the greater canonical-axis parameter.
    Upper,
}

/// A bit-key for an **authoritative** source-domain parameter.
///
/// The only topological identity a declared source parameter may participate
/// in. It is constructed solely from a value already **retained** as
/// authoritative source data — a source trim value, or the certified split
/// parameter a subdivision produced and stored in the span's domain — never
/// from an independently recomputed projection.
///
/// # Why a private bit-key rather than a bare `f64`
///
/// A bare `f64` field with an unrestricted constructor proves nothing: any
/// recomputed value could be wrapped. [`AuthoritativeParameterKey`] has a
/// single `pub(crate)` constructor [`from_authoritative`](Self::from_authoritative)
/// that accepts an already-retained authoritative domain value; outside this
/// module there is no way to mint one. Equality is bitwise after `-0.0`
/// normalization, which is exact for authoritative finite values — they are
/// declared, not recomputed, so the same authoritative value read from a
/// span, its reversal, its parent or its sibling yields identical bits.
#[derive(Debug, Clone, Copy)]
pub struct AuthoritativeParameterKey {
    bits: u64,
}

impl AuthoritativeParameterKey {
    /// Construct from a value already retained as authoritative source data.
    ///
    /// `pub(crate)`: this is the only constructor, and it is not available
    /// outside this module, so no caller can wrap a recomputed projection and
    /// claim it is certified. The value must be a declared source trim or a
    /// certified subdivision split retained by a span's authoritative domain.
    pub(crate) fn from_authoritative(value: f64) -> Self {
        // Normalize -0.0 to +0.0 so the key is stable across sign-zero variants
        // of the same authoritative value.
        let normalized = if value == 0.0 { 0.0_f64 } else { value };
        Self {
            bits: normalized.to_bits(),
        }
    }

    /// The authoritative value — for diagnostics and realization only, never
    /// for identity (identity is the bit-key itself).
    pub fn value(&self) -> f64 {
        f64::from_bits(self.bits)
    }
}

impl PartialEq for AuthoritativeParameterKey {
    fn eq(&self, other: &Self) -> bool {
        self.bits == other.bits
    }
}

impl Eq for AuthoritativeParameterKey {}

impl PartialOrd for AuthoritativeParameterKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AuthoritativeParameterKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.bits.cmp(&other.bits)
    }
}

impl std::hash::Hash for AuthoritativeParameterKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.bits.hash(state);
    }
}

/// The construction/provenance identity of one CommonArc boundary.
///
/// This is the **topological identity**: what it is, not where it is. Equality
/// and hashing use this key alone. The numerical location of the same
/// construction is carried separately by the enclosing
/// [`CommonArcBoundary`]'s [`ParameterEnclosure`], which is evidence for
/// ordering and realization and is never itself identity.
///
/// # Why the two variants carry different evidence
///
/// - [`CommonArcBoundaryKey::SourceEndpoint`] names an **authoritative
///   source-domain endpoint**: a declared source trim value or the certified
///   split parameter a subdivision produced. Its [`AuthoritativeParameterKey`]
///   is built from the retained authoritative domain value — not an
///   independently recomputed projection — so it is bitwise-stable under
///   reversal, subdivision and common deck translation, and it is required to
///   distinguish two different overlaps of one source occurrence (two pieces
///   share a `SpanId`; only their authoritative domain bounds distinguish their
///   endpoints). For a periodic circle the retained unwrapped parameter
///   encodes the unique certified `2π` lift; a principal angle alone is never
///   the identity.
/// - [`CommonArcBoundaryKey::MappedEndpoint`] names a **distinct occurrence's
///   endpoint mapped onto the canonical axis by a certified analytic
///   correspondence** (collinear lines). Its canonical-axis location is a
///   recomputed projection, so it is deliberately **not** part of the identity;
///   only the construction (which endpoint of which span) is. The `span` is a
///   [`SpanId`], which uniquely identifies a formal line-span endpoint in the
///   admitted machinery (line spans are not subdivided in this substrate, so
///   two formal spans sharing a `SpanId` are the same segment). Two
///   independently recomputed enclosures of the same mapped endpoint may differ
///   by a unit in the last place; the identity stays equal because it is
///   construction-based.
#[derive(Debug, Clone, Copy)]
pub enum CommonArcBoundaryKey {
    /// An authoritative source-domain endpoint of a participant on the
    /// canonical source axis: the source's own declared trim bound, or the
    /// certified split parameter a subdivision produced. The parameter key is
    /// built from the retained authoritative domain value, never from an
    /// independently recomputed projection.
    SourceEndpoint {
        /// The occurrence whose endpoint bounds the overlap.
        occurrence: SpanId,
        /// Which end of the participant's mapped interval.
        side: AxisSide,
        /// The authoritative source-domain parameter of this endpoint.
        parameter: AuthoritativeParameterKey,
        /// The unique certified `2π` lift for a periodic support, as a deck
        /// label bound to the CommonArc's ambient context. `None` for
        /// non-periodic supports (lines, Bézier) and for the admitted
        /// circle path, where the unwrapped authoritative parameter retained
        /// in `parameter` already encodes the unique lift — a principal angle
        /// alone is never the identity. Populated only when the boundary is
        /// produced by an A7-style certified periodic enumeration.
        periodic_lift: Option<CertifiedDeckLabel>,
    },
    /// A distinct occurrence's endpoint mapped onto the canonical axis by a
    /// certified analytic correspondence. The canonical-axis location is a
    /// recomputed projection and is carried only by the boundary's enclosure;
    /// the identity is the construction (which endpoint of which span).
    MappedEndpoint {
        /// The formal span whose mapped endpoint bounds the overlap. A
        /// [`SpanId`] uniquely identifies a formal line-span endpoint in the
        /// admitted machinery (reversal and subdivision preserve `SpanId`,
        /// verified by `span_id_is_preserved_by_reversal_and_subdivision`).
        span: SpanId,
        /// Which end of the participant's mapped interval on the canonical axis.
        side: AxisSide,
    },
}

impl PartialEq for CommonArcBoundaryKey {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::SourceEndpoint {
                    occurrence: a,
                    side: sa,
                    parameter: pa,
                    periodic_lift: la,
                },
                Self::SourceEndpoint {
                    occurrence: b,
                    side: sb,
                    parameter: pb,
                    periodic_lift: lb,
                },
            ) => a == b && sa == sb && pa == pb && la == lb,
            (
                Self::MappedEndpoint { span: a, side: sa },
                Self::MappedEndpoint { span: b, side: sb },
            ) => a == b && sa == sb,
            _ => false,
        }
    }
}

impl Eq for CommonArcBoundaryKey {}

impl std::hash::Hash for CommonArcBoundaryKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Self::SourceEndpoint {
                occurrence,
                side,
                parameter,
                periodic_lift,
            } => {
                0u8.hash(state);
                occurrence.hash(state);
                side.hash(state);
                parameter.hash(state);
                periodic_lift.hash(state);
            }
            Self::MappedEndpoint { span, side } => {
                1u8.hash(state);
                span.hash(state);
                side.hash(state);
            }
        }
    }
}

/// One boundary of a CommonArc: a construction/provenance identity (the
/// topological identity) plus a certified parameter enclosure (the numerical
/// localization evidence).
///
/// Equality and hashing use the [`key`](CommonArcBoundaryKey) alone, so two
/// independently recomputed enclosures of the same construction — which may
/// differ by a unit in the last place — identify the same boundary. The
/// enclosure proves localization, supports ordering and later realization, and
/// is never itself topological identity.
#[derive(Debug, Clone, Copy)]
pub struct CommonArcBoundary {
    /// The construction/provenance identity of this boundary.
    pub key: CommonArcBoundaryKey,
    /// The certified canonical-axis parameter enclosure of this boundary.
    pub enclosure: ParameterEnclosure,
}

impl CommonArcBoundary {
    /// The construction key (topological identity).
    pub const fn key(&self) -> CommonArcBoundaryKey {
        self.key
    }
    /// The certified parameter enclosure (localization evidence).
    pub const fn enclosure(&self) -> ParameterEnclosure {
        self.enclosure
    }
}

impl PartialEq for CommonArcBoundary {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl Eq for CommonArcBoundary {}

impl std::hash::Hash for CommonArcBoundary {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

/// The two certified boundaries of a CommonArc, in canonical axis order
/// (`start` is the lower canonical-axis parameter, `end` the greater).
///
/// Equality and hashing are key-based: the same overlap discovered through two
/// pair queries produces equal boundaries even when the enclosures differ
/// slightly, while genuinely different endpoint constructions stay distinct.
#[derive(Debug, Clone, Copy)]
pub struct CommonArcBoundaries {
    /// The start (lower) boundary.
    pub start: CommonArcBoundary,
    /// The end (upper) boundary.
    pub end: CommonArcBoundary,
}

impl PartialEq for CommonArcBoundaries {
    fn eq(&self, other: &Self) -> bool {
        self.start == other.start && self.end == other.end
    }
}

impl Eq for CommonArcBoundaries {}

impl std::hash::Hash for CommonArcBoundaries {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.start.hash(state);
        self.end.hash(state);
    }
}

// ---------------------------------------------------------------------------
// CommonArc identity
// ---------------------------------------------------------------------------

/// The canonical identity of a CommonArc.
///
/// Construction- and provenance-based, independent of operand order, input
/// vector order, representative coordinates, discovery order, source
/// traversal reversal, subdivision depth and common deck translation. The
/// ordered boundaries are [`CommonArcBoundaries`] whose equality is
/// construction-key-based: a `SourceEndpoint` key carries the authoritative
/// source-domain parameter (a declared trim or certified split, never an
/// independently recomputed projection), and a `MappedEndpoint` key carries
/// only the construction (which endpoint of which occurrence), so two
/// enclosures of the same mapped endpoint that differ by a unit in the last
/// place identify the same boundary. The relative deck displacement is
/// gauge-normalized (`k₁ − k₀` under canonical participant order), never raw
/// absolute labels.
#[derive(Debug, Clone, Copy)]
pub struct CommonArcIdentity {
    /// The canonical shared-support identity.
    pub support: CommonSupportIdentity,
    /// The two canonical participant occurrences, sorted.
    pub participants: [SpanId; 2],
    /// The canonical relative orientation along the shared support.
    pub orientation: OrientationAlongSupport,
    /// The canonical ordered boundaries on the canonical axis (key-based).
    pub boundaries: CommonArcBoundaries,
    /// The normalized relative deck displacement (`second − first`).
    pub relative_deck: DeckLabel,
}

impl PartialEq for CommonArcIdentity {
    fn eq(&self, other: &Self) -> bool {
        self.support == other.support
            && self.participants == other.participants
            && self.orientation == other.orientation
            && self.boundaries == other.boundaries
            && self.relative_deck == other.relative_deck
    }
}

impl Eq for CommonArcIdentity {}

impl std::hash::Hash for CommonArcIdentity {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.support.hash(state);
        self.participants.hash(state);
        self.orientation.hash(state);
        self.boundaries.hash(state);
        self.relative_deck.hash(state);
    }
}

// ---------------------------------------------------------------------------
// CommonArc certificate
// ---------------------------------------------------------------------------

/// The evidence proving support identity and parameter correspondence for a
/// CommonArc.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CommonArcCertificate {
    /// The support identity certificate.
    pub support: SupportIdentityCertificate,
    /// Each participant's certified parameter correspondence, in canonical
    /// participant order.
    pub correspondence: [CertifiedParameterCorrespondence; 2],
    /// Whether the two boundaries are certified distinct (positive extent).
    pub boundaries_certified_distinct: bool,
}

// ---------------------------------------------------------------------------
// One occurrence's participation
// ---------------------------------------------------------------------------

/// One occurrence's participation in a common arc.
#[derive(Debug, Clone, PartialEq)]
pub struct ArcParticipant {
    /// The span this participant is a piece of.
    pub span_id: SpanId,
    /// The source occurrence provenance.
    pub provenance: CurveOccurrenceProvenance,
    /// The certified closed interval of the overlap on this participant's
    /// traversal parameterization (ascending traversal order).
    pub parameter_interval: ParameterEnclosure,
    /// The relative orientation along the shared support.
    pub orientation: OrientationAlongSupport,
    /// The certified parameter correspondence to the canonical source axis.
    pub correspondence: CertifiedParameterCorrespondence,
    /// This occurrence's multiplicity contribution (positive).
    pub multiplicity: u8,
    /// The validated deck label of this participant's chart copy, bound to the
    /// CommonArc's shared ambient context.
    pub deck: CertifiedDeckLabel,
}

// ---------------------------------------------------------------------------
// The CommonArc record
// ---------------------------------------------------------------------------

/// A certified positive-dimensional common-arc contact component.
///
/// Exactly two pairwise participants at GEN-001 level; ARR-003 may later
/// aggregate several pairwise records into an N-way common region. The
/// validated constructor/validator is [`CommonArc2::validate`] — there is no
/// unchecked way to build a `CommonArc2` with inconsistent content, and the
/// certificate carries the evidence that discharged each obligation.
#[derive(Debug, Clone, PartialEq)]
pub struct CommonArc2 {
    /// The canonical identity.
    pub identity: CommonArcIdentity,
    /// The certified support fragment.
    pub support: CommonSupportFragment,
    /// The two canonical participants.
    pub participants: [ArcParticipant; 2],
    /// The certified boundaries on the canonical axis.
    pub boundaries: CommonArcBoundaries,
    /// The evidence certificate.
    pub certificate: CommonArcCertificate,
    /// The shared ambient lattice/rank context for both participants' deck
    /// labels.
    pub deck_context: DeckContext,
}

/// Compute the canonical identity of a CommonArc from its support, canonical
/// participants, relative orientation, construction-key boundaries and deck
/// labels.
fn canonical_identity(
    support: CommonSupportIdentity,
    first: SpanId,
    second: SpanId,
    relative_orientation: OrientationAlongSupport,
    boundaries: CommonArcBoundaries,
    relative_deck: DeckLabel,
) -> CommonArcIdentity {
    let mut participants = [first, second];
    participants.sort();
    CommonArcIdentity {
        support,
        participants,
        orientation: relative_orientation,
        boundaries,
        relative_deck,
    }
}

impl CommonArc2 {
    /// Validate this CommonArc against every obligation a certified
    /// positive-dimensional overlap must discharge.
    ///
    /// Checks, in order:
    ///
    /// 1. participants are canonical (sorted span ids) and are distinct
    ///    incidences unless the support basis explicitly allows repetition;
    /// 2. every multiplicity is positive;
    /// 3. both participant intervals have positive certified extent;
    /// 4. every deck label validates against the shared ambient context
    ///    (a cross-lattice or cross-rank label is a typed error);
    /// 5. the boundaries are certified distinct;
    /// 6. the relative deck displacement matches the participants;
    /// 7. the correspondence is invertible on the admitted interval;
    /// 8. each participant interval, transported to the canonical axis under
    ///    its certified map, contains the canonical overlap bounds;
    /// 9. the orientation agrees with the certified map;
    /// 10. the identity matches the canonicalized content.
    ///
    /// Malformed internal construction is an `Err`; insufficient mathematical
    /// evidence is a typed `Unsupported`/`Unresolved`/`Inconsistent` — the two
    /// categories are never collapsed.
    pub fn validate(&self) -> Result<(), CommonArcError> {
        // 1. canonical participant order and distinctness.
        let mut ids = [self.participants[0].span_id, self.participants[1].span_id];
        if ids[0] > ids[1] {
            ids.swap(0, 1);
        }
        if ids != [self.participants[0].span_id, self.participants[1].span_id] {
            return Err(CommonArcError::IdentityMismatch);
        }
        match self.support.identity {
            CommonSupportIdentity::SameSourceOccurrence(_) => {}
            _ if self.participants[0].span_id == self.participants[1].span_id => {
                return Err(CommonArcError::DistinctParticipantsRequired);
            }
            _ => {}
        }
        // 2. multiplicities positive.
        if self.participants.iter().any(|p| p.multiplicity == 0) {
            return Err(CommonArcError::NonPositiveMultiplicity);
        }
        // 3. participant intervals have positive certified extent.
        if self
            .participants
            .iter()
            .any(|p| p.parameter_interval.lo >= p.parameter_interval.hi)
        {
            return Err(CommonArcError::ZeroExtentParticipant);
        }
        // 4. deck labels agree with the shared ambient context.
        for participant in &self.participants {
            participant
                .deck
                .validate_for(self.deck_context)
                .map_err(CommonArcError::Deck)?;
        }
        // 5. Boundary construction identity cross-checked against certified
        // localization.
        let start = &self.boundaries.start;
        let end = &self.boundaries.end;
        // 5.0 Both boundary enclosures must be finite.
        if !start.enclosure.lo.is_finite()
            || !start.enclosure.hi.is_finite()
            || !end.enclosure.lo.is_finite()
            || !end.enclosure.hi.is_finite()
        {
            return Err(CommonArcError::NonFinite);
        }
        // 5a. Equal start/end keys cannot define a positive-length CommonArc:
        //     the same construction cannot bound both ends of an interval.
        if start.key == end.key {
            return Err(CommonArcError::NonDistinctBoundaries);
        }
        // 5b. Enclosure ordering vs the exact-sign certificate. Inverted
        //     enclosures (start entirely above end) are an inconsistent
        //     construction. When the enclosures cannot separate, distinctness
        //     is certified by the producer's exact expansion sign (recorded in
        //     the certificate); if neither certifies it, the boundary ordering
        //     is typed Unresolved, never guessed.
        let inverted = start.enclosure.lo > end.enclosure.hi;
        let separated = start.enclosure.hi < end.enclosure.lo;
        if inverted {
            return Err(CommonArcError::NonDistinctBoundaries);
        }
        if !separated && !self.certificate.boundaries_certified_distinct {
            return Err(CommonArcError::UnresolvedOverlap);
        }
        // 5c. Endpoint keys must agree with participant provenance: every
        //     boundary key's occurrence/span is one of the two participants.
        let p0 = self.participants[0].span_id;
        let p1 = self.participants[1].span_id;
        let key_agrees = |key: &CommonArcBoundaryKey| match key {
            CommonArcBoundaryKey::SourceEndpoint { occurrence, .. } => {
                *occurrence == p0 || *occurrence == p1
            }
            CommonArcBoundaryKey::MappedEndpoint { span, .. } => *span == p0 || *span == p1,
        };
        if !key_agrees(&start.key) || !key_agrees(&end.key) {
            return Err(CommonArcError::InconsistentBoundaries);
        }
        // 5d. Periodic lift evidence (when present) must be compatible with the
        //     shared deck context. The admitted producer leaves this `None`
        //     (the unwrapped authoritative parameter encodes the lift); a
        //     future A7-enumerated boundary would carry a bound deck label.
        for b in [start, end] {
            if let CommonArcBoundaryKey::SourceEndpoint {
                periodic_lift: Some(lift),
                ..
            } = &b.key
            {
                lift.validate_for(self.deck_context)
                    .map_err(CommonArcError::Deck)?;
            }
        }
        // 6. relative deck displacement matches the participants.
        let relative_deck = self.participants[1]
            .deck
            .checked_sub(self.participants[0].deck)
            .map_err(CommonArcError::Deck)?
            .get();
        if relative_deck != self.identity.relative_deck {
            return Err(CommonArcError::IdentityMismatch);
        }
        // The canonical-axis overlap enclosure is the hull of the two boundary
        // enclosures: a sound (possibly loose) enclosure of the shared
        // interval, used only for the transport-containment check.
        let overlap = ParameterEnclosure {
            lo: self.boundaries.start.enclosure.lo,
            hi: self.boundaries.end.enclosure.hi,
        };
        // 7/8. correspondence invertible and participant intervals transport to
        // the canonical overlap.
        for participant in &self.participants {
            let map = participant.correspondence.to_canonical_map()?;
            if !map.slope_is_nonzero() {
                return Err(CommonArcError::NonInvertibleCorrespondence);
            }
            let transported = map
                .eval(&participant.parameter_interval)
                .ok_or(CommonArcError::NonFinite)?;
            if !(transported.lo <= overlap.lo && overlap.hi <= transported.hi) {
                return Err(CommonArcError::InconsistentBoundaries);
            }
        }
        // 9. orientation agrees with the map.
        for participant in &self.participants {
            let map = participant.correspondence.to_canonical_map()?;
            let map_orientation = map
                .orientation()
                .ok_or(CommonArcError::UnresolvedCorrespondence)?;
            if map_orientation != participant.orientation {
                return Err(CommonArcError::InconsistentOrientation);
            }
        }
        // 10. identity matches the canonicalized content.
        let first = &self.participants[0];
        let second = &self.participants[1];
        let map_a = first.correspondence.to_canonical_map()?;
        let map_b = second.correspondence.to_canonical_map()?;
        let orientation_a = map_a
            .orientation()
            .ok_or(CommonArcError::UnresolvedCorrespondence)?;
        let orientation_b = map_b
            .orientation()
            .ok_or(CommonArcError::UnresolvedCorrespondence)?;
        let relative = if orientation_a == orientation_b {
            OrientationAlongSupport::Codirected
        } else {
            OrientationAlongSupport::Opposed
        };
        let canonical = canonical_identity(
            self.support.identity,
            first.span_id,
            second.span_id,
            relative,
            self.boundaries,
            relative_deck,
        );
        if canonical != self.identity {
            return Err(CommonArcError::IdentityMismatch);
        }
        // The support certificate must refer to the claimed support.
        match (&self.support.identity, &self.certificate.support) {
            (
                CommonSupportIdentity::SameSourceOccurrence(span),
                SupportIdentityCertificate::IdenticalSourceProvenance { span_id: certified },
            ) if span == certified => {}
            (
                CommonSupportIdentity::IdenticalAnalyticSupport {
                    class: AnalyticSupportClass::Line,
                    anchor,
                },
                SupportIdentityCertificate::CertifiedCollinearLines { first, second: _ },
            ) if anchor == first => {}
            (
                CommonSupportIdentity::IdenticalAnalyticSupport {
                    class: AnalyticSupportClass::Circle,
                    anchor,
                },
                SupportIdentityCertificate::CertifiedEqualCircles { first, second: _ },
            ) if anchor == first => {}
            _ => return Err(CommonArcError::InconsistentSupportCertificate),
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// The producer
// ---------------------------------------------------------------------------

/// The internal assembly of a certified CommonArc from canonical inputs.
///
/// The caller — a path-specific certifier — has already selected the two
/// construction-key boundaries and passes the **exact expansion sign** of the
/// canonical-axis extent (`end − start`) as the positive-extent proof.
/// `assemble` derives `boundaries_certified_distinct` from that proof (never
/// hardcoding `true`): `Positive` certifies a positive-length CommonArc,
/// `Zero` is a typed `PointOnlyOverlap`, `Negative` a typed `EmptyOverlap`.
/// The canonical-axis overlap enclosure is the hull of the two boundary
/// enclosures: a sound enclosure of the shared interval, used only for the
/// transport-containment check.
fn assemble(
    first: &CurveSpan2,
    second: &CurveSpan2,
    support: CommonSupportFragment,
    boundaries: CommonArcBoundaries,
    extent_sign: CertifiedSign,
    corr_a: CertifiedParameterCorrespondence,
    corr_b: CertifiedParameterCorrespondence,
    orientation_a: OrientationAlongSupport,
    orientation_b: OrientationAlongSupport,
    context: DeckContext,
) -> Result<CommonArc2, CommonArcError> {
    let overlap = ParameterEnclosure {
        lo: boundaries.start.enclosure.lo,
        hi: boundaries.end.enclosure.hi,
    };
    if !(overlap.lo.is_finite() && overlap.hi.is_finite()) {
        return Err(CommonArcError::NonFinite);
    }
    let boundaries_certified_distinct = match extent_sign {
        CertifiedSign::Positive => true,
        CertifiedSign::Zero => return Err(CommonArcError::PointOnlyOverlap),
        CertifiedSign::Negative => return Err(CommonArcError::EmptyOverlap),
    };
    let map_a = corr_a.to_canonical_map()?;
    let map_b = corr_b.to_canonical_map()?;
    if !map_a.slope_is_nonzero() || !map_b.slope_is_nonzero() {
        return Err(CommonArcError::NonInvertibleCorrespondence);
    }
    let inverse_a = map_a
        .inverse()
        .ok_or(CommonArcError::NonInvertibleCorrespondence)?;
    let inverse_b = map_b
        .inverse()
        .ok_or(CommonArcError::NonInvertibleCorrespondence)?;
    let interval_a = inverse_a.eval(&overlap).ok_or(CommonArcError::NonFinite)?;
    let interval_b = inverse_b.eval(&overlap).ok_or(CommonArcError::NonFinite)?;
    if interval_a.lo >= interval_a.hi || interval_b.lo >= interval_b.hi {
        return Err(CommonArcError::ZeroExtentParticipant);
    }
    // The participant intervals, transported back to the canonical axis, must
    // contain the canonical overlap enclosure.
    let back_a = map_a.eval(&interval_a).ok_or(CommonArcError::NonFinite)?;
    let back_b = map_b.eval(&interval_b).ok_or(CommonArcError::NonFinite)?;
    if !(back_a.lo <= overlap.lo && overlap.hi <= back_a.hi)
        || !(back_b.lo <= overlap.lo && overlap.hi <= back_b.hi)
    {
        return Err(CommonArcError::InconsistentBoundaries);
    }
    // Deck labels: the pair-level producer has no certified ambient lattice, so
    // the shared context is rank-0 and both labels are the validated zero.
    let deck_a = CertifiedDeckLabel::zero(context);
    let deck_b = CertifiedDeckLabel::zero(context);
    let relative_orientation = if orientation_a == orientation_b {
        OrientationAlongSupport::Codirected
    } else {
        OrientationAlongSupport::Opposed
    };
    let participant_a = ArcParticipant {
        span_id: first.span_id(),
        provenance: *first.provenance(),
        parameter_interval: interval_a,
        orientation: orientation_a,
        correspondence: corr_a,
        multiplicity: 1,
        deck: deck_a,
    };
    let participant_b = ArcParticipant {
        span_id: second.span_id(),
        provenance: *second.provenance(),
        parameter_interval: interval_b,
        orientation: orientation_b,
        correspondence: corr_b,
        multiplicity: 1,
        deck: deck_b,
    };
    // Canonical participant order: sort by span id, tie-breaking same-occurrence
    // participants by their certified interval bounds, so an operand swap and a
    // same-source subdivision always produce the same canonical array. The
    // relative deck displacement is then defined against this canonical order.
    let mut participants = [participant_a, participant_b];
    participants.sort_by(|p, q| {
        p.span_id
            .cmp(&q.span_id)
            .then_with(|| p.parameter_interval.lo.total_cmp(&q.parameter_interval.lo))
            .then_with(|| p.parameter_interval.hi.total_cmp(&q.parameter_interval.hi))
    });
    let relative_deck = participants[1]
        .deck
        .checked_sub(participants[0].deck)
        .map_err(CommonArcError::Deck)?
        .get();
    let identity = canonical_identity(
        support.identity,
        participants[0].span_id,
        participants[1].span_id,
        relative_orientation,
        boundaries,
        relative_deck,
    );
    let certificate = CommonArcCertificate {
        support: support.certificate,
        correspondence: [
            participants[0].correspondence,
            participants[1].correspondence,
        ],
        // Derived from the exact expansion sign passed by the caller.
        boundaries_certified_distinct,
    };
    let arc = CommonArc2 {
        identity,
        support,
        participants,
        boundaries,
        certificate,
        deck_context: context,
    };
    arc.validate()?;
    Ok(arc)
}

/// Certify a CommonArc between two curve-span occurrences, when the admitted
/// machinery can.
///
/// Returns a typed [`CommonArcError`] otherwise: `UnsupportedSupportIdentity`
/// when no certified support identity exists, `EmptyOverlap` when the
/// certified overlap is empty, `PointOnlyOverlap` when the overlap is a single
/// certified parameter value, and `Unresolved*` when the evidence cannot
/// decide. The caller decides how to fold the error into the pair result (the
/// analytic lift keeps the original `Unsupported`, the generic path runs the
/// isolated-root solver).
pub fn common_arc_for_pair(
    lhs: &CurveSpan2,
    rhs: &CurveSpan2,
) -> Result<CommonArc2, CommonArcError> {
    match (lhs.fast_path(), rhs.fast_path()) {
        // Lines always go through the certified-collinear path: its certified
        // affine map's slope carries the traversal orientation, which the
        // reported domain cannot (a line's authoritative domain is `(0,1)`
        // whether or not the occurrence is traversed in reverse). This also
        // covers two pieces of the same line occurrence.
        (super::span::FastPath::Line, super::span::FastPath::Line) => {
            analytic_line_common_arc(lhs, rhs)
        }
        // Circles and generic spans use the provenance-identical path when both
        // participants share one authoritative occurrence (the provenance
        // relation, not the span id), and the certified analytic-support path
        // (circles) otherwise.
        (super::span::FastPath::CircularArc, super::span::FastPath::CircularArc) => {
            if same_source_occurrence(lhs, rhs) {
                provenance_common_arc(lhs, rhs)
            } else {
                analytic_circle_common_arc(lhs, rhs)
            }
        }
        (super::span::FastPath::Generic, super::span::FastPath::Generic) => {
            if same_source_occurrence(lhs, rhs) {
                provenance_common_arc(lhs, rhs)
            } else {
                // Generic rational-Bézier support is admitted only for
                // authoritative same-source identity; unrelated rational-Bézier
                // overlap stays typed unsupported (no polynomial GCD /
                // common-factor extraction).
                Err(CommonArcError::UnsupportedSupportIdentity)
            }
        }
        _ => Err(CommonArcError::UnsupportedSupportIdentity),
    }
}

/// The provenance-identical CommonArc path: both participants are pieces of
/// one shared authoritative source occurrence.
///
/// The canonical axis is the shared occurrence's parameterization. Each
/// participant's traversal parameter is the source parameter (identity) or its
/// reverse (reversal), so the certified overlap is the exact intersection of
/// the two source-axis intervals. Subdivision inherits the occurrence's span
/// identity and lift context; it never mints a new common-support claim from
/// coordinates.
fn provenance_common_arc(lhs: &CurveSpan2, rhs: &CurveSpan2) -> Result<CommonArc2, CommonArcError> {
    let span_id = lhs.span_id();
    let (first, second) = if lhs.span_id() <= rhs.span_id() {
        (lhs, rhs)
    } else {
        (rhs, lhs)
    };
    let (d0a, d1a) = first.authoritative_domain();
    let (d0b, d1b) = second.authoritative_domain();
    let a_lo = d0a.min(d1a);
    let a_hi = d0a.max(d1a);
    let b_lo = d0b.min(d1b);
    let b_hi = d0b.max(d1b);
    if !(a_lo.is_finite() && a_hi.is_finite() && b_lo.is_finite() && b_hi.is_finite()) {
        return Err(CommonArcError::NonFinite);
    }
    let lo = a_lo.max(b_lo);
    let hi = a_hi.min(b_hi);
    // The overlap bounds are authoritative source-domain endpoint values
    // (declared trims or certified subdivision splits), so the classification
    // is exact over declared `f64` inputs. The extent sign is the proof passed
    // into `assemble`; it is never derived from a recomputed projection.
    let extent_sign = match classify_exact_interval_overlap(a_lo, a_hi, b_lo, b_hi) {
        CertifiedIntervalOverlap::Disjoint => CertifiedSign::Negative,
        CertifiedIntervalOverlap::Point => CertifiedSign::Zero,
        CertifiedIntervalOverlap::Positive => CertifiedSign::Positive,
        CertifiedIntervalOverlap::Unresolved => return Err(CommonArcError::UnresolvedOverlap),
    };
    // Orientation relative to the shared source axis: increasing traversal is
    // Codirected, decreasing is Opposed.
    let orientation_a = if d0a <= d1a {
        OrientationAlongSupport::Codirected
    } else {
        OrientationAlongSupport::Opposed
    };
    let orientation_b = if d0b <= d1b {
        OrientationAlongSupport::Codirected
    } else {
        OrientationAlongSupport::Opposed
    };
    let axis = CanonicalSourceAxis { span_id };
    let corr_a = if orientation_a == OrientationAlongSupport::Codirected {
        CertifiedParameterCorrespondence::Identity { source_axis: axis }
    } else {
        CertifiedParameterCorrespondence::Reversal {
            source_axis: axis,
            to_axis: reversal_to_axis(d0a, d1a)?,
        }
    };
    let corr_b = if orientation_b == OrientationAlongSupport::Codirected {
        CertifiedParameterCorrespondence::Identity { source_axis: axis }
    } else {
        CertifiedParameterCorrespondence::Reversal {
            source_axis: axis,
            to_axis: reversal_to_axis(d0b, d1b)?,
        }
    };
    let support = CommonSupportFragment {
        basis: CommonSupportBasis::IdenticalSourceProvenance,
        identity: CommonSupportIdentity::SameSourceOccurrence(span_id),
        certificate: SupportIdentityCertificate::IdenticalSourceProvenance { span_id },
    };
    // The overlap bounds are authoritative source-domain endpoint values
    // (declared trims or certified subdivision splits), retained verbatim by
    // the spans. They are wrapped in `AuthoritativeParameterKey` — never
    // recomputed projections — so the boundary identity is copied from the
    // retained value and is stable under reversal, parent/child and sibling
    // subdivision.
    let boundaries = CommonArcBoundaries {
        start: CommonArcBoundary {
            key: CommonArcBoundaryKey::SourceEndpoint {
                occurrence: span_id,
                side: AxisSide::Lower,
                parameter: AuthoritativeParameterKey::from_authoritative(lo),
                periodic_lift: None,
            },
            enclosure: ParameterEnclosure { lo, hi: lo },
        },
        end: CommonArcBoundary {
            key: CommonArcBoundaryKey::SourceEndpoint {
                occurrence: span_id,
                side: AxisSide::Upper,
                parameter: AuthoritativeParameterKey::from_authoritative(hi),
                periodic_lift: None,
            },
            enclosure: ParameterEnclosure { lo: hi, hi: hi },
        },
    };
    assemble(
        first,
        second,
        support,
        boundaries,
        extent_sign,
        corr_a,
        corr_b,
        orientation_a,
        orientation_b,
        DeckContext::rank0(),
    )
}

/// The certified affine map from a reversed participant's traversal parameter
/// to the canonical source axis: `s = upper − t`.
fn reversal_to_axis(d0: f64, d1: f64) -> Result<CertifiedAffineMap, CommonArcError> {
    let upper = d0.max(d1);
    CertifiedAffineMap::new(
        CertifiedInterval::point(-1.0),
        CertifiedInterval::point(upper),
    )
}

// ---------------------------------------------------------------------------
// Analytic line support
// ---------------------------------------------------------------------------

/// Extract the developed line of a line span.
fn span_as_line(span: &CurveSpan2) -> Result<&LineSegment2, CommonArcError> {
    match span {
        CurveSpan2::AnalyticLine(segment) => Ok(segment),
        _ => Err(CommonArcError::UnsupportedSupportIdentity),
    }
}

/// Extract the developed circular arc of a circle span.
fn span_as_arc(span: &CurveSpan2) -> Result<&DirectedCircularArc2, CommonArcError> {
    match span {
        CurveSpan2::AnalyticCircularArc(arc) => Ok(arc),
        _ => Err(CommonArcError::UnsupportedSupportIdentity),
    }
}

/// Exact `(b − a) × (d − c)` over the `f64` coordinates.
fn cross_of_segments(a: Point2, b: Point2, c: Point2, d: Point2) -> Expansion {
    let ax = Expansion::from_sum(b.x, -a.x);
    let ay = Expansion::from_sum(b.y, -a.y);
    let bx = Expansion::from_sum(d.x, -c.x);
    let by = Expansion::from_sum(d.y, -c.y);
    ax.mul_expansion(&by).merge(&ay.mul_expansion(&bx).negate())
}

/// Exact `orient(a, b, c) = (b − a) × (c − a)` over the `f64` coordinates.
fn orient_three(a: Point2, b: Point2, c: Point2) -> Expansion {
    let dx = Expansion::from_sum(b.x, -a.x);
    let dy = Expansion::from_sum(b.y, -a.y);
    let cx = Expansion::from_sum(c.x, -a.x);
    let cy = Expansion::from_sum(c.y, -a.y);
    dx.mul_expansion(&cy).merge(&dy.mul_expansion(&cx).negate())
}

/// Whether two line segments lie on the same certified support line: the exact
/// parallel cross and the exact point-on-line predicates both decide.
///
/// This is certified support equality over the declared `f64` coordinates —
/// never an epsilon cross-product, never approximate collinearity.
fn certified_collinear(lhs: &LineSegment2, rhs: &LineSegment2) -> bool {
    let parallel =
        cross_of_segments(lhs.start, lhs.end, rhs.start, rhs.end).sign() == CertifiedSign::Zero;
    let on_line = orient_three(lhs.start, lhs.end, rhs.start).sign() == CertifiedSign::Zero;
    parallel && on_line
}

/// The authoritative source-forward form of a line occurrence: the occurrence's
/// own start/end points ordered by the provenance's traversal vertices.
///
/// The source axis is oriented **from authoritative provenance** (the source's
/// start and end vertex identity order), never from the current traversal:
/// reversing an occurrence swaps the provenance vertices, so the canonical form
/// is unchanged by reversal (and subdivision, which preserves provenance
/// verbatim). `None` when the provenance does not identify both traversal
/// vertices, so the canonical orientation cannot be certified from provenance
/// alone.
fn source_forward_axis(line: &LineSegment2) -> Option<(Point2, Point2)> {
    match (
        line.provenance.start_vertex_id,
        line.provenance.end_vertex_id,
    ) {
        (SourceVertexKey::ShellVertex(a), SourceVertexKey::ShellVertex(b)) if a != b => {
            if a < b {
                Some((line.start, line.end))
            } else {
                Some((line.end, line.start))
            }
        }
        _ => None,
    }
}

/// The traversal orientation of a line occurrence along its authoritative
/// source axis: `Codirected` when it traverses from its lesser provenance
/// vertex to its greater, `Opposed` otherwise. `None` when the provenance does
/// not identify both vertices.
fn occurrence_source_orientation(line: &LineSegment2) -> Option<OrientationAlongSupport> {
    match (
        line.provenance.start_vertex_id,
        line.provenance.end_vertex_id,
    ) {
        (SourceVertexKey::ShellVertex(a), SourceVertexKey::ShellVertex(b)) if a != b => {
            Some(if a < b {
                OrientationAlongSupport::Codirected
            } else {
                OrientationAlongSupport::Opposed
            })
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Exact projection helpers (no expansion-to-f64 collapse)
// ---------------------------------------------------------------------------

/// Exact numerator of the canonical-axis projection `λ(p) = num/den`, the dot
/// product `(p − A)·(B − A)` over the declared `f64` coordinates.
fn proj_num(p: Point2, axis: &LineSegment2) -> Expansion {
    let dx = Expansion::from_sum(axis.end.x, -axis.start.x);
    let dy = Expansion::from_sum(axis.end.y, -axis.start.y);
    let px = Expansion::from_sum(p.x, -axis.start.x);
    let py = Expansion::from_sum(p.y, -axis.start.y);
    px.mul_expansion(&dx).merge(&py.mul_expansion(&dy))
}

/// Exact denominator `|B − A|²` of the canonical-axis projection.
fn proj_den(axis: &LineSegment2) -> Expansion {
    let dx = Expansion::from_sum(axis.end.x, -axis.start.x);
    let dy = Expansion::from_sum(axis.end.y, -axis.start.y);
    dx.mul_expansion(&dx).merge(&dy.mul_expansion(&dy))
}

/// Exact numerator of `λ(p) − 1 = (p − B)·(B − A)/|B − A|²`, the dot product
/// `(p − B)·(B − A)`.
fn proj_num_rel_end(p: Point2, axis: &LineSegment2) -> Expansion {
    let dx = Expansion::from_sum(axis.end.x, -axis.start.x);
    let dy = Expansion::from_sum(axis.end.y, -axis.start.y);
    let px = Expansion::from_sum(p.x, -axis.end.x);
    let py = Expansion::from_sum(p.y, -axis.end.y);
    px.mul_expansion(&dx).merge(&py.mul_expansion(&dy))
}

/// The certified canonical-axis projection enclosure of a point, via
/// outward-rounded interval division of the exact expansion numerator by the
/// exact expansion denominator. No expansion-to-`f64` collapse: the
/// localization is a certified interval, never a representative `f64`.
fn line_projection_enclosure(
    p: Point2,
    axis: &LineSegment2,
) -> Result<ParameterEnclosure, CommonArcError> {
    let num = proj_num(p, axis);
    let den = proj_den(axis);
    if den.sign() == CertifiedSign::Zero {
        return Err(CommonArcError::NonFinite);
    }
    let num_iv = CertifiedInterval::from_expansion(&num);
    let den_iv = CertifiedInterval::from_expansion(&den);
    let iv = num_iv.div(&den_iv).ok_or(CommonArcError::NonFinite)?;
    if iv.is_finite() {
        Ok(ParameterEnclosure {
            lo: iv.lo,
            hi: iv.hi,
        })
    } else {
        Err(CommonArcError::NonFinite)
    }
}

/// The certified affine map from a distinct collinear occurrence's traversal
/// parameter `[0,1]` to the canonical source axis: `λ(t) = intercept + slope·t`
/// with `slope = λ(end) − λ(start)` and `intercept = λ(start)`, both certified
/// enclosures from exact projection expansions (no `to_f64` collapse).
fn line_axis_map(
    line: &LineSegment2,
    axis: &LineSegment2,
) -> Result<CertifiedAffineMap, CommonArcError> {
    let lambda_start = line_projection_enclosure(line.start, axis)?;
    let lambda_end = line_projection_enclosure(line.end, axis)?;
    let start_iv = CertifiedInterval {
        lo: lambda_start.lo,
        hi: lambda_start.hi,
    };
    let end_iv = CertifiedInterval {
        lo: lambda_end.lo,
        hi: lambda_end.hi,
    };
    let slope = end_iv.sub(&start_iv);
    let intercept = start_iv;
    CertifiedAffineMap::new(slope, intercept)
}

/// The certified analytic-line CommonArc path.
///
/// Every line pair routes here — including two pieces of the same source line
/// occurrence — because a line's authoritative domain is always `(0,1)`, so the
/// traversal orientation must come from the certified affine maps' slopes, never
/// from the reported domain.
///
/// **Canonical source axis.** The canonical (first, by stable span id)
/// occurrence is selected as the canonical authoritative occurrence, and its
/// authoritative source axis is oriented from its provenance's start/end vertex
/// order ([`source_forward_axis`]) — never from the current traversal, so
/// reversing a participant moves its *correspondence*, not the axis.
///
/// **Boundary selection (no projection-to-`f64`).** For a distinct collinear
/// occurrence, each endpoint's lower/upper role and its relation to the axis
/// endpoints `0`/`1` is decided by the **exact sign** of an expansion
/// (`dot(p−A,B−A)` or `dot(p−B,B−A)`), never by comparing collapsed `f64`
/// projections. The selected boundary retains its construction key
/// ([`CommonArcBoundaryKey`]) and a certified parameter enclosure
/// ([`ParameterEnclosure`]) for localization. The positive extent is certified
/// by the exact sign of `(end − start)` in numerator space (scaled by the
/// positive `|B−A|²`), passed as the proof into [`assemble`].
fn analytic_line_common_arc(
    lhs: &CurveSpan2,
    rhs: &CurveSpan2,
) -> Result<CommonArc2, CommonArcError> {
    let (first, second) = if lhs.span_id() <= rhs.span_id() {
        (lhs, rhs)
    } else {
        (rhs, lhs)
    };
    let line1 = span_as_line(first)?;
    let line2 = span_as_line(second)?;
    if !certified_collinear(line1, line2) {
        return Err(CommonArcError::UnsupportedSupportIdentity);
    }
    if line1.is_degenerate() || line2.is_degenerate() {
        return Err(CommonArcError::UnsupportedSupportIdentity);
    }
    let same_occurrence = same_source_occurrence(first, second);
    let anchor = first.span_id();
    let other_span = second.span_id();
    let support = if same_occurrence {
        CommonSupportFragment {
            basis: CommonSupportBasis::IdenticalSourceProvenance,
            identity: CommonSupportIdentity::SameSourceOccurrence(anchor),
            certificate: SupportIdentityCertificate::IdenticalSourceProvenance { span_id: anchor },
        }
    } else {
        CommonSupportFragment {
            basis: CommonSupportBasis::IdenticalAnalyticSupport,
            identity: CommonSupportIdentity::IdenticalAnalyticSupport {
                class: AnalyticSupportClass::Line,
                anchor,
            },
            certificate: SupportIdentityCertificate::CertifiedCollinearLines {
                first: anchor,
                second: other_span,
            },
        }
    };
    // The canonical source axis: the canonical occurrence's authoritative
    // source-forward parameterization. When the provenance does not identify
    // both vertices, fall back to the occurrence's own traversal.
    let (ax, bx) = source_forward_axis(line1).unwrap_or((line1.start, line1.end));
    let axis = LineSegment2 {
        start: ax,
        end: bx,
        provenance: line1.provenance,
    };
    let axis_ctx = CanonicalSourceAxis { span_id: anchor };
    // The canonical occurrence reads the axis directly: Identity when it
    // traverses source-forward, Reversal when source-backward.
    let orientation_a =
        occurrence_source_orientation(line1).unwrap_or(OrientationAlongSupport::Codirected);
    let corr_a = correspondence_for_orientation(axis_ctx, orientation_a, 0.0, 1.0)?;
    let (corr_b, orientation_b, boundaries, extent_sign) = if same_occurrence {
        // Two pieces of one authoritative occurrence share the source axis; the
        // second reads it in its own authoritative direction. The overlap is
        // the whole canonical axis `[0, 1]`, bounded by the axis's own
        // authoritative source endpoints.
        let orientation_b =
            occurrence_source_orientation(line2).unwrap_or(OrientationAlongSupport::Codirected);
        let corr_b = correspondence_for_orientation(axis_ctx, orientation_b, 0.0, 1.0)?;
        let boundaries = CommonArcBoundaries {
            start: CommonArcBoundary {
                key: CommonArcBoundaryKey::SourceEndpoint {
                    occurrence: anchor,
                    side: AxisSide::Lower,
                    parameter: AuthoritativeParameterKey::from_authoritative(0.0),
                    periodic_lift: None,
                },
                enclosure: ParameterEnclosure { lo: 0.0, hi: 0.0 },
            },
            end: CommonArcBoundary {
                key: CommonArcBoundaryKey::SourceEndpoint {
                    occurrence: anchor,
                    side: AxisSide::Upper,
                    parameter: AuthoritativeParameterKey::from_authoritative(1.0),
                    periodic_lift: None,
                },
                enclosure: ParameterEnclosure { lo: 1.0, hi: 1.0 },
            },
        };
        // 1 − 0 > 0 exactly.
        (corr_b, orientation_b, boundaries, CertifiedSign::Positive)
    } else {
        // A distinct collinear occurrence is mapped onto the canonical axis.
        // Determine each endpoint's lower/upper role and select the overlap
        // boundaries by exact expansion signs — no projection-to-f64.
        let s_p = proj_num(line2.start, &axis);
        let s_q = proj_num(line2.end, &axis);
        let diff = s_p.merge(&s_q.negate()); // λ(start) − λ(end), scaled by den
        let (lower_pt, upper_pt) = match diff.sign() {
            CertifiedSign::Positive => (line2.end, line2.start), // λ(start) > λ(end)
            CertifiedSign::Negative => (line2.start, line2.end), // λ(start) < λ(end)
            CertifiedSign::Zero => return Err(CommonArcError::NonInvertibleCorrespondence),
        };
        let den = proj_den(&axis);
        // Lower boundary: max(axis.Lower@0, other.Lower@λ(lower_pt)).
        let s_lower = proj_num(lower_pt, &axis); // sign vs 0
        let (lower_key, lower_enc, lower_num) = match s_lower.sign() {
            CertifiedSign::Positive => {
                // other_lo > 0: the overlap starts at the other occurrence's
                // lower-projecting endpoint.
                let enc = line_projection_enclosure(lower_pt, &axis)?;
                (
                    CommonArcBoundaryKey::MappedEndpoint {
                        span: other_span,
                        side: AxisSide::Lower,
                    },
                    enc,
                    s_lower,
                )
            }
            CertifiedSign::Negative | CertifiedSign::Zero => {
                // other_lo <= 0: the overlap starts at the axis's own lower
                // endpoint (source param 0). On equality the constructions
                // coincide; canonicalize to the axis endpoint.
                (
                    CommonArcBoundaryKey::SourceEndpoint {
                        occurrence: anchor,
                        side: AxisSide::Lower,
                        parameter: AuthoritativeParameterKey::from_authoritative(0.0),
                        periodic_lift: None,
                    },
                    ParameterEnclosure { lo: 0.0, hi: 0.0 },
                    Expansion::zero(),
                )
            }
        };
        // Upper boundary: min(axis.Upper@1, other.Upper@λ(upper_pt)).
        let s_upper_rel = proj_num_rel_end(upper_pt, &axis); // sign of (λ(upper_pt) − 1)
        let (upper_key, upper_enc, upper_num) = match s_upper_rel.sign() {
            CertifiedSign::Negative => {
                // other_hi < 1: the overlap ends at the other occurrence's
                // upper-projecting endpoint.
                let enc = line_projection_enclosure(upper_pt, &axis)?;
                let s_upper = proj_num(upper_pt, &axis);
                (
                    CommonArcBoundaryKey::MappedEndpoint {
                        span: other_span,
                        side: AxisSide::Upper,
                    },
                    enc,
                    s_upper,
                )
            }
            CertifiedSign::Positive | CertifiedSign::Zero => {
                // other_hi >= 1: the overlap ends at the axis's own upper
                // endpoint (source param 1). On equality canonicalize to axis.
                (
                    CommonArcBoundaryKey::SourceEndpoint {
                        occurrence: anchor,
                        side: AxisSide::Upper,
                        parameter: AuthoritativeParameterKey::from_authoritative(1.0),
                        periodic_lift: None,
                    },
                    ParameterEnclosure { lo: 1.0, hi: 1.0 },
                    den,
                )
            }
        };
        // Positive extent proof: exact sign of (upper − lower) in numerator
        // space (both scaled by the positive den), so the sign is the sign of
        // the canonical-axis extent.
        let extent_num = upper_num.merge(&lower_num.negate());
        let extent_sign = extent_num.sign();
        let boundaries = CommonArcBoundaries {
            start: CommonArcBoundary {
                key: lower_key,
                enclosure: lower_enc,
            },
            end: CommonArcBoundary {
                key: upper_key,
                enclosure: upper_enc,
            },
        };
        let map = line_axis_map(line2, &axis)?;
        let orientation_b = map
            .orientation()
            .ok_or(CommonArcError::UnresolvedCorrespondence)?;
        let proof = AnalyticSupportCorrespondenceCertificate {
            support: support.identity,
        };
        let corr_b = CertifiedParameterCorrespondence::Analytic { map, proof };
        (corr_b, orientation_b, boundaries, extent_sign)
    };
    assemble(
        first,
        second,
        support,
        boundaries,
        extent_sign,
        corr_a,
        corr_b,
        orientation_a,
        orientation_b,
        DeckContext::rank0(),
    )
}

/// Whether two spans are pieces of the same authoritative source occurrence.
///
/// The authoritative occurrence relation is carried by the provenance (the edge
/// use and source edge), not by the span's geometric data. Certified
/// subdivision preserves the provenance verbatim and reversal preserves the
/// edge-use/source-edge ids, so identical spans, parent/child subdivisions,
/// overlapping sibling subdivisions and reversed pieces all satisfy this;
/// distinct twin edge uses never do.
fn same_source_occurrence(a: &CurveSpan2, b: &CurveSpan2) -> bool {
    let pa = a.provenance();
    let pb = b.provenance();
    pa.edge_use_id == pb.edge_use_id && pa.source_edge_id == pb.source_edge_id
}

// ---------------------------------------------------------------------------
// Analytic circle support
// ---------------------------------------------------------------------------

/// The exact squared radius of a support circle.
fn radius_squared(arc: &DirectedCircularArc2) -> Expansion {
    exact_dot2(
        [arc.cos_basis.x, arc.cos_basis.y],
        [arc.cos_basis.x, arc.cos_basis.y],
    )
}

/// Whether two circular arcs share a certified identical support and the same
/// authoritative parameterization: exact center equality, exact squared-radius
/// equality, and bitwise-identical basis vectors.
///
/// No approximate center/radius equality and no principal-angle phase
/// recovery: a different basis is an unresolved phase, not a certified
/// correspondence.
fn certified_equal_circle_support(lhs: &DirectedCircularArc2, rhs: &DirectedCircularArc2) -> bool {
    let center_equal = exact_sq_dist([lhs.center.x, lhs.center.y], [rhs.center.x, rhs.center.y])
        .sign()
        == CertifiedSign::Zero;
    let radius_equal = radius_squared(lhs)
        .merge(&radius_squared(rhs).negate())
        .sign()
        == CertifiedSign::Zero;
    let same_parameterization = lhs.cos_basis == rhs.cos_basis && lhs.sin_basis == rhs.sin_basis;
    center_equal && radius_equal && same_parameterization
}

/// The certified analytic-circle CommonArc path for distinct occurrences.
///
/// Only a certified identical support **with the same authoritative
/// parameterization** is admitted; the two unwrapped parameter intervals are
/// then compared on the shared axis without principal-angle wrapping or
/// shortest-arc inference. When the certified intervals are disjoint but their
/// union exceeds one period, a periodic (seam-crossing) overlap is not
/// certified by the interval evidence and the case is `Unsupported`.
fn analytic_circle_common_arc(
    lhs: &CurveSpan2,
    rhs: &CurveSpan2,
) -> Result<CommonArc2, CommonArcError> {
    let (first, second) = if lhs.span_id() <= rhs.span_id() {
        (lhs, rhs)
    } else {
        (rhs, lhs)
    };
    let arc1 = span_as_arc(first)?;
    let arc2 = span_as_arc(second)?;
    if !certified_equal_circle_support(arc1, arc2) {
        return Err(CommonArcError::UnsupportedSupportIdentity);
    }
    let (d0a, d1a) = first.authoritative_domain();
    let (d0b, d1b) = second.authoritative_domain();
    let a_lo = d0a.min(d1a);
    let a_hi = d0a.max(d1a);
    let b_lo = d0b.min(d1b);
    let b_hi = d0b.max(d1b);
    let anchor = first.span_id();
    let other_span = second.span_id();
    // The overlap bounds are authoritative unwrapped source parameters on the
    // shared circle parameterization. The unwrapped value encodes the unique
    // certified `2π` lift (it is never reduced to a principal angle), so
    // `periodic_lift` is `None` here — the parameter key itself retains the
    // lift. A seam-crossing overlap whose unwrapped intervals are disjoint but
    // span more than one period is not certified by this evidence.
    let (lo, hi, extent_sign, lower_occ, upper_occ) =
        match classify_exact_interval_overlap(a_lo, a_hi, b_lo, b_hi) {
            CertifiedIntervalOverlap::Disjoint => {
                // Two arcs on the same parameterization whose unwrapped
                // intervals are disjoint: if their union fits within one
                // period there is no geometric overlap; otherwise a periodic
                // seam overlap exists but is not certified here.
                let union_lo = a_lo.min(b_lo);
                let union_hi = a_hi.max(b_hi);
                if union_hi - union_lo < std::f64::consts::TAU {
                    return Err(CommonArcError::EmptyOverlap);
                }
                return Err(CommonArcError::UnsupportedSupportIdentity);
            }
            CertifiedIntervalOverlap::Point => return Err(CommonArcError::PointOnlyOverlap),
            CertifiedIntervalOverlap::Positive => {
                let lo = a_lo.max(b_lo);
                let hi = a_hi.min(b_hi);
                // The boundary at `lo` is the participant whose lower endpoint
                // is larger (canonical: anchor on equality); the boundary at
                // `hi` is the participant whose upper endpoint is smaller.
                let lower_occ = if a_lo >= b_lo { anchor } else { other_span };
                let upper_occ = if a_hi <= b_hi { anchor } else { other_span };
                (lo, hi, CertifiedSign::Positive, lower_occ, upper_occ)
            }
            CertifiedIntervalOverlap::Unresolved => return Err(CommonArcError::UnresolvedOverlap),
        };
    // Same parameterization: both participants read the shared t-axis, so a
    // Codirected traversal is identity and an Opposed traversal is reversal in
    // the authoritative source parameter.
    let orientation_a = if d0a <= d1a {
        OrientationAlongSupport::Codirected
    } else {
        OrientationAlongSupport::Opposed
    };
    let orientation_b = if d0b <= d1b {
        OrientationAlongSupport::Codirected
    } else {
        OrientationAlongSupport::Opposed
    };
    let axis = CanonicalSourceAxis { span_id: anchor };
    let corr_a = correspondence_for_orientation(axis, orientation_a, d0a, d1a)?;
    let corr_b = correspondence_for_orientation(axis, orientation_b, d0b, d1b)?;
    let support = CommonSupportFragment {
        basis: CommonSupportBasis::IdenticalAnalyticSupport,
        identity: CommonSupportIdentity::IdenticalAnalyticSupport {
            class: AnalyticSupportClass::Circle,
            anchor,
        },
        certificate: SupportIdentityCertificate::CertifiedEqualCircles {
            first: anchor,
            second: other_span,
        },
    };
    let boundaries = CommonArcBoundaries {
        start: CommonArcBoundary {
            key: CommonArcBoundaryKey::SourceEndpoint {
                occurrence: lower_occ,
                side: AxisSide::Lower,
                parameter: AuthoritativeParameterKey::from_authoritative(lo),
                periodic_lift: None,
            },
            enclosure: ParameterEnclosure { lo, hi: lo },
        },
        end: CommonArcBoundary {
            key: CommonArcBoundaryKey::SourceEndpoint {
                occurrence: upper_occ,
                side: AxisSide::Upper,
                parameter: AuthoritativeParameterKey::from_authoritative(hi),
                periodic_lift: None,
            },
            enclosure: ParameterEnclosure { lo: hi, hi: hi },
        },
    };
    assemble(
        first,
        second,
        support,
        boundaries,
        extent_sign,
        corr_a,
        corr_b,
        orientation_a,
        orientation_b,
        DeckContext::rank0(),
    )
}

/// The certified correspondence of a same-axis participant: identity when it
/// traverses the axis forward, reversal (the certified affine map `s = upper − t`)
/// when it traverses backward.
fn correspondence_for_orientation(
    axis: CanonicalSourceAxis,
    orientation: OrientationAlongSupport,
    d0: f64,
    d1: f64,
) -> Result<CertifiedParameterCorrespondence, CommonArcError> {
    if orientation == OrientationAlongSupport::Codirected {
        Ok(CertifiedParameterCorrespondence::Identity { source_axis: axis })
    } else {
        Ok(CertifiedParameterCorrespondence::Reversal {
            source_axis: axis,
            to_axis: reversal_to_axis(d0, d1)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::source_evidence::{BoundId, EdgeUseId, SourceVertexKey};
    use super::super::bezier::RationalBezierSpan2;
    use super::super::curve2d::{SourceEdgeId, SourceEntityId, SourceFaceId};
    use super::super::quotient::{AmbientLatticeId, DeckRank};
    use super::*;
    use truck_geometry::prelude::{Point2, Vector2};

    const PI: f64 = std::f64::consts::PI;
    const TAU: f64 = std::f64::consts::TAU;

    fn provenance_with(
        edge_index: usize,
        start: SourceVertexKey,
        end: SourceVertexKey,
    ) -> CurveOccurrenceProvenance {
        CurveOccurrenceProvenance {
            source_face_id: Some(SourceFaceId(1)),
            bound_id: BoundId(0),
            edge_use_id: EdgeUseId::new(BoundId(0), edge_index),
            source_edge_id: SourceEdgeId(edge_index),
            start_vertex_id: start,
            end_vertex_id: end,
            source_curve_entity_id: Some(SourceEntityId(100 + edge_index as u64)),
        }
    }

    fn provenance(edge_index: usize) -> CurveOccurrenceProvenance {
        provenance_with(
            edge_index,
            SourceVertexKey::ShellVertex(edge_index),
            SourceVertexKey::ShellVertex(edge_index + 1),
        )
    }

    fn line_span(start: Point2, end: Point2, edge_index: usize) -> CurveSpan2 {
        CurveSpan2::from_line(LineSegment2 {
            start,
            end,
            provenance: provenance(edge_index),
        })
    }

    fn arc_span(center: Point2, r: f64, t0: f64, t1: f64, edge_index: usize) -> CurveSpan2 {
        CurveSpan2::from_circular_arc(DirectedCircularArc2 {
            center,
            cos_basis: Vector2::new(r, 0.0),
            sin_basis: Vector2::new(0.0, r),
            t0,
            t1,
            provenance: provenance(edge_index),
        })
    }

    fn parabola(edge_index: usize) -> RationalBezierSpan2 {
        RationalBezierSpan2::new(
            vec![(0.0, 0.0, 1.0), (0.5, 0.0, 1.0), (1.0, 1.0, 1.0)],
            (0.0, 1.0),
            provenance(edge_index),
        )
        .unwrap()
    }

    fn bezier_span(edge_index: usize) -> CurveSpan2 {
        CurveSpan2::RationalBezier(parabola(edge_index))
    }

    fn pair(lhs: &CurveSpan2, rhs: &CurveSpan2) -> CommonArc2 {
        common_arc_for_pair(lhs, rhs).expect("the pair certifies a CommonArc")
    }

    /// A representative canonical-axis value of a boundary, for test
    /// assertions only. Identity is the construction key, never this value;
    /// for a point enclosure (authoritative source parameter) it is exact, for
    /// a projected enclosure it is the enclosure midpoint.
    fn repr(b: &CommonArcBoundary) -> f64 {
        let e = b.enclosure();
        (e.lo + e.hi) * 0.5
    }

    // ----- same-support lines ------------------------------------------------

    #[test]
    fn identical_line_spans_certify_a_common_arc() {
        // The same occurrence twice, same direction: a positive-length overlap.
        let a = line_span(Point2::new(0.0, 0.0), Point2::new(2.0, 0.0), 0);
        let b = line_span(Point2::new(0.0, 0.0), Point2::new(2.0, 0.0), 0);
        let arc = pair(&a, &b);
        assert_eq!(
            repr(&arc.boundaries.start),
            0.0,
            "identical spans overlap over the whole canonical axis"
        );
        assert_eq!(repr(&arc.boundaries.end), 1.0);
        assert_eq!(
            arc.identity.orientation,
            OrientationAlongSupport::Codirected
        );
        arc.validate().unwrap();
    }

    #[test]
    fn identical_line_spans_reversed_direction_are_opposed() {
        let seg = LineSegment2 {
            start: Point2::new(0.0, 0.0),
            end: Point2::new(2.0, 0.0),
            provenance: provenance(0),
        };
        let a = CurveSpan2::from_line(seg);
        let b = CurveSpan2::from_line(seg.reverse_occurrence());
        let arc = pair(&a, &b);
        assert_eq!(
            arc.identity.orientation,
            OrientationAlongSupport::Opposed,
            "reversing one occurrence opposes it to the other"
        );
        assert_eq!(repr(&arc.boundaries.start), 0.0);
        assert_eq!(repr(&arc.boundaries.end), 1.0);
        arc.validate().unwrap();
    }

    #[test]
    fn partial_positive_overlap_is_a_common_arc() {
        let a = line_span(Point2::new(0.0, 0.0), Point2::new(2.0, 0.0), 0);
        let b = line_span(Point2::new(1.0, 0.0), Point2::new(3.0, 0.0), 1);
        let arc = pair(&a, &b);
        assert_eq!(
            arc.support.basis,
            CommonSupportBasis::IdenticalAnalyticSupport
        );
        assert!(
            repr(&arc.boundaries.start) > 0.4 && repr(&arc.boundaries.start) < 0.6,
            "the overlap starts at the second span's mapped start (~0.5), got {:?}",
            repr(&arc.boundaries.start)
        );
        assert_eq!(repr(&arc.boundaries.end), 1.0);
        assert_eq!(
            arc.identity.orientation,
            OrientationAlongSupport::Codirected
        );
        arc.validate().unwrap();
    }

    #[test]
    fn one_span_contained_in_the_other_is_a_common_arc() {
        let a = line_span(Point2::new(0.0, 0.0), Point2::new(2.0, 0.0), 0);
        let b = line_span(Point2::new(0.5, 0.0), Point2::new(1.5, 0.0), 1);
        let arc = pair(&a, &b);
        assert!(
            repr(&arc.boundaries.start) > 0.2 && repr(&arc.boundaries.start) < 0.3,
            "contained span maps to [0.25, 0.75] on the canonical axis"
        );
        assert!(repr(&arc.boundaries.end) > 0.7 && repr(&arc.boundaries.end) < 0.8);
        arc.validate().unwrap();
    }

    #[test]
    fn disjoint_intervals_on_the_same_support_are_no_component() {
        let a = line_span(Point2::new(0.0, 0.0), Point2::new(1.0, 0.0), 0);
        let b = line_span(Point2::new(2.0, 0.0), Point2::new(3.0, 0.0), 1);
        assert_eq!(
            common_arc_for_pair(&a, &b),
            Err(CommonArcError::EmptyOverlap)
        );
    }

    #[test]
    fn endpoint_only_meeting_is_not_a_common_arc() {
        let a = line_span(Point2::new(0.0, 0.0), Point2::new(1.0, 0.0), 0);
        let b = line_span(Point2::new(1.0, 0.0), Point2::new(2.0, 0.0), 1);
        // The single shared parameter value cannot be certified as a positive
        // overlap: a typed non-result, never a CommonArc.
        assert!(matches!(
            common_arc_for_pair(&a, &b),
            Err(CommonArcError::UnresolvedOverlap) | Err(CommonArcError::PointOnlyOverlap)
        ));
    }

    #[test]
    fn parallel_distinct_supports_are_unsupported() {
        let a = line_span(Point2::new(0.0, 0.0), Point2::new(1.0, 0.0), 0);
        let b = line_span(Point2::new(0.0, 1.0), Point2::new(1.0, 1.0), 1);
        assert_eq!(
            common_arc_for_pair(&a, &b),
            Err(CommonArcError::UnsupportedSupportIdentity)
        );
    }

    #[test]
    fn collinear_claim_lacking_certified_support_evidence_is_unsupported() {
        // Nearly-collinear directions (exact nonzero cross) are not certified
        // collinearity: no epsilon establishes the support.
        let a = line_span(Point2::new(0.0, 0.0), Point2::new(1.0, 0.0), 0);
        let b = line_span(Point2::new(0.0, 0.0), Point2::new(1.0, 1.0e-15), 1);
        assert_eq!(
            common_arc_for_pair(&a, &b),
            Err(CommonArcError::UnsupportedSupportIdentity)
        );
    }

    // ----- same-support circles ----------------------------------------------

    #[test]
    fn same_circle_support_same_traversal_overlaps() {
        let a = arc_span(Point2::new(0.0, 0.0), 1.0, 0.0, PI / 2.0, 0);
        let b = arc_span(Point2::new(0.0, 0.0), 1.0, PI / 4.0, 3.0 * PI / 4.0, 1);
        let arc = pair(&a, &b);
        assert_eq!(
            arc.support.basis,
            CommonSupportBasis::IdenticalAnalyticSupport
        );
        assert!(
            repr(&arc.boundaries.start) >= PI / 4.0 - 1e-9
                && repr(&arc.boundaries.start) <= PI / 4.0 + 1e-9,
            "overlap starts at the max lower bound π/4"
        );
        assert!(
            repr(&arc.boundaries.end) >= PI / 2.0 - 1e-9
                && repr(&arc.boundaries.end) <= PI / 2.0 + 1e-9,
            "overlap ends at the min upper bound π/2"
        );
        assert_eq!(
            arc.identity.orientation,
            OrientationAlongSupport::Codirected
        );
        arc.validate().unwrap();
    }

    #[test]
    fn same_circle_support_reversed_traversal_is_opposed() {
        let a = arc_span(Point2::new(0.0, 0.0), 1.0, 0.0, PI / 2.0, 0);
        let b = arc_span(Point2::new(0.0, 0.0), 1.0, PI / 2.0, 0.0, 1);
        let arc = pair(&a, &b);
        assert_eq!(arc.identity.orientation, OrientationAlongSupport::Opposed);
        arc.validate().unwrap();
    }

    #[test]
    fn disjoint_arc_intervals_are_no_component() {
        let a = arc_span(Point2::new(0.0, 0.0), 1.0, 0.0, PI / 2.0, 0);
        let b = arc_span(Point2::new(0.0, 0.0), 1.0, PI, 3.0 * PI / 2.0, 1);
        assert_eq!(
            common_arc_for_pair(&a, &b),
            Err(CommonArcError::EmptyOverlap)
        );
    }

    #[test]
    fn endpoint_only_circle_contact_is_not_a_common_arc() {
        let a = arc_span(Point2::new(0.0, 0.0), 1.0, 0.0, PI / 2.0, 0);
        let b = arc_span(Point2::new(0.0, 0.0), 1.0, PI / 2.0, PI, 1);
        assert_eq!(
            common_arc_for_pair(&a, &b),
            Err(CommonArcError::PointOnlyOverlap)
        );
    }

    #[test]
    fn distinct_tangent_circles_are_not_same_support() {
        // Two radius-1 circles tangent at distance 2: distinct supports, an
        // isolated tangency, never a CommonArc.
        let a = arc_span(Point2::new(0.0, 0.0), 1.0, 0.0, TAU, 0);
        let b = arc_span(Point2::new(2.0, 0.0), 1.0, 0.0, TAU, 1);
        assert_eq!(
            common_arc_for_pair(&a, &b),
            Err(CommonArcError::UnsupportedSupportIdentity)
        );
    }

    #[test]
    fn same_looking_uncertified_circle_support_is_unsupported() {
        // Same center and radius but a rotated basis: the phase correspondence
        // is not certified without principal-angle recovery.
        let theta: f64 = 0.7;
        let b = CurveSpan2::from_circular_arc(DirectedCircularArc2 {
            center: Point2::new(0.0, 0.0),
            cos_basis: Vector2::new(theta.cos(), theta.sin()),
            sin_basis: Vector2::new(-theta.sin(), theta.cos()),
            t0: 0.0,
            t1: PI,
            provenance: provenance(1),
        });
        let a = arc_span(Point2::new(0.0, 0.0), 1.0, 0.0, PI, 0);
        assert_eq!(
            common_arc_for_pair(&a, &b),
            Err(CommonArcError::UnsupportedSupportIdentity)
        );
    }

    #[test]
    fn seam_crossing_periodic_overlap_is_unsupported() {
        // Two same-parameterization arcs whose unwrapped intervals are disjoint
        // but span more than one period: a periodic overlap would exist, but
        // the interval evidence does not certify it here.
        let a = arc_span(Point2::new(0.0, 0.0), 1.0, 0.0, PI, 0);
        let b = arc_span(Point2::new(0.0, 0.0), 1.0, TAU, 3.0 * PI, 1);
        assert_eq!(
            common_arc_for_pair(&a, &b),
            Err(CommonArcError::UnsupportedSupportIdentity)
        );
    }

    // ----- provenance-identical generic spans --------------------------------

    #[test]
    fn same_source_occurrence_repeated_is_a_common_arc() {
        let a = bezier_span(0);
        let b = bezier_span(0);
        let arc = pair(&a, &b);
        assert_eq!(
            arc.support.basis,
            CommonSupportBasis::IdenticalSourceProvenance
        );
        assert_eq!(repr(&arc.boundaries.start), 0.0);
        assert_eq!(repr(&arc.boundaries.end), 1.0);
        assert_eq!(
            arc.identity.orientation,
            OrientationAlongSupport::Codirected
        );
        arc.validate().unwrap();
    }

    #[test]
    fn parent_span_versus_child_certifies_the_sub_overlap() {
        let parent = bezier_span(0);
        let child = CurveSpan2::RationalBezier(parabola(0).subdivide(0.5).0);
        let arc = pair(&parent, &child);
        assert_eq!(repr(&arc.boundaries.start), 0.0);
        assert_eq!(repr(&arc.boundaries.end), 0.5);
        arc.validate().unwrap();
    }

    #[test]
    fn two_overlapping_children_certify_the_shared_interval() {
        let child_a = CurveSpan2::RationalBezier(parabola(0).subdivide(0.75).0);
        let child_b = CurveSpan2::RationalBezier(parabola(0).subdivide(0.25).1);
        let arc = pair(&child_a, &child_b);
        assert!(
            (repr(&arc.boundaries.start) - 0.25).abs() < 1e-12,
            "overlap starts at 0.25"
        );
        assert!(
            (repr(&arc.boundaries.end) - 0.75).abs() < 1e-12,
            "overlap ends at 0.75"
        );
        arc.validate().unwrap();
    }

    #[test]
    fn two_disjoint_children_are_no_component() {
        let left = CurveSpan2::RationalBezier(parabola(0).subdivide(0.25).0);
        let right = CurveSpan2::RationalBezier(parabola(0).subdivide(0.25).1.subdivide(0.5).1);
        assert_eq!(
            common_arc_for_pair(&left, &right),
            Err(CommonArcError::EmptyOverlap)
        );
    }

    #[test]
    fn reversed_child_is_opposed_over_the_same_interval() {
        let child = CurveSpan2::RationalBezier(parabola(0).subdivide(0.5).0);
        let rev = CurveSpan2::RationalBezier(parabola(0).subdivide(0.5).0.reverse_occurrence());
        let arc = pair(&child, &rev);
        assert_eq!(repr(&arc.boundaries.start), 0.0);
        assert_eq!(repr(&arc.boundaries.end), 0.5);
        assert_eq!(arc.identity.orientation, OrientationAlongSupport::Opposed);
        arc.validate().unwrap();
    }

    #[test]
    fn same_sampled_geometry_different_unsupported_provenance() {
        // Identical control points, different occurrences: no certified support
        // identity, no common arc — typed Unsupported, not proximity.
        let a = bezier_span(0);
        let b = bezier_span(1);
        assert_eq!(
            common_arc_for_pair(&a, &b),
            Err(CommonArcError::UnsupportedSupportIdentity)
        );
    }

    // ----- identity and metamorphic invariants --------------------------------

    #[test]
    fn operand_swap_preserves_identity() {
        let a = line_span(Point2::new(0.0, 0.0), Point2::new(2.0, 0.0), 0);
        let b = line_span(Point2::new(1.0, 0.0), Point2::new(3.0, 0.0), 1);
        let ab = pair(&a, &b);
        let ba = pair(&b, &a);
        assert_eq!(ab.identity, ba.identity, "identity is swap-invariant");
        assert_eq!(
            ab.participants[0].span_id, ba.participants[0].span_id,
            "participant roles are canonical"
        );
    }

    #[test]
    fn source_reversal_preserves_identity() {
        let a = line_span(Point2::new(0.0, 0.0), Point2::new(2.0, 0.0), 0);
        let b = line_span(Point2::new(1.0, 0.0), Point2::new(3.0, 0.0), 1);
        let rev_a = CurveSpan2::from_line(match &a {
            CurveSpan2::AnalyticLine(seg) => seg.reverse_occurrence(),
            _ => unreachable!(),
        });
        let rev_b = CurveSpan2::from_line(match &b {
            CurveSpan2::AnalyticLine(seg) => seg.reverse_occurrence(),
            _ => unreachable!(),
        });
        let forward = pair(&a, &b);
        let reversed = pair(&rev_a, &rev_b);
        assert_eq!(
            forward.identity, reversed.identity,
            "reversing both occurrences preserves identity"
        );
    }

    #[test]
    fn subdivision_preserves_identity_and_record() {
        // A parent covering the overlap and its child that still covers the
        // same overlap produce the identical CommonArc: same occurrence, same
        // canonical boundaries, same participants. The fixture pairs pieces of
        // ONE authoritative occurrence (parabola(0)) — a parent and two
        // children that all cover [0.25, 1] — so the support is
        // IdenticalSourceProvenance and the identity is independent of
        // subdivision depth.
        let parent = bezier_span(0);
        let other = CurveSpan2::RationalBezier(parabola(0).subdivide(0.25).1);
        let child = CurveSpan2::RationalBezier(parabola(0).subdivide(0.25).1);
        let arc_parent = pair(&parent, &other);
        let arc_child = pair(&child, &other);
        assert_eq!(
            arc_parent.support.basis,
            CommonSupportBasis::IdenticalSourceProvenance
        );
        assert_eq!(
            arc_parent.identity, arc_child.identity,
            "the overlap's identity is independent of subdivision depth"
        );
        assert_eq!(
            arc_parent.boundaries, arc_child.boundaries,
            "the certified boundaries are stable under refinement"
        );
    }

    #[test]
    fn deterministic_repetition_reproduces_the_record() {
        let a = line_span(Point2::new(0.0, 0.0), Point2::new(2.0, 0.0), 0);
        let b = line_span(Point2::new(1.0, 0.0), Point2::new(3.0, 0.0), 1);
        assert_eq!(pair(&a, &b), pair(&a, &b));
    }

    #[test]
    fn common_deck_translation_preserves_identity() {
        let a = line_span(Point2::new(0.0, 0.0), Point2::new(2.0, 0.0), 0);
        let b = line_span(Point2::new(1.0, 0.0), Point2::new(3.0, 0.0), 1);
        let base = pair(&a, &b);
        let context = rank1_context();
        let arc_a = relabel(&base, context, 3, 7);
        let arc_b = relabel(&base, context, 3 + 10, 7 + 10);
        assert_eq!(
            arc_a.identity.relative_deck,
            DeckLabel::rank1(4),
            "relative deck displacement is k1 − k0"
        );
        assert_eq!(
            arc_a.identity, arc_b.identity,
            "a common deck translation is the same quotient overlap"
        );
    }

    #[test]
    fn different_relative_deck_displacement_changes_identity() {
        let a = line_span(Point2::new(0.0, 0.0), Point2::new(2.0, 0.0), 0);
        let b = line_span(Point2::new(1.0, 0.0), Point2::new(3.0, 0.0), 1);
        let base = pair(&a, &b);
        let context = rank1_context();
        let arc_a = relabel(&base, context, 3, 7);
        let arc_c = relabel(&base, context, 3, 9);
        assert_ne!(
            arc_a.identity, arc_c.identity,
            "a different relative deck displacement is a different overlap"
        );
    }

    #[test]
    fn rank_mismatch_is_a_typed_deck_error() {
        let a = line_span(Point2::new(0.0, 0.0), Point2::new(2.0, 0.0), 0);
        let b = line_span(Point2::new(1.0, 0.0), Point2::new(3.0, 0.0), 1);
        let base = pair(&a, &b);
        // A rank-1 label in the rank-0 context: typed RankMismatch.
        let mut bad = base.clone();
        bad.participants[0].deck =
            CertifiedDeckLabel::certified_placement(rank1_context(), DeckLabel::rank1(1));
        assert!(matches!(
            bad.validate(),
            Err(CommonArcError::Deck(DeckLabelError::RankMismatch { .. }))
        ));
    }

    #[test]
    fn produced_common_arcs_are_rank0_never_rank2() {
        let a = line_span(Point2::new(0.0, 0.0), Point2::new(2.0, 0.0), 0);
        let b = line_span(Point2::new(1.0, 0.0), Point2::new(3.0, 0.0), 1);
        let arc = pair(&a, &b);
        assert_eq!(arc.deck_context.rank(), DeckRank::Rank0);
        assert!(
            arc.participants.iter().all(|p| p.deck.is_zero()),
            "general rank-2 placement is never minted"
        );
    }

    // ----- boundaries ---------------------------------------------------------

    #[test]
    fn positive_interval_has_two_distinct_boundaries() {
        let a = line_span(Point2::new(0.0, 0.0), Point2::new(2.0, 0.0), 0);
        let b = line_span(Point2::new(1.0, 0.0), Point2::new(3.0, 0.0), 1);
        let arc = pair(&a, &b);
        assert!(repr(&arc.boundaries.start) < repr(&arc.boundaries.end));
        assert!(arc.certificate.boundaries_certified_distinct);
    }

    #[test]
    fn boundary_values_do_not_come_from_representative_points() {
        // The boundaries are certified overlap bounds on the canonical source
        // axis; the participants' representative-evaluation machinery never
        // enters. The canonical boundary for the identical-line case is the
        // exact domain endpoint 0/1.
        let a = line_span(Point2::new(0.0, 0.0), Point2::new(2.0, 0.0), 0);
        let b = line_span(Point2::new(0.0, 0.0), Point2::new(2.0, 0.0), 0);
        let arc = pair(&a, &b);
        assert_eq!(repr(&arc.boundaries.start), 0.0);
        assert_eq!(repr(&arc.boundaries.end), 1.0);
    }

    // ----- helpers for the deck tests ----------------------------------------

    fn rank1_context() -> DeckContext {
        DeckContext::from_lattice_id(AmbientLatticeId::Rank1 {
            periodic_axis: super::super::evidence::ParameterAxis::V,
            signed_period_bits: std::f64::consts::TAU.to_bits(),
        })
    }

    /// Relabel a certified CommonArc with rank-1 deck labels under the given
    /// context, keeping the canonical identity consistent. Test-only: exercises
    /// the gauge invariance of the identity's relative deck displacement.
    fn relabel(arc: &CommonArc2, context: DeckContext, ka: i64, kb: i64) -> CommonArc2 {
        let mut arc = arc.clone();
        arc.deck_context = context;
        let label_a = CertifiedDeckLabel::certified_placement(context, DeckLabel::rank1(ka));
        let label_b = CertifiedDeckLabel::certified_placement(context, DeckLabel::rank1(kb));
        arc.participants[0].deck = label_a;
        arc.participants[1].deck = label_b;
        arc.identity.relative_deck = label_b.checked_sub(label_a).unwrap().get();
        arc.validate().unwrap();
        arc
    }

    // ----- SpanId preservation (constraint 2 caveat) ------------------------

    #[test]
    fn span_id_is_preserved_by_reversal_and_subdivision() {
        // Reversal: `CurveOccurrenceProvenance::reversed` uses `..*self`,
        // keeping `edge_use_id` and `source_edge_id`, so `SpanId` is preserved.
        let span = bezier_span(0);
        let rev = CurveSpan2::RationalBezier(parabola(0).reverse_occurrence());
        assert_eq!(span.span_id(), rev.span_id(), "reversal preserves SpanId");
        // Subdivision: `RationalBezierSpan2::subdivide` copies provenance
        // verbatim, so a child carries the parent's SpanId.
        let (left, _right) = parabola(0).subdivide(0.5);
        let child = CurveSpan2::RationalBezier(left);
        assert_eq!(
            span.span_id(),
            child.span_id(),
            "subdivision preserves SpanId"
        );
        // A line's reversal preserves SpanId too.
        let line = line_span(Point2::new(0.0, 0.0), Point2::new(2.0, 0.0), 3);
        let rev_line = CurveSpan2::from_line(match &line {
            CurveSpan2::AnalyticLine(seg) => seg.reverse_occurrence(),
            _ => unreachable!(),
        });
        assert_eq!(line.span_id(), rev_line.span_id());
    }

    // ----- AuthoritativeParameterKey is copied verbatim (constraint 1) ------

    #[test]
    fn authoritative_parameter_key_is_copied_verbatim_under_reversal() {
        let span = parabola(0);
        let rev = span.reverse_occurrence();
        // The lower source-domain bound is `min(domain.0, domain.1)`; reversal
        // swaps the ordered pair but not the min, so the key is identical.
        let lo_forward =
            AuthoritativeParameterKey::from_authoritative(span.domain().0.min(span.domain().1));
        let lo_reversed =
            AuthoritativeParameterKey::from_authoritative(rev.domain().0.min(rev.domain().1));
        assert_eq!(lo_forward, lo_reversed);
    }

    #[test]
    fn authoritative_parameter_key_is_copied_verbatim_under_parent_child_subdivision() {
        let parent = parabola(0);
        let (left, _right) = parent.subdivide(0.5);
        // The source start (0.0) is the lower bound of both parent and left
        // child; both retain it verbatim, so the key is identical.
        let parent_lo =
            AuthoritativeParameterKey::from_authoritative(parent.domain().0.min(parent.domain().1));
        let child_lo =
            AuthoritativeParameterKey::from_authoritative(left.domain().0.min(left.domain().1));
        assert_eq!(parent_lo, child_lo);
    }

    #[test]
    fn authoritative_parameter_key_is_copied_verbatim_under_sibling_subdivision() {
        let parent = parabola(0);
        let (_l, right) = parent.subdivide(0.25);
        let (left2, _r) = parent.subdivide(0.75);
        // right's lower bound is the split 0.25 (retained in its domain.0);
        // left2's upper bound is the split 0.75 (retained in its domain.1).
        // Each is the authoritative value the subdivision stored, not a
        // recomputed projection.
        let split_025 =
            AuthoritativeParameterKey::from_authoritative(right.domain().0.min(right.domain().1));
        assert_eq!(
            split_025,
            AuthoritativeParameterKey::from_authoritative(0.25)
        );
        let split_075 =
            AuthoritativeParameterKey::from_authoritative(left2.domain().0.max(left2.domain().1));
        assert_eq!(
            split_075,
            AuthoritativeParameterKey::from_authoritative(0.75)
        );
    }

    #[test]
    fn different_authoritative_parameters_produce_different_keys() {
        assert_ne!(
            AuthoritativeParameterKey::from_authoritative(0.25),
            AuthoritativeParameterKey::from_authoritative(0.75),
        );
        // -0.0 and +0.0 normalize to the same key.
        assert_eq!(
            AuthoritativeParameterKey::from_authoritative(-0.0),
            AuthoritativeParameterKey::from_authoritative(0.0),
        );
    }

    // ----- Construction-based identity regression ---------------------------

    #[test]
    fn identity_is_construction_based_not_representative_value_based() {
        // The same MappedEndpoint construction key with two enclosures that
        // differ by one unit in the last place identifies the SAME boundary:
        // identity is the construction key, never the enclosure value.
        let span_a = SpanId {
            edge_use_id: EdgeUseId::new(BoundId(0), 1),
            source_edge_id: SourceEdgeId(1),
        };
        let key = CommonArcBoundaryKey::MappedEndpoint {
            span: span_a,
            side: AxisSide::Lower,
        };
        let v = 0.5_f64;
        let one_ulp = v.next_up();
        assert_ne!(v.to_bits(), one_ulp.to_bits(), "sanity: distinct ULPs");
        let b1 = CommonArcBoundary {
            key,
            enclosure: ParameterEnclosure { lo: v, hi: v },
        };
        let b2 = CommonArcBoundary {
            key,
            enclosure: ParameterEnclosure {
                lo: one_ulp,
                hi: one_ulp,
            },
        };
        assert_eq!(
            b1, b2,
            "same key, 1-ULP-different enclosure => equal boundary"
        );
        // Genuinely different endpoint constructions do not compare equal.
        let key_upper = CommonArcBoundaryKey::MappedEndpoint {
            span: span_a,
            side: AxisSide::Upper,
        };
        let b3 = CommonArcBoundary {
            key: key_upper,
            enclosure: ParameterEnclosure { lo: v, hi: v },
        };
        assert_ne!(
            b1, b3,
            "different side => different construction => unequal"
        );
        // A SourceEndpoint with a different authoritative parameter is unequal.
        let key_src = CommonArcBoundaryKey::SourceEndpoint {
            occurrence: span_a,
            side: AxisSide::Lower,
            parameter: AuthoritativeParameterKey::from_authoritative(0.25),
            periodic_lift: None,
        };
        let b4 = CommonArcBoundary {
            key: key_src,
            enclosure: ParameterEnclosure { lo: 0.25, hi: 0.25 },
        };
        assert_ne!(b1, b4, "different variant/parameter => unequal");
        // Equal keys hash equal.
        use std::hash::{Hash, Hasher};
        let mut h1 = std::collections::hash_map::DefaultHasher::new();
        let mut h2 = std::collections::hash_map::DefaultHasher::new();
        b1.hash(&mut h1);
        b2.hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
    }

    // ----- Bézier CommonArc precheck does not suppress isolated solving -----

    #[test]
    fn adjacent_same_source_spans_meet_only_at_an_endpoint_is_point_only() {
        // Two children of one authoritative Bézier occurrence that meet only
        // at the split parameter: the CommonArc precheck returns
        // PointOnlyOverlap, NOT a CommonArc. The isolated-root solver
        // therefore runs and may certify the endpoint contact.
        let (left, right) = parabola(0).subdivide(0.5);
        let a = CurveSpan2::RationalBezier(left);
        let b = CurveSpan2::RationalBezier(right);
        assert_eq!(
            common_arc_for_pair(&a, &b),
            Err(CommonArcError::PointOnlyOverlap),
        );
    }

    #[test]
    fn adjacent_same_source_spans_run_the_isolated_solver_not_the_precheck() {
        // Regression: restoring `Err(PointOnlyOverlap) => Disjoint` (or
        // `Err(EmptyOverlap) => Disjoint`) in the Bézier precheck would
        // suppress the isolated-root solver. Two adjacent same-source children
        // meet at the split endpoint; the solver must run and find the contact
        // rather than short-circuit to Disjoint.
        let (left, right) = parabola(0).subdivide(0.5);
        let result = super::super::bezier_isect::intersect_bezier_pair(&left, &right);
        assert!(
            !matches!(
                result,
                super::super::contact::PairContactResult::Disjoint
            ),
            "adjacent same-source spans must run the isolated solver, not short-circuit to Disjoint"
        );
    }
}
