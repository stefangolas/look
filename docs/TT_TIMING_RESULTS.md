# TTC per-model timing results

Segmented per-model timing of the text-to-cad corpus rows through the
`corpus/ttc/door.py` harness, one engine per column. Protocol is identical to
TTC-TIMING-FH: release regime, quiet machine, one fresh process per run, five
measured runs, median reported, raw samples retained, DNF rows kept, no
cross-row averages. Each model is facts-gated: an engine's timing is reported
only when that engine's door-run geometry facts equal the OCC-derived recorded
reference facts within the recorded tolerances (`corpus/ttc/reference/*.json`).

## Method and segmentation

Rows run the corpus's real geometry entry through the door — no transcription.
Each run is one fresh Python process that reproduces the door exactly (cadgen
alias → the engine, the entry call, the facts record, the deflection rule
`max(diag * 2e-4, 0.01)`, `export_stl` with `angular_tolerance=0.5`). Within a
row the build wall is split into three segments:

- **construction** — authoring (section faces, lofts/sweeps, primitives), i.e.
  the build wall minus the boolean wall;
- **boolean** — wall time inside the boolean verbs the unmodified corpus script
  actually reaches (observed as `Shape.fuse` / `Shape.cut` calls; observation
  only, no geometry is altered);
- **tessellation** — the door's STL export.

Per-run `build` (construction + boolean) is also recorded and is consistent
with the door's own `build_seconds`.

## Machine / environment

- Windows x86_64 (win32), measured 2026-09-07 in a quiet window of the
  autobuild loop (no concurrent `cargo`/`rustc` during the measured runs).
- Python 3.14.3; `build123d` 0.11.1; `cadquery-ocp` 7.9.3.1.1 (OCC baseline).
- Door versions: `occ` regime `door_version 1`; `truck` (kernel) regime
  `door_version 2` over the release-built `truck123d` native module.
- Worktree HEAD `fd12346`, branch `packet/TTC-TIMING-MONO`.
- Reference: `corpus/ttc/reference/monocoque.json` (recorded OCC baseline,
  PB-011 lift evidence).

## Facts-match gate — f1/monocoque

| engine | gate | outcome |
|---|---|---|
| occ (door_version 1) | OCC facts vs recorded reference | GREEN — 5/5 runs facts identical to the recorded reference |
| truck (door_version 2) | kernel facts vs OCC door facts | RED — the kernel engine refuses on the tub (typed) before any geometry fact exists |

The cross-engine facts-match gate (kernel facts == OCC door facts within
tolerance) is therefore **RED for this row**: the truck regime refuses the tub
typed (DNF-KERNEL below), so no kernel facts and no kernel timing are
published. The OCC numbers below are the one-sided baseline only (the OCC
column of the row); they are not a kernel-vs-OCC comparison.

## f1/monocoque — survival tub (`build_monocoque`, `lib.monocoque`)

`cut(swept,canonical)` (lofted skin minus cavity) + `fuse(swept,canonical)`
(proud bosses). OCC run: 2 boolean ops observed — the cavity cut of the lofted
tub skin and the 14-body proud-boss fuse (`mono_tub._fuse_proud`).

| engine | verdict | construction median (s) | boolean median (s) | tessellation median (s) | build (cons+bool) median (s) | solid_count / volume / STL tris |
|---|---:|---:|---:|---:|---:|---:|
| occ | green | 5.639 | 36.454 | 0.195 | 42.096 | 2 / 6.348040e+08 / 6971 |
| truck | **DNF-KERNEL** | — | — | — | — | — |

OCC raw samples (5 measured runs, seconds):

| run | construction | boolean | tessellation | build |
|---|---:|---:|---:|---:|
| 1 | 5.532 | 34.489 | 0.193 | 40.024 |
| 2 | 5.557 | 34.559 | 0.195 | 40.119 |
| 3 | 5.687 | 36.627 | 0.186 | 42.318 |
| 4 | 5.639 | 36.454 | 0.203 | 42.096 |
| 5 | 5.705 | 36.729 | 0.199 | 42.437 |
| median | 5.639 | 36.454 | 0.195 | 42.096 |

