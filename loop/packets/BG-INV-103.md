# WORK PACKET BG-INV-103 — invariant checker 3: Euler–Poincaré

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-INV-103","status":"DONE","contracts":["BG-INV-103"],
 "tests_added":4,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. **If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-INV-103
contract:    [BG-INV-001]
class:       mechanical
crates:      [truck-topology]
write_allow:
  - vendor/truck/truck-topology/src/invariants/euler_poincare.rs
read_allow:
  - vendor/truck/truck-topology/src/invariants/mod.rs
  - vendor/truck/truck-topology/src/shell.rs
  - vendor/truck/truck-topology/src/edge.rs
  - vendor/truck/truck-topology/src/vertex.rs
  - vendor/truck/truck-topology/src/lib.rs
  - vendor/truck/truck-base/src/evidence.rs
tests_required:
  - euler_poincare_closed_cube_holds
  - euler_poincare_two_components_each_hold
  - euler_poincare_odd_counts_violate
  - euler_poincare_never_substitutes_for_vertex_link
budget:      {turns: 30, ctx_tokens: 80000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub mod euler_poincare' vendor/truck/truck-topology/src/invariants/mod.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub mod invariants' vendor/truck/truck-topology/src/lib.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub fn edge_iter' vendor/truck/truck-topology/src/shell.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub fn vertex_iter' vendor/truck/truck-topology/src/shell.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'pub fn face_iter(&self)' vendor/truck/truck-topology/src/shell.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub fn connected_components' vendor/truck/truck-topology/src/shell.rs"}
  - {id: A7, expect: 1, cmd: "grep -c 'EulerPoincare' vendor/truck/truck-base/src/evidence.rs"}
  - {id: A8, expect: 0, cmd: "grep -c 'pub fn' vendor/truck/truck-topology/src/invariants/euler_poincare.rs"}
```

(A8 pins the scaffold as EMPTY; `grep -c` exits 1 on zero matches, which IS
the expected count. A3 counts 2 if `edge_iter_mut` prefix-matches — the
anchor pattern `pub fn edge_iter` matches both; report what you see, the
load-bearing claim is that BOTH exist.)

## Problem

§1.1 invariant 3: the Euler–Poincaré relation — for a closed orientable
surface, χ = V − E + F = 2s − 2g, in particular **even**, per connected
component. truck has no Euler function; the counts are available from the
shell's own iterators — with two traps this packet pre-decides:

- `Shell::edge_iter()` yields every edge HANDLE per face (an interior edge
  appears twice) and `Shell::vertex_iter()` yields every edge's FRONT vertex
  (many duplicates). Both must be deduplicated **by id** (`Edge::id()`,
  `Vertex::id()`) before counting.
- The relation is per **connected component** (`connected_components()`),
  not per shell: two disjoint spheres in one `Shell` value have χ = 4 total,
  which is even anyway — but the per-component form is the invariant.

**Two documented truths, do not fight them:** (1) on consistently-built
shells the parity is a theorem — the checker is a regression net over the
counting machinery itself, and its violator test therefore exercises the
pure counting core on synthetic odd counts. (2) The relation is **never a
substitute for the vertex-link invariant** — a pinch point satisfies Euler–
Poincaré while the vertex link is not a single cycle. The doc comment and
one test pin both.

The checkers module tree is already scaffolded and declared — read
`invariants/mod.rs` first; its module docs are the contract your module
implements. **Only `euler_poincare.rs` is yours.**

## Decisions already made for you

1. **The two public functions, verbatim:**

   ```rust
   use crate::shell::Shell;
   use std::collections::HashSet;
   use truck_base::evidence::{
       Budget, Certificate, Certified, ContradictionWitness, Margin, Method,
       Modulus, Outcome, Prop, PropMap, Refusal, Truth,
   };

   /// BG-INV-103's counting core, pure: χ = v − e + f must be even (the
   /// Euler–Poincaré characteristic of any closed orientable component is
   /// 2s − 2g). Exposed so the parity logic is testable against synthetic
   /// violator counts — on consistently built shells the parity is a
   /// theorem, and this checker's job is to catch counting-machinery
   /// regressions, not to out-think topology.
   pub fn check_counts(vertices: usize, edges: usize, faces: usize) -> Outcome<()> {
   ```

   Body: `(vertices + edges + faces) % 2 == (vertices + faces) % 2` is
   vacuous — compute `chi = vertices as i64 - edges as i64 + faces as i64`
   and require `chi % 2 == 0`. Holds → the certificate of decision 2 with
   `Prop::EulerPoincare`; odd → the violation of decision 3.

   ```rust
   /// BG-INV-103: Euler–Poincaré (§1.1 invariant 3) over a shell's
   /// connected components.
   ///
   /// Counts DISTINCT vertices and edges by id per component (the shell's
   /// `edge_iter`/`vertex_iter` yield duplicates) and faces directly.
   /// **Never a substitute for the vertex-link invariant (BG-INV-102): a
   /// pinch point satisfies Euler–Poincaré while its vertex link is not a
   /// single cycle.** On consistently built shells the parity is a theorem;
   /// this checker is the regression net for the counting machinery.
   pub fn check<P, C, S>(shell: &Shell<P, C, S>) -> Outcome<()>
   ```

   Body: for each component of `shell.connected_components()`, count
   `v` = `component.vertex_iter().map(|x| x.id()).collect::<HashSet<_>>().len()`,
   `e` = the same over `edge_iter` with `Edge::id()`, `f` =
   `component.face_iter().count()`, and run `check_counts(v, e, f)`; the
   FIRST refusal is returned; all hold → ONE certificate (not one per
   component).

2. **The holds certificate** — the house structural pattern:
   `props.set(Prop::EulerPoincare, Truth::True)`, `method: Method::None`,
   `budget_left: Budget::new(0, 0, 0)`, `margin: Margin::UNBOUNDED`,
   `modulus: Modulus::Unbounded`.

3. **The violation refusal, verbatim:**

   ```rust
   Err(Refusal::Contradictory(ContradictionWitness {
       prop: Prop::EulerPoincare,
       left: Truth::True,
       right: Truth::False,
   }))
   ```

4. **Tests** — one `#[cfg(test)]` module opening with
   `#![deny(clippy::unwrap_used)]`, `use super::*;` and
   `use truck_topology::*;`:

   - `euler_poincare_closed_cube_holds` — the `Closed` doctest witness from
     `shell.rs` (V=8, E=12, F=6, χ=2): `check` is `Ok` with
     `props.get(Prop::EulerPoincare) == Truth::True`.
   - `euler_poincare_two_components_each_hold` — build two disjoint
     closed shells, `Shell::append` them into one value (or construct via
     `collect` of both faces lists): `check` is `Ok` (each component χ=2);
     the test ALSO asserts the component count via
     `connected_components().len() == 2` so the per-component path is
     exercised, not bypassed.
   - `euler_poincare_odd_counts_violate` — **the violator**: `check_counts`
     on synthetic odd-χ triples (`(3, 3, 1)` → χ=1; `(5, 4, 2)` → χ=3)
     returns `Err(Refusal::Contradictory(w))` with
     `w.prop == Prop::EulerPoincare`; even triples (`(8, 12, 6)`, `(2, 3, 3)`)
     return `Ok`.
   - `euler_poincare_never_substitutes_for_vertex_link` — the pin: build
     the singular-vertex witness from `singular_vertices`' doctest (the
     one whose `v[0]` link is not a cycle) and assert `check` on it STILL
     RETURNS `Ok` if its χ is even (count first; if the doc witness's χ is
     odd, adjust the witness minimally so χ is even while `v[0]` stays
     singular) — with a comment naming the pinch-point truth: Euler–
     Poincaré passing does NOT clear the vertex link.

5. One doctest on `check`: the cube witness, `is_ok()`.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N` literal unless that line ends with an `// H-3` comment. This packet
uses no float literals at all — counts are `usize`. Run
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
116 doctests, zero clippy findings, measured at HEAD 4917c55); your bar is
everything above stays green plus your four tests and one doctest.

## Forbidden

Editing any file outside `write_allow`. Changing the refusal shape or the
certificate fields of decisions 2-3 (the wave's seven checkers share them).
Counting edges or vertices WITHOUT id-deduplication. Adding `#[ignore]`.
Adding `unwrap()`/`expect()` outside the test module.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- `connected_components` does not return `Vec<Shell<P, C, S>>` as this packet
  assumes → adapt the call, do not stop; but if its semantics differ from
  "face-connected components of the shell", that is a `SPEC_GAP`
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(topology): Euler–Poincaré invariant checker (BG-INV-103)`.
