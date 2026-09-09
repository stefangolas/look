# WORK PACKET ADM-L4-DEFLATE-SEAM â€” exact deflation and seam identification (lemma 4)

Lemma packet: pure functions over the frozen patch type. Proves IN ADVANCE
Theorem B2's computational layer (collapsed-edge deflation: divide the
known `(1âˆ’v)` factor out of `M`, certify the quotient, iterate
multiplicity) and the seam-identification test
(`Aâ‚€Wâ‚ âˆ’ Aâ‚Wâ‚€ â‰¡ 0` on aligned degrees/knots).

```yaml
id:          ADM-L4-DEFLATE-SEAM
contract:    [ADM-L4-DEFLATE-SEAM]
class:       design
crates:      [truck-certified]
depends_on:  [ADM-SHIM]
write_allow:
  - vendor/truck/truck-certified/src/construct/deflate.rs
  - vendor/truck/truck-certified/src/construct/mod.rs
  - vendor/truck/truck-certified/tests/deflate_seam_conformance.rs
read_allow:
  - docs/SWEPT_PAIR_ADMISSION_SPEC.md
  - vendor/truck/truck-certified/src/construct/patches.rs
tests_required:
  - deflation_divides_exactly_zero_remainder
  - multiplicity_iteration_handles_double_collapse
  - deflated_certificate_passes_interior_fixture
  - seam_identity_exact_on_aligned_fixture
  - misaligned_seam_refuses_typed
anchors:
  - {id: A1, expect: 15, cmd: "grep -c 'TensorBernsteinPatch' vendor/truck/truck-certified/src/construct/patches.rs"}
  - {id: A2, expect: 4, cmd: "grep -c 'admit_tensor_spline_pair' vendor/truck/truck-certified/src/construct/admission.rs"}
budget:      {turns: 55, ctx_tokens: 150000}
```

## The lemmas

1. **Exact factor division.** If `M` vanishes on `v = 1` (the collapsed
   boundary â€” detected exactly: the `v=1` slice's Bernstein coefficients
   are all zero), then `M` has the factor `(1âˆ’v)`; divide the Bernstein
   polynomial by the known factor EXACTLY (zero remainder proven, not
   assumed) and certify the quotient. Iterate to multiplicity `k`
   (`M = (1âˆ’v)^k Â· M*`).
2. **The deflated certificate.** `cÂ·M* > 0` over the closed box â‡’ regular
   interior, the only rank defect is the intentional collapse â€” the
   hemisphere test (L3) applied to the quotient.
3. **Seam identification.** Align the two boundary patches (degree
   elevation + knot insertion to common knots â€” exact), then the exact
   zero test `Aâ‚€Wâ‚ âˆ’ Aâ‚Wâ‚€ â‰¡ 0` on the coefficient vector. A seam that
   closes is a paired-BRep-edge fact; a misaligned near-seam REFUSES
   typed (near-miss is not identity â€” the fixture pins this).

H-1/H-3/H-6, SFC: inherited. Pure exact algebra.

## Done when

```
cargo fmt --check -p truck-certified
cargo clippy -p truck-certified --all-targets -- -D warnings
cargo test -p truck-certified --lib --tests
```

with all five named tests green and the anchors holding.

## Forbidden

Numeric near-equality in the seam test (identity is exact or refused).
Editing the shim's landed types. Corpus contact. Boolean-boundary changes.

## Stop conditions

- A collapsed boundary whose `M`-vanishing is NOT a clean Bernstein
  factor (the zero-slice test passes but the division leaves a nonzero
  remainder) â†’ a substrate bug, stop-and-file (this would falsify the
  B2 algebra â€” impossible if the identity is as verified; treat as the
  highest-priority defect).

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `feat(certified): deflation and seam-identity lemmas â€” exact factor division, multiplicity iteration (ADM-L4)`.
