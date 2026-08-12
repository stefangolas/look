# I. Purpose and status of the formal system

We want a system with the following property:

Every normalized STEP face within a declared envelope is assigned exactly one semantic outcome belonging to a fixed formal language. No new geometric instance within that envelope requires inventing a new semantic category or an exporter-specific repair.

This is symbolic closure, not universal STEP support.

A newly encountered face may still be:
- valid and realizable;
- valid but not yet realizable;
- inconsistent;
- ambiguous;
- unsupported by the envelope;
- unresolved because a required numerical proposition could not be certified.

What must not occur is an untyped “unexpected geometry” state.

The atlas is therefore not principally a finite list such as:
- disk
- annulus
- apex disk
- spherical cap
- torus band

It is a formal system whose canonical objects can be assigned such names as derived summaries.

# II. Foundational distinctions

We distinguish five levels.

### Definition 1: Source evidence

Source evidence $E$ consists of the retained STEP entities and their interpreted relations:

$$E = (S_{\text{src}}, B_{\text{src}}, U_{\text{src}}, V_{\text{src}}, C_{\text{src}}, P_{\text{src}}, T_{\text{src}})$$

where these include:
- support-surface identity;
- face bounds;
- edge uses;
- vertices;
- 3D edge curves;
- pcurves and trim representations;
- orientation wrappers;
- units and transforms;
- source entity identities.

Source evidence is not yet a material region.

### Definition 2: Geometric fact

A geometric fact is a proposition accompanied by an epistemic status:

$$\text{Fact}(P, \kappa)$$

where $\kappa \in \{\text{Declared}, \text{Analytic}, \text{CertifiedNumerical}, \text{Assumed}, \text{Unresolved}\}$.

A production implementation may rely on a fact only if its status meets the requirement of the consuming stage.

For example:
- STEP explicitly declares an edge-use identity: $\text{Declared}$;
- an analytic cylinder has angular period $2\pi$: $\text{Analytic}$;
- two spline arcs intersect transversely once according to an interval-certified solver: $\text{CertifiedNumerical}$;
- “the surface appears injective from sampling”: not sufficient for an authoritative topological decision.

### Definition 3: Semantic result

A semantic result is one of:

$$\text{SemanticOutcome} ::= \text{Valid}(R) \mid \text{Inconsistent}(I) \mid \text{Ambiguous}(A) \mid \text{Unsupported}(U) \mid \text{Unresolved}(N)$$

These alternatives are mutually exclusive by construction.

### Definition 4: Realization result

Realization is a separate judgment:

$$\text{RealizationOutcome} ::= \text{Realized}(M) \mid \text{RecognizedButUnsupported}(R_u) \mid \text{RealizationFailure}(R_f)$$

A semantically valid region need not yet have an implemented mesher.

### Definition 5: Atlas

The atlas is the tuple

$$\mathcal{A} = (\mathcal{L}, [[\cdot]], \equiv, \text{Can}, \text{Class}, \text{Check})$$

where:
- $\mathcal{L}$ is the language of admissible semantic objects;
- $[[\cdot]]$ gives their denotation;
- $\equiv$ is semantic equivalence under declared gauge transformations;
- $\text{Can}$ computes a canonical representative;
- $\text{Class}$ produces derived human-readable labels;
- $\text{Check}$ evaluates validity and realizability obligations.

# III. The bounded envelope

We define a family of envelopes rather than one hard-coded corpus.

### Definition 6: Complexity bounds

Let $\beta = (r_{\max}, s_{\max}, n_{\max}, e_{\max}, w_{\max}, x_{\max}, v_{\max}, g_{\max})$, where:
- $r_{\max} \le 2$: maximum lattice rank;
- $s_{\max}$: maximum number of collapsed strata;
- $n_{\max}$: maximum number of ordinary native-boundary strata;
- $e_{\max}$: maximum normalized source arcs;
- $w_{\max}$: maximum norm of an arc’s deck displacement;
- $x_{\max}$: maximum certified pairwise intersection count;
- $v_{\max}$: maximum regular arrangement-vertex valence;
- $g_{\max}$: maximum number of resulting arrangement cells or graph elements, as an implementation resource bound.

