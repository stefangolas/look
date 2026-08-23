# WORK PACKET BG-NUM-003-r2 — krawczyk: refuse an unsplittable box instead of looping on it

You are amending one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

**Background (measured, not hypothesized).** BG-FID-008-r2's worker
instrumented this exact failure: when the operator's internal bisection
descends to a box of ~1 ulp width, `push_children` computes
`mid = 0.5*a + 0.5*b` which ROUNDS ONTO an edge (`mid == a` or `mid == b`),
and pushes a width-zero child PLUS a child identical to its parent. The
identical child re-bisects the same way, and the loop consumes the ENTIRE
remaining budget before returning the `NumericallyUnresolved` it could have
returned immediately. One measured call on a box of width 8.97e-14 (root
exactly on its left edge) spent the whole 4096-subdivision budget. The
verdict was always sound; the spend was pure waste, and a caller with a
shared budget loses everything else it wanted to spend on.

```json
{"id":"BG-NUM-003-r2","status":"DONE","contracts":["BG-NUM-003"],
 "tests_added":2,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: **if anything below
contradicts what you find in the code as you work it, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-NUM-003-r2
contract:    [BG-NUM-003]
class:       design
crates:      [truck-evidence]
write_allow:
  - vendor/truck/truck-evidence/src/num/krawczyk.rs
read_allow:
  - vendor/truck/truck-evidence/src/num/roots.rs
budget:      {turns: 16, ctx_tokens: 60000}
anchors:
  # Measured under Git Bash on integration HEAD at dispatch time.
  # A count mismatch is a stop condition (ANCHOR_MISMATCH), not a nuisance.
  - {id: K1, expect: 1, cmd: "grep -c 'pub fn krawczyk' vendor/truck/truck-evidence/src/num/krawczyk.rs"}
  - {id: K2, expect: 1, cmd: "grep -c 'fn push_children' vendor/truck/truck-evidence/src/num/krawczyk.rs"}
  - {id: K3, expect: 7, cmd: "grep -c 'NumericallyUnresolved' vendor/truck/truck-evidence/src/num/krawczyk.rs"}
  - {id: K4, expect: 1, cmd: "grep -c 'mod tests' vendor/truck/truck-evidence/src/num/krawczyk.rs"}
  - {id: K5, expect: 1, cmd: "grep -c 'let mid = 0.5' vendor/truck/truck-evidence/src/num/krawczyk.rs"}
```

## Problem

`push_children` already refuses a width-zero box ("a degenerate point box
that cannot subdivide"). It fails to detect the degenerate SPLIT: a box of
positive width whose float midpoint rounds onto one of its own edges. The
halves it pushes are `[a, a]` (width zero — refuses only when popped) and
`[a, b]` (the parent again). The parent-child loop is bounded only by budget
exhaustion, so a single unsplittable box burns the caller's entire remaining
budget — and every caller of `krawczyk` shares one `Budget`.

## Decisions already made for you

### Decision 1 — the fix, exactly

In `push_children`, after computing the split midpoint of the widest axis
(`let mid = 0.5 * a + 0.5 * b`), refuse when the split is degenerate on that
axis:

```text
if mid == a || mid == b   (the widest axis cannot be bisected in f64)
    -> return Err(Refusal::NumericallyUnresolved { spent, witness: KrawczykIndeterminate })
```

with `spent` computed exactly as the existing width-zero refusal computes it
(nothing spent for this call). This is the same refusal the loop eventually
produces, minus the wasted subdivisions; no reachable verdict changes, only
the spend. Write the comparison in the exact form above (`mid == a`, `mid ==
b`) — no negated comparisons, no `<=`/`>=` stand-ins. Extend the module doc's
worklist paragraph by one clause noting a box whose widest axis cannot be
bisected in f64 refuses immediately (it is at resolution); do not otherwise
touch the docs.

### Decision 2 — tests (in krawczyk.rs's existing test module)

1. `unsplittable_box_refuses_without_burning_budget` — the measured shape:
   `Quad(1.0, -L, 0.0)` (roots at 0 and `L`), start box `[L, L + 4.0*EPS]`
   with `L` a named const (e.g. `1.0`, `// H-3:` comment) — the root sits on
   the left edge, strict interior can never hold, and the descent reaches a
   box whose midpoint rounds onto an edge. Budget 1024. Assert
   `Err(NumericallyUnresolved)` AND `spent.subdiv < 16` — before the fix the
   same call reports `spent.subdiv == 1024`; the assertion is the regression.
   (Also assert the remaining `budget.subdiv` is above 1000 — the spend was
   refused, not consumed.)
2. `centered_root_box_still_certifies` — the contrast case, from the same
   instrumentation: `Quad(1.0, -L, 0.0)`, start `[L - w, L + w]` for a small
   named `w` (e.g. `4.0*EPS`): the root is strictly interior, the operator
   certifies `Unique` one-shot with zero spend.

All floats named consts with same-line `// H-3:` comments. The test module's
existing `#[allow(clippy::unwrap_used, ...)]` discipline stays.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N` literal unless that same line ends with an `// H-3` comment. Every new
float is a named const whose defining line carries a same-line `// H-3:`
comment. Run `bash scripts/kernel-gates.sh <base>` before writing RESULT.json.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-evidence
cargo clippy -p truck-evidence --all-targets --no-deps
cargo test -p truck-evidence --lib --no-fail-fast
cargo check --workspace --all-targets
bash scripts/kernel-gates.sh <base>        # base = merge-base with integration tip
```

truck-evidence is green at baseline. Any baseline failure you did not cause is
a stop condition. Send cargo output to a file and read the tail. Never run a
bare `cargo test`.

## Forbidden

Editing files outside `write_allow`. Changing any verdict the operator returns
on splittable boxes. Weakening or deleting an existing test. Spending budget
before refusing (the fix's whole point is the refused spend). Bare float
literals without `// H-3`. `unwrap()`/`expect()` on fallible production paths.
Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the anchor
- the degenerate split cannot be detected without restructuring push_children's
  axis selection → `SPEC_GAP` naming the mismatch
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. Commit
on the current branch with subject

```
fix(evidence,num): refuse an unsplittable krawczyk box without burning budget (BG-NUM-003-r2)
```
