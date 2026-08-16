# WORK PACKET BG-NUM-001-FILLET — the rolling-ball fillet spends a real budget

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```yaml
id:          BG-NUM-001-FILLET
contract:    [BG-NUM-001]
class:       mechanical
crates:      [truck-geometry]
depends_on:  []
write_allow:
  - vendor/truck/truck-geometry/src/decorators/af_surface.rs
  - vendor/truck/truck-geometry/tests/af_surface.rs
read_allow:
  - vendor/truck/truck-base/src/evidence.rs
tests_required:
  - refinement_spends_subdivision_budget
  - exhausted_budget_refuses_with_what_was_spent
  - budget_refusal_reports_the_counter_that_ran_out
  - sufficient_budget_reaches_the_same_surface_as_before
budget:      {turns: 40, ctx_tokens: 100000}
```

## Problem

`approx_rolling_ball_fillet` refines its contact curves in a bare
`for _i in 0..16`. Sixteen is not a budget — it is a constant chosen once, with
no relationship to the input, no record of what was spent, and no way for a
caller to ask for more or less. That violates house rule **H-5: budget or
bound, never a bare loop**, and it is the specific loop the specification names
when it defines this contract.

When the loop runs out it does not say so. It falls through with whatever it had
after sixteen passes, and the caller cannot distinguish "converged in four" from
"gave up at sixteen". BG-S0-002 left five `TODO(BG-NUM-001)` markers in this
file for exactly this reason: it had to construct `Budget::new(0, 0, 0)` at
refusal sites because no real budget was threaded through to report.

## Anchors — verified 2026-08-16, counts are exact

Locate by running the `rg` command. **Never locate by line number.**
**If a count differs, STOP** and report `ANCHOR_MISMATCH` with what you saw.
All in `vendor/truck/truck-geometry/src/decorators/af_surface.rs`.

| # | enclosing symbol | `rg` pattern | expect |
|---|---|---|---|
| A1 | `fn approx_rolling_ball_fillet` | `for _i in 0\.\.16` | **1** |
| A2 | (file) | `TODO\(BG-NUM-001\): thread the real budget` | **5** |
| A3 | (file) | `Budget::new\(0, 0, 0\)` | **6** |

A2 and A3 do **not** pair one-to-one, and the difference matters. Five of the
six placeholders are `spent:` at refusal sites, each with a TODO above it. The
sixth, at the `Certificate` on the **success** path, is `budget_left:` — a
different quantity: not what was consumed but what remains. It carries no TODO
because BG-S0-002 never touched it.

Fix all six. The five `spent:` sites report what was consumed; the one
`budget_left:` site reports the budget still remaining when the fillet
succeeded, which is the honest value and is what a caller composing further
operations needs. Reporting zero remaining after a successful call would tell
every downstream operation it has nothing left to spend. After this packet the
file contains **zero** `Budget::new(0, 0, 0)` and **zero** TODOs.

## The budget API you are threading

From `truck_base::evidence` — **not** `truck_evidence`. Already landed; do not
add to it, do not modify it.

```rust
pub struct Budget { pub subdiv: u32, pub newton: u32, pub depth: u32 }

impl Budget {
    pub fn new(subdiv: u32, newton: u32, depth: u32) -> Self;
    /// Err ⇒ the caller returns NumericallyUnresolved carrying what was spent.
    pub fn spend_subdiv(&mut self, n: u32) -> Result<(), Exhausted>;
    pub fn spend_newton(&mut self, n: u32) -> Result<(), Exhausted>;
    pub fn spend_depth(&mut self) -> Result<(), Exhausted>;
}

pub struct Exhausted { pub counter: BudgetCounter }
```

Read the exact shapes in `evidence.rs` before you use them — the constructor and
`BudgetCounter`'s variants are what is actually there, not what this sketch
implies.

## The design — decided; implement it, do not re-litigate

**1. `approx_rolling_ball_fillet` takes `&mut Budget`.** It becomes the last
parameter. Every caller inside `write_allow` passes one through; a caller that
has no budget of its own constructs one at its entry point rather than defaulting
silently, and the value it constructs is stated in a comment.

**2. The refinement loop spends before each pass.**

```rust
while budget.spend_subdiv(1).is_ok() {
    // ... the existing body, unchanged ...
    if converged { break; }
}
```

The loop is bounded by the budget rather than by 16, so a caller that hands it a
`subdiv` of 16 gets exactly today's behaviour. **That is the migration's safety
property**: identical results for identical effort, with the effort now named and
accountable.

