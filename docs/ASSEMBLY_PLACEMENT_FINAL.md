# ASSEMBLY_PLACEMENT_FINAL.md

Stop-condition handoff for **STEP assembly rendering end-to-end** (`core_xy.step`),
replacing the prior `ASSEMBLY_PLACEMENT_AUDIT.md` stop condition. The root cause
from the audit is fixed in Truck and consumed by Look.

---

## Provenance

| Repository | Final commit | Working tree |
|---|---|---|
| `C:\Users\stefa\look` | `d2e0fcd0b74086080f0273a6527741d582f6e348` | `M src/cli.rs`, `M src/ui.rs` (unrelated `--gui` HTML-viewer work, untouched), plus pre-existing untracked scratch |
| `C:\Users\stefa\truck-fork` | `7a0174436c31d7cc88541e5f46b23618efe1a9b9` | untracked `NIST_RECOVERY_HANDOFF*.md` only |

Look resolves all truck crates (direct deps + `[patch.crates-io]`) at
**`7a017443`**, pushed to `stefangolas/truck` branch
`feature/cone-apex-lift-recovery`. `.cargo/config.toml` local path override is
**disabled/re-commented**; a clean clone builds the pushed SHA. `ruststep`
pinned at `67f1f7c`.

Truck commit history for this change:

```text
4731551b stepio: attach assembly geometry through representation relationships
f073773e stepio: carry assembly definition source shell ids
7a017443 stepio: build the assembly graph in source entity order
```

---

## Truck semantic change

Files: `truck-stepio/src/in/convert.rs` (plus `tests/io/ioi.rs`,
`examples/step-to-mesh.rs` destructuring updates).

