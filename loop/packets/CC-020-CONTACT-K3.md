# CC-020-CONTACT-K3 — the three-support constrained contact system

CC program Phase C (spine S11; theory §3.1–3.3). At k = 3 the contact
stratum with a radius law is 0-dimensional: an isolated triple-contact
junction. This packet designs the square-system reduction and certifies the
node. Consumers: offset corner strata (CC-021), blend branch junctions
(CC-030), setback corner inputs (CC-033).

```yaml
id:          CC-020-CONTACT-K3
contract:    [CC-020-CONTACT-K3]
class:       design
crates:      [truck-certified]
depends_on:  [CC-000-CONTRACT, CC-025-CANAL]
write_allow:
  - vendor/truck/truck-certified/src/construct/contact3.rs
  - vendor/truck/truck-certified/src/construct/mod.rs
  - vendor/truck/truck-certified/tests/construct_contact3.rs
read_allow:
  - docs/CERTIFIED_CONSTRUCTION_CONTRACTS.md
  - vendor/truck/truck-certified/src/construct
  - vendor/truck/truck-certified/src/kernel
budget:      {turns: 26, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub fn krawczyk_c1_n4' vendor/truck/truck-certified/src/kernel/engine.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub struct TripleContactNode' vendor/truck/truck-certified/src/construct/stubs.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub enum RadiusLaw' vendor/truck/truck-certified/src/construct/stubs.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub fn radius_eval' vendor/truck/truck-certified/src/construct/canal.rs"}
tests_required:
  - three_plane_wedge_node_matches_hand_computed_centre
  - submersion_margin_below_threshold_refuses_rank_deficient_contact
  - no_root_in_box_returns_no_root_not_refusal
  - radius_law_node_matches_constant_radius_ground_truth
  - seed_box_bisection_terminates_within_depth_cap
```

Section 1: the reduction — THE designed content of this packet, pre-made in
shape: the raw k = 3 contact equations (c = S_i(u_i) + ε_i r n_i(u_i),
i = 1..3, plus Φ) are 7 unknowns in a 7-equation square form — ABOVE the
landed engine's arity. The reduction to a ≤4-unknown square system is:
per support, work in the 2-parameter chart (u_i, v_i); eliminate c by
pairwise differences (S_1 + ε_1 r n_1 = S_2 + ε_2 r n_2, S_1 + ε_1 r n_1 =
S_3 + ε_3 r n_3 — six scalar equations); project each difference equation
onto the tangent planes of TWO of the three supports (pre-made choice: the
two supports with the largest certified rank margins, evaluated first in a
fixed order), leaving a square system in (r, plus one tangent parameter on
each of the three supports) = 4 unknowns, closed by Φ through
`radius_eval` (A4). The exact chart/projection choices are the worker's one
named judgement — record the choice and its justification in RESULT notes.

Section 2: the solver — `pub fn solve_triple_node(supports:
&[SurfaceRegion; 3], radius: &RadiusLaw, seed: IBox3, budget: &mut Budget)
-> Result<TripleContactNode, ConstructRefusal>` per spine S11. Wait — the
seed is a box over the REDUCED variables, so the packet's first deliverable
is the reduced-variable mapping: `pub struct ReducedSystem` carrying the
chart selection, the four-variable box type (`IBox4` from the landed kernel
patch machinery), and the Krawczyk evaluation of the reduced F and DF over
that box, consuming `krawczyk_c1_n4` (A1) — the engine's Lemma-8.0
contraction, never a hand-rolled Newton. Outcomes, pre-made: contraction
with strict interior → the node, packaged as `TripleContactNode` (A2:
centre, radius, per-support parameter enclosures) with the submersion
margin η_F computed from the interval DF (DF·DFᵀ ⪰ η_F²I certified
componentwise; η_F &lt; `CC_ETA_J`-class floor →
`Err(RankDeficientContact)`); Krawczyk disproof → `Ok`-side "NoRoot"
reported as `Err(ConstructRefusal::InvalidInput)` with a no-root witness
flag in the message path is WRONG — instead return the typed outcome via
`pub enum TripleNodeOutcome { Node(TripleContactNode), Empty }` and let
`solve_triple_node` return `Result<TripleNodeOutcome, ConstructRefusal>`
(spine S11's Result<TripleContactNode, _> is thereby refined: Empty is a
certified answer, not a refusal — record this refinement as the one named
seam amendment in RESULT notes); bisection until `CC_DEPTH_MAX`, then
`Err(RankDeficientContact)`.

Section 3: ground truth — three planes forming a trihedral corner with a
constant radius law have the hand-computable in-sphere centre at (r, r, r)
in the corner's octant (r from the law): the fixture builds the three
admitted surfaces through the landed map machinery and asserts the node's
centre enclosure contains the hand value (H-3 opt-outs). The submersion
refusal is exercised with a degenerate support (two coincident planes —
rank drop is structural, the refusal must fire before any iteration).

House rules: **H-1: no `unwrap`/`expect`/`panic!` in shipped code, no
module-level `allow`.** **H-3: float comparisons in tests take the `// H-3`
opt-out ON THE SAME LINE.** **All cargo invocations go through the queue
(the `cargo` on PATH IS the queue shim). Do not invoke cargo by absolute
path; do not unset the shim.** Scoped checks only: `cargo check -p
truck-certified` and `cargo test -p truck-certified --test
construct_contact3`. No workspace builds. The `pub mod contact3;` line in
`construct/mod.rs` is the DESIGNED one-line conflict. COMMIT BEFORE writing
RESULT.json AT THE WORKTREE ROOT.

Stop conditions: (1) if NO ≤4-unknown reduction preserves the transversality
certificates (you cannot certify submersion of the reduced system), STOP and
file QUESTION.md — that is a theory-seam defect (the spine's S11 arity claim
would be wrong), not something to route around by raising the engine's
arity; (2) `TripleContactNode` is the CC-000 stub with pub fields — fill it,
extend nothing; the `TripleNodeOutcome` refinement above is the only new
public type this packet may add; (3) the ε_i side signs come from the
caller's support descriptions — if the `SurfaceRegion` input cannot carry
them, record the actual convention you used in RESULT notes (offset side is
a caller-level fact until CC-021 books it).
