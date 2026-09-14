# WORK PACKET CT-201-HIERARCHY-CORE — aggregate flux annotation and whole-subtree retirement (P18, Theorem 8A)

Build-spec Wave 2. The hierarchy-first clear-region fast path: a BVH node
carries the outward-rounded sum of its descendant leaf flux brackets
`F(U)`, and a certified whole-node membership result retires the entire
subtree at `sigma * F(U)` — no descendant patch classification, contact,
or fallback work (spec §6A, Theorem 8A). Pure interval bookkeeping over
the CT-200 BVH and the CT-000 contracts; dispatch integration is CT-210.

```yaml
id:          CT-201-HIERARCHY-CORE
contract:    [CT-201-HIERARCHY-CORE]
class:       design
crates:      [truck123d]
depends_on:  [CT-200-BVH-CORE]
needs:       [CT-200-BVH-CORE]
write_allow:
  - truck123d/src/atlas/retire.rs
  - truck123d/tests/atlas_retire.rs
read_allow:
  - docs/CONTACT_ATLAS_SPEC.md
tests_required: [truck123d/tests/atlas_retire.rs]
anchors:
  - {id: A1, min: 1,  cmd: "grep -cE '\\bmod atlas\\b' truck123d/src/lib.rs"}
  - {id: A2, min: 40, cmd: "grep -cE '\\bVolumeRow\\b' truck123d/src/bd_bridge.rs"}
budget:      {turns: 35, ctx_tokens: 120000}
```

Anchors at authoring time `e271ba9`. **Re-measure at dispatch.**

## Frozen contracts

1. **Aggregate flux `F(U)`**: bottom-up outward-rounded interval sum of
   leaf whole-patch flux brackets. Leaf brackets are INPUTS (the caller's
   per-patch flux certificates — in production the operand-volume phase
   produces them); this module never recomputes a leaf flux.
2. **Node membership**: `EntireInside | EntireOutside | Unresolved` per
   the spec's contract. **Conservative rule, frozen**: a retirement-
   grade result requires a certificate covering every descendant point —
   hull-level box tests alone yield `Unresolved` (a box test may be
   necessary but is never sufficient). The certificate provider is
   injected; this module defines the interface and the retirement rule,
   not the geometry.
3. **Retirement rule**: `EntireInside/EntireOutside` + the operation's
   selector multiplier `sigma ∈ {-1,0,+1}` (frozen per-operation table:
   union A outside B ⇒ +1, A inside ⇒ 0; difference A outside ⇒ +1, A
   inside ⇒ 0, B inside A ⇒ −1, B outside ⇒ 0; intersection: inside/inside
   ⇒ +1 per operand, otherwise 0) ⇒ subtree contribution enclosed by
   `sigma * F(U)` (interval scale, outward). `Unresolved` descends.
4. **Never hides contact**: an `Unresolved` node always reaches the
   co-support/contact machinery. The retirement fast path is additive —
   the fallback path is unchanged.

## Done-when

1. `cargo test -p truck123d --test atlas_retire --locked` green: bottom-up
   sum matches direct leaf sums (interval soundness); retirement on
   synthetic trees with stub certificates retires exactly the certified
   subtrees; sigma tables per operation match Theorem 8A's statement;
   Unresolved descends and never retires.
2. fmt clean; `clippy -p truck123d --lib` clean on the diff.
3. Write `RESULT.json` AT THE WORKTREE ROOT (nowhere else).
