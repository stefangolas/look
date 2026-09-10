# WORK PACKET TRIM-EXTRUDE-CTOR — the spline-trimmed extrude constructor (door-callable)

The four rows (rear_wing, suspension_front, suspension_rear, steering_rack)
refuse typed at "the spline-trimmed extrude": an extrusion whose boundary is
cut by a spline curve. Every theoretical ingredient is LANDED — the trim
clip (`truck-certified/src/kernel/trimclip.rs`, R9 crossings +
`TrimCrossing` nodes + winding classification), the B-rep promotion, the
ADM-003 algebraic-trim volume brackets (route (a) over the pullback
polynomial), and CG-BINDING's exports. This packet composes them into ONE
door-callable constructor: extrude the profile in its local frame, cut the
result by the recorded spline trim curve, emit facts (volume bracket /
world bbox / mesh) and refuse typed wherever the composition cannot close.

```yaml
id:          TRIM-EXTRUDE-CTOR
contract:    [TRIM-EXTRUDE-CTOR]
class:       design
crates:      [truck123d]
depends_on:  [CG-BINDING]
write_allow:
  - truck123d/src/binding.rs
  - truck123d/src/bd_bridge.rs
  - corpus/ttc/door.py
read_allow:
  - loop/results/CG-BINDING.json
  - docs/CONSTRUCTIVE_GEOMETRY_KERNEL_SPEC_V2.md
  - docs/TTC_CENSUS_FINAL.md
tests_required:
  - spline_trim_extrude_volume_matches_recorded_reference
  - trim_crossings_are_certified_nodes_not_coordinates
  - self_crossing_trim_loop_refuses_typed
  - full_extrude_without_trim_answers_bit_identically
anchors:
  - {id: T1, expect: 9, cmd: "grep -c 'TrimCrossing' vendor/truck/truck-certified/src/kernel/trimclip.rs"}
  - {id: T2, expect: 9, cmd: "grep -c 'algebraic' truck123d/src/binding.rs"}
  - {id: T3, expect: 1, cmd: "grep -c '\\<trim\\>' corpus/ttc/door.py"}
budget:      {turns: 70, ctx_tokens: 190000}
```

## Scope decisions (pre-decided)

1. **One export, composed.** `binding_trim_extrude` (new pyo3 export in
   binding.rs): profile data row + trim curve data row in; certified volume
   bracket + realization data out. It composes the landed stages IN ORDER —
   local-frame extrude (the landed frame carrier), pullback polynomial,
   R9 crossings via the trim clip, winding classification, ADM-003 bracket —
   and NEVER reimplements any stage (no trim math in the bridge layer).
2. **The door idiom.** The shim's trim-carrier rows (whatever the corpus
   scripts write — `bd.split`, `bd.trim`, extrude-then-cut-by-spline —
   match the corpus's actual idiom recorded in the four rows) flip to the
   constructor. Any trim class the composition cannot close (self-crossing
   loops, multiple nested loops beyond the clip's discipline, tangential
   trim) refuses TYPED naming it (`Refuse(TrimClipFailed)` family —
   Inconclusive, never silent, per spec §9.4).
3. **Facts gate discipline.** The four rows facts-match their recorded
   OCC references EXACTLY or refuse typed. No OCC runs (owner directive);
   the recorded references are the oracle. A `TrimClipFailed` on a row is a
   valid recorded verdict; an untyped failure is not.
4. **V5 net.** A plain extrude with NO trim answers bit-identically through
   the new path (the no-trim degenerate case must not change landed
   behavior); all landed rows keep their verdicts.

## Done when

```
cargo check --locked -p truck123d
cargo test --locked -p truck123d --lib --tests   (serial, interpreter dir on PATH)
```

with the four named tests green and a kernel-door smoke over the four
trim-stopped rows (one fresh python per row, serial) recorded in the RESULT:
green-with-facts-match or typed-refusal naming the open carrier.

## Forbidden

Any trim math in bd_bridge/door layers (composition only). Approximate
brackets. Silent winding-number shortcuts. Editing recorded references.
Widening the clip discipline past the spec's §9.4 refusal.

## Stop conditions

- The corpus's actual trim idiom cannot map onto the constructor's input
  rows → SPEC_GAP naming the recorded carrier collision.
- A certified crossing cannot isolate at depth_max on a corpus row →
  record `TrimClipFailed` (correct verdict), not a retry loop.

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `feat(bridge): the spline-trimmed extrude constructor — certified trim clip composed to the door (TRIM-EXTRUDE-CTOR)`.
