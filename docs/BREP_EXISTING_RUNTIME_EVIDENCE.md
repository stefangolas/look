# BRep existing runtime evidence — inventory only (nothing was run)

This document collects **only timing data already recorded** in the
repository as of 2026-09-08. No benchmark, test, corpus row, kernel
operation, or instrumentation was executed to produce it. Each datum states
what was actually measured, on which path, in which configuration, and its
interpretation limits. Classification: `MEASURED` (raw numbers with a stated
protocol) / `PARTIALLY ATTRIBUTABLE` (a number exists, stage attribution is
incomplete) / `WHOLE-OP ONLY` / `NO EXISTING DATA`.

Headline negative finding, recorded authoritatively in
`docs/audits/BREP_BENCHMARK_AUDIT.md`: *"All three [TTC rows] lack a
successful truck timing column … **Current complete-model kernel timings and
candidate speedups: UNMEASURED.**"* Every kernel-vs-OCC comparison attempted
so far ended at a red facts gate (kernel typed refusal) before any kernel
geometry ran — except the PB-011 r1 cutaway lift (§3.4).

---

## 1. Certified kernel operations

### 1.1 Certified pair dispatch (Phase-1 floor) — MEASURED
- **Source:** `loop/results/BG-CK-P1-FLOOR.STOP.json` (lines 40, 67-68;
  aggregate `CERTIFIED_PHASE1_FLOOR_AGGREGATE`), booked at
  `loop/PACKETS.jsonl:151`.
- **Operation:** `truck_certified::pair_dispatch::dispatch_pair` per adjacent
  face pair, full 38-file `LOOK_CORPUS` walk: 166,307 pairs walked, 71,957
  admitted. Exact analytic screens only (plane/plane, plane/cylinder,
  plane/sphere, cylinder/cylinder, sphere/sphere, …).
- **Config:** release (`cargo test --release -j 2 -p look --test
  certified_phase1_floor -- --ignored`).
- **Values:** median 6,400 ns; p95 23,100 ns; max 8,224,400 ns (8.22 ms);
  min 0.
