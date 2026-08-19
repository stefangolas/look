# WORK PACKET BG-TOL-001-SMALL — Stage-A tolerance migration, truck-polymesh + truck-geotrait

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-TOL-001-SMALL","status":"DONE","contracts":["BG-TOL-001"],
 "tests_added":1,"sites_migrated":7,"sites_deferred":0,"unscaled_legacy_calls":7,
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
id:          BG-TOL-001-SMALL
contract:    [BG-TOL-001]
class:       wide-mechanical
crates:      [truck-polymesh, truck-geotrait]
depends_on:  [BG-TOL-001-TYPE, BG-TOL-001-TYPE-r2, BG-TOL-001-TYPE-r3]
write_allow:
  - vendor/truck/truck-geotrait/src/algo/curve.rs
  - vendor/truck/truck-geotrait/src/algo/surface.rs
  - vendor/truck/truck-polymesh/src/polyline_curve.rs
  - vendor/truck/truck-geotrait/tests/tolerance_small.rs
read_allow:
  - vendor/truck/truck-base/src/tolerance.rs
tests_required:
  - every_migrated_small_site_is_marked
budget:      {turns: 50, ctx_tokens: 120000}
census_fragment: truck-polymesh,truck-geotrait
unscaled_legacy_budget: 7
anchors:
  - {id: A1, expect: 2, cmd: "grep -cE '\\.near\\(|so_small\\(|TOLERANCE' vendor/truck/truck-geotrait/src/algo/curve.rs"}
  - {id: A2, expect: 3, cmd: "grep -cE '\\.near\\(|so_small\\(|TOLERANCE' vendor/truck/truck-geotrait/src/algo/surface.rs"}
  - {id: A3, expect: 5, cmd: "grep -cE '\\.near\\(|so_small\\(|TOLERANCE' vendor/truck/truck-polymesh/src/polyline_curve.rs"}
