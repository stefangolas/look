# WORK PACKET BG-ENC-004-PROCESSOR — enclosure for the `Processor` decorator

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-ENC-004-PROCESSOR","status":"DONE","contracts":["BG-ENC-001","BG-ENC-002"],
 "tests_added":6,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: if a decision below is wrong,
say so rather than working around it.

```yaml
id:          BG-ENC-004-PROCESSOR
contract:    [BG-ENC-001, BG-ENC-002]
class:       mechanical
crates:      [truck-evidence]
depends_on:  [BG-ENC-002-LINE]
write_allow:
  - vendor/truck/truck-evidence/src/decorators/processor.rs
read_allow:
  - vendor/truck/truck-evidence/src/lib.rs
  - vendor/truck/truck-evidence/src/decorators/mod.rs
  - vendor/truck/truck-evidence/src/elementary.rs
  - vendor/truck/truck-evidence/src/enclosure.rs
  - vendor/truck/truck-evidence/src/harness.rs
  - vendor/truck/truck-evidence/src/plane.rs
  - vendor/truck/truck-evidence/src/cylinder.rs
  - vendor/truck/truck-evidence/src/sphere.rs
  - vendor/truck/truck-geometry/src/decorators/processor.rs
  - vendor/truck/truck-geometry/src/decorators/mod.rs
tests_required:
  - processor_encloses_sampled_points
  - processor_inverted_orientation_swaps_the_parameters
  - processor_der_enclosures_match_partials
  - processor_normal_cone_contains_sampled_normals
  - processor_immersion_lower_bound_is_a_true_lower_bound
  - processor_enclosure_converges_under_bisection
budget:      {turns: 40, ctx_tokens: 95000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub const fn entity' vendor/truck/truck-geometry/src/decorators/processor.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub const fn transform' vendor/truck/truck-geometry/src/decorators/processor.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub const fn orientation' vendor/truck/truck-geometry/src/decorators/processor.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'impl<S, T> ParametricSurface for Processor<S, T>' vendor/truck/truck-geometry/src/decorators/processor.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'impl EnclosureSurface for Plane' vendor/truck/truck-evidence/src/plane.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub mod processor' vendor/truck/truck-evidence/src/decorators/mod.rs"}
  - {id: A7, expect: 4, cmd: "grep -c 'pub fn assert_' vendor/truck/truck-evidence/src/harness.rs"}
```

## Problem

`truck-evidence` has the enclosure interface (BG-ENC-001) and six analytic
carriers, all of which evaluate a closed-form parameterisation. This packet adds
a **decorator**: a carrier whose enclosure is a *composition*, computed by
calling an inner carrier's `enclose`/`enclose_der` and combining the boxes.

`Processor<S, Matrix4>` is the placement decorator — it is how every transformed
surface in this kernel arrives. It holds an inner surface, a matrix, and a
`bool` orientation.

**There are two traps here and both are in `processor.rs`'s own
`ParametricSurface` impl. Read it before you write anything.**

**Trap 1 — `orientation == false` swaps `u` and `v`. It does not merely flip a
sign.** The carrier's own code is

```rust
fn subs(&self, u: f64, v: f64) -> Self::Point {
    match self.orientation {
        true  => self.transform.transform_point(self.entity.subs(u, v)),
        false => self.transform.transform_point(self.entity.subs(v, u)),
    }
}
```

and `der_mn` does the same to *both* the orders and the arguments:
`self.entity.der_mn(n, m, v, u)`. An enclosure that only negates a normal is
**unsound** — it will report a box that does not contain the surface. The
sampling test in this packet exists to catch exactly that.

**Trap 2 — `transform_point` is projective, `transform_vector` is linear.**
cgmath's `Transform<Point3> for Matrix4` is

```rust
fn transform_vector(&self, vec: Vector3) -> Vector3 { (self * vec.extend(0.0)).truncate() }
fn transform_point(&self, point: Point3) -> Point3 { Point3::from_homogeneous(self * point.to_homogeneous()) }
```

`from_homogeneous` **divides by `w`**. For the affine matrices this kernel
actually uses (bottom row `(0, 0, 0, 1)`) the divide is by exactly `1.0` and the
transform is exact; but the type does not promise that, and an enclosure may not
assume it. Decision 4 below handles the general case in three extra lines.

