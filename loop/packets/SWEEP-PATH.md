# WORK PACKET SWEEP-PATH — the spline-path sweep carrier: exact path queries + the recording arm

The post-census boundary records mvac refusing typed at "a spline path
tangent query is not a kernel-engine row", and the door shim's `sweep`
refuses everything (`"sweep is not a kernel-engine row"`). The kernel-side
sweep-as-loft-chain row is landed (F1-AUTHORING-ARMS); the bridge already
reconstructs the interpolating spline EXACTLY for volumes (the Lagrange/
Hermite machinery behind `spline_edge_volume`). This packet answers the path
queries with that same exact math and lands the sweep recording arm. No new
theory; one frame-law pin (scope 2).

```yaml
id:          SWEEP-PATH
contract:    [SWEEP-PATH]
class:       design
crates:      [truck123d]
depends_on:  [FRAME-REVOLVE]
write_allow:
  - truck123d/src/bd_bridge.rs
  - truck123d/src/facade.rs
  - corpus/ttc/door.py
read_allow:
  - loop/results/AUTHOR-FRAME-CARRIERS.json
  - loop/results/FRAME-REVOLVE.json
  - docs/TTC_CENSUS_FINAL.md
tests_required:
  - spline_path_position_and_tangent_answer_exactly
  - sweep_line_path_records_loft_chain
  - sweep_spline_path_records_stations
  - z_revolve_and_loft_rows_answer_bit_identically
anchors:
  - {id: A1, expect: 6, cmd: "grep -c 'sweep' corpus/ttc/door.py"}
  - {id: A2, expect: 7, cmd: "grep -c 'tangent' corpus/ttc/door.py"}
  - {id: A3, expect: 2, cmd: "grep -c 'position_at' corpus/ttc/door.py"}
budget:      {turns: 60, ctx_tokens: 160000}
```

## Scope decisions (pre-decided)

1. **Exact path queries.** `Edge.position_at` / `tangent_at` on a recorded
   spline answer through the SAME exact interpolant the bridge's volume arms
   use (the recorded-sample Hermite/Lagrange reconstruction, fixed-order) —
   never a Python-side approximation, never a refinement-dependent answer.
   `position_at` stops refusing mid-interval; `tangent_at` returns the exact
   interpolated derivative. Endpoints stay exact.
2. **The frame-transport pin (the one decision).** A sweep walks the path
   with per-station frames. The pinned law: parallel-transport-style frames
   computed by the SAME fixed-order double-reflection discipline at the
   recorded station list, stored as recipe data (the spec §5.3 FrameData
   pattern) — resolution-independent once frozen, recorded, never
   recomputed from a different station set. Refuse a path with a
   zero-tangent station typed (`DegenerateSweepPath`). Do NOT implement a
   validated ODE integrator (the spec forbids it); do not use Frenet.
3. **The recording arm.** `sweep(section, path)`: a line path records the
   landed straight-sweep carrier; a spline path records a loft chain over
   the recorded stations (the landed sweep-as-loft-chain row), with section
   frames per scope 2. Section carriers outside the recorded vocabulary
   refuse typed naming the gap. `fillet`/`chamfer` on swept rows refuse
   typed exactly as today.
4. **V5 net.** Every landed green row (lofts, revolves, z-rows) answers
   bit-identically; the battery tests pin it.
5. **No OCC runs.** Kernel-door smoke on mvac (one fresh python, serial);
   expected green-with-facts-match OR a typed refusal naming the NEXT
   boundary; never untyped, never a tolerance stretch.

## Done when

```
cargo check --locked -p truck123d
cargo test --locked -p truck123d --lib --tests   (serial, interpreter dir on PATH)
```

with the four named tests green and the mvac smoke verdict recorded in the
RESULT.

## Forbidden

vendor/truck/** edits. Approximate tangents. Frenet framing. Editing
recorded references. Weakening landed rows' bit-identity.

## Stop conditions

- The exact interpolant cannot answer the tangent (degenerate stations) →
  typed refusal, not SPEC_GAP, unless the CLASS is unexpressible — then
  SPEC_GAP naming it.
- Any existing green row flips verdict → defect record, stop-and-file (V5).

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `feat(bridge): the spline-path sweep carrier — exact path queries, recorded station frames (SWEEP-PATH)`.
