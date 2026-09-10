# TTC-CENSUS-FINAL — F1 post-chain re-census (TTC-RECENSUS-F1-R2)

Post-door-gap-chain re-census of the F1 family. With the full chain landed
(AUTHOR-FRAME-CARRIERS, FRAME-REVOLVE, SWEEP-PATH, BRIDGE-BOOLEANS,
BRIDGE-LOFT-FACTS, TRIM-EXTRUDE-CTOR), all 21 F1 manifest rows
(`corpus/ttc/MANIFEST.json`, family `f1`) were re-run through the kernel door
(`corpus/ttc/door.py --engine truck`, `door_version 2`): one fresh Python
process per row, serial, quiet machine. Facts gates compare the kernel door's
facts against the RECORDED references (`corpus/ttc/reference/*.json`) —
`solid_count` exact, `volume` within the recorded `volume_rel` 1e-4 /
`volume_abs` 1e-6 band, `bbox` within `bbox_abs` 1e-3. **No OCC process ran
anywhere in this packet** (owner directive); the recorded references are the
sole oracle. A row is GREEN only when the door returns `ok: true` and every
fact matches the recorded reference. Rows that refuse typed keep the typed
verdict and the refusing carrier; rows that die untyped are kernel-door DNF
with the recorded reason. No reference or manifest was edited, no tolerance
was stretched, and no concurrent door runs were made.

## Result in one line

**No F1 row certifies on the post-chain kernel door.** 0 green / 0
facts-gated flips, 15 typed-refusal (with the refusing carrier named), 6
kernel-door DNF (untyped, recorded reason). The door-gap chain **cured the
Python authoring surface for every row** — no row refuses at `Plane` frame
algebra, `Pos` placement, `Spline` profile authoring or the `Vector`
data-row arithmetic any more — and the boundary moved **into the native
executor's carrier envelope**: multi-station spline-section lofts (the
BRIDGE-LOFT-FACTS honesty line certifies only a two-station smooth loft), the
TRIM-EXTRUDE-CTOR constructor, the BRIDGE-BOOLEANS admission, plus two drop-in
data-surface gaps (`Shape.wrapped` OCC probes) and the corpus floor-loft
builder. Every recorded reference stayed the sole oracle; no reference drifted.

## Machine / environment

- Windows x86_64 (win32), measured 2026-09-10 in a quiet window of the
  autobuild loop (no concurrent `cargo`/`rustc`/test process during the runs).
- Python 3.14.3; the kernel regime's native executor is the **release-built**
  `truck123d` module at this worktree HEAD (`cargo build --release --locked -p
  truck123d` green; the release `truck123d.dll` staged as `truck123d.pyd`
  beside the interpreter runtime, importable from a real 3.14 interpreter).
- Worktree HEAD `de33f33`, branch `packet/TTC-RECENSUS-F1-R2`.
- Kernel door: `door_version 2` (`--engine truck`). Recorded references:
  `door_version 1` (OCC baseline, recorded earlier — not re-run here).
- No OCC process ran; `corpus/ttc/reference/*.json` and
  `corpus/ttc/MANIFEST.json` are read-only and unchanged.

## Method

Per row, exactly the packet protocol: one fresh Python process,

```console
python corpus/ttc/door.py --engine truck corpus/ttc/trees/f1/src <module> <entry> <args> <stl>
```

serial, quiet. The facts gate is adjudicated before any timing is considered.
The recorded `ttc_reference.v1` schema carries `solid_count`, `volume` and
`bbox` (the recorded references carry no STL-triangle oracle), so the facts
gate is `solid_count` exact / `volume` in band / `bbox` in band; a green row's
STL triangle count would be recorded as door evidence (no row is green, so no
F1 triangle column exists). A row that raises is classified typed when the
door returns `kind: Refused` (with the carrier named) and DNF when the failure
is an untyped Python exception.

## Census verdict table — all 21 F1 rows

Columns: post-chain verdict; first refusing carrier (post-chain); the chain
element that moved the boundary past the R1 carrier; prior R1 verdict; kernel
door wall (s). Prior verdicts are the R1 record
(`loop/results/TTC-RECENSUS-F1.json`, `docs/TTC_CENSUS_FINAL.md` at R1).

