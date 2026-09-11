# WORK PACKET MONO-9-FUSE-FOLD — certified multi-operand union + chained composition

The monocoque gap item 3 plus the depth-2 composition the census found
everywhere (21 of 27 f1 rows chain booleans on one body; the door
refuses `BooleanResultOperand` past depth 1). Theory:
docs/MONO8_THEORY_STATEMENT.md section 4 — the fold is per-tool flux
against a compound membership indicator, NOT a pairwise fold (a union
of patch 2-cycles is not a patch 2-cycle, and the theory forbids
reconstructing it).

```yaml
id:          MONO-9-FUSE-FOLD
contract:    [MONO-9-FUSE-FOLD]
class:       design
crates:      [truck123d]
depends_on:  [MONO-8-SWEPT-ADMISSION-WIRING]
write_allow:
  - truck123d/src/bd_bridge.rs
  - truck123d/src/facade.rs
  - truck123d/tests/fuse_fold.rs
read_allow:
  - truck123d/src/binding.rs
  - docs/MONO8_THEORY_STATEMENT.md
tests_required: [truck123d/tests/fuse_fold.rs]
anchors:
  - {id: A1, expect: 0, cmd: "grep -c 'compound_indicator' truck123d/src/bd_bridge.rs"}
  - {id: A2, expect: 3, cmd: "grep -c 'BooleanResultOperand' corpus/ttc/door.py"}
budget:      {turns: 70, ctx_tokens: 200000}
```

## Pre-made judgements (do not relitigate; deviations go in RESULT notes)

1. **The fold formulation (theory statement section 4, verbatim):**
   `V(A ∪ B_1 ∪ ... ∪ B_n) = V(A) + Σ_i ∫_{∂B_i} (x·n/3) ·
   1_{outside A ∪ B_1 ∪ ... ∪ B_{i-1}} dS` — each integral is the
   landed MONO-6 flux machinery over `B_i`'s patches with the MONO-5
   membership primitive evaluating against MULTIPLE earlier solids. No
   intermediate union is ever constructed.
2. **Width budget:** `eps_i = eps_total / n` from the fold count;
   the fold's bracket is the interval sum of the per-tool brackets.
   Fold order is the recorded operand order, deterministic, and does
   not affect the bound.
3. **Chained cut/intersect generalizes the same way:** a chained
   subtract `A - t_1 - t_2` is per-tool flux with compound indicator
   `1_inside A and outside t_1..t_{i-1}}`; intersect is flux over each
   operand's boundary with "inside ALL others". One evaluator, three
   modes — do NOT build three solvers.
4. **The depth-1 refusal (`BooleanResultOperand`) is replaced by mode
   dispatch:** an operand that is itself a boolean result contributes
   its EXTRACTED PATCHES if it was a pure union/cut fold with a
   certified bracket, else refuses typed. When in doubt, refuse typed
   naming the open composition — never approximate.
5. **synthetic-first:** tests prove the fold on boxes with known
   closed-form answers (3-tool union exact, chained subtract exact,
   one intersect case) before any spline-carrier composition.

## Method

1. bd_bridge.rs: the compound-indicator evaluator over the MONO-6
   machinery (judgements 1-3); the `Facts`/bracket plumbing rides
   MONO-8's interval-valued volume.
2. facade.rs + door.py: the depth-1 refusal becomes mode dispatch per
   judgement 4 (door.py:1170 is the site).
3. Tests: 3-tool union of boxes, exact; chained subtract, exact;
   one intersect; one spline-carrier 3-tool fuse at tub scale closing
   within budget; depth-2 operand with an uncertifiable sub-result
   refuses typed.
4. All cargo through the queue; scoped checks only; DLL workaround on
   PATH if 0xc0000135.

## Done when

Scoped checks green serially; anchors hold (A1 -> nonzero); the
existing facade/bridge batteries stay green bit-for-bit. Write
RESULT.json AT THE WORKTREE ROOT.

## Forbidden

No `vendor/truck/**` changes. No mesh work (MONO-10). No corpus-row
verdicts. No new solver — one evaluator, three modes.
