# WORK PACKET BG-SOL-P0-REC — the structural recognizer: a witness, not a type

You are implementing the solver family's structural recognition layer: a
`recognize_curve`/`recognize_surface` pair that answers "what canonical
analytic carrier is this stored surface or curve, and what certified parameter
correspondence φ maps the stored parameterization onto the canonical one"
(`S_stored = S_canonical ∘ φ`). Everything you need is in this document.
**Do not read any other spec file** — this packet is self-contained. It
implements the approved design in `docs/SOLVER_FAMILY_PLAN.md` §2 and §4
(Phase 0, `truck-geometry` module `recognize`). The scaffold already moved
`ParamMap` to `truck_base::param_map` (re-exported by `truck-evidence`) so
this packet can name it.

```json
{"id":"BG-SOL-P0-REC","status":"DONE","contracts":["BG-SOL-P0-REC"],
 "tests_added":5,"deviations":[],"disagreements":[],
 "baseline_failures":[],"notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. **If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-SOL-P0-REC
class:       design
crates:      [truck-geometry]
write_allow:
  - vendor/truck/truck-geometry/src/recognize.rs
read_allow:
  - vendor/truck/truck-geometry/src/canonical.rs
  - vendor/truck/truck-base/src/param_map.rs
tests_required:
  - recognize_line_and_plane_are_exact_canonical
  - recognize_extruded_line_is_plane
  - recognize_extruded_circle_is_cylinder
  - recognize_skew_or_degenerate_extrude_is_unrecognized
  - recognize_processor_places_the_inner_carrier
budget:      {turns: 70, ctx_tokens: 160000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub mod recognize' vendor/truck/truck-geometry/src/lib.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub struct ParamMap' vendor/truck/truck-base/src/param_map.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub use truck_base::param_map::ParamMap' vendor/truck/truck-evidence/src/deviation.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'clippy::unwrap_used' vendor/truck/truck-geometry/src/recognize.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'pub enum Curve' vendor/truck/truck-geometry/src/canonical.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub enum Surface' vendor/truck/truck-geometry/src/canonical.rs"}
```

## Problem

The solver family's ladder (plan §1) is: recognize structure aggressively,
solve in the simplest representation, escalate only when necessary. The
recognizer is the first rung. A B-rep face whose carrier is an
`ExtrudedCurve(Circle, +z)` homotopy NURBS is REALLY a cylinder; a profile
edge stored as a spline arc is REALLY a circle. The recognizer answers "what
canonical carrier is this, and how do I parametrize it canonically", producing
**a witness, not a type** (plan §2): coincidence (S5.0) becomes a lookup on
the witness, never a re-solve. When it cannot decide, `Unrecognized` is a
result, and the caller treats the carrier as a spline — a regular validated
solver path, not a failure.

Phase-0 scope (record in `disagreements`): the canonical set is the analytic
arms of `Curve`/`Surface` (`Line`, `Circle` for curves; `Plane`, `Cylinder`,
`Cone`, `Sphere`, `Torus` for surfaces), plus the two derived constructions M1
needs — `ExtrudedCurve` of a line/circle, and a `Processor`-placed analytic
carrier. Exact spline→analytic detection (a NURBS circle whose control points
lie on a circle) is a **documented later packet**; for now splines are
`Unrecognized`. `RevolutedCurve` recognition (line→cylinder/cone) lands with
S2's `revolve_profile`, which is where it is consumed.

## Design decisions already made for you

### 1. The witness types — decide nothing, type them exactly

