# Certified prevalence — analytic-pair census on the look-corpus

**Packet.** `BG-CK-P0-PREVALENCE` (certified-kernel Phase 0 exit gate). The
question the gate asks is not a hypothesis but a measurement: of all adjacent
face pairs in the corpus, how many are **analytic pairs** — pairs whose two
support surfaces both classify into the five-carrier analytic set that the
class-2 fast path dispatches on (plane, cylinder, cone, sphere, torus)?

Measured 2026-08-31 against the full `LOOK_CORPUS` checkout at
`C:\Users\stefa\look-corpus` (38 STEP files). All numbers below were produced
by `tests/certified_prevalence.rs` and are copy-out reproducible with the exact
command in [Reproduction](#reproduction).

## Headline

| Quantity | Exact count | Fraction |
|---|---|---|
| Adjacent face pairs (all 38 files) | 166,307 | — |
| **Analytic pairs** (both sides analytic) | **103,649 / 166,307** | **62.32 %** |
| Faces (all 38 files) | 71,390 | — |
| **Analytic faces** (per-side analytic share) | **52,034 / 71,390** | **72.89 %** |

All 38 corpus files loaded through the landed import path and were measured;
none was excluded. The NIST set is 96.44 % analytic pairs (16,839 / 17,461)
while the five large assemblies are 58.32 % (86,810 / 148,846); the combined
corpus headline is 62.32 %.

## Verdict (the plan's own decision rule)

The plan's Phase-2 deferral rule is: the fast path "carries the majority of
pairs per the design's own corpus claim". The measured share of adjacent face
pairs that are analytic pairs is

```
103,649 / 166,307 = 62.32 %
```

which is a strict majority of pairs (50 % is the threshold). The design's
corpus claim is **confirmed** on this corpus: the class-2 analytic fast path
carries the majority of adjacent pairs, so per the plan's own rule the
analytic share does justify deferring Phase 2. No other claim is made here.

## Aggregate face histogram

One row per face, per the classifier's first-accepting class. Exact counts over
all 38 files.

| Class | Faces |
|---|---|
| `Plane` | 20,570 |
| `Cylinder` | 18,582 |
| `Cone` | 5,364 |
| `Torus` | 5,687 |
| `Sphere` | 1,831 |
| `Spline` | 19,072 |
| `Other` | 284 |
| **Total** | **71,390** |

Analytic carriers (Plane, Cylinder, Cone, Torus, Sphere): 52,034 faces
(72.89 %). Non-analytic (Spline, Other): 19,356 faces.

### The `Other` residual bucket

Every `Other` face is tagged by its representation's own name. The residual is
fully named — nothing lands in an unexplained bucket:

| `Other` tag | Faces |
|---|---|
| `degenerate_toroidal_surface` | 280 |
| `toroidal_surface` | 4 |
| **Total** | **284** |

- **280 `degenerate_toroidal_surface`** — STEP's `degenerate_toroidal_surface`
  entity (`major < minor`, one sheet). No certified reader exists for it today;
  it is a genuine Phase-1 coverage gap, correctly not guessed into `Torus`.
  Distribution: `formula1` 142, `quadruped` 99, `jackhammer` 39.
- **4 `toroidal_surface`** — faces whose carrier *is* a torus but whose
  world-space parameters `identify_torus_world` refused (`formula1`): a
  spindle/horn torus or a non-similarity placement is not a regular embedded
  ring torus. These are the certified reader refusing honestly, exactly the
  dispatch behavior the fast path will run.

## Aggregate pair histogram

Each adjacent unordered (face, face) pair is one row, keyed by
`min(class_a, class_b) ~ max(class_a, class_b)` so `plane~cylinder` and
`cylinder~plane` are one bucket. Exact counts over all 38 files. Analytic pairs
are the rows whose two classes are both in the analytic set.

| Pair | Count |
|---|---|
| `cone~cone` | 2,050 |
| `cone~cylinder` | 3,918 |
| `cone~other` | 132 |
| `cone~plane` | 8,379 |
| `cone~sphere` | 36 |
| `cone~spline` | 3,808 |
| `cone~torus` | 2,400 |
| `cylinder~cylinder` | 5,354 |
| `cylinder~other` | 200 |
| `cylinder~plane` | 37,361 |
| `cylinder~sphere` | 3,249 |
| `cylinder~spline` | 15,566 |
| `cylinder~torus` | 6,436 |
| `other~other` | 16 |
| `other~plane` | 160 |
| `other~sphere` | 8 |
| `other~spline` | 282 |
| `other~torus` | 220 |
| `plane~plane` | 26,274 |
| `plane~sphere` | 281 |
| `plane~spline` | 16,137 |
| `plane~torus` | 5,385 |
| `sphere~sphere` | 126 |
| `sphere~spline` | 1,202 |
| `sphere~torus` | 539 |
| `spline~spline` | 21,004 |
| `spline~torus` | 3,923 |
| `torus~torus` | 1,861 |
| **Total** | **166,307** |

Analytic rows sum to **103,649**; rows involving a non-analytic side sum to
**62,658**.

## Per-file table

`file`, `shells` (STEP shells declared in the table), `faces` (faces that
survived conversion and were classified), `pairs` (adjacent face pairs),
`analytic_pairs`. Exact counts, 38 files.

| file | shells | faces | pairs | analytic pairs |
|---|---|---|---|---|
| `core_xy/core_xy.step` | 79 | 5,670 | 13,090 | 12,438 |
| `formula1/formula1.step` | 12 | 5,235 | 12,447 | 2,912 |
| `jackhammer/jackhammer.step` | 189 | 17,143 | 41,698 | 24,562 |
| `quadruped/quadruped.step` | 195 | 29,392 | 68,193 | 40,796 |
| `ur10/ur10.step` | 58 | 6,048 | 13,418 | 6,102 |
| `nist/NIST-PMI-STEP-Files/AP203 geometry only/nist_ctc_01_asme1_rd.stp` | 1 | 139 | 370 | 370 |
| `nist/NIST-PMI-STEP-Files/AP203 geometry only/nist_ctc_02_asme1_rc.stp` | 2 | 664 | 1,302 | 1,188 |
| `nist/NIST-PMI-STEP-Files/AP203 geometry only/nist_ctc_03_asme1_rc.stp` | 1 | 139 | 390 | 390 |
| `nist/NIST-PMI-STEP-Files/AP203 geometry only/nist_ctc_04_asme1_rd.stp` | 4 | 520 | 1,092 | 1,092 |
| `nist/NIST-PMI-STEP-Files/AP203 geometry only/nist_ctc_05_asme1_rd.stp` | 2 | 209 | 461 | 427 |
| `nist/NIST-PMI-STEP-Files/AP203 geometry only/nist_ftc_06_asme1_rd.stp` | 1 | 144 | 310 | 310 |
| `nist/NIST-PMI-STEP-Files/AP203 geometry only/nist_ftc_07_asme1_rd.stp` | 1 | 269 | 583 | 511 |
| `nist/NIST-PMI-STEP-Files/AP203 geometry only/nist_ftc_08_asme1_rc.stp` | 1 | 270 | 685 | 685 |
| `nist/NIST-PMI-STEP-Files/AP203 geometry only/nist_ftc_09_asme1_rd.stp` | 1 | 158 | 421 | 421 |
| `nist/NIST-PMI-STEP-Files/AP203 geometry only/nist_ftc_10_asme1_rb.stp` | 1 | 214 | 439 | 435 |
| `nist/NIST-PMI-STEP-Files/AP203 geometry only/nist_ftc_11_asme1_rb.stp` | 1 | 6 | 6 | 6 |
| `nist/NIST-PMI-STEP-Files/AP203 with PMI/nist_ctc_01_asme1_ap203.stp` | 1 | 139 | 370 | 370 |
| `nist/NIST-PMI-STEP-Files/AP203 with PMI/nist_ctc_02_asme1_ap203.stp` | 1 | 487 | 926 | 828 |
| `nist/NIST-PMI-STEP-Files/AP203 with PMI/nist_ctc_03_asme1_ap203.stp` | 1 | 139 | 390 | 390 |
| `nist/NIST-PMI-STEP-Files/AP203 with PMI/nist_ctc_04_asme1_ap203.stp` | 2 | 406 | 830 | 830 |
| `nist/NIST-PMI-STEP-Files/AP203 with PMI/nist_ctc_05_asme1_ap203.stp` | 1 | 156 | 329 | 309 |
| `nist/NIST-PMI-STEP-Files/nist_ctc_01_asme1_ap242-e1.stp` | 1 | 117 | 306 | 306 |
| `nist/NIST-PMI-STEP-Files/nist_ctc_02_asme1_ap242-e2.stp` | 3 | 637 | 1,224 | 1,116 |
| `nist/NIST-PMI-STEP-Files/nist_ctc_03_asme1_ap242-e2.stp` | 1 | 120 | 339 | 339 |
| `nist/NIST-PMI-STEP-Files/nist_ctc_04_asme1_ap242-e1.stp` | 2 | 484 | 996 | 996 |
| `nist/NIST-PMI-STEP-Files/nist_ctc_05_asme1_ap242-e1.stp` | 1 | 156 | 329 | 309 |
| `nist/NIST-PMI-STEP-Files/nist_ftc_06_asme1_ap242-e2.stp` | 1 | 187 | 428 | 428 |
| `nist/NIST-PMI-STEP-Files/nist_ftc_07_asme1_ap242-e2.stp` | 3 | 306 | 672 | 600 |
| `nist/NIST-PMI-STEP-Files/nist_ftc_08_asme1_ap242-e1-tg.stp` | 0 | 0 | 0 | 0 |
| `nist/NIST-PMI-STEP-Files/nist_ftc_08_asme1_ap242-e2.stp` | 1 | 247 | 615 | 615 |
| `nist/NIST-PMI-STEP-Files/nist_ftc_09_asme1_ap242-e1.stp` | 6 | 163 | 421 | 421 |
| `nist/NIST-PMI-STEP-Files/nist_ftc_10_asme1_ap242-e2.stp` | 13 | 282 | 579 | 575 |
| `nist/NIST-PMI-STEP-Files/nist_ftc_11_asme1_ap242-e2.stp` | 1 | 42 | 104 | 104 |
| `nist/NIST-PMI-STEP-Files/nist_stc_06_asme1_ap242-e3.stp` | 1 | 144 | 310 | 310 |
| `nist/NIST-PMI-STEP-Files/nist_stc_07_asme1_ap242-e3.stp` | 3 | 306 | 672 | 600 |
| `nist/NIST-PMI-STEP-Files/nist_stc_08_asme1_ap242-e3.stp` | 2 | 271 | 685 | 685 |
| `nist/NIST-PMI-STEP-Files/nist_stc_09_asme1_ap242-e3.stp` | 1 | 125 | 326 | 326 |
| `nist/NIST-PMI-STEP-Files/nist_stc_10_asme1_ap242-e2.stp` | 1 | 256 | 551 | 547 |

`nist_ftc_08_asme1_ap242-e1-tg.stp` is the AP242 *tessellated-surface* variant
(NIST README: "the part geometry uses tessellated (faceted) surfaces instead
of exact b-rep geometry"). It declares no shells, so it contributes 0 B-rep
faces and 0 pairs legitimately; it is measured, not excluded.

## Method notes

**Loader.** Every file is read through the same landed import path the
renderer uses (`src/step.rs`): the Part-21 reader `look::step::part21::parse`
with the `ruststep::parser::parse` fallback, `Table::from_owned_data_section`,
then `Table::to_compressed_shell_with_losses` per shell — the identical parse
and conversion `parse_step_table` runs, stopped before tessellation so the
per-face support surfaces the conversion leaves on each compressed face can be
read. No second parser route exists. Shells are censused in their compressed
form, the exact form production tessellates from; the editable-shell round trip
(`Shell::extract`) is deliberately not introduced because it refuses degenerate
seam edges the renderer accepts on 22 of the 38 real corpus files (e.g.
`shell #97554: Two same vertices cannot construct an edge.`). Adjacency is
`Shell::face_adjacency()`'s relation — two faces sharing an edge — computed on
the compressed edge indices; a pair sharing more than one edge contributes one
row, and each unordered pair contributes exactly one row.

The corpus's own conformance quirks surface as per-entity `Error while
deserialize STEP struct: ...` lines on stderr from the landed table builder
(the NIST README states these files are "NOT reference STEP files without any
errors"). They do not exclude any file: all 38 loaded and produced shells.

**Classifier.** One face's support surface, in the Phase-1 fast-path dispatch
order, recording only the FIRST accepting class:

1. `Plane` — `identify_plane` accepts it (via the landed `support_schema_of`;
   the schema carries authority, not the `Unresolved`/failure arms).
2. `Cylinder` — a `RevolutedCurve<Line<Point3>>`-shaped support that
   `identify_cylinder` certifies (via the landed
   `identify_source_cylinder_opt`).
3. `Cone` — the same representation shape, `identify_cone` certifies it (via
   the landed `identify_source_cone_opt`).
4. `Torus` — a `Torus` whose world-space parameters `identify_torus_world`
   certifies (via the landed `identify_source_torus_opt`).
5. `Sphere` — **no landed certified constructor exists**; classified by the
   landed in-memory representation (a sphere-carried surface). Its evidence is
   representation-named, not certified-identified — this is itself a Phase-1
   gap finding, published here.
6. `Spline` — B-spline / rational B-spline (Bézier) carried.
7. `Other(tag)` — everything else, tagged by the representation's own name
   (the `NoStructuralReader` doctrine: record what the representation says it
   is, never guess "probably a plane").

The certified constructors are REFUSING constructors; a refusal is the
classifier saying "not this class", which is exactly the dispatch order the
Phase-1 fast path will run. The residual `Other` bucket is fully named (see
above), so no unexplained residual remains.

**What this is not.** This is a measurement, not a pass/fail gate. The census
test asserts only structural sanity (corpus found, every measured shell-bearing
file has faces > 0, every face lands in exactly one of the seven buckets) and
deliberately carries no threshold assertion on the analytic fraction — a
threshold would make the measurement self-fulfilling.

## Reproduction

```console
$env:LOOK_CORPUS = "C:\Users\stefa\look-corpus"
cargo test -p look --test certified_prevalence -- --ignored --nocapture
```

The test prints one JSON row per file (`{"file", "shells", "faces", "pairs",
"analytic_pairs", "analytic_faces", "face_histogram", "pair_histogram",
"other_tags"}`) plus the `CERTIFIED_PREVALENCE_AGGREGATE` line carrying the
headline exact counts. The numbers above were taken verbatim from that output.
The test skips with a clear message when `LOOK_CORPUS` is unset.
