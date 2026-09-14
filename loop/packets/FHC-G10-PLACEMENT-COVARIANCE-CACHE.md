# WORK PACKET FHC-G10 — placement-invariant flux cache: identical parts at different poses share certified work

G6's evidence: certification cost is per-patch and repeated across the fuse
chain (34 `bd_facts` calls on one suspension row with heavily overlapping
subtrees). The landed memos (`facts_memo`/`patch_memo`/`product_memo`,
`bd_bridge.rs:4307-4340`) key on **placed** content hashes — a part moved one
micron re-certifies everything. The proposal's §11 (Theorem 11.1) makes the
cache **placement-invariant**: cache the two intrinsic invariants
`Φ₀[X] = (1/3)∫X·(X_u×X_v)` and the area vector `A⃗[X]`; the placed flux is
then `O(1)` arithmetic per patch under any similarity placement:

    Φ₀[g∘X] = s³·Φ₀[X] + (s²/3)·Rᵀ(t−a)·A⃗[X]        (det R = −1 ⇒ negate)

**Normative theory:** Certified Flux Calculus §11 (Theorem 11.1, Corollary
11.2) and §17.2-17.3, via `docs/CERTIFIED_FLUX_CALCULUS_INTEGRATION_MAP.md`.

```yaml
id:          FHC-G10-PLACEMENT-COVARIANCE-CACHE
contract:    [FHC-G10-PLACEMENT-COVARIANCE-CACHE]
class:       mechanical
crates:      [truck123d]
depends_on:  []
needs:       []
write_allow:
  - truck123d/src/bd_bridge.rs
  - truck123d/tests/placement_cache.rs
read_allow:
  - docs/F1_HYPERCAR_GAP_REGISTER.md
  - docs/CERTIFIED_FLUX_CALCULUS_INTEGRATION_MAP.md
tests_required: [truck123d/tests/placement_cache.rs]
anchors:
  - {id: A1, expect: 3, cmd: "grep -c 'facts_memo' truck123d/src/bd_bridge.rs"}
  - {id: A2, expect: 3, cmd: "grep -c 'patch_memo' truck123d/src/bd_bridge.rs"}
  - {id: A3, expect: 11, cmd: "grep -c 'spline_loop_area_vector' truck123d/src/bd_bridge.rs"}
budget:      {turns: 45, ctx_tokens: 180000}
```

Anchors measured 2026-09-13 at HEAD. **Re-measure at dispatch.**

## Pre-made judgements

1. **Extend the landed memo pattern, not a new one.** A fourth memo
   (`invariant_memo`) keyed by the intrinsic (unplaced) control net —
   identical parts at different rigid/similarity placements hit the same
   entry (§17.3: do NOT key by placed coefficients).
2. **The transform is exact, so outputs are byte-identical** by
   construction: the cached-invariant route and direct recomputation produce
   the same rational number. The determinism law is untouched (fixed
   reduction order inside the cached computation).
3. **Scope: exact polynomial patches** (the current admitted set).
   Rational-patch covariance follows the same law but waits for G1's
   rational arm; trimmed faces' covariance is §11's closing note — both out
   of scope here.
4. **No heuristic invalidation.** Pure function of the intrinsic net; entries
   live for the process lifetime exactly like the existing memos.

## Tests required (`truck123d/tests/placement_cache.rs`)

1. `covariance_law_exact` — random patches × random rigid placements:
   cached-invariant route equals direct recomputation bit-for-bit (doc T11).
2. `scale_and_mirror` — uniform scaling (s³ law) and an orientation-reversing
   placement (det R = −1 sign flip) verified exactly.
3. `cache_hits_across_poses` — two fuse chains over the same part at
   different placements: second chain records cache hits (instrument via the
   landed phase timers), identical final facts records.
4. `no_false_hits` — geometrically near-identical but control-distinct nets
   MISS the cache (key correctness), outputs still correct.

## Done when

```
cargo fmt --check -p truck123d
cargo clippy -p truck123d --all-targets -- -D warnings
cargo test -p truck123d --test placement_cache --locked
```

plus a bounded door spot-check on a row with repeated placed geometry
(`compound_from_instances` class) recording the hit rate in RESULT. All cargo
through the queue.

## Forbidden

Any file outside `write_allow`; editing `corpus/ttc/**` or `vendor/truck/**`;
floating-point shortcuts in the transform (exact arithmetic only);
`#[ignore]`, deleted or weakened tests, bare `cargo test`; committing to
`main`.

## Stop conditions

- the covariance law cannot be made exact over the landed representation →
  `SPEC_GAP` naming the gap
- a false cache hit is ever observed → that is a BUG: fix the key, never
  widen acceptance
- anchor count differs at dispatch and was not re-measured → `ANCHOR_MISMATCH`
- three consecutive failed cargo runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

```json
{"id":"FHC-G10-PLACEMENT-COVARIANCE-CACHE","status":"DONE","contracts":["FHC-G10-PLACEMENT-COVARIANCE-CACHE"],
 "anchors_verified":{"A1":3,"A2":3,"A3":11},
 "rows_flipped":[],"rows_still_refused":[],
 "notes":"suite 1-4 verdicts; hit-rate record; row spot-check"}
```

Commit subject: `truck123d: placement-invariant flux cache - intrinsic invariants + similarity law (FHC-G10)`.
