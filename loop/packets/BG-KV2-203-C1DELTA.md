# BG-KV2-203-C1DELTA — the Spine enum, PH fast path, and FrameData (constructive half)

Wave-2 implementation packet (build spec §4; §19 row 5's v2 delta; spec
§5.2–§5.3). Lands the `Spine` ENUM with the PH fast path (RrmfQuintic |
RmErfSeptic) and `FrameData` (the declared double-reflection refinement
level stored as data). This is the constructive half: write set in
truck-geometry + truck-modeling (the ripple of the trait rename), disjoint
from the certified-crate Wave-2 packets.

**The owner seam (build spec §4): the landed `Spine` is a TRAIT
(`constructive/recipe.rs:44`). The enum lands beside it at module level
with the spec spelling; the trait RENAMES to `SpineCurve`** with the old
name retained as a deprecated alias ONLY if a rename ripples beyond the
measured call sites.

**CENSUS CORRECTION (r2 amendment, from the r1 worker's stop-condition-1
finding — the session-46 census-scope-vs-write-set class, re-hit): the
design-time census UNDERCOUNTED the trait rename ripple. Measured at
dispatch, the trait `Spine` is referenced by five additional files, all now
IN write_allow: `constructive/frame_transport.rs` (use + two `&dyn Spine`
signatures), `truck-modeling/src/facet_sweep.rs` (:85 and :413 generics),
and the integration-test files `constructive_frames.rs`,
`constructive_transport.rs`, `facet_sweep_conformance.rs` (impl blocks +
generic fns). All five migrate in the same commit. canonical.rs / graph.rs
`Spine*` matches are SpineFrameSurface/SpineFrameCurve/AnyArc::Spine names,
NOT the constructive trait — do not touch them for the rename.**

**H-1.** New modules (`constructive/spine_ph.rs`,
`tests/constructive_spine_enum.rs`) carry their crate's unwrap discipline:
no `unwrap`/`expect`/`panic!`, no module-level `allow`. truck-geometry's
crate-wide lint header style is the template (check `truck-geometry/src/
lib.rs`; if it does not deny unwrap_used, keep new code unwrap-free anyway
and say so in the module doc).

```yaml
id:          BG-KV2-203-C1DELTA
contract:    [BG-KV2-203-C1DELTA]
class:       design
crates:      [truck-geometry, truck-modeling, truck-certified]
depends_on:  [BG-KV2-102-LEAF]
write_allow:
  - vendor/truck/truck-geometry/src/constructive/recipe.rs
  - vendor/truck/truck-geometry/src/constructive/spine_ph.rs
  - vendor/truck/truck-geometry/src/constructive/mod.rs
  - vendor/truck/truck-geometry/src/constructive/frame_transport.rs
  - vendor/truck/truck-geometry/src/decorators/spine_frame.rs
  - vendor/truck/truck-geometry/tests/constructive_frames.rs
  - vendor/truck/truck-geometry/tests/constructive_transport.rs
  - vendor/truck/truck-geometry/tests/constructive_spine_enum.rs
  - vendor/truck/truck-modeling/src/spine_sweep.rs
  - vendor/truck/truck-modeling/src/facet_sweep.rs
  - vendor/truck/truck-modeling/tests/facet_sweep_conformance.rs
read_allow:
  - docs/CONSTRUCTIVE_GEOMETRY_KERNEL_SPEC_V2.md
  - vendor/truck/truck-geometry/src/constructive
  - vendor/truck/truck-modeling/src/spine_sweep.rs
  - vendor/truck/truck-geometry/src/decorators/spine_frame.rs
budget:      {turns: 34, ctx_tokens: 110000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub trait Spine' vendor/truck/truck-geometry/src/constructive/recipe.rs"}
  - {id: A2, expect: 5, cmd: "grep -c 'TRANSPORT_STATIONS' vendor/truck/truck-geometry/src/constructive/frame_transport.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub struct SpineFrameRecipe' vendor/truck/truck-geometry/src/constructive/recipe.rs"}
  - {id: A4, expect: 0, cmd: "grep -rnw 'PhSpine' vendor/truck/truck-geometry/src | wc -l"}
  - {id: A5, expect: 0, cmd: "grep -rnw 'FrameData' vendor/truck/truck-geometry/src | wc -l"}
tests_required:
  - spine_enum_dispatches_general_to_the_landed_spine_curve
  - polyline_spine_refuses_as_not_c1_through_the_enum
  - ph_quintic_yields_rational_frame_and_exact_arclength_samples
  - frame_data_refinement_level_changes_surface_and_is_recorded
  - frame_data_is_resolution_independent_once_frozen
  - general_spine_becomes_certifiedpatch_not_refused_for_promotion
  - ph_arclength_matches_f64_integration_ground_truth
```

