# WORK PACKET BG-AUD-FIX-005 — wedge slope lower-bound arithmetic (AUD-007)

You are repairing one defect found by the formal-kernel correctness audit
`loop/audits/BG-AUDIT-001.md` (finding AUD-007), in
`truck-evidence/src/fid/lfs.rs`. Everything you need is in this document. **Do
not read any other spec file** — this packet is self-contained.

```json
{"id":"BG-AUD-FIX-005","status":"DONE","contracts":["AUD-007"],
 "tests_added":1,"deviations":[],"disagreements":[],
 "baseline_failures":[],"notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. **If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-AUD-FIX-005
contract:    [AUD-007]
class:       mechanical
crates:      [truck-evidence]
write_allow:
  - vendor/truck/truck-evidence/src/fid/lfs.rs
read_allow:
  - vendor/truck/truck-evidence/src/elementary.rs
tests_required:
  - wedge_slope_lower_bound_is_conservative_at_small_margins
budget:      {turns: 25, ctx_tokens: 70000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'fn wedge_slope_lower_from_sin_margin' vendor/truck/truck-evidence/src/fid/lfs.rs"}
  - {id: A2, expect: 4, cmd: "grep -c 'WedgeSlopeLowerBound' vendor/truck/truck-evidence/src/fid/lfs.rs"}
```

## Problem

`wedge_slope_lower_from_sin_margin(s)` must return a certified **lower bound**
on `d(0, conv{n_A, n_B}) = cos(phi/2)` given `sin phi >= s`; a lower bound must
be conservative (may refuse what the truth admits, never admit what the truth
refuses). The current formula

```rust
let value = ((1.0 - (1.0 - sin_margin * sin_margin).sqrt()) / 2.0).sqrt();
```

cancels catastrophically: in `1 − sqrt(1 − s²)` the subtraction of two nearby
quantities turns an absolute rounding error ~`eps` into a relative error
~`eps·s⁻²`, and round-to-nearest can land ABOVE the true worst case
`sin(asin(s)/2)`. Measured on this tree:

| s        | computed       | true `sin(asin(s)/2)` | relative over-report |
|----------|----------------|-----------------------|----------------------|
| 1e-6     | 5.0002222465e-7 | 5.0000000000e-7      | +4.4e-5              |
| 1e-8     | 7.45e-9         | 5.00e-9              | +49%                 |
| 1e-9     | 0.0             | 5.0e-10              | −100% (collapse, safe direction) |

The over-report at `s = 1e-8` is +49%: a "lower bound" larger than the true
infimum, in the unsound direction for a gate bound.

## Repair

**Amendment (2026-08-24, orchestrator, after the first dispatch returned a
SPEC_GAP):** the pinned `inari = { version = "2.0", default-features = false }`
does NOT expose `Interval::asin` — it is compiled only under the `gmp`
feature, which is off in this tree (and `gmp` pulls `gmp-mpfr-sys`/`rug`,
which need `make`/`m4` and do not support this toolchain). The decided repair
is therefore the **series with directed rounding**, hybridised with the closed
form for the non-cancelling regime:

- `sin(asin(s)/2) = s/2 + s³/16 + 7s⁵/256 + …` — every coefficient positive
  for `s > 0`, so any partial sum is a certified LOWER bound. At the audit's
  witnesses (`1e-6`, `1e-8`) the 3-term series matches the true value to the
  last bit (measured).
- For `s < 1e-6` (the catastrophic-cancellation regime): evaluate the 3-term
  series in interval arithmetic and take `.inf()`. Each interval op rounds
  outward, so `.inf()` is a certified downward lower bound.
- For `s >= 1e-6`: the closed form is accurate to ulps (measured relative
  error `6.8e-9` at `1e-4`, `3e-11` at `1e-3`, decaying further), so evaluate
  the closed form in interval arithmetic and take `.inf()` — sound, and within
  `~2e-16` of the round-to-nearest closed form the existing tests pin.

Concretely, replacing the body of `wedge_slope_lower_from_sin_margin` (keep
the `InvalidMargin` guard and the return shape unchanged):