1. **`Table::product_node_entity`** (convert.rs) — after resolving the SDR's
   `used_representation` (the placement/frame representation), it now also walks
   every explicit `SHAPE_REPRESENTATION_RELATIONSHIP` whose `rep_1` is that used
   representation, resolves `rep_2` (the
   `ADVANCED_BREP_SHAPE_REPRESENTATION`), converts its items via
   `ProductShape::try_from_index`, and appends them to the node's shape vector.
   Placement and geometry are both preserved in the node's `Vec<ProductShape>`
   (`[Matrix, Solid]` for the witness's part nodes). The relationship is the
   source-authoritative path:

   ```text
   SDR → placement SHAPE_REPRESENTATION → SHAPE_REPRESENTATION_RELATIONSHIP
        → ADVANCED_BREP_SHAPE_REPRESENTATION → MANIFOLD_SOLID_BREP → shell
   ```

   No heuristics: no proximity/name/order/bbox/entity-id selection. A product
   with no geometry relationship keeps its placement-only shape (T3).

2. **`ProductShape`** — the `Solid`/`Shells` variants now carry the source shell
   entity ids they were converted from (outer first, then voids), so a consumer
   can re-derive the DIAG-002 per-shell conversion-loss stream without
   re-resolving the assembly or re-walking the source relationship.

3. **`Table::step_assy`** — iterates `product_definition_shape` in entity-id
   order so graph node numbering is a function of the document, not of HashMap
   iteration order (determinism invariant for downstream geometry/occurrence
   indexing).

`Path::matrix()` and `ItemDefinedTransformation → Matrix4` (`mat2 · inverse(mat1)`,
root→leaf fold) are **untouched**; the already-certified transforms are unchanged.

Focused regression tests: `step_assy_geometry_tests` in convert.rs —
T1 direct geometry, T2 placement + linked geometry (both retained), T3 no
invented attachment, T4 repeated definition → one node / distinct occurrences,
plus a source-shell-ids test and a world-transform-vs-source-frame test. The
synthetic fixture is committed with the tests, so `core_xy.step` is not the only
permanent regression.

---

## `core_xy.step` accounting

| Metric | Value |
|---|---|
| Source `NEXT_ASSEMBLY_USAGE_OCCURRENCE` | 175 |
| Truck occurrence paths (non-trivial root paths) | 175 |
| Product nodes (assembly graph) | 95 |
| Geometry-bearing nodes (`Matrix + Solid`) | 77 |
| Matrix-only (subassembly/root) nodes | 18 |
| Look unique geometries (tessellated) | 76 |
| Look instances | 155 |
| Hierarchy depth | ≤ 4 (max path 4 nodes) |
| Scene bounds (rendered) | X[-0.254, 0.254] Y[-0.224, 0.3097] Z[-0, 0.5639] m |
| Triangles / vertices | 666865 / 2000595 (identical to the old flat path) |

**175 NAUO vs 155 instances explanation** (explicit graph semantics, not a blind
`== 175` gate):

- 175 NAUO edges ↔ 175 non-trivial root paths (the graph is a layered DAG where
  every parent has one root path, so paths ↔ edges 1:1; the 176th `paths_iter`
  item is the trivial root path). Repeated definitions appear once per incoming
  occurrence edge (e.g. `corner_bracket` 16, `din_rail_clip` 12).
- 17 of the 18 matrix-only nodes are occurrence targets (the root `main` is the
  18th and has 0 incoming occurrences). Those 17 occurrence paths terminate at
  definitions with no geometry of their own and correctly render no instance —
  their children are separate occurrences.
- 1 geometry-bearing definition (`ball`, a sphere, 3 occurrences) tessellates to
  nothing through the current policy. This is **not a regression**: the old flat
  path produced the identical triangle count (666865) without it, and both paths
  report the same per-face loss census. It is a pre-existing definition whose
  sphere fails the current triangulation route.
- Renderable occurrences: 175 − 17 (subassembly targets) − 3 (ball occurrences)
  = 155 instances. Unique geometry = 77 solids − ball = 76.

---

## Acceptance witnesses

### A. `pi_4_enclosure_top`

- NAUO #666, IDTF #1016.
- Certified translation `(-0.0262604611974987, 0.24232, 0.316053894888206)`.
- Truck `Path::matrix()` translation: `(-0.026260, 0.242320, 0.316054)` —
  matches to print precision.
- Definition-local bbox (from the audit walk of shell #97490):
  X[-0.047, 0.047] Y[-0.0325, 0.040] Z[0.00238, 0.023].
- Predicted world bbox X[-0.073, 0.021] Y[0.244, 0.265] Z[0.284, 0.356] lies
  inside the rendered scene bounds; the probe confirms the certified transform
  is what is applied.

### B. `motor` (repeated definition)

- Child PD #175851; two certified occurrences: NAUO #697 → x = −0.20289,
  NAUO #725 → x = +0.20289 (y = 0.181, z = 0.4468).
- The probe reports both world transforms exactly, `geometry=true` for both, and
  a single shared definition node (one geometry index, two occurrences, two
  distinct instance transforms).
- `motor` also appears nested (depth-4 occurrence, T = (-0.017803, 0.040097,
  0.480000)).

### C. Nested composition

The depth-4 `motor` path and the synthetic Look fixture both satisfy
`T_world(child) = T_world(parent) * T_local(child)`. The committed
`tests/assembly.rs` `nested_occurrence_composes_parent_and_local` asserts the
relation numerically (parent (5,0,0) ∘ local (1,0,0) = world (6,0,0)).

### D. Transform multiplicity

- Definition vertices stay definition-local (never baked). `tests/assembly.rs`
  `definition_geometry_stays_local_and_transform_applies_once` asserts the
  definition soup sits at the origin triangle while its occurrence world
  transform equals the source placement — applied exactly once by the renderer.
- Instance transform = `normalization(up_axis) * Path::matrix()`; the GLB path
  uses the same parent·local order.

---

## Single-part and colour regression

- `bracket.step`: 1 node / 1 geometry / 1 instance, 1814 triangles, bounds
  unchanged. `styled_bracket.step`: identical scene, colours through the
  existing appearance path. A file with no occurrences takes the flat path
  (`parse_step_scene` falls back; `tests/assembly.rs` asserts this).
- All tessellation machinery is shared, not duplicated: `parse_step_table` and
  `parse_step_table_assembly` both call the same `mesh_shell` and
  `report_step_losses` helpers. DIAG-002 conversion-loss records, G8 refusal
  reasons, the face tally, the torus observer census, and the styled-item
  diagnostics are emitted identically on both paths (verified on core_xy:
  "47 of 5670 faces", 13 conversion losses, 34 unsurfaced, torus 40 attempted).

---

## Build / test results

Look (final clean-pin build, override disabled, `CARGO_BUILD_JOBS=2` to fit the
page file):

```text
cargo fmt --all -- --check           ok
cargo check --locked --all-targets  ok
cargo test --locked --lib            187 passed
cargo test --locked --test assembly  5 passed
cargo test --locked --test step      8 passed
cargo test --locked --test spline_carrier  4 passed
cargo test --locked --test torus_deck      25 passed
cargo test --release --test gpu_smoke -- --ignored --nocapture --test-threads=1  2 passed
git diff --check                    ok
```

Truck (`truck-fork` @ 7a017443):

```text
cargo fmt -p truck-stepio -- --check  ok (truck-geometry has pre-existing drift
                                       in committed files, untouched)
cargo test -p truck-stepio --lib      49 passed, 1 failed (geom_impls::builder,
                                       pre-existing at baseline 52dbc2e7)
cargo test -p truck-stepio --test input  39 passed, 6 failed (golden/resource
                                       tests, identical at baseline 52dbc2e7)
cargo test -p truck-stepio --test input assy   occt_assy ok
cargo test -p truck-assembly          39 passed
git diff --check                     ok
```

All pre-existing failures were verified identical on the untouched baseline via
a temporary worktree; none involve the assembly change.

---

## Source constructs still unsupported

- The `ball` definition (a sphere) tessellates to nothing on both the old and
  new paths; not introduced here.
- A definition whose linked geometry cannot be converted at all (e.g. an
  unsupported surface family inside the source BREP) is reported as a lost face
  via the existing DIAG-002/G8 streams; the definition either renders partially
  or is skipped with a warning, exactly as the flat path would.
- The assembly path requires Truck to resolve the graph; a file that declares
  occurrences but whose graph fails to build degrades to the flat single-part
  path (today's behavior) rather than failing the render.
