# STEP IMPORT FRONTIER RESULTS

## Provenance

| Item | Value |
| --- | --- |
| Truck before | `09726a9e20c3ddb6cb09ec82bd2fbd24d3ab7cfc` (pin at session start) |
| Truck F1-A | `b926c48749d3597e57d8db85d0923647bcf786d8` |
| Truck CX-A | `ab3fac4facb50f04954cab4bbae6bc1005250656` |
| Truck CX-B | `c5f4b6e9778e0721a1d446f10568eb5e5594e8ed` |
| Look F1-A repin | `c920439` |
| Look CX-A repin | `d814074` |
| Look CX-B repin | `9c5d5de` |

All truck commits are pushed to `origin/feature/cone-apex-lift-recovery` of the
`stefangolas/truck` fork; every look repin references the exact SHA through the
`https://github.com/stefangolas/truck` URL, so fresh checkouts resolve.

## Final accounting (release build, physical GPU)

```
                     BEFORE       AFTER

formula1
SurfaceConversion      142           0
total missing          142           0
rendered              5093/5235   5235/5235
triangles            365,624      374,653   (+9,029 = 142 recovered faces)

core_xy
EdgeCurveConversion     12           0
AllBoundsCollapsed       1           0
total missing           13           0
rendered              5657/5670   5670/5670
triangles            406,096      406,930   (+834 = 13 recovered faces)
```

`before` figures are the authoritative pre-session state (the task handoff and
the `scratch/unseen_audit` captures). `after` figures are from the release
build on this machine (NVIDIA GeForce RTX 5050 Laptop GPU, DX12).

No historical tessellation failure population was reintroduced: the release
`core_xy` and `formula1` runs emit zero conversion records and zero
tessellation failures (`BoundaryProjectionFailed`, `EvaluatorOutOfDomain`,
`SingularEvaluation` all absent). The old `33 × EvaluatorOutOfDomain`,
`1 × SingularEvaluation` records in `scratch/core_xy_diag.jsonl` belong to the
superseded diagnostic artifact and were deliberately not worked on.

## Mechanism 1 — F1-A: DEGENERATE_TOROIDAL_SURFACE (142 → 0)

- **Exact STEP construct:** `DEGENERATE_TOROIDAL_SURFACE('', #placement,
  major_radius, minor_radius, select_outer)`, an AP242 subtype of
  `toroidal_surface` with EXPRESS `WHERE major_radius < minor_radius`. The
  file contains exactly 142 of them: `select_outer` TRUE ×138, FALSE ×4, in
  six `(R, r)` families `(0.063, 0.1) ×128`, `(0.1303, 0.2) ×4`,
  `(0.1287, 0.2) ×4`, `(0.665, 1.0) ×2` (all FALSE), `(1.165, 1.5) ×2` (all
  FALSE), `(0.003, 0.005) ×2`. Each corresponds 1:1 to one lost face.
- **Rust conversion symbol:** `Table::push_instance` record dispatch
  (`"DEGENERATE_TOROIDAL_SURFACE"` arm), record struct
  `in::DegenerateToroidalSurface`, `TryFrom<&DegenerateToroidalSurface> for
  step_geometry::DegenerateToroidalSurface`, carrier
  `step_geometry::DegenerateTorus`, alias
  `step_geometry::DegenerateToroidalSurface = Processor<DegenerateTorus,
  Matrix4>`, `ElementarySurface::DegenerateToroidalSurface` variant,
  `TryFrom<&ElementarySurfaceAny> for ElementarySurface` (was infallible
  `From`).
- **Source semantics / theorem:** a `degenerate_toroidal_surface` is a spindle
  torus (`R < r`); the parametrisation is self-intersecting and the face must
  name one sheet via `select_outer`. With `cos φ = -R/r`, the outer sheet is
  `u ∈ [0, 2π], v ∈ [-φ, φ]` and the inner sheet is
  `u ∈ [0, 2π], v ∈ [φ, 2π - φ]`. `DegenerateTorus` wraps the existing
  `Torus` carrier, restricts `parameter_range` to the source-defined sheet,
  reports `u_period = 2π` / `v_period = None`, and provides sheet-aware
  closed-form `search_parameter` inverses so boundary lifting never mixes the
  two sheets. This is the exact source-defined lowering, not a plain torus
  approximation: a full-`[0,2π]×[0,2π]` torus would include the fold.
- **Refusals preserved:** non-positive or non-finite radii and
  `major_radius >= minor_radius` (WHERE violation) refuse conversion with a
  typed `StepConvertingError` → `SurfaceConversionFailed`; genuinely missing
  references still refuse. Orientation for `select_outer`/`same_sense` uses
  the existing `Processor` orientation machinery.
- **Focused tests:** `truck-stepio` unit tests
  `degenerate_torus::tests::{the_sheet_domain_is_the_source_interval,
  the_sheet_inverse_round_trips_on_sheet_points, an_off_sheet_point_is_refused,
  invalid_radii_are_refused}` and input tests
  `degenerate_toroidal_surface_outer_sheet`, `_inner_sheet`,
  `_refuses_invalid_radii`.
- **Witnesses:** faces `#76853` (shell `#81967`) and `#76735` (shell
  `#81966`) convert and reach tessellation; full Formula 1 gate 142 → 0 with
  zero diagnostic records.

## Mechanism 2 — CX-A: tiny-range b-spline edge curves (12 → 0)

