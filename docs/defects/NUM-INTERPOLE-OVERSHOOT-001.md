# NUM-INTERPOLE-OVERSHOOT-001 — try_interpole produces catastrophically oscillating interpolants at moderate data counts

**Family** `NUM` · **Manifestation** `DISTORTION`, `INSTABILITY`
**Contracts** the CC program's Phase-A audit row for P1 ("try_interpole
takes a caller-supplied knot vector and plain f64 solve — non-certified");
the general interpolation obligation: an interpolant through bounded data
of extent $E$ must not exceed $E$ by orders of magnitude between data
points.

## 1. Status

```
Closed for the standing API (CC-DEF-BREP-FIXES, commit 10a1d13: SW admission + BOUND_FACTOR; certified path remains CC-001 business)
```
## 2. Mathematical objects

Cubic B-spline interpolation of $n$ data points
$\{(t_i, P_i)\}_{i=0}^{n-1}$, $P_i \subset [0, 110]^3$-scale, parameters
near-uniform on $[0, 1]$, via
`BSplineCurve::try_interpole(knot_vec, parameter_points)`
(`truck-geometry/src/nurbs/bspcurve.rs:271`), solved by
`gaussian_elimination::gaussian_elimination` (plain f64).

## 3. Required obligation

Either the returned curve stays within data-scale bounds between the data
points, or the failure is typed. An interpolation API whose result is
exact at the data and $10^{10}\times$ the data extent between two adjacent
data points is unusable as a silent default — and the CC-010 loft books
its certified replacement precisely because of this class.

## 4. What the implementation did

Measured on the waterslide composite path (drop → helix → runout, 109 m
total extent), client knot vectors:

- **Uniform interior knots** (clamped, `i/(n-3)`): the interpolant passes
  through every data point (max deviation $10^{-15} \dots 10^{-6}$ at all
  $n$), but the between-sample coordinate maximum grows:

  | $n$ | max \|coord\| between samples |
  |---|---|
  | 51 | 30 (clean — equals the path extent) |
  | 66 | $4.2 \times 10^{2}$ |
  | 98 | $2.0 \times 10^{4}$ |
  | 130 | $3.1 \times 10^{9}$ |
  | 161 | $9.4 \times 10^{6}$ |
  | 257 | $8.2 \times 10^{10}$ |

- **De Boor averaged knots**: strictly worse — the solve stops hitting the
  data at all ($10^{9}$ deviation at $n{=}51$, $10^{67}$ at $n{=}161$),
  consistent with the no-pivot elimination collapsing on the (worse
  conditioned) averaged-knot collocation matrix.

The realized consequence: `facet_sweep` over such a spine emits a mesh
with a few stations at $10^{3} \dots 10^{8}$ m while the winding audit
stays clean and the verdict is `CertifiedWithinTolerance` (the oscillation
is locally well-wound), i.e. the pipeline ships a distorted sweep as
certified.

## 5. Minimal counterexample

`showcases/examples/knot_probe.rs` (the measurement table above) and
`showcases/examples/mesh_probe.rs` (the realized-mesh consequence:
`bad` stations with $10^6$-scale coordinates at 160 path samples, clean
verdict). The data is the waterslide composite path
(`showcases/src/spine.rs::composite_path`); any dense smooth path should
reproduce the class.

## 6. Control / oracle

$n = 48$: clean at both knot choices' uniform case (max coordinate equals
the true path extent) — the interpolation is well-behaved in the small-$n$
regime, which bounds the defect to the conditioning cliff, not to
interpolation per se. The oracle is data-scale boundedness.

## 7. Measurements

Tables above (`knot_probe.rs`, fresh run 2026-09-04, dev profile,
x86_64-pc-windows-gnullvm).

## 8. First divergent checkpoint

**The linear solve** (`gaussian_elimination::gaussian_elimination`, no
pivoting) is the asserted site: the uniform-knot system stays exactly
interpolating while its solution's between-sample behavior explodes —
huge control points from a near-singular system — and the averaged-knot
system (worse conditioned in this ordering) loses the data entirely,
which is the signature of elimination without pivoting, not of the
interpolation problem itself.

## 9. Causal derivation

```
collocation matrix conditioning degrades with n (knot placement + no pivoting)
→ control points with enormous magnitudes
→ the interpolant stays exact at the data but swings orders of
  magnitude beyond the data extent between samples
→ downstream realization samples the swing regions
→ facet mesh with 10^6-scale stations, clean local winding
→ certified-within-tolerance distorted sweep
```

## 10. Proposed correction

Partial pivoting (or the certified banded solver CC-001 lands) in the
interpolation path, plus either Schoenberg-Whitney validation of the
caller's knot vector or documentation that makes the conditioning contract
explicit. The CC-010 loft design (de Boor averaging + SW a-priori
nonsingularity + CC-001 certified solve) is the booked structural fix on
the loft side; this record covers the standing `try_interpole` API.

## 11. Experimental correction

Client-side mitigation validated: staying at $n \approx 48$ data points
keeps the interpolant bounded (the showcase tables pin this).

## 12. Production correction

None — `vendor/truck` changes only through the packet loop.

## 13. Regression tests

`showcases/examples/knot_probe.rs` (the scaling table) and the
showcase battery's overshoot guard
(`showcases/tests/battery_waterslide.rs::facet_mesh_stays_within_path_bounds`).

## 14. Corpus-wide effect

Not measured beyond the showcase; any client interpolating more than
~60 points through `try_interpole` is exposed.

## 15. Known exclusions

The certified kernel path (`truck-certified`) is unaffected by
construction; CC-010 does not consume `try_interpole`.

## 16. Relationship to other defects

Provides the measured motivation for the CC program's Phase-A audit row
(P1: no certified banded solve); unrelated to the ORI/SEM frame and
backend-symmetry defects.

## 17. Claim status

- **(D)** The scaling table (both knot choices) — measured, reproducible
  via `knot_probe.rs`.
- **(D)** The realized-mesh consequence (distorted stations under a clean
  verdict) — measured via `mesh_probe.rs`.
- **(A)** Root cause = no-pivot elimination — from code reading plus the
  averaged-knot failure signature; a pivoting A/B has not been run
  in-tree.

## 18. Links

- `truck-geometry/src/nurbs/bspcurve.rs` (`try_interpole`, the GE call)
- `docs/CERTIFIED_CONSTRUCTION_BUILD_SPEC.md` (CC-001/CC-010 rows)
- `showcases/src/spine.rs` (the affected client construction)
- `showcases/examples/{knot_probe,mesh_probe}.rs`
