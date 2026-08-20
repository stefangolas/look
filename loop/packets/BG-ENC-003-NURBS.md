# WORK PACKET BG-ENC-003-NURBS — `EnclosureCurve for NurbsCurve<Vector4>`

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-ENC-003-NURBS","status":"DONE","contracts":["BG-ENC-001","BG-ENC-002"],
 "tests_added":8,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: if a decision below is wrong,
say so rather than working around it.

```yaml
id:          BG-ENC-003-NURBS
contract:    [BG-ENC-001, BG-ENC-002]
class:       mechanical
crates:      [truck-evidence]
depends_on:  [BG-ENC-003-BSPLINE]
write_allow:
  - vendor/truck/truck-evidence/src/nurbs.rs
read_allow:
  - vendor/truck/truck-evidence/src/lib.rs
  - vendor/truck/truck-evidence/src/enclosure.rs
  - vendor/truck/truck-evidence/src/harness.rs
  - vendor/truck/truck-evidence/src/bspline.rs
  - vendor/truck/truck-evidence/src/line.rs
  - vendor/truck/truck-evidence/src/circle.rs
  - vendor/truck/truck-geometry/src/nurbs/mod.rs
  - vendor/truck/truck-geometry/src/nurbs/nurbscurve.rs
  - vendor/truck/truck-geometry/src/nurbs/bspcurve.rs
  - vendor/truck/truck-geometry/src/nurbs/knot_vec.rs
  - vendor/truck/truck-base/src/evidence.rs
  - vendor/truck/truck-base/src/cgmath_extend_traits.rs
tests_required:
  - nurbs_encloses_sampled_points
  - nurbs_negative_weight_is_refused
  - nurbs_out_of_range_box_is_unbounded
  - nurbs_der_enclosures_match_partials
  - nurbs_tangent_cone_contains_sampled_tangents
  - nurbs_tangent_cone_refuses_when_the_derivative_hull_contains_zero
  - nurbs_subbox_enclosure_is_tighter_than_full_range
  - nurbs_enclosure_converges_under_bisection
budget:      {turns: 42, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub struct NurbsCurve' vendor/truck/truck-geometry/src/nurbs/mod.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub const fn non_rationalized' vendor/truck/truck-geometry/src/nurbs/nurbscurve.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub const fn control_points' vendor/truck/truck-geometry/src/nurbs/nurbscurve.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub fn try_from_bspline_and_weights' vendor/truck/truck-geometry/src/nurbs/nurbscurve.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'ParametricCurve for NurbsCurve<V>' vendor/truck/truck-geometry/src/nurbs/nurbscurve.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'fn rat_der<V: Homogeneous>' vendor/truck/truck-base/src/cgmath_extend_traits.rs"}
  - {id: A7, expect: 1, cmd: "grep -c 'impl<S: BaseFloat> Homogeneous for Vector4<S>' vendor/truck/truck-base/src/cgmath_extend_traits.rs"}
  - {id: A8, expect: 1, cmd: "grep -c 'pub fn derivation' vendor/truck/truck-geometry/src/nurbs/bspcurve.rs"}
  - {id: A9, expect: 1, cmd: "grep -c 'pub mod nurbs' vendor/truck/truck-evidence/src/lib.rs"}
  - {id: A10, expect: 1, cmd: "grep -c 'impl EnclosureCurve for BSplineCurve<Point3>' vendor/truck/truck-evidence/src/bspline.rs"}
  - {id: A11, expect: 1, cmd: "grep -c 'fn hull_of' vendor/truck/truck-evidence/src/bspline.rs"}
  - {id: A12, expect: 1, cmd: "grep -c 'NonPositiveNurbsWeight' vendor/truck/truck-base/src/evidence.rs"}
```

## Problem

`BG-ENC-003-BSPLINE` landed `EnclosureCurve for BSplineCurve<Point3>` by the
convex-hull property: extract the sub-curve over `tt` by Boehm knot insertion,
bound its control points. **Read `bspline.rs` first — it is the template for
everything here, including its four recorded deviations, which this packet
inherits as landed behavior rather than re-deriving.**

This packet adds the **rational** spline: `NurbsCurve<Vector4>`, a newtype over
`BSplineCurve<Vector4>` whose control points are *homogeneous* —
`(w·x, w·y, w·z, w)` — and whose `subs(t)` projects by the perspective divide
(`NurbsCurve::subs` = `non_rationalized().subs(t).to_point()`). Two things
change relative to the plain B-spline:

1. **The hull property lives in homogeneous coordinates.** The 4D curve
   `A(t) = Σ Nᵢ(t)·(wᵢxᵢ, wᵢyᵢ, wᵢzᵢ, wᵢ)` is an ordinary `BSplineCurve<Vector4>`;
   sub-curve extraction and control-point hulling bound it exactly as
   `bspline.rs` bounds a `Point3` curve. The 3D image is then bounded by
   **projecting the 4D box**: each coordinate is `interval[coord] /
   interval[w]` in inari (outward-rounded division). Project *after* bounding,
   never before — the projection of a hull is not the hull of the projection
   unless every weight is positive.

2. **The weights can break the curve.** A non-positive weight means the
   denominator `Σ Nᵢ(t)·wᵢ` can vanish on the domain: the "curve" is not a
   well-defined rational curve at all. That is a **refusal** —
   `EnvelopeCase::NonPositiveNurbsWeight` exists in truck-base for exactly this
   — never a silently mis-enclosed box.

**Do NOT use naive interval arithmetic on the rational basis sum** (neither the
numerator sum nor the denominator sum). A submission whose `enclose` evaluates
the basis in inari is rejected for missing the design even if it is sound.

## Decisions already made for you

1. **One existing file**, `vendor/truck/truck-evidence/src/nurbs.rs`. It is
   already created and already declared as `pub mod nurbs;` in `lib.rs`, which
   is **read-only for you** — it is not on your `write_allow` and editing it is
   a scope violation that will get this packet rejected. The declaration was
   made up front, by the orchestrator, so that this packet and its siblings
   have disjoint write sets; your file currently holds only a scaffolding doc
   comment, which you replace. The crate-level `#![deny(...)]` in `lib.rs`
   covers your module; do not add a second header. Follow `bspline.rs` for
   structure, doc tone and the helper set; it is the nearest landed sibling.

2. **Two public surfaces, deliberately.** `EnclosureCurve::enclose` returns a
   bare `Box3` and cannot carry a refusal, so:

   - the trait impl `impl EnclosureCurve for NurbsCurve<Vector4>` is **total**:
     on a non-positive-weight curve it degrades to the *widest sound* answers
     (`enclose`/`enclose_der` → the unbounded box, `tangent_cone` → `None`),
     documented as such in the doc comment — it can never return a narrow box
     for a curve it cannot bound;
   - the certified entry, which you also write, is the one that refuses:

   ```rust
   pub fn try_enclose(curve: &NurbsCurve<Vector4>, tt: Interval) -> Outcome<Box3>
   ```

   returning `Err(Refusal::UnsupportedEnvelope(EnvelopeCase::NonPositiveNurbsWeight))`
   when any weight is non-positive (decision 3), and `Ok(Certified { .. })`
   otherwise. `Outcome`, `Certified`, `Refusal`, `EnvelopeCase` and friends are
   imported from `truck_base::evidence` exactly as the analytic modules import
   them.

3. **The positive-weight gate, exact form.** Weights are carrier data, not
   computed values, so a plain f64 comparison is decisive — no intervals:

   ```rust
   let positive = curve.control_points().iter().all(|v| v.weight() > 0.0);
   ```

   `weight()` is `Homogeneous::weight()` (the fourth coordinate; no indexing,
   H-1). NaN fails `> 0.0` and is refused with the same arm — state that in a
   comment. Do **not** write `!(w <= 0.0)` (differs on NaN; clippy's
   `neg_cmp_op_on_partial_ord` also bites related forms). Note the gate checks
   the **source** curve's control points: the sub-curve's weights are convex
   combinations of these under Boehm insertion, so positivity is preserved
   along the way — say so in the doc comment, it is the soundness link.

4. **The certificate, field-by-field at the `Ok` site** — deliberately no
   helper (BG-EVD-002), same discipline as the analytic shards:

   ```rust
   let mut props = PropMap::new();
   props.set(Prop::SoundEnclosure, Truth::True);
   Certified::new(
       box3,
       Certificate {
           props,
           method: Method::Interval,
           budget_left: Budget::new(0, 0, 0),
           margin: Margin::UNBOUNDED,
           modulus: Modulus::Unbounded,
       },
   )
   ```

   Doc-comment what `Method::Interval` means here: the hull endpoints are f64
   `min`/`max` padded outward by a relative `HULL_PAD` and the projection is an
   outward-rounded inari division — no step in the construction rounds inward.
   `SoundEnclosure` is the BG-ENC-001 prop: the box provably contains the image.
   No `τ_rep` anywhere.

