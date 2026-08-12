# Handoff — RECOVERY-WAVE-2 (PROJ-003B + PROJ-003C + ARR-TAIL-001)

**State.** look on `integration/formal-atlas-wave-2`, pinned to truck
`67c19555` on `feature/cone-apex-lift-recovery` (Wave-2 committed and pushed;
Stage A was `bcf676ee`). The `.cargo/config.toml` path override is
RE-COMMENTED; the pinned build was verified via `cargo check --locked`.

**Stage A + B + C are all landed, default-on, measured.** Stage B (structural-
seed residual recovery) and Stage C (domain/periodicity recovery) were built in
this wave together with the ARR-TAIL mechanistic census. Headline ABC-19 (all
of ABC-20 except `00007744`, which stalls the deep probe — see Known issue):

```
ABC-19: 827,149 declared, 817,838 rendered (98.87%), 9,311 lost
vs Stage-A-off base: +2,549 rendered, 0 regressions, 0 triangle churn
vs Stage-A-only:     +570 rendered (B+C), 0 regressions, 0 triangle churn
Stage B admitted:  234 faces / 294 points
Stage C admitted:  452 faces / 839 points (all BoundaryEpsilon)
B/C admitted -> rendered 570, still lost 103
NIST (33): 7,902 declared, 7,717 rendered; A+B+C +36 vs no-policy, B+C +0;
  0 regressions, 0 churn
```

The PROJ-002 `213 SeedBasinGap` / `789 DomainOrContractIssue` ceilings were NOT
forced: B recovered 234 faces and C 452, all under the existing caller
tolerance (residual max ≈ 7e-6, none > 0.9·tol), with no clamping of arbitrary
UVs.

---

## What this wave did

- **Stage B** (`TRUCK_FORMAL_RECOVERY_PROJ_STAGE_B`, nested under the master
  gate): promotes PROJ-002's diagnostic structural-seed nearest search into a
  residual-certified recovery. On a point the whole legacy chain (including the
  spline-seed `search_parameter` retry) and Stage A refused, the bounded
  seed-nearest iterate (`seed_best`) is admitted under the identical Stage A
  contract. `probe_structural_seeds` is the shared helper; the deep probe and
  Stage B read the same candidate.
- **Stage C** (`TRUCK_FORMAL_RECOVERY_PROJ_STAGE_C`): classifies
  `DomainOrContractIssue` candidates by mechanism and recovers only the two
  principled subclasses. Every admitted C point on the corpus was
  `BoundaryEpsilon` (microscopically outside a closed knot-range boundary,
  clamped and re-certified `|S|-P| <= tol`). `PeriodicEquivalent` (normalize by
  a certified generator) is implemented and unit-tested but found no corpus
  faces (splines have no certified generator). `RepresentationRangeMismatch`
  and `TrueOutOfDomain` are diagnostic-only; no tolerance widening anywhere.
- **ARR-TAIL-001**: `FailedFaceDiagnosis.arr` now carries a mechanistic
  signature (failure_stage / material_stage / bound bucket / seam-deck / pair
  relation / pair provenance / projection verdict); `ProjectionWitness` gains
  per-point `point_evidence`. Diagnostic-only; with the gates closed geometry
  is unchanged.

---

## ARR-TAIL dominant mechanisms (ABC-19 residual tail, 9,311 faces)

54 distinct signatures; concentration **50% = 3 sigs, 75% = 8, 90% = 13,
95% = 19**. Top 5:

```
1,921  CII  ConstraintIncomplete  b2  no-seam  ProperInteriorCrossing  SourceSource
1,620  CII  ConstraintIncomplete  b1  no-seam  ProperInteriorCrossing  SourceSource
1,408  CII  ConstraintIncomplete  b2  Seam     ProperInteriorCrossing  SourceSynthetic
  773  COU  ConstraintRejected    b1  no-seam  DuplicateTraversal      SourceSource
  420  CII  ConstraintIncomplete  b3  Seam     ProperInteriorCrossing  SourceSource
```

