//! Typed identities and transactional arenas.
//!
//! Conversion from STEP has two kinds of number in it and they were both
//! `usize`: the entity id a file writes as `#1234`, and the position a
//! converted value ends up at in a vector. Nothing in the type system kept them
//! apart, and nothing tied a position to the value that was actually stored
//! there.
//!
//! That gap is where the worst defect of this codebase lived. An index was
//! claimed from a map's length *before* the conversion that decides whether the
//! value exists; when the conversion failed the value was never pushed but the
//! map kept its entry, so map and vector desynchronised and every later lookup
//! addressed its neighbour. Faces then received a curve belonging to a
//! different surface — valid geometry in the wrong place, which meshes into a
//! large smooth wrong region rather than failing.
//!
//! The fix is not to be careful. It is to make the careless version
//! inexpressible:
//!
//! - [`SourceId`] and [`Index`] are distinct types, and each is tagged with the
//!   kind of thing it refers to, so an edge position cannot be passed where a
//!   vertex position belongs and neither can be confused with an entity id.
//! - An [`Index`] is obtainable only from [`Arena::get_or_try_insert`] or a
//!   lookup that found one, and `get_or_try_insert` runs the conversion
//!   *first*, so a position exists only once the value occupying it does.
//!
//! The invariant `items.len() == positions.len()`, with every position
//! addressing the value converted from the id that maps to it, is therefore
//! maintained by construction rather than asserted after the fact.
//!
//! Contracts: `TOP-001` (source reference integrity), `TOP-002` (transactional
//! conversion insertion), `TOP-007` (canonical entity identity). See
//! `MATHEMATICAL_FOUNDATION.md` §22.
//!
//! Structural correctness and retained evidence are separate requirements, and
//! only the first is free. Storing the identity is not needed to *maintain*
//! TOP-001 in a correct arena; it is needed to check it rather than trust it
//! ([`Arena::get_checked`]), to name the entity in a failure report, and to keep
//! provenance once the source table is out of scope. So every item carries its
//! own [`SourceId`] — one `u64` per entity, one integer compare per checked
//! lookup.

use core::fmt;
use core::marker::PhantomData;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// Marks identities and positions belonging to `EDGE_CURVE`s.
#[derive(Debug)]
pub enum EdgeKind {}

/// Marks identities and positions belonging to `VERTEX_POINT`s.
#[derive(Debug)]
pub enum VertexKind {}

/// Marks identities and positions belonging to surfaces — any entity a
/// `FACE_SURFACE.face_geometry` can name.
#[derive(Debug)]
pub enum SurfaceKind {}

/// The entity id a STEP file writes as `#1234`, tagged with what it names.
///
/// The tag is what stops a vertex id being looked up in the edge arena. It is
/// erased at runtime — `PhantomData<fn() -> K>` so the tag imposes no variance
/// or auto-trait constraints of its own.
pub struct SourceId<K> {
    raw: u64,
    kind: PhantomData<fn() -> K>,
}

impl<K> SourceId<K> {
    /// Name an entity. This asserts nothing about whether it resolves; only
    /// [`Arena::try_insert`] can answer that.
    pub fn new(raw: u64) -> Self {
        Self {
            raw,
            kind: PhantomData,
        }
    }
}

// Derived impls would demand `K: Clone` and so on, which the marker types
// deliberately cannot satisfy — they are uninhabited. Written out instead.
impl<K> Clone for SourceId<K> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<K> Copy for SourceId<K> {}
impl<K> PartialEq for SourceId<K> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}
impl<K> Eq for SourceId<K> {}
impl<K> Hash for SourceId<K> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}
impl<K> fmt::Debug for SourceId<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.raw)
    }
}

/// A position in the arena of `K`, and evidence that something is stored there.
///
/// There is no public constructor. An `Index` can only come from an insert that
/// succeeded or a lookup that found one, which is what makes indexing total.
pub struct Index<K> {
    position: usize,
    kind: PhantomData<fn() -> K>,
}

impl<K> Index<K> {
    fn new(position: usize) -> Self {
        Self {
            position,
            kind: PhantomData,
        }
    }

    /// The raw position, for the one boundary where a foreign type demands a
    /// bare `usize`. Every use of this is a place the guarantee stops.
    pub fn position(self) -> usize {
        self.position
    }
}

impl<K> Clone for Index<K> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<K> Copy for Index<K> {}
impl<K> PartialEq for Index<K> {
    fn eq(&self, other: &Self) -> bool {
        self.position == other.position
    }
}
impl<K> Eq for Index<K> {}
impl<K> fmt::Debug for Index<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}]", self.position)
    }
}

/// A converted value together with the identity it was converted from.
///
/// The pairing is the point: a value and its provenance are one object, so they
/// cannot drift apart the way a value vector and an identity vector can.
#[derive(Debug)]
pub struct Stored<K, T> {
    source_id: SourceId<K>,
    value: T,
}

impl<K, T> Stored<K, T> {
    /// The entity this value was converted from.
    pub fn source_id(&self) -> SourceId<K> {
        self.source_id
    }

