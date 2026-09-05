# WORK PACKET SEM-PCURVE-MASTER-001-FIX — honor the declared 3D curve over pcurve mastery

You are fixing a recorded STEP-ingestion defect inside the Certified
Interaction Engine (BIE) program wave — the write set is fully disjoint from
every BIE packet, so nothing here touches the interaction solver. Everything
you need is in this document and `docs/BIE_BUILD_SPINE.md`. Do not read other
spec files. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```yaml
id:          SEM-PCURVE-MASTER-001-FIX
contract:    [SEM-PCURVE-MASTER-001-FIX]
class:       mechanical
crates:      [truck-stepio]
depends_on:  []
write_allow:
  - vendor/truck/truck-stepio/src/in/mod.rs
  - vendor/truck/truck-stepio/tests/sem_pcurve_master_001.rs
read_allow:
  - vendor/truck/truck-stepio/tests/input/geometry.rs
  - vendor/truck/truck-meshalgo/src/tessellation/triangulation.rs
  - docs/defects/SEM-PCURVE-MASTER-001.md
  - docs/BIE_BUILD_SPINE.md
tests_required:
  - sem_pcurve_master_001_pcurve_s1_uses_declared_3d_curve
  - sem_pcurve_master_001_pcurve_s2_uses_declared_3d_curve
  - sem_pcurve_master_001_broken_curve_3d_refuses
  - sem_pcurve_master_001_seam_crossing_extent_reconciles
budget:      {turns: 45, ctx_tokens: 110000}
```

**New file** (`tests/sem_pcurve_master_001.rs`): H-1 applies — no
`unwrap_used` without a justified same-line opt-out. It is a NEW test path
(Test-Path confirmed absent at base); no landed test file may be touched.

## Problem (defect record: `docs/defects/SEM-PCURVE-MASTER-001.md`)

A STEP `SURFACE_CURVE` declares one 3D curve and per-face parametric traces
of it. The landed importer's `CurveAny::SurfaceCurve` arm honors
`master_representation`: for `.PCURVE_S1.` / `.PCURVE_S2.` it discards the
mandatory `curve_3d` and substitutes the pcurve, whose 2D geometry is
re-derived from vertex anchors recovered by
`surface.search_nearest_parameter` — principal-branch only. A seam-crossing
trim (u: 5.9 → 6.4) folds its end anchor to ≈0.117; the rebuilt pcurve no
longer reconciles with the vertex positions, and every face referencing the
edge refuses `EdgeTraversalUnresolved` (GitHub issue #1, `hub.step`: 2 of 24
faces drop; the `write_pcurves=False` twin of the identical solid renders
clean — the declared 3D curves are sufficient).

## The correction — PRE-DECIDED (record's correction 1), do not relitigate

**Route the `CurveAny::SurfaceCurve` arm through `c.curve_3d` regardless of
`master_representation`.** Concretely, in `sub_parse_curve3d`'s
`CurveAny::SurfaceCurve(c)` match arm:

- The existing `ctx.near_pt(p, q)` BG-TOL-001 early arm already routes
  through `c.curve_3d` and STAYS as is.
- The `Curve3D =>`, `PcurveS1 =>`, and `PcurveS2 =>` match arms are
  REPLACED by the single unconditional call
  `Self::sub_parse_curve3d(&c.curve_3d, p, q, same_sense)?`.
- **No pcurve fallback on failure.** If `curve_3d` parsing errors, the
  error propagates — a silent fallback to the pcurve path would
  reintroduce the defect through the back door. Honest refusal.
- `PreferredSurfaceCurveRepresentation` (the parsed entity field) STAYS on
  the holder struct — it is part of the entity schema and its
  `Deserialize` surface; it is simply no longer branched on. Do not delete
  the enum.
- `curve_3d` is a mandatory attribute of `SURFACE_CURVE`, so the pcurve
  branch is never necessary; the counterexample proves sufficiency. This
  also matches the documented ingestion contract (`ACCURACY_FINDINGS.md`
  records the trim pipeline as reading no pcurves — this branch was the
  exception violating it).

**Out of scope, explicitly**: the record's correction (2) (a
meshalgo-side single-edge retry safety net) and (3) (deck search) are NOT
built here — (3) becomes unreachable under (1), and (2) is booked as a
candidate follow-up only if corpus pressure demands it. Do not touch
`truck-meshalgo`.

## Anchors — measured 2026-09-05, counts are exact

Locate by pattern, never by line number. If a count differs, STOP and
report `ANCHOR_MISMATCH` with what you saw.

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `vendor/truck/truck-stepio/src/in/mod.rs` | `fn sub_parse_curve3d` | 1 |
| A2 | `vendor/truck/truck-stepio/src/in/mod.rs` | `Curve3D => Self::sub_parse_curve3d\(&c.curve_3d` | 1 |
| A3 | `vendor/truck/truck-stepio/src/in/mod.rs` | `PcurveS1 =>` | 2 |
| A4 | `vendor/truck/truck-stepio/src/in/mod.rs` | `PcurveS2 =>` | 2 |
| A5 | `vendor/truck/truck-stepio/src/in/mod.rs` | `master_representation` | 3 |
| A6 | `vendor/truck/truck-stepio/src/in/mod.rs` | `impl TryFrom<&SurfaceCurve> for Curve3D` | 1 |

