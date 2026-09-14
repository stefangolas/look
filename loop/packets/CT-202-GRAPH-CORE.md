# WORK PACKET CT-202-GRAPH-CORE — the certified interaction graph for multi-solid folds (P19, Theorem 8B)

Build-spec Wave 2. The union fold over `m` operands becomes a sparse
decomposition: an interaction graph with an edge `(i,j)` iff broad phase
cannot certify `A_i ∩ A_j = ∅` — an absent edge is a *proof* of
disjointness — connected components computed by union-find, each component
evaluated as an independent sub-fold, singleton components requiring no
boolean work (spec §6B). Pure bookkeeping over the CT-000 contracts and
hull boxes; fold integration is CT-220.

```yaml
id:          CT-202-GRAPH-CORE
contract:    [CT-202-GRAPH-CORE]
class:       design
crates:      [truck123d]
depends_on:  [CT-000-ATLAS-CONTRACT-SHIM]
needs:       [CT-000-ATLAS-CONTRACT-SHIM]
write_allow:
  - truck123d/src/atlas/graph.rs
  - truck123d/tests/atlas_graph.rs
read_allow:
  - docs/CONTACT_ATLAS_SPEC.md
tests_required: [truck123d/tests/atlas_graph.rs]
anchors:
  - {id: A1, min: 2,  cmd: "grep -cE '\\bcompound_indicator\\b' truck123d/src/bd_bridge.rs"}
  - {id: A2, min: 40, cmd: "grep -cE '\\bVolumeRow\\b' truck123d/src/bd_bridge.rs"}
budget:      {turns: 35, ctx_tokens: 120000}
```

Anchors at authoring time `e271ba9`. **Re-measure at dispatch.**

## Frozen contracts

1. **Edge semantics**: absent edge ⇒ certified pairwise disjointness
   (decided from control-hull box separation — the conservative
   may-overlap test; CT-200's BVH accelerates it later, never weakens
   it). Presence of an edge proves nothing.
2. **Operation-specific decomposition, frozen**: union → per-component
   sub-folds (Theorem 8B); intersection → more than one certified-disjoint
   component ⇒ the total intersection is EMPTY (certificate: zero with
   zero-width bracket); difference `A \ (union B_i)` ⇒ subtractor
   components certified disjoint from A are discarded. Arbitrary Boolean
   DAGs do NOT decompose — this type refuses them at its interface.
3. **Determinism**: components numbered by smallest member id; union-find
   with union-by-min; the decomposition output is a canonical list.
4. **Finiteness**: `O(m + |E| α(m))` after the may-overlap relation.

## Done-when

1. `cargo test -p truck123d --test atlas_graph --locked` green: known
   overlap patterns (two clusters far apart → two singleton-ish
   components; a chain → one component; all-disjoint → m components with
   zero boolean work); the intersection-early-empty and
   difference-discard rules; determinism of the decomposition output.
2. fmt clean; `clippy -p truck123d --lib` clean on the diff.
3. Write `RESULT.json` AT THE WORKTREE ROOT (nowhere else).
