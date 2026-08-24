# WORK PACKET BG-AUD-FIX-001 — sphere evidence soundness (AUD-001, AUD-016)

You are repairing two defects found by the formal-kernel correctness audit
`loop/audits/BG-AUDIT-001.md` (findings AUD-001 and AUD-016), both in
`truck-evidence/src/sphere.rs`. Everything you need is in this document. **Do
not read any other spec file** — this packet is self-contained.

```json
{"id":"BG-AUD-FIX-001","status":"DONE","contracts":["AUD-001","AUD-016"],
 "tests_added":3,"deviations":[],"disagreements":[],
 "baseline_failures":[],"notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. **If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-AUD-FIX-001
contract:    [AUD-001, AUD-016]
class:       design
crates:      [truck-evidence]
write_allow:
  - vendor/truck/truck-evidence/src/sphere.rs
read_allow:
  - vendor/truck/truck-evidence/src/enclosure.rs
  - vendor/truck/truck-evidence/src/elementary.rs
tests_required:
  - sphere_normal_cone_wide_azimuth_contains_all_normals
  - sphere_normal_cone_azimuth_below_pi_stays_tight
  - sphere_immersion_lower_bound_is_directed
budget:      {turns: 30, ctx_tokens: 80000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'fn normal_cone' vendor/truck/truck-evidence/src/sphere.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'fn immersion_lower_bound' vendor/truck/truck-evidence/src/sphere.rs"}
  - {id: A3, expect: 2, cmd: "grep -c 'CONVEX_HALF_ANGLE' vendor/truck/truck-evidence/src/sphere.rs"}
```

## Problem

### AUD-001 — `normal_cone` under-encloses normals

`EnclosureSurface::normal_cone` for `Sphere` returns a cone that must contain
every unit normal over the parameter box `(uu, vv)`. The current corner rule
treats the patch as the geodesic hull of its four corners and returns a tight
cone whenever the corner half-angle is `< π/2`. That justification is false for
this parameterization: the u-edges map to parallels (small circles), not
geodesics, so when the azimuth span `wv = v1 − v0` approaches/exceeds π the
interior of a parallel bulges arbitrarily far from the corner-average axis.

**The exact audit witness** (machine-checked on this tree): unit sphere at the
origin, `uu = [0.5, 0.6]`, `vv = [0, 3.6]`. The current code returns a tight
cone with corner half-angle ≈ `33.37°`, but the interior normal
`n(0.6, 1.80)` deviates `42.31°` from the returned axis — 79% of sampled
normals escape the cone. That is an under-estimation, which this crate's own
rules forbid (BG-ENC-001): a consumer certifies a too-large minimal
inter-surface angle.

**Decided repair:** emit the everything-cone whenever the azimuth span
`vv.sup() − vv.inf()` is `>= π`. For `wv < π` the corner-hull argument is
sound (verified numerically: no interior escape over a wide sweep of patches),
so the existing tight-cone path stays unchanged. The check goes at the top of
`normal_cone`, before the corner computation. The everything-cone
(`axis = +z`, `half_angle = π`) already exists as the module's helper.

Do NOT "fix" this by sampling to tighten the cone. The conservative
everything-cone (or a proven geodesic-diameter bound) is the only sound option;
an everything-cone/refusal is acceptable where a tight proof is unavailable.

### AUD-016 — `immersion_lower_bound` is not directed

`immersion_lower_bound` must return a certified LOWER bound on
`‖S_u × S_v‖ = r²·sin(u)`; a lower bound must round DOWN (the crate's
BG-ENC-003 outward-rounding rule). The current body:

```rust
let su = sin(uu).inf().max(0.0);
(self.radius() * self.radius() * su).max(0.0)
```

`sin(uu)` is already a certified interval and `.inf()` rounds down — but the
final product `r*r*su` is computed in round-to-nearest f64, which can round UP
by an ulp, making the "lower bound" exceed the true minimum. This is C1
hardening (the audit did not demonstrate a bite), but the direction is the one
the crate's own rules forbid.

**Decided repair:** compute the product in interval arithmetic so the returned
value is the downward-rounded interval product:

