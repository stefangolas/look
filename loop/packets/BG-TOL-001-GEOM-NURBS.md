# WORK PACKET BG-TOL-001-GEOM-NURBS — Stage-A tolerance migration, truck-geometry/src/nurbs

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-TOL-001-GEOM-NURBS","status":"DONE","contracts":["BG-TOL-001"],
 "tests_added":2,"sites_migrated":57,"sites_deferred":12,"unscaled_legacy_calls":26,
 "anchors_verified":{"A1":0},
 "deviations":[
   {"code":"EXTRA_BINDING","sites":["file.rs:123"],
     "why":"one clause: what you did differently and why the packet's literal text did not work"}],
 "disagreements":[
   {"code":"CLASSIFICATION_WRONG","site":"file.rs:123",
     "claim":"one sentence: what the packet asserts and what you found instead"}],
 "baseline_failures":[
   {"test":"module::path::name","fails_at_base":true}],
 "notes":"free text for anything the fields above cannot carry"}
```

**Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty.** They are not a nicer layout for the same prose — they are the fields a
reviewer reads first, and they exist because a previous worker's single most
valuable finding arrived as the fifth paragraph of a 2,000-character `notes`
string and was nearly missed. Codes, so the vocabulary is closed:

- `deviations` — you did the work but not the way the packet literally said:
  `EXTRA_BINDING` (hoisted a subexpression into a `let`), `MARKER_PLACEMENT`
  (rustfmt moved a marker), `TEST_SHAPE` (a required test needed a different
  form). Each needs `sites` and a one-clause `why`.
- `disagreements` — the packet asserts something you found to be untrue:
  `BUDGET_WRONG`, `CLASSIFICATION_WRONG`, `ANCHOR_STALE`, `RULE_MISSING`,
  `SITE_UNREACHABLE`. **This is the highest-value field in the file.** Do not
  soften a disagreement into a note; a packet that is wrong and is obeyed
  silently costs far more than one that is contradicted. The last three shards
  each contained an orchestrator error, and two of them were found this way.
- `baseline_failures` — any pre-existing test that fails. Set `fails_at_base` by
  actually running it at the base commit, and say so if you could not.

`notes` stays free text on purpose: a genuinely novel observation has no code
yet, and inventing one to fit is worse than a sentence.

```yaml
id:          BG-TOL-001-GEOM-NURBS
contract:    [BG-TOL-001]
class:       wide-mechanical
crates:      [truck-geometry]
depends_on:  [BG-TOL-001-TYPE, BG-TOL-001-TYPE-r2, BG-TOL-001-TYPE-r3]
write_allow:
  - vendor/truck/truck-geometry/src/nurbs/bspcurve.rs
  - vendor/truck/truck-geometry/src/nurbs/bspsurface.rs
  - vendor/truck/truck-geometry/src/nurbs/knot_vec.rs
  - vendor/truck/truck-geometry/src/nurbs/mod.rs
  - vendor/truck/truck-geometry/src/nurbs/nurbscurve.rs
  - vendor/truck/truck-geometry/src/nurbs/nurbssurface.rs
  - vendor/truck/truck-geometry/tests/tolerance_nurbs.rs
read_allow:
  - vendor/truck/truck-base/src/tolerance.rs
tests_required:
  - every_migrated_nurbs_site_is_marked
  - deferred_generic_bound_sites_carry_a_fixme
budget:      {turns: 120, ctx_tokens: 240000}
census_fragment: nurbs
unscaled_legacy_budget: 26
anchors:
  - {id: A1, expect: 17, cmd: "grep -cE '\\.near\\(|so_small\\(|TOLERANCE' vendor/truck/truck-geometry/src/nurbs/bspcurve.rs"}
  - {id: A2, expect: 34, cmd: "grep -cE '\\.near\\(|so_small\\(|TOLERANCE' vendor/truck/truck-geometry/src/nurbs/bspsurface.rs"}
  - {id: A3, expect: 6, cmd: "grep -cE '\\.near\\(|so_small\\(|TOLERANCE' vendor/truck/truck-geometry/src/nurbs/knot_vec.rs"}
  - {id: A4, expect: 1, cmd: "grep -cE '\\.near\\(|so_small\\(|TOLERANCE' vendor/truck/truck-geometry/src/nurbs/mod.rs"}
  - {id: A5, expect: 7, cmd: "grep -cE '\\.near\\(|so_small\\(|TOLERANCE' vendor/truck/truck-geometry/src/nurbs/nurbscurve.rs"}
  - {id: A6, expect: 21, cmd: "grep -cE '\\.near\\(|so_small\\(|TOLERANCE' vendor/truck/truck-geometry/src/nurbs/nurbssurface.rs"}
