# Certified Phase-2 floor — the gate measurement harness (wave W3, FLOOR shape)

**Packet.** `BG-CK-P2-RESIDUAL` (Phase-2 implementation wave, member W3). The
Phase-2 gate measurement in the FLOOR packet's shape: `tests/
certified_phase2_floor.rs` + this doc. The production chain this harness
measures (dispatch admission → Bézier decomposition → `SquareSystem3` → 3×3
Krawczyk → branch trace) does NOT exist in this tree — W1's and W2's modules
land at integration. This is a **measurement packet**: the certify-rate and the
refusal distribution are OUTPUTS published here, never thresholds. Fail-closed
is not passable by refusing everything: the doc shows the certify-rate AND the
admitted mass (the FLOOR anomaly-column discipline carries over).

**Wave-phase scope (read first).** All numbers below are **structural**: pair
counts, bucket totals, and named seeds. Every measured pair in this wave-phase
run is reported `integration_pending`, and **all-dispositions-pending is the
EXPECTED wave-phase output** — the production chain is not landed, so no pair
is certified, refused, or left unresolved here. The certify-rate table (below)
is empty by design and fills at integration, when the single integration seam
is wired to the production chain. Nothing in this doc is a threshold.

## Headline (wave-phase structural run)

The wave-phase run completed on the full `LOOK_CORPUS` checkout (38 STEP
files). The aggregate, verbatim from the harness's
`CERTIFIED_PHASE2_FLOOR_AGGREGATE` line:

```
CERTIFIED_PHASE2_FLOOR_AGGREGATE {"admitted_mass":60438,"certified_contact":0,"certified_disjoint":0,"certify_rate":null,"files":38,"integration_pending":60438,"refused":{"coincident_circles":0,"conditioning":0,"non_transverse":0,"overlap":0,"singular":0,"unrelated_tangency":0,"unsupported_pair_class":0},"seed_rows":{"cone~spline":3808,"cylinder~spline":15566,"plane~spline":16137,"spline~spline":21004,"spline~torus":3923},"seeds":60438,"unresolved":0}
```

Structural reading:

- The corpus subset rows are exactly the booking's spline-mass rows
  (`docs/CERTIFIED_PHASE2_BOOKING.md`): spline~spline 21,004; plane~spline
  16,137; cylinder~spline 15,566; cone~spline 3,808; spline~torus 3,923 —
  **60,438 seeds in total**, reproduced exactly from the landed prevalence
  buckets (`docs/CERTIFIED_PREVALENCE.md` pair histogram) by re-walking the
  corpus with the prevalence-adjacency machinery (same loader path, no
  re-derived adjacency semantics). The harness asserts these rows exist with
  nonzero pair mass; the exact equality to the booked totals is the re-walk's
  machine cross-check (printed above, not threshold-asserted in-tree).
- `admitted_mass` = 60,438 (the mass the gate will hand the seam at
  integration). `integration_pending` = 60,438. All other disposition buckets
  are structurally zero, because no dispatch or trace ran.
- `certify_rate` is `null` in the wave phase. It fills at integration.
- 38 files measured, none excluded. The loader's per-entity conformance
  warnings on stderr (the NIST suite's "Error while deserialize STEP struct:
  ..." lines) do not exclude any file — the identical table-builder behavior
  the prevalence census documented.

### The FLOOR anomaly exclusion (explicit)

The FLOOR STOP filing (`loop/results/BG-CK-P1-FLOOR.STOP.json`) found 4,381
adjacent face pairs answered exactly `certified_disjoint` by the Phase-1
dispatch — an anomaly column firing at mass (concentrated in cylinder~plane
3,600 and cylinder~cylinder 746), i.e. a disagreement between the census's
adjacency enumeration and the dispatch's exact admission screens about what a
pair IS. **Those anomaly pairs are NOT folded into this measurement.** That
disagreement is a Phase-1 dispatch/census concern and an open owner decision;
out of scope here. Consistently, the wave-phase run reports
`certified_disjoint` = 0 — not because any pair was certified disjoint, but
because no Phase-1 dispatch runs in this tree. At integration the Phase-2 gate
measures the spline-residual subset only; the anomaly's pair classes are not
part of it.

## The seeds table — which files carry the Phase-2 mass

Per-file spline-mass pair counts from the recorded run, one row per file that
carries any mass. `seeds` is that file's adjacent-pair mass in the five
subset rows. Files not listed carry zero Phase-2 mass.

### Large assemblies

| file | cone~spline | cylinder~spline | plane~spline | spline~spline | spline~torus | seeds |
|---|---|---|---|---|---|---|
| `quadruped/quadruped.step` | 850 | 7,803 | 6,289 | 9,974 | 1,481 | 26,397 |
| `jackhammer/jackhammer.step` | 1,866 | 4,369 | 5,439 | 4,073 | 932 | 16,679 |
| `formula1/formula1.step` | 516 | 1,143 | 2,302 | 3,782 | 1,224 | 8,967 |
| `ur10/ur10.step` | 509 | 1,680 | 1,717 | 2,957 | 266 | 7,129 |
| `core_xy/core_xy.step` | 39 | 196 | 240 | 167 | 8 | 650 |
| **Assemblies total** | **3,780** | **15,191** | **15,987** | **20,953** | **3,911** | **59,822** |

### NIST files carrying mass

| file | cone~spline | cylinder~spline | plane~spline | spline~spline | spline~torus | seeds |
|---|---|---|---|---|---|---|
| `nist/NIST-PMI-STEP-Files/AP203 geometry only/nist_ctc_02_asme1_rc.stp` | 10 | 60 | 34 | 10 | 0 | 114 |
| `nist/NIST-PMI-STEP-Files/AP203 with PMI/nist_ctc_02_asme1_ap203.stp` | 8 | 60 | 28 | 2 | 0 | 98 |
| `nist/NIST-PMI-STEP-Files/nist_ctc_02_asme1_ap242-e2.stp` | 10 | 60 | 28 | 10 | 0 | 108 |
| `nist/NIST-PMI-STEP-Files/AP203 geometry only/nist_ftc_07_asme1_rd.stp` | 0 | 48 | 16 | 8 | 0 | 72 |
| `nist/NIST-PMI-STEP-Files/nist_ftc_07_asme1_ap242-e2.stp` | 0 | 48 | 16 | 8 | 0 | 72 |
| `nist/NIST-PMI-STEP-Files/nist_stc_07_asme1_ap242-e3.stp` | 0 | 48 | 16 | 8 | 0 | 72 |
| `nist/NIST-PMI-STEP-Files/AP203 geometry only/nist_ctc_05_asme1_rd.stp` | 0 | 21 | 4 | 5 | 4 | 34 |
| `nist/NIST-PMI-STEP-Files/AP203 with PMI/nist_ctc_05_asme1_ap203.stp` | 0 | 12 | 4 | 0 | 4 | 20 |
| `nist/NIST-PMI-STEP-Files/nist_ctc_05_asme1_ap242-e1.stp` | 0 | 12 | 4 | 0 | 4 | 20 |
| `nist/NIST-PMI-STEP-Files/AP203 geometry only/nist_ftc_10_asme1_rb.stp` | 0 | 2 | 0 | 0 | 0 | 2 |
| `nist/NIST-PMI-STEP-Files/nist_ftc_10_asme1_ap242-e2.stp` | 0 | 2 | 0 | 0 | 0 | 2 |
| `nist/NIST-PMI-STEP-Files/nist_stc_10_asme1_ap242-e2.stp` | 0 | 2 | 0 | 0 | 0 | 2 |
| **NIST total** | **28** | **375** | **150** | **51** | **12** | **616** |

Cross-check: assemblies 59,822 + NIST 616 = **60,438** = the aggregate `seeds`.
The five large assemblies carry 98.98 % of the Phase-2 mass; the remaining 21
NIST files carry none. The prevalence doc's per-file face counts reproduce
exactly (`core_xy` 5,670 faces / 79 shells, `quadruped` 29,392 faces / 195
shells, etc.), confirming this is the same adjacency enumeration re-walked,
not a re-derivation.

## Disposition vocabulary + refusal-cause mapping

Every measured pair lands in exactly one of five top-level buckets:
`certified_contact` / `certified_disjoint` / `refused:<cause>` / `unresolved` /
`integration_pending`. The `refused:<cause>` causes are all named — no
catch-all. The vocabulary holds BOTH the FLOOR pair-level dispositions (the
landed Phase-1 gate's `refused_unsupported` causes, carried over 1:1) and the
Phase-2 trace-level named causes (the plan's own names).

| bucket | meaning | integration source |
|---|---|---|
| `certified_contact` | a certified locus for the pair | the trace's certified branch output (mapping section C row 3) |
| `certified_disjoint` | a certified no-contact answer | the trace certifying no shared locus |
| `refused:overlap` | FLOOR pair-level cause (landed `PairUnsupported::Overlap`) | the seam's pre-trace admission screens |
| `refused:coincident_circles` | FLOOR pair-level cause (`PairUnsupported::CoincidentCircles`) | the seam's pre-trace admission screens |
| `refused:unrelated_tangency` | FLOOR pair-level cause (`PairUnsupported::UnrelatedTangency`) | the seam's pre-trace admission screens |
| `refused:unsupported_pair_class` | FLOOR pair-level cause (`PairUnsupported::UnsupportedPairClass`) | the seam's pre-trace admission screens (the DISPATCH widening) |
| `refused:non_transverse` | trace-level named cause (the plan's own name): the trace could not certify a transverse crossing | mapped at integration from the shim `TraceRefusal` variants — the `Hull(_)` enclosure failures and the `Unresolved(GenericUnresolved)` tangency/stationary/boundary families (`UnresolvedStationaryBranch`, `UnresolvedTangencyOrSingularity`, `UnresolvedBoundaryRoot`, ...); the exact arm list is refined by the integration amendment |
| `refused:conditioning` | trace-level named cause (the plan's own name): the frozen F3 rule refused the box | `TraceRefusal::Conditioning(Refusal::ConditioningBelowThreshold)` — never retried with a weaker test (F3) |
| `refused:singular` | trace-level named cause (the plan's own name): a certified singular / collapsed-stratum branch reading | mapped at integration from the shim `TraceRefusal` variants and the landed germ classes — `BranchGerm::Singular` / `CuspCandidate`, and the `Unresolved(GenericUnresolved)` singular families (`SingularJacobian`, `UnsupportedSingularBranch`); the exact arm list is refined by the integration amendment |
| `unresolved` | per the landed unresolved causes | the trace's `TraceOutcome`/germ unresolved cases |
| `integration_pending` | the pair was not measured because the chain is not landed | the wave-phase-only bucket; every pair here at wave phase |

The trace-level named causes are mapped from the shim's `TraceRefusal`
variants (`truck_certified::TraceRefusal` and the `ssi_types`/`ssi_fixtures`
wave shim, BG-CK-P2-CONTRACT) **at integration**; in the wave-phase tree the
enum exists in the harness with this mapping documented and receives no counts.

## The certify-rate table (EMPTY — fills at integration)

Certify-rate ≥ 80 % is the PLAN's floor, cited as context only. This doc
publishes measured numbers and asserts nothing; the table below is empty
because no pair is certified in the wave phase. The **admitted mass column**
sits beside it (FLOOR anomaly-column discipline): a below-floor rate must be
read against the mass that was admitted, so refusing everything can never
masquerade as the gate passing.

| Subset row | admitted mass (wave-phase seeds) | certified | refused:<cause> | unresolved | certify-rate |
|---|---|---|---|---|---|
| `spline~spline` | 21,004 | — | — | — | — |
| `plane~spline` | 16,137 | — | — | — | — |
| `cylinder~spline` | 15,566 | — | — | — | — |
| `cone~spline` | 3,808 | — | — | — | — |
| `spline~torus` | 3,923 | — | — | — | — |
| **Total** | **60,438** | — | — | — | — |

At integration the run fills this table from the seam's real dispositions and
publishes the refusal distribution by cause alongside the certify-rate and the
admitted mass. The plan floor (≥ 80 %) is a context citation, never an
in-tree assertion.

## Wave-phase vs integration-phase scope

- **Wave phase (this deliverable):** the harness compiles against the SHIM
  only (types + fixture kit via the landed dev-dependency edge
  `truck-certified = { path = "vendor/truck/truck-certified" }`) and the corpus
  walk reports the structural headline. Numbers here are pair counts, bucket
  totals, and named seeds — real, from the landed prevalence data, machine
  cross-checked against the booked totals.
- **The integration seam is single, named, and marked** (in code and doc):
  `run_certified_pair_pair(...)` in `tests/certified_phase2_floor.rs` is the
  ONLY site that will call the production chain at integration. In the
  wave-phase tree it is a compile-only seam — it is `unimplemented!()`-free,
  is never called by the corpus walk (a stub that returned data would fake a
  measurement), and every measured pair is counted `integration_pending`
  without pretending to certify. The seam's parameter names the shim's frozen
  `SquareSystem3` carrier, pinning the dev-dependency re-export reachability
  the integration relies on. At integration the orchestrator amends the seam to
  the production chain's real inputs and routes each measured pair through it;
  the certify-rate table above then fills.
- The structural tests assert only: corpus found; seeds named with nonzero
  mass; refusal distribution buckets exhaustive (no catch-all); every pair
  lands in exactly one disposition bucket. There is **no threshold assertion on
  any rate** anywhere in-tree, and no `unwrap` (the crate denies it).

## Reproduction

```console
$env:LOOK_CORPUS = "C:\Users\stefa\look-corpus"
cargo test -p look --test certified_phase2_floor -- --nocapture --test-threads=1
```

The test prints one JSON row per file (`file`, `shells`, `faces`, `seeds`,
`seed_rows`) plus the `CERTIFIED_PHASE2_FLOOR_AGGREGATE` line carrying the
structural headline. When `LOOK_CORPUS` is unset the harness skips cleanly
with a clear message (the four structural tests still pass). The recorded
numbers above came from a **debug (test profile)** run on this machine
(`target/debug`); they are structural pair counts and are deterministic across
build profiles (no timing or performance claims are made here), and a
`--release` run reproduces them.

## Corpus provenance

Corpus: the full `LOOK_CORPUS` checkout at `C:\Users\stefa\look-corpus`
(38 STEP files), the same corpus the prevalence census
(`docs/CERTIFIED_PREVALENCE.md`, `tests/certified_prevalence.rs`) measured.
Loader path: the landed import path the renderer uses (`src/step.rs`):
`look::step::part21::parse` with the `ruststep::parser::parse` fallback,
`Table::from_owned_data_section`, then `Table::to_compressed_shell_with_losses`
per shell — identical to the prevalence census; adjacency is the census's
relation (two faces sharing a compressed edge), copied, not re-derived. The
classifier is the prevalence classifier verbatim. Seeds are the five
spline-mass rows from the booking; the FLOOR anomaly pairs are excluded (see
above).
