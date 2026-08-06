# Handoff — recovering the most faces from here

**State.** look `2e40130` on `integration/formal-atlas-wave-2`, pinned to truck
`d9661022` on `feature/deck-join-and-seeds`. Both pushed. Path override
disabled and the pinned build verified to reproduce the override build's census
exactly.

**WAVE-3B changed no geometry and recovered no faces.** It built the seam that
develops a planar boundary arc analytically, and used it to falsify package 6's
premise below. Read `docs/WAVE_3B_PLANAR_ARC_DEVELOPMENT.md` before planning any
planar work: **Step 8A-arc, not the arrangement, is what recovers this cell.**

**823,559 of 839,179 faces render (98.14%). 15,620 are lost.** WAVE-3A added
+3,790 (`docs/WAVE_3A_DECK_JOIN_AND_SEED.md`); `docs/WAVE_2C_GATE_GRADUATION.md`
covers the wave before it.

**Two of the twenty models now render every face** — `00005642` (177,285) and
`00009272` (146,520), the two largest in the corpus. `00009972` is 8 faces from
a third. The other 17 range from 0.17% to 11.79% loss.

This document is about the 15,620, ordered by **how many faces you get for how
much work**. Every number is freshly measured at the current pin, default-on.

**Three sessions are planned out below**: A+B now, C next, and a third that the
first two are asked to pre-size as they pass. Read "The plan" and then the
package sections it names — the rest is reference.

---

## The plan — three sessions

**This session: A and B.** Next session: C. The one after that is sized by two
diagnostics that A/B and C are asked to collect on their way past, so it does
not open with a survey.

| session | package | faces in scope | shape |
|---|---|---:|---|
| **1 — now** | **A** cylinder parity (§2) | 1,422 | one line |
| **1 — now** | **B** analytic projection (§1) | 2,127 | measure, then a small fix |
| 2 | **C** Step 8A-arc (§6) | ~1,163 | a real build |
| 3 | the remaining ~10,900 | — | pre-sized by A/B's and C's ride-along diagnostics |

Everything in session 1 needs **no new theory and no new machinery**. Do A
first: it is an afternoon at most and settles a question deferred three waves
running. Then B, which is the largest single mechanism in the residual and whose
first step is a measurement, not a build — if the residuals come back small, the
fix is a few lines in `Processor::search_nearest_parameter`.

**Do not start C this session.** Its machinery is in place and its mechanism is
the best understood of the three, but it is a full session on its own and
splitting it across two is how the arc work gets half-landed. §6 has the build
spec ready.

### Measure A and B separately, always

Each route ships behind its own `TRUCK_FORMAL_RECOVERY_<NAME>` gate, default-on,
**so its own contribution stays one subtraction**. Landing both and taking one
census gives one number that cannot be attributed. This project has already lost
time to exactly that (the cylinder-band wave, where one gate covered two routes).
Run the corpus with each route's gate at `0` in turn and diff on
`source_face_id`.

For A specifically: watch **triangles per face**, not just rendered/lost. The
ledger carries `triangles=` for this. Flipping `toggles_material` changes
material state for *every* face carrying synthetic segments, and a face that
starts rendering with a wildly different triangle count is not a recovery.

### Two diagnostics to collect while you are already there

Both are close to free *if done in the session that is already in that code*,
and expensive as their own survey later. They are what makes session 3 a build
instead of a week of measurement.

**In session 1, during B — the spline projection diagnostics.** B puts you
inside the boundary projection path, which is exactly where package 4 (§4,
splines, **3,652 faces — the largest post-C block**) is decided. §4 has the
full four-field spec; the short version is: failed points *per face and as a
ratio*, which link of the five-step chain failed, how many seeds were actually
offered, and the best seed's residual.

