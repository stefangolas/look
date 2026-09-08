# TORUS-CONTACT PROGRAM — booking doc (2026-09-07, owner-approved direction)

The torus carrier is the last analytic family without contact solvers in
the funnel. Current state and the booked path:

## What is already landed

- Torus PRIMITIVES (S3) construct; the formal substrate certifies the
  regular ring torus with verified rank-2 deck arithmetic
  (`truck-certified/src/formal/torus.rs` — spindle/horn refuse typed).
- **Torus×plane contact pairs certify** under healthy budgets
  (`truck-evidence/tests/torus_pairs.rs`: axial, oblique, offset
  mixed-quadric rows green).
- Torus is EXCLUDED from the Theorem-4 implicit reduction (the implicit
  form is quartic → composition would be bidegree (4p,4q)) and from the
  seed-ray crossing forms (`classify.rs::surface_ray_crossings` has no
  torus arm — the `require_canonical_carriers` gate refuses torus-bearing
  solids before any casting).

## The corpus exposure

The wheels/brakes rows (`cut(revolved,canonical)`, cell 4) hit torus
carriers where a revolved CIRCLE forms the carrier surface. PB-011B
records these as typed refusals (`ContactReductionDeferred`) — the census
rows those refusals create are this program's booking evidence.

## Booked packet family (entry conditions: the PB-011 census shows the
## torus refusal count justifies it)

1. **TOR-A — torus×canonical exact degeneracy + implicit reduction
   extension.** The quartic h = g∘S on the spline chart at bidegree
   (4p,4q): the composition is exact (torus implicit is polynomial
   degree 4), the cost is the concern — measure before committing. The
   cheaper first step: torus×plane and torus×quadric pairs through the
   landed pair machinery (torus_pairs exists) routed into the funnel.
2. **TOR-B — seed-ray torus crossings.** The ray-torus quartic via the
   landed three-state root isolation (Bernstein/Krawczyk — no new
   solver; the same discipline as bezier_isect applied to the quartic in
   t). Unlocks torus-bearing solids in the boolean classify stage.
3. **TOR-C — torus×spline.** The 4-D deep end extension; sequenced
   after the ssi4 corpus exposure from PB-011C (its stagnation census
   sizes this).

## Doctrine constraints

- Only the regular ring torus certifies (spindle/horn refuse) — carry
  that forward at every stage.
- Analytic preservation: torus×torus coaxial/degenerate positions are
  exact predicates (BG-ANA class), never float.
- The frozen exclusions lift by EXTENDING the dispatch tables, never by
  widening a verdict.

## Status

Booked (this doc). No packets authored. Entry: after PB-011B's census
counts the torus refusals on the corpus rows.
