# WORK PACKET CT-310-INTEGRATE-COSUPPORT — wire the co-support oracle into boolean admission

Build-spec Wave 3 integration. The final Wave 3 wiring: candidate patch
pairs consult the CT-300 oracle before entering transversality admission
and refinement. `Resolved` regions contribute per the Theorem 6 table and
exit the refinement queue; `Unknown` refuses typed
`CoincidentSupportUncertified` with the current bracket; `NoPositiveAreaOverlap`
flows to the existing broad-phase/geometry path unchanged.

```yaml
id:          CT-310-INTEGRATE-COSUPPORT
contract:    [CT-310-INTEGRATE-COSUPPORT]
class:       mechanical
crates:      [truck123d]
depends_on:  [CT-100-SCHEDULER-WRAP, CT-300-COSUPPORT-CORE]
needs:       [CT-100-SCHEDULER-WRAP, CT-300-COSUPPORT-CORE]
write_allow:
  - truck123d/src/bd_bridge.rs
  - truck123d/tests/atlas_integrate_cosupport.rs
read_allow:
  - docs/CONTACT_ATLAS_SPEC.md
  - docs/CONTACT_ATLAS_BUILD_SPEC.md
tests_required: [truck123d/tests/atlas_integrate_cosupport.rs]
anchors:
  - {id: A1, min: 1,  cmd: "grep -cE '\\bmod atlas\\b' truck123d/src/lib.rs"}
  - {id: A2, min: 40, cmd: "grep -cE '\\bVolumeRow\\b' truck123d/src/bd_bridge.rs"}
budget:      {turns: 45, ctx_tokens: 140000}
```

Anchors at authoring time `e271ba9`. **Re-measure at dispatch.**

## Pre-made judgements

1. **Admission order, frozen**: co-support oracle FIRST (candidate pairs
   from the broad phase), THEN the transversality gate on what remains.
   A Resolved-abutting pair never reaches the transversality gate — that
   is the production unlock (abutting lofts would refuse there forever).
2. **Refusal contract**: `Unknown` ⇒ `CoincidentSupportUncertified` with
   the sound bracket and the pair identity (carrier names, patch ranges)
   in the refusal payload — a future widening packet reads exactly which
   configuration was uncertifiable. Never a silent fallback into
   refinement on an Unknown pair.
3. **V5 net**: fixtures with no co-support (overlapping volumetric
   booleans) take the identical path as CT-100/CT-210; the abutting-box
   fixtures — which today refuse `TransversalityUncertified` or grind —
   now certify with the shared interface contributing per the table.
4. **Stats**: `cosupport_resolved`, `cosupport_refused`, `pairs_exited`
   recorded per dispatch.

## Done-when

1. `cargo test -p truck123d --test atlas_integrate_cosupport --locked`
   green: abutting-box union certifies (interface exited, analytic sum
   matched); the transversality-refusing abutting case from the R4
   taxonomy now certifies; a genuinely uncertifiable pair refuses typed
   with pair identity; V5 net on overlapping fixtures.
2. `cargo test -p truck123d --test fuse_fold --locked` green.
3. fmt clean; `clippy -p truck123d --lib` clean on the diff.
4. Write `RESULT.json` AT THE WORKTREE ROOT (nowhere else), including the
   stats per fixture and the R4-refusal rows this unlock flips (as a
   door spot-check list for the operator).
