# WORK PACKET FSSI-004-RULED — ruled × ruled exact analytic locus (the BG-ANA-002 family)

Theory §9.3 ([`FSSI_BUILD_SPEC.md`](../../docs/FSSI_BUILD_SPEC.md) §3,
packet 4): the smallest solver, the largest leverage, and the only FSSI
reduction with **no preconditioner and no interval wrapping term**. Exact
predicates in the landed analytic family — consumed through the EXISTING
transverse-`Curve` path in the splitter. No splitter changes.

```yaml
id:          FSSI-004-RULED
contract:    [FSSI-004-RULED]
class:       mechanical+
crates:      [truck-evidence, truck-certified]
depends_on:  [FSSI-000-CONTRACT]
write_allow:
  - vendor/truck/truck-evidence/src/analytic/ruled_pair.rs
  - vendor/truck/truck-evidence/src/analytic/mod.rs
  - vendor/truck/truck-certified/src/pair_dispatch.rs
  - vendor/truck/truck-evidence/tests/ruled_pair_conformance.rs
read_allow:
  - docs/FSSI_BUILD_SPEC.md
  - vendor/truck/truck-evidence/src/
  - vendor/truck/truck-certified/src/pair_dispatch.rs
  - vendor/truck/truck-shapeops/src/boolean/split.rs
tests_required:
  - two_linear_extrusions_cross_dyadic_exact
  - parallel_generators_refuse_typed
  - clipped_generator_domain_crossing_events_certified
  - v5_analytic_pairs_bit_identical
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'deny(clippy::unwrap_used)' vendor/truck/truck-evidence/src/analytic/ruled_pair.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn ruled_pair_locus' vendor/truck/truck-evidence/src/analytic/ruled_pair.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'ruled_pair' vendor/truck/truck-certified/src/pair_dispatch.rs"}
budget:      {turns: 70, ctx_tokens: 180000}
```

## Pre-digested context

- **The reduction (theory §9.3).** For `X(u,t) = A(t) + u·d(t)` and
  `Y(v,s) = B(s) + v·e(s)` on `d × e ≠ 0`: intersection of the generator
  lines is the exact scalar predicate
  `λ(t,s) = (B(s) − A(t))·(d(t) × e(s)) = 0`, with rational recovery
  `u = det[B−A, e, d×e]/‖d×e‖²`, `v = det[B−A, d, d×e]/‖d×e‖²`.
- **Revision-2 corrections are scope, not commentary:** (1) `λ = 0`
  characterizes the INFINITE generator lines — the surface intersection
  additionally requires the domain-crossing event curves
  (`u − u_min = 0`, …) in the `(t,s)` plane; (2) the parallel-generator
  locus `d(t) × e(s) = 0` must be EXCISED — `u,v` blow up in a
  neighborhood, so it gets its own enclosure test and a typed refusal.
- **Events are `{λ, λ_s}` in two variables**; fixed-`t` continuation is a
  scalar root solve under the Bernstein/Descartes discipline
  (`truck-evidence/src/num/roots.rs`: every returned interval contains
  exactly one root; multiple roots refuse `NumericallyUnresolved` — the
  exact shape the parallel-generator neighborhood needs).
- **Consumption**: emit the landed `AnalyticIntersection::Curve(ExactCurve)`
  locus — transverse ruled crossings ride the existing
  `ContactLocus::Analytic` path in `truck-shapeops/src/boolean/split.rs`,
  which already accepts `Curve` for transverse FF pairs. A DISTINCT
  `RuledCrossing` variant is booked ONLY if a consumer needs to
  distinguish (none does today); adding one ripples the splitter's
  exhaustive match and is out of scope (FSSI-LAYER: splitter writes are a
  stop-and-file, not scope growth).
- **Admission**: ruled recognition lives behind `pair_dispatch.rs` (exact
  `Expansion` admission, D-sorted operands — the BG-ANA-001 house style).
  Carriers eligible: linear extrusions and ruled B-spline spans recognized
  exactly; anything not exactly-ruled is NOT this family's input (the
  general path keeps it).

## Scope decisions

1. **Analytic preservation**: `λ`, its partials, and the `u,v` recovery are
   exact-`Expansion`/interval predicates — never float (the TORUS doc's
   doctrine, generalized). SFC applies to the search over `(t,s)` only.
2. **Parallel-generator excision** refuses typed
   (`Refusal::NumericallyUnresolved` with the landed witness vocabulary —
   no new witness variant; FSSI-REFUSAL). The enclosure test is a
   two-equation two-unknown isolation, generically isolated points.
3. **Domain-crossing events** are certified as part of the locus: a
   crossing whose `u,v` recovery exits the generator domain terminates the
   curve at a certified boundary event, not at a clipped float.
4. **V5-analytic identity**: all landed BG-ANA-001 pair answers stay
   bit-identical. This family only ADDS recognized pairs (monotone
   widening; a red landed analytic test is reporting the bug being fixed —
   but there should be none, since recognition is conjunctive).

H-1: `#![deny(clippy::unwrap_used)]` on the new module. H-3: constants
named. H-6: floats never in evidence. F1: locus lives in `truck-evidence`
(no cross-layer edge; `pair_dispatch.rs` registration is the landed seam).

## Done when

```
cargo fmt --check -p truck-evidence -p truck-certified
cargo clippy -p truck-evidence -p truck-certified --all-targets -- -D warnings
cargo test -p truck-evidence --lib --tests
cargo test -p truck-certified --lib
```

with the four named tests green and the anchors holding.

## Forbidden

`truck-shapeops` writes. New `AnalyticIntersection` variants (see scope 3's
booking note). Float predicates anywhere in the locus or recovery. Catch-all
refusal arms.

## Stop conditions

- A target pair is ruled but its carriers are not exactly-recognizable
  ruled spans with the landed substrate → SPEC_GAP naming the carrier
  (do not approximate a ruling).
- The `{λ, λ_s}` event system needs a 2-D Krawczyk beyond the landed
  roots.rs discipline → stop-and-file (that machinery belongs to the
  FSSI-002 wave, not this packet).

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `feat(evidence): ruled x ruled exact analytic locus — scalar predicate, generator excision, domain-crossing events (FSSI-004)`.
