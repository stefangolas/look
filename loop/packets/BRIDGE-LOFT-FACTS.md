# WORK PACKET BRIDGE-LOFT-FACTS — the spline-section loft facts arm through the binding

The spline-section loft rows (monocoque tub, engine_cover, beam_wing,
drs_flap, floor/diffuser class) refuse typed at the facts arm: the bridge's
analytic arms answer line-section lofts exactly but have no smooth
spline-section loft volume. The kernel machinery is landed (L1 extraction ->
TensorBernsteinPatch -> ADM-003's certified volume over patches) and
CG-BINDING exported it (`binding_volume_facts`). This packet wires the
bridge's Loft facts arm to consume it.

```yaml
id:          BRIDGE-LOFT-FACTS
contract:    [BRIDGE-LOFT-FACTS]
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
  - spline_section_loft_volume_certified_bracket
  - loft_facts_match_recorded_reference_on_flip
  - line_loft_rows_answer_bit_identically
  - unmatched_loft_carrier_refuses_typed
anchors:
  - {id: A1, expect: 0, cmd: "grep -c 'binding_volume_facts' truck123d/src/bd_bridge.rs"}
  - {id: A2, expect: 0, cmd: "grep -c 'TensorBernsteinPatch' truck123d/src/bd_bridge.rs"}
  - {id: A3, expect: 37, cmd: "grep -c '\<loft\>' truck123d/src/bd_bridge.rs"}
budget:      {turns: 60, ctx_tokens: 170000}
```

## Scope decisions (pre-decided)

1. **Smooth-loft semantics are the oracle.** The recorded references are
   OCC smooth lofts (build123d's default). The kernel volume arm runs over
   the L1-extracted patches of the SAME smooth surface — the ruled-vs-smooth
   honesty line holds: if the exported arm cannot certify the smooth
   surface's volume exactly, the row refuses TYPED naming it; never
   substitute a ruled approximation and never stretch a tolerance.
2. **Facts gate or typed refusal — no third outcome.** A spline-section
   loft row either facts-matches its recorded reference EXACTLY (solid_count,
   volume doubles, STL triangles) or refuses typed naming the open carrier.
   The corpus's tolerance regime is unchanged.
3. **V5 net.** The landed line-section loft rows answer bit-identically —
   the existing analytic arm stays the fast path for the class it already
   certifies; the kernel arm is only invoked for spline/section carriers the
   analytic arm refuses.
4. **No OCC runs.** Recorded references are the oracle (owner directive:
   no OCC runs at all).

## Done when

```
cargo check --locked -p truck123d
cargo test --locked -p truck123d --lib --tests   (serial, interpreter dir on PATH)
```

with the four named tests green and a kernel-door smoke over the
loft-stopped rows (one fresh python per row, serial) recorded in the RESULT:
green-with-facts-match or typed-refusal naming the open carrier; untyped
failures invalid.

## Forbidden

Approximate volumes. Tolerance stretches. Editing recorded references.
Editing the analytic line-loft arm's landed verdicts.

## Stop conditions

- The smooth-surface volume cannot be certified exactly through the landed
  machinery → typed refusal is the DELIVERABLE for that row; only the
  inability to express it typed is a SPEC_GAP.

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `feat(bridge): spline-section loft facts through the certified volume arm (BRIDGE-LOFT-FACTS)`.
