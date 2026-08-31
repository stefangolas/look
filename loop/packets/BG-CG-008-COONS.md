# WORK PACKET BG-CG-008-COONS — the bilinearly blended Coons patch

You are landing the Coons4 deliverable of the constructive geometry program
(plan §3.7): a `CoonsSurface` decorator over four boundary curves, with
analytic derivatives, a corner-validating constructor, and a certified (never
assumed) Jacobian. The design is already made — transcribe it. Do not read
other spec files and do not redesign anything named here. If something you
need is genuinely missing, that is a SPEC_GAP (see "Stop conditions"): you
stop and report, you do not research it.

This packet is parallel-eligible and OFF the rendering critical path (plan
§2); it shares no files with any other live packet.

```yaml
id:          BG-CG-008-COONS
contract:    [BG-CG-008-COONS]
class:       design
crates:      [truck-geometry]
depends_on:  [BG-CG-000-CONTRACT]
write_allow:
  - vendor/truck/truck-geometry/src/decorators/coons.rs
  - vendor/truck/truck-geometry/src/decorators/mod.rs
  - vendor/truck/truck-geometry/tests/coons_conformance.rs
read_allow:
  - docs/CONSTRUCTIVE_GEOMETRY_PLAN.md
  - vendor/truck/truck-geometry/src/decorators/mod.rs
  - vendor/truck/truck-geometry/src/decorators/homotopy.rs
  - vendor/truck/truck-geotrait/src/traits/surface.rs
  - vendor/truck/truck-geotrait/src/traits/curve.rs
  - vendor/truck/truck-geometry/src/constructive/mod.rs
  - vendor/truck/truck-base/src/tolerance.rs
tests_required:
  - coons_corners_validate_and_refuse_mismatched
  - coons_boundary_interpolates_exactly
  - coons_first_derivatives_match_finite_differences
  - coons_degenerate_u_collapse_has_vanishing_jacobian
  - coons_convenience_constructor_picks_a_consistent_orientation
  - coons_inverse_matches_reparametrization
  - coons_higher_derivatives_vanish
budget:      {turns: 40, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 0, cmd: "grep -c 'coons' vendor/truck/truck-geometry/src/decorators/mod.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub struct HomotopySurface' vendor/truck/truck-geometry/src/decorators/mod.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'fn der_mn' vendor/truck/truck-geometry/src/decorators/homotopy.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub struct Line' vendor/truck/truck-geometry/src/specifieds/mod.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'pub struct DirectTolerance' vendor/truck/truck-geometry/src/constructive/mod.rs"}
```

## The two lines you add to `decorators/mod.rs`

With the other private module declarations (after `mod trimmied_curve;` is a
fine place):

```rust
mod coons;
```

and next to whatever mechanism exposes the sibling surface types (note:
`HomotopySurface`'s STRUCT lives in mod.rs and homotopy.rs carries only impls
— Coons deliberately does NOT follow that split; it is self-contained so the
CG write set stays minimal):

```rust
pub use coons::CoonsSurface;
```

Nothing else in mod.rs moves.

## The type (all in `decorators/coons.rs`, new file)

File header: `#![deny(clippy::unwrap_used)]` (GATE-1), then
`use super::*;` and whatever trait imports the sibling impls use
(`homotopy.rs` lines 1–3 are the reference).

Convention (normative): `bottom` runs u: 0→1 at v = 0; `top` runs u: 0→1 at
v = 1; `left` runs v: 0→1 at u = 0; `right` runs v: 0→1 at u = 1. Corners:
P00 = bottom(0) = left(0), P10 = bottom(1) = right(0), P01 = top(0) =
left(1), P11 = top(1) = right(1).

```rust
/// The bilinearly blended Coons patch of four boundary curves (plan §3.7).
///
/// Boundary correctness is by EXACT pairwise cancellation against the corner
/// term in exact arithmetic; in floats it holds to
/// `DirectTolerance::default().position` and the tests assert exactly that.
///
/// Regularity is certified, never assumed: `jacobian` exposes
/// J = S_u × S_v; a folded patch is construction-valid but geometry-invalid.
#[derive(Clone, Debug, PartialEq)]
pub struct CoonsSurface<C0, C1, D0, D1> {
    bottom: C0,   // private fields; accessors below
    top: C1,
    left: D0,
    right: D1,
    p00: Point3, p10: Point3, p01: Point3, p11: Point3,  // corners, cached at construction
}
```

All four curves are `ParametricCurve3D` (i.e.
`ParametricCurve<Point = Point3, Vector = Vector3>` — copy the exact bound
spelling from `homotopy.rs`). Provide `bottom()`, `top()`, `left()`,
`right()` accessors returning `&C0` etc. Do not add `*_mut` accessors (a
mutated boundary would invalidate the cached corners).

## Constructors (exact semantics)

```rust
impl<C0, C1, D0, D1> CoonsSurface<C0, C1, D0, D1> { /* both constructors */ }
```

