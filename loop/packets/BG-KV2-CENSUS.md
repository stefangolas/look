# BG-KV2-CENSUS — measurement: representation-recovery census over the Phase-2 funnel

Elastic-pool measurement packet (build spec §2 decision 3 — CORRECTED: the
booking's "census WIP is archived" premise measured FALSE; the patch is a
72-byte stub with no captured work, so this is a FRESH harness, not a
resurrection). It re-walks the booked spline face-pair corpus with
representation-recovery instrumentation: for every funnel stage, WHERE the
mass dies (carrier recognition, patch extraction, admission screens) and
what the v2 rational-carrier world (§3.2) would change about that.

Pattern: `tests/certified_phase2_floor.rs` (the W3 harness) is the template —
same corpus, same discipline: **the re-walk must reproduce the booked totals
EXACTLY (60,438 face-pairs; 726 admitted; the booked refusal totals) before
any new counter's output is published.** The census numbers are OUTPUTS,
never thresholds. `tests/certified_phase2_floor.rs` is READ-ONLY context and
is NOT in this packet's write set — copy what is not `pub`, never clobber it
(the P7 rule).

```yaml
id:          BG-KV2-CENSUS
contract:    [BG-KV2-CENSUS]
class:       measurement
crates:      [look]
depends_on:  [BG-KV2-000-CONTRACT]
write_allow:
  - tests/kernel_v2_census.rs
  - docs/KERNEL_V2_CENSUS.md
read_allow:
  - tests/certified_phase2_floor.rs
  - docs/CERTIFIED_PHASE2_FLOOR.md
  - docs/CERTIFIED_PREVALENCE.md
  - docs/CONSTRUCTIVE_GEOMETRY_KERNEL_SPEC_V2.md
  - docs/KERNEL_V2_BUILD_SPEC.md
budget:      {turns: 26, ctx_tokens: 90000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'not_admitted_reasons' tests/certified_phase2_floor.rs"}
  - {id: A2, expect: 1, cmd: "grep -c '60438\\|60,438' docs/CERTIFIED_PHASE2_FLOOR.md"}
  - {id: A3, expect: 1, cmd: "grep -c 'representation_name' tests/certified_phase2_floor.rs"}
  - {id: A4, expect: 0, cmd: "grep -c 'kernel_v2_census' Cargo.toml"}
tests_required:
  - census_rewalk_reproduces_booked_funnel_totals
  - representation_census_covers_every_booked_pair
  - leaf_extraction_applicability_counts_published
```

## Section 1 — the harness (`tests/kernel_v2_census.rs`, NEW, root look crate)

Reuses the W3 harness's loading/classification machinery by copying its
shapes (it is a test file — import what is `pub`, copy what is not, and say
so in a provenance comment). New instrumentation, all counts keyed by the
W3 funnel stages (admission_refused / rational-form mismatch /
non-spline-carrier / patch-pair product / traced / completed):

1. **Carrier-kind census** — every booked pair, both sides: the
   `representation_name`/`classify` outcome, cross-tabulated. Publishes the
   (surface-kind, surface-kind) -> count table. Ground truth: the table's
   grand total MUST equal 60,438 exactly.
2. **Representation-recovery mass** — per pair admitted to patch
   extraction: count of `spline_face_patches` invocations and their
   sub-patch yields, and the share of total walk wall-time spent in
   extraction vs admission screens vs certified engine consults (timed
   with std::time::Instant, medians over 3 runs, hardware fingerprint from
   `look doctor --json` NOT required — CPU timing only, recorded as
   machine-local).
3. **Leaf-applicability census** — for each extracted patch: would
   knot-span Bézier extraction (the KV2-102 contract shape: per-span
   homogeneous Bézier nets, positive control weights) produce leaves
   directly? Count per patch: extractable-as-is / needs-weight-repair
   (negative or zero control weight present) / degree-out-of-band. This is
   the v2 §3.2 delta quantified. NOT a full leaf implementation — the
   census applies the extraction algorithm inline at the census site (or
   imports it if KV2-102 has landed first; both paths recorded).
4. **Re-scored funnel** — under a hypothetical that every
   extractable-as-is patch is consumed as a BezierLeaf: which booked
   refusals change class? Publish the delta table against the W3 booked
   numbers. Labeled HYPOTHETICAL in the doc — no landed behavior changes.

## Section 2 — `docs/KERNEL_V2_CENSUS.md` (NEW)

The published output: the four tables, the exact reproduction statement
(walk seeds, totals), the machine-local timing caveat, and one paragraph
relating the numbers to build-spec Wave-2 scoping (which admission screens
the rational-carrier world actually unlocks). No performance claims — this
is a census, and the AGENTS.md rule about recorded performance numbers
applies to anything that looks like one.

## Done-when

- `cargo test -p look --test kernel_v2_census -- --nocapture` green,
  reproducing the booked totals EXACTLY first (test 1).
- The doc carries all four tables with the grand-total equality stated.
- `cargo check --workspace --all-targets` green; fmt clean; clippy
  (exact verify form, unfiltered) clean on the new files.
- CARGO_BUILD_JOBS=2-4 on every cargo invocation.

## Stop conditions

1. The re-walk does NOT reproduce a booked total — stop; the corpus or the
   harness drifted and every published census number would be built on it.
   Record which total, both values.
2. The corpus table files are not at the paths the W3 harness reads —
   stop, name the path tried (do not improvise a substitute corpus).
3. Leaf-applicability classification needs a decision the spec does not
   make (e.g. degree out of band) — record the chosen band in the doc and
   RESULT notes; it is a census classification, not a kernel decision.

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

Commit on the current branch (subject: `test(census): representation-
recovery census over the Phase-2 funnel (BG-KV2-CENSUS)`) BEFORE writing
`RESULT.json`.
