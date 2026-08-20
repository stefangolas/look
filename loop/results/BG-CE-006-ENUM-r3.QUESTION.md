# SPEC_GAP — BG-CE-006-ENUM

Status: `SPEC_GAP` (see `RESULT.json`). The packet's ripple analysis is wrong:
the packet's own mandated changes make two files outside `write_allow` fail to
compile, and one pre-existing test regresses at runtime. Per the stop
conditions I stopped rather than widening my scope.

## 1. `vendor/truck/truck-stepio/src/in/step_geometry/mod.rs` — does not compile

Decision 1 mandates `pub use canonical::*;` in `truck_geometry::prelude`.
`truck_stepio::in::step_geometry` glob-re-exports that prelude
(`pub use re_exports::*;`, line 11) while also defining its own
`pub enum Surface` (line 437). Inside `impl DisplayByStep for Surface`,
`use Surface::*;` (line 447) then resolves ambiguously:

```
error[E0659]: `Surface` is ambiguous
error[E0532]: expected tuple struct or tuple variant, found enum `ElementarySurface`
error[E0532]: expected tuple struct or tuple variant, found enum `SweptCurve`
error[E0531]: cannot find tuple struct or tuple variant `OffsetSurface` in this scope
error[E0308]: mismatched types (x2)
```

The `in` module was unaffected before because the prelude did not export any
`Surface`. There is no in-scope fix: the collision lives in `truck-stepio/src/in/`,
which is not writable, and decision 1 (the prelude re-export) is mandatory.

## 2. `vendor/truck/truck-meshalgo/tests/tessellation/triangulation.rs` — does not compile

Line 103 constructs `let surface: Surface = Processor::new(surface_row).into();`
with `surface_row: RevolutedCurve<Curve>`. It relies on the derived
`From<Processor<RevolutedCurve<Curve>, Matrix4>> for Surface`, which decision 6
removes (the `RevolutedCurve` payload is now the bare `RevolutedCurve<Curve>`):

```
error[E0277]: the trait bound `Surface: From<Processor<RevolutedCurve<Curve>, _>>` is not satisfied
```

## 3. `truck-modeling::builder::partial_torus` regresses at runtime

The `RevolutedCurve<Curve>` payload now carries a periodic `Curve::Circle`
profile (the sweep path no longer degrades circle arcs to NURBS), so
`RevolutedCurve::search_parameter` returns branch-inconsistent `u` values
(observed `-10·π` and `11·π` for the same face) and
`test_boundary_orientation`'s `area > 0` assertion fails. The fix belongs in
`decorators/revolved_curve.rs` (branch-normalized periodic search) or by
degrading the swept profile — both outside `write_allow`.

## Questions the packet should answer

- How should `truck_geometry::prelude` re-export canonical's `Curve`/`Surface`
  without colliding with `truck_stepio::in::step_geometry::Surface`? (Options:
  narrow the re-export, qualify stepio's in-import, or widen `write_allow` to
  `truck-stepio/src/in/`.)
- Who fixes `truck-meshalgo/tests/tessellation/triangulation.rs:103` and the
  `partial_torus` regression — is that a widening of `write_allow` here, or a
  follow-up packet?
- Also recorded in `RESULT.json` (disagreements): the four analytic specifieds
  do not implement `Invertible` (decision 2's claim is false) and
  `Transformed<Matrix4>` was never addressed for them or for bare
  `RevolutedCurve<C>`; I added in-canonical impls.
