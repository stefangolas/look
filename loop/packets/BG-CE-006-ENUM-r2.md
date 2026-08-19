# WORK PACKET BG-CE-006-ENUM-r2 — close the SPEC_GAP of the canonical model

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

**Context.** The previous packet on this branch moved `Curve`/`Surface` into
`truck-geometry/src/canonical.rs` and did all the in-scope work. It stopped at
SPEC_GAP because two files outside its write allowance did not compile and one
design question had no honest answer inside it. This packet is that delta. The
branch tip (your starting commit) holds the finished first stage; **build on
it, do not redo it**.

```json
{"id":"BG-CE-006-ENUM-r2","status":"DONE","contracts":["BG-CE-006"],
 "tests_added":3,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

```yaml
id:          BG-CE-006-ENUM-r2
contract:    [BG-CE-006]
class:       mechanical
crates:      [truck-geometry, truck-modeling, truck-stepio, truck-meshalgo]
depends_on:  [BG-CE-006-ENUM]
write_allow:
  - vendor/truck/truck-geometry/src/canonical.rs
  - vendor/truck/truck-geometry/src/lib.rs
  - vendor/truck/truck-geometry/src/decorators/revolved_curve.rs
  - vendor/truck/truck-modeling/src/geometry.rs
  - vendor/truck/truck-modeling/src/builder.rs
  - vendor/truck/truck-stepio/src/in/step_geometry/mod.rs
  - vendor/truck/truck-stepio/src/out/geometry.rs
  - vendor/truck/truck-meshalgo/tests/tessellation/triangulation.rs
read_allow:
  - vendor/truck/truck-geometry/src/decorators/processor.rs
  - vendor/truck/truck-geometry/src/specifieds/
  - vendor/truck/truck-stepio/src/in/
tests_required:
  - search_parameter_is_branch_consistent_for_periodic_profiles
  - placed_analytic_transform_goes_to_processor
  - processor_cylinder_emits_cylindrical_surface_with_rotated_placement
budget:      {turns: 35, ctx_tokens: 90000}
anchors:
  # Branch-neutral on purpose: these hold at the integration tip AND at this
  # packet's base (the branch commit holding stage 1). run_packet checks
  # anchors against the main worktree; the stage-1 files (canonical.rs etc.)
  # exist only on this branch, so they are prose, not anchors.
  - {id: A1, expect: 1, cmd: "grep -c 'impl SearchNearestParameter<D2> for Cylinder' vendor/truck/truck-geometry/src/specifieds/cylinder.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'use Surface::\\*' vendor/truck/truck-stepio/src/in/step_geometry/mod.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'Processor::new(surface_row).into()' vendor/truck/truck-meshalgo/tests/tessellation/triangulation.rs"}
  - {id: A4, expect: 4, cmd: "grep -c -E 'fn (search_parameter|search_nearest_parameter)' vendor/truck/truck-geometry/src/decorators/revolved_curve.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'pub struct Torus' vendor/truck/truck-geometry/src/specifieds/mod.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'mod partial_torus' vendor/truck/truck-modeling/src/builder.rs"}
  - {id: A7, expect: 1, cmd: "grep -c 'impl ParametricSurface for Sphere' vendor/truck/truck-geometry/src/specifieds/sphere.rs"}
