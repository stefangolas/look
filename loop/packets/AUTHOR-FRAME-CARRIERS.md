# WORK PACKET AUTHOR-FRAME-CARRIERS — generalize the drop-in frame carrier and flip the recorded authoring stubs

The TTC-RECENSUS-F1 closing census recorded the measured boundary: all 21 F1
rows refuse at the AUTHORING layer (12 typed stub refusals + 9 untyped
AttributeErrors), zero rows reach a boolean. This packet cures the authoring
surface end to end on the data-row path. No vendor/truck edits. No new
theory: the placement transform is already exact recorded data — this packet
generalizes the recorded rotation from pure-z to a full orthonormal frame and
flips the shim stubs that the landed executor can now answer.

```yaml
id:          AUTHOR-FRAME-CARRIERS
contract:    [AUTHOR-FRAME-CARRIERS]
class:       design
crates:      [truck123d]
depends_on:  [ADM-004-FUNNEL-WIRING, TTC-RECENSUS-F1]
write_allow:
  - truck123d/src/bd_bridge.rs
  - truck123d/src/facade.rs
  - truck123d/python/truck123d/__init__.py
  - corpus/ttc/door.py
read_allow:
  - docs/TTC_CENSUS_FINAL.md
  - docs/TTC_RECENSUS evidence in loop/results/TTC-RECENSUS-F1.json
  - truck123d/src/
  - corpus/ttc/
tests_required:
  - placed_frame_facts_match_unplaced_facts_under_rigid_motion
  - plane_frame_extrude_answers_world_facts
  - pos_placed_assembly_counts_solids
  - vector_surface_answers_direction_math
anchors:
  - {id: A1, expect: 14, cmd: "grep -c '\\brz\\b' truck123d/src/bd_bridge.rs"}
  - {id: A2, expect: 17, cmd: "grep -c 'ProfileEdge' truck123d/src/bd_bridge.rs"}
  - {id: A3, expect: 4,  cmd: "grep -c 'recorded client-layer data' corpus/ttc/door.py"}
budget:      {turns: 75, ctx_tokens: 190000}
```

## Scope decisions (pre-decided)

1. **The frame policy (decided — do not relitigate).** Placement is recorded
   data applied AFTER local construction: `world = translate(o) ∘ R ∘ M`
   where R is a full orthonormal 3x3 rotation recorded from the authoring
   frame and M is the existing coordinate-plane mirror. Volume is invariant
   under R (exact, no recomputation); world bbox = AABB of the 8 rotated
   local corners; mesh triangles transform per-vertex. Placement NEVER
   changes a carrier class and NEVER participates in admission — the census
   pair law is defined on canonical geometry; a placed solid admits exactly
   as its unplaced self. The generalization replaces the `rz`-only rotation;
   keep `mirror` semantics byte-identical.
2. **The bridge has no booleans — keep it that way.** A row that, after
   authoring is cured, reaches a boolean records that as the next boundary
   (typed refusal naming the boolean op). Do NOT add boolean ops to the
   data-row executor in this packet. The monocoque tub cut is expected to
   land on that boundary — that is a SUCCESSFUL deepening, not a failure.
3. **Spline-section lofts: exact or typed.** `ProfileEdge::Spline` already
   exists in `SolidSpec::Loft`. If the executor can answer the facts exactly
   with its existing analytic machinery (or by consuming the landed
   kernel-side loft/volume rows through the facade), do it; if the smooth
   (non-ruled) loft surface question is genuinely open, refuse TYPED naming
   the open carrier — never approximate. Either outcome is a valid row
   verdict; record which.
4. **Shim flips name-for-name (spec-8 discipline).** In `corpus/ttc/door.py`:
   `Plane(origin, x_dir, z_dir)` records a frame (orthonormalize x/z, y =
   z cross x); `Plane.XY/XZ/YZ.offset(...)` composes a translation into the
   marker frame; `Pos` records a translation frame; `Rotation` records an
   axis-angle frame; `Vector` gains the direction-math attribute surface the
   corpus uses (at minimum `normalized`, `dot`, `cross`, `length`) as pure
   client-side data (never a kernel row); `Location` generalizes beyond
   translation/pure-z using the same frame data; `rotate` about a non-z
   axis records a frame. The `Spline`/`Circle`/`Polyline` profile stubs flip
   to profile data rows feeding `loft`/`extrude`/`revolve` ONLY where the
   executor answers exactly (per scope 3); otherwise they keep their typed
   refusal — but the refusal must name the open carrier, not "not a
   carrier".
5. **Determinism and honesty carried verbatim**: fixed-order float
   reductions; orthonormalization is Gram-Schmidt in fixed order; no
   silent fallbacks; facts gates compare against the UNCHANGED recorded OCC
   references (corpus/ttc/reference/*.json — do not re-record or edit any
   reference file; OCC 21/21 was verified bit-identical at the census).

## Done when

```
cargo check --locked -p truck123d
cargo test --locked -p truck123d --lib --tests
```

with all four named tests green (placed-frame rigidity: a solid's facts
under an arbitrary frame equal the unplaced facts volume-wise exactly and
bbox-wise as the rotated AABB; a Plane-frame extrude answers world facts;
a Pos-placed multi-solid assembly reports placed solid_count; the Vector
attribute surface answers client-side), and a kernel-door smoke census over
the 21 F1 rows (kernel engine only, one fresh python per row, serial) whose
per-row verdicts are recorded in the RESULT — flips expected on the 9
Vector-surface rows and the Pos/Plane-frame rows that do not reach a
boolean; rows that reach the boolean boundary keep a TYPED refusal naming
it. Do NOT run the OCC door (the references are unchanged and already
verified bit-identical).

## Forbidden

vendor/truck/** edits. Boolean ops in the data-row executor. Approximate
facts. Editing any corpus reference file or manifest. Weakening an existing
typed refusal that this packet does not explicitly flip.

## Stop conditions

- A placed-solid facts path cannot be made exact → SPEC_GAP naming the
  quantity (volume must be invariant; only bbox/mesh transforms are
  allowed to be recomputed).
- The frame generalization would change any existing green row's verdict →
  defect record, stop-and-file (V5: placement must be verdict-neutral).

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `feat(bridge): general frame carriers — recorded rigid placement, shim authoring stubs flipped (AUTHOR-FRAME-CARRIERS)`.
