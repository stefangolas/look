# CC-014-LOFT-VALIDITY — L5: regularity + self-contact postcondition, three-valued

CC program Phase B (spine S4/S6/S7 consumers; theory §2.2 L5). Once
correspondence is an orientation-preserving combinatorial homeomorphism the
loft can fail geometrically in exactly two ways: regularity loss
(Sᵤ×Sᵥ = 0) or self-contact (S(p) = S(q), p ≠ q). There is no separate
pinch or twist theory. The postcondition is THREE-VALUED per the CG
verdict doctrine: certified / failed / inconclusive is surfaced, never
converted into success.

```yaml
id:          CC-014-LOFT-VALIDITY
contract:    [CC-014-LOFT-VALIDITY]
class:       design
crates:      [truck-certified]
depends_on:  [CC-000-CONTRACT, CC-002-INJECTIVITY, CC-004-CLEAR, CC-005-GRAPHDISK, CC-012-LOFT-STRIPS]
write_allow:
  - vendor/truck/truck-certified/src/construct/loft_validity.rs
  - vendor/truck/truck-certified/src/construct/mod.rs
  - vendor/truck/truck-certified/tests/construct_loft_validity.rs
read_allow:
  - docs/CERTIFIED_CONSTRUCTION_CONTRACTS.md
  - vendor/truck/truck-certified/src/construct
  - vendor/truck/truck-evidence/src/contact
budget:      {turns: 26, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub fn injectivity_radius' vendor/truck/truck-certified/src/construct/injectivity.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn certify_graph_disk' vendor/truck/truck-certified/src/construct/graphdisk.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub fn contact' vendor/truck/truck-evidence/src/contact/mod.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub fn rank_margin' vendor/truck/truck-certified/src/certified_map.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'pub struct LoftStrips' vendor/truck/truck-certified/src/construct/loft_strips.rs"}
tests_required:
  - regular_margin_below_eta_j_fails_the_postcondition
  - near_diagonal_pairs_are_excluded_by_radius_never_searched
  - far_pair_contact_found_reports_unintended_contact
  - undecided_pairs_surface_as_inconclusive_never_certified
  - graphdisk_discharge_takes_precedence_over_pairwise_search
```

Section 1: verdict type (pre-made) — `pub enum PairVerdict { Certified,
Contact, Inconclusive }` and `pub struct LoftValidityCert { pub regularity:
Result<Interval, ConstructRefusal>, pub pairs: Vec<PairVerdict>, pub
discharged_by_graphdisk: bool }`. `pub fn certify_loft_validity(map:
&CertifiedSurfaceMap, strips: &LoftStrips, budget: &mut Budget) ->
Result<LoftValidityCert, ConstructRefusal>` — Ok ALWAYS means the
certificate was PRODUCED; whether the loft is valid is read off the
certificate (a loft that fails validity is a valid certificate about an
invalid loft). `Contact` and `Inconclusive` pairs are data, and the CALLER
decides to refuse — the three-valued doctrine from CG §3.3.

Section 2: the two arms, in the pre-made order. Regularity arm: `rank_margin`
(A4) over the loft's admitted map per strip; below `CC_ETA_J` → recorded
(failure is data + the regularity Result carries the enclosure). Self-contact
arm, three discharge regimes in the theory's order: (a) near-diagonal within
a stratum — `injectivity_radius` (A1) per region; pairs within δ are EXCLUDED
from the candidate list by construction and are never searched (test 2
observes the exclusion via a counter of attempted pairs); (b) whole regions
where an admissible projection exists — `certify_graph_disk` (A2) FIRST
(test 5: graphdisk discharge takes precedence; pairwise search runs only on
undischarged regions); (c) everything else — broad phase over the strip
control boxes followed by the landed evidence contact funnel (A3) through
the CC-000 manifest edge, converting its `ContactComplex`/Refusal outcomes:
a certified contact → `PairVerdict::Contact`; a refusal of the
`NumericallyUnresolved` family → `PairVerdict::Inconclusive`; anything else
→ propagate as `Err(ConstructRefusal::...)` via the conversion documented at
the S7 seam. The inari/certified boundary at (c) is `convert.rs` — the ONLY
sanctioned bridge.

Section 3: budget discipline — subdivision and contact spends draw from the
caller's `Budget` (A3-side convention: entry-minus-remaining reporting);
`CC_DEPTH_MAX` caps region splitting. A budget-exhausted pair is
`Inconclusive`, never `Certified`.

House rules: **H-1: no `unwrap`/`expect`/`panic!` in shipped code, no
module-level `allow`.** **H-3: float comparisons in tests take the `// H-3`
opt-out ON THE SAME LINE.** **All cargo invocations go through the queue
(the `cargo` on PATH IS the queue shim). Do not invoke cargo by absolute
path; do not unset the shim.** Scoped checks only: `cargo check -p
truck-certified` and `cargo test -p truck-certified --test
construct_loft_validity`. No workspace builds. The `pub mod
loft_validity;` line in `construct/mod.rs` is the DESIGNED one-line
conflict. COMMIT BEFORE writing RESULT.json AT THE WORKTREE ROOT.

Stop conditions: (1) this packet COMPOSES landed certificates — if you find
yourself implementing a new contact solver, a new hull kernel, or a new SSI,
stop: the composition contract is the deliverable; (2) if the evidence
contact funnel's `Refusal` variants do not map cleanly onto the three
`PairVerdict` outcomes, record the actual mapping table in RESULT notes and
file the gap as a QUESTION — do not collapse Inconclusive into Contact to
make the types fit; (3) the near-diagonal counter in test 2 is a debug-only
counting hook (`#[cfg(test)]`) — it must not exist in shipped signatures.
