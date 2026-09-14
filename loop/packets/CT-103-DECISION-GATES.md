# WORK PACKET CT-103-DECISION-GATES — certified threshold/comparison early exit (Theorem 5A)

Adds the decision-directed gates to the atlas scheduler: threshold
`V(R) > X` and pairwise comparison `V_1 > V_2` predicates that terminate
the moment the anytime bracket decides them (Theorem 5A), with the
required margin reported when undecided. Consumer: the census tooling and
the admission gates — most callers ask predicates, not tight volumes.

```yaml
id:          CT-103-DECISION-GATES
contract:    [CT-103-DECISION-GATES]
class:       mechanical
crates:      [truck123d]
depends_on:  [CT-100-SCHEDULER-WRAP]
needs:       [CT-100-SCHEDULER-WRAP]
write_allow:
  - truck123d/src/atlas/scheduler.rs
  - truck123d/tests/atlas_decision_gates.rs
read_allow:
  - docs/CONTACT_ATLAS_SPEC.md
tests_required: [truck123d/tests/atlas_decision_gates.rs]
anchors:
  - {id: A1, min: 1, cmd: "grep -cE '\\bmod atlas\\b' truck123d/src/lib.rs"}
  - {id: A2, min: 6, cmd: "grep -cE '\\bBooleanVolumeCertificate\\b' truck123d/src/bd_bridge.rs"}
budget:      {turns: 30, ctx_tokens: 100000}
```

Anchors at authoring time `e271ba9` (A1 is 1 after CT-000 lands).
**Re-measure at dispatch.**

## Pre-made judgements

1. **Gate semantics, exactly Theorem 5A**: TRUE iff `V_lo > X`; FALSE iff
   `V_hi <= X`; otherwise refine. Comparison: decide on the difference
   interval. Undecided results carry the bracket AND the residual margin
   `delta_needed = W - |V(R) - X|` so the caller can pick the next
   tolerance.
2. **No solver change**: the gates live in the scheduler; the refinement
   machinery from CT-100 is untouched. A decision call may stop EARLY —
   the certificate records `decided: true/false` and the predicate, so
   callers cannot confuse a decided bracket with a converged one.
3. Zero-margin inputs (value exactly at the threshold) must refuse to
   decide (the spec's stated boundary case) — test it.

## Done-when

1. `cargo test -p truck123d --test atlas_decision_gates --locked` green:
   threshold decide-TRUE/decide-FALSE/undecided; comparison
   decide/undecided; zero-margin refusal; scheduler totality preserved
   (finite budgets still refuse typed).
2. fmt clean on touched files; `clippy -p truck123d --lib` clean on the
   diff.
3. Write `RESULT.json` AT THE WORKTREE ROOT (nowhere else).
