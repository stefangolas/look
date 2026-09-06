# WORK PACKET CFP-001-LIFT-ENCLOSURE — certified lift enclosures; two defect corrections

You are correcting the two live enclosure defects and landing the certified
lift screen. Everything you need is in this document,
`docs/CONTACT_FAST_PATH_BUILD_SPEC.md`, and the two defect records. Do not
read other spec files. If something you need is genuinely missing, that is a
SPEC_GAP: stop and report.

```yaml
id:          CFP-001-LIFT-ENCLOSURE
contract:    [CFP-001-LIFT-ENCLOSURE]
class:       mechanical
crates:      [truck-evidence, truck-shapeops]
depends_on:  [CFP-000-SPINE]
write_allow:
  - vendor/truck/truck-evidence/src/enclosure.rs
  - vendor/truck/truck-evidence/src/lib.rs
  - vendor/truck/truck-shapeops/src/boolean/assemble.rs
  - docs/defects/DSC-BOUNDARY-SAMPLE-EXTENT-001.md
  - docs/defects/NUM-SPLINE-ENCLOSURE-CONVERGENCE-001.md
read_allow:
  - docs/CONTACT_FAST_PATH_BUILD_SPEC.md
  - docs/defects/DSC-BOUNDARY-SAMPLE-EXTENT-001.md
  - docs/defects/NUM-SPLINE-ENCLOSURE-CONVERGENCE-001.md
  - vendor/truck/truck-certified/src/cfp/fixtures.rs
  - vendor/truck/truck-certified/src/patch_admit.rs
  - vendor/truck/truck-evidence/src/enclosure.rs
  - vendor/truck/truck-shapeops/src/boolean/assemble.rs
  - vendor/truck/truck-geometry/src/nurbs.rs
tests_required:
  - fc0_defect_counterexample_red_on_sampled_green_on_certified
  - fc2_enclosure_containment_randomized
  - fc2_enclosure_convergence_under_bisection
  - fc2_enclosure_monotonicity
  - fc7_cross_check_evidence_hull_matches_certified_side
  - face_enclosure_used_by_lift_screen
  - param_box_derived_from_trim_not_boundary
budget:      {turns: 90, ctx_tokens: 200000}
```

## Problem

Two landed defects sit one layer below every certificate the contact funnel
builds:

1. **`DSC-BOUNDARY-SAMPLE-EXTENT-001`** (BG-ENC-001): `face_aabb`/`face_uv_box`
   (`truck-shapeops/src/boolean/assemble.rs:338-381`) derive the lift screen
   extents from boundary-curve samples. For a curved carrier the interior
   leaves the boundary hull, so the screen silently drops real contact pairs
   — the refusal machinery is never reached. The doc comment's claim ("the
   trimmed region's closure lies inside it") is false for curved carriers.
2. **`NUM-SPLINE-ENCLOSURE-CONVERGENCE-001`** (BG-ENC-002):
   `BSplineSurface::enclose/enclose_der/normal_cone`
   (`truck-evidence/src/enclosure.rs:302`) return whole-net bounds regardless
   of the query box. Sound but non-convergent; any loop that separates on
   enclosures cannot terminate on spline pairs, and cones never shrink under
   subdivision.

You fix BOTH with ONE shared primitive, and rewire the lift screen onto it.

## Scope decisions — pre-made, do not relitigate

