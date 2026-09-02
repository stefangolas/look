# BG-CK-SPLINE-CENSUS — spline-bucket structural census (measurement-only)

The booking's fourth input gate (`docs/CERTIFIED_PHASE2_BOOKING.md`
amendment, owner direction; the interleave is decided in
`docs/CERTIFIED_INTERLEAVE_BUILD_SPEC.md`): extend the Phase-0 prevalence
harness with a MEASUREMENT-ONLY decomposition of every spline-carried face
in the corpus — control-net reads that name which splines are canonical
geometry in disguise. No recognizer is built, no vendor code changes, no
threshold is asserted. Either census outcome is a win (booking, verbatim):
a big fast-path win (a recognizer family with measured mass) or a
certified greenlight for Phase 2's generic engine with the residual
quantified.

This is the PREVALENCE packet's shape exactly: load every `.step`/`.stp`
under `LOOK_CORPUS` through the same landed import path, classify, print
JSON rows + aggregate headline so the doc's numbers are copy-out
reproducible. MEASUREMENT test, not a pass/fail gate.

```yaml
id:          BG-CK-SPLINE-CENSUS
contract:    [BG-CK-SPLINE-CENSUS]
class:       mechanical
crates:      [look]
depends_on:  [BG-CK-P0-PREVALENCE]
write_allow:
  - tests/certified_spline_census.rs
  - docs/CERTIFIED_SPLINE_CENSUS.md
read_allow:
  - docs/CERTIFIED_PHASE2_BOOKING.md
  - docs/CERTIFIED_INTERLEAVE_BUILD_SPEC.md
  - tests/certified_prevalence.rs
  - docs/CERTIFIED_PREVALENCE.md
  - vendor/truck/truck-stepio/src/in/mod.rs
  - vendor/truck/truck-stepio/src/in/convert.rs
budget:      {turns: 30, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 7, cmd: "grep -c 'LOOK_CORPUS' tests/certified_prevalence.rs"}
  - {id: A2, expect: 2, cmd: "grep -c 'BSplineSurface' tests/certified_prevalence.rs"}
  - {id: A3, expect: 2, cmd: "grep -c 'NurbsSurface' tests/certified_prevalence.rs"}
  - {id: A4, expect: 2, cmd: "grep -c '21,004' docs/CERTIFIED_PHASE2_BOOKING.md"}
  - {id: A5, expect: 0, cmd: "ls docs/CERTIFIED_SPLINE_CENSUS.md 2>/dev/null | wc -l"}
  - {id: A6, expect: 0, cmd: "ls tests/certified_spline_census.rs 2>/dev/null | wc -l"}
tests_required:
  - synthetic_nets_classify_to_their_constructed_bucket
  - every_spline_face_lands_in_exactly_one_headline_bucket
  - degree_histogram_counts_every_measured_face
  - census_skips_cleanly_without_look_corpus
```

## The measurement (pre-made definitions; the worker machine-checks them)

For every face whose support surface is `Surface::BSplineSurface(_)` or
`Surface::NurbsSurface(_)` (the landed `classify` step 6, byte-compatible
with the prevalence census), read the control net — degrees `(p_u, p_v)`,
the control-point grid, and the weights when rational — and classify the
net into the FIRST matching bucket of this priority order (headline
buckets are mutually exclusive by construction):

1. **bilinear** — `p_u <= 1 && p_v <= 1`.
2. **planar_net** — all control points (weights applied as `p_i / w_i`
   when rational) are coplanar: the plane through the first three
   distinct net points, every remaining point's signed distance within
   the harness tolerance.
3. **circular_row_rational** — rational net where every row (constant-v
   cross-section) of weighted control points lies on one circle (circle
   from the row's first three distinct points; all residuals within
   tolerance). Sub-flags recorded, not bucket-splitting: rows-congruent,
   rows-coaxial (per-row circle centers collinear on one axis).
4. **revolution_structured** — some parameter direction's cross-sections
   are rigid rotations of the first cross-section about one fixed axis
   (within tolerance), non-circular profiles included. Both grid
   directions are tried; the direction that matches is recorded.
5. **extrusion_structured** — some parameter direction's cross-sections
   are translations of the first cross-section (within tolerance). Both
   directions tried, direction recorded.
6. **general** — none of the above.

Row/column orientation is part of the worker's job to fix mechanically:
the net is an `(n_u+1) x (n_v+1)` grid; "cross-section" and "row" above
mean whichever constant-index slice makes the definition well-formed, and
the classifier tries both directions where the definition allows it.

