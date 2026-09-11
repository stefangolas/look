# ASSEMBLY_PLACEMENT_AUDIT.md

Bounded semantic audit of the STEP assembly-placement path in `look` against the
declarative placement contract, witnessed by `C:\Users\stefa\core_xy.step`
("core_xy"). Audit only; no code changed.

---

## 1. Exact provenance

| Repository | Commit | Working tree |
|---|---|---|
| `C:\Users\stefa\look` | `ea21c9c4c06e72791b1fd2e5848306ac993c004f` | `M src/cli.rs`, `M src/ui.rs`, untracked `benchmarks/occt_corpus_inner.py`, `core_xy_viewer.html`, `docs/TEXT_TO_CAD_INTEGRATION.md`, `examples/probe_step.rs`, `mobius.step` |
| `C:\Users\stefa\truck-fork` | `10d87c771bd9b42dd1529aea9fe5f85a08cdfc2b` | `M truck-meshalgo/src/tessellation/diagnosis.rs`, `.../triangulation.rs`, untracked `NIST_RECOVERY_HANDOFF*.md` |

Truck resolved by `look`: all truck crates pinned by git rev **`10d87c77`**
(`Cargo.toml` lines 30–46, and `[patch.crates-io]` lines 36–46). That rev equals
`truck-fork` HEAD `10d87c77…`, so a clean clone builds exactly the audited fork.
`.cargo/config.toml`'s `[patch]` path override is commented out — the build is
not path-redirected. `ruststep` pinned at `67f1f7c`.

Witness: `C:\Users\stefa\core_xy.step` (9,153,139 bytes), 176,482 entity
records. Content census: 175 `NEXT_ASSEMBLY_USAGE_OCCURRENCE`, 175
`ITEM_DEFINED_TRANSFORMATION`, 175 `CONTEXT_DEPENDENT_SHAPE_REPRESENTATION`, 270
`PRODUCT_DEFINITION_SHAPE`, 555 `PRODUCT_DEFINITION`, 95
`SHAPE_DEFINITION_REPRESENTATION`, 95 `SHAPE_REPRESENTATION` (76 of them
`ADVANCED_BREP_SHAPE_REPRESENTATION`), 76 `MANIFOLD_SOLID_BREP`, 9,086
`AXIS2_PLACEMENT_3D`. Length unit: `SI_UNIT($,.METRE.)` — the file is in
**metres**.

---

## 2. The complete implemented placement code path

`look` has **no occurrence/placement machinery**. The entire STEP path is:

```
scene.rs:284   extension() dispatch        "step" | "stp" => compile_step(...)
scene.rs:356   compile_step()              read + hash, then parse_step(...)
scene.rs:368   → crate::step::parse_step() → flat StepTriangleSoup
step.rs:81     parse_step()
step.rs:102    Table::from_owned_data_section(section)     (truck-stepio)
step.rs:142    table.shell.par_iter()      ← ALL shells, flat, no graph
step.rs:151    table.to_compressed_shell(id, shell)        (per shell, definition-local)
step.rs:232    wrap_shell_with_closure(...).robust_triangulation_with_torus_outcome(...)
step.rs:352    flatten every shell's faces into one positions/indices/colors soup
scene.rs:376    soup_normals(...)
scene.rs:413    compile_triangle_mesh()
scene.rs:448    transform = normalization_transform(up_axis)   // Y default ⇒ identity
scene.rs:454    ONE Instance { geometry: 0, material: 0, transform: normalization }
```

Data flow actually executed:

```
STEP file
  → read_exchange (part21 fast path or ruststep)
  → Table::from_owned_data_section         (truck-stepio `in/mod.rs`)
  → table.shell                             (flat HashMap<u64, ShellHolder>)
  → to_compressed_shell per shell           (truck-stepio `in/convert.rs`)
  → tessellation (look policy wrappers)
  → single StepTriangleSoup
  → ONE Geometry + ONE Instance (transform = up-axis normalization only)
  → renderer (ui.rs consumes Instance.transform/normal_transform)
```

