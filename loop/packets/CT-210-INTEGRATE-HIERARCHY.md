# WORK PACKET CT-210-INTEGRATE-HIERARCHY — wire BVH + retirement into the boolean dispatch

Build-spec Wave 2 integration. Wires CT-200's BVH and CT-201's retirement
into the dispatch path in `bd_bridge.rs`: operand patches are indexed,
subtrees retire on certified whole-node membership before any per-patch
work, and only non-retired patches enter broad phase / refinement. The
certificate records the fast-path stats.

```yaml
id:          CT-210-INTEGRATE-HIERARCHY
contract:    [CT-210-INTEGRATE-HIERARCHY]
class:       mechanical
crates:      [truck123d]
depends_on:  [CT-100-SCHEDULER-WRAP, CT-200-BVH-CORE, CT-201-HIERARCHY-CORE]
needs:       [CT-100-SCHEDULER-WRAP, CT-200-BVH-CORE, CT-201-HIERARCHY-CORE]
write_allow:
  - truck123d/src/bd_bridge.rs
  - truck123d/tests/atlas_integrate_hierarchy.rs
read_allow:
  - docs/CONTACT_ATLAS_SPEC.md
  - docs/CONTACT_ATLAS_BUILD_SPEC.md
tests_required: [truck123d/tests/atlas_integrate_hierarchy.rs]
anchors:
  - {id: A1, min: 1,  cmd: "grep -cE '\\bmod atlas\\b' truck123d/src/lib.rs"}
  - {id: A2, min: 40, cmd: "grep -cE '\\bVolumeRow\\b' truck123d/src/bd_bridge.rs"}
budget:      {turns: 45, ctx_tokens: 140000}
```

Anchors at authoring time `e271ba9`. **Re-measure at dispatch.**

## Pre-made judgements

1. **Order of application, frozen**: co-support/admission unchanged →
   BVH build over each operand's patches → hierarchy-first retirement
   passes (per operation's selector table) → broad phase over
   NON-retired patches only → refinement. The retirement never runs
   after refinement starts.
2. **Membership certificate provider**: Whole-node membership calls the
   landed membership primitive at representative points PLUS hull-level
   separation tests; the conservative rule (box tests alone ⇒
   Unresolved) is binding — an incorrect Entire* result is the one way
   this fast path could break soundness.
3. **Stats are part of the certificate**: `retired_subtrees`,
   `retired_patches`, `bvh_pairs_visited` recorded per dispatch. A run
   with zero retirements must behave identically to CT-100's output
   (V5 net) — retired counts of zero is the only difference.
4. **No broad-phase behavior change beyond filtering**: pairs that
   survive retirement behave exactly as in CT-100. Determinism
   preserved (BVH build is CT-200-deterministic).

## Done-when

1. `cargo test -p truck123d --test atlas_integrate_hierarchy --locked`
   green: a separated-geometry fixture retires most patches (retired
   counts > 0, same bracket as the unretired path); a fully-contacting
   fixture retires nothing and matches CT-100's bracket exactly; V5 net
   on the CT-000 fixture kit.
2. `cargo test -p truck123d --test fuse_fold --locked` green.
3. fmt clean; `clippy -p truck123d --lib` clean on the diff.
4. Write `RESULT.json` AT THE WORKTREE ROOT (nowhere else), including
   retired/visited counts per fixture.
