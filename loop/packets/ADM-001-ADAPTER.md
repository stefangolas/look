# WORK PACKET ADM-001-ADAPTER — assemble extraction + product into the Ssi4System pipeline (r2: the lemma wave's integration)

r2 REWRITE (spine restructure): the lemma layer (extraction L1, product
L2) is landed and machine-tested by prior packets. This packet ASSEMBLES:
wire `extract_patches` + `patch_product` into the polynomialized pair
system `F = Ŵ_Y·Â − Ŵ_X·B̂` per span-pair, feed the landed `Ssi4System`,
and ADMIT the first pair class (ruled-section lofts) behind its admitting
test. The theory is committed and the lemmas proven — this is integration.

```yaml
id:          ADM-001-ADAPTER
contract:    [ADM-001-ADAPTER]
class:       design
crates:      [truck-certified]
depends_on:  [ADM-SHIM, ADM-L1-EXTRACT, ADM-L2-PRODUCT]
write_allow:
  - vendor/truck/truck-certified/src/construct/admission.rs
  - vendor/truck/truck-certified/src/construct/bie/ssi4.rs
  - vendor/truck/truck-certified/tests/admission_conformance.rs
read_allow:
  - docs/SWEPT_PAIR_ADMISSION_SPEC.md
  - vendor/truck/truck-certified/src/construct/
tests_required:
  - polynomialized_F_zeros_equal_direct_difference_on_fixture
  - ruled_section_loft_pair_admits_and_certifies
  - non_admitted_carriers_still_refuse_typed
  - v5_pair_identity_battery_green
anchors:
  - {id: A1, expect: 15, cmd: "grep -c 'TensorBernsteinPatch' vendor/truck/truck-certified/src/construct/patches.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub struct Ssi4System' vendor/truck/truck-certified/src/construct/bie/ssi4.rs"}
  - {id: A3, expect: 3, cmd: "grep -c 'extract_patches' vendor/truck/truck-certified/src/construct/extract.rs"}
budget:      {turns: 65, ctx_tokens: 180000}
```

## Scope decisions

1. **Assembly only.** The extraction (L1) and product (L2) kernels are
   landed and conformance-tested — this packet COMPOSES them: per
   span-pair, `F = Ŵ_Y·Â − Ŵ_X·B̂` via `patch_product`, assembled into the
   `Ssi4System` input. Any mismatch between the landed lemma signatures
   and the system's input format is an integration seam: reconcile in
   `admission.rs`, never edit the lemma kernels.
2. **First admitted class: ruled-section lofts** — the admitting test is
   REQUIRED (theory v2 §3: general spline sections admit ONLY after
   ADM-002's certificates; until then they refuse typed exactly as
   today).
3. **V5 battery**: every landed green pair answers bit-identically; the
   adapter is reachable ONLY where the old path returned
   `NonCanonicalCarrier` (the ADM-000 dispatch rule).

H-1/H-3/H-6, SFC, single-interval-algebra rule: inherited.

## Done when

```
cargo fmt --check -p truck-certified
cargo clippy -p truck-certified --all-targets -- -D warnings
cargo test -p truck-certified --lib --tests
```

with all four named tests green and the anchors holding.

## Forbidden

Editing the lemma kernels (L1/L2 files) or the shim's landed types.
Splitter/classifier writes (FSSI-LAYER). Admitting general spline sections
before ADM-002 lands. Float predicates.

## Stop conditions

- The landed lemma kernels cannot compose into the `Ssi4System` input
  without a kernel change → stop-and-file (the operator is frozen).
- A ruled-section fixture fails to certify with the lemmas green →
  SPEC_GAP naming the failing stage (the lemma isolation makes this
  diagnosable in minutes).

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `feat(certified): admission assembly — polynomialized pair system over the proven lemmas, ruled-section admission (ADM-001)`.
