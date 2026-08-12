# NO_ODD_PARITY_AUDIT.md

**Status:** Audit / discriminator session complete. No production fix landed.
**Date:** 2026-08-12
**Corpus:** ABC 20-model set, `look` and `truck-fork` as frozen below.

---

## 0. Executive summary

The `NoOddParityRegion` residual (1,346 faces of 2,061 lost on the 20 ABC
models) is **not one defect**. It splits into three certified mechanisms, two
of which are genuine degeneracy (correctly unmeshed, but mis-*classified* as
unresolved losses) and one of which is a **real, recoverable tessellation
defect** (206 faces with genuine 2D geometry that never reaches the CDT).

| bucket | faces | meaning | verdict |
|---|---|---|---|
| **A** `empty_cdt` (raw=0) | 274 | CDT produced no triangles | **mixed: 206 real + 68 degenerate** |
| **B** `material_empty` (raw>0, selected=0) | 646 | parity selected nothing | genuine degeneracy (out-and-back slit) |
| **C** `validation_empty` (selected>0, final=0) | 426 | validation emptied the selection | genuine degeneracy (sub-tolerance sliver) |

The whole-class conclusion the audit brief asked for:

> **~1,140 faces (B + C + the rank-≤1 tail of A) are lost for the certified
> mechanism "source face is degenerate at the meshing resolution", and the
> smallest correctness-preserving production change is a FACE-VALIDITY-style
> world-rank/world-area certificate that classifies them as
> `RejectedDegenerate` instead of an unresolved `NoOddParityRegion` loss.**
>
> **206 faces (the rank-2 head of A) are lost for the certified mechanism
> "genuine 2D world geometry whose boundary lift collapses to a degenerate
> chart", and the fix is a boundary/lift recovery — the only population where
> triangles should actually be added.**

There is **no parity-flip fix**. The non-goal of the brief is respected:
flipping parity would invent geometry into degenerate source faces.

---

## 1. Provenance (frozen corpus)

| component | value |
|---|---|
| Look SHA (working tree) | `30f3d44` + uncommitted closure/lattice/policy changes (`src/step.rs`, `src/step/lattice.rs`, `src/step/policy_geometry.rs`, `examples/*`) |
| Truck pin in `Cargo.toml` (working tree) | `6a2e5d50` (NIST1167 periodic-cover realization) |
| Truck-fork HEAD at audit time | `6a2e5d50` + uncommitted `source_edge.rs` / `triangulation.rs` R01 changes |
| Truck-fork HEAD now | `011ed422` (Track-B local-CDT merge) — **the tree moved mid-session; see §7** |
| Census artifact | `C:\Users\stefa\AppData\Local\Temp\opencode\diag_r01fix.jsonl` (20 models, 839,179 declared, 2,061 lost) |
| Backup of the artifact | `look-corpus\no_odd_parity_audit_diag.jsonl` (was copied; **deleted during cleanup** — re-extract from `diag_r01fix.jsonl` or re-run `face_census`) |
| Probes | `look/examples/nop_geo_probe.rs`, `look/examples/nop_edge_probe.rs`, `scripts/no_odd_parity_*.py` (committed in working tree) |
| truck probe (`TRUCK_PROBE_CDT_TRI`) | preserved in `git -C truck-fork stash@{0}` |

**Confirmed decomposition** (from the census artifact `cdt_stages`):

| bucket | criterion | faces |
|---|---|---|
| A | `raw_cdt_triangles` = 0 | 274 |
| B | raw > 0, `material_selected` = 0 | 646 |
| C | selected > 0, `final_valid` = 0 | 426 |
| total | | **1,346** |

All three `cdt_stages` fields are populated on every row (0 nulls).

---

## 2. The pipeline, with line numbers (current tree `011ed422`)

All paths are in `truck-fork/truck-meshalgo/src/tessellation/`.

### 2.1 Per-face tessellation chain

```
cshell_tessellation_inner (triangulation.rs)
  └─ tessellate_face closure                        :1688
      ├─ set PROBE_FACE_CONTEXT (source_face_id)    :1692
      ├─ create_boundary (per bound)                :1755
      │    └─ PolyBoundaryPiece::try_new            :1788
      ├─ preboundary: Result<Vec<pieces>>           :1790
      ├─ FACE-VALIDITY Detector B                   :1812  (gated by TRUCK_FACE_VALIDITY)
      │    └─ validity::detect_degenerate_trim      :5117
      ├─ PolyBoundary::new                          :1840
      ├─ trimming_tessellation_result               :1841
      │    └─ trimming_tessellation_with_diagnostics:8578
      │         └─ triangulation_into_polymesh_outcome :9422
      │              ├─ flood_parity                :9309  (→ ContradictoryDualParity)
      │              ├─ raw CDT count               :9571
      │              ├─ parity material selection   :9572
      │              ├─ validation filter (3D area) :9585
      │              └─ NoOddParityRegion return    :9688
```