Do not shorten it to the bare failing-point count. Each of the other three
guards against a specific wrong reading — a route that never ran, a route that
offered one seed and so did nothing, and residuals that cluster just above
tolerance and mean the class belongs to §1 rather than §4 at all. **This is the
number that decides what session 3 is**, and the epistemics are currently good
enough on §4 that a sloppy version would be worse than none.

**In session 2, during C — the `no_developable_curve` histogram.** C puts you in
the planar funnel. 1,956 planar lost faces exit there, the largest remaining
planar block, and *nothing* is known about them beyond the count. The
`SliceRecord` now carries curve representations even on refusal (WAVE-3B), so
this is a tabulation over `curves=`, not new instrumentation — one command with
`p1-out/slice_tab.py` adapted. It sizes the spline-on-planar-boundary work and
will very likely merge with §4.

Neither is a detour worth taking on its own. Both are ten minutes when you are
already in the file.

### Your first twenty minutes

```bash
# Build (LLVM-MinGW only; there is no MSVC toolchain on this machine).
cd ~/look && cargo +stable-x86_64-pc-windows-gnullvm build --release \
      --target x86_64-pc-windows-gnullvm --example face_census

# One model, with whichever probes the package needs.
cd ~/look-corpus
STEP=$(ls abc/00009190/*.step | head -1)
TRUCK_PROBE_SLICE=1 TRUCK_PROBE_DEVELOPED=1 \
  target/.../face_census.exe "$STEP" > census.txt 2> probe.txt
```

The binary at `~/look/target/x86_64-pc-windows-gnullvm/release/examples/face_census.exe`
is built at this pin and works — the other target trees were deleted to reclaim
disk, so `cargo test`/`cargo check` will rebuild cold once (~1–6 min).

Tables and joins: `p1-out/slice_tab.py` (the `SliceExit` funnel against the lost
faces), `p1-out/dev_tab.py` (the developed-arc verdict), `p1-out/reconcile.py`
(the per-`source_face_id` regression gate — **use this before claiming a
delta**).

### What "recovering a face" requires

A face renders only when *every* stage clears, so a package that clears one
stage recovers nothing on a face blocked at another. This bit WAVE-3B: the
planar funnel showed two sequential gates and the second was invisible until the
first moved. Before scoping any planar package, cross the funnel against the
stage you are fixing rather than assuming the population is the cell count.

---

## Read this before you plan anything

**Measure the corpus, not one model.** Two editions of this file planned from a
single model and named a target that was mostly already solved. The corpus and
the benchmark geometries disagree about which class is largest, and only the
corpus is the product.

**Measure with the routes on.** Everything below is default-on. A census taken
with `TRUCK_FORMAL_RECOVERY=0` describes a configuration nobody ships.

**Use production's entry point.** `face_census` must call
`robust_triangulation_with_torus_outcome`. Each `robust_triangulation_with_*`
takes one more adapter than the last; a caller that stops at the cone form has
the torus route compiled in but unable to fire, and it fails *silently*. If a
route's count is suspiciously zero, check this first. The same shape bit
WAVE-3A in a second place: a defaulted trait method that the derive macros did
not forward answers "nothing" for the production enum and reads exactly like a
route that does not help.

**Join on `source_face_id`.** `declared_face_index` resets per shell and
collides. `p1-out/reconcile.py` does it correctly; reuse it.

**Build first, then hypothesise.** Both WAVE-3A packages had a target from this
file that measurement corrected — one by 36%, one by 84%. WAVE-3B then found
package 6's target was wrong in kind, not degree: its named population could not
be reached and its named mechanism does not exist. The DIAG-001 records are
cheap to extend and have now paid for themselves three times.

**A refusal reached at stage N says nothing about stage N+1.** Every planar
funnel reading in the edition before this one was taken at the *first* gate a
face hit, and two sequential gates hid everything behind them. When a cell's
exits all sit at one stage, that is a sign the stage is a lens, not the cause.

---

## The residual

