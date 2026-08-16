//! Serialized topological data exchange format
//!
//! Topological data structures in truck is subject to editing and has complex reference relationships.
//! They are not suitable for direct serialization and must be converted to lighter and simpler data structures.
//! These structures, prefixed with `Compressed`, are a group of structures that are easy to serialize,
//! but not suitable for real-time shape editing.
//!
//! They directly reflect the results of parsing data from json or STEP, and all member variables are public.
//! Boundary connectivity and closure are checked when converting to proprietary data structures, `Vertex`, `Edge`, and so on.

use crate::*;
use rustc_hash::FxHashMap as HashMap;
use serde::{Deserialize, Serialize};

/// Serialized compressed edge
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompressedEdge<C> {
    /// vertices of the edge
    pub vertices: (usize, usize),
    /// curve geometry of the edge
    pub curve: C,
}

impl<C> CompressedEdge<C> {
    #[inline(always)]
    fn create_edge<P>(self, v: &[Vertex<P>]) -> Result<Edge<P, C>> {
        let front = &v[self.vertices.0];
        let back = &v[self.vertices.1];
        Edge::try_new(front, back, self.curve)
    }
}

/// The index of an edge in `CompressedShell`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CompressedEdgeIndex {
    /// the index of the edge
    pub index: usize,
    /// the orientation of the edge
    pub orientation: bool,
}

impl From<(usize, bool)> for CompressedEdgeIndex {
    fn from((index, orientation): (usize, bool)) -> Self {
        Self { index, orientation }
    }
}

/// The identity of an entity in the document an object was imported from.
///
/// An importer that knows which entity something came from — a STEP file writes
/// them as `#1234` — records it so that later stages can name what they are
/// complaining about. Without it a failure can only be counted, and "604 of
/// 24202 faces produced no geometry" localises nothing.
///
/// Opaque and importer-agnostic: `truck-topology` carries the number and does
/// not interpret it.
///
/// **Document-local.** `#1234` in one file has nothing to do with `#1234` in
/// another. Comparing ids across documents, or using one as a key in a
/// structure spanning several, is meaningless. Nothing here associates an id
/// with the document it came from, so that association is the caller's
/// obligation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceEntityId(u64);

impl SourceEntityId {
    /// Name an entity of the document being imported.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    /// The underlying number, for printing and for importer-side lookups.
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for SourceEntityId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// Where an imported face came from, as a chain rather than a single name.
///
/// A B-rep document distinguishes a **use** of a face from the **definition**
/// of that face, and both from the surface the definition names. STEP spells
/// the chain
///
/// ```text
/// shell → ORIENTED_FACE (use) → FACE_SURFACE (definition) → surface geometry
/// ```
///
/// and other formats make the same distinction under other names, which is why
/// the field names here are generic. Several uses may resolve to one
/// definition, orientation is composed at the use layer, and geometry is shared
/// at the surface layer — so collapsing the chain to one id makes exactly the
/// failures one wants to tell apart indistinguishable: a wrong shell-use
/// orientation, a wrong underlying face, a wrong face-to-surface association,
/// and a duplicated use all reduce to the same ambiguous number.
///
/// Every field is optional because a document may inline a definition instead
/// of referencing it — STEP's `PlaceHolder::Owned` — and because a face built
/// by modelling or by a healing pass was imported from nothing at all.
/// Fabricating an id in either case would make provenance unfalsifiable.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FaceProvenance {
    /// The entity the containing shell referenced to reach this face.
    ///
    /// A *use*: it carries orientation, and two of them may name one
    /// definition.
    pub use_id: Option<SourceEntityId>,
    /// The entity that defines the face — its bounds and its surface reference.
    pub definition_id: Option<SourceEntityId>,
    /// The entity defining the face's supporting surface.
    ///
    /// Separate from the definition because one surface is commonly shared by
    /// many faces, so "which surface did this face name" is a different
    /// question from "which face is this".
    pub surface_id: Option<SourceEntityId>,
    /// Which of this face's bounds the source declared as its *outer* bound.
    ///
    /// `boundaries` is a bare `Vec` and gives no bound any standing, so
    /// "the outer one" is not recoverable from it. STEP does distinguish them —
    /// `FACE_OUTER_BOUND` is a subtype of `FACE_BOUND` — and material selection
    /// needs the distinction: with one outer loop the material region is the
    /// bounded complementary component, and with an inner loop it is not. See
    /// [`OuterBoundStanding`].
    #[serde(default)]
    pub outer_bound: OuterBoundStanding,
}

