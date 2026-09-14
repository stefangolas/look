# WORK PACKET CT-203-MEMO-CORE — rigid-placement volume certificate cache (P20, Theorem 8C)

Build-spec Wave 2. Closes operand-volume certificates under the
construction identity frozen in CT-000: `V(T(A)) = V(A)` for rigid
placements `T`, so a fold that dispatches `k` times over `m_unique`
immutable constructions pays the own-volume phase once per construction
at the cached precision (spec §6C, eq. 4f). Pure cache semantics over the
CT-000 contracts; dispatch integration is CT-220.

```yaml
id:          CT-203-MEMO-CORE
contract:    [CT-203-MEMO-CORE]
class:       design
crates:      [truck123d]
depends_on:  [CT-000-ATLAS-CONTRACT-SHIM]
needs:       [CT-000-ATLAS-CONTRACT-SHIM]
write_allow:
  - truck123d/src/atlas/memo.rs
  - truck123d/tests/atlas_memo.rs
read_allow:
  - docs/CONTACT_ATLAS_SPEC.md
tests_required: [truck123d/tests/atlas_memo.rs]
anchors:
  - {id: A1, min: 1,  cmd: "grep -cE '\\bmod atlas\\b' truck123d/src/lib.rs"}
  - {id: A2, min: 40, cmd: "grep -cE '\\bVolumeRow\\b' truck123d/src/bd_bridge.rs"}
budget:      {turns: 30, ctx_tokens: 100000}
```

Anchors at authoring time `e271ba9`. **Re-measure at dispatch.**

## Frozen contracts

1. **Cache key**: the CT-000 `ConstructionIdentity` (canonical
   pre-placement serialization + certificate-precision tag). Placement
   transforms MUST NOT change the key — that is Theorem 8C's content and
   the reason the identity is pre-placement.
2. **Reuse rule**: a cached bracket is returned when its width is ≤ the
   request's required width; otherwise the certificate is refined ONCE,
   the tighter bracket replaces the entry, and the refined value is
   returned. Never return a wider bracket than the request allows; never
   return an unsound bracket (entries are only ever written from
   certified computations).
3. **Invalidation**: immutable construction identity means no
   mutation path exists; a NEW construction (edited tree) is a new
   identity by hashing, not an invalidation. This is the design
   consequence of content addressing — the packet states it explicitly
   so no one adds invalidation hooks later.
4. **Scope limit (frozen)**: closed-operand volume certificates ONLY.
   Open-patch flux intervals are placement-dependent and MUST NOT be
   cached under this mechanism (spec §6C's explicit warning).

## Done-when

1. `cargo test -p truck123d --test atlas_memo --locked` green: hit on
   same-identity/same-precision; miss on precision upgrade with
   refine-once-then-reuse; miss on identity change; the width rule (never
   returns wider than requested); a rigid-placement scenario (identity
   invariant under a recorded transform) reuses the entry.
2. fmt clean; `clippy -p truck123d --lib` clean on the diff.
3. Write `RESULT.json` AT THE WORKTREE ROOT (nowhere else).
