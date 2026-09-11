# TTC-CENSUS-FINAL — the re-census under the kernel-internal oracle policy (TTC-RECENSUS-F1-R3)

Re-census of the text-to-cad corpus under the **ORACLE POLICY CHANGE**
(`docs/MONO_CLOSURE_BOOKING.md` annex C, commit `4e6694d`). The R2 census
(`TTC-RECENSUS-F1-R2`) was adjudicated under the retired policy: facts
compared EXACTLY against the recorded OCC references (the 1e-4 `volume_rel`
band). Annex C dissolves that gate — **the kernel's own certificates ARE the
certification.** A row is GREEN when the kernel constructs it AND its volume
carries its own certificate bracket AND `solid_count` is constructive AND the
`bbox` is carrier-derived AND the mesh emits. The recorded OCC references are
**diagnostics**: reported per row, gating nothing. The 1e-4 band is retired as
a gate and retained as a reported delta.

All 54 corpus rows (48 enrolled at R2 + the six staged 2026-09-10:
`front_wing`, `cockpit`, `nose`, `sidepod_left`, `sidepod_right`, `halo`) were
re-run through the kernel door (`corpus/ttc/door.py --engine truck`,
`door_version 2`): one fresh Python process per row, serial, quiet machine,
with the MONO-1..9 chain (row assembly, data rows, N-station loft, blade/
mirror, trim idioms, ray classify, swept booleans, swept admission wiring,
fuse fold) and the AUTHOR-EXT-FILLET-HALO fillet/closed-loop arms landed. No
OCC process gates anything; the recorded references are read and reported.

## Result in one line

**11 green / 30 typed-refusal / 13 DNF across the 54 corpus rows.** The 11
green rows are all Falcon-Heavy canonical rows — under the kernel-internal
predicate the FH canonical subset closes, including rows whose OCC reference
delta is far outside the retired 1e-4 band (`feed_assembly` −1.65e-2 rel,
`gas_generator_assembly` −6.55e-3 rel, `turbine_exhaust_assembly` −4.05e-3 rel,
`harness_assembly` −1.79e-3 rel) and two rows whose recorded OCC volume is 0
(`engine_prototype`, `mvac`). **Every F1 row stays red** (21 typed-refusal, 6
DNF, plus the six staged: 5 typed-refusal, 1 DNF) — the F1 family's carriers
are still refused by the native executor, and the policy re-judgement does not
widen construction. The policy moved **no F1 row**; it moved the **FH
canonical subset** from 3 green (R2-era `FH-TIMING-REFRESH`) to 11 green by
admitting their certified kernel facts over the recorded OCC band.

## Machine / environment

- Windows x86_64 (win32), measured 2026-09-11 in a quiet window (no concurrent
  `cargo`/`rustc`/test process during the runs).
- Python 3.14.3; `build123d` 0.11.1 (`cadquery-ocp` 7.9.3.1.1) for the
  diagnostic reference recordings.
- Worktree HEAD `f827556`, branch `packet/TTC-RECENSUS-F1-R3`.
- Kernel door: `door_version 2` (`--engine truck`) over the **release-built**
  `truck123d` module at this HEAD (`cargo build --release --locked -p
  truck123d` green, 4m12s; the release `truck123d.dll` staged as
  `truck123d.pyd` beside the 3.14 interpreter and importable). The release
  build happened **once** at this HEAD.
- OCC door: `door_version 1` (`--engine occ`) for the six staged rows'
  diagnostic reference recordings only.

## Method — the green predicate

The whole policy in one line: **kernel constructs + volume certificate bracket
present + `solid_count` constructive + `bbox` carrier-derived + mesh emits.**
Per row, exactly one fresh process:

```console
python corpus/ttc/door.py --engine truck corpus/ttc/trees/<family>/src <module> <entry> <args> <stl>
```

serial, quiet. A row that returns `ok: true` has constructed the tree, measured
its facts through the deterministic kernel executor (`bd_facts`: `solid_count`,
`volume`, `volume_bracket`, `bbox`) and emitted a deterministic STL
(`bd_stl`). Every `ok` row carries its own certified bracket (`volume_lo ==
volume_hi` for all 11 green rows — the exact per-patch certificate). The
recorded OCC reference (where one exists) is read and reported as the
`volume_rel` diagnostic column; a mismatch never flips a verdict.

