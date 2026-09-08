# FH-CENSUS — Falcon-Heavy kernel-door sweep (six untested canonical rows)

Mechanical census (PB-011C protocol applied to the Falcon-Heavy family): every
FH canonical row that had never seen the kernel door
(`chamber_assembly`, `feed_assembly`, `gas_generator_assembly`,
`second_stage`, `thrust_structure`, `turbine_exhaust_assembly`) was run
through `corpus/ttc/door.py --engine truck` (the kernel-engine door regime,
`door_version 2`) once, one fresh Python process per row, serial — the
documented under-load flake regime, never concurrent. Typed refusals are valid
outcomes; this is a measurement, not a lift packet. No timing was performed
(benchmark-grade wall columns are FH-TIMING-REFRESH's, against green rows
only); the `wall_s` column below is the single fresh door-process wall of the
census run itself.

## Machine / environment

- Windows x86_64 (win32), census run 2026-09-08 on the autobuild host.
- Python 3.14.3; the truck regime's native executor is the release-built
  `truck123d` module at this worktree HEAD, staged as `truck123d.pyd` beside
  the interpreter runtimes (same staging the TTC-TIMING-FH run used).
- Worktree HEAD `54ef5f6`, branch `packet/FH-CENSUS`.
- Reference rows all present: `corpus/ttc/reference/<row>.json` exists for all
  six rows (no UNSTAGED stop condition fired).
- No OCC door run was needed (the stop condition is per-row UNSTAGED or OCC-DNF
  only; every row below has a recorded OCC reference and no OCC run failed).

## Census verdict table

| row | first refusing verb | carrier classes in collision | verdict | wall_s |
|---|---|---|---|---:|
| falcon_heavy/chamber_assembly | `revolve` | spline-profile revolve (sampled-contour non-canonical carrier) vs the executor's lathe arm (closed line-loop profile only) | typed-refusal | 0.064 |
| falcon_heavy/thrust_structure | `extrude` | Face extrusion (closed line-loop planar profile; the S6 extrusion verb is not yet a kernel-engine row) | typed-refusal | 0.072 |
| falcon_heavy/second_stage | `revolve` | spline-profile revolve (same carrier as the chamber row; `make_second_stage` builds `make_mvac` first) | typed-refusal | 0.069 |
| falcon_heavy/feed_assembly | `sweep` (spline-sweep authoring) | spline-path sweep carrier via `tube()`; door fails before the sweep verb, untyped | kernel-door DNF (untyped) | 0.062 |
| falcon_heavy/gas_generator_assembly | `sweep` (spline-sweep authoring) | spline-path sweep carrier via `tube()`; door fails before the sweep verb, untyped | kernel-door DNF (untyped) | 0.066 |
| falcon_heavy/turbine_exhaust_assembly | `sweep` (spline-sweep authoring) | spline-path sweep carrier via `tube()`; door fails before the sweep verb, untyped | kernel-door DNF (untyped) | 0.061 |

Six rows, six verdicts, no gaps. No row returned geometry facts: every run
refused or failed before any fact existed, so no
`corpus/ttc/reference/<row>.json` comparison ran and no DNF-FACTS delta was
recorded. No row stagnated (no solver budget numbers attach): every run
terminated in well under a tenth of a second on a typed refusal or an untyped
door failure.

## Typed refusals (verbatim)

Each typed refusal carries the drop-in's standard mapped payload
`{"case": "unsupported_envelope", "envelope": "non_canonical_carrier"}`. No
typed refusal came back with a different case.

### falcon_heavy/chamber_assembly — census verb `revolve`

`make_chamber_assembly` (`lib.merlin_common`) opens with the chamber liner as a
`revolved_shell` over `chamber_gas_contour()`, a sampled contour authored with
`bd.Edge.make_spline`. The truck drop-in's lathe arm answers only a closed
line-loop profile; the first spline-profile revolve refuses typed. Refusal
recorded verbatim (`door --engine truck` exit 1, `door_version 2`):

```json
{
  "kind": "Refused",
  "message": "a spline-profile revolve is not a kernel-engine row",
  "payload": {"case": "unsupported_envelope", "envelope": "non_canonical_carrier"}
}
```

### falcon_heavy/thrust_structure — census verb `extrude`

