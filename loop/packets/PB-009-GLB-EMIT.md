# WORK PACKET PB-009-GLB-EMIT — GLB emission with named occurrences and per-solid color

You are adding GLB (glTF 2.0 binary) emission to the bridge so text-to-cad
models can reach a renderer with their assembly names and per-solid colors
intact. Everything you need is in this document,
`docs/TRUCK123D_PY_BRIDGE_SPEC.md` (§ packet table), and the anchors below.
Do not read other spec files. If something you need is genuinely missing,
that is a SPEC_GAP (see "Stop conditions"): you stop and report, you do not
research it.

```yaml
id:          PB-009-GLB-EMIT
contract:    [PB-009-GLB-EMIT]
class:       mechanical
crates:      [truck123d]
depends_on:  [PB-006-ASSEMBLY, PB-008-TTC-HARNESS]
write_allow:
  - truck123d/src/glb_emit.rs
  - truck123d/src/lib.rs
  - truck123d/src/assembly_emit.rs
  - truck123d/tests/pb_glb_emit.rs
  - docs/PY_BRIDGE_COMPAT_SURFACE.md
read_allow:
  - truck123d/src/python.rs
  - truck123d/src/tables.rs
  - truck123d/src/marshal.rs
  - truck123d/src/assembly_emit.rs
  - truck123d/compat/
  - examples/generate_ball_bearing.rs
  - corpus/ttc/trees/f1/STEP/f1.step.js
  - corpus/ttc/PROVENANCE.md
  - docs/PY_BRIDGE_COMPAT_SURFACE.md
  - docs/TRUCK123D_PY_BRIDGE_SPEC.md
tests_required:
  - glb_round_trips_names_hierarchy_colors
  - glb_binary_structure_is_valid
  - f1_occurrence_naming_parity
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'serde_json::to_vec' examples/generate_ball_bearing.rs"}
  - {id: A2, expect: 29, cmd: "grep -cF '#o1.' corpus/ttc/trees/f1/STEP/f1.step.js"}
  - {id: A3, expect: 0, cmd: "ls truck123d/src/glb_emit.rs 2>/dev/null | wc -l"}
  - {id: A4, expect: 2, cmd: "grep -c 'pub fn' truck123d/src/assembly_emit.rs"}
  - {id: A5, expect: 0, cmd: "grep -c 'S8' docs/PY_BRIDGE_COMPAT_SURFACE.md"}
budget:      {turns: 60, ctx_tokens: 140000}
```

New files (glb_emit.rs, tests/pb_glb_emit.rs): H-1 applies — no
unwrap_used without a justified same-line opt-out.

## Problem

