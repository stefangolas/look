# SEM-PCURVE-MASTER-001 — A pcurve master representation displaced the declared 3D curve

**Family** `SEM` · **Manifestation** `REFUSAL`, `OMISSION` · **Contracts**
`GEO-001`, `GEO-004`, `GEO-006`, `QUO-004`

**Status** `Mechanism established; correction proposed`.

## Obligation

A `SURFACE_CURVE` declares one 3D curve and per-face parametric traces of it.
The 3D curve is the entity: both pcurves are projections of it onto two
charts. The realized edge curve must therefore be the declared 3D locus:

$$C^{\mathrm{int}}_e = C^{\mathrm{STEP}}_e,$$

and where a trim anchor is recovered by projection, the projection must be a
**valid inverse** (`GEO-006` — a nearest point on the principal branch of a
periodic chart is not), with deck consistency across the lattice (`QUO-004`).
Substituting a chart-local re-derivation for the declared curve converts an
exact representation into an approximation whose correctness depends on an
unguarded branch selection.

## What the implementation did

`truck-stepio`'s `EdgeCurveHolder::sub_parse_curve3d`
(`vendor/truck/truck-stepio/src/in/mod.rs:3460–3489`) honors the
`SURFACE_CURVE.master_representation`: for `.PCURVE_S1.` / `.PCURVE_S2.` it
discards `curve_3d` — present and mandatory — and substitutes the pcurve. The
pcurve arm (`mod.rs:3439–3458`) then *re-derives* the 2D geometry from vertex
anchors recovered by `surface.search_nearest_parameter(vertex_point)`, which
on a periodic surface returns only the principal-branch parameter. A
seam-crossing arc whose trim runs u: 5.9 → 6.4 has its end anchor folded to
≈0.117; this site carries no deck handling at all (the 3D conic arm at least
applies a single `v += 2π` rule). `sub_parse_2d` (`mod.rs:3199–3316`) then
rebuilds the 2D conic by radial projection of those anchors onto the 2D
circle — with a branch-wrong anchor, the recovered endpoints sit on the wrong
side.

A secondary structural defect rides along: truck resolves **one curve per
`EDGE_CURVE`**, but the entity carries a different pcurve per incident face;
choosing S1 couples both faces to one face's chart (`GEO-004` — face-side
pcurves may differ, and here the shared realization silently picks one).

## Counterexample / control

GitHub issue #1 (`hub.step`, build123d 0.11.0 / OpenCASCADE 7.9): the
identical solid exported with `write_pcurves=False` renders all 24 faces on
the same look revisions — the declared 3D curves are sufficient, isolating
the failure to the pcurve-bearing path. The control differs in exactly one
exporter flag.

Witnesses: face `#213` (planar, 5 bounds, 18 edge uses) and face `#991`
(trimmed cylinder, 1 bound, 4 edge uses, u-periodic) — the two faces adjacent
to the seam-crossing arcs of the box cut through the cylinder wall. They
share the failing `EDGE_CURVE`s, so one unresolved edge refuses both.

## Causal derivation

```
OCCT writes write_pcurves=True
→ curved edges become SURFACE_CURVE(3d, (pc1, pc2), .PCURVE_S1.)
→ sub_parse_curve3d honors master_representation, discards curve_3d
→ pcurve arm re-derives the 2D trim from search_nearest_parameter anchors
→ principal-branch anchor folds a seam-crossing extent (no deck handling)
→ PCurve endpoint evaluations no longer reconcile with VERTEX_POINT positions
→ establish_source_edge_traversal returns Unresolved
   (truck-meshalgo triangulation.rs:1788–1810)
→ any face referencing the edge is refused whole: EdgeTraversalUnresolved
   (triangulation.rs:2032–2049, refusal by design, cf. INC-EDGE-DROP-001)
→ the two adjacent faces drop from the render
```

## Correction

