# WORK PACKET BG-TOL-001-GEOM-DECORATORS — Stage-A tolerance migration, truck-geometry/src/decorators

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-TOL-001-GEOM-DECORATORS","status":"DONE","contracts":["BG-TOL-001"],
 "tests_added":2,"sites_migrated":28,"sites_deferred":1,"unscaled_legacy_calls":14,
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
id:          BG-TOL-001-GEOM-DECORATORS
contract:    [BG-TOL-001]
class:       wide-mechanical
crates:      [truck-geometry]
depends_on:  [BG-TOL-001-TYPE, BG-TOL-001-TYPE-r2, BG-TOL-001-TYPE-r3]
write_allow:
  - vendor/truck/truck-geometry/src/decorators/intersection_curve.rs
  - vendor/truck/truck-geometry/src/decorators/offset/curve.rs
  - vendor/truck/truck-geometry/src/decorators/offset/surface.rs
  - vendor/truck/truck-geometry/src/decorators/rbf_surface/algo.rs
  - vendor/truck/truck-geometry/src/decorators/rbf_surface/contact_circle.rs
  - vendor/truck/truck-geometry/src/decorators/revolved_curve.rs
  - vendor/truck/truck-geometry/tests/tolerance_decorators.rs
read_allow:
  - vendor/truck/truck-base/src/tolerance.rs
tests_required:
  - every_migrated_decorators_site_is_marked
  - the_deferred_dimension_site_carries_a_fixme
budget:      {turns: 80, ctx_tokens: 170000}
census_fragment: decorators
# CORRECTED AFTER DISPATCH (session 9): this was 14. The two offset
# `search_parameter` sites cannot take `ctx.near_points` -- both impls bound
# `P: ControlPoint<f64, Diff = V> + Copy + Tolerance` with no
# `MetricSpace<Metric = f64>`, so neither near_points nor .distance() exists
# there. The worker returned SPEC_GAP and was right; the packet never ran the
# bound check that the GEOM-NURBS packet ran over all twenty of its model
# rows. Both sites deferred FIXME(BG-TOL-001, GENERIC_BOUND), 14 -> 12.
unscaled_legacy_budget: 12
anchors:
  - {id: A1, expect: 1, cmd: "grep -cE '\\.near\\(|so_small\\(|TOLERANCE' vendor/truck/truck-geometry/src/decorators/intersection_curve.rs"}
  - {id: A2, expect: 2, cmd: "grep -cE '\\.near\\(|so_small\\(|TOLERANCE' vendor/truck/truck-geometry/src/decorators/offset/curve.rs"}
  - {id: A3, expect: 3, cmd: "grep -cE '\\.near\\(|so_small\\(|TOLERANCE' vendor/truck/truck-geometry/src/decorators/offset/surface.rs"}
  - {id: A4, expect: 14, cmd: "grep -cE '\\.near\\(|so_small\\(|TOLERANCE' vendor/truck/truck-geometry/src/decorators/rbf_surface/algo.rs"}
  - {id: A5, expect: 2, cmd: "grep -cE '\\.near\\(|so_small\\(|TOLERANCE' vendor/truck/truck-geometry/src/decorators/rbf_surface/contact_circle.rs"}
  - {id: A6, expect: 8, cmd: "grep -cE '\\.near\\(|so_small\\(|TOLERANCE' vendor/truck/truck-geometry/src/decorators/revolved_curve.rs"}
