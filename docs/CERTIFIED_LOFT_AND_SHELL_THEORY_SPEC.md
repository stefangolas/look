# Certified Construction Theory

**Status:** theory spec, unified. Supersedes and replaces the previous
"Certified Loft and Shell — Theory Specification" (v2) in this file; that v2
content is deleted, and no amendment document accompanies this one. The
codebase audit and packet plan converting this into an executable build spec
live in `CERTIFIED_CONSTRUCTION_BUILD_SPEC.md` (CC program). See also
`CONSTRUCTIVE_GEOMETRY_PLAN.md` for the current CG program and
`CERTIFICATE_MAPPING.md` for the shared evidence booking surface.

**Revision:** v1, unified. Loft · Offset/Shell · Blend.

---

**Loft · Offset/Shell · Blend — unified specification, v1**

---

## 0. Scope and architecture

This document specifies the mathematics required to certify three B-rep
construction families:

* **Loft** — a tensor-product surface interpolating $n+1$ compatible section
  curves;
* **Offset / Shell** — the thickened boundary of a solid at a requested
  signed distance $t$;
* **Blend** — rolling-ball fillets and rounds, at constant or prescribed
  variable radius, including chains, runout, face consumption, and $n$-valent
  corners.

The organizing claim is that these are not three theories.

> Loft is a certified **linear** construction with a topological postcondition.
> Offset and blend are the **same** contact-stratum theory under two different
> radius laws: $\Phi(c,r)=r-t$ for the offset, $\Phi(c,r)=r-R(\lambda)$ for the
> blend. Their certification obligations reduce to six shared primitives.

Everything below is either a construction rule (deterministic, untrusted) or a
certificate (interval-verifiable, trusted). No result depends on a heuristic
search, a fairness objective, a tolerance-driven fit, or an unverified Newton
iteration. Where a certificate can fail, the specified outcome is a typed
refusal (§9), never a silent approximation.

**Notation.** $S:D\subset\mathbb R^2\to\mathbb R^3$ is an oriented $C^2$ face
with unit normal $n$; $\sigma_{\min}(M)$ is the smallest singular value of $M$;
for a non-square $M\in\mathbb R^{a\times b}$ with $a<b$ the surjectivity margin
is $\sigma_a(M)$, equivalently $MM^{\mathsf T}\succeq\sigma_a^2 I$. Interval
enclosures are written $[\,\cdot\,]$. All floating computation is assumed
IEEE-754 with a normative evaluation order, so that every result is
bit-reproducible; *determinism*, not exactness, is what reproducibility
requires.

---

## 1. Shared primitives

### P1 — Certified banded solve

Let $Ax=b$ with $A$ banded and totally positive. All B-spline collocation
matrices satisfying the Schoenberg–Whitney conditions are of this class.

**Fast path.** Interval Gaussian elimination without pivoting. Nonsingularity is
certified if no interval pivot contains $0$; the enclosure follows from interval
back-substitution. Cost $O(nq^2)$ to factor, $O(nq)$ per right-hand side.

The stability justification is specific to this matrix class: for totally
positive matrices, Gaussian elimination without row exchanges has growth factor
exactly $1$ (de Boor–Pinkus). Interval widths are therefore governed by rounding
alone and do not amplify across the elimination, which is why the no-pivoting
interval factorization is safe here and would not be in general.

**General fallback.** For systems outside the banded-TP class — Hermite ribbon
construction, radius-law splines, degree-elevation systems — use the
Rump/Ogita/Oishi residual certificate. Let $\widehat x$ be a floating proposal,
$R\approx A^{-1}$, and suppose interval arithmetic proves
$\eta=\lVert I-RA\rVert_\infty<1$. Let $\mathbf r\ni b-A\widehat x$. Then $A$ is
nonsingular and

$$\lVert x-\widehat x\rVert_\infty\;\le\;\frac{\lVert R\mathbf r\rVert_\infty}{1-\eta}.$$

*Proof.* $RA=I-E$ with $\lVert E\rVert_\infty=\eta<1$, so $RA$ and hence $A$ is
invertible; $RA(x-\widehat x)=R\mathbf r$ gives
$x-\widehat x=(I-E)^{-1}R\mathbf r$, and
$\lVert(I-E)^{-1}\rVert\le(1-\eta)^{-1}$. $\square$

Cost $O(n^2)$, since $R$ is dense. This path is never used where the fast path
applies.

**Exact path.** When the inputs are rational and $A$ is rational, exact rational
banded LU returns the exact solution. For the uniform cubic system the
denominators grow like $(2+\sqrt3)^{\,n}$, so the bit-work is $O(mn^2)$; enable
this path below a threshold $n\le n_{\text{exact}}$ and use the interval fast
path above it.

**Consumers.** Loft rows; Hermite ribbons for setback patches; radius-law
construction; basis-compatibility systems.

### P2 — Local injectivity radius

This is the primitive that makes self-intersection testing terminate. Without a
computable injectivity radius, a subdivision contact solver asked to certify
$S(p)\ne S(q)$ for $p\ne q$ subdivides indefinitely as $q\to p$.

**Lemma (P2).** Let $B\subset\mathbb R^2$ be a convex parameter box, $S\in
C^2(B)$, and set
$$\sigma:=\inf_B\sigma_{\min}(DS)>0,\qquad L:=\sup_B\lVert D^2S\rVert .$$
Then for all $p\ne q\in B$,
$$\lVert S(q)-S(p)\rVert\;\ge\;\Bigl(\sigma-\tfrac{L}{2}\lVert q-p\rVert\Bigr)\lVert q-p\rVert .$$
Consequently $S$ is injective on $B$ whenever $\operatorname{diam}B<2\sigma/L$,
and
$$\delta:=2\sigma/L$$
is a certified injectivity radius: no contact test is required for parameter
pairs with $\lVert p-q\rVert<\delta$.

