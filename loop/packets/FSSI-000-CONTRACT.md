# WORK PACKET FSSI-000-CONTRACT — FSSI topology contract: names, carriers, mapping

Re-authored 2026-09-08 (orchestrator). The previous draft of this packet
anchored on a mis-cited substrate (`SsiRefusal` cited at
`ssi_types.rs:97-116`; the enum actually lives at `src/ssi.rs:97` —
`ssi_types.rs` is the CFP wave shim). The build spec
(`docs/FSSI_BUILD_SPEC.md`, now committed) is corrected at the source;
this packet anchors only on measured tree facts.

Freezes the FSSI program's refusal names, verdict carriers, and evidence
mapping BEFORE any solver code ([`FSSI_BUILD_SPEC.md`](../../docs/FSSI_BUILD_SPEC.md)
§3, packet 1 of 5). Everything downstream builds against these names. The
D-shim rule applies in full: types and refusing constructors only — any
method that would evaluate, solve, isolate, or certify NUMERICALLY refuses.

```yaml
id:          FSSI-000-CONTRACT
contract:    [FSSI-000-CONTRACT]
class:       design
crates:      [truck-certified]
depends_on:  []
write_allow:
  - vendor/truck/truck-certified/src/ssi.rs
  - vendor/truck/truck-certified/src/ssi_trace.rs
  - vendor/truck/truck-certified/src/construct/bie/ssi4.rs
  - docs/CERTIFICATE_MAPPING.md
read_allow:
  - docs/FSSI_BUILD_SPEC.md
  - vendor/truck/truck-certified/src/
  - vendor/truck/truck-evidence/src/num/
tests_required:
  - refusal_tags_stable
  - downstream_matches_exhaustive_compile
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'enum SsiRefusal' vendor/truck/truck-certified/src/ssi.rs"}
  - {id: A2, expect: 5, cmd: "grep -c 'Switched' vendor/truck/truck-certified/src/ssi_trace.rs"}
  - {id: A3, expect: 9, cmd: "grep -c 'KrawczykSystem' vendor/truck/truck-evidence/src/num/krawczyk.rs"}
  - {id: A4, expect: 0, cmd: "grep -c 'TangentCurveSuspected' vendor/truck/truck-certified/src/ssi.rs"}
budget:      {turns: 50, ctx_tokens: 120000}
```

## Pre-digested context (all citations re-verified against the tree 2026-09-08)

- The landed substrate this contract names into (do not rebuild any of it):
  cofactor 3-of-4 minor selection (`ssi4.rs`), the frozen F3 rule
  (`src/contract.rs`, 8 mentions), the generic Krawczyk operator +
  parallelotope tube (`truck-evidence/src/num/krawczyk.rs` — `KrawczykSystem`
  ×9, `num/parallelotope.rs`), `TraceOutcome::Switched` fold bridge
  (`ssi_trace.rs`, ×5), CFP-008's stagnation verdicts and the BG-KV2-207B
  escalation lattice.
- The `SsiRefusal` enum is at `src/ssi.rs:97` (single definition; 85
  references in that file; `Conditioning` ×24 — the refusal sites FSSI-002
  later swaps). Add the new variants THERE, not in `ssi_types.rs`.
- A4 = 0 is the point: `TangentCurveSuspected` does not exist yet. If it is
  nonzero at dispatch, the tree moved — STOP and report (H-8).
- House refusal layering (`truck-certified/src/contract.rs`): no new
  variants on the base `truck_base::evidence::Refusal`; named cases land in
  the layered local vocabularies. This packet adds them to `SsiRefusal`.
- FSSI-REFUSAL: named cases only, no catch-all. FSSI-EXT: pure extension —
  existing variants, tags, and verdict classes are untouched.

## Scope decisions

1. **New `SsiRefusal` variants** (with `tag()` strings in the established
   style): `TangentCurveSuspected { margin: (f64, f64) }` — the undecided
   measure and its shrink exponent at halt; `CoincidentPatchSuspected
   { margin: (f64, f64) }`. The payload records WHY the suspicion halt
   fired; it is evidence, never a tolerance.
2. **New fold-verdict carrier** `FoldCert { chart: usize, sigma: i8,
   det_enclosure: (f64, f64) }` — refusing constructor; FSSI-002-FOLD
   populates it. Registered as a NEW tier of the escalation lattice in the
   mapping doc; CFP-008's stagnation verdicts stand unchanged (FSSI-EXT).
3. **Mapping-table rows in `docs/CERTIFICATE_MAPPING.md`** (exactly three):
   fold event → `FoldCert` + which existing certificate tuple carries it;
   per-segment projection index → the arc type extension FSSI-003 freezes
   (carrier named now, populated then); tube certificate → the landed
   parallelotope proof type. The table is the single booking surface —
   FSSI-003/005 may not invent new evidence positions.
4. **Exhaustive-match ripple**: adding the two `SsiRefusal` variants breaks
   every exhaustive match. Update the arms in `ssi.rs`, `ssi_trace.rs`,
   `ssi4.rs` to a named `unreachable`-free arm that refuses with the new
   tag (these sites cannot fire until FSSI-001/002 wire the producers —
   the arm documents that, it does not guess).

## Done when

```
cargo fmt --check -p truck-certified
cargo clippy -p truck-certified --all-targets -- -D warnings
cargo test -p truck-certified --lib
```

with the tag-stability test green and every anchor holding.

## Forbidden

Any numeric method (D-shim). New base-`Refusal` variants. Editing the
Krawczyk operator, the F3 rule, or any landed verdict class. Weakening any
landed test. Touching `src/ssi_types.rs` (it is NOT the enum's home — the
previous draft's error).

## Stop conditions

- An exhaustive-match ripple reaches a file outside `write_allow` →
  SPEC_GAP with the file named (do not widen the write set silently).
- A4 nonzero at start → the tree moved since 2026-09-08; report, do not
  proceed.

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `feat(certified): FSSI topology contract — gate refusal names, fold verdict carrier, evidence mapping (FSSI-000)`.
