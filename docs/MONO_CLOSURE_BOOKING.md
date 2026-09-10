# MONO-CLOSURE — program booking (2026-09-10)

Owner directive: close the gap on the machinery that generates the monocoque.
The row-closure composition (row x carrier x status) is the booking evidence;
this document is the program's spine.

## The row-closure facts (measured)

Source-level chain of `f1/monocoque` (audited this session, `mono_tub.py` +
`mono_halo.py` via `surfaces.py`):

```
build_monocoque
  -> mono_tub.build_tub_bodies()
       _tub_solid:        42-station closed-spline-section loft (body_loft)
                          -> surfaces.obox probe (mono_tub.py:526)
                          -> surfaces.cut(skin, _cavity())   [cut(swept,swept)]
                          -> surfaces.is_valid_shape probe
       _rim_bead:         swept_plate member along a path
       _roll_hoop/_stay:  blade members
       _headrest_shoulder/_blister: two more N-station lofts
       _fuse_proud:       fuse(swept,swept) of boss fairings onto the shell
       mirror_y of stays/rack boss
  -> surfaces.group(...)
```

Static op sweep over the F1 family (scratch/row_closure_sweep_v0.py, 15 lib
files): loft 104, sections-author 95, sweep/blade 38, cut/fuse 20, mirror 16,
probes 14, extrude 16, spline-author 16. The trim refusals reach the
constructor through composition (zero `trim(` calls) — identification is a
packet stop-condition (MONO-4), not an assumption.

## Packet graph (this program)

| packet | closes | write set | depends |
|---|---|---|---|
| MONO-1-DATA-ROWS | probe deaths: monocoque obox, engine_cover bbox, corner Face bbox | binding.rs, marshal.rs | - |
| MONO-2-NSTATION-LOFT | beam_wing, drs_flap, power_unit, track_rod x2, floor, diffuser; monocoque's 3 lofts | bd_bridge.rs | - |
| MONO-3-BLADE-MEMBERS-MIRROR | members + mirror_y of kernel rows | bd_bridge.rs | MONO-2 |
| MONO-4-TRIM-IDIOMS | rear_wing, steering_rack, suspension x2 | bd_bridge.rs | MONO-2 |
| (wave 3, theory-gated) MONO-5-SWEPT-BOOLEANS | airbox, details, drivetrain, engine_cover deep, monocoque cut/fuse | bd_bridge.rs | MONO-2/3 + the frontier theory |
| (after MONO-5) MONO-6-ROW-ASSEMBLY | the monocoque row itself end to end | bd_bridge.rs | MONO-1..5 |

Dispatch: MONO-1 and MONO-2 parallel (disjoint write sets); MONO-3/4 serial
after MONO-2 (shared bd_bridge.rs). Expected post-wave-2 census state: 5 rows
green with kernel timing columns; all six R2 DNFs reclassified.

## The wave-3 gate

MONO-5 is booked ONLY after the frontier review returns on
`docs/MONO_WAVE3_THEORY_BRIEF.md` (certified intersection enclosure +
split-boundary volume accounting). Everything else in this program is
mechanical wiring riding landed theory.

## Annex A — the pinned OCCT ThruSections convention (measured 2026-09-10)

Probes: `scratch/probe_thrussections_convention.py`,
`probe_thrussections_law.py`, `probe_thrussections_recover.py` (synthetic
closed sections, non-uniform station spacing, OCC BRepOffsetAPI_ThruSections,
smooth and ruled). Findings, each reproduced across N in {4,5,6,8,9,10,16,24}:

- **A1 (faces).** Smooth loft of closed sections = ONE B-spline face + 2
  planar caps (no seam splitting). Ruled = per-interval faces.
- **A2 (section hit).** Sections are hit EXACTLY: the u-unification is
  geometry-preserving knot insertion, not tolerance approximation. Measured
  section-hit deviation is at the sampling floor (1e-1 mm on 600 mm sections
  with 300-sample isocurve polylines; the corpus's own 200 mm "wandering
  loft" incident applies only to OCC's degenerate fallback paths, not to
  transversal well-conditioned stacks).
- **A3 (station law).** Station parameter v_i = CHORD-LENGTH on station
  positions (cumulative centroid distance, normalized). Law-fit residuals
  2.7e-5..5.3e-5 vs 4e-2..6e-2 for uniform/centripetal/sqrt-chord. Confirmed
  with CheckCompatibility(True) (the build123d default) and (False).
- **A4 (v-degree law).** N <= 9: a single global polynomial of degree N-1 in
  v (v-knots [0,1] only). N >= 10: C2 cubic spline in v with distinct v-knots
  AT the chord-length station parameters (knot-vs-station max difference
  2.8e-5..3.9e-5 = the recovery resolution of this probe).
- **A5 (u-degree).** 3, unconditionally.
- **A6 (ruled).** v-degree 1, uniform per interval, per-interval faces.

Corollary for the kernel: the smooth N-station loft is a deterministic
function of the section curves alone (given the law above); certification is
per-patch through the landed `volume_facts` binding; OCC never runs at
certify time. The corpus's recorded references remain the end-to-end oracle;
the one-time synthetic fixtures (MONO-2 step 7) prove convention fidelity.

## Annex B — the row-closure machinery (booking gate)

`loop/row_closure.py` (to be landed with this program): per-row op inventory
(scratch/row_closure_sweep_v0.py v0) joined against the capability matrix and
packet graph; program booking is valid iff the union of its packets covers
every gap cell of its target rows; `door --trace-carriers` (soft-refusal
execution, logging the real carrier sequence) is the ground-truth instrument
the static sweep defers to. Both are loop-side; neither is a packet.
