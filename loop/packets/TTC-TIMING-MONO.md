# WORK PACKET TTC-TIMING-MONO — per-model timing: F1 monocoque, the hard-model entry

The F1 survival tub: `cut(swept,canonical)` (lofted skin minus cavity) +
`fuse(swept,canonical)` (proud bosses) — the highest-prevalence capability
cells on the marquee F1 part. Timing runs ONLY after its machinery is
landed and its row is lifted; this packet is the timing + facts
adjudication, not the lift (PB-011 owns that).

```yaml
id:          TTC-TIMING-MONO
contract:    [TTC-TIMING-MONO]
class:       mechanical
crates:      [look]
depends_on:  [PB-011-TTC-PARITY-CHURN, TTC-EXECUTOR-BINDING]
write_allow:
  - docs/TT_TIMING_RESULTS.md
read_allow:
  - corpus/ttc/trees/f1/src/
  - corpus/ttc/reference/
  - corpus/ttc/door.py
  - docs/TT_TIMING_RESULTS.md
  - docs/BENCHMARKS.md
tests_required:
  - facts-match gate green for f1/monocoque (kernel facts == OCC door
    facts within tolerance) before any timing is reported
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'build_monocoque' corpus/ttc/MANIFEST.json"}
budget:      {turns: 30, ctx_tokens: 120000}
```

## Scope

1. **Method: the REAL surface.** `mono_tub.py`'s build entry runs through
   the door's kernel-engine regime (`--engine truck`) — the same script
   OCC ran, per the executor binding. No transcription.
2. **OCC column:** fresh door run of `f1/monocoque` (its reference may
   need recording first — PB-011's lift run produces it; reuse that).
3. **Protocol:** identical to TTC-TIMING-FH (release build, quiet
   machine, alternating order, 5 runs, median, DNF rows kept, no
   cross-row averages). The boolean stage is timed as its own segment
   within the row (construction vs boolean vs tessellation) — the
   segmentation is the interesting part on this model.
4. **Report:** appended to `docs/TT_TIMING_RESULTS.md` as its own table.

## Done when

```
facts-match gate green for f1/monocoque
docs/TT_TIMING_RESULTS.md carries the monocoque table (both engines, three segments)
```

## Forbidden

Publishing contended numbers. Comparing across deflection settings.
Averaging rows. Timing before the facts gate.

## Stop conditions

- The kernel boolean refuses on the tub (typed) → record the refusal
  verbatim as the row's verdict (DNF-KERNEL), file the finding for the
  funnel programs, do not tune.
- OCC door fails the tub → record OCC-DNF (the corpus documents OCC
  trouble on this class); the comparison then reports one-sided time +
  the failure.

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `bench: F1 monocoque timing — segmented construction/boolean/tessellation, facts-gated (TTC-TIMING-MONO)`.