The exact numerical values are policy. The closure proof requires only that they are finite.

### Definition 7: Admissible support map

An admissible support schema is $\mathcal{A} = (\Omega, \Lambda, N, \Sigma, S, C)$, where:
- $\Omega \subseteq \mathbb{R}^2$ is a connected cover domain with an effective finite representation.
- $\Lambda = L \mathbb{Z}^r$, with $0 \le r \le r_{\max}$, acts by translations on $\Omega$.
- $N$ is a finite set of ordinary native-boundary strata.
- $\Sigma$ is a finite set of certified collapsed strata.
- $S : \Omega \to \mathbb{R}^3$ is continuous and piecewise $C^1$.

On the regular set $\Omega_{\text{reg}} = \Omega \setminus \bigcup_{\sigma \in \Sigma} \sigma$, the induced map $\bar{S} : \Omega_{\text{reg}} / \Lambda \to \mathbb{R}^3$ is an embedding over the certified face neighborhood.

Every failure of injectivity inside that neighborhood is accounted for by:
- a deck translation in $\Lambda$; or
- one declared collapse in $\Sigma$.

$C$ contains certificates for these propositions and for the local topology of every collapsed stratum.

### Definition 8: Parameter quotient

The semantic parameter object is $Q_{\mathcal{A}} = (\Omega / \Lambda) / \Sigma$.

The second quotient notation is schematic: each $\sigma \in \Sigma$ is attached according to its certified collapse relation and local-link schema.

### Definition 9: Admissible normalized arc

An admissible normalized arc is a tuple $a = (\gamma, p, q, \delta, \tau, \ell, \pi)$, where:
- $\gamma : [0,1] \to \Omega$ is a continuous lifted curve;
- $p, q$ are endpoint descriptors;
- $\delta \in \mathbb{Z}^r$ is its deck displacement;
- $\tau$ is its traversal semantics;
- $\ell \in \{+1, -1\}$ is its normalized orientation;
- $\pi$ is source provenance.

Endpoint descriptors are $p, q \in \text{Endpoint} = \text{Regular}(u) \mid \text{Native}(n,t) \mid \text{Singular}(\sigma)$.

Traversal semantics are $\text{Traversal} = \text{Ordinary} \mid \text{FullPeriod}(k) \mid \text{MultiPeriod}(k) \mid \text{DegeneratePoint} \mid \text{Unresolved}$.

For two regular endpoints, the lifted endpoint equation is $\gamma(1) = \gamma(0) + L \delta$.

For a singular endpoint, equality is required only after applying the declared collapse relation.

### Definition 10: Boundary regularity restrictions

The first closed envelope excludes:
- unresolved curve overlaps over positive intervals;
- infinite intersection populations;
- nonisolated intersections;
- noncertified tangential contacts;
- unsupported nonmanifold regular junctions;
- surface sheet ambiguity;
- unbounded winding;
- unbounded curve enclosures.

It admits:
- zero, one, or two periodic coordinates;
- disconnected boundary systems;
- contractible and essential cycles;
- finite transverse intersections;
- native-boundary attachments;
- certified singular attachments;
- holes and islands;
- disconnected material components;
- finite winding and multi-period traversal.

### Definition 11: Envelope $\mathcal{E}_\beta$

A normalized face belongs to $\mathcal{E}_\beta$ exactly when:
- its ambient support schema is admissible;
- it contains at most $e_{\max}$ normalized arcs;
- every deck displacement has norm at most $w_{\max}$;
- every pair of translated admissible arc pieces has at most $x_{\max}$ certified intersections;
- every regular normalized arrangement vertex has valence at most $v_{\max}$;
- all required curve enclosures are bounded;
- all topology-changing predicates are certified, or else the face is classified Unresolved;
- the normalized arrangement remains below the resource bound $g_{\max}$.

