# WORK PACKET BG-TOL-001-MESHALGO — Stage-A tolerance migration, truck-meshalgo

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```yaml
id:          BG-TOL-001-MESHALGO
contract:    [BG-TOL-001]
class:       wide-mechanical
crates:      [truck-meshalgo]
depends_on:  [BG-TOL-001-TYPE, BG-TOL-001-TYPE-r2, BG-TOL-001-TYPE-r3]
write_allow:
  - vendor/truck/truck-meshalgo/src/analyzers/collision.rs
  - vendor/truck/truck-meshalgo/src/analyzers/in_out_judge.rs
  - vendor/truck/truck-meshalgo/src/analyzers/point_cloud/mod.rs
  - vendor/truck/truck-meshalgo/src/analyzers/point_cloud/sort_end_points.rs
  - vendor/truck/truck-meshalgo/src/filters/normal_filters.rs
  - vendor/truck/truck-meshalgo/src/tessellation/source_edge.rs
  - vendor/truck/truck-meshalgo/src/tessellation/triangulation.rs
  - vendor/truck/truck-meshalgo/tests/tolerance_meshalgo.rs
read_allow:
  - vendor/truck/truck-base/src/tolerance.rs
tests_required:
  - every_migrated_meshalgo_site_is_marked
  - deferred_area_sites_carry_a_fixme
budget:      {turns: 70, ctx_tokens: 150000}
census_fragment: truck-meshalgo
unscaled_legacy_budget: 11
anchors:
  - {id: A1, expect: 6, cmd: "grep -cE '\\.near\\(|so_small\\(|TOLERANCE' vendor/truck/truck-meshalgo/src/analyzers/collision.rs"}
  - {id: A2, expect: 1, cmd: "grep -cE '\\.near\\(|so_small\\(|TOLERANCE' vendor/truck/truck-meshalgo/src/analyzers/in_out_judge.rs"}
  - {id: A3, expect: 2, cmd: "grep -cE '\\.near\\(|so_small\\(|TOLERANCE' vendor/truck/truck-meshalgo/src/analyzers/point_cloud/mod.rs"}
  - {id: A4, expect: 1, cmd: "grep -cE '\\.near\\(|so_small\\(|TOLERANCE' vendor/truck/truck-meshalgo/src/analyzers/point_cloud/sort_end_points.rs"}
  - {id: A5, expect: 8, cmd: "grep -cE '\\.near\\(|so_small\\(|TOLERANCE' vendor/truck/truck-meshalgo/src/filters/normal_filters.rs"}
  - {id: A6, expect: 43, cmd: "grep -cE '\\.near\\(|so_small\\(|TOLERANCE' vendor/truck/truck-meshalgo/src/tessellation/source_edge.rs"}
  - {id: A7, expect: 41, cmd: "grep -cE '\\.near\\(|so_small\\(|TOLERANCE' vendor/truck/truck-meshalgo/src/tessellation/triangulation.rs"}
```

**This is a churn packet.** Every judgement has been made for you and is in the
table below. Your job is to apply exactly those 20 rewrites, add exactly 6 FIXME
comments, mark them, and keep the crate building and its tests passing. If you
find yourself deciding whether a site is `model` or `param`, stop and re-read
the table — it is already there. This packet's classifications were produced by
a survey pass and then reviewed and corrected site by site; four of them were
changed during that review, so do not assume a row is a guess.

## Problem

Tolerance in this crate is a bare absolute constant: `TOLERANCE` (1e-6),
`.near()`, `.so_small()`. A comparison against a **model-space length** — a
distance between points, a radius, a surface derivative magnitude — is only
meaningful relative to how big the model is; the same 1e-6 that is generous on a
10mm bracket is meaningless on a 30m airframe. A comparison against a
**dimensionless** quantity — a `uv` coordinate, a curve parameter, a unit
normal's magnitude, a ratio — is already scale-free, and scaling it would be a
new bug.

`truck-meshalgo` is where this matters most immediately, because it is the crate
that turns a B-rep into triangles. A `param` predicate mistakenly scaled would
change how a boundary loop is cut in `uv`; a `model` predicate left unscaled is
why a large part tessellates differently from a small one. Today the code does
not record which is which, and that judgement cannot be recovered mechanically
later. This packet records it, for `truck-meshalgo`, without changing any
threshold.

