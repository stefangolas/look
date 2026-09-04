# CC-002-INJECTIVITY â€” P2: local injectivity radius Î´ = 2Ïƒ/L

CC program Phase A (spine S4). Theory:
`docs/CERTIFIED_LOFT_AND_SHELL_THEORY_SPEC.md` Â§1 P2. This is the primitive
that makes self-contact testing terminate: no contact test is required for
parameter pairs with â€–pâˆ’qâ€– &lt; Î´. Consumers: loft L5 near-diagonal
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
  - vendor/truck/truck-certified/src/certified_map.rs
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
  - {id: A5, expect: 1, cmd: "grep -c 'pub fn flat_patch' vendor/truck/truck-certified/src/construct/fixtures.rs"}
tests_required:
  - flat_patch_yields_infinite_radius
  - curved_patch_radius_is_a_certified_lower_bound
  - degenerate_patch_refuses_invalid_input
  - curve_radius_on_unit_circle_matches_two
  - radius_shrinks_monotonically_under_region_refinement
```

Section 1: `construct/injectivity.rs` â€” surface variant per spine S4:
`pub fn injectivity_radius(map: &CertifiedSurfaceMap, sub: SurfaceRegion) ->
Result<Interval, ConstructRefusal>`. Ïƒ = the certified lower bound from
`map.rank_margin(sub)` (A1: two impls â€” surface and curve â€” use the surface
one here). L = sup over the region of â€–DÂ²Sâ€–: decompose the region over the
map's BÃ©zier patches (`admit_surface` already landed the decomposition;
`patch_grids`/`patch_boxes` are the accessors) and bound each second
partial with `hull::bernstein_derivative_2d` (A2) on the control grid;
L = max over patches of the interval norm of the three second partials
composed into a componentwise bound, then take the max â€–Â·â€– over the
enclosure (fixed accumulation order). Î´ = 2Ïƒ/L as an `Interval`. Pre-made
decisions: Ïƒ â‰¤ 0 â†’ `Err(ConstructRefusal::InvalidInput)` (a degenerate
parameterization is an input defect, and the map's own admit-time check is
`ParameterizationDegenerate` â€” do not conflate the two); L = 0 (flat patch)
â†’ Î´ = `Interval` at +âˆž (lo = hi = f64::INFINITY), documented â€” a flat patch
has no curvature-driven self-contact; non-finite intermediate â†’ refuse
`InvalidInput`, never propagate NaN.

Section 2: seam amendment (session 51, QUESTION.md accepted verbatim): CertifiedCurveMap gains the D-map structural accessor 'pub fn piece_grids(&self) -> Vec<[Vec<f64>; 3]>' returning the per-piece, per-coordinate Bernstein coefficient vectors in piece_intervals order — mirror of the surface patch_grids(), landed IN THIS PACKET inside certified_map.rs. The curve variant consumes it for the second-derivative hull. Curve variant per spine S4: `pub fn
curve_injectivity_radius(map: &CertifiedCurveMap, sub: CurveRegion) ->
Result<Interval, ConstructRefusal>` â€” Ïƒ from the curve `rank_margin` (|
Câ€²| lower bound), L from the second-derivative bound (Bernstein derivative
1-D hull over the curve's BÃ©zier pieces), Î´ = 2Ïƒ/L, same refusal and flat
(L=0) conventions.

Section 3: monotonicity gate (test
`radius_shrinks_monotonically_under_region_refinement`): splitting a region
in half must never INCREASE the certified Î´ computed over either half
beyond the parent's Î´ up to enclosure width â€” this is the property that
makes the primitive sound under subdivision, and it is the test that would
catch an unsound sup/inf swap. Ground truth for the circle fixture: unit
circle Ïƒ = 1, â€–Câ€³â€– = 1, Î´ = 2 (fixture `curved_patch`/circle data from
CC-000; assert the enclosure contains 2 and its width shrinks with
refinement â€” H-3 opt-out on the comparison lines).

House rules: **H-1: no `unwrap`/`expect`/`panic!` in shipped code, no
module-level `allow`.** **H-3: float comparisons in tests take the `// H-3`
opt-out ON THE SAME LINE.** **All cargo invocations go through the queue
(the `cargo` on PATH IS the queue shim). Do not invoke cargo by absolute
path; do not unset the shim.** Scoped checks only: `cargo check -p
truck-certified` and `cargo test -p truck-certified --test
construct_injectivity`. No workspace builds. The `pub mod injectivity;`
line in `construct/mod.rs` is the DESIGNED one-line conflict. COMMIT BEFORE
writing RESULT.json AT THE WORKTREE ROOT.

Stop conditions: (1) read `certified_map.rs` first â€” the map types own the
BÃ©zier decomposition; if the second-derivative grid is NOT reachable
through the landed accessors, STOP and file QUESTION.md (that is a spine
seam defect, not something to work around by re-deriving decomposition);
(2) if `hull::bernstein_derivative_2d` cannot bound a second partial over a
patch box directly, read its signature and compose per its contract â€” do
not write a new hull kernel; (3) record in RESULT notes the actual L
computation order for the curved fixture.
