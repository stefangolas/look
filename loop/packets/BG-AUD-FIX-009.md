# WORK PACKET BG-AUD-FIX-009 — fillet derivative + totality (AUD-011, AUD-012)

You are repairing two defects found by the formal-kernel correctness audit
`loop/audits/BG-AUDIT-001.md` (findings AUD-011 and AUD-012), in
`truck-geometry`. Everything you need is in this document. **Do not read any
other spec file** — this packet is self-contained.

```json
{"id":"BG-AUD-FIX-009","status":"DONE","contracts":["AUD-011","AUD-012"],
 "tests_added":3,"deviations":[],"disagreements":[],
 "baseline_failures":[],"notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. **If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-AUD-FIX-009
contract:    [AUD-011, AUD-012]
class:       mechanical
crates:      [truck-geometry]
write_allow:
  - vendor/truck/truck-geometry/src/specifieds/torus.rs
  - vendor/truck/truck-geometry/src/decorators/af_surface.rs
  - vendor/truck/truck-geometry/src/decorators/rbf_surface/contact_circle.rs
read_allow:
  - vendor/truck/truck-geometry/src/decorators/rbf_surface/mod.rs
tests_required:
  - torus_normal_uder_matches_finite_difference
  - contact_points_singular_frame_refuses
  - fillet_reversed_range_refuses
budget:      {turns: 40, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'fn normal_uder' vendor/truck/truck-geometry/src/specifieds/torus.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'fn approx_rolling_ball_fillet' vendor/truck/truck-geometry/src/decorators/af_surface.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'fn contact_points' vendor/truck/truck-geometry/src/decorators/rbf_surface/contact_circle.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'fn next_point' vendor/truck/truck-geometry/src/decorators/rbf_surface/contact_circle.rs"}
```

## Problem

### AUD-011 — torus `normal_uder` returns a wrong z-component

`Torus::normal_uder` (torus.rs:121-124) returns `(−cos v·sin u, cos v·cos u,
sin v)`. But `normal(u,v) = (cos v·cos u, cos v·sin u, sin v)`, so `∂/∂u`
normal is `(−cos v·sin u, cos v·cos u, 0)` — the returned z-component `sin v`
is wrong (should be 0). Finite-difference verified. This feeds the fillet
Newton solve through `contact_circle::next_point`'s use of `normal_uder`
(AUD-011's consequence), so torus fillets solve with a wrong derivative.

### AUD-012 — reachable panics in the fillet approximation path (H-1)

`af_surface.rs` and `contact_circle.rs` violate the house rule H-1 (no
`unwrap`/`expect`/`debug_assert` panics on data reachable from untrusted
geometry). The audit-listed sites:

- af_surface.rs:442 `KnotVec::try_from(vec).unwrap()` — panics on a reversed
  edge parameter range (descending `v` values);
- af_surface.rs:468, :477 `mat.invert().unwrap()` — panics on a singular
  `[handle, cder, n]` frame (degenerate contact);
- af_surface.rs:539 `ccs.sort_by(|(x,_), (y,_)| x.partial_cmp(y).unwrap())` —
  panics on a NaN parameter;
- contact_circle.rs:149, :168 `mat.invert().unwrap()` — panics on a singular
  contact frame in `contact_points` / `next_point`;
- contact_circle.rs:176 `debug_assert!(del.z.so_small())` — fires in debug
  builds on scaled models (`del.z` is dimensionally `1/length`, documented as a
  SPEC_GAP at the site; the assertion compares it against a dimensionless
  tolerance).

**Your first obligation — observe the regressions fail on the buggy code:** add
the three tests below. `torus_normal_uder_matches_finite_difference` must FAIL
on the current wrong derivative; the two fillet tests must PANIC (abort) on the
buggy code. Record the pre-fix observations in `RESULT.json.notes`.

## Repair

### AUD-011

`torus.rs`: return `Vector3::new(-sv.x * f64::sin(u), sv.x * f64::cos(u),
0.0)` from `normal_uder` — the z-component becomes `0`. Keep `normal_vder`
unchanged (it is already correct: `∂/∂v normal`).

### AUD-012

Convert every audit-listed panic site into a typed refusal. All the af_surface
sites are inside `approx_rolling_ball_fillet`, which already returns
`Outcome<Self>` and already uses the `?`-style refusal
(`Refusal::NumericallyUnresolved { spent: budget_spent(initial, *budget),
witness: UnresolvedWitness::ContactCurveNotFound }`) — reuse that exact shape.
The contact_circle sites are inside `ContactCircle::try_new` (returns
`Option<Self>`) and its private helpers `contact_points` / `next_point`
(return plain tuples today); change those helpers to return `Option` and
propagate with `?` so `try_new` returns `None` (the existing refusal shape at
the public boundary).

- **af_surface.rs:442** — and to make the defect unreachable early, add an
  explicit guard at the top of `approx_rolling_ball_fillet`: a reversed range
  `v0 > v1` is invalid input → return the `NumericallyUnresolved` refusal
  immediately (before any contact computation). Keep the `KnotVec::try_from`
  failure as a refusal too (belt and braces).
- **af_surface.rs:468, :477** — `mat.invert()` returns an `Option`; map `None`
  to the same refusal with `?`.
- **af_surface.rs:539** — the sort's `partial_cmp` panics on NaN. Before
  sorting, if any `v` in `ccs` is non-finite, return the refusal. (The `v`
  values are parameters; a NaN here is a genuine numeric failure, not a sort
  problem.) If you prefer `sort_unstable_by` with `total_cmp`, you must STILL
  refuse on non-finite parameters — `total_cmp` sorts NaN silently and a NaN
  contact parameter would flow downstream into NaN geometry.
- **contact_circle.rs:149, :168** — `mat.invert()` → `None`, propagate through
  `?`.
- **contact_circle.rs:176** — remove the `debug_assert!` (its dimensional
  semantics are already documented as an unresolved SPEC_GAP at the site) and
  replace it with a non-finite/divergence guard in `next_point` that returns
  `None` when `del` is not finite. Do NOT keep a dimensionally-wrong
  assertion.

## Regression tests (exact names)

1. `torus_normal_uder_matches_finite_difference` — in `torus.rs`'s test module.
   At a nontrivial `v` (e.g. `v = 1.0`) and several `u`, compare
   `torus.normal_uder(u, v)` against the central finite difference of
   `normal`:
   `(normal(u+h, v) − normal(u−h, v)) / (2h)` with a small `h` (e.g. `1e-6`,
   same-line `// H-3`), asserting each component matches within `1e-5`-class
   slack (same-line `// H-3`). On the buggy code the z-component is `sin v`
   instead of `0` and the test fails.

2. `contact_points_singular_frame_refuses` — in `contact_circle.rs`'s test
   module. Call the private `contact_points` directly with a deliberately
   singular frame: `der` parallel to both `n0` and `n1` (so the 3×3 matrix
   `[der, n0, n1]` has rank < 3). After the fix it must return `None`; on the
   buggy code `mat.invert().unwrap()` panics (abort).

3. `fillet_reversed_range_refuses` — in `af_surface.rs`'s test module. Call
   `approx_rolling_ball_fillet` with a reversed `edge_parameter_range`
   `(1.0, 0.0)` on the existing test module's fillet witness (or the minimal
   setup needed to reach the range guard). It must return
   `Err(Refusal::NumericallyUnresolved { .. })`, never panic. On the buggy code
   it panics (abort) at the range guard's absence / the `KnotVec` unwrap.

