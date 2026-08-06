# WAVE-3B — Developing planar boundary arcs, and what it proved about package 6

**Pin.** look `2e40130` on `integration/formal-atlas-wave-2`, truck `d9661022`
on `feature/deck-join-and-seeds`. Both pushed. Path override re-commented and
the pinned build verified to reproduce the override build's records.

**Faces recovered: 0.** This wave adds no recovery route. It builds the seam
that every later recovery on this population needs, and it settles — by
measurement — which recovery that should be.

---

## What the handoff asked for, and why it could not be built

`HANDOFF.md` package 6 named 2,947 rank-0 plane crossings, 19% of the residual,
and described the work as a build: split arcs at their certified intersections,
extract the arrangement's faces, select the material region by parity. It said
the `SliceExit` variants `NonadjacentCrossing` and `BoundaryComponentsCross`
already isolate the population.

They do not, because nothing reaches them. Sweeping the corpus with
`TRUCK_PROBE_SLICE` at the WAVE-3A pin and joining on `source_face_id`:

```
lost(all families)=15,620   lost(Plane)=3,251

Plane-family lost faces, by formal rank-0 exit:
  1,631   holes  : ambient_rank0 / unsupported_curve_representation
  1,120   planar : ambient_rank0 / missing_outer_bound_authority
    500   planar : ambient_rank0 / unsupported_curve_representation
```

**Not one planar lost face reaches Step 7.** Two gates hold all of them:

- **2,131 at Step 3.** `certified_planar_curves` discharges its whole-interval
  curve-on-surface obligation by requiring `CurveSchema::polygonal()`, and
  `look`'s `curve_schema_of` mapped every conic to `unread("circle")`. The
  comment there said so outright: *"Admitting a circular arc here is Milestone
  B, not this change."*
- **1,120 at Step 2**, all `none_declared`. `00007705` is 91% plain
  `FACE_SURFACE` rather than `ADVANCED_FACE`, and those carry `FACE_BOUND`
  without `FACE_OUTER_BOUND` — 25,330 against 2,213. Of the 1,120, **431
  declare exactly one bound**, where the standing is derivable without geometry.

The arrangement had an empty input population. So did every other plan for this
cell.

## The missing seam

`curve2d::DevelopedCurve2D{Line, CircularArc}` — the analytic representation
`xmonotone`, `intersection`, `contact` and `bezier_isect` all consume, and the
data model `GEN-001.md` §8 says ARR-003 through ARR-006 are written against —
**was constructed only in tests**. Nothing built one from a real face.

That, not the sweep, was what package 6 was missing.

## What this wave built

- **`CurveSchema::CircularArc` carries the source circle's placement**
  (`CircularArcPlacement3`: center, cos/sin basis, the curve's own trimmed
  interval). A stage that understands arcs obtains it from the schema the entry
  point already threads, rather than from another adapter parameter — the
  `robust_triangulation_with_*` chain gains no argument, which is the trap
  `census-entry-point-trap` records.
- **`formal::planar_developed`** develops a loop into `DevelopedCurve2D`
  occurrences and certifies their arrangement pairwise through
  `make_x_monotone` / `intersect_x_monotone`.

An arc's complete-interval on-plane obligation is **three vector tests** —
center on the plane, both basis vectors parallel to it — because a circle's
image under an affine map is a circle with the same parameterization. There is
no interval to bound and nothing to sample. It is the same argument
`planar_slice` makes for a polygonal chain, applied to the other family closed
under affine maps.

Two things were needed to make the track see the corpus at all:

- **The closed-edge rule, applied a second time.** An importer recovers an
  edge's trim by solving both vertex points onto the curve; for a closed
  circular edge those are the same solve, so the interval arrives as `(u, u)`.
  `curve_witness` states the rule for the cylinder route — a circular edge
  closed *by source vertex identity* is one full period in the curve's own
  direction — and it applies verbatim here. Without it, 4,230 of `00009190`'s
  11,187 bound records exited `monotone_degenerate_interval`.
