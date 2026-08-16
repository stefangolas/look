//! Generic curve-span contract: the family-independent surface ARR-003 consumes.
//!
//! ARR-002's [`super::curve2d::DevelopedCurve2D`] is line/circle only, and every
//! arrangement-facing consumer ([`super::xmonotone`], [`super::intersection`])
//! matches on its two variants. GEN-001 inserts [`CurveSpan2`] above it: one
//! family instance carrying its authoritative source occurrence, parameter
//! domain and traversal, behind a contract of certified operations. ARR-003
//! calls the contract; it never `match`es on the family variant to decide
//! topology, and it never learns which solver (analytic fast path or generic
//! Bézier isolation) produced a result.
//!
//! Analytic lines and circles remain optimized implementations of the contract.
//! The rational-Bézier variant is declared here so the contract's variant set is
//! frozen before any code depends on it; its constructor and certified
//! operations land in GEN-001B.
//!
//! # Identity discipline
//!
//! A [`CurveSpan2`] is one family instance. A *piece* of it over a sub-interval
//! — the arrangement's actual input — is a [`super::contact::BranchIncidence`]:
//! span + certified parameter enclosure + branch germ + deck label. Coordinates
//! appear only as representative evaluation hints; identity is the span id and
//! source occurrence, never a point.

use super::super::source_evidence::EdgeUseId;
use super::bezier::RationalBezierSpan2;
use super::curve2d::{CurveOccurrenceProvenance, DirectedCircularArc2, LineSegment2, SourceEdgeId};

/// The stable identity of one curve-span family instance.
///
/// Two arrangement pieces cut from the same source occurrence share a span id;
/// what distinguishes them is their parameter interval (carried on
/// [`super::contact::BranchIncidence`]), not the span. Built from the source
/// occurrence's edge-use and edge identities, never from coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SpanId {
    /// The source edge use this span belongs to.
    pub edge_use_id: EdgeUseId,
    /// The source edge in the shell's edge table.
    pub source_edge_id: SourceEdgeId,
}

impl SpanId {
    /// The span id of a source occurrence.
    pub fn from_occurrence(provenance: &CurveOccurrenceProvenance) -> Self {
        Self {
            edge_use_id: provenance.edge_use_id,
            source_edge_id: provenance.source_edge_id,
        }
    }
}

/// The analytic family of a span, for fast-path routing only.
///
/// ARR-003 does not dispatch topology on this. The generic intersection layer
/// uses it to route to the analytic solvers as accelerators and to fall back to
/// generic Bézier isolation for [`Generic`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FastPath {
    /// A straight segment.
    Line,
    /// A directed circular arc.
    CircularArc,
    /// A generic rational Bézier span (no analytic fast path).
    Generic,
}

/// The certified local behavior of one branch at an event parameter.
///
/// Replaces ARR-002's implicit "the first derivative is nonzero everywhere"
/// assumption ([`super::xmonotone::MonotoneKind`] is regular-only). A zero first
/// derivative is not invalid; it is a signal to read the next nonzero jet:
/// `k = min { j >= 1 : C^(j)(t0) != 0 }`.
///
/// The parity of `first_nonzero_order` decides the pairwise contact: odd orders
/// are transverse-like (the vertical order swaps across the event), even orders
/// are tangent-like (it is preserved). GEN-001C derives the crossing
/// classification from the germ configuration; GEN-001A carries the germ only.
///
/// No singular branch may be silently treated as an ordinary tangent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchGerm {
    /// A regular branch: `first_nonzero_order == 1`.
    Regular,
    /// A stationary branch whose first `k - 1` derivatives vanish and whose
    /// `k`-th is certified nonzero, `k >= 2`.
    StationaryRegular {
        /// The first nonzero derivative order `k >= 2`.
        first_nonzero_order: u8,
    },
    /// A candidate cusp: the parametrization is singular in a way that may
    /// collapse the tangent direction. Classified further before any topology.
    CuspCandidate,
    /// A singular branch (e.g. a collapsed-stratum attachment) whose local
    /// topology is not that of a regular branch.
    Singular,
    /// The branch behavior could not be certified at the declared policy.
    Unresolved,
}

impl BranchGerm {
    /// A short stable tag, for diagnostics.
    pub fn tag(self) -> &'static str {
        match self {
            Self::Regular => "germ_regular",
            Self::StationaryRegular { .. } => "germ_stationary_regular",
            Self::CuspCandidate => "germ_cusp_candidate",
            Self::Singular => "germ_singular",
            Self::Unresolved => "germ_unresolved",
        }
    }

    /// Whether this germ is a certified regular-ish branch (order known).
    pub fn is_resolved(self) -> bool {
        matches!(self, Self::Regular | Self::StationaryRegular { .. })
    }
}

/// The rational-Bézier span type and its certified substrate live in
/// [`super::bezier`] (GEN-001B); the type is re-used here as the third
/// [`CurveSpan2`] variant. Its Bernstein fields are `pub(crate)`, so the solver
/// strategy never reaches the arrangement interface.
///
/// A developed curve-span family instance, in the plane's native chart.
///
/// The family-independent surface the generic arrangement consumes. Each variant
/// is an optimized implementation of the same contract; consumers call the
/// methods below and never `match` on the variant to decide topology.
#[derive(Debug, Clone, PartialEq)]
pub enum CurveSpan2 {
    /// An analytic straight segment.
    AnalyticLine(LineSegment2),
    /// An analytic directed circular arc.
    AnalyticCircularArc(DirectedCircularArc2),
    /// A homogeneous rational Bézier span (populated in GEN-001B).
    RationalBezier(RationalBezierSpan2),
}

