# WORK PACKET ADM-L3-NORMALCONE — the rational normal numerator and the hemisphere certificate (lemma 3)

Lemma packet: pure functions over the frozen patch type. Proves IN ADVANCE
Theorem B1's computational layer: the polynomial normal numerator `M` (the
verified identity `X_u×X_v = M/W³`), the hemisphere regularity certificate
(`min(bernstein coefficients of c·M) > 0` over a box), and the per-box
normal-cone extraction that Theorem C's transversality gate consumes.

```yaml
id:          ADM-L3-NORMALCONE
contract:    [ADM-L3-NORMALCONE]
class:       design
crates:      [truck-certified]
depends_on:  [ADM-SHIM]
write_allow:
  - vendor/truck/truck-certified/src/construct/normal_cone.rs
  - vendor/truck/truck-certified/src/construct/mod.rs
  - vendor/truck/truck-certified/tests/normal_cone_conformance.rs
read_allow:
  - docs/SWEPT_PAIR_ADMISSION_SPEC.md
  - vendor/truck/truck-certified/src/construct/patches.rs
tests_required:
  - normal_numerator_identity_exact
  - hemisphere_certificate_passes_regular_fixture
  - hemisphere_certificate_refuses_singular_fixture
  - cone_extraction_brackets_the_true_normals
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'TensorBernsteinPatch' vendor/truck/truck-certified/src/construct/patches.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'hull_bernstein_2d' vendor/truck/truck-certified/src/hull.rs"}
budget:      {turns: 60, ctx_tokens: 170000}
```

## The lemmas

1. **Derivative operators on tensor-Bernstein patches** — exact (the
   Bernstein derivative: coefficients transform, degree is retained),
   machine-checked against known-derivative fixtures.
2. **The normal numerator assembly** — `M = W(A_u×A_v) − W_v(A_u×A) −
   W_u(A×A_v)` from the patch data (the identity `X_u×X_v = M/W³` was
   hand-verified pre-adoption; the conformance test re-verifies it per
   fixture by exact evaluation against a direct rational-derivative
   computation).
3. **The hemisphere certificate (B1)** — dyadic `c` from the float
   midpoint normal (SFC: the float choice searches; the certificate is the
   exact interval evaluation), `min(bernstein coefficients of c·M) > 0`
   over the box ⇒ regular on ALL of `B`. Failure ⇒ subdivide (the caller
   owns subdivision; this module returns the verdict + the coefficients).
4. **Normal-cone extraction** — per box, the cone bracketing `n` over the
   patch (from `M`'s Bernstein hull) — Theorem C's exact input, shared
   with FSSI-001.

H-1/H-3/H-6, SFC, single-interval-algebra rule: inherited.

## Done when

```
cargo fmt --check -p truck-certified
cargo clippy -p truck-certified --all-targets -- -D warnings
cargo test -p truck-certified --lib --tests
```

with all four named tests green and the anchors holding.

## Forbidden

Sampled normals in any certificate. Editing the shim's landed types.
Corpus contact. Boolean-boundary changes.

## Stop conditions

- The dyadic rounding of `c` cannot certify what the float normal
  suggests even after subdivision → the subdivision-stall record (the
  caller decides: deeper budget or typed refusal) — not silent.
- The `M` identity fails on a rational fixture → a substrate bug, stop.

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `feat(certified): normal-numerator lemma — hemisphere certificate, cone extraction, exact identity (ADM-L3)`.
