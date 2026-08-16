//! The source-evidence seam: what a face's STEP provenance still says by the
//! time tessellation sees it.
//!
//! This is the input type of the formal face pipeline (`FORMAL_SYSTEM.md`
//! Def. 1), built by an adapter from whichever shell representation the caller
//! holds. It exists separately from the geometry it describes because the
//! legacy path *consumed* its evidence: a bound's edge uses are flattened into
//! one undifferentiated point vector, and edge-use orientation is applied as
//! `curve.inverse()` and then dropped.
//!
//! # Step 0: this type is built and reported, not consumed
//!
//! Nothing here affects a production decision. It is constructed beside the
//! legacy boundary construction so that the evidence actually available at this
//! seam can be *counted* on the corpus before the pipeline that depends on it
//! is written. See `TRUCK_PROBE_EVIDENCE`.
//!
//! # Orientation: one factor is retained, three are erased
//!
//! `FORMAL_SYSTEM.md` §V defines the normalized traversal sign as the product
//! `s_f · s_b · s_o · s_e · s_c`, and makes the STEP adapter — not the atlas —
//! responsible for mapping Boolean fields to `±1`. The atlas needs the product.
//! What it also needs is to know whether the product is *computable*, and today
//! the factors arrive in two states:
//!
//! - `s_b · s_o` is **retained**, pre-composed into
//!   `CompressedEdgeIndex.orientation` (`truck-stepio` `convert.rs:244`,
//!   `oriented_edge.orientation == ori`), with the bound's own reversal applied
//!   to the edge-use *order*. The face use's own flag is retained too.
//! - `s_e`, `s_c` and `s_f` are **history-erased**: each was folded into
//!   converted geometry — the curve for the first two, the surface for the
//!   third — and no Boolean survives to this layer.
//!
//! So on the compressed path no face has a computable normalized sign. That is
//! the measurement step 0 exists to make, and it is why the rewrite cannot
//! assign certified physical material sides from this input alone.
//!
//! # Two things this module refuses to say
//!
//! **It does not summarize.** An earlier draft carried a single "weakest
//! assumption", which is the epistemic compression the rewrite exists to
//! remove: repairing `s_f` would have flipped the record to established while
//! `s_e` and `s_c` were still nothing but upstream assertions. Every factor is
//! its own field, and there is no authoritative aggregate status.
//!
//! **It does not claim a factor was applied.** `HistoryErased` says the
//! conversion consumed the value, not that the value was `false`. This layer
//! cannot know whether a given face had `same_sense == false`; it knows only
//! that it would no longer be able to tell. Note that the `s_f` mechanism —
//! `surface.invert()` — is independently recorded as breaking curve-on-surface
//! incidence rather than only reversing the parameterization, which is what
//! [`ErasedOrientationMechanism::is_suspect`] marks.
//!
//! **Contracts:** carries the evidence `TOP-005` requires (effective traversal
//! is the composition of face, bound, oriented-edge and edge-curve
//! orientation), and measures how much of it survives. Checking it against
//! source incidence is the rest of `TOP-005` and is not done here. `TOP-001`
//! identity is carried at the point of use.

/// Which bound of a face, in source order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BoundId(pub usize);

/// Which edge use of a face, in source order.
///
/// Composite rather than a bare counter: every bound restarts its own indexing,
/// so a face-wide key must name the bound too. Mirrors the source structure,
/// which also makes a diagnostic locate itself without a lookup.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EdgeUseId {
    /// The bound this use belongs to.
    pub bound: BoundId,
    /// Position within that bound, in source traversal order.
    pub index: usize,
}

impl EdgeUseId {
    /// Names one edge use of one bound.
    pub const fn new(bound: BoundId, index: usize) -> Self {
        Self { bound, index }
    }
}