/// Which bound of a face its source declared to be the outer one.
///
/// The distinction between "nothing recorded this" and "the source declared no
/// outer bound" is the whole point of the type: a face reaching a consumer with
/// [`Self::NotRetained`] is one whose outer-bound authority was never carried,
/// and inferring outer standing from a face happening to have one bound is
/// exactly the guess this prevents. A `FACE_BOUND`-only face is legal STEP and
/// its single bound is *not* thereby an outer bound.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OuterBoundStanding {
    /// No stage carried the standing. The default, so a face built by
    /// modelling, healing or deserialization of an older document claims
    /// nothing.
    #[default]
    NotRetained,
    /// The standing was read and the source declared no `FACE_OUTER_BOUND`.
    NoneDeclared,
    /// The source declared an outer bound, at this index into `boundaries`.
    Declared {
        /// Index into the face's `boundaries`, after any bound that
        /// contributed no wire was dropped.
        bound_index: u32,
        /// How many `FACE_OUTER_BOUND` entities the face declared. STEP
        /// permits at most one; more than one is a source contradiction, and
        /// recording the count is what lets a consumer say so rather than
        /// silently take the first.
        declared_count: u32,
    },
}

impl OuterBoundStanding {
    /// The outer bound's index, when exactly one was declared.
    ///
    /// `None` for every other state, including a face declaring two outer
    /// bounds — that is a contradiction, not a choice.
    pub fn unique_outer_bound_index(self) -> Option<u32> {
        match self {
            Self::Declared {
                bound_index,
                declared_count: 1,
            } => Some(bound_index),
            _ => None,
        }
    }

    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::NotRetained => "not_retained",
            Self::NoneDeclared => "none_declared",
            Self::Declared {
                declared_count: 1, ..
            } => "declared",
            Self::Declared { .. } => "multiply_declared",
        }
    }
}

impl FaceProvenance {
    /// True when nothing at all is known — an unimported face.
    pub fn is_empty(self) -> bool {
        self.use_id.is_none() && self.definition_id.is_none() && self.surface_id.is_none()
    }

    /// The most specific identity available for naming this face in one word.
    ///
    /// Prefers the definition: it is the thing a reader is usually looking for,
    /// and it is stable across the several uses that may reach it.
    pub fn best_id(self) -> Option<SourceEntityId> {
        self.definition_id.or(self.use_id).or(self.surface_id)
    }
}

impl std::fmt::Display for FaceProvenance {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match (self.use_id, self.definition_id) {
            (Some(u), Some(d)) => write!(f, "face use {u} of face {d}")?,
            (Some(u), None) => write!(f, "face use {u}")?,
            (None, Some(d)) => write!(f, "face {d}")?,
            (None, None) => write!(f, "unimported face")?,
        }
        if let Some(s) = self.surface_id {
            write!(f, ", surface {s}")?;
        }
        Ok(())
    }
}

/// Serialized compressed face
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompressedFace<S> {
    /// Boundaries of the face
    pub boundaries: Vec<Vec<CompressedEdgeIndex>>,
    /// orientation of the face
    pub orientation: bool,
    /// surface geometry of the face
    pub surface: S,
    /// where this face came from, when it was imported from a document
    ///
    /// Every stage that rebuilds a face must carry this through, or the
    /// identity is lost exactly where a failure is about to need it.
    ///
    /// Note that this records what the importer *asserted*, and conserving it
    /// downstream is a weaker claim than it being right. That the stored chain
    /// matches the document is a separate obligation, discharged where the
    /// references are read, not here.
    #[serde(default)]
    pub provenance: FaceProvenance,
}

