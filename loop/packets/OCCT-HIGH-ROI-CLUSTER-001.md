# WORK PACKET OCCT-HIGH-ROI-CLUSTER-001 — the cheap high-yield OCCT ingestion cluster

You are implementing one packet of the Certified Interaction Engine (BIE)
program wave. The write set is fully disjoint from every BIE packet (they
write `truck-certified/**` and `truck-geometry/src/span.rs`); everything here
is `truck-stepio` only. Everything you need is in this document and
`docs/BIE_BUILD_SPINE.md`. Do not read other spec files. If something you
need is genuinely missing, that is a SPEC_GAP (see "Stop conditions"): you
stop and report, you do not research it.

```yaml
id:          OCCT-HIGH-ROI-CLUSTER-001
contract:    [OCCT-HIGH-ROI-CLUSTER-001]
class:       mechanical
crates:      [truck-stepio]
depends_on:  []
write_allow:
  - vendor/truck/truck-stepio/src/in/mod.rs
  - vendor/truck/truck-stepio/tests/occt_high_roi_cluster_001.rs
read_allow:
  - vendor/truck/truck-stepio/src/in/convert.rs
  - vendor/truck/truck-stepio/tests/input/
  - vendor/truck/truck-meshalgo/src/tessellation/triangulation.rs
  - docs/defects/SEM-PCURVE-MASTER-001.md
  - docs/BIE_BUILD_SPINE.md
tests_required:
  - sem_pcurve_master_001_pcurve_s1_uses_declared_3d_curve
  - sem_pcurve_master_001_pcurve_s2_uses_declared_3d_curve
  - sem_pcurve_master_001_broken_curve_3d_refuses
  - sem_pcurve_master_001_seam_crossing_extent_reconciles
  - occt_high_roi_cluster_001_trimmed_curve_parameter_trim
  - occt_high_roi_cluster_001_trimmed_curve_point_trim
  - occt_high_roi_cluster_001_trimmed_curve_dual_trim_disagreement_refuses
  - occt_high_roi_cluster_001_trimmed_curve_line_parameter_scaling
  - occt_high_roi_cluster_001_pcurve_fallback_reconciles
  - occt_high_roi_cluster_001_pcurve_fallback_rejects_wrong_branch
  - occt_high_roi_cluster_001_void_solid_bookkeeping
budget:      {turns: 80, ctx_tokens: 180000}
```

**New file** (`tests/occt_high_roi_cluster_001.rs`): H-1 applies — no
`unwrap_used` without a justified same-line opt-out. It is a NEW test path;
no landed test file may be touched. Build STEP inputs as in-test strings
following the landed `tests/input/` recipe module.

**Read-only context, stated explicitly**: the `read_allow` entry
`vendor/truck/truck-stepio/tests/input/` (the landed STEP-string recipe
module) is a LANDED file tree you must READ for the test-input recipe and
must never clobber — it is not in `write_allow` and any write to it is a V1
rejection.

This packet **absorbs and supersedes `SEM-PCURVE-MASTER-001-FIX`** (owner
directive: one packet for the high-ROI cluster). Its four tests and its
correction are items C1 below, unchanged in substance; its anchors are A1–A6
here.

## Problems (three sources, one cluster)

1. **`docs/defects/SEM-PCURVE-MASTER-001.md`** — a `SURFACE_CURVE` with
   `.PCURVE_S1.`/`.PCURVE_S2.` mastery discards its mandatory `curve_3d` and
   substitutes a pcurve whose 2D geometry is re-derived from
   principal-branch-only vertex anchors; a seam-crossing trim folds, the edge
   no longer reconciles with its vertices, and every face referencing it
   refuses `EdgeTraversalUnresolved` (GitHub issue #1: 2 of 24 faces drop;
   the `write_pcurves=False` twin of the identical solid renders clean).
2. **Entity gap** — `TRIMMED_CURVE` is legal STEP edge geometry
   (`EDGE_CURVE.geometry` may reference it) but the importer has no holder,
   no dispatch arm, and no conversion: an edge referencing one fails
   conversion and the face is lost. Rare in OCCT manifold solids (measured:
   zero in `core_xy.step`) but routine in boolean/wireframe output.