/// The synthetic source identity of one presented boundary edge use.
///
/// This is the identity the legacy path can still recover at
/// `create_boundary`/`create_edge` time, before the wire's polylines are
/// flattened: the bound's position in `face.boundaries`, the edge use's
/// position within that bound, and the composed `s_b · s_o` orientation that
/// was applied to the curve. No STEP edge entity survives this far; the
/// `(BoundId, EdgeUseId)` pair is the identity PLANAR-C's arrangement layer
/// needs, so it is carried rather than reconstructed.
///
/// This is deliberately not [`SourceEdgeUseInput`]: that type describes the
/// evidence seam for the formal pipeline, while this is the light provenance
/// the legacy boundary path records. They answer different questions and are
/// never mixed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SourceEdgeUse {
    /// The bound this use belongs to.
    pub bound: BoundId,
    /// Position within that bound, in source traversal order.
    pub index: usize,
    /// Whether the presented curve runs in the bound's own orientation.
    /// `false` means `curve.inverse()` was applied (`s_b · s_o` = reversed).
    pub orientation: bool,
}

impl SourceEdgeUse {
    /// The [`EdgeUseId`] this use names.
    pub const fn edge_use_id(self) -> EdgeUseId {
        EdgeUseId::new(self.bound, self.index)
    }
}

/// A source vertex identity.
///
/// Never a coordinate. Two `VERTEX_POINT` entities at the same position are two
/// vertices, and a relation built from their coincidence is a relation built on
/// the exporter rather than on the file — which is exactly the inference the
/// deck solve must not make.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SourceVertexKey {
    /// A position in the shell's vertex table. Stable within one shell, which
    /// is the scope every relation this feeds is built in.
    ShellVertex(usize),
    /// The representation offers no vertex identity at this seam.
    Absent,
}

impl SourceVertexKey {
    /// Whether this key can support a source-incidence relation.
    pub const fn is_identified(self) -> bool {
        matches!(self, Self::ShellVertex(_))
    }
}

/// Which source field a retained orientation factor came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OrientationOrigin {
    /// `CompressedFace::orientation`, the face use's own flag.
    CompressedFaceOrientation,
    /// `FACE_BOUND.orientation × ORIENTED_EDGE.orientation`, composed by the
    /// STEP converter before this layer sees it.
    BoundTimesOrientedEdge,
    /// `EDGE_CURVE.same_sense`.
    EdgeCurveSameSense,
    /// The direction of the selected curve parameterization.
    SelectedCurveParameterDirection,
    /// `FACE_SURFACE.same_sense`.
    FaceSurfaceSameSense,
}

/// How a factor's Boolean value came to be unavailable here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ErasedOrientationMechanism {
    /// `EDGE_CURVE.same_sense` was applied by inverting the curve at
    /// conversion, so the polyline arriving here is already in the intended
    /// direction.
    EdgeCurveSenseFoldedIntoConvertedCurve,
    /// The selected curve parameterization's direction was folded into the
    /// converted curve.
    SelectedCurveDirectionFoldedIntoConvertedCurve,
    /// `FACE_SURFACE.same_sense` was folded in by `surface.invert()` on the
    /// face's copy of its surface.
    FaceSurfaceSenseFoldedViaSurfaceInvert,
}

impl ErasedOrientationMechanism {
    /// Whether the mechanism that erased this factor is independently recorded
    /// as unsound.
    ///
    /// `surface.invert()` breaks curve-on-surface incidence rather than only
    /// reversing the parameterization; `TRUCK_NO_INVERT` disables it for
    /// diagnosis. The two curve-folding mechanisms are sound — the value is
    /// simply gone.
    pub const fn is_suspect(self) -> bool {
        matches!(self, Self::FaceSurfaceSenseFoldedViaSurfaceInvert)
    }

    /// The reported token.
    pub const fn tag(self) -> &'static str {
        match self {
            Self::EdgeCurveSenseFoldedIntoConvertedCurve => "folded_into_curve",
            Self::SelectedCurveDirectionFoldedIntoConvertedCurve => "folded_into_curve",
            Self::FaceSurfaceSenseFoldedViaSurfaceInvert => "folded_via_surface_invert_suspect",
        }
    }
}

