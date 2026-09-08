# WORK PACKET FSSI-000-CONTRACT — FSSI topology contract: names, carriers, mapping (r2)

**r2 — orchestrator amendment after the r1 SPEC_GAP**
(`loop/results/FSSI-000-CONTRACT.SPEC_GAP.json`). The r1 worker stopped
cleanly before any kernel change: the exhaustive-match ripple reaches
`tangency/chart.rs` and `tangency/a2.rs` (outside the r1 write set), the
`Eq` derive chain conflicts with the new payloads, and the root-harness
disposition function needs buckets. All four re-author decisions are
PRE-DECIDED below — this packet freezes them; the worker executes.

Freezes the FSSI program's refusal names, verdict carriers, and evidence
mapping BEFORE any solver code ([`FSSI_BUILD_SPEC.md`](../../docs/FSSI_BUILD_SPEC.md)
§3, packet 1 of 5). The D-shim rule applies in full: types and refusing
constructors only — any method that would evaluate, solve, isolate, or
certify NUMERICALLY refuses.

```yaml
id:          FSSI-000-CONTRACT
contract:    [FSSI-000-CONTRACT]
class:       design
crates:      [truck-certified]
depends_on:  []
write_allow:
  - vendor/truck/truck-certified/src/ssi.rs
  - vendor/truck/truck-certified/src/ssi_trace.rs
  - vendor/truck/truck-certified/src/tangency/chart.rs
  - vendor/truck/truck-certified/src/tangency/a2.rs
  - vendor/truck/truck-certified/src/ssi_admit.rs
  - tests/certified_phase2_floor.rs
  - docs/CERTIFICATE_MAPPING.md
read_allow:
  - docs/FSSI_BUILD_SPEC.md
  - loop/results/FSSI-000-CONTRACT.SPEC_GAP.json
  - vendor/truck/truck-certified/src/
  - vendor/truck/truck-evidence/src/num/
tests_required:
  - refusal_tags_stable
  - downstream_matches_exhaustive_compile
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'enum SsiRefusal' vendor/truck/truck-certified/src/ssi.rs"}
  - {id: A2, expect: 5, cmd: "grep -c 'Switched' vendor/truck/truck-certified/src/ssi_trace.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'enum TangencyRefusal' vendor/truck/truck-certified/src/tangency/chart.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'fn map_ssi' vendor/truck/truck-certified/src/tangency/a2.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'enum SsiAdmitCause' vendor/truck/truck-certified/src/ssi_admit.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'disposition_of_ssi_refusal' tests/certified_phase2_floor.rs"}
budget:      {turns: 55, ctx_tokens: 130000}
```

## Pre-decided decisions (frozen — the worker does not relitigate)

1. **The two variants** land on `SsiRefusal` (ssi.rs:97):
   `TangentCurveSuspected { margin: (f64, f64) }` and
   `CoincidentPatchSuspected { margin: (f64, f64) }` — the undecided measure
   and its shrink exponent at halt. Evidence, never a tolerance.
2. **Tag strings** (stable, asserted by `refusal_tags_stable`):
   `ssi_tangent_curve_suspected`, `ssi_coincident_patch_suspected`.
3. **Eq chain**: drop `Eq` from `SsiRefusal` (ssi.rs:96),
   `TangencyRefusal` (chart.rs:67), and `SsiAdmitCause` (ssi_admit.rs:61);
   keep `Debug, Clone, Copy, PartialEq`. Measured 2026-09-08: nothing in
   `truck-certified` uses these types as map keys or in `==` (the derives
   are unused load). Do NOT touch the enum at ssi.rs:812 (different type,
   carries Ord for the F3 selection).
4. **Tangency-layer mapping**: both new cases map in `map_ssi` (a2.rs) to
   the existing low-information pattern
   `TangencyRefusal::Input(Refusal::InvalidInput)` — NO new
   `TangencyRefusal` variant (FSSI-EXT). The SSI-layer `tag()` carries the
   specificity; FSSI-001/002 producers will assert against it.
5. **Root harness**: `disposition_of_ssi_refusal`
   (tests/certified_phase2_floor.rs:672) gains a pre-decided bucket for the
   two new variants (they are refusals → the refused bucket). Existing
   buckets unchanged (V5 identity).
6. **ssi_trace.rs** gets the exhaustive-match arms; **ssi4.rs is OUT of
   scope** (r1 measured: nothing to update there).

## Pre-digested context (re-verified against the tree 2026-09-08)

- Landed substrate, do not rebuild: cofactor 3-of-4 minor selection
  (`construct/bie/ssi4.rs`), the frozen F3 rule (`src/contract.rs`),
  the generic Krawczyk operator + parallelotope tube
  (`truck-evidence/src/num/krawczyk.rs` — `KrawczykSystem` ×9,
  `num/parallelotope.rs`), `TraceOutcome::Switched` fold bridge
  (`ssi_trace.rs`, ×5), CFP-008 stagnation verdicts, BG-KV2-207B escalation
  lattice.
- `SsiRefusal` is at `src/ssi.rs:97`; `TangentCurveSuspected` count = 0
  today (the variants do not exist; if nonzero at dispatch, the tree moved
  — STOP, H-8).
- FSSI-REFUSAL: named cases only, no catch-all. FSSI-EXT: pure extension.

## Scope decisions

1. The two `SsiRefusal` variants + `FoldCert { chart: usize, sigma: i8,
   det_enclosure: (f64, f64) }` refusing-constructor carrier (FSSI-002
   populates it; registered as a NEW escalation-lattice tier in the mapping
   doc; CFP-008 stagnation verdicts stand unchanged).
2. Mapping-table rows in `docs/CERTIFICATE_MAPPING.md` (exactly three):
   fold event → `FoldCert` + the existing certificate tuple carrying it;
   per-segment projection index → the arc type extension FSSI-003 freezes;
   tube certificate → the landed parallelotope proof type. The table is the
   single booking surface.
3. Exhaustive-match ripple: `ssi.rs`, `ssi_trace.rs`, `tangency/chart.rs`,
   `tangency/a2.rs` — each new arm refuses with the pre-decided tag (the
   sites cannot fire until FSSI-001/002 wire the producers; the arm
   documents that).

## Done when

```
cargo fmt --check -p truck-certified
cargo clippy -p truck-certified --all-targets -- -D warnings
cargo test -p truck-certified --lib
```

with the tag-stability test green, every anchor holding, and the root
harness (certified_phase2_floor) green at merged HEAD.

## Forbidden

Any numeric method (D-shim). New base-`Refusal` variants. New
`TangencyRefusal` variants. Editing the Krawczyk operator, the F3 rule, or
any landed verdict class. Weakening any landed test. Touching
`src/ssi_types.rs` or `construct/bie/ssi4.rs`.

## Stop conditions

- An exhaustive-match ripple reaches a file outside `write_allow` →
  SPEC_GAP with the file named (do not widen the write set silently).
- A `TangentCurveSuspected` nonzero count at start → the tree moved; report.

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `feat(certified): FSSI topology contract — gate refusal names, fold verdict carrier, evidence mapping (FSSI-000)`.
