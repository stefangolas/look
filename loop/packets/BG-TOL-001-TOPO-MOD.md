# WORK PACKET BG-TOL-001-TOPO-MOD — Stage-A tolerance migration, truck-topology + truck-modeling

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```yaml
id:          BG-TOL-001-TOPO-MOD
contract:    [BG-TOL-001]
class:       wide-mechanical
crates:      [truck-topology, truck-modeling]
depends_on:  [BG-TOL-001-TYPE, BG-TOL-001-TYPE-r2, BG-TOL-001-TYPE-r3]
write_allow:
  - vendor/truck/truck-topology/src/edge.rs
  - vendor/truck/truck-modeling/src/builder.rs
  - vendor/truck/truck-modeling/src/geom_impls.rs
  - vendor/truck/truck-modeling/src/geometry.rs
  - vendor/truck/truck-modeling/tests/tolerance_migration.rs
read_allow:
  - vendor/truck/truck-base/src/tolerance.rs
tests_required:
  - one_sided_range_guard_keeps_its_legacy_answers
  - param_sites_are_unaffected_by_model_scale
budget:      {turns: 50, ctx_tokens: 120000}
```

**Two crates in one packet because both are tiny.** Their write sets are
disjoint from each other and from every other shard. **Six sites migrate and
three are deliberately left alone with a `FIXME` — the three are not oversights
and migrating one is a rejection.**

## Problem

Tolerance in these crates is a bare absolute constant: `TOLERANCE` (1e-6),
`.near()`, `.so_small()`. A comparison against a **model-space length** — a
distance, a radius, a box diagonal — is only meaningful relative to how big the
model is. A comparison against a **dimensionless** quantity — a curve
parameter, a normalized magnitude, a sine — is already scale-free, and scaling
it would be a new bug. The code does not record which is which, and that
judgement cannot be recovered mechanically later.

**Stage A, which is all this packet is.** Each site is rewritten through a
`ToleranceCtx` from `ToleranceCtx::unscaled_legacy()`, which carries
`model_scale = 1.0` and `tau_rep = TOLERANCE`. **No threshold moves and no
signature changes.** Stage B threads a real `model_scale` later.

## Anchors — verified 2026-08-16, counts are exact

Locate by running the pattern. **Never locate by line number** — the numbers in
the site table are provenance for a human reader. `rg` is not installed on this
machine; any case-sensitive literal search is equivalent.
**If a count differs, STOP** and report `ANCHOR_MISMATCH`.

| # | file | pattern | expect |
|---|---|---|---|
| A1 | `truck-topology/src/edge.rs` | `TOLERANCE` | **4** |
| A2 | `truck-topology/src/edge.rs` | `\.near\(` | **3** |
| A3 | `truck-modeling/src/builder.rs` | `\.near\(&1\.0\)` | **3** |
| A4 | `truck-modeling/src/builder.rs` | `so_small\(` | **1** |
| A5 | `truck-modeling/src/geom_impls.rs` | `so_small\(` | **6** |
| A6 | `truck-modeling/src/geometry.rs` | `LEADER_WITNESS_MARGIN` | **3** |
| A7 | `truck-base/src/tolerance.rs` | `length_margin` | **≥1** |

A7 is a dependency check. `length_margin`, `ratio_margin` and `near_points`
must already exist on `ToleranceCtx`. If they do not, report `BLOCKED`; do not
write them yourself.

Note A3 is **3** and A5 is **6** while only one site in each is yours. The
others are doc-comment examples and `#[cfg(test)]` proptest bodies. Counting
them is how you confirm you are looking at the file this packet was written
against; migrating them is a rejection.

## The recipes — use these and nothing else

`ToleranceCtx` gives you these predicates. There is no other form.

| classification | shape | rewrite |
|---|---|---|
| `model` | `Vector3` against zero (`v.so_small()`) | `ctx.is_small_len(v.magnitude())` |
| `model` | an `f64` length against zero | `ctx.is_small_len(x)` |
| `model` | **one-sided** length threshold | `... > k * ctx.length_margin()` |
| `param` | two `f64` dimensionless values | `ctx.is_small_ratio(a - b)` |
| `param` | **one-sided** parameter threshold | `... < t0 + ctx.ratio_margin()` |

