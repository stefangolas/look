# WORK PACKET BG-CE-003 — the construction-DAG identity algebra (EntityId / Selector / OpId / Op)

You are implementing one item from a formal kernel specification. Everything
you need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-CE-003","status":"DONE","contracts":["BG-CE-003"],
 "tests_added":9,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: the claims below were
validated by compiling and RUNNING the entire design in a scratch crate, but
they are exactly the kind of claim that can be confidently wrong. **If
anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-CE-003
contract:    [BG-CE-003]
class:       design
crates:      [truck-topology]
write_allow:
  - vendor/truck/truck-topology/src/lib.rs
  - vendor/truck/truck-topology/src/entity_id.rs
  - vendor/truck/truck-topology/Cargo.toml
  - Cargo.lock
read_allow:
  - vendor/truck/truck-topology/src/compress.rs
  - vendor/truck/truck-topology/src/vertex.rs
  - vendor/truck/truck-topology/src/edge.rs
tests_required:
  - stable_hasher_known_answer
  - entity_id_same_construction_yields_same_id
  - entity_id_distinct_constructions_yield_distinct_ids
  - entity_id_serialise_round_trip_preserves_ids
  - entity_id_derivation_never_mutates_the_base
  - entity_id_invariant_under_rigid_motion_and_scale
  - entity_id_slot_distinguishes_outputs
  - entity_id_selector_paths_compose
  - entity_id_bitwise_equality_semantics
