# WORK PACKET BG-SOL-P0-BVH — the broad-phase BVH and the `BoundedPiece` abstraction

You are implementing the solver family's broad-phase substrate: a flat-array
BVH over `BoundingBox<Point3>` plus the `BoundedPiece` trait that analytic
faces, Bézier spans, trim segments and intersection candidates all share.
Everything you need is in this document. **Do not read any other spec file** —
this packet is self-contained. It implements the approved design in
`docs/SOLVER_FAMILY_PLAN.md` §2 and §4 (Phase 0, `truck-base` module `bvh`).

```json
{"id":"BG-SOL-P0-BVH","status":"DONE","contracts":["BG-SOL-P0-BVH"],
 "tests_added":5,"deviations":[],"disagreements":[],
 "baseline_failures":[],"notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. **If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-SOL-P0-BVH
class:       design
crates:      [truck-base]
write_allow:
  - vendor/truck/truck-base/src/bvh.rs
read_allow:
  - vendor/truck/truck-base/src/bounding_box.rs
tests_required:
  - bvh_candidate_pairs_matches_brute_force
  - bvh_self_pairs_are_ordered_and_complete
  - bvh_query_returns_intersecting_pieces
  - bvh_build_is_deterministic
  - empty_bvh_has_no_candidate_pairs
budget:      {turns: 50, ctx_tokens: 120000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub struct BoundingBox' vendor/truck/truck-base/src/bounding_box.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub mod bvh' vendor/truck/truck-base/src/lib.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'clippy::unwrap_used' vendor/truck/truck-base/src/bvh.rs"}
```

## Problem

The certified solver family (plan §2) reduces every surface-surface,
surface-curve and curve-curve question to a **broad phase** followed by a
certified narrow phase: BREP → faces → carrier spans → BVH nodes → candidate
span pairs → certified solver. The BVH's job is only to produce candidate
pairs cheaply and **deterministically** (the repo's deterministic-traversal
invariant); it certifies nothing and must never be allowed to look like it
does. The two-phase split is what keeps the certified stage honest: a BVH that
culled by certified boxes would have to defend the boxes, which is the narrow
phase's job.

`truck-base` has no `inari` dependency and no interval arithmetic. The plan's
§4 target signature reads `fn bbox(&self) -> Box3`, but `Box3` is the
*truck-evidence* certified interval box and is out of reach of `truck-base`.
The resolved reading (record it in `disagreements` as decided): the broad-phase
box is `truck_base::bounding_box::BoundingBox<Point3>` — the crate-local
`f64` axis-aligned box, already used by the splines' `roughly_bounding_box()`.
When a certified stage later feeds the BVH it converts its conservative
enclosure outward into a `BoundingBox<Point3>`; the BVH never reasons about
intervals.

## The design — decide nothing, implement it

The scaffold has already declared the module (`pub mod bvh;` in
`truck-base/src/lib.rs`) and this file carries the H-1 deny header. You fill
the file. Do not edit `lib.rs`.

### 1. `impl BoundingBox<Point3>` — the one missing box primitive

`BoundingBox<Point3>` has `new()`, `push`, `is_empty`, `contains`, `min()`,
`max()`, `center()`, `diagonal()`, `diameter()`. It has **no `intersects`**.
Add exactly one method, in `bvh.rs` (not in `bounding_box.rs` — this packet's
write set is single-file and the wave owns that file):

```rust
impl BoundingBox<Point3> {
    /// Whether the two boxes overlap in all three axes (closed boxes; an
    /// empty box never intersects).
    pub fn intersects(&self, other: &Self) -> bool {
        !(self.max().x < other.min().x || other.max().x < self.min().x
            || self.max().y < other.min().y || other.max().y < self.min().y
            || self.max().z < other.min().z || other.max().z < self.min().z)
    }
}
```

`Point3` and `Vector3` come from `crate::cgmath64`. An empty box has
`min() = +inf`, so every `other.max().axis < self.min().axis` comparison is
true and `intersects` is false — correct for free.

### 2. `DerivativeBounds`

```rust
/// Conservative bounds on a piece's first and second partials over its whole
/// domain. An EMPTY `first` box means "no certified derivative bound is
/// available" (e.g. a rational surface, whose derivative control points are
/// not a hull); consumers must not use an empty box for culling. The broad
/// phase only ever reads `bbox`; `derivative_bounds` exists so the solver
/// phases can use the same pieces without re-extraction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DerivativeBounds {
    /// Box containing every first partial of the piece.
    pub first: BoundingBox<Point3>,
    /// Box containing every second partial of the piece.
    pub second: BoundingBox<Point3>,
}

impl DerivativeBounds {
    /// Both bounds unknown (empty boxes).
    pub fn new() -> Self {
        Self { first: BoundingBox::new(), second: BoundingBox::new() }
    }
    /// Whether a certified first-derivative bound is available.
    pub fn is_known(&self) -> bool {
        !self.first.is_empty()
    }
}
```

