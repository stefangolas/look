# WORK PACKET PB-002-SKETCH-ARCS — arc + spline authoring over the (s,v) chart

You are implementing the sketch authoring layer of the Python Bridge (PB)
program's Rust client phase. Everything you need is in this document and
`docs/TRUCK123D_PY_BRIDGE_SPEC.md` + `docs/PY_BRIDGE_CONTRACT.md`. If
something you need is genuinely missing, that is a SPEC_GAP (see "Stop
conditions"): you stop and report, you do not research it.

```yaml
id:          PB-002-SKETCH-ARCS
contract:    [PB-002-SKETCH-ARCS]
class:       design
crates:      [truck-geometry]
depends_on:  [PB-000-CONTRACT]
write_allow:
  - vendor/truck/truck-geometry/src/arrange.rs
  - vendor/truck/truck-geometry/src/sketch.rs
read_allow:
  - vendor/truck/truck-geometry/src/arrange.rs
  - vendor/truck/truck-geometry/src/nurbs/bspcurve.rs
  - docs/PY_BRIDGE_CONTRACT.md
  - docs/TRUCK123D_PY_BRIDGE_SPEC.md
tests_required:
  - arc_constructors_refuse_degenerate
  - arc_line_cells_certify_known_crossings
  - spline_authoring_periodic_and_open
  - mixed_loop_profile_assembles
budget:      {turns: 60, ctx_tokens: 150000}
```

**New file** (`sketch.rs`): H-1 applies. All `arrange.rs` edits are
ADDITIVE — the landed `Arrangement`/`arrange` semantics are byte-stable
(V5 identity guard).

## Problem

`Carrier2D` is `Line | CircleCarrier` with trimmed circles representable
(`(c.t0, c.t1)`) — what's missing is the AUTHORING layer: 3-point/radius
arc constructors, Region2 cells for arc×line and arc×arr crossings, and
the spline authoring the text-to-cad corpus idiom needs
(`Spline(*pts, periodic=True)` → make_face → loft is F1's section shape).

## Scope decisions — pre-made, do not relitigate

1. **Arc constructors**: `arc_three_point(p0, p1, p2)` and
   `arc_radius(center, r, a0, a1)` produce trimmed `CircleCarrier`s.
   Collinear/degenerate input refuses typed.
2. **Region2 cells**: arc×line and arc×arc intersections reuse the landed
   analytic circle intersections — this is WIRING into the arrangement's
   `intersect` dispatcher, not new geometry. Crossings the exact dyadic
   predicates cannot certify refuse typed (the landed discipline).
3. **Spline authoring** (amended scope, 2026-09-05): `spline(points,
   periodic)` producing a `Carrier2D`-compatible chart curve — the
   certified-PL path is acceptable if it reuses BIE-005's `ChartCurve`
   machinery (which refuses non-certified input); a direct analytic spline
   carrier is NOT required. State which you landed in RESULT notes.
4. **Mixed loops**: a profile loop mixing lines, arcs, and spline segments
   assembles into a closed wire for the landed extrude/loft paths — the
   endpoint-pairing care of P3 applies (shared endpoints must agree within
   the ctx tolerance; mismatches refuse typed).
5. `arrange.rs` may gain `pub mod`-free additive items only; `sketch.rs`
   holds the constructors. No landed test changes.

## Anchors — measured 2026-09-05, counts are exact

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `vendor/truck/truck-geometry/src/arrange.rs` | `enum Carrier2D` | 1 |
| A2 | `vendor/truck/truck-geometry/src/arrange.rs` | `CircleCarrier` | 8 |
| A3 | `vendor/truck/truck-geometry/src/arrange.rs` | `pub struct ArrRegion` | 1 |
| A4 | `vendor/truck/truck-geometry/src/constructive/sweep_surface.rs` | `pub struct SpineFrameSweep` | 1 |

## House rules

- **H-1** no unwrap/expect/panic reachable from geometry; **H-3** same-line
  `// H-3`; **H-6** never record `Float` as `Exact`. **Dyadic discipline**:
  fixture crossing parameters must be dyadic.
- **All cargo through the queue shim.** Scoped commands only.

## Tests required

1. `arc_constructors_refuse_degenerate` — collinear 3-point, zero radius,
   inverted range refuse typed.
2. `arc_line_cells_certify_known_crossings` — dyadic arc×line and arc×arc
   crossings certify at the known parameters; a non-dyadic crossing
   refuses typed.
3. `spline_authoring_periodic_and_open` — periodic through 6 points closes
   exactly; open splines refuse unclosed loops; non-finite points refuse.
4. `mixed_loop_profile_assembles` — line+arc+spline loop assembles closed;
   the landed `arrange` accepts the profile and yields the expected region
   count on a figure with known ground truth.

No existing test may be deleted, `#[ignore]`d, or weakened.

## Done when

```
cargo fmt --check -p truck-geometry
cargo clippy -p truck-geometry --all-targets -- -D warnings
cargo test -p truck-geometry --lib --tests
cargo check -p truck-shapeops -p truck-certified
```

## Forbidden

Anything outside `write_allow` — especially `canonical.rs`,
`intersection_carrier.rs` (BIE-003's), `span.rs`, landed test files,
`scripts/kernel-gates.sh`, `Cargo.lock`. Changing landed `arrange`
semantics. Adding `#[ignore]`. Unjustified `#[allow]`. Committing to
`main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- arc cells genuinely cannot reuse the landed analytic intersections →
  `SPEC_GAP`, naming the gap
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

```json
{"id":"PB-002-SKETCH-ARCS","status":"DONE","contracts":["PB-002-SKETCH-ARCS"],
 "tests_added":4,"anchors_verified":{"A1":1,"A2":8,"A3":1,"A4":1},
 "notes":"the spline carrier decision (ChartCurve vs analytic) and why"}
```

Commit subject: `feat(geometry): arc + spline sketch authoring over the chart (PB-002-SKETCH-ARCS)`.
