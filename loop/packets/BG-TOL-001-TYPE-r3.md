# WORK PACKET BG-TOL-001-TYPE-r3 — the two predicate shapes the shards cannot express

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```yaml
id:          BG-TOL-001-TYPE-r3
contract:    [BG-TOL-001]
class:       mechanical
crates:      [truck-base]
depends_on:  [BG-TOL-001-TYPE, BG-TOL-001-TYPE-r2]
write_allow:
  - vendor/truck/truck-base/src/tolerance.rs
  - vendor/truck/truck-base/tests/tolerance_ctx.rs
read_allow:
  - vendor/truck/truck-base/src/evidence.rs
  - vendor/truck/truck-base/src/bounding_box.rs
tests_required:
  - one_sided_margins_match_the_legacy_threshold
  - near_points_agrees_with_near_pt_on_point3
  - near_points_works_in_two_dimensions
budget:      {turns: 35, ctx_tokens: 90000}
```

**Three additions to an existing type, no behaviour changes.** Five migration
shards are blocked on them. As with r2, the doc comments are the deliverable as
much as the code: every later call site copies whatever is written here.

## Problem

`ToleranceCtx` has four predicates and they cover only one predicate *shape*:
the symmetric "are these two things within tolerance of each other". Writing the
`truck-topology` shard found two shapes the vendored tree actually uses that
cannot be expressed at all.

**One-sided threshold comparisons.** `truck-topology/src/edge.rs` reads

```rust
if t < t0 + TOLERANCE || t1 - TOLERANCE < t { return None; }
```

This asks "is `t` at or past the low end of the range", which is **not** the
same question as "is `t` near `t0`". Rewriting it as `is_small_ratio(t - t0)`
would silently change the answer for every `t` below `t0`: the original is true
there, the symmetric version is false, because `is_small_ratio` takes an
absolute value. There are **59 such one-sided comparisons across the vendored
tree**, so this is not an edge case — it is a large fraction of the remaining
199 migration sites.

**Generic point types.** `truck-topology` is generic over its point type
`P: Tolerance`, and `near_pt` takes a concrete `Point3`. Two sites in
`edge.rs` — the ones checking that an edge's curve endpoints agree with its
vertices, which is precisely what the BG-INV checkers are about — cannot be
migrated at all today.

## Anchors — verified 2026-08-16, counts are exact

Locate by running the `rg` command. **Never locate by line number.**
**If a count differs, STOP** and report `ANCHOR_MISMATCH` with what you saw.
Note that `rg` is not installed on this machine; any case-sensitive literal
search is equivalent for these patterns.

| # | file | pattern | expect |
|---|---|---|---|
| A1 | `vendor/truck/truck-base/src/tolerance.rs` | `pub fn near_pt` | **1** |
| A2 | `vendor/truck/truck-base/src/tolerance.rs` | `pub fn sin_margin` | **1** |
| A3 | `vendor/truck/truck-base/src/tolerance.rs` | `length_margin` | **0** |
| A4 | `vendor/truck/truck-base/src/tolerance.rs` | `pub fn unscaled_legacy` | **1** |
| A5 | `vendor/truck/truck-base/src/tolerance.rs` | `MetricSpace` | **0** |
| A6 | `vendor/truck/truck-base/tests/tolerance_ctx.rs` | `#\[test\]` | **8** |

## The design — all of it is decided; implement it, do not re-litigate

Added to the existing `impl ToleranceCtx` block. Change nothing that is already
there.

### 1. The two one-sided margins

```rust
/// MODEL-SPACE. The absolute margin a length comparison uses at this model's
/// scale: `tau_rep * model_scale`.
///
/// This exists for **one-sided** comparisons, which the symmetric predicates
/// cannot express. `a < b + ctx.length_margin()` asks whether `a` is at or
/// below `b` within tolerance; `is_small_len(a - b)` asks whether they are
/// close, and answers differently for every `a` far below `b`. Turning a
/// one-sided comparison into a symmetric one is a behaviour change disguised
/// as a migration.
pub fn length_margin(&self) -> f64

/// DIMENSIONLESS — deliberately NOT scaled. The one-sided counterpart of
/// [`Self::sin_margin`], named for what it bounds rather than for sines, since
/// most call sites comparing it are comparing curve parameters and knot values
/// rather than angles. Identical in value to `sin_margin`; both return
/// `tau_rep`.
pub fn ratio_margin(&self) -> f64
```

`length_margin` returns `self.tau_rep * self.model_scale`. `ratio_margin`
returns `self.tau_rep`. **Do not** deprecate or remove `sin_margin` — call
sites already reference it and this packet migrates nothing.