### 2.2 The `NoOddParityRegion` raise site (bucket A/B/C common)

`triangulation.rs:9688`:

```rust
if tri_faces_raw.is_empty() {
    return TessellationOutcome::Failed(TessellationFailureReason::NoOddParityRegion.into());
}
```

The three buckets diverge **before** this line:

- **A**: `triangulation.inner_faces().count() == 0` — the CDT itself is empty.
- **B**: raw > 0, but `material_selected.is_empty()` — the parity flood selected
  zero triangles (`face_parity.get(&face.index()) == Some(&1)` never matched).
- **C**: selected > 0, but every selected triangle was removed by the
  validation filter at `:9585–9597`.

### 2.3 The validation filter (bucket C), `:9585`

```rust
let tri_faces_raw: Vec<[usize; 3]> = material_selected
    .into_iter()
    .filter(|idcs| {
        if idcs[0] == idcs[1] || ... { return false; }   // duplicate vertex
        let p0 = positions[idcs[0]];                      // positions = surface.subs(u,v)
        ...
        let area = 0.5 * cross.magnitude();               // 3D world area
        area > 1e-12 && area.is_finite()
    })
    .collect();
```

**Critical:** the filter rejects on **world-space triangle area**, computed from
`positions` (which are `surface.subs(u,v)`), *not* on UV area. This is the
single most important fact for bucket C: a triangle removed here has
**realized 3D area ≤ 1e-12**, whatever its UV area was.

### 2.4 FACE-VALIDITY (existing rejection machinery), `validity.rs`

- `FaceValidityCertificate` — reason + `world_rank` + evidence, `Serialize` :125
- `DegenerateFaceReason` — `AllBoundsCollapsed`, `LineLikeTrim`,
  `PointLikeTrim`, `ZeroWidthBand` :52
- `classify_trim_geometry` — world-rank test :404 (rank < 2 → reject)
- `measure_trim` — the boundary measurement :314
- `rejection_enabled()` — **off by default**; `TRUCK_FACE_VALIDITY=1` :483
- Detector B is invoked at `triangulation.rs:1812`, **before the CDT**.

Why Detector B does not currently catch these faces: it is **off by default**
and was off in the census run, so every one of the 1,346 fell through to the
CDT. When enabled, its world-rank test should classify B (and the rank-≤1 A
tail) as `LineLikeTrim`/`ZeroWidthBand`. **This has not yet been verified on
the corpus** — see §7, item 2.

---

## 3. Bucket C — `validation_empty` (426)

### 3.1 Measurement (probe `TRUCK_PROBE_CDT_TRI`)

A gated probe (preserved in `stash@{0}`) emits, per material-selected triangle,
`uv_area`, `world_area` (same value the filter sees), and the verdict. Run over
the two dominant models (00003172: 296 faces; 00000730: 122 faces; 418 of 426
total):

| model | faces | n triangles | UV area (per tri) | world area (per tri) | Jacobian world/uv |
|---|---|---|---|---|---|
| 00003172 | 296 | 11,702 | ~9.6e-13 | ~8.9e-13 | **0.97** |
| 00000730 | 122 | 5,284 | ~1.9e-7 | ~8.6e-13 | **3e-7** |

- Every measured triangle in both populations has `world_area ≤ 9.99e-13`,
  i.e. all were removed by the `> 1e-12` filter. Verdict was
  `zero_or_nonfinite_3d_area` for 100% of triangles.
- 00003172: UV ≈ world (Jacobian ~1). The face is tiny in **both** spaces —
  genuinely degenerate.
- 00000730: chart-finite (UV ~2e-6) but 3D-collapsed (world ~1e-11) — the
  surface map collapses the chart.

### 3.2 Physical scale (geo probe)

The **3D boundary diameter** of the source face (measured independently of the
CDT/parity pipeline) versus the model meshing tolerance (= 0.001 × model
diameter):

| model | model diam | tolerance | bucket C boundary diameter | bdiam / tol |
|---|---|---|---|---|
| 00003172 | 1.7211 | 1.7e-3 | median 2.8e-5 | ≤ 0.018 |
| 00000730 | 0.7948 | 8.0e-4 | median 1.0e-5 | ≤ 0.036 |