```

## Problem

Four gaps, each already diagnosed; you close them.

1. `truck-stepio` does not compile: `truck-stepio/src/in/step_geometry/mod.rs`
   has its **own** `Surface` enum and a `use Surface::*;` that is now
   ambiguous (E0659) because the canonical `Surface` arrives via the
   truck-geometry prelude glob.
2. `truck-meshalgo/tests/tessellation/triangulation.rs` does not compile: it
   constructs `Processor::new(surface_row).into()` relying on the
   `From<Processor<RevolutedCurve<Curve>, Matrix4>> for Surface` impl the move
   deleted (E0277).
3. `builder::partial_torus` regresses at runtime: with a periodic
   `Curve::Circle` profile preserved through the sweep, `RevolutedCurve`'s
   parameter search returns branch-inconsistent u values (observed `−10π` and
   `11π`), flipping boundary orientation.
4. `Transformed<Matrix4>` on the analytic carriers has no honest story: the
   current center/apex-only transform is silently wrong under rotation, which
   is exactly the defect class this kernel exists to prevent.

## Decisions already made for you

1. **stepio-in disambiguation, minimal:** within
   `truck-stepio/src/in/step_geometry/`, the module's own enum keeps the bare
   name. Change the `use Surface::*;` to `use self::Surface::*;` and qualify
   any other bare uses in that file the same way if the compiler names them.
   Do not rename either enum. Do not touch the rest of `src/in/` — only what
   the compiler proves broken, and only in this one file.

2. **meshalgo port, explicit:** replace `Processor::new(surface_row).into()`
   with an explicit construction: if the fixture's transform is identity,
   `Surface::RevolutedCurve(surface_row.entity().clone())`; otherwise
   `Surface::Processor(surface_row)` (decision 4 adds the variant). Read the
   fixture to see which it is; the ported test's geometry must be unchanged
   and the test must pass.

3. **Branch-consistent search in `revolved_curve.rs`:** in every
   `search_parameter`/`search_nearest_parameter` implementation in that file,
   when the entity curve reports a period (its `u_period`/range admits one —
   for `Curve` the enum provides it; for the inner curve types use their own
   trait methods), normalize a found parameter by shifting it in multiples of
   the period to the branch **nearest the hint** when a hint parameter was
   given, and into the principal period otherwise. Add the regression test
   (see Tests). `builder::partial_torus` must pass again — it is the
   motivating case and V5 will run it.

4. **The placed-surface variant.** Add to `Surface`:

   ```rust
   /// A placed surface: the inner carrier composed with an affine map.
   /// Exact under affine; the honest home for a transformed z-canonical
   /// carrier (BG-CE-006-r2).
   Processor(Processor<Surface, Matrix4>),
   ```

   Then fix `Transformed<Matrix4> for Surface`'s analytic arms: for
   `Cylinder`, `Cone`, `Sphere`, `Torus` — if the matrix's linear part is
   **exactly the identity** (compare the 3×3 block; exact `==`, no epsilon),
   transform center/apex and keep radius/half-angle/both radii; **otherwise**
   return `Surface::Processor(Processor::with_transform(inner, m))`.
   `Surface::Processor(p).transformed(m)` composes:
   `Surface::Processor(p.transformed(m))`. The existing spline/decorator arms
   stay as they are (they are exact). Document on the variant why
   center-only was rejected. If `Processor<Surface, Matrix4>` lacks a trait
   the enum's derives require (`ParameterDivision2D`, search traits, …),
   write the delegating impl in `canonical.rs` (same crate; sanctioned).

5. **STEP-out `Processor` arm:** emit the inner carrier with the placement
   composed — for an analytic inner, the entity
   (`CYLINDRICAL_SURFACE`/`CONICAL_SURFACE`/`SPHERICAL_SURFACE`/
   `TOROIDAL_SURFACE`) with its placement composed with `m`; for spline
   inners, transform then emit (control-point transform is exact). Follow the
   emitter pattern already in the file from the previous packet.

6. Nothing else. The first packet's in-scope work stands; do not restructure
   it. Its three circle tests that skip non-finite samples of the degenerate
   full-range NURBS stay as they are (the NaN is the *old* conversion's
   defect and is now unreachable through the sweep path).

## Tests required

1. `search_parameter_is_branch_consistent_for_periodic_profiles` — in
   `revolved_curve.rs`'s test module: revolve a periodic profile (a
   `Curve::Circle`), search with a hint in the principal branch, assert the
   returned u lies within half a period of the hint; search without a hint,
   assert the result lies in the principal period.
2. `placed_analytic_transform_goes_to_processor` — in `canonical.rs`'s test
   module: translate a `Cylinder` → still `Surface::Cylinder` with the moved
   center; rotate the same → `Surface::Processor(_)`, and the placed point
   set equals the rotated original at sampled parameters (float tolerance).
3. `processor_cylinder_emits_cylindrical_surface_with_rotated_placement` — in
   `truck-stepio/src/out/geometry.rs` beside the previous packet's emitter
   tests: emit a rotated cylinder, assert the output contains
   `CYLINDRICAL_SURFACE` and that the placement's direction in the emitted
   text is the rotated axis.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-geometry -p truck-modeling -p truck-stepio -p truck-meshalgo
cargo clippy -p truck-geometry -p truck-modeling -p truck-stepio -p truck-meshalgo --all-targets --no-deps -- -D warnings
cargo test -p truck-geometry --lib --tests --no-fail-fast
cargo test -p truck-modeling --lib --tests --no-fail-fast
cargo test -p truck-stepio --lib --tests --no-fail-fast
cargo test -p truck-meshalgo --lib --tests --no-fail-fast
cargo check --workspace --all-targets
```

Never run a bare `cargo test`. Send cargo output to a file and read the tail.
`builder::partial_torus` passing again is a **requirement**, not a bonus.

## Forbidden

Editing any file outside `write_allow`. Renaming either `Surface` enum.
Re-adding `From<Processor<RevolutedCurve<Curve>, Matrix4>> for Surface` (the
explicit construction is the point). Epsilon comparisons in the
identity-linear-part check. Making a bound smaller. Adding `#[ignore]`.
Adding `unscaled_legacy(` call sites. Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- a file outside `write_allow` still fails to compile after decisions 1–2 →
  `SPEC_GAP`, naming it
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(geometry): placed-surface variant, branch-consistent revolution search (BG-CE-006-ENUM-r2)`.
