# QUESTION — BG-CE-003-MIGRATE (SPEC_GAP)

## Status: SPEC_GAP

The storage migration `Arc<Mutex<G>>` → `Arc<G>` and the replacement API are
complete and green, but the **loops_store call-site migration cannot express the
original information flow**, per the packet's stop condition.

## What breaks

`truck-shapeops`'s boolean pipeline test `transversal::integrate::tests::punched_cube`
fails with `This shell is not oriented and closed.` after the migration.

## What the mutation semantics carried that construction cannot

`add_geom_vertex` (loops_store) registered **one shared `Vertex` instance in
both stores** (`gv0`/`gv1` passed by reference to both surfaces' stores). Its
point was mutated by each store's `set_point` call, so:

1. The **final point** (fixed by the store-1 calls) sat on the store-0 boundary
   vertices too, keeping every cut/gedge edge `is_geometric_consistent`.
2. The **single closing edge** connected that shared instance, so `add_edge`
   matched both stores' boundaries and split the loops identically.
3. The same shared instance kept **cross-store edge identity**: when a later
   intersection pair re-replaced the shared edge (via `change_vertex`), both
   stores' loops referenced the same replacement, so faces produced from store-0
   and store-1 loops shared the intersection edge and the boolean shell closed.

With immutable `Arc<G>`:

- The packet's literal reassignment (single `gv0`/`gv1`, one closing edge) fails
  the loops_store tests: the closing edge connects store-1's effective vertices,
  which are not on store-0's boundary, so store-0's loops do not split.
- The closest faithful reconstruction (construct effective vertices per store,
  then re-point store-0's boundary at store-1's effective vertices, with
  per-store replacement caches) makes the three loops_store tests pass and makes
  the loops byte-identical to baseline, but the boolean pipeline still fails:
  the two stores' copies of the shared intersection edge diverge once a later
  pair's `change_vertex` re-replaces them, so the final shell is not closed.

The shared mutable vertex is load-bearing for the cut parameter checks, the
geometric-consistency tolerance, the `add_edge` vertex-identity matching, and the
cross-store edge identity of the boolean operation. None of these can be
reproduced by construction with `Arc<P>` storage.

## Evidence

- `cargo test -p truck-shapeops --lib transversal::loops_store` — passes (3/3).
- `cargo test -p truck-shapeops --lib transversal::integrate::tests::punched_cube`
  — fails (`This shell is not oriented and closed.`). Passes at baseline.
- Structure of the produced loops-store loops is point-for-point and
  structure-for-structure identical to baseline; only the cross-store edge-id
  sharing of the intersection edges differs.

## Also reported

- `entity_id.rs`: the packet claims the pinned KAT constants "do not move";
  adding `OpKind::Replace` alphabetically shifts discriminants, so the two
  constants in `stable_hasher_known_answer` were updated (see RESULT.json).
- The tree is not "clean at baseline" as the packet claims: `vendor/truck/resources/`
  is absent (drives several test failures), `fillet.rs::complex_surface` and
  `healing::tests::step_import` fail, and `truck-meshalgo/src` carries 153
  pre-existing clippy errors. All are listed in RESULT.json `baseline_failures`.

## Ask

How should the loops_store call site be handled? Options:

1. Accept the boolean-pipeline regression as a tracked SPEC_GAP (this packet
   reports it) and let a follow-up packet design a faithful replacement
   (e.g. a two-phase "compute effective points, then register one shared
   vertex in both stores" API on `LoopsStore`).
2. Amend the packet to scope decision 7 to the loops_store tests only, and
   declare the boolean-pipeline failure out of scope for BG-CE-003-MIGRATE.
3. Provide a different migration design for `add_geom_vertex` that preserves
   the shared-instance semantics under `Arc<G>`.