3. **Refusal context** — an edge whose converted curve does not reconcile
   with its vertex positions fails far downstream in `truck-meshalgo`
   (`EdgeTraversalUnresolved`) with no residual evidence at the layer that
   had both the curve and the vertices in hand.

## The corrections — PRE-DECIDED, do not relitigate

### C1 — honor the declared 3D curve over pcurve mastery (defect record, correction 1)

In `sub_parse_curve3d`'s `CurveAny::SurfaceCurve(c)` arm:

- The existing `ctx.near_pt(p, q)` BG-TOL-001 early arm already routes
  through `c.curve_3d` and STAYS as is.
- The `Curve3D =>`, `PcurveS1 =>`, and `PcurveS2 =>` match arms are REPLACED
  by the single unconditional call
  `Self::sub_parse_curve3d(&c.curve_3d, p, q, same_sense)?`.
- **No pcurve fallback on failure here.** If `curve_3d` parsing errors, the
  error propagates. (The controlled fallback is C2, a separate mechanism
  with its own gate.)
- `PreferredSurfaceCurveRepresentation` STAYS on the holder struct — it is
  part of the entity schema and its `Deserialize` surface; it is simply no
  longer branched on here. Do not delete the enum.
- **Leave A6's site (`impl TryFrom<&SurfaceCurve> for Curve3D`) untouched** —
  it is outside the defect's causal chain. State in your RESULT notes
  whether that `TryFrom` path is reachable from face-trim edge ingestion
  (follow the callers read-only); if it is, that is a `SPEC_GAP`.

### C2 — the pcurve realization as a gated fallback (defect record, correction 2, inverted)

The record's correction (2) said "retry from `curve_3d`" — meaningless once
C1 lands. The useful inversion, pre-decided: **the pcurve realizations
become the fallback, under a reconciliation gate.** In
`EdgeCurveHolder::parse_curve3d` (`vendor/truck/truck-stepio/src/in/mod.rs`):

1. Convert the edge curve as today (post-C1 this is always the declared 3D
   curve).
2. **Reconcile**: evaluate the returned curve at its two range endpoints and
   check both vertex positions `p`, `q` (the EDGE_CURVE's start/end, already
   in hand) are covered — compute `d(front,p) + d(back,q)` and
   `d(front,q) + d(back,p)`, take the smaller total, and require BOTH
   residuals in the better assignment ≤ the context tolerance. Use the
   landed `ToleranceCtx` (the file already constructs
   `ToleranceCtx::unscaled_legacy()` in every parse path); the predicate is
   the ctx's Euclidean point predicate (`near_pt` semantics), never a bare
   literal (H-3).
3. If the primary reconciles, return it — **behavior is unchanged for every
   edge that converts correctly today.** The gate only ever swaps a failing
   realization.
4. If it does not reconcile and `edge_geometry` is a `SurfaceCurve`, try the
   associated pcurves in order S1 then S2, via the existing
   `Self::sub_parse_curve3d(&CurveAny::Pcurve(pc.clone()), p, q, true)` arm,
   reconciling each candidate by step 2's predicate. Accept the FIRST
   candidate that reconciles at BOTH ends; never average residuals, never
   pick the smaller failure.
5. If nothing reconciles, return the ORIGINAL conversion error (or, if the
   original succeeded but failed reconciliation, a StepConvertingError whose
   message carries both endpoint residuals in the better assignment and
   which realization produced them). Honest refusal with evidence — this is
   what the downstream `EdgeTraversalUnresolved` path lacks today.

