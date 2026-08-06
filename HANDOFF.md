# Handoff — recovering the most faces from here

**State.** look on `integration/formal-atlas-wave-2`, pinned to truck
`3a81a169` on `feature/deck-join-and-seeds`. Both pushed. Path override
re-commented and the pinned build verified to reproduce the override build's
census exactly.

**Session 1 of the three-session plan is done. WAVE-4A + WAVE-4B recovered
+2,186 faces with zero regressions**, and the PROJ-001 ride-along was
collected. Read `docs/WAVE_4A_WINDING_PARITY.md` and
`docs/WAVE_4B_ANALYTIC_INVERSE.md`; `docs/WAVE_3B_PLANAR_ARC_DEVELOPMENT.md`
is still the required reading before any planar work.

**825,745 of 839,179 faces render (98.40%). 13,434 are lost.**

**Session 2 is package C, Step 8A-arc, unchanged and still specified below.**
Session 3's shape has changed — PROJ-001 landed on an outcome §4's decision
table did not have a row for, and the cheap next step on splines is smaller
than a session. See "What session 3 is now".

---

## What session 1 landed

| | faces | regressions | gate |
|---|---:|---:|---|
| **A** winding parity (§2) | **+1,404** | 0 | `TRUCK_FORMAL_RECOVERY_PARITY` |
| **B** analytic inverse (§1) | **+782** | 0 | `TRUCK_FORMAL_RECOVERY_ANALYTIC` |

Each measured as its own subtraction with the other left on, per the
discipline. Zero `rendered -> lost` **and** zero triangle-count changes on any
already-rendered face, on all 20 models.

**Both packages' hypotheses were about *what to compute*. Both fixes turned out
to be about *where the retry fires*, and in both cases the wrong placement
looked clean on rendered/lost.**

- A's retry run where the contradiction is detected recovers the same faces but
  pre-empts the torus annulus route on 8 of them, replacing a validated
  64-triangle mesh with a 1–2 triangle remnant. Only the **triangle count**
  caught it. It now runs after every other recovery route.
- B's fallback admitted on the hinted call pre-empts the better answer the
  hintless call finds — one cone face went `rendered -> lost`. It is now
  restricted to the hintless call, which is last in the projection chain.

**Diff triangle counts per `source_face_id`, not just rendered/lost.**
`p1-out/parityA/tab.py` does both and is the gate to use for any route that
changes how material is selected or how a parameter is chosen.

### What was falsified

**The handoff's one-line parity hypothesis is dead, measured.**
`toggles_material` returning `Some(true)` for `UnresolvedSyntheticClosure` is
not the cause of `ContradictoryDualParity`. Read as non-toggling per §X
Definition 20, all 126 contradicting faces on `00009190` still contradict and
the obstruction count *rises*. The legacy answer stays, with the measurement
recorded beside it. Do not re-litigate this.

The real cause was found by asking a better question: **a parity flood is
consistent iff every vertex has an even number of incident toggling constraint
edges.** That turns a contradiction into a located, countable obstruction, and
it is what separates "some role's material reading is wrong" from "the
constraint set is not a closed boundary at all". `ConstraintRoles::roles` is a
*set*, so a second boundary segment realizing onto the same CDT edge leaves no
trace, and mod 2 a twice-traversed edge separates nothing. 126 of 126
contradicting faces have a repeated traversal; 0 of 23,258 clean ones do.

---

## Read this before you plan anything

**Measure the corpus, not one model.** Two editions of this file planned from a
single model and named a target that was mostly already solved.

**Measure with the routes on.** Everything below is default-on.

**Use production's entry point.** `face_census` must call
`robust_triangulation_with_torus_outcome`. A caller that stops at the cone form
has the torus route compiled in but unable to fire, and it fails *silently*.

**Join on `source_face_id`.** `declared_face_index` resets per shell and
collides. `p1-out/reconcile.py` and `p1-out/parityA/tab.py` do it correctly.

**A refusal reached at stage N says nothing about stage N+1.** WAVE-4B is the
sharpest instance yet: it took cylinder `BoundaryProjectionFailed` from 1,072
to **0** and torus from 939 to **7**, and recovered 782 faces, because most of
that population is now blocked at constraint insertion instead. **"In scope" is
not "recovered"** — treat every cell count below as a ceiling.

**A defaulted trait method that nothing forwards reads exactly like a route
that does not help.** Third occurrence: `search_parameter_seeds` is defaulted
empty on `SearchParameter` and `Processor` does not override it, so a
Processor-wrapped cylinder or torus is offered **zero** seeds. That is why
WAVE-3A's seeding never moved the analytic families.

