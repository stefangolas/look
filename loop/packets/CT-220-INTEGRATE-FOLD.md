# WORK PACKET CT-220-INTEGRATE-FOLD — wire the interaction graph and volume memo into the multi-solid fold

Build-spec Wave 2 integration. The MONO-9 compound fold dispatches through
CT-202's interaction graph (per-component sub-folds; certified-disjoint
singleton/degenerate components short-circuit per the operation's
decomposition rule) and CT-203's memo (own-volume certificates paid once
per immutable construction). No change to the per-component certified
machinery itself.

```yaml
id:          CT-220-INTEGRATE-FOLD
contract:    [CT-220-INTEGRATE-FOLD]
class:       mechanical
crates:      [truck123d]
depends_on:  [CT-100-SCHEDULER-WRAP, CT-202-GRAPH-CORE, CT-203-MEMO-CORE]
needs:       [CT-100-SCHEDULER-WRAP, CT-202-GRAPH-CORE, CT-203-MEMO-CORE]
write_allow:
  - truck123d/src/bd_bridge.rs
  - truck123d/tests/atlas_integrate_fold.rs
read_allow:
  - docs/CONTACT_ATLAS_SPEC.md
  - docs/CONTACT_ATLAS_BUILD_SPEC.md
tests_required: [truck123d/tests/atlas_integrate_fold.rs]
anchors:
  - {id: A1, min: 2,  cmd: "grep -cE '\\bcompound_indicator\\b' truck123d/src/bd_bridge.rs"}
  - {id: A2, min: 40, cmd: "grep -cE '\\bVolumeRow\\b' truck123d/src/bd_bridge.rs"}
budget:      {turns: 45, ctx_tokens: 140000}
```

Anchors at authoring time `e271ba9`. **Re-measure at dispatch.**

## Pre-made judgements

1. **Decomposition routing, frozen per operation**: union → per-component
   sub-folds summed; intersection → more than one certified-disjoint
   component ⇒ EMPTY certificate (zero-width zero); difference ⇒
   subtractor components certified disjoint from the base are dropped.
   The graph type's refusal for non-decomposable DAGs surfaces typed.
2. **Memo scope**: own-volume lookups keyed by ConstructionIdentity; a
   component sub-fold's INTERNAL work is never memoized (only the
   closed-operand volume phase — spec §6C's scope limit).
3. **V5 net**: single-component graphs (the today-normal case) must
   produce behavior identical to CT-100/CT-210 — the graph is invisible
   when every operand interacts. Multi-component fixtures must match
   analytic sums exactly.
4. **Stats**: `components`, `graph_edges`, `memo_hits`, `memo_refines`
   recorded per dispatch.

## Done-when

1. `cargo test -p truck123d --test atlas_integrate_fold --locked` green:
   a two-cluster union decomposes (component counts asserted, analytic
   sum matched); an all-disjoint union short-circuits with zero boolean
   dispatches; an intersection of certified-disjoint clusters returns the
   empty certificate; memo hits observed on a repeated construction;
   single-component V5 parity with CT-100.
2. `cargo test -p truck123d --test fuse_fold --locked` green.
3. fmt clean; `clippy -p truck123d --lib` clean on the diff.
4. Write `RESULT.json` AT THE WORKTREE ROOT (nowhere else), including the
   component/memo stats per fixture.