```

## Problem

`truck-geometry/src/decorators` is where surfaces get built on top of other
surfaces: revolutions, offsets, intersection curves, and the RBF fillet
machinery. Unlike `nurbs`, this module is dominated by **model-space**
predicates — 23 of its 28 live sites are `model` — because a decorator's job is
to place real geometry relative to real geometry: a contact point on a fillet, an
offset distance, a point on an axis of revolution.

That makes it the module where an unscaled tolerance does the most visible
damage. A fillet that converges on a 10mm bracket and refuses on the identical
30m part is exactly this bug, and the RBF code in `rbf_surface/` is a Newton
iteration whose convergence tests are all absolute lengths. Nothing in the source
records that they are lengths.

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
14 functions in this packet hold a migrated site, so you should introduce
exactly **14** `unscaled_legacy()` calls. See "The ratchet" — this number
is enforced by a gate, and it is a budget rather than an allowance.

**If you cannot reach 14 honestly, say so and stop.** That instruction is
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

4. **`revolved_curve.rs:29` and `:434` are `model`, and the degree-2 heuristic
   that flags them is wrong here — this is already adjudicated.**
   `(p - self.origin).cross(self.axis)` looks like an area and is not:
   `RevolutedCurve::new` **normalizes its axis**, so `self.axis` is a unit vector
   and the cross-product magnitude is `|p - origin| · sin(t)` — degree ONE in
   length. `ctx.is_small_len(...magnitude())` is correct. Do not defer these and
   do not raise them as a SPEC_GAP; the check has been made against the
   constructor.

5. **`algo.rs:849` and `:959` each carry four predicates and you migrate exactly
   one of them.** The line is
   `if p0.near(&e) && dt.so_small2() && ds.so_small2() && dw.so_small2() {`.
   Only the leading `near` migrates, to `ctx.near_pt(p0, e)`; the three
   `so_small2()` calls are squared-order against `TOLERANCE2` = 1e-12, which
   nothing on `ToleranceCtx` reproduces, and they stay verbatim. **This is a
   partial rewrite of a compound condition and it drops no guard** — it has been
   checked. Leaving the `so_small2` calls in place is the correct outcome, not
   an unfinished one.

6. **`rbf_surface/contact_circle.rs:167` is deferred, and the reason is a
   dimension you have not seen in an earlier shard.** The line is
   `debug_assert!(del.z.so_small(), "{del:?}");` where `del` solves
   `mat * del = q - p` and `mat`'s third column is the **unnormalized** normal
   `uder × vder`. The first two columns are degree 1 in length and the third is
   degree 2, so `del.x` and `del.y` come out dimensionless — the next line uses
   them as parameter increments, which confirms it — while `del.z` has dimension
   `1/length`. It is not a length, so `is_small_len` is wrong; it is not
   dimensionless, so `is_small_ratio` is wrong; and under a model rescale it
   moves in the **opposite** direction from `length_margin()`. It gets
   `FIXME(BG-TOL-001, DIMENSION)` and no rewrite.

   That it is a `debug_assert!` is not a reason to migrate it cheaply. At Stage A
   `model_scale = 1.0`, so every rewrite here is a no-op today and the entire
   cost of a wrong one lands on Stage B, which sees a migrated site and never
   looks again. This exclusion was reported by the survey worker as a SPEC_GAP,
   it was right, and the specification has been amended because of it.

## The sites — 28 migrate, 14 contexts

Line numbers are provenance for a human reader; locate by the enclosing symbol.

**`intersection_curve.rs`**

| enclosing fn | line | code | class and why | write instead |
|---|---|---|---|---|
| `search_parameter` | 353 | `match pt.near(&point) {` | **`model`** — pt is the point on the intersection curve at parameter t and point is the query point, so this is the distance between two points in model space | `match ctx.near_pt(pt, point) {` |

**`curve.rs`**

| enclosing fn | line | code | class and why | write instead |
|---|---|---|---|---|
| `period` | 42 | `(Some(x), Some(y)) if x.near(&y) => Some((x + y) / 2.0),` | **`param`** — x and y are curve periods, i.e. parameter-space increments after which the curve repeats, not model-space lengths | `(Some(x), Some(y)) if ctx.is_small_ratio(x - y) => Some((x + y) / 2.0),` |

**`surface.rs`**

| enclosing fn | line | code | class and why | write instead |
|---|---|---|---|---|
| `u_period` | 54 | `(Some(x), Some(y)) if x.near(&y) => Some((x + y) / 2.0),` | **`param`** — x and y are surface u-periods, i.e. parameter-space increments in the u direction, not model-space lengths | `(Some(x), Some(y)) if ctx.is_small_ratio(x - y) => Some((x + y) / 2.0),` |
| `v_period` | 61 | `(Some(x), Some(y)) if x.near(&y) => Some((x + y) / 2.0),` | **`param`** — x and y are surface v-periods, i.e. parameter-space increments in the v direction, not model-space lengths | `(Some(x), Some(y)) if ctx.is_small_ratio(x - y) => Some((x + y) / 2.0),` |

**`algo.rs`**

| enclosing fn | line | code | class and why | write instead |
|---|---|---|---|---|
| `search_parameter` | 671 | `if pp0.so_small() {` | **`model`** — pp0 is the displacement p0 - point between the point on surface0 and the query point in model space, so its magnitude is a length | `if ctx.is_small_len(pp0.magnitude()) {` |
| `search_parameter` | 678 | `if pp1.so_small() {` | **`model`** — pp1 is the displacement p1 - point between the point on surface1 and the query point in model space, so its magnitude is a length | `if ctx.is_small_len(pp1.magnitude()) {` |
| `search_parameter` | 685 | `let center_contact0 = (p0 + r * n0).near(&c);` | **`model`** — p0 + r * n0 is the rolling-ball center in model space and c is the edge-curve point, so this is the distance between two points in model space | `let center_contact0 = ctx.near_pt(p0 + r * n0, c);` |
| `search_parameter` | 686 | `let center_contact1 = (p1 + r * n1).near(&c);` | **`model`** — p1 + r * n1 is the rolling-ball center in model space and c is the edge-curve point, so this is the distance between two points in model space | `let center_contact1 = ctx.near_pt(p1 + r * n1, c);` |
| `search_parameter` | 711 | `debug_assert!(duv0.0.z.so_small() && duv0.1.z.so_small(), "{duv0:?}");` | **`model`** — duv0 is the solution of a Newton system whose third matrix column is the unit normal n0, so duv0.0.z and duv0.1.z are the normal-direction position and velocity residuals, lengths in model space | `debug_assert!(ctx.is_small_len(duv0.0.z) && ctx.is_small_len(duv0.1.z), "{duv0:?}");` |
| `search_parameter` | 721 | `debug_assert!(duv1.0.z.so_small() && duv1.1.z.so_small(), "{duv1:?}");` | **`model`** — duv1 is the solution of a Newton system whose third matrix column is the unit normal n1, so duv1.0.z and duv1.1.z are the normal-direction position and velocity residuals, lengths in model space | `debug_assert!(ctx.is_small_len(duv1.0.z) && ctx.is_small_len(duv1.1.z), "{duv1:?}");` |
| `search_parameter` | 746 | `match (rot * cp0).near(&cp) {` | **`model`** — cp0 and cp are radius vectors of length r from the center, so the difference rot * cp0 - cp is a chord between two points on the ball surface, a length in model space (not a pure sine, the vectors are not unit) | `match ctx.is_small_len((rot * cp0 - cp).magnitude()) {` |
| `search_contact_curve0_cross_point_with_adjacent_edge` | 826 | `debug_assert!(duv0.0.z.so_small() && duv0.1.z.so_small(), "{duv0:?}");` | **`model`** — duv0 is the solution of a Newton system whose third matrix column is the unit normal n0, so duv0.0.z and duv0.1.z are the normal-direction position and velocity residuals, lengths in model space | `debug_assert!(ctx.is_small_len(duv0.0.z) && ctx.is_small_len(duv0.1.z), "{duv0:?}");` |
| `search_contact_curve0_cross_point_with_adjacent_edge` | 836 | `debug_assert!(duv1.0.z.so_small() && duv1.1.z.so_small(), "{duv1:?}");` | **`model`** — duv1 is the solution of a Newton system whose third matrix column is the unit normal n1, so duv1.0.z and duv1.1.z are the normal-direction position and velocity residuals, lengths in model space | `debug_assert!(ctx.is_small_len(duv1.0.z) && ctx.is_small_len(duv1.1.z), "{duv1:?}");` |
| `search_contact_curve0_cross_point_with_adjacent_edge` | 849 | `if p0.near(&e) && dt.so_small2() && ds.so_small2() && dw.so_small2() {` | **`model`** — the deciding test is whether the contact point p0 lies on the adjacent curve (a model-space distance); dt and ds are edge/adjacent parameter increments (dimensionless) and dw is the normal-direction offset residual (a length), all three deferred to BG-TOL-004 as so_small2 squared-order sites **REVIEWED — orchestrator, session 8: the partial rewrite is CORRECT. The three unmigrated predicates on the line are so_small2(), squared order, excluded by rule -- the rewrite drops no guard.** | `if ctx.near_pt(p0, e) && dt.so_small2() && ds.so_small2() && dw.so_small2() {` |
| `search_contact_curve1_cross_point_with_adjacent_edge` | 936 | `debug_assert!(duv0.0.z.so_small() && duv0.1.z.so_small(), "{duv0:?}");` | **`model`** — duv0 is the solution of a Newton system whose third matrix column is the unit normal n0, so duv0.0.z and duv0.1.z are the normal-direction position and velocity residuals, lengths in model space | `debug_assert!(ctx.is_small_len(duv0.0.z) && ctx.is_small_len(duv0.1.z), "{duv0:?}");` |
| `search_contact_curve1_cross_point_with_adjacent_edge` | 946 | `debug_assert!(duv1.0.z.so_small() && duv1.1.z.so_small(), "{duv1:?}");` | **`model`** — duv1 is the solution of a Newton system whose third matrix column is the unit normal n1, so duv1.0.z and duv1.1.z are the normal-direction position and velocity residuals, lengths in model space | `debug_assert!(ctx.is_small_len(duv1.0.z) && ctx.is_small_len(duv1.1.z), "{duv1:?}");` |
| `search_contact_curve1_cross_point_with_adjacent_edge` | 959 | `if p1.near(&e) && dt.so_small2() && ds.so_small2() && dw.so_small2() {` | **`model`** — the deciding test is whether the contact point p1 lies on the adjacent curve (a model-space distance); dt and ds are edge/adjacent parameter increments (dimensionless) and dw is the normal-direction offset residual (a length), all three deferred to BG-TOL-004 as so_small2 squared-order sites **REVIEWED — orchestrator, session 8: the partial rewrite is CORRECT. The three unmigrated predicates on the line are so_small2(), squared order, excluded by rule -- the rewrite drops no guard.** | `if ctx.near_pt(p1, e) && dt.so_small2() && ds.so_small2() && dw.so_small2() {` |

**`contact_circle.rs`**

| enclosing fn | line | code | class and why | write instead |
|---|---|---|---|---|
| `try_new` | 61 | `if p0.near(&q0) && p1.near(&q1) {` | **`model`** — p0 and q0 are candidate contact points on surface0 and p1 and q1 on surface1, so each predicate is the distance between two points in model space | `if ctx.near_pt(p0, q0) && ctx.near_pt(p1, q1) {` |

**`revolved_curve.rs`**

| enclosing fn | line | code | class and why | write instead |
|---|---|---|---|---|
| `contains` | 29 | `(p - self.origin).cross(self.axis).so_small()` | **`model`** — p - self.origin is a displacement and self.axis is a unit vector, so the cross-product magnitude is |displacement| sin(theta), which still carries length units **REVIEWED — orchestrator, session 8: `model` is CORRECT. RevolutedCurve::new normalizes its axis, so (p - origin).cross(axis) is |v| sin t -- degree ONE. The degree-2 heuristic flags this shape deliberately and cannot resolve it statically.** | `ctx.is_small_len((p - self.origin).cross(self.axis).magnitude())` |
| `normal` | 155 | `let (uder, vder) = if u.near(&u0) {` | **`param`** — u and u0 are curve parameters of the entity curve, so this compares two dimensionless parameter values | `let (uder, vder) = if ctx.is_small_ratio(u - u0) {` |
| `normal` | 158 | `if radius.so_small() {` | **`model`** — radius = self.axis().cross(pt - self.origin()) is the perpendicular distance of the curve point from the revolution axis, a length in model space | `if ctx.is_small_len(radius.magnitude()) {` |
| `normal` | 164 | `} else if u.near(&u1) {` | **`param`** — u and u1 are curve parameters of the entity curve, so this compares two dimensionless parameter values | `} else if ctx.is_small_ratio(u - u1) {` |
| `normal` | 167 | `if radius.so_small() {` | **`model`** — radius = self.axis().cross(pt - self.origin()) is the perpendicular distance of the curve point from the revolution axis, a length in model space | `if ctx.is_small_len(radius.magnitude()) {` |
| `search_parameter` | 373 | `if self.is_front_fixed() && self.curve.front().near(&point) {` | **`model`** — self.curve.front() is the front point of the entity curve in model space and point is the query point, so this is the distance between two points in model space | `if self.is_front_fixed() && ctx.near_pt(self.curve.front(), point) {` |
| `search_parameter` | 379 | `} else if self.is_back_fixed() && self.curve.back().near(&point) {` | **`model`** — self.curve.back() is the back point of the entity curve in model space and point is the query point, so this is the distance between two points in model space | `} else if self.is_back_fixed() && ctx.near_pt(self.curve.back(), point) {` |
| `search_nearest_parameter` | 434 | `op.cross(self.revolution.axis).so_small() && op.dot(normal) >= 0.0` | **`model`** — op = point - o is a displacement and self.revolution.axis is a unit vector, so the cross-product magnitude is |op| sin(theta), a length in model space; the trailing dot-product sign check is not a tolerance predicate **REVIEWED — orchestrator, session 8: `model` is CORRECT. RevolutedCurve::new normalizes its axis, so (p - origin).cross(axis) is |v| sin t -- degree ONE. The degree-2 heuristic flags this shape deliberately and cannot resolve it statically.** | `ctx.is_small_len(op.cross(self.revolution.axis).magnitude()) && op.dot(normal) >= 0.0` |

## Not in this packet — 3 deferrals: a FIXME and nothing else

**Two of these three were added by amendment after the worker reported them**
(see the corrected budget above). `offset/curve.rs` `search_parameter` and
`offset/surface.rs` `search_parameter` are `model` sites whose classification is
correct and whose rewrite does not compile: the enclosing
`impl<C, N, P, V> SearchParameter<D1> for Offset<C, N>` and
`impl<S, N, P, V> SearchParameter<D2> for Offset<S, N>` both bound
`P: ControlPoint<f64, Diff = V> + Copy + Tolerance`, which supplies no
`MetricSpace<Metric = f64>`. Widening a public generic bound is cross-crate and
is Stage B. Both take `// FIXME(BG-TOL-001, GENERIC_BOUND)` and no rewrite —
the same treatment as the twelve in `BG-TOL-001-GEOM-NURBS`.

These sites are real and their line numbers resolve. You **leave the code
exactly as it is** and add one marker comment on the line above each. Do
not introduce a `ToleranceCtx` for a function that has only deferrals —
a FIXME is a comment and costs no context.

**`contact_circle.rs`**

| enclosing fn | line | code | marker |
|---|---|---|---|
| `next_point` | 167 | `debug_assert!(del.z.so_small(), "{del:?}");` | `// FIXME(BG-TOL-001, DIMENSION)` — SPEC_GAP: del is the Newton solution of mat * (du, dv, del.z) = vec where the third matrix column is the unnormalized normal uder x vder (magnitude is a parametrization area, degree 2 in length), so del.z is dimensionally 1/length and scales as 1/k under a model rescale -- it is not a length and not dimensionless, so neither model nor param, and either rewrite changes the debug assertion's behaviour under a real model_scale |

## Not in this packet — 2 excluded, no marker

- `algo.rs:581` — dist2 is a squared distance and r*r is a squared radius, so the compared quantity is degree 2 in length (length-squared); there is no predicate for it on ToleranceCtx
- `algo.rs:687` — a scalar triple product of three displacements (degree 3 in length) tested with the squared-order so_small2; no ToleranceCtx predicate reproduces TOLERANCE^2 or handles the volume dimension

<!-- 1 low-confidence row(s) above. Review each against the
     source before dispatching; that is the half V10 cannot check. -->

### Everything else in these files

Leaving these alone is correct; migrating one is a rejection.

1. **All doc comments and `#[cfg(test)]` code.**
2. **Every `.near2()` and `.so_small2()`,** including the three on `algo.rs:849`
   and the three on `:959`. Squared order against `TOLERANCE2` = 1e-12; deferred
   to BG-TOL-004.
3. **Any `TOLERANCE` used as a value rather than a comparison** — a `.max()`
   floor, an offset, a `const` initializer. It compares nothing, so it has no
   class.

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
it can push a sibling's correct work over the line. Introduce 14 and no
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

New file `vendor/truck/truck-geometry/tests/tolerance_decorators.rs`.

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

1. `every_migrated_decorators_site_is_marked` — read the migrated source files from
   `CARGO_MANIFEST_DIR` at runtime and assert that the number of lines
   containing `ctx.near_pt(`, `ctx.near_points(`, `ctx.is_small_len(`,
   `ctx.is_small_ratio(` or `ctx.ratio_margin()` equals the number containing a
   `// BG-TOL-001:` marker. This is what makes the marking checkable rather than
   a convention; without it the markers rot the first time someone edits a line.
2. `the_deferred_dimension_site_carries_a_fixme` — assert that
   `decorators/rbf_surface/contact_circle.rs` contains exactly **one** line
   matching `FIXME(BG-TOL-001, DIMENSION)` and that the file contains **no**
   `ToleranceCtx` at all. The second half is the load-bearing one: it proves
   the deferred site was not migrated and that the file costs nothing against
   the ratchet.


**The crate hosting the test file is `truck-geometry`, and that is a decision, not
an accident.** It is the crate the migrated files live in and it has no `autotests = false`, so the file is picked up without touching `Cargo.toml` — which is not on your allowlist.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-geometry
cargo clippy -p truck-geometry --all-targets --no-deps -- -D warnings
cargo test -p truck-geometry --lib --test tolerance_decorators --no-fail-fast
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
`refactor(geometry): classify every decorators tolerance site model or param (BG-TOL-001-GEOM-DECORATORS)`.
