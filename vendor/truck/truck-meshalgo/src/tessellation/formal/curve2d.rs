//! Developed planar curves: the authoritative source occurrence, carried in
//! the plane's own native chart.
//!
//! # What a "developed" curve is
//!
//! An occurrence of a source edge, after the composed traversal sense has
//! been applied exactly once, represented as an analytic curve in the
//! certified planar chart. It is *not* a polygonal approximation: a line
//! carries its endpoints and an arc carries its center, parameter basis and
//! the source's own unwrapped parameter interval, so every later stage can
//! evaluate it exactly and never needs to weld coordinates.
//!
//! # What is preserved here
//!
//! The generic arrangement stages below ([`super::xmonotone`]) must never
//! reconstruct source incidence, vertex identity or edge identity from
//! coordinates. Every occurrence therefore carries its complete
//! [`CurveOccurrenceProvenance`]; the pieces a decomposition produces inherit
//! it verbatim.
//!
//! # The parameter basis is authoritative, not inferred
//!
//! An arc's `t0`/`t1` are the source curve's own trimmed interval in the
//! curve's own parameter direction, with the selected edge-use reversal
//! applied exactly once. This module never reduces an endpoint modulo `TAU`,
//! never picks the shortest sweep, never infers a full circle from coincident
//! endpoints, and never merges source vertices by proximity.

use super::super::source_evidence::{BoundId, EdgeUseId, SourceVertexKey};
use truck_geometry::prelude::{InnerSpace, Point2, Vector2};

/// The document entity id of the source face, when the adapter retained it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SourceFaceId(pub u64);

/// Position in the shell's edge table. Not a document entity id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SourceEdgeId(pub usize);

/// The document entity id of the source curve, when the adapter retained it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SourceEntityId(pub u64);

/// The complete provenance of one source curve occurrence.
///
/// Every field names an identity the source itself declares. Two occurrences
/// at identical coordinates are still distinct if their ids differ, and a
/// relation built from coordinate proximity is a relation built on the
/// exporter rather than on the file — which is exactly the inference the
/// arrangement must not make.
///
/// `source_face_id` and `source_curve_entity_id` are optional because the
/// importer seam itself retains them optionally (`Option<u64>`); absence is
/// preserved, never invented. The bound, edge-use, source-edge and vertex
/// identities are always available at this seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CurveOccurrenceProvenance {
    /// The document entity id of the source face.
    pub source_face_id: Option<SourceFaceId>,
    /// Which bound of the face, in source order.
    pub bound_id: BoundId,
    /// Which edge use of that bound, in source traversal order.
    pub edge_use_id: EdgeUseId,
    /// Which edge in the shell's edge table.
    pub source_edge_id: SourceEdgeId,
    /// The source vertex the occurrence starts at, in traversal order.
    pub start_vertex_id: SourceVertexKey,
    /// The source vertex the occurrence ends at, in traversal order.
    pub end_vertex_id: SourceVertexKey,
    /// The document entity id of the curve, when retained.
    pub source_curve_entity_id: Option<SourceEntityId>,
}

impl CurveOccurrenceProvenance {
    /// The occurrence with its traversal endpoints swapped.
    ///
    /// The identity-level mirror of [`DirectedCircularArc2::reverse_occurrence`]
    /// and [`LineSegment2::reverse_occurrence`]: start and end vertex ids
    /// exchange places, nothing else changes.
    pub fn reversed(&self) -> Self {
        Self {
            start_vertex_id: self.end_vertex_id,
            end_vertex_id: self.start_vertex_id,
            ..*self
        }
    }
}

/// A straight segment in the plane's native chart, in traversal order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineSegment2 {
    /// The traversal start.
    pub start: Point2,
    /// The traversal end.
    pub end: Point2,
    /// The source occurrence this segment represents.
    pub provenance: CurveOccurrenceProvenance,
}

impl LineSegment2 {
    /// The traversal direction `end - start`.
    pub fn direction(&self) -> Vector2 {
        self.end - self.start
    }

    /// The squared length of the traversal displacement.
    pub fn length_squared(&self) -> f64 {
        self.direction().magnitude2()
    }

    /// Whether both endpoints coincide exactly.
    ///
    /// Exact, bitwise: a segment whose endpoints are *almost* equal still
    /// spans a positive length and is not degenerate.
    pub fn is_degenerate(&self) -> bool {
        self.start == self.end
    }

    /// Evaluate the segment at `t in [0, 1]`, linearly.
    pub fn point_at(&self, t: f64) -> Point2 {
        self.start + t * self.direction()
    }

