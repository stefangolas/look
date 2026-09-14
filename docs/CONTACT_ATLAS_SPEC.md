# Certified Boolean Volume by Contact Atlas, Exact Region Flux, and Best-First Residual Refinement

Standalone theorem specification, soundness proof, and structural runtime bounds

Status: integrated ship-candidate formal design specification.

This document supersedes the earlier refinement-only specification and the contact-tracing addendum. It states one integrated certified algorithm. The principal change is that regular transversal contact is solved geometrically by a certified one-dimensional contact atlas and high-order trace tubes; cell covering remains a sound fallback rather than the primary accuracy mechanism.

Every asymptotic statement is parameterized by certified structural quantities. No empirical timing constant is used in a proof.

## 0. Contract

The engine computes a certified interval for the volume of a regularized Boolean operation on two oriented solids.

For every invocation it guarantees:

Soundness. Every returned interval contains the true regularized Boolean volume.

Global error control. Acceptance is determined from the width of the total volume bracket, not from a fixed local threshold.

Exact geometric fast paths. Positive-area co-support is handled algebraically when certified. Regular transversal contact is converted into a certified contact atlas and trace tube rather than covered blindly by area cells.

Refusal semantics. Unsupported or unresolved configurations return a typed refusal and the current sound bracket. No interval is narrowed by guessing.

Totality. Finite topology, tracing, subdivision, depth, and arithmetic budgets make every implementation invocation terminate.

Determinism. With fixed arithmetic, tie-breaking, chart-selection, splitting, and reduction rules, the result is bit-reproducible on the same execution platform.

Structural runtime bounds. Runtime is expressed in terms of patch count, hierarchical membership traversal, candidate-pair traversal, co-support complexity, contact topology, trace regularity, requested bracket width or decision margin, near-coincidence conditioning, patch degree, and fallback unresolved-set dimension.

Hierarchy-first retirement. Certified whole-node membership may retire a BVH subtree using a precomputed aggregate flux interval before any descendant patch or contact work is performed.

Decision-directed execution. When the consumer asks a certified predicate rather than a tight numeric volume, the same anytime bracket may terminate as soon as that predicate is decided.

The specification deliberately distinguishes:

geometric information acquisition
    -> shrinks the admissible selector family

certified integration
    -> converts resolved and unresolved geometry into interval flux

residual scheduling
    -> spends a global error budget on trace spans and fallback cells

A stronger geometric theorem is allowed to improve the runtime exponent. The scheduler is never asked to compensate for missing geometric information by unbounded subdivision.

## 1. Geometric model

Let A and B be compact regular closed solids in R^3. Their boundaries are represented by finitely many oriented regular patches

P_p : U_p -> R^3,

where each parameter domain U_p is a compact rectangle, normally [0,1]^2 after normalization.

Assume on each patch interior:

rank D P_p = 2.

Patch orientation agrees with the outward orientation of its operand. Certified seams, singular parameter locations, and patch-boundary events may be present; they are handled as lower-dimensional events and have zero surface measure.

For rational patches, the weight function must be certified nonzero on the admitted parameter domain. Standard positive-weight NURBS satisfy this condition.

The core theorems are stated for one binary Boolean dispatch. Multi-solid expressions are evaluated by certified orchestration of these binary kernels. Section 6B gives an exact sparse decomposition for union folds; no generic additive decomposition is assumed for arbitrary Boolean expression DAGs.

Let

R in { A union B, A intersection B, A \ B }

be the regularized Boolean result.

## 2. Boolean volume as selected boundary flux

For a source patch p, define

g_p(u,v)
    = (1/3) P_p(u,v) . (P_{p,u}(u,v) x P_{p,v}(u,v)).

For each Boolean operation associate to each source patch a selector

s_{R,p}(u,v) in {-1, 0, +1}.

0 means the source boundary is absent from the regularized result; +1 means it appears with source orientation; -1 means it appears with reversed orientation. The reversal of a retained B boundary under difference is absorbed into the selector.

After certified positive-area co-support handling,

V(R)
  = sum_p integral_{U_p} s_{R,p}(u,v) g_p(u,v) du dv.        (1)

Proof obligation P1 — selector semantics

For the concrete BRep orientation and regularization convention, prove equation (1) for union, intersection, and difference, including all source-orientation signs.

## 3. Certified knowledge state

The algorithm never reasons from a single guessed membership state. It carries the family of selector states still consistent with certified evidence.

For a parameter region C subset U_p, define

S(p,C)

as the admissible family of selector restrictions on C.

For s in S(p,C), define

J_{p,C}(s) = integral_C s(u,v) g_p(u,v) du dv.

A certified bound object returns an outward-rounded interval

Phi(p,C) = [lo,hi],
width(Phi) = hi - lo.

Axioms E1-E5

E1 — epistemic enclosure.

J_{p,C}(s) in Phi(p,C)     for every s in S(p,C).

E2 — truth inclusion. The true Boolean selector restricted to C belongs to S(p,C).

E3 — monotone information. Subdivision or a stronger geometric certificate may remove admissible selector states but may not remove the true one.

E4 — exact additivity. If C is partitioned into disjoint regions C_i up to measure-zero boundaries, then the true contribution on C is the sum of the true contributions on the C_i.

E5 — outward composition. Interval addition, integration bounds, and all arithmetic enclosures use directed rounding and contain their exact-real counterparts.

Theorem 1 — local soundness

For every certified region (p,C), Phi(p,C) contains its true Boolean flux contribution.

Proof. By E2 the true selector belongs to S(p,C). E1 encloses the integral of every member of that family. QED

Theorem 2 — partition soundness

Replacing a parent region by an outward-rounded sum of certified child-region intervals preserves the parent's true contribution.

Proof. Apply Theorem 1 to each child, E4 to the exact contributions, and E5 to the interval sum. QED

Definition — ambiguity diameter

Delta(p,C)
  = sup_{s,t in S(p,C)} |J_{p,C}(s) - J_{p,C}(t)|.

Theorem 3 — information lower bound

Every interval satisfying E1 obeys

width(Phi(p,C)) >= Delta(p,C).

Proof. An interval containing all values J_{p,C}(s) must have width at least the diameter of that set. QED

This is the correct impossibility quantity. Positive area alone does not imply a nonzero residual floor; the ambiguity diameter does.

## 4. Global bracket and acceptance

At any time the algorithm holds a finite collection of certified contribution objects. They may be:

exact or narrow resolved-region flux intervals;

trace-tube residual intervals;

unresolved fallback cells;

frozen cells whose resource budget is exhausted.

Let their outward-rounded sum be

B = [V_lo,V_hi],
W = V_hi - V_lo.

Theorem 4 — global bracket invariant

At every execution state,

V(R) in B.

Proof. Initially the contribution objects partition all source-patch domains after co-support decomposition. Theorem 1 encloses every part. Every replacement step is a certified partition or a stronger enclosure of the same semantic contribution, so Theorem 2 preserves the sum. QED

Absolute gate

For requested absolute width tau > 0, accept when

W <= tau.                                                (2)

Mixed gate

For eps_abs >= 0, eps_rel >= 0, accept when

W <= eps_abs + eps_rel * max(V_lo,0).                   (3)

Theorem 5 — mixed-gate guarantee

