# CC-010-LOFT-CORE — Loft construction: compatibility, stationing, collocation

CC program Phase B (spine S8). Theory:
`docs/CERTIFIED_LOFT_AND_SHELL_THEORY_SPEC.md` §2.1–2.2 (L0, L1, L2).
Construction is exact algebra; the artifact is the P1 enclosure. No new
carrier: the output is an ordinary tensor-product B-spline surface over the
landed `truck_geometry::nurbs` types.

```yaml
id:          CC-010-LOFT-CORE
contract:    [CC-010-LOFT-CORE]
class:       design
crates:      [truck-certified]
depends_on:  [CC-000-CONTRACT, CC-001-BANDED]
write_allow:
  - vendor/truck/truck-certified/src/construct/loft.rs
  - vendor/truck/truck-certified/src/construct/mod.rs
  - vendor/truck/truck-certified/tests/construct_loft.rs
read_allow:
  - docs/CERTIFIED_CONSTRUCTION_CONTRACTS.md
  - vendor/truck/truck-certified/src/construct
  - vendor/truck/truck-geometry/src/nurbs
budget:      {turns: 24, ctx_tokens: 90000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub fn factor_banded_tp' vendor/truck/truck-certified/src/construct/banded.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn solve_homogeneous' vendor/truck/truck-certified/src/construct/banded.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub fn elevate_degree' vendor/truck/truck-geometry/src/nurbs/bspcurve.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub fn add_knot' vendor/truck/truck-geometry/src/nurbs/bspcurve.rs"}
tests_required:
  - averaged_knot_vector_satisfies_schoenberg_whitney_on_increasing_stations
  - chord_length_stations_are_deterministic_and_normalized
  - loft_reproduces_sections_identically_up_to_epsilon
  - delivered_epsilon_matches_max_control_error
  - knot_union_is_exact_additive_never_tolerance_merged
  - incompatible_sections_refuse
```

Section 1: compatibility (theory §2.1 step 1, pre-made) — `pub fn
make_compatible(sections: &[BSplineCurve<Point4>]) ->
Result<Vec<BSplineCurve<Point4>>, ConstructRefusal>`: exact degree
elevation to `p = max_k p_k` (`elevate_degree`, A3) then exact knot-vector
union by knot INSERTION (`add_knot`, A4). Knot equality is exact `f64`
value equality — no tolerance merging anywhere; unequal near-equal knots
are BOTH retained and BOTH inserted (the old v2 spec's H1 rule, restated as
binding). Empty input → `InvalidInput`; a section whose knot vector cannot
be unioned (unclamped) → `InvalidInput`. Cost is additive in total knots —
state it in the module doc.

Section 2: stationing (theory §2.1, pre-made) — `pub fn
chord_length_stations(sections: &[BSplineCurve<Point4>]) -> Vec<f64>`:
accumulate per-section polyline chord lengths in section order with a FIXED
summation order (sequential f64 adds over sampled edges at the map's
default sampling; the sample count is a named const in this module, not a
parameter), then normalize by dividing by the total. `pub fn
averaged_knot_vector(stations: &[f64], degree: usize) -> KnotVec` — de Boor
averaging `ξ_{j+q} = (1/q)·Σ_{r=j}^{j+q−1} v_r` with clamped ends repeated
q+1 times, accumulation in the fixed order `j..j+q−1`. Strictly increasing
stations are a caller precondition; violations → `InvalidInput`.

Section 3: the solve per spine S8 — build the collocation band storage
`A_{kj} = M_{j,q}(v_k)` in a fixed evaluation order, factor through
`factor_banded_tp` (A1; a refusal there means the stationing policy
produced a Schoenberg–Whitney violation — propagate as
`SingularInterpolationSystem`, never fall back to a dense or pivoting
solve), and `solve_homogeneous` (A2) all control rows in one call. Output
`pub struct LoftOutput { pub surface: BSplineSurface<Point4>, pub epsilon:
f64 }` where `epsilon = factor.max_control_error()` (L2: downstream
predicates consume ε; they may not assume exactness). Assemble the
tensor-product net from the solved rows in row-major order.

Section 4: L1 gate test (`loft_reproduces_sections_identically_up_to_
epsilon`): evaluate the delivered surface at each station `v_k` over a
sample grid in u and assert the deviation from the input section is ≤ ε
(H-3 opt-outs). Ground-truth fixture built IN THE TEST from the landed
nurbs types: three cubic clamped sections with known control points — the
interpolated net's station rows must equal the section control points up
to ε.

House rules: **H-1: no `unwrap`/`expect`/`panic!` in shipped code, no
module-level `allow`.** **H-3: float comparisons in tests take the `// H-3`
opt-out ON THE SAME LINE.** **All cargo invocations go through the queue
(the `cargo` on PATH IS the queue shim). Do not invoke cargo by absolute
path; do not unset the shim.** Scoped checks only: `cargo check -p
truck-certified` and `cargo test -p truck-certified --test construct_loft`.
No workspace builds. The `pub mod loft;` line in `construct/mod.rs` is the
DESIGNED one-line conflict. COMMIT BEFORE writing RESULT.json AT THE
WORKTREE ROOT.

Stop conditions: (1) if `BSplineCurve<Point4>` arithmetic needs operators
the landed nurbs types do not provide, write free functions in this module
— do NOT modify `truck-geometry`; (2) if knot union over multi-section
input produces a knot vector `try_bspline_basis_functions` cannot
evaluate, file QUESTION.md with the failing knot vector — that is a landed
substrate seam; (3) record the chosen polyline sample count and the exact
summation order in RESULT notes (reproducibility contract).
