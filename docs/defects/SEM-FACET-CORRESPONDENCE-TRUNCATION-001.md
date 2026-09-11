# SEM-FACET-CORRESPONDENCE-TRUNCATION-001 — Facet backend silently zip-truncates a mismatched LinearCorrespondence

**Family** `SEM` · **Manifestation** `OVERACCEPTANCE`, `MISATTRIBUTION`
*(acceptance measured; geometry distortion asserted)*
**Contracts** `ProfileLaw::LinearCorrespondence` — "requires an EXPLICIT
declared vertex/edge correspondence between start and end; correspondence is
never inferred" (`constructive/mod.rs`, normative); `ProfileLaw::try_linear_correspondence`
(the refusing constructor); plan §3.2.

## 1. Status

```
Closed (CC-DEF-BREP-FIXES, commit 10a1d13)
```n
## 2. Mathematical objects

`ProfileLaw::LinearCorrespondence { start, end }` declares a **positional**
vertex correspondence: vertex $i$ of `start` maps to vertex $i$ of `end`,
and intermediate stations interpolate vertex-wise. The declaration is only
well-formed when the counts are equal; `try_linear_correspondence` refuses
`ProfileCorrespondenceMismatch` otherwise. The struct is also constructible
as a plain literal with unequal counts.

## 3. Required obligation

A mismatched correspondence is a typed refusal
(`ProfileCorrespondenceMismatch`, or the entry's `InvalidInput`), on every
realization path. Correspondence is "never inferred" — and never silently
re-derived by dropping vertices.

## 4. What the implementation did

`ProfileLaw::evaluate`'s `LinearCorrespondence` arm builds the interpolated
ring with `start.vertices.iter().zip(end.vertices.iter())` — **Rust's `zip`
truncates to the shorter side**. A 4-vertex start with a 6-vertex end
silently interpolates to the first 4 vertices of the end profile: two of the
declared end vertices are dropped and the ring closes across the gap.
`facet_sweep` performs no count validation (its V1-domain check covers
`Constant`/`Scale` only), so the malformed law realizes. `spine_sweep`
refuses the identical law at its entry.

## 5. Minimal counterexample

`showcases/tests/battery_construction.rs::correspondence_mismatch_facet_path_behavior`:
vertical `LineSpine`, `LinearCorrespondence { start: square(4), end: hexagon(6) }`,
`FixedPlane`. Facet: `Ok`, `signed_volume = 0.0830`, clean verdict. BREP twin
(`correspondence_mismatch_spine_sweep_refuses`): `Err` on identical input.

## 6. Control / oracle

The BREP entry's refusal of the same recipe — the oracle is backend symmetry
plus the `try_linear_correspondence` constructor's own refusal.

## 7. Measurements

Battery test above: facet `Ok` with truncated-interpolation volume; BREP
`Err` on the identical recipe and stations.

## 8. First divergent checkpoint

**The evaluator's zip** (`constructive/profile.rs`, `LinearCorrespondence`
arm) is the truncation site; the missing count gate at the facet entry is
what lets the malformed law reach it.

## 9. Causal derivation

```
LinearCorrespondence built with unequal counts (struct literal)
→ facet_sweep has no count gate on this arm
→ evaluate() zips, truncating the longer profile
→ two declared end vertices silently dropped, ring closes across the gap
→ a sheared sweep of a profile nobody declared ships as certified
```

## 10. Proposed correction

Add the count-equality check to the `LinearCorrespondence` evaluation (or
better: a shared V1-domain validator called by both entries, per
[`SEM-FACET-SCALE-ZERO-001`](SEM-FACET-SCALE-ZERO-001.md)'s correction),
refusing `ProfileCorrespondenceMismatch` with the law attached.

## 11. Experimental correction

None yet.

## 12. Production correction

None — `vendor/truck` changes only through the packet loop.

## 13. Regression tests

`showcases/tests/battery_construction.rs::correspondence_mismatch_facet_path_behavior`
(pins the defect's signature) and its BREP twin. Invert the facet-side
assertion when the fix lands.

## 14. Corpus-wide effect

Not measured beyond the showcase; every mismatched-correspondence literal
reaching the facet path is affected.

## 15. Known exclusions

Laws built through `try_linear_correspondence` cannot trigger this — the
defect requires the struct-literal construction plus the ungated facet path.

## 16. Relationship to other defects

Entry-validation asymmetry, same class as
[`SEM-FACET-SCALE-ZERO-001`](SEM-FACET-SCALE-ZERO-001.md).

## 17. Claim status

- **(D)** The asymmetry (BREP refuses, facet accepts identical input) —
  measured by the twin battery tests.
- **(D)** Zip truncation is the mechanism — read from
  `constructive/profile.rs`'s `LinearCorrespondence` arm.
- **(A)** The realized geometry is "a profile nobody declared" — asserted
  from the truncation semantics; not yet independently visualized.

## 18. Links

- `truck-geometry/src/constructive/profile.rs` (the zip site)
- `truck-geometry/src/constructive/mod.rs` (the never-inferred doctrine)
- `truck-modeling/src/{spine_sweep,facet_sweep}.rs` (asymmetric entries)
- `showcases/tests/battery_construction.rs`