**One-sided comparisons are the trap in this packet, so read this twice.**
`t < t0 + TOLERANCE` asks whether `t` is at or below the low end of a range. It
is **not** `is_small_ratio(t - t0)`, which asks whether `t` is *near* `t0` and
answers differently for every `t` far below `t0`, because it takes an absolute
value. Rewrite one-sided comparisons by substituting the margin accessor for
`TOLERANCE` **in place**, keeping the comparison exactly as it is:

```rust
if t < t0 + ctx.ratio_margin() || t1 - ctx.ratio_margin() < t {   // BG-TOL-001: param
```

Obtain the context **once at the top of each function that contains at least
one site**, as `let ctx = ToleranceCtx::unscaled_legacy();`. Do not construct
one per site. Do not add a parameter to any signature.

Every rewritten line carries a trailing `// BG-TOL-001: model` or
`// BG-TOL-001: param`. If rustfmt relocates the comment to the following line,
that is fine and expected; leave it where rustfmt puts it.

## The sites — 6 migrate

**`truck-topology/src/edge.rs`** — 2 sites, both `param`, both one-sided
| line | code | class |
|---|---|---|
| 459 | `if t < t0 + TOLERANCE \|\| t1 - TOLERANCE < t` (in `cut`) | `param` |
| 478 | `if t < t0 + TOLERANCE \|\| t1 - TOLERANCE < t` (in `cut_with_parameter`) | `param` |

Both are curve parameters. Both lines carry **two** comparisons; substitute
`ctx.ratio_margin()` for `TOLERANCE` in all four.

**`truck-modeling/src/builder.rs`** — 2 sites
| line | code | class | why |
|---|---|---|---|
| 622 | `debug_assert!(axis.magnitude().near(&1.0))` | `param` | a unit vector's magnitude is dimensionless |
| 727 | `(pt1 - pt0).cross(axis).so_small()` | `model` | `axis` is a unit vector, so the cross of a displacement with it has length units |

Site 622 is a `debug_assert!`. Migrate it in place — it stays a `debug_assert!`.

**`truck-modeling/src/geom_impls.rs`** — 1 site
| line | code | class | why |
|---|---|---|---|
| 101 | `if !diag[2].so_small()` | `model` | a bounding-box diagonal component is a length |

**`truck-modeling/src/geometry.rs`** — 1 site
| line | code | class | why |
|---|---|---|---|
| 318 | `if signed.abs() > LEADER_WITNESS_MARGIN * TOLERANCE` | `model`, one-sided | `signed` is a displacement dotted with a unit normal, so a length |

For 318, substitute in place: `> LEADER_WITNESS_MARGIN * ctx.length_margin()`.
`LEADER_WITNESS_MARGIN` is a dimensionless multiplier and **stays exactly as it
is** — do not fold it into the tolerance and do not change its value. The
comment above it says `TOLERANCE` is the interim `tau_rep` until `ToleranceCtx`
replaces it; update that comment to say it now is.

## The three sites that must NOT be migrated

Each gets a `FIXME(BG-TOL-001)` comment on the line above it, with the stated
reason, and is otherwise left byte-for-byte alone.

1. **`truck-topology/src/edge.rs:421`**, `geom_front.near(&*top_front) &&
   geom_back.near(&*top_back)` in `is_geometric_consistent`.
2. **`truck-topology/src/edge.rs:474`**, `!curve0.subs(t).near(&vertex.point())`
   in `cut_with_parameter`.

   Both compare the generic point type `P`, which is bounded `P: Tolerance`.
   `ctx.near_points` needs `P: MetricSpace<Metric = f64>`, and adding that bound
   to these two public methods forces it onto the forwarding
   `is_geometric_consistent` in `face.rs`, `shell.rs`, `solid.rs` and `wire.rs`,
   and then onto callers in truck-modeling and truck-shapeops. **That is a
   cross-crate public signature change, which is Stage B by definition** — Stage
   A changes no signatures. Comment:
   `// FIXME(BG-TOL-001): generic P is bounded Tolerance, not MetricSpace; the bound change is cross-crate and belongs to Stage B`

3. **`truck-modeling/src/geom_impls.rs:90`**, `let n = match normal.so_small()`.

   `normal` is a sum of `(p0 - center).cross(p1 - center)`, so its magnitude has
   units of **length squared** — an area, not a length. Neither `is_small_len`
   (a length) nor `is_small_ratio` (dimensionless) is dimensionally right, and
   guessing between them is exactly what the convention forbids. Comment:
   `// FIXME(BG-TOL-001): accumulated cross products, so the quantity is an area (length squared); neither predicate fits`