```

## Problem

`truck-geometry/src/nurbs` is the B-spline and NURBS core: knot vectors,
`BSplineCurve`, `BSplineSurface`, and their rational counterparts. It holds more
tolerance predicates than any other module in the vendored tree, and they divide
almost perfectly along the line this contract item cares about. A knot value, a
curve parameter, a knot-span length, a `uv` hint — all dimensionless, all
scale-free. A control point, a curve position, a surface position — all model
lengths. The code compares both against the same absolute `TOLERANCE = 1e-6`
and records nothing about which is which.

That is reachable from untrusted geometry directly. A STEP file names its own
unit; the same spline imported in metres and in millimetres produces control
points a thousand times apart, and `TOLERANCE` does not move. Knot degeneracy
tests then behave identically (correctly — knots are parameters) while
control-point coincidence tests behave completely differently (incorrectly — those
are lengths). Today nothing in the source distinguishes the two cases, and the
judgement cannot be recovered by a machine later, because it depends on what the
quantity *means*.

**Stage A, which is all this packet is.** Each site is rewritten through a
`ToleranceCtx` obtained from `ToleranceCtx::unscaled_legacy()`, which carries
`model_scale = 1.0` and `tau_rep = TOLERANCE`. **No threshold moves and no
signature changes** — every rewrite in this packet is behaviour-preserving today
except for the deliberate Euclidean tightening in decision 1. A later Stage-B
packet derives a real `model_scale` at the entry points and threads it inward,
deleting the `unscaled_legacy()` calls. That is what actually fixes the scale
bug; this packet buys the judgement, which is the expensive half and the half
that cannot be recovered mechanically later.

## Anchors — verified 2026-08-19, counts are exact

Locate by running the `grep` command. **Never locate by line number** — the line
numbers in the tables below are provenance for a human reader, not a way to find
anything. `rg` is not installed on this machine; use `grep -cE` exactly as
written in the `anchors:` block above.

If any count differs from the `expect:` value, the tree has moved since this
packet was written. That is `ANCHOR_MISMATCH` and you stop — it is a stop
condition, not a nuisance, because a packet whose counts are stale is a packet
whose tables may point at the wrong code.

These counts cover **every** occurrence in each file, including doc comments and
in-src tests. Only the rows in the site table migrate. An anchor is a fingerprint
of the file, not a work list.

## The recipes — the only four rewrites you will make

| class | shape of the quantity | rewrite |
|---|---|---|
| `model` | a length, against zero | `ctx.is_small_len(l)` |
| `model` | two points that satisfy `MetricSpace<Metric = f64>` | `ctx.near_points(a, b)` |
| `param` | a dimensionless value against zero, or a difference | `ctx.is_small_ratio(x)` |
| `param` | a one-sided margin on a parameter | `ctx.ratio_margin()` |

`ctx.near_pt(a, b)` is the `Point3`-only form of `near_points` and either is
fine where both apply. The full surface of `ToleranceCtx` is `near_pt`,
`near_points`, `is_small_len`, `is_small_ratio`, `length_margin`, `sin_margin`,
`ratio_margin`, `entity_tau`, `model_scale`, `scaled`, `new`,
`unscaled_legacy` — there is nothing else on it, and in particular **there is no
squared-order and no area predicate**. If a site needs one, it is deferred, not
approximated.

Obtain the context once per function, as the first statement:

```rust
let ctx = ToleranceCtx::unscaled_legacy();
```

Mark every rewritten line with a trailing `// BG-TOL-001: model` or
`// BG-TOL-001: param` comment. Where the line is part of a multi-line
expression, put the marker on the line carrying the `ctx.` call.

**One context per function, never one per site and never one per block.**
26 functions in this packet hold a migrated site, so you should introduce
exactly **26** `unscaled_legacy()` calls. See "The ratchet" — this number
is enforced by a gate, and it is a budget rather than an allowance.

**If you cannot reach 26 honestly, say so and stop.** That instruction is
here because the previous shard could not: its packet demanded 11 contexts when
the truth was 10, and the worker built a shadow `let ctx = ...` inside a `match`
arm to satisfy the number. It was obeying a packet that was wrong, and the
orchestrator's counter had the same bug as the claim it was checking. A
`disagreements` entry with code `BUDGET_WRONG` is worth more here than a green
gate.

## Decisions already made for you

Read these before the tables. Each one is a judgement that has been made,
checked against the tree, and is not yours to revisit. Where a row in the site
table is marked **REVIEWED**, the same applies to that row.

1. **`.near()` is componentwise; `ctx.near_points` and `ctx.is_small_ratio` are
   Euclidean.** Not the same predicate — Euclidean is stricter by up to
   `sqrt(3)`. Every Stage-A shard is therefore a small deliberate tightening. If
   an existing test moves because of it, **report it in `baseline_failures` and
   in your notes with the test name and the reason**; do not widen a tolerance,
   do not add `#[ignore]`, and do not put a site back to componentwise to make it
   pass. A test that moves is a finding, not a bug in this packet.

2. **`so_small()` on a vector becomes `is_small_len(v.magnitude())`, not
   `is_small_len(v.x)`.** Same tightening as above, same rule about tests.

3. **A one-sided comparison keeps its shape.** `x - y > TOLERANCE` becomes
   `x - y > ctx.ratio_margin()` (or `ctx.length_margin()` for a `model` site),
   *not* a negated `is_small_ratio`. `is_small_ratio(d)` is `|d| <= margin` and
   is two-sided; substituting it for a one-sided guard changes behaviour on the
   negative side. The `write instead` column of the site table already has the
   correct form for every such row — use it.

4. **Twelve `model` sites are deferred for a generic bound, and this has been
   checked impl by impl — do not try to migrate them.** `ctx.near_points<P>` is
   declared `where P: MetricSpace<Metric = f64>`. Most of the `BSplineCurve` and
   `BSplineSurface` methods live in impls bounded `P: ControlPoint<f64> +
   Tolerance` (and one, `bspsurface.rs:652`, in `impl<V: Tolerance>`, weaker
   still). `ControlPoint` — `truck-base/src/cgmath_extend_traits.rs:9` — requires
   arithmetic, `Copy`, `Debug` and `Index`, and **not** `MetricSpace`. The NURBS
   wrappers are worse: their bound is `V: Homogeneous + ControlPoint<f64, Diff =
   V>` with `V::Point: Tolerance`, and `Homogeneous::Point` is only
   `EuclideanSpace<Scalar = Self::Scalar>`, which again does not give
   `MetricSpace`. Neither `ctx.near_points` nor `.distance()` is available in
   any of them. **Widening a public generic bound is cross-crate and is Stage B**,
   so those twelve get a `FIXME` and no rewrite. They are listed under
   "Not in this packet".