budget:      {turns: 40, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 12, cmd: "grep -r 'will result in a deadlock' vendor/truck/truck-topology/src | wc -l"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub type VertexID<P> = ID<Mutex<P>>' vendor/truck/truck-topology/src/lib.rs"}
  - {id: A3, expect: 0, cmd: "grep -r 'enum EntityId\\|struct EntityId' vendor/truck | wc -l"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub struct SourceEntityId' vendor/truck/truck-topology/src/compress.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'pub mod errors' vendor/truck/truck-topology/src/lib.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub mod face' vendor/truck/truck-topology/src/lib.rs"}
  - {id: A7, expect: 7, cmd: "grep -c 'pub mod' vendor/truck/truck-topology/src/lib.rs"}
  - {id: A8, expect: 1, cmd: "grep -c 'serde = ' vendor/truck/truck-topology/Cargo.toml"}
  - {id: A9, expect: 0, cmd: "grep -c 'serde_json' vendor/truck/truck-topology/Cargo.toml"}
  - {id: A10, expect: 1, cmd: "grep -c 'dev-dependencies' vendor/truck/truck-topology/Cargo.toml"}
  - {id: A11, expect: 1, cmd: "grep -c 'use parking_lot::Mutex' vendor/truck/truck-topology/src/lib.rs"}
```

(`grep -c` and `grep -r` exit 1 on zero matches — that IS the expected count
for A9, not a command failure. A3's `grep -r ... | wc -l` form counts matching
LINES — zero when neither name exists yet; do NOT substitute `grep -rc` there,
which prints one line per file regardless of matches.)

## Problem

truck's topology identifies entities by **allocation**: `VertexID<P> =
ID<Mutex<P>>` is a raw pointer to a heap cell (lib.rs:224, anchor A2). Cloning
is identity; mutating the geometry keeps the id; two runs build different ids
for the same construction; serialising and reloading changes every id. §20 of
the formal system needs the opposite: **identity is a pure function of the
construction DAG** — same construction, same id, forever, across processes
and serialisation. The spec sketches the enum:

```rust
pub enum EntityId {
    Src(u64),
    Op { op: OpId, inputs: Box<[EntityId]>, slot: u32 },
    Sel { base: Box<EntityId>, selector: Selector },
}
```

but `Selector`, `OpId` and `Op` are defined **nowhere** in the tree (anchor
A3) — `Sel { base, selector }` is a name, not a design. This packet designs
and lands the algebra as a **standalone module**: no truck geometry types, no
`Mutex`, no `Arc`, no dependency on any other module of the crate — pure
data, one stable hash, serde, and property tests. The `Arc<Mutex<G>> -> Arc<G>`
migration and the 12 documented deadlock hazards (anchor A1 — two per file in
`edge.rs`, `face.rs`, `shell.rs`, `solid.rs`, `vertex.rs`, `wire.rs`) are a
**separate follow-on row (BG-CE-003-MIGRATE)**; do NOT touch any of those
files or the locking. Your write set is one new file, two small edits to
`lib.rs`, one dev-dependency, and the lock.

**Fence — the near-collision.** `vendor/truck/truck-topology/src/compress.rs:64`
already defines `pub struct SourceEntityId(u64)` — the STEP-import compression
metadata type. It is a DIFFERENT type for a different purpose; do not touch
it, do not rename anything around it, do not wire it into `EntityId`. The
`u64` of `EntityId::Src` plays the same *role* (serial import index) but the
two types stay unrelated until an importer packet wires them. Anchor A4 pins
its presence.

## Decisions already made for you

Every type below was compiled and tested in a scratch crate against the real
serde/rustc toolchain before this packet was written. Two hard-won facts are
baked into the design — do not "fix" them:

- **`f64` implements neither `Eq` nor `Hash` in std** (bit-equality breaks
  std's hash contract because `-0.0 == 0.0` with different bits), so
  `OpParams` carries MANUAL bit-wise `PartialEq`, `Eq` and `Hash` impls.
  Deriving any of the three on it does not compile.
- **`Hasher` has no stable float/str/length-prefix write methods on this
  toolchain** (`write_f64`/`write_bool`/`write_char` are not trait methods;
  `write_str`/`write_length_prefix` are unstable). `StableHasher` implements
  only the stable integer/byte methods; floats reach it as `to_bits()`
  through the integer writes that std's `Hash for f64` performs.

### 1. The stable hasher, verbatim

```rust
use std::hash::{Hash, Hasher};

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
#[derive(Default)]
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
```

`StableHasher` is `pub` (the KAT test constructs it directly); `stable_hash`
can stay private. Do not add `Default`'s `new` clippy collision — `Default` is
derived AND `new` is defined; that is the standard allowed pattern
(`clippy::new_without_default` fires without the derive — keep the derive).

### 2. The algebra, verbatim

```rust
use serde::{Deserialize, Serialize};

/// BG-CE-003: the identity of a geometric entity — a pure function of the
/// construction DAG. No arm carries geometry: an id records what the
/// construction SAID, never something measured from a result.
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
            (Point(a), Point(b)) => {
                a.iter().zip(b.iter()).all(|(x, y)| x.to_bits() == y.to_bits())
            }
            (Matrix(a), Matrix(b)) => {
                a.iter().zip(b.iter()).all(|(x, y)| x.to_bits() == y.to_bits())
            }
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
```

The vocabulary is grounded: every `OpKind` verb is a real construction entry
point in `truck-modeling`'s builder (tsweep/rsweep, homotopy, attach_plane,
transformed/translated/rotated/scaled, the primitives) or `truck-shapeops`
(boolean, fillet, offset); every `Selector` arm is a structural feature the
spec's own downstream items reference (boundary wires for INV-105, wire
edges, edge endpoints, the seam for BG-CE-001's cylinder test, the cone apex
of §16.1, the poles of BG-EVD-004). Do not add or remove arms — extending the
vocabulary is a spec amendment, not a worker decision.

### 3. The wire-up

`entity_id.rs` opens with the H-1 deny block (GATE-1 gates new kernel files
on it — copy the form from truck-evidence's `lib.rs:27-34`, i.e.
`#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic,
clippy::todo, clippy::unimplemented, clippy::indexing_slicing)]` as inner
attributes at the file top). truck-topology also warns `missing_docs` and
denies `warnings` in release builds — **every public item carries a doc
comment**, exactly as written above (the sketches already include them).

In `lib.rs`: add `pub mod entity_id;` in the module block between
`pub mod errors;` and `pub mod face;` (anchors A5/A6 bracket the insertion
point; A7's count becomes 8), and immediately after the module block:

```rust
pub use entity_id::{End, EntityId, Op, OpId, OpKind, OpParams, Selector};
```

None of these names collide with anything in the tree (verified by grep
before this packet was written; `End` has no other binding in
truck-topology).

In `Cargo.toml`, under the existing empty `[dev-dependencies]` section
(anchor A10):

```toml
serde_json = "1.0"
```

(serde_json 1.0 is already in the workspace's lock via the root crate and
four sibling truck crates, so this only adds the dependency edge.) Then run
`cargo check -p truck-topology` ONCE WITHOUT `--locked` to update the root
`Cargo.lock`, and commit `Cargo.toml` and `Cargo.lock` together — a
`--locked` run before the lock is updated will refuse. The lock change is
expected and is why `Cargo.lock` is in your write set.

### 4. Tests — the KAT is load-bearing

The known-answer test pins the ENTIRE hash pipeline end to end. These
constants were computed by running exactly the code above on this toolchain:

- a fresh `StableHasher`'s `finish()` (i.e. `fmix64(FNV_OFFSET_BASIS)`):
  `0xefd01f60ba992926`
- `Op { kind: OpKind::Sweep, params: OpParams::Point([1.0, 2.0, 3.0]) }
  .id().0`:
  `0xee38828cf99fd120`
- `Op { kind: OpKind::Transform, params: OpParams::Matrix([1.0, 0.0, 0.0,
  0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]) }.id().0`:
  `0x2dc8dada62ef7c61`

**If a KAT fails, your implementation deviates from the specified hash
pipeline — fix the implementation, never the constant.** (The KAT transitively
pins std's derive-`Hash` discriminant routing on this toolchain; if a
toolchain change ever breaks it, that is a real event to report, not to patch
over.)

The required tests, by name, all in `entity_id.rs`'s `#[cfg(test)]` module
(opening `#[allow(clippy::unwrap_used, clippy::expect_used)]` with the
standard H-1 justification comment — the tests are hand-built witnesses, not
untrusted-geometry paths):