    /// The converted value.
    pub fn value(&self) -> &T {
        &self.value
    }
}

/// `TOP-001` failed: an index resolved to a value converted from some other
/// entity than the one the caller named.
///
/// This is the failure the whole arena exists to make impossible, so in a
/// correct build it is unreachable. It is a type rather than an assertion
/// because an unreachable state that can still be *printed* is what turns a
/// smooth unexplained blob into `MATHEMATICAL_FOUNDATION.md` §61's one-line
/// localisation.
pub struct IdentityMismatch<K> {
    /// The entity the caller asked for.
    pub requested: SourceId<K>,
    /// The entity whose conversion is actually stored there.
    pub stored: SourceId<K>,
    /// Where it was stored.
    pub index: Index<K>,
}

impl<K> fmt::Display for IdentityMismatch<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TOP-001 failed: requested {:?}, but arena index {:?} stores {:?}",
            self.requested, self.index, self.stored
        )
    }
}

impl<K> fmt::Debug for IdentityMismatch<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

/// Converted values of one kind, addressable by the entity that produced them.
pub struct Arena<K, T> {
    items: Vec<Stored<K, T>>,
    positions: HashMap<SourceId<K>, Index<K>>,
}

impl<K, T> Arena<K, T> {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            positions: HashMap::new(),
        }
    }

    /// Resolve an identity to its one converted value, converting on first use.
    ///
    /// `convert` runs before anything is claimed, so a failed conversion leaves
    /// the arena exactly as it was — no position reserved, no entry left
    /// pointing at a slot some later value will occupy (`TOP-002`). This is the
    /// whole reason the type exists.
    ///
    /// It resolves rather than inserts (`TOP-007`). An id already present
    /// returns its existing position without converting again: a STEP shell
    /// names the same edge from both faces that share it, and that is one edge,
    /// not a duplicate to reject. A repeated *reference* is ordinary; only a
    /// second canonical *object* for one identity would be a defect, and this
    /// signature cannot express one.
    pub fn get_or_try_insert(
        &mut self,
        id: SourceId<K>,
        convert: impl FnOnce() -> Option<T>,
    ) -> Option<Index<K>> {
        if let Some(existing) = self.positions.get(&id) {
            return Some(*existing);
        }
        let value = convert()?;
        let index = Index::new(self.items.len());
        self.items.push(Stored {
            source_id: id,
            value,
        });
        self.positions.insert(id, index);
        debug_assert_eq!(
            self.items.len(),
            self.positions.len(),
            "an arena holds exactly one value per mapped identity"
        );
        Some(index)
    }

    /// Where the value converted from `id` is, if it converted at all.
    pub fn index_of(&self, id: SourceId<K>) -> Option<Index<K>> {
        self.positions.get(&id).copied()
    }

    /// The value at an index. Total: an [`Index`] is evidence that one is
    /// there.
    pub fn get(&self, index: Index<K>) -> &T {
        &self.items[index.position].value
    }

    /// The value at an index, having checked it was converted from the entity
    /// the caller named (`TOP-001`, §22.2).
    ///
    /// One integer comparison. Use it wherever an index is followed back to a
    /// value on behalf of a source reference — that is the exact step the
    /// desynchronisation defect corrupted, and the only step at which the
    /// corruption is still cheap to name.
    pub fn get_checked(
        &self,
        index: Index<K>,
        requested: SourceId<K>,
    ) -> Result<&T, IdentityMismatch<K>> {
        let stored = &self.items[index.position];
        if stored.source_id != requested {
            return Err(IdentityMismatch {
                requested,
                stored: stored.source_id,
                index,
            });
        }
        Ok(&stored.value)
    }

    /// The value at a bare position, for the boundary where a foreign type
    /// carries positions rather than [`Index`]es. The mirror of
    /// [`Index::position`], and like it, a place the guarantee stops.
    pub fn value_at(&self, position: usize) -> Option<&T> {
        self.items.get(position).map(Stored::value)
    }

    /// The identity converted into a bare position, for failure reports built
    /// from data that has already left the typed world.
    pub fn source_id_at(&self, position: usize) -> Option<SourceId<K>> {
        self.items.get(position).map(Stored::source_id)
    }

    /// How many values are stored.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The stored values, in position order, dropping their identities.
    ///
    /// Every call is a loss of provenance, and is only correct where the
    /// consumer addresses values by position and can no longer ask which entity
    /// it is holding — which is what `CompressedShell` does today. It stops
    /// being needed when those types carry identity themselves (§33a item 11).
    pub fn into_items(self) -> Vec<T> {
        self.items.into_iter().map(|stored| stored.value).collect()
    }
}

// Written out rather than derived so the arena stays printable for values that
// are not, which most converted geometry is not.
impl<K, T> fmt::Debug for Arena<K, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Arena")
            .field("len", &self.items.len())
            .finish_non_exhaustive()
    }
}

