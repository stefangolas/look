# CC-024-OFFSET-EXACT â€” sharp and concave completion via the arrangement engine

CC program Phase C (theory Â§3.4, Â§4.1 stars note; open obligation Â§10.2).
Sharp (mitered/extended) edges and corners are produced by the ARRANGEMENT
ENGINE, not the contact system: extend the adjacent offset faces and
intersect them. Their points are NOT within |t| of their source (a convex
corner of dihedral half-angle Î¸ yields |t|/sin Î¸), so each mitered stratum
carries a computed reach bound Ï_A â€” never the |t| shortcut. Concave edges
under inward offsets are trims by the same engine.

```yaml
id:          CC-024-OFFSET-EXACT
contract:    [CC-024-OFFSET-EXACT]
class:       design
crates:      [truck-geometry, truck-shapeops]
depends_on:  [CC-021-OFFSET-STRATA]
write_allow:
  - vendor/truck/truck-geometry/src/arrange.rs
  - vendor/truck/truck-shapeops/src/boolean/assemble.rs
  - vendor/truck/truck-shapeops/tests/cc024_offset_exact.rs
read_allow:
  - docs/CERTIFIED_CONSTRUCTION_CONTRACTS.md
  - vendor/truck/truck-geometry/src/arrange.rs
  - vendor/truck/truck-shapeops/src/boolean
budget:      {turns: 24, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub fn arrange(profile: &[Curve], domain: Option<BoundingBox<Point2>>) -> Outcome<Arrangement> {' vendor/truck/truck-geometry/src/arrange.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub enum OffsetStratum' vendor/truck/truck-certified/src/construct/offset_strata.rs"}
tests_required:
  - mitered_wedge_edge_reach_bound_is_t_over_sin_theta
  - concave_edge_trim_discards_covered_cells
  - existing_arrange_behavior_bit_identical_on_its_own_fixtures
  - mitered_stratum_carries_computed_reach_not_t
```

Section 1: the mitered stratum â€” for two adjacent offset faces meeting at a
convex edge, extend both and intersect: the miter line is the extend-and-
intersect rule evaluated on the landed face carriers (plane faces give an
exact line; curved faces route through the landed certified pair machinery).
`pub fn mitered_edge_reach(dihedral_half_angle: f64, t: f64) -> f64` pins
the |t|/sin Î¸ bound as code; `pub struct MiteredStratum` carries the
constructed miter geometry plus its computed reach (test 4: the bound is
the computed value, STRICTLY greater than |t| for Î¸ < Ï€/2 â€” H-3 opt-outs).
This is the theory Â§3.4 interface: sharp strata differ from ball strata
ONLY in the stratum-generation rule and the reach bound.

Section 2: concave trims â€” the arrangement engine's existing `arrange` (A1)
gains a production path that marks the cells covered by the overlapping
adjacent offset face and discards them (theory Â§3.4 concave completion; the
same mechanism CC-032 uses for face consumption). Output: surviving cells +
the trim curves, with provenance through the boolean `StratumRef` convention
already landed in assemble.rs (A2's OffsetStratum is read-only context â€”
the sharp/concave strata are NOT new OffsetStratum variants; they are
arrangement outputs, per theory Â§3.4).

Section 3: the V5 identity gate (`existing_arrange_behavior_bit_identical_
on_its_own_fixtures`): every fixture `arrange` already answers must answer
IDENTICALLY after this packet â€” additive paths only, no signature change,
no reordering. The concave-trim path is a NEW entry point next to `arrange`,
never a behavior change inside it.

House rules: **H-1: no `unwrap`/`expect`/`panic!` in shipped code, no
module-level `allow`.** **H-3: float comparisons in tests take the `// H-3`
opt-out ON THE SAME LINE.** **All cargo invocations go through the queue
(the `cargo` on PATH IS the queue shim). Do not invoke cargo by absolute
path; do not unset the shim.** Scoped checks only: `cargo check -p
truck-geometry`, `cargo check -p truck-shapeops`, `cargo test -p
truck-shapeops --test cc024_offset_exact`. No workspace builds. COMMIT
BEFORE writing RESULT.json AT THE WORKTREE ROOT.

Stop conditions: (1) this packet is the theory's OPEN OBLIGATION Â§10.2
given its first stratum-by-stratum treatment â€” the mitered rule here covers
PLANE-face edges exactly; curved-face miters may need the certified pair
machinery and may partially refuse â€” record what refuses and why in RESULT
notes rather than approximating; (2) rounded strata are CC-021's â€” no ball
strata are built here; (3) if `arrange`'s dyadic-exact v1 cannot express a
trim curve the concave rule needs, file QUESTION.md â€” that is the booked
arrangement upgrade decision, not a per-packet improvisation.
