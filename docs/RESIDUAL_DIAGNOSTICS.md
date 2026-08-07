# Residual diagnostics — mapping the mechanistic dimensionality of the loss tail

**Status: items 1–4 landed and measured 2026-08-07** (truck `f85cc3ff`). Items
5–8 are unbuilt. See §10 for what the first four items measured; the plan text
below is unchanged except where §10 corrects it. No production behaviour
changed: the whole residual reconciles per `source_face_id` across all 20 ABC
models, 12,368 lost either side, identical face sets and identical terminal
reasons.

**Purpose.** Turn the remaining lost faces on NIST and ABC into a small map of
recurring numerical/topological mechanisms, and rank the next recovery packets
by what that map says. The cone investigation is the model: all 416 NIST cone
`AmbiguousLift` faces collapsed to one signature —

```
one bound + rank-1 chart + periodic v
  + specialized conical-band route not applicable
  + falls through to PolyBoundaryPiece::try_new
  + v continuation step pinned at ±0.5 period under bisection
```

— so hundreds of nominally separate surface failures were one continuation
mechanism. This sweep asks how often the rest of the tail does the same thing.

**Not in scope:** recovering faces. Every item here is a witness or a counter.

Line references are into `truck-meshalgo/src/tessellation/triangulation.rs`
unless otherwise qualified; `look` references are repo-relative.

---

## 0. Ground state — check this before measuring anything

The working tree is **mid-flight and not at the handoff's pin**:

- `Cargo.toml` is bumped (uncommitted) from `3a81a169` to **`2a63a5f6`**, and
  `truck-fork` HEAD is `2a63a5f6` = *"Admit half-period singular transitions in
  the periodic lift"* — the cone fix, behind `TRUCK_LIFT_SINGULAR_RECOVERY`,
  **default off**.
- `look` also carries staged `src/step.rs`, `src/step/meshing_policy.rs`,
  `src/step/policy_geometry.rs`, and untracked `ACCURACY_FINDINGS.md` /
  `examples/tolerance_probe.rs`.
- `.cargo/config.toml`'s path override is correctly commented out.

So the handoff's residual table — **13,434 lost of 839,179 declared** across the
20 ABC models — is a `3a81a169` number. Re-baseline at whatever pin the sweep
runs from, or the tail being dimensioned is not the tail that was measured. The
truck bump is low risk (the new route is off); the staged `step.rs` changes are
the ones to discharge.

---

## 1. The witness: extend `FailedFaceDiagnosis`, do not build a framework

The infrastructure is already the right shape. In
`truck-meshalgo/src/tessellation/diagnosis.rs`:

| thing | site | role |
|---|---|---|
| `FailedFaceDiagnosis` | `diagnosis.rs:457` | the per-face record, serialized to JSONL |
| `DiagnosisSink` | `diagnosis.rs:733` | per-face accumulator |
| `FACE_DIAGNOSIS_SINK` | `diagnosis.rs:760` | thread-local (faces run under `par_iter`) |
| `build_face_diagnosis` | `diagnosis.rs:955` | drains the sink at the end of a face |
| `SinkSuspension` | `diagnosis.rs:775` | keeps the record about the *legacy* attempt |
| `record_segment` / `record_realized_edge` / `record_conflict` / `record_boundary_piece` / `record_two_loop_join` | `diagnosis.rs:800`+ | the established "record at the choke point, assemble at the end" pattern |

**The whole sweep is: five new sink fields, five new `record_*` functions, five
new `FailedFaceDiagnosis` fields, called from six sites.** No new telemetry
layer and no new output channel — it rides the existing
`TRUCK_FACE_DIAG_JSONL` JSONL.

New records in `diagnosis.rs`, all `Serialize`, all `Option`/`Vec` so existing
rows stay parseable:

```rust
RouteDecisionRecord { route, eligible, precondition_outcome, refusal_tag, attempted }
LiftTrace           { steps: Vec<LiftStep> /* ≤16, ring */, summary: LiftSummary }
ProjectionWitness   { per-link results, seed results, best, winning route,
                      residual, tol, residual_over_tol, in_domain }
ConstraintPairDetail (extends the existing ConstraintConflictWitness)
CdtStageVector      { boundary_vertices, constraints_presented, constraints_inserted,
                      raw_cdt_triangles, material_selected, final_valid }
```

### The one semantic decision

