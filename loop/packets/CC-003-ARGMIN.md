# CC-003-ARGMIN â€” P4: argmin-with-margin operator

CC program Phase A (spine S5). Theory:
`docs/CERTIFIED_LOFT_AND_SHELL_THEORY_SPEC.md` Â§1 P4. The operator
certifies STRICT SEPARATION, never intent: return i\* only if
sup[Î»_i\*] &lt; inf[Î»_j] for all j â‰  i\*; overlap â†’ typed refusal.
Consumers: cyclic correspondence disambiguation (CC-013), thickness event
selection (CC-026), blend event ordering (CC-030).

```yaml
id:          CC-003-ARGMIN
contract:    [CC-003-ARGMIN]
class:       mechanical
crates:      [truck-certified]
depends_on:  [CC-000-CONTRACT]
write_allow:
  - vendor/truck/truck-certified/src/construct/argmin.rs
  - vendor/truck/truck-certified/src/construct/mod.rs
  - vendor/truck/truck-certified/tests/construct_argmin.rs
read_allow:
  - docs/CERTIFIED_CONSTRUCTION_CONTRACTS.md
  - vendor/truck/truck-certified/src/construct
budget:      {turns: 12, ctx_tokens: 60000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub type Interval' vendor/truck/truck-certified/src/kernel/mod.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn argmin_separated' vendor/truck/truck-certified/src/construct/fixtures.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub fn argmin_overlapping' vendor/truck/truck-certified/src/construct/fixtures.rs"}
tests_required:
  - strictly_separated_enclosures_select_the_unique_minimizer
  - overlapping_enclosures_refuse_ambiguous_event_ordering
  - empty_input_refuses_invalid_input
  - single_element_returns_zero
  - tie_is_refused_never_broken_by_value_comparison
```

Section 1: `construct/argmin.rs` â€” `pub fn argmin_margin(enclosures:
&[Interval]) -> Result<usize, ConstructRefusal>` per spine S5. Semantics,
pre-made: empty slice â†’ `Err(ConstructRefusal::InvalidInput)`; scan once in
index order (deterministic; no reordering, no sorting); the candidate i\*
is the index whose enclosure's UPPER bound is smallest (strict `<`
comparison of sup values â€” a tie in sup values among distinct indices is
itself ambiguous); after selecting i\*, verify `sup[i\*] < inf[j]` for ALL
j â‰  i\*; any violation â†’ `Err(ConstructRefusal::AmbiguousEventOrdering)`.
There is NO tie-breaking by value, no epsilon slack, no "closest wins":
overlap refuses. NaN or non-finite enclosure bounds â†’ `InvalidInput`.

Section 2: doc comment carries the theory Â§1 P4 contract verbatim in
substance: the operator certifies strict separation; callers use it to
disambiguate cyclic shifts, event orderings, and thickness candidates, and
every consumer must handle the refusal as a typed outcome, never as a
fallback heuristic.

House rules: **H-1: no `unwrap`/`expect`/`panic!` in shipped code, no
module-level `allow`.** **H-3: float comparisons in tests take the `// H-3`
opt-out ON THE SAME LINE.** **All cargo invocations go through the queue
(the `cargo` on PATH IS the queue shim). Do not invoke cargo by absolute
path; do not unset the shim.** Scoped checks only: `cargo check -p
truck-certified` and `cargo test -p truck-certified --test
construct_argmin`. No workspace builds. The `pub mod argmin;` line in
`construct/mod.rs` is the DESIGNED one-line conflict. COMMIT BEFORE writing
RESULT.json AT THE WORKTREE ROOT.

Stop conditions: (1) the two CC-000 fixtures (`argmin_separated`,
`argmin_overlapping`) are the required inputs for the first two tests; if
their ground truths do not hold, file QUESTION.md (CC-000 defect), do not
bend the fixture; (2) this packet is deliberately tiny â€” if it grows past
~120 lines of production code, the design is wrong: re-read spine S5.