## The ratchet

`scripts/kernel-gates.sh` counts `unscaled_legacy(` call sites in
`vendor/truck/*/src/**` and **fails when the total exceeds the ceiling** in
`scripts/unscaled_legacy_ceiling.txt`. The ceiling has been raised to **19** for
this packet (11 already in the tree, plus a budget of 8). That file is **not**
on your allowlist and you must not edit it. One context per function containing
sites is 5 here; if you need more than 8 you have constructed one per site.

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `unimplemented!`, `todo!`, or
  out-of-range indexing on any line you add. `unscaled_legacy()` is infallible.
  Note `geom_impls.rs` already contains a `mat.invert().unwrap()` near your
  site — it is not on your list, leave it.
- **H-2** Fallible operations return `Outcome<T>`. You are adding none.
- **H-3** No absolute constants in predicates. **`kernel-gates.sh` flags a bare
  float literal on any added line, and test epsilons trip it. The opt-out is a
  `// H-3` comment ON THE SAME LINE as the literal** — not the line above.
  rustfmt relocates such a comment off a line ending in `{`; if that happens,
  extract the literal onto its own statement line.
- **H-6** A value computed in floats is never recorded as `Method::Exact`.

## Tests required

New file `vendor/truck/truck-modeling/tests/tolerance_migration.rs`. Each must
be a named `#[test]` fn — the verifier checks the names appear in your diff.

1. `one_sided_range_guard_keeps_its_legacy_answers` — on
   `ToleranceCtx::unscaled_legacy()`, `t < t0 + ctx.ratio_margin()` agrees with
   `t < t0 + TOLERANCE` for a range of `t` that **includes values well below
   `t0`**, and show that `is_small_ratio(t - t0)` disagrees there. This is the
   one-sided trap, pinned. A test that only samples `t` near `t0` passes against
   the bug and is worthless.
2. `param_sites_are_unaffected_by_model_scale` — across several `model_scale`
   values, `ratio_margin()` and `is_small_ratio` are unchanged while
   `length_margin()` and `is_small_len` scale. This is the invariant every
   `param` classification in the table depends on.

Neither crate sets `autotests = false`, so a new file in `tests/` is picked up
automatically. Check `Cargo.toml` and report it if that is not what you find.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-topology -p truck-modeling
cargo clippy -p truck-topology -p truck-modeling --all-targets --no-deps -- -D warnings
cargo test -p truck-topology -p truck-modeling --lib --tests --no-fail-fast
cargo check --workspace --all-targets
```

`--no-deps` matters: without it clippy lints the vendored path dependencies too
and aborts before reaching your crates. Never run a bare `cargo test` — it
builds 56 examples. Send cargo output to a file and read the tail.

**Confirm the baseline before you edit anything.** Run the test command at the
base commit first and record which tests already fail; this tree has
pre-existing failures that are not yours. Report them in your notes and do not
try to fix them.

## Forbidden

Editing any file outside `write_allow` — in particular
`scripts/unscaled_legacy_ceiling.txt`, `truck-base/src/tolerance.rs`,
`truck-topology/src/{face,shell,solid,wire}.rs`, and **`loop/` anything: your
result file goes in the root of your worktree and nowhere else.** Changing any
function signature or `where` clause. Adding a `ctx` parameter. Changing any
threshold or the value of `LEADER_WITNESS_MARGIN`. Migrating any of the three
`FIXME` sites. Widening a tolerance or adding `#[ignore]` to make a test pass.
Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- a site does not typecheck under its assigned recipe → `SPEC_GAP`, naming the
  site and the actual types. Do not reclassify it to make it compile.
- adding a `FIXME` comment changes any count in the anchor table → say so; it
  should not, since the comments go on their own lines
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

```json
{"id":"BG-TOL-001-TOPO-MOD","status":"DONE","contracts":["BG-TOL-001"],
 "tests_added":2,"sites_migrated":6,"sites_fixmed":3,"unscaled_legacy_calls":0,
 "anchors_verified":{"A1":4,"A2":3,"A3":3,"A4":1,"A5":6,"A6":3},
 "notes":"set unscaled_legacy_calls to the number you actually introduced. Report the baseline test failures you confirmed, and anything about the one-sided rewrites that surprised you."}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`refactor(topology,modeling): classify every tolerance site model or param (BG-TOL-001-TOPO-MOD)`.
