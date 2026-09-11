# WORK PACKET AUTHOR-CENSUS-NAMES — complete the door's census vocabulary

SWARM FINDING 2026-09-10: eight census names are absent from the
door's vocabulary (`_TRUCK_NAMES`), so 9 of 13 hypercar rows die with
UNTYPED AttributeErrors before any typed verdict — the worst failure
class (no carrier named, no boundary measured):
`RectangleRounded`, `Align`, `Cone`, `Helix`, `Ellipse`,
`RegularPolygon`, `FilletPolyline`, `make_hull`.

```yaml
id:          AUTHOR-CENSUS-NAMES
contract:    [AUTHOR-CENSUS-NAMES]
class:       mechanical
crates:      [truck123d]
depends_on:  [MONO-7-ROW-ASSEMBLY]
write_allow:
  - corpus/ttc/door.py
  - truck123d/src/bd_bridge.rs
  - truck123d/tests/census_names.rs
read_allow:
  - truck123d/src/binding.rs
tests_required: [truck123d/tests/census_names.rs]
anchors:
  - {id: A1, expect: 0, cmd: "grep -c 'RectangleRounded' corpus/ttc/door.py"}
  - {id: A2, expect: 0, cmd: "grep -c 'make_hull' corpus/ttc/door.py"}
  - {id: A3, expect: 0, cmd: "grep -c 'Ellipse' truck123d/src/bd_bridge.rs"}
  - {id: A4, expect: 0, cmd: "grep -c 'SolidSpec::Cone' truck123d/src/bd_bridge.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'enum ProfileEdge' truck123d/src/bd_bridge.rs"}
budget:      {turns: 70, ctx_tokens: 220000}
```

AMENDED 2026-09-11 (owner session) per the parked SPEC_GAP QUESTION
(slot-0 tip `46ff8cc`, measured at its worktree HEAD `82dcf32`): the
executor section vocabulary `ProfileEdge` (bd_bridge.rs) carries exactly
`Line`/`Spline`/`Circle` — **no ellipse carrier and no arc carrier** — and
`SolidSpec` carries no `Cone`. TIER A therefore needs real executor
carriers, exactly the DOOR-CIRCLE-FLIP precedent (which landed
`ProfileEdge::Circle`). A3–A5 are the executor pre-state anchors; the
anchor set was re-measured at HEAD `cb1fabf` (A1/A2 = 0, A3 = 0, A4 = 0,
A5 = 1). Re-measure at dispatch.

## Pre-made judgements (tiered by what the kernel can answer EXACTLY)

1. **TIER A — record exactly now:** `Align` (pure placement metadata —
   pass-through like styled, no geometry), `Cone` (canonical solid,
   SolidSpec variant — the landed canonical facts path), `Ellipse`
   (exact analytic profile carrier, same treatment as the Circle
   flip), `RegularPolygon` (route through the EXISTING polygon profile
   handler — it is a polygon with a vertex count), `RectangleRounded`
   (a closed profile: four lines + four exact quarter-arc segments —
   recorded in the line/arc section vocabulary).
2. **TIER B — typed refusal NAMING the open carrier (still an
   improvement over AttributeError):** `Helix` (path carrier — needs a
   helical sweep extension, out of scope), `FilletPolyline` (needs the
   fillet-arm composition), `make_hull` (convex-hull op — a new
   construction class).
3. Every TIER A name must round-trip exact facts; every TIER B name
   must refuse TYPED (never AttributeError). The hypercar corpus rows
   then produce honest verdicts either way.
4. **AMENDED** — bd_bridge.rs is in the write set for the executor
   carriers TIER A needs, following DOOR-CIRCLE-FLIP: `ProfileEdge::Ellipse`
   (exact analytic profile carrier), the arc segments
   `RectangleRounded` needs (exact quarter-arc edges composing a closed
   profile through `profile_loop`), and the `SolidSpec::Cone` canonical
   variant. These are the QUESTION's named gap; nothing further. If more
   bridge surface than that turns out to be needed, STOP with QUESTION.md.

## Method

Tier A first, one name end-to-end (record -> bd_facts green) before
the next; Tier B as one-line typed refusals in the vocabulary table.
Tests: one green round-trip per Tier A name; one typed refusal per
Tier B name; a hypercar-shaped composition (RectangleRounded profile
extruded, aligned, coned) green. Cargo through the queue; DLL
workaround on PATH if 0xc0000135.

## Done when

Scoped checks green; anchors hold (A1/A2 -> nonzero). Write
RESULT.json AT THE WORKTREE ROOT.