- **`try_new(bottom, top, left, right) -> Result<Self, ConstructError>`** —
  evaluates the four corners and validates, pairwise, the four corner
  equalities at `DirectTolerance::default().position` (from
  `truck_geometry::constructive::DirectTolerance` — the CG-000 type; import
  it through `crate::constructive::DirectTolerance`). Any mismatch or any
  non-finite corner → `Err(ConstructError::InvalidInput)`. (Use
  `crate::constructive::ConstructError` — the frozen currency; do NOT invent
  a new error type.) On success, cache the four corners.
- **`try_new_any_orientation(bottom, top, left, right) ->
  Result<(Self, [bool; 4]), ConstructError>`** — the plan's "convenience
  constructor MAY try finite legal reversals and return the chosen one".
  Booked exactly: try the 16 combinations of `(inverted?, ...)` flags over
  (bottom, top, left, right) in lexicographic order (false < true; bottom's
  flag is the most significant), using `curve.inverse()` for a `true` flag;
  return the FIRST `try_new` success together with the flag vector; the
  `false`-everywhere combination is tried first, so a consistent-as-given
  input returns flips `[false; 4]`. All 16 refuse → `Err(_)` of the last
  `try_new`'s error. Deterministic; never guesses beyond the finite set.

## The evaluation formula and derivatives (exact, quoted — transcribe, do not re-derive)

Let b = bottom(u), t = top(u), l = left(v), r = right(v). For
`(u, v) ∈ [0,1]²`:

```text
S(u,v)  = (1−v)·b + v·t + (1−u)·l + u·r
        − [ (1−u)(1−v)·P00 + u(1−v)·P10 + (1−u)v·P01 + uv·P11 ]

S_u     = (1−v)·b′(u) + v·t′(u) − l + r − [ (1−v)·(P10−P00) + v·(P11−P01) ]

S_v     = −b + t + (1−u)·l′(v) + u·r′(v) − [ (1−u)·(P01−P00) + u·(P11−P10) ]

S_uu    = (1−v)·b″(u) + v·t″(u)

S_uv    = −b′(u) + t′(u) − l′(v) + r′(v) + (P10−P00) − (P11−P01)

S_vv    = (1−u)·l″(v) + u·r″(v)
```

`der_mn(m, n, u, v)`: (0,0) → `subs`; (1,0) → S_u; (0,1) → S_v; (2,0) →
S_uu; (1,1) → S_uv; (0,2) → S_vv; **every (m, n) with m + n ≥ 3 →
`Vector3::zero()`** (the corner term is degree ≤ 1 in each variable and the
boundary terms are single-curve derivatives — this vanishing is a theorem,
and test 7 pins it). Implement `der_mn` as the single source of truth and
define `subs`/`uder`/`vder`/`uuder`/`uvder`/`vvder` as forwarders to it,
matching the sibling style.

```rust
/// J = S_u × S_v at (u, v) — the certified regularity witness. A folded
/// patch (construction-valid) has J vanishing somewhere; the caller
/// certifies, this only reports.
pub fn jacobian(&self, u: f64, v: f64) -> Vector3 { self.uder(u, v).cross(self.vder(u, v)) }
```

## The trait checklist (plan §3.7 — all of them, no exceptions)

`ParametricSurface` (Point = Point3, Vector = Vector3), `ParametricSurface3D`,
`BoundedSurface`, `ParameterDivision2D`, `SearchParameter<D2>`,
`Invertible`, `Transformed<Matrix4>`, `IncludeCurve` for each of the four
boundary curve parameters (copy the exact bound shape the siblings use —
`homotopy.rs` implements the same checklist for a two-curve surface and is
the worked reference for every trait's shape; read it and its imports before
writing anything).

Trait-by-trait semantics:

- `parameter_range`: `((Included(0.0), Included(1.0)), (Included(0.0),
  Included(1.0)))`.
- `ParametricSurface3D::normal`: the sibling's form (cross of the first
  partials).
- `SearchParameter<D2>`: the standard Newton projection over the analytic
  derivatives, exactly the pattern the siblings use (the crate's own
  `algo::surface::search_parameter` machinery). This is a decorator
  obligation, NOT the FAC fast path — the §3.3 performance contract does not
  apply here.
- `Invertible::inverse`: flips the u direction — the target identity is
  `self.inverse().subs(u, v) == self.subs(1.0 - u, v)` pointwise, and the
  normal flips sign. Derive the curve/corner assignment from the formula
  (this is the packet's single derivation duty — the identity above is the
  machine-checkable contract; test 6 pins it on a grid).
- `Transformed<Matrix4>::transform_by`: transform the four curves AND the
  four cached corners (`Point3::transform_by`), preserving the corner
  equalities (a rigid/affine map preserves coincidence within the transform's
  own arithmetic; no revalidation).
