# CC-030-BLEND-SPINE — two-support continuation with event isolation and P5 admissibility

CC program Phase D (spine S12; theory §5.1–5.2). A rolling-ball fillet
network is a walk in the admissible stratum graph: pairwise strata E_ij
(1-D canal spines) meeting at triple-contact nodes V_ijk (0-D, CC-020's
output). The continuation chooses no branch heuristically — P5 admissibility
retains the outgoing strata on the boundary of the admissible centre region.
The operative rule: NO TOPOLOGY SPECULATION BETWEEN CERTIFIED EVENTS.

```yaml
id:          CC-030-BLEND-SPINE
contract:    [CC-030-BLEND-SPINE]
class:       design
crates:      [truck-certified]
depends_on:  [CC-000-CONTRACT, CC-004-CLEAR, CC-020-CONTACT-K3]
write_allow:
  - vendor/truck/truck-certified/src/construct/blend.rs
  - vendor/truck/truck-certified/src/construct/mod.rs
  - vendor/truck/truck-certified/tests/construct_blend.rs
read_allow:
  - docs/CERTIFIED_CONSTRUCTION_CONTRACTS.md
  - vendor/truck/truck-certified/src/construct
  - vendor/truck/truck-certified/src/ssi_trace.rs
budget:      {turns: 28, ctx_tokens: 110000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub fn solve_triple_node' vendor/truck/truck-certified/src/construct/contact3.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub enum EventKind' vendor/truck/truck-certified/src/construct/stubs.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub struct BranchSeed' vendor/truck/truck-certified/src/construct/stubs.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub fn trace_blend_chain' vendor/truck/truck-certified/src/construct/blend.rs"}
tests_required:
  - two_plane_chain_walks_and_terminates_at_trim_events
  - event_isolation_holds_between_certified_events
  - clear_loss_stops_the_branch_as_collision_event
  - triple_node_joins_three_branches_exactly
  - topology_between_events_is_never_speculated
```

Section 1: the two-support branch — for supports (S_i, S_j) with side signs
and radius law, the constrained branch is the 1-D solution family of
c = S_i(u_i) + ε_i r n_i = S_j(u_j) + ε_j r n_j, Φ(r) = 0. Pre-made v1
formulation: reduce to the 3-unknown square system (one tangent parameter
on the FIRST support, r, and the second support's along-edge parameter)
closed by Φ through the landed radius evaluators, then continue with the
landed kernel engine's arity-3 Krawczyk (`krawczyk_c1_n3`) in a
predictor/corrector walk: predictor = tangent step from the certified
chart data; corrector = Krawczyk on the stepped box; a step is certified
only when the operator contracts with strict interior — otherwise halve.
`BranchSeed` (A3) gains its production meaning here: the two support
descriptions, side signs, radius law, and the seed box. If this 3-unknown
reduction cannot preserve the transversality margin, file QUESTION.md (the
same seam rule as CC-020).

Section 2: the event vocabulary — `EventKind` is the CC-000 stub (A2: Trim
| ThirdFace | Focal | Rank | Collision | Trace). Events are detected as
isolated-root problems at the walk's certified-step boundaries: `Focal` and
`Rank` when the branch's regularity/rank margins collapse (the landed
margin machinery); `Collision` when `ball_clearance` flips to Rejected for
the rolling ball against the excluded boundary (A-tests: test 3); `ThirdFace`
when `solve_triple_node` (A1) certifies a node on the branch; `Trim` when a
contact parameter reaches a support's trim boundary. Discrete state Σ per
theory §5.2: every component's defining function must be certified
nonzero/separated on each accepted step — the step is rejected otherwise
(test 2 pins isolation: Σ identical across all accepted steps between two
events).

Section 3: the walk — `pub fn trace_blend_chain(branches: &[BranchSeed],
radius: &RadiusLaw, budget: &mut Budget) -> Result<BlendTrace,
ConstructRefusal>` (A4) walks each seed, joins branches at triple nodes
(nodes are SOLVED ONCE and referenced — P6; test 4 asserts the shared node
identity across three incident branches), and terminates a branch only at
certified events. Between events the topology is FIXED — no speculation,
no extrapolation past an undecided step: an undecided step surfaces as
`Err` of the underlying refusal family, never as a guessed continuation
(test 5 observes the walk stopping exactly at the last certified step).
`BlendTrace` records events in walk order with their certified enclosures.

House rules: **H-1: no `unwrap`/`expect`/`panic!` in shipped code, no
module-level `allow`.** **H-3: float comparisons in tests take the `// H-3`
opt-out ON THE SAME LINE.** **All cargo invocations go through the queue
(the `cargo` on PATH IS the queue shim). Do not invoke cargo by absolute
path; do not unset the shim.** Scoped checks only: `cargo check -p
truck-certified` and `cargo test -p truck-certified --test
construct_blend`. No workspace builds. The `pub mod blend;` line in
`construct/mod.rs` is the DESIGNED one-line conflict. COMMIT BEFORE writing
RESULT.json AT THE WORKTREE ROOT.

Stop conditions: (1) face consumption is CC-032 and setback corners are
CC-033 — the walk's deliverable ends at the certified event record; (2)
variable-radius laws ride the SAME 3-unknown system through the foot-point
formulation — that is CC-031's amendment, not this packet's; if the
constant-radius fixtures cannot terminate within `CC_DEPTH_MAX`, record the
termination statistics in RESULT notes; (3) the EventKind stub is closed —
new event kinds are a CC-000 amendment, never a local enum.
