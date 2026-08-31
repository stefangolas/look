# WORK PACKET BG-CG-001-RECIPE — the spine trait, profile evaluation, and the C¹ refusals

> **r2 amendment (orchestrator, session 44).** The r1 worker stopped with an
> honest SPEC_GAP; both of its findings were packet defects, and both readings
> it proposed are adopted:
> 1. mod.rs may additionally RE-EXPORT the spine types
>    (`pub use recipe::{LineSpine, PolylineSpine, Spine};`) — `mod recipe;`
>    itself stays private (do NOT make it `pub mod`).
> 2. A SECOND landed test is booked for in-place amendment:
>    `recipe_evaluators_refuse_while_stub` (name kept, session-34 rule), and
>    the evaluator impl block is bounded to the concrete laws (see Design
>    decision 3) — the struct's `S, P, F` parameters stay generic; only the
>    evaluator `impl` is specialized.

You are filling in the constructive geometry recipe landed by
BG-CG-000-CONTRACT. The types already exist and are frozen; this packet fills
the evaluators that CG-000 stubbed, adds the spine trait surface, and pins the
C¹ refusal contract. The design is already made — transcribe it. Do not read
other spec files and do not redesign anything named here. If something you
need is genuinely missing, that is a SPEC_GAP (see "Stop conditions"): you stop
and report, you do not research it.