Yes, `ratio_margin` and `sin_margin` return the same number today. They are
kept separate because they are named for different quantities, and a later
packet that gives angles their own tolerance will change one and not the other.
Say that in the doc comment.

### 2. The generic point predicate

```rust
/// MODEL-SPACE, generic over the point type. True when `a` and `b` are within
/// representation tolerance, scaled by the model.
///
/// [`Self::near_pt`] is this specialised to `Point3` and is kept because it is
/// the common case and reads better at a call site. Generic code — the
/// topology crate is generic over its point type, and cannot name `Point3` —
/// uses this.
pub fn near_points<P>(&self, a: P, b: P) -> bool
where
    P: MetricSpace<Metric = f64>,
```

Implement as `a.distance(b) <= self.tau_rep * self.model_scale`, which is what
`near_pt` already computes for `Point3` — the two must agree exactly on
`Point3`, and one of your tests pins that.

`MetricSpace` comes from `cgmath`. `vendor/truck/truck-base/src/bounding_box.rs`
already uses that bound; read how it imports it and follow that, do not invent
an import path.

**Do not** rewrite `near_pt` to call `near_points`, and do not delete it. A
generic call that must monomorphise is not a free substitution for a concrete
one, and this packet is not the place to find that out.

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `unimplemented!`, `todo!`, or
  out-of-range indexing. None of these three functions is fallible.
- **H-2** Fallible operations return `Outcome<T>`. You are adding none.
- **H-3** No absolute constants in predicates. **`scripts/kernel-gates.sh` flags
  a bare float literal on any added line, and test epsilons trip it. The opt-out
  is a `// H-3` comment ON THE SAME LINE as the literal** — not on the line
  above, which does not work. Note that **rustfmt relocates a trailing comment
  off a line that ends in `{`**, which silently defeats the opt-out; if that
  happens, extract the literal onto its own statement line, as
  BG-TOL-001-TYPE-r2 did.
- **H-6** A value computed in floats is never recorded as `Method::Exact`.

## Tests required

Append to `vendor/truck/truck-base/tests/tolerance_ctx.rs`, which already holds
eight tests. Leave all eight exactly as they are.

1. `one_sided_margins_match_the_legacy_threshold` — on
   `ToleranceCtx::unscaled_legacy()`, `length_margin()` and `ratio_margin()`
   both equal `TOLERANCE`, and `t < t0 + ctx.ratio_margin()` reproduces
   `t < t0 + TOLERANCE` across a range of `t` **including values well below
   `t0`**, where the symmetric `is_small_ratio(t - t0)` gives the opposite
   answer. Assert that disagreement explicitly — it is the whole reason this
   function exists, and a test that only checks values near `t0` would pass
   against the bug.
2. `near_points_agrees_with_near_pt_on_point3` — over a fixed set of point
   pairs straddling the threshold, `near_points(a, b) == near_pt(a, b)` for
   every pair. Use a fixed seed if you generate any.
3. `near_points_works_in_two_dimensions` — `near_points` accepts `Point2` and
   scales with `model_scale` the same way. This is the case `near_pt` cannot
   serve and the reason the generic exists.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-base
cargo clippy -p truck-base --all-targets --no-deps -- -D warnings
cargo test -p truck-base --lib --test tolerance_ctx
cargo check --workspace --all-targets
```

`--no-deps` matters: without it clippy lints the vendored path dependencies too
and aborts before reaching your crate. Never run a bare `cargo test` — it builds
56 examples. Send cargo output to a file and read the tail.

## Forbidden

Editing any file outside `write_allow` — in particular do not touch
`scripts/`, `evidence.rs`, `bounding_box.rs` or `lib.rs`, do not migrate any
call site anywhere, and **do not write to `loop/`: your result file goes in the
root of your worktree and nowhere else.** Changing, deprecating or deleting any
existing item in `tolerance.rs`, including `sin_margin` and `near_pt`. Making
`model_scale` public or adding a setter. Adding `Default` for `ToleranceCtx`.
Adding `#[ignore]`. Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`
- `MetricSpace<Metric = f64>` will not admit both `Point2` and `Point3` as this
  packet assumes → `SPEC_GAP`, naming the compiler error. Do not add a second
  trait bound or a helper trait to work around it; that is a design decision
  and it is not yours to make.
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

```json
{"id":"BG-TOL-001-TYPE-r3","status":"DONE","contracts":["BG-TOL-001"],
 "tests_added":3,"anchors_verified":{"A1":1,"A2":1,"A3":0,"A4":1,"A5":0,"A6":8},
 "notes":"anything a reviewer should know, especially anything about the MetricSpace bound that surprised you"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(base): one-sided margins and a generic point predicate (BG-TOL-001-TYPE-r3)`.
