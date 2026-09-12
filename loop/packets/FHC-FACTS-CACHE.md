# WORK PACKET FHC-FACTS-CACHE — stop re-running the certified bracket on unchanged subtrees

Owner directive 2026-09-12, evidence measured (`scratch/facts_calls.log`):
the door's authoring layer calls `bd_facts` at every composition site
(door.py:1453 boolean, :2604 trim, :2728 fillet) and each call re-executes
the ENTIRE certified evaluation of the composed subtree from scratch:

- `beam_wing` (2 booleans): 4 authoring probes ≈ 16.3 s of a 17.4 s build
  (94% hidden bracket work) + 20.2 s final facts — ~45% of row wall is
  redundancy.
- `power_unit`: 88+ calls in 4 minutes, each on a GROWING subtree (2.8 s,
  then 11 s each, climbing) — the direct cause of its census timeout.

Subtree evaluation is a deterministic pure function of canonical node data,
so content-hash caching is exact — the same doctrine as BD-EMIT-MESH-CACHE.
No theory: nothing here touches the enclosure mathematics.

```yaml
id:          FHC-FACTS-CACHE
contract:    [FHC-FACTS-CACHE]
class:       mechanical+
crates:      [truck123d]
depends_on:  [BD-EMIT-MESH-CACHE]
needs:       [BD-EMIT-MESH-CACHE]
write_allow:
  - truck123d/src/bd_bridge.rs
  - truck123d/tests/facts_cache.rs
read_allow:
  - corpus/ttc/door.py
  - docs/F1_HYPERCAR_GAP_REGISTER.md
tests_required: [truck123d/tests/facts_cache.rs]
anchors:
  - {id: A1, expect: 4, cmd: "grep -c 'boolean_product_volume_certified' truck123d/src/bd_bridge.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'fn bd_facts' truck123d/src/bd_bridge.rs"}
budget:      {turns: 60, ctx_tokens: 180000}
```

Anchors measured 2026-09-12 at the TRIM-landed HEAD. **Re-measure at
dispatch** — the docket ahead WILL drift them; update at dispatch, never
dispatch stale.

## Pre-made judgements

1. **Cache key = content hash of the canonical node JSON.** Node data is
   deterministic canonical data, so equal hash ⇒ equal evaluation. Hash-map
   iteration order must never reach an output (house determinism law).
2. **Two cache layers, in order of value:**
   a. **Top-level `bd_facts` memoization** — identical node evaluated twice
      in one process (authoring probe + final facts; repeated probes) pays
      once. This alone makes the final-facts pass free on any subtree the
      authoring layer already evaluated.
   b. **Child-patch reuse in the boolean entry** — when a parent boolean's
      operands are subtrees whose evaluated patch extraction is already
      cached, reuse the extracted patches and run only the NEW top-level
      operation, instead of re-solving the child's whole internal boolean
      chain. This is the fix for growing-subtree compositions (power_unit's
      `_fuse` chain, the suspension bore trees). If the entry's structure
      makes (b) larger than ~400 LOC, land (a) + the cheapest sound form of
      (b) and record the rest in RESULT notes — stay under the 1000-LOC
      owner budget for this packet.
3. **Eager probe calls stay.** door.py's discarded-result probes are
   functional for corpus retry idioms (the suspension `_fuse` micron-nudge
   retry depends on the failure signal at composition time). Do NOT remove
   them; the cache makes them cheap. If an investigation shows a probe is
   safe to drop, that is a door.py change outside this packet's write set —
   note it in RESULT instead.
4. **Soundness invariants (the gates):** cached and fresh evaluations of the
   same node return BIT-IDENTICAL records; cache population order never
   changes any output; the `None`-deflection fingerprint path (vehicle
   fingerprint 4,461,816) stays bit-identical — verify explicitly.
5. Out of scope: the enclosure-scheme cost itself (that is FHC-G6's
   decomposition after this lands — its profile should be re-run against
   the cached path), any change to the certified solver's mathematics, and
   any door.py edit.

## Method

Read the boolean entry path (`boolean_product_volume_certified` →
`node_box_patches` → `placed_box_patches`) and the `bd_facts` entry; add the
content-hash memo layer (a) with the identity tests, then (b) if it fits the
budget. Tests:

- `identical_node_evaluated_once` — same node through `bd_facts` twice:
  second call returns the bit-identical record (and the test may assert the
  cache hit via a counter if the bridge exposes one; otherwise via timing
  recorded in RESULT notes as data).
- `distinct_nodes_independent` — distinct subtrees never collide (hash
  sanity on structured near-identical nodes).
- `growing_composition_reuses_children` — a three-deep fuse chain where the
  children are pre-cached runs only the new top-level op (assert via call
  counters or timing data recorded in RESULT).
- `fingerprint_unchanged` — `bd_stl(None)` vehicle fingerprint
  4,461,816 bit-identical with the cache active.
- `cache_order_invisible` — populate the cache in different orders; outputs
  byte-identical.

## Done when

```
cargo fmt --check -p truck123d
cargo clippy -p truck123d --all-targets -- -D warnings
cargo test -p truck123d --test facts_cache --locked
```

plus a full-vehicle door spot-check with the cache active and the
fingerprint re-verified. Record before/after `bd_facts` call counts and wall
for `beam_wing` and `power_unit` (the counting-wrapper probe,
`scratch/facts_call_count.py`, is the instrument) in RESULT notes as data.
All cargo through the queue (the `cargo` on PATH IS the queue shim);
interpreter dir on PATH if STATUS_DLL_NOT_FOUND.

## Forbidden

Any file outside `write_allow`; editing `corpus/ttc/**` (including door.py —
the eager probes stay) or `vendor/truck/**`; touching the certified solver's
mathematics; `#[ignore]`, deleted or weakened tests, bare `cargo test`;
committing to `main`.

## Stop conditions

- cached-vs-fresh bit-identity cannot be achieved (the evaluation is not a pure function of node data) → `SPEC_GAP` naming the impurity — that finding matters more than the cache
- child-patch reuse exceeds the LOC budget → land layer (a) only, note the rest
- anchor count differs at dispatch and was not re-measured → `ANCHOR_MISMATCH`
- three consecutive failed cargo runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

```json
{"id":"FHC-FACTS-CACHE","status":"DONE","contracts":["FHC-FACTS-CACHE"],
 "anchors_verified":{"A1":4,"A2":1},
 "notes":"call counts + wall before/after for beam_wing and power_unit; fingerprint verified; which layers landed"}
```

Commit subject: `truck123d: content-hash facts memoization across compositions (FHC-FACTS-CACHE)`.