```rust
/// A canonical analytic curve carrier: the analytic arms of `Curve`.
#[derive(Clone, Debug, PartialEq)]
pub enum CanonicalCurve {
    /// A line.
    Line(Line<Point3>),
    /// A placed analytic circle.
    Circle(Processor<TrimmedCurve<UnitCircle<Point3>>, Matrix4>),
}

/// A canonical analytic surface carrier: the analytic arms of `Surface`.
#[derive(Clone, Debug, PartialEq)]
pub enum CanonicalSurface {
    Plane(Plane),
    Cylinder(Cylinder),
    Cone(Cone),
    Sphere(Sphere),
    Torus(Torus),
    /// A canonical analytic carrier composed with an affine placement. The
    /// bare carriers (bare `Cylinder`, `Cone`, …) are z-axes-only; a rotated
    /// analytic carrier is representable only as `Placed` (the canonical.rs
    /// `Processor` rule). Exact under affine.
    Placed(Processor<Box<CanonicalSurface>, Matrix4>),
}

/// A canonical carrier: curve or surface.
#[derive(Clone, Debug, PartialEq)]
pub enum CanonicalCarrier {
    Curve(CanonicalCurve),
    Surface(CanonicalSurface),
}

/// How a derived canonical carrier was obtained.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConstructionWitness {
    /// The carrier is the stored surface's analytic inner carrier under an
    /// affine placement.
    Placed,
    /// The carrier is obtained by sweeping a canonical profile curve.
    Extruded,
}

/// The certified parameter correspondence φ with `S_stored = S_canonical ∘ φ`.
/// The plan's §4 single `ParamMap` is the curve case; a surface needs the
/// (u, v) pair, so the correspondence is a two-armed sum (record this
/// deviation in `disagreements`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CanonicalParamMap {
    Curve(ParamMap),
    Surface { u: ParamMap, v: ParamMap },
}

/// The structural recognizer's witness (plan §2).
#[derive(Clone, Debug, PartialEq)]
pub enum CanonicalCarrierWitness {
    /// `S_stored` IS the canonical carrier under φ (a directly-canonical
    /// variant, φ = IDENTITY).
    ExactCanonical { carrier: CanonicalCarrier, map: CanonicalParamMap },
    /// `S_stored = S_canonical ∘ φ` by construction, `provenance` says how.
    Derived { carrier: CanonicalCarrier, provenance: ConstructionWitness, map: CanonicalParamMap },
    /// No canonical carrier recognized; treat as a generic spline carrier.
    Unrecognized,
}

/// Recognize the canonical carrier of a stored curve.
pub fn recognize_curve(c: &Curve) -> CanonicalCarrierWitness;

/// Recognize the canonical carrier of a stored surface.
pub fn recognize_surface(s: &Surface) -> CanonicalCarrierWitness;
```

`Line`, `Plane`, `Cylinder`, `Cone`, `Sphere`, `Torus`, `UnitCircle`,
`Processor`, `TrimmedCurve`, `Curve`, `Surface` come from `crate::*`;
`ParamMap` from `truck_base::param_map`; `Matrix4`/`Point3`/`Vector3`/
`Vector4` from `crate::prelude::*`. The module denies `clippy::indexing_slicing`
— read matrices/vectors with fields, not `[]` (the house H-1 rule).

### 2. Recognition rules — `recognize_curve`, exactly these

- `Curve::Line(l)` → `ExactCanonical { carrier: Curve(CanonicalCurve::Line(*l)),
  map: CanonicalParamMap::Curve(ParamMap::IDENTITY) }`.
- `Curve::Circle(p)` → `ExactCanonical { carrier: Curve(CanonicalCurve::Circle(*p)),
  map: CanonicalParamMap::Curve(ParamMap::IDENTITY) }`.
- `Curve::BSplineCurve(_)`, `Curve::NurbsCurve(_)`,
  `Curve::IntersectionCurve(_)` → `Unrecognized` (spline→analytic detection is
  a documented later packet; the profile builders emit `Line`/`Circle`
  directly).

### 3. Recognition rules — `recognize_surface`

- `Surface::Plane(p)` → `ExactCanonical { carrier: Surface(Plane(*p)),
  map: Surface { u: IDENTITY, v: IDENTITY } }`. Same for `Cylinder`, `Cone`,
  `Sphere`, `Torus`.
- `Surface::BSplineSurface(_)`, `Surface::NurbsSurface(_)` → `Unrecognized`
  (Phase-0 scope as above).
- `Surface::RevolutedCurve(_)` → `Unrecognized` (revolve recognition lands
  with S2).
- `Surface::Processor(pr)` → look at `pr.entity()`:
  - if the inner is one of the five analytic carriers → `Derived {
    carrier: Surface(CanonicalSurface::Placed(Processor::with_transform(
        Box::new(canonical of inner), *pr.transform()))),
    provenance: ConstructionWitness::Placed,
    map: Surface { u: IDENTITY, v: IDENTITY } }`.
    (`Processor` composes the affine map on output without reparameterizing,
    so φ = IDENTITY; the placement rides in the carrier.)
  - otherwise → `Unrecognized`.
