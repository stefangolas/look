# WORK PACKET DEF-TESS-ANALYTIC-SEAM-R2 — analytic-shell seam closure via EdgeID-keyed per-edge sample contracts

r1 (loop/results/DEF-TESS-ANALYTIC-SEAM.PENDING.RESULT.json — READ FIRST)
reproduced the seam failures (bottle=Irregular, torus=Oriented; planar
shells close) and attempted the density-agreement fix (make adjacent faces
sample shared edges at matching density): REVERTED — it impacted planar
cases. r1's evidence redirects the mechanism: the problem is not sample
DENSITY, it is sample IDENTITY. The certified path already solves this
class: the constructive backend's index-identity discipline
(`truck-geometry/src/constructive/mod.rs`, frozen BG-CG-000) assigns each
mesh boundary vertex a position index that is a pure function of
(EdgeID, sample ordinal) — so two faces sharing edge E agree on seam
vertices BY CONSTRUCTION, with no welding and no density matching.

```yaml
id:          DEF-TESS-ANALYTIC-SEAM-R2
contract:    [DEF-TESS-ANALYTIC-SEAM-R2]
class:       design
crates:      [truck-meshalgo]
depends_on:  []
write_allow:
  - vendor/truck/truck-meshalgo/src/tessellation/
  - vendor/truck/truck-meshalgo/src/filters/
read_allow:
  - vendor/truck/truck-meshalgo/src/
  - vendor/truck/truck-geometry/src/constructive/mod.rs
  - vendor/truck/resources/shape/
  - loop/results/DEF-TESS-ANALYTIC-SEAM.PENDING.RESULT.json
tests_required:
  - solid_is_closed + csolid_is_closed green (all 8 vendored fixtures
    close; bottle must go Irregular -> Closed)
  - special_cylinder + special_cylinder_csolid green
  - stepio geom_impls::builder green
  - planar fixtures' triangle counts unchanged (the r1 regression class)
  - the seam-dump probe: pre-weld boundary edges on torus.json reduce to
    zero after the fix, with the merged positions identical
anchors:
  - {id: A1, expect: 2, cmd: "grep -c 'fn put_together_same_attrs' vendor/truck/truck-meshalgo/src/filters/optimizing.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'resources/shape' vendor/truck/truck-meshalgo/tests/tessellation/triangulation.rs"}
budget:      {turns: 70, ctx_tokens: 190000}
```

## Scope decisions

1. **DIAGNOSE with the seam-dump probe first (r1 built one; rebuild it
   cfg(test)-only):** for torus.json, dump every pre-weld boundary vertex —
   position, owning (edge id, parameter), and which face emitted it. The
   r1 evidence says seam vertices disagree; the probe must say whether
   (a) the same curve is sampled at DIFFERENT parameters per face
   (parameterization mismatch — the EdgeID-keyed contract fixes it),
   (b) identical parameters evaluate to different positions (evaluation
   asymmetry — the two faces' carrier surfaces disagree on the shared
   curve's embedding), or (c) a face emits no seam vertices at all. The
   diagnosis decides the fix; record it in RESULT.
2. **The fix: per-edge sample contracts keyed on edge identity** — when
   face A's triangulation samples shared edge E, the samples come from a
   per-edge ledger (EdgeID, ordinal) -> position, computed ONCE from a
   canonical owner (e.g. the edge's own 3-D curve, not either face's
   carrier evaluation), and face B consumes the same ledger. This is the
   constructive path's discipline transplanted; welding stays forbidden.
   If the legacy path's faces do not carry edge identity to the mesher,
   diagnose what identity IS available and stop before fabricating one.
3. **Planar immunity**: the fix must be a no-op when both faces are
   planar and already agree (the r1 regression class). The planar
   fixtures' triangle counts are the guard.
4. H-1: no panics; H-3: every tolerance named on its defining line; no
   welding (`put_together_same_attrs` call sites must not grow).

## Done when

```
cargo test -p truck-meshalgo --test tessellation  (green except the 2 recorded compare_occt)
cargo test -p truck-meshalgo --lib
cargo test -p truck-stepio --lib
```

## Forbidden

vendor/truck outside write_allow. Loosening the Closed assertions.
Silent welds. Touching the certified constructive backend. Density-only
matching (r1's reverted approach).

## Stop conditions

- The probe shows evaluation asymmetry (case b) — the two faces' surfaces
  disagree on the shared curve's embedding — that is a deeper defect in
  the carrier realization: STOP, SPEC_GAP naming it.
- Edge identity is absent from the legacy mesher's inputs and cannot be
  threaded without touching truck-topology → SPEC_GAP.

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `fix(meshalgo): EdgeID-keyed seam contracts close analytic-surface shells (DEF-TESS-ANALYTIC-SEAM-R2)`.
