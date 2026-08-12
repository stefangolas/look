# P1 SPLINE REGRESSION RECOVERY — PHASE 2 HANDOFF

**Date:** 2026-08-09
**truck-fork:** `472bfd34` (working tree, no commits)
**look HEAD:** `1482e87` (working tree, no commits)

---

## 1. Current repo state

**truck-fork** — branch `feature/cone-apex-lift-recovery`, at `472bfd34`, clean
except one **untracked, uncommitted** new source file:

```
truck-meshalgo/src/tessellation/source_edge.rs   (NEW, draft, NOT compiled, NOT wired)
```

That is the only source change made this session. It is a first draft of the
`EstablishSourceEdgeTraversal` module containing `ParamTraversal`
(Simple/Wrapped), `SourceEdgeTraversal`
(CanonicalByEvalRange / CanonicalBySourceInterval / Unresolved), a witness
struct, `establish_source_edge_traversal`, `sample_traversal`, a golden-section
root solver, and two unit tests. **It has not been registered in
`tessellation/mod.rs`, not wired into `tessellate_edge`, and not compiled.**
Treat it as scratch that needs review before use — do not assume its API is
final.

**look** — branch `integration/formal-atlas-wave-2`, at `1482e87`, with
untracked scratch probes only (not committed):

```
examples/spline_edge_00007667_plane_witness.rs   (direct #10428 witness)
examples/spline_edge_00007667_tolreconcile.rs    (production-tolerance reconciliation)
opencode.json                                    (do not commit)
```

`.cargo/config.toml` overrides remain commented; `look` builds against the git
pin `472bfd34`. No production change landed in either repo.

---

## 2. Settled theorem

A topological STEP edge is **not** identified by a curve's evaluator domain.
`evaluation_range()` means "safe/genuine evaluator domain"; it does **not** mean
"source edge interval". For ordinary P1 they coincide; for `00007667` edge #30
they do not.

Edge #30 (`EDGE_CURVE #543271`, spline `#573595`, `same_sense=.T.`) is a closed
loop over `evaluation_range=(0,1)` with `C(0)=C(1)`. Source vertices sit at
interior parameters:

```
front  #570840 = vertex 23 = pv_a:  t_a ≈ 0.887738874  (residual ≈ 1.3e-12)
back   #570839 = vertex 22 = pv_b:  t_b ≈ 0.171098596  (residual ≈ 3.1e-16)
```

The source traversal is the **seam-wrapped arc** from `t_a` through the `[1→0]`
evaluator closure to `t_b` (span ≈ 0.2834):

```
C(0.887738874) → C(1.0)=C(0.0) → C(0.171098596)
```

This is the geometric content of the underlying edge for **both** the extruded
faces (`#10340 #11866 #13844 #15760 #16752 #19018 #20292`, `ORIENTED_EDGE .T.`)
and the plane faces (`#10428 #21482`, `.F.`); the uses differ only by
orientation. The full `[0,1]` loop is never the correct boundary for either.

---

## 3. Settled architecture

- At `tessellate_edge` (triangulation.rs:1448) we have the curve, the
  `CompressedEdge.vertices` **front/back source-vertex indices**, and the
  `vertices` clone (line 1377) giving both vertex positions. That is enough.
- **`same_sense` is already normalized by the importer.** `truck-stepio`
  `sub_parse_curve3d` ends with `if !same_sense { curve.invert(); }`
  (`truck-stepio/src/in/mod.rs:2988`). Therefore the **stored curve's
  increasing-parameter direction is always the source edge direction**, from
  `edge_start` to `edge_end`. `CompressedEdge.vertices = (front, back)` keeps
  that order. So the traversal is: follow increasing internal parameter from
  the front vertex root to the back vertex root, wrapping through the seam when
  the closed domain puts the front root after the back root. This is the
  orientation theorem, not an "if t_start > t_end then wrap" heuristic — it is
  what makes `same_sense=.F.` (curve inverted by the importer) come out right.
- **Arbitrary sub-range sampling already exists.** `Curve3D` derives
  `ParameterDivision1D` and `algo::curve::parameter_division(curve, range, tol)`
  samples any sub-range. A wrapped traversal is `(start, hi)` + `(lo, end)`
  joined, dropping the duplicate closure sample.
- Surface compatibility is enforced **downstream** (boundary projection at
  `PolyBoundaryPiece::try_new`); do not re-derive it at the edge layer.

---

## 4. Next implementation

