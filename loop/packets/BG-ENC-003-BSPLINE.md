# WORK PACKET BG-ENC-003-BSPLINE — `EnclosureCurve for BSplineCurve<Point3>`

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-ENC-003-BSPLINE","status":"DONE","contracts":["BG-ENC-001","BG-ENC-002"],
 "tests_added":7,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: if a decision below is wrong,
say so rather than working around it.

```yaml
id:          BG-ENC-003-BSPLINE
contract:    [BG-ENC-001, BG-ENC-002]
class:       mechanical
crates:      [truck-evidence]
depends_on:  [BG-ENC-002-LINE]
write_allow:
  - vendor/truck/truck-evidence/src/bspline.rs
read_allow:
  - vendor/truck/truck-evidence/src/lib.rs
  - vendor/truck/truck-evidence/src/enclosure.rs
  - vendor/truck/truck-evidence/src/harness.rs
  - vendor/truck/truck-evidence/src/line.rs
  - vendor/truck/truck-evidence/src/circle.rs
  - vendor/truck/truck-evidence/src/plane.rs
  - vendor/truck/truck-evidence/src/decorators/extruded.rs
  - vendor/truck/truck-geometry/src/nurbs/bspcurve.rs
  - vendor/truck/truck-geometry/src/nurbs/knot_vec.rs
tests_required:
  - bspline_encloses_sampled_points
  - bspline_out_of_range_box_unions_the_origin
  - bspline_der_enclosures_match_partials
  - bspline_tangent_cone_contains_sampled_tangents
  - bspline_tangent_cone_refuses_when_the_hodograph_hull_contains_zero
  - bspline_enclosure_is_tighter_than_naive_interval_arithmetic
  - bspline_enclosure_converges_under_bisection
budget:      {turns: 40, ctx_tokens: 95000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub const fn control_points' vendor/truck/truck-geometry/src/nurbs/bspcurve.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn derivation' vendor/truck/truck-geometry/src/nurbs/bspcurve.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub fn roughly_bounding_box' vendor/truck/truck-geometry/src/nurbs/bspcurve.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'fn cut(&mut self, mut t: f64)' vendor/truck/truck-geometry/src/nurbs/bspcurve.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'impl EnclosureCurve for Line<Point3>' vendor/truck/truck-evidence/src/line.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub fn assert_encloses_curve' vendor/truck/truck-evidence/src/harness.rs"}
  - {id: A7, expect: 1, cmd: "grep -c 'pub mod bspline' vendor/truck/truck-evidence/src/lib.rs"}
  - {id: A8, expect: 1, cmd: "grep -c 'fn add_knot(&mut self, x: f64)' vendor/truck/truck-geometry/src/nurbs/bspcurve.rs"}
  - {id: A9, expect: 1, cmd: "grep -c 'pub fn multiplicity' vendor/truck/truck-geometry/src/nurbs/knot_vec.rs"}
```

## Problem

`truck-evidence` has the enclosure interface (BG-ENC-001) and six analytic
carriers plus three decorators. This packet adds the first **spline** carrier:
`BSplineCurve<Point3>`, whose parameterization is not closed-form but a basis
sum. The enclosure is *not* computed by evaluating that sum in interval
arithmetic.

**The convex-hull property is the technique and it is the whole item.** Over a
knot span, a B-spline lies in the convex hull of its control points. Extract
the sub-curve over `tt` by knot insertion, then bound the control points: the
axis-aligned box of the sub-curve's control points contains every curve point
over `tt`. This is *tighter* than naive interval arithmetic (which suffers
dependency loss across the basis sum) and *cheaper* (no interval basis
evaluation). `BSplineCurve::roughly_bounding_box` in
`truck-geometry/src/nurbs/bspcurve.rs` documents exactly this — "the bounding
box including all control points" — but you hull the **sub-curve's** control
points, not the whole curve's, and you widen endpoints one ulp outward (see
decision 4).

The tangent cone comes off the **hodograph**: `BSplineCurve::derivation()`
returns the derivative curve, whose control points are the scaled forward
differences. The derivative hull contains 0 exactly where the tangent
direction is undefined — that is the `None` case.

**Do NOT use naive interval arithmetic on the basis sum.** A submission whose
`enclose` evaluates `Σ Nᵢ(t)·Pᵢ` in inari is rejected for missing the design
even if it is sound.

## Decisions already made for you