| terminal reason | faces | share |
|---|---:|---:|
| `ConstraintInsertionIncomplete` | 5,988 | 38.3% |
| `BoundaryProjectionFailed` | 5,825 | 37.3% |
| `AmbiguousLift` | 1,492 | 9.6% |
| `ContradictoryDualParity` | 1,422 | 9.1% |
| `ConstraintOverlapUnsupported` | 427 | 2.7% |
| `NoOddParityRegion` | 338 | 2.2% |
| `BoundaryConstructionFailed` | 127 | 0.8% |
| `ConstraintRoleMissing` | 1 | 0.0% |

Cross-tabbed against surface family — the view that separates work:

```
family          CII   Proj   Lift Parity   Ovlp  NoMat    Bnd   total
Cylinder       1238   1072    426   1180    101                  4017
Plane          2947                         104    200           3251
Nurbs           468   2472             3      9      1           2953
Bspline         467   1180                   69    137           1854
Torus           257    939      3    122                         1321
Cone            303     85    673     81    136                  1278
Sphere          130     31    380     28                          569
Extruded        150     12      4      6      4                   176
Unknown                                                   127     127
Revolved         20      7      6      2      4                    39
Offset            8     27                                         35
```

**The residual is concentrated.** Five reason/family cells hold 58% of it; ten
hold 81%. There is no long tail worth chasing until those are gone.

**The `Plane` row of that table is now explained and should not be read as
2,947 crossings.** WAVE-3B showed the crossings are chord-approximation
artefacts (§6): the boundary curves themselves do not intersect. The cell is
real loss, but its mechanism is the arc approximation, not an arrangement.

`ConstraintInsertionIncomplete` has changed character completely. The seam×seam
class that dominated it before WAVE-3A is **2 faces**. What is left:

```
2,519  SourceSourceInterBoundCrossing
1,796  SourceSyntheticCrossing
1,643  SourceSourceSameBoundCrossing
   27  MixedConstraintConflict
```

by chart rank, 4,028 at rank 0 and 1,960 periodic.

---

## Work packages, in yield-per-effort order

### 1. Analytic surfaces are failing a projection they can solve in closed form — 2,127 faces

Cylinder 1,072, torus 939, cone 85, sphere 31 lost to
`BoundaryProjectionFailed`. **These are not deck artifacts.** The previous
edition said to re-measure them after the deck-join fix landed; that is done,
and the cylinder and torus counts are *identical* to before it — 1,072 and 939,
unchanged. Whatever this is, it is its own defect.

**Where to look first (a hypothesis read off the source, not measured).** The
STEP path represents these as `Processor<E, Matrix4>` — cylinder and cone are
`Processor<RevolutedCurve<Line<Point3>>, Matrix4>`, torus is
`Processor<Torus, Matrix4>`. `Processor`'s `search_nearest_parameter`
(`truck-geometry/src/decorators/processor.rs:571`) asks the entity for its
answer, then **uses it only as a hint** and returns
`algo::surface::search_nearest_parameter(self, point, hint, trials)` — a
generic Newton over the transformed surface. So when that Newton fails to
converge, a closed-form answer that was already in hand is discarded and the
projection fails.

Two more absolute tolerances sit on the same path, neither scaled to the model
or to the caller's `tol`:

- `RevolutedCurve::search_parameter` (`decorators/revolved_curve.rs:~412`)
  ends with `self.subs(t, ang).distance(point) <= 1.0e-5`;
- `Torus::search_parameter` (`specifieds/torus.rs:157`) ends with
  `self.subs(u, v).near(&point)`, which is truck's *global* `TOLERANCE`.

A millimetre-scale part at 500 mm can carry more rounding than either admits.

**First test, before writing any fix:** for these faces, evaluate the entity's
closed-form inverse, transform it back, and record the residual. If the
residuals are small and the Newton simply failed, the fix is to keep the
entity's answer as a candidate and admit it on the caller's tolerance — which
is the tolerance the pipeline validates against anyway. If the residuals are
large, the hypothesis is wrong and the cause is upstream of projection.