In `truck-meshalgo/src/tessellation`:

1. Register `source_edge` in `tessellation/mod.rs` (`pub mod source_edge;` or
   `mod source_edge;` + re-export as needed) and review the draft in
   `source_edge.rs` before wiring.
2. Wire `establish_source_edge_traversal` into `tessellate_edge`
   (triangulation.rs:1448): compute vertex positions from the `vertices` clone,
   call the helper, and:
   - `CanonicalByEvalRange { range }` → sample `range` (preserves ordinary P1;
     keep the existing closed-edge period/POU extensions as-is).
   - `CanonicalBySourceInterval { traversal, .. }` → `sample_traversal` (simple,
     or wrapped = two pieces joined, seam sample deduplicated).
   - `Unresolved` → see warning below.
3. Two tests (already drafted in `source_edge.rs`): one ordinary canonical
   closed/unclamped case (expects `CanonicalByEvalRange`, no origin garbage),
   one `00007667`-shape closed-spline case (expects `CanonicalBySourceInterval`
   with wrapped traversal through the seam).
4. Compile `cargo check --locked --all-targets` before adding the full six-test
   matrix, then targeted `00007667` validation (7 extruded faces recover; plane
   faces `#10428 #21482` correct to the source crescent) and the NIST/ABC
   regressions (`7901/7902`, `nist_13 #1167` stays unresolved, `#33016` sphere
   track untouched).
5. Commit discipline: commit truck changes/tests, keep scratch probes out, do
   not commit `opencode.json`, keep `.cargo/config.toml` overrides commented,
   record the exact truck commit, then update the look pin.

---

## 5. Warnings for the next agent

- **`Unresolved` must NOT silently fall back to sampling the full
  `evaluation_range`.** For edge #30 that full-loop sample is exactly the
  malformed boundary that must not be re-emitted (it is the current
  `MeshedToNothing`/false-positive cause). `Unresolved` means "no certified
  traversal", not "sample the loop". Decide the caller's fallback explicitly
  and document it; do not let the legacy full-loop sample leak back through the
  `Unresolved` arm.
- **Fixed-grid numeric root search is candidate generation, not proof of
  uniqueness.** A coarse scan + golden-section can miss a second candidate
  between grid points. The production witness must isolate all source-consistent
  candidate roots (deterministic isolation/refinement) and require exactly one
  relevant root per vertex (modulo the known closure equivalence `lo ~ hi`),
  or return `Unresolved`. Do not encode "simple closed curve, therefore the
  nearest root is unique" as the whole argument.
- **Do not derive source-incidence semantics casually from mesh tolerance.**
  `tol` is a chord-length/geometric-error bound; it is not evidence that a
  vertex lies on a curve or that two entities are source-consistent. Use a
  source tolerance that is a small fraction of the model tolerance but far above
  vertex-on-curve numerical noise, and justify it — the residuals for edge #30
  are ~1e-12, so a tight source tolerance is well within reach.
- **Do not investigate or fix any of the new RS-02+ audit findings in this
  packet.** They are a separate track. Keep this change scoped to the source-edge
  traversal abstraction and its validation.

### Already reconciled — do not re-investigate

**Production-tolerance discrepancy.** At the exact production entry point and
policy (`face_census`, same pin `472bfd34`, whole-model diameter
`1.517322`, tolerance `1.517e-3`), `#19018` reproduces the ledger exactly:
`MeshedToNothing`; `#10428` = 28 triangles, `#21482` = 27 triangles. The
earlier probe's 57-triangle result came from a hardcoded `tol=1e-3`
(tolerance-sensitive malformed full-loop behavior), and the `spline_edge_...mesh`
probe's reported `t≈0` roots are a known bug in that probe's coarse scanner —
the dedicated solve probe confirms `t_a=0.887738874`, `t_b=0.171098596`. This
is an experimental-control artifact, not evidence against the edge semantics.

**Direct `#10428` false-positive witness.** The currently-rendered `#10428`
mesh contains a vertex exactly at `C(0.5)` (distance `0.0`), which lies on the
complementary/far-side arc — on the plane, but on **neither** the source arc
`[t_a → t_b]` nor the chord (`C(0.5)` is `3.54e-2` from the chord). The rendered
bbox diagonal is `5.44e-2` vs the full loop's `5.46e-2`. This is a direct
witness, independent of bbox inference: the plane face is a whole-loop
false-positive and must change to the source crescent after the fix.
