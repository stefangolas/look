# WORK PACKET TTC-RECENSUS-F1 — the closing re-census: every F1 row re-run on the admitted kernel

Mechanical closing packet: with admission landed (ADM-004) and the
authoring arms landed (F1-AUTHORING-ARMS), re-run ALL 21 F1 manifest rows
through the kernel door and produce the final coverage table. Rows flip
typed-refusal → certified-lift ONLY with facts matching the recorded
references; rows that still refuse keep their typed verdicts — recorded,
with the refusing op named, as the permanent boundary evidence.

```yaml
id:          TTC-RECENSUS-F1
contract:    [TTC-RECENSUS-F1]
class:       mechanical
crates:      [look]
depends_on:  [ADM-004-FUNNEL-WIRING, F1-AUTHORING-ARMS]
write_allow:
  - docs/TTC_CENSUS_FINAL.md
  - docs/TT_TIMING_RESULTS.md
read_allow:
  - corpus/ttc/
  - docs/TT_TIMING_RESULTS.md
  - loop/results/PB-011C-TTC-PARITY-CHURN.json
  - docs/FH_CENSUS.md
tests_required: []
anchors:
  - {id: A1, expect: 48, cmd: "grep -c '\"id\"' corpus/ttc/MANIFEST.json"}
  - {id: A2, expect: 21, cmd: "grep -c 'median' docs/TT_TIMING_RESULTS.md"}
budget:      {turns: 60, ctx_tokens: 170000}
```

## Method

1. The PB-011C protocol verbatim: fresh python per row, serial, quiet
   machine; typed refusals recorded verbatim with the refusing verb;
   green rows facts-compared against the recorded references.
2. For every row that flips to green: the BENCHMARKS timing series
   (release regime, five measured runs per engine, medians, raw samples)
   appends the kernel column to `docs/TT_TIMING_RESULTS.md` — the timing
   table closes row by row.
3. The final coverage doc: per row — verdict (green/typed-refusal/DNF),
   the refusing op where applicable, the carrier classes, and the delta
   against the pre-admission census (which refusals admission cured, which
   it did not).
4. Rows still refusing are NOT failures of this packet — they are the
   measured boundary, and their census rows are the booking evidence for
   whatever follow-up (FSSI-002/003 activation, torus program) the
   distribution demands.

## Done when

`docs/TTC_CENSUS_FINAL.md` carries all 21 rows' final verdicts with the
delta table; the timing table carries a kernel column for every green row;
anchors hold.

## Forbidden

Tuning anything to flip a row. Concurrent door runs. Publishing timing
against a red facts gate. Declaring a row lifted without facts matching.

## Stop conditions

- A row's recorded reference has drifted from the current OCC door →
  record the discrepancy, do not adjudicate the kernel against a moving
  baseline.
- An OCC flake appears (the Null TopoDS_Shape regime) → re-run serial and
  quiet before recording; the corpus documents the regime.

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `census: F1 closing re-census — post-admission coverage table, timing column closed row by row (TTC-RECENSUS-F1)`.
