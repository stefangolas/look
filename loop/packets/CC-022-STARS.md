# CC-022-STARS — closed-star embedding certification + broad phase over constructed strata

CC program Phase C (spine S6 consumer; theory §4.1 stars, §4.3 broad phase).
Within a single stratum P2 supplies the injectivity radius; a CLOSED STAR
spanning several glued strata is certified embedded by P3 — the constructive
form of local embeddedness at edges and corners. The broad phase prunes on
certified reach bounds over the CONSTRUCTED strata.

```yaml
id:          CC-022-STARS
contract:    [CC-022-STARS]
class:     design
crates:      [truck-certified]
depends_on:  [CC-000-CONTRACT, CC-005-GRAPHDISK, CC-021-OFFSET-STRATA]
write_allow:
  - vendor/truck/truck-certified/src/construct/stars.rs
  - vendor/truck/truck-certified/src/construct/mod.rs
  - vendor/truck/truck-certified/tests/construct_stars.rs
read_allow:
  - docs/CERTIFIED_CONSTRUCTION_CONTRACTS.md
  - vendor/truck/truck-certified/src/construct
budget:      {turns: 20, ctx_tokens: 90000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub fn certify_graph_disk' vendor/truck/truck-certified/src/construct/graphdisk.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub enum OffsetStratum' vendor/truck/truck-certified/src/construct/offset_strata.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub fn reach_bound' vendor/truck/truck-certified/src/construct/offset_strata.rs"}
tests_required:
  - two_plane_wedge_star_certifies_embedded
  - star_with_folded_piece_refuses_star_not_embedded
  - reach_pruning_disjoint_pair_never_enters_candidate_list
  - reach_pruning_close_pair_is_retained
  - glue_seam_mismatch_refuses_before_graphdisk
```

Section 1: the star — `pub struct Star { pub strata: Vec<OffsetStratum>,
pub glue_plan: GluePlan }` where `GluePlan` records which stratum boundaries
are identified (the intended identifications; a caller-supplied combinatorial
plan, exact — P6 discipline: identified points are referenced by identity,
never matched by proximity). `pub fn certify_star(star: &Star) ->
Result<GraphDiskCert, ConstructRefusal>` reduces the star to CC-005's
machinery: per-piece determinant data for the P3 certificate comes from each
stratum's landed regularity certificate (face: J_t margin; edge: canal
interval; corner: the node's centre enclosure mapped through the projection).
Pre-made glue gate BEFORE graphdisk (test 5): every pair of strata named in
`GluePlan` must agree on the shared boundary's identity reference; any
mismatch → `Err(StarNotEmbedded)` before the projection search runs.

Section 2: broad phase — `pub fn reach_prune(strata: &[OffsetStratum]) ->
Vec<(usize, usize)>`: the certified per-stratum reach bounds (A3) give the
theory §4.3 prune `d(A,B) > ρ_A + ρ_B ⟹ realizations disjoint`, with the
distance lower-bounded by the CC-004 axis-gap formula on the strata's
bounding boxes (the landed `Bvh::distance_lower_bound` is available through
the manifest edge for piece-set inputs; at this layer boxes suffice).
Deterministic: candidate pairs sorted, deduplicated. Tests 3/4 pin the
predicate at known box separations (H-3 opt-outs): a disjoint pair never
enters the candidate list, a close pair is retained. Note in the module doc:
this prune is SOUND but not complete — retained pairs go to the contact
funnel in CC-023.

House rules: **H-1: no `unwrap`/`expect`/`panic!` in shipped code, no
module-level `allow`.** **H-3: float comparisons in tests take the `// H-3`
opt-out ON THE SAME LINE.** **All cargo invocations go through the queue
(the `cargo` on PATH IS the queue shim). Do not invoke cargo by absolute
path; do not unset the shim.** Scoped checks only: `cargo check -p
truck-certified` and `cargo test -p truck-certified --test construct_stars`.
No workspace builds. The `pub mod stars;` line in `construct/mod.rs` is the
DESIGNED one-line conflict. COMMIT BEFORE writing RESULT.json AT THE
WORKTREE ROOT.

Stop conditions: (1) `GraphDiskCert`/`DiskPiece` are CC-005's output types —
consume, extend nothing; (2) this packet does not run the contact funnel or
prove global embedding (CC-023); (3) if the landed stratum certificates
cannot supply per-piece determinant data for the projection without new
geometry evaluation, record exactly what is missing in RESULT notes and
implement the projection input as caller-supplied enclosures — do not
re-derive stratum certificates here.
