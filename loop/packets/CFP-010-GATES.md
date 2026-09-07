# WORK PACKET CFP-010-GATES — the integrated battery and defect closure

You are landing the CFP program's closing battery: the one full verification
at integrated HEAD, the V5-boolean adjudication record across every landed
diff, the fixture battery evaluation, and the defect-status closure. This is
the CTE-008-GATES role for the CFP program. Everything you need is in this
document, `docs/CONTACT_FAST_PATH_BUILD_SPEC.md`, and the two defect records.
Do not read other spec files. SPEC_GAP discipline applies.

```yaml
id:          CFP-010-GATES
contract:    [CFP-010-GATES]
class:       mechanical
crates:      [truck-certified, truck-evidence, truck-shapeops]
depends_on:  [CFP-004-IMPLICIT-REDUCTION, CFP-005-BRANCH-BVH, CFP-006-CONE-CERTIFICATES, CFP-008-STAGNATION]
write_allow:
  - vendor/truck/truck-certified/tests/cfp_battery.rs
  - vendor/truck/truck-shapeops/tests/cfp_gates.rs
  - docs/defects/DSC-BOUNDARY-SAMPLE-EXTENT-001.md
  - docs/defects/NUM-SPLINE-ENCLOSURE-CONVERGENCE-001.md
  - docs/CONTACT_FAST_PATH_BUILD_SPEC.md
read_allow:
  - docs/CONTACT_FAST_PATH_BUILD_SPEC.md
  - docs/defects/DSC-BOUNDARY-SAMPLE-EXTENT-001.md
  - docs/defects/NUM-SPLINE-ENCLOSURE-CONVERGENCE-001.md
  - vendor/truck/truck-certified/src/cfp/spine.rs
  - vendor/truck/truck-certified/src/cfp/fixtures.rs
tests_required:
  - fc0_defect_correction_green_at_head
  - fc2_enclosure_battery_green_at_head
  - fc3_fc4_fc5_fixture_battery_at_head
  - v5_boolean_monotone_adjudication_record
  - determinism_full_program
  - instrument_f_datum_recorded
anchors:
  - {id: A1, expect: 0, cmd: "grep -c 'Mechanism established' docs/defects/DSC-BOUNDARY-SAMPLE-EXTENT-001.md"}
  - {id: A2, expect: 0, cmd: "grep -c 'Not corrected' docs/defects/NUM-SPLINE-ENCLOSURE-CONVERGENCE-001.md"}
  - {id: A3, expect: 1, cmd: "ls vendor/truck/truck-certified/src/bvh.rs 2>/dev/null | wc -l"}
  - {id: A4, expect: 1, cmd: "ls vendor/truck/truck-evidence/src/contact/implicit2d.rs 2>/dev/null | wc -l"}
  - {id: A5, expect: 1, cmd: "grep -c 'mod instrument' vendor/truck/truck-evidence/src/contact/mod.rs"}
budget:      {turns: 70, ctx_tokens: 160000}
```

## Problem

The CFP program's gates (build spec §5) require ONE full verification at the
integrated HEAD after the implementation packets land: the fixture battery
evaluated end-to-end, the V5-boolean diffs adjudicated and recorded, the
determinism gate over the whole program, and the two defect records closed
against measured evidence. Per-packet verifies were suspended (one-verify
amendment); THIS packet is the verify.

## Scope decisions — pre-made, do not relitigate

1. **The battery is read-only over production code.** Write set is test
   files + docs. Any production-code edit discovered here is a finding —
   STOP and report it; it goes back through its own packet.
2. **V5-boolean adjudication is cumulative.** Every boolean-output diff
   introduced by CFP-001 (screen widening), CFP-004 (new stage), and
   CFP-005 (pair enumeration) is adjudicated ONCE here against the merged
   HEAD: each diff must be a monotone superset (new events added, none
   removed) on identical inputs. The record lives in
   `truck-shapeops/tests/cfp_gates.rs` as named cases + the notes field.
   A non-monotone diff is a FINDING: STOP and report — it means one of the
   landed packets broke the invariant, and the battery does not paper over
   it.
3. **Defect closure requires the measured witnesses.** F-C0 green at HEAD
   closes `DSC-BOUNDARY-SAMPLE-EXTENT-001`'s "not corrected" status (status
   moves to `Closed — correction validated by F-C0 at integrated HEAD`);
   F-C2's convergence row closes
   `NUM-SPLINE-ENCLOSURE-CONVERGENCE-001` the same way. Synthetic witnesses
   only — the records keep their corpus-witness caveats and are NOT deleted.