**Not executed anywhere in `look`** (verified by repository-wide search — zero
hits for `step_assy`, `ProductEntity`, `AssembleEntity`, `NodeEntity`,
`truck_assembly`):

```
Table::step_assy()                        truck-stepio/src/in/convert.rs:833-861
assy_node_entity (NAUO→CDSR→SRRWT→IDTF)   convert.rs:779-831
ProductShape::try_from_index              convert.rs:706-718
ItemDefinedTransformation → Matrix3/4     truck-stepio/src/in/mod.rs:3870-3886
Path::matrix() (world composition)        truck-assembly/src/assy.rs:68-75
per-occurrence transform application      examples/step-to-mesh.rs:88-111
```

Every assembly-capable building block exists in the pinned truck and is simply
**bypassed** by `look`'s STEP path. `look inspect core_xy.step --json` confirms
the flat result: `instances: 1, nodes: 1, unique_geometries: 1,
mesh_primitives: 1`.

---

## 3. Rules 1–15 compliance table (implemented `look` STEP path)

| # | Rule | Verdict | Evidence / divergence |
|---|---|---|---|
| 1 | Definition identity ≠ occurrence identity | **VIOLATION** | No occurrence concept exists. All 76 solids are merged into one `Geometry`; one `Instance` is emitted (scene.rs:454). Occurrence identity is not merely conflated — it is erased. |
| 2 | Every transform has explicit direction | **UNSUPPORTED** | No transform is modelled at all, so no direction can be stated. The file's semantics require `T_rep1→rep2 = F_rep2 · inverse(F_rep1)` (see §5); nothing in the path computes it. |
| 3 | Child local transform has one canonical meaning | **UNSUPPORTED** | No `T_local` concept exists. `T_local` would be the per-occurrence `ITEM_DEFINED_TRANSFORMATION` result; it is never read. |
| 4 | World = `T_world(P) · T_local(C)` | **UNSUPPORTED** (STEP) | The GLB path does `world = parent_transform * local` correctly (scene.rs:918-919), but the STEP path has no hierarchy to compose. |
| 5 | STEP frames normalized once | **UNSUPPORTED** | `AXIS2_PLACEMENT_3D` is only consumed inside surface placement (tessellation); never normalized into an occurrence frame. |
| 6 | Frame-to-frame transform mathematically derived | **UNSUPPORTED** in `look`; truck has the correct derivation | Truck computes `M = mat2 * mat1.invert()` (mod.rs:3879-3886) — directionally correct (see §5) — but `look` never reaches it. |
| 7 | Units normalized before composition | **UNSUPPORTED** (vacuous) | No translation is composed, so nothing is mis-normalized; the tolerance is relative (step.rs:604). Native units (metres) pass through untouched. A fix must not silently assume millimetres. |
| 8 | Hierarchy only from source assembly relationships | **VIOLATION** | `table.shell.par_iter()` (step.rs:142) tessellates every shell as a peer. The 175 source `NEXT_ASSEMBLY_USAGE_OCCURRENCE` edges are ignored; hierarchy is effectively "every shell is its own root", a family of relationships the source never declared. |
| 9 | Definition geometry stays definition-local | **PASS** | Shells are tessellated in their file-local coordinates (step.rs:151, 232). This is a passive pass — nothing ever transforms them. |
| 10 | Transforms applied exactly once | **PASS** (for what exists) | Occurrence transforms are applied 0 times; up-axis normalization is applied once. No double application anywhere. The rule's real failure is under Rule 11, not multiplicity. |
| 11 | Unresolved placement must not become identity | **VIOLATION** | Every unresolved occurrence effectively renders at identity. In this file 175/175 occurrences carry non-identity world frames (see §4); all render at definition origin. |
| 12 | Rigid transforms satisfy claimed class | **PASS** | No occurrence transform is claimed; the up-axis normalization is rigid. Nothing to violate. |
| 13 | Nested subassemblies obey same composition | **UNSUPPORTED** | File is 3 levels deep (§8); no nesting is processed. |
| 14 | Representation mapping and occurrence placement distinct | **UNSUPPORTED** | No representation-map transform and no occurrence placement are handled; nothing can be double-counted. |
| 15 | Renderer consumes resolved occurrences | **PASS** | The renderer consumes `Instance { transform }` as-is (ui.rs:56-58); it never reinterprets STEP semantics. The failure is entirely upstream, in the resolver that never resolves. |