If (3) holds, then

W <= eps_abs + eps_rel * V(R).

Proof. Physical volume is nonnegative and Theorem 4 gives V(R) >= max(V_lo,0). QED

The mixed gate remains meaningful for zero-volume results; a purely relative gate does not.

Decision-directed gates

The global bracket is an anytime certificate. A caller need not request a numerically tight volume when it only needs a predicate.

For the threshold predicate

V(R) > X,

return TRUE as soon as

V_lo > X,

and return FALSE as soon as

V_hi <= X.

For two independently certified quantities

V_1 in [L_1,U_1],    V_2 in [L_2,U_2],

the comparison V_1 > V_2 is certified TRUE when L_1 > U_2 and FALSE when U_1 <= L_2.

Theorem 5A — decision-gate soundness and stopping margin

Every threshold/comparison answer returned by the rules above is correct. Moreover, if the exact threshold margin

delta_dec = |V(R)-X| > 0,

then any bracket of width strictly below delta_dec necessarily decides V(R) > X. For a comparison, if

delta_cmp = |V_1-V_2| > 0,

then combined bracket width

(U_1-L_1) + (U_2-L_2) < delta_cmp

necessarily decides the ordering.

Proof. Soundness follows directly from containment of the exact value in each interval. If V>X and W< V-X, then V_lo >= V-W > X; the V<X case is symmetric. Apply the same argument to the difference interval [L_1-U_2,U_1-L_2] for comparisons. QED

Thus a decision workload inherits the runtime theorems below with the required numerical width replaced by the actual separation margin needed to decide the predicate. For fixed regular traced geometry this gives

O(delta_dec^(-1/(k+1))),

whereas a subdivision-only regular-curve fallback gives O(1/delta_dec). If the exact value lies on the decision boundary, no positive separation margin exists and no finite-width theorem can force an early decision.

## 5. Positive-area co-support

Subdivision and tracing are not the primary solvers for a shared two-dimensional support.

Definition — co-supported region

Source patches p subset dA and q subset dB are co-supported on a regular region S if their world-space images coincide on S with positive area.

At a regular point of S:

parallel means their outward normals agree;

antiparallel means their outward normals oppose.

Theorem 6 — local regularized co-support rule

operation

relative orientation

contribution on shared region S

A union B

antiparallel

0

A union B

parallel

one copy with common outward orientation

A intersection B

antiparallel

0

A intersection B

parallel

one copy with common outward orientation

A \ B

antiparallel

the A copy

A \ B

parallel

0

Proof. Choose a signed normal coordinate t with A locally occupying t<0. Parallel normals imply B occupies the same half-neighborhood; antiparallel normals imply it occupies t>0. Evaluating the regularized union, intersection, and difference of these local half-neighborhoods yields the table. QED

Required co-support oracle

For every candidate pair that may share positive-area support, the geometry layer must return exactly one of

NoPositiveAreaOverlap
Resolved(shared-region decomposition, orientations)
UnknownPositiveAreaOverlap

Resolved may split a source patch into finitely many subregions whose boundaries have zero surface measure.

UnknownPositiveAreaOverlap is an admission refusal:

CoincidentSupportUncertified.

Identical control nets under a certified parameter symmetry are a useful fast path, not a complete detector. Reparameterized coincidence and partial support overlap must also be resolved or refused.

Theorem 7 — no persistent positive-area support ambiguity on admitted inputs

If the co-support oracle is complete for admission, no unresolved positive-area shared-support region enters the asymptotic tracing or subdivision stages.

QED

Near-but-distinct supports may still look two-dimensional at coarse scale; their finite conditioning cost is treated in Section 14.

## 6. Certified pair search

Let each patch or patch node have a certified bounding volume. For a pairwise objective sigma(p,q), every node pair (U,W) used for pruning must provide a lower bound

LB(U,W) <= sigma(p,q)

for every leaf pair p in U, q in W.

A branch-and-bound traversal stores an incumbent exact leaf value sigma_hat and prunes (U,W) when

LB(U,W) >= sigma_hat.

Theorem 8 — branch-and-bound exactness

The traversal returns the exact minimum over all non-excluded leaf pairs.

Proof. Every pruned subtree contains only leaves whose objective value is at least its certified lower bound and hence at least the incumbent. Every unpruned leaf pair is eventually examined. QED

For balanced BVHs, construction costs

O(N log N),    N = N_A + N_B.

Let P_vis be the number of node pairs visited. Traversal costs

O(P_vis * T_pair),                                      (4)

with unconditional worst case

P_vis = O(N_A N_B).

An O(N log N + n) output-sensitive claim requires an additional packing/overlap theorem and is not assumed here.

Metric separation and angular transversality are distinct objectives. A metric box-gap lower bound cannot be reused as an angular lower bound unless a theorem explicitly relates them.

6A. Hierarchical flux aggregation and whole-subtree retirement

The source-patch BVH may carry one additional certified annotation per node. For a node U, let Leaves(U) be its source patches and define

F(U)
  = outward_sum_{p in Leaves(U)}
      integral_{U_p} g_p(u,v) du dv.                   (4a)

Leaf whole-patch flux intervals are evaluated once for the current placement; internal values are then one bottom-up interval sum. If those leaf certificates are already produced by the operand-volume phase, the annotation adds only O(N) work and O(N) interval storage. Otherwise let C_leaf_flux denote the one-time cost of producing them; the bottom-up aggregation itself is still O(N).

Let a certified node-membership query against the opposite solid return one of

EntireInside,
EntireOutside,
Unresolved.

EntireInside/EntireOutside mean that every point of every descendant source patch has that membership state and that no descendant point lies on the opposite boundary. A bounding-box test may return Unresolved conservatively even when the patches themselves are classifiable.

For a fixed source operand and Boolean operation, a constant opposite-solid membership state determines one selector multiplier

sigma in {-1,0,+1}

for every descendant patch. For A \ B, for example, an A node is retained with +1 when entirely outside B and discarded when entirely inside; a B node is retained with -1 when entirely inside A and discarded when entirely outside.

Theorem 8A — certified subtree retirement

If node U has a certified EntireInside or EntireOutside result, and the Boolean selector multiplier for that source operand/state is sigma, then the entire subtree contribution is enclosed by

sigma * F(U),                                           (4b)

and no descendant patch classification, contact tracing, or fallback cell work is required.

Proof. The membership certificate makes the Boolean selector constant and equal to sigma on every descendant patch. Flux is additive over the descendant patch domains, and F(U) is their outward-rounded sum. Multiplication by the exact selector and outward rounding therefore encloses the exact subtree contribution. QED

Let H_vis be the number of source-BVH nodes visited by hierarchical membership classification and T_hmem the cost of one such certified node query. The hierarchy-first clear-region cost is

C_leaf_flux + O(N) + O(H_vis * T_hmem),                 (4c)

where C_leaf_flux=0 denotes the case in which the required leaf flux certificates were already available from existing operand-volume work. Worst-case H_vis=O(N) per source tree. In favorable separated geometry a high node may retire hundreds of descendant patches, so H_vis can be far smaller than the patch count. A stronger claim such as work strictly proportional to the geometric contact set requires an additional spatial-separation/packing theorem and is not assumed here.