Marginals: terminal = ConstraintInsertionIncomplete 7,049 / Overlap 1,028 /
BoundaryProjectionFailed 547 / NoOddParityRegion 407 / AmbiguousLift 145 /
BoundaryConstructionFailed 116 / ContradictoryDualParity 18. Material stage:
94.4% NotReached (failure is upstream of material), 4.3% MaterialDegenerated
(the ~401 post-material-degeneracy `NoOddParityRegion`), 0.06% MaterialEmpty.
Seam/deck: 33.2% involve a seam. Pair relation: 70.0% ProperInteriorCrossing,
16.7% DuplicateTraversal. Pair provenance: 61.4% SourceSource, 22.9%
SourceSynthetic, 13.3% none. Same-bound: 70.5% within one bound.

Known populations segregate cleanly (no rediscovery as "unknown"):
~7,049 ConstraintInsertionIncomplete, ~1,022 duplicate-traversal overlap,
~547 unresolved projection, ~401 post-material degeneracy, ~145 residual
singular-lift.

**Recommended next implementation targets:**
1. **Constraint-insertion incompleteness** (~7,049 faces; 3 signatures already
   explain 53%, all `ConstraintInsertionIncomplete` + `ProperInteriorCrossing`
   + `SourceSource`, b1/b2 with and without seam). The dominant single
   signature is a b2 SourceSource ProperInteriorCrossing refusal.
2. **Duplicate-traversal overlap** (~1,022 `ConstraintOverlapUnsupported`
   `DuplicateTraversal`; already-witnessed at the `overlapping` test in
   `insert_to`). Needs topology/provenance semantics, not a blanket normalize.
3. **Post-material degeneracy** (~401 `NoOddParityRegion` with
   `MaterialDegenerated`; `CdtStageVector.material_selected > final_valid`)
   and the residual singular-lift (~145 `AmbiguousLift`) are smaller.

---

## Known issue: deep-probe stall on 00007744

With `TRUCK_PROBE_PROJ_DEEP` set, `00007744` stalls: the per-point deep probe
(≤8 points × ≤24 seeds × 100 Newton trials) runs without a per-face budget, and
one face with a large number of failing boundary points takes unbounded time.
The FACE ledger line only writes when the face finishes, so the model appears
frozen. `00007744` is therefore EXCLUDED from this wave's ABC-20 numbers (19/20
models). **Next session: add a per-face probe budget (e.g. cap total probe
searches per face) or a time/point cap, then re-run `00007744`.** The stall is
diagnosis-only — it does not affect rendering with the deep probe off.



---

## What PROJ-003A did (recap, 4 sentences)

The legacy chain treats `search_nearest_parameter == newton::solve(..).ok()`,
so a `None` means only that Newton's `near2` gate was not met. PROJ-003A admits
a finite, in-domain best iterate from a start production already uses (the
caller hint and the hintless presearch start) when it re-evaluates within the
caller's tolerance — without requiring `near2`. It fires only where the legacy
chain returned `None`, so it is refinement-only. Result: ABC-20 lost
12,368 → 10,251 (98.526% → 98.778%), +2,117 faces rendered, **0 regressions, 0
triangle-count changes**; NIST lost 221 → 185, +36, 0 regressions.

---

## Concise code-navigation map for the next packet

All truck paths are under `truck-fork/`. Line numbers are current.

### Production boundary-point projection / nearest-parameter search
- `truck-meshalgo/src/tessellation/triangulation.rs`
  - `by_search_nearest_parameter(...)` ~:424 — the SP closure the tessellator
    passes for every boundary point. Production arm is the exact legacy chain
    (`search_parameter` x2 → `search_nearest_parameter` x2 → `by_structural_seeds`).
  - `truck-geotrait/src/algo/surface.rs` `search_nearest_parameter` ~:205 —
    the shared free function = original `newton::solve(..).ok()`; called by all
    surface trait impls (bspsurface/nurbssurface/decorators).

