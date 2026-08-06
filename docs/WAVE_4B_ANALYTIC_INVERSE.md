# WAVE-4B — the closed-form inverse was being computed and thrown away

Package B of the three-session plan, plus the PROJ-001 ride-along diagnostic
that session 3 was to be sized by. The handoff's hypothesis was right, the fix
is four lines, and the diagnostic answers §4's open question in a way none of
the four anticipated outcomes covered.

---

## The hypothesis, confirmed

From the handoff:

> `Processor`'s `search_nearest_parameter` asks the entity for its answer, then
> **uses it only as a hint** and returns
> `algo::surface::search_nearest_parameter(self, point, hint, trials)` — a
> generic Newton over the transformed surface. So when that Newton fails to
> converge, a closed-form answer that was already in hand is discarded and the
> projection fails.

That is exactly what happens. `hint` is not a hint: it is the entity's own
answer for the inverse-transformed point, mapped back to the processor's
parameter axes, and closed-form for every primitive that reaches here —
cylinder and cone are `RevolutedCurve<Line<Point3>>`, torus is `Torus`. The
Newton refines it; when the Newton does not converge, the refinement is
discarded *along with the answer it started from*, and a surface that can
invert itself exactly reports that it cannot invert itself at all.

## The fix

Return the unrefined entity answer when the Newton fails. Four lines, behind
`TRUCK_FORMAL_RECOVERY_ANALYTIC` (default-on).

**This is safe rather than optimistic, and the reason matters.**
`SearchNearestParameter` promises a *nearest* parameter and never an
incidence, so every caller that needs the point to lie on the surface already
checks the residual itself. The meshalgo boundary lift does exactly that,
against the caller's tolerance, immediately after this returns. A bad
closed-form answer is therefore refused by a check that already exists, and
typed as the off-surface point it is (`BoundaryPointOffSurface`) rather than
as a projection that failed. Nothing new is admitted; something already
computed stops being thrown away.

### Only on the hintless call — and that restriction is load-bearing

The meshalgo chain asks four things in order; the third is
`search_nearest_parameter(point, hint)` and the fourth is
`search_nearest_parameter(point, None)`.

Admitted on the third, the entity's answer comes from whichever branch or
period copy the caller's hint led to, and it **pre-empts the better answer the
hintless call would have found**. Measured: one cone face on `00009190` went
`rendered -> lost` that way, with the recovery otherwise bit-identical.
Restricted to the hintless call, the fallback is last in the chain and can
replace nothing but a failure.

Two packages in a row have now turned on *where* a recovery is allowed to fire
rather than on what it computes. WAVE-4A's retry had to move after the torus
route; this one had to move after the hinted call. Both were caught by the
per-`source_face_id` reconciliation and by nothing else.

## PROJ-001 — the ride-along diagnostic

`TRUCK_PROBE_PROJ` emits one line per face that loses at least one boundary
point, with the four fields §4 asked for. Under the probe the boundary walk
does **not** stop at the first failure — the ratio is the measurement, and
three failures out of 400 is a different diagnosis from three out of five.

Over the six models holding most of the projection loss (4,436 faces whose
terminal reason is `BoundaryProjectionFailed`):

```
family        faces   failed pts/face   ratio of boundary
Nurbs          1766      med 2 max 52       med 0.031
Cylinder        997     med 11 max 33       med 0.245
Bspline         882      med 2 max 41       med 0.035
Torus           791      med 3 max 33       med 0.065
```

### Every failing point reaches the last link

```
family        1 sp(hint)  2 sp(None)  3 snp(hint)  4 snp(None)   5 seeds
Nurbs                  0           0            0            0      7109
Cylinder               0           0            0            0     13542
Bspline                0           0            0            0      4109
Torus                  0           0            0            0      2893
```

Not one failing point in any family is lost before the end of the chain. **"The
seed route never ran" is dead as an explanation** — it ran every time.

### The seed route is a structural no-op on the analytic families

Seeds actually offered, by face:

```
              0 seeds   1 seed   2+ seeds
Nurbs              20      487       1259
Cylinder          996        1          0
Bspline            10      566        306
Torus             781       10          0
```

`search_parameter_seeds` is knot-span based and defaulted to empty on
`SearchParameter`; `Processor` does not override it, so a Processor-wrapped
cylinder or torus offers **nothing**. WAVE-3A's seeding could never have
touched package B's population, which is why the counts were unchanged by it —
that is now explained rather than merely observed.

It is also the trap the handoff names, found in a third place: a defaulted
trait method that nothing forwards, answering "no structure to offer" for a
surface that has plenty, and reading exactly like a route that does not help.

