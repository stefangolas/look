//! BG-CE-003: the construction-DAG identity algebra.
//!
//! `EntityId`, `OpId`, `Op` and `Selector` give every geometric entity a
//! stable identity that is a pure function of the construction DAG: the same
//! construction yields the same ids forever, across processes and
//! serialisation. No arm carries geometry — an id records what the
//! construction said, never a measurement from a result.
//!
//! This module is standalone by design: pure data, one stable hash, serde and
//! property tests. It touches no truck geometry types, no `Mutex`, no `Arc`,
//! and depends on no other module of the crate.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

/// FNV-1a 64-bit offset basis.
const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
/// FNV-1a 64-bit prime.
const FNV_PRIME: u64 = 0x00000100000001b3;

/// A process-, platform- and toolchain-stable hash: FNV-1a over the `Hash`
/// byte stream, finalized by MurmurHash3's `fmix64`. Unlike
/// `std::hash::DefaultHasher`, the output is a property of this crate's
/// source, not of the std implementation. All integer writes are
/// little-endian so the byte stream is endianness-independent; `usize`
/// writes as `u64` (every target of this workspace is 64-bit).
#[derive(Debug, Default)]
pub struct StableHasher(u64);

impl StableHasher {
    /// A fresh hasher at the offset basis.
    pub fn new() -> Self {
        StableHasher(FNV_OFFSET_BASIS)
    }

    fn byte(&mut self, b: u8) {
        self.0 ^= u64::from(b);
        self.0 = self.0.wrapping_mul(FNV_PRIME);
    }
}

/// MurmurHash3's 64-bit finalizer.
fn fmix64(mut k: u64) -> u64 {
    k ^= k >> 33;
    k = k.wrapping_mul(0xff51afd7ed558ccd);
    k ^= k >> 33;
    k = k.wrapping_mul(0xc4ceb9fe1a85ec53);
    k ^= k >> 33;
    k
}

impl Hasher for StableHasher {
    fn finish(&self) -> u64 {
        fmix64(self.0)
    }

    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.byte(b);
        }
    }

    fn write_u8(&mut self, i: u8) {
        self.byte(i);
    }

    fn write_u16(&mut self, i: u16) {
        self.write(&i.to_le_bytes());
    }

    fn write_u32(&mut self, i: u32) {
        self.write(&i.to_le_bytes());
    }

    fn write_u64(&mut self, i: u64) {
        self.write(&i.to_le_bytes());
    }

    fn write_u128(&mut self, i: u128) {
        self.write(&i.to_le_bytes());
    }

    fn write_usize(&mut self, i: usize) {
        self.write(&(i as u64).to_le_bytes());
    }

    fn write_i8(&mut self, i: i8) {
        self.byte(i as u8);
    }

    fn write_i16(&mut self, i: i16) {
        self.write(&i.to_le_bytes());
    }

    fn write_i32(&mut self, i: i32) {
        self.write(&i.to_le_bytes());
    }

    fn write_i64(&mut self, i: i64) {
        self.write(&(i as u64).to_le_bytes());
    }

    fn write_i128(&mut self, i: i128) {
        self.write(&i.to_le_bytes());
    }

    fn write_isize(&mut self, i: isize) {
        self.write(&(i as u64).to_le_bytes());
    }
}

/// The stable hash of any hashable value.
fn stable_hash<T: Hash + ?Sized>(value: &T) -> u64 {
    let mut hasher = StableHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

/// BG-CE-003: the identity of a geometric entity — a pure function of the
/// construction DAG. No arm carries geometry: an id records what the
/// construction SAID, never something measured from a result.
///
/// ```
/// use truck_topology::{EntityId, Op, OpKind, OpParams, Selector};
///
/// let op = Op {
///     kind: OpKind::Sweep,
///     params: OpParams::Point([1.0, 2.0, 3.0]),
/// };
/// let a = EntityId::sel(op.output(&[EntityId::src(4)], 0), Selector::Seam);
/// let b = EntityId::sel(op.output(&[EntityId::src(4)], 0), Selector::Seam);
/// assert_eq!(a, b);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityId {
    /// An imported entity, identified by its serial import index.
    Src(u64),
    /// An entity derived by an operation: which operation, from which
    /// inputs, which output slot.
    Op {
        /// The operation's content identity.
        op: OpId,
        /// The identities of the operation's inputs.
        inputs: Box<[EntityId]>,
        /// Which output of the operation this entity is.
        slot: u32,
    },
    /// An entity selected structurally from a base entity. NEVER a
    /// geometric query: selectors are structural paths, not coordinates or
    /// distances.
    Sel {
        /// The entity selected from.
        base: Box<EntityId>,
        /// The structural path.
        selector: Selector,
    },
}

