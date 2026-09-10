# WORK PACKET SOLVER-SURVEY-B — rule-table extraction: certified construction + admission

Extract the semantic solver-rule table for the **certified construction and
admission subsystem**, per `docs/SOLVER_COVERAGE_SPEC.md` (READ FIRST —
section 1 schema, section 2 axes, section 5 authority bounds):

- `vendor/truck/truck-certified/src/construct/admission.rs`
- `vendor/truck/truck-certified/src/construct/bie/*` (mod, closure, ssi4)
- `vendor/truck/truck-certified/src/construct/contact3.rs`
- `vendor/truck/truck-certified/src/construct/blend.rs`,
  `blend_varradius.rs`
- `vendor/truck/truck-certified/src/tangency/gates.rs`, `tsystem.rs`
- `vendor/truck/truck-certified/src/formal/*` (the formal doc modules —
  their stated theorems are rule sources with `theorem_comment` excerpts)

```yaml
id:          SOLVER-SURVEY-B
contract:    [SOLVER-SURVEY-B]
class:       survey
crates:      []
depends_on:  []
write_allow:
  - loop/solver_coverage/fragments/B.json
read_allow:
  - docs/SOLVER_COVERAGE_SPEC.md
  - vendor/truck/truck-certified/src/construct
  - vendor/truck/truck-certified/src/tangency
  - vendor/truck/truck-certified/src/formal
tests_required: []
anchors:
  - {id: A1, expect: 11, cmd: "grep -ci 'transversal' vendor/truck/truck-certified/src/construct/admission.rs"}
  - {id: A2, expect: 3,  cmd: "grep -c 'pub fn' vendor/truck/truck-certified/src/kernel/minor_algebra.rs"}
budget:      {turns: 55, ctx_tokens: 180000}
```

## Method

Identical to SOLVER-SURVEY-A (read it — sections apply verbatim), with these
B-scope specifics:

1. **Admission rules** are gates: their preconditions are exactly the
   transversality/margin facts they certify, and their outcomes split into
   admitted/refused — model both sides (a refusal is a declared outcome with
   an empty postcondition set, marked with the exact variant).
2. **The `formal/*` modules** state theorems in doc form — each becomes a
   rule row whose `source.symbol` is the module item it annotates and whose
   `theorem_comment` is the verbatim statement. Where a formal theorem has
   NO landed implementation, the row's `calls` is empty and the note field
   says `theory-only` — that distinction (theory vs code) is the checker's
   routing-gap input, so it must be preserved exactly.
3. **Krawczyk/ssi4 machinery** (49 sites in ssi4.rs alone): group by
   distinct theorem TRANSFORMATION (exclusion, uniqueness, existence,
   refinement), not per call site — a rule row covers the transformation;
   `source` cites the primary fn; the census of sites goes in RESULT.json.
4. Output `loop/solver_coverage/fragments/B.json` per the schema.

## Done when / Stop conditions

As SOLVER-SURVEY-A (apply verbatim). The theory-only marking rule (2) is
load-bearing — do not merge theory-only rows with implemented rows.

Write RESULT.json AT THE WORKTREE ROOT.
