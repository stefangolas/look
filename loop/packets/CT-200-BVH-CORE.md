# WORK PACKET CT-200-BVH-CORE — certified bounding hulls and the pair-search BVH (P5 core)

Build-spec Wave 2. A deterministic BVH over patch control hulls with a
certified node-pair lower-bound contract, per spec §6 / Theorem 8. Pure
geometry/bookkeeping over intervals and boxes: **no kernel coupling** —
the integration into dispatch is CT-210. This packet is what makes the
pair search sub-quadratic in the common case.

```yaml
id:          CT-200-BVH-CORE
contract:    [CT-200-BVH-CORE]
class:       design
crates:      [truck123d]
depends_on:  [CT-000-ATLAS-CONTRACT-SHIM]
needs:       [CT-000-ATLAS-CONTRACT-SHIM]
write_allow:
  - truck123d/src/atlas/bvh.rs
  - truck123d/tests/atlas_bvh.rs
read_allow:
  - docs/CONTACT_ATLAS_SPEC.md
tests_required: [truck123d/tests/atlas_bvh.rs]
anchors:
  - {id: A1, min: 40, cmd: "grep -cE '\\bVolumeRow\\b' truck123d/src/bd_bridge.rs"}
  - {id: A2, min: 1,  cmd: "grep -cE '\\bmod atlas\\b' truck123d/src/lib.rs"}
budget:      {turns: 40, ctx_tokens: 130000}
```

Anchors at authoring time `e271ba9`. **Re-measure at dispatch.**

## Frozen contracts

1. **Hull representation**: a node's hull is one outward-rounded interval
   box per axis, built from leaf control-hull boxes by outward hull
   composition. Leaf input: the caller's per-patch control-hull boxes
   (never recomputed here).
2. **Build**: O(N log N), deterministic — split at the median of the
   widest axis, tie-break by canonical leaf id. No randomness, no
   insertion-order dependence.
3. **Lower-bound contract (P5)**: for two nodes,
   `lb_gap(U, W) : R^3 interval` is a certified LOWER bound on
   `dist(p, q)` for every leaf pair `p ∈ U, q ∈ W` — one-sided error
   toward refusal (overestimating the gap is forbidden; the proof is the
   convex-hull property + outward rounding). Pairs prune only when
   `lb_gap > 0` on some axis (strict separation) — the prune predicate
   CT-210 dispatches on.
4. **Transversality is NOT this type's job**: metric separation here;
   angular/rank margins are CT-400's. The spec's warning holds — a box
   gap is never reused as an angle bound.

## Done-when

1. `cargo test -p truck123d --test atlas_bvh --locked` green: build on
   synthetic hull sets (known overlap/disjoint configurations) — pruning
   exactness against brute-force all-pairs on the same inputs; depth and
   balance sanity; determinism (same input tree built twice → identical
   structure).
2. fmt clean; `clippy -p truck123d --lib` clean on the diff.
3. Write `RESULT.json` AT THE WORKTREE ROOT (nowhere else).