The node annotation is a fast path only. Unresolved descends normally and eventually reaches the co-support/contact machinery, so it cannot hide contact.

6B. Sparse multi-solid union by certified interaction graph

For a union fold over solids A_1,...,A_m, form an undirected interaction graph

G_int = ( {1,...,m}, E ),

with edge (i,j) whenever broad phase cannot certify

A_i intersect A_j = empty.

Thus absence of an edge is a proof of pairwise disjointness, not merely failure to observe an overlap. Let the connected components be C_1,...,C_r and define

U_a = union_{i in C_a} A_i.

Theorem 8B — exact union-component decomposition

Distinct component unions are pairwise disjoint and

V(union_{i=1}^m A_i)
  = sum_{a=1}^r V(U_a).                                 (4d)

Each component may therefore be evaluated by an independent Boolean sub-fold, and singleton components require no Boolean interaction work.

Proof. If a != b, no graph edge joins any i in C_a to any j in C_b; by the edge contract every such operand pair is certified disjoint. Hence U_a intersect U_b = empty. Volume is finitely additive on disjoint measurable regular closed sets, giving (4d). QED

The connected components are computed by union-find in

O(m + |E| alpha(m))

after the may-overlap relation is available. If a naive orchestration would perform quadratic cross-operand interaction work and every connected component has size at most s, the corresponding pairwise orchestration term becomes O(m s) rather than O(m^2).

This theorem is intentionally operation-specific. For an intersection of all operands, more than one certified-disjoint component proves the total intersection empty. For A \ (union_i B_i), subtractor components certified disjoint from A may be discarded. An arbitrary Boolean expression DAG does not in general split additively by connected components and must use only separately proved rewrite rules.

6C. Memoization of rigid-invariant operand volume

Let an immutable construction identity name a closed operand geometry before placement, and let

T(x) = Qx + a,    Q^T Q = I,

be a rigid Euclidean placement.

Theorem 8C — rigid-placement volume reuse

V(T(A)) = V(A).                                         (4e)

Therefore any certified interval enclosing the closed operand volume may be cached by immutable construction/version identity and reused under arbitrary rigid placements. If a later request requires a narrower interval than the cached certificate provides, that certificate may be refined once and the tighter result reused subsequently.

Proof. Euclidean isometries preserve Lebesgue measure; equivalently |det Q|=1. QED

This optimization applies to closed operand volume certificates used by dispatch, normalization, or admission. It does not by itself justify reusing arbitrary open-patch flux intervals under translation, because the flux of an individual open boundary piece is origin/placement dependent.

If a fold performs k dispatches over only m_unique immutable operand constructions, a repeated own-volume phase drops from O(k C_vol) recomputation to

O(m_unique C_vol + k T_cache),                          (4f)

subject to the requested certificate widths.

Caching larger topology/contact certificates by construction identity is also sound in principle when all dependencies and invalidations are explicit, but that incremental edit-graph machinery is outside the normative ship specification.

## 7. Transversal contact system

For a candidate pair of regular patches p,q, write

G(u,v,s,t)
  = P_p(u,v) - P_q(s,t) : U_p x U_q -> R^3.             (5)

For polynomial patches, G is polynomial in Bernstein form. For rational patches with certified nonzero weights, one may either evaluate (5) with rational interval arithmetic or clear denominators using the certified nonzero weight factors; clearing denominators then introduces no spurious zero.

The pairwise contact set is

Z_{pq} = G^{-1}(0).

Define the quantitative transversality margin on Z_{pq} by

mu_tr
  = inf_{x in Z_{pq}} sigma_3(DG(x)),                   (6)

where sigma_3 is the smallest nonzero singular value of the 3 x 4 Jacobian. Equivalently, for regular surfaces, a certified lower bound on the sine of the angle between their tangent planes can be converted into such a rank margin after accounting for patch parameter conditioning.

Theorem 9 — regular contact is a one-manifold

If both patches are regular and

mu_tr > 0,

then every interior point of Z_{pq} is regular, rank DG = 3, and Z_{pq} is locally a C^r one-dimensional manifold whenever the patches are C^r. At the boundary of U_p x U_q, the same statement holds as a one-manifold with boundary after the usual domain-boundary restriction.

Proof. DG = [P_u P_v -Q_s -Q_t]. Its column space is the sum of the two tangent planes. Distinct tangent planes span R^3; the quantitative margin (6) makes the rank uniformly three. The regular-value/implicit-function theorem gives a local one-dimensional manifold. QED

This theorem provides local traceability, not yet global topology.

## 8. Parametric certified graph charts

A one-dimensional zero set must not be certified with an isolated-root theorem. The chart primitive is parametric.

Choose one coordinate t from (u,v,s,t) as local marching parameter and write the remaining three coordinates as y in R^3. The contact equations become

F(t,y) = 0.                                              (7)

Let T be an interval for the marching coordinate and Y a box for the remaining coordinates. Let R be a floating approximation to the inverse of D_y F at a chosen center; its accuracy affects contraction efficiency, never soundness.

Define a uniform parametric Krawczyk enclosure

K(T,Y)
  = y0 - R F(T,y0)
      + (I - R D_yF(T,Y)) (Y-y0).                       (8)

All terms are interval-valued and outward rounded.

Chart certificate C1

A box T x Y is a certified graph chart when

K(T,Y) subset int(Y).                                   (9)

Theorem 10 — uniform graph existence and uniqueness

If (9) holds, then for every t in T there exists exactly one y(t) in Y with

F(t,y(t)) = 0,

and every zero of F in T x Y belongs to this graph. The map y(t) inherits the differentiability permitted by F and the nonsingularity of D_yF.

Proof. For each fixed t, (8)-(9) imply the ordinary Krawczyk inclusion for the square system in y; because the interval evaluation was taken uniformly over T, the inclusion holds simultaneously for all t in T. Existence and uniqueness therefore hold pointwise for every parameter value. If a zero existed in the box off the graph, it would contradict uniqueness for its fixed t. QED

Exclusion certificate C0

A box is certified empty if any sound interval exclusion test proves

Z_{pq} intersect box = empty.

Examples include 0 notin G(box) or a certified interval-Newton/Krawczyk exclusion. The theorem package does not require one particular exclusion formula.

Deterministic marching-coordinate selection

A chart chooses the first coordinate, under a fixed canonical order, whose certified 3 x 3 Jacobian minor excludes zero with a prescribed margin. Adaptive floating-point comparison without a certified tie rule is not permitted.

## 9. Finite certified contact atlas

Hypotheses A1-A4

For one admitted patch pair assume:

A1. U_p x U_q is compact.

A2. G is C^1 on an open neighborhood of the domain.

A3. rank DG = 3 on Z_{pq} with uniform margin mu_tr > 0.

A4. The interval evaluation used by C0/C1 converges under box subdivision: on shrinking boxes it converges to the point value/Jacobian in the standard inclusion-isotone sense.

Theorem 11 — finite atlas plus exclusion cover

Under A1-A4 there exist two finite families of certified boxes:

{X_i}: C1 graph-chart boxes,
{E_j}: C0 zero-free boxes,

such that

Z_{pq} subset union_i X_i

and

U_p x U_q subset (union_i X_i) union (union_j E_j).     (10a)

