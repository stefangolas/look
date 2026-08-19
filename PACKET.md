# WORK PACKET BG-TOL-001-STEPIO — Stage-A tolerance migration, truck-stepio

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```yaml
id:          BG-TOL-001-STEPIO
contract:    [BG-TOL-001]
class:       wide-mechanical
crates:      [truck-stepio]
depends_on:  [BG-TOL-001-TYPE, BG-TOL-001-TYPE-r2, BG-TOL-001-TYPE-r3]
write_allow:
  - vendor/truck/truck-stepio/src/in/mod.rs
  - vendor/truck/truck-stepio/src/in/step_geometry/degenerate_torus.rs
  - vendor/truck/truck-stepio/src/in/step_geometry/geom_impls.rs
  - vendor/truck/truck-stepio/src/in/step_geometry/stepout_impls.rs
  - vendor/truck/truck-stepio/src/out/geometry.rs
  - vendor/truck/truck-stepio/tests/tolerance_stepio.rs
read_allow:
  - vendor/truck/truck-base/src/tolerance.rs
tests_required:
  - every_migrated_stepio_site_is_marked
  - scale_factor_comparisons_do_not_scale_with_the_model
budget:      {turns: 70, ctx_tokens: 150000}
# Read by run_packet.py's dispatch preflight: GATE-4's count plus this budget
# must fit under the ceiling committed on the slot's branch. 15 = one context
# per enclosing function in the table below. See "The ratchet".
unscaled_legacy_budget: 15
census_fragment: truck-stepio
# Runnable form of the anchor table below. `python loop/gen_packet.py --check`
# executes these and refuses on a mismatch, and run_packet.py calls it before
# dispatch -- the markdown table is for a human reader, and a table is not a
# thing a script can run, which is how three counts once shipped wrong.
anchors:
  - {id: A1, expect: 7, cmd: "grep -cE '\.near\(|so_small\(|TOLERANCE' vendor/truck/truck-stepio/src/in/mod.rs"}
  - {id: A2, expect: 5, cmd: "grep -cE '\.near\(|so_small\(|TOLERANCE' vendor/truck/truck-stepio/src/in/step_geometry/degenerate_torus.rs"}
  - {id: A3, expect: 2, cmd: "grep -cE '\.near\(|so_small\(|TOLERANCE' vendor/truck/truck-stepio/src/in/step_geometry/geom_impls.rs"}
  - {id: A4, expect: 1, cmd: "grep -cE '\.near\(|so_small\(|TOLERANCE' vendor/truck/truck-stepio/src/in/step_geometry/stepout_impls.rs"}
  - {id: A5, expect: 5, cmd: "grep -cE '\.near\(|so_small\(|TOLERANCE' vendor/truck/truck-stepio/src/out/geometry.rs"}
```

## Problem — why this is reachable from untrusted geometry

`truck-stepio` **is** the untrusted boundary. It is the crate that reads a STEP
file written by someone else's CAD system and turns it into kernel geometry, and
the crate that writes one back out. Every tolerance predicate in it decides
something about a file the kernel did not produce and cannot trust: whether a
knot range is degenerate, whether two curve endpoints coincide, whether a
transform is a uniform scale. All 19 of them currently compare against
`TOLERANCE`, an absolute constant of `1.0e-6`, which means the same physical
question gets a different answer depending on whether the exporter wrote metres
or millimetres. That is exactly what BG-TOL-001 exists to fix, and stepio is the
most directly reachable surface in the tree.

**This is Stage A.** You classify every site `model` (a length, scales with the
model) or `param` (dimensionless — a ratio, an angle, a sine, a knot value, a
scale factor — never scales) and route it through `ToleranceCtx`. You move **no
threshold**: `ToleranceCtx::unscaled_legacy()` carries the same absolute value
the code has today. A later packet threads a real `model_scale` and that is what
actually changes behaviour. **Your diff must not change what any predicate
decides.**

## Anchors — verified 2026-08-18, counts are exact

Locate by running the pattern. **Never locate by line number.** `rg` is not
installed on this machine; any case-sensitive literal search is equivalent.
**If a count differs, STOP** and report `ANCHOR_MISMATCH`.

Counts are **matching lines** for `\.near\(|so_small\(|TOLERANCE`, i.e.
`grep -cE '\.near\(|so_small\(|TOLERANCE' <file>`, over the whole file:

