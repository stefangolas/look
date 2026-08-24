# WORK PACKET BG-AUD-FIX-007 — B-spline surface section indexing (AUD-006)

You are repairing one defect found by the formal-kernel correctness audit
`loop/audits/BG-AUDIT-001.md` (finding AUD-006), in
`truck-geometry/src/nurbs/bspsurface.rs`. Everything you need is in this
document. **Do not read any other spec file** — this packet is self-contained.

```json
{"id":"BG-AUD-FIX-007","status":"DONE","contracts":["AUD-006"],
 "tests_added":2,"deviations":[],"disagreements":[],
 "baseline_failures":[],"notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. **If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-AUD-FIX-007
contract:    [AUD-006]
class:       mechanical
crates:      [truck-geometry]
write_allow:
  - vendor/truck/truck-geometry/src/nurbs/bspsurface.rs
read_allow:
  - vendor/truck/truck-geometry/src/nurbs/knot_vec.rs
tests_required:
  - sectional_curve_vcut_u_half_box_does_not_panic
  - sectional_curve_vcut_returns_half_v_section
budget:      {turns: 25, ctx_tokens: 70000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'fn sectional_curve' vendor/truck/truck-geometry/src/nurbs/bspsurface.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'fn vcut' vendor/truck/truck-geometry/src/nurbs/bspsurface.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'fn ucut' vendor/truck/truck-geometry/src/nurbs/bspsurface.rs"}
```

## Problem

`sectional_curve(bnd_box)` must return the section of the surface over the
parameter box `bnd_box`. The u-cut decisions correctly test the box's **u**
coordinate against the u-knots, but the v-cut decisions test `p[0]`/`q[0]` —
the **u** coordinate — against the **v**-knots (bspsurface.rs:1299 and :1303),
then call `vcut(p[1])`/`vcut(q[1])`. Two concrete failures on a `[0,1]×[0,1]`
surface:

- box `u ∈ [0,0.5], v ∈ [0,1]`: `q[0] = 0.5` is tested against the last v-knot
  `1.0`, the test fires, and `vcut(q[1] = 1.0)` cuts at the back v-knot,
  producing a degenerate surface that panics in `BSplineSurface::new` (unwrap).
- box `u ∈ [0,1], v ∈ [0,0.5]`: `q[0] = 1.0` equals the last u-knot AND the
  last v-knot, so the comparison does not fire and no v-cut happens — the
  section wrongly spans the full `v ∈ [0,1]`.

Both confirmed on this tree (subagent probe in the audit).

**Observe the regression fail first:** add the two tests below, watch
`sectional_curve_vcut_u_half_box_does_not_panic` PANIC and
`sectional_curve_vcut_returns_half_v_section` produce the wrong section on the
buggy code, then fix and watch them pass. Record the pre-fix observation in
`RESULT.json.notes`.

## Repair

In `sectional_curve`, change the two v-cut decision conditions to test the **v**
coordinate against the v-knots: line 1299 `ctx.is_small_ratio(p[0] -
bspsurface.vknot(0))` becomes `p[1] - bspsurface.vknot(0)`, and line 1303
`ctx.is_small_ratio(q[0] - bspsurface.vknot(...))` becomes `q[1] -
bspsurface.vknot(...)`. Nothing else changes — the `vcut(p[1])`/`vcut(q[1])`
calls already use the correct coordinate. The two `// BG-TOL-001: param`
markers stay attached to their lines.

## Regression tests (exact names)

Add to the crate's test module (a new `#[cfg(test)] mod section_tests` beside
the existing test modules is fine; `use super::*;`). Build one surface:

```rust
let uknots = KnotVec::from(vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
let vknots = KnotVec::from(vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
let ctrl_pts = (0..3)
    .map(|i| {
        (0..3)
            .map(|j| Point3::new(i as f64, j as f64, (i * j) as f64))
            .collect()
    })
    .collect();
let surface = BSplineSurface::new((uknots, vknots), ctrl_pts);
```

(a bilinear-degree-2 net on `[0,1]²`, so `S(1, 1)` differs clearly from
`S(1, 0.5)`).

1. `sectional_curve_vcut_u_half_box_does_not_panic`

   ```rust
   let bnd = BoundingBox::from_iter(&[Vector2::new(0.0, 0.0), Vector2::new(0.5, 1.0)]);
   let curve = surface.sectional_curve(bnd);
   assert_near2!(curve.subs(0.0), surface.subs(0.0, 0.0));
   assert_near2!(curve.subs(1.0), surface.subs(0.5, 1.0));
   ```

   Must not panic (the buggy code panics before returning).

2. `sectional_curve_vcut_returns_half_v_section`

   ```rust
   let bnd = BoundingBox::from_iter(&[Vector2::new(0.0, 0.0), Vector2::new(1.0, 0.5)]);
   let curve = surface.sectional_curve(bnd);
   assert_near2!(curve.subs(0.0), surface.subs(0.0, 0.0));
   assert_near2!(curve.subs(1.0), surface.subs(1.0, 0.5));
   ```

   On the buggy code the section spans the full v-range and `curve.subs(1.0)`
   equals `surface.subs(1.0, 1.0)` — the assertion fails.

`assert_near2!` and `BoundingBox` are available in the crate's test context
(the `sectional_curve` doctest uses both). If `Point3` is not in scope in your
test module, import it. H-1 note: use `#[allow(clippy::unwrap_used)]` only if
your test body needs it; the crate's existing test modules already set the
pattern.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N`-shaped literal unless that line ends with an `// H-3` comment. This
packet's literals are `0.0`/`0.5`/`1.0` (no match). Run `bash
scripts/kernel-gates.sh <your base commit>` yourself before writing
`RESULT.json`.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-geometry
cargo clippy -p truck-geometry --all-targets --no-deps
cargo test -p truck-geometry --lib --tests --no-fail-fast
cargo check --workspace --all-targets
bash scripts/kernel-gates.sh <your base commit>
```

Never run a bare `cargo test`.

## Forbidden

Editing any file outside `write_allow`. Changing the u-cut logic. Changing the
`vcut`/`ucut` methods themselves (they are correct; only the decision
condition indexes the wrong coordinate). Adding `#[ignore]`. Weakening an
existing negative test.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- the `BoundingBox`/`assert_near2!`/`Vector2` usage does not compile in the
  crate's test context → `SPEC_GAP`, with the exact mismatch and a corrected
  construction
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`fix(geometry): v-cut decisions test the v coordinate in sectional_curve (BG-AUD-FIX-007)`.