/// What became of one factor of the normalized traversal sign.
///
/// `Retained` carries the factor's **value**, not merely the claim that it is
/// readable: a state asserting a sign is available must contain the sign, or a
/// record can report itself complete while carrying nothing from which the
/// normalized sign could be computed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OrientationEvidence {
    /// Readable here, with its sign.
    Retained {
        /// `true` represents `+1`; `false` represents `-1`.
        forward: bool,
        /// Which source field it came from.
        origin: OrientationOrigin,
    },
    /// The factor was consumed into converted geometry and its original Boolean
    /// value is no longer available at this layer.
    ///
    /// This asserts the *erasure*, not the application: nothing here knows
    /// whether the flag was set on any particular face.
    HistoryErased {
        /// How it was consumed.
        mechanism: ErasedOrientationMechanism,
    },
    /// Neither retained nor known to have been consumed.
    Missing,
}

impl OrientationEvidence {
    /// The factor's sign, when it is actually carried.
    pub const fn sign(self) -> Option<bool> {
        match self {
            Self::Retained { forward, .. } => Some(forward),
            Self::HistoryErased { .. } | Self::Missing => None,
        }
    }

    /// The reported token.
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Retained { forward: true, .. } => "retained_forward",
            Self::Retained { forward: false, .. } => "retained_reversed",
            Self::HistoryErased { mechanism } => mechanism.tag(),
            Self::Missing => "missing",
        }
    }
}

/// Multiplies signs, where `true` is `+1`.
///
/// Boolean equality implements multiplication under that encoding:
/// `(+1)(-1) = -1` is `true == false`, which is `false`. `None` as soon as any
/// factor carries no value — a partial product is not a sign.
fn compose_signs<const N: usize>(evidence: [OrientationEvidence; N]) -> Option<bool> {
    let mut sign = true;
    for factor in evidence {
        sign = sign == factor.sign()?;
    }
    Some(sign)
}

/// The orientation factors owned by the face.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SourceFaceOrientationEvidence {
    /// The face use's own orientation flag.
    pub face_use_orientation: OrientationEvidence,
    /// `s_f` — `FACE_SURFACE.same_sense`.
    pub face_surface_same_sense: OrientationEvidence,
}

/// The orientation factors owned by one edge use.
///
/// These are per-use facts, not per-face ones: each `EDGE_CURVE` carries its
/// own `same_sense`, and each use selects its own curve representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SourceEdgeOrientationEvidence {
    /// `s_b · s_o`, composed upstream.
    pub bound_times_oriented_edge: OrientationEvidence,
    /// `s_e` — `EDGE_CURVE.same_sense`.
    pub edge_curve_same_sense: OrientationEvidence,
    /// `s_c` — the direction of the selected curve parameterization.
    pub selected_curve_direction: OrientationEvidence,
}

/// One edge use of one bound, with the identity its source gives it.
///
/// # Endpoint invariant
///
/// [`Self::use_vertices`] is in **edge-use traversal order** — it already has
/// `bound_times_oriented_edge` applied, exactly once.
/// [`Self::source_vertices`] is the edge's own `(front, back)`, untouched.
///
/// Both are kept because they answer different questions. `use_vertices` is
/// what `TOP-004` continuity reads — `edge_uses[i].end_vertex()` must equal
/// `edge_uses[i+1].start_vertex()`, cyclically. `source_vertices` answers a
/// different proposition: that the use order was derived from the edge's
/// absolute endpoints by *exactly one* application of the composed sense. A
/// consumer that reads the sign and swaps the use vertices again has applied
/// the same fact twice; see [`Self::endpoints_consistent`].
#[derive(Clone, Debug)]
pub struct SourceEdgeUseInput {
    /// Position within the face.
    pub id: EdgeUseId,
    /// Which edge in the shell's edge table this use refers to.
    pub source_edge_index: usize,
    /// Absolute endpoints in the source edge's own direction.
    pub source_vertices: (SourceVertexKey, SourceVertexKey),
    /// The same endpoints in edge-use traversal order.
    pub use_vertices: (SourceVertexKey, SourceVertexKey),
    /// The orientation factors this use owns.
    pub orientation: SourceEdgeOrientationEvidence,
}

impl SourceEdgeUseInput {
    /// The vertex this use starts at.
    pub const fn start_vertex(&self) -> SourceVertexKey {
        self.use_vertices.0
    }

    /// The vertex this use ends at.
    pub const fn end_vertex(&self) -> SourceVertexKey {
        self.use_vertices.1
    }

