# WORK PACKET BG-SOL-SPLIT-PERIODIC - the periodic-branch region-check fix

The landed flagship test `split_flagship_top_face_by_ff_circle` FAILS at
HEAD (`cargo test -p truck-shapeops --lib boolean` — verified by the
orchestrator before this packet was written; it panics with
`UnsupportedEnvelope(ContactReductionDeferred)` at split.rs:2211). The
cause is a periodic-branch defect in `split.rs`'s region checks, exposed
when BG-SOL-S2-DISK-ORIENT (which landed in PARALLEL with RW2) changed
the extruded wall's top-wire traversal direction. This packet fixes the
defect; it is a prerequisite for BG-SOL-RW3-CLASSIFY (whose flagship
witness constructs this exact mesh with the full event set). If live
code contradicts this packet, report it in `disagreements`.

```json
{"id":"BG-SOL-SPLIT-PERIODIC","status":"DONE","contracts":["BG-SOL-SPLIT-PERIODIC"],
 "tests_added":1,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],"notes":"free text"}
```

```yaml
id:          BG-SOL-SPLIT-PERIODIC
contract:    [BG-SOL-SPLIT-PERIODIC]
class:       design
crates:      [truck-shapeops]
write_allow:
  - vendor/truck/truck-shapeops/src/boolean/split.rs
read_allow:
  - vendor/truck/truck-modeling/src/extrude.rs
  - docs/SOLVER_FAMILY_PLAN.md
tests_required:
  - split_ff_only_circle_skips_the_on_boundary_wall
budget:      {turns: 16, ctx_tokens: 80000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'fn on_face_boundary' vendor/truck/truck-shapeops/src/boolean/split.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'fn region_contains' vendor/truck/truck-shapeops/src/boolean/split.rs"}
  - {id: A3, expect: 6, cmd: "grep -cF '#[test]' vendor/truck/truck-shapeops/src/boolean/split.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub fn split_fragments' vendor/truck/truck-shapeops/src/boolean/split.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'fn classify_curve' vendor/truck/truck-shapeops/src/boolean/split.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'fn fragment_covering' vendor/truck/truck-shapeops/src/boolean/split.rs"}
```

A3 becomes 7 (the new test). All others stay (signatures change but the
`fn` names do not).

## Problem

`classify_curve` folds a candidate curve's sample parameters onto the
principal u-branch (`uv.x = uv.x.rem_euclid(period)`, split.rs:902) and
compares them against the face's boundary-wire parameter polygons. Those
polygons are built by `create_parameter_boundary`, which UNWRAPS u
continuously from each wire's own front vertex — so a boundary wire
traversed the other way around lives on a DIFFERENT branch than the
folded samples, and the 2-D point/segment comparisons cannot see the
coincidence even though the 3-D geometry is identical.

Concretely: after the S2 fix, the extruded outer-cycle wall stores its
top wire as `[te.inverse()]`, whose polygon spans u from 0 down to
−2π at v = height. The flagship's FF circle at z = height folds to
u ∈ [0, 2π). Every sample of the circle lies ON the wall's top rim, but
the 2-D distance from (u, height) to the segment [−2π, 0] × {height}
reads > tol for almost every sample. The sample loop therefore sees a
mix of on-boundary and outside samples, classifies the relation as
`Crossing` instead of `OnBoundary`, and `insert_clipped_arc` refuses
(no Point events certify the "crossings" — correctly, because there is
no crossing). Before the S2 fix the wall's top wire was `[te]` (u from
0 up to +2π) and the folded samples landed on it by luck.

The same branch disease exists wherever a query parameter produced in
one frame is compared against polygons unwrapped in another: the
`region_contains` calls in `classify_curve`'s inside/outside tests, and
`fragment_covering`'s containment test (its `uv` comes from
`search_parameter`, which returns principal-branch values).

## Decisions already made

### 1. The fix: periodic-aware region checks, ± period translates

- `fn region_contains(polys, p, u_period: Option<f64>) -> bool` — the
  point is inside iff it is inside at `p`, or (when `u_period` is
  `Some(T)`) at `Point2::new(p.x + T, p.y)` or
  `Point2::new(p.x - T, p.y)`. The outer-polygon and hole tests all
  apply to the same translate (translate the QUERY, not the polygons).
- `SplitEngine::on_face_boundary(&self, face_polys, uv, u_period)` —
  on-boundary iff the boundary-segment distance test passes at `uv` or
  at `uv.x ± period` (same three translates; only the u coordinate
  shifts).

Thread the period from the face at hand:

- `classify_curve` already computes `let u_period = surface.u_period();`
  (split.rs:888). Pass it to every `on_face_boundary` and
  `region_contains` call inside that function (the sample loop AND the
  consecutive-sample transition loop).
- `fragment_covering` reads `face.surface().u_period()` and passes it to
  its `region_contains` call.
- `region_representative` (its `region_contains` call at split.rs:1840)
  and `containment_screen` (split.rs:1863–1874) pass `None`: the
  representative's candidates are derived from the SAME polygons they
  are tested against (no frame mismatch), and the Region2 screen's
  two-face periodic case (coaxial coincident cylinders) is outside this
  packet's scope — record it as a known limitation in `notes`, it is the
  RW-COPLANAR follow-up family's concern.

### 2. Why three translates are enough

The boundary wires of a face in this envelope wind at most once around
the periodic axis (simple wires; a wire polygon spans at most one full
period). A query point and its ± period neighbors therefore cover every
branch on which a segment of the polygon can coincide with the query's
3-D locus. A point that was already frame-consistent gains nothing (its
± period translates land strictly outside sub-period-sized regions) —
the change is conservative: it can only turn false outside/off-boundary
answers into true ones where the 3-D geometry says they must be true.