Why this is safe: a folded-anchor pcurve (the issue #1 mechanism) fails the
gate by construction — its endpoint sits on the wrong side of the seam, at a
residual on the order of the model diameter. A correct pcurve reconciles
orders of magnitude below any plausible tolerance. The gate cannot accept
wrong geometry; it can only rescue correct geometry that the 3D-curve
conversion mis-anchored.

### C3 — realize `TRIMMED_CURVE` edge geometry

Follow the landed `POLYLINE` wiring exactly — four sites, all in `mod.rs`:

1. **Table field**: `pub trimmed_curve: HashMap<u64, TrimmedCurveHolder>`
   beside `pub polyline`.
2. **Dispatch arm**: `"TRIMMED_CURVE" => { self.trimmed_curve.insert(*id,
   Deserialize::deserialize(record)?); }` beside the `"POLYLINE" =>` arm.
3. **`BoundedCurveAny` variant**: `TrimmedCurve(Box<TrimmedCurve>)` beside
   the `Polyline` variant, same `#[holder(...)]` attributes.
4. **Conversion**: in `sub_parse_curve3d`'s `BoundedCurve` arm (and, if the
   enum plumbing requires it, in the `BoundedCurveAny → Curve2D/Curve3D`
   `TryFrom`s — mirror wherever `Polyline` is handled), realize:

The entity is `TRIMMED_CURVE(name, basis_curve, trim1, trim2, sense,
master)` where each trim selects a `CARTESIAN_POINT` and/or a
`PARAMETER_VALUE` and `master` is `.UNSPECIFIED.` / `.PARAMETER.` /
`.CARTESIAN_POINT.`. Realization algorithm, pre-decided:

- Convert the basis curve by recursing into `sub_parse_curve3d` (the basis
  may be any `curve`, including conics and `POLYLINE`).
- **Trim selection**: if `master` is `.PARAMETER.`, use the parameter
  selects; if `.CARTESIAN_POINT.`, the point selects; if `.UNSPECIFIED.`,
  use whichever pair is present, and if BOTH are present that is the duality
  case of the next bullet. If a required select is missing for the chosen
  master, that is a typed conversion error.
- **Trim duality certificate**: when a trim carries both a point and a
  parameter, the point's solved parameter (via the basis curve's
  parameter-search, residual ≤ the ctx tolerance — `GEO-006`: a distant
  nearest point is not a valid inverse) must agree with the declared
  parameter within the ctx tolerance. Disagreement is a typed refusal
  (`StepConvertingError` naming the entity and the two readings) — never an
  average, never a silent pick.
- **Parameter semantics** (the `PAR-RANGE-INHERITANCE` trap, pre-empted):
  STEP parameters live in the basis curve's OWN parameterization. Conics:
  radians on the unit conic — identical to truck's realization, use
  directly. `LINE`: STEP parameterizes by distance along the unit direction,
  truck's `Line` by 0..=1 over `[pnt, pnt+dir]` — **re-anchor**: construct a
  fresh truck `Line` from `pnt + dir·t₀` to `pnt + dir·t₁`. Never pass a
  STEP parameter into a truck `Line`'s 0..=1 range unscaled.
- **Periodic lift**: if the basis is periodic and `t₁ < t₀ − ctx.margin`,
  add one period to `t₁` (the landed conic wrap rule). If `|t₁ − t₀| >`
  one period, refuse — a multi-period trim is legal STEP but not
  realizable here; honest refusal, not a modulo.
- **Sense**: `sense = false` inverts the resulting curve
  (`curve.invert()`), matching the file's existing convention.
