# WORK PACKET BG-SOL-S5-CYLPAIR — Contact Layer FF: the cylinder-family analytic pairs

You are filling the next stage of the Contact Layer funnel (plan §4 Phase 3):
the **cylinder-family FF dispatch**. The skeleton (S3) landed the FF table
(plane/plane, plane/sphere, sphere/sphere, plane/cylinder, plane/cone) and
deferred everything else with `ContactReductionDeferred`; the strata-reduction
stage (S4) landed FE/EE. The remaining curved × curved FF cells that canonical
carriers make reachable — the `parallel_cylinders`, `equal_radius_cylinders`
and `coaxial` families of plan §3.3 — are still deferred. This packet wires
them into the FF dispatcher. Everything you need is in this document.
**Do not read any other spec file** — this packet is self-contained.

```json
{"id":"BG-SOL-S5-CYLPAIR","status":"DONE","contracts":["BG-SOL-S5-CYLPAIR"],
 "tests_added":6,"deviations":[],"disagreements":[],
 "baseline_failures":[],"notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. **If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-SOL-S5-CYLPAIR
class:       design
crates:      [truck-evidence, truck-geometry, truck-base]
write_allow:
  - vendor/truck/truck-evidence/src/contact/mod.rs
read_allow:
  - vendor/truck/truck-evidence/src/analytic/parallel_cylinders.rs
  - vendor/truck/truck-evidence/src/analytic/equal_radius_cylinders.rs
  - vendor/truck/truck-evidence/src/analytic/coaxial.rs
  - vendor/truck/truck-evidence/src/analytic/mod.rs
  - vendor/truck/truck-geometry/src/specifieds/cylinder.rs
  - vendor/truck/truck-geometry/src/specifieds/cone.rs
  - vendor/truck/truck-geometry/src/specifieds/sphere.rs
  - vendor/truck/truck-base/src/evidence.rs
tests_required:
  - contact_ff_cylinder_cylinder_parallel_returns_two_lines
  - contact_ff_cylinder_cylinder_coaxial_returns_empty
  - contact_ff_cylinder_cone_coaxial_returns_analytic
  - contact_ff_cylinder_sphere_coaxial_returns_analytic
  - contact_ff_cone_cone_coaxial_returns_analytic
  - contact_ff_non_coaxial_curved_pair_refuses_deferred
budget:      {turns: 60, ctx_tokens: 140000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'fn analytic_ff' vendor/truck/truck-evidence/src/contact/mod.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn parallel_cylinders' vendor/truck/truck-evidence/src/analytic/parallel_cylinders.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub fn coaxial' vendor/truck/truck-evidence/src/analytic/coaxial.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub enum CoaxialPair' vendor/truck/truck-evidence/src/analytic/coaxial.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'pub fn equal_radius_cylinders' vendor/truck/truck-evidence/src/analytic/equal_radius_cylinders.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub enum AnalyticIntersection' vendor/truck/truck-evidence/src/analytic/mod.rs"}
  - {id: A7, expect: 1, cmd: "grep -c 'pub enum EnvelopeCase' vendor/truck/truck-base/src/evidence.rs"}
  - {id: A8, expect: 1, cmd: "grep -c 'pub enum CanonicalSurface' vendor/truck/truck-geometry/src/recognize.rs"}
```

## Problem

The flagship differential test's RHS Boolean (the Boundary Rewrite, Phase 4)
dispatches every FF pair through `contact()`. S3 landed the plane-involved FF
cells; the curved × curved cells — cylinder × cylinder, cylinder × cone,
cylinder × sphere, cone × cone, cone × sphere — are the remaining analytic
families (plan §3.3 `parallel_cylinders`, `equal_radius_cylinders`, `coaxial`).
The canonical carriers are **z-axis-aligned** (a canonical `Cylinder` runs
along z through its `center`, a `Cone` along z through its `apex`, a `Sphere`
is centered), so every curved × curved pair of canonical carriers has
**parallel** axes; the pair is either **coaxial** (the axes coincide) or
parallel-but-offset. The same-axis cells are the `coaxial` family's job; the
offset cylinder × cylinder cell is `parallel_cylinders`. This packet wires the
reachable cells and defers the rest honestly.

## Design decisions already made for you

### 1. Where the change goes

The FF dispatch lives in the private `fn analytic_ff(l, r, budget)` inside
`vendor/truck/truck-evidence/src/contact/mod.rs` (the S3 skeleton, untouched by
S4). Its match on `(&CanonicalSurface, &CanonicalSurface)` currently ends with
the plane/plane … plane/cone arms, then two deferred arms:
`(Torus, _) | (_, Torus) | (Placed, _) | (_, Placed)` and the `_` catch-all,
both returning `Err(Refusal::UnsupportedEnvelope(
EnvelopeCase::ContactReductionDeferred))`. Extend this match — nothing else in
the module changes. `analytic_records` already maps every
`AnalyticIntersection` arm onto the 2-D ontology (`Curve`/`TwoCurves` → Arc1/
Transverse, `Tangent*` → Arc1/Tangency, `Parallel`/`Empty` → no contact,
`Coincident` → Region2/CoincidentInterval), so no vocabulary work is needed.

