# WORK PACKET CFP-003-SEPARABILITY — stop materializing the product-chart grid

You are refactoring the SSI engine's central data structure so that every
enclosure is computed PER SIDE and combined by the separability corollaries,
instead of materializing the 4-axis tensor grid. Everything you need is in
this document and `docs/CONTACT_FAST_PATH_BUILD_SPEC.md` §3 (Theorem 1,
Corollaries 1.1–1.4, Propositions 2.1) and §4 (packet row). Do not read other
spec files. SPEC_GAP discipline applies.

```yaml
id:          CFP-003-SEPARABILITY
contract:    [CFP-003-SEPARABILITY]
class:       design
crates:      [truck-certified]
depends_on:  [CFP-000-SPINE, CFP-002-INSTRUMENT, CFP-007-SAMPLE-CONSTANTS]
write_allow:
  - vendor/truck/truck-certified/src/ssi_types.rs
  - vendor/truck/truck-certified/src/ssi.rs
  - vendor/truck/truck-certified/src/ssi_admit.rs
  - vendor/truck/truck-certified/src/ssi_trace.rs
  - vendor/truck/truck-certified/src/tangency/minors.rs
  - vendor/truck/truck-certified/src/tangency/exclude.rs
  - vendor/truck/truck-certified/src/tangency/tsystem.rs
  - vendor/truck/truck-certified/src/patch_admit.rs
read_allow:
  - docs/CONTACT_FAST_PATH_BUILD_SPEC.md
  - vendor/truck/truck-certified/src/cfp/spine.rs
  - vendor/truck/truck-certified/src/cfp/fixtures.rs
  - vendor/truck/truck-certified/src/ssi_types.rs
  - vendor/truck/truck-certified/src/ssi.rs
  - vendor/truck/truck-certified/src/tangency/minors.rs
  - vendor/truck/truck-evidence/src/enclosure.rs
tests_required:
  - fc3_separation_exact_sign_certificate
  - per_side_hull_set_identical_to_grid_hull_on_fixtures
  - minor_factorization_matches_direct_expansion
  - memoization_distinct_boxes_bounded
  - normal_net_degree_and_d3_count
  - landed_ssi_fixture_verdicts_unchanged
budget:      {turns: 90, ctx_tokens: 200000}
```

## Problem

`SquareSystem3` (`ssi_types.rs:75`) materializes
`F_k(u,v,s,t) = W2·N1_k − W1·N2_k` as 4-axis tensor-Bernstein grids. Under D2
(unit weights) that materialization is pure overhead: Theorem 1 gives
`c_αβ = P¹_α − P²_β` with no cross terms, so the per-cell 4-axis de Casteljau
hull (O(deg⁴)) is set-identical to the interval difference of two per-side
2-D hulls (O(deg²), Corollary 1.1). The minors factor per Corollary 1.3, and
every quantity keys on one side's box (Corollary 1.4 — memoization). This
packet is the refactor; the expected magnitude is 5–20x on the subdivision
phase.

## Scope decisions — pre-made, do not relitigate

1. **The stored shape becomes per-side.** `SquareSystem3` carries the two
   admitted patch representations (or their grids) + degrees; the 4-axis
   `grids: [Vec<Vec<f64>>; 3]` field and its constructor path are REMOVED,
   not kept behind a flag. Consumers read per-side enclosures through new
   accessors. The grid layout documentation is updated or deleted with the
   field.
2. **Normal nets, counted against D3.** Normal fields are enclosed by their
   OWN Bernstein grids (bidegree `(2p−1, 2q−1)`, `4pq` coefficients per
   component) — never as interval cross products of derivative hulls, which
   reintroduces intra-carrier dependency (Proposition 2.1's correction). The
   net composition is exact coefficient algebra (products of Bernstein
   coefficients), composed once per patch, not per cell.
3. **Minors by Corollary 1.3**: dot products of a per-side normal enclosure
   with a per-side tangent enclosure — disjoint variable sets, no dependency
   inflation. The landed `det3`-of-interval-Jacobian path is replaced by
   these dot products wherever it consumed the materialized grid.
4. **Memoization by Corollary 1.4**: per-side hulls cached on (side, box).
   The cache is deterministic (fixed eviction: none — the subdivision visit
   order is fixed; a bounded map keyed on box coordinates with total-order
   keys, documented).
5. **Verdict gate, certificate adjudication.** Corollary 1.1 is
   SET-identical, not bit-identical: certificate enclosures may differ in
   ulps. The gate is: the landed `ssi_fixtures` battery's VERDICT SET is
   unchanged (same Certified/Unresolved pattern, same roots), enclosure
   widths may differ, and any width that WIDENS beyond the fixture's asserted
   margins is adjudicated like V5-boolean: recorded, never silently accepted.
6. **Consumers carried across the accessors**: `ssi_trace`, `ssi_admit`, and
   CTE's `minors`/`exclude`/`tsystem` (which read the grids via
   `from_system_grid`) are rewritten against the per-side API in THIS packet
   — the program does not land a half-refactored state. `ssi4.rs` is
   untouched (direct carrier evaluation; file owned by CFP-007).
