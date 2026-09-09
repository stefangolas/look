# TTC-CENSUS-FINAL — F1 closing re-census: every F1 row re-run on the admitted kernel

Closing mechanical census (PB-011C protocol applied verbatim to the F1 family at the
post-admission HEAD): with the certified funnel's admission wired (ADM-004) and the
authoring arms landed (F1-AUTHORING-ARMS), ALL 21 F1 manifest rows
(`corpus/ttc/MANIFEST.json`, family `f1`) were re-run through the kernel door —
`corpus/ttc/door.py --engine truck` (the kernel-engine door regime, `door_version 2`),
one fresh Python process per row, serial, quiet machine. Rows flip
`typed-refusal → certified-lift` ONLY when the kernel door returns geometry facts equal
to the recorded OCC reference within the recorded tolerances; rows that still refuse
keep their typed verdicts — recorded verbatim, with the refusing op named — as the
permanent boundary evidence. Rows that die untyped on the drop-in data surface are
recorded as kernel-door DNF (untyped), exactly as FH-CENSUS and HYPERCAR-CENSUS
recorded the same class.

## Result in one line

**No F1 row certifies on this dispatch.** 0 green / certified-lift, 12 typed-refusal,
9 kernel-door DNF (untyped). Every row stops in AUTHORING — one or more carriers
before its recorded boolean — because the F1 section-frame / placement / profile
authoring surface (`Plane` frame algebra, `Pos` placement, `Spline` profile authoring)
is still recorded client-layer data on the truck drop-in, and the drop-in `Vector`
data row does not yet answer the corpus's direction math (`.normalized`, `.X`,
scalar/difference arithmetic). No row reaches the boolean/admission layer, so the
ADM-004 admission state is not exercised by any F1 row's kernel-door run; no row's
recorded OCC reference drifted (all 21 OCC-baseline runs reproduce the recorded
`corpus/ttc/reference/<row>.json` bit-identically).

## Machine / environment

- Windows x86_64 (win32), census run 2026-09-09 on the autobuild host, quiet window
  (no concurrent `cargo`/`rustc` during the runs).
- Python 3.14.3; the truck regime's native executor is the release-built `truck123d`
  module at this worktree HEAD, staged as `truck123d.pyd` beside the interpreter
  runtime (rebuilt at this HEAD; the stale pre-authoring-arms staging was replaced).
- Worktree HEAD `07b2090`, branch `packet/TTC-RECENSUS-F1`.
- Kernel door: `door_version 2` (`--engine truck`). OCC-baseline door: `door_version 1`.
- Reference rows all present for all 21 rows (`corpus/ttc/reference/*.json`); no
  UNSTAGED stop condition fired.

## Method

Per row, exactly the PB-011C census protocol: one fresh Python process,
`python corpus/ttc/door.py --engine truck <tree> <module> <entry> <args> <stl>`,
serial, quiet. A second serial pass confirmed determinism (identical verdicts on both
passes). Green rows would be facts-compared against the recorded reference; none was
green, so no facts-vs-reference kernel gate ran and no kernel timing is published.
Each row also ran once through the OCC-baseline door (`--engine occ`, `door_version 1`)
for the door-evidence columns and reference reproduction; 21/21 OCC runs were green and
reproduced the recorded references bit-identically (the recorded tolerances were not
needed — facts were bit-equal).

## Census verdict table — all 21 F1 rows

Columns: kernel-door verdict; first refusing verb / carrier in collision; carrier
classes in collision; kernel wall (s); OCC-baseline evidence (solid count, STL
triangles, OCC door wall s); pre-admission census record (marker in
`corpus/ttc/SKIPS.json`).