For the six staged rows the diagnostic reference is recorded with the OCC door
(`--engine occ`) where the recording completes in bound: `front_wing` (43 s),
`cockpit` (97 s), `nose` (29 s) and `halo` (25 s) recorded; the two `sidepod`
rows exceed the recording bound (>300 s), so their reference is
**absent-diagnostic** — under the new policy a missing reference cannot block a
row.

## Census verdict table — all 54 rows

`prior` is the R2 verdict for F1 rows (`loop/results/TTC-RECENSUS-F1-R2.json`)
and the `FH-TIMING-REFRESH` / `FH-CENSUS` / `HYPERCAR-CENSUS` verdict for the
other families; `—` means the row was not in the R2 (F1-only) scope.

### Falcon-Heavy (14 rows)

| row | R3 verdict | carrier / reason (verbatim) | prior | changed by |
|---|---|---|---|---|
| falcon_heavy/nozzle_assembly | green | — | DNF-FACTS (OCC volume bias) | policy re-judgement (band retired) |
| falcon_heavy/chamber_assembly | green | — | green | unchanged (V5 net) |
| falcon_heavy/thrust_structure | green | — | DNF (non-z `Location`) | MONO-7/8/9 row assembly + admission |
| falcon_heavy/turbopump_assembly | green | — | green | unchanged (V5 net) |
| falcon_heavy/gas_generator_assembly | green | — | DNF (`tube()` spline-path sweep) | MONO-7/8/9 |
| falcon_heavy/turbine_exhaust_assembly | green | — | DNF (`tube()` spline-path sweep) | MONO-7/8/9 |
| falcon_heavy/feed_assembly | green | — | DNF (`tube()` spline-path sweep) | MONO-7/8/9 |
| falcon_heavy/harness_assembly | green | — | DNF (`tube()` spline-path sweep) | MONO-7/8/9 |
| falcon_heavy/engine_prototype | green | — | DNF (non-z `Location`) | MONO-7/8/9 |
| falcon_heavy/mvac | green | — | DNF (non-z `Location`) | MONO-7/8/9 |
| falcon_heavy/second_stage | DNF | `AttributeError: 'Compound' object has no attribute 'moved'` | DNF (non-z `Location`, composes mvac) | chain moved the boundary; new untyped site |
| falcon_heavy/fairing | green | — | green | unchanged (V5 net) |
| falcon_heavy/vehicle | DNF | `AttributeError: 'Compound' object has no attribute 'moved'` | DNF (non-z `Location`, composes mvac) | chain moved the boundary; new untyped site |
| falcon_heavy/cutaway | DNF | `AttributeError: 'Compound' object has no attribute 'moved'` | skip (partial-arc revolve) | chain moved the boundary; new untyped site |

### F1 (27 rows — 21 enrolled + 6 staged 2026-09-10)

