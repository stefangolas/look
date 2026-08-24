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

Evaluate the same lower-bound quantity with directed arithmetic, rounded DOWN.
inari 2.0.0 provides `Interval::asin` (directed) and the crate's own
`elementary::sin` is a certified interval pair; the module already imports
`crate::enclosure::Interval` (which re-exports `inari::Interval`) and `sin` is
available as `crate::elementary::sin`. The decided form:

```rust
let tt = Interval::try_from((sin_margin, sin_margin)).unwrap_or(Interval::EMPTY);
let value = crate::elementary::sin(tt.asin() / 2.0).inf();
```

This is a certified downward lower bound on `sin(asin(s)/2)`: `asin` rounds
outward, the halving and `sin` stay outward, and `.inf()` rounds the result
down. It never exceeds the true infimum under the module's own arithmetic
semantics. Keep the `InvalidMargin` guard (`!(sin_margin > 0.0 && sin_margin
<= 1.0)`) exactly as it is, and keep the `scope`/return shape.

Note: `Interval::try_from` never fails for a finite in-range `sin_margin`;
`.unwrap_or(Interval::EMPTY)` is for H-1 totality only. The module has
`#![deny(clippy::unwrap_used)]` at the top (lfs.rs:27), so do NOT call
`unwrap()`; the `unwrap_or` form is mandatory.

## Regression test (exact name)

`wedge_slope_lower_bound_is_conservative_at_small_margins` — in the existing
`mod tests` of `lfs.rs`. For `s in [1e-6, 1e-8]` assert the certificate
direction: the returned `value` must never exceed the true quantity computed
with an independent upward-rounded evaluation,

```rust
let tt = Interval::try_from((s, s)).unwrap_or(Interval::EMPTY);
let true_sup = crate::elementary::sin(tt.asin() / 2.0).sup();
assert!(bound.value <= true_sup, "s={s}: bound {} exceeds the true sup {true_sup}", bound.value);
```

(`.sup()` is an upward-rounded over-approximation of the true value, so a
sound lower bound must satisfy `bound <= true_sup`; the old formula fails this
at `s = 1e-8`.) Also assert `bound.value > 0.0` for `s = 1e-8` (the old formula
collapses to `0.0` at `s = 1e-9`; the new form must not collapse at `1e-8`).
The `1e-6`/`1e-8` literals need the same-line `// H-3` comment (see below). The
existing tests `wedge_slope_monotone_and_knife_limit` and
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
- `Interval::asin` is not available on the pinned `inari` (check the Cargo.lock
  version) → `SPEC_GAP`, with the exact API difference and an alternative
  directed construction (e.g. the power series `s/2 + s³/16 + 5s⁵/512 + …`
  with each term rounded down, all terms positive)
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`fix(evidence): directed-rounding wedge-slope lower bound (BG-AUD-FIX-005)`.
