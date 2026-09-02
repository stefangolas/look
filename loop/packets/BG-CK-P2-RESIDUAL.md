# BG-CK-P2-RESIDUAL — the Phase-2 gate measurement harness (wave W3, FLOOR shape)

Wave member W3 of the Phase-2 implementation wave
(`docs/CERTIFIED_INTERLEAVE_BUILD_SPEC.md`; ORCHESTRATOR "wave mode").
Builds the Phase-2 gate measurement in the FLOOR packet's shape:
`tests/certified_phase2_floor.rs` + `docs/CERTIFIED_PHASE2_FLOOR.md`.

**This is a measurement packet.** No threshold assertions in-tree. The
certify-rate and the refusal distribution are OUTPUTS published in the
doc, never thresholds to tune against. Fail-closed is not passable by
refusing everything: the doc must show the certify rate AND the admitted
mass (the FLOOR anomaly-column discipline carries over).

**Wave-phase vs integration-phase scope (read twice).** The production
chain this harness measures (dispatch admission → Bézier decomposition →
`SquareSystem3` → 3×3 Krawczyk → branch trace) does not exist in your
tree — W1's and W2's modules land at integration. Your wave-phase
deliverable compiles against the SHIM ONLY (types + fixture kit), and the
production call sites are booked as a marked integration seam the
orchestrator amends in (mechanical, small). Do not stub production
functions, do not invent their APIs.

```yaml
id:          BG-CK-P2-RESIDUAL
contract:    [BG-CK-P2-RESIDUAL]
class:       mechanical
crates:      [look, truck-certified]
depends_on:  [BG-CK-P2-CONTRACT, BG-CK-P1-FLOOR]
write_allow:
  - tests/certified_phase2_floor.rs
  - docs/CERTIFIED_PHASE2_FLOOR.md
read_allow:
  - docs/CERTIFIED_PHASE2_BOOKING.md
  - docs/CERTIFIED_INTERLEAVE_BUILD_SPEC.md
  - loop/packets/BG-CK-P1-FLOOR.md
  - loop/results/BG-CK-P1-FLOOR.STOP.json
  - tests/certified_prevalence.rs
  - docs/CERTIFIED_PREVALENCE.md
budget:      {turns: 30, ctx_tokens: 120000}
anchors:
  - {id: A1, expect: 9, cmd: "grep -c 'certified_disjoint' loop/results/BG-CK-P1-FLOOR.STOP.json"}
  - {id: A2, expect: 0, cmd: "ls tests/certified_phase2_floor.rs 2>/dev/null | wc -l"}
  - {id: A3, expect: 0, cmd: "ls docs/CERTIFIED_PHASE2_FLOOR.md 2>/dev/null | wc -l"}
  - {id: A4, expect: 2, cmd: "grep -c '21,004' docs/CERTIFIED_PHASE2_BOOKING.md"}
  - {id: A5, expect: 7, cmd: "grep -c 'LOOK_CORPUS' tests/certified_prevalence.rs"}
  - {id: A6, expect: 2, cmd: "grep -c 'truck-certified' Cargo.toml"}
tests_required:
  - floor_harness_skips_cleanly_without_look_corpus
  - floor_refusal_distribution_buckets_are_exhaustive
  - floor_admitted_mass_is_published_not_asserted
  - floor_integration_seam_is_single_and_marked
```

## Corpus subset (the seeds must be NAMED — booking input gate 2)

The booking's spline-mass rows are the subset: spline~spline 21,004;
plane~spline 16,137; cylinder~spline 15,566; cone~spline 3,808;
spline~torus 3,923 (~60k pairs). Name WHICH FILES carry that mass the
way the prevalence census did (per-file pair counts from the landed
`certified_prevalence` buckets — copy the adjacency machinery's shape,
do not re-derive adjacency semantics). Publish the file list + pair
counts in the doc. The FLOOR STOP finding's anomaly pairs (adjacent
certified_disjoint, 4,381 mass) are NOT folded in: that is the Phase-1
dispatch/census disagreement, an open owner decision, out of scope here
— but the doc cites the STOP filing and states the exclusion explicitly.

## Section 1 — `tests/certified_phase2_floor.rs` (NEW, root crate)

FLOOR shape, wave-phase scope:

- Same loader path discipline as the FLOOR harness. NOTE: the FLOOR
  harness itself (`tests/certified_phase1_floor.rs` +
  `docs/CERTIFIED_PHASE1_FLOOR.md`) is NOT in your tree — it rides as
  WIP evidence on branch `packet/BG-CK-P1-FLOOR` (the packet STOPPED;
  its STOP filing `loop/results/BG-CK-P1-FLOOR.STOP.json` IS landed and
  is your anchor A1). For the harness SHAPE, read the FLOOR packet doc
  (`loop/packets/BG-CK-P1-FLOOR.md`, in your read_allow) — do not try
  to check out or read the WIP branch, and do not re-derive adjacency
  semantics beyond what the landed prevalence data provides.
- **The integration seam, single and marked**: one function,
  `run_certified_pair_pair(...)` (name yours), is the ONLY site that
  will call the production chain. In the wave-phase tree it is a
  `todo!()`-free stub that returns the named refusal
  `TraceRefusal`-shaped "integration pending" — NO, a stub that returns
  data would fake a measurement. It must be an `unimplemented!()`-free
  compile-only seam: the wave-phase harness compiles, and the corpus
  walk reports `integration_pending` counts for every pair WITHOUT
  pretending to certify. The doc states this plainly: wave-phase
  numbers are structural (pair counts, bucket totals, seeds named),
  the certify-rate table fills at integration.
  - Concretely: the wave-phase test asserts only structural sanity
    (corpus found, seeds named with nonzero pair mass, refusal
    distribution buckets exhaustive, every pair lands in exactly one
    disposition bucket: `certified_contact` / `certified_disjoint` /
    `refused:<cause>` / `unresolved` / `integration_pending`).
- No threshold assertion on any rate. No `unwrap` (crate denies it).
- Reuse the landed FLOOR's disposition-counting shape where it maps
  1:1; this packet's disposition vocabulary adds the trace-level named
  causes (the plan's own names): `NonTransverse`, `Conditioning`,
  `Singular` — mapped from the shim's `TraceRefusal` variants at
  integration; in the wave phase the enum exists in the harness with
  the mapping documented.

## Section 2 — `docs/CERTIFIED_PHASE2_FLOOR.md` (NEW)

- The seeds table (file, pair counts by class pair) — wave-phase, real
  numbers from the landed prevalence data.
- The disposition vocabulary + refusal-cause mapping table.
- The empty certify-rate table with its column headers fixed
  (certify-rate ≥80% is the PLAN's floor, cited as context only — the
  doc publishes measured numbers and asserts nothing) and the admitted
  mass column beside it (FLOOR anomaly-column discipline).
- The run command, the corpus provenance, and the explicit
  wave-phase/integration-phase scope statement.

## Wave-mode rules for this worker (binding)

- LOCAL checks only: `cargo check -p look` (the dev-dep edge is
  landed), `cargo test -p look --test certified_phase2_floor` with
  LOOK_CORPUS set, scoped clippy + fmt on your two files. Do NOT run
  workspace gates, do NOT run verify.py.
- The write set is exactly the two files. No manifest change, no vendor
  change, no production-code change. `tests/certified_phase1_floor.rs`
  stays byte-identical (V5-guarded).
- The corpus walk you CAN run is the prevalence-adjacency re-walk to
  name the seeds — measurement only, same loader path, no thresholds.

## Done-when

- The four `tests_required` functions exist and pass locally with
  `LOOK_CORPUS` set; the wave-phase run completes and prints the
  structural headline (seed files + pair masses + all-dispositions-
  pending is the EXPECTED wave-phase output — say so in the doc).
- fmt + scoped clippy clean; no threshold assertions in-tree; the
  integration seam is single, named, and marked in code (doc comment)
  and doc.
- RESULT.json AT THE WORKTREE ROOT (claim, not verdict), commit on the
  current branch first (subject: `feat(measure): Phase-2 gate harness
  skeleton (BG-CK-P2-RESIDUAL)`).

## Stop conditions

Stop, commit nothing beyond WIP evidence, write RESULT.json AT THE
WORKTREE ROOT with the finding verbatim if:

1. The seeds cannot be named from the landed prevalence data (the
   adjacency/pair-mass tables do not exist in the form this packet
   assumes) — record what the landed data actually provides; do not
   re-derive adjacency semantics (the FLOOR anomaly taught exactly
   this).
2. The single-marked-seam shape cannot compile without stubbing
   production behavior (e.g. the dev-dep re-export reachability is
   broken) — record the exact compiler error; that reachability is
   load-bearing and the shim's stop condition 3 covers its side.
3. The disposition vocabulary cannot hold both the FLOOR pair-level
   dispositions and the trace-level named causes without a catch-all —
   record the conflict; the vocabulary is frozen by the shim + FLOOR
   precedents.