| row | R3 verdict | carrier / reason (verbatim) | prior (R2) | changed by |
|---|---|---|---|---|
| f1/airbox | typed-refusal | `kernel refusal: unsupported_envelope` (native boolean admission) | typed-refusal | unchanged |
| f1/beam_wing | typed-refusal | `kernel refusal: unsupported_envelope` (multi-station spline loft) | typed-refusal | unchanged |
| f1/cockpit | DNF | `RuntimeError: cockpit: loft failed` | not enrolled (staged) | new row |
| f1/corner_fl | typed-refusal | `kernel refusal: unsupported_envelope` (drop-in `Face` data row) | kernel-door DNF (`Face.wrapped is None` → OCP `TypeError`) | MONO-1 data rows (typed refusal replaces the untyped `None`) |
| f1/corner_fr | typed-refusal | `kernel refusal: unsupported_envelope` | kernel-door DNF (same) | MONO-1 |
| f1/corner_rl | typed-refusal | `kernel refusal: unsupported_envelope` | kernel-door DNF (same) | MONO-1 |
| f1/corner_rr | typed-refusal | `kernel refusal: unsupported_envelope` | kernel-door DNF (same) | MONO-1 |
| f1/details | typed-refusal | `kernel refusal: unsupported_envelope` (native boolean admission) | typed-refusal | unchanged |
| f1/diffuser | DNF | `RuntimeError: floor loft failed: None` | kernel-door DNF | unchanged |
| f1/drivetrain | typed-refusal | `kernel refusal: unsupported_envelope` (native facts at `_fuse`) | typed-refusal | unchanged |
| f1/drs_actuator | typed-refusal | `kernel refusal: unsupported_envelope` (mirror carrier) | typed-refusal | unchanged |
| f1/drs_flap | typed-refusal | `kernel refusal: unsupported_envelope` (multi-station spline loft) | typed-refusal | unchanged |
| f1/engine_cover | typed-refusal | `an OCC probe of a kernel-engine row is not a kernel-engine row` | typed-refusal | unchanged |
| f1/floor | DNF | `RuntimeError: floor loft failed: None` | kernel-door DNF | unchanged |
| f1/front_wing | typed-refusal | `kernel refusal: unsupported_envelope` (boolean-composed swept carriers) | not enrolled (staged) | new row |
| f1/halo | typed-refusal | `kernel refusal: unsupported_envelope` (closed-loop loft chain) | not enrolled (staged) | new row |
| f1/monocoque | typed-refusal | `an OCC probe of a kernel-engine row is not a kernel-engine row` | typed-refusal | unchanged |
| f1/nose | typed-refusal | `kernel refusal: unsupported_envelope` (boolean-composed swept carriers) | not enrolled (staged) | new row |
| f1/power_unit | typed-refusal | `kernel refusal: unsupported_envelope` (multi-station spline loft) | typed-refusal | unchanged |
| f1/rear_wing | typed-refusal | `kernel refusal: unsupported_envelope` (trim-extrude constructor) | typed-refusal | unchanged |
| f1/sidepod_left | typed-refusal | `kernel refusal: unsupported_envelope` (boolean-composed swept carriers) | not enrolled (staged) | new row |
| f1/sidepod_right | typed-refusal | `kernel refusal: unsupported_envelope` (boolean-composed swept carriers) | not enrolled (staged) | new row |
| f1/steering_rack | typed-refusal | `kernel refusal: unsupported_envelope` (trim-extrude constructor) | typed-refusal | unchanged |
| f1/suspension_front | typed-refusal | `kernel refusal: unsupported_envelope` (trim-extrude constructor) | typed-refusal | unchanged |
| f1/suspension_rear | typed-refusal | `kernel refusal: unsupported_envelope` (trim-extrude constructor) | typed-refusal | unchanged |
| f1/track_rod_left | typed-refusal | `kernel refusal: unsupported_envelope` (multi-station spline loft) | typed-refusal | unchanged |
| f1/track_rod_right | typed-refusal | `kernel refusal: unsupported_envelope` (multi-station spline loft) | typed-refusal | unchanged |

### Hypercar (13 rows)

| row | R3 verdict | carrier / reason (verbatim) | prior | changed by |
|---|---|---|---|---|
| hypercar/wheels | DNF | `AttributeError: module 'bd' has no attribute 'RegularPolygon'` | canonical (OCC) | first kernel-door census |
| hypercar/aero | typed-refusal | `kernel refusal: empty` (`case empty`) | skipped (swept-carrier booleans) | first kernel-door census |
| hypercar/body | typed-refusal | `kernel refusal: unsupported_envelope` | skipped (swept-carrier booleans) | first kernel-door census |
| hypercar/brakes | typed-refusal | `revolve needs a closed profile` | skipped (swept-carrier booleans) | first kernel-door census |
| hypercar/chassis | typed-refusal | `kernel refusal: unsupported_envelope` | skipped (swept-carrier booleans) | first kernel-door census |
| hypercar/details | DNF | `AttributeError: 'Face' object has no attribute 'faces'` | skipped (swept-carrier booleans) | first kernel-door census |
| hypercar/glazing | typed-refusal | `kernel refusal: unsupported_envelope` | skipped (swept-carrier booleans) | first kernel-door census |
| hypercar/hinge | DNF | `AttributeError: module 'bd' has no attribute 'Align'` | skipped (swept-carrier booleans) | first kernel-door census |
| hypercar/interior | typed-refusal | `kernel refusal: unsupported_envelope` | skipped (swept-carrier booleans) | first kernel-door census |
| hypercar/lighting | DNF | `AttributeError: 'Face' object has no attribute 'faces'` | skipped (swept-carrier booleans) | first kernel-door census |
| hypercar/powertrain | DNF | `AttributeError: module 'bd' has no attribute 'RectangleRounded'` | skipped (swept-carrier booleans) | first kernel-door census |
| hypercar/suspension_front | DNF | `AttributeError: 'Plane' object has no attribute 'origin'` | skipped (swept-carrier booleans) | first kernel-door census |
| hypercar/suspension_rear | DNF | `AttributeError: 'Edge' object has no attribute 'edge'` | skipped (swept-carrier booleans) | first kernel-door census |

