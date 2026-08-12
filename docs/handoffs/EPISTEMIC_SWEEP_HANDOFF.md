# EPISTEMIC SWEEP — P1 spline-domain regression cluster + pending #33016

**Date:** 2026-08-09
**truck-fork:** `472bfd34` (pushed) — no changes this packet.
**look HEAD:** `ac201c7` (pins truck `472bfd34`), override re-commented. This
packet adds corpus tooling and the epistemic sweep findings only; the census
integration was already committed at `ac201c7`.

---

## 1. Controlled attribution (00007705)

Ledgers generated with the same census logic at four remote pins, no local
override (worktrees at `Temp\opencode\look-pin-*`):

| pin | rendered | lost | transition vs prior |
|---|---|---|---|
| `018bd469` (baseline) | 21,909 | 167 | — |
| `17ac0f15` (P1-only) | 21,703 | 373 | **+247 rendered→lost, +41 lost→rendered** |
| `d7bb5166` (P3b) | 21,703 | 373 | 0 |
| `472bfd34` (current) | 21,703 | 373 | 0 |

**First responsible pin for the 247-face cluster: P1 (`17ac0f15`).** P2/P3b and
the hardening tranche change nothing on this model. Full ABC:
`018bd469` 837,103 rendered → `472bfd34` 837,170 rendered (net +67;
rendered→lost 308 by 190 plane + 71 cylinder + 17 extruded + 15 cone + 10
bspline + 2 nurbs; lost→rendered 375 dominated by bspline/nurbs/sphere).
`rejected_ambiguous = 0` across ABC at the current pin; the 7 former
`RejectedAmbiguous` sphere faces (00000959 ×4, 00001075 ×2, 00005760 ×1) are
now `AmbiguousLift` (unresolved) — P2 epistemic fix verified.

## 2. Representative historical "rendered" faces are false positives

`scratch_origin_probe` on the P1 pin (rebuilt against truck `17ac0f15`)
shows, for one plane (#120193), one cylinder (#131469), one cone (#125885):

- boundary curve is a degree-3 closed `B_SPLINE_CURVE_WITH_KNOTS` with
  **unclamped end knots** (end multiplicity 2), knot vector `[-0.0625 … 1.0625]`;
- `range_tuple() = (-0.0625, 1.0625)` vs `evaluation_range() = (0, 1)`;
- `subs` at the declared extremes returns the **world origin (0,0,0)** (zero
  basis window);
- over `evaluation_range()` the curve evaluates to genuine source points with
  machine-precision agreement to the STEP vertices.

The pre-P1 baseline sampled boundaries over `range_tuple()`, injecting origin
points; the "rendered" meshes were lenses from the world origin to the tiny
genuine patch. **Classified: false-positive render removed, not accuracy
regression.**

## 3. Edge/face classification sweep (Track 1) — COMPLETE

`examples/spline_edge_epistemic_compact.rs` ran over every baseline→current
rendered→lost face (308 faces) in the ABC corpus, classifying each spline/
NURBS boundary edge. Aggregate (edges):

| model | Canonical | Canonical-sliver | Inconsistent | Ambiguous / NeedsOther |
|---|---|---|---|---|
| 00007705 | 157 | 292 | 0 | 0 |
| 00005760 | 3 | 34 | 0 | 0 |
| 00009190 | 0 | 49 | 0 | 0 |
| 00007667 | 14 | 0 | **7** | 0 |
| 00005641 | 13 | 1 | 0 | 0 |
| 00007744 / 00008001 | 1 each | 1 each | 0 | 0 |

- `Canonical` / `Canonical-sliver`: `evaluation_range()` (resp. with a declared
  sliver that evaluates to origin garbage) already yields a complete,
  source-consistent, closed traversal — basis is a partition of unity
  throughout, endpoints agree with the source vertices to ~1e-15, boundary
  closes. **These cover the overwhelming majority of the 308 (297 faces have
  no Inconsistent edge; every non-00007667 model is fully canonical).**
- **No `Ambiguous`, no `NeedsOtherReconstruction` found.**
- The **only** genuine outlier is 00007667's 7 faces (extruded/swept family).

## 4. The 7 inconsistent edges (00007667) — established, needs its own packet

All 7 faces (#10340 #11866 #13844 #15760 #16752 #19018 #20292) reuse the **same
shared spline boundary edge** (edge idx 30 / 26, orientation true) on a swept
(`NoStructuralReader "swept_surface"`) face. Characteristics that distinguish
it from the canonical cluster:

- `range_tuple = (-0.5147, 1.4853)`, `evaluation_range = (0, 1)`, sliver
  evaluates to the world origin;
- **`res_er = (1.3e-2, 1.4e-2)`** — the evaluation-range endpoints do NOT meet
  the compressed source vertices within tolerance (canonical faces were
  ~1e-15). The genuine edge's closure is not inside `evaluation_range()`.
- the 2-edge bound does not close (`closes=false`) with the other edge
  canonical.

So the generic `CanonicalByEvalRange` rule (a single edge-use canonicalization
witness before loop assembly) does **not** cover these 7. They are a separate,
small, homogeneous mechanism (likely: swept-surface boundary where the spline's
genuine closure sits outside the interior knot span, or the edge-use trim
differs from the evaluated span). Next packet should classify them as
`Reconstructable` vs genuinely `Inconsistent` by searching a source-determined
sub-interval of the genuinely evaluable domain and testing closure against the
STEP vertices — do NOT treat "not canonical" as source rejection.

## 5. Pending (Track 2) — 00009190 #33016

Deferred. Sphere-pole triangle; renders 11 triangles at `d7bb5166`, becomes
`AmbiguousLift` at `7f8e4890` (certified sphere azimuth + P2 `InsufficientEvidence`
demotion). Needs: extract the d7bb5166 11-triangle mesh, test source fidelity
(surface/boundary/material/topology), and identify the exact evidence gap.
Not a P1 spline case; do not conflate with the canonicalization work.

## 6. Next work packet recommendation

If the 00007667 7-face investigation confirms `Reconstructable` (a
source-determined interval realizes the edge), the smallest production fix is
**one generic edge-use canonicalization witness, inserted before face-loop
assembly in `tessellate_edge`**, applied uniformly (no plane/cylinder/cone
special cases), that:
1. computes `evaluation_range()` and checks `basis_is_partition_of_unity` at
   its ends;
2. if the declared range has origin garbage in the sliver, uses
   `evaluation_range()` (already the P1 behavior);
3. for the swept/offset case, additionally searches for the unique
   source-consistent sub-interval that meets the STEP edge vertices and
   closes.

Do not restore raw-range evaluation, do not add tolerance-based healing, and do
not reject on absence of recovery evidence.

## 7. Tooling added (look, untracked → commit)

- `scripts/ensure-build-space.ps1` — run before corpus sweeps/heavy builds
  (removes regenerable `target` dirs when C: free disk < 15 GB).
- `scripts/compare_ledgers.ps1`, `compare_census.ps1`, `transition_analysis.ps1`,
  `list_lost_transitions.ps1`, `three_way.ps1`, `aggregate_census.ps1`,
  `sweep_epistemic.ps1` — corpus comparison/attribution tooling.
- `examples/spline_edge_epistemic_compact.rs` — the edge classification probe.
- `examples/spline_edge_epistemic.rs` (verbose) — superseded by compact;
  kept as scratch, remove if not wanted.

Corpus artifacts: `look-corpus\pin-p1\`, `pin-p3b\`, `pin-harden\`, `final-472\`
(ledgers + JSONL at the three/four pins, current pin = `472bfd34`).