- `stable_hasher_known_answer` — assert the three constants above, plus
  determinism: two fresh hashers agree, and `Op::id()` is stable across two
  independently built `Op`s.
- `entity_id_same_construction_yields_same_id` — build the same DAG twice
  through independent constructor calls (a `Src`, a `Sel` chain, an `Op`
  with multiple inputs, an `Op` whose inputs are other `Op` outputs); the
  id trees are equal, and a hand-`Clone` of the same `Op` gives the same
  `OpId`.
- `entity_id_distinct_constructions_yield_distinct_ids` — a corpus over
  all 8 `OpKind`s × a dozen distinct `OpParams` (unit, bools, indices,
  scalars of both signs and magnitudes, points, nested lists): every `OpId`
  pairwise distinct (collect into a `HashSet`, assert set size == corpus
  size).
- `entity_id_serialise_round_trip_preserves_ids` — the contract test: a
  DAG corpus (`Src`, nested `Sel`, `Op` outputs with different slots,
  deeply composed trees) through `serde_json::to_string` and
  `serde_json::from_str`, asserting equality per id. This is the
  "serialising and reloading preserves all ids" contract; serde_json is the
  dev-dependency added in decision 3.
- `entity_id_derivation_never_mutates_the_base` — clone a base id tree,
  derive selections and op outputs off it, assert the base is unchanged and
  deriving twice yields equal derived ids (derivation is clone-and-extend;
  values are immutable).
- `entity_id_invariant_under_rigid_motion_and_scale` — the spec test's
  standalone form: build a source id; build `Op::Transform` ids carrying
  two DIFFERENT motion matrices over it; the source's id is unchanged by
  their existence; the two transform outputs differ from each other and
  from the source; the SAME matrix applied to two clones of the source
  yields EQUAL output ids (the id is a function of construction content
  only — geometry values cannot affect an id that never sees them).
