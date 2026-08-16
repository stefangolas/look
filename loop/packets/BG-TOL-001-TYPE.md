# WORK PACKET BG-TOL-001-TYPE — the scale-relative tolerance context

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```yaml
id:          BG-TOL-001-TYPE
contract:    [BG-TOL-001, BG-TOL-003]
class:       mechanical
crates:      [truck-base]
depends_on:  []
write_allow:
  - vendor/truck/truck-base/src/tolerance.rs
  - vendor/truck/truck-base/src/lib.rs
  - vendor/truck/truck-base/tests/tolerance_ctx.rs
read_allow:
  - vendor/truck/truck-base/src/evidence.rs
tests_required:
  - near_pt_scales_with_the_model
  - dimensionless_predicates_do_not_scale
  - scaled_context_preserves_every_predicate
  - entity_tolerance_never_below_boundary_tolerance
  - non_finite_or_non_positive_scale_is_refused
budget:      {turns: 40, ctx_tokens: 100000}
```

**This packet creates a type and nothing else.** It migrates no call sites. 184
call sites will be migrated onto it by later packets, one crate at a time, and
they will all copy whatever convention you establish here — so the doc comments
are part of the deliverable, not decoration.

## Problem

Tolerance in this kernel is currently a bare constant compared against a length:
`TOLERANCE`, `so_small()`, `.near()`, `.near2()`. That is wrong in two separate
ways, and they must not be conflated.

A comparison against a **model-space length** — a distance between points, a
chord height, a gap — is only meaningful relative to how big the model is. The
same 1e-6 that is generous on a 10mm bracket is meaningless on a 30m airframe.
Those comparisons must scale.

A comparison against a **dimensionless quantity** — a knot value, a normalized
parameter, a sine, a cosine, a NURBS weight — is already scale-free. Scaling it
is not a fix; it is a new bug.

This item introduces the type that lets every later call site state which of the
two it is, in a way a reader and a gate can both check.

## Anchors — verified 2026-08-16, counts are exact

Locate by running the `rg` command. **Never locate by line number.**
**If a count differs, STOP** and report `ANCHOR_MISMATCH` with what you saw.

| # | file | `rg` pattern | expect |
|---|---|---|---|
| A1 | `vendor/truck/truck-base/src/tolerance.rs` | `pub const TOLERANCE: f64` | **1** |
| A2 | `vendor/truck/truck-base/src/tolerance.rs` | `pub trait Tolerance` | **1** |
| A3 | `vendor/truck/truck-base/src/tolerance.rs` | `ToleranceCtx` | **0** |
| A4 | `vendor/truck/truck-base/src/evidence.rs` | `pub struct Budget` | **1** |

`tolerance.rs` already exists and holds the **legacy** API this item replaces:
`TOLERANCE`, the `Tolerance` trait (`near`/`near2`), the `Origin` trait
(`so_small`), and four assertion macros. **Leave every one of them exactly as
it is.** 465 sites across the tree still use them, and migrating those is later
packets' work, one crate at a time. You are adding the new type beside the old
one in the same module, not replacing anything.

## The design — all of it is decided; implement it, do not re-litigate

Added to the **existing** `vendor/truck/truck-base/src/tolerance.rs`, below the
legacy items and without disturbing them. Export it the way that module's
existing public items are exported — read `lib.rs`, do not guess.

```rust
/// The three tolerance budgets of the formal system, carried together with the
/// scale they are relative to.
///
/// `model_scale` is the declared characteristic length of the model. Every
/// **model-space** comparison in the kernel is `tau * model_scale`; every
/// **dimensionless** comparison is `tau` alone. Which one a call site needs is
/// a judgement the call site must state, never a default this type picks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToleranceCtx {
    model_scale: f64,
    /// Backward: the perturbation admitted by validation and repair.
    pub tau_in: f64,
    /// Representation error.
    pub tau_rep: f64,
    /// The collapse quotient.
    pub tau_col: f64,
}
```

`model_scale` is **private**, and there is no setter. It is reachable only
through the constructor and `scaled`, so no call site can quietly divide by it
and reintroduce an absolute comparison.

### Constructors

```rust
/// Refuses a `model_scale` that is not finite and strictly positive: a
/// zero, negative, or NaN scale makes every length predicate below meaningless,
/// and silently substituting 1.0 would make a wrong answer look like a right
/// one.
pub fn new(model_scale: f64, tau_in: f64, tau_rep: f64, tau_col: f64) -> Outcome<Self>

/// The same context at a different model scale (BG-TOL-002). The taus are
/// dimensionless ratios and therefore unchanged; only the scale moves.
pub fn scaled(&self, s: f64) -> Outcome<Self>

/// The declared characteristic length.
pub fn model_scale(&self) -> f64
```

Refuse with `Refusal::UnsupportedEnvelope(EnvelopeCase::ChartDegenerate)` — a
non-positive characteristic length is exactly a degenerate frame for the model.
The taus themselves must also be finite and non-negative; refuse the same way.

### The predicates, and the distinction that is the whole point

