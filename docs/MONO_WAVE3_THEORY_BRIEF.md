# Certified booleans over spline-carried solids — theory brief for frontier development

**Status:** open problem statement for the MONO-CLOSURE wave-3 gate. Every
mechanical component named below is ALREADY LANDED in this repository; what is
requested is the development of the certified-intersection theory (proofs +
algorithm + termination argument) described in sections 3-5. Do not redesign
the landed machinery; consume it through the stated interfaces.

---

## 1. Problem statement (formal)

Let A, B be bounded solids in R^3 whose boundaries ∂A, ∂B are finite unions
of **tensor-product Bernstein patches of bidegree (3,3)** (obtained by exact
degree elevation from the landed spline carriers), each patch
`P: D_P = [u0,u1] x [v0,v1] -> R^3` polynomial-rational of total coordinate
degree <= 3 in each parameter, with the patch family forming an **oriented,
closed, piecewise-C2 2-cycle** (corners/creases allowed; they are
measure-zero edges shared consistently by adjacent patches).

Goal: compute a rigorous bracket `[V_lo, V_hi]` with

```
V(A \ B) ∈ [V_lo, V_hi],   V_hi − V_lo <= ε
```

where ε = 1e-4 relative to the target solid's recorded reference volume (the
facts gate band), **or** return a typed refusal when the pair is not in
general position (section 5). No approximate-only path exists: every numeric
output is either a certified bracket or a typed refusal. Sampling, Monte
Carlo, and tolerance-stretched verifications are all forbidden by the loop's
doctrine.

## 2. Landed machinery the solution must consume (do not reinvent)

| component | what it provides | where |
|---|---|---|
| `volume_facts(VolumeRow)` | rigorous bracket ∫_P x·n dS / 3 for one tensor-Bernstein patch (exact Bernstein-coefficient integration; weights + orientation fields) | the sanctioned plain-data entry behind the pyo3 `binding_volume_facts` export |
| FSSI-001 transversality gate | decidable general-position certificate machinery (Theorem C mechanism) | committed spec + code |
| 2-D implicit-pullback fast path | the mechanism for reducing a surface-pair intersection system to a 2-D certified problem | `docs/TORUS_CONTACT_THEORY.md` (proven for torus x plane; to be generalized) |
| SEEDRAY classification | certified point-vs-solid membership | landed (DEF-SEEDRAY-A) |
| interval discipline | outward-rounded interval arithmetic (inari) is already in the tree; the predicate discipline forbids naked f64 decisions | loop-wide |
| Theorem D / ADM lemma wave | per-patch volume certification, extraction, product, normal cones | landed (ADM-L1..L5) |

Interface law: the certificate code consumes **plain data** (patch control
nets, weights, orientation flags) exactly as `volume_facts` does. No kernel
type crosses the boundary.

## 3. The reduction (the accounting theorem — must be proven)

**Lemma 1 (measure-zero split).** For transversal oriented solids A, B as in
section 1:

```
V(A ∩ B) = (1/3) [ ∫_{∂A ∩ int B} x·n_A dS  +  ∫_{∂B ∩ int A} x·n_B dS ]
```

The intersection curve ∂A ∩ ∂B contributes nothing (2-dimensional measure on
the boundary). This is classical, but the proof must be carried in the
piecewise-C2 / oriented-2-cycle setting with crease corners, because that is
what the corpus's solids actually are (the tub skin has a hard crease line).

**Theorem 2 (no-trim accounting).** Define for each patch P of ∂A the trim
region R_P = { p ∈ D_P : P(p) ∈ int B }. Then

```
∫_{R_P} x·n_A dS  is bracketed by
[ ∫_C w⁻ ,  ∫_{D_P \ O} w⁺ ]
```

where w = x·n_A expressed in (u,v) (a POLYNOMIAL of component degree <= 6 —
degree-(3,3) position times the cross product of degree-(2,3)x(3,2)
partials; exact Bernstein degree elevation applies), and C ⊆ R_P
(certified-inside union of boxes), O ⊆ D_P \ R_P (certified-outside union of
boxes) come from the curve enclosures of section 4, with the uncovered band
B_P = D_P \ (C ∪ O) satisfying area(B_P) <= δ · L_P (δ = enclosure width,
L_P = certified bound on the total length of the curve pieces in P).

**Corollary 3 (δ → ε budget).** Total bracket width <= Σ_P W_P · area(B_P)
with W_P = max w − min w over the band (interval bound). Since
area(B_P) <= 2 δ L_P, the enclosure fineness required is

```
δ <= ε / ( Σ_P 2 W_P L_P )
```

