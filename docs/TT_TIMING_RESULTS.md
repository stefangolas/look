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

> **Superseded (FH-TIMING-REFRESH).** The section below records the initial FH
> kernel-door adjudication at HEAD `cdd4480`, where the truck regime refused
> both rows typed on the first spline-profile revolve before any geometry fact
> existed. That lathe boundary landed since (FH-SPLINE-LATHE) together with the
> authoring arms (extrude/loft/sweep-chain/mirror), so every FH canonical row's
> kernel-door gate was re-adjudicated at HEAD `34bcc5a` under FH-TIMING-REFRESH
> — see that section below for the current facts-gate verdicts and the three
> green rows' kernel timing column. The historical record is kept intact.

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

# FH-TIMING-REFRESH — Falcon-Heavy kernel timing column, facts-gated

Second Falcon-Heavy timing run (FH-TIMING-REFRESH) at a later worktree HEAD
than the TTC-TIMING-FH record above. Since that run, FH-SPLINE-LATHE landed the
spline-profile lathe arm and the authoring arms (extrude/loft/sweep-chain/
mirror/make_face) on the truck drop-in, so the FH kernel-door gates were
re-adjudicated at this HEAD before any timing: every FH canonical row was run
once through `corpus/ttc/door.py --engine truck` (the kernel-engine door
regime, `door_version 2`, one fresh Python process per row), and the kernel
facts were compared to the recorded OCC reference facts within the recorded
tolerances (`corpus/ttc/reference/*.json`). Every row whose kernel facts gate
is GREEN then carries the full BENCHMARKS-protocol timing series below — OCC
baseline and kernel, one unmeasured conditioning run per engine per row, five
measured runs per engine per row, median reported, raw samples retained,
alternating launch order. Rows with a RED gate stay DNF — no timing is
published against a red facts gate (no cross-row averages, no driver tuning, no
debug builds).

## Machine / environment (FH-TIMING-REFRESH run)

- Windows x86_64 (win32), measured 2026-09-08 in a quiet window of the
  autobuild loop (no concurrent `cargo`/`rustc`/test process during the
  measured runs; the cargo queue was idle immediately before and after the
  series).
- Python 3.14.3; `build123d` 0.11.1; `cadquery-ocp` 7.9.3.1.1 (OCC baseline).
- Door versions: `occ` regime `door_version 1`; `truck` (kernel) regime
  `door_version 2` over the release-built `truck123d` native module at this
  worktree HEAD (`cargo build --release --locked` and
  `cargo build --release -p truck123d --locked` green; `truck123d.dll`
  staged as `truck123d.pyd`, importable from a real 3.14 interpreter).
- Worktree HEAD `34bcc5a`, branch `packet/FH-TIMING-REFRESH`.
- References: `corpus/ttc/reference/chamber_assembly.json`,
  `corpus/ttc/reference/turbopump_assembly.json`,
  `corpus/ttc/reference/fairing.json` (the green rows), plus the existing
  `nozzle_assembly.json` / `mvac.json` for the re-adjudicated red rows.
- For every row the reported wall time is the whole fresh door-process wall per
  measured run (spawn to the stdout JSON record), which puts both engines on
  equal footing: in the truck regime the deterministic kernel executor computes
  the volume/bbox facts and the STL tessellation inside the door process after
  the script's build wall, so a `build_seconds`-only column would hide the
  kernel's geometry time while including all of OCC's. The door's own
  `build_seconds` samples are retained alongside for transparency.

## Facts-gate adjudication — FH canonical rows at this HEAD

| row | kernel door (`door_version 2`) | facts gate vs recorded reference |
|---|---|---|
| falcon_heavy/turbopump_assembly | runs end to end (59 solids) | GREEN — timed below |
| falcon_heavy/chamber_assembly | runs end to end (46 solids) | GREEN — timed below |
| falcon_heavy/fairing | runs end to end (4 solids) | GREEN — timed below |
| falcon_heavy/nozzle_assembly | runs end to end (12 solids) | RED — volume 1.159e-4 rel beyond the recorded `volume_rel` 1e-4 band (kernel 50350859.12990023 vs recorded 50356695.264747284); solid_count/bbox GREEN → DNF-FACTS |
| falcon_heavy/mvac | Refused typed at the thrust-structure TVC housing `Location` | RED — no kernel facts exist → DNF |
| falcon_heavy/thrust_structure | Refused typed at the same `Location` carrier | RED — no kernel facts exist → DNF |
| falcon_heavy/second_stage | Refused typed at the same `Location` carrier (composes mvac) | RED → DNF |
| falcon_heavy/engine_prototype | Refused typed at the same `Location` carrier | RED → DNF |
| falcon_heavy/vehicle | Refused typed at the same `Location` carrier (composes mvac) | RED → DNF |
| falcon_heavy/gas_generator_assembly | Refused typed at the `tube()` spline-path sweep | RED → DNF |
| falcon_heavy/turbine_exhaust_assembly | Refused typed at the `tube()` spline-path sweep | RED → DNF |
| falcon_heavy/feed_assembly | Refused typed at the `tube()` spline-path sweep | RED → DNF |
| falcon_heavy/harness_assembly | Refused typed at the `tube()` spline-path sweep | RED → DNF |

