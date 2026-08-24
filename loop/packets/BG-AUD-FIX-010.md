# WORK PACKET BG-AUD-FIX-010 — analytic carrier inverse/predicate cleanup (AUD-010, AUD-013, AUD-017)

You are repairing three defects found by the formal-kernel correctness audit
`loop/audits/BG-AUDIT-001.md` (findings AUD-010, AUD-013, AUD-017), all in
`truck-geometry/src/specifieds/`. Everything you need is in this document.
**Do not read any other spec file** — this packet is self-contained.

```json
{"id":"BG-AUD-FIX-010","status":"DONE","contracts":["AUD-010","AUD-013","AUD-017"],
 "tests_added":4,"deviations":[],"disagreements":[],
 "baseline_failures":[],"notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. **If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-AUD-FIX-010
contract:    [AUD-010, AUD-013, AUD-017]
class:       mechanical
crates:      [truck-geometry]
write_allow:
  - vendor/truck/truck-geometry/src/specifieds/cone.rs
  - vendor/truck/truck-geometry/src/specifieds/sphere.rs
  - vendor/truck/truck-geometry/src/specifieds/hyperbola.rs
  - vendor/truck/truck-geometry/src/specifieds/parabola.rs
read_allow:
  - vendor/truck/truck-base/src/tolerance.rs
tests_required:
  - cone_include_holds_pointwise_on_both_nappes
  - cone_nearest_parameter_near_side_for_lower_nappe_query
  - sphere_search_nearest_parameter_center_is_none
  - conic_containment_scale_invariant
budget:      {turns: 40, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 3, cmd: "grep -c 'fn include' vendor/truck/truck-geometry/src/specifieds/cone.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'fn search_parameter' vendor/truck/truck-geometry/src/specifieds/cone.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'fn search_nearest_parameter' vendor/truck/truck-geometry/src/specifieds/cone.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'fn search_nearest_parameter' vendor/truck/truck-geometry/src/specifieds/sphere.rs"}
  - {id: A5, expect: 3, cmd: "grep -c 'is_small_ratio' vendor/truck/truck-geometry/src/specifieds/hyperbola.rs"}
  - {id: A6, expect: 3, cmd: "grep -c 'is_small_ratio' vendor/truck/truck-geometry/src/specifieds/parabola.rs"}
```

## Problem

### AUD-010 — cone lower nappe inconsistent with the declared unbounded v domain

The cone's `parameter_range` declares `v` unbounded (cone.rs:121-124) and the
build spec pins the DOUBLE cone ("the carrier's `v` is unbounded both ways").
`subs(u, v < 0)` generates the lower nappe, but the containment/inverse
predicates only hold for `v ≥ 0`:

- `include` tests `radial ≈ r.z·tan` (cone.rs:155); on the lower nappe
  `radial = |v|·slope` while `r.z = v < 0`, so the test fails for points the
  surface itself generates;
- `search_parameter` has the same predicate (cone.rs:248);
- `search_nearest_parameter` (cone.rs:296) minimizes
  `(r_q − v·slope)² + (z_q − v)²` — the SINGLE-nappe graph `radial = v·slope`.
  On the double cone the graph is `radial = |v|·slope`, so the single formula
  returns the wrong nappe's stationary point. Witness:
  `search_nearest_parameter(Point3(0.5, 0, -3))` returns the far side instead
  of the near side.

The decided semantic is the DOUBLE cone (it is what `subs` generates, what
`normal` already handles with its `v > 0` / `v < 0` sign flip, and what the
spec pins). Make `include`/`search_parameter`/`search_nearest_parameter`
correct for `v < 0`.

### AUD-013 — sphere `search_nearest_parameter(center)` returns `Some((NaN, NaN))`

`Sphere::search_nearest_parameter` (sphere.rs:238-257) normalizes
`point − center` with no zero guard; at the sphere's own center the vector is
zero and the returned `(u, v)` is `(NaN, NaN)` wrapped in `Some`. A query at
the center has no nearest parameter → `None`.

### AUD-017 — hyperbola/parabola containment compares a length against a dimensionless ratio

`UnitHyperbola::search_parameter` (hyperbola.rs:120, :133) and
`UnitParabola::search_parameter` (parabola.rs:124, and the `Point3` sibling)
compare `(p − subs(t)).magnitude()` — a model-space LENGTH — with
`is_small_ratio` (dimensionless). Under a real `model_scale != 1` the
predicate loosens/tightens wrongly. A length must use `is_small_len`
(scaled by model scale).

**Your first obligation — observe the regressions fail on the buggy code:** add
the four tests below. `cone_include_holds_pointwise_on_both_nappes` and
`cone_nearest_parameter_near_side_for_lower_nappe_query` must FAIL on the
current code; `sphere_search_nearest_parameter_center_is_none` must fail (it
returns `Some((NaN, NaN))`); `conic_containment_scale_invariant` must fail at
non-unit scale. Record the pre-fix observations in `RESULT.json.notes`.

## Repair

### cone.rs

