# WORK PACKET PB-006-ASSEMBLY — multi-solid assembly emission

You are adding N-solid assembly emission to the bridge. Everything you need
is in this document and `docs/TRUCK123D_PY_BRIDGE_SPEC.md` (§ packet table
row PB-006). Do not read other spec files. If something you need is
genuinely missing, that is a SPEC_GAP (see "Stop conditions"): you stop and
report, you do not research it.

```yaml
id:          PB-006-ASSEMBLY
contract:    [PB-006-ASSEMBLY]
class:       mechanical
crates:      [truck123d, truck-assembly]
depends_on:  [PB-004-PYO3-CORE]
write_allow:
  - truck123d/src/assembly_emit.rs
  - truck123d/src/lib.rs
  - truck123d/Cargo.toml
  - truck123d/Cargo.lock
  - truck123d/tests/pb_assembly.rs
read_allow:
  - truck123d/src/{python,tables,marshal}.rs
  - vendor/truck/truck-assembly/src/assy.rs
  - vendor/truck/truck-assembly/src/dag.rs
  - docs/TRUCK123D_PY_BRIDGE_SPEC.md
tests_required:
  - assembly_emits_n_solids_step
  - contact_intents_recorded_until_bie
  - teapot_body_spout_handle_assembles
budget:      {turns: 60, ctx_tokens: 140000}
```

**New file** (`assembly_emit.rs`): H-1 applies — no `unwrap_used` without a
justified same-line opt-out.

## Problem

The corpus ships multi-part models (the teapot: body + spout + handle). The
landed `truck-assembly` crate (DAG resolver, `assy.rs`) emits STEP
assemblies; the bridge has no N-solid entry. This packet adds the client-side
emission: N solids + an intended-contact/evidence list → STEP assembly.

## Scope decisions — pre-made, do not relitigate

**R2 AMENDMENT (2026-09-06, adjudicated SPEC_GAP):** the r1 worker proved
the emission cannot compile inside the r1 write set — `truck123d` carries
no `truck-assembly` edge (PB-004's manifest never added one), `PyTruckSolid`
holds no solid representation, and the intent record did not exist
(`CL-005-STOP`-class write-set boundary; probe transcripts in the slot's
QUESTION.md). Adjudicated per the worker's resolution 1:

1. **Write set grows**: `truck123d/Cargo.toml` (+ `Cargo.lock`, scoped to
   the workspace-internal `truck-assembly` path-dep edge — the PB-004
   precedent, which shipped its manifest the same way). Add the edge, then
   the emission compiles.
2. **The INTENT record is defined here** (the missing certificate named by
   the worker): `AssemblyContactIntent { node_a, node_b, kind: Intent,
   note: String }` — recorded as evidence rows on assembly nodes, typed
   INTENT, never certified. This is a client-side record over the landed
   evidence marshaling; zero new kernel `Refusal` arms.
3. **The teapot test fixture is built through the bridge tables** (three
   simple solids — body/spout/handle shapes via PB-004's primitives +
   extrude), NOT by linking `showcases/` (a separate crate, unreachable
   from `truck123d` tests). The test asserts 3 nodes + the recorded
   intents; shape fidelity is not under test here.
4. **Solid representation**: use the bridge's marshaled solid handles —
   do not widen `python.rs` (PB-004's, read-only).
5. **Client-side, additive**: `truck-assembly` is read-only except through
   its public emission API (V5 guard on the landed DAG/assy surface).
6. **V5, absolute**: landed bridge tests are byte-identical constraints;
   this packet is purely additive.

## Anchors — measured 2026-09-05 evening, re-check on your branch

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `truck123d/src/python.rs` | `pub struct PyTruckSolid` | 1 |
| A2 | `vendor/truck/truck-assembly/src/assy.rs` | `pub fn ` | 10 |
| A3 | `showcases/src/teapot.rs` | `fn main` or `pub fn` | ≥1 |
| A4 | `truck123d/src/assembly_emit.rs` | anything | **0 before (absent), yours after** |

## House rules

- **H-1** no unwrap/expect/panic reachable from geometry; **H-3** same-line
  `// H-3`; **H-6** never record `Float` as `Exact`.
- **Determinism**: node ordering fixed (insertion order); no hash ordering
  in output.
- **All cargo through the queue shim.** Scoped commands only.

## Tests required

1. `assembly_emits_n_solids_step` — 3 solids → a STEP assembly with 3
   nodes (round-trip read-back asserts the node count).
2. `contact_intents_recorded_until_bie` — each intended contact appears as
   an evidence row on the node, typed as INTENT (never as a certified
   contact).
3. `teapot_body_spout_handle_assembles` — the teapot table assembles
   end-to-end; the report lists the three parts and their intents.

## Done when — run these, all must pass

```
cargo fmt --check -p truck123d
cargo clippy -p truck123d --all-targets -- -D warnings
cargo test -p truck123d
cargo check -p truck-assembly
```

Send cargo output to a file and read the tail.

## Forbidden

Anything outside `write_allow` — especially `vendor/truck/truck-assembly/**`
(read-only), PB-004's landed core files, any landed test file,
`Cargo.lock`. Resolving contacts in this packet. Adding `#[ignore]`.
Unjustified `#[allow]`. Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- truck-assembly's emission cannot represent an intended-contact row →
  `SPEC_GAP`, naming the missing record
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

```json
{"id":"PB-006-ASSEMBLY","status":"DONE","contracts":["PB-006-ASSEMBLY"],
 "tests_added":3,"anchors_verified":{"A1":1,"A2":10,"A3":1,"A4":1},
 "notes":"the intent-record shape; the teapot node census; any API deviation"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit subject: `feat(bridge): multi-solid assembly emission with recorded contact intents (PB-006-ASSEMBLY)`.
