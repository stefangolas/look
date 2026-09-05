# WORK PACKET PB-003-CONCAVE-CAPS — CDT cap path for non-convex facet rings

You are implementing the concave-cap layer of the Python Bridge (PB)
program's Rust client phase. Everything you need is in this document and
`docs/TRUCK123D_PY_BRIDGE_SPEC.md` + `docs/PY_BRIDGE_CONTRACT.md`. If
something you need is genuinely missing, that is a SPEC_GAP (see "Stop
conditions"): you stop and report, you do not research it.

```yaml
id:          PB-003-CONCAVE-CAPS
contract:    [PB-003-CONCAVE-CAPS]
class:       mechanical
crates:      [truck-modeling]
depends_on:  [PB-000-CONTRACT]
write_allow:
  - vendor/truck/truck-modeling/src/facet_sweep.rs
  - vendor/truck/truck-modeling/tests/pb_concave_caps.rs
read_allow:
  - vendor/truck/truck-modeling/src/facet_sweep.rs
  - vendor/truck/truck-meshalgo/src/tessellation/triangulation.rs
  - docs/PY_BRIDGE_CONTRACT.md
  - docs/TRUCK123D_PY_BRIDGE_SPEC.md
tests_required:
  - concave_ring_caps_through_cdt
  - convex_fast_path_bit_identical
  - u_chute_negative_test_now_positive
budget:      {turns: 45, ctx_tokens: 110000}
```

**New test file** (`pb_concave_caps.rs`): H-1 applies; no landed test file
may be touched.

## Problem

`ring_is_convex` gates the facet backend's cap triangulation — a
non-convex cap ring refuses. The per-face CDT path exists (CG-005 landed;
`truck-meshalgo` triangulation internals). This packet routes non-convex
rings through the CDT path; the U-chute negative test inverts to positive.

## Scope decisions — pre-made, do not relitigate

1. **The convexity fast path stays for convex rings, bit-identical** (V5
   identity guard): same ring → same triangles, byte-equal STL on every
   convex fixture that exists today. The CDT path only ever swaps a
   refusal for a triangulation.
2. **Routing**: replace the convexity gate's refuse arm with a CDT
   dispatch for the non-convex case; the gate itself stays as the
   ROUTER (its boolean result selects the path) — you are not deleting the
   convexity analysis, you are giving its `false` arm a destination.
3. **Self-intersecting rings** refuse typed as today — CDT is not a repair.
4. `truck-meshalgo` is read-only (the ledger/CDT internals are consumed,
   never edited).

## Anchors — measured 2026-09-05, counts are exact

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `vendor/truck/truck-modeling/src/facet_sweep.rs` | `ring_is_convex` | 2 |
| A2 | `vendor/truck/truck-meshalgo/src/tessellation/triangulation.rs` | `fn triangulate` | 1 |

## House rules

- **H-1** no unwrap/expect/panic reachable from geometry; **H-3** same-line
  `// H-3`; **H-6** caps are facet output, tagged accordingly.
- **All cargo through the queue shim.** Scoped commands only.

## Tests required

1. `concave_ring_caps_through_cdt` — an L-shaped (non-convex) profile
   sweeps and caps correctly; cap triangles cover the ring without
   overlap; the solid is closed with positive volume.
2. `convex_fast_path_bit_identical` — every convex fixture's cap
   triangulation is byte-identical before/after (hash the STL bytes).
3. `u_chute_negative_test_now_positive` — the recorded U-chute refusal
   case now produces a valid solid; the ORIGINAL refusal assertion is
   replaced, not deleted (state the replacement in RESULT notes).

No existing test may be deleted, `#[ignore]`d, or weakened — except the
single recorded U-chute assertion this packet's spec books as inverted
(name it in RESULT notes; the verifier accepts exactly this documented
inversion).

## Done when

```
cargo fmt --check -p truck-modeling
cargo clippy -p truck-modeling --all-targets -- -D warnings
cargo test -p truck-modeling --tests
cargo check -p truck-shapeops
```

## Forbidden

Anything outside `write_allow` — especially `truck-meshalgo/**`,
`selectors.rs` (PB-001's file), landed test files,
`scripts/kernel-gates.sh`, `Cargo.lock`. Adding `#[ignore]`. Unjustified
`#[allow]`. Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- the CDT internals cannot be reached read-only (an API gap in
  truck-meshalgo) → `SPEC_GAP`, naming the missing entry
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

```json
{"id":"PB-003-CONCAVE-CAPS","status":"DONE","contracts":["PB-003-CONCAVE-CAPS"],
 "tests_added":3,"anchors_verified":{"A1":2,"A2":1},
 "notes":"the inverted U-chute assertion by name, and the convex-path byte-identity evidence"}
```

Commit subject: `feat(modeling): CDT cap path for non-convex facet rings (PB-003-CONCAVE-CAPS)`.
