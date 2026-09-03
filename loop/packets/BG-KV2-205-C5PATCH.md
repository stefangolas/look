# BG-KV2-205-C5PATCH — CoonsSurface implements CertifiedPatch (the C5 delta)

Wave-2 implementation packet (build spec §4; §19 row 18's v2 delta). The
landed `CoonsSurface` (truck-geometry/src/decorators/coons.rs) is bilinear
— POLYNOMIAL, so interval evaluation needs no transcendental and no weight
handling (its §7.1 weight_bound is the constant-1 plumbing). This packet
adds the `CertifiedPatch` implementation as a NEW file in truck-certified
(orphan rule: the trait lives in truck-certified, the type in
truck-geometry — the impl must live where the trait is). **No changes to
CoonsSurface itself** (V5 identity: the landed decorator is untouched).

```yaml
id:          BG-KV2-205-C5PATCH
contract:    [BG-KV2-205-C5PATCH]
class:       design
crates:      [truck-certified]
depends_on:  [BG-KV2-102-LEAF]
write_allow:
  - vendor/truck/truck-certified/src/kernel/coons_patch.rs
  - vendor/truck/truck-certified/src/kernel/mod.rs
  - vendor/truck/truck-certified/tests/kernel_coons_patch.rs
read_allow:
  - docs/CONSTRUCTIVE_GEOMETRY_KERNEL_SPEC_V2.md
  - vendor/truck/truck-certified/src/kernel
  - vendor/truck/truck-geometry/src/decorators/coons.rs
budget:      {turns: 20, ctx_tokens: 70000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub struct CoonsSurface' vendor/truck/truck-geometry/src/decorators/coons.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub trait CertifiedPatch {' vendor/truck/truck-certified/src/kernel/patch.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub fn jacobian' vendor/truck/truck-geometry/src/decorators/coons.rs"}
  - {id: A4, expect: 0, cmd: "grep -c 'coons_patch' vendor/truck/truck-certified/src/kernel/mod.rs"}
tests_required:
  - coons_enclose_contains_sampled_points
  - coons_regularity_proven_on_a_regular_patch
  - coons_folded_patch_is_inconclusive_not_proven
  - coons_weight_bound_is_the_constant_one_plumbing
  - coons_jacobian_and_certifiedpatch_regularity_agree
  - no_transcendental_call_in_coons_patch_module
```

## Section 1 — the impl (`kernel/coons_patch.rs`, NEW)

`impl CertifiedPatch for CoonsSurface` (import the landed type;
`CoonsSurface`'s stored corners `p00..p11` are the whole geometry — the
bilinear Bernstein form is exact):

- `enclose` — interval bilinear evaluation of the four corners over the
  box (product-form interval arithmetic via `CertifiedInterval`;
  deterministic order pinned in the module doc per N2: expand
  ((1-u)(1-v)p00 + u(1-v)p10) + ((1-u)v p01 + u v p11) in exactly this
  order).
- `derivs` — the analytic first derivatives differentiated ONCE by hand
  from the bilinear form; interval evaluation of the same.
- `normal_cone` — cone over the cross-product enclosure of the derivative
  enclosure (the local constructor discipline from `leaf.rs`).
- `regularity` — EG − F² enclosure; Proven iff lower >
  `config::TOL_JACOBIAN`; Disproven iff upper < TOL_JACOBIAN (the folded
  patch: `Degeneracy` carries the box and the enclosure — spec §5.9: folded
  is construction-valid, geometry-invalid); else Inconclusive.
- `weight_bound` — `Some(Proven(CertifiedPositive of 1.0))` (the frozen
  §3.1 constant-1 plumbing from BG-KV2-104's plane spelling).

## Section 2 — tests

The six `tests_required` names:
1. Sampled (u,v) grid points lie in `enclose` (corner fixture: the unit
   square mapped to a known bilinear patch; ground truth the four corners
   and the center u=v=0.5 = average of corners, asserted exactly).
2. A regular (non-degenerate) patch: Proven with margin >> TOL_JACOBIAN.
3. A folded patch (one corner pulled across, e.g. p11 = p00 + small
   opposite orientation): the regularity outcome is Inconclusive or
   Disproven — NEVER Proven (assert the class).
4. weight_bound returns Proven(1.0) exactly.
5. The §5.9 one-call rule: `regularity`'s EG − F² computation and the
   landed `jacobian()` cross product AGREE at sample points (evaluate the
   landed jacobian as f64, the certified enclosure contains it).
6. Source scan for transcendentals.

House rules: H-1; H-3 same-line opt-outs; fmt + clippy (exact verify form,
unfiltered, ALL findings) clean; `cargo check --workspace --all-targets`
green.

## Done-when

- `cargo test -p truck-certified --lib --tests --no-fail-fast` green.
- RESULT.json AT THE WORKTREE ROOT.

## Stop conditions

1. `CoonsSurface`'s stored fields/constructors differ from the census
   (corners p00..p11, `try_new` validating at tol.position) — stop,
   record the actual shape.
2. The bilinear interval form cannot prove regularity on the regular
   fixture with margin — record the numbers; do not loosen anything.

Commit subject: `feat(certified): CoonsSurface implements CertifiedPatch
(BG-KV2-205-C5PATCH)`.