- `Surface::ExtrudedCurve(ec)` → match `ec.entity_curve()`:
  - `Curve::Line(Line(a, b))` with `v = ec.extruding_vector()`:
    - if `(b - a).cross(v).magnitude() == 0.0` (the extrusion is parallel to
      the profile — a degenerate "surface" that is really a line) →
      `Unrecognized`.
    - else → `Derived { carrier: Surface(Plane(Plane::new(a, b, a + v))),
      provenance: Extruded,
      map: Surface { u: IDENTITY, v: IDENTITY } }`.
      Why IDENTITY: `Line::subs(t) = a + t(b−a)` over `t ∈ [0,1]` and
      `Plane::new(a, b, a+v)` is `a + u(b−a) + w·v` over `u,w ∈ (0,1)` (plane
      `parameter_range` is `(0,1)²`); `ExtrudedCurve::subs(u,w) =
      line.subs(u) + v·w` with `w ∈ [0,1]` (extruded `parameter_range` second
      axis is `[0,1]`). φ(u,w) = (u,w).
  - `Curve::Circle(c)` with `v = ec.extruding_vector()`:
    - Apply the exact cylinder test copied from
      `truck-geometry/src/canonical.rs` `to_same_geometry` (lines ~997-1021):
      decompose `c.transform()` as `Matrix4 { x: m1, y: m2, z: m3, w: tw }`;
      the circle is an exact z-preserving placement iff `m1.z == 0.0 &&
      m2.z == 0.0 && m3.x == 0.0 && m3.y == 0.0 &&
      m1.magnitude() == m2.magnitude() && m1.dot(m2) == 0.0 &&
      m1.magnitude() > 0.0`, and the extrusion is along the cylinder axis iff
      `v.x == 0.0 && v.y == 0.0`. If all hold, `radius = m1.magnitude()`,
      `center = tw.to_point()` (each coordinate finite), and:
      - `Cylinder::new(center, radius)` (returns `Outcome<Cylinder>`; a
        refusal here → `Unrecognized`) →
        `Derived { carrier: Surface(Cylinder(c)), provenance: Extruded,
        map: Surface { u: IDENTITY,
                        v: ParamMap::from_ranges(0.0, 1.0, 0.0, v.magnitude())
                           .expect("nonzero extrusion length") } }`.
      Why: `UnitCircle<Point3>::subs(t) = (cos t, sin t, 0)` over `t ∈
      [0, TAU)`, the placed circle is the affine image (so its parameter IS
      the angle), and `ExtrudedCurve::subs(u,w) = circle.subs(u) + v·w`; the
      canonical `Cylinder` (`center + r(cos θ, sin θ, 0) + (0,0,z)`) matches
      with `θ = u` and `z = |v|·w`. (The `.expect` is inside the module's
      test-free code path: `v.magnitude()` is strictly positive here because
      `v.x == 0 && v.y == 0` and a zero extrusion vector is already refused by
      the `Cylinder::new` refusal, so `from_ranges` cannot return `None` — but
      the module denies `expect_used`, so handle it with an explicit match that
      maps `None` → `Unrecognized` rather than unwrapping.)
    - otherwise → `Unrecognized`.
  - `Curve::BSplineCurve(_) | Curve::NurbsCurve(_) |
    Curve::IntersectionCurve(_)` → `Unrecognized`.
  - `Surface::ExtrudedCurve(_)` whose entity is another `ExtrudedCurve` (etc.)
    → `Unrecognized` (only `Line`/`Circle` profiles are canonical in Phase 0).

### 4. The certified-relationship contract

For every `Derived` witness the packet's tests must verify the relationship
`S_stored = S_canonical ∘ φ` by sampling: build the expected canonical carrier
from the input's parameters, then assert that sampled stored points equal the
canonical surface evaluated at φ(stored param), within the representation
tolerance. This is a regression witness for the map; the map is certified by
its construction (affine correspondence derived from the carriers' exact
parameter ranges), never by the sampling alone.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N`-shaped literal unless that line ends with an `// H-3` comment. The
map-verification tests compare sampled points with
`diff <= 64.0 * TOLERANCE` where `TOLERANCE` is `truck_base::tolerance::`
`TOLERANCE` (a name — H-3-compliant, a length comparison through the named
representation tolerance). `TAU` is `core::f64::consts::TAU`. If any other
small literal is unavoidable, use the same-line form:

```rust
const SLACK: f64 = 1.0e-9; // H-3: <why this slack, dimensionally>
```

Run `bash scripts/kernel-gates.sh <your base commit>` yourself before writing
`RESULT.json`.

## GATE-4 / `unscaled_legacy` (the ratchet)

This packet adds NO `unscaled_legacy()` calls. Do not touch
`scripts/unscaled_legacy_ceiling.txt` — the orchestrator owns the ratchet.

## Regression tests (exact names)

Put the tests in a `#[cfg(test)] mod tests` inside `recognize.rs` with
`#[allow(clippy::unwrap_used, clippy::expect_used)]`. Helpers you will need:
a placed-circle constructor

