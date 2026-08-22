# WORK PACKET BG-CE-003-MIGRATE-r2 — canonical endpoint vertices restore cross-store identity

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-CE-003-MIGRATE-r2","status":"DONE","contracts":["BG-CE-003"],
 "tests_added":1,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: the design below was derived
by reading `loops_store/mod.rs` and the attempt-1 diff line by line, but it is
exactly the kind of claim that can be confidently wrong. **If anything below
contradicts what you find in the code, say so in `disagreements` rather than
making the code match the packet.**

```yaml
id:          BG-CE-003-MIGRATE-r2
contract:    [BG-CE-003]
class:       design
crates:      [truck-topology, truck-shapeops, truck-meshalgo]
depends_on:  [BG-CE-003]
write_allow:
  - vendor/truck/truck-shapeops/src/transversal/loops_store/mod.rs
  - vendor/truck/truck-shapeops/src/transversal/loops_store/tests.rs
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
  - vendor/truck/truck-shapeops/src/fillet/mod.rs
  - vendor/truck/truck-meshalgo/tests/tessellation/triangulation.rs
read_allow:
  - vendor/truck/truck-shapeops/src/transversal/integrate/mod.rs
  - vendor/truck/truck-shapeops/src/transversal/integrate/tests.rs
  - QUESTION.md
tests_required:
  - intersection_edges_share_identity_across_stores
budget:      {turns: 36, ctx_tokens: 90000}
anchors:
  # Your tree starts at branch packet/BG-CE-003-MIGRATE commit c5cb4c6 —
  # attempt 1's storage migration carrying its FAILED call-site reconstruction.
  # These counts are pinned to that exact commit (`git show`), because the
  # integration branch does not carry attempt 1. A count mismatch is a stop
  # condition (ANCHOR_MISMATCH), not a nuisance.
  - {id: A1, expect: 4, cmd: "git show c5cb4c6:vendor/truck/truck-shapeops/src/transversal/loops_store/mod.rs | grep -c 'add_geom_vertex('"}
  - {id: A2, expect: 0, cmd: "git show c5cb4c6:vendor/truck/truck-shapeops/src/transversal/loops_store/mod.rs | grep -c 'set_point'"}
  - {id: A3, expect: 7, cmd: "git show c5cb4c6:vendor/truck/truck-shapeops/src/transversal/loops_store/mod.rs | grep -cE 'gemap0b|gemap1b|gemap_store0'"}
  - {id: A4, expect: 2, cmd: "git show c5cb4c6:vendor/truck/truck-shapeops/src/transversal/loops_store/mod.rs | grep -c 'fn change_vertex'"}
  - {id: A5, expect: 1, cmd: "git show c5cb4c6:vendor/truck/truck-shapeops/src/transversal/loops_store/mod.rs | grep -c 'fn search_parameter'"}
  - {id: A6, expect: 3, cmd: "git show c5cb4c6:vendor/truck/truck-shapeops/src/transversal/loops_store/mod.rs | grep -c 'curve_surface_projection'"}
  - {id: A7, expect: 9, cmd: "git show c5cb4c6:vendor/truck/truck-shapeops/src/transversal/loops_store/mod.rs | grep -c 'Vertex::new'"}
  - {id: A8, expect: 7, cmd: "git show c5cb4c6:vendor/truck/truck-shapeops/src/transversal/loops_store/mod.rs | grep -c 'swap_edge_into_wire'"}
  - {id: A9, expect: 1, cmd: "git show c5cb4c6:vendor/truck/truck-shapeops/src/transversal/integrate/tests.rs | grep -c 'punched_cube'"}
  - {id: A10, expect: 1, cmd: "git show c5cb4c6:vendor/truck/truck-topology/tests/parallel_query.rs | grep -c 'parallel_query_never_deadlocks'"}
  - {id: A11, expect: 1, cmd: "git show c5cb4c6:vendor/truck/truck-shapeops/src/fillet/mod.rs | grep -c 'with_curve'"}
  - {id: A12, expect: 0, cmd: "git show c5cb4c6:vendor/truck/truck-topology/src/lib.rs | grep -c 'Arc<Mutex'"}
  - {id: A13, expect: 1, cmd: "git show c5cb4c6:vendor/truck/truck-topology/src/entity_id.rs | grep -c 'replaced_id_derives_stably'"}
  - {id: A14, expect: 3, cmd: "git show c5cb4c6:vendor/truck/truck-shapeops/src/transversal/loops_store/tests.rs | grep -c '#\\[test\\]'"}
```

