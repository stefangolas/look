# WORK PACKET SOLVER-CHECKER — the AND-OR coverage checker + the first audit

Merge the four survey fragments (loop/solver_coverage/fragments/A-D.json)
into one rule table, compute the AND-OR winning regions per goal, and emit
the first semantic solver-coverage audit. The contract is
`docs/SOLVER_COVERAGE_SPEC.md` sections 1-5 (READ FIRST): the schema, the
frozen semantic axes, the coverage semantics, the gap classes, and the two
binding demonstrations.

```yaml
id:          SOLVER-CHECKER
contract:    [SOLVER-CHECKER]
class:       mechanical
crates:      []
depends_on:  [SOLVER-SURVEY-A, SOLVER-SURVEY-B, SOLVER-SURVEY-C, SOLVER-SURVEY-D]
write_allow:
  - loop/solver_coverage
  - docs/SOLVER_COVERAGE_AUDIT.md
read_allow:
  - docs/SOLVER_COVERAGE_SPEC.md
  - docs/OP_CAPABILITY_MATRIX.md
  - loop/solver_coverage/fragments
tests_required: []
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'RULES schema' docs/SOLVER_COVERAGE_SPEC.md"}
budget:      {turns: 60, ctx_tokens: 180000}
```

## Method

1. **Checker implementation** — `loop/solver_coverage/checker.py` (orchestra-
   tion-side tooling; no kernel changes):
   a. Merge fragments; dedupe rules by source symbol; CONFLICTING
      postcondition claims between fragments are reported as conflicts for
      orchestrator adjudication — never silently merged.
   b. Build the explicit state set: the product of the spec's axes
      restricted to T_geom-feasible combinations (rank DF=3 over 2x2
      implies local dim 1; canonical carrier implies exact-implicit;
      infeasible cells are excluded from arithmetic, retained in the
      vocabulary).
   c. For each goal: compute W_G by the AND-OR fixed point (iterate: add
      states from which some rule has ALL semantic successors already
      winning; terminal = postconditions prove G, plus explicit
      OUTSIDE_ENVELOPE marks). `M_G = A_G \ W_G` reported by the satisfying
      symbolic description (a concrete axis assignment per missing cell).
   d. Emit per goal: horizontal table (region -> COVERED / route /
      OUTSIDE_ENVELOPE / THEORY GAP), vertical chains (per route: which
      proof-strength level stalls before the goal), routing table
      (W_theory vs W_code differences from fragment D), abstraction gaps
      (every `!unmodeled` value), performance notes (covered only by
      expensive generic routes). Output `docs/SOLVER_COVERAGE_AUDIT.md`
      plus `loop/solver_coverage/state.json` (the computed machine-readable
      winning regions).
2. **Demonstration (b) — deliberate gap:** run the checker with a copy of
   the rule table minus ONE rule (pick a load-bearing one, e.g. the
   ssi4 Krawczyk uniqueness rule); the audit MUST name the exact orphaned
   semantic cell(s) for the affected goal(s). Restore; the gap clears.
   Both runs' reports are committed (the gap run under
   `docs/SOLVER_COVERAGE_AUDIT.deliberate-gap.md`).
3. **Demonstration (a) — retrodiction:** the audit must report, for the
   swept-operand boolean volume goal, the rank_deficient residual parent as
   UNCOVERED (theory gap) — i.e. the checker independently re-derives the
   gap class the loop historically discovered reactively through four
   census rounds. Assert this in a test (`assert retrodiction`) so it is
   regression-protected: if a future rule table starts claiming coverage of
   that residual without a real theorem, the assertion fails and forces
   review.
4. **Low-confidence discipline:** any winning route that depends on a
   `confidence: low` rule row is flagged in the audit (routes are only as
   trustworthy as their weakest extracted link).
5. No kernel changes; no OCC; no cargo builds (pure Python over JSON).

## Done when

- `python loop/solver_coverage/checker.py` runs clean and writes
  `docs/SOLVER_COVERAGE_AUDIT.md` + `state.json`; the deliberate-gap and
  retrodiction demonstrations both behave as specified.
- The audit reports, for all 7 goals: the horizontal table, the vertical
  chains, routing deltas, and the unmodeled-value inventory.
- RESULT.json (worktree root) carries the rule census (per-fragment counts,
  conflicts found, unmodeled values) and the gap-demonstration evidence.

## Stop conditions

- If fragment conflicts cannot be resolved without inventing semantics
  (i.e., two fragments genuinely disagree about a rule's postcondition),
  STOP — adjudication is orchestrator work; never average two claims.
- If the state-space product exceeds explicit-enumeration scale (the v1
  lattice must stay human-checkable), STOP and record the axis that
  exploded — the spec is refined before proceeding, not the checker
  abstracted.

Write RESULT.json AT THE WORKTREE ROOT.
