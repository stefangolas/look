# WORK PACKET SOLVER-SURVEY-A — rule-table extraction: kernel surface-relation solvers

You are extracting the semantic solver-rule table for the kernel's
surface-relation solvers, per the frozen contract in
`docs/SOLVER_COVERAGE_SPEC.md` (READ IT FIRST — the RULES schema in section 1
is the deliverable's shape, the semantic axes in section 2 are the
vocabulary, and section 5 bounds your authority). This packet's scope is the
**surface-relation solver subsystem**:

- `vendor/truck/truck-certified/src/tangency/*` (a2, chart, shapes, gates,
  tsystem, minors, exclude, witness, cascade, graph)
- `vendor/truck/truck-certified/src/ssi.rs`, `ssi_admit.rs`, `ssi_trace.rs`,
  `ssi_types.rs`, `ssi_gate.rs`, `ssi_fixtures.rs`
- `vendor/truck/truck-certified/src/kernel/selfint.rs`,
  `kernel/contact.rs`, `kernel/canal.rs`
- `vendor/truck/truck-certified/src/pair_dispatch.rs`

```yaml
id:          SOLVER-SURVEY-A
contract:    [SOLVER-SURVEY-A]
class:       survey
crates:      []
depends_on:  []
write_allow:
  - loop/solver_coverage/fragments/A.json
read_allow:
  - docs/SOLVER_COVERAGE_SPEC.md
  - vendor/truck/truck-certified/src/tangency
  - vendor/truck/truck-certified/src/ssi.rs
  - vendor/truck/truck-certified/src/ssi_admit.rs
  - vendor/truck/truck-certified/src/ssi_trace.rs
  - vendor/truck/truck-certified/src/ssi_types.rs
  - vendor/truck/truck-certified/src/ssi_gate.rs
  - vendor/truck/truck-certified/src/ssi_fixtures.rs
  - vendor/truck/truck-certified/src/kernel/selfint.rs
  - vendor/truck/truck-certified/src/kernel/contact.rs
  - vendor/truck/truck-certified/src/kernel/canal.rs
  - vendor/truck/truck-certified/src/pair_dispatch.rs
tests_required: []
anchors:
  - {id: A1, expect: 21, cmd: "grep -c 'pub fn' vendor/truck/truck-certified/src/kernel/selfint.rs"}
  - {id: A2, expect: 49, cmd: "grep -ci 'krawczyk' vendor/truck/truck-certified/src/construct/bie/ssi4.rs"}
budget:      {turns: 60, ctx_tokens: 190000}
```

## Method

1. **Enumerate solver/reduction/gate entry points** in scope: every `pub fn`
   and dispatch arm that consumes or produces proof-bearing evidence
   (certificates, crossing records, rank/minor verdicts, Krawczyk results,
   refusal variants). One RULE row each, per the spec's schema.
2. **Preconditions/postconditions** translate code facts + theorem comments
   into the spec's semantic axes. Use `!unmodeled:<verbatim>` for any value
   the axes cannot express — that is a FINDING, not a failure.
3. **Theorem comments are load-bearing**: each row whose precondition or
   postcondition comes from a comment carries the verbatim excerpt in
   `theorem_comment` and a confidence. `low` confidence is acceptable and
   expected; never drop the row, never upgrade confidence without code
   evidence.
4. **Call edges**: record `calls` as exact symbol paths (cross-fragment
   edges use symbol paths too; the checker resolves them).
5. **Refusals**: every distinct fail-closed variant constructed in scope
   (grep `Refusal::` and named result enums), with the emitting rule.
6. Output `loop/solver_coverage/fragments/A.json`:
   `{"fragment": "A", "rules": [...], "unmodeled_values": [...],
     "notes": "..."}`. Counts in RESULT.json: rules extracted, per-axis
   coverage, unmodeled values.

## Done when

- The fragment file exists, schema-conformant, with every in-scope public
  entry point represented either as a rule or in the notes with a reason.
- RESULT.json (worktree root) reports the extraction census; anchors hold
  (A2 is outside your write scope and must still hold — it cites the scale
  of the Krawczyk machinery the ssi4 extraction must cover).

## Stop conditions

- A subsystem whose rule structure cannot be expressed in the schema at all
  (not just unmodeled values — the shape itself): STOP, record the symbol
  and the structural mismatch; the spec is amended before proceeding.

Write RESULT.json AT THE WORKTREE ROOT.
