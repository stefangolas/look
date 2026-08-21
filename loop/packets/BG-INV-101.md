# WORK PACKET BG-INV-101 — invariant checker 1: coedge pairing

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-INV-101","status":"DONE","contracts":["BG-INV-101"],
 "tests_added":4,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. **If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-INV-101
contract:    [BG-INV-001]
class:       mechanical
crates:      [truck-topology]
write_allow:
  - vendor/truck/truck-topology/src/invariants/coedge_pairing.rs
read_allow:
  - vendor/truck/truck-topology/src/invariants/mod.rs
  - vendor/truck/truck-topology/src/shell.rs
  - vendor/truck/truck-topology/src/lib.rs
  - vendor/truck/truck-base/src/evidence.rs
tests_required:
  - coedge_pairing_closed_shell_holds
  - coedge_pairing_three_faces_one_edge_violates
  - coedge_pairing_open_boundary_violates
  - coedge_pairing_certificate_names_the_invariant
budget:      {turns: 30, ctx_tokens: 80000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub mod coedge_pairing' vendor/truck/truck-topology/src/invariants/mod.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub mod invariants' vendor/truck/truck-topology/src/lib.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub fn shell_condition' vendor/truck/truck-topology/src/shell.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub enum ShellCondition' vendor/truck/truck-topology/src/shell.rs"}
  - {id: A5, expect: 4, cmd: "grep -c '^    Irregular,$\\|^    Regular,$\\|^    Oriented,$\\|^    Closed,$' vendor/truck/truck-topology/src/shell.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'CoedgePairing' vendor/truck/truck-base/src/evidence.rs"}
  - {id: A7, expect: 0, cmd: "grep -c 'pub fn' vendor/truck/truck-topology/src/invariants/coedge_pairing.rs"}
```

(A7 pins the scaffold as EMPTY — no public function yet; `grep -c` exits 1 on
zero matches, which IS the expected count. A5 counts the four variant
DECLARATIONS — the names also appear in doc prose, hence the
anchored-at-line-start form.)

## Problem

§1.1 invariant 1: in a valid solid boundary, every edge use pairs — each
non-degenerate edge is used by exactly two faces with opposite orientation.
truck already computes this: `Shell::shell_condition()` (shell.rs:145) walks
the faces' boundary edges and classifies the shell `Irregular` / `Regular` /
`Oriented` / `Closed`, where `Closed` is exactly "every edge shared by two
faces, opposite sense". The invariant checker wraps that existing machinery
in the evidence algebra, so a boolean API consumer gets a certified verdict
with the invariant named in the property map instead of a bare enum.

The checkers module tree is already scaffolded and declared — read
`invariants/mod.rs` first; its module docs are the contract your module
implements: `Ok(Certified(()))` holds (the invariant's `Prop` arm set `True`
in the certificate), violation is `Err(Refusal::Contradictory(
ContradictionWitness { prop, left: Truth::True, right: Truth::False }))`,
any other `Err` means "could not decide". **`invariants/mod.rs`, `lib.rs`
and every other file are read-only** — only `coedge_pairing.rs` is yours.

## Decisions already made for you

1. **The public API, verbatim:**

   ```rust
   use crate::shell::{Shell, ShellCondition};
   use truck_base::evidence::{
       Certificate, Certified, ContradictionWitness, Outcome, Prop, Refusal, Truth,
   };

   /// BG-INV-101: coedge pairing (§1.1 invariant 1) — every edge of a solid
   /// boundary is shared by exactly two faces of opposite sense.
   ///
   /// Wraps `Shell::shell_condition()`: `Closed` holds; `Irregular` (some
   /// edge in more than two faces), `Regular` (some pair same-sense) and
   /// `Oriented` (some edge in fewer than two faces — an open boundary)
   /// all violate the pairing. The "declared even number" and "declared 1"
   /// clauses of the invariant are the CALLER's assertion about open
   /// boundaries; they cannot be checked from the shell alone and are out
   /// of scope until a topology carries the declaration. Localise a
   /// violation with the shell's own `edge_iter` — the Boundaries pass in
   /// `shell.rs` is the reference grouping.
   pub fn check<P, C>(shell: &Shell<P, C>) -> Outcome<()> {
   ```

   The body: match `shell.shell_condition()`; `Closed` → the certificate of
   decision 2; anything else → the violation of decision 3.

2. **The holds certificate**, house pattern (`PropMap::new()`, set the arm,
   `Method::None` — the verdict is structural, no arithmetic was performed;
   `budget_left: Budget::new(0, 0, 0)`, `margin: Margin::UNBOUNDED`,
   `modulus: Modulus::Unbounded`, matching every structural `Outcome`
   producer in the tree):

   ```rust
   let mut props = PropMap::new();
   props.set(Prop::CoedgePairing, Truth::True);
   Ok(Certified::new(
       (),
       Certificate {
           props,
           method: Method::None,
           budget_left: Budget::new(0, 0, 0),
           margin: Margin::UNBOUNDED,
           modulus: Modulus::Unbounded,
       },
   ))
   ```

3. **The violation refusal, verbatim:**

   ```rust
   Err(Refusal::Contradictory(ContradictionWitness {
       prop: Prop::CoedgePairing,
       left: Truth::True,
       right: Truth::False,
   }))
   ```

4. **Tests** — one `#[cfg(test)]` module opening with
   `#![deny(clippy::unwrap_used)]` (the house convention for new test
   modules), `use super::*;` and `use truck_topology::*;`. Copy the shell
   witnesses VERBATIM from the `ShellCondition` doc examples in `shell.rs`
   (they are complete, runnable constructions):

   - `coedge_pairing_closed_shell_holds` — the `Closed` doctest witness
     (the 8-vertex, 12-edge, 6-wire construction with `shell[5].invert()`):
     `check` is `Ok`, and the certificate's `props.get(Prop::CoedgePairing)`
     is `Truth::True`.
   - `coedge_pairing_three_faces_one_edge_violates` — the `Irregular`
     doctest witness (three faces sharing `edge[0]`): `check` is
     `Err(Refusal::Contradictory(w))` with `w.prop == Prop::CoedgePairing`,
     `w.left == Truth::True`, `w.right == Truth::False`.
   - `coedge_pairing_open_boundary_violates` — the `Oriented` doctest
     witness (edge[0] in only the 0th face): same refusal shape.
   - `coedge_pairing_certificate_names_the_invariant` — on the holds case,
     assert the FULL certificate shape: `method == Method::None`,
     `budget_left == Budget::new(0, 0, 0)` (derive or construct `Debug`/
     `PartialEq` comparisons as needed — `Budget` is `PartialEq`), and
     `props.get(Prop::SoundEnclosure) == Truth::Unknown` (the certificate
     claims ONLY the pairing property).

   Keep the doc examples' vertex/edge/wire code exactly as written in
   `shell.rs` — including the `wire!` macro and `Face::new(vec![w], ())` —
   with `S = ()` so no geometry is involved.

5. Also add one doctest to `check` itself: build the small 2-face closed
   "pillow" (two triangles over the same 3 vertices, second face inverted
   edge-wise — copy the `Closed` example's shape at minimal size) and assert
   `check(&shell).is_ok()`.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N` literal unless that line ends with an `// H-3` comment. This packet
uses no float literals at all — the witnesses are pure topology over `()`.
Run `bash scripts/kernel-gates.sh` yourself before writing `RESULT.json`.

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
116 doctests, zero clippy findings, measured at HEAD 4917c55 by BG-CE-003's
verify); your bar is everything above stays green plus your four tests and
one doctest.

## Forbidden

Editing any file outside `write_allow`. Changing the refusal shapes or the
certificate fields of decisions 2-3 (the wave's seven checkers share them —
a deviation here is a wave-wide contract break; if you believe the shape is
wrong, that is a SPEC_GAP, not an edit). Touching `shell.rs`. Adding
`#[ignore]`. Adding `unwrap()`/`expect()` outside the test module.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- a `ShellCondition` doc example does not compile as copied → `SPEC_GAP`,
  with the example and the error
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(topology): coedge pairing invariant checker (BG-INV-101)`.
