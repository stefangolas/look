# WORK PACKET CFP-005-BRANCH-BVH — span BVH with Theorem 3 separation and memoization

You are landing the inner broadphase: a per-carrier BVH over the D1 Bézier
decomposition so that loft×loft contact enumerates a handful of span-pair
hull evaluations instead of the full span-pair cartesian product. Everything
you need is in this document, `docs/CONTACT_FAST_PATH_BUILD_SPEC.md` §3
(Theorem 3 + certificate discipline) and §4. Do not read other spec files.
SPEC_GAP discipline applies.

```yaml
id:          CFP-005-BRANCH-BVH
contract:    [CFP-005-BRANCH-BVH]
class:       design
crates:      [truck-certified]
depends_on:  [CFP-003-SEPARABILITY]
write_allow:
  - vendor/truck/truck-certified/src/bvh.rs
  - vendor/truck/truck-certified/src/lib.rs
read_allow:
  - docs/CONTACT_FAST_PATH_BUILD_SPEC.md
  - vendor/truck/truck-certified/src/cfp/spine.rs
  - vendor/truck/truck-certified/src/cfp/fixtures.rs
  - vendor/truck/truck-certified/src/patch_admit.rs
  - vendor/truck/truck-certified/src/hull.rs
  - vendor/truck/truck-certified/src/formal/exact.rs
tests_required:
  - fc5_span_pair_found_in_large_decomposition
  - fc5_prune_count_matches_ground_truth
  - separation_certificate_is_exact_sign_row
  - float_hint_never_evidence
  - separation_inherits_to_descendants
  - bvh_traversal_deterministic
budget:      {turns: 80, ctx_tokens: 180000}
```

## Problem

After D1 decomposition, a 20×30-span loft pair is ~360,000 Bézier-pair
admissions beneath a single `contact()` call — knot insertion, grid
construction, and chart search paid per span pair with no geometric screen
between them. Theorem 3 makes a convex separation test on control nets both
cheap (O(n₁+n₂) certificate) and hereditary (a separating direction at a
parent node separates every descendant pair — no re-verification). A BVH over
span rectangles with certified `enclose` boxes at the leaves converts the
admission mass to a few thousand hull evaluations (10–100x on the phase; the
feasibility item for multi-span loft booleans).

## Scope decisions — pre-made, do not relitigate

1. **The leaves are `AdmittedPatch`es.** The BVH is built over
   `SplinePatchStack`'s span rectangles (`patch_admit.rs:87-101`); leaf
   bounds are the per-span certified `enclose` boxes — the same computation
   CFP-003's per-side hulls already pay for. Internal nodes bound the union.
