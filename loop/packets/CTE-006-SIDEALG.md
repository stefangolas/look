# WORK PACKET CTE-006-SIDEALG — the SideState refactor (mechanical)

You are refactoring the coincident-contact decision algebra in the landed
Boolean pipeline to the theory's two-bit carrier algebra. Everything you need
is in this document, `docs/CTE_BUILD_SPINE.md`, and the root theory
`docs/CERTIFIED_TANGENCY_EXACT_CONTACT_SPEC.md` (§5.3–§5.5, §5.9 are
normative). Do not read other spec files. If something you need is genuinely
missing, that is a SPEC_GAP: you stop and report, you do not research it.

```yaml
id:          CTE-006-SIDEALG
contract:    [CTE-006-SIDEALG]
class:       mechanical
crates:      [truck-shapeops]
depends_on:  []
write_allow:
  - vendor/truck/truck-shapeops/src/boolean/mod.rs
read_allow:
  - vendor/truck/truck-shapeops/src/boolean/{split,classify,assemble}.rs
  - vendor/truck/truck-certified/src/tangency/shapes.rs
  - docs/CERTIFIED_TANGENCY_EXACT_CONTACT_SPEC.md
  - docs/CTE_BUILD_SPINE.md
tests_required:
  - side_state_from_material_state4_is_lossless
  - truth_rows_reproduced_by_exhaustion
  - difference_is_not_associative_pin
  - fragment_decision_congruence_after_refactor
budget:      {turns: 60, ctx_tokens: 140000}
```

## Problem

The theory's T2 decision shape is `SideState(u8) ∈ {00,01,10,11}` per
operand, with coordinatewise bitwise algebra (§5.4) and the three-valued
boundary extraction (§5.5). The landed `MaterialState4`/`fragment_decision`
(`boolean/mod.rs:75/:103`) already implement the CORRECT three-valued shape
(`{Keep{flip}, Discard}` — the draft's four-valued `decide` was already
rejected here); this packet factors it into the carrier-relative `SideState`
type the later T2 packet consumes, and pins the algebra by exhaustion. ZERO
new decision logic.

## Scope decisions — pre-made, do not relitigate

1. **Refactor, not redesign.** `fragment_decision`'s verdicts and the
   assembler's behavior are byte-identical after the refactor (the
   congruence test pins this). `SideState` is the carrier-relative
   representation; `MaterialState4` derives from/to it losslessly.
2. **The §5.9 truth rows are a property test, not a table copy**: exhaustion
   over all `4×4×4` (σ_A, σ_B, op) triples reproduces the landed behavior —
   the theory's §9 verification record, re-derived in-tree.
3. **Difference non-associativity is PINNED** (theory §5.7/T2.5): a test
   asserts `(σ_A ∧ ¬σ_B) ∧ ¬σ_C ≠ σ_A ∧ ¬(σ_B ∧ ¬σ_C)` on a witness triple
   — correct behavior, deliberately not "fixed".
4. **KeepBothSplit semantics**: the landed assembler already emits ONE
   canonical face + provenance for coincident pairs (`assemble.rs:594-636`)
   — you add the doc comment pinning this as the §5.5 corollary's
   requirement. No behavior change.
5. **You are `boolean/mod.rs`'s sole writer this program** (CTE-007 owns
   `split.rs`/`assemble.rs`). Do not touch either.

## Anchors — measured this session; re-check on your branch before building

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `truck-shapeops/src/boolean/mod.rs` | `pub fn fragment_decision` | 1 |
| A2 | `truck-shapeops/src/boolean/mod.rs` | `pub struct MaterialState4` | 1 |
| A3 | `truck-shapeops/src/boolean/mod.rs` | `pub enum BoolOp` | 1 |
| A4 | `truck-shapeops/src/boolean/split.rs` | `pub struct CoincidentPair` | 1 |
| A5 | `truck-certified/src/tangency/shapes.rs` | `pub struct SideState` | 1 |

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `todo!`, or out-of-range indexing.
- **H-3** No absolute constants in predicates; test epsilons carry `// H-3`.
- **Determinism**: exhaustion loops in fixed order.
- **All cargo invocations go through the queue (the `cargo` on PATH IS the
  queue shim). Do not invoke cargo by absolute path; do not unset the shim.**
- Never run a bare `cargo test` — use the scoped commands below.

## Tests required

1. `side_state_from_material_state4_is_lossless` — round-trip
   `MaterialState4 ↔ (SideState, SideState)` on all 16 inputs.
2. `truth_rows_reproduced_by_exhaustion` — the §5.9 rows plus the full
   exhaustion; the extractor is `m_R⁻ = m_R⁺ ⇒ Discard, else
   Keep{flip = ¬m_R⁻}`.
3. `difference_is_not_associative_pin` — the §5.7 witness triple.
4. `fragment_decision_congruence_after_refactor` — `fragment_decision`
   pre/post refactor agrees on all 16×4 inputs (the V5-equivalent gate for
   this file; the landed `mod.rs` tests must also pass unchanged).

## Done when — run these, all must pass

```
cargo fmt --check -p truck-shapeops
cargo clippy -p truck-shapeops --all-targets -- -D warnings
cargo test -p truck-shapeops --lib boolean
cargo check -p truck-shapeops
```

Send cargo output to a file and read the tail.

## Forbidden

Editing any file outside `write_allow` — especially `boolean/{split,
classify,assemble}.rs` (CTE-007's write set), `truck-certified/**` beyond
reading `shapes.rs`, any landed test file, `Cargo.lock`. Changing
`fragment_decision`'s verdicts. Adding `#[ignore]`. Adding `#[allow]`
without a justification comment on the same line. Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- the congruence test fails on any input → `SPEC_GAP` (the landed algebra
  and the theory disagree — the loop must see which side is wrong)
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

**COMMIT BEFORE writing `RESULT.json`.**

```json
{"id":"CTE-006-SIDEALG","status":"DONE","contracts":["CTE-006-SIDEALG"],
 "tests_added":4,"anchors_verified":{"A1":1,"A2":1,"A3":1,"A4":1,"A5":1},
 "notes":"any landed-test interaction observed; any API deviation"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`refactor(shapeops): SideState carrier algebra refactor of the fragment decision (CTE-006-SIDEALG)`.
