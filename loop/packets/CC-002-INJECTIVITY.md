# CC-002-INJECTIVITY — P2: local injectivity radius δ = 2σ/L

CC program Phase A (spine S4). Theory:
`docs/CERTIFIED_LOFT_AND_SHELL_THEORY_SPEC.md` §1 P2. This is the primitive
that makes self-contact testing terminate: no contact test is required for
parameter pairs with ‖p−q‖ &lt; δ. Consumers: loft L5 near-diagonal
discharge (CC-014), offset star certificates (CC-021/022), blend spine
self-intersection (CC-030).

```yaml
id:          CC-002-INJECTIVITY
contract:    [CC-002-INJECTIVITY]
class:       mechanical
crates:      [truck-certified]
depends_on:  [CC-000-CONTRACT]
write_allow:
  - vendor/truck/truck-certified/src/construct/injectivity.rs
  - vendor/truck/truck-certified/src/construct/mod.rs
  - vendor/truck/truck-certified/tests/construct_injectivity.rs
read_allow:
  - docs/CERTIFIED_CONSTRUCTION_CONTRACTS.md
  - vendor/truck/truck-certified/src/construct
  - vendor/truck/truck-certified/src/certified_map.rs
  - vendor/truck/truck-certified/src/hull.rs
budget:      {turns: 18, ctx_tokens: 80000}
anchors:
  - {id: A1, expect: 2, cmd: "grep -c 'pub fn rank_margin' vendor/truck/truck-certified/src/certified_map.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn bernstein_derivative_2d' vendor/truck/truck-certified/src/hull.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub fn admit_surface(' vendor/truck/truck-certified/src/certified_map.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub fn admit_curve(' vendor/truck/truck-certified/src/certified_map.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'flat_patch' vendor/truck/truck-certified/src/construct/fixtures.rs"}
tests_required:
  - flat_patch_yields_infinite_radius
  - curved_patch_radius_is_a_certified_lower_bound
  - degenerate_patch_refuses_invalid_input
  - curve_radius_on_unit_circle_matches_two
  - radius_shrinks_monotonically_under_region_refinement
```

Section 1: `construct/injectivity.rs` — surface variant per spine S4:
`pub fn injectivity_radius(map: &CertifiedSurfaceMap, sub: SurfaceRegion) ->
Result<Interval, ConstructRefusal>`. σ = the certified lower bound from
`map.rank_margin(sub)` (A1: two impls — surface and curve — use the surface
one here). L = sup over the region of ‖D²S‖: decompose the region over the
map's Bézier patches (`admit_surface` already landed the decomposition;
`patch_grids`/`patch_boxes` are the accessors) and bound each second
partial with `hull::bernstein_derivative_2d` (A2) on the control grid;
L = max over patches of the interval norm of the three second partials
composed into a componentwise bound, then take the max ‖·‖ over the
enclosure (fixed accumulation order). δ = 2σ/L as an `Interval`. Pre-made
decisions: σ ≤ 0 → `Err(ConstructRefusal::InvalidInput)` (a degenerate
parameterization is an input defect, and the map's own admit-time check is
`ParameterizationDegenerate` — do not conflate the two); L = 0 (flat patch)
→ δ = `Interval` at +∞ (lo = hi = f64::INFINITY), documented — a flat patch
has no curvature-driven self-contact; non-finite intermediate → refuse
`InvalidInput`, never propagate NaN.

Section 2: curve variant per spine S4: `pub fn
curve_injectivity_radius(map: &CertifiedCurveMap, sub: CurveRegion) ->
Result<Interval, ConstructRefusal>` — σ from the curve `rank_margin` (|
C′| lower bound), L from the second-derivative bound (Bernstein derivative
1-D hull over the curve's Bézier pieces), δ = 2σ/L, same refusal and flat
(L=0) conventions.

Section 3: monotonicity gate (test
`radius_shrinks_monotonically_under_region_refinement`): splitting a region
in half must never INCREASE the certified δ computed over either half
beyond the parent's δ up to enclosure width — this is the property that
makes the primitive sound under subdivision, and it is the test that would
catch an unsound sup/inf swap. Ground truth for the circle fixture: unit
circle σ = 1, ‖C″‖ = 1, δ = 2 (fixture `curved_patch`/circle data from
CC-000; assert the enclosure contains 2 and its width shrinks with
refinement — H-3 opt-out on the comparison lines).

House rules: **H-1: no `unwrap`/`expect`/`panic!` in shipped code, no
module-level `allow`.** **H-3: float comparisons in tests take the `// H-3`
opt-out ON THE SAME LINE.** **All cargo invocations go through the queue
(the `cargo` on PATH IS the queue shim). Do not invoke cargo by absolute
path; do not unset the shim.** Scoped checks only: `cargo check -p
truck-certified` and `cargo test -p truck-certified --test
construct_injectivity`. No workspace builds. The `pub mod injectivity;`
line in `construct/mod.rs` is the DESIGNED one-line conflict. COMMIT BEFORE
writing RESULT.json AT THE WORKTREE ROOT.

Stop conditions: (1) read `certified_map.rs` first — the map types own the
Bézier decomposition; if the second-derivative grid is NOT reachable
through the landed accessors, STOP and file QUESTION.md (that is a spine
seam defect, not something to work around by re-deriving decomposition);
(2) if `hull::bernstein_derivative_2d` cannot bound a second partial over a
patch box directly, read its signature and compose per its contract — do
not write a new hull kernel; (3) record in RESULT notes the actual L
computation order for the curved fixture.