Three FH rows carry a GREEN kernel facts gate at this HEAD and are timed below;
the ten RED rows publish no kernel timing (DNF kept). Two adjudications moved
since the TTC-TIMING-FH record: the spline-profile lathe boundary that stopped
both headline rows is closed, so the rows were re-adjudicated at their current
carrier instead of the old "spline-profile revolve" refusal —

- **nozzle_assembly** now runs to kernel facts, but its recorded reference
  volume is OCC's default-`BRepGProp` measurement, which carries a
  deterministic ~1.16e-4-relative bias on spline surfaces of revolution
  (FH-SPLINE-LATHE finding F1). The kernel's exact volume
  (50350859.12990023) equals OCC's Eps-converged value to ~2e-9 yet lies
  1.159e-4-relative outside the recorded `volume_rel` 1e-4 band, so **no exact
  engine can satisfy the recorded nozzle `volume_rel`**. The gate is RED and
  the row is DNF-FACTS with that delta — no kernel timing is published.
- **mvac** proceeds through the chamber/nozzle spline shells and the
  thrust-cone-gusset extrude, then refuses typed at the first non-z Euler
  `Location` placement (the TVC actuator housing in `make_thrust_structure`,
  `bd.Location(..., (55*sin(a), 55*cos(a), 0))`). Verbatim (`door --engine
  truck` exit 1, `door_version 2`; the drop-in maps every typed refusal to the
  standard payload):

```json
{
  "kind": "Refused",
  "message": "Location forms beyond a translation or pure-z rotation are not census carriers",
  "payload": {"case": "unsupported_envelope", "envelope": "non_canonical_carrier"}
}
```

The four `tube()` rows (`gas_generator_assembly`, `turbine_exhaust_assembly`,
`feed_assembly`, `harness_assembly`) refuse typed at the spline-path sweep
surface the FH-CENSUS recorded as an untyped door DNF; the authoring arms
closed that defect and the rows now die with the mapped `Refused` class
(`"a spline path tangent query is not a kernel-engine row"`). All of these stay
DNF rows under their RED facts gate.

## falcon_heavy/turbopump_assembly — turbopump package (`make_turbopump_assembly`, `lib.merlin_common`)

The row builds the turbopump package from cylinder/sphere primitives and
line-loop revolves (volute, impeller-ish disks, housings, flanges) plus the
spline-contour feed-line stubs — 59 solids, no boolean ops, no spline-profile
lathe carriers in the authoring path. Recorded reference facts:
`solid_count 59`, `volume 77563936.47352602`,
`bbox [[-162.0, -172.0000001, 1040.0], [672.5, 172.0000001, 2018.0]]`.

| engine | verdict | facts-match | wall median (s) | build median (s) | STL triangles |
|---|---:|---:|---:|---:|---:|
| occ | green | GREEN | 4.866 | 3.927 | 8830 |
| truck (kernel) | green | GREEN | 0.097 | 0.003 | 35352 |

Every measured run reproduced its engine's facts deterministically. OCC facts
equal the recorded reference exactly; kernel facts match within the recorded
tolerances: `solid_count 59` exact, kernel volume 77563936.47352579 vs recorded
77563936.47352602 (`volume_rel` ~3e-15 — the line-profile facts stay
bit-identical to the FH-SPLINE-LATHE V5 pair), bbox within `bbox_abs` 1e-3. The
kernel wall median (~0.097 s) is dominated by the fresh interpreter spawn; the
kernel build + facts + STL are milliseconds — the same fresh-process footing
the OCC column reports.

OCC raw samples (5 measured fresh door runs, seconds):

| run | door process wall | door `build_seconds` |
|---|---:|---:|
| 1 | 4.866 | 3.964 |
| 2 | 4.866 | 3.927 |
| 3 | 5.092 | 4.182 |
| 4 | 4.633 | 3.754 |
| 5 | 4.631 | 3.708 |
| median | 4.866 | 3.927 |

