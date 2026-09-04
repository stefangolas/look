# CC-011-LOFT-WEIGHTS — L1r: certified positive weight field

CC program Phase B (spine S8 consumer; theory §2.2 L1r). Collocation is not
weight-preserving and the inverse of a totally positive matrix has a
checkerboard sign pattern, so strictly positive input weights can yield
negative interpolated `w_ij` — a pole inside the domain. Certify positivity
of the delivered weight field, or refuse.

```yaml
id:          CC-011-LOFT-WEIGHTS
contract:    [CC-011-LOFT-WEIGHTS]
class:       mechanical
crates:      [truck-certified]
depends_on:  [CC-000-CONTRACT, CC-010-LOFT-CORE]
write_allow:
  - vendor/truck/truck-certified/src/construct/loft_weights.rs
  - vendor/truck/truck-certified/src/construct/mod.rs
  - vendor/truck/truck-certified/tests/construct_loft_weights.rs
read_allow:
  - docs/CERTIFIED_CONSTRUCTION_CONTRACTS.md
  - vendor/truck/truck-certified/src/construct
  - vendor/truck/truck-certified/src/hull.rs
budget:      {turns: 18, ctx_tokens: 80000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub fn hull_bernstein_2d' vendor/truck/truck-certified/src/hull.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub struct LoftOutput' vendor/truck/truck-certified/src/construct/loft.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'weight_straddles_zero' vendor/truck/truck-certified/src/kernel/fixtures.rs"}
tests_required:
  - all_positive_control_weights_admit_without_subdivision
  - straddling_weight_field_refuses_non_positive_weight_field
  - refinement_budget_exhaustion_refuses_non_positive_weight_field
  - certified_net_is_the_refined_net_never_the_coarse_one
```

Section 1: `construct/loft_weights.rs` — `pub struct WeightCert { pub
min_control_weight: f64, pub refined: bool }` and `pub fn
certify_weight_field(surface: &BSplineSurface<Point4>, budget: &mut Budget)
-> Result<WeightCert, ConstructRefusal>`. Fast path (free, sufficient):
`min w_ij > 0` over the control net in row-major order → admit, `refined:
false`. Fallback: extract each Bézier patch of the weight field and run
`hull_bernstein_2d` (A1); all-strictly-positive coefficients on a patch →
that patch admits (convex-hull property). Patches that straddle zero are
subdivided (budgeted: each split spends one `budget.spend_subdiv()`; depth
cap `CC_DEPTH_MAX`). Budget exhaustion, or any patch whose enclosure stays
straddling at the cap, or any certified zero →
`Err(ConstructRefusal::NonPositiveWeightField)`. Never admit a straddling
patch; never report a negative weight as a failure of GEOMETRY (it is a
failure of THIS field's admissibility — the refusal says exactly that).

Section 2: the storage rule, restated as a HARD structural rule (theory
D4-clause-(a) lineage): a certificate produced under refinement is valid
ONLY if the identical knot insertions are applied to the shipped surface.
Pre-made mechanism: `certify_weight_field` does not mutate its input; it
returns the refined knot insertions inside `WeightCert` as `pub
refinements: Vec<(bool, usize, f64)>` (axis, span, knot — the
`add_uknot`/`add_vknot` argument triple) and the test
`certified_net_is_the_refined_net_never_the_coarse_one` applies them and
re-checks positivity on the refined net. The caller (CC-012/CC-014) applies
`refinements` to the shipped net; applying them is a CC-012 obligation,
booked there.

House rules: **H-1: no `unwrap`/`expect`/`panic!` in shipped code, no
module-level `allow`.** **H-3: float comparisons in tests take the `// H-3`
opt-out ON THE SAME LINE.** **All cargo invocations go through the queue
(the `cargo` on PATH IS the queue shim). Do not invoke cargo by absolute
path; do not unset the shim.** Scoped checks only: `cargo check -p
truck-certified` and `cargo test -p truck-certified --test
construct_loft_weights`. No workspace builds. The `pub mod loft_weights;`
line in `construct/mod.rs` is the DESIGNED one-line conflict. COMMIT BEFORE
writing RESULT.json AT THE WORKTREE ROOT.

Stop conditions: (1) the landed kernel fixture `weight_straddles_zero` (A3)
documents the straddling-field ground truth pattern — read it before
building the loft-side straddling fixture in the test; (2) if
`hull_bernstein_2d` cannot be applied to a homogeneous weight extraction
directly, extract weights to a plain `BSplineSurface<f64>`-shaped grid and
hull that — do not modify `hull.rs`; (3) subdivision must be dyadic
(midpoint splits only) so refinement knots are exactly representable —
record this in the module doc.
