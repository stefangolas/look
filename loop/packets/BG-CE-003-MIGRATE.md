# WORK PACKET BG-CE-003-MIGRATE — `Arc<Mutex<G>>` → `Arc<G>` and the replacement API

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-CE-003-MIGRATE","status":"DONE","contracts":["BG-CE-003"],
 "tests_added":6,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: the claims below were
derived by command against the tree, and the design (which API replaces what,
and exactly which files need edits) was decided by reading every call site —
but they are exactly the kind of claim that can be confidently wrong. **If
anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-CE-003-MIGRATE
contract:    [BG-CE-003]
class:       wide-mechanical
crates:      [truck-topology, truck-shapeops, truck-meshalgo]
write_allow:
  - vendor/truck/truck-topology/src/lib.rs
  - vendor/truck/truck-topology/src/vertex.rs
  - vendor/truck/truck-topology/src/edge.rs
  - vendor/truck/truck-topology/src/wire.rs
  - vendor/truck/truck-topology/src/face.rs
  - vendor/truck/truck-topology/src/shell.rs
  - vendor/truck/truck-topology/src/solid.rs
  - vendor/truck/truck-topology/src/invariants/same_parameter.rs
  - vendor/truck/truck-topology/src/entity_id.rs
  - vendor/truck/truck-topology/tests/parallel_query.rs
  - vendor/truck/truck-shapeops/src/transversal/loops_store/mod.rs
  - vendor/truck/truck-shapeops/src/fillet/mod.rs
  - vendor/truck/truck-meshalgo/tests/tessellation/triangulation.rs
read_allow:
  - vendor/truck/truck-topology/src/imported.rs
  - vendor/truck/truck-topology/src/compress.rs
  - vendor/truck/truck-topology/Cargo.toml
  - vendor/truck/truck-shapeops/src/transversal/integrate/mod.rs
  - vendor/truck/truck-shapeops/src/fillet/experiment.rs
  - vendor/truck/truck-modeling/src/mapped.rs
  - vendor/truck/truck-meshalgo/src/tessellation/triangulation.rs
tests_required:
  - vertex_replacement_changes_id_not_old_handles
  - with_curve_preserves_topology
  - with_surface_preserves_boundaries
  - mapped_closure_may_access_geometry
  - replaced_id_derives_stably
  - parallel_query_never_deadlocks
budget:      {turns: 44, ctx_tokens: 105000}
anchors:
  - {id: A1, expect: 12, cmd: "grep -r 'will result in a deadlock' vendor/truck/truck-topology/src | wc -l"}
  - {id: A2, expect: 3, cmd: "grep -c 'Arc<Mutex<' vendor/truck/truck-topology/src/lib.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub fn set_point' vendor/truck/truck-topology/src/vertex.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub fn set_curve' vendor/truck/truck-topology/src/edge.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'pub fn set_surface' vendor/truck/truck-topology/src/face.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'VertexID<P> = ID<Mutex<P>>' vendor/truck/truck-topology/src/lib.rs"}
  - {id: A7, expect: 3, cmd: "grep -r 'set_point(' vendor/truck/truck-shapeops/src | wc -l"}
  - {id: A8, expect: 1, cmd: "grep -c 'set_curve' vendor/truck/truck-shapeops/src/fillet/mod.rs"}
  - {id: A9, expect: 1, cmd: "grep -c 'set_curve' vendor/truck/truck-meshalgo/tests/tessellation/triangulation.rs"}
  - {id: A10, expect: 1, cmd: "grep -c 'edge.curve.lock()' vendor/truck/truck-topology/src/invariants/same_parameter.rs"}
  - {id: A11, expect: 1, cmd: "grep -c 'pub enum OpKind' vendor/truck/truck-topology/src/entity_id.rs"}
  - {id: A12, expect: 1, cmd: "grep -c 'use parking_lot::Mutex' vendor/truck/truck-topology/src/lib.rs"}
  - {id: A13, expect: 1, cmd: "grep -c 'pub fn try_mapped' vendor/truck/truck-topology/src/wire.rs"}
  - {id: A14, expect: 1, cmd: "grep -c 'pub fn mapped' vendor/truck/truck-topology/src/solid.rs"}