Thus the chart boxes cover the entire zero set and the exclusion boxes cover every point not already covered by a chart box. The required local certification scale is a geometry-dependent quantity h_top > 0 independent of the requested volume tolerance tau.

Proof. Every point of the compact zero set has, by A3 and the implicit-function theorem, an open neighborhood on which some coordinate is a valid marching parameter and the zero set is a graph. By A4, a sufficiently small certified box inside that neighborhood satisfies C1. Compactness of Z_{pq} gives a finite collection of C1 neighborhoods whose union contains Z_{pq}. Because this union is open around the zero set, the compact remainder of the parameter product outside a slightly smaller chart neighborhood is disjoint from Z_{pq}; |G| therefore has a positive minimum there. By A4, sufficiently fine interval boxes on that compact remainder satisfy C0. A finite subcover gives the E_j. The maximum of the finitely many required local resolutions defines a positive h_top depending only on geometry and conditioning. QED

The theorem is a cover theorem, not a claim that an arbitrary axis-aligned partition cell intersecting the curve must itself be a full graph chart. Chart and exclusion boxes may overlap.

Definition — topology cost

Let

C_top(p,q)

be the cost of constructing and certifying such a finite chart/exclusion cover, including deterministic chart selection and chart-overlap bookkeeping. C_top depends on geometry and conditioning but not on tau.

Theorem 12 — no missed closed component

If the topology pass produces a certified cover satisfying (10a), then every connected component of Z_{pq} is covered by C1 charts; in particular, a closed contact loop wholly inside the parameter domain cannot be missed.

Proof. Every point of Z_{pq} is contained in the chart union by Theorem 11. Hence every connected component is contained in that union regardless of whether it meets the external patch boundary. QED

For full component reconstruction, the implementation additionally certifies chart continuation/overlap adjacency. This is an explicit obligation in Section 19.

## 10. Certified trace approximation

Let one certified graph component be parameterized by arc/marching parameter

c : I -> R^4.

On a span I_i of length h_i, construct a deterministic degree-k polynomial approximant

c_tilde_i.

The approximant may be Hermite, Taylor, Chebyshev, or another fixed scheme, provided it has a certified remainder theorem.

Assume c has k+1 derivatives on the span and the implementation certifies

H_i >= sup_{t in I_i} ||c^{(k+1)}(t)||.

Let C_k be the scheme's proven remainder constant.

Theorem 13 — certified trace tube

If the approximation scheme satisfies its standard order-k+1 remainder theorem, then

rho_i
  = C_k H_i h_i^(k+1)                                  (10)

is a certified Euclidean radius such that

c(I_i) subset Tube(c_tilde_i, rho_i).

Proof. Apply the approximation remainder formula and replace all derivative quantities by outward-rounded certified upper bounds. QED

Projection from R^4 to the source (u,v) coordinates is non-expansive in the Euclidean norm. Therefore the projected true contact arc lies within radius rho_i of the projected nominal polynomial arc.

At a parameter seam where C^{k+1} regularity is unavailable, the chart is split. The local approximation order is reduced to the highest degree supported by certified derivative regularity on that side. No derivative is implicitly continued across a knot or seam.

## 11. Tube area and selector uncertainty

For a rectifiable nominal projected arc of length L_i, let T_i(rho_i) be its planar radius-rho_i neighborhood.

Theorem 14 — planar tube-area upper bound

For each span,

area(T_i(rho_i))
  <= 2 L_i rho_i + pi rho_i^2.                         (11)

For a union of spans, summing the right side remains a valid upper bound even when tubes overlap.

Let

M_i >= sup_{(u,v) in T_i} |g_p(u,v)|

and let S_i be the diameter of the still-admissible selector values on the tube. Since selectors lie in {-1,0,+1},

0 <= S_i <= 2.

Theorem 15 — trace-tube ambiguity bound

If outside the certified tube the selector is resolved, then the contribution interval needed for span i has width at most

w_i
  <= S_i M_i (2 L_i rho_i + pi rho_i^2).               (12)

Proof. Any two admissible selector functions can differ only in the tube and pointwise by at most S_i. Hence the difference of their flux contributions is at most S_i integral_T |g_p|, which is bounded by S_i M_i area(T). Apply (11) and Theorem 3. QED

For sufficiently small rho_i, the leading term is

w_i = O(S_i M_i L_i rho_i).

## 12. Certified parameter-domain arrangement

Tracing a single pair is not enough to classify a whole source patch. Contact arcs from all relevant opposing patches must be combined.

For each source patch p, let the projected nominal trace network be

A_p.

Let T_p be the union of all certified trace tubes and all certified small neighborhoods reserved for isolated contact vertices, seam events, or unresolved trace junctions.

The implementation constructs a deterministic planar arrangement of the nominal trace arcs, patch edges, and certified event vertices.

Arrangement obligations R1-R4

R1 — coverage. Every true transversal contact point on p lies in T_p.

R2 — certified complement topology. The implementation enumerates every connected component of

U_p \ T_p

or a certified polygonal/curvilinear subset with the same component structure.

R3 — no hidden membership boundary. On each complement component there is no point where p meets an opposing operand boundary.

R4 — seed classification. At least one point in every complement component receives a certified Boolean selector value.

Theorem 16 — selector constancy on clear arrangement faces

Under R1-R4, the Boolean selector is constant on every connected component of U_p \ T_p and equals its certified seed value.

Proof. The selector can change only when the source surface crosses an opposing boundary or a separately certified singular event. R1-R3 exclude such a crossing from the component. A continuous path between any two points in a connected component therefore remains in the same inside/outside class. R4 fixes that class. QED

Thus the only residual selector uncertainty on a traceable regular source patch is confined to T_p plus explicitly isolated event neighborhoods.

## 13. Flux over resolved arrangement faces

Let R subset U_p \ T_p be a resolved arrangement face with selector value s_R.

Polynomial patch case

If g_p(u,v) is polynomial and the face boundary consists of polynomial parametric arcs, choose a polynomial Q(u,v) with

partial Q / partial u = g_p.

Green's theorem gives

integral_R g_p du dv
  = contour_integral_{dR} Q dv.                        (13)

Composition of polynomial Q with a polynomial boundary arc is polynomial, so the line integral can be evaluated algebraically exactly over the reals and enclosed only by outward rounding.

Theorem 17 — exact polynomial resolved-region flux

Under the polynomial hypotheses above, the resolved-region contribution

s_R integral_R g_p

has no approximation error apart from certified arithmetic enclosure.

QED

Rational patch case

For rational patches, g_p is generally rational and clearing denominators does not make the desired integral polynomial. The implementation must provide a certified rational region integrator.

Define its returned interval width on resolved face R as

eta_int(R).

Proof obligation RI1 — rational resolved-region integration

For every admitted rational patch and polynomial/approved boundary arc, prove:

the exact resolved-region flux lies in the returned interval;

the interval width eta_int(R) is explicitly bounded;

the runtime of obtaining a requested integration width is explicitly bounded.

The reciprocal-power/weight machinery may discharge this obligation; the theorem package does not assume a particular construction.

Resolved-region integration error participates in the same global bracket as trace-tube and fallback-cell error.

## 14. Tracing runtime

Consider a trace component split into degree-k approximation spans. For readability first assume uniform bounds

H_i <= H,
M_i <= M,
S_i <= S,

