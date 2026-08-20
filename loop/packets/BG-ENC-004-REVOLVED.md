# WORK PACKET BG-ENC-004-REVOLVED — enclosure for the `RevolutedCurve` decorator

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-ENC-004-REVOLVED","status":"DONE","contracts":["BG-ENC-001","BG-ENC-002"],
 "tests_added":6,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: if a decision below is wrong,
say so rather than working around it.

```yaml
id:          BG-ENC-004-REVOLVED
contract:    [BG-ENC-001, BG-ENC-002]
class:       mechanical
crates:      [truck-evidence]
depends_on:  [BG-ENC-002-CIRCLE, BG-ENC-005]
write_allow:
  - vendor/truck/truck-evidence/src/decorators/revolved.rs
read_allow:
  - vendor/truck/truck-evidence/src/lib.rs
  - vendor/truck/truck-evidence/src/decorators/mod.rs
  - vendor/truck/truck-evidence/src/elementary.rs
  - vendor/truck/truck-evidence/src/enclosure.rs
  - vendor/truck/truck-evidence/src/harness.rs
  - vendor/truck/truck-evidence/src/plane.rs
  - vendor/truck/truck-evidence/src/line.rs
  - vendor/truck/truck-evidence/src/circle.rs
  - vendor/truck/truck-evidence/src/cone.rs
  - vendor/truck/truck-geometry/src/decorators/revolved_curve.rs
  - vendor/truck/truck-geometry/src/decorators/mod.rs
tests_required:
  - revolved_encloses_sampled_points
  - revolved_rotation_matrix_derivatives_match
  - revolved_der_enclosures_match_partials
  - revolved_normal_cone_contains_sampled_normals
  - revolved_immersion_lower_bound_vanishes_on_the_axis
  - revolved_enclosure_converges_under_bisection
budget:      {turns: 42, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'fn from_axis_angle_derivation' vendor/truck/truck-geometry/src/decorators/revolved_curve.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'impl<C: ParametricCurve3D> ParametricSurface for RevolutedCurve<C>' vendor/truck/truck-geometry/src/decorators/revolved_curve.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub const fn origin' vendor/truck/truck-geometry/src/decorators/revolved_curve.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub const fn axis' vendor/truck/truck-geometry/src/decorators/revolved_curve.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'impl EnclosureCurve for Line<Point3>' vendor/truck/truck-evidence/src/line.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'impl EnclosureCurve for UnitCircle<Point3>' vendor/truck/truck-evidence/src/circle.rs"}
  - {id: A7, expect: 1, cmd: "grep -c 'pub mod revolved' vendor/truck/truck-evidence/src/decorators/mod.rs"}
  - {id: A8, expect: 4, cmd: "grep -c 'pub fn assert_' vendor/truck/truck-evidence/src/harness.rs"}
```

## Problem

`truck-evidence` has the enclosure interface (BG-ENC-001) and six analytic
carriers, all of which evaluate a closed-form parameterisation. This packet adds
a **decorator**: a carrier whose enclosure is a *composition*, computed by
calling an inner carrier's `enclose`/`enclose_der` and combining the boxes.

`RevolutedCurve<C>` sweeps a profile curve around an axis. It is the most
arithmetic-heavy of the BG-ENC-004 family because the sweep is a **rotation
matrix**, so this is the one decorator whose enclosure needs interval
trigonometry of its own.

`revolved_curve.rs`'s `der_mn` is unusually clean and is the whole design:

```rust
fn der_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Vector3 {
    let center  = match (m, n) { (0, 0) => self.origin().to_vec(), _ => Vector3::zero() };
    let u_part  = match m { 0 => self.curve.subs(u) - self.origin(), _ => self.curve.der_n(m, u) };
    let v_part  = from_axis_angle_derivation(n, self.axis(), Rad(v));
    v_part * u_part + center
}
```

Every partial is `R^(n)(v) · (something the profile curve already knows) +
origin` for `(m, n) = (0, 0)`. Your job is that product in intervals.

**The singular locus is where the profile curve meets the axis.** There
`C(u) − origin` is parallel to the axis, rotating it does nothing, `S_v = 0`,
and the surface has no normal — the apex of a revolved cone, the poles of a
revolved circle. `RevolutedCurve` even carries `is_front_fixed()` and
`is_back_fixed()` as the carrier's own hints that its profile ends on the axis.
You do **not** need to call them (they require a `C: BoundedCurve` bound this
packet does not take); the generic construction in decision 5 detects the
singularity numerically. They are named here so you know what the case is.

## Decisions already made for you