Notice point 7: a numerically uncertain face still has an atlas outcome. It simply does not have a valid-region outcome.

# IV. Formal language of the atlas

The formal language has four principal sorts: $\text{Ambient}$, $\text{Boundary}$, $\text{Region}$, $\text{Outcome}$.

### A. Ambient terms

Ambient terms are generated by:

$$A ::= \text{Patch}(D, S, C) \mid \text{Periodize}(A, p) \mid \text{Collapse}(A, \sigma, \lambda_\sigma) \mid \text{AddNativeBoundary}(A, n)$$

Here:
- $D$ is a represented cover domain;
- $p$ is a deck generator;
- $\lambda_\sigma$ is the certified local-link description of a collapsed stratum.

This grammar is structural. “Cylinder,” “cone,” “sphere,” and “torus” are constructors or certified adapters yielding ambient terms.

Examples:
- $A_{\text{plane}} = \text{Patch}(\mathbb{R}^2, S, C)$
- $A_{\text{cyl}} = \text{Periodize}(\text{Patch}(\mathbb{R}^2, S, C), p_u)$
- $A_{\text{cone}} = \text{Collapse}(A_{\text{cyl}}, \sigma_{\text{apex}}, \text{CircleLink})$
- $A_{\text{torus}} = \text{Periodize}(\text{Periodize}(\text{Patch}(\mathbb{R}^2, S, C), p_u), p_v)$

### B. Boundary terms

Boundary terms are generated by:

$$B ::= \text{Arc}(a) \mid \text{Concat}(B_1, B_2) \mid \text{Union}(B_1, B_2) \mid \text{Reverse}(B) \mid \text{Subdivide}(B, T) \mid \text{DeckTranslate}(B, k) \mid \text{NormalizeIntersections}(B)$$

This syntax is not itself canonical. It expresses constructions that denote an embedded directed quotient boundary complex.

### C. Arrangement complex

The normalized denotation of a boundary term is a labeled combinatorial map $G = (V, H, \mathcal{C}, \text{orig}, \text{twin}, \text{next}, \text{inc}, d, \eta, \rho)$, where:
- $V$ is a finite vertex set;
- $H$ is a finite half-edge set;
- $\mathcal{C}$ is a finite set of two-cells;
- $\text{orig} : H \to V$;
- $\text{twin} : H \rightharpoonup H$;
- $\text{next} : H \to H$;
- $\text{inc} : H \to \mathcal{C} \times \mathcal{C}$ gives left and right cells;
- $d : H \to \mathbb{Z}^r$ gives deck transition;
- $\eta$ labels native and singular attachments;
- $\rho$ records source-provenance equivalence classes.

Physical boundary arcs and artificial decomposition arcs are distinguished: $\text{kind}(h) \in \{\text{Physical}, \text{ArtificialCut}, \text{NativeBoundary}, \text{SingularLink}\}$.

### D. Region terms

A region term is $R = (A, G, \mu)$, where $\mu : \mathcal{C} \to \{0,1\}$ selects nonmaterial and material two-cells.

The human-readable atlas cell is not $R$. It is a derived label: $\text{Class}(R) = \text{RegionNormalForm}$.

# V. Orientation normalization

This must be normative because material-side semantics depend on it.

### Definition 12: Orientation signs

For each edge use $u$, define signs:
- $s_f(u) \in \{\pm 1\}$ for face support-surface sense,
- $s_b(u) \in \{\pm 1\}$ for face-bound orientation,
- $s_o(u) \in \{\pm 1\}$ for oriented-edge orientation,
- $s_e(u) \in \{\pm 1\}$ for edge-curve sense,
- $s_c(u) \in \{\pm 1\}$ for the direction of the selected curve parameterization or pcurve representation.