| row | post-chain verdict | refusing carrier (post-chain) | cured by | prior R1 verdict | kernel wall (s) |
|---|---|---|---|---|---|
| f1/airbox | typed-refusal | native boolean admission: `_tcam` subtract `engine_cover.py:1029` (`kernel refusal: unsupported_envelope`) | frame carriers + `Vector` data row | kernel-door DNF (`Vector` ctor) | 0.149 |
| f1/beam_wing | typed-refusal | multi-station spline loft (15 stations, spline airfoil sections) — BRIDGE-LOFT-FACTS honesty line | `Spline` profile authoring | typed-refusal (`Spline`) | 0.172 |
| f1/corner_fl | kernel-door DNF (untyped) | `surfaces.bbox` `shape.wrapped is None` → OCP `BRepBndLib.Add_s(None, ...)` `TypeError` (`surfaces.py:725`, via `wheels._loft`) | `Pos` placement | typed-refusal (`Pos`) | 1.508 |
| f1/corner_fr | kernel-door DNF (untyped) | same `surfaces.bbox` `Face.wrapped is None` → OCP `TypeError` | `Pos` placement | typed-refusal (`Pos`) | 1.233 |
| f1/corner_rl | kernel-door DNF (untyped) | same `surfaces.bbox` `Face.wrapped is None` → OCP `TypeError` | `Pos` placement | typed-refusal (`Pos`) | 1.282 |
| f1/corner_rr | kernel-door DNF (untyped) | same `surfaces.bbox` `Face.wrapped is None` → OCP `TypeError` | `Pos` placement | typed-refusal (`Pos`) | 1.283 |
| f1/details | typed-refusal | native boolean admission: `surfaces.cut` `details.py:359` (`kernel refusal: unsupported_envelope`) | frame carriers + `Vector` data row | kernel-door DNF (`Vector` `.normalized`) | 0.147 |
| f1/diffuser | kernel-door DNF (untyped) | corpus floor-loft builder `RuntimeError: floor loft failed: None` (`floor.py:530`, via `_diffuser_shell`) | `Plane` frame algebra | typed-refusal (`Plane`) | 1.375 |
| f1/drivetrain | typed-refusal | native facts refusal at `_fuse` `drivetrain.py:373` (`kernel refusal: unsupported_envelope`) | `Plane` frame algebra | typed-refusal (`Plane`) | 0.167 |
| f1/drs_actuator | typed-refusal | mirror carrier: `surfaces.mirror_y` `surfaces.py:72` (`a mirror of this carrier is not a kernel-engine row`) | frame carriers + `Vector` data row | kernel-door DNF (`Vector` `.X`) | 0.113 |
| f1/drs_flap | typed-refusal | multi-station spline loft (19 stations, spline airfoil sections) — BRIDGE-LOFT-FACTS honesty line | frame carriers + `Vector` data row | kernel-door DNF (`Vector` scalar mul) | 0.152 |
| f1/engine_cover | typed-refusal | OCC-probe data gap: `surfaces.bbox` via `_bounded` `engine_cover.py:163` (`an OCC probe of a kernel-engine row is not a kernel-engine row`) | `Spline` profile authoring | typed-refusal (`Spline`) | 1.373 |
| f1/floor | kernel-door DNF (untyped) | corpus floor-loft builder `RuntimeError: floor loft failed: None` (`floor.py:530`, via `_floor_shell`) | `Plane` frame algebra | typed-refusal (`Plane`) | 1.231 |
| f1/monocoque | typed-refusal | OCC-probe data gap: `surfaces.obox` via `_tub_solid` `mono_tub.py:526` (`an OCC probe of a kernel-engine row is not a kernel-engine row`) | `Plane` frame algebra | typed-refusal (`Plane`) | 1.326 |
| f1/power_unit | typed-refusal | multi-station spline loft (16 stations, spline airfoil sections) — BRIDGE-LOFT-FACTS honesty line | `Plane` frame algebra | typed-refusal (`Plane`) | 0.908 |
| f1/rear_wing | typed-refusal | trim-extrude constructor: `_louvre_cutters` `rear_wing.py:390` (`kernel refusal: unsupported_envelope`) | `Spline` profile authoring | typed-refusal (`Spline`) | 0.136 |
| f1/steering_rack | typed-refusal | trim-extrude constructor: `_plate` `suspension.py:162` (`kernel refusal: unsupported_envelope`) | frame carriers + `Vector` data row | kernel-door DNF (`Vector` difference) | 0.155 |
| f1/suspension_front | typed-refusal | trim-extrude constructor: `_plate` `suspension.py:162` (`kernel refusal: unsupported_envelope`) | frame carriers + `Vector` data row | kernel-door DNF (`Vector` difference) | 0.181 |
| f1/suspension_rear | typed-refusal | trim-extrude constructor: `_plate` `suspension.py:162` (`kernel refusal: unsupported_envelope`) | frame carriers + `Vector` data row | kernel-door DNF (`Vector` difference) | 0.198 |
| f1/track_rod_left | typed-refusal | multi-station spline loft (7 stations, spline airfoil sections) — BRIDGE-LOFT-FACTS honesty line | frame carriers + `Vector` data row | kernel-door DNF (`Vector` difference) | 0.126 |
| f1/track_rod_right | typed-refusal | multi-station spline loft (7 stations, spline airfoil sections) — BRIDGE-LOFT-FACTS honesty line | frame carriers + `Vector` data row | kernel-door DNF (`Vector` difference) | 0.127 |