`make_thrust_structure` (`lib.merlin_common`) builds the thrust cone as a
line-loop revolve (answered), then the four thrust-cone gussets as
`bd.extrude(face, amount=9.0, both=True)` over a triangular line-loop Face.
The drop-in's `extrude` refuses any Face extrusion typed. Refusal recorded
verbatim (`door --engine truck` exit 1, `door_version 2`):

```json
{
  "kind": "Refused",
  "message": "Face extrusion is not yet a kernel-engine row",
  "payload": {"case": "unsupported_envelope", "envelope": "non_canonical_carrier"}
}
```

### falcon_heavy/second_stage — census verb `revolve`

`make_second_stage` (`lib.falcon_common`) composes `make_mvac()` first, whose
first member is `make_chamber_assembly()`; the chamber liner's first
spline-profile revolve refuses typed exactly as the chamber row does. Refusal
recorded verbatim (`door --engine truck` exit 1, `door_version 2`):

```json
{
  "kind": "Refused",
  "message": "a spline-profile revolve is not a kernel-engine row",
  "payload": {"case": "unsupported_envelope", "envelope": "non_canonical_carrier"}
}
```

## Kernel-door DNF rows (untyped, recorded as-is — not tuned)

Three rows route through the `tube()` spline-sweep helper
(`lib/merlin_common.py`), which authors the sweep path as
`bd.Edge.make_spline(...)` and then queries it with `path.position_at(0)` /
`path.tangent_at(0)` before constructing the section (`Plane * Circle`) and
calling `bd.sweep`. The truck drop-in's `Edge` is a data carrier that does not
answer those query attributes, so the row fails before any census verb is
reached — an untyped `AttributeError`, not the mapped `Refused`. The row's
carrier class is the spline-path sweep, which is not a kernel-engine row; the
door simply never gets to answer `sweep` typed. Per the packet the door and the
bridge are not tuned to force a verdict, so the failure is recorded verbatim as
the observed kernel-door DNF.

### falcon_heavy/feed_assembly

First corpus surface reached: `tube([...], 80.0, "lox_main_feed_line...")`.
Recorded verbatim (`door --engine truck` exit 1, `door_version 2`):

```json
{
  "kind": "AttributeError",
  "message": "'Edge' object has no attribute 'position_at'"
}
```

### falcon_heavy/gas_generator_assembly

Preceding GG body/dome/flange/bolt parts are answered (cylinder/sphere/
line-loop revolve), then the GG feed lines reach `tube(...)`. Recorded
verbatim (`door --engine truck` exit 1, `door_version 2`):

```json
{
  "kind": "AttributeError",
  "message": "'Edge' object has no attribute 'position_at'"
}
```

### falcon_heavy/turbine_exhaust_assembly

First corpus surface reached: `tube([...], EXHAUST_DUCT_R,
"turbine_exhaust_duct...")`. Recorded verbatim (`door --engine truck` exit 1,
`door_version 2`):

```json
{
  "kind": "AttributeError",
  "message": "'Edge' object has no attribute 'position_at'"
}
```

## Findings filed

1. None of the six rows reaches the kernel executor's boolean/admission state:
   all six terminate in authoring before any geometry fact exists. The two
   blocker classes are the same spline-profile lathe boundary the
   TTC-TIMING-FH runs recorded on `nozzle_assembly`/`mvac`
   (`a spline-profile revolve is not a kernel-engine row`), plus two authoring
   carriers the truck door has not yet been asked to answer: Face extrusion
   (`thrust_structure`) and the `tube()` spline-path sweep surface
   (`feed_assembly`, `gas_generator_assembly`, `turbine_exhaust_assembly`).
2. The three `tube()` rows fail untyped (`Edge.position_at` is not on the
   drop-in's data `Edge`), i.e. the door regime's surface ends before the
   row's first census verb (`sweep`). A later sweep-authoring lift must answer
   the spline-path section/query attributes (or refuse them typed) before these
   rows can carry a typed census verdict.

## Anchors

- A1 `grep -c '"id"' corpus/ttc/MANIFEST.json` = 48 (expect 48) — holds.
- A2 `grep -c 'chamber_assembly' corpus/ttc/MANIFEST.json` = 1 (expect 1) —
  holds.
