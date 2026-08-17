# WORK PACKET BG-TOL-001-GEOM-SPECIFIEDS — Stage-A tolerance migration, truck-geometry/specifieds

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```yaml
id:          BG-TOL-001-GEOM-SPECIFIEDS
contract:    [BG-TOL-001]
class:       wide-mechanical
crates:      [truck-geometry]
depends_on:  [BG-TOL-001-TYPE, BG-TOL-001-TYPE-r2, BG-TOL-001-TYPE-r3]
write_allow:
  - vendor/truck/truck-geometry/src/specifieds/circle.rs
  - vendor/truck/truck-geometry/src/specifieds/hyperbola.rs
  - vendor/truck/truck-geometry/src/specifieds/line.rs
  - vendor/truck/truck-geometry/src/specifieds/parabola.rs
  - vendor/truck/truck-geometry/src/specifieds/plane.rs
  - vendor/truck/truck-geometry/src/specifieds/sphere.rs
  - vendor/truck/truck-geometry/src/specifieds/torus.rs
  - vendor/truck/truck-geometry/tests/tolerance_specifieds.rs
read_allow:
  - vendor/truck/truck-base/src/tolerance.rs
  - vendor/truck/truck-geometry/src/specifieds/mod.rs
tests_required:
  - canonical_sites_do_not_scale_with_the_model
  - model_space_sites_do_scale_with_the_model
budget:      {turns: 70, ctx_tokens: 150000}
```

**22 sites migrate, 1 is deliberately left with a `FIXME`.** Every judgement is
made for you in the site table. If you find yourself deciding a classification,
re-read the table — it is already there.

## Problem

Tolerance in this module is a bare absolute constant: `TOLERANCE` (1e-6),
`.near()`, `.so_small()`. Whether a comparison should scale with the size of
the model is a judgement no one has recorded, and it cannot be recovered
mechanically later. This packet records it for the analytic primitives.

**Stage A, which is all this packet is.** Each site is rewritten through a
`ToleranceCtx` from `ToleranceCtx::unscaled_legacy()`, which carries
`model_scale = 1.0` and `tau_rep = TOLERANCE`, so **no threshold moves and no
signature changes.** Stage B threads a real `model_scale` later.

## The rule that decides every site here: the frame, not the type

This module contains two kinds of primitive and they classify oppositely.

**Canonical primitives** — `UnitCircle`, `UnitHyperbola`, `UnitParabola` — are
the `PhantomData` types in `mod.rs`. They carry no geometry of their own; their
shape is fixed in a normalized frame where the characteristic radius is **1 by
construction**. A distance in that frame is a dimensionless multiple of that
unit, so it is `param` **even though it is a distance and looks exactly like a
model-space length**. Scaling it by `model_scale` would be a new bug.

**Model-space primitives** — `Line`, `Plane`, `Sphere`, `Torus` — carry real
geometry (centres, radii, points), and their lengths are model-space lengths.

The rule is **the frame the quantity lives in decides, not its type.** These
two lines are byte-identical in shape and classify differently:

```rust
// circle.rs, inside UnitCircle::parameter_division -- canonical frame
let tol = tol.max(TOLERANCE);      // BG-TOL-001: param
// sphere.rs, inside Sphere::parameter_division -- model space
let tol = tol.max(TOLERANCE);      // BG-TOL-001: model
```

**The whole-file shortcut holds here and you may rely on it:**
`circle.rs`, `hyperbola.rs` and `parabola.rs` are entirely canonical →
every site is `param`. `line.rs`, `plane.rs`, `sphere.rs` and `torus.rs` are
model-space → every site is `model` **except the two dimensionless quantities
named in the table** (a homogeneous weight and a sine).

## Anchors — verified 2026-08-16, counts are exact

Locate by running the pattern. **Never locate by line number.** `rg` is not
installed on this machine; any case-sensitive literal search is equivalent.
**If a count differs, STOP** and report `ANCHOR_MISMATCH`.

Every count is of `\.near\(|so_small\(|TOLERANCE` in that file. They are raw
counts and are **larger than the number of sites you migrate**, because they
include doc-comment examples and `#[cfg(test)]` bodies. That is deliberate:
matching the raw count confirms you are looking at the file this packet was
written against.

| # | file | expect |
|---|---|---|
| A1 | `specifieds/circle.rs` | **5** |
| A2 | `specifieds/hyperbola.rs` | **3** |
| A3 | `specifieds/line.rs` | **2** |
| A4 | `specifieds/parabola.rs` | **3** |
| A5 | `specifieds/plane.rs` | **5** |
| A6 | `specifieds/sphere.rs` | **7** |
| A7 | `specifieds/torus.rs` | **3** |
| A8 | `truck-base/src/tolerance.rs`, pattern `length_margin` | **≥1** |

