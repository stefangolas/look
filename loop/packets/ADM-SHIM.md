# WORK PACKET ADM-SHIM â€” the extracted-patch type freeze: the lemma layer's shared contract

Contract-shim packet (the spine pattern): freezes EVERYTHING the admission
lemma wave parallelizes against â€” the extracted tensor-Bernstein patch
type, the five lemma-kernel signatures, and the synthetic fixture kit â€”
BEFORE the lemma packets are authored. D-shim discipline: types and
refusing constructors only; every kernel body refuses. Consumes the
LANDED ADM-000 carriers (`construct/admission.rs`) and extends them with
the layer-2 representation.

```yaml
id:          ADM-SHIM
contract:    [ADM-SHIM]
class:       design
crates:      [truck-certified]
depends_on:  [ADM-000-CONTRACT]
write_allow:
  - vendor/truck/truck-certified/src/construct/patches.rs
  - vendor/truck/truck-certified/src/construct/mod.rs
  - vendor/truck/truck-certified/tests/patch_fixtures.rs
read_allow:
  - docs/SWEPT_PAIR_ADMISSION_SPEC.md
  - vendor/truck/truck-certified/src/construct/admission.rs
  - vendor/truck/truck-certified/src/
tests_required:
  - fixture_patches_satisfy_closed_form_invariants
  - refusing_kernels_refuse
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub struct Ssi4System' vendor/truck/truck-certified/src/construct/bie/ssi4.rs"}
  - {id: A2, expect: 4, cmd: "grep -c 'admit_tensor_spline_pair' vendor/truck/truck-certified/src/construct/admission.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'hull_bernstein_2d' vendor/truck/truck-certified/src/hull.rs"}
budget:      {turns: 45, ctx_tokens: 120000}
```

## What freezes here (the single point of failure â€” get the types right)

1. **`TensorBernsteinPatch`** (`construct/patches.rs`): the extracted
   per-knot-rectangle patch â€” numerator coefficients `Ã‚` over the span
   (tensor-Bernstein, bidegrees recorded), weight patch `Å´` with its
   **certified positive bracket** `[wâ‚‹, wâ‚Š]` (Bernstein hull), the span's
   domain box, and the parent face/edge ids. Refusing constructor: a patch
   whose weight bracket cannot be certified positive refuses
   `InvalidInput` â€” never a silent non-positive weight.
2. **Lemma-kernel signatures** (declared, REFUSING â€” bodies land in the
   lemma packets, each with its admitting test):
   - `extract_patches(face) -> Result<Vec<TensorBernsteinPatch>, _>` (L1)
   - `patch_product(a, b) -> Result<TensorBernsteinPatch, _>` â€” exact
     degree-grown multiplication (L2)
   - `normal_numerator(patch) -> Result<M-Polynomial, _>` â€” the verified
     `M = W(A_uÃ—A_v) âˆ’ W_v(A_uÃ—A) âˆ’ W_u(AÃ—A_v)` assembly data (L3)
   - `deflate_factor(patch, boundary) -> Result<...>` and
     `seam_identified(patch_a, patch_b) -> Result<bool, _>` (L4)
   - `certified_reciprocal_power(w, p, target_error) -> Result<(Q_m, Îµ_m)>`
     â€” declared here as the truck-evidence `num/` signature L5 lands
     against (the type alias only; the body is L5's, in truck-evidence â€”
     F1 layer respected).
3. **The synthetic fixture kit** (`tests/patch_fixtures.rs`): patches with
   KNOWN closed forms and invariants â€” a bilinear patch, a rational
   quadratic patch with exactly computable volume, a collapsed-edge patch
   (known deflation multiplicity), a seam-identified pair, a
   weight-bracket-violating patch (refuses). Every lemma packet tests
   against THESE â€” one fixture kit, six consumers, no divergent fixtures.
4. **What does NOT land here**: any kernel body (D-shim), any corpus
   contact, any boolean-boundary change.

H-1: `#![deny(clippy::unwrap_used)]` on the new module. H-3: constants
named. The adapters consume the landed ADM-000 carriers â€” this shim
EXTENDS, never edits, `admission.rs`'s landed types.

## Done when

```
cargo fmt --check -p truck-certified
cargo clippy -p truck-certified --all-targets -- -D warnings
cargo test -p truck-certified --lib --tests
```

with the fixture invariants green and the refusing kernels refusing.

## Forbidden

Kernel bodies. Corpus contact. Boolean-boundary changes. Editing the
landed ADM-000 carrier types (extend only). Weakening landed tests.

## Stop conditions

- The patch type cannot represent a landed spline face's extracted form
  without loss â†’ SPEC_GAP naming the representation gap (this is THE type
  freeze; a gap here reshapes the lemma wave â€” stop, do not improvise).
- ADM-000's landed carriers and the patch type cannot be reconciled â†’
  SPEC_GAP with both signatures quoted.

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `feat(certified): the extracted-patch contract shim â€” lemma-layer types, refusing kernels, shared fixture kit (ADM-SHIM)`.
