# WORK PACKET BG-ANA-001-PS — exactly solvable pair: plane × sphere

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-ANA-001-PS","status":"DONE","contracts":["BG-ANA-001","BG-ANA-002"],
 "tests_added":5,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: if a decision below is wrong,
say so rather than working around it.

```yaml
id:          BG-ANA-001-PS
contract:    [BG-ANA-001, BG-ANA-002]
class:       mechanical
crates:      [truck-evidence]
depends_on:  [BG-CE-006-ENUM]
write_allow:
  - vendor/truck/truck-evidence/src/analytic/plane_sphere.rs
read_allow:
  - vendor/truck/truck-evidence/src/lib.rs
  - vendor/truck/truck-evidence/src/analytic/mod.rs
  - vendor/truck/truck-evidence/src/harness.rs
  - vendor/truck/truck-base/src/evidence.rs
  - vendor/truck/truck-geometry/src/specifieds/mod.rs
  - vendor/truck/truck-geometry/src/specifieds/plane.rs
  - vendor/truck/truck-geometry/src/specifieds/sphere.rs
  - vendor/truck/truck-geometry/src/specifieds/circle.rs
  - vendor/truck/truck-geometry/src/decorators/processor.rs
  - vendor/truck/truck-geometry/src/decorators/trimmied_curve.rs
tests_required:
  - ps_circle_lies_on_both_carriers
  - ps_great_circle_when_the_plane_passes_through_the_center
  - ps_tangent_point_and_empty_classify_exactly
  - ps_undecidable_predicates_refuse
  - ps_certificate_is_exact
budget:      {turns: 32, ctx_tokens: 85000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub enum AnalyticIntersection' vendor/truck/truck-evidence/src/analytic/mod.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub enum ExactCurve' vendor/truck/truck-evidence/src/analytic/mod.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub type PlacedCircle' vendor/truck/truck-evidence/src/analytic/mod.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub mod plane_sphere' vendor/truck/truck-evidence/src/analytic/mod.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'pub fn normal' vendor/truck/truck-geometry/src/specifieds/plane.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub const fn new(center: Point3, radius: f64) -> Sphere' vendor/truck/truck-geometry/src/specifieds/sphere.rs"}
  - {id: A7, expect: 1, cmd: "grep -c 'pub const fn center' vendor/truck/truck-geometry/src/specifieds/sphere.rs"}
  - {id: A8, expect: 1, cmd: "grep -c 'pub const fn radius' vendor/truck/truck-geometry/src/specifieds/sphere.rs"}
  - {id: A9, expect: 1, cmd: "grep -c 'pub const fn with_transform' vendor/truck/truck-geometry/src/decorators/processor.rs"}
  - {id: A10, expect: 1, cmd: "grep -c 'impl ParametricCurve for UnitCircle<Point3>' vendor/truck/truck-geometry/src/specifieds/circle.rs"}
```

## Problem

A plane cuts a sphere in a circle, touches it in a single tangent point, or
misses it. All three outcomes — and the tangency boundary between them — are
decided by one exact comparison: the signed distance from the sphere centre to
the plane against the radius. This packet classifies the pair exactly and emits
the closed-form circle. It is also the first shard that places a circle through
the shared `PlacedCircle` channel, so read `analytic/mod.rs`'s module docs and
its `#[cfg(test)]` module before writing anything — `TrimmedCurve` does **not**
remap its parameter; `subs(t)` takes the angle directly, and the module's own
tests assert that convention.

## Decisions already made for you

1. **One existing file**, `vendor/truck/truck-evidence/src/analytic/plane_sphere.rs`.
   It is already created and already declared as `pub mod plane_sphere;` in
   `analytic/mod.rs`, which is itself already declared in `lib.rs`. **Both
   `lib.rs` and `analytic/mod.rs` are read-only for you** — editing either is
   a scope violation that will get this packet rejected. The declarations and
   the shared result type were landed up front by the orchestrator so the
   eight sibling packets have disjoint write sets and can run in parallel;
   your file currently holds only a scaffolding doc comment, which you
   replace. The crate-level `#![deny(...)]` covers your module; do not add a
   second header.

2. **The shared result type is `crate::analytic::AnalyticIntersection` with
   `crate::analytic::{AnalyticOutcome, ExactCurve, PlacedCircle}` — read
   `analytic/mod.rs` first.** You do NOT define any result type of your own.
   Your public function is:

   ```rust
   pub fn plane_sphere(plane: &Plane, sphere: &Sphere) -> AnalyticOutcome
   ```

3. **The exact predicates: interval computation, three-way comparison,
   refusal.** Compute predicate quantities as `inari::Interval` (inari rounds
   outward), with named private helpers written exactly this way:

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
   site** — deliberately no helper (BG-EVD-002):

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

   Doc-comment what `Method::Exact` means here: the classification is exact
   (decisive interval predicates on the f64 carrier parameters), and the
   emitted curve is the closed-form intersection. Coordinates are computed in
   f64; the spec's obligation is "lies on both carriers to machine
   precision", asserted with an H-3-commented slack. No `τ_rep` anywhere.