Kernel raw samples (5 measured fresh door runs, seconds):

| run | door process wall | door `build_seconds` |
|---|---:|---:|
| 1 | 0.100 | 0.004 |
| 2 | 0.097 | 0.003 |
| 3 | 0.103 | 0.004 |
| 4 | 0.087 | 0.003 |
| 5 | 0.082 | 0.003 |
| median | 0.097 | 0.003 |

## falcon_heavy/chamber_assembly — regeneratively cooled chamber (`make_chamber_assembly`, `lib.merlin_common`)

The row builds the chamber liner/jacket as sampled-contour spline-profile
revolved shells over `chamber_gas_contour()` plus the flange/ring/bolt
primitives — 46 solids, no boolean ops. Recorded reference facts:
`solid_count 46`, `volume 45459001.162568755`,
`bbox [[-249.0, -249.0, 1164.9999999], [249.0, 249.0, 2124.0]]`.

| engine | verdict | facts-match | wall median (s) | build median (s) | STL triangles |
|---|---:|---:|---:|---:|---:|
| occ | green | GREEN | 4.766 | 3.875 | 5457 |
| truck (kernel) | green | GREEN | 0.111 | 0.004 | 27520 |

Every measured run reproduced its engine's facts deterministically. OCC facts
equal the recorded reference exactly; kernel facts match within the recorded
tolerances: `solid_count 46` exact, kernel volume 45458782.5489467 vs recorded
45459001.162568755 (`volume_rel` 4.8e-6), bbox within `bbox_abs` 1e-3.

OCC raw samples (5 measured fresh door runs, seconds):

| run | door process wall | door `build_seconds` |
|---|---:|---:|
| 1 | 4.742 | 3.875 |
| 2 | 5.327 | 4.255 |
| 3 | 5.180 | 4.263 |
| 4 | 4.715 | 3.773 |
| 5 | 4.766 | 3.834 |
| median | 4.766 | 3.875 |

Kernel raw samples (5 measured fresh door runs, seconds):

| run | door process wall | door `build_seconds` |
|---|---:|---:|
| 1 | 0.112 | 0.003 |
| 2 | 0.111 | 0.004 |
| 3 | 0.102 | 0.004 |
| 4 | 0.097 | 0.003 |
| 5 | 0.111 | 0.004 |
| median | 0.111 | 0.004 |

## falcon_heavy/fairing — payload fairing (`make_fairing`, `lib.falcon_common`)

The row builds the two payload-fairing half shells (spline-profile revolved
over the ogive contour) plus tip and base collars — 4 solids, no boolean ops.
Recorded reference facts: `solid_count 4`, `volume 11027028467.643568`,
`bbox [[-2761.7733353381223, -2761.7733353381223, 56849.9999999],
[2761.7733353381223, 2761.7733353381223, 70000.0000001]]`.

| engine | verdict | facts-match | wall median (s) | build median (s) | STL triangles |
|---|---:|---:|---:|---:|---:|
| occ | green | GREEN | 4.563 | 3.728 | 784 |
| truck (kernel) | green | GREEN | 0.096 | 0.004 | 11032 |

Every measured run reproduced its engine's facts deterministically. OCC facts
equal the recorded reference exactly; kernel facts match within the recorded
tolerances: `solid_count 4` exact, kernel volume 11027439648.490429 vs recorded
11027028467.643568 (`volume_rel` 3.7e-5), bbox within `bbox_abs` 1e-3 (kernel
bbox [[-2761.773335238123, -2761.773335238123, 56850.0],
[2761.773335238123, 2761.773335238123, 70000.0]]). The OCC door tessellates
this 15 km-diagonal smooth shell coarsely at the deflection rule
(`max(diag * 2e-4, 0.01)`), hence 784 OCC triangles vs the kernel's
deterministic 11032-triangle mesh.

OCC raw samples (5 measured fresh door runs, seconds):

| run | door process wall | door `build_seconds` |
|---|---:|---:|
| 1 | 4.560 | 3.696 |
| 2 | 5.015 | 4.111 |
| 3 | 4.563 | 3.728 |
| 4 | 4.545 | 3.660 |
| 5 | 4.957 | 4.082 |
| median | 4.563 | 3.728 |

Kernel raw samples (5 measured fresh door runs, seconds):

