# HYPERCAR-CENSUS — first kernel-door contact for every hypercar row

Mechanical census (the PB-011C protocol applied to the hypercar family): every
manifest row in the hypercar family that had never seen the kernel door
(`aero`, `body`, `brakes`, `chassis`, `glazing`, `hinge`, `interior`,
`lighting`, `powertrain`, `wheels` — the ten rows with a recorded reference —
plus `details`, `suspension_front`, `suspension_rear` — the three rows with no
recorded reference) was run through `corpus/ttc/door.py --engine truck` (the
kernel-engine door regime, `door_version 2`) once, one fresh Python process per
row, serial, quiet machine — the documented under-load flake regime, never
concurrent. Typed refusals are valid outcomes; this is a measurement, not a
lift packet. No benchmark timing was performed (benchmark-grade wall columns
are FH-TIMING-REFRESH's, against green rows only); the `wall_s` column below is
the OCC-baseline door-process wall of the single census evidence run per row
(the kernel census run itself terminates every row in well under two seconds —
see the verdict records).

## Census scope and staging

The census scope is every row whose manifest `family` is `hypercar`
(`corpus/ttc/MANIFEST.json`): 13 rows. Ten of them carry a recorded OCC
reference (`corpus/ttc/reference/<row>.json`, row ids `hypercar/*`) and were
run against the kernel door. Three rows — `hypercar/details`,
`hypercar/suspension_front`, `hypercar/suspension_rear` — have **no recorded
reference**: the reference file their short name would use is occupied by the
same-named F1 row (`reference/details.json`, `reference/suspension_front.json`
and `reference/suspension_rear.json` all carry an `f1/*` `row_id`), so the
per-row UNSTAGED stop condition fired and they were not run. `hypercar/wheels`
is the single manifest-canonical hypercar row (its reference is
`reference/wheels.json`); the other nine staged rows are manifest-skipped under
the `booleans-on-swept-carriers` reason in `corpus/ttc/SKIPS.json` but carry
recorded OCC references, so they stage for the census exactly as the F1 rows
PB-011C ran did.

## Machine / environment

- Windows x86_64 (win32), census run 2026-09-08 on the autobuild host.
- Python 3.14.3; `build123d` 0.11.1; `cadquery-ocp` 7.9.3.1.1 (OCC baseline).
- The truck regime's native executor is the release-built `truck123d` module at
  this worktree HEAD, staged as `truck123d.pyd` beside the interpreter runtimes
  (the same staging the FH-CENSUS / TTC-TIMING runs used).
- Worktree HEAD `9117732`, branch `packet/HYPERCAR-CENSUS`.
- Reference rows: present for ten hypercar rows (no UNSTAGED stop condition on
  those); absent for the three rows recorded as UNSTAGED above.
- OCC-baseline door runs (`--engine occ`, `door_version 1`), one fresh process
  per row, serial: 10/10 green, and every run reproduced its recorded
  `corpus/ttc/reference/<row>.json` facts **bit-identically** (solid_count
  exact; volume and bbox equal to the recorded doubles) — no OCC-DNF stop
  condition fired. The `solid_count` and `STL tris` table columns below are
  that OCC-baseline door evidence (the PB-011C per-row door-evidence columns);
  no kernel geometry facts exist for any row.

## Census verdict table

| row | first refusing verb | carrier classes in collision | verdict | solid_count | STL tris | wall_s |
|---|---|---|---|---:|---:|---:|
| hypercar/aero | `Spline` | spline band/profile authoring (`_xz_band` XZ band, `bd.Spline`/`bd.Line`, aero.py:128) vs the drop-in's profile vocabulary (a spline profile is not a corpus kernel-engine carrier) | typed-refusal | 24 | 4754 | 10.86 |
| hypercar/body | `Spline` | master-shell section-band authoring (`surfaces._mirrored_face` half-section band with end tangents, surfaces.py:378) feeding the 26-station `body_master` loft vs the drop-in's profile vocabulary | typed-refusal | 28 | 5590 | 38.62 |
| hypercar/brakes | `Location` | corner placement `bd.Location((x, y, z), (-90\|90, 0, 0))` (brakes.py:701) — a placement beyond the translation/pure-z-rotation census carrier | typed-refusal | 440 | 155264 | 7.25 |
| hypercar/chassis | `Spline` | tub section-band authoring (`chassis._face` closed section with end tangents, chassis.py:142) feeding the 24-station monocoque-tub loft vs the drop-in's profile vocabulary | typed-refusal | 129 | 45314 | 19.90 |
| hypercar/glazing | `Spline` | canopy-section band authoring (`surfaces._mirrored_face`, surfaces.py:376) feeding the `canopy_master` loft vs the drop-in's profile vocabulary | typed-refusal | 10 | 1229 | 16.06 |
| hypercar/hinge | `Plane` | explicit tower-frame `bd.Plane(origin, x_dir, z_dir)` (hinge.py:605) vs the drop-in's frame algebra (no kernel row) | typed-refusal | 26 | 6316 | 7.53 |
| hypercar/interior | `Plane` (frame constant) | cabin-floor extrude authoring `bd.Plane.XY.offset(...)` (interior.py:820) — the drop-in's `Plane` answers only a call-refusal, so the `.XY` frame constant raises untyped | kernel-door DNF (untyped) | 78 | 68525 | 16.80 |
| hypercar/lighting | `Vector` (direction math) | crown-crease crest-frame authoring `bd.Vector(...).normalized()` (lighting.py:191) — the drop-in's `Vector` data row carries no `normalized` | kernel-door DNF (untyped) | 92 | 189154 | 19.36 |
| hypercar/powertrain | `Plane` | engine-block section frame `bd.Plane(origin, x_dir, z_dir)` (powertrain.py:115) vs the drop-in's frame algebra (no kernel row) | typed-refusal | 114 | 325242 | 10.69 |
| hypercar/wheels | `Location` | corner placement `bd.Location((x, y, z), (±(90−camber), 0, 0))` (wheels.py:716) — a placement beyond the translation/pure-z-rotation census carrier | typed-refusal | 76 | 37952 | 23.81 |
| hypercar/details | — | — | UNSTAGED (no recorded reference) | — | — | — |
| hypercar/suspension_front | — | — | UNSTAGED (no recorded reference) | — | — | — |
| hypercar/suspension_rear | — | — | UNSTAGED (no recorded reference) | — | — | — |

Ten rows run, ten verdicts, three UNSTAGED rows recorded — no gaps over the 13
manifest rows. No row returned kernel geometry facts: every run refused or
failed in authoring before any fact existed, so no kernel facts-vs-reference
comparison ran and no DNF-FACTS delta was recorded. No row stagnated (no solver
budget numbers attach): every run terminated on a typed refusal or an untyped
door failure in authoring. No row carries a lift marker.

Kernel census-run walls (the fresh `--engine truck` door-process wall of each
single census run; FH-CENSUS's `wall_s` convention): aero 0.119, body 0.082,
brakes 0.091, chassis 1.75, glazing 0.090, hinge 0.088, interior 0.10,
lighting 0.092, powertrain 0.086, wheels 0.085 (seconds). The table's `wall_s`
column above is the OCC-baseline door-evidence run wall (PB-011C's convention),
because only the OCC runs produced solids/STL to time.

## Typed refusals (verbatim)

Each typed refusal carries the drop-in's standard mapped payload
`{"case": "unsupported_envelope", "envelope": "non_canonical_carrier"}` (the
door's stdout record carries `kind` + `message`; the `payload` case/envelope is
captured from the raised exception, as the FH census did). No typed refusal
came back with a different case.

### hypercar/aero — census verb `Spline`

`build()` (aero.py:597) opens with `_splitter()`; the front-splitter blade is a
lofted band whose XZ profiles are authored as `bd.Spline(*lo)` (aero.py:128,
`_xz_band`). The first spline profile refuses typed. Refusal recorded verbatim
(`door --engine truck` exit 1, `door_version 2`):

```json
{
  "kind": "Refused",
  "message": "spline profile authoring is not a corpus kernel-engine carrier",
  "payload": {"case": "unsupported_envelope", "envelope": "non_canonical_carrier"}
}
```

### hypercar/body — census verb `Spline`

`build()` (body.py:132) opens with `skin = S.body_skin()`; the master shell is
the 26-station `bd.loft` of spline half-section faces (surfaces.py:415-416),
and the first section band is authored as `bd.Spline(*pts, tangents=...)`
(surfaces.py:378). The first spline profile refuses typed. Refusal recorded
verbatim (`door --engine truck` exit 1, `door_version 2`):

```json
{
  "kind": "Refused",
  "message": "spline profile authoring is not a corpus kernel-engine carrier",
  "payload": {"case": "unsupported_envelope", "envelope": "non_canonical_carrier"}
}
```

### hypercar/brakes — census verb `Location`

`build()` (brakes.py:707) opens by iterating `_corner_locations()`, whose base
is `bd.Location((x, y, z), (-90 if side > 0 else 90, 0, 0))` (brakes.py:701) —
a ±90° x-rotation placement. The drop-in's `Location` answers translation and
pure-z rotation only; this placement refuses typed before any rotor/caliper
prototype is built. Refusal recorded verbatim (`door --engine truck` exit 1,
`door_version 2`):

```json
{
  "kind": "Refused",
  "message": "Location forms beyond a translation or pure-z rotation are not census carriers",
  "payload": {"case": "unsupported_envelope", "envelope": "non_canonical_carrier"}
}
```

### hypercar/chassis — census verb `Spline`

`build()` (chassis.py:891) opens with `_tub_solids()` → `_monocoque()`, the
24-station monocoque-tub loft (chassis.py:389); each tub section is a closed
spline face authored with `bd.Spline(*pts, tangents=...)` (chassis.py:142). The
first spline profile refuses typed. Refusal recorded verbatim (`door --engine
truck` exit 1, `door_version 2`):

```json
{
  "kind": "Refused",
  "message": "spline profile authoring is not a corpus kernel-engine carrier",
  "payload": {"case": "unsupported_envelope", "envelope": "non_canonical_carrier"}
}
```

### hypercar/glazing — census verb `Spline`

`build()` (glazing.py:27) opens with `shell = S.canopy_glass_shell()`; the
canopy shell loft's section band is authored as `bd.Spline(*pts,
tangents=...)` (surfaces.py:376). The first spline profile refuses typed before
the shell's `(canopy_master − canopy_master_inner) − body_master` difference
runs. Refusal recorded verbatim (`door --engine truck` exit 1,
`door_version 2`):

```json
{
  "kind": "Refused",
  "message": "spline profile authoring is not a corpus kernel-engine carrier",
  "payload": {"case": "unsupported_envelope", "envelope": "non_canonical_carrier"}
}
```

### hypercar/hinge — census verb `Plane`

`build()` (hinge.py:674) opens with `_left_leaves()`, whose tower frame is
`bd.Plane(origin=TOWER_BASE, x_dir=_N, z_dir=_U)` (hinge.py:605). An explicit
Plane frame is recorded client-layer data with no kernel row, so it refuses
typed before any mechanism leaf is placed. Refusal recorded verbatim (`door
--engine truck` exit 1, `door_version 2`):

```json
{
  "kind": "Refused",
  "message": "Plane algebra is recorded client-layer data; no kernel row",
  "payload": {"case": "unsupported_envelope", "envelope": "non_canonical_carrier"}
}
```

### hypercar/powertrain — census verb `Plane`

`build()` (powertrain.py:636) opens with the engine block `_xz_loft`, whose
section frames are `bd.Plane(origin=(xc, y, zc), x_dir=(1, 0, 0), z_dir=(0, -1,
0))` (powertrain.py:115). The explicit Plane frame refuses typed before the
block's lofts and fuses. Refusal recorded verbatim (`door --engine truck` exit
1, `door_version 2`):

```json
{
  "kind": "Refused",
  "message": "Plane algebra is recorded client-layer data; no kernel row",
  "payload": {"case": "unsupported_envelope", "envelope": "non_canonical_carrier"}
}
```

### hypercar/wheels — census verb `Location`

`build()` (wheels.py:756) opens by iterating `_corners()`, whose base is
`bd.Location((x, y, z), (±(90 − camber), 0, 0))` (wheels.py:716) — an x-rotation
corner placement. The wheel's constructive tyre/rim revolves and spoke lofts
never start; the placement refuses typed. Refusal recorded verbatim (`door
--engine truck` exit 1, `door_version 2`):

```json
{
  "kind": "Refused",
  "message": "Location forms beyond a translation or pure-z rotation are not census carriers",
  "payload": {"case": "unsupported_envelope", "envelope": "non_canonical_carrier"}
}
```

## Kernel-door DNF rows (untyped, recorded as-is — not tuned)

### hypercar/interior — census verb `Plane` (frame constant), untyped

`build()` (interior.py:983) opens with `_floor()`; the cabin-floor plate is
`bd.extrude(bd.Plane.XY.offset(FLOOR_BOT) * _floor_outline(), ...)`
(interior.py:820). The drop-in exposes `Plane` only as a call-refusing function
— it does not carry the `Plane.XY`/`Plane.XZ` frame-constant attributes — so the
argument evaluation raises an untyped `AttributeError` before the extrusion
verb is reached. Recorded verbatim (`door --engine truck` exit 1,
`door_version 2`):

```json
{
  "kind": "AttributeError",
  "message": "'function' object has no attribute 'XY'"
}
```

### hypercar/lighting — census verb `Vector` (direction math), untyped

`build()` (lighting.py:843) opens with `_front(side)` → `_crown_crease` → the
crest-frame authoring `t = bd.Vector(2.0 * d, yb - ya, zb - za).normalized()`
(lighting.py:191). The drop-in's `Vector` is a bare data row with no
`normalized` (or `dot`/`cross`) methods, so the direction math raises an
untyped `AttributeError`. Recorded verbatim (`door --engine truck` exit 1,
`door_version 2`):

```json
{
  "kind": "AttributeError",
  "message": "'Vector' object has no attribute 'normalized'"
}
```

## UNSTAGED rows (stop condition, not run)

| row | reason |
|---|---|
| hypercar/details | no recorded reference: `reference/details.json` carries `f1/details` |
| hypercar/suspension_front | no recorded reference: `reference/suspension_front.json` carries `f1/suspension_front` |
| hypercar/suspension_rear | no recorded reference: `reference/suspension_rear.json` carries `f1/suspension_rear` |

## Coverage map

| class | rows | count |
|---|---|---|
| authoring-blocked | aero, body, brakes, chassis, glazing, hinge, interior, lighting, powertrain, wheels | 10 |
| boolean-blocked | — | 0 |
| green | — | 0 |
| UNSTAGED (no recorded reference) | details, suspension_front, suspension_rear | 3 |

Every one of the ten staged hypercar rows stops in **authoring** under the
kernel door before any boolean/composition operation executes: the refusing
frames are the splitter/master-shell/canopy/tub **spline section bands**
(`Spline`), the door/block **explicit Plane frames** (`Plane`), the corner
**placements** (`Location`), and — untyped — the **`Plane.XY` frame constant**
and **`Vector.normalized` direction math**. The rows' boolean composition
regions (the `booleans-on-swept-carriers` op graphs SKIPS records for all nine
skipped rows) are never reached, so the hypercar family's boolean-blocked set
is empty on this dispatch and no row can be green until the authoring surface
is lifted. `hypercar/wheels` — the one manifest-canonical hypercar row — is
also authoring-blocked, at its corner `Location` placement.

## Findings filed

1. **The hypercar family is authoring-blocked at a richer surface than the
   FH/F1 families.** FH-CENSUS rows stopped at the census verbs
   `revolve`/`extrude`/`sweep`; the hypercar rows stop one or two carriers
   earlier, on profile/frame/placement authoring the truck regime answers typed
   (`bd.Spline` profile bands, explicit `bd.Plane` frames, non-pure-z `Location`
   placements) or fails on untyped (`Plane.XY`/`Plane.XZ` frame constants,
   `Vector` direction methods). None of the ten staged rows reaches its first
   loft/revolve verb or any boolean, so the kernel executor's boolean/admission
   state is never exercised by a hypercar row on this dispatch.
2. **Two rows fail untyped because the census data model's attribute surface is
   thinner than the corpus's client-layer algebra.** `interior` reaches for
   `bd.Plane.XY` and `lighting` for `Vector.normalized`; neither is on the
   drop-in data row. As with the FH `tube()` rows, these are defects a later
   authoring-arm packet (the F1-AUTHORING-ARMS extrude/`tube()` program) should
   close by answering the frame-constant and direction-math attributes or
   refusing them typed — before the two rows can carry a typed census verdict.
3. **Three hypercar rows are unstaged: their reference-file short names collide
   with same-named F1 rows.** `hypercar/details`, `hypercar/suspension_front`
   and `hypercar/suspension_rear` have no recorded `hypercar/*` reference
   (`reference/details.json`, `reference/suspension_front.json`,
   `reference/suspension_rear.json` belong to the F1 rows), so the coverage map
   carries a 3-row gap the next corpus-recording step must close (e.g. a
   re-census that records hypercar-namespaced references) before those rows can
   be run against the kernel door.
4. **The OCC-baseline references for all ten staged hypercar rows reproduce
   bit-identically** (10/10 OCC door runs green, facts equal to the recorded
   doubles; the row solids range 10–440 and STL 1229–325242 triangles). The
   green-side admission program therefore has a trustworthy baseline to compare
   against once the authoring surface lifts.

## Anchors

- A1 `grep -c '"id"' corpus/ttc/MANIFEST.json` = 48 (expect 48) — holds.
- A2 `grep -c 'chassis' corpus/ttc/MANIFEST.json` = 1 (expect 1) — holds.