Highest confidence per unit of work in this document, and the cell group is
14% of all remaining loss.

### 2. Cylinder parity — 1,422 faces, and the experiment is one line

`ContradictoryDualParity`, overwhelmingly cylindrical (1,180), with torus 122
and cone 81. **Unchanged by WAVE-3A** — the prediction that it would shrink once
seams stopped generating physical-boundary constraints is now falsified twice.

The live question is still the one deferred two waves ago:
`toggles_material` returns `Some(true)` for
`ConstraintRole::UnresolvedSyntheticClosure`, generating `μ_L = 1, μ_R = 0`. An
artificial cut should generate `μ_L = μ_R` (§X Definition 20, second bullet).

Flipping it changes material state for *every* face carrying synthetic
segments, so measure it strictly on its own, and watch **triangles per face**,
not just rendered/lost — the ledger carries `triangles=` for exactly this. A
face that starts rendering with a wildly different triangle count is not a
recovery.

Cheapest experiment on the list by a wide margin. Do it early even if it fails,
because a falsified one-line hypothesis is worth more than another survey.

### 3. Cone and sphere lift — 1,053 of the 1,492 `AmbiguousLift`

Cone 673, sphere 380, cylinder 426. `AMBIGUOUS_STEP_FRACTION = 0.45` bisection
exhausting is the proximate cause; a certified per-family lift rule should clear
most of it. Small and self-contained.

`00005427` is 723 lost faces of which **494 are cone `AmbiguousLift`** — one
model that this package alone would nearly clear, and one of the two models
where the WAVE-2C routes fired zero times.

### 4. Spline projection, the remainder — 3,652 faces

NURBS 2,472 + B-spline 1,180. Still the largest family/reason group after the
plane cell.

WAVE-3A's knot-span seeding **proved the mechanism** — Newton from a single
start on a piecewise surface — and cleared 705 faces with zero regressions and
no effect on any other family. It did not clear more because a face is lost if
*any one* of its boundary points fails to project, so partial success on a face
recovers nothing.

#### The epistemic state, stated precisely

- NURBS 2,472, B-spline 1,180.
- Knot-span seeding recovered 705 faces with zero regressions and no effect on
  any other family, so **initialisation is demonstrably one real cause**.
- Recovery is **all-or-nothing per face**: a face is lost if *any one* boundary
  point fails to project, so partial success recovers nothing.
- **What the diagnostics do not say** is whether each remaining face has one
  failed point or dozens. Everything about how to spend the next session on this
  class turns on that, and nothing on disk answers it.

#### The measurement spec — collect in session 1, during package B

B already works inside the projection path, so this is instrumentation at a site
you are editing anyway. Four fields, all cheap; the first is the decisive one
and the rest are what stop a wrong reading of it.

1. **`failed_points` and `boundary_points` per face.** The counter goes next to
   the existing `return Err(BoundaryProjectionFailed)` in
   `PolyBoundaryPiece::try_new`; the denominator is `bdry3d.len()`. Report the
   *ratio* as well as the count — three failures out of 400 and three out of
   five are different diagnoses.
2. **Which link of the chain failed.** `by_search_nearest_parameter` tries five
   things in order: `search_parameter(hint)`, `search_parameter(None)`,
   `search_nearest_parameter(hint)`, `search_nearest_parameter(None)`, then
   `by_structural_seeds`. Record the furthest link reached. "The seed route ran
   and still failed" and "the seed route never ran" are opposite conclusions and
   are currently indistinguishable.
3. **Seed count actually offered.** `search_parameter_seeds` returns one start
   per knot-span cell, so a surface with a single span offers **one** seed — the
   route fires, does nothing different from the plain call, and looks like a
   failed hypothesis rather than a no-op. Separate those two populations or the
   705 will be misread.
