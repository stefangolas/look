# Proof obligations: certified boolean volume for tangentially contacting spline patches

**Purpose.** A self-contained formal problem statement for a prover
(frontier model or human). The task: discharge the proof obligations
O1-O8 below. Each is stated with explicit hypotheses, conclusion, and
what counts as a discharge. Definitions are self-contained; no
codebase access is assumed. A discharge is a rigorous proof — sketches
already exist and are included where they help — with every genericity
condition explicit and every asymptotic remainder replaced by an
enclosed, quantified bound.

**Global conventions.** All computations are performed with outward-
rounded interval arithmetic; any bound stated as O(·) in a *certificate*
must be replaced by an explicit enclosure. No tolerance-based decisions
anywhere: thresholds only select which proof applies.

---

## 0. Definitions and standing hypotheses

**Patches.** A *patch* is a map `P : [0,1]^2 -> R^3`,
`P(u,v) = sum_i sum_j c_ij B_i(u) B_j(v)` — a bidegree-(3,3)
tensor-product Bézier patch (degree raised as needed), with control
points `c_ij in R^3` given exactly (rational or dyadic) and positive
weights `w_ij` (rational), i.e. a rational tensor-Bernstein patch in
homogeneous form. All partial derivatives up to order 3 exist and are
polynomial (rational) on the closed domain.

**Cells and subdivision.** A *cell* is a product of intervals
`U = I x J ⊂ [0,1]^2`. Subdivision splits a cell by de Casteljau
(exact: child control points are convex combinations of parent control
points).

**Normal cones.** For a patch P and cell U on which
`n_P(u,v) = (P_u x P_v)(u,v)` satisfies `||n_P|| >= mu_P > 0` (a
certified, positive interval bound), the *certified unit normal cone*
is a cone `C_P(U) = cone(a, theta_P)` with axis unit vector a and
half-angle `theta_P in [0, pi)` such that the unit normal
`n_P/||n_P|| in cone(a, theta_P)` for all `(u,v) in U`. Such a cone is
computable by outward-rounded interval evaluation of the cross product.

**Regular cell.** U is *P-regular* if such `mu_P > 0` is certified on U.

**Coincidence and the gap function.** For two patches P, Q with cells
U, V, the coincidence set is
`C = {(u,v) in U x V : P(u) = Q(v)}`. In the tangential regime
(Definition D below) a is the common centre normal and the *gap
function* is the scalar `h = a . (P - Q)` on the projected overlap.

**The Boolean volume target.** For solids A, B whose boundaries are
finite unions of patches (an oriented 2-cycle each), the goal is a
certified bracket `[V_lo, V_hi] contains Vol(A \ B)` (and Vol(A ∪ B)),
with `V_hi - V_lo <= eps` for a requested budget eps, or a typed
refusal. The *existing certified machinery* (given, not to be re-proved)
computes, for any region of the boundary where the inside/outside
classification against the other solid is decided, the exact flux
integral `(1/3) ∮ x . n dS` over that region, and composes such
contributions additively into a bracket whose width is exactly the
total undecided-measure contribution.

**Regime dichotomy (admission, taken as given by O1).** A pair of
cells (U, V) is in the *transversal regime (T)* or *tangential regime
(G)* according to Lemma D below. In regime (G), both P|U and Q|V are
graphs over the plane `Pi = a^perp` (a the common centre normal):
there exist functions `z_P, z_Q : Pi -> R` with
`P(u,v) = (pi(P(u,v)), z_P(pi(P(u,v))))` for `pi` the orthogonal
projection onto Pi, and similarly Q.

**Graph injectivity (G-INJ, taken as given by O2).** pi restricted to
P(U) is injective, and likewise for Q(V).

**Undecided pair.** A (G) pair with G-INJ whose certified enclosure of
h on `R = pi(P(U)) ∩ pi(Q(V))` contains 0. Pairs whose h-enclosure
excludes 0 are *resolved* (disjoint or separated there).

---

## O1 — The regime dichotomy (Lemma D)

**Hypotheses.** U, V are P- and Q-regular cells with certified normal
cones `cone(a, theta_P)`, `cone(b, theta_Q)`, half-angle sum
`Theta = theta_P + theta_Q < pi/4`, and `phi = angle(a, b)`.

