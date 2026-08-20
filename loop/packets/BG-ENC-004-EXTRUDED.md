# WORK PACKET BG-ENC-004-EXTRUDED — enclosure for the `ExtrudedCurve` decorator

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-ENC-004-EXTRUDED","status":"DONE","contracts":["BG-ENC-001","BG-ENC-002"],
 "tests_added":6,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: if a decision below is wrong,
say so rather than working around it.

```yaml
id:          BG-ENC-004-EXTRUDED
contract:    [BG-ENC-001, BG-ENC-002]
class:       mechanical
crates:      [truck-evidence]
depends_on:  [BG-ENC-002-LINE, BG-ENC-002-CIRCLE]
write_allow:
  - vendor/truck/truck-evidence/src/decorators/extruded.rs
read_allow:
  - vendor/truck/truck-evidence/src/lib.rs
  - vendor/truck/truck-evidence/src/decorators/mod.rs
  - vendor/truck/truck-evidence/src/elementary.rs
  - vendor/truck/truck-evidence/src/enclosure.rs
  - vendor/truck/truck-evidence/src/harness.rs
  - vendor/truck/truck-evidence/src/plane.rs
  - vendor/truck/truck-evidence/src/line.rs
  - vendor/truck/truck-evidence/src/circle.rs
  - vendor/truck/truck-geometry/src/decorators/extruded_curve.rs
  - vendor/truck/truck-geometry/src/decorators/mod.rs
tests_required:
  - extruded_encloses_sampled_points
  - extruded_der_enclosures_match_partials
  - extruded_normal_cone_contains_sampled_normals
  - extruded_normal_cone_refuses_when_the_tangent_meets_the_extrusion
  - extruded_immersion_lower_bound_is_a_true_lower_bound
  - extruded_enclosure_converges_under_bisection
budget:      {turns: 38, ctx_tokens: 90000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub const fn entity_curve' vendor/truck/truck-geometry/src/decorators/extruded_curve.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub const fn extruding_vector' vendor/truck/truck-geometry/src/decorators/extruded_curve.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'impl<C> ParametricSurface for ExtrudedCurve<C, C::Vector>' vendor/truck/truck-geometry/src/decorators/extruded_curve.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'impl EnclosureCurve for Line<Point3>' vendor/truck/truck-evidence/src/line.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'impl EnclosureCurve for UnitCircle<Point3>' vendor/truck/truck-evidence/src/circle.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub mod extruded' vendor/truck/truck-evidence/src/decorators/mod.rs"}
  - {id: A7, expect: 4, cmd: "grep -c 'pub fn assert_' vendor/truck/truck-evidence/src/harness.rs"}
```

## Problem

`truck-evidence` has the enclosure interface (BG-ENC-001) and six analytic
carriers, all of which evaluate a closed-form parameterisation. This packet adds
the first **decorator**: a carrier whose enclosure is a *composition*, computed
by calling an inner carrier's `enclose`/`enclose_der` and combining the boxes.
Nothing here evaluates a parameterisation directly.

`ExtrudedCurve<C, Vector3>` sweeps a curve along a constant vector:

    S(u, v) = C(u) + v·V

with `u` the curve's own parameter and `v` ranging over `[0, 1]` by default
(but **`v` is not clamped** — `subs` accepts any `v`, and your enclosure must
accept any `vv`, including negative and mixed-sign). It is affine in `v`, which
is what makes it the easy member of the BG-ENC-004 family and the right one to
establish the composition pattern on.

**The singular locus is the point of this packet.** `S_u × S_v = C'(u) × V`,
which vanishes exactly where the curve's tangent is parallel (or antiparallel)
to the extrusion vector. There the surface has no normal — a line extruded along
its own direction is a degenerate strip, not a plane. That is the `None` case of
`normal_cone` and the `0.0` case of `immersion_lower_bound`.

## Decisions already made for you

