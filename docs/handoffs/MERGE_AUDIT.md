# MERGE_AUDIT — Look / Look-Accuracy Integration

**Date:** 2026-08-11
**Status:** AUDIT ONLY — no worktree was modified, merged, rebased, or reset.
All git reads were run against the existing worktrees; the only filesystem
change was deleting regenerable `C:\Users\stefa\look\target` build artifacts
at the operator's request. Nothing in either repo was committed.

This document tells the next implementation session exactly what to integrate,
in what order, and what must still measure identically when the merge lands.

---

## 1. Canonical vs accuracy provenance

Both `look` worktrees are linked worktrees of the **same** repository
(`origin https://github.com/stefangolas/look.git`). They share one object
store; they are not two independent clones.

### Canonical — `C:\Users\stefa\look`

| item | value |
|---|---|
| branch | `integration/formal-atlas-wave-2` |
| HEAD | `30f3d4405dd843f3d12f30b164c79b17bf06d7b6` |
| vs origin | ahead 15 (`origin/integration/formal-atlas-wave-2` = `f5e9a88` WAVE-4) |
| tracked tree | **NOT clean** — 16 modified, 27 untracked (see §7) |
| committed truck pin | `b4cebf05` (R01 source-edge traversal) |
| worktree truck pin | `6a2e5d50` + TEMP-ENABLED `paths` override to `../truck-fork/truck-*` (uncommitted) |

### Accuracy — `C:\Users\stefa\look-accuracy`

| item | value |
|---|---|
| branch | `accuracy/certified-cell-triangulation` |
| HEAD | `e4682d92baea81b177e0a6f2317f6626db710642` |
| vs origin | **not pushed** (local only) |
| tracked tree | clean (only untracked handoffs/probes, see §7) |
| committed truck pin | `e115c49b` (Track-B local-CDT refinement) |
| path override | re-commented; disabled — clean clone reproducible |

### Merge base and intended canonical branch

```
merge-base(30f3d44, e4682d9) = ac201c70
  (ac201c70 = "Pin truck 472bfd34: certified sphere pole collapse, P3b H4 …")
```

The **intended canonical integration branch is `integration/formal-atlas-wave-2`**,
not `main`. `main` (`a358d70`) is behind the merge base (it pins truck
`79eaaf36` and carries none of the R01/periodic-cover lineage). Both feature
branches sit on top of `ac201c70`; everything below `ac201c70` is shared
history. Only 3 commits are canonical-only and 1 is accuracy-only (see §3).

---

## 2. Truck dependency provenance

### Canonical Look → Truck

| layer | rev | origin |
|---|---|---|
| committed `Cargo.toml`/`Cargo.lock` | `b4cebf05` | `R01 source-edge traversal` |
| worktree `Cargo.toml`/`Cargo.lock` (uncommitted) | `6a2e5d50` | `A–F periodic-cover realization` |
| `.cargo/config.toml` (uncommitted) | TEMP-ENABLED | `paths = ["../truck-fork/truck-*", …]` |
| truck-fork worktree HEAD | `6a2e5d50` | branch `feature/cone-apex-lift-recovery` (pushed) |
| truck-fork worktree uncommitted | `source_edge.rs` + `triangulation.rs` | **R01-REG repair (uncommitted)** |

The canonical worktree currently measures a tree that is **`6a2e5d50` + the
uncommitted R01-REG repair** (via the TEMP-ENABLED path override). Any census
number taken today from `C:\Users\stefa\look` is a measurement of that hybrid,
**not** of anything a clean clone builds. The committed canonical HEAD pins
only `b4cebf05`.

### Accuracy Look → Truck

| layer | rev | origin |
|---|---|---|
| committed `Cargo.toml`/`Cargo.lock` | `e115c49b` | `Track-B local-CDT refinement` |
| `.cargo/config.toml` | re-commented | `paths = ["C:/Users/stefa/truck-accuracy/truck-*", …]` (disabled) |
| truck-accuracy worktree HEAD | `e115c49b` | branch `accuracy/certified-cell-triangulation` (pushed) |
| truck-accuracy worktree | clean | — |

Accuracy pins `e115c49b` by exact revision with no active override; a clean
clone reproduces it. `cargo tree -p truck-meshalgo -i --prefix none` resolves
`truck-meshalgo v0.4.0 (…truck?rev=e115c49b#e115c49b)`.

### The two Truck lineages share a common base

