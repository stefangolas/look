# WORK PACKET BG-ENC-002-CONE — enclosure for the `Cone` carrier

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-ENC-002-CONE","status":"DONE","contracts":["BG-ENC-001","BG-ENC-002"],
 "tests_added":6,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: if a decision below is wrong,
say so rather than working around it.

```yaml
id:          BG-ENC-002-CONE
contract:    [BG-ENC-001, BG-ENC-002]
class:       mechanical
crates:      [truck-evidence]
depends_on:  [BG-CE-006-ENUM]
write_allow:
  - vendor/truck/truck-evidence/src/cone.rs
read_allow:
  - vendor/truck/truck-evidence/src/lib.rs
  - vendor/truck/truck-evidence/src/elementary.rs
  - vendor/truck/truck-evidence/src/enclosure.rs
  - vendor/truck/truck-evidence/src/harness.rs
  - vendor/truck/truck-evidence/src/plane.rs
  - vendor/truck/truck-geometry/src/specifieds/cone.rs
  - vendor/truck/truck-geometry/src/specifieds/mod.rs
tests_required:
  - cone_encloses_sampled_points
  - cone_trig_extrema_inside_interval
  - cone_enclosure_converges_under_bisection
  - cone_normal_cone_refuses_across_the_apex
  - cone_immersion_lower_bound_vanishes_at_the_apex
  - cone_der_enclosures_match_partials
budget:      {turns: 34, ctx_tokens: 80000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'impl ParametricSurface3D for Cone' vendor/truck/truck-geometry/src/specifieds/cone.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub struct Cone' vendor/truck/truck-geometry/src/specifieds/mod.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub mod plane' vendor/truck/truck-evidence/src/lib.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'impl EnclosureSurface for Plane' vendor/truck/truck-evidence/src/plane.rs"}
  - {id: A5, expect: 4, cmd: "grep -c 'pub fn assert_' vendor/truck/truck-evidence/src/harness.rs"}
```

## Problem

`truck-evidence` has the enclosure interface (BG-ENC-001) and one reference
carrier (`Plane`, exact because affine). This packet adds the **cone** — the
first carrier that is *not* an immersion everywhere. The parameterization (read
it off `specifieds/cone.rs`, confirmed at packet time) is

    S(u, v) = apex + v·tan(α)·(cos u, sin u, 0) + (0, 0, v)

with `u ∈ [0, 2π)` periodic, `v` unbounded and signed, and `α` the half angle.
`Cone::new` refuses anything outside `0 < α < π/2`, so **`tan(α) > 0` and finite
is an invariant you may rely on**. Note `v` is signed: `v < 0` is the opposite
nappe, and the carrier is a *double* cone joined at the apex.

**The apex is the point of this packet.** At `v = 0` the whole `u` circle
collapses to the single point `apex`; `S_u = tan(α)·v·(−sin u, cos u, 0)`
vanishes identically, the cross product `S_u × S_v` is zero, and there is no
normal direction — `specifieds/cone.rs` returns the zero vector from `normal`
for exactly this reason. Every other carrier in this family is an immersion on
its whole domain and this one is not, so this is where the `Option<DirCone>`
in the trait earns its existence.

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

1. **One existing file**, `vendor/truck/truck-evidence/src/cone.rs`. It is
   already created and already declared as `pub mod cone;` in `lib.rs`, and
   `lib.rs` is **read-only for you** — it is not on your `write_allow` and
   editing it is a scope violation. The declaration was made up front so the
   six sibling carrier packets have disjoint write sets and can run in
   parallel; the file currently holds only a scaffolding doc comment, which
   you replace. Crate-level `#![deny(...)]` in `lib.rs` covers your module; do
   not add a second header. Follow `plane.rs` for structure, doc tone, and the
   `interval_at` helper (copy it or reuse it via `pub(crate)` — your call, but
   one definition is better than two).

2. **`impl EnclosureSurface for Cone`**, on the plain `Cone` type. `Cone::new`
   returns an `Outcome<Cone>`; that is the *constructor's* business and none of
   yours. You are given a `&Cone` that already exists, so its invariants
   (`0 < α < π/2`) already hold. Do not re-validate them and do not make your
   methods return `Outcome`.

3. **`enclose`**: let `s = interval_at(self.half_angle().tan())` — one
   degenerate interval computed once, because `tan` is evaluated in `f64` and
   you must not pretend that is exact. Then

       x = apex.x + s * vv * cos(uu)
       y = apex.y + s * vv * sin(uu)
       z = apex.z + vv

   all in `inari` arithmetic, which rounds outward for you. `vv` is signed and
   `inari` multiplication already handles mixed-sign intervals correctly — do
   **not** hand-roll a sign case analysis, and in particular do not assume
   `vv.inf() ≥ 0`.

4. **`enclose_der(m, n, uu, vv)`** — from the partials in `cone.rs`:
   - `(1,0)` → `s * vv * (−sin u, cos u, 0)` componentwise;
   - `(0,1)` → `s * (cos u, sin u, 0) + (0, 0, 1)`, i.e. the `z` component is
     the degenerate interval at `1.0` and is exact;
   - `(2,0)` → `s * vv * (−cos u, −sin u, 0)`;
   - `(1,1)` → `s * (−sin u, cos u, 0)`, `z` degenerate at `0.0`;
   - `(0,2)` → the zero box (`S` is affine in `v` for fixed `u`);
   - any higher `n ≥ 2` → the zero box; higher `m` continues the mod-4 trig
     cycle, and if you would rather return a sound-but-loose box for `m ≥ 3`
     than write the cycle out, that is acceptable — say so in `deviations`.