5. **The homogeneous hull: copy `bspline.rs`'s helper set, extended to four
   coordinates.** Copy `knot_multiplicity`, `raise_to_full_multiplicity`,
   `sub_curve`, `min_max`, `hull_interval`, `hull_min_max` from `bspline.rs`
   (sibling duplication is deliberate and not a deviation — do not share and do
   not report it). Three local changes:

   - extend the local `Coord` trait with `impl Coord for Vector4`
     (`0..=3` by fields `x, y, z, w` — fields, not `Index`, H-1);
   - the hull is four intervals, not a `Box3` — write a private struct

     ```rust
     struct Hull4 { x: Interval, y: Interval, z: Interval, w: Interval }
     ```

     built exactly as `hull_sub_curve` builds its three, **including the two
     boundary values** `non_rationalized().subs(lo)` and `subs(hi)` (the
     degree-0 boundary union, `bspline.rs`'s fourth recorded deviation — read
     its comment for why; the same reasoning holds per homogeneous coordinate);
   - keep `HULL_PAD = 64.0 * f64::EPSILON` and the `HULL_PAD * (1 + |·|)`
     endpoint pad exactly as landed (the second recorded deviation — one ulp
     under-estimated; do not re-litigate it).

   All the generic bounds already hold for `P = Vector4`:
   `Vector4: ControlPoint<f64, Diff = Vector4>` (so `derivation()` chains),
   `Vector4: Tolerance` (blanket `AbsDiffEq` impl), `BSplineCurve<Vector4>`:
   `Cut` — verified at this packet's writing.

6. **`enclose(tt)` — total behavior, all cases spelled** (mirroring
   `hull_of`, whose case analysis you copy):

   - `tt` empty or non-finite → `Box3::empty()`.
   - Non-positive weight (decision 3) → the unbounded box
     (`Interval::ENTIRE` per axis).
   - **Out-of-range `tt`** (`tt.inf() < kmin || tt.sup() > kmax`) → the
     unbounded box. This **inherits `bspline.rs`'s first recorded deviation**:
     the basis window *extrapolates* outside the knot range (verified there:
     `subs(±10)` lands far outside any origin union), so there is **no origin
     union and no clamped-hull fallback** — the whole line per axis. Do not
     re-derive an origin union; it was measured false for this evaluator.
   - Clamp `(lo, hi)` into the knot range; `lo > hi` → empty box; `lo == hi` →
     the projected point box (hull the homogeneous `subs(lo)` as four padded
     degenerate intervals, then project per decision 7 — the degenerate-box
     deviation, inherited).
   - Otherwise: `Hull4` of the homogeneous sub-curve over `[lo, hi]`, then
     project (decision 7).

7. **The projection: inari interval division.**

   ```text
   Box3.x = h4.x / h4.w    Box3.y = h4.y / h4.w    Box3.z = h4.z / h4.w
   ```

   with `/` the inari `Interval` division (outward-rounded, sign-case aware).
   All sub-curve weights are positive, so `h4.w` is a positive interval up to
   its pad; if a legitimately tiny weight (≲ `HULL_PAD`) makes the padded
   `h4.w.inf()` reach zero, inari's division over a denominator straddling
   zero returns the whole line — **sound automatically, no special case**.
   Document that sentence in the code; it is why no zero-weight guard is
   needed beyond decision 3.

