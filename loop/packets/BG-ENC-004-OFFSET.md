# WORK PACKET BG-ENC-004-OFFSET — `EnclosureSurface` for the `Offset` decorator (composition over new vector/scalar field traits)

You are implementing the last BG-ENC-004 carrier: a certified enclosure for
`Offset<S, N>`, the pointwise sum `S(u,v) + N(u,v)`. Everything you need is in
this document. **Do not read any other spec file** — this packet is
self-contained, but it implements a decided owner amendment (2026-08-24) whose
design was validated in a scratch crate against the real carriers before
dispatch (`scratch/offsetscratch/`). The three flagship witnesses there are the
packet's regression tests.

```json
{"id":"BG-ENC-004-OFFSET","status":"DONE","contracts":["BG-ENC-004-OFFSET"],
 "tests_added":3,"deviations":[],"disagreements":[],
 "baseline_failures":[],"notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. **If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-ENC-004-OFFSET
class:       design
crates:      [truck-evidence, truck-geometry]
write_allow:
  - vendor/truck/truck-evidence/src/enclosure.rs
  - vendor/truck/truck-evidence/src/decorators/offset.rs
  - vendor/truck/truck-geometry/src/decorators/offset/mod.rs
read_allow:
  - vendor/truck/truck-geometry/src/decorators/offset/surface.rs
  - vendor/truck/truck-evidence/src/decorators/revolved.rs
  - vendor/truck/truck-geometry/src/decorators/scalar_function.rs
tests_required:
  - offset_sphere_constant_distance_encloses
  - offset_plane_constant_distance_is_exact
  - offset_pole_box_degrades_honestly
budget:      {turns: 55, ctx_tokens: 130000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub struct Offset' vendor/truck/truck-geometry/src/decorators/mod.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub struct NormalField' vendor/truck/truck-geometry/src/decorators/mod.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub const fn entity' vendor/truck/truck-geometry/src/decorators/offset/mod.rs"}
  - {id: A4, expect: 2, cmd: "grep -c 'BG-ENC-004-OFFSET' vendor/truck/truck-evidence/src/decorators/offset.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'pub trait EnclosureSurface' vendor/truck/truck-evidence/src/enclosure.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub trait EnclosureCurve' vendor/truck/truck-evidence/src/enclosure.rs"}
```

## Problem

`Offset<T, N>` is the pointwise sum `S(u,v) + N(u,v)` of two parametric
surfaces, and truck-geometry's only `ParametricSurface` impl for it requires
`N: ParametricSurface<Point = C::Vector>` — the offset field is **vector**-
valued. `EnclosureSurface` is bounded `ParametricSurface<Point = Point3>`, so
for any `S` that is an `EnclosureSurface`, `N` has `Point = Vector3` and can
never be one: the naive `impl EnclosureSurface for Offset<S, N>` does not
typecheck for any choice of the two. This is a **type error, not a curvature
bound**, and it is why the scaffold says OFFSET is a design item.

The owner's decided resolution (2026-08-24): **composition over new interface
surface**, never `N: EnclosureSurface`. Two new traits go alongside the family;
`NormalField<S, F>` implements the vector trait; `Offset` composes.

## The design, validated in the scratch before dispatch

The scratch (`scratch/offsetscratch/`, dev profile) implemented every formula
below against the real `Sphere`/`Plane` carriers and ran the three flagship
witnesses. The measured numbers in this packet are from that run; do not
"improve" them without re-measuring.

### 1. Two new traits in `enclosure.rs`

Exact signatures from the owner amendment:

```rust
pub trait EnclosureVectorField: ParametricSurface<Point = Vector3, Vector = Vector3> {
    /// MUST contain { self.subs(u, v) : (u,v) ∈ uu×vv } (BG-ENC-001).
    fn enclose(&self, uu: Interval, vv: Interval) -> Box3;
    /// MUST contain { self.der_mn(m, n, u, v) : (u,v) ∈ uu×vv }.
    fn enclose_der(&self, m: usize, n: usize, uu: Interval, vv: Interval) -> Box3;
}

pub trait EnclosureScalarField2 {
    /// MUST contain { self.subs(u, v) : (u,v) ∈ uu×vv }.
    fn enclose(&self, uu: Interval, vv: Interval) -> Interval;
    /// MUST contain { self.der_mn(m, n, u, v) : (u,v) ∈ uu×vv }.
    fn enclose_der(&self, m: usize, n: usize, uu: Interval, vv: Interval) -> Interval;
}
```