**A full-model panic looks like an empty output file.** `00005641` aborted
mid-sweep and produced a zero-byte census that nothing flagged; the
`.meta.json` sentinel is what caught it. Check for it.

**Build first, then hypothesise.** Every wave in this file's history has had a
target corrected by measurement, and WAVE-4 had two.

---

## The residual

13,434 faces. Freshly measured at the current pin, default-on
(`p1-out/diag-w4/`, `p1-out/w4_tab.py`).

| terminal reason | faces | share | was |
|---|---:|---:|---:|
| `ConstraintInsertionIncomplete` | 7,023 | 52.3% | 5,988 |
| `BoundaryProjectionFailed` | 3,703 | 27.6% | 5,825 |
| `AmbiguousLift` | 1,239 | 9.2% | 1,492 |
| `ConstraintOverlapUnsupported` | 977 | 7.3% | 427 |
| `NoOddParityRegion` | 342 | 2.5% | 338 |
| `BoundaryConstructionFailed` | 127 | 0.9% | 127 |
| `ContradictoryDualParity` | 22 | 0.2% | **1,422** |
| `ConstraintRoleMissing` | 1 | 0.0% | 1 |

The `was` column is not a like-for-like loss table: CII and Overlap *grew*
because WAVE-4B pushed ~1,400 analytic faces past projection into the stage
that blocks them next. Those are not new defects, they are the same faces
further along.

```
family          CII   Proj   Lift   Ovlp  NoMat    Bnd Parity   Role   total
Plane          2908                  104    200                         3212
Nurbs           458   2426             9      1             1           2895
Cylinder       1480           107    651                    4           2242
Bspline         405   1156            61    137                    1    1760
Cone            312           747    136                                1195
Torus           970      7                                               977
Sphere          150           371                          17            538
Extruded        141     12      2      4                                 159
Unknown                                            127                   127
Offset            8     27                                                35
Revolved         22      5      2      4                                  33
```

Still concentrated: three cells hold 52%, ten hold 88%.

`ConstraintInsertionIncomplete`, by bucket — and note the shape has changed
again, `SourceSourceSameBoundCrossing` having nearly doubled:

```
2,544  SourceSourceInterBoundCrossing
2,266  SourceSourceSameBoundCrossing
2,012  SourceSyntheticCrossing
   27  MixedConstraintConflict
    4  SyntheticSyntheticCrossing
```

rank 0: 3,917; periodic: 2,937.

**The `Plane` row is still not 2,908 crossings.** WAVE-3B showed the crossings
are chord-approximation artefacts; the cell is real loss but its mechanism is
the arc approximation.

---

## Session 2 — package C, Step 8A-arc

**Unchanged by WAVE-4, and still the right next build.** Its machinery is in
place, its mechanism is the best understood of anything remaining, and it is a
full session on its own. `docs/WAVE_3B_PLANAR_ARC_DEVELOPMENT.md` has the
evidence; the build spec:

A certified polygonal approximation of an arc within the caller's tolerance,
feeding the existing ear clipping and the **unchanged** Step 8B battery.
`certified_polygonal_region`'s `approximation_is_exact` guard was written for
exactly this arrival. For an arc of radius `r` split into `n` equal sub-arcs
over sweep `|t1 − t0|`, the chord's maximum deviation is the sagitta

```
e(n) = r * (1 − cos(|t1 − t0| / (2n)))
```

so the smallest admissible `n` is closed form in the caller's `tol`. The error
is *bounded*, not zero — carry the bound into
`Rank0DevelopedBoundary::approximation_is_exact`'s successor rather than
asserting exactness.

**Build it in `planar_holes`, not only in `planar_slice`.** Of the 1,163
arc-ready faces:

```
  1,065  delegated to the holes slice   <- multi-bound
     64  only the arc family (the hole-free slice alone unblocks it)
     34  blocked on outer-bound standing (29 of them declare one bound)
```

Landed in the hole-free slice alone it is worth **64 faces**, not 1,163. The
shared piece is the only new piece: once an arc is polygonalized within a
certified bound, `classify_components`, `point_strictly_inside` and
`certify_region_with_holes` all consume cycles of `Point2` unchanged. Build the
polygonalization once against `DevelopedCurve2D` and feed both slices.

Gate it `TRUCK_FORMAL_RECOVERY_ARC`, default-on, nested under the master gate,
and enter it only where the legacy path produced no mesh — **and then check the
placement against the other routes the way WAVE-4A had to.** Two obligations
the developed track does not discharge: per-curve simplicity
(`IndividualCurveNotSimple` is unproved; for a circular arc it is a one-liner,
but it must be *asked*) and material selection, which needs outer-bound
standing.