impl<S: PartialEq> CompressedFace<S> {
    /// Compare the face as geometry and topology, ignoring where it came from.
    ///
    /// `PartialEq` on this type is **whole-record** equality and includes
    /// provenance, so two faces of identical shape imported from different
    /// entities are not equal. That is the right default for a serialized
    /// record — a round trip must preserve everything — but it is the wrong
    /// question for healing, deduplication, or a test comparing shapes. Ask
    /// that question with this.
    pub fn geometrically_eq(&self, other: &Self) -> bool {
        self.boundaries == other.boundaries
            && self.orientation == other.orientation
            && self.surface == other.surface
    }
}

impl<S> CompressedFace<S> {
    fn create_face<P, C>(self, edges: &[Edge<P, C>]) -> Result<Face<P, C, S>> {
        let wires: Vec<Wire<P, C>> = self
            .boundaries
            .into_iter()
            .map(|wire| {
                wire.into_iter()
                    .map(
                        |CompressedEdgeIndex { index, orientation }| match orientation {
                            true => edges[index].clone(),
                            false => edges[index].inverse(),
                        },
                    )
                    .collect()
            })
            .collect();
        let mut face = Face::try_new(wires, self.surface)?;
        if !self.orientation {
            face.invert();
        }
        Ok(face)
    }
}

/// Serialized compressed shell
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompressedShell<P, C, S> {
    /// all geometries of vertices
    pub vertices: Vec<P>,
    /// all geometries and end vertices of edges
    pub edges: Vec<CompressedEdge<C>>,
    /// all geometries and boundaries of faces
    pub faces: Vec<CompressedFace<S>>,
    /// The geometric-uncertainty declared by the source representation that
    /// owns this shell, in the source's native coordinate units.
    ///
    /// STEP declares it in the shape's geometric representation context
    /// (`GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT` / `UNCERTAINTY_MEASURE_WITH_UNIT`),
    /// and it is the tolerance under which the source asserts its connectivity
    /// (vertex↔carrier incidence). It is semantic provenance, not a numerical
    /// floor: a downstream consumer may use it to judge whether a source vertex
    /// realizes a point on an edge-curve carrier, and must never replace it with
    /// a generic epsilon. `None` when the source declares none, in which case a
    /// consumer falls back to its own numerical tolerance.
    #[serde(default)]
    pub source_geometric_uncertainty: Option<f64>,
}

/// Serialized compressed solid
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompressedSolid<P, C, S> {
    /// all boundaries of solid
    pub boundaries: Vec<CompressedShell<P, C, S>>,
}

struct CompressDirector<P, C> {
    vmap: HashMap<VertexID<P>, (usize, P)>,
    emap: HashMap<EdgeID<C>, (usize, CompressedEdge<C>)>,
}

impl<P: Clone, C: Clone> CompressDirector<P, C> {
    #[inline(always)]
    fn new() -> Self {
        Self {
            vmap: HashMap::default(),
            emap: HashMap::default(),
        }
    }
    #[inline(always)]
    fn get_vid(&mut self, vertex: &Vertex<P>) -> usize {
        let id = self.vmap.len();
        self.vmap
            .entry(vertex.id())
            .or_insert_with(|| (id, vertex.point()))
            .0
    }

    #[inline(always)]
    fn get_eid(&mut self, edge: &Edge<P, C>) -> CompressedEdgeIndex {
        match self.emap.get(&edge.id()) {
            Some(got) => (got.0, edge.orientation()).into(),
            None => {
                let id = self.emap.len();
                let front_id = self.get_vid(edge.absolute_front());
                let back_id = self.get_vid(edge.absolute_back());
                let curve = edge.curve();
                let cedge = CompressedEdge {
                    vertices: (front_id, back_id),
                    curve,
                };
                self.emap.insert(edge.id(), (id, cedge));
                (id, edge.orientation()).into()
            }
        }
    }

    #[inline(always)]
    fn create_boundary(&mut self, boundary: &Wire<P, C>) -> Vec<CompressedEdgeIndex> {
        boundary.iter().map(|edge| self.get_eid(edge)).collect()
    }

