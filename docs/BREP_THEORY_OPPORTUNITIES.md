# BRep theory opportunities — static analysis, 2026-09-08

Ranked mathematical substitutions for the structurally expensive or missing
transitions found in `BREP_TRANSITION_AUDIT.md` and `BREP_GAP_REGISTER.md`.
**No new benchmarking was performed.** Where no measurement exists, "expected
mechanism" is a static/literature judgement, never a claimed speedup; the
repository's own measured bottlenecks are recorded separately in
`BREP_EXISTING_RUNTIME_EVIDENCE.md` (tessellation CDT insertion blowup —
fixed; PNG/output on the resident renderer path; nothing measured inside the
certified kernel beyond the Phase-1 dispatch latency and the Phase-2 whole-run
wall).

Design constraint honored throughout: every candidate enters the **existing**
implementation as a stronger exclusion predicate, stronger contractor,
lower-dimensional reduction, better certification primitive, or a
wider-admission adapter over the landed engine — no new symbolic runtime, no
new evidence universe (`docs/BREP_SYSTEM.md` §8).

---

## TO-1 — Admit positive-weight rational faces into the existing SSI form
```text
current transition:   NURBS face → SSI            (representation exists; admission refuses)
current implementation: patch_admit.rs:97,151-198 refuses weights ≠ 1 (UNIT_WEIGHT_REL_TOL 1e-7);
                      engine ssi.rs:1356-1410 already consumes RationalBipatch with positive
                      weight grids via the cross-multiplied form F_k = W2·N1_k − W1·N2_k
candidate:            admission widening with span-level weight-positivity certificates
input hypotheses:     positive weight field on each admitted span (W(u,v) > 0)
repo representations satisfy them?: partially — WeightCert machinery exists
                      (loft_weights.rs::certify_weight_field; formal/bezier.rs W>0
                      certificate); not composed per-span at SSI admission
same downstream output/proof contract?: yes — the engine's certificate types
                      (KrawczykCertificate3, TraceOutcome) are unchanged
expected mechanism:   admission widening (moves work from typed NumericallyUnresolved
                      into the certified engine; no architectural change)
invasiveness:         small–medium (guard: BREP_SOLVER_AUDIT S5 P0 first — an admission
                      path that can normalize away real weights)
literature:           the cross-multiplied rational form is the classical SSI substrate
                      (Pratt & Geisow, "Surface/surface intersection problems", The
                      Mathematics of Surfaces I, 1986; Sederberg & Parry, "Comparison of
                      three curve intersection algorithms", CAD 18(1), 1986). Weight-
                      positivity discipline as practiced here matches the Bernstein
                      same-sign weight test (Farin, Curves and Surfaces for CAGD,
                      ch. on rational Bézier forms).
uncertainty:          conditioning of deg=8 cross-multiplied systems on real NURBS data
                      is unmeasured; the ConditioningBelowThreshold floor will refuse.
existing runtime evidence: NONE (see ME-1).
```
Prerequisite chain: S5 correction → AD-1 → AD-2 (multi-span) → AD-3
(production registration). This is the highest-leverage admission change in
the tree because the engine substrate is already landed.

## TO-2 — Bernstein domain clipping / projected-polyhedron contractors as stronger exclusion
```text
current transition:   subdivision exclusion inside exclude_five (tangency/exclude.rs),
                      gff cover_branch (contact/gff.rs), ssi_trace certify_box (:588),
                      num/roots.rs
current implementation: interval separation + 4-of-5 Krawczyk subsystem screens, then
                      uniform bisection under Budget; per-side Bernstein hulls (hull.rs)
candidate:           Bézier clipping / projected-polyhedron domain contraction before
                      each bisection: reduce the box by cutting away regions proven
                      empty by Bernstein convex-hull bounds, rather than splitting
input hypotheses:    polynomial (or cross-multiplied polynomial) form in the Bernstein
                      basis over the box — satisfied everywhere in the tree (hull.rs is
                      exactly this substrate; box4.rs exists)
repo representations satisfy them?: yes (polynomial-only discipline; dehomogenization
                      is the consumer's F2 composition)
same downstream contract?: yes — contractors only shrink the search box; every
                      certificate emitted (Krawczyk, exclusion, cluster) is unchanged
expected mechanism:  stronger exclusion / fewer subdivisions per decision
invasiveness:        tiny–small (insert a contractor step in the existing work loops)
literature:          Sederberg & Nishita, "Curve intersection using Bézier clipping",
                      CAD 22(9), 1990; Nishita, Sederberg & Kakimoto, "Trimming
                      algorithms for the intersection of trimmed patches", IEEE CG&A
                      10(5), 1990; Sherbrooke & Patrikalakis, "Computation of the
                      solutions of nonlinear polynomial systems", CAGD 10(5), 1993
                      (projected polyhedron); Mourrain & Pavone, "Subdivision methods
                      for solving polynomial equations", J. Symbolic Comput. 44(10),
                      2009.
uncertainty:         unmeasured; the Phase-1 max latency of 8.22 ms per dispatch call
                      and the Phase-2 budget exhaustion are consistent with a
                      subdivision floor but prove nothing per-op.
existing runtime evidence: NONE per-op.
```