**Stage A, which is all this packet is.** Each site is rewritten through a
`ToleranceCtx` obtained from `ToleranceCtx::unscaled_legacy()`, which carries
`model_scale = 1.0` and `tau_rep = TOLERANCE`. **No threshold moves and no
signature changes.** A later Stage-B packet derives a real `model_scale` at the
crate's entry points and threads it inward, deleting the `unscaled_legacy()`
calls. That is what actually fixes the scale bug; this packet buys the
judgement, which is the expensive half.

## Anchors — verified 2026-08-18, counts are exact

Locate by running the `grep` command. **Never locate by line number** — the line
numbers in the table below are provenance for a human reader, not a way to find
anything. `rg` is not installed on this machine; use `grep -cE` exactly as
written in the `anchors:` block above.

If any count differs from the `expect:` value, the tree has moved since this
packet was written. That is `ANCHOR_MISMATCH` and you stop — it is a stop
condition, not a nuisance, because a packet whose counts are stale is a packet
whose table may point at the wrong code.

Note that these counts cover **every** occurrence in each file, including doc
comments and in-src tests. Only the 20 rows in the site table migrate. The
anchor is a fingerprint of the file, not a work list.

## The recipes — the only four rewrites you will make

| class | shape of the quantity | rewrite |
|---|---|---|
| `model` | a length, against zero | `ctx.is_small_len(l)` |
| `model` | two `Point3` | `ctx.near_pt(a, b)` |
| `param` | a dimensionless value against zero, or a difference | `ctx.is_small_ratio(x)` |
| `param` | a one-sided margin on a parameter | `ctx.ratio_margin()` |

Obtain the context once per function, as the first statement:

```rust
let ctx = ToleranceCtx::unscaled_legacy();
```

Mark every rewritten line with a trailing `// BG-TOL-001: model` or
`// BG-TOL-001: param` comment. Where the line is a multi-line expression, put
the marker on the line carrying the `ctx.` call.

**One context per function, never one per site.** Eleven functions in this
packet hold a migrated site; you should introduce exactly **11**
`unscaled_legacy()` calls. See "The ratchet" below — this number is enforced.

## Decisions already made for you

Read these before the table. Each one is a judgement that has been made, checked
against the tree, and is not yours to revisit.

1. **`.near()` is componentwise; `ctx.near_pt` and `ctx.is_small_ratio` are
   Euclidean.** Not the same predicate — Euclidean is stricter by up to
   `sqrt(3)`. Every Stage-A shard is therefore a small deliberate tightening. If
   an existing test moves because of it, **report it in your notes with the test
   name and the reason**; do not widen a tolerance and do not put a site back to
   componentwise to make it pass. A test that moves is a finding, not a bug in
   this packet.

2. **`so_small()` on a vector becomes `is_small_len(v.magnitude())`, not
   `is_small_len(v.x)`.** Same tightening as above, same rule about tests.

3. **The four `reconcile_singular_transition` lines carry TWO predicates each,
   of two different classes, and both must be migrated.** This is the one place
   in this packet where a naive reading loses behaviour. Each line reads

   ```rust
   if !previous_uv.x.near(&current_uv.x) && surface.uder(current_uv.x, current_uv.y).so_small() {
   ```

   The `!near` guard compares **u parameters** and is `param`. The `so_small`
   compares a **surface derivative magnitude** and is `model`. Migrating only
   the `so_small` half deletes the guard and changes what the function does.
   Write both. The exact replacements, in order, are:

   | line | replacement condition |
   |---|---|
   | 4517 | `!ctx.is_small_ratio(previous_uv.x - current_uv.x) && ctx.is_small_len(surface.uder(current_uv.x, current_uv.y).magnitude())` |
   | 4523 | `!ctx.is_small_ratio(previous_uv.y - current_uv.y) && ctx.is_small_len(surface.vder(current_uv.x, current_uv.y).magnitude())` |
   | 4529 | `!ctx.is_small_ratio(previous_uv.x - current_uv.x) && ctx.is_small_len(surface.uder(previous_uv.x, previous_uv.y).magnitude())` |
   | 4535 | `!ctx.is_small_ratio(previous_uv.y - current_uv.y) && ctx.is_small_len(surface.vder(previous_uv.x, previous_uv.y).magnitude())` |

   These four lines are **one function and one context**, and the marker on a
   mixed line is `// BG-TOL-001: param+model`.