```
truck 472bfd34            ← certified sphere pole collapse + P3b
 ├── b4cebf05             ← R01 source-edge traversal (canonical, pushed)
 │    └── 6a2e5d50        ← A–F periodic-cover realization (canonical, pushed)
 │         └── [UNCOMMITTED R01-REG caller_tol repair]
 └── 34a9d087 … e115c49b  ← Track-B (6 commits, pushed) [NO R01, NO A–F]
```

`merge-base(6a2e5d50, e115c49b) = 472bfd34`. Neither lineage contains the
other: `b4cebf05`/`6a2e5d50` are **not** ancestors of `e115c49b` and vice
versa. In particular:

- `source_edge.rs` exists at `b4cebf05`/`6a2e5d50`, **absent** at `e115c49b`.
- `CompressedShell.source_geometric_uncertainty` and the id-aware
  `to_compressed_shell(shell_id, …)` API exist only on the canonical lineage
  (`b4cebf05`+); the Track-B lineage keeps the old signatures.
- The **repaired R01 (`caller_tol`) is committed nowhere**. `git log --all -S
  'caller_tol'` is empty; it lives only as uncommitted changes in the
  truck-fork worktree. It must be committed before or during the Truck merge.

---

## 3. Commit-set differences (merge-base aware)

### Commits reachable from accuracy but not canonical — 1

| commit | subject | classification |
|---|---|---|
| `e4682d9` | truck: pin Track-B local-CDT refinement at `e115c49b`, drop local path override | **must integrate** (dependency/config + one census diagnostic) |

`e4682d9` changes only: `.cargo/config.toml` (re-comment + re-point at
truck-accuracy), `Cargo.lock`, `Cargo.toml` (pin `e115c49b`), and
`examples/face_census.rs` (+3 env-gated `MODEL_DIAMETER` diagnostic lines).
All Look source beyond that is untouched; Track-B is a Truck-side refinement.

### Commits reachable from canonical but not accuracy — 3

| commit | subject | classification |
|---|---|---|
| `1482e87` | Record P1 regression epistemic sweep (handoff + scripts + `spline_edge_epistemic_compact.rs`) | **diagnostic/tooling + handoff** (scripts are reusable census tooling) |
| `2da6f30` | Resolve 00007667 shared-edge exception probes (`spline_edge_00007667_*`, `canonical_probe`) | **diagnostic/probe only** + handoff |
| `30f3d44` | Pin truck `b4cebf05` (R01) + `src/step.rs`/`policy_geometry.rs` API updates + example call-site updates | **must integrate** (R01 Look-side API alignment) |

### Symmetric note

There are **no obsolete experiments or redundant duplicates** on either side;
the diff sets are small and each commit carries one purpose.

---

## 4. Source-level diff inventory (from merge base `ac201c70`)

Subsystem classification of every file that differs on either lineage
(committed only; worktree-uncommitted A–F changes are listed separately in §5
and §7).

### Accuracy-only (from `ac201c70` → `e4682d9`)

| file | subsystem | semantic? |
|---|---|---|
| `.cargo/config.toml` | Cargo/dependency config | provenance only |
| `Cargo.lock` | Cargo/dependency config | provenance only |
| `Cargo.toml` | Cargo/dependency config | provenance only (pin `e115c49b`) |
| `examples/face_census.rs` | census tooling | env-gated diameter diagnostic (3 lines) |

### Canonical-only committed (from `ac201c70` → `30f3d44`)

| file | subsystem | semantic? |
|---|---|---|
| `.cargo/config.toml` | Cargo/dependency config | re-comment override + R01 note |
| `Cargo.lock` | Cargo/dependency config | pin `b4cebf05` |
| `Cargo.toml` | Cargo/dependency config | pin `b4cebf05` |
| `src/step.rs` | STEP import | **yes** — id-aware `to_compressed_shell(*id, …)` |
| `src/step/policy_geometry.rs` | boundary realization | **yes** — carry `source_geometric_uncertainty` through `wrap_shell` |
| `examples/` (18 files) | examples/census tooling | call-site updates for the R01 API |
| `scripts/` (5 new ps1) | examples/census tooling | reusable census sweep/compare scripts |
| `EPISTEMIC_SWEEP_HANDOFF.md`, `P1_SPLINE_PHASE1_VERDICT.md` | handoff/docs | handoff |
| `examples/spline_edge_00007667_*` (5 new) + `canonical_probe` + `epistemic_compact` | accuracy diagnostics | probes only |

