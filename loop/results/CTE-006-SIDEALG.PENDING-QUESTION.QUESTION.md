# STOP: ANCHOR_MISMATCH (CTE-006-SIDEALG)

Anchor A5 is unmet on this branch and stops the packet before any build.

## What the packet expects

| anchor | file | pattern | expect |
|---|---|---|---|
| A5 | `vendor/truck/truck-certified/src/tangency/shapes.rs` | `pub struct SideState` | 1 |

## What this tree shows

- `vendor/truck/truck-certified/src/tangency/shapes.rs` does not exist.
- The `vendor/truck/truck-certified/src/tangency/` directory does not exist.
- No `SideState` definition exists anywhere under `vendor/` (grep of the whole
  vendor tree: zero matches).

So the measured count is A5 = 0, not 1.

## Why this is not a SPEC_GAP and not a worker error

`SideState` is CTE-000-SPINE's write target (`truck-certified/src/tangency/
{mod,shapes,fixtures}.rs`). CTE-000-SPINE has not landed: this branch
(`packet/CTE-006-SIDEALG`) is forked at `7a7ff96`, and the registered packet
row `depends_on: [CTE-000-SPINE]` is unsatisfied. This is exactly the
guaranteed-ANCHOR_MISMATCH dispatch that the orchestrator recorded in
`aaf7fff` ("its anchor A5 reads SideState from tangency/shapes.rs - a CTE-006
dispatch before CTE-000 lands is a guaranteed ANCHOR_MISMATCH, caught before
the fourth bounce").

The other anchors re-derived clean on this tree: A1 = 1
(`pub fn fragment_decision`, boolean/mod.rs:103), A2 = 1
(`pub struct MaterialState4`, boolean/mod.rs:75), A3 = 1
(`pub enum BoolOp`, boolean/mod.rs:45), A4 = 1
(`pub struct CoincidentPair`, boolean/split.rs:188).

## Action taken

No file was edited (write_allow is only `boolean/mod.rs`, and the refactor
cannot start without the missing carrier type). No cargo command was run.
`RESULT.json` reports `status: ANCHOR_MISMATCH` with `anchors_verified`
A5 = 0.

## What the loop should do

Do not redispatch CTE-006-SIDEALG until CTE-000-SPINE lands and `SideState`
exists in `truck-certified/src/tangency/shapes.rs`. On that tree the
refactor's exhaustion battery (lossless `MaterialState4 <-> (SideState,
SideState)`, §5.9 truth rows, difference-non-associativity pin, and the
fragment_decision congruence gate) can proceed byte-identically as the packet
specifies.