A8 is a dependency check: `length_margin`, `ratio_margin` and `near_points`
must already exist. If they do not, report `BLOCKED`; do not write them.

## The recipes — use these and nothing else

| classification | shape | rewrite |
|---|---|---|
| `model` | two `Point3` | `ctx.near_pt(a, b)` |
| `model` | a vector against zero | `ctx.is_small_len(v.magnitude())` |
| `model` | an `f64` length against zero | `ctx.is_small_len(x)` |
| `model` | two `f64` lengths | `ctx.is_small_len(a - b)` |
| `model` | a tolerance floor | `tol.max(ctx.length_margin())` |
| `param` | an `f64` dimensionless value against zero | `ctx.is_small_ratio(x)` |
| `param` | two `f64` dimensionless values | `ctx.is_small_ratio(a - b)` |
| `param` | two canonical-frame points | `ctx.is_small_ratio((a - b).magnitude())` |
| `param` | a vector in a canonical frame | `ctx.is_small_ratio(v.magnitude())` |
| `param` | a tolerance floor | `tol.max(ctx.ratio_margin())` |

Obtain the context **once at the top of each function that contains at least
one site**, as `let ctx = ToleranceCtx::unscaled_legacy();`. Do not construct
one per site. **Do not add a parameter to any signature or a bound to any
`where` clause** — Stage B does that, and doing it here breaks callers in other
crates that are not on your allowlist.

Every rewritten line carries a trailing `// BG-TOL-001: model` or
`// BG-TOL-001: param`. If rustfmt relocates the comment to the following line,
leave it where rustfmt puts it.

## The sites — 22 migrate

Line numbers are provenance for a human reader; locate with the patterns.

**`circle.rs`** — canonical, all `param`
| line | code |
|---|---|
| 86 | `let tol = tol.max(TOLERANCE);` → `tol.max(ctx.ratio_margin())` |
| 111 | `if v.magnitude().so_small()` |
| 128 | `if !v.magnitude().near(&1.0)` → `!ctx.is_small_ratio(v.magnitude() - 1.0)` |
| 188 | `if !f64::abs(pt.z).so_small()` → `!ctx.is_small_ratio(pt.z)` (the `abs` is redundant once migrated — `is_small_ratio` takes its own absolute value; drop it) |

**`hyperbola.rs`** — canonical, all `param`
| line | code |
|---|---|
| 84 | `match z.im.so_small()` — imaginary part of a polynomial root |
| 117 | `match p.near(&self.subs(t))` → `ctx.is_small_ratio((p - self.subs(t)).magnitude())` (`Point2`) |
| 128 | same, `Point3` |

**`parabola.rs`** — canonical, all `param`
| line | code |
|---|---|
| 86 | `match x.im.so_small()` |
| 121 | `match pt.near(&pt0)` → `ctx.is_small_ratio((pt - pt0).magnitude())` |
| 137 | `match pt.z.so_small()` |

**`line.rs`** — model
| line | code |
|---|---|
| 186 | `match self.subs(t).near(&pt)` → `ctx.is_small_len((self.subs(t) - pt).magnitude())` |

`Line<P>` is generic and `P` is bounded `ControlPoint<f64> + Tolerance`, which
does **not** give you `near_pt` or `near_points`. It does give
`P::Diff: InnerSpace<Scalar = f64>`, so `(a - b).magnitude()` is available and
that is why the recipe above goes through a magnitude. **Do not add a bound.**

**`plane.rs`** — model, except line 230
| line | code | class |
|---|---|---|
| 208 | `.all(\|pt\| (pt - origin).dot(normal).so_small())` | `model` |
| 226 | `if !(s - origin).dot(normal).so_small() \|\| !(e - origin).dot(normal).so_small()` — **two** predicates | `model` |
| 230 | `if pt[3].so_small()` | **`param`** — a homogeneous coordinate is a weight, dimensionless |
| 234 | `(pt - origin).dot(normal).so_small()` | `model` |
| 285 | `match v[2].so_small()` | `model` |

**`sphere.rs`** — model, except line 216
| line | code | class |
|---|---|---|
| 26 | `self.center.distance(pt).near(&self.radius)` → `ctx.is_small_len(self.center.distance(pt) - self.radius)` | `model` |
| 172 | `let tol = tol.max(TOLERANCE);` → `tol.max(ctx.length_margin())` | `model` |
| 216 | `if sinu.so_small()` | **`param`** — a sine is a ratio |

**`torus.rs`** — model
| line | code |
|---|---|
| 157 | `match self.subs(u, v).near(&point)` → `ctx.near_pt(self.subs(u, v), point)` |
| 174 | `if rxy.so_small()` — `Vector2` |
| 180 | `if diff.so_small()` — `Vector3` |

## The one site that must NOT be migrated