| run | door process wall | door `build_seconds` |
|---|---:|---:|
| 1 | 0.099 | 0.003 |
| 2 | 0.095 | 0.003 |
| 3 | 0.084 | 0.004 |
| 4 | 0.096 | 0.004 |
| 5 | 0.106 | 0.004 |
| median | 0.096 | 0.004 |

## Findings — stop-condition record (FH-TIMING-REFRESH)

1. Three FH rows carry a GREEN kernel facts gate at this HEAD — turbopump_assembly,
   chamber_assembly, fairing — and each was timed on both engines in one quiet
   window (conditioning + 5 measured runs per engine, raw samples above). The
   kernel fresh door-process wall is ~0.10 s median on all three against
   ~4.6–4.9 s for OCC — a per-row ~45–50x fresh-door-process ratio — with the
   kernel's own build + facts + STL inside the process at milliseconds and the
   remainder interpreter spawn, and OCC's at ~3.7–3.9 s `build_seconds` plus
   import/tessellation overhead. These are per-row numbers on one machine, not
   a cross-row claim; no cross-row averages are computed.
2. The two TTC-TIMING-FH headline rows stay DNF but for moved reasons:
   `nozzle_assembly` runs to kernel facts yet its recorded reference volume is
   OCC-default-`BRepGProp` biased (kernel volume is 1.159e-4-relative outside
   the recorded 1e-4 band — no exact engine can satisfy it as recorded;
   FH-SPLINE-LATHE F1, re-record-converged resolution still open), and `mvac`
   refuses typed at the non-z Euler `Location` placement carrier inside
   `make_thrust_structure`. No kernel timing is published against either RED
   gate.
3. The remaining FH canonical rows (thrust_structure, second_stage,
   engine_prototype, vehicle; gas_generator_assembly, turbine_exhaust_assembly,
   feed_assembly, harness_assembly) refuse typed at the same two carrier
   surfaces (non-z `Location` placement; `tube()` spline-path sweep). They keep
   DNF rows until those carriers are answered; the sweep table above is the
   per-row verdict record.
4. Quiet-window stop condition: the measured series started only after the
   cargo queue was idle and no `cargo`/`rustc`/test process existed; the queue
   was re-checked idle immediately after the series. No contended measurement
   was recorded.

# TTC-RECENSUS-F1 — F1 closing re-census (no F1 row flips; no new kernel column)

The F1 closing re-census (`docs/TTC_CENSUS_FINAL.md`) re-ran all 21 F1 manifest rows
through the kernel door at this worktree HEAD (`07b2090`, post-ADM-004 / post
F1-AUTHORING-ARMS). **No F1 row certified on that dispatch** — 0 green rows out of 21
(12 typed-refusal at authoring carriers — `Plane` frame algebra / `Pos` / `Spline`
profile authoring — and 9 kernel-door DNF, untyped, on the drop-in `Vector` data-row
surface), so no kernel timing column is appended here and no F1 row's timing is
published. The existing F1 rows' records above are the standing timing content; the
census's timing statement is that the F1 kernel timing column stays closed row by row
only for green rows, and none flipped. OCC-baseline door runs reproduced every F1
row's recorded reference bit-identically (21/21), so the recorded facts gate did not
drift and no OCC flake was recorded.

# TTC-RECENSUS-F1-R2 — F1 post-chain re-census (no F1 row green; no new kernel column)

The post-door-gap-chain F1 re-census (`docs/TTC_CENSUS_FINAL.md`, HEAD
`de33f33`, release-built `truck123d`) re-ran all 21 F1 manifest rows through the
kernel door (`corpus/ttc/door.py --engine truck`, `door_version 2`, one fresh
Python process per row, serial, quiet machine) and compared the kernel facts
against the recorded references (`corpus/ttc/reference/*.json`; `solid_count`
exact, `volume`/`bbox` in the recorded bands). **Zero rows are facts-green** —
0 green, 15 typed-refusal (multi-station spline loft / native boolean / trim
constructor / mirror / OCC-probe), 6 kernel-door DNF (untyped corner
`surfaces.bbox` and floor-loft builder). No OCC process ran; the recorded
references were the sole oracle.

Because no row flipped green, no F1 row carries a kernel timing column and no
timing is published against a red facts gate. The F1 kernel timing column
therefore stays closed, and the standing FH timing content above is unchanged.

The V5 net holds: the three previously-green FH timing rows answer
bit-identically on the release build at this HEAD — `turbopump_assembly`
(solid_count 59, volume 77563936.47352579, 35352 triangles), `chamber_assembly`
(46, 45458782.5489467, 27520) and `fairing` (4, 11027439648.490429, 11032) — so
no verdict flip occurred on a landed row.