## Decisions already made for you

1. **One existing file**, `vendor/truck/truck-evidence/src/decorators/processor.rs`.
   It is already created and already declared as `pub mod processor;` in
   `vendor/truck/truck-evidence/src/decorators/mod.rs`, which is itself already
   declared as `pub mod decorators;` in `lib.rs`. **Both `lib.rs` and
   `decorators/mod.rs` are read-only for you** — they are not on your
   `write_allow` and editing either is a scope violation that will get this
   packet rejected. The declarations were made up front, by the orchestrator, so
   that the sibling decorator packets have disjoint write sets and can run in
   parallel; your file currently holds only a scaffolding doc comment, which you
   replace. The crate-level `#![deny(...)]` in `lib.rs` covers your module; do
   not add a second header. Follow `plane.rs` for structure, doc tone, and the
   private `interval_at` helper (define your own copy).

2. **Scope: surfaces only, and `Matrix4` only.** The impl is

   ```rust
   impl<S: EnclosureSurface<Vector = Vector3>> EnclosureSurface for Processor<S, Matrix4>
   ```

   Both restrictions are deliberate and neither is up for renegotiation:

   - **`Matrix4` only.** Every `Processor` instantiation in this tree is
     `Processor<_, Matrix4>` — `canonical.rs`, `truck-modeling/builder.rs`,
     `truck-meshalgo`'s formal witnesses, all of them. `Processor<_, Matrix3>`
     for a surface does not occur. Writing a second impl for it would double the
     test surface for a case nothing constructs.
   - **Surfaces only.** `Processor` is also a `ParametricCurve`, but that impl
     reverses its *parameter* through `get_curve_parameter`, which needs
     `C: BoundedCurve` and a range flip — a differently shaped job. It is a
     separate item; do not attempt it here.
   - The `Vector = Vector3` bound is required and is not redundant:
     `EnclosureSurface` is bounded `ParametricSurface<Point = Point3>` only, and
     `ParametricSurface::Vector` is a free associated type. If the compiler asks
     for one more bound than this, add the minimum it asks for and record it in
     `deviations`.

3. **Resolve the orientation swap once, at the top of each method.** Do not
   scatter `match self.orientation` through the arithmetic. The whole of Trap 1
   is:

   ```text
   let (au, av) = if self.orientation() { (uu, vv) } else { (vv, uu) };
   let (bm, bn) = if self.orientation() { (m, n) } else { (n, m) };
   ```

   `enclose` uses `(au, av)`; `enclose_der` uses `(bm, bn)` **and** `(au, av)`.
   Everything downstream is orientation-agnostic. In particular you do **not**
   need to flip the normal cone by hand: with the parameters swapped,
   `enclose_der(1, 0, ..)` already returns an enclosure of the *outer* `S_u`,
   which is the transformed inner `S_v`, and the generic cross product in
   decision 5 gets the reversed normal for free. Adding a manual sign flip on
   top would double-flip it.

4. **`enclose(uu, vv)`** — the interval homogeneous transform of the inner box.
   `cgmath::Matrix4` is **column-major**: `m.x`, `m.y`, `m.z`, `m.w` are its
   four *columns*, each a `Vector4`. Use the field accessors, not indexing (the
   crate denies `clippy::indexing_slicing`). With
   `let b = self.entity().enclose(au, av)` and `let m = *self.transform()`:

       nx = interval_at(m.x.x)*b.x + interval_at(m.y.x)*b.y + interval_at(m.z.x)*b.z + interval_at(m.w.x)
       ny = interval_at(m.x.y)*b.x + interval_at(m.y.y)*b.y + interval_at(m.z.y)*b.z + interval_at(m.w.y)
       nz = interval_at(m.x.z)*b.x + interval_at(m.y.z)*b.y + interval_at(m.z.z)*b.z + interval_at(m.w.z)
       w  = interval_at(m.x.w)*b.x + interval_at(m.y.w)*b.y + interval_at(m.z.w)*b.z + interval_at(m.w.w)

       if w.contains(0.0) or w is empty  ->  return the ENTIRE box
                                             (Interval::ENTIRE on all three axes)
       else                              ->  Box3 { x: nx/w, y: ny/w, z: nz/w }

   **Why interval arithmetic rather than "hull the eight mapped corners", which
   is what a reader of the spec might expect.** For an affine map the two are
   equally tight — each output coordinate is a linear function in which every
   input interval appears exactly once, so interval arithmetic has no dependency
   loss and returns precisely the bounding box of the mapped box. The interval
   form additionally gets outward rounding for free (BG-ENC-003) and extends to
   the projective case, which a corner hull does not. Put that sentence in the
   doc comment.

   The `w.contains(0.0)` arm is a sound fallback for a matrix that projects part
   of the box to infinity. It cannot be reached by an affine matrix, where `w`
   is the degenerate interval at `1.0` and the division is exact — say so in a
   comment so the next reader does not think it is dead code.