impl EntityId {
    /// The id of the imported entity with serial index `index`.
    pub fn src(index: u64) -> Self {
        EntityId::Src(index)
    }

    /// The id of the sub-entity reached from `base` by `selector`.
    pub fn sel(base: EntityId, selector: Selector) -> Self {
        EntityId::Sel {
            base: Box::new(base),
            selector,
        }
    }
}

/// The content identity of an operation node: the stable hash of its [`Op`].
/// Two `Op`s with equal content have equal ids — identity is content, never
/// allocation. The field is public so ids can be stored and compared; hand-
/// constructing an `OpId` without an `Op` forges identity and is a caller
/// defect.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OpId(pub u64);

/// One node of the construction DAG: the construction verb plus its
/// parameters.
///
/// ```
/// use truck_topology::{Op, OpKind, OpParams};
///
/// let a = Op {
///     kind: OpKind::Sweep,
///     params: OpParams::Index(3),
/// };
/// let b = Op {
///     kind: OpKind::Sweep,
///     params: OpParams::Index(3),
/// };
/// assert_eq!(a.id(), b.id());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Op {
    /// The construction verb.
    pub kind: OpKind,
    /// What the verb was told: construction data, never a measurement.
    pub params: OpParams,
}

impl Op {
    /// This operation's content identity.
    pub fn id(&self) -> OpId {
        OpId(stable_hash(self))
    }

    /// The id of the `slot`-th output of this operation applied to `inputs`.
    pub fn output(&self, inputs: &[EntityId], slot: u32) -> EntityId {
        EntityId::Op {
            op: self.id(),
            inputs: inputs.into(),
            slot,
        }
    }
}

/// The kernel's construction verbs. A closed vocabulary: it extends only
/// with a spec amendment, in the same breaking data-model release as the
/// rest of the CE items.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OpKind {
    /// A primitive placed by parameters (line, arc, bezier, cone, ...).
    Primitive,
    /// Sweeping: translational (tsweep) or rotational (rsweep).
    Sweep,
    /// Homotopy/loft between curves or wires.
    Loft,
    /// Plane attachment to wires (attach_plane).
    Attach,
    /// Boolean union / intersection / difference.
    Boolean,
    /// Fillet and chamfer.
    Fillet,
    /// Offset, shell and hollow.
    Offset,
    /// Rigid motion or scale applied to a construction.
    Transform,
}

/// Construction parameters: a small closed value language. Floats compare
/// and hash BY BITS: `-0.0` and `0.0` are different constructions, and a NaN
/// with a given bit pattern is equal to itself (id-stable). `f64` implements
/// neither `Eq` nor `Hash` in std, so all three impls here are manual.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum OpParams {
    /// No parameters.
    Unit,
    /// A boolean switch.
    Bool(bool),
    /// A count, division or index.
    Index(u32),
    /// A length, angle or ratio.
    Scalar(f64),
    /// A position or direction.
    Point([f64; 3]),
    /// A 4x4 transform, row-major.
    Matrix([f64; 16]),
    /// An ordered parameter list.
    List(Vec<OpParams>),
}

/// Bit-wise equality: equal bits are equal constructions.
impl PartialEq for OpParams {
    fn eq(&self, other: &Self) -> bool {
        use OpParams::*;
        match (self, other) {
            (Unit, Unit) => true,
            (Bool(a), Bool(b)) => a == b,
            (Index(a), Index(b)) => a == b,
            (Scalar(a), Scalar(b)) => a.to_bits() == b.to_bits(),
            (Point(a), Point(b)) => a
                .iter()
                .zip(b.iter())
                .all(|(x, y)| x.to_bits() == y.to_bits()),
            (Matrix(a), Matrix(b)) => a
                .iter()
                .zip(b.iter())
                .all(|(x, y)| x.to_bits() == y.to_bits()),
            (List(a), List(b)) => a == b,
            _ => false,
        }
    }
}

/// Bit-wise equality is an equivalence relation.
impl Eq for OpParams {}

/// Bit-wise hashing, consistent with bit-wise equality. Variant tags are
/// explicit (0u8..=6u8) so the byte stream is a property of this source,
/// not of derive internals.
impl Hash for OpParams {
    fn hash<H: Hasher>(&self, state: &mut H) {
        use OpParams::*;
        match self {
            Unit => 0u8.hash(state),
            Bool(b) => {
                1u8.hash(state);
                b.hash(state);
            }
            Index(i) => {
                2u8.hash(state);
                i.hash(state);
            }
            Scalar(x) => {
                3u8.hash(state);
                x.to_bits().hash(state);
            }
            Point(p) => {
                4u8.hash(state);
                p.iter().for_each(|x| x.to_bits().hash(state));
            }
            Matrix(m) => {
                5u8.hash(state);
                m.iter().for_each(|x| x.to_bits().hash(state));
            }
            List(xs) => {
                6u8.hash(state);
                xs.hash(state);
            }
        }
    }
}

