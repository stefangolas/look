# WORK PACKET BG-TOL-001-TYPE-r2 — the migration scaffold the shards need

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```yaml
id:          BG-TOL-001-TYPE-r2
contract:    [BG-TOL-001]
class:       mechanical
crates:      [truck-base]
depends_on:  [BG-TOL-001-TYPE]
write_allow:
  - vendor/truck/truck-base/src/tolerance.rs
  - vendor/truck/truck-base/tests/tolerance_ctx.rs
read_allow:
  - vendor/truck/truck-base/src/evidence.rs
  - vendor/truck/truck-base/src/lib.rs
tests_required:
  - unscaled_legacy_carries_the_legacy_epsilon
  - unscaled_legacy_is_never_looser_than_the_legacy_predicate
  - unscaled_legacy_agrees_with_new_at_scale_one
budget:      {turns: 30, ctx_tokens: 80000}
```

**This packet adds one constructor and three tests.** It migrates no call sites
and changes no existing item. It is small on purpose: seven later shards depend
on the exact semantics you write here, so the doc comments are the deliverable
as much as the code is.

## Problem

`ToleranceCtx` landed in `BG-TOL-001-TYPE` with two constructors, `new` and
`scaled`, both of which demand a `model_scale` the caller must already know.

The next seven packets migrate ~184 legacy tolerance sites onto that type, one
crate at a time. Every one of those sites is inside a function that has no
`ToleranceCtx` and no model scale — the value is not in scope, not in the
signature, and not reachable without changing public signatures in every crate
simultaneously. So the migration is staged, and this packet builds the thing
Stage A stands on:

- **Stage A** (the seven shards) rewrites each site through a context obtained
  from `ToleranceCtx::unscaled_legacy()` and marks it `model` or `param`. It
  changes no signature and, at `model_scale = 1.0`, changes no threshold. What
  it buys is the model/param judgement, made once, in writing.
- **Stage B** (later, per entry point) derives a real `model_scale` from the
  input and threads it inward, deleting `unscaled_legacy()` calls as it goes.

`unscaled_legacy` is a scaffold, and a scaffold nobody removes is a permanent
absolute tolerance — exactly the bug BG-TOL-001 exists to kill. It is therefore
ratcheted by `scripts/kernel-gates.sh` against a recorded ceiling that only ever
moves down. **You do not write that gate** — it is already in place and it is
outside your allowlist. It matters to you only because it means the doc comment
saying "scaffold, expected to reach zero" is load-bearing and must be there.

## Anchors — verified 2026-08-16, counts are exact

Locate by running the `rg` command. **Never locate by line number.**
**If a count differs, STOP** and report `ANCHOR_MISMATCH` with what you saw.

| # | file | `rg` pattern | expect |
|---|---|---|---|
| A1 | `vendor/truck/truck-base/src/tolerance.rs` | `pub fn new\(model_scale` | **1** |
| A2 | `vendor/truck/truck-base/src/tolerance.rs` | `unscaled_legacy` | **0** |
| A3 | `vendor/truck/truck-base/src/tolerance.rs` | `pub const TOLERANCE: f64` | **1** |
| A4 | `vendor/truck/truck-base/src/tolerance.rs` | `fn near\(&self, other: &Self\)` | **1** |
| A5 | `vendor/truck/truck-base/tests/tolerance_ctx.rs` | `#\[test\]` | **5** |

## The design — all of it is decided; implement it, do not re-litigate

### The constructor

Added to the existing `impl ToleranceCtx` block in
`vendor/truck/truck-base/src/tolerance.rs`, below `scaled`.

```rust
/// The migration scaffold for BG-TOL-001 Stage A: a context whose predicates
/// are numerically the legacy absolute ones.
///
/// `model_scale` is 1.0 and `tau_rep` is [`TOLERANCE`], so `is_small_len` and
/// `is_small_ratio` use exactly the epsilon the legacy `Tolerance` trait used.
/// A site migrated onto this context therefore keeps its present behaviour;
/// what the migration buys is that the site now *states* whether it compares a
/// model-space length or a dimensionless quantity, which is the judgement that
/// cannot be made mechanically later.
///
/// **This is scaffolding and is expected to reach zero uses.** A real
/// `model_scale` comes from the model, and every call here is a site whose
/// entry point has not yet been threaded (Stage B). `scripts/kernel-gates.sh`
/// counts these against a ceiling that only moves down; BG-TOL-001 is not
/// discharged until the count is zero. Do not call it from new code that has a
/// real scale available.
///
/// Infallible by construction — every argument is a compile-time constant that
/// `new` accepts — so it returns `Self`, not `Outcome<Self>`. That is
/// deliberate: an `Outcome` here would force ~184 migration sites to handle an
/// error that cannot occur, and H-1 forbids the `unwrap` they would reach for.
pub fn unscaled_legacy() -> Self
```

