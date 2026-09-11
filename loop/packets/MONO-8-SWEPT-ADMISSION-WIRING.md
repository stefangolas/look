# WORK PACKET MONO-8-SWEPT-ADMISSION-WIRING — extraction adapter + Swept×Swept dispatch

The executor-boundary seam (the booked monocoque gap, items 1+2 plus
the owner-challenge findings). Theory: docs/MONO8_THEORY_STATEMENT.md
sections 1-3 and 5 — READ FIRST, it is normative (extraction map,
orientation signs, admission predicate, bracket aggregation). The
solver (MONO-6, landed), the transversality gate (FSSI-001, landed),
and the patch vocabulary (ADM shim, landed) all exist; this packet
wires them into the executor path so a recorded `cut(loft, loft)` /
`fuse` pair routes to the certified solver instead of the unconditional
`NonCanonicalCarrier` refusal at facade.rs `swept_pair_is_admitted`.

```yaml
id:          MONO-8-SWEPT-ADMISSION-WIRING
contract:    [MONO-8-SWEPT-ADMISSION-WIRING]
class:       design
crates:      [truck123d]
depends_on:  [MONO-7-ROW-ASSEMBLY]
write_allow:
  - truck123d/src/bd_bridge.rs
  - truck123d/src/facade.rs
  - truck123d/tests/swept_admission_wiring.rs
read_allow:
  - truck123d/src/binding.rs
  - docs/MONO8_THEORY_STATEMENT.md
  - docs/MONO_WAVE3_THEORY_BRIEF.md
tests_required: [truck123d/tests/swept_admission_wiring.rs]
anchors:
  - {id: A1, expect: 2, cmd: "grep -c 'swept_pair_is_admitted' truck123d/src/facade.rs"}
  - {id: A2, expect: 0, cmd: "grep -c 'fn extract_patches' truck123d/src/bd_bridge.rs"}
  - {id: A3, expect: 2, cmd: "grep -c 'TensorBernsteinPatch' truck123d/src/bd_bridge.rs"}
budget:      {turns: 75, ctx_tokens: 220000}
```

## Pre-made judgements (do not relitigate; deviations go in RESULT notes)

1. **Extraction is exact and total on the landed vocabulary** (theory
   statement section 1): loft/member carriers -> their landed kernel
   construction's tensor-Bernstein patches; the placement transform
   applies to control points; `sigma = sign det(M_tau)` — REFLECTIVE
   placements (mirror) FLIP the orientation signs. An extraction that
   would resample or approximate is forbidden: refuse typed instead.
2. **The gate order is fixed:** extract both operands -> transversality
   gate (FSSI-001) -> solver. Each failure refuses typed NAMING which
   stage failed (`ExtractionUnavailable` / `NonTransversalContact` /
   `BudgetExhausted`) — the refusal vocabulary is the measurement
   instrument; never collapse the three cases into one message.
3. **The solver's verdict is the facts.** A certified bracket
   `[V_lo, V_hi]` becomes the boolean node's interval-valued volume
   fact (MONO-7's `Facts` carries it; groups aggregate interval sums —
   theory statement section 5). No tolerance band against OCC anywhere.
4. **synthetic-first, corpus-second:** the tests prove the wiring on
   the MONO-6 synthetic pairs (nested/disjoint boxes — exact) and one
   spline-loft pair (the 42-station tub scale) BEFORE any real corpus
   geometry; the corpus's own verdicts belong to the R3 census.
5. `BooleanResultOperand` depth discipline stays: this packet wires
   PAIR dispatch only; chained composition is MONO-9's.

## Method

1. `fn extract_patches(row: &TreeNode) -> Result<Vec<(Patch, i8)>, Refusal>`
   in bd_bridge.rs (theory statement section 1 is the spec; the landed
   `spline_loft_volume_rows` at :1778 already walks the same
   construction — reuse its span decomposition).
2. facade.rs: replace the unconditional Swept×Swept exclusion with the
   gate order of judgement 2; keep every non-Swept path bit-identical
   (V5 discipline — the existing facade battery must not move).
3. Tests: nested boxes exact bracket; disjoint boxes; a spline-loft
   pair closes within budget; a mirror-placed operand gets flipped
   orientation signs and a correct-sign bracket; a deliberately
   tangential synthetic pair refuses `NonTransversalContact`; a member
   carrier not in the extracted vocabulary refuses
   `ExtractionUnavailable`.
4. All cargo through the queue; scoped checks only (`cargo check -p
   truck123d --lib --locked` + the new test file); DLL workaround on
   PATH if 0xc0000135.

## Done when

Scoped checks green serially; anchors hold (A2 -> nonzero, A3 grows);
the existing facade/bridge batteries stay green bit-for-bit. Write
RESULT.json AT THE WORKTREE ROOT.

## Forbidden

No `vendor/truck/**` changes. No multi-operand fold (MONO-9). No mesh
work (MONO-10). No corpus-row verdicts in RESULT notes — this packet
proves the machinery, the census measures the corpus.
