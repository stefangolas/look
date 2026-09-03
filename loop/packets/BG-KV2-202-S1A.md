# BG-KV2-202-S1A — the R8/R9 residuals: curve-surface and curve-curve, square C1

Wave-2 implementation packet (build spec §4; §19 rows 10). Lands spec §7's
R8 (curve-surface, 3 eq in 3 unknowns) and R9 (curve-curve in one lifted
chart, 2 eq in 2 unknowns) as square C1 residuals implementing the S2A
frozen seam. **The seam is frozen verbatim in BOTH packets (BG-KV2-201-S2A
Section 1): `SquareResidualEval { arity, eval, jac_encl }` +
`krawczyk_c1(g, b, w)`.** If the S2A spelling differs at your fork point,
STOP (stop condition 1) — do not adapt silently.

R8 boundary-stratum seeds (§9.3) and R9 trim crossings (§9.4) are later
waves; this packet delivers the residuals, their constructors, and their C1
certification.

```yaml
id:          BG-KV2-202-S1A
contract:    [BG-KV2-202-S1A]
class:       design
crates:      [truck-certified]
depends_on:  [BG-KV2-201-S2A]
write_allow:
  - vendor/truck/truck-certified/src/kernel/residuals_r89.rs
  - vendor/truck/truck-certified/src/kernel/mod.rs
  - vendor/truck/truck-certified/tests/kernel_r89.rs
read_allow:
  - docs/CONSTRUCTIVE_GEOMETRY_KERNEL_SPEC_V2.md
  - docs/KERNEL_V2_BUILD_SPEC.md
  - vendor/truck/truck-certified/src/kernel
  - vendor/truck/truck-certified/src/formal/bezier_isect.rs
  - vendor/truck/truck-certified/src/kernel/leaf_extract.rs
budget:      {turns: 32, ctx_tokens: 110000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub trait SquareResidualEval' vendor/truck/truck-certified/src/kernel/engine.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn krawczyk_c1' vendor/truck/truck-certified/src/kernel/engine.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub fn extract_bezier_leaves' vendor/truck/truck-certified/src/kernel/leaf_extract.rs"}
  - {id: A4, expect: 0, cmd: "grep -rnw 'R8System' vendor/truck/truck-certified/src | wc -l"}
tests_required:
  - r8_system_builds_from_curve_and_surface_leaves
  - r8_refuses_nonfinite_or_degree_zero_inputs
  - r8_regular_root_certifies_with_point_cert
  - r8_transversality_refusal_when_tangent_to_surface
  - r9_system_builds_from_two_curve_leaves_in_one_chart
  - r9_crossing_certifies_and_non_crossing_disproves
  - homogeneous_evaluation_no_premature_division
  - no_transcendental_call_in_r89_module
```

## Section 1 — R8: H(t, u, v) = C(t) - S(u, v)

```rust
pub struct R8System { /* Bernstein grids: curve leaf (1-var) and surface
                        leaf (2-var) lifted homogeneous, cross-multiplied:
                        H_k = Cw(t)*S_k(u,v) - C_k(t)*Sw(u,v)  (k in x,y,z) */ }
impl R8System {
    pub fn try_new(curve: &BezierLeaf1, surface: &BezierLeaf) -> Construction<R8System>;
}
impl SquareResidualEval for R8System { /* arity 3 */ }
```

- Input types: the surface leaf is the landed `BezierLeaf` (kernel::leaf);
  the curve leaf is `BezierLeaf1` — a NEW minimal 1-var homogeneous leaf
  struct in THIS file (`try_new` refuses degree 0, non-finite, non-positive
  weights; same discipline as the shim's `BezierLeaf` — if the Wave-1 leaf
  packet already landed a 1-var leaf, USE IT and record that).
- Construction cross-multiplies to polynomial grids (N5: no division
  anywhere in eval; the weights arrive as the §7.1 value argument to
  `krawczyk_c1`, never divided inside).