4. **Best-seed residual.** `by_structural_seeds` already computes
   `surface.subs(uv).distance(point)` for every seed and keeps the best; emit it
   on failure. This is what distinguishes *diverged* from *converged just
   outside tolerance* — and if the residuals cluster just above `tol`, the class
   is not an initialisation problem at all but the same
   tolerance/compatibility-factor question package 1 is about, which would merge
   §4 into §1 rather than into §5.

Plus a sample of the failing 3D points, which are already in hand at the failure
site — clustered on one edge (a bad edge curve) reads differently from scattered
across the boundary.

#### How to read it

| result | what session 3 does |
|---|---|
| mostly 1–2 failed points/face, seeds ran, residuals large | **more or better starts.** §4 is the session; 3,652 faces plus whatever share of the 1,956 planar `no_developable_curve` is the same mechanism. |
| seeds never ran, or offered one seed | the 705 understates the route. **Extend the seed source** before concluding anything — cheapest possible outcome. |
| residuals cluster just above `tol` | not initialisation. Merges into §1's tolerance question, and package 1's fix may take this class with it. |
| tens of failed points/face, seeds ran, residuals large | initialisation is dead as a hypothesis and the seeds were treating a symptom. Session 3 is **§5** instead. |

**If only one ride-along survives the session, make it this one** — it is the
number that decides what session 3 even is.

### 5. `SourceSyntheticCrossing` — 1,796 faces, and nobody has looked

Now the second-largest insertion bucket, and the only large class in this
document with **no** hypothesis attached. WAVE-3A established that these do not
come from the two-loop join: 1,507 are `SeamWithoutTwoLoopJoin` and 269 carry
no seam evidence at all.

So a real source trim segment is crossing a synthesised one. Either the
synthetic segment is being placed wrongly, or the face needed no synthetic
segment at all. The DIAG-001 witnesses already name both segments and their
origins; a morning with them would probably produce a mechanism. Cheap to
investigate, and 11% of the residual.

### 6. Rank-0 plane crossings — **measured, and the premise was wrong**

This package said the 2,947 plane `ConstraintInsertionIncomplete` faces were
"physical boundary arcs properly crossing each other", and that the work was an
arrangement build: Step 7′ face extraction plus §X parity selection.

WAVE-3B measured it (`docs/WAVE_3B_PLANAR_ARC_DEVELOPMENT.md`). Two corrections,
both load-bearing:

**The `SliceExit` variants did not isolate the population — nothing reached
them.** All 3,251 planar lost faces exit at Step 2 or Step 3; none reach Step 7.
The arrangement had an empty input.

**The crossings are not real.** With arcs developed analytically and their
intersections certified, 1,163 of the 1,164 faces that resolve carry **zero**
crossings. On a plane the chart map is affine and preserves crossing exactly, so
the legacy CDT's crossing was introduced by approximating arcs as chords before
asking. **ARR-003 is owed to one face in the corpus.**

#### What to build instead: Step 8A-arc

A certified polygonal approximation of an arc within the caller's tolerance,
feeding the existing ear clipping and the **unchanged** Step 8B battery.
`certified_polygonal_region`'s `approximation_is_exact` guard was written for
exactly this arrival: *"The first arc family that arrives will fail that guard
and be forced through its own check rather than inheriting this one."*

The math is elementary and needs no new certification machinery. For an arc of
radius `r` split into `n` equal sub-arcs over sweep `|t1 − t0|`, the chord's
maximum deviation is the sagitta

```
e(n) = r * (1 − cos(|t1 − t0| / (2n)))
```

so the smallest admissible `n` is a closed form in the caller's `tol`. The
approximation error is then *bounded*, not zero, which is precisely the case the
guard exists to distinguish — carry the bound into
`Rank0DevelopedBoundary::approximation_is_exact`'s successor rather than
asserting exactness.