1. **One existing file**, `vendor/truck/truck-evidence/src/bspline.rs`. It is
   already created and already declared as `pub mod bspline;` in `lib.rs`,
   which is **read-only for you** — it is not on your `write_allow` and
   editing it is a scope violation that will get this packet rejected. The
   declaration was made up front, by the orchestrator, so that this packet and
   the parallel analytic fan-out have disjoint write sets; your file currently
   holds only a scaffolding doc comment, which you replace. The crate-level
   `#![deny(...)]` in `lib.rs` covers your module; do not add a second header.
   Follow `line.rs` and `circle.rs` for structure and doc tone; read
   `decorators/extruded.rs` for the ball-around-midpoint cone construction
   (decision 7) — it is the same construction with "tangent" substituted for
   "normal".

2. **The impl is `impl EnclosureCurve for BSplineCurve<Point3>`.**
   `BSplineCurve<Point3>: ParametricCurve<Point = Point3>` already holds via
   `ControlPoint<f64>`; no extra bounds. If the compiler asks for one more
   bound, add the minimum and record it in `deviations`.

3. **Sub-curve extraction over `tt` — the exact construction, spelled out.**
   Write one private helper:

   ```text
   fn sub_curve(bsp: &BSplineCurve<Point3>, lo: f64, hi: f64) -> BSplineCurve<Point3>
   ```

   where `lo < hi` are **already clamped into the knot range**
   `[knot_vec[0], knot_vec[len-1]]`. Two steps, both from the public API:

   - **Raise `lo` and `hi` to full knot multiplicity by `add_knot`, checking
     multiplicity first.** Inserting beyond multiplicity `degree + 1` produces
     an invalid knot vector, and `add_knot` does not check — so loop: query
     the current multiplicity of `x` via `knot_vec().floor(x)` (if the knot at
     that index equals `x`, `knot_vec().multiplicity(idx)`, else 0), and call
     `bsp.add_knot(x)` until the multiplicity is `degree + 1`. This is Boehm
     insertion: each call inserts one exact copy.
   - **Then cut twice and keep the middle.** `Cut::cut(&mut self, t)` mutates
     `self` into the part **before** `t` and **returns** the part after
     `t` (read the impl at the anchor A4 site to confirm). So:

     ```text
     let mut c = raised.clone(); // after the add_knot loop, above
     let _tail = c.cut(hi);      // c is now [front, hi]
     c.cut(lo)                   // RETURNS [lo, hi]; c keeps [front, lo]
     ```

     Because `lo` and `hi` now have full multiplicity, `cut`'s
     tolerance-based snapping (it snaps `t` to a nearby knot under
     `ToleranceCtx`) is exact here: `t` *is* the knot, `t − t == 0.0`, and
     `cut` inserts zero further copies. That is why the `add_knot` loop comes
     first — calling `cut` on un-raised knots can snap the boundary by up to
     the tolerance, and the sliver it fails to remove is an under-estimation,
     which BG-ENC-001 calls a silent wrong answer.

   **Why the hull of the sub-curve is the right box:** over `[lo, hi]` the
   basis functions of the *extracted* curve are non-negative and sum to 1, so
   every `subs(t)` for `t ∈ [lo, hi]` is a convex combination of the
   sub-curve's control points. Its axis-aligned bounding box therefore
   contains the curve image. This is the convex-hull property; state it in
   the doc comment.

4. **Widen every hull endpoint one ulp outward.** Boehm insertion computes new
   control points in `f64` (`(1−a)·P + a·Q` with `a = (x−kⱼ)/(kⱼ₊d−kⱼ)`), so
   a computed point can sit an ulp outside the true hull. After collecting
   `min`/`max` of the sub-curve's control-point coordinates, build each
   `inari::Interval` from `(f64::next_down(min), f64::next_up(max))`. This is
   the same outward-rounding discipline the other carriers get from inari
   arithmetic for free; say so in the doc comment.