The vector trait is `EnclosureSurface` minus the `Point3` bound — that bound is
precisely what `N` can never satisfy. No `direction_cone` method: a cone is
derivable from the field's own enclosure box via the existing
`midpoint_ball_cone` helper whenever a tight path wants one; the composition
needs only the two methods. `EnclosureScalarField2` has no supertrait — the
constant-distance case (`f64`) is its only v1 impl, and a variable-distance
scalar field gets an impl only when a carrier needs one.

### 2. `impl EnclosureScalarField2 for f64` (in `decorators/offset.rs`)

`f64` is a constant: `enclose` returns the degenerate interval `[x,x]`,
`enclose_der(m,n)` returns `[x,x]` for `(0,0)` and `[0,0]` otherwise. `f64`
already implements `ScalarFunctionD2` in truck-geometry (the geometry's
`subs = normal · scalar.subs` requires it), so the f64 impl is the only scalar
impl v1 ships.

### 3. `impl EnclosureVectorField for NormalField<S, F>` (in `decorators/offset.rs`)

`NormalField::subs = S.normal(u,v) · F.subs(u,v)` where `normal = (S_u × S_v)
/ ‖S_u × S_v‖`. The impl composes over the base's `EnclosureSurface` and the
scalar's `EnclosureScalarField2`:

