# CC-DEF-BREP-FIXES â€” the five B-rep generation defects, one combined packet

Defect records (normative, read all five first):
ORI-FRAME-HANDEDNESS-001, ORI-FRAME-ORTHONORMALITY-GATE-001,
SEM-FACET-SCALE-ZERO-001, SEM-FACET-CORRESPONDENCE-TRUNCATION-001,
NUM-INTERPOLE-OVERSHOOT-001 â€” all in docs/defects/. Three independent
mechanical fixes, collapsed into one packet (they share only the designed
constructive/mod.rs one-liner; collapse decision recorded in the registry).
Work them in this order; ONE commit at the end covering all three.

```yaml
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
budget:      {turns: 34, ctx_tokens: 110000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'normal: tangent.cross(binormal)' vendor/truck/truck-geometry/src/constructive/frame_up.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'architectural_up_handedness_inverts_the_solid' showcases/tests/battery_waterslide.rs"}
  - {id: A3, expect: 2, cmd: "grep -c 'scale_touches_zero' vendor/truck/truck-modeling/src/spine_sweep.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'through_zero_scale_facet_path_behavior' showcases/tests/battery_construction.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'correspondence_mismatch_facet_path_behavior' showcases/tests/battery_construction.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub fn try_interpole' vendor/truck/truck-geometry/src/nurbs/bspcurve.rs"}
  - {id: A7, expect: 2, cmd: "grep -c 'GaussianEliminationFailure' vendor/truck/truck-geometry/src/errors.rs"}
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
```

## Fix 1 â€” frames (ORI-FRAME-HANDEDNESS-001 + ORI-FRAME-ORTHONORMALITY-GATE-001)

Section 1: the sign fix â€” `frame_up.rs`: `normal: binormal.cross(tangent)`
(the right-handed completion of the prescribed b-up law: t Ã— n = b).
`ParallelTransport` is measured clean and its behavior must stay
bit-identical â€” touch it only if the gate routing below requires it, and
record any diff in RESULT notes.

Section 2: the gate â€” every frame law constructor routes its result through
`Frame3::try_new` (the landed validated constructor); a gate failure maps to
`ConstructError::FrameSingular { at, law }` â€” typed refusal, never a
silently-degraded frame. PRE-MADE decision (plan Â§3.2's booked
refuse-or-project for `FixedPlane`): v1 REFUSES on non-planar spines;
projection onto the spine's osculating plane is a later amendment and must
not be improvised. `frame_transport`'s Bishop frame is rotation-minimizing
and passes the gate by construction â€” route it through `try_new` anyway so
the gate is structural, not per-law convention.

Section 3: the showcases battery test
`architectural_up_handedness_inverts_the_solid` (A2) PINS the defect â€”
INVERT its assertion to expect a right-handed (volume-sign-preserving)
result, and rename it
`architectural_up_frame_is_right_handed_in_the_solid`. The side-session
ID-named regressions (`ori_frame_handedness_001_*`,
`ori_frame_orthonormality_gate_001_*`, per the defect index's naming
convention) are DESIGNED EXTERNALLY: do not author, rename, or delete
ID-named tests; if they exist in the tree when you commit, they must be
green.


## Fix 2 â€” facet admission (SEM-FACET-SCALE-ZERO-001 + SEM-FACET-CORRESPONDENCE-TRUNCATION-001)

Section 1: the shared validator â€” NEW file
`constructive/validation.rs`: `pub fn validate_scalar_law_range(law:
&ScalarLaw, domain: (f64, f64)) -> Result<(), ConstructError>` returning
`Err(ProfileCollapse { at })` with the THROUGH-ZERO detection: the signed
scale must not change sign (or touch zero) anywhere in the CLOSED domain â€”
interval-style endpoint+sign reasoning over the law's declared form, not
station sampling (that is the defect: sampling missed it). And `pub fn
validate_correspondence(start: usize, end: usize) -> Result<(),
ConstructError>` refusing count mismatch with `ProfileCorrespondenceMismatch`
â€” the check `try_linear_correspondence` enforces at construction, now also
enforced at evaluation for struct-literal laws (the defect path:
`ProfileLaw` built by struct literal bypasses `try_linear_correspondence`;
keep `try_linear_correspondence` unchanged and add the evaluate-time gate).

