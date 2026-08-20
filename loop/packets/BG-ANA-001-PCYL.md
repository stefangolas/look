# WORK PACKET BG-ANA-001-PCYL — exactly solvable pair: plane × cylinder

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-ANA-001-PCYL","status":"DONE","contracts":["BG-ANA-001","BG-ANA-002"],
 "tests_added":6,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: if a decision below is wrong,
say so rather than working around it.

```yaml
id:          BG-ANA-001-PCYL
contract:    [BG-ANA-001, BG-ANA-002]
class:       mechanical
crates:      [truck-evidence]
depends_on:  [BG-CE-006-ENUM]
write_allow:
  - vendor/truck/truck-evidence/src/analytic/plane_cylinder.rs
read_allow:
  - vendor/truck/truck-evidence/src/lib.rs
  - vendor/truck/truck-evidence/src/analytic/mod.rs
  - vendor/truck/truck-evidence/src/harness.rs
  - vendor/truck/truck-base/src/evidence.rs
  - vendor/truck/truck-geometry/src/specifieds/mod.rs
  - vendor/truck/truck-geometry/src/specifieds/plane.rs
  - vendor/truck/truck-geometry/src/specifieds/cylinder.rs
  - vendor/truck/truck-geometry/src/specifieds/circle.rs
  - vendor/truck/truck-geometry/src/decorators/processor.rs
  - vendor/truck/truck-geometry/src/decorators/trimmied_curve.rs
tests_required:
  - pcyl_two_lines_when_the_plane_is_parallel_to_the_axis
  - pcyl_tangent_line_and_empty_when_parallel
  - pcyl_circle_when_the_plane_is_perpendicular
  - pcyl_ellipse_when_tilted
  - pcyl_undecidable_predicates_refuse
  - pcyl_certificate_is_exact
budget:      {turns: 36, ctx_tokens: 90000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub enum AnalyticIntersection' vendor/truck/truck-evidence/src/analytic/mod.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub enum ExactCurve' vendor/truck/truck-evidence/src/analytic/mod.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub type PlacedCircle' vendor/truck/truck-evidence/src/analytic/mod.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub mod plane_cylinder' vendor/truck/truck-evidence/src/analytic/mod.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'pub fn normal' vendor/truck/truck-geometry/src/specifieds/plane.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub fn new(center: Point3, radius: f64) -> Outcome<Self>' vendor/truck/truck-geometry/src/specifieds/cylinder.rs"}
  - {id: A7, expect: 1, cmd: "grep -c 'pub const fn center' vendor/truck/truck-geometry/src/specifieds/cylinder.rs"}
  - {id: A8, expect: 1, cmd: "grep -c 'pub const fn radius' vendor/truck/truck-geometry/src/specifieds/cylinder.rs"}
  - {id: A9, expect: 1, cmd: "grep -c 'pub const fn with_transform' vendor/truck/truck-geometry/src/decorators/processor.rs"}
```

## Problem