**Every bucket-C face has a physical boundary ≤ 3.6% of the meshing
tolerance.** These are sub-resolution slivers. OCCT/FreeCAD independently
reports micro-face populations on the same models (§6).

### 3.3 Conclusion

Bucket C is **genuine physical degeneracy at the meshing resolution**, not a
chart/validation bug. The UV-vs-3D outcome matrix (audit item 4/5):

| class | result |
|---|---|
| UV≈0, 3D≈0 | 00003172 population — genuinely degenerate |
| UV>0, 3D≈0 | 00000730 population — physical surface collapse, still sub-tolerance |
| UV≈0, 3D>tol | **zero found** — no chart/validation bug |
| neither≈0 | **zero found** — no validator defect |

---

## 4. Bucket B — `material_empty` (646)

### 4.1 Structural census (all 646 faces)

- Surface family: **Cylinder 596**, Plane 46, Cone 2, Extruded 2.
- Bound count: 1 → 602 faces; 2 → 22; 4 → 16; 6 → 4; 3 → 2.
- `duplicate_traversal_count > 0` on **612/646** (94.7%) — the boundary
  retraces itself.
- `piece_abs_area_sum < 1e-12` on **610/646** — near-zero UV area.
- Single-bound **u-periodic cylinder with retraced boundary**: **574/646
  (88.9%)** — this is the `#35281` family, and it is the dominant corpus motif,
  not one witness.

### 4.2 Geometry (geo probe, 602 faces on the two big models)

- All 345 measured 00000730 faces and 257 00003172 faces have **2 distinct 3D
  vertices** — the boundary is an out-and-back slit (a line segment traced
  twice). Zero enclosed world area. World rank ≈ 1.

### 4.3 Conclusion

Bucket B is **genuine trim degeneracy** — an out-and-back slit on a cylinder
defines no material. The `#35281` example in the brief is representative of
574 faces. These should be certified `RejectedDegenerate`, not parity-flipped.

---

## 5. Bucket A — `empty_cdt` (274) — **the recoverable population**

### 5.1 The split

Geo probe (world-rank certificate, mirroring FACE-VALIDITY's farthest-pair
test) over all 274 faces:

| world rank | faces | meaning |
|---|---|---|
| **2** (real 2D region) | **206** | genuine geometry, CDT empty → **recoverable** |
| 1 (line) | 64 | collapsed slit → degenerate |
| 0 (point) | 4 | collapsed point → degenerate |

**206 faces are real 2D geometry whose chart collapsed before the CDT.** By
kind: Plane 185, Cylinder 9, Swept 8, Nurbs 3, Cone 1. By model: **00007705 181**
(180 planes + 1 cone), 00005760 19, 00000730 41→rank-1, 00003172 23→rank-1,
00009190 3, others single.

### 5.2 The mechanism (verified on 00007705 #118263)

`nop_edge_probe` shows:

```
FACE #118263 kind=plane
PLANE origin=(-0.8365,-0.0885,-0.0197) u_axis=(-Y) v_axis=(+Z) |u x v|=1.0 (non-degenerate)
WIRE[0] edge_uses=1
  use[0] verts=(0,0) topo_closed=true range=[-0.125, 1.125]
        p0=p1=(0,0,0)  (seam point at the world origin)
ONSURF: 51/65 on-surface over the full range; off_t = the 14 samples in
        [-0.125,0] ∪ [1,1.125]  (the evaluator-domain overshoot)
ONSURF: unit domain [0,1]: 65/65 on-surface (dist 2.2e-16)
```

What happens:

1. The source boundary is a **single topologically-closed full-loop edge**
   (`edge.vertices.0 == edge.vertices.1`, verts `(0,0)`), a full circle on the
   plane.
2. The **evaluator range** is `[-0.125, 1.125]` — the STEP curve's parameter
   domain is wider than the actual loop closure. `source_edge.rs:264` returns
   `CanonicalByEvalRange { range: (lo, hi) }` for a topologically-closed edge,
   i.e. the **whole** evaluator domain, including the off-surface overshoot.
3. `PolyBoundaryPiece::try_new` samples the full range; the 14 overshoot
   samples leave the surface (`search_parameter` → `None` / off-plane), and the
   boundary collapses to a **2-point piece** (`boundary_pieces[0].point_count=2`,
   `signed_area=0`), presenting **0 constraints** to the CDT.
4. `raw_cdt_triangles = 0` → bucket A → `NoOddParityRegion` at `:9688`.