- **Exact STEP construct:** six `B_SPLINE_CURVE_WITH_KNOTS` entities
  (`#2437, #2441, #2451, #2455, #2669, #2680`) whose knot intervals are
  nonzero but below truck's absolute `TOLERANCE = 1e-6`
  (`~2.4e-8`..`6.4e-7`). Each edge is shared by two faces → the 12 lost
  faces `96104, 96107, 96146, 96147, 96149, 96150, 96157, 96159, 97134,
  97139, 97147, 97149`.
- **Rust conversion symbol:** `TryFrom<&BSplineCurveWithKnots> for
  BSplineCurve<P>` in `truck-stepio/src/in/mod.rs`, which previously built the
  knot vector and let `BSplineCurve::try_new` refuse it with
  `Error::ZeroRange` ("This knot vector consists single value.").
- **Source semantics / theorem:** the source does **not** contain a one-value
  knot vector — the two knots are distinct. It is a valid B-spline over a tiny
  parameter span. The canonical geometric interpretation is the curve itself,
  which is preserved exactly by normalizing the knot vector to `[0, 1]` (a
  linear, shape-preserving reparameterization). `ValidatedKnotVector::validate`
  already proves the active domain is nonzero (refuses `≤ 1e-12`), so the
  normalize branch never fires on a truly degenerate source. The knot vector
  is normalized only when `range_length().so_small()`, exactly the refusal
  threshold of `try_new`.
- **Refusals preserved:** unsorted (decreasing) knots still refuse;
  `ValidatedKnotVector` failures other than `UnsortedRawKnots` keep the
  existing quasi-uniform fallback.
- **Focused tests:** `b_spline_curve_with_knots_tiny_knot_interval_converts`
  (converts and matches the analytic cubic Bezier over the whole domain) and
  `b_spline_curve_with_knots_unsorted_knots_still_refuse`.
- **Witness:** the 12 known faces all recover; full Core XY gate
  `EdgeCurveConversionFailed` 12 → 0.

## Mechanism 3 — CX-B: full closed surface trimmed by a vertex loop (1 → 0)

- **Exact STEP construct:** `ADVANCED_FACE #94753` on `SPHERICAL_SURFACE
  #125` (r = 0.005) whose single bound `#84710` is a `VERTEX_LOOP` at the
  sphere's pole — the ball definition in shell `#97529`. One source bound,
  zero source edge uses, no edge conversion failure: the vertex loop converts
  to `BoundOutcome::Collapsed`, leaving no wire, and the face was refused as
  `AllBoundsCollapsed`.
- **Rust conversion symbols:** `Table::shell_faces` (truck-stepio) — the
  `wires.is_empty()` refusal now exempts `closed_surface_accepts_untrimmed`
  (`ElementarySurface::Sphere`), producing a face with empty boundaries —
  and `working_range` (truck-meshalgo `triangulation.rs`), which now returns
  the surface's declared domain when a face has no source boundary pieces so
  the existing synthetic full-domain closure rectangle is built.
- **Source semantics / theorem:** a `VERTEX_LOOP` (or a face declaring no
  bound at all) trims nothing, so on a **closed** surface the whole surface is
  the face. That is the ball: a full sphere trimmed by a pole vertex loop.
  The refusal stands for every other carrier (a plane, an open cylinder would
  mesh an invented region), and the `!had_source_pieces` closure-rectangle
  path was already the designed mechanism for "a face with no enclosing loop
  takes its domain from the surface" — the missing piece was only that an
  empty boundary must not report `None` for the non-periodic axis.
- **Refusals preserved:** vertex-loop-only faces on non-closed surfaces still
  refuse `AllBoundsCollapsed`; the "accept untrimmed carrier" fallback was not
  generalized.
- **Focused tests:** `vertex_loop_only_sphere_face_is_a_full_closed_surface`
  (converts, one face, empty boundaries, zero losses) and
  `vertex_loop_only_plane_face_still_refuses` (still `AllBoundsCollapsed`).
- **Witness:** face `#94753` meshes as the full ball; full Core XY gate
  `AllBoundsCollapsed` 1 → 0, zero diagnostic records.

## Regression validation (release build)

- **`step_corpus.py` run** (36 files: 33 NIST + `core_xy`, `formula1`,
  `ur10`): 36/36 `ok`. Compared against
  `scratch/corpus_regression/step_corpus_baseline.json`:
  - all 33 NIST files have **byte-identical triangle counts** to the baseline
    (no rendered face changed on NIST);
  - `ur10.step`: baseline `timeout` → `ok`;
  - `core_xy.step` / `formula1.step`: triangle counts increased by exactly the
    recovered faces (see accounting).
- The `core_xy` baseline triangle count (668,351) is from a much older pin
  that predates the current tessellation architecture; the pre-session state
  of this frontier (13 missing faces, `core_xy_audit.stderr`) is what the
  changes moved to 0, and every change in this session strictly **added**
  faces. No previously rendered face was lost by any of the three mechanisms.
- Look test suite (`cargo qtest`): 229 passed, 0 failed.
- Truck-stepio: all pre-existing tests plus the new focused tests pass; the 6
  `input` tests and the `builder` lib test that fail do so identically at the
  pre-session branch HEAD (recorded, unrelated to this work).
- `cargo fmt --all -- --check` clean for every file touched by this work.

## Notes

- The truck-stepio `input`/`builder` test failures predate this session and are
  unchanged by it (verified by stashing the working tree and re-running).
- The formula1 `torus observer` census is byte-identical before and after all
  three mechanisms (538 attempted / 382 not eligible / 152 recovered clean /
  4 inconsistent boundary homology) — no tessellation behavior changed.
