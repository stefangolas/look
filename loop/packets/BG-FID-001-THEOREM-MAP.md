# BG-FID-001/FID-003 THEOREM MAP — instantiation, not invention

Written before the BG-FID-001 packet. Purpose: every FID certificate must cite
the published theorem it instantiates, with hypotheses preserved exactly; where
our B-rep statement is not covered, we add the smallest named kernel lemma
instead of silently generalizing the paper or coining a novel quantity.
Sources were fetched and read for this map (URLs inline); anything not read
verbatim is flagged TRANSCRIBE-AT-PACKET-TIME.

## Sources verified

- **[CCS05]** F. Chazal, D. Cohen-Steiner, *A condition for isotopic
  approximation*, Graphical Models 67(5):390-404, 2005.
  https://geometrica.saclay.inria.fr/team/Fred.Chazal/papers/PublishedIsotopyGMOD.pdf
  (Theorems 2.1/2.2 quoted verbatim below.)
- **[CCSL09-DCG]** F. Chazal, D. Cohen-Steiner, A. Lieutier, *A sampling theory
  for compact sets in Euclidean space*, Discrete Comput. Geom. 41:461-479,
  2009. (mu-reach Def 4.3, critical function, wfs Lemma 2.1, critical-value
  stability Thm 2.2 - statements confirmed via publisher page + INRIA copies.)
- **[CCSL09-CGTA]** F. Chazal, D. Cohen-Steiner, A. Lieutier, *Normal cone
  approximation and offset shape isotopy*, Comp. Geom. Theory Appl.
  42(6-7):566-581, 2009.
  https://geometrica.saclay.inria.fr/team/Fred.Chazal/papers/ccsl-ncaoi-09/ccsl-ncaoi-09.pdf
  (Isotopy lemma = their Thm 2.2 citing Edelsbrunner; Level sets isotopy Thm
  4.2; Isotopic reconstruction Thm 4.3.) **The explicit interval/constants in
  Thm 4.2's hypothesis are garbled in every text extraction I have; they are
  flagged TRANSCRIBE-AT-PACKET-TIME below and no kernel gate may depend on
  them until read off the published PDF.**
- **[Federer59]** H. Federer, *Curvature measures*, Trans. AMS 93:418-491,
  1959. Not yet re-read this session: the reach decomposition used below is
  quoted via [CCSL09-CGTA]'s and the spec's own usage. TRANSCRIBE-AT-PACKET-
  TIME if any packet leans on the exact equality rather than the inequality.

---

## Cluster 1 - [CCS05] Theorems 2.1 / 2.2 (thickening -> isotopy)

**Theorem 2.1, verbatim.** Suppose that:
1. S' is homeomorphic to S.
2. S' is included in a topological thickening M of S.
3. S' separates the sides of M.
Then S' is isotopic to S in M.
("Separates" = every continuous path in M from one side to the other meets
S'. A topological thickening is M ⊆ R^3 with a homeomorphism U : S×[0,1] → M;
sides = U(S×{0}), U(S×{1}). No smoothness of S is required.)

**Theorem 2.2, verbatim structure.** If additionally S sits in a thickening M'
of S' and separates ITS sides, the homeomorphy hypothesis drops; S and S' are
isotopic in both M and M'.

| | |
|---|---|
| Hypotheses | purely topological: containment + side-separation (+ homeomorphy for 2.1) |
| Conclusion | ambient isotopy inside M |
| Kernel certificate today | **BG-FID-003 (i)-(iv)** is exactly this, mechanized: (i) `d_H ≤ eps < rho_lower/2` puts X' inside the normal tube `M := {x : d(x,X) ≤ eps}` — which for `eps < reach` IS a topological thickening (disc bundle over X), sides = the two offset sheets. (iii)/(ii) make phi transverse to fibres; (iv-a/b) certify each fibre met once ⇒ along ANY fibre path from side to side, X' appears ⇒ **side separation holds by intermediate value on each fibre**. Homeomorphy for T2.1 comes free from (i)-(iii)'s covering property plus (iv-a) degree one: a degree-one proper covering is a homeomorphism. |
| Missing certificate | (a) the spec never NAMES the theorem — soundness of isotopy_ok rests on prose; add citation + the one-line fibre-wise IVT argument to §6.2 so reviewers check the mapping, not a vibe. (b) the small lemma "**reach-tube thickening lemma**": for eps < reach(X), the closed eps-tube of a connected compact C² surface-with-boundary is a topological thickening whose sides are the offset sheets. Standard tubular-neighborhood theorem for CLOSED manifolds; the WITH-BOUNDARY case for trimmed faces is OUR adaptation and must be stated as kernel lemma L-TUBE (proof: restrict the disc bundle to the boundary-respecting subbundle; needs only reach > eps on the face interior stratum). |

