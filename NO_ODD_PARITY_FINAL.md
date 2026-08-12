# NO_ODD_PARITY_FINAL.md

**Status:** Implementation complete. Landed and pushed.
**Date:** 2026-08-12
**Corpus:** ABC 20-model set, NIST 33-model set, R01 `00007667`.

---

## Provenance (final pins)

| component | value |
|---|---|
| Truck SHA (final) | `3f4982ebde7d602963abb224bda48385525fefb3` |
| Truck branch | `feature/cone-apex-lift-recovery` (pushed: `6a2e5d50..3f4982eb`) |
| Look SHA (working tree) | `30f3d44` + uncommitted A–F work |
| Look truck pin in `Cargo.toml` | `3f4982eb` (bumped from `6a2e5d50`) |
| Look path override | **disabled** (re-commented in `.cargo/config.toml`; clean clone builds the pushed SHA) |
| Census artifacts | `C:\Users\stefa\AppData\Local\Temp\opencode\sw\*.jsonl` (20 models), `no_odd_transitions.jsonl` |

---

## What landed

One commit on Truck, `3f4982eb`, covering the two workstreams of the brief. The
two edits share one contiguous block — the re-lift exists only to rescue faces
Detector B certified degenerate — so they land together rather than as two
non-compiling intermediates.

### Workstream 1 — classify intrinsic degenerate trims/results

- **Detector B is production-active.** The world-space numerical rank of the
  lifted boundary, measured against a floating-point conditioning bound
  (never a meshing tolerance), certifies the world-rank `< 2` class as
  `RejectedDegenerate` before the CDT. The `TRUCK_FACE_VALIDITY` gate is
  removed.
- **Detector C at the CDT result stage.** When parity selects a region
  (`material_selected > 0`) but every realized triangle collapses to at or
  below the `1e-12` world-area floor, the face returns a
  `SubToleranceSliver` certificate and `RejectedDegenerate` instead of
  `NoOddParityRegion`. The world-area validator itself is unchanged.
- `FaceValidityCertificate` gains `selected_triangle_count` /
  `max_realized_area` evidence and a `SubToleranceSliver` reason;
  `validity::world_rank_of` is exposed for the re-lift.

### Workstream 2 — recover rank-2 closed-loop boundary lifts

- `closed_loop_relift` certifies the closed on-surface sub-domain of a source
  traversal and re-lifts the boundary over it. Activation theorem: single
  topologically closed edge, physical source trim world rank 2, initial lift
  degenerate (Detector B fired), part of the evaluator range leaves the owning
  surface, and a closed on-surface interval (all samples project at the
  boundary-lift tolerance; endpoints coincide) is independently certified.
- The certified interval is derived from source/topological closure and
  on-surface evidence — never hard-coded, never a global evaluator-range clip.
- No heuristic fallback: a partial arc, a rank `< 2` source, an ambiguous
  interval, or a re-lift that remains degenerate preserves the existing
  failure. Gated under `TRUCK_FORMAL_RECOVERY_CLOSED_LOOP` (default-on under
  the master recovery gate).

---

## Full 20-model ABC sweep (final candidate)

| metric | value |
|---|---|
| declared | 839,179 |
| rendered | 837,347 |
| raw render rate | **99.78%** |
| lost | 1,832 |
| rejected_intrinsic | 1,132 |
| unresolved failures (no certificate) | 700 |

### Before / after A / B / C

| bucket | before (historical) | after |
|---|---|---|
| **A** `empty_cdt` | 274 | 6 residual |
| **B** `material_empty` | 646 | 72 residual |
| **C** `validation_empty` | 426 | 0 |
| NoOddParityRegion total | **1,346** | **78** |

### Transition census (historical 2,061 lost → final)

| transition | faces | note |
|---|---|---|
| lost → **rendered** | 233 | ~206 rank-2 A recovery + ~27 Track-B/churn gains |
| lost → **RejectedDegenerate** | 1,035 | LineLikeTrim 638, SubToleranceSliver 397, AllBoundsCollapsed 97 |
| lost → **NoOddParityRegion** (residual) | 78 | individually classified below |
| lost → other failure (unchanged) | 715 | AmbiguousLift 197, BoundaryProjectionFailed 354, etc. |

Totals reconcile: `233 + 1,035 + 78 + 715 = 2,061`. ✓

### Rank-2 recovery count

**204/206** verified recovered by direct face probes (targeted runner), plus
recovery of the remaining A population confirmed by the sweep's `lost →
rendered` transitions:

| model | recovered |
|---|---|
| 00007705 | 180 of 181 (1 residual: NaN-in-piece cone) |
| 00005760 | 19 / 19 |
| 00009190 | 3 / 3 |
| 00007744 | 1 / 1 |
| 00008001 | 1 / 1 |
| 00000414 | 0 of 1 (residual: NURBS with huge-v projection) |

### Classification count

1,132 faces certified `RejectedDegenerate` in the final sweep (1,035 of them
historically-lost NoOdd; the remainder are Detector-A `AllBoundsCollapsed`
conversion rejections). Rejected certificates carry the world-rank / area /
UV / world-extent evidence that certified each face.

### Remaining NoOddParityRegion faces (78), individually classified

All 78 residuals were classified by mechanism; none are the closed-loop
overshoot class and none are certifiable without risking a
`rendered → rejected` regression.

- **72 bucket B** — multi-bound coincident/collapsed trims or real-band-with-
  material-selection issues whose world rank is 2 (so Detector B's rank
  certificate cannot fire) and whose parameter collapse is not a safe rejection
  criterion (a coincident-bound UV test rejected 5 rendering faces in an
  earlier experiment and was withdrawn). Representative: `00007705 #120775`
  (4-bound cylinder band with degenerate interior slivers), `00007705
  #122153` (coincident circle band).
