# BG-CK-P0-PREVALENCE — measure analytic-pair prevalence on the corpus

Certified-kernel plan Phase 0's exit gate requires a published prevalence
table: "analytic pairs are the majority" is a hypothesis, not a result — this
packet measures it on the local corpus now. The number decides whether Phase 2
(class 2 generic certified SSI) is urgent or deferrable. This is a MEASUREMENT
packet: no kernel code changes, no behavior changes, one new read-only test
and one published doc.

```yaml
id:          BG-CK-P0-PREVALENCE
contract:    [BG-CK-P0-PREVALENCE]
class:       mechanical
crates:      [look]
depends_on:  []
write_allow:
  - tests/certified_prevalence.rs
  - docs/CERTIFIED_PREVALENCE.md
read_allow:
  - CERTIFIED-KERNEL-PLAN.md
  - docs/CERTIFICATE_MAPPING.md
  - src/step.rs
  - src/step/circular_arc.rs
  - vendor/truck/truck-meshalgo/src/tessellation/formal/support.rs
  - vendor/truck/truck-meshalgo/src/tessellation/formal/cylinder.rs
  - vendor/truck/truck-meshalgo/src/tessellation/formal/cone.rs
  - vendor/truck/truck-meshalgo/src/tessellation/formal/torus.rs
  - vendor/truck/truck-stepio/src/lib.rs
budget:      {turns: 35, ctx_tokens: 120000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub fn identify_plane' vendor/truck/truck-meshalgo/src/tessellation/formal/support.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn identify_cylinder' vendor/truck/truck-meshalgo/src/tessellation/formal/cylinder.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub fn identify_cone' vendor/truck/truck-meshalgo/src/tessellation/formal/cone.rs"}
  - {id: A4, expect: 2, cmd: "grep -c 'pub fn identify_torus' vendor/truck/truck-meshalgo/src/tessellation/formal/torus.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'pub fn face_adjacency' vendor/truck/truck-topology/src/shell.rs"}
  - {id: A6, expect: 38, cmd: "find /c/Users/stefa/look-corpus -name '*.step' -o -name '*.stp' | wc -l"}
```

