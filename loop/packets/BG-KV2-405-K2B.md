# BG-KV2-405-K2B — the full lifted atlas: pole charts and the rational carrier family (section 3.3)

Wave-4 packet (build spec section 4; section 19 row 19; spec sections
3.3-3.4). Completes the K2 substrate: Param lifts with deck integers as
FIRST-CLASS coordinates, pole charts (genuine alternate charts, not
covering lifts), the chart-switch doctrine of section 3.4 (rank-deficient
parameterization -> switch chart if the image is certified regular
elsewhere; carrier singularity -> refuse/trim), and DeckExhausted as the
termination bound.

```yaml
id:          BG-KV2-405-K2B
contract:    [BG-KV2-405-K2B]
class:       design
crates:      [truck-certified]
depends_on:  [BG-KV2-104-RATCARRIER, BG-KV2-103-IDENTITY]
write_allow:
  - vendor/truck/truck-certified/src/kernel/atlas.rs
  - vendor/truck/truck-certified/src/kernel/mod.rs
  - vendor/truck/truck-certified/tests/kernel_atlas.rs
read_allow:
  - docs/CONSTRUCTIVE_GEOMETRY_KERNEL_SPEC_V2.md
  - vendor/truck/truck-certified/src/kernel
  - vendor/truck/truck-certified/src/domain/lattice.rs
budget:      {turns: 26, ctx_tokens: 95000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub struct Param' vendor/truck/truck-certified/src/kernel/graph.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub struct ChartId' vendor/truck/truck-certified/src/kernel/graph.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'sphere_rational_param_matches_implicit_form_on_grid' vendor/truck/truck-certified/tests/kernel_rational.rs"}
tests_required:
  - sphere_pole_chart_switches_and_continues_the_arc
  - param_lift_never_wraps_and_deck_is_first_class
  - chart_switch_vs_carrier_singularity_distinguished
  - cone_carrier_admitted_with_its_chart_family
  - deck_exhausted_terminates_helical_lifts
  - pcurve_runs_unwrapped_5_9_to_6_4
  - no_transcendental_call_in_atlas_module
```

Section 1: `pub struct ChartAtlas` over a rational carrier: the FINITE
atlas of regular charts per carrier kind (sphere: the stereographic pair
+ equatorial band charts as needed — the RATIONAL forms from 104 are the
enclosure machinery; the atlas adds the CHART BOOKKEEPING: chart ids,
overlap regions, transition maps as exact affine/rational data with
outward-rounded transport). Cone and Torus JOIN the admitted carrier
family here (404's deferred refusal moves): cone with apex-excluding
charts (section 3.4's carrier-singularity case at the apex), torus with
its rational parameterization's chart family. `cone_carrier_admitted_
with_its_chart_family` updates the 104 pending-refusal surface: the
refusal for Cone/Torus in kernel::rational now ROUTES to the atlas
implementors (a documented re-route, not a silent behavior change — the
test asserts the new route AND keeps the old refusal available for
out-of-atlas boxes).

Section 2: section 3.4's doctrine as code —
`pub fn classify_degeneracy(p: &dyn CertifiedPatch, box_: IBox2) ->
DegeneracyRoute` with the two outcomes: `SwitchChart { target: ChartId }`
(regularity of the IMAGE certified elsewhere — via the other chart's
regularity) vs `CarrierSingular` (refuse/trim). The sphere pole is the
first case (the fixture: an arc crossing u=v=0 continues on the partner
chart — same arc, no valence change); the cone apex is the second.
`param_lift_never_wraps`: Param(u) is the LIFTED coordinate (5.9 stays
5.9; the deck integer carries the winding) — the pcurve test runs 5.9 ->
6.4 with deck +1 (the shim kit's deck-wrap fixture, now through the
atlas). `deck_exhausted_terminates_helical_lifts`: |deck| > DECK_MAX on
one edge -> Refused(DeckExhausted) (Inconclusive) — the spec's
termination bound.

House rules: standing Wave-4 block. **H-1: the new modules (`atlas.rs`,
`kernel_atlas.rs`) carry the crate's unwrap discipline — no
`unwrap`/`expect`/`panic!`, no module-level `allow` (header style from
`hull.rs`). The landed `tests/kernel_rational.rs` is READ-ONLY context:
the 404-refusal re-route is asserted in THIS packet's new test file, never
by editing the landed file.**

Stop conditions: 1. frozen seam differs — stop, record. 2. A carrier's
chart family needs a transition map that is not affine/rational with
outward-rounded transport — stop, name it (that carrier waits for the
published form; do not approximate). 3. The 404-refusal re-route breaks
a landed 104 test — the re-route is WRONG as designed; stop and record.

Commit subject: `feat(certified): lifted atlas, pole charts, cone+torus
carrier family (BG-KV2-405-K2B)`.