2. **SFC discipline is structural.** The GJK/LP float search produces a
   `FloatHint` (the spine's type — carries no evidence status). The
   certificate is the exact `Expansion` sign row:
   `min_α λ·P¹_α > max_β λ·P²_β` over the node's net constants — O(n₁+n₂),
   BG-ENC-003-clean. A separation verdict EXISTS only where the exact row
   certifies. If the search finds no direction the exact row certifies, the
   node is NOT separated and is descended — the float layer can never cause
   a false prune.
3. **Separation certificates inherit** (Theorem 3): a node certificate
   prunes every descendant pair without re-verification. Contact-candidate
   pairs that survive traversal are emitted in a FIXED order (leaf index
   lexicographic) — the consumer's downstream behavior is unchanged, only
   shorter.
4. **Dyadic memoization rides CFP-003's cache**: per-side hulls on the
   subdivision path are the landed per-side memoization (Corollary 1.4);
   this packet adds no second cache — it consumes the one that exists.
5. **Leaves count against D3** (the spec's budget note; the constant was
   revisited in CFP-003 — if the leaf boxes do not fit the revised budget,
   STOP, SPEC_GAP).
6. **Zero new top-level evidence kinds.** A pruned node produces nothing; a
   surviving pair produces exactly what the landed funnel produced before.
   The BVH changes WHICH pairs are enumerated, never what a pair's answer is.
7. **`ssi4.rs` is untouched** (direct-evaluation path, frozen after CFP-007).
   This packet is consumed by the span-pairing site of the SSI funnel —
   wire through the accessors CFP-003 landed; if that wire site cannot be
   reached without editing a file outside `write_allow`, SPEC_GAP.

## Anchors — base-relative, drift-tolerant

Re-check on your branch:

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `truck-certified/src/patch_admit.rs` | `pub struct SplinePatchStack` | 1 |
| A2 | `truck-certified/src/lib.rs` | `pub mod bvh` | **0 before, 1 after your change** |
| A3 | `truck-certified/src/ssi_types.rs` | `grids: [Vec<Vec<f64>>; 3]` | **0** (CFP-003 landed; if 1, CFP-003 has not merged — BLOCKED, not ANCHOR_MISMATCH) |
| A4 | `truck-certified/src/cfp/spine.rs` | `FloatHint` | 1 (read-only spine anchor) |

A2 is the packet-owned delta. A3 failing means a DEPENDENCY is missing:
report `BLOCKED` with the missing packet id, do not improvise.

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `todo!` reachable from geometry.
- **H-3** No absolute constants in predicates; test epsilons carry `// H-3`
  on the SAME line as the literal.
- **BG-ENC-003**: outward rounding; no fast-math; no FMA contraction.
- **Determinism**: fixed build order (span stack order), fixed axis choice
  rule (widest extent, ties lowest index, low-before-high), fixed traversal
  order; identical ordered input → identical pair list, byte-for-byte.
- **All cargo invocations go through the queue (the `cargo` on PATH IS the
  queue shim).**

## Tests required

Named `#[test]` fns; the verifier checks the names appear in your diff.
Fixture data copied from `cfp/fixtures.rs` as constants, read-only.

1. `fc5_span_pair_found_in_large_decomposition` — F-C5: the one contact span
   pair in the 20×30-span record is found.
2. `fc5_prune_count_matches_ground_truth` — the fixture's expected dyadic
   prune count is met exactly (the 360k→handful claim, pinned).
3. `separation_certificate_is_exact_sign_row` — every prune in the F-C3/F-C5
   runs carries the exact `Expansion` row; none carries only a `FloatHint`.
4. `float_hint_never_evidence` — compile-time shape assertion: `FloatHint`
   does not implement the evidence traits (spine decision 2, enforced here).
5. `separation_inherits_to_descendants` — a parent-node certificate prunes
   the descendant pairs; the test descends manually and asserts zero
   re-verification calls.
6. `bvh_traversal_deterministic` — two runs on identical input produce
   identical pair lists and identical prune records.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-certified
cargo clippy -p truck-certified --all-targets -- -D warnings
cargo test -p truck-certified --lib bvh
```

Send cargo output to a file and read the tail.

## Forbidden

Editing any file outside `write_allow` — especially `ssi.rs`, `ssi_types.rs`,
`patch_admit.rs`, `tangency/**`, `construct/**`, anything under
`truck-evidence/` or `truck-shapeops/`, `Cargo.lock`. Pruning on a float hint
without an exact certificate. Changing the span stack or admission gates.
Adding `#[ignore]`. Adding `#[allow]` without a justification comment on the
same line. Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- A3 shows the CFP-003 refactor has not merged → `BLOCKED` (dependency), name
  the packet
- leaf boxes do not fit the revised D3 budget → SPEC_GAP
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

**COMMIT BEFORE writing `RESULT.json`.** Then write `RESULT.json` at the root
of your worktree.

```json
{"id":"CFP-005-BRANCH-BVH","status":"DONE","contracts":["CFP-005-BRANCH-BVH"],
 "tests_added":6,"anchors_verified":{"A1":1,"A2":1,"A3":0,"A4":1},
 "notes":"span BVH landed; SFC certificate discipline enforced; any deviation stated here"}
```