```

(`grep -c` exits 1 on zero matches — a count of 0 IS the expected answer where
the packet says so, not a command failure. A1/A7 pipe through `wc -l`, which
counts matching LINES.)

## Problem

Geometry in `truck-topology` lives in shared mutable cells:
`Vertex { point: Arc<Mutex<P>> }`, `Edge { curve: Arc<Mutex<C>>, .. }`,
`Face { surface: Arc<Mutex<S>>, .. }` (with `Mutex` = `parking_lot::Mutex`,
`Arc` = `rclite::Arc` under the default feature). Mutation through a shared
handle (`set_point`/`set_curve`/`set_surface`) is documented to keep the
allocation id, and the `mapped`/`try_mapped` family carries **12 doc remarks
saying "Accessing geometry elements directly in the closure will result in a
deadlock"** — 2 per file across vertex/edge/wire/face/shell/solid, each method
pair `#[doc(hidden)]`. §20 of the formal system needs the opposite regime:
immutable geometry, replacement instead of mutation, identity as a function of
construction.

This packet migrates the storage, replaces the mutation API, un-hides the
mapping family, adds the replacement-event id derivation to the landed
identity algebra (`entity_id.rs`), migrates every live call site, and lands
the 8-rayon-thread regression test that pins the whole point: with no locks,
parallel construction and query cannot deadlock.

**What does NOT change** (measured by grep against the tree — every one of
these is signature-compatible and must end the packet untouched):
`truck-modeling/{sweep,multi_sweep,closed_sweep,mapped,builder}.rs` (they only
CALL `mapped`, whose signature is unchanged), `truck-shapeops/src/transversal/
integrate/mod.rs` (calls `try_mapped`/`mapped` — unchanged),
`truck-meshalgo/src/tessellation/triangulation.rs` (`v.mapped(Point3::clone)`
at 2 sites — unchanged), all of `truck-stepio` (`mapped_assembly` is an
unrelated local test helper name), `truck-shapeops/src/fillet/experiment.rs`
(dead code, not compiled — never edit it), and every `VertexID`/`EdgeID`/
`FaceID` alias use (the aliases change definition, not spelling).
`parking_lot` STAYS in `Cargo.toml` (the `nightly` feature references it); only
the `use` dies. No manifest edits at all — `rayon` is already a regular
dependency of truck-topology.

## Decisions already made for you

### 0. The storage change, in `lib.rs`

- The three struct fields: `Arc<Mutex<P>>` → `Arc<P>`,
  `Arc<Mutex<C>>` → `Arc<C>`, `Arc<Mutex<S>>` → `Arc<S>`.
- Delete `use parking_lot::Mutex;` (A12's count goes to 0; it is the only
  parking_lot use in the crate).
- The id aliases: `pub type VertexID<P> = ID<P>;` (likewise `EdgeID<C>`,
  `FaceID<S>` — the `ID<Mutex<..>>` forms die). Pointer identity over the
  fresh cell, exactly as before mechanically.
