# WORK PACKET PB-000-CONTRACT — the Python bridge shim: API table, exception mapping, schema v1, determinism contract

You are implementing the contract packet of the Truck123d Python Bridge (PB)
program. Everything you need is in this document and the build spec
`docs/TRUCK123D_PY_BRIDGE_SPEC.md`. Do not read other spec files. If
something you need is genuinely missing, that is a SPEC_GAP (see "Stop
conditions"): you stop and report, you do not research it.

```yaml
id:          PB-000-CONTRACT
contract:    [PB-000-CONTRACT]
class:       design
crates:      [showcases]
depends_on:  []
write_allow:
  - docs/PY_BRIDGE_CONTRACT.md
  - docs/TRUCK123D_PY_BRIDGE_SPEC.md
  - showcases/src/cc_ports.rs
  - showcases/tests/pb_contract.rs
read_allow:
  - vendor/truck/truck-shapeops/src/facade.rs
  - vendor/truck/truck-topology/src/entity_id.rs
  - vendor/truck/truck-base/src/evidence.rs
  - showcases/tables/waterslide.json
  - showcases/tables/teapot.json
  - showcases/tables/amphora.json
  - showcases/src/harness.rs
  - docs/TRUCK123D_PY_BRIDGE_SPEC.md
tests_required:
  - pb_table_schema_v1_parses_all_three_tables
  - pb_api_table_covers_landed_facade
  - pb_report_determinism_same_table_same_rev
  - pb_refusal_mapping_covers_landed_refusal_variants
budget:      {turns: 40, ctx_tokens: 100000}
```

This is the program's **shim packet**: frozen CONTRACT DOCUMENTS + a
contract-pinning test file. NO pyo3, NO Python, NO new Rust modules. Later
packets (PB-001..007) type against what you freeze here.

## Problem

The PB program (spec §1) is a naming + semantics table with zero geometric
content over the landed kernel. Before any code lands, four contracts must
be frozen as documents a reviewer can diff against the Python surface:

1. **The API mapping table** — every build123d-facing name → the landed Rust
   entry it dispatches to (and NOTHING else; a name with no landed entry is
   a row that says "refuses typed" with the refusal case named).
2. **The refusal→exception mapping** — the landed `Refusal` taxonomy
   (`truck-base/src/evidence.rs`) maps onto a TWO-CLASS Python exception
   hierarchy: `Refused` (kernel typed refusal, carrying the
   `EnvelopeCase`/witness payload) and `Unresolved` (the certified
   three-valued verdict, carrying κ/cell/slope). Nothing degrades to a bare
   `Exception`.
3. **The table schema v1** — the `showcases/tables/*.json` format, written
   as a normative schema (required keys, value domains, version field).
4. **The byte-determinism contract** — same table + same kernel rev →
   byte-identical report JSON, whether the builder ran from Rust or Python.

## What lands, concretely

**`docs/PY_BRIDGE_CONTRACT.md`** (NEW): the four contract sections above.
The API table covers the 16 landed facade entries verbatim
(`extrude`, `extrude_vector`, `revolve`, `fillet`, `chamfer`, `mirror`,
`mirror_about_plane`, `rotate`, `scale`, `translate`, `section`, `split`,
`bounding_box`, `boolean_op`, `make_face`, `make_hull`) plus the
sweep/loft-shaped entries the showcases consume through
`showcases/src/cc_ports.rs` (read it; name the trait methods it forwards —
do not invent new geometry names). Each row: build123d name, Rust entry
path, argument table-shape, refusal cases, and the stable-regime notes that
apply (NUM-INTERPOLE-OVERSHOOT-001's n ≲ 48 bound is a documented kernel
refusal, not papered over — spec §5).

**`showcases/src/cc_ports.rs`**: DOC-COMMENT freeze only — add a
module-level doc paragraph stating this module is the PB program's
anti-corruption layer and its method set is frozen as of this packet (no
signature changes; the file is otherwise untouched).

**`showcases/tests/pb_contract.rs`** (NEW): the contract-pinning tests below.

## Anchors — measured 2026-09-05 at `90672a7`, counts are exact

Locate by pattern, never by line number. If a count differs, STOP and report
`ANCHOR_MISMATCH` with what you saw.

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `vendor/truck/truck-shapeops/src/facade.rs` | `^pub fn ` | 16 |
| A2 | `vendor/truck/truck-topology/src/entity_id.rs` | `pub enum Selector` | 1 |
| A3 | `vendor/truck/truck-topology/src/entity_id.rs` | `pub fn sel\(` | 1 |
| A4 | `Cargo.toml` | `showcases` | 1 |
| A5 | `showcases/tables` | `\.json` files present | 3 |

A1 is the whole landed facade surface the API table must cover. PB-001
consumes A2/A3 (the `Selector` vocabulary) — this packet only records it.
The workspace has **zero pyo3 dependencies** (manifest-level; the two
prose mentions are doc comments) — PB-004 introduces the only one, not you.

## House rules

- **Zero geometric content**: you write no geometry code and no new Rust
  modules. If a contract row cannot be stated without computing something,
  that is a SPEC_GAP.
- **H-1** No `unwrap`, `expect`, `panic!` in the test file without a
  justified same-line opt-out (`// H-1` style is not registered for
  showcases; keep the test file panic-free by using `Result` returns and
  `assert!`).
- **H-3** No absolute constants in predicates; test tolerance literals
  carry `// H-3` on the SAME line.
- **Determinism** (spec §5): the determinism test builds the same table
  twice IN-PROCESS and compares report bytes; it does not shell out.
- **All cargo invocations go through the queue (the `cargo` on PATH IS the
  queue shim). Do not invoke cargo by absolute path; do not unset the
  shim.**
- Never run a bare `cargo test` — use the scoped commands below.

## Tests required

Named `#[test]` fns in `showcases/tests/pb_contract.rs` — the verifier
checks the names appear in your diff.

1. `pb_table_schema_v1_parses_all_three_tables` — each of the three
   `showcases/tables/*.json` parses against the schema v1 you documented
   (required keys present, value domains respected, a schema-version field
   exists or the schema documents its absence as v1-by-omission — pre-decide
   in the doc, then pin it here).
2. `pb_api_table_covers_landed_facade` — the test embeds the API table's
   Rust-entry column as a const array and references the corresponding
   `truck_shapeops::facade` items (compile-enforced existence + a
   name-by-name assert against the doc's row count, 16 + the cc_ports
   forwards).
3. `pb_report_determinism_same_table_same_rev` — build the waterslide table
   through the landed showcase harness twice; the two report JSONs are
   byte-equal. (If the harness is already deterministic this is a pin, not
   a fix; if it is NOT, STOP — `SPEC_GAP` naming the nondeterminism, and
   write no fix: that is a different packet.)
4. `pb_refusal_mapping_covers_landed_refusal_variants` — the mapping
   section's variant list, embedded as a const array, is asserted to cover
   every variant of the landed `Refusal` enum (iterate the enum via a
   match-exhaustive helper in the test; adding a variant later breaks this
   test on purpose).

No existing test may be deleted, `#[ignore]`d, or weakened.
`battery_construction.rs` / `battery_waterslide.rs` are byte-identical
constraints.

## Done when — run these, all must pass

```
cargo fmt --check -p showcases
cargo clippy -p showcases --all-targets -- -D warnings
cargo test -p showcases --tests
```

Send cargo output to a file and read the tail.

## Forbidden

Editing anything outside `write_allow` — especially `facade.rs`,
`entity_id.rs`, `evidence.rs`, anything under `truck-geometry/` or
`truck-modeling/`, `Cargo.toml`/`Cargo.lock` (PB-004 adds the pyo3
dependency, not you), any landed test file, `scripts/kernel-gates.sh`.
Writing pyo3 or Python code. Adding `#[ignore]`. Adding `#[allow]` without
a justification comment on the same line. Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- a contract row cannot be stated from landed surface alone → `SPEC_GAP`,
  naming the row and what is missing
- the determinism test observes nondeterminism → `SPEC_GAP` naming the
  report field (do not fix it here)
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

**COMMIT BEFORE writing `RESULT.json`.** Then write `RESULT.json` at the
root of your worktree (not `loop/results/` — the orchestrator files it
there).

```json
{"id":"PB-000-CONTRACT","status":"DONE","contracts":["PB-000-CONTRACT"],
 "tests_added":4,"anchors_verified":{"A1":16,"A2":1,"A3":1,"A4":1,"A5":3},
 "notes":"the cc_ports forwards you named in the API table, and any schema-v1 decision the tables forced"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`docs(showcases): PB contract shim — API table, exception mapping, schema v1, determinism contract (PB-000-CONTRACT)`.
