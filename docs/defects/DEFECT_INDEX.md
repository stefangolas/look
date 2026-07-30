# Defect index

A defect ID names a **violated mathematical or semantic obligation**. It does
not name a file, a face, a symptom, a `truck` function, or an error string.

`PAR-RANGE-INHERITANCE-001` means *a working surface domain was inherited from
an implementation primitive's arbitrary parameter range instead of being derived
from the represented geometry*. It does not mean *face `#370` disappeared*.
`#370` is a **witness**.

This index is part of the research process, not documentation deferred until
the end. Every investigation must end by creating an entry, strengthening one,
splitting one into distinct mechanisms, falsifying a hypothesis, validating a
correction, or closing a defect.

Read alongside:

- [`MATHEMATICAL_FOUNDATION.md`](../../MATHEMATICAL_FOUNDATION.md) — the
  numbered contract registry (`TOP-`, `GEO-`, `QUO-`, `DOM-`, `ARR-`, `CDT-`,
  `MSH-`, `SHL-`, `RES-`). A **contract** is an obligation the design imposes;
  a **defect** is an obligation the implementation broke. Each record below
  cites the contracts it touches.
- [`PLAN.md`](../../PLAN.md) — what was built and what it measured.
- [`HANDOFF.md`](../../HANDOFF.md) — current state, goes stale.

---

## Taxonomy

Primary family — the **violated obligation**:

| | |
|---|---|
| `SEM` | Semantic interpretation and units |
| `IDN` | Identity, provenance, and entity correspondence |
| `INC` | Incidence and topology |
| `ORI` | Orientation, sign, parity, and handedness |
| `PAR` | Parameterization, ranges, and chart mappings |
| `QUO` | Periodicity, seams, winding, and quotient spaces |
| `DOM` | Material-domain construction and classification |
| `GEO` | Geometric compatibility and residuals |
| `SNG` | Singularities and numerical conditioning |
| `DSC` | Discretization, sampling, and constraint conservation |
| `NUM` | Numerical algorithms, convergence, and resource bounds |
| `AGG` | Mesh aggregation, indexing, transforms, and export |

Secondary **manifestation** — what a user saw. A single defect may cause
several, and the manifestation is never the classification.

**A manifestation is listed as demonstrated only when a witness measured it.**
Manifestations inferred from code reading are marked *(asserted)* in the
register and justified in the record's claim-status section. A structural
defect made unrepresentable before any file triggered it has an asserted
manifestation, and saying so is the point of the distinction.

`OMISSION` · `EXCESS` · `DISTORTION` · `INVERSION` · `DEGENERATION` ·
`DISCONNECTION` · `INSTABILITY` · `NONTERMINATION` · `REFUSAL` ·
`OVERACCEPTANCE` · `MISATTRIBUTION`

The flag-gated cone-range experiment both recovered missing faces **and**
removed a longstanding blob shell, so "missing-face bug" and "blob bug" would
each have been a misleading primary classification of the same defect.

Status vocabulary — not "open"/"fixed":

`Observed` · `Localized` · `Mechanism established` · `Correction proposed` ·
`Correction experimentally validated` · `Closed` · `Refused correctly` ·
`Falsified`

---

## Register

