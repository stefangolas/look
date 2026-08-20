# WORK PACKET BG-ENC-002-CYLINDER — enclosure for the `Cylinder` carrier

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-ENC-002-CYLINDER","status":"DONE","contracts":["BG-ENC-001","BG-ENC-002"],
 "tests_added":6,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: if a decision below is wrong,
say so rather than working around it.

```yaml
id:          BG-ENC-002-CYLINDER
contract:    [BG-ENC-001, BG-ENC-002]
class:       mechanical
crates:      [truck-evidence]
depends_on:  [BG-CE-006-ENUM]
write_allow:
  - vendor/truck/truck-evidence/src/cylinder.rs
read_allow:
  - vendor/truck/truck-evidence/src/lib.rs
  - vendor/truck/truck-evidence/src/elementary.rs
  - vendor/truck/truck-evidence/src/enclosure.rs
  - vendor/truck/truck-evidence/src/harness.rs
  - vendor/truck/truck-evidence/src/plane.rs
  - vendor/truck/truck-geometry/src/specifieds/cylinder.rs
  - vendor/truck/truck-geometry/src/specifieds/mod.rs
tests_required:
  - cylinder_encloses_sampled_points
  - cylinder_trig_extrema_inside_interval
  - cylinder_enclosure_converges_under_bisection
  - cylinder_normal_cone_over_arc_and_full_circle
  - cylinder_immersion_lower_bound_is_radius
  - cylinder_der_enclosures_match_partials
budget:      {turns: 35, ctx_tokens: 80000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'impl ParametricSurface for Cylinder' vendor/truck/truck-geometry/src/specifieds/cylinder.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub struct Cylinder' vendor/truck/truck-geometry/src/specifieds/mod.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub mod plane' vendor/truck/truck-evidence/src/lib.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'impl EnclosureSurface for Plane' vendor/truck/truck-evidence/src/plane.rs"}
  - {id: A5, expect: 4, cmd: "grep -c 'pub fn assert_' vendor/truck/truck-evidence/src/harness.rs"}
```

## Problem

`truck-evidence` has the enclosure interface (BG-ENC-001) and one reference
carrier (`Plane`, exact because affine). Every further carrier is the same shape
of work; this packet adds the **cylinder**, the carrier where the classic
interval-trigonometry bug lives. The parameterization (read it off
`specifieds/cylinder.rs`, confirmed at packet time) is

    S(u, v) = center + r·(cos u, sin u, 0) + (0, 0, v),   u ∈ [0, 2π) periodic, v unbounded

with normal `(cos u, sin u, 0)` and `Cylinder::new` refusing `r ≤ 0`, so
**`r > 0` is an invariant you may rely on**.

**Where the interval trig comes from.** `inari::Interval` has **no** `sin`/`cos`
in this tree: they live in `inari`'s own `elementary` module behind its `gmp`
feature, and `truck-evidence` takes `inari` with `default-features = false`.
Use the crate's own certified pair instead —

    use crate::elementary::{cos, sin};

free functions from `inari::Interval` to `inari::Interval`, already
outward-rounded and already accounting for the interior extrema at `kπ/2`.
Write `cos(uu)`, never `uu.cos()`; the method does not exist and a design that
needs it is a design that stops. **Never evaluate a trig function only at the
interval endpoints** — an interval spanning an interior extremum (e.g.
`[0.4π, 0.6π]` for `cos`) must contain the extremal value, and endpoint
evaluation is the historic under-estimation bug this item exists to prevent.

## Decisions already made for you

1. **One existing file**, `vendor/truck/truck-evidence/src/cylinder.rs`. It is
   already created and already declared as `pub mod cylinder;` in `lib.rs`, and
   `lib.rs` is **read-only for you** — it is not on your `write_allow` and
   editing it is a scope violation. The declaration was made up front so the
   six sibling carrier packets have disjoint write sets and can run in
   parallel; the file currently holds only a scaffolding doc comment, which
   you replace. Crate-level `#![deny(...)]` in `lib.rs` covers your module;
   do not add a second header. Follow `plane.rs` for structure, doc tone, and the
   `interval_at` helper (copy it or reuse it via `pub(crate)` — your call, but
   one definition is better than two).

2. **`enclose`**: `x = c.x + interval_at(r) * cos(uu)`, `y = c.y +
   interval_at(r) * sin(uu)`, `z = c.z + vv` — all in `inari` arithmetic,
   which rounds outward for you. Affine in `v`, so the `z` bound is exact.