Define the normalized traversal sign $s(u) = s_f(u) s_b(u) s_o(u) s_e(u) s_c(u)$.

The exact mapping from STEP Boolean fields to $\pm 1$ is part of the STEP adapter specification and must be verified against the standard and corpus witnesses. The atlas relies only on the resulting normalized sign.

### Orientation axiom

After applying $s(u)$, every physical boundary half-edge is oriented so that material lies locally on its left.

Thus, for every physical half-edge $h$, $\mu(L(h)) = 1, \mu(R(h)) = 0$.

If the implementation chooses the opposite convention, it must do so globally and rewrite every theorem accordingly. Mixing conventions is forbidden.

### Orientation consistency condition

When several normalized source uses map to the same arrangement half-edge, their material-side assignments must agree.

If they do not agree, the result is: $\text{Inconsistent}(\text{ContradictoryBoundaryOrientation})$.

# VI. Curve-on-surface evidence

### Definition 13: Compatible curve-on-surface witness

A UV curve $\gamma$ is compatible with a source 3D edge curve $c$ and surface $S$ when there exists a monotone traversal correspondence $\phi$ such that: $\|S(\gamma(t)) - c(\phi(t))\| \le \varepsilon_{\text{proj}}$ for the certified interval, and:
- endpoints agree within the appropriate source tolerance;
- traversal orientation agrees;
- periodic lift and deck displacement agree;
- no branch or sheet jump occurs;
- singular endpoints agree with declared singular strata;
- the correspondence is continuous.

Compatibility is not established by isolated nearest-point samples alone.

### Evidence precedence rule

A face arc is resolved using the first certified compatible source in:
1. source pcurve;
2. analytic inverse;
3. continuation-tracked numerical inverse;
4. unresolved.

Precedence does not permit accepting incompatible evidence.

### Conflict rule

If two authoritative source representations are individually certified but semantically incompatible, the result is: $\text{Inconsistent}(\text{CurveSurfaceEvidenceConflict})$.

# VII. Universal-cover lifting

### Definition 14: Lift

Given a quotient path $\bar{\gamma}$, a lift is a continuous path $\gamma : [0,1] \to \Omega$ such that the quotient projection satisfies $q \circ \gamma = \bar{\gamma}$.

Its deck displacement is the unique $\delta \in \mathbb{Z}^r$ satisfying $\gamma(1) = \gamma(0) + L \delta$ when both endpoints are regular representatives of the same quotient relation.

### Lift consistency

For each concatenation $a_1 a_2 \cdots a_m$ forming a quotient-closed boundary walk, the deck displacements satisfy: $\sum_{i=1}^m \delta_i = \Delta_{\text{walk}}$, where $\Delta_{\text{walk}}$ is the homology/deck class of the walk.

For a contractible regular boundary, $\Delta_{\text{walk}} = 0$. For an essential boundary, it may be nonzero.

### Potential formulation

Let the lifted endpoint-copy index of regular vertex $v$ be $\psi(v) \in \mathbb{Z}^r$.

For a half-edge $h : v \to w$, $\psi(w) - \psi(v) = d(h)$.

A weighted union-find or graph-potential solver determines whether these equations are consistent.

An inconsistent cycle gives: $\text{Inconsistent}(\text{DeckPotentialContradiction})$.

# VIII. Finite cover construction

This is central because the arrangement must be finite without missing relevant translated interactions.

### Definition 15: Conservative enclosure

Each lifted arc piece $\gamma_i$ has a certified compact enclosure $B_i \subset \mathbb{R}^2$ such that $\gamma_i([0,1]) \subseteq B_i$.

The enclosure may be a box, interval enclosure, convex region, or another set supporting finite lattice-candidate enumeration.

### Definition 16: Candidate translation set

For pieces $i, j$, define $K_{ij} = \{k \in \mathbb{Z}^r : B_i \cap (B_j + L k) \neq \emptyset\}$. Equivalently, $K_{ij} = \{k \in \mathbb{Z}^r : L k \in B_i - B_j\}$.