## TO-3 — Ruled/extrusion/revolution pair reductions instead of generic treatment
```text
current transition:  pairs involving extruded/revolved/swept carriers → today: typed
                     refusal (RevolutedCurve unrecognized, recognize.rs:155-160) or the
                     restricted sweep path (ssi4.rs, with recorded P0 defects S1/S2/S6)
current implementation: decorator enclosures exist (decorators/extruded.rs with the
                     C′×V=0 singular-locus rule; revolved.rs) but no pair dispatch is
                     keyed on them (P2/P4 finding: generic path never even reached)
candidate:           exact section/meridian reductions:
                     - extruded×extruded: reduce to planar profile-curve×profile-curve
                       events, lift transversally (dimension reduction R4→R2 per
                       section);
                     - revolution×plane/cone/cylinder: meridian-plane reduction;
                     - ruled×ruled: the classical line-geometry reduction — the
                       intersection locus is ruled; directors from the three director
                       planes
input hypotheses:    exact extrusion vector / rotation axis stored (true in the carrier
                     types: RevolutedCurve carries the rotation matrix; ExtrudedCurve
                     carries V)
repo representations satisfy them?: yes for stored carriers once recognition lands;
                     the singular-locus rule C′(u)×V=0 is already coded in the
                     enclosure decorator
same downstream contract?: partial — outputs would be exact algebraic section curves
                     (admitted by the existing analytic/ ExactCurve vocabulary) rather
                     than traced branches
expected mechanism:  lower-dimensional equations + exact algebraic classification
invasiveness:        medium (recognition + dispatch arms + section-event completeness)
literature:          Farouki, "The characterization of parametric surface sections",
                     Computer Vision, Graphics & Image Processing 33, 1986 (plane
                     sections of quadrics/parametric families); Heo, Kim & Elber, "The
                     intersection of two ruled surfaces", CAD 31(1), 1999 (ruled×ruled
                     director-plane method); Levin, "A quadric-surface intersection
                     algorithm", Computer Graphics (SIGGRAPH), 1979 and related quadric
                     papers.
uncertainty:         high on payoff; unmeasured (structural observation only).
existing runtime evidence: NONE.
```

## TO-4 — Quadric-quadric exact classification to extend the analytic table
```text
current transition:  cone×cylinder, cone×cone, sphere×cone, non-coaxial mixed quadrics
                     → validated FF (gff subdivision + Krawczyk) or refusals; torus
                     pairs → torus_ff quartic composition (cost flagged
                     "measure before committing", TORUS_CONTACT_PROGRAM.md)
current implementation: contact/mod.rs::analytic_ff dispatches 8 exact families; the
                     CFP-004 implicit2d reduction excludes the torus explicitly
candidate:           Levin quadric-quadric analysis: every non-degenerate quadric pair
                     intersection is computed in the pencil basis where one member
                     becomes a canonical (parabolic/rectangular/hyperbolic) case, with
                     a rational parameterization when the pencil contains a degenerate
                     member; near-degenerate pairs get certified ε-canonical routes
input hypotheses:    both carriers quadric with exact coefficients (repo: exact carrier
                     parameters in CanonicalSurface; outward-rounded inari predicates
                     landed)
repo representations satisfy them?: yes for canonical quadrics; partially for placed
                     quadrics (Processor recognition landed)
same downstream contract?: yes — outputs are ExactCurve/CertifiedInterval loci, the
                     existing ContactComplex vocabulary
expected mechanism:  dimension reduction + algebraic simplification; replaces certified
                      subdivision with closed form where the pencil admits it
invasiveness:        small–medium (extend analytic/ family table; reuse AnalyticInter
                     section plumbing)
literature:          Levin 1979 (above); Levin, "Analysis of quadric-surface
                     intersections", CAD 22(4), 1990; Dupont, Lazard, Lazard &
                     Petitjean, "Near-optimal parameterization of the intersection of
                     quadrics I–III", Discrete Comput. Geom. 39–40, 2008 (rationality
                     classification of quadric intersection curves).
uncertainty:         none about correctness of approach; effort is in the degenerate
                     cases (the repo's typed-refusal culture handles this well).
existing runtime evidence: NONE (TOR-A explicitly unmeasured).
```