Section 2: both entries call the shared validator â€” `spine_sweep` REPLACES
its private `scale_touches_zero` (A1) with the shared fn (behavior
identical â€” its existing refusals must stay byte-identical on the twin
fixtures); `facet_sweep` ADDS the two validation calls at entry, refusing
`ProfileCollapse`/`ProfileCorrespondenceMismatch` exactly as the BREP path
already does. No other behavior change: the grid registry, winding audit,
and verdicts are untouched.

Section 3: the showcases twins
`through_zero_scale_facet_path_behavior` (A2) and
`correspondence_mismatch_facet_path_behavior` (A3) PIN the defect â€” INVERT
both facet-path assertions to expect `Err` (mirror the BREP twins' shapes).
The side-session ID-named regressions (`sem_facet_scale_zero_001_*`,
`sem_facet_correspondence_truncation_001_*`) are DESIGNED EXTERNALLY: do
not author or rename them; if present at commit, they must be green.


## Fix 3 â€” interpole admission (NUM-INTERPOLE-OVERSHOOT-001)

Section 1: the SW gate â€” after solving, VALIDATE before returning: the
delivered control points are checked against a BOUNDEDNESS criterion, and
the knot vector against the Schoenbergâ€“Whitney condition
(`M_{j,q}(v_k) != 0` for every station). Pre-made decisions: (a) a knot
vector violating SW â†’ `Err(Error::InterpolationNotSwVerified { at })` (NEW
typed variant in `errors.rs`, alongside `GaussianEliminationFailure` (A2) â€”
never a panic, never a silent accept); (b) a solve whose control-point
extent exceeds `BOUND_FACTOR Ã—` the data extent (new named const, value
1e3, justified in the module doc from the record's scaling table: honest
interpolants sit at O(1)Ã— data extent, the defect at 10â¹Ã—) â†’ the same typed
refusal â€” an interpolant that wildly overshoots is not delivered as
success; (c) NO pivoting, NO change to the solve itself: valid inputs get
bit-identical results (the landed-behavior guard).

Section 2: the helper â€” `pub fn averaged_interpolation_knots(stations:
&[f64], degree: usize) -> KnotVec` in `knot_vec.rs`: de Boor averaging
`Î¾_{j+q} = (1/q) Î£_{r=j}^{j+qâˆ’1} v_r` with clamped ends repeated q+1 â€” the
pure-math port of the landed certified version (truck-geometry cannot
depend on truck-certified; the two implementations must agree, and a test
asserts agreement on a shared fixture against the certified one is the
side-session's business â€” here the DEFINITION is the ground truth). Doc
comment points interpolant users at it as the default knot choice.

Section 3: the probe fixture â€” test 3 reproduces the record's scaling table
in miniature: bounded data points (unit-scale), the BAD knot choice that
triggered the defect, assert the typed refusal; then the same data with
`averaged_interpolation_knots`, assert between-sample evaluation stays
within `BOUND_FACTOR`-style bounds (H-3 opt-outs). The showcases probes
(`knot_probe.rs`, `mesh_probe.rs`) and
`facet_mesh_stays_within_path_bounds` are the external oracles â€” the
side-session owns their ID-named successors
(`num_interpole_overshoot_001_*`); do not author or rename them.

House rules: **H-1: no `unwrap`/`expect`/`panic!` in shipped code, no
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
gates â€” the side-session ID-named regressions (`ori_frame_*_001_*`,
`sem_facet_*_001_*`, `num_interpole_overshoot_001_*`) are DESIGNED
EXTERNALLY: never author, rename, or delete them; if present at commit they
must be green; (4) if inverting handedness breaks downstream facet/spine
tests beyond the named inversions, record them verbatim in RESULT notes â€”
they are the defect's own evidence, not this packet's to patch; (5) any
defect record contradicted by the tree: STOP and QUESTION.md.
