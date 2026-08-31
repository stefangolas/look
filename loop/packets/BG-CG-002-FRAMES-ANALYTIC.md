# WORK PACKET BG-CG-002-FRAMES-ANALYTIC — the three analytic frame laws

> **r2 amendment (orchestrator, session 44).** The r1 worker stopped with an
> honest SPEC_GAP; all four findings are adopted:
> 1. The dispatcher's calls are `super::`-qualified (Rust 2021 does not
>    resolve sibling modules bare — E0433): `super::frame_fixed::fixed_plane(..)`
>    etc. No import lines are needed or permitted.
> 2. `radial_about_axis` takes the unit tangent as an argument — its body
>    forms `b = t × n`, so the dispatcher passes the `t` it already computed.
> 3. The circle fixture is in the XY plane about the Z axis
>    (`C(s) = (cos θ, sin θ, 0)`), NOT XZ — the r1 worker correctly showed
>    the XZ fixture makes the radial̂ constant and `t · n ≠ 0`.
> 4. A SECOND landed test is booked for in-place amendment:
>    `constructive_contract.rs::recipe_evaluators_refuse_while_stub` (name
>    kept; already once amended by CG-001) goes fully positive once the
>    frames land — `tests/constructive_contract.rs` joins write_allow.
>
> **r3 amendment (orchestrator, session 44 — packet self-contradiction, zero
> worker fault).** `tests_required` listed a name
> (`recipe_position_succeeds_with_landed_frames`) that the packet's own
> amendment rule (same name, in place) forbids creating; V6 correctly
> reported TEST_MISSING on the landed worker commit. The list is corrected
> to the landed name `recipe_position_refuses_until_frames_land` (and the
> landed `recipe_position_evaluates_profile_before_frame` added for
> completeness). The worker's commit stands unchanged; re-verify only.

