# Open theory problems — admission coverage and non-transversal contact

*For a frontier model with no code access. Companion to
`docs/SLOW_SOLVER_DIAGNOSTICS_R4.md` (which specifies the certified boolean
algorithm whose semantics these problems must preserve). Notation and
soundness definitions carry over unchanged from that document, §2–3.*

---

## Problem 1 (the coverage frontier beyond mechanical widening): certified booleans on coincident-support compositions

### 1.1 The obstruction, formally

Let `A`, `B` be solids whose boundaries are finite unions of patches
`P : [0,1]² → R³` in Bernstein form (§2 of the companion document). The
admission gate requires a strictly positive certified separation:

```
σ = min over non-excluded patch pairs (p,q) of σ_min(p,q) ≥ τ > 0
```

where `σ_min(p,q)` is a computable lower bound on the Euclidean distance
between the two surface pieces. The production compositions that refuse
today (`spline_loft × spline_loft`: monocoque, sidepods, airbox, chassis,
body, …) violate this hypothesis **structurally, not marginally**: the
fused solids are designed to *abut* — they share planar interface faces, so
there exist sets `S ⊆ dom(p) ∩ dom(q)` of positive two-dimensional measure
with

```
p|S = q|S   (exact coincidence of support, or coincidence up to the
             modeling tolerance ε_M with a provably planar interface)
```

For such pairs `σ_min = 0` and no threshold choice admits them: the
transversality hypothesis is not merely violated, it is **unrecoverable at
any refinement depth**. Meanwhile the boolean itself is benign — coincident
support is the *easy* case geometrically (the intersection volume is
supported on a lower-dimensional locus or cancels exactly) — it is only the
certificate machinery's separation hypothesis that fails.

### 1.2 What is needed

A **certified coincident-support calculus**: an extension of the boolean
certificate that admits pairs with exactly-coincident (or
certificate-planar-abutting) support, producing the same sound bracket,
without subdivision to infinite depth. Concretely, one or more of:

**(a) Exact-cancellation certificates.** In the flux formulation, the
intersection volume integrates the divergence form
`g_P = (1/3) P·(P_u × P_v)` over the boundary of the compound solid. When
two patches coincide with opposite orientation along a shared interface
`S`, their flux contributions cancel **exactly** in the real-arithmetic
sum. The needed theory: a *machine-checkable coincidence certificate* —
decidable, sound under outward rounding — for "these two patch regions
represent the same supporting surface with opposite orientation"
(sufficient condition: both are planar with identical supporting plane,
provable from Bernstein control data by interval evaluation of the plane
equation over each control hull; the overlap region must be computable as
an exact polygonal decomposition of the two parameter rectangles). Given
the certificate, the pair contributes **zero flux** by cancellation and is
removed from the refinement queue. Prove: the cancellation preserves
soundness (the discarded pair's true contribution is exactly zero, so the
interval sum remains a valid enclosure).

**(b) Lower-dimensional contact reduction.** Where coincidence is partial
(an edge or curve of contact rather than a face), the intersection-volume
integrand is supported on a set of measure ≤ 1 in parameter space. The
needed theory: a certified reduction showing the sub-cell refinement of
§4 phase 3 terminates after O(1) levels when the contact set is a provable
curve (detected by interval evaluation: one transverse direction is
provably separated), replacing the current measure-2 refinement with a
1-dimensional certified integration along the traced curve.

**(c) A transversality substitute for degenerate contact.** Replace the
scalar separation hypothesis `σ ≥ τ` with a *structural* admission
predicate that admits configurations provably free of *true tangency*
(osculation), even under coincidence: e.g. a certificate that at every
nearby point of the contact set, the two surface normals are oppositely
oriented and the relative position function changes sign across the
interface. The required output is the predicate's decision procedure
(interval-evaluable on control data) and the soundness proof that
admitted configurations yield convergent, finite certified brackets.

**Acceptance criterion (production):** the nine `spline_loft × spline_loft`
rows (monocoque, sidepods ×2, airbox, chassis, body, drivetrain, details,
floor) construct with sound brackets under 120 s; no currently-green row
changes verdict; `TransversalityUncertified` refusals remain for genuinely
tangential configurations.