**Claim.** At least one of the following holds:

- **(T)** `phi - Theta > 0` and `phi + Theta < pi`. Then no normal of
  P|U is parallel (or anti-parallel) to any normal of Q|V.
- **(G)** `phi + Theta < pi/2` or `phi - Theta > pi/2`. Then every
  normal of P|U and every normal of Q|V satisfies `|n_P . n_Q| > 0`,
  and both P|U and Q|V are graphs over `Pi = a^perp` (for the second
  disjunct, Q is a graph over Pi with inverted height orientation).

**Discharge requires.** A complete proof of the three-case split
(`phi <= pi/4`, `pi/4 < phi < 3pi/4`, `phi >= 3pi/4`), including: the
angle-interval inclusion `[phi - Theta, phi + Theta]` for all normal
pairs (spherical-geometry statement about two cones); the graph
property from "all normals within pi/2 of a"; careful treatment of the
strict inequalities at case boundaries; and the statement that
`(T) ∧ (G)` overlap is nonempty (shallow crossings) with the tie rule
"prefer (T)" consistent — i.e., any region certifiable under both
regimes yields agreeing verdicts.

**Note.** A proof sketch exists and is believed complete; what is
wanted is a written proof with the spherical-geometry lemma about cone
pairwise angles made explicit (the inclusion
`angle(n_P, n_Q) in [max(0, phi - Theta), min(pi, phi + Theta)]` needs
justification for cones of possibly unequal half-angles).

---

## O2 — Graph injectivity (G-INJ)

**Hypotheses.** U is a convex cell; every matrix in the interval hull
`J` of the 2x2 Jacobian `D(pi ∘ P)` over U is nonsingular (where the
Jacobian is taken in local orthonormal coordinates of Pi).

**Claim.** `pi ∘ P` is injective on U.

**Discharge requires.** The mean-value/integral argument:
`(pi∘P)(x) - (pi∘P)(y) = M(x-y)` with `M` in the interval hull of the
Jacobian along the segment (convexity of the cell), plus a *uniform*
nonsingularity argument: nonsingularity of every element of a compact
hull gives `|det| >= delta > 0` on the hull, hence a uniform bound on
`||M^{-1}||`, hence injectivity. State precisely which norm and why
compactness applies. Also state and prove the converse-flavored
warning: nonsingularity on the hull is sufficient but not necessary —
and record the counterexample family (spiral ramps with near-vertical
normals everywhere: all (G) hypotheses hold, injectivity fails), so
the obligation is not vacuous.

---

## O3 — The sandwich lemma (Lemma S) — the central obligation

**Hypotheses.** A collection of (G) pairs with G-INJ, each either
resolved (h-enclosure excludes 0 on R) or undecided. `S*` is the true
set-difference solid A \ B (A, B = the solids bounded by the patch
2-cycles). Define the candidate solid `S~`:
- outside the cylinders `R x R·a` (the vertical cylinders over the
  projected overlaps R of undecided pairs), `S~` agrees with the true
  boundary classification, which is decided there;
- inside each such cylinder, `S~`'s boundary is a single graph (either
  z_P or z_Q, the choice fixed by certified orientation), closed by
  vertical walls over the boundary of R.

**Claim (S-main).** The symmetric difference `S* Δ S~` is contained in
the union of the sandwich regions
`{x in R x R·a : x lies between the two graphs z_P and z_Q}`.
Consequently
`|Vol(S*) - Vol(S~)| <= sum_undecided |R| . sup_R |h|`,
and therefore a certified bracket for `Vol(S*)` is obtained by
computing `Vol(S~)` (or its flux) exactly on the decided part and
adding/subtracting the sandwich bounds.

**Discharge requires.**
1. The membership lemma: inside a (G) cylinder with G-INJ, a point's
   membership in A (resp. B) is decided exactly by its side relative
   to the graph z_P (resp. z_Q) — including the inverted-orientation
   case of O1's second (G) disjunct.
2. The symmetric-difference containment: a point where membership in
   S* differs from S~ must have its deciding graphs straddle it. Use
   Fubini/coarea over Pi: the volume of each sandwich region equals
   the integral over R of |h|, bounded by |R| sup|h|.
