# Formal Algorithmic Closure for Constructive B-rep Generation

> **Scope.** This is the formal system for *generating* B-reps: Booleans,
> extrude, revolve, sweep, loft, offset, shell, and fillet. It is the sibling of
> [`FORMAL_SYSTEM_STEP_INGESTION.md`](FORMAL_SYSTEM_STEP_INGESTION.md), which is
> the formal system for *reading* them — face-level symbolic closure over
> normalized STEP input. The two are separate systems with separate envelopes.
>
> Where they touch: ingestion produces the operands this system consumes, and
> §19 here (input validation and repair) is the seam. The contract registry in
> [`MATHEMATICAL_FOUNDATION.md`](MATHEMATICAL_FOUNDATION.md) governs the
> ingestion side; the cell inventory below governs this side. Neither is
> subordinate to the other.
>
> Nothing in this document is implemented in `look` today. It is a
> specification and an audit target — see
> [`TRUCK_GENERATION_AUDIT.md`](TRUCK_GENERATION_AUDIT.md) for where the
> current kernel stands against it.

*Revision 4. Single unified proposal.*

> **Revision 4 changes.** Three additions, in prevalence order rather than
> difficulty order:
> 1. **§9.2 rewritten at full density.** The classification now splits on the
>    *dimension* of the tangency locus before contact order. The
>    one-dimensional case — fillet-to-support, cylinder-on-plane, coaxial pairs,
>    i.e. the dominant real configuration — had no cell at all through r3 and now
>    has SS-TAN-CRV-001, solved by tracing a ridge curve with a transverse
>    Jacobian rather than by singular solving. Isolated contact is unified under a
>    **polar blow-up** (the §10 deflation move, reused) gated on the discriminant
>    of the leading form, subsuming r3's SS-TAN-ELL/HYP/DEG. Flip parity is
>    certified by sector sign, not by computing a multiplicity. TAN-SNAP-001 adds
>    backward promotion of near-degeneracy to exact degeneracy.
> 2. **§17 restructured as blends, and chamfer added.** Chamfer was absent
>    through r3 — a coverage hole, not a deferral. It is *easier* than fillet
>    ($G^0$ to its supports, hence no tangency-preserving `rep` and no $G^1$
>    corner obstruction) and should be scheduled ahead of the fillet corner cases.
>    Ends, spillover, mutual trim and corners are now a shared substrate (BLD-\*)
>    rather than fillet-private.
> 3. **§17.4 corners, promoted from one sentence.** The common corner — three
>    faces, convex vertex, uniform radius — is a **spherical triangle exactly in
>    $\mathcal{G}$ with automatic $G^1$**, reducing to the three-offset vertex gate
>    §16.2 already had. The general case keeps its $n$-sided patch, but the
>    obstruction is now named: $G^1$ filling around a closed loop is singular for
>    even $n$, which is exactly the $k=4$ corner.
>
> A new REP-G1-001 (tangency-preserving representation) is introduced in §17.1,
> because positional $\tau_{\text{rep}}$ never certified $G^1$ and every blend
> claiming $G^1$ was relying on it.

*Revision 3 changes.*

> Five corrections, all of which remove or weaken claims rather than add cells:
> 1. §6.2 — the isotopy lemma acquires a **one-sheet condition (iv)**. (i)–(iii)
>    give a proper local homeomorphism, hence a covering of *some* degree, not a
>    homeomorphism. A counterexample and two discharge routes are given.
>    Consequences propagate to §6.3, REP-CRV/SRF-001 and OB-7.
> 2. §15.4 LFT-RULED-001 — the parallel-plane "positive normal component"
>    condition is withdrawn as **vacuous** (every ruling joins the two planes),
>    and replaced by the correct criterion: injectivity iff every *intermediate
>    section* is simple, gated by a certified chord margin. SHL-CAP-001 no
>    longer cites it.
> 3. §18 — the composition bound is restated as the **nested recurrence**; the
>    split form becomes a corollary conditional on subadditivity, which is now
>    part of an explicit `Modulus` contract (M1)–(M4).
> 4. §6.1 — reach quantities are consistently **certified lower bounds**
>    $\underline\rho$, never identified with reach; refusal names follow.
> 5. §16.3 OFF-TOPO-001 — critical values are taken in the **generalized
>    (Grove–Shiohama/Clarke)** sense with the weak feature size as the governing
>    quantity, and the claim is reduced to the one direction the isotopy lemma
>    actually supports.

**Gate labelling convention.** Every scalar condition is tagged:
- **[DERIVED]** — follows from a stated theorem; may appear in a constructive precondition.
- **[PROVISIONAL SUFFICIENT GATE]** — believed sufficient, discriminant analysis not yet done. May gate an operation but may **not** support a `ProvenConstruction` claim; instances passing it emit `CertifiedEquivalentConstruction` at most, pending derivation.
- **[POLICY]** — a choice, not a theorem (corner treatment, correspondence heuristics). Must be recorded in provenance so regeneration is stable (§20).

---

## 0. Scope, semantics, and the two closure notions

### 0.1 Master semantics: backward error

> **Semantic contract.** For an operation with budgets $(\tau_{\text{in}},\tau_{\text{rep}},\tau_{\text{col}})$, the kernel returns $B_{\text{out}}$ with certificates that there exists $\tilde B_{\text{in}}$, $d_H(\tilde B_{\text{in}},B_{\text{in}})\le\tau_{\text{in}}$, and an ideal result $\tilde B=\mathrm{op}_{\text{exact}}(\tilde B_{\text{in}})$, such that
> $$B_{\text{out}}=\mathrm{rep}(\tilde B),\qquad d_H(B_{\text{out}},\tilde B)\le\tau_{\text{rep}},\qquad B_{\text{out}}\simeq_{\text{isotopy}}\tilde B .$$

Forward accuracy is derived, requires a conditioning bound (§18), and is never claimed unconditionally.

Three budgets, never one $\tau$: $\tau_{\text{in}}$ (perturbation admitted by validation/repair), $\tau_{\text{rep}}$ (representation error), $\tau_{\text{col}}$ (collapse quotient). All derived from operand tolerances and operation conditioning.

### 0.2 Epistemic vs. constructive closure

- **Epistemic** over $E$: every input yields a sound typed outcome; no unclassified failure, no silent wrong answer.
- **Constructive** over $E_c\subset E$: every input meeting the *quantitative* preconditions yields a certified realisation or a certified collapse.

`Unsupported` cells lie strictly outside $E_c$ and are diagnostic only.

---

## 1. Formal B-rep data model

$$B=(V,E,U,W,F,S,\Omega;\ I,O,\Gamma,\Sigma,\tau_{\mathrm{loc}},\Lambda,P)$$

**Vertices** $V$: point $p_v$, certified ball $B(p_v,r_v)$, $r_v\le\tau_{\mathrm{loc}}(v)$.

**Edges** $E$: 3D carrier $c_e\in\mathcal{G}$, domain $I_e$, endpoint vertices, tolerance $\tau_e$. An edge may be **degenerate** — $c_e$ constant, the image a single point (cone apex, sphere pole). First-class; without it the sphere is not representable.

**Coedges** $U$ — first-class, not a field of $E$:
$$\mathrm{edge}:U\to E,\quad \mathrm{wire}:U\to W,\quad \mathrm{face}:U\to F,\quad \mathrm{sense}:U\to\{\pm1\},$$
with the **pcurve attached to $u$, not $e$**. This is what represents a seam edge appearing twice on one periodic face, opposite parametric representations of one 3D edge, open shells (one coedge), and declared non-manifold incidence ($2k$ coedges).

**Wires** $W$: cyclic coedge sequences, consistent sense, endpoints matched inside tolerance balls.

**Faces** $F$: carrier $S_f$, trimmed domain $D_f$ (or a fundamental domain, §8), orientation $o_f$, one outer wire and $k\ge0$ hole wires, tolerance $\tau_f$.

**Shells** $S$: oriented face collections with coedge pairing. **Solids** $\Omega$: one outer shell, $k\ge0$ inner shells, certified nesting; $\emptyset$ is a terminal object.

$\Sigma$ is **declared to be the boundary CW 2-complex**. $\Lambda$ is the periodicity contract (§8). $P$ is provenance (§20).

### 1.1 Invariants

1. **Coedge pairing.** Every non-degenerate edge has exactly two coedges of opposite sense (manifold), a declared even number (non-manifold), or one (declared open shell).
2. **Links.** Vertex link is a single cycle $S^1$; interior-edge link is $S^0$. ($S^2$ links belong to a volumetric 3-complex and are the wrong test here.)
3. **Euler–Poincaré.** $V-E+F-R=2(S-G)$. Necessary only — a pinch point satisfies it, so it never substitutes for the link test.
4. **Same-parameter / same-range.** $\lVert\Gamma_f(\mathrm{pc}_u(t))-c_e(\varphi_u(t))\rVert\le\tau_e$ for **all** $t$, certified by interval evaluation over the whole span. The most common defect in imported data and the most common corruption introduced by operations.
5. **Domain–boundary correspondence.** $\partial D_f$ maps to the edge images of its bounding wires.
6. **Representation.** Carriers lie in $\mathcal{G}$; geometry is certified within $\tau_{\text{rep}}$ of the ideal object (§6). Without $\mathrm{rep}$, "carriers stay in $\mathcal{G}$" contradicts the intersection engine, whose exact output is not in $\mathcal{G}$.
7. **Tolerance monotonicity.** $\tau$ on an entity is $\ge$ that of its boundary and $\le\theta\cdot\underline{\mathrm{lfs}}_\sigma$ for its stratum (§6.1), $\theta<\tfrac12$.
8. **Nesting.** Inner shells certified contained and mutually disjoint.
9. **Wedge non-degeneracy.** Every edge has a dihedral angle certified bounded away from $0$ and $2\pi$ (no knife edges or cracks), or the edge is declared singular. Required for stratified reach (§6.1) to be positive.

---

## 2. Carriers, representation, envelope

$\mathcal{G}$ = analytic primitives $\cup$ NURBS degree $\le3$, $\le32$ spans, rational control data.

**Input semantics.** Exact with respect to *the rationals as given*; floats are the exact dyadics they are.

**Representation operator** $\mathrm{rep}$ with certificates $(\varepsilon,\theta,\rho)$ per §6. $\mathcal{G}$ is not closed under intersection, offset, or envelope formation; $\mathrm{rep}$ is the explicit projection back, and its error enters the backward-error contract.

**Scale invariance.** All predicates scale-invariant or relative to a declared model scale. Absolute constants in predicates are a defect.

Membership in $E$ is not statically decidable; what is guaranteed is that the classifier is total.

---

## 3. Global proof obligations

**OB-1 (Dispatch completeness).** At every dispatch point: $\bigvee_i\mathrm{Pre}(c_i)\vee\mathrm{Pre}(c_\perp)\equiv\top$, plus overlap agreement.

**OB-2 (Budgeted termination).** Explicit ledger $\beta$; exhaustion is a typed terminal state.

**OB-3 (Quantitative $E_c$).** Constructive cells state margins and the depth bound they imply, e.g. $\sin\theta\ge\delta$, separation $\ge\sigma$ $\Rightarrow d\le\log_2(CDL/\delta\sigma)$. Symbolic-only preconditions are epistemic-only.

**OB-4 (Fidelity).** No cell emits topology certified for an object whose geometry it does not emit.

**OB-5 (Identity totality).** Every reference resolves to exactly one of `Preserved | Split | Merged | Vanished | Ambiguous`.

**OB-6 (Conditioning).** Every cell publishes a certified **modulus of continuity** $\omega$ relating input perturbation to output displacement, satisfying the `Modulus` contract (M1)–(M3) of §18 and declaring explicitly whether it satisfies (M4). A cell without $\omega$ may not participate in a composition claim; a cell whose $\omega$ is not certified subadditive may participate only via the nested recurrence, not the split bound.

---

## 4. Evidence algebra

$$\mathcal{E}=(\pi,\mu,\beta,\mathfrak{m},\omega),\qquad \pi:\mathcal{P}\to\{\bot,\mathsf{T},\mathsf{F},\top\},\quad\bot\le_k\{\mathsf{T},\mathsf{F}\}\le_k\top$$

$\mathcal{P}$ is a fixed proposition set (`source_valid`, `intersection_complete`, `geometry_exists`, `metric_resolvable`, `manifold`, `orientation_consistent`, `nesting_valid`, `fidelity_certified`, …). $\mu$ is **method** (exact / interval / float / none), an axis distinct from truth value. $\beta$ is budget residue, $\mathfrak{m}$ the topological stability margin, $\omega$ the metric modulus. Accumulation is monotone in $\le_k$; $\top$ anywhere is `ContradictoryEvidence`.

**Terminal outcomes** (total, mutually exclusive): `ProvenConstruction`, `CertifiedEquivalentConstruction`, `CertifiedGeometricCollapse`, `EmptyResult`, `UnsupportedEnvelopeCase`, `NumericallyUnresolved`, `CompositionMarginExhausted`, `ForwardToleranceExceeded`, `InputOutsideBackwardBudget`, `ContradictoryEvidence`. (`ForwardToleranceExceeded` was referenced by §18 but omitted here through r2 — a totality gap under OB-1, since a chain can fail the metric condition while passing every topological one.)