— an explicit, checkable pre-computation from interval bounds alone. The
developed theory must state this inversion as a theorem with the constants
made rigorous.

**Why this matters:** the accounting never requires reconstructing trimmed
spline patches. The full-patch integrals go through the landed
`volume_facts`; the corrections are exact Bernstein integrals over boxes
minus band-area bounds. The entire difficulty is concentrated in section 4.

## 4. Certified curve enclosure (the core development)

For each patch pair (P, Q) (P from ∂A, Q from ∂B) define
`F : R^4 → R^3, F(u,v,s,t) = P(u,v) − Q(s,t)`. The intersection curve is
F⁻¹(0) (generically a 1-manifold). Develop:

**4.1 (Exclusion).** Bernstein range enclosure of F over a 4-D box: if
0 ∉ F(box), the curve does not meet the box. Prove: (a) the range bound
converges under bisection (hull property), (b) **termination for every box
disjoint from the curve** — for transversal intersections the curve has
positive distance from such boxes, and the interval extension's overestimation
tends to zero; make this quantitative (the distance-to-curve vs subdivision
depth relation, with the constants from the patch Lipschitz bounds).

**4.2 (Inclusion — the implicit step).** Where some 3-column minor of the 3x4
Jacobian J_F is invertible over the box (an interval-determinant
certificate), fix the remaining parameter as the curve's coordinate and apply
the **interval Krawczyk operator** to the 3x3 square subsystem. If
K(box) ⊆ int(box), there exists a unique implicit function g over the
u-interval with F(u, g(u)) = 0, enclosed and C-analytic. Output: (pair id,
coordinate interval, enclosure of the segment, Lipschitz bound of g for
chaining). Generalize the torus-theory pullback (which fixes nothing — its
fast path was exact) to this interval form; the torus case must remain a
special case that short-circuits to the landed exact loci.

**4.3 (Chaining).** Consecutive segment enclosures connect when their
coordinate intervals overlap and their enclosures intersect; a closed chain
or a boundary-to-boundary chain certifies a curve component. State the
correspondence between chains and the trim regions' boundary arcs.

**4.4 (Termination).** Complete the argument: the subdivision worklist
empties because every box is either excluded (4.1), absorbed into a segment
(4.2), or bisected; the only obstruction is a degeneracy, which section 5
refuses up front. Make the depth bound explicit in the transversality margin.

## 5. Transversality admission (the refusal semantics)

A pair is admissible iff, certified by interval tests: (a) rank dF = 3 along
the entire curve (checked on the enclosures), (b) the smallest singular value
of J_F along the curve is >= σ_min > 0 with the interval margin that 4.2's
operator requires, (c) triple points (curve self-intersections and
curve-x-curve contacts) are finite and isolated. On failure: **typed refusal**
(`UnsupportedEnvelope(NonCanonicalCarrier)` or a new named case), matching
the loop's doctrine. Prove that (a)-(c) hold on a neighborhood of the corpus's
actual pairs OR show which pairs fail and why (both outcomes are valuable;
the corpus's tub-skin/cavity pair is the primary target).

## 6. Deliverables

1. **Proofs** of Lemma 1, Theorem 2, Corollary 3, and the 4.1/4.2/4.4
   termination argument, in the outward-rounded interval semantics.
2. **Algorithm specification** at pseudocode level against the section-2
   interfaces: inputs (patch control nets, weights, orientations), outputs
   (bracket or typed refusal), complexity notes.
3. **Validation protocol:** synthetic pairs with analytically known
   intersections (plane x loft closed-form curve; ruled x ruled via the
   landed FSSI-004 closed forms) reproduced inside the certificates; a
   deliberately tangential pair refused typed; the corpus's actual
   skin/cavity pair run end to end with the δ budget checked before compute.
4. **Feasibility estimate for the corpus's real pairs:** the tub skin is a
   grid of ~(u-spans of a 64-point closed section) x 41 v-spans bicubic
   patches against the cavity's grid; quantify the worklist scale, the
   exclusion locality (the cavity only approaches the skin across the deck
   band — seed the exclusion with the corpus's own `loft_escaped`-style coarse
   bounds), and the expected subdivision depth from the measured σ_min.
5. **Non-goals, explicit:** no sampling, no tolerance stretching, no
   "approximately intersect then fix up", no trimmed-patch reconstruction.

## 7. What happens after acceptance

The accepted theory is implemented as MONO-5-SWEPT-BOOLEANS through the normal
loop (packet, worker, gates), consuming the landed interfaces; its acceptance
criterion is the corpus's boolean rows (airbox, details, drivetrain) flipping
green against their recorded references, and finally the monocoque row's
cut/fuse cells.