The text-to-cad corpus (F1 tree) colors every solid via shape.color and
addresses assembly children by frozen occurrence labels (#o1.N). OCCT's
export_step carries both into the file, which is what the corpus's render
stack consumes. The bridge cannot emit STEP for these models (TR-NRB-001:
swept/lofted carriers export mesh only), so the corpus's colored assemblies
are unreachable from the bridge. GLB carries the same payload natively —
node names, hierarchy transforms, per-node base colors — and the
corpus-side renderer reads GLB. This packet adds the emission.

## Scope decisions — pre-made, do not relitigate

1. GLB, not STEP. TR-NRB-001 makes STEP a non-starter for the F1
   carriers. Do not open the STEP-writer question.
2. Hand-rolled writer over serde_json — GLB is a 12-byte header, one
   JSON chunk, one BIN chunk, 4-byte alignment. The in-repo precedent is
   examples/generate_ball_bearing.rs (serde_json document → bytes).
   **serde_json is ALREADY a dependency of truck123d (verified in
   Cargo.toml 2026-09-06)** — do NOT touch Cargo.toml or Cargo.lock; they
   are outside your write set. Do NOT add a glTF-writer crate or any new
   dependency.
3. Naming convention is corpus-fixed from the vendored tree, not
   researched from PyPI. The vendored f1.step.js pins the occurrence
   table (#o1.N, insertion order under the root). Your emission must
   reproduce that convention from assembly insertion order. The open
   integration question — whether the corpus's animate mode binds its JS
   sidecar against GLB node names — is OUT of scope; the contract here is
   the vendored table.
4. Color payload. The bridge's client-layer color record (corpus
   shape.color, an S2-class data row) rides as
   pbrMetallicRoughness.baseColorFactor. Linear vs sRGB: convert the
   corpus sRGB color to linear on emission (glTF base colors are linear).
   Metallic/roughness: fixed defaults (1.0 / 0.5), no stage vocabulary —
   that is the renderer's config, not file content.
5. Geometry input is mesh data, not solids. Emission takes per-node
   mesh payloads (positions f32, indices u32, one color, one name, one
   parent index + transform) exactly as the harness door's STL path
   marshals them. No new kernel surface; zero new Refusal arms.
6. Additive only. assembly_emit.rs gets at most one new pub fn that
   walks the landed assembly graph into the node payload list (V5 guard:
   its landed tests stay byte-identical). Everything else is new files.
7. Compat surface row. Add row S8 to docs/PY_BRIDGE_COMPAT_SURFACE.md —
   surface id S8, Color/Shape.color + occurrence naming, status
   `recorded-client-layer` (a valid STATUS_TOKEN — verified against
   compat/surface.rs STATUS_TOKENS), answered-by the GLB emission. One
   row, same table format; do not touch rows S1–S7. The machine-check
   test (`compat_surface_table_is_complete`) iterates the 7 landed
   SURFACE_ROWS one-directionally — it does NOT reject extra doc rows —
   so ttc_harness.rs and compat/surface.rs stay untouched, and they are
   outside your write set.
8. V5, absolute: landed bridge tests are byte-identical constraints.

## Anchors — measured 2026-09-06 at arming; re-check on your branch

| id | file | pattern | expect |
|----|------|---------|--------|
| A1 | examples/generate_ball_bearing.rs | serde_json::to_vec | ≥1 (measured 1) |
| A2 | corpus/ttc/trees/f1/STEP/f1.step.js | `#o1.` | ≥28 (measured 29) |
| A3 | truck123d/src/glb_emit.rs | anything | 0 before (absent), yours after |
| A4 | truck123d/src/assembly_emit.rs | pub fn | ≥1 (measured 2) |
| A5 | docs/PY_BRIDGE_COMPAT_SURFACE.md | S8 | 0 before, ≥1 after |

## House rules

- H-1 no unwrap/expect/panic reachable from geometry; H-3 same-line
  `// H-3`; H-6 never record Float as Exact.
- Determinism: node ordering is insertion order; bufferViews/accessors
  are emitted in a fixed derivation order; no hash ordering in output.
  Same input table → byte-identical GLB.
- All cargo through the queue shim (the `cargo` on PATH IS the queue
  shim). Do not invoke cargo by absolute path; do not unset the shim.
  Scoped commands only.

## Tests required

1. `glb_binary_structure_is_valid` — magic glTF, version 2, total length
   == file length, both chunk lengths 4-byte aligned, JSON chunk format
   0x4E4F534A, BIN chunk format 0x004E4942.
2. `glb_round_trips_names_hierarchy_colors` — a 3-node fixture (parent +
   two children, distinct transforms and colors): parse back the JSON
   chunk with serde_json::Value (no new dev-deps) and assert node names,
   parent/child indices, TRS/matrices, and baseColorFactor values —
   including the sRGB→linear conversion on one input color.
3. `f1_occurrence_naming_parity` — a 28-node insertion-order fixture
   asserts the naming function reproduces #o1.1 … #o1.28 exactly as the
   vendored occurrence table (f1.step.js) pins them.

## Done when — run these, all must pass

```
cargo fmt --check -p truck123d
cargo clippy -p truck123d --all-targets -- -D warnings
cargo test -p truck123d
```

Send cargo output to a file and read the tail.

## Forbidden

Anything outside write_allow — especially vendor/truck/** (read-only),
PB-004's landed core files, corpus/ttc/** (read-only fixtures), any landed
test file, truck123d/Cargo.toml + Cargo.lock (serde_json is already a
dependency). Rows S1–S7 of the compat table. ttc_harness.rs and
compat/surface.rs. Any stage/lighting/animation vocabulary in the GLB
(that is renderer config). Adding #[ignore]. Unjustified #[allow].
Committing to main.

## Stop conditions

- any anchor count differs → ANCHOR_MISMATCH
- the landed assembly graph cannot yield node names + parent indices +
  transforms without widening python.rs → SPEC_GAP, naming the missing
  record
- three consecutive failed cargo test runs on the same error → BLOCKED

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

```json
{"id":"PB-009-GLB-EMIT","status":"DONE","contracts":["PB-009-GLB-EMIT"],
 "tests_added":3,"anchors_verified":{"A1":1,"A2":28,"A3":1,"A4":1,"A5":1},
 "notes":"the node payload record shape; the sRGB conversion call site; any API deviation"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-DONE status also write `QUESTION.md` beside it.

Commit subject: `feat(bridge): glb emission with named occurrences and per-solid color (PB-009-GLB-EMIT)`.