### The current PROJ-003A recovery path
- `truck-meshalgo/src/tessellation/triangulation.rs`
  - `residual_certified_recovery` ~:414 and `residual_certified_admission` ~:379 —
    the Stage A gate + contract (finite UV, `try_range_tuple` in-domain,
    re-evaluated `|S(u,v)-P| <= tol`). Runs at the `(None, false)` arm of the
    boundary-point match inside `PolyBoundaryPiece::try_new` ~:3720.
  - `probe_nearest` ~:274 — maps `search_nearest_parameter_outcome` onto
    `NearestOutcome` (adds in-domain).
  - `truck-geotrait/src/algo/surface.rs` `search_nearest_parameter_outcome`
    ~:139 — converged answer comes from the real legacy `search_nearest_parameter`;
    best-iterate tracking is a separate identical pass. **Do not** reintroduce a
    second Newton loop here.

### Structural-seed generation and projection (Stage B)
- `truck-meshalgo/src/tessellation/triangulation.rs`
  - `by_structural_seeds` ~:557 — production link 5; runs `search_parameter`
    from each knot-span seed, keeps best by residual then by drift from hint.
    Gate: `spline_seed_recovery_enabled()`.
  - `truck-geotrait/src/traits/search_parameter.rs` `search_parameter_seeds`
    ~:130 — trait default (empty). Real impls: `truck-geometry/src/nurbs/bspsurface.rs`
    and `nurbssurface.rs` (one seed per knot span).
- The nearest-search-from-seeds probe (the thing Stage B will promote into
  production) currently runs only under the deep probe; see `attempt.seed_best`
  below.

### Where the best structural-seed iterate/result is available or should be retained
- `truck-meshalgo/src/tessellation/triangulation.rs`, struct `ProjectionAttempt`
  ~:87 — `seed_best: NearestOutcome` and `seed_best_index` are filled by the
  deep-probe block in `by_search_nearest_parameter` (~:520), i.e. diagnostic-only
  today. Stage B = mirror Stage A: promote the seed nearest-search into
  production (bounded seeds, early exit on admissible, residual/domain
  certification) and read it at the same `(None,false)` arm in `try_new`.

### Surface parameter ranges / domain checking
- `truck-geotrait/src/traits/surface.rs` `try_range_tuple` ~:40 — returns
  `(Option<range_u>, Option<range_v>)`; `None` axis = unbounded.
- Used by `residual_certified_admission` and by `probe_nearest`'s in-domain.

### Surface periodicity, periods, UV wrapping/normalization (Stage C)
- `truck-meshalgo/src/tessellation/domain/lattice.rs` — `CertifiedLattice`;
  `declared_u_period()`/`declared_v_period()` (accessor evidence) vs
  `generator()` (representation-derived, the only deck-valid period).
- `truck-meshalgo/src/tessellation/triangulation.rs`
  - `get_mindiff` ~:4315 — nearest-period-copy UV wrapping used in the walk.
  - `AMBIGUOUS_STEP_FRACTION` ~:4368 and the periodic lift / bisection inside
    `PolyBoundaryPiece::try_new` (the `ambiguous(...)` half-period machinery) —
    the existing continuous-lift machinery Stage C must not bypass.
  - Stage C belongs after Stage A admission: normalize out-of-range periodic
    candidates by integer periods (only on certified periodic axes), never clamp
    arbitrary UVs. See `lattice.declared_u_period()` reads ~:3741, :4080.

### Where projection failures become `BoundaryProjectionFailed`
- `truck-meshalgo/src/tessellation/triangulation.rs` `PolyBoundaryPiece::try_new`
  — the `(None, false)` arms return
  `TessellationFailureReason::BoundaryProjectionFailed` (the else arm and the
  `proj_probe && failed_points > 0` block ~:4054).

### Projection diagnostic state/types
- `truck-meshalgo/src/tessellation/triangulation.rs` — `ProjectionAttempt`
  (~:87), `NearestOutcome` (~:131), thread-local `PROJECTION_ATTEMPT`,
  `last_projection_attempt()` (~:197), `classify_projection_point` (~:340),
  `better_outcome` (~:333), `probe_nearest`.
