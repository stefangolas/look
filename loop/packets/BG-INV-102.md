# WORK PACKET BG-INV-102 — invariant checker 2: vertex link is a single cycle

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-INV-102","status":"DONE","contracts":["BG-INV-102"],
 "tests_added":4,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. **If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-INV-102
contract:    [BG-INV-001]
class:       mechanical
crates:      [truck-topology]
write_allow:
  - vendor/truck/truck-topology/src/invariants/vertex_link.rs
read_allow:
  - vendor/truck/truck-topology/src/invariants/mod.rs
  - vendor/truck/truck-topology/src/shell.rs
  - vendor/truck/truck-topology/src/lib.rs
  - vendor/truck/truck-base/src/evidence.rs
tests_required:
  - vertex_link_regular_shell_holds
  - vertex_link_singular_vertex_violates
  - vertex_link_documented_dependency_on_closed
  - vertex_link_certificate_names_the_invariant
budget:      {turns: 30, ctx_tokens: 80000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub mod vertex_link' vendor/truck/truck-topology/src/invariants/mod.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub mod invariants' vendor/truck/truck-topology/src/lib.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub fn singular_vertices' vendor/truck/truck-topology/src/shell.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'VertexLink' vendor/truck/truck-base/src/evidence.rs"}
  - {id: A5, expect: 0, cmd: "grep -c 'pub fn' vendor/truck/truck-topology/src/invariants/vertex_link.rs"}
```

(A5 pins the scaffold as EMPTY; `grep -c` exits 1 on zero matches, which IS
the expected count.)

## Problem

§1.1 invariant 2: around every vertex of a valid solid boundary, the edge
uses form a single cycle. truck already computes the violation:
`Shell::singular_vertices()` (shell.rs:536) returns the vertices whose links
are NOT a single cycle. The invariant checker wraps it in the evidence
algebra.

**The load-bearing subtlety — do not "simplify" it away:** the link argument
is valid GIVEN the shell is closed. `singular_vertices` tests link
connectivity; on a `Closed` shell (every edge paired, opposite sense) every
link node then has degree 2, so connectivity implies the single cycle. On an
OPEN shell the implication fails — a link can be one path, not one cycle,
without any vertex being singular. The checker therefore documents the
dependency on `Closed` in its doc comment and its `vertex_link_documented_
dependency_on_closed` test pins it. This packet does NOT pre-check closure
itself (that is BG-INV-101's checker, a sibling module); the doc comment
states the precondition plainly.

The checkers module tree is already scaffolded and declared — read
`invariants/mod.rs` first; its module docs are the contract your module
implements. **Only `vertex_link.rs` is yours; everything else is read-only.**

## Decisions already made for you

1. **The public API, verbatim:**

   ```rust
   use crate::shell::Shell;
   use truck_base::evidence::{
       Budget, Certificate, Certified, ContradictionWitness, Margin, Method,
       Modulus, Outcome, Prop, PropMap, Refusal, Truth,
   };

   /// BG-INV-102: vertex link is a single cycle (§1.1 invariant 2).
   ///
   /// Wraps `Shell::singular_vertices()`. **The single-cycle conclusion is
   /// valid only on a closed shell**: `singular_vertices` tests link
   /// CONNECTIVITY, and the "every link node has degree 2" step comes from
   /// coedge pairing (BG-INV-101). On an open shell a connected link may be
   /// a path rather than a cycle. Callers should run BG-INV-101 first, or
   /// accept that this checker certifies link connectivity only.
   /// Localisation: the violating vertices are `singular_vertices()`'s own
   /// return value.
   pub fn check<P, C>(shell: &Shell<P, C>) -> Outcome<()> {
   ```

   The body: `if shell.singular_vertices().is_empty()` → the certificate of
   decision 2, else the violation of decision 3.

2. **The holds certificate** — the house structural pattern, exactly:
   `PropMap::new()` with `props.set(Prop::VertexLink, Truth::True)`,
   `method: Method::None`, `budget_left: Budget::new(0, 0, 0)`,
   `margin: Margin::UNBOUNDED`, `modulus: Modulus::Unbounded`.

3. **The violation refusal, verbatim:**

   ```rust
   Err(Refusal::Contradictory(ContradictionWitness {
       prop: Prop::VertexLink,
       left: Truth::True,
       right: Truth::False,
   }))
   ```

4. **Tests** — one `#[cfg(test)]` module opening with
   `#![deny(clippy::unwrap_used)]`, `use super::*;` and
   `use truck_topology::*;`. Copy the witnesses from `singular_vertices`' own
   doctests in `shell.rs` (the Regular 6-vertex/9-edge construction at
   shell.rs:482-499 and the singular variant at :499-536 which asserts
   `shell.singular_vertices() == vec![v[0].clone()]`):

   - `vertex_link_regular_shell_holds` — the doctest's regular shell:
     `check` is `Ok` with `props.get(Prop::VertexLink) == Truth::True`.
   - `vertex_link_singular_vertex_violates` — the doctest's singular shell
     (same construction minus the faces that would pair `v[0]`'s edges — the
     doctest shows exactly which): `Err(Refusal::Contradictory(w))` with
     `w.prop == Prop::VertexLink`, `left == Truth::True`,
     `right == Truth::False`.
   - `vertex_link_documented_dependency_on_closed` — the pin: an OPEN but
     link-connected witness (a single closed triangular wire as one face —
     every vertex link is a path, `singular_vertices` is empty) certifies
     `Ok`, and the test asserts this is exactly the documented open-shell
     limitation (a comment in the test names it; the assertion is that the
     checker does not refuse what it cannot conclude).
   - `vertex_link_certificate_names_the_invariant` — the holds case's full
     certificate shape: `method == Method::None`,
     `budget_left == Budget::new(0, 0, 0)`,
     `props.get(Prop::CoedgePairing) == Truth::Unknown` (claims ONLY its
     own property).

5. One doctest on `check`: the regular-shell witness, `is_ok()`.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N` literal unless that line ends with an `// H-3` comment. This packet
uses no float literals at all. Run `bash scripts/kernel-gates.sh` yourself
before writing `RESULT.json`.

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
Adding a closure pre-check to `check` (the dependency is DOCUMENTED, not
enforced — enforcing it would double-count BG-INV-101's refusal). Touching
`shell.rs`. Adding `#[ignore]`. Adding `unwrap()`/`expect()` outside the
test module.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- a `singular_vertices` doc example does not compile as copied → `SPEC_GAP`
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(topology): vertex-link invariant checker (BG-INV-102)`.
