# Certified Phase-2 floor — the gate measurement (wave W3, FLOOR shape)

**Packet.** `BG-CK-P2-RESIDUAL` (Phase-2 implementation wave, member W3), in
its integration-amendment form: the composed chain landed (W1 `ssi.rs` +
W2 `ssi_trace.rs`), the single marked integration seam
(`run_certified_pair_pair` in `tests/certified_phase2_floor.rs`) is wired to
`truck_certified::ssi_trace::certified_pair_trace`, and the gate measurement
ran on the corpus. This is a **measurement packet**: the certify-rate and the
refusal distribution are OUTPUTS published here, never thresholds. Fail-closed
is not passable by refusing everything: the doc shows the certify-rate AND the
admitted mass (the FLOOR anomaly-column discipline carries over).

**The measurement is the wave's output, and it is a FINDING, not a pass.** The
composed chain's real behavior on corpus geometry diverges sharply from its
fixture claims: the corpus's spline mass is blocked at extraction/admission by
three named carrier forms, and the small admitted remainder refuses at the
trace level (0 certified in every fully-measured pair). This is recorded here,
not tuned around — the wave amendment's instruction verbatim.

## Headline (integration run, measured)

The run completed on the full `LOOK_CORPUS` checkout (38 STEP files) under the
certified-trace budget. The aggregate, verbatim from the harness's
`CERTIFIED_PHASE2_FLOOR_AGGREGATE` line:

```
CERTIFIED_PHASE2_FLOOR_AGGREGATE {"admitted_mass":726,"admitted_pairs":726,"admitted_rows":{"spline~spline":726},"certified_contact":0,"certified_disjoint":0,"certify_rate":0.0,"completed_pairs":6,"completion_fraction":0.008264462809917356,"files":38,"not_admitted_reasons":{"admission_refused":21566,"non_spline_carrier":28790,"rational_nurbs":9356},"refused":{"coincident_circles":0,"conditioning":2,"non_transverse":3,"overlap":0,"singular":1,"unrelated_tangency":0,"unsupported_pair_class":0},"seed_rows":{"cone~spline":3808,"cylinder~spline":15566,"plane~spline":16137,"spline~spline":21004,"spline~torus":3923},"seeds":60438,"truncated_pairs":720,"unit_pairs_total":226654,"unit_pairs_traced":400,"unresolved":0,"wall_seconds":279.240241}
CERTIFIED_PHASE2_FLOOR_BUDGET_EXHAUSTED completed=6 admitted=726 unit_pairs_traced=400 wall_seconds=279.2
```

Structural + measured reading:

- The structural seeds reproduce the booking exactly (spline~spline 21,004;
  plane~spline 16,137; cylinder~spline 15,566; cone~spline 3,808; spline~torus
  3,923; **60,438** seeds over 38 files) — the wave-phase table's numbers hold.
- **Admitted mass: 726 face pairs (1.2 % of seeds), all in the `spline~spline`
  row** — the only pairs whose two spline carriers are non-rational B-splines
  that the LANDED whole-domain decomposition (`certified_map::admit_surface`)
  admits. The remaining 59,712 subset pairs cannot reach the landed
  decomposition; the reason split is published (below).
- **Certify-rate (measured over completed pairs): 0.0** — of the 726 admitted
  pairs, the run fully dispositioned 6 before the certified-trace budget was
  spent (completion fraction 0.83 %); every one of the 6 refused with a named
  cause (conditioning 2, non-transverse 3, singular 1). `certified_contact` =
  0, `unresolved` = 0.
- **Unit-pair totals:** the 726 admitted face pairs carry 226,654 unit-pairs
  (the full patch-pair products); 400 were traced (the budget) before the run
  stopped. 720 of 726 admitted pairs are truncated (not dispositioned) — never
  silently dropped: the completion fraction and wall time are published.
- 38 files measured, none excluded. Run profile: debug (test profile);
  wall time 279.2 s. The loader's NIST conformance warnings on stderr excluded
  nothing (identical table-builder behavior to the prevalence census).

### The finding (recorded, not tuned around)

The phase-2 chain, measured on the corpus, does not certify: the whole corpus
is blocked before or at the trace. Per the wave amendment, this divergence is
the loop's most valuable output:

1. **47.6 % of seeds (28,790 pairs) carry a non-spline carrier** on one side
   (plane/cylinder/cone/torus). The composed engine's own typed boundary
   (`construct_square_system` on a `NonSpline` participant) refuses
   `UnsupportedPairClass`; there is no landed rational-Bézier route from an
   analytic carrier to a `RationalBipatch` in this tree. These rows
   (plane~spline, cylinder~spline, cone~spline, spline~torus) are therefore
   outside what the composed chain can admit as written.