- `IncludeCurve`: `true` exactly for a boundary curve of the patch.

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!` — in
  code and tests (the file denies `clippy::unwrap_used`; tests use
  `matches!` / `assert!(matches!(..))` / field comparisons).
- **H-3** No `1e-…` literals anywhere (GATE-2 scans test files). The
  finite-difference step in test 3 must be a DYADIC decimal written out
  (e.g. `1.0 / 1024.0`), and every comparison bound routes through
  `truck_base::tolerance::TOLERANCE` or `DirectTolerance::default()` —
  never a bare literal. If a line truly cannot avoid the regex, the same-line
  `// H-3` marker is the only opt-out (same line, never the line above).
- **H-6** Float comparisons in tests assert nearness through the tolerance
  constants, never exact equality against computed geometry (exact equality
  is fine only for cached corner values copied verbatim).
- The crate warns `missing_docs, missing_debug_implementations` and denies
  warnings in release: doc-comment every public item; `#[derive(Clone, Debug,
  PartialEq)]` on the struct (copy `homotopy.rs`'s derive set and add
  `Serialize, Deserialize` ONLY if the siblings carry it for this shape).
- No `unscaled_legacy(` calls (GATE-4); no `debug_new`, no
  `cfg!(debug_assertions)` (GATE-3).

## Tests required — `tests/coons_conformance.rs` (new file)

Header `#![deny(clippy::unwrap_used)]` (GATE-1). Fixtures: four `Line`-based
boundary curves (or `PolylineCurve<Point3>` — whichever implements
`ParametricCurve3D` in this crate; both live in `specifieds`/prelude) forming
a planar unit square first, then a warped quad. The warped quad (test 6
needs it non-planar): move `top` to a different height. Every fixture's
premise is machine-checked before the assertion that depends on it.

1. `coons_corners_validate_and_refuse_mismatched` — the unit-square quad is
   `Ok`; moving `top`'s endpoint off `right(1)` by `10.0 * TOLERANCE` makes
   `try_new` `Err(ConstructError::InvalidInput)`.
2. `coons_boundary_interpolates_exactly` — on an 11×11 grid, `S(u, 0)` ≈
   `bottom(u)`, `S(u, 1)` ≈ `top(u)`, `S(0, v)` ≈ `left(v)`, `S(1, v)` ≈
   `right(v)`, each within `DirectTolerance::default().position` (the float
   cancellation the plan asserts numerically).
3. `coons_first_derivatives_match_finite_differences` — analytic S_u/S_v vs
   central differences (dyadic step) on an interior 7×7 grid, agreeing
   within `64.0 * TOLERANCE`.
4. `coons_degenerate_u_collapse_has_vanishing_jacobian` — `left == right`
   (identical curves, consistent corners): `try_new` still SUCCEEDS
   (construction-valid), but `jacobian` is within
   `DirectTolerance::default().position` of the zero vector on an interior
   grid (geometry-invalid — surfaced, never hidden).
5. `coons_convenience_constructor_picks_a_consistent_orientation` — feed the
   warped quad with `top` and `right` inverted: plain `try_new` refuses,
   `try_new_any_orientation` succeeds with flips `[false, true, false, true]`,
   and the returned patch evaluates like the hand-forwarded version.
6. `coons_inverse_matches_reparametrization` — on an 11×11 grid:
   `inverse().subs(u, v)` ≈ `subs(1.0 - u, v)` within
   `DirectTolerance::default().position`, and `inverse()`'s normal ≈ −
   original's normal.
7. `coons_higher_derivatives_vanish` — `der_mn(1, 2, ..)`,
   `der_mn(3, 0, ..)`, `der_mn(2, 1, ..)` are exactly `Vector3::zero()`.

No existing test may be deleted, `#[ignore]`d, or weakened.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-geometry
cargo clippy -p truck-geometry --all-targets -- -D warnings
cargo test -p truck-geometry --lib --tests
```

Never run a bare `cargo test`. Purely additive over the workspace; the
verifier runs the workspace gates authoritatively. Send cargo output to a
file and read the tail.

## Forbidden

Editing any file outside `write_allow` — especially
`constructive/**` (import the CG-000 types, never edit them),
`decorators/homotopy.rs` (read-only reference), `Cargo.toml`, `Cargo.lock`,
`scripts/kernel-gates.sh`. Adding any smoothing/fitting/optimization on the
evaluation path. Adding `#[ignore]`. Adding `#[allow]` without a same-line
justification. Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH` (A1 must read 0 — it proves
  coons does not exist yet; A4 must read 1 — the fixture curve type exists)
- the design as written cannot compile as specified → `SPEC_GAP`, naming the
  exact conflict
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

```json
{"id":"BG-CG-008-COONS","status":"DONE","contracts":["BG-CG-008-COONS"],
 "tests_added":7,"anchors_verified":{"A1":0,"A2":1,"A3":1,"A4":1,"A5":1},
 "notes":"any deviation from the quoted design, with the reason"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it: what you attempted, the
exact ambiguity, and the readings you could not choose between.

Commit on the current branch with subject
`feat(geometry): bilinearly blended Coons patch (BG-CG-008-COONS)`
BEFORE writing `RESULT.json`.