## TO-5 — Normal-cone separation and tangential-contact specialization
```text
current transition:  near-tangency boxes in gff/cover_branch and singular.rs —
                     subdivision floor then typed unresolved remainder
current implementation: chart-aware 2×2 slab Krawczyk (a certified-nonzero minor picks
                     the chart); normal cones exist as primitives (patch.rs::normal_
                     cone, leaf.normal_cone used by canal.rs:1541-1544) but are NOT
                     used as an exclusion predicate in the SSI/contact loops
candidate:           Gauss-map/normal-cone separation test: if the two patches' normal
                     cones over a box are separated from each other's negation, the
                     box cannot contain an intersection point (gradients would have
                     to be antiparallel); plus Markot–Magedson-style tangential
                     handling for the residual coincident/overlap floor
input hypotheses:    certified normal-cone enclosures over the box (landed:
                     EnclosureSurface::normal_cone, hull-derived derivative bounds)
repo representations satisfy them?: yes
same downstream contract?: yes — pure exclusion predicate; BranchCover vocabulary
                     unchanged
expected mechanism:  stronger exclusion (specifically for the tangency band where
                     Krawczyk's transversal chart fails)
invasiveness:        small
literature:          Sederberg & Meyers, "Loop detection in surface patch
                     intersections", CAD 20(4), 1988 (normal-based loop tests);
                     Markot & Magedson, "Solutions for tangential surface
                     intersections", CAD 23(4), 1991; Grandine & Klein, "A new
                     approach to the surface intersection problem", CAGD 14(2), 1997
                     (subdivision with derivative bounds, the closest published
                     analogue of the present cascade).
uncertainty:         exclusion strength on real near-tangencies unmeasured.
existing runtime evidence: NONE.
```

## TO-6 — Certified mass properties over trimmed Bernstein faces
```text
current transition:  (absent) certified BRep → certified volume/reference facts
current implementation: float divergence theorem over triangles (analyzers/volume.rs),
                     doc-conditioned on mesh closure; facet_sweep signed volume feeds
                     the three-valued verdict only
candidate:           exact integration identity: a polynomial in Bernstein form
                     integrates to the sum of its coefficients scaled by 1/(n+1) per
                     axis — exact in rationals, outward-rounded for a certified bound;
                     Green/divergence reduction over each trimmed span, with
                     boundary (trim-curve) terms for faces whose patches are not
                     divergence-convex; rational faces via one-dimensional weight
                     quadrature certificates (roots.rs machinery) or standard rational
                     integral enclosures
input hypotheses:    trimmed domain with certified boundary curves (trimclip/arrange-
                     landed classes); positive weight certificate for rational faces
repo representations satisfy them?: partially — hull.rs + CertifiedInterval exist;
                     trim boundary certificates exist for the landed carrier classes
same downstream contract?: new op; deliberately decoupled from construction (per the
                     audit contract — a reference-fact op, not a construction step)
expected mechanism:  exact algebraic reduction (no subdivision; one pass per span)
invasiveness:        small–medium
literature:          the Bernstein integral identity is elementary and standard (Farin,
                     Curves and Surfaces for CAGD; conditioning context: Farouki &
                     Rajan, "On the numerical condition of polynomials in Bernstein
                     form", CAGD 4, 1988); Green's-theorem volume/surface-integral
                     formulations for trimmed spline patches are standard mass-
                     properties practice in CAD kernels.
uncertainty:         low for polynomial faces; rational-face boundary terms are the
                     real work.
existing runtime evidence: NONE (op does not exist).
```