- Realize the trimmed extent with the landed `TrimmedCurve::new(basis,
  (t0, t1))` decorator (16 uses of that constructor exist in this file —
  copy the nearest conic call site's shape).

**Out of scope, explicitly**: `COMPOSITE_CURVE`, `OFFSET_CURVE_3D`,
`CURVE_BOUNDED_SURFACE`, `MAPPED_ITEM` assemblies, and every
`truck-meshalgo` change. Do not touch them.

### C4 — `BREP_WITH_VOIDS` bookkeeping oracle (test only, no production change)

Void shells already convert (`convert.rs` loops `solid.voids`; the landed
doc-comment at its "outer shell first, then each void shell" site states the
ordering). The deliverable is the regression test `occt_high_roi_cluster_001_void_solid_bookkeeping`
pinning that behavior: a synthetic solid with one exterior shell and one
void shell converts with both shells present, in that order, and the face
count is the sum of both shells' faces. If the production behavior does not
match that oracle, STOP — `SPEC_GAP` naming what you measured, and write no
production change (that would be a different packet).

## Anchors — measured 2026-09-05 against the tree, counts are exact

Locate by pattern, never by line number. If a count differs, STOP and report
`ANCHOR_MISMATCH` with what you saw.

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `vendor/truck/truck-stepio/src/in/mod.rs` | `fn sub_parse_curve3d` | 1 |
| A2 | `vendor/truck/truck-stepio/src/in/mod.rs` | `Curve3D => Self::sub_parse_curve3d\(&c\.curve_3d` | 1 |
| A3 | `vendor/truck/truck-stepio/src/in/mod.rs` | `PcurveS1 =>` | 2 |
| A4 | `vendor/truck/truck-stepio/src/in/mod.rs` | `PcurveS2 =>` | 2 |
| A5 | `vendor/truck/truck-stepio/src/in/mod.rs` | `master_representation` | 3 |
| A6 | `vendor/truck/truck-stepio/src/in/mod.rs` | `impl TryFrom<&SurfaceCurve> for Curve3D` | 1 |
| A7 | `vendor/truck/truck-stepio/src/in/mod.rs` | `"POLYLINE" => \{` | 1 |
| A8 | `vendor/truck/truck-stepio/src/in/mod.rs` | `pub polyline: HashMap` | 1 |
| A9 | `vendor/truck/truck-stepio/src/in/mod.rs` | `enum BoundedCurveAny` | 1 |
| A10 | `vendor/truck/truck-stepio/src/in/mod.rs` | `fn parse_curve3d` | 1 |

A3/A4 count TWO sites each: the arms you replace in `sub_parse_curve3d` AND
the same-named arms in A6's `TryFrom` — leave A6's sites untouched (see C1).
A5 drops to 2 after the fix (field declaration + `TryFrom` match; the
`sub_parse_curve3d` arm goes). A7–A9 are the three `POLYLINE` wiring sites
C3 mirrors; A10 is where C2 lands. Assert post-state counts in your notes.

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `unimplemented!`, `todo!`, or
  out-of-range indexing reachable from geometry.
- **H-2** Fallible operations return the file's existing
  `Result<_, StepConvertingError>` convention — match the file, do not
  introduce a new error type.
- **H-3** No absolute constants in predicates; test epsilons carry `// H-3`
  on the SAME line as the literal. Tolerances come from the landed
  `ToleranceCtx`, never from new literals.
- **All cargo invocations go through the queue (the `cargo` on PATH IS the
  queue shim). Do not invoke cargo by absolute path; do not unset the shim.**
- Never run a bare `cargo test` — use the scoped commands below.
- A tradeoff the packet has already decided is not a judgement to relitigate.
  The one judgement left to you, if any: which of the C3 wiring sites need
  the `Curve2D` `TryFrom` arm (mirror `Polyline`'s footprint exactly and say
  in your notes what you mirrored). Everything else is decided above.

## Tests required

Named `#[test]` fns in `tests/occt_high_roi_cluster_001.rs` — the verifier
checks the names appear in your diff.

1. `sem_pcurve_master_001_pcurve_s1_uses_declared_3d_curve` — a cylinder
   with a circular arc written as `SURFACE_CURVE(3d, (pc1, pc2),
   .PCURVE_S1.)`: the parsed edge evaluates to the DECLARED 3D curve's locus
   at sampled parameters, endpoints reconciling with the vertex positions.
2. `sem_pcurve_master_001_pcurve_s2_uses_declared_3d_curve` — same with
   `.PCURVE_S2.` mastery.
3. `sem_pcurve_master_001_broken_curve_3d_refuses` — a `SURFACE_CURVE`
   whose `curve_3d` reference is unparseable returns the parse error (typed
   refusal), never a pcurve substitution and never a panic. (C2's fallback
   must NOT fire here: the pcurve candidates are only tried when the primary
   PARSES but fails reconciliation.)
4. `sem_pcurve_master_001_seam_crossing_extent_reconciles` — the
   seam-crossing trim extent (u: 5.9 → 6.4 on a 2π-periodic cylinder)
   survives: the parsed curve's parameter range spans the declared extent,
   not its principal-branch fold.
5. `occt_high_roi_cluster_001_trimmed_curve_parameter_trim` — a
   `TRIMMED_CURVE` over a `CIRCLE` basis with two `PARAMETER_VALUE` trims
   realizes the arc between them; sampled points land on the circle at the
   declared angles.
6. `occt_high_roi_cluster_001_trimmed_curve_point_trim` — same with
   `CARTESIAN_POINT` trims and `.CARTESIAN_POINT.` mastery.
7. `occt_high_roi_cluster_001_trimmed_curve_dual_trim_disagreement_refuses`
   — a trim carrying both a point and a parameter that disagree beyond the
   ctx tolerance converts to a typed error naming both readings, not a
   silent pick.
8. `occt_high_roi_cluster_001_trimmed_curve_line_parameter_scaling` — a
   `TRIMMED_CURVE` over a `LINE` with parameter trims t₀=2.5, t₁=4.0 (units
   of the direction vector) realizes the segment from `pnt + 2.5·dir` to
   `pnt + 4.0·dir` — NOT the 0..=1 range misread.
9. `occt_high_roi_cluster_001_pcurve_fallback_reconciles` — an edge whose
   3D-curve conversion fails endpoint reconciliation (construct one: e.g., a
   `SURFACE_CURVE` whose `curve_3d` is a `CIRCLE` whose placement cannot
   host the vertex points, plus an associated pcurve that IS correct) picks
   up the pcurve realization and reconciles. The test asserts the realized
   curve reconciles at both ends.
10. `occt_high_roi_cluster_001_pcurve_fallback_rejects_wrong_branch` — the
    issue-#1 shape: a `SURFACE_CURVE` whose pcurve realization is
    branch-folded (seam-crossing). The fallback must NOT accept it: the
    conversion refuses (typed error with residuals), and the realized curve
    is never the folded one.
11. `occt_high_roi_cluster_001_void_solid_bookkeeping` — the C4 oracle: a
    synthetic `BREP_WITH_VOIDS` (one exterior `CLOSED_SHELL`, one void
    `CLOSED_SHELL`) converts with both shells, exterior first, face count =
    exterior faces + void faces.

No existing test may be deleted, `#[ignore]`d, or weakened. The landed
`tests/input/` proptest suite must stay green — if a proptest failure
appears, check whether it failed at your fork point too (throwaway worktree)
before attributing it.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-stepio
cargo clippy -p truck-stepio --all-targets -- -D warnings
cargo test -p truck-stepio --tests
cargo check -p look
```

The last one proves the importer changes did not break the `look` binary
build. Send cargo output to a file and read the tail.

## Forbidden

Editing any file outside `write_allow` — especially anything under
`truck-meshalgo/`, `truck-geometry/`, `truck-certified/`, `truck-shapeops/`,
the landed `tests/input/` files, any landed test file,
`scripts/kernel-gates.sh`, `Cargo.lock`. Deleting the
`PreferredSurfaceCurveRepresentation` enum. Editing A6's `TryFrom`
conversion. Adding `#[ignore]`. Adding `#[allow]` without a justification
comment on the same line. Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- the C4 oracle does not match the production behavior described →
  `SPEC_GAP`, naming what you measured (and write no production change)
- the C2/C3 machinery cannot be realized on the synthetic fixtures without
  semantic loss the packet does not describe → `SPEC_GAP`, naming the loss
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

**COMMIT BEFORE writing `RESULT.json`.** Then write `RESULT.json` at the
root of your worktree (not `loop/results/` — the orchestrator files it
there).

```json
{"id":"OCCT-HIGH-ROI-CLUSTER-001","status":"DONE","contracts":["OCCT-HIGH-ROI-CLUSTER-001"],
 "tests_added":11,"anchors_verified":{"A1":1,"A2":1,"A3":2,"A4":2,"A5":3,"A6":1,"A7":1,"A8":1,"A9":1,"A10":1},
 "notes":"post-fix master_representation count (expect 2), whether the TryFrom path is reachable from face-trim ingestion, which C3 wiring sites you mirrored, and the residual evidence C2's refusal path now carries"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`fix(stepio): declared 3D curves over pcurve mastery; TRIMMED_CURVE; gated pcurve fallback; void-solid oracle (OCCT-HIGH-ROI-CLUSTER-001)`.