**Ship the outer-bound derivation with it.** 431 single-bound faces exit
`missing_outer_bound_authority` while declaring exactly one bound; a lone bound
is the face's boundary, derivable with no geometry, and must be labelled
*derived* rather than source-declared. The cause is a real source class:
`00007705` is 91% plain `FACE_SURFACE`, which legitimately carries `FACE_BOUND`
without `FACE_OUTER_BOUND` — 25,330 against 2,213.

**Collect the ride-along while you are in the planar funnel:** the
`no_developable_curve` histogram. 1,956 planar lost faces exit at Step 2 and
nothing is known about them beyond the count. `SliceRecord` carries curve
representations even on refusal, so this is a tabulation over `curves=`, not
new instrumentation — one command with `p1-out/slice_tab.py` adapted. It sizes
the spline-on-planar-boundary work and will very likely merge with §4.

---

## What session 3 is now

PROJ-001 was collected (`docs/WAVE_4B_ANALYTIC_INVERSE.md`, `p1-out/proj_tab.py`,
`p1-out/projB/*.proj.txt`). It landed on an outcome §4's decision table did not
have a row for, and it makes the next spline step **smaller than a session**:

- **Every failing point reaches the last chain link.** Not one is lost earlier,
  in any family. Rows 1 and 4 of §4's table are out.
- **1,053 spline faces are offered exactly one seed** (NURBS 487, B-spline
  566), where the route fires and does nothing the plain call did. Only 1,565
  got a genuine multi-seed attempt. **The 3,652-face §4 target is not one
  population.**
- **The residual column is unanswerable as instrumented, not answered.**
  `by_structural_seeds` scores seeds with `search_parameter`, which is
  all-or-nothing and returns `None` unless the point is already within
  tolerance — so a *failing* point yields no residual at all. Re-ask with
  `search_nearest_parameter` from each seed. **Do not read the empty column as
  "residuals are large."**

**So do these two cheap things before committing a session to splines:** widen
the seed source past one-per-knot-span, and re-ask the residual with
`search_nearest_parameter`. Between them they decide whether §4 is an
initialisation problem, a tolerance problem, or neither — which is what the
ride-along was supposed to buy, and it still buys it, just one step further
back than expected.

Where the rest sits:

| block | faces | state |
|---|---:|---|
| **plane CII** (§6) | 2,908 | session 2's target via Step 8A-arc; crossings are chord artefacts |
| **spline projection** (§4) | 3,582 | two cheap experiments above decide the shape |
| **`SourceSyntheticCrossing`** (§5) | 2,012 | still the largest class with **no** hypothesis; DIAG-001 witnesses already name both segments and their origins. A morning of reading, not instrumentation |
| **cylinder CII + overlap** | 2,131 | grew under WAVE-4B — these are analytic faces that now project and block at insertion. **Newly worth a look**, and the diagnosis is fresh |
| **cone + sphere lift** (§3) | 1,118 | self-contained; `AMBIGUOUS_STEP_FRACTION` bisection exhausting. `00005427` is 494 of them, and is 723 lost of which cone lift is the bulk |
| **torus CII** | 970 | was 257; same story as cylinder — WAVE-4B's faces arriving at the next stage |
| remainder | ~700 | long tail |

**`ConstraintOverlapUnsupported` at 977 (was 427) deserves a first look**, since
651 of it is cylinder and it is now the fourth-largest reason. It is a typed
refusal with a one-line meaning — a boundary traversing an edge it already
constrained — and WAVE-4A's traversal counter is *exactly* the instrument for
asking whether those are duplicate bounds, slits, or something else. That is
the cheapest unexamined thing on this list.

---

## Where the loss lives

| model | residual | biggest cell |
|---|---:|---|
| `00007705` | 2,074 | 894 plane CII |
| `00000414` | 1,733 | 1,175 NURBS projection |
| `00003172` | 1,310 | 256 B-spline projection |
| `00001075` | 1,232 | 363 NURBS projection |
| `00005760` | 1,129 | 515 plane CII |
| `00000730` | 1,048 | 387 cylinder overlap |
| `00003902` | 792 | 712 torus CII |
| `00009190` | 771 | 205 plane CII |
| `00005427` | 723 | 494 cone lift |

`00000414` is 68% NURBS projection on its own and was untouched by both WAVE-4
packages. `00009190` is still the standard planar workbench. `00005427` is
package 3 nearly on its own.

