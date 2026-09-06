# WORK PACKET CFP-002-INSTRUMENT — the three counters that price the program

You are landing the measurement hooks for the Contact Fast-Path program. The
build spec deliberately claims NO composed end-to-end number; this packet
produces the one datum (`f`, the stage-5 plane/quadric×spline share) that
fills it, plus the two inputs the sizing of CFP-003/CFP-005 rest on.
Everything you need is in this document and
`docs/CONTACT_FAST_PATH_BUILD_SPEC.md` §2 (instrument schema) and §6. Do not
read other spec files. SPEC_GAP discipline applies.

```yaml
id:          CFP-002-INSTRUMENT
contract:    [CFP-002-INSTRUMENT]
class:       mechanical
crates:      [truck-evidence, truck-certified]
depends_on:  [CFP-000-SPINE]
write_allow:
  - vendor/truck/truck-evidence/src/contact/mod.rs
  - vendor/truck/truck-evidence/src/contact/instrument.rs
  - vendor/truck/truck-certified/src/patch_admit.rs
  - vendor/truck/truck-certified/src/cfp/instrument.rs
read_allow:
  - docs/CONTACT_FAST_PATH_BUILD_SPEC.md
  - vendor/truck/truck-certified/src/cfp/spine.rs
  - vendor/truck/truck-certified/src/cfp/fixtures.rs
  - vendor/truck/truck-evidence/src/contact/mod.rs
  - vendor/truck/truck-certified/src/patch_admit.rs
tests_required:
  - counter_increment_stage5_pair_class
  - counter_increment_knot_span_distribution
  - counter_increment_composed_bidegree
  - instrument_output_json_key_order_matches_spine
  - counters_are_zero_on_fresh_process
budget:      {turns: 60, ctx_tokens: 140000}
```

## Problem

The CFP spec's §6 leaves the composed end-to-end speedup unfilled pending one
number: `f`, the share of stage-5 entries that are plane×spline or
quadric×spline pairs (the class CFP-004 removes from the 4-D machinery). Two
further numbers gate CFP-005's sizing (knot-span count distribution on
admitted carriers) and the D3 budget revisit (composed-degree distribution
against `MAX_BIDEGREE`). All three are cheap hooks on paths that already
run; nothing here changes any verdict.

## Scope decisions — pre-made, do not relitigate

1. **Counters are per-crate, schema is shared.** F1 forbids a shared
   implementation, so each crate lands its own counter struct with the field
   names and JSON key order frozen at the spine
   (`cfp/spine.rs::InstrumentCounters`): `knot_span_count`,
   `stage5_pair_class`, `composed_bidegree`, `cells_visited`,
   `distinct_side_boxes`, `unresolved_provenance`. This packet implements
   `contact/instrument.rs` (evidence) and `cfp/instrument.rs` (certified)
   with identical schemas; the spine's
   `instrument_output_json_key_order_matches_spine` pattern is the template.
2. **Gated recording, zero overhead when off**: counters increment only when
   enabled (a process-global `AtomicBool`, default OFF; enabled by the
   `TRUCK_CFP_INSTRUMENT` env var read ONCE). When off, the hooks are one
   atomic load + branch — no allocation, no lock, no change to any verdict.
   This packet must not perturb the hot path measurably or semantically.
3. **The three hooks, exactly:**
   - `patch_admit::admit_surface` — per admitted carrier: record the knot-span
     count (`knot_span_count`) and the admitted bidegree
     (`composed_bidegree`) against `MAX_BIDEGREE`.
   - `contact::contact` — at the stage-4→stage-5 boundary (the dispatch that
     would reach the general validated FF / spline SSI arm): record the
     carrier-class pair (`stage5_pair_class`), keyed by the recognized class
     of each side (`Plane|Cylinder|Sphere|Cone|Torus|Spline|Sweep|Other`).
   - No other hook exists in this packet. `cells_visited`/
     `distinct_side_boxes`/`unresolved_provenance` land post-CFP-001 via the
     same structs (their increment sites do not exist yet — do not stub them
     with fake increments; declare the fields, leave them at zero).
4. **No verdict changes**: identical ordered input → identical verdicts with
   counters on or off. The gate `counters_are_zero_on_fresh_process` plus a
   determinism run guard this.
5. **Zero new top-level evidence kinds.** Counters are process-local
   diagnostics, never evidence; they are not serialized into any
   `Certificate`.

## Anchors — base-relative, drift-tolerant

Re-check on your branch:

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `truck-evidence/src/contact/mod.rs` | `pub fn contact` | 1 |
| A2 | `truck-evidence/src/contact/mod.rs` | `mod instrument` | **0 before, 1 after your change** |
| A3 | `truck-certified/src/patch_admit.rs` | `pub fn admit_surface` | 1 |
| A4 | `truck-certified/src/cfp/spine.rs` | `InstrumentCounters` | 1 (read-only spine anchor) |
| A5 | `truck-certified/src/cfp/instrument.rs` | anything | **0 before, 1 after** (new file) |

A2/A5 are packet-owned deltas. If present at base, another writer landed
first: STOP, `ANCHOR_MISMATCH`.

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `todo!` reachable from geometry.
- **Determinism**: counters ON or OFF must not change any verdict; no output
  ordering from hash iteration (pair-class keys are a fixed enum order).
- **All cargo invocations go through the queue (the `cargo` on PATH IS the
  queue shim).**

## Tests required

Named `#[test]` fns; the verifier checks the names appear in your diff:

1. `counter_increment_stage5_pair_class` — a synthetic pair through the
   stage boundary increments `stage5_pair_class` with the right class key.
2. `counter_increment_knot_span_distribution` — a multi-span admit records
   the span count.
3. `counter_increment_composed_bidegree` — the admitted bidegree is recorded
   against `MAX_BIDEGREE`.
4. `instrument_output_json_key_order_matches_spine` — per-crate output
   serializes in the spine's frozen key order.
5. `counters_are_zero_on_fresh_process` — with the env var unset, all fields
   are zero after a full contact/admit pass.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-evidence -p truck-certified
cargo clippy -p truck-evidence -p truck-certified --all-targets -- -D warnings
cargo test -p truck-evidence --lib contact::instrument
cargo test -p truck-certified --lib cfp::instrument
```

Send cargo output to a file and read the tail.

## Forbidden

Editing any file outside `write_allow` — especially `ssi.rs`, `tangency/**`,
`construct/**`, `enclosure.rs`, anything under `truck-shapeops/`,
`Cargo.lock`. Changing any verdict, dispatch order, or admission gate.
Emitting counters into evidence. Adding `#[ignore]`. Adding `#[allow]`
without a justification comment on the same line. Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- a counter hook cannot be placed without changing dispatch order → SPEC_GAP
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

**COMMIT BEFORE writing `RESULT.json`.** Then write `RESULT.json` at the root
of your worktree.

```json
{"id":"CFP-002-INSTRUMENT","status":"DONE","contracts":["CFP-002-INSTRUMENT"],
 "tests_added":5,"anchors_verified":{"A1":1,"A2":1,"A3":1,"A4":1,"A5":1},
 "notes":"three hooks landed, schema per spine; any deviation stated here"}
```
