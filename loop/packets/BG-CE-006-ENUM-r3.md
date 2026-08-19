# WORK PACKET BG-CE-006-ENUM-r3 — the boolean engine must consume `Curve::Circle`

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

**Context.** This branch already holds two landed-in-branch stages: the
canonical `Curve`/`Surface` model in `truck-geometry/src/canonical.rs` (which
now **preserves placed circles as `Curve::Circle`** instead of degrading them
to NURBS), and the placed-surface/branch-consistency delta. The canonical model
is correct and green for its own crates. What is broken is downstream:
`truck-shapeops`' transversal boolean engine cannot process the preserved
circle, and a previously-working boolean now fails.

```json
{"id":"BG-CE-006-ENUM-r3","status":"DONE","contracts":["BG-CE-006"],
 "tests_added":1,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

```yaml
id:          BG-CE-006-ENUM-r3
contract:    [BG-CE-006]
class:       mechanical
crates:      [truck-geometry, truck-shapeops]
depends_on:  [BG-CE-006-ENUM-r2]
write_allow:
  - vendor/truck/truck-shapeops/src/transversal/
  - vendor/truck/truck-geometry/src/canonical.rs
  - vendor/truck/truck-geometry/src/specifieds/circle.rs
read_allow:
  - vendor/truck/truck-shapeops/src/
  - vendor/truck/truck-geometry/src/decorators/
  - vendor/truck/truck-meshalgo/src/tessellation/
tests_required:
  - circle_boundary_is_processable_by_transversal_engine
budget:      {turns: 45, ctx_tokens: 90000}
anchors:
  # Branch-neutral (true at the integration tip and at this branch's base).
  - {id: A1, expect: 1, cmd: "grep -c 'pub fn and' vendor/truck/truck-shapeops/src/transversal/integrate/mod.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'fn punched_cube' vendor/truck/truck-shapeops/src/transversal/integrate/tests.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub struct UnitCircle' vendor/truck/truck-geometry/src/specifieds/mod.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'fn parameter_division' vendor/truck/truck-geometry/src/specifieds/circle.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'mod integrate' vendor/truck/truck-shapeops/src/transversal/mod.rs"}
```

## Problem — with the bisection already done for you

`truck-shapeops/src/transversal/integrate/tests.rs::punched_cube` builds a
cube, punches it with a swept circle (`rsweep` → arc wire → `try_attach_plane`
→ `tsweep` → `not()` → `and`), and fails at `crate::and(...).unwrap()`:
**the boolean returns `None`**. The orchestrator bisected this to root-cause
candidate level by direct experiment on this branch's tip:

1. Reverting **only** `truck-geometry/src/decorators/revolved_curve.rs` to the
   pre-r2 state does **not** fix it — r2's branch normalization is not the
   cause.
2. Restoring **only** the old circle→NURBS degradation inside
   `impl ToSameGeometry<Curve> for Processor<TrimmedCurve<UnitCircle<Point3>, Matrix4>>`
   in `canonical.rs` **makes it pass** — the preserved `Curve::Circle`
   representation is the cause.
3. Changing the punch's sweep from an over-wrapping `Rad(7.0)` to an
   under-wrapping `Rad(6.0)` still fails — **every** preserved circle breaks
   the engine, not just over-wrapping trims.

So: somewhere in `process_one_pair_of_shells` (in
`transversal/integrate/mod.rs`), the pipeline returns `None` when an input edge
is a `Curve::Circle`. The candidate give-up points, in the order the code calls
them: `Shell::triangulation` → `loops_store::create_loops_stores` →
`divide_face::divide_faces` → `cls.and_or_unknown` / `signed_crossing_faces` →
`altshell_to_shell`'s `BSplineCurve::quadratic_approximation(...)`. Note the
engine is **generic** over the curve type — it never names `Curve::Circle`;
whatever breaks is a behavior difference of the circle payload through a trait
the engine already uses (`ParameterDivision1D`, `SearchParameter`, `Cut`,
`SearchNearestParameter` on `Processor<TrimmedCurve<UnitCircle<Point3>, Matrix4>>`).

## Decisions already made for you

1. **The fix direction is to make the engine correct on the preserved circle,
   not to re-degrade circles.** Restoring the old conversion globally is
   forbidden — it is the regression of the whole point of this branch. If,
   after diagnosis, you conclude the only sane fix is a **localized**
   representation change *inside the engine's own machinery* (e.g. the engine
   pre-mapping `Curve::Circle` edges to their spline approximation at its
   boundary while leaving the input shell untouched), that is admissible
   **only** if the *result* shell keeps analytic identity where the input had
   it on edges the engine did not cut, and only if you say so in RESULT.json
   deviations with the reason. A global or caller-visible degrade is not.
2. **Diagnose first, then fix minimally.** Add temporary eprintln/instrument
   runs as you need (they are probes — remove them before your final commit).
   Find the exact `None`. Fix the minimal cause. Do not rewrite the engine.
3. **Likely suspects, checked or not by the orchestrator** (you verify):
   - `ParameterDivision1D` of `UnitCircle`/`TrimmedCurve<UnitCircle>` yielding
     a different point distribution than the old NURBS path did — e.g. a
     duplicate or near-duplicate point at the seam/period boundary that the
     polyline machinery (spatial hash, polyline intersection) cannot digest.
   - `SearchParameter`/`SearchNearestParameter` on the circle payload
     returning branch-inconsistent values into
     `loops_store`/`divide_faces` (r2 fixed `RevolutedCurve`'s searches; the
     circle payload's own searches were not audited).
   - `Cut` on `TrimmedCurve<UnitCircle>` producing a trim representation the
     engine's edge re-assembly rejects.
4. **Regression test:** `circle_boundary_is_processable_by_transversal_engine`
   — a focused unit test in `transversal/integrate/tests.rs` (or beside the
   stage you fix, your judgement) that exercises the exact failing stage with a
   circle-bearing boundary, so the fix is pinned at the level it was made.
   `punched_cube` passing is also required and V5 will enforce it.
5. **Nothing else moves.** `builder::partial_torus` and
   `tsweep_circle_yields_cylinder` must stay green (V5 runs truck-modeling).
   No behavior change may be reachable for boundary curves that are not
   circles.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-geometry -p truck-shapeops
cargo clippy -p truck-geometry -p truck-shapeops --all-targets --no-deps -- -D warnings
cargo test -p truck-geometry --lib --tests --no-fail-fast
cargo test -p truck-shapeops --lib --tests --no-fail-fast
cargo test -p truck-modeling --lib --tests --no-fail-fast
cargo check --workspace --all-targets
```

Never run a bare `cargo test`. Send cargo output to a file and read the tail.
Pre-existing failures you did not cause: confirm at the branch base commit,
record in `baseline_failures`, leave them.

## Forbidden

Editing any file outside `write_allow`. Re-introducing the global circle→NURBS
degradation. Changing the canonical enum shapes. Committing instrumentation.
Adding `#[ignore]`. Adding `unscaled_legacy(` call sites. Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- the root cause is a design defect in the engine that a minimal fix cannot
  reach (i.e. the engine structurally requires spline-representable curves and
  no localized fix is honest) → `SPEC_GAP` with the analysis — this is a
  valuable answer, not a failure
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it. **Report the root cause
in `notes` — the one-line "the None came from X" is the single most valuable
sentence in your result.**

Commit on the current branch with subject
`fix(shapeops): transversal engine consumes preserved Circle edges (BG-CE-006-ENUM-r3)`.
