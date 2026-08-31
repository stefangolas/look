# QUESTION.md — BG-CG-001-RECIPE reports SPEC_GAP

Status: SPEC_GAP (stop condition 2: "the contract as written cannot compile as
specified").

## What I attempted

1. Verified all six anchors at dispatch time — A1=1, A2=1, A3=1, A4=0, A5=1,
   A6=1 — all matched the packet's expectations (no ANCHOR_MISMATCH).
2. Read the landed CG-000 skeleton in full: `constructive/mod.rs`,
   `constructive/recipe.rs`, `constructive/errors.rs`, `constructive/sampling.rs`,
   `tests/constructive_contract.rs`, `truck-base/src/tolerance.rs`.
3. Confirmed empirically that an integration test cannot reach the spine
   types: a probe test using `use truck_geometry::constructive::recipe::
   SpineFrameRecipe;` fails with `error[E0603]: module `recipe` is private`
   (module `recipe` is declared at `constructive/mod.rs:54` as private
   `mod recipe;`, and the only public surface is the three `pub use` lines
   315–317).
4. Derived, from the packet's own quoted bodies, that the landed contract test
   `recipe_evaluators_refuse_while_stub` cannot pass after CG-001 fills the
   evaluators.
5. The probe was removed; no packet files were edited.

## The exact conflicts (the packet as written cannot compile as specified)

### Conflict 1 — the new tests cannot name the spine types

- Design decision 1 places the `Spine` trait, `LineSpine`, and `PolylineSpine`
  in `recipe.rs`, which is declared as a private `mod recipe;` at
  `constructive/mod.rs:54`.
- The packet's new test file `tests/constructive_recipe.rs` must construct and
  call these types directly — tests 1 (`line_spine_domain_position_and_derivative`),
  2 (`polyline_spine_derivative_refuses_at_corners`),
  3 (`polyline_spine_out_of_domain_refuses`), and tests 8–10 (recipes with
  `spine: LineSpine` / `spine: PolylineSpine`) all require naming the types and
  the trait.
- The only mechanisms that expose them to an integration test are a
  re-export (`pub use recipe::{Spine, LineSpine, PolylineSpine};`) or
  `pub mod recipe;` in `constructive/mod.rs`. Both are explicitly forbidden:
  "Nothing else in mod.rs moves" and "constructive/mod.rs beyond the single
  `mod profile;` line".
- Result: the mandated test file cannot compile. Empirically confirmed via
  E0603.

### Conflict 2 — the landed test `recipe_evaluators_refuse_while_stub` cannot pass

The packet requires every landed test to stay byte-identical and pass ("All
other landed tests stay byte-identical"; "No existing test may be deleted,
`#[ignore]`d, or weakened — except the ONE booked in-place amendment
[`sampling_policy_resolve_refuses_while_stub`]"). The landed test
(`tests/constructive_contract.rs:152`) builds
`SpineFrameRecipe::new((), ProfileLaw::Constant(triangle), FrameLaw::FixedPlane{..})`
with spine `S = ()` and asserts all three evaluators return
`Err(ConstructError::InvalidInput)`.

After the CG-001 bodies the packet quotes are filled:

- `recipe.profile(0.5, 0.25)` no longer refuses: `ProfileLaw::Constant`
  evaluates to `Ok` (ring point on the triangle, e.g. (0.75, 0.0)), so the
  third assertion (`matches!(.. Err(InvalidInput))`) fails.
- `recipe.position`/`recipe.profile` cannot even exist for `S = ()`: the
  quoted bodies delegate to `self.spine.position_at(s)` (trait method) and
  `self.profile_law.evaluate(s, v)` (inherent method on the `ProfileLaw`
  enum). A generic `impl<S, P, F>` cannot call the inherent `evaluate` on a
  type parameter `P`, and `ProfileLaw` is an enum so it cannot be used as a
  trait bound; the bodies therefore force the impl to be specialized/bounded
  to `S: Spine` (and `P = ProfileLaw`). With `S = ()` the methods are then
  either absent (compile error at the call site) or — if the impl stays
  unbounded for `profile` — the assertion above fails.

Either flavor breaks the byte-identical, must-pass landed test.

## The readings I could not choose between

1. **The spine types were meant to be public.** The packet intends `Spine`,
   `LineSpine`, `PolylineSpine` to be reachable from `tests/` and the
   "exactly one declaration line / nothing else in mod.rs moves" rule was
   written against a mental model in which `recipe` was already public. Under
   this reading the fix is a two-line mod.rs change: add `mod profile;` plus
   `pub use recipe::{Spine, LineSpine, PolylineSpine};` (or change
   `mod recipe;` to `pub mod recipe;`). But that directly violates the
   explicit Forbidden rule, so I cannot apply it silently.
2. **The landed recipe test was meant to be amended too.** The packet says
   CG-001 "amends [the new tests 9/10] in place" later, and lists only
   `sampling_policy_resolve_refuses_while_stub` as the booked in-place
   amendment — but the fill of `profile` necessarily invalidates
   `recipe_evaluators_refuse_while_stub`. Under this reading CG-001 would also
   rewrite that landed test (the `()` spine, or the third assertion, or both).
   But the packet explicitly says all other landed tests stay byte-identical.
3. **The packet is internally inconsistent and the correct action is to stop
   and report.** This is the reading I acted on, per the stop conditions.

## Minimal consistent resolutions (for the orchestrator to choose)

- Option A (smallest): allow `constructive/mod.rs` to additionally re-export
  the spine types (`pub use recipe::{Spine, LineSpine, PolylineSpine};`), and
  amend `recipe_evaluators_refuse_while_stub` in place (e.g. keep `()`-spine
  refusal via the frame stub for `position`, but relax/drop the `profile`
  refusal assertion) — i.e. extend the "booked amendment" list by one test.
- Option B: declare `mod recipe;` as `pub mod recipe;` in mod.rs and amend the
  landed test as in Option A.
- Option C: instruct where the spine types should be re-exported and which
  landed-test change is sanctioned, then re-dispatch.

The library-side bodies themselves (recipe/profile/sampling fills) are
unambiguous and unblocked; the gap is only the two integration points above.
