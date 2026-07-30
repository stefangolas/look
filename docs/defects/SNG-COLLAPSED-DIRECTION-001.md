# SNG-COLLAPSED-DIRECTION-001 — Rank-deficient apex treated as an ordinary chart point

**Family** `SNG` · **Manifestation** `OMISSION` (52 NIST faces measured moving
into `NoSurfaceProduced` under the counterfactual) · **Contracts** `QUO-005`,
`GEO-004`

> Full record:
> [`../look-collapsed-boundary/FORMALISM.md`](../../../look-collapsed-boundary/FORMALISM.md)
> §2, §7.2, U2.

```
Status: Observed
```

The one defect in this chain that is **not** yet localized. It was predicted as
`U2` before it was seen, and it arrived on schedule when the cone range was
extended.

## Objects and obligation

At the apex $w^{*} = -R/\tan\theta$ the whole circle $\{\varphi\}\times\{w^{*}\}$
maps to a single point, so

$$\operatorname{rank} J(\varphi, w^{*}) = 1,\qquad \det G = 0,$$

and the angular parameter has **no unique inverse** there. A pipeline stage that
requires a regular chart must either exclude the singular point or be told about
it:

$$\sigma_{\min}(J) > 0 \quad\text{for every ordinary regular-chart operation.}$$

A collapsed boundary is not an absent one: it marks precisely where this fails.

## What the implementation does

Nothing represents it. `QUO-005`'s singular chart has no representation, no
$\sigma_{\min}$ is computed anywhere, and inverse projection is called without
any statement of whether the query is near a rank-deficient point. A collapsed
`VERTEX_LOOP` contributes no trim segment — correct — but downstream is told
only that a bound was empty, not that the chart degenerates there.

`TRUCK_PROBE_SINGULAR` counts faces carrying a collapsed bound and is the only
acknowledgement in the system.

## Counterexample / control

**Not reduced.** The population is 52 NIST cone faces that move from
`MeshedToNothing` to `NoSurfaceProduced` when `TRUCK_CONE_APEX_RANGE` is set.
Reducing one of them to a single face — the way `apex_only.stp` was produced,
with `../look-collapsed-boundary/tools/reduce_to_face.py` — is the next
concrete step and has not been done.

## Measurement

Under `TRUCK_CONE_APEX_RANGE`, NIST:

| terminal reason | off | on |
|---|---:|---:|
| `NoSurfaceProduced` / cone | 216 | **268** |
| `MeshedToNothing` / cone | 132 | **0** |
| faces lost, total | 356 | **276** |

All 132 collapsed-apex faces leave `MeshedToNothing`; 80 render, and **52 trade
an empty domain for a chart that now contains the singular point.** The trade is
the witness: same faces, same file, one flag.

## First divergent checkpoint

Unknown — this is what "Observed" means here. The candidates, in the order they
should be tested, are **G (inverse projection near $\sigma_{\min} \to 0$)**,
**H (angular lift where the inverse is non-unique)**, and **J/K (arrangement or
CDT rejecting a degenerate cell)**. `NoSurfaceProduced` is too coarse to
distinguish them, which is itself the first thing to fix.

## Correction

**Proposed**, from `FORMALISM.md` §7: a working domain derived from face bounds;
an **explicit collapsed singular boundary** as a representable state;
quotient-space periodic closure; and **no requirement of a unique angular
inverse at the apex**. Together these are the principled version of
`PAR-RANGE-INHERITANCE-001`'s fix, and this defect is why that fix cannot simply
extend the range.

**Forbidden**: synthesising a small circle around the apex.

**Experimental / production**: none.

## Tests

None. A per-face $\sigma_{\min}(J)$ census over all cone faces is the
prerequisite measurement, not a test — it would establish whether the 52 form a
$\sigma_{\min}$-defined population or merely share a label.

## Known exclusions

Distinct from `UNKNOWN-NIST-ORDINARY-CONE`'s 216: those fail with the same
terminal reason **with the flag off**, from ordinary three-edge non-collapsed
bounds. The two must not be merged on the shared `NoSurfaceProduced` label —
that is the same mistake the apex/ordinary-cone falsification already caught
once.

## Claim status

- **(D)** 52 NIST faces change terminal reason under the flag; 132 → 0
  `MeshedToNothing`; 216 → 268 `NoSurfaceProduced`.
- **(A)** That the apex is rank-deficient with non-unique angular inverse —
  standard geometry, not measured here.
- **(A)** That the 52 fail *because of* the singularity. The correlation with
  chart extension is exact, but **no per-face trace has been run**, and the
  extended chart differs from the original in more than its inclusion of the
  apex.
- **(U)** Which pipeline stage rejects them. **This is the open question the
  investigation now returns to.**

## Links

[`FORMALISM.md`](../../../look-collapsed-boundary/FORMALISM.md) §2, §7, U2 ·
`truck` `7199cc90` · `TRUCK_PROBE_SINGULAR` ·
upstream [`PAR-RANGE-INHERITANCE-001`](PAR-RANGE-INHERITANCE-001.md)