| row | verdict | refusing verb / carrier | carrier classes in collision | kernel wall (s) | OCC solids | OCC STL tris | OCC wall (s) | pre-admission census |
|---|---|---|---|---:|---:|---:|---:|---|
| f1/airbox | kernel-door DNF (untyped) | `Vector` (direction math, ctor) | `surfaces.plate_plane` `bd.Vector(...).normalized()` (surfaces.py:696) vs the drop-in `Vector` data row | 0.061 | 6 | 4003 | 12.222 | PB-011C CENSUS (typed-refusal) |
| f1/beam_wing | typed-refusal | `Spline` (profile authoring) | airfoil profile authoring `bd.Spline` (surfaces.py:392) vs the drop-in profile vocabulary | 0.061 | 2 | 1994 | 4.917 | PB-011C CENSUS (typed-refusal) |
| f1/corner_fl | typed-refusal | `Pos` (placement) | corner placement `bd.Pos` (wheels.py:963) vs the drop-in placement vocabulary | 0.063 | 21 | 33970 | 11.001 | PB-011B LIFT EVIDENCE |
| f1/corner_fr | typed-refusal | `Pos` (placement) | corner placement `bd.Pos` (wheels.py:963) vs the drop-in placement vocabulary | 0.057 | 21 | 33960 | 10.802 | PB-011B LIFT EVIDENCE |
| f1/corner_rl | typed-refusal | `Pos` (placement) | corner placement `bd.Pos` (wheels.py:963) vs the drop-in placement vocabulary | 0.060 | 21 | 34634 | 23.915 | PB-011B LIFT EVIDENCE |
| f1/corner_rr | typed-refusal | `Pos` (placement) | corner placement `bd.Pos` (wheels.py:963) vs the drop-in placement vocabulary | 0.062 | 21 | 34630 | 23.911 | PB-011B LIFT EVIDENCE |
| f1/details | kernel-door DNF (untyped) | `Vector` (direction math, `.normalized`) | `surfaces.plate_plane` `bd.Vector(...).normalized()` (surfaces.py:696) vs the drop-in `Vector` data row | 0.059 | 54 | 17938 | 12.567 | PB-011C CENSUS (typed-refusal) |
| f1/diffuser | typed-refusal | `Plane` (frame algebra) | `surfaces.half_section_face` `bd.Plane(origin, x_dir, z_dir)` (surfaces.py:613) vs the drop-in `Plane` frame markers | 0.063 | 9 | 6582 | 4.700 | PB-011B LIFT EVIDENCE |
| f1/drivetrain | typed-refusal | `Plane` (frame algebra) | `surfaces.half_section_face` `bd.Plane(...)` (surfaces.py:613) vs the drop-in `Plane` frame markers | 0.062 | 63 | 60141 | 43.212 | PB-011C CENSUS (typed-refusal) |
| f1/drs_actuator | kernel-door DNF (untyped) | `Vector` (`.X` access) | bellcrank outline polar `cp.X` (rear_wing.py:757) vs the drop-in `Vector` data row | 0.060 | 26 | 4968 | 3.997 | PB-011B LIFT EVIDENCE |
| f1/drs_flap | kernel-door DNF (untyped) | `Vector` (scalar multiply) | `_flap_station` `_chord_dir(...) * float` (rear_wing.py:188) vs the drop-in `Vector` data row | 0.057 | 9 | 1902 | 4.548 | PB-011C CENSUS (typed-refusal) |
| f1/engine_cover | typed-refusal | `Spline` (profile authoring) | airfoil profile authoring `bd.Spline` (surfaces.py:392) vs the drop-in profile vocabulary | 0.060 | 44 | 11863 | 57.869 | PB-011C CENSUS (typed-refusal) |
| f1/floor | typed-refusal | `Plane` (frame algebra) | `surfaces.half_section_face` `bd.Plane(...)` (surfaces.py:613) vs the drop-in `Plane` frame markers | 0.056 | 20 | 95024 | 9.949 | PB-011B LIFT EVIDENCE |
| f1/monocoque | typed-refusal | `Plane` (frame algebra) | `surfaces.half_section_face` `bd.Plane(...)` (surfaces.py:613) vs the drop-in `Plane` frame markers | 0.071 | 2 | 6971 | 43.747 | PB-011 LIFT EVIDENCE |
| f1/power_unit | typed-refusal | `Plane` (frame algebra) | `surfaces.half_section_face` `bd.Plane(...)` (surfaces.py:613) vs the drop-in `Plane` frame markers | 0.117 | 100 | 100060 | 39.132 | PB-011C CENSUS (typed-refusal) |
| f1/rear_wing | typed-refusal | `Spline` (profile authoring) | airfoil profile authoring `bd.Spline` (surfaces.py:392) vs the drop-in profile vocabulary | 0.081 | 18 | 14594 | 7.024 | PB-011C CENSUS (typed-refusal) |
| f1/steering_rack | kernel-door DNF (untyped) | `Vector` (difference) | `suspension._beam` `(b - a).normalized()` (suspension.py:188) vs the drop-in `Vector` data row | 0.104 | 11 | 7344 | 6.084 | PB-011B LIFT EVIDENCE |
| f1/suspension_front | kernel-door DNF (untyped) | `Vector` (difference) | `suspension._leg` `b - a` (suspension.py:341) vs the drop-in `Vector` data row | 0.054 | 89 | 72496 | 17.907 | PB-011B LIFT EVIDENCE |
| f1/suspension_rear | kernel-door DNF (untyped) | `Vector` (difference) | `suspension._leg` `b - a` (suspension.py:341) vs the drop-in `Vector` data row | 0.053 | 99 | 75514 | 18.706 | PB-011B LIFT EVIDENCE |
| f1/track_rod_left | kernel-door DNF (untyped) | `Vector` (difference) | `suspension._blade_rod` `(b - a).normalized()` (suspension.py:407) vs the drop-in `Vector` data row | 0.070 | 5 | 1916 | 4.124 | PB-011B LIFT EVIDENCE |
| f1/track_rod_right | kernel-door DNF (untyped) | `Vector` (difference) | `suspension._blade_rod` `(b - a).normalized()` (suspension.py:407) vs the drop-in `Vector` data row | 0.066 | 5 | 1908 | 4.196 | PB-011B LIFT EVIDENCE |

