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
| Hypotheses | purely topological: containment + side-separation (+ homeomorphy for 2.1). NOTE (feedback): the paper's TOPOLOGICAL framework allows compact orientable surfaces WITH boundary — its thickening is S×[0,1] whose boundary contains ∂S×[0,1] — but its METRIC tubular-neighborhood section explicitly switches to "S is C²-smooth and closed", and only there states S^eps ≅ S×[-eps,eps] for eps < lfs(S). The closed-smooth metric tube is theirs; the trimmed-face version is OURS (L-TUBE below). |
| Conclusion | ambient isotopy inside M |
| Kernel certificate today | **BG-FID-003 (i)-(iv) is DESIGNED TO discharge CCS05 T2.1/T2.2; the equivalence is CONDITIONAL on the named bridge lemmas L-COVERING, L-SEPARATES and L-TUBE below** — it is not yet an identity, and no spec prose may claim it is. Mapping: (i) `d_H ≤ eps < rho_lower/2` puts X' inside the normal tube `M := {x : d(x,X) ≤ eps}`; (iii)/(ii) give transversality of phi to fibres; (iv-a/b) certify fibre multiplicity one. |
| Missing certificate | THREE bridge lemmas, each a proof obligation carried in structured comments at the certificate site: **L-COVERING**: the certified fibre projection is a proper local homeomorphism (from transversality/local inverse), hence a finite covering (compact/proper); the certified fibre multiplicity establishes it is ONE-SHEETED; a one-sheeted covering is a homeomorphism. (The precise fact is "one-sheeted covering ⇒ homeomorphism", not bare "degree one implies homeomorphism".) **L-SEPARATES**: once the approximation is a continuous one-sheet SECTION of the product thickening — a graph S' = {(x, f(x)) : x ∈ S} inside S×[0,1] — any path from fibre coordinate 0 to fibre coordinate 1 crosses the graph by continuity. "Each fibre met once" in isolation does NOT complete this proof; the continuous-section property (from L-COVERING's homeomorphism inverse) is what makes it work. **L-TUBE**: for eps < reach(X), the closed eps-tube of a connected compact C² surface-WITH-boundary is a topological thickening whose sides are the offset sheets. Standard tubular-neighborhood theorem covers CLOSED manifolds only ([CCS05]'s own metric section assumes closed); the with-boundary case for trimmed faces is our adaptation (restrict the disc bundle to the boundary-respecting subbundle; needs reach > eps on the face interior stratum). Chain the structured comments encode: transversality/local-inverse + compact/proper + one sheet ⇒ local homeomorphism ⇒ covering ⇒ homeomorphism ⇒ continuous section ⇒ side separation ⟹ CCS05 isotopy. |

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

For a CLOSED C² submanifold, reach is realized either by a bottleneck of size
2tau or by curvature 1/tau (Aamari et al., *Estimating the Reach of a
Manifold*, EJS 2019, state this for compact smooth boundaryless manifolds;
Federer 1959 is the origin). **This decomposition is MOTIVATION for the
implementation's shape; it is NOT yet the proof of it.** The exact quantity
the literature calls the bottleneck term has NOT been matched to what our
`mu_self_lower` would compute — until that match exists, the string
`reach = min(1/rho_max, mu_bottleneck)` appears in NO executable contract,
no packet decision, and no structured comment.

| | |
|---|---|
| Hypotheses | CLOSED C² submanifold (no boundary!) — the spec/lfs.rs warning stands |
| Conclusion | equality for the closed smooth case only |
| Kernel certificate today | `EnclosureSurface::enclose_der(2,·)` supplies interval II boxes; scratch-validated sound this session. mu_self_lower has NO implementation AND no matched literature semantics yet. |
| Missing certificate | **L-FEDERER-PATCH is a red-gate theorem packet of its own**, not a line item here. The statement we actually NEED (feedback-revised): given a parameter cell C lying at certified distance h from the trimmed boundary, curvature bounded above by K over C, and certified EXCLUSION of non-incident sheets inside radius r, prove that the normal tube of radius `min(1/K, r, h)` is single-valued over C. If that admits a direct quantitative inverse-function/tubular-neighborhood proof, we never lean on an informal "local Federer equality" at all. Until this lemma is proved, FID-001 may ship the three-way min ONLY as a certified lower bound on TUBE WIDTH (which is what FID-003(i)'s eps budget actually consumes), and must NOT name it "reach" or "lfs" in any API surface. |

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
  not by refusal. Recorded as a design note; NOT needed unless a downstream
  theorem inequality fails with the current bound.

Every downstream gate has form `q < c · lfs_lower` (BG-FID-007): substituting
a smaller certified value can refuse but cannot admit — the property that
makes tightness irrelevant to correctness.

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
we already plan.

**L-WEDGE-SLOPE (kernel lemma, red-gate; the cos formula is SPECULATIVE until
derived).** The candidate bound `chi_K >= cos(theta/2)` must remain untrusted:
dihedral wedges carry MULTIPLE angle conventions (interior angle, exterior
angle, angle between face normals, angle between distance gradients), and
depending on which one BG-INV-109 supplies, the expression can become a sine
or involve the supplementary angle. The derivation recipe, in order:

1. define the wedge mathematically (two half-planes, chosen convention);
2. write its two nearest-point gradients explicitly;
3. compute the minimum norm of their convex hull (this is what the normalized
   slope at a point equidistant from both sheets reduces to);
4. express the result in EXACTLY the dihedral convention BG-INV-109's
   certificate uses;
5. test limiting behaviour at BOTH degeneracies: theta -> degenerate angle
   (0 or 2pi per INV-109's convention) must give chi_lower -> 0.

Inputs: theta_wedge LOWER bound = BG-INV-109's certificate (verify its landed
state and numeric content at packet time); separation = FID-001's own term.

**Globality caveat (feedback):** a local edge-neighbourhood bound on chi_K
does NOT by itself establish a global r_mu(K) or wfs(K): chi_K takes an
infimum over the relevant distance level/set. Converting local stratum
certificates into the compact-set theorem's global critical function requires
CERTIFIED COVERAGE of all competing regions and a certified minimum over them.
The compact-set statement applies to a B-rep boundary read as one compact set;
that conversion is OURS — name it **L-COVERAGE** and treat it as a proof
obligation wherever a global quantity is claimed. Until it exists, edge/vertex
rows return LOCAL contributions typed as such (`ChiLowerBound { scope: Cell }`),
never a global r_mu/wfs.

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

**Revised FID-001 scope for the packet** (feedback-incorporated):

- **Naming discipline:** until L-FEDERER-PATCH and L-COVERAGE are proved, the
  API ships quantities named for what they certify — `tube_width_lower`,
  `chi_lower` — NOT `reach`/`lfs`. The `lfs_lower` name from the scaffold may
  only attach once its theorem chain is discharged; otherwise the scaffold
  doc comment is amended to record the deferral. (A bound that refuses more
  than true lfs is still safe to consume under BG-FID-007's inequality form;
  what is forbidden is CLAIMING it equals or lower-bounds reach without the
  lemma.)
- Face interior: three-way min shipped as `tube_width_lower =
  min(1/rho_max_upper(cell), mu_self_lower(cell), d_boundary_lower(x))`,
  with rho_max_upper implemented EXACTLY as scratch-validated (enclose_der
  II + iota normalization + refusal). mu_self_lower semantics must be
  MATCHED to a literature quantity or explicitly defined as "certified
  exclusion radius of non-incident sheets" feeding L-FEDERER-PATCH's r —
  the packet picks the latter (it is what our Box3 distances actually give).
- Edge interior: `chi_lower` via L-WEDGE-SLOPE following the five-step
  derivation recipe, consuming BG-INV-109's certificate in ITS convention,
  degeneracy limits asserted by test at both ends.
- Vertex: star separation min (mechanical), typed as local chi contribution.
- Bridge lemmas L-TUBE / L-COVERING / L-SEPARATES are FID-003's proof
  obligations, not FID-001 code — but §6.2's spec amendment records them NOW
  so isotopy_ok's soundness claim stops being prose. Each carries: statement,
  hypotheses in the cited paper's own terms, proof sketch or SPEC_GAP.
- Structured comments: every certificate site cites its theorem instance and
  which hypothesis each input discharges, per the L-COVERING→L-SEPARATES
  chain shape.
- Tests per scaffold (cube unit with `<=` assertions; anti-global-reach;
  scale homogeneity; knife-edge zero) PLUS: sphere-carrier unit asserting
  the curvature term matches the scratch numbers; wedge degeneracy limits
  (theta->0 and theta->2pi both drive chi_lower -> 0); double-cover circle
  refused by consumer-style use (ties to BG-TEST-007).
- Spec amendment BEFORE dispatch: §6.2 names CCS05 T2.1/T2.2 with the
  conditional instantiation map and the three bridge lemmas; §6.1 replaces
  the theta_wedge row with chi_K-lower-bound framing + globality caveat
  (L-COVERAGE); Federer decomposition demoted to motivation until matched.

**Stop conditions added to the packet:** any bridge lemma that cannot be
justified from the cited theorem's actual hypotheses => SPEC_GAP naming the
gap; do not invent the bridge. Resuming curvature-tightness work requires a
downstream theorem inequality that FAILS with the current conservative bound.

## Open items carried to packet time

- TRANSCRIBE-AT-PACKET-TIME: exact constants of [CCSL09-CGTA] Thm 4.2/4.3
  (fetch the PDF again and copy character-for-character); Federer's exact
  equality statement — now DEMOTED to motivation; no gate cites it.
- Verify BG-INV-109's landed state and, critically, WHICH angle convention
  its wedge certificate returns (interior/exterior/normal-normal) before
  writing L-WEDGE-SLOPE's step 4.
- Decide locality granularity jointly with FID-003's consumption pattern in
  rep's refine loop (the asin(eps/rho_lower) term wants rho_lower valid ON
  THE CELL BEING EMITTED).
- L-FEDERER-PATCH and L-COVERAGE may each deserve their own red-gate theorem
  packets rather than riding inside FID-001 — decide after the FID-001
  packet's write set is fixed.