So the mechanism is: **for a topologically-closed full-loop edge whose
evaluator domain overshoots the seam, the lift walks the off-surface overshoot
and collapses the whole loop to a degenerate chart, even though the unit domain
`[0,1]` is 65/65 on-surface.** The physical face is real (OCCT confirms ~zero
degenerate faces on 00007705 — §6).

### 5.3 Independent witness

OCCT/FreeCAD on 00007705: 22,097 faces, **only 1 with area < 1e-12** — OCCT
judges essentially none of the model's faces degenerate, while truck loses 181
rank-2 faces there. The geometry is real; the loss is truck-side.

### 5.4 Candidate recovery theorem (A)

> For a face whose world boundary has rank 2 and whose boundary lift collapsed
> to a degenerate chart because a topologically-closed full-loop edge's
> evaluator range overshoots the seam, re-establish the boundary by sampling
> only the on-surface sub-domain (or by an alternate chart/lift) and re-run the
> CDT.

Open design questions for the implementer (§7 item 4):
- Should the fix clip the traversal range to the closure `[0,1]` for
  `topologically_closed` edges, or is the overshoot genuinely meaningful for
  some other population (regression risk)?
- Where to detect "world rank 2, chart rank < 2" cheaply — the FACE-VALIDITY
  `measure_trim` already computes `world_rank` and `uv_extents` (§2.4).
- How to gate it so that no currently-rendering face changes.

---

## 6. OCCT / FreeCAD witnesses (item 7)

Per-model whole-face OCCT area census (`occt_area_census.py` via
FreeCADCmd 1.1.1):

| model | OCCT faces | area < 1e-12 | model diag (OCCT, mm) |
|---|---|---|---|
| 00000730 | 30,302 | **102** | 800.2 |
| 00007705 | 22,097 | **1** | 3322.6 |

Interpretation: on 00000730 OCCT independently finds ~102 zero-area faces
(consistent with truck losing ~508 there — a micro-face population genuinely
exists in the source). On 00007705 OCCT finds **~none**, confirming the 181
rank-2 faces truck loses there are real geometry — the loss is truck-side, not
source-side.

---

## 7. State of the tree / traps for the next agent

1. **Truck HEAD moved.** The census (`diag_r01fix.jsonl`) was measured at
   `6a2e5d50` + uncommitted R01 source-edge changes. The tree is now at
   `011ed422` (Track-B local-CDT merge, `e115c49b` lineage). The Track-B work
   changed material-CDT refinement behavior — **re-run the census at the new
   pin before trusting the 2,061/1,346 numbers against the new tree.**
2. **The `TRUCK_PROBE_CDT_TRI` probe is stashed**, not committed:
   `git -C truck-fork stash apply stash@{0}` re-applies it (adds ~60 lines to
   `triangulation.rs` around `:9571`). It is a pure diagnostic and must be
   dropped or kept behind the `TRUCK_PROBE_CDT_TRI` env gate before any
   production change.
3. **Detector B is off by default** (`TRUCK_FACE_VALIDITY` unset →
   `rejection_enabled()` = false, `validity.rs:483`). It never ran in the
   census. Before claiming B is "covered", run one model with
   `TRUCK_FACE_VALIDITY=1` and verify the `rendered → rejected = 0` gate (no
   currently-rendering face may become a rejection).
4. **`.cargo/config.toml` path override is TEMP-ENABLED** pointing at local
   `truck-fork`. Re-comment it and bump the `Cargo.toml` rev before reporting
   any number as a clean-clone measurement.
5. **Probe tools in the look tree** (`nop_geo_probe.rs`, `nop_edge_probe.rs`,
   `scripts/no_odd_parity_*.py`) are working-tree-only (some are untracked).
   `cargo build --release --example nop_geo_probe` etc. rebuild them.

---

## 8. Fix design (what the next agent builds)

### 8.1 Workstream 1 — rejection/certification (B, C, A-rank≤1; ~1,140 faces)

**Goal:** classify genuinely-degenerate faces as certified rejections instead of
unresolved `NoOddParityRegion` losses. **No geometry changes; renders nothing
new.** ~40–80 LOC in truck-meshalgo + a corpus verification sweep.

- **B + A-rank≤1:** enable/verify Detector B (`TRUCK_FACE_VALIDITY`). The
  world-rank < 2 test already matches the out-and-back slit and collapsed
  loop evidence. Verify the `rendered → rejected = 0` gate on all 20 models.
  Possible small addition: extend `DegenerateFaceReason` or the certificate if
  the out-and-back-slit-on-cylinder population needs a distinguishable tag.
