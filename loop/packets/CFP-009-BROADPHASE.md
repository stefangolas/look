# WORK PACKET CFP-009-BROADPHASE — spatial index over lifted strata

You are replacing the flat O(n·m) AABB double loop at the boolean entry with
a spatial index. The observable contract is EXACTNESS: the indexed sweep must
admit precisely the same pair set the flat loop admits. Everything you need
is in this document, `docs/CONTACT_FAST_PATH_BUILD_SPEC.md` §4 (packet row),
and the landed screen you are replacing. Do not read other spec files.
SPEC_GAP discipline applies.

```yaml
id:          CFP-009-BROADPHASE
contract:    [CFP-009-BROADPHASE]
class:       mechanical
crates:      [truck-shapeops]
depends_on:  [CFP-001-LIFT-ENCLOSURE]
write_allow:
  - vendor/truck/truck-shapeops/src/boolean/assemble.rs
  - vendor/truck/truck-shapeops/src/boolean/broadphase.rs
read_allow:
  - docs/CONTACT_FAST_PATH_BUILD_SPEC.md
  - vendor/truck/truck-shapeops/src/boolean/assemble.rs
  - vendor/truck/truck-certified/src/cfp/fixtures.rs
tests_required:
  - indexed_pair_set_identical_to_flat_loop
  - index_build_deterministic
  - empty_index_short_circuits_clean
budget:      {turns: 50, ctx_tokens: 120000}
```

## Problem

`sweep_contact_events` (`truck-shapeops/src/boolean/assemble.rs:123-210`)
tests every cross-solid stratum pair with an AABB `touches()` check in a flat
double loop — O(n_F·m_F + n_F·m_E + n_E·m_F + n_E·m_E) per boolean. At
assembly scale (13 systems, hundreds of parts) the loop is visible, and it
grows with the product. A spatial index reduces the screen to
O((n+m) + k) where k is the touching-pair count, which is what the funnel
was going to look at anyway.

## Scope decisions — pre-made, do not relitigate

1. **Exactness is the whole contract.** The index must return EXACTLY the
   pair set the flat loop returns — same pairs, no additions, no omissions.
   The index is a pure reorganization of the same `touches()` predicate over
   the same boxes. Any discrepancy is a bug, not a heuristic: the fixture
   gate compares pair sets exhaustively on every landed boolean fixture.
2. **Uniform grid, not a tree.** A uniform grid keyed on cell size derived
   from the median box extent (fixed formula, documented) is sufficient and
   deterministic. No rebalancing, no recursion, no adaptive structures. If a
   fixture shows pathological skew (all boxes in one cell), the fallback IS
   the flat loop over that cell's contents — still exact.
3. **Pair ordering is canonical and unchanged.** The flat loop's emission
   order (surface 0 strata outer, stratum index ascending) is preserved:
   the index collects candidate pairs, then they are EMITTED in the flat
   loop's canonical order. Downstream consumers see identical sequences.
4. **The edge special cases move with the pairs.** The `ee_circle_circle`
   skip and the FE/EE/FF arm routing live at the call sites — the index only
   replaces the box test, never the dispatch semantics. Those guards are
   applied to indexed candidates exactly as the flat loop applies them.
5. **Zero new evidence, zero new refusals.** The screen is an input filter;
   this packet changes no verdict, no certificate, no refusal arm.
6. **Determinism**: the grid cell assignment is a pure function of box
   coordinates; ties and boundary-crossing boxes resolve by a fixed rule
   (a box registers in EVERY cell it overlaps; cells iterate in fixed
   order; pair de-duplication by (index_a, index_b) with a fixed key order).

## Anchors — base-relative, drift-tolerant

Re-check on your branch:

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `truck-shapeops/src/boolean/assemble.rs` | `fn sweep_contact_events` | 1 |
| A2 | `truck-shapeops/src/boolean/assemble.rs` | `fn face_enclosure` | 1 (CFP-001 landed — read-only anchor; if 0, CFP-001 has not merged: BLOCKED, name the packet) |
| A3 | `truck-shapeops/src/boolean/broadphase.rs` | anything | **0 before, 1 after** (new file) |
| A4 | `truck-shapeops/src/boolean/assemble.rs` | `ee_circle_circle` | ≥1 (the guard survives; read-only anchor) |

A3 is the packet-owned delta. A2/A4 anchor the tree this packet expects.

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `todo!` reachable from geometry.
- **H-3** No absolute constants in predicates; the cell-size formula's
  constants carry `// H-3` on the SAME line as the literal.
- **Determinism**: cell assignment, iteration order, and de-duplication are
  pure functions of the input; identical ordered input → identical pair
  sequence.
- **All cargo invocations go through the queue (the `cargo` on PATH IS the
  queue shim).**

## Tests required

Named `#[test]` fns; the verifier checks the names appear in your diff:

1. `indexed_pair_set_identical_to_flat_loop` — on every landed boolean
   fixture, the indexed sweep's pair set equals the flat loop's pair set
   exactly (both computed in the test; exhaustive comparison).
2. `index_build_deterministic` — two builds on identical input produce
   identical structures and identical pair sequences.
3. `empty_index_short_circuits_clean` — a boolean of two solids whose
   certified boxes cannot touch produces zero candidates and the same
   (empty-event) outcome as the flat loop.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-shapeops
cargo clippy -p truck-shapeops --all-targets -- -D warnings
cargo test -p truck-shapeops --lib boolean
```

Send cargo output to a file and read the tail.

## Forbidden

Editing any file outside `write_allow` — especially `split.rs`,
`classify.rs`, `sweep_lift.rs`, anything under `truck-evidence/` or
`truck-certified/`, `Cargo.lock`. Changing the `touches()` predicate, the
`ee_circle_circle` guard, or the emission order. Any heuristic that could
drop a pair the flat loop admits. Adding `#[ignore]`. Adding `#[allow]`
without a justification comment on the same line. Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- A2 shows CFP-001 has not merged → `BLOCKED` (dependency), name the packet
- any pair-set discrepancy on any fixture → STOP and report the pair (that is
  a finding; exactness failures are never tuned away)
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

**COMMIT BEFORE writing `RESULT.json`.** Then write `RESULT.json` at the root
of your worktree.

```json
{"id":"CFP-009-BROADPHASE","status":"DONE","contracts":["CFP-009-BROADPHASE"],
 "tests_added":3,"anchors_verified":{"A1":1,"A2":1,"A3":1,"A4":1},
 "notes":"uniform-grid broadphase landed; pair set exactness gate green on all fixtures; any deviation stated here"}
```