| # | file | expect |
|---|---|---|
| A1 | `src/in/mod.rs` | **7** |
| A2 | `src/in/step_geometry/degenerate_torus.rs` | **5** |
| A3 | `src/in/step_geometry/geom_impls.rs` | **2** |
| A4 | `src/in/step_geometry/stepout_impls.rs` | **1** |
| A5 | `src/out/geometry.rs` | **5** |
| A6 | `truck-base/src/tolerance.rs`, pattern `length_margin` | **≥1** |

A1 is 7 but only **6** are sites: one is a prose comment mentioning `TOLERANCE`
inside the doc block above `try_from`. Leave the comment alone. A2 is 5 but only
**4** are sites: the fifth is an `assert!` inside the `#[cfg(test)]` module at
the bottom of the file, which is the test's own business and not migration work.
A5 is 5 lines but **6** predicates — see the table.

A6 is a dependency check: `length_margin`, `ratio_margin`, `near_pt` and
`is_small_ratio` must already exist on `ToleranceCtx`. If they do not, report
`BLOCKED`; do not write them.

## The recipes — use these and nothing else

| classification | shape | rewrite |
|---|---|---|
| `model` | two `Point3` | `ctx.near_pt(a, b)` |
| `model` | a length, against zero | `ctx.is_small_len(l)` |
| `param` | two `f64` dimensionless values | `ctx.is_small_ratio(a - b)` |
| `param` | a dimensionless value against zero | `ctx.is_small_ratio(x)` |
| `param` | a one-sided margin on a parameter | `ctx.ratio_margin()` |

Obtain the context **once at the top of each function that contains at least one
site**, as `let ctx = ToleranceCtx::unscaled_legacy();`. Do not construct one per
site — `sub_parse_curve3d` has three sites and gets **one** context, and two of
those three are `param` while the third is `model`, which is normal. **Do not
add a parameter to any signature or a bound to any `where` clause** — Stage B
does that, and doing it here breaks callers in crates not on your allowlist.

Every rewritten line carries a trailing `// BG-TOL-001: model` or
`// BG-TOL-001: param`. If rustfmt relocates the comment to the following line,
leave it where rustfmt puts it.

## The sites — 19 predicates in 15 functions

Line numbers are provenance for a human reader; locate by the enclosing symbol.

**`src/in/mod.rs`** — 6 sites in 3 functions

| enclosing fn | line | code | class |
|---|---|---|---|
| `TryFrom<&BSplineCurveWithKnots> for BSplineCurve<P>::try_from` | 1879 | `if kv.range_length().so_small()` | **`param`** — a knot range is a parameter-space extent, not a length in the model |
| `EdgeCurve::sub_parse_2d` | 3190 | `if v < u - TOLERANCE` | **`param`** — `u`,`v` are angles on a `UnitCircle`; use `u - ctx.ratio_margin()` |
| `EdgeCurve::sub_parse_2d` | 3213 | `if v < u - TOLERANCE` | **`param`**, same |
| `EdgeCurve::sub_parse_curve3d` | 3309 | `if v < u - TOLERANCE` | **`param`**, same |
| `EdgeCurve::sub_parse_curve3d` | 3341 | `if v < u - TOLERANCE` | **`param`**, same |
| `EdgeCurve::sub_parse_curve3d` | 3416 | `if p.near(&q)` | **`model`** — `p`,`q` are the curve's endpoints in model space → `ctx.near_pt(p, q)` |

The four `v < u - TOLERANCE` sites are **margins, not predicates**: the rewrite
is `if v < u - ctx.ratio_margin()`, keeping the subtraction. Do not turn them
into `is_small_ratio`; that would change what they decide.

**`src/in/step_geometry/degenerate_torus.rs`** — 4 sites in 4 functions

| enclosing fn | line | code | class |
|---|---|---|---|
| `DegenerateTorus::inverse_outer` | 105 | `if rxy.so_small()` | **`model`** — `rxy` is the xy part of `point - center`, a length → `ctx.is_small_len(rxy.magnitude())` |
| `DegenerateTorus::inverse_inner` | 141 | `if rxy.so_small()` | **`model`**, same |
| `SearchParameter<D2>::search_parameter` | 234 | `match self.subs(u, v).near(&point)` | **`model`** → `ctx.near_pt(self.subs(u, v), point)` |
| `SearchNearestParameter<D2>::search_nearest_parameter` | 251 | `if self.subs(uv.0, uv.1).near(&point)` | **`model`** → `ctx.near_pt(...)` |