## Cluster 2 - metric corollaries of CCS05 (lfs enters)

The paper's own Section 3 instantiates the abstract thickening M by METRIC
objects (offset slabs / interval solids; their Cor. 3.x quantitative interval-
solid version). The role local feature size plays: **reach bounds how small
eps may be while M stays a thickening** (Cluster 1's missing lemma L-TUBE).

| | |
|---|---|
| Hypotheses | S smooth-ish with reach τ > 0; approximant within eps < τ and separating |
| Conclusion | isotopy; quantitatively, eps budgeted against τ |
| Kernel certificate today | BG-FID-003(i)'s `eps < rho_lower/2` consumes `rho_lower` = lfs_lower/reach lower bound = **BG-FID-001's output**. The factor 1/2 (and angle condition's asin(eps/rho_lower)) is the paper-side constant chain. |
| Missing certificate | none mathematical. FID-001 owes ONE number per stratum with certified direction (lower), which is already the scaffold contract. What is genuinely missing downstream: FID-003(ii)'s `asin(eps/rho_lower)` presumes curvature-radius semantics of rho_lower at the SAME point x — i.e., rho_lower must be LOCAL (per-cell), not a global min, or the asin bound is taken at the wrong scale. Decide at packet time: lfs_lower(x, cell) API shape. |

## Cluster 3 - Federer reach decomposition (curvature vs bottleneck)

For a CLOSED C² submanifold, reach = min(1/rho_max, mu_bottleneck) where
rho_max = sup |k_n| (max absolute normal curvature) and mu_bottleneck = closest
self-approach of distinct sheets (medial-axis distance). This is the split the
spec's face-interior row already encodes: `min(1/rho_max_upper, mu_self_lower)`.

| | |
|---|---|
| Hypotheses | CLOSED C² submanifold (no boundary!) — the spec/lfs.rs warning stands |
| Conclusion | equality; used by us conservatively as inequality |
| Kernel certificate today | `EnclosureSurface::enclose_der(2,·)` supplies interval II boxes; scratch-validated this session (see Scratch findings). mu_self_lower has NO implementation yet. |
| Missing certificate | **L-FEDERER-PATCH (kernel lemma, smallest form):** for x on a face interior with d(x, ∂face) ≥ h, the reach of the face AT x is ≥ min(1/rho_max_upper(cell_x,h), mu_self_lower(...)) where cell terms are computed over the h-interior subdomain. Proof obligation: Federer's equality restricted to a compact subdomain with boundary stays an inequality (min of the two mechanisms still lower-bounds local reach); this is the "local Federer" adaptation and must carry its own proof sketch in the packet, NOT a citation. |

### Sufficiency verdict on the curvature upper bound (user question)

**The current conservative bound is SUFFICIENT; do not refine it.** Judged
against the downstream inequality, the ONLY requirement is soundness (never
underestimate rho_max) plus honest refusal. Session-scratch evidence on the
sphere carrier (r=2, true sup|k_n| = 1/2):

- Sound everywhere it answers: k_up >= truth on all cells (ratios 20.5x down
  to ~2.0x under refinement).
- Refuses cleanly where it cannot certify (pole-straddling cells, wide cells).
- Normalization MUST use the carrier's `immersion_lower_bound` (iota route):
  consistently tighter than the naive cross-box norm bracket, same refusals.
- Wide cells refuse because first-form coefficient BOXES collapse when S_u's
  box straddles zero — refusal-driven subdivision alone certifies but with
  terrible constants (77x-409x) because moderately-wide cells answer badly;
  IF tightness ever matters, subdivision must be driven by a target bound,
  not by refusal. Recorded as a design note; NOT needed for FID-003, whose
  gates consume lfs_lower through inequalities where over-estimation of
  rho_max merely costs eps budget (over-refusal = epistemic refusal, already
  the spec's stated semantics).

Every downstream gate has form `q < c · lfs_lower` (BG-FID-007): substituting
a smaller certified value can refuse but cannot admit — the property that
makes tightness irrelevant to correctness.

### reach_lower = min(curvature_radius_lower, bottleneck_lower) route

Adopt for smooth face interiors, structured as:

```
reach_lower(face interior point x)
  = min( 1 / rho_max_upper(cell around x),
         mu_self_lower(distance to non-incident sheets),
         d_boundary_lower(x) )        # strata table's separation term
```

This is NOT Federer's equality claimed as equality — it is the conservative
min-of-lower-bounds, valid because min(a,b,c) <= min(A,B,C) whenever each
certified piece <= its true counterpart, and local reach >= min of the three
mechanisms by L-FEDERER-PATCH. The three-way min (adding the boundary term)
is what makes the patch lemma work without closedness.

## Cluster 4 - [CCSL09] critical function, wfs, mu-reach

Definitions (as used across both 2009 papers):

- Critical points of the distance function d_K via generalized gradient /
  normalized slope kappa(x) = |x - zeta(x)|/d(x) with zeta(x) the centre of
  the smallest enclosing ball of the nearest-point set Gamma_K(x) — exactly
  the spec line ~2123 formulation.
- **wfs(K)** = infimum of positive critical values of d_K. Offsets strictly
  below wfs are pairwise isotopic ([CCSL09-DCG] Lemma 2.1, crediting
  Chazal-Lieutier 2005/2007).
- **critical function chi_K(t)** = inf of the normalized slope over points at
  distance >= t from K; **mu-reach r_mu(K) = inf{t : chi_K(t) < mu}**
  ([CCSL09-DCG] Def 4.3). Interpolates wfs (mu->0) and lfs (mu=1).
- **Level-sets isotopy theorem** [CCSL09-CGTA] Thm 4.2: Hausdorff-close K, K'
  (d_H < eps); if chi_K exceeds a gamma-dependent threshold on a doubled
  interval around a, then the level sets at a are isotopic with Frechet bound
  eps/gamma. Constants: TRANSCRIBE-AT-PACKET-TIME.
- **Isotopic reconstruction theorem** Thm 4.3: K with r_mu(K) > 0, K' a
  (kappa,mu)-approximation with kappa below an explicit constant times
  mu^2/(16+2mu^2)-type bound, offsets at d < wfs(K) isotopic.
  Constants: TRANSCRIBE-AT-PACKET-TIME.
- **Critical-value separation**: d_K' has no critical value in an explicit
  interval derived from (eps, mu) — the engine behind both theorems.

| | |
|---|---|
| Hypotheses | compact K, positive mu-reach (non-smooth OK — this is why this cluster matters for B-reps); approximation quality vs kappa·r_mu |
| Conclusion | critical-value-free intervals; isotopy of offsets; reconstruction |
| Kernel certificate today | NOTHING computes chi_K, wfs or r_mu. BG-FID-001's edge/vertex rows currently gesture at a `theta_wedge` quantity with no theorem behind it. BG-INV-109 supplies wedge non-degeneracy (dihedral bounded off 0 and 2pi). Stage 4 interface notes reference Grove-Shiohama/Clarke critical values. |
| Missing certificate | see the decision below — do NOT build a novel L_wedge |

### Decision: sharp edges/vertices — instantiate chi_K, don't invent L_wedge

The current lfs.rs table row "edge interior: theta_wedge(e), -> 0 as theta ->
0 or 2pi" coins a quantity no paper defines. Replace the CONTRACT (not just
the implementation): the edge/vertex rows must return **certified lower bounds
on the critical function / mu-reach contribution**, built from two certificates
we already plan:

- **L-WEDGE-SLOPE (kernel lemma, smallest form):** for a solid bounded by two
  half-planes meeting at dihedral angle theta, the normalized slope of d_K
  satisfies |grad-normalized| >= cos(min(theta, 2pi-theta)/2) on the bisector
  region within distance s of the edge, with s = separation to non-incident
  structure. Elementary geometry of wedges; proof is a direct computation in
  the packet. Consequence: chi_K(t) >= cos(theta_worst/2) for t up to
  s_edge-scale, hence r_mu(edge neighbourhood) >= s · (something explicit in
  theta, mu) and wfs-contribution >= min over star of these.
- Inputs: theta_wedge LOWER bound = BG-INV-109's certificate (exists as an
  item; checker landed? INV-109 status: DONE per earlier registry waves —
  verify at packet time); separation = BG-FID-001's own separation term.

This preserves published hypotheses exactly: we compute a lower bound on the
PAPERS' quantity (chi_K / mu-reach), so downstream consumers (Stage 4 topology
events, §16.3 generalized critical values) sit directly on CCSL09 statements.
If our B-rep statement (stratified, trimmed, corners where three faces meet)
is not literally covered by [CCSL09]'s compact-set setting — it is, B-rep
boundaries ARE compact sets, but positivity of wfs for polyhedra-like sets is
cited to [Chazal-Lieutier 05/07] — then the smallest adaptation is a cited
positivity lemma ("piecewise analytic => wfs > 0", [CCSL09-CGTA] intro citing
refs 6,7), not a new theory.

