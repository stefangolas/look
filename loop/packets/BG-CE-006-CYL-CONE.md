# WORK PACKET BG-CE-006-CYL-CONE — Cylinder and Cone as first-class carriers

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```yaml
id:          BG-CE-006-CYL-CONE
contract:    [BG-CE-006]
covers:      [BG-CE-006-CYLINDER, BG-CE-006-CONE]
class:       mechanical
crates:      [truck-geometry]
depends_on:  [BG-EVD-r3, BG-TOL-001-TYPE]
write_allow:
  - vendor/truck/truck-geometry/src/specifieds/cylinder.rs
  - vendor/truck/truck-geometry/src/specifieds/cone.rs
  - vendor/truck/truck-geometry/src/specifieds/mod.rs
  - vendor/truck/truck-geometry/tests/analytic_carriers.rs
read_allow:
  - vendor/truck/truck-geometry/src/specifieds/sphere.rs
  - vendor/truck/truck-geometry/src/specifieds/torus.rs
  - vendor/truck/truck-base/src/tolerance.rs
  - vendor/truck/truck-base/src/evidence.rs
tests_required:
  - cylinder_point_normal_relation
  - cylinder_round_trips_through_search_parameter
  - cone_apex_is_a_first_class_point
  - cone_round_trips_through_search_parameter
  - degenerate_radius_refuses
budget:      {turns: 80, ctx_tokens: 180000}
# Read by run_packet.py's dispatch preflight: GATE-4's count plus this budget
# must fit under the ceiling committed on the slot's branch. Four contexts:
# one per search_parameter, one per include. See "The ratchet".
unscaled_legacy_budget: 4
```

## Problem — why this is reachable from untrusted geometry

`specifieds/` has Line, UnitCircle, UnitHyperbola, UnitParabola, Plane, Sphere
and Torus — **no Cylinder and no Cone**, which are the two commonest mechanical
surfaces after the plane. Every cylindrical hole and every countersink in every
STEP file the kernel imports is currently carried as something else: a
`RevolutedCurve` decorator, or worse, degraded to a NURBS by
`BSplineSurface::homotopy`. Once code depends on a NURBS cylinder, the closed
forms that later stages need — cylinder→cylinder offset is `r ± d`, cone→cone is
a shifted apex — have no type to attach to, and every call site has to be found
and changed later.

This packet adds the two carriers. It does **not** wire them into any enum,
conversion, or import path: that is a separate packet and touching it here is a
V1 rejection.

## Anchors — verified 2026-08-18, counts are exact

Locate by running the pattern. **Never locate by line number.** `rg` is not
installed on this machine; any case-sensitive literal search is equivalent.
**If a count differs, STOP** and report `ANCHOR_MISMATCH`.

| # | command | expect |
|---|---|---|
| A1 | `grep -c '^mod ' vendor/truck/truck-geometry/src/specifieds/mod.rs` | **7** |
| A2 | `grep -c 'pub struct Sphere' vendor/truck/truck-geometry/src/specifieds/mod.rs` | **1** |
| A3 | `grep -rc 'Cylinder' vendor/truck/truck-geometry/src/` — files with a nonzero count | **0** |
| A4 | `grep -rc '\bCone\b' vendor/truck/truck-geometry/src/` — files with a nonzero count | **0** |
| A5 | `grep -c '^impl' vendor/truck/truck-geometry/src/specifieds/sphere.rs` | **9** |
| A6 | `grep -c autotests vendor/truck/truck-geometry/Cargo.toml` | **0** |

A3 and A4 are the ones that matter: if either is nonzero, someone has already
added one of these types and this packet is stale. Stop.

A6 confirms `truck-geometry` does **not** set `autotests = false`, so a new file
in `tests/` is picked up automatically with no `Cargo.toml` edit. `Cargo.toml`
is not on your allowlist; if you find you need it, that is a `SPEC_GAP`.

## Decisions already made for you — do not relitigate these