    /// Whether the traversal endpoints are the source endpoints reordered by
    /// exactly the composed sense and nothing else.
    pub fn endpoints_consistent(&self) -> bool {
        let Some(forward) = self.orientation.bound_times_oriented_edge.sign() else {
            return false;
        };
        let expected = match forward {
            true => self.source_vertices,
            false => (self.source_vertices.1, self.source_vertices.0),
        };
        self.use_vertices == expected
    }

    /// The normalized traversal sign of `FORMAL_SYSTEM.md` §V:
    ///
    /// ```text
    /// face-use × FACE_SURFACE × (bound × oriented-edge)
    ///          × EDGE_CURVE × selected-curve-direction
    /// ```
    ///
    /// `None` while any factor's value is erased or missing — which is every
    /// use on the compressed path today.
    pub fn normalized_sign(&self, face: &SourceFaceOrientationEvidence) -> Option<bool> {
        compose_signs([
            face.face_use_orientation,
            face.face_surface_same_sense,
            self.orientation.bound_times_oriented_edge,
            self.orientation.edge_curve_same_sense,
            self.orientation.selected_curve_direction,
        ])
    }
}

/// One bound of a face.
///
/// There is deliberately no bound-orientation field. The bound's own sense is
/// not separately recoverable at this seam — it is already inside every edge
/// use's `bound_times_oriented_edge` — and inventing a field to hold a value
/// that does not arrive would be the same error as inferring one.
#[derive(Clone, Debug)]
pub enum SourceBoundInput {
    /// A bound whose edge uses are present.
    EdgeUses {
        /// Position within the face, in source order.
        id: BoundId,
        /// Edge uses in source traversal order.
        edge_uses: Vec<SourceEdgeUseInput>,
    },
    /// The compressed representation contains no edge uses for this bound and
    /// cannot distinguish a legitimate collapsed `VERTEX_LOOP` — which trims
    /// nothing, the apex being closed by the surface's own degeneracy — from
    /// lost data. Never a face-level failure: the face's other bounds are
    /// unaffected, and whether this one is supported belongs to the ambient and
    /// singularity stages.
    DegenerateEvidenceUnavailable {
        /// Position within the face, in source order.
        id: BoundId,
    },
}

impl SourceBoundInput {
    /// Position within the face.
    pub const fn id(&self) -> BoundId {
        match self {
            Self::EdgeUses { id, .. } | Self::DegenerateEvidenceUnavailable { id } => *id,
        }
    }

    /// This bound's edge uses, empty when it has none.
    pub fn edge_uses(&self) -> &[SourceEdgeUseInput] {
        match self {
            Self::EdgeUses { edge_uses, .. } => edge_uses,
            Self::DegenerateEvidenceUnavailable { .. } => &[],
        }
    }

    /// Whether consecutive edge uses meet at a shared source vertex, cyclically.
    ///
    /// `None` means the question does not apply — this is a degenerate evidence
    /// term with no traversal, which is not the same as a walk that fails to
    /// close. `Some(false)` means source endpoint identities *prove* the cycle
    /// is discontinuous.
    ///
    /// This states the `TOP-004` proposition; it does not discharge it, since
    /// nothing consumes the answer authoritatively at step 0.
    pub fn cyclically_continuous(&self) -> Option<bool> {
        let Self::EdgeUses { edge_uses, .. } = self else {
            return None;
        };
        if edge_uses.is_empty() {
            return Some(false);
        }
        let continuous = edge_uses
            .iter()
            .zip(edge_uses.iter().cycle().skip(1))
            .take(edge_uses.len())
            .all(|(current, next)| {
                current.end_vertex().is_identified() && current.end_vertex() == next.start_vertex()
            });
        Some(continuous)
    }
}

/// How many edge uses hold one factor in each state.
///
/// Reported instead of a representative value taken from the first edge use.
/// That shortcut is correct only while every use is constructed identically;
/// the moment one adapter retains a factor that another erases, a
/// representative silently becomes whichever use sorted first.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OrientationFactorCounts {
    /// Uses whose factor carries a sign.
    pub retained: usize,
    /// Uses whose factor was folded into converted geometry.
    pub history_erased: usize,
    /// Uses whose factor is neither retained nor known to have been consumed.
    pub missing: usize,
}

