# WORK PACKET MONO-7-ROW-ASSEMBLY — group/styled/clean pass-throughs + per-row facts + timing

The row-assembly layer of the MONO-CLOSURE program (booking table row
"the monocoque row itself end to end", renumbered after the membership
packet). The monocoque row is `{"group": [styled(shell), styled(hoop)]}`
(mono_tub.py:748-749): today the bridge aggregates group facts but emits
NO per-child breakdown, NO timing, and would REFUSE the styled metadata
if the door ever sent it (`deny_unknown_fields`, bd_bridge.rs SolidSpec
container). This packet makes the row-assembly vocabulary pass through
exactly, records per-row facts and timing columns, and proves it on a
monocoque-shaped synthetic assembly. The Swept×Swept cut wiring is NOT
in this packet — it stays the admission chain's cell, and the test
records the typed refusal as the measured boundary.

```yaml
id:          MONO-7-ROW-ASSEMBLY
contract:    [MONO-7-ROW-ASSEMBLY]
class:       design
crates:      [truck123d]
depends_on:  [MONO-6-SWEPT-BOOLEANS, MONO-3-BLADE-MEMBERS-MIRROR]
write_allow:
  - truck123d/src/bd_bridge.rs
  - corpus/ttc/door.py
  - truck123d/tests/mono_row_assembly.rs
read_allow:
  - truck123d/src/binding.rs
  - truck123d/src/facade.rs
  - corpus/ttc/trees/f1/src/lib/mono_tub.py
  - corpus/ttc/trees/f1/src/lib/surfaces.py
  - docs/MONO_CLOSURE_BOOKING.md
tests_required: [truck123d/tests/mono_row_assembly.rs]
anchors:
  - {id: A1, expect: 0, cmd: "grep -c 'styled' truck123d/src/bd_bridge.rs"}
  - {id: A2, expect: 0, cmd: "grep -c 'construct_ms' truck123d/src/bd_bridge.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'def styled' corpus/ttc/trees/f1/src/lib/surfaces.py"}
  - {id: A4, expect: 1, cmd: "grep -c 'def clean' corpus/ttc/door.py"}
budget:      {turns: 55, ctx_tokens: 160000}
```

## Pre-made judgements (do not relitigate; deviations go in RESULT notes)

1. **Metadata is metadata.** `label`/`color` are carried verbatim and are
   never read by any semantic path (classification, facts arithmetic,
   meshing). Bridge side: `PartSpec` gains `#[serde(default)]
   #[serde(skip_serializing_if = "Option::is_none")] pub label: Option<String>`
   and the same for `color: Option<String>`; the group node variant gains
   the same two optional fields. `deny_unknown_fields` STAYS on — the
   fields are added to the structs, not removed from the guard. The Facts
   struct mirrors them into the emitted JSON verbatim.
2. **The door serializes the metadata.** `_Part._node()` (door.py:1073)
   adds `"label"` / `"color"` keys when set (omit when unset — absent key,
   never null); `Compound._node()` (door.py:1282) adds the group's
   `"label"` the same way. `clean()` (door.py:1009) stays the identity it
   is; its docstring gains one sentence stating that identity is the
   recorded semantics (no bridge round-trip).
3. **Per-row facts, immediate children only.** For a group top node,
   `bd_facts` emits `"rows": [...]` — one entry per IMMEDIATE child of the
   group: `{"label": <when present>, "solid_count": n, "volume": v,
   "bbox": b}` using the same arithmetic as the aggregate path
   (`count_and_union` / `top_volume`, bd_bridge.rs:2972-3029). Nested
   groups appear as ONE row with their aggregate (the nested-group
   weights-zero rule at :3014-3026 is unchanged). Absent for a part top
   node. Aggregates are unchanged — this is an addition, never a redefinition.
4. **Timing columns, recorded never gated.** `bd_facts` emits
   `"timing": {"construct_ms": f64, "facts_ms": f64}` (wall-clock around
   the construct and facts phases); `bd_stl` emits `"mesh_ms"`. Values are
   diagnostic columns for the census/BENCHMARKS protocol; NO gate, test
   assertion, or tolerance ever compares them except `>= 0.0`.
5. **V5 discipline.** The classifier path is untouched: a styled/clean
   row changes nothing about carrier class (`solid_carrier_class`
   :2808-2844 reads geometry only). Existing tests must stay green
   bit-for-bit on the unchanged keys.

## Method

1. Read `bd_bridge.rs` facts/mesh/tree paths first (the survey's line map:
   TreeNode :367, Facts :387, tree_facts :2915, tree_mesh :3043, bd_facts
   :5727, bd_stl :5758). Add the metadata fields, the per-row breakdown,
   and the timing wrappers.
2. door.py: metadata serialization per judgement 2. The door's existing
   tests (the ttc_* battery) must stay green.
3. Tests in `truck123d/tests/mono_row_assembly.rs`:
   - a labelled group of three parts (one spline-station loft at the tub's
     42-station scale, one blade member, one mirrored member) submits,
     facts match the aggregate path AND carry `rows` with per-child
     values, labels round-trip, timing fields present and `>= 0.0`;
   - a nested group appears as one row with its aggregate;
   - a part node with NO metadata omits the keys (absent, never null);
   - the monocoque-shaped boolean (loft cut by its cavity loft) refuses
     typed at the boolean boundary naming the carrier — recorded as the
     boundary, not a failure.
4. All cargo through the queue (the `cargo` on PATH IS the queue shim).
   Scoped checks only: `cargo check -p truck123d --lib --locked` and the
   new test file plus the existing ttc battery files the door change
   touches. The test binary needs `C:\Program Files\PyManager\runtime` on
   PATH to run on this host (documented DLL_NOT_FOUND workaround) — run
   the built binary directly if the cargo-driven run reports 0xc0000135.

## Done when

`cargo check -p truck123d --lib --locked` green; the new test file passes
serially; the door-side ttc battery files that exercise _node() pass;
anchors hold (A1/A2 grow from 0, A3/A4 unchanged at 1). Write
RESULT.json AT THE WORKTREE ROOT (the slot worktree root, not
loop/results/ — the orchestrator files it).

## Forbidden

No kernel (`vendor/truck/**`) changes — the binding surface already
carries everything this packet needs. No facade.rs changes (scheduling
kept MONO-7 and the fillet/halo arms disjoint where it matters). No
timing assertions beyond `>= 0.0`. Do not "improve" clean() into a
bridge call.
