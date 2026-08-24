# WORK PACKET BG-AUD-FIX-006 — canonical transform + full-circle routing (AUD-005, AUD-009)

You are repairing two defects found by the formal-kernel correctness audit
`loop/audits/BG-AUDIT-001.md` (findings AUD-005 and AUD-009), both in
`truck-geometry`. Everything you need is in this document. **Do not read any
other spec file** — this packet is self-contained.

```json
{"id":"BG-AUD-FIX-006","status":"DONE","contracts":["AUD-005","AUD-009"],
 "tests_added":3,"deviations":[],"disagreements":[],
 "baseline_failures":[],"notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. **If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-AUD-FIX-006
contract:    [AUD-005, AUD-009]
class:       design
crates:      [truck-geometry]
write_allow:
  - vendor/truck/truck-geometry/src/canonical.rs
  - vendor/truck/truck-geometry/src/specifieds/circle.rs
read_allow:
  - vendor/truck/truck-geometry/src/decorators/revolved_curve.rs
  - vendor/truck/truck-geometry/src/decorators/processor.rs
  - vendor/truck/truck-geometry/src/nurbs/nurbscurve.rs
tests_required:
  - revoluted_curve_nonconformal_transform_is_placed
  - full_circle_conversion_antipode_is_finite
  - full_circle_include_on_plane_is_true
budget:      {turns: 45, ctx_tokens: 110000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'fn transform_revolution_axis' vendor/truck/truck-geometry/src/canonical.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'Self::RevolutedCurve(entity) => Self::RevolutedCurve' vendor/truck/truck-geometry/src/canonical.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'ToSameGeometry<NurbsCurve<Vector4>> for TrimmedCurve<UnitCircle<Point3>>' vendor/truck/truck-geometry/src/specifieds/circle.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'fn placed_surface' vendor/truck/truck-geometry/src/canonical.rs"}
```

## Problem

### AUD-005 — `RevolutedCurve::transformed` silently returns the wrong surface

`Surface::transformed` (canonical.rs:344) rebuilds a `RevolutedCurve` from the
transformed profile, origin and a normalized image of the axis for ANY affine
matrix. The image of a surface of revolution under a non-uniform scale or
shear is generally NOT a surface of revolution — scaling a circular cylinder
by `diag(1, 2, 1)` yields an elliptic cylinder, but the code returns the
original circular cylinder rebuilt on the scaled origin. Separately,
`transform_revolution_axis` substitutes `unit_z()` when the axis image is
zero/NaN, silently choosing a different axis instead of refusing.

**Witness (AUD-005):** a revolved surface with profile `Line((1,0,0),
(1,0,1))` about the z-axis (a unit circular cylinder), transformed by
`diag(1, 2, 1)`. True image: the elliptic cylinder `(cos u, 2 sin u, t)`.
Current result: the original circular cylinder (max point error 1.0).

### AUD-009 — full-circle → NURBS conversion degrades to negative/zero weights

`Surface::include` routes every `Curve::Circle` through
`ToSameGeometry::<NurbsCurve<Vector4>>` (canonical.rs:676-679). For a full
circle (`angle = t1 − t0 = 2π`) the conversion (circle.rs:219-237) produces a
middle control point with weight `cos(angle/2) = cos(π) = −1`; at the antipode
the evaluated weight is exactly 0, so `subs(π)` is NaN and `include` returns
false for a full circle that genuinely lies on the surface. A half-circle
(weight `cos(π/2) = 0` at the middle control point) is fine — the evaluated
weight `(1−t)² + t²` stays positive — which is why the same half-circle
includes as true.

## Repair — AUD-005

In `Surface::transformed` (canonical.rs), change the `RevolutedCurve` arm:

- if `identity_linear_part(trans)` (a translation), keep the current rebuild
  (`Self::RevolutedCurve(entity.transformed(trans))` — the axis image is the
  axis, never degenerate);
- otherwise route through `placed_surface(Surface::RevolutedCurve(*entity),
  trans)` exactly like the analytic carriers do. `Processor::with_transform`
  composes the map exactly at every evaluation, so the result is the TRUE
  transformed image (for `diag(1,2,1)`, the elliptic cylinder).

Then make the direct trait impls honest about the axis. Change
`transform_revolution_axis` to return `Option<Vector3>` (the normalized image
when finite and nonzero, else `None`), and in `RevolutedCurve::transformed` /
`transform_by`: on `Some(axis)` rebuild with it; on `None` (a degenerate axis
image — a projection/zero matrix) return the surface UNCHANGED, with a doc
note that a degenerate matrix is refused-by-identity. NEVER substitute
`unit_z()` for a degenerate axis image. `transform_by` mirrors `transformed`.

