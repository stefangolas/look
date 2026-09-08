# WORK PACKET FH-CENSUS — kernel-door sweep over the untested Falcon-Heavy rows

Mechanical census packet (the PB-011C protocol applied to FH): run
`door.py --engine truck` on every FH canonical row that has never seen the
kernel door (chamber_assembly, feed_assembly, gas_generator_assembly,
second_stage, thrust_structure, turbine_exhaust_assembly) and record the
per-row, per-op verdict table. Typed refusals are valid outcomes — this is
a measurement, not a lift packet. NO timing (that is FH-TIMING-REFRESH's,
against green rows only).

```yaml
id:          FH-CENSUS
contract:    [FH-CENSUS]
class:       mechanical
crates:      []
depends_on:  [FH-SPLINE-LATHE]
write_allow:
  - docs/FH_CENSUS.md
read_allow:
  - corpus/ttc/
  - docs/TT_TIMING_RESULTS.md
  - loop/results/PB-011C-TTC-PARITY-CHURN.json
tests_required: []
anchors:
  - {id: A1, expect: 48, cmd: "grep -c '\"id\"' corpus/ttc/MANIFEST.json"}
  - {id: A2, expect: 1, cmd: "grep -c 'chamber_assembly' corpus/ttc/MANIFEST.json"}
budget:      {turns: 50, ctx_tokens: 150000}
```

## Method (identical to PB-011C's, do not innovate)

1. One fresh python process per run, serial (the documented under-load flake
   regime — never concurrent door runs).
2. Per row: the door's typed refusal is recorded VERBATIM (kind, message,
   payload case/envelope) with the census verb that triggered it.
3. Rows whose facts come back must be compared against
   `corpus/ttc/reference/<row>.json` — a mismatch is DNF-FACTS with the
   delta recorded, never tuned.
4. The census doc carries one table: row | first refusing verb | carrier
   classes in collision | verdict | wall_s. Stagnation is recorded with its
   budget; typed refusals with their case.

## Done when

`docs/FH_CENSUS.md` carries all six rows' verdicts (green-with-facts or
typed-refusal, no gaps) and the anchors hold.

## Forbidden

Tuning the door or the bridge to force a verdict. Timing anything. Editing
the corpus scripts. Concurrent door runs.

## Stop conditions

- A row's OCC-baseline reference is missing from `corpus/ttc/reference/` →
  record the row UNSTAGED, move on.
- An OCC door run fails on a row where the recorded reference exists →
  record OCC-DNF with the observed duration, move on.

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `census: Falcon-Heavy kernel-door sweep — six untested rows, per-op verdicts (FH-CENSUS)`.