- **Position** (`enclose`): the unit normal's box is the cross product box of
  the base's two first-partial enclosures, scaled by `[0, 1/L]` where `L` is
  the base's **own certified immersion margin**
  (`S.immersion_lower_bound(uu, vv)`), because `1/‖S_u×S_v‖ ∈ (0, 1/L]`. The
  field's box is that times the scalar field's interval:
  `n_box * F.enclose(uu, vv)` componentwise. **When `L = 0`** (singular locus
  somewhere in the box) the unit normal's POSITION enclosure still exists — a
  unit vector never leaves the unit ball, so the `[-1,1]³` fallback is always
  sound (spec decision 4). Do NOT sample to manufacture a finite answer.

  **The impl's bounds are `S: ParametricSurface3D + EnclosureSurface,
  F: ScalarFunctionD2 + EnclosureScalarField2`.** `NormalField<S,F>:
  ParametricSurface<Point = Vector3>` (the `EnclosureVectorField` supertrait)
  needs `S: ParametricSurface3D` and `F: ScalarFunctionD2` (surface.rs:69-72);
  the enclosure methods need `S: EnclosureSurface` and `F:
  EnclosureScalarField2`. All four are required; `f64` satisfies `ScalarFunctionD2`
  already.

- **First partials** (`enclose_der(1,0)` / `(0,1)`): the projection form of the
  quotient rule, which the scratch proved is the only form tight enough to be
  usable:

  ```text
  n_u = (I − nnᵀ)·(c_u/‖c‖),   c = S_u × S_v,   c_u = S_uu × S_v + S_u × S_uv
  N_u = n_u·f + n·f_u           (f_u = F.enclose_der(1,0); 0 for f64)
  ```

  Enclose `c` via `cross_box(S.enclose_der(1,0), S.enclose_der(0,1))`, `c_u`
  via `cross_box(S.enclose_der(2,0), S.enclose_der(0,1)) ⊕ cross_box(S.enclose_der(1,0), S.enclose_der(1,1))`
  (and symmetrically `c_v` from `S_uv, S_v, S_u, S_vv`), and the denominator
  by the base's certified immersion margin `L`. Intersect the normal's position
  box with the unit ball before applying the projector (a unit vector has
  coordinates in `[-1,1]`, always sound, keeps the projector bounded).
  **When `L = 0`** return `Interval::ENTIRE` per axis: curvature is genuinely
  unbounded at a singular locus, and the honest answer is the unbounded box
  (spec decision 4; the ISC/PCURVE precedent).

- **Higher partials** (`enclose_der(m,n)` for `m+n ≥ 2`): the unbounded box
  `ENTIRE` per axis. Deriving them needs the base's THIRD partials and the full
  shape-operator chain; the honest answer is the unbounded box (the ISC/PCURVE
  fourth-order precedent). The offset's own `enclose_der` at these orders is
  then `S.enclose_der ⊕ ENTIRE = ENTIRE`, sound.

This is where curvature (the shape operator) genuinely enters; it still does
not make `N` an `EnclosureSurface`.

### 4. `impl EnclosureSurface for Offset<S, N>` (in `decorators/offset.rs`)

The composition is the geometry's own arithmetic, method for method:

```text
enclose(Offset, U)           = enclose(S, U)           ⊕ enclose_vec(N, U)
enclose_der(Offset, m, n, U) = enclose_der(S, m, n, U) ⊕ enclose_der_vec(N, m, n, U)
```

where `⊕` is componentwise outward-rounded interval addition
(`Box3`'s per-axis `+`). The bounds: `S: ParametricSurface3D + EnclosureSurface,
N: EnclosureVectorField` — this typechecks because `EnclosureVectorField`
requires only `Point = Vector3`, exactly `Offset`'s `N::Point`. **Two
composition details, both found by the scratch:**

- **`enclose_der(0,0)` must return `self.enclose(uu, vv)`, NOT the
  composition of the carriers' `enclose_der(0,0)`.** `plane.rs` returns the
  ZERO box at `(0,0)` (an outlier; `line.rs`/`cone.rs`/`revolved.rs` return the
  point box). Composing `S.enclose_der(0,0) ⊕ N.enclose_der(0,0)` for a plane
  base under-estimates to zero, a BG-ENC-001 violation. The `(0,0)` partial is
  the position, and the position is `enclose`. (The same trap is documented in
  `revolved.rs`'s comment about `plane.rs`/`cylinder.rs` being the outliers —
  do not copy them on this point.)
- **`normal_cone` and `immersion_lower_bound` follow the family construction
  off the summed derivative boxes' cross product** (spec decision 2):
  `normal_cone = midpoint_ball_cone(cross_box(enclose_der(1,0), enclose_der(0,1)))`,
  `immersion_lower_bound = immersion_lower_bound_box(same cross)`. This
  certifies **per cell**: the scratch measured the constant-offset sphere's
  cone at cell half-width ≤ 0.05 and `None` (honest) on wider boxes — the same
  per-cell pattern as every other carrier. The offset normal EQUALS the base
  normal for the NormalField constant case (n·(S_u + d·n_u) = 0 since n⊥S_u and
  n·n_u = 0; scratch-verified dot = 1.0), which is why the family construction
  works at all; it is the generic construction for a GENERIC `N`, and the
  tightness fact is recorded in a comment, not assumed.

### 5. The accessor blocker (found by the scratch, must be fixed)

`NormalField<T, F>`'s fields `entity` and `scalar` are PRIVATE to
truck-geometry, and `NormalField` exposes NO accessors (only `Offset` has
`entity()`/`offset()`). `impl EnclosureVectorField for NormalField<S,F>` cannot
reach the base surface or the scalar field from truck-evidence without them.
Add to `impl<T, F> NormalField<T, F>` in `decorators/offset/mod.rs`:

```rust
/// Returns the base entity geometry.
pub const fn entity(&self) -> &T {
    &self.entity
}
/// Returns the scalar field.
pub const fn scalar(&self) -> &F {
    &self.scalar
}
```

(mirroring `Offset`'s own accessors in the same file). This is why
`vendor/truck/truck-geometry/src/decorators/offset/mod.rs` is in
`write_allow`. `Offset<T,N>` itself is unchanged.

## Regression tests (exact names)

The three flagship witnesses from the scratch, with its measured numbers.
Import the pieces you need from `crate::*`; the refusal/convention shapes are
specified inline.

1. `offset_sphere_constant_distance_encloses`

   ```rust
   let base = Sphere::new(Point3::new(0.0, 0.0, 0.0), 1.0);
   let d = 0.3_f64;
   let offset = Offset::new(base, NormalField::new(base, d));
   ```

   Assert, on a pole-free box `uu = [0.3, 0.9]`, `vv = [0.4, 1.3]`:
   - `offset.enclose(uu, vv)` contains every sampled `offset.subs(u, v)` on a
     25×25 grid (BG-ENC-001 soundness);
   - `offset.enclose_der(m, n, uu, vv)` contains every sampled
     `offset.der_mn(m, n, u, v)` for `(m,n)` in `(1,0)`, `(0,1)`, `(2,0)`,
     `(1,1)`, `(0,2)` on a 15×15 (9×9 for second order) grid;
   - `offset.normal_cone` is `Some` on a cell `uu = [1.15, 1.25]`,
     `vv = [0.7, 0.9]` and contains every sampled
     `offset.uder.cross(offset.vder).normalize()` by angle (scratch: half-angle
     ≈ 0.86 rad);
   - `offset.immersion_lower_bound` is strictly positive on that cell
     (scratch: ≈ 0.50);
   - the offset's unit normal equals the base's unit normal at a sample point
     (`(offset.uder × offset.vder).normalize().dot(base.normal(u,v)).abs() >
     1 − 1e-9`).

2. `offset_plane_constant_distance_is_exact`

   ```rust
   let base = Plane::new(Point3::new(0.0,0.0,0.0), Point3::new(1.0,0.0,0.0), Point3::new(0.0,1.0,0.0));
   let d = 0.5_f64;
   let offset = Offset::new(base, NormalField::new(base, d));
   ```

   On `uu = vv = [-0.5, 1.5]`: `offset.enclose` contains every sampled
   `offset.subs` (21×21 grid); `offset.normal_cone` is `Some` with half-angle
   < 1e-9 (the plane normal is constant); `offset.immersion_lower_bound` ≈ 1.0
   (the plane's constant cross norm). This is the affine-exact case — the box
   width should be the plane's own (scratch: 2.0), not inflated by the offset.

3. `offset_pole_box_degrades_honestly`

   A sphere box touching a pole (`uu = [0.0, 0.15]`, `vv = [0.0, 1.0]`):
   - the base `immersion_lower_bound` is 0;
   - `NormalField::enclose` still contains every sampled
     `base.normal(u,v) * d` (the unit-ball fallback is sound);
   - `NormalField::enclose_der(1,0)` is `Interval::ENTIRE` per axis;
   - `offset.enclose` still contains every sampled `offset.subs`;
   - `offset.normal_cone` is `None` — the honest singular-locus arm.

Every other existing evidence test must stay green — in particular the
`enclosure.rs` `Box3`/cone/cross-box unit tests and all of `revolved.rs`,
`processor.rs`, `extruded.rs`, `pcurve.rs`, `intersection_curve.rs`, and the
elementary carrier impls.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N`-shaped literal unless that line ends with an `// H-3` comment. This
packet's tests compare floats (dot products, cone cosines, half-angle slack,
immersion slack). The house form is a named `const` for each slack
(`const DOT_SLACK: f64 = 1.0e-9; // H-3: ...` on the same line), or a `// H-3`
comment on the literal line. Copy the pattern from `revolved.rs`'s tests.
Run `bash scripts/kernel-gates.sh <your base commit>` yourself before writing
`RESULT.json`.

