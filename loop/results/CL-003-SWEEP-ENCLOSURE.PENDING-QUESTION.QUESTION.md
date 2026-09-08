# QUESTION — CL-003-SWEEP-ENCLOSURE: anchor A4 count mismatch (`num/mod.rs` `^pub mod` = 4, packet expects 3)

## Status

**STOP — ANCHOR_MISMATCH (stop condition: "any anchor count differs → ANCHOR_MISMATCH").**
No kernel code was written; the stop fired pre-build at zero cost.

## The mismatch

The packet's anchor table (measured 2026-09-05) requires:

| id | file | pattern | expect | actual |
|---|---|---|---|---|
| A1 | `truck-geometry/src/constructive/sweep_surface.rs` | `pub struct SpineFrameSweep` | 1 | 1 ✓ |
| A2 | `truck-geometry/src/constructive/sweep_surface.rs` | `impl ParametricSurface for SpineFrameSweep` | 1 | 1 ✓ |
| A3 | `truck-evidence/src/enclosure.rs` | `pub trait EnclosureSurface` | 1 | 1 ✓ |
| A4 | `truck-evidence/src/num/mod.rs` | `^pub mod` | **3** | **4 ✗** |

`vendor/truck/truck-evidence/src/num/mod.rs` carries **four** `pub mod`
declarations on this tree:

```
L23  pub mod cluster;        (BG-NUM-004)
L26  pub mod krawczyk;       (BG-NUM-003)
L31  pub mod parallelotope;  (BIE-002-SSI4)
L34  pub mod roots;          (BG-NUM-002)
```

each backed by a real module file. The packet expects three and notes "A4
becomes 4 when you add `pub mod sweep_sigma;`" — i.e. it was authored against
a base where `parallelotope` had not yet landed in `num/mod.rs`.

## Why this is a stop, not a workaround

- The anchor instruction is unconditional: "Locate by pattern, never by line
  number. If a count differs, STOP and report `ANCHOR_MISMATCH`." A4 differs
  (4 ≠ 3), so the worker halts and reports; it does not re-base the anchor
  itself.
- The RESULT.json template's `anchors_verified` also encodes the stale base
  (`"A4":4` after adding `sweep_sigma`, i.e. base 3). The true tree would
  produce A4 = 5 after the addition.
- This is the same "authoring rot" class already recorded in the loop session
  record (commit `8002d23`): CTE-000 r1 stopped at an identical mismatch
  (lib.rs pub-mod total drifted 13 → 16 under three landings), the worker
  stopped pre-build at zero cost, and the loop rotated the anchor to
  base-relative before redispatch. The parallelotope module (`dae7c7f`,
  BIE-002-SSI4, 2026-09-05 13:18) landed after the CL-003 anchors were
  measured and before this dispatch, so the A4 count drifted 3 → 4 under a
  landing outside this packet's write set.

## Proposed amendment (loop-side, minimal)

Rotate anchor A4 to a base-relative count of **4** (and the RESULT template's
`anchors_verified.A4` to **5**), or pin A4 to a less drift-prone pattern
(e.g. the specific `pub mod parallelotope;` line plus a total), then redispatch
CL-003-SWEEP-ENCLOSURE unchanged. Nothing in this packet's design, scope
decisions, write set, or tests is affected by the drift — the enclosure work
itself is untouched.

## Pre-stop verification

- A1/A2/A3 match the packet exactly (counts 1, 1, 1).
- A4 measured twice (ripgrep `^pub mod` and `Select-String`, plus file read):
  exactly 4 module declarations (`cluster`, `krawczyk`, `parallelotope`,
  `roots`), each with a backing file under `truck-evidence/src/num/`.
- No code was written. The worktree is clean apart from the dispatch-provided
  `PACKET.md` / `CONTEXT.md`, this `QUESTION.md`, and `RESULT.json`.
