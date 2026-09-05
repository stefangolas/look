# CC-DEF-FRAMES — ORI-FRAME-HANDEDNESS-001 + ORI-FRAME-ORTHONORMALITY-GATE-001

Defect records: `docs/defects/ORI-FRAME-HANDEDNESS-001.md`,
`docs/defects/ORI-FRAME-ORTHONORMALITY-GATE-001.md` (normative; read both
first). Two defects, one root cause: frame laws build `Frame3` by struct
literal, bypassing the landed right-handedness/orthonormality gate
(`constructive/mod.rs:116`).

- **HANDEDNESS**: `frame_up.rs:40` computes `normal: tangent.cross(binormal)`
  = t × b = −b — LEFT-handed; measured |t × n − b| = 2.0 at 13/13 stations;
  swept solids invert (volume-sign flip).
- **GATE**: `FixedPlane`/`RadialAboutAxis` emit non-orthogonal frames on
  non-planar spines (max |t·b| = 0.880, |t·n| = 0.877) with no refusal.

```yaml
id:          CC-DEF-FRAMES
contract:    [CC-DEF-FRAMES]
class:       mechanical
crates:      [truck-geometry]
depends_on:  []
write_allow:
  - vendor/truck/truck-geometry/src/constructive/frame_up.rs
  - vendor/truck/truck-geometry/src/constructive/frame_fixed.rs
  - vendor/truck/truck-geometry/src/constructive/frame_radial.rs
  - vendor/truck/truck-geometry/src/constructive/frame_transport.rs
  - vendor/truck/truck-geometry/src/constructive/mod.rs
  - vendor/truck/truck-geometry/tests/constructive_frames.rs
  - showcases/tests/battery_waterslide.rs
read_allow:
  - docs/defects
  - vendor/truck/truck-geometry/src/constructive
  - showcases/examples/frame_probe.rs
budget:      {turns: 16, ctx_tokens: 70000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'normal: tangent.cross(binormal)' vendor/truck/truck-geometry/src/constructive/frame_up.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'architectural_up_handedness_inverts_the_solid' showcases/tests/battery_waterslide.rs"}
tests_required:
  - architectural_up_frame_is_right_handed_at_every_station
  - fixed_plane_non_planar_spine_refuses_frame_singular
  - radial_about_axis_non_orthogonal_refuses_frame_singular
  - parallel_transport_behavior_bit_identical_on_planar_and_nonplanar_fixtures
  - existing_constructive_frames_tests_stay_green
```

Section 1: the sign fix — `frame_up.rs`: `normal: binormal.cross(tangent)`
(the right-handed completion of the prescribed b-up law: t × n = b).
`ParallelTransport` is measured clean and its behavior must stay
bit-identical — touch it only if the gate routing below requires it, and
record any diff in RESULT notes.

Section 2: the gate — every frame law constructor routes its result through
`Frame3::try_new` (the landed validated constructor); a gate failure maps to
`ConstructError::FrameSingular { at, law }` — typed refusal, never a
silently-degraded frame. PRE-MADE decision (plan §3.2's booked
refuse-or-project for `FixedPlane`): v1 REFUSES on non-planar spines;
projection onto the spine's osculating plane is a later amendment and must
not be improvised. `frame_transport`'s Bishop frame is rotation-minimizing
and passes the gate by construction — route it through `try_new` anyway so
the gate is structural, not per-law convention.

Section 3: the showcases battery test
`architectural_up_handedness_inverts_the_solid` (A2) PINS the defect —
INVERT its assertion to expect a right-handed (volume-sign-preserving)
result, and rename it
`architectural_up_frame_is_right_handed_in_the_solid`. The side-session
ID-named regressions (`ori_frame_handedness_001_*`,
`ori_frame_orthonormality_gate_001_*`, per the defect index's naming
convention) are DESIGNED EXTERNALLY: do not author, rename, or delete
ID-named tests; if they exist in the tree when you commit, they must be
green.

House rules: **H-1: no `unwrap`/`expect`/`panic!` in shipped code, no
module-level `allow`.** **H-3: float comparisons in tests take the `// H-3`
opt-out ON THE SAME LINE.** **All cargo invocations go through the queue
(the `cargo` on PATH IS the queue shim). Do not invoke cargo by absolute
path; do not unset the shim.** Scoped checks: `cargo check -p
truck-geometry`, `cargo test -p truck-geometry --test constructive_frames`,
and the showcases battery (`cargo test -p showcases --test
battery_waterslide`). COMMIT BEFORE writing RESULT.json AT THE WORKTREE
ROOT.

Stop conditions: (1) the other frame consumers are the CG facet backend and
`spine_sweep` — if inverting handedness breaks their tests, that is the
defect's own downstream evidence: record the failures verbatim in RESULT
notes, do not "fix" them here; (2) `Frame3`'s gate tolerance semantics are
landed — do not widen them; (3) the defect records are the authority on
mechanism — if the tree disagrees with a record's claim, STOP and file
QUESTION.md (a record is falsified, which is index business).