```rust
fn immersion_lower_bound(&self, uu: Interval, _vv: Interval) -> f64 {
    let r = self.radius();
    if !r.is_finite() || r <= 0.0 {
        return 0.0;
    }
    let su = sin(uu);
    (interval_at(r) * interval_at(r) * su).inf().max(0.0)
}
```

This preserves the two existing guarantees: a degenerate (NaN/zero/negative)
radius still returns the honest `0.0` (never `+inf` — do NOT rely on
`Interval::EMPTY.inf()` being a valid answer, it is `+inf` and would be an
unsound "lower bound"), and a pole-touching box returns `0.0` via `.max(0.0)`.

## Regression tests (exact names)

All in the existing `mod tests` of `sphere.rs`. The module's test-only allow
shape (`#![allow(clippy::unwrap_used, clippy::expect_used)]` then
`#![deny(clippy::unwrap_used)]`) stays; prefer `match` over `unwrap` in test
bodies. `const_interval!` is already imported in the test module.

1. `sphere_normal_cone_wide_azimuth_contains_all_normals`

   The AUD-001 witness: `uu = const_interval!(0.5, 0.6)`,
   `vv = const_interval!(0.0, 3.6)` on the unit sphere. Take the returned cone
   (it may be `Some`; it will be the everything-cone) and assert that EVERY
   sample of a 61×61 grid over the box satisfies
   `normal(u,v).dot(cone.axis).clamp(-1.0,1.0).acos() <= cone.half_angle + 1e-12`
   (same-line `// H-3` on the 1e-12 literal). Also assert the returned
   `half_angle` equals `core::f64::consts::PI` (the decided repair emits the
   everything-cone for this box) — if you implement the geodesic-diameter
   alternative instead, assert containment only and explain in `notes`.

2. `sphere_normal_cone_azimuth_below_pi_stays_tight`

   `uu = const_interval!(0.5, 0.6)`, `vv = const_interval!(0.0, 3.0)` — the
   same polar band but `wv = 3.0 < π`. Assert the returned cone is TIGHT
   (`half_angle < π/2`) AND every sample of a 61×61 grid lies inside it. This
   pins the threshold: the fix must not collapse all cones to the
   everything-cone.

3. `sphere_immersion_lower_bound_is_directed`

   Radius `1.3`, `uu = const_interval!(0.3, 0.4)`, `vv = const_interval!(0.0,
   1.0)`. Assert the returned bound equals the downward-rounded interval
   product computed inline:
   ```rust
   let expected = (const_interval!(1.3, 1.3)
       * const_interval!(1.3, 1.3)
       * sin(const_interval!(0.3, 0.4)))
       .inf()
       .max(0.0);
   assert_eq!(s.immersion_lower_bound(uu, vv), expected);
   ```
   This deterministically fails on the old round-to-nearest product (measured:
   old returns `0.4994291492576638`, the directed product is
   `0.4994291492576637`).

The pre-existing tests `sphere_normal_cone_over_patch` (its wide-patch case
uses `vv = [0, π]`, which the `>= π` threshold keeps as the everything-cone)
and `sphere_immersion_lower_bound_and_poles` must stay green unchanged.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N`-shaped literal unless that line ends with an `// H-3` comment. The
existing tests already spell the sampling slack as `1e-12 // H-3`; copy that
form for any small float you add. Run `bash scripts/kernel-gates.sh <your base
commit>` yourself before writing `RESULT.json`.

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

Editing any file outside `write_allow`. Tightening the cone by sampling (the
everything-cone is the sound answer for `wv >= π`; a tighter-but-unsound cone
is the defect). Returning `+inf` from `immersion_lower_bound` for a degenerate
radius. Adding `#[ignore]`, or weakening/deleting a negative test.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- the interval product `interval_at(r) * interval_at(r) * su` does not compile
  against the real `inari` API (e.g. `sin` here is the crate's `elementary::sin`
  re-export — `crate::elementary::{cos, sin}` is already imported at the top of
  `sphere.rs`) → `SPEC_GAP`, with the exact mismatch
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`fix(evidence): sound sphere normal-cone for wide azimuth + directed immersion bound (BG-AUD-FIX-001)`.
