# WORK PACKET PB-014-AUTHORING-REVOLVE — circle-profile authoring + partial-arc revolve (the corpus authoring prerequisites)

You are landing the two authoring capabilities the parity audit named as
missing-facade/missing-coverage, ahead of the parity churn so the churn
lifts rows instead of building prerequisites. Everything you need is in
this document, `docs/OP_CAPABILITY_MATRIX.md` (cells 10 and 19 — the two
you flip), `loop/results/PB-010-TTC-PARITY-AUDIT.json` (G3/G5 findings),
and the anchors. Do not read other spec files. A genuine gap is a
SPEC_GAP: stop and report.

```yaml
id:          PB-014-AUTHORING-REVOLVE
contract:    [PB-014-AUTHORING-REVOLVE]
class:       mechanical
crates:      [truck123d]
depends_on:  [PB-013-OP-CAPABILITY-MATRIX]
write_allow:
  - truck123d/src/facade.rs
  - docs/OP_CAPABILITY_MATRIX.md
  - truck123d/tests/pb_op_matrix.rs
  - docs/PY_BRIDGE_COMPAT_SURFACE.md
  - corpus/ttc/SKIPS.json
read_allow:
  - truck123d/src/
  - truck123d/compat/
  - truck123d/tests/pb_op_matrix.rs
  - docs/OP_CAPABILITY_MATRIX.md
  - docs/PY_BRIDGE_COMPAT_SURFACE.md
  - loop/results/PB-010-TTC-PARITY-AUDIT.json
  - corpus/ttc/
anchors:
  - {id: A1, expect: 4, cmd: "grep -cF 'author(circle-profile)' docs/OP_CAPABILITY_MATRIX.md"}
  - {id: A2, expect: 2, cmd: "grep -cF 'revolve(partial-arc' docs/OP_CAPABILITY_MATRIX.md"}
  - {id: A3, expect: 3, cmd: "grep -c NonCanonicalCarrier truck123d/src/facade.rs"}
  - {id: A4, expect: 1, cmd: "grep -cF 'boolean sectioning' corpus/ttc/SKIPS.json"}
budget:      {turns: 50, ctx_tokens: 120000}
```

H-1: no unwrap/expect without a justified same-line opt-out. H-3
same-line `// H-3`. H-6: never record Float as Exact.

## Problem — the audit's G3 and G5 findings

- **G3 (cell 19, `author(circle-profile)`):** the corpus authors closed
  circle profiles (`f1/src/lib/cockpit.py:circle_section`,
  merlin rings) but the facade has no Circle-profile authoring entry —
  S5 names Spline authoring only. The matrix cell is `unavailable`.
- **G5 (cell 10, `revolve(partial-arc,spline-profile)`):** the falcon
  cutaway builds its section via **partial-arc revolves**
  (`falcon_common.py:202-206, :277-282` — `arc_deg`/`start_deg`), and
  the facade's revolve has no arc parameters. The matrix cell is
  `unavailable`. SKIPS.json's cutaway note mis-describes this as
  "boolean sectioning".

## Scope decisions — pre-made, do not relitigate

1. **Kernel check already done (orchestrator, 2026-09-06):** the landed
   `RevolvedSurface` v-range is hardwired `[0, 2π)`
   (`revolved_curve.rs:152`, `by_revolution` :321 takes no angles) — and
   that is sufficient. A partial-arc revolve is a BRIDGE-LEVEL
   construction: build the full revolved surface, then emit a shell of
   trimmed faces whose v-range is `[start, start+arc]` plus TWO PLANAR
   CAP FACES bounded by the profile and its rotated image. No
   `vendor/truck` change is expected or permitted.
2. **Circle-profile authoring** mirrors the landed Spline authoring entry
   (S5): a closed-circle profile carrier + `make_face`-compatible
   output, feeding the landed revolve. Same table-row shape as the
   spline authoring cells; add the S5 surface-row mention
   (`docs/PY_BRIDGE_COMPAT_SURFACE.md` — extend S5's row text, do not
   add an 8th surface id).
3. **Matrix flips are the deliverable.** Flip cell 19 and cell 10 in
   `docs/OP_CAPABILITY_MATRIX.md` (`current: certified`,
   `flipped-by: PB-014`) AND the corresponding assertions in
   `truck123d/tests/pb_op_matrix.rs` TOGETHER, one commit — the matrix
   test must stay green against live behavior. These are the sanctioned
   V5 edits; no other cell may change.
4. **SKIPS.json note correction (G5):** the `falcon_heavy/cutaway` row's
   note changes from the boolean description to the true code path
   (partial-arc revolves). The ROW STAYS (its lift is PB-011's, pending
   the green door run) — only the reason's descriptive text is corrected
   to match the vendored code. Do not remove rows or change reason ids.
5. **Determinism/cargo rules** as every packet: queue shim, scoped
   commands, fixed orders.

## Tests required

1. `circle_profile_authors_and_feeds_revolve` — the cockpit ring shape:
   a closed circle profile authors through the new entry and feeds the
   landed full revolve, producing the expected geometry.
2. `partial_arc_revolve_builds_capped_shell` — a spline-carrier profile
   revolved over `[start, start+arc]` (the falcon cutaway shape):
   shell topology = swept wall + two planar caps, angles honored, and
   the solid is watertight (the caps close it).
3. `matrix_cells_flipped_certified` — cells 10 and 19 assert `certified`
   against live behavior, matching the updated doc rows.

## Done when — run these, all must pass

```
cargo fmt --check -p truck123d
cargo clippy -p truck123d --all-targets -- -D warnings
cargo test -p truck123d --test pb_op_matrix
cargo test -p truck123d
```

Scoped commands only, through the queue shim (the `cargo` on PATH IS the
shim). Send output to a file and read the tail.

## Forbidden

Anything outside write_allow — especially `vendor/truck/**` (the kernel
check is done; the construction is bridge-level), landed test files other
than the sanctioned matrix-cell flips, `compat/surface.rs`,
`ttc_harness.rs`, the other 24 matrix cells. Adding `#[ignore]`.
Unjustified `#[allow]`. Committing to main.

## Stop conditions

- any anchor count differs → ANCHOR_MISMATCH
- the trimmed-shell construction requires a kernel change (e.g. the
  revolved surface cannot be trimmed to a v-range through the landed
  face API) → SPEC_GAP naming the exact limitation
- three consecutive failed cargo test runs on the same error → BLOCKED

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

```json
{"id":"PB-014-AUTHORING-REVOLVE","status":"DONE","contracts":["PB-014-AUTHORING-REVOLVE"],
 "tests_added":3,"anchors_verified":{"A1":1,"A2":1,"A3":3,"A4":1},
 "notes":"the cap-face construction approach; the trimmed-face API used; any deviation"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-DONE status also write `QUESTION.md` beside it.

Commit subject: `feat(bridge): circle-profile authoring + partial-arc revolve — corpus authoring prerequisites (PB-014-AUTHORING-REVOLVE)`.
