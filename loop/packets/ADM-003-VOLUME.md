# WORK PACKET ADM-003-VOLUME — T2′ certified volume: boundary-reduced Bernstein integration + the rational primitive

Design packet, fourth of the admission program (parallel with ADM-002;
disjoint files). Lands Theorem D and the published boundary-reduction
construction: certified volume for spline-faced solids — quadrature-free
for polynomial faces, polynomial-plus-certified-remainder for rational.

```yaml
id:          ADM-003-VOLUME
contract:    [ADM-003-VOLUME]
class:       design
crates:      [truck-certified, truck-evidence]
depends_on:  [ADM-001-ADAPTER]
write_allow:
  - vendor/truck/truck-certified/src/construct/volume_facts.rs
  - vendor/truck/truck-certified/src/construct/mod.rs
  - vendor/truck/truck-evidence/src/num/reciprocal.rs
  - vendor/truck/truck-evidence/src/num/mod.rs
  - vendor/truck/truck-evidence/tests/reciprocal_conformance.rs
read_allow:
  - docs/SWEPT_PAIR_ADMISSION_SPEC.md
  - vendor/truck/truck-certified/src/
  - vendor/truck/truck-evidence/src/num/
tests_required:
  - frustum_special_case_bit_identical
  - polynomial_face_volume_matches_closed_form
  - rational_face_volume_within_certified_bound
  - unclosed_boundary_detected_not_scored
anchors:
  - {id: A1, expect: 9, cmd: "grep -c 'KrawczykSystem' vendor/truck/truck-evidence/src/num/krawczyk.rs"}
  - {id: A2, expect: 27, cmd: "grep -c 'pub mod' vendor/truck/truck-certified/src/construct/mod.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'hull_bernstein_2d' vendor/truck/truck-certified/src/hull.rs"}
budget:      {turns: 80, ctx_tokens: 200000}
```

## Scope decisions (pre-decided per the spec — owner Theorem D)

1. **Polynomial faces (published construction).** `g = X·(X_u×X_v)` is
   polynomial; `H(u,v) = ∫ g du`; Green ⇒ `∬ g = ∮ H dv`; Bézier trim
   segments `γ(t)` make the final integrand a UNIVARIATE Bernstein
   polynomial integrated as `Σ pᵢ/(n+1)` — no quadrature anywhere
   (Antolin–Hirschler boundary reduction).
2. **Rational faces (Theorem D — the new small primitive).** The
   triple-product cancellation `X·(X_u×X_v) = P/W³`, `P = A·(A_u×A_v)`
   (verified: all `W_u, W_v` terms vanish). New `num/reciprocal.rs`:
   `certified_reciprocal_power(W, power, target_error) -> (polynomial Q_m,
   remainder_bound ε_m)` via the geometric series in `δ = (W−w₀)/w₀`,
   `|δ| ≤ ρ < 1`, tail `ε_m ≤ w₀⁻ᵖ Σ_{k>m} C(k+p−1, p−1) ρᵏ`. The volume
   error certificate: `|I − Ĩ| ≤ area(R)·‖P‖∞·ε_m`, every term a Bernstein
   enclosure. Geometric convergence in ρ; subdivide once (de Casteljau) on
   violent weight variation.
3. **Closure discipline.** The volume certificate is scored ONLY over a
   proven-closed, oriented trimmed boundary (`V = (1/3) Σ_patches` with
   consistent orientation); an unclosed or mis-oriented boundary is
   DETECTED and refused — never scored. (The same discipline as the
   mesher's shell-closure checks, lifted to certified arithmetic.)
4. **V5 anchor.** The landed frustum telescoping (the revolved-polygon
   special case) must answer bit-identically through the new machinery on
   its own fixtures — the general construction degenerates to it.
5. **F1 layer.** The primitive lives in `truck-evidence/src/num/` (the
   `krawczyk.rs` home); the volume assembly in `construct/` mirrors the
   landed evidence-consumption pattern. No cross-layer edge beyond the
   landed seam.

H-1/H-3/H-6, SFC, single-interval-algebra rule: inherited verbatim.

## Done when

```
cargo fmt --check -p truck-certified -p truck-evidence
cargo clippy -p truck-certified -p truck-evidence --all-targets -- -D warnings
cargo test -p truck-certified --lib --tests
cargo test -p truck-evidence --lib --tests
```

with all four named tests green and the anchors holding.

## Forbidden

Numerical quadrature. Approximation without a certified remainder bound.
Weakening landed facts tolerances. Scoring an unproven-closed boundary.

## Stop conditions

- The trimmed-domain integral needs a representation the landed trim
  substrate cannot supply → stop-and-file naming the gap (FSSI-003's
  boundary strata are the follow-up, not this packet's scope growth).
- Rational weight ranges cannot be bracketed positive on a corpus-derived
  face → SPEC_GAP with the face class.

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `feat(certified): T2-prime certified volume — boundary-reduced Bernstein integration, certified reciprocal-power rational primitive (ADM-003)`.