## Delta against the pre-admission census

The pre-admission census (PB-011C result `loop/results/PB-011C-TTC-PARITY-CHURN.json`;
per-row census markers in `corpus/ttc/SKIPS.json`) recorded the F1 rows by their first
**boolean** op class at the certified funnel: 12 canonical-cutter rows carried a
`PB-011B LIFT EVIDENCE` marker (first refusing op adjudicated as a canonical-tool pair
routed through the certified-entry dispatch), 8 swept-x-swept rows carried a
`PB-011C CENSUS` typed-refusal record (fuse/cut(swept,swept) refusing
`NonCanonicalCarrier` at the boolean boundary), and `f1/monocoque` carried a
`PB-011 LIFT EVIDENCE` marker.

The closing kernel-door re-run at this HEAD measures the actual corpus execution, and
it shows a **moved, earlier boundary for every row**: none of the 21 rows reaches its
boolean/composition op under the truck regime, because the corpus's section authoring
(Plane frame algebra via `surfaces.half_section_face`, airfoil `bd.Spline` profile
authoring, `bd.Pos` placement, and the `Vector` direction-math helpers) is still
refused or unanswered by the drop-in's authoring surface. The rows whose first
refusing op the funnel consult placed at the boolean now stop one or two carriers
earlier, in authoring:

- Rows that refuse **typed** at an authoring carrier (12): `beam_wing`,
  `corner_fl/fr/rl/rr`, `diffuser`, `drivetrain`, `engine_cover`, `floor`, `monocoque`,
  `power_unit`, `rear_wing` — refusing verb `Plane` (frame algebra), `Pos`, or
  `Spline` (profile authoring).
- Rows that die **untyped** on the drop-in `Vector` data surface (9): `airbox`,
  `details`, `drs_actuator`, `drs_flap`, `steering_rack`, `suspension_front`,
  `suspension_rear`, `track_rod_left`, `track_rod_right` — authoring-carrier DNFs of
  the same class FH-CENSUS/HYPERCAR-CENSUS recorded (untyped, not tuned).

**Which refusals admission cured: none observable at the kernel door.** No F1 row's
kernel-door run returned geometry facts, so the ADM-004 admission state was never
reached through the corpus door on this dispatch; the certified-funnel boolean
admission is unreachable behind the authoring surface. **Which it did not: all 21.**
The `PB-011B LIFT EVIDENCE` / `PB-011C CENSUS` / `PB-011 LIFT EVIDENCE` markers in
`corpus/ttc/SKIPS.json` are unchanged (they stay staged under
`booleans-on-swept-carriers`); the corpus manifest (`corpus/ttc/MANIFEST.json`) is
untouched. Rows still refusing are the permanent boundary — no row was tuned, no
corpus script was edited, no concurrent door runs were made.

## Typed refusals (verbatim)

Each typed refusal carries the drop-in's standard mapped payload
`{"case": "unsupported_envelope", "envelope": "non_canonical_carrier"}`. No typed
refusal came back with a different case.

### f1/beam_wing — `bd.Spline` airfoil profile authoring (surfaces.py:392)

`beam_wing` builds its airfoil section profiles through `surfaces.airfoil_profile`
(`bd.Spline` profile authoring); the drop-in profile vocabulary refuses typed before
any boolean. Verbatim (`door --engine truck` exit 1, `door_version 2`):

```json
{"kind": "Refused", "message": "spline profile authoring is not a corpus kernel-engine carrier"}
```

### f1/corner_fl / corner_fr / corner_rl / corner_rr — `bd.Pos` placement (wheels.py:963)

Each corner row places its wheel assembly with `bd.Pos(x, y, z)` in `build_corner`;
the drop-in refuses typed at the placement carrier. Verbatim:

```json
{"kind": "Refused", "message": "Pos is recorded client-layer data; no kernel row"}
```

### f1/diffuser / drivetrain / floor / monocoque / power_unit — `bd.Plane` frame algebra (surfaces.py:613)

These rows author their section faces through `surfaces.half_section_face`, which
constructs an explicit frame `bd.Plane(origin=..., x_dir=..., z_dir=...)`; Plane frame
algebra is recorded client-layer data and refuses typed. Verbatim:

```json
{"kind": "Refused", "message": "Plane algebra is recorded client-layer data; no kernel row"}
```