5. **`bspcurve.rs:1102` and `:1112` are NOT among them, and this is the one place
   two nearly identical sites diverge.** Their enclosing `impl<P>
   BSplineCurve<P>` at `bspcurve.rs:1058` bounds `P` with `MetricSpace<Metric =
   f64>` explicitly. They migrate normally with `ctx.near_points`. If you find
   yourself reasoning "`is_arc_of` is on `BSplineCurve<P>` so it must be blocked
   like `try_concat`", read the impl header — the bound, not the type, decides.

6. **The `include` implementations repeat, and repetition is not a reason to
   factor.** Six `IncludeCurve::include` impls across `bspsurface.rs` and
   `nurbssurface.rs` carry the identical five-line shape: one `near`/`near_points`
   on a surface point and four one-sided `TOLERANCE` margins on `uv` hints. Each
   is a separate function and gets its own context. **Do not extract a shared
   helper**; that is a signature change, it is outside this packet, and the six
   impls have different concrete point types.

7. **`mod.rs:186`, `if delta.abs() <= TOLERANCE`, is `param` and keeps its
   `.abs()` semantics for free.** `ctx.is_small_ratio(delta)` is already
   `|delta| <= ratio_margin()`, so write `ctx.is_small_ratio(delta)` and drop the
   explicit `.abs()`. This is the one row where the rewrite is shorter than the
   original and still identical.

## The sites — 57 migrate, 26 contexts

Line numbers are provenance for a human reader; locate by the enclosing symbol.

**`bspcurve.rs`**

| enclosing fn | line | code | class and why | write instead |
|---|---|---|---|---|
| `try_new` | 39 | `} else if knot_vec.range_length().so_small() {` | **`param`** — knot-vector range length in parameter space; zero-range detection must not scale with the model | `ctx.is_small_ratio(knot_vec.range_length())` |
| `sub_near_as_curve` | 235 | `if delta.so_small() {` | **`param`** — delta is a knot-interval width in parameter space, used only to skip empty spans | `ctx.is_small_ratio(delta)` |
| `try_remove_knot` | 589 | `if a.so_small() {` | **`param`** — a is the dimensionless knot-span ratio (knot[idx]-knot[i])/(knot[i+k+1]-knot[i]) used as a barycentric coordinate | `ctx.is_small_ratio(a)` |
| `syncro_knots` | 773 | `if self.knot(i) - other.knot(j) > TOLERANCE {` | **`param`** — one-sided comparison of normalized knot values (dimensionless) deciding which knot to insert | `self.knot(i) - other.knot(j) > ctx.ratio_margin()` |
| `syncro_knots` | 775 | `} else if other.knot(j) - self.knot(i) > TOLERANCE {` | **`param`** — one-sided comparison of normalized knot values (dimensionless) deciding which knot to insert | `other.knot(j) - self.knot(i) > ctx.ratio_margin()` |
| `cut` | 977 | `let s = if t.near(&self.knot_vec[idx]) {` | **`param`** — compares the cut parameter t with a knot value; both are dimensionless parameters | `ctx.is_small_ratio(t - self.knot_vec[idx])` |
| `is_arc_of` | 1102 | `if !self.subs(knots[0]).near(&curve.subs(hint)) {` | **`model`** — compares two curve sample points in model space to check the arc shares an endpoint **REVIEWED — orchestrator, session 9: MIGRATES. The session-8 handoff listed this among the generic-bound deferrals and that was wrong -- the enclosing `impl<P> BSplineCurve<P>` at bspcurve.rs:1058 bounds P with `MetricSpace<Metric = f64>` explicitly, so near_points applies.** | `if !ctx.near_points(self.subs(knots[0]), curve.subs(hint)) {` |
| `is_arc_of` | 1112 | `let flag = res.map(\|res\| hint <= res && curve.subs(res).near(&pt));` | **`model`** — compares a curve sample point to the reference point in model space; hint <= res is a plain parameter ordering, not a tolerance predicate **REVIEWED — orchestrator, session 9: MIGRATES. The session-8 handoff listed this among the generic-bound deferrals and that was wrong -- the enclosing `impl<P> BSplineCurve<P>` at bspcurve.rs:1058 bounds P with `MetricSpace<Metric = f64>` explicitly, so near_points applies.** | `let flag = res.map(\|res\| hint <= res && ctx.near_points(curve.subs(res), pt));` |

**`bspsurface.rs`**