and total nominal projected length

L = sum_i L_i.

Let every span have marching length at most h. Standard regularity of the chart converts marching length to projected arc length by a geometry-dependent bounded speed factor; absorb it into the constants below.

From Theorems 13 and 15,

W_trace
  <= C_trace,1 * L * H * h^(k+1)
     + C_trace,2 * n_span * H^2 * h^(2k+2),             (14)

where C_trace,1 contains S, M, projection-speed bounds, and C_k, and C_trace,2 contains the quadratic tube-area constants.

Since

n_span = O(L/h),

the quadratic term is higher order for k >= 0 as h -> 0.

Theorem 18 — certified transversal tracing upper bound

For fixed geometry satisfying the atlas and derivative hypotheses, a target trace residual width tau_tr > 0 is achieved with

h
  = O( (tau_tr / (C_tr H L))^(1/(k+1)) )               (15)

and therefore

n_span
  = O(
      L * (C_tr H L / tau_tr)^(1/(k+1))
    ).                                                   (16)

The total trace cost is

C_top
  + O(T_span * n_span)
  + C_arr(n_span)
  + C_int,                                               (17)

where:

C_top is the tolerance-independent certified atlas/topology cost;

T_span is the bounded cost of one certified span construction and tube bound;

C_arr is the deterministic arrangement cost;

C_int is resolved-region integration cost.

Proof. Substitute the certified approximation radius rho = O(H h^(k+1)) into the leading tube-width term and choose h so that it is at most tau_tr. The quadratic tube-area term is then asymptotically smaller; if desired it may be separately budgeted by taking the minimum with its sufficient bound. The number of spans is O(L/h). QED

For fixed L,H,C_tr, the tolerance exponent is

O(tau_tr^(-1/(k+1))).                                  (18)

For cubic order k=3, this is

O(tau_tr^(-1/4)).

Matching lower bound for the chosen approximation family

A Theta(tau^(-1/(k+1))) statement requires a nondegeneracy hypothesis: on a positive fraction of trace length, every allowed degree-k approximant must incur normal deviation at least

c_k h^(k+1)

and the local selector/flux uncertainty must be bounded below by positive constants. Under those hypotheses, the span count in (16) is asymptotically tight for that approximation family.

The theorem is therefore an unconditional upper bound under certified smoothness, and a matching Theta bound only when the explicit lower-approximation hypotheses are also proved.

## 15. Isolated events

Triple-contact vertices, certified trace endpoints, seam crossings, and other zero-dimensional residual events need not be forced into the trace theorem.

Let a quadtree parameter cell have side h and assume on the stable isolated-event regime

w(cell) <= K h^2.                                       (19)

If at most A_0 unresolved cells persist per level:

Theorem 19 — isolated-event subdivision cost

A total isolated-event residual at most tau_0 is obtained after

O( A_0 log_+(K A_0 / tau_0) )                          (20)

processed cells.

Proof. At level ell, h=2^-ell and aggregate width is at most K A_0 4^-ell. Solve for the level needed to make this at most tau_0; only O(A_0) cells persist per level. QED

Under a matching positive lower-width law on a fixed fraction of these chains, the logarithmic dependence is tight.

## 16. Covering fallback for unresolved curves

The high-order trace theorem is the primary path for regular transversal contact, but a sound cell-covering fallback remains useful for unsupported trace configurations, resource exhaustion, or deliberately simpler implementations.

Let M(h) be the number of unresolved quadtree cells of side h in a stable fallback regime. Assume

M(h) <= A_d h^(-d),       0 <= d < 2,                  (21)

and

w(cell) <= K h^2.                                       (22)

Theorem 20 — subdivision residual upper bound

After refining all stable unresolved cells to side at most h,

W_sub(h) <= K A_d h^(2-d).                             (23)

QED

Theorem 21 — subdivision work for 0<d<2

A residual at most tau_sub is obtained after

O(
  A_d * (K A_d / tau_sub)^(d/(2-d))
)                                                        (24)

stable unresolved cells.

For a regular curve (d=1),

O(K A_1^2 / tau_sub).                                  (25)

For isolated points (d=0), Theorem 19 gives the logarithmic law rather than O(1).

Under matching lower cover and local-width laws, (24) is tight for solvers that remove the same uncertainty only by cell subdivision.

Consequence

The 1/tau curve law is a theorem about the covering strategy, not about certified Boolean volume itself. The tracing theorem of Section 14 supersedes it on the regular traceable class.

## 17. Near-coincident but distinct support

A distinct-support pair can remain ambiguous over a two-dimensional coarse region before geometry localization becomes fine enough to separate it.

Let the physical support gap on a parameter region of area alpha be at least

delta > 0.

Suppose the certified hull localization error satisfies

radius(hull(cell), image(cell)) <= Lambda h.            (26)

Theorem 22 — near-coincidence conditioning cost

Once

2 Lambda h < delta,                                     (27)

the two localized supports are certifiably separated. Covering area alpha down to this scale costs

O( alpha (Lambda/delta)^2 )                            (28)

quadtree cells.

Proof. The critical side length is h = Theta(delta/Lambda). Covering parameter area alpha by side-h squares requires Theta(alpha/h^2). QED

This is a finite conditioning term independent of the final volume tolerance once separation is certified.

## 18. Persistent ambiguity floor

Although positive-area co-support is resolved or refused at admission, the general obstruction is useful to state independently.

Theorem 23 — persistent ambiguity-diameter obstruction

Suppose every partition obtainable without adding a stronger geometric certificate satisfies

sum_j Delta(p,C_j) >= Delta_* > 0.                     (29)

Then no subdivision-only execution preserving the same selector-information model can return a global bracket narrower than Delta_*.

Proof. Apply Theorem 3 on every region and sum. QED

This theorem is immune to flux cancellation because it is stated in terms of the epistemic contribution diameter, not the signed integral of one arbitrary ambiguity set.

The correct response is to add a geometric theorem, resolve the support algebraically, or refuse. Infinite subdivision is never required.

## 19. Complete contact-topology obligations

The finite-atlas theorem guarantees local graph coverage. A production contact tracer additionally discharges the following finite topological obligations.

T1 — complete chart cover

Every zero belongs to at least one certified C1 chart; every non-chart leaf is certified C0.

T2 — continuation adjacency

For every chart endpoint not on a parameter-domain boundary, the algorithm certifies which neighboring chart continues the same zero branch. Ambiguous adjacency is subdivided until certified or refused.

T3 — component closure

Every reconstructed branch is certified to terminate either:

at a parameter-domain boundary/seam event;

at a separately certified isolated event; or

by returning to its initial chart, forming a closed component.

T4 — projection coverage

The projected tubes of the reconstructed branches contain every true contact point on the source patch.

T5 — trace-junction handling

Intersections among nominal projected traces, seam coincidences, and multi-patch contact vertices are isolated into certified event neighborhoods unless a stronger exact arrangement theorem resolves them.

Theorem 24 — topology-pass soundness

If T1-T5 hold, the subsequent arrangement construction cannot miss an interior closed contact loop or an unrepresented contact branch.

Proof. T1 covers the full zero set locally. T2-T3 reconstruct every local piece into a complete branch/component. T4 transfers that coverage to the source parameter domain. T5 removes non-generic junctions from the regular-arc assumptions and reserves them for separate certified treatment. QED

