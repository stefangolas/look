# CC-013-CORRESPONDENCE — L4: cyclic correspondence via separation-margin argmin

CC program Phase B (spine S9; theory §2.2 L4). Correspondence is an
orientation, an anchor, and an edge matching on an abstract oriented cyclic
complex. Resolution order is FIXED: caller anchor → unique isomorphism
(combinatorially forced) → argmin over the r cyclic shifts under a DECLARED
functional → refuse. Twist minimization is not an objective anywhere.

```yaml
id:          CC-013-CORRESPONDENCE
contract:    [CC-013-CORRESPONDENCE]
class:       design
crates:      [truck-certified]
depends_on:  [CC-000-CONTRACT, CC-003-ARGMIN, CC-012-LOFT-STRIPS]
write_allow:
  - vendor/truck/truck-certified/src/construct/correspondence.rs
  - vendor/truck/truck-certified/src/construct/mod.rs
  - vendor/truck/truck-certified/tests/construct_correspondence.rs
read_allow:
  - docs/CERTIFIED_CONSTRUCTION_CONTRACTS.md
  - vendor/truck/truck-certified/src/construct
budget:      {turns: 22, ctx_tokens: 90000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub fn argmin_margin' vendor/truck/truck-certified/src/construct/argmin.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub struct LoftStrips' vendor/truck/truck-certified/src/construct/loft_strips.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub struct WireComplex' vendor/truck/truck-certified/src/construct/stubs.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub struct ShiftFunctional' vendor/truck/truck-certified/src/construct/stubs.rs"}
tests_required:
  - anchor_supplied_wins
  - unique_isomorphism_resolves_without_argmin
  - four_arc_two_circle_shift_resolves_by_separated_argmin
  - overlapping_shift_values_refuse_ambiguous_correspondence
  - orientation_reversal_requires_explicit_caller_consent
```

Section 1: wire production — `WireComplex` (CC-000 stub, A3) gains its
PRODUCTION meaning here without changing the stub file: an oriented cyclic
sequence of matched edges with per-vertex positions as `Interval` enclosures.
`pub fn wire_complex_of(arc_count: usize, vertices: &[[Interval; 3]]) ->
Result<WireComplex, ConstructRefusal>` builds it (arc_count ≥ 2; vertex
count must equal arc_count — it is a cycle). Isomorphism between two wires
is exact combinatorics: equal arc count is the only structural requirement
(edge splitting already happened upstream; this module never splits).

Section 2: the resolver per spine S9 — `pub fn resolve_correspondence(wire:
&WireComplex, sections: &[WireComplex], functional: &ShiftFunctional) ->
Result<Correspondence, ConstructRefusal>` walks the fixed order: (1) an
anchor in `ShiftFunctional` → return immediately with that anchor and
forward orientation (test `anchor_supplied_wins` — the anchor is NEVER
second-guessed); (2) if the combinatorial data forces a unique isomorphism
(r = 2, or a labeled asymmetric vertex pattern supplied by the caller),
return it without invoking the argmin (test 2); (3) otherwise compute, for
EACH of the r cyclic shifts, the declared functional as an `Interval`
enclosure — the v1 functional is pre-made: sum of squared distances between
matched vertices, accumulated in index order over interval arithmetic
(`ShiftFunctional::VertexSumSq`; any other functional is a later amendment,
the enum lives in the stub and stays closed) — and pass the r enclosures to
`argmin_margin` (A1). Strict separation → that shift; overlap →
`Err(AmbiguousCorrespondence)` (test 4: the two-circle-four-arc case is
four-fold ambiguous and MUST resolve by separation or refuse — never by
proximity). `Correspondence` is the CC-000 stub type; here it is CONSUMED,
not defined — read `stubs.rs` first.

Section 3: orientation — an orientation-reversing match is only ever taken
when the caller supplied it explicitly in the anchor; the automatic path is
orientation-preserving only (test 5). This is pre-made, not a heuristic
guard: the isomorphism searched in step 3 ranges over cyclic shifts of the
FORWARD orientation.

House rules: **H-1: no `unwrap`/`expect`/`panic!` in shipped code, no
module-level `allow`.** **H-3: float comparisons in tests take the `// H-3`
opt-out ON THE SAME LINE.** **All cargo invocations go through the queue
(the `cargo` on PATH IS the queue shim). Do not invoke cargo by absolute
path; do not unset the shim.** Scoped checks only: `cargo check -p
truck-certified` and `cargo test -p truck-certified --test
construct_correspondence`. No workspace builds. The `pub mod
correspondence;` line in `construct/mod.rs` is the DESIGNED one-line
conflict. COMMIT BEFORE writing RESULT.json AT THE WORKTREE ROOT.

Stop conditions: (1) `WireComplex`/`ShiftFunctional`/`Correspondence`
stubs own the type surface — extend nothing in `stubs.rs`; if a field you
need is missing, file QUESTION.md (spine seam defect); (2) the functional
is DECLARED, and its declaration rides with the construction identity —
record in RESULT notes which functional value each test fixture produced;
(3) r = 2 must resolve in step 2 without argmin — if the combinatorial
uniqueness argument for r = 2 fails on a fixture, that is a theory seam:
file QUESTION.md, do not silently route it through step 3.
