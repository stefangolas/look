# WORK PACKET BG-ANA-001-PCONE — exactly solvable pair: plane × cone

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-ANA-001-PCONE","status":"DONE","contracts":["BG-ANA-001","BG-ANA-002"],
 "tests_added":7,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: if a decision below is wrong,
say so rather than working around it.

```yaml
id:          BG-ANA-001-PCONE
contract:    [BG-ANA-001, BG-ANA-002]
class:       mechanical
crates:      [truck-evidence]
depends_on:  [BG-CE-006-ENUM]
write_allow:
  - vendor/truck/truck-evidence/src/analytic/plane_cone.rs
read_allow:
  - vendor/truck/truck-evidence/src/lib.rs
  - vendor/truck/truck-evidence/src/analytic/mod.rs
  - vendor/truck/truck-evidence/src/harness.rs
  - vendor/truck/truck-evidence/src/elementary.rs
  - vendor/truck/truck-base/src/evidence.rs
  - vendor/truck/truck-geometry/src/specifieds/mod.rs
  - vendor/truck/truck-geometry/src/specifieds/plane.rs
  - vendor/truck/truck-geometry/src/specifieds/cone.rs
  - vendor/truck/truck-geometry/src/specifieds/circle.rs
  - vendor/truck/truck-geometry/src/specifieds/hyperbola.rs
  - vendor/truck/truck-geometry/src/specifieds/parabola.rs
  - vendor/truck/truck-geometry/src/decorators/processor.rs
  - vendor/truck/truck-geometry/src/decorators/trimmied_curve.rs
tests_required:
  - pcone_horizontal_planes_cut_circles
  - pcone_vertical_plane_through_axis_two_lines
  - pcone_vertical_plane_two_hyperbola_branches
  - pcone_tilted_ellipse
  - pcone_boundary_parabola
  - pcone_through_apex_degenerates
  - pcone_certificate_is_exact
budget:      {turns: 45, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub enum AnalyticIntersection' vendor/truck/truck-evidence/src/analytic/mod.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub enum ExactCurve' vendor/truck/truck-evidence/src/analytic/mod.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub mod plane_cone' vendor/truck/truck-evidence/src/analytic/mod.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub fn normal' vendor/truck/truck-geometry/src/specifieds/plane.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'pub fn new(apex: Point3, half_angle: f64)' vendor/truck/truck-geometry/src/specifieds/cone.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub const fn apex' vendor/truck/truck-geometry/src/specifieds/cone.rs"}
  - {id: A7, expect: 1, cmd: "grep -c 'pub const fn half_angle' vendor/truck/truck-geometry/src/specifieds/cone.rs"}
  - {id: A8, expect: 1, cmd: "grep -c 'pub fn cos' vendor/truck/truck-evidence/src/elementary.rs"}
  - {id: A9, expect: 1, cmd: "grep -c 'pub fn sin' vendor/truck/truck-evidence/src/elementary.rs"}
```

## Problem

A plane cutting a cone produces the **conic sections** — this is their
definition. The cone of the specifieds is canonical: apex at `apex`, opening
along **+z**, half angle α, and **double-napped** (its v parameter is unbounded
both ways — read `cone.rs` to confirm; v < 0 is the lower nappe). The section
by a general plane is: a **circle** (plane ⊥ axis), an **ellipse** (plane
steeper than the generators, not through the apex), a **parabola** (plane
parallel to exactly one generator), a **hyperbola with two branches**
(one per nappe; plane shallower than the generators), or a degenerate: the
**apex point**, **one generator line**, or **two generator lines** (planes
through the apex). A horizontal plane always cuts the double cone — there is
no empty case for it.

**The technique, pre-decided: reduce to the plane's 2D coordinates and classify
the conic there.** Substitute the plane's parameterization `P(u, v) = o' +
u·û + v·v̂` into the cone's implicit equation and classify the resulting
quadratic `A u² + B uv + C v² + D u + E v + F = 0` by its invariants. One
code path produces every arm.

Read `analytic/mod.rs`'s module docs and `#[cfg(test)]` module first —
`TrimmedCurve` does **not** remap its parameter; `subs(t)` takes the
parameter directly, and the module's own tests assert that convention for the
circle. Read `hyperbola.rs` and `parabola.rs` for the unit conics' evaluation
and parameter ranges before placing them.

## Decisions already made for you

1. **One existing file**, `vendor/truck/truck-evidence/src/analytic/plane_cone.rs`.
   It is already created and already declared as `pub mod plane_cone;` in
   `analytic/mod.rs`, itself declared in `lib.rs`. **Both `lib.rs` and
   `analytic/mod.rs` are read-only for you** — editing either is a scope
   violation that will get this packet rejected. The declarations and the
   shared result type were landed up front by the orchestrator so the eight
   sibling packets have disjoint write sets and can run in parallel; your
   file currently holds only a scaffolding doc comment, which you replace.
   The crate-level `#![deny(...)]` covers your module; do not add a second
   header.