### f1/engine_cover / rear_wing — `bd.Spline` airfoil profile authoring (surfaces.py:392)

Identical to the beam_wing carrier. Verbatim:

```json
{"kind": "Refused", "message": "spline profile authoring is not a corpus kernel-engine carrier"}
```

## Kernel-door DNF rows (untyped, recorded as-is — not tuned)

Nine rows die untyped because the drop-in `Vector` data row does not yet answer the
corpus's direction-math surface (`.normalized`, `.X`, scalar multiply, vector
difference), or the `Vector` constructor rejects the passed form. These are
authoring-carrier data-surface DNFs — recorded verbatim, never tuned (same class the
FH-CENSUS tube rows and HYPERCAR-CENSUS interior/lighting rows recorded before their
authoring-surface lifts).

| row | kind | message (verbatim) | failing corpus site |
|---|---|---|---|
| f1/airbox | `TypeError` | `Vector requires three coordinates` | `surfaces.plate_plane` `bd.Vector(normal).normalized()` (surfaces.py:696), reached from `engine_cover._disc_face` |
| f1/details | `AttributeError` | `'Vector' object has no attribute 'normalized'` | `surfaces.plate_plane` (surfaces.py:696), reached from `details._plate` |
| f1/drs_actuator | `AttributeError` | `'Vector' object has no attribute 'X'` | `rear_wing.polar` `cp.X` (rear_wing.py:757) |
| f1/drs_flap | `TypeError` | `unsupported operand type(s) for *: 'Vector' and 'float'` | `rear_wing._flap_station` (rear_wing.py:188) |
| f1/steering_rack | `TypeError` | `unsupported operand type(s) for -: 'Vector' and 'Vector'` | `suspension._beam` (suspension.py:188) |
| f1/suspension_front | `TypeError` | `unsupported operand type(s) for -: 'Vector' and 'Vector'` | `suspension._leg` (suspension.py:341) |
| f1/suspension_rear | `TypeError` | `unsupported operand type(s) for -: 'Vector' and 'Vector'` | `suspension._leg` (suspension.py:341) |
| f1/track_rod_left | `TypeError` | `unsupported operand type(s) for -: 'Vector' and 'Vector'` | `suspension._blade_rod` (suspension.py:407) |
| f1/track_rod_right | `TypeError` | `unsupported operand type(s) for -: 'Vector' and 'Vector'` | `suspension._blade_rod` (suspension.py:407) |

## Findings filed

1. **No F1 row certifies on the admitted kernel door at this HEAD.** 21/21 rows stop
   in authoring; 0 rows return kernel facts. ADM-004's certified-funnel admission is a
   boolean-layer change and is unreachable through the corpus door for every F1 row:
   each row's authoring surface (`Plane` frame algebra, `Pos` placement, `Spline`
   profile authoring, `Vector` direction math) still refuses or fails before any
   boolean/composition op runs.
2. **The refusing boundary moved one or two carriers earlier than the pre-admission
   census recorded.** PB-011B/C recorded each F1 row's first refusing op at the boolean
   layer (canonical-tool routed / swept-x-swept typed refusal). Re-running the actual
   corpus scripts through the truck regime shows the rows refuse in AUTHORING — the
   section-frame/profile/placement/`Vector`-data surface is the standing blocker. The
   SKIPS census markers stay accurate as *funnel-state* records but do not describe
   where a corpus execution stops; the rows' booleans are never reached.
3. **The 9 untyped rows are authoring data-surface gaps, not kernel refusals.** The
   drop-in `Vector` data row answers no direction math (`.normalized`, `.X`, `*`, `-`)
   and the constructor accepts only tuple/list/3-scalar forms, so corpus helpers
   (`plate_plane`, `polar`, `_flap_station`, `suspension` rod/leg math) die untyped.
   As with the FH tube rows and the hypercar interior/lighting rows, these are booked
   for the authoring-surface program, not recorded as typed refusals.
4. **OCC-baseline references for all 21 rows reproduce bit-identically (21/21), and no
   OCC flake occurred.** The green-side baseline is trustworthy; the stop conditions on
   reference drift / OCC flake did not fire.

## Timing

No row flipped green, so no kernel timing column is appended to
`docs/TT_TIMING_RESULTS.md` (no timing is published against a red facts gate). The
existing monocoque DNF-KERNEL record in that doc is confirmed unchanged: the tub still
refuses typed at `Plane` frame algebra on the kernel door.

## Anchors

- A1 `grep -c '"id"' corpus/ttc/MANIFEST.json` = 48 (expect 48) — holds
  (manifest untouched).
- A2 `grep -c 'median' docs/TT_TIMING_RESULTS.md` = 21 (expect 21) — holds (no kernel
  timing column added; the doc's standing timing content is unchanged).
