# WORK PACKET BD-EMIT-MESH-CACHE — memoized tessellation for the GLB/STL emit path

Owner-directed 2026-09-11: the full-vehicle attribution probes
(`scratch/fh_render/glb_mesh_attribution*.json`) proved the 2,142-part
Falcon Heavy is a handful of UNIQUE parts stamped repeatedly — 27 byte-identical
engines (236,378 tris each), port/starboard booster groups identical
(2,177,646 tris each). The emit path today tessellates EVERY copy
(mesh phase 1,012 of the 1,033 ms GLB emit; the door meshes the tree a
second time for STL). The BRep/tree is the authority and builds in 9 ms;
the emit artifacts are derived — so the tessellation may be computed once
per unique part and reused. The owner ruled instancing acceptable at the
tessellation/GLB side ("we still have the BRep generated instantly, so the
instancing is done tessellation-side for speed").

```yaml
id:          BD-EMIT-MESH-CACHE
contract:    [BD-EMIT-MESH-CACHE]
class:       mechanical+
crates:      [truck123d]
depends_on:  [FHC-MIRROR-FORM]
needs:       [FHC-MIRROR-FORM]
write_allow:
  - truck123d/src/bd_bridge.rs
  - truck123d/tests/mesh_cache.rs
read_allow:
  - truck123d/tests/pb_glb_emit.rs
  - corpus/ttc/door.py
  - docs/TT_TIMING_RESULTS.md
tests_required: [truck123d/tests/mesh_cache.rs]
anchors:
  - {id: A1, expect: 1,  cmd: "grep -c 'fn bd_glb' truck123d/src/bd_bridge.rs"}
  - {id: A2, expect: 6,  cmd: "grep -c 'mesh_ms' truck123d/src/bd_bridge.rs"}
  - {id: A3, expect: 18, cmd: "grep -c 'MESH_SEGMENTS' truck123d/src/bd_bridge.rs"}
budget:      {turns: 60, ctx_tokens: 180000}
```

Anchors measured 2026-09-11 at the FHC-MIRROR-FORM landing base. Re-measure
at dispatch (the FHC chain lands before this packet and WILL drift counts);
update the packet at dispatch, never dispatch stale.

## Pre-made judgements

1. **Cache key = content hash of the canonical solid spec + deflection +
   color.** The kernel's carriers are deterministic canonical specs
   (`SolidSpec`/`ProfileEdge`), so the hash is EXACT — no fuzzy geometric
   matching. Hash the spec payload, not coordinates-as-identity, and never
   let hash-map iteration order reach an output (house determinism law):
   outputs are assembled in fixed tree order regardless of cache state.
2. **GLB: shared accessors + per-node transforms.** One node per part is
   PRESERVED (labels, hierarchy, per-node colors). Primitives of
   parts with identical (spec-hash, deflection, color) reference the same
   accessor/buffer; distinct colors or distinct specs get distinct meshes.
   The `pb_glb_emit.rs` contracts (names/hierarchy/colors round-trip,
   binary structure) must stay green unchanged.
3. **STL: full soup, expanded from the cached meshes.** The STL artifact's
   structure does not change (triangle soup); it just stops re-tessellating.
   **The fingerprint path is untouchable: `bd_stl` with `None` deflection
   (the `MESH_SEGMENTS` fingerprint rule) must stay BIT-IDENTICAL** —
   vehicle fingerprint 4,461,816 must re-verify. The memoization may serve
   the None-path mesh from cache only if the expanded bytes are identical;
   if in doubt, bypass the cache on the None path (simplest, and fast
   enough — the census uses it once per row).
4. **Determinism:** identical ordered input + deflection → byte-identical
   GLB and STL, repeated runs, cache populated or not (a cold-cache run and
   a warm-cache run must produce identical outputs; only `mesh_ms` may
   differ).
5. out of scope: chordal-deviation recalibration of segment counts (separate
   decision), SIMD/trig tables, rayon across solids. This packet is the
   memoization + shared-accessor change only.

## Method

1. Add the content-hash cache to the emit path (key: spec hash + deflection
   + color; value: meshed payload). Mesh on miss, reuse on hit.
2. GLB writer: emit one accessor/buffer per unique key; nodes reference it
   with their transforms. Keep one node per part with its label and color.
3. STL writer: expand cached meshes into the soup per part (transformed).
   None-deflection fingerprint path bypasses the cache (judgement 3).
4. Tests (`truck123d/tests/mesh_cache.rs`):
   - identical parts share one accessor; distinct colors do not;
   - node/hierarchy/label/color round-trip unchanged (mirror the
     `pb_glb_emit` assertions over a two-copy scene);
   - repeated emit byte-identical (cold vs warm cache);
   - `bd_stl(None)` vehicle fingerprint unchanged;
   - full-vehicle GLB accessor count < part count (the cache actually fires
     on the Falcon Heavy tree).
5. Record before/after emit timings and artifact sizes for the full vehicle
   in RESULT notes (diagnostic, not thresholds).

## Done when

```
cargo fmt --check -p truck123d
cargo clippy -p truck123d --all-targets -- -D warnings
cargo test -p truck123d --test mesh_cache --locked
cargo test -p truck123d --test pb_glb_emit --locked
```

plus a full-vehicle door run emitting both artifacts with `--glb` at the
shipped 0.4 mm and the vehicle fingerprint re-verified. All cargo through
the queue (the `cargo` on PATH IS the queue shim); interpreter dir on PATH
if STATUS_DLL_NOT_FOUND.

## Forbidden

Any file outside `write_allow`. Changing the `MESH_SEGMENTS` fingerprint
rule or any landed refusal/entry signature. Hash-map-order-dependent
output. `#[ignore]`, deleted or weakened tests, bare `cargo test`.
Committing to `main`.

## Stop conditions

- the fingerprint (None-deflection STL) cannot be kept bit-identical with the cache active → bypass there and note it; if even that fails, `SPEC_GAP`
- GLB consumers in `pb_glb_emit` break on shared accessors in a way the packet's judgements do not cover → `SPEC_GAP`
- anchor count differs at dispatch and was not re-measured → `ANCHOR_MISMATCH`
- three consecutive failed cargo runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

```json
{"id":"BD-EMIT-MESH-CACHE","status":"DONE","contracts":["BD-EMIT-MESH-CACHE"],
 "anchors_verified":{"A1":1,"A2":6,"A3":18},
 "notes":"before/after emit ms + artifact MB; fingerprint re-verified; accessor-sharing count"}
```

Commit on the current branch, subject
`truck123d: memoized tessellation + shared GLB accessors (BD-EMIT-MESH-CACHE)`.