### 2. The coaxiality predicate

Two canonical curved carriers are **coaxial** iff their axis positions are
exactly equal: the `(x, y)` of the two `Cylinder::center`s, of a
`Cylinder::center` and a `Cone::apex`, of a `Cone::apex` and another
`Cone::apex`, and (for a sphere) the sphere's `center` `(x, y)` against the
other carrier's axis position. Use **exact f64 equality**, matching
`CoaxialPair::validate` (coaxial.rs lines 79-101), which documents: "These are
exact f64 equality tests on point coordinates that ARE the carrier parameters
… No intervals are needed and no tolerance is applied: either the axis
positions are exactly equal or the pair is not coaxial." A pair that is
1-ulp-apart in `x` is not coaxial, and the parallel-cell answer (for
cylinder × cylinder) or the deferred refusal (for the mixed pairs) is the
correct one for it. Write one small helper per axis-kind (e.g.
`cyl_cyl_coaxial`, `cyl_cone_coaxial`, `cone_sphere_coaxial`) or a single
helper taking the two axis positions; your choice, but the predicate must be
the exact `(x, y)` equality.

### 3. The dispatch table — exactly this, nothing more

Add these arms to `analytic_ff`'s match (both orientations for the asymmetric
pairs):

| ordered pair | predicate | result |
|---|---|---|
| `(Cylinder, Cylinder)` | axes equal | `coaxial(&CoaxialPair::CylCyl(a, b))` |
| `(Cylinder, Cylinder)` | axes unequal | `parallel_cylinders(a, b)` |
| `(Cylinder, Cone)` / `(Cone, Cylinder)` | axes equal | `coaxial(&CoaxialPair::CylCone(a, b))` |
| `(Cylinder, Cone)` / `(Cone, Cylinder)` | axes unequal | deferred |
| `(Cylinder, Sphere)` / `(Sphere, Cylinder)` | sphere center on axis | `coaxial(&CoaxialPair::CylSphere(a, b))` |
| `(Cylinder, Sphere)` / `(Sphere, Cylinder)` | off-axis | deferred |
| `(Cone, Cone)` | apexes equal | `coaxial(&CoaxialPair::ConeCone(a, b))` |
| `(Cone, Cone)` | apexes unequal | deferred |
| `(Cone, Sphere)` / `(Sphere, Cone)` | sphere center on axis | `coaxial(&CoaxialPair::ConeSphere(a, b))` |
| `(Cone, Sphere)` / `(Sphere, Cone)` | off-axis | deferred |

"deferred" is `Err(Refusal::UnsupportedEnvelope(
EnvelopeCase::ContactReductionDeferred))`, the same arm the current catch-all
returns. **Do not translate the `coaxial` function's own
`NonCanonicalCarrier` refusal** — the dispatch predicate above guarantees
`validate` passes, so a `NonCanonicalCarrier` can only mean a bug; propagate it
rather than hide it.

**`equal_radius_cylinders` is NOT wired.** It solves the equal-radius
cylinders with **intersecting axes** cell, which canonical carriers cannot
reach: every canonical `Cylinder`/`Cone` axis is the z direction, so two of
them are always parallel, never intersecting. That cell needs a rotated
(`Placed`) cylinder, and the funnel defers `Placed` faces. Document this in
your RESULT notes as the reason the family stays out of the FF table (it
remains the analytic-cell oracle for BG-NUM-003, its intended role).

**Torus stays deferred.** Any pair involving `Torus` keeps hitting the
existing `(Torus, _) | (_, Torus)` deferred arm. Do not add torus arms.

### 4. Certificate construction

Unchanged from the current `analytic_ff`: after `let Certified { value, .. } =
outcome?;`, the existing `analytic_records(&value)` + explicit field-by-field
`Certificate { props: {Prop::AnalyticCarrier: True}, method: Method::Exact,
budget_left: *budget, margin: Margin::UNBOUNDED, modulus: Modulus::Unbounded
}`. Nothing is spent from `budget`. The `coaxial` and `parallel_cylinders`
functions return `AnalyticOutcome`; their own certificates are discarded the
same way the landed FF arms discard the analytic pairs' (the `Certified {
value, .. }` pattern).

### 5. Tests (in `contact/mod.rs` tests)

House rule: GATE-1 requires `#![deny(clippy::unwrap_used)]` — the module header
already carries it and the test module already has the
`#[allow(clippy::unwrap_used, clippy::expect_used)]`. Build the strata with the
existing `face(surface)` helper (`BoundedStratum::Face` with the unit
`(u, v)` box — the FF stage ignores the box). Build the carriers directly from
`CanonicalSurface::Cylinder(Cylinder::new(center, radius).expect(...).value)`
and the `coaxial` test module's dyadic cone convention
(`Cone::new(apex, tan_value.atan())` — `tan(atan(3/4)) == 3/4` exactly in
f64, and likewise 1/2). Assert with `matches!` on the record's
`dimension`/`kind`/`locus`; no float literals without `// H-3` on the same
line. The six required tests:

1. `contact_ff_cylinder_cylinder_parallel_returns_two_lines` — two offset
   parallel cylinders, e.g. `Cylinder::new((0,0,0), 1.0)` and
   `Cylinder::new((1.5, 0, 0), 1.0)` (axis distance 1.5, strictly between
   `r0+r1 = 2` and `|r0−r1| = 0`) → exactly one record,
   `Arc1` / `Transverse`, `ContactLocus::Analytic(AnalyticIntersection::
   TwoCurves([ExactCurve::Line(_), ExactCurve::Line(_)]))`.
2. `contact_ff_cylinder_cylinder_coaxial_returns_empty` — two coaxial
   cylinders of different radii, e.g. `(0,0,0), 1.0` and `(0,0,0), 2.0`
   (struct-unequal, so the C0-C2 identity stage cannot fire) → `Ok` with an
   empty `contacts` vec (`coaxial(CylCyl)` → `Empty`).
3. `contact_ff_cylinder_cone_coaxial_returns_analytic` — a cylinder
   `(0,0,0), r=1` and a cone `apex (0,0,0), tan = 3/4` (coaxial; the cone's
   lateral surface meets the cylinder's in the circle `z = 4/3, r = 1`) →
   exactly one record, `Arc1`, `ContactLocus::Analytic(_)`.
4. `contact_ff_cylinder_sphere_coaxial_returns_analytic` — a cylinder
   `(0,0,0), r=1` and a sphere `center (0,0,0), r=2` (the wall circle
   `x²+y²=1` lies in the sphere at `z² = 3`) → at least one record with
   `dimension = Arc1` and `ContactLocus::Analytic(_)`.
5. `contact_ff_cone_cone_coaxial_returns_analytic` — cones
   `apex (0,0,0), tan 3/4` and `apex (0,0,1), tan 1/2` (coaxial, different
   angles; they meet in two circles — the `TwoCurves` arm, as the coaxial
   module's own test proves) → one record, `Arc1` / `Transverse`,
   `ContactLocus::Analytic(AnalyticIntersection::TwoCurves(_))`.
6. `contact_ff_non_coaxial_curved_pair_refuses_deferred` — a non-coaxial
   curved pair, e.g. cylinder `(0,0,0), r=1` × cone `apex (1,0,0), tan 3/4`
   (offset axes) and a non-coaxial cylinder × sphere, e.g. cylinder
   `(0,0,0), r=1` × sphere `center (2,0,0), r=2` → both refuse with
   `Err(Refusal::UnsupportedEnvelope(EnvelopeCase::
   ContactReductionDeferred))`.

Also keep the landed S3 and S4 tests green (they are untouched), and add a
commutativity assertion inside test 3 (or a seventh test of your own naming):
`contact(lhs=cone, rhs=cylinder)` and `contact(lhs=cylinder, rhs=cone)` must
produce structurally equal `ContactComplex` values (the metamorphic property).

## Done-when gates

```
cargo fmt --check -p truck-evidence
cargo clippy -p truck-evidence --all-targets --no-deps
cargo test -p truck-evidence --lib --tests --no-fail-fast
cargo check --locked -p truck-evidence --all-targets
cargo check --locked -p truck-base --all-targets
bash scripts/kernel-gates.sh <your base commit>
```

Never run a bare `cargo test`. Never run `cargo check --workspace` — it
exhausts disk on a shared machine.

## H-3 / GATE-4

GATE-2 rejects added lines carrying bare `1e-N` literals unless the line ends
with `// H-3`. Float tolerances in test assertions must carry `// H-3` on the
same line. This packet adds NO `unscaled_legacy()` calls; do not touch
`scripts/unscaled_legacy_ceiling.txt` (GATE-4 stays at 111).

## Forbidden

Editing any file outside `write_allow`. Editing the FE/EE machinery
(`contact/fe_ee.rs`), the `coaxial`/`parallel_cylinders`/`equal_radius_cylinders`
modules, `truck-evidence/src/lib.rs`, or any topology/modelling file. Wiring
`equal_radius_cylinders`. Adding torus or `Placed` FF arms. Implementing the
deferred funnel stages (general validated FF, singular event cells, 2-D
overlap). Changing the `(dimension, kind, locus)` shape of `ContactRecord`.
Adding `#[ignore]`. Changing the GATE-4 ceiling. Running `cargo check
--workspace` / `cargo build --workspace`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. Record in
`notes`: the dispatch table actually landed (which ordered pairs hit which
family), the coaxiality predicate (exact `(x, y)` equality, matching
`CoaxialPair::validate`), the `equal_radius_cylinders` exclusion reason (canonical
axes are parallel by construction — the intersecting-axes cell needs `Placed`
cylinders, which the funnel defers), the certificate shape, and your read of
whether any in-scope representation was infeasible.

Commit on the current branch with subject
`feat(evidence): Contact Layer FF — cylinder-family analytic pairs (parallel/coaxial) (BG-SOL-S5-CYLPAIR)`.
