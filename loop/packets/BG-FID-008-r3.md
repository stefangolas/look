# WORK PACKET BG-FID-008-r3 — move the witness parameter off a float bisection edge

You are amending one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

**Your prior work is already committed on this branch** (commits `c6a833e`
then `6534bc8`). The r2 engine is sound and 6 of 7 tests pass. The one
failure, `double_cover_witness_refuses`, is NOT an engine defect: your r2
instrumentation proved the in-disc root at `t = WITNESS_T + 2π` (with
`WITNESS_T = 0.7`) coincides EXACTLY (in f64) with a bisection box edge on
the descent path, and a root on a box edge can never certify strict-interior
`Unique` — the operator correctly refuses, so the test sees
`Err(SheetCountUnresolved)` instead of `NotOne { count: 2 }`. This packet
moves the WITNESS PARAMETER (a checker sampling choice, not part of the
spec's canonical double-cover CURVE) to a value whose descending roots never
land on a float bisection edge.

```json
{"id":"BG-FID-008-r3","status":"DONE","contracts":["BG-FID-008"],
 "tests_added":7,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: **if anything below
contradicts what you find in the code as you work it, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-FID-008-r3
covers:      [BG-FID-008, BG-FID-008-r2]
contract:    [BG-FID-008]
class:       design
crates:      [truck-evidence]
write_allow:
  - vendor/truck/truck-evidence/src/fid/one_sheet.rs
  - vendor/truck/truck-evidence/src/fid/mod.rs
read_allow:
  - vendor/truck/truck-evidence/src/num/krawczyk.rs
budget:      {turns: 14, ctx_tokens: 50000}
anchors:
  # Pinned to THIS branch's tip 6534bc8 with `git show`, because the packet
  # is dispatched onto a branch carrying prior work. A count mismatch is a
  # stop condition (ANCHOR_MISMATCH).
  - {id: W1, expect: 21, cmd: "git show 6534bc8:vendor/truck/truck-evidence/src/fid/one_sheet.rs | grep -c 'WITNESS_T'"}
  - {id: W2, expect: 1, cmd: "git show 6534bc8:vendor/truck/truck-evidence/src/fid/one_sheet.rs | grep -c 'fn sup_distance'"}
  - {id: W3, expect: 0, cmd: "git show 6534bc8:vendor/truck/truck-evidence/src/fid/one_sheet.rs | grep -c 'DISC_DECIDE_WIDTH'"}
  - {id: W4, expect: 2, cmd: "git show 6534bc8:vendor/truck/truck-evidence/src/fid/one_sheet.rs | grep -c 'boundary_root_on_disc_edge'"}
  - {id: W5, expect: 1, cmd: "git show 6534bc8:vendor/truck/truck-evidence/src/fid/one_sheet.rs | grep -c 'double_cover_witness_refuses'"}
  - {id: W6, expect: 1, cmd: "git show 6534bc8:vendor/truck/truck-evidence/src/fid/mod.rs | grep -c 'pub mod one_sheet'"}
```

## Problem

The r2 engine drains enumeration to the width floor and decides disc
membership by containment — sound in both directions, ambiguity refusing as
`SheetCountUnresolved`. On the double-cover witness with `WITNESS_T = 0.7`,
one descending root sits exactly ON a float bisection edge
(`t_x + 2π = 6.983185307179586` equals an edge produced by
`0.5*a + 0.5*b` rounding), and strict-interior `Unique` is unreachable from
any box born with that edge. The honest outcome for THAT root is refusal —
but the test's legitimate expectation (`NotOne { count: 2 }`) requires the
root to be certifiable, so the witness parameter must avoid the measure-zero
set of edge-coincident values. This is the BG-NUM-002 lesson in a new guise:
witness parameters must be chosen against the ARITHMETIC of the engine, not
just against dyadic rationals in exact math.

Separately (no action here): the same instrumentation exposed a budget-burn
loop in krawczyk's degenerate split — being fixed by BG-NUM-003-r2, whose
landing this packet does NOT depend on. With this packet's witness move the
double-cover test passes WITHOUT that fix; the two packets are independent.

## Decisions already made for you

### Decision 1 — the change, exactly one constant

`WITNESS_T` changes from `0.7` to `0.71` (same `// H-3:` comment form). The
original packet said "t_x ≈ 0.7 rad"; 0.71 is within that approximation, is
off every dyadic bisection midpoint, and — verified by machine-check —
neither `0.71` nor `0.71 + 2π` (nor any descending root) coincides with any
f64 bisection edge produced by `0.5*a + 0.5*b` on the descent paths within
`[0, 2π]` and `[0, 4π]` down to the width floor. NOTHING ELSE in the module
changes — no engine edits, no test-logic edits, no new tests, no deleted
tests. Every other reference number shifts mechanically with the constant
(cos/sin of 0.71 etc.) and is re-machine-checked per Decision 2.

### Decision 2 — machine-check, extended

Re-run the machine-check script BEFORE writing RESULT.json, extended with the
edge-coincidence check: for each floor-descending root of each certifying
test (single-sheet: `WITNESS_T`; double-cover: `WITNESS_T` and
`WITNESS_T + 2π`; boundary: `WITNESS_T`), simulate the bisection descent
(`mid = 0.5*lo + 0.5*hi`, child = the half containing the root) from the
test's span down to the width floor and assert the root NEVER equals `lo`,
`hi`, or `mid` at any level. Also re-derive through the module's own curve
formulas: the single-sheet crossing distance `eps/2`, the double-cover
in-disc crossing distance `eps*cos(WITNESS_T/2)` and the pair separation
`2*eps*cos(WITNESS_T/2)`, the boundary crossing at exactly `eps`, the offset
sheet at `3*eps`. Paste the script and its output into `notes`.

### Decision 3 — documentation, one comment block

Extend the test module's `WITNESS_T` const comment (or add one comment line
at its first test use) recording WHY 0.71: a witness parameter whose
descending root lands exactly on a float bisection edge can never certify
strict-interior Unique (measured at 0.7: `t_x + 2π` was edge-coincident);
the choice is engine arithmetic, not taste. Do not touch any other doc.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N` literal unless that same line ends with an `// H-3` comment. The
changed const line keeps its same-line `// H-3:` comment. Run
`bash scripts/kernel-gates.sh fc8925f` before writing RESULT.json (the base
spans this branch's whole history back to the original packet's fork point).

## Done when — run these, all must pass

```
cargo fmt --check -p truck-evidence
cargo clippy -p truck-evidence --all-targets --no-deps
cargo test -p truck-evidence --lib --no-fail-fast        # 7/7 must pass now
cargo check --workspace --all-targets
bash scripts/kernel-gates.sh fc8925f
```

## Forbidden

Editing files outside `write_allow`. Changing any constant other than
`WITNESS_T` (and only where the packet's Decisions say). Changing the engine,
the enumeration, the disc decision, or any test's logic or expectation.
Weakening or deleting tests. Bare float literals without `// H-3`.
`unwrap()`/`expect()` on fallible production paths. Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the anchor
- `WITNESS_T = 0.71` still produces an edge-coincident descending root or a
  test that cannot pass → `BLOCKED` with the instrumentation
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. Commit
on the current branch with subject

```
fix(evidence,fid): witness parameter off the float bisection edges (BG-FID-008-r3)
```
