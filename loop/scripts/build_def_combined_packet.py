"""Build the combined CC-DEF-BREP-FIXES packet from the three DEF packets."""
frames = open("loop/packets/CC-DEF-FRAMES.md", encoding="utf-8").read()
facet = open("loop/packets/CC-DEF-FACET-ADMISSION.md", encoding="utf-8").read()
interp = open("loop/packets/CC-DEF-INTERPOLE.md", encoding="utf-8").read()

BT = "`" * 3
header = f"""# CC-DEF-BREP-FIXES — the five B-rep generation defects, one combined packet

Defect records (normative, read all five first):
ORI-FRAME-HANDEDNESS-001, ORI-FRAME-ORTHONORMALITY-GATE-001,
SEM-FACET-SCALE-ZERO-001, SEM-FACET-CORRESPONDENCE-TRUNCATION-001,
NUM-INTERPOLE-OVERSHOOT-001 — all in docs/defects/. Three independent
mechanical fixes, collapsed into one packet (they share only the designed
constructive/mod.rs one-liner; collapse decision recorded in the registry).
Work them in this order; ONE commit at the end covering all three.

{BT}yaml
id:          CC-DEF-BREP-FIXES
contract:    [CC-DEF-BREP-FIXES]
class:       mechanical
crates:      [truck-geometry, truck-modeling]
depends_on:  []
write_allow:
  - vendor/truck/truck-geometry/src/constructive/frame_up.rs
  - vendor/truck/truck-geometry/src/constructive/frame_fixed.rs
  - vendor/truck/truck-geometry/src/constructive/frame_radial.rs
  - vendor/truck/truck-geometry/src/constructive/frame_transport.rs
  - vendor/truck/truck-geometry/src/constructive/profile.rs
  - vendor/truck/truck-geometry/src/constructive/validation.rs
  - vendor/truck/truck-geometry/src/constructive/mod.rs
  - vendor/truck/truck-geometry/src/nurbs/bspcurve.rs
  - vendor/truck/truck-geometry/src/nurbs/mod.rs
  - vendor/truck/truck-geometry/src/nurbs/knot_vec.rs
  - vendor/truck/truck-modeling/src/facet_sweep.rs
  - vendor/truck/truck-modeling/src/spine_sweep.rs
  - vendor/truck/truck-geometry/tests/constructive_frames.rs
  - vendor/truck/truck-geometry/tests/constructive_interpole_bounds.rs
  - showcases/tests/battery_waterslide.rs
  - showcases/tests/battery_construction.rs
read_allow:
  - docs/defects
  - vendor/truck/truck-geometry/src/constructive
  - vendor/truck/truck-geometry/src/nurbs
  - showcases/examples/frame_probe.rs
budget:      {{turns: 34, ctx_tokens: 110000}}
anchors:
  - {{id: A1, expect: 1, cmd: "grep -c 'normal: tangent.cross(binormal)' vendor/truck/truck-geometry/src/constructive/frame_up.rs"}}
  - {{id: A2, expect: 1, cmd: "grep -c 'architectural_up_handedness_inverts_the_solid' showcases/tests/battery_waterslide.rs"}}
  - {{id: A3, expect: 1, cmd: "grep -c 'scale_touches_zero' vendor/truck/truck-modeling/src/spine_sweep.rs"}}
  - {{id: A4, expect: 1, cmd: "grep -c 'through_zero_scale_facet_path_behavior' showcases/tests/battery_construction.rs"}}
  - {{id: A5, expect: 1, cmd: "grep -c 'correspondence_mismatch_facet_path_behavior' showcases/tests/battery_construction.rs"}}
  - {{id: A6, expect: 1, cmd: "grep -c 'pub fn try_interpole' vendor/truck/truck-geometry/src/nurbs/bspcurve.rs"}}
  - {{id: A7, expect: 1, cmd: "grep -c 'GaussianEliminationFailure' vendor/truck/truck-geometry/src/errors.rs"}}
tests_required:
  - architectural_up_frame_is_right_handed_at_every_station
  - fixed_plane_non_planar_spine_refuses_frame_singular
  - radial_about_axis_non_orthogonal_refuses_frame_singular
  - parallel_transport_behavior_bit_identical_on_planar_and_nonplanar_fixtures
  - facet_path_refuses_through_zero_scale
  - facet_path_refuses_mismatched_correspondence
  - spine_sweep_refusals_unchanged_on_the_twin_fixtures
  - valid_recipes_still_realize_on_both_paths
  - sw_violating_knot_vector_refused_typed
  - averaged_knots_helper_matches_de_boor_definition
  - interpolation_stays_within_data_bounds_on_the_probe_fixture
  - existing_interpole_callers_behavior_unchanged_on_valid_inputs
{BT}

"""


