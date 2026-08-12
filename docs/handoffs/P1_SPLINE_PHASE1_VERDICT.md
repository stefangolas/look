# P1 SPLINE REGRESSION RECOVERY — PHASE 1 VERDICT (00007667)

**Date:** 2026-08-09
**truck-fork:** `472bfd34` (unchanged this phase)
**look HEAD:** `1482e87` + probe examples (untracked in `examples/`)

This document records the Phase-1 investigation of the sole Track-1 exception in
`00007667`. It answers the narrow redirect question: **does edge #30 have one
source-faithful geometric traversal used consistently by both the plane and the
extruded face uses, or are there face-use-specific traversals?**

Primary verdict: **A — Global source-edge interpretation.**

---

## 1. The shared edge #30, from the STEP source

Face pair compared:

- extruded face `#19018` (lost; `MeshedToNothing` at `472bfd34`)
- plane face `#10428` (renders 28–38 triangles at `472bfd34`) — same shell
  instance, same shared edge entity.

Both faces reference the **same** `EDGE_CURVE` entity:

```
#543271 = EDGE_CURVE( '', #570840, #570839, #573595, .T. )
#573595 = B_SPLINE_CURVE_WITH_KNOTS( '', 3, (...7 ctrl pts...), .UNSPEC., .T., .F.,
                                     (2,1,1,1,1,1,1,1,2),
                                     (-0.514736297197993 ... 1.48526370280201) )
#570840 = VERTEX_POINT(#647154) = vertex 23 = pv_a
#570839 = VERTEX_POINT(#647153) = vertex 22 = pv_b
```

The two face uses differ **only in the `ORIENTED_EDGE` sense**:

| face | loop | oriented edge | sense |
|---|---|---|---|
| #19018 (extruded) | `#338490` use1 | `#518118 = ORIENTED_EDGE(#543271, .T.)` | same-sense |
| #10428 (plane) | `#106650` use1 | `#482783 = ORIENTED_EDGE(#543271, .F.)` | reversed |

This is the ordinary manifold convention: one shared edge, two adjacent faces,
opposite traversal directions. There is exactly **one** underlying edge geometry.

## 2. Source-consistent geometric traversal of edge #30

Curve `#573595`:

- `range_tuple = (-0.5147363, 1.4852637)`
- `evaluation_range = (0, 1)` — the genuine spline domain, basis partition of
  unity throughout
- `C(0) = C(1)` — a **closed loop** over the evaluable domain; the declared
  slivers `[-0.5147, 0) ∪ (1, 1.4853]` evaluate to world-origin garbage
- STEP flag `.T., .F.` = closed, not self-intersecting (a simple loop)

Vertex roots (solved on the genuine domain, residuals at numerical zero):

| vertex | STEP entity | position | parameter | residual |
|---|---|---|---|---|
| pv_a (vtx 23) | `#570840/#647154` | `(-0.0908871, -0.1063173, -0.2015325)` | `t_a = 0.887738874` | `1.3e-12` |
| pv_b (vtx 22) | `#570839/#647153` | `(-0.0856022, -0.1024702, -0.2106862)` | `t_b = 0.171098596` | `3.1e-16` |

Both roots are unique on the simple closed loop (each point of a simple closed
curve has one parameter; the two vertices are interior points, not the seam).

`EDGE_CURVE #543271` has `same_sense = .T.` and runs from start `#570840`
(pv_a, vertex 23) to end `#570839` (pv_b, vertex 22). The curve's natural
parameter direction is increasing `t`. Since `t_a = 0.8877 > t_b = 0.1711`, the
source-consistent oriented traversal is the **seam-wrapped arc**:

```
pv_a = C(0.8877)  →  1.0 (=0.0)  →  C(0.1711) = pv_b
span = (1.0 − 0.8877) + 0.1711 = 0.283360
```

This arc is genuine spline geometry throughout (`basis_is_partition_of_unity`
true at every sample; `min |p| = 0.245`, far from the origin sliver). Its two
endpoints reproduce the STEP vertices to `1e-12`/`1e-16`.

## 3. The two face uses compared

| property | edge #30 source | extruded `#19018` use | plane `#10428` use |
|---|---|---|---|
| STEP vertices | `#570840`→`#570839` (23→22) | same | same |
| vertex roots | `t_a=0.8877`, `t_b=0.1711` | same | same |
| unique? | yes (simple loop, interior pts) | yes | yes |
| source arc | seam-wrapped `[0.8877 → 0.1711]`, span 0.2834 | same | same |
| wraps parameter boundary? | yes (through seam) | yes | yes |
| edge-use orientation | — (same_sense `.T.`) | `ORIENTED_EDGE .T.` | `ORIENTED_EDGE .F.` |
| resulting traversal | `C(0.8877) → C(0.1711)` | natural (23→22) | reversed (22→23) |
| bound closes with source arc? | — | **yes** (paired edge 29 + arc) | **yes** (line 28 + arc) |
| bound closes with `[0,1]` full loop? | — | **no** | **no** |
| current tessellation traversal | — | full `[0,1]` closed loop | full `[0,1]` closed loop |
| supporting-surface compatible? | — | on swept surface (edge = profile + `0.04929·dir`) | on plane (all loop pts at `3e-18`) |

