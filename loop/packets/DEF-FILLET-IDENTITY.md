# WORK PACKET DEF-FILLET-IDENTITY — restore shared-edge identity in the fillet replacement path

`tests/fillet.rs complex_surface` fails at fillet.rs:412 with
`ShellCondition::Oriented` (want `Closed`): after filleting, the
fillet-face/side-face boundary exists as TWO different Edge instances with
two different EdgeIDs, so `Boundaries::condition()` (shell.rs:1184-1190)
finds unpaired ids. Root cause identified by audit.

```yaml
id:          DEF-FILLET-IDENTITY
contract:    [DEF-FILLET-IDENTITY]
class:       design
crates:      [truck-shapeops]
depends_on:  []
write_allow:
  - vendor/truck/truck-shapeops/src/fillet/mod.rs
  - vendor/truck/truck-shapeops/src/fillet/
read_allow:
  - vendor/truck/truck-shapeops/src/fillet/
  - vendor/truck/truck-topology/src/shell.rs
  - vendor/truck/truck-topology/src/entity_id.rs
tests_required:
  - complex_surface green (fillet.rs:383, 412, 417)
  - fillet_pp.rs + chamfer.rs unaffected
  - new regression assertion: the fillet face's absolute_boundaries ids
    pair with the side faces' boundaries
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'with_curve' vendor/truck/truck-shapeops/src/fillet/mod.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'set_curve' vendor/truck/truck-shapeops/src/fillet/experiment.rs"}
budget:      {turns: 50, ctx_tokens: 160000}
```

## Problem — the mechanism (audit-verified)

Commit `c5cb4c6` (BG-CE-003-MIGRATE, Aug 22) migrated fillet/mod.rs from
`Arc<Mutex<G>>` in-place mutation to the "replacement, never in-place
mutation" API. The mechanical migration broke identity:

- BEFORE: `fillet_edge.set_curve(...)` mutated the SHARED edge instance in
  place — same Edge, same EdgeID in both faces (the untouched prototype
  experiment.rs:690 still shows this form).
- AFTER: `fillet_with_side` takes `fillet_edge = &simple_fillet.fillet
  .absolute_boundaries()[0][1]` (mod.rs:464) — the SAME instance the
  fillet face's wire holds (`fillet_boundary = [fillet_edge0.inverse(),
  edge0, ...]`, mod.rs:339-340). `create_new_side` replaces it via
  `with_curve` (mod.rs:397) and inserts the REPLACEMENT only into the side
  face's boundary (mod.rs:409/413; side1 likewise via [0][3]/edge1,
  mod.rs:475).
- Result: the shared boundary is two Edge instances / two ids.
  `Boundaries::insert` (shell.rs:1164-1181) sees each id once → Oriented.
- The pre-fillet assert (line 383) passes because the ORIGINAL faces share
  instances; only the fillet-created boundary leaks.

## Scope decisions

1. Propagate the replaced instance to BOTH adjacent faces: `create_new_side`
   returns (or exposes) the replaced `fillet_edge`; the caller rebuilds the
   fillet face's wire (mod.rs:338-341) from the same instances. If
   entity_id.rs offers an id-preserving replacement route, prefer it and
   record the choice.
2. Decide `experiment.rs:690` (still `set_curve`): migrate or mark
   prototype-only in a doc comment (it is not compiled into the landed
   path — record the decision).
3. The BG-AUD-FIX-008 gate (shell.rs:150-156) is NOT the trigger; do not
   touch shell.rs.
4. Acceptance: fillet.rs:383/412/417 all green; fillet_pp.rs and
   chamfer.rs verdicts unchanged; one new regression assertion pinning
   id-pairing so the class cannot silently return.

## Done when

```
cargo test -p truck-shapeops --test fillet
cargo test -p truck-shapeops --test fillet_pp
cargo test -p truck-shapeops --test chamfer
cargo test -p truck-shapeops --lib
```

## Forbidden

vendor/truck outside write_allow. Test-expectation edits (the test is
correct; the kernel broke). In-place mutation reintroduction.

## Stop conditions

- The identity fix changes fillet GEOMETRY (not just identity) anywhere →
  stop, record the diff evidence, SPEC_GAP.
- `create_new_side`'s replacement cannot be propagated without violating
  the replacement doctrine → SPEC_GAP naming the conflict.

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `fix(shapeops): propagate the fillet-edge replacement to both adjacent faces — one boundary, one EdgeID (DEF-FILLET-IDENTITY)`.
