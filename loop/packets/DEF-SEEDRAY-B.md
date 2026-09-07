# WORK PACKET DEF-SEEDRAY-B — certified trimming first, then the seed-parity assembly

The seed bit is a theorem only if EVERY stage is certified. Adjudicated
priority: certified TRIMMING outranks transversality, because a tangential
root pair straddling the trim boundary splits into an ODD count (the
parity killer), while plain quadratic discriminant errors are even and
parity-invariant. This packet wires DEF-SEEDRAY-A's certified crossings
into a certified seed bit, with the region-containment stage certified.

```yaml
id:          DEF-SEEDRAY-B
contract:    [DEF-SEEDRAY-B]
class:       design
crates:      [truck-shapeops]
depends_on:  [DEF-SEEDRAY-A]
write_allow:
  - vendor/truck/truck-shapeops/src/boolean/classify.rs
  - vendor/truck/truck-shapeops/src/boolean/ray_cert.rs
  - vendor/truck/truck-shapeops/src/boolean/mod.rs
read_allow:
  - vendor/truck/truck-shapeops/src/boolean/
  - vendor/truck/truck-certified/src/kernel/trimclip.rs
  - vendor/truck/truck-evidence/src/
  - docs/WAVE_4A_WINDING_PARITY.md
tests_required:
  - certified region containment agrees with the float classify_region on
    an interior fixture battery, and returns typed Inconclusive on the
    boundary band instead of a tolerance guess
  - the through-edge fixture (ray through two meeting faces) refuses
    instead of double-counting
  - the dropped-crossing fixture (Newton-non-convergent projection)
    refuses instead of skipping
  - end-to-end: ray_seed's verdicts unchanged on all existing green
    boolean tests (no new refusals on non-grazing fixtures beyond a
    recorded census)
anchors:
  - {id: A1, expect: 5, cmd: "grep -c 'SEARCH_TRIALS' vendor/truck/truck-shapeops/src/boolean/classify.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'fn ray_seed' vendor/truck/truck-shapeops/src/boolean/classify.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'fn require_canonical_carriers' vendor/truck/truck-shapeops/src/boolean/classify.rs"}
budget:      {turns: 70, ctx_tokens: 180000}
```

H-1: no panics. H-3: every constant named on its defining line. NEW
MODULES carry `#![deny(clippy::unwrap_used)]` (ray_cert.rs is created by
DEF-SEEDRAY-A under that deny; this packet must not weaken it). H-6: never
record Float as Exact.

## Problem — three uncertified stages between a certified crossing and the bit

1. **Region containment is float.** `classify_region(face, (u,v), tol)`
   decides Inside/Boundary/Outside with tolerance comparisons (and the
   crossing's (u,v) itself comes from float `search_parameter` +
   `near_pt`). A certified crossing in an uncertified region is an
   uncertified bit — the defect moves into the pcurve trim loops. The
   trim-straddle case makes it ODD: a near-tangential pair with one root
   Inside and one Outside splits, flipping parity.
2. **Crossings can be silently dropped**: the crossing-to-(u,v) projection
   `let Some(uv) = ... else { continue }` skips a real crossing whose
   float Newton fails — an odd corruption.
3. **The through-edge double count**: two faces meeting at an edge both
   classify Inside for a ray through the edge — two counted, one true.
   There is no certified edge/vertex avoidance (that criterion is
   DEF-SEEDRAY-C's audit; this packet makes the failure VISIBLE instead
   of silent: a crossing pair whose t-intervals overlap refuses rather
   than double-counts).

## Scope decisions — ordered

1. **Certified (u,v) inversion**: from a certified t-interval, the point
   q = p + t·d is an interval point; invert to the carrier's parameter
   box with interval Newton (the machinery class exists in truck-evidence;
   the 2×2 surface system is square). Non-convergence is typed, never a
   skip.
2. **Certified region containment**: adapt the trimclip exact-winding
   discipline (`truck-certified/src/kernel/trimclip.rs` — the exact
   integer ray-crossing count of a closed trim loop about a certified
   query, in the certified Bernstein representation) to the fragment
   faces' parameter polygons. Verdicts: Inside / Outside / typed
   Inconclusive within the certified boundary band. The float
   `classify_region` survives ONLY as the hint layer.
3. **Certified transversality per crossing**: enclosure of d·n_eff over
   the crossing box excluding 0 (hygiene by the parity analysis — pairs
   are even — but it is what makes the trim-straddle case DECIDABLE:
   transversality + certified trimming together decide the split pair).
4. **Crossing separation**: certified t-intervals must be pairwise
   disjoint; overlapping intervals (through-edge signature) refuse typed.
   The t_exit box bound (DEF-SEEDRAY-A helper) discharges completeness:
   crossings beyond the box exit are excluded by construction.
5. **Parity assembly**: winding from certified entering/exiting signs;
   the 14-direction table stays as the SEARCH layer (SFC: the first
   direction whose certificate completes wins); exhaustion of all 14
   returns the EXISTING `Err(numerically_unresolved())` — no new refusal
   arm (the return type already admits failure; consumers already handle
   it).
6. **Census obligation**: RESULT records, over the existing green boolean
   test battery, how many seed decisions changed verdict class (ok-guessed
   → certified-same, ok-guessed → refused). The doctrine floor
   ("fail-closed is not passable by refusing everything") is adjudicated
   on this census.

## Done when

```
cargo test -p truck-shapeops --lib
cargo test -p truck-shapeops --tests
```

with the new fixtures above green and the census recorded.

## Forbidden

vendor/truck outside write_allow. Weakening any landed boolean test.
Removing the 14-direction table or the float hint layer. Silent skips
(any crossing either counts with a certificate or refuses).

## Stop conditions

- The certified (u,v) inversion cannot reuse/adapt landed machinery and
  needs a new solver → SPEC_GAP with the gap named (do not hand-roll).
- The census shows >20% of previously-green seed decisions now refuse →
  STOP and record: the avoidance criterion (DEF-SEEDRAY-C) must land
  before this path can go live.

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `feat(shapeops): certified seed-parity assembly — trimming first, typed degradation, census (DEF-SEEDRAY-B)`.