5. **The classification algorithm, pre-decided:**

   1. `h = (sphere.center() − plane.origin()) · plane.normal()`, the signed
      distance, **in inari**. Compare `h*h` against `sphere.radius()²` with
      `three_way` (both inari intervals).
   2. `Less` (|h| < r) → **circle**. `Equal` (both degenerate, h² == r²) →
      **tangent point**. `Greater` → **empty**. `None` → refuse.
   3. Circle: centre `cc = sphere.center() − h_f * n̂` with `h_f` the f64
      value `(c − o)·n̂`; radius `ρ = sqrt(r² − h_f²)` in f64. Choose in-plane
      axes: `u` = the unit vector perpendicular to `n̂` obtained by crossing
      `n̂` with the least-aligned coordinate axis (write this tiny helper;
      pick the axis by comparing `|n̂.x|, |n̂.y|, |n̂.z|`), `v = n̂ × u`.
      Emit `ExactCurve::Circle` via the placement helper of decision 6.
   4. Tangent point: `p = c − h_f * n̂` (the foot of the perpendicular);
      emit `AnalyticIntersection::TangentPoint(p)`.

6. **Placing circles** — write this private helper in your own file:

   ```text
   fn frame(u: Vector3, v: Vector3, n: Vector3, o: Point3, ru: f64, rv: f64) -> Matrix4
   ```

   = `Matrix4::from_cols(Vector4::new(u.x, u.y, u.z, 0.0), Vector4::new(v.x,
   v.y, v.z, 0.0), Vector4::new(n.x, n.y, n.z, 0.0), Vector4::new(o.x, o.y,
   o.z, 1.0)) * Matrix4::from_nonuniform_scale(ru, rv, 1.0)`.
   A circle of radius `r` through `o` with in-plane unit axes `u`, `v`
   (`n = u × v`) is `Processor::with_transform(TrimmedCurve::new(
   UnitCircle::<Point3>::new(), (0.0, TAU)), frame(u, v, n, o, r, r))`.
   Sibling shards write their own copy of `frame`; that duplication is
   deliberate (disjoint write sets) and explicitly not a deviation — do not
   share it and do not report it.

## Tests required

All in the `#[cfg(test)]` module of `plane_sphere.rs`: named consts, and a
same-line `// H-3:` comment wherever a bare float slack literal appears.

1. `ps_circle_lies_on_both_carriers` — dyadic witness: plane z = 0, sphere
   centre (0, 0, 1), radius 5/4 (h = 1 < 5/4 → circle of radius 3/4 at the
   origin — every number dyadic). Sample the emitted circle (≥ 30 points over
   its parameter range) and assert `|p·ẑ| < slack` (on the plane) and
   `| |p − c| − r | < slack` (on the sphere), both H-3-commented
   dimensionless slacks of a unit-scale witness.
2. `ps_great_circle_when_the_plane_passes_through_the_center` — plane z = 0,
   sphere centred at the origin → circle of radius r in the plane z = 0,
   emitted radius equals `r` to machine precision.
3. `ps_tangent_point_and_empty_classify_exactly` — sphere (0,0,1) r = 1 vs
   plane z = 0 → `TangentPoint` at the origin; vs plane z = 2 → `TangentPoint`
   at (0,0,2); sphere (0,0,2) r = 1 vs plane z = 0 → `Empty`.
4. `ps_undecidable_predicates_refuse` — cover the refusal path by unit-testing
   the private comparator on hand-built inari intervals (a `[-w, w]` interval
   is neither decisively-zero nor excludes-zero; overlapping non-degenerate
   intervals give `three_way == None`). Additionally try one bit-neighbour
   witness (radius `f64::from_bits`-adjacent to the tangency value) and
   report in `notes` whether a genuine straddle refusal was constructible.
5. `ps_certificate_is_exact` — for a circle, a tangent and an empty outcome:
   every `Ok` carries `method == Method::Exact` and the `AnalyticCarrier`
   prop set to `Truth::True`.

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

Directions, angles, direction cosines, parameter values, residuals of
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
especially. Defining a private result enum. Changing the shared types, the
harness, or any carrier. Deciding a predicate by sampling the surfaces.
Returning an `Ok` arm chosen by an undecidable predicate. Adding `#[ignore]`.
Adding `unscaled_legacy(` call sites. Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- `Plane`/`Sphere` accessors do not supply what decision 5 needs → `SPEC_GAP`,
  naming exactly what is missing
- `inari` lacks a primitive this design needs → `SPEC_GAP`, naming it
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(evidence): exact plane × sphere (BG-ANA-001-PS)`.
