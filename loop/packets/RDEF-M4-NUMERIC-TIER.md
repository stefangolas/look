# WORK PACKET RDEF-M4-NUMERIC-TIER — collinear-normal classification + 1D rules + refusal tags

Milestone M4 of `docs/RANK_DEFICIENT_CONTACT_SPEC.md` (§5.3-5.6, §6,
§9 M4 row — the spec is normative). Kernel-side: vendor/truck/** via
the normal loop. PREREQUISITE: the `TANGENCY-TSYSTEM-FROM-DEFLATED`
conflict must be adjudicated (M0 did it — read its adjudication notes
before reusing deflation).

```yaml
id:          RDEF-M4-NUMERIC-TIER
contract:    [RDEF-M4-NUMERIC-TIER]
class:       design
crates:      [truck-certified]
depends_on:  [RDEF-M3-WITNESS-TIER]
write_allow:
  - vendor/truck/truck-certified/src/tangency/a2.rs
  - vendor/truck/truck-certified/src/tangency/classify.rs
  - vendor/truck/truck-certified/src/tangency/fixtures.rs
  - vendor/truck/truck-certified/tests/rdef_m4_numeric.rs
read_allow:
  - docs/RANK_DEFICIENT_CONTACT_SPEC.md
  - vendor/truck/truck-certified/src/tangency/mod.rs
tests_required: [vendor/truck/truck-certified/tests/rdef_m4_numeric.rs]
anchors:
  - {id: A1, expect: 0, cmd: "grep -c 'collinear_normal' vendor/truck/truck-certified/src/tangency/classify.rs"}
budget:      {turns: 75, ctx_tokens: 220000}
```

## Method

1. **Collinear-normal system (spec §5.3):** the four-equation system
   in original parameters, no chart inversion; non-degeneracy
   obligation discharged in writing; isolation via clipping (§8:
   cubic/bivariate quadratic clipping rates), Krawczyk once isolated.
2. **Classification table (spec §5.3):** the six numeric outcomes ->
   lattice values exactly as tabulated; `NearDegenerate{hint, margin}`
   and `HighOrderJet{k}` refusals with payloads.
3. **1D rules (spec §5.6):** scalar h root analysis, clipping +
   Krawczyk; the h ≡ 0 case needs a witness (M3's).
4. **Refusal tags (spec §6):** all eight tags with payloads, stable
   strings, fail-closed.
5. Fixtures T6-T9 (spec §10). T9 (z = x⁴ + y⁴) is the
   HighOrderJet-without-witness test.
6. Compare against Cheng et al. / Yang–Jia–Yan enumerations per the
   spec's class-vocabulary note; record divergences.

## Done when

Spec §9 M4 acceptance satisfied: T6-T9 pass; no rank-deficient cell
ends without a class or a named tag; scoped checks green. Write
RESULT.json AT THE WORKTREE ROOT.

## Forbidden

No W4 separation bounds in this packet (off by default; M5 data
decides). No boundary-tangency handling (stays
`refuse(BoundaryTangency)`). No tolerance.