**First-order summary:** rules 9, 10, 12, 15 pass (some vacuously); 1, 8, 11
are active violations; the rest are unsupported because the occurrence layer is
absent.

---

## 4. Real occurrence traces from the scattered witness

### 4.1 `pi_4_enclosure_top` — a visibly misplaced part

Source graph (entity IDs as written in the file):

```
NAUO #666   NEXT_ASSEMBLY_USAGE_OCCURRENCE('NAUO1','pi_4_enclosure_top',…,#175825,#175826,'')
             parent PD #175825 (assembly root), child PD #175826
PDS #175557 PRODUCT_DEFINITION_SHAPE(' ','NAUO PRDDFN',#666)     (occurrence shape)
SDR #98353  SHAPE_DEFINITION_REPRESENTATION(#175556,#98448)
CDSR #491   CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(#841,#175557)
SRRWT #841  (REPRESENTATION_RELATIONSHIP(' ',' ',#98448,#98447)
             REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#1016)
             SHAPE_REPRESENTATION_RELATIONSHIP())
IDTF #1016  ITEM_DEFINED_TRANSFORMATION(' ',' ',#98542,#98737)
op  #98542  AXIS2_PLACEMENT_3D('',#134678,#107628,#107629)
            loc (0,0,0), axis (0,0,1), ref (1,0,0)   ⇒ identity frame
tf  #98737  AXIS2_PLACEMENT_3D('',#135307,#108243,#108244)
            loc (−0.0262604611974987, 0.24232, 0.316053894888206)
            axis (1.11e-16, 1, −2.78e-16), ref (−1, 1.11e-16, −1.11e-16)
rep_1 #98448 SHAPE_REPRESENTATION('pi_4_enclosure_top',(#98542),#175363)
rep_2 #98447 SHAPE_REPRESENTATION('main',(…82 placement frames…),#175362)
geometry: MSB #98276 → CLOSED_SHELL #97490 (in ADVANCED_BREP_SHAPE_REPRESENTATION #1268)
```

Derived local transform (truck semantics, `F_tf · inverse(F_op)`):
`T_local = M(#98737) · inverse(M(#98542)) = M(#98737)`, basis columns
X=(−1,0,0), Y=(0,0,1), Z=(0,1,0), origin=(−0.0263, 0.2423, 0.3161).