- `truck-meshalgo/src/tessellation/diagnosis.rs` — `ProjectionWitness`,
  `PointVerdict`, `NearestRoute`, `derive_face_verdict`; sink field
  `projection_witness`; `record_projection_witness`.

### Constraint insertion and `ConstraintInsertionIncomplete`
- `truck-meshalgo/src/tessellation/triangulation.rs` `insert_to` ~:5457 — the
  per-segment CDT constraint insertion; raises
  `ConstraintInsertionIncomplete` at ~:5545 and ~:5770 (endpoint/insertion
  refusals) and `ConstraintOverlapUnsupported` at ~:5622 (overlap refusal).
- `SegmentOrigin` ~:4455, `BoundaryPath` ~:4586, `PartJoin` ~:4570,
  `BoundaryLoop` ~:4487 — the labelled boundary path that feeds insertion.

### Constraint overlap / duplicate-traversal diagnostics
- `truck-meshalgo/src/tessellation/diagnosis.rs` — `ConstraintConflictWitness`,
  `PresentedSegmentRelation` (ProperInteriorCrossing / CollinearOverlap /
  DuplicateTraversal), `record_conflict`, `record_overlap_conflict` (separate
  `overlap_witnesses` vector that feeds `derive_loss_bucket`).

### Arrangement/provenance that survives into constraint insertion
- `truck-meshalgo/src/tessellation/source_evidence.rs` — `SourceFaceInput`,
  `BoundId`, `EdgeUseId`, `SourceBoundInput`, `OrientationEvidence` (the
  authoritative STEP provenance).
- `truck-meshalgo/src/tessellation/diagnosis.rs` — `SemanticSegmentRef`
  (semantic_constraint_id, origin, boundary_component, source_bound,
  source_edge_use), built from `SegmentOrigin` in `record_segment`.

### CDT construction and failure reporting
- `truck-meshalgo/src/tessellation/triangulation.rs` — `Cdt` type (spade
  `ConstrainedDelaunayTriangulation`, ~:30), `insert_to` (~:5457),
  `trimming_tessellation_with_outcome` (~:6250) and `trimming_tessellation`
  (~:6300) — the CDT → parity → material pipeline and typed
  `TessellationFailure` reporting.

### Material/parity selection and post-material degeneracy
- `truck-meshalgo/src/tessellation/triangulation.rs` — `ParityReading`
  (~:5921), `flood_parity` (~:6418, raises `ContradictoryDualParity`),
  `triangulation_into_polymesh_outcome` (~:6531), the winding-parity retry
  (~:1877, gate `winding_parity_enabled()`), and the fused
  material-selection → validation chain (~:6643).

### Where `NoOddParityRegion` is generated
- `truck-meshalgo/src/tessellation/triangulation.rs` ~:6689 — returned when
  parity selected no odd-parity cells (or the selected cells were emptied by
  degeneracy/zero-area validation).

### Look `face_census` reporting path / where to add ARR-tail diagnostics
- `look/examples/face_census.rs` — `census()` ~:276 (per-face FACE ledger lines,
  rendered/triangles), `--ledger`, `TRUCK_FACE_DIAG_JSONL` row emission ~:1026,
  per-route funnels (`BandTally`/`ConeBandTally`/`TorusTally`). Best place for
  ARR-tail structured output: the JSONL `FailedFaceDiagnosis` (already carries
  `terminal_reason`, `route_decisions`, `projection_witness`) plus a new
  admission→terminal cross-table in a `proj3`-style analysis script.

### Runtime/env gates for projection recovery + diagnostics
- `truck-meshalgo/src/tessellation/diagnosis.rs` — `recovery_route_enabled`
  (~:862), `formal_recovery_enabled` (`TRUCK_FORMAL_RECOVERY`),
  `spline_seed_recovery_enabled` (`TRUCK_FORMAL_RECOVERY_SEED`),
  `proj_residual_recovery_enabled` (`TRUCK_FORMAL_RECOVERY_PROJ_STAGE_A`),
  `diag_enabled` (`TRUCK_FACE_DIAG_JSONL`).