**`src/in/step_geometry/geom_impls.rs`** — 2 sites in 2 functions

| enclosing fn | line | code | class |
|---|---|---|---|
| `IncludeCurve<Curve3D> for Plane::include` | 72 | `axis.cross(self.normal()).so_small()` | **`param`** — `self.normal()` is unit and `axis` comes from a posture matrix, so this is a sine (times a dimensionless scale factor if the posture carries one). Dimensionless either way → `ctx.is_small_ratio(axis.cross(self.normal()).magnitude())` |
| `ToSameGeometry<Surface> for RevolutedCurve<Curve3D>::to_same_geometry` | 137 | `if v.cross(axis).so_small()` | **`model`** — `v = q - p` is a model-space vector, so `\|v × axis\|` is `\|v\| sin θ` and carries length units → `ctx.is_small_len(v.cross(axis).magnitude())` |

Those two look identical and are classified differently. That is the whole point
of this packet: 72 crosses two directions, 137 crosses a *displacement* with a
direction. If you think either is wrong, say so in `RESULT.json` — a disagreement
here is worth more than silent compliance.

**`src/in/step_geometry/stepout_impls.rs`** — 1 site

| enclosing fn | line | code | class |
|---|---|---|---|
| `DisplayByStep for Processor<DegenerateTorus, Matrix4>::fmt` | 48 | `if !r0.near(&r1)` | **`param`** — `r0`,`r1` are `transform[i].magnitude()`, i.e. **scale factors**. A scale factor is dimensionless: comparing two of them asks "is this transform a uniform scale", and the answer must not depend on model units → `!ctx.is_small_ratio(r0 - r1)` |

**`src/out/geometry.rs`** — 6 predicates in 5 functions, **every one `param`**,
all the same uniform-scale test as above:

| enclosing impl | line | code |
|---|---|---|
| `Processor<TrimmedCurve<UnitCircle<Point2>>, Matrix3>` | 256 | `if r0.near(&r1)` → `ctx.is_small_ratio(r0 - r1)` |
| `Processor<TrimmedCurve<UnitCircle<Point3>>, Matrix4>` | 286 | `if r0.near(&r1)` |
| `Processor<Sphere, Matrix4>` | 548 | `if !r0.near(&r1)` |
| `Processor<Torus, Matrix4>` | 585 | `if !r0.near(&r1)` |
| `Processor<RevolutedCurve<C>, Matrix4>` | 813 | `if !a[0][0].near(&a[1][1]) \|\| !a[1][1].near(&a[2][2])` — **two** predicates on one line, both `param` |

## What is NOT in this packet

There is one `assert!(point.near(&back), ...)` in `degenerate_torus.rs` inside
the `#[cfg(test)]` module. A test's own epsilon is the test's business. **Leave
it exactly as it is** — migrating it is a V1-clean but wrong change, and the
anchor count A2 of 5 already accounts for it.

There are no squared-order (`near2`, `so_small2`, `TOLERANCE2`) sites in this
crate. If you find one, that is an `ANCHOR_MISMATCH`.

## Tests required

New file `vendor/truck/truck-stepio/tests/tolerance_stepio.rs`. Each must be a
named `#[test]` fn.

**Its first line must be `#![deny(clippy::unwrap_used)]`.** GATE-1 (H-1)
requires it of every new module under `vendor/truck/`, including test files, and
`scripts/kernel-gates.sh` fails the packet without it. Every landed shard's test
file carries it -- see `truck-shapeops/tests/tolerance_migration.rs` and
`truck-geometry/tests/tolerance_nurbs.rs`. Write your tests so the attribute
costs nothing: return `Result` or match rather than `unwrap`. This line is
called out because a previous shard's packet omitted it and the omission -- the
orchestrator's, not the worker's -- cost a full round trip.

## The ratchet -- read this before you commit

`scripts/kernel-gates.sh` counts `unscaled_legacy(` call sites in
`vendor/truck/*/src/**` and **fails when the total exceeds the ceiling** in
`scripts/unscaled_legacy_ceiling.txt`. The ceiling has been raised to **111**,
covering the 75 already in the tree plus this packet's 15 and the budgets of the
two sibling shards dispatched alongside it. That file is **not** on your
allowlist and you must not edit it -- a packet that can move its own ceiling is
not constrained by anything.

