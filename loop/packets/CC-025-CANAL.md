# CC-025-CANAL — canal surface exact regularity, arc-restricted

CC program Phase C (spine S10; theory §6). Canal regularity is a CLOSED
FORM: ‖X_s × X_θ‖ = r·|a² − rq − ra(c″·e_θ)|, and the patch is regular on
its arc iff min over the arc of |a² − rq| > max ra‖c″‖, with a = √(1−r′²).
No fallback tier exists — the only refusal is `CanalSingular`. Consumers:
offset edge strata (CC-021), blend surfaces (CC-030/031).

```yaml
id:          CC-025-CANAL
contract:    [CC-025-CANAL]
class:       mechanical
crates:      [truck-certified]
depends_on:  [CC-000-CONTRACT, CC-002-INJECTIVITY]
write_allow:
  - vendor/truck/truck-certified/src/construct/canal.rs
  - vendor/truck/truck-certified/src/construct/mod.rs
  - vendor/truck/truck-certified/tests/construct_canal.rs
read_allow:
  - docs/CERTIFIED_CONSTRUCTION_CONTRACTS.md
  - vendor/truck/truck-certified/src/construct
  - vendor/truck/truck-certified/src/certified_map.rs
budget:      {turns: 16, ctx_tokens: 70000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub fn injectivity_radius' vendor/truck/truck-certified/src/construct/injectivity.rs"}
  - {id: A2, expect: 2, cmd: "grep -c 'pub fn rank_margin' vendor/truck/truck-certified/src/certified_map.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub enum RadiusLaw' vendor/truck/truck-certified/src/construct/stubs.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub fn bernstein_derivative_1d' vendor/truck/truck-certified/src/hull.rs"}
tests_required:
  - constant_radius_unit_circle_satisfies_pipe_condition
  - radius_law_slope_at_or_above_one_refuses_canal_singular
  - singular_spine_fixture_refuses_canal_singular
  - arc_restriction_is_strictly_more_permissive_than_all_theta
  - radius_law_evaluation_matches_declared_law
```

Section 1: radius-law evaluation — `RadiusLaw` is the CC-000 stub (A3);
this packet adds the PRODUCTION evaluators in `canal.rs` (the stub file is
not touched): `pub fn radius_eval(law: &RadiusLaw, s: Interval) ->
Result<Interval, ConstructRefusal>` and `pub fn radius_derivs(law:
&RadiusLaw, s: Interval) -> Result<(Interval, Interval), ConstructRefusal>`
(r and r′; r″ where the law carries it). Per-law semantics pre-made:
`Constant(c)` → (c, 0); `Linear{r0, r1}` → interpolation with constant
slope (r1 − r0 over the declared arc length carried by the caller's arc);
`CubicHermite{r0, r1, m0, m1}` / `MonotoneCubic(pts)` → Hermite evaluation
on the unit sub-interval; `VertexControl(pts)` → monotone cubic through the
control radii. A law whose |r′| enclosure reaches 1 anywhere in the arc →
`Err(CanalSingular)` immediately (the characteristic circle degenerates —
theory §6.1 first gate).

Section 2: the criterion per spine S10 — `pub fn canal_regularity(spine:
&CertifiedCurveMap, radius: &RadiusLaw, arc: (f64, f64)) -> Result<Interval,
ConstructRefusal>`: over the arc's Bézier pieces (the map's landed
decomposition), bound r, r′, r″ from the law evaluators and ‖c″‖ from the
Bernstein 1-D derivative hulls (A4) exactly as CC-002 bounds second
derivatives; compose a = √(1−r′²) (refuse if the radicand's enclosure is
not strictly positive — same `CanalSingular`), then compute the arc value
`min |a² − rq| − max (r·a)·‖c″‖` with FIXED min/max accumulation order over
pieces. Result > 0 → regular, return the enclosure; enclosure straddling 0
→ `Err(CanalSingular)`; result ≤ 0 → `Err(CanalSingular)` with the
enclosure in the message path. The all-θ variant is a separate `pub fn
canal_regularity_closed_pipe(...)` with the same body minus the arc
restriction — booked here because the closed-pipe consumer needs the
identity, but the arc-restricted fn is the one CC-021/CC-030 call (test 4:
a spine that fails all-θ but passes arc-restricted decides differently —
the permissiveness gap is observed, not assumed).

Section 3: ground truths — unit circle spine, constant radius r &lt; 1: σ
= 1, ‖c″‖ = 1, pipe condition r·‖c″‖ &lt; 1 holds (test 1, H-3 opt-outs);
a linear law with slope ≥ 1 over the arc refuses at the first gate (test
2); the CC-000 `curved_patch`-style singular-spine data (or a
locally-constructed fixture with r·‖c″‖ ≥ 1) refuses `CanalSingular` (test
3). The law evaluators are pinned against their declared semantics on
hand-computed values (test 5).

House rules: **H-1: no `unwrap`/`expect`/`panic!` in shipped code, no
module-level `allow`.** **H-3: float comparisons in tests take the `// H-3`
opt-out ON THE SAME LINE.** **All cargo invocations go through the queue
(the `cargo` on PATH IS the queue shim). Do not invoke cargo by absolute
path; do not unset the shim.** Scoped checks only: `cargo check -p
truck-certified` and `cargo test -p truck-certified --test
construct_canal`. No workspace builds. The `pub mod canal;` line in
`construct/mod.rs` is the DESIGNED one-line conflict. COMMIT BEFORE writing
RESULT.json AT THE WORKTREE ROOT.

Stop conditions: (1) `RadiusLaw` stays closed — evaluator production lands
here, variant growth is a CC-000 amendment; (2) the torsion cancels in the
closed form (theory §6.3): if your derivation needs a Frenet frame or a
torsion term, the composition is wrong — re-read §6.3 and stop; (3) if the
map's Bézier decomposition cannot supply ‖c″‖ bounds directly, reuse
CC-002's approach via its landed module — do not write a second hull path.
