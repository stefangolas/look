# WORK PACKET ADM-L2-PRODUCT — exact tensor-Bernstein multiplication (lemma 2)

Lemma packet: pure function over the frozen patch type. Proves IN ADVANCE
the kernel of Theorem A's polynomialized interaction: the exact product of
two tensor-Bernstein patches with degree growth, from which
`F = Ŵ_Y·Â − Ŵ_X·B̂` is assembled.

```yaml
id:          ADM-L2-PRODUCT
contract:    [ADM-L2-PRODUCT]
class:       design
crates:      [truck-certified]
depends_on:  [ADM-SHIM]
write_allow:
  - vendor/truck/truck-certified/src/construct/prod.rs
  - vendor/truck/truck-certified/src/construct/mod.rs
  - vendor/truck/truck-certified/tests/prod_conformance.rs
read_allow:
  - docs/SWEPT_PAIR_ADMISSION_SPEC.md
  - vendor/truck/truck-certified/src/construct/patches.rs
tests_required:
  - product_identity_exact_at_matched_parameters
  - degree_growth_recorded_not_hidden
  - zero_patch_product_is_zero
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'TensorBernsteinPatch' vendor/truck/truck-certified/src/construct/patches.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub struct Ssi4System' vendor/truck/truck-certified/src/construct/bie/ssi4.rs"}
budget:      {turns: 50, ctx_tokens: 140000}
```

## The lemma

The product of two tensor-Bernstein patches is the tensor-Bernstein patch
of ADDED degree with exactly computable coefficients (the coefficient
convolution along each dimension — exact in `Expansion` arithmetic). The
conformance test proves the identity: `patch_product(a, b)` evaluated at
any exact parameter equals `a(t)·b(t)` evaluated directly — machine-checked
per fixture, including rational-weight patches (the product operates on
numerator patches; weights multiply the same way).

1. `patch_product(a, b) -> Result<TensorBernsteinPatch, _>`: degree growth
   `deg = deg_a + deg_b` recorded ON the patch (never hidden); spans must
   match (mismatched spans refuse typed).
2. Degree-elevation helper: raising a patch to a target degree WITHOUT
   changing its values (exact — the elevation identity, machine-checked).
3. Zero-product and annihilator cases behave exactly (the algebra the
   `F` assembly relies on: `A·(A_u×A)`-type annihilations are coefficient
   facts, tested here at the patch level).

H-1/H-3/H-6: inherited. Pure exact algebra — nothing searches, nothing
samples.

## Done when

```
cargo fmt --check -p truck-certified
cargo clippy -p truck-certified --all-targets -- -D warnings
cargo test -p truck-certified --lib --tests
```

with all three named tests green and the anchors holding.

## Forbidden

Corpus contact. Float evaluation. Editing the shim's landed types.
Approximate products (truncated convolutions without the exact degree).

## Stop conditions

- The degree-grown product's coefficients cannot be computed exactly at
  the corpus's spline degrees (explosion) → SPEC_GAP with the degree
  arithmetic — the atlas degree bounds bound this, so a blow-up is a
  representation bug, not a law of nature.

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `feat(certified): tensor-Bernstein product lemma — exact degree growth, identity machine-checked (ADM-L2)`.
