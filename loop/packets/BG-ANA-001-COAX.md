# WORK PACKET BG-ANA-001-COAX — exactly solvable pairs: coaxial families

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-ANA-001-COAX","status":"DONE","contracts":["BG-ANA-001","BG-ANA-002"],
 "tests_added":7,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: if a decision below is wrong,
say so rather than working around it.

```yaml
id:          BG-ANA-001-COAX
contract:    [BG-ANA-001, BG-ANA-002]
class:       mechanical
crates:      [truck-evidence]
depends_on:  [BG-CE-006-ENUM]
write_allow:
  - vendor/truck/truck-evidence/src/analytic/coaxial.rs
read_allow:
  - vendor/truck/truck-evidence/src/lib.rs
  - vendor/truck/truck-evidence/src/analytic/mod.rs
  - vendor/truck/truck-evidence/src/harness.rs
  - vendor/truck/truck-base/src/evidence.rs
  - vendor/truck/truck-geometry/src/specifieds/mod.rs
  - vendor/truck/truck-geometry/src/specifieds/cylinder.rs
  - vendor/truck/truck-geometry/src/specifieds/cone.rs
  - vendor/truck/truck-geometry/src/specifieds/sphere.rs
  - vendor/truck/truck-geometry/src/specifieds/torus.rs
  - vendor/truck/truck-geometry/src/specifieds/circle.rs
  - vendor/truck/truck-geometry/src/decorators/processor.rs
  - vendor/truck/truck-geometry/src/decorators/trimmied_curve.rs
tests_required:
  - coax_cylinder_sphere_two_circles
  - coax_cylinder_sphere_tangent_circle
  - coax_cone_sphere_inscribed_tangent_circle
  - coax_cylinder_torus_two_circles
  - coax_same_kind_pairs_classify_exactly
  - coax_undecidable_predicates_refuse
  - coax_certificate_is_exact
budget:      {turns: 42, ctx_tokens: 95000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub enum AnalyticIntersection' vendor/truck/truck-evidence/src/analytic/mod.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub enum ExactCurve' vendor/truck/truck-evidence/src/analytic/mod.rs"}
  - {id: A3, expect: 2, cmd: "grep -c 'TangentCircle' vendor/truck/truck-evidence/src/analytic/mod.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub mod coaxial' vendor/truck/truck-evidence/src/analytic/mod.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'pub fn new(center: Point3, radius: f64) -> Outcome<Self>' vendor/truck/truck-geometry/src/specifieds/cylinder.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub const fn apex' vendor/truck/truck-geometry/src/specifieds/cone.rs"}
  - {id: A7, expect: 1, cmd: "grep -c 'pub const fn half_angle' vendor/truck/truck-geometry/src/specifieds/cone.rs"}
  - {id: A8, expect: 1, cmd: "grep -c 'pub const fn center' vendor/truck/truck-geometry/src/specifieds/sphere.rs"}
  - {id: A9, expect: 1, cmd: "grep -c 'pub const fn large_radius' vendor/truck/truck-geometry/src/specifieds/torus.rs"}
  - {id: A10, expect: 1, cmd: "grep -c 'pub const fn small_radius' vendor/truck/truck-geometry/src/specifieds/torus.rs"}
```

## Problem

Every carrier in the specifieds is **canonical**: the cylinder runs along the z
axis through its centre, the cone opens along +z from its apex, the torus is
centred with its axis along z, and the sphere is free. A **coaxial pair** —
both carriers sharing the z axis — meets, when it meets at all, in **circles at
constant z**: the radial profile of each carrier is a function of z alone, and
circles happen where the two profiles are equal. This is the counterbore and
fillet family: every counterbore is coaxial, and coaxial tangency is decidable,
not approximated.

The algebra is uniform: matching the radial profiles reduces each pair to a
**linear or quadratic equation in z**, and the discriminant's three-way
comparison classifies 2 circles / 1 tangent circle / empty. The shared type's
`TangentCircle` arm exists for exactly this shard.

Read `analytic/mod.rs`'s module docs and `#[cfg(test)]` module first —
`TrimmedCurve` does **not** remap its parameter; `subs(t)` takes the angle
directly, and the module's own tests assert that convention.

## Decisions already made for you

1. **One existing file**, `vendor/truck/truck-evidence/src/analytic/coaxial.rs`.
   It is already created and already declared as `pub mod coaxial;` in
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
   Your public interface is an input enum plus one function:

   ```rust
   pub enum CoaxialPair<'a> {
       CylCyl(&'a Cylinder, &'a Cylinder),
       CylCone(&'a Cylinder, &'a Cone),
       CylSphere(&'a Cylinder, &'a Sphere),
       CylTorus(&'a Cylinder, &'a Torus),
       ConeCone(&'a Cone, &'a Cone),
       ConeSphere(&'a Cone, &'a Sphere),
       ConeTorus(&'a Cone, &'a Torus),
       SphereTorus(&'a Sphere, &'a Torus),
   }

   pub fn coaxial(pair: &CoaxialPair) -> AnalyticOutcome
   ```

   **Coaxiality is validated, not assumed**: add
   `CoaxialPair::validate(&self) -> Result<(), Refusal>` that refuses with
   `Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)` when the
   two carriers' axes are not the same z line (for the cylinder/cyl: the (x, y)
   of the centres differ; for anything with the cone or torus: the (x, y) of
   the apex/centre differ from the partner's). These are **exact f64 equality
   tests on point coordinates that ARE the carrier parameters** — no intervals
   needed and no tolerance; say so in the doc comment. `coaxial` calls
   `validate` first and propagates the refusal.

