# QUESTION.md — CL-000-SPLINE-ADMIT

**Status: STOP — anchor mismatch (A4).**

CL-000-SPLINE-ADMIT stopped before any code was written because anchor A4 does
not hold on this dispatch tree. Per the packet rule ("Locate by pattern, never
by line number. If a count differs, STOP and report `ANCHOR_MISMATCH`"), the
work is reported, not improvised.

## The measured anchors (re-derived by command on this dispatch)

| id | file | pattern | expected | measured |
|---|---|---|---|---|
| A1 | `vendor/truck/truck-certified/src/ssi.rs` | `pub fn construct_square_system` | 1 | 1 |
| A2 | `vendor/truck/truck-certified/src/hull.rs` | `pub fn hull_bernstein_2d` | 1 | 1 |
| A3 | `vendor/truck/truck-certified/src/lib.rs` | `^pub mod` | 15 | 15 (DELTA +1 → 16 with `pub mod patch_admit;`) |
| A4 | `vendor/truck/truck-geometry/src/nurbs/bspsurface.rs` | `pub struct BSplineSurface` | 1 | **0** |

## What A4 actually is on this tree

`vendor/truck/truck-geometry/src/nurbs/bspsurface.rs` is impl-only. It contains
no `pub struct` line at all (verified with `Select-String` for `pub struct` and
for the exact pattern `pub struct BSplineSurface`). The only occurrence of the
looser text `struct BSplineSurface` in that file is the private test struct
`BSplineSurface_<P>` at line 2108 (a trailing underscore — not the anchor
pattern).

The single declaration the packet is pointing at lives one file over:

```rust
// vendor/truck/truck-geometry/src/nurbs/mod.rs:162
pub struct BSplineSurface<P> {
    knot_vecs: (KnotVec, KnotVec),
    control_points: Vec<Vec<P>>,
}
```

Count of `pub struct BSplineSurface` in `nurbs/mod.rs` is exactly 1, and 1
across the whole `truck-geometry` crate.

## Why this is reported rather than absorbed

- The A4 count genuinely differs (0 ≠ 1) on the file the packet names, and the
  packet's stop-condition language is absolute for anchors other than the A3
  DELTA carve-out (which is about a *concurrent landing* moving `^pub mod`,
  not about a mis-pointed file column).
- Git history shows the struct has lived in `nurbs/mod.rs` since the vendoring
  commit `da72cd5` ("build: vendor truck as the BG-generation kernel"). It did
  not move recently and no other program landed a change that could have
  produced this: this is a packet-side file mis-specification, i.e. exactly the
  class of drift anchors exist to catch before code is written against a wrong
  assumption about where `BSplineSurface` is declared.
- The rest of the packet's surface is unaffected and ready: A1/A2/A3 hold, the
  write set is untouched, and no kernel code was written (`enclosure.rs`,
  `patch_admit.rs`, `lib.rs` all unmodified).

## Proposed resolution

Re-point A4 at `vendor/truck/truck-geometry/src/nurbs/mod.rs` (measured count
there is exactly 1) and redispatch CL-000-SPLINE-ADMIT unchanged. Everything
else in the packet — the three required tests, the write set, the bidegree
budget, and the `hull_bernstein_2d`-based enclosure discipline — is unaffected
by the re-point: `BSplineSurface`'s public methods are implemented in
`bspsurface.rs` and reachable through `nurbs::BSplineSurface` exactly as the
packet's read set assumes.
