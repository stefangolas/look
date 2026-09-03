# BG-KV2-502-S9B — promotion to a model edge (spec 14.2-14.3)

Wave-5 packet (build spec section 4; serial, integrator-owned — second of
the chain). The census (build spec section 3, row S9) records: deck
machinery strong (303 landed `assemble`/`glue`/`deck_identify` over the
frozen shapes), but promotion of an assembled arc to a B-rep edge is ZERO —
no promotion entry, no `SliverOrNearOverlap`, no `deck_max` routing through
promotion. Spec 14.3 books the eight promotion conditions; this packet
lands them as one refusing entry over the landed assemble output.

```yaml
id:          BG-KV2-502-S9B
contract:    [BG-KV2-502-S9B]
class:       design
crates:      [truck-certified]
depends_on:  [BG-KV2-501-C6]
write_allow:
  - vendor/truck/truck-certified/src/kernel/promote.rs
  - vendor/truck/truck-certified/src/kernel/mod.rs
  - vendor/truck/truck-certified/tests/kernel_promote.rs
read_allow:
  - docs/CONSTRUCTIVE_GEOMETRY_KERNEL_SPEC_V2.md
  - vendor/truck/truck-certified/src/kernel
budget:      {turns: 24, ctx_tokens: 90000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub fn assemble(' vendor/truck/truck-certified/src/kernel/assemble.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub struct ChainArc' vendor/truck/truck-certified/src/kernel/assemble.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub struct CertifiedGraph' vendor/truck/truck-certified/src/kernel/graph.rs"}
tests_required:
  - promotion_emits_arclength_parameterized_edge
  - promoted_endpoints_are_shared_c1_nodes
  - sliver_near_overlap_refuses_never_snaps
  - deck_exhausted_routes_through_promotion
  - knot_multiplicity_set_at_crossings_and_cusps
  - tangency_tag_requires_explicit_opt_in
```

Section 1: `pub struct PromotedEdge` — the spec 14.3 output as a KERNEL
RECORD, deliberately NOT a live `truck_topology::Edge` handle (the landed
topology constructors panic in debug on circle-carried self-loops — the
record avoids the whole class; binding to live handles is downstream
integration, not this packet). Fields: the model-space Hermite approximant,
both pcurves in their lifted charts, the exported arclength
parameterization with its position table (spec 14.3 condition 7), knot
multiplicities (condition 6), and the owning-face chart ids. `pub fn
promote(arc: &ChainArc, ctx: &PromoContext) -> Result<PromotedEdge,
Refusal>` walks the eight conditions in order; each failing condition is a
NAMED refusal carrying the evidence the spec names for it. `PromotedEdge`
also `serialize`s? No: keep it plain data (Debug + Clone), serialization is
not booked for this wave.

Section 2: the conditions as code. Endpoints: shared `TopoNode` with a C1
certificate, identified by the LANDED A4.2 rules (303's `regions_identify` —
reuse, never re-derive); condition 2's failure is the refusal. Trim events
in one chart (condition 3) route through the landed R9 residuals
(`residuals_r89.rs`). `deck_exhausted_routes_through_promotion`: |deck| >
`DECK_MAX` (the landed config constant) inside a promoted arc ->
`Refused(DeckExhausted)` — the spec's termination bound holds at promotion
even though `deck_identify` already refuses at assembly (the test drives it
through `promote` directly with a forced deck overflow fixture). The
`TangencyAtTolerance` gate (condition 8): an endpoint carrying the tag
refuses UNLESS `PromoContext` carries the explicit opt-in flag — the flag
is a typed field, not a bool default true.

Section 3: `sliver_near_overlap_refuses_never_snaps` — tubes overlap
(the landed `c1_bound_of`/`glue` agreement machinery says the arcs'
Hermite ends agree within the tube radius) but NO A4.2 rule identifies
their endpoints -> `Refused(SliverOrNearOverlap)`, and the fixture asserts
no coordinate was moved (never snap: the refusal carries both endpoints
verbatim).

House rules: standing Wave-5 block. **H-1: the new files (`promote.rs`,
`kernel_promote.rs`) carry the crate's unwrap discipline — no
`unwrap`/`expect`/`panic!`, no module-level `allow`.** **H-3: float
comparisons in tests take the `// H-3` opt-out ON THE SAME LINE.** **All
cargo invocations go through the queue (the `cargo` on PATH IS the queue
shim). Do not invoke cargo by absolute path; do not unset the shim.** The
`pub mod promote;` line in `kernel/mod.rs` is the DESIGNED one-line
conflict. The fixture kit (`kernel/fixtures.rs`) is READ-ONLY: build
promotion fixtures from its shapes, extend nothing. COMMIT BEFORE writing
RESULT.json AT THE WORKTREE ROOT.

Stop conditions: 1. a frozen seam (assemble's `ChainArc`/`GlueCert` shape,
the A4.2 rule entries, the config constants) differs from what this packet
names — stop, record. 2. a promotion condition cannot be checked from
stored data alone (needs a numeric solve outside the landed seams) — stop,
name the condition. 3. an A4.2 rule cannot be reused without Euclidean
welding — stop (that would be the census S9 identity gap resurfacing, a
design decision, not this packet's call).

Commit subject: `feat(certified): promotion to a model edge with the eight
refusing conditions (BG-KV2-502-S9B)`.
