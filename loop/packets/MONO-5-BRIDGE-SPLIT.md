# WORK PACKET MONO-5-BRIDGE-SPLIT — partition bd_bridge.rs into a module directory

**STATUS: ON DECK, NOT REGISTERED (owner steering 2026-09-10: no same-file
parallelism; serial tail by default).** Dispatch only if the serial
bd_bridge.rs tail (MONO-3 -> MONO-4) is MEASURED as the program bottleneck —
and its purpose is conflict-avoidance (packets own disjoint files), never
worker stacking on one file.

`truck123d/src/bd_bridge.rs` is 182 KB and the loop's central write corridor:
every MONO-CLOSURE packet (loft arm, members, mirror, trim constructor, and
the coming boolean wave) lands in it, so the dispatcher serializes otherwise
independent packets on one file. This packet performs the MECHANICAL
partition into `truck123d/src/bd_bridge/` so subsequent packets own disjoint
modules. It is a move-only packet: zero semantic changes, zero behavior
changes, the public surface byte-identical.

```yaml
id:          MONO-5-BRIDGE-SPLIT
contract:    [MONO-5-BRIDGE-SPLIT]
class:       mechanical
crates:      [truck123d]
depends_on:  [MONO-2-NSTATION-LOFT, MONO-1-DATA-ROWS]
write_allow:
  - truck123d/src/bd_bridge.rs
  - truck123d/src/bd_bridge/
read_allow:
  - truck123d/src/lib.rs
  - truck123d/src/facade.rs
  - truck123d/src/binding.rs
tests_required: []
anchors:
  - {id: A1, expect: 4,   cmd: "grep -c 'pub fn' truck123d/src/bd_bridge.rs"}
  - {id: A2, expect: 145, cmd: "grep -c 'fn ' truck123d/src/bd_bridge.rs"}
  - {id: A3, expect: 1,   cmd: "grep -c 'mod bd_bridge' truck123d/src/lib.rs"}
budget:      {turns: 40, ctx_tokens: 120000}
```

## Method

1. **Partition by carrier family**, following the packet graph so future
   write sets are disjoint: `bd_bridge/loft.rs` (the loft arms + span
   machinery), `bd_bridge/facts.rs` (bbox/validity/face-count data rows),
   `bd_bridge/trim.rs` (the trim-extrude constructor), `bd_bridge/sweep.rs`
   (sweep members + mirror when they land — create the module with the moved
   sweep-adjacent helpers), `bd_bridge/mod.rs` (the shared types:
   `SpanPoly3`, `BooleanNode`, `Refusal` mapping, re-exports preserving the
   module's public surface exactly). Shared private helpers move to
   `bd_bridge/shared.rs` if used by more than one family.
2. **Move-only discipline.** `git mv` semantics: functions move verbatim; the
   only new lines are `mod`/`pub(crate) use` declarations in `mod.rs`. The
   public surface of `bd_bridge` (the 4 `pub fn`s + public types) is
   untouched — `facade.rs`, `binding.rs` and tests compile without edits
   beyond `use` path resolution if needed (prefer `pub use` re-exports in
   `mod.rs` so NO caller edits at all).
3. **Verify the move is a move.** `cargo check --locked -p truck123d` clean;
   the full lib suite serial green (the same counts as the pre-split baseline
   — no test result changes); `git diff --stat` shows only the expected
   renames plus `mod.rs` declarations.
4. All cargo through the queue. fmt/clippy clean on the touched lines (the
   move itself introduces no new findings; pre-existing findings on moved
   lines keep their baseline attribution per the V3 rename rule).

## Done when

- check + full lib tests green serial at the split HEAD; no test renamed,
  removed, or weakened.
- A1/A2 hold by COMMAND at the split HEAD counting across the whole
  `bd_bridge/` directory (the counts move with the code — run
  `grep -rc 'pub fn' truck123d/src/bd_bridge/ | sum` etc. and record the
  numbers in RESULT.json notes; A3 stays 1 — the module declaration path is
  unchanged).
- The future write-set map is recorded in RESULT.json notes: which new module
  each open packet (MONO-3 members, MONO-4 trim, wave-3 booleans) will own.
- `docs/MONO_CLOSURE_BOOKING.md` annex B row updated? NO — out of scope; the
  orchestrator updates the booking doc at landing.

## Stop conditions

- If any partition boundary cannot be drawn without a semantic edit (a
  private helper entangled across families), stop, record the entanglement —
  the orchestrator adjudicates the partition before the packet proceeds.
  Never resolve an entanglement by changing behavior.
- The pre-existing `indexing_slicing` baseline findings must move with their
  code unattributioned — if a moved file's clippy baseline changes, stop and
  record.

Write RESULT.json AT THE WORKTREE ROOT.