Fifteen is a budget and it is exact, not an allowance: one context per function
containing a migrated site. **If you cannot reach it honestly, say so and
stop.** A previous shard's packet demanded 11 contexts when the truth was 10 and
its worker built a shadow `let ctx = ...` inside a `match` arm to satisfy the
number. It was obeying a packet that was wrong. A `disagreements` entry with
code `BUDGET_WRONG` is worth more here than a green gate.

Note also that **GATE-4 counts the token `unscaled_legacy(` anywhere in the
file, comments included** -- so do not write the constructor's name with its
parentheses inside a comment, or you will inflate the ratchet by one.

1. `every_migrated_stepio_site_is_marked` — read the five source files with
   `include_str!` and assert the number of `// BG-TOL-001:` markers equals the
   number of sites you migrated, and that no `.near(`/`so_small(`/`TOLERANCE`
   remains on a line you touched outside a comment or a `#[cfg(test)]` block.
   This is the test that keeps the migration honest; write it first and let it
   fail until the file set is done.
2. `scale_factor_comparisons_do_not_scale_with_the_model` — build `ToleranceCtx`
   at several `model_scale` values and assert `is_small_ratio` gives identical
   answers at all of them. This is the invariant the eleven `param` scale-factor
   classifications rest on: a transform is uniformly scaled or it is not, and
   the model's units have no say in it.

**`truck-stepio` does not set `autotests = false`** — check `Cargo.toml` and
report it if that is not what you find; a new file in `tests/` is otherwise
picked up automatically.

**H-3 escape hatch, you will need it.** `scripts/kernel-gates.sh` rejects bare
absolute float literals in predicates, and a test comparing floats trips it. The
opt-out is a `// H-3` comment **on the same line as the literal** — not the line
above, and rustfmt will move a trailing comment off a brace-opener line, so put
the literal on its own statement line first.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-stepio
cargo clippy -p truck-stepio --all-targets --no-deps -- -D warnings
cargo test -p truck-stepio --lib --test tolerance_stepio --no-fail-fast
cargo check --workspace --all-targets
```

`--no-deps` matters: without it clippy lints the vendored path dependencies too
and aborts before reaching your crate. Never run a bare `cargo test` — it builds
56 examples. Send cargo output to a file and read the tail.

**Confirm the baseline before you edit anything.** `truck-stepio` has
**pre-existing test failures** in this tree, including a `proptest` failure in
its geometry tests. Run the test command at the base commit first, record what
already fails, and report it. Do not fix any of it. If proptest writes a
`proptest-regressions/` file, leave it untracked and do not commit it.

## Forbidden

Editing any file outside `write_allow` — in particular
`scripts/unscaled_legacy_ceiling.txt`, `truck-base/src/tolerance.rs`,
`Cargo.toml`, and **`loop/` anything: your result file goes in the root of your
worktree and nowhere else.** Changing any function signature or `where` clause,
or adding any trait bound. Adding a `ctx` parameter. Changing any threshold.
Migrating the `#[cfg(test)]` assert. Widening a tolerance or adding `#[ignore]`
to make a test pass. Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- a site does not typecheck under its assigned recipe → `SPEC_GAP`, naming the
  site and the actual types. **Do not reclassify it to make it compile** — a
  `param` site that will not take `is_small_ratio` is telling you something.
- an existing test changes its result → **report it, do not fix it.** Stage A is
  supposed to move no threshold, so a moved test is evidence a classification in
  the table is wrong. Say which test and which site.
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

```json
{"id":"BG-TOL-001-STEPIO","status":"DONE","contracts":["BG-TOL-001"],
 "tests_added":2,"sites_migrated":19,"unscaled_legacy_calls":0,
 "anchors_verified":{"A1":7,"A2":5,"A3":2,"A4":1,"A5":5},
 "notes":"set unscaled_legacy_calls to the number you actually introduced. Report the baseline test failures you confirmed, and say explicitly whether you agree with the geom_impls.rs:72 param / :137 model split -- that pair is the one judgement in this packet worth arguing with."}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`refactor(stepio): classify every STEP import/export tolerance site model or param (BG-TOL-001-STEPIO)`.
