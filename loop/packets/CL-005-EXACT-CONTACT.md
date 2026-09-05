# WORK PACKET CL-005-EXACT-CONTACT — certified decisions for the measured exact-contact cases

You are implementing the exact-contact layer of the Carrier Lift (CL)
program. Everything you need is in this document and
`docs/CARRIER_LIFT_BUILD_SPEC.md`. If something you need is genuinely
missing, that is a SPEC_GAP (see "Stop conditions"): you stop and report,
you do not research it.

```yaml
id:          CL-005-EXACT-CONTACT
contract:    [CL-005-EXACT-CONTACT]
class:       mechanical
crates:      [truck-shapeops]
depends_on:  [BIE-006-CLASSIFY]
write_allow:
  - vendor/truck/truck-shapeops/src/boolean/mod.rs
  - vendor/truck/truck-shapeops/src/boolean/assemble.rs
  - vendor/truck/truck-shapeops/tests/cl_exact_contact.rs
read_allow:
  - vendor/truck/truck-shapeops/src/boolean/classify.rs
  - vendor/truck/truck-shapeops/tests/boolean_m2.rs
  - docs/CARRIER_LIFT_BUILD_SPEC.md
tests_required:
tests_required:
  - butt_join_coplanar_union_certified
  - exact_footprint_halfspace_difference_certified
  - canonical_controls_bit_identical
budget:      {turns: 100, ctx_tokens: 200000}
```

**New test file** (`cl_exact_contact.rs`): H-1 applies; no landed test file
may be touched.

## Problem

The funnel defers two MEASURED exact-contact classes (recorded in the loop
traps): (a) butt-join coplanar unions keep cosmetically-split faces and the
refusal `ContactReductionDeferred` fires on the general class; (b) an
exact-footprint halfspace box whose walls are coplanar with the solid's
faces refuses `Contradictory(FragmentInsideOther)`. This packet certifies
THESE TWO CASES (pre-decided), not the general measure-zero calculus (that
is theory-class, deliberately out of scope).

## Scope decisions — pre-made, do not relitigate

**R2 AMENDMENT (2026-09-05, adjudicated SPEC_GAP):** the r1 worker proved by
experiment that neither case below is derivable at the DECIDE/ASSEMBLE
boundary — the choke points live in `split.rs`/`classify.rs`, which this
packet's write set freezes (`CL-005-STOP-QUESTION.md`, commit `4d59d5b`;
probe transcripts preserved). The packet is RESCOPED to what the boundary can
certify, per the root theory's own §5.9 scoping correction (the
exact-footprint halfspace class "should not be booked as a T2 battery row").

1. **Case (a) — coplanar butt-join union, REBOOKED**: the x/y-axis
   full-face butt joins refuse inside `split.rs::finish` (Region2
   `CoincidentInterval` between vertical side faces) — upstream of this
   packet's boundary. Certifying them requires the splitter's vertical-face
   seam to split like its z-axis twin: that is **CTE-007's** write set and
   its §5.9 truth-row battery. NOT this packet's row anymore.
2. **Case (b) — exact-footprint halfspace, DROPPED**: the theory's §5.9
   scoping correction excludes it from the contact battery (no coplanar cap
   pair; cutter walls terminate inside the plate — interior-loop rewrite
   machinery, booked open). It keeps its typed refusal with a recorded
   reason; a refusal that is typed and recorded is SUCCESS under §5.9.
3. **This packet's actual work**: pin the families that ALREADY certify at
   HEAD (z-axis full-face union 10-face, P3 recombination 10-face, padded
   over-box controls 6-face, M2 flagship 7/8-face) as a regression battery;
   document the certified decision predicates behind them; and assert the
   two rescoped classes carry their typed refusals (never silent wrong
   output). The worker's experiment (Region2 `Coincident` events are
   load-bearing for strictly-interior containments and harmful only for the
   boundary-touching coplanar class) is the recorded evidence CTE-007
   builds against — preserve it in the RESULT notes.
4. **V5, absolute**: every canonical fixture that certifies today
   certifies bit-identically. `boolean_m2` byte-identical.
5. `classify.rs` propagation logic is NOT edited (BIE-006's frozen reuse);
   `split.rs` stays frozen here (CTE-007's write set).

## Anchors — measured 2026-09-05, counts are exact

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `vendor/truck/truck-shapeops/src/boolean/mod.rs` | `pub fn fragment_decision` | 1 |
| A2 | `vendor/truck/truck-shapeops/src/boolean/assemble.rs` | `pub fn boolean\(` | 1 |
| A3 | `vendor/truck/truck-shapeops/src/boolean/classify.rs` | `pub fn classify_fragments` | 1 |
| A4 | `vendor/truck/truck-base/src/evidence.rs` | `ContactReductionDeferred` | 1 |

## House rules

- **H-1** no unwrap/expect/panic reachable from geometry; **H-3** same-line
  `// H-3`; **H-6** never record `Float` as `Exact` — plane equality is a
  certified predicate, not a tolerance comparison.
- **Determinism**: the two decisions are pure functions of the certified
  event records.
- **All cargo through the queue shim.** Scoped commands only.

## Tests required

1. `z_axis_full_face_butt_join_certified` — the z-stacked full-face union
   and the P3 `(S−pad) ∪ (S∩pad)` recombination keep their recorded
   10-face structure through the certified decision path (pinned as
   regression, not changed behavior).
2. `rescoped_classes_carry_typed_refusals` — the x/y full-face butt joins
   refuse inside the splitter and the exact-footprint halfspace cases
   refuse `Contradictory`/typed — asserted AS REFUSALS with their named
   causes (a silent wrong output on either is a failure).
3. `canonical_controls_bit_identical` — boolean_m2's fixture set through
   the same entries gives byte-identical results (hash asserted), plus the
   padded controls (6 faces) and M2 flagship (7/8 faces).

No existing test may be deleted, `#[ignore]`d, or weakened.

## Done when

```
cargo fmt --check -p truck-shapeops
cargo clippy -p truck-shapeops --all-targets -- -D warnings
cargo test -p truck-shapeops --lib --tests
cargo check -p truck-certified -p truck-evidence
```

## Forbidden

Anything outside `write_allow` — especially `classify.rs`, `split.rs`,
any landed test file, `scripts/kernel-gates.sh`, `Cargo.lock`. The general
measure-zero calculus. Adding `#[ignore]`. Unjustified `#[allow]`.
Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- a case's decision cannot be derived from the certified event records
  alone → `SPEC_GAP`, naming the missing certificate
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

```json
{"id":"CL-005-EXACT-CONTACT","status":"DONE","contracts":["CL-005-EXACT-CONTACT"],
 "tests_added":3,"anchors_verified":{"A1":1,"A2":1,"A3":1,"A4":1},
 "notes":"the certified predicates behind each decision, and the control hash evidence"}
```

Commit subject: `feat(shapeops): certified decisions for the measured exact-contact cases (CL-005-EXACT-CONTACT)`.