### Both-side overlap candidates (see §5)

| file | canonical | accuracy | likely conflict? |
|---|---|---|---|
| `.cargo/config.toml` | re-comment + truck-fork | re-comment + truck-accuracy | **yes (trivial, provenance)** |
| `Cargo.lock` | pin `b4cebf05` | pin `e115c49b` | **yes (trivial, final SHA)** |
| `Cargo.toml` | pin `b4cebf05` | pin `e115c49b` | **yes (trivial, final SHA)** |
| `examples/face_census.rs` | R01 call-site updates | diameter diagnostic | **yes (small, mechanical)** |

The `src/step.rs` and `src/step/policy_geometry.rs` changes are **canonical-only
committed**; the accuracy branch is based on the pre-R01 API and carries the old
signatures, so those files differ between the two HEADs (old vs id-aware
`to_compressed_shell`), but only the canonical side *edited* them relative to
the merge base.

---

## 5. Genuine overlapping edits (symbol level)

### Look-side

No shared file has a *semantic* conflict on the Look side. All four both-side
files are Cargo provenance or a census diagnostic:

- `.cargo/config.toml`, `Cargo.toml`, `Cargo.lock`: **same configuration
  value, different pins** → the merged value is the **final Truck SHA**; the
  `paths` block must end re-commented and pointing at nothing (or the standard
  disabled comment). Resolution is deterministic, confidence **high**.
- `examples/face_census.rs`: canonical updated call sites for the id-aware API;
  accuracy added a `TRUCK_CENSUS_DIAMETER` env-gated `eprintln`. **Orthogonal,
  mechanically mergeable** — take both. Confidence **high**.

### Truck-side

This is where the real overlap lives, in two files.

#### `truck-meshalgo/src/tessellation/triangulation.rs` — **semantic conflict**

Predicted by `git merge-tree 472bfd34 6a2e5d50 e115c49b`: exactly **one**
conflict region.

| symbol | canonical (`6a2e5d50`) | Track-B (`e115c49b`) | verdict |
|---|---|---|---|
| `PolyBoundary::new` | `let join_policy = primary_two_loop_join_policy(&pieces, lattice); Self::new_with_join(…, join_policy)` | unconditional `Self::new_with_join(…, TwoLoopJoinPolicy::DeckConsistent)` | **same code path, conflicting selection** — see below |

Everything else in `triangulation.rs` auto-merges cleanly (verified by
`merge-tree`): the merged blob retains `align_two_loop_phase`,
`trimming_tessellation_with_refinement`, `REFINE_SUPPORT_CELL`, `EstablishedEdge`,
`EdgeTraversalUnresolved`, `source_tolerance`, and both test suites.

**Resolution.** Keep the canonical structural classifier
`primary_two_loop_join_policy` (the A–F handoff explicitly forbids making *all*
two-loop joins `DeckConsistent` without a structural classifier) **and** keep
Track-B's `align_two_loop_phase` phase-correspondence (which the canonical
lineage does not contain). The canonical classifier already routes the
certified structural deck-pair class through `DeckConsistent`, so the two
implementations agree on the population that Track-B's OCCT gates exercise;
Track-B adds the phase alignment *inside* the deck-consistent arm. Recommended
merge result for `PolyBoundary::new`:

```rust
let join_policy = primary_two_loop_join_policy(&pieces, lattice);
Self::new_with_join(pieces, surface, tol, lattice, join_policy).0
```

with `align_two_loop_phase` retained inside `new_with_join`. Confidence
**medium-high**; validate with the T1–T7 + bow-tie tests and the OCCT gates
(§9, §10).

#### `truck-meshalgo/src/tessellation/diagnosis.rs` — **auto-merges**

Canonical edits `derive_projection_status`/`derive_arr_signature` (add
`EdgeTraversalUnresolved` classification); Track-B edits `TwoLoopJoinRecord`
doc comments and the `loop1_reversed` doc. Different symbols →
**orthogonal, auto-merges** (verified by `merge-tree`).

#### `truck-fork` uncommitted R01-REG repair

`source_edge.rs` (+`caller_tol` parameter, `effective_source_tol` floor, T7–T9
tests) and a one-line `triangulation.rs` call-site change. Track-B does **not**
touch `source_edge.rs` and its `triangulation.rs` lineage has no `caller_tol`,
so the repair is **orthogonal** to Track-B; it must be committed first (it is
the "repaired R01": ABC ≈ 2,061 lost, 25,548 accidental
`EdgeTraversalUnresolved` eliminated).

