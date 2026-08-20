# WORK PACKET BG-ENC-004-PCURVE — `EnclosureCurve for PCurve<BSplineCurve<Point2>, S>`

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-ENC-004-PCURVE","status":"DONE","contracts":["BG-ENC-001","BG-ENC-002"],
 "tests_added":8,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: if a decision below is wrong,
say so rather than working around it.

```yaml
id:          BG-ENC-004-PCURVE
contract:    [BG-ENC-001, BG-ENC-002]
class:       mechanical
crates:      [truck-evidence]
depends_on:  [BG-ENC-003-BSPLINE]
write_allow:
  - vendor/truck/truck-evidence/src/decorators/pcurve.rs
read_allow:
  - vendor/truck/truck-evidence/src/lib.rs
  - vendor/truck/truck-evidence/src/enclosure.rs
  - vendor/truck/truck-evidence/src/harness.rs
  - vendor/truck/truck-evidence/src/bspline.rs
  - vendor/truck/truck-evidence/src/plane.rs
  - vendor/truck/truck-evidence/src/sphere.rs
  - vendor/truck/truck-evidence/src/decorators/mod.rs
  - vendor/truck/truck-evidence/src/decorators/extruded.rs
  - vendor/truck/truck-geometry/src/decorators/mod.rs
  - vendor/truck/truck-geometry/src/decorators/pcurve.rs
  - vendor/truck/truck-geometry/src/nurbs/bspcurve.rs
  - vendor/truck/truck-geometry/src/nurbs/knot_vec.rs
tests_required:
  - pcurve_encloses_sampled_points
  - pcurve_out_of_range_box_is_unbounded
  - pcurve_der_enclosures_match_partials
  - pcurve_tangent_cone_contains_sampled_tangents
  - pcurve_tangent_cone_refuses_when_the_derivative_hull_contains_zero
  - pcurve_subbox_enclosure_is_tighter_than_full_range
  - pcurve_enclosure_converges_under_bisection
  - pcurve_der_above_three_is_unbounded
budget:      {turns: 40, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub struct PCurve' vendor/truck/truck-geometry/src/decorators/mod.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub const fn curve' vendor/truck/truck-geometry/src/decorators/pcurve.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub const fn surface' vendor/truck/truck-geometry/src/decorators/pcurve.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'fn der3' vendor/truck/truck-geometry/src/decorators/pcurve.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'ParametricCurve for PCurve<C, S>' vendor/truck/truck-geometry/src/decorators/pcurve.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'impl EnclosureCurve for BSplineCurve<Point3>' vendor/truck/truck-evidence/src/bspline.rs"}
  - {id: A7, expect: 1, cmd: "grep -c 'fn hull_of' vendor/truck/truck-evidence/src/bspline.rs"}
  - {id: A8, expect: 1, cmd: "grep -c 'pub mod pcurve' vendor/truck/truck-evidence/src/decorators/mod.rs"}
  - {id: A9, expect: 1, cmd: "grep -c 'impl EnclosureSurface for Plane' vendor/truck/truck-evidence/src/plane.rs"}
  - {id: A10, expect: 1, cmd: "grep -c 'impl EnclosureSurface for Sphere' vendor/truck/truck-evidence/src/sphere.rs"}
  - {id: A11, expect: 1, cmd: "grep -c 'EnclosureSurface for ExtrudedCurve<C, Vector3>' vendor/truck/truck-evidence/src/decorators/extruded.rs"}
  - {id: A12, expect: 1, cmd: "grep -c 'pub fn assert_encloses_curve' vendor/truck/truck-evidence/src/harness.rs"}
```

## Problem

`truck-evidence` has enclosure impls for the analytic carriers, for
`BSplineCurve<Point3>` and `NurbsCurve<Vector4>` by the convex-hull property,
and for three decorators (`ExtrudedCurve`, `Processor`, `RevolutedCurve`).
This packet adds the fourth decorator: **`PCurve`** — a curve living in a
surface's parameter space. `PCurve<C, S>`'s `subs(t)` is
`surface.subs(curve.subs(t).x, curve.subs(t).y)`: the 2D parameter curve
`C` composed with the surface `S` (read the carrier at
`truck-geometry/src/decorators/pcurve.rs` — its `der`, `der2`, `der3` are the
chain rule, spelled there in exactly the forms this packet reuses).

A decorator's enclosure is a **composition**, never a re-derivation: call the
inner carriers' `enclose`/`enclose_der` and combine the boxes. Here the
composition has two levels:

1. **Hull the parameter curve in 2D.** The parameter curve is a
   `BSplineCurve<Point2>` — the *same* convex-hull machinery `bspline.rs`
   lands for `Point3` (sub-curve extraction, control-point hull, `HULL_PAD`,
   boundary values, out-of-range → unbounded), just over two coordinates and
   producing a parameter box `(uu, vv)` instead of a `Box3`.
2. **Take the surface's enclosure over that parameter box.**
   `{ S(c(t)) : t ∈ tt } ⊆ { S(u, v) : (u, v) ∈ uu × vv } ⊆
   surface.enclose(uu, vv)` — BG-ENC-001's soundness of the inner carrier,
   applied to the hulled parameter image. The derivative boxes compose by
   the chain rule in inari over the surface's `enclose_der` boxes and the
   parameter hodograph hulls.

**Do NOT evaluate the composed parameterization directly** (no interval
substitution into `S(c(t))`, no sampling in the implementation). A submission
that re-derives the surface's enclosure instead of composing is rejected for
missing the design even if it is sound.

## Decisions already made for you

1. **One existing file**, `vendor/truck/truck-evidence/src/decorators/pcurve.rs`.
   It is already created and already declared as `pub mod pcurve;` in
   `decorators/mod.rs`, itself declared in `lib.rs`. **`lib.rs` and
   `decorators/mod.rs` are read-only for you** — editing either is a scope
   violation that will get this packet rejected. The declarations were landed
   up front by the orchestrator so the sibling decorator packets have disjoint
   write sets; your file currently holds only a scaffolding doc comment, which
   you replace. The crate-level `#![deny(...)]` covers your module; do not add
   a second header. Follow `bspline.rs` for the hull helper set and doc tone
   and `decorators/extruded.rs` for the cone construction (decision 6).

2. **The impl is concrete in the curve, generic in the surface:**

   ```rust
   impl<S: EnclosureSurface> EnclosureCurve for PCurve<BSplineCurve<Point2>, S>
   ```

   `EnclosureSurface: ParametricSurface<Point = Point3>` gives
   `PCurve<BSplineCurve<Point2>, S>: ParametricCurve<Point = Point3>` via the
   carrier's own impl (`C: ParametricCurve2D`). If the compiler asks for one
   more bound, add the minimum and record it in `deviations`.

3. **The 2D hull: copy `bspline.rs`'s helper set, two coordinates.** Copy
   `knot_multiplicity`, `raise_to_full_multiplicity`, `sub_curve`, `min_max`,
   `hull_interval`, `hull_min_max` from `bspline.rs` (sibling duplication is
   deliberate and not a deviation — do not share it and do not report it).
   Local changes: a `Coord`-style trait over `Point2`/`Vector2`
   (`0..=1` by fields `.x`, `.y` — fields, not `Index`, H-1), and the hull is
   a pair of intervals rather than a `Box3` — write it as one private
   function

   ```text
   fn hull2_of(bsp: &BSplineCurve<P>, tt: Interval) -> (Interval, Interval)
   ```

   with `bspline.rs`'s `hull_of` case analysis verbatim: empty/non-finite
   `tt` → `(EMPTY, EMPTY)`; clamp into the knot range; `lo > hi` → empty;
   `lo == hi` → the point hull; else the sub-curve hull **including the
   boundary values** `subs(lo)`, `subs(hi)` (the degree-0 boundary union —
   read its comment in `bspline.rs`), everything padded by
   `HULL_PAD (1 + |·|)` with `HULL_PAD = 64.0 * f64::EPSILON` exactly as
   landed. For the parameter **hodographs** (decision 5) the same function
   runs on `derivation()` chains (`Point2` differentiates to `Vector2`, both
   are `ControlPoint<f64> + Tolerance`).

4. **`enclose(tt)` — total behavior, all cases spelled.** With `(kmin, kmax)`
   the parameter curve's knot range:

   - `tt` empty or non-finite → `Box3::empty()`.
   - `tt` reaching **outside the knot range** (`tt.inf() < kmin || tt.sup() >
     kmax`) → **the unbounded box, returned directly** — `Interval::ENTIRE`
     per axis, the convention `bspline.rs` and `nurbs.rs` landed (the
     parameter basis extrapolates outside the range; there is no origin
     union). **Do not forward an unbounded parameter box into
     `surface.enclose`**: the landed surface carriers' behavior on
     unbounded input boxes is not uniform (`bspline.rs`'s `hull_of` returns
     the EMPTY box for non-finite `tt`, and an empty composed box would
     under-estimate). Return `unbounded_box()` yourself.
   - Otherwise `(uu, vv) = hull2_of(curve, tt)` and return
     `self.surface().enclose(uu, vv)`.

   State the composition argument in the doc comment: the hulled parameter
   image contains every `c(t)`, so the surface's BG-ENC-001 box over the
   hull contains every `S(c(t))`.

