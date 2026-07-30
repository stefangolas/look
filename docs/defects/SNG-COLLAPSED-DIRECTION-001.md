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

> **Superseded, 2026-07-29. The "52 faces moved sideways" reading was an
> aggregate artifact and is withdrawn.** See §Falsified below. This record's
> status stays `Observed`, but it now has *less* evidence than it was recorded
> with, not more.

Aggregate, under `TRUCK_CONE_APEX_RANGE`, NIST (reproduced exactly):

| terminal reason | off | on |
|---|---:|---:|
| `NoSurfaceProduced` / cone | 216 | **268** |
| `MeshedToNothing` / cone | 132 | **0** |
| faces lost, total | 356 | **276** |

## Falsified: the 52 faces do not exist as a population

Resolving the same census **per model** shows the two sets of failing faces are
**disjoint by model**, so no face "trades an empty domain for a singular chart":

| model | off | on |
|---|---|---|
| `geom/ctc_02` | `NoSurfaceProduced` 148 | **0** |
| `geom/ctc_05` | `NoSurfaceProduced` 20 | **0** |
| `242/ftc_07`, `242/ftc_10`, `242/stc_07` | `NoSurfaceProduced` 16 each | **0** |
| `pmi/ctc_02` | `MeshedToNothing` 74 | **0** |
| `pmi/ctc_04`, `pmi/ctc_05`, `242/ctc_05` | `MeshedToNothing` 22 / 10 / 10 | **0** |
| `geom/ctc_01`, `geom/ftc_06`, `pmi/ctc_01`, `242/stc_06`, `242/stc_10` | `MeshedToNothing` 2–8 | **0** |
| `geom/ctc_04` | — | **`NoSurfaceProduced` 56** |
| `242/ctc_04` | — | **56** |
| `242/ctc_02` | — | **148** |
| `242/ctc_01`, `242/ftc_06` | — | **4 each** |

**Every model that failed with the flag off succeeds with it on, and a
different, previously-clean set of models fails with it on.** 348 faces
recovered, 268 *different* faces destroyed. The net −80 that looked like partial
progress is the difference between two unrelated populations.

The decisive pair is one part in two encodings:

| | faces | lost off | lost on |
|---|---:|---:|---:|
| `AP203geom / nist_ctc_02_asme1_rc` | 664 | **148** | **0** |
| `AP242 / nist_ctc_02_asme1_ap242-e2` | 637 | 2 | **150** (148 cone) |

Same part. Same 148 cone faces. The flag flips **which encoding loses them**.
And `ctc_04` renders completely in both encodings with the flag off and loses
exactly 56 in **both** with it on.

This does not exonerate the singular apex — it removes the only measurement
that was being used as evidence for it. Nothing here shows a rank-deficient
chart rejecting a face.

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

- **(D)** The aggregate counts: 216 → 268 `NoSurfaceProduced`, 132 → 0
  `MeshedToNothing`, 356 → 276 lost.
- **(D)** The per-model resolution above, and the `ctc_02` encoding pair.
- **(A)** That the apex is rank-deficient with non-unique angular inverse —
  standard geometry, not measured here.
- **~~(A)~~ Withdrawn**: that 52 faces fail *because of* the singularity. The
  population does not exist; the failing sets are disjoint by model.
- **(U)** Whether the singular apex causes **any** observed failure. This defect
  is now `Observed` on the *mathematics* (the chart genuinely is rank-deficient
  at $w^{*}$, and nothing represents that) and on **no measurement at all**. It
  should not be worked until a witness exists.

## Reduction, no longer valid

`apex_only.stp` **recovers under the flag — 49 triangles.** It is therefore a
witness for [`PAR-RANGE-INHERITANCE-001`](PAR-RANGE-INHERITANCE-001.md) and
**not** for this defect. The full `nist_ctc_01_asme1_rd` goes from 2 lost to 0.
Any witness for this defect must come from the newly-failing set —
`geom/ctc_04` at 56 faces is the smallest coherent candidate — and reducing one
is the prerequisite step, not a follow-up.

## Links

[`FORMALISM.md`](../../../look-collapsed-boundary/FORMALISM.md) §2, §7, U2 ·
`truck` `7199cc90` · `TRUCK_PROBE_SINGULAR` ·
upstream [`PAR-RANGE-INHERITANCE-001`](PAR-RANGE-INHERITANCE-001.md)
