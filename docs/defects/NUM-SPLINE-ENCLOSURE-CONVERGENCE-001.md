# NUM-SPLINE-ENCLOSURE-CONVERGENCE-001 — Spline enclosures ignore the query box; width does not converge

**Family** `NUM` · **Manifestation** `EXCESS` → `NONTERMINATION` *(asserted — code reading; no run witness yet)*
**Contracts** violates **BG-ENC-002** (enclosure width → 0 as box → 0) directly, and
degrades every consumer that separates on enclosures. BG-ENC-001 (soundness) is
**not** violated: the whole-net box is a valid over-approximation.

## 1. Status

```
Correction landed — validated by F-C0/F-C2
```

Corrected by `CFP-001-LIFT-ENCLOSURE`. `BSplineSurface::enclose` /
`enclose_der` / `normal_cone` (`truck-evidence/src/enclosure.rs`) now return
the sub-box spline hull: locate the knot spans overlapping the query box,
de Casteljau-restrict each span's net to the box∩span intersection (exact
`KnotVec` insertion via the landed ops), and union the per-span hulls — width
→ 0 as the box → 0 (BG-ENC-002). The whole-domain query still returns the
whole control-net hull (`control_net_box`). The permanent guard is the F-C2
battery `fc2_enclosure_convergence_under_bisection` (strict width decrease down
the nest) plus `fc2_enclosure_monotonicity` and
`fc2_enclosure_containment_randomized`; the F1 duplicate-leaf cross-check is
`fc7_cross_check_evidence_hull_matches_certified_side` (evidence suite).
Synthetic witnesses only so far — the record stays open.
```

## 2. Mathematical objects

`BSplineSurface<Point3>::enclose(uu, vv)` (`truck-evidence/src/enclosure.rs:302`),
and with it `enclose_der(m, n, uu, vv)` and `normal_cone(uu, vv)`, validate the
query box against the spline's interior and then return bounds derived from the
**whole-surface control net** (`control_net_box(self)` for values; the whole
derived net for derivatives; the cross of whole-net derivative boxes for the
normal cone) — regardless of `(uu, vv)`.

## 3. Required obligation

BG-ENC-002:

$$\operatorname{width}(\operatorname{enclose}(B)) \to 0 \quad\text{as}\quad \operatorname{width}(B) \to 0 .$$

A carrier whose enclosure is constant in the query box fails this for every
box strictly smaller than the full domain. The landed `DirCone` contract
inherits the failure: `normal_cone` is computed from whole-net derivative
boxes, so cones **never shrink under subdivision**.

## 4. What the implementation did

The spline impl is sound (over-approximation is the acceptable direction) but
query-independent. Consequences, in dependency order:

1. Any subdivision loop that separates on enclosures **cannot terminate on a
   spline pair** — cells shrink, enclosures do not. Every such loop burns to
   `Budget` and reports exhaustion; the exhaustion is an artifact, not
   geometry.
2. The CTE stagnation detector's precondition (cones that *stop* shrinking
   between levels) is vacuously true everywhere on splines — a detector built
   on today's cones would fire on every spline pair.
3. The Unresolved provenance split (budget-burn vs. genuine singularity) is
   unmeasurable until this is corrected: today's degenerate tail is dominated
   by the artifact.
4. The gff stage's "certified AABB intersection of their patches" receives
   whole-surface bounds for spline patches — sound, maximally coarse.

## 5. Minimal counterexample

Any spline carrier and two nested boxes: `B₂ ⊂ B₁` with
`width(B₂) ≪ width(B₁)`. `enclose(B₁) = enclose(B₂) = control_net_box` exactly.
The convergence assertion is falsified at zero width approaching; no corpus
file is needed (synthetic witness pending, same status as
`DSC-BOUNDARY-SAMPLE-EXTENT-001`'s).

## 6. Control / oracle

The canonical-carrier enclosures (Plane, Cylinder, Sphere, Cone, Torus), which
are per-box exact and converge.

## 7. Fix direction

Sub-box Bernstein hull: locate the knot spans overlapping the query box,
de Casteljau-restrict each span's net to the intersection, union the hulls
(exact knot manipulation via the landed `KnotVec` ops — no new spline math;
D1's discipline, applied per query). Implemented once as the shared
span-restriction primitive and reused at face level by the lift screen.
`truck-certified`'s per-span stack (`SplinePatchStack`) is the same computation
seen from the other side of the F1 edge; the duplicate leaf math carries a
permanent randomized cross-check test (fixture F-C7 of the CFP spine).

## 8. Related, deliberately not folded in

- `DSC-BOUNDARY-SAMPLE-EXTENT-001` — the boundary-sample screen under-encloses
  (BG-ENC-001); this record is its convergence sibling (BG-ENC-002). Both are
  corrected by CFP-001, but the mechanisms are distinct: one derives an
  enclosure from the wrong points, the other from the right points at the
  wrong scale.
- The V5-boolean consequence — correcting this widens the screen further and
  changes boolean outputs monotonically — is adjudicated under the CFP gate,
  not here.