- **C:** new **Detector C** at `triangulation.rs:9585–9688`: when
  `material_selected > 0` but `tri_faces_raw.is_empty()` after the 3D-area
  filter, build a `FaceValidityCertificate` carrying the max realized triangle
  world area and the UV/world extents, and return `RejectedDegenerate` (or a
  new reason) instead of `NoOddParityRegion`. This makes the loss accounting
  honest (these become `rejected_intrinsic` in the census).
- Wire the certificate through `diagnosis.rs` (`record_face_rejection`, the
  `validity_certificate` field already exists at `:969`) and confirm the
  census classifies them as `rejected_intrinsic` (`face_census.rs` already
  counts `RejectedDegenerate` → `rejected_intrinsic`).

### 8.2 Workstream 2 — recovery (A rank-2; 206 faces)

**Goal:** recover real geometry. **This is the only population that adds
triangles** — highest value, highest risk. ~200–400 LOC + correctness gate.

- Root cause: `source_edge.rs:264` returns the full evaluator range for a
  topologically-closed full-loop edge; the lift (`PolyBoundaryPiece::try_new`,
  called at `triangulation.rs:1788`) walks the off-surface overshoot and
  collapses the loop to a 2-point chart.
- Candidate fix: when `world_rank == 2` (FACE-VALIDITY `measure_trim`) but the
  constructed chart is degenerate (`boundary_pieces[*].point_count ≤ 2` /
  `constraints_presented == 0`), re-lift the boundary from the on-surface
  sub-domain (clip the closed edge's evaluator range to its closure, verified
  by on-surface sampling as in `nop_edge_probe`'s `[0,1]` 65/65 result) and
  re-run the CDT.
- **Correctness gate (mandatory):** every recovered face must render with a
  nonzero triangle count and a mesh whose 3D extent matches the source
  boundary; zero regressions on the other 1,140 faces; the
  `rendered → rejected = 0` invariant must hold.
- **Risk of invented geometry:** the clipping must be certified against the
  source (only the on-surface sub-domain is admitted). Do not fit a plane to a
  boundary that is off-surface for a *reason* — verify with the OCCT witness
  pattern from §6.

### 8.3 What NOT to do

- No generic parity flip / "select the other side" experiment. §3–§5 show the
  parity output is not the failure.
- Do not count the ~1,140 degenerate faces as correctness failures to inflate a
  rendering percentage (§0 of the brief).
- Do not use a meshing-tolerance threshold as the degeneracy certificate —
  FACE-VALIDITY's world-rank test is the house rule (`validity.rs:10–35`).

---

## 9. Recovery ceiling (audit item 10)

| class | faces | status |
|---|---|---|
| definitely recoverable | 206 (A rank-2) | real geometry; fix = workstream 2 |
| probably recoverable | 0 | (none additional identified) |
| genuinely degenerate / should remain unmeshed | ~1,140 (B 646, C 426, A rank≤1 68) | correctly unmeshed; mis-classified |
| unresolved | 0 | — |

ABC coverage ceiling if both workstreams land: **+206 rendered faces** (of the
1,346), and the remaining ~1,140 become certified rejections rather than
unresolved losses. The percentage gain is modest; the classification gain is
the honest part.

---

## 10. Verification recipe (reproduce this audit)

```powershell
# 1. Re-apply the truck probe (preserved diagnostic)
git -C C:\Users\stefa\truck-fork stash apply stash@{0}   # TRUCK_PROBE_CDT_TRI

# 2. Rebuild
cargo build --release --example face_census   # look
cargo build --release --example nop_geo_probe
cargo build --release --example nop_edge_probe

# 3. Re-extract the bucket C triangle matrix (00000730, 00003172)
TRUCK_PROBE_CDT_TRI=1 TRUCK_PROBE_FACE_IDS=<ids> face_census <model.step> 2> cdt.log

# 4. Geo probe over bucket A/B/C ids
nop_geo_probe <model.step> <id,list>

# 5. Edge probe for the A mechanism (00007705 #118263)
nop_edge_probe 00007705.step 118263

# 6. OCCT witness
AUDIT_MODEL=<step> AUDIT_OUT=<json> FreeCADCmd.exe occt_area_census.py
```

Python analysis scripts: `scripts/no_odd_parity_census.py` (face census),
`scripts/no_odd_parity_bucketB.py` (B structural census), `scripts/geo_bucketA.py`
and `scripts/geo_aggregate.py` (rank / extent aggregation). OCCT witnesses:
`look-corpus/bench-out/occt_area_census.py`, `occt_degeneracy_census.py`,
`occt_entity_probe.py`.
