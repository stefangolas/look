# WORK PACKET BIE-004-CLOSURE — Theorem E polar exclusion + the escalation scheduler

You are implementing the completeness layer of the Certified Interaction
Engine (BIE) program. Everything you need is in this document and
`docs/BIE_BUILD_SPINE.md`. Do not read other spec files. If something you
need is genuinely missing, that is a SPEC_GAP (see "Stop conditions"): you
stop and report, you do not research it.

```yaml
id:          BIE-004-CLOSURE
contract:    [BIE-004-CLOSURE]
class:       design
crates:      [truck-certified]
depends_on:  [BIE-002-SSI4]
write_allow:
  - vendor/truck/truck-certified/src/construct/bie/mod.rs
  - vendor/truck/truck-certified/src/construct/bie/ssi4.rs
  - vendor/truck/truck-certified/src/construct/bie/closure.rs
read_allow:
  - vendor/truck/truck-certified/src/interval/
  - vendor/truck/truck-certified/src/formal/exact.rs
  - vendor/truck/truck-evidence/src/num/krawczyk.rs
  - vendor/truck/truck-certified/src/construct/bie/fixtures.rs
  - docs/BIE_BUILD_SPINE.md
tests_required:
  - polar_exclusion_disposes_known_loop
  - no_loop_property_holds_on_every_cover
  - slope_diagnostic_orders_escalation
  - retry_terminates_or_escalates
budget:      {turns: 60, ctx_tokens: 150000}
```

**New file** (`construct/bie/closure.rs`): H-1 applies — no `unwrap_used`
without a justified same-line opt-out.

## Problem

The BIE-002 solver certifies individual branches; completeness needs the
polar exclusion (Theorem E, theory §4.2): a square 4×4 system that disposes
of interior-loop regions the boundary seeding cannot see, so planted
interior loops are FOUND and empty regions are PROVEN empty. On top of it,
the escalate-iff-predicted-cost scheduler decides when subdivision/retry
continues and when it stops in a typed `Unresolved` — the face-tangency
retry loop (theory §13.3, the booking's top upward-LOC risk) must terminate
or escalate, never spin.

## Scope decisions — pre-made, do not relitigate

1. **This packet escalates the landed BIE-002 module in place** — that is
   why it depends on BIE-002 (spine §4: a real code dependency, the one
   legitimate serialization in the program). You may edit
   `construct/bie/ssi4.rs` to add the scheduler hooks, but every landed
   BIE-002 test must pass UNCHANGED (V5 discipline within the program: the
   solver's certified behavior is not regressed).
2. **Polar exclusion is a 4×4 square system** over the same restricted F
   forms: the polar form's zero set excludes a region iff the system has no
   root in it — certified with the landed `krawczyk::<4>` /
   `KrawczykSystem<4>` machinery (empty-box verdict = Proven(no root)).
   Do not build a homotopy stack; Theorem E makes it unnecessary on the
   normal path (booking §6).
3. **The slope diagnostic (§5.4)** computes the certified slope bound that
   feeds `Unresolved { kappa, cell, slope }` — the witness BIE-000 froze.
4. **The no-loop property (Theorem B)** is asserted as a test oracle on
   every cover the tests construct: a region proven empty by exclusion
   contains no certified branch, and conversely every planted loop is
   found by the cover.
5. **Scheduler policy**: escalate iff predicted cost (box width × depth
   budget from the landed `Budget` type) exceeds the certified-progress
   rate. The exact policy constants are YOUR ONE JUDGEMENT — derive them,
   state them in RESULT notes, and make the retry-termination test prove
   the loop terminates (or escalates typed) on the degenerate fixture.

## Anchors — measured 2026-09-05, counts are exact

Locate by pattern, never by line number. If a count differs, STOP and report
`ANCHOR_MISMATCH` with what you saw.

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `vendor/truck/truck-certified/src/kernel/engine.rs` | `pub fn krawczyk_c1_n4` | 1 |
| A2 | `vendor/truck/truck-evidence/src/num/krawczyk.rs` | `pub fn krawczyk<const N: usize>` | 1 |
| A3 | `vendor/truck/truck-certified/src/construct/bie/ssi4.rs` | `pub struct CertifiedChartCurve` | 1 |

A3 is BIE-002's landed output type — if BIE-002 landed a different name,
STOP and report `ANCHOR_MISMATCH` with the actual name (do not rename it).

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `unimplemented!`, `todo!`, or
  out-of-range indexing reachable from geometry.
- **H-2** Fallible operations return `Outcome<T>`.
- **H-3** No absolute constants in predicates; test epsilons carry `// H-3`
  on the SAME line as the literal.
- **H-6** Float-computed values are never recorded as `Method::Exact`.
- **Determinism** (spine §8): identical ordered input → identical verdicts;
  cover subdivision order is by axis, then low-before-high, always.
- **All cargo invocations go through the queue (the `cargo` on PATH IS the
  queue shim). Do not invoke cargo by absolute path; do not unset the shim.**
- Never run a bare `cargo test` — use the scoped commands below.

## Tests required

Named `#[test]` fns (in-module test sections) — the verifier checks the
names appear in your diff. Fixtures come from the BIE-000 kit plus your own
planted-loop constructions (state the construction math in comments).

1. `polar_exclusion_disposes_known_loop` — a planted interior loop is FOUND
   by the polar system on a fixture pair (e.g. two offset cylinders'
   overlapping band: known loop geometry, stated in comments).
2. `no_loop_property_holds_on_every_cover` — Theorem B: on every cover in
   this test module, regions proven empty contain no branch and every branch
   lies in a non-empty region.
3. `slope_diagnostic_orders_escalation` — the slope bound ranks two
   degenerate boxes in the expected order and feeds the witness.
4. `retry_terminates_or_escalates` — on the tangency fixture, the retry
   loop terminates or returns typed `Unresolved` within the budget — never
   spins (assert the budget is not exhausted by retry alone).

No existing test may be deleted, `#[ignore]`d, or weakened — including
BIE-002's landed tests, which must pass unchanged.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-certified
cargo clippy -p truck-certified --all-targets -- -D warnings
cargo test -p truck-certified --lib
cargo check -p truck-evidence -p truck-shapeops
```

Send cargo output to a file and read the tail.

## Forbidden

Editing any file outside `write_allow` — especially `num/krawczyk.rs`,
`kernel/engine.rs`, `formal/exact.rs`, `src/interval/`, `src/ssi.rs`,
anything under `truck-geometry/` or `truck-shapeops/`. Building a homotopy
or deflation stack (booking §6: deliberately not built). Adding
`#[ignore]`. Adding `#[allow]` without a justification comment on the same
line. Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- Theorem E cannot be instantiated over the restricted F forms without new
  machinery outside the write set → `SPEC_GAP`, naming the gap
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

**COMMIT BEFORE writing `RESULT.json`.** Then write `RESULT.json` at the root
of your worktree (not `loop/results/` — the orchestrator files it there).

```json
{"id":"BIE-004-CLOSURE","status":"DONE","contracts":["BIE-004-CLOSURE"],
 "tests_added":4,"anchors_verified":{"A1":1,"A2":1,"A3":1},
 "notes":"the scheduler policy constants you derived and why; the observed retry behavior on the tangency fixture"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(certified): Theorem E polar exclusion + slope diagnostic + escalation scheduler (BIE-004-CLOSURE)`.
