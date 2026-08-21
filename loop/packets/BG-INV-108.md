# WORK PACKET BG-INV-108 — invariant checker 8: shell nesting is a forest

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md**
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-INV-108","status":"DONE","contracts":["BG-INV-108"],
 "tests_added":6,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. **If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-INV-108
contract:    [BG-INV-001]
class:       design
crates:      [truck-topology]
write_allow:
  - vendor/truck/truck-topology/src/invariants/shell_nesting.rs
read_allow:
  - vendor/truck/truck-topology/src/invariants/mod.rs
  - vendor/truck/truck-topology/src/shell.rs
  - vendor/truck/truck-topology/src/solid.rs
  - vendor/truck/truck-topology/src/lib.rs
  - vendor/truck/truck-base/src/evidence.rs
tests_required:
  - nesting_disjoint_components_are_two_solids
  - nesting_nested_component_is_an_inner_shell
  - nesting_three_levels_yield_two_solids
  - nesting_containment_cycle_is_contradictory
  - nesting_undecided_pair_is_unresolved
  - nesting_antiparallel_pair_is_nested
budget:      {turns: 35, ctx_tokens: 90000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub mod shell_nesting' vendor/truck/truck-topology/src/invariants/mod.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub mod invariants' vendor/truck/truck-topology/src/lib.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'ShellNesting' vendor/truck/truck-base/src/evidence.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub fn new' vendor/truck/truck-topology/src/solid.rs"}
  - {id: A5, expect: 0, cmd: "grep -c 'pub fn' vendor/truck/truck-topology/src/invariants/shell_nesting.rs"}
```

(A5 pins the scaffold as EMPTY; `grep -c` exits 1 on zero matches, which IS
the expected count.)

## Problem

§1.1 invariant 8 / audit F-1: the containment order of a solid's boundary
shell components must be a **nesting forest** — antisymmetric (a cycle is a
contradiction), and each maximal component is one solid whose immediate
children are its inner (cavity) shells. Today `Solid::new(
connected_components())` packs every component into one solid, declaring
disjoint lumps to be cavities — F-1.

The inside query ("is component C's witness point inside shell D?") is
**not yet certified** — the certified winding is BG-NUM-004's, unwritten.
So this checker is **pure**: it takes the containment relation as an
injected oracle and certifies the GRAPH — exactly the part that is
topology, not geometry. The production oracle is a thin adapter when
NUM-004 lands; the wiring that turns the forest into `Vec<Solid>` for the
boolean operators is the NUM-004 wiring packet's job (a partition without
a certified oracle would mis-split nested cavity components — the reason
the and/or signature break was deferred; the spec records this).

The checkers module tree is already scaffolded and declared — read
`invariants/mod.rs` first. **Only `shell_nesting.rs` is yours.**

## Decisions already made for you

1. **The public API, verbatim:**

   ```rust
   use truck_base::evidence::{
       Budget, Certificate, Certified, ContradictionWitness, Margin, Method,
       Modulus, Outcome, Prop, PropMap, Refusal, Truth, UnresolvedWitness,
   };

   /// The containment oracle: `contains(i, j)` answers whether component
   /// `i`'s witness point lies strictly inside component `j`.
   /// `Some(true)` / `Some(false)` are certified answers; `None` is
   /// undecided. The production implementation is BG-NUM-004's certified
   /// winding; tests inject hand-built answers.
   pub type Contains = dyn Fn(usize, usize) -> Option<bool>;

   /// BG-INV-108: shell nesting is a forest (§1.1 invariant 8, audit F-1).
   ///
   /// Given `n` connected shell components and a containment oracle over
   /// them, certifies the containment relation is a strict partial order
   /// (a cycle — including the two-cycle of mutual containment — is
   /// `Contradictory` with `Prop::ShellNesting`) and returns the solid
   /// partition: one entry `(outer, inner_shells)` per SOLID — the
   /// even-depth components are solids, each with its immediately
   /// contained (odd-depth) components as inner shells. A component at
   /// depth 2 — inside a cavity — is its own solid again (the solid ⊃
   /// void ⊃ solid case yields two solids).
   ///
   /// Any `None` from the oracle is `NumericallyUnresolved`
   /// (`UncertifiedContainment`): an undecided pair cannot be classified
   /// either way, and an honest refusal beats a guess. This checker is
   /// pure graph logic — the geometry lives in the oracle.
   pub fn nesting_forest(n: usize, contains: &Contains) -> Outcome<Vec<(usize, Vec<usize>)>> {
   ```

2. **The algorithm, step by step:**

   - **Query all pairs.** For every ordered pair `(i, j)`, `i ≠ j`, call
     the oracle ONCE and cache the answer (`n ≤ small` in practice; the
     cache exists so the oracle is called at most once per pair — an
     oracle may be expensive). Any `None` → `Err(Refusal::
     NumericallyUnresolved { spent: Budget::new(0, 0, 0), witness:
     UnresolvedWitness::UncertifiedContainment })`.
   - **Cycle detection.** The `Some(true)` edges form a digraph; run a
     DFS (or Kahn's algorithm) — a cycle → the violation of decision 4.
     The two-cycle `contains(i, j) == contains(j, i) == Some(true)` is
     the common case (mutual containment); longer cycles are the same
     refusal.
   - **Depths.** In the DAG, each component's **nesting depth** = the
     number of components transitively containing it (`contains(k, i) ==
     Some(true)` for that many distinct `k`). Compute by processing
     components in topological order from the outermost (contained by
     nothing): `depth(i) = 1 + max(depth(j))` over the `j` with
     `contains(j, i) == Some(true)`; components contained by nothing have
     depth 0.
   - **Immediate containment.** `j` is `i`'s immediate child iff
     `contains(i, j) == Some(true)` and there is no `k` with
     `contains(i, k) == Some(true)` and `contains(k, j) == Some(true)`
     (the transitive reduction).
   - **The partition.** Every EVEN-depth component `c` is a solid:
     `(c, immediate children of c)` — the children are odd-depth by
     construction and are `c`'s inner shells. Sort the return by outer
     component index for determinism; sort each children list too.

3. **The holds certificate** — the house structural pattern, wrapping the
   partition: `props.set(Prop::ShellNesting, Truth::True)`, `method:
   Method::None` (pure graph logic, no arithmetic), `budget_left:
   Budget::new(0, 0, 0)`, `margin: Margin::UNBOUNDED`, `modulus:
   Modulus::Unbounded`.

4. **The violation refusal, verbatim:**

   ```rust
   Err(Refusal::Contradictory(ContradictionWitness {
       prop: Prop::ShellNesting,
       left: Truth::True,
       right: Truth::False,
   }))
   ```

5. **Tests** — one `#[cfg(test)]` module opening with
   `#![deny(clippy::unwrap_used)]` (H-1 justification comment) and
   `use super::*;`. All witnesses are hand-built oracles (closures over
   tables) — no geometry:

   - `nesting_disjoint_components_are_two_solids` — n=3, every pair
     `Some(false)`: `Ok` with `[(0, []), (1, []), (2, [])]` (three solids,
     no inner shells) — and `cert.value.len() == 3`.
   - `nesting_nested_component_is_an_inner_shell` — n=2,
     `contains(1, 0) == Some(true)` (1 inside 0), the inverse
     `Some(false)`: `Ok` with `[(0, [1])]`.
   - `nesting_three_levels_yield_two_solids` — n=3 with 2 ⊂ 1 ⊂ 0 (the
     oracle is TRANSITIVE: `contains(2, 0) == Some(true)` too): `Ok` with
     `[(0, [1]), (2, [])]` — component 2, at depth 2, is its own solid
     with NO inner shells (it has no immediate odd-depth children). This
     is the spec's three-level test at the pure-graph level.
   - `nesting_containment_cycle_is_contradictory` — n=2 with mutual
     `Some(true)`: the violation refusal, `w.prop ==
     Prop::ShellNesting`. Also a 3-cycle (0→1→2→0): the same refusal.
   - `nesting_undecided_pair_is_unresolved` — n=2 with `contains(1, 0)`
     returning `None`: `Err(Refusal::NumericallyUnresolved { … })` with
     `witness == UnresolvedWitness::UncertifiedContainment`.
   - `nesting_antiparallel_pair_is_nested` — n=2 with `contains(1, 0) ==
     Some(true)` and `contains(0, 1) == Some(false)`: the same nested
     result as the second test (the antiparallel pair is a legal
     containment — only BOTH-true is a cycle).

   Build the oracle tables as small closures, e.g.

   ```rust
   let contains = |i: usize, j: usize| match (i, j) {
       (1, 0) => Some(true),
       (0, 1) => Some(false),
       _ => Some(false),
   };
   ```

   matching the `Contains` type exactly.

6. One doctest on `nesting_forest`: the nested pair (n=2), asserting the
   returned partition is `[(0, [1])]` and the verdict is `Ok`.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N` literal unless that line ends with an `// H-3` comment. This packet
uses no float literals at all — pure graph logic. Run
`bash scripts/kernel-gates.sh` yourself before writing `RESULT.json`.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-topology
cargo clippy -p truck-topology --all-targets --no-deps
cargo test -p truck-topology --lib --tests --no-fail-fast
cargo test -p truck-topology --doc
cargo check --workspace --all-targets
bash scripts/kernel-gates.sh <your base commit>
```

Never run a bare `cargo test`. The crate is clean at baseline (all tests,
116 doctests, zero clippy findings, measured at HEAD 49997d3); your bar is
everything above stays green plus your six tests and one doctest.

## Forbidden

Editing any file outside `write_allow`. Calling the oracle more than once
per ordered pair (the cache of decision 2 is required). Deciding anything
from a `None` answer (undecided is undecided — `NumericallyUnresolved`).
Changing the refusal shapes or the certificate fields of decisions 3-4
(the wave's seven checkers share them). Returning unsorted partitions.
Touching `solid.rs`. Adding `#[ignore]`. Adding `unwrap()`/`expect()`
outside the test module.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- the depth/partition algebra of decision 2 produces a different partition
  than the tests of decision 5 assert for the same oracle — TRUST THE
  TESTS' algebra (it was derived by hand from the spec's three-level case)
  and report the disagreement in `disagreements`, implementing the tests'
  version
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(topology): shell-nesting forest checker (BG-INV-108)`.