Facts were identical on every OCC run and equal the recorded reference exactly:
`solid_count 2`, `volume 634804090.9180949`,
`bbox [[-1981.2766075073669, -396.69746881621256, 103.69655901288232],
[700.0000001000002, 396.69746881621836, 949.883519840629]]`, STL 6971
triangles. A single door `--engine occ` cross-check run reproduced the same
facts and 6971 triangles (`build_seconds 45.093`). The corpus documents OCC
trouble on this class (a `Null TopoDS_Shape` 1-in-3 under 4-way concurrency);
these serial quiet runs were green and deterministic.

### Kernel verdict — DNF-KERNEL

The truck regime refuses on the tub, typed. Refusal recorded verbatim:

```json
{
  "kind": "Refused",
  "message": "Plane algebra is recorded client-layer data; no kernel row",
  "payload": {"case": "unsupported_envelope", "envelope": "non_canonical_carrier"}
}
```

The tub's build path refuses at the first non-census carrier before any boolean
runs: the lofted-skin/cavity/boss authoring goes through Plane algebra and
spline section faces (`surfaces.section_face`/`half_section_face` authoring
`bd.Plane`/`bd.Spline` + `bd.make_face`), none of which the truck door regime
answers as kernel-engine rows. The boolean pair itself is swept-carrier
composition (lofted skin − lofted cavity; lofted proud bosses fused). Do not
tune: per the packet stop condition the verdict is recorded as-is.

**Finding filed for the funnel programs:** before `f1/monocoque` can carry a
kernel timing column, the truck door regime must answer the tub's authoring
path — Plane/spline/loft authoring carriers in the corpus's `lib/surfaces.py`
vocabulary — or the row must be driven through the native facade table regime
whose swept-carrier boolean routing is landed. The swept x swept boolean
composition class is PB-011C's (the rows whose first refusing op is swept x
swept) and resolves to the BIE program's sweep-pair certification (BIE-006,
`corpus/ttc/SKIPS.json` reason `booleans-on-swept-carriers`). This row was not
timed on the kernel side because the authoring-carrier surface is the blocker,
and no timing is published against a red facts gate.

# TTC-TIMING-FH — Falcon-Heavy rows (nozzle_assembly, mvac)

First OCCT-vs-truck per-model timing run over Falcon-Heavy canonical rows. The
method is the packet's plain door protocol (no hand-transcribed drivers, no
bypass): both engines run the SAME vendored corpus scripts through the door
(`corpus/ttc/door.py`, fresh process per run). The OCC column comes from the
door's `occ` regime (door_version 1); the kernel column from the door's
`--engine truck` regime (door_version 2, the executor binding landed by
TTC-EXECUTOR-BINDING), over the release-built `truck123d` native module at
this HEAD. Each row reports its own three columns — wall time / verdict /
facts-match — so a blow-up on one row never contaminates another. The
facts-match gate is adjudicated BEFORE any timing is reported: the OCC door
facts must equal the recorded OCC reference facts (solid count exact, volume
and bbox within the recorded tolerances), and the kernel facts (when they
exist) must equal the same reference. No cross-row averages are computed.

For these two rows the reported wall time is the whole fresh door-process wall
per measured run (spawn to the stdout JSON record), which puts both engines on
equal footing: in the truck regime the deterministic kernel executor computes
the volume/bbox facts and the STL tessellation inside the door process, after
the script's build wall, so a `build_seconds`-only column would hide the
kernel's geometry time while including all of OCC's. The door's own
`build_seconds` samples are retained alongside for transparency.

## Machine / environment (FH run)