5. **`enclose_der(m, n, uu, vv)`** — mirror `Processor`'s own `der_mn`:

   - `(0, 0)` → `self.enclose(uu, vv)`. **This is deliberate and it is the
     crate's convention**: `der_mn(0, 0)` returns `subs(u, v).to_vec()`, a
     vector whose components equal the point's coordinates, so the zeroth
     enclosure is the point box. `line.rs` and `cone.rs` document the same
     choice. Note that `plane.rs` and `cylinder.rs` return the zero box here
     instead; they are the outliers, they are not your files, and you must not
     copy them on this point.
   - otherwise → the **linear** part of the matrix applied to
     `self.entity().enclose_der(bm, bn, au, av)`. That is the same three row
     sums as decision 4 **without** the `m.w` column and **without** the
     `w` divide, mirroring `transform_vector` exactly.

   A consequence worth stating so you do not relitigate it: for a *non-affine*
   bottom row, truck's `der_mn` is not the derivative of truck's `subs`. That is
   a property of the carrier, and this trait's contract is to enclose
   `der_mn` — the doc comment on `EnclosureSurface::enclose_der` says so.
   Mirror the carrier. Note it in `notes`, not in `deviations`.

6. **`normal_cone` and `immersion_lower_bound` both go through one private
   helper each, on the interval cross product.** This is the construction the
   whole BG-ENC-004 family uses and it is pre-decided here so you do not have to
   invent one:

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
   `{ p × q : p ∈ a, q ∈ b }`, a superset of `{ S_u(x) × S_v(x) : x ∈ box }`,
   because it lets `p` and `q` vary independently when in truth they are
   evaluated at the same point. Over-estimation is always acceptable
   (BG-ENC-001); tightening is not your job.

   **Do not try to be clever about the transform here.** A tempting shortcut is
   to take the inner surface's `immersion_lower_bound` and scale it by something
   read off the matrix. There is no such single factor: for a linear `A`,
   `(A p) × (A q) = det(A) · A^{-T} (p × q)`, so the cross product is mapped by
   the *inverse transpose* and its norm does not scale uniformly. The
   cross-product-of-boxes route above avoids the question entirely and is what
   you must implement.

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
     or straddle too many directions for any cone to bound — including a
     singular immersion — so the `None` arm is the contract, not a convenience.
     Nudge the half-angle upward by a few ulps
     (`* (1.0 + 8.0 * f64::EPSILON) + 8.0 * f64::EPSILON` is the house form) so
     that the f64 `asin` and the `normalize()` cannot round the cone too narrow;
     `f64::EPSILON` is a named constant and does not need an H-3 comment.
     Name the `PI` clamp as a `const` with a word on what it is.

7. **A sibling packet writes a near-identical private helper in its own file.**
   BG-ENC-004-EXTRUDED and BG-ENC-004-REVOLVED both need the same cross-product
   cone. That duplication is deliberate and known: the three packets must have
   **disjoint write sets** so they can run in parallel. Do not attempt to share
   it, do not create a shared module, and do not edit `decorators/mod.rs` to
   host it. Consolidating the three copies is a later refactor and is explicitly
   not in scope. Do not mention it in `deviations`; it is not a deviation.

8. **No changes to `enclosure.rs`, `harness.rs`, any existing carrier, or
   anything under `truck-geometry`.** If you find yourself wanting to touch the
   `EnclosureSurface` trait, that is a SPEC_GAP, not an edit.

