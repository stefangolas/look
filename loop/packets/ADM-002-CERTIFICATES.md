# WORK PACKET ADM-002-CERTIFICATES — T1′ regularity certificates and the transversality gate wiring

Design packet, third of the admission program (parallel with ADM-003;
disjoint files). Lands Theorems B1/B2/C: the scalar hemisphere regularity
certificate, the collapsed-edge deflation, the seam identification — and
wires the per-surface normal-cone subsystem to FSSI-001's transversality
gate. Four outcomes only: regular | regular+seam | regular-interior+
collapsed-boundary | genuine parameter singularity → typed refusal.

```yaml
id:          ADM-002-CERTIFICATES
contract:    [ADM-002-CERTIFICATES]
class:       design
crates:      [truck-certified]
depends_on:  [ADM-001-ADAPTER]
write_allow:
  - vendor/truck/truck-certified/src/construct/normal_cone.rs
  - vendor/truck/truck-certified/src/construct/mod.rs
  - vendor/truck/truck-certified/src/construct/admission.rs
  - vendor/truck/truck-certified/tests/admission_certificates.rs
read_allow:
  - docs/SWEPT_PAIR_ADMISSION_SPEC.md
  - docs/FSSI_BUILD_SPEC.md
  - vendor/truck/truck-certified/src/
tests_required:
  - hemisphere_certificate_passes_regular_fixture
  - hemisphere_certificate_refuses_genuine_singularity
  - collapsed_apex_deflation_certifies_interior
  - seam_identification_exact_on_closed_fixture
  - normal_cone_delta_certifies_transversality
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'hull_bernstein_2d' vendor/truck/truck-certified/src/hull.rs"}
  - {id: A2, expect: 5, cmd: "grep -c 'TangentCurveSuspected' vendor/truck/truck-certified/src/ssi.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'admission' vendor/truck/truck-certified/src/construct/mod.rs"}
budget:      {turns: 80, ctx_tokens: 200000}
```

## Scope decisions (pre-decided per the spec — owner theorems B1/B2/C)

1. **The rational normal numerator.** For `X = A/W`:
   `M = W(A_u×A_v) − W_v(A_u×A) − W_u(A×A_v)` (verified identity:
   `X_u×X_v = M/W³`; all regularity questions are polynomial questions in
   `M`). The certificate works on `M`'s Bernstein coefficients, never on
   sampled normals.
2. **Hemisphere certificate (B1).** Choose `c` from the floating midpoint
   normal rounded to a dyadic vector; compute `c·M`'s Bernstein
   coefficients over the box; `min > 0` certifies regularity on ALL of `B`
   (convex-hull property) plus an orientation-preserving projection chart.
   `min ≤ 0` ⇒ Bernstein subdivide under budget; the failure path is
   subdivision, never a guess.
3. **Collapsed-edge deflation (B2).** A boundary collapsed to `p` gives
   `M` a known factor `(1−v)`; divide the Bernstein polynomial by the
   known factor (exact) and certify the quotient; repeat for multiplicity
   `k`. The certificate says: interior regular, the only rank defect is the
   intentional collapse. No subdivision-toward-the-apex.
4. **Seam identification.** `X(u,0) = X(u,1)` certified exactly by
   `A_0·W_1 − A_1·W_0 ≡ 0` on aligned degrees/knots; identified edges are
   paired BRep edges, not singularities.
5. **Transversality wiring (C).** The per-surface normal cones feed
   `δ = min{∠(a,b), π−∠(a,b)} − θ_X − θ_Y > 0 ⇒ ‖n_X×n_Y‖ ≥ sin δ > 0`
   ⇒ `rank DF = 3` at every intersection. This is the FSSI-001 gate's
   mechanism (transversality, NOT emptiness — emptiness is `F`'s own
   exclusion); implement the cone test so FSSI-001 consumes it as its
   substrate rather than rebuilding a 4-D normal field.

H-1/H-3/H-6, SFC, single-interval-algebra rule: inherited verbatim.

## Done when

```
cargo fmt --check -p truck-certified
cargo clippy -p truck-certified --all-targets -- -D warnings
cargo test -p truck-certified --lib --tests
```

with all five named tests green and the anchors holding.

## Forbidden

Sampling normals or points for any certificate. A "tangent sections"
taxonomy (tangent construction sections matter only if they force `M = 0`).
Weakening landed tests. `classify.rs`/`split.rs` writes (FSSI-LAYER).

## Stop conditions

- A genuine singularity the certificate cannot classify (neither regular
  nor collapsed nor seam) → typed refusal recorded, and the fixture archived
  as the boundary evidence (the four-outcome space is exhaustive by design;
  a fifth outcome is a SPEC_GAP).
- The dyadic rounding of `c` cannot certify what the float normal suggests
  → subdivide; if that stalls, SPEC_GAP with the measure.

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `feat(certified): T1-prime regularity certificates — hemisphere, deflation, seam identity, transversality cones (ADM-002)`.