- Windows x86_64 (win32), measured 2026-09-08 in a quiet window of the
  autobuild loop (no concurrent `cargo`/`rustc`/test process during the
  measured runs; confirmed immediately before the measured series).
- Python 3.14.3; `build123d` 0.11.1; `cadquery-ocp` 7.9.3.1.1 (OCC baseline).
- Door versions: `occ` regime `door_version 1`; `truck` (kernel) regime
  `door_version 2` over the release-built `truck123d` native module
  (`cargo build --release --locked` and
  `cargo build --release -p truck123d --locked` green; `truck123d.dll`
  staged as `truck123d.pyd` beside the interpreter runtimes).
- Worktree HEAD `cdd4480`, branch `packet/TTC-TIMING-FH`.
- One unmeasured conditioning run per engine per row first, then five
  measured runs per engine per row (median reported, raw samples retained).
- References: `corpus/ttc/reference/nozzle_assembly.json` and
  `corpus/ttc/reference/mvac.json` (recorded OCC baselines).

## Facts-match gate — both rows

| row | engine | gate | outcome |
|---|---|---|---|
| falcon_heavy/nozzle_assembly | occ (door_version 1) | OCC facts vs recorded reference | GREEN — conditioning + 5/5 measured runs facts identical to the recorded reference |
| falcon_heavy/nozzle_assembly | truck (door_version 2) | kernel facts vs recorded reference | RED — the kernel engine refuses typed on the first spline-profile revolve before any geometry fact exists |
| falcon_heavy/mvac | occ (door_version 1) | OCC facts vs recorded reference | GREEN — conditioning + 5/5 measured runs facts identical to the recorded reference |
| falcon_heavy/mvac | truck (door_version 2) | kernel facts vs recorded reference | RED — the kernel engine refuses typed on the first spline-profile revolve before any geometry fact exists |

The cross-engine facts-match gate (kernel facts == reference facts within
tolerance) is therefore **RED for both rows**: the truck regime refuses typed
on the authoring carrier, so no kernel facts and no kernel timing are
published. The OCC numbers below are the one-sided baseline only (the OCC
column of each row); no kernel-vs-OCC comparison is made against a red facts
gate.

## falcon_heavy/nozzle_assembly — sea-level nozzle (`make_nozzle_assembly`, `lib.merlin_common`)

The row builds the Merlin 1D nozzle as sampled-contour revolved shells (regen
liner + jacket over `bell_gas_contour()`, the gas-side contour is a spline
through 30+ samples), six decorative stiffener band rings, three heat-tint
sleeves and the exit stiffener torus — 12 solids, no boolean ops. Recorded
reference facts: `solid_count 12`, `volume 50356695.264747284`,
`bbox [[-504.0000001, -504.0000001, -1e-07],
[504.0000001, 504.0000001, 1165.0000001]]`.

| engine | verdict | facts-match | wall median (s) | STL triangles |
|---|---:|---:|---:|---:|
| occ | green | GREEN | 4.123 | 3696 |
| truck (kernel) | DNF-FACTS | RED (typed refusal before any geometry fact) | — | — |

OCC raw samples (5 measured fresh door runs, seconds):

| run | door process wall | door `build_seconds` |
|---|---:|---:|
| 1 | 4.137 | 3.330 |
| 2 | 4.047 | 3.253 |
| 3 | 4.206 | 3.347 |
| 4 | 4.123 | 3.313 |
| 5 | 4.063 | 3.256 |
| median | 4.123 | 3.313 |

Every OCC run reproduced the recorded reference facts exactly (solid_count 12,
volume 50356695.264747284, bbox to the recorded precision) and a 3696-triangle
STL. A fresh conditioning run agreed.

### Kernel verdict — DNF-FACTS

The truck regime refuses on the first `revolved_shell` of the liner, typed.
Refusal recorded verbatim (`door --engine truck` exit 1, `door_version 2`):