5. **`enclose(tt)`** — total behavior, all cases spelled:

   - `tt` empty or non-finite (NaN bounds, `inf > sup`) → `Box3::empty()`.
   - Clamp `(lo, hi) = (tt.inf().max(kmin), tt.sup().min(kmax))`. If
     `lo >= hi`: the active range contributes nothing; fall through to the
     origin-union step below with an empty hull.
   - If `lo < hi`: `sub_curve` per decision 3, hull per decision 4.
   - **Origin union:** if `tt` extends beyond the knot range — `tt.inf() <
     kmin || tt.sup() > kmax` — union the box with `Box3::point(Point3::origin())`.
     Reason: `subs(t) = Σ Nᵢ(t)·Pᵢ` is defined for *every* `t` (no panic; read
     the `der_n` impl — it is pure basis evaluation), the Cox–de Boor basis
     functions are non-negative *everywhere*, and outside the active domain
     they sum to at most 1 — so the image lies in the convex hull of the
     control points together with the origin. Inside the domain they sum to
     exactly 1 and the origin is not needed. Hulling in the origin for
     out-of-range `tt` is sound over-estimation; skipping it is the bug.

6. **`enclose_der(n, tt)`**:

   - `n == 0` → `self.enclose(tt)`.
   - `n >= 1` → the `n`-fold hodograph: apply `derivation()` `n` times
     (`derivation()` on `BSplineCurve<Vector3>` yields another
     `BSplineCurve<Vector3>`; `Vector3` is a `ControlPoint`). The derivative
     curve's `subs(t)` reproduces `der_n(t, self)` on the same basis
     evaluation path, so the enclosure of the hodograph over `tt` — by the
     **identical hull construction** of decisions 3–5 (sub-curve, hull,
     origin union) — encloses `{ der_n(t) : t ∈ tt }`. Write it as one shared
     private `fn hull_of(bsp: &BSplineCurve<...>, tt: Interval) -> Box3` used
     by both `enclose` and `enclose_der`, generic over the point type or
     monomorphized twice; your choice, but do not duplicate the body.

7. **`tangent_cone(tt)`** — the ball-around-midpoint cone off the hodograph
   hull, the same construction `decorators/extruded.rs` uses for normals:

   ```text
   let b = hull_of(first hodograph over tt)      // encloses { der(t) }
   c  = midpoint vector of b,  h = half-width vector of b
   rho = ‖h‖ computed in inari, take .sup()      (round UP)
   cn  = ‖c‖ computed in inari, take .inf()      (round DOWN)
   if !cn.is_finite() || !rho.is_finite() || cn <= rho  ->  None
   axis = c.normalize()
   half_angle = asin(rho / cn), nudged UP, clamped to at most PI
   ```

   Write the guard in exactly that order and that form — `!(cn > rho)` is
   rejected by clippy's `neg_cmp_op_on_partial_ord` under `-D warnings`, and
   the explicit finiteness tests are what make the clippy-clean form
   equivalent (they differ on NaN). They are load-bearing: the empty box or a
   hull containing the origin yields NaN or zero `cn`, and without them you
   would return `Some` with a garbage cone. `rho >= cn` is precisely "the
   hull contains the origin or straddles enough directions that no cone
   bounds it" — including the whole point of this method, the derivative
   crossing zero (a cusp or an inflection with horizontal tangent). Nudge the
   half-angle by the house form `* (1.0 + 8.0 * f64::EPSILON) + 8.0 *
   f64::EPSILON`; `f64::EPSILON` is a named constant and needs no H-3
   comment. Name the `PI` clamp as a `const` with a word on what it is.
   Declare in the doc comment that this is sound but loose the same way
   `extruded.rs`'s is: it bounds the hull, not the true derivative set.

8. **A sibling packet does not exist.** `nurbs.rs` is scaffolded but is a
   separate later packet (BG-ENC-003-NURBS, blocked on this one). Do not
   implement NURBS, do not edit `nurbs.rs`, and do not add
   `NonPositiveNurbsWeight` handling — that refusal belongs to the NURBS
   packet.

## Constructing witnesses in tests

`KnotVec::bezier_knot(degree)` gives the clamped uniform knots on `[0, 1]`,
so `BSplineCurve::new(KnotVec::bezier_knot(d), control_points)` is a Bézier
curve — the hull property is exact and the polynomial is known. Useful
witnesses, with **dyadic** control points so the hull endpoints are exact:

- the quadratic `x(t) = t² − t` on `[0, 1]`: control ordinates `[0, −1/2, 0]`
  (Bernstein form), true range `[−1/4, 0]`, hull `[−1/2, 0]`;
- the cubic `t³ − t`: control ordinates `[0, −1/3, −2/3, 0]`;
- a helix-like 3D cubic with mixed-sign coordinates of your choosing;
- a non-Bézier witness: `KnotVec::uniform_knot(degree, division)` with
  several control points, boxes straddling interior knots.