### Lemma 1: Candidate translation finiteness

For every $i, j$, $K_{ij}$ is finite.

*Proof.* $B_i$ and $B_j$ are compact. Therefore their Minkowski difference $B_i - B_j = \{x - y : x \in B_i, y \in B_j\}$ is compact and hence bounded. The image $L \mathbb{Z}^r$ is a discrete lattice because $L$ has full rank on its $r$-dimensional image. A bounded set intersects a discrete lattice in finitely many points. Therefore $(B_i - B_j) \cap L \mathbb{Z}^r$ is finite. Each such lattice point corresponds to finitely many—and, for a lattice basis, exactly one—integer vector $k$. Hence $K_{ij}$ is finite. $\blacksquare$

### Definition 17: Required copy set

Let $K$ be the finite closure of:
- all endpoint deck potentials;
- all intermediate deck copies traversed by multi-period arcs;
- all $K_{ij}$ required for candidate intersections;
- every deck neighbor needed to pair a quotient crossing;
- a finite boundary-neighborhood closure required to identify incident cells.

The fifth item must be defined combinatorially, not as an informal “one-cell margin”: For every translated half-edge retained in the arrangement, retain both incident local sides and the deck copies required to identify their quotient-equivalent cells.

Because each retained half-edge has two local sides and finitely many deck relations, this closure remains finite.

### Lemma 2: Required copy-set finiteness

The set $K$ is finite.

*Proof.* Items 1 and 2 are finite because there are finitely many arcs and bounded deck displacement. Item 3 is a finite union of the finite sets $K_{ij}$ from Lemma 1. Item 4 adds finitely many deck neighbors for finitely many retained quotient crossings. Item 5 adds at most finitely many incident local-cell representatives per retained edge and deck identification. The finite union of finite sets is finite. $\blacksquare$

### Sufficiency proposition

Assuming the conservative enclosures and certified intersection solver are correct, every intersection between any retained quotient arc pair has a representative among the translated arc pairs indexed by $K$.

*Proof.* Suppose quotient arcs $i$ and $j$ intersect. Choose a lift of the intersection on $\gamma_i$. Some deck translate $\gamma_j + L k$ passes through that same lifted point. Hence their enclosures overlap: $B_i \cap (B_j + L k) \neq \emptyset$. Thus $k \in K_{ij} \subseteq K$. Therefore that lifted candidate pair is examined. $\blacksquare$

This proves intersection sufficiency. Cell-reconstruction sufficiency follows from retaining both local sides and all deck pairing relations of every retained half-edge.

# IX. Arrangement normalization

### Definition 18: Certified intersection normalization

For every candidate translated pair:
- determine certified disjointness;
- or enumerate all isolated intersections;
- classify each intersection: transverse, endpoint, certified tangential if later admitted, overlap, unresolved.

For the initial envelope:
- transverse and endpoint intersections are supported;
- overlaps are unsupported unless normalized by a separately proven overlap subsystem;
- uncertified tangencies are unresolved.

Every arc is split at its finite certified parameter set.

### Lemma 3: Arrangement finiteness

The normalized lifted arrangement is finite.

*Proof.* There are finitely many translated arc copies by Lemma 2. Each candidate pair has at most $x_{\max}$ intersections. Therefore the total number of intersection points is finite. Splitting finitely many compact arcs at finitely many parameters creates finitely many arc segments. The graph consisting of these segments and their endpoints is therefore finite. $\blacksquare$

### Quotient construction

Identify regular arrangement vertices only when supported by a certified deck relation: $v \sim_\Lambda w \iff x_w = x_v + L k$ for a certified $k \in \mathbb{Z}^r$.

Singular collapse is represented separately: $v \sim_\Sigma \sigma$ only when $v$ lies on a certified collapsed stratum and its attachment is permitted by that stratum’s schema.