**Proposed (1, preferred): honor `curve_3d` whenever present.** Route the
`SurfaceCurve` arm through `c.curve_3d` regardless of
`master_representation`. `curve_3d` is a mandatory attribute of
`SURFACE_CURVE`, so the pcurve branch is never necessary; the counterexample
proves the 3D curve is sufficient. This also matches the documented
ingestion contract — `ACCURACY_FINDINGS.md` records the trim pipeline as
reading no pcurves; this branch is the exception that violates it. Vendor
change: through the packet loop, not a direct edit.

**(2, safety net regardless):** when an edge resolves `Unresolved`, retry that
single edge from its retained `EDGE_CURVE.curve_3d` before failing the face;
accept only if the reconstructed traversal reconciles with the vertex
positions. Preserves refusals for genuinely broken sources.

**(3, only if pcurve mastery must ever be honored):** recover anchors with a
deck search (±1 period, cf. `src/step/torus_deck.rs`), accept the candidate
whose PCurve evaluation reconciles within source tolerance, and resolve the
curve per (edge use, owning face) rather than once per `EDGE_CURVE`.

## Tests

None yet. Proposed: the issue's `hub.step` (both export variants) in the
corpus, asserting 24/24 faces on the PCURVE variant; a unit test with a
seam-crossing circular arc on a cylinder written as
`SURFACE_CURVE/.PCURVE_S1.` asserting endpoint reconciliation. To be named
`sem_pcurve_master_001_*` when landed.

## Producer landscape

The trigger is exporter-dependent, and the exposure is skewed. Kernels that
maintain persistent parametric trim curves — OpenCASCADE, and ACIS also
carries them — are the ones that emit `SURFACE_CURVE` with `.PCURVE_S1.` /
`.PCURVE_S2.` mastery. Parasolid does not keep pcurves as first-class
persistent geometry; it resolves trims from 3D edge geometry on demand, and
Parasolid-based STEP output (NX, SolidWorks, Onshape) writes plain 3D edge
curves that pass through the current importer. The exposed population is
therefore essentially OCCT/ACIS-derived files, OCCT dominating it through the
open-source CAD ecosystem. **(A)** asserted from kernel-architecture
knowledge, not from a survey of exporter output.

This asymmetry has a consumer-side consequence: an OCCT producer can set
`write_pcurves=False`, but a recipient of a third-party pcurve-bearing file
cannot re-export — the importer-side correction is what covers that half.

## Known exclusions

The `EdgeTraversalUnresolved` populations in the ABC diagnostics
(`scratch/ur10_diag.jsonl` ×8, `scratch/corpus_additions/quadruped.jsonl` ×4)
carry **no PCURVE indirection** (`ABC_BAND_SWEEP.md`: "no `PCURVE` at all" in
that refused population) and are **not** attributed to this defect — same
terminal string, different mechanism.

If correction (3) is ever built, the principal-branch anchor recovery is its
own quotent defect (`QUO` — deck-less nearest-parameter anchoring) and should
be split out; under correction (1) it becomes unreachable and the split stays
hypothetical.

## Claim status

- **(D)** 2 of 24 faces refused, `EdgeTraversalUnresolved` ×2; the
  `write_pcurves=False` twin renders clean. Measured by the issue reporter on
  look `a7966c26` and `f0ed52ed`; not re-measured locally.
- **(A)** The substitution and deck-less anchoring steps: established by code
  reading, deterministic from the source, but the *specific* folded anchor and
  the resulting residual were never printed — a probe on `hub.step` would
  discharge this.
- **(A)** That the failing edge is exactly the seam-crossing arc (inferred
  from the two adjacent faces' geometry, not measured per edge).

## Links

GitHub issue
[`stefangolas/look#1`](https://github.com/stefangolas/look/issues/1) ·
`truck-stepio` `in/mod.rs:3439–3489` · `truck-meshalgo`
`tessellation/triangulation.rs:1788–1810, 2032–2049` ·
[`ACCURACY_FINDINGS.md`](../handoffs/ACCURACY_FINDINGS.md) ·
[`GEO-006`](../../docs/MATHEMATICAL_FOUNDATION.md), `QUO-004` ·
[`INC-EDGE-DROP-001`](INC-EDGE-DROP-001.md) (the all-or-nothing refusal the
witness surfaces through)