5. **`normal_cone(uu, vv)` returns `Option<DirCone>`, and returns `None`
   whenever `vv` contains `0.0`.** This is the contract, not a convenience:
   at `v = 0` there is no normal direction at all, and a cell that straddles
   `v = 0` also contains both nappes, whose normals point into opposite
   half-spaces. `vv.contains(0.0)` → `None`. No exceptions, including a `vv`
   that is exactly the degenerate interval at `0.0`.

   When `vv` does **not** contain `0.0`, the unit normal (read it off
   `cone.rs`) is

       n(u, v) = ±(cos u, sin u, −tan α) / sqrt(1 + tan²α)

   with the sign fixed by `sign(v)`, which is now constant over the cell. So
   the direction depends on `u` alone (up to that fixed sign) and the cone
   construction is the cylinder's, tilted:
   - let `w = uu.sup() − uu.inf()` (do NOT wrap; the interval is what it is);
   - `w ≤ π` → axis is the unit normal at the midpoint angle `m`, with the
     `sign(v)` applied, and `half_angle = w / 2`. **This is the one place a
     word of care is needed:** the normals over an arc of angular width `w`
     are *not* spread by `w/2` about their bisector once they are tilted out
     of the plane — the tilt shrinks the spread. `w / 2` is therefore an
     over-estimate, which is **sound** (a cone that is too wide still contains
     every normal) and is what you should return. Tightness is BG-ENC-004's
     problem, not yours. Put that sentence in the doc comment.
   - `w > π` → axis `(0, 0, −sign(v))` and `half_angle = π/2`: every normal on
     one nappe makes the constant angle `α`... `< π/2` with the `−sign(v)·z`
     axis, so a `π/2` cone around it contains all of them. Sound, loose,
     correct.
   Name the `π` threshold as a `const` with a word on what it is (H-3).

6. **`immersion_lower_bound(uu, vv)`**: `‖S_u × S_v‖ = tan(α)·|v|·sqrt(1 + tan²α)`,
   so it is minimized at the `v` in `vv` of smallest absolute value:

       let m = if vv.contains(0.0) { 0.0 } else { min(|vv.inf()|, |vv.sup()|) }

   times `tan α · sqrt(1 + tan²α)`, **rounded down** — this is a lower bound
   and returning something a rounding-unit too large is a soundness bug, not a
   tightness one. Compute it in `inari` and take `.inf()` of the result rather
   than doing it in `f64`. It is `0.0` exactly when the cell touches the apex,
   which is the answer §10's immersion margin wants.

7. **No changes to `enclosure.rs`, `harness.rs`, or `plane.rs`.** If you find
   yourself wanting to touch the trait, that is a SPEC_GAP, not an edit.

## Tests required

All in the `#[cfg(test)]` module of `cone.rs`, using the shared harness
(`crate::harness::{assert_encloses_surface, assert_converges}`) and the
`plane.rs` test style for literals (named consts; a `// H-3` same-line opt-out
if a bare float is ever unavoidable — note rustfmt moves trailing comments off
brace-opening lines). Build cones with `Cone::new(...)` and take the value out
of the `Outcome`; if that is awkward in a test, say so in `disagreements`
rather than reaching into private fields.

1. `cone_encloses_sampled_points` — several boxes, including: a small arc at
   `v > 0`; an arc crossing `π/2`; one spanning more than `π`; a full `2π`
   sweep; a box entirely at `v < 0` (the far nappe); and a box whose `vv`
   **straddles zero**, which must still enclose correctly even though its
   normal cone is `None`. `assert_encloses_surface` with ≥ 20 samples per axis.
2. `cone_trig_extrema_inside_interval` — the spec's mandated unit test: for
   `uu = [0.4π, 0.6π]` and a `vv` bounded away from zero, the `x`-interval of
   `enclose` must contain the interior extremum `cos(0.5π) = 0` scaled by the
   cell, and must be strictly wider than the endpoint-only evaluation would
   have been. State the check in terms of relations, not bit-equality.
3. `cone_enclosure_converges_under_bisection` — `assert_converges` from a
   moderate box **bounded away from the apex**, depth ~20. Then, separately,
   assert that a box *containing* the apex does not need to converge to a
   point: this is a documented property of the carrier, so assert what is
   true (the enclosure still shrinks in `u` and `v`) rather than asserting a
   width that the apex makes impossible.
4. `cone_normal_cone_refuses_across_the_apex` — `normal_cone` is `None` for
   `vv` straddling `0`, for `vv` touching `0` at an endpoint, and for the
   degenerate `vv = [0, 0]`; and is `Some` for a `vv` bounded away from zero
   on either nappe. For the `Some` cases, assert the returned cone **contains**
   the sampled unit normals over a grid, by angle.
5. `cone_immersion_lower_bound_vanishes_at_the_apex` — exactly `0.0` for every
   `vv` containing `0.0`; strictly positive and a genuine *lower* bound (`≤`
   the sampled `‖S_u × S_v‖` over a grid, for every sample) for cells away
   from it.
6. `cone_der_enclosures_match_partials` — `(1,0)`, `(0,1)`, `(2,0)` and `(1,1)`
   enclosures contain the analytically sampled partials over a grid; `(0,2)` is
   the zero box.

`DirCone` containment by angle: `cos(angle between axis and d) >= cos(half_angle)`
— implement as a small test-local helper with a comment; `half_angle = π/2`
needs the `>=` with float tolerance to survive rounding.

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
the harness, or `plane.rs`. Endpoint-only trig evaluation anywhere. Returning
`Some` from `normal_cone` for a cell that touches `v = 0`. Adding `#[ignore]`.
Adding `unscaled_legacy(` call sites. Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- `inari` lacks a trig or rounding primitive this design needs → `SPEC_GAP`,
  naming it — do not hand-roll directed rounding
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(evidence): EnclosureSurface for Cone (BG-ENC-002-CONE)`.