3. **The exact predicates: interval computation, three-way comparison,
   refusal.** Compute predicate quantities as `inari::Interval` (inari rounds
   outward), with named private helpers written exactly this way:

   - `decisively_zero(i) == (i.inf() == 0.0 && i.sup() == 0.0)`
   - `excludes_zero(i) == (i.inf() > 0.0 || i.sup() < 0.0)`
   - `three_way(a, b) -> Option<std::cmp::Ordering>`:
     `Some(Less)` iff `a.sup() < b.inf()`; `Some(Greater)` iff `b.sup() <
     a.inf()`; `Some(Equal)` iff both intervals are degenerate and identical;
     `None` otherwise.

   For this shard the workhorse is a **quadratic classifier**: given the
   intersection equation reduced to `A z² + B z + C = 0` with **interval**
   coefficients (each an inari enclosure of the real coefficient), compute the
   discriminant `Δ = B² − 4AC` in inari and classify: `excludes_zero` and
   positive → two roots (two circles); `decisively_zero` → one double root
   (**tangent circle** — the `TangentCircle` arm); decisively negative →
   `Empty`; otherwise → refuse. Degenerate `A ≈ 0` (linear equation): if A is
   `decisively_zero`, solve `B z + C = 0` (one circle if `B` excludes zero;
   if B also decisively zero: C decisively zero → `Coincident`, C excludes
   zero → `Empty`, else refuse); if A is neither → refuse. **Write this
   classifier once as a private helper and use it for every pair kind** — the
   whole shard is: reduce to the quadratic, classify, emit.

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
   emitted circles are the closed-form intersections. Coordinates are computed
   in f64; the spec's obligation is "lies on both carriers to machine
   precision", asserted with an H-3-commented slack. No `τ_rep` anywhere.

