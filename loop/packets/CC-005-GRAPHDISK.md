# CC-005-GRAPHDISK â€” P3: graph-disk embedding certificate with projection search

CC program Phase A (spine S6). Theory:
`docs/CERTIFIED_LOFT_AND_SHELL_THEORY_SPEC.md` Â§1 P3. P3 certifies
injectivity for a GLUED region â€” a corner patch or a closed offset star â€”
where no single parameterization exists: a projection w with positive
determinant on every sub-patch and a simple projected boundary makes the
projection a homeomorphism onto a Jordan domain, hence P injective.
Consumers: loft whole-patch validity (CC-014), offset stars (CC-022), shell
bridge (CC-023), setback corners (CC-033).

```yaml
id:          CC-005-GRAPHDISK
contract:    [CC-005-GRAPHDISK]
class:       design
crates:      [truck-certified]
depends_on:  [CC-000-CONTRACT, CC-002-INJECTIVITY]
write_allow:
  - vendor/truck/truck-certified/src/construct/graphdisk.rs
  - vendor/truck/truck-certified/src/construct/mod.rs
  - vendor/truck/truck-certified/tests/construct_graphdisk.rs
read_allow:
  - docs/CERTIFIED_CONSTRUCTION_CONTRACTS.md
  - vendor/truck/truck-certified/src/construct
  - vendor/truck/truck-certified/src/formal/intersection.rs
  - vendor/truck/truck-certified/src/formal/xmonotone.rs
budget:      {turns: 24, ctx_tokens: 90000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub fn genuine_star' vendor/truck/truck-certified/src/construct/fixtures.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn folded_corner' vendor/truck/truck-certified/src/construct/fixtures.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub fn make_x_monotone' vendor/truck/truck-certified/src/formal/xmonotone.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub fn injectivity_radius' vendor/truck/truck-certified/src/construct/injectivity.rs"}
tests_required:
  - genuine_star_certifies
  - folded_corner_refuses_no_admissible_projection_or_star_not_embedded
  - non_simple_boundary_refuses_star_not_embedded
  - unglued_seam_refuses_star_not_embedded
  - projection_candidate_order_is_the_normative_sequence
  - projected_boundary_simplicity_uses_planar_exclusion_and_near_diagonal_radius
```

Section 1: `construct/graphdisk.rs` â€” the certificate per spine S6. `pub
struct DiskPiece` with PUB fields (consumers construct these from their own
certificates): `det_lower: Interval` (certified lower bound of
det D(Ï€âˆ˜P) on the piece), `boundary_simple: bool`, `seam_glued: bool`. `pub
struct GraphDiskCert` (per-piece records + the winning projection w). `pub
fn certify_graph_disk(pieces: &[DiskPiece], boundary: &BoundaryPlan) ->
Result<GraphDiskCert, ConstructRefusal>` â€” pre-made decision table, checked
in this order: any piece with `det_lower` NOT strictly positive (sup â‰¤ 0 or
enclosure straddling 0 is not admissible; require inf &gt; 0) â†’
`Err(NoAdmissibleProjection)` (the caller must search another projection);
any piece with `seam_glued == false` â†’ `Err(StarNotEmbedded)` (theory Â§1
P3: the seam clause is NOT implied by per-piece determinants); boundary not
simple (`BoundaryPlan` says so) â†’ `Err(StarNotEmbedded)`; all pass â†’
`Ok(GraphDiskCert)`. No heuristics, no repair, no second chances inside
this fn â€” it is the DECIDER over caller-supplied certificates.

Section 2: the projection search â€” `pub fn search_projection(...) ->
Result<(Vector3-ish w, Vec<DiskPiece>), ConstructRefusal>` over an
admitted surface: the normative candidate sequence from theory Â§1 P3 is
FROZEN and is a tested behavior (`projection_candidate_order_is_the_
normative_sequence`): (1) area-weighted average patch normal; (2) principal
directions of the control net; (3) a fixed spherical code. Pre-made: the
spherical code for v1 is the 14-point vertices of a refined octahedron
(Â±3 axes + the 6 face-centre diagonals of the cube octants, fixed order);
its exact point list is a `pub const` array in this module so consumers and
tests see one table. Exhaustion â†’ `Err(NoAdmissibleProjection)`; the
pairwise-SSI fallback with inside/outside witness is a LATER packet
(CC-014's composition), not this one. For each candidate w, per-piece
determinant lower bounds come from the caller-provided per-piece derivative
enclosures evaluated in interval arithmetic against w (fixed order); the
near-diagonal planar machinery (`formal/xmonotone` A3, `formal/
intersection`) plus the P2 radius (A4 â€” call
`curve_injectivity_radius`/the plane-projected curve analogue for boundary
simplicity) discharge boundary simplicity, per theory Â§1 P3 hypothesis (2).

Section 3: `BoundaryPlan` (CC-000 stub) gains its PRODUCTION meaning here
without changing the stub file: `certify_graph_disk` consumes the stub's
opaque verdict through the accessor CC-000 books â€” read `construct/
stubs.rs` first and use exactly the accessor it exposes; if the stub lacks
an accessor for boundary simplicity, STOP and file QUESTION.md (spine seam
defect).

House rules: **H-1: no `unwrap`/`expect`/`panic!` in shipped code, no
module-level `allow`.** **H-3: float comparisons in tests take the `// H-3`
opt-out ON THE SAME LINE.** **All cargo invocations go through the queue
(the `cargo` on PATH IS the queue shim). Do not invoke cargo by absolute
path; do not unset the shim.** Scoped checks only: `cargo check -p
truck-certified` and `cargo test -p truck-certified --test
construct_graphdisk`. No workspace builds. The `pub mod graphdisk;` line in
`construct/mod.rs` is the DESIGNED one-line conflict. COMMIT BEFORE writing
RESULT.json AT THE WORKTREE ROOT.

Stop conditions: (1) the CC-000 fixtures `genuine_star` (A1) and
`folded_corner` (A2) carry DiskPiece-shaped data with machine-checked
ground truths â€” if they do not decide as Â§6 of the spine says, file
QUESTION.md (CC-000 defect); (2) if the spherical code's 14 points cannot
be justified as fixed and order-stable, write the table and pin it with a
test asserting the exact f64 values (H-3 opt-outs) â€” the normative table
IS the deliverable; (3) this packet does NOT implement pairwise patch/patch
SSI â€” if you find yourself writing one, stop: that is CC-014's composition.