8. **`enclose_der(n, tt)` — the weighted-derivative recursion.** The rational
   derivative is **not** the projection of the homogeneous derivative; use the
   classical recursion instead. With `A` the homogeneous curve, `w` its weight
   coordinate and `C` the rational curve, differentiating `A⁽ⁿ⁾ = (w·C)⁽ⁿ⁾`
   and solving for `C⁽ⁿ⁾` gives

   ```text
   C⁽ⁿ⁾ = ( A⁽ⁿ⁾_xyz − Σ_{k=1..n} binom(n, k) · w⁽ᵏ⁾ · C⁽ⁿ⁻ᵏ⁾ ) / w
   ```

   Box form, by induction on `n` — every operation an inari interval op:

   ```text
   H4_k = Hull4 of the k-fold homogeneous hodograph over [lo, hi]
          (derivation() applied k times to non_rationalized(); Vector4 is its
          own ControlPoint::Diff so the chain never changes type; derivation()
          of a degree-0 curve returns the zero curve, so n past the degree
          hulls to zero without a special case)
   Box(C⁽⁰⁾)     = the projected hull (decisions 5–7)
   Box(C⁽ⁿ⁾)_c   = ( H4_n.c − Σ_{k=1..n} binom(n,k) · H4_k.w · Box(C⁽ⁿ⁻ᵏ⁾)_c ) / H4_0.w
   ```

   Soundness: each `H4_k` over-estimates the true hodograph image by the hull
   property + pad, each `Box(C⁽ⁿ⁻ᵏ⁾)` over-estimates by induction, and interval
   arithmetic is monotone — so the right-hand side over-estimates
   `{ C⁽ⁿ⁾(t) : t ∈ tt }`. State that in the doc comment. It over-estimates
   *more* as `n` grows (decorrelated repeated factors) — acceptable, BG-ENC-001
   permits over-estimation. `n == 0` → `self.enclose(tt)`. Non-positive weight
   or out-of-range/empty `tt` → the same total behavior as `enclose`.
   Binomials: a small f64 loop or table (exact integers in f64 for any `n` a
   curve degree produces; do not use a float approximation).

   **Verify the recursion numerically before committing to it** (test 4 does
   exactly this: sampled `der_n` must lie inside); if it cannot be made
   sound on the witnesses, that is a stop condition, not a pad to enlarge.

9. **`tangent_cone(tt)`** — the identical ball-around-midpoint construction as
   `bspline.rs` decision 7, off `Box(C⁽¹⁾)` from the recursion: midpoint `c`,
   half-width `h`, `rho = ‖h‖` rounded up, `cn = ‖c‖` rounded down, guard in
   exactly the landed order and form
   `if !cn.is_finite() || !rho.is_finite() || cn <= rho { return None; }`
   (the `neg_cmp_op_on_partial_ord` trap; the finiteness tests are what make
   the clippy-clean form NaN-equivalent), `axis = c.normalize()`, half-angle
   `asin(rho/cn)` nudged by the house form
   `* (1.0 + 8.0 * f64::EPSILON) + 8.0 * f64::EPSILON`, clamped by a named
   `MAX_HALF_ANGLE` const. Non-positive weight → `None` (no direction can be
   certified). Copy the landed code's comments; the reasoning transfers
   verbatim.

10. **A note you are invited to disagree with.** This packet bounds in
    homogeneous coordinates and projects the box because the spec says so and
    because `enclose_der` needs the homogeneous hulls anyway. It is *also*
    true (a convex-combination argument with positive weights) that the
    projected control points of the sub-curve give a sound and *tighter* box
    than dividing the 4D box by the weight box — the spec's "hull property
    holds in homogeneous coordinates **only**" overstates the restriction. If
    your witnesses show the divided box failing a required tightness or
    soundness test that the projected-control hull passes, record it in
    `disagreements` with the numbers; do not silently switch constructions.

## Constructing witnesses in tests

All in the `#[cfg(test)]` module of `nurbs.rs`, using
`crate::harness::assert_encloses_curve` and the `circle.rs` literal style
(named consts; same-line `// H-3` opt-outs where noted).

- **The unit circle.** `nurbs/mod.rs`'s own doc example is a 9-control-point
  quadratic NURBS circle on `[0, 1]` with weights 1 and 2 (knots
  `[0,0,0,1/4,1/4,1/2,1/2,3/4,3/4,1,1,1]`) — copy the control-point table
  from that doc comment verbatim (it is on your `read_allow`). Every sampled
  point satisfies `x² + y² == 1` to machine precision: an exact oracle, with
  mixed weights.
