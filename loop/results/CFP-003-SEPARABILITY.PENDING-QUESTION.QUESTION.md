# QUESTION.md — CFP-003-SEPARABILITY

**Worker verdict: SPEC_GAP.** This packet cannot be executed to a green
`DONE` on the dispatched tree without editing files outside its `write_allow`
(which the packet's Forbidden clause forbids) or leaving the 4-axis
materialization in place (which scope decision 1 and anchor A2 forbid). The
packet's consumer census is incomplete for the current tree; the fix is an
orchestrator amendment widening `write_allow` (or re-scoping the removal), then
a redispatch.

## What the packet requires

1. Scope decision 1 + anchor A2: `SquareSystem3` drops its stored field
   `grids: [Vec<Vec<f64>>; 3]` (pattern count 1 -> 0 in `ssi_types.rs`) and the
   struct carries per-side patch representations instead.
2. Scope decision 3 + anchor A3: `from_system_grid` (pattern count >0 -> 0 in
   `tangency/minors.rs`) is removed; the landed `det3`-of-interval-Jacobian path
   is replaced by Corollary-1.3 per-side normal/tangent dot products wherever it
   consumed the materialized grid.
3. Scope decision 6: the consumers named (`ssi_trace`, `ssi_admit`,
   `tangency/{minors,exclude,tsystem}.rs`) are carried across new per-side
   accessors in THIS packet, so the program does not land a half-refactored
   state.

## The blocker (empirically verified on the dispatched tree)

The materialized four-axis `SquareSystem3` grids are consumed by production code
in **ten files that are NOT in the packet's `write_allow`**, in every case
through the exact API surface the packet removes:

| consumer file | API it needs that the packet removes |
|---|---|
| `kernel/engine.rs` | `system.grids()[component]` |
| `kernel/tracer.rs` | `sys.grids().get(component)` |
| `kernel/claims.rs` | `SquareSystem3::new(grids, degrees, maps)` |
| `ssi_fixtures.rs` | `SquareSystem3::new(...)`, `system.grids()` |
| `tangency/hessian.rs` | `minors::from_system_grid(&system.grids()[..], ..)` |
| `tangency/cascade.rs` | `minors::from_system_grid(...)`, `system.grids()` |
| `tangency/chart.rs` | `system.grids().get(..)`, `SquareSystem3::new(...)` |
| `tangency/graph.rs` | `&system.grids()[row]` |
| `tangency/a2.rs` | `system.grids()`, `SquareSystem3::new(...)` |
| `tangency/gates.rs` | `SquareSystem3::new(grids, degrees, maps)` |

The packet's `write_allow` names only `ssi_types.rs`, `ssi.rs`,
`ssi_admit.rs`, `ssi_trace.rs`, `tangency/{minors,exclude,tsystem}.rs`, and
`patch_admit.rs`.

**Probe.** I removed the `grids` field + accessor from `SquareSystem3` (the A2
delta) and ran `cargo check -p truck-certified`. The crate fails to compile in
14 modules, including every non-writable consumer above:

```
kernel/engine.rs  kernel/tracer.rs  ssi_fixtures.rs
tangency/a2.rs  tangency/cascade.rs  tangency/chart.rs  tangency/hessian.rs
(plus the writable ssi.rs / ssi_admit.rs / ssi_trace.rs / exclude.rs / tsystem.rs / minors.rs)
```

The probe edit was fully reverted; the tree is clean. Baseline `cargo check -p
truck-certified` is green at the dispatch commit, so this is not a pre-existing
breakage.

Consequences that make the packet unsatisfiable as written:

- **A2 cannot reach 0 while the crate compiles.** `system.grids()` returns
  `&[Vec<Vec<f64>>; 3]`. A reference-returning accessor requires the data to
  live in the struct, so removing the field without editing the ten consumers
  above is a compile error, and editing them is Forbidden.
- **A3 cannot reach 0 while the crate compiles.** `tangency/hessian.rs` and
  `tangency/cascade.rs` `use crate::tangency::minors::from_system_grid`. The
  symbol must stay defined in `minors.rs` for those files (which are owned by
  CTE-004, not by this packet), so the anchor's expected post-change count of 0
  is unreachable.
- The packet's own "Done when" (`cargo test -p truck-certified --lib ssi` and
  `--lib tangency`) compiles the whole crate, so the out-of-scope consumers
  cannot be dodged by scoping the run.

## Why this is a packet defect, not a worker reading

The design spec (`docs/CONTACT_FAST_PATH_BUILD_SPEC.md` §1, §4) states "Grid
consumers CFP-003 carries: `ssi_trace`, `ssi_admit`, CTE
`minors`/`exclude`/`tsystem` (via `from_system_grid`)". On the dispatched tree
that census is incomplete: CTE-004's `hessian.rs`/`cascade.rs` (landed
`7609e86`), CTE-002's `chart.rs`/`graph.rs`, CTE-005's `a2.rs`, CTE-008's
`gates.rs`, the SSI fixture kit `ssi_fixtures.rs`, and the kernel-v2
`kernel/{claims,engine,tracer}.rs` all consume the materialized grid through the
same API surface this packet deletes. All are outside `write_allow` and all are
owned by other (landed or later) packets; the one-writer map gives CFP-003 only
the tangency trio among them.

## Amendment needed for a redispatch

Widen `write_allow` (and the scope-decision-6 consumer list) to carry every
grid consumer on the current tree — at minimum
`tangency/{hessian,cascade,chart,graph,a2,gates}.rs`,
`kernel/{claims,engine,tracer}.rs`, and `ssi_fixtures.rs` — or re-scope
decision 1/6 (e.g. keep a reference-returning materialized view for the
out-of-scope CTE/kernel consumers while the SSI hot path and the tangency trio
go per-side) and re-state anchors A2/A3 to the counts that reachable state
produces. No code was changed by this worker; no test names from
`tests_required` were added because no production delta could be landed.