3. Rigorous treatment of the cylinder walls and of the boundaries ∂R
   (measure-zero sets under G-INJ: projected cell images overlap only
   on boundaries).
4. **OBL-S2 (coverage of ambiguity):** every boundary point of A whose
   classification against B is undecided lies in some sandwich
   cylinder. This is the statement that the pair-cover's undecided
   cells cover all classification ambiguity; prove it from the
   subdivision structure (cells partition the parameter domains) and
   G-INJ.
5. **OBL-S3 (no third-face interference):** if any other face of A or
   B passes through a cylinder's interior sandwich, the bound is
   invalid for that cell — the algorithm must split or refuse. Formalize
   the detection condition ("some third patch's projection intersects R
   and its height range meets the sandwich") and prove the bound holds
   for all cells that pass it.

**Known pitfall to address head-on.** The sum `sum |R| sup|h|`
requires the projected overlaps to not double-count: with G-INJ and
disjoint cells, the images `pi(P(U))` overlap only on boundaries
(measure zero), so the total projected area is additive. Prove this
covering lemma.

---

## O4 — Termination (Lemma T)

**Hypotheses.** The range enclosure of h on a cell of width w has
width `omega(w) -> 0` as `w -> 0`, uniformly (the enclosure function
converges — e.g. Bernstein convex-hull bounds with order 2, or
Lagrange/Hermite range forms with their published orders).

**Claim.** For any eps > 0 there is a subdivision depth such that
`sum_undecided |R| sup|h| <= eps`. I.e. the sandwich schedule
(subdivide the max-|R|·sup|h| pair) terminates.

**Discharge requires.** The covering lemma from O3 (total projected
area additive, bounded by the total face area A), plus
`sup_undecided (enclosure width) -> 0` under uniform refinement, giving
`sum <= A . max width -> 0`. Handle: the number of undecided cells
growing under subdivision (cells may split into several undecided
children — the bound must survive the count growth because it is
area-weighted); cells whose R is empty; the distinction between
parameter width w and geometric width under G-INJ (the Jacobian hull
gives a Lipschitz bound relating them).

---

## O5 — Rate with second-order enclosures

**Hypotheses.** Near a tangential contact that is A1 (isolated) or A2
(Morse-Bott curve, nondegenerate quadratic degeneracy), the gap
function satisfies, on cells of width w meeting the contact set:
`sup|h| <= mu w^2 / 2 + (enclosed remainder)`, with `mu` a certified
bound on the second derivatives of h (difference of second fundamental
forms).

**Claim.** With second-order range enclosures:
- a tangential contact curve of length L contributes
  `≈ L . mu . w^3` to the sandwich sum at width w;
- an isolated tangency contributes `≈ mu . w^4`;
- consequently the schedule's width-vs-work follows the stated rates.

**Discharge requires.** A tubular-neighborhood / Morse-coordinates
argument around the contact stratum, with the remainder terms replaced
by *explicit enclosures* (the certificate may only contain quantified
remainders). State the genericity hypotheses on the contact exactly
(Hessian definiteness/indefiniteness modulo the curve, transversality
of the stratum). If the hypothesis set needed is stronger than "A2
nondegenerate", state the minimal version.

---

## O6 — Knowledge lift (OBL-K)

**Hypotheses.** The sandwich construction of O3 over ALL undecided
pairs, with resolved cells decided pointwise.

**Claim.** For the purpose of `Vol(A\B)` (and `Vol(A∪B)`), no
component of the true symmetric difference can escape accounting:
every point of `S* Δ S~` lies in a sandwich (this is O3), so the
bracket is valid *without enumerating the connected components* of the
contact or of the result. Hence the solver may report
"all-components knowledge for volume purposes" over the covered
region.

**Discharge requires.** Making precise in what sense component
enumeration is unnecessary for the *volume* claim while it would be
necessary for *topology* claims; i.e., prove the bracket-validity
statement using only O3's containment, and exhibit (by counterexample
or argument) which stronger statements (component counts, closedness
of contact curves) do NOT follow.

---

## O7 — Non-degeneracy of the collinear-normal system

