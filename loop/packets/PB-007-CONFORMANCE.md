# WORK PACKET PB-007-CONFORMANCE — the bridge conformance battery

You are building the gate for the whole bridge program: the three showcase
scripts run IN PYTHON, byte-equal against the Rust runs of the same tables.
Everything you need is in this document and
`docs/TRUCK123D_PY_BRIDGE_SPEC.md` (§ packet table row PB-007, amended).
Do not read other spec files. If something you need is genuinely missing,
that is a SPEC_GAP (see "Stop conditions"): you stop and report, you do not
research it.

```yaml
id:          PB-007-CONFORMANCE
contract:    [PB-007-CONFORMANCE]
class:       design
crates:      [truck123d]
depends_on:  [PB-005-PYTHON-FACADE, PB-006-ASSEMBLY, PB-008-TTC-HARNESS]
write_allow:
  - truck123d/tests/pb_conformance.rs
  - truck123d/tests/conformance_py/**
read_allow:
  - truck123d/src/
  - truck123d/tests/ttc_harness.rs
  - showcases/src/
  - docs/TRUCK123D_PY_BRIDGE_SPEC.md
tests_required:
  - python_runs_match_rust_runs_byte_equal
  - refusal_battery_mirrors_typed_refusals
  - determinism_across_processes
budget:      {turns: 80, ctx_tokens: 160000}
```

## Problem

The program's gate: the three showcase scripts (amphora, teapot,
waterslide — or their canonical-subset equivalents) executed through the
Python facade must produce report JSON **byte-equal** to the Rust run of
the same table; the typed-refusal battery mirrors as pytest `raises`; and
a determinism test across processes. Amended: the battery also consumes
PB-008's corpus rows for the canonical-only ttc subset.

## Scope decisions — pre-made, do not relitigate

1. **Byte-equality is the gate**: same table + same ordered input → the
   Python run's report JSON is byte-identical to the Rust run's. A single
   differing byte fails.
2. **Refusals mirror as `raises`**: the typed-refusal tests of the landed
   construction battery, expressed as Python exceptions through the
   facade's exception mapping (PB-004's landed vocabulary).
3. **Determinism across processes**: two fresh Python processes, same
   script, byte-identical outputs.
4. **V5, absolute**: landed tests are byte-identical constraints; this
   packet adds only its own test files.
5. **No new tolerance anywhere**: if a byte differs, that is a defect to
   report, never a comparison threshold to widen.

## Anchors — measured 2026-09-06 morning, re-check on your branch

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `truck123d/tests/ttc_harness.rs` | `#[test]` | ≥3 |
| A2 | `showcases/src/teapot.rs` | `fn main` or `pub fn` | ≥1 |
| A3 | `truck123d/tests/` | `pb_conformance.rs` | **absent before, yours after** |

## House rules

- **H-1** no unwrap/expect/panic reachable from geometry; **H-3** same-line
  `// H-3`.
- **Determinism**: the battery IS the determinism gate.
- **All cargo through the queue shim.** Scoped commands only.

## Tests required

1. `python_runs_match_rust_runs_byte_equal` — the three showcase tables
   through both regimes, byte-equal report JSON.
2. `refusal_battery_mirrors_typed_refusals` — each landed typed refusal
   raises the mapped Python exception with the same named cause.
3. `determinism_across_processes` — two fresh processes, byte-identical.

No existing test may be deleted, `#[ignore]`d, or weakened.

## Done when — run these, all must pass

```
cargo fmt --check -p truck123d
cargo clippy -p truck123d --all-targets -- -D warnings
cargo test -p truck123d --test pb_conformance
```

Send cargo output to a file and read the tail.

## Forbidden

Anything outside `write_allow` — especially `truck123d/src/**` (read-only),
`vendor/**`, `Cargo.lock`, any landed test file. Widening a comparison.
Adding `#[ignore]`. Unjustified `#[allow]`. Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- a byte difference you cannot attribute to a named defect → `SPEC_GAP`
  with both sides' outputs
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

```json
{"id":"PB-007-CONFORMANCE","status":"DONE","contracts":["PB-007-CONFORMANCE"],
 "tests_added":3,"anchors_verified":{"A1":3,"A2":1,"A3":1},
 "notes":"byte-equality results per script; refusal mapping census; any API deviation"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit subject: `test(bridge): the conformance battery — Python/Rust byte-equality, refusal mirror, determinism (PB-007-CONFORMANCE)`.