Every pre-existing test in the three files must stay green, in particular the
`ApproxFilletSurface` proptest in `af_surface.rs` (which exercises the normal
`(0.0, 1.0)`-ordered range) and the tolerance_decorators test that asserts the
`next_point` FIXME site is NOT migrated — that test asserts the source still
contains `fn next_point` and no `ToleranceCtx` inside it; keep the `// FIXME`
comment at the site.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N`-shaped literal unless that line ends with an `// H-3` comment. The
finite-difference `h = 1e-6` and the component slack need the same-line
`// H-3` comment. Run `bash scripts/kernel-gates.sh <your base commit>`
yourself before writing `RESULT.json`.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-geometry
cargo clippy -p truck-geometry --all-targets --no-deps
cargo test -p truck-geometry --lib --tests --no-fail-fast
cargo test -p truck-geometry --doc
cargo check --workspace --all-targets
bash scripts/kernel-gates.sh <your base commit>
```

Also run `cargo test -p truck-shapeops --lib` and report it in
`RESULT.json.notes` — the shapeops fillet module is the consumer of
`approx_rolling_ball_fillet`; its existing tests must stay green against the
new refusal paths.

Never run a bare `cargo test`.

## Forbidden

Editing any file outside `write_allow` (the shapeops consumer is NOT in your
write set — if its suite fails on your change, report it; do not edit it).
Keeping any audit-listed `unwrap`/`debug_assert` reachable from data.
Silently sorting a NaN parameter instead of refusing it. Adding `#[ignore]`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- a required regression cannot be driven through the real API (e.g. the
  reversed-range test cannot reach the guard without a full `RbfSurface`
  setup that is not constructible in the test module) → `SPEC_GAP`, with the
  exact obstacle and the closest constructible regression
- the shapeops suite fails on your change in a way you cannot see from
  `truck-geometry` → report it in `disagreements`; do not weaken the refusal
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`fix(geometry): torus normal_uder z-component; typed refusals for fillet panics (BG-AUD-FIX-009)`.