**For splines the seeding is real but thin**: 487 NURBS and 566 B-spline faces
are offered exactly **one** seed, where the route fires and does nothing the
plain call had not already done. Those 1,053 faces are not evidence against
initialisation; they are faces the route never really got to try. Only 1,565
faces (NURBS 1,259 + B-spline 306) got a genuine multi-seed attempt.

### What §4's table does not have a row for

The spec's fourth field was the best seed's residual, to distinguish *diverged*
from *converged just outside tolerance*. It came back **empty for every family
and every face**.

That is not a null result, it is a structural one. `by_structural_seeds` scores
seeds with `search_parameter`, which is all-or-nothing: it returns `None`
unless the point is already on the surface within tolerance, so a failing point
produces no residual to compare. The residual question cannot be asked with
`search_parameter` at all — it needs `search_nearest_parameter` from each seed,
which returns a parameter regardless and lets the distance be measured.

**So §4's "residuals cluster just above `tol`" branch is currently
unanswerable, not answered.** Session 3 should not read the empty column as
"residuals are large".

## How this changes the plan for §4

§4's decision table maps four outcomes onto four different sessions. The
measurement lands mostly on the second row — *"seeds never ran, or offered one
seed → extend the seed source before concluding anything, the cheapest possible
outcome"* — but for a reason the row did not anticipate:

- the seed route **always** ran (row 1's and row 4's premise is out);
- for 1,053 spline faces it offered a single seed and was a no-op;
- for the analytic families it offered nothing at all, which package B has now
  addressed by a different route entirely;
- and the residual column, which decides between row 3 and the rest, was asked
  with the wrong primitive and has to be re-asked.

The concrete next step on splines is therefore **widen the seed source and
re-ask the residual with `search_nearest_parameter`**, both cheap, before
spending a session on either initialisation or §5. The median spline face fails
on 2 of ~60 boundary points (ratio 0.03), which is the shape of a problem that
better starts fix — but that is now a statement about 1,565 faces that got a
real attempt, not about all 3,652.

## Result

**+782 faces, 0 regressions, 0 triangle-count changes**, measured as B's own
subtraction with package A left on in both configurations. Cylinder 557,
torus 222, cone 2, sphere 1; 40,980 triangles added.

Best models: `00000730` +281, `00003172` +168, `00007705` +115, `00001075` +87,
`00006483` +61, `00009190` +46.

### The projection stage moved much further than 782 faces

Cross-tabbed against the new residual, the *reasons* shifted far more than the
count did:

| cell | before | after |
|---|---:|---:|
| Torus `BoundaryProjectionFailed` | 939 | **7** |
| Cylinder `BoundaryProjectionFailed` | 1,072 | **0** |
| Torus `ConstraintInsertionIncomplete` | 257 | 970 |
| Cylinder `ConstraintOverlapUnsupported` | 101 | 651 |
| Cylinder `ConstraintInsertionIncomplete` | 1,238 | 1,480 |

**The analytic projection failure is essentially gone** — cylinder to zero,
torus from 939 to 7. What it bought in *faces* is 782, because most of those
faces are now blocked at the next stage instead. This is the handoff's own rule
holding exactly: *a refusal reached at stage N says nothing about stage N+1.*

The 2,127-face target in §1 was a fair count of the cell and an unfair forecast
of the recovery — which is the third time this file's history has recorded that
distinction, and the reason "in scope" is not "recovered".

`BoundaryPointOffSurface` stays at **0**, so no closed-form answer that reaches
the lift is off-surface. The residual check is admitting them on merit rather
than being bypassed.

### One panic, found and typed

`00005641` began aborting **the whole model** once B started returning
parameters for boundaries that previously failed projection outright: some
collapse to a point in the chart, and `flood_parity`'s outer-face branch
unwrapped an `adjacent_edge()` that is `None` for a CDT with fewer than three
distinct vertices. Now typed — the flood stops and step 3 reports
`NoOddParityRegion`, which is what a degenerate chart image is. Verified to
change nothing on `00009190`.

Worth noting for its own sake: a full-model abort showed up as an *empty output
file* in a sweep, not as an error anyone would notice. The `.meta.json`
sentinel is what caught it.

## Reproduce

```bash
p1-out/analytic-sweep.sh                      # B's own subtraction, A left on
python p1-out/parityA/tab.py p1-out/projB/corpus

TRUCK_PROBE_PROJ=1 face_census.exe abc/00000730/*.step 2>proj.txt >/dev/null
python p1-out/proj_tab.py proj.txt p1-out/diag-w3a/00000730.jsonl
```
