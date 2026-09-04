# CC-015-GORDON — Gordon surface as a construction algorithm, no new carrier

CC program Phase B (spine S8 consumer; theory §2.4). If both profile and
guide curves are supplied, S = S_u + S_v − S_uv with cardinal functions
satisfying φ_j(u_i) = δ_ij, ψ_j(v_i) = δ_ij; the correction term removes the
doubly-counted network intersections exactly. Gordon introduces no new
certification obligations beyond basis compatibility of the three
components; the output is certified by CC-014 like any other surface.

```yaml
id:          CC-015-GORDON
contract:    [CC-015-GORDON]
class:       mechanical
crates:      [truck-certified]
depends_on:  [CC-000-CONTRACT, CC-010-LOFT-CORE]
write_allow:
  - vendor/truck/truck-certified/src/construct/gordon.rs
  - vendor/truck/truck-certified/src/construct/mod.rs
  - vendor/truck/truck-certified/tests/construct_gordon.rs
read_allow:
  - docs/CERTIFIED_CONSTRUCTION_CONTRACTS.md
  - vendor/truck/truck-certified/src/construct
budget:      {turns: 16, ctx_tokens: 70000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub fn make_compatible' vendor/truck/truck-certified/src/construct/loft.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn loft_sections' vendor/truck/truck-certified/src/construct/loft.rs"}
  - {id: A3, expect: 2, cmd: "grep -c 'pub fn rank_margin' vendor/truck/truck-certified/src/certified_map.rs"}
tests_required:
  - cardinal_functions_are_exactly_delta_at_stations
  - correction_term_removes_double_counting_at_network_points
  - output_passes_the_same_validity_postcondition
  - incompatible_component_bases_refuse
```

Section 1: `construct/gordon.rs` — `pub struct GordonInput { pub profiles:
Vec<BSplineCurve<Point4>>, pub guides: Vec<BSplineCurve<Point4>>, pub
stations_u: Vec<f64>, pub stations_v: Vec<f64> }` and `pub fn
gordon_surface(input: &GordonInput) -> Result<LoftOutput,
ConstructRefusal>` (the CC-010 output type — no new carrier, deliberately).
Pre-made steps, in order: (1) all three component families through
`make_compatible` (A1) to one u-basis for profiles and one v-basis for
guides; (2) the profile loft S_u and guide loft S_v through `loft_sections`
(A2) over the shared factorization where the stationing matches; (3) the
correction surface S_uv: interpolate AT the network points (u_i, v_j) the
difference values, reusing the SAME cached collocation factorizations for
both directions — the complexity claim depends on this reuse, so the code
must structure it (one factorization per direction, asserted by identical
`epsilon` on S_u and the correction's u-direction solve). (4) combine
control nets pointwise: S = S_u + S_v − S_uv in homogeneous R4 with fixed
accumulation order.

Section 2: the cardinal gate (`cardinal_functions_are_exactly_delta_at_
stations`): the interpolated correction surface evaluated at (u_i, v_j)
must reproduce the network point up to ε; the correction term evaluated at
a network point must equal the amount double-counted by S_u + S_v there
(test 2 asserts the SUM S_u + S_v − S_uv equals the expected cross-boundary
value at network points up to ε, H-3 opt-outs). Basis incompatibility →
`InvalidInput` (test 4), never silent re-basis.

House rules: **H-1: no `unwrap`/`expect`/`panic!` in shipped code, no
module-level `allow`.** **H-3: float comparisons in tests take the `// H-3`
opt-out ON THE SAME LINE.** **All cargo invocations go through the queue
(the `cargo` on PATH IS the queue shim). Do not invoke cargo by absolute
path; do not unset the shim.** Scoped checks only: `cargo check -p
truck-certified` and `cargo test -p truck-certified --test
construct_gordon`. No workspace builds. The `pub mod gordon;` line in
`construct/mod.rs` is the DESIGNED one-line conflict. COMMIT BEFORE writing
RESULT.json AT THE WORKTREE ROOT.

Stop conditions: (1) this packet is mechanical: every mathematical object
already exists from CC-010 — if you need a new interpolation kernel, the
decomposition is wrong, re-read theory §2.4; (2) profiles and guides are
independent families; if the fixture needs them to share a knot vector,
that is the fixture's business (make_compatible each family separately),
not a reason to couple them in code; (3) record in RESULT notes the two
factorization-sharing assertions' observed epsilon values.