- **`include`** (the `BSplineCurve<Point3>` impl, line 155): change the
  predicate to the double-nappe radial relation `radial ≈ |r.z|·slope`:
  `ctx.is_small_len(radial − r.z.abs() * self.half_angle().tan())`. This holds
  pointwise on both nappes including the apex (`radial = r.z = 0`).
- **`search_parameter`** (line 248): the same absolute-value predicate
  `radial − r.z.abs() * tan`.
- **`search_nearest_parameter`**: derive the sign-aware double-nappe projection.
  In the (radial, z) plane the cone is `radial = |v|·slope`, so the squared
  distance is `(r_q − |v|·s)² + (z_q − v)²`. The two one-sided stationary
  candidates are `v₊ = (s·r_q + z_q)/(1 + s²)` (valid when `v₊ ≥ 0`) and
  `v₋ = (z_q − s·r_q)/(1 + s²)` (valid when `v₋ ≤ 0`); pick the candidate with
  the smaller squared distance. Keep the azimuth `u` from the query's horizontal
  direction exactly as today (the cone is z-symmetric, so `u` is the same on
  either nappe). The witness `(0.5, 0, −3)` must now return the near-side
  parameter; `search_nearest_parameter(subs(0.7, −3.0))` must return `(0.7,
  −3.0)`-class parameters (distance 0), not the apex.

### sphere.rs

- **`search_nearest_parameter`**: guard the length before normalizing. When
  `point` is within the crate's tolerance of the center, return `None`. Use
  the same context pattern as `search_parameter`:
  `let ctx = ToleranceCtx::unscaled_legacy();` then
  `if ctx.is_small_len((point − self.center).magnitude()) { return None; }`.
  For a valid query, normalize and compute `(u, v)` exactly as today.

### hyperbola.rs / parabola.rs

- Replace `ctx.is_small_ratio((p − self.subs(t)).magnitude())` with
  `ctx.is_small_len((p − self.subs(t)).magnitude())` in every
  `search_parameter` impl (hyperbola `Point2`/`Point3`, parabola
  `Point2`/`Point3`). The `// BG-TOL-001: param` markers stay; if the marker's
  meaning changes (it now reads `model`), update the marker to `// BG-TOL-001:
  model` where the migration class actually is.

## Regression tests (exact names)

1. `cone_include_holds_pointwise_on_both_nappes` — for a `Cone` with
   half-angle `π/4`, build a pointwise constant-curve include of points on BOTH
   nappes: `cone.include(&BSplineCurve::new(KnotVec::bezier_knot(0),
   vec![cone.subs(0.7, 3.0)]))` and the `v = −3.0` version must both return
   `Ok(Certified { value: true, .. })`, and the apex `cone.subs(0.0, 0.0)`
   must include as `true`. (The `include` impl only handles constant curves —
   this is the contract.)

2. `cone_nearest_parameter_near_side_for_lower_nappe_query` — for the same
   cone, `search_nearest_parameter(Point3::new(0.5, 0.0, -3.0))` returns
   `Some((u, v))` whose surface point is the NEAR side: assert
   `(cone.subs(u, v) − Point3::new(0.5, 0.0, -3.0)).magnitude() < 1.0`
   (same-line `// H-3` on the `1.0`... a length threshold is not a bare `1e-N`,
   so no H-3 needed, but if you use a small float add the comment). Also assert
   `search_nearest_parameter(cone.subs(0.7, -3.0))` returns parameters whose
   `subs` is within the crate's tolerance of `cone.subs(0.7, -3.0)`.

3. `sphere_search_nearest_parameter_center_is_none` — a unit sphere at the
   origin: `search_nearest_parameter(Point3::origin())` is `None`. On the
   buggy code it returns `Some((NaN, NaN))`.

4. `conic_containment_scale_invariant` — for `UnitParabola<Point2>` (and, if
   convenient, `UnitHyperbola<Point2>`), for several `model_scale` values in
   `[0.5, 1.0, 2.0, 10.0]`, build the context with
   `ToleranceCtx::new(scale, ...)` and assert that a point ON the curve
   (`subs(t)` for a few `t`) is contained (`search_parameter` returns `Some`)
   while a point clearly OFF the curve by a scale-proportional offset is not.
   This pins that the predicate scales correctly. The `ToleranceCtx::new`
   arguments follow the `legacy_ctx()` pattern in the crate's other tests; a
   non-unit `model_scale` is the point of the test.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N`-shaped literal unless that line ends with an `// H-3` comment. The
values `0.5`/`1.0`/`2.0`/`10.0`/`3.0` do not match the pattern; any `1e-N`
does. Run `bash scripts/kernel-gates.sh <your base commit>` yourself before
writing `RESULT.json`.

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

Editing any file outside `write_allow`. Declaring the cone single-nappe (the
spec pins the double nappe). Removing the `v > 0`/`v < 0` sign flip in
`Cone::normal`. Adding `#[ignore]`. Deleting or weakening a negative test.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- the double-nappe nearest-parameter derivation does not match the real
  geometry on the audit witness → `SPEC_GAP`, with your derivation and the
  measured mismatch
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`fix(geometry): double-nappe cone predicates, sphere center guard, length-scaled conic containment (BG-AUD-FIX-010)`.
