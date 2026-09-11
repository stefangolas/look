# UNSEEN MODEL PERFORMANCE + FAILURE AUDIT

Diagnostic-only session. No production algorithm, no dependency source, and no
Look production source were modified. One new look-side diagnostic example
(`examples/step_face_timing.rs`) was created and reverted/left as an untracked
artifact; the pinned truck-meshalgo checkout was edited for a per-face timing
experiment and then restored byte-for-byte to `40212ece`.

## Provenance

```
Look SHA        78bd208de5c100242286f71b4b22dd5867e92582
                 (working tree: .cargo/config.toml, Cargo.lock, Cargo.toml,
                  docs/BENCHMARKS.md modified — pre-existing at session start;
                  Cargo.toml pin moved to truck rev 40212ece)
Truck SHA       40212ece7cb0a7a3eaf9576f29fac8097adaf99c  (truck-fork HEAD)
Truck resolved  Cargo.toml pin 40212ece == cargo checkout 40212ec == fork HEAD
Build mode      cargo build --release --locked (profile: thin LTO,
                 codegen-units=1, panic=unwind); 16 logical processors
GPU             NVIDIA GeForce RTX 5050 Laptop GPU / dx12 (render controls)
Witnesses
  C:\Users\stefa\core_xy.step                      9,153,139 bytes
  C:\Users\stefa\look-corpus\formula1\formula1.step 46,179,762 bytes
  C:\Users\stefa\look-corpus\ur10\ur10.step        29,519,685 bytes
```

## Control models

One fresh render run each (`look <model> --view iso --json`), current release
binary, identical build configuration used for UR10. Both are healthy controls:
nothing in the current instrumentation path alters their runtime materially.

```
               step_parse  step_table  step_tessellate  total(wall)  faces   tris      peak WS
core_xy          90.3 ms     92.2 ms      2115.5 ms      4964 ms      5,670   668,351   422 MB
formula1        266.7 ms    219.4 ms      1541.1 ms      3092 ms      5,235   365,624   402 MB
```

(Previously inherited core_xy values — parse 49 ms / table 57 ms / tessellate
718 ms / total 2.42 s — are the same model on the same machine at a quieter
moment; today's control wall is higher because the GPU adapter init lands on
the measured wall. Order of magnitude unchanged: neither model comes near the
90–600 s UR10 regime.)

## UR10 face timing

Sequential per-face ledger through the exact production tessellation entry
(`wrap_shell_with_closure` + `robust_triangulation_with_torus_outcome`, model
tolerance 0.001266367), with a hard external timeout so a hanging face is
identified by its unmatched `BEGIN`.

First ledger (production path, 60 s budget → killed at 75 s):

```
completed faces   3397 of 6048
in-progress face  BEGIN face=88144 shell=31 idx=1  (t=14.7 s, never ended; >60 s at kill)
progress rate     ~46 faces / s
```

Second run skipping 88144 (different shell ordering) hit a second in-progress
face:

```
in-progress face  BEGIN face=89705 shell=26 idx=11  (t=32.2 s, never ended; >128 s at kill)
```

Third run skipping both pathological faces: all remaining 6046 faces completed
in 24.7 s wall / 12,849 ms of tessellation CPU.

Top completed slow faces (all legitimate large B-spline meshes, none hanging):

```
face   kind     ms       tris
93620  bspline  1400.5   123,511
92963  bspline  1188.7   121,148
91928  bspline  1168.2   121,090
88728  bspline   668.9    46,257
88781  bspline   146.4    22,533
```

Unmatched BEGIN faces: `88144` (ledger 1), `89705` (ledger 2). No other face
exceeds ~1.5 s on the production path.

## Pathological face deep timing

Both pathological faces share one structural signature (boundary probe):

```
88144: in_closed=1 in_open=1 loops=2 areas=[-1.29e-2,-7.75e-3]
       declared urange=(0.4625,0.7209) vrange=(-0.3927,6.6759)  <- v spans a full revolution
       open piece gap = 6.282935 (≈ 2π)
89705: in_closed=1 in_open=1 loops=2 areas=[-3.77e-4, +2.69e-2]
       declared urange=(0.4843,0.5129) vrange=(-0.03125,1.03125)
       open piece gap = 0.9987914 (≈ one full u-span)
```

Face 88144 deep timing (identical production call, tolerance scaled only):

