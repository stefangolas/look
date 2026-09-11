# WORK PACKET RDEF-M2-REGIME-SANDWICH — admission dichotomy + tangential volume, 2x2

Milestone M2 of `docs/RANK_DEFICIENT_CONTACT_SPEC.md` — the spec is
normative for all mathematics (Lemma D §3.1, Lemma S §4.2 with
obligations S1-S3, Lemma T §4.4, enclosures §4.3, schedule §4.4).
This packet implements the (T)/(G) admission dichotomy and the
tangential sandwich volume rule for 2x2, with BOTH range-function
paths (§4.3's regression trap is a test requirement, not a note).

```yaml
id:          RDEF-M2-REGIME-SANDWICH
contract:    [RDEF-M2-REGIME-SANDWICH]
class:       design
crates:      [truck123d]
depends_on:  [RDEF-M1-LATTICE-V2, MONO-8-SWEPT-ADMISSION-WIRING]
write_allow:
  - truck123d/src/bd_bridge.rs
  - truck123d/src/facade.rs
  - truck123d/tests/rdef_m2_sandwich.rs
read_allow:
  - truck123d/src/binding.rs
  - docs/RANK_DEFICIENT_CONTACT_SPEC.md
tests_required: [truck123d/tests/rdef_m2_sandwich.rs]
anchors:
  - {id: A1, expect: 0, cmd: "grep -c 'sandwich_bounded' truck123d/src/bd_bridge.rs"}
budget:      {turns: 80, ctx_tokens: 240000}
```

## Method

1. **Admission dichotomy (spec §3.1):** normal cones (reuse
   `ADMISSION-WHOLE-BOX-REGULAR-CONE` if it exposes them — open
   question 6 of the spec; report the answer), the (T)/(G) split with
   the tie rule, G-INJ via the interval-hull Jacobian, the
   `SingularParametrization` / `GraphNotInjective` refusals. This
   AMENDS MONO-8's gate: `NonTransversalContact` becomes the regime
   split.
2. **Sandwich volume (spec §4):** the (G) graph set-up, the sandwich
   bound with obligations S1-S3 discharged IN CODE AND ARGUMENT (the
   written proofs are deliverables, cited in RESULT notes), both
   range paths (Bernstein fast path + mandatory generic fallback),
   the priority-queue schedule keyed |R|·sup|h| with (key, cell id)
   tie-break, the `CoincidenceWithoutExactCarrier` floor and
   `BudgetExhausted{achieved_width}` refusals.
3. **Fixtures V1-V6 (spec §10):** each asserts true-volume-in-bracket,
   monotone width vs budget, determinism, and correct refusal tags.
   V5 is the fallback regression trap; V6 is the retrodiction case.
4. All cargo through the queue; scoped checks; DLL workaround on PATH
   if 0xc0000135. The written Lemma D/S/T proofs go in the RESULT
   notes or a docs addendum referenced from it.

## Done when

Spec §9 M2 acceptance row satisfied (fixtures pass, width-vs-budget
curves published, swept cut/fuse return bracket-or-named-refusal);
`cargo check -p truck123d --lib --locked` green; existing batteries
green. Write RESULT.json AT THE WORKTREE ROOT.

## Forbidden

No vendor/truck/** changes. No classification in the volume path
(§5 is M3/M4's). No tolerance anywhere. No knowledge lift without
OBL-K discharged (the fact stays dual-mode in the checker until then).
