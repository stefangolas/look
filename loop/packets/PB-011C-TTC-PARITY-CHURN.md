# WORK PACKET PB-011C-TTC-PARITY-CHURN — the swept×swept wave: the 4-D deep end on real corpus geometry

The last F1 capability wave: rows whose first refusing op pairs TWO
swept/lofted carriers — the ssi4 parallelotope continuation's first real
corpus exposure. Expect stop-and-files: this packet is where the funnel's
envelope meets the hardest geometry the corpus has.

```yaml
id:          PB-011C-TTC-PARITY-CHURN
contract:    [PB-011C-TTC-PARITY-CHURN]
class:       design
crates:      [truck123d, truck-evidence]
depends_on:  [PB-011B-TTC-PARITY-CHURN]
write_allow:
  - truck123d/src/facade.rs
  - truck123d/compat/surface.rs
  - truck123d/tests/pb_parity.rs
  - docs/PY_BRIDGE_COMPAT_SURFACE.md
  - corpus/ttc/SKIPS.json
read_allow:
  - corpus/ttc/
  - loop/results/PB-011-TTC-PARITY-CHURN.json
  - loop/results/PB-011B-TTC-PARITY-CHURN.json
  - docs/CONTACT_FAST_PATH_BUILD_SPEC.md
tests_required:
  - lifted_skip_rows_run_green (the C lifted set, iterated; empty set
    fails ONLY if every row files a typed refusal census record — record
    which)
anchors:
  - {id: A1, expect: 35, cmd: "grep -c booleans-on-swept-carriers corpus/ttc/SKIPS.json"}
budget:      {turns: 80, ctx_tokens: 200000}
```

## Pre-digested context

- **C's rows (swept×swept class):** f1/airbox, f1/beam_wing, f1/details,
  f1/engine_cover, f1/power_unit, f1/rear_wing, f1/drs_flap,
  f1/drivetrain (8 rows; drivetrain also carries canonical cuts — its
  lift may be partial: lift it only if its swept×swept ops certify).
- The 4-D path: `truck-certified/src/construct/bie/ssi4.rs` — the
  restricted-pair interaction solver over (u,v,s,t); stagnation routes to
  certified verdicts (CFP-008). B's wave will have banked the simpler
  rows; C's rows are where budget burn and typed stagnation verdicts
  concentrate.
- Per-row complexity keys are in `scratch/reference_batch_log.json`
  (engine_cover 98s, power_unit 85s, drivetrain 72s — the heaviest).
- Atomic lifts, cheapest-first, exactly PB-011B's protocol.

## Scope decisions

1. Same lift discipline as B, with ONE addition: a row whose kernel run
   ends in `NumericallyUnresolved` (budget/stagnation) is recorded as a
   STAGNATION census row (typed, budget numbers attached) — not a defect
   and not a lift. The census distinguishes: certified-lift /
   typed-refusal (unsupported carrier) / stagnation (budget) / facts
   mismatch (defect, stop-and-file).
2. Expect and WELCOME the first real 4-D exposure: every stagnation or
   refusal record names the stratum pair and the budget spent — that is
   the data the next funnel program books against.
3. Vendor/truck is read-only. A funnel defect (wrong geometry, not a
   refusal) → stop-and-file immediately.

## Done when

```
cargo fmt --check -p truck123d
cargo clippy -p truck123d -p truck-evidence --all-targets -- -D warnings
cargo test -p truck123d
python corpus/ttc/census.py
```

## Forbidden

vendor/truck writes. Lifting without green facts. Skipping the census
record for any row outcome.

## Stop conditions

- A facts mismatch → stop-and-file.
- All 8 rows end stagnation/refusal → valid DONE-with-census; the
  results book the next funnel program.

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `feat(bridge): swept-swept F1 lifts — the 4-D wave, census-first (PB-011C)`.
