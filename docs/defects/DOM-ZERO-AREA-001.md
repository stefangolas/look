# DOM-ZERO-AREA-001 — Artificial closure collapses a material region to zero area

**Family** `DOM` · **Manifestation** `OMISSION`, `DEGENERATION` (both measured
on the reproducer) · **Contracts** `DOM-001`, `DOM-004`, `CDT-005`

> Full record:
> [`../look-collapsed-boundary/FORMALISM.md`](../../../look-collapsed-boundary/FORMALISM.md)
> §4 steps 4–5, §4.1.

```
Status: Mechanism established
```

## Obligation

A face that its source describes as having nonempty physical extent must yield a
material region of positive measure, and a constructed region of measure zero
must be **refused, not meshed**:

$$S(\Gamma) \text{ nondegenerate} \implies \mu(M) > 0,
\qquad \mu(M) = 0 \implies \text{refusal with a named contract.}$$

## What the implementation did

Built the region, computed $\mu(M) = 0$, produced no triangles, and reported it
as an ordinary empty mesh — the coarse bucket `MeshedToNothing`. The zero area
is available at the moment it is computed and is discarded.

## Counterexample / control

`repro/apex_only.stp`, face `#370` — a cone of radius 10 and half-angle 59°
bounded by its base circle and its apex. Physically a substantial patch;
$\mu(M) = 0$. Control `repro/plane_control.stp`, 46 triangles.

## Measurement

```
PROBE in_closed=0 in_open=1 loops=1 areas=[+0.0000e0]
```

Exactly zero, not small. With $u_b = u_0 = 0$ the appended path runs along the
*same* line as the circle:

$$A(\gamma_{\text{stitched}}) = (u_b - u_0)\cdot 2\pi = 0 .$$

Then $\mu(M) = 0 \implies T_h = \varnothing$, and `look` reports
*"STEP tessellation produced no triangles"*.

## First divergent checkpoint

**I → J.** The zero area is the last checkpoint at which the defect is cheaply
detectable; by **K** the only evidence left is an empty triangle set.

## Causal derivation

Arrows 6–7 of `PAR-RANGE-INHERITANCE-001`'s chain. `DOM-ZERO-AREA-001` is
recorded separately from `DOM-ARTIFICIAL-CLOSURE-001` because it is the
**detectable** step: a pipeline that never fixed the range or the closure test
could still refuse here, name the contract, and stop attributing the loss to a
generic empty mesh.

## Correction

**Proposed**: refuse a zero-area material region at construction and report it
as a named contract failure carrying the loop and its computed area. This is
diagnostic quality — Axis 4 of the acceptance criteria — and it is the cheapest
item in the whole cone chain: it fixes no face and makes every one of them say
why it died.

It also directly serves the open item *"tessellation-stage loss reasons are
coarse"*: `NoSurfaceProduced` and `MeshedToNothing` should split into
projection / domain / arrangement / CDT terminal reasons. **Zero-area domain is
the first such reason and its witness already exists.**

**Experimental / production**: none.

## Tests

None. `dom_zero_area_001_degenerate_domain_is_refused_by_name` against the
reproducer would assert a typed refusal rather than triangle count, and would
keep passing after the upstream range defect is fixed — the right shape for a
regression on a *detection* obligation.

## Known exclusions

Refusing here **recovers no geometry**. The face is still lost; it is lost
*legibly*. Any claim that this fixes the cone faces would be wrong —
`PAR-RANGE-INHERITANCE-001` is where the geometry comes back.

## Claim status

- **(D)** Area is exactly 0, and the face produces no triangles.
- **(D)** An ordinary face from the same file produces 46.
- **(A)** That the region is *physically* nonempty — from the source geometry
  ($R = 10$, apex at $u^{*} = -6.010$), not from an independent oracle mesh of
  the same face.

## Links

[`FORMALISM.md`](../../../look-collapsed-boundary/FORMALISM.md) §4, §4.1 ·
`../look-collapsed-boundary/measurements/probe-apex-vs-control.txt` ·
upstream [`DOM-ARTIFICIAL-CLOSURE-001`](DOM-ARTIFICIAL-CLOSURE-001.md)