| enclosing fn | line | code | class and why | write instead |
|---|---|---|---|---|
| `try_new` | 51 | `} else if knot_vecs.0.range_length().so_small() \|\| knot_vecs.1.range_length().so_small() {` | **`param`** — both are knot-vector range lengths in parameter space, so zero-range detection must not scale with the model | `ctx.is_small_ratio(knot_vecs.0.range_length()) \|\| ctx.is_small_ratio(knot_vecs.1.range_length())` |
| `sub_near_as_surface` | 523 | `if delta0.so_small() {` | **`param`** — delta0 is a u-knot-interval width in parameter space, used only to skip empty spans | `ctx.is_small_ratio(delta0)` |
| `sub_near_as_surface` | 530 | `if delta1.so_small() {` | **`param`** — delta1 is a v-knot-interval width in parameter space, used only to skip empty spans | `ctx.is_small_ratio(delta1)` |
| `try_remove_uknot` | 807 | `if a.so_small() {` | **`param`** — a is the dimensionless knot-span ratio used as a barycentric coordinate during uknot removal | `ctx.is_small_ratio(a)` |
| `try_remove_vknot` | 905 | `if a.so_small() {` | **`param`** — a is the dimensionless knot-span ratio used as a barycentric coordinate during vknot removal | `ctx.is_small_ratio(a)` |
| `syncro_uvknots` | 1083 | `if self.uknot(i) - self.vknot(j) > TOLERANCE {` | **`param`** — one-sided comparison of normalized u/v knot values (dimensionless) deciding which knot to insert | `self.uknot(i) - self.vknot(j) > ctx.ratio_margin()` |
| `syncro_uvknots` | 1085 | `} else if self.vknot(j) - self.uknot(i) > TOLERANCE {` | **`param`** — one-sided comparison of normalized u/v knot values (dimensionless) deciding which knot to insert | `self.vknot(j) - self.uknot(i) > ctx.ratio_margin()` |
| `ucut` | 1165 | `let s = if u.near(&self.uknot_vec()[idx]) {` | **`param`** — compares the cut parameter u with a u-knot value; both are dimensionless parameters | `ctx.is_small_ratio(u - self.uknot_vec()[idx])` |
| `sectional_curve` | 1271 | `if !p[0].near(&bspsurface.uknot(0)) {` | **`param`** — p[0] is the u-coordinate of the parameter-space sectioning box, compared against a u-knot value | `!ctx.is_small_ratio(p[0] - bspsurface.uknot(0))` |
| `sectional_curve` | 1274 | `if !q[0].near(&bspsurface.uknot(bspsurface.uknot_vec().len() - 1)) {` | **`param`** — q[0] is the u-coordinate of the parameter-space sectioning box, compared against a u-knot value | `!ctx.is_small_ratio(q[0] - bspsurface.uknot(bspsurface.uknot_vec().len() - 1))` |
| `sectional_curve` | 1277 | `if !p[0].near(&bspsurface.vknot(0)) {` | **`param`** — p[0] (u-coordinate) compared against a v-knot value; the quantity is a parameter either way, though this looks like a typo for p[1] | `!ctx.is_small_ratio(p[0] - bspsurface.vknot(0))` |
| `sectional_curve` | 1280 | `if !q[0].near(&bspsurface.vknot(bspsurface.vknot_vec().len() - 1)) {` | **`param`** — q[0] (u-coordinate) compared against a v-knot value; the quantity is a parameter either way, though this looks like a typo for q[1] | `!ctx.is_small_ratio(q[0] - bspsurface.vknot(bspsurface.vknot_vec().len() - 1))` |
| `include` | 1844 | `if !ParametricSurface::subs(self, hint.0, hint.1).near(&pt)` | **`model`** — compares the surface point at the resolved parameter to the curve point in model space; this is the deciding predicate of the include test | `!ctx.near_points(ParametricSurface::subs(self, hint.0, hint.1), pt)` |
| `include` | 1845 | `\|\| hint.0 < uknot_vec[0] - TOLERANCE` | **`param`** — one-sided check that the resolved u parameter stays above the u knot domain bottom | `\|\| hint.0 < uknot_vec[0] - ctx.ratio_margin()` |
| `include` | 1846 | `\|\| hint.0 - uknot_vec[0] > uknot_vec.range_length() + TOLERANCE` | **`param`** — one-sided check that the resolved u parameter does not overshoot the u knot domain top | `\|\| hint.0 - uknot_vec[0] > uknot_vec.range_length() + ctx.ratio_margin()` |
| `include` | 1847 | `\|\| hint.1 < vknot_vec[0] - TOLERANCE` | **`param`** — one-sided check that the resolved v parameter stays above the v knot domain bottom | `\|\| hint.1 < vknot_vec[0] - ctx.ratio_margin()` |
| `include` | 1848 | `\|\| hint.1 - vknot_vec[0] > vknot_vec.range_length() + TOLERANCE` | **`param`** — one-sided check that the resolved v parameter does not overshoot the v knot domain top | `\|\| hint.1 - vknot_vec[0] > vknot_vec.range_length() + ctx.ratio_margin()` |
| `include` | 1894 | `if !ParametricSurface::subs(self, hint.0, hint.1).near(&pt)` | **`model`** — compares the surface point at the resolved parameter to the curve point in model space; this is the deciding predicate of the include test | `!ctx.near_points(ParametricSurface::subs(self, hint.0, hint.1), pt)` |
| `include` | 1895 | `\|\| hint.0 < uknot_vec[0] - TOLERANCE` | **`param`** — one-sided check that the resolved u parameter stays above the u knot domain bottom | `\|\| hint.0 < uknot_vec[0] - ctx.ratio_margin()` |
| `include` | 1896 | `\|\| hint.0 - uknot_vec[0] > uknot_vec.range_length() + TOLERANCE` | **`param`** — one-sided check that the resolved u parameter does not overshoot the u knot domain top | `\|\| hint.0 - uknot_vec[0] > uknot_vec.range_length() + ctx.ratio_margin()` |
| `include` | 1897 | `\|\| hint.1 < vknot_vec[0] - TOLERANCE` | **`param`** — one-sided check that the resolved v parameter stays above the v knot domain bottom | `\|\| hint.1 < vknot_vec[0] - ctx.ratio_margin()` |
| `include` | 1898 | `\|\| hint.1 - vknot_vec[0] > vknot_vec.range_length() + TOLERANCE` | **`param`** — one-sided check that the resolved v parameter does not overshoot the v knot domain top | `\|\| hint.1 - vknot_vec[0] > vknot_vec.range_length() + ctx.ratio_margin()` |
| `include` | 1944 | `if !ParametricSurface::subs(self, hint.0, hint.1).near(&pt)` | **`model`** — compares the surface point at the resolved parameter to the curve point in model space; this is the deciding predicate of the include test | `!ctx.near_points(ParametricSurface::subs(self, hint.0, hint.1), pt)` |
| `include` | 1945 | `\|\| hint.0 < uknot_vec[0] - TOLERANCE` | **`param`** — one-sided check that the resolved u parameter stays above the u knot domain bottom | `\|\| hint.0 < uknot_vec[0] - ctx.ratio_margin()` |
| `include` | 1946 | `\|\| hint.0 - uknot_vec[0] > uknot_vec.range_length() + TOLERANCE` | **`param`** — one-sided check that the resolved u parameter does not overshoot the u knot domain top | `\|\| hint.0 - uknot_vec[0] > uknot_vec.range_length() + ctx.ratio_margin()` |
| `include` | 1947 | `\|\| hint.1 < vknot_vec[0] - TOLERANCE` | **`param`** — one-sided check that the resolved v parameter stays above the v knot domain bottom | `\|\| hint.1 < vknot_vec[0] - ctx.ratio_margin()` |
| `include` | 1948 | `\|\| hint.1 - vknot_vec[0] > vknot_vec.range_length() + TOLERANCE` | **`param`** — one-sided check that the resolved v parameter does not overshoot the v knot domain top | `\|\| hint.1 - vknot_vec[0] > vknot_vec.range_length() + ctx.ratio_margin()` |

