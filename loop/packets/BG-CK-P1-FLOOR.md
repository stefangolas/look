# BG-CK-P1-FLOOR — the Phase-1 gate measurement: certify-rate and throughput on the corpus

Certified-kernel Phase 1, fifth packet — the Phase-1 exit-gate measurement
(plan §3, `docs/CERTIFIED_PHASE1_BOOKING.md`): for every corpus pair the
landed dispatch ADMITS, run the certified path and count
certify/refuse/unresolved; publish the certify-rate (the floor is 95%) and
the throughput comparison. Publishes `docs/CERTIFIED_PHASE1_FLOOR.md`. The
measurement is the output — **no threshold assertions in-tree** (the same
discipline as the prevalence census: a threshold would make the measurement
self-fulfilling).

Corpus basis: the prevalence census (`docs/CERTIFIED_PREVALENCE.md`,
38 files, 166,307 pairs; the analytic rows carry 103,649). The admitted
classes are the DISPATCH exact arms (plane~plane 26,274; cylinder~plane
37,361; plane~sphere 281; sphere~sphere 126; the coaxial/parallel subset
of cylinder~cylinder 5,354) — the dispatch's own admission screens decide
per pair; this packet does not pre-admit by class alone.

```yaml
id:          BG-CK-P1-FLOOR
contract:    [BG-CK-P1-FLOOR]
class:       mechanical
crates:      [look]
depends_on:  [BG-CK-P1-DISPATCH]
write_allow:
  - tests/certified_phase1_floor.rs
  - docs/CERTIFIED_PHASE1_FLOOR.md
  - Cargo.toml
  - Cargo.lock
read_allow:
  - CERTIFIED-KERNEL-PLAN.md
  - docs/CERTIFIED_PHASE1_BOOKING.md
  - docs/CERTIFIED_PREVALENCE.md
  - docs/CERTIFICATE_MAPPING.md
  - tests/certified_prevalence.rs
  - vendor/truck/truck-certified/src/pair_dispatch.rs
  - vendor/truck/truck-certified/src/formal/sphere.rs
  - vendor/truck/truck-certified/src/formal/support.rs
  - vendor/truck/truck-certified/src/formal/cylinder.rs
  - vendor/truck/truck-certified/src/lib.rs
budget:      {turns: 25, ctx_tokens: 90000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub fn dispatch_pair' vendor/truck/truck-certified/src/pair_dispatch.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub enum CertifiedPairParticipant' vendor/truck/truck-certified/src/pair_dispatch.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub enum ContactLocus' vendor/truck/truck-certified/src/pair_dispatch.rs"}
  - {id: A4, expect: 0, cmd: "ls tests/certified_phase1_floor.rs 2>/dev/null | wc -l"}
  - {id: A5, expect: 7, cmd: "grep -c 'LOOK_CORPUS' tests/certified_prevalence.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub struct CertifiedEmbeddedSphere' vendor/truck/truck-certified/src/formal/sphere.rs"}
tests_required:
  - floor_measurement_is_ignored_and_skips_cleanly_without_corpus
  - every_admitted_pair_gets_exactly_one_disposition
  - adjacent_pair_disjoint_answers_are_counted_as_an_anomaly_column
  - aggregate_line_carries_certify_rate_and_latency_distribution
  - no_threshold_assertion_in_tree
```

## Pre-made decisions (do not relitigate; quote the tags into the doc)

**D-self-contained.** Integration test files cannot import each other: the
floor test is SELF-CONTAINED (it re-walks the corpus with the same
classification walk as `tests/certified_prevalence.rs`, cited as
provenance, not imported). Duplication between the two harness files is
accepted; the census file stays byte-identical (it is NOT in write_allow —
the V5 identity guard enforces this).

**D-ignored.** The measurement test is `#[ignore]` and gated on
`LOOK_CORPUS` exactly like the census (anchor A5): skips with a clear
message when unset. Release-build only is NOT required (no debug-panic
fixtures here — the corpus walk and dispatch_pair never tessellate), but
the published numbers come from a release run; the doc states the build
profile of the recorded run.

**D-disposition.** Every pair the dispatch ADMITS gets exactly one
disposition, and all five are reported:

| Disposition | Meaning |
|---|---|
| `certified_contact` | `Contact(_)` — a certified locus for the pair |
| `certified_disjoint` | `Disjoint` — a certified answer, BUT for ADJACENT faces (the census's pair enumeration) a disjoint answer is an ANOMALY: adjacent faces share an edge, so a disjoint locus signals a representation/admission mismatch, not a success. Counted in the certify numerator AND reported as its own anomaly column with per-class breakdown |
| `refused_unsupported` | `Unsupported(_)` — the typed no-silent-downgrade boundary; broken down by `PairUnsupported` variant |
| `unresolved` | `Unresolved(_)` — per variant |
| `not_admitted` | The dispatch refused before admission (constructors refused, participants unroutable) — reported per class so the admitted mass is visible |

The certify-rate is `(certified_contact + certified_disjoint) / admitted`
with `admitted = certified_contact + certified_disjoint +
refused_unsupported + unresolved`. The floor is 95%: a run below it is a
FINDING published in the doc, never a test failure.

**D-anomaly-first.** `certified_disjoint` on adjacent pairs is the
measurement's most important diagnostic: if it fires at mass, the
dispatch's admission screens and the census's adjacency enumeration
disagree about what a pair IS, and the certify-rate would be flattered by
counting the disagreement as success. The doc leads with this column.

**D-latency.** The test measures per-pair wall time of `dispatch_pair`
over the admitted set (monotonic clock around each call, the corpus walk
excluded), publishing min/median/p95/max in the aggregate. The legacy
comparator half of the Phase-1 gate: pre-decided fallback — if the moved
`formal/` transversal machinery exposes a directly callable pair-contact
entry the test can drive on the same pairs, measure and publish it side
by side (cite the entry by name in the doc); if none is directly callable
from the test crate, publish the certified distribution alone and record
the throughput comparison as DEFERRED TO INTEGRATION in the doc (the
exit-gate's throughput half is then measured where the certified path is
wired into the real pipeline — not manufactured here). Either way is a
published finding, not a gate.

**D-output.** The aggregate prints one line:
`CERTIFIED_PHASE1_FLOOR_AGGREGATE {"files", "pairs", "admitted",
"certified_contact", "certified_disjoint", "refused_unsupported",
"unresolved", "certify_rate", "latency_ns_min", "latency_ns_median",
"latency_ns_p95", "latency_ns_max"}` (census format discipline). The doc
`docs/CERTIFIED_PHASE1_FLOOR.md` carries: the aggregate verbatim, the
per-file table, the per-class breakdown, the anomaly column analysis, the
build profile, and the reproduction command. No throughput or floor
claims beyond what the recorded run shows.

## Section 1 — `tests/certified_phase1_floor.rs` (NEW)

Structure mirrors the census harness: corpus walk → shell → face
classification (the same seven buckets, the same identify_* entry points)
→ adjacent pair enumeration → for pairs whose both sides are admitted
classes, construct `CertifiedPairParticipant`s through the landed
identification routes and call `dispatch_pair`. Timing: one
`Instant::now()/elapsed()` pair per dispatch call, collected into a Vec,
aggregated at the end (no timing inside the classification walk). The
five dispositions are one enum in the test; the aggregate JSON is built
from the counters.

## Section 2 — `docs/CERTIFIED_PHASE1_FLOOR.md` (NEW)

The doc is written by the test run's output being transcribed by the
worker from the recorded run (the run happens on this machine with
`LOOK_CORPUS` set; the reproduction command is in the doc). If the corpus
is unavailable in the worker's environment, the doc lands with the
structure, the empty-run placeholders, and a RESULT note saying the
recorded numbers await an owner-side run — the packet's done-when
adjusts accordingly (state which happened).

## Section 3 — house rules

No threshold assertion anywhere in the test (`no_threshold_assertion_in_tree`
is a required test NAME and a source-scan discipline: no `assert!` on a
rate, ever). Clippy zero findings on the new files; pre-existing baseline
findings out of scope. `crates: [look]`.

**AMENDED (r2, orchestrator — the r1 SPEC_GAP):** `dispatch_pair` and the
pair-dispatch types are NOT reachable from the look test target through
the meshalgo compat re-exports (the r1 worker verified this empirically:
E0433/E0432; the re-export path carries the identification witnesses but
not `pair_dispatch`). The fix: add EXACTLY ONE line to the root
`Cargo.toml`'s `[dev-dependencies]`:

```toml
truck-certified = { path = "vendor/truck/truck-certified" }
```

A dev-only edge: it changes no production code (dev-dependencies are
invisible to `look`'s own src), adds no vendor change, and the lockfile
line rides in `Cargo.lock` (both are in write_allow). After that,
`use truck_certified::pair_dispatch::{dispatch_pair, CertifiedPairParticipant,
CertifiedPairResult, ContactLocus};` compiles in the test target and the
packet proceeds exactly as written. No other manifest change is wanted;
the "add NOTHING to the manifest" instruction of r1 is superseded by this
one named edge.

## Done-when

- `cargo fmt` clean on the new files.
- `cargo clippy -p look --all-targets --message-format=short --no-deps` —
  zero findings attributable to the new test file.
- `cargo test -p look --lib --tests --no-fail-fast` green with the new
  test SKIPPING (no corpus in the worker environment) — the census test's
  own skip proves the pattern.
- `cargo check --workspace --all-targets` green.
- If the corpus WAS available: the recorded aggregate in the doc matches
  the printed `CERTIFIED_PHASE1_FLOOR_AGGREGATE` line verbatim.

## Stop conditions

Stop, commit nothing beyond WIP evidence, write RESULT.json (AT THE
WORKTREE ROOT) with the finding verbatim if:

1. The landed dispatch surface differs from the anchors (a renamed
   participant, a different result shape) — do not adapt silently; the
   packet's anchors were written against the DISPATCH packet's contract.
2. The census's classification walk cannot be re-derived from the landed
   identify_* entries (a route the census uses is not public) — record
   the gap; the fix is an orchestrator decision, never a test-side
   reimplementation of an identifier.
3. The anomaly column (`certified_disjoint` on adjacent pairs) fires at
   mass in a recorded run — do not "fix" the dispatch or the census;
   publish the finding and stop. A disagreement between two landed
   measurements is the loop's most valuable output.

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

Commit your work on the current branch (subject: `feat(look): Phase-1
floor measurement harness (BG-CK-P1-FLOOR)`) BEFORE writing `RESULT.json`.