**The order that keeps it refinement-only.** Build against
`formal::planar_developed`'s occurrences, gate it
`TRUCK_FORMAL_RECOVERY_ARC` (default-on, nested under the master gate, per the
regression discipline below), and enter it only where the legacy path already
produced no mesh. Then it can replace nothing but a failure and
`rendered -> lost = 0` is structural.

**Build it in `planar_holes`, not only in `planar_slice`.** Crossing the
developed-track verdict against the funnel (`p1-out/yield_tab.py`) over the
1,163 faces whose boundary is developed, simple and crossing-free:

```
  1,065  delegated to the holes slice   <- multi-bound
     64  only the arc family (the hole-free slice alone unblocks it)
     34  blocked on outer-bound standing (29 of them declare one bound)
```

So Step 8A-arc landed in the hole-free slice alone is worth **64 faces**, not
1,163. The population is overwhelmingly multi-bound. This is the same trap the
preamble warns about — a package that clears one stage recovers nothing on a
face blocked at another — and it is cheap to fall into here because the
developed track surveys every bound and so looks like it has already cleared
them.

The good news is that the shared piece is the *only* new piece. Once an arc is
polygonalized within a certified bound, `planar_holes`'s Steps 7H and 8H are
polygon-based and work unchanged: `classify_components`, `point_strictly_inside`
and `certify_region_with_holes` all consume cycles of `Point2`. So build the
polygonalization once, against `DevelopedCurve2D`, and feed both slices from it.

Adding the outer-bound derivation below takes the ceiling to the full 1,163.

**Two obligations the developed track does *not* discharge**, and that Step 8A
will need:

- **Per-curve simplicity.** `survey_arrangement` skips intra-occurrence piece
  pairs (two x-monotone pieces of one circle share support by construction) and
  reports how many it skipped. That leaves `IndividualCurveNotSimple` unproved.
  For a circular arc it is a one-liner — an arc is simple exactly when its sweep
  is under a full turn — but it must be *asked*.
- **Material selection.** The track surveys every bound and is deliberately
  independent of outer-bound standing, because 1,120 planar lost faces have
  none. Selecting the material region still needs it. See the item below.

Two cheaper items fell out of the same measurement:

- **431 single-bound faces** exit `missing_outer_bound_authority` while
  declaring exactly one bound. A lone bound is the face's boundary; the standing
  is derivable with no geometry and no containment test, and must be labelled
  *derived* rather than source-declared. The cause is a real source class, not a
  bug: `00007705` is 91% plain `FACE_SURFACE` rather than `ADVANCED_FACE`, and
  those legitimately carry `FACE_BOUND` without `FACE_OUTER_BOUND` — 25,330
  against 2,213. **Ship this with Step 8A-arc**, since material selection needs
  the standing anyway. Of the arc-ready 1,163 only 34 are blocked on it (29
  declaring one bound), so on *that* population it is a tail — its value is that
  it also unblocks faces the arc work does not reach.
- **1,956 `no_developable_curve`** — Step 2 cannot traverse the bound at all.
  Splines. Their own family, unmeasured. This is the *largest* remaining planar
  block and nothing is known about it beyond the count; a `curves=` histogram
  over it (the `SliceRecord` now carries representations even on refusal) is an
  hour and would size the next planar wave.

---

## After C — where the residual lives, and what session 3 opens with

A, B and C together are **4,712 faces in scope**, leaving roughly **10,900**.
"In scope" is not "recovered" — this file's history is that measurement corrects
these targets, so treat 10,900 as a floor on what is left, not a forecast.

Where it sits today, by mechanism rather than by cell:

