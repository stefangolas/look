# WORK PACKET FRAME-REVOLVE — the non-z revolve carrier: record the revolve axis as a frame

The post-census boundary records the four wheel-corner rows (corner_fl/fr/
rl/rr) refusing typed at "the wheel-frame non-z revolve". The lathe arm is
z-axis-only by profile-plane convention; the landed general-frame placement
(world = translate(o) ∘ R ∘ M, volume invariant under R, exact) makes a
revolve about an arbitrary axis a pure composition. No new theory.

```yaml
id:          FRAME-REVOLVE
contract:    [FRAME-REVOLVE]
class:       design
crates:      [truck123d]
depends_on:  [AUTHOR-FRAME-CARRIERS]
write_allow:
  - truck123d/src/bd_bridge.rs
  - truck123d/src/facade.rs
  - corpus/ttc/door.py
read_allow:
  - loop/results/AUTHOR-FRAME-CARRIERS.json
  - docs/TTC_CENSUS_FINAL.md
tests_required:
  - revolve_about_nonz_axis_answers_world_facts
  - z_revolve_rows_answer_bit_identically
  - revolve_refuses_unsupported_axes_typed
anchors:
  - {id: A1, expect: 9,  cmd: "grep -c '\\<Lathe\\>' truck123d/src/bd_bridge.rs"}
  - {id: A2, expect: 16, cmd: "grep -c '\\<revolve\\>' corpus/ttc/door.py"}
  - {id: A3, expect: 29, cmd: "grep -c '\\<rotation\\>' truck123d/src/bd_bridge.rs"}
budget:      {turns: 55, ctx_tokens: 150000}
```

## Scope decisions (pre-decided)

1. **Composition, not a new kernel.** The revolve is built in its LOCAL frame
   exactly as today's z-axis lathe (profile in the (x, z) revolve-plane
   convention), then placed by the landed general frame taking local z to the
   recorded axis direction. Facts: volume is invariant under the placement
   rotation (exact, never recomputed); world bbox = AABB of the rotated local
   corners; mesh triangles transform per-vertex. All of this machinery landed
   in AUTHOR-FRAME-CARRIERS — reuse it, do not parallel-implement it.
2. **Axis recording.** The shim's `revolve(shape, axis=..., revolution_arc=...)`
   records the axis as a direction vector (build123d `Axis` or a Vector); the
   bridge stores the revolve as (profile, arc, frame). An axis-aligned revolve
   (x or y) is the same composition with the corresponding rotation. The
   legacy z-revolve path keeps its exact recorded row shape bit-for-bit (V5:
   every landed green row answers identically — the z rows are the regression
   net).
3. **Typed refusal for genuinely unsupported axes.** A zero/degenerate axis
   vector refuses typed (`DegenerateRevolveAxis`); an axis not expressible as
   the landed frame composition refuses typed naming the gap. Never
   approximate, never normalize silently past a zero vector.
4. **Determinism carried verbatim**: fixed-order orthonormalization (the
   AUTHOR-FRAME-CARRIERS Gram-Schmidt order), fixed-order float reductions,
   no hash-iteration-dependent output.
5. **No OCC runs.** The recorded references are unchanged; verification is
   kernel-door smoke on the four wheel-corner rows + the named tests. A row
   that flips must facts-match its recorded reference EXACTLY (solid_count,
   volume to the recorded doubles, STL triangles); a facts mismatch is a
   defect record, not a tolerance stretch.

## Done when

```
cargo check --locked -p truck123d
cargo test --locked -p truck123d --lib --tests   (serial, interpreter dir on PATH)
```

with the three named tests green — in particular the rigidity property
(a non-z-axis revolve's facts equal the z-revolve-of-the-rotated-profile's
facts volume-wise exactly), and a kernel-door smoke run of corner_fl (one
fresh python, serial) whose verdict is recorded in the RESULT: expected
either green with facts matching the recorded reference, or a typed refusal
naming the NEXT boundary (both are valid verdicts; an untyped failure is
not).

## Forbidden

vendor/truck/** edits. Approximate facts. Silent axis normalization past
zero. Changing any recorded reference file. Weakening the z-revolve rows'
bit-identity.

## Stop conditions

- The placement composition cannot express an axle revolve exactly →
  SPEC_GAP naming the quantity.
- Any existing green row flips verdict → defect record, stop-and-file (V5).

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `feat(bridge): the non-z revolve carrier — recorded axis frames over the landed placement (FRAME-REVOLVE)`.