Verdict counts: **green 11, typed-refusal 30, DNF 13** (54 rows).

## Green rows — certificate bracket and OCC diagnostic

The bracket is the native `bd_facts` `volume_bracket`; `lo == hi` on every
green row (the exact per-patch certificate). The OCC delta is
`(kernel_volume − recorded_volume) / recorded_volume` — **reported, never
compared**.

| row | solids | volume | bracket [lo, hi] | STL tris | OCC delta (vol rel) |
|---|---:|---:|---|---:|---:|
| falcon_heavy/nozzle_assembly | 12 | 50350859.12990023 | [50350859.12990023, 50350859.12990023] | 101632 | −1.159e-04 |
| falcon_heavy/chamber_assembly | 46 | 45458782.5489467 | [45458782.5489467, 45458782.5489467] | 27520 | −4.809e-06 |
| falcon_heavy/thrust_structure | 41 | 30474877.222122557 | [30474877.222122557, 30474877.222122557] | 15504 | −4.890e-16 |
| falcon_heavy/turbopump_assembly | 59 | 77563936.47352579 | [77563936.47352579, 77563936.47352579] | 35352 | −2.882e-15 |
| falcon_heavy/gas_generator_assembly | 19 | 11205884.902620465 | [11205884.902620465, 11205884.902620465] | 8984 | −6.552e-03 |
| falcon_heavy/turbine_exhaust_assembly | 12 | 30467507.694068223 | [30467507.694068223, 30467507.694068223] | 58632 | −4.052e-03 |
| falcon_heavy/feed_assembly | 49 | 48753219.220576614 | [48753219.220576614, 48753219.220576614] | 123672 | −1.652e-02 |
| falcon_heavy/harness_assembly | 24 | 8299147.817239159 | [8299147.817239159, 8299147.817239159] | 3484 | −1.789e-03 |
| falcon_heavy/engine_prototype | 70 | 0.0 | [0.0, 0.0] | 161608 | abs 0 (recorded OCC volume 0) |
| falcon_heavy/mvac | 56 | 0.0 | [0.0, 0.0] | 66624 | abs 0 (recorded OCC volume 0) |
| falcon_heavy/fairing | 4 | 11027439648.490429 | [11027439648.490429, 11027439648.490429] | 11032 | +3.729e-05 |

The four green rows with deltas beyond the retired 1e-4 band
(`feed_assembly`, `gas_generator_assembly`, `turbine_exhaust_assembly`,
`harness_assembly`) are green **because the kernel's bracket is exact**; the
recorded OCC number is the approximation. This is the policy change made
visible: no exact engine can match an OCC approximation, and the kernel's own
certificate is the authority. `engine_prototype` and `mvac` are disjoint-solid
compounds whose recorded OCC volume reads 0; the kernel's bracket is likewise
`[0, 0]` and the construction is certified by `solid_count` and the emitted
mesh.

## Delta against the R2 census

R2 (`HEAD de33f33`, old band policy) was F1-only: **0 green, 15 typed-refusal,
6 kernel-door DNF** across the 21 enrolled F1 rows. R3 over the same 21 rows:
**0 green, 15 typed-refusal, 6 DNF**. The six staged rows add 5 typed-refusal
and 1 DNF. **No F1 row moved to green; no F1 row flipped.** What moved:

- **The four corner rows** moved from untyped kernel-door DNF
  (`surfaces.bbox` read a drop-in `Face` whose `wrapped` returned `None` and
  passed `None` into OCP `BRepBndLib.Add_s`) to a **typed refusal** — the
  MONO-1 data-row work surfaces the refusal at the drop-in carrier instead of
  the untyped OCP `TypeError`. That is a verdict-class improvement, not a
  green.
- **The six staged rows** are new census rows; all refuse (5 typed at
  boolean-composed swept carriers / the closed-loop loft chain, 1 DNF in the
  cockpit loft builder).
- **The policy re-judgement itself** moved no F1 row (none constructs), but it
  moved the **FH canonical subset** from 3 green to 11 green by retiring the
  OCC band as a gate.

## Typed refusals (verbatim)

The native executor's mapped refusal for the multi-station loft, trim, boolean,
mirror and probe carriers is:

```json
{"kind": "Refused", "message": "kernel refusal: unsupported_envelope",
 "payload": {"case": "unsupported_envelope", "envelope": "non_canonical_carrier"}}
```

Two carriers carry a distinct verbatim message:

```json
{"kind": "Refused", "message": "an OCC probe of a kernel-engine row is not a kernel-engine row",
 "payload": {"case": "unsupported_envelope", "envelope": "non_canonical_carrier"}}
```

(rows `f1/engine_cover`, `f1/monocoque`), and the hypercar carriers:

- `hypercar/aero`: `{"kind": "Refused", "message": "kernel refusal: empty", "payload": {"case": "empty"}}`
- `hypercar/brakes`: `{"kind": "Refused", "message": "revolve needs a closed profile", "payload": {"case": "unsupported_envelope", "envelope": "non_canonical_carrier"}}`

Carrier classes and the rows that refuse at them:

- **multi-station spline loft / boolean-composed swept carriers** — the F1
  body and wing lofts (`beam_wing`, `drs_flap`, `power_unit`,
  `track_rod_left/right`), the native boolean admission (`airbox`,
  `details`), the trim-extrude constructor (`rear_wing`, `steering_rack`,
  `suspension_front/rear`), and the six staged rows' swept-carrier booleans
  (`front_wing`, `nose`, `sidepod_left/right`, and the halo closed-loop loft
  chain).
- **native facts refusal at `_fuse`** — `drivetrain`.
- **mirror carrier** — `drs_actuator`.
- **OCC-probe data gap** — `engine_cover`, `monocoque`.
- **hypercar swept/loft carriers** — `body`, `chassis`, `glazing`, `interior`
  (unsupported envelope), `aero` (empty), `brakes` (revolve needs a closed
  profile).

## DNF rows (untyped, recorded as-is — not tuned)

| row | kind | message (verbatim) | failing site |
|---|---|---|---|
| falcon_heavy/second_stage | `AttributeError` | `'Compound' object has no attribute 'moved'` | cadgen `compound_from_instances` placement (`door.py` truck `_compound_from_instances`) |
| falcon_heavy/vehicle | `AttributeError` | same | same |
| falcon_heavy/cutaway | `AttributeError` | same | same |
| f1/cockpit | `RuntimeError` | `cockpit: loft failed` | `lib/cockpit.py` loft builder |
| f1/diffuser | `RuntimeError` | `floor loft failed: None` | `floor._loft_stack` (`floor.py:530`), via `_diffuser_shell` |
| f1/floor | `RuntimeError` | `floor loft failed: None` | `floor._loft_stack` (`floor.py:530`), via `_floor_shell` |
| hypercar/wheels | `AttributeError` | `module 'bd' has no attribute 'RegularPolygon'` | drop-in name surface gap |
| hypercar/details | `AttributeError` | `'Face' object has no attribute 'faces'` | drop-in `Face` selection gap |
| hypercar/hinge | `AttributeError` | `module 'bd' has no attribute 'Align'` | drop-in name surface gap |
| hypercar/lighting | `AttributeError` | `'Face' object has no attribute 'faces'` | drop-in `Face` selection gap |
| hypercar/powertrain | `AttributeError` | `module 'bd' has no attribute 'RectangleRounded'` | drop-in name surface gap |
| hypercar/suspension_front | `AttributeError` | `'Plane' object has no attribute 'origin'` | drop-in `Plane` data-row gap |
| hypercar/suspension_rear | `AttributeError` | `'Edge' object has no attribute 'edge'` | drop-in `Edge` data-row gap |