---

## Regression discipline

Any new route must:

- be **refinement-only** — entered only where the legacy path produced no mesh.
  **And check that against the whole pipeline, not one function**: WAVE-4A was
  refinement-only inside `trimming_tessellation` and still pre-empted a
  downstream route;
- ship behind `TRUCK_FORMAL_RECOVERY_<ROUTE>`, default-on, disabled by an
  explicit `0`/`off`/`false`/`no`, nested under the master gate;
- keep the declared population fixed and be reconciled per `source_face_id`
  against the previous pin on all 20 models, **including triangle counts**;
- validate before replacing: constraint completeness, boundary preservation,
  seam-pair consistency, connectedness, Euler characteristic,
  boundary-component count, world-space tessellation tolerance.

Traps that have each cost a session:

- **Never batch benchmark configs.** Alternate, and take the minimum of ≥5 reps.
- **A fresh exe timestamp is not a fresh exe.** Wait for the build's own
  completion, then verify by behaviour. A build that collides with a running
  sweep fails with `Access is denied` rather than swapping the binary — that is
  the good case; do not work around it.
- **`.cargo/config.toml`'s path override is invisible in `Cargo.lock`.**
  Re-comment it, bump the rev, and confirm the pinned build reproduces the
  number before writing it down.
- **Check free disk before timing anything.** It sat at **11 GB free** through
  WAVE-4.
- **Probe stderr interleaves.** Compare *sorted*, keyed on `source_face_id`.
- **Do not run two corpus sweeps concurrently.** WAVE-4 did, and the contention
  is what turned `00005641`'s panic into a silent zero-byte file.

---

## Artifacts

Outside git, in `C:\Users\stefa\look-corpus\p1-out\`:

- `diag-w4/<id>.jsonl` — **the current residual's DIAG-001 records**, per model,
  default-on at the pin. This is the file to start from. `diag-w3a/` is the
  previous pin, kept for diffing.
- `w4_tab.py` — every residual table in this file
- `parityA/`, `projB/` — WAVE-4's `{off,on}` ledger pairs and `parityA/tab.py`,
  the reconciliation gate that checks triangle counts as well as rendered/lost
- `proj_tab.py`, `projB/*.proj.txt` — PROJ-001, the spline/analytic projection
  diagnostic
- `parity-sweep.sh`, `analytic-sweep.sh`, `diag-w4-sweep.sh` — WAVE-4's sweeps
- `dev-arc/`, `dev_tab.py`, `slice_tab.py`, `yield_tab.py` — WAVE-3B's
  developed-arc survey and the tables sizing Step 8A-arc. **Re-run `yield_tab.py`
  after any planar change.**
- `reconcile.py` — the per-`source_face_id` regression gate

Build (there is no MSVC toolchain on this machine, only LLVM-MinGW):

```
cargo +stable-x86_64-pc-windows-gnullvm build --release \
      --target x86_64-pc-windows-gnullvm --example face_census
```

`~/.cargo/git` (3.1 GB) and `~/fastbrep/target` are headroom if a link fails
with `LLVM ERROR: No space left on device`, which is how disk exhaustion has
always presented.

### Probes

| variable | what it emits |
|---|---|
| `TRUCK_FACE_DIAG_JSONL=<path>` | DIAG-001, one JSON row per lost face |
| `TRUCK_PROBE_PARITY` | `PARITY`: repeated traversals and the odd-vertex obstruction count under both readings, one line per face reaching the flood |
| `TRUCK_PROBE_PROJ` | `PROJ`: per failing face, failed points and ratio, which chain link was reached, seeds offered, best residual. **Does not stop the boundary walk at the first failure** |
| `TRUCK_PROBE_SLICE` | `SLICE` / `HOLES`: the rank-0 funnel, one line per face |
| `TRUCK_PROBE_DEVELOPED` | `DEV`: the developed-arc survey, one line per bound |
| `TRUCK_CENSUS_TOL_FACTOR=<f>` | scales the census tolerance |
| `TRUCK_FORMAL_RECOVERY=0` | master kill switch; each route also has `_<ROUTE>=0` |

Routes, all default-on and nested under the master gate: `_BAND`, `_TORUS`,
`_HOLES`, `_CYLINDER`, `_DECK_JOIN`, `_SEED`, `_PARITY` (WAVE-4A), `_ANALYTIC`
(WAVE-4B).

`TRUCK_PROBE_DEVELOPED` costs an O(pieces²) certified pass per bound and is off
by default for that reason.
