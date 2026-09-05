# WORK PACKET BIE-005-ARRANGE — chart curves and (s,v) arrangement

You are implementing the chart-arrangement layer of the Certified Interaction
Engine (BIE) program. Everything you need is in this document and
`docs/BIE_BUILD_SPINE.md`. Do not read other spec files. If something you
need is genuinely missing, that is a SPEC_GAP (see "Stop conditions"): you
stop and report, you do not research it.

```yaml
id:          BIE-005-ARRANGE
contract:    [BIE-005-ARRANGE]
class:       design
crates:      [truck-geometry]
depends_on:  [BIE-002-SSI4, BIE-003-CARRIER]
write_allow:
  - vendor/truck/truck-geometry/src/arrange.rs
read_allow:
  - vendor/truck/truck-geometry/src/constructive/intersection_carrier.rs
  - vendor/truck/truck-geometry/src/constructive/sweep_surface.rs
  - vendor/truck/truck-geometry/src/canonical.rs
  - docs/BIE_BUILD_SPINE.md
tests_required:
  - chart_carrier_constructs_and_refuses
  - crossings_certified_on_known_figure
  - containment_matches_region_semantics
  - pcurve_simplicity_oracle_holds
budget:      {turns: 70, ctx_tokens: 180000}
```

## Problem

The split stage trims faces in the surface's own (s,v) chart. For sweep
faces the chart is planar — the landed planar arrangement machinery is
reusable — but its curve carrier is lines and circles only. This packet
adds the chart-curve carrier (certified PL/B-spline 2-D curves) to
`Carrier2D`, the FF-arc and containment equivalents in the chart, and
certified inter-curve crossings — so the split stage can trim sweep faces
from `CertifiedChartCurve` inputs projected into (s,v).

## Scope decisions — pre-made, do not relitigate

1. **All edits are ADDITIVE to `arrange.rs`** (spine §5: you share the
   crate with nothing else in your wave; the file is yours alone this
   wave). The landed `Arrangement`/`ArrRegion`/`ArrHalfEdge` machinery and
   `pub fn arrange` are NOT changed semantically — the V5 identity guard
   applies to every landed test.
2. **The booking's "Region2" is the landed `ArrRegion`** (spine §2 drift
   record — there is no `Region2` type in the tree). Containment in the
   chart reuses `ArrRegion` semantics; do not invent a parallel type.
3. **The chart carrier extends `enum Carrier2D`** (the `Line |
   CircleCarrier` enum) with a certified-PL variant carrying the 2-D
   projection of BIE-003's `CertifiedImplicitIntersectionCurve` /
   BIE-002's `CertifiedChartCurve` samples. truck-geometry does not depend
   on truck-certified: accept the certified samples as data (a plain
   `Vec<(f64, f64)>` + certificate flag decided by the constructor's
   refusing signature), the same pattern the carrier packet used.
4. **Crossings are certified**: two chart curves cross iff the certified
   sign test says so (reuse the landed exact-sign approach — dyadic-exact
   where the landed `arrange` is dyadic; a crossing the predicates cannot
   certify is a typed refusal, never a guess).
5. **Lemma F is a test oracle, not code**: the per-pair pcurve simplicity
   (theory §7.1) is asserted metamorphically in tests — each chart curve's
   projection is simple (no self-intersection) on the fixture pairs.

## Anchors — measured 2026-09-05, counts are exact

Locate by pattern, never by line number. If a count differs, STOP and report
`ANCHOR_MISMATCH` with what you saw.

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `vendor/truck/truck-geometry/src/arrange.rs` | `enum Carrier2D` | 1 |
| A2 | `vendor/truck/truck-geometry/src/arrange.rs` | `pub struct ArrRegion` | 1 |
| A3 | `vendor/truck/truck-geometry/src/arrange.rs` | `^pub fn arrange\(profile` | 1 |

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `unimplemented!`, `todo!`, or
  out-of-range indexing reachable from geometry.
- **H-2** Fallible operations return `Outcome<T>`.
- **H-3** No absolute constants in predicates; test epsilons carry `// H-3`
  on the SAME line as the literal. **Dyadic discipline**: fixture crossing
  parameters must be dyadic (the landed `arrange` is dyadic-exact; a
  non-dyadic fixture crossing refuses by design).
- **H-6** Float-computed crossings are certified by the sign test, never
  recorded `Method::Exact`.
- **Determinism** (spine §8): identical ordered input → identical output;
  crossing order is by curve index, then parameter order, always.
- **All cargo invocations go through the queue (the `cargo` on PATH IS the
  queue shim). Do not invoke cargo by absolute path; do not unset the shim.**
- Never run a bare `cargo test` — use the scoped commands below.

## Tests required

Named `#[test]` fns (in-module test section at the end of `arrange.rs`,
following its landed test style) — the verifier checks the names appear in
your diff.

1. `chart_carrier_constructs_and_refuses` — the certified-PL carrier
   constructs from ≥2-point certified samples; refuses degenerate
   (<2 points, non-finite, unclosed where closed is required) input typed.
2. `crossings_certified_on_known_figure` — two chart curves with a dyadic
   crossing (e.g. a PL diagonal across a dyadic rectangle) certify the
   crossing at the known parameter.
3. `containment_matches_region_semantics` — a chart region containment
   answer agrees with the landed `ArrRegion` semantics on a constructed
   arrangement.
4. `pcurve_simplicity_oracle_holds` — the Lemma-F oracle: every fixture
   chart curve is simple (assert no self-crossing via the same certified
   crossing predicate); a deliberately self-crossing control curve FAILS
   the oracle.

No existing test may be deleted, `#[ignore]`d, or weakened.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-geometry
cargo clippy -p truck-geometry --all-targets -- -D warnings
cargo test -p truck-geometry --lib --tests
cargo check -p truck-shapeops -p truck-certified
```

Send cargo output to a file and read the tail.

## Forbidden

Editing any file outside `write_allow` — especially
`constructive/intersection_carrier.rs` (BIE-003's file), `canonical.rs`,
`span.rs`, anything under `truck-shapeops/` or `truck-certified/`,
`scripts/kernel-gates.sh`, `Cargo.lock`. Changing the landed
`Arrangement`/`arrange` semantics. Adding `#[ignore]`. Adding `#[allow]`
without a justification comment on the same line. Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- chart containment genuinely cannot reuse `ArrRegion` semantics →
  `SPEC_GAP`, naming the semantic difference
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

**COMMIT BEFORE writing `RESULT.json`.** Then write `RESULT.json` at the root
of your worktree (not `loop/results/` — the orchestrator files it there).

```json
{"id":"BIE-005-ARRANGE","status":"DONE","contracts":["BIE-005-ARRANGE"],
 "tests_added":4,"anchors_verified":{"A1":1,"A2":1,"A3":1},
 "notes":"the carrier variant shape you landed and how crossings are certified"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(geometry): certified chart-curve carrier + (s,v) arrangement + crossings (BIE-005-ARRANGE)`.
