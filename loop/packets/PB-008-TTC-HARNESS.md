# WORK PACKET PB-008-TTC-HARNESS — the text-to-cad corpus harness

You are building the harness that runs the hardest available external models
through the bridge. Everything you need is in this document and
`docs/TRUCK123D_PY_BRIDGE_SPEC.md` §8 (normative: provenance, measured API
surface, correctness oracle, timing oracle, staged skips, licensing,
boundaries). Do not read other spec files. If something you need is genuinely
missing, that is a SPEC_GAP (see "Stop conditions"): you stop and report, you
do not research it.

```yaml
id:          PB-008-TTC-HARNESS
contract:    [PB-008-TTC-HARNESS]
class:       design
crates:      [truck123d]
depends_on:  [PB-002-SKETCH-ARCS, PB-006-ASSEMBLY]
write_allow:
  - corpus/ttc/**
  - docs/PY_BRIDGE_COMPAT_SURFACE.md
  - truck123d/compat/**
  - truck123d/tests/ttc_harness.rs
read_allow:
  - docs/TRUCK123D_PY_BRIDGE_SPEC.md
  - docs/PY_BRIDGE_CONTRACT.md
  - truck123d/src/
tests_required:
  - corpus_runner_executes_canonical_subset
  - staged_skips_carry_reasons
  - compat_surface_table_is_complete
budget:      {turns: 110, ctx_tokens: 220000}
```

## Problem

`github.com/earthtojake/text-to-cad` (MIT, Thompson Labs LLC) ships the F1
and Falcon-Heavy model trees as build123d scripts — the hardest available
models and the whole point of the bridge program. The harness vendors those
trees, lands the compat-surface document, and runs corpus scripts against
the compat module asserting geometry facts + report JSON + STL.

## Scope decisions — pre-made, do not relitigate

1. **Vendored trees keep upstream headers** + `corpus/ttc/PROVENANCE.md`
   (repo, commit sha, license). Fixtures, not kernel code — nothing under
   `corpus/` is imported by production code.
2. **`docs/PY_BRIDGE_COMPAT_SURFACE.md`** is the build123d-compat vocabulary
   the corpus exercises (spec §8's measured table), DISTINCT from PB-000's
   kernel-facing API table.
3. **STL, never STEP, for swept parts** (TR-NRB-001 boundary).
4. **Staged skips** (`corpus/ttc/SKIPS.json`): every script not runnable at
   the current program stage carries a machine-checked reason
   (`booleans-on-swept-carriers: resolved by BIE-006` where applicable). A
   skip without a reason row is a harness failure. The skip list is an
   OUTPUT of program progress, never a threshold to tune.
5. **Per-script timing in three regimes** (OCC baseline, our drop-in, native
   facade) recorded but NOT published — fresh-process only, physical-GPU
   machine for claims (BENCHMARKS doctrine). This machine's numbers are
   local evidence.
6. **Never reimplement `cadgen`'s daemon/store** — the runner is a plain
   process-per-script door.
7. **The BIE battery owns sweep-pair certification; the harness consumes
   it.** Boolean-heavy scripts that still refuse carry typed reasons.

## Anchors — measured 2026-09-06 morning, re-check on your branch

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `truck123d/src/lib.rs` | `pub fn` | ≥1 |
| A2 | `docs/PY_BRIDGE_CONTRACT.md` | `## ` | ≥3 |
| A3 | `corpus/ttc/` | directory | **absent before, yours after** |

## House rules

- **H-1** no unwrap/expect/panic reachable from geometry; **H-3** same-line
  `// H-3`; **H-6** never record `Float` as `Exact`.
- **Determinism**: identical ordered input → identical reports; per-script
  order fixed.
- **All cargo through the queue shim.** Scoped commands only.

## Tests required

1. `corpus_runner_executes_canonical_subset` — the canonical-only scripts
   run end-to-end producing report JSON + STL; geometry facts assert.
2. `staged_skips_carry_reasons` — every skip row carries a machine-checked
   reason; a reasonless skip fails the harness.
3. `compat_surface_table_is_complete` — every API surface row the corpus
   exercises (spec §8's table) appears in
   `docs/PY_BRIDGE_COMPAT_SURFACE.md` with its landed status.

## Done when — run these, all must pass

```
cargo fmt --check -p truck123d
cargo clippy -p truck123d --all-targets -- -D warnings
cargo test -p truck123d --test ttc_harness
```

Send cargo output to a file and read the tail.

## Forbidden

Anything outside `write_allow` — especially `vendor/**`, PB-004's landed
core, `Cargo.lock`, any landed test file. Publishing timing claims from this
machine. Adding `#[ignore]`. Unjustified `#[allow]`. Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- the upstream corpus trees cannot be fetched or their license headers are
  missing → `SPEC_GAP`, naming what is absent
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

```json
{"id":"PB-008-TTC-HARNESS","status":"DONE","contracts":["PB-008-TTC-HARNESS"],
 "tests_added":3,"anchors_verified":{"A1":1,"A2":3,"A3":1},
 "notes":"corpus subset executed vs skipped (with reasons); F1/Falcon-Heavy reach statement; any API deviation"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit subject: `feat(bridge): text-to-cad corpus harness — vendored trees, compat surface, staged skips (PB-008-TTC-HARNESS)`.