- `truck-meshalgo/src/tessellation/triangulation.rs` — `projection_probe_enabled`
  (`TRUCK_PROBE_PROJ`), `projection_deep_probe_enabled`
  (`TRUCK_PROBE_PROJ_DEEP`), `proj_residual_recovery_enabled_cached`,
  `TRUCK_PROBE_PROJ_RECOVERY` (Stage A admission probe line `PROJ_RECOVER`),
  `TRUCK_LIFT_SINGULAR_RECOVERY`, `TRUCK_PROBE_LIFT`, `TRUCK_PROBE_PARITY`,
  `TRUCK_COMPAT_FACTOR`.

### Existing targeted tests relevant to B/C
- `truck-meshalgo/src/tessellation/triangulation.rs`
  - `mod proj003_stage_a_tests` (end of file) — admission contract tests
    (ProductionMiss admit, genuine miss, out-of-domain, non-finite, no-ran) +
    `shared_outcome_matches_legacy_newton` battery.
  - `cone_topology_tests`, `singular_transition_tests`, `segment_origin_tests`,
    `test_parity_*` — periodic lift, singular transitions, boundary labelling,
    parity.
- `truck-meshalgo/src/tessellation/diagnosis.rs` — bucket/derivation tests
  (`derive_loss_bucket`, `deterministic_serialization`, etc.).

### Existing PROJ-002 / PROJ-003 corpus ledgers + analysis scripts (reuse, don't rerun)
- `look-corpus/p1-out/proj2/` — per-model `.jsonl` deep-witness + `.census.txt`
  (final: 3,703 BoundaryProjectionFailed; ProductionMiss 2,393 / Domain 789 /
  SeedBasinGap 213 / Inconclusive 278 / NearestTooFar 30).
- `look-corpus/p1-out/proj2_tab.py` — verdict tabulation.
- `look-corpus/p1-out/proj3/` — `base/`, `stageA/` (per-model `.ledger.txt`,
  `.census.txt`, `.jsonl`; byte-exact via `cmd /c` redirect), `nist/`
  (`base/`+`stageA/`), `sweep.ps1`, `sweep_nist.ps1`, `reconcile.py`,
  `analyze.py`.
- `look-corpus/nist-census/nist_files.txt` — the 33-file NIST list.

---

## The 5–8 files the next agent should read first
1. `truck-fork/truck-meshalgo/src/tessellation/triangulation.rs` (whole
   projection → boundary → CDT → parity pipeline; this is the workbench).
2. `truck-fork/truck-geotrait/src/algo/surface.rs` (shared nearest solver +
   `search_nearest_parameter_outcome`).
3. `truck-fork/truck-meshalgo/src/tessellation/diagnosis.rs` (gates, witnesses,
   sink).
4. `truck-fork/truck-geometry/src/nurbs/bspsurface.rs` + `nurbssurface.rs`
   (`search_parameter_seeds`, nearest-parameter delegation — Stage B).
5. `truck-fork/truck-meshalgo/src/tessellation/domain/lattice.rs`
   (`CertifiedLattice` periods/generators — Stage C).
6. `look/examples/face_census.rs` (reporting path — ARR-TAIL).
7. `look-corpus/p1-out/proj3/reconcile.py` + `analyze.py` (the reconciliation
   harness).

## Likely files that would actually need edits
- **Stage B**: `triangulation.rs` (`by_structural_seeds`, the `seed_best`
  computation block, the `(None,false)` arm in `try_new`); maybe
  `bspsurface.rs`/`nurbssurface.rs` if seed ordering needs work.
- **Stage C**: `triangulation.rs` (`residual_certified_admission` + a new
  periodic-normalization helper near `get_mindiff`; the periodic lift
  machinery); `lattice.rs` (`generator()` reads); `diagnosis.rs` (a
  `DomainOrContractIssue` subtype witness).
- **ARR-TAIL**: `triangulation.rs` (already has the terminal-reason enums);
  `face_census.rs` (reporting); a new `proj3`-style analysis script.