**3. Exhaustion is a refusal, not a fallthrough.** When the budget runs out
before convergence, return
`Refusal::NumericallyUnresolved { spent, witness }` where `spent` is the budget
actually consumed — not `Budget::new(0, 0, 0)` — and the witness is the existing
`UnresolvedWitness::ContactCurveNotFound`. Do not invent a new witness variant;
if none of the existing ones fits a site, that is a `SPEC_GAP`.

**4. All six placeholder sites get the real thing.** Each
`TODO(BG-NUM-001)` / `Budget::new(0, 0, 0)` pair is replaced by the budget in
scope at that point, reporting what was genuinely spent. Delete the TODO comment
with it — a TODO left beside a fix is a lie to the next reader.

**Threading `spent` correctly is the one judgement here.** `Budget` is `Copy`,
so a refusal site needs the *difference* between what it started with and what
remains, not the remaining budget. Say in your `RESULT.json` notes how you
computed it.

## Template — the reference answer

`vendor/truck/truck-shapeops/src/fillet/mod.rs` holds BG-S0-002's landed diff:
`unwrap()` converted to `?` over `Outcome`, with refusals carrying witnesses.
Same file's `create_pcurve_edge` and `simple_fillet` show the house style for
signature changes that ripple to call sites. Match it.

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `unimplemented!`, `todo!`, or
  out-of-range indexing reachable from geometry.
- **H-2** Fallible operations return `Outcome<T>` — never `Option`, never a
  bare `Result`.
- **H-3** No absolute constants in predicates. **`scripts/kernel-gates.sh` flags
  a bare float literal on any added line, and test epsilons trip it. The opt-out
  is a `// H-3` comment ON THE SAME LINE as the literal** — not the line above,
  which does not work. Use it on float comparisons in your tests, naming the
  quantity.
- **H-5** Budget or bound, never a bare loop. This item is that rule.
- **H-6** A value computed in floats is never recorded as `Method::Exact`.

## Tests required

Each must be a named `#[test]` fn — the verifier checks the names appear in your
diff, so a test you describe but do not write fails the gate.

1. `refinement_spends_subdivision_budget` — after a successful call, the
   budget's `subdiv` has strictly decreased. The loop is genuinely drawing on it.
2. `exhausted_budget_refuses_with_what_was_spent` — a deliberately tiny budget
   (`subdiv: 1`) returns `NumericallyUnresolved`, and the `spent` it reports is
   non-zero and no larger than what it was given.
3. `budget_refusal_reports_the_counter_that_ran_out` — the refusal identifies
   the subdivision counter, not some other one.
4. `sufficient_budget_reaches_the_same_surface_as_before` — with `subdiv: 16`,
   the result equals what the unbudgeted loop produced. **This is the
   regression guard**: the migration must be behaviour-preserving at equal
   effort. Compare the surface, not just that it is `Ok`.

No existing test may be deleted, `#[ignore]`d, or weakened.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-geometry
cargo clippy -p truck-geometry --all-targets --no-deps -- -D warnings
cargo test -p truck-geometry --lib --tests
cargo check --workspace --all-targets
```

`cargo clippy -p truck-geometry` currently fails on a pre-existing
`items after a test module` error in `src/decorators/revolved_curve.rs`, which
is **not yours** — it is outside your allowlist and predates this packet. If it
is the only clippy failure, that is expected; say so in `RESULT.json` and do not
attempt to fix it. Any clippy finding in `af_surface.rs` **is** yours.

Never run a bare `cargo test` — it builds 56 examples. Send cargo output to a
file and read the tail.

## Forbidden

Editing any file outside `write_allow` — especially `truck-base/src/evidence.rs`
(another packet owns it), `revolved_curve.rs`, and anything under `loop/` (your
result file goes in the worktree root, nowhere else). Adding a new
`UnresolvedWitness` variant. Changing `Budget`'s API. Raising the loop bound to
"fix" a failing test. Adding `#[ignore]`. Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`
- a refusal site has no existing witness variant that fits → `SPEC_GAP`, naming
  the site and the variants you considered
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

```json
{"id":"BG-NUM-001-FILLET","status":"DONE","contracts":["BG-NUM-001"],
 "tests_added":4,"anchors_verified":{"A1":1,"A2":5,"A3":6},
 "notes":"how you computed `spent` at each refusal site, and whether the pre-existing revolved_curve.rs clippy error was the only one left"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`fix(geometry): the rolling-ball fillet spends a budget, not sixteen (BG-NUM-001-FILLET)`.