3. **`enclose_der(m, n)`**: `(1,0)` → `(-r·sin u, r·cos u, 0)` componentwise
   interval-arithmetic; `(0,1)` → the constant `(0, 0, 1)`; every higher order
   → the zero box (second and higher derivatives of the parameterization
   vanish identically — same reasoning as `plane.rs`).

4. **`normal_cone`** over `uu` with angular half-width rules, all sound and
   simple:
   - let `w = uu.sup() − uu.inf()` (do NOT wrap; the interval is what it is);
   - `w ≤ π` → `{ axis: (cos m, sin m, 0)` at the midpoint angle `m`,
     `half_angle: w / 2 }` — an arc of half-width `w/2` around its bisector
     contains every unit direction on the arc;
   - `π < w` (anything longer than a semicircle, including full `2π`) →
     `{ axis: (0, 0, 1), half_angle: π / 2 }`: every cylinder normal is
     horizontal, i.e. at angle exactly `π/2` from `z`, so this cone contains
     all of them regardless of arc length. Not tight; sound. Tightness is
     BG-ENC-004's problem, not yours.
   Name the `π` threshold as a `const` with a word on what it is (H-3).

5. **`immersion_lower_bound`**: `‖S_u × S_v‖ = r` exactly, constant over the
   cell — return `self.radius()`.

6. **No changes to `enclosure.rs`, `harness.rs`, or `plane.rs`.** If you find
   yourself wanting to touch the trait, that is a SPEC_GAP, not an edit.

## Tests required

All in the `#[cfg(test)]` module of `cylinder.rs`, using the shared harness
(`crate::harness::{assert_encloses_surface, assert_converges}`) and the
plane.rs test style for literals (named consts; a `// H-3` same-line opt-out
if a bare float is ever unavoidable — note rustfmt moves trailing comments off
brace-opening lines).

1. `cylinder_encloses_sampled_points` — several boxes, including: a small arc,
   an arc crossing `π/2` (the trig-extremum direction), one spanning more than
   `π`, a full `2π` sweep, and a box with a `v` range of mixed sign. Use
   `assert_encloses_surface` with ≥ 20 samples per axis.
2. `cylinder_trig_extrema_inside_interval` — the spec's mandated unit test:
   for `uu = [0.4π, 0.6π]` on a unit cylinder at the origin, the `x`-interval
   of `enclose` must contain `cos(0.5π) = 0` *in its interior or at a bound
   strictly below* the endpoint values — concretely: assert
   `box.x.sup() >= 0.0` **and** that a naive endpoint evaluation
   (`[cos(0.6π), cos(0.4π)]`) would have under-estimated it, by asserting the
   enclosure's sup is the endpoint-evaluation's max plus the interior bump
   (`cos(0.5π) = 0 > cos(0.6π)`). State the check in terms of relations, not
   bit-equality.
3. `cylinder_enclosure_converges_under_bisection` — `assert_converges` from a
   moderate box, depth ~20.
4. `cylinder_normal_cone_over_arc_and_full_circle` — small arc: axis ≈
   midpoint direction, `half_angle ≈ w/2` (float tolerance); full `2π` sweep:
   axis `z`, `half_angle` `π/2`; assert the full-circle cone **contains** the
   sampled normals (`(cos u, sin u, 0)` for a grid of `u`) by angle.
5. `cylinder_immersion_lower_bound_is_radius` — equals `r` exactly for several
   boxes (it is constant).
6. `cylinder_der_enclosures_match_partials` — `(1,0)` and `(0,1)` enclosures
   contain the analytically sampled partials over a grid; `(2,0)` is the zero
   box.

`DirCone` containment by angle: `cos(angle between axis and d) >= cos(half_angle)`
— implement as a small test-local helper with a comment; `half_angle = π/2`
needs the `>=` with float tolerance to survive rounding.

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

Never run a bare `cargo test` — it builds 56 examples. Send cargo output to a
file and read the tail. The plane tests must keep passing unchanged.

## Forbidden

Editing any file outside `write_allow`. Changing the `EnclosureSurface` trait,
the harness, or `plane.rs`. Endpoint-only trig evaluation anywhere. Adding
`#[ignore]`. Adding `unscaled_legacy(` call sites. Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- `inari` lacks a trig or rounding primitive this design needs → `SPEC_GAP`,
  naming it — do not hand-roll directed rounding
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(evidence): EnclosureSurface for Cylinder (BG-ENC-002-CYLINDER)`.