(`grep -c` exits 1 on zero matches — a count of 0 IS the expected answer where
the packet says so, not a command failure.)

## Problem

Attempt 1 (your own commit `c5cb4c6`, already on this branch) migrated
`truck-topology` from `Arc<Mutex<G>>` to immutable `Arc<G>` and added the
replacement API. All of that is green and stays. What failed was the
`loops_store` call-site reconstruction in
`truck-shapeops/src/transversal/loops_store/mod.rs`:
`transversal::integrate::tests::punched_cube` fails with *"This shell is not
oriented and closed."*

Your own QUESTION.md (in your worktree root, on read_allow) derives why. The
old mutation semantics registered ONE shared mutable `Vertex` instance per
intersection-curve endpoint (`gv0`/`gv1`) in BOTH geom stores, and its
load-bearing properties were:

1. **One instance, final point everywhere.** Each store's `set_point`
   re-pointed the SAME instance; the store-1 call wrote last, so both stores'
   boundaries ended up reading the store-1 effective point through one handle.
2. **Cross-store vertex identity through later replacements.** When a later
   intersection pair's `change_vertex` re-replaced an endpoint shared by two
   curves, both stores received edges referencing the same new instance,
   because both calls passed the same `&Vertex`.
3. **Cross-store EDGE identity through replacements.** `pemap0`/`pemap1`/
   `gemap0`/`gemap1` were each SHARED between the two stores' corresponding
   calls, so when both stores replaced an edge of the same `EdgeID` (the
   closing edge inserted by an earlier pair lives in BOTH stores as clones),
   `or_insert_with` handed both stores ONE replacement instance.

Attempt 1 reconstructed (1) per store with post-hoc re-pointing, but split the
maps into five per-store caches (`gemap0`, `gemap0b`, `gemap1`, `gemap1b`,
`gemap_store0`) — destroying property 3 — and its reconciliation pass allocated
fresh replacement edges outside any shared map, so the two stores' copies of a
shared intersection edge diverge as soon as a later pair re-replaces them. The
loops look identical point-for-point; the shell does not close.

## Decisions already made for you

**Everything outside `loops_store/{mod,tests}.rs` on this branch is landed
attempt-1 work: do not touch it.** The topology files and fillet/meshalgo are
in `write_allow` only so V1 tolerates their existing diff; editing them needs a
compile error you cannot otherwise fix, reported in `notes`.

### Decision 1 — restore the baseline map topology

Delete the five-map scheme. Per intersection curve (inside the non-closed
`else` arm of `create_loops_stores`) there are exactly FOUR maps again, with
baseline's sharing:

- `pemap0`: used by poly store-0 AND poly store-1 commits for endpoint pv0.
- `pemap1`: likewise for pv1.
- `gemap0`: used by geom store-0 AND geom store-1 registrations for gv0.
- `gemap1`: likewise for gv1.

Each map is created once per intersection curve, before the endpoint's first
use, and passed by `&mut` to every commit that baseline would have passed it
to. This restores property 3 mechanically.

### Decision 2 — pure discovery, extracted

Split `add_polygon_vertex` (≈329) into its halves:

```rust
// pure: what add_polygon_vertex's search half computes today
fn search_polygon_vertex(&self, pt: P) -> Option<(usize, usize, ParameterKind)>
```

(The body is today's `search_parameter` call plus nothing else;
`search_parameter` itself is unchanged.) Keep a private commit half or inline
it at the call sites — whichever reads better; no public API moves.

Add the geom-side discovery helper (pure, mutates nothing):

```rust
// LoopsStore<Point3, C>: everything add_geom_vertex's Front/Back/Inner arms
// need to know BEFORE registering anything.
struct GeomEndpointDiscovery {
    old_vertex: Option<Vertex<Point3>>, // F/B arm: the boundary vertex to replace
    cut: Option<(Point3, f64)>,         // Inner arm: (projected point, parameter)
    effective_point: Point3,            // what set_point would have written
}

fn discover_geom_endpoint(
    &self,
    face_index: usize,
    wire_index: usize,
    edge_index: usize,
    kind: ParameterKind,
    another_surface: &impl ParametricSurface3D + SearchNearestParameter<D2, Point = Point3>,
    query_point: Point3, // v.point() at the corresponding moment in the baseline flow
) -> Option<GeomEndpointDiscovery>
```

