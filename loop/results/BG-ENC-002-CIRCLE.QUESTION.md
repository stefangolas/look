# QUESTION.md — BG-ENC-002-CIRCLE (SPEC_GAP)

## Status

SPEC_GAP. Implementation stopped at the first compilation gate; the packet's
"Done when" checks cannot pass as specified.

## The gap

The packet mandates outward-rounded interval trigonometry on `inari::Interval`
(decisions 2 and 3, and the `circle_trig_extrema_inside_interval` test):

- `tt.cos()`, `tt.sin()` for `enclose`
- the `n % 4` derivative cycle via the same `inari` trig calls
- "`inari::Interval` provides outward-rounded `sin()`/`cos()`; use them, never
  endpoint-only evaluation."

Those primitives do not exist under the locked dependency configuration:

- `inari` 2.0.0 defines `Interval::sin()`/`Interval::cos()` only in
  `src/elementary.rs`, which is `#[cfg(feature = "gmp")]` (lib.rs:35).
- `truck-evidence` declares `inari = { version = "2.0", default-features = false }`
  (`vendor/truck/truck-evidence/Cargo.toml:9`), which disables `gmp`.
- No other workspace crate depends on `inari`, so feature unification never
  turns `gmp` on.
- `cargo clippy -p truck-evidence --all-targets --no-deps -- -D warnings` fails:
  `error[E0599]: no method named 'sin'/'cos' found for struct 'inari::Interval'`.

## What I verified

- Anchors A1..A5 all match (1, 1, 1, 1, 4).
- All allowed read files inspected; `plane.rs` pattern followed.
- A complete draft of `circle.rs` (impl + the five required tests) was written
  and fails only on the missing `Interval::sin`/`Interval::cos`.
- `circle.rs` has been restored to its original scaffolding; the repository is
  left buildable.

## What I need

A decision on how the outward-rounded trig is supposed to be available:

1. Enable `inari`'s `gmp` feature (e.g. `features = ["gmp"]` in
   `truck-evidence/Cargo.toml`) — but that file is not on this packet's
   `write_allow`, so the packet owner/another packet must make the change, and
   it pulls in `gmp-mpfr-sys`/`rug` native deps.
2. Allow an approved non-`inari` outward-rounded `sin`/`cos` path.
3. Adjust the dependency contract so a `gmp`-capable `inari` is guaranteed.

Note: this likely affects every sibling carrier packet that needs interval trig
(cone, cylinder, sphere, torus).
