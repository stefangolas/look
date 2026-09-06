# WORK PACKET CL-002-SPLINE-ASSEMBLY — cross-patch branch stitching

You are stitching per-patch intersection branches into whole carried curves.
Everything you need is in this document and
`docs/CARRIER_LIFT_BUILD_SPEC.md` (row CL-002, §4 gates). Do not read other
spec files. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```yaml
id:          CL-002-SPLINE-ASSEMBLY
contract:    [CL-002-SPLINE-ASSEMBLY]
class:       mechanical
crates:      [truck-certified, truck-geometry]
depends_on:  [CL-001-SPLINE-LIFT]
write_allow:
  - vendor/truck/truck-certified/src/ssi_trace.rs
  - vendor/truck/truck-geometry/src/constructive/intersection_carrier.rs
read_allow:
  - vendor/truck/truck-certified/src/ssi.rs
  - vendor/truck/truck-certified/src/ssi_types.rs
  - vendor/truck/truck-certified/src/tangency/
  - vendor/truck/truck-geometry/src/constructive/intersection_carrier.rs
  - docs/CARRIER_LIFT_BUILD_SPEC.md
tests_required:
  - shared_edge_samples_agree_within_enclosure
  - stitched_branch_rides_carrier
  - unstitchable_pair_refuses_typed
budget:      {turns: 80, ctx_tokens: 160000}
```

**Additive constructor** in `intersection_carrier.rs`: H-1 applies — no
`unwrap_used` without a justified same-line opt-out.

## Problem

The spline×analytic dispatch (CL-001, landed) certifies intersection
branches PER PATCH — a multi-patch spline carrier produces branch fragments
that must stitch into one carried curve where patches share an edge. The
stitch certificate asserts the shared-edge samples agree (interval
bookkeeping); the stitched branch rides the landed
`CertifiedImplicitIntersectionCurve` carrier.

## Scope decisions — pre-made, do not relitigate

1. **The stitch predicate is interval agreement on shared edges**: for two
   patches meeting along an edge, the per-patch branch samples restricted
   to the shared edge agree within their certified enclosures — a
   bookkeeping assertion over already-certified data, never a new solve.
2. **The carrier is additive**: a new constructor on the landed
   `CertifiedImplicitIntersectionCurve` taking stitched fragments; the
   landed carrier type and its consumers are untouched (V5).
3. **`ssi_trace.rs` is yours this cycle**: extend the trace outcome to
   carry stitched branches; `ssi.rs`/`ssi_types.rs` stay read-only.
4. **Typed outcomes only**: a pair whose shared-edge samples disagree
   beyond enclosures refuses typed (`Unresolved` with the witness) — zero
   new `Refusal` arms.

## Anchors — measured 2026-09-06 morning, re-check on your branch

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `truck-certified/src/ssi_trace.rs` | `pub fn certified_pair_trace` | 1 |
| A2 | `truck-geometry/src/constructive/intersection_carrier.rs` | `pub struct CertifiedImplicitIntersectionCurve` | 1 |
| A3 | `truck-certified/src/ssi_types.rs` | `pub struct TraceStep` | 1 |

## House rules

- **H-1** no unwrap/expect/panic reachable from geometry; **H-3** same-line
  `// H-3`; **H-6** never record `Float` as `Exact`.
- **Determinism**: fixed fragment order; no hash ordering in output.
- **All cargo through the queue shim.** Scoped commands only.

## Tests required

1. `shared_edge_samples_agree_within_enclosure` — two adjacent admitted
   patches whose branches share an edge: the stitch predicate certifies on
   the shared edge's enclosure.
2. `stitched_branch_rides_carrier` — the stitched branch constructs the
   landed carrier with the fragment provenance intact.
3. `unstitchable_pair_refuses_typed` — a mismatched pair refuses typed
   with the witness, never a silent gap.

No existing test may be deleted, `#[ignore]`d, or weakened.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-certified -p truck-geometry
cargo clippy -p truck-certified -p truck-geometry --all-targets -- -D warnings
cargo test -p truck-certified --lib ssi
cargo test -p truck-geometry --lib intersection
```

Send cargo output to a file and read the tail.

## Forbidden

Anything outside `write_allow` — especially `ssi.rs`, `tangency/**`
(additive read), `construct/bie/**`, any landed test file, `Cargo.lock`.
Adding `#[ignore]`. Unjustified `#[allow]`. Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- no admissible stitch fixture can be built from the landed admission
  layer → `SPEC_GAP`, naming the gap
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

```json
{"id":"CL-002-SPLINE-ASSEMBLY","status":"DONE","contracts":["CL-002-SPLINE-ASSEMBLY"],
 "tests_added":3,"anchors_verified":{"A1":1,"A2":1,"A3":1},
 "notes":"the stitch predicate's enclosure discipline; fragment provenance shape; any API deviation"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit subject: `feat(certified): cross-patch branch stitching onto the implicit intersection carrier (CL-002-SPLINE-ASSEMBLY)`.