The corpus is the `LOOK_CORPUS` checkout at `C:\Users\stefa\look-corpus` (see
its README): 38 STEP files total (anchor A6) — 5 large assemblies
(ur10, formula1, core_xy, jackhammer, quadruped) plus the NIST PMI set
(33 files). The census test reads the corpus root from the `LOOK_CORPUS`
environment variable (the repo's own regression-test convention) and skips
with a clear message when it is unset.

## What this packet measures — definitions are the contract, pre-made

**Analytic set** = the five carriers the plan's class-2 fast path dispatches
on: plane, cylinder, cone, sphere, torus. A pair is **analytic** iff BOTH
sides classify into the analytic set. The headline number is the fraction of
adjacent face pairs that are analytic pairs.

**Classifier (one face's support surface, in priority order):**

1. `Plane` — `identify_plane` (`formal/support.rs`) accepts the surface (the
   schema carries authority, i.e. NOT the `Unresolved`/failure arms).
2. `Cylinder` — the surface is a `RevolutedCurve<Line<Point3>>`-shaped
   support and `identify_cylinder` (`formal/cylinder.rs`) accepts it.
3. `Cone` — same representation shape, `identify_cone` (`formal/cone.rs`)
   accepts it.
4. `Torus` — the support is a `Torus` and `identify_torus`/`identify_torus_world`
   (`formal/torus.rs`) accepts it.
5. `Sphere` — no landed certified constructor exists; classify by the landed
   in-memory representation (a sphere-carried surface). Tag it `Sphere` and
   record in the doc that its evidence is representation-named, not
   certified-identified (this is itself a Phase-1 gap finding, publish it).
6. `Spline` — the support is B-spline/Bézier carried.
7. `Other(tag)` — everything else, tagged by the representation's own name
   (the `NoStructuralReader` doctrine: record what the representation says it
   is, never guess "probably a plane").

Machine-check every constructor's exact signature against the landed source
before calling it; the identification constructors are REFUSING constructors
— a refusal is the classifier saying "not this class", which is exactly the
dispatch order the Phase-1 fast path will run. Record, per face, only the
FIRST accepting class. Every `Other` bucket's counts and tags go in the doc —
an unexplained residual bucket is a finding, not noise.

**Pairs** = adjacent face pairs via `shell.face_adjacency()`
(`truck-topology/src/shell.rs`); walk each loaded model's shells; each
adjacent (face, face) pair contributes one row to the pair histogram keyed by
(class_a, class_b) with the pair treated UNORDERED (min/max by class tag) so
plane/cylinder and cylinder/plane are one bucket.

**Deliverable — `docs/CERTIFIED_PREVALENCE.md`** must contain:

1. The headline: analytic-pair fraction (and per-side analytic fraction) over
   ALL corpus files, stated as exact counts (N_analytic_pairs / N_pairs), no
   percentages without the denominator.
2. The per-class face histogram and the per-pair-class histogram (aggregated,
   plus a per-file table: file, faces, adjacent pairs, analytic pairs).
3. The verdict row the plan asks for: does the analytic share justify
   deferring Phase 2 (state the plan's own decision rule: the fast path
   "carries the majority of pairs per the design's own corpus claim" — your
   number confirms or refutes it; do not editorialize beyond that).
4. Method notes: the classifier order, the Sphere/Other evidence caveat, the
   loader used, and the exact command to reproduce
   (`cargo test -p look --test certified_prevalence -- --ignored --nocapture`).

## The test — `tests/certified_prevalence.rs` (look root crate, NEW file)

A single `#[ignore = "corpus census: needs LOOK_CORPUS; run explicitly"]`
test that: locates the corpus from `LOOK_CORPUS` (skip with a clear message
when unset — do not fail CI on a missing corpus), loads each `.step`/`.stp`
file through the SAME landed import path look's renderer uses (read
`src/step.rs` for the entry; do not invent a second parser route), classifies
every face and every adjacent pair per the definitions above, and PRINTS the
JSON rows (`{"file":…, "faces":…, "pairs":…, "face_histogram":…,
"pair_histogram":…}`) plus the aggregate headline so the doc's numbers are
copy-out reproducible. The test asserts only structural sanity (corpus found,
faces > 0 per file, every face classified into one of the seven buckets) —
the MEASUREMENT is the output, not a pass/fail threshold. Never add a
threshold assertion on the analytic fraction: that would make the measurement
self-fulfilling.

House rules: the look root crate is NOT fmt-clean at base — run
`cargo fmt --all -- --check` and if pre-existing drift outside your file
fires, note it in RESULT (V3 grades only your diff's lint surface; do not
reformat others' files). Float comparisons in the classifier need the `//
H-3` opt-out ON THE SAME LINE as the comparison. Clippy
(`cargo clippy -p look --test certified_prevalence --message-format=short
--no-deps`) zero findings on your file.

## Stop conditions

Stop, commit nothing beyond WIP evidence, write RESULT.json (AT THE WORKTREE
ROOT) with the finding verbatim if:

1. A corpus file cannot be loaded through the landed path (record the file,
   the error verbatim, and EXCLUDE it from the table with its exclusion
   reason — an excluded file is a finding about the loader, publish it; do
   not hand-patch around the failure).
2. The landed import path does not expose per-face support surfaces without
   new kernel plumbing (that would be a kernel change — this packet changes
   no kernel code; file the gap).
3. A support surface matches none of the seven buckets with an honest tag.

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

Commit your work on the current branch (subject: `test(corpus): certified
prevalence census (BG-CK-P0-PREVALENCE)`) BEFORE writing `RESULT.json`. Both
deliverables are contract: the test exists with the run command working, and
the doc carries the measured tables with exact counts.