**System.** For patches P, Q, define
`F : R^4 -> R^4`,
`F(u,v,s,t) = ( (P-Q).P_u , (P-Q).P_v , Q_s.(P_u x P_v) , Q_t.(P_u x P_v) )`.
Zeros of F are pairs of points joined by a common normal line
(critical points of the height gap h in any common-normal direction).

**Claim.**
(a) On a transversal contact curve (normals non-parallel), F has no
zeros in a neighbourhood of the curve (the last two equations fail
there).
(b) At an isolated tangency of Morse type (definite difference of
second fundamental forms, nonzero gap Hessian), `(u0,v0,s0,t0)` is a
simple root of F: `DF` has full rank 4.
(c) At a Morse-Bott tangential curve, the root set of F is a
1-dimensional manifold near the point (`DF` rank 3), and the rank
drops precisely for the higher-degeneracy reasons catalogued as
`HighOrderJet`.

**Discharge requires.** An explicit computation of `DF` at a zero,
expressing it in terms of the two surfaces' first and second
fundamental forms in the common tangent frame, and a rank
characterization in terms of the difference form's definiteness and
the normal-map differential. This is what licenses Krawczyk
certification of isolated tangencies (rank 4) and the Krawczyk-failure
signature of tangent curves (rank 3). Any additional genericity
condition (e.g. non-inflection, distinct principal directions) must be
stated as a hypothesis and made checkable by interval evaluation.

---

## O8 — The floating-point floor

**Claim.** With outward-directed rounding at unit roundoff u and
geometry of scale s, range-enclosure widths of the gap h stop
shrinking below `c . u . s` for a modest constant c depending only on
the range-function form (control-net slabs: exactly the evaluation
error of degree-(3,3) convex sums; Hermite forms: sum of jet
evaluation errors). Consequently, for coincident carriers without an
exact-evidence certificate, the achievable bracket width is at least
overlap_area x c·u·s, and a refusal `CoincidenceWithoutExactCarrier`
with this computed floor is the strongest possible statement.

**Discharge requires.** An error-accounting proof for the specific
range forms used (which are: Bernstein convex hull; Hermite/Lagrange
range forms), in directed rounding, to the stated form. Standard
techniques; the value is the explicit constant and its independence
from the subdivision schedule.

---

## 1. What a discharge looks like

For each obligation: a complete written proof at the level of a
computational-geometry journal, with (i) all hypotheses explicit and
checkable by interval evaluation on the inputs, (ii) every remainder
in a certificate replaced by an explicit enclosure, (iii) every
genericity condition stated and shown decidable, (iv) counterexamples
where claims fail (O2's spiral ramp; O3's third-face interference;
O7's higher-order contacts) used to show the hypotheses are not
vacuous. Priority order if effort must be allocated: O3 (with S2, S3)
> O4 > O7 > O6 > O2 > O5 > O1 > O8. O3 and O4 together are what make
the volume goal close on tangential inputs; O1 and O2 are the
admission gates those hypotheses consume; O5 and O8 only affect the
cost model and the floor; O7 gates the topology tier, not the volume
tier.

## 2. Literature anchors (already vetted as starting points)

Sederberg–Meyers, *Loop detection in surface patch intersections*
(CAGD 1988) — the collinear-normal criterion. Kriezis–Patrikalakis–
Wolter (CAD 1992) — oriented distance functions and implicit
representation of intersections. Hohmeyer (IJCGA 1991) — Gauss-map
separability for loop detection. Plantinga–Vegter (SGP 2004) —
interval-based isotopic approximation with similar
subdivision/injectivity structure. Schulz (CAGD 2009) — quadratic
convergence of Bézier clipping. Bartoň–Jüttler (CAGD 2007, MCS 2011)
— quadratic clipping rates for univariate and bivariate systems. Liu
et al. (CAGD 2009) — cubic clipping. Burr–Choi–Galehouse–Yap (JSC 2012)
— certified treatment of singularities for algebraic curves.
Dupont–Lazard–Lazard–Petitjean (JSC 2008) — exact quadric intersection
classification. Cheng et al. (TOG 2023) and Yang–Jia–Yan (TOG 2023) —
modern SSI topology enumerations to cross-check the case analysis.