### 3. What must NOT change

- `create_parameter_boundary` and `unwrap_periodic_parameter` — the
  unwrapping itself is correct; do not touch it.
- `classify_curve`'s `rem_euclid` folding of samples — the folding is
  fine once the checks are branch-aware.
- The split semantics, the sewing oracle, the Region2 screen's logic,
  `is_full_period_circle`, every refusal — untouched.
- The pre-existing test bodies: `split_flagship_top_face_by_ff_circle`
  is the regression test and its assertions stay EXACTLY as landed (it
  must pass again — that is the point of this packet). Do not rename,
  retarget, or "fix" it.

### 4. Expected behavior changes (machine-check both)

- The flagship (full event set `[ff, fe, r2]`) stops refusing: the FF
  circle reads `OnBoundary` against the wall (every sample on the top
  rim, in the −2π translate) so the wall is skipped by the FF pass, the
  FE sewing and the Region2 screen proceed exactly as the landed test
  asserts (10 fragments, 17 adjacency entries, 2 Flip, 1 Identical
  coincident pair — the landed assertions).
- `split_open_arc_uses_point_events_for_trimming`: the two generator
  lines against b's wall now have BOTH their v=0 and v=2 samples
  on-boundary (the v=2 samples via the −2π translate), so
  `insert_clipped_arc` receives both certified endpoints and inserts
  the FULL lines into the wall (the wall splits into two half-band
  fragments with 2 Flip adjacencies). At HEAD the same test passes via
  a degenerate single-crossing clip. The test's existing assertions
  (3 side fragments, 2 a-side Flip, 4 total Flip, Same entries, no
  cross-solid adjacency) hold under BOTH behaviors — verify they still
  hold after the change and record in `notes` which b-side fragment
  count the new behavior produces (expected: the wall becomes 2
  fragments).

## Tests required

1. `split_ff_only_circle_skips_the_on_boundary_wall` (NEW): a = the
   4×4 block extrude (height 2), b = the disk extrude at (2,2) r=1
   (height 2) — the flagship inputs. Events: ONLY the FF record
   `{Arc1, Transverse, Analytic(Curve(<the r=1 circle at (2,2,2)>))}`
   between a's top face and b's cylinder wall (the flagship test's
   `ff` event, nothing else). Expected, derived: the circle is `Inside`
   a's top face (it becomes the doubled independent loop: a's top face
   → 2 fragments, the disk and the annulus) and `OnBoundary` the wall
   (skipped — b's wall is NOT split, stays 1 fragment with its two
   original self-loop wires). Total fragments: a: 1 bottom + 2 top + 4
   sides = 7; b: 3 — NINE fragments. Adjacency: 2 Flip (disk ↔ annulus,
   the fresh circle halves) + a's 12 Same + b's 3 Same = 17 entries.
   `coincident` is EMPTY (no Region2 event). The two circle half-edge
   instances appear in a's disk and annulus fragments but NOT in b's
   wall or cap wires (no sewing event). Assert all of these counts.

Machine-check every count above against your own derivation from the
construction before asserting it (the BG-NUM-002 rule) and record any
discrepancy with both derivations in `deviations`.

## House form (H-3)

This crate is under the kernel's house rules. Any ADDED line with a
bare `1e-N` float literal must end `// H-3`; prefer dyadic values,
`TAU`/`std::f64::consts`, or named constants. Run
`bash scripts/kernel-gates.sh <your base commit>` before writing
RESULT.json - a failing gate is a finding to report, never one to work
around.

## Done when

```console
cargo fmt --check -p truck-shapeops
cargo clippy -p truck-shapeops --all-targets --no-deps
cargo check --locked -p truck-shapeops --all-targets
cargo test -p truck-shapeops --lib boolean --no-fail-fast
bash scripts/kernel-gates.sh <your base commit>
```

All NINE boolean tests must pass — the currently-failing flagship test
turning green is this packet's acceptance criterion. Never run bare
`cargo test` or a workspace-wide cargo command.

**Commit your work on the current branch** (subject
`shapeops: periodic-branch-aware region checks in the splitter (BG-SOL-SPLIT-PERIODIC)`)
**before** writing `RESULT.json`: the verifier measures the committed
diff, and an uncommitted tree reads as an interrupted run.

## Forbidden

Editing anything outside `write_allow`; changing
`create_parameter_boundary` or `unwrap_periodic_parameter`; altering
any pre-existing test's name or assertions; changing the Region2
screen's behavior; adding a `_` wildcard arm anywhere; `#[ignore]`;
loosening a gate; changing the GATE-4 ceiling; widening `tol`.

## Stop conditions

- anchor mismatch -> `ANCHOR_MISMATCH` with observed count;
- the flagship test still fails after the fix as specified -> `SPEC_GAP`
  with your diagnosis (do NOT improvise a different fix silently);
- `split_open_arc_uses_point_events_for_trimming` regresses under the
  new behavior and cannot be satisfied without touching its assertions
  -> `SPEC_GAP` with the observed counts;
- three consecutive cargo failures with one cause -> `BLOCKED`.

Finish by writing `RESULT.json` in the worktree root, not `loop/results/`.
Record in `notes`: the observed post-fix fragment counts for the
open-arc test's b-side wall, whether any other test's behavior shifted,
and the known-limitation note for `containment_screen`'s periodic case.
