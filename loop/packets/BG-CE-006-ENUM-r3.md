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
covers:      [BG-CE-006-ENUM, BG-CE-006-ENUM-r2]
contract:    [BG-CE-006]
class:       mechanical
crates:      [truck-geometry, truck-shapeops]
depends_on:  [BG-CE-006-ENUM-r2]
write_allow:
  - vendor/truck/truck-shapeops/src/transversal/divide_face/mod.rs
  - vendor/truck/truck-shapeops/src/transversal/divide_face/tests.rs
  - vendor/truck/truck-shapeops/src/transversal/integrate/mod.rs
  - vendor/truck/truck-shapeops/src/transversal/integrate/tests.rs
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

## Problem — root cause FOUND (probes, 2026-08-19)

`truck-shapeops/src/transversal/integrate/tests.rs::punched_cube` fails at
`crate::and(...).unwrap()`: the boolean returns `None`. Bisection (all
verified by direct experiment on this branch's tip):

1. Reverting **only** `revolved_curve.rs` to the pre-r2 state does not fix it.
2. Restoring **only** the old circle→NURBS degradation in `canonical.rs`
   makes it pass — the preserved `Curve::Circle` is the cause.
3. The failure is at `divide_face::create_parameter_boundary`, located by
   running the test under instrumentation: the punch cylinder's wall face has
   parameter domain `(u = angle ∈ [0, 2π), v = height)`, the boundary wire's
   circle edges carry **over-wrapping ranges (e.g. `(0.0, 8.0)`)**, and
   `create_parameter_boundary` maps each polyline point through
   `surface.search_parameter(q, Some(p.into()), 100)` — which returns
   **principal-branch** angles. Consecutive points at circle-parameters 6.2
   and 6.3 therefore map to angles ~6.2 and ~0.02: the mapped boundary
   teleports across the seam, the 2D "polygon" self-crosses, the domain
   decomposition degenerates (`pre_faces=0`, `negative_wires=3`), and
   `divide_faces` returns `None`. The old NURBS degrade never hit this
   because its parameterization was non-periodic, so an over-wrap was a
   simple polygon in parameter space.

This is the same branch-consistency contract r2 applied to `RevolutedCurve`,
missing at the engine's surface-parameter mapping.

## Decisions already made for you

1. **The fix: periodic-domain parameter unwrapping in
   `create_parameter_boundary`.** When the face's surface reports a `u_period`
   (`ParametricSurface::u_period()` — already implemented; the analytic
   carriers return `Some(2π)`), each parameter returned by
   `search_parameter` must be **unwrapped relative to the previous point's
   parameter**: shifted by integer multiples of the period so that the jump
   from the previous point is at most half a period in absolute value. The
   initial point (`search_parameter(pt, None, 100)`) stays on the principal
   branch. Apply the same treatment anywhere else in the transversal engine
   that chains `search_parameter` results into a parameter-space polyline
   (grep the module for other `search_parameter` uses; fix what chains,
   leave what does not).
2. **Check the reverse path.** If the engine maps parameter-domain results
   back through `subs(u, v)` or re-emits curves cut at these parameters,
   over-wrapped u values must survive that round trip (they are legitimate
   parameters of a periodic domain). Say in RESULT.json whether the reverse
   path needed anything.
3. **The fix direction is unwrap-in-the-engine, not re-degrade circles.**
   Restoring the old conversion globally is forbidden — it is the regression
   of the whole point of this branch.
4. **Regression test:** `circle_boundary_is_processable_by_transversal_engine`
   — a focused test exercising `create_parameter_boundary` (or the smallest
   public seam above it) with an over-wrapping circle edge on a
   `Cylinder`-surface face, asserting the mapped boundary is seam-continuous
   (no jump exceeds half a period between consecutive points).
   `punched_cube` passing is also required and V5 will enforce it.
5. **Nothing else moves.** `builder::partial_torus` and
   `tsweep_circle_yields_cylinder` must stay green (V5 runs truck-modeling).
   No behavior change may be reachable for boundary curves on
   non-periodic-domain surfaces.
6. **The prior attempt's instrumentation is archived, not yours to restore.**
   If you add probes of your own, remove them before the final commit.

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
