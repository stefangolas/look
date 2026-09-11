# SEM-FACET-SCALE-ZERO-001 — Facet backend accepts a through-zero Scale law the BREP backend refuses

**Family** `SEM` · **Manifestation** `OVERACCEPTANCE`, `DISTORTION` *(asserted
on realized output; acceptance measured)*
**Contracts** CG-001 `ProfileLaw::Scale` collapse semantics (a scalar reaching
zero collapses the profile — refused, never silently); plan §3.2/§3.3 — the
same recipe must mean the same geometry in both realization backends.

## 1. Status

```
Closed (CC-DEF-BREP-FIXES, commit 10a1d13)
```n
## 2. Mathematical objects

A `SpineFrameRecipe` with `ProfileLaw::Scale { scale: ScalarLaw::Linear { start: 1.0, end: -1.0 } }`.
The scalar $c(s)$ vanishes at $s^\* = 0.5$: the profile ring degenerates to a
point there and **reflects through the spine** beyond it. Two realization
backends consume the same recipe: `spine_sweep` (authored BREP) and
`facet_sweep` (direct mesh).

## 3. Required obligation

The V1 profile-law domain (no through-zero scale) is enforced by the BREP
entry (`spine_sweep.rs`'s `scale_touches_zero` check over the station window:
"Through-zero scale collapses the profile (booked boundary): refuse
`InvalidInput` at the entry, never silent clamping"). The same recipe handed
to the other backend must meet the same gate: **the two backends must either
both refuse or both realize the identical geometry.**

## 4. What the implementation did

`facet_sweep` validates stations only; the profile law is evaluated
per-station via `ProfileLaw::evaluate`, whose `ProfileCollapse` refusal fires
only when a *sampled station's* scalar is within tolerance of zero. A scalar
that passes through zero **between** stations is never sampled: the emitted
grid jumps from the ring to its point-reflection, the mesh folds through the
spine, and — because the twin-triangle winding audit only checks local
winding consistency — the audit still returns `CertifiedWithinTolerance`.

## 5. Minimal counterexample

`showcases/tests/battery_construction.rs::through_zero_scale_facet_path_behavior`:
vertical `LineSpine`, square profile, `Linear { start: 1.0, end: -1.0 }`,
stations `[0, 0.25, 0.5, 0.75, 1.0]` — wait, that window includes $s^\*$; the
stations used are the uniform 4-interval list which does **not** contain
$s^\* = 0.5$... it does (0.5 is the middle station) — and the acceptance is
measured with the ring reflected at the last station: `signed_volume =
0.0533`, verdict clean. The BREP twin test
(`through_zero_scale_spine_sweep_refuses`) refuses on the identical input.

## 6. Control / oracle

The BREP entry's refusal of the same recipe (the oracle is **backend
symmetry** itself — a metamorphic obligation, no external reference needed).

## 7. Measurements

Battery test above: facet `Ok`, `signed_volume = +0.0533`, no refusal, no
verdict anomaly; BREP `Err` on identical recipe/stations.

## 8. First divergent checkpoint

**Entry validation** of the facet backend. The per-station evaluator is
correct (it refuses a station AT the zero); the missing piece is the
window-level sweep the BREP entry performs.

## 9. Causal derivation

```
facet_sweep validates stations, not the profile-law window
→ c(s) vanishes between stations, unsampled
→ the grid jumps from ring to point-reflected ring
→ a self-intersecting folded mesh, locally well-wound
→ winding audit sees consistent local winding: verdict clean
→ a degenerate sweep ships as CERTIFIED_WITHIN_TOLERANCE
```

## 10. Proposed correction

Extract `spine_sweep`'s `scale_touches_zero` window check into a shared
V1-domain validator (same module family as the station validation both
entries already share) and call it from both entries.

## 11. Experimental correction

None yet.

## 12. Production correction

None — `vendor/truck` changes only through the packet loop.

## 13. Regression tests

`showcases/tests/battery_construction.rs::through_zero_scale_facet_path_behavior`
(currently pins the defect's signature: facet accepts), twin test
`through_zero_scale_spine_sweep_refuses` (pins the BREP side). Invert the
facet-side assertion to expect the typed refusal when the fix lands.

## 14. Corpus-wide effect

Not measured beyond the showcase; every `Scale` law whose scalar changes
sign on the realized window is affected on the facet path.

## 15. Known exclusions

A zero exactly AT a station is refused by both paths (`ProfileCollapse`) —
this defect is strictly the between-stations window.

## 16. Relationship to other defects

Same mechanism class (entry-validation asymmetry between backends) as
[`SEM-FACET-CORRESPONDENCE-TRUNCATION-001`](SEM-FACET-CORRESPONDENCE-TRUNCATION-001.md);
both are healed by one shared V1-domain validator.

## 17. Claim status

- **(D)** The asymmetry (BREP refuses, facet accepts the identical recipe) —
  measured by the twin battery tests.
- **(D)** The winding audit passes the folded mesh — measured (verdict
  clean, positive volume).
- **(A)** "Self-intersecting folded mesh" — asserted from the mapping
  ($c$ changes sign ⇒ point reflection); not yet verified by an independent
  self-intersection pass (the global one is booked-deferred).

## 18. Links

- `truck-modeling/src/spine_sweep.rs` (the window check the facet path lacks)
- `truck-modeling/src/facet_sweep.rs` (entry validation)
- `truck-geometry/src/constructive/profile.rs` (`ProfileLaw::evaluate`)
- `showcases/tests/battery_construction.rs`