- **The rationalized polynomial.** `try_from_bspline_and_weights` with all
  weights `1.0` on the quadratic `t² − t` Bernstein ordinates `[0, −1/2, 0]`
  (BSPLINE's witness): the curve is the polynomial, the derivative `2t − 1`
  vanishes at `t = 1/2` — the cone-refusal witness.
- **The constant curve.** All control points `(1, 2, 3)` (any positive
  weights): `enclose` is a point box, the derivative is identically zero, the
  cone is `None` everywhere.
- **The negative weight.** `try_from_bspline_and_weights` with one weight
  `< 0.0` (and one `= 0.0` in a second assertion — both are non-positive):
  the refusal witness. Also assert the trait's `enclose` on the same curve
  returns the unbounded box (the never-silently-mis-enclosed half).
- Boxes: interior sub-boxes of `[0, 1]`, a box straddling an interior knot of
  the circle (`1/4`, `1/2`, `3/4` are knots), the degenerate `[0.25, 0.25]`,
  the full range.

## Tests required

1. `nurbs_encloses_sampled_points` — `assert_encloses_curve` with ≥ 30 samples
   on each of: the circle over the full range; an interior sub-box; a box
   straddling the knot `1/2`; the degenerate point box; the equal-weights
   polynomial witness; the constant curve.
2. `nurbs_negative_weight_is_refused` — decision 3's gate:
   `try_enclose` → `Err(UnsupportedEnvelope(NonPositiveNurbsWeight))` for a
   negative weight and for a zero weight; the trait `enclose` on the same
   curves → the unbounded box per axis; `tangent_cone` → `None`. Also
   `try_enclose` on a *valid* curve returns `Ok` whose certificate has
   `method == Method::Interval` and `SoundEnclosure` set to `Truth::True`
   (decision 4, asserted field-by-field).
3. `nurbs_out_of_range_box_is_unbounded` — a box with `lo < 0` and one with
   `hi > 1` (and `[-10, 10]`) → per-axis `Interval::ENTIRE` on the circle;
   a fully interior box is finite on every axis.
4. `nurbs_der_enclosures_match_partials` — for the circle and the polynomial
   witness, `enclose_der(1..=3, tt)` contains the curve's own `der_n`
   sampled over a grid (≥ 20 points per box), for interior and
   knot-straddling boxes. This is the recursion's soundness test; the
   certificate-witness circle with its mixed weights is the one that catches
   a wrong rational-derivative formula.
5. `nurbs_tangent_cone_contains_sampled_tangents` — on a circle arc box away
   from parameter ends, the cone contains every sampled unit tangent
   (`cos(angle) >= cos(half_angle) − slack`, H-3-commented, test-local helper
   with a comment).
6. `nurbs_tangent_cone_refuses_when_the_derivative_hull_contains_zero` —
   `None` for the polynomial witness on any box containing `t = 1/2`; `None`
   for the constant curve everywhere; `Some` for a box bounded away from
   both.
7. `nurbs_subbox_enclosure_is_tighter_than_full_range` — on the circle, the
   enclosure of an arc sub-box (say `[1/16, 1/8]`, a single-span arc) is
   strictly narrower than the full-range enclosure in at least one coordinate,
   and the full-range box contains every sampled point of the arc (both sound;
   only one is tight).
8. `nurbs_enclosure_converges_under_bisection` — 16 bisections toward a
   point on the circle: non-increasing width (up to an H-3-commented slack for
   the pad) and final width below the starting width by a factor only
   bisection explains (`< initial / 16`, slack commented).

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
file and read the tail. The existing suite (115 lib + 3 integration as this
packet was written; sibling shards may have landed more since — the gate is
zero failures on tests you did not add) must keep passing unchanged.

## Forbidden

Editing any file outside `write_allow` — `lib.rs` especially, which is already
correct. Changing the `EnclosureCurve` trait, the harness, or any existing
carrier or decorator. Naive interval arithmetic on the rational basis sums
anywhere in the implementation. Projecting control points **before** bounding
(decision 10's construction is noted, not chosen — do not switch without
recording it in `disagreements`). Returning `Some` from `tangent_cone` for a
hull failing `cn > rho`. Returning a narrow box for a non-positive-weight
curve on any path. Adding `#[ignore]`. Adding `unscaled_legacy(` call sites.
Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- the recursion of decision 8 cannot be made to contain sampled `der_n` on the
  witnesses and you cannot correct it within this design → `SPEC_GAP`, with
  the witness and the escaping sample
- `Vector4` fails a bound this design relies on (`ControlPoint<Diff =
  Vector4>`, `Tolerance`, `Cut`) in a way the packet's anchors did not show →
  `SPEC_GAP`, naming the bound and the error
- `derivation()` of a degree-0 curve does not return the zero curve as decision
  8 assumes → `SPEC_GAP`, with the counterexample
- `inari` lacks a primitive this design needs → `SPEC_GAP`, naming it; do not
  hand-roll directed rounding
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-DONE status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(evidence): EnclosureCurve for NurbsCurve (BG-ENC-003-NURBS)`.