2. **The shared result type is `crate::analytic::AnalyticIntersection` with
   `crate::analytic::{AnalyticOutcome, ExactCurve, PlacedCircle, PlacedParabola,
   PlacedHyperbola}` — read `analytic/mod.rs` first.** You do NOT define any
   result type of your own. Your public function is:

   ```rust
   pub fn plane_cone(plane: &Plane, cone: &Cone) -> AnalyticOutcome
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
   emitted conic is the closed-form section. Coordinates are computed in f64;
   the spec's obligation is "lies on both carriers to machine precision",
   asserted with an H-3-commented slack. No `τ_rep` anywhere.

5. **The algorithm, pre-decided.** Let `n̂` = plane normal (f64 unit), `o` =
   plane origin, `p` = cone apex, `t = tan α` (f64; `tan` of the half angle —
   the cone's `der_mn` itself uses `half_angle().tan()`, so the carrier's own
   radial law is `ρ(z) = |z − p.z| · t`).

   **Build the 2D reduction.** Choose plane axes: `û` = unit in-plane
   perpendicular to the axis' horizontal shadow (concretely `û =
   normalize(ẑ × (n̂ × ẑ))` when `n̂` is not parallel to ẑ — this is the
   horizontal in-plane direction; if it is parallel, take `û = x̂`), and
   `v̂ = n̂ × û`. A plane point is `o + u·û + v·v̂`. Substituting into the
   cone equation `(x − p.x)² + (y − p.y)² = (z − p.z)² t²` gives the 2D
   quadratic's coefficients **as exact symbolic expressions in the carrier
   parameters** (each a polynomial in n̂'s, o's, p's coordinates and t;
   compute each coefficient as an inari interval from the f64 parameters —
   degree ≤ 2 polynomials, all inari arithmetic).

   **Classify in 2D, by the classical invariants**, all compared with the
   interval helpers:

   - `Δ2 = B² − 4AC` — the **type** discriminant: decisively `< 0` → ellipse
     family; decisively `> 0` → hyperbola family; decisively zero → parabola
     family; else refuse.
   - The **degeneracy**: the conic degenerates (to a point, one line, or two
     lines) exactly when the full quadratic form's determinant vanishes —
     equivalently here, when the **plane passes through the apex**: `h =
     (p − o) · n̂` **in inari**; `decisively_zero` → degenerate; `excludes_zero`
     → non-degenerate; else refuse. (For this carrier, apex-through is the
     only degeneracy — say so in a comment; it is the classical result that a
     plane through the apex of a non-degenerate cone degenerates the section.)

   **Emit:**

   - Ellipse family, non-degenerate: solve the 2D conic's centre (the linear
     system `2A u + B v + D = 0`, `B u + 2C v + E = 0` — Cramer in f64),
     rotate to principal axes (the `atan2(B, A − C)` eigenvector formula,
     f64), semi-axes from the translated quadratic's coefficients
     (`sqrt(−F'/λᵢ)` per eigenvalue — f64). If both semi-axes are equal
     within exact f64 equality of the squared values → `Circle` via the
     placement helper (radius = that value); else `Ellipse`. Sanity rule:
     if a squared semi-axis is decisively negative in inari, the reduction is
     wrong → `SPEC_GAP`, do not patch it silently.
   - Hyperbola family, non-degenerate → **two branches** → `TwoCurves([
     Hyperbola, Hyperbola])`: same centre/rotation solve, then place each
     branch of `UnitHyperbola` (read `hyperbola.rs` for its parameterization
     and range; trim to a symmetric finite range large enough for sampling —
     record the chosen range in `deviations` if there is no natural one).
   - Parabola family, non-degenerate → `Parabola(PlacedParabola)`: vertex and
     axis direction from the reduced 2D coefficients (translate to eliminate
     D, E as far as possible; the classic `(B u + 2C v)² = −4C F' (…)`
     reduction — derive it in a comment in the code and verify by sampling).
   - Degenerate + ellipse family (plane through apex, steeper than the
     generators) → `TangentPoint(apex)` — the section is the apex alone.
   - Degenerate + hyperbola family → `TwoCurves([Line, Line])`: the two
     generator lines through the apex lying in the plane — direction
     `(±sin α cos φ, ±sin α sin φ, cos α)` rotated to the plane's azimuth;
     solve which generators lie in the plane by requiring the direction to be
     perpendicular to `n̂` (a quadratic in the azimuth; solve in f64, verify
     decisively in inari, and derive both roots).
   - Degenerate + parabola family → `TangentLine`: exactly one generator lies
     in the plane; emit it as a `Line` through the apex.
   - The horizontal-plane special case (`n̂.z == ±1.0` exactly — a component
     test, no intervals) cuts a **circle** at height `o.z`: radius
     `|o.z − p.z| · t`, centre on the axis at `(p.x, p.y, o.z)`; if the plane
     passes through the apex (`o.z == p.z` exactly) → `TangentPoint(apex)`
     (a circle of radius zero). Handle it before the general reduction — it
     makes every number dyadic for the test.

6. **Placing conics** — write this private helper in your own file:

   ```text
   fn frame(u: Vector3, v: Vector3, n: Vector3, o: Point3, ru: f64, rv: f64) -> Matrix4
   ```

   = `Matrix4::from_cols(Vector4::new(u.x, u.y, u.z, 0.0), Vector4::new(v.x,
   v.y, v.z, 0.0), Vector4::new(n.x, n.y, n.z, 0.0), Vector4::new(o.x, o.y,
   o.z, 1.0)) * Matrix4::from_nonuniform_scale(ru, rv, 1.0)`.
   Circles and ellipses place the trimmed `UnitCircle` through it exactly as
   `analytic/mod.rs`'s tests show; the parabola and hyperbola place
   `TrimmedCurve<UnitParabola<Point3>>` / `TrimmedCurve<UnitHyperbola<Point3>>`
   with the same `frame` (their affine images under the 2D reduction's
   rotation and translation — the semi-axis scaling enters through `ru, rv`).
   Sibling shards write their own copy; that duplication is deliberate and
   explicitly not a deviation — do not share it and do not report it.

## Tests required

All in the `#[cfg(test)]` module of `plane_cone.rs`: named consts, and a
same-line `// H-3:` comment wherever a bare float slack literal appears.
Construct the cone through `Cone::new(apex, half_angle)` (an `Outcome` — no
unwrap, H-1). Prefer a half angle with **dyadic** trigonometry: α with
`sin α = 3/5, cos α = 4/5, tan α = 3/4` (`half_angle` value
`(3.0f64 / 5.0f64).asin()` — computed, not a literal).

1. `pcone_horizontal_planes_cut_circles` — cone apex at the origin, α as
   above; plane z = 2 (normal ±ẑ): circle centred (0, 0, 2) of radius
   `2 · (3/4) = 3/2` (dyadic). Sample ≥ 30 points; assert on the plane
   exactly and on the cone (`x² + y² == ((3/4) z)²` to machine precision,
   H-3-commented slack). Plane z = 0 through the apex → `TangentPoint(apex)`.
2. `pcone_vertical_plane_through_axis_two_lines` — plane y = 0 through the
   z axis: `TwoCurves([Line, Line])`; the generators through the apex in
   that plane have directions `(±3/5, 0, 4/5)`; assert the emitted lines'
   directions match (angle slack, H-3-commented) and sampled points satisfy
   both carriers.
3. `pcone_vertical_plane_two_hyperbola_branches` — plane x = 1 (normal ±x̂):
   `TwoCurves([Hyperbola, Hyperbola])` — the section `1 + y² = (3z/4)²` has
   two branches (z > 0 and z < 0 nappes). Sample within each branch's
   trimmed range; assert on-cone and on-plane to machine precision.
4. `pcone_tilted_ellipse` — a plane strictly steeper than the generators and
   not through the apex (e.g. through (0, 0, 4) with normal
   `normalize((1, 0, 1))`): `Ellipse` (or `Circle` only if the 2D semi-axes
   come out exactly equal — they will not here). Sample and assert
   on-both-carriers.
5. `pcone_boundary_parabola` — the parabola boundary is `|n̂.z| == sin α`
   with the plane not through the apex. Dyadic witness: α as above
   (`sin α = 3/5`), plane normal `n̂ = (4/5, 0, 3/5)` (unit, dyadic), plane
   through `(0, 0, 5)` so it clears the apex: → `Parabola`. Sample and
   assert on-both-carriers.
6. `pcone_through_apex_degenerates` — the three degenerate arms: ellipse
   family through the apex → `TangentPoint`; hyperbola family through the
   apex (e.g. plane y = 0) → two lines (test 2's assertion belongs there;
   here assert the arm); parabola family through the apex (normal
   `(4/5, 0, 3/5)`, plane through the origin) → `TangentLine`; sampled
   points of the emitted line lie on the cone.
7. `pcone_certificate_is_exact` — for a circle, a two-lines, a hyperbola-pair
   and a parabola outcome: every `Ok` carries `method == Method::Exact` and
   the `AnalyticCarrier` prop set to `Truth::True`.

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
file and read the tail. The existing 115 lib tests + 3 integration tests must
keep passing unchanged.

## Forbidden

Editing any file outside `write_allow` — `lib.rs` and `analytic/mod.rs`
especially. Defining a private result enum. Changing the shared types, the
harness, or any carrier. Deciding a predicate by sampling the surfaces.
Returning an `Ok` arm chosen by an undecidable predicate. Adding `#[ignore]`.
Adding `unscaled_legacy(` call sites. Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- the 2D reduction produces coefficients whose classification contradicts the
  closed-form witnesses above in a way you cannot correct within this design →
  `SPEC_GAP`, with the witness and the contradicting invariant values
- `UnitHyperbola`/`UnitParabola` cannot express a branch (parameterization or
  trimming insufficient) → `SPEC_GAP`, naming exactly what is missing
- `inari` lacks a primitive this design needs → `SPEC_GAP`, naming it
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(evidence): exact plane × cone (BG-ANA-001-PCONE)`.
