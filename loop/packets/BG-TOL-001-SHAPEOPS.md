# WORK PACKET BG-TOL-001-SHAPEOPS — Stage-A tolerance migration, truck-shapeops

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```yaml
id:          BG-TOL-001-SHAPEOPS
contract:    [BG-TOL-001]
class:       wide-mechanical
crates:      [truck-shapeops]
depends_on:  [BG-TOL-001-TYPE, BG-TOL-001-TYPE-r2]
write_allow:
  - vendor/truck/truck-shapeops/src/fillet/mod.rs
  - vendor/truck/truck-shapeops/src/healing/split_closed_faces.rs
  - vendor/truck/truck-shapeops/src/transversal/intersection_curve/mod.rs
  - vendor/truck/truck-shapeops/src/transversal/loops_store/mod.rs
  - vendor/truck/truck-shapeops/src/transversal/polyline_construction/mod.rs
  - vendor/truck/truck-shapeops/tests/tolerance_migration.rs
read_allow:
  - vendor/truck/truck-base/src/tolerance.rs
tests_required:
  - every_migrated_shapeops_site_is_marked
  - param_sites_are_unaffected_by_model_scale
budget:      {turns: 60, ctx_tokens: 140000}
```

**This is a churn packet.** Every judgement has been made for you and is in the
table below. Your job is to apply exactly those 17 rewrites, mark them, and keep
the crate building and its tests passing. If you find yourself deciding whether
a site is `model` or `param`, stop and re-read the table — it is already there.

## Problem

Tolerance in this crate is a bare absolute constant: `TOLERANCE` (1e-6),
`.near()`, `.so_small()`. A comparison against a **model-space length** — a
distance between points, a radius, a gap — is only meaningful relative to how
big the model is; the same 1e-6 that is generous on a 10mm bracket is
meaningless on a 30m airframe. A comparison against a **dimensionless**
quantity — a curve parameter, a `uv` coordinate, a sine, a weight — is already
scale-free, and scaling it would be a new bug.

Today the code does not record which is which, and that judgement cannot be
recovered mechanically later. This packet records it, for `truck-shapeops`,
without changing any threshold.

**Stage A, which is all this packet is.** Each site is rewritten through a
`ToleranceCtx` obtained from `ToleranceCtx::unscaled_legacy()`, which carries
`model_scale = 1.0` and `tau_rep = TOLERANCE`. **No threshold moves and no
signature changes.** A later Stage-B packet derives a real `model_scale` at the
crate's entry points and threads it inward, deleting the `unscaled_legacy()`
calls. That is what actually fixes the scale bug; this packet buys the
judgement, which is the expensive half.

## Anchors — verified 2026-08-16, counts are exact

Locate by running the `rg` command. **Never locate by line number** — the line
numbers in the table below are provenance for a human reader, not a way to find
anything. **If a count differs, STOP** and report `ANCHOR_MISMATCH`.

| # | file | `rg` pattern | expect |
|---|---|---|---|
| A1 | `vendor/truck/truck-shapeops/src/fillet/mod.rs` | `\.near\(` | **3** |
| A2 | `vendor/truck/truck-shapeops/src/healing/split_closed_faces.rs` | `\.near\(\|\.so_small\(` | **8** |
| A3 | `vendor/truck/truck-shapeops/src/transversal/loops_store/mod.rs` | `\.near\(` | **4** |
| A4 | `vendor/truck/truck-shapeops/src/transversal/intersection_curve/mod.rs` | `\.near\(` | **1** |
| A5 | `vendor/truck/truck-shapeops/src/transversal/polyline_construction/mod.rs` | `TOLERANCE` | **2** |
| A6 | `vendor/truck/truck-base/src/tolerance.rs` | `unscaled_legacy` | **≥1** |

A6 is a dependency check: `unscaled_legacy` must already exist. If it does not,
this packet cannot be done — report `BLOCKED`, do not write the constructor
yourself.

## The four recipes — use these and nothing else

`ToleranceCtx` gives you exactly four predicates. Which one a site gets follows
from its classification and the type it compares. There is no fifth form.

| classification | what is compared | rewrite |
|---|---|---|
| `model` | two `Point3` | `ctx.near_pt(a, b)` |
| `model` | a `Vector3` against zero (`v.so_small()`) | `ctx.is_small_len(v.magnitude())` |
| `model` | two `f64` lengths | `ctx.is_small_len(a - b)` |
| `param` | two `f64` parameters | `ctx.is_small_ratio(a - b)` |
| `param` | two `Point2` / `Vector2` (`uv`) | `ctx.is_small_ratio((a - b).magnitude())` |
| `param` | an `f64` `uv`-space distance against zero | `ctx.is_small_ratio(d)` |

