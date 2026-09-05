# WORK PACKET CL-000-SPLINE-ADMIT â€” BÃ©zier patch admission + certified enclosures for BSplineSurface

You are implementing the spline-carrier admission layer of the Carrier Lift
(CL) program. Everything you need is in this document and
`docs/CARRIER_LIFT_BUILD_SPEC.md`. Do not read other spec files. If
something you need is genuinely missing, that is a SPEC_GAP (see "Stop
conditions"): you stop and report, you do not research it.

```yaml
id:          CL-000-SPLINE-ADMIT
contract:    [CL-000-SPLINE-ADMIT]
class:       mechanical
crates:      [truck-evidence, truck-certified]
depends_on:  []
write_allow:
  - vendor/truck/truck-evidence/src/enclosure.rs
  - vendor/truck/truck-certified/src/patch_admit.rs
  - vendor/truck/truck-certified/src/lib.rs
read_allow:
  - vendor/truck/truck-certified/src/ssi.rs
  - vendor/truck/truck-certified/src/ssi_types.rs
  - vendor/truck/truck-certified/src/hull.rs
  - vendor/truck/truck-evidence/src/num/krawczyk.rs
  - vendor/truck/truck-geometry/src/nurbs/bspsurface.rs
  - vendor/truck/truck-geometry/src/nurbs/bspcurve.rs
  - docs/CARRIER_LIFT_BUILD_SPEC.md
tests_required:
  - patch_admission_decomposes_and_refuses
  - bspline_enclosure_brackets_brute_sample
  - admitted_patches_feed_square_system
budget:      {turns: 50, ctx_tokens: 120000}
```

**New file** (`patch_admit.rs`): H-1 applies.

## Problem

The landed certified funnel refuses spline carriers (pinned by
`contact_ff_spline_surface_refuses`) because there is no admission bridge
from a `BSplineSurface` to the landed rational tensor-Bernstein engine
(`ssi.rs`) and no certified derivative enclosure for spline faces. This
packet lands both halves of that bridge. NO dispatch changes â€” the contact
layer is another packet's file.

## Scope decisions â€” pre-made, do not relitigate

1. **Patch decomposition is exact knot manipulation**: extract BÃ©zier
   patches by inserting interior knots to multiplicity (degree+1) â€” the
   landed `KnotVec` ops; no new spline math. Non-rational surfaces carry
   weight 1 (the corpus lofts are non-rational); a rational input with
   non-unit weights refuses typed (`NonPositiveNurbsWeight` exists in the
   landed refusal vocabulary â€” reuse it) unless all weights are within
   tolerance of 1.
2. **Admission gates** (all typed refusals, no new `Refusal` arms): finite
   control points, bidegree within the engine's budget, non-empty knots,
   positive patch count.
3. **`impl EnclosureSurface for BSplineSurface`**: the derivative of a
   B-spline is a B-spline of degree kâˆ’1 with exactly-derived control
   points; per-BÃ©zier-patch bounds come from the landed
   `hull_bernstein_2d` composition. The enclosure is OUTWARD by
   construction â€” never a sample-based claim (H-6).
4. `ssi.rs`, `ssi_types.rs`, `hull.rs` are NOT edited (V5 guard).

## Anchors â€” measured 2026-09-05, counts are exact

Locate by pattern, never by line number. If a count differs, STOP and
report `ANCHOR_MISMATCH`.

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `vendor/truck/truck-certified/src/ssi.rs` | `pub fn construct_square_system` | 1 |
| A2 | `vendor/truck/truck-certified/src/hull.rs` | `pub fn hull_bernstein_2d` | 1 |
| A3 | `vendor/truck/truck-certified/src/lib.rs` | `^pub mod` | 15 |
| A4 | `vendor/truck/truck-geometry/src/nurbs/bspsurface.rs` | `pub struct BSplineSurface` | 1 |

A3 becomes 16 when you add `pub mod patch_admit;`. If the measured count
of `^pub mod` differs because another program landed first, re-derive and
state the actual count in your RESULT notes â€” the anchor is the DELTA (+1).

## House rules

- **H-1** No `unwrap`, `expect`, `panic!` reachable from geometry.
- **H-2** Fallible operations return `Outcome<T>` / the file's Result
  convention â€” match the file you are in.
- **H-3** No absolute constants in predicates; test epsilons carry `// H-3`
  on the SAME line.
- **H-6** Enclosure bounds are certified by construction (outward), never
  recorded `Method::Exact`.
- **All cargo invocations go through the queue (the `cargo` on PATH IS the
  queue shim).** Never run a bare `cargo test` â€” scoped commands only.

## Tests required

1. `patch_admission_decomposes_and_refuses` â€” a known bicubic patch
   decomposes into the expected BÃ©zier grid (spot-check control points
   against the hand-derived case); inverted knots, NaN control points,
   and rational weights â‰  1 refuse typed.
2. `bspline_enclosure_brackets_brute_sample` â€” for â‰¥3 surfaces, the
   derivative enclosure brackets â‰¥1000 brute derivative samples per axis.
3. `admitted_patches_feed_square_system` â€” two admitted patch stacks
   construct a landed `SquareSystem3` via `construct_square_system`
   without error (the engine accepts the bridge's output â€” end-to-end
   compile+run proof).

No existing test may be deleted, `#[ignore]`d, or weakened.

## Done when â€” run these, all must pass

```
cargo fmt --check -p truck-evidence -p truck-certified
cargo clippy -p truck-evidence -p truck-certified --all-targets -- -D warnings
cargo test -p truck-certified --lib
cargo test -p truck-evidence --lib enclosure
```

Send cargo output to a file and read the tail.

## Forbidden

Editing anything outside `write_allow` â€” especially `contact/` (hot file,
another packet's), `ssi.rs`, `hull.rs`, `krawczyk.rs`, `bspsurface.rs`,
`scripts/kernel-gates.sh`, `Cargo.lock`. Adding `#[ignore]`. Adding
`#[allow]` without a same-line justification. Committing to `main`.

## Stop conditions

- any anchor count differs â†’ `ANCHOR_MISMATCH`
- degree elevation or knot insertion requires machinery outside the
  landed `KnotVec`/`BSplineCurve` API â†’ `SPEC_GAP`, naming the missing op
- three consecutive failed `cargo test` runs on the same error â†’ `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

**COMMIT BEFORE writing `RESULT.json`.** Then write `RESULT.json` at the
root of your worktree.

```json
{"id":"CL-000-SPLINE-ADMIT","status":"DONE","contracts":["CL-000-SPLINE-ADMIT"],
 "tests_added":3,"anchors_verified":{"A1":1,"A2":1,"A3":16,"A4":1},
 "notes":"the bidegree budget you enforced, and the enclosure tightness you observed vs brute samples"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(evidence): spline patch admission + BSplineSurface derivative enclosures (CL-000-SPLINE-ADMIT)`.