- `entity_id_slot_distinguishes_outputs` — same op, same inputs, slots
  0/1/2: pairwise distinct ids.
- `entity_id_selector_paths_compose` — `Sel(Sel(Sel(src, Seam),
  WireEdge(3)), End(End::Front))`-style deep paths: two equal paths equal,
  paths differing in one index differ, round-trip through serde_json.
- `entity_id_bitwise_equality_semantics` — `Scalar(0.0) != Scalar(-0.0)`
  (and their ops differ); `Scalar(NAN) == Scalar(NAN)` and
  `Op{params: Scalar(NAN)}.id()` is self-equal (NaN is id-stable by bit
  pattern).

Also add one doctest each to `EntityId` (build a small src→op→sel chain,
assert equality with a rebuilt copy) and `Op` (`id()` determinism) — they run
in `--doc` and are part of Done-when.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N` literal (the regex catches `1e-6`, `1.0e-6`, ...) unless that same
line ends with an `// H-3` comment. It is a text gate on the diff: it does
not know your literal is a parameter value, and it does not care that the
line is in a test. This packet's code uses NO scientific-notation literals
at all — coordinates are `1.0`/`2.0`/`3.0`/`0.5`-class decimals, hash
constants are hex integers — so H-3 should never bite. If you ever do write
a bare `1e-N` literal, the line must end with a same-line `// H-3:` comment
naming the dimensionless quantity. Run `bash scripts/kernel-gates.sh`
yourself before you write `RESULT.json`; it is the same script V4 runs.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-topology
cargo clippy -p truck-topology --all-targets --no-deps
cargo test -p truck-topology --lib --tests --no-fail-fast
cargo test -p truck-topology --doc
cargo check --workspace --all-targets
bash scripts/kernel-gates.sh <your base commit>
```

Never run a bare `cargo test` — it builds 56 examples. Send cargo output to a
file and read the tail.

**This crate is clean at baseline** — measured at the tree this packet was
written against (HEAD 2cf9094, the same tree BG-CE-001's verify ran green
on): all lib/integration tests pass (one `#[ignore]`d test in
`tests/large-solid-torus.rs` is pre-existing — leave it ignored), all
doctests pass, and clippy reports zero findings on the whole crate. Your
bar: everything above stays green, plus your nine new tests and two new
doctests. There are no baseline failures to tolerate — any failure you did
not cause is a stop condition, and any failure you did cause is yours to
fix.

## Forbidden

Editing any file outside `write_allow` — especially `compress.rs` (the
`SourceEntityId` fence), `vertex.rs`/`edge.rs`/`face.rs`/`wire.rs`/
`shell.rs`/`solid.rs` (the `Arc<Mutex>` migration is NOT this packet), and
every other crate. Removing or adding `OpKind`/`Selector`/`OpParams` arms.
Changing the hash pipeline (constants, write order, tags) — the KATs pin it.
Replacing the manual `PartialEq`/`Eq`/`Hash` on `OpParams` with derives
(does not compile) or with value-based float equality (breaks the hash
contract). Adding `#[ignore]`. Adding `unwrap()`/`expect()` on fallible
paths in production code. Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- a KAT constant does not reproduce after typing the specified pipeline
  exactly → `SPEC_GAP` with the constant you got and the exact code that
  produced it — do not adjust the constant and do not improvise a different
  hash
- serde (de)serialisation of `Box<[EntityId]>` / `[f64; 16]` / the enums
  fails to compile → `SPEC_GAP` (the scratch crate compiled all of it, but
  on serde derive's mercy)
- `cargo check --workspace --all-targets` fails in a crate outside
  truck-topology after your change → `SPEC_GAP`: report the file and the
  exact error; do not fix it
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(topology): construction-DAG identity algebra (BG-CE-003)`.