impl OrientationFactorCounts {
    fn tally(evidence: impl Iterator<Item = OrientationEvidence>) -> Self {
        let mut counts = Self::default();
        for factor in evidence {
            match factor {
                OrientationEvidence::Retained { .. } => counts.retained += 1,
                OrientationEvidence::HistoryErased { .. } => counts.history_erased += 1,
                OrientationEvidence::Missing => counts.missing += 1,
            }
        }
        counts
    }

    /// Total uses counted.
    pub const fn total(self) -> usize {
        self.retained + self.history_erased + self.missing
    }
}

/// The source evidence of one face.
#[derive(Clone, Debug)]
pub struct SourceFaceInput {
    /// The document entity this face came from, when the importer retained it.
    pub source_face_id: Option<u64>,
    /// Position within the shell. Always available; never an identity.
    pub declared_face_index: usize,
    /// Bounds in source order.
    pub bounds: Vec<SourceBoundInput>,
    /// The orientation factors the face owns.
    pub orientation: SourceFaceOrientationEvidence,
}

impl SourceFaceInput {
    /// Every edge use of every bound, in source order.
    pub fn edge_uses(&self) -> impl Iterator<Item = &SourceEdgeUseInput> {
        self.bounds.iter().flat_map(SourceBoundInput::edge_uses)
    }

    /// Total edge uses across every bound.
    pub fn edge_use_count(&self) -> usize {
        self.edge_uses().count()
    }

    /// How many bounds carry edge uses.
    pub fn regular_bound_count(&self) -> usize {
        self.bounds
            .iter()
            .filter(|bound| matches!(bound, SourceBoundInput::EdgeUses { .. }))
            .count()
    }

    /// How many bounds carry no edge-use evidence.
    pub fn degenerate_bound_count(&self) -> usize {
        self.bounds.len() - self.regular_bound_count()
    }

    /// Whether every edge use carries both endpoint identities.
    ///
    /// This is the fact the deck solve depends on: a relation between two arcs
    /// is admissible only when their shared endpoint is a shared *source
    /// vertex*, so a face missing any endpoint identity cannot have its
    /// relations built from evidence.
    pub fn endpoint_ids_complete(&self) -> bool {
        self.edge_uses()
            .all(|use_| use_.start_vertex().is_identified() && use_.end_vertex().is_identified())
    }

    /// Whether every edge use satisfies [`SourceEdgeUseInput::endpoints_consistent`].
    pub fn endpoints_consistent(&self) -> bool {
        self.edge_uses()
            .all(SourceEdgeUseInput::endpoints_consistent)
    }

    /// How many edge-use-bearing bounds close on source vertex identity.
    pub fn continuous_regular_bound_count(&self) -> usize {
        self.bounds
            .iter()
            .filter(|bound| bound.cyclically_continuous() == Some(true))
            .count()
    }

    /// The state of `s_b · s_o` across this face's edge uses.
    pub fn bound_times_oriented_edge_counts(&self) -> OrientationFactorCounts {
        OrientationFactorCounts::tally(
            self.edge_uses()
                .map(|use_| use_.orientation.bound_times_oriented_edge),
        )
    }

    /// The state of `s_e` across this face's edge uses.
    pub fn edge_curve_sense_counts(&self) -> OrientationFactorCounts {
        OrientationFactorCounts::tally(
            self.edge_uses()
                .map(|use_| use_.orientation.edge_curve_same_sense),
        )
    }

    /// The state of `s_c` across this face's edge uses.
    pub fn selected_curve_direction_counts(&self) -> OrientationFactorCounts {
        OrientationFactorCounts::tally(
            self.edge_uses()
                .map(|use_| use_.orientation.selected_curve_direction),
        )
    }

    /// How many edge uses have a computable normalized sign.
    ///
    /// Zero for every face on the compressed path, by construction — three
    /// factors are unconditionally erased. Kept as a tripwire: it becoming
    /// nonzero means the adapter's evidence improved.
    pub fn computable_normalized_sign_count(&self) -> usize {
        self.edge_uses()
            .filter(|use_| use_.normalized_sign(&self.orientation).is_some())
            .count()
    }
}