1. **One existing file**, `vendor/truck/truck-evidence/src/decorators/extruded.rs`.
   It is already created and already declared as `pub mod extruded;` in
   `vendor/truck/truck-evidence/src/decorators/mod.rs`, which is itself already
   declared as `pub mod decorators;` in `lib.rs`. **Both `lib.rs` and
   `decorators/mod.rs` are read-only for you** — they are not on your
   `write_allow` and editing either is a scope violation that will get this
   packet rejected. The declarations were made up front, by the orchestrator, so
   that the sibling decorator packets have disjoint write sets and can run in
   parallel; your file currently holds only a scaffolding doc comment, which you
   replace. The crate-level `#![deny(...)]` in `lib.rs` covers your module; do
   not add a second header. Follow `plane.rs` and `line.rs` for structure, doc
   tone, and the private `interval_at` helper (define your own copy — the
   existing ones are private to their modules).

2. **The impl is**

   ```rust
   impl<C: EnclosureCurve<Vector = Vector3>> EnclosureSurface for ExtrudedCurve<C, Vector3>
   ```

   The `Vector = Vector3` bound is required and is not redundant:
   `EnclosureCurve` is bounded `ParametricCurve<Point = Point3>` only, and
   `ParametricCurve::Vector` is a free associated type, so without it
   `ExtrudedCurve<C, Vector3>` is not a `ParametricSurface` and the impl will
   not compile. If the compiler asks for one more bound than this, add the
   minimum it asks for and record it in `deviations`.

3. **`enclose(uu, vv)`**: `C.enclose(uu)` shifted by `vv·V`, componentwise:

       x = c.x + vv * interval_at(V.x)
       y = c.y + vv * interval_at(V.y)
       z = c.z + vv * interval_at(V.z)

   where `c = self.entity_curve().enclose(uu)` and `V = self.extruding_vector()`.
   All in `inari` arithmetic, which rounds outward for you. `vv` is signed and
   inari handles mixed-sign multiplication correctly — do **not** hand-roll a
   sign case analysis and do not assume `vv.inf() >= 0`.

