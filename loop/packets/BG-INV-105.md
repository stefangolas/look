# WORK PACKET BG-INV-105 — invariant checker 5: domain–boundary correspondence

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-INV-105","status":"DONE","contracts":["BG-INV-105"],
 "tests_added":4,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. **If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-INV-105
contract:    [BG-INV-001]
class:       mechanical
crates:      [truck-topology]
write_allow:
  - vendor/truck/truck-topology/src/invariants/domain_boundary.rs
read_allow:
  - vendor/truck/truck-topology/src/invariants/mod.rs
  - vendor/truck/truck-topology/src/face.rs
  - vendor/truck/truck-topology/src/wire.rs
  - vendor/truck/truck-topology/src/lib.rs
  - vendor/truck/truck-base/src/evidence.rs
tests_required:
  - domain_boundary_closed_wires_hold
  - domain_boundary_open_wire_violates
  - domain_boundary_no_boundaries_violates
  - domain_boundary_certificate_names_the_invariant
budget:      {turns: 30, ctx_tokens: 80000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub mod domain_boundary' vendor/truck/truck-topology/src/invariants/mod.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub mod invariants' vendor/truck/truck-topology/src/lib.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub fn boundaries' vendor/truck/truck-topology/src/face.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub fn is_closed' vendor/truck/truck-topology/src/wire.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'DomainBoundary' vendor/truck/truck-base/src/evidence.rs"}
  - {id: A6, expect: 0, cmd: "grep -c 'pub fn' vendor/truck/truck-topology/src/invariants/domain_boundary.rs"}
```

(A6 pins the scaffold as EMPTY; `grep -c` exits 1 on zero matches, which IS
the expected count.)

## Problem

§1.1 invariant 5: a face's domain boundary and its boundary wires
correspond — the boundary of the face's parameter domain is traced, edge
use by edge use, by the face's boundary wires. The FULL invariant needs
pcurves wired into faces (BG-CE-001 landed the payload; nothing attaches
real pcurves yet), so this packet lands the **checkable core**: the
topological half — every boundary wire is a CLOSED loop, and a face has at
least one boundary wire. An open wire cannot correspond to any domain
boundary (the domain boundary is a union of closed loops); a face with no
boundary wires claims a bounded domain with no boundary at all. The
pcurve-carrying half is documented as waiting on pcurve wiring, in the
checker's doc comment.

The checkers module tree is already scaffolded and declared — read
`invariants/mod.rs` first. **Only `domain_boundary.rs` is yours.**

## Decisions already made for you

1. **The public API, verbatim:**

   ```rust
   use crate::Face;
   use truck_base::evidence::{
       Budget, Certificate, Certified, ContradictionWitness, Margin, Method,
       Modulus, Outcome, Prop, PropMap, Refusal, Truth,
   };

   /// BG-INV-105: domain–boundary correspondence (§1.1 invariant 5), the
   /// topological core: every boundary wire of `face` is a closed loop and
   /// the face has at least one.
   ///
   /// The FULL invariant — the wires tracing the parameter domain's
   /// boundary, edge use by edge use — needs pcurves attached to the edge
   /// uses (BG-CE-001's `PC` payload, still unwired in this tree) and is
   /// NOT checked here; this checker certifies the topological half only.
   /// Localise a violation with the wire index: the refusal's `prop` names
   /// the invariant; the offending wire is the first index for which
   /// `is_closed()` is false (or index 0 when there are no wires at all).
   pub fn check<P, C, S>(face: &Face<P, C, S>) -> Outcome<()> {
   ```

   The body: `face.boundaries()` is empty → the violation of decision 3;
   else if any wire `!is_closed()` → the violation; else the certificate of
   decision 2.

2. **The holds certificate** — the house structural pattern:
   `props.set(Prop::DomainBoundary, Truth::True)`, `method: Method::None`,
   `budget_left: Budget::new(0, 0, 0)`, `margin: Margin::UNBOUNDED`,
   `modulus: Modulus::Unbounded`.

3. **The violation refusal, verbatim:**

   ```rust
   Err(Refusal::Contradictory(ContradictionWitness {
       prop: Prop::DomainBoundary,
       left: Truth::True,
       right: Truth::False,
   }))
   ```

4. **Tests** — one `#[cfg(test)]` module opening with
   `#![deny(clippy::unwrap_used)]`, `use super::*;` and
   `use truck_topology::*;`. Build the witnesses with `P = C = S = ()` —
   `Vertex::news`, `Edge::new`, `wire!`/`Wire::from`, `Face::new`:

   - `domain_boundary_closed_wires_hold` — a triangle face (three vertices,
     three edges, one closed wire): `Ok` with
     `props.get(Prop::DomainBoundary) == Truth::True`.
   - `domain_boundary_open_wire_violates` — a face whose single wire is two
     edges that do NOT loop (v0→v1, v1→v2 — the last edge's back is v2, the
     first edge's front is v0, so `is_closed()` is false): the violation
     refusal, `w.prop == Prop::DomainBoundary`, `w.left == Truth::True`,
     `w.right == Truth::False`.
   - `domain_boundary_no_boundaries_violates` — `Face::new(vec![], ())`:
     the violation refusal.
   - `domain_boundary_certificate_names_the_invariant` — the holds case's
     full certificate: `method == Method::None`,
     `budget_left == Budget::new(0, 0, 0)`,
     `props.get(Prop::CoedgePairing) == Truth::Unknown` (claims ONLY its
     own property).

5. One doctest on `check`: the triangle face, `is_ok()`.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N` literal unless that line ends with an `// H-3` comment. This packet
uses no float literals at all — pure topology over `()`. Run
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
everything above stays green plus your four tests and one doctest.

## Forbidden

Editing any file outside `write_allow`. Changing the refusal shape or the
certificate fields of decisions 2-3 (the wave's seven checkers share them).
Adding geometric checks beyond wire topology (the pcurve half is explicitly
not this packet's). Touching `face.rs` or `wire.rs`. Adding `#[ignore]`.
Adding `unwrap()`/`expect()` outside the test module.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- `Wire::is_closed()`'s semantics differ from "the wire's edges connect
  head-to-tail into a loop" (verify on the `wire!` doc examples) →
  `SPEC_GAP`, with the example and what you measured
- `Face::new(vec![], ())` panics rather than constructing an empty-boundary
  face → `SPEC_GAP` (the no-boundaries test needs it constructible)
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(topology): domain–boundary invariant checker (BG-INV-105)`.
