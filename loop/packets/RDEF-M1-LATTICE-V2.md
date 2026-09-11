# WORK PACKET RDEF-M1-LATTICE-V2 — lattice v2, fragment E, dual-mode projection

Milestone M1 of `docs/RANK_DEFICIENT_CONTACT_SPEC.md` (§2 CHK-5..9,
§7, §9 M1 row — the spec is normative for every detail). Acceptance is
model-level: under fragment E, the `volume_bracket` rank-deficient
UNDECIDED count is 0 (fail-closed) in lift mode, and the retrodiction
witness cell flips to PROVED.

```yaml
id:          RDEF-M1-LATTICE-V2
contract:    [RDEF-M1-LATTICE-V2]
class:       mechanical
crates:      []
depends_on:  [RDEF-M0-CHECKER-ACCOUNTING]
write_allow:
  - loop/solver_coverage
  - docs/SOLVER_COVERAGE_SPEC.md
read_allow:
  - docs/RANK_DEFICIENT_CONTACT_SPEC.md
tests_required: []
anchors:
  - {id: A1, expect: 0, cmd: "grep -c 'fragment_e' loop/solver_coverage/state.json"}
budget:      {turns: 55, ctx_tokens: 180000}
```

## Method

1. CHK-5 lattice v2: the `relation.regime` and `relation.volume_evidence`
   axes, the six children under `rank_deficient` with their fixed
   `local_dim`s, per spec §2.
2. CHK-6 revised goal predicates (`volume_bracket` accepts
   `volume_evidence = sandwich_bounded`).
3. CHK-7 dual mode (knowledge lift on/off) until OBL-K is discharged;
   report both projections.
4. CHK-8 fragment E: encode the nine rules of spec §7 with
   `confidence: proposed`, existing schema; proposed rules NEVER merge
   into production numbers — separate projection. Answer the spec's
   §11 open questions 1-4 in RESULT notes as you touch the schema.
5. CHK-9 reachability: every new rule shown reachable from the
   door/bridge path or flagged as a routing gap.
6. Acceptance re-run: the M1 row of spec §9 is the done-when.

## Done when

The M1 acceptance line passes; the retrodiction witness flips; the
§1.3 residual gaps are reported; anchors hold (A1 -> nonzero). Write
RESULT.json AT THE WORKTREE ROOT.