4. **`enclose_der(m, n, uu, vv)`** — mirror `ExtrudedCurve`'s own `der_mn`
   exactly, which is a four-arm match you can read off `extruded_curve.rs`:
   - `(0, 0)` → `self.enclose(uu, vv)`. **This is deliberate and it is the
     crate's convention**: `der_mn(0, 0)` returns `subs(u, v).to_vec()`, a
     vector whose components equal the point's coordinates, so the zeroth
     enclosure is the point box. `line.rs` documents the same choice ("Match the
     carrier; do not 'fix' it"). Note that `plane.rs` and `cylinder.rs` return
     the zero box here instead; they are the outliers, they are not your files,
     and you must not copy them on this point.
   - `(0, 1)` → the degenerate box at `V`, i.e. `interval_at(V.x)` and friends.
     Exact.
   - `(m, 0)` for `m >= 1` → `self.entity_curve().enclose_der(m, uu)`,
     delegated unchanged.
   - everything else (`m >= 1 && n >= 1`, and `n >= 2`) → the zero box, because
     `S` is affine in `v`.

5. **`normal_cone` and `immersion_lower_bound` both go through one private
   helper each, on the interval cross product.** This is the construction the
   whole BG-ENC-004 family uses and it is pre-decided here so you do not have to
   invent it:

   ```text
   let a = self.enclose_der(1, 0, uu, vv);   // encloses S_u
   let b = self.enclose_der(0, 1, uu, vv);   // encloses S_v
   let n = cross_box(a, b);                  // encloses { S_u × S_v }
   ```

   with the interval cross product written out componentwise:

       n.x = a.y*b.z - a.z*b.y
       n.y = a.z*b.x - a.x*b.z
       n.z = a.x*b.y - a.y*b.x

   **Say in the doc comment that this is sound but loose:** it encloses
   `{ p × q : p ∈ a, q ∈ b }`, which is a superset of
   `{ S_u(x) × S_v(x) : x ∈ box }` because it lets `p` and `q` vary
   independently when in truth they are evaluated at the same point. Over-
   estimation is always acceptable (BG-ENC-001); tightening is not your job.
   Here `b` is a *degenerate* box (`V` is constant), so the looseness is small.

   - **`immersion_lower_bound(uu, vv)`** is the smallest `‖n‖` over that box:

         sqrt(mig(n.x)^2 + mig(n.y)^2 + mig(n.z)^2)

     using `inari::Interval::mig` (the mignitude — `0.0` if the interval
     contains zero, else `min(|inf|, |sup|)`). Each coordinate attains its
     mignitude independently, so this is exactly the box's minimum norm, and
     since the box contains the true set it is a valid lower bound on the true
     minimum. **Compute it in `inari` and return `.inf()` of the result**, not
     in `f64`: returning a value one rounding unit too large is a soundness bug,
     not a tightness one. Return `0.0` for a non-finite or empty result.

   - **`normal_cone(uu, vv)`** turns the same box into a `DirCone`. Let `c` be
     the box's midpoint vector (`n.x.mid()`, …) and `h` its half-width vector
     (`n.x.wid() / 2.0`, …). Every element of the box lies within distance
     `rho = ‖h‖` of `c`, so if `rho < ‖c‖` every element makes an angle of at
     most `asin(rho / ‖c‖)` with `c`. Therefore:

         rho  = ‖h‖  computed in inari, take .sup()   (round UP)
         cn   = ‖c‖  computed in inari, take .inf()   (round DOWN)
         if !cn.is_finite() || !rho.is_finite() || cn <= rho  ->  None

     Write the guard in that order and in that form. The natural phrasing
     `!(cn > rho)` is what this packet said until BG-ENC-004-EXTRUDED and
     -REVOLVED both reported it: clippy's `neg_cmp_op_on_partial_ord` rejects it
     under `-D warnings`. The two are **not** interchangeable on their own --
     `!(x > y)` is true for NaN and `x <= y` is false -- so the explicit
     finiteness tests are what makes the clippy-clean form equivalent, and they
     are load-bearing rather than defensive: an empty or entire cross-product
     box yields a NaN or infinite `cn`, and without them it would return `Some`
     with a garbage cone instead of `None`.
         axis       = c.normalize()
         half_angle = asin(rho / cn), nudged UP, clamped to at most PI

     `rho >= cn` is exactly the case where the box may contain the zero vector
     or straddle enough directions that no cone bounds it — including the
     singular locus this packet exists to detect — so the `None` arm is the
     contract, not a convenience. Nudge the half-angle upward by a few ulps
     (`* (1.0 + 8.0 * f64::EPSILON) + 8.0 * f64::EPSILON` is the house form) so
     that the f64 `asin` and the `normalize()` cannot round the cone too narrow;
     `f64::EPSILON` is a named constant and does not need an H-3 comment.
     Name the `PI` clamp as a `const` with a word on what it is.

6. **A sibling packet writes a near-identical private helper in its own file.**
   BG-ENC-004-PROCESSOR and BG-ENC-004-REVOLVED both need the same cross-product
   cone. That duplication is deliberate and known: the three packets must have
   **disjoint write sets** so they can run in parallel. Do not attempt to share
   it, do not create a shared module, and do not edit `decorators/mod.rs` to
   host it. Consolidating the three copies is a later refactor and is explicitly
   not in scope. Do not mention it in `deviations`; it is not a deviation.

7. **No changes to `enclosure.rs`, `harness.rs`, `plane.rs`, `line.rs`,
   `circle.rs`, or anything under `truck-geometry`.** If you find yourself
   wanting to touch the `EnclosureSurface` trait, that is a SPEC_GAP, not an
   edit.

## Interval trigonometry, if your tests need it

`inari::Interval` has **no** `sin`/`cos` in this tree — they are behind
`inari`'s `gmp` feature and `truck-evidence` takes `inari` with
`default-features = false`. Use `use crate::elementary::{cos, sin};` and write
`cos(uu)`, never `uu.cos()`. That module is BG-ENC-005, added in session 11 for
exactly this. Your own impl should not need trig at all — the inner curve does
it — but a test that computes an expected value may.

## Tests required

All in the `#[cfg(test)]` module of `extruded.rs`, using the shared harness
(`crate::harness::{assert_encloses_surface, assert_converges}`) and the
`plane.rs` / `cone.rs` test style for literals (named consts; a `// H-3`
same-line opt-out where a bare float is unavoidable — note rustfmt moves
trailing comments off brace-opening lines).

Your two witnesses are the crate's two `EnclosureCurve` impls:
`Line<Point3>` (from `line.rs`) and `UnitCircle<Point3>` (from `circle.rs`).
`UnitCircle<Point3>` is `(cos t, sin t, 0)`. Extruding it along `z` is a
cylinder; extruding a `Line` along a non-parallel vector is a plane patch;
extruding a `Line` along **its own direction** is the degenerate case.
Construct with `ExtrudedCurve::by_extrusion(curve, vector)`.

1. `extruded_encloses_sampled_points` — several boxes over both witnesses,
   including: a small arc of the extruded circle; an arc crossing `π/2`; one
   spanning more than `π`; a full `2π` sweep; a box with `vv` entirely negative;
   and a box whose `vv` straddles zero. `assert_encloses_surface` with at least
   20 samples per axis.
2. `extruded_der_enclosures_match_partials` — for both witnesses, the `(0,0)`,
   `(1,0)`, `(0,1)`, `(2,0)`, `(1,1)` and `(0,2)` enclosures contain the
   surface's own `der_mn` sampled over a grid. Assert `(0,1)` is exactly the
   extrusion vector and that `(1,1)` and `(0,2)` are the zero box.
3. `extruded_normal_cone_contains_sampled_normals` — extruded circle, a moderate
   arc: the returned cone **contains** every sampled unit normal
   `(S_u × S_v).normalize()` over a grid, tested by angle.
4. `extruded_normal_cone_refuses_when_the_tangent_meets_the_extrusion` — `None`
   for a `Line` extruded along its own direction (singular everywhere), and
   `None` for a full `2π` sweep of the extruded circle, where the normals cover
   every horizontal direction and no cone bounds them. `Some` for a moderate
   arc bounded away from that.
5. `extruded_immersion_lower_bound_is_a_true_lower_bound` — for cells where the
   surface is an immersion, the returned value is `<=` the sampled
   `‖S_u × S_v‖` at *every* grid point (that is the property; do not assert
   equality). Exactly `0.0` for the `Line`-along-its-own-direction case.
6. `extruded_enclosure_converges_under_bisection` — `assert_converges` on the
   extruded circle from a moderate box, depth ~20.

`DirCone` containment by angle: `cos(angle between axis and d) >= cos(half_angle)`
— implement as a small test-local helper with a comment; a `half_angle` at or
near `π/2` needs the `>=` with float tolerance to survive rounding.

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
file and read the tail. The existing 56 tests must keep passing unchanged.

## Forbidden

Editing any file outside `write_allow` — `lib.rs` and `decorators/mod.rs`
especially, both of which are already correct. Changing the `EnclosureSurface`
trait, the harness, or any existing carrier. Returning `Some` from `normal_cone`
for a cell whose cross-product box fails `cn > rho`. Endpoint-only evaluation
of anything. Adding `#[ignore]`. Adding `unscaled_legacy(` call sites.
Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- the impl bound in decision 2 cannot be made to compile with at most one added
  bound → `SPEC_GAP`, naming the compiler's exact requirement
- `inari` lacks a primitive this design needs → `SPEC_GAP`, naming it; do not
  hand-roll directed rounding
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(evidence): EnclosureSurface for ExtrudedCurve (BG-ENC-004-EXTRUDED)`.