## TO-7 — Complete certified seed rays and wire DEF-SEEDRAY-B
```text
current transition:  Boolean fragment seed classification (Select)
current implementation: classify::surface_ray_crossings is plain f64 with NORMAL_SLACK
                     guards; ray_cert.rs is certified but standalone and covers only
                     Plane/Sphere/Cylinder/Cone (recorded P0 finding S7)
candidate:           wire ray_cert into classify (machinery, not math) + add Torus and
                     sweep-carrier certified crossing solves; ray direction chosen so
                     the crossing count is decided away from tangency (normal-aligned
                     pick with a certified margin)
input hypotheses:    ray×carrier crossing decidable with outward-rounded arithmetic
repo representations satisfy them?: yes for the 4 landed carriers; torus needs the
                     quartic-in-z reduction (same substrate as TOR-A)
same downstream contract?: yes — seed bits only; parity graph unchanged
expected mechanism:  certification primitive (removes the recorded P0)
invasiveness:        small (wiring) / medium (torus + sweep solves)
literature:          none needed beyond existing interval-geometry practice; torus
                     quartic composition is the TOR-A booked item.
existing runtime evidence: NONE (debug fixture-scale µs-class samples only).
```

## TO-8 — Evidence transport under exact placements (Map/instances)
```text
current transition:  Map(transform, object) — transports geometry, not evidence
current implementation: canonical.rs:374-434 exact-placement doctrine (identity 3×3
                     keeps carriers; else Processor wrap); Processor interval
                     transform landed (decorators/processor.rs); no certificate
                     transport
candidate:           scoped transport rules: rigid → transport certificates verbatim
                     (enclosures/krawczyk boxes are affine-equariant); reflection →
                     orientation-affected claims flip, magnitude claims transport;
                     uniform scale → enclosure scaling by |s|; nonuniform → refuse
                     transport, re-derive (metric-dependent claims do not survive)
input hypotheses:    exact representability of the transform (the exact_deck_shift fma
                     discipline in kernel/identity.rs:194-213 is the precedent)
repo representations satisfy them?: partially — the Margin/Modulus algebra and
                     UseSite/EvidenceRequirement machinery (formal/evidence.rs) give
                     the scoping substrate; transport theorems are absent
same downstream contract?: yes — consumers already demand claims via
                     AuthoritativeFact; transport mints nothing new
expected mechanism:  reuse (P6); eliminates repeated discovery for instances
invasiveness:        medium
literature:          standard equariance of interval/Bernstein bounds under affine
                     maps; the scope discipline matches the existing
                     formal/evidence.rs UseSite design.
existing runtime evidence: NONE.
```

## TO-9 — Implicitization/resultant machinery (last-resort replacement)
```text
current transition:  procedural/intersection carriers and cross-degree spline pairs
                     → typed refusal or generic subdivision
candidate:           implicitization via resultants (Dixon/KS) or Sederberg-style
                     matrix implicitization to obtain exact implicit witnesses for
                     the realized locus, consumed by the existing implicit.rs /
                     implicit2d.rs interval machinery
input hypotheses:    rational parametric carrier of bounded bidegree (MAX_BIDEGREE=8
                     admission already bounds degree); nonzero weight field
repo representations satisfy them?: representation-wise yes; machinery-wise no
                     (no resultant code exists in the tree)
same downstream contract?: partial — an implicit witness is a different carrier type;
                     BREP_SYSTEM.md §4 already anticipates "implicit algebraic
                     witnesses rather than another explicit NURBS curve", so the
                     vocabulary exists, consumers must admit it
expected mechanism:  dimension reduction for specific pair classes; large machinery
                     cost (degree blowup of implicitizations is the classical hazard)
invasiveness:        large
literature:          Sederberg, Anderson & Goldman, "Implicit representation of
                     parametric curves and surfaces", CVGIP 28, 1984; Dixon resultant
                     literature standard.
existing runtime evidence: NONE.
recommendation:      defer; TO-2/TO-4/TO-5 dominate it for the landed carrier classes.
```

---

## Ranking rationale

1. **TO-1 / AD-1** first: the only gap where the expensive machinery is fully
   landed and a narrow admission gate holds it back; unlocks the corpus's
   NURBS class without touching the solver.
2. **TO-2** is the cheapest cross-cutting win (a contractor step in existing
   loops) and directly attacks the one measured pathology pattern (budget
   exhaustion → typed refusal).
