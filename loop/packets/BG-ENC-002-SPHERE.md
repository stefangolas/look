# WORK PACKET BG-ENC-002-SPHERE — enclosure for the `Sphere` carrier

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-ENC-002-SPHERE","status":"DONE","contracts":["BG-ENC-001","BG-ENC-002"],
 "tests_added":6,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

```yaml
id:          BG-ENC-002-SPHERE
contract:    [BG-ENC-001, BG-ENC-002]
class:       mechanical
crates:      [truck-evidence]
depends_on:  [BG-CE-006-ENUM]
write_allow:
  - vendor/truck/truck-evidence/src/sphere.rs
read_allow:
  - vendor/truck/truck-evidence/src/lib.rs
  - vendor/truck/truck-evidence/src/elementary.rs
  - vendor/truck/truck-evidence/src/enclosure.rs
  - vendor/truck/truck-evidence/src/harness.rs
  - vendor/truck/truck-evidence/src/plane.rs
  - vendor/truck/truck-geometry/src/specifieds/sphere.rs
  - vendor/truck/truck-geometry/src/specifieds/mod.rs
tests_required:
  - sphere_encloses_sampled_points
  - sphere_trig_extrema_inside_interval
  - sphere_enclosure_converges_under_bisection
  - sphere_normal_cone_over_patch
  - sphere_immersion_lower_bound_and_poles
  - sphere_der_enclosures_match_partials
budget:      {turns: 35, ctx_tokens: 80000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'impl ParametricSurface for Sphere' vendor/truck/truck-geometry/src/specifieds/sphere.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub struct Sphere' vendor/truck/truck-geometry/src/specifieds/mod.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub mod plane' vendor/truck/truck-evidence/src/lib.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'impl EnclosureSurface for Plane' vendor/truck/truck-evidence/src/plane.rs"}
  - {id: A5, expect: 4, cmd: "grep -c 'pub fn assert_' vendor/truck/truck-evidence/src/harness.rs"}
```

## Problem

Add the sphere enclosure, the carrier with **products of two interval trig
functions** and a parameterization that degenerates at the poles. The
parameterization (read it off `specifieds/sphere.rs`, confirmed at packet time)
is, with `u` the **polar** angle from `+z` and `v` the **azimuth**:

    S(u, v) = center + r · (sin u·cos v,  sin u·sin v,  cos u),   normal = the same unit vector

`Sphere::new` does not validate the radius (unlike `Cylinder`); a sphere with
`r ≤ 0` or non-finite `r` has no sound enclosure — see decision 6.

**Where the interval trig comes from.** `inari::Interval` has **no** `sin`/`cos`
in this tree: they live in `inari`'s own `elementary` module behind its `gmp`
feature, and `truck-evidence` takes `inari` with `default-features = false`.
Use the crate's own certified pair instead —

    use crate::elementary::{cos, sin};

free functions from `inari::Interval` to `inari::Interval`, already
outward-rounded and already accounting for the interior extrema at `kπ/2`.
Write `cos(uu)`, never `uu.cos()`; the method does not exist and a design that
needs it is a design that stops. **Never evaluate a trig function only at the
interval endpoints.** Interval products (`[a,b]·[c,d]`) are also
outward-rounded — soundness composes.

## Decisions already made for you

1. **One existing file**, `vendor/truck/truck-evidence/src/sphere.rs`. It is
   already created and already declared as `pub mod sphere;` in `lib.rs`, and
   `lib.rs` is **read-only for you** — it is not on your `write_allow` and
   editing it is a scope violation. The declaration was made up front so the
   six sibling carrier packets have disjoint write sets and can run in
   parallel; the file currently holds only a scaffolding doc comment, which
   you replace. Crate-level `#![deny(...)]` in `lib.rs` covers your module;
   do not add a second header. Follow `plane.rs` for structure and tone.

2. **`enclose`**: `x = c.x + interval_at(r) * sin(uu) * cos(vv)` and cyclically
   (`y`: `sin u · sin v`, `z`: `cos u`). Pure inari arithmetic throughout.

3. **`enclose_der(m, n)`**: compute the analytic partials (read them off
   `sphere.rs` — first partials are `r` times products/powers of `sin`/`cos`,
   second partials likewise) and evaluate each component in interval
   arithmetic over `(uu, vv)`. Do not differentiate numerically. The trait
   takes `(m, n)` with `m` the `u`-order and `n` the `v`-order; orders above
   `(2, 2)` need not be special-cased beyond evaluating the analytic
   expression — if the trait is asked for an order whose closed form you have
   not written, return the whole-space box for that component (`Interval:: hull`
   of `±f64::INFINITY` if `inari` supports it, else the largest finite
   interval) **and say so in a comment**; an over-wide derivative enclosure is
   sound, a wrong one is not.