**1. Canonical placement, axis `+z`.** This is the house convention and `Torus`
is the proof: it stores `center` plus two radii and no axis, and its `subs` puts
the tube around the `z` axis through `center`. Arbitrary placement in the model
is carried by a `Processor` transform wrapped around the canonical surface, not
by fields on the surface. Follow it exactly. **Do not add an axis field.**

**2. The two structs, defined in `mod.rs` beside `Sphere` and `Torus`** (that is
where every specified struct is declared; the per-type file holds only `impl`
blocks):

```rust
/// cylinder of the given radius, around the `z` axis through `center`
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, SelfSameGeometry)]
pub struct Cylinder {
    center: Point3,
    radius: f64,
}

/// cone with its apex at `apex`, opening along `+z` with the given half angle
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, SelfSameGeometry)]
pub struct Cone {
    apex: Point3,
    half_angle: f64,
}
```

Fields stay private with `const fn` getters, exactly as `Torus` does.

**3. The parameterizations.** `u` is the angle around the axis, `v` is the
height along it. These are not negotiable — the tests below assert them.

```
Cylinder::subs(u, v) = center + (r cos u,  r sin u,  v)
Cone::subs(u, v)     = apex   + (v tan a cos u,  v tan a sin u,  v)      a = half_angle
```

Derivatives follow by differentiating those two lines. `der_mn` must agree with
`subs`/`uder`/`vder`/`uuder`/`uvder`/`vvder` — write `der_mn` in the cyclic
`match m % 4` style `Torus::der_mn` uses, then make the named methods delegate
to it or be written consistently with it. A mismatch between `der_mn` and the
named derivatives is the single most likely defect in this packet; the property
test below exists to catch it and you should run it early.

**4. Constructors refuse, they do not panic (H-1).** `Torus::new` panics on a
non-positive radius. **Do not copy that.** New code returns `Outcome<Self>`:

```rust
pub fn new(center: Point3, radius: f64) -> Outcome<Self>
```

Refuse with `Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ChartDegenerate))`
when `radius` is not finite or `radius <= 0.0`; for `Cone`, when `half_angle` is
not finite or is outside the open interval `(0, PI/2)`. That is the same refusal
`ToleranceCtx::new` already raises for a non-positive scale — read it in
`truck-base/src/tolerance.rs` and match its shape. On success return
`Ok(Certified::new(value, Certificate { .. }))` with `method: Method::Float` and
the same `Certificate` shape the `IncludeCurve` impls in `sphere.rs` use.

**5. `parameter_range` and periodicity.**

| | u | v |
|---|---|---|
| `Cylinder` | `(Included(0.0), Excluded(2π))`, `u_period = Some(2π)` | `(Unbounded, Unbounded)`, no period |
| `Cone` | `(Included(0.0), Excluded(2π))`, `u_period = Some(2π)` | `(Unbounded, Unbounded)`, no period |

**6. Do NOT implement `BoundedSurface` for either type.** Both are unbounded in
`v`, and `BoundedSurface::range_tuple` calls `.expect(UNBOUNDED_ERROR)` on the
range — implementing it would install a panic on a path reachable from imported
geometry, which is an H-1 violation. ~~`Plane` does implement it despite being
unbounded; that is a pre-existing defect in this tree.~~ **Do not copy it and do
not fix it here** — report it in your notes.

> **Correction, session 8 — the struck sentence was wrong and the worker said
> so.** This packet's worker reported that `Plane`'s `BoundedSurface` impl is
> not a defect, and it was checked and is right.
> `Plane::parameter_range` returns `(Bound::Included(0.0),
> Bound::Included(1.0))` on **both** axes, so `try_range_tuple` yields
> `(Some(..), Some(..))` and `range_tuple`'s `.expect(UNBOUNDED_ERROR)` cannot
> fire. `impl BoundedSurface for Plane {}` is sound.
>
> The rest of decision 6 stands, and the check strengthens it: `Cylinder` and
> `Cone` both return `(Bound::Unbounded, Bound::Unbounded)` for `v`, so
> `try_range_tuple` yields `None` on that half and `range_tuple` **would**
> panic. Giving either type a `BoundedSurface` impl installs exactly the defect
> described above. So the mechanism was right and only the example was wrong —
> and the answer to "should Cylinder and Cone implement it after all" is a
> firmer **no** than when this packet was written.

