# WORK PACKET ADM-000-CONTRACT — the swept-pair admission dispatch contract (theorems A–D names)

Design packet, first of the admission program
([`SWEPT_PAIR_ADMISSION_SPEC.md`](../../docs/SWEPT_PAIR_ADMISSION_SPEC.md),
owner theorems A–D). Freezes the admission contract: the representation
adapter signature (Theorem A), the certificate carrier types (Theorems
B1/B2/C), and the dispatch rule — EVERYTHING refuses `NonCanonicalCarrier`
by default; admission widens case by case behind admitting tests. No solver
code lands here (D-shim discipline): types, refusing constructors, mapping.

```yaml
id:          ADM-000-CONTRACT
contract:    [ADM-000-CONTRACT]
class:       design
crates:      [truck-certified]
depends_on:  [FSSI-000-CONTRACT]
write_allow:
  - vendor/truck/truck-certified/src/construct/admission.rs
  - vendor/truck/truck-certified/src/construct/mod.rs
  - docs/SWEPT_PAIR_ADMISSION_SPEC.md
tests_required:
  - admission_contract_refuses_by_default
  - adapter_signature_types_are_compile_checked
anchors:
  - {id: A1, expect: 27, cmd: "grep -c 'pub mod' vendor/truck/truck-certified/src/construct/mod.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub struct Ssi4System' vendor/truck/truck-certified/src/construct/bie/ssi4.rs"}
  - {id: A3, expect: 7, cmd: "grep -c 'bernstein_box4\\b' vendor/truck/truck-certified/src/interval/bounds.rs"}
budget:      {turns: 45, ctx_tokens: 120000}
```

## Scope decisions (pre-decided per the spec)

1. **The adapter signature (Theorem A).** `admit_tensor_spline_pair(X, Y,
   domain_hint) -> Result<SsiPairSystem, ConstructRefusal>` in the new
   `construct/admission.rs`: consumes two positive-weight tensor-spline
   faces (B-spline/NURBS — Bézier extraction is an exact local change of
   basis per knot rectangle), yields the POLYNOMIALIZED interaction system
   `F = Ŵ_Y·Â − Ŵ_X·B̂` over the extracted spans (positive weights make the
   clearing exact), wired toward the landed `Ssi4System` (the constructor
   itself is ADM-001's; here it REFUSES `InvalidInput` — D-shim).
2. **Certificate carriers.** `RegularPatch { cone: NormalCone }`,
   `CollapsedBoundary { multiplicity: usize }`, `SeamIdentified { paired:
   (EdgeId, EdgeId) }`, `TransversePair { delta: (f64, f64) }` — refusing
   constructors, named tags in the established style, registered in the
   spec's §3 table (the single booking surface).
3. **Dispatch rule.** The boolean boundary consults admission BEFORE the
   `NonCanonicalCarrier` refusal: not-yet-admitted carrier forms keep the
   exact current refusal (V5 rule — the adapter fires only where the old
   path returned `NonCanonicalCarrier`; nothing already-green can reach it).
4. **Mapping rows** in the spec's §3: adapter → Ssi4System; certificates →
   the landed certificate tuples; volume primitive → ADM-003's booking.
5. **Lazy extraction contract**: the signature carries the culling order
   (face pairs by control-hull/AABB, then knot spans) so ADM-001 implements
   it without a signature change; extraction operators cached per knot
   configuration.

## Done when

```
cargo fmt --check -p truck-certified
cargo clippy -p truck-certified --all-targets -- -D warnings
cargo test -p truck-certified --lib
```

with the refusing-by-default tests green and the anchors holding.

## Forbidden

Any numeric method (D-shim). Admitting any real carrier pair (that is
ADM-001+, each with its admitting test). Widening `split.rs` or `classify.rs`
(FSSI-LAYER). New base Refusal variants. H-1: the new module
(`construct/admission.rs`) carries `#![deny(clippy::unwrap_used)]` — no
panics anywhere in the admission layer.

## Stop conditions

- The adapter signature cannot be stated over the landed `Ssi4System`
  without an operator change → SPEC_GAP naming the conflict (the Krawczyk
  operator is frozen).
- The boolean boundary's `NonCanonicalCarrier` origin is not where the
  census recorded it → SPEC_GAP with the actual site.

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `feat(certified): swept-pair admission contract — theorems A-D carriers, refusing-by-default dispatch (ADM-000)`.