4. **`normal_cone`** — the spherical-patch cone, by the corner rule:
   - the four corner directions `n(u_i, v_j)` at `u_i ∈ {uu.inf, uu.sup}`,
     `v_j ∈ {vv.inf, vv.sup}`;
   - `axis = normalize(sum of the four)`; if the sum is (near-)zero, fall back
     to decision 5's everything-cone;
   - `half_angle = max angle from axis over the four corners`, computed by
     `acos(clamp(dot))`;
   - **if `half_angle < π/2`**, emit it: a cone of half-angle `< π/2` is
     geodesically convex on the sphere, and the patch is the geodesic hull of
     its corners whenever the corners lie in such a cone, so the cone contains
     the whole patch — sound;
   - **if `half_angle ≥ π/2`**, emit `{ axis: (0,0,1), half_angle: π }` —
     contains every direction; sound, not tight. A comment must say why
     (corner-set convexity argument no longer applies to a wide patch).
   Name the threshold `const` (H-3).

5. **`immersion_lower_bound`**: `‖S_u × S_v‖ = r² · sin u`, so return
   `r² · sin(uu).inf().max(0.0)` clamped at 0 — `sin(uu)` can interval-contain
   0 when `uu` reaches a pole (`u = 0` or `π`), and **the honest answer there
   is exactly 0**: the parameterization is singular at the poles. Do not
   return a negative number; clamp.

6. **Degenerate radius**: every method must be total (H-1) — for `r ≤ 0` or
   non-finite `r`, `normal_cone` and `immersion_lower_bound` may return the
   trivial answers (`π`-cone / `0.0`) and `enclose`/`enclose_der` the hull of
   what the arithmetic produces, provided nothing panics. (Whether
   `Sphere::new` should refuse like `Cylinder::new` is a different item, not
   yours.)

7. **No changes to `enclosure.rs`, `harness.rs`, `plane.rs`, or any other
   carrier file.**

## Tests required

1. `sphere_encloses_sampled_points` — `assert_encloses_surface` over several
   boxes: a small patch off the poles, a patch straddling `u = π/2` (interior
   trig extrema), a full-azimuth thin band, a box containing a pole in its
   `u`-range, and a near-hemisphere.
2. `sphere_trig_extrema_inside_interval` — for `uu` straddling `π/2` on a unit
   sphere at the origin, the `z`-interval of `enclose` must contain
   `cos(π/2) = 0` even though `cos` at both endpoints has the same sign;
   assert the relation (contains 0) rather than bit-equality.
3. `sphere_enclosure_converges_under_bisection` — `assert_converges` on a
   pole-free box, depth ~20. (A pole-containing box does not converge to zero
   in `u`-width for `x`/`y` — do not assert convergence there.)
4. `sphere_normal_cone_over_patch` — small patch: axis ≈ corner-average
   direction, half_angle small, and every normal sampled on a grid over the
   box is inside the cone by angle; wide patch: the everything-cone comes back
   and still contains every sampled normal.
5. `sphere_immersion_lower_bound_and_poles` — pole-free box: bound equals
   `r²·sin(u_min)` (float tolerance); box whose `uu` touches `u = 0`: bound is
   exactly `0.0`.
6. `sphere_der_enclosures_match_partials` — `(1,0)`, `(0,1)`, `(2,0)` contain
   the analytic partials sampled on a grid.

## H-3, which is what rejected the two carrier packets before yours

GATE-2 fails any **added** line carrying a bare `1e-N` literal unless that same
line ends with an `// H-3` comment. It is a text gate on the diff: it does not
know your literal is an angle, and it does not care that the line is in a test.
`BG-ENC-002-LINE` was rejected for one such line and `BG-ENC-002-CIRCLE` for
six, both times on assertion epsilons in tests, both times costing a verify.

So: **every comparison epsilon you write gets a same-line `// H-3:` comment
naming the dimensionless quantity being compared.** The house form, from
`truck-base/src/evidence.rs`:

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

Never run a bare `cargo test`. Send cargo output to a file and read the tail.
The plane tests must keep passing unchanged.

## Forbidden

Editing any file outside `write_allow`. Changing the trait, the harness, or
another carrier's file. Endpoint-only trig evaluation. Adding `#[ignore]`.
Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- `inari` lacks a primitive this design needs (interval product with directed
  rounding, `sin`, `cos`) → `SPEC_GAP`, naming it
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(evidence): EnclosureSurface for Sphere (BG-ENC-002-SPHERE)`.
