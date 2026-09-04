# QUESTION.md — CC-002-INJECTIVITY

**Status: STOP — packet stop condition (1) triggered.**

## What the packet requires

`construct/injectivity.rs`, Section 2 (`curve_injectivity_radius`): the curve
variant computes `L = sup ‖C″‖` over the queried region as a certified
"second-derivative bound (Bernstein derivative 1-D hull over the curve's Bézier
pieces)" — i.e. per Bézier piece, differentiate the piece's Bernstein
coefficient vectors twice (`bernstein_derivative_1d` composed) and hull the
resulting source second-derivative coefficients over the region overlap. That
requires the per-piece Bernstein coefficient vectors of the curve map.

## What the landed accessors provide

The map types own the Bézier decomposition (`certified_map.rs`). The public
accessor surface is asymmetric:

- `CertifiedSurfaceMap` exposes `patch_boxes()` and `patch_grids()`
  (`certified_map.rs:399,407`) — the per-patch tensor coefficient grids are
  reachable, so the surface second-derivative grid is reachable by composing
  `hull::bernstein_derivative_2d` on the grids (Section 1 is implementable).
- `CertifiedCurveMap` exposes only `enclosure()`, `rank_margin()`, and
  `piece_intervals()` (`certified_map.rs:294,321,345`). The per-piece
  coefficient vectors live in the private `CurvePiece { interval, coeffs }`
  (`certified_map.rs:148-152`), and the map stores no handle to the admitting
  `BSplineCurve<Point3>`. No public API returns the curve pieces' Bernstein
  coefficient vectors. The curve's second-derivative grid is therefore NOT
  reachable through the landed accessors.

`rank_margin` returns only a certified interval lower bound of `|C'|` over a
region (`certified_map.rs:321-341`); it does not expose derivative coefficient
data, and no other public path reconstructs the pieces.

## Why this is a stop, not a workaround

Packet stop condition (1): "the map types own the Bézier decomposition; if the
second-derivative grid is NOT reachable through the landed accessors, STOP and
file QUESTION.md (that is a spine seam defect, not something to work around by
re-deriving decomposition)". Re-deriving the decomposition is unavailable here
anyway (the `&CertifiedCurveMap` argument carries no curve handle and the piece
coefficients are private), and adding a curve coefficient accessor would edit
`certified_map.rs`, which is outside this packet's `write_allow`.

## Proposed amendment (minimal, curve-only)

Mirror the surface accessor on `CertifiedCurveMap`, in the same module and
style:

```rust
/// The per-piece, per-coordinate Bernstein coefficient vectors of the map, in
/// the same order as [`Self::piece_intervals`] (D-map structural accessor; the
/// CC-002 curve injectivity radius consumes these to bound ‖C″‖).
pub fn piece_grids(&self) -> Vec<[Vec<f64>; 3]> {
    self.pieces.iter().map(|piece| piece.coeffs.clone()).collect()
}
```

Add that one method to `certified_map.rs`, widen CC-002's `write_allow` to
include `vendor/truck/truck-certified/src/certified_map.rs`, and redispatch.
CC-005 (which already declares a needs-edge to `curve_injectivity_radius`) is
blocked on the same seam.

## Pre-stop verification

- Anchors A1..A5 all matched the expected counts before this stop (A1=2,
  A2=1, A3=1, A4=1, A5=1).
- No kernel code was written; the worktree is clean apart from this file and
  the dispatch-provided `PACKET.md` / `CONTEXT.md`.