```rust
/// MODEL-SPACE. True when `a` and `b` are within representation tolerance,
/// scaled by the model: `|a - b| <= tau_rep * model_scale`.
pub fn near_pt(&self, a: Point3, b: Point3) -> bool

/// MODEL-SPACE. True when a length is negligible at this model's scale.
pub fn is_small_len(&self, l: f64) -> bool

/// DIMENSIONLESS — deliberately NOT scaled. A sine is a ratio; multiplying a
/// ratio by a length is a category error. Callers comparing angles, knot
/// values, normalized parameters or weights use this and nothing else.
pub fn sin_margin(&self) -> f64

/// DIMENSIONLESS. True when a ratio-valued quantity is within `sin_margin`.
pub fn is_small_ratio(&self, x: f64) -> bool
```

`sin_margin` returns `tau_rep` unscaled. Any comparison a caller can make with
these four is either explicitly model-space or explicitly dimensionless; there
is deliberately no fifth method that is ambiguous.

### BG-TOL-003, monotonicity

An entity's tolerance is never tighter than its boundary's. Give
`ToleranceCtx` a method that states this rather than leaving it to prose:

```rust
/// BG-TOL-003: an entity's tolerance may never be tighter than its boundary's.
/// Returns the entity tolerance to use given a boundary tolerance, which is the
/// larger of the two.
pub fn entity_tau(&self, boundary_tau: f64) -> f64
```

### The migration convention you are establishing

Write this into the module doc comment, because 184 later call sites will follow
it and they will follow whatever is written here:

> Every migrated site carries `// BG-TOL-001: model` or `// BG-TOL-001: param`,
> naming which kind of quantity it compares. A site whose kind is genuinely
> unclear gets `FIXME(BG-TOL-001)` and is reported, never guessed — guessing
> converts an obvious absolute-tolerance bug into a subtle scale bug.

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `unimplemented!`, `todo!`, or
  out-of-range indexing.
- **H-2** Fallible operations return `Outcome<T>` — never `Option`, never a
  bare `Result`. The constructors above are the fallible ones.
- **H-3** No absolute constants in predicates. This type is the mechanism by
  which that rule becomes satisfiable, so it must not itself contain one.
  **`scripts/kernel-gates.sh` flags a bare float literal on any added line, and
  test epsilons trip it. The opt-out is a `// H-3` comment ON THE SAME LINE as
  the literal** — not on the line above, which does not work. Use it on float
  comparison epsilons in your tests and say what the quantity is.
- **H-6** A value computed in floats is never recorded as `Method::Exact`.

## Tests required

Each must be a named `#[test]` fn — the verifier checks the names appear in your
diff, so a test you describe but do not write fails the gate.

1. `near_pt_scales_with_the_model` — two points a fixed distance apart are near
   at a large `model_scale` and not near at a small one. This is the property
   the type exists for.
2. `dimensionless_predicates_do_not_scale` — `sin_margin` and `is_small_ratio`
   return identical answers across several `model_scale` values. If this test
   ever fails, someone has scaled a ratio.
3. `scaled_context_preserves_every_predicate` — **BG-TOL-002 in miniature.**
   For random points and several scale factors `s`, `ctx.scaled(s)` applied to
   `s`-scaled points gives the same boolean as `ctx` on the originals, for both
   `near_pt` and `is_small_len`. Use a fixed seed.
4. `entity_tolerance_never_below_boundary_tolerance` — `entity_tau` is never
   less than either input, over a range of values including equal ones.
5. `non_finite_or_non_positive_scale_is_refused` — zero, negative, NaN and
   infinite `model_scale` each return a `Refusal`, not a panic and not a
   silently substituted default. Cover a bad tau as well.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-base
cargo clippy -p truck-base --all-targets --no-deps -- -D warnings
cargo test -p truck-base --lib --tests
cargo check --workspace --all-targets
```

`--no-deps` matters: without it clippy lints the vendored path dependencies too
and aborts before reaching your crate. Never run a bare `cargo test` — it builds
56 examples. Send cargo output to a file and read the tail.

## Forbidden

Editing any file outside `write_allow` — in particular **do not migrate any call
site**, do not change or delete any legacy item in `tolerance.rs`
(`TOLERANCE`, `Tolerance`, `Origin`, the macros), do not touch `evidence.rs`, and do not write to `loop/` (your result
file goes in the worktree root, nowhere else). Making `model_scale` public or
adding a setter for it. Adding a predicate that does not state model-space or
dimensionless in its name or doc comment. Adding `#[ignore]`. Committing to
`main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`
- a required test cannot be written without inventing a rule this packet does
  not state → `SPEC_GAP`, naming the readings you could not choose between
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

```json
{"id":"BG-TOL-001-TYPE","status":"DONE","contracts":["BG-TOL-001","BG-TOL-003"],
 "tests_added":5,"anchors_verified":{"A1":1,"A2":1,"A3":0,"A4":1},
 "notes":"anything a reviewer should know, especially any predicate you were tempted to add and did not"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(base): scale-relative tolerance context (BG-TOL-001-TYPE)`.