| block | faces | state going into session 3 |
|---|---:|---|
| **spline projection** (§4) — NURBS 2,472 + B-spline 1,180 | 3,652 | **The best epistemics of any remaining class**, and sized by session 1's ride-along. Seeding is a *demonstrated* cause (705 faces, zero regressions); what is unknown is only whether each face has one failed point or dozens. |
| **`SourceSyntheticCrossing`** (§5) | 1,796 | The only large class with **no hypothesis**. Witnesses already exist in DIAG-001 — this needs a morning of reading, not instrumentation. |
| **planar `no_developable_curve`** | 1,956 | **Sized by session 2's ride-along diagnostic.** Almost certainly splines on planar boundaries, in which case it merges with §4 and the two are one build. |
| **cone + sphere lift** (§3) | 1,053 | Self-contained. `AMBIGUOUS_STEP_FRACTION` bisection exhausting; a certified per-family lift rule. `00005427` is 494 of them. |
| **cylinder CII** | 1,238 | Untouched by every wave so far. No hypothesis. |
| remainder | ~1,200 | Long tail across torus/extruded/revolved/offset. Not worth chasing until the above are gone. |

**Read the overlaps carefully.** `SourceSyntheticCrossing` is a sub-bucket of
`ConstraintInsertionIncomplete` and cuts *across* families, so it double-counts
against the per-family CII rows in the residual table above. The family×reason
cross-tab and the insertion-bucket histogram are two views of one population,
not two populations.

**What session 3 opens with**, assuming both ride-alongs were collected: §4's
"How to read it" table maps the four possible diagnostic outcomes onto four
different sessions — more seeds, a wider seed source, a merge into §1's
tolerance question, or a pivot to §5. Three of those four make session 3 the
largest wave since WAVE-2C; the fourth kills a hypothesis that would otherwise
have cost a session to kill.

**§4 is the default next target and deserves to be.** It is the only large class
where a mechanism has been *demonstrated* rather than hypothesised — the 705
faces are evidence, not a story — and the single open question is a counter, not
a theory. §5 is bigger in ambition and emptier in evidence; take it only if the
counter sends you there.

Either way session 3 starts from a decision, not a survey. That is the whole
point of collecting the counters en route.

---

## Where the loss lives

| model | residual | biggest cell |
|---|---:|---|
| `00007705` | 2,602 | 894 plane CII |
| `00000414` | 1,733 | 1,175 NURBS projection |
| `00003172` | 1,537 | 369 cylinder projection |
| `00001075` | 1,458 | 363 NURBS projection |
| `00000730` | 1,396 | 631 cylinder projection |
| `00005760` | 1,363 | 515 plane CII |
| `00009190` | 935 | 205 plane CII |
| `00003902` | 828 | 682 torus projection |
| `00005427` | 723 | 494 cone lift |

The top four hold 47% of all remaining loss, and three of them are dominated by
a projection failure — package 1 or package 4, not the arrangement build.
`00000414` is 68% NURBS projection failures on its own.

**Good single-model targets.** `00003172` and `00000730` are the cleanest tests
of package 1 (369 and 631 cylinder projection failures, nothing else large).
`00009190` is the standard planar workbench — small enough to iterate on, and
every planar number in this document and in `WAVE_3B` was developed on it first,
then confirmed corpus-wide. `00005427` is package 3 nearly on its own (494 of
its 723 are cone `AmbiguousLift`).

---

## Regression discipline

Any new route must:

- be **refinement-only** — entered only where the legacy path produced no mesh,
  so it can replace nothing but a failure. This is what makes
  `rendered -> lost = 0` structural rather than lucky. WAVE-3A's two routes
  both achieve it by construction: one re-runs the ordinary tessellator on the
  same pieces, the other is the last link of the projection chain;
- ship behind `TRUCK_FORMAL_RECOVERY_<ROUTE>`, default-on, disabled by an
  explicit `0`/`off`/`false`/`no`, nested under the master gate, so its own
  contribution stays one subtraction;
- keep the declared population fixed and be reconciled per `source_face_id`
  against the previous pin on all 20 models;
- validate before replacing: constraint completeness, boundary preservation,
  seam-pair consistency, connectedness, Euler characteristic,
  boundary-component count, world-space tessellation tolerance.

