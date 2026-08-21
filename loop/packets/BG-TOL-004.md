# WORK PACKET BG-TOL-004 — degree-aware squared tolerances on ToleranceCtx

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-TOL-004","status":"DONE","contracts":["BG-TOL-004"],
 "tests_added":4,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: the claims below were derived
by command against the tree, but they are exactly the kind of claim that can be
confidently wrong. **If anything below contradicts what you find in the code,
say so in `disagreements` rather than making the code match the packet.**

```yaml
id:          BG-TOL-004
contract:    [BG-TOL-004]
class:       design
crates:      [truck-base]
write_allow:
  - vendor/truck/truck-base/src/tolerance.rs
read_allow:
  - vendor/truck/truck-base/src/evidence.rs
tests_required:
  - is_small_len2_reproduces_tolerance2_at_stage_a
  - is_small_len2_scales_quadratically
  - is_small_ratio2_is_scale_invariant
  - length2_margin_is_the_square_of_length_margin
budget:      {turns: 30, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 0, cmd: "grep -c 'len2\\|ratio2\\|length2' vendor/truck/truck-base/src/tolerance.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn is_small_len(' vendor/truck/truck-base/src/tolerance.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub fn is_small_ratio(' vendor/truck/truck-base/src/tolerance.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub fn length_margin(' vendor/truck/truck-base/src/tolerance.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'pub const TOLERANCE2' vendor/truck/truck-base/src/tolerance.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'fn unscaled_legacy' vendor/truck/truck-base/src/tolerance.rs"}
```

## Problem — what a squared-order tolerance means in a scale-relative system

Stage A migrated first-order tolerance predicates onto `ToleranceCtx`:
`is_small_len` (a length against `tau_rep * model_scale`) and
`is_small_ratio` (a dimensionless quantity against `tau_rep` alone). Twenty
sites tree-wide were excluded from every Stage-A shard because they compare
against the **squared** constant `TOLERANCE2 = 1e-12` — the `near2`/`so_small2`
legacy family — and nothing on `ToleranceCtx` reproduces that number. The
exclusion census (verified: 20 sites today) splits into four classes:

1. **Squared-distance comparisons** — `d.distance2(c) <= TOLERANCE2` and
   friends: algebraically `distance <= TOLERANCE`, an ordinary first-order
   predicate written squared to skip a `sqrt`. Its scale-relative form is
   `q <= (tau_rep * model_scale)^2`.
2. **Genuine degree-2 quantities** — cross-product magnitudes and areas: a
   quantity that scales as `k^2` under a model rescale needs a margin that
   scales as `k^2`. Same predicate as class 1: `q <= (k tau)^2`.
3. **Dimensionless tight floors** — knot-normalization and Newton-convergence
   checks (`knot(i).near2(&1.0)`, `next.near2(&param)`): degree-ZERO
   quantities compared against `tau^2` purely as a "much tighter than tau"
   floor. Degree zero means scale-invariant: the correct scale-relative form
   keeps the tight floor and deliberately does NOT scale it — `|x| <= tau^2`
   at every model scale.
4. **Dimensionally incoherent sites** — degree-3 triple products
   (`collision.rs`), homogeneous-point comparisons, the `1/k` residual at
   `contact_circle.rs`. No single predicate can fit these; they stay excluded
   for per-site redesign and are NOT this packet's problem.

**This packet delivers the two predicates classes 1-3 migrate onto** (the
follow-up shards that walk the 20 sites are separate rows; you touch no call
site):

- `is_small_len2` + `length2_margin` — the degree-2-in-length companion of
  `is_small_len`/`length_margin`, for classes 1 and 2.
- `is_small_ratio2` — the named tight floor for class 3.

## Decisions already made for you

