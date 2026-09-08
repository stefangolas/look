# WORK PACKET ADM-003-VOLUME — assemble certified volume facts over admitted patches (r2: the lemma wave's integration)

r2 REWRITE (spine restructure): the reciprocal-power primitive (L5,
Theorem D) is landed and machine-tested, and the extraction (L1) supplies
patches. This packet ASSEMBLES the certified volume facts for
spline-faced solids: the exact face 2-form over extracted patches, the
trim-boundary reduction consuming L5's primitive, and the closure
discipline. The algebraic-trim subtlety is CONCENTRATED HERE (the one
research-adjacent risk in the family).

```yaml
id:          ADM-003-VOLUME
contract:    [ADM-003-VOLUME]
class:       design
crates:      [truck-certified, truck-evidence]
depends_on:  [ADM-SHIM, ADM-L1-EXTRACT, ADM-L5-RECIPROCAL]
write_allow:
  - vendor/truck/truck-certified/src/construct/volume_facts.rs
  - vendor/truck/truck-certified/src/construct/mod.rs
  - vendor/truck/truck-certified/tests/volume_facts_conformance.rs
read_allow:
  - docs/SWEPT_PAIR_ADMISSION_SPEC.md
  - vendor/truck/truck-evidence/src/num/reciprocal.rs
  - vendor/truck/truck-certified/src/construct/
tests_required:
  - frustum_special_case_bit_identical
  - polynomial_face_volume_matches_closed_form
  - rational_face_volume_within_certified_bound
  - unclosed_boundary_detected_not_scored
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'TensorBernsteinPatch' vendor/truck/truck-certified/src/construct/patches.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'certified_reciprocal_power' vendor/truck/truck-evidence/src/num/reciprocal.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'extract_patches' vendor/truck/truck-certified/src/construct/extract.rs"}
budget:      {turns: 75, ctx_tokens: 200000}
```

## Scope decisions

1. **The exact face 2-form over patches.** Per extracted patch, the
   divergence-form contribution `V = (1/3)∬ X·(X_u×X_v)` — the
   integrand assembled from the patch coefficients via the landed product
   lemma (L2's kernel where products arise). Polynomial faces: exact.
2. **Rational faces via L5.** The verified cancellation
   `X·(X_u×X_v) = P/W³`, `P = A·(A_u×A_v)`: the reciprocal-power
   primitive (L5) polynomializes `W⁻³` with a certified tail; the volume
   error certificate `|I − Ĩ| ≤ area·‖P‖∞·ε_m` — every factor a Bernstein
   enclosure.
3. **The algebraic-trim subtlety (CONCENTRATED HERE — the family's one
   research-adjacent risk).** Boolean trim boundaries on a spline face are
   the pullback's algebraic curve `P(s,t) = 0` — NOT parametric
   polynomials. Certified routes (choose on simplicity): (a) the
   certified cell decomposition of the face domain (subdivision cells
   classified by `P`'s sign with the landed exclusion discipline;
   boundary-cell contributions bounded by the implicit function's
   certified Lipschitz data — the Krishnamurthy–McMains shape), or (b)
   Hermite interpolation of the traced curve with a certified error
   bracket carried into the volume bound. Either route: the volume is a
   two-sided certified bracket, never a point value pretending
   exactness. A route that cannot close its bracket → typed refusal of
   the volume FACT (the geometry may still be certified by the funnel).
4. **Closure discipline.** Volume is scored only over a proven-closed,
   oriented boundary (the trimmed patches' union must close — detected
   via the shared-boundary structure); unclosed ⇒ the fact is refused,
   never scored.
5. **V5 anchor.** The landed frustum telescoping answers bit-identically
   through the new machinery on its own fixtures.

H-1/H-3/H-6, SFC, single-interval-algebra rule: inherited.

## Done when

```
cargo fmt --check -p truck-certified -p truck-evidence
cargo clippy -p truck-certified -p truck-evidence --all-targets -- -D warnings
cargo test -p truck-certified --lib --tests
cargo test -p truck-evidence --lib --tests
```

with all four named tests green and the anchors holding.

## Forbidden

Numerical quadrature. Uncertified approximation. Scoring an unproven-
closed boundary. Editing the lemma kernels or the shim's types.

## Stop conditions

- Neither certified route (a)/(b) closes its bracket on a corpus-derived
  face → SPEC_GAP with the face class and the bracket data (this is the
  family's known research risk, pre-escaped: the volume FACT refuses
  typed while the geometry stays certified).
- The frustum V5 anchor drifts → a substrate defect, stop-and-file.

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `feat(certified): certified volume assembly — exact 2-forms, reciprocal-power rational faces, algebraic-trim brackets (ADM-003)`.