---

## 5. Tolerance, clustering, collapse

$p\sim_\tau q$ is not transitive and is not used. Certified clustering instead: connect $i\sim j$ when $B(X_i,r_i+\epsilon)\cap B(X_j,r_j+\epsilon)\ne\emptyset$; components are clusters; compute certified enclosing ball $B(c_C,R_C)$.

**Admissibility.**
$$R_C\le\min\!\big(\tau_{\text{col}},\ \theta\cdot\underline{\mathrm{lfs}}_\sigma(C)\big),\qquad\theta<\tfrac12,$$
with $\underline{\mathrm{lfs}}_\sigma$ the certified lower bound on the **stratified** local feature size of §6.1 — not a global reach. This is what makes membership invariance provable and prevents a chain of near-coincident entities from merging distinct features. A violated bound triggers a refinement loop (re-solve at higher precision, re-cluster) before any refusal.

**Dimensional gates.** $\operatorname{diam}(X)\le C_1\tau$ and $\mathcal{H}^k(X)\le C_k\tau^k$; primary criterion is Hausdorff boundary displacement, measure bounds secondary and dimensionally homogeneous.

**Collapse calculus (seven gates).** metric gate; admissible cluster; local replacement patch; tubular enclosure in the **stratum-adapted** tube (§6.1); membership invariance outside a slightly larger neighbourhood; local link test; global homology, shell adjacency and orientation.

**Tolerance propagation** is monotone with a declared ceiling. Exceeding it is `ToleranceBudgetExhausted`, never silent absorption — unbounded tolerance inflation is the classic production-kernel failure.

---

## 6. Representation and topological fidelity

### 6.1 Stratified reach and local feature size

A mechanical B-rep boundary is **not** $C^2$ across an edge, and the smooth-boundary reach of $\partial\Omega$ collapses to zero there. A single global $\rho(\partial\Omega)$ is therefore unusable as local feature size. Define reach per stratum of the boundary complex.