**`knot_vec.rs`**

| enclosing fn | line | code | class and why | write instead |
|---|---|---|---|---|
| `same_range` | 44 | `self[0].near(&other[0]) && self.range_length().near(&other.range_length())` | **`param`** — knot values and the knot-range span are parameter-space (dimensionless) quantities, so range equality must not scale with the model | `ctx.is_small_ratio(self[0] - other[0]) && ctx.is_small_ratio(self.range_length() - other.range_length())` |
| `multiplicity` | 80 | `self.iter().filter(\|u\| self[i].near(u)).count()` | **`param`** — compares a knot value to its neighbours to count multiplicity; knots are dimensionless parameters | `self.iter().filter(\|u\| ctx.is_small_ratio(self[i] - *u)).count()` |
| `try_bspline_basis_functions` | 242 | `if self[0].near(&self[n]) {` | **`param`** — compares first and last knot values to detect a zero-range knot vector; knots are dimensionless parameters | `ctx.is_small_ratio(self[0] - self[n])` |
| `try_normalize` | 358 | `if range.so_small() {` | **`param`** — range is the knot-vector span in parameter space, so its zero-detection must not scale with the model | `ctx.is_small_ratio(range)` |
| `try_concat` | 450 | `if front < back \|\| !front.near(back) {` | **`param`** — front/back are knot values compared for equality; the front < back clause is a plain ordering, not a tolerance predicate | `if front < back \|\| !ctx.is_small_ratio(front - back) {` |
| `to_single_multi` | 510 | `if knot.near(next) {` | **`param`** — compares consecutive knot values to merge duplicates; knots are dimensionless parameters | `ctx.is_small_ratio(knot - *next)` |

**`mod.rs`**

| enclosing fn | line | code | class and why | write instead |
|---|---|---|---|---|
| `inv_or_zero` | 186 | `if delta.abs() <= TOLERANCE {` | **`param`** — delta is a knot-span difference in parameter space used only as a division guard; the fn is const, so the rewrite needs the ctx threaded or the fn made non-const | `ctx.is_small_ratio(delta)` |

**`nurbssurface.rs`**

| enclosing fn | line | code | class and why | write instead |
|---|---|---|---|---|
| `include` | 722 | `if !self.subs(hint.0, hint.1).near(&pt)` | **`model`** — compares the surface point at the resolved parameter to the curve point in model space; this is the deciding predicate of the include test | `!ctx.near_points(self.subs(hint.0, hint.1), pt)` |
| `include` | 723 | `\|\| hint.0 < uknot_vec[0] - TOLERANCE` | **`param`** — one-sided check that the resolved u parameter stays above the u knot domain bottom | `\|\| hint.0 < uknot_vec[0] - ctx.ratio_margin()` |
| `include` | 724 | `\|\| hint.0 - uknot_vec[0] > uknot_vec.range_length() + TOLERANCE` | **`param`** — one-sided check that the resolved u parameter does not overshoot the u knot domain top | `\|\| hint.0 - uknot_vec[0] > uknot_vec.range_length() + ctx.ratio_margin()` |
| `include` | 725 | `\|\| hint.1 < vknot_vec[0] - TOLERANCE` | **`param`** — one-sided check that the resolved v parameter stays above the v knot domain bottom | `\|\| hint.1 < vknot_vec[0] - ctx.ratio_margin()` |
| `include` | 726 | `\|\| hint.1 - vknot_vec[0] > vknot_vec.range_length() + TOLERANCE` | **`param`** — one-sided check that the resolved v parameter does not overshoot the v knot domain top | `\|\| hint.1 - vknot_vec[0] > vknot_vec.range_length() + ctx.ratio_margin()` |
| `include` | 769 | `if !self.subs(hint.0, hint.1).near(&pt)` | **`model`** — compares the surface point at the resolved parameter to the curve point in model space; this is the deciding predicate of the include test | `!ctx.near_points(self.subs(hint.0, hint.1), pt)` |
| `include` | 770 | `\|\| hint.0 < uknot_vec[0] - TOLERANCE` | **`param`** — one-sided check that the resolved u parameter stays above the u knot domain bottom | `\|\| hint.0 < uknot_vec[0] - ctx.ratio_margin()` |
| `include` | 771 | `\|\| hint.0 - uknot_vec[0] > uknot_vec.range_length() + TOLERANCE` | **`param`** — one-sided check that the resolved u parameter does not overshoot the u knot domain top | `\|\| hint.0 - uknot_vec[0] > uknot_vec.range_length() + ctx.ratio_margin()` |
| `include` | 772 | `\|\| hint.1 < vknot_vec[0] - TOLERANCE` | **`param`** — one-sided check that the resolved v parameter stays above the v knot domain bottom | `\|\| hint.1 < vknot_vec[0] - ctx.ratio_margin()` |
| `include` | 773 | `\|\| hint.1 - vknot_vec[0] > vknot_vec.range_length() + TOLERANCE` | **`param`** — one-sided check that the resolved v parameter does not overshoot the v knot domain top | `\|\| hint.1 - vknot_vec[0] > vknot_vec.range_length() + ctx.ratio_margin()` |
| `include` | 816 | `if !self.subs(hint.0, hint.1).near(&pt)` | **`model`** — compares the surface point at the resolved parameter to the curve point in model space; this is the deciding predicate of the include test | `!ctx.near_points(self.subs(hint.0, hint.1), pt)` |
| `include` | 817 | `\|\| hint.0 < uknot_vec[0] - TOLERANCE` | **`param`** — one-sided check that the resolved u parameter stays above the u knot domain bottom | `\|\| hint.0 < uknot_vec[0] - ctx.ratio_margin()` |
| `include` | 818 | `\|\| hint.0 - uknot_vec[0] > uknot_vec.range_length() + TOLERANCE` | **`param`** — one-sided check that the resolved u parameter does not overshoot the u knot domain top | `\|\| hint.0 - uknot_vec[0] > uknot_vec.range_length() + ctx.ratio_margin()` |
| `include` | 819 | `\|\| hint.1 < vknot_vec[0] - TOLERANCE` | **`param`** — one-sided check that the resolved v parameter stays above the v knot domain bottom | `\|\| hint.1 < vknot_vec[0] - ctx.ratio_margin()` |
| `include` | 820 | `\|\| hint.1 - vknot_vec[0] > vknot_vec.range_length() + TOLERANCE` | **`param`** — one-sided check that the resolved v parameter does not overshoot the v knot domain top | `\|\| hint.1 - vknot_vec[0] > vknot_vec.range_length() + ctx.ratio_margin()` |