*Proof.* $S(q)-S(p)=DS(p)(q-p)+\int_0^1\bigl[DS(p+\tau(q-p))-DS(p)\bigr](q-p)\,d\tau$.
The first term has norm $\ge\sigma\lVert q-p\rVert$; the integral has norm
$\le\tfrac L2\lVert q-p\rVert^2$. $\square$

Both constants are interval-computable, and for a $3\times2$ Jacobian
$\sigma_{\min}=\lVert S_u\times S_v\rVert/\sigma_{\max}$, so $\sigma$ costs no
more than the regularity test already performed. The same lemma applies verbatim
in one dimension to certify that a planar or spatial curve does not
self-intersect near the diagonal.

**Consumers.** Diagonal exclusion in loft self-contact; near-diagonal exclusion
within a single offset stratum; simplicity of projected boundary curves in P3;
spine self-intersection in blends.

### P3 — Graph-disk embedding certificate

P2 certifies injectivity on a single smooth patch. P3 certifies it for an entire
glued region — a corner patch, or a closed star of the offset complex spanning
several strata — where no single parameterization exists.

**Theorem (P3).** Let $D$ be a closed disk and $P:D\to\mathbb R^3$ continuous and
piecewise $C^1$ on a finite fan of sub-patches. Suppose there is a unit vector
$w$, with $\pi$ the orthogonal projection onto $w^\perp$, such that

1. $\det D(\pi\circ P)\ge\eta_\pi>0$ on every sub-patch, and $\pi\circ P$ is a
   local homeomorphism across every internal seam;
2. $\pi\circ P|_{\partial D}$ is a simple closed curve.

Then $\pi\circ P$ is a homeomorphism of $D$ onto the closed Jordan domain
bounded by its boundary image, and $P$ is injective — an embedded regular disk.

*Proof.* Write $f=\pi\circ P$ and $J=f(\partial D)$, a Jordan curve. For
$y\notin J$ the degree $\deg(f,\mathring D,y)$ is defined and locally constant on
components of $\mathbb R^2\setminus J$. It is $0$ on the unbounded component and,
by the winding number of the boundary parameterization, $+1$ on the bounded one.
By (1), $f$ is an orientation-preserving local homeomorphism, so every preimage
of a regular value contributes $+1$ and $\#f^{-1}(y)=\deg(f,\mathring D,y)$.
Hence $y$ inside $J$ has exactly one preimage and $y$ outside has none. Openness
of $f$ excludes $f(\mathring D)\cap J$. So $f$ is a continuous bijection from the
compact $D$ onto the closed Jordan domain, hence a homeomorphism; injectivity of
$P$ follows from injectivity of $f$. $\square$

Hypothesis (2) is discharged by pairwise planar curve/curve intersection
exclusion together with P2 in the plane for the near-diagonal. Hypothesis (1)'s
seam clause is not implied by the per-piece determinant condition and must be
checked: two orientation-preserving local diffeomorphisms agreeing along an
embedded seam arc glue to a local homeomorphism, which is the condition to
certify.

**Projection search.** The candidate set for $w$ is normative for
reproducibility: the area-weighted average patch normal, then the principal
directions of the control net, then a fixed spherical code of fixed cardinality.
Exhaustion yields `NoAdmissibleProjection` and falls back to pairwise
patch/patch intersection with an inside/outside witness.

**Consumers.** Setback corner patches; closed stars of the offset complex;
post-arrangement certification that a trimmed face has not folded.

### P4 — Isolated root with separation margin

Krawczyk / interval-Newton existence-and-uniqueness on a box, together with the
derived **argmin-with-margin** operator: given candidates with certified
enclosures $[\lambda_i]$, return $i^\*$ only if
$$\sup[\lambda_{i^\*}]<\inf[\lambda_j]\quad\text{for all }j\ne i^\*;$$
otherwise refuse. The operator certifies *strict separation*, never intent.

**Consumers.** Focal and first-tangency events; the blend event vocabulary;
cyclic correspondence disambiguation in loft; the first critical thickness.

### P5 — Ball clearance

This is the admissibility predicate underlying both offset validity and blend
validity.

Let $\Omega$ be the solid with boundary $\partial\Omega$, let $A$ be an active
support set, and let $(c,r)$ be a contact configuration. Define
$$\mathrm{Clear}_A(c,r;\mu)\;:\iff\;
d\Bigl(c,\;\partial\Omega\setminus\bigcup_{i\in A}S_i\Bigr)\;\ge\;r+\mu,\qquad \mu>0 .$$
For a **round** (material-removing) configuration additionally require
$B(c,r)\cap\operatorname{int}\Omega=\varnothing$; for a **fillet**
(material-adding) configuration require $B(c,r)\subset\operatorname{int}\Omega$.
Both are decided by signed-distance interval queries against a single BVH.

$\mathrm{Clear}$ subsumes the obstruction event of the blend system, the
"no unintended contact" condition of the offset bridge theorem, and — at
$r=\lvert t\rvert$ — the classical Federer $r$-regularity condition under which
offsets are embedded. Offset validity and blend validity resemble each other
because they are the same statement about a ball rolling freely against the
boundary.

### P6 — Share by identity

**Rule.** Whenever two constructions are required to produce identical geometry,
the shared sub-problem is solved **once** and referenced by construction
identity. It is never recomputed in two places and reconciled by tolerance.

Two floating evaluations of the same split point in different call orders need
not agree bitwise; a tolerance comparison then converts an exact structural fact
into a numerical gamble. Under P6 the fact stays exact. This is the mechanism
behind exact loft seam cancellation (L3), and it is equally required for
triple-contact nodes shared by three blend branches, corner centres shared by
three offset edge strata, and split vertices shared by adjacent wire segments.

P6 is a representation-level obligation, not a coding convention: construction
identity must survive Booleans, transforms, and blends.

---

## 2. Loft

### 2.1 Construction

