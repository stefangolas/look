# WORK PACKET ADM-001-ADAPTER — exact Bézier extraction and the first admitted pairs

Design packet, second of the admission program. Lands Theorem A for real:
lazy exact Bézier extraction (knot-rectangle-local, geometry-preserving) of
two positive-weight tensor-spline faces, the polynomialized interaction
`F = Ŵ_Y·Â − Ŵ_X·B̂` over extracted spans, and the FIRST ADMITTED pair
class — ruled-section lofts (where FSSI-004's closed forms give the locus)
— each behind its admitting test. General spline sections follow only with
their own admitting tests; everything else keeps refusing typed.

```yaml
id:          ADM-001-ADAPTER
contract:    [ADM-001-ADAPTER]
class:       design
crates:      [truck-certified]
depends_on:  [ADM-000-CONTRACT]
write_allow:
  - vendor/truck/truck-certified/src/construct/admission.rs
  - vendor/truck/truck-certified/src/construct/bie/ssi4.rs
  - vendor/truck/truck-certified/src/construct/mod.rs
  - vendor/truck/truck-certified/tests/admission_conformance.rs
read_allow:
  - docs/SWEPT_PAIR_ADMISSION_SPEC.md
  - vendor/truck/truck-certified/src/
  - vendor/truck/truck-evidence/src/num/
tests_required:
  - extraction_is_exact_against_knot_insertion_identity
  - polynomialized_F_zeros_equal_direct_difference_on_fixture
  - ruled_section_loft_pair_admits_and_certifies
  - non_admitted_carriers_still_refuse_typed
  - v5_pair_identity_battery_green
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub struct Ssi4System' vendor/truck/truck-certified/src/construct/bie/ssi4.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'hull_bernstein_2d' vendor/truck/truck-certified/src/hull.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'admission' vendor/truck/truck-certified/src/construct/mod.rs"}
budget:      {turns: 85, ctx_tokens: 210000}
```

## Scope decisions (pre-decided per the spec)

1. **Extraction is exact and lazy.** Knot-insertion to Bézier form per
   knot rectangle (a linear change of basis — the zero locus is unchanged);
   extract ONLY surviving spans after control-hull/AABB culling of face
   pairs; extraction operators cached per knot configuration (the cache key
   is the knot configuration, never the geometry instance).
2. **Polynomialized system.** `F = Ŵ_Y·Â − Ŵ_X·B̂` in tensor-Bernstein form
   — the landed `bernstein_box4` exclusion applies to it directly. Degree
   growth (deg F = deg Â + deg Ŵ_Y) is recorded per span, not hidden.
3. **First admitted class: ruled-section lofts.** A loft whose sections are
   line/ruled profiles against another ruled carrier admits via FSSI-004's
   closed forms where applicable and via the continuation otherwise.
   Admitting test REQUIRED per class; general spline sections admit ONLY
   with the T1′ certificates present (ADM-002) — until then they refuse
   typed exactly as today.
4. **V5 battery.** Every landed green pair (analytic, restricted-sweep,
   canonical) answers bit-identically; the adapter is reachable ONLY where
   the old path returned `NonCanonicalCarrier` (structural, per ADM-000's
   dispatch rule).

H-1/H-3/H-6, SFC, single-interval-algebra rule: inherited verbatim.

## Done when

```
cargo fmt --check -p truck-certified
cargo clippy -p truck-certified --all-targets -- -D warnings
cargo test -p truck-certified --lib --tests
```

with all five named tests green and the anchors holding.

## Forbidden

Materializing full-solid extractions. Float predicates in any certificate.
Weakening the F3 rule. Splitter/classifier writes (FSSI-LAYER). Admitting
general spline sections before ADM-002 lands.

## Stop conditions

- Extraction on a corpus-derived loft face produces coefficients whose
  interval evaluation cannot close (weight sign, degree blow-up) →
  SPEC_GAP with the face class named.
- The polynomialized `F` cannot reach `Ssi4System` without an operator
  change → stop-and-file (the operator is frozen).

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `feat(certified): exact Bezier extraction adapter — polynomialized pair system, ruled-section admission first (ADM-001)`.
