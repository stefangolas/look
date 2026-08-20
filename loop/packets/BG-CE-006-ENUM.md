# WORK PACKET BG-CE-006-ENUM — one canonical Curve/Surface model

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-CE-006-ENUM","status":"DONE","contracts":["BG-CE-006"],
 "tests_added":4,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: if you find a decision below
is wrong — especially a claim about the current tree — say so rather than
working around it.

```yaml
id:          BG-CE-006-ENUM
contract:    [BG-CE-006]
class:       design
crates:      [truck-geometry, truck-modeling, truck-stepio]
depends_on:  [BG-CE-006-CYLINDER, BG-CE-006-CONE]
write_allow:
  - vendor/truck/truck-geometry/src/canonical.rs
  - vendor/truck/truck-geometry/src/lib.rs
  - vendor/truck/truck-modeling/src/geometry.rs
  - vendor/truck/truck-modeling/src/lib.rs
  - vendor/truck/truck-modeling/src/builder.rs
  - vendor/truck/truck-stepio/src/out/geometry.rs
read_allow:
  - vendor/truck/truck-geometry/src/specifieds/
  - vendor/truck/truck-geometry/src/decorators/
  - vendor/truck/truck-base/src/evidence.rs
  - vendor/truck/truck-base/src/tolerance.rs
  - vendor/truck/truck-stepio/src/out/
  - scripts/kernel-gates.sh
tests_required:
  - tsweep_circle_yields_cylinder
  - circle_conversion_preserves_variant
  - extruded_noncanonical_circle_degrades
  - stepio_out_emits_analytic_entities
budget:      {turns: 60, ctx_tokens: 120000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub enum Curve' vendor/truck/truck-modeling/src/geometry.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub enum Surface' vendor/truck/truck-modeling/src/geometry.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'Surface::RevolutedCurve(Processor::new' vendor/truck/truck-modeling/src/geometry.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'impl ToSameGeometry<Curve> for Processor<TrimmedCurve<UnitCircle' vendor/truck/truck-modeling/src/geometry.rs"}
  - {id: A5, expect: 1, cmd: "grep -c '_ => unreachable!' vendor/truck/truck-modeling/src/geometry.rs"}
  - {id: A6, expect: 8, cmd: "grep -c 'ModelingSurface::' vendor/truck/truck-stepio/src/out/geometry.rs"}
  - {id: A7, expect: 1, cmd: "grep -c 'pub struct Cylinder' vendor/truck/truck-geometry/src/specifieds/mod.rs"}
  - {id: A8, expect: 1, cmd: "grep -c 'pub struct Cone' vendor/truck/truck-geometry/src/specifieds/mod.rs"}
  - {id: A9, expect: 1, cmd: "grep -c 'pub struct Sphere' vendor/truck/truck-geometry/src/specifieds/mod.rs"}
  - {id: A10, expect: 1, cmd: "grep -c 'pub struct Torus' vendor/truck/truck-geometry/src/specifieds/mod.rs"}
```

## Problem

Truck has two competing curve/surface models. `truck-modeling/src/geometry.rs`
defines `Curve` and `Surface` enums whose `Surface` holds only
`Plane | BSplineSurface | NurbsSurface | RevolutedCurve` — it silently drops the
analytic carriers `Cylinder`, `Cone`, `Sphere`, `Torus` that exist in
`truck-geometry/src/specifieds/`, so every operation that flows through the
modeling layer degrades analytic geometry to splines (a placed circle becomes a
NURBS at conversion, and extruding it becomes a homotopy B-spline surface — the
cylinder-ness is destroyed). This packet makes **one** canonical model, owned by
`truck-geometry`, and turns `truck-modeling`'s copy into a re-export.

This is the kernel's one breaking data-model release. Nothing else is in
flight; take the break cleanly.

## Decisions already made for you

Every judgement is made here. You execute, you do not design.