## 20. Global residual scheduler

After geometric preprocessing, the remaining uncertainty objects have two principal forms:

TraceSpanResidual
FallbackCellResidual

plus fixed resolved-region integration intervals.

Each residual object x carries:

Phi(x) = [lo_x,hi_x],
width(x) = hi_x-lo_x,
canonical_id(x).

Refining a trace span bisects its marching interval and reconstructs certified daughter approximants/tubes. Refining a fallback cell performs a quadtree split.

A simple deterministic scheduler is parameterized by a certified stopping predicate gate:

REFINE_GLOBAL(gate, budgets):
    Q <- all refinable residual objects,
         ordered by descending width,
         tie-break by canonical_id

    while not gate(global bracket):
        if Q is empty:
            return REFUSE(BudgetExceeded, bracket)

        if any applicable finite budget is exhausted:
            freeze or refuse according to the typed policy

        x <- Q.pop()
        children <- CERTIFIED_REFINE(x)
        replace x by children in the global interval tree
        insert refinable children in Q

    return ACCEPT(bracket_or_decision, certificate)

For numerical volume mode, gate is the absolute or mixed-width test of Section 4. For decision mode, it is one of the threshold/comparison predicates of Theorem 5A. The refinement machinery is unchanged.

Theorem 25 — scheduler and gate soundness

Any scheduler that replaces a residual object only by a certified enclosure of the same semantic contribution preserves Theorem 4, independently of refinement order. If it terminates through a gate proved sound in Section 4, the returned numerical tolerance statement or decision is therefore correct.

QED

Theorem 26 — totality with finite budgets

If topology subdivision, trace-span refinement, fallback-cell refinement, maximum depth, and arithmetic work all have finite configured budgets, every invocation terminates with either acceptance or typed refusal.

Proof. Every loop step consumes a finite resource or removes a refinable object. No resource can be consumed infinitely often. QED

A no-progress detector may accelerate refusal but is not needed for totality.

## 21. Best-first scheduling theorem

Best-first is an efficiency policy, not a soundness condition.

Assume a residual-object class has the property that refining an item of width w reduces aggregate width by exactly

theta w,     theta in (0,1),                            (30)

with the same theta for every item in that class.

Theorem 27 — greedy optimality under uniform contraction

Under (30), repeatedly refining a maximum-width available item minimizes the number of refinements needed to reach any fixed global-width threshold.

Proof sketch. The immediate gain is proportional to current width. An exchange argument swaps any refinement of a smaller available item with a larger one without reducing cumulative progress. Repeating produces maximum-width order with no larger schedule length. QED

The real trace and cell engines need not satisfy uniform contraction. Their runtime theorems come from Sections 14-17, not from Theorem 27.

## 22. Per-operation computational costs

Let D be the maximum tensor-product patch degree.

Define:

T_pair       cost of one certified BVH/contact pair test
T_cell(D)    cost of one fallback cell split + membership/integration update
T_span(D,k)  cost of one certified trace-span construction/refinement

For tensor-product de Casteljau subdivision,

T_cell(D) = O(D^3 + T_mem(D)).                          (31)

If membership/contact evaluation is also O(D^3), then

T_cell(D) = O(D^3).

T_span includes parametric interval-Newton/Krawczyk certification, derivative enclosures through order k+1, approximation construction, and tube bounds. Its concrete degree dependence is an implementation theorem and is kept symbolic here.

## 23. End-to-end runtime: fully traced regular class

Define:

N = N_A + N_B, source-patch count;

C_leaf_flux, one-time cost of obtaining whole-patch flux certificates needed by hierarchical aggregation when they are not already available;

H_vis, source-BVH node visits made by hierarchy-first whole-node membership retirement;

T_hmem, cost of one certified whole-node membership query;

P_vis, certified BVH node-pair visits remaining for pair/contact search;

C_cosup, finite co-support decomposition/integration cost;

C_near = sum_j alpha_j (Lambda_j/delta_j)^2;

C_top = sum C_top(p,q) over traceable transversal pair components;

trace components indexed by j, each with degree k_j, projected length L_j, derivative constant H_j, and uncertainty/flux constant Q_j absorbing S_j, M_j, projection conditioning, and the approximation remainder constant;

A_0, isolated-event count constant;

K_0, isolated-event local width constant;

tau the requested absolute global bracket width.

Allocate the global tolerance among trace tubes, isolated events, and resolved-region integration with any deterministic positive budget split. Constant-factor budget splitting does not change exponents.

For trace component j, let its assigned trace budget be tau_j.

Theorem 28 — fully traced admitted-input runtime

Assume:

P1 and E1-E5;

sound hierarchical node-membership certificates wherever subtree retirement is used;

complete co-support resolution/refusal;

sound pair-search lower bounds;

A1-A4 and T1-T5 for every admitted transversal contact pair;

the trace derivative and tube bounds of Sections 10-11;

certified resolved-region integration;

near-coincidence localization (26);

isolated-event width law (19);

finite resource limits are large enough not to trigger before the requested tolerance is achieved.

Then a fully traceable admitted input reaches width tau in

O(
    N log N
  + C_leaf_flux
  + H_vis * T_hmem
  + P_vis * T_pair
  + C_cosup
  + T_cell(D) * C_near
  + C_top
  + sum_j [
        T_span(D,k_j)
        * L_j * (Q_j H_j L_j / tau_j)^(1/(k_j+1))
    ]
  + C_arr
  + C_int
  + T_cell(D) * A_0 log_+(K_0 A_0 / tau_0)
).                                                       (32)

Here C_arr is total certified arrangement work and C_int total resolved-region integration work at their allocated budgets.

Proof. BVH pair search follows from Theorem 8, hierarchy-first retirement from Theorem 8A, and near-coincidence from Theorem 22. Co-support is finite by Theorem 6 and the oracle contract. Each regular contact component contributes its topology cost plus Theorem 18's span count. Isolated events contribute Theorem 19. All contribution intervals are combined by Theorem 4, and the deterministic budget split ensures their widths sum to at most tau. QED

Corollary 28.1 — fixed-complexity regular transversal contact

For fixed geometry, fixed trace degree k, bounded conditioning, bounded arrangement/integration cost per span, and a fixed number of contact components,

T(tau)
  = O(tau^(-1/(k+1)))                                  (33)

up to tolerance-independent setup terms and lower-order logarithmic isolated-event terms.

For cubic trace approximation (k=3):

T(tau) = O(tau^(-1/4)).

This is the primary transversal-contact accuracy law of the integrated specification.

## 24. End-to-end runtime: hybrid with covering fallback

Let F index fallback unresolved regimes for which tracing is not used but the subdivision hypotheses (21)-(22) hold. Give regime f dimension d_f, cover constant A_f, width constant K_f, and budget tau_f.

Theorem 29 — hybrid runtime

Under the soundness hypotheses above but allowing fallback subdivision, add to (32)

sum_{f: 0<d_f<2}
  T_cell(D)
  * A_f (K_f A_f / tau_f)^(d_f/(2-d_f)),               (34)

plus logarithmic d=0 fallback terms.

In particular, an untraced regular curve fallback contributes