## Not in this packet — 12 deferrals: a FIXME and nothing else

These sites are real and their line numbers resolve. You **leave the code
exactly as it is** and add one marker comment on the line above each. Do
not introduce a `ToleranceCtx` for a function that has only deferrals —
a FIXME is a comment and costs no context.

**`bspcurve.rs`**

| enclosing fn | line | code | marker |
|---|---|---|---|
| `is_const` | 474 | `.all(move \|vec\| vec.near(&self.control_points[0]))` | `// FIXME(BG-TOL-001, GENERIC_BOUND)` — ctx.near_points<P> is bounded P: MetricSpace<Metric = f64> and this impl does not supply it. Widening a public generic bound is cross-crate and is Stage B, so the site is left exactly as it is. Enclosing impl: `impl<P: ControlPoint<f64> + Tolerance> BSplineCurve<P>`. |
| `try_remove_knot` | 598 | `if !new_points.last().unwrap().near(self.control_point(idx)) {` | `// FIXME(BG-TOL-001, GENERIC_BOUND)` — ctx.near_points<P> is bounded P: MetricSpace<Metric = f64> and this impl does not supply it. Widening a public generic bound is cross-crate and is Stage B, so the site is left exactly as it is. Enclosing impl: `impl<P: ControlPoint<f64> + Tolerance> BSplineCurve<P>`. |
| `near_as_curve` | 921 | `self.sub_near_as_curve(other, 1, \|x, y\| x.near(y))` | `// FIXME(BG-TOL-001, GENERIC_BOUND)` — ctx.near_points<P> is bounded P: MetricSpace<Metric = f64> and this impl does not supply it. Widening a public generic bound is cross-crate and is Stage B, so the site is left exactly as it is. Enclosing impl: `impl<P: ControlPoint<f64> + Tolerance> BSplineCurve<P>`. |
| `try_concat` | 1037 | `if !front.near(back) {` | `// FIXME(BG-TOL-001, GENERIC_BOUND)` — ctx.near_points<P> is bounded P: MetricSpace<Metric = f64> and this impl does not supply it. Widening a public generic bound is cross-crate and is Stage B, so the site is left exactly as it is. Enclosing impl: `impl<P: ControlPoint<f64> + Tolerance> Concat<..> for BSplineCurve<P>`. |

**`bspsurface.rs`**

| enclosing fn | line | code | marker |
|---|---|---|---|
| `is_const` | 652 | `if !vec.near(&self.control_points[0][0]) {` | `// FIXME(BG-TOL-001, GENERIC_BOUND)` — ctx.near_points<P> is bounded P: MetricSpace<Metric = f64> and this impl does not supply it. Widening a public generic bound is cross-crate and is Stage B, so the site is left exactly as it is. Enclosing impl: `impl<V: Tolerance> BSplineSurface<V> -- not even ControlPoint`. |
| `try_remove_uknot` | 823 | `if !pt0.near(pt1) {` | `// FIXME(BG-TOL-001, GENERIC_BOUND)` — ctx.near_points<P> is bounded P: MetricSpace<Metric = f64> and this impl does not supply it. Widening a public generic bound is cross-crate and is Stage B, so the site is left exactly as it is. Enclosing impl: `impl<P: ControlPoint<f64> + Tolerance> BSplineSurface<P>`. |
| `try_remove_vknot` | 918 | `if !pt0.near(pt1) {` | `// FIXME(BG-TOL-001, GENERIC_BOUND)` — ctx.near_points<P> is bounded P: MetricSpace<Metric = f64> and this impl does not supply it. Widening a public generic bound is cross-crate and is Stage B, so the site is left exactly as it is. Enclosing impl: `impl<P: ControlPoint<f64> + Tolerance> BSplineSurface<P>`. |
| `near_as_surface` | 1581 | `self.sub_near_as_surface(other, 1, \|x, y\| x.near(y))` | `// FIXME(BG-TOL-001, GENERIC_BOUND)` — ctx.near_points<P> is bounded P: MetricSpace<Metric = f64> and this impl does not supply it. Widening a public generic bound is cross-crate and is Stage B, so the site is left exactly as it is. Enclosing impl: `impl<P: ControlPoint<f64> + Tolerance> BSplineSurface<P>`. |

