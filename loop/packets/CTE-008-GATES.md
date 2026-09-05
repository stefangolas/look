# WORK PACKET CTE-008-GATES — the program battery

You are closing the Certified Tangency and Exact Contact (CTE) program with
its battery. Everything you need is in this document,
`docs/CTE_BUILD_SPINE.md`, and the root theory
`docs/CERTIFIED_TANGENCY_EXACT_CONTACT_SPEC.md`. Do not read other spec
files. If something you need is genuinely missing, that is a SPEC_GAP: you
stop and report, you do not research it.

```yaml
id:          CTE-008-GATES
contract:    [CTE-008-GATES]
class:       mechanical
crates:      [truck-certified, truck-shapeops]
depends_on:  [CTE-007-T2ARRANGE]
write_allow:
  - vendor/truck/truck-certified/src/tangency/gates.rs
  - vendor/truck/truck-certified/src/tangency/mod.rs
  - vendor/truck/truck-certified/tests/cte_battery.rs
  - vendor/truck/truck-shapeops/tests/cte_gates.rs
read_allow:
  - vendor/truck/truck-certified/src/tangency/
  - vendor/truck/truck-shapeops/src/boolean/
  - vendor/truck/truck-shapeops/tests/boolean_m2.rs
  - vendor/truck/truck-shapeops/tests/conformance_battery.rs
  - docs/CERTIFIED_TANGENCY_EXACT_CONTACT_SPEC.md
  - docs/CTE_BUILD_SPINE.md
tests_required:
  - differential_congruence_with_boolean_m2
  - termination_battery_t18
  - a1_node_battery_is_not_unresolved
  - t14_never_evaluated_at_runtime_gate
  - determinism_gate_field_for_field
budget:      {turns: 80, ctx_tokens: 180000}
```

## Problem

The one-verify amendment's program battery: differential congruence against
the landed Boolean results, the T1.8 termination battery, the R3 node
regression gate, the R1 no-runtime-T1.4 gate, and the determinism gate. New
test files ONLY — every landed test name is a byte-identical constraint.

## Scope decisions — pre-made, do not relitigate

1. **Differential congruence**: on the landed `boolean_m2` fixture family
   (read-only — copy the fixture CONSTRUCTION code, do not import the test
   module), the CTE path agrees face-for-face with the landed results
   (V5-equivalent: zero regressions on canonical pairs).
2. **T1.8 termination battery** (theory §2.11): every fixture (F1–F6)
   terminates on one of the first four verdicts within the booked budget;
   the battery RECORDS the refinement depth per fixture.
3. **A₁⁻ node regression** (theory R3): F4 asserts `A1Node` — the gate that
   fails if a later change silently degrades the indefinite case to
   `Unresolved`.
4. **R1 gate**: no runtime call site evaluates the T1.4 identity — assert
   via the crate's public API surface (no such function is exported) plus a
   grep-shaped review note in RESULT.
5. **Determinism gate**: identical ordered input → field-for-field identical
   verdicts, twice, on every fixture.
6. **No golden images, no recorded performance claims.** If a congruence
   mismatch appears, the DEFAULT is that CTE is wrong and the landed path
   is right — report the mismatch, do not reconcile silently.

## Anchors — measured this session; re-check on your branch before building

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `truck-certified/src/tangency/cascade.rs` | `pub fn classify_box` | 1 |
| A2 | `truck-certified/src/tangency/arrange.rs` | `pub fn arrange_carrier` | 1 |
| A3 | `truck-shapeops/tests/boolean_m2.rs` | `fn m2_self_pair_refuses_the_typed_envelope` | 1 |
| A4 | `truck-certified/src/tangency/shapes.rs` | `pub enum ContactVerdict` | 1 |
| A5 | `vendor/truck/truck-certified/src/tangency/fixtures.rs` | `pub fn` | ≥7 |

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `todo!`.
- **H-3** No absolute constants in predicates; test epsilons carry `// H-3`
  on the SAME line as the literal.
- **All cargo invocations go through the queue (the `cargo` on PATH IS the
  queue shim). Do not invoke cargo by absolute path; do not unset the shim.**
- Never run a bare `cargo test` — use the scoped commands below.

## Tests required

The five named tests above, in the two new test files
(`truck-certified/tests/cte_battery.rs`, `truck-shapeops/tests/cte_gates.rs`).

## Done when — run these, all must pass

```
cargo fmt --check -p truck-certified -p truck-shapeops
cargo clippy -p truck-certified -p truck-shapeops --all-targets -- -D warnings
cargo test -p truck-certified --lib tangency
cargo test -p truck-certified --test cte_battery
cargo test -p truck-shapeops --test cte_gates
cargo test -p truck-shapeops --test boolean_m2
cargo test -p truck-shapeops --test conformance_battery
```

Send cargo output to a file and read the tail.

## Forbidden

Editing any landed test file or any `write_allow`-outside file. Updating
golden/baseline data to make a gate pass. Weakening any landed test. Adding
`#[ignore]`. Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- a differential mismatch you cannot attribute to a CTE bug with a stated
  mechanism → `SPEC_GAP` with both sides' outputs
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

**COMMIT BEFORE writing `RESULT.json`.**

```json
{"id":"CTE-008-GATES","status":"DONE","contracts":["CTE-008-GATES"],
 "tests_added":5,"anchors_verified":{"A1":1,"A2":1,"A3":1,"A4":1,"A5":7},
 "notes":"termination depths per fixture; differential results per fixture family; any mismatch with attribution"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`test(certified): CTE program battery — differential, termination, node, determinism gates (CTE-008-GATES)`.