impl CurveSpan2 {
    /// Wrap an ARR-002 developed line occurrence.
    pub fn from_line(segment: LineSegment2) -> Self {
        Self::AnalyticLine(segment)
    }

    /// Wrap an ARR-002 developed circular-arc occurrence.
    pub fn from_circular_arc(arc: DirectedCircularArc2) -> Self {
        Self::AnalyticCircularArc(arc)
    }

    /// The stable span identity.
    pub fn span_id(&self) -> SpanId {
        SpanId::from_occurrence(self.provenance())
    }

    /// The source occurrence provenance, whichever family this is.
    pub fn provenance(&self) -> &CurveOccurrenceProvenance {
        match self {
            Self::AnalyticLine(segment) => &segment.provenance,
            Self::AnalyticCircularArc(arc) => &arc.provenance,
            Self::RationalBezier(bezier) => &bezier.provenance,
        }
    }

    /// The analytic fast path, for accelerator routing only.
    pub fn fast_path(&self) -> FastPath {
        match self {
            Self::AnalyticLine(_) => FastPath::Line,
            Self::AnalyticCircularArc(_) => FastPath::CircularArc,
            Self::RationalBezier(_) => FastPath::Generic,
        }
    }

    /// A deterministic debug/serialization tag.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::AnalyticLine(_) => "span_analytic_line",
            Self::AnalyticCircularArc(_) => "span_analytic_circular_arc",
            Self::RationalBezier(_) => "span_rational_bezier",
        }
    }

    /// The authoritative source parameter domain, in traversal order.
    ///
    /// For a line this is `(0.0, 1.0)`; for a circular arc the unwrapped
    /// `(t0, t1)`; for a Bézier span its declared domain.
    pub fn authoritative_domain(&self) -> (f64, f64) {
        match self {
            Self::AnalyticLine(_) => (0.0, 1.0),
            Self::AnalyticCircularArc(arc) => (arc.t0, arc.t1),
            Self::RationalBezier(bezier) => bezier.domain,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::source_evidence::{BoundId, EdgeUseId, SourceVertexKey};
    use super::super::curve2d::{SourceEdgeId, SourceEntityId, SourceFaceId};
    use super::*;
    use truck_geometry::prelude::{Point2, Vector2};

    fn provenance() -> CurveOccurrenceProvenance {
        CurveOccurrenceProvenance {
            source_face_id: Some(SourceFaceId(7)),
            bound_id: BoundId(0),
            edge_use_id: EdgeUseId::new(BoundId(0), 3),
            source_edge_id: SourceEdgeId(11),
            start_vertex_id: SourceVertexKey::ShellVertex(1),
            end_vertex_id: SourceVertexKey::ShellVertex(2),
            source_curve_entity_id: Some(SourceEntityId(99)),
        }
    }

    #[test]
    fn line_span_identity_and_domain() {
        let span = CurveSpan2::from_line(LineSegment2 {
            start: Point2::new(0.0, 0.0),
            end: Point2::new(1.0, 0.0),
            provenance: provenance(),
        });
        assert_eq!(span.fast_path(), FastPath::Line);
        assert_eq!(span.tag(), "span_analytic_line");
        assert_eq!(span.authoritative_domain(), (0.0, 1.0));
        assert_eq!(span.span_id(), SpanId::from_occurrence(&provenance()));
        assert_eq!(span.span_id().source_edge_id, SourceEdgeId(11));
    }

    #[test]
    fn arc_span_identity_and_domain() {
        let arc = DirectedCircularArc2 {
            center: Point2::new(0.0, 0.0),
            cos_basis: Vector2::new(1.0, 0.0),
            sin_basis: Vector2::new(0.0, 1.0),
            t0: 0.0,
            t1: std::f64::consts::FRAC_PI_2,
            provenance: provenance(),
        };
        let span = CurveSpan2::from_circular_arc(arc);
        assert_eq!(span.fast_path(), FastPath::CircularArc);
        assert_eq!(span.tag(), "span_analytic_circular_arc");
        assert_eq!(
            span.authoritative_domain(),
            (0.0, std::f64::consts::FRAC_PI_2)
        );
    }

    #[test]
    fn germ_resolves_only_for_known_order() {
        assert!(BranchGerm::Regular.is_resolved());
        assert!(BranchGerm::StationaryRegular {
            first_nonzero_order: 3
        }
        .is_resolved());
        assert!(!BranchGerm::CuspCandidate.is_resolved());
        assert!(!BranchGerm::Singular.is_resolved());
        assert!(!BranchGerm::Unresolved.is_resolved());
    }

    #[test]
    fn span_id_is_independent_of_coordinates() {
        // The same occurrence at different coordinates shares the span id; two
        // distinct occurrences do not, even at coincident coordinates.
        let mut p = provenance();
        let id_a = SpanId::from_occurrence(&p);
        p.source_edge_id = SourceEdgeId(12);
        let id_b = SpanId::from_occurrence(&p);
        assert_ne!(id_a, id_b, "span id follows the source edge, not the point");
    }
}