**`nurbscurve.rs`**

| enclosing fn | line | code | marker |
|---|---|---|---|
| `is_const` | 170 | `.all(move \|vec\| vec.to_point().near(&pt))` | `// FIXME(BG-TOL-001, GENERIC_BOUND)` — ctx.near_points<P> is bounded P: MetricSpace<Metric = f64> and this impl does not supply it. Widening a public generic bound is cross-crate and is Stage B, so the site is left exactly as it is. Enclosing impl: `impl<V: Homogeneous + ControlPoint<f64, Diff = V>> where V::Point: Tolerance`. |
| `near_as_curve` | 199 | `.sub_near_as_curve(&other.0, 2, move \|x, y\| x.to_point().near(&y.to_point()))` | `// FIXME(BG-TOL-001, GENERIC_BOUND)` — ctx.near_points<P> is bounded P: MetricSpace<Metric = f64> and this impl does not supply it. Widening a public generic bound is cross-crate and is Stage B, so the site is left exactly as it is. Enclosing impl: `impl<V: Homogeneous + ControlPoint<f64, Diff = V>> where V::Point: Tolerance`. |

**`nurbssurface.rs`**

| enclosing fn | line | code | marker |
|---|---|---|---|
| `is_const` | 299 | `if !vec.to_point().near(&pt) {` | `// FIXME(BG-TOL-001, GENERIC_BOUND)` — ctx.near_points<P> is bounded P: MetricSpace<Metric = f64> and this impl does not supply it. Widening a public generic bound is cross-crate and is Stage B, so the site is left exactly as it is. Enclosing impl: `impl<V: Homogeneous + ControlPoint<f64, Diff = V>> where V::Point: Tolerance`. |
| `near_as_surface` | 329 | `.sub_near_as_surface(&other.0, 2, move \|x, y\| x.to_point().near(&y.to_point()))` | `// FIXME(BG-TOL-001, GENERIC_BOUND)` — ctx.near_points<P> is bounded P: MetricSpace<Metric = f64> and this impl does not supply it. Widening a public generic bound is cross-crate and is Stage B, so the site is left exactly as it is. Enclosing impl: `impl<V: Homogeneous + ControlPoint<f64, Diff = V>> where V::Point: Tolerance`. |

## Not in this packet — 23 excluded, no marker

- `bspcurve.rs:772` — squared order
- `bspcurve.rs:852` — not code: doc comment
- `bspcurve.rs:861` — not code: doc comment
- `bspcurve.rs:898` — not code: doc comment
- `bspcurve.rs:927` — not code: doc comment
- `bspcurve.rs:931` — not code: doc comment
- `bspcurve.rs:951` — squared order
- `bspsurface.rs:1082` — squared order
- `bspsurface.rs:1561` — not code: doc comment
- `bspsurface.rs:1586` — not code: doc comment
- `bspsurface.rs:1590` — not code: doc comment
- `bspsurface.rs:1608` — squared order
- `nurbscurve.rs:175` — not code: doc comment
- `nurbscurve.rs:205` — not code: doc comment
- `nurbscurve.rs:223` — not code: doc comment
- `nurbscurve.rs:229` — squared order
- `nurbscurve.rs:359` — not code: doc comment
- `nurbscurve.rs:370` — not code: doc comment
- `nurbssurface.rs:308` — not code: doc comment
- `nurbssurface.rs:335` — not code: doc comment
- `nurbssurface.rs:339` — not code: doc comment
- `nurbssurface.rs:358` — squared order
- `nurbssurface.rs:554` — not code: doc comment

### Everything else in these files

Leaving these alone is correct; migrating one is a rejection.

1. **All doc comments and `#[cfg(test)]` code.** `nurbs` is heavily documented
   with runnable examples and they account for most of the anchor counts. A doc
   example is prose and a test's epsilon is the test's own business.
2. **Anything using `.near2()` or `.so_small2()`.** The survey found six such
   helpers — `sub_near_as_curve`/`sub_near_as_surface` and the
   `near2_as_curve`/`near2_as_surface` family — and they are correctly
   squared-order: they compare against `TOLERANCE2` = 1e-12, which nothing on
   `ToleranceCtx` reproduces. Mapping them onto `tau_rep` would loosen them by
   six orders of magnitude while looking like a migration. They are deferred to
   BG-TOL-004 and are not your work.
3. **Any `TOLERANCE` used as a value rather than a comparison** — a `.max()`
   floor, a `+ TOLERANCE` offset, a `const` initializer, a `use` import. Such a
   line compares nothing, so it has no `model`/`param` class, and there is no
   `ctx` in scope for a `const` anyway. Its *consumers* are the sites.

## The ratchet — read this before you commit

`scripts/kernel-gates.sh` counts `unscaled_legacy(` call sites in
`vendor/truck/*/src/**` and **fails when the total exceeds the ceiling** in
`scripts/unscaled_legacy_ceiling.txt`. The ceiling has been raised to **97**,
covering the 50 already in the tree plus the budgets of this packet and the two
sibling shards dispatched alongside it. That file is **not** on your allowlist
and you must not edit it — the ceiling exists to constrain this packet, and a
packet that can move its own ceiling is not constrained by anything.