**Dispatch note (for the orchestrator's session, not the worker):** this
packet was pre-written against the landed CG-000 skeleton at merged HEAD
`f3137ae` and is dispatched only after a fresh cold-start read of
`loop/STATE.md` + the landed `constructive/` module.

```yaml
id:          BG-CG-001-RECIPE
contract:    [BG-CG-001-RECIPE]
class:       design
crates:      [truck-geometry]
depends_on:  [BG-CG-000-CONTRACT]
write_allow:
  - vendor/truck/truck-geometry/src/constructive/recipe.rs
  - vendor/truck/truck-geometry/src/constructive/profile.rs
  - vendor/truck/truck-geometry/src/constructive/sampling.rs
  - vendor/truck/truck-geometry/src/constructive/mod.rs
  - vendor/truck/truck-geometry/tests/constructive_recipe.rs
  - vendor/truck/truck-geometry/tests/constructive_contract.rs
read_allow:
  - docs/CONSTRUCTIVE_GEOMETRY_PLAN.md
  - vendor/truck/truck-geometry/src/constructive/mod.rs
  - vendor/truck/truck-geometry/src/constructive/recipe.rs
  - vendor/truck/truck-geometry/src/constructive/errors.rs
  - vendor/truck/truck-geometry/src/constructive/sampling.rs
  - vendor/truck/truck-base/src/tolerance.rs
tests_required:
  - line_spine_domain_position_and_derivative
  - polyline_spine_derivative_refuses_at_corners
  - polyline_spine_out_of_domain_refuses
  - profile_constant_evaluates_vertices_and_edges
  - profile_scale_interpolates_and_collapses_through_zero
  - profile_linear_correspondence_interpolates_vertexwise
  - profile_evaluation_refuses_nonfinite_parameters
  - recipe_profile_evaluation_matches_profile_law
  - recipe_position_refuses_until_frames_land
  - recipe_position_evaluates_profile_before_frame
  - sampling_uniform_count_resolves_inclusive_endpoints
  - sampling_custom_parameters_sorts_and_dedupes
  - sampling_tolerance_variants_still_refuse_in_cg001
budget:      {turns: 40, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub struct SpineFrameRecipe' vendor/truck/truck-geometry/src/constructive/recipe.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn resolve' vendor/truck/truck-geometry/src/constructive/sampling.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub fn try_linear_correspondence' vendor/truck/truck-geometry/src/constructive/mod.rs"}
  - {id: A4, expect: 0, cmd: "grep -c 'mod profile' vendor/truck/truck-geometry/src/constructive/mod.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'sampling_policy_resolve_refuses_while_stub' vendor/truck/truck-geometry/tests/constructive_contract.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub struct DirectTolerance' vendor/truck/truck-geometry/src/constructive/mod.rs"}
```

## The existing file you may touch (mod.rs, exactly these changes)

`constructive/mod.rs` gains exactly two things, both additive (nothing
existing moves; the frozen convention blocks and all landed types stay
byte-identical):

1. After the existing `mod sampling;` line:

```rust
mod profile;
```

2. After the existing `pub use recipe::SpineFrameRecipe;` line, the re-export
   that makes the spine surface reachable from integration tests and future
   consumers (r2 — the r1 worker's SPEC_GAP finding 1):

```rust
pub use recipe::{LineSpine, PolylineSpine, Spine};
```

`mod recipe;` itself stays PRIVATE — do not make it `pub mod`.

## What CG-000 landed (quote — do not re-derive, do not change)

- `constructive::errors::ConstructError` — variants `ZeroTangent{at}`,
  `FrameSingular{at, law}`, `SpineNotC1{at}`, `ProfileCorrespondenceMismatch`,
  `ProfileCollapse{at}`, `NonFinite{at}`, `InvalidInput`
  (`Debug, Clone, Copy, PartialEq, Error`; NOT `Eq` — f64 fields).
- `constructive::SpineFrameRecipe<S, P, F>` (`recipe.rs`) — fields
  `spine, profile_law, frame_law`; `const fn new`; three evaluator methods,
  all currently stubs returning `Err(ConstructError::InvalidInput)`:
  `position(&self, s: f64, v: f64) -> Result<Point3, ConstructError>`,
  `frame(&self, s: f64) -> Result<Frame3, ConstructError>`,
  `profile(&self, s: f64, v: f64) -> Result<Point2, ConstructError>`.
  Point/Vector types are the crate aliases from `truck_base::cgmath64::*`.
- `constructive::Profile2D` — `pub vertices: Vec<Point2>`, CCW closed polygon,
  implicit closing edge; `try_closed` validates >= 3 finite vertices.
- `constructive::ProfileLaw` — `Constant(Profile2D)`,
  `Scale { profile, scale: ScalarLaw }`, `LinearCorrespondence { start, end }`
  (positional vertex correspondence, validated equal counts).
- `constructive::ScalarLaw` — `Constant(f64)`, `Linear { start, end }`;
  `at(s) = start + (end - start) * s` (linear extrapolation outside [0, 1]).
- `constructive::DirectTolerance` — `{ position, parameter, jacobian,
  intersection }`, all defaulting to `truck_base::tolerance::TOLERANCE`.
- `constructive::SamplingPolicy` — `UniformCount { spine: usize }`,
  `CustomParameters(Vec<f64>)`, `ChordTolerance(f64)`,
  `AngularTolerance(f64)`; `resolve(&self, s0, s1) -> Result<Vec<f64>,
  ConstructError>` currently a stub.
- Landed tests in `tests/constructive_contract.rs` pin these shapes. One of
  them (`sampling_policy_resolve_refuses_while_stub`) asserts all four
  `SamplingPolicy` variants refuse — CG-001 amends THAT TEST'S BODY in place
  (see below). Its name must not change (session-34 identity rule).

## Design decision 1 — the spine trait (goes in `recipe.rs`)

```rust
/// The spine curve C(s) of a recipe: position and first derivative over a
/// bounded parameter domain. This is the CG-001 spine surface; realizations
/// that need higher derivatives book them additively later.
///
/// C¹ contract (normative, plan §3.2): a spine consumed on an interval must be
/// C¹ there. There is no global screening pass in CG-001 — the refusal fires
/// where the tangent is actually consumed (frame laws, CG-002/003) or where
/// the spine type itself declares non-C¹ (`PolylineSpine::derivative_at`
/// refuses at corners). This boundary is deliberate; do not add a scan.
pub trait Spine {
    /// The closed parameter domain `[s_min, s_max]`.
    fn domain(&self) -> (f64, f64);

    /// The spine point C(s). Total on the domain; outside it (beyond
    /// `DirectTolerance::parameter`), refuse `ConstructError::InvalidInput`.
    fn position_at(&self, s: f64) -> Result<Point3, ConstructError>;

    /// The (unnormalized) tangent C'(s). Frame laws normalize; a vanishing
    /// derivative is refused downstream as `ZeroTangent` (CG-002's business,
    /// not the spine's).
    fn derivative_at(&self, s: f64) -> Result<Vector3, ConstructError>;
}
```

### `LineSpine` (C¹ fixture and the simplest real spine)

```rust
/// A straight segment spine: C(s) = start + (end - start) * s on [0, 1].
/// C¹ trivially; `derivative_at` is the constant `end - start` (not
/// normalized). A degenerate start == end is NOT refused here — the zero
/// tangent refuses downstream (`ZeroTangent`, frame side), because the spine
/// itself is still a total, honest map.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineSpine { pub start: Point3, pub end: Point3 }
```

`domain()` is `(0.0, 1.0)`; `position_at`/`derivative_at` validate finiteness
of the input (`NonFinite { at: s }`) and the domain window
(`InvalidInput`); implement `Spine` for it.

### `PolylineSpine` (the declared non-C¹ spine — the refusal fixture)

```rust
/// A piecewise-linear spine through `vertices`: segment i covers
/// [i, i + 1], so the domain is [0, vertices.len() - 1] and the interior
/// integers 1 ..= n - 2 are CORNERS. Declared non-C¹:
/// `derivative_at` refuses `ConstructError::SpineNotC1 { at: s }` for any s
/// within `DirectTolerance::default().parameter` of a corner, and succeeds
/// mid-segment with that segment's (constant) direction. `position_at` is
/// total on the domain (piecewise-linear interpolation). This typed refusal
/// is the plan §7 C¹ gate: the fixture refuses, it never clamps or smooths.
#[derive(Debug, Clone, PartialEq)]
pub struct PolylineSpine { pub vertices: Vec<Point3> }
```

`PolylineSpine::try_new(vertices) -> Result<PolylineSpine, ConstructError>`:
at least two vertices, all finite, else `InvalidInput`. Domain edges:
`s < 0` or `s > n - 1` beyond `DirectTolerance::default().parameter` →
`InvalidInput`; `at` in the corner refusal is the QUERIED s (not the corner
parameter) — the error names where the caller asked.

## Design decision 2 — profile evaluation (`constructive/profile.rs`, new file)

The per-station profile evaluator. The profile ring parameter `v ∈ [0, 1]` is
**uniform per edge, NOT arc-length**: with k vertices (k edges), vertex j sits
at `v = j / k`; the closing edge k (v = 1) wraps to vertex 0. Between vertex j
and j+1, linear interpolation. This determinism is what makes the FAC grid
vertex `(i, j)` land on profile vertex ordinals (CG-004).

```rust
//! BG-CG-001-RECIPE — per-station profile evaluation.

use super::errors::ConstructError;
use super::{ProfileLaw, ScalarLaw};
use truck_base::cgmath64::*;

impl ProfileLaw {
    /// The profile point P(s, v): the profile law applied at spine station
    /// `s`, ring parameter `v ∈ [0, 1]`.
    pub fn evaluate(&self, s: f64, v: f64) -> Result<Point2, ConstructError> { /* body below */ }
}
```

Body, exactly:

- `!s.is_finite() || !v.is_finite()` → `Err(NonFinite { at: s })` (both
  non-finite parameter kinds report the spine parameter `s`; documented).
- `v < 0.0 || v > 1.0` (beyond `DirectTolerance::default().parameter`) →
  `Err(InvalidInput)`.
- **Constant(p)**: `ring_point(p, v)`.
- **Scale { profile, scale }**: let `c = scale.at(s)`. If `c.abs() <=
  DirectTolerance::default().parameter` → `Err(ProfileCollapse { at: s })`
  (scale through zero refuses; a NEGATIVE scale is a mirrored profile and is
  allowed). Else `ring_point(profile, v) * c` (both coordinates).
- **LinearCorrespondence { start, end }**: the interpolated profile at s is
  the vertex-wise lerp `start_i + (end_i - start_i) * s` (equal counts are
  guaranteed by the constructor); then `ring_point` on the interpolated
  polygon. Build the interpolated vertex `Vec<Point2>` per call — no caching,
  no allocation-avoidance cleverness (determinism over speed; CG-004 owns the
  fast path).

`fn ring_point(profile: &Profile2D, v: f64) -> Point2` (private helper in the
same file): with `k = profile.vertices.len()`, `x = v * k`; edge index
`e = floor(x) as usize`, clamped so `x == k` (i.e. `v == 1.0`) wraps to
edge 0; fraction `f = x - e as f64`; the point is
`vertices[e] + (vertices[(e + 1) % k] - vertices[e]) * f`. Note `v = 1.0`
lands on vertex 0 (the closing edge's end == start — the implicit closure);
`v = j/k` for `0 <= j < k` lands exactly on vertex j.

## Design decision 3 — the recipe evaluators (fill `recipe.rs`)

Fill the three landed stub bodies IN PLACE (signatures are frozen — do not
touch them). **r2 spelling adjustment (booked):** the evaluator `impl` block
is bounded to the concrete laws — the bodies must call `ProfileLaw::evaluate`
(an inherent method on an enum, impossible through a generic `P`) and, in
CG-002/003, will consume `Spine::derivative_at` through `S`. The struct's
`S, P, F` parameters stay exactly as landed; only the impl specializes:

```rust
impl<S: Spine> SpineFrameRecipe<S, ProfileLaw, FrameLaw> {
    // position / frame / profile bodies, as below
}
```

A recipe assembled with a non-canonical `P`/`F` simply has no evaluators —
that is the honest surface. Everything else about the struct (fields, docs,
`new`) is untouched.

- **`profile(s, v)`**: finite checks (`NonFinite { at: s }`), then
  `self.profile_law.evaluate(s, v)` — delegate; no duplicated semantics.
- **`frame(s)`**: STAYS a stub (`Err(InvalidInput)`), but update its doc
  comment: the stub note now reads "the frame laws land with CG-002 (analytic)
  and CG-003 (transport); CG-001 filled everything frame-adjacent (the spine
  trait, C¹ refusals) so the frame laws only swap this body."
- **`position(s, v)`**: the full composition, ordered (the order is contract,
  tested below):
  1. `!s.is_finite() || !v.is_finite()` → `NonFinite { at: s }`.
  2. `let p = self.profile(s, v)?` — profile first (collapse and
     correspondence refusals fire before any frame work).
  3. `let c = self.spine.position_at(s)?` (spine second).
  4. `let f = self.frame(s)?` (frame last — currently the stub refusal).
  5. `Ok(c + f.tangent * p.x + f.normal * p.y)` — the profile plane maps
     profile-x to the tangent and profile-y to the normal; the binormal
     carries nothing at CG-001 (a profile offset along the binormal is not a
     booked feature; do not invent one).

  Until CG-002/003 land, step 4 refuses, so `position` refuses on every valid
  input with the frame stub's `InvalidInput`. That is the booked state; the
  tests below pin the ORDER, not a positive position value, and CG-002 amends
  them in place when `FixedPlane` makes step 4 succeed.

## Design decision 4 — `SamplingPolicy::resolve` (fill `sampling.rs`)

Fill the landed stub body IN PLACE (signature frozen):

- `s0 > s1` (beyond `DirectTolerance::default().parameter`) → `InvalidInput`
  (the window must be ascending).
- **UniformCount { spine }**: `spine < 2` → `InvalidInput`; else the n
  stations `s0 + (s1 - s0) * (i as f64) / ((n - 1) as f64)` for
  `i in 0..n` — inclusive of both endpoints, computed by that exact formula
  (the test uses the same expression, so exact `assert_eq!` is meaningful).
- **CustomParameters(list)**: the caller-owned list used verbatim after
  validation and normalization: any non-finite member → `InvalidInput`; empty
  → `InvalidInput`; then sort ascending and dedupe ADJACENT duplicates by
  exact `f64` equality. The `[s0, s1]` window is deliberately IGNORED for
  this variant (caller-owned takes precedence) — document it.
- **ChordTolerance(_) | AngularTolerance(_)**: STILL refuse
  `Err(InvalidInput)` in CG-001, with the doc note updated to: "requires
  spine-aware refinement (it must consume `Spine::derivative_at`); booked as
  a follow-up packet, deliberately NOT filled here." This is a typed envelope
  line, not a missing implementation — the resolver that can see the spine
  arrives with the realization backend that owns one.

## The booked amendments to landed tests (r2: TWO, both name-preserving)

Both keep their exact landed names (session-34 identity rule); only bodies
change, each with a one-line comment naming BG-CG-001-RECIPE as the amendment.

1. `tests/constructive_contract.rs::sampling_policy_resolve_refuses_while_stub`
   — replace the body so it asserts the CG-001 reality: `UniformCount { spine: 4 }`
   and a `CustomParameters` list now RESOLVE (assert the exact sorted values), while
   `ChordTolerance`/`AngularTolerance` still refuse `InvalidInput`.

2. `tests/constructive_contract.rs::recipe_evaluators_refuse_while_stub`
   (r2 — the r1 worker's SPEC_GAP finding 2) — the landed `S = ()` spine no
   longer typechecks against the bounded evaluator impl, and a filled
   `profile` no longer refuses. Rewrite the body: build the recipe with
   `spine: LineSpine { start, end }`, `profile_law: Constant(triangle)`,
   `frame_law: FixedPlane { normal }`, and assert `profile(s, v)` now returns
   `Ok(_)` equal to `profile_law.evaluate(s, v)` (positive assertion), while
   `frame(s)` and `position(s, v)` still return `Err(ConstructError::InvalidInput)`
   (the frame stub). The name stays: position/frame DO still refuse while the
   frame is a stub.

All other landed tests stay byte-identical. No existing test may be deleted,
`#[ignore]`d, or weakened.

## New tests — `tests/constructive_recipe.rs` (new file)

Header `#![deny(clippy::unwrap_used)]` (GATE-1). Fixtures are tiny: a triangle
`(0,0) (1,0) (0,1)`, a quad `(0,0) (1,0) (1,1) (0,1)`, `LineSpine` from
`(0,0,0)` to `(1,0,0)`, a `PolylineSpine` through `(0,0,0) (1,0,0) (1,1,0)`.
Plain decimal literals are fine; the H-3 regex bans only `1e-…` forms; any
tolerance comparison goes through `DirectTolerance::default()` or
`truck_base::tolerance::TOLERANCE`, never a literal.

1. `line_spine_domain_position_and_derivative` — domain is `(0.0, 1.0)`;
   `position_at(0.25)` is the exact lerp; `derivative_at` at both 0.0 and 1.0
   is `end - start`.
2. `polyline_spine_derivative_refuses_at_corners` — the 3-vertex polyline:
   `derivative_at(1.0)` is `Err(SpineNotC1 { at: 1.0 })`; `derivative_at(0.5)`
   succeeds with the first segment's direction; `derivative_at(1.5)` succeeds
   with the second's. (This is the plan §7 C¹ gate.)
3. `polyline_spine_out_of_domain_refuses` — `position_at(-0.5)` and
   `position_at(2.5)` are `Err(InvalidInput)`; `position_at(0.0)` and
   `position_at(2.0)` succeed.
4. `profile_constant_evaluates_vertices_and_edges` — with k = 4:
   `evaluate(s, 0.0)` == vertex 0; `evaluate(s, 0.25)` == vertex 1;
   `evaluate(s, 0.5)` == vertex 2; `evaluate(s, 0.75)` == vertex 3;
   `evaluate(s, 1.0)` == vertex 0 (closure); `evaluate(s, 0.125)` is the exact
   midpoint of vertices 0–1. Same answer for every s (Constant).
5. `profile_scale_interpolates_and_collapses_through_zero` —
   `Scale { quad, scale: ScalarLaw::Linear { start: 1.0, end: 3.0 } }` at
   `s = 0.5` is the quad scaled by 2.0; `Scale { quad, scale: ScalarLaw::Linear
   { start: 1.0, end: -1.0 } }` at `s = 0.5` is
   `Err(ProfileCollapse { at: 0.5 })`; a constant negative scale
   (`ScalarLaw::Constant(-1.0)`) SUCCEEDS with mirrored coordinates.
6. `profile_linear_correspondence_interpolates_vertexwise` — triangle →
   translated triangle at `s = 0.5`: vertex i of the evaluated profile is the
   exact vertex-wise lerp; `evaluate(0.5, j/3.0)` returns lerped vertex j for
   j = 0, 1, 2.
7. `profile_evaluation_refuses_nonfinite_parameters` — `evaluate(NaN, 0.5)` is
   `Err(NonFinite { at: NaN })` (match the variant, not the payload); v out of
   `[0, 1]` is `Err(InvalidInput)`.
8. `recipe_profile_evaluation_matches_profile_law` — a recipe
   `{ spine: LineSpine, profile_law: Constant(quad), frame_law: FixedPlane }`:
   `recipe.profile(s, v) == profile_law.evaluate(s, v)` for several (s, v)
   pairs — the recipe delegates, no duplicated arithmetic.
9. `recipe_position_refuses_until_frames_land` — a valid input through
   `recipe.position(s, v)` is `Err(_)` (the frame stub's refusal propagates);
   assert only the Err, and comment that CG-002 amends this test in place.
10. `recipe_position_evaluates_profile_before_frame` — the same recipe with
    `profile_law: Scale { quad, scale: ScalarLaw::Constant(0.0) }`:
    `recipe.position(s, v)` is `Err(ProfileCollapse { .. })` — the profile
    refusal fires even though the frame stub would refuse too; that is the
    ordering contract, pinned.
11. `sampling_uniform_count_resolves_inclusive_endpoints` —
    `UniformCount { spine: 4 }.resolve(0.0, 1.0)` is exactly
    `[0.0, 1.0/3.0, 2.0/3.0, 1.0]` computed by the booked formula;
    `UniformCount { spine: 1 }` is `Err(InvalidInput)`;
    `resolve(1.0, 0.0)` is `Err(InvalidInput)`.
12. `sampling_custom_parameters_sorts_and_dedupes` —
    `CustomParameters(vec![1.0, 0.0, 0.5, 0.0]).resolve(7.0, 9.0)` is exactly
    `[0.0, 0.5, 1.0]` (window ignored, caller-owned wins).
13. `sampling_tolerance_variants_still_refuse_in_cg001` — `ChordTolerance` and
    `AngularTolerance` each `Err(InvalidInput)` (the booked envelope line).

No existing test may be deleted, `#[ignore]`d, or weakened — except the ONE
booked in-place amendment above, which is an update, not a weakening.

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `unimplemented!`, `todo!` — in code
  and tests (the modules deny `clippy::unwrap_used`; write tests with
  `matches!` / `assert!(matches!(..))` / field `assert_eq!`).
- **H-2** Fallible operations return `Result<_, ConstructError>` — the frozen
  currency.
- **H-3** No bare `1e-…` literals anywhere (GATE-2 scans test files too);
  tolerances come from `DirectTolerance::default()` or the
  `truck_base::tolerance::TOLERANCE` const.
- The crate warns `missing_docs, missing_debug_implementations` and denies
  warnings in release: doc-comment every public item, derive `Debug` on every
  public type.
- No `unscaled_legacy(` calls (GATE-4); no `debug_new`, no
  `cfg!(debug_assertions)` (GATE-3).
- GATE-1: every new kernel `.rs` file carries `#![deny(clippy::unwrap_used)]`
  (`profile.rs` and the new test file are new files; `recipe.rs` and
  `sampling.rs` already carry it).

## Done when — run these, all must pass

```
cargo fmt --check -p truck-geometry
cargo clippy -p truck-geometry --all-targets -- -D warnings
cargo test -p truck-geometry --lib --tests
```

Never run a bare `cargo test`. The module is still purely additive over the
rest of the workspace; the verifier runs the workspace gates authoritatively.
Send cargo output to a file and read the tail.

## Forbidden

Editing any file outside `write_allow` — in particular
`constructive/errors.rs` (the error set is frozen; no new variants),
`constructive/mod.rs` beyond the two permitted additions (the `mod profile;`
line and the spine re-export line — `mod recipe;` stays private),
the crate `prelude`, `Cargo.toml`, `Cargo.lock`, `scripts/kernel-gates.sh`.
Changing any frozen signature (the three recipe evaluator signatures,
`SamplingPolicy::resolve`'s signature, `ConstructError`'s variants) or making
the struct's `S, P, F` parameters concrete (only the evaluator impl block
specializes). Adding a frame-law implementation or a spine-aware sampling
resolver (CG-002/003 and a booked follow-up own those — do not blur the write
sets). Adding `#[ignore]`. Adding `#[allow]` without a same-line
justification. Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH` (A4 must read 0 — it proves
  `profile.rs` is not declared yet; A5 must read 1 — the amendable test exists)
- the contract as written cannot compile as specified → `SPEC_GAP`, naming
  the exact conflict (do not silently adjust a frozen signature)
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

```json
{"id":"BG-CG-001-RECIPE","status":"DONE","contracts":["BG-CG-001-RECIPE"],
 "tests_added":13,"anchors_verified":{"A1":1,"A2":1,"A3":1,"A4":0,"A5":1,"A6":1},
 "notes":"any deviation from the quoted design, with the reason"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it: what you attempted, the
exact ambiguity, and the readings you could not choose between.

Commit on the current branch with subject
`feat(geometry): spine trait, profile evaluation, C1 refusals (BG-CG-001-RECIPE)`
BEFORE writing `RESULT.json`.