impl<K, T> Default for Arena<K, T> {
    fn default() -> Self {
        Self::new()
    }
}

/// The entity id of an `EDGE_CURVE`.
pub type EdgeCurveId = SourceId<EdgeKind>;
/// A position in the edge arena.
pub type EdgeIndex = Index<EdgeKind>;
/// The entity id of a `VERTEX_POINT`.
pub type VertexPointId = SourceId<VertexKind>;
/// A position in the vertex arena.
pub type VertexIndex = Index<VertexKind>;
/// The entity id of a surface.
pub type SurfaceId = SourceId<SurfaceKind>;
/// A position in the surface arena.
pub type SurfaceIndex = Index<SurfaceKind>;

#[cfg(test)]
mod tests {
    use super::*;

    /// The regression for the defect this type exists to prevent.
    ///
    /// A valid entity, then one whose conversion fails, then another valid one.
    /// The third must land at position 1 and be addressable as itself. Under
    /// the old reserve-before-convert scheme the failure consumed position 1,
    /// so C was mapped to 2 while sitting at 1, and every lookup past the
    /// failure returned its neighbour.
    #[test]
    fn a_failed_conversion_consumes_no_position() {
        let mut arena = Arena::<EdgeKind, &str>::new();
        let a = arena.get_or_try_insert(EdgeCurveId::new(10), || Some("A"));
        let b = arena.get_or_try_insert(EdgeCurveId::new(20), || None);
        let c = arena.get_or_try_insert(EdgeCurveId::new(30), || Some("C"));

        assert_eq!(a.map(Index::position), Some(0));
        assert_eq!(b, None, "a failed conversion yields no position");
        assert_eq!(c.map(Index::position), Some(1), "C must take the next slot");

        assert_eq!(arena.index_of(EdgeCurveId::new(20)), None);
        assert_eq!(
            arena.index_of(EdgeCurveId::new(30)).map(Index::position),
            Some(1)
        );
        let items = arena.into_items();
        assert_eq!(items, vec!["A", "C"]);
    }

    #[test]
    fn a_repeated_identity_stores_one_value() {
        let mut arena = Arena::<EdgeKind, u32>::new();
        let first = arena.get_or_try_insert(EdgeCurveId::new(7), || Some(1));
        // Two faces share an edge, so the same id arrives twice. The second
        // must not convert again, and must not occupy a second slot. TOP-007:
        // a repeated reference resolves, it does not error.
        let second = arena.get_or_try_insert(EdgeCurveId::new(7), || {
            panic!("a present identity must not be converted twice")
        });
        assert_eq!(first, second);
        assert_eq!(arena.into_items(), vec![1]);
    }

    #[test]
    fn every_mapped_identity_addresses_its_own_value() {
        let mut arena = Arena::<VertexKind, u64>::new();
        for raw in [3u64, 9, 4, 1] {
            // Every other conversion fails, so positions and ids diverge.
            arena.get_or_try_insert(VertexPointId::new(raw), || (raw % 2 == 1).then_some(raw));
        }
        let mapped: Vec<_> = [3u64, 9, 4, 1]
            .into_iter()
            .filter_map(|raw| Some((raw, arena.index_of(VertexPointId::new(raw))?)))
            .collect();
        for (raw, index) in mapped {
            assert_eq!(*arena.get(index), raw, "#{raw} addresses another value");
        }
    }

    /// TOP-001 stated as a check rather than as trust: the arena is asked
    /// whether the value it is about to hand over came from the entity the
    /// caller named. Before source identity was retained this question had no
    /// answer at all.
    #[test]
    fn a_checked_lookup_accepts_the_identity_it_stored() {
        let mut arena = Arena::<EdgeKind, &str>::new();
        arena.get_or_try_insert(EdgeCurveId::new(10), || Some("A"));
        let index = arena
            .get_or_try_insert(EdgeCurveId::new(30), || Some("C"))
            .unwrap();

        assert_eq!(
            arena.get_checked(index, EdgeCurveId::new(30)).unwrap(),
            &"C"
        );
        assert_eq!(arena.source_id_at(1), Some(EdgeCurveId::new(30)));
    }

    /// The other half: a lookup for the *wrong* entity is refused and says so,
    /// in the form §61 asks for. Reaching this in production would mean the
    /// arena itself is corrupt, which is why the mismatch is constructed here
    /// by hand rather than by a conversion sequence.
    #[test]
    fn a_checked_lookup_names_both_entities_when_it_refuses() {
        let mut arena = Arena::<EdgeKind, &str>::new();
        let index = arena
            .get_or_try_insert(EdgeCurveId::new(714442), || Some("A"))
            .unwrap();

        let err = arena
            .get_checked(index, EdgeCurveId::new(714381))
            .unwrap_err();
        assert_eq!(err.requested, EdgeCurveId::new(714381));
        assert_eq!(err.stored, EdgeCurveId::new(714442));
        assert_eq!(
            err.to_string(),
            "TOP-001 failed: requested #714381, but arena index [0] stores #714442"
        );
    }
}