Verdict counts: **green 0, typed-refusal 15, kernel-door DNF (untyped) 6**.

## Delta against the R1 census

The R1 census (`loop/results/TTC-RECENSUS-F1.json`, at HEAD `07b2090`,
post-ADM-004 / post-F1-AUTHORING-ARMS) recorded **0 green, 12 typed-refusal, 9
kernel-door DNF**, with every row stopping in the Python authoring surface: 12
rows refused typed at `Plane` frame algebra (`surfaces.half_section_face`),
`Pos` placement (`wheels.build_corner`) or `Spline` profile authoring
(`surfaces.airfoil_profile`), and 9 died untyped on the drop-in `Vector`
data-row arithmetic (`.normalized`, `.X`, `*`, `-`).

The post-chain re-run at HEAD `de33f33` shows the chain **cured all of those
authoring carriers**: every one of the 21 rows now authors past `Plane`,
`Pos`, `Spline` and the `Vector` arithmetic (AUTHOR-FRAME-CARRIERS /
FRAME-REVOLVE / SWEEP-PATH / the drop-in `Vector` data row), and the boundary
moved one or more carriers deeper — into the native executor's carrier
envelope and two remaining drop-in data-surface gaps. Concretely:

- **Spline-loft rows (beam_wing, drs_flap, power_unit, track_rod_left,
  track_rod_right) moved from the R1 `Spline`/`Vector` boundary to
  BRIDGE-LOFT-FACTS.** The landed smooth-loft arm certifies only a
  **two-station** smooth loft (its honesty line: a three-or-more-station
  smooth loft's station parameterization is an OCC `ThruSections` convention
  the row does not record, so the arm refuses typed rather than substituting a
  ruled approximation). The corpus's F1 lofts are **7–19 stations** of spline
  airfoil sections (2 splines + 1 line per section, 25–49 samples per spline),
  so they refuse typed. This is the dominant still-open carrier.
- **Trim rows (rear_wing, steering_rack, suspension_front, suspension_rear)
  moved to the TRIM-EXTRUDE-CTOR constructor.** The constructor refuses
  `UnsupportedEnvelope` on the corpus's actual trim idioms (the louvre cutters
  and the suspension `_plate` profiles), so the trim rows do not close.
- **Boolean rows (airbox, details) moved to the BRIDGE-BOOLEANS admission.**
  The native boolean facts entry refuses the corpus's real pairs
  (`UnsupportedEnvelope` / `NonCanonicalCarrier`), so the boolean rows do not
  flip.
- **drivetrain** moved to a native facts refusal at its `_fuse`
  (`drivetrain.py:373`).