You are landing the analytic frame laws of the constructive geometry program
(plan §4, CG-002): `FixedPlane`, `ArchitecturalUp`, `RadialAboutAxis`. The
recipe's `frame()` stub swaps its body for a dispatcher; `ParallelTransport`
stays refused (CG-003's packet, never split). The design is already made —
transcribe it. Do not read other spec files and do not redesign anything
named here. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```yaml
id:          BG-CG-002-FRAMES-ANALYTIC
contract:    [BG-CG-002-FRAMES-ANALYTIC]
class:       mechanical
crates:      [truck-geometry]
depends_on:  [BG-CG-001-RECIPE]
write_allow:
  - vendor/truck/truck-geometry/src/constructive/frame_fixed.rs
  - vendor/truck/truck-geometry/src/constructive/frame_up.rs
  - vendor/truck/truck-geometry/src/constructive/frame_radial.rs
  - vendor/truck/truck-geometry/src/constructive/recipe.rs
  - vendor/truck/truck-geometry/src/constructive/mod.rs
  - vendor/truck/truck-geometry/tests/constructive_frames.rs
  - vendor/truck/truck-geometry/tests/constructive_recipe.rs
  - vendor/truck/truck-geometry/tests/constructive_contract.rs
read_allow:
  - docs/CONSTRUCTIVE_GEOMETRY_PLAN.md
  - vendor/truck/truck-geometry/src/constructive/mod.rs
  - vendor/truck/truck-geometry/src/constructive/recipe.rs
  - vendor/truck/truck-geometry/src/constructive/errors.rs
  - vendor/truck/truck-geometry/src/constructive/profile.rs
  - vendor/truck/truck-base/src/tolerance.rs
tests_required:
  - fixed_plane_frame_matches_spec_formula
  - fixed_plane_refuses_zero_tangent
  - fixed_plane_refuses_degenerate_normal
  - architectural_up_matches_spec_formula
  - architectural_up_refuses_parallel_up
  - radial_frame_matches_spec_formula
  - radial_frame_refuses_axis_incident_point
  - parallel_transport_still_refuses_in_cg002
  - recipe_position_refuses_until_frames_land
  - recipe_position_evaluates_profile_before_frame
  - radial_frame_is_equivariant_under_rotation
budget:      {turns: 40, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub fn frame' vendor/truck/truck-geometry/src/constructive/recipe.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'STUB (BG-CG-000-CONTRACT): the frame laws land with CG-002' vendor/truck/truck-geometry/src/constructive/recipe.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'mod profile' vendor/truck/truck-geometry/src/constructive/mod.rs"}
  - {id: A4, expect: 0, cmd: "grep -c 'frame_fixed' vendor/truck/truck-geometry/src/constructive/mod.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'pub trait Spine' vendor/truck/truck-geometry/src/constructive/recipe.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub enum FrameLaw' vendor/truck/truck-geometry/src/constructive/mod.rs"}
  - {id: A7, expect: 1, cmd: "grep -c 'recipe_position_refuses_until_frames_land' vendor/truck/truck-geometry/tests/constructive_recipe.rs"}
```

## The existing files you may touch (exactly these changes)

**`constructive/mod.rs`** gains exactly three declaration lines next to the
existing private `mod profile;`:

```rust
mod frame_fixed;
mod frame_up;
mod frame_radial;
```

Nothing else in mod.rs moves.

**`constructive/recipe.rs`** changes in EXACTLY two places, nothing else:
1. The `frame()` body: the stub refusal is replaced by the dispatcher below.
2. The `frame()` doc comment: the STUB note is replaced by the landed note
   ("Filled (BG-CG-002-FRAMES-ANALYTIC): dispatches on the frame law;
   `ParallelTransport` refuses until CG-003 lands").
Every other line of recipe.rs (the `Spine` trait, `LineSpine`,
`PolylineSpine`, `position`, `profile`, `new`, all docs) stays
byte-identical.

**`tests/constructive_recipe.rs`** changes in EXACTLY one place: the body of
the landed test `recipe_position_refuses_until_frames_land` is amended IN
PLACE (name kept — session-34 identity rule) to the positive form test 9
below requires, with a one-line comment naming BG-CG-002-FRAMES-ANALYTIC.
Every other landed test stays byte-identical. (The other landed test
`recipe_position_evaluates_profile_before_frame` keeps passing unchanged —
profile refusals fire before the frame step, which is still true with real
frames.)

**`tests/constructive_contract.rs`** changes in EXACTLY one place (r2): the
landed test `recipe_evaluators_refuse_while_stub` (already once amended by
CG-001) is amended IN PLACE again — name kept (session-34 identity rule;
the name is now fully historical) — to the positive form test 9's sibling
below requires: profile Ok, frame Ok (orthonormal, right-handed), position
Ok matching the hand-derived composition, with a one-line comment naming
BG-CG-002-FRAMES-ANALYTIC. Every other landed test in that file stays
byte-identical.

## The dispatcher (recipe.rs `frame()` body, exact)

```rust
pub fn frame(&self, s: f64) -> Result<Frame3, ConstructError> {
    let d = self.spine.derivative_at(s)?;
    if !s.is_finite() {
        return Err(ConstructError::NonFinite { at: s });
    }
    let mag = d.magnitude();
    if mag <= DirectTolerance::default().position {
        return Err(ConstructError::ZeroTangent { at: s });
    }
    let t = d / mag;
    match self.frame_law {
        FrameLaw::FixedPlane { normal } => super::frame_fixed::fixed_plane(normal, t, s),
        FrameLaw::ArchitecturalUp { up } => super::frame_up::architectural_up(up, t, s),
        FrameLaw::RadialAboutAxis { origin, axis } => {
            let c = self.spine.position_at(s)?;
            super::frame_radial::radial_about_axis(origin, axis, c, t, s)
        }
        FrameLaw::ParallelTransport { .. } => Err(ConstructError::InvalidInput),
    }
}
```

(r2: the calls are `super::`-qualified and `radial_about_axis` receives the
unit tangent `t` — the r1 worker's E0433/E0425 findings.)

(`FrameLaw` is already imported in recipe.rs. The per-law functions are
`pub(super)` — the module docs of each new file say they are reachable only
through the dispatcher.)

Normative reading of the dispatcher, pinned by the tests: the ZERO-TANGENT
refusal (`‖C′‖ ≤ DirectTolerance::default().position`) fires for ALL laws
before any law-specific work; the tangent handed to every law is UNIT
length; `RadialAboutAxis` additionally consumes `C(s)` (the spine point) —
derivative and position are each evaluated exactly once, in this order.
`ParallelTransport` refuses `ConstructError::InvalidInput` (the CG-003
envelope line), NOT `FrameSingular` — it is unimplemented, not singular.

## The three laws (each in its own new file, formulas exact)

All three refuse with `ConstructError::FrameSingular { at: s, law: <the
exact string from `FrameLaw::law_name`> }` for law-specific degeneracies —
a zero/degenerate law INPUT (e.g. a zero plane normal, a zero `up`, a zero
axis) is a singularity of the law at `s`, not `InvalidInput`: these are
plain enum payloads with no validating constructor, so evaluation is where
the refusal lives. Non-finite inputs likewise refuse `FrameSingular` at the
queried `s`.

### `frame_fixed.rs` — `FixedPlane` (plan §3.2, normative)

```rust
pub(super) fn fixed_plane(normal: Vector3, tangent: Vector3, at: f64) -> Result<Frame3, ConstructError>
```

`t = tangent` (already unit, from the dispatcher); `b = normal`; `n = b × t`.
Refuse (FrameSingular) when `normal` is non-finite or its magnitude is ≤
`DirectTolerance::default().position` (the zero plane normal). Otherwise
normalize `b` and return `Frame3 { tangent: t, normal: n, binormal: b̂ }`.
This is the preferred law for planar spines; a planar-spine fixture's frames
are constant — test 1 pins the formula and the right-handedness
(`t × n == b`).

### `frame_up.rs` — `ArchitecturalUp` (plan §3.2, normative)

```rust
pub(super) fn architectural_up(up: Vector3, tangent: Vector3, at: f64) -> Result<Frame3, ConstructError>
```

`b = normalize(up × t)`, `n = t × b`. Refuse (FrameSingular) when `up` is
non-finite, zero, or `up ∥ t` — the singularity test is
`(up × t).magnitude() <= DirectTolerance::default().position`. NO silent
frame rotation, no fallback policy (the plan books an explicit fallback
policy as a possible later extension; it does not exist in this packet).

### `frame_radial.rs` — `RadialAboutAxis` (plan §3.2, normative)

```rust
pub(super) fn radial_about_axis(
    origin: Point3,
    axis: Vector3,
    spine_point: Point3,
    tangent: Vector3,
    at: f64,
) -> Result<Frame3, ConstructError>
```

Analytic from the axis: let `â = normalize(axis)`; let
`d = spine_point − origin`; let `radial = d − (d · â)·â` (the component of
`d` perpendicular to the axis). Refuse (FrameSingular) when `axis` is
non-finite or zero, when `d` is zero/non-finite, or when
`radial.magnitude() <= DirectTolerance::default().position` (the spine point
lies ON the axis — no radial direction exists). Otherwise `n = radial̂`
(profile-y points radially outward), `b = tangent × n` (t is the unit spine
tangent handed in by the dispatcher), returned as
`Frame3 { tangent, normal: n, binormal: b }`. Rotated copies MUST remain
equivariant under a rotation about the axis, modulo floating-point (test 10;
the recorded cgmath rotation residue is ~6.1e-17 — assert within a
TOLERANCE-scaled bound, never exact equality).

All three files start with `#![deny(clippy::unwrap_used)]` (GATE-1) and
import through `use super::{ConstructError, Frame3}; use
truck_base::cgmath64::*;` (extend as the bodies require; drop what clippy
calls unused under `-D warnings`).

## Tests required — `tests/constructive_frames.rs` (new file)

Header `#![deny(clippy::unwrap_used)]`. The circle-spine fixture (tests 6,
7, 10) is a test-local `Spine` impl — the trait is public and this is the
sanctioned extension point; implement a unit-circle arc in the **XY plane
about the Z axis** (`C(s) = (cos θ, sin θ, 0)` with `θ = φ0 + s·Δ`, `s ∈
[0, 1]` — r2: the r1 worker proved the XZ plane wrong: about the Z axis the
radial̂ would be constant (1,0,0) and `t · n ≠ 0`), whose `derivative_at`
returns the analytic tangent `Δ·(−sin θ, cos θ, 0)`. Every
fixture premise is machine-checked before the assertion that depends on it.
Plain decimal literals are fine; the H-3 regex bans only `1e-…` forms; every
tolerance comparison goes through `DirectTolerance::default()` or
`truck_base::tolerance::TOLERANCE`.

1. `fixed_plane_frame_matches_spec_formula` — a LineSpine + FixedPlane with
   a non-axis-aligned plane normal: at several s, `frame(s)` is `Ok`, `t`
   equals the normalized segment direction, `b` equals the normalized
   normal, `n` equals `b × t`, and `t × n` equals `b` within
   `DirectTolerance::default().position`.
2. `fixed_plane_refuses_zero_tangent` — a degenerate LineSpine (start ==
   end): `frame(s)` is `Err(ZeroTangent { at: s })` for every s.
3. `fixed_plane_refuses_degenerate_normal` — zero normal:
   `Err(FrameSingular { law: "FixedPlane", .. })`.
4. `architectural_up_matches_spec_formula` — up = world +Z, a diagonal
   spine: `b = normalize(up × t)`, `n = t × b` within the position bound.
5. `architectural_up_refuses_parallel_up` — spine along +Z with up = +Z:
   `Err(FrameSingular { law: "ArchitecturalUp", .. })`.
6. `radial_frame_matches_spec_formula` — the circle-spine fixture about the
   Z axis through the origin: at several s, `n` is the outward radial
   direction `(cos θ, sin θ, 0)`, `t` is the analytic unit tangent
   `(−sin θ, cos θ, 0)`, `b = t × n` — all within the position bound.
7. `radial_frame_refuses_axis_incident_point` — a spine whose C(s) sits ON
   the axis: `Err(FrameSingular { law: "RadialAboutAxis", .. })`.
8. `parallel_transport_still_refuses_in_cg002` — a recipe carrying
   `ParallelTransport { initial_normal }`: `frame(s)` is
   `Err(ConstructError::InvalidInput)` (the CG-003 envelope line).
9. `recipe_position_refuses_until_frames_land` — LineSpine from
   `(0,0,0)` to `(2,0,0)`, `Constant` triangle profile, `FixedPlane` with
   normal +Z: `position(s, v)` returns the hand-derived
   `C(s) + t·p.x + n·p.y` for several (s, v) — the composed evaluator is
   alive end to end. (This is the amended body of the landed test of this
   exact name — same name per the session-34 identity rule, positive
   assertions, amendment comment. r3: the tests_required list names the
   landed name; the old positive-form name was a packet self-contradiction.)
10. `radial_frame_is_equivariant_under_rotation` — the circle-spine fixture
    rotated 90° about the axis (rotate the spine points, keep the same axis)
    yields frames whose tangent/normal/binormal equal the unrotated frames'
    vectors rotated by the same rotation, within `64.0 * TOLERANCE` (the
    recorded cgmath rotation residue is ~6.1e-17; the bound absorbs it).

No existing test may be deleted, `#[ignore]`d, or weakened — except the ONE
booked in-place amendment above (name-preserving).

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!` — in
  code and tests.
- **H-3** No `1e-…` literals anywhere (GATE-2 scans test files); tolerances
  come from `DirectTolerance::default()` or
  `truck_base::tolerance::TOLERANCE`.
- The crate warns `missing_docs, missing_debug_implementations` and denies
  warnings in release: doc-comment every public item; the three per-law
  functions are `pub(super)` and still need doc comments (missing_docs
  applies to non-public items only if exported — match whatever clippy
  demands under `-D warnings`).
- No `unscaled_legacy(` calls (GATE-4); no `debug_new`, no
  `cfg!(debug_assertions)` (GATE-3).
- The frame laws must NOT consume the profile law or the profile evaluation —
  frames depend on the spine only (separation of concerns; CG-004 composes).

## Done when — run these, all must pass

```
cargo fmt --check -p truck-geometry
cargo clippy -p truck-geometry --all-targets -- -D warnings
cargo test -p truck-geometry --lib --tests
```

Never run a bare `cargo test`. Send cargo output to a file and read the tail.

## Forbidden

Editing any file outside `write_allow` — especially
`constructive/errors.rs` (no new variants; the refusal mapping above is
frozen), `constructive/profile.rs`, `constructive/sampling.rs`, the landed
`Spine` trait and spine types, the crate `prelude`, `Cargo.toml`,
`Cargo.lock`, `scripts/kernel-gates.sh`. Implementing `ParallelTransport`
(CG-003's packet, never split — the plan books it separately). Adding any
fallback policy to `ArchitecturalUp` (not booked). Adding `#[ignore]`.
Adding `#[allow]` without a same-line justification. Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH` (A4 must read 0 — it proves
  the frame modules do not exist yet; A2 must read 1 — the stub doc you are
  replacing exists)
- the design as written cannot compile as specified → `SPEC_GAP`, naming the
  exact conflict
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

```json
{"id":"BG-CG-002-FRAMES-ANALYTIC","status":"DONE","contracts":["BG-CG-002-FRAMES-ANALYTIC"],
 "tests_added":10,"anchors_verified":{"A1":1,"A2":1,"A3":1,"A4":0,"A5":1,"A6":1,"A7":1},
 "notes":"any deviation from the quoted design, with the reason"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it: what you attempted, the
exact ambiguity, and the readings you could not choose between.

Commit on the current branch with subject
`feat(geometry): analytic frame laws (BG-CG-002-FRAMES-ANALYTIC)`
BEFORE writing `RESULT.json`.
