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
budget:      {turns: 55, ctx_tokens: 130000}
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

1. **Case (a) — coplanar butt-join union**: when the contact funnel's
   events certify that the shared boundary region between two solids is
   exactly a common face set (carrier-equal, opposite orientation, zero
   measure intersection interior), the union decision for those fragments
   is: keep both, emit the split faces as today's record — the DECISION is
   certified (the typed refusal is replaced by a certified answer whose
   content matches the recorded 10-split-face behavior). The decision is
   derived from the already-certified event records — no new tolerance.
2. **Case (b) — exact-footprint halfspace difference**: when the cutter's
   wall planes are certified coplanar with the target's faces (plane
   equality within the ctx's exact-predicate discipline — dyadic where
   possible), the fragment classification takes the ON-BOUNDARY fragments
   from the certified plane-equality record instead of refusing
   `Contradictory`. The padded over-box construction stays valid.
3. **V5, absolute**: every canonical fixture that certifies today
   certifies bit-identically; the two cases above are reached ONLY where
   the funnel currently refuses. `boolean_m2` byte-identical.
4. **The general calculus is out of scope**: if a case arrives that the
   two certified decisions do not cover, it keeps its typed refusal —
   that is success, not failure.
5. `classify.rs` propagation logic is NOT edited (BIE-006's frozen reuse);
   the decisions enter at the DECIDE/ASSEMBLE boundary your write set owns.

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

1. `butt_join_coplanar_union_certified` — two boxes sharing an exact face
   union WITHOUT `ContactReductionDeferred`; the output carries the
   recorded split-face structure; the exact-box metamorphic test (landed
   semantics) holds.
2. `exact_footprint_halfspace_difference_certified` — the recorded
   exact-footprint halfspace case (P3's D3 fixture class) now certifies;
   the padded over-box control still works identically.
3. `canonical_controls_bit_identical` — boolean_m2's fixture set through
   the same entries gives byte-identical results (hash asserted).

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
