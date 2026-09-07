# WORK PACKET TTC-TIMING-FH — per-model timing pilot: Falcon-Heavy rows, truck kernel vs OCC door

First OCCT-vs-Truck timing comparison, per-model isolated. Each row reports
its own three columns (wall time, verdict, facts-match) so a blow-up on one
model never contaminates another. Correctness gate: the kernel build must
match the recorded OCC reference facts (solid count, volume, bbox) BEFORE
any timing is reported — a mismatching build reports DNF-FACTS, never a
time.

```yaml
id:          TTC-TIMING-FH
contract:    [TTC-TIMING-FH]
class:       mechanical
crates:      [look]
depends_on:  [TTC-EXECUTOR-BINDING]
write_allow:
  - docs/TT_TIMING_RESULTS.md
read_allow:
  - corpus/ttc/trees/falcon_heavy/src/
  - corpus/ttc/reference/
  - corpus/ttc/door.py
  - docs/BENCHMARKS.md
  - docs/TT_MODEL_CODEPATH_AUDIT.md
tests_required:
  - facts-match gate green for every timed row (kernel facts == recorded
    reference facts within tolerance)
anchors:
  - {id: A1, expect: 14, cmd: "ls corpus/ttc/reference | wc -l"}
  - {id: A2, expect: 1, cmd: "grep -c 'fn run_one' truck123d/compat/runner.rs"}
budget:      {turns: 40, ctx_tokens: 140000}
```

## Scope — rows and method

1. **Method: the REAL surface, no bypass.** Timing runs through the door's
   kernel-engine regime (`--engine truck`, landed by
   TTC-EXECUTOR-BINDING) — the same vendored scripts, both engines. No
   hand-transcribed drivers.
3. **OCC column:** fresh `door.py` runs per row (the recorded references
   provide the facts; the door's stdout provides the wall time).
4. **Protocol (BENCHMARKS doctrine, mandatory):** `cargo build --release`;
   timed runs happen ONLY with no loop workers running (orchestrator
   coordinates the quiet window); alternating launch order OCC/kernel per
   row; 5 measured runs per row per engine, median reported, raw samples
   retained in the results doc; same resolution/deflection parameters on
   both sides (the door's bbox-scaled deflection).
5. **Report:** `docs/TT_TIMING_RESULTS.md`, one table per row —
   time / verdict / facts-match — with DNF rows kept (OCC hang or kernel
   refusal is a RESULT, not a missing line). No averages across rows.

## Done when

```
facts-match gate green for nozzle_assembly + mvac
docs/TT_TIMING_RESULTS.md carries both engines' medians per row
```

## Forbidden

Publishing numbers from a contended machine. Comparing across different
deflection settings. Averaging rows. Skipping the facts gate before
timing. Debug builds.

## Stop conditions

- Kernel facts mismatch the reference on a row → record DNF-FACTS with
  the delta, do not tune the driver to force a match.
- OCC door hangs on a row → record OCC-DNF with the observed duration,
  kill the process, move to the next row.

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `bench: per-model Falcon-Heavy timing — truck kernel vs OCC door, facts-gated (TTC-TIMING-FH)`.
