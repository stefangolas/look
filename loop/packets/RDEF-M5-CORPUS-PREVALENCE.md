# WORK PACKET RDEF-M5-CORPUS-PREVALENCE — measure the degeneracy classes on real corpora

Milestone M5 of `docs/RANK_DEFICIENT_CONTACT_SPEC.md` (§9 M5 row):
run the certified classifier + sandwich machinery over the STEP and
build123d corpora and measure prevalence — per class, per witness
route, per refusal tag. This measurement DECIDES W4, boundary
tangency, and higher-order jets (spec §9: "from the data").

```yaml
id:          RDEF-M5-CORPUS-PREVALENCE
contract:    [RDEF-M5-CORPUS-PREVALENCE]
class:       survey
crates:      [truck123d]
depends_on:  [RDEF-M2-REGIME-SANDWICH, RDEF-M3-WITNESS-TIER, RDEF-M4-NUMERIC-TIER]
write_allow:
  - docs/RDEF_PREVALENCE.md
  - loop/results
read_allow:
  - corpus/
  - docs/RANK_DEFICIENT_CONTACT_SPEC.md
tests_required: []
anchors:
  - {id: A1, expect: 0, cmd: "grep -c 'per_class' loop/PACKETS.jsonl"}
budget:      {turns: 55, ctx_tokens: 180000}
```

## Method

1. Run every corpus row that reaches the boolean/swept boundary
   through the regime split; tally: transversal / tangential /
   singular_param / refused-with-tag.
2. For tangential pairs: which witness route fired (W1/W2/W3), which
   numeric outcome, which refusal tag. No aggregates without the
   per-row evidence.
3. Publish `docs/RDEF_PREVALENCE.md`: the prevalence tables + the
   W4/boundary-tangency/higher-order-jet recommendations the data
   supports. Determinism discipline: fresh python per row, serial.
4. The recommendations are the owner's decision inputs, not verdicts.

## Done when

Prevalence tables published over the full boolean-reaching corpus;
recommendations recorded; anchors hold (A1 -> nonzero). Write
RESULT.json AT THE WORKTREE ROOT.