- `truck-geotrait/src/algo/surface.rs` and `Cargo.toml` — only if a solver API
  change is genuinely needed (avoid).

## Areas to safely ignore
- `truck-meshalgo` filters/analyzers/vtk; `truck-modeling`; `truck-stepio`
  internals; `truck-topology`.
- The Look GPU/render path (`src/` wgpu/atlas/session), `src/step/part21`
  parser, `benchmarks/`, `tests/` gpu smoke, examples other than
  `face_census.rs` and `tolerance_probe.rs`.
- `look-corpus/p1-out/projB`, `proj2-cost`, `seed`, `diag-*` (historical; the
  proj3 harness supersedes them for reconciliation).

---

## Commit / pin (final for this wave)
- Fork: `cargo test -p truck-meshalgo --lib` (615 pass) green; committed
  `67c19555` "Land PROJ-003 Stage B/C ... + ARR-TAIL signature" (only
  `diagnosis.rs` + `triangulation.rs`); pushed to `feature/cone-apex-lift-recovery`.
  The `tessellation` integration-test failures (5) are pre-existing at `bcf676ee`
  and unchanged by this wave.
- Look: `.cargo/config.toml` override re-commented; every `stefangolas/truck`
  rev in `Cargo.toml` bumped to `67c19555`; `Cargo.lock` re-resolved (only the
  truck source entries changed); `examples/face_census.rs` gained the
  `arr` field. Verified `cargo check --locked` + full wave below.
- New gates: `TRUCK_FORMAL_RECOVERY_PROJ_STAGE_B`,
  `TRUCK_FORMAL_RECOVERY_PROJ_STAGE_C` (both nested under `TRUCK_FORMAL_RECOVERY`).
  B/C are default-on with explicit `=0` opt-out, like every formal route.
- New/adjusted probe lines: `PROJ_RECOVER_B`, `PROJ_RECOVER_C` (each carry
  `admitted`, `residual_min/max`, `tol`; C also `classes=<Class>=<n>,...`).

## Wave-2 code locations
- `truck-meshalgo/src/tessellation/triangulation.rs`
  - `admit_outcome` (~:360) — the shared residual-certified contract (finite
    UV, in-domain, finite eval, `|S|-P| <= tol`).
  - `residual_certified_seed_recovery` / `_seed_admission` (~:490) — Stage B.
  - `classify_domain_point` + `DOMAIN_BOUNDARY_EPSILON_REL` (~:520) — Stage C
    classification (1e-6 of span).
  - `residual_certified_domain_recovery` (~:610) — Stage C admission.
  - `probe_structural_seeds` (~:270) — shared seed-nearest search.
  - The `(None,false)` arm of `PolyBoundaryPiece::try_new` (~:4130) — the chain
    `Stage A → Stage B → Stage C → fail`; per-face accumulators and the
    `PROJ_RECOVER_B/C` emission (~:4370).
  - `mod proj003_stage_a_tests` — 16 tests including the 11 new Stage B/C ones
    (unit cylinder `RevolutedCurve<Line>` for periodic equivalence).
- `truck-meshalgo/src/tessellation/diagnosis.rs`
  - `DomainRecoveryClass`, `ProjectionPointEvidence` (witness `point_evidence`).
  - `ArrFailureStage`, `ArrMaterialStage`, `ArrProvenanceClass`, `ArrSeamOrDeck`,
    `CurvePairClass`, `ArrSignature` (`FailedFaceDiagnosis.arr`), and
    `derive_arr_signature` (in `build_face_diagnosis`).
  - Gates `proj_seed_recovery_enabled` / `proj_domain_recovery_enabled`.
- Corpus harness: `look-corpus/p1-out/proj3/{sweep_abc,sweep_nist_abc,
  run_remaining_abc}.ps1`, `abc/` and `nist/abc/` output, `analyze_abc.py`
  (B/C + reconciliation), `arr_tail.py` (ranked signatures + concentration +
  cross-tabs).