Because two other shards are running concurrently against the same ceiling, a
context you add that the budget did not account for is not merely over-budget;
it can push a sibling's correct work over the line. Introduce 26 and no
more.

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `unimplemented!`, `todo!`, or
  out-of-range indexing on any line you add. `unscaled_legacy()` is infallible
  and returns `Self`, so there is nothing to unwrap. Note that several lines in
  your table sit inside expressions that already contain an `unwrap()` —
  **leave those exactly as they are**; the rule is about lines you add, and
  rewriting an existing `unwrap` is out of scope and outside this packet.
- **H-2** Fallible operations return `Outcome<T>`. You are not adding any.
- **H-3** No absolute constants in predicates — that is the whole point of this
  packet. **`scripts/kernel-gates.sh` flags a bare float literal on any added
  line, and test epsilons trip it. The opt-out is a `// H-3` comment ON THE SAME
  LINE as the literal** — not on the line above, which does not work. Note also
  that **rustfmt will move a trailing `// H-3` off a line that opens a brace**,
  which silently defeats it; if that happens, extract the literal onto its own
  statement line and mark that. Use the opt-out in your tests and say what the
  quantity is.
- **H-6** A value computed in floats is never recorded as `Method::Exact`.

## Tests required

New file `vendor/truck/truck-geometry/tests/tolerance_nurbs.rs`.

**Its first line must be `#![deny(clippy::unwrap_used)]`.** GATE-1 (H-1)
requires it of every new module under `vendor/truck/`, including test files, and
`scripts/kernel-gates.sh` fails the packet without it. Every landed shard's test
file carries it — see `truck-shapeops/tests/tolerance_migration.rs` and
`truck-geometry/tests/analytic_carriers.rs`. Write your tests so the attribute
costs nothing: return `Result` or match rather than `unwrap`. This line is
called out because the last shard's packet omitted it and the omission, not the
worker, cost a round trip.

Each test must be a named `#[test]` fn — the verifier checks the names appear in
your diff, so the names below are exact.

1. `every_migrated_nurbs_site_is_marked` — read the migrated source files from
   `CARGO_MANIFEST_DIR` at runtime and assert that the number of lines
   containing `ctx.near_pt(`, `ctx.near_points(`, `ctx.is_small_len(`,
   `ctx.is_small_ratio(` or `ctx.ratio_margin()` equals the number containing a
   `// BG-TOL-001:` marker. This is what makes the marking checkable rather than
   a convention; without it the markers rot the first time someone edits a line.
2. `deferred_generic_bound_sites_carry_a_fixme` — assert that `bspcurve.rs`, `bspsurface.rs`,
   `nurbscurve.rs` and `nurbssurface.rs` contain exactly **4, 4, 2 and 2**
   lines matching `FIXME(BG-TOL-001, GENERIC_BOUND)` respectively. Twelve
   total. The count is the point: it is what stops a later reader from
   "finishing the job" by widening a bound, and what proves none of the
   twelve was quietly migrated instead.


**The crate hosting the test file is `truck-geometry`, and that is a decision, not
an accident.** It is the crate the migrated files live in and it has no `autotests = false`, so the file is picked up without touching `Cargo.toml` — which is not on your allowlist.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-geometry
cargo clippy -p truck-geometry --all-targets --no-deps -- -D warnings
cargo test -p truck-geometry --lib --test tolerance_nurbs --no-fail-fast
cargo check --workspace --all-targets
```

`--no-deps` matters: without it clippy lints the vendored path dependencies too
and aborts before reaching your crate. **Never run a bare `cargo test`** — it
builds 56 examples. Send cargo output to a file and read the tail.

The vendored tree is **not clean at the base commit** — neither clippy-clean nor
rustfmt-clean, and its test suite has pre-existing failures. Those are not
yours. The verifier scopes clippy to the lines your diff adds, rustfmt to the
files your diff changes, and test failures to the test functions your diff adds.
If a pre-existing test fails, **confirm it fails identically at the base commit,
record it in `baseline_failures`, and move on** — do not try to fix it and do
not let it stop you.

## Forbidden

Editing any file outside `write_allow` — in particular
`scripts/unscaled_legacy_ceiling.txt`, `scripts/kernel-gates.sh`,
`vendor/truck/truck-base/src/tolerance.rs`, and **`loop/` anything: your result
file goes in the root of your worktree and nowhere else.** Changing any function
signature. Adding or widening a generic bound. Adding a `ctx` parameter.
Changing any threshold. Introducing a `ToleranceCtx` in a function that has only
deferrals. Migrating a site the "Not in this packet" section excludes. Widening
a tolerance or adding `#[ignore]` to make a test pass. Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- a site in the table does not typecheck under the rewrite the table gives →
  `SPEC_GAP`, naming the site and the actual types. **Do not reclassify it to
  make it compile**, and do not reach for a different predicate because one
  compiles: a `model` site that will not take its recipe is telling you
  something, and reporting that is worth more than a green build. This packet's
  deferrals exist because exactly that check was run in advance.
- you cannot reach the context budget without constructing a context you would
  not otherwise write → finish the honest work and report `BUDGET_WRONG` under
  `disagreements`. Do not manufacture one.
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

Use the shape at the top of this document. `status` is one of `DONE`,
`ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any non-`DONE` status also write
`QUESTION.md` beside it.

Commit on the current branch with subject
`refactor(geometry): classify every nurbs tolerance site model or param (BG-TOL-001-GEOM-NURBS)`.