`SinkSuspension` currently blanks **all** recording while a recovery route
re-tessellates, so the record stays a statement about the legacy boundary — and
the loss bucket the band routes admit on is derived from that record, so this is
a production input, not only a report.

- **Route-selection records must bypass suspension** — the point is to know a
  route was entered.
- **Lift / projection / CDT records must respect it** — otherwise the derived
  bucket describes a mixture of two attempts and the band routes' admission
  rule changes meaning.

Put the route records in a separate, unsuspended sub-sink. Getting this wrong is
a production behaviour change wearing a diagnostic's clothes.

---

## 2. Where each subsystem is captured

### A. Route selection — `cshell_tessellation_inner`, `:718–1520`

Most of this is **already typed and already reaches the census**; it just does
not reach the diagnosis row. `MeshedShellOutcome` carries `band_attempts`,
`cone_band_attempts`, `torus_band_attempts` (`:427`, `:454`, `:480`), each
`Recovered{…} | Refused(exit)` with `exit.tag()`.

Cheapest correct move: **fold the existing per-face attempt enums into
`FailedFaceDiagnosis`** in `face_census.rs` — they are already index-aligned
with `shell.faces` and already read for the ledger. Zero new instrumentation for
the *refused* case.

The gap the cone investigation exposed is the **not-eligible** case.
`run_conical_band_for_face` (`:2508`) returns a bare `None` at three separate
early exits:

1. `cone_of(&face.surface)` refused;
2. `source_face_input_from_compressed` failed;
3. `input.bounds.len() != 2 || input.regular_bound_count() != 2`.

Exit 3 is exactly "one-bound apex cones never enter the two-bound conical-band
route, and therefore fall into the generic lift." Same shape in
`run_cylinder_band_for_face` (`:2355`) and `run_torus_annulus_for_face`
(`:2672`).

→ Either give those functions a typed `RouteIneligible` reason in place of
`None`, or (less invasive) call `record_route_decision(route, reason)` before
each `return None`. Also record the gate states (`TRUCK_FORMAL_RECOVERY` and
`_BAND`, `_HOLES`, `_CYLINDER`, `_TORUS`, `_DECK_JOIN`, `_SEED`, `_PARITY`) and
which arm of the dispatcher chain finally produced the face's `failure`.

### B. Lift / continuation — `PolyBoundaryPiece::try_new`, `:3257–3470`

Highest-value site, and the one where state is genuinely destroyed.
`TRUCK_PROBE_LIFT` already computes exactly the right per-step quantities at
`:3395` — `raw`, `chosen`, `step`, `step/period` — and then `eprintln!`s them
and drops them. The ambiguity/bisection block is `:3420–3462`; `AmbiguousLift`
returns at `:3455`.

→ Keep a bounded ring buffer local to `try_new` holding the last 16 steps:

```
previous_uv, raw_uv, chosen_uv, delta_uv, delta_over_period,
refinements, synthetic, ambiguous_u, ambiguous_v,
half_period_tie_u, half_period_tie_v
```

The `half_period_tie` closure already exists at `:3437`, and
`singular_resolvable` at `:3444` already answers "was singular handling
available?" — capture its value whether or not `TRUCK_LIFT_SINGULAR_RECOVERY`
is set. Emit ring + summary through `record_lift_trace()` **at the
`AmbiguousLift` return, before it returns**. That is the whole lesson of the
cone investigation: after `try_new` fails there is no boundary piece left for
the terminal diagnostic to inspect, and the decisive evidence lived inside the
loop.

Summary statistics — max normalized periodic step, final normalized step,
per-coordinate shrink-under-bisection, pinned-near-0.5 / near-1.0 flags, total
bisection count — are all derivable from the ring plus two running maxima.
Carry `MAX_LIFT_REFINEMENTS`, `AMBIGUOUS_STEP_FRACTION` and
`SINGULAR_HALF_PERIOD_TOL` in the summary so a trace stays interpretable
against a later re-tune.

Record `reconcile_singular_transition` (`:3216`) availability too. It is **not
called from `try_new` at all** — the generic lift path never reaches it.
Recording "available: no" across the whole `AmbiguousLift` population is itself
a finding, and it is one line.

**Trap:** the ring is per-`try_new` call, i.e. **per bound**, not per face. A
multi-bound face produces several; key them by bound index or the summary is a
mixture.