For $x$ in stratum $\sigma$ (face interior, edge interior, or vertex), every term is a **certified lower bound**, and the composite is written $\underline{\mathrm{lfs}}_\sigma$ to keep that visible:
$$\underline{\mathrm{lfs}}_\sigma(x)=\min\Big(\underbrace{\underline\rho(\sigma)}_{\text{intrinsic}},\ \underbrace{\underline{\operatorname{dist}}(x,\ \textstyle\bigcup\{\sigma'\ \text{not incident to}\ \sigma\})}_{\text{separation}},\ \underbrace{\underline\varrho_{\text{wedge}}(x)}_{\text{incident structure}}\Big)\ \le\ \mathrm{lfs}_\sigma(x)$$

| stratum | intrinsic term (certified lower bound on reach) | incident-structure term |
|---|---|---|
| face interior | $\min(1/\overline\kappa_{\max},\tfrac12\underline\sigma_{\text{self}})$ | distance to the face's own boundary wires |
| edge interior | curve-reach lower bound for $c_e$ | $\underline\varrho_{\text{wedge}}$: bound from the dihedral angle $\psi$, degenerate as $\psi\to0$ or $2\pi$ (invariant 9), and from incident face curvatures |
| vertex | $0$-dimensional | star separation: min incident edge length, min angular separation of incident edges, min dihedral over the star |

**Lower bounds, not equalities.** Federer's identity $\mathrm{reach}=\min(1/\kappa_{\max},\tfrac12\delta_{\text{bottleneck}})$ holds for a compact $C^2$ submanifold **without boundary**, with $\delta_{\text{bottleneck}}$ the double-normal distance. A trimmed patch has boundary, $\overline\kappa_{\max}$ is a computed upper bound on curvature, and $\underline\sigma_{\text{self}}$ is a computed lower bound on the bottleneck — so $\min(1/\overline\kappa_{\max},\tfrac12\underline\sigma_{\text{self}})$ is a lower bound on reach and is nowhere in this document identified with it. Safety is one-directional and that direction is the useful one: every gate has the form $q<c\cdot\mathrm{lfs}$, so substituting a lower bound is conservative and never admits an instance the true value would reject. What it does change is the meaning of refusals: `ReachLowerBoundTooSmall` asserts that the bound could not be certified large enough, **not** that the feature size is small. Cells must not report the converse, and margin sweeps (§21) must not treat the refusal as evidence about the geometry.

**Notational convention for the rest of the document.** An unbarred $\mathrm{reach}$ or $\mathrm{lfs}$ denotes the ideal quantity and appears only inside statements of theorems. Wherever a *gate* is written against one of them, the quantity actually evaluated is the certified lower bound $\underline\rho$ / $\underline{\mathrm{lfs}}$, and the corresponding refusal is a `...LowerBound...` refusal in the sense above.

**Stratified tubular neighbourhoods.** Collapse and isotopy use a **compatible system of tubular neighbourhoods** in the Thom–Mather sense: projections $\pi_\sigma$ and distance functions $\rho_\sigma$ per stratum, satisfying the control relations $\pi_\sigma\pi_{\sigma'}=\pi_\sigma$, $\rho_\sigma\pi_{\sigma'}=\rho_\sigma$ where defined. A vertex gets a ball, an edge a normal disc bundle over $c_e$ of radius $<\underline{\mathrm{lfs}}$, a face a normal tube — and the three agree on overlaps. Without the control relations the stratum tubes overlap inconsistently and the isotopy cannot be glued.

**OB-7 (Stratified isotopy).** The ambient isotopy of §6.2 is constructed stratum-by-stratum on this compatible system and glued via the control relations; the resulting isotopy is piecewise-smooth, not smooth, and that is the correct category for B-reps. The one-sheet condition §6.2(iv) is an obligation **per stratum**: it must be discharged separately on each face, edge and vertex tube. The control relations $\pi_\sigma\pi_{\sigma'}=\pi_\sigma$ are what guarantee that a sheet counted once in the edge tube is not counted a second time in the adjacent face tube — without them the per-stratum counts do not compose into a global degree-one statement.

### 6.2 Isotopy lemma (smooth stratum)

> Let $X$ (curve or surface, possibly with boundary) be compact with certified reach lower bound $\underline\rho>0$, and let $X'$ be compact and satisfy
> (i) two-sided $d_H(X,X')\le\varepsilon<\underline\rho/2$;
> (ii) $\angle(T_xX',T_{\pi(x)}X)\le\theta<\tfrac\pi2-\arcsin(\varepsilon/\underline\rho)$;
> (iii) boundary correspondence under $\pi$ (or both closed);
> (iv) **one-sheet condition:** every normal fibre $\pi^{-1}(x)\cap\mathrm{tube}(X,\varepsilon)$, $x\in X$, meets $X'$ in exactly one point.
> Then $\pi|_{X'}$ is a homeomorphism onto $X$ and the straight-line homotopy along normal fibres is an ambient isotopy supported in $\mathrm{tube}(X,\varepsilon)$.

**Why (iv) does not follow from (i)–(iii).** Those three give exactly this much: $\pi$ is well defined and smooth on the tube; (ii) makes $X'$ transverse to the normal fibres, so $d\pi|_{TX'}$ is an isomorphism and $\pi|_{X'}$ is a local diffeomorphism; compactness makes it proper. A proper local homeomorphism onto a connected, locally connected base is a **covering map of some constant finite degree $k$** — and nothing above forces $k=1$. Concretely, let $X$ be the circle of radius $R$ and
$$X'=\big\{(R+\varepsilon\cos(t/2))\,e(t)\ :\ t\in[0,4\pi]\big\},\qquad e(t)=(\cos t,\sin t).$$
$X'$ closes up, lies within $\varepsilon$ of $X$ in both directions, has tangent deviation $O(\varepsilon/R)$, and satisfies (iii) vacuously — yet $\pi|_{X'}$ is a $2$-to-$1$ covering, and no fibrewise displacement carries $X'$ to $X$. The surface case is the same construction realised as a double sheet inside the normal tube. Hausdorff closeness plus almost-tangency controls how a sheet behaves locally and says nothing about how many sheets there are.

The division of labour is then: the $\supseteq$ direction of the two-sided Hausdorff bound gives surjectivity, (ii) gives local injectivity, (iv) gives global injectivity. None of the three implies another. Condition (ii) is the one usually omitted from informal statements; (iv) is the one usually left implicit. Neither is free.

**Two admissible discharges of (iv).**

*(iv-a) Degree one per component — cheapest.* Once (i)–(iii) hold, the fibre cardinality of $\pi|_{X'}$ is **constant on each connected component** of $X$. (Condition (iii) is what makes this valid for manifolds with boundary: it forces $\partial X'\to\partial X$ and $\mathrm{int}\,X'\to\mathrm{int}\,X$, so the covering argument applies in the manifold-with-boundary category.) It therefore suffices to certify **one** normal fibre per connected component of $X$ as meeting $X'$ exactly once: isolate the roots of the fibre equation in that single normal disc by Krawczyk, plus an exclusion certificate covering the remainder of the disc. Cost is one certified fibre per component, independent of the size of the approximant. Note the ordering dependency — this reduction is only licensed *after* (i)–(iii) are established, since it is their consequence (constancy of degree) that makes a single fibre decisive.

*(iv-b) Fibrewise uniqueness on a certified partition — what the emitters actually produce.* $X'$ is emitted as $\varphi:D\to\mathbb{R}^3$ over a partition $\{D_j\}$ of the parameter domain of $X$. Certify per cell:
(a) $\varphi(D_j)\subset\pi^{-1}(X_j)$, where $X_j$ is the exact image of $D_j$ — *fibre-block* containment, which is strictly stronger than the radial tube containment usually checked;
(b) $\pi\circ\varphi|_{D_j}$ injective, from a sign-definite Jacobian on $D_j$ together with boundary correspondence on $\partial D_j$;
(c) adjacent cells meet only along shared fibres, and non-adjacent cells have disjoint fibre blocks.
Then $\bigcup_j\varphi(D_j)=X'$ meets every fibre exactly once. This is the better implementation formulation: the certificate is per-cell rather than global, so a failure localises to the offending cell and feeds the refinement loop directly, whereas (iv-a) returns only a global degree.

**Refusals.** `MultiSheetInTube` — degree certified $>1$; the cause is either a partition too coarse for (iv-b), which refinement fixes, or a genuine self-overlap of the emitted geometry, which routes to §10. `SheetCountUnresolved` — the fibre root count was not certified within budget; this is `NumericallyUnresolved`, not a fidelity claim.

### 6.3 Arrangement label preservation

If every emitted arc satisfies §6.2 **including (iv)** with $\varepsilon<\sigma_{\text{cl}}/3$ ($\sigma_{\text{cl}}$ = certified separation between distinct contact clusters), and no arc's tube meets a cluster ball it is not incident to, the DCEL built from approximants is combinatorially isomorphic to the exact arrangement and all labels transfer.

(iv) is load-bearing here rather than decorative: an emitted arc of degree $2$ over an exact arc contributes two combinatorial arcs, two half-edge pairs and a spurious face to the DCEL while satisfying every metric bound in sight, so the isomorphism fails *silently* and the transferred labels are wrong on a cell that looks certified. This is the sharpest reason to make (iv) an explicit obligation rather than an assumption about well-behaved approximants.

**CELL REP-CRV-001.** Pre: margins $(\delta,\sigma)$, certified reach lower bound $\underline\rho$, target $\varepsilon<\min(\underline\rho/2,\sigma_{\text{cl}}/3,\tau_{\text{rep}})$. Algorithm: adaptive interpolation with control-polygon tube bounds; refine while $(\varepsilon,\theta)$ violate §6.2(i)–(ii); discharge (iv) by (iv-b) **on the interpolation's own cell decomposition**, which is already the required partition — so fibre-block containment and per-cell Jacobian sign are the natural certificates and cost no extra subdivision structure. Termination: $\varepsilon$ halves, $\theta=O(h^2)$, degree-one certification is monotone under refinement, budget-capped. Refusals: `ReachLowerBoundTooSmall` (route to §5 collapse), `MultiSheetInTube`, `SheetCountUnresolved`, `FidelityBudgetExhausted`.

**CELL REP-SRF-001.** As above for trimmed patches, with (iv-b) over the 2D partition and fibre blocks taken in the normal disc bundle, plus interval certification of invariant 4 over the whole span. The surface case is where (iv) is least intuitive and most necessary: two sheets of an approximant can sit inside one normal tube with correct Hausdorff distance and correct tangent planes on both sheets.

---

## 7. Numerical substrate and termination

Two tiers. **Certified/exact:** rational arithmetic, subresultants, algebraic separation bounds — terminating by theorem, defines the decidable core of $E_c$. **Resource-bounded:** interval arithmetic, Bernstein/Descartes subdivision, Newton–Krawczyk, under an explicit budget; exhaustion returns `NumericallyUnresolved`.

"Bounded degree + compact domain $\Rightarrow$ finite subdivision" is false: at fixed degree, roots may be arbitrarily close. Quantitative non-degeneracy (OB-3) yields the depth bound. There is no non-termination detector — there is a budget, and that is why epistemic closure survives.

---

## 8. Periodicity contract

Each periodic carrier declares $\Lambda=\langle\lambda_1,\dots,\lambda_k\rangle$, $k\in\{1,2\}$, a finite-rank deck group, and a fundamental domain. Seam coedges as in §1.

**Lift bounds.** Arc length does **not** bound seam crossings: a branch can oscillate across a seam with negligible net displacement. Bound the deck coordinate directly. For deck coordinate $u$ with period $P_u$ along a branch:
$$N_{\text{copies}}\ \le\ 1+\Big\lceil\frac{\mathrm{Range}(u)}{P_u}\Big\rceil,\qquad N_{\text{crossings}}\ \le\ 1+\Big\lceil\frac{\mathrm{Var}(u)}{P_u}\Big\rceil$$
with $\mathrm{Range}$ bounding the width of the unfolding and $\mathrm{Var}$ (total variation) bounding the number of arc segments after splitting at seams.

**Certification.** Isolate the zeros of $du/dt$ along the branch (certified univariate root isolation), decompose into monotone pieces, and sum $|\Delta u|$ per piece: this yields certified $\mathrm{Var}(u)$ and $\mathrm{Range}(u)$ simultaneously. Seam transversality with margin is required so the crossing set is finite and isolated.

$E_c$ bounds $N_{\text{copies}}$ and $N_{\text{crossings}}$ per operation; exceeding either is `Unsupported`.

---

## 9. Intersection atlas

### 9.1 Rank and transversality

$F=S_1-S_2\in\mathbb{R}^3$ on four parameters, so $J$ is $3\times4$ and $\operatorname{rank}J\le3$. Regular intersection is $\operatorname{rank}J=3$, $\dim F^{-1}(0)=1$. Given immersions, $\operatorname{rank}J=3\iff n_1\times n_2\ne0$, so the primitive predicate is scale-invariant:
$$\sin\theta=\frac{\lVert n_1\times n_2\rVert}{\lVert n_1\rVert\lVert n_2\rVert}\ \ge\ \delta .$$

This separates **chart degeneracy** ($\operatorname{rank}[S_u\ S_v]<2$ at a pole; a reparameterisation problem, surfaces may be transverse) from **contact degeneracy** (genuine tangency).

### 9.2 Contact classification

In Monge form over the common tangent plane, $g=f_1-f_2$, $\operatorname{Hess}g=II_1-II_2$. Tangential contact means $\nabla g=0$ on the contact set, so **the intersection curve is the zero level of a function at a critical point** and the classification is a singularity classification.

**Two independent axes, and conflating them is the standard error.** The first is the *dimension of the tangency locus* $\{g=0,\nabla g=0\}$; the second is the *contact order* at a point of it. Revisions through r3 classified only by contact order and tacitly assumed the locus was a point. That assumption is false for the dominant real case (§9.2.2), and a solver built on it does not fail loudly — it subdivides to budget on every box along the locus and reports `NumericallyUnresolved` for a configuration that is exactly decidable.

| dimension of $\{g=0,\nabla g=0\}$ | meaning | cells |
|---|---|---|
| $0$ (isolated points) | surfaces touch at points | §9.2.1, SS-TAN-BLOW-001 |
| $1$ (a curve) | surfaces touch along a curve — **fillet-to-support, cylinder-on-plane, coaxial pairs** | §9.2.2, SS-TAN-CRV-001 |
| $2$ (a region) | coincidence | SS-COIN-001, §13 ON atlas |

#### 9.2.1 Isolated tangential contact

By the splitting lemma, a corank-1 critical point of $g$ is right-equivalent to $\pm y^2+h(x)$ with $\operatorname{ord}h=k+1$, giving the $A_k$ series; corank 2 gives $D_k,E_k$ or a non-isolated locus.

| $\operatorname{Hess}g$ | type | local zero set | membership flip |
|---|---|---|---|
| definite | $A_1^{+}$ | isolated point | no |
| indefinite, nondegenerate | $A_1^{-}$ | two branches crossing | yes |
| rank 1 | $A_{k\ge2}$ | $k$ even: one cuspidal branch; $k$ odd: two tangent branches (tacnode) or an isolated point, by sign | by sector sign, below |
| $\equiv0$ | corank 2 | $D_k/E_k$, or escalate to §9.2.2 / SS-COIN-001 | as below |

**Polar blow-up deflation — the algorithmic core, and the same move as §10.** The naive system is singular at the contact point, so subdivision never separates the branches, exactly as the self-intersection system vanishes on the diagonal. Deflate the same way. With $m=\operatorname{ord}_0 g$ and polar coordinates $x=r\cos\theta$, $y=r\sin\theta$,
$$g(r\cos\theta,r\sin\theta)=r^m\,G(r,\theta),\qquad G(0,\theta)=L_m(\cos\theta,\sin\theta),$$
where $L_m$ is the leading form. On the punctured disc $\{g=0\}\equiv\{G=0\}$, and $G$ is a *regular* system wherever $L_m$ has simple roots.

> **Gate [DERIVED].** If the binary form $L_m$ has only simple roots on $S^1$ with margin — $\operatorname{disc}(L_m)\ge\delta_{\text{disc}}$, equivalently $|L_m|+|\partial_\theta L_m|\ge\delta$ pointwise — then $\partial_\theta G(0,\theta)\ne0$ at each root, and the implicit function theorem gives **exactly one branch per real root of $L_m$**, each certified by Krawczyk on $G$. The number of real half-branches equals the number of real roots of $L_m$ on $S^1$, counted once each.

This supersedes r3's "rank 1 needs the 3-jet" and "$\operatorname{Hess}\equiv0$, escalate": one construction handles every $A_k$, $D_k$ and $E_k$ with a non-degenerate leading form, and only a **Newton-degenerate** $L_m$ (a repeated root, meaning the Newton polygon has a non-generic face) requires iterated blow-up.

**Certified Milnor radius.** The branch count is meaningful only inside a radius where the germ is conical. Certify $r_0$ by excluding solutions of $\{G=0,\ \partial_\theta G=0\}$ on $(0,r_0]\times S^1$ — the same subdivision, on the same deflated system. Outside $r_0$ the local model does not apply and the arcs are handed to the transverse cells.

**Flip parity by sector sign, not by multiplicity.** The $b$ certified branches cut the punctured disc into $2b$ sectors. Evaluate $\operatorname{sign}g$ at one certified point per sector by interval arithmetic; **$\chi_B$ flips across a branch iff the signs on its two sides differ.** This is equivalent to §12's "flips iff contact order is odd" and is far easier to certify — no Milnor number, no local algebra dimension, no multiplicity extraction. Retain $\mu=\dim\mathcal{E}_2/(\partial_xg,\partial_yg)$ as the *invariant* reported in provenance, but do not make the flip decision depend on computing it.

#### 9.2.2 Tangential contact along a curve — the dominant case

Every fillet is tangent to its supports **along a curve**, not at a point; a cylinder rests on a plane along a line; coaxial cylinders of equal radius touch along a line. Here $g$ vanishes to order two along a curve, $g=h^2u$ with $u\ne0$ and $\{h=0\}$ the locus, so
$$\operatorname{Hess}g\big|_{\{h=0\}}=2u\,\nabla h\otimes\nabla h$$
has **rank exactly 1**, with eigenvector $\nabla h$ normal to the locus and kernel tangent to it.

This is why the practical failure mode is what it is: $\{g=0,\nabla g=0\}$ is one-dimensional, so Krawczyk never certifies uniqueness anywhere on it, Bernstein subdivision never separates, and every box meeting the locus subdivides to budget. The instance is not hard — it is *misclassified*.

**Deflation: trace the ridge, do not solve the singular system.** Let $e$ be the unit eigenvector of the nonzero eigenvalue $\lambda$ of $\operatorname{Hess}g$. Then
$$T=\{\partial_e g=0\}$$
is a **regular** curve wherever $\partial_e^2 g=\lambda\ne0$, by the implicit function theorem in the $e$ direction — one equation, two unknowns, transverse Jacobian. The tangency locus is a ridge/valley curve of $g$ and is computable with no singular solving at all. Trace $T$ by certified continuation (Krawczyk on the 1D system, transverse Jacobian $\lambda$), then read $g$ along it:

| $\operatorname{sign}g$ on $T$ | configuration | output |
|---|---|---|
| $|g|\le\tau$ throughout | genuine tangential contact | tangential edge, declared $G^1$, **no membership flip** — the surfaces touch without crossing, both sides keep their material state |
| $g>0$ (or $<0$) with margin | **near-tangency without contact** | `EmptyResult` with a *certified clearance* — the case production kernels return as an empty solid by accident, returned here by proof |
| sign changes at isolated points | tangency curve with **transition points** | split $T$ there; each transition is an $A_{2}$/$A_3$ point handed to SS-TAN-BLOW-001 |

**SS-TAN-CRV-001.** Pre: $\operatorname{rank}\operatorname{Hess}g=1$ with $|\lambda|\ge\delta_\lambda$ over the region, certified by interval evaluation of $\operatorname{Hess}g$ on a tube about the traced locus. Algorithm: trace $T$; classify by the table; emit via REP-CRV-001 with §6.2 including (iv). Termination $d\le\log_2(CDL/\delta_\lambda)$. Modulus: transverse displacement $\omega(\varepsilon)=\sqrt{2\varepsilon/|\lambda|}$.

**SS-TAN-CRV-END-001.** Endpoints of $T$: either $\lambda\to0$ (rank drops to 0 — corank 2, escalate to SS-COIN-001 or refuse), or $T$ exits the trimmed domain (a boundary incidence, handed to §11), or $T$ closes up (a closed tangency curve — the coaxial case; check the periodicity contract §8).

#### 9.2.3 Near-degeneracy is more expensive than degeneracy

Exact tangency, coaxiality and coincidence are decided by **rational predicates on carrier parameters** in the analytic subset — cheaper than the transverse case. What is expensive is tangency-to-$10^{-7}$: $\delta$ small but nonzero, so no exact predicate fires and subdivision runs to the budget. Mechanical design produces the cheap case; float round-tripping through exchange formats converts it into the expensive one, and offset/blend construction manufactures more of it.

**TAN-SNAP-001 — backward promotion of near-degeneracy.** Given a pair certified within $\eta$ of an exact tangency, coincidence or coaxiality, and $\eta\le\tau_{\text{in}}$, replace the pair by the exactly-degenerate pair, decide it symbolically, and record $\eta$ in the backward budget per §0.1. This is what the backward-error semantics were paid for, and by prevalence it is the highest-leverage cell in the atlas.

*The obstruction is joint consistency, not the individual snap.* Snapping one pair can un-snap another, and a chain of pairwise-admissible snaps can move a feature by an inadmissible amount. So the decision is **one certified clustering over the whole constraint set** (§5, with the admissibility bound $R_C\le\theta\cdot\underline{\mathrm{lfs}}_\sigma$ applied to the constraint cluster), never a sequence of pairwise snaps. **[PROVISIONAL SUFFICIENT GATE]** for the joint version: all snapped constraints lie in one admissible cluster and the induced displacement field is certified $\le\tau_{\text{in}}$ in the sup norm. The pairwise version is **[DERIVED]** only when the pair's cluster is a singleton.

Refusals: `SnapExceedsBackwardBudget`, `SnapClusterInadmissible` (the joint gate fails — fall back to the numeric path, do not snap partially).

### 9.3 Atlas index

$(\dim)\times(\text{transversality margin})\times(\text{contact order / singularity type})\times(\text{boundary/seam incidence})$, assigned **per connected component of the certified solution set**, not per surface pair. One face pair routinely carries transverse branches, an isolated tangency and a seam incidence at once.

### 9.4 Cells

**SS-TR-001 — transverse regular.** Pre: immersion $\iota$, $\sin\theta\ge\delta$, separation $\ge\sigma$. Algorithm: Bernstein subdivision + Krawczyk; emit via REP-CRV-001. Termination $d\le\log_2(CDL/\delta\sigma)$. Modulus $\omega(\varepsilon)=\varepsilon/\sin\theta$. Adjacent: $\delta\to0\Rightarrow$ SS-TAN-\*, $\iota\to0\Rightarrow$ SS-CHART-001, residual $\equiv0\Rightarrow$ SS-COIN-001.

**SS-TAN-BLOW-001 — isolated tangential contact, any $A_k/D_k/E_k$ with non-degenerate leading form.** Supersedes the r3 triple SS-TAN-ELL/HYP/DEG, which are now the $m=2$ special cases. Pre: $\nabla g=0$ with $\{g=0,\nabla g=0\}$ certified $0$-dimensional locally; $\operatorname{disc}(L_m)\ge\delta_{\text{disc}}$; certified Milnor radius $r_0$. Algorithm: polar blow-up, isolate the roots of $L_m$ on $S^1$ (BG-NUM-002-class univariate isolation), continue each to a branch by Krawczyk on $G$, then sector signs for flip parity. Emits: branch count, contact order $k$, flip parity per branch, $\mu$ for provenance. Termination $d\le\log_2(CDL/\delta_{\text{disc}})$. Modulus $\omega(\varepsilon)=C\varepsilon^{1/(k+1)}$. Refusals: `LeadingFormDegenerate` (repeated root — route to SS-TAN-NEWTON-001), `MilnorRadiusUnresolved`.

**SS-TAN-NEWTON-001 — degenerate leading form.** Iterated blow-up along the Newton polygon (Newton–Puiseux), budgeted by the number of admissible faces. **[PROVISIONAL SUFFICIENT GATE]**: the Newton polygon has a face whose associated form is non-degenerate after one further blow-up. Caps at `CertifiedEquivalentConstruction`.

**SS-TAN-CRV-001 / SS-TAN-CRV-END-001** — per §9.2.2. These, not the isolated cells, are what fillet supports, coaxial features and resting contacts actually need.

**SS-CHART-001** — chart degeneracy with geometric transversality: reparameterise or work in the blown-up chart.

**SS-COIN-001** — support coincidence: residual identically zero on a positive-area region, detected by exact rational arithmetic on control points; feeds the ON atlas of §13.

**TAN-SNAP-001** — §9.2.3. Runs *before* the numeric cells, since a successful snap replaces an expensive numeric instance with an exact symbolic one.

**CC-\*, CS-\*** mirror these with $2\times2$ and $3\times3$ Jacobians and the same margin discipline. The curve–curve analogue of §9.2.2 — two curves tangent along a shared arc — is CC-TAN-ARC-001 and is the profile-level cause of most loft and sweep tangency failures.

**Scoping note.** Tangency is designed into mechanical parts — every fillet is tangent to its supports, every counterbore coaxial. An $E_c$ requiring transversality excludes most real parts, so the tangential cells are early-roadmap. Within them the priority is not the exotic singularities: it is SS-TAN-CRV-001 and TAN-SNAP-001, which together cover the configurations mechanical design actually produces. $A_{k\ge3}$ isolated contact is comparatively rare and is specified above mainly so that the transition points of §9.2.2 have somewhere to go.

---

## 10. Self-intersection engine

For $S:D\to\mathbb{R}^3$, $Z=\{(p,q):S(p)=S(q),p\ne q\}$. The naive system vanishes identically on the diagonal, so every box straddling $\Delta$ subdivides forever. **Deflate by blow-up.** With $m=\tfrac12(p+q)$, $q-p=h\omega$, $\lVert\omega\rVert=1$:
$$G(p,q)=-h\,H(m,\omega,h),\qquad H(m,\omega,0)=DS(m)\,\omega .$$
Where $S$ is an immersion, $H\ne0$ at $h=0$, so the deflated system has no diagonal solutions and certified subdivision applies on $D\times S^1\times[0,h_{\max}]$, quotiented by $(p,q)\mapsto(q,p)$.

**SI-DEF-001.** Pre: $\lVert S_u\times S_v\rVert\ge\iota>0$; $H$ transverse with margin $\delta$. Emits a **seam-like edge with two coedges on the same face** — which is why $U$ must be first-class. Termination $d\le\log_2(CDL/\delta\iota)$. Adjacent: $\iota\to0\Rightarrow$ SI-CHART-001; $h\to0$ with $H\to0\Rightarrow$ SI-PINCH-001 (Whitney umbrella, curve terminates on the surface).

**SI-TRIM-001.** Select retained sheets by **local material side** relative to the generator, not global point-in-solid (the object is not yet a solid). Certificate: discarded sheets strictly interior to the retained boundary.

---

## 11. Arrangement engine

Contact classification of every curve pair (CC atlas) $\to$ atomisation at certified contact clusters $\to$ DCEL embedding on the fundamental domain with bounded unfolding (§8) $\to$ cell labelling $\to$ contradiction detection with the offending arc localised. Termination from finitely many clusters; §6.3 guarantees the arrangement built on approximants is the exact one.

---

## 12. Membership by propagation

> **Theorem.** For connected face $f$ of $A$ and $\Xi=f\cap\partial B$: $\chi_B$ is locally constant on $f\setminus\Xi$, the fragments are exactly the components of $f\setminus\Xi$, and their dual adjacency graph is connected. **One certified seed per face** determines all fragments.

**Flip parity.** Crossing an arc flips $\chi_B$ iff the contact order is odd — read from §9.2. Naive parity counting fails exactly on the even-order arcs, the standard source of inverted-material defects. The *certificate* for this is the sector-sign test of §9.2.1, not a computed multiplicity: evaluate $\operatorname{sign}g$ by interval arithmetic at one certified point per sector of the link. A tangential arc from SS-TAN-CRV-001 is even-order by construction and therefore **never** flips — that single fact removes the commonest inverted-material defect, since fillet-to-support contacts are exactly these arcs.

**CLS-SEED-001.** Ray direction admissible iff the ray meets no vertex, edge, or tangential locus — certified by interval separation, not symbolic-perturbation folklore. Resample from a certified admissible set; `NoAdmissibleRay` routes to winding number by certified quadrature.

**CLS-PROP-001.** Spanning-tree propagation plus verification of every non-tree edge. Cycle disagreement is a contradiction witness that localises the offending arc — a better diagnostic than a failed shell walk.

---

## 13. Boolean reconstruction

**Semantics.** Regularized: $A\cup^*B=\mathrm{cl}\,\mathrm{int}(A\cup B)$, likewise $\cap^*,\setminus^*$. Zero-thickness walls cannot arise; non-regularized results are a separately declared semantics.

**Procedure.** Face–face intersections (§9) with certified convex-hull pruning $\to$ atomisation (§11) $\to$ classification (§12) $\to$ selection $\to$ orientation $\to$ coincident-locus quotient (§5) $\to$ shell assembly with link **and** Euler tests, nesting determination $\to$ emission with the §0.1 certificate.

**Selection, non-coincident fragments.**

| | keep from $A$ | keep from $B$ |
|---|---|---|
| $\cup^*$ | OUT of $B$ | OUT of $A$ |
| $\cap^*$ | IN $B$ | IN $A$ |
| $\setminus^*$ | OUT of $B$ | IN $A$, **orientation reversed** |

### 13.1 Coincident fragments: material-state formulation

Orientation alone is not the right state, and enumerating cases invites error. The robust symbolic object is the **material state on each normal side**. Fix a reference normal $n$ on the coincident fragment and record
$$\big(m_A^-,m_A^+,m_B^-,m_B^+\big)\in\{0,1\}^4 .$$
Then the result's material state is obtained by applying the Boolean truth table **pointwise on each side**:
$$m_R^\pm=\mathrm{op}_{\text{bool}}(m_A^\pm,m_B^\pm)$$
and the selection rule is a single line:

> **Keep the fragment iff $m_R^-\ne m_R^+$; orient its normal toward the side with $m_R=0$.**

No case enumeration. Sanity checks: two cubes sharing a face give $(1,0,0,1)$ — union $m_R=(1,1)$ drop; intersection $(0,0)$ drop; difference $(1,0)$ keep, oriented outward from $A$. Nested contact gives $(1,0,1,0)$ — union keep once, intersection keep once, difference $(0,0)$ drop. These reproduce the orientation-only table exactly, which is the consistency check that the reduction is sound for two regularized solids.

The material-state form is strictly more general and is what is actually needed for: fragments produced *after* trimming (where a side's membership is inherited rather than read off an operand orientation), $n$-ary Booleans (state in $\{0,1\}^{2n}$), declared non-regularized or open-shell semantics (where $m^-=m^+$ is legal and denotes a dangling face), and three-way coincidence. Adopt it as the primitive; the orientation table is a derived special case.

---

## 14. Envelopes and discriminants

$\mathcal{E}=\{x:\exists\alpha,\ F(x,\alpha)=0\wedge F_\alpha(x,\alpha)=0\}$.

| condition | stratum |
|---|---|
| $\nabla_xF\ne0$, $F_{\alpha\alpha}\ne0$ | smooth sheet, foliated by characteristics |
| $F_{\alpha\alpha}=0$, $F_{\alpha\alpha\alpha}\ne0$ | $A_2$: cuspidal edge (edge of regression) |
| $F_{\alpha\alpha}=F_{\alpha\alpha\alpha}=0$ | $A_3$: swallowtail — self-intersects, must trim |
| $\nabla_xF=0$ | generator singularity |

**ENV-REG-001.** Pre $|F_{\alpha\alpha}|\ge\delta_2$, $\lVert\nabla_xF\rVert\ge\delta_1$; IFT radius $\delta_1\delta_2/L$; emit via REP-SRF-001.
**ENV-A2-001.** Isolate $\{F_{\alpha\alpha}=0\}$ and **emit the regression edge as a real edge with declared tangential singularity**; the two sheets are separate faces. Omitting this is a common source of invalid swept faces.
**ENV-A3-001.** Hand to SI-TRIM-001; `Unsupported` if unresolvable within $\tau$.

---

## 15. Generative operations

Each factors as: carrier generation with its own singularity atlas $\to$ self-intersection trimming (§10) $\to$ arrangement and selection $\to$ capping $\to$ collapse where generators degenerate. Every operation separates **local regularity** from **global embedding**, following the offset pattern.

### 15.1 Extrude

Profile $P$ (wire or face) along $d$, optionally with draft angle $\alpha$ or up to a target.

**EXT-REG-001 — straight extrusion.**
*Pre:* $P$ simple (certified by the CC atlas on the profile), planar with normal $n_P$, $|\langle \hat d,n_P\rangle|\ge\delta$; profile tolerance $<\theta\cdot\underline{\mathrm{lfs}}(P)$.
*Unknowns:* lateral carriers $x(t,s)=c_e(t)+s\,d$ per profile edge; two caps.
*Algorithm:* direct; line$\to$plane, circle$\to$cylinder, general$\to$general cylinder, all exact in $\mathcal{G}$ (no $\mathrm{rep}$ error on the lateral faces).
*Certificate:* **[DERIVED]** the map $P\times[0,1]\to\mathbb{R}^3$ is injective whenever $P$ is simple and $\langle\hat d,n_P\rangle\ne0$, so global embedding is proved outright — extrusion is one of the few operations needing no self-intersection pass.
*Produces:* one lateral face per profile edge; caps from $P$ with the start cap orientation-reversed; vertical edges from profile vertices.
*Modulus:* $\omega(\varepsilon)=\varepsilon$ (isometric in $d$).
*Refusals:* `DegenerateDirection` ($\delta$ gate), `ProfileNotSimple`.

**EXT-PROF-SMOOTH-001 — vertex suppression.** At a profile vertex with tangent discontinuity below an angular threshold, the two lateral faces are $G^1$: emit a smooth edge or merge the faces. **[POLICY]** — the threshold is a choice, recorded in provenance so regeneration is stable.

**EXT-DRAFT-001 — drafted extrusion.** With draft $\alpha$, the section at height $s$ is the **2D offset** of $P$ by $s\tan\alpha$. Drafted extrusion therefore *inherits the whole offset atlas in the profile plane* — this is the structural fact that makes draft hard, and it is why draft failures look like offset failures.
*Local gate* **[DERIVED]:** offset regular iff $s\tan\alpha\ne1/\kappa_{2D}(t)$ for all profile points, i.e. $h\tan\alpha<1/\kappa^{+}_{\max}$ over the height range.
*Global gate* **[DERIVED]:** $h\tan\alpha<\underline\rho_{2D}(P)$.
*Events:* at a concave corner of curvature $\kappa$ the offset self-annihilates at height $h^*=1/(\kappa\tan\alpha)$ — a **face disappearance at a certified height** which splits the extrusion into two topological regimes and *requires a horizontal edge at $h^*$*. Emitting no edge there is a topology error, not an approximation error.
*Cells:* EXT-DRAFT-VANISH-001 (event height isolation and the induced edge), EXT-DRAFT-CORNER-001 (convex corner treatment: arc / sharp-extend / mitre — **[POLICY]**), EXT-DRAFT-SI-001 (beyond the global gate $\to$ §10).

**EXT-UPTO-001 — extrude to a target face/surface.**
*Pre:* every generator line meets the target with $|\langle \hat d,n_{\text{target}}\rangle|\ge\delta$ over the whole profile shadow; the target covers the shadow (certified by projecting the profile bounding region).
*Algorithm:* CS atlas per generator, then the cap is the pullback of the target through the projection.
*Refusals:* `TargetNotReached`, `TargetTangentToDirection`, `ShadowNotCovered`.

### 15.2 Revolve

Profile $P$ about axis $A$ through angle $\varphi\in(0,2\pi]$.

**REV-REG-001 — axis-disjoint partial revolve.**
*Pre:* $P$ simple, coplanar with $A$ (or handle generally as `Unsupported`), $\operatorname{dist}(P,A)\ge\delta>0$, $\varphi\le2\pi$.
*Carriers:* per profile edge — line$\to$plane/cylinder/cone, circle$\to$torus/sphere, general$\to$surface of revolution, all exact in $\mathcal{G}$.
*Certificate:* **[DERIVED]** the revolve map is injective on $P\times[0,\varphi)$ when $\operatorname{dist}(P,A)>0$ and $P$ is simple, so global embedding is proved without a self-intersection pass.
*Produces:* lateral faces, two planar caps (start and end), radial edges.
*Modulus:* $\omega(\varepsilon)=\varepsilon\cdot\max(1,R_{\max}/\operatorname{dist}(P,A))$ — angular error amplifies with radius ratio.

**REV-FULL-001 — $\varphi=2\pi$.** Seam edge creation, periodicity contract $\Lambda=\langle2\pi\rangle$, coedge pairing across the seam, **no caps**. Gate: the seam must not coincide with a profile tangency, else the seam edge is degenerate.

**REV-POLE-001 — profile endpoint on the axis.** Produces a **pole**. Sub-classified by the angle $\beta$ between the profile tangent and the axis at contact:
- $\beta$ bounded away from $0$ and $\pi/2$: conical apex — emit an apex vertex with a degenerate edge and run the link test in the blown-up chart;
- $\beta\to\pi/2$ (profile meets the axis perpendicularly): smooth pole (sphere-like) — emit a degenerate edge but no geometric edge, and certify $G^1$ across the pole;
- $\beta\to0$ (profile tangent to the axis): cusp — `Unsupported`.
This is exactly where §1's degenerate edges pay for themselves.

**REV-AXIS-TOUCH-001 — interior contact.** Profile touches the axis at an interior point: the result has a pinch point, a non-manifold vertex. Declared singularity or `Unsupported`; never silently manifold.

**REV-AXIS-CROSS-001 — profile crosses the axis.** The naive sweep double-covers. Split the profile at certified axis crossings, revolve each side, and union the results under §13. Certificate: the union is the regularized image.

**REV-MULTI-001 — $\varphi>2\pi$ (helical/multi-turn).** Guaranteed self-overlap; `Unsupported` unless a declared non-regularized semantics.

### 15.3 Sweep

Profile $Q$ along spine $c(s)$ with frame $R(s)$: $x(s,t)=c(s)+R(s)\,q(t)$.

**Frame cells.**
- SWP-FRM-FRENET-001: requires $\kappa(s)\ne0$ everywhere; certified by isolating zeros of $\kappa$ along the spine. Fails at inflections — refusal `FrenetUndefined`, route to RMF.
- SWP-FRM-RMF-001: rotation-minimising frame, defined for any regular spine; obtained from a linear ODE, which must be integrated with **validated** methods so the frame error enters $\tau_{\text{rep}}$ rather than being unbounded.
- SWP-FRM-FIXED-001: fixed reference direction; gate $|\hat T(s)\times \hat u_{\text{ref}}|\ge\delta$.
- SWP-FRM-CLOSED-001: closed spine — the RMF need not close up. Gate: the **holonomy defect** (total twist mod $2\pi$) must vanish or be absorbed by a declared twist. Silently ignoring the defect produces a discontinuous surface at the seam; this is a frequently missed condition.

**SWP-REG-001 — local regularity.** **[DERIVED]** With an RMF, $\partial_s x=\big(1-\kappa(s)\,\langle q(t),N(s)\rangle\big)T+\dots$, so the sweep is immersive iff
$$\kappa(s)\,\langle q(t),N(s)\rangle<1\quad\text{pointwise},$$
with the cheap sufficient test $\kappa_{\max}R_{\text{circ}}<1$ where $R_{\text{circ}}$ is the profile circumradius. This supersedes the earlier heuristic: the pointwise form is the actual condition and the circumradius form is a certified sufficient corollary.

**SWP-EMB-001 — global embedding.** **[DERIVED]** The tube of radius $R_{\text{circ}}$ about the spine is embedded iff $R_{\text{circ}}<\mathrm{reach}(c)$; since the sweep is contained in that tube and each cross-section map is injective for simple $Q$, $R_{\text{circ}}<\underline\rho(c)$ is a sufficient global embedding certificate. Beyond it, run §10 — do **not** treat local regularity as global validity. This is the offset pattern (focal $\ne$ medial) transplanted, and it makes the sweep gates derived rather than provisional.

**SWP-SPINE-DEG-001.** Spine corner ($G^0$): mitre or round treatment **[POLICY]**, each with its own carrier cell. Spine cusp or $c'=0$: `Unsupported`.

**SWP-SCALE-TWIST-001.** Scaling $\lambda(s)$ and twist $\vartheta(s)$ add terms to $\partial_sx$; the immersion condition becomes $\det$ of a $3\times2$ system with $\lambda',\vartheta'$ contributions. **[PROVISIONAL SUFFICIENT GATE]** until the full discriminant is derived: $\kappa_{\max}R_{\text{circ}}+|\lambda'|R_{\text{circ}}/\lambda+|\vartheta'|R_{\text{circ}}<1$.

**SWP-CAP-001.** End caps planar iff $Q$ is planar and normal to $T$ at the ends; otherwise the cap is the profile face transported, and needs its own planarity or ruled-patch certificate.

### 15.4 Loft

**LFT-CORR-001 — section correspondence.** Sections $Q_1,\dots,Q_k$ must be refined to a common edge count; the correspondence is a matching that must be orientation-consistent and **non-crossing** (monotone in the cyclic parameter). Well-posedness certificate: the induced surface is immersive — and, in the parallel-plane case, the chord margin $m_0$ of LFT-RULED-001 is the quantitative form of both non-crossing and immersion, so a correspondence is well-posed exactly when it admits a certified $m_0>0$. The *choice* of matching (minimal twist, minimal area) is **[POLICY]** and must be recorded in provenance, because a correspondence that silently changes on regeneration is a naming-stability bug (§20), not a geometry bug.

**LFT-RULED-001 — two sections, ruled.** Exact and cheap, and it **depends on LFT-CORR-001**: a non-crossing, orientation-consistent correspondence is a precondition of this cell, not an alternative to its gate.

*Withdrawn.* The earlier gate — sections in parallel planes and every ruling positive along the separating normal, claimed $\iff$ injectivity — is withdrawn in both directions. It is **vacuous**: a ruling joins a point of $\Pi_0$ to a point of $\Pi_h$, so its component along the separating normal equals $h>0$ by construction, for *every* correspondence including a crossing one. The endpoint-swapped correspondence sends $\gamma_1(t_1)\mapsto\gamma_2(t_2)$ and $\gamma_1(t_2)\mapsto\gamma_2(t_1)$: both rulings are monotone in the normal direction and they meet at mid-height. A per-ruling condition cannot see crossings *between* rulings, which is the only failure mode that matters here.

*Correct criterion* **[DERIVED]**. For $\gamma_1\subset\Pi_0$, $\gamma_2\subset\Pi_h$ in parallel planes, $\Phi(t,s)=(1-s)\gamma_1(t)+s\gamma_2(t)$ has height $sh$ independent of $t$, so $\Phi(t,s)=\Phi(t',s')$ forces $s=s'$. Hence

> $\Phi$ is injective $\iff$ every intermediate section $\gamma_s=(1-s)\gamma_1+s\gamma_2$ is simple, for all $s\in[0,1]$.

This is an honest iff, and it reduces global embedding of the ruled loft to a one-parameter family of planar simplicity tests.

*Quantitative gate* **[DERIVED]**. Certify the **chord margin**
$$m=\inf_{s\in[0,1]}\ \inf_{t\ne t'}\ \frac{\lVert\gamma_s(t)-\gamma_s(t')\rVert}{d_{S^1}(t,t')}\ \ge\ m_0>0,$$
whose diagonal limit $t'\to t$ is $\lVert\gamma_s'(t)\rVert\ge m_0$ — so immersion is a boundary case of the same quantity rather than a separate condition, and $m_0$ is simultaneously the non-crossing margin LFT-CORR-001 needs. Certification is SI-DEF-001's diagonal deflation applied to a one-parameter family: blow up along $\{t=t'\}$ and subdivide on $D\times D\times[0,1]$ quotiented by $(t,t')\mapsto(t',t)$. Depth bound $d\le\log_2(CDL/m_0)$. Refusals: `SectionCrossingAtInteriorHeight` (certified crossing, with the witness $(t,t',s)$ — a far better diagnostic than a downstream self-intersection report), `ChordMarginUnresolved`.

*Cheap sufficient corollary* **[DERIVED]**. If $\gamma_1,\gamma_2$ are strictly convex and $C^1$ and the correspondence is the **common-outer-normal** matching $u\in S^1$, then $\gamma_s(u)$ traces the boundary of the Minkowski combination $(1-s)K_1\oplus sK_2$, hence is convex and simple for every $s$. With the monotonicity margins $m_i=\inf_{u\ne u'}\langle\gamma_i(u)-\gamma_i(u'),u-u'\rangle/|u-u'|^2>0$ one gets $m_s\ge(1-s)m_1+sm_2\ge\min(m_1,m_2)$, so no subdivision is needed at all. Under any other monotone correspondence convexity is **not** inherited and the chord test is required — which is the precise sense in which correspondence choice is a well-posedness question and not only a naming-stability one.

Non-parallel section planes, or non-planar sections, lose the height argument entirely and with it the iff: fall back to §10.

**LFT-TOPO-CHG-001.** Differing section topology (different hole counts): `Unsupported` unless a declared branching semantics with an explicit splitting locus.

**LFT-POLE-001.** A section collapsed to a point: degenerate edges and a pole, with the same $\beta$-angle sub-classification as REV-POLE-001, plus a $G^1$ certificate if smoothness at the pole is claimed.

**LFT-CONT-001.** Tangent lofting ($G^1$ to adjacent faces at the ends): adds boundary constraints to the surface fit; gate on solvability of the constrained fit and on the resulting surface still being immersive.

**LFT-SI-001.** Self-intersection from widely separated or strongly twisted sections $\to$ §10. There is no cheap sufficient gate in general; this is why loft is the least closed of the four.

---

## 16. Offset and shell

Three conditions, never conflated:
- **Local regularity** **[DERIVED]**: $1-d\kappa_i\ne0$ for both principal curvatures. Violation is a **focal event**, singular at a point.
- **Global embedding** **[DERIVED]**: $d<\underline\rho(S)$ on the offset side. Violation is a **medial-axis event**: smooth everywhere, yet self-intersecting.
- **Topological stability** **[DERIVED]**: $d$ avoids the critical values of $\operatorname{dist}(\cdot,\partial\Omega)$ in the *generalized* sense of §16.3; the governing quantity is the weak feature size $\mathrm{wfs}\ge\mathrm{reach}$. Violation is neither focal nor medial: it is a genuine topology change, and it is to be realised rather than refused.

The three thresholds are ordered $\min_i 1/\kappa_i^{+}\ \gtrless\ \mathrm{reach}\ \le\ \mathrm{wfs}$ with only the second inequality guaranteed, which is why each is gated separately.

### 16.1 Per-face carriers

**OFF-CARRIER-001.** Plane$\to$plane; cylinder$\to$cylinder $r\pm d$; sphere$\to$sphere; cone$\to$cone with shifted apex (and apex vanishing when $d$ exceeds the apex distance — a topology event, not an approximation); torus$\to$torus $r\pm d$ (with the inner-radius sign change at $d=r_{\text{minor}}$). All exact.

**OFF-CARRIER-NURBS-001.** The offset of a NURBS is **not** a NURBS. It must go through $\mathrm{rep}$ with a certified error bound, and the error is *not* uniform: it degrades as $1/|1-d\kappa|$. Refusal `OffsetApproximationBudgetExhausted` when the required $\tau_{\text{rep}}$ is unattainable at degree $\le3$ within the span budget — this is a real and common outcome that must not be silently absorbed.

### 16.2 Edge and vertex treatment

The offset of a boundary with sharp edges is not the offset of a smooth surface, which is precisely why §6.1 stratifies reach.

**OFF-EDGE-CONVEX-001.** Offset faces separate; the gap is filled by the natural offset of the edge stratum — a cylindrical patch of radius $d$ about $c_e$ (for a straight edge, a partial cylinder; generally a pipe surface). Gate **[DERIVED]** $d<\underline\rho(c_e)$ and $d\overline\kappa_e<1$ (SWP-REG-001 applied to the edge). Alternatives — extend-and-intersect (sharp) or mitre — are **[POLICY]**, and the choice must be stable across regeneration.

**OFF-EDGE-CONCAVE-001.** Offset faces overlap; trim by mutual intersection (SS atlas). Gate: the intersection exists with margin $\delta$; as the dihedral $\psi\to0$ the trim curve conditioning degrades like $1/\sin(\psi/2)$, which is the concrete reason invariant 9 bounds $\psi$ away from $0$.

**OFF-VERTEX-CONVEX-001.** Spherical patch of radius $d$, trimmed by the incident edge patches.
**OFF-VERTEX-CONCAVE-001.** Mutual trim of $\ge3$ offset faces; gate $|\det[n_1\,n_2\,n_3]|\ge\delta$ (the three-surface vertex condition).
**OFF-VERTEX-SADDLE-001.** Mixed convexity: partial spherical patch plus partial trim, with the patch boundary determined by the sign change of the dihedral around the star. The hardest vertex case; **[PROVISIONAL SUFFICIENT GATE]**: all incident dihedral angles bounded away from $\pi$ and the star separation exceeding $2d$.

### 16.3 Topology events

**OFF-TOPO-001.** **[DERIVED, one direction only]** $\operatorname{dist}(\cdot,\partial\Omega)$ is Lipschitz and not $C^1$, and for a piecewise-smooth B-rep the topologically relevant critical points are *not* smooth Morse points: the medial sheets generated by edges and vertices — the ones every mechanical part has — carry no smooth critical points at all, so ordinary Morse theory of $d$ misses precisely the events this gate exists to catch. The correct notion is the **generalized (Grove–Shiohama / Clarke) critical point**. With $\Gamma(x)$ the set of nearest points on $\partial\Omega$ and $\theta(x)$ the centre of the smallest ball enclosing $\Gamma(x)$, define the critical function
$$\chi(x)=\frac{\lVert x-\theta(x)\rVert}{d(x)}\in[0,1],$$
the magnitude of the generalized gradient. $x$ is **critical** iff $\chi(x)=0$, equivalently $x\in\mathrm{conv}\,\Gamma(x)$. The **weak feature size** $\mathrm{wfs}(\partial\Omega)$ is the infimum of the positive critical values, and $\mathrm{wfs}\ge\mathrm{reach}$ — for $d<\mathrm{reach}$ nearest points are unique and $\chi\equiv1$.

*Isotopy lemma used.* If $[a,b]\subset(0,\infty)$ contains no critical value, the sublevel sets $d^{-1}([0,a])$ and $d^{-1}([0,b])$ are isotopic, so the offsets at $a$ and $b$ have the same topology. **Only this direction is claimed and only this direction is needed.** The converse — "topology changes *exactly* at critical values" — is withdrawn: a critical value need not change topology (a degenerate generalized critical point can be topologically inert), so asserting the equivalence would claim more than the lemma supplies and would make a $\ne$ result at a critical value look like a defect.

*Gate.* Certify $\chi\ge\mu>0$ on the band $d^{-1}([d-\eta,d+\eta])$ ($\mu$-criticality), by bounding the angular spread of $\Gamma(x)$ over that band. This is a stratified computation against the medial sheets of face, edge and vertex strata per §6.1 — a nearest-point set spanning two faces across an edge is the normal case, not a degeneracy — and it is not a smooth critical-point system.

*When the gate fails* it is still not a refusal: isolate the critical values in the band, order them, and realise the offsets between consecutive values, emitting the shell count appropriate to each interval. Refusal `CriticalValueUnresolved` applies only when the isolation itself exhausts budget.

**OFF-MED-001.** $d$ not certified $<\underline\rho$ (i.e. possibly $d\ge\mathrm{reach}$): run SI-DEF-001 on the offset, trim by SI-TRIM-001, then **re-run the link test** — trimmed offsets frequently change genus, which an Euler check alone accepts.

**OFF-VANISH-001.** A face whose offset domain empties is removed and neighbours re-stitched; the extended neighbour surfaces must intersect in a certified new edge, else `ShellNotReconstructible`.

### 16.4 Shell

**SHL-SEL-001.** Removed-face selection: the boundary of the removed set must form valid wires; disconnected removals produce multiple openings, each capped independently.
**SHL-OFF-001.** Inward offset of the retained faces by thickness $t$ via §16.1–16.3.
**SHL-CAP-001.** Ruled caps between each opening wire and its offset image. The LFT-RULED-001 citation is withdrawn: the two wires are in general neither planar nor parallel, so that cell's height argument does not apply, and its per-ruling form was vacuous in any case. Gate **[DERIVED]** directly instead: the cap map $(t,s)\mapsto\gamma(t)+s\,t\,n(t)$ is the restriction to the opening wire of the normal displacement map of the retained surface, which is injective on $\{0\le s\le1\}$ whenever $t<\underline\rho_{\text{inward}}$ along the opening (tubular neighbourhood theorem, §6.1 stratum-wise). This is the §16 embedding gate evaluated along a curve, not a loft gate; the two happen to coincide numerically with SHL-THICK-001, which is a consistency check rather than a coincidence.
**SHL-THICK-001.** Global thin-wall gate **[DERIVED]**: $t<\underline\rho_{\text{inward}}(\partial\Omega)$ evaluated stratum-wise per §6.1; else `WallThicknessExceedsFeatureSize`. Evaluating it with a single global reach is precisely the error §6.1 corrects — sharp edges would drive it to zero.
**SHL-VAR-001.** Per-face thickness: a variable offset, so the carrier is an envelope (§14) rather than a parallel surface. **[PROVISIONAL SUFFICIENT GATE]** $|\nabla t|<1$ pointwise plus the constant-thickness gates at $t_{\max}$.

---

## 17. Blends: fillet, chamfer, corner

Fillet and chamfer are the same construction with a different rolling primitive, and their end conditions, spillover tests, mutual trims and corner treatment are **literally the same cells**. Revisions through r3 specified only the fillet and gave the corner one sentence; this section separates the shared substrate from the two primitives so the corner is written once and chamfer does not duplicate a fillet's worth of machinery.

### 17.1 Shared substrate

A **blend** on an edge $e$ between supports $f_1,f_2$ is generated by a rolling primitive $\Pi$ maintained in contact with both:

| primitive | blend | carrier | continuity to supports |
|---|---|---|---|
| ball of radius $r$ | fillet | canal surface (envelope of the ball family) | $G^1$ |
| plane | chamfer | ruled, developable in the envelope convention | $G^0$ with certified nonzero dihedral |
| general profile | shaped blend | envelope (§14) | declared |

**BLD-SUP-001 — support admissibility.** Common to both: the dihedral $\psi$ along $e$ certified bounded away from $0$ and $2\pi$ (invariant 9), **and away from $\pi$** — $|\psi-\pi|\ge\delta_\psi$. The last is new and is chamfer-critical: a chamfer of a tangent ($G^1$) edge is empty, and a fillet of one is the identity. Refusal `EdgeAlreadySmooth`.

**BLD-CONTACT-001 — contact curves.** Both primitives produce two contact curves $\gamma_i\subset f_i$. Shared gates: $\gamma_i$ regular; $\gamma_i$ inside the trimmed domain of $f_i$ (else `BlendSpillover`, or a certified face disappearance); $\gamma_i$ separated from the other boundary wires of $f_i$ by $\underline{\mathrm{lfs}}$.

**BLD-EMB-001 — global embedding.** The blend carrier is a one-parameter family swept along the edge; local regularity of the family is *not* global embedding. Certify via the chord margin of LFT-RULED-001 applied to the cross-section family. For the fillet, the parallel-plane iff does not hold and the chord margin is a sufficient gate; the sharper statement is gate F-3 below.

**BLD-END-001 — end conditions** (was FIL-END-001). Cap, run-out onto the adjacent face, or extension. Each a sub-cell with a continuity certificate. Shared verbatim between fillet and chamfer.

**BLD-CNR-\* — corners.** §17.4. Shared.

**BLD-FIL-001 — mutual trim** (was FIL-FIL-001). Adjacent blended edges whose carriers overlap; gate: the sum of the two primitives' transverse extents is below the local separation, and the mutual intersection is transverse with margin $\delta$.

**REP-G1-001 — tangency-preserving representation.** New, and required by every $G^1$ blend. Positional $\tau_{\text{rep}}$ does **not** certify $G^1$: two surfaces can agree to $\varepsilon$ everywhere and disagree in normal direction by $O(1)$. A $G^1$ claim is a claim about first derivatives, so the certificate is a triple $(\varepsilon,\theta,\theta_\partial)$ — positional error, §6.2(ii) tangent-angle bound, and additionally a **normal-cone agreement bound $\theta_\partial$ along the shared boundary** between the emitted patch and the support surface. For a NURBS approximant this is a bound on the hodograph difference, obtained from the same control-polygon machinery as $\varepsilon$ but on the derivative curve. A blend emitting only $(\varepsilon,\theta)$ and asserting $G^1$ is asserting something it has not certified.

### 17.2 Fillet

Rolling ball on $f_1,f_2$ with radius $r$. Spine $\Sigma_r=O_{-r}(f_1)\cap O_{-r}(f_2)$; carrier = envelope of $\{B(c,r):c\in\Sigma_r\}$; contact curves $\gamma_i(\alpha)=c(\alpha)-r\,n_i$.

Gates (F-1…F-6, unchanged from r3 apart from numbering and the shared-substrate attributions):
1. **[DERIVED]** $r<\min_i(1/\kappa^{+}_{\max}(f_i))-\tau$ — offsets locally regular; else `RadiusExceedsCurvature`.
2. **[DERIVED]** $\Sigma_r\ne\emptyset$ with transversality margin $\delta$ between the two offsets; else `NoSpine`.
3. **[DERIVED]** $r<\underline\rho(\Sigma_r)$ — the canal surface about the spine is embedded (SWP-EMB-001 applied to the spine). This is BLD-EMB-001 in its sharp form.
4. **[DERIVED]** $r<\tfrac12\,\underline{\mathrm{lfs}}_\sigma$ along the edge stratum — the ball touches no third face; else route to §17.4.
5. **[DERIVED]** BLD-CONTACT-001; else `BlendSpillover` / face disappearance.
6. **[DERIVED]** envelope free of $A_3$ (§14); else SI-TRIM-001.

**FIL-SPN-001** — constant radius, two faces, all six gates: the only fully constructive two-face blend cell. Its contact with each support is a **tangential contact along a curve**, so it is a consumer of SS-TAN-CRV-001 and not of the isolated-tangency cells — the single most common instance of §9.2.2 in the whole system.

**FIL-VAR-001 — variable radius.** The earlier gate $|r'(\alpha)|<\tan(\text{contact angle})$ is withdrawn; it was not the right object. The carrier is a **canal surface with variable radius**, for which the classical condition is stated with respect to spine arc length $s$:
- **[DERIVED]** the characteristic circle is real, hence the envelope is a real surface, iff $|dr/ds|<1$;
- **[DERIVED]** local regularity as in SWP-REG-001 with the $r'$ term included;
- **[DERIVED]** global embedding $r_{\max}<\underline\rho(\Sigma)$;
- **[PROVISIONAL SUFFICIENT GATE]** the composite: the *spine itself* now depends on $r(s)$ (it is the intersection of two **variable** offsets), so $\Sigma$ is not fixed while $r$ varies, and the coupled discriminant has not been derived. Until it is, FIL-VAR-001 may emit at most `CertifiedEquivalentConstruction`.

### 17.3 Chamfer

Absent from revisions through r3, which is a coverage hole rather than an oversight of difficulty: chamfer is about as common as fillet in mechanical parts, and it is **strictly easier**, for one structural reason worth stating up front.

> A chamfer meets its supports at a $G^0$ edge with certified nonzero dihedral. It therefore needs **no tangency-preserving representation** (REP-G1-001), no tangential-contact cell at its supports, and no $G^1$ corner compatibility. It is a transverse construction throughout. Chamfer should land before BLD-CNR-SETBACK-001, not after it.

**Two conventions, and the choice is [POLICY].** They give different geometry and must be recorded in provenance (§20), because a convention that silently re-decides on regeneration is a geometry change wearing a naming bug's clothes:

- **Section convention (CHM-RULED).** In the normal plane $N(s)$ of the edge, $\gamma_i(s)$ is the point of $f_i\cap N(s)$ at arc length $d_i$ from $c(s)$. The carrier is the ruled surface between $\gamma_1$ and $\gamma_2$. This matches the usual "distance from the edge measured on the face" interpretation, and the carrier is **not** developable in general.
- **Envelope convention (CHM-DEV).** The chamfer plane $P(s)$ is the plane at distance $d_1$ from $f_1$ and $d_2$ from $f_2$ on the material side, i.e. tangent to both shrunken offsets $O_{-d_1}(f_1)$, $O_{-d_2}(f_2)$. The carrier is the envelope of $\{P(s)\}$ and is therefore **developable by construction**, ruled by the characteristics $P(s)\cap P'(s)$. Analytic supports give exact members of $\mathcal{G}$ far more often — plane, cylinder, cone, tangent developable — which is the practical argument for this convention.

**CHM-RULED-001 — distance-distance chamfer, section convention.**
*Pre:* BLD-SUP-001 including $|\psi-\pi|\ge\delta_\psi$; edge regular, $|c'|\ge\delta$.
*Gates:*
- **[DERIVED]** $\gamma_i$ regular: $d_i\,\kappa_i^{\text{sec}}<1$, where $\kappa_i^{\text{sec}}$ is the normal-section curvature of $f_i$ — the exact analogue of fillet gate F-1, and the same focal mechanism.
- **[DERIVED]** BLD-CONTACT-001 (spillover, domain containment).
- **[DERIVED]** global embedding by the chord margin (BLD-EMB-001). The sections are not in parallel planes, so LFT-RULED-001's iff does not transfer and the chord margin is sufficient-only; beyond it, §10.
- **[DERIVED]** carrier in $\mathcal{G}$ exactly when both supports are planar (a plane), or plane-against-cylinder/cone in the coaxial and parallel positions (a cone or cylinder); otherwise through $\mathrm{rep}$ with §6.2 conditions **(i)–(iv)** and no $G^1$ requirement.
*Modulus:* $\omega(\varepsilon)=\varepsilon/|1-d_i\kappa_i^{\text{sec}}|$ — the same focal factor as §16, since $\gamma_i$ is an offset-type locus.
*Refusals:* `ChamferDistanceExceedsCurvature`, `EdgeAlreadySmooth`, `BlendSpillover`.

**CHM-DEV-001 — envelope convention.** Same gates plus one that has no fillet analogue: the developable's **edge of regression** (the $A_2$ stratum of §14) must not enter the trimmed chamfer region. Gate **[DERIVED]**: certified distance from the chamfer region to the cuspidal edge $\ge$ margin. Violation is a cuspidal chamfer on a strongly curved edge — a real and commonly-hit failure, and one that ENV-A2-001 already knows how to emit as a real edge if the semantics call for it.

**CHM-ANG-001 — distance-angle.** One distance plus an angle to a reference face; the second contact curve is determined by the angle rather than a distance. Same machinery; the gate on the *derived* distance must be checked, since a fixed angle on a curved edge produces a varying second distance which can exceed its curvature bound partway along.

**CHM-VAR-001 — variable distance.** **[PROVISIONAL SUFFICIENT GATE]** $|d_i'|<1$ along the edge plus the constant-distance gates at $d_{\max}$; the coupled discriminant is not derived, exactly as in FIL-VAR-001.

**CHM-FIL-001 — chamfer meeting fillet.** Adjacent edges with different blend types: mutual trim by BLD-FIL-001. The trim curve is $G^0$ on the chamfer side and $G^1$ on the fillet side, so the resulting edge is a genuine sharp edge and must not be suppressed by the $G^1$ edge-merging policy of EXT-PROF-SMOOTH-001.

### 17.4 Corners

The r3 text ("setback plus an $n$-sided patch, the hardest single cell") is true of the general case and **badly wrong as a default**, because the commonest corner in mechanical parts has a closed-form solution in $\mathcal{G}$ and needs no patch at all. Classify first.

**BLD-CNR-BALL-001 — spherical corner. [DERIVED]** *Three edges, one convex vertex, equal constant radius $r$.* The ball that reaches the vertex has centre at the point $c^\*$ equidistant $r$ from all three supports on the material side:
$$c^\*\in O_{-r}(f_1)\cap O_{-r}(f_2)\cap O_{-r}(f_3),$$
which is exactly the three-offset vertex of §16.2, with the same gate $|\det[\hat n_1\,\hat n_2\,\hat n_3]|\ge\delta$. The corner patch is then the region of the sphere $S(c^\*,r)$ bounded by the three **characteristic circles** at which the three canal surfaces terminate — a spherical triangle bounded by three circular arcs.

Two consequences worth being explicit about, because they overturn the r3 framing:
- the patch is a **sphere**, hence exactly in $\mathcal{G}$: no $n$-sided patch, no $\mathrm{rep}$, no approximation error;
- $G^1$ to all three fillets is **automatic**, not a fitted constraint: sphere and canal surface share the same ball at the terminal spine parameter, so they share the tangent plane along the whole characteristic circle. REP-G1-001 is not invoked.

*Additional gates:* $r<\tfrac12\,\underline{\mathrm{lfs}}_\sigma$ at the **vertex** stratum (no fourth face — the star-separation term of §6.1 is what this reads); and the three arcs bound a **simple** spherical triangle, certified by running the §11 arrangement engine on $S^2$ (circular arcs on a sphere — the same DCEL machinery with the periodicity contract $\Lambda$ of the sphere). Concave vertex: identical with the offsets on the other side, patch on the interior. Refusals: `NoOffsetVertex`, `CornerArcsNotSimple`.

**BLD-CNR-TRIM-001 — mutual trim, no patch. [DERIVED]** *Three edges, unequal radii or mixed blend types.* No common ball point exists, but the three blend carriers may still meet pairwise in three curves that share a common triple point with margin. If so the corner is a pure trim: no new face. Gate: certified triple point of the three carriers, transversality $\delta$ pairwise. This covers most fillet/chamfer mixtures at a three-face vertex.

**BLD-CNR-SETBACK-001 — $n$-sided patch. [PROVISIONAL SUFFICIENT GATE]** Required when $k\ge4$, when the vertex is a **saddle** (dihedral signs mixed around the star, mirroring OFF-VERTEX-SADDLE-001), or when neither of the above certifies. Set each blend end back by $s_i$ (**[POLICY]**), collect the closed loop of $n$ alternating boundary curves — blend ends and strips of the supports — and fill.

*The theory gap is specific and should be stated rather than gestured at.* $G^1$ filling around a **closed** loop is obstructed. At each corner of the loop the cross-boundary tangent fields must satisfy a **twist compatibility (vertex enclosure) condition**; the linear system relating them is generically solvable for odd $n$ and **singular for even $n$**, so a four-sided corner — the $k=4$ case, i.e. the one that sends you here in the first place — is exactly the obstructed one. The admissible responses, and the choice is **[POLICY]** recorded in provenance:
1. adjust the cross-boundary tangent fields within a certified tolerance until the compatibility system is consistent, and report the adjustment as part of $\tau_{\text{rep}}$;
2. split the $n$-gon into $n$ quadrilateral subpatches meeting at a central point (the standard fix; moves the obstruction to the interior vertex where it is solvable);
3. refuse.

*Certification obligations that do not disappear once a patch exists:*
- the patch is not in $\mathcal{G}$, so it goes through $\mathrm{rep}$ with **REP-G1-001** — positional error is not enough, since the whole point of the patch is its normal field;
- §6.2 including **(iv)**: a badly conditioned fill folds, and a folded patch satisfies every positional bound while being multiply-sheeted over its own domain. Gate the immersion directly, $\lVert S_u\times S_v\rVert\ge\iota$, and discharge (iv-b) on the patch's own partition;
- the patch must lie inside the region vacated by the setback and touch no support outside its loop.

*Refusals:* `CornerLoopNotG1Compatible`, `CornerPatchFolds`, `SetbackExceedsEdgeLength`.

**BLD-CNR-CASCADE-001.** The setback runs past a neighbouring vertex, or gate F-4 fails so the primitive touches a fourth face: two corners interact and must be solved jointly. `Unsupported` beyond a declared $k$ and a declared setback-overlap threshold. This is the honest boundary of the corner atlas, and it is where a production kernel's fillet failures actually cluster.

**Chamfer corners** run through the same cells with one simplification worth naming: for planar supports the corner patch is a **planar polygon** (a triangle for $k=3$), exactly in $\mathcal{G}$, and the $G^1$ obstruction does not arise at all because the chamfer corner is $G^0$ by construction. Chamfering a box corner is fully constructive; filleting one is BLD-CNR-BALL-001; filleting a four-face corner is the provisional cell.

---

## 18. Composition, conditioning, and margins

**Two independent requirements, propagated separately.** Topological stability does **not** imply metric conditioning: a construction can preserve exactly the same combinatorics while amplifying perturbations geometrically.

$$\textbf{metric:}\quad \varepsilon_{i+1}\ \le\ \omega_i(\varepsilon_i)+\tau_{\text{rep},i} \qquad\qquad \textbf{topological:}\quad \varepsilon_i<\mathfrak{m}_i$$

$\mathfrak{m}_i$ is the certified lower bound on the input perturbation that would change the combinatorial output — the minimum over all transversality margins, separations and clearances used to decide the result.

$\omega_i$ is a certified **modulus of continuity**, not a scalar. A scalar $\kappa_i$ is adequate only where the operation is locally Lipschitz; at tangency it is not, and forcing a linear bound there is unsound.

**`Modulus` contract.** A cell publishes $\omega_i$ together with its domain of validity and an explicit declaration of which properties are certified:

- **(M1)** $\omega_i(0)=0$ and $\omega_i$ continuous at $0$;
- **(M2)** $\omega_i$ nondecreasing;
- **(M3)** the bound holds only on $[0,\mathfrak{m}_i)$ — outside the topological stability cell the operation is not continuous and no modulus exists;
- **(M4)** $\omega_i$ subadditive there: $\omega_i(a+b)\le\omega_i(a)+\omega_i(b)$ whenever $a+b<\mathfrak{m}_i$.

(M1)–(M3) are mandatory. **(M4) is optional and must be certified, not assumed.** Concavity with $\omega(0)=0$ implies it, so every linear $\varepsilon/c$ and every $k\varepsilon^p$ with $0<p\le1$ qualifies; a modulus that blows up at the margin, e.g. $\omega(\varepsilon)=\varepsilon/(\mathfrak{m}-\varepsilon)$ — the natural shape for a near-degenerate cell — is convex and does not. A cell may additionally publish the least concave majorant $\hat\omega_i\ge\omega_i$ on any compact $[0,m']\subset[0,\mathfrak{m}_i)$ where $\omega_i$ is bounded; $\hat\omega_i(0)=0$ since $0$ is an extreme point of the interval, and $\hat\omega_i$ satisfies (M4) by construction, so (M4) is always *available* at the price of declared pessimism.

| situation | modulus $\omega(\varepsilon)$ | source |
|---|---|---|
| transverse SSI | $\varepsilon/\sin\theta$ | §9.1 |
| tangential contact (order 2) | $\sqrt{2\varepsilon/|\lambda_{\min}(\operatorname{Hess}g)|}$ | §9.2 — **Hölder-$\tfrac12$, not Lipschitz** |
| three-surface vertex | $\varepsilon/|\det[\hat n_1\,\hat n_2\,\hat n_3]|$ | §16.2 |
| offset at distance $d$ | $\varepsilon/|1-d\kappa_{\max}|$ | §16 focal factor |
| fillet spine | $\varepsilon/\sin\theta_{\text{offset}}$ | §17.2 gate F-2 |
| isolated contact of order $k$ | $C\varepsilon^{1/(k+1)}$ | §9.2.1 — concave, so (M4) holds for every $k$ |
| tangency curve, transverse | $\sqrt{2\varepsilon/|\lambda|}$ | §9.2.2, $\lambda$ the nonzero Hessian eigenvalue |
| chamfer contact curve | $\varepsilon/|1-d\kappa^{\text{sec}}|$ | §17.3 — offset-type focal factor |
| spherical corner centre | $\varepsilon/|\det[\hat n_1\,\hat n_2\,\hat n_3]|$ | §17.4 — the three-surface vertex row, reused |
| extrude / revolve | $\varepsilon$, resp. $\varepsilon\max(1,R_{\max}/\mathrm{dist})$ | §15.1–15.2 |

> **Composition theorem.** Suppose at every step $\varepsilon_i<\mathfrak{m}_i$ **and** $\varepsilon_{i+1}\le\omega_i(\varepsilon_i)+\tau_{\text{rep},i}$. Then the combinatorial result of the chain equals the exact result on $\tilde B_{\text{in}}$, and — under (M1)–(M3) alone — the geometric error obeys the **nested recurrence**
> $$\varepsilon_n\ \le\ \omega_{n-1}\big(\cdots\omega_1\big(\omega_0(\varepsilon_0)+\tau_{\text{rep},0}\big)+\tau_{\text{rep},1}\cdots\big)+\tau_{\text{rep},n-1}.$$
> This is the fundamental bound.
>
> **Corollary (split form).** If in addition every $\omega_i$ satisfies (M4), then
> $$\varepsilon_n\ \le\ (\omega_{n-1}\circ\cdots\circ\omega_0)(\varepsilon_0)+\sum_i(\omega_{n-1}\circ\cdots\circ\omega_{i+1})(\tau_{\text{rep},i}).$$
>
> Failure of the topological condition at any step returns `CompositionMarginExhausted`; failure of the metric condition (the recurrence exceeding the declared forward tolerance) returns `ForwardToleranceExceeded`.

**Why the split form is a corollary and not the statement.** Separating the propagated initial error from the separately propagated representation errors *is* the inequality $\omega(a+b)\le\omega(a)+\omega(b)$, applied once per level. For Lipschitz moduli that is free and the distinction is invisible — which is how the split form comes to be written down as though it were the theorem. But the substance of §9.2 is that tangential contact is Hölder-$\tfrac12$, so this system contains nonlinear moduli by design and the question is live rather than pedantic. Hölder-$\tfrac12$ is concave and therefore does satisfy (M4), and in fact every entry in the table above satisfies it — but that is a property of the current table, not of the theorem, and any new cell must certify (M4) before its contribution may be split out.

Implementations should evaluate the nested recurrence directly with outward rounding: it is a scalar forward pass of length $n$, cheaper than the split form and valid unconditionally. Two consequences worth stating: the recurrence is **not order-symmetric** — a Hölder-$\tfrac12$ step amplifies everything upstream of it and nothing downstream, so composition order is part of the conditioning claim, and reordering commuting operations can change the certified forward bound by orders of magnitude; and the recurrence must be evaluated *stepwise against $\mathfrak{m}_i$*, since a bound computed only at the end cannot tell which step left the stability cell.

Note the dependency direction: $\omega_i$ is a valid bound only *within* the topological stability cell, since outside it the operation is not continuous at all. So the topological condition gates the metric one, and neither implies the other.

---

## 19. Input validation and repair

**Validation atlas**, each check with an interval certificate: entity tolerance vs. stratified local feature size; same-parameter and same-range per coedge; wire closure and parametric non-self-intersection; seam and pole conformance to $\Lambda$; shell closure by coedge pairing; orientation consistency; inner-shell nesting; degenerate-edge conformance; dihedral non-degeneracy (invariant 9).

**Repair atlas**, each a bounded backward perturbation reported into $\tau_{\text{in}}$: `REP-TOL-001` re-tightening by re-projection; `REP-PCV-001` pcurve recomputation from the 3D curve (highest yield on real STEP); `REP-GAP-001` cluster quotient (reuses §5); `REP-ORI-001` orientation repair by shell walk; `REP-SEAM-001` seam reconstruction.

Unrepairable within budget $\Rightarrow$ `InputOutsideBackwardBudget`.

---

## 20. Identity and regeneration

$$\mathrm{id}::=\mathrm{src}(k)\mid\mathrm{op}_j(\mathrm{id}_1,\dots,\mathrm{id}_n)\mid\mathrm{sel}(\mathrm{id},\pi)$$
with $\pi$ a **construction-derived** selector, never a geometric query on the realised model. Every **[POLICY]** decision in §15–17 (corner treatment, correspondence, vertex-suppression threshold) is part of $\pi$ and must be recorded, since a policy that silently re-decides on regeneration is a naming failure regardless of the geometry being correct.

**Regeneration closure.** A *total* map from old ids to $\{\texttt{Preserved},\texttt{Split}(n),\texttt{Merged}(m),\texttt{Vanished},\texttt{Ambiguous}\}$, each with a certificate. Downstream references consuming non-`Preserved` results declare a resolution policy or fail with `ReferenceUnresolved`. Totality is OB-5; correctness of any heuristic is explicitly not claimed.

---

## 21. Verification

**OCCT is a differential oracle, not a truth oracle.** Treating it as ground truth would contradict the epistemic philosophy of this entire document — a second implementation is evidence, not proof. Three-way classification of every corpus instance:

| relation | epistemic status |
|---|---|
| result $=$ OCCT (up to isotopy and tolerance) | **corroborating evidence** — raises confidence, proves nothing; both may share convention or algorithmic lineage |
| result $\ne$ OCCT | **discrepancy requiring adjudication** — either side may be wrong, or it may be a convention difference |
| certified / formal / metamorphic test passes | **correctness evidence** |

**Adjudication protocol.** On discrepancy, re-run the instance in the exact tier (§7) at high budget and check the metamorphic invariants; classify the outcome as `ours-wrong | occt-wrong | convention-difference | both-valid-within-tolerance` and log it. Comparison is always up to isotopy and tolerance, never literal entity match — seam placement, fragment ordering and face splitting are conventions.

**Cost management.** OCCT's expense is handled by making it an *offline labelling* pass: run once over the corpus to produce cached comparanda, per-cell prevalence weights and witness extractions; then test against the cache at speed.

**Metamorphic invariants** (no reference implementation required): $A\cup^*A=A$; $A\setminus^*A=\emptyset$; associativity and commutativity up to isotopy; De Morgan within a bounding box; $(A\cup^*B)\setminus^*B\subseteq A$; $\mathrm{vol}(A)+\mathrm{vol}(B)=\mathrm{vol}(A\cup^*B)+\mathrm{vol}(A\cap^*B)$ within accumulated $\varepsilon$; invariance under rigid motion, uniform scale, knot insertion, degree elevation and reparameterisation; link and Euler tests on every result; round-trip through the tessellation atlas.

**Margin sweeps.** Per cell, sweep $\delta$ logarithmically toward zero and assert the outcome degrades monotonically `ProvenConstruction` $\to$ `NumericallyUnresolved`, never silently to a wrong answer. This is the directly testable statement of epistemic closure and the one test no differential oracle can serve.

---

## 22. Closure theorems

**Theorem (Epistemic Closure).** Under OB-1, OB-2, and totality with mutual exclusivity of the outcome classifier, every $x\in E$ terminates within its ledger and returns a well-formed $\mathcal{E}$ with exactly one terminal outcome. No path ends unclassified; no path returns a construction whose preconditions were not established.
*Proof.* Induction over the dispatch DAG with lexicographic measure $(\text{depth},\beta)$ — budget decrease covers refinement loops, which re-enter the tree and are not covered by depth alone. OB-1 gives progress at each node, OB-2 termination at each leaf. $\square$

**Theorem (Constructive Closure).** Let $E_c$ be the instances satisfying the quantitative preconditions of cells that are complete (OB-3), fidelity-certified (OB-4) and conditioning-certified (OB-6), using only **[DERIVED]** gates. Then every $x\in E_c$ yields `ProvenConstruction`, `CertifiedEquivalentConstruction` or `CertifiedGeometricCollapse`, satisfying §0.1.
*Proof.* Per cell: OB-3 bounds subdivision depth; OB-4 transfers exact-object certificates to the emitted approximant via §6.2/6.3, glued across strata by OB-7; §5 preserves invariants or replaces them with certified weaker ones; §4 accumulation is monotone. Compose along the DAG by §18, discharging both the topological and metric conditions. Instances relying on any **[PROVISIONAL SUFFICIENT GATE]** are excluded from $E_c$ and cap at `CertifiedEquivalentConstruction`. $\square$

---

## 23. Dependency DAG

```
 0  representation operator + stratified reach/isotopy engine  (§6)   ← root
 1  certified clustering / quotient                            (§5)
 2  interval/Krawczyk kernel + budget ledger                   (§7)
 3  curve–curve atlas                                          (§9)
 4  curve–surface atlas                                        (§9)
 5  surface–surface atlas: transverse | tangential | coincident | chart
 6  self-intersection engine (diagonal deflation)              (§10)
 7  arrangement engine + periodicity contract                  (§11, §8)
 8  membership propagation                                     (§12)
 9  link / Euler / nesting tests                               (§1.1)
10  Boolean reconstruction + material-state ON atlas           (§13)
11  input validation & repair                                  (§19)  [parallel]
12  envelope / discriminant engine                             (§14)
13  extrude (REG → PROF → DRAFT → UPTO)                        (§15.1)
14  revolve (REG → FULL → POLE → AXIS-*)                       (§15.2)
15  sweep (FRM-* → REG → EMB → SPINE-DEG)                      (§15.3)
16  loft (CORR → RULED → POLE → CONT)                          (§15.4)
17  offset carriers → edge/vertex treatment → topology events  (§16)
18  shell (SEL → OFF → CAP → THICK)                            (§16.4)
19  blends: substrate (BLD-SUP/CONTACT/EMB/END/FIL, REP-G1)     (§17.1)
19a   fillet (SPN → VAR)                                       (§17.2)
19b   chamfer (RULED | DEV → ANG → VAR)        ── parallel ──   (§17.3)
19c   corners (BALL → TRIM → SETBACK → CASCADE)                 (§17.4)
20  composition, conditioning, margins                         (§18)
21  identity / regeneration                                    (§20)
22  verification harness (adjudication + metamorphic + sweeps) (§21)
```

Ordering points that matter: the stratified fidelity engine is a **root**; self-intersection precedes every generative operation; extrude-with-draft depends on the *2D* offset atlas, so draft should be sequenced after §16 rather than treated as an extrude variant; and LFT-CORR-001 precedes LFT-RULED-001, since the ruled cell's gate is a property of the correspondence, not of the individual rulings.

Three orderings added in r4, all of which cut against the intuitive sequence:

- **SS-TAN-CRV-001 is a dependency of every blend**, not a refinement of the transverse case. A fillet's contact with its supports is a tangential contact along a curve, so FIL-SPN-001 cannot be certified before §9.2.2 exists. Sequencing the blends before the tangency-curve cell produces a fillet that works and cannot say why.
- **TAN-SNAP-001 runs before the numeric tangential cells**, not after them as a fallback. A successful snap replaces an expensive numeric instance with an exact symbolic one, so it belongs at the *entry* to the atlas; running it only after subdivision has exhausted its budget pays the full cost and then discards the result.
- **Chamfer (§17.3) is parallel to fillet (§17.2) and independent of §17.4's hard cases.** It needs the shared substrate and the transverse intersection engine, and nothing else — no tangential cell at its supports, no REP-G1-001, no $G^1$ corner compatibility. It is therefore the cheapest real capability in the whole generative half and should not be scheduled behind fillet's corner problem.

---

## 24. Coverage assessment

| Layer | r1 | r2 | r3 | r4 |
|---|---|---|---|---|
| Semantics (backward error, composition, conditioning) | 85% | 92% | 94% | 94% |
| Data model & invariants | 90% | 92% | 92% | 92% |
| Tolerance, collapse, stratified reach | 85% | 90% | 90% | 90% |
| Evidence & dispatch | 80% | 82% | 82% | 82% |
| Fidelity / representation | 70% | 80% | 76% | 78% |
| Intersection atlas (all arities) | 45% | 48% | 48% | 66% |
| Arrangement, periodicity, membership | 75% | 82% | 82% | 85% |
| Boolean reconstruction (incl. ON atlas) | 75% | 88% | 88% | 88% |
| Self-intersection & envelopes | 55% | 58% | 58% | 60% |
| Extrude / revolve | 20% | 65% | 65% | 65% |
| Sweep / loft | 20% | 50% | 54% | 54% |
| Offset / shell | 30% | 65% | 66% | 66% |
| Blends: fillet / chamfer / corner | 30% | 40% | 40% | 62% |
| Validation, repair, identity, verification | 60% | 70% | 70% | 72% |

**Overall: ≈78% as a specification; ≈42% as an implementation-ready cell inventory.**

The fidelity row **decreased** in r3. That was the honest bookkeeping: §6.2(iv) is an obligation REP-CRV-001 and REP-SRF-001 must discharge and previously did not, so r2's 80% was measuring a lemma that did not hold as stated. Sweep/loft rose because LFT-RULED-001 gained a correct criterion with a certifiable margin, at the cost of a subdivision it did not previously pay. Semantics rose because the `Modulus` contract turned an implicit assumption into a declared, checkable property. No r3 row rose because a cell was added; **r3 was entirely repair.**

**r4 is the opposite: entirely addition.** Three items, and the two large jumps are worth reading carefully rather than as progress.

*Intersection atlas 48% → 66%.* The gain is not from the exotic singularities. It is from recognising that the dominant tangential configuration has a **one-dimensional** contact locus, which no cell in r3 addressed — and that this case is solved by tracing a ridge curve of $g$ with a transverse Jacobian, i.e. by a *regular* construction with no singular solving at all. The polar blow-up then subsumes r3's three isolated-tangency cells and everything above them in the $A_k/D_k/E_k$ hierarchy into one gate on the discriminant of the leading form. What remains open is genuinely narrower: Newton-degenerate leading forms, and the joint consistency of TAN-SNAP-001.

*Blends 40% → 62%.* Chamfer is new and lands nearly complete, because it is a transverse $G^0$ construction and inherits almost nothing hard. The corner number moved because the **common** corner turned out to be constructive: a three-face convex vertex at uniform radius is a spherical triangle in $\mathcal{G}$ with automatic $G^1$, reducing to the three-offset vertex gate that §16.2 already had. r3's "the corner patch is not a member of $\mathcal{G}$ under any natural construction" is true only of the cases that reach BLD-CNR-SETBACK-001, and stating it as the general case badly mis-priced the item.

What did not move, and is now the honest residual: FIL-VAR-001 and CHM-VAR-001 share one underived coupled discriminant (the spine or contact curve depends on the varying parameter). BLD-CNR-SETBACK-001 remains provisional, and its obstruction is now **named** rather than gestured at — $G^1$ filling around a closed loop is singular for even $n$, which is precisely the $k=4$ corner that sends you there. Loft correspondence still has no general well-posedness theorem off the parallel-plane case. And TAN-SNAP-001's joint-consistency gate is provisional despite being, by prevalence, the most valuable cell in the document.

Critical path, **reordered in r4**:
1. **§6 stratified fidelity** — every other certificate concerns an object the kernel does not emit, and the stratification is what makes it applicable to parts with sharp edges, i.e. all of them.
2. **§9.2.2 tangency along a curve, and §9.2.3 snapping** — promoted above the isolated-tangency cells. These two are what fillet supports, coaxial features, resting contacts and float-round-tripped coplanar faces actually need, and neither existed before r4. The isolated $A_{k\ge3}$ cells are specified but are not on the critical path.
3. **§9.2.1 polar blow-up** — needed because the transition points of §9.2.2 must land somewhere, not because isolated high-order contact is common.
4. **§10 self-intersection** — without it no generative operation is trustworthy beyond convex profiles.
5. **§17.3 chamfer** — out of order relative to its difficulty, because it is the cheapest unclaimed capability in the document and is blocked by nothing above item 1.