```rust
fn placed_circle(center: Point3, radius: f64) -> Processor<TrimmedCurve<UnitCircle<Point3>>, Matrix4> {
    let m = Matrix4 {
        x: Vector4::new(radius, 0.0, 0.0, 0.0),
        y: Vector4::new(0.0, radius, 0.0, 0.0),
        z: Vector4::new(0.0, 0.0, 1.0, 0.0),
        w: Vector4::new(center.x, center.y, center.z, 1.0),
    };
    Processor::with_transform(TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, core::f64::consts::TAU)), m)
}
```

and a map-verification helper that samples `stored.subs(u, w)` against
`canonical.subs(map.u.apply_f64(u), map.v.apply_f64(w))` (via the
`CanonicalCarrier`'s extracted carrier) on a grid and asserts
`diff <= 64.0 * TOLERANCE`.

1. `recognize_line_and_plane_are_exact_canonical` —
   `recognize_curve(Curve::Line(Line((0,0,0), (1,0,0))))` →
   `ExactCanonical { carrier: Curve(Line), map: Curve(IDENTITY) }`; and
   `recognize_surface(Surface::Plane(Plane::xy()))` →
   `ExactCanonical { carrier: Surface(Plane), map: Surface{IDENTITY, IDENTITY} }`.
   Assert by pattern; also assert the carrier matches the input by value.
2. `recognize_extruded_line_is_plane` —
   `ExtrudedCurve::by_extrusion(Curve::Line(Line((0,0,0), (2,0,0))), Vector3::unit_z())`:
   witness is `Derived { provenance: Extruded }`; extract the `Plane` carrier
   and run the sampling map-check (grid over `u ∈ [0,1]`, `w ∈ [0,1]`).
3. `recognize_extruded_circle_is_cylinder` —
   `placed_circle((1,2,0), 3.0)` extruded by `(0,0,5)`: witness is
   `Derived { provenance: Extruded }`; extract the `Cylinder` carrier and
   assert `center() == (1,2,0)` and `radius() == 3.0`; run the sampling
   map-check (grid over `u ∈ [0, 2π)`, `w ∈ [0,1]`).
4. `recognize_skew_or_degenerate_extrude_is_unrecognized` — (a) the circle of
   test 3 extruded by `(1,0,0)` (not along the axis) → `Unrecognized`;
   (b) `ExtrudedCurve::by_extrusion(Curve::Line(Line((0,0,0), (1,0,0))),
   Vector3::new(2.0, 0.0, 0.0))` (extrusion parallel to the profile) →
   `Unrecognized`.
5. `recognize_processor_places_the_inner_carrier` —
   `Surface::Processor(Processor::with_transform(Box::new(Surface::Plane(Plane::xy())),
   Matrix4::from_translation(Vector3::new(1.0, 2.0, 3.0))))` → witness is
   `Derived { provenance: Placed }`; the carrier is `Placed` with an inner
   `Plane`; sample `stored.subs(u,v)` against the inner plane's
   `subs(u,v) + (1,2,3)` on a grid (equivalently, the placed surface's own
   `subs`).

Every other existing truck-geometry test must stay green — in particular the
`canonical.rs` suite (the cylinder conditions you copy live there) and the
`decorators` tests.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-geometry
cargo clippy -p truck-geometry --all-targets --no-deps
cargo test -p truck-geometry --lib --tests --no-fail-fast
cargo check --locked -p truck-geometry --all-targets
bash scripts/kernel-gates.sh <your base commit>
```

Never run a bare `cargo test`.

## Forbidden

Editing any file outside `write_allow`. Returning a `Derived` witness whose
`S_stored = S_canonical ∘ φ` relationship is not exact by construction —
sampling is a regression witness, never the certification. Degrading a
recognized analytic carrier to a spline in the witness (the carrier keeps the
analytic arm). Spline→analytic detection in this packet (documented later).
Adding `#[ignore]`. Changing the GATE-4 ceiling. Running cargo check --workspace / cargo build --workspace / a bare cargo check (the crate-scoped -p <crate> checks in Done-when are the contract; a workspace-wide build on a shared machine with concurrent workers exhausts disk).

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- a pre-existing test you did not expect to touch is broken → do NOT weaken the
  gate; report it in `disagreements` with the failing test name and the exact
  reason
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it. In `notes`, record the
deviation you made explicit (the two-armed `CanonicalParamMap`) and the
measured max sampling deviation you observed on the map checks (should be
ulp-class, well under `64 * TOLERANCE`).

Commit on the current branch with subject
`feat(geometry): structural recognizer producing CanonicalCarrierWitness (BG-SOL-P0-REC)`.
