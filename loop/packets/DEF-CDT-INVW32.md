# WORK PACKET DEF-CDT-INVW32 â€” align two cone_topology invariant tests with the landed W3 semantics (test-only)

The two tests have been failing since the vendoring commit `da72cd5`. The
production CDT machinery is CORRECT; the tests encode pre-W3 semantics and a
fixture bug. Production code must not change.

```yaml
id:          DEF-CDT-INVW32
contract:    [DEF-CDT-INVW32]
class:       mechanical
crates:      [truck-meshalgo]
depends_on:  []
write_allow:
  - vendor/truck/truck-meshalgo/src/tessellation/triangulation.rs
read_allow:
  - vendor/truck/truck-meshalgo/src/tessellation/triangulation.rs
  - docs/defects/
tests_required:
  - the two repaired tests pass
  - no other cone_topology test changes verdict
anchors:
  - {id: A1, expect: 3, cmd: "grep -c 'INV-W3-2\\|INV-W3-3' vendor/truck/truck-meshalgo/src/tessellation/triangulation.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'test_parity_intersecting_constraints_rejected' vendor/truck/truck-meshalgo/src/tessellation/triangulation.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'duplicate_edge_creates_no_second_cdt_edge' vendor/truck/truck-meshalgo/src/tessellation/triangulation.rs"}
budget:      {turns: 40, ctx_tokens: 160000}
```

## Problem

1. `duplicate_edge_creates_no_second_cdt_edge` (tests at
   triangulation.rs:12607-12670): the fixture is an OPEN chain (5 points,
   never closes), so the lone-open-piece synthetic closure
   (triangulation.rs:8238-8314) adds three SyntheticClosure segments and the
   realized loop has 5 unique constraint edges, not 3. The asserts
   (`num_constraints == 3`, traversal sum 5) were written for a CLOSED
   pentagon. Fix: append `corner(0.0, 0.0)` to close the chain. The bottom
   edge stays traversed 3x; INV-W3-2/3 keep their meaning.
2. `test_parity_intersecting_constraints_rejected` (12057-12087): encodes
   PRE-W3 rejection semantics (the loop must fail/produce empty mesh). The
   landed ARR-SEAM W3 semantics (see the doc at triangulation.rs:8660-8664,
   and the PASSING sibling `duplicate_edge_is_not_constraint_overlap_unsupported`
   at 12572-12605) admit duplicate traversals with parity = multiplicity
   mod 2 â€” the degenerate spike encloses zero area and the square's
   triangles are correct. Fix: align the test with INV-W3-1 â€” assert the
   mesh equals the plain square's triangle count (reuse the
   `triple_traversal_equals_single` style, 12544-12570), and rename the
   intent: the fixture has DUPLICATE traversals, not proper crossings
   (proper crossings are planarized by `insert_with_split`, 9238-9378, and
   covered by the bowtie test at 13833ff). Record the rename in RESULT.

## Scope decisions

1. TEST-ONLY. Zero production deltas: the duplicate arm (8686-8749), the
   parity reading (9067-9109), the closure (8238-8314), the split/reject
   machinery (9238-9378) are all correct as landed.
2. Do NOT re-baseline test A to `5`/`8` â€” that would measure the synthetic
   rectangle closure, not the duplicate invariant.
3. Do NOT re-introduce ConstraintOverlapUnsupported rejection anywhere.
4. V5: every OTHER landed test in the file is byte-identical constraint.

## Done when

```
cargo test -p truck-meshalgo --lib tessellation::triangulation::cone_topology_tests
cargo test -p truck-meshalgo --lib
```

## Forbidden

Production code changes in triangulation.rs or anywhere else.
`catch_unwind` removal. H-1 violations. Touching other tests.

## Stop conditions

- Any anchor drift â†’ ANCHOR_MISMATCH.
- Closing the fixture makes some OTHER cone_topology test fail â†’ SPEC_GAP
  (record which and stop).

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `test(meshalgo): align INV-W3-2/W3-1 cone_topology tests with landed W3 semantics (DEF-CDT-INVW32)`.