Do not add an epsilon-based "conformal" check: the identity-vs-else rule above
is deliberately conservative (a rotation routes through `placed_surface`,
which is always geometrically exact) and is the same rule the analytic
carriers already use. The regression below only needs `diag(1,2,1)` to come
back as the true elliptic image.

## Repair — AUD-009

In `specifieds/circle.rs`, special-case the full circle in
`ToSameGeometry<NurbsCurve<Vector4>> for TrimmedCurve<UnitCircle<Point3>>`.
Decided shape: when `angle >= 2.0 * PI` (a full circle, within the crate's
usual tolerance-free exact comparison on the range tuple — if the range is
exactly `(0, 2π)` from the caller, treat `angle >= 2π` as full), split the
arc into two half-circle pieces `[t0, t0+π]` and `[t0+π, t0+2π]`, convert each
piece through the EXISTING half-circle path (which already produces
weight-0-middle but never-degenerate arcs), and join them into ONE
`NurbsCurve<Vector4>` on a shared knot vector (two quadratic Bezier spans).
The join must keep the same endpoint/antipode geometry. A multi-turn arc
(`angle > 2π`) may be split into `ceil(angle/π)` half-circles or refused with
a documented refusal — choose one and say which in `RESULT.json.notes`.

The result must satisfy: `subs(π)` (the antipode of a circle starting at
`t0 = 0`) is FINITE and equals the antipodal point; the evaluated weight never
hits 0; and the include paths that consume this conversion (`Surface::Plane`
include via NURBS, and any NURBS-carrier include that reaches circles) return
`true` for a full circle that lies on the surface.

## Regression tests (exact names)

1. `revoluted_curve_nonconformal_transform_is_placed` — build
   `Surface::RevolutedCurve(RevolutedCurve::by_revolution(Line(Point3::new(1.0,
   0.0, 0.0), Point3::new(1.0, 0.0, 1.0)), Point3::origin(),
   Vector3::unit_z()))`, transform by the non-uniform scale
   `Matrix4::from_nonuniform_scale(1.0, 2.0, 1.0)` (or the cgmath64 spelling
   that compiles), and assert the result evaluates to the true elliptic image:
   at sample parameters `(u, v)` the point equals `(cos u, 2·sin u, v)` up to
   the crate's float assertion macro. Also assert the result is NOT
   `Surface::RevolutedCurve` (it is `Surface::Processor`) — or assert only the
   geometry if the placed wrapper type is not nameable in the pattern.

2. `full_circle_conversion_antipode_is_finite` — build a full circle
   `TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, 2.0 * PI))`, convert
   via `ToSameGeometry::<NurbsCurve<Vector4>>::to_same_geometry`, and assert:
   `nurbs.subs(t)` is finite for a sweep of `t` including `t = 0.5` (the
   antipode after the added interior knots — find the exact antipodal
   parameter by evaluating) and the antipodal point matches the circle's true
   antipode. Also assert every sampled evaluated weight is `> 0` if the weight
   is reachable from the public API.

3. `full_circle_include_on_plane_is_true` — a unit circle at `z = 1` in the
   plane `z = 1`, full circle, must include as `true` through
   `Surface::include(&Curve::Circle(circle))`. On the buggy tree this returns
   `false` (or `NumericallyUnresolved` through whichever reachable arm); after
   the fix it must be `true`. If the reachable `Surface` arm for the plane
   routes the circle through the NURBS conversion, this exercises the fix end
   to end. Document in `notes` exactly which arm it hit and whether the
   cylinder path (through any NURBS-carrier include that reaches circles) also
   returns `true`.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N`-shaped literal unless that line ends with an `// H-3` comment. This
packet's literals are `1.0`/`2.0`/`0.5` (no match); `2.0 * PI` is fine. Run
`bash scripts/kernel-gates.sh <your base commit>` yourself before writing
`RESULT.json`.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-geometry
cargo clippy -p truck-geometry --all-targets --no-deps
cargo test -p truck-geometry --lib --tests --no-fail-fast
cargo check --workspace --all-targets
bash scripts/kernel-gates.sh <your base commit>
```

Never run a bare `cargo test`.

## Forbidden

Editing any file outside `write_allow`. Substituting `unit_z()` (or any other
axis) for a degenerate axis image. Turning a full-circle include that should
be `true` into a `NumericallyUnresolved` refusal (the audit requires the
containment question to be answered correctly; if the refusal is genuinely the
only honest answer for a path, say so in `disagreements` with the precise
reason). Adding `#[ignore]`. Weakening an existing test.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- `Matrix4::from_nonuniform_scale` does not exist in the crate's `cgmath64` →
  `SPEC_GAP`, with the exact constructor that does
- the two-half-circle join cannot be expressed against the real `NurbsCurve` /
  `BSplineCurve` concat API → `SPEC_GAP`, with the exact mismatch
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`fix(geometry): place non-conformal revolved transforms; piecewise full-circle NURBS (BG-AUD-FIX-006)`.
