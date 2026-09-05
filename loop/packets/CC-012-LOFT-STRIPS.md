# CC-012-LOFT-STRIPS â€” Closed-wire lofts as strips with P6-shared split data (L3)

CC program Phase B (spine S8/S9 consumer; theory Â§2.2 L3). A closed-wire
loft is r strips over matched edges, not one periodic surface. Adjacent
strips share split vertex data by construction identity (P6), which makes
their common boundary the SAME computation and the seam agreement BITWISE â€”
the only exactness available in the loft pipeline.

```yaml
id:          CC-012-LOFT-STRIPS
contract:    [CC-012-LOFT-STRIPS]
class:       design
crates:      [truck-certified]
depends_on:  [CC-000-CONTRACT, CC-010-LOFT-CORE, CC-011-LOFT-WEIGHTS]
write_allow:
  - vendor/truck/truck-certified/src/construct/loft_strips.rs
  - vendor/truck/truck-certified/src/construct/mod.rs
  - vendor/truck/truck-certified/tests/construct_loft_strips.rs
read_allow:
  - docs/CERTIFIED_CONSTRUCTION_CONTRACTS.md
  - vendor/truck/truck-certified/src/construct
  - vendor/truck/truck-topology/src/entity_id.rs
budget:      {turns: 20, ctx_tokens: 90000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub fn loft_sections' vendor/truck/truck-certified/src/construct/loft.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub struct LoftOutput' vendor/truck/truck-certified/src/construct/loft.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub fn output' vendor/truck/truck-topology/src/entity_id.rs"}
  - {id: A4, expect: 2, cmd: "grep -c 'kind: OpKind::Loft' vendor/truck/truck-topology/src/entity_id.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'pub fn certify_weight_field' vendor/truck/truck-certified/src/construct/loft_weights.rs"}
tests_required:
  - adjacent_strip_boundaries_agree_bitwise
  - one_factorization_shared_across_all_strips
  - split_vertex_identity_surfaces_recomputation
  - weight_refinements_applied_to_shipped_net
  - open_loft_degenerates_to_single_strip
```

Section 1: identity-bearing split data (P6, pre-made) â€” the shared
boundary vertex values `V_k` of each strip pair are derived from
`EntityId`s built with the landed algebra (A3/A4):
`Op { kind: OpKind::Loft, params: OpParams::List([..section ids..]) }.output
(&inputs, slot)` with slot = the strip-pair index. The split VALUE is
computed once per identity in a `BTreeMap<EntityId, [f64; 4]>` registry
inside the builder (deterministic iteration; never a HashMap), and both
strips consume the registry entry. Two evaluations of the same split point
in different call orders never meet: the value is computed ONCE. Recomputing
instead of consuming the registry is the failure the bitwise test exists to
catch.

Section 2: the builder â€” `pub struct LoftStrips { pub strips:
Vec<LoftOutput>, pub seam_ids: Vec<EntityId> }` and `pub fn
loft_closed_wire(sections: &[BSplineCurve<Point4>], splits: &[usize],
stations: &[f64], degree: usize) -> Result<LoftStrips, ConstructRefusal>`:
split each section at the matched parameters into r arcs (exact knot
insertion + cut through the landed nurbs ops), build r strip lofts through
CC-010's `loft_sections` (A1) with ONE shared `BandedFactorization` â€” the
factorization is computed once and reused across all strips (test
`one_factorization_shared_across_all_strips` observes this through the
identical `epsilon` on every strip: distinct factorizations of the same
matrix can deliver different max-error enclosures; identical Îµ on every
strip is the observable). Hypotheses (1)/(2) of L3 (clamped u-knots;
identical v-stations/degree/knots) are structural here â€” assert them at
build time, refuse `InvalidInput` on violation. Weight certification
(CC-011) runs once per strip and `refinements` are applied to every shipped
strip net (A5, test `weight_refinements_applied_to_shipped_net`).

Section 3: the L3 gate (`adjacent_strip_boundaries_agree_bitwise`): take
the u-endpoint control row of strip i and the u-start row of strip i+1 and
assert BYTE equality of their f64 bit patterns (`to_bits()`, H-3 exempt as
an integer comparison â€” no epsilon anywhere). A tolerance comparison here
is a test failure even when the values agree to 1 ulp: the whole point is
that under P6 the agreement is not a numerical fact.

House rules: **H-1: no `unwrap`/`expect`/`panic!` in shipped code, no
module-level `allow`.** **H-3: float comparisons in tests take the `// H-3`
opt-out ON THE SAME LINE (bit-pattern assertions are exempt).** **All cargo
invocations go through the queue (the `cargo` on PATH IS the queue shim).
Do not invoke cargo by absolute path; do not unset the shim.** Scoped
checks only: `cargo check -p truck-certified` and `cargo test -p
truck-certified --test construct_loft_strips`. No workspace builds. The
`pub mod loft_strips;` line in `construct/mod.rs` is the DESIGNED one-line
conflict. COMMIT BEFORE writing RESULT.json AT THE WORKTREE ROOT.

Stop conditions: (1) read `entity_id.rs` first and use `Op`/`OpParams`
exactly as landed â€” the FNV-1a identity derivation is the P6 mechanism, do
not invent a parallel id scheme; (2) if the landed nurbs `cut`/`add_knot`
ops cannot produce the exact split CC-010's compatibility output requires,
file QUESTION.md with the failing case; (3) r = 1 (no splits) must
degenerate cleanly to a single open-style strip (last test) â€” if the strip
machinery needs a special case for it, that is fine; if it needs a
different algorithm, stop and file QUESTION.md.