---

## 6. Does Track-B already contain older periodic/join work?

Track-B's six commits, classified against the canonical lineage:

| Track-B commit | canonical equivalent? | verdict |
|---|---|---|
| `34a9d087` final u-column v-grid links | no (canonical has no grid-wiring change) | **needed** |
| `ead8ebcc` constrain every material sub-segment | no | **needed** |
| `012311f5` keep windows grid wiring for seam boundaries | no | **needed** |
| `3a885f66` deck-equation two-loop join on primary path | **yes** — canonical `primary_two_loop_join_policy` implements the same primary-path DeckConsistent routing, but gated by a structural classifier | **superseded/overlapping** — keep canonical classifier, keep Track-B phase alignment |
| `b862eed0` full-period loop phase alignment | no (`align_two_loop_phase` absent from canonical) | **needed** |
| `e115c49b` local CDT refinement | no | **needed** |

Canonical's `primary_two_loop_join_policy` (A–F, committed at `6a2e5d50`) is
the *equivalent of* Track-B's `3a885f66` primary-path deck-consistent join but
deliberately narrower. **Do not cherry-pick `3a885f66` as-is**; it would
unconditionally route every two-loop face through `DeckConsistent`, which the
A–F handoff's stop-condition rejects. Merge the phase-alignment behavior
(`b862eed0`) and keep the classifier.

---

## 7. Production changes vs worktree debris

### Canonical worktree (`C:\Users\stefa\look`) — uncommitted

**Must land (A–F periodic-cover Look-side work + config):**
- `src/step/lattice.rs` — `SplineAxisClosure`, `spline_closure_map`,
  `lattice_of_with_closure`, `spline_lattice`, `spline_seam_compatible`,
  `spline_quotient_axes` (+ source-closure tests)
- `src/step/policy_geometry.rs` — `QuotientAxis`, `PolicySurface::with_closure`,
  `native_uv`, `accept_inverse_result`, `quotient_parameter_division`,
  `map_division_to_cover`, `wrap_shell_with_closure` (+ quotient tests)
- `src/lib.rs`, `src/step.rs` — closure-map wiring, `wrap_shell_with_closure`
- `examples/face_census.rs` — closure-map census path
- `Cargo.toml`/`Cargo.lock` — pin `6a2e5d50` (will become final Truck SHA)
- `.cargo/config.toml` — TEMP-ENABLED override → must be **re-commented** before
  any census number is reported as production

**Useful regression/tooling (consider committing):**
- `scripts/ledger_diff.py`, `scripts/ledger_keyed_diff.py` — ledger comparison
  (complements the committed ps1 census tooling)

**Temporary diagnostics (do NOT commit):**
- `examples/bbox_probe.rs`, `examples/r01_edge_probe.rs`,
  `examples/source_tol_tally.rs`, `examples/stageb_probe.rs`,
  `examples/nist1167_*` (13 probes), `examples/nist1169_mesh.rs`,
  `examples/spline_edge_00007667_plane_witness.rs`,
  `examples/spline_edge_00007667_tolreconcile.rs`
- handoff MDs: `NIST1167_*.md` (3), `P1_SPLINE_PHASE2_HANDOFF.md`,
  `R01_HANDOFF.md`
- `opencode.json` (agent config, personal)

`activation_census.json` is **not present** anywhere in either Look worktree or
either Truck worktree; if it reappears, treat it as a generated diagnostic and
do not commit it.

### Accuracy worktree (`C:\Users\stefa\look-accuracy`) — untracked

**Useful permanent census tooling (consider committing):**
- `examples/final_mesh_accuracy.rs`, `examples/mesh_accuracy_census.rs`,
  `examples/mesh_volume_probe.rs` — the OCCT/accuracy gate binaries

**Handoff/docs (do NOT commit):**
- `ACCURACY_BOWTIE_HANDOFF.md`, `ACCURACY_HANDOFF.md`,
  `ACCURACY_PHASE_FINDINGS.md`, `ACCURACY_TRACKB_FINAL.md`

**Unrelated debris:**
- `20260108_ChoryLab_Jerkat/` — microscopy data, **not** part of the project.

### Truck-fork worktree (`C:\Users\stefa\truck-fork`) — uncommitted

- `truck-meshalgo/src/tessellation/source_edge.rs` +
  `truck-meshalgo/src/tessellation/triangulation.rs` — **the repaired R01
  (R01-REG). This is production work that must be committed**, not debris.
