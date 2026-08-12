# R01 HANDOFF — P1 SPLINE PHASE 2 SOURCE-EDGE TRAVERSAL — FINAL

**Date:** 2026-08-10
**Status:** R01 LANDED. NIST 7901/7902, sole loss `nist_13 #1167`; 00007667 7703/7713 with the complementary-arc witness preserved.

---

## 1. Final commits

| worktree | commit | notes |
|---|---|---|
| truck-fork (`feature/cone-apex-lift-recovery`) | **`b4cebf05`** | R01 source-edge traversal under the source's declared STEP geometric uncertainty; pushed to origin. |
| look (`integration/formal-atlas-wave-2`) | pin `b4cebf05`, rev bumped from `472bfd34` in `Cargo.toml` + `Cargo.lock`; `.cargo/config.toml` paths override re-commented | pinned-state census verified from the git pin (not a path override). |

## 2. Final theorem

> STEP source-edge traversal is established over the genuine evaluator domain
> from source topology and source endpoint incidence. Endpoint incidence is
> judged under the geometric uncertainty declared by the applicable STEP
> representation context when present, with Truck's numerical tolerance used
> only as fallback. Parameter uniqueness, evaluator-domain validity, topology,
> and wrap authority remain independent obligations.

Architectural lesson recorded: **source uncertainty is source semantic
provenance and must not be replaced downstream by a generic numerical epsilon.**

## 3. NIST census (final, pinned state)

```
33 models, 7902 faces declared, 7901 rendered, 1 lost (0.01%)
  sole loss: nist_13 #1167  (MeshedToNothing / bspline, declared_face_index=61,
             byte-identical to the 472bfd34 baseline ledger)
```

