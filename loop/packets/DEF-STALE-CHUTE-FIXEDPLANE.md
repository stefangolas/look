# WORK PACKET DEF-STALE-CHUTE-FIXEDPLANE â€” two stale test expectations vs landed behavior

Two SMALL, independent test-expectation repairs found by the CFP one-verify
audit. Both are flips to the LANDED truth; neither weakens anything.

```yaml
id:          DEF-STALE-CHUTE-FIXEDPLANE
contract:    [DEF-STALE-CHUTE-FIXEDPLANE]
class:       mechanical
crates:      [showcases, truck-geometry]
depends_on:  []
write_allow:
  - showcases/tests/battery_waterslide.rs
  - vendor/truck/truck-geometry/tests/constructive_spine_enum.rs
read_allow:
  - vendor/truck/truck-modeling/src/facet_sweep.rs
  - vendor/truck/truck-geometry/src/constructive/frame_fixed.rs
tests_required:
  - both repaired tests green; no other test in either file changes verdict
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'concave_u_chute_refuses_on_the_facet_path' showcases/tests/battery_waterslide.rs"}
  - {id: A2, expect: 3, cmd: "grep -c 'FixedPlane' vendor/truck/truck-geometry/tests/constructive_spine_enum.rs"}
budget:      {turns: 30, ctx_tokens: 120000}
```

## Problem 1 â€” `concave_u_chute_refuses_on_the_facet_path`

The test expects the concave U-chute to REFUSE on the facet path. Landed
PB-003-CONCAVE-CAPS (`e4537d4`) makes the facet path EAR-CLIP non-convex
caps instead of refusing â€” a deliberate capability landing (validated by
PB-003's own battery). The test's expectation is stale. Fix: flip the test
to assert the NEW certified truth â€” the chute BUILDS on the facet path, and
the assertion that replaces the refusal must pin the strongest honest claim
the run supports (the built solid's volume/bounds against the recorded
reference facts; if the test previously asserted a specific refusal enum,
assert the successful construct + the invariants it guarantees). Record the
flip rationale in RESULT: capability landed, expectation moved forward, not
weakened.

## Problem 2 â€” `spine_enum_dispatches_general_to_the_landed_spine_curve` +
`general_spine_becomes_certifiedpatch_not_refused_for_promotion`

Both die at `expect_ok` ("recipe ... refused unexpectedly"): their
`FixedPlane { normal: unit_x }` fixtures ride spines whose tangent is NOT
perpendicular to x, and the `Frame3::try_new` orthonormality gate added by
CC-DEF-BREP-FIXES (`10a1d13`, frame_fixed.rs:38-48) refuses
`FrameSingular{law: FixedPlane}`. The gate is CORRECT (a left-handed or
non-orthonormal frame must refuse). The FIXTURES predate the gate and pin
sloppy normals. Fix: make the fixtures' FixedPlane normals actually
perpendicular to their spines' tangents at the evaluated station (rotate
the pinned normal into the plane perpendicular to the seed tangent) so the
recipe certifies â€” the tests' INTENT (general spine dispatches to the
landed spine curve; general spine becomes CertifiedPatch, not refused) is
preserved. Do NOT weaken the gate.

## Done when

```
cargo test -p showcases --test battery_waterslide concave_u_chute
cargo test -p truck-geometry --test constructive_spine_enum
```

## Forbidden

Weakening the Frame3 orthonormality gate. Deleting tests instead of
aligning them. Touching frame_fixed.rs or facet_sweep.rs (the landed
behavior is correct in both).

## Stop conditions

- The chute run disagrees with its recorded reference facts (volume/bounds
  differ beyond tolerance) -> STOP: that is a REAL defect, file it in
  RESULT as a finding, do not flip the test.
- No perpendicular normal exists for a fixture spine at the needed station
  (tangent parallel to every candidate) -> SPEC_GAP naming the spine.

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `test: align stale chute + FixedPlane fixture expectations with landed semantics (DEF-STALE-CHUTE-FIXEDPLANE)`.