---

## Problem 2 (the per-class lemma obligations): certified separation bounds for the five narrow carrier pairs

### 2.1 The general form

For a carrier-pair class `(𝒞₁, 𝒞₂)` to enter the funnel, the machinery
needs, for every patch pair `(p, q)` with `p : 𝒞₁`, `q : 𝒞₂` surviving the
control-hull broad phase:

1. **A computable certified separation bound** `σ_min(p,q) ≤ dist(p,q)`:
   an interval-evaluable function of the control data that never
   *overestimates* the true distance (one-sided error toward refusal), so
   the gate `σ ≥ τ` is sound;
2. **Membership coverage**: the certified point-classification primitive
   (§3.2) must resolve points against both surface classes with the
   Inside/Outside/Ambiguous contract;
3. **A width-convergence bound** for the refinement on this class: the
   certified flux bracket over a contact cell must contract at a known
   rate so the width budget terminates.

These are the "lemmas through the gate" each widening must carry. For the
five narrow pairs the surface classes are already certified
individually — the obligations are the *pairwise* bounds and their
composition.

### 2.2 Per-class statements

**(a) `lathe × cylinder`** (wheels: a surface of revolution cut by a
circular cylinder; 4 production rows). The revolute patch is
`P(u,v) = R_z(θ(u)) · ρ(v)` — a profile curve `ρ` revolved by rotation
`R_z`. Needed: the separation bound combining the profile's certified
control polygon (2-D, in the meridian plane) with the cylinder's axis-range
argument — for coaxial configurations this reduces to a planar
profile-vs-rectangle problem in the meridian half-plane; for general
placements, a certified bound from the rotation group's isometry (distances
are preserved per angle, so `σ_min` factors through the planar bound).
Deliverable: the planar certified bound + its lifting lemma.

**(b) `trim_prism × trim_prism`, `section_prism × section_prism`,
`spline_loft × box`** (prismatic classes). Prisms are extrusions of planar
profiles: `P(u,v) = γ(u) + v·d`, `γ` a planar Bernstein curve, `d` the
direction. Needed: a certified 2-D separation bound between the profile
curves' convex hulls lifted along the extrusion direction — the 3-D
separation reduces to a 2-D problem in the plane orthogonal to `d` when
directions are parallel (prove the reduction), and to a swept-hull argument
when they are not. The box class is the trivial base case of the prism
class (rectilinear profile), so one lemma family covers all three.

**(c) `cylinder × cylinder`** (nose fuse). Canonical-vs-canonical cylinder
pairs: coaxial and non-coaxial sub-cases. Coaxial: the annulus/radial
separation argument (certified inner/outer radius intervals). Non-coaxial:
the classical distance-between-axis-segments bound lifted by radius
intervals — needed as a certified (interval) version with one-sided
rounding. This is the smallest lemma but it is the gate's canonical
example; it must compose with the placement algebra (isometries preserve
the bound exactly).

### 2.3 Acceptance criterion

Each class ships as: the certified bound (interval-evaluable, one-sided),
the soundness proof sketch in the packet's `RESULT.json`, one
ground-truth test per class with exact analytic volumes, and the
production row flipping from typed refusal to constructed (or to a
narrower typed refusal if the specific instance genuinely fails
transversality — which for these classes it should not, as they are
designed-cut geometries).

---

## Priority and interaction

Problem 2's classes are packet-sized (mechanical widening; two packets
already booked: `FHC-G16-ADMISSION-PAIRS-NARROW`,
`FHC-G17-TRIM-PRISM-EXTRUDE-ADMISSION`) — the theory above is what the
worker verifies rather than invents. **Problem 1 is the genuine theory
gap**: it gates nine production rows and interacts with the speed frontier
(`docs/SLOW_SOLVER_DIAGNOSTICS_R4.md` §5.1) — the abutting-interface
configurations that refuse today under Problem 1's absence are the same
configurations whose *weakly-transversal* neighbors dominate the refinement
cost. A solution to (a)/(b)/(c) likely collapses both fronts at once.