### C. Projection / spline inverse — `by_search_nearest_parameter`, `:129–180`

**Mechanical constraint that shapes the design.** `try_new` is generic over
`PreMeshableSurface`, which has no `search_parameter` and no
`search_nearest_parameter` (`truck-meshalgo/src/tessellation/mod.rs:33–45`).
The surface is reached only through the opaque `sp: impl SP<S>` closure. The
alternate searches therefore **cannot** live at the failure site in `try_new` —
they must live in `by_search_nearest_parameter`, where
`S: RobustMeshableSurface`, and travel to `try_new` through the existing
`PROJECTION_ATTEMPT` thread-local (`:110`, read via `last_projection_attempt()`
at `:120`, consumed at `:3346`). That plumbing is already built.

So: extend `ProjectionAttempt` in place, and extend the **instrumented arm** of
the chain (`:143–180`), which is already forked from the production
one-expression arm precisely so the two cannot drift.

Fields to add:

- production result per link, individually — 1 hinted `search_parameter`,
  2 hintless, 3 hinted `search_nearest_parameter`, 4 hintless. Links are
  already tracked; their *results* are not.
- previous-UV hint (already the `hint` parameter).
- **nearest inverse from each structural seed, asked with
  `search_nearest_parameter`.** This is PROJ-001's unanswered column.
  `by_structural_seeds` (`:202`) scores seeds with `search_parameter`, which is
  all-or-nothing and returns `None` unless the point is already within
  tolerance — so a failing point yields no residual at all, and the probe arm
  at `:158` recomputes it the same wrong way. Re-ask with
  `search_nearest_parameter(point, seed, 100)` **in the probe arm only**. Do not
  read the currently-empty column as "residuals are large."
- winning seed index and winning route; world-space residual
  `surface.subs(u, v).distance(point)`; caller `tol`; `residual / tol`;
  and `try_range_tuple()` containment for parameter-domain validity.

`by_structural_seeds` itself and the production `.or_else` chain must not
change. The existing two-arm split is what enforces "do not change production
acceptance behaviour" structurally rather than by review.

Widening the seed source past one-per-knot-span is **out of scope for this
sweep** — it is a `SearchParameter::search_parameter_seeds` change. Note for
whoever picks it up that `Processor` does not override the defaulted-empty
method, which is why Processor-wrapped analytic families are offered zero seeds.

`try_new`'s `proj_probe` block (`:3336–3355`, `:3470–3502`) already declines to
stop the boundary walk at the first failure and already accumulates the
`failed_points / boundary_points` ratio and the per-link histogram. Keep that
behaviour; redirect the `eprintln!` into `record_projection_witness()`.

Target discrimination:

```
production/search-contract miss   |  seed-basin coverage failure
nearest solution genuinely too far |  nearest inverse fails everywhere
```

### D. Arrangement / constraint insertion — `PolyBoundary::insert_to`, `:4895–5220`

Best-instrumented site already. `record_conflict` (`:5140`) names both segments
via `diag_edge_map`, and `SemanticSegmentRef` carries origin and boundary
component. Against the wanted list, four gaps:

1. **`source_bound` / `source_edge_use` are declared but always `None`**
   (`diagnosis.rs:800`). The provenance exists in
   `source_face_input_from_compressed` (`:1591`), but `create_boundary`
   (`:1008`) flattens edge-uses away before `insert_to` sees them. Threading it
   is the single largest piece of new work in the sweep and the only item that
   touches the *boundary construction* path rather than only failure paths —
   scope it deliberately, or accept `boundary_component` + `segment_index` as
   round-one provenance.
