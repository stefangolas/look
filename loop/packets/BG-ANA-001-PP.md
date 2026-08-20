# WORK PACKET BG-ANA-001-PP — exactly solvable pair: plane × plane

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-ANA-001-PP","status":"DONE","contracts":["BG-ANA-001","BG-ANA-002"],
 "tests_added":5,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: if a decision below is wrong,
say so rather than working around it.

```yaml
id:          BG-ANA-001-PP
contract:    [BG-ANA-001, BG-ANA-002]
class:       mechanical
crates:      [truck-evidence]
depends_on:  [BG-CE-006-ENUM]
write_allow:
  - vendor/truck/truck-evidence/src/analytic/plane_plane.rs
read_allow:
  - vendor/truck/truck-evidence/src/lib.rs
  - vendor/truck/truck-evidence/src/analytic/mod.rs
  - vendor/truck/truck-evidence/src/harness.rs
  - vendor/truck/truck-base/src/evidence.rs
  - vendor/truck/truck-geometry/src/specifieds/mod.rs
  - vendor/truck/truck-geometry/src/specifieds/plane.rs
tests_required:
  - pp_transverse_line_lies_on_both_planes
  - pp_parallel_and_coincident_classify_exactly
  - pp_coincident_through_different_point_triples
  - pp_undecidable_predicates_refuse
  - pp_certificate_is_exact
budget:      {turns: 30, ctx_tokens: 80000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub enum AnalyticIntersection' vendor/truck/truck-evidence/src/analytic/mod.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub enum ExactCurve' vendor/truck/truck-evidence/src/analytic/mod.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub mod plane_plane' vendor/truck/truck-evidence/src/analytic/mod.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub const fn new(origin: Point3, one: Point3, another: Point3)' vendor/truck/truck-geometry/src/specifieds/plane.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'pub fn normal' vendor/truck/truck-geometry/src/specifieds/plane.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub const fn origin' vendor/truck/truck-geometry/src/specifieds/plane.rs"}
```

## Problem

Two planes intersect in a line, or they are parallel (no intersection), or they
coincide (the intersection is the whole plane — not a curve). This packet
classifies the pair **exactly** and, in the transverse case, emits the exact
line. The result is not a float-certified approximation: the classification is
decided by exact predicates on the carrier parameters, and the line is the
closed-form solution of the two plane equations.

This is the reference shard for the eight-packet analytic family: it is the
smallest, and the decisions below (shared result type, comparator, certificate)
are the ones every sibling copies. Read them carefully; the siblings will not
restate them as fully.

## Decisions already made for you

1. **One existing file**, `vendor/truck/truck-evidence/src/analytic/plane_plane.rs`.
   It is already created and already declared as `pub mod plane_plane;` in
   `vendor/truck/truck-evidence/src/analytic/mod.rs`, which is itself already
   declared in `lib.rs`. **Both `lib.rs` and `analytic/mod.rs` are read-only
   for you** — neither is on your `write_allow`, and editing either is a scope
   violation that will get this packet rejected. The declarations and the
   shared result type were landed up front by the orchestrator precisely so
   the eight sibling packets have disjoint write sets and can run in parallel;
   your file currently holds only a scaffolding doc comment, which you
   replace. The crate-level `#![deny(...)]` in `lib.rs` covers your module;
   do not add a second header.

2. **The shared result type is `crate::analytic::AnalyticIntersection` with
   `crate::analytic::{AnalyticOutcome, ExactCurve}` — read `analytic/mod.rs`
   first.** You do NOT define any result type of your own; a private enum
   shadowing the shared one is a rejection. Your public function is:

   ```rust
   pub fn plane_plane(plane0: &Plane, plane1: &Plane) -> AnalyticOutcome
   ```