### 3. `BoundedPiece`

```rust
/// The shared broad-phase abstraction (plan §2): everything that enters the
/// BVH reports a conservative bounding box, optional derivative bounds, and a
/// subdivision into smaller pieces.
pub trait BoundedPiece {
    /// A conservative box containing the piece's image. MUST contain the whole
    /// piece (soundness); looseness is acceptable.
    fn bbox(&self) -> BoundingBox<Point3>;
    /// Conservative bounds on the piece's partials; empty boxes mean unknown.
    fn derivative_bounds(&self) -> DerivativeBounds;
    /// Subdivide into smaller pieces covering the same image; an empty vec is
    /// a valid answer meaning "cannot subdivide".
    fn subdivide(&self) -> Vec<Self>
    where
        Self: Sized;
}
```

### 4. `Bvh<P>` — flat node array, contiguous leaves

```rust
pub struct Bvh<P: BoundedPiece> {
    nodes: Vec<BvhNode>,
    primitives: Vec<u32>,
    _marker: PhantomData<P>,
}

struct BvhNode {
    bbox: BoundingBox<Point3>,
    left: u32,          // u32::MAX when this is a leaf
    right: u32,
    start: u32,         // leaf: half-open range [start, start + count) into `primitives`
    count: u32,         // 0 for interior nodes
}
```

`primitives` holds indices into the `&[P]` slice passed to `build`; leaves own
contiguous ranges of it (this is the "contiguous leaves" requirement). House
rule H-1 applies: `#![deny(clippy::indexing_slicing)]` is in the header, so the
implementation reads nodes with `.get(idx)` and returns `None`→`Unresolved`/
empty answers rather than `nodes[i]`.

`impl<P: BoundedPiece> Bvh<P>` — exact signatures:

```rust
/// Builds the BVH over `pieces`. Deterministic: identical input produces an
/// identical tree and identical query answers.
pub fn build(pieces: &[P]) -> Self;

/// Leaf-box-overlapping primitive pairs (i, j) where i indexes `pieces` of
/// THIS tree and j indexes the OTHER tree's `pieces`. The two trees may be the
/// same object's trees from two different spans; they are NOT required to be
/// different structures. Returns pairs sorted by (i, j).
pub fn candidate_pairs(&self, other: &Self) -> Vec<(usize, usize)>;

/// Self-intersection pairs: (i, j) with i < j whose leaf boxes overlap.
/// Sorted by (i, j). Primitive pairs INSIDE one leaf are included (two
/// distinct pieces in the same leaf can overlap).
pub fn candidate_pairs_self(&self) -> Vec<(usize, usize)>;

/// Indices of pieces whose leaf box intersects `aabb`. Sorted.
pub fn query(&self, aabb: &BoundingBox<Point3>) -> Vec<usize>;

/// The number of primitives this BVH was built over.
pub fn len(&self) -> usize;

/// Whether the BVH has no primitives.
pub fn is_empty(&self) -> bool;
```

### 5. Build algorithm (pre-decided — implement exactly)

- Compute `pieces[i].bbox()` once into a scratch vec; do not call `bbox()`
  repeatedly.
- `primitives = 0..n` (the identity permutation).
- Recursive build over a half-open range `[lo, hi)` of `primitives`:
  - `hi - lo <= 8` (a named `const LEAF_CAP: usize = 8;`) → a leaf node
    carrying `start = lo as u32, count = (hi - lo) as u32` and the union box
    of the range's bboxes. Empty input (`hi == lo`) is impossible in the
    recursive call; the top-level `build(&[])` produces an empty tree with no
    nodes.
  - otherwise: union-box the range; find the longest axis of
    `union.diagonal()`; **stable-sort** the range by the pieces' centroid
    coordinate on that axis (`bbox.center().axis`; `slice::sort_by` is
    stable — do not use `sort_unstable_by`); split at the midpoint
    `lo + (hi - lo) / 2`; recurse both halves; this node is interior with
    `left`/`right` child indices.
- Node array is built **pre-order** (parent before children); child indices are
  `u32` positions in `nodes`.
- An input slice containing an EMPTY box (a degenerate piece) is valid; the
  union box handles it (an empty box contributes `+inf` min, which a union
  `push` simply keeps). Do not special-case it.

Determinism note: `slice::sort_by` is stable, so equal centroids keep their
input order and the whole tree is a pure function of the input slice.

### 6. Pair queries (pre-decided)