Obtain the context **once at the top of each function or closure that contains
at least one site**, as `let ctx = ToleranceCtx::unscaled_legacy();`, and use it
for every site in that function. Do not construct one per site. Do not add a
parameter to any signature — Stage B does that, and doing it here breaks
callers in other crates that are not on your allowlist.

Every rewritten line carries a trailing `// BG-TOL-001: model` or
`// BG-TOL-001: param`.

### The one semantic difference, already decided

Legacy `.near()` is `abs_diff_eq` — **componentwise**: every coordinate within
`TOLERANCE`. `near_pt` is **Euclidean**: the magnitude of the difference. These
are not the same predicate; Euclidean is the stricter, by at most a factor of
`sqrt(3)`. This is a deliberate tightening and Euclidean is correct — a
tolerance that depends on the coordinate frame is not a tolerance.

**If an existing test moves because of this, that is a finding, not a
nuisance.** Report it in `RESULT.json` notes with the test name and what
changed. Do not widen a tolerance, do not `#[ignore]` it, and do not switch the
site back to componentwise to make it pass.

## The sites — all 17, already classified

Line numbers are provenance only; locate with the `rg` patterns above.

**`fillet/mod.rs`** — 3 sites
| line | code | class | why |
|---|---|---|---|
| 76 | `!curve_hint.near(&t0)` | `param` | curve parameters |
| 98 | `!t0.near(&curve_hint)` | `param` | curve parameters |
| 615 | `!r0.near(&radius.subs(t0))` | `model` | `r0` is a radius, a length |

**`healing/split_closed_faces.rs`** — 8 sites
| line | code | class | why |
|---|---|---|---|
| 183 | `p.x.near(&q.x)` | `param` | `p`, `q` are `Point2` in `uv` |
| 190 | `p.y.near(&q.y)` | `param` | same |
| 366 | `line.distance_to_point(first).so_small()` | `param` | `line` is `Line(Point2, Point2)`, `uv` space |
| 369 | `line.distance_to_point(last).so_small()` | `param` | same |
| 414 | `previous0.near(&t0) && previous1.near(&t1)` | `param` | curve parameters, **two** predicates on this line |
| 618 | `!vec[0].near(&last)` | `param` | `Vec<Point2>` in `uv` |
| 620 | `surface.uder(u0, v0).so_small() \|\| surface.vder(u0, v0).so_small()` | `model` | derivatives are model-space `Vector3`; **two** predicates |
| 661 | `line.distance_to_point_as_segment(*uv).so_small()` | `param` | `uv` space |

Note 618 and 620 sit in the same `if`: 618's condition is `param` and 620's is
`model`. That is not a mistake — one compares `uv` coordinates and the other
compares surface derivatives. Mark them differently.

**`transversal/loops_store/mod.rs`** — 4 sites
| line | code | class | why |
|---|---|---|---|
| 168 | `t0.near(&t)` | `param` | curve parameters |
| 170 | `t1.near(&t)` | `param` | curve parameters |
| 425 | `point.near(&pt0) && point.near(&pt1) && pt0.near(&pt1)` | `model` | `Point3`; **three** predicates on this line |
| 509 | `polyline.front().near(&polyline.back())` | `model` | `Point3` |

**`transversal/intersection_curve/mod.rs`** — 1 site
| line | code | class | why |
|---|---|---|---|
| 41 | `poly[0].near(&poly[len - 1])` | `model` | polyline `Point3` |

**`transversal/polyline_construction/mod.rs`** — 1 site
| line | code | class | why |
|---|---|---|---|
| 86 | `!line.0.near(&line.1)` | `model` | `Point3` |

## Explicitly out of scope — do not touch these

Each is excluded for a stated reason. Leaving them alone is correct; migrating
one is a rejection.

1. **`fillet/experiment.rs`, all 5 sites.** The module is not compiled —
   `fillet/mod.rs` carries `//mod experiment;`, commented out. Migrating code
   that nothing builds or tests is unverifiable. The file is not on your
   allowlist.
2. **`transversal/polyline_construction/mod.rs:32`,
   `pt.add_element_wise(TOLERANCE) / (2.0 * TOLERANCE)`.** This is a spatial
   hash bucket size, not a predicate — `TOLERANCE` is being used as a grid
   pitch. It has no model/param classification because it compares nothing.
   Leave the line exactly as it is and add `// FIXME(BG-TOL-001): quantization
   pitch, not a predicate` above it. This is why A5 expects **2** and only one
   of them migrates.
3. **`transversal/intersection_curve/tests.rs`, both sites.** In-crate
   `#[cfg(test)]` code. Stage A is about production predicates; a test's
   epsilon is the test's own business. Not on your allowlist.
4. **`fillet/mod.rs` lines 171 and 181.** Both are commented-out
   `debug_assert!`s. They are not code.
