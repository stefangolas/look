# BG-KV2-303-S9A — segment gluing, deck identification, graph assembly

Wave-3 implementation packet (build spec §4; §19 row 13; spec §14.1–§14.2).
Assembles `CertifiedGraph`s from certified arcs: node identity via the
LAND Rules A/B/C (kernel/identity.rs — consume, never restate), segment
gluing per §14.2's three conditions, deck identification with the
DECK_MAX winding bound, and the graph construction itself. The deck
ARITHMETIC substrate is landed (formal/deck.rs, domain/lattice.rs) —
consume it; this packet wires it to arc chains.

```yaml
id:          BG-KV2-303-S9A
contract:    [BG-KV2-303-S9A]
class:       design
crates:      [truck-certified]
depends_on:  [BG-KV2-103-IDENTITY, BG-KV2-201-S2A]
write_allow:
  - vendor/truck/truck-certified/src/kernel/assemble.rs
  - vendor/truck/truck-certified/src/kernel/mod.rs
  - vendor/truck/truck-certified/tests/kernel_assemble.rs
read_allow:
  - docs/CONSTRUCTIVE_GEOMETRY_KERNEL_SPEC_V2.md
  - vendor/truck/truck-certified/src/kernel
  - vendor/truck/truck-certified/src/formal/deck.rs
  - vendor/truck/truck-certified/src/domain/lattice.rs
budget:      {turns: 30, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub struct CertifiedGraph' vendor/truck/truck-certified/src/kernel/graph.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub struct TubeOverlapCert' vendor/truck/truck-certified/src/kernel/certs.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub struct DeckBudget' vendor/truck/truck-certified/src/formal/deck.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub mod contact;' vendor/truck/truck-certified/src/kernel/mod.rs"}
tests_required:
  - gluing_requires_tube_overlap_and_c1_agreement
  - gluing_refuses_sliver_instead_of_snapping
  - deck_identification_closes_a_full_cylinder_wrap
  - helix_exceeding_deck_max_refuses_deck_exhausted
  - node_identity_uses_rules_abc_not_proximity
  - morse_saddle_identifies_against_its_half_arc_endpoints
  - assembled_graph_has_no_refuse_nodes
  - no_transcendental_call_in_assemble_module
```

## Section 1 — gluing (`kernel/assemble.rs`, NEW)

§14.2's three conditions between arcs A, B meeting at a SegmentBreak:
(1) tube overlap with a common certified point ->
  `TubeOverlapCert::try_new(shared_point, c1_bound)` (the shim
  constructor; c1_bound <= EPS_REP) — the shared point is computed as the
  identity-Rule-A/B/C match of the two arcs' endpoint regions (float
  propose: the nearer endpoint pair; intervals dispose: the
  C1-agreement enclosure below);
(2) stored Hermite approximants agree to C1 at the break within EPS_REP:
  the C1 bound certified by interval evaluation of both approximants'
  endpoints/derivatives over the shared point's box (a plain enclosure
  comparison — no snapping, ever);
(3) the concatenated pcurve reparameterizes to a single monotone
  parameter (arclength of the model-space approximant, recorded as the
  ledger's parameter domain — a data statement on the glued arc, the
  EdgeSampleLedger integration is C3's landed entry, consumed via its
  public path if reachable, else recorded as the S9b seam).
Tubes overlapping with endpoints NOT matching under any rule of §4.2 ->
`Refused(SliverOrNearOverlap)` (Inconclusive) — `gluing_refuses_sliver_
instead_of_snapping` pins it with a fixture whose endpoints are near but
not certified-equal.

## Section 2 — deck identification

An arc ending at (chart, deck=k, u~) and one beginning at (chart, deck=
k+1, u~ - P) denote the same point. Deliverables:
- `pub fn deck_identify(chain: &[ArcEnd...]) -> Construction<Vec<Break>>`
  — compute total deck displacement per closed chain (integer arithmetic
  via the landed lattice/deck types where they apply; plain exact i32
  sums where the chain's chart data carries the periods); a chain whose
  endpoints differ by an exact integer deck translation AND whose nodes
  identify by Rule B closes as a loop; the displacement is recorded on
  the edge as its winding (data on the assembled graph's arcs).
- |deck| > DECK_MAX on one edge -> `Refused(DeckExhausted)`
  (Inconclusive) — `helix_exceeding_deck_max_refuses` pins it with a
  chain whose winding exceeds config::DECK_MAX.
- `deck_identification_closes_a_full_cylinder_wrap`: the shim kit's deck-
  wrap fixture + a second arc at deck+1 -> one loop, winding +1.

## Section 3 — assembly

`pub fn assemble(arcs: Vec<AnyArc>, breaks: Vec<Break>, nodes: Vec<Node>)
-> Construction<CertifiedGraph>` — validation: every ArcEnd::Topo
resolves to a Node whose NodeCert is Exact or AtTolerance; every
ArcEnd::Seg resolves to a Break with a TubeOverlapCert; NO TopoNode
variant is Refuse (the shim's enum makes this structural — the test
pins the exhaustive match); identity uses Rules A/B/C via
`rule_a`/`rule_b`/`rule_c` — the
`node_identity_uses_rules_abc_not_proximity` test asserts a
near-miss pair (dist ~ 1e-10, different residuals) does NOT identify.
`morse_saddle_identifies_against_its_half_arc_endpoints` is the Rule-C
fixture (spec section 20 row): an R2-stamped PointCert pair vs R1
half-arc endpoints identifying through the implication.

House rules: H-1; H-3 same-line; fmt + clippy (exact verify form,
unfiltered, ALL findings) clean; `cargo check --workspace --all-targets`
green. CARGO_BUILD_JOBS=2-4. COMMIT BEFORE writing RESULT.json AT THE
WORKTREE ROOT.

## Stop conditions

1. A frozen shape differs (graph/certs/identity spellings) — stop,
   record the diff.
2. The landed deck machinery's types do not compose with chain-level
   identification (a genuinely missing adapter) — stop, name the
   adapter; the wiring is an amendment, not an improvisation.

Commit subject: `feat(certified): segment gluing + deck identification +
graph assembly (BG-KV2-303-S9A)`.