4. **`try_new:5523`, `if !vec[0].near(&last)`, is `param` and compares `uv`
   only.** `SurfacePoint` has no `AbsDiffEq` impl of its own and `#[deref]`s to
   its `uv: Point2` field, so this `near` already resolves to `Point2::near` and
   never looks at the `point: Point3`. Write the deref out explicitly:
   `!ctx.is_small_ratio(vec[0].uv.distance(last.uv))`. Making the deref visible
   is deliberate — the next reader should not have to know about the attribute
   to know what is being compared.

5. **`on_boundary:8398` becomes a first-order comparison and gains a `sqrt`.**
   The line is `(a + ab * t).distance2(c) <= TOLERANCE * TOLERANCE`, which is
   algebraically `distance <= TOLERANCE` written squared to avoid the square
   root. `ToleranceCtx` has no squared-order predicate, so write
   `ctx.is_small_ratio((a + ab * t).distance(c))`. **This is not the
   `near2`/`so_small2` family** — those compare a value against the *tighter*
   `TOLERANCE2` = 1e-12 constant and genuinely cannot be migrated; this one
   compares against `TOLERANCE` squared and can. The `sqrt` per boundary segment
   is accepted here as the price of a checkable classification; do not
   micro-optimise it back and do not skip the site because of it.

6. **`is_small_ratio` is the right predicate for a unit-normal degeneracy
   test.** `triangulation_into_polymesh_outcome:10898` asks whether a normal
   that has already been normalized is zero. A unit vector's magnitude is
   dimensionless. This is `param` even though the vector lives in model space —
   the classification is about the **quantity being compared**, not about where
   the vector points.

7. **A `const` item is never a site.** `source_edge.rs:111` is
   `pub const SOURCE_INCIDENCE_TOLERANCE: f64 = TOLERANCE;`. A `const`
   initializer has no `ctx` to call, so there is nothing to migrate; re-homing
   that value onto a threaded context is Stage-B work. Its two *consumers* are
   the predicates, and one of them (`source_edge.rs:311`) is in your table.
   Leave line 111 exactly as it is.

8. **Six sites compare an AREA and are deferred, not migrated.** See "Not in
   this packet" below. They get a `FIXME` comment and no rewrite. Migrating one
   of them is a rejection.

## The sites — 20 migrate, 11 contexts

Line numbers are provenance for a human reader; locate by the enclosing symbol.

**`normal_filters.rs`**

| enclosing fn | line | code | class |
|---|---|---|---|
| `normalize_normals` | 200 | `if !normals[idx].magnitude2().near(&1.0) {` | **`param`** — compares the squared magnitude of an already-normalized unit normal against 1.0; a unit vector's length is dimensionless and does not scale with the model |

**`source_edge.rs`**

| enclosing fn | line | code | class |
|---|---|---|---|
| `establish_source_edge_traversal` | 311 | `let carrier_closed = subs_lo.distance(subs_hi) <= SOURCE_INCIDENCE_TOLERANCE;` | **`model`** — it compares the distance between two curve points (a model-space length) against the source-incidence length tolerance to decide whether the carrier closes at the evaluator seam, so it scales with the model |

**`triangulation.rs`**

