# WORK PACKET BG-ENC-002-CIRCLE — enclosure for the `UnitCircle<Point3>` carrier

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-ENC-002-CIRCLE","status":"DONE","contracts":["BG-ENC-001","BG-ENC-002"],
 "tests_added":5,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

```yaml
id:          BG-ENC-002-CIRCLE
contract:    [BG-ENC-001, BG-ENC-002]
class:       mechanical
crates:      [truck-evidence]
depends_on:  [BG-CE-006-ENUM]
write_allow:
  - vendor/truck/truck-evidence/src/circle.rs
read_allow:
  - vendor/truck/truck-evidence/src/lib.rs
  - vendor/truck/truck-evidence/src/elementary.rs
  - vendor/truck/truck-evidence/src/enclosure.rs
  - vendor/truck/truck-evidence/src/harness.rs
  - vendor/truck/truck-evidence/src/plane.rs
  - vendor/truck/truck-geometry/src/specifieds/circle.rs
  - vendor/truck/truck-geometry/src/specifieds/mod.rs
tests_required:
  - circle_encloses_sampled_points
  - circle_trig_extrema_inside_interval
  - circle_enclosure_converges_under_bisection
  - circle_tangent_cone_over_arc_and_full_circle
  - circle_der_enclosures_cycle_mod_four
budget:      {turns: 30, ctx_tokens: 70000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'impl ParametricCurve for UnitCircle<Point3>' vendor/truck/truck-geometry/src/specifieds/circle.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub struct UnitCircle' vendor/truck/truck-geometry/src/specifieds/mod.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub mod plane' vendor/truck/truck-evidence/src/lib.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'impl EnclosureSurface for Plane' vendor/truck/truck-evidence/src/plane.rs"}
  - {id: A5, expect: 4, cmd: "grep -c 'pub fn assert_' vendor/truck/truck-evidence/src/harness.rs"}
```

## Problem

The first `EnclosureCurve` after the reference set: the unit circle. This is
the purest instance of the interval-trigonometry obligation — the whole
carrier is two trig functions. The carrier (read it off
`specifieds/circle.rs`, confirmed at packet time) is the phantom-typed unit
circle; for `UnitCircle<Point3>`,

    C(t) = (cos t, sin t, 0),   t ∈ [0, 2π) periodic,   tangent (−sin t, cos t, 0),

with derivatives cycling mod 4 (`der_n`: cos/sin, −sin/cos, −cos/−sin,
sin/−cos).

**Scope note:** this packet encloses the *unit* carrier only. Placed circles
(`Processor<…, Matrix4>`) and trimmed domains are compositional
(`BG-ENC-004-PROCESSOR` maps boxes through the matrix; trimming intersects
domains) and are deliberately not yours.

**Where the interval trig comes from.** `inari::Interval` has **no** `sin`/`cos`
in this tree: they live in `inari`'s own `elementary` module behind its `gmp`
feature, and `truck-evidence` takes `inari` with `default-features = false`.
Use the crate's own certified pair instead —

    use crate::elementary::{cos, sin};

free functions from `inari::Interval` to `inari::Interval`, already
outward-rounded and already accounting for the interior extrema at `kπ/2`.
Write `cos(uu)`, never `uu.cos()`; the method does not exist and a design that
needs it is a design that stops. **Never evaluate a trig function only at the
interval endpoints** — an interval spanning an interior extremum (e.g.
`[0.4π, 0.6π]` for `cos`) must contain the extremal value, and endpoint
evaluation is the historic under-estimation bug this item exists to prevent.

## Decisions already made for you

1. **One existing file**, `vendor/truck/truck-evidence/src/circle.rs`. It is
   already created and already declared as `pub mod circle;` in `lib.rs`, and
   `lib.rs` is **read-only for you** — it is not on your `write_allow` and
   editing it is a scope violation. The declaration was made up front so the
   six sibling carrier packets have disjoint write sets and can run in
   parallel; the file currently holds only a scaffolding doc comment, which
   you replace. Crate-level `#![deny(...)]` in `lib.rs` covers it. Follow
   `plane.rs` for structure and tone.

2. **`enclose`**: `x = cos(tt)`, `y = sin(tt)`, `z` the degenerate
   `interval_at(0.0)`.

3. **`enclose_der(n, tt)`**: by `n % 4` exactly as `der_n` does —
   `(cos, sin)`, `(−sin, cos)`, `(−cos, −sin)`, `(sin, −cos)` — each component
   via the corresponding `inari` trig call on `tt`, `z` degenerate. This is
   exact-form interval evaluation, not numerical differentiation.

4. **`tangent_cone`** over `tt` — same arc rule as a cylinder's normal cone,
   rotated a quarter turn, and the worker-visible statement of it:
   - `w = tt.sup() − tt.inf()`;
   - `w ≤ π` → `{ axis: tangent at the midpoint angle, half_angle: w / 2 }`;
   - `w > π` → `{ axis: (0, 0, 1), half_angle: π / 2 }` — every circle tangent
     is horizontal, so this contains all of them; sound, not tight.
   Name the threshold `const` (H-3).

5. **No changes to `enclosure.rs`, `harness.rs`, `plane.rs`, or another
   carrier's file.**

## Tests required

1. `circle_encloses_sampled_points` — `assert_encloses_curve` over: a short
   arc, an arc straddling `π/2` and one straddling `π`, an arc longer than
   `π`, a full `2π` sweep. ≥ 40 samples.
2. `circle_trig_extrema_inside_interval` — `tt = [0.4π, 0.6π]`: the `y`
   enclosure must contain `sin(π/2) = 1`; assert `box.y.sup() >= 1.0 − 1e-15`
   and that the endpoint-evaluation max (`sin(0.6π)`) is strictly below it —
   the relation is the point, not bit-equality.
3. `circle_enclosure_converges_under_bisection` — bisection shrinks widths
   monotonically to below the initial width over ~20 halvings of `[0, 0.3π]`
   (a curve: compare `enclose(tt).width()`; the shared `assert_converges` is
   surface-only, so write the four-line curve version locally, in the test
   module).
4. `circle_tangent_cone_over_arc_and_full_circle` — short arc: axis ≈ tangent
   at the midpoint, half_angle ≈ w/2; full sweep: axis `z`, half_angle `π/2`,
   and sampled tangents are inside by angle (`cos(angle) ≥ cos(half_angle)`
   with float tolerance).
5. `circle_der_enclosures_cycle_mod_four` — `enclose_der(0, tt)` matches
   `enclose`, `(1)` contains sampled `(−sin t, cos t, 0)`, `(4)` matches
   `(0)` again on a box where the widths are comparable, `(5)` matches `(1)`.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-evidence
cargo clippy -p truck-evidence --all-targets --no-deps -- -D warnings
cargo test -p truck-evidence --lib --tests --no-fail-fast
cargo check --workspace --all-targets
```

Never run a bare `cargo test`. Send cargo output to a file and read the tail.
The plane tests must keep passing unchanged.

## Forbidden

Editing any file outside `write_allow`. Changing the trait, the harness, or
another carrier's file. Endpoint-only trig evaluation. Enclosing placed or
trimmed circles (out of scope). Adding `#[ignore]`. Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- `inari` lacks a primitive this design needs → `SPEC_GAP`, naming it
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(evidence): EnclosureCurve for UnitCircle (BG-ENC-002-CIRCLE)`.