**7. The apex is a first-class point.** `Cone::subs(u, 0.0)` is the apex for
every `u`, `uder` vanishes there, and `normal` is undefined there. Do **not**
paper over it by nudging `v` or returning an arbitrary normal. `normal(u, v)`
for `v == 0.0` returns `Vector3::zero()`, and `cone_apex_is_a_first_class_point`
asserts exactly that. A later packet makes apex-vanishing a topology event; this
packet only has to not lie about it.

**8. `ParametricSurface3D::normal` is a unit vector** for both types away from
the apex. For the cone that means dividing by `sqrt(1 + tan^2 a)` — do not skip
the normalization because the cylinder's happens to come out unit for free.

## The recipes for predicates

Every comparison you write against a length goes through a `ToleranceCtx`
obtained **once at the top of the function**, as
`let ctx = ToleranceCtx::unscaled_legacy();` — the same Stage-A scaffold the
rest of the crate now uses. Four functions need one and no others do:
`Cylinder::search_parameter`, `Cone::search_parameter`, and the two
`IncludeCurve<BSplineCurve<Point3>>` impls.

| what you are comparing | use |
|---|---|
| two `Point3` in model space | `ctx.near_pt(a, b)` |
| a length, against zero | `ctx.is_small_len(l)` |
| a dimensionless ratio, a sine, a normalized parameter | `ctx.is_small_ratio(x)` |

`search_parameter` returns `Option<(f64, f64)>` (matching `Sphere`, whose
signature you must not change): `Some` when the point really is on the surface
within `ctx`, `None` otherwise. For the cylinder, the radial distance
`((p - center) projected onto the xy plane).magnitude()` compared against
`radius` is a **length** — `is_small_len` of the difference. For the cone the
same comparison against `v * tan a` is also a length. Neither is a ratio.

`search_nearest_parameter` always answers — it is the nearest point, not a
membership test — and needs no context.

## Template — copy this shape

`sphere.rs` is the file to copy. It has the 9 `impl` blocks you need modulo
`BoundedSurface` (see decision 6): the inherent `impl` with the constructor and
getters, `ParametricSurface`, `ParametricSurface3D`, two `IncludeCurve`s,
`ParameterDivision2D`, `SearchParameter<D2>`, `SearchNearestParameter<D2>`.
`torus.rs` is the better model for `der_mn` and for `parameter_range` with a
period.

For `ParameterDivision2D`, copy `Sphere`'s **including its comment about a
tolerance coarser than the surface** — the same clamp is needed here and for the
same reason. The `u` division comes from the circular cross-section, so
`ratio = min(tol / radius, 1.0)` for the cylinder; for the cone the radius
varies with `v`, so use the **larger** of the two ends of `vrange`, i.e. the
widest cross-section in the requested range, which is the conservative choice.
The `v` division is a straight line for both: two points, the ends.

## Tests required

New file `vendor/truck/truck-geometry/tests/analytic_carriers.rs`. Each must be
a named `#[test]` fn. Follow the three-layer house rule: these five are the unit
and property layer.

1. `cylinder_point_normal_relation` — over a grid of `(u, v)`, assert
   `subs(u, v) - (center + (0, 0, v)) == normal(u, v) * radius` to `TOLERANCE`,
   the same shape as the `Sphere` doc example. This catches a swapped `u`/`v`
   and a wrong normal sign in one assertion.
2. `cylinder_round_trips_through_search_parameter` — for a grid of parameters,
   `search_parameter(subs(u, v))` returns `Some` and recovers `(u, v)`; and a
   point displaced off the surface by ten times the tolerance returns `None`.
3. `cone_apex_is_a_first_class_point` — `subs(u, 0.0)` is the apex for several
   `u`; `uder(u, 0.0)` is zero; `normal(u, 0.0)` is zero.