- `candidate_pairs`: recurse `(nA, nB)` starting at both roots. If
  `!a.bbox.intersects(b.bbox)` return. If both leaves, emit every
  `(prims[i], prims[j])` across the two leaves. Otherwise recurse the
  cross-product of the non-leaf children (an interior node recurses into both
  children against the other side). At the end `sort()` the pair vec and
  **dedup** (two different leaf paths can reach the same primitive pair when a
  tree is unbalanced against an empty-box leaf; dedup is cheap and makes the
  contract unconditional). The indices in the returned pairs are the primitive
  indices, i.e. `prims` values, not positions.
- `candidate_pairs_self`: at a node, first recurse into `left` and `right`
  separately (their internal self-pairs), then traverse the cross pair
  `(left, right)` with the same pair-traversal as above. At a LEAF, emit every
  pair of distinct indices within that leaf with `i < j`. Collect, then
  sort+dedup.
- `query(aabb)`: walk the tree; at each node whose `bbox` intersects the query
  box, descend; at intersecting leaves emit the leaf's indices. Sort.

Sorting is the determinism contract: the output order of every query method is
lexicographic, independent of traversal.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N`-shaped literal unless that line ends with an `// H-3` comment. This
packet's tests compare boxes exactly (integer/dyadic coordinates, no
tolerances), so it should not need any bare small literals at all. If a
comparison ever needs one, use the house form:

```rust
const EPS: f64 = 1.0e-12; // H-3: <why this slack, dimensionally>
```

on the same line. Run `bash scripts/kernel-gates.sh <your base commit>` yourself
before writing `RESULT.json`.

## GATE-4 / `unscaled_legacy` (the ratchet)

This packet adds NO `unscaled_legacy()` calls. Do not touch
`scripts/unscaled_legacy_ceiling.txt` — the orchestrator owns the ratchet.

## Regression tests (exact names)

Write a test helper `struct Piece { bbox: BoundingBox<Point3> }` implementing
`BoundedPiece` (`derivative_bounds = DerivativeBounds::new()`,
`subdivide = vec![]`), and a small deterministic LCG (copy the `lcg_next`
pattern from `truck-base/src/evidence.rs`'s tests) to generate boxes. Test
`Piece::bbox` returns the stored box. Put the tests in a
`#[cfg(test)] mod tests` inside `bvh.rs` with
`#[allow(clippy::unwrap_used, clippy::expect_used)]` — the module-level H-1
deny does not apply to test assertions, but the allow is required for them to
compile.

1. `bvh_candidate_pairs_matches_brute_force` — build two BVHs over LCG boxes;
   brute-force `intersects` over every (i, j); assert the BVH pair set equals
   the brute-force set (both sorted), and that every reported pair's boxes
   actually intersect.
2. `bvh_self_pairs_are_ordered_and_complete` — one BVH over overlapping LCG
   boxes; assert `candidate_pairs_self()` equals the brute-force set of pairs
   `(i, j)` with `i < j` and boxes intersecting (which includes pairs inside a
   shared leaf).
3. `bvh_query_returns_intersecting_pieces` — a query box known to intersect a
   known subset; assert the returned sorted index set matches the brute-force
   `contains`-overlap answer.
4. `bvh_build_is_deterministic` — build twice on identical input (same pieces,
   cloned); assert `candidate_pairs` and `candidate_pairs_self` are bit-for-bit
   equal across the two trees.
5. `empty_bvh_has_no_candidate_pairs` — `build(&[])`; assert
   `is_empty()`, `len() == 0`, `candidate_pairs`/`candidate_pairs_self`/
   `query` all return empty.

Every other existing truck-base test must stay green — in particular
`tests/bounding_box.rs` (do not add methods to `bounding_box.rs`; your
`intersects` impl lives in `bvh.rs`).

## Done when — run these, all must pass

```
cargo fmt --check -p truck-base
cargo clippy -p truck-base --all-targets --no-deps
cargo test -p truck-base --lib --tests --no-fail-fast
cargo test -p truck-base --doc
cargo check --locked -p truck-base --all-targets
bash scripts/kernel-gates.sh <your base commit>
```

Never run a bare `cargo test`.

## Forbidden

Editing any file outside `write_allow`. Editing `bounding_box.rs` — the
`intersects` impl belongs in `bvh.rs`. Using a non-stable sort (the tree must
be a pure function of its input). Using `HashSet` or any unordered collection
for a query result (the output contract is sorted). Returning unsorted or
deduplicated pairs. Calling `pieces[i].bbox()` more than once per piece during
build. Adding `#[ignore]`. Changing the GATE-4 ceiling.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- a pre-existing test you did not expect to touch is broken → do NOT weaken the
  gate; report it in `disagreements` with the failing test name and the exact
  reason
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it. In `notes`, record the
resolved broad-phase-box decision (`BoundingBox<Point3>`, not the plan's
`Box3`, because `truck-base` has no `inari`) and the leaf capacity used.

Commit on the current branch with subject
`feat(base): broad-phase BVH and BoundedPiece over BoundingBox<Point3> (BG-SOL-P0-BVH)`.