O(T_cell(D) * K_f A_f^2 / tau_f).                      (35)

Therefore:

the regular traced class has the high-order exponent of Theorem 28;

the general hybrid implementation retains the subdivision upper bound on cases that fall back;

no soundness theorem depends on successful tracing.

If tracing failure is only a finite implementation resource failure, raising the resource budget may restore the stronger traced-class theorem. If a mathematical chart/regularity hypothesis fails, the input belongs to a different geometric class and must use another certified theorem or fallback/refusal.

Corollary 29.1 — sparse union-fold decomposition

For a union of m operands, let C_1,...,C_r be the certified interaction-graph components of Theorem 8B. If T(C_a) denotes the complete certified Boolean cost of component C_a, then

T_union
  = O(m + |E| alpha(m)) + sum_a T(C_a),                 (35a)

apart from the cost already paid to obtain the may-overlap relation. Cross-component Boolean refinement work is exactly zero.

If the interaction graph has uniformly bounded component size s and a monolithic orchestration would otherwise contain a quadratic operand-pair term, that term becomes O(m s).

Corollary 29.2 — decision-mode runtime

For any theorem above whose tolerance-dependent work is monotone in the requested width, a threshold or comparison workload may replace that width by the certified decision separation margin of Theorem 5A. Thus, on fixed regular degree-k traced geometry,

T_decision = O(delta_dec^(-1/(k+1))),                   (35b)

up to the same tolerance-independent setup terms. On an untraced regular-curve covering fallback,

T_decision = O(1/delta_dec).                            (35c)

These are worst-case sufficient bounds for a nonzero decision margin, not promises that every decision reaches the bound.

## 25. Determinism and parallel execution

Assign canonical identities to:

operands and patches;

BVH nodes;

topology boxes;

graph charts;

trace branches and spans;

arrangement vertices/edges/faces;

fallback dyadic cells.

All choices use fixed lexicographic rules after certified comparisons. Interval reductions use a fixed binary reduction tree.

Independent-object SIMD batching

Membership tests, fallback-cell updates, and trace-span bound evaluations for independent residual objects may be executed in SIMD lanes. This vectorizes across objects, leaving each lane's certified scalar semantics unchanged.

A vector interval kernel must still prove outward enclosure in every lane. Storing an interval as [-lo,+hi] can allow both stored endpoints to use a common upward-directed rounding mode for operations whose transformed endpoint formulas prove that property; it does not eliminate the outward-rounding obligation and is not assumed for unsupported operations.

Theorem 29A — lane-batch soundness

Suppose a width-w SIMD kernel computes exactly the same certified interval operator as the scalar implementation in each lane, with outward rounding/error inflation proved lane-wise. Then batching w independent residual objects preserves every scalar soundness theorem. If lane assignment and result merge order are canonical, it also preserves deterministic output.

Proof. The lanes have no semantic dependence. Apply the scalar enclosure theorem independently to each lane and merge the resulting certified intervals in the same canonical reduction tree. QED

SIMD changes only constants. For m homogeneous independent queries, the ideal arithmetic issue count changes from m scalar invocations to ceil(m/w) vector batches, subject to occupancy, divergence, and vector-kernel overhead. No asymptotic exponent in tau changes.

Theorem 30 — sequential determinism

On a fixed execution platform and arithmetic configuration, the sequential algorithm produces a deterministic certificate and bit-identical interval endpoints.

Proof. Every branch choice, split, chart coordinate, priority tie, arrangement ordering, and reduction order is a deterministic function of the certified input state. QED

Theorem 31 — deterministic parallel composition

A batched implementation remains deterministic if:

the batch membership is selected deterministically;

local tasks are pure functions of their certified inputs;

results are merged in canonical order through the fixed reduction tree.

The batch schedule may differ from sequential best-first and therefore does not inherit Theorem 27's exact greedy-optimality claim. The structural tolerance exponents remain unchanged.

## 26. Refusal taxonomy

The integrated specification uses explicit typed refusals.

Geometry/admission

CoincidentSupportUncertified — possible positive-area shared support could not be resolved or excluded.

TransversalityUncertified — a path requiring regular contact could not certify the rank/transversality margin.

UnsupportedPatchRepresentation — required contact, flux, derivative, or weight bounds are unavailable.

SeparationCertificateUnavailable — pair pruning lacks a certified lower bound for the requested objective.

Topology/tracing

TraceTopologyBudgetExceeded — the C0/C1 topology pass hit its finite budget before full certification.

TraceChartUncertified — a required graph chart could not be certified within the allowed local budget.

TraceAdjacencyUncertified — branch continuation/component topology could not be certified.

TraceRegularityInsufficient — requested approximation degree lacks certified derivative regularity; the implementation may lower order before refusing.

ArrangementUncertified — projected trace/event arrangement could not be certified.

Integration/refinement

ResolvedFluxUncertified — the resolved-region integrator cannot provide a sound bound.

RefinementBudgetExceeded — global residual refinement consumed its configured budget.

DepthLimitExceeded — unresolved fallback cells reached maximum depth.

ArithmeticResolutionInsufficient — only if the concrete arithmetic layer itself proves a nonzero attainable-width floor.

Every refusal carries the current sound global bracket and observability counters.

Fallback policy

Failure of the trace fast path may fall back to sound cell covering only if the covering path's selector enclosure remains valid without assuming successful topology reconstruction.

In particular, an incomplete trace topology pass must never be treated as evidence that the untraced region is clear. It may be discarded entirely and replaced by the independent covering path, or the call may refuse.

## 27. Concrete proof obligations

The generic theorems become theorems about a concrete CAD kernel only after these obligations are discharged.

P1. Boolean flux semantics

Prove equation (1) for the exact orientation/regularization convention.

P2. Epistemic selector enclosure

For every certified region type, prove E1-E3 with directed rounding.

P3. Additive interval composition

Prove E4-E5 for patch splitting, co-support decomposition, arrangement faces, trace tubes, and fallback cells.

P4. Co-support completeness for admission

Every persistent positive-area support overlap is resolved into certified shared regions or refused.

P5. Pair-search objective bounds

Every BVH pruning function is proved to lower-bound the exact objective it prunes.

P6. Quantitative transversality

For every trace-admitted pair, certify a positive rank margin mu_tr or an equivalent bound sufficient for the selected chart theorem.

P7. Parametric chart primitive

Prove the C1 parametric interval-Newton/Krawczyk inclusion theorem for the implementation's exact operator, and prove soundness of every C0 exclusion predicate.

P8. Finite topology reconstruction

Implement and certify T1-T5, including interior loops and branch adjacency.

P9. Trace derivative bounds

For each supported patch representation and trace degree, certify the implicit derivatives required through order k+1, respecting knot/seam continuity.

P10. Approximation remainder

Prove the constant C_k and tube radius formula used by each trace approximant.

P11. Tube flux bound

Prove the flux supremum M_i, selector diameter S_i, and geometric tube-area bound used in (12).

P12. Arrangement correctness

Prove R1-R4 for the nominal trace/tube arrangement on every source patch.

P13. Resolved-region flux

For polynomial patches, discharge the exact Green-integral construction. For rational patches, prove the soundness/runtime contract RI1 of Section 13.

P14. Fallback local width law

For every subdivision fallback class, prove a certified constant K satisfying (22).

