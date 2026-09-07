# WORK PACKET DEF-SEEDRAY-C — the certified avoidance criterion: an AUDIT-FIRST survey

The one genuinely researchy piece of the seed-ray fix: a certified
edge/vertex avoidance criterion for the seed ray. This packet does NOT
implement it. It produces a design document + measured probe evidence so
an external frontier agent can audit the plan before any implementation
packet is booked.

```yaml
id:          DEF-SEEDRAY-C
contract:    [DEF-SEEDRAY-C]
class:       survey
crates:      [truck-shapeops]
depends_on:  []
write_allow:
  - docs/AUDIT_SEEDRAY_AVOIDANCE.md
  - vendor/truck/truck-shapeops/tests/seedray_avoidance_probe.rs
read_allow:
  - vendor/truck/truck-shapeops/src/boolean/classify.rs
  - vendor/truck/truck-evidence/src/
  - docs/AUDIT_MATH_CODE_CORRESPONDENCE.md
  - docs/FORMAL_SYSTEM_BREP_GENERATION.md
tests_required:
  - the probe compiles and its #[ignore] measurements run on demand,
    printing the census tables the document cites
anchors:
  - {id: A1, expect: 1, cmd: "grep -c '3.0_f64' vendor/truck/truck-shapeops/src/boolean/classify.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'fn find_seed' vendor/truck/truck-shapeops/src/boolean/classify.rs"}
budget:      {turns: 50, ctx_tokens: 160000}
```

## Problem

The seed-parity theorem needs `dist(ray, edges/vertices of ∂B) ≥ δ > 0`
certified along the WHOLE ray. Nothing in the tree measures this today:
the only guard is float `classify_region → Boundary → ambiguous` at found
crossings, and the 14-direction table is symbolic perturbation by retry.
The adjudicated parity analysis concentrates the wrong-answer risk here
(the through-edge double count is odd) and in dropped crossings — but
avoidance is the part with NO landed machinery to adapt, which is why it
is audited before it is built.

Severity context (record in the document): a wrong seed bit propagates
across an ENTIRE component (§12), and `ContradictoryDualParity`
(classify.rs:78) catches only INCONSISTENT components — a uniformly wrong
one passes clean. Also note the corpus-correlation with
DEF-TESS-ANALYTIC-SEAM: both surface as failures on analytic geometry.

## The document must contain (docs/AUDIT_SEEDRAY_AVOIDANCE.md)

1. **Candidate criteria, each with its soundness argument stated as a
   theorem sketch** (hypotheses explicit):
   a. Per-edge subdivision + hull exclusion: for each boundary curve of
      ∂B, subdivide its parameter domain; on each sub-box, an interval
      hull of the curve (Bernstein hull discipline) vs the ray line gives
      either "distance ≥ δ" (excluded) or subdivision continues;
      termination by compactness + width convergence; the ray's own
      interval segment is the second operand. Cost: O(edges × subdivision
      depth). State the distance formulation precisely (point-to-ray vs
      line — the ray is a HALF-line; the segment beyond t_exit matters).
   b. Cross-product sign discipline: for each edge curve C(t) and the
      ray, the avoidance condition is sign-stability of
      (C(t) − p) × d over the edge's parameter box — decide by interval
      evaluation per sub-box; a sign change = potential near-pass =
      subdivide or refuse. (Cheaper: no square roots.)
   c. Slab/exclusion formulation: partition the ray's t-range; per slab,
      certify the ray point's interval box is disjoint from every edge's
      hull box. (Cheapest per step; more slabs.)
   For each: what the certificate RECORD looks like (so the verdict is
   unconstructible without it — the cross-cutting doctrine), the refusal
   path, and the budget interaction with the 14-direction retry.
2. **Vertex avoidance** (trivial: interval point-to-line distance) —
   spell it out anyway; it is the cheap half.
3. **MEASURED probe evidence** (the #[ignore] probe):
   - census over the existing boolean test fixtures: how many seeds hit
     near-edge configurations per direction; the direction-table hit
     profile;
   - for criterion (b) on Line/Circle edges (the analytically decidable
     subset): measured refuse-rate on the corpus fixtures — this feeds
     the doctrine-floor question (how many formerly-green seeds would
     the certified path refuse);
   - the measured runtime overhead per seed decision.
4. **The calibration question, stated honestly**: the interaction between
   refusal rate and the doctrine floor
   ("fail-closed is not passable by refusing everything") — with the
   census numbers, recommend: which criterion, which δ formulation
   (absolute vs scale-relative, H-3), and whether avoidance failure
   should refuse the DIRECTION (retry next of 14) or the SEED
   (NumericallyUnresolved).
5. **Open questions for the external auditor**, explicitly listed.

## Done when

```
cargo test -p truck-shapeops --test seedray_avoidance_probe -- --ignored --nocapture
```

prints the cited census tables; the document's numbers match the probe.

## Forbidden

Implementing the criterion in production code (classify.rs is
DEF-SEEDRAY-B's). Changing any verdict. Hand-waving a soundness argument
— every criterion entry either carries a proof sketch with hypotheses or
is marked OPEN.

## Stop conditions

- A criterion's soundness sketch cannot be completed without new theory
  → mark OPEN in the document (that is a deliverable, not a failure).

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `docs: seed-ray avoidance criterion audit — candidate theorems, probe census, calibration recommendation (DEF-SEEDRAY-C)`.