    #[inline(always)]
    fn create_cface<S: Clone>(&mut self, face: &Face<P, C, S>) -> CompressedFace<S> {
        CompressedFace {
            boundaries: face
                .boundaries
                .iter()
                .map(|wire| self.create_boundary(wire))
                .collect(),
            orientation: face.orientation(),
            surface: face.surface(),
            // Compressing an in-memory `Face` recovers no importer identity:
            // a `Face` is the editable form and carries none. Anything that
            // round-trips through it loses provenance, which is a real
            // limitation and is better stated as `None` than papered over with
            // the face's position in some vector.
            provenance: FaceProvenance::default(),
        }
    }

    #[inline(always)]
    fn map2vec<K, T>(map: HashMap<K, (usize, T)>) -> Vec<T> {
        let mut vec: Vec<_> = map.into_iter().map(|entry| entry.1).collect();
        vec.sort_by_key(|x| x.0);
        vec.into_iter().map(|x| x.1).collect()
    }

    #[inline(always)]
    fn vertices_edges(self) -> (Vec<P>, Vec<CompressedEdge<C>>) {
        (Self::map2vec(self.vmap), Self::map2vec(self.emap))
    }
}

impl<P: Clone, C: Clone, S: Clone> Shell<P, C, S> {
    /// Compresses the shell into the serialized compressed shell.
    pub fn compress(&self) -> CompressedShell<P, C, S> {
        let mut director = CompressDirector::new();
        let mut face_closure = |face: &Face<P, C, S>| director.create_cface(face);
        let faces = self.iter().map(&mut face_closure).collect();
        let (vertices, edges) = director.vertices_edges();
        CompressedShell {
            vertices,
            edges,
            faces,
            // Compressing an in-memory `Shell` recovers no source
            // representation, so no geometric uncertainty is claimed. Anything
            // that round-trips through the editable `Shell` form loses the
            // importer's provenance, exactly as `CompressedFace`'s provenance
            // is lost; `None` states that rather than fabricating a number.
            source_geometric_uncertainty: None,
        }
    }

    /// Extracts the serialized compressed shell into the shell.
    pub fn extract(cshell: CompressedShell<P, C, S>) -> Result<Self> {
        let CompressedShell {
            vertices,
            edges,
            faces,
            source_geometric_uncertainty: _,
        } = cshell;
        let vertices: Vec<_> = vertices.into_iter().map(Vertex::new).collect();
        let edges = edges
            .into_iter()
            .map(move |edge| edge.create_edge(&vertices))
            .collect::<Result<Vec<_>>>()?;
        faces
            .into_iter()
            .map(move |face| face.create_face(&edges))
            .collect()
    }
}

impl<P: Clone, C: Clone, S: Clone> Solid<P, C, S> {
    /// Compresses the solid into the serialized compressed solid.
    pub fn compress(&self) -> CompressedSolid<P, C, S> {
        CompressedSolid {
            boundaries: self
                .boundaries()
                .iter()
                .map(|shell| shell.compress())
                .collect(),
        }
    }

    /// Extracts the serialized compressed shell into the shell.
    pub fn extract(csolid: CompressedSolid<P, C, S>) -> Result<Self> {
        let shells: Result<Vec<Shell<P, C, S>>> =
            csolid.boundaries.into_iter().map(Shell::extract).collect();
        Solid::try_new(shells?)
    }
}

// -------------------------- test -------------------------- //

#[test]
fn compress_extract() {
    let cube = solid::cube();
    let shell0 = &cube.boundaries()[0];
    let shell1 = Shell::extract(shell0.compress()).unwrap();
    assert!(same_topology(shell0, &shell1));
}

#[allow(dead_code)]
fn vmap_subroutin<P, Q>(
    v0: &Vertex<P>,
    v1: &Vertex<Q>,
    vmap: &mut HashMap<VertexID<P>, VertexID<Q>>,
) -> bool {
    match vmap.get(&v0.id()) {
        Some(got) => *got == v1.id(),
        None => {
            vmap.insert(v0.id(), v1.id());
            true
        }
    }
}