3. **TO-7** removes a recorded P0 with mostly-wired machinery.
4. **TO-4/TO-3** convert the most common non-admitted classes (cone pairs,
   extruded/revolved carriers) into exact reductions, matching the tree's
   proven analytic-family style.
5. **TO-5/TO-6** are quality upgrades with clean contracts.
6. **TO-9** stays deferred.

---

## Final summary matrix

Implementation / Certification / Semantic closure / Completion guarantee /
Existing runtime evidence / Primary gap. Closure kinds per §4 of the audit:
semantic = proofs compose if transitions succeed; completion = proved
termination envelope with sufficient resources.

| Transition / class | Implementation | Certification | Semantic closure | Completion guarantee | Existing runtime evidence | Primary gap |
|---|---|---|---|---|---|---|
| Generate: primitives / extrude / revolve | LANDED | LANDED (typed refusals, carrier tables) | Yes, per admitted class | None | NONE | ME-1 (no measurements); primitive set narrow |
| Generate: spine/frame + FAC | LANDED | LANDED (FacetVerdict, ConstructError) | Yes within tolerance contract | None (ChordTolerance variants SPEC) | NONE | ME-1; SamplingPolicy follow-up |
| Generate: loft family | LANDED | LANDED (LoftValidityCert; degeneracy arm PARTIAL) | Partial (collapse/apex) | None | NONE | TP-1 |
| Map (transform + evidence) | ABSENT | — | — | — | NONE | MP-1 |
| Interact: analytic pairs | LANDED (production funnel) | LANDED (exact expansions/inari) | Yes per admitted class | Yes for those classes (closed-form) | MEASURED (dispatch 6.4 µs median, 8.22 ms max) | AD-4 (dual dispatchers) |
| Interact: spline×analytic SSI | LANDED engine; PARTIAL wiring | LANDED (Krawczyk3, hulls) | Partial (unit weight, 1 span, plane) | None | PARTIAL (Phase-2 whole-run only) | AD-1/AD-2/AD-3 |
| Interact: spline×spline trace + stitch | LANDED (test-only callers) | LANDED (trace discipline, germ ladder) | Partial | None | PARTIAL (budget-exhausted wall) | WR-1 |
| Interact: tangency cascade (A1/A2) | LANDED machinery; wiring PARTIAL/ABSENT | LANDED (test-gated A2; stage 6 unreachable) | Partial | None | NONE | CT-2, CT-1, CT-3 |
| Interact: singular/coincident strata | PARTIAL (gff/singular; arrange axis-aligned only) | PARTIAL (τ-widening deviation documented) | Partial | None | NONE | CT-3, S-records |
| Interact: self-interaction (R6) | LANDED machinery, PARTIAL integration | LANDED (test-wired) | Partial | None | NONE | WR-1 |
| Refine: kernel trimclip/atlas | LANDED (packet/test-wired) | LANDED (R9, winding, OffLoopCert) | Partial (collapsed boundaries) | None (budgets) | NONE | WR-1, TP-1, TP-2 |
| Refine: formal arrangements (production) | LANDED | LANDED (Jordan, homology, Fourier winding) | Partial (holes route 0 faces) | None | NONE | TP-4 |
| Select: Boolean (shapeops) | LANDED (test/probe + facade routing) | PARTIAL (S7 seed rays; S6 sweep assumptions) | Partial (single shell; S8 re-entry) | None | NONE | SR-1, SW-1, WS-1 |
| Select: formal material classification | LANDED | LANDED | Partial | None | NONE | TP-4 |
| Identify: Rules A/B/C, deck, quotient, provenance | LANDED | LANDED (exact, refuse-not-snap) | Partial (domain-layer prototypes) | n/a | NONE | ID-1 |
| Realize: tessellation (production) | LANDED | LANDED (MeshedShellOutcome, FaceValidityCertificate) | Partial | None | MEASURED (per-face/stage; UR10 fix) | ID-1, ME-1 |
| Realize: FAC | LANDED | LANDED (RealizationVerdict; assemble test-wired) | Within tolerance | None | NONE | ME-1 |
| Realize: output kinds | SPEC ONLY | — | — | — | NONE | TL-1 |
| Mass properties (certified) | ABSENT | ABSENT | — | — | NONE | TP-3 (TO-6) |
| Evidence transport (O7) | ABSENT (formal scoping LANDED) | PARTIAL (Analytic-only authority) | — | — | NONE | PG-1, MP-1 |