5. **Anything using `.near2()` or `.so_small2()`.** These compare against
   `TOLERANCE2` = 1e-12, and `ToleranceCtx` has no squared-order predicate —
   mapping them onto `tau_rep` would loosen them by six orders of magnitude
   while appearing to migrate them. There are none in your allowlisted files,
   so this should not arise; if one does, mark it
   `// FIXME(BG-TOL-001): squared order` and leave it.

## The ratchet — read this before you commit

`scripts/kernel-gates.sh` counts `unscaled_legacy(` call sites in
`vendor/truck/*/src/**` and **fails when the total exceeds the ceiling** in
`scripts/unscaled_legacy_ceiling.txt`. The ceiling has been raised to **20** for
this packet. That file is **not** on your allowlist and you must not edit it —
the ceiling exists to constrain this packet and a packet that can move its own
ceiling is not constrained by anything.

Twenty is a budget, not a target. One context per function containing sites is
roughly a dozen; if you need more than twenty you have constructed one per site
instead of one per function, which is the mistake this number is sized to catch.

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `unimplemented!`, `todo!`, or
  out-of-range indexing on any line you add. `unscaled_legacy()` is infallible
  and returns `Self`, so there is nothing to unwrap.
- **H-2** Fallible operations return `Outcome<T>`. You are not adding any.
- **H-3** No absolute constants in predicates — that is the whole point of this
  packet. **`scripts/kernel-gates.sh` flags a bare float literal on any added
  line, and test epsilons trip it. The opt-out is a `// H-3` comment ON THE SAME
  LINE as the literal** — not on the line above, which does not work. Use it in
  your tests and say what the quantity is.
- **H-6** A value computed in floats is never recorded as `Method::Exact`.

## Tests required

New file `vendor/truck/truck-shapeops/tests/tolerance_migration.rs`. Each must
be a named `#[test]` fn — the verifier checks the names appear in your diff.

1. `every_migrated_shapeops_site_is_marked` — read the five migrated source
   files from `CARGO_MANIFEST_DIR` at runtime and assert that the number of
   lines containing `ctx.near_pt(`, `ctx.is_small_len(` or `ctx.is_small_ratio(`
   equals the number containing a `// BG-TOL-001:` marker. This is the test that
   makes the marking checkable rather than a convention; without it the markers
   rot the first time someone edits a line.
2. `param_sites_are_unaffected_by_model_scale` — a property test on
   `ToleranceCtx` itself, not on this crate's functions: for several
   `model_scale` values, `is_small_ratio` gives identical answers, while
   `is_small_len` does not. This is the invariant every `param` classification
   in the table above depends on, and if it ever fails, someone has scaled a
   ratio.

`truck-shapeops` does **not** set `autotests = false`, so a new file in
`tests/` is picked up automatically; you do not need a `[[test]]` entry. Check
`Cargo.toml` and report it if that is not what you find.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-shapeops
cargo clippy -p truck-shapeops --all-targets --no-deps -- -D warnings
cargo test -p truck-shapeops --lib --test tolerance_migration --no-fail-fast
cargo check --workspace --all-targets
```

`--no-deps` matters: without it clippy lints the vendored path dependencies too
and aborts before reaching your crate. Never run a bare `cargo test` — it builds
56 examples. Send cargo output to a file and read the tail.

**Two tests in this crate fail before you touch anything** and are not yours:
`tests::fillet::complex_surface` (triangulates to `Irregular`) and
`healing::tests::step_import` (needs a STEP data file absent on this machine).
Confirm they fail identically at the base commit and say so in your notes; do
not try to fix them and do not let them stop you.

## Forbidden

Editing any file outside `write_allow` — in particular
`scripts/unscaled_legacy_ceiling.txt`, `scripts/kernel-gates.sh`,
`truck-base/src/tolerance.rs`, `fillet/experiment.rs`, and
**`loop/` anything: your result file goes in the root of your worktree and
nowhere else.** Changing any function signature. Adding a `ctx` parameter.
Changing any threshold. Migrating a site the "out of scope" section excludes.
Widening a tolerance or adding `#[ignore]` to make a test pass. Committing to
`main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- a site in the table does not typecheck under its assigned recipe → `SPEC_GAP`,
  naming the site and the actual types. Do not reclassify it to make it compile;
  a `model` site that will not take `near_pt` is telling you something.
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

```json
{"id":"BG-TOL-001-SHAPEOPS","status":"DONE","contracts":["BG-TOL-001"],
 "tests_added":2,"sites_migrated":17,"unscaled_legacy_calls":0,
 "anchors_verified":{"A1":3,"A2":8,"A3":4,"A4":1,"A5":2},
 "notes":"set unscaled_legacy_calls to the number you actually introduced. Report every existing test whose behaviour moved, with the name and the reason, and anything the Euclidean/componentwise difference changed."}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`refactor(shapeops): classify every tolerance site model or param (BG-TOL-001-SHAPEOPS)`.