1. **New file `vendor/truck/truck-geometry/src/canonical.rs`.** Declare
   `pub mod canonical;` in `truck-geometry/src/lib.rs` and re-export its public
   names through the crate's existing prelude block in that same file. The file
   starts with the mandatory lint header (a gate checks new modules):

   ```rust
   #![deny(
       clippy::unwrap_used,
       clippy::expect_used,
       clippy::panic,
       clippy::todo,
       clippy::unimplemented,
       clippy::indexing_slicing
   )]
   ```

2. **The enums move, with payload-naming kept.** Move `pub enum Curve`, `pub
   enum Surface`, and every impl currently in `truck-modeling/src/geometry.rs`
   (the two `macro_rules!` delegation macros, `Transformed`, `ParametricSurface3D`,
   `IncludeCurve`, `SearchNearestParameter`, `lift_up`, all `ToSameGeometry`
   impls, `plane_include_intersection_curve`, and the pure-geometry test
   modules) into `canonical.rs`, then extend them to exactly this shape:

   ```rust
   pub enum Curve {
       Line(Line<Point3>),
       /// analytic circle: a placed (possibly full-range) trimmed unit circle
       Circle(Processor<TrimmedCurve<UnitCircle<Point3>>, Matrix4>),
       BSplineCurve(BSplineCurve<Point3>),
       NurbsCurve(NurbsCurve<Vector4>),
       IntersectionCurve(IntersectionCurve<Box<Curve>, Box<Surface>, Box<Surface>>),
   }

   pub enum Surface {
       Plane(Plane),
       Cylinder(Cylinder),
       Cone(Cone),
       Sphere(Sphere),
       Torus(Torus),
       /// REVOLUTED payload loses the legacy identity `Processor` wrapper:
       /// the sole construction site wrapped with `Processor::new` (identity).
       RevolutedCurve(RevolutedCurve<Curve>),
       /// RESERVED: no conversion emits this yet (BG-CE-007 will); the variant
       /// exists now so this release is the last breaking one. Tessellation and
       /// STEP-out must still handle it (see decisions 8 and 9).
       ExtrudedCurve(ExtrudedCurve<Curve, Vector3>),
       BSplineSurface(BSplineSurface<Point3>),
       NurbsSurface(NurbsSurface<Vector4>),
   }
   ```

   Keep the existing variant names (`Revolved`/`Extruded`/`BSpline`/`Nurbs`
   short names from the design sketch were NOT adopted — payload-naming is the
   existing convention of `Curve` and renaming every construction site buys
   nothing). Keep the existing derive lists unchanged; the new payloads
   already implement everything the derives require (verified: all four
   analytic specifieds implement `ParametricSurface`, `ParameterDivision2D`,
   `Invertible`, `SearchParameterD2`, `SearchNearestParameterD2`).

3. **`truck-modeling/src/geometry.rs` becomes a shim.** It keeps its module
   doc, then `pub use truck_geometry::canonical::*;` plus whatever re-exports
   it currently provides (`truck_geometry::prelude::{algo, inv_or_zero}` etc.
   stay). Its import path `truck_modeling::geometry::Surface` and
   `truck_modeling::{Curve, Surface}` (used by truck-stepio) must keep
   resolving — that is the point of the shim. **One test stays in this file**:
   `boolean_derived_face_consistency_returns` from
   `include_intersection_curve_tests` constructs `Vertex`/`Edge`/`Wire`/`Face`
   (truck-topology types); `truck-geometry` must not depend on
   `truck-topology`, so that single test cannot move. Move the rest of that
   module's tests and the whole `extrude_intersection_curve_tests` module to
   `canonical.rs`.

4. **The circle conversion stops degrading.** The existing impl

   `impl ToSameGeometry<Curve> for Processor<TrimmedCurve<UnitCircle<Point3>>, Matrix4>`

   currently returns `Curve::NurbsCurve(...)`. It now returns
   `Curve::Circle(self.clone())`. (The variant's payload is exactly the type
   this impl consumes — that is deliberate.)