- Regularity (§7 R8): det DH != 0 i.e. C'(t) not in T(u,v) — certified by
  C1's inclusion itself (a Proven PointCert IS the transversality
  certificate on the box); the `r8_transversality_refusal` test pins that
  a curve TANGENT to the surface refuses Inconclusive (Conditioning) at
  the tangency box.
- eval/jac_encl: interval Bernstein evaluation through the landed hull
  kernels (de Casteljau restriction, the `leaf.rs`/`hull.rs` discipline);
  outward-rounded; NO transcendental (rational leaves only — a
  transcendental-carrier input is `RefusalKind::TranscendentalCarrier`,
  Disproven, at try_new).

## Section 2 — R9: J(t, r) = C1(t) - C2(r), one chart

```rust
pub struct R9System { /* H_k = C1w(t)*C2_k(r) - C1_k(t)*C2w(r), k in x,y */ }
impl R9System { pub fn try_new(a: &BezierLeaf1, b: &BezierLeaf1) -> Construction<R9System>; }
impl SquareResidualEval for R9System { /* arity 2 */ }
```

The 2D prior art is `formal/bezier_isect.rs` (square 2x2 Krawczyk,
Bernstein exclusion) — REUSE its algorithms via the S2A seam (the seam's
n=2 arm composes them); do not fork a second 2D engine. Chart discipline
(§3.3): both curves must be in the SAME lifted chart — `try_new` takes the
chart id as data (`chart: ChartId` field) and refuses mismatched charts
with `RefusalKind::ChartExhausted`-adjacent evidence
(`RefusalEvidence::Predicate { name: "r9_requires_one_chart" }`).

## Section 3 — tests

The eight `tests_required` names; ground truths:
1. R8 from a line curve and a plane surface leaf: the known root
   (t*, u*, v*) where the line pierces the plane — Proven PointCert whose
   box contains it (exact rational fixture data).
2. try_new refusals: non-finite coefficients, degree 0, a
   transcendental-carrier marker (construct via CarrierData misuse) —
   each the named refusal.
3. The tangent fixture: a line lying IN the plane's tangent cone at the
   contact — Inconclusive (Conditioning) at the tangency box, per the
   regularity note in Section 1.
4. R9 from two coplanar rational quadratics crossing at a known (t*, r*):
   Proven; two non-intersecting curves in a shared box: the C1 outcome is
   Disproven-backed (K disjoint from B — no crossing in the box) or
   Inconclusive — assert the class per the S2A backing table.
5. Homogeneous discipline: source scan — no `/` on weight-bearing interval
   expressions outside the documented final-rationalization sites; the
   `no_transcendental_call_in_r89_module` scan (sin|cos|atan2|exp|ln|log|
   powf outside comments; sqrt permitted only if a normalization needs it
   — prefer none).

House rules: H-1; H-3 same-line opt-outs; fmt + clippy (exact verify form,
unfiltered, ALL findings) clean; `cargo check --workspace --all-targets`
green.

## Done-when

- `cargo test -p truck-certified --lib --tests --no-fail-fast` green.
- RESULT.json AT THE WORKTREE ROOT.

## Stop conditions

1. The S2A seam (`SquareResidualEval` / `krawczyk_c1`) differs from the
   verbatim shapes above — stop, record the actual spelling; the seam is a
   two-packet contract and gets amended in BOTH packets, not adapted here.
2. The cross-multiplied R8 grids for a needed fixture exceed the landed
   hull kernels' degree support — stop, name the degree; do not
   reimplement Bernstein arithmetic.
3. A test fixture's known root cannot be certified Proven with
   rho <= RHO_MAX after honest subdivision — record the numbers; do not
   widen a box or loosen a comparison.

Commit subject: `feat(certified): R8/R9 square residuals over the C1 seam
(BG-KV2-202-S1A)`.
