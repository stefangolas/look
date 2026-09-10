# WORK PACKET BRIDGE-BOOLEANS â€” the shim cut/fuse/intersect flip through the binding

CG-BINDING landed the exports (boolean dispatch / volume facts / trim facts)
over the stabilized facade. This packet flips the door shim's boolean verbs
to consume them: `cut`/`fuse`/`intersect` record boolean rows and dispatch
through `binding_boolean_dispatch`; placed operands transform to canonical
before dispatch (the recorded exactness rule); every typed refusal names its
carrier.

```yaml
id:          BRIDGE-BOOLEANS
contract:    [BRIDGE-BOOLEANS]
class:       design
crates:      [truck123d]
depends_on:  [CG-BINDING]
write_allow:
  - truck123d/src/bd_bridge.rs
  - truck123d/src/facade.rs
  - corpus/ttc/door.py
read_allow:
  - loop/results/CG-BINDING.json
  - docs/TTC_CENSUS_FINAL.md
tests_required:
  - cut_swept_canonical_certifies_end_to_end
  - fuse_swept_swept_certifies_end_to_end
  - intersect_swept_canonical_certifies_end_to_end
  - placed_operands_transform_to_canonical_before_dispatch
  - refused_pairs_still_refuse_typed_unchanged
anchors:
  - {id: A1, expect: 0, cmd: "grep -c 'binding_boolean_dispatch' truck123d/src/bd_bridge.rs"}
  - {id: A2, expect: 2, cmd: "grep -c '\<certified\>' truck123d/src/bd_bridge.rs"}
  - {id: A3, expect: 1, cmd: "grep -c '\<cut\>' corpus/ttc/door.py"}
budget:      {turns: 65, ctx_tokens: 180000}
```

## Scope decisions (pre-decided)

1. **The shim flips name-for-name.** `cut`/`fuse`/`intersect` (and Mode
   SUBTRACT/INTERSECT spellings) record BooleanOp rows {mode, a, b} and
   dispatch through `binding_boolean_dispatch`. A `Routed` verdict carries
   the certified event; a refused pair keeps the typed
   NonCanonicalCarrier-family refusal naming the carrier class â€” admission
   widens monotonically or not at all (V5).
2. **Placed operands go canonical first.** A placed solid's local geometry
   is what dispatches; the placement composes after (recorded exactness).
   The dispatch NEVER sees placements, so admission is placement-blind and
   verdict-stable.
3. **Depth-1 booleans only.** Chained booleans (boolean of boolean results)
   are the recorded open composition cell â€” a chain deeper than depth-1
   refuses typed naming it (`BooleanResultOperand`), booking the follow-up.
   Do not silently recurse.
4. **Facts gates.** Every row that flips must facts-match its recorded
   reference EXACTLY (solid_count, volume to the recorded doubles, STL
   triangles). A mismatch is a defect record â€” never a tolerance stretch.
   No OCC runs (recorded references are the oracle).

## Done when

```
cargo check --locked -p truck123d
cargo test --locked -p truck123d --lib --tests   (serial, interpreter dir on PATH)
```

with the five named tests green and a kernel-door smoke over the boolean-
stopped rows (airbox, details, monocoque â€” one fresh python per row, serial)
recorded in the RESULT: green-with-facts-match expected on rows whose
carriers admit; typed refusals naming the carrier class are valid verdicts;
untyped failures are not.

## Forbidden

Admission widening without an admitting test. Depth-2 boolean recursion.
Editing recorded references. Weakening any landed row's verdict.

## Stop conditions

- A routed pair's certified verdict cannot express as a door fact â†’ SPEC_GAP
  naming the quantity.
- Any landed green row flips verdict â†’ defect record, stop-and-file (V5).

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `feat(bridge): booleans through the certified funnel â€” cut/fuse/intersect flip, depth-1 (BRIDGE-BOOLEANS)`.
