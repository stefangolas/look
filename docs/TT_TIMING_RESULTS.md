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