```
tol-scale  tol        sampling constraints   samples_on_boundary   tris     elapsed
100        0.1266     212                    30                    409       15 ms
30         0.0380     1,898                  115                   2,348     82 ms
10         0.0127     2,604                  115                   2,994     96 ms
 7         0.00886    19,722                 559                   21,098    15.8 s
 4         0.00507    n/a                    n/a                   n/a       >85 s
 1         0.00127    n/a                    n/a                   n/a       >100 s
```

Stage attribution (each is a separate gate, none modifies code):

| experiment | result | rules out |
|---|---|---|
| `TRUCK_FORMAL_RECOVERY=0` | still >100 s | formal recovery routes |
| `--edge-trace` (replicate `establish_source_edge_traversal`+sample) | all 7 edges `CanonicalByEvalRange`, µs | edge traversal / sampling |
| `--edge-scan` | boundary = 49 points | O(U·V·B) boundary-length theory |
| `--pcurve` (replicate `polyline_on_surface`) | all closure walks ≤ 90 steps | range-rectangle closure walks |
| `--grid` (replicate `insert_surface` range) | ~85 cells; ROLES census shows real grid 2,604→19,722 constraints | plain grid-size theory (a 119k-constraint face 92963 completes in 1.2 s) |
| working set during hang | flat 188 MB (no allocation growth) | building a huge `insert_res` grid |
| `TRUCK_FACE_DOMAIN=1` (face-derived working range instead of `surface.try_range_tuple()`) | 88144 → 4.7–21 ms, 181 tris; 89705 → 24.4 ms, 310 tris | the declared-range boundary handling is the trigger |

**Dominant stage.** The legacy per-face tessellation
(`trimming_tessellation_with_refinement` inside `cshell_tessellation_inner`) —
specifically the interior sampling-grid construction and its constraint
insertion into the spade constrained-Delaunay triangulation
(`insert_surface` → `wire_grid_segment`/`constrain_grid_edge` →
`try_add_constraint`). The trigger is the boundary produced by
`PolyBoundary::new_with_join`'s one-open-piece closure, which walks the open
boundary to the surface's **declared** parameter rectangle; that boundary shape
drives the tolerance-dependent interior grid into pathological interaction
(`samples_on_boundary` grows 115 → 559; per-constraint cost 37 µs → 800 µs+ as
the CDT grows, a near-quadratic blowup). The 8 formal routes, the edge phase,
and every standalone boundary/closure walk are all cheap and uninvolved.

Growth is super-linear: 96 ms at 2,604 constraints → 15.8 s at 19,722 → >100 s
at production tolerance (extrapolated ~100k constraints). Memory is flat, so
this is compute-only — a combinatorial/superlinear CDT constraint-insertion
blowup, not an allocation blowup.

## Eight R01 faces

```
face    elapsed  outcome
92959   19.03 ms Failed EdgeTraversalUnresolved
91924   16.61 ms Failed EdgeTraversalUnresolved
93520   19.07 ms Failed EdgeTraversalUnresolved
93521   21.65 ms Failed EdgeTraversalUnresolved
93548    2.32 ms Failed EdgeTraversalUnresolved
92485   24.19 ms Failed EdgeTraversalUnresolved
92513    2.43 ms Failed EdgeTraversalUnresolved
92486   18.88 ms Failed EdgeTraversalUnresolved
```

Verdict: **UNRELATED CHEAP REFUSALS.** All eight fail in 2–25 ms with
`EdgeTraversalUnresolved` (a source-edge traversal refusal that never reaches
the expensive stages) and are completely unrelated to the UR10 runtime
pathology. No R01 face is pathological, so no R01 stage profiling was needed.

## UR10 pathology classification

**P2 — a small handful of pathological faces: exactly two (88144, 89705).**

- Each is a >100 s hang at production tolerance (observed >60 s and >128 s
  in-progress before kill).
- Every other face (6,046 of 6,048) tessellates on the production path: 12,849
  ms of sequential CPU (~25.5 s wall); a handful of large B-spline fillets
  (93620, 92963, 91928, 88728) are legitimately expensive because they emit
  46k–123k triangles each, not because they are pathological.
- Not P1 (there are two, not one), not P3 (no broad per-face slowdown — the
  rest complete at ~4 ms/face average), not P4 (the model completes in seconds
  when the two faces are skipped; there is no global bottleneck outside
  per-face tessellation).

## Current conversion frontier

Diagnostics regenerated against the current build and written to
`scratch/unseen_audit/<model>_40212ece.diag.jsonl`.

### core_xy — 13 faces

```
reason histogram:
  EdgeCurveConversionFailed ×12   (conversion_stage = edge_conversion)
  AllBoundsCollapsed         × 1   (conversion_stage = bound_conversion)
entity type: face ×13; provenance established ×13
```