---

## Minimum FID-001 outputs required to discharge FID-003

FID-003 consumes exactly TWO things from FID-001 (audit of §6.2's formulas):

1. **rho_lower** appearing in (i) `eps < rho_lower/2` and (ii)
   `theta < pi/2 - asin(eps/rho_lower)`: needs a LOCAL, per-cell curvature-
   mechanism lower bound on face interiors = `1/rho_max_upper(cell)` via the
   validated II term, PLUS the bottleneck/separation pieces so the MIN is a
   true reach lower bound (L-FEDERER-PATCH). The curvature term is DONE
   modulo landing the design as specified above (iota normalization; refusal
   on wide cells; bisection only if a caller demands tightness).
2. **separation_lower** feeding (iii) boundary correspondence and the
   strata-aware application of (i)-(ii) away from edges: distance lower
   bounds to non-incident strata — enclosure-box based, no new math.

NOT required by FID-003 (defer unless another item needs them):
edge/vertex intrinsic rows beyond the wedge lemma's chi_K bound (that is
FID-002/§6.1 positivity routing and Stage 4's business), global wfs
computation, mu-reach evaluation machinery, tight curvature refinement.

**Revised FID-001 scope for the packet:**

- `lfs_lower` API: stratified struct returning the three-way min per stratum,
  typed `LfsLowerBound`, direction documented (BG-FID-007).
- Face interior: `min(1/rho_max_upper, mu_self_lower, d_boundary_lower)`
  with rho_max_upper implemented EXACTLY as scratch-validated (enclose_der
  II + iota normalization + refusal), mu_self_lower and d_boundary_lower
  via Box3/enclosure distances between incident/non-incident cells.
- Edge interior: `chi_contribution_lower = cos(theta/2)` form via L-WEDGE-
  SLOPE consuming BG-INV-109's wedge certificate + separation; returned AS a
  critical-function lower bound type (`ChiLowerBound`), not as "lfs".
- Vertex: star separation min (mechanical), flagged as feeding the same
  chi_K machinery.
- Tests per scaffold (cube unit with `<=` assertions; anti-global-reach;
  scale homogeneity; knife-edge zero) PLUS: sphere-carrier unit asserting
  the curvature term matches the scratch numbers (soundness witnesses), and
  the double-cover circle refused by any consumer-style use (ties to
  BG-TEST-007).
- Spec amendment BEFORE dispatch: name CCS05 T2.1/T2.2 in §6.2 with the
  instantiation map above; replace the `theta_wedge` prose in §6.1 with the
  chi_K-lower-bound framing; record L-TUBE / L-FEDERER-PATCH / L-WEDGE-SLOPE
  as named kernel lemmas with their proof obligations.

## Open items carried to packet time

- TRANSCRIBE-AT-PACKET-TIME: exact constants of [CCSL09-CGTA] Thm 4.2/4.3
  (fetch the PDF again and copy character-for-character); Federer's exact
  equality statement if any gate cites it as more than motivation.
- Verify BG-INV-109's landed state and what its wedge certificate returns
  numerically (angle only? angle + witness?) before writing L-WEDGE-SLOPE.
- Decide lfs_lower's locality granularity (per-point with cached per-cell
  refinement) jointly with FID-003's consumption pattern in rep's refine
  loop — the asin(eps/rho_lower) term wants rho_lower valid ON THE CELL BEING
  EMITTED, which suggests lfs_lower should be callable per partition cell.