1. **One existing file**, `vendor/truck/truck-evidence/src/decorators/revolved.rs`.
   It is already created and already declared as `pub mod revolved;` in
   `vendor/truck/truck-evidence/src/decorators/mod.rs`, which is itself already
   declared as `pub mod decorators;` in `lib.rs`. **Both `lib.rs` and
   `decorators/mod.rs` are read-only for you** — they are not on your
   `write_allow` and editing either is a scope violation that will get this
   packet rejected. The declarations were made up front, by the orchestrator, so
   that the sibling decorator packets have disjoint write sets and can run in
   parallel; your file currently holds only a scaffolding doc comment, which you
   replace. The crate-level `#![deny(...)]` in `lib.rs` covers your module; do
   not add a second header. Follow `plane.rs` and `cone.rs` for structure, doc
   tone, and the private `interval_at` helper (define your own copy).

2. **The impl is**

   ```rust
   impl<C: EnclosureCurve<Vector = Vector3>> EnclosureSurface for RevolutedCurve<C>
   ```

   The `Vector = Vector3` bound is required and is not redundant:
   `EnclosureCurve` is bounded `ParametricCurve<Point = Point3>` only, and
   `ParametricCurve::Vector` is a free associated type; `ParametricSurface for
   RevolutedCurve<C>` is bounded `C: ParametricCurve3D`, which is the blanket
   alias for `ParametricCurve<Point = Point3, Vector = Vector3>`. If the
   compiler asks for one more bound than this, add the minimum it asks for and
   record it in `deviations`.

3. **The interval rotation matrix is the core of this packet. Write it once, as
   a private helper**, mirroring `from_axis_angle_derivation` exactly:

   ```text
   fn rot_der(n: usize, axis: Vector3, vv: Interval) -> [[Interval; 3]; 3]
   ```

   From the source: let `S = sin(vv)` and `C = cos(vv)` (the crate's own
   certified interval pair — see the trig section below), then rotate them by
   `n % 4` exactly as the carrier does:

       n % 4 == 0  ->  (s, c) = ( S,  C)
       n % 4 == 1  ->  (s, c) = ( C, -S)
       n % 4 == 2  ->  (s, c) = (-S, -C)
       n % 4 == 3  ->  (s, c) = (-C,  S)

   and the `1 − cos` coefficient, which is **keyed on `n`, not on `n % 4`**:

       n == 0  ->  k = interval_at(1.0) - c
       n >= 1  ->  k = -c

   With `a = axis` (already unit — `Revolution::new` normalises it at
   construction, so treat its components as exact degenerate intervals), the
   nine entries are, in the same layout the carrier writes them:

       M[0][0] = k*a.x*a.x + c      M[1][0] = k*a.x*a.y - s*a.z      M[2][0] = k*a.x*a.z + s*a.y
       M[0][1] = k*a.x*a.y + s*a.z  M[1][1] = k*a.y*a.y + c          M[2][1] = k*a.y*a.z - s*a.x
       M[0][2] = k*a.x*a.z - s*a.y  M[1][2] = k*a.y*a.z + s*a.x      M[2][2] = k*a.z*a.z + c

   **`cgmath::Matrix3::new` is column-major**, so in the carrier's source the
   first three arguments are column 0 — that is `M[0][0]`, `M[0][1]`, `M[0][2]`
   above. Index your helper the same way (`M[col][row]`) so the correspondence
   with the source is line-for-line checkable, and say in a comment that you
   have done so. Getting this transposed is the most likely defect in this
   packet and test 2 exists to catch it.

4. **`enclose` and `enclose_der`.** Define, once:

       u_part(m, uu) = if m == 0 { curve.enclose(uu) - origin }   // componentwise
                       else      { curve.enclose_der(m, uu) }

   (`curve.enclose(uu)` is a `Box3`; subtract `interval_at(origin.c)` from each
   coordinate.) Then

       enclose_der(m, n, uu, vv) = rot_der(n, axis, vv) * u_part(m, uu)
                                   + (origin if (m, n) == (0, 0) else 0)

   where the matrix-box product is the ordinary three row sums in intervals,

       out_r = M[0][r]*p.x + M[1][r]*p.y + M[2][r]*p.z

   and `enclose(uu, vv) = enclose_der(0, 0, uu, vv)`. Write `enclose` in terms
   of that one expression rather than duplicating it.

   The `(0, 0)` case returning the point box **is deliberate and it is the
   crate's convention**: `der_mn(0, 0)` returns `subs(u, v).to_vec()`, a vector
   whose components equal the point's coordinates. `line.rs` and `cone.rs`
   document the same choice. Note that `plane.rs` and `cylinder.rs` return the
   zero box at `(0, 0)` instead; they are the outliers, they are not your files,
   and you must not copy them on this point.

   **Say in the doc comment that the matrix-box product is sound but loose.** It
   encloses `{ R·p : R ∈ M, p ∈ u_part }`, a superset of the true set, because
   it lets the rotation and the profile point vary independently when in truth
   `R` depends only on `v` and `p` only on `u`. That decorrelation is precisely
   what makes the product an over-estimate, and over-estimation is always
   acceptable (BG-ENC-001). Tightening it is not your job. It does mean the
   enclosure is noticeably wider than the analytic `Torus` carrier's for the
   same patch; that is expected and is not a defect.