- `NIST_RECOVERY_HANDOFF*.md` (3) — handoff docs, untracked, do not commit.

### Truck-accuracy worktree — clean.

---

## 8. Truck-first integration plan

Both Truck lineages diverge from `472bfd34`, so the Truck merge is a genuine
two-way merge (verified with `git merge-tree`), not a rebase. Minimal order:

```
truck 472bfd34 (canonical base)
   ↓ (already) b4cebf05   R01 source-edge traversal            [pushed]
   ↓ (already) 6a2e5d50   A–F periodic-cover realization       [pushed]
   ↓ COMMIT the uncommitted R01-REG repair (caller_tol)        [NEW]
   ↓ MERGE accuracy/certified-cell-triangulation e115c49b
       → one triangulation.rs conflict (PolyBoundary::new)
       → keep primary_two_loop_join_policy + align_two_loop_phase
   ↓ final Truck integration SHA  (T_FINAL)
```

then:

```
look integration/formal-atlas-wave-2 (30f3d44)
   ↓ land uncommitted A–F Look-side work (§7 "must land")
   ↓ merge accuracy branch e4682d9 (diameter diagnostic)
   ↓ pin Cargo.toml/Cargo.lock to T_FINAL; re-comment .cargo/config.toml
   ↓ final Look integration SHA
```

The Look-side merge itself (30f3d44 ↔ e4682d9 at base ac201c70) conflicts only
on the 4 provenance files (§5) — all mechanical. `src/step.rs` /
`policy_geometry.rs` automatically take the canonical (R01/id-aware) side
because only canonical edited them.

---

## 9. Semantic invariants that must survive integration

### Source-edge / R01

| gate | command / artifact | expected after integration |
|---|---|---|
| ABC loss class | `face_census --ledger` over `look-corpus\abc` (20 models, 839,179 declared) | ≈ 2,061 lost, **not** ~27k; no mass `EdgeTraversalUnresolved` population |
| 00007667 complementary arc | `spline_edge_00007667_compare` / `_probe` | 7703/7713 rendered, complementary arc `C(0.5)` absent, `#10428` witness intact |

### Periodic A–F

| gate | command / artifact | expected after integration |
|---|---|---|
| NIST full census | `face_census --ledger` over 33 NIST `.stp` files | **7902 / 7902 / 0** (no `#1167` loss) |
| `#1169` | `nist1167_production_mesh` / `nist1167_quotient_counterfactual` on `nist_ctc_02_asme1_ap203.stp` | not silently wrong (area ≈ 8,322, no 4×-area mesh) |

### Track-B

| gate | command / artifact | expected after integration |
|---|---|---|
| ctc_01 OCCT | `mesh_accuracy_census --json … nist_ctc_01_asme1_ap203.stp GT.bin GT.meta.json` | `#617/#619/#621` ≈ 0.76 mm, `#622` 0.662, `#574` 0.624, `#560` 0.689; 12098 → 12298 tris |
| ftc_08 OCCT | same tool on `nist_ftc_08_asme1_ap242-e1-tg.stp` | `#6001` 0.232 mm; `#6049` **unchanged** 15 tris / 1.917 mm |
| structural tests | `cargo test --locked --lib -p truck-meshalgo` | **T1–T7 PASS** (also the bow-tie orientation tests) |

---

## 10. Predicted merge conflicts

### Look (`30f3d44` ↔ `e4682d9`)

| file | canonical change | accuracy change | resolution | confidence |
|---|---|---|---|---|
| `.cargo/config.toml` | re-comment truck-fork note | re-comment truck-accuracy note | re-comment; final text points at T_FINAL, override off | high |
| `Cargo.toml` | pin `b4cebf05` | pin `e115c49b` | pin T_FINAL | high |
| `Cargo.lock` | truck rev `b4cebf05` | truck rev `e115c49b` | `cargo update -p` to T_FINAL | high |
| `examples/face_census.rs` | R01 id-aware call sites | `TRUCK_CENSUS_DIAMETER` diagnostic | keep both | high |

### Truck (`6a2e5d50` ↔ `e115c49b`)

| file / symbol | canonical change | Track-B change | resolution | confidence | validation |
|---|---|---|---|---|---|
| `triangulation.rs` / `PolyBoundary::new` | `primary_two_loop_join_policy` classifier | unconditional `DeckConsistent` | keep classifier + `align_two_loop_phase` | medium-high | T1–T7, bow-tie, ctc_01 OCCT, NIST 7902/7902/0 |