5. **Analytic preservation on extrusion, with an exact representability rule.**
   In `impl ToSameGeometry<Surface> for ExtrudedCurve<Curve, Vector3>`, the
   `_ => unreachable!()` arm dies. Replace the match so that:

   - `(Line, Line)` → `Plane` — unchanged, exists today.
   - `(Circle(c0), Circle(c1))` → attempt `Cylinder`: let `M = c0.transform()`
     (the `Processor` accessor returning `&Matrix4`), with columns
     `m1, m2, m3` and translation `t`; let `v` be the extruding vector. Emit
     `Cylinder::new(t, m1.magnitude())` **only if all of these hold exactly**
     (no epsilon — see below): `m1.z == 0.0 && m2.z == 0.0` (the circle's
     plane is horizontal), `m3.x == 0.0 && m3.y == 0.0` (no tilt), 
     `m1.magnitude() == m2.magnitude() && m1.dot(m2) == 0.0` (uniform,
     unskewed scale — else the "circle" is an ellipse), `m1.magnitude() > 0.0`,
     and `v.x == 0.0 && v.y == 0.0` (extrusion along ±z). If
     `Cylinder::new` refuses (it validates the radius), or any condition
     fails, **fall through to the NurbsCurve homotopy arm** — which is
     exactly today's behaviour for every circle, so a near-miss placement
     degrades, never mis-carries. Rationale for exact comparison:
     z-preserving placements are built from z-rotations and translations and
     compare exactly in practice; anything else was already degrading
     yesterday. `curve1` is `curve0` translated by `v`, so testing `c0`
     decides the pair.
   - spline pairs and the `(IntersectionCurve, IntersectionCurve)` refusal —
     unchanged.
   - `Circle` needs a `lift_up` arm: reuse today's degradation (the
     non-rationalized lift of the same placed circle the old conversion
     produced). `lift_up` is infallible and must stay so.

6. **The `Revolved` payload change.** `impl ToSameGeometry<Surface> for
   RevolutedCurve<Curve>` returns `Surface::RevolutedCurve(self.clone())` —
   no `Processor::new`. `RevolutedCurve` carries its own
   `origin`/`axis` (`RevolutedCurve::by_revolution`), and the removed wrapper
   was identity at the only construction site. Everywhere the old code called
   `.entity()` / `.entity_curve()` on the wrapper, call the method on the
   payload directly. `truck-modeling/src/builder.rs` has one wildcarded match
   on `Surface` (`partial_torus`): adapt its `RevolutedCurve` arm to the bare
   payload and **change nothing else in that file** beyond the new test module
   of decision 10.

7. **`IncludeCurve<Curve> for Surface`:** the four analytic arms
   (`Cylinder`, `Cone`, `Sphere`, `Torus`) return
   `Err(Refusal::NumericallyUnresolved { spent: Budget::new(0,0,0), witness:
   UnresolvedWitness::UncertifiedContainment })` — the same honest refusal the
   non-`Plane` arms already use. Certified curve-in-analytic-surface is
   BG-CE-002/BG-ENC work and is NOT yours. The `ExtrudedCurve` arm: same
   refusal. Extend `SearchNearestParameter<D2> for Surface` by direct
   delegation to the payload (all five new payloads implement it themselves).

