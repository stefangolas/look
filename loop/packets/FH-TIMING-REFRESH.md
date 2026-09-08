# WORK PACKET FH-TIMING-REFRESH — fill the kernel timing column for every facts-green FH row

Mechanical timing packet: for every FH row whose kernel facts gate is GREEN
(turbopump today; nozzle_assembly + mvac once FH-SPLINE-LATHE lands; any
FH-CENSUS-green rows), run the BENCHMARKS-protocol timing series and append
the kernel column to `docs/TT_TIMING_RESULTS.md`. Rows with red gates stay
DNF — no timing is published against a red gate.

```yaml
id:          FH-TIMING-REFRESH
contract:    [FH-TIMING-REFRESH]
class:       mechanical
crates:      []
depends_on:  [FH-SPLINE-LATHE, FH-CENSUS]
write_allow:
  - docs/TT_TIMING_RESULTS.md
read_allow:
  - docs/TT_TIMING_RESULTS.md
  - docs/BENCHMARKS.md
  - corpus/ttc/
tests_required: []
anchors:
  - {id: A1, expect: 9, cmd: "grep -c 'median' docs/TT_TIMING_RESULTS.md"}
  - {id: A2, expect: 1, cmd: "grep -c 'fn run_one' truck123d/compat/runner.rs"}
budget:      {turns: 50, ctx_tokens: 150000}
```

## Protocol (mandatory, per BENCHMARKS doctrine)

- `cargo build --release --locked` first; measured runs ONLY with no
  concurrent cargo/rustc/test process (orchestrator coordinates the quiet
  window — check with the worker before the measured series).
- One fresh python process per run; one unmeasured conditioning run per
  engine per row; five measured runs per engine per row; median reported;
  raw samples retained in the results doc; alternating launch order OCC/
  kernel per row (moot for rows where one engine DNFs).
- Same resolution/deflection parameters on both sides. No cross-row
  averages. DNF rows kept.

## Done when

Every FH row in the results doc carries a facts-gate verdict and, where
green, both engines' medians with raw samples.

## Forbidden

Publishing numbers from a contended machine. Debug builds. Averaging rows.
Timing against a red facts gate. Editing the corpus or the bridge.

## Stop conditions

- The quiet window cannot be established (any concurrent builder) → wait,
  record the wait in RESULT, do not measure contended.
- Kernel facts mismatch a reference → DNF-FACTS with the delta.

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `bench: Falcon-Heavy kernel timing column — facts-gated, medians, raw samples (FH-TIMING-REFRESH)`.