Make the sections $u$-compatible by degree elevation and knot refinement, so
that
$$C_k(u)=\sum_{i=0}^{m}N_{i,p}(u)\,Q_{ik},\qquad k=0,\dots,n,$$
with $Q_{ik}\in\mathbb R^4$ homogeneous for the rational case. Choose stations
$0=v_0<v_1<\dots<v_n=1$, a degree $q$, and the clamped knot vector obtained by
de Boor averaging,
$$\xi_{j+q}=\frac1q\sum_{r=j}^{j+q-1}v_r,$$
with endpoints repeated $q+1$ times. Set $A_{kj}=M_{j,q}(v_k)$ and solve, for
each $u$-control row $i$ independently,
$$AP_i=Q_i .$$
The result
$$S^h(u,v)=\sum_i\sum_j N_{i,p}(u)M_{j,q}(v)P_{ij}$$
is an ordinary tensor-product B-spline/NURBS surface. No new carrier type is
introduced.

**Stationing policy.** The default is deterministic chord-length with a
normative summation order; uniform $v_k=k/n$ is available as an option. Both are
bit-reproducible. Uniform stationing alone permits caching $A$, its
factorization, and its verification metadata keyed by $(n,q)$ only, since under
uniform stationing $A$ is independent of the loft geometry.

### 2.2 Theorems

**L0 (a priori nonsingularity).** For any strictly increasing stations, de Boor
averaging produces a knot vector satisfying the Schoenberg–Whitney conditions
$\xi_j<v_j<\xi_{j+q+1}$. Hence $A$ is nonsingular; moreover $A$ is banded and
totally positive.

Nonsingularity is therefore a theorem about the stationing policy, not a
property to be discovered numerically. The interval factorization of P1 delivers
the enclosure; it is not what establishes invertibility.

**L1 (exact section reproduction).** If $AP_i=Q_i$ for every $i$, then
$$S^h(u,v_k)=C_k^h(u)$$
identically in $u$, for every section $k$.

*Proof.* $S^h(u,v_k)=\sum_iN_i(u)\sum_jM_j(v_k)P_{ij}=\sum_iN_i(u)(AP_i)_k
=\sum_iN_i(u)Q_{ik}=C_k^h(u)$. $\square$

**L1r (rational case).** L1 holds in $\mathbb R^4$. To descend to $\mathbb R^3$
the weight field
$$W(u,v)=\sum_i\sum_jN_i(u)M_j(v)w_{ij}$$
must be certified positive on $[0,1]^2$. This is a genuine side condition:
B-spline collocation is not weight-preserving, and the inverse of a totally
positive matrix has a checkerboard sign pattern, so negative $w_{ij}$ can arise
from strictly positive input weights, placing a pole in the domain. Certify by
the control-net bound $\min_{ij}w_{ij}>0$ — sufficient and free — with
Bernstein/interval subdivision as fallback. Otherwise refuse
`NonPositiveWeightField`.

**L2 (delivered accuracy).** The construction is exact; the artifact is an
enclosure. With $\widehat P$ the delivered floating control net and $P$ the exact
solution, the partition-of-unity bound gives
$$\bigl\lVert\widetilde S(u,v_k)-C_k(u)\bigr\rVert_\infty
\;\le\;\varepsilon:=\max_{i,j}\lVert\widehat P_{ij}-P_{ij}\rVert_\infty,$$
with $\varepsilon$ supplied by P1. Certified interpolation is algebraic
construction — the target is the unique solution of exact interpolation
equations, with no least-squares objective, fairness term, or tolerance-driven
fit — and the evidence it emits is the pair (exact-solution identity L1,
enclosure width $\varepsilon$). Downstream predicates asking whether the loft
meets its wires consume $\varepsilon$; they may not assume exactness.

**L3 (exact seam cancellation).** Build a closed-wire loft as $r$ strips over
matched edges rather than one periodic surface. Suppose two adjacent strips use

1. clamped $u$-knot vectors, so that $N_i(1)=\delta_{im}$ and $N_i(0)=\delta_{i0}$;
2. identical $v$-stations, degree, and $v$-knot vector;
3. shared split vertex data $V_k$ referenced by construction identity (P6),
   not independently recomputed.

Then the two generated boundary curves are the same computation,
$v\mapsto\sum_jM_j(v)(A^{-1}V)_j$, and agree bitwise.

*Proof.* By (1) the shared boundary of each strip is its $u$-endpoint control
row, which by construction solves $AP=V$ with the same $A$ by (2) and the same
$V$ by (3). $\square$

This is the only exactness available in the loft pipeline, and hypothesis (3) is
the one that carries it. Under recomputation the two evaluations are equal in
exact arithmetic but not necessarily in floating arithmetic, and the guarantee
degrades to a tolerance comparison.

**L4 (correspondence).** Require an abstract oriented cyclic complex $W$ and an
orientation-preserving combinatorial isomorphism $\phi_k:W\to W_k$ for each
section. Edge splitting is an exact combinatorial operation, so correspondence
consists of an orientation, an anchor, and an edge matching.

Resolution proceeds in a fixed order:

1. caller-supplied anchor;
2. a unique isomorphism forced by the combinatorial data;
3. P4 separation-margin argmin over the $r$ cyclic shifts under a declared
   geometric functional;
4. `Refuse(AmbiguousCorrespondence)`.

Step 3 is required for usability, not convenience: for a closed wire of $r$
matched edges the combinatorial automorphism group is $\mathbb Z_r$ (times
orientation), so step 2 essentially never succeeds — two circles represented as
four arcs are always four-fold ambiguous. Step 3 does not claim to recover
designer intent; it certifies that one shift is strictly separated from all
others under the declared functional, and refuses when the enclosures overlap.
Twist minimization is not an objective anywhere in the specification.

**L5 (validity postcondition).** Once correspondence is an orientation-preserving
combinatorial homeomorphism, the loft can fail geometrically in exactly two
ways: loss of regularity, $S_u\times S_v=0$; or self-contact, $S(p)=S(q)$ with
$p\ne q$. The certified condition is therefore