## GATE-4 / `unscaled_legacy` (the ratchet)

This packet adds NO `unscaled_legacy()` calls. Do not touch
`scripts/unscaled_legacy_ceiling.txt` — the orchestrator owns the ratchet and
will reset it to the measured post-merge count.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-evidence -p truck-geometry
cargo clippy -p truck-evidence -p truck-geometry --all-targets --no-deps
cargo test -p truck-evidence --lib --tests --no-fail-fast
cargo test -p truck-geometry --lib --tests --no-fail-fast
cargo test -p truck-evidence --doc
cargo check --workspace --all-targets
bash scripts/kernel-gates.sh <your base commit>
```

Never run a bare `cargo test`.

## Forbidden

Editing any file outside `write_allow`. Making `N` an `EnclosureSurface` in any
form (the type error is the whole reason this is a design item). Returning a
finite box for a NormalField derivative at a singular locus — the honest answer
there is `ENTIRE`, never a sample. Sampling to manufacture a finite cone or
immersion bound. Making `Offset::subs`/`der_mn` (the geometry type) do anything
other than add the two fields. Adding `#[ignore]`. Changing the GATE-4 ceiling.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- a pre-existing test you did not expect to touch is broken by the new traits
  or impls → do NOT weaken the gate; report it in `disagreements` with the
  failing test name and the exact reason
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it. In `notes`, record the
three measured numbers (sphere cell cone half-angle and immersion bound, plane
box width) as you observed them.

Commit on the current branch with subject
`feat(evidence): certified enclosure for Offset via vector-field composition (BG-ENC-004-OFFSET)`.