5. **`normal_cone` and `immersion_lower_bound` both go through one private
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

   Sound but loose, for the same decorrelation reason as decision 4.

   - **`immersion_lower_bound(uu, vv)`** is the smallest `‖n‖` over that box:

         sqrt(mig(n.x)^2 + mig(n.y)^2 + mig(n.z)^2)

     using `inari::Interval::mig` (the mignitude — `0.0` if the interval
     contains zero, else `min(|inf|, |sup|)`). Each coordinate attains its
     mignitude independently, so this is exactly the box's minimum norm, and
     since the box contains the true set it is a valid lower bound on the true
     minimum. **Compute it in `inari` and return `.inf()` of the result**, not
     in `f64`: returning a value one rounding unit too large is a soundness bug,
     not a tightness one. Return `0.0` for a non-finite or empty result. It goes
     to `0.0` when the cell reaches the axis, which is the answer the immersion
     margin wants.

   - **`normal_cone(uu, vv)`** turns the same box into a `DirCone`. Let `c` be
     the box's midpoint vector (`n.x.mid()`, …) and `h` its half-width vector
     (`n.x.wid() / 2.0`, …). Every element of the box lies within distance
     `rho = ‖h‖` of `c`, so if `rho < ‖c‖` every element makes an angle of at
     most `asin(rho / ‖c‖)` with `c`. Therefore:

         rho  = ‖h‖  computed in inari, take .sup()   (round UP)
         cn   = ‖c‖  computed in inari, take .inf()   (round DOWN)
         if !(cn > rho) or either is not finite  ->  None
         axis       = c.normalize()
         half_angle = asin(rho / cn), nudged UP, clamped to at most PI

     `rho >= cn` is exactly the case where the box may contain the zero vector
     or straddle too many directions for any cone to bound — including a cell
     that reaches the axis — so the `None` arm is the contract, not a
     convenience. Nudge the half-angle upward by a few ulps
     (`* (1.0 + 8.0 * f64::EPSILON) + 8.0 * f64::EPSILON` is the house form) so
     that the f64 `asin` and the `normalize()` cannot round the cone too narrow;
     `f64::EPSILON` is a named constant and does not need an H-3 comment.
     Name the `PI` clamp as a `const` with a word on what it is.

     **Expect `None` more often here than in the sibling decorators**, because
     the decorrelated product widens the derivative boxes; a `vv` spanning much
     of a full turn will not produce a bounded cone. That is sound behaviour,
     not a bug, and your tests should use `vv` cells that are a modest fraction
     of a turn when they want `Some`.

6. **A sibling packet writes a near-identical private helper in its own file.**
   BG-ENC-004-PROCESSOR and BG-ENC-004-EXTRUDED both need the same cross-product
   cone. That duplication is deliberate and known: the three packets must have
   **disjoint write sets** so they can run in parallel. Do not attempt to share
   it, do not create a shared module, and do not edit `decorators/mod.rs` to
   host it. Consolidating the three copies is a later refactor and is explicitly
   not in scope. Do not mention it in `deviations`; it is not a deviation.

7. **No changes to `enclosure.rs`, `harness.rs`, any existing carrier, or
   anything under `truck-geometry`.** If you find yourself wanting to touch the
   `EnclosureSurface` trait, that is a SPEC_GAP, not an edit.

## Interval trigonometry — read this before you write `rot_der`

`inari::Interval` has **no** `sin`/`cos` in this tree — they are behind
`inari`'s `gmp` feature and `truck-evidence` takes `inari` with
`default-features = false`. Use

    use crate::elementary::{cos, sin};

and write `cos(vv)`, never `vv.cos()`; the method does not exist and a design
that needs it is a design that stops. That module is BG-ENC-005, added in
session 11 for exactly this, and it is already outward-rounded and already
accounts for the interior extrema at `kπ/2`. **Never evaluate a trig function
only at the interval endpoints** — an interval spanning an interior extremum
(e.g. `[0.4π, 0.6π]` for `sin`) must contain the extremal value, and endpoint
evaluation is the historic under-estimation bug this whole item exists to
prevent.