5. **`enclose_der(n, tt)` — the chain rule over boxes.** Write one private
   helper per order (they differ only in the formula; do not abstract):

   - `n == 0` → `self.enclose(tt)`.
   - Empty `tt` → empty box; out-of-range `tt` → the unbounded box (decision
     4's rule, same reason).
   - Otherwise, with `(uu, vv) = hull2_of(curve, tt)`, `(cu, cv) = hull2_of`
     of the first hodograph over `tt`, `(cuu, cvv) = hull2_of` of the
     second, `(cuuu, cvvv) = hull2_of` of the third, and the surface boxes
     `S_mn = surface.enclose_der(m, n, uu, vv)` — all products/sums below
     are **inari interval operations** (outward-rounded; `[a]·[b]` is
     `a * b` on `Interval`):

     **n = 1** (the carrier's `der`):
     `D1_c = S_10.c · cu + S_01.c · cv`

     **n = 2** (the carrier's `der2`):
     `D2_c = S_20.c · (cu·cu) + S_11.c · (cu·cv·2) + S_02.c · (cv·cv)
     + S_10.c · cuu + S_01.c · cvv`

     **n = 3** (the carrier's `der3`, same term order):
     `D3_c = S_30.c·(cu·cu·cu) + S_21.c·(cu·cu·cv·3) + S_12.c·(cu·cv·cv·3)
     + S_03.c·(cv·cv·cv) + S_20.c·(cuu·cu·3) + S_11.c·((cuu·cv +
     cvv·cu)·3) + S_02.c·(cvv·cv·3) + S_10.c·cuuu + S_01.c·cvvv`

     (the scalar coefficients are exact small integers; fold them into the
     interval products as f64 literals `3.0` — dimensionless term counts,
     no H-3 comment needed). Every box over-estimates its true set and
     interval arithmetic is monotone, so each `Dn` over-estimates
     `{ der_n(t) : t ∈ tt }` — say so in the doc comment, and note the
     decorrelation over-estimation grows with `n` (acceptable, BG-ENC-001
     permits over-estimation).

   - **`n ≥ 4` → the unbounded box**, documented in the doc comment: the
     fourth-order chain rule is Faà di Bruno over surface partials, no
     kernel consumer asks past third order (the carrier itself special-cases
     `der`/`der2`/`der3`), and a sound widest box is the honest answer
     rather than an unverified formula.

6. **`tangent_cone(tt)`** — the ball-around-midpoint cone off the `n = 1`
   box, the identical construction as `bspline.rs` decision 7 and
   `extruded.rs`: midpoint `c`, half-width `h`, `rho = ‖h‖` rounded up,
   `cn = ‖c‖` rounded down, the guard in exactly the landed order and form
   `if !cn.is_finite() || !rho.is_finite() || cn <= rho { return None; }`
   (the `neg_cmp_op_on_partial_ord` trap; the finiteness tests are what make
   the clippy-clean form NaN-equivalent), `axis = c.normalize()`, half-angle
   `asin(rho/cn)` nudged by the house form
   `* (1.0 + 8.0 * f64::EPSILON) + 8.0 * f64::EPSILON`, clamped by a named
   `MAX_HALF_ANGLE` const. `None` is the derivative-hull-contains-zero case
   — for this carrier that is the parameter curve's velocity vanishing (a
   cusp in parameter space) or both surface partials degenerating at a
   pole. Copy `bspline.rs`'s comments; the reasoning transfers verbatim.

## Constructing witnesses in tests

All in the `#[cfg(test)]` module of `pcurve.rs`, using
`crate::harness::assert_encloses_curve` and the `circle.rs` literal style
(named consts; same-line `// H-3` opt-outs where noted).

- **PCurve over `Plane`** — the tight witness. `Plane`'s enclosure is exact
  interval arithmetic (no pad), so composition tightness is observable.
  Parameter curve: `BSplineCurve::new(KnotVec::bezier_knot(2), vec![Point2,
  Point2, Point2])` with dyadic control points, e.g. ordinates giving
  `c(t) = (t, t²)` on `[0, 1]`; the composed image `S(c(t))` is then a
  polynomial on the plane with a closed form to assert against.
- **PCurve over `Sphere`** — the trig witness: a parameter-space Bézier arc
  of your choosing (read `sphere.rs` for the parameterization and pick an
  equatorial or meridional arc with dyadic endpoint parameters); every
  sampled point satisfies `|p − centre| == r` to machine precision.
- **PCurve over `ExtrudedCurve`** — decorator-on-decorator composition:
  `ExtrudedCurve` of a `BSplineCurve<Point3>` (its `EnclosureSurface` is
  landed, anchor A11); assert soundness by sampling only.
- **The degenerate cases**: a constant parameter curve (all control points
  equal — the image is one surface point); the box `[0.25, 0.25]`.
- **The cone-refusal witness**: the parameter curve whose derivative
  vanishes — the quadratic Bernstein ordinates `[0, −1/2, 0]` on either
  axis has `c'(1/2) = 0`, so `der = S_u·0 + S_v·0 = 0` and the cone must be
  `None` on any box containing `t = 1/2`.

## Tests required

1. `pcurve_encloses_sampled_points` — `assert_encloses_curve` with ≥ 30
   samples on each of: the plane witness (full range and an interior
   sub-box); the sphere witness; the extruded-composition witness; the
   constant curve; `[0.25, 0.25]` on the plane witness.
2. `pcurve_out_of_range_box_is_unbounded` — boxes with `lo < kmin`, `hi >
   kmax`, and a large `[-10, 10]` on the plane witness → per-axis
   `Interval::ENTIRE`; an interior box is finite on every axis.
3. `pcurve_der_enclosures_match_partials` — for the plane and sphere
   witnesses, `enclose_der(1..=3, tt)` contains the curve's own `der_n`
   sampled over a grid (≥ 20 points per box), for interior sub-boxes. The
   plane witness's `der_n` have closed forms — assert those too.
4. `pcurve_tangent_cone_contains_sampled_tangents` — on a plane-witness box
   away from `t = 1/2`, the cone contains every sampled unit tangent
   (`cos(angle) >= cos(half_angle) − slack`, H-3-commented, test-local
   helper with a comment).
5. `pcurve_tangent_cone_refuses_when_the_derivative_hull_contains_zero` —
   `None` for the cone-refusal witness on any box containing `t = 1/2`;
   `None` for the constant curve everywhere; `Some` for a box bounded away
   from both.
6. `pcurve_subbox_enclosure_is_tighter_than_full_range` — on the plane
   witness, the enclosure of an interior sub-box is strictly narrower than
   the full-range enclosure in at least one coordinate, and the full-range
   box contains every sampled point of the sub-box (both sound; only one is
   tight).
7. `pcurve_enclosure_converges_under_bisection` — 16 bisections toward a
   point on the plane witness: non-increasing width (up to an
   H-3-commented slack for the pad) and final width below the starting
   width by a factor only bisection explains (`< initial / 16`, slack
   commented).
8. `pcurve_der_above_three_is_unbounded` — `enclose_der(4, tt)` and
   `enclose_der(7, tt)` are the per-axis unbounded box on an interior box
   (documents the deliberate ceiling of decision 5).

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
file and read the tail. The existing suite (128 lib + 3 integration as this
packet was written; sibling shards may add more before you fork — the gate is
zero failures on tests you did not add) must keep passing unchanged.

## Forbidden

Editing any file outside `write_allow` — `lib.rs` and `decorators/mod.rs`
especially. Changing the `EnclosureCurve`/`EnclosureSurface` traits, the
harness, or any existing carrier or decorator. Evaluating the composed
parameterization directly in the implementation (interval or f64 — sampling
lives in tests only). Forwarding an unbounded or empty parameter box into
`surface.enclose` instead of deciding the total behavior yourself (decision
4). Returning `Some` from `tangent_cone` for a hull failing `cn > rho`.
Adding `#[ignore]`. Adding `unscaled_legacy(` call sites. Committing to
`main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- the composition cannot be made to preserve BG-ENC-001 soundness (a sampled
  point escapes) in a way you cannot correct within this design → `SPEC_GAP`,
  with the failing witness and the escaping sample
- `PCurve<BSplineCurve<Point2>, S>` fails a bound this design relies on
  (`ParametricCurve<Point = Point3>` via the carrier's impl, `Point2`/
  `Vector2` as `ControlPoint + Tolerance`, `derivation()` chains) →
  `SPEC_GAP`, naming the bound and the compiler error
- the carrier's `der`/`der2`/`der3` differ from the chain-rule forms decision
  5 copies (your anchors pass but the described behavior does not hold) →
  `SPEC_GAP`, with the counterexample
- `inari` lacks a primitive this design needs → `SPEC_GAP`, naming it; do not
  hand-roll directed rounding
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-DONE status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(evidence): EnclosureCurve for PCurve (BG-ENC-004-PCURVE)`.