8. **STEP-out.** In `truck-stepio/src/out/geometry.rs` the two exhaustive
   `ModelingSurface` matches (the `DisplayByStep` impl and the `step_length`
   impl) gain arms. `Cylinder`, `Cone`, `Sphere`, `Torus` emit
   `CYLINDRICAL_SURFACE`, `CONICAL_SURFACE`, `SPHERICAL_SURFACE`,
   `TOROIDAL_SURFACE`; `Curve::Circle` emits `CIRCLE`. Write the
   `DisplayByStep` impls for the four specifieds following the existing
   emitter pattern for analytic entities in that file (placement entities via
   the same helpers the current code uses). Two notes:
   - The cone: `CONICAL_SURFACE`'s point set is the complete cone for any
     positive reference radius; the reference radius only fixes the u=0
     circle. Emit `tan(half_angle)` as the reference radius at the apex
     placement (positive unless the cone itself is degenerate, which
     `Cone`'s constructor already refuses). Say this in a comment at the arm.
   - The reserved `ExtrudedCurve` arm: not reachable from any conversion
     today; emit the B-spline homotopy of the entity curve — the same surface
     the pre-packet conversion would have produced — with a comment saying
     so. It must not panic and must not be `unimplemented!`.
   - `step_length` arms: follow the existing pattern (`Plane::LENGTH` /
     payload `step_length()`); use the payload's own step length where it has
     one, an analytic constant where it does not.
   **STEP-in is NOT in this packet.** Imported cylinders stay NURBS; do not
   touch `truck-stepio/src/in/`.

9. **No tolerance-predicate work.** You add zero new
   `unscaled_legacy(` call sites — production or test. The moved code carries
   its existing calls (moving is ratchet-neutral). Tests compare geometry
   with `assert_near!` (already the house macro) or exact assertions.

## Tests required

All new tests go in `canonical.rs`'s test modules except the sweep test, which
goes in a new `#[cfg(test)]` module in `truck-modeling/src/builder.rs`.

1. `tsweep_circle_yields_cylinder` — build the canonical placed circle (unit
   `UnitCircle<Point3>` in an identity-placement `Processor`, full-range
   `TrimmedCurve`), extrude along +z through the same path `tsweep` uses, and
   assert the result is `Surface::Cylinder(_)` **and** that its point set
   agrees with the NURBS construction (what the old conversion produced) at
   sampled parameters, within the house tolerance.
2. `circle_conversion_preserves_variant` — the conversion of decision 4
   returns `Curve::Circle`, and the variant's evaluated points equal the old
   degradation's points at sampled parameters.
3. `extruded_noncanonical_circle_degrades` — a tilted or ellipse-scaling
   placement extruded along a non-z vector produces a valid surface (not a
   `Cylinder`, not a panic), again point-equal to today's behaviour at
   sampled parameters.
4. `stepio_out_emits_analytic_entities` — emit STEP for each of the four
   analytic surfaces and the circle, assert the output text contains
   `CYLINDRICAL_SURFACE`, `CONICAL_SURFACE`, `SPHERICAL_SURFACE`,
   `TOROIDAL_SURFACE` and `CIRCLE`.

**H-3 applies to your added lines**: no bare absolute float literals. Name
constants for what they are, or take the same-line `// H-3` opt-out **on the
same line as the literal** (rustfmt moves a trailing comment off a
brace-opening line — extract the literal to its own statement line if that
happens). Test fixtures need many literals; the house style is named `const`
items at module top with a word on what each quantity is.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-geometry -p truck-modeling -p truck-stepio
cargo clippy -p truck-geometry -p truck-modeling -p truck-stepio --all-targets --no-deps -- -D warnings
cargo test -p truck-geometry --lib --tests --no-fail-fast
cargo test -p truck-modeling --lib --tests --no-fail-fast
cargo test -p truck-stepio --lib --tests --no-fail-fast
cargo check --workspace --all-targets
```

Never run a bare `cargo test` — it builds 56 examples. Send cargo output to a
file and read the tail. The moved tests (the BG-S0-001 and BG-S0-003 modules)
must keep passing unchanged — if one moves, something is wrong with your
move, not with the test. A pre-existing failure in a file you did not touch:
confirm it fails identically at the base commit, record it in
`baseline_failures`, and report it.

## Forbidden

Editing any file outside `write_allow` (if something outside it fails to
compile, that is a stop condition, not an edit). Renaming existing variants.
Changing any signature or refusal behaviour of moved code. Emitting
`Surface::ExtrudedCurve` from any conversion. Touching `truck-stepio/src/in/`.
Adding `unscaled_legacy(` call sites. Adding `#[ignore]`. Weakening or
deleting a moved test. Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- **a file outside `write_allow` fails to compile** → `SPEC_GAP`, naming the
  file and the error. The ripple analysis said this cannot happen; if it does,
  the packet is wrong and you must not fix it by widening your own scope.
- **you find a second construction site of `Surface::RevolutedCurve` with a
  non-identity `Processor`** (decision 6 claims there is none) → `SPEC_GAP`
  with the site; do not silently keep the wrapper.
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(geometry): one canonical Curve/Surface model (BG-CE-006-ENUM)`.