1. **One primitive, two consumers.** The sub-box spline hull is: locate the
   knot spans overlapping the query box, de Casteljau-restrict each span's
   net to the box∩span intersection (exact knot manipulation via the landed
   `KnotVec` insertion ops — NO new spline math), union the per-span hulls.
   It serves (a) `enclosure.rs`'s per-query `enclose`/`enclose_der`/
   `normal_cone` and (b) `assemble.rs`'s face-level screen box (union of
   per-span boxes over the trim). Implement the primitive ONCE per crate as
   F1 requires (evidence cannot see certified's `SplinePatchStack`); the
   duplication is guarded by the F-C7 cross-check test, which is PERMANENT.
2. **Whole-net returns are replaced, not patched**: `enclose` on a query box
   returns the sub-box hull; `enclose_der(m,n)` restricts the degree-`(k−1)`
   derived net to the sub-box; `normal_cone` is composed from sub-box
   derivative enclosures. The interior/out-of-range guards stay (ENTIRE for
   out-of-domain queries, exactly as landed).
3. **The screen consumes certified enclosures**: `face_aabb` → per-span
   `enclose` boxes unioned; `face_uv_box` → the trim's true parameter extent
   (the parameter box the face already carries), NOT the boundary polygon
   hull. Boundary sampling (`create_parameter_boundary`, `SEARCH_TRIALS`
   root-finding) is REMOVED from the lift path.
4. **Canonical carriers are untouched**: their per-box enclosures are already
   exact. Only the spline arm of the enclosure impl changes.
5. **V5-boolean is in force**: the widened screen is monotone (sampled box ⊆
   image box ⊆ certified box), so boolean outputs can only GAIN contact
   events. Adjudicate every diff on the landed boolean batteries: a diff is
   accepted iff the new event set is a superset of the old on the same
   inputs. Record the adjudication in the packet notes. A red regression that
   is a pure superset diff is the bug being fixed — never a revert trigger.
6. **Refusal localization rides this packet**: the lift already builds
   `LiftedFace`/`LiftedEdge` with provenance (`assemble.rs:252+`); the sweep
   must pass stratum-pair indices into any refusal it emits, and the
   `boolean()` caller re-attributes to faces. Localizable-refusal SHAPE is
   frozen at the spine (read-only anchor); shapeops implements the
   re-attribution wrapper here.
7. **Zero new top-level evidence kinds.** The screen box is an input, not
   evidence; the enclosures are `Box3`/interval values under the landed
   BG-ENC discipline.
8. **Defect status updates**: both records move to
   `Correction landed — validated by F-C0/F-C2` with a pointer to the tests.
   Do not close them (synthetic witnesses only so far).

## Anchors — base-relative, drift-tolerant

Re-check on your branch:

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `truck-evidence/src/enclosure.rs` | `impl EnclosureSurface for BSplineSurface` | 1 |
| A2 | `truck-evidence/src/enclosure.rs` | `fn control_net_box` | ≥1 (the landed helper you keep for the whole-domain case) |
| A3 | `truck-shapeops/src/boolean/assemble.rs` | `fn face_aabb` | **1 before, 0 after your change** (renamed/replaced) |
| A4 | `truck-shapeops/src/boolean/assemble.rs` | `fn face_enclosure` | **0 before, 1 after your change** |
| A5 | `truck-shapeops/src/boolean/assemble.rs` | `fn face_uv_box` | 1 before (its body is replaced; the fn may survive with a new body — anchor on `create_parameter_boundary` instead) |
| A6 | `truck-shapeops/src/boolean/assemble.rs` | `create_parameter_boundary` | **>0 before**; after your change its ONLY remaining call sites must be outside the lift path (state the count you leave and why) |

A3/A4 are the packet-owned delta. If `fn face_enclosure` already exists,
another writer landed first: STOP, `ANCHOR_MISMATCH`.

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `unimplemented!`, `todo!`, or
  out-of-range indexing reachable from geometry.
- **H-3** No absolute constants in predicates; test epsilons carry `// H-3`
  on the SAME line as the literal.
- **BG-ENC-003**: outward rounding only; no fast-math; no FMA contraction.
- **Determinism**: identical ordered input → identical verdicts; span-locate
  and bisection orders fixed (axis order, low-before-high).
- **All cargo invocations go through the queue (the `cargo` on PATH IS the
  queue shim).**

## Tests required

Named `#[test]` fns; the verifier checks the names appear in your diff.
Fixture data is copied from `truck-certified/src/cfp/fixtures.rs` as
CONSTANTS, read-only (F1 — never import the module).

1. `fc0_defect_counterexample_red_on_sampled_green_on_certified` — F-C0: the
   sampled-screen pair is dropped, the certified-screen pair is admitted.
2. `fc2_enclosure_containment_randomized` — F-C2 data: dense point sampling
   of each box's image is contained in the enclosure (BG-ENC-001).
3. `fc2_enclosure_convergence_under_bisection` — F-C2: strict width decrease
   down the nest (BG-ENC-002) — the permanent guard.
4. `fc2_enclosure_monotonicity` — F-C2: `B ⊆ B′ ⇒ enclose(B) ⊆ enclose(B′)`.
5. `fc7_cross_check_evidence_hull_matches_certified_side` — F-C7: the
   evidence-side sub-box hull agrees with the certified-side per-span hull on
   the seeded inputs.
6. `face_enclosure_used_by_lift_screen` — compile/reach assertion: the sweep
   screen consumes `face_enclosure`, not a sampled box.
7. `param_box_derived_from_trim_not_boundary` — F-C1: the face's parameter
   box covers the trim's true extent; the boundary hull does not.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-evidence -p truck-shapeops
cargo clippy -p truck-evidence -p truck-shapeops --all-targets -- -D warnings
cargo test -p truck-evidence --lib enclosure
cargo test -p truck-shapeops --lib boolean
cargo check -p truck-certified
```

Send cargo output to a file and read the tail.

## Forbidden

Editing any file outside `write_allow` — especially `contact/**`, `ssi.rs`,
`tangency/**`, `construct/**`, `patch_admit.rs`, `Cargo.lock`. Changing the
canonical-carrier enclosure impls. Weakening the landed guards (ENTIRE on
out-of-domain queries stays). Adding `#[ignore]`. Adding `#[allow]` without a
justification comment on the same line. Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- a boolean battery diff is NOT a pure superset on adjudication → STOP and
  report in `RESULT.json` notes with the failing pair — that is a finding, not
  a failure to paper over
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

**COMMIT BEFORE writing `RESULT.json`.** Then write `RESULT.json` at the root
of your worktree.

```json
{"id":"CFP-001-LIFT-ENCLOSURE","status":"DONE","contracts":["CFP-001-LIFT-ENCLOSURE"],
 "tests_added":7,"anchors_verified":{"A1":1,"A2":1,"A3":0,"A4":1,"A5":1,"A6":0},
 "v5_boolean_adjudication":"<count of battery diffs, each a superset; one line each>",
 "notes":"both defects corrected; screen rewired; any deviation stated here"}
```

## Amendment r2 (orchestrator, 2026-09-06) - rebase and resolve the assemble.rs seam

Your r1 work committed as 930db75 and is intact, but its merge into
integration/kernel-bg CONFLICTED in
`vendor/truck/truck-shapeops/src/boolean/assemble.rs`: CTE-007-T2ARRANGE
landed first (its own tests appended at the same end-of-module anchor) and
your branch forked before that landing. Your commit is checked out at the
slot worktree root on branch `wip/CFP-001-r2` - unmodified, exactly as you
wrote it.

Required correction:
1. `git rebase integration/kernel-bg` (or cherry-pick / re-apply) so your
   commit sits on the integrated HEAD.
2. Resolve the assemble.rs conflict by keeping BOTH sides: every
   CTE-007-landed test keeps its exact name and assertions
   (`self_pair_rewrites_before_sweep`, `butt_join_union_drops_shared_wall`,
   their helpers), and all of your fixture constants, fixture fns, and
   tests land alongside them at module level. Union the use-blocks. No
   assertion may be weakened or deleted on either side - that is a
   Forbidden outcome.
3. Your enclosure.rs and defect-record changes should apply cleanly; if
   anything else conflicts, keep both semantics and state it in the RESULT.
4. Commit the resolution as ONE commit:
   subject `fix(evidence/shapeops): CFP-001 r2 - rebase onto integrated HEAD, resolve the assemble.rs seam (CFP-001-LIFT-ENCLOSURE)`.
5. Scoped checks (run them all):
   cargo check -p truck-shapeops -p truck-evidence
   cargo test -p truck-shapeops --lib boolean
   cargo test -p truck-evidence --lib
6. Write RESULT.json AT THE WORKTREE ROOT (r2 - status DONE only if every
   r1 test and every landed CTE-007 test passes).

Note: a worker commit 930db75 exists on branch wip/CFP-001-r1 as well; if
you find the rebase in an unexpected state, `git log --oneline -3` first
and recover from wip/CFP-001-r1.
