# DSC-BOUNDARY-SAMPLE-EXTENT-001 — Boundary-sample extent used as an enclosure for a curved carrier's image

**Family** `DSC` · **Manifestation** `OMISSION` *(asserted — code reading; no run witness yet)*
**Contracts** violates the lift-layer instance of **BG-ENC-001** (an extent
consumed as an enclosure must satisfy `enclose(B) ⊇ {f(p) : p ∈ B}`;
under-estimation is the cardinal failure). No contract in
`MATHEMATICAL_FOUNDATION.md` yet names the stratum screen — candidate for
registration.

## 1. Status

```
Mechanism established   — code-verified at named sites; counterexample is
                          synthetic and not yet run
Not corrected           — fix direction recorded, no experiment landed
```

## 2. Mathematical objects

Two extents derived per lifted stratum in the boolean entry
(`truck-shapeops/src/boolean/assemble.rs`):

1. **The 3-D AABB** (`face_aabb`, `:366-374`; `edge_aabb`, `:376-381`): the
   min/max over `curve.parameter_division` samples of the face's *boundary
   curves*. The doc comment asserts: *"the trimmed region's closure lies
   inside it."*
2. **The `(u, v)` parameter box** (`face_uv_box`, `:338-355`): the min/max over
   the boundary wires' parameter polygons (`create_parameter_boundary` hull).

Both are consumed as enclosures:

- the 3-D AABB gates the cross-solid sweep — a pair is passed to the contact
  oracle only when `fa.aabb.touches(&fb.aabb)` (`:144-189`);
- the parameter box becomes the stratum's parameter domain for the certified
  stages.

## 3. Required obligation

For a carrier `S` with trim domain `T` and image `S(T)`:

$$\operatorname{AABB}_{\text{screen}} \supseteq S(T), \qquad \Box_{uv} \supseteq T .$$

A rejection screen built on an under-estimating box drops work silently: the
refusal machinery is never reached, so the failure mode is a *completed
boolean with missing contact events* — precisely the class BG-ENC-001 names
cardinal, one layer below every certificate the funnel builds.

The obligation is violated whenever the carrier is **curved and the trim's
interior leaves the boundary hull**. The doc comment's claim holds for planar
faces (the region lies in-plane within its boundary's extent) and fails for
any carrier whose interior bulges away from its boundary curves: a spherical
cap whose boundary is a small circle has its apex strictly outside the
boundary-sample hull; the same holds for lofted body panels, which are the
corpus's primary carrier class.

## 4. What the implementation did

`face_aabb` hulled boundary-curve samples and the sweep used the result as a
rejection test. For a curved face, the true image can touch the partner where
the boundary-sample hull does not — the pair is never dispatched to
`contact()`, and the boolean proceeds as if no contact existed. No refusal is
emitted anywhere, because the oracle was never reached.

The parameter-space twin has the same shape: a trim whose parameter extent is
not spanned by its boundary's parameter polygon (interior islands, caps around
a pole, trimmed sub-regions of a windowed sweep) yields a stratum box that
under-covers the trim, so downstream certified work reasons over a wrong
domain.

## 5. Minimal counterexample

Synthetic, not yet run: `boolean(sphere_cap_solid, Difference, plane_slab)`
where the cap's apex is the contact region and the cap's boundary circle lies
above the slab. The boundary-sample AABBs do not touch; the expected
intersection fragment is silently absent. A loft-panel version on the
showcase corpus is the production-shaped witness.

## 6. Control / oracle

The same pair with the two solids swapped, or the same boolean through the
closed-form canonical path where no screen intervenes; and any planar-faced
boolean, where the sampled extent happens to be sound.

## 7. Measurements

None yet. Required before correction: (a) the synthetic counterexample above
red/green; (b) the false-positive-pair rate under candidate fixes on the
corpus, since the fix direction trades silent omission for over-admission and
its width regression must be measured, not assumed (diagonal loft panels are
both the motivating case and the worst regression risk for a whole-trim
enclosure).

## 8. Fix direction (from the 2026-09-06 carrier-lift review)

Replace boundary-derived extents with certified enclosures from the landed
`EnclosureSurface` interface every carrier already implements:

- world screen: `enclose(parameter_box)` per stratum — sound by construction,
  over-estimating in the acceptable direction (extra pairs admitted, refused
  typed downstream);
- the whole-trim enclosure may be materially wider than the truth for diagonal
  skinned surfaces; the tighter certified variant is a **union of
  per-knot-span `enclose` boxes** over the D1 Bézier decomposition, which the
  carrier has already paid for at admission;
- the parameter twin likewise derives from the trim's true parameter extent,
  not the boundary polygon hull.

The lift's per-stratum boundary sampling (`create_parameter_boundary`,
`SEARCH_TRIALS` root-finding) disappears with the fix, which is a measured
cost removed from every boolean.

## 9. Falsified hypotheses (recorded so they stay dead)

- **"Circle–circle EE pairs are silently skipped too."** Falsified: the
  `ee_circle_circle` guard (`assemble.rs:315-331`) is a *documented*
  v1-envelope decision — those pairs reach the splitter through the identity
  and FF/Region2 records, and skipping the funnel dispatch keeps the
  through-cut recombination inside the v1 envelope. Not a defect.
- **"The screen's sample count makes this tolerance-dependent."** Unrelated:
  the mechanism is structural (interior vs. boundary), not a sampling-density
  artifact; refining `INSERTION_TOL` narrows but never closes the gap.

## 10. Related, deliberately not logged here

- **gff cover seeding completeness** (interior closed loops): plausible but
  *unverified* — no measured or code-reading witness that `cover_branch` seeds
  from boundary crossings only. Promote to its own record only with that
  verification.
- **Quadric×spline typed refusal** (`AnalyticNotAffine`): honest typed
  refusal, not a correctness bug — a coverage item booked by the carrier-lift
  program, not a defect.
- **Unresolved propagation into the boundary rewrite**: an unspecified
  consumer contract (spec gap), not a violated one.