/// Which end of an edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum End {
    /// The front vertex.
    Front,
    /// The back vertex.
    Back,
}

/// A structural path from an entity to one of its sub-entities. NEVER a
/// geometric query: every arm is an index or a named structural feature, and
/// the type carries no coordinates, distances or directions at all — that
/// is the §20 "never a geometric query" rule made structural.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Selector {
    /// The `i`-th boundary wire of a face or shell.
    BoundaryWire(u32),
    /// The `i`-th edge of a wire, in wire order.
    WireEdge(u32),
    /// An endpoint of an edge.
    End(End),
    /// The seam of a periodic carrier.
    Seam,
    /// The apex of a cone — a first-class point (§16.1).
    Apex,
    /// The `i`-th pole of a parametric surface, in (u, v) order.
    Pole(u32),
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect on paths reachable from untrusted
// geometry. Unit-test assertions on hand-built witnesses are not such a path;
// these unwraps cannot fire for the values constructed below.
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn stable_hasher_known_answer() {
        assert_eq!(StableHasher::new().finish(), 0xefd01f60ba992926);

        let sweep = Op {
            kind: OpKind::Sweep,
            params: OpParams::Point([1.0, 2.0, 3.0]),
        };
        assert_eq!(sweep.id().0, 0xee38828cf99fd120);

        let transform = Op {
            kind: OpKind::Transform,
            params: OpParams::Matrix([
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ]),
        };
        assert_eq!(transform.id().0, 0x2dc8dada62ef7c61);

        let mut first = StableHasher::new();
        let mut second = StableHasher::new();
        first.write_u64(42);
        second.write_u64(42);
        assert_eq!(first.finish(), second.finish());

        let rebuilt = Op {
            kind: OpKind::Sweep,
            params: OpParams::Point([1.0, 2.0, 3.0]),
        };
        assert_eq!(sweep.id(), rebuilt.id());
    }

    #[test]
    fn entity_id_same_construction_yields_same_id() {
        let sweep = Op {
            kind: OpKind::Sweep,
            params: OpParams::Scalar(2.0),
        };
        let loft = Op {
            kind: OpKind::Loft,
            params: OpParams::Unit,
        };

        let sel_a = EntityId::sel(EntityId::src(1), Selector::Seam);
        let sel_b = EntityId::sel(EntityId::src(1), Selector::Seam);
        assert_eq!(sel_a, sel_b);

        let out_a = loft.output(&[EntityId::src(1), sweep.output(&[EntityId::src(2)], 0)], 1);
        let out_b = loft.output(&[EntityId::src(1), sweep.output(&[EntityId::src(2)], 0)], 1);
        assert_eq!(out_a, out_b);

        let op_a = Op {
            kind: OpKind::Fillet,
            params: OpParams::Scalar(0.5),
        };
        let op_b = op_a.clone();
        assert_eq!(op_a.id(), op_b.id());
    }

    #[test]
    fn entity_id_distinct_constructions_yield_distinct_ids() {
        let kinds = [
            OpKind::Primitive,
            OpKind::Sweep,
            OpKind::Loft,
            OpKind::Attach,
            OpKind::Boolean,
            OpKind::Fillet,
            OpKind::Offset,
            OpKind::Transform,
        ];
        let params = [
            OpParams::Unit,
            OpParams::Bool(false),
            OpParams::Bool(true),
            OpParams::Index(0),
            OpParams::Index(7),
            OpParams::Index(u32::MAX),
            OpParams::Scalar(0.0),
            OpParams::Scalar(-0.0),
            OpParams::Scalar(1.0),
            OpParams::Scalar(-1000.0),
            OpParams::Point([1.0, 2.0, 3.0]),
            OpParams::List(vec![OpParams::Scalar(0.5), OpParams::Bool(true)]),
        ];
        let mut ids = HashSet::new();
        for kind in kinds {
            for p in &params {
                let op = Op {
                    kind,
                    params: p.clone(),
                };
                ids.insert(op.id());
            }
        }
        assert_eq!(ids.len(), kinds.len() * params.len());
    }

    #[test]
    fn entity_id_serialise_round_trip_preserves_ids() {
        let sweep = Op {
            kind: OpKind::Sweep,
            params: OpParams::Point([1.0, 2.0, 3.0]),
        };
        let corpus = [
            EntityId::src(3),
            EntityId::sel(EntityId::src(1), Selector::Seam),
            EntityId::sel(
                EntityId::sel(EntityId::src(2), Selector::WireEdge(1)),
                Selector::End(End::Front),
            ),
            sweep.output(&[EntityId::src(4)], 0),
            sweep.output(&[EntityId::src(4)], 1),
            sweep.output(
                &[
                    EntityId::src(5),
                    EntityId::sel(EntityId::src(6), Selector::Pole(2)),
                ],
                0,
            ),
        ];
        for id in &corpus {
            let text = serde_json::to_string(id).unwrap();
            let back: EntityId = serde_json::from_str(&text).unwrap();
            assert_eq!(id, &back);
        }
    }

    #[test]
    fn entity_id_derivation_never_mutates_the_base() {
        let base = EntityId::sel(EntityId::src(1), Selector::BoundaryWire(0));
        let base_before = base.clone();

        let derived_a = EntityId::sel(base.clone(), Selector::WireEdge(2));
        let derived_b = EntityId::sel(base.clone(), Selector::WireEdge(2));
        let op = Op {
            kind: OpKind::Attach,
            params: OpParams::Index(1),
        };
        let out_a = op.output(std::slice::from_ref(&base), 0);
        let out_b = op.output(std::slice::from_ref(&base), 0);

        assert_eq!(base, base_before);
        assert_eq!(derived_a, derived_b);
        assert_eq!(out_a, out_b);
        assert_ne!(base, derived_a);
    }

    #[test]
    fn entity_id_invariant_under_rigid_motion_and_scale() {
        let src = EntityId::src(9);
        let identity = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        let scaled = [
            2.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 5.0, -1.0, 1.0,
        ];
        let rigid = Op {
            kind: OpKind::Transform,
            params: OpParams::Matrix(identity),
        };
        let scaled_op = Op {
            kind: OpKind::Transform,
            params: OpParams::Matrix(scaled),
        };

        let before = src.clone();
        let out_rigid_a = rigid.output(std::slice::from_ref(&src), 0);
        let out_rigid_b = rigid.output(std::slice::from_ref(&src), 0);
        let out_scaled = scaled_op.output(std::slice::from_ref(&src), 0);

        assert_eq!(src, before);
        assert_ne!(out_rigid_a, src);
        assert_ne!(out_rigid_a, out_scaled);
        assert_eq!(out_rigid_a, out_rigid_b);
    }

    #[test]
    fn entity_id_slot_distinguishes_outputs() {
        let op = Op {
            kind: OpKind::Loft,
            params: OpParams::Unit,
        };
        let inputs = [EntityId::src(1), EntityId::src(2)];
        let slot0 = op.output(&inputs, 0);
        let slot1 = op.output(&inputs, 1);
        let slot2 = op.output(&inputs, 2);
        assert_ne!(slot0, slot1);
        assert_ne!(slot1, slot2);
        assert_ne!(slot0, slot2);
    }

    #[test]
    fn entity_id_selector_paths_compose() {
        let path = EntityId::sel(
            EntityId::sel(
                EntityId::sel(EntityId::src(0), Selector::Seam),
                Selector::WireEdge(3),
            ),
            Selector::End(End::Front),
        );
        let rebuilt = EntityId::sel(
            EntityId::sel(
                EntityId::sel(EntityId::src(0), Selector::Seam),
                Selector::WireEdge(3),
            ),
            Selector::End(End::Front),
        );
        let other = EntityId::sel(
            EntityId::sel(
                EntityId::sel(EntityId::src(0), Selector::Seam),
                Selector::WireEdge(2),
            ),
            Selector::End(End::Front),
        );
        assert_eq!(path, rebuilt);
        assert_ne!(path, other);

        let text = serde_json::to_string(&path).unwrap();
        let back: EntityId = serde_json::from_str(&text).unwrap();
        assert_eq!(path, back);
    }

    #[test]
    fn entity_id_bitwise_equality_semantics() {
        assert_ne!(OpParams::Scalar(0.0), OpParams::Scalar(-0.0));

        let op_zero = Op {
            kind: OpKind::Primitive,
            params: OpParams::Scalar(0.0),
        };
        let op_neg_zero = Op {
            kind: OpKind::Primitive,
            params: OpParams::Scalar(-0.0),
        };
        assert_ne!(op_zero.id(), op_neg_zero.id());

        let nan = OpParams::Scalar(f64::NAN);
        assert_eq!(nan, nan.clone());

        let op_nan = Op {
            kind: OpKind::Primitive,
            params: nan,
        };
        let id_nan_a = op_nan.id();
        let id_nan_b = op_nan.id();
        assert_eq!(id_nan_a, id_nan_b);
    }
}
