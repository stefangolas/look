# WORK PACKET BIE-006-CLASSIFY — sweep lift/path adapters + windowed sweep output

You are implementing the pipeline-tie-in packet of the Certified Interaction
Engine (BIE) program — the one that lets a `SpineFrameSweep` face through
the landed boolean funnel. Everything you need is in this document and
`docs/BIE_BUILD_SPINE.md`. Do not read other spec files. If something you
need is genuinely missing, that is a SPEC_GAP (see "Stop conditions"): you
stop and report, you do not research it.

```yaml
id:          BIE-006-CLASSIFY
contract:    [BIE-006-CLASSIFY]
class:       design
crates:      [truck-evidence, truck-shapeops]
depends_on:  [BIE-003-CARRIER, BIE-005-ARRANGE]
write_allow:
  - vendor/truck/truck-evidence/src/contact/mod.rs
  - vendor/truck/truck-shapeops/src/boolean/mod.rs
  - vendor/truck/truck-shapeops/src/boolean/assemble.rs
  - vendor/truck/truck-shapeops/src/boolean/sweep_lift.rs
read_allow:
  - vendor/truck/truck-geometry/src/constructive/intersection_carrier.rs
  - vendor/truck/truck-geometry/src/arrange.rs
  - vendor/truck/truck-geometry/src/constructive/sweep_surface.rs
  - vendor/truck/truck-topology/src/entity_id.rs
  - vendor/truck/truck-shapeops/src/boolean/classify.rs
  - docs/BIE_BUILD_SPINE.md
tests_required:
  - sweep_stratum_lifts_with_window
  - sweep_pairs_reach_interaction_solver
  - classifier_seeds_from_arrangement_cells
  - assembler_emits_windowed_sweep_faces
  - provenance_records_sweep_fragments
budget:      {turns: 80, ctx_tokens: 200000}
```

**New file** (`boolean/sweep_lift.rs`): H-1 applies — no `unwrap_used`
without a justified same-line opt-out.

**This is the program's HOT-FILE packet**: `contact/mod.rs` and
`truck-shapeops/src/boolean/*` are the most-contended files in the kernel.
You are the program's ONLY writer of them this wave (spine §4). Every edit
additive; every landed behavior byte-stable.

## Problem

The landed LIFT stage refuses swept faces today: the recognizer's
`CanonicalCarrierWitness::Unrecognized` arm returns
`NonCanonicalCarrier` (the gate that blocks sweeps — contact/mod.rs:367).
This packet adds `BoundedStratum::Sweep`, the lift adapter that recognizes
`SpineFrameSweep` faces, the CONTACT dispatch to the BIE-002 restricted
solver, classifier seeds from BIE-005's arrangement cells, windowed sweep
output faces in ASSEMBLE, and provenance rows.

## Scope decisions — pre-made, do not relitigate

1. **CLASSIFY and DECIDE are reused VERBATIM** (the booking's largest
   downward correction): `classify_fragments` (seed-and-propagate over the
   parity graph) and `fragment_decision` are carrier-agnostic. You add
   SEEDS from arrangement cells — the classifier logic itself is not
   edited. If you find yourself editing `classify.rs`'s propagation logic,
   STOP: that is a SPEC_GAP, not a task.
2. **`BoundedStratum::Sweep { recipe, window }`** joins the landed enum
   (contact/mod.rs:87) carrying the whole-sweep recipe reference and the
   windowed domain — the same window the landed `SpineFrameSweep` closed
   value already carries (spec 5.10 normative deviation).
3. **Output faces are windowed `SpineFrameSweep`s** — the type already
   carries windowed domains (`sweep_surface.rs:52`); ASSEMBLE emits them
   directly, no new surface type. Edges carry BIE-003's
   `CertifiedImplicitIntersectionCurve` where the boundary came from the
   interaction.
4. **Provenance** (§8.3): output fragments record `EntityId`/`Op` rows —
   the landed `entity_id.rs` vocabulary (`pub enum EntityId`/:145,
   `pub enum OpKind`/:249); propagation rules are thin: each output
   fragment's Op cites the input strata and the boolean op.
5. **V5 identity guard, absolute**: the canonical × canonical path is
   bit-identical. Every landed test in `truck-shapeops` and every landed
   assertion in `contact/mod.rs` passes unchanged. The new arm is additive:
   `Unrecognized` still refuses exactly what it refused before; sweeps stop
   refusing because they now lift into the new variant, not because the
   refusal changed.

## Anchors — measured 2026-09-05, counts are exact

