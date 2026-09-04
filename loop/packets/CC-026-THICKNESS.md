# CC-026-THICKNESS — conservative certified shell thickness t_safe

CC program Phase C (spine S7 consumer; theory §7.1, with §7.2–7.3 DEFERRED).
`shell(body,t)` needs no critical-parameter theory (§4.4); this packet
serves `max_shell_thickness` with the conservative certified lower bound
t_safe = min(t_focal, d_min/2): focal events from interval H/K, the global
bottleneck from certified non-adjacent stratum distances. No root finding,
no semialgebraic projection.

```yaml
id:          CC-026-THICKNESS
contract:    [CC-026-THICKNESS]
class:       mechanical
crates:      [truck-certified]
depends_on:  [CC-000-CONTRACT, CC-004-CLEAR, CC-021-OFFSET-STRATA]
write_allow:
  - vendor/truck/truck-certified/src/construct/thickness.rs
  - vendor/truck/truck-certified/src/construct/mod.rs
  - vendor/truck/truck-certified/tests/construct_thickness.rs
read_allow:
  - docs/CERTIFIED_CONSTRUCTION_CONTRACTS.md
  - vendor/truck/truck-certified/src/construct
budget:      {turns: 20, ctx_tokens: 90000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub fn reach_bound' vendor/truck/truck-certified/src/construct/offset_strata.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn distance_lower_bound' vendor/truck/truck-base/src/bvh.rs"}
tests_required:
  - unit_sphere_t_safe_is_the_whole_radius
  - thin_plate_t_safe_is_bounded_by_focal_term
  - two_parallel_plates_t_safe_is_bounded_by_half_gap
  - non_adjacent_exclusion_matches_star_glue_plan
  - enclosure_straddle_refuses_no_generic_event
```

Section 1: the focal term — `pub fn t_focal(map: &CertifiedSurfaceMap,
sub: SurfaceRegion) -> Result<Interval, ConstructRefusal>`: per Bézier
patch, bound the focal quadratic 1 − 2Ht + Kt² ≥ `CC_ETA_J` from below.
Pre-made: v1 derives [H] and [K] from the landed map WITHOUT a second-form
module by composing the per-patch first/second-derivative hull enclosures
(the CC-002 path) into the two invariant enclosures — H from the mean of
the principal curvatures is NOT extracted; instead the DIRECT composition
is: bound the quadratic's coefficients by interval evaluation over the
patch's derivative enclosures, then solve the interval quadratic for the
admissible t-set in closed form (coefficient corners, plus the degenerate
0 ∈ [K] case per theory §7.1), intersecting over all patches. If the
coefficient composition cannot be made SOUND from the landed hull kernels,
STOP and file QUESTION.md — that is the booked second-form decision
(deferred §7.2–7.3), not a per-packet derivation.

Section 2: the bottleneck term — `pub fn d_min_over_nonadjacent(strata:
&[OffsetStratum], glue: &GluePlan) -> Result<f64, ConstructRefusal>`:
certified minimum distance between NON-ADJACENT source strata, adjacent
pairs excluded by the glue plan (they are handled by the local star
certificates of theory §4.1 — test 4 pins the exclusion). Distance
lower-bounds via the landed `Bvh::distance_lower_bound` (A2) over the
strata's control boxes; the reach bounds (A1) are NOT subtracted here —
d_min is over SOURCE strata, per theory §7.1.

Section 3: the bound — `pub fn t_safe(map: &CertifiedSurfaceMap, strata:
&[OffsetStratum], glue: &GluePlan) -> Result<Interval, ConstructRefusal>`
returns min(t_focal_lower, d_min/2) as a certified LOWER bound (round the
min DOWN; H-3 opt-outs in tests). Ground truths: the unit sphere's t_safe
covers the full radius (focal at t = 1 via K = 1); a thin plate is
focal-bounded; two parallel plates are bottleneck-bounded at half the gap
(tests 1–3, H-3 opt-outs). An enclosure straddling the minimum →
`Err(NonGenericThicknessEvent)` (test 5). The exact `valid_shell_interval`
(§7.2–7.3, 5×5 systems) is DEFERRED and out of this packet.

House rules: **H-1: no `unwrap`/`expect`/`panic!` in shipped code, no
module-level `allow`.** **H-3: float comparisons in tests take the `// H-3`
opt-out ON THE SAME LINE.** **All cargo invocations go through the queue
(the `cargo` on PATH IS the queue shim). Do not invoke cargo by absolute
path; do not unset the shim.** Scoped checks only: `cargo check -p
truck-certified` and `cargo test -p truck-certified --test
construct_thickness`. No workspace builds. The `pub mod thickness;` line in
`construct/mod.rs` is the DESIGNED one-line conflict. COMMIT BEFORE writing
RESULT.json AT THE WORKTREE ROOT.

Stop conditions: (1) the conservative bound is the v1 PRODUCT — its
conservatism (lower bound of the true max thickness) is a feature; do not
tighten it by heuristic; (2) `GluePlan` is CC-022's type — consume, extend
nothing; (3) record the per-patch count of interval-quadratic solves in
RESULT notes (the O(N) claim made observable).