| ID | Title | Status | Manifestation | Witness |
|---|---|---|---|---|
| [SEM-UNIT-ANGLE-001](SEM-UNIT-ANGLE-001.md) | Declared plane-angle unit ignored | Closed | DISTORTION, EXCESS | `nist_ftc_07_asme1_rd`, corner fans vs. its AP242 twin |
| [IDN-TRANSACTIONAL-INSERT-001](IDN-TRANSACTIONAL-INSERT-001.md) | Identity committed before fallible conversion | Closed | MISATTRIBUTION *(asserted)* | none — synthetic A/B/C only; no file observed to trigger it |
| [INC-EDGE-DROP-001](INC-EDGE-DROP-001.md) | Failed edge conversion silently shortened a bound | Closed | EXCESS; DISTORTION *(asserted)* | ABC `00009190`, 211 faces × 2,168 triangles from undescribed regions |
| [INC-VERTEX-LOOP-001](INC-VERTEX-LOOP-001.md) | Collapsed `VERTEX_LOOP` treated as unresolved | Closed for resolution; downstream split out | OMISSION | ABC `00009190` 272 faces, NIST 132; 1:1 with entity count in 8 files |
| [ORI-CHART-REFLECTION-001](ORI-CHART-REFLECTION-001.md) | Signed UV area used as chart-invariant | Closed | INVERSION, EXCESS *(share of 70→12 blobs asserted)* | ABC `00009190` blob shells; `trimming_domain.rs` |
| [ORI-SAME-SENSE-001](ORI-SAME-SENSE-001.md) | Face-local sense applied to shared surface state | Closed | INVERSION *(asserted)* | none — designed out; no corpus file counted for shared surfaces with disagreeing sense |
| [GEO-INCIDENCE-ACCEPTANCE-001](GEO-INCIDENCE-ACCEPTANCE-001.md) | Nearest point accepted as an incidence | Mechanism established; production correction refused | OVERACCEPTANCE | ABC `00009190`, 315 points at median 191× chord tolerance |
| [NUM-SUBDIVISION-GROWTH-001](NUM-SUBDIVISION-GROWTH-001.md) | Sample count from imported geometry, unbounded | Correction experimentally validated; `RES-003` violated by the fix | NONTERMINATION; MISATTRIBUTION *(the cap's silence, asserted)* | ABC `00000730`, 6.6-exabyte allocation |
| [QUO-LIFT-TIEBREAK-001](QUO-LIFT-TIEBREAK-001.md) | Winding class decided by an arbitrary tie-break | Closed | EXCESS; INSTABILITY *(asserted — invariance never tested)* | ABC `00009190`, 12 → 4 blob shells |
| [PAR-RANGE-INHERITANCE-001](PAR-RANGE-INHERITANCE-001.md) | Cone domain inherited from `Line`'s `[0,1]` | Mechanism established; counterfactual validated; production correction not validated | OMISSION, EXCESS | `apex_only.stp`; counterfactual +137 faces and blob shell `#161274` |
| [QUO-EUCLIDEAN-CLOSURE-001](QUO-EUCLIDEAN-CLOSURE-001.md) | Periodic closure tested by lifted Euclidean equality | Mechanism established | OMISSION | `apex_only.stp`, gap = 2π = perimeter, `closed=false` |
| [DOM-ARTIFICIAL-CLOSURE-001](DOM-ARTIFICIAL-CLOSURE-001.md) | Lone open trim closed against a parameter-range edge | Mechanism established | OMISSION | `apex_only.stp`, `in_open=1 loops=1` |
| [DOM-ZERO-AREA-001](DOM-ZERO-AREA-001.md) | Artificial closure collapses a material region to zero area | Mechanism established | OMISSION, DEGENERATION | `apex_only.stp`, `areas=[+0.0000e0]`, 0 triangles vs. control's 46 |
| [SNG-COLLAPSED-DIRECTION-001](SNG-COLLAPSED-DIRECTION-001.md) | Rank-deficient apex treated as an ordinary chart point | Observed in the mathematics; **its only measured witness was withdrawn 2026-07-29** | OMISSION *(withdrawn)* | **none.** The "52 faces moved sideways" population was an aggregate artifact — the failing sets are disjoint by model |

The last five are one investigation, and its full record — geometry,
measurements, and a claim-by-claim demonstrated / asserted / undemonstrated
labelling — is
[`../look-collapsed-boundary/FORMALISM.md`](../../../look-collapsed-boundary/FORMALISM.md)
(pushed as `stefangolas/look-collapsed-boundary`). Each of the five records
below states its own slice and links back to that document rather than
restating it.

`QUO-LIFT-TIEBREAK-001` is **not** in the seeded set handed over; it was added
because `PLAN.md` names the periodic-lift instability as one of the four
defects that motivated the architecture, and an index that omitted it would
misrepresent the record.

---

## Unknown populations

Real, measured, and **deliberately unnamed**. A mathematical ID asserts a
known violated obligation; these have only a population and a symptom. Naming
them early is how the apex/ordinary-cone conflation nearly happened.

| Tag | Population | What is known |
|---|---|---|
| ~~`UNKNOWN-NIST-ORDINARY-CONE`~~ **→ folded into [`PAR-RANGE-INHERITANCE-001`](PAR-RANGE-INHERITANCE-001.md), 2026-07-29** | 216 NIST cone faces, `NoSurfaceProduced` | Ordinary non-collapsed three-edge bounds, and **measured to be distinct** from the 132 collapsed-apex `MeshedToNothing` cones — anti-correlated across all 33 models, present in radian as well as converted degree files. Both of those exclusions still stand. But **all 216 recover under `TRUCK_CONE_APEX_RANGE`** (`geom/ctc_02` 148, `geom/ctc_05` 20, `242/ftc_07` · `242/ftc_10` · `242/stc_07` 16 each → 0), so they are sensitive to the same face-independent domain window. Distinct *manifestation* and distinct *bound structure*, **same violated obligation.** `U4`'s first divergent checkpoint is now attributed, not merely unmeasured. |
| `UNKNOWN-ABC-BSPLINE` | 112 bspline + 70 nurbs `NoSurfaceProduced` on ABC `00009190` | The largest untouched category. Never investigated at all. |
| `UNKNOWN-CTC05-FUNNEL` | `ap203pmi/nist_ctc_05_asme1_ap203` renders its cylindrical shaft as a cone with a disc cap | Improved but not fixed by `SEM-UNIT-ANGLE-001` (2,230 → 2,196 triangles). Suspected: angular `PARAMETER_VALUE` trims on circles, which are not unit-converted; 20 of 33 NIST files contain them. Design rule already decided: **never assign a unit to `PARAMETER_VALUE` at parse time** — the dimension comes from the consuming entity and parameter slot. (`FORMALISM.md` U5) |

---

## Baseline

Every measurement in this index is against the **default-off** configuration:
pinned fork revision, no `.cargo/config.toml` path override,
`TRUCK_CONE_APEX_RANGE` unset, `COMPATIBILITY_FACTOR = f64::INFINITY`.

Reconfirmed 2026-07-29 on a clean tree at `look` `a56ac6b` / `truck`
`7199cc90`, rebuilt release:

```console
look inspect <abc>/00009190/00009190_..._step_000.step
#  warning: 396 of 24202 STEP faces produced no geometry
#           (3 failed to convert, 227 had no surface, 166 meshed to nothing)
#  triangles: 216335
```

Matches the recorded figures exactly. Give every run a fresh `LOOK_CACHE_DIR` —
`look inspect` caches statistics keyed on the source file, so a sweep that
varies behaviour by environment variable and not by input reports the first
run's numbers every time.

## Record length

A record is a research instrument, not a monograph. Two are kept full because
their length is measured evidence rather than exposition:
[`SEM-UNIT-ANGLE-001`](SEM-UNIT-ANGLE-001.md) (the corpus sweep and the
refusal-catches-itself finding) and
[`INC-VERTEX-LOOP-001`](INC-VERTEX-LOOP-001.md) (which carries a **falsified
hypothesis** and the three measurements that killed it). Everything else states
the obligation, what the code did, the witness, the causal chain, and the
claim status — and links out rather than restating. The five cone records link
to `FORMALISM.md`; they do not duplicate it.

## Method

The governing rule: **measure broadly, reduce surgically, identify the first
violated mathematical obligation, correct the model rather than the fixture,
and preserve the counterexample and proof as a permanent indexed test.**

1. **Cheap intrinsic census.** Per face and per boundary, without repairing
   anything: surface class, bound count and kind, collapsed-bound count,
   periodic axes and periods, declared parameter range versus sampled boundary
   range, curve–surface max and RMS residual, endpoint closure residual,
   quotient winding vector, Jacobian singular values, loop area, arrangement
   cell count, triangles produced, mesh area and AABB, last successful stage,
   typed terminal reason. `examples/face_census.rs` supplies the conversion
   half of this today; the tessellation half is still two coarse buckets
   (`NoSurfaceProduced`, `MeshedToNothing`) where it needs projection / domain
   / arrangement / CDT terminal reasons.
2. **Find an intrinsic anomaly**, not a historical signature. *"Every face
   where the claimed geometric relationships do not commute"*, never *"every
   face that looks like the last blob"*.
3. **One representative and one nearby control**, differing in as few relevant
   properties as possible.
4. **Reduce**: one model → one shell → one face → one boundary → one
   curve/surface relationship, preserving source semantics and recording
   exactly what changed.
5. **Trace checkpoint by checkpoint** — source graph, entity and unit
   resolution, converted geometry and transforms, topology and oriented uses,
   sampled 3D boundaries, curve–surface compatibility, inverse projection,
   periodic lifting and winding, material domain, arrangement and constraints,
   CDT, face mesh, shell aggregation — and stop at the **first** checkpoint
   where the representative stops satisfying the same relationships as the
   control. Never repair a later symptom above an earlier false relationship.
6. **Write the violated invariant** as a formula, stating both what was
   promised and what the code actually checked.
7. **Derive the mechanism**, which is more than a correlation.
8. **Test a counterfactual** behind a flag when it is not yet principled.
9. **Design the principled correction** in geometric terms. Never land
   file-specific exceptions, arbitrary range multipliers, tolerances loosened
   until 2π counts as zero, fabricated small circles around singular points, or
   silently guessed parameters.
10. **Validate against invariances**: original counterexample, nearby control,
    equivalent encoding, sampling-density change, wire-start rotation,
    orientation reversal, seam shift, chart reflection, full corpus. A repair
    that only fixes the fixture is not closed.
11. **Update this index.**

### Statistics

Used to discover intrinsic regimes and outliers, never to memorise prior
failure labels. Robust z-score

$$z_{\text{robust}} = \frac{x - \operatorname{median}(x)}{1.4826\,\operatorname{MAD}(x)}$$

over intrinsic features: normalized curve–surface residual, minimum Jacobian
singular value, inverse condition number, distance of bounds outside the
declared range, winding vector, domain-area ratio, constraint survival
fraction, mesh/source diameter ratio.

Statistics can say *these faces form a distinct geometric population*. They
cannot say *this loop is closed* — that needs quotient topology, source
semantics, or a numerical certificate tied to the correct metric.

---

## Test naming

A closed defect's regressions carry its ID, so the test index and this index
are one thing:

```rust
#[test] fn sem_unit_angle_001_degree_cone_matches_radian_twin() {}
#[test] fn inc_vertex_loop_001_collapsed_bound_is_retained() {}
#[test] fn quo_euclidean_closure_001_full_period_is_closed_in_quotient() {}
#[test] fn par_range_inheritance_001_face_domain_does_not_use_line_default() {}
```

Ideally four per closed defect: the original counterexample, the nearby
control, a metamorphic or equivalent-encoding test, and a corpus-level
assertion that the affected population no longer fails.

> **Debt, stated plainly: no test in either repository is named for a defect
> ID today.** Every closed defect below does have regressions, and each record
> names the ones it has under their current names — but the ID-keyed
> convention starts from this index, and the existing tests have not been
> renamed. Retro-naming them is mechanical and is the cheapest way to make the
> index load-bearing rather than descriptive. No record claims a test that
> does not exist.