It returns `model_scale: 1.0`, `tau_in: TOLERANCE`, `tau_rep: TOLERANCE`,
`tau_col: TOLERANCE`.

Construct the struct literally. **Do not** call `new` and unwrap it, do not
`expect`, do not `match` and panic on the impossible arm — H-1 forbids all
three, and the whole point of the signature is that no such arm exists.

### The one semantic difference, which you must document and test

The legacy `Tolerance::near` is `abs_diff_eq`, which for a cgmath point or
vector is **componentwise**: every coordinate within `TOLERANCE`.
`ToleranceCtx::near_pt` is **Euclidean**: the magnitude of the difference within
`tau_rep * model_scale`. These are not the same predicate. Euclidean is the
stricter of the two — it is never true where componentwise is false, and it is
false for a difference of `(TOLERANCE, TOLERANCE, TOLERANCE)`, which
componentwise accepts, because that magnitude is `TOLERANCE * sqrt(3)`.

This is a deliberate tightening and Euclidean is the correct predicate; a
tolerance that depends on the coordinate frame is not a tolerance. Say so in a
doc comment on `unscaled_legacy` — one sentence, naming the `sqrt(3)` factor —
so that a shard whose test moves knows this is the reason and reports it rather
than widening something.

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `unimplemented!`, `todo!`, or
  out-of-range indexing. This bites here: see "Construct the struct literally".
- **H-2** Fallible operations return `Outcome<T>`. `unscaled_legacy` is *not*
  fallible and must not return one.
- **H-3** No absolute constants in predicates. Use the existing `TOLERANCE`
  const, never a fresh `1.0e-6`. **`scripts/kernel-gates.sh` flags a bare float
  literal on any added line, and test epsilons trip it. The opt-out is a
  `// H-3` comment ON THE SAME LINE as the literal** — not on the line above,
  which does not work. You will need it on the `1.0` and on any comparison
  epsilon in your tests; say what the quantity is.
- **H-6** A value computed in floats is never recorded as `Method::Exact`.

## Tests required

Append to the existing `vendor/truck/truck-base/tests/tolerance_ctx.rs`, which
already holds five tests. Leave those five exactly as they are.

Each must be a named `#[test]` fn — the verifier checks the names appear in your
diff, so a test you describe but do not write fails the gate.

1. `unscaled_legacy_carries_the_legacy_epsilon` — `model_scale()` is `1.0`, and
   `is_small_ratio` and `is_small_len` accept a quantity just under `TOLERANCE`
   and reject one just over it. This is the property Stage A rests on.
2. `unscaled_legacy_is_never_looser_than_the_legacy_predicate` — over a fixed
   set of point pairs including the `(TOLERANCE, TOLERANCE, TOLERANCE)`
   difference, `near_pt` implies `Tolerance::near`, and the two disagree on that
   specific pair. Assert the implication in that direction only: the reverse is
   false and asserting it would be asserting the bug.
3. `unscaled_legacy_agrees_with_new_at_scale_one` — the context from
   `unscaled_legacy()` equals the one `new(1.0, TOLERANCE, TOLERANCE,
   TOLERANCE)` produces, on every predicate you can call. `new` returns
   `Outcome`, so unwrap it the way the existing five tests in this file do —
   read them, copy that, do not invent a way.

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
`scripts/kernel-gates.sh`, `evidence.rs`, or `lib.rs`, do not migrate any call
site anywhere, and **do not write to `loop/`: your result file goes in the root
of your worktree and nowhere else.** Changing or deleting any existing item in
`tolerance.rs` (`TOLERANCE`, `Tolerance`, `Origin`, the macros, `new`, `scaled`,
`model_scale`, the four predicates, `entity_tau`) or any of the five existing
tests. Making `model_scale` public or adding a setter. Giving `unscaled_legacy`
an argument. Adding `Default` for `ToleranceCtx` — a default context is exactly
the silent absolute tolerance this item exists to make countable. Adding
`#[ignore]`. Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`
- a required test cannot be written without inventing a rule this packet does
  not state → `SPEC_GAP`, naming the readings you could not choose between
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

```json
{"id":"BG-TOL-001-TYPE-r2","status":"DONE","contracts":["BG-TOL-001"],
 "tests_added":3,"anchors_verified":{"A1":1,"A2":0,"A3":1,"A4":1,"A5":5},
 "notes":"anything a reviewer should know, especially any place the Euclidean/componentwise difference surprised you"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(base): the Stage-A migration scaffold for BG-TOL-001 (BG-TOL-001-TYPE-r2)`.