2. **15.5 % of seeds (9,356 pairs) carry a rational (NURBS) spline surface.**
   The landed surface decomposition (`certified_map`, D-map) is declared
   non-rational-only; no rational surface Bézier cut exists in the tree. These
   faces cannot reach the landed decomposition.
3. **35.7 % of seeds (21,566 pairs) are non-rational B-spline pairs whose
   whole-domain admission refused** (`MapRefusal` under the declared τ =
   1e-6). The Phase-1 map's D-tau rule refuses a domain whose certified rank
   margin is not above τ (true degeneracy or cannot-decide); its documented
   remedy is per-subregion admission, which the landed public surface does not
   expose at face level. At face granularity the landed path cannot admit
   them.
4. **The 726 admitted pairs (1.2 %) systematically refuse at the trace.**
   Every fully-measured pair refused with a named cause (F3 conditioning,
   non-transverse hull/inclusion, singular determinant) — behavior the W1/W2
   fixture suite never showed. The certify-rate over the completed subset is
   0.0, far below the plan's ≥80 % floor (context only).

Together these make the Phase-2 gate NOT measured by this run in any
gate-passing sense; what is measured is the reachability and refusal
distribution above. That distribution is the deliverable.

## The FLOOR anomaly exclusion (explicit)

The FLOOR STOP filing (`loop/results/BG-CK-P1-FLOOR.STOP.json`) found 4,381
adjacent face pairs answered exactly `certified_disjoint` by the Phase-1
dispatch — an anomaly column firing at mass, i.e. a disagreement between the
census's adjacency enumeration and the dispatch's exact admission screens about
what a pair IS. **Those anomaly pairs are NOT folded into this measurement**;
that disagreement is a Phase-1 dispatch/census concern and an open owner
decision, out of scope here. Consistently, the integration run reports
`certified_disjoint` = 0 — no pair was certified disjoint (the composed trace
engine certifies branches, not emptiness; no dispatch ran on the blocked mass).

## The seeds table — which files carry the Phase-2 mass

Structural re-walk output (per-file spline-mass pair counts; files not listed
carry zero Phase-2 mass). These numbers are unchanged from the wave-phase run
and reproduce the landed prevalence buckets.

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

Assemblies 59,822 + NIST 616 = 60,438. The five large assemblies carry
98.98 % of the Phase-2 mass; the remaining 21 NIST files carry none.

## Admitted vs blocked (the measurement's reachability)

The full patch-pair product is the unit of measurement per admitted FACE pair.
Only 726 of the 60,438 subset pairs are admitted to the trace chain; every
other subset pair is blocked by a named carrier reason. `not_admitted_reasons`
from the aggregate, verbatim:

| reason | subset pairs | share of seeds | meaning |
|---|---|---|---|
| `non_spline_carrier` | 28,790 | 47.6 % | one side is an analytic carrier (plane/cylinder/cone/torus); the composed engine refuses `NonSpline` participants and no analytic-to-bipatch route is landed |
| `admission_refused` | 21,566 | 35.7 % | both sides non-rational B-spline, but the landed whole-domain admission (`certified_map`, τ = 1e-6) refused (D-tau: degenerate or cannot-decide at face granularity) |
| `rational_nurbs` | 9,356 | 15.5 % | a rational (NURBS) spline carrier; the landed surface decomposition is non-rational-only |
| **admitted** | **726** | **1.2 %** | both sides decompose; all in `spline~spline` |

Check: 28,790 + 21,566 + 9,356 + 726 = 60,438.

## The certify-rate table (measured; the plan floor is context only)

Certify-rate ≥ 80 % is the PLAN's floor, cited as context only — the doc
publishes measured numbers and asserts nothing. `admitted mass` sits beside
the certify-rate (FLOOR anomaly-column discipline). The run completed only
6 of 726 admitted pairs before the certified-trace budget (400 calls) was
spent, so the certify-rate is measured over that completed subset and the
completion fraction is published — never a silently truncated gate.

| subset row | admitted mass | completed | certified_contact | refused:<cause> | unresolved | certify-rate |
|---|---|---|---|---|---|---|
| `spline~spline` | 726 | 6 | 0 | conditioning 2 / non_transverse 3 / singular 1 | 0 | **0.0** |
| `plane~spline` | 0 | — | — | — | — | — (blocked: non-spline carrier) |
| `cylinder~spline` | 0 | — | — | — | — | — (blocked: non-spline carrier) |
| `cone~spline` | 0 | — | — | — | — | — (blocked: non-spline carrier) |
| `spline~torus` | 0 | — | — | — | — | — (blocked: non-spline carrier) |
| **Total** | **726** | **6** | **0** | **6** | **0** | **0.0** |

