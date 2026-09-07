# WORK PACKET DEF-SPINEFRAME-GRAZE â€” pad the profile-v domain check to the landed s-axis discipline

The boolean funnel's Newton searches evaluate `SpineFrameSweep` at
window-grazing iterates (measured witness: v = -1.2138e-6, s = 2.3875e-4),
and the profile-law v-domain check refuses EXACTLY where every s-axis check
in the same evaluator stack already refuses only beyond a named tolerance.
The panic (H-1: a panic reachable from untrusted geometry, at
decorators/spine_frame.rs:443 via the sanctioned match-unwrap) kills ~10
tests across showcases. Defect record: docs/defects/SEM-SPINEFRAME-WINDOW-GRAZE-001.md
(READ IT FIRST â€” it carries the full audit).

```yaml
id:          DEF-SPINEFRAME-GRAZE
contract:    [DEF-SPINEFRAME-GRAZE]
class:       design
crates:      [truck-geometry]
depends_on:  []
write_allow:
  - vendor/truck/truck-geometry/src/constructive/profile.rs
  - vendor/truck/truck-geometry/src/constructive/errors.rs
  - vendor/truck/truck-geometry/src/decorators/spine_frame.rs
  - vendor/truck/truck-geometry/src/constructive/mod.rs
read_allow:
  - vendor/truck/truck-geometry/src/
  - vendor/truck/truck-shapeops/src/boolean/
  - vendor/truck/truck-geotrait/src/algo/surface.rs
  - docs/defects/SEM-SPINEFRAME-WINDOW-GRAZE-001.md
tests_required:
  - a new regression test evaluating a SpineFrameSweep at a
    tolerance-grazing v below 0 returns typed Ok (search-path doctrine)
    and refuses typed InvalidInput beyond the named pad
  - showcases battery_waterslide + pb_contract suites green
anchors:
  - {id: A1, expect: 1, cmd: "grep -c '!(0.0..=1.0).contains' vendor/truck/truck-geometry/src/constructive/profile.rs"}
  - {id: A2, expect: 5, cmd: "grep -c 'panic!' vendor/truck/truck-geometry/src/decorators/spine_frame.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'DirectTolerance::parameter' vendor/truck/truck-geometry/src/constructive/recipe.rs"}
budget:      {turns: 60, ctx_tokens: 180000}
```

## Problem â€” the mechanism (audit-verified)

- `ProfileLaw::evaluate` (constructive/profile.rs:21) refuses
  `ConstructError::InvalidInput` on `!(0.0..=1.0).contains(&v)` â€” EXACT,
  tolerance-free. Every s-axis sibling check is padded:
  spine_frame.rs:53/64/268, recipe.rs:246/257 all use
  `DirectTolerance::parameter` (1.0e-6). The v axis is the anomaly.
- Refusal chain: `SpineFrameSweep::subs` (sweep_surface.rs:141) â†’
  `evaluate_position` (spine_frame.rs:435-445, sanctioned match-unwrap
  whose doc premise is "inside the validated window") â†’
  `SpineFrameRecipe::position` (recipe.rs:359) â†’ profile law.
- The caller evaluating out-of-window is the SHAPEOPS BOOLEAN FUNNEL
  (BIE-006 admitted SpineFrameSweep faces at `417cdce`): unclamped Newton
  iterates in truck-geotrait `algo::surface::search_parameter`
  (surface.rs:278-299) evaluate `subs/uder/vder` at raw iterates; call
  sites: sweep_lift.rs:122, classify.rs:412/443/505/528,
  split.rs:1221/2014/2036/2108/2119, divide_face:36/45,
  loops_store:509-511, healing:55-109.
- The measured witness overshoots by 1.21x DirectTolerance::parameter, so
  a pad of exactly 1e-6 does NOT rescue this iterate â€” size the pad
  against the witness class and document it.

## Scope decisions â€” pre-made, do not relitigate

1. **Fix at the evaluator, option (c)**: pad the v-domain refusal at ALL
   THREE sites â€” profile.rs:21 (`ProfileLaw::evaluate`),
   spine_frame.rs:156 (`profile_derivative_v`), spine_frame.rs:176
   (`profile_derivative_s`) â€” with ONE named H-3 constant (e.g.
   `PROFILE_V_DOMAIN_PAD`), typed `InvalidInput` beyond it. Fixing
   position alone moves the panic to the derivative sites (the same Newton
   call evaluates uder/vder).
2. **Do NOT** pad the constructor's `validate_surface_window` (it is
   correct; widening it would hide a caller-side problem), do NOT clamp
   inside `evaluate_position` (masks construction bugs; the evaluators are
   infallible by design), do NOT touch the frozen seam
   (`c2_certify_tube4`/`build_frame4`) or the boolean funnel.
3. Optional doctrine-clean hardening if trivial: clamp the SEARCH HINT
   into the window in SpineFrameSweep/SpineFrameSurface::search_parameter
   (a hint is not a result). Skip if it complicates.
4. `ring_point` is linear in v: a boundary-grazing evaluation extrapolates
   linearly â€” no discontinuity; the search's own residual gate
   (`ctx.near_points`) still decides acceptance. Put this justification
   in the doc comment of the new constant.
5. H-3: the constant's defining line carries `// H-3` and names the
   quantity. No other tolerance sites change.

## Tests required

1. New regression test (constructive or decorators tests): evaluate a
   SpineFrameSweep at v = -0.5*tol and v = -2*tol â€” first returns Ok, second
   refuses InvalidInput typed; same for the derivative fns.
2. All 10 recorded showcase tests flip green (see the defect record Â§5).

## Done when

```
cargo test -p truck-geometry --lib
cargo test -p showcases --tests
cargo test -p truck-geometry --test constructive_spine_enum (the 3 frame-law tests only)
```

## Forbidden

vendor/truck outside the write_allow. Weakening any landed assertion.
Silent clamps. Panic removal by unwrapping differently. Touching
truck-shapeops/truck-geotrait (read-only: the funnel stays as-is).

## Stop conditions

- The pad rescues the witness but any showcase test STILL dies at 443 â†’
  a second site exists: diagnose, extend the packet via orchestrator
  amendment (record in RESULT, do not widen silently).
- Any landed kernel test flips red from the pad â†’ SPEC_GAP.

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `fix(geometry): profile-v domain check joins the padded-s discipline, named H-3 pad sized to the search-iterate witness (DEF-SPINEFRAME-GRAZE)`.