A3/A4 count TWO sites each: the arms you replace in `sub_parse_curve3d`
(`PcurveS1 =>` at the SurfaceCurve match) AND the same-named arms in the
landed `impl TryFrom<&SurfaceCurve> for Curve3D` (A6) — a DIFFERENT
conversion path the defect record does not cover. **Leave A6's site
untouched** (it is outside the record's causal chain), but state in your
RESULT notes whether the `TryFrom` path is reachable from face-trim edge
ingestion (follow the callers read-only); if it is, that is a `SPEC_GAP` —
the record's correction would be incomplete without it.

A2–A4's `sub_parse_curve3d` arms are the three match arms you replace; A5
drops to 2 after the fix (the field declaration and the `TryFrom` match
remain; the `sub_parse_curve3d` match arm goes). Assert the post-state in
your notes.

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `unimplemented!`, `todo!`, or
  out-of-range indexing reachable from geometry.
- **H-2** Fallible operations return `Outcome<T>`/`Result` per the
  surrounding parser's existing convention — match the file, do not
  introduce a new error type.
- **H-3** No absolute constants in predicates; test epsilons carry `// H-3`
  on the SAME line as the literal.
- **All cargo invocations go through the queue (the `cargo` on PATH IS the
  queue shim). Do not invoke cargo by absolute path; do not unset the shim.**
- Never run a bare `cargo test` — use the scoped commands below.

## Tests required

Build the STEP input as an in-test string following the landed
`tests/input/geometry.rs` recipe (`step_to_entity` / `DataSection` from
str). Named `#[test]` fns in `tests/sem_pcurve_master_001.rs` — the
verifier checks the names appear in your diff.

1. `sem_pcurve_master_001_pcurve_s1_uses_declared_3d_curve` — a cylinder
   with a seam-crossing circular arc written as
   `SURFACE_CURVE(3d, (pc1, pc2), .PCURVE_S1.)`: the parsed edge evaluates
   to the DECLARED 3D curve's locus at sampled parameters, endpoints
   reconciling with the vertex positions (`// H-3` tolerance).
2. `sem_pcurve_master_001_pcurve_s2_uses_declared_3d_curve` — same with
   `.PCURVE_S2.` mastery.
3. `sem_pcurve_master_001_broken_curve_3d_refuses` — a `SURFACE_CURVE`
   whose `curve_3d` reference is unparseable returns the parse error
   (typed refusal), never a pcurve substitution and never a panic.
4. `sem_pcurve_master_001_seam_crossing_extent_reconciles` — the
   seam-crossing trim extent (u: 5.9 → 6.4 on a 2π-periodic cylinder)
   survives: the parsed curve's parameter range spans the declared extent,
   not its principal-branch fold.

No existing test may be deleted, `#[ignore]`d, or weakened.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-stepio
cargo clippy -p truck-stepio --all-targets -- -D warnings
cargo test -p truck-stepio --tests
cargo check -p look
```

The last one proves the importer change did not break the `look` binary
build. Send cargo output to a file and read the tail. Note: the landed
`tests/input/` proptest suite must stay green — if a proptest failure
appears, check whether it failed at your fork point too (throwaway
worktree) before attributing it.

## Forbidden

Editing any file outside `write_allow` — especially anything under
`truck-meshalgo/` (correction 2's territory), `truck-geometry/`,
`truck-shapeops/`, the landed `tests/input/` files, any landed test file,
`scripts/kernel-gates.sh`, `Cargo.lock`. Adding `#[ignore]`. Adding
`#[allow]` without a justification comment on the same line. Committing to
`main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- the declared-3D-curve route cannot parse the synthetic fixture without
  semantic loss the defect record does not describe → `SPEC_GAP`, naming
  the loss
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

**COMMIT BEFORE writing `RESULT.json`.** Then write `RESULT.json` at the root
of your worktree (not `loop/results/` — the orchestrator files it there).

```json
{"id":"SEM-PCURVE-MASTER-001-FIX","status":"DONE","contracts":["SEM-PCURVE-MASTER-001-FIX"],
 "tests_added":4,"anchors_verified":{"A1":1,"A2":1,"A3":2,"A4":2,"A5":3,"A6":1},
 "notes":"post-fix master_representation count (expect 2), whether the TryFrom path is reachable from face-trim ingestion, and anything the synthetic fixtures pinned down"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`fix(stepio): SURFACE_CURVE honors the declared 3D curve over pcurve mastery (SEM-PCURVE-MASTER-001)`.