4. `cone_round_trips_through_search_parameter` — as 2, for `v > 0` only.
5. `degenerate_radius_refuses` — `Cylinder::new` with radius `0.0`, `-1.0`,
   `f64::NAN` and `f64::INFINITY` each return
   `Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ChartDegenerate))`; same for
   `Cone::new` with `half_angle` of `0.0`, `PI/2`, `-0.1` and `f64::NAN`.

Also add a `proptest` property test for the derivative consistency named
`ders_agree_with_der_mn`, comparing `uder`/`vder`/`uuder`/`uvder`/`vvder`
against `der_mn` at random parameters for both types. `truck-geometry` already
depends on `proptest`; follow the style in
`truck-geometry/src/decorators/af_surface.rs`.

**H-3 escape hatch, you will need it.** `scripts/kernel-gates.sh` rejects bare
absolute float literals in predicates, and a test comparing floats trips it. The
opt-out is a `// H-3` comment **on the same line as the literal** — not the line
above, and rustfmt will move a trailing comment off a brace-opener line, so put
the literal on its own statement line first.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-geometry
cargo clippy -p truck-geometry --all-targets --no-deps -- -D warnings
cargo test -p truck-geometry --lib --test analytic_carriers --test sphere --test torus --no-fail-fast
cargo check --workspace --all-targets
```

`--no-deps` matters: without it clippy lints the vendored path dependencies too
and aborts before reaching your crate. Never run a bare `cargo test` — it builds
56 examples. Send cargo output to a file and read the tail.

**Confirm the baseline before you edit anything.** Run that test command at the
base commit first and record which tests already fail; this tree has
pre-existing failures that are not yours. Report them and do not fix them.

## The ratchet

`scripts/kernel-gates.sh` counts `unscaled_legacy(` call sites tree-wide and
fails when the total exceeds the ceiling in
`scripts/unscaled_legacy_ceiling.txt`. Your budget is **4** — one context in
each of the four functions named under "The recipes". That file is **not** on
your allowlist and you must not edit it. If you find yourself wanting a fifth,
you have constructed a context per call site instead of per function.

## Forbidden

Editing any file outside `write_allow` — in particular
`truck-geometry/src/lib.rs`, `truck-modeling/**` anything,
`scripts/unscaled_legacy_ceiling.txt`, `Cargo.toml`, and **`loop/` anything:
your result file goes in the root of your worktree and nowhere else.** Wiring
the new types into any enum, conversion, `From` impl or import path — that is
BG-CE-006-ENUM and not yours. Implementing `BoundedSurface`. Changing any
existing signature. Adding an axis field. Panicking, `unwrap`, `expect`, or
indexing that can go out of range on any path. Widening a tolerance or adding
`#[ignore]` to make a test pass. Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the command and what you
  saw. A3 or A4 nonzero means this packet is stale; stop immediately.
- a trait you must implement needs a method this packet does not specify →
  `SPEC_GAP`, naming the trait, the method and its signature. **Do not invent a
  behaviour to make it compile** — a missing method is the packet's defect.
- an existing test changes its result → **report it, do not fix it.** This
  packet adds new types and touches no existing one, so a moved test means
  something in `mod.rs` broke; say which test.
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

```json
{"id":"BG-CE-006-CYL-CONE","status":"DONE","contracts":["BG-CE-006"],
 "covers":["BG-CE-006-CYLINDER","BG-CE-006-CONE"],
 "tests_added":6,"unscaled_legacy_calls":0,
 "anchors_verified":{"A1":7,"A2":1,"A3":0,"A4":0,"A5":9,"A6":0},
 "notes":"set unscaled_legacy_calls to the number you actually introduced. Report the baseline test failures you confirmed, whether Plane's BoundedSurface impl looked like a defect to you on reading it, and any place where the canonical-placement convention forced something awkward -- that judgement is worth more than silent compliance."}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(geometry): Cylinder and Cone as first-class canonical carriers (BG-CE-006)`.
