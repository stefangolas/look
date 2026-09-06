# WORK PACKET PB-005-PYTHON-FACADE — the build123d-shaped Python layer

You are writing the Python-facing facade over the landed pyo3 bridge
(PB-004's `truck123d` crate). Everything you need is in this document,
`docs/TRUCK123D_PY_BRIDGE_SPEC.md` (§ packet table row PB-005, §8 corpus
motive), and `docs/PY_BRIDGE_CONTRACT.md` (PB-000's API table). Do not read
other spec files. If something you need is genuinely missing, that is a
SPEC_GAP (see "Stop conditions"): you stop and report, you do not research it.

```yaml
id:          PB-005-PYTHON-FACADE
contract:    [PB-005-PYTHON-FACADE]
class:       design
crates:      [truck123d]
depends_on:  [PB-001-SELECTORS, PB-004-PYO3-CORE]
write_allow:
  - truck123d/src/facade.rs
  - truck123d/src/lib.rs
  - truck123d/python/truck123d/__init__.py
  - truck123d/python/truck123d/*.py
  - truck123d/tests/pb_facade.rs
read_allow:
  - truck123d/src/{python,tables,marshal,exceptions,gil}.rs
  - docs/PY_BRIDGE_CONTRACT.md
  - docs/TRUCK123D_PY_BRIDGE_SPEC.md
tests_required:
  - facade_round_trips_through_tables
  - builder_mode_is_python_side_only
  - facade_selectors_fluent
  - export_stl_and_step_entries
budget:      {turns: 90, ctx_tokens: 180000}
```

**New files** (`facade.rs`, the Python package): H-1 applies — no
`unwrap_used` without a justified same-line opt-out.

## Problem

The corpus (F1 / Falcon-Heavy, spec §8) exercises the build123d vocabulary:
`BuildPart`/`BuildSketch` context managers, `Mode` algebra, primitives,
`extrude/revolve/sweep/loft/fillet/chamfer`, fluent selectors, and
`export_stl/export_step`. The bridge core (PB-004) marshals data tables and
evidence; this packet is the sugar layer that makes the Python side read
like build123d while NOTHING computes in Python.

## Scope decisions — pre-made, do not relitigate

1. **The Python side EDITS TABLES, then submits** (spec row): builder-mode
   statefulness is Python-side only — a context manager collects operations
   into a table and submits once. No kernel state crosses the boundary.
2. **Every entry point lands in `facade.rs` as one Rust fn** taking the
   submitted table, so the native Rust facade entry (PB-008's third timing
   regime) can call the same fn without Python.
3. **Selectors are fluent but thin**: PB-001's selector machinery, exposed
   as Python methods that produce table rows — the corpus has 4 selector
   uses total; do not build more.
4. **`export_stl` / `export_step`** ride the landed mesh/STEP paths;
   STEP for swept parts stays behind the TR-NRB-001 boundary (STL only).
5. **V5, absolute**: the landed bridge tests (`test_bridge.rs`) are
   byte-identical constraints; the facade is additive.

## Anchors — measured 2026-09-05 evening, re-check on your branch

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `truck123d/src/python.rs` | `pub struct PyTruckSolid` | 1 |
| A2 | `truck123d/src/python.rs` | `pub fn marshal_unresolved` | 1 |
| A3 | `truck123d/src/python.rs` | `pub fn kernel_evidence_compose` | 1 |
| A4 | `truck123d/src/lib.rs` | `pub fn` | 1 |
| A5 | `truck123d/src/facade.rs` | anything | **0 before (absent), yours after** |

## House rules

- **H-1** no unwrap/expect/panic reachable from geometry; **H-3** same-line
  `// H-3`; **H-6** never record `Float` as `Exact`.
- **Determinism**: identical ordered input → identical tables → identical
  reports.
- **All cargo through the queue shim.** Scoped commands only.

## Tests required

1. `facade_round_trips_through_tables` — a BuildPart-shaped Python session
   (primitive + extrude + fillet) produces the same table as the direct
   Rust call (byte-equal JSON).
2. `builder_mode_is_python_side_only` — the context manager submits once;
   an exception mid-build submits nothing (no partial kernel state).
3. `facade_selectors_fluent` — the 4-use selector vocabulary produces the
   documented filtered rows.
4. `export_stl_and_step_entries` — STL exports for the swept fixture;
   STEP export for the prismatic fixture only.

## Done when — run these, all must pass

```
cargo fmt --check -p truck123d
cargo clippy -p truck123d --all-targets -- -D warnings
cargo test -p truck123d
```

Send cargo output to a file and read the tail.

## Forbidden

Anything outside `write_allow` — especially `marshal.rs`, `python.rs`,
`tables.rs` (PB-004's landed core), `vendor/**`, any landed test file,
`Cargo.lock`. Computing geometry in Python. Adding `#[ignore]`. Unjustified
`#[allow]`. Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- the bridge cannot express a corpus surface row (spec §8 table) →
  `SPEC_GAP`, naming the row
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

```json
{"id":"PB-005-PYTHON-FACADE","status":"DONE","contracts":["PB-005-PYTHON-FACADE"],
 "tests_added":4,"anchors_verified":{"A1":1,"A2":1,"A3":1,"A4":1,"A5":1},
 "notes":"the builder-mode design; which corpus surface rows are covered; any API deviation"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit subject: `feat(bridge): build123d-shaped Python facade over the pyo3 bridge (PB-005-PYTHON-FACADE)`.