```

## Problem

This is the tail of the Stage-A migration: the seven remaining production
predicates in `truck-polymesh` and `truck-geotrait`, which are too few to be
worth a shard each. They are not incidental. `truck-geotrait::algo` holds the
generic Newton solvers that *every* curve and surface type in the kernel calls
to answer "is this point on this curve" — `search_parameter`,
`search_intersection_parameter` — and each one ends with an absolute-tolerance
acceptance test on a model-space distance. If those three sites are wrong about
scale, every geometric search in the kernel is wrong about scale, whatever the
concrete type.

`truck-polymesh::PolylineCurve` supplies the other four, and they are the
interesting half of the judgement: two of them are point-in-polygon tests in a
**parameter** frame that read exactly like model-space distances.

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
7 functions in this packet hold a migrated site, so you should introduce
exactly **7** `unscaled_legacy()` calls. See "The ratchet" — this number
is enforced by a gate, and it is a budget rather than an allowance.

**If you cannot reach 7 honestly, say so and stop.** That instruction is
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

4. **`algo/curve.rs:66` and `algo/surface.rs:292` look identical and take
   different rewrites. This is the whole judgement in this packet.** Both end a
   Newton solve with an acceptance test on a point-to-point distance, both are
   `model`, and the bounds differ:

   - `algo/surface.rs`'s `search_parameter<P, S>` bounds
     `P: EuclideanSpace<Scalar = f64> + MetricSpace<Metric = f64> + Tolerance`.
     `MetricSpace` is there, so `ctx.near_points(surface.subs(u, v), point)`
     applies.
   - `algo/curve.rs`'s `search_parameter<C>` bounds
     `C::Point: EuclideanSpace<Scalar = f64, Diff = C::Vector>` and
     `C::Vector: InnerSpace<Scalar = f64> + Tolerance`. There is **no**
     `MetricSpace`, so neither `ctx.near_points` nor `.distance()` compiles.
     `C::Vector: InnerSpace<Scalar = f64>` does give `.magnitude()`, so the form
     that compiles is
     `ctx.is_small_len((curve.subs(t) - point).magnitude())`.

   Same predicate, same tightening, different spelling. **Do not widen the bound
   on `search_parameter<C>` to make the tidier form work** — that is a public
   generic bound, it is cross-crate, and it is Stage B. The survey proposed
   `near_points` for both and was corrected here.

5. **`polyline_curve.rs:54` and `:132` are `param`, and dimensional analysis
   alone gets this backwards.** The quantity is `x = s2 / (s1 - s0)` where `s2`
   is a cross product of two displacements (degree 2) and `s1 - s0` involves the
   **unit** ray `r = (cos t, sin t)` (degree 1), so `x` is a *length* — it is the
   distance along the ray to where it crosses the boundary edge. What settles it
   is the frame, not the arithmetic: the enclosing impl is
   `impl PolylineCurve<Point2>`, and that `Point2` is a `uv` parameter point.
   A length measured in parameter coordinates does not scale with `model_scale`,
   so `is_small_ratio` is right. This matches the identical algorithm already
   accepted as `param` at `truck-meshalgo`'s `include_along_ray`. The survey
   marked both `confidence: low` and was right to; the question is answered and
   is not yours to reopen.

6. **`polyline_curve.rs:331`'s `h` is a vector and `.magnitude()` is available.**
   The enclosing impl bounds `P::Diff: InnerSpace<Scalar = f64> + Tolerance`, so
   `ctx.is_small_len(h.magnitude())` compiles. This has been checked.

7. **Nothing in this packet is deferred.** There is no `FIXME` to write. If you
   conclude a site needs one, that is a `SPEC_GAP` and you say so rather than
   writing a marker this packet did not ask for.

## The sites — 7 migrate, 7 contexts

Line numbers are provenance for a human reader; locate by the enclosing symbol.

**`curve.rs`**

| enclosing fn | line | code | class and why | write instead |
|---|---|---|---|---|
| `search_parameter` | 66 | `match curve.subs(t).to_vec().near(&point.to_vec()) {` | **`model`** — the scrutinee compares the curve position curve.subs(t) with the query point's position, so the predicate is the model-space distance between two points and it scales with the model **REVIEWED — orchestrator, session 9: class stands, REWRITE CORRECTED. The survey proposed ctx.near_points(curve.subs(t), point), which does not compile here: search_parameter<C> bounds C::Point with EuclideanSpace, not MetricSpace, so near_points does not apply. C::Vector is InnerSpace<Scalar = f64>, so the difference has .magnitude() and is_small_len is the form that compiles. Same predicate, same Euclidean tightening as everywhere else. Note algo/surface.rs :292 does carry MetricSpace<Metric = f64> and keeps near_points -- these two look identical and are not.** | `match ctx.is_small_len((curve.subs(t) - point).magnitude()) {` |

**`surface.rs`**

| enclosing fn | line | code | class and why | write instead |
|---|---|---|---|---|
| `search_parameter` | 292 | `\|Vector2 { x: u, y: v }\| match surface.subs(u, v).near(&point) {` | **`model`** — the scrutinee compares the surface position surface.subs(u, v) with the query point's position, so the predicate is the model-space distance between two points and it scales with the model | `\|Vector2 { x: u, y: v }\| match ctx.near_points(surface.subs(u, v), point) {` |
| `search_intersection_parameter` | 317 | `match surface.subs(x, y).near(&curve.subs(z)) {` | **`model`** — the scrutinee compares a surface point against a curve point, so the predicate is the model-space distance between two points and it scales with the model | `match ctx.near_points(surface.subs(x, y), curve.subs(z)) {` |

**`polyline_curve.rs`**

| enclosing fn | line | code | class and why | write instead |
|---|---|---|---|---|
| `include` | 54 | `if x.so_small() && s0 * s1 < 0.0 {` | **`param`** — x is the ray-crossing parameter s2/(s1-s0): the signed distance from the query point c to the boundary edge along the unit hash ray. Its dimension follows the caller's Point2 coordinate space, which this generic public API cannot know: in the tessellation context where this identical algorithm was reviewed (truck-meshalgo include_along_ray) the boundary lives in uv parameter space and x is dimensionless, but for a model-space polygon x scales linearly with the model and the predicate's absolute TOLERANCE window would change under a metre/mm export. Genuinely ambiguous; classified param to match the reviewed sibling. **REVIEWED — orchestrator, session 9: `param` CONFIRMED, and the reason is the frame, not the arithmetic. x = s2/(s1 - s0) is (area)/(length) = a LENGTH in the coordinates of the enclosing `impl PolylineCurve<Point2>` -- and that Point2 is a uv parameter point, not a model point, so the length does not scale with model_scale. Consistent with the ray-crossing parameter already accepted at triangulation.rs:8420. Dimensional reasoning alone gets this one backwards, which is why it came back confidence: low.** | `ctx.is_small_ratio(x) && s0 * s1 < 0.0` |
| `include` | 132 | `if x.so_small() && s0 * s1 < 0.0 {` | **`param`** — x is the ray-crossing parameter s2/(s1-s0): the signed distance from the query point c to the boundary edge along the unit hash ray. Its dimension follows the caller's Point2 coordinate space, which this generic public API cannot know: in the tessellation context where this identical algorithm was reviewed (truck-meshalgo include_along_ray) the boundary lives in uv parameter space and x is dimensionless, but for a model-space polygon x scales linearly with the model. Genuinely ambiguous; classified param to match the reviewed sibling. **REVIEWED — orchestrator, session 9: `param` CONFIRMED, and the reason is the frame, not the arithmetic. x = s2/(s1 - s0) is (area)/(length) = a LENGTH in the coordinates of the enclosing `impl PolylineCurve<Point2>` -- and that Point2 is a uv parameter point, not a model point, so the length does not scale with model_scale. Consistent with the ray-crossing parameter already accepted at triangulation.rs:8420. Dimensional reasoning alone gets this one backwards, which is why it came back confidence: low.** | `ctx.is_small_ratio(x) && s0 * s1 < 0.0` |
| `cut` | 302 | `if t.near(&(n as f64)) {` | **`param`** — t is the polyline's curve parameter, a dimensionless segment-index coordinate in [0, len] that does not change when the model is exported in a different unit; near() tests whether the cut parameter coincides with an integer knot, so the comparison is parameter space and must not scale. | `ctx.is_small_ratio(t - n as f64)` |
| `search_parameter` | 331 | `if h.so_small() {` | **`model`** — h = a - b*t where a = point - p[0] and b = p[1] - p[0] is the edge vector, so h is the perpendicular displacement from the query point to the segment and its magnitude is the shortest model-space distance from the point to the segment; the test decides whether the point lies on the segment, so it scales with the model. **REVIEWED — orchestrator, session 9: checked to compile. h: P::Diff is bounded InnerSpace<Scalar = f64> + Tolerance in the enclosing impl, so .magnitude() applies.** | `ctx.is_small_len(h.magnitude())` |

## Not in this packet — 9 excluded, no marker

- `polyline_curve.rs:426` — not code: inside #[test] fn polyline_test; a test's own epsilon is the test's business
- `stl.rs:277` — not a predicate: TOLERANCE is a spatial quantization bucket pitch (offset TOLERANCE * 0.25 into a grid of cell size TOLERANCE * 0.5) for deduplicating f32 positions and normals; the line computes a hash code and compares nothing
- `stl.rs:283` — not a predicate: the inverse of the quantization above, reconstructing a bucket centre as code * TOLERANCE * 0.5; a value computation that compares nothing
- `curve.rs:77` — not code: a doc comment; a documented precondition is prose, not a predicate
- `surface.rs:173` — squared order: near2 compares against the tighter TOLERANCE2 = 1e-12 token, which no ToleranceCtx predicate reproduces (mapping it onto tau_rep would loosen it by six orders of magnitude); deferred to BG-TOL-004
- `surface.rs:327` — not code: a doc comment; a documented precondition is prose, not a predicate
- `lib.rs:28` — not a predicate: the nonpositive_tolerance! macro (a value floor, the .max(TOLERANCE) family) asserts that a caller-supplied tolerance parameter is at least the absolute TOLERANCE constant; it is a precondition guard on an input value, compares no geometric quantity, and a const-free rewrite cannot compile in the macro body
- `curve.rs:279` — not code: a doc comment; a documented precondition is prose, not a predicate
- `surface.rs:254` — not code: a doc comment; a documented precondition is prose, not a predicate

<!-- 2 low-confidence row(s) above. Review each against the
     source before dispatching; that is the half V10 cannot check. -->

### Two live predicates that are NOT this packet's work, recorded so they are not rediscovered

The survey found two sites the site census structurally cannot see, at
`truck-geotrait/src/algo/curve.rs:134` and `algo/surface.rs:398`. Both compare a
squared distance against a squared tolerance — `dist2 < tol * tol` — with no
`TOLERANCE` token anywhere on the line, which is why no inventory contains them.
They are real first-order predicates and the survey was right to report them.

**They are still not migration work, and the reason is worth stating precisely:**
the epsilon in both is the **caller's runtime `tol` argument**, not the absolute
`TOLERANCE` constant. There is no absolute threshold to re-home, so there is
nothing for a `ToleranceCtx` to carry. Re-homing a caller-supplied chord
tolerance onto a threaded context is Stage-B work on the `parameter_division`
API. Leave both lines exactly as they are and add no marker.

### Everything else in these files

1. **All doc comments and `#[cfg(test)]` code**, including the four
   `/// \`tol\` must be greater than or equal to \`TOLERANCE\`` precondition
   comments and the assertion inside `polyline_test`.
2. **`stl.rs` entirely.** Its two `TOLERANCE` uses are a spatial-hash
   quantization pitch and its inverse — a value computation that compares
   nothing. `stl.rs` is not on your allowlist.
3. **`truck-geotrait/src/lib.rs:28`,** the `nonpositive_tolerance!` macro. It
   asserts a caller-supplied tolerance is at least `TOLERANCE`; that is a
   precondition check on an argument, not a geometric predicate.
4. **`surface.rs:173`, `if next.near2(&param)`.** Squared order against
   `TOLERANCE2`; deferred to BG-TOL-004.

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
it can push a sibling's correct work over the line. Introduce 7 and no
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

New file `vendor/truck/truck-geotrait/tests/tolerance_small.rs`.

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

1. `every_migrated_small_site_is_marked` — read the migrated source files from
   `CARGO_MANIFEST_DIR` at runtime and assert that the number of lines
   containing `ctx.near_pt(`, `ctx.near_points(`, `ctx.is_small_len(`,
   `ctx.is_small_ratio(` or `ctx.ratio_margin()` equals the number containing a
   `// BG-TOL-001:` marker. This is what makes the marking checkable rather than
   a convention; without it the markers rot the first time someone edits a line.


**The crate hosting the test file is `truck-geotrait`, and that is a decision, not
an accident.** `truck-polymesh/Cargo.toml` sets **`autotests = false`** — a new test file there silently never runs, and adding the `[[test]]` entry it would need means editing a `Cargo.toml` that is not on your allowlist. `truck-geotrait` has no such setting. Read `polyline_curve.rs` from there at runtime as `concat!(env!("CARGO_MANIFEST_DIR"), "/../truck-polymesh/src/polyline_curve.rs")`. If you find yourself wanting a test file under `truck-polymesh`, that is the trap this paragraph exists to close.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-polymesh -p truck-geotrait
cargo clippy -p truck-polymesh -p truck-geotrait --all-targets --no-deps -- -D warnings
cargo test -p truck-geotrait --lib --test tolerance_small --no-fail-fast
cargo test -p truck-polymesh --lib --no-fail-fast
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
`refactor(polymesh,geotrait): classify every tolerance site model or param (BG-TOL-001-SMALL)`.
