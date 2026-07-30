# INC-VERTEX-LOOP-001 — Collapsed `VERTEX_LOOP` treated as unresolved

**Family** `INC` · **Manifestation** `OMISSION`
**Contracts** `TOP-003`; opens `QUO-005`, `DOM-003`

## 1. Status

```
Closed          — for the resolution obligation
Split           — the downstream failure is PAR-RANGE-INHERITANCE-001 et seq.
```

The largest single measured recovery in the project, and explicitly **not
sufficient**. Resolving the entity is done; the geometry is not.

## 2. Mathematical objects

ISO 10303-42 permits a face bound to reference an `EDGE_LOOP` **or** a
`VERTEX_LOOP`. A `VERTEX_LOOP` is a loop of a single vertex and no edges: a
boundary that has **collapsed** to a point, which happens where the chart
itself degenerates — a cone apex, a sphere pole.

In the chart it is one point of a whole $\varphi$-interval; in $\mathbb{R}^3$
it is one point. It bounds by the surface's own degeneracy, not by a curve.

## 3. Required obligation

Every bound the source names must **resolve to a representable bound**, and the
representation must distinguish a collapsed bound from an ordinary one, because
they contribute differently to the trim. `TOP-003` for resolution; `QUO-005`
for the singular chart the collapsed bound marks.

## 4. What the implementation did

`FaceBoundHolder::bound_holder` consulted `table.edge_loop` only, with a comment
admitting it:

> *"For now, we are going with the policy of accepting nothing but edgeloop."*

Every `VERTEX_LOOP` therefore failed to resolve, which under
[`INC-EDGE-DROP-001`](INC-EDGE-DROP-001.md) destroyed the **whole face**.

## 5. Minimal counterexample

`../look-collapsed-boundary/repro/apex_only.stp`, reduced from NIST
`nist_ctc_01_asme1_rd.stp` by rewriting the shell's face list and nothing else,
so `diff` against the original shows exactly one altered record:

```
#370  = ADVANCED_FACE('',(#549,#550),#113,.F.)
#549  = FACE_BOUND('',#671,.T.)   →  #671 = EDGE_LOOP('',(#1225))    base circle
#550  = FACE_BOUND('',#115,.T.)   →  #115 = VERTEX_LOOP('',#1702)    apex
#113  = CONICAL_SURFACE('',#3501,10.,59.)
#1702 = VERTEX_POINT('',#5144)    →  (-30, -73.9914, -25)
```

## 6. Control / oracle

`../look-collapsed-boundary/repro/plane_control.stp` — an ordinary planar face
from the same file, reduced the same way: **46 triangles**.

## 7. Measurements

**The entity count matches the failure count exactly, in all eight files that
contain one.**

| | before | after |
|---|---:|---:|
| ABC `00009190` faces lost | 604 | **396** |
| — failed to convert | 274 | **3** |
| — meshed to nothing | 103 | 166 |
| faces rendered | 23,598 | **23,806** |
| triangles | 214,211 | **216,335** |
| blob shells | 10 | 10 (ratios identical to 5 dp) |
| NIST faces lost | 356 | **356** |

`00009190` contains exactly **272 `VERTEX_LOOP` entities and exactly 272
`LoopReferenceUnresolved` failures** — 1:1.

**Necessary, not sufficient.** Of the 404 faces that now convert:

- **ABC +208 render, not +272** — 64 recovered apex faces convert and then mesh
  to nothing;
- **NIST +0** — all 132 turned from `LoopReferenceUnresolved` into
  `MeshedToNothing`, with spot-checked triangle counts identical, so nothing
  regressed and nothing improved.

## 8. First divergent checkpoint

**B/D — entity resolution.** For the residual 196 faces the first divergence
moves downstream to **I — material-domain construction**, which is the split.

## 9. Causal derivation

```
bound_holder resolves only against table.edge_loop
→ a VERTEX_LOOP bound yields LoopReferenceUnresolved
→ fail-whole-bound (correctly) refuses the entire face
→ every face with an apex or pole vanishes
```

And after the fix, for the residual population:

```
collapsed bound contributes no trim segment (correct)
→ the face's only remaining bound is its base circle
→ that circle bounds nothing enclosing area in the declared domain
→ empty material region
→ MeshedToNothing
```

## 10. Proposed correction

Resolve the entity, and keep the two kinds of bound apart **at the type level**.

## 11. Experimental correction

None.

## 12. Production correction

`stefangolas/truck` `4470ae89` *"Resolve a face bound that collapses to a
vertex"*; `look` `9a04e93`.

A collapsed bound contributes **no trim segment**. The apex is closed by the
surface's own degeneracy, so nothing is the honest contribution — a synthesised
zero-size loop would trim the face by an empty region and delete it just as
thoroughly. `FaceBoundLoop` keeps the two kinds apart so that mistake is not
expressible.