- Front/Back: `old_vertex = self[face][wire][edge].absolute_front()/absolute_back().clone()`;
  `effective_point = old_vertex.point()`; `cut = None`. (Indices come from the
  POLY search, exactly as baseline's `add_geom_vertex` receives them.)
- Inner: `(pt, t, _) = curve_surface_projection(&curve, None, another_surface,
  None, query_point, 100)?`; `cut = Some((pt, t))`; `effective_point = pt`.
- Returning `None` propagates exactly like baseline's `?`.

The `query_point` chain is load-bearing and must match baseline exactly:
for the front endpoint, store-0's discovery queries `polyline.front()`; store-1's
discovery queries **store-0's effective point** (what store-0's arm would have
set), because baseline's second `add_geom_vertex` saw the mutated content.
For the back endpoint, substitute `polyline.back()`.

### Decision 3 — one canonical vertex per endpoint, born before registration

For each endpoint X ∈ {front → gv0, back → gv1} of the intersection curve:

1. Discover on store 0 (`d0`) and on store 1 (`d1`), purely, in the baseline
   order (see Decision 5 for exact interleaving).
2. Compute the single canonical point:

   ```text
   p_canon(X) =
       d1 discovered ? d1.effective_point      // store-1 set_point wrote last
     : d0 discovered ? d0.effective_point      // store-0's write stands
     : polyline endpoint                        // nobody touched gvX
   ```

3. Construct **ONE** `let gvx = Vertex::new(p_canon);` and register THAT
   instance in BOTH stores:

   - Front/Back arm: `change_vertex(&old_vertex_i, &gvx, &mut gemapX)` per
     store i — same handle both times, which restores property 1 without any
     mutation.
   - Inner arm: `cut_with_parameter` refuses unless
     `curve.subs(t).near(&v.point())` (edge.rs ≈339), so cut with a LOCAL
     vertex carrying the locally-projected point, then unify:

     ```rust
     let vl = Vertex::new(pt_i);                       // pt_i = d_i.cut.point
     let (edge0, edge1) = edge.cut_with_parameter(&vl, t_i)?;
     let new_wire: Wire<_, _> = vec![edge0, edge1].into();
     self.swap_edge_into_wire(edge_id, &new_wire);
     self.change_vertex(&vl, &gvx, emap);              // unify with the canon
     ```

     (The sweep touches only the fresh halves — `vl` exists nowhere else —
     and goes through `gemapX`, so it cannot disturb cross-store identity.)
4. Leader readback: `*polyline.first_mut().unwrap() = gv0.point()` iff `d0` or
   `d1` discovered something (baseline updated it inside each `if let Some`),
   and the value written is `p_canon` — identical to baseline's final state.
5. `gedge = Edge::new(&gv0, &gv1, intersection_curve.into())` then
   `add_edge` into both stores, unchanged — but now built on instances that
   are already present in both stores' boundaries.

Property 2 is restored by induction: pair N registers one instance per
endpoint in both stores; pair N+1 whose endpoint coincides discovers F/B at
that very instance in each store's own boundary and replaces it with ITS
canonical instance via the same two calls — so both stores converge on pair
N+1's instance, exactly as the mutation semantics did.

### Decision 4 — delete attempt-1 residue

All seven lines matching anchor A3 (`gemap0b`, `gemap1b`, `gemap_store0`,
`gv0_store0`/`gv1_store0` clones and the two post-hoc
`geom_loops_store0.change_vertex(...)` reconciliation calls) go away. After r2
there is NO registration that is ever re-pointed: every store sees each
endpoint exactly once, at its final point. `add_geom_vertex` as a separate
method may be absorbed into `create_loops_stores` or kept as a thin
registration fn taking the pre-built canonical vertex — your call, say which
in `notes`. `QUESTION.md` is DELETED from the worktree (the question it asks
is answered by this packet).

### Decision 5 — exact operation order (do not reorder anything else)

Baseline order, which you preserve step-for-step, substituting D=iscover /
C=ommit:

```text
d0(front) -> polyC0(front) -> d0(back)? NO — baseline interleaves:
```

Concretely, per intersection curve:

1. `d00 = poly_loops_store0.search_polygon_vertex(polyline.front())`
2. commit polyC0(front): F/B `change_vertex(old0p→pv0)` / Inner cut+swap
   inserting `pv0`, map `pemap0` (as today).
