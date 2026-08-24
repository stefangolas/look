# WORK PACKET BG-AUD-FIX-008 — closed-shell / vertex-link degeneracy (AUD-008, AUD-015)

You are repairing two defects found by the formal-kernel correctness audit
`loop/audits/BG-AUDIT-001.md` (findings AUD-008 and AUD-015), in
`truck-topology`. Everything you need is in this document. **Do not read any
other spec file** — this packet is self-contained.

```json
{"id":"BG-AUD-FIX-008","status":"DONE","contracts":["AUD-008","AUD-015"],
 "tests_added":3,"deviations":[],"disagreements":[],
 "baseline_failures":[],"notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. **If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-AUD-FIX-008
contract:    [AUD-008, AUD-015]
class:       design
crates:      [truck-topology]
write_allow:
  - vendor/truck/truck-topology/src/wire.rs
  - vendor/truck/truck-topology/src/shell.rs
  - vendor/truck/truck-topology/src/solid.rs
  - vendor/truck/truck-topology/src/invariants/coedge_pairing.rs
  - vendor/truck/truck-topology/src/invariants/vertex_link.rs
read_allow:
  - vendor/truck/truck-topology/src/edge.rs
  - vendor/truck/truck-topology/src/face.rs
  - vendor/truck/truck-base/src/evidence.rs
tests_required:
  - single_face_edge_inverse_shell_is_not_closed
  - wire_reusing_edge_id_is_not_simple
  - vertex_link_open_shell_refuses
budget:      {turns: 45, ctx_tokens: 110000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub fn is_simple' vendor/truck/truck-topology/src/wire.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn shell_condition' vendor/truck/truck-topology/src/shell.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'fn try_new' vendor/truck/truck-topology/src/solid.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'fn check' vendor/truck/truck-topology/src/invariants/vertex_link.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'fn check' vendor/truck/truck-topology/src/invariants/coedge_pairing.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub fn singular_vertices' vendor/truck/truck-topology/src/shell.rs"}
```

## Problem

### AUD-008 — degenerate single-face `[e, e.inverse()]` shell certified Closed

A single face whose boundary wire is `[e, e.inverse()]` — the SAME edge used
twice, opposite sense — is certified `ShellCondition::Closed`: the `Boundaries`
mechanism sees each edge once with each orientation, so the boundary set is
empty and the condition is `Oriented` (shell.rs:1173-1179). Then
`coedge_pairing::check` sets `CoedgePairing = True`, `vertex_link::check` sets
`VertexLink = True`, and `Solid::try_new` accepts it. The result is a
zero-volume degenerate "solid" certified as a valid closed manifold boundary.
`Wire::is_simple` misses it because it only checks vertex-stream distinctness.

### AUD-015 — vertex-link certifies "single cycle" where only connectivity holds

`vertex_link::check` sets `Prop::VertexLink = True` when
`shell.singular_vertices()` is empty. `singular_vertices` tests link
CONNECTIVITY; "connected = single cycle" holds only on a CLOSED shell. The
checker neither requires nor verifies closure (its doc says the closure
pre-check "belongs to BG-INV-101's checker", which nothing enforces). An open
single-triangle shell (every link a path) certifies the full S^1 vertex-link
proposition. The audit grades this a documented over-claim.

**Your first obligation — observe the regressions fail on the buggy code:** add
the three tests below and run them. Test 1 must show the degenerate shell
certifying Closed on the current code; test 3 must show the open triangle
certifying a hold. Record the pre-fix observations in `RESULT.json.notes`.

## Repair

1. **`Wire::is_simple` rejects edge-id reuse** (`wire.rs`). In addition to the
   existing vertex-stream distinctness check, require that no edge id appears
   more than once in the wire (`edge_iter()` ids into a `HashSet`). A wire that
   reuses an edge id (`[e, e.inverse()]`) is not simple. The existing doctests
   (closed loop → not simple via vertex distinctness; `[e0,e1,e2,e4]` → simple)
   must stay green.

2. **Closedness requires distinct faces per edge** (`shell.rs`). `Closed` must
   require that each face's boundary wires do not reuse an edge id — the
   degenerate `[e, e.inverse()]` face is exactly an edge used twice by ONE
   face. In `shell_condition()`, first check every face's boundary wires
   (`face.boundary_iters()` → wires → `edge_iter()` ids) for edge-id reuse; if
   any face reuses an edge id, return `ShellCondition::Oriented` (an oriented
   but not validly-closed boundary) instead of `Closed`. The cube witness in
   the `Closed` doctest (each edge used by two DIFFERENT faces) has no per-face
   reuse and must stay `Closed`. `Solid::try_new` needs no change — it reads
   `shell_condition()` and now refuses the degenerate shell with
   `NotClosedShell`.