    /// The same segment traversed the other way.
    pub fn reverse_occurrence(&self) -> Self {
        Self {
            start: self.end,
            end: self.start,
            provenance: self.provenance.reversed(),
        }
    }
}

/// A directed circular arc in the plane's native chart.
///
/// The parameterization is the authoritative source basis:
///
/// ```text
/// point(t) = center + cos_basis * cos(t) + sin_basis * sin(t)
/// ```
///
/// `t0`/`t1` are the source's own trimmed parameter interval in the curve's
/// own direction, with the selected edge-use reversal applied exactly once.
/// They are deliberately not reduced modulo `TAU`: a seam-crossing arc's
/// interval extends past `2π` (or below `0`), a negative sweep is
/// `t0 > t1`, and a full turn is an interval of width `TAU` — never an
/// inference from coincident endpoints.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectedCircularArc2 {
    /// The circle center.
    pub center: Point2,
    /// The transformed image of the unit circle's `e_x`: `point(t0)`'s
    /// cos-term direction.
    pub cos_basis: Vector2,
    /// The transformed image of the unit circle's `e_y`.
    pub sin_basis: Vector2,
    /// The authoritative unwrapped start parameter, selected traversal
    /// direction.
    pub t0: f64,
    /// The authoritative unwrapped end parameter, selected traversal
    /// direction. `t0 > t1` is a negative sweep.
    pub t1: f64,
    /// The source occurrence this arc represents.
    pub provenance: CurveOccurrenceProvenance,
}

impl DirectedCircularArc2 {
    /// The point at parameter `t`.
    pub fn point_at(&self, t: f64) -> Point2 {
        self.center + t.cos() * self.cos_basis + t.sin() * self.sin_basis
    }

    /// The velocity at parameter `t`.
    pub fn tangent_at(&self, t: f64) -> Vector2 {
        -t.sin() * self.cos_basis + t.cos() * self.sin_basis
    }

    /// The traversal start point, `point_at(t0)`.
    pub fn start_point(&self) -> Point2 {
        self.point_at(self.t0)
    }

    /// The traversal end point, `point_at(t1)`.
    pub fn end_point(&self) -> Point2 {
        self.point_at(self.t1)
    }

    /// The signed sweep `t1 - t0`, in the selected traversal direction.
    ///
    /// Never reduced modulo `TAU`: the authoritative unwrapped interval is
    /// what the sweep means.
    pub fn sweep(&self) -> f64 {
        self.t1 - self.t0
    }

    /// The squared radius `|cos_basis|^2`.
    ///
    /// For a certified circle the sine basis has the same squared length;
    /// this accessor reports the cosine basis's value, and the certification
    /// of equality belongs to the admission stage, not to this record.
    pub fn radius_squared(&self) -> f64 {
        self.cos_basis.magnitude2()
    }

    /// The same arc traversed the other way.
    ///
    /// Swaps `t0`/`t1` and the occurrence's start/end vertex ids; the source
    /// parameter basis is not mutated.
    pub fn reverse_occurrence(&self) -> Self {
        Self {
            t0: self.t1,
            t1: self.t0,
            provenance: self.provenance.reversed(),
            ..*self
        }
    }
}

/// A developed planar curve occurrence, in the plane's native chart.
///
/// The union the generic arrangement stages dispatch on. Future curve
/// families (rational Bézier spans, affine conics, general certified
/// parametric pcurves) join as new variants here; the arrangement topology
/// code above them dispatches on this enum and never matches geometry
/// directly.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DevelopedCurve2D {
    /// A straight segment.
    Line(LineSegment2),
    /// A directed circular arc.
    CircularArc(DirectedCircularArc2),
}

impl DevelopedCurve2D {
    /// The source occurrence provenance, whichever family this is.
    pub fn provenance(&self) -> &CurveOccurrenceProvenance {
        match self {
            Self::Line(segment) => &segment.provenance,
            Self::CircularArc(arc) => &arc.provenance,
        }
    }

    /// The traversal start point.
    pub fn start_point(&self) -> Point2 {
        match self {
            Self::Line(segment) => segment.start,
            Self::CircularArc(arc) => arc.start_point(),
        }
    }

    /// The traversal end point.
    pub fn end_point(&self) -> Point2 {
        match self {
            Self::Line(segment) => segment.end,
            Self::CircularArc(arc) => arc.end_point(),
        }
    }

    /// The same occurrence traversed the other way.
    pub fn reverse_occurrence(&self) -> Self {
        match self {
            Self::Line(segment) => Self::Line(segment.reverse_occurrence()),
            Self::CircularArc(arc) => Self::CircularArc(arc.reverse_occurrence()),
        }
    }
}