The implementation must not combine $\sim_\Lambda$ and $\sim_\Sigma$ into one undifferentiated proximity weld.

### Lemma 4: Quotient-complex finiteness

The resulting quotient boundary complex $G$ is finite.

*Proof.* The lifted arrangement graph is finite by Lemma 3. A quotient of a finite set by any equivalence relation has finitely many equivalence classes. Singular attachments add only finitely many declared stratum records. Hence $G$ is finite. $\blacksquare$

# X. Material-region solution

The material solve should be formulated as a constraint problem.

### Definition 19: Cell variables

For every quotient arrangement cell $c \in \mathcal{C}$, introduce $\mu_c \in \{0,1\}$.

### Definition 20: Constraint generation

- **Physical oriented boundary**: For physical half-edge $h$ with incident cells $L(h)$ and $R(h)$: $\mu_{L(h)} = 1, \mu_{R(h)} = 0$.
- **Artificial split or chart cut**: For an artificial cut whose two sides represent the same quotient-local material state: $\mu_{L(h)} = \mu_{R(h)}$.
- **Deck-identified cells**: For $c_i \sim_\Lambda c_j$: $\mu_{c_i} = \mu_{c_j}$.
- **Native ambient boundary**: A native ambient boundary does not itself toggle material. Its interpretation is determined by incident physical boundary constraints and the ambient domain.
- **Singular attachment**: A singular attachment contributes a local-link constraint. For example, a cone apex disk requires the selected incident sectors around the collapsed orbit to form exactly one connected cyclic link.

These constraints are schema-specific but finite and declared.

### Definition 21: Material solution set

Let $M(A,G) = \{\mu : \mathcal{C} \to \{0,1\} : \mu \text{ satisfies all generated constraints}\}$. Then define:

$$\text{Solve}(A,G) = \begin{cases} \text{Inconsistent}, & |M| = 0, \\ \text{Unique}(\mu), & |M| = 1, \\ \text{Ambiguous}(M), & |M| > 1. \end{cases}$$

A production implementation need not enumerate all assignments. It may solve a finite SAT, union-find-with-constants, or graph-labeling system. The semantics are nevertheless defined by $M$.

### Lemma 5: Material-solve totality

For every finite arrangement $G$, exactly one of $\text{Inconsistent}$, $\text{Unique}$, or $\text{Ambiguous}$ applies.

*Proof.* The set of all Boolean assignments to $\mathcal{C}$ has cardinality $2^{|\mathcal{C}|}$, which is finite. The constraint-satisfying subset $M$ therefore has a well-defined finite cardinality. Exactly one of the mutually exclusive conditions $|M| = 0, |M| = 1, |M| > 1$ holds. $\blacksquare$

This is the key semantic exhaustiveness result for material selection.

# XI. Validity of the selected region

A unique labeling does not automatically imply that the material set is a valid face region.

### Definition 22: Selected subcomplex

Given unique $\mu$, let $G_\mu$ be the subcomplex consisting of selected two-cells, their incident edges, their incident vertices, and declared singular attachments.

### Definition 23: Regular manifold-link conditions

At a regular interior point, the link must be a circle.
At a regular physical-boundary point, the link must be an interval.
At a regular vertex, the cyclic sequence of incident selected sectors must be:
- one cycle for an interior vertex;
- one contiguous interval for a boundary vertex.

Any other pattern is nonmanifold.

### Definition 24: Singular-link condition

For each collapsed stratum $\sigma$, the ambient schema declares an allowed link family $\mathcal{L}_\sigma$. The selected material neighborhood induces a link $\text{Link}_{G_\mu}(\sigma)$. Validity requires $\text{Link}_{G_\mu}(\sigma) \in \mathcal{L}_\sigma$.

Examples:
- cone apex disk: one circular selected link;
- apex sector: one interval selected link with two physical boundary ends;
- sphere pole cap: one circular selected link;
- invalid apex welding: disconnected link or multiple cycles.