4. **The instrument's `f` datum is recorded.** Run the corpus battery with
   `TRUCK_CFP_INSTRUMENT=1`, extract the stage-5 pair-class histogram, and
   record `f` (plane/quadric×spline share) in the build spec's §6 table,
   filling the composed end-to-end cell with the formula's evaluated value.
   This is the number the spec deliberately left unfilled.
5. **Determinism over the whole program**: identical ordered input →
   identical verdicts, certificates, pair lists, and counter outputs, across
   two full battery runs at HEAD.
6. **Zero new top-level evidence kinds.** The battery asserts landed
   vocabularies only.

## Anchors — base-relative, drift-tolerant

Re-check on your branch:

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `docs/defects/DSC-BOUNDARY-SAMPLE-EXTENT-001.md` | `Mechanism established` | ≥1 (the record you will close) |
| A2 | `docs/defects/NUM-SPLINE-ENCLOSURE-CONVERGENCE-001.md` | `Not corrected` | **1 before, 0 after your change** (status flipped) |
| A3 | `vendor/truck/truck-certified/src/bvh.rs` | anything | ≥1 (CFP-005 landed; if 0: BLOCKED, name the packet) |
| A4 | `vendor/truck/truck-certified/src/implicit2d.rs` | anything | ≥1 (CFP-004 landed; if 0: BLOCKED, name the packet) |
| A5 | `vendor/truck/truck-evidence/src/contact/mod.rs` | `mod instrument` | 1 (CFP-002 landed; read-only anchor) |

A2 is the packet-owned delta on the docs. A3/A4 failing means a dependency
is missing: report `BLOCKED` with the packet id, do not improvise.

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `todo!` reachable from geometry.
- **H-3** No absolute constants in predicates; test epsilons carry `// H-3`
  on the SAME line as the literal.
- **Determinism**: two full battery runs must agree byte-for-byte on
  verdicts, boxes, pair lists, and counters.
- **All cargo invocations go through the queue (the `cargo` on PATH IS the
  queue shim).**

## Tests required

Named `#[test]` fns; the verifier checks the names appear in your diff:

1. `fc0_defect_correction_green_at_head` — F-C0 through the integrated
   funnel: the sphere-cap pair reaches `contact()` and certifies; the
   sampled-screen behavior is gone.
2. `fc2_enclosure_battery_green_at_head` — containment, convergence,
   monotonicity over the landed sub-box hulls, evaluated through the public
   path (not the packet-internal one).
3. `fc3_fc4_fc5_fixture_battery_at_head` — Theorem 3 certificates, Theorem 4
   implicit reduction, and the span-BVH prune counts, all through the
   integrated pipeline.
4. `v5_boolean_monotone_adjudication_record` — every recorded boolean diff
   across the program is a monotone superset; the test replays the recorded
   cases.
5. `determinism_full_program` — two full runs, byte-identical outputs.
6. `instrument_f_datum_recorded` — the histogram hook produces the stage-5
   pair-class breakdown; the test asserts the datum is extractable and the
   classes sum to the entry count.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-certified -p truck-evidence -p truck-shapeops
cargo clippy -p truck-certified -p truck-evidence -p truck-shapeops --all-targets -- -D warnings
cargo test -p truck-certified --test cfp_battery
cargo test -p truck-shapeops --test cfp_gates
```

Send cargo output to a file and read the tail. This packet IS the program's
one full verify: the loop runs `verify.py --base <integrated-HEAD-parent>` on
merge of this packet and NOTHING else.

## Forbidden

Editing any file outside `write_allow` — especially anything under `src/` of
the three crates (a production fix discovered here is a FINDING: STOP,
report, route back through a packet). Weakening any fixture ground truth to
make a gate green. Deleting defect records. Adding `#[ignore]`. Adding
`#[allow]` without a justification comment on the same line. Committing to
`main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- A3/A4 dependency missing → `BLOCKED`, name the packet
- a non-monotone V5-boolean diff → STOP and report the case (finding)
- a production-code fix seems required → STOP and report (finding, routed to
  its own packet)
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

**COMMIT BEFORE writing `RESULT.json`.** Then write `RESULT.json` at the root
of your worktree.

```json
{"id":"CFP-010-GATES","status":"DONE","contracts":["CFP-010-GATES"],
 "tests_added":6,"anchors_verified":{"A1":1,"A2":0,"A3":1,"A4":1,"A5":1},
 "f_datum":"<stage-5 plane/quadric×spline share>",
 "v5_boolean_adjudication":"<cumulative diff count, all monotone supersets>",
 "notes":"integrated battery green; defects closed against F-C0/F-C2; end-to-end cell filled; any finding stated here"}
```