- **Two drop-in OCC-probe data gaps (engine_cover, monocoque)** now stop the
  rows: the corpus `surfaces.bbox` / `surfaces.obox` helpers probe
  `shape.wrapped`, and the drop-in's kernel-row `wrapped` refuses typed
  (`an OCC probe of a kernel-engine row is not a kernel-engine row`). The rows
  author past `Spline`/`Plane` and die at the probe.
- **Four corner rows** author past `Pos` and die untyped: `surfaces.bbox`
  calls `shape.wrapped` on a drop-in **`Face`**, whose `wrapped` property
  returns `None`, so OCP `BRepBndLib.Add_s(None, ...)` raises `TypeError`.
- **floor and diffuser** author past `Plane` and die untyped in the corpus
  floor-loft builder (`surfaces.body_loft` + `surfaces.is_valid_shape`, whose
  `is_valid_shape` swallows the drop-in refusal and returns `False`), so all
  trial lofts fail and the builder raises `RuntimeError: floor loft failed:
  None`.

**Which refusals the chain cured: the entire Python authoring surface** — no
row refuses at `Plane`, `Pos`, `Spline` or the `Vector` data row any more.
**Which it did not: the native carrier envelope** — the multi-station smooth
loft, the trim-extrude constructor and the boolean admission still refuse the
corpus's real carriers, and the OCC-probe/mirror/floor-loft gaps remain. **No
row flipped to green**, so no facts-gated flip and no kernel timing column was
produced; the SKIPS.json markers and `MANIFEST.json` are unchanged, no corpus
script was edited, and no tolerance was stretched.

## Typed refusals (verbatim)

The native executor's marshaled refusal for the multi-station loft, trim and
boolean carriers is the generic mapped message (the evidence is the mapped
payload):

```json
{"kind": "Refused", "message": "kernel refusal: unsupported_envelope",
 "payload": {"case": "unsupported_envelope", "envelope": "non_canonical_carrier"}}
```

Rows and their carriers:

- **multi-station spline loft (BRIDGE-LOFT-FACTS honesty line)** —
  `beam_wing` (15 stations), `drs_flap` (19), `power_unit` (16),
  `track_rod_left` (7), `track_rod_right` (7). The failing node is a
  `part:loft(sections=N, closed=false)` whose sections are spline-outlined
  airfoils.
- **native boolean admission** — `airbox` (`_tcam` subtract,
  `engine_cover.py:1029`), `details` (`surfaces.cut`, `details.py:359`).
- **native facts refusal at `_fuse`** — `drivetrain`
  (`drivetrain.py:373`).
- **trim-extrude constructor (TRIM-EXTRUDE-CTOR)** — `rear_wing`
  (`_louvre_cutters`, `rear_wing.py:390`), `steering_rack` (`_plate`,
  `suspension.py:162`), `suspension_front` (`_plate`), `suspension_rear`
  (`_plate`).
- **mirror carrier** — `drs_actuator` (`surfaces.mirror_y`,
  `surfaces.py:72`):

```json
{"kind": "Refused", "message": "a mirror of this carrier is not a kernel-engine row",
 "payload": {"case": "unsupported_envelope", "envelope": "non_canonical_carrier"}}
```

- **OCC-probe data gap** — `engine_cover` (`surfaces.bbox` via `_bounded`,
  `engine_cover.py:163`), `monocoque` (`surfaces.obox` via `_tub_solid`,
  `mono_tub.py:526`):

```json
{"kind": "Refused", "message": "an OCC probe of a kernel-engine row is not a kernel-engine row",
 "payload": {"case": "unsupported_envelope", "envelope": "non_canonical_carrier"}}
```

## Kernel-door DNF rows (untyped, recorded as-is — not tuned)

| row | kind | message (verbatim) | failing corpus site |
|---|---|---|---|
| f1/corner_fl | `TypeError` | `Add_s(): incompatible function arguments ... Invoked with: None, <OCP.OCP.Bnd.Bnd_Box object ...>, False` | `surfaces.bbox` `shape.wrapped` on a drop-in `Face` (`surfaces.py:725`), reached from `wheels._loft` |
| f1/corner_fr | `TypeError` | same | same |
| f1/corner_rl | `TypeError` | same | same |
| f1/corner_rr | `TypeError` | same | same |
| f1/diffuser | `RuntimeError` | `floor loft failed: None` | `floor._loft_stack` (`floor.py:530`), via `_diffuser_shell` |
| f1/floor | `RuntimeError` | `floor loft failed: None` | `floor._loft_stack` (`floor.py:530`), via `_floor_shell` |