A face whose bounds **all** collapse is still refused (`AllBoundsCollapsed`),
because trimming by no boundary at all emits the entire unbounded surface —
the blob failure mode. **The refusal is why the fix added no blobs and it must
be preserved.**

Faces carrying a collapsed bound are counted under `TRUCK_PROBE_SINGULAR`,
since their domain now has a singular point nothing downstream is told about.

## 13. Regression tests

None named for the ID. The reproducer pair (`apex_only.stp` /
`plane_control.stp`) exists and is run by hand; wiring it into
`tests/step.rs` as `inc_vertex_loop_001_collapsed_bound_is_retained` plus its
control is the concrete next step, and it is the **highest-value ID-named test
to write first** because it is the one that would currently fail for a
different reason than it did originally.

## 14. Corpus-wide effect

ABC conversion failures 274 → 3. NIST loss unchanged at 356, reclassified
entirely from convert-stage to tessellate-stage.

## 15. Known exclusions

- Does **not** touch `UNKNOWN-NIST-ORDINARY-CONE`. Measured, not assumed —
  see §16.
- Does **not** fix the recovered faces' geometry. The residual is
  [`PAR-RANGE-INHERITANCE-001`](PAR-RANGE-INHERITANCE-001.md),
  [`QUO-EUCLIDEAN-CLOSURE-001`](QUO-EUCLIDEAN-CLOSURE-001.md),
  [`DOM-ARTIFICIAL-CLOSURE-001`](DOM-ARTIFICIAL-CLOSURE-001.md),
  [`DOM-ZERO-AREA-001`](DOM-ZERO-AREA-001.md).

## 16. A falsified hypothesis, preserved

It was worth asking whether the 216 `NoSurfaceProduced` cone faces shared this
cause — one fix would have recovered both. **They do not**, on three
measurements:

**Perfectly anti-correlated.** Across all 33 NIST models, every model with cone
tessellation failures has **zero** `VERTEX_LOOP` entities, and every model with
`VERTEX_LOOP` failures has **zero** cone failures. Not one model has both.

| part | encoding | cone no-surface | `VERTEX_LOOP` in file | loop failures |
|---|---|---:|---:|---:|
| `ctc_02` | ap203geom | 148 | 0 | 0 |
| `ctc_02` | ap203pmi | 0 | 74 | 74 |
| `ctc_05` | ap203geom | 20 | 0 | 0 |
| `ctc_05` | ap203pmi | 0 | 10 | 10 |
| `ftc_07` | ap242 | 16 | 0 | 0 |

The anti-correlation is an **encoding artifact, not a shared cause**: two
encodings of one part model the same features differently, so each file exhibits
only one defect. The tidy 2:1 ratio is a property of how each exporter splits
those features and nothing more.

**The failing cone faces are not collapsed boundaries.** Face `#4932` of
`ap203geom/ctc_05`:

```
#4932 = ADVANCED_FACE('',(#4931),#4924,.F.)
#4931 = FACE_OUTER_BOUND('',#4930,.F.)
#4930 = EDGE_LOOP('',(#4926,#4928,#4929))   ← three real edges,
        two LINEs and a CIRCLE, three distinct vertices, none coincident
```

**Nor is it the angle-unit defect** — see `SEM-UNIT-ANGLE-001` §15.

Consequence for the estimate: fixing `VERTEX_LOOP` recovers ~404 faces, **not
404 + 216**. This is the entry that justifies the index's rule against merging
populations on aggregate counts.

## 17. Claim status

- **(D)** 1:1 entity-to-failure correspondence in all eight files.
- **(D)** ABC 604 → 396, conversion failures 274 → 3, +2,124 triangles, no blob
  regression.
- **(D)** Only 208 of 404 recovered faces render.
- **(D)** The 216-cone population is distinct — three independent measurements.
- **(U)** The mechanism for the **64 ABC apex faces** that still mesh to
  nothing. They share the label and the collapsed-bound structure with the NIST
  132, but no per-face trace has been run. NIST loses *all* its apex faces while
  ABC loses 64 of 272 — **that asymmetry is unexplained and is evidence the apex
  population is not homogeneous.** (`FORMALISM.md` U3)

## 18. Links

- `truck` `4470ae89`; `look` `9a04e93`, `ee978f6` (the falsification)
- [`../look-collapsed-boundary/FORMALISM.md`](../../../look-collapsed-boundary/FORMALISM.md)
  D1–D5, D10, D11, U3
- `../look-collapsed-boundary/measurements/census-nist-all.txt`,
  `census-abc-00009190.txt`
- `examples/face_census.rs`