```rust
let tt = Interval::try_from((sin_margin, sin_margin)).unwrap_or(Interval::EMPTY);
let one = Interval::try_from((1.0, 1.0)).unwrap_or(Interval::EMPTY);
let value = if sin_margin < 1.0e-6 {
    // s/2 + s³/16 + 7s⁵/256, all terms positive (certified downward).
    let s2 = tt * tt;
    let s3 = tt * s2;
    let s5 = s3 * s2;
    (tt / 2.0 + s3 / 16.0 + s5 * 7.0 / 256.0).inf()
} else {
    let inner = (one - (one - tt * tt).sqrt()) / 2.0;
    inner.sqrt().inf()
};
```

`1.0e-6` and `2.0`/`16.0`/`256.0`/`7.0` are fine (the `1e-6` needs the
same-line `// H-3` comment if it matches the gate pattern — check with
`kernel-gates.sh`). The module has `#![deny(clippy::unwrap_used)]` at the top
(lfs.rs:27); do NOT call `unwrap()` — the `unwrap_or(Interval::EMPTY)` form is
mandatory. `Interval / f64` and `Interval * f64` compile (inari scales by a
float).

The existing tests stay green: `wedge_slope_monotone_and_knife_limit` samples
`[0.1, 1.0]` (all in the closed-form branch → strictly increasing exactly as
before); `wedge_formula_matches_geometry` at `s ∈ {0.5, 1.0}` uses the
closed-form branch whose `.inf()` is within `~2e-16` of the round-to-nearest
closed form (the test's `1e-15` slack absorbs it, and the `cos(phi/2) + 1e-9`
geometric claim holds with huge margin); the tiny-margin check at `s = 1e-4`
uses the closed-form branch and stays `~5e-5 < 1e-3`.

## Regression test (exact name)

`wedge_slope_lower_bound_is_conservative_at_small_margins` — in the existing
`mod tests` of `lfs.rs`. For `s in [1e-6, 1e-8]` assert the certificate
direction with an independent upward-slack reference computed WITHOUT inari's
`asin` (which is gmp-gated and unavailable): the true value is
`sin(asin(s)/2) = s/2 + s³/16 + …`, and `next_up(0.5*s + s³/16)` (f64,
round-to-nearest then one ulp up) is an UPPER bound on the true value at these
scales (measured: `ref >= true`). Assert:

```rust
let ref_hi = (0.5 * s + s * s * s / 16.0).next_up();
assert!(bound.value <= ref_hi, "s={s}: bound {} exceeds the true sup {ref_hi}", bound.value);
assert!(bound.value > 0.0, "s={s}: bound collapsed to zero");
```

On the buggy code `s = 1e-8` gives `7.45e-9 > 5.000000000000001e-9` (fails) and
`s = 1e-6` gives `5.000222e-7 > 5.000000000000001e-7` (fails); the series
branch passes both with `bound > 0`. `next_up` is stable on this toolchain
(rustc 1.97). The `1e-6`/`1e-8` literals need the same-line `// H-3` comment.
The existing tests `wedge_slope_monotone_and_knife_limit` and
`wedge_formula_matches_geometry` must stay green unchanged.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N`-shaped literal unless that line ends with an `// H-3` comment. Every
`1e-6`/`1e-8` in the test MUST carry the same-line `// H-3` comment (the file's
existing constants already do — copy that form). Run `bash
scripts/kernel-gates.sh <your base commit>` yourself before writing
`RESULT.json`.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-evidence
cargo clippy -p truck-evidence --all-targets --no-deps
cargo test -p truck-evidence --lib --tests --no-fail-fast
cargo check --workspace --all-targets
bash scripts/kernel-gates.sh <your base commit>
```

Never run a bare `cargo test`.

## Forbidden

Editing any file outside `write_allow`. Changing the `InvalidMargin` guard or
the return type/shape. Widening a tolerance or adding slack. Adding `#[ignore]`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- the hybrid threshold (`1.0e-6`) or the interval arithmetic does not compile
  against the pinned `inari` → `SPEC_GAP`, with the exact mismatch
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`fix(evidence): directed-rounding wedge-slope lower bound (BG-AUD-FIX-005)`.
