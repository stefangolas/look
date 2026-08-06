# Handoff — recovering the most faces from here

**State.** look `165e865` on `integration/formal-atlas-wave-2`, pinned to truck
`562299a5` on `feature/deck-join-and-seeds`. Both pushed. Path override
disabled and the pinned build verified to reproduce the override build's census
exactly.

**823,559 of 839,179 faces render (98.14%). 15,620 are lost.** WAVE-3A added
+3,790 (`docs/WAVE_3A_DECK_JOIN_AND_SEED.md`); `docs/WAVE_2C_GATE_GRADUATION.md`
covers the wave before it.

**Two of the twenty models now render every face** — `00005642` (177,285) and
`00009272` (146,520), the two largest in the corpus. `00009972` is 8 faces from
a third. The other 17 range from 0.17% to 11.79% loss.

This document is about the 15,620, ordered by **how many faces you get for how
much work**. Every number is freshly measured at the current pin, default-on.

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
file that measurement corrected — one by 36%, one by 84%. The DIAG-001 records
are cheap to extend and have paid for themselves twice.

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

**Measure this before adding seeds.** The record does not currently carry how
many points failed per face. Extend DIAG-001 with a failing-point count and a
sample of the failing 3D points. If the distribution is mostly one or two
points per face, more or better starts will clear whole faces cheaply. If it is
tens, the cause is not the initialisation and the seeds were treating a
symptom.

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

### 6. Rank-0 plane crossings — 2,947 faces, but this one is a build

The single largest cell, 19% of the residual: physical boundary arcs properly
crossing each other in a simply connected parameter domain. 2,250 inter-bound
plus 697 same-bound, plus ~590 more on splines.

Do not schedule this as a fix. Everything downstream of the planar slice's Step
7 assumes a **simple** Jordan boundary — Step 8A's polygonal region, ear
clipping "inserts no Steiner points and emits only interior triangles", Step
8B's battery asserting "the mesh boundary equals the expected polygon cycle". A
normalized arrangement is not a simple polygon. Splitting arcs at their
certified intersections yields a planar *arrangement* whose faces must be
extracted and whose material region must be selected by parity: a new Step 7′
and a new material selection.

What exists to build it on: `formal/bezier_isect::intersect_bezier_pair`
(certified roots, germs, transverse orientation, canonical pair-local
identities), `formal/exact`, and the `SliceExit` variants that already isolate
the population (`NonadjacentCrossing`, `BoundaryComponentsCross`).
`ParameterEnclosure2` on the DIAG-001 witnesses is `None` everywhere and is the
natural first observable.

Take it on when packages 1–5 are done, and give it a session of its own.

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
  5.5 s workload into 136 s.

---

## Artifacts

Outside git, in `C:\Users\stefa\look-corpus\p1-out\`:

- `diag-w3a/<id>.jsonl` — the current residual's DIAG-001 records, default-on
  at the pin, per model. **This is the file to start from.**
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