3. **The exact predicates: interval computation, three-way comparison,
   refusal.** Every position predicate compares real quantities derived from
   the carrier parameters. Compute each quantity as an `inari::Interval`
   (inari rounds outward for you), with a **named private comparator section**
   written exactly this way:

   - `decisively_zero(i) == (i.inf() == 0.0 && i.sup() == 0.0)`
   - `excludes_zero(i) == (i.inf() > 0.0 || i.sup() < 0.0)`
   - `three_way(a, b) -> Option<std::cmp::Ordering>`:
     `Some(Less)` iff `a.sup() < b.inf()`; `Some(Greater)` iff `b.sup() <
     a.inf()`; `Some(Equal)` iff both intervals are degenerate and identical;
     `None` otherwise.

   Why `decisively_zero` requires degeneracy: an inari enclosure of a dot
   product that is exactly zero only through cancellation is a wide-ish
   `[-ulp, +ulp]`, and claiming it proves zero is exactly the
   wrong-but-confident answer BG-ANA-002 forbids. Dyadic-clean inputs produce
   degenerate intervals, so exact classifications stay exact.

   **Undecidable is a stop, not a guess:** return
   `Err(Refusal::NumericallyUnresolved { spent: Budget::new(0, 0, 0), witness:
   UnresolvedWitness::RootNotIsolated })`. Return `Ok` only when every
   predicate that chose the returned arm was decisive.

4. **Every `Ok` carries the exact certificate, field-by-field at every return
   site** — deliberately no helper (BG-EVD-002: "exact" cannot be manufactured
   casually):

   ```rust
   let mut props = PropMap::new();
   props.set(Prop::AnalyticCarrier, Truth::True);
   Certified::new(
       value,
       Certificate {
           props,
           method: Method::Exact,
           budget_left: Budget::new(0, 0, 0),
           margin: Margin::UNBOUNDED,
           modulus: Modulus::Unbounded,
       },
   )
   ```

   Doc-comment what `Method::Exact` means here, precisely: the
   *classification* is exact — decided by decisive interval predicates on the
   f64 carrier parameters — and the emitted curve is the closed-form solution
   of the two plane equations. Curve coordinates are computed in f64; the
   spec's obligation is "lies on both carriers to machine precision", which
   your on-both-carriers test asserts with an H-3-commented slack. No `τ_rep`
   is attached anywhere.