1. **Add exactly these three methods to `impl ToleranceCtx`**, placed
   immediately after `is_small_ratio` (before `entity_tau`), with docs in the
   house voice (read the doc comments on `is_small_len` and `length_margin`
   first and match them):

   ```rust
   /// MODEL-SPACE, DEGREE 2 IN LENGTH. The absolute margin a squared-length
   /// comparison uses at this model's scale: `(tau_rep * model_scale)^2`.
   ///
   /// The one-sided squared counterpart of [`Self::length_margin`], for
   /// quantities that are degree two in length: squared distances, squared
   /// magnitudes, twice a triangle's area. Under a model rescale by `k` such a
   /// quantity scales as `k^2`, and so does this margin.
   pub fn length2_margin(&self) -> f64 {
       self.length_margin() * self.length_margin()
   }

   /// MODEL-SPACE, DEGREE 2 IN LENGTH. True when a quantity of degree two in
   /// length is negligible at this model's scale: `q <= (tau_rep *
   /// model_scale)^2`.
   ///
   /// This is the sqrt-free form of [`Self::is_small_len`] for squared
   /// distances: `d.distance2(c) <= TOLERANCE2` migrates to
   /// `ctx.is_small_len2(d.distance2(c))` with identical behaviour at Stage A
   /// (`model_scale == 1.0` makes the margin exactly `TOLERANCE2`). At the
   /// boundary it can differ from `is_small_len(q.sqrt())` by one ulp — the
   /// squared form is the predicate, not an approximation of the sqrt form.
   /// The argument must be non-negative by construction (a squared distance,
   /// an area); `.abs()` is applied anyway so a stray negative is small
   /// rather than silently never-small.
   pub fn is_small_len2(&self, q: f64) -> bool {
       q.abs() <= self.length2_margin()
   }

   /// DIMENSIONLESS, DEGREE ZERO — deliberately NOT scaled, and deliberately
   /// the SQUARE of `ratio_margin`. The legacy family used `TOLERANCE2` as a
   /// "much tighter than tau" floor for iteration convergence and
   /// normalization checks on dimensionless quantities (knot values, Newton
   /// parameters). Degree zero means scale-invariant: the tight floor is
   /// correct at every model scale, and this predicate names it instead of
   /// leaving a bare `1e-12` at the call site. It is a floor, not a derived
   /// quantity — do not use it for anything that is genuinely a squared
   /// length; that is [`Self::is_small_len2`].
   pub fn is_small_ratio2(&self, x: f64) -> bool {
       x.abs() <= self.ratio_margin() * self.ratio_margin()
   }
   ```

   You may polish wording, not semantics. If a doc claim feels wrong, put it
   in `disagreements` and keep the semantics as written.

2. **No other production change.** Not the `Tolerance`/`Origin` traits, not
   the macros, not `TOLERANCE`/`TOLERANCE2` themselves, not `evidence.rs`, and
   **no call site anywhere** — the 20 excluded sites migrate in follow-up
   shards. Do NOT add any `unscaled_legacy()` call: GATE-4 sits at 110/110 and
   this packet must not move it.

3. **Tests — four, added inline in `tolerance.rs`** beside the existing
   `#[test]` fns (the file already carries inline tests like
   `assert_near_without_msg`; match that style):

   - `is_small_len2_reproduces_tolerance2_at_stage_a`:
     `unscaled_legacy().is_small_len2(TOLERANCE2)` is `true` (the boundary is
     inclusive — margin is exactly `TOLERANCE2` at Stage A), and
     `is_small_len2(TOLERANCE2 * 2.0)` is `false`.
   - `is_small_len2_scales_quadratically`: build a context at scale `10.0`
     via `ToleranceCtx::new(10.0, TOLERANCE, TOLERANCE, TOLERANCE)` — it
     returns `Outcome<Self>`, so bind with
     `let Ok(c) = ... else { unreachable!() };` (the crate denies clippy
     lints at crate level; `unreachable!()` is fine, `unwrap`/`expect` are
     not the house style). Then `is_small_len2(100.0 * TOLERANCE2)` is `true`
     and `is_small_len2(200.0 * TOLERANCE2)` is `false` — the margin moved by
     `10^2`, not `10`.
   - `is_small_ratio2_is_scale_invariant`: the same `TOLERANCE2` / `2.0 *
     TOLERANCE2` pair gives identical answers at scale `1.0`
     (`unscaled_legacy()`) and scale `10.0` (the context above).
   - `length2_margin_is_the_square_of_length_margin`: for both contexts,
     `assert_eq!(ctx.length2_margin(), ctx.length_margin() *
     ctx.length_margin())` — this is exact float equality and must be, because
     the method is DEFINED as that product (do not compute the margin any
     other way, or this test is a lie).

4. **The crate is `#![deny(clippy::all, rust_2018_idioms)]` and
   `#![cfg_attr(not(debug_assertions), deny(warnings))]`** — every lint on a
   line you add is a hard error. The crate is clippy-clean and its tests are
   green at baseline (verified); keep both that way.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N` literal unless that same line ends with an `// H-3` comment. Use the
named constants `TOLERANCE` and `TOLERANCE2` — that is what they are for — and
multipliers like `2.0`/`10.0`/`100.0`, which are not in `1e-N` form. If a bare
`1e-N` literal is ever truly unavoidable, the line must end with a same-line
`// H-3:` comment naming the dimensionless quantity. Run
`bash scripts/kernel-gates.sh` yourself before writing `RESULT.json`; it is
the same script V4 runs.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-base
cargo clippy -p truck-base --all-targets --no-deps
cargo test -p truck-base --lib --tests --no-fail-fast
cargo check --workspace --all-targets
```

Never run a bare `cargo test` — it builds 56 examples. Send cargo output to a
file and read the tail.

**The crate is clean at baseline** — measured at the tree this packet was
written against: clippy reports zero findings, and the test suite passes.
Your bar: everything above stays green, plus your four new tests. There are
no baseline failures to tolerate — any failure you did not cause is a stop
condition, and any failure you did cause is yours to fix.

## Forbidden

Editing any file outside `write_allow` — `evidence.rs`, any other module of
truck-base, and every other crate especially. Migrating any call site (the 20
excluded sites belong to follow-up shards). Adding any `unscaled_legacy()`
call. Changing `TOLERANCE`/`TOLERANCE2`, the `Tolerance`/`Origin` traits, or
the assertion macros. Adding `#[ignore]`. Adding `unwrap()`/`expect()` on
fallible paths. Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- you conclude one of the three methods as specified is semantically wrong for
  its documented class → do NOT redesign it silently: implement it as written
  and put the argument in `disagreements` — the adjudication is the
  orchestrator's
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(tolerance): degree-aware squared margins on ToleranceCtx (BG-TOL-004)`.
