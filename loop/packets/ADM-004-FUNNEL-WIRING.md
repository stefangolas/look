# WORK PACKET ADM-004-FUNNEL-WIRING — integrate admission into the funnel and hold the V5 line

Design packet, fifth of the admission program. Wires the landed pieces
(adapter, certificates, volume) into the boolean boundary end-to-end for
the admitted classes, and runs the V5 identity battery across every landed
green path. No new theory; integration + the strongest regression net in
the loop.

```yaml
id:          ADM-004-FUNNEL-WIRING
contract:    [ADM-004-FUNNEL-WIRING]
class:       design
crates:      [truck-certified, truck-shapeops, truck123d, look]
depends_on:  [ADM-002-CERTIFICATES, ADM-003-VOLUME]
write_allow:
  - vendor/truck/truck-certified/src/construct/admission.rs
  - vendor/truck/truck-certified/src/construct/mod.rs
  - truck123d/src/facade.rs
  - tests/admission_v5_battery.rs
read_allow:
  - docs/SWEPT_PAIR_ADMISSION_SPEC.md
  - vendor/truck/truck-shapeops/src/boolean/split.rs
  - vendor/truck/truck-certified/src/
tests_required:
  - admitted_pair_cut_certifies_end_to_end
  - v5_battery_every_landed_green_path_bit_identical
  - non_admitted_carriers_refuse_unchanged
anchors:
  - {id: A1, expect: 38, cmd: "grep -c 'ContactLocus' vendor/truck/truck-shapeops/src/boolean/split.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn run_facade' truck123d/src/facade.rs"}
  - {id: A3, expect: 34, cmd: "grep -c 'pub mod' vendor/truck/truck-certified/src/construct/mod.rs"}
budget:      {turns: 75, ctx_tokens: 190000}
```

## Scope decisions (pre-decided)

1. **The locus is emitted in the landed vocabulary.** Admitted pairs
   produce `ContactLocus` per the landed transverse path — the splitter is
   NOT modified (FSSI-LAYER: any splitter change is a stop-and-file). If
   the admitted pipeline cannot express its result in the landed locus
   vocabulary, that is a SPEC_GAP against the spec's §2 row 9, not a
   splitter edit.
2. **The facade's admission consult.** `run_facade` (facade.rs:484)
   consults admission before the `NonCanonicalCarrier` refusal, exactly
   per ADM-000's dispatch rule — canonical dispatch stays ahead.
3. **The V5 battery is the deliverable's spine.** Every landed green path
   (canonical pairs, restricted sweeps, analytic loci, the 12+1 lifted F1
   rows' evidence) answers bit-identically; the battery is a committed
   root test so the program-end verification inherits it.
4. **Determinism.** Same pair → same construction rows → identical output
   bytes (the binding's determinism rule, extended to admitted pairs).

## Done when

```
cargo fmt --check --all -- --check
cargo clippy -p truck-certified -p truck-shapeops -p truck123d --all-targets -- -D warnings
cargo test -p truck-certified --lib --tests
cargo test -p truck-shapeops --lib --tests
cargo test -p truck123d --tests
```

with all three named tests green — in particular the first end-to-end
certified cut between two spline-loft solids.

## Forbidden

Splitter/classifier edits. Approximate answers. Widening admission beyond
the classes with admitting tests. Touching the Krawczyk operator, the F3
rule, or the frozen seam.

## Stop conditions

- The admitted pipeline's result cannot be expressed in the landed locus
  vocabulary → SPEC_GAP naming the mismatch (per scope 1).
- A landed green path flips ANY verdict → defect record, stop-and-file;
  admission widens monotonically or not at all.

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `feat(certified): admission wired into the funnel — first certified swept-pair boolean, V5 battery green (ADM-004)`.