### Definition 25: Region validity

$$A, G, \mu \vdash \text{ValidRegion}$$

iff:
- incidence is internally consistent;
- the selected regular part is an orientable two-manifold with boundary;
- every singular link is permitted;
- the selected quotient region is compact;
- every physical boundary is represented;
- artificial boundaries are paired;
- no unresolved or unsupported relation remains.

### Lemma 6: Validity is decidable over $\mathcal{E}_\beta$

*Proof.* $G_\mu$ is finite. Each condition above reduces to a finite computation. Therefore the validity judgment terminates with true or false. $\blacksquare$

An invalid unique labeling yields a typed inconsistency such as $\text{Inconsistent}(\text{NonmanifoldSelectedRegion})$, not a new atlas cell.

# XII. The canonical region complex

### Definition 26: Raw region complex

A valid raw region complex is $R = (A, G, \mu, \Pi)$, where $\Pi$ includes provenance and epistemic evidence.

### Definition 27: Gauge transformations

The always-admitted gauge transformations are generated by:
- graph relabeling;
- half-edge starting-point rotation;
- source-edge subdivision and inverse subdivision;
- permutation of disconnected components;
- global deck translation;
- seam relocation preserving the same certified parameterization;
- introduction or removal of paired artificial cuts;
- replacement of source segmentation by an equivalent segmentation with the same embedded directed image and provenance partition.

Conditional transformations include chart reflection or generator-basis changes only when accompanied by a certified automorphism $\phi : A \to A$ of the actual ambient parameterized surface.

### Definition 28: Semantic equivalence

Two valid region complexes satisfy $R_1 \equiv R_2$ iff there exists an allowed gauge transformation inducing an isomorphism that preserves ambient schema, selected-cell structure, oriented physical boundary, deck cocycle, native and singular attachments, physical 3D realization under $S$, and provenance equivalence classes, modulo allowed source subdivision.

### Canonicalization preparation

Choose one regular vertex in each connected component according to a graph-intrinsic rule after provisional graph canonicalization, and translate its deck potential to zero. Artificial subdivision is removed by suppressing every degree-two regular vertex that is not a source-semantic vertex, intersection, singular/native attachment, provenance partition change, or primitive transition.

### Definition 29: Canonical encoding

Encode the normalized finite labeled complex as a finite word $\text{Enc}(R) \in \Sigma^*$. Let $\Gamma_R$ be the finite effective gauge group remaining after deck anchoring and subdivision suppression. Define:

$$\text{Can}(R) = \arg\min_{\gamma \in \Gamma_R} \text{Enc}(\gamma R)$$

under lexicographic order.

### Lemma 7: Finite effective gauge orbit

For every $R \in \mathcal{E}_\beta$, the set $\{\text{Enc}(\gamma R) : \gamma \in \Gamma_R\}$ is finite.

*Proof.* After subdivision suppression, $R$ has finitely many vertices, half-edges, and cells. Graph relabelings form a finite permutation group. Component permutations are finite. Boundary rotations are finite. Deck translations have been anchored away. Relative deck labels are bounded by finite paths of at most $|H|$ edges, each with bounded displacement $w_{\max}$. Therefore the effective orbit of finite encodings is finite. $\blacksquare$

### Lemma 8: Canonicalization existence and termination

$\text{Can}(R)$ exists and is computable.

*Proof.* By Lemma 7, the candidate encoding set is finite and nonempty. A finite nonempty set of words under a total lexicographic order has a unique minimum. $\blacksquare$

### Lemma 9: Canonicalization soundness

If $R_1 \equiv R_2$, then $\text{Can}(R_1) = \text{Can}(R_2)$.

### Lemma 10: Canonicalization completeness relative to equivalence definition

If $\text{Can}(R_1) = \text{Can}(R_2)$, then $R_1 \equiv R_2$.