5. **The classification algorithm, pre-decided:**

   1. `n0 = plane0.normal()`, `n1 = plane1.normal()` (f64 unit vectors), `o0`,
      `o1` the origins.
   2. The normal cross product `c = n0 × n1`, computed **per component in
      inari**. If any component `excludes_zero` → **transverse**; if all three
      are `decisively_zero` → **parallel**; otherwise → undecidable → refuse.
   3. Transverse → `Ok(AnalyticIntersection::Curve(ExactCurve::Line(line)))`.
      The line: direction `d = (n0 × n1).normalize()` in f64. A point on both
      planes — use the standard closed form

      ```text
      p = ( (o0·n0) (n1 − (n0·n1) n0) + (o1·n1) (n0 − (n0·n1) n1) ) / (1 − (n0·n1)²)
      ```

      (note `o·n` here means the plane's constant, `origin()·normal()`), and
      emit `Line(p, p + d)` — `Line` is the two-point struct from
      `truck_geometry::specifieds`, re-exported through `analytic/mod.rs`'s
      imports. **Verify the formula yourself numerically before committing
      it** (your transverse test does this); if you find it wrong, fix it and
      record that in `deviations`.
   4. Parallel → the offset `h = (o1 − o0) · n0` **in inari**:
      `decisively_zero` → `Coincident`; `excludes_zero` → `Parallel`;
      otherwise → refuse.

## Tests required

All in the `#[cfg(test)]` module of `plane_plane.rs`, in the style of the
existing carriers (`circle.rs`, `plane.rs`-adjacent tests): named consts, and
a same-line `// H-3:` comment wherever a bare float slack literal appears.

1. `pp_transverse_line_lies_on_both_planes` — witnesses with dyadic points:
   `Plane::xy()`-style planes built by hand from axis-aligned points (e.g.
   z = 0 through the origin, y = 0 through the origin → the x-axis; plus one
   generic pair like the planes through the origin spanned by
   `(0,0,0),(1,0,1),(0,1,1)` and `(0,0,0),(1,1,0),(1,0,0)`). For each,
   sample the emitted line over its parameter range (≥ 30 samples) and assert
   every point satisfies both plane equations to machine precision:
   `|(p − oᵢ)·nᵢ| < slack` (H-3-commented, dimensionless — plane residuals of
   a unit-scale witness). Also assert the direction is perpendicular to both
   normals.
2. `pp_parallel_and_coincident_classify_exactly` — z = 0 vs z = 2 →
   `Parallel`; a plane vs itself (identical three points) → `Coincident`.
3. `pp_coincident_through_different_point_triples` — the same geometric plane
   built from two different point triples (shift the points along in-plane
   directions) → `Coincident`. This pins that the predicate uses the offset
   between the planes, not the representation points.
4. `pp_undecidable_predicates_refuse` — a bit-level straddle witness is not
   constructible for plane normals (say so in a comment if you try and fail);
   cover the refusal path instead by unit-testing the private comparator
   directly on hand-built inari intervals: a `[-w, w]` interval is neither
   decisively-zero nor excludes-zero; overlapping non-degenerate intervals
   give `three_way == None`. Assert those.
5. `pp_certificate_is_exact` — for a transverse, a parallel, and a coincident
   pair: every `Ok` carries `method == Method::Exact`, the `AnalyticCarrier`
   prop set to `Truth::True`, and no other props.

## H-3, which is what rejected three packets in this family

GATE-2 fails any **added** line carrying a bare `1e-N` literal unless that same
line ends with an `// H-3` comment. It is a text gate on the diff: it does not
know your literal is an angle, and it does not care that the line is in a test.
`BG-ENC-002-LINE` was rejected for one such line and `BG-ENC-002-CIRCLE` for
six, both times on assertion epsilons in tests, both times costing a verify.

So: **every comparison epsilon you write gets a same-line `// H-3:` comment
naming the dimensionless quantity being compared.** The house form:

    assert!((a - b).magnitude() < 1.0e-12, ...); // H-3: float slack between two unit direction vectors, not a length
    assert!((h - expected).abs() < 1.0e-12, ...); // H-3: float slack between two half-angles in radians, not a length
    assert!(cos_angle >= limit - 1.0e-12, ...);   // H-3: float slack between two direction cosines, not a length

Directions, angles, direction cosines, parameter values, plane residuals of
unit-scale witnesses and interval bounds are all dimensionless and all
legitimate — the comment is what says so. A literal that really is a
model-space *length* does not get an opt-out; it goes through `ToleranceCtx`
instead. Run `bash scripts/kernel-gates.sh` yourself before you write
`RESULT.json`; it is the same script V4 runs.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-evidence
cargo clippy -p truck-evidence --all-targets --no-deps -- -D warnings
cargo test -p truck-evidence --lib --tests --no-fail-fast
cargo check --workspace --all-targets
```

Never run a bare `cargo test` — it builds 56 examples. Send cargo output to a
file and read the tail. The existing 74 lib tests + 3 integration tests must
keep passing unchanged.

## Forbidden

Editing any file outside `write_allow` — `lib.rs` and `analytic/mod.rs`
especially, both of which are already correct. Defining a private result enum
or re-exporting the shared one under a new name. Changing the shared types,
the harness, or any carrier. Deciding a predicate by sampling the surfaces
(BG-ANA-002's explicit prohibition). Returning an `Ok` arm chosen by an
undecidable predicate. Adding `#[ignore]`. Adding `unscaled_legacy(` call
sites. Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- `Plane`'s accessors do not supply what decision 5 needs (anchors pass but the
  described accessors do not exist) → `SPEC_GAP`, naming exactly what is missing
- `inari` lacks a primitive this design needs → `SPEC_GAP`, naming it; do not
  hand-roll directed rounding
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(evidence): exact plane × plane (BG-ANA-001-PP)`.