The canonical `Cylinder` of the specifieds runs along the **z axis** through
its `center` (read `cylinder.rs` — this is BG-CE-006's canonical form). A plane
cuts it in: **two lines** (plane parallel to the axis, offset inside), **one
tangent line** (parallel, offset exactly r), **a circle** (plane perpendicular
to the axis), **an ellipse** (any other tilt), or **nothing** (parallel,
outside). Which one is decided by the axis-normal angle and the offset — both
exact predicates on the carrier parameters. This packet implements all five.

Note the geometry of the perpendicular case carefully: a plane whose normal is
parallel to ẑ is pierced by the (infinite) axis exactly once, so it **always**
cuts a circle — there is no empty perpendicular case.

Read `analytic/mod.rs`'s module docs and `#[cfg(test)]` module first —
`TrimmedCurve` does **not** remap its parameter; `subs(t)` takes the angle
directly, and the module's own tests assert that convention.

## Decisions already made for you

1. **One existing file**, `vendor/truck/truck-evidence/src/analytic/plane_cylinder.rs`.
   It is already created and already declared as `pub mod plane_cylinder;` in
   `analytic/mod.rs`, itself declared in `lib.rs`. **Both `lib.rs` and
   `analytic/mod.rs` are read-only for you** — editing either is a scope
   violation that will get this packet rejected. The declarations and the
   shared result type were landed up front by the orchestrator so the eight
   sibling packets have disjoint write sets and can run in parallel; your file
   currently holds only a scaffolding doc comment, which you replace. The
   crate-level `#![deny(...)]` covers your module; do not add a second header.

2. **The shared result type is `crate::analytic::AnalyticIntersection` with
   `crate::analytic::{AnalyticOutcome, ExactCurve, PlacedCircle}` — read
   `analytic/mod.rs` first.** You do NOT define any result type of your own.
   Your public function is:

   ```rust
   pub fn plane_cylinder(plane: &Plane, cylinder: &Cylinder) -> AnalyticOutcome
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

   Dyadic-clean inputs produce degenerate intervals, so exact classifications
   stay exact; an enclosure that merely contains zero proves nothing.

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
   f64; the spec's obligation is "lies on both carriers to machine precision",
   asserted with an H-3-commented slack. No `τ_rep` anywhere.

5. **The classification algorithm, pre-decided.** `n̂ = plane.normal()` (f64
   unit), `o = plane.origin()`, `c = cylinder.center()`, `r = cylinder.radius()`.
   The tilt predicate is the **component** `a = n̂.z` — an exact f64 value, no
   arithmetic; test it directly:

   1. `a == 1.0 || a == -1.0` (exactly) → **perpendicular** → circle: the
      axis meets the plane at `t = ((o − c) · n̂) / (n̂ · ẑ)` in f64 (with
      `n̂·ẑ = a`), centre `cc = c + t·ẑ`, radius `r`. Emit
      `Curve(ExactCurve::Circle)` via the placement helper (decision 6) with
      in-plane axes `u = x̂`, `v = ŷ` (any in-plane pair works for a circle;
      these are exact).
   2. `a == 0.0` (exactly) → **parallel**: the offset from the axis to the
      plane `δ = (c − o) · n̂` **in inari**, compare `δ²` with `r²` by
      `three_way`:
      - `Less` → **two lines**: in f64, the in-plane direction perpendicular
        to both `n̂` and ẑ is `û = normalize(n̂ × ẑ)`; the foot
        `f = c − δ_f n̂` (δ_f the f64 value); half-chord
        `s = sqrt(r² − δ_f²)`; the lines are `f + s·û` and `f − s·û`
        extruded along ẑ — emit `TwoCurves([ExactCurve::Line(p−, p−+ẑ),
        ExactCurve::Line(p+, p++ẑ)])` (two-point `Line` values; any nonzero
        extent along ẑ is fine, use `ẑ` itself).
      - `Equal` (both degenerate) → **tangent line**: `TangentLine(Line(f,
        f + ẑ))`.
      - `Greater` → `Empty`.
      - `None` → refuse.
   3. Otherwise (`0 < |a| < 1` strictly — `a` is an exact component, so this
      is decided without intervals) → **ellipse**: centre where the axis
      pierces the plane, `t = ((o − c) · n̂) / a` (f64), `cc = c + t ẑ`;
      minor semi-axis `r` along `û = normalize(n̂ × ẑ)` (horizontal, ⊥ the
      tilt plane); major semi-axis `r / |a|` along `v̂ = n̂ × û` (in-plane,
      ⊥ `û`, pointing "uphill"). Check the orientation claim numerically in
      the tilted test before trusting it; record any correction in
      `deviations`. Emit `Curve(ExactCurve::Ellipse)` via the placement
      helper with `ru = r / |a|` along `v̂` and `rv = r` along `û` (mind
      which axis of `frame` takes which semi-axis — verify by sampling).

6. **Placing circles and ellipses** — write this private helper in your own
   file:

   ```text
   fn frame(u: Vector3, v: Vector3, n: Vector3, o: Point3, ru: f64, rv: f64) -> Matrix4
   ```

   = `Matrix4::from_cols(Vector4::new(u.x, u.y, u.z, 0.0), Vector4::new(v.x,
   v.y, v.z, 0.0), Vector4::new(n.x, n.y, n.z, 0.0), Vector4::new(o.x, o.y,
   o.z, 1.0)) * Matrix4::from_nonuniform_scale(ru, rv, 1.0)`.
   A circle of radius `r` through `o` with in-plane unit axes `u`, `v`
   (`n = u × v`) is `Processor::with_transform(TrimmedCurve::new(
   UnitCircle::<Point3>::new(), (0.0, TAU)), frame(u, v, n, o, r, r))`; an
   ellipse uses `ru ≠ rv`. Sibling shards write their own copy; that
   duplication is deliberate and explicitly not a deviation — do not share it
   and do not report it.

## Tests required

All in the `#[cfg(test)]` module of `plane_cylinder.rs`: named consts, and a
same-line `// H-3:` comment wherever a bare float slack literal appears.
Construct `Cylinder` through `Cylinder::new(center, radius)`, which returns an
`Outcome` — build witnesses with `.map(|c| c.value)` or match; do not unwrap
(H-1).

1. `pcyl_two_lines_when_the_plane_is_parallel_to_the_axis` — cylinder at the
   origin r = 1 (radius exactly 1), plane x = 3/5 (normal ±x̂, origin
   (3/5, 0, 0)): δ = 3/5 < 1 → two lines at x = 3/5, y = ±4/5 (the 3-4-5
   triple — every coordinate dyadic). Sample both lines; assert every point
   satisfies `(x − cx)² + (y − cy)² == r²` to machine precision and the plane
   equation exactly-ish (H-3-commented slacks).
2. `pcyl_tangent_line_and_empty_when_parallel` — plane x = 1 → `TangentLine`
   (the line x = 1, y = 0); plane x = 2 → `Empty`.
3. `pcyl_circle_when_the_plane_is_perpendicular` — plane z = 3 (normal ±ẑ) →
   circle centred (cx, cy, 3) of radius r; sample and assert on-cylinder and
   on-plane.
4. `pcyl_ellipse_when_tilted` — the plane through the origin spanned by
   `(0,0,0), (0,1,0), (1,0,1)` has normal ∝ (−1, 0, 1)/√2 — a decisive 45°
   tilt (|a| = 1/√2 strictly between 0 and 1). Assert `Ellipse`; sample the
   emitted ellipse (≥ 30 points) and assert every point is on the cylinder
   (`(x−cx)² + (y−cy)² == r²` to machine precision) and on the plane. Also
   assert the semi-axes ratio is 1/cos(45°) = √2 within an H-3-commented
   slack.
5. `pcyl_undecidable_predicates_refuse` — unit-test the private comparator on
   hand-built inari intervals (a `[-w, w]` interval is neither
   decisively-zero nor excludes-zero; overlapping non-degenerate intervals
   give `three_way == None`); try one bit-neighbour parallel-offset witness
   and report in `notes` whether a genuine straddle refusal was constructible.
6. `pcyl_certificate_is_exact` — for a two-lines, a circle and an ellipse
   outcome: every `Ok` carries `method == Method::Exact` and the
   `AnalyticCarrier` prop set to `Truth::True`.

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
- `Plane`/`Cylinder` accessors do not supply what decision 5 needs →
  `SPEC_GAP`, naming exactly what is missing
- `inari` lacks a primitive this design needs → `SPEC_GAP`, naming it
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(evidence): exact plane × cylinder (BG-ANA-001-PCYL)`.