- Rendered→lost vs baseline `472bfd34`: **0**.
- Lost→rendered vs baseline: **none** (baseline rendered everything except #1167).
- All 156 R01-induced regressions recovered: **23** by the open-carrier theorem
  (nist_23, nist_27) + **133** by source-tolerance semantics (the six models).
- New rejected population: none.

### The 133 source-tolerance recoveries (targeted 6-model run)

All six targeted models: **2114/2114 rendered, 0 lost**; the pre-fix lost
population (47+17+27+11+27+4 = 133 faces) is fully recovered.

All **110** previously-unresolved distinct edges transitioned
`Unresolved → EvalRange` (0 remain unresolved; 0 became SourceInterval), each
with endpoint residuals ≤ the model's STEP uncertainty:

| model | STEP uncertainty (native units) | evidence |
|---|---|---|
| nist_2 | 1.192e-1 mm | residuals 1.3e-6..2.6e-4 |
| nist_5 | 3.669e-3 in | residuals 5.4e-7..7.2e-4 |
| nist_18 | 1.0e-2 mm | residuals 1.3e-7..1.7e-5 |
| nist_28 | 5.0e-3 mm | residuals 2.5e-3..3.75e-3 |
| nist_30 | 1.969e-4 in | residuals 1.06e-6..3.15e-5 |
| nist_33 | 5.0e-3 mm | residuals 2.3e-4..3.4e-4 |

The transition is specifically `residual ≤ STEP source_tol` (the probe reported
`source_tol` = each model's declared uncertainty, and every accepted endpoint
residual lies in the `(1e-6, source_tol]` band), not a fallback or a changed
geometry path. No SourceInterval edge flipped to EvalRange under the larger
tolerance (verified: 40/40 stayed SourceInterval).

## 4. 00007667 (final, pinned state)

```
1 models, 7713 faces declared, 7703 rendered, 10 lost
  remaining 10 = pre-existing non-extruded set: 9 NoSurfaceProduced nurbs + 1
  MeshedToNothing cylinder #12154 (unchanged from baseline).
```

Seven recovered extruded faces, all rendered with the expected triangle counts:
#10340 102, #11866 100, #13844 100, #15760 109, #16752 49, #19018 44, #20292 108.

Complementary-arc witness (#10428, fresh build):
```
shared edge idx=30 verts=(23,22) er=(0.0,1.0)
root t_a=0.887738874 residual=1.28e-12
root t_b=0.171098596 residual=3.07e-16
C(0.5) on source arc [t_a -> t_b]: false
face 10428 rendered triangles=18
nearest rendered vertex to C(0.5): distance=3.5398e-2   (the chord distance)
rendered bbox diag=1.7851e-2
```
The complementary arc containing C(0.5) remains absent.

## 5. #1167 unchanged

`nist_13 #1167` (declared_face_index=61, bspline, `MeshedToNothing`, 0
triangles) is byte-identical to the `472bfd34` baseline ledger line. It is a
pre-existing non-R01 loss (surface produced but meshes to nothing — a different
mechanism from `EdgeTraversalUnresolved`), deliberately not fixed.

## 6. What changed (truck-fork b4cebf05)

- **truck-stepio**: parses the STEP geometric-uncertainty chain
  (`UNCERTAINTY_MEASURE_WITH_UNIT` → `GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT` →
  representation context → shape representation → solid → shell) and resolves
  it per shell entity id, in the file's native units. `to_compressed_shell`
  takes the shell id and stamps `CompressedShell.source_geometric_uncertainty`.
- **truck-topology**: `CompressedShell` gains
  `source_geometric_uncertainty: Option<f64>` (`#[serde(default)]`).
- **truck-meshalgo**: `cshell_tessellation_inner` computes
  `source_tolerance` = shell uncertainty (fallback `SOURCE_INCIDENCE_TOLERANCE`
  = 1e-6) and passes it to `establish_source_edge_traversal`.
  `carrier_closed` (lo≈hi seam equivalence / wrap authority) is decoupled from
  `source_tolerance` and stays at the fixed 1e-6. New module `source_edge.rs`.
- Focused tests: T1 approximate-incidence admission, T2 residual rejection,
  T3 large-tolerance parameter uniqueness, T4 open-carrier simple interval,
  T5 open carrier never wraps, T6 topological self-loop unchanged. truck-stepio
  uncertainty-parsing tests (5).
- Probes removed: `TRUCK_PROBE_R01DIAG`, `R01EDGE`/`R01FACE` logging,
  `TRUCK_PROBE_TRAVERSAL` in the unresolved arm.

## 7. Test status

- `cargo check --locked --all-targets`: clean.
- truck-meshalgo lib: **706 pass, 2 fail** — both `cone_topology_tests`
  (`duplicate_edge_creates_no_second_cdt_edge`, `test_parity_intersecting_constraints_rejected`),
  pre-existing at `472bfd34` (CDT code byte-identical; confirmed by stash test).
- P1 evaluator-domain, P2 singular-continuation, P3b sphere-recovery, and all
  source_edge tests pass.
- Pre-existing failures documented (all confirmed at `472bfd34` by pristine
  stash runs): truck-geometry `circle` (2) + b-spline/nurbs proptests,
  truck-stepio `input` (6: `table::read`, 4 b-spline proptests,
  `tessellate_shape` occt-cone), truck-stepio `io` (2: `oi`, `ioi`).
- `cargo fmt --all -- --check` is not a clean gate: committed truck-base /
  truck-geometry files carry pre-existing drift under both stable and nightly
  fmt (unstable rustfmt options in rustfmt.toml). The packet's files are free
  of committed-line churn; new code follows the repo's rustfmt.toml style.

## 8. Do-not-commit artifacts (still present, as before)

- `opencode.json`, `P1_SPLINE_PHASE2_HANDOFF.md`, scratch examples
  (`spline_edge_00007667_plane_witness.rs`, `spline_edge_00007667_tolreconcile.rs`).
- truck-fork `NIST_RECOVERY_HANDOFF*.md` (untracked, pre-existing).
- `R01_HANDOFF.md` itself is left untracked per the handoff-doc convention.

## 9. Re-run

```
cd C:\Users\stefa\look
cargo metadata --format-version 1   # truck-* must resolve under .cargo/git/checkouts/.../b4cebf0
Remove-Item target\release\examples\face_census.exe
cargo build --release --example face_census
# full NIST census
target\release\examples\face_census.exe --ledger $files   # 7901/7902, sole loss #1167
```