Locate by pattern, never by line number. If a count differs, STOP and report
`ANCHOR_MISMATCH` with what you saw.

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `vendor/truck/truck-evidence/src/contact/mod.rs` | `pub enum BoundedStratum` | 1 |
| A2 | `vendor/truck/truck-evidence/src/contact/mod.rs` | `CanonicalCarrierWitness::Unrecognized =>` | 1 |
| A3 | `vendor/truck/truck-shapeops/src/boolean/classify.rs` | `pub fn classify_fragments` | 1 |
| A4 | `vendor/truck/truck-shapeops/src/boolean/mod.rs` | `pub fn fragment_decision` | 1 |
| A5 | `vendor/truck/truck-shapeops/src/boolean/assemble.rs` | `pub fn boolean\(` | 1 |
| A6 | `vendor/truck/truck-topology/src/entity_id.rs` | `pub enum EntityId` | 1 |

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `unimplemented!`, `todo!`, or
  out-of-range indexing reachable from geometry.
- **H-2** Fallible operations return `Outcome<T>`.
- **H-3** No absolute constants in predicates; test epsilons carry `// H-3`
  on the SAME line as the literal.
- **H-6** Float-computed values are never recorded as `Method::Exact`.
- **Determinism** (spine §8): identical ordered input → identical verdicts.
- **All cargo invocations go through the queue (the `cargo` on PATH IS the
  queue shim). Do not invoke cargo by absolute path; do not unset the shim.**
- Never run a bare `cargo test` — use the scoped commands below.

## Tests required

Named `#[test]` fns (in-module test sections) — the verifier checks the
names appear in your diff. Fixtures: the teapot-junction-class pair (a
straight-spine `SpineFrameSweep` × a canonical plane/sphere from the
BIE-000 kit recipes) plus canonical-only controls.

1. `sweep_stratum_lifts_with_window` — a sweep face lifts to
   `BoundedStratum::Sweep` with the correct window; the `Unrecognized`
   refusal still fires for a genuinely non-canonical face (both arms
   asserted).
2. `sweep_pairs_reach_interaction_solver` — a sweep×canonical pair
   dispatches to the restricted solver path and returns a certified or
   typed-`Unresolved` outcome — never the old `NonCanonicalCarrier`.
3. `classifier_seeds_from_arrangement_cells` — the classifier's inside/out
   bits for sweep fragments agree with the arrangement-cell seeds (and the
   parity propagation is unchanged — assert a canonical-only control case
   gives the landed answer).
4. `assembler_emits_windowed_sweep_faces` — a union/difference over the
   junction fixture emits output faces that are windowed `SpineFrameSweep`s
   with the expected window bounds (`// H-3` tolerance).
5. `provenance_records_sweep_fragments` — output fragments carry
   `EntityId`/`Op` rows citing the input strata.

No existing test may be deleted, `#[ignore]`d, or weakened. The landed
`truck-shapeops/tests/boolean_m2.rs` results must be unchanged (V5).

## Done when — run these, all must pass

```
cargo fmt --check -p truck-evidence -p truck-shapeops
cargo clippy -p truck-evidence -p truck-shapeops --all-targets -- -D warnings
cargo test -p truck-shapeops --lib --tests
cargo test -p truck-evidence --lib contact
cargo check -p truck-certified
```

Send cargo output to a file and read the tail. If any LANDED test fails
identically before and after your change, verify it fails at your fork
point too (throwaway worktree) and record it — do not fix landed tests.

## Forbidden

Editing any file outside `write_allow` — especially
`boolean/classify.rs`, `boolean/split.rs`, anything under
`truck-geometry/` or `truck-certified/`, any landed test file,
`scripts/kernel-gates.sh`, `Cargo.lock`. Changing classifier propagation
or decision logic. Adding `#[ignore]`. Adding `#[allow]` without a
justification comment on the same line. Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- a landed test fails and you cannot attribute it to your diff → stop,
  record the test name and both runs, `BLOCKED`
- the lift genuinely needs a canonical variant not present (BIE-003 not
  landed under this fork) → `BLOCKED` with the missing type named
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

**COMMIT BEFORE writing `RESULT.json`.** Then write `RESULT.json` at the root
of your worktree (not `loop/results/` — the orchestrator files it there).

```json
{"id":"BIE-006-CLASSIFY","status":"DONE","contracts":["BIE-006-CLASSIFY"],
 "tests_added":5,"anchors_verified":{"A1":1,"A2":1,"A3":1,"A4":1,"A5":1,"A6":1},
 "notes":"the certified/unresolved split you observed on the junction fixture, and any landed test that failed at fork point"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(shapeops): sweep lift/path adapters + windowed sweep output + provenance (BIE-006-CLASSIFY)`.