The corner rows are a drop-in data-surface gap: the kernel-row `_Shape.wrapped`
refuses typed, but the profile `Face` data row's `wrapped` returns `None`, so
the corpus's `surfaces.bbox` passes `None` into OCP. The floor rows are a
corpus-builder gap: `surfaces.is_valid_shape` catches the drop-in refusal and
returns `False`, so every trial loft is rejected and the builder raises an
untyped `RuntimeError` whose embedded cause is `None`. Both classes are
recorded verbatim, never tuned.

## V5 net — previously-green rows

The V5 net holds. The three previously-green FH timing rows were re-run through
the kernel door on the release build at this HEAD and answer **bit-identically**
to their recorded `FH-TIMING-REFRESH` facts (same `solid_count`, `volume`,
`bbox`, STL triangles):

| row | solid_count | volume | STL triangles | verdict |
|---|---:|---:|---:|---|
| falcon_heavy/turbopump_assembly | 59 | 77563936.47352579 | 35352 | green (unchanged) |
| falcon_heavy/chamber_assembly | 46 | 45458782.5489467 | 27520 | green (unchanged) |
| falcon_heavy/fairing | 4 | 11027439648.490429 | 11032 | green (unchanged) |

No landed F1 class exists (no F1 row has ever been green), so the V5 net for
this packet is the three FH rows; no verdict flip occurred on a landed row.

## Timing

No row flipped green, so no kernel timing column is appended to
`docs/TT_TIMING_RESULTS.md` (no timing is published against a red facts gate).
The F1 kernel timing column stays closed row by row; the `TTC-RECENSUS-F1-R2`
section of the timing doc records the zero-green outcome and the V5 net. The
standing FH timing content is unchanged (anchors hold; see below).

## Findings filed

1. **No F1 row certifies on the post-chain kernel door.** 0 green of 21; 15
   typed-refusal and 6 kernel-door DNF. The door-gap chain cured the Python
   authoring surface for every row and moved the boundary into the native
   executor's carrier envelope.
2. **The dominant still-open carrier is the multi-station smooth loft.** The
   BRIDGE-LOFT-FACTS arm's honesty line certifies only a two-station smooth
   loft; the corpus's F1 body lofts are 7–19 station spline-section lofts, so
   they refuse typed. Closing the spline-loft class needs either the recorded
   station parameterization (so a 3+-station smooth loft is determined) or a
   certified multi-station smooth-volume arm.
3. **The trim rows do not close.** TRIM-EXTRUDE-CTOR refuses the corpus's real
   trim idioms (the rear-wing louvre cutters and the suspension `_plate`
   profiles) with `UnsupportedEnvelope`, so the trim rows stay typed.
4. **The boolean rows do not flip.** BRIDGE-BOOLEANS refuses the corpus's real
   boolean pairs (airbox `_tcam`, details `mirror_optics`) at the native facts
   entry; the admission does not admit them.
5. **Two drop-in data-surface gaps remain:** the kernel-row `Shape.wrapped`
   OCC probe (engine_cover, monocoque) refuses typed, and the profile `Face`
   `wrapped` returns `None` (corner rows) so `surfaces.bbox` dies untyped in
   OCP. These are drop-in/authoring-surface gaps, not native kernel refusals.
6. **The corpus floor-loft builder is an untyped DNF** (floor, diffuser):
   `surfaces.is_valid_shape` swallows the drop-in refusal and the builder
   raises `RuntimeError: floor loft failed: None`.
7. **The V5 net holds** (three FH green rows bit-identical); no reference
   drifted and no OCC flake is possible (no OCC process ran).

## Anchors

- A1 `grep -c '"id"' corpus/ttc/MANIFEST.json` = 48 (expect 48) — holds
  (manifest untouched).
- A2 `grep -c median docs/TT_TIMING_RESULTS.md` = 21 (expect 21) — holds (no
  F1 kernel timing column was added; the doc's standing timing content is
  unchanged).
