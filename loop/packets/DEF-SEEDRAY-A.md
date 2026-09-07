# WORK PACKET DEF-SEEDRAY-A — certified interval crossings for the four seed-ray carriers

The seed-ray parity test (`classify.rs::ray_seed`) decides crossing
existence by f64 rounding (`disc < 0.0`, `if t1 != t0`, `denom.abs() <=
NORMAL_SLACK`). This packet transcribes the crossing closed forms into
certified interval arithmetic. It is a STANDALONE module: it does NOT touch
the live classify path (that is DEF-SEEDRAY-B).

```yaml
id:          DEF-SEEDRAY-A
contract:    [DEF-SEEDRAY-A]
class:       design
crates:      [truck-shapeops]
depends_on:  []
write_allow:
  - vendor/truck/truck-shapeops/src/boolean/ray_cert.rs
  - vendor/truck/truck-shapeops/src/boolean/mod.rs
read_allow:
  - vendor/truck/truck-shapeops/src/boolean/classify.rs
  - vendor/truck/truck-evidence/src/
tests_required:
  - per-carrier: interval crossings enclose the float crossings on a
    fixture battery (the float form stays the hint oracle)
  - near-tangent fixtures (disc within a few ulp of 0) return the typed
    DegeneratePair / NoCrossing verdicts EXACTLY, never by rounding
  - the seven silent arms are unreachable-but-typed (see scope 3)
anchors:
  - {id: A1, expect: 1, cmd: "grep -c '=> Vec::new()' vendor/truck/truck-shapeops/src/boolean/classify.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'fn surface_ray_crossings' vendor/truck/truck-shapeops/src/boolean/classify.rs"}
  - {id: A3, expect: 7, cmd: "grep -c 'NORMAL_SLACK' vendor/truck/truck-shapeops/src/boolean/classify.rs"}
budget:      {turns: 55, ctx_tokens: 160000}
```

## Problem

`surface_ray_crossings` (classify.rs:602-711) answers "where does the ray
p + t·d hit this carrier" in plain f64: plane linear solve, cylinder/sphere
quadratics, cone double-nappe quadratic. Discriminant signs, root
distinctness (`t1 != t0`), and degeneracy guards (`NORMAL_SLACK`) are all
rounding decisions. The seed-parity theorem needs each crossing as a
CERTIFIED t-interval with an exact existence/exclusion decision.

Scope note (measured): the live classify path admits only
Plane/Cylinder/Cone/Sphere via `require_canonical_carriers`
(classify.rs:262 + the four-arm match at ~715); Torus/BSpline/NURBS/
Processor/SpineFrame/Revoluted/Extruded currently hit a SILENT
`=> Vec::new()` catch-all (703-710) behind that gate. This packet covers
the FOUR gated carriers only; the silent arms become type-explicit.

## Scope decisions

1. New module `boolean/ray_cert.rs` (+ the one-line `mod.rs` declaration):
   `certified_crossings(surface, p: Point3, d: Vector3, budget) ->
   Result<Vec<CertifiedCrossing>, Refusal>` where CertifiedCrossing =
   { t: CertifiedInterval, q: [CertifiedInterval; 3] } and the per-carrier
   decision vocabulary is EXACT: roots as outward-enclosed t-intervals;
   a discriminant enclosure containing 0 emits `DegeneratePair` (a typed
   near-tangency verdict), never a silent 0-or-2.
2. The four carriers: plane (linear), cylinder (xy quadratic), sphere,
   cone (double-nappe quadratic; the nappe filter belongs to the region
   stage, not here — emit both nappe roots, documented). Outward rounding
   end-to-end via the landed `CertifiedInterval`/inari algebra. The
   quadratic-in-t evaluation is monotone-safe: evaluate
   a·t² + b·t + c with `sqr`/`mul` on intervals; the discriminant sign
   is an exact `CertifiedSign` decision (Shewchuk/interval — reuse
   `formal/exact.rs` discipline where applicable).
3. The seven catch-all arms: this packet does NOT implement them, but the
   module's dispatch enum makes non-implemented carriers a TYPED
   `UnsupportedCarrier` variant (never an empty vec), so removing
   `require_canonical_carriers` later cannot silently corrupt parity.
4. Box-exit bound helper: `ray_exit_t(box3, p, d) -> CertifiedInterval`
   — where the ray leaves a certified enclosing box (feeds
   DEF-SEEDRAY-B's completeness argument). Pure interval arithmetic.
5. The float `surface_ray_crossings` stays UNTOUCHED (hint oracle for the
   conformance tests).
6. H-1: no panics. H-3: every constant named on its defining line.

## Done when

```
cargo test -p truck-shapeops --lib boolean::ray_cert
cargo test -p truck-shapeops --lib
```

## Forbidden

Touching classify.rs (DEF-SEEDRAY-B owns it). Touching the float form.
vendor/truck outside write_allow. Implementing spline/torus crossings
(typed refusals only — the expensive spline version is a separate booking).

## Stop conditions

- An exact discriminant decision requires machinery that does not exist in
  truck-evidence → SPEC_GAP naming it (do not approximate).

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `feat(shapeops): certified interval ray-crossing primitives for the four seed-ray carriers (DEF-SEEDRAY-A)`.
