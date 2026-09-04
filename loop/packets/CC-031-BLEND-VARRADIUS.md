# CC-031-BLEND-VARRADIUS — variable radius via foot-point equations

CC program Phase D (spine S10 consumer; theory §5.3). Variable radius adds
a guide curve G and law R with the FOOT-POINT equations
(c − G(λ))·G′(λ) = 0, r − R(λ) = 0: one unknown and two equations replacing
Φ, leaving the solver dimensionality IDENTICAL to the constant-radius case.
No nearest-point projection, no tubular-radius bottleneck computation.

```yaml
id:          CC-031-BLEND-VARRADIUS
contract:    [CC-031-BLEND-VARRADIUS]
class:       design
crates:      [truck-certified]
depends_on:  [CC-000-CONTRACT, CC-030-BLEND-SPINE]
write_allow:
  - vendor/truck/truck-certified/src/construct/blend_varradius.rs
  - vendor/truck/truck-certified/src/construct/mod.rs
  - vendor/truck/truck-certified/tests/construct_blend_varradius.rs
read_allow:
  - docs/CERTIFIED_CONSTRUCTION_CONTRACTS.md
  - vendor/truck/truck-certified/src/construct
budget:      {turns: 22, ctx_tokens: 90000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub fn trace_blend_chain' vendor/truck/truck-certified/src/construct/blend.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub enum RadiusLaw' vendor/truck/truck-certified/src/construct/stubs.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub fn radius_derivs' vendor/truck/truck-certified/src/construct/canal.rs"}
tests_required:
  - foot_point_uniqueness_gate_refuses_when_curvature_product_at_one
  - variable_law_chain_matches_constant_law_at_constant_radius
  - linear_law_radius_follows_the_declared_law
  - guide_global_branch_excluded_by_clearance
```

Section 1: the foot-point gate — uniqueness of λ is local and cheap:
∂_λ[(c−G)·G′] = −‖G′‖² + (c−G)·G″ ≤ −η < 0, i.e. for unit-speed G,
‖c − G‖·κ_G < 1. `pub fn foot_point_gate(map: &CertifiedCurveMap, c:
&[Interval; 3], sub: CurveRegion) -> Result<Interval, ConstructRefusal>`
certifies that bound from the landed map margins (‖G′‖ via rank_margin,
‖G″‖ via the CC-002 hull path) and the ball-centre enclosure; the gate
failing → `Err(ConditioningBelowThreshold)` (the foot point is not locally
unique on this region). The GLOBAL branch — a distant part of G passing near
c — is excluded by P5 (`ball_clearance` through the manifest edge), which
the walk runs regardless (test 4).

Section 2: the amended walk — `pub fn trace_blend_chain_variable(branches:
&[BranchSeed], guide: &CertifiedCurveMap, law: &RadiusLaw, budget: &mut
Budget) -> Result<BlendTrace, ConstructRefusal>`: the CC-030 walk with the
system closed by the foot-point pair instead of Φ. `RadiusLaw` (A2) rides
through the landed `radius_eval`/`radius_derivs` (A3). Pre-made: the
constant law must reduce EXACTLY to the CC-030 system (test 2 runs both
walks on the same fixture and asserts identical event records up to
enclosure width — the dimensionality claim made observable); a linear law
produces radii matching the declared law at each certified step (test 3,
H-3 opt-outs).

House rules: **H-1: no `unwrap`/`expect`/`panic!` in shipped code, no
module-level `allow`.** **H-3: float comparisons in tests take the `// H-3`
opt-out ON THE SAME LINE.** **All cargo invocations go through the queue
(the `cargo` on PATH IS the queue shim). Do not invoke cargo by absolute
path; do not unset the shim.** Scoped checks only: `cargo check -p
truck-certified` and `cargo test -p truck-certified --test
construct_blend_varradius`. No workspace builds. The `pub mod
blend_varradius;` line in `construct/mod.rs` is the DESIGNED one-line
conflict. COMMIT BEFORE writing RESULT.json AT THE WORKTREE ROOT.

Stop conditions: (1) the network optimizer (choosing all radii
simultaneously) is OUT OF SCOPE by theory §5.3 — the kernel answers whether
the REQUESTED law certifies, never invents one; (2) `BlendTrace` and
`EventKind` are frozen — consume, extend nothing; (3) if the foot-point
system cannot stay polynomial for an admissible v1 law, file QUESTION.md —
the admissible law list is closed.