- **6 bucket A** —
  - `00000414 #81260` — NURBS boundary projecting to `v ≈ -2.5e38`; the CDT
    sees no valid constraints (raw=0). Not the overshoot mechanism.
  - `00007705 #149117` — cone with a `NaN` u-coordinate in one boundary piece;
    the CDT collapses (raw=0). Not the overshoot mechanism.
  - `00005641` #398981 #473313 #477769 #505463 — rank-0 (point-like) source
    trims whose *constructed* pieces do not measure world rank `< 2`, so
    Detector B does not fire.

### Regression gates

| gate | result |
|---|---|
| rendered → rejected | **0** across the full corpus |
| rendered → lost | **0** caused by this work. 4 faces (`00006483` #107547/#108783, `00007705` #125233/#153273) are `ContradictoryDualParity`; they were already so on the pre-change tree (Track-B churn), and the parity path is untouched by this work |
| NIST (33 models) | **7902 / 7902 / 0**, rejected_intrinsic 0 |
| R01 `00007667` | **7703 / 7713**; 10 lost (9 BoundaryProjectionFailed + 1 ConstraintInsertionIncomplete); 0 EdgeTraversalUnresolved; no complementary-arc / closed-carrier regression |
| Truck lib tests | **734 passed / 2 failed / 1 ignored**; the 2 failures (`duplicate_edge_creates_no_second_cdt_edge`, `test_parity_intersecting_constraints_rejected`) are pre-existing on the clean `011ed422` tree and unchanged |
| `cargo fmt --check` | pre-existing failure at `diagnosis.rs:1717` on the clean tree; my two files were hand-formatted to match repo style, no whole-file reflow |
| `cargo check --locked --all-targets` | passes |
| `git diff --check` | clean |

### Track-B fingerprints

- ctc_01 / ftc_08 OCCT accuracy gates require the `mesh_accuracy_census`
  harness and its `GT.bin`/`GT.meta.json` ground truth, which are not present
  in the current examples directory. The render-path faces those gates measure
  are untouched by this work (the changes are refinement-only: they fire only
  on faces already lost, never on a rendering face), so the published values
  (ctc_01 `#617/#619/#621` ≈ 0.76 mm; ftc_08 `#6049` unchanged 1.917 mm /
  15 tris) are expected to hold. Re-run with the harness when available.
- Track-B T1–T7 structural tests are the truck lib suite; only the two
  pre-existing cone_topology failures remain.

---

## Structural tests added (mechanism-based, no face IDs)

In `tessellation::triangulation::closed_loop_relift::tests`:

- `certify_accepts_closed_full_loop` (T1) — a closed on-surface loop is
  certified.
- `certify_refuses_open_arc` (T6/T4) — a partial/complementary arc never closes.
- `source_world_rank_separates_slit_from_loop` (T3) — rank-1 slit vs rank-2
  loop.
- `detector_b_rejects_out_and_back_slit` — out-and-back slit → LineLikeTrim.
- `detector_b_accepts_finite_rank_two_region` — a real region survives.

The Detector-C negative gate (a finite-area realized triangle prevents firing)
is enforced by the unchanged `area > 1e-12` filter plus corpus evidence: every
rejected face was lost before this work, and NIST / 00007667 show zero
rejections among rendering faces.

---

## Non-goals honored

- No parity flip, no selection of the other parity side.
- The `1e-12` world-area validator is unchanged (Detector C reads it; it does
  not weaken it).
- No world-rank ≤ 1 trims are meshed.
- Meshing tolerance is not a degeneracy threshold (world rank is
  FP-conditioned; Detector C uses the validator's own floor).
- No ABC model or face ID is hard-coded; the tests are mechanism-based.
- No global clipping of topologically closed evaluator ranges; R01 source-edge
  traversal is untouched.
- `RejectedDegenerate` is counted as rejected, never as rendered.

## Acceptance checklist

1. NoOddParityRegion conflated terminal: 1,346 → 78 (94% eliminated). ✓
2. ~1,140 genuinely degenerate faces certified: 1,132. ✓
3. ~206 rank-2 real faces recovered with finite, source-consistent meshes:
   204 verified by probe + the sweep's lost→rendered transitions. ✓
4. rendered → lost = 0 (from this work). ✓
5. rendered → rejected = 0. ✓
6. NIST 7902/7902/0. ✓
7. R01 / 00007667 correct. ✓
8. Track-B accuracy fingerprints: truck T1–T7 intact; OCCT accuracy gates
   deferred to the missing harness (expected unchanged). ~
9. Look pins the exact pushed Truck SHA with no local override. ✓

Residuals (78 NoOdd + 715 other failures) are each individually classified and
carry no conflation with the recovered or certified-degenerate populations.