**`sphere.rs:211`**, `if (self.radius * self.radius).near(&radius.magnitude2())`.

Both sides are **squared lengths**. `tau_rep` is first order and `ToleranceCtx`
has no squared-order predicate, so mapping this onto `is_small_len` would
loosen it by six orders of magnitude while looking like a migration. Leave the
line byte-for-byte alone and put this on the line above it:

```rust
// FIXME(BG-TOL-001): squared order -- both sides are length squared and tau_rep is first order
```

Do **not** "fix" it by rewriting it as a first-order comparison on the
distance. That may well be the right answer, and deciding it is not this
packet's job.

## The ratchet

`scripts/kernel-gates.sh` counts `unscaled_legacy(` call sites in
`vendor/truck/*/src/**` and **fails when the total exceeds the ceiling** in
`scripts/unscaled_legacy_ceiling.txt`. The ceiling has been raised to cover
**at most 12 new call sites** for this packet. That file is **not** on your
allowlist and you must not edit it. One context per function containing sites
is about 12 here; if you need many more you have constructed one per site.

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `unimplemented!`, `todo!`, or
  out-of-range indexing on any line you add. `unscaled_legacy()` is infallible.
- **H-2** Fallible operations return `Outcome<T>`. You are adding none.
- **H-3** No absolute constants in predicates. **`kernel-gates.sh` flags a bare
  float literal on any added line, and test epsilons trip it. The opt-out is a
  `// H-3` comment ON THE SAME LINE as the literal** — not the line above.
  rustfmt relocates such a comment off a line ending in `{`; if that happens,
  extract the literal onto its own statement line. Note `circle.rs` already
  contains `f64::min(tol, 0.8)` and `2.0 * f64::acos(...)` near your site —
  those lines are not yours and you are not adding them.
- **H-6** A value computed in floats is never recorded as `Method::Exact`.

## Tests required

New file `vendor/truck/truck-geometry/tests/tolerance_specifieds.rs`. Each must
be a named `#[test]` fn.

1. `canonical_sites_do_not_scale_with_the_model` — build `ToleranceCtx` at
   several `model_scale` values and assert `ratio_margin()` and
   `is_small_ratio` give identical answers at all of them. This is the
   invariant every `param` classification in `circle.rs`, `hyperbola.rs` and
   `parabola.rs` rests on; if it ever fails, someone has scaled a canonical
   quantity.
2. `model_space_sites_do_scale_with_the_model` — the converse: `length_margin()`
   and `is_small_len` change with `model_scale`, and a fixed separation that is
   "small" at a large scale is not small at a small one.

`truck-geometry` does not set `autotests = false`, so a new file in `tests/` is
picked up automatically. Check `Cargo.toml` and report it if that is not what
you find.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-geometry
cargo clippy -p truck-geometry --all-targets --no-deps -- -D warnings
cargo test -p truck-geometry --lib --test tolerance_specifieds --test circle --test plane --test sphere --test torus --test hyperbola --no-fail-fast
cargo check --workspace --all-targets
```

Those `--test` targets are the existing integration tests for the primitives
you are touching; they are the ones that would notice a changed threshold.
`--no-deps` matters: without it clippy lints the vendored path dependencies too
and aborts before reaching your crate. Never run a bare `cargo test` — it builds
56 examples. Send cargo output to a file and read the tail.

**Confirm the baseline before you edit anything.** Run that test command at the
base commit first and record which tests already fail; this tree has
pre-existing failures that are not yours. Report them and do not fix them.

## Forbidden

Editing any file outside `write_allow` — in particular
`scripts/unscaled_legacy_ceiling.txt`, `truck-base/src/tolerance.rs`,
`specifieds/mod.rs`, and **`loop/` anything: your result file goes in the root
of your worktree and nowhere else.** Changing any function signature or `where`
clause, or adding any trait bound. Adding a `ctx` parameter. Changing any
threshold. Migrating `sphere.rs:211`. Widening a tolerance or adding
`#[ignore]` to make a test pass. Committing to `main`.

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
{"id":"BG-TOL-001-GEOM-SPECIFIEDS","status":"DONE","contracts":["BG-TOL-001"],
 "tests_added":2,"sites_migrated":22,"sites_fixmed":1,"unscaled_legacy_calls":0,
 "anchors_verified":{"A1":5,"A2":3,"A3":2,"A4":3,"A5":5,"A6":7,"A7":3},
 "notes":"set unscaled_legacy_calls to the number you actually introduced. Report the baseline test failures you confirmed, and any site where the canonical-vs-model-space call felt wrong to you -- that judgement is the whole point of this packet and a disagreement is worth more than a silent compliance."}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`refactor(geometry): classify every specifieds tolerance site model or param (BG-TOL-001-GEOM-SPECIFIEDS)`.