```json
{
  "kind": "Refused",
  "message": "a spline-profile revolve is not a kernel-engine row",
  "payload": {"case": "unsupported_envelope", "envelope": "non_canonical_carrier"}
}
```

The nozzle's gas-side contour is authored as `bd.Edge.make_spline` over the
sampled `bell_gas_contour()` points and revolved via the truck drop-in's
lathe arm, whose recorded envelope covers only a closed line-loop profile.
Do not tune: per the packet stop condition the verdict is recorded as-is with
no kernel timing and no facts.

## falcon_heavy/mvac — MVac derivative (`make_mvac`, `lib.falcon_common`)

The row composes the powerhead subsystems (`make_chamber_assembly`,
`make_thrust_structure`, `make_turbopump_assembly`,
`make_gas_generator_assembly`) plus the schematic two-cone niobium skirt and
exit stiffener — 56 solids, no boolean ops. Recorded reference facts:
`solid_count 56`, `volume 0`, `bbox [[-1462.0, -1462.0, -2730.0],
[1462.0, 1462.0, 2526.0]]` (the recorded OCC baseline reads the compound's
volume as 0).

| engine | verdict | facts-match | wall median (s) | STL triangles |
|---|---:|---:|---:|---:|
| occ | green | GREEN | 4.170 | 10259 |
| truck (kernel) | DNF-FACTS | RED (typed refusal before any geometry fact) | — | — |

OCC raw samples (5 measured fresh door runs, seconds):

| run | door process wall | door `build_seconds` |
|---|---:|---:|
| 1 | 4.151 | 3.300 |
| 2 | 4.173 | 3.304 |
| 3 | 4.199 | 3.339 |
| 4 | 4.151 | 3.307 |
| 5 | 4.170 | 3.313 |
| median | 4.170 | 3.307 |

Every OCC run reproduced the recorded reference facts exactly (solid_count 56,
volume 0, bbox to the recorded precision) and a 10259-triangle STL. A fresh
conditioning run agreed.

### Kernel verdict — DNF-FACTS

Identical blocker to the nozzle row: the truck regime refuses typed on the
first spline-profile revolve (the chamber liner's sampled contour), before any
geometry fact exists. Refusal recorded verbatim (`door --engine truck` exit 1,
`door_version 2`):

```json
{
  "kind": "Refused",
  "message": "a spline-profile revolve is not a kernel-engine row",
  "payload": {"case": "unsupported_envelope", "envelope": "non_canonical_carrier"}
}
```

## Findings — stop-condition record (both rows)

1. The OCC side of the gate is GREEN and deterministic for both FH canonical
   rows, and both are fast: ~4.1–4.2 s median fresh door-process wall with the
   door's `build_seconds` at ~3.3 s. Neither row reaches a boolean op — the
   build is sampled-contour revolves and primitives — so these rows cannot
   carry mono's boolean-segmentation story.
2. The kernel engine refuses BOTH rows typed at the first spline-profile
   revolve — the bell/chamber sampled contours are authored with
   `bd.Edge.make_spline` (`bell_gas_contour` / `chamber_gas_contour` →
   `revolved_shell`) — before any geometry fact exists. The facts-match gate
   is RED and no kernel timing is published (DNF rows kept; no driver tuning,
   no averaging, no debug builds).
3. **Finding filed for the funnel programs:** the blocker is the same
   authoring-carrier surface mono's tub hit, one carrier deeper. The truck
   door regime's lathe arm answers only a closed line-loop profile; the
   Falcon-Heavy canonical rows' nozzle/chamber/skirt shells are spline-profile
   revolves over sampled contours. Before a kernel timing column can exist on
   this family, the truck regime must answer a spline-profile revolve (sample
   the contour to a line-polyline loop on the recording side, or cover the
   sampled-contour lathe on the executor side) — the FH canonical rows are the
   natural first rows for that lift, and until it lands the OCC column above
   is the one-sided baseline only.