- Definition-local bbox (walked from shell #97490): X[−0.047, 0.047]
  Y[−0.0325, 0.04] Z[0.00238, 0.023] m.
- Correct world placement (`p_world = M(#98737)·p_local`): world X = −0.026−px,
  world Y = 0.242+pz, world Z = 0.316+py ⇒ **predicted world bbox
  X[−0.073, 0.021] Y[0.244, 0.265] Z[0.284, 0.356].**
- **Transform handed to the renderer by `look`: identity** (normalization for
  up-axis Y is `Mat4::IDENTITY`, scene.rs:448). The part renders at its local
  bbox near the origin.
- Actual rendered realization (inspect global bounds): the part contributes to
  look's scene bounds X[−0.25, 0.2555] Y[−0.22, 0.22] Z[−0.0606, 0.4], i.e. it
  sits at the origin, not at (−0.026, 0.242, 0.316) with the 90° twist.

### 4.2 `motor` — repeated instance of a shared definition, both occurrences collapsed

```
NAUO #697 NEXT_ASSEMBLY_USAGE_OCCURRENCE('NAUO32','motor','motor',#175825,#175851,'')
NAUO #725 NEXT_ASSEMBLY_USAGE_OCCURRENCE('NAUO60','motor','motor',#175825,#175851,'')
          same child PD #175851, same rep_1 #98473 SHAPE_REPRESENTATION('motor',(#98542),#175388)
IDTF #1047 ITEM_DEFINED_TRANSFORMATION(' ',' ',#98542,#100338)  → frame #100338 loc (−0.20289, 0.181, 0.4468)
IDTF #1075 ITEM_DEFINED_TRANSFORMATION(' ',' ',#98542,#102619)  → frame #102619 loc ( 0.20289, 0.181, 0.4468)
```

Same definition, two occurrence IDs, two distinct mirror world frames. `look`
tessellates the shared definition once and emits one instance at identity —
**both occurrences render on top of each other at the origin**, and the
left/right mirror distinction of the machine is lost.

### 4.3 Nested occurrence (hierarchy depth ≥ 1)

The file has 18 distinct parents and depth up to 3 (§8). A nested occurrence's
world transform would require the recursive `T_world = T_world(parent) ·
T_local(child)` composition; `look` composes nothing, so every level collapses
to definition origin. No nested witness can render correctly.

---

## 5. Transform-direction analysis

For occurrence #666 the two candidate matrices are:

```
T_candidate   = M(#98737) · inverse(M(#98542)) = M(#98737)
inverse(T)    = inverse(M(#98737))
```

Source/target frame semantics: `#98542` is the child frame **inside rep_1**
(the child's own representation, item of `#98448`) and `#98737` is the same
child frame **inside rep_2** (the assembly "main" representation, item of
`#98447`). Both frames are expressed in their own context's coordinates. The
mapping a renderer needs is *child-local geometry → assembly coordinates*:

```
p_assembly = F_rep2 · inverse(F_rep1) · p_child_local
           = M(#98737) · inverse(M(#98542)) · p_local
           = M(#98737) · p_local
```

Equation proving direction: the identity operator `#98542` is the child frame in
its own space; the transformed item `#98737` is that frame in the assembly
space. `F_rep2 · inverse(F_rep1)` is the only direction for which
`F_rep1 ↦ F_rep2` — placing the child's origin exactly at (−0.026, 0.242,
0.316). Using `inverse(T)` instead would place the part at
(−0.026, 0.242, 0.316) *measured from itself*, i.e. the identity would be
dropped and the frame reflected — geometrically false for this file. This is
exactly the direction truck implements (`mat2 * mat1.invert()`,
mod.rs:3879-3886), so the correct derivation is **already available**; `look`
simply never calls it.

## 6. Composition-order analysis

Truck's assembly path composes `path.matrix()` as a root→leaf fold
`matrix * edge.matrix` (truck-assembly/src/assy.rs:68-75). Applied to a point
that is `M1·M2·…·Mn·p` (rightmost applied first), i.e. leaf local → parent → …
→ root: **`T_world(child) = T_world(parent) · T_local(child)`**, contract Rule 4.
The alternative `T_local · T_parent` is never used. The GLB path in `look`
(scene.rs:919 `world = parent_transform * local`) uses the same order.

The audited STEP path composes **nothing**: with one instance and one geometry,
the question of order never arises because there is exactly one (identity)
transform. Composition order is therefore not *wrong* — it is absent.

## 7. Transform-multiplicity analysis

| Semantic transform | Applications in implemented path |
|---|---|
| Assembly occurrence transform (`ITEM_DEFINED_TRANSFORMATION`, 175 in file) | **0 times** — never read |
| Representation-map transform (`REPRESENTATION_RELATIONSHIP` / CDSR) | **0 times** |
| Root transform | identity (up-axis Y default), applied once |
| Renderer/model transform (instance transform) | once — but it holds only the up-axis normalization |
| Transform baked into vertices | none |

No double application anywhere (Rule 10/I8-I10 pass), but the occurrence
transform's multiplicity is **zero**, which is the contract failure. There is no
path where the same matrix is baked into vertices and also supplied to the
renderer.

## 8. Occurrence/definition reuse

Verified on the file: 175 occurrences over 76 distinct `MANIFOLD_SOLID_BREP`
definitions; 28 child definitions instantiated by >1 occurrence (max 16 uses);
e.g. `motor` (#175851) at ±0.20289 m (trace §4.2). `look` produces one geometry
and one instance, so **any map keyed by definition identity collapses all
occurrences onto one placement**. The identity↔occurrence separation required by
Rule 1 is structurally absent.

## 9. Hierarchy-integrity summary (file side)

| Metric | Value |
|---|---|
| Occurrences (`NEXT_ASSEMBLY_USAGE_OCCURRENCE`) | 175 |
| Distinct parents | 18 |
| Distinct children | 94 |
| Roots | 1 |
| Parent–child relations | 175 |
| Parents with >1 child | 18 |
| Shared child definitions (used by >1 occurrence) | 28 (max 16 uses) |
| Max hierarchy depth (root = 0) | 3 |
| Cycles | none detected |
| Occurrences with multiple incompatible parents | none detected |

Implemented-side: `look` reports 1 instance, 1 node, 1 geometry. The hierarchy
is not *mis-built*; it is entirely absent, which is why the audit classifies the
sole rendered entity as an occurrence collapsing every source occurrence, not as
a legitimate root.

## 10. Unit handling

All coordinates in the file are metres (`SI_UNIT($,.METRE.)`; placement frames
and uncertainties such as `LENGTH_MEASURE(2.E-5)` = 20 µm are metre-scale).
`look` tessellates in native units with a relative tolerance (step.rs:604-614),
so no scale error is introduced **today** — but that is only because no
translation is ever composed. Raw placement translation for #666:
(−0.0263, 0.2423, 0.3161) m; normalized translation (would-be world): identical
in metres; world translation actually applied: (0,0,0). The mm↔m mismatch class
is not the demonstrated bug here; the fix must still not silently convert to
millimetres.

---

## 11. First demonstrated semantic divergence

> The first demonstrated semantic divergence occurs at **`src/step.rs:142-155`
> (`parse_step` iterating `table.shell.par_iter()`)** — actually earlier at the
> decision in `src/scene.rs:368` to route STEP into a flat-soup parser — where
> the implementation tessellates every `table.shell` entry in definition-local
> coordinates and merges them into one triangle soup and one instance, but the
> declarative contract requires resolving the source assembly graph (175
> `NEXT_ASSEMBLY_USAGE_OCCURRENCE` occurrences, 175
> `ITEM_DEFINED_TRANSFORMATION` placements) into per-occurrence world transforms
> and rendering each definition at its own `T_world`.

Demonstrated with the witness, occurrence #666 (`pi_4_enclosure_top`):
- File asserts world frame (−0.0263, 0.2423, 0.3161) with a 90° rotation.
- Contract world transform `M(#98737) · inverse(M(#98542))` places the local
  bbox X[−0.047,0.047] Y[−0.033,0.04] Z[0.0024,0.023] at predicted
  X[−0.073,0.021] Y[0.244,0.265] Z[0.284,0.356].
- `look` hands the renderer identity and renders it at the origin. The
  occurrence transform is applied 0 times.

Because all 175 occurrences are dropped the same way, the machine's parts pile
at their definition origins — the "not deformed, but scattered / unrealistically
placed" symptom. Local geometry is correct (Rule 9 passes); the failure is
entirely in occurrence/transform semantics (Rules 1, 2, 8, 11).

Ranked independent defects:
1. **Occurrence graph never traversed** (the demonstrated divergence) — impact:
   every assembly renders flat at definition origins.
2. **Shared-definition reuse collapses** — impact: repeated parts overlap and
   lose their distinct world transforms (motor trace).
3. (Secondary, unproven here) Unit/context mismatches — would only matter after
   a resolver exists; no evidence in this file.

---

## 12. Smallest justified fix plan

The correct derivation and composition already exist in the pinned truck and in
`look`'s GLB path; the fix is to **consume them** for STEP.

- **Files/symbols:**
  - `src/step.rs` — add an assembly-aware entry (e.g. `parse_step_assembly`)
    that builds `table.step_assy()`, walks `top_nodes()` + `paths_iter`, and
    returns per-occurrence `(definition key, world Mat4)`.
  - `src/scene.rs` — extend the STEP compile tail to emit one `Geometry` per
    unique definition and one `Instance { transform: world }` per occurrence
    (mirroring the GLB `visit_node` pattern at scene.rs:918-936); keep the
    existing per-shell tessellation quality path.
  - No changes required to `truck-stepio`/`truck-assembly` — `step_assy`,
    `ItemDefinedTransformation→Matrix4` (`mat2·mat1⁻¹`), and `Path::matrix()`
    already satisfy Rules 4, 6, 13.
- **Exact semantic change:** occurrence world transform = `path.matrix()` from
  `step_assy` (root→leaf fold), applied once via the instance transform;
  vertices stay definition-local; renderer receives `T_world` exactly once.
- **Expected LOC:** ~250–350 (step.rs new entry + scene.rs multi-instance
  compile; single small struct for `(definition_id, world)`).
- **Tests required:**
  1. `core_xy.step` (or a reduced fixture slice of it): instance count == 175;
     each instance transform equals the derived `F_tf · inverse(F_op)`;
     scene bounds ≈ occurrence-frame bbox 0.66 × 0.67 × 0.54 m.
  2. Shared-definition fixture (e.g. `motor` at ±0.203 m): two instances, same
     geometry index, distinct transforms.
  3. Nested fixture (depth ≥ 2): `T_world(child) == T_world(parent) ·
     T_local(child)`.
  4. Regression: existing single-part STEP fixtures (bracket, styled_bracket,
     washer) must keep one instance and unchanged geometry — these files contain
     no `NEXT_ASSEMBLY_USAGE_OCCURRENCE`, so they must remain flat (a root-only
     assembly degrades to today's behavior).

---

## 13. Structural invariants I1–I12

| Invariant | Status |
|---|---|
| I1 occurrence identity ≠ definition identity | **FAIL** — no occurrences; 175 collapse to 1 |
| I2 every rendered occurrence resolves to one definition | **FAIL** — 1 rendered "occurrence" resolves to 76 merged definitions |
| I3 every non-root occurrence has a source-supported parent | **FAIL** — all shells treated as peers |
| I4 every transform has known source/target direction | **FAIL** — no transforms |
| I5 every local transform finite/structurally valid | **PASS** (vacuously — only identity) |
| I6 translation units normalized before composition | **PASS** (vacuous — nothing composed; native metres) |
| I7 `T_world(child) = T_world(parent) · T_local(child)` | **FAIL** — nothing composed (GLB path is correct) |
| I8 every semantic transform composed exactly once | **FAIL** — occurrence transform composed 0 times |
| I9 shared definition geometry not mutated by occurrence transforms | **PASS** — no mutation, because no transforms |
| I10 renderer applies final world transform exactly once | **FAIL** — applies only up-axis identity, not `T_world` |
| I11 unresolved placement never silently becomes identity | **FAIL** — every occurrence silently renders as identity |
| I12 rendered world agrees with `T_world · definition geometry` | **FAIL** — rendered bbox is the origin-piled local union, not the placed machine |

---

## 14. GO / NO-GO for immediate implementation

**NO-GO for immediate implementation in this session.** The divergence is fully
proven and the fix is well-scoped, but it is a structural change to the STEP
compile pipeline (new assembly entry point, multi-geometry/multi-instance
compile tail, ~250–350 LOC), not an "extremely local and unambiguous" patch.
The plan in §12 is implementation-ready; the next session can proceed directly
from it. A quick GO/N-GO re-check gate: if the file degrades gracefully (root
only) and the 4 tests above pass on the reduced fixture, the fix is complete.