5. **The reductions, pre-derived — verify each by the on-both-carriers test
   before trusting it, and record corrections in `deviations`:**

   Radial profiles along z (all with z measured in world coordinates; `zc`,
   `za`, `zs`, `zt` the carriers' centre/apex z): cylinder `rc`; cone
   `|z − za| tan α`; sphere `√(rs² − (z − zs)²)`; torus `R ± √(rt² − (z −
   zt)²)` (outer and inner branches — the branch signs must be carried
   through the squaring below, see `SphereTorus`).

   - **CylCyl**: `rc0 == rc1` exact f64 → `Coincident`, else `Empty` (two
     coaxial cylinders never meet transversally). No intervals needed.
   - **CylCone**: `|z − za| tan α == rc` → **linear** in z (two signs of
     |·|: z = za ± rc/tanα — both are real circles; emit `TwoCurves` when
     tan α is decisively nonzero, `Empty` when `tan α` is decisively zero
     (degenerate cone), else refuse).
   - **CylSphere**: `rc² == rs² − (z − zs)²` → `(z − zs)² == rs² − rc²` —
     right side in inari, `three_way` against 0: positive → `TwoCurves`
     (z = zs ± √(rs² − rc²)); zero (degenerate) → `TangentCircle` at z = zs;
     negative → `Empty`; `None` → refuse.
   - **CylTorus**: outer/inner contacts both reduce (squaring once) to
     `(z − zt)² == rt² − (rc − R)²` — same equation for both branches because
     outer needs rc ≥ R and inner needs rc ≤ R (mutually exclusive). Compare
     the right side in inari against 0 exactly as CylSphere: positive →
     `TwoCurves`; zero → `TangentCircle` at z = zt; negative → `Empty`.
   - **ConeCone**: `|z − za0| tan α0 == |z − za1| tan α1` → piecewise linear;
     handle: same apex and same tan α (exact f64) → `Coincident`; same tan α,
     different apex → `Empty` (parallel cones never meet); otherwise solve
     the linear equation on each sign region and emit the solutions that lie
     in that region (0, 1 or 2 circles → `Empty` / `Curve` / `TwoCurves`).
     **The region test (z on the correct side of both apexes) is an exact f64
     comparison — z is a computed root; compare it in inari against the apex
     and refuse on straddle.**
   - **ConeSphere**: `√(rs² − (z − zs)²) == |z − za| tan α` → square:
     `rs² − (z − zs)² == (z − za)² tan² α` → **one quadratic** in z; classify
     by the discriminant helper; two roots → `TwoCurves`, double root →
     `TangentCircle` (the inscribed-sphere case), none → `Empty`. Squaring
     adds no spurious roots here because both sides are non-negative on the
     domains — say so in a comment.
   - **ConeTorus**: `(z − za) tan α == R ± √(rt² − (z − zt)²)` → square once:
     `((z − za) tan α − R)² == rt² − (z − zt)²` → **one quadratic** (take
     zt = za case or carry both through; the quadratic is in z either way);
     discriminant classifier; verify each emitted circle satisfies the
     **unsquared** branch equation (± sign) in the test, and drop roots that
     only solve the squared equation — with a comment saying why none exist
     or a `deviations` entry if you find some.
   - **SphereTorus**: substitute the sphere's radial into the torus equation;
     with `Δz = z − zs = z − zt` (validate they share z? no — the torus centre
     may sit at a different z than the sphere centre; **carry both z offsets
     through**: `√(rs² − (z − zs)²) − R` all squared equals `rt² − (z − zt)²`;
     this is a quadratic in z after one squaring — the z² terms cancel only
     when zs == zt, otherwise they survive; implement the general quadratic
     path and let the classifier handle it). Discriminant classifier as
     usual. Verify roots against the unsquared equation in tests (the squaring
     can introduce spurious roots when the sphere radial and the torus branch
     have opposite signs — check each root against `√(rs² − (z−zs)²) == R ±
     √(rt² − (z−zt)²)` **in inari**, decisively, and refuse if a root is
     undecidable).

   Emit every circle via the placement helper of decision 6 at height z with
   the matching radius.

6. **Placing circles** — write this private helper in your own file:

   ```text
   fn frame(u: Vector3, v: Vector3, n: Vector3, o: Point3, ru: f64, rv: f64) -> Matrix4
   ```

   = `Matrix4::from_cols(Vector4::new(u.x, u.y, u.z, 0.0), Vector4::new(v.x,
   v.y, v.z, 0.0), Vector4::new(n.x, n.y, n.z, 0.0), Vector4::new(o.x, o.y,
   o.z, 1.0)) * Matrix4::from_nonuniform_scale(ru, rv, 1.0)`.
   A coaxial circle of radius `r` at height `z` is
   `Processor::with_transform(TrimmedCurve::new(UnitCircle::<Point3>::new(),
   (0.0, TAU)), frame(x̂, ŷ, ẑ, Point3::new(x0, y0, z), r, r))` where
   `(x0, y0)` is the common axis position. Sibling shards write their own
   copy; that duplication is deliberate and explicitly not a deviation — do
   not share it and do not report it.

## Tests required

All in the `#[cfg(test)]` module of `coaxial.rs`: named consts, and a
same-line `// H-3:` comment wherever a bare float slack literal appears.
Construct carriers through their `new`s (cylinder/cone return `Outcome` — no
unwrap, H-1).

1. `coax_cylinder_sphere_two_circles` — cylinder r = 3/4 on the z axis,
   sphere centred at the origin r = 1: `rs² − rc² = 7/16` → two circles at
   z = ±√7/4 of radius 3/4. Sample both; every point satisfies
   `x² + y² == rc²` and `x² + y² + z² == rs²` to machine precision
   (H-3-commented slacks).
2. `coax_cylinder_sphere_tangent_circle` — cylinder r = 1, sphere r = 1 at
   the origin → `TangentCircle` at z = 0 of radius 1 (all dyadic). Assert the
   arm and sample the circle.
3. `coax_cone_sphere_inscribed_tangent_circle` — cone apex at the origin,
   half angle with `tan α = 3/4`; sphere centre (0, 0, 1), radius 3/5 (so
   `sin α = 3/5`): the discriminant is **exactly zero** and the tangent
   circle sits at z = 16/25 with radius 12/25 (all dyadic — derive it in the
   test as a comment and assert those exact values within an
   H-3-commented slack).
4. `coax_cylinder_torus_two_circles` — torus R = 2, rt = 1, centred at the
   origin; cylinder rc = 5/2: `(z)² == 1 − (1/2)² = 3/4` → two circles at
   z = ±√3/2 of radius 5/2. Sample; assert on both carriers. Add the tangent
   case rc = 3 → `TangentCircle` at z = 0.
5. `coax_same_kind_pairs_classify_exactly` — CylCyl equal radii →
   `Coincident`; different → `Empty`. ConeCone identical → `Coincident`;
   same angle different apex → `Empty`; different angles → one circle (pick
   dyadic tans, e.g. tan α0 = 1, tan α1 = 1/2, apexes both at the origin:
   |z| = |z|/2 has only z = 0 — radius 0! pick apexes apart: derive a
   witness with a nonzero circle and assert it).
6. `coax_undecidable_predicates_refuse` — unit-test the quadratic classifier
   directly on hand-built interval coefficients with a straddling
   discriminant (e.g. Δ interval `[-w, w]`) → refusal. Also
   `CoaxialPair::validate` refuses off-axis placements (exact-coordinate
   check).
7. `coax_certificate_is_exact` — for a two-circles, a tangent-circle and an
   empty outcome: every `Ok` carries `method == Method::Exact` and the
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
- a reduction in decision 5 is wrong in a way the on-both-carriers test
  catches and you cannot correct within the design → `SPEC_GAP`, with the
  witness and the failing sample
- `inari` lacks a primitive this design needs → `SPEC_GAP`, naming it
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(evidence): exact coaxial families (BG-ANA-001-COAX)`.
