# QUESTION.md — BG-AUD-FIX-005 (status: SPEC_GAP)

## Why the packet stopped (second dispatch, amended packet)

The amended packet replaced the `Interval::asin` form with a hybrid
series + closed-form repair, but its decided arithmetic does not compile
against the pinned inari:

```rust
let tt = Interval::try_from((sin_margin, sin_margin)).unwrap_or(Interval::EMPTY);
let one = Interval::try_from((1.0, 1.0)).unwrap_or(Interval::EMPTY);
let value = if sin_margin < 1.0e-6 {
    let s2 = tt * tt;
    let s3 = tt * s2;
    let s5 = s3 * s2;
    (tt / 2.0 + s3 / 16.0 + s5 * 7.0 / 256.0).inf()
} else {
    let inner = (one - (one - tt * tt).sqrt()) / 2.0;
    inner.sqrt().inf()
};
```

The packet asserts "`Interval / f64` and `Interval * f64` compile (inari
scales by a float)". They do not.

- Cargo.lock pins `inari 2.0.0`.
- inari 2.0.0 implements `Mul`/`Div` for `Interval` only against another
  `Interval` (`arith.rs:44`, `arith.rs:138`); there is no
  `impl Mul<f64> for Interval` and no `impl Div<f64> for Interval`.
- A probe compiling the exact arithmetic fails with
  `E0277: no implementation for 'Interval / {float}'` (four sites:
  `tt / 2.0`, `s3 / 16.0`, `s5 * 7.0`, `inner / 2.0`).
- In-tree confirmation: `vendor/truck/truck-evidence/src/decorators/
  intersection_curve.rs:189-193` documents that "inari implements `Mul` for
  `Interval` only, not for `f64` operands, so the scalar side is wrapped".

## What I need decided

1. **Approve wrapping the scalars as degenerate intervals** so I can finish the
   packet as DONE. Every `Interval / f64` and `Interval * f64` in the decided
   form becomes `Interval / iv_scalar` / `Interval * iv_scalar` where
   `iv_scalar = Interval::try_from((k, k)).unwrap_or(Interval::EMPTY)`. The
   coefficients `1/2`, `1/16`, `7/256` are exactly representable, so a
   degenerate-interval multiply/divide is exact outward-rounded scaling — the
   `.inf()` is bit-identical to what the packet's intended `Interval / f64`
   semantics would produce, and the pure-f64 `next_up` regression reference is
   unaffected.

2. **Re-issue the packet** with scalar-free interval arithmetic written against
   the operations inari 2.0.0 actually exposes.

No source file was edited. Anchors matched (A1 = 1, A2 = 4), so this is not an
anchor mismatch. `vendor/truck/truck-evidence/src/fid/lfs.rs` is unchanged and
the tree still builds as before.