| enclosing fn | line | code | class |
|---|---|---|---|
| `reconcile_singular_transition` | 4517 | `if !previous_uv.x.near(&current_uv.x) && surface.uder(current_uv.x, current_uv.y).so_small() {` | **`param+model`** — see decision 3; both predicates migrate |
| `reconcile_singular_transition` | 4523 | `if !previous_uv.y.near(&current_uv.y) && surface.vder(current_uv.x, current_uv.y).so_small() {` | **`param+model`** — see decision 3; both predicates migrate |
| `reconcile_singular_transition` | 4529 | `if !previous_uv.x.near(&current_uv.x) && surface.uder(previous_uv.x, previous_uv.y).so_small() {` | **`param+model`** — see decision 3; both predicates migrate |
| `reconcile_singular_transition` | 4535 | `if !previous_uv.y.near(&current_uv.y) && surface.vder(previous_uv.x, previous_uv.y).so_small() {` | **`param+model`** — see decision 3; both predicates migrate |
| `try_new` | 5523 | `if !vec[0].near(&last) {` | **`param`** — `SurfacePoint` has no `AbsDiffEq` impl and derefs to `uv: Point2`, so this `near` resolves to `Point2::near` and compares the first and last lifted boundary points' uv parameters, not their positions. See decision 4 |
| `try_new` | 5525 | `if surface.uder(u0, v0).so_small() \|\| surface.vder(u0, v0).so_small() {` | **`model`** — the surface u- and v-derivative magnitudes are model-space lengths (length per dimensionless parameter) and the test decides whether the loop closes on a collapsed direction. **Two** predicates on this line, both `model` |
| `singular_transition_branch` | 6379 | `if vp.is_some() && surface.vder(u, v).so_small() {` | **`model`** — the v-derivative magnitude is a model-space length, and the test detects a collapsed periodic axis at a chart singularity |
| `singular_transition_branch` | 6381 | `} else if up.is_some() && surface.uder(u, v).so_small() {` | **`model`** — the u-derivative magnitude is a model-space length, and the test detects a collapsed periodic axis at a chart singularity |
| `working_range` | 7220 | `(hi - lo > TOLERANCE).then_some((lo, hi))` | **`param`** — `hi - lo` is the span of the boundary's u (or v) parameter coordinate, so the comparison asks whether the parameter interval is non-degenerate and a parameter range is dimensionless. One-sided: use `!ctx.is_small_ratio(hi - lo)` |
| `new_with_join` | 8204 | `if p.x < q.x - TOLERANCE {` | **`param`** — `p.x`, `q.x` are u parameters of the open boundary's endpoints; a one-sided margin, so use `q.x - ctx.ratio_margin()` |
| `new_with_join` | 8220 | `} else if q.x < p.x - TOLERANCE {` | **`param`** — same, mirrored |
| `new_with_join` | 8236 | `} else if p.y < q.y - TOLERANCE {` | **`param`** — same, on v |
| `new_with_join` | 8252 | `} else if q.y < p.y - TOLERANCE {` | **`param`** — same, mirrored, on v |
| `end_pts` | 8278 | `if !p0.x.near(&p1.x) && !q0.x.near(&q1.x) {` | **`param`** — u parameters of the two open curves' endpoints; asks whether both curves need the same u-range normalization. **Two** predicates on this line, both `param` |
| `end_pts` | 8283 | `} else if !p0.y.near(&p1.y) && !q0.y.near(&q1.y) {` | **`param`** — same, on v. **Two** predicates on this line, both `param` |
| `on_boundary` | 8398 | `(a + ab * t).distance2(c) <= TOLERANCE * TOLERANCE` | **`param`** — parameter-space point-to-segment distance: `c` and the boundary loop points are `Point2` in uv. First-order, not squared-order. See decision 5 |
| `include_along_ray` | 8420 | `if x.so_small() && s0 * s1 < 0.0 {` | **`param`** — `x` is `s2 / (s1 - s0)`, a ratio of parameter-space cross products locating where the ray crosses the boundary edge, so it is dimensionless. Migrate only the `so_small`; `s0 * s1 < 0.0` is a sign test, not a tolerance |
| `triangulation_into_polymesh_outcome` | 10898 | `if norm.so_small() \|\| !norm.x.is_finite() {` | **`param`** — `norm` is an already-normalized vertex normal, so its magnitude is dimensionless. See decision 6. Migrate only the `so_small`; `is_finite` is not a tolerance |

## Not in this packet — the deferred and the excluded

### The six deferred area sites — add a FIXME, change nothing else

