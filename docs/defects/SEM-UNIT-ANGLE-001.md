# SEM-UNIT-ANGLE-001 — Declared plane-angle unit ignored

**Family** `SEM` · **Manifestation** `DISTORTION`, `EXCESS`
**Contracts** `GEO-001`

## 1. Status

```
Closed
```

Corpus-validated across all 33 NIST models. Two known extensions remain and
are **separate** obligations, recorded in §16.

## 2. Mathematical objects

A STEP file declares its units in a `GEOMETRIC_REPRESENTATION_CONTEXT` via
`GLOBAL_UNIT_ASSIGNED_CONTEXT`. Angle-valued scalars reaching geometry:
`CONICAL_SURFACE.semi_angle`, and (see §16) angular `PARAMETER_VALUE` trims.

## 3. Required obligation

Converted geometry must equal the **declared transform of the source**, not the
transform of a reinterpretation of it:

$$G_{\text{internal}} = T_{\text{declared context}}\bigl(G_{\text{source}}\bigr).$$

For a plane angle declared in a `CONVERSION_BASED_UNIT`, $T$ multiplies by the
declared factor. The obligation is `GEO-001`.

## 4. What the implementation did

Nothing. The importer had **no unit handling of any kind**; every scalar was
consumed in the numeric form the file wrote it. $T = \mathrm{id}$ unconditionally.

## 5. Minimal counterexample

`nist_ftc_07_asme1_rd.stp`:

```
#42  = (GEOMETRIC_REPRESENTATION_CONTEXT(3)
        GLOBAL_UNIT_ASSIGNED_CONTEXT((#24,#28,#38)) ...)
#24  = (CONVERSION_BASED_UNIT('DEGREE',#20) NAMED_UNIT(#19) PLANE_ANGLE_UNIT())
#20  = PLANE_ANGLE_MEASURE_WITH_UNIT(PLANE_ANGLE_MEASURE(0.0174532925),#18)
#686 = CONICAL_SURFACE('',#685,0.282184119986423,1.999999999999705)
```

A 2° draft cone read as 2 radians: $\tan 2^\circ = 0.0349$ against
$\tan(2\ \mathrm{rad}) = -2.185$. **Wrong by 63× and inverted in sign.**

## 6. Control / oracle

`nist_ftc_07_asme1_ap242-e2.stp` — the same part in a second encoding, which
renders correctly. The NIST corpus ships most parts in three encodings
(AP203 geometry-only, AP203+PMI, AP242), which makes it a **metamorphic oracle
that was sitting unused**: disagreement is self-evidencing and needs no
reference renderer.

## 7. Measurements

| | before | after |
|---|---:|---:|
| `ftc_07` triangles | 2,501 | **2,140** |
| escaping cone faces in the differential | 9 | **0** |
| render vs. its AP242 twin | fans | **agrees** |

Corpus sweep, all 33 NIST models: seven change, every one of the seven declares
degrees, five change imperceptibly (1–92 triangles), none regress visually. ABC
`00009190` byte-identical — it declares no degree units, so **this fixes none of
the ABC blobs**.

## 8. First divergent checkpoint

**B — entity and unit resolution.** The source graph (A) is identical between
the two encodings up to unit declaration; the converted surface (C) already
differs.

## 9. Causal derivation

```
plane-angle unit declared as DEGREE, factor 0.0174532925
→ importer applies no factor, semi_angle = 2.0 consumed as radians
→ tan(semi_angle) = -2.185 instead of +0.0349
→ generatrix direction (tan θ, 0, 1) points inward and 63× too steep
→ each corner draft face opens backwards at enormous angle
→ revolving it sweeps a full fan outside the part
```

**Why angles and not lengths.** The same file is in inches and always was, with
no ill effect: a length unit is a uniform scale, tolerance is relative, and
nothing downstream cares. An angle is not scale-covariant, and degrees beside
inches is dimensionally inconsistent — the error is a different *shape*, not a
different size. That asymmetry is why a total absence of unit handling stayed
invisible for the life of the project and then produced a blob.

## 10. Proposed correction

Resolve the plane-angle factor from the context and apply it at conversion.
Refuse rather than guess when independently assigned declarations disagree.

## 11. Experimental correction

None; went straight to production.

## 12. Production correction

`stefangolas/truck` `e1d4d4a0` *"Read angles in the units the file declares"*;
`look` `87a1d51`.

A factor applies only when every **independently assigned** declaration agrees;
otherwise it warns and converts nothing.

> **The honest-refusal rule caught a bug in itself.** The first version refused
> every file it existed to fix, printing
> `plane angle units disagree (1 vs 0.0174532925)` — a degree unit is *defined*
> as a multiple of a radian unit, so every degree file necessarily also contains
> a radian `SI_UNIT`, referenced but not assigned. Conversion bases are now
> excluded from the comparison rather than treated as competing declarations.
> Guessing instead of refusing would have converted by the wrong factor,
> silently.

## 13. Regression tests

In `truck-stepio`, under current names — the conversion-base exclusion has a
test named for it. **Not yet renamed to the `sem_unit_angle_001_*` convention**;
see the index's test-naming debt. The four-test ideal is not met: the
cross-encoding metamorphic test (`ftc_07` AP203 vs AP242 bounding box up to the
declared factor) is the obvious missing one and is `PR 9`'s first family.

## 14. Corpus-wide effect

NIST: 7 of 33 models change, all degree-declaring, none regress.
ABC: no change (no degree units in the corpus).

## 15. Known exclusions

- Does **not** touch `UNKNOWN-NIST-ORDINARY-CONE`: those 216 failures occur in
  radian files (`ap242/ftc_07` at `1.0297442575` = 59°) as well as converted
  degree files. A defect present on both sides of the unit fix is not a unit
  defect.
- Does **not** fix `UNKNOWN-CTC05-FUNNEL`, only improves it.

## 16. Successor obligations (not this defect)

- **Angular `PARAMETER_VALUE` trims are not converted.** 20 of 33 NIST files
  contain them. Design rule already decided: never assign a unit to
  `PARAMETER_VALUE` at parse time — the dimension comes from the consuming
  entity and parameter slot.
- **Resolution is file-global**, where the correct rule is
  per-`GEOMETRIC_REPRESENTATION_CONTEXT`. Sufficient today only because every
  file met so far agrees across its contexts; it refuses when they disagree.

## 17. Claim status

- **(D)** The unit was declared, ignored, and the resulting angle was wrong by
  63× with inverted sign — source arithmetic.
- **(D)** Fixing it changes `ftc_07` from fans to agreement with its twin, and
  changes exactly the seven degree-declaring NIST models.
- **(A)** That the corner fans *were* the draft cones — from the differential
  screen, which localised 9 escaping cone faces to 0, not from a per-face trace.

## 18. Links

- `truck` `e1d4d4a0`, `look` `87a1d51`
- [`PLAN.md` § Plane-angle units](../../PLAN.md)
- `examples/face_fingerprint.rs` — the per-face differential screen used to
  localise it