Traps that have each cost a session:

- **Never batch benchmark configs.** All-of-A-then-all-of-B once reported a
  2.6× slowdown that does not exist. Alternate, and take the minimum of ≥5 reps.
- **A fresh exe timestamp is not a fresh exe.** `cargo test` rebuilds examples
  using whatever the manifest said at the time. Wait for the build's own
  completion, then verify by behaviour.
- **`.cargo/config.toml`'s path override is invisible in `Cargo.lock`.** A
  measurement taken through it is not a measurement of anything pushed.
  Re-comment it, bump the rev, and confirm the pinned build reproduces the
  number before writing it down.
- **Check free disk before timing anything.** A near-full disk once turned a
  5.5 s workload into 136 s. It sat at **6.7 GB free** during WAVE-3B; the
  stderr probes are 4–100 MB per model and a full corpus sweep of one is ~200 MB.
  Delete them when the table is written.
- **Probe stderr interleaves.** Face tessellation is parallel, so two runs'
  probe output are never byte-identical. Compare them *sorted*, keyed on
  `source_face_id` — a byte diff will report a difference that is not one.

---

## Artifacts

Outside git, in `C:\Users\stefa\look-corpus\p1-out\`:

- `diag-w3a/<id>.jsonl` — the current residual's DIAG-001 records, default-on
  at the pin, per model. **This is the file to start from.**
- `dev-arc/<id>.dev.txt`, `dev_tab.py`, `slice_tab.py` — WAVE-3B's developed-arc
  survey and the two tables in this file's package 6
- `yield_tab.py`, `YIELD_TAB.txt` — the cross-tab that sizes Step 8A-arc: what
  *else* stands between an arc-ready face and a mesh. Run it again after any
  planar change; it is the check that keeps a package's claimed population
  honest.
- `w3a_tab.py`, `W3A_TAB.txt` — every table above
- `deck_tab.py`, `DECK_TAB.txt` — the deck/seam cross-tab that decided WAVE-3A
- `diag-w3a-sweep.sh`, `deck-join-sweep.sh`, `seed-sweep.sh`,
  `sweep-pinned.sh` — the sweeps, all alternating configs per model
- `deckjoin/`, `seed/`, `corpus-pinned/` — `{off,on}` census and ledger pairs
- `reconcile.py` — the per-`source_face_id` regression gate

Build (there is no MSVC toolchain on this machine, only LLVM-MinGW):

```
cargo +stable-x86_64-pc-windows-gnullvm build --release \
      --target x86_64-pc-windows-gnullvm --example face_census
```

**Disk.** WAVE-3B ended by deleting `look/target/{debug,release}` and all of
`truck-fork/target` — 6.2 GB free became 13 GB. The gnullvm tree was kept, so
`face_census` is built and current at this pin; `cargo test` and `cargo check`
will rebuild cold once. `~/.cargo/git` (3.1 GB) and `~/fastbrep/target`
(310 MB) are further headroom if a link ever fails with
`LLVM ERROR: No space left on device`, which is how this has always presented.

### Probes

| variable | what it emits |
|---|---|
| `TRUCK_FACE_DIAG_JSONL=<path>` | DIAG-001, one JSON row per lost face |
| `TRUCK_PROBE_SLICE` | `SLICE` / `HOLES`: the rank-0 funnel, one line per face |
| `TRUCK_PROBE_DEVELOPED` | `DEV`: the developed-arc survey, one line per bound |
| `TRUCK_CENSUS_TOL_FACTOR=<f>` | scales the census tolerance, to ask whether a loss class is an approximation artefact |
| `TRUCK_FORMAL_RECOVERY=0` | master kill switch; each route also has `_<ROUTE>=0` |

`TRUCK_PROBE_DEVELOPED` costs an O(pieces²) certified pass per bound and is off
by default for that reason. On the two 150k-face models a sweep with both probes
on takes several minutes each.