These compare a quantity that is **degree 2 in length** — a cross-product
magnitude (twice a triangle's area) or a 3×3 determinant of two model-space
displacements and a unit direction. Under a model rescale by `k` such a quantity
scales as `k²` while `ctx.length_margin()` scales as `k`. Neither `model` nor
`param` fits, and `is_small_len` applied to an area is a migration that *looks*
correct and is wrong the moment Stage B threads a real `model_scale` — worse
than no migration, because Stage B would then see a migrated site and not look
again. `ToleranceCtx` has no area predicate; adding one is out of scope here.

This is the same treatment a previous shard already gave the identical problem
at `truck-modeling/src/geom_impls.rs:91`. Copy that form exactly: the original
line untouched, and immediately above it

```rust
// FIXME(BG-TOL-001): <quantity> is an area (length squared); neither predicate fits
```

| file | line | code |
|---|---|---|
| `analyzers/collision.rs` | 87 | `.filter(\|(_, tri)\| !(tri[1] - tri[0]).cross(tri[2] - tri[0]).so_small())` |
| `analyzers/collision.rs` | 93 | `.filter(\|(_, tri)\| !(tri[1] - tri[0]).cross(tri[2] - tri[0]).so_small())` |
| `analyzers/collision.rs` | 153 | `if nor.so_small() {` |
| `analyzers/in_out_judge.rs` | 23 | `if mat.determinant().so_small() {` |
| `analyzers/point_cloud/mod.rs` | 76 | `if coef < 2.0 \|\| nor.magnitude().so_small() {` |
| `analyzers/point_cloud/sort_end_points.rs` | 55 | `.filter(\|(_, tri)\| !(tri[1] - tri[0]).cross(tri[2] - tri[0]).so_small())` |

These four files are on your allowlist **only** so you can add those comments.
**Introduce no `ToleranceCtx` in any of them** — a context in a function with no
migrated site is an unused binding, a clippy failure, and a wasted slot against
the ratchet.

### Everything else

Leaving these alone is correct; migrating one is a rejection.

1. **All doc comments and `#[cfg(test)]` code.** A doc example is prose and a
   test's epsilon is the test's own business. This is most of the 43 hits in
   `source_edge.rs` and the 8 in `normal_filters.rs`.
2. **Anything using `.near2()` or `.so_small2()`,** including
   `collision.rs:164`, `:165`, `:166`. These compare against `TOLERANCE2` =
   1e-12 and `ToleranceCtx` has no squared-order predicate — mapping them onto
   `tau_rep` would loosen them by six orders of magnitude while appearing to
   migrate them. Leave them; they already have their own deferral.
3. **`triangulation.rs:6910`,** `let tmp = f64::min(p[compidx], q[compidx]) + TOLERANCE;`
   — a range-cut offset in parameter space. It compares nothing.
4. **`triangulation.rs:11072`,** `let tol = tol.max(TOLERANCE);` — floors a
   chord tolerance. It compares nothing. Its value is a model-space length and
   re-homing it is Stage B.
5. **`vtk.rs` entirely** — the `use` import at line 4 and the spatial-hash
   bucket pitch at line 195, `(p / (TOLERANCE * 50.0) - ...)`. `TOLERANCE` is a
   grid quantization pitch there, not a predicate. Not on your allowlist.
6. **`source_edge.rs:111` and `:252`.** Line 111 is the `const` item (decision
   7); line 252 is `source_tolerance.max(SOURCE_INCIDENCE_TOLERANCE)`, a value
   computation that compares nothing.
7. **`tessellation/formal/*.rs`** and its `RELATIVE_TOLERANCE` constant. Those
   files are not on your allowlist and are not part of this contract item.

## The ratchet — read this before you commit

`scripts/kernel-gates.sh` counts `unscaled_legacy(` call sites in
`vendor/truck/*/src/**` and **fails when the total exceeds the ceiling** in
`scripts/unscaled_legacy_ceiling.txt`. The ceiling has been raised to **51** for
this packet, covering the 40 already in the tree plus your 11. That file is
**not** on your allowlist and you must not edit it — the ceiling exists to
constrain this packet, and a packet that can move its own ceiling is not
constrained by anything.

Eleven is a budget and it is exact, not an allowance. One context per function
containing a migrated site is eleven; if you need more, you have constructed one
per site instead of one per function, which is the mistake this number is sized
to catch.

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `unimplemented!`, `todo!`, or
  out-of-range indexing on any line you add. `unscaled_legacy()` is infallible
  and returns `Self`, so there is nothing to unwrap.
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

New file `vendor/truck/truck-meshalgo/tests/tolerance_meshalgo.rs`.

**Its first line must be `#![deny(clippy::unwrap_used)]`.** GATE-1 (H-1)
requires it of every new module under `vendor/truck/`, including test files, and
`scripts/kernel-gates.sh` fails the packet without it. Both landed shards carry
it -- see `truck-shapeops/tests/tolerance_migration.rs`. Write your tests so the
attribute costs nothing: return `Result` or match rather than `unwrap`.

Each test must be a named `#[test]` fn — the verifier checks the names appear in your diff.

1. `every_migrated_meshalgo_site_is_marked` — read the three migrated source
   files (`filters/normal_filters.rs`, `tessellation/source_edge.rs`,
   `tessellation/triangulation.rs`) from `CARGO_MANIFEST_DIR` at runtime and
   assert that the number of lines containing `ctx.near_pt(`,
   `ctx.is_small_len(`, `ctx.is_small_ratio(` or `ctx.ratio_margin()` equals the
   number containing a `// BG-TOL-001:` marker. This is the test that makes the
   marking checkable rather than a convention; without it the markers rot the
   first time someone edits a line.
2. `deferred_area_sites_carry_a_fixme` — read the four deferred files
   (`analyzers/collision.rs`, `analyzers/in_out_judge.rs`,
   `analyzers/point_cloud/mod.rs`, `analyzers/point_cloud/sort_end_points.rs`)
   and assert each contains exactly the expected number of
   `FIXME(BG-TOL-001)` lines — 3, 1, 1, 1 — and **no** `ToleranceCtx` at all.
   The second half is the load-bearing one: it is what stops a later reader from
   "finishing the job" by migrating an area site, and what proves those four
   files cost nothing against the ratchet.

`truck-meshalgo` — **check `Cargo.toml` for `autotests = false` before you
rely on the file being picked up.** `truck-polymesh` sets it and a new test file
there silently never runs; if `truck-meshalgo` does the same you must add an
explicit `[[test]]` entry, and `Cargo.toml` is then part of your write set —
report that as a SPEC_GAP rather than editing a file this packet did not
allowlist.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-meshalgo
cargo clippy -p truck-meshalgo --all-targets --no-deps -- -D warnings
cargo test -p truck-meshalgo --lib --test tolerance_meshalgo --no-fail-fast
cargo check --workspace --all-targets
```

`--no-deps` matters: without it clippy lints the vendored path dependencies too
and aborts before reaching your crate. Never run a bare `cargo test` — it builds
56 examples. Send cargo output to a file and read the tail.

**This crate is not clippy-clean at the base commit** — around 93 pre-existing
lints. They are not yours. The verifier scopes clippy to the lines your diff
adds; you are responsible for those and nothing else. If a pre-existing test
fails, **confirm it fails identically at the base commit and say so in your
notes** — do not try to fix it and do not let it stop you.

## Forbidden

Editing any file outside `write_allow` — in particular
`scripts/unscaled_legacy_ceiling.txt`, `scripts/kernel-gates.sh`,
`truck-base/src/tolerance.rs`, `src/vtk.rs`, `tessellation/formal/**`, and
**`loop/` anything: your result file goes in the root of your worktree and
nowhere else.** Changing any function signature. Adding a `ctx` parameter.
Changing any threshold. Introducing a `ToleranceCtx` in any of the four deferred
files. Migrating a site the "Not in this packet" section excludes. Widening a
tolerance or adding `#[ignore]` to make a test pass. Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- a site in the table does not typecheck under its assigned recipe → `SPEC_GAP`,
  naming the site and the actual types. **Do not reclassify it to make it
  compile**; a `model` site that will not take `is_small_len` is telling you
  something, and reporting that is worth more than a green build.
- `truck-meshalgo/Cargo.toml` sets `autotests = false` → `SPEC_GAP`
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

```json
{"id":"BG-TOL-001-MESHALGO","status":"DONE","contracts":["BG-TOL-001"],
 "tests_added":2,"sites_migrated":20,"sites_deferred":6,"unscaled_legacy_calls":11,
 "anchors_verified":{"A1":6,"A2":1,"A3":2,"A4":1,"A5":8,"A6":43,"A7":41},
 "notes":"set unscaled_legacy_calls to the number you actually introduced. Report every existing test whose behaviour moved, with the name and the reason, and anything the Euclidean/componentwise difference changed. If you disagree with a classification in the table, say so here with your reasoning rather than changing it."}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`refactor(meshalgo): classify every tolerance site model or param (BG-TOL-001-MESHALGO)`.