# XIII. Generative atlas normal forms

### Definition 30: Ambient signature
$$\text{Amb}(R) = (r, \Sigma_{\text{type}}, N_{\text{type}}, A_{\text{map}})$$

### Definition 31: Component signature
For each connected material component $K$, derive: $\text{Comp}(K) = (\chi, g, b, H, S, N)$.

### Definition 32: Region normal form
A structured normal-form label is:

$$\text{RNF}(R) = (\text{Amb}(R), \text{Multiset}\{\text{Comp}(K)\}, \text{AdjacencyPattern}(R))$$

Friendly names (Disk, ApexDisk, EssentialAnnulus, etc.) are partial renderings of this structure.

# XIV. Main closure theorem

### Theorem 1: Symbolic closure of the atlas over $\mathcal{E}_\beta$

Let $F$ be a normalized STEP face candidate subject to the bounds $\beta$. Then the atlas procedure terminates and produces exactly one outcome:

$$\text{Valid}(\text{Can}(R)) \mid \text{Inconsistent}(I) \mid \text{Ambiguous}(A) \mid \text{Unsupported}(U) \mid \text{Unresolved}(N)$$

Furthermore:
1. If the outcome is Valid, the canonical region complex is invariant under all declared gauge transformations.
2. Every valid semantic configuration in $\mathcal{E}_\beta$ has a representation in the atlas language.
3. No input inside the procedural envelope reaches an untyped “unknown configuration” state.
4. The human-readable normal-form catalog may be incomplete without compromising semantic closure.

# XV. Corollaries

- **Corollary 1**: No individual geometric bug class inside the envelope is semantically primitive.
- **Corollary 2**: Novel labels do not imply loss of closure.
- **Corollary 3**: Corpus growth cannot invalidate closure by itself.
- **Corollary 4**: The reverse atlas is nonfoundational.

# XVI. What is genuinely proved and what remains assumed

**Proved from finite combinatorics**: finite candidate translation enumeration, finite arrangement size, totality of material constraint classification, decidability of finite validity checks, existence/invariance of canonicalization, representational closure, exhaustive typed outcomes.

**Assumed or certificate-dependent**: numerical embedding, correct curve enclosures, intersection solver completeness, pcurve/3D compatibility, STEP orientation mapping, singularity local-link certificates.

# XVII. Realization theorem boundary

$$\text{RealizationStatus} ::= \text{Realizable}(\text{CutOpenDomainPlan}) \mid \text{RecognizedButNotImplemented} \mid \text{GeometricallyUnresolved}$$

### Theorem 2: Topological cut-open existence

For every compact connected orientable two-manifold with boundary represented by a finite cellular complex, there exists a finite cut graph such that cutting along it yields a planar polygonal schema.

# XVIII. Architecture mandated by the formal system

```
STEP source evidence
    ↓
orientation and traversal normalization
    ↓
curve-on-surface evidence resolution
    ↓
continuous lifted arcs with deck labels
    ↓
finite translated-copy enumeration
    ↓
certified quotient arrangement
    ↓
material-cell constraint solve
    ↓
region validity and singular-link checks
    ↓
canonical region complex
    ↓
derived normal-form label
    ↓
cut-open plan
    ↓
shared-edge-constrained meshing
    ↓
shell validation
```

### Required modules
```
atlas/
    evidence.rs
    ambient.rs
    traversal.rs
    projection.rs
    lift.rs
    cover.rs
    arrangement.rs
    material.rs
    validity.rs
    canonical.rs
    normal_forms.rs
    outcomes.rs
    certificates.rs
```

# XIX. Procedure for extending the envelope

Monotonic extension requires adding formal syntax, denotation, orientation/multiplicity semantics, arrangement rules, material constraints, validity conditions, canonicalization behavior, termination/closure lemmas, metamorphic witnesses, and reverse signatures for known incorrect treatments before moving a configuration from `Unsupported` to admitted atlas states.
