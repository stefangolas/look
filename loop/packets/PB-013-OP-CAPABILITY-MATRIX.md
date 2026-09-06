# WORK PACKET PB-013-OP-CAPABILITY-MATRIX — machine-checked per-op capability annotation for the corpus surface

You are turning the PB-010 parity audit's op census into an executable
capability matrix: one annotated cell per (operation x carrier-class) pair,
a test asserting every annotated verdict against LIVE behavior, and nothing
else. Everything you need is in this document,
`loop/results/PB-010-TTC-PARITY-AUDIT.json` (the census — normative),
`docs/PY_BRIDGE_COMPAT_SURFACE.md`, and the anchors. Do not read other spec
files. A genuine gap is a SPEC_GAP: stop and report.

```yaml
id:          PB-013-OP-CAPABILITY-MATRIX
contract:    [PB-013-OP-CAPABILITY-MATRIX]
class:       mechanical
crates:      [truck123d]
depends_on:  [PB-010-TTC-PARITY-AUDIT]
write_allow:
  - truck123d/tests/pb_op_matrix.rs
  - docs/OP_CAPABILITY_MATRIX.md
read_allow:
  - loop/results/PB-010-TTC-PARITY-AUDIT.json
  - truck123d/src/
  - truck123d/compat/
  - docs/PY_BRIDGE_COMPAT_SURFACE.md
  - docs/TRUCK123D_PY_BRIDGE_SPEC.md
  - corpus/ttc/
tests_required:
  - matrix_doc_rows_match_live_semantics
  - swept_carrier_boolean_cells_refuse_typed_today
  - authoring_cells_match_surface_rows
  - partial_arc_revolve_cell_annotated
anchors:
  - {id: A1, expect: 3, cmd: "grep -c NonCanonicalCarrier truck123d/src/facade.rs"}
  - {id: A2, expect: 93, cmd: "grep -c '\"classification\"' loop/results/PB-010-TTC-PARITY-AUDIT.json"}
  - {id: A3, expect: 0, cmd: "ls docs/OP_CAPABILITY_MATRIX.md 2>/dev/null | wc -l"}
budget:      {turns: 60, ctx_tokens: 140000}
```

New files: H-1 applies (no unwrap/expect without a justified same-line
opt-out); H-3 same-line `// H-3`; H-6: never record Float as Exact.

## Problem

PB-011-TTC-PARITY-CHURN will route swept-carrier booleans and lift 22 skip
rows. Its risk is not the routing — it is that the audit's per-op verdicts
have never been EXECUTED: a lift whose underlying op behaves other than the
audit recorded surfaces mid-churn, attributably to nothing. This packet
converts the audit's classification into executable truth BEFORE the churn:
every capability cell gets a test asserting its CURRENT semantics, and a
doc annotating the verdict, the packet that flips it, and the target
verdict.

## Scope decisions — pre-made, do not relitigate

1. **The cell unit is (op x carrier-class), not the audit's 93 site rows.**
   Group the census rows: `fuse(swept,swept)`, `cut(revolved,canonical)`,
   `cut(canonical,canonical)`, `revolve(full,spline-profile)`,
   `revolve(partial-arc,spline-profile)`, `author(circle-profile)`,
   `author(spline-profile)`, `loft(...)`, `sweep(...)`, `color`,
   `label`, `export_stl`, `export_step`, ... — derive the cell set from
   the census's `op` fields; every census row must map to exactly one
   cell (assert that mapping in the test).
2. **Each cell's doc row annotates**: `current` verdict (one of
   `certified` / `refuses(<EnvelopeCase or reason>)` / `client-layer` /
   `unavailable`), `flipped-by` (the packet id whose landing changes it —
   CFP-004, PB-011, ...), `target` verdict, and the census row ids that
   map to it. Verdict vocab is closed; a seeming new arm is a SPEC_GAP
   against `docs/PY_BRIDGE_COMPAT_SURFACE.md`.
3. **The core test drives live behavior and compares it to the doc.** For
   each cell, call the facade/compat entry the way the corpus would (the
   audit's `evidence` names the entry), and assert the annotated current
   verdict: certified cells assert success with the census's recorded
   ground truth; refusal cells assert the typed refusal INCLUDING its
   envelope case; client-layer cells assert the data-row behavior. A
   mismatch fails with the cell id — that is the audit being corrected by
   execution, and it is the desired outcome of THIS packet (record it in
   RESULT; do not silently update the doc to match a surprising verdict).
4. **Refusal cells are asserted, not worked around.** `fuse(swept,swept)`
   refusing `NonCanonicalCarrier` today is the CORRECT current verdict;
   its `flipped-by` is PB-011 (routing) — the test pins the refusal so
   the flip is visible and attributable.
5. **V5 forward-protection**: this packet's tests are landed constraints.
   PB-011 will flip verdicts by editing the doc row + the asserted
   expectation TOGETHER, one commit per flip; until then every assertion
   here is byte-stable truth.
6. **Determinism**: fixed cell order (doc order); no hash iteration.

## Tests required

1. `matrix_doc_rows_match_live_semantics` — the core: iterate the doc's
   cells, drive live behavior, assert the annotated verdict. Fails with
   the cell id on any mismatch.
2. `swept_carrier_boolean_cells_refuse_typed_today` — the G1 class
   pinned: every swept-carrier boolean cell asserts its typed refusal
   with envelope case (the census said so; prove it).
3. `authoring_cells_match_surface_rows` — the authoring cells (spline,
   circle, polyline) asserted against the S5/S6 surface rows; the
   missing Circle-profile row (audit G3) annotated as `unavailable`,
   `flipped-by: PB-011`.
4. `partial_arc_revolve_cell_annotated` — the
   `revolve(partial-arc,spline-profile)` cell annotated per the audit's
   G5 finding: current verdict, `flipped-by: PB-011`, target certified.

## Done when — run these, all must pass

```
cargo fmt --check -p truck123d
cargo clippy -p truck123d --all-targets -- -D warnings
cargo test -p truck123d --test pb_op_matrix
```

Scoped commands only, through the queue shim (the `cargo` on PATH IS the
shim). Send output to a file and read the tail.

## Forbidden

Anything outside write_allow — especially `facade.rs` (the matrix observes;
it does not change behavior), `vendor/truck/**`, landed test files,
`corpus/ttc/**` sources (read-only), `loop/results/**` (read-only). New
verdict vocabulary (SPEC_GAP against the compat surface doc). Adding
`#[ignore]`. Unjustified `#[allow]`. Committing to main.

## Stop conditions

- any anchor count differs → ANCHOR_MISMATCH
- a live verdict matches NO closed-vocabulary arm → SPEC_GAP against the
  compat surface doc (name the cell and the observed behavior)
- three consecutive failed cargo test runs on the same error → BLOCKED

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

```json
{"id":"PB-013-OP-CAPABILITY-MATRIX","status":"DONE","contracts":["PB-013-OP-CAPABILITY-MATRIX"],
 "tests_added":4,"anchors_verified":{"A1":3,"A2":93,"A3":1},
 "notes":"the cell set derived from the census; any audit verdict corrected by execution (cell id + observed); any deviation"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-DONE status also write `QUESTION.md` beside it.

Commit subject: `test(bridge): op capability matrix — per-cell verdict annotation matching live semantics (PB-013-OP-CAPABILITY-MATRIX)`.