## Section 1 — the enum (`constructive/recipe.rs` + NEW `constructive/spine_ph.rs`)

```rust
pub enum Spine {
    Ph(PhSpine),        // exact fast path — never an admission criterion
    General(Box<dyn SpineCurve>),   // procedural, non-rational, first-class
}
```

- The existing trait renames `Spine` -> `SpineCurve` (same methods
  `domain`/`position_at`/`derivative_at`); ALL measured impls/call sites
  migrate in the same commit (the ripple IS the write set: spine_frame.rs,
  spine_sweep.rs). If a call site outside the write set references the
  trait, STOP (stop condition 2) and name it.
- `Spine::General` wraps the trait object; the landed `LineSpine`/
  `PolylineSpine`/`Curve` impls keep working unchanged through the rename.
- `PhSpine` (NEW, `spine_ph.rs`): the two characterized subclasses verbatim
  per spec §5.2 — `RrmfQuintic` (quintic PH satisfying the RRMF condition)
  and `RmErfSeptic` (degree-7 PH whose Euler-Rodrigues frame is
  rotation-minimizing). Data: the Bézier control net of the PH curve (PH
  curves are polynomial — Bézier form is exact); constructors validate the
  PH property (the |c'(w)|^2 = sigma(w)^2 structure: control-coefficient
  checks per Farouki's characterization, implemented as explicit
  polynomial identity checks on the coefficients — no transcendental).
- The PH fast path delivers, per the spec's implication chain: rational
  unit tangent -> rational RMF (the EXACT rotation-minimizing frame for the
  two subclasses — implement the closed-form rational frame documented for
  RRMF quintics; cite the characterization in the doc, the FORMULA is the
  deliverable) -> exact polynomial parametric speed -> exact arc length
  for chord sampling. The `ph_arclength_matches_f64_integration_ground_
  truth` test: the rational arc length vs adaptive Simpson in f64 on
  fixtures, agreement to 1e-9 (H-3 same-line).

## Section 2 — FrameData (§5.3 verbatim)

```rust
pub struct FrameData { pub refinement_level: u32 }   // stored in the recipe
```

- `SpineFrameRecipe` gains `frame_data: FrameData` (a fourth field — the
  struct's construction sites migrate; `try_new`-style constructors
  default it and record).
- For `Spine::General` + `ParallelTransport`: the double-reflection frame
  runs at the DECLARED refinement level (the hardcoded `TRANSPORT_
  STATIONS = 64` becomes `frame_data.refinement_level` fed through; the
  default stays 64 so landed behavior is bit-identical at the default).
- Changing the recorded level changes the surface — BY DESIGN, recorded in
  the doc (`frame_data_is_resolution_independent_once_frozen` test: same
  level twice -> byte-identical sample positions; different level ->
  different positions, both documented).

## Section 3 — the promotion contract (§5.2's load-bearing sentence)

A general B-spline spine becomes a working surface — NOT refused for
promotion. `SpineFrameSurface`'s evaluator consumes `Spine::General`
through `SpineCurve` exactly as it consumed the trait. The
`general_spine_becomes_certifiedpatch_not_refused_for_promotion` test
pins: a cubic B-spline spine sweep constructs, samples, and (via the
landed decorator path) realizes; NURBS/STEP export of a general spine is a
CERTIFIED APPROXIMATION with a declared representation bound — a doc
comment + a `representation_bound: Option<f64>` field on the export-path
struct is the v1 shape; the bound is declared, never silently absent.

`polyline_spine_refuses_as_not_c1_through_the_enum`: the landed
declaration-based SpineNotC1 refusal surfaces through `Spine::General`
construction (`ConstructError::SpineNotC1` — the landed vocabulary; no new
refusal kind).

## Done-when

- `cargo test -p truck-geometry -p truck-modeling --lib --tests
  --no-fail-fast` green — landed suites unchanged (the default FrameData
  keeps bit-identical behavior).
- `cargo check --workspace --all-targets` green (the trait rename ripples;
  every consumer is in the write set or the packet stops).
- fmt + clippy (exact verify form, unfiltered, ALL findings) clean on
  packet files.
- RESULT.json AT THE WORKTREE ROOT.

## Stop conditions

1. The trait rename ripples to a crate/file outside the write set — stop,
   name the site; the write set grows by amendment, not improvisation.
2. The RRMF-quintic rational-RMF closed form cannot be implemented from
   the characterization without a reference formula the packet does not
   supply — stop and say exactly what is missing; do not approximate the
   frame with double reflection for the PH path (that would erase the fast
   path's reason to exist).
3. Default-level FrameData changes any landed test's sampled output —
   stop; the bit-identity premise is broken and needs the orchestrator,
   not a tolerance.

Commit subject: `feat(geometry): Spine enum + PH fast path + FrameData
(BG-KV2-203-C1DELTA)`.