P15. Fallback cover law

Prove or certify the relevant A_d,d cover law for the fallback geometry class.

P16. Near-coincidence localization

Prove the constant Lambda in (26) wherever near-coincident distinct supports are admitted.

P17. Deterministic arithmetic contract

Fix rounding mode, FMA policy where relevant, canonical identities, tie-breaking, chart selection, split locations, and reduction order.

P18. Hierarchical node membership and aggregate flux

Prove that every EntireInside/EntireOutside node result applies to every descendant source point, that boundary contact is excluded on retired nodes, and that each node's aggregate flux annotation encloses the outward-rounded sum of its descendant patch fluxes for the current placement.

P19. Interaction-graph disjointness semantics

For every absent interaction edge used by an operation-specific decomposition, prove the set-disjointness statement required by that rewrite. Use Theorem 8B only for union components unless another Boolean identity has been separately proved.

P20. Cache identity and invalidation

Operand-volume cache keys must name immutable geometry construction/version state and the certificate precision. Reuse under placement is permitted only for transformations covered by Theorem 8C. Geometry mutation or a stricter requested certificate invalidates or refines the cached entry deterministically.

P21. Vector interval kernels

For every SIMD interval primitive used in lane batching, prove lane-wise outward enclosure under the configured rounding/error-inflation scheme and prove deterministic canonical merge semantics.

Once P1-P21 are discharged, Theorems 1-31 together with Theorems 5A, 8A-8C, and 29A apply directly to the implementation.

## 28. Capability and complexity table

geometric regime

primary certified treatment

tolerance dependence

clear separated BVH subtree

whole-node membership + aggregate flux retirement

none

exact/partial positive-area co-support

algebraic decomposition + Theorem 6

none apart from flux integration

possible positive-area co-support, uncertified

typed refusal

finite

regular transversal contact, degree-k trace

finite contact atlas + polynomial trace tube

O(tau^(-1/(k+1))) for fixed geometry

cubic regular trace

same

O(tau^(-1/4))

isolated contact/event

fallback quadtree

Theta(log(1/tau)) under matching regularity

untraced regular contact curve

cell-cover fallback

Theta(1/tau) for fixed geometry under matching regularity

general fallback dimension 0<d<2

cell covering

Theta(tau^(-d/(2-d))) modulo structural constants

near-coincident distinct supports

refine until certified gap separation

Theta(alpha (Lambda/delta)^2) when tight; tolerance-independent afterward

persistent ambiguity diameter Delta_*>0

stronger theorem or refusal

cannot be removed by same-information subdivision

sparse multi-solid union

interaction-graph component decomposition

sum of component costs; no cross-component refinement

decision-only query

stop when certified bracket decides predicate

replace numerical tolerance by nonzero decision margin

repeated rigid placement of same closed operand

cached operand-volume certificate

cache lookup after first required precision

independent homogeneous residual queries

SIMD lane batching with certified vector intervals

constant-factor only

resource exhaustion

typed refusal with current sound bracket

finite by configured budgets

The important hierarchy is:

positive-area support theorem
    > high-order contact theorem
        > isolated-event theorem
            > generic cell-cover fallback

The engine should always use the strongest certified geometric representation available before spending tolerance budget on subdivision.

## 29. Principal end-to-end theorem

Theorem 32 — certified Boolean volume by contact atlas and residual refinement

Consider a regularized Boolean operation on finitely patched oriented solids. Assume P1-P21 for every optimization that is enabled. Run the integrated algorithm with finite resource budgets.

Then:

Totality. Every invocation terminates with acceptance or typed refusal.

Soundness. Every returned bracket contains the true regularized Boolean volume.

Tolerance/decision correctness. Every accepted numerical result satisfies the requested global absolute or mixed error gate; every accepted threshold/comparison result satisfies its certified decision predicate.

Co-support safety. Persistent positive-area support ambiguity is resolved algebraically or refused before asymptotic tracing/subdivision.

Topology completeness. Every contact component in the trace-admitted transversal class, including an interior closed loop, is represented by the certified atlas/tube system.

Traced-class runtime. On fully traceable regular contact with degree-k approximation and fixed geometry, the tolerance-dependent regular-contact work is

O(tau^(-1/(k+1))).

Isolated-event runtime. Stable isolated events contribute logarithmic work in 1/tau.

Fallback runtime. Any geometry intentionally or necessarily handled by cell covering obeys the dimension-dependent bound (24); a regular untraced curve has the 1/tau covering law.

Near-coincidence runtime. Distinct supports with certified gap delta incur the finite conditioning term (28), not a persistent two-dimensional tolerance exponent.

Hierarchy-first clear-region runtime. After any required one-time leaf-flux acquisition, certified subtree retirement adds O(N + H_vis T_hmem) and may remove all descendant per-patch work on resolved nodes; worst-case H_vis=O(N) per source tree without stronger spatial assumptions.

Broad-phase runtime. Candidate-pair discovery is O(N log N + P_vis T_pair) with worst-case P_vis=O(N_A N_B) unless a stronger packing theorem is proved.

Sparse union-fold decomposition. Multi-solid unions split exactly across certified interaction-graph components as in Corollary 29.1.

Rigid-volume memoization. Closed operand-volume certificates are reusable under rigid placement according to Theorem 8C.

SIMD batching. Independent-object lane batching preserves scalar soundness when P21 holds and changes constants only.

Determinism. The certificate and interval endpoints are deterministic on a fixed execution platform under P17 and P21 where SIMD is enabled.

QED by Theorems 4, 5A, 6, 7, 8, 8A, 8B, 8C, 11-22, 25-31, 29A, and the proof obligations P1-P21.

## 30. Design consequence

The integrated theorem changes the interpretation of the solver's runtime frontier.

A transversal contact curve is not intrinsically a 1/tau certified-computation problem. It is a 1/tau problem only if the solver represents the unknown selector by a first-order area cover of that curve.

Once the same regular contact is promoted to a certified geometric object,

G^{-1}(0)
  -> finite graph atlas
  -> degree-k trace
  -> radius O(h^(k+1)) tube
  -> residual flux O(h^(k+1)),

the accuracy exponent becomes

tau^(-1/(k+1)).

The subdivision engine remains valuable because it supplies a small, representation-independent soundness kernel for isolated events, fallback geometry, and refusal-safe degradation. It is no longer the theorem that defines the regular-contact performance ceiling.

The resulting architecture is therefore:

multi-solid interaction graph when applicable
    -> BVH build + aggregate node-flux annotation
    -> hierarchy-first whole-subtree membership retirement
    -> residual candidate-pair search
    -> positive-area co-support resolution or refusal
    -> certified transversal contact atlas
    -> high-order deterministic trace tubes
    -> certified parameter-domain arrangement
    -> exact/certified resolved-region flux
    -> global trace-tube residuals
    -> best-first fallback subdivision for leftovers
    -> numerical tolerance gate or certified decision gate

Rigid-invariant operand-volume memoization removes repeated closed-volume work across dispatches, while independent-object SIMD batching reduces constant factors without changing any theorem exponent. Full incremental reuse of contact/topology certificates across edit-graph revisions remains a future optimization because its dependency/invalidation surface is materially larger.

This is the intended certified Boolean volume architecture.