| face | shell | reason |
|---|---|---|
| 94753 | 97529 | AllBoundsCollapsed (declared 1, surviving 0) |
| 96104, 96107, 96146, 96147, 96149, 96150, 96157, 96159 | 97550 | EdgeCurveConversionFailed (declared 104, surviving 96) |
| 97134, 97139, 97147, 97149 | 97562 | EdgeCurveConversionFailed (declared 162, surviving 158) |

Mechanism: 12 faces whose boundary edge curves cannot be converted
(`EdgeCurveConversionFailed`), concentrated in two shells of the extruder
(`fan_mount` 97550, `duct` 97562); plus one single-face shell (97529) whose
only bound collapsed (`AllBoundsCollapsed`). All 13 are conversion-stage losses
— no `SurfaceConversionFailed`, no `no-surface`, no `meshed-to-nothing`.

### formula1 — 142 faces

```
reason histogram:
  SurfaceConversionFailed ×142   (conversion_stage = surface_conversion)
entity type: face ×142; provenance established ×142
```

Every one of the 142 lost faces references a **`DEGENERATE_TOROIDAL_SURFACE`**
support surface — the file contains exactly 142 such entities, a perfect 1:1
match. `Surface::try_from(&SurfaceAny)` covers only ElementarySurface /
BSplineSurface / SweptSurface / OffsetSurface, and the `ElementarySurfaceAny`
sub-enum has no `DegenerateToroidalSurface` arm, so the construct parses but
has no conversion path.

Shell concentration: 81967 (32), 81970 (32), 81971 (32), 81975 (32) = 128;
plus 81966 (3), 81968 (3), 81976 (3), 81977 (3), 81973 (2) = 14.

The 142 are **one dominant unsupported conversion construct** (a single
repeated mechanism), not a heterogeneous mix.

### Assembly / appearance observations (kept separate)