3. **`vertex_link::check` requires the closed precondition**
   (`invariants/vertex_link.rs`). Establish the precondition inside the checker
   before certifying: when `shell.shell_condition() != ShellCondition::Closed`,
   return `Err(Refusal::NumericallyUnresolved { spent: Budget::new(0,0,0),
   witness: UnresolvedWitness::UncertifiedContainment })` — the closure
   precondition is not established, so the S^1 single-cycle proposition is not
   certified. When the shell IS closed and `singular_vertices().is_empty()`,
   certify `VertexLink = True` exactly as today. Update the module doc and the
   three affected tests/doctest:
   - the doctest and `vertex_link_regular_shell_holds` currently use the open
     two-face Möbius witness — replace the witness with a genuinely CLOSED
     shell (a tetrahedron or the cube used by `coedge_pairing`'s tests), or
     change them to assert the open-shell refusal;
   - `vertex_link_documented_dependency_on_closed` must now assert the
     REFUSAL for the open single triangle (its name stays accurate).

`coedge_pairing.rs` is in `write_allow` in case the Closed-gate change needs a
matching adjustment, but it likely needs none (it already reads
`shell_condition()`). Only touch it if a test forces it.

## Regression tests (exact names)

1. `single_face_edge_inverse_shell_is_not_closed`

   ```rust
   let v = Vertex::news([(), ()]);
   let e = Edge::new(&v[0], &v[1], ());
   let shell: Shell<(), (), ()> = vec![Face::new(vec![wire![&e, &e.inverse()]], ())].into();
   assert_ne!(shell.shell_condition(), ShellCondition::Closed);
   assert!(matches!(invariants::coedge_pairing::check(&shell), Err(Refusal::NumericallyUnresolved { .. }) | Err(Refusal::Contradictory(_))));
   assert_eq!(Solid::try_new(vec![shell.clone()]), Err(Error::NotClosedShell));
   ```

   (import the pieces you need from `crate::*`; the refusal arm shape depends
   on `coedge_pairing`'s current non-Closed branch — match whatever it returns
   and assert it is NOT an `Ok` certificate).

2. `wire_reusing_edge_id_is_not_simple`

   ```rust
   let v = Vertex::news([(), ()]);
   let e = Edge::new(&v[0], &v[1], ());
   let wire = wire![&e, &e.inverse()];
   assert!(!wire.is_simple());
   ```

3. `vertex_link_open_shell_refuses`

   The open single triangle (three edges, one face) — `shell_condition() !=
   Closed` — `invariants::vertex_link::check(&shell)` must return
   `Err(NumericallyUnresolved)` and never an `Ok` certificate.

Every other existing topology test must stay green — in particular the `Closed`
doctest cube, the `errors.rs` doctests (`NotClosedShell` uses two DISTINCT
edges, so it is unaffected; `NotManifold`/`NotConnected` too), and the
`coedge_pairing`/`vertex_link` closed witnesses.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N`-shaped literal unless that line ends with an `// H-3` comment. This
packet adds no float literals. Run `bash scripts/kernel-gates.sh <your base
commit>` yourself before writing `RESULT.json`.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-topology
cargo clippy -p truck-topology --all-targets --no-deps
cargo test -p truck-topology --lib --tests --no-fail-fast
cargo test -p truck-topology --doc
cargo check --workspace --all-targets
bash scripts/kernel-gates.sh <your base commit>
```

Never run a bare `cargo test`.

## Forbidden

Editing any file outside `write_allow`. Making `Closed` depend on the FULL
`is_simple` (vertex distinctness would reject every closed cube face — only
edge-id reuse is the defect). Adding arbitrary geometry tests instead of the
precondition strengthening. Weakening a negative test. Adding `#[ignore]`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- a pre-existing test you did not expect to touch is broken by the closedness
  or vertex-link change → do NOT weaken the gate; report it in `disagreements`
  with the failing test name and the exact reason, and (if it is a legitimately
  updated limitation test) update it as the packet directs
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`fix(topology): reject edge-id-reuse wires; closed shells need distinct faces per edge; vertex-link needs the closed precondition (BG-AUD-FIX-008)`.