7. **D3 budget revisited here, ONCE**: with normal nets now composed per
   patch, re-derive the `MAX_BIDEGREE` budget so the nets + BVH leaves
   (CFP-005) fit; state the new constant and its arithmetic in the notes.
8. **Zero new top-level evidence kinds.** All outputs are the landed
   vocabularies (`SsiRefusal`, `CertifiedInterval`, `HullRefusal`).

## Anchors — base-relative, drift-tolerant

Re-check on your branch:

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `truck-certified/src/ssi_types.rs` | `pub struct SquareSystem3` | 1 |
| A2 | `truck-certified/src/ssi_types.rs` | `grids: [Vec<Vec<f64>>; 3]` | **1 before, 0 after your change** (the materialization removed) |
| A3 | `truck-certified/src/tangency/minors.rs` | `from_system_grid` | >0 before (the consumer you carry) |
| A4 | `truck-certified/src/ssi.rs` | `pub fn krawczyk3_certificate` | 1 |
| A5 | `truck-certified/src/construct/bie/ssi4.rs` | anything | read-only — must show NO diff (CFP-007 owns it, then frozen) |

A2 is the packet-owned delta. If already 0 at base, another writer landed
first: STOP, `ANCHOR_MISMATCH`.

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `todo!` reachable from geometry.
- **H-3** No absolute constants in predicates; test epsilons carry `// H-3`
  on the SAME line as the literal.
- **BG-ENC-003**: outward rounding; no fast-math; no FMA contraction.
- **Determinism**: fixed visit/eviction orders; identical ordered input →
  identical verdicts and identical enclosures.
- **All cargo invocations go through the queue (the `cargo` on PATH IS the
  queue shim).**

## Tests required

Named `#[test]` fns; the verifier checks the names appear in your diff.
Fixture data copied from `cfp/fixtures.rs` as constants, read-only.

1. `fc3_separation_exact_sign_certificate` — F-C3: the separated pair
   certifies via the exact `Expansion` dot-sign row; the touching pair does
   not separate.
2. `per_side_hull_set_identical_to_grid_hull_on_fixtures` — for every landed
   fixture pair, the per-side composition contains and is contained by the
   old grid hull up to directed-rounding slack (set-identity check).
3. `minor_factorization_matches_direct_expansion` — Corollary 1.3 vs the
   direct `det3` expansion on fixture Jacobians, equal within rounding.
4. `memoization_distinct_boxes_bounded` — F-C5-style subdivision visits `M`
   cells while paying `O(k)` hulls (counter through the landed instrument
   fields `cells_visited`/`distinct_side_boxes`).
5. `normal_net_degree_and_d3_count` — the composed normal net has bidegree
   `(2p−1, 2q−1)` and `4pq` coefficients per component on the fixture.
6. `landed_ssi_fixture_verdicts_unchanged` — the `ssi_fixtures` battery
   verdict set is identical pre/post refactor (adjudication record in notes).

## Done when — run these, all must pass

```
cargo fmt --check -p truck-certified
cargo clippy -p truck-certified --all-targets -- -D warnings
cargo test -p truck-certified --lib ssi
cargo test -p truck-certified --lib tangency
```

Send cargo output to a file and read the tail.

## Forbidden

Editing any file outside `write_allow` — especially `construct/**`
(including `construct/bie/ssi4.rs`), `num/**`, `contact/**`, anything under
`truck-evidence/` or `truck-shapeops/`, `Cargo.lock`. Keeping the 4-axis
materialization behind a flag. Changing any fixture ground truth. Adding
`#[ignore]`. Adding `#[allow]` without a justification comment on the same
line. Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- the landed fixture battery's verdict set differs → STOP and report the
  differing row (finding, not failure)
- a consumer cannot be carried without changing its verdict semantics →
  SPEC_GAP
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

**COMMIT BEFORE writing `RESULT.json`.** Then write `RESULT.json` at the root
of your worktree.

```json
{"id":"CFP-003-SEPARABILITY","status":"DONE","contracts":["CFP-003-SEPARABILITY"],
 "tests_added":6,"anchors_verified":{"A1":1,"A2":0,"A3":0,"A4":1,"A5":0},
 "max_bidegree_revised":"<constant + arithmetic>",
 "notes":"per-side refactor landed; consumers carried; verdict adjudication record; any deviation stated here"}
```