## Tests required

All in the `#[cfg(test)]` module of `bspline.rs`, using the shared harness
`crate::harness::assert_encloses_curve` and the `circle.rs` test style for
literals (named consts; a `// H-3` same-line opt-out where a bare float is
unavoidable — note rustfmt moves trailing comments off brace-opening lines).

1. `bspline_encloses_sampled_points` — `assert_encloses_curve` with ≥ 30
   samples on each of: an interior sub-box of a Bézier witness; a box
   straddling an interior knot of the uniform witness; the full `[0, 1]`;
   `[0.25, 0.25]` (degenerate point box — hull is the point, up to the ulp
   widening); a box with negative `lo` and one with `hi > 1` (they exercise
   the origin union together with decision 5); and a large box like
   `[-10, 10]`.
2. `bspline_out_of_range_box_unions_the_origin` — for a witness whose hull
   excludes the origin, `enclose` of a box entirely beyond the knot range
   contains `Point3::origin()`; and for a box entirely inside, it does not
   (the origin is not unioned when `tt` is inside the range — assert the box
   does not contain the origin).
3. `bspline_der_enclosures_match_partials` — for the cubic witnesses,
   `enclose_der(1..=3, tt)` contains the curve's own `der_n` sampled over a
   grid (≥ 20 points per box), for `tt` interior and straddling a knot.
4. `bspline_tangent_cone_contains_sampled_tangents` — on a box where the
   derivative does not vanish, the returned cone contains every sampled unit
   tangent `der(t).normalize()`, tested by angle: `cos(angle between axis
   and d) >= cos(half_angle) - slack`. Implement the angle test as a small
   test-local helper with a comment.
5. `bspline_tangent_cone_refuses_when_the_hodograph_hull_contains_zero` —
   `None` for the quadratic `t² − t` on any box containing `t = 1/2` (the
   derivative `2t − 1` vanishes there), and `None` for a full-period witness
   whose derivative hull covers every direction; `Some` for a box bounded
   away from both.
6. `bspline_enclosure_is_tighter_than_naive_interval_arithmetic` — the test
   that justifies the design. For the two power-form witnesses above, compute
   the **naive** enclosure by Horner's rule in inari on the power basis
   (`t² − t` → `tt * tt - tt`; `t³ − t` → `(tt * tt - 1.0) * tt` — write each
   as one expression in inari interval arithmetic), per coordinate. Assert
   the hull enclosure is contained in the naive one and **strictly narrower**
   (at least one coordinate with `width_hull < width_naive`, on both
   witnesses). Also assert the containment direction is the one that matters:
   the naive box contains sampled curve points too (both are sound; only one
   is tight).
7. `bspline_enclosure_converges_under_bisection` — from a starting box,
   bisect 16 times toward a point; the hull width is non-increasing (up to an
   H-3-commented slack) and the final width is below the starting width by a
   factor that only bisection-convergence explains (assert final < initial /
   16, say, with the slack commented).

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

Directions, angles, direction cosines, parameter values and interval bounds are
all dimensionless and all legitimate — the comment is what says so. A literal
that really is a model-space *length* does not get an opt-out; it goes through
`ToleranceCtx` instead. Run `bash scripts/kernel-gates.sh` yourself before you
write `RESULT.json`; it is the same script V4 runs.

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

Editing any file outside `write_allow` — `lib.rs` especially, which is already
correct. Changing the `EnclosureCurve` trait, the harness, or any existing
carrier or decorator. Naive interval arithmetic on the basis sum anywhere in
the implementation (tests may use inari Horner as the *baseline* — that is
decision 6's test). Returning `Some` from `tangent_cone` for a hull failing
`cn > rho`. Endpoint-only evaluation of anything. Adding `#[ignore]`. Adding
`unscaled_legacy(` call sites. Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- the sub-curve construction of decision 3 cannot be made to preserve
  `BG-ENC-001` soundness (a sampled point escapes) that you cannot fix within
  this design → `SPEC_GAP`, with the failing witness and the escaping sample
- `inari` lacks a primitive this design needs → `SPEC_GAP`, naming it; do not
  hand-roll directed rounding
- `cut` or `add_knot` semantics differ from what decision 3 states (your
  anchors pass but the described behavior does not hold) → `SPEC_GAP`, with the
  counterexample
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-DONE status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(evidence): EnclosureCurve for BSplineCurve (BG-ENC-003-BSPLINE)`.