## Interval trigonometry, if your tests need it

`inari::Interval` has **no** `sin`/`cos` in this tree — they are behind
`inari`'s `gmp` feature and `truck-evidence` takes `inari` with
`default-features = false`. Use `use crate::elementary::{cos, sin};` and write
`cos(uu)`, never `uu.cos()`. That module is BG-ENC-005, added in session 11 for
exactly this. Your own impl needs no trig — the inner carrier does it — but a
test that builds an expected value may.

## Tests required

All in the `#[cfg(test)]` module of `processor.rs`, using the shared harness
(`crate::harness::{assert_encloses_surface, assert_converges}`) and the
`plane.rs` / `cone.rs` test style for literals (named consts; a `// H-3`
same-line opt-out where a bare float is unavoidable — note rustfmt moves
trailing comments off brace-opening lines).

Your inner witnesses are the crate's existing carriers: `Plane` (affine, so the
composition is exactly checkable), `Cylinder` and `Sphere` (curved, so the
composition is the interesting case). Build the processor with
`Processor::with_transform(entity, matrix)` and invert it with
`Invertible::invert` / `inverse` (which is what sets `orientation` to `false` —
the field is private and there is no other way in; if that turns out not to be
reachable from your allowlist, say so in `disagreements` rather than reaching
into private fields).

**Use a matrix with translation, rotation and non-uniform scale in every test.**
An identity or pure-translation matrix passes a transposed row/column mistake,
which is the single most likely defect in decision 4.

1. `processor_encloses_sampled_points` — several boxes over at least two inner
   carriers, `assert_encloses_surface` with at least 20 samples per axis. Include
   a box with negative parameter values and, for the curved carriers, one arc
   crossing `π/2` and one spanning more than `π`.
2. `processor_inverted_orientation_swaps_the_parameters` — the trap test, and
   the most important one here. Take a processor over a **non-symmetric** inner
   surface (one where `subs(a, b) != subs(b, a)`; a `Cylinder` patch or a
   `Plane` with different `u` and `v` axes will do), invert it, and assert that
   `enclose(uu, vv)` contains the sampled `subs(u, v)` of the *inverted*
   processor over an asymmetric box `uu != vv`. Then assert positively that the
   inverted `enclose(uu, vv)` equals the upright `enclose(vv, uu)`. A
   swap-blind implementation fails both.
3. `processor_der_enclosures_match_partials` — `(0,0)`, `(1,0)`, `(0,1)`,
   `(2,0)`, `(1,1)` and `(0,2)` enclosures contain the processor's own `der_mn`
   sampled over a grid, for both orientations.
4. `processor_normal_cone_contains_sampled_normals` — the returned cone
   **contains** every sampled unit normal `(S_u × S_v).normalize()` over a grid,
   tested by angle, for both orientations; and the two orientations' axes point
   into opposite half-spaces (their dot product is negative), which is the
   normal reversal falling out of decision 3 rather than being applied by hand.
   Also assert `None` for a full `2π` sweep of a transformed `Cylinder`, where
   the normals cover every direction around the axis.
5. `processor_immersion_lower_bound_is_a_true_lower_bound` — the returned value
   is `<=` the sampled `‖S_u × S_v‖` at *every* grid point (that is the
   property; do not assert equality), over both a scaled and an unscaled matrix,
   which is what would catch a "scale the inner bound by one factor" shortcut.
6. `processor_enclosure_converges_under_bisection` — `assert_converges` from a
   moderate box, depth ~20.

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
trait, the harness, or any existing carrier. Scaling an inner
`immersion_lower_bound` by a factor read off the matrix. Applying a manual sign
flip to the normal cone on top of the parameter swap. Adding a second impl for
`Matrix3` or for curves. Adding `#[ignore]`. Adding `unscaled_legacy(` call
sites. Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- the impl bound in decision 2 cannot be made to compile with at most one added
  bound → `SPEC_GAP`, naming the compiler's exact requirement
- `orientation = false` turns out not to be reachable from your allowlist →
  `SPEC_GAP`, naming what you tried; do not reach into private fields
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(evidence): EnclosureSurface for Processor (BG-ENC-004-PROCESSOR)`.