2. **`relation` is hardcoded `ProperInteriorCrossing`** at `:5147`.
   `EndpointOnInterior` versus a proper crossing is decidable cheaply from the
   conflicting edge's endpoints, and `CollinearOverlap` / `DuplicateTraversal`
   are already separately detectable — the `overlapping` test at `:5015` and
   `roles.traversals` (`:5090`, WAVE-4A's counter) both exist.
3. **`intersection_enclosure` is always `None`.** Both segments' UV endpoints
   are in hand at the failure site. Record the four endpoints unconditionally
   (cheap). The exact "do the sampled chords cross while the underlying
   analytic curves do not" split — the whole plane-CII question, and WAVE-3B's
   chord-artefact finding — has predicates in `formal/intersection.rs` and
   `formal/exact.rs`; make that classification opt-in.
4. **`ConstraintOverlapUnsupported` records no witness at all.** `:5019`
   `continue`s before any `record_conflict`, so 977 faces carry zero pair
   evidence. One `record_conflict` call with a `CollinearOverlap` /
   `DuplicateTraversal` relation fixes it. This is the cheapest unexamined thing
   on the whole list.

Capture the pair while it still exists. The terminal enum has lost it.

### E. CDT / material pipeline — `:5585` and `:5921`

Every number in the wanted stage vector is computed and then discarded:

| stage | site | today |
|---|---|---|
| boundary vertices | `insert_to` `poly2tri`, `:4947` | local |
| constraints presented | `insert_to` k-loop, `:4985` | local |
| constraints inserted | `chain.is_empty()`, `:5058` | local |
| raw CDT triangles | `triangulation.inner_faces()`, `:6039` | never counted |
| material-selected | parity filter, `:6041` | filtered inline |
| final valid | degenerate / zero-area filter, `:6045–6058` | `tri_faces_raw` |

`NoOddParityRegion` is raised at `:6058` when `tri_faces_raw` is empty, which
**conflates "parity selected nothing" with "parity selected only degenerate
triangles."** Splitting the count at the filter separates them and re-buckets
the 342 on its own.

→ Count the six, thread them out of `insert_to` (which already returns
`Result<(), reason>`), and `record_cdt_stages()` from
`trimming_tessellation_with_diagnostics` (`:5585`) before the outcome returns.
This is arithmetic, not tracing. Deep CDT tracing stays unbuilt unless the first
sweep says otherwise.

Separates: bad/degenerate input · constraint insertion failure · empty CDT ·
material classification removes everything · final validation removes
everything.

---

## 3. Emission and serialization

Terminal site is unchanged: `build_face_diagnosis` (`diagnosis.rs:955`), called
from `:1505` inside the `if diag` block. Add the new fields to the drain.

**`PROBE_FACE_CONTEXT` is reset at `:1493`, before the diagnosis is built** — new
records must already be in the sink by then, never read from the context there.

Runner side, `look/examples/face_census.rs`:

- `FaceDiagRow` (`:20`) `#[serde(flatten)]`s the diagnosis, so new fields appear
  in the JSONL automatically.
- `:525–563` fills `model_id` / `surface_family` and hand-rolls a fallback
  record for faces lost before tessellation — that literal needs the new fields
  (all `None` / empty).
- Fold `band_attempts` / `cone_band_attempts` / `torus_band_attempts` in here;
  they are already read at `:471–520` for the ledger.
- The JSONL write at `:1016` and the `.meta.json` sentinel are unchanged.

---

## 4. Gating and cost

Two tiers, matching house style:

- `TRUCK_FACE_DIAG_JSONL` continues to enable the base record, and is implied by
  the default-on band routes through `diag_enabled()` (`diagnosis.rs:725`).
- **`TRUCK_FACE_DIAG_DEEP=1`, default off**, gates the four expensive
  additions: the lift ring, the alternate-inverse searches, analytic crossing
  classification, and the CDT counters. Read once into a `OnceLock` —
  `projection_probe_enabled()` (`:126`) already documents why: this sits on the
  per-boundary-point path, and an `env::var_os` per call is a syscall per point
  per model.

Production acceptance is untouched by construction: every new call site is
either on a path that has already decided to fail, or is a counter.

**Cost to measure before the ABC sweep.** The alternate inverse searches run per
*failing boundary point*, not per face, and each is a 100-iteration Newton from
every knot-span seed. On `00000414` (1,175 NURBS projection faces) that could
dominate. Cap seeds per point and points per face, and record that the cap was
hit.

**Regression gate.** Reconcile per `source_face_id` against the previous pin on
all 20 models, **including triangle counts** (`p1-out/parityA/tab.py`). Require
zero movement in both directions.

---

## 5. Sweep plan

Clean build, one config, no concurrency — WAVE-4 ran two corpus sweeps at once
and the contention turned `00005641`'s panic into a silent zero-byte census.

```
cargo +stable-x86_64-pc-windows-gnullvm build --release \
      --target x86_64-pc-windows-gnullvm --example face_census
```

- **NIST** — `look-corpus/nist-census/` already holds `nist_files.txt`,
  `nist_diag.jsonl`, `nist_per_model.csv`. Re-run the same list into
  `p1-out/diag-w5-nist/`.
- **ABC** — `p1-out/diag-w4-sweep.sh` is the template; it iterates
  `look-corpus/abc/*/`, the **20-model** set. `look-corpus/abc_extracted/` holds
  **7,168** models. Decide which "ABC losses" is meant. Recommendation: the
  20-model set for the deep witness — it is the set every prior number is keyed
  to, and it is where the deep probes are affordable — and the 7,168-model set
  with base-record-only for the population-share denominators.

Check the `.meta.json` sentinel on every model: a zero-byte census is a panic,
not an empty result. Check free disk before timing anything; it sat at 11 GB
through WAVE-4.

---

## 6. Analysis

Extend `look/examples/face_diag_aggregate.rs`. It already produces
`bucket × rank`, `bucket × surface`, `bucket × periodic-axis` and a
reconciliation, and it reads the JSONL as untyped `serde_json::Value`, so new
fields need no schema change there.

Add a **signature miner**: project each row onto a tuple of discriminative
fields, count exact combinations, sort descending, and emit the
cumulative-coverage curve — the 50 / 75 / 90 / 95 % answer is that curve's
inverse. Suggested projection:

```
(terminal_reason, surface_family, bound_count_bucket, chart_rank, periodic_axes,
 selected_route, lift_pin_class, projection_verdict, conflict_pair_class,
 cdt_stage_class)
```

where the last four are small derived enums, not raw numbers:

- `lift_pin_class ∈ {pinned_half_period, pinned_full_period, shrinking, no_trace}`
- `projection_verdict ∈ {production_miss, seed_basin_gap, nearest_too_far, no_inverse_anywhere}`
- `conflict_pair_class ∈ {source_source_same_bound, source_source_inter_bound, source_synthetic, synthetic_synthetic, overlap, vertex_insertion}`
- `cdt_stage_class ∈ {degenerate_input, insertion_failed, empty_cdt, material_empty, validation_empty}`

Raw fields stay in the row; the miner buckets them. That is what turns 416 cone
faces into one line.

Signatures to look for first:

```
AmbiguousLift + one bound + rank 1 + periodic v + generic lift
              + v-step pinned at ±0.5 period

BoundaryProjectionFailed + NURBS + production inverse None
              + structural seed succeeds + residual < tolerance

ConstraintInsertionFailed + same-bound + source/source
              + sampled segments cross + underlying analytic curves do not
```

Per important signature report: NIST count, ABC count, affected surface
families, shared code path, likely mechanism, confidence, smallest plausible
recovery intervention, rough engineering risk. **Cross-tab NIST against ABC
explicitly** — a signature present in one corpus only is a corpus artefact until
shown otherwise.

Use additional targeted probes only if a large population is still ambiguous
after this first sweep.

---

## 7. Traps specific to this work

1. **A refusal at stage N says nothing about stage N+1.** WAVE-4B moved ~1,400
   analytic faces from projection into constraint insertion, and CII and Overlap
   *grew*. Every signature count is a ceiling on recovery, not an estimate of it.
2. **`SinkSuspension` semantics** — see §1. Wrong split changes what the band
   routes admit.
3. **The lift ring is per bound, not per face.** Key by bound index.
4. **`declared_face_index` collides across shells.** Join on `source_face_id`.
5. **Probe stderr interleaves** under `par_iter`. Everything new goes into the
   sink and out through the sorted JSONL, never through `eprintln!`.
6. **A fresh exe timestamp is not a fresh exe.** Verify by behaviour.
7. Re-run `p1-out/yield_tab.py` if anything planar moves.

---

## 8. Sequencing

| # | piece | size | why here |
|---|---|---|---|
| 1 | Re-baseline at the current pin; confirm staged `step.rs` moves nothing | small | everything downstream is keyed to it |
| 2 | Route-decision record — fold existing attempts, type the `None` exits | small | pure win, the cone lesson, no new math |
| 3 | CDT stage vector | small | six counters; splits `NoOddParityRegion` immediately |
| 4 | `ConstraintOverlapUnsupported` witness + UV endpoints on every conflict | small | 977 faces currently carry zero pair evidence |
| 5 | Lift trace ring + summary | medium | the 1,239 `AmbiguousLift`, and generalizing the cone signature |
| 6 | Projection witness with the `search_nearest_parameter` re-ask | medium | answers PROJ-001's unanswerable column |
| 7 | Sweeps — NIST, ABC-20 deep, ABC-7168 base | long | serial, unattended |
| 8 | Signature miner, coverage curve, report | medium | the deliverable |

Items 2–4 are independently landable and each answers something on its own.
5 and 6 are where the tail's dimensionality actually gets measured.

---

## 9. Open decisions

1. **Which ABC set** the deep witness runs over — the 20-model set every prior
   number is keyed to, or all 7,168 extracted models.
2. **Whether to thread real `source_bound` / `source_edge_use` provenance into
   `SemanticSegmentRef`** in this sweep or defer it. It is the one item that
   touches the boundary construction path rather than only failure paths.

---

## 10. What items 1–4 measured (2026-08-07)

Pin: truck `f85cc3ff`, `.cargo/config.toml` override re-commented, resolved rev
confirmed by `cargo tree` and by behaviour. Sweeps in `look-corpus/p1-out/`:
`diag-w5-nopolicy`, `diag-w5-policy`, `diag-w5-base`, `diag-w5-instr`.
Aggregation: `p1-out/w5_tab.py`.

### Item 1 — the baseline moved twice, and §0 was wrong about the second

| config | lost of 839,179 |
|---|---:|
| truck `3a81a169`, no meshing policy — the handoff's number | 13,434 |
| truck `95d0df30`, no policy | 13,434 |
| truck `95d0df30` + meshing policy | 13,018 |
| truck `9bae285a` + meshing policy — **the baseline** | **12,368** |

- **The `95d0df30` bump is exactly inert**, as §0 expected — and more strongly
  than a total: the per-face JSONL is byte-identical on 19 of 20 models, the
  20th differing only in a `model_id` path string.
- **The staged `step.rs` meshing policy is *not* inert.** §0's "confirm staged
  `step.rs` moves nothing" is falsified. It moves 416 net and moves them **both
  ways** — −557 over seven models, +141 over six; `00005760` −259 and
  `00001075` −126 against `00007667` +60 and `00000730` +50. This is the
  bidirectional density sensitivity `ACCURACY_FINDINGS.md` §7 predicted. **The
  +141 is uncharacterised and is the one open item from this step.**
- `9bae285a` (singular-lift recovery default-on) recovers 650 with zero
  regressions on any model.
- Baseline chosen: **policy on**, because it is what `parse_step` ships.

**Trap learned.** `truck-fork` HEAD advanced mid-session while `Cargo.toml`'s
pin stood still, and building through the `paths` override silently picked the
newer commit up — so a diagnostics-only change appeared to recover 39 faces.
The override decouples the build from the pin in *both* directions, not only by
hiding unpushed work. If a change that cannot move geometry moves it, check
`git -C ../truck-fork log -1` before believing the diff.

### Item 3 — `NoOddParityRegion` is almost entirely the *other* thing

The name says "parity selected no region". Split at the validation filter:

| class | faces |
|---|---:|
| `validation_empty` — parity selected a region, every triangle then removed as degenerate or zero-area | **395** |
| `empty_cdt` | 6 |
| `material_empty` — parity genuinely selected nothing | 1 |

**98% of the bucket is degenerate output, not absent material.** Any recovery
aimed at material selection would have addressed one face.

Whole-residual stage classes (12,368):

| class | faces |
|---|---:|
| `insertion_failed` | 7,967 |
| `never_reached_insertion` | 3,976 |
| CDT/material stages | 402 |
| `cdt_not_reached` (parity contradiction / role missing) | 23 |

`never_reached_insertion` is exactly `BoundaryProjectionFailed` 3,703 +
`AmbiguousLift` 145 + `BoundaryConstructionFailed` 127 + 1 — the classes are
disjoint and account for the residual with no remainder. Across all faces,
1,283,165 constraints were presented and 62,468 (4.87%) never realized.

### Item 4 — the 1,031 unwitnessed faces, witnessed

Every `ConstraintOverlapUnsupported` face now carries pair evidence:
**1,031 of 1,031, with zero unattributed blocking edges.** 1,883 faces carry an
overlap witness once faces whose terminal reason is something else are counted.

Witness pairs, by origin and bound:

| incoming/blocking | same bound | witnesses |
|---|---|---:|
| Seam / Seam | yes | 23,318 |
| Source / Source | yes | 12,793 |
| SyntheticClosure / Source | no | 800 |
| Source / Source | no | 198 |
| Seam / Source | yes | 46 |
| Source / SyntheticClosure | yes | 13 |

By face: 793 source/source only, 130 seam/seam only, 108 mixed. The relation is
`DuplicateTraversal` and it is exact rather than inferred — the test matches the
*direct* edge, so the presented segment's endpoints coincide with an existing
constraint edge. **A boundary is traversing its own edges twice**, and the first
record examined presents 32 constraints and realizes exactly 16.

### Item 2 — the band routes are no longer eligible for anything

| route | outcome | faces |
|---|---|---:|
| CylinderBand | PreconditionUnmet | 12,237 |
| ConeBand | PreconditionUnmet | 12,237 |
| TorusAnnulus | Ineligible: `SurfaceNotCertified` | 11,264 |
| TorusAnnulus | Refused: `torus_not_eligible` | 977 |
| WindingParity | Refused | 22 |
| CylinderBand | Ineligible | 4 |
| ConeBand | Ineligible | 4 |

**The cylinder and cone bands fail their precondition on 12,237 of 12,368
faces** — only eight faces in the entire residual even reach a band route
function. Their admitted bucket, `SyntheticSyntheticCrossing`, has been emptied
by the earlier waves. Widening either cell buys nothing until its admission
rule changes; that is a ceiling, per §7.1, not an estimate.

127 faces carry no route record at all: they fail at boundary construction,
before the dispatcher.

### Corrections to the plan text above

- §8 item 1's premise ("confirm staged `step.rs` moves nothing") is wrong; see
  above.
- §2D item 4 estimates 977 faces with zero pair evidence; at this baseline it is
  1,031, and all are now witnessed.
- §2E's `NoOddParityRegion` count of 342 is 402 at this baseline.
- The `WindingParity` route records only when it runs, unlike the three band
  routes which record every dispatcher arm. Absence means not run.

---

## 11. What item 6 (PROJ-002) measured (2026-08-07)

Pin: truck `f85cc3ff` + uncommitted PROJ-002 instrumentation
(`diagnosis.rs`, `triangulation.rs`), `.cargo/config.toml` override live
against the fork. Production acceptance unchanged: 839,179 declared, 12,368
lost across the 20 ABC models — identical to the §10 baseline. Sweep:
`p1-out/proj2-sweep.sh` over `look-corpus/abc/*/` (the 20-model set),
`TRUCK_PROBE_PROJ_DEEP=1`, `TRUCK_FACE_DIAG_JSONL`. Aggregation:
`p1-out/proj2_tab.py`.

### Witness coverage

**3,703 of 3,703 `BoundaryProjectionFailed` faces carry a deep witness; 0
without.** The corrected gate (§2C: `projection_probe_enabled()` ORs in the
deep gate; the walk site reads the shared helper, not the raw variable)
produces a witness for every probed face. The first sweep was invalid for the
reason the handoff records — `TRUCK_PROBE_PROJ_DEEP` armed the producer but
the walk still checked raw `TRUCK_PROBE_PROJ`, so witnesses were silently
empty — and is not repeated here.

### The recoverable ceiling: 91.7%

| best world residual | faces | share |
|---|---:|---:|
| ≤ 1 × tol | 3,396 | 91.7% |
| ≤ 10 × tol | 3,460 | 93.4% |
| ≤ 100 × tol | 3,585 | 96.8% |
| > 100 × tol | 118 | 3.2% |

For 91.7% of these failing faces the surface genuinely passes within tolerance
of the boundary point. The face is lost to a search defect, not a geometric
one. Only **30 faces (0.8%) are `NearestTooFar`** — a converged stationary
point whose residual exceeds `tol`, a geometric statement that the boundary
does not lie on the surface. The worst-point view (a face is lost by its worst
point) is only slightly lower: 89.1% have every probed point within 1× tol.

This is PROJ-001's unanswered column, answered. Production's
`search_nearest_parameter` is `newton::solve(..).ok()`, so its `None` means
**Newton did not converge**, not that the nearest point is far. The deep probe
keeps the best iterate: 91.7% of those `None`s have a within-tolerance answer
that was thrown away.

### The dominant mechanism: ProductionMiss, not seeding

| verdict | faces | share |
|---|---:|---:|
| ProductionMiss | 2,393 | 64.6% |
| DomainOrContractIssue | 789 | 21.3% |
| Inconclusive | 278 | 7.5% |
| SeedBasinGap | 213 | 5.8% |
| NearestTooFar | 30 | 0.8% |

`ProductionMiss` — a start production already uses reached `residual ≤ tol`,
in-domain, and Newton's `near2` convergence test rejected it — is 64.6% of the
population. The winning route is a **production start on 89.2%** of faces
(3,304); only 10.8% (399) are won by a structural seed. 3,423 faces (92.4%)
had a trial-exhausted Newton search. **New seeds would fix only the 5.8%
`SeedBasinGap` class; the dominant population needs the convergence gate
changed, not the seed list.**

`DomainOrContractIssue` (21.3%) is the second class: a within-tol solution
exists but lies outside the declared parameter range — a domain/contract
question, partially recoverable, and concentrated on BSpline (below).

### BSpline vs NURBS

| family | total | ProdMiss | Domain | SeedGap | Inconc | TooFar |
|---|---:|---:|---:|---:|---:|---:|
| Nurbs | 2,472 | 1,800 (72.8%) | 437 | 124 | 109 | 2 |
| Bspline | 1,180 | 586 (49.7%) | 346 (29.3%) | 89 | 134 | 25 |
| Offset | 27 | — | 6 | — | 21 | — |
| Extruded | 12 | 2 | — | — | 10 | — |

NURBS is the dominant family and is overwhelmingly a convergence-gate defect
(72.8% ProductionMiss). BSpline is more mixed: half ProductionMiss, but
`DomainOrContractIssue` is 29.3% — the out-of-domain mechanism is a real
secondary story on BSpline, not on NURBS. The 51 faces offered zero seeds are
the `Processor`-defaulted-empty families (Offset/Extruded/Torus/Revolved);
they cannot be `SeedBasinGap`, and they are not — confirming §2C's note.

### Caps and pathology

- per-face point cap (8 points): 585 faces hit it.
- per-point seed cap (24 seeds): 801 faces hit it.
- degenerate-Jacobian stops: 526 faces.
- trial-exhausted Newton searches: 3,423 faces (92.4%).

The caps do not distort the verdict: a face that hit the point cap still has
its probed points classified, and the cap is on points, not on the face
verdict.

### Per model

`00000414` alone is 1,296 faces (35% of the population, 1,264 ProductionMiss)
— the NURBS-heavy model. `00009190` is the exception: 182 faces,
`Inconclusive`-dominant (88), a different mechanism profile that warrants a
targeted look. `00003172` is the `DomainOrContractIssue` outlier (280 of 398).

### The meshing-policy newly-lost cohort is orthogonal to projection

The 520 faces the meshing policy newly loses (policy-on lost, policy-off
rendered; the on/off pair was taken at `95d0df30`, and all 520 are still lost
at the `f85cc3ff` baseline) are **zero `BoundaryProjectionFailed`**:

| terminal reason | faces |
|---|---:|
| ConstraintInsertionIncomplete | 460 |
| NoOddParityRegion | 60 |

The policy densifies circular edges (the 24-segment angular floor); the damage
is in the CDT constraint-insertion chain, not in projection. The projection
witness population (3,703 BPF) and the policy-newly-lost cohort (520 CDT) are
disjoint. This confirms `ACCURACY_FINDINGS.md` §7 / `policy_geometry.rs`: the
density sensitivity is a triangulator effect, and it lands in the
`insertion_failed` class that ARR-TAIL-001 will decompose — not here.

### What this does not establish

The 91.7% ceiling is a ceiling, not an estimate (§7.1): admitting the
within-tol iterate on one point does not make the face survive, because a face
is lost by its worst point and the boundary walk may still fail elsewhere.
`ProductionMiss` says the iterate is good; it does not say the face is
recoverable in isolation. The 278 `Inconclusive` (cap / non-finite numerics)
are not classified and could move the ceiling either way; `00009190`'s
Inconclusive-dominant profile is the largest unexamined block within them.

Recovery is not implemented (per scope). The two candidate levers this
characterization isolates: (a) admit the within-tolerance iterate that
Newton's `near2` test discards — the `ProductionMiss` class; (b) relax or
repair the parameter-domain contract on BSpline — the `DomainOrContractIssue`
class. Both are ceiling-only until a recovery experiment measures them.