No other Truck file conflicts (`diagnosis.rs` auto-merges; `source_edge.rs`
is canonical-only; Track-B has no `source_geometric_uncertainty`).

---

## 11. Exact integration sequence

**Phase 0 — precondition (truck-fork):**
1. Commit the R01-REG repair in `truck-fork` (`source_edge.rs`,
   `triangulation.rs`) on `feature/cone-apex-lift-recovery`; message documents
   "repaired R01: caller-tol endpoint incidence + source-tol floor".
2. `cargo fmt -p truck-meshalgo --check`; `cargo check --locked --all-targets`.

**Phase 1 — Truck merge (feature/cone-apex-lift-recovery):**
3. `git merge accuracy/certified-cell-triangulation` (`e115c49b`).
4. Resolve the single `triangulation.rs::PolyBoundary::new` conflict per §5
   (keep `primary_two_loop_join_policy`, keep `align_two_loop_phase`).
5. `cargo fmt -p truck-meshalgo --check`; `cargo check --locked --all-targets`;
   `cargo test --locked --lib -p truck-meshalgo` → T1–T7 green (2 known
   pre-existing `cone_topology_tests` failures are baseline debt, unchanged).
6. Focused gates: `source_edge` T1–T9, bow-tie orientation tests,
   periodic-cap tests.
7. `git push` → record `T_FINAL`.

**Phase 2 — Look canonical (integration/formal-atlas-wave-2):**
8. Commit the uncommitted A–F Look-side work (§7 "must land") as **distinct
   commits** (source-closure provenance; quotient lattice; PolicySurface
   quotient adapter; quotient subdivision; inverse-acceptance gate), not one
   opaque `fix #1167` commit.
9. `git merge accuracy/certified-cell-triangulation` (`e4682d9`); resolve the
   4 provenance-file conflicts (§10); keep the `face_census` diameter diagnostic.
10. Bump `Cargo.toml` + `Cargo.lock` to `T_FINAL`; **re-comment**
    `.cargo/config.toml`; `cargo update`; confirm `cargo tree` resolves to
    `T_FINAL` (no override active).
11. Clean build: `cargo check --locked --all-targets`; `cargo test --locked
    --all-targets`.

**Phase 3 — fingerprints:**
12. NIST full census: **7902 / 7902 / 0**.
13. ABC census: ≈ 2,061 lost, no mass `EdgeTraversalUnresolved`; 00007667
    7703/7713 with complementary-arc witness preserved.
14. OCCT gates: ctc_01 `#617/#619/#621` ≈ 0.76 mm, `#622` 0.662, `#574` 0.624,
    `#560` 0.689; ftc_08 `#6001` 0.232, `#6049` 1.917 mm / 15 tris unchanged.
15. `cargo fmt --all -- --check`; commit; `git push`.

Do not squash the R01 repair, the A–F stack, and Track-B into one commit; each
carries a distinct correctness theorem with its own corpus evidence.

---

## 12. GO / NO-GO assessment

### Expected final fingerprints

| corpus | expected |
|---|---|
| NIST | 7902 / 7902 / 0 |
| ABC | ≈ 2,061 lost (R01 repaired class, not ~27k) |
| 00007667 | 7703/7713, complementary arc absent |
| ctc_01 | `#617/#619/#621` ≈ 0.76 mm, `#622` 0.662, `#574` 0.624, `#560` 0.689 |
| ftc_08 | `#6001` 0.232 mm, `#6049` unchanged (1.917 mm / 15 tris) |
| structural | T1–T7 + bow-tie + source_edge T1–T9 green |

### Assessment

**GO**, with two non-blocking preconditions that the integration session must
satisfy before the merge runs:

1. **Commit the R01-REG repair** in truck-fork first. It is uncommitted today,
   so any "canonical" tree measured from the current worktree is not a clean
   clone yet.
2. **Confirm the `PolyBoundary::new` resolution** (canonical classifier +
   Track-B phase alignment) passes the Track-B OCCT gates. The merge itself is
   fully mechanical and the only real decision point is that one symbol.

All other differences are either provenance (pins/overrides, trivially
resolvable to `T_FINAL`) or intentionally retained diagnostics. The final tree
= repaired R01 + A–F periodic-cover + Track-B accuracy refinement, reachable
by the sequence in §11. Nothing here requires redesigning A–F or re-deriving
Track-B.