- Assembly definition `ball` (node #50, shell 97529) tessellated to nothing.
  This is **the same event** as one of the 13 conversion losses: face 94753 in
  shell 97529 is the ball's only face, lost to `AllBoundsCollapsed` during
  conversion. **YES** — the ball event is a conversion loss, not a separate
  assembly/placement defect.
- Unresolved styled items: core_xy **8**, formula1 **2**. Recorded only;
  appearance is an independent workstream, no change made.

## Stale-diagnostic verdict

`scratch/core_xy_diag.jsonl` (8/13, 47 rows) is **superseded**. It conflates
two diagnostic streams under one file: 13 `ImportDiagnosticRecord` conversion
rows (identical to today's 13) and **34 `FaceDiagnosticRecord` tessellation
rows** (`BoundaryProjectionFailed`, etc.) that no longer occur — the current
build reports 0 faces with no surface and 0 meshed-to-nothing for core_xy. It
predates the DIAG-002 split that separates conversion losses from tessellation
failures. The fresh, SHA-labelled
`scratch/unseen_audit/core_xy_40212ece.diag.jsonl` (13 records) supersedes it.

## Recommended next actions

1. **Performance mechanism to fix:** the declared-range one-open-piece boundary
   closure in `PolyBoundary::new_with_join` and its interaction with the
   interior sampling grid. `TRUCK_FACE_DOMAIN=1` proves a face-derived working
   range eliminates both hangs (4.7–24 ms vs >100 s), but it also slows the
   other ~6,000 faces, so the fix must be targeted at faces with the
   one-open-piece + periodic-axis signature (88144, 89705) rather than applied
   globally, and validated end-to-end (fidelity + timing on UR10 and the
   controls).
2. **Conversion mechanism to attack:** add a conversion arm for
   `DEGENERATE_TOROIDAL_SURFACE`. It is the entire formula1 conversion frontier
   (142 faces, 1:1 entity match, single repeated construct).
3. **Optional secondary:** enrich `ImportDiagnosticRecord` with the failing
   surface/edge entity id so the next conversion frontier can be classified
   from the diagnostic stream alone (today the DEGENERATE_TOROIDAL_SURFACE
   attribution required a source-file grep), and retire
   `scratch/core_xy_diag.jsonl` as a stale mixed-stream artifact.

---

> UR10 is slow because face(s) **88144 and 89705** spend **~99% of their
> runtime in the interior sampling-grid and CDT constraint insertion of the
> legacy per-face tessellation, triggered by the declared-parameter-range
> open-piece boundary closure in `PolyBoundary::new_with_join`**, performing
> **on the order of 100,000 grid-constraint insertions whose per-constraint
> cost explodes from ~37 µs to ~800 µs+ (near-quadratic CDT growth) versus
> approximately 50–200 constraint insertions on a normal face**; the eight R01
> refusals are **unrelated cheap refusals (2–25 ms each)**.

---

# Implementation result — face-local synthetic boundary closure

Shipped after this audit: `PolyBoundary::new_with_join` now obtains its
synthetic boundary-closure rectangle unconditionally through the existing
face-derived `working_range`, replacing the `TRUCK_FACE_DOMAIN`-gated
declared-range choice. `working_range`, the deck/two-loop machinery, the CDT,
and all tolerances are untouched.

## Provenance

```
Truck before   40212ece
Truck after    09726a9e  (tessellation: use face-local range for synthetic boundary closure)
Look repin     1d29299   (deps: update truck for face-local trim closure)
Regression     cone_topology_tests::open_piece_closure_uses_face_local_range_not_declared_range
               (truck-meshalgo lib: 752 passed; the two failures below are pre-existing
                on pristine 40212ece — duplicate_edge_creates_no_second_cdt_edge,
                test_parity_intersecting_constraints_rejected — not caused by this edit)
```

## Witnesses (production tolerance, same per-face production path)

| face | before | after |
|---|---|---|
| #88144 | >100 s, ~100k constraints | 12.5 ms, 57 sampling constraints, 181 tris |
| #89705 | >100 s (observed >128 s in-progress) | 10.9 ms, 122 constraints, 310 tris |
| #92963 | 1.2 s, 119,012 constraints, 121,148 tris | 4.3 ms, 135 constraints, 240 tris |

#92963 is not "unchanged" — the audit mis-classified it as a healthy control.
Its boundary also contains an open piece (gap = 1.0), and its 121k-triangle
mesh was the same artificial declared-rectangle mesh as the pathological
faces, merely completing in 1.2 s instead of hanging. Under the fix it produces
its true small fillet mesh. 8 R01 refusals: unchanged typed
`EdgeTraversalUnresolved` refusals (2–91 ms class).

## UR10 whole-model (pinned build, `--view iso` render)

```
before   >90 s direct, >600 s corpus timeout
after    wall 3.2 s, step_parse 143 ms, step_table 138 ms,
         step_tessellate 2,173 ms, total 3,166 ms, peak ~396 MB,
         502,130 triangles
```

## Faces lost by this change: NONE

Face-outcome census on the pinned build:

```
UR10      8 of 6048 missing  (0 convert, 8 no-surface, 0 empty)
          = the same 8 R01 EdgeTraversalUnresolved refusals as before:
            92959, 91924, 93520, 93521, 93548, 92485, 92513, 92486
core_xy   13 of 5670 missing (13 convert, 0 no-surface, 0 empty)
          = unchanged conversion losses (12 EdgeCurveConversionFailed +
            1 AllBoundsCollapsed), incl. the 'ball' node #97529 event
formula1  142 of 5235 missing (142 convert, 0 no-surface, 0 empty)
          = unchanged DEGENERATE_TOROIDAL_SURFACE conversion losses
```

No face that rendered before is dropped; no R01 refusal is turned into
invented geometry; no new tessellation refusal appears.

## Output-framing change to disclose

The fix corrects every open-piece face that was meshing an artificial
declared-range rectangle, not only the two that hung. Triangle counts drop on
those faces because the artificial region is gone:

```
core_xy   668,351 -> 406,096 tris  (-262k = faces 92720/92718/92719/92721,
          the audit's "large bspline" 77k-triangle faces; now 58-98 tris each)
UR10      ~853k (audit non-pathological sum) -> 502,130 tris
formula1  essentially unchanged 365,624 -> 365,809 (no open-piece faces)
```

Ordinary closed-loop faces are untouched (e.g. core_xy face 93534, 3,747 tris
before and after). This is a geometry-correctness correction, not a fidelity
loss: the removed triangles lay between the true trim boundary and the
carrier's declared rectangle.

## Final verdict

> UR10's two pathological B-spline faces were fixed by promoting the existing
> face-local `working_range` to the production synthetic-closure domain in
> `PolyBoundary::new_with_join`; this eliminated the artificial constraint
> explosion without changing carrier evaluation, source topology, tolerance,
> or CDT semantics.

Open follow-ups (out of scope here): `DEGENERATE_TOROIDAL_SURFACE` conversion
(formula1's 142 faces), and `EdgeCurveConversionFailed`/`AllBoundsCollapsed`
(core_xy's 13 faces).

