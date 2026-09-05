# WORK PACKET PB-001-SELECTORS — scoped selector layer over live topology

You are implementing the selector layer of the Python Bridge (PB) program's
Rust client phase. Everything you need is in this document and
`docs/TRUCK123D_PY_BRIDGE_SPEC.md` + the frozen contract
`docs/PY_BRIDGE_CONTRACT.md`. If something you need is genuinely missing,
that is a SPEC_GAP (see "Stop conditions"): you stop and report, you do not
research it.

```yaml
id:          PB-001-SELECTORS
contract:    [PB-001-SELECTORS]
class:       mechanical
crates:      [truck-modeling]
depends_on:  [PB-000-CONTRACT]
write_allow:
  - vendor/truck/truck-modeling/src/selectors.rs
  - vendor/truck/truck-modeling/src/lib.rs
  - vendor/truck/truck-modeling/tests/pb_selectors.rs
read_allow:
  - vendor/truck/truck-topology/src/entity_id.rs
  - vendor/truck/truck-modeling/src/facade.rs
  - vendor/truck/truck-shapeops/src/facade.rs
  - docs/PY_BRIDGE_CONTRACT.md
  - docs/TRUCK123D_PY_BRIDGE_SPEC.md
tests_required:
  - face_iteration_yields_stable_order
  - centroid_and_aabb_match_brute
  - axis_sort_group_filter_semantics
  - edge_resolution_names_blend_targets
budget:      {turns: 50, ctx_tokens: 120000}
```

**New file** (`selectors.rs`): H-1 applies.

## Problem

The facade books selectors to this program (`facade.rs:10`). The Python
layer's fluent `> Faces().SortBy(Axis.Z)` vocabulary needs a Rust substrate:
iteration over a solid's faces/edges with deterministic order, geometric
facts per element, and resolution into `BlendSpec`-compatible edge names.

## Scope decisions — pre-made, do not relitigate

1. **Identity comes from `entity_id.rs`** — consume `EntityId`/`Selector`/
   `Op` (`sel(base, selector)`); do not invent a parallel identity scheme.
   Your refs ARE EntityId-derived.
2. **Per-face facts**: centroid + AABB via fan-sampling of the tessellated
   face (the showcases harness method — read `showcases/src/harness.rs`
   for the pattern); exact where the carrier is analytic, sampled
   otherwise, and the Method tag says which (H-6).
3. **The query vocabulary** (pre-decided): `faces(solid)`, `edges(solid)`
   (deterministic topological order — by EntityId, never hash order),
   `sort_by_axis(axis)`, `group_by_axis(axis)`, `filter_by_plane(plane,
   tol-from-ctx)`, `take(n)`/`last()`.
4. **Edge resolution**: a selected edge resolves to the name
   `BlendSpec`/facade fillet/chamfer consume — endpoint pairs for straight
   edges, the canonical rim for circles (read the landed facade's
   `fillet`/`chamfer` signatures and match what they accept).
5. `lib.rs` gets ONE line: `pub mod selectors;`.

## Anchors — measured 2026-09-05, counts are exact

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `vendor/truck/truck-topology/src/entity_id.rs` | `pub enum Selector` | 1 |
| A2 | `vendor/truck/truck-topology/src/entity_id.rs` | `pub fn sel\(` | 1 |
| A3 | `vendor/truck/truck-modeling/src/lib.rs` | `^pub mod` | 13 |
| A4 | `vendor/truck/truck-shapeops/src/facade.rs` | `pub fn fillet\(` | 1 |

A3 becomes 14 when you add `pub mod selectors;`.

## House rules

- **H-1** no unwrap/expect/panic reachable from geometry; **H-3** same-line
  `// H-3` for test epsilons; **H-6** sampled facts are never `Exact`.
- **Determinism**: iteration order is topological (EntityId), never hash.
- **All cargo through the queue shim.** Scoped commands only.

## Tests required

1. `face_iteration_yields_stable_order` — two iterations of the same solid
   yield identical order; a transformed copy yields EntityId-consistent refs.
2. `centroid_and_aabb_match_brute` — the sampled centroid/AABB bracket the
   brute dense-sampled values within `// H-3` tolerance on ≥3 solids.
3. `axis_sort_group_filter_semantics` — sort/group/filter/take compose into
   the documented selection (top face by Z, faces on a plane, last edge).
4. `edge_resolution_names_blend_targets` — selected edges resolve to names
   the landed `fillet` accepts (compile + a runtime round-trip on a box:
   select vertical edges → fillet → result is a real solid).

No existing test may be deleted, `#[ignore]`d, or weakened.

## Done when

```
cargo fmt --check -p truck-modeling
cargo clippy -p truck-modeling --all-targets -- -D warnings
cargo test -p truck-modeling --tests
cargo check -p truck-shapeops
```

## Forbidden

Anything outside `write_allow` — especially `entity_id.rs`, the shapeops
facade, `facet_sweep.rs` (PB-003's file), `scripts/kernel-gates.sh`,
`Cargo.lock`. Adding `#[ignore]`. Unjustified `#[allow]`. Committing to
`main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- edge resolution cannot match what landed `fillet` accepts → `SPEC_GAP`,
  naming the signature mismatch
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

```json
{"id":"PB-001-SELECTORS","status":"DONE","contracts":["PB-001-SELECTORS"],
 "tests_added":4,"anchors_verified":{"A1":1,"A2":1,"A3":14,"A4":1},
 "notes":"the EdgeRef name format you landed and its compatibility argument"}
```

Commit subject: `feat(modeling): scoped selector layer over EntityId identity (PB-001-SELECTORS)`.