def section(text, start_marker):
    i = text.index(start_marker)
    j = text.index("House rules:", i)
    return text[i:j]


s1 = section(frames, "Section 1: the sign fix")
s2 = section(facet, "Section 1: the shared validator")
s3 = section(interp, "Section 1: the SW gate")

tail = """House rules: **H-1: no `unwrap`/`expect`/`panic!` in shipped code, no
module-level `allow` (targeted same-line allows with justification are the
sanctioned escape).** **H-3: float comparisons in tests take the `// H-3`
opt-out ON THE SAME LINE.** **All cargo invocations go through the queue
(the `cargo` on PATH IS the queue shim). Do not invoke cargo by absolute
path; do not unset the shim.** Scoped checks: `cargo check -p
truck-geometry`, `cargo check -p truck-modeling`, `cargo test -p
truck-geometry --lib`, `cargo test -p truck-geometry --test
constructive_frames`, `cargo test -p truck-geometry --test
constructive_interpole_bounds`, `cargo test -p truck-modeling --lib`,
`cargo test -p showcases --test battery_waterslide`, `cargo test -p
showcases --test battery_construction`. The `pub mod validation;` line in
`constructive/mod.rs` is the DESIGNED one-line conflict. COMMIT (one commit,
all three fixes) BEFORE writing RESULT.json AT THE WORKTREE ROOT.

Stop conditions: (1) behavior-preserving constraint: `ParallelTransport`
bit-identical; `spine_sweep` refusal behavior identical; valid inputs to
`try_interpole` bit-identical (no pivoting anywhere); (2) `FixedPlane`
refuses on non-planar spines in v1 (pre-made; projection is a later
amendment); `(x > 0.0)`-style NaN-refusing comparison semantics preserved
in any rewrite; (3) the showcases battery inversions are the acceptance
gates — the side-session ID-named regressions (`ori_frame_*_001_*`,
`sem_facet_*_001_*`, `num_interpole_overshoot_001_*`) are DESIGNED
EXTERNALLY: never author, rename, or delete them; if present at commit they
must be green; (4) if inverting handedness breaks downstream facet/spine
tests beyond the named inversions, record them verbatim in RESULT notes —
they are the defect's own evidence, not this packet's to patch; (5) any
defect record contradicted by the tree: STOP and QUESTION.md.
"""

combined = (
    header
    + "## Fix 1 — frames (ORI-FRAME-HANDEDNESS-001 + ORI-FRAME-ORTHONORMALITY-GATE-001)\n\n"
    + s1
    + "\n## Fix 2 — facet admission (SEM-FACET-SCALE-ZERO-001 + SEM-FACET-CORRESPONDENCE-TRUNCATION-001)\n\n"
    + s2
    + "\n## Fix 3 — interpole admission (NUM-INTERPOLE-OVERSHOOT-001)\n\n"
    + s3
    + tail
)
open("loop/packets/CC-DEF-BREP-FIXES.md", "w", encoding="utf-8", newline="\n").write(combined)
print("combined packet written:", len(combined), "chars")
