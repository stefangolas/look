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
  - vendor/truck/truck-modeling/tests/facet_sweep_conformance.rs
read_allow:
  - vendor/truck/truck-modeling/src/facet_sweep.rs
  - vendor/truck/truck-meshalgo/src/tessellation/triangulation.rs
  - docs/PY_BRIDGE_CONTRACT.md
  - docs/TRUCK123D_PY_BRIDGE_SPEC.md
tests_required:
  - concave_ring_caps_through_cdt
  - convex_fast_path_bit_identical
  - concave_ring_refusals_stay_typed
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

> **AMENDMENT r2 (2026-09-05, orchestrator; resolves the r1 ANCHOR_MISMATCH
> and its two corroborating SPEC_GAP findings).** The r1 packet assumed the
> CG-005 per-face CDT (`truck-meshalgo`) was reusable from
> `facet_sweep.rs`. Measured: `truck-meshalgo` depends on
> `truck-modeling` (a modeling→meshalgo edge is a cycle), no cap-reachable
> CDT entry exists in modeling's dependency closure, and no landed
> U-chute test exists. The re-scope:
>
> 1. **Self-contained deterministic cap triangulation lands IN
>    `truck-modeling`** (inside `facet_sweep.rs` or a sibling module in
>    the same write set — prefer a `cap_triangulation` module-unit within
>    facet_sweep.rs to keep the write set to one file): ear-clipping over
>    the cap ring projected to its carrier plane. Caps are planar,
>    hole-free simple polygons; ear-clipping is O(n²) worst case, fully
>    deterministic (leftmost-most-convex ear order — state the tie-break
>    in a doc comment), and needs no new dependency.
> 2. **The convexity fast path stays bit-identical** (V5 guard): same
>    convex ring → byte-equal STL. The ear-clipping path only ever
>    replaces a refusal.
> 3. **Self-intersecting rings refuse typed as today** — no repair, no
>    tolerance games; the ring's simplicity check precedes triangulation.
> 4. **No U-chute inversion** (the booked test never existed): the
>    positive-case test is a non-convex ring of your construction with
>    stated ground truth instead.
> 5. **AMENDMENT r3 (2026-09-05, orchestrator; resolves the r2
>    remaining-blocker): `facet_sweep_conformance.rs` joins the write
>    set with ONE booked change** — the landed
>    `non_convex_cap_refuses` test (L-shape fixture, ring_resolution 6,
>    asserting `Err(ConstructError::InvalidInput)`) is the envelope line
>    this packet deliberately moves: replace the refusal assertion with
>    the certified-success assertion matching r2 test 1 (same fixture,
>    valid closed solid), keeping the test's NAME. Every other assertion
>    in that file is byte-identical (V5). The packet's Forbidden
>    "landed test file" clause does not apply to this single booked
>    inversion.
4. `truck-meshalgo` is read-only (the ledger/CDT internals are consumed,
   never edited).

## Anchors — measured 2026-09-05, counts are exact

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `vendor/truck/truck-modeling/src/facet_sweep.rs` | `ring_is_convex` | 2 |
| A2 | `vendor/truck/truck-meshalgo/Cargo.toml` | `truck-modeling` | 1 |

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
3. `concave_ring_refusals_stay_typed` — a self-intersecting (figure-eight)
   ring still refuses typed; the simplicity check precedes triangulation.

No existing test may be deleted, `#[ignore]`d, or weakened.

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
