# WORK PACKET SOLVER-SURVEY-C — rule-table extraction: trim/assemble/fragment stage

Extract the semantic solver-rule table for the **boolean stage machinery**
(split -> classify -> decide -> assemble), per `docs/SOLVER_COVERAGE_SPEC.md`
(READ FIRST — section 1 schema, section 2 axes, section 5 authority):

- `vendor/truck/truck-certified/src/kernel/trimclip.rs`
- `vendor/truck/truck-certified/src/kernel/assemble.rs`
- `vendor/truck/truck-certified/src/kernel/atlas.rs`
- `vendor/truck/truck-certified/src/kernel/sheet.rs`
- `vendor/truck/truck-certified/src/kernel/engine.rs`
- `vendor/truck/truck-certified/src/kernel/residuals_r89.rs`
- `vendor/truck/truck-certified/src/kernel/promote.rs`
- `vendor/truck/truck-certified/src/kernel/patch.rs`, `certs.rs`,
  `evidence.rs`, `claims.rs`

```yaml
id:          SOLVER-SURVEY-C
contract:    [SOLVER-SURVEY-C]
class:       survey
crates:      []
depends_on:  []
write_allow:
  - loop/solver_coverage/fragments/C.json
read_allow:
  - docs/SOLVER_COVERAGE_SPEC.md
  - vendor/truck/truck-certified/src/kernel
tests_required: []
anchors:
  - {id: A1, expect: 28, cmd: "grep -ci 'krawczyk' vendor/truck/truck-certified/src/kernel/engine.rs"}
  - {id: A2, expect: 5,  cmd: "grep -c 'pub fn' vendor/truck/truck-certified/src/kernel/assemble.rs"}
budget:      {turns: 55, ctx_tokens: 180000}
```

## Method

As SOLVER-SURVEY-A (read it — sections apply verbatim), with these C-scope
specifics:

1. **Stage rules**: each stage of the boolean pipeline (split, classify,
   decide, assemble) is a rule whose postconditions are the facts it hands
   the NEXT stage. The guarantee-carried-across-transitions question is the
   point: record exactly which evidence types each stage consumes and
   produces; a stage whose output evidence under-delivers its consumer's
   precondition is a VERTICAL GAP row — mark it in the notes, don't fix it.
2. **Fragment/mesh classification** (`FragmentMesh`, material states,
   winding): these are classifier rules — outcomes are the material/state
   variants, each with its postcondition axis values.
3. **trimclip crossings** (`TrimCrossing`, R9): the crossing record types
   are evidence; each constructor/classifier of them is a rule.
4. Group repeated Krawczyk sites by distinct transformation (the B-packet
   convention applies here too).
5. Output `loop/solver_coverage/fragments/C.json` per the schema.

## Done when / Stop conditions

As SOLVER-SURVEY-A (apply verbatim).
