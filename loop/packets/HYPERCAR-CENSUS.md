# WORK PACKET HYPERCAR-CENSUS — first kernel-door contact for every hypercar row

Mechanical census packet: the hypercar family (body, brakes, chassis,
engine_prototype, fairing, glazing, harness_assembly, hinge, interior,
lighting, powertrain, vehicle, wheels — every manifest row in the hypercar
family) has NEVER run against the kernel door. This packet produces the
same per-row, per-op verdict table PB-011C produced for F1. Typed refusals
are valid outcomes.

```yaml
id:          HYPERCAR-CENSUS
contract:    [HYPERCAR-CENSUS]
class:       mechanical
crates:      [look]
depends_on:  []
write_allow:
  - docs/HYPERCAR_CENSUS.md
read_allow:
  - corpus/ttc/
  - docs/TT_TIMING_RESULTS.md
  - loop/results/PB-011C-TTC-PARITY-CHURN.json
tests_required: []
anchors:
  - {id: A1, expect: 48, cmd: "grep -c '\"id\"' corpus/ttc/MANIFEST.json"}
  - {id: A2, expect: 1, cmd: "grep -c 'chassis' corpus/ttc/MANIFEST.json"}
budget:      {turns: 60, ctx_tokens: 160000}
```

## Method (the PB-011C protocol, verbatim)

1. One fresh python process per run, serial, quiet machine.
2. Per row: typed refusal recorded VERBATIM (kind, message, payload
   case/envelope) with the first refusing census verb; green rows compared
   against `corpus/ttc/reference/<row>.json` — a mismatch is DNF-FACTS with
   the delta, never tuned.
3. Census doc table: row | first refusing verb | carrier classes in
   collision | verdict | solid_count | STL tris | wall_s.
4. The output is the hypercar family's coverage map — which rows are
   authoring-blocked vs boolean-blocked vs green — feeding the same
   admission program the F1 census booked.

## Done when

`docs/HYPERCAR_CENSUS.md` carries every hypercar manifest row's verdict,
no gaps, anchors holding.

## Forbidden

Tuning anything to force a verdict. Timing. Concurrent door runs. Editing
the corpus scripts.

## Stop conditions

- A row is unstaged (no recorded reference) → record UNSTAGED, move on.
- An OCC door failure where a reference exists → record OCC-DNF, move on.

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `census: hypercar kernel-door sweep — first contact, per-op verdicts (HYPERCAR-CENSUS)`.
