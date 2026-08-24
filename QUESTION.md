# QUESTION.md — BG-AUD-FIX-005 (status: SPEC_GAP)

## Why the packet stopped

The packet's decided repair form for `wedge_slope_lower_from_sin_margin` uses
`Interval::asin`:

```rust
let tt = Interval::try_from((sin_margin, sin_margin)).unwrap_or(Interval::EMPTY);
let value = crate::elementary::sin(tt.asin() / 2.0).inf();
```

Per the packet's stop conditions, I checked whether `Interval::asin` is
available on the pinned inari. It is not.

- Cargo.lock pins `inari 2.0.0`.
- `inari` only defines `Interval::asin` in `src/elementary.rs:211`, and that
  module is compiled only under `#[cfg(feature = "gmp")]`
  (`src/lib.rs:35-36`).
- Both in-tree dependents declare `inari = { version = "2.0",
  default-features = false }` (truck-evidence and truck-topology), so the
  `gmp` feature is off.
- A minimal offline probe crate against the same inari build fails to compile
  with `E0599: no method named asin found for struct Interval`.

`crate::elementary::sin` exists precisely because inari's own sin/cos are
gmp-gated (`elementary.rs:7-9`, `lib.rs:36-37`); the same gap applies to
`asin`. Enabling gmp is not viable on this machine: it pulls `gmp-mpfr-sys` and
`rug`, which build GMP/MPFR via autotools (`make`/`m4`) and do not list
`x86_64-pc-windows-gnullvm` as supported (see `elementary.rs:10-15`).

## What I need decided

1. **Approve the power-series fallback** so I can finish the packet as DONE:

   `sin(asin(s)/2) = s/2 + s^3/16 + 7 s^5/256 + 9 s^7/2048 + ...`

   All coefficients are positive for `s > 0`, so computing the terms in
   outward-rounded interval arithmetic and reading `.inf()` is a certified
   downward lower bound — the same guarantee the decided form was meant to
   give, with the same `InvalidMargin` guard and `scope`/return shape. The
   coefficient `7/256` corrects the packet's example `5/512` (see RESULT.json
   notes; verified twice against `sqrt((1 - sqrt(1 - s^2))/2)`).

2. **Enable the `gmp` feature** of inari on a toolchain that can build it, then
   the packet's exact decided form compiles unchanged.

3. **Vendor an `asin`** into `crate::elementary` mirroring the existing sin/cos
   construction (e.g. Newton/bisection against the certified sin on the
   monotone arc), so the packet's decided call shape works as written.

No source file was edited. Anchors matched (A1 = 1, A2 = 4), so this is not an
anchor mismatch. `vendor/truck/truck-evidence/src/fid/lfs.rs` is otherwise
unchanged and the tree still builds as before.