## Tests required

All in the `#[cfg(test)]` module of `revolved.rs`, using the shared harness
(`crate::harness::{assert_encloses_surface, assert_converges}`) and the
`plane.rs` / `cone.rs` test style for literals (named consts; a `// H-3`
same-line opt-out where a bare float is unavoidable — note rustfmt moves
trailing comments off brace-opening lines).

Your two profile witnesses are the crate's two `EnclosureCurve` impls:
`Line<Point3>` (from `line.rs`) and `UnitCircle<Point3>` (from `circle.rs`,
which is `(cos t, sin t, 0)`). Construct with
`RevolutedCurve::by_revolution(curve, origin, axis)`. Useful configurations:

- a `Line` from `(1, 0, 0)` to `(1, 0, 1)` about the `z` axis at the origin — a
  cylinder, an immersion everywhere, the easy case;
- a `Line` from `(0, 0, 0)` to `(1, 0, 1)` about the `z` axis at the origin — a
  cone whose profile *starts on the axis*, so `u` near `0` is singular;
- the `UnitCircle` about the `y` axis at the origin — the circle crosses that
  axis at `t = ±π/2`, giving two singular parameters and a curved profile;
- at least one configuration with a **non-axis-aligned** axis (e.g.
  `(1, 1, 1).normalize()`) and a non-zero origin. An axis-aligned test passes a
  transposed `rot_der`, which is the defect most likely to be present.

1. `revolved_encloses_sampled_points` — several boxes over all four
   configurations above, `assert_encloses_surface` with at least 20 samples per
   axis. Include a `vv` crossing `π/2`, one spanning more than `π`, a full `2π`
   sweep, and a `vv` entirely negative.
2. `revolved_rotation_matrix_derivatives_match` — the transpose test. For
   several `n` in `0..=4`, several axes including a non-axis-aligned one, and
   several `v`, assert that every entry of `rot_der(n, axis, interval_at(v))`
   **contains** the corresponding entry of the surface's own behaviour at that
   `v`. You cannot call the private `from_axis_angle_derivation` directly, so
   test it through the public surface: build a `RevolutedCurve` whose profile
   makes `u_part` each of the three basis directions in turn (three `Line`s
   through the origin) and compare `der_mn(0, n, u, v)` against your matrix
   column. State in a comment which column each probe recovers. If that turns
   out not to be constructible, fall back to asserting `enclose_der(0, n, ..)`
   contains `der_mn(0, n, ..)` over a dense grid for the non-axis-aligned case
   and say so in `deviations` — but try the direct version first, because the
   dense-grid version is what test 3 already does.
3. `revolved_der_enclosures_match_partials` — for all four configurations, the
   `(0,0)`, `(1,0)`, `(0,1)`, `(2,0)`, `(1,1)` and `(0,2)` enclosures contain
   the surface's own `der_mn` sampled over a grid.
4. `revolved_normal_cone_contains_sampled_normals` — for the cylinder
   configuration and a modest `vv` cell bounded away from the axis, the returned
   cone **contains** every sampled unit normal `(S_u × S_v).normalize()` over a
   grid, tested by angle. Also assert `None` for a full `2π` sweep.
5. `revolved_immersion_lower_bound_vanishes_on_the_axis` — exactly `0.0` for a
   cell of the cone configuration containing `u = 0` (profile on the axis) and
   for a cell of the revolved circle containing `t = π/2`; strictly positive and
   a genuine *lower* bound (`<=` the sampled `‖S_u × S_v‖` at every grid point)
   for the cylinder configuration.
6. `revolved_enclosure_converges_under_bisection` — `assert_converges` on the
   cylinder configuration from a moderate box, depth ~20.

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
trait, the harness, or any existing carrier. Endpoint-only trig evaluation
anywhere. Writing `vv.cos()`. Taking a `C: BoundedCurve` bound in order to call
`is_front_fixed`. Adding `#[ignore]`. Adding `unscaled_legacy(` call sites.
Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- the impl bound in decision 2 cannot be made to compile with at most one added
  bound → `SPEC_GAP`, naming the compiler's exact requirement
- `inari` or `crate::elementary` lacks a primitive this design needs →
  `SPEC_GAP`, naming it; do not hand-roll directed rounding and do not
  hand-roll a trig function
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(evidence): EnclosureSurface for RevolutedCurve (BG-ENC-004-REVOLVED)`.