- **Intra-occurrence piece pairs are skipped, and counted.** Two x-monotone
  pieces of one circle share support by construction; asking ARR-002's pairwise
  solver about them asks it about a curve and itself, and it correctly answers
  `Unsupported` to the wrong question — 5,090 bound records on `00009190`. The
  obligation those pairs would have discharged is per-curve simplicity
  (`IndividualCurveNotSimple`), which this survey does not claim to discharge.

The track produces **no mesh**. It runs only under `TRUCK_PROBE_DEVELOPED`, its
whole output is a typed record, and every model's census matches the WAVE-3A
pin to the face.

## The result

Corpus-wide, over the 3,251 planar faces the legacy tessellator loses
(`p1-out/dev-arc/`, `p1-out/dev_tab.py`):

```
developed-track outcome per face:
  1,956  no_developable_curve
  1,164  resolved
     86  monotone_interior_classification_undecided
     42  pair_unsupported
      3  pair_unresolved

of the 1,164 that resolved every bound:
  1,163  faces with 0 certified crossings
      1  face  with 1 certified crossing
```

**The crossings are not real.** 1,163 of 1,164 faces the legacy path lost to
`ConstraintInsertionIncomplete` have boundaries that do not self-intersect at
all. On a plane the chart map is affine, so it preserves crossing and
non-crossing exactly — the crossing was introduced by approximating arcs as
chords *before* asking, and it is a property of the polyline, not of the face.

This is falsifiable and was falsified in the intended direction. A positive
control ships with the module: a figure-eight boundary must certify a crossing,
or a zero everywhere else proves nothing.

### What that changes

**ARR-003 is owed to one face in the corpus, not 2,947.** Face extraction and
§X parity selection are still the right general answer, and the handoff's
description of them is still correct — but they are no longer the way to
recover this population, and scheduling a session for them would buy one face.

**What the 1,164 need is Step 8A-arc**: a certified polygonal approximation of
an arc within the caller's tolerance, feeding the existing ear clipping and the
unchanged Step 8B battery. `certified_polygonal_region` already anticipates it
— `PolygonalRegion` refuses to inherit the source-curve Jordan proof unless
approximation error is *exactly* zero, precisely so "the first arc family that
arrives will fail that guard and be forced through its own check". The sagitta
bound is elementary: for radius `r` and sub-arc angle `δ`, error is
`r(1 − cos(δ/2))`.

That is the next build, and it is the one that recovers faces.

## Where the remaining planar loss is

| blocker | faces | what it needs |
|---|---:|---|
| `no_developable_curve` | 1,956 | Step 2 cannot traverse the bound — splines, or degenerate evidence. A spline development is its own family. |
| resolved, 0 crossings | 1,163 | **Step 8A-arc.** Nothing else. |
| `monotone_interior_classification_undecided` | 86 | An x-critical enclosure straddles an interval end. GEN-001 A8. |
| `pair_unsupported` | 42 | ARR-002's admitted envelope. |
| `pair_unresolved` | 3 | The declared numerical policy. Says nothing about the face. |
| resolved, 1 crossing | 1 | ARR-003, genuinely. |

Separately, and not on the arc path at all: **431 single-bound faces** exit
`missing_outer_bound_authority` where the face declares exactly one bound. A
lone bound is that face's boundary; the standing is derivable with no geometry
and no containment test. It must be labelled *derived* rather than
source-declared, but it needs none of the machinery above.

## Artifacts

In `C:\Users\stefa\look-corpus\p1-out\`:

- `dev-arc/<id>.dev.txt` — the `DEV` records, per model, at this pin
- `dev-arc/<id>.census.txt` — the census beside them, matching WAVE-3A
- `dev_tab.py` — the crossing verdict table above
- `slice_tab.py` — the `SliceExit` funnel joined against the lost faces

Reproduce with `TRUCK_PROBE_DEVELOPED=1` (and `TRUCK_PROBE_SLICE=1` for the
funnel). Note that `DEV` records interleave across the parallel face loop:
compare them sorted, never byte-for-byte.
