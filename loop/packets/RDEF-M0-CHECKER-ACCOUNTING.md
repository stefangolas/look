# WORK PACKET RDEF-M0-CHECKER-ACCOUNTING — three-way status, tightened lattice, dual metrics

Milestone M0 of `docs/RANK_DEFICIENT_CONTACT_SPEC.md` — the spec of
record for the rank-deficient program. Read its §2 and §9 (M0 row)
FIRST. The current checker mixes refuted, undecided and geometrically
impossible states; fix the accounting before any progress is measured.

```yaml
id:          RDEF-M0-CHECKER-ACCOUNTING
contract:    [RDEF-M0-CHECKER-ACCOUNTING]
class:       mechanical
crates:      []
depends_on:  []
write_allow:
  - loop/solver_coverage
  - docs/SOLVER_COVERAGE_SPEC.md
read_allow:
  - docs/RANK_DEFICIENT_CONTACT_SPEC.md
  - loop/solver_coverage/fragments
tests_required: []
anchors:
  - {id: A1, expect: 2, cmd: "grep -c 'postcondition_claim_mismatch' docs/SOLVER_COVERAGE_AUDIT.md"}
budget:      {turns: 50, ctx_tokens: 160000}
```

## Pre-made decisions (the spec's two [ASM]s, confirmed by the owner
## proxy 2026-09-10 — record them in the spec as decided)

1. **CHK-4:** `regular` means "regular where nonempty" (the second
   reading). This keeps the `TANGENCY-CASCADE-REFINE-EMPTY` routes
   valid; re-audit them under the pinned semantics and report any
   route that changes status.
2. **CHK-2:** the three T_geom constraints are confirmed — they are
   standard dimension facts for the carrier vocabulary the kernel
   admits. Report the state counts before/after tightening.

## Method

1. CHK-1 three-way status (PROVED / REFUTED / UNDECIDED) with the
   refutation predicates of spec §2.
2. CHK-2 tightening, counts before/after.
3. CHK-3 rename the current metric fail-closed coverage; add strict
   coverage (no refusal branch on the winning route).
4. CHK-4 pin `regular` per decision 1; re-audit the cascade routes.
5. **Adjudicate the three v1 conflicts** (TANGENCY-TSYSTEM-FROM-DEFLATED
   content mismatch; exclude_rec NoRootFiveEq; CurveSpan2::RationalBezier)
   — compare each fragment's claim against the source tree, keep the
   correct one, record the adjudication in the fragment files' notes
   and the checker output. The spec (§5.3) makes the TANGENCY-TSYSTEM
   adjudication a precondition for M4's deflation reuse.
6. All fragments stay untouched except recorded adjudications; no
   kernel files; no worker-authored Rust.

## Done when

The checker re-run emits the refuted/undecided split, the strict-vs-
fail-closed table, and the tightened counts (before/after); the three
conflicts are adjudicated with evidence; anchors hold. Write
RESULT.json AT THE WORKTREE ROOT.