- **Limits:** no legacy-path comparator ("DEFERRED TO INTEGRATION … no
  directly-callable legacy pair-contact entry exists in the test crate"); no
  per-pair-class latency breakdown; the run stopped on stop-condition 3
  (certify-rate 0.586 < 0.95 floor, 4,381 adjacent pairs certified disjoint);
  fresh-process variance noted on a repeat run. This is the only per-call
  latency distribution for any certified-kernel operation in the repo.

### 1.2 Certified SSI trace (Phase-2 floor) — PARTIALLY ATTRIBUTABLE
- **Source:** `docs/CERTIFIED_PHASE2_FLOOR.md:27-28, 50-52, 172-175,
  220-225`; result context `loop/results/BG-CK-P2-RESIDUAL` (W1 `ssi.rs`, W2
  `ssi_trace.rs`).
- **Operation:** composed chain ending in
  `truck_certified::ssi_trace::certified_pair_trace` over 726 admitted
  spline~spline face pairs (226,654 unit patch-pairs); 400 unit-pairs traced
  before `PHASE2_TRACE_BUDGET=400` spent; **6 pairs fully dispositioned**,
  certify-rate 0.0 (2 conditioning, 3 non_transverse, 1 singular).
- **Config:** debug (test profile), explicitly recorded; the doc notes a
  release run "reproduces them (pair counts identical; wall time shorter)"
  but **no release wall was recorded**.
- **Value:** `wall_seconds: 279.24` for the entire harness run over 38 STEP
  files.
- **Limits:** whole-process wall = corpus load + census + admission + the
  400 traced unit-pairs; per-pair or per-unit-pair cost **cannot be
  extracted**. Budget-bound truncation (720/726). This is a refusal-
  distribution finding, not a performance datum — and it is the **only
  existing runtime evidence for the certified SSI path**.

### 1.3 Seed-direction decision overhead (boolean classify support) — MEASURED, fixture-scale, debug
- **Source:** `docs/AUDIT_SEEDRAY_AVOIDANCE.md` §T4 (430-449).
- **Operation:** per ray-segment quadratic projection, per edge verdict,
  per seed-direction decision in the truck-shapeops seed rules.
- **Config:** debug ("the Done command runs the dev profile"); 3 synthetic
  fixture pairs, 98 seed-direction decisions; values vary run-to-run.
- **Values:** tessellated distance census ~0.22–0.24 µs per ray-segment QP;
  ~0.91–1.00 ms per seed-direction decision (the doc flags this row as the
  *naive tessellated* measure, NOT the intended criterion's cost);
  criterion-(b) Line-edge ~0.24–0.42 µs (12-edge block ~2.9–5.0 µs/decision);
  Circle-edge ~1.8–2.9 µs (~3.7–5.8 µs/decision).
- **Limits:** the interval-hull production path "is not implemented here;
  its cost is bracketed between (b) and the tessellated census". Not a
  corpus or release measurement.

### 1.4 Tangency (CTE), torus quartic reduction, deviation, fid — NO EXISTING DATA
- `docs/CERTIFIED_TANGENCY_BUILD_SPEC.md`, `docs/CTE_BUILD_SPINE.md`,
  `docs/CERTIFIED_TANGENCY_EXACT_CONTACT_SPEC.md`,
  `docs/CERTIFIED_PHASE2_BOOKING.md` book gates and LOC budgets; no runtime
  figures.
- `docs/TORUS_CONTACT_PROGRAM.md` (TOR-A) states for the quartic implicit
  composition: "the cost is the concern — **measure before committing**".
  The measurement has not been made.
- `deviation.rs`, `fid/*`, `construct/*` (blend/canal/shell/setback), kernel
  Krawczyk operators, atlas, trimclip: no recorded durations anywhere.

---

## 2. Boolean

### 2.1 Kernel boolean on any pair — NO EXISTING DATA
- No file records a wall time for `truck-shapeops::boolean()` on any pair.
  `loop/results/BG-SOL-M2-WITNESS.json`, `BG-SOL-RW2-SPLIT.json`,
  `BG-SOL-RW4-ASSEMBLE.json`, `CL-005-EXACT-CONTACT.json`,
  `BG-CAD-P3-SPLIT*.json`, `CL-005-STOP-QUESTION.md` record face counts,
  fragment/adjacency censuses, and typed refusals — never durations. The M2
  flagship battery (`Budget::new(1000,1000,1000)`) reports budget **spend
  accounting**, not wall time.
- OCC comparator (external, for contrast): `docs/TT_TIMING_RESULTS.md` —
  f1/monocoque OCC build spends **36.454 s median inside 2 boolean ops**
  (`Shape.cut` of the lofted tub skin + 14-body proud-boss fuse), measured
  by an observer harness segmenting `Shape.fuse`/`Shape.cut`. That is an
  OCC-side stage measurement, not a kernel number.

---

## 3. Construction and realization

### 3.1 Construction ops (extrude/revolve/loft/sweep/fillet) — NO EXISTING DATA
- `loop/results/BG-CAD-P1..P12*`, `BG-CG-*`: verification is exactness-based
  ("measured f64-EXACT (achieved precision 0)") — precision, never time.
- `docs/CONSTRUCTIVE_GEOMETRY_PLAN.md:331` books "measure construction wall
  time, allocations if available" as future work — confirming it does not
  exist yet.

### 3.2 Tessellation (truck-meshalgo production path) — MEASURED
- **Source:** `UNSEEN_MODEL_PERFORMANCE_AUDIT.md` (release build, per-face
  production path `wrap_shell_with_closure` +
  `robust_triangulation_with_torus_outcome`).
- **Pathological-face study (UR10):** two faces hung >100 s each (declared-
  range open-piece boundary → CDT constraint-insertion blowup; growth 96 ms
  @2,604 constraints → 15.8 s @19,722 → >100 s; per-constraint 37 µs →
  800 µs+, near-quadratic; memory flat 188 MB). After the face-local
  `working_range` fix (truck `09726a9e`): 12.5 ms / 10.9 ms / 4.3 ms. Whole
  UR10: before >90 s direct / >600 s corpus timeout; after wall 3.2 s
  (step_parse 143 ms, step_table 138 ms, step_tessellate 2,173 ms; peak
  ~396 MB; 502,130 tris).
- **Healthy-face rate:** 6,046 faces in 12,849 ms sequential CPU (~4 ms/face
  average; slowest legitimate faces 1,168–1,400 ms — 46k–123k-tri B-spline
  fillets; 8 R01 refusals 2–25 ms each).
- **Whole-process controls (release):** core_xy parse 90.3 / table 92.2 /
  tessellate 2,115.5 ms, wall 4,964 ms, 668,351 tris; formula1 266.7 /
  219.4 / 1,541.1 ms, wall 3,092 ms. (A quieter-moment inherited core_xy
  set: 49 / 57 / 718 ms, total 2.42 s — same machine; GPU-adapter init lands
  on the wall.)
- **Limits:** UR10 classification P2 (two faces); per-face instrumentation
  with external timeout; the "before >600 s" is a corpus timeout, not a
  clean measurement. This is the best stage-attributed kernel-side data in
  the repo, and it covers **tessellation**, not construction/boolean/SSI.
- Band routes (cone/cylinder/torus recovery; `docs/ABC_BAND_SWEEP.md`,
  `docs/ABC_CONE_BAND_SWEEP.md`, `docs/RESIDUAL_DIAGNOSTICS.md` §10-11):
  face counts and residual ceilings only (e.g. 1,283,165 constraints
  presented / 62,468 unrealized, 4.87%; 3,703 BPF faces with 91.7% ≤1×tol)
  — **no route runtime measured**.

### 3.3 FAC / constructive realization — NO EXISTING DATA
- `facet_sweep` timings were never recorded; the Exeter regression gate
  (`docs/CONSTRUCTIVE_GEOMETRY_PLAN.md` §8) books the measurement for the
  client migration, not yet run.

### 3.4 Swept-carrier corpus lift (PB-011 r1) — PARTIALLY ATTRIBUTABLE
- **Source:** `loop/STATE.md` ("PB-011 r1 LANDED 2a12623 … cutaway lifted
  green **~11 s / 409k tris**; the facade dispatch is a MIRROR of the CL-006
  solver-entry dispatch per the dependency law").
- **Operation:** first kernel-side corpus geometry through the
  swept-carrier boolean routing (falcon_heavy/cutaway).
- **Limits:** single figure recorded in the session state; no committed
  result artifact with protocol, configuration, or stage split was found;
  treat as indicative, not citable performance evidence. The monocoque lift
  (PB-011 r2) was in flight and blocked at the time of the snapshot — no
  number.
- Related: OCC reference facts for all 34 staged rows are recorded
  (`corpus/ttc/reference/*.json`; wall times in
  `scratch/reference_batch_log.json` per `loop/STATE.md`). The OCC column of
  the timing table is measured; **the truck column does not exist yet**
  except §3.4.

---

## 4. Text-to-CAD corpus comparisons (kernel-vs-OCC door) — OCC measured, kernel DNF

- **Protocol (recorded):** release-built `truck123d` native module, quiet
  machine, fresh process per run, 1 conditioning + 5 measured runs, median +
  raw samples retained, DNF rows kept, facts gate adjudicated before timing.
  Windows x86_64; Python 3.14.3, build123d 0.11.1, cadquery-ocp 7.9.3.1.1.
- **f1/monocoque (OCC):** MEASURED, stage-segmented — construction 5.639 s /
  boolean 36.454 s / tessellation 0.195 s / build 42.096 s medians
  (`docs/TT_TIMING_RESULTS.md`; `loop/results/TTC-TIMING-MONO.json`).
  Kernel column: **DNF-KERNEL** (typed refusal before geometry).
- **falcon_heavy/nozzle_assembly:** OCC wall median 4.123 s; door
  `build_seconds` 3.313 s; kernel **DNF-FACTS** (typed refusal at first
  spline-profile revolve). **mvac:** 4.170 s / 3.307 s; kernel DNF-FACTS.
  WHOLE-OP ONLY on the OCC side ("a `build_seconds`-only column would hide
  the kernel's geometry time while including all of OCC's" — the two
  denominators "must not be pooled", `docs/audits/BREP_BENCHMARK_AUDIT.md:194`).
- **hypercar/wheels:** OCC `build_seconds` 160.4 single run
  (`loop/results/PB-012-HYPERCAR-VENDOR.json`), WHOLE-OP ONLY; the other 12
  Hypercar systems stage-skipped (`booleans-on-swept-carriers`).
- **Structural gap B9** (`BREP_BENCHMARK_AUDIT.md`): the door's
  `build_seconds` field ends before truck facts/mesh execution, so even a
  future green kernel run would be misattributed until the field is fixed.

---

## 5. look renderer / STEP load path (secondary, headline figures)

- `docs/BENCHMARKS.md` (RTX 5050 Laptop, DX12, MEASURED): fresh-process look
  vs F3D 3.5 geometric mean 1.40× (Khronos set), New York Boulevard 2.001×,
  Sponza-2k 6.432×, foliage 5.943×; STEP NIST medians 643.5–687.7 ms
  (floor ≈650 ms process+adapter); stage splits per model (e.g. stc_09 parse
  54.3 / table 67.2 / tessellate 10.0 ms); Part-21 reader stc_09 729.9 →
  54.3 ms (~80 MB/s); look-vs-F3D STEP geometric mean 1.26×. Resident path:
  3.459 ms/view vs Three.js WebGL2 7.600 ms (2.20×); "PNG encoding and file
  output dominate the resident path". Corpus regression baseline timings
  explicitly untrusted (`benchmarks/corpus_regression_baseline.json`).
- `AGENTS.md:156-166` / README: core_xy.step fresh ≈2.0 s @512×512 vs OCCT
  via F3D ≈6.4 s (3.2×), WHOLE-OP ONLY, with the stated caveat (one
  assembly, one GPU; exact README protocol required).
- Build-time (not kernel-op): `BUILD_SPEEDUP_RESULTS.md` MEASURED — release
  edit→rebuild L1 49.52 s / L2 77.52 s medians vs Quick 6.98/7.65 s;
  quick-vs-release runtime ratios 1.3–1.6×.

---

## 6. Explicit list of transitions with NO existing timing data

1. Generic certified SSI per-pair/per-unit-pair cost (only the budget-
   exhausted 279.2 s debug whole-run exists; release wall never recorded).
2. Tangency stack (cascade, Hessian, exact-contact closure) — nothing.
3. Boolean on nontrivial pairs (canonical or swept) — zero kernel wall
   times; only OCC baselines and typed kernel refusals.
4. Phase-1 dispatch vs legacy pair-contact throughput — explicitly deferred
   to integration; no legacy comparator entry exists.
5. Construction ops (extrude, revolve, sweep, loft, fillet, facet_sweep) —
   none.
6. Contact funnel FE/EE and seed-ray at corpus scale/release — only debug
   fixture-scale µs-class samples.
7. Band/recovery routes runtime — populations only.
8. Torus quartic implicit reduction (TOR-A) — flagged measure-before-commit.
9. Whole-model kernel timings on any TTC row — all kernel columns DNF except
   the PB-011 cutaway figure; B9 makes even a green run misattributable.
10. Kernel-vs-OCC differential timing — does not exist.

**Standing rule observed throughout:** absence of measurement is reported as
absence; no performance conclusion in this document is inferred from source
inspection. Static plausibility analysis belongs to
`BREP_THEORY_OPPORTUNITIES.md` and is labelled as such there.