#[allow(dead_code)]
fn emap_subroutin<P, Q, C, D>(
    edge0: &Edge<P, C>,
    edge1: &Edge<Q, D>,
    vmap: &mut HashMap<VertexID<P>, VertexID<Q>>,
    emap: &mut HashMap<EdgeID<C>, EdgeID<D>>,
) -> bool {
    match emap.get(&edge0.id()) {
        Some(got) => *got == edge1.id(),
        None => {
            emap.insert(edge0.id(), edge1.id());
            vmap_subroutin(edge0.front(), edge1.front(), vmap)
                && vmap_subroutin(edge0.back(), edge1.back(), vmap)
        }
    }
}

#[allow(dead_code)]
fn same_topology<P, C, S, Q, D, T>(one: &Shell<P, C, S>, other: &Shell<Q, D, T>) -> bool {
    let mut vmap = HashMap::<VertexID<P>, VertexID<Q>>::default();
    let mut emap = HashMap::<EdgeID<C>, EdgeID<D>>::default();
    if one.len() != other.len() {
        return false;
    }
    for (face0, face1) in one.iter().zip(other.iter()) {
        let biters0 = face0.boundary_iters();
        let biters1 = face1.boundary_iters();
        if biters0.len() != biters1.len() {
            return false;
        }
        for (biter0, biter1) in biters0.into_iter().zip(biters1) {
            if biter0.len() != biter1.len() {
                return false;
            }
            for (edge0, edge1) in biter0.zip(biter1) {
                if !emap_subroutin(&edge0, &edge1, &mut vmap, &mut emap) {
                    return false;
                }
            }
        }
    }
    true
}

impl<P, C, S> Serialize for Shell<P, C, S>
where
    P: Clone + Serialize,
    C: Clone + Serialize,
    S: Clone + Serialize,
{
    fn serialize<Serializer>(
        &self,
        serializer: Serializer,
    ) -> std::result::Result<Serializer::Ok, Serializer::Error>
    where
        Serializer: serde::Serializer,
    {
        self.compress().serialize(serializer)
    }
}

impl<'de, P, C, S> Deserialize<'de> for Shell<P, C, S>
where
    P: Clone + Deserialize<'de>,
    C: Clone + Deserialize<'de>,
    S: Clone + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        let compressed = CompressedShell::<P, C, S>::deserialize(deserializer)?;
        Shell::extract(compressed).map_err(D::Error::custom)
    }
}

impl<P, C, S> Serialize for Solid<P, C, S>
where
    P: Clone + Serialize,
    C: Clone + Serialize,
    S: Clone + Serialize,
{
    fn serialize<Serializer>(
        &self,
        serializer: Serializer,
    ) -> std::result::Result<Serializer::Ok, Serializer::Error>
    where
        Serializer: serde::Serializer,
    {
        self.compress().serialize(serializer)
    }
}

impl<'de, P, C, S> Deserialize<'de> for Solid<P, C, S>
where
    P: Clone + Deserialize<'de>,
    C: Clone + Deserialize<'de>,
    S: Clone + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        let compressed = CompressedSolid::<P, C, S>::deserialize(deserializer)?;
        Solid::extract(compressed).map_err(D::Error::custom)
    }
}

impl<P, C, S> Serialize for Face<P, C, S>
where
    P: Clone + Serialize,
    C: Clone + Serialize,
    S: Clone + Serialize,
{
    fn serialize<Serializer>(
        &self,
        serializer: Serializer,
    ) -> std::result::Result<Serializer::Ok, Serializer::Error>
    where
        Serializer: serde::Serializer,
    {
        Shell::from(vec![self.clone()]).serialize(serializer)
    }
}

impl<'de, P, C, S> Deserialize<'de> for Face<P, C, S>
where
    P: Clone + Deserialize<'de>,
    C: Clone + Deserialize<'de>,
    S: Clone + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Shell::deserialize(deserializer).map(|mut shell| shell.pop().unwrap())
    }
}