Completion: 6 / 726 admitted pairs = 0.83 %; unit-pairs enumerated 226,654,
traced 400; wall time 279.2 s. The non-spline rows are not measured by the
composed chain as it stands — that absence, quantified above, is part of the
finding. No threshold is asserted anywhere in-tree.

## Disposition vocabulary + refusal-cause mapping

Every measured pair lands in exactly one of five buckets: `certified_contact` /
`certified_disjoint` / `refused:<cause>` / `unresolved` / `integration_pending`
(the last is the wave-phase-only bucket; the integration aggregate no longer
emits it). The `refused:<cause>` causes are all named, no catch-all. The
composed chain's refusals are mapped in code
(`disposition_of_trace_refusal` / `disposition_of_ssi_refusal`) as:

| bucket | meaning | integration source (mapped from the composed chain) |
|---|---|---|
| `certified_contact` | a certified branch for the pair | `TraceOutcome::ClosedLoop / Terminated / Switched` |
| `certified_disjoint` | a certified no-contact answer | no chain path emits it (0 in the run) |
| `refused:overlap` | FLOOR pair-level cause | reserved (FLOOR vocabulary, carried over) |
| `refused:coincident_circles` | FLOOR pair-level cause | reserved (FLOOR vocabulary, carried over) |
| `refused:unrelated_tangency` | FLOOR pair-level cause | reserved (FLOOR vocabulary, carried over) |
| `refused:unsupported_pair_class` | FLOOR pair-level cause | `SsiRefusal::PairClass(_)` / `InvalidInput` |
| `refused:non_transverse` | trace-level named cause (the plan's name) | `SsiRefusal::Hull(_)`, `SsiRefusal::InclusionNotStrict`, `TraceRefusal::Hull(_)` — the trace could not certify a transverse crossing |
| `refused:conditioning` | trace-level named cause (the plan's name) | `SsiRefusal::Conditioning(_)` / `TraceRefusal::Conditioning(_)` — the frozen F3 rule refused the box |
| `refused:singular` | trace-level named cause (the plan's name) | `SsiRefusal::DeterminantSpansZero` — the reduced Jacobian determinant spans zero |
| `unresolved` | per the landed unresolved causes | `TraceRefusal::Unresolved(GenericUnresolved)` |

## Wave-phase history (one paragraph)

The wave-phase tree (no chain) reported all 60,438 seeds `integration_pending`
with `certify_rate` null — the structural run whose seeds table is above. At
integration the seam was filled, `integration_pending` disappeared from the
aggregate, and the measured table above replaced the placeholder columns. The
structural numbers did not change; the measurement did.

## Reproduction

```console
$env:LOOK_CORPUS = "C:\Users\stefa\look-corpus"
$env:PHASE2_TRACE_BUDGET = "400"
cargo test -p look --test certified_phase2_floor -- --nocapture --test-threads=1
```

The test prints one JSON row per file (`file`, `shells`, `faces`, `seeds`,
`seed_rows`, `admitted_*`, `not_admitted_reasons`, `unit_pairs_total`,
`completed_pairs`, `truncated_pairs`) plus the
`CERTIFIED_PHASE2_FLOOR_AGGREGATE` line and, when the budget was spent, the
`CERTIFIED_PHASE2_FLOOR_BUDGET_EXHAUSTED` line. When `LOOK_CORPUS` is unset
the harness skips cleanly (the four structural tests still pass). The recorded
numbers above came from a **debug (test profile)** run on this machine
(`target/debug`); they are structural counts and certified-disposition counts,
deterministic across build profiles, and a `--release` run reproduces them
(pair counts identical; wall time shorter). Raising `PHASE2_TRACE_BUDGET`
extends the measured completion fraction.

## Corpus provenance and scope

Corpus: the full `LOOK_CORPUS` checkout at `C:\Users\stefa\look-corpus`
(38 STEP files), the same corpus the prevalence census measured. Loader path:
the landed import path the renderer uses (`src/step.rs`): `look::step::part21::parse`
with the `ruststep::parser::parse` fallback, `Table::from_owned_data_section`,
then `Table::to_compressed_shell_with_losses` per shell — identical to the
prevalence census; adjacency is the census's relation, copied, not re-derived.
Patch extraction is the LANDED decomposition (`certified_map::admit_surface`,
the Phase-1 map's row-then-column Bézier cut) read through its public
`patch_grids`; the rational-Bézier unit patches are built through
`RationalBipatch::new` (unit weights for the non-rational pieces). The frozen
seed grid is the domain midpoint plus dyadic offsets ±1/4 in all four
parameters (17 seeds), first certified box wins; no per-pair search beyond the
grid. The FLOOR anomaly pairs are excluded (see above). The single marked
integration seam `run_certified_pair_pair` is the measurement's only
production-call site (`truck_certified::ssi_trace::certified_pair_trace`),
marked in code and doc.