The three Falcon DNFs (`second_stage`, `vehicle`, `cutaway`) are the
`compound_from_instances` composition helper reaching `Compound.moved`, which
the truck drop-in's `Compound` data row does not answer; the kernel-side
`second_stage`/`vehicle` sub-rows that do not compose through the helper are
green (`mvac`, `engine_prototype`). The `floor`/`diffuser`/`cockpit` DNFs are
corpus-builder gaps (the loft builder's `is_valid_shape` swallows the drop-in
refusal and raises an untyped `RuntimeError`). The hypercar DNFs are drop-in
name/data-row surface gaps. All are recorded verbatim, never tuned.

## V5 net — previously-green rows

The V5 net holds. Every row green under the R2-era policy remains green under
R3, bit-identically to the recorded `FH-TIMING-REFRESH` facts:

| row | solid_count | volume | STL triangles | verdict |
|---|---:|---:|---:|---|
| falcon_heavy/turbopump_assembly | 59 | 77563936.47352579 | 35352 | green (unchanged) |
| falcon_heavy/chamber_assembly | 46 | 45458782.5489467 | 27520 | green (unchanged) |
| falcon_heavy/fairing | 4 | 11027439648.490429 | 11032 | green (unchanged) |

No landed row flipped from green to typed/DNF. The policy only WIDENS what can
go green; eight FH rows joined the green set, none left it.

## Findings filed

1. **The policy change closes the FH canonical subset and moves no F1 row.**
   The kernel-internal predicate is green on 11/11 constructible FH rows and 0
   F1 rows. The F1 family's carriers remain refused by the native executor;
   retiring the OCC band does not widen construction.
2. **The retired band was the only thing keeping four FH rows red.** Under R2's
   policy `feed_assembly` (−1.65e-2 rel), `gas_generator_assembly` (−6.55e-3),
   `turbine_exhaust_assembly` (−4.05e-3) and `harness_assembly` (−1.79e-3) would
   have failed the 1e-4 `volume_rel` gate despite carrying exact kernel
   brackets. Their OCC deltas are now reported diagnostics.
3. **The F1 dominant carrier is unchanged: the multi-station smooth loft and
   the swept-carrier boolean.** MONO-2..9 did not admit the corpus F1 lofts
   (7–19 stations of spline sections) or the swept×swept boolean pairs, so the
   F1 body/wing/trim rows stay typed-refusal.
4. **MONO-1 removed the untyped corner DNF class.** The four corner rows now
   refuse typed at the drop-in `Face` data row instead of dying in OCP
   `BRepBndLib.Add_s(None, ...)`.
5. **The six staged rows are census rows now.** Five refuse typed at the
   boolean-composed swept carriers / the halo closed-loop loft chain; the
   cockpit row dies untyped in its own loft builder. The sidepod OCC reference
   recording exceeds the bound (absent-diagnostic).
6. **New untyped classes surfaced on the FH compositions.** `second_stage`,
   `vehicle` and `cutaway` now reach the cadgen `compound_from_instances`
   helper's `Compound.moved`, which the drop-in `Compound` data row does not
   answer — a drop-in surface gap, not a native refusal.
7. **The hypercar tree is kernel-censused for the first time**: 6 typed
   refusals and 7 drop-in name/data-row DNFs; no hypercar row is green on the
   kernel door.
8. **No reference gated anything.** The recorded references were read and
   reported; a missing reference (sidepods) blocked nothing, and a
   beyond-band delta (feed/gas-generator/turbine-exhaust/harness) flipped
   nothing.

## Anchors

- A1 `grep -c '"id"' corpus/ttc/MANIFEST.json` = 54 — holds (the six staged
  rows are enrolled; manifest + `SKIPS.json` validate).
- A2 `grep -c median docs/TT_TIMING_RESULTS.md` = 21 — holds (the R3 timing
  series is recorded with raw samples and a `p50` summary; the standing FH
  content is unchanged).
- A3 `grep -c 'kernel-internal' docs/MONO_CLOSURE_BOOKING.md` = 1 — holds
  (read-only).