$$\text{regularity } \lVert S_u\times S_v\rVert\ge\eta_J>0
\;\;+\;\;\text{absence of off-diagonal self-contact}.$$

Near-diagonal pairs are discharged by P2, far pairs by BVH broad phase followed
by interval contact solves, and whole patches by P3 where an admissible
projection exists. There is no separate pinch or twist theory.

### 2.3 Complexity

With $m+1$ compatible $u$-control points, $n+1$ sections, and degree $q$: the
banded factorization costs $O(nq^2)$ and all homogeneous right-hand sides cost
$O(mnq)$, so for cubic $q=3$

$$T_{\text{loft}}=O(mn),$$

against an output of $\Theta(mn)$ control values — asymptotically output-size
optimal. The factorization is computed once and shared across all $r$ strips of
a closed-wire loft. The validity postcondition L5 costs
$O(N\log N+C\,T_{\text{contact}})$ and dominates the construction.

### 2.4 Gordon surfaces

If both profile and guide curves are supplied, the Boolean sum
$S=S_u+S_v-S_{uv}$ applies, the cardinal functions satisfying
$\phi_j(u_i)=\delta_{ij}$ and $\psi_j(v_i)=\delta_{ij}$ so that the correction
term removes the doubly-counted network intersections exactly. Convert the three
components to compatible spline bases and combine their control nets: the result
is again an ordinary B-spline/NURBS surface.

**Gordon is a construction algorithm, not a geometric carrier.** It introduces no
new certification obligations beyond basis compatibility of the three
components, and its output is certified by L5 like any other surface.

---

## 3. The contact complex

This section defines the object shared by offset and blend.

### 3.1 Definition

For each oriented $C^2$ support face $S_i$ with unit normal $n_i$ and side
$\epsilon_i\in\{\pm1\}$, the signed normal-contact manifold is
$$H_i=\bigl\{(c,r,u_i)\;:\;c=S_i(u_i)+\epsilon_i r\,n_i(u_i),\;r>0\bigr\}.$$
For an active support set $A=\{i_1,\dots,i_k\}$,
$$\mathcal C_A=\bigl\{(c,r,u_{i_1},\dots,u_{i_k})\;:\;
c=S_{i_j}(u_{i_j})+\epsilon_{i_j}r\,n_{i_j}(u_{i_j})\ \ \forall j\bigr\}.$$

A **radius law** is a scalar constraint $\Phi(c,r)=0$.

### 3.2 Dimension

**Theorem (contact stratum).** Write $F_A$ for the $3k$ contact equations in the
$4+2k$ unknowns $(c,r,u_{i_1},\dots,u_{i_k})$. If $F_A$ is submersive on a
neighbourhood, i.e.
$$DF_A\,DF_A^{\mathsf T}\succeq\eta_F^2\,I_{3k},\qquad \eta_F>0,$$
then $\mathcal C_A$ is locally a $C^1$ manifold of dimension
$$\dim\mathcal C_A=4-k .$$
If moreover $\Phi$ is transverse to it, certified by
$$D(F_A,\Phi)\,D(F_A,\Phi)^{\mathsf T}\succeq\eta_\Phi^2\,I_{3k+1},$$
then
$$\dim\bigl(\mathcal C_A\cap\{\Phi=0\}\bigr)=3-k .$$

*Proof.* Implicit function theorem; the rank hypothesis is exactly surjectivity
of the differential, and the dimension is the difference of unknowns and
independent equations. $\square$

The margin conditions are stated as surjectivity margins because $DF_A$ is
$3k\times(4+2k)$ and is never injective; the smallest singular value of the full
matrix is not the relevant quantity, the smallest of its $3k$ singular values is.
Both forms are interval-certifiable and plug directly into P4 continuation.

### 3.3 The stratum table

| $k$ | $\dim\mathcal C_A$ | with $\Phi=0$ | offset reading ($\Phi=r-t$) | blend reading ($\Phi=r-R(\lambda)$) |
|---|---|---|---|---|
| 1 | 3 | 2 | offset **face** stratum | — |
| 2 | 2 | 1 | offset **edge** stratum: pipe over a 1-D spine | fillet spine |
| 3 | 1 | 0 | offset **corner** stratum: spherical patch at an isolated centre | triple-contact junction |
| 4 | 0 | $-1$ | isolated exceptional centre | generically empty |
| $\ge5$ | $<0$ | $<0$ | generically empty | generically empty |

Two consequences follow immediately.

**The rounded offset of a solid is the constant-radius rolling-ball contact
complex.** Its faces, edges and corners are the $k=1,2,3$ strata. Focal
degeneracy of the offset face is the $k=1$ regularity condition; the appearance
of a third support is the edge-to-corner incidence; the clearance predicate P5
at $r=\lvert t\rvert$ is the global embedding condition. The offset and blend
feature families share their regularity theory, their event theory, their
broad phase, and their arrangement engine.

**A fixed-radius sphere tangent to four generic faces is overconstrained.** At
$k=4$ with a radius law the dimension is $-1$. Extending a three-face corner
solver to four faces is therefore the wrong generalization; general $n$-valent
corners require setback patches (§5.5).

### 3.4 Rounded and sharp variants

The table describes the **rounded** completion, in which convex edges and
corners receive ball strata. Two other completions occur and are specified
separately:

* **Sharp (mitered / extended)** edges and corners, obtained by extending the
  adjacent offset faces and intersecting them. These are produced by the
  arrangement engine, not by the contact system, and their points are **not**
  within $\lvert t\rvert$ of their source: a convex corner of dihedral half-angle
  $\theta$ yields points at distance $\lvert t\rvert/\sin\theta$, unbounded as
  $\theta\to0$. Every stratum therefore carries a certified reach bound
  $$\rho_A\ \ge\ \sup_{x\in A'}d(x,A),$$
  computed from its own construction; for ball strata $\rho_A=\lvert t\rvert$.
* **Concave** edges under an offset that moves material inward, where adjacent
  offset faces overlap and the completion is a trim, again by the arrangement
  engine.

All three completions share P3, P4, P5, P6 and the arrangement engine; they
differ only in the stratum-generation rule. The $k=1$ face stratum is common to
all of them.

---

## 4. Offset and shell

### 4.1 Local certificates

**Face stratum ($k=1$).** With principal curvatures $\kappa_1,\kappa_2$ of the
oriented source face, the offset Jacobian determinant is
$$J_t=(1-t\kappa_1)(1-t\kappa_2)=1-2Ht+Kt^2 .$$
Require a certified margin
$$J_t\ \ge\ \eta_F>0 \quad\text{on the whole face.}$$
Sign consistency is then automatic: on a connected face $J_t$ is continuous and
cannot change sign without vanishing, so the margin also rules out an
orientation-reversed offset face. Failure yields `FocalDegeneracy`.

**Edge stratum ($k=2$).** The stratum is a canal surface over the certified
spine; use the exact regularity criterion of §6.

**Corner stratum ($k=3$).** The centre is an isolated point certified by P4; the
stratum is a spherical patch bounded by the incident edge strata, and is regular
wherever $r>0$.

**Stars.** A closed star spanning several glued strata is certified embedded by
P3, which is the constructive form of local embeddedness at edges and corners.
Within a single stratum, P2 supplies the injectivity radius.

### 4.2 The bridge theorem

Let $K_t$ be the finite cell complex of §3 with its intended identifications, and
$F_t:K_t\to\mathbb R^3$ its geometric realization.

**Theorem S1 (stratified offset embedding bridge).** Assume

1. $K_t$ is a compact $2$-manifold-with-corners carrying the intended incidence
   structure;
2. identified strata have exactly consistent realizations (P6), so that $F_t$ is
   well defined and continuous on the quotient;
3. $F_t$ is injective on the quotient: for $x\not\sim y$, $F_t(x)\ne F_t(y)$.

Then $F_t:K_t\to\mathbb R^3$ is a topological embedding. Local regularity (§4.1)
additionally supplies the stratified smooth structure on each stratum.

*Proof.* By (2), $F_t$ is continuous; by (3) it is a bijection onto its image.
$K_t$ is compact and $\mathbb R^3$ is Hausdorff, so a continuous bijection from
$K_t$ onto its image is a homeomorphism onto that image. $\square$

This is a finite-realization theorem about the B-rep quotient, not a reach
theorem. Sharp edges and corners destroy ordinary smooth reach; S1 is the
correct finite replacement.

Hypothesis (3) is the whole difficulty, and it quantifies over *all* pairs,
including pairs within a single stratum arbitrarily close to the diagonal. It is
discharged in three regimes and is undecidable without the first:

* near-diagonal within a stratum — P2;
* near-diagonal across identified strata, i.e. within a closed star — P3;
* everything else — broad phase (§4.3) followed by interval contact solves,
  equivalently P5.

**Corollary S1′ (solid).** If in addition $K_t$ is closed, connected and
orientable, then by Jordan–Brouwer separation $F_t(K_t)$ divides $\mathbb R^3$
into exactly two components with $F_t(K_t)$ as their common boundary; the bounded
component is the resulting solid. Without this corollary S1 certifies a surface,
not a body.

### 4.3 Broad phase

Construct the strata — which the pipeline does in any case, at $O(N)$ — and
build the BVH over the **constructed** strata. Overlap queries against that
hierarchy are exact and require no inflation argument.

Where a pre-construction filter is wanted, prune on
$$d(A,B)>\rho_A+\rho_B\;\;\Longrightarrow\;\;\text{offset realizations are disjoint},$$
with $\rho$ the certified per-stratum reach of §3.4.

*Proof.* If offset points $a'\in A'$, $b'\in B'$ coincided, then
$d(A,B)\le\lVert a-a'\rVert+\lVert b'-b\rVert\le\rho_A+\rho_B$. $\square$

For ball strata this specializes to the familiar $2\lvert t\rvert$ bound. It does
**not** specialize that way for mitered or extended strata, which is why the
bound is stated with $\rho$ rather than $\lvert t\rvert$.

### 4.4 The fixed-$t$ pipeline

For `shell(body, t)` the kernel need only decide the propositions at that $t$;
no critical-parameter theory is required.

$$\text{construct }K_t\;\to\;\text{local certificates §4.1}\;\to\;
\text{P3 on stars}\;\to\;\text{BVH §4.3}\;\to\;\text{P5 / contact solves}\;\to\;
\text{S1}\;\to\;\text{S1}' .$$

A v1 shell is therefore not restricted to generic $t$. It certifies essentially
arbitrary fixed $t$ and refuses only where the contact machinery cannot decide a
particular configuration. Cost is
$$O\bigl(N\log N+C\,T_{\text{contact}}\bigr),$$
$C$ being the number of genuinely nearby candidate pairs. Quadratic worst-case
behaviour is irreducible: a model may contain $\Theta(N^2)$ near-contacting
pairs.

---

## 5. Blends

### 5.1 The admissible stratum graph

Build only the local $1$-skeleton of the constrained contact complex:
$$\mathcal E_{ij}=\mathcal C_{\{i,j\}}\cap\{\Phi=0\}
\quad(\text{1-D}),\qquad
\mathcal V_{ijk}=\mathcal C_{\{i,j,k\}}\cap\{\Phi=0\}\quad(\text{0-D}).$$
A rolling-ball fillet network is a walk in
$$\mathcal G_r=\{\mathcal E_{ij}\}\cup\{\mathcal V_{ijk}\}.$$
Computing an $\mathcal E_{ij}$ is the certified offset/SSI continuation problem
already required elsewhere.

At a triple-contact node the three pairwise strata $\mathcal E_{ij}$,
$\mathcal E_{jk}$, $\mathcal E_{ki}$ meet. The continuation chooses no branch
heuristically: attach the admissibility inequalities P5 and retain only the
outgoing strata lying on the boundary of the admissible centre region. An
arbitrary selected edge train $e_1-e_2-\dots-e_m$ is then ordinary active-set
continuation with linear orchestration and no global coupling between edges.

Nodes shared by several branches are governed by P6: the node is solved once and
referenced, so incident branches meet exactly.

### 5.2 Discrete state and event isolation

Define the **discrete state** of a branch point $s$ as
$$\Sigma(s)=\Bigl(A(s),\;\bigl\{\text{trim cell containing }q_i(s)\bigr\}_{i\in A},\;
\operatorname{rank}DF,\;\operatorname{sign}J,\;\operatorname{sign}\mathrm{Clear}\Bigr),$$
where $q_i(s)=S_i(u_i(s))$ are the contact points. The event vocabulary is
exactly the set of functions whose vanishing permits a component of $\Sigma$ to
change:

$$\begin{aligned}
E_{\text{trim}}&:\ \text{a contact point reaches a support-face trim boundary},\\
E_{\text{third}}&:\ \text{a further face becomes tangent to the ball},\\
E_{\text{focal}}&:\ \text{a normal offset loses regularity},\\
E_{\text{rank}}&:\ \operatorname{rank}DF\ \text{drops},\\
E_{\text{collision}}&:\ \mathrm{Clear}\ \text{is lost},\\
E_{\text{trace}}&:\ \text{two contact or trim curves meet}.
\end{aligned}$$

**Theorem (event isolation).** Let $B$ be a compact regular segment of a
two-support constrained branch. If every event function is certified nonzero and
interval-separated on $B$, every active support remains in its trim interior, and
the branch Jacobian retains a positive rank margin, then $\Sigma$ is constant
throughout $B$.

*Proof.* Each component of $\Sigma$ is locally constant wherever its defining
function is separated from its threshold, and $B$ is connected. $\square$

Because $\Sigma$ was defined as precisely the tuple these functions control,
completeness of the vocabulary is definitional rather than asserted; the residual
obligation — that $\Sigma$ determines the output B-rep topology — belongs to the
arrangement stage and is listed in §10.

Each event is an isolated root problem for P4. A third-face event is a
constrained triple-contact node,
$$c=S_i(u_i)+\epsilon_i rn_i,\quad
c=S_j(u_j)+\epsilon_j rn_j,\quad
c=S_k(u_k)+\epsilon_k rn_k,\quad \Phi(c,r)=0;$$
a face-boundary event adds a trim parameter $t$ and the equation
$S_i(u_i)=E(t)$.

The architectural rule that follows is the operative one:

$$\textbf{No topology speculation between certified events.}$$

### 5.3 Variable radius

Introduce a guide curve $G$ and a radius law $R$. Rather than defining a
progress coordinate as a nearest-point projection — which would require
certifying a positive tubular radius, hence a global bottleneck computation —
impose the **foot-point** equations
$$\bigl(c-G(\lambda)\bigr)\cdot G'(\lambda)=0,\qquad r-R(\lambda)=0 .$$

This adds one unknown $\lambda$ and two equations while removing $\Phi$, so the
dimension count of §3.2 is unchanged at $3-k$: two-support configurations remain
$1$-D spines, three-support configurations remain isolated junctions,
four-support configurations remain generically empty. The system stays polynomial
and is directly amenable to interval/Krawczyk continuation.

Uniqueness of the foot point is local and cheap:
$$\partial_\lambda\bigl[(c-G)\cdot G'\bigr]=-\lVert G'\rVert^2+(c-G)\cdot G''\;\le\;-\eta<0,$$
which for unit-speed $G$ is $\lVert c-G\rVert\,\kappa_G<1$. The global branch —
a distant part of the guide passing near $c$ — is excluded by P5, which the
pipeline runs regardless.

The solver dimensionality is therefore identical to the constant-radius case.
Admissible laws for v1 are constant, linear, cubic Hermite, monotone cubic, and
control radii at chain vertices. A network optimizer choosing all radii
simultaneously is a separate concern and is deliberately out of scope: the kernel
answers *does the requested radius law produce a valid certified blend?*, not
*what radius law should be invented?*

### 5.4 Face consumption

While tracing the ball, the certified contact curves $q_i(s)=S_i(u_i(s))$ are
obtained on the support faces. Do not decide in advance how much of $F_i$
survives. On each affected face, construct the arrangement of the original
trimming pcurves, the new fillet contact pcurves, and the contact curves of
neighbouring fillets; mark the cells removed by the blend; set
$$F_i^{\text{new}}=F_i\setminus R_i .$$
If no retained $2$-cell remains, $F_i^{\text{new}}=\varnothing$ and the face
disappears.

Face-consuming blends are therefore not a geometric primitive but an outcome of
the trim arrangement. The classic short intermediate face $A-B-C$ is handled by
the same mechanism: the centre path reaches an $A/B/C$ triple-contact node,
departs on another pair-support branch, and the arrangement subsequently finds
that $B$ retains no cell. No cascading special-case solver is invoked.

The same arrangement engine performs concave-edge trimming in the sharp offset
variant of §3.4.

### 5.5 Setback corners

For a genuine $n$-valent corner the rolling-ball system has no degrees of freedom
left (§3.3). The established answer is a setback vertex blend, whose corner
region is naturally $2n$-sided with boundary
$$P_1,Q_1,P_2,Q_2,\dots,P_n,Q_n,$$
$P_i$ a profile curve cut across incoming fillet $i$, and $Q_i$ a spring curve
lying on a surviving primary face. This supplies exact boundary and
tangent-plane data.

Use a deterministic setback split with Hermite ribbon construction (P1); the
construction itself is untrusted. Certify the resulting patch $P:D\to\mathbb R^3$
on four counts:

1. **Boundary** — each outer patch boundary equals the prescribed $P_i$ or $Q_i$;
2. **$G^1$ ribbons** — on each boundary, $P_v(u,0)=\lambda(u)d(u)$ with
   $\lambda(u)>0$ and $d(u)$ in the tangent plane of the adjacent fillet or
   primary face. With regularity this makes $\operatorname{span}\{P_u,P_v\}$ the
   adjacent tangent plane; $\lambda>0$ additionally prevents fold-back;
3. **Local regularity** — $\inf_D\lVert P_u\times P_v\rVert\ge\eta_J>0$;
4. **Global embeddedness** — P3.

Fallback when no admissible projection exists: pairwise patch SSI, boundary
intersection exclusion, regularity, and an inside/outside witness.

---

## 6. Canal surfaces

The edge strata of the rounded offset and the surfaces of all rolling-ball
blends are canal surfaces, so this section is shared by §4 and §5.

### 6.1 Envelope and characteristic circles

Let $c(s)$ be a unit-speed $C^2$ spine and $r(s)>0$ a radius, with
$$p=r',\qquad q=r'',\qquad a=\sqrt{1-p^2}.$$
The envelope of $\lVert x-c(s)\rVert^2-r(s)^2=0$ satisfies
$$(x-c)\cdot c'=-r r' .$$
Decomposing $x-c$ in an orthonormal frame $(T,e_\theta,f_\theta)$ with $T=c'$
gives the characteristic circle
$$X(s,\theta)=c(s)-r(s)p(s)\,T(s)+r(s)a(s)\,e_\theta(s).$$
The first gate is immediate:
$$\boxed{\;\lvert r'(s)\rvert<1\;}$$
since otherwise $a=0$ and the characteristic circle degenerates.

### 6.2 Contact points lie on the envelope

**Lemma.** Every certified contact point of the rolling-ball system lies on the
characteristic circle of its spine parameter, and the canal patch meets the
support face with a common tangent plane.

*Proof.* From $c=q_i+\epsilon_i r n_i$ with $q_i=S_i(u_i(s))$,
$$(q_i-c)\cdot c'=-\epsilon_i r\,n_i\cdot\bigl(S_i'+\epsilon_i r'n_i+\epsilon_i r n_i'\bigr)
=-\epsilon_i^2 r r'=-rr',$$
using $n_i\cdot S_i'=0$ and $n_i\cdot n_i'=0$. This is the envelope condition.
The canal normal at $q_i$ is along $(q_i-c)/r=-\epsilon_i n_i$, parallel to the
face normal, so the tangent planes agree and the join is $G^1$. $\square$

The $G^1$ join is therefore free — a consequence of the contact equations, not an
additional constraint to impose.

### 6.3 Exact regularity

**Theorem (canal regularity).** With the notation above and any orthonormal frame
$(T,e_\theta,f_\theta)$,
$$\boxed{\;\bigl\lVert X_s\times X_\theta\bigr\rVert
= r\,\Bigl\lvert\,a^2-rq-ra\,\bigl(c''\!\cdot e_\theta\bigr)\Bigr\rvert\;}$$
and consequently $X$ is regular at $s$ for all $\theta$ if and only if
$$\boxed{\;\lvert a^2-rq\rvert\;>\;r\,a\,\lVert c''\rVert\;.}$$

*Derivation.* In the Frenet frame, with $e_\theta=\cos\theta\,N+\sin\theta\,B$
and $f_\theta=-\sin\theta\,N+\cos\theta\,B$, one has $X_\theta=ra\,f_\theta$ and,
using $(rp)'=p^2+rq$ and $(ra)'=pa-rpq/a$,
$$X_s=\alpha\,T+\frac{p}{a}\,\alpha\,e_\theta+\bigl(ra\tau+rp\kappa\sin\theta\bigr)f_\theta,
\qquad \alpha:=a^2-rq-ra\kappa\cos\theta .$$
The $f_\theta$ component is annihilated by the cross product with $X_\theta$, and
since $(T,e_\theta,f_\theta)$ is orthonormal,
$$\lVert X_s\times X_\theta\rVert
= ra\,\lvert\alpha\rvert\sqrt{1+p^2/a^2}=r\,\lvert\alpha\rvert,$$
using $a^2+p^2=1$. Ranging $\theta$ and noting
$\kappa\cos\theta=c''\!\cdot e_\theta$ gives the criterion. $\square$

Three consequences:

* the torsion cancels, so the criterion is frame-independent and $\kappa$ may be
  written $\lVert c''\rVert$; no Frenet frame is needed and inflection points of
  the spine are not degenerate;
* for constant radius, $p=q=0$ and $a=1$, recovering the classical pipe condition
  $r\lVert c''\rVert<1$;
* the criterion is necessary and sufficient, so no interval-Jacobian fallback
  tier is required for the canal. The only refusal is `CanalSingular`, issued
  when the enclosure of $\lvert a^2-rq\rvert-ra\lVert c''\rVert$ straddles zero.

**Arc restriction.** A blend surface occupies only the arc
$\theta\in[\theta_1(s),\theta_2(s)]$ between its two contact points. Certifying
$$\min_{\theta\in[\theta_1(s),\theta_2(s)]}\lvert\alpha(s,\theta)\rvert>0$$
is both cheaper and strictly more permissive than the all-$\theta$ criterion, and
is the form the implementation should use. The all-$\theta$ criterion is the
correct one for a closed pipe.

---

## 7. Thickness and critical parameters

`shell(body,t)` needs none of this section (§4.4). It is required by
`max_shell_thickness(body)` and `valid_shell_interval(body)`.

### 7.1 Conservative certified thickness

Over a parameter box $B$ with interval enclosures $[H],[K]$ of the mean and
Gaussian curvature, the focal condition $J_t=1-2Ht+Kt^2\ge\eta$ is a quadratic in
$t$ with interval coefficients; the admissible $t$-set has a closed form obtained
from the coefficient corners together with the degenerate case $0\in[K]$.
Intersecting over all boxes yields $t_{\text{focal}}$.

Separately, let $d_{\min}$ be the certified minimum distance between
**non-adjacent** source strata, from the BVH. Rounded offsets of two source
strata can meet only if $2\lvert t\rvert\ge d_{\min}$. Hence

$$\boxed{\;t_{\text{safe}}=\min\bigl(t_{\text{focal}},\;d_{\min}/2\bigr)\;}$$

is a certified safe thickness computed in $O(N\log N)$ with no root finding and
no semialgebraic projection. Adjacent-stratum pairs are excluded here and handled
by the local star certificates of §4.1.

This is the recommended v1 of `max_shell_thickness`, reported as a conservative
lower bound. It is the constructive form of the reach dichotomy: for a compact
smooth embedded manifold, first normal-tube failure occurs either through a local
curvature (focal) event or through a global bottleneck, and the two terms above
are exactly those alternatives. For a globally smooth closed surface it
specializes to $\lvert t\rvert<\operatorname{reach}(M)\Rightarrow$ the normal
offset is embedded; the B-rep case is harder only because sharp edges destroy
ordinary smooth reach, which is what S1 replaces.

### 7.2 Exact interval structure

**Theorem S3 (finite thickness stratification).** The source B-rep admits
finitely many combinatorial types $\sigma$ — subsets of strata together with
their incidences — permitted by the construction rules. For each type, after
introducing auxiliary variables for normalized normals ($n\cdot n=1$,
$n\cdot S_u=n\cdot S_v=0$, with an orientation sign), the family of bad
configurations is semialgebraic, so the valid set
$$V_\sigma=\{t:F_t\text{ is a valid embedded shell of type }\sigma\}$$
is semialgebraic by Tarski–Seidenberg. Hence $V=\bigcup_\sigma V_\sigma$ is
semialgebraic. Moreover $V$ is open, because validity is defined by strict
regularity margins together with injectivity of a compact family, and embeddings
of compact manifolds are stable under $C^1$-small perturbations. Therefore
$$V=\bigcup_{i=1}^{r}(a_i,b_i)$$
for finite $r$, with no isolated points.

Away from stratified critical values the family has constant topological type
(Hardt/Thom triviality), so topology cannot change in the interior of a parameter
cell containing no focal, contact or boundary event.

### 7.3 Event systems

Interval endpoints arise from three families: focal events, first global
tangencies, and boundary/corner events. A face–face first tangency is the square
$5\times5$ system in $(u,v,s,w,t)$,
$$F_A(u,v,t)-F_B(s,w,t)=0,\qquad
n_A\cdot\partial_sF_B=0,\qquad n_A\cdot\partial_wF_B=0,$$
isolated by P4. Candidate pairs are pruned by the §4.3 bound; the minimum over
survivors is taken with the P4 separation-margin operator, which refuses on
overlap rather than choosing. Positive-dimensional event sets return
`NonGenericThicknessEvent`.

---

## 8. Complexity

| Operation | Cost |
|---|---|
| Loft solve, $q=3$ | $O(mn)$; output $\Theta(mn)$ — optimal |
| Loft validity (L5) | $O(N\log N+C\,T_{\text{contact}})$ — dominates |
| `shell(body,t)` | $O(N)$ construct $+\;O(N\log N)$ BVH $+\;C\,T_{\text{contact}}$ |
| `t_safe` conservative (§7.1) | $O(N\log N)$, closed form |
| `valid_shell_interval` (§7.2–7.3) | $O(C)$ isolated $5\times5$ solves after pruning |
| Fillet chain, $m$ edges | $\sum_e(\text{continuation cost}_e)$; no global coupling |
| Canal regularity (§6.3) | closed form per box; no fallback tier |
| Setback patch | one projection search $+\;O(\#\text{sub-patches})$ sign tests |
| Face arrangement | output-sensitive in the arrangement engine |

$C$ is the number of genuinely near-contacting candidate pairs. Quadratic
worst-case behaviour is irreducible for contact and appears nowhere else.

---

## 9. Refusal taxonomy

```
Construction   NonPositiveWeightField        §2.2 L1r
               SingularInterpolationSystem   §1 P1 fallback path only
               AmbiguousCorrespondence       §2.2 L4

Regularity     FocalDegeneracy               §4.1  (k = 1)
               CanalSingular                 §6.3  (k = 2)
               RankDeficientContact          §3.2

Embedding      UnintendedContact             §1 P5 / §4.2 S1(3)
               StarNotEmbedded               §1 P2, P3
               NoAdmissibleProjection        §1 P3 projection search

Parametric     NonGenericThicknessEvent      §7.3
               AmbiguousEventOrdering        §1 P4
```

Every refusal is a statement that a specific certificate could not be decided at
the available precision, not a statement that the geometry is invalid. Where
increasing interval precision can separate a quantity from zero, the
implementation retries at higher precision before refusing.

---

## 10. Open obligations

1. **$\Sigma$ determines topology.** §5.2 certifies that the discrete state is
   constant between events. The arrangement stage must be shown to produce a
   B-rep whose combinatorics depend only on $\Sigma$ and not on the branch
   parameterization.
2. **Sharp and concave completion rules.** §3.4 fixes their interface — the reach
   bound $\rho_A$ and the arrangement engine — but the extend-and-intersect rule
   for mitered edges and corners, and the concave-edge trim rule, need the same
   stratum-by-stratum treatment §3 gives the rounded variant.
3. **P6 enforcement.** Construction identity must survive Booleans, transforms
   and blends at the representation level. L3 and the shared-node guarantees of
   §5.1 are otherwise tolerance comparisons rather than exact facts.
4. **Projection candidate set.** §1 P3 specifies the search order; the spherical
   code and its cardinality must be fixed normatively for reproducibility.
5. **General sweeps.** §6 supplies the regularity theory a general sweep feature
   would require, but sweeps along an arbitrary spine with an arbitrary profile
   are not specified here; only canal surfaces arising from the contact system
   are.
