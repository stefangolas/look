# WORK PACKET ADM-L5-RECIPROCAL — the certified reciprocal-power primitive (lemma 5, Theorem D)

Lemma packet: PURE polynomial mathematics in `truck-evidence/src/num/` —
no geometry, no patches, dispatchable independently. Lands
`certified_reciprocal_power(W, p, target_error) -> (Q_m, ε_m)`: the exact
polynomial approximating `W⁻ᵖ` with a CERTIFIED uniform error bound
(theorem D), from which ADM-003's volume assembly brackets rational-face
integrals.

```yaml
id:          ADM-L5-RECIPROCAL
contract:    [ADM-L5-RECIPROCAL]
class:       design
crates:      [truck-evidence]
depends_on:  []
write_allow:
  - vendor/truck/truck-evidence/src/num/reciprocal.rs
  - vendor/truck/truck-evidence/src/num/mod.rs
  - vendor/truck/truck-evidence/tests/reciprocal_conformance.rs
read_allow:
  - docs/SWEPT_PAIR_ADMISSION_SPEC.md
  - vendor/truck/truck-evidence/src/num/
tests_required:
  - constant_weight_exact_no_remainder
  - linear_weight_against_closed_form
  - tail_bound_brackets_true_error_on_adversarial_fixtures
  - non_positive_weight_bracket_refuses
anchors:
  - {id: A1, expect: 9, cmd: "grep -c 'KrawczykSystem' vendor/truck/truck-evidence/src/num/krawczyk.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn' vendor/truck/truck-evidence/src/num/roots.rs"}
budget:      {turns: 55, ctx_tokens: 150000}
```

## The lemma (Theorem D, verbatim from the adopted review)

Given a Bernstein polynomial `W` on `[0,1]` with certified hull
`0 < w₋ ≤ W ≤ w₊`: set `w₀ = (w₋+w₊)/2`, `δ = (W − w₀)/w₀`,
`|δ| ≤ ρ = (w₊−w₋)/(w₊+w₋) < 1`. Then

```
W⁻ᵖ = w₀⁻ᵖ (1+δ)⁻ᵖ,   (1+δ)⁻ᵖ = Σ_{k≥0} (−1)ᵏ C(k+p−1, p−1) δᵏ
```

Truncate after `m`: `Q_m = w₀⁻ᵖ Σ_{k≤m} (−1)ᵏ C(k+p−1, p−1) δᵏ` — an
ordinary polynomial — with the CERTIFIED uniform tail

```
ε_m ≤ w₀⁻ᵖ Σ_{k=m+1}^∞ C(k+p−1, p−1) ρᵏ
```

(evaluate the closed-form geometric-sum bound exactly). The certificate is
the pair `(Q_m, ε_m)`: any integral of `W⁻ᵖ·(polynomial)` computed with
`Q_m` is within `‖polynomial‖∞ · ε_m · measure` of the truth — both factors
Bernstein-enclosable.

1. The weight bracket comes from the Bernstein hull (the landed
   `hull_bernstein_2d` culture, applied in 1-D here).
2. `Q_m`'s coefficients: exact in `Expansion` arithmetic (dyadic `w₀, ρ`
   from dyadic hulls; the binomial coefficients are integers).
3. Non-positive or non-finite bracket ⇒ typed refusal (the primitive
   never divides by an uncertified sign).
4. Convergence is geometric in ρ; the caller subdivides when `m` exceeds
   the budget — the primitive reports `m` and `ε_m`.

F1 layer: the primitive lives in `truck-evidence/src/num/` (the
`krawczyk.rs` home); no certified-layer dependency. H-1/H-3/H-6 inherited.

## Done when

```
cargo fmt --check -p truck-evidence
cargo clippy -p truck-evidence --all-targets -- -D warnings
cargo test -p truck-evidence --lib --tests
```

with all four named tests green (constant weight ⇒ `Q_m = w⁻ᵖ` exactly,
`ε_m = 0`; linear weight against the closed form; adversarial
near-vanishing weights where the tail bound must bracket the true error —
verified by high-precision interval evaluation) and the anchors holding.

## Forbidden

Numerical quadrature anywhere. uncertified truncation. Float coefficients
in `Q_m` or `ε_m`.

## Stop conditions

- The tail bound fails to bracket the true error on any conformance
  fixture (checked by interval evaluation) → that is a THEOREM-LEVEL
  defect — stop-and-file at the highest priority (it would falsify the
  adopted review's Theorem D).

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `feat(evidence): certified reciprocal-power primitive — exact polynomial, certified geometric tail (ADM-L5)`.
