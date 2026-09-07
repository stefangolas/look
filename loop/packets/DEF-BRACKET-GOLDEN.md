# WORK PACKET DEF-BRACKET-GOLDEN — attribute the bracket tessellation drift, then refresh the goldens with the attribution

`bracket_tessellates_to_a_known_mesh` pins 1814 triangles / 5442 vertices;
HEAD measures 1860. The refresh must be ATTRIBUTED, not assumed (AGENTS:
never update goldens solely to make a test pass).

```yaml
id:          DEF-BRACKET-GOLDEN
contract:    [DEF-BRACKET-GOLDEN]
class:       mechanical
crates:      [look]
depends_on:  []
write_allow:
  - tests/geometry_fingerprint.rs
read_allow:
  - tests/geometry_fingerprint.rs
  - tests/fixtures/bracket.step
  - docs/BENCHMARKS.md
tests_required:
  - bracket test green at the refreshed goldens
  - washer goldens (9518/28554) UNCHANGED and green
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'f.triangles, 1814' tests/geometry_fingerprint.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'f.vertices, 5442' tests/geometry_fingerprint.rs"}
budget:      {turns: 45, ctx_tokens: 160000}
```

## Problem — the recorded drift history

- 1814 set in `ba2b7be` (Aug 16). Moved to ~1875 during the BG-CG program
  (window Aug 18-22; suspects: BG-TOL-001-MESHALGO tolerance refactor,
  BG-CE-006-ENUM canonical model, BG-S0-004 spline-knots refusal,
  BG-CE-001/003 edge realization). Drift already present at base `fd65c24`
  (verified Sep 3 by throwaway worktree).
- 1875 -> 1860 in the window `fd65c24`..`d0108af`: exactly ONE commit
  touches the bracket's import path: `2bf7025` (OCCT-HIGH-ROI-CLUSTER-001,
  declared 3D curves over pcurve mastery) — edge realization changes =>
  parameter_division changes => triangle counts move. A deliberate
  correctness landing, so the -15 is attributable.
- The recorded suspicion of `10a1d13` (CC-DEF-BREP-FIXES) looks MIS-AIMED:
  that commit is constructive-spine code, not on the STEP-load path.
- c7c9b41 (BREP-001A) landed AFTER the 1860 confirmation and edits
  triangulation.rs/arrange.rs — re-measure at HEAD first; the count may
  have moved again.

## Work plan

1. **Re-measure at HEAD** (the `fingerprint()` path): triangle count,
   vertex count, and run the bounds gate (line 88) — bounds have never
   been reached in a failing run and are UNVERIFIED.
2. **Attribute by throwaway worktree** (the `1c31c4d` recipe:
   `git worktree add <path> <sha>` + `cargo --manifest-path`, remove after
   with `git worktree remove --force`; DISCIPLINE: the V9 floor is 15 GB —
   run `janitor ensure --need 15` FIRST, one worktree at a time, delete
   each worktree's target before creating the next):
   - at `ba2b7be` expect 1814;
   - at `fd65c24` expect ~1875 (if not, Window-1 bisect over the Aug 18-22
     commits);
   - at `2bf7025^` vs `2bf7025` (if 1875->1860 appears here, the -15 is
     attributed to the curve-realization change).
3. **Fidelity proof, not just a count**: bounds gate green; the bracket
   mesh passes manifold/watertightness diagnostics; vertices = 3 x
   triangles (5580) — a count that moves without bounds moving and with
   the WASHER green (9518, the tolerance-sharp circular-edge fixture) is a
   sampling-density change, not a fidelity regression. If the bounds gate
   or watertightness FAILS at HEAD: STOP — that is a real regression,
   record it and do not refresh.
4. **One commit** updating lines 86-87 to the measured values, with the
   attribution (commit hashes + mechanism) in the commit message AND in
   RESULT.

## Done when

```
cargo test --test geometry_fingerprint
```

both bracket and washer green.

## Forbidden

Refreshing without the attribution. Touching the washer goldens. Touching
the tessellation pipeline. Skipping the fidelity proof.

## Stop conditions

- Bounds or watertightness fail at HEAD -> real regression, STOP.
- The worktree bisect cannot attribute the move to any landing -> STOP,
  record the ambiguity; the refresh waits for owner adjudication.

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `test: attributed bracket golden refresh — <attribution> (DEF-BRACKET-GOLDEN)`.
