# WORK PACKET SOLVER-SURVEY-D — rule-table extraction: the routing layer (W_code)

Extract the CODE ROUTING table for the bridge/door layer — this is the
W_code half of the routing-gap computation (theory rules from fragments
A-C vs what the dispatch graph actually reaches), per
`docs/SOLVER_COVERAGE_SPEC.md` (READ FIRST):

- `truck123d/src/bd_bridge.rs` (all dispatch arms: loft, members, mirror,
  trim constructor, booleans, facts, mesh)
- `truck123d/src/binding.rs`, `truck123d/src/marshal.rs`,
  `truck123d/src/facade.rs`
- `corpus/ttc/door.py` (the dispatch from corpus ops to bridge entries)
- `docs/OP_CAPABILITY_MATRIX.md` (cross-reference only — the coarse layer
  this survey refines; cite cells, do not re-derive them)

```yaml
id:          SOLVER-SURVEY-D
contract:    [SOLVER-SURVEY-D]
class:       survey
crates:      []
depends_on:  []
write_allow:
  - loop/solver_coverage/fragments/D.json
read_allow:
  - docs/SOLVER_COVERAGE_SPEC.md
  - docs/OP_CAPABILITY_MATRIX.md
  - truck123d/src
  - corpus/ttc/door.py
tests_required: []
anchors:
  - {id: A1, expect: 185, cmd: "grep -cE 'fn [a-z_]+' truck123d/src/bd_bridge.rs"}
  - {id: A2, expect: 7,  cmd: "grep -cE 'fn member_[a-z_]+\\(' truck123d/src/bd_bridge.rs"}
budget:      {turns: 55, ctx_tokens: 180000}
```

## Method

As SOLVER-SURVEY-A (read it — sections apply verbatim), with these D-scope
specifics:

1. **Routing rules, not math rules**: for each dispatch arm record the
   match/discriminator that selects it (the carrier class, the op name, the
   evidence shape), the bridge entry it calls, and the kernel rule ids it
   REACHES (resolve via the kernel symbols into fragments A-C; unresolved
   symbols stay exact paths — the checker resolves or flags them).
2. **The refusal surface**: every typed refusal the routing layer emits,
   with the arm and the trigger — these are the `OUTSIDE_ENVELOPE` /
   fail-closed marks that make safety-closure distinct from coverage.
3. **Carrier-class cells**: each `OP_CAPABILITY_MATRIX` cell touched by an
   arm is cited in the row (`matrix_cell` field) — the refinement relation
   coarse-cell -> semantic routes is the deliverable that kills the
   "one corpus model per cell" blind spot.
4. **door.py dispatch** (`--engine truck` vs `--engine occ`): the engine
   switch and every op-name dispatch is a routing row — this is where
   "the lift ran green through the FACADE path but the timing measured the
   DOOR path" class lives; record both paths where they diverge.
5. Output `loop/solver_coverage/fragments/D.json` per the schema, plus
   `matrix_cell` cross-references.

## Done when / Stop conditions

As SOLVER-SURVEY-A (apply verbatim). A1 is a large anchor: rows need not be
one-per-fn — group helpers under their dispatch arm; the census in
RESULT.json must show every pub entry accounted for (rule, grouped, or
noted).
