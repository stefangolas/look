# WORK PACKET ADM-L1-EXTRACT — exact Bézier extraction over the landed spline faces (lemma 1)

Lemma packet (the spine pattern — pure function over the frozen
`TensorBernsteinPatch` type, synthetic fixtures, no corpus contact, no
boolean boundary). Proves IN ADVANCE the intermediate result ADM-001
consumes: knot-insertion to Bézier form, per knot rectangle, with EXACT
coefficients, over the landed spline face representations.

```yaml
id:          ADM-L1-EXTRACT
contract:    [ADM-L1-EXTRACT]
class:       design
crates:      [truck-certified]
depends_on:  [ADM-SHIM]
write_allow:
  - vendor/truck/truck-certified/src/construct/extract.rs
  - vendor/truck/truck-certified/src/construct/mod.rs
  - vendor/truck/truck-certified/tests/extract_conformance.rs
read_allow:
  - docs/SWEPT_PAIR_ADMISSION_SPEC.md
  - vendor/truck/truck-certified/src/construct/patches.rs
  - vendor/truck/truck-certified/src/
tests_required:
  - extraction_identity_exact_on_synthetic_splines
  - span_enumeration_covers_the_domain_exactly
  - weight_brackets_certified_positive_or_refused
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'TensorBernsteinPatch' vendor/truck/truck-certified/src/construct/patches.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'hull_bernstein_2d' vendor/truck/truck-certified/src/hull.rs"}
budget:      {turns: 55, ctx_tokens: 150000}
```

## The lemma (what is proven, and how)

Knot insertion to Bézier form is an EXACT local change of basis: on every
knot rectangle, the landed rational tensor-spline face equals the
extracted patch `Â/Ŵ` identically — no approximation, no sampling. The
conformance test proves the identity by exact evaluation: for synthetic
splines with known closed forms, the extracted patch evaluated at any
exact parameter equals the original face's exact evaluation — same
polynomial, different basis.

1. `extract_patches(face) -> Result<Vec<TensorBernsteinPatch>, _>`:
   enumerate the knot rectangles (span coverage test: the rectangles
   partition the face's domain exactly — no gap, no overlap, boundary
   shared), extract each to tensor-Bernstein form with `Expansion`-exact
   coefficients, certify each patch's weight bracket `[w₋, w₊]` via the
   Bernstein hull (the landed `hull_bernstein_2d` discipline); a
   non-positive bracket refuses — the D-shim constructor rule.
2. Degree/range recording: per-span bidegrees and the weight bracket are
   part of the patch data (ADM-003's Theorem D consumes the bracket).
3. The identity test is the proof: `extract` then evaluate at matched
   exact parameters — equality is machine-checked, per fixture.

H-1: `#![deny(clippy::unwrap_used)]` on the new module. H-3: constants
named. H-6: floats never in evidence. SFC: nothing here searches — this
module is pure exact algebra.

## Done when

```
cargo fmt --check -p truck-certified
cargo clippy -p truck-certified --all-targets -- -D warnings
cargo test -p truck-certified --lib --tests
```

with all three named tests green and the anchors holding.

## Forbidden

Corpus contact. Boolean-boundary changes. Float evaluation in any
certificate. Editing the shim's landed types (consume only).

## Stop conditions

- A landed spline face form cannot be extracted losslessly (a
  representation the knot-insertion identity cannot cover) → SPEC_GAP
  naming the form — this reshapes the shim, stop immediately.

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `feat(certified): exact Bezier extraction lemma — knot-rectangle identity, certified weight brackets (ADM-L1)`.