/// Why a face's source evidence could not be assembled.
///
/// One variant, deliberately: a face-level refusal is reserved for what
/// prevents the record from being constructed at all. A bound that merely
/// carries no evidence is [`SourceBoundInput::DegenerateEvidenceUnavailable`],
/// not a lost face.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceEvidenceError {
    /// A bound referenced an edge position the shell's edge table does not
    /// hold. Under `TOP-001` this cannot happen on the converter's own output;
    /// it is checked because the type cannot yet say so.
    EdgeIndexOutOfRange {
        /// The edge use that referenced it.
        edge_use: EdgeUseId,
        /// The out-of-range position.
        index: usize,
    },
}

impl SourceEvidenceError {
    /// The reported token.
    pub const fn tag(self) -> &'static str {
        match self {
            Self::EdgeIndexOutOfRange { .. } => "edge_index_out_of_range",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn retained(forward: bool, origin: OrientationOrigin) -> OrientationEvidence {
        OrientationEvidence::Retained { forward, origin }
    }

    fn erased(mechanism: ErasedOrientationMechanism) -> OrientationEvidence {
        OrientationEvidence::HistoryErased { mechanism }
    }

    /// The compressed path's edge-use evidence: one retained factor, two erased.
    fn compressed_edge_orientation(forward: bool) -> SourceEdgeOrientationEvidence {
        SourceEdgeOrientationEvidence {
            bound_times_oriented_edge: retained(forward, OrientationOrigin::BoundTimesOrientedEdge),
            edge_curve_same_sense: erased(
                ErasedOrientationMechanism::EdgeCurveSenseFoldedIntoConvertedCurve,
            ),
            selected_curve_direction: erased(
                ErasedOrientationMechanism::SelectedCurveDirectionFoldedIntoConvertedCurve,
            ),
        }
    }

    /// The compressed path's face evidence: one retained factor, one erased.
    fn compressed_face_orientation(forward: bool) -> SourceFaceOrientationEvidence {
        SourceFaceOrientationEvidence {
            face_use_orientation: retained(forward, OrientationOrigin::CompressedFaceOrientation),
            face_surface_same_sense: erased(
                ErasedOrientationMechanism::FaceSurfaceSenseFoldedViaSurfaceInvert,
            ),
        }
    }

    fn edge_use(index: usize, front: usize, back: usize, forward: bool) -> SourceEdgeUseInput {
        let source_vertices = (
            SourceVertexKey::ShellVertex(front),
            SourceVertexKey::ShellVertex(back),
        );
        let use_vertices = match forward {
            true => source_vertices,
            false => (source_vertices.1, source_vertices.0),
        };
        SourceEdgeUseInput {
            id: EdgeUseId::new(BoundId(0), index),
            source_edge_index: index,
            source_vertices,
            use_vertices,
            orientation: compressed_edge_orientation(forward),
        }
    }

    fn face(bounds: Vec<SourceBoundInput>) -> SourceFaceInput {
        SourceFaceInput {
            source_face_id: Some(7),
            declared_face_index: 0,
            bounds,
            orientation: compressed_face_orientation(true),
        }
    }

    /// A bare counter restarts at zero in every bound, so a face-wide map keyed
    /// on it silently merges the first use of every bound.
    #[test]
    fn edge_use_ids_are_unique_across_bounds() {
        assert_ne!(EdgeUseId::new(BoundId(0), 0), EdgeUseId::new(BoundId(1), 0));
    }

    /// The edge's own direction is a fact about the edge; a use does not
    /// rewrite it.
    #[test]
    fn reversed_use_preserves_absolute_endpoint_order() {
        let use_ = edge_use(0, 4, 9, false);
        assert_eq!(
            use_.source_vertices,
            (
                SourceVertexKey::ShellVertex(4),
                SourceVertexKey::ShellVertex(9)
            )
        );
    }

    /// And the traversal order is the reversed one.
    #[test]
    fn reversed_use_has_correct_traversal_order() {
        let use_ = edge_use(0, 4, 9, false);
        assert_eq!(use_.start_vertex(), SourceVertexKey::ShellVertex(9));
        assert_eq!(use_.end_vertex(), SourceVertexKey::ShellVertex(4));
        assert!(use_.endpoints_consistent());
    }

    /// The hazard the dual order exists to catch: a consumer reads the sign and
    /// swaps the use vertices *again*.
    #[test]
    fn double_application_of_orientation_is_detected() {
        let mut use_ = edge_use(0, 4, 9, false);
        use_.use_vertices = (use_.use_vertices.1, use_.use_vertices.0);
        assert!(!use_.endpoints_consistent());
    }

    /// `1 -> 2`, `2 -> 3` through a reversed use, `3 -> 1`.
    #[test]
    fn regular_closed_bound_is_cyclically_continuous() {
        let bound = SourceBoundInput::EdgeUses {
            id: BoundId(0),
            edge_uses: vec![
                edge_use(0, 1, 2, true),
                edge_use(1, 3, 2, false),
                edge_use(2, 3, 1, true),
            ],
        };
        assert_eq!(bound.cyclically_continuous(), Some(true));
    }

    /// Proved discontinuous is `Some(false)` — a positive finding, distinct
    /// from the degenerate case's `None`.
    #[test]
    fn regular_discontinuous_bound_is_detected() {
        let bound = SourceBoundInput::EdgeUses {
            id: BoundId(0),
            edge_uses: vec![edge_use(0, 1, 2, true), edge_use(1, 3, 4, true)],
        };
        assert_eq!(bound.cyclically_continuous(), Some(false));
    }

    /// A collapsed `VERTEX_LOOP` must not cost the face the evidence its other
    /// bounds carry.
    #[test]
    fn degenerate_bound_does_not_destroy_face_evidence() {
        let input = face(vec![
            SourceBoundInput::EdgeUses {
                id: BoundId(0),
                edge_uses: vec![edge_use(0, 1, 2, true), edge_use(1, 2, 1, true)],
            },
            SourceBoundInput::DegenerateEvidenceUnavailable { id: BoundId(1) },
        ]);
        assert_eq!(input.edge_use_count(), 2, "the ordinary bound survives");
        assert_eq!(input.regular_bound_count(), 1);
        assert_eq!(input.degenerate_bound_count(), 1);
        assert!(input.endpoint_ids_complete());
        assert!(input.endpoints_consistent());
        assert_eq!(input.continuous_regular_bound_count(), 1);
        assert_eq!(
            input.bounds[1].cyclically_continuous(),
            None,
            "not applicable, which is not the same as discontinuous"
        );
    }

    /// A state asserting a sign is available must contain the sign.
    #[test]
    fn retained_orientation_evidence_contains_its_sign() {
        assert_eq!(
            retained(false, OrientationOrigin::EdgeCurveSameSense).sign(),
            Some(false)
        );
        assert_eq!(
            erased(ErasedOrientationMechanism::EdgeCurveSenseFoldedIntoConvertedCurve).sign(),
            None
        );
        assert_eq!(OrientationEvidence::Missing.sign(), None);
    }

    /// The compressed path's answer, and the reason step 0 exists: no use has a
    /// computable normalized sign.
    #[test]
    fn normalized_sign_requires_every_factor_value() {
        let input = face(vec![SourceBoundInput::EdgeUses {
            id: BoundId(0),
            edge_uses: vec![edge_use(0, 1, 2, true), edge_use(1, 2, 1, true)],
        }]);
        assert_eq!(input.computable_normalized_sign_count(), 0);
        assert_eq!(input.edge_use_count(), 2);
    }

    /// Repairing one erased factor must not make the sign computable while the
    /// others are still erased. This is the compression the module refuses.
    #[test]
    fn repairing_only_face_surface_factor_does_not_make_sign_computable() {
        let mut input = face(vec![SourceBoundInput::EdgeUses {
            id: BoundId(0),
            edge_uses: vec![edge_use(0, 1, 2, true)],
        }]);
        input.orientation.face_surface_same_sense =
            retained(true, OrientationOrigin::FaceSurfaceSameSense);
        assert_eq!(
            input.computable_normalized_sign_count(),
            0,
            "s_e and s_c are still erased"
        );
    }

    /// All five terms, with the `true = +1` encoding.
    #[test]
    fn normalized_sign_multiplies_all_five_terms_correctly() {
        let face_orientation = SourceFaceOrientationEvidence {
            face_use_orientation: retained(true, OrientationOrigin::CompressedFaceOrientation),
            face_surface_same_sense: retained(true, OrientationOrigin::FaceSurfaceSameSense),
        };
        let mut use_ = edge_use(0, 1, 2, true);
        use_.orientation = SourceEdgeOrientationEvidence {
            bound_times_oriented_edge: retained(true, OrientationOrigin::BoundTimesOrientedEdge),
            edge_curve_same_sense: retained(true, OrientationOrigin::EdgeCurveSameSense),
            selected_curve_direction: retained(
                true,
                OrientationOrigin::SelectedCurveParameterDirection,
            ),
        };
        assert_eq!(use_.normalized_sign(&face_orientation), Some(true), "+1^5");

        // One negative flips the product.
        use_.orientation.edge_curve_same_sense =
            retained(false, OrientationOrigin::EdgeCurveSameSense);
        assert_eq!(use_.normalized_sign(&face_orientation), Some(false));

        // A second negative flips it back.
        use_.orientation.selected_curve_direction =
            retained(false, OrientationOrigin::SelectedCurveParameterDirection);
        assert_eq!(use_.normalized_sign(&face_orientation), Some(true));

        // A third leaves it negative.
        use_.orientation.bound_times_oriented_edge =
            retained(false, OrientationOrigin::BoundTimesOrientedEdge);
        assert_eq!(use_.normalized_sign(&face_orientation), Some(false));
    }

    /// A face whose uses disagree must not be summarized by whichever one comes
    /// first. This is the state the compressed adapter cannot produce today and
    /// a better adapter will.
    #[test]
    fn mixed_edge_use_evidence_is_counted_not_represented() {
        let mut partly_repaired = edge_use(1, 2, 3, true);
        partly_repaired.orientation.edge_curve_same_sense =
            retained(false, OrientationOrigin::EdgeCurveSameSense);

        let input = face(vec![SourceBoundInput::EdgeUses {
            id: BoundId(0),
            edge_uses: vec![edge_use(0, 1, 2, true), partly_repaired],
        }]);

        let counts = input.edge_curve_sense_counts();
        assert_eq!(counts.retained, 1);
        assert_eq!(counts.history_erased, 1);
        assert_eq!(counts.missing, 0);
        assert_eq!(counts.total(), input.edge_use_count());

        // The retained one still cannot produce a sign, because `s_c` and `s_f`
        // remain erased. Improving one factor is visible without pretending the
        // product became computable.
        assert_eq!(input.computable_normalized_sign_count(), 0);

        // And the factor that is uniformly retained reads as uniformly retained.
        let composed = input.bound_times_oriented_edge_counts();
        assert_eq!(composed.retained, 2);
        assert_eq!(composed.history_erased, 0);
    }

    /// Lattice rank and orientation evidence are independent axes. A face with
    /// no periodic direction says nothing about whether its traversal sign is
    /// computable, and the probe must not let one stand in for the other.
    #[test]
    fn rank_zero_is_reported_independently_of_orientation_evidence() {
        use crate::tessellation::domain::lattice::CertifiedLattice;

        let input = face(vec![SourceBoundInput::EdgeUses {
            id: BoundId(0),
            edge_uses: vec![edge_use(0, 1, 2, true)],
        }]);
        let plane = CertifiedLattice::NON_PERIODIC;
        assert_eq!(plane.certified_rank(), 0);
        assert_eq!(plane.declared_u_period(), None);
        assert!(
            input.endpoint_ids_complete(),
            "endpoint evidence is unaffected by the lattice"
        );
        assert_eq!(input.computable_normalized_sign_count(), 0);

        // And an uncertified periodic axis is rank 0 while still declaring a
        // period — also independent of everything above.
        let sphere = CertifiedLattice::from_unevidenced_accessors(Some(6.28), None);
        assert_eq!(sphere.certified_rank(), 0);
        assert_eq!(sphere.declared_u_period(), Some(6.28));
    }
}