3. `d10 = poly_loops_store1.search_polygon_vertex(polyline.front())`
4. commit polyC1(front): map `pemap0` (shared — baseline shared it too).
5. discover geomD0(front): indices/kind FROM `d00`; query `polyline.front()`;
   surface argument `surface1` (baseline's idx00 call). Reads `geom_store0`,
   which has undergone exactly the edits baseline had made by this point.
6. construct `gv0` at `p_canon(front)`; register geomC0(front) then
   geomC1(front): geomD1(front) happens BEFORE any geom commit (indices/kind
   from `d10`, surface argument `surface0`, query = store-0's effective
   point); both registrations use `&gv0` and shared `gemap0`. If either
   discovery returned None, skip that store's registration only, exactly like
   baseline's `if let Some`.
7. Repeat 1–6 for the back endpoint with pv1/gv1/pemap1/gemap1 and
   `polyline.back()`.

The invariant that makes this faithful: every discovery reads its store in
exactly the state baseline read it (pure reads inserted immediately before
their own commits; poly commits keep baseline's relative order; geom commits
keep baseline's relative order; nothing reads a store across a foreign
commit). If you find this ordering claim false anywhere, STOP — that is a
`disagreements` finding, not something to paper over.

### Decision 6 — the new test

In `loops_store/tests.rs` (which has `#![deny(clippy::unwrap_used)]`-style
test modules — check the file's existing test module attributes and match
them):

```rust
fn intersection_edges_share_identity_across_stores()
```

Build two shells whose intersection produces at least one non-closed
intersection curve (reuse the existing fixtures — `parabola_surfaces` and
however the current tests assemble `Shell`s; the punched_cube fixture in
`integrate/tests.rs` shows the assembly pattern). Run
`create_loops_stores(...)`, then assert that for at least one edge present in
both `geom_loops_store0` and `geom_loops_store1`, the TWO INSTANCES ARE THE
SAME (`EdgeID<C>` equality via `.id()`, or `std::ptr::eq` on
`Edge::shared_curve()` backing Arcs — id equality is enough and is what
`add_edge` matching uses). Before r2 this fails on attempt-1's code (per-store
maps diverge the copies); after r2 it must hold. Also assert the analogous
vertex identity: some endpoint `Vertex` appears in both stores with equal ids.
If the fixture work balloons past ~80 lines, shrink the assertion to whatever
two-shell fixture you can reuse wholesale and say so in `notes`.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N` literal unless that same line ends with an `// H-3` comment. This
packet should introduce none; if you need a comparison epsilon, make it a
named const whose defining line carries a same-line `// H-3:` comment naming
the dimensionless quantity. Run `bash scripts/kernel-gates.sh <your base>`
before writing RESULT.json — it is the same script V4 runs.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-shapeops
cargo clippy -p truck-shapeops --all-targets --no-deps
cargo test -p truck-shapeops --lib --no-fail-fast
cargo check --workspace --all-targets
bash scripts/kernel-gates.sh <base>        # base = ddcd706 (merge-base)
```

plus, explicitly:

```
cargo test -p truck-shapeops --lib transversal::integrate::tests::punched_cube
cargo test -p truck-shapeops --lib transversal::loops_store
```

**`punched_cube` passing is THE acceptance criterion of this packet.** It is a
pre-existing test, so V5 will not fail on it — do not let that fool you into
reporting DONE while it is red; the orchestrator runs it by hand at landing.
The three existing loops_store tests must pass UNCHANGED (assertion-strength
may grow; it may never shrink). Send cargo output to a file and read the tail.
Never run a bare `cargo test`.

## Forbidden

Editing any file outside the spirit of this packet even where `write_allow`
technically allows it — topology, fillet, meshalgo are landed attempt-1 work.
Weakening any loops_store assertion or deleting a loops_store test. Adding
`#[ignore]`. Reintroducing any `set_point`/`Mutex` under `vendor/truck/`.
Adding `unwrap()`/`expect()`/`panic!` on fallible paths in production code
(test modules' allow blocks excepted). Committing to `main`. Rewriting git
history on the packet branch — commit ON TOP of `c5cb4c6`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the anchor and what you saw
- Decision 5's ordering claim proves false (some discovery provably reads a
  state baseline never read) → `SPEC_GAP` with the concrete counterexample
- `punched_cube` still fails after the design is implemented faithfully →
  `BLOCKED` with the failure output attached; do NOT iterate blind fixes more
  than twice
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

REPLACE the existing tracked `RESULT.json` entirely (it describes attempt 1);
DELETE `QUESTION.md`. `status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`,
`BLOCKED`. Commit on the current branch (`packet/BG-CE-003-MIGRATE`) with
subject

```
fix(shapeops): canonical endpoint vertices restore cross-store identity (BG-CE-003-MIGRATE-r2)
```

and include RESULT.json in the commit as last time.