**Tolerances are harness constants, published in the doc.** The headline
table reports each bucket's count at the primary tolerance AND at 10x the
primary tolerance (one sensitivity column — the recognizer-family decision
must know whether a count is tolerance-fragile). This is a measurement
harness, not a certified predicate: no threshold assertion in-tree
(census discipline), every tolerance value printed in the doc beside the
counts it produced.

**Degenerate guard:** a net with fewer than three distinct points in a
slice cannot define the plane/circle for that definition; such slices make
the bucket definition NOT MATCH (fall through), and the fall-through is
counted, not hidden — the doc reports how many faces fell through each
definition for degeneracy rather than geometry.

## Section 1 — `tests/certified_spline_census.rs` (NEW)

- Same loader path as the prevalence census (`src/step.rs` import chain;
  read the landed file, do not re-derive it — the corpus quirks it
  handles are load-bearing).
- Same output discipline: JSON rows per file, aggregate headline, all
  copy-out reproducible.
- The four `tests_required` functions:
  1. `synthetic_nets_classify_to_their_constructed_bucket` — build tiny
     known nets in code (a bilinear net, a planar net, a rational net
     whose rows are circular arcs — a cylinder in disguise —, a
     revolution-structured net, an extrusion-structured net, and one
     general net) and assert the classifier names each. This is the
     MAP packet's tensor-commutation discipline: the definitions are
     machine-checked against constructed ground truth, not trusted.
  2. `every_spline_face_lands_in_exactly_one_headline_bucket` — the
     corpus run's structural sanity (priority order makes this true by
     construction; the test asserts it on the measured data).
  3. `degree_histogram_counts_every_measured_face` — histogram total
     equals spline-face total.
  4. `census_skips_cleanly_without_look_corpus` — the prevalence skip
     shape (clear message, no failure) when the env var is unset.
- No threshold assertion on any fraction. No `unwrap` (crate denies it).

## Section 2 — `docs/CERTIFIED_SPLINE_CENSUS.md` (NEW)

Published table: per-bucket face counts (primary tol + 10x column),
degree histogram, per-file rows for any file with spline mass, the
tolerance values used, the degeneracy fall-through counts, and a context
paragraph quoting the booking's spline pair masses (spline~spline 21,004;
plane~spline 16,137; cylinder~spline 15,566; cone~spline 3,808;
spline~torus 3,923) LABELED as pair masses from the booking, not measured
here (this census counts faces; the pair masses are the booking's).
Close with the decision framing only — what each outcome would mean —
and make NO recommendation; the recognizer-family decision reads this doc
after the wave.

## House rules

- H-1: the root crate's lint profile covers the new test file; no
  `unwrap`/`expect`/`panic!` in it.
- H-3 opt-outs same-line if any are needed.
- `cargo fmt` clean on the new files; `cargo clippy -p look --test
  certified_spline_census --message-format=short --no-deps` zero findings
  on the new file.
- No manifest change, no vendor change, no production-code change. The
  census file `tests/certified_prevalence.rs` stays byte-identical
  (V5-guarded).
- Worker checks scoped: `cargo check -p look` + the new test target only.

## Done-when

- The four `tests_required` functions exist and pass locally with
  `LOOK_CORPUS` set to the corpus path; the corpus run completes over all
  38 STEP files and prints the headline.
- The doc's tables are copy-out from the run's printed JSON/summary (the
  numbers in the doc and the run output agree — state the run command in
  the doc).
- fmt + scoped clippy clean; no threshold assertions in-tree.

## Stop conditions

Stop, commit nothing beyond WIP evidence, write RESULT.json (AT THE
WORKTREE ROOT) with the finding verbatim if:

1. A spline face's control net (degrees, grid, weights) is not readable
   through the landed representation — record the exact type and the
   missing accessor; do NOT widen the write set into vendor.
2. A bucket definition cannot be made machine-checkable by control-net
   read alone (it would need fitting beyond the stated construction —
   the Caution B line) — say which bucket and what the obstruction is;
   the bucket list is frozen.
3. The loader refuses a file the prevalence census measured — record
   file + error verbatim; do not fix the loader.

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

Commit your work on the current branch (subject: `feat(census): spline-bucket
structural census (BG-CK-SPLINE-CENSUS)`) BEFORE writing `RESULT.json`.