The **current tessellation samples edge #30 over the full closed `[0,1]` loop**
for both faces — that is the `evaluation_range()` P1 behaviour, applied to a
topologically-open edge whose curve happens to be a closed loop. Neither bound
closes with that traversal; the source-defined wrapped arc is what closes both.

## 4. Is the plane face a false-positive render? — YES

The rendered mesh of plane face `#10428` (and `#21482`, the other instance) spans
the **full closed loop**: bbox diagonal `5.44e-2` vs the full loop's `5.46e-2`,
covering points at `t≈0.5` on the far side of the loop. The source face is the
thin region bounded by the straight line edge (#28/#541524) and the short wrapped
arc — a crescent of extent ~`1.7e-2`, not the whole loop disc.

Why it "renders": every point of edge #30's loop lies exactly on the plane
(plane distance `3.5e-18`), so the planar CDT happily fills the region bounded by
the malformed full-loop boundary and emits triangles. **Rendering is not evidence
of semantic correctness**: the plane face is surviving with the wrong edge
realization, exactly as the packet warned not to assume otherwise.

Why the extruded face fails: the same full-loop boundary is fed to the swept
surface, whose trimming does not accept the non-closing/malformed loop →
`MeshedToNothing`. The asymmetry is a projection/trimming robustness difference,
**not** a difference in edge semantics.

## 5. Evidence table (as requested)

| property | edge #30 source | extruded #19018 use | plane #10428 use |
|---|---|---|---|
| STEP vertices | `#570840`/`#570839` | same | same |
| vertex roots | `t_a=0.8877`, `t_b=0.1711` | same | same |
| unique? | yes | yes | yes |
| source arc | seam-wrapped `0.8877→0.1711` | same | same |
| wraps parameter boundary? | yes | yes | yes |
| edge-use orientation | — | `.T.` | `.F.` |
| bound closes with source arc? | — | yes | yes |
| bound closes with `[0,1]`? | — | no | no |
| current tessellation traversal | — | full `[0,1]` loop | full `[0,1]` loop |
| supporting-surface compatible? | — | swept: exact (profile copy) | plane: exact (loop ⊂ plane) |

## 6. Primary verdict: **A — Global source-edge interpretation**

The STEP source defines **one** geometric traversal for edge #30 between its two
vertices: the seam-wrapped arc `C(0.8877) → C(0.1711)` (span 0.2834), genuine
spline geometry throughout, lying on both the swept surface (edge is a translated
profile copy at level `0.0493`) and the plane (whole loop ⊂ plane). The extruded
and plane face uses differ **only by orientation** (`.T.` vs `.F.`), which is the
standard shared-edge manifold convention. There is no source evidence for
face-use-specific intervals.

The currently rendered plane face is **semantically wrong** — it survives because
the full-loop boundary happens to be exactly planar and the CDT fills a wrong
(whole-loop) region. The extruded face is lost for the same underlying cause: the
production `evaluation_range()` sampling of a topologically-open edge whose curve
is a closed loop does not realize the source edge use.

## 7. Minimum production implication

Implement at the **generic edge-use realization layer** (`tessellate_edge` in
`truck-meshalgo/src/tessellation/triangulation.rs`), not per surface family:

For a spline edge whose `evaluation_range()` is a **closed loop**
(`|C(er.0) − C(er.1)|` small) but whose topological edge is **open** (two distinct
source vertices), sample the edge over the **source-determined seam-wrapped arc**
between the two vertex parameters, not over the full `[0,1]` loop. Requirements:

1. both vertex roots exist on the genuine domain within source tolerance;
2. unique roots (simple loop);
3. `basis_is_partition_of_unity` true throughout the sampled arc;
4. no origin sliver samples;
5. orientation preserved (increasing parameter, seam-wrapped when `t_a > t_b`);
6. if any requirement fails, leave the edge unresolved (do not snap, do not heal,
   do not restore raw-range evaluation).

This is the generic "Reconstructable interval" route (`Case 2`) of the packet.
`Case 1` (`CanonicalByEvalRange`) is unchanged: closed topological edges already
realize the source vertex on `[0,1]`.

Predicted effect on `00007667`: the 7 extruded faces recover; the plane faces
`#10428`, `#21482`, and the other plane faces sharing the edge change from
whole-loop false positives to the correct thin crescent (their triangle count
and region will move — that is a *correction*, not a regression; verify region,
not count). Other ABC models are unaffected because no other model has an open
spline edge on a closed loop (sweep showed zero other `Inconsistent` edges).

## 8. Probe tooling added

- `examples/spline_edge_00007667_probe.rs` — per-shell edge deep-dive
- `examples/spline_edge_00007667_instance.rs` — dump the shell containing a face
- `examples/spline_edge_00007667_solve.rs` — exact vertex roots + arc + closure +
  swept-surface structural check
- `examples/spline_edge_00007667_compare.rs` — plane-distance of full/arc/complement
- `examples/spline_edge_00007667_mesh.rs` — production-tolerance tessellation of
  a face and its rendered mesh extent
- `examples/spline_edge_canonical_probe.rs` — canonical-cluster comparison

All are scratch probes; none are wired into production.

## 9. Next step

Phase 2: implement the generic seam-wrapped-arc edge-use canonicalization in
`tessellate_edge` (truck), add the packet's four unit/regression tests, then run
the 308-face transition accounting and the ABC/NIST censuses. Track-2
(`00009190 #33016`) remains deferred to a separate packet.
