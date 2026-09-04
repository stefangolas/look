# CC-033-SETBACK — n-valent corner setback patches, certified on four counts

CC program Phase D (spine S6/S3 consumers; theory §5.5). For a genuine
n-valent corner the rolling-ball system has no degrees of freedom left
(theory §3.3) — the answer is a setback vertex blend whose corner region is
naturally 2n-sided (P_i profile cuts across incoming fillets, Q_i spring
curves on surviving primary faces). The Hermite ribbon construction is
UNTRUSTED; the patch is certified on four counts: boundary, G¹ ribbons,
local regularity, global embeddedness (P3).

```yaml
id:          CC-033-SETBACK
contract:    [CC-033-SETBACK]
class:       design
crates:      [truck-certified]
depends_on:  [CC-000-CONTRACT, CC-001-BANDED, CC-005-GRAPHDISK, CC-030-BLEND-SPINE]
write_allow:
  - vendor/truck/truck-certified/src/construct/setback.rs
  - vendor/truck/truck-certified/src/construct/mod.rs
  - vendor/truck/truck-certified/tests/construct_setback.rs
read_allow:
  - docs/CERTIFIED_CONSTRUCTION_CONTRACTS.md
  - vendor/truck/truck-certified/src/construct
budget:      {turns: 26, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub fn solve_homogeneous' vendor/truck/truck-certified/src/construct/banded.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn residual_solve_dense' vendor/truck/truck-certified/src/construct/residual_solve.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub fn certify_graph_disk' vendor/truck/truck-certified/src/construct/graphdisk.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub struct BlendTrace' vendor/truck/truck-certified/src/construct/blend.rs"}
tests_required:
  - boundary_matches_prescribed_profiles_and_springs
  - g1_ribbon_conditions_hold_with_positive_lambda
  - regularity_margin_holds_on_the_whole_patch
  - projection_exhaustion_falls_back_and_still_certifies_or_refuses
```

Section 1: the construction (untrusted, deterministic) — setback split
with Hermite ribbon patches per boundary arc: `pub struct SetbackInput`
(the 2n boundary curves with exact tangent-plane data from the incoming
fillets and surviving faces — carried in from the `BlendTrace` (A4) and
the incident strata), `pub fn build_setback_patch(input: &SetbackInput) ->
Result<SetbackPatch, ConstructRefusal>` builds the Hermite ribbons through
the P1 machinery: the ribbon interpolation systems are dense (NOT banded-TP)
→ `residual_solve_dense` (A2) is the certified solve; where a system IS
banded-TP-shaped, `factor_banded_tp`/`solve_homogeneous` (A1) is the fast
path. The construction never guesses: an unsolvable ribbon system refuses.

Section 2: the four certification counts, each a named check on the built
patch, all PRE-MADE: (1) boundary — each outer patch boundary equals the
prescribed P_i/Q_i up to the delivered enclosure ε (test 1); (2) G¹
ribbons — on each boundary, P_v(u,0) = λ(u)·d(u) with λ(u) &gt; 0 certified
and d(u) in the adjacent tangent plane (test 2 asserts λ's enclosure is
strictly positive — fold-back prevention is part of the certificate, not a
separate check); (3) local regularity — inf ‖P_u × P_v‖ ≥ `CC_ETA_J` over
the patch domain via the CC-002 hull path on the patch's Bézier form
(test 3); (4) global embeddedness — `certify_graph_disk` (A3) over the
whole 2n-sided region with the normative projection search; exhaustion →
the PAIRWISE fallback (patch/patch SSI through the manifest edge, boundary
intersection exclusion, regularity, inside/outside witness) and still
certify or refuse `NoAdmissibleProjection` (test 4 exercises the fallback
path).

House rules: **H-1: no `unwrap`/`expect`/`panic!` in shipped code, no
module-level `allow`.** **H-3: float comparisons in tests take the `// H-3`
opt-out ON THE SAME LINE.** **All cargo invocations go through the queue
(the `cargo` on PATH IS the queue shim). Do not invoke cargo by absolute
path; do not unset the shim.** Scoped checks only: `cargo check -p
truck-certified` and `cargo test -p truck-certified --test
construct_setback`. No workspace builds. The `pub mod setback;` line in
`construct/mod.rs` is the DESIGNED one-line conflict. COMMIT BEFORE writing
RESULT.json AT THE WORKTREE ROOT.

Stop conditions: (1) the setback SPLIT (where to cut P_i across incoming
fillets) is deterministic here — a heuristic setback distance is a spec
amendment, not an implementation choice; record the rule used in the
module doc; (2) n = 3 with a genuine triple node routes through CC-020's
node, not a setback patch — setback is for n ≥ 4 or degenerate triples;
assert that routing in a test comment at minimum; (3) if the Hermite
ribbons cannot be made polynomial-solvable for the v1 boundary data, file
QUESTION.md — the boundary data classes are closed.