- `format::MutexFmt` (lib.rs's Debug bridge that locks) is DELETED; the Debug
  impls in vertex/edge/face that use it format `&*self.point` /
  `&*self.curve` / `&*self.surface` directly (`{:?}` of the inner value — same
  output text as today's lock-and-format).
- The `VertexID` doc block at lib.rs ("The id does not changed even if the
  value of point changes", with the `set_point` doctest) is REWRITTEN to state
  the replacement semantics: replacing a point constructs a new vertex with a
  new id; existing handles keep the old geometry. Keep the HashMap idiom
  example; replace the mutation example with
  `let v2 = Vertex::new(1); assert_ne!(v_id, v2.id());`.

### 1. `vertex.rs` — mutation dies, `mapped` is un-hidden

- `point()`: `(*self.point).clone()` — same signature (`P: Clone`).
- `set_point`: DELETED. The replacement is the constructor — `Vertex::new`
  IS the replacement API for points, and the packet's docs say so where
  `set_point` used to be documented. Do NOT add a `with_point`.
- `try_mapped` / `mapped`: keep signatures; bodies lose the lock
  (`point_mapping(&*self.point)`); DELETE `#[doc(hidden)]` and the deadlock
  remark; write real doc examples (see decision 8's test for the shape).
- `id()`, `count()`, `news()`: unchanged (`Arc::as_ptr`,
  `Arc::strong_count`, `rclite` supports both).
- The struct-level docs referencing `set_point` (lib.rs:219's doctest and
  vertex.rs's own) migrate to the constructor-as-replacement phrasing.

### 2. `edge.rs` — `with_curve`, `shared_curve`, and the lock-pair that dies

- `curve()` (`self.curve.lock().clone()`) → `(*self.curve).clone()` — same
  signature.
- `set_curve` → DELETED, replaced by:

  ```rust
  /// BG-CE-003: replacement, never in-place mutation. A fresh edge with the
  /// same vertices (same handles — the topology is shared, not copied),
  /// the same orientation and pcurve payload, and the given curve: a new id.
  pub fn with_curve(&self, curve: C) -> Edge<P, C, PC>
  ```

  (build via the struct's private fields or `Edge::new_unchecked` on cloned
  vertices — match how the crate's own constructors assemble edges; the
  doctest where `set_curve`'s was shows the new id and the unchanged old
  handle).
- NEW generic accessor (works for every `PC`, unlike `curve()` which is
  bounded to `PC = ()`):

  ```rust
  /// The shared entity curve by reference — no lock, no clone.
  pub fn shared_curve(&self) -> &C { &self.curve }
  ```
- `try_mapped`/`mapped` (238/275): signatures kept; `curve_mapping(&*self.curve)`;
  un-hide + delete remarks + doc examples.
- `inverse()`/`dereverse()`-family bodies at 226-227 (`self.curve.lock().inverse()`
  → `(*self.curve).inverse()` or `self.curve.inverse()` as borrow rules allow).
- `is_geometric_consistent` (298-304): the three simultaneous locks (curve +
  both endpoint points — the undocumented cross-entity deadlock) become plain
  borrows; keep the FIXME comment untouched.
- The `set_curve` doctest (≈178-186) migrates to `with_curve`.

### 3. `face.rs` — the surface mirror of decision 2

- `surface()` → deref-clone; `set_surface` DELETED → `with_surface(&self,
  surface: S) -> Face<P, C, S>` (same boundaries — same wire handles — same
  orientation, new surface, new id) + `shared_surface(&self) -> &S`.
- `try_mapped`/`mapped` (415/487): bodies at 426/498 lose `face.surface.lock()`
  → `&*face.surface`; un-hide, delete remarks, doc examples.
- The `inverse`-family bodies at 1101-1102 and 1117-1120: locks → borrows.
- The `set_surface` doctest (≈535-549) migrates.

### 4. `wire.rs`, `shell.rs`, `solid.rs` — the container mapping bodies

Only lock-touching bodies and docs change; no public signature moves:

- `wire.rs`: `sub_try_mapped` (410) / `try_mapped` (435) / `mapped` (509) —
  the `edge.curve.lock()` reaches at 588/606 become `&*edge.curve`; the
  `Vertex::id`-keyed EntryMap plumbing is untouched; un-hide the pub pair,
  delete remarks, add doc examples.
- `shell.rs`: `try_mapped` (565) / `mapped` (659) — bodies at 583/677; same
  treatment.
- `solid.rs`: `try_mapped` (101) / `mapped` (124) — bodies at 111/134; the
  `Clone` impl's `self.try_mapped(Clone::clone, ..)` at 230 is untouched;
  same treatment.

### 5. `invariants/same_parameter.rs` — the mutex reach

Line 89's `let leader = edge.curve.lock();` (with the comment explaining the
`PC = ()` accessor limitation) becomes `let leader = edge.shared_curve();`
and the comment updates to name the accessor instead of the mutex. Nothing
else in the file moves.

### 6. `entity_id.rs` — the replacement event in the algebra (additive)

- `OpKind` gains one arm, alphabetically placed, doc comment included:

  ```rust
  /// A payload replaced by value (BG-CE-003-MIGRATE): the input is the
  /// replaced entity, the params carry the replacement value.
  Replace,
  ```

  There is no exhaustive `match` on `OpKind` in the tree (verified — count 0),
  so this compiles everywhere; serde round-trips the new arm; the pinned KAT
  constants hash specific values and do not move.
- `EntityId` gains the derivation helper:

  ```rust
  /// The id of an entity produced by REPLACING `self`'s payload: an `Op`
  /// node with kind `Replace`, the given params, `self` as the only input,
  /// slot 0. A pure function of (old id, params) — two replacements with
  /// equal params from equal ids yield equal ids, and distinct params from
  /// one id yield distinct ids.
  pub fn replaced(&self, params: &OpParams) -> EntityId
  ```

- Tests for the arm live in this file's test module (decision 8 names them).

### 7. The live call sites — exactly three files

**`truck-shapeops/src/transversal/loops_store/mod.rs`** — the subtle one.
`add_geom_vertex` (366) mutates `v` via `set_point` and then swaps it in with
`change_vertex`; the caller (≈530-589) passes THE SAME `gv0`/`gv1` handles to
BOTH surfaces' stores and reads the final points back into the polyline
leader (`*polyline.first_mut().unwrap() = gv0.point()`), so the shared
mutation carries information across the two calls. The replacement keeps the
information flow explicit:

- `add_geom_vertex` becomes `fn add_geom_vertex(...) -> Option<Vertex<Point3>>`
  — each arm builds the effective vertex by CONSTRUCTION and returns it:
  - Front/Back: `let v2 = Vertex::new(old_vertex.point());` then
    `self.change_vertex(&old_vertex, &v2, emap);` then `return v2` — identical
    semantics to the old `v.set_point(old_vertex.point()); change_vertex(..v)`.
  - Inner: project FIRST (`pt` as today), then `let v2 = Vertex::new(pt);`,
    then `edge.cut_with_parameter(&v2, t)` and `swap_edge_into_wire`, return
    `v2`.
- The four call sites (≈538-589) reassign:
  `gv0 = geom_loops_store0.add_geom_vertex(..)?;` (and gv1, and the store-1
  pair) — declaring `let mut gv0`/`let mut gv1` at 530-533. The readbacks and
  the closing `Edge::new(&gv0, &gv1, ..)` then use the effective vertices,
  exactly as the mutation semantics did.
- The polygon-side `add_polygon_vertex` performs NO mutation today and needs
  no change.

**`truck-shapeops/src/fillet/mod.rs`** (line 414-415) — shared mutation is
load-bearing here too: `new_boundary` already holds `fillet_edge` clones when
`set_curve` fires. The fix is a construction-order swap:

```rust
let new_curve = IntersectionCurve::new(side_surface, fillet_surface, fillet_edge.curve());
let fillet_edge = fillet_edge.with_curve(new_curve.to_same_geometry());
```

moved ABOVE the `new_boundaries` map (≈391-411) so the boundary insertions
(400-406) use the replaced edge. Same final topology and geometry; one more
allocation.

**`truck-meshalgo/tests/tessellation/triangulation.rs`** (line 158) —
`edge.set_curve(bsp.into())` in a test: the test builds the edge then mutates
it; migrate to building with the final curve or `with_curve` rebinding,
whichever reads naturally in context.

### 8. Tests

- In `vertex.rs`'s test module: `vertex_replacement_changes_id_not_old_handles`
  — `v0 = Vertex::new(0); let h = v0.clone(); let v2 = Vertex::new(1);
  assert_ne!(v0.id(), v2.id()); assert_eq!(v0.point(), 0); assert_eq!(h.point(), 0);`
- In `edge.rs`: `with_curve_preserves_topology` — an edge over two vertices
  (use `()` geometry); `e2 = e.with_curve(())`: vertices compare equal by id,
  orientation/pcurve equal, ids differ; the original handle's curve accessor
  is unchanged.
- In `face.rs`: `with_surface_preserves_boundaries` — the face analogue
  (boundaries are the same wire handles, ids differ).
- In `vertex.rs` (or `wire.rs`, whichever reads better): `mapped_closure_may_access_geometry`
  — the regression the old remarks forbade: `v0.mapped(|p| { let _ = v0.point(); *p })`
  (and an edge-level variant mapping while reading `edge.curve()`) — runs to
  completion, no deadlock; before the migration this was the documented hazard.
- In `entity_id.rs`: `replaced_id_derives_stably` — `a.replaced(&p1) ==
  a.replaced(&p1)` for equal params, `!=` for distinct params from the same
  id, `!=` the same params from a distinct id; serde round-trip of an id
  containing the `Replace` arm; the result is `EntityId::Op { .. }` with
  `kind == OpKind::Replace` and one input.
- NEW FILE `truck-topology/tests/parallel_query.rs`, opening with
  `#![deny(clippy::unwrap_used)]` (GATE-1 gates new test files on it):
  `parallel_query_never_deadlocks` — build a shell with real `Point3`-
  geometry-free topology (the lib.rs doc tetrahedron with `()` geometry is
  fine, or `Vertex::news` + `Edge::new` loops), then from **8 rayon threads**
  (`(0..8).into_par_iter()`, `use rayon::prelude::*;` — rayon is already a
  dependency) concurrently: clone vertices, call `.point()`/`.id()`/`.count()`,
  map a wire (`mapped(|p: &()| *p)`), cut an edge, and format an edge with
  `Debug`. Each thread asserts its results; the test completing IS the
  regression (before the migration the mapped-with-geometry-access case could
  deadlock; with `Arc<P>` there is nothing to lock). Keep the per-thread work
  small — this must run in well under a second.

Doctests replaced per decisions 0-3 (VertexID's id-stability reversal,
set_point/set_curve/set_surface → constructor/with_curve/with_surface, the
un-hidden mapped family's new examples). All doctest floats are integer-valued
— no H-3 exposure is expected; if you ever need a comparison epsilon, use a
named const with a same-line `// H-3:` comment (see below).

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N` literal (the regex catches `1e-6`, `1.0e-6`, `1.0e-06`, ...) unless
that same line ends with an `// H-3` comment. It is a text gate on the diff:
it does not know your literal is a tolerance, and it does not care that the
line is in a test. This packet's code is integer-and-unit geometry (`()`,
`0`, `1`), so the rule should never fire; if it does, you have introduced a
bare epsilon — make it a named const whose defining line carries a same-line
`// H-3:` comment naming the dimensionless quantity. Run
`bash scripts/kernel-gates.sh` yourself before you write `RESULT.json`; it is
the same script V4 runs.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-topology -p truck-shapeops -p truck-meshalgo
cargo clippy -p truck-topology -p truck-shapeops -p truck-meshalgo --all-targets --no-deps
cargo test -p truck-topology -p truck-shapeops -p truck-meshalgo --lib --tests --no-fail-fast
cargo test -p truck-topology --doc
cargo check --workspace --all-targets
bash scripts/kernel-gates.sh <your base commit>
```

Never run a bare `cargo test` — it builds 56 examples. Send cargo output to a
file and read the tail.

**The three crates are clean at baseline** — measured at the tree this packet
was written against (HEAD ddcd706, verified green at the session-16 close):
all lib+integration tests pass across the workspace, and clippy is clean on
the three crates (truck-topology denies `clippy::all` at its root). Your bar:
everything above stays green plus your six new tests and the migrated
doctests. Any baseline failure you did not cause is a stop condition; any
failure you did cause is yours to fix. The existing `truck-shapeops/tests/
fillet.rs` and meshalgo tessellation suites are the guards on decision 7's
call-site migrations — they must pass unchanged.

## Forbidden

Editing any file outside `write_allow` — in particular
`truck-modeling/**` (all call sites signature-compatible, verified),
`truck-stepio/**`, `truck-meshalgo/src/**` (the `src` triangulation is
untouched; only its `tests/` sibling is yours), `truck-shapeops/src/fillet/
experiment.rs` (dead code — never edit), `truck-shapeops/src/transversal/
integrate/mod.rs`, `truck-topology/src/{imported,compress}.rs`, both crates'
`Cargo.toml` files (rayon and parking_lot stay as declared), and every other
crate. Keeping any `set_point`/`set_curve`/`set_surface` method (the removal
IS the contract). Keeping `#[doc(hidden)]` or any deadlock remark on the
mapped family. Adding `#[ignore]`. Adding `unwrap()`/`expect()`/`panic!` on
fallible paths in production code (test modules' allow blocks are the house
exception; `RemoveTry` in lib.rs is pre-existing — leave it). Committing to
`main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- `mapped`/`try_mapped` signatures turn out NOT to be source-compatible with
  some call site outside your write set (the packet's "what does NOT change"
  list is wrong) → `SPEC_GAP`, naming the file and the break
- `rclite::Arc` lacks `as_ptr`/`strong_count` under the default feature (the
  packet assumes both; the code compiles today using them on
  `Arc<Mutex<T>>`, and the methods are pointee-independent, but verify) →
  `SPEC_GAP`
- the loops_store reassignment (decision 7) cannot express the original
  information flow (gv0's point from store 0 feeding store 1 and the leader
  readback) → `SPEC_GAP`, describing what the mutation semantics carried
  that construction cannot
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(topology): Arc<Mutex<G>> to Arc<G>, replacement API (BG-CE-003-MIGRATE)`.
