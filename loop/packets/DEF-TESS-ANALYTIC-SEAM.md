# WORK PACKET DEF-TESS-ANALYTIC-SEAM — legacy tessellation cannot close analytic-surface shells

Discovered by DEF-VENDOR-FIXTURES (adjudicated 2026-09-07): the fork's
legacy tessellation + positional-welding path yields `ShellCondition::
Irregular/Oriented/Regular` (never `Closed`) for closed shells whose faces
carry analytic (revolved/cylindrical/spherical/toroidal) surfaces. Planar
shells close. Fixture-INDEPENDENT (reproduced in-memory: special_cylinder,
stepio geom_impls::builder cylinder segment). This is the path `look` uses
to render STEP.

Measured merged-mesh conditions at the reading pipeline (triangulate 0.01 +
put_together(TOLERANCE*2) + remove_degenerate/unused):
bottle=Irregular, punched-cube=Oriented, torus-punched-cube=Oriented,
sphere=Regular, torus=Oriented; only planar cube/cube-in-cube are Closed.

```yaml
id:          DEF-TESS-ANALYTIC-SEAM
contract:    [DEF-TESS-ANALYTIC-SEAM]
class:       design
crates:      [truck-meshalgo]
depends_on:  []
write_allow:
  - vendor/truck/truck-meshalgo/src/tessellation/
read_allow:
  - vendor/truck/truck-meshalgo/src/tessellation/
  - vendor/truck/resources/shape/
  - vendor/truck/truck-topology/src/shell.rs
tests_required:
  - solid_is_closed + csolid_is_closed green (all 8 vendored fixtures close)
  - special_cylinder + special_cylinder_csolid green
  - stepio geom_impls::builder green
  - the planar fixtures' triangle counts unchanged (no planar regression)
anchors:
  - {id: A1, expect: 2, cmd: "grep -c 'fn put_together_same_attrs' vendor/truck/truck-meshalgo/src/filters/optimizing.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'resources/shape' vendor/truck/truck-meshalgo/tests/tessellation/triangulation.rs"}
  - {id: A3, expect: 5, cmd: "grep -c 'ShellCondition::Closed' vendor/truck/truck-meshalgo/tests/tessellation/triangulation.rs"}
budget:      {turns: 70, ctx_tokens: 180000}
```

## Problem — the mechanism to diagnose first

`put_together_same_attrs(TOLERANCE * 2)` merges vertices POSITIONALLY (the
welding the certified path forbids, the legacy path is built on). A
revolved/cylindrical surface's seam (the u=0/u=2pi closure line, or the
v-range poles) emits duplicate vertices whose positions agree only to
evaluation/tolerance roundoff — if the seam vertices from adjacent faces
differ by MORE than the weld tolerance (or the seam is emitted with
different sample parameterizations per face), the weld misses, the boundary
1-chain does not vanish, and the shell reads Oriented/Irregular. The
condition varying by surface type (sphere=Regular vs torus=Oriented vs
bottle=Irregular) suggests the per-face sample counts at shared seams are
type-dependent (parameter_division per carrier) so faces disagree on seam
sampling density.

## Scope decisions

1. **DIAGNOSE FIRST, with the instrumented evidence in RESULT**: for one
   failing fixture (torus.json is the cheapest), dump the pre-weld mesh's
   boundary edges — count them, group by position error vs TOLERANCE*2,
   and identify whether the miss is (a) seam parameterization mismatch
   (different sample counts on the shared curve per adjacent face), (b)
   evaluation roundoff beyond the weld tolerance, or (c) genuinely
   unshared vertices (a face emits no seam vertices at all).
2. **The fix must be in the tessellation/welding pipeline**, NOT by
   loosening the tests and NOT by touching the certified constructive
   backend (which carries its own index-identity proof and is out of
   scope). Sanctioned directions, in preference order: (a) make adjacent
   faces agree on shared-edge sample parameterization (a per-edge sample
   contract keyed on the edge identity — the certified path's (EdgeID,
   ordinal) discipline, transplanted to the legacy path); (b) widen the
   weld step for seam vertices with a NAMED H-3 tolerance (weakest);
   (c) a topological post-pass that closes identified seam chains by
   index (must record every closure it makes — no silent welding).
3. H-1: no panics; H-3: every new tolerance named on its defining line.
4. The upstream OCCT-comparison tests stay environmental (no OCCT data);
   do not chase them.

## Done when

```
cargo test -p truck-meshalgo --test tessellation   (green except the 2 recorded compare_occt)
cargo test -p truck-stepio --lib                   (green except 0 - geom_impls::builder must pass)
cargo test -p truck-meshalgo --lib
```

## Forbidden

vendor/truck outside write_allow. Loosening the Closed assertions.
Touching the constructive backend or its tests. Silent welds (every
closure recorded).

## Stop conditions

- The diagnosis shows the seam miss is by MORE than a tolerance band
  (positions genuinely differ) -> the fix is (a) or (c); if neither is
  reachable without touching truck-topology, SPEC_GAP naming the
  boundary.
- Planar fixtures regress (triangle counts move) -> stop, re-plan.

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `fix(meshalgo): close analytic-surface shell seams in the legacy tessellation path (DEF-TESS-ANALYTIC-SEAM)`.
