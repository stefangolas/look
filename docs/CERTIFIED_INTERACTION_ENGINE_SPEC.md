# A Certified Interaction Engine for B-rep Booleans — Theory Specification (draft 1)

**Status:** adopted draft, session 51+ (look). This is the root theory for the
Boolean work. Sections 0–15 below are the theory as authored; the appended
§16 is the loop-side codebase reconciliation audit (substrate status per
theory element, packet skeleton) and is look-side content.

**Provenance note:** every claim in §0–15 carries the author's status tag —
`[std]` established literature, `[drv]` derived here (unchecked),
`[cnj]` conjecture, `[open]` unresolved. §13 flags what needs independent
verification before any of it becomes load-bearing. Nothing in §0–15 has
been implemented or benchmarked.

---

## 0. Status and how to read this

This document specifies a solid-modelling kernel whose Boolean operation
returns either a certified-correct result or an explicit refusal, never a
silently-invalid shape.

Every claim carries a status tag:

| tag | meaning |
|---|---|
| `[std]` | Established result from the literature, cited. Use as-is. |
| `[drv]` | Derived here from `[std]` results. Proof included. Believed correct, not peer-reviewed, not machine-checked. |
| `[cnj]` | Plausible, argued for, not proved. Do not build load-bearing code on it without proving it first. |
| `[open]` | Genuinely unresolved. |

The `[drv]` results were developed rapidly and have been checked by nobody.
Several nest — later ones assume earlier ones. Before any becomes
load-bearing, it needs independent verification. The items most likely to
contain an error are flagged in §13.

Nothing in this document has been implemented or benchmarked. All
performance claims are derived from cost models, not measured.

## 1. The object

### 1.1 Interaction fiber product

For solids A, B with boundary parameterizations $R_A : D_A \to \mathbb{R}^3$,
$R_B : D_B \to \mathbb{R}^3$, define the interaction

$$I = \partial A \times_{\mathbb{R}^3} \partial B = \{(a,b) \in D_A \times D_B : R_A(a) = R_B(b)\}$$

Write $F(a,b) = R_A(a) - R_B(b)$, so $I = F^{-1}(0)$, with
$F : D \to \mathbb{R}^3$, $D = D_A \times D_B \subset \mathbb{R}^4$.

For NURBS spans $S_A = P_A/W_A$, $S_B = P_B/W_B$, cross-multiplication gives
a polynomial system

$$H_i(u,v,s,t) = P_{A,i} W_B - P_{B,i} W_A = 0, \quad i = 1,2,3$$

three equations in four unknowns. Expected dimension 1.

**Caveat.** Cross-multiplication admits spurious components where
$W_A = 0$ or $W_B = 0$. Positive weights exclude these; the certificate must
record the positivity check rather than assume it.

### 1.2 The domain is stratified, not a box

$D$ is not $[0,1]^4$. Each trimmed face domain carries its own
stratification

$$D_A = F_A \supset E_A \supset V_A$$

(face interior, trim edges, vertices) and the true domain is the product
complex, with product strata $\sigma_A \times \sigma_B$.

**[drv] Genericity of product strata.** For generic operands, every product
stratum except $F_A \times F_B$ is empty:

- $E \times E$: two curves in $\mathbb{R}^3$, expected dim $1+1-3 < 0$
- $F \times V$: a point on a surface, expected dim $2+0-3 < 0$
- $E \times F$, $F \times E$: expected dim 0, isolated points

**Consequence.** Interval exclusion disposes of the non-generic strata
instantly in the common case, so auditing all $(1+n_E+n_V)^2$ product strata
is cheap despite the count.

**Consequence.** These strata are exactly the classical hand-enumerated
degeneracies: through-edge, through-vertex, edge-edge, shared-vertex. The
product stratification *derives* that list. No taxonomy is written by hand.

### 1.3 What the engine must produce

SSI is not the Boolean. The full pipeline is

```
interaction → parameter-domain arrangement → cell classification
            → boundary selection → output B-rep
```

Interaction and arrangement are orthogonal axes, each with its own
conditioning and escalation ladder. Treating arrangement as a deeper
fallback level of interaction is a modelling error.

## 2. Conditioning

### 2.1 Why raw derivatives are the wrong measurement

$DF = [J_A, -J_B]$ is taken with respect to raw parameters. NURBS
parameterizations are non-uniform, so $\sigma_{\min}(DF)$ collapses in
regions of low parametric speed that are geometrically benign. A kernel
conditioned on raw $DF$ escalates on knot placement rather than on geometry.

### 2.2 Metric normalization

Let $G = \mathrm{diag}(G_A, G_B)$ be the block first-fundamental-form
metric, $G_A = J_A^T J_A$. Define

$$\widehat{DF} = DF \cdot G^{-1/2}$$

**[drv] Theorem 2.1 (crossing-angle identity).**
$\widehat{DF} = [Q_A, -Q_B]$ with $Q_A, Q_B$ orthonormal tangent frames.
Hence

$$\widehat{DF}\,\widehat{DF}^T = P_A + P_B = 2I - (n_A n_A^T + n_B n_B^T)$$

with spectrum $\{2, 1+c, 1-c\}$, $c = |n_A \cdot n_B|$. Therefore

$$\sigma(x) := \sigma_{\min}(\widehat{DF}(x)) = \sqrt{1 - |n_A \cdot n_B|} \quad \text{exactly}, \qquad M_1 := \|\widehat{DF}\| = \sqrt{2} \quad \text{exactly}.$$

*Proof.* $Q_A = J_A G_A^{-1/2}$ has orthonormal columns since
$G_A = J_A^T J_A$. Then $\widehat{DF}\widehat{DF}^T = Q_A Q_A^T + Q_B Q_B^T = P_A + P_B$. The projector sum has the stated spectrum by decomposing
$\mathbb{R}^3$ into the intersection of tangent planes and the
two-dimensional complement. ∎

**Consequences.**

- With $\theta$ the crossing angle, $\sigma \approx \theta/\sqrt{2}$. The
  conditioning of SSI *is* the crossing angle, up to a constant.
- $\sigma$ is invariant under knot insertion, degree elevation, affine
  reparameterization, and global rescaling.
- $M_1$ is a constant, not an estimate. It never needs bounding.
- $M_2 := \mathrm{Lip}(\widehat{DF})$ in normalized coordinates has units of
  inverse length and is essentially a curvature: $M_2 \approx 1/\rho$ with
  $\rho$ the smaller principal radius of curvature.

### 2.3 The degenerate-chart term — mandatory, not optional

$G^{-1/2}$ does not exist where the chart degenerates: collapsed poles of
spherical patches, cone apexes, revolve axes, the degenerate edge of a NURBS
sphere. This is ordinary STEP content, not pathology.

$\sigma$ must therefore be paired with a separate chart-quality term
$\sigma_G = \sigma_{\min}(G)$. The two escalate to different places:

- $\sigma \to 0$: interaction degenerate → algebraic escalation (§6)
- $\sigma_G \to 0$: chart degenerate → reparameterize or excise (§13.1)

Conflating them sends every sphere in the corpus down the expensive path.

### 2.4 Two condition numbers, not one

There are two distinct conditioning questions and they must not be
identified.

- $\kappa_{\text{geom}}$ on realized maps with metric $G$; drives subdivision
  and tracking
- $\kappa_{\text{data}} = 1/\mathrm{dist}(c, \Delta)$ in
  coefficient/provenance space; drives snap safety and intent projection

**[std]** By the condition-number theorem (Demmel), condition is
proportional to reciprocal distance to the ill-posed set in a specified
input norm. The norms differ here, so the two numbers differ.

**[drv] Why they cannot be identified.** Knot insertion leaves the realized
surface fixed while changing the dimension and geometry of coefficient
space. So no raw coefficient-space distance can be invariant under
refinement, while $\kappa_{\text{geom}}$ is invariant by Theorem 2.1.

**[drv] Transfer bound.** For polynomial B-splines, basis functions are
non-negative and partition unity, so

$$\|R_c - R_{c'}\|_\infty \le \max_i \|P_i - P_i'\|$$

($C^0$ constant = 1, sharp). For rational, multiply by $w_{\max}/w_{\min}$.
Since $\kappa_{\text{geom}}$ depends on normals, the relevant constant is
the $C^1$ one; derivatives of a B-spline are B-splines on differenced
control points divided by knot spans, so
$L_{\text{realize}}^{C^1} \sim 1/h_{\min}$, which degrades under knot
insertion. This quantifies the non-invariance exactly, and localizes it to
one mechanism.

**[cnj] Mitigation.** Weighting the coefficient metric by knot spacing (as
in mesh-dependent norms in isogeometric analysis) makes $\kappa_{\text{data}}$
approximately refinement-invariant without constructing a quotient metric on
representation space. This is a diagonal rescaling. Not proved.

Certificates are then related by

$$d_{\text{data}}(c, \Delta_{\text{data}}) \ge d_{\text{geom}}(R(c), \Delta_{\text{geom}}) / L_{\text{realize}}.$$

## 3. Certified predicates

Notation: cell $C \subset D$, $X = F^{-1}(0) \cap C$, $n = 4$.

### 3.1 (E) Exclusion

$$0 \notin \Box F(C) \implies X = \emptyset$$

Bernstein range evaluation with de Casteljau subdivision. `[std]`

### 3.2 Column selection

**[std]** (Gu–Eisenstat). For $A \in \mathbb{R}^{k \times n}$ there is a
column subset $J$, $|J| = k$, with

$$\sigma_{\min}(A_J) \ge \sigma_{\min}(A) / \sqrt{1 + f^2 k(n-k)}$$

For $k=3$, $n=4$, exhaustive search over the four candidate subsets attains
maximum volume, hence $f = 1$, hence

$$\sigma_{\min}(\hat{A}_J) \ge \sigma/2.$$

No RRQR algorithm is needed; try all four.

**[drv] Slope bound.** Let $j \notin J$ and let $v$ be the unit kernel
vector. $\hat{A}_J v_J + a_j v_j = 0$ gives
$v_J = -\hat{A}_J^{-1} a_j v_j$. Maximum volume gives
$\|\hat{A}_J^{-1} a_j\|_\infty \le 1$, so

$$|v_j| \ge 1/2 \quad \text{and} \quad L := \|dy^*/dx_j\| = \|\hat{A}_J^{-1} a_j\| \le \sqrt{3}.$$

The excluded column is automatically a valid graph direction. No separate
test.

### 3.3 (R) Spanning-arc predicate, via slicewise Krawczyk

Split $x = (y, t)$, $y = x_J$, $t = x_j$, cell $C = Y \times T$,
$\mathrm{rad}(Y) = r$, $\mathrm{rad}(T) = h$. Let
$C_0 = (\partial_y \hat{G}(m))^{-1}$ and

$$K = m_y - C_0 \hat{G}(m_y, T) + (I - C_0 \partial_y \hat{G}(C))(Y - m_y).$$

**[drv] Theorem A.** If $K \subseteq \mathrm{int}\, Y$, then $X$ is a single
$C^1$ arc, the graph of $y^* : T \to Y$, meeting each slice $\{t = \text{const}\}$
exactly once, entering through $t = t^-$, exiting through $t = t^+$, and
disjoint from the lateral faces.

*Proof.* For fixed $t$, $\hat{G}(\cdot, t) : Y \to \mathbb{R}^3$ is square
and by inclusion monotonicity $K(t) \subseteq K \subseteq \mathrm{int}\,Y$.
Krawczyk's theorem `[std]` gives exactly one zero $y^*(t) \in Y$.
$K \subset \mathrm{int}\,Y$ forces $\|I - C_0 \partial_y \hat{G}\| < 1$, so
$\partial_y \hat{G}$ is nonsingular throughout $C$, and the implicit
function theorem gives $y^* \in C^1$ with
$y^{*\prime} = -(\partial_y \hat{G})^{-1} \partial_t \hat{G}$. The solution
set is exactly $\mathrm{graph}(y^*)$. ∎

**[drv] Size condition.**
$\|I - C_0 \partial_y \hat{G}\| \le (2/\sigma) M_2\, \mathrm{diam}(C) \le 1/2$
gives

$$\mathrm{diam}(C) < \sigma / (4 M_2) \approx \theta\rho / (4\sqrt{2}).$$

Certified cell size is crossing angle times radius of curvature.

### 3.4 Cubes do not satisfy Theorem A

**[drv]** The residual condition requires

$$(2/\sigma)(\|F(m)\| + M_1 h) < r/2, \qquad M_1 = \sqrt{2}.$$

With $r = h$ this demands $\sqrt{2} < \sigma/4$, and $\sigma \le 1$ always.
It fails unconditionally. Geometrically: the arc has slope up to $\sqrt{3}$,
so as $t$ sweeps the cell the root moves further in $y$ than a cube's
half-width permits.

Two repairs:

| repair | shape | aspect |
|---|---|---|
| anisotropic box | $r \gtrsim 8 M_1 h / \sigma$ | $\Theta(1/\sigma)$, wide normal, short along arc |
| shear (parallelotope) | $r \gtrsim M_2 h^2 / \sigma$ | long along arc |

The second is the parallelotope predictor–corrector of Martin, Goldsztejn,
Granvilliers and Jermann `[std]`, and this calculation is the reason it
exists. Axis-aligned subdivision cannot express the sheared cell.

### 3.5 (R′) Cube predicate, via monotonicity

Cubes are still needed for exclusion and seeding, so they need a predicate
that does not require contraction.

**[drv] Theorem B (no loops).** If some 3×3 minor $M_j = \det DF_J$ has
certified constant sign on box $B$, then $X$ is a compact 1-manifold with
boundary, every component is a $C^1$ arc strictly monotone in $x_j$ with
both endpoints on $\partial B$, and $X$ contains no closed loop.

*Proof.* $M_j \neq 0$ gives $\mathrm{rank}\, DF = 3$, so 0 is a regular
value and $X \cap \mathrm{int}\,B$ is a 1-manifold. The kernel satisfies
$v_J = -A_J^{-1} a_j v_j$, so $v_j \neq 0$: the tangent is nowhere in
$\{x_j = \text{const}\}$, so $\pi_j|_X$ is a submersion, hence a local
homeomorphism onto its image. A circle admits no local homeomorphism to
$\mathbb{R}$ — the image would be open, and compact hence closed, forcing
it to be all of $\mathbb{R}$, which is not compact. So no component is a
circle. Each component is an interval, compact, and cannot terminate in
$\mathrm{int}\,B$. ∎

*Monotonicity kills loops. This is the load-bearing line.*

**[drv] Theorem C (isotopy).** Strengthen to (R′): the minor condition of
Theorem B and $|X \cap \partial B| = 2$. Then $X$ is a single arc with
endpoints $p, q$, and the chord $pq$ is isotopic to $X$ rel endpoints
within $B$.

*Proof.* Two boundary points and no loops force one component. Monotonicity
gives $p_j \neq q_j$, so arc and chord are both graphs over $[p_j, q_j]$,
say $y^*(t)$ and $y_{\text{ch}}(t)$. Then
$H_s(t) = ((1-s)y^*(t) + s\, y_{\text{ch}}(t),\, t)$ is a graph over $t$ for
every $s$, hence an embedding, and stays in $B$ by convexity. ∎

**Resolution of §3.4.** Counting boundary intersections replaces the
contraction argument. Cubes work after all. Theorem A's parallelotope
remains the right structure for *fast tracking*, not for correctness of
subdivision.

**Recursion depth is one, not four.** On a 3-face $\{x_i = c\}$, $F$
restricted is 3 equations in 3 unknowns — square. Endpoints are found by
ordinary Krawczyk root isolation plus exclusion. There is no
parameterizability predicate at level 3.

**[open] Face tangency.** If $X$ is tangent to a face, the face system has a
double root and Krawczyk fails. The subdivision grid is ours to choose, so
perturb the cut plane and retry. Persistent failure across perturbations is
evidence of a genuine degeneracy and must be typed as a distinct outcome.
This retry loop is expected to be a recurring bug source.

## 4. Completeness

### 4.1 Completeness by cover

**[drv] Theorem D.** If $D$ is partitioned into cells each certified by (E)
or (R′), the assembled PL graph is isotopic to $F^{-1}(0) \cap D$ and no
connected component is missing.

*Proof.* Every point of $F^{-1}(0)$ lies in some cell; no (E) cell contains
one; so every point lies in an (R′) cell and appears in the assembly. A
missed component would be a loop interior to a single cell, forbidden by
Theorem B. Isotopy is Theorem C cellwise, glued along faces where endpoint
sets agree by construction. ∎

Once the cover completes, completeness is certified. Real critical-point
witnesses are not needed on the normal path. They fire only when the cover
cannot be completed.

### 4.2 Completeness by polar exclusion — the practical route

Subdivision is too expensive to be the primary solver (§5.3). The engine
seeds from the boundary and tracks. This theorem makes that complete.

Boundary seeding is complete because 3-face systems are square: Krawczyk
isolates all roots.

Let $A$ be the tracked arcs with certified tubes $N(A)$, and
$U = D \setminus N(A)$. Fix generic $w \in \mathbb{R}^4$ and set

$$\Sigma_w = \{x : F(x) = 0, \; \det[DF(x)\,;\, w^T] = 0\}.$$

**[drv] Theorem E.** If interval exclusion certifies $\Sigma_w \cap U = \emptyset$,
then $A$ is all of $X$.

*Proof.* Suppose $C$ is a component not in $A$. Boundary seeding was
complete, so $C \cap \partial D = \emptyset$, hence $C \subset \mathrm{int}\,D$
is compact without boundary. Then $\phi = \langle w, \cdot \rangle$ attains a
maximum on $C$, where the tangent $\ker DF$ is orthogonal to $w$, so
$[DF; w^T]$ is singular. That point is in $\Sigma_w \cap U$. ∎

**Why this is practical:**

- It is *exclusion*, not solving. The homotopy never runs in the common
  case, and $U$ is mostly far from $X$ where plain (E) disposes of it
  instantly.
- If it does run, $\Sigma_w$ is square (4 equations, 4 unknowns), so sparse
  homotopy returns everything and Krawczyk certifies it.
- $\Sigma_w$ is the same polar system as the ε-selection solve of §6.3 with
  a different linear form. One machine, several targets.

**[cnj] Genericity of $w$:** the argument needs $w$ not orthogonal to the
tangent along a positive-dimensional set. Randomize and retry. Not proved.

### 4.3 Real versus complex components

**[std]** A complex witness set does not certify real completeness: one
complex irreducible curve can carry several real connected components
($y^2 = x^3 - x$), and a generic complex slice need not meet each.

**[drv] Squareness repairs this.** In a square zero-dimensional system,
every real solution is an isolated complex solution, so complex
completeness implies real completeness. This is why §4.2 and §6.3 both
insist on square systems. *The squareness is doing the work, not the
geometry.*

## 5. Cost model

### 5.1 Continuous amortization in dimension n

Call $F_* : D \to \mathbb{R}_{>0}$ a **local size function** if: for every
cell $B$, the existence of a single $x \in B$ with $w(B) < F_*(x)$ implies
$B$ is retired.

**[drv] Theorem 5.1.** Dyadic $2^n$-ary subdivision satisfies

$$\#\text{leaves} \le \max\left(1,\; 2^n \int_D F_*(x)^{-n}\, dx\right).$$

*Proof.* Contrapositive of the definition: if $B$ is split then
$F_*(x) \le w(B)$ for all $x \in B$. For a leaf $L$ with parent $P$,
$F_*(x) \le w(P) = 2w(L)$ on $L$, so
$\int_L F_*^{-n} \ge \mathrm{vol}(L)/(2w(L))^n = 2^{-n}$. Leaves are
disjoint and cover $D$. ∎

Total tree size is $\#\text{leaves} \cdot (1 + 1/(2^n - 1))$.

### 5.2 Instantiation

**[drv]** Exclusion fires once $2M_1\sqrt{n}\, w(B) < \|F(x)\|$; the
regularity margin fires once $2M_2\sqrt{n}\, w(B) < \sigma(x)$
($\sigma_{\min}$ is 1-Lipschitz in operator norm). So

$$F_E = \|F\|/(2\sqrt{n} M_1), \quad F_R = \sigma/(2\sqrt{n} M_2), \quad F_* = \max(F_E, F_R)$$

$$\kappa(x) = \left[\max\left(\|F(x)\|/M_1,\; \sigma(x)/M_2\right)\right]^{-1}$$

giving

$$\#\text{leaves} \le (4\sqrt{n})^n \int_D \kappa^n = 4096 \int_D \kappa^4 \qquad (n = 4).$$

Note the two scale factors differ — the residual term normalizes by a
first-derivative bound, the rank term by a second-derivative bound. A single
`scale(F, X)` numerator is wrong.

**Monotonicity.** Any predicate at least as strong as (E)/(R) admits a
pointwise larger $F_*$, hence a smaller integral. The conservative constants
above bound whatever sharper predicates ship. This analysis is never redone.

### 5.3 Divergence behaviour, and the θ⁻⁴ problem

**[drv] Transverse case.** Near the curve $\kappa \approx M_2/\sigma_0$; at
distance $d$ the exclusion term takes over, $\kappa \approx M_1/(\sigma_0 d)$;
crossover at $d^* = M_1/M_2$. Integrating over the tube (three transverse
dimensions, curve length $L$):

$$\int \kappa^4 \asymp L\, M_1^3 M_2 / \sigma_0^4 \qquad \text{i.e.} \quad \text{cost} \sim \theta^{-4}.$$

**[drv] Singular case.** Near a stratum of dimension $m$ with
$\kappa \asymp r^{-k}$, truncating at box width $2^{-d}$:

| stratum | $k$ | $m$ | boxes vs depth |
|---|---|---|---|
| none | — | — | bounded |
| ordinary node, isolated | 1 | 0 | $\Theta(d)$ — logarithmic |
| tangency along a curve | 1 | 1 | $\Theta(2^d)$ |
| coincident surface region | 1 | 2 | $\Theta(2^{2d})$ |
| higher multiplicity, isolated | $k$ | 0 | $\Theta(2^{4(k-1)d})$ |

Exponent $= 4(k-1) + m$. Dimension and multiplicity are confounded; $m$ is
readable only under reducedness.

### 5.4 Consequences for the architecture

**Divergence is not an escalation signal.** At an ordinary node the
divergence is logarithmic: four more levels cost a constant factor. A
threshold $\kappa > K_2$ would be arbitrary. The correct scheduler compares
predicted work:

$$\text{escalate} \iff \hat{C}_{\text{subdiv}}(X) > \hat{C}_{\text{algebraic}}(X), \qquad \hat{C}_{\text{subdiv}} = 4096 \int_X \kappa^4.$$

This removes MAX_DEPTH-style heuristics without introducing a universal
constant.

**The slope is a free diagnostic.** Measure $\log_2(\#\text{boxes})$ against
depth over three or four levels; the slope estimates $4(k-1) + m$ directly.
Slope ≈ 0 means isolated node, keep going. Slope ≥ 1 means
positive-dimensional degeneracy, escalate now — and the slope predicts which
machinery to escalate to. This fires before any algebraic computation, from
a counter already maintained.

**Subdivision cannot be the primary solver.** $\theta^{-4}$ at
$\theta = 1°$ is $\sim 10^8$ cells. Near-tangential intersections are what
blends and draft faces produce; they are routine, not exotic.

**[drv] Continuation is not affected.** Certified step size is $\sim \theta\rho$
(§3.3), so tracking costs $O(\text{length}/(\theta\rho))$ — linear in
$1/\theta$. Three orders of magnitude better at $\theta = 1°$.

*Therefore: seed and track. Subdivide only to prove exclusion and to
localize polar candidates. The octree is a support structure, not the
engine.*

## 6. Singular loci

### 6.1 Where singularities are, and why they are always there

The first singular locus is

$$\Sigma_1 = \{F = 0,\; \mathrm{rank}\, DF < 3\}.$$

For a 3×4 matrix, rank ≤ 2 has codimension $(3-2)(4-2) = 2$. So
$\dim \Sigma_1 = 4 - 3 - 2 < 0$: **generically empty**.

**Consequence:** every singular case in a real file is there because a human
put it there — a tangent fillet, a symmetric pattern, two faces from one
construction. The data, however, is floating-point. This is the central
practical problem (§8).

### 6.2 Deflation, and what it does not do

**[std]** Isosingular deflation (Hauenstein–Wampler) regularizes a singular
solution by differentiation; the determinantal form adds polynomials without
new variables. Certification then proceeds by ordinary Krawczyk. Deflation
sequences are Thom–Boardman sequences, so the singularity type is a computed
label — the taxonomy is derived, not enumerated.

**[std]** But deflation does not recover real local topology. $y^2 - x^2$
and $y^2 + x^2$ have identical multiplicity and deflation behaviour; over
$\mathbb{R}$ one is a crossing, the other an isolated point. For a Boolean
that is the difference between a branching SSI and a single touch point with
no curve.

*So multiplicity is not the target branch count.*

### 6.3 ε-selection is a square solve, not a separation bound

**[std]** Burr and Byrd note that the singular Plantinga–Vegter extension
relies on singular-point separation bounds pessimistic enough to make
practicality questionable. This is the classical wall.

**[drv] It is avoidable.** After isolating an isolated singularity $p$:

1. Critical points of $r(q) = \|q - p\|^2$ on $X$ are cut out by $F(q) = 0$
   together with the adjugate-form orthogonality condition — 4 equations,
   4 unknowns, square. The adjugate formulation avoids dividing by anything
   vanishing at $p$.
2. Square and zero-dimensional ⟹ sparse homotopy returns all complex
   roots, Krawczyk isolates, real filtering gives the exact list of critical
   values. $c_{\text{next}}$ is *computed*, not bounded below.
3. Choose $0 < \varepsilon^2 < c_{\text{next}}$ with the ball containing no
   other singular stratum.
4. Solve $F(q) = 0$, $\|q - p\|^2 = \varepsilon^2$ — again square, 4×4. The
   sphere is certified transverse, so every link point is nonsingular.
   Complex completeness implies real completeness (§4.3):

   ```
   sparse homotopy → all isolated complex roots → Krawczyk → real filter
   ```

   gives the link and its completeness certificate. No generic separation
   bound, no multiplicity assumption, no "hope we found every branch."

**Failure mode:** a positive-dimensional critical locus, arising under exact
rotational symmetry (a circle of equidistant points). Detected by the
homotopy; repaired by moving $p$ off-axis.

Deflation's remaining job: isolate and regularize the stratum, establish the
normal slice, and separate $p$ from the smallest positive critical value.
For a positive-dimensional singular stratum, take a certified normal slice
first and repeat.

### 6.4 The Boolean lives on the R³ link, not the parameter link

A frequent conflation, worth stating explicitly.

- The **parameter link** $I \cap S_\varepsilon(p) \subset S^3$ is, for a
  curve, a finite point set. Its complement is connected. It has no sectors
  to label. It tells you the branch structure.
- The **model link** $S^2_\varepsilon(q) \cap (\partial A \cup \partial B) \subset \mathbb{R}^3$
  is a 2-sphere cut by two curves into faces. Each face carries
  $(\chi_A, \chi_B) \in \{0,1\}^2$, and $b_*(\chi_A, \chi_B)$ applied
  facewise gives the local output as a cone over a subsurface. This is the
  local Boolean theorem.

Both are needed; the realization map joins them.

### 6.5 Whitney machinery: offline only

**[std]** Helmer, Leykin and Nanda stratify a complexification using
conormal or polar techniques and show the result is describable by real
polynomials, with extensions to basic semialgebraic sets and maps. Helmer
and Mohr improve the key step via equidimensional decomposition and add
minimal-stratification coarsening.

These are complexification-first and Gröbner-bound. Use **offline** for
ground-truth generation, adversarial corpus classification, and proof
discovery. Never in the hot path.

**[std]** Where a quantitative regularity condition is needed, use Verdier's
(w) or Kuo's ratio test, not Whitney (b). Whitney (a)/(b) are limit
conditions on secant and tangent directions, with no pointwise inequality
for interval arithmetic to bound. Verdier (w) is an inequality with a
constant, (w)-regular stratifications exist in the subanalytic setting,
(w) ⟹ (b) there, and Verdier's isotopy lemma gives Lipschitz-type control.

**[std]** Thom's isotopy lemma requires properness. $I$ lives over trimmed
domains with boundary; the boundary strata must be included or the
trivialization statement does not apply near them.

**[std]** For $A \cap A$ (offset self-intersection, imprint) the diagonal is
always a component of the fiber product and must be excised explicitly.

## 7. Arrangement

### 7.1 Pcurves are embedded

**[drv] Lemma F.** If $R_B$ is injective and both charts are immersions,
then $\pi_A : I \to D_A$ is an embedding.

*Proof.* Injectivity: $(a,b), (a, b') \in I$ give
$R_B(b) = R_A(a) = R_B(b')$, so $b = b'$. Immersion: if the tangent
$v = (v_a, v_b)$ had $v_a = 0$ then $J_B v_b = J_A v_a = 0$, forcing
$v_b = 0$ since $J_B$ has full rank. An injective immersion on a compact
domain is an embedding. ∎

**Consequence.** A single face pair's SSI pcurve is a *simple* curve in
$D_A$. It cannot self-cross. Every crossing in the arrangement comes from a
different face pair or from a trim curve. Self-crossings appear only where
the operand is genuinely self-intersecting, which is a validity failure to
reject upstream.

This materially shrinks the arrangement layer. Per-curve topology needs only
the ordinary Lin–Yap parameterizability predicate `[std]`; Burr–Byrd
simultaneous isotopy `[std]` is needed only for inter-curve crossings.

### 7.2 The arrangement ladder

| level | mechanism |
|---|---|
| existing trim topology | reuse |
| per-curve parameterizability | Lemma F + Lin–Yap |
| simultaneous crossings | Burr–Byrd |
| singular arrangement | local links |
| symbolic tail | offline |

## 8. Representation and intent

### 8.1 Certified procedural carrier — prerequisite for everything

Generic intersection curves are not NURBS. Approximating them and recording
the residual as tolerance is the mechanism behind tolerance accumulation:
tolerances are monotone increasing across a feature history, so each
operation starts from geometry known only to the previous operation's error.

Remove the refit and the mechanism is gone. The B-rep type system must admit

```text
CertifiedImplicitIntersectionCurve
```

as a legitimate edge carrier, PL only at tessellation. This is a
prerequisite, it is independently useful, and it requires none of the theory
above.

### 8.2 Discriminant projection as intent semantics

**[cnj]** Define a coefficient/provenance-space discriminant
$\Delta_*$: operand pairs at which the interaction topology changes. Then
$d(c, \Delta_*)$ is the structural stability radius, and an intended
tangency correction is

$$c \mapsto \Pi_{\Delta_{\text{tangent}}}(c).$$

Snapping is projection onto a discriminant stratum. It is well-posed exactly
when the projection is unique, i.e. inside the reach of the discriminant in
coefficient space.

**[drv] Selection rule: deepest, not nearest.** Discriminant strata form a
poset under containment. Design intent is typically maximally degenerate — a
designer who made two faces tangent *and* coaxial meant both. Nearest-point
projection may land on the shallower stratum. The rule is: among strata
within the certified radius, project to the deepest; refuse only if two
incomparable strata are both in range.

**[drv] Projections do not commute.** Snapping A to B's carrier then
re-snapping B differs from the reverse. The certificate needs a canonical
order or a joint projection onto the intersection stratum.

```rust
IntentProjectionCert {
    source,
    target_stratum,
    perturbation_metric,
    displacement,
    unique_projection_radius,
}
```

Ambiguity is a first-class outcome: `Refuse(IntentAmbiguous)`.

**Note on reach.** Ordinary reach vanishes at any transverse crossing (the
set is non-smooth). Weak feature size does not — for two transversally
crossing planes every positive offset is topologically identical, so there
is no positive critical value. Distinguishing wfs from reach and μ-reach is
precisely why Chazal and Lieutier introduced it `[std]`. But Hausdorff
distance is the wrong perturbation model for CAD regardless: nobody perturbs
a STEP file by moving point sets. Coefficient space is the right model,
which is why §8.2 is stated there.

### 8.3 Provenance precedes everything

**Level P₋₁: construction proof fast path.**

If a fillet was created with the construction theorem
$\mathrm{dist}(S_{\text{blend}}, S_A) = r$ and certified tangency along an
authored contact curve, a later operation does not *rediscover* that
tangency from a rank-deficient system. It verifies that the construction
certificate still applies — a far weaker statement.

Applies to: coaxial holes, concentric cylinders, extrusion side/base
incidence, mirrored surfaces, shared carriers, exact offsets.

*The best way to know a tangency is exact is never to lose the information.*
This is likely the largest speed advantage on native geometry, and it
requires owning the modelling front end.

## 9. Validity gates

**[std]** $\chi$ is a valuation on compact semialgebraic sets:

$$\chi(A \cup B) = \chi(A) + \chi(B) - \chi(A \cap B)$$
$$\chi(\partial\Omega) = V - E + F, \qquad \chi(\partial\Omega) = 2\chi(\Omega) \text{ for compact orientable 3-manifolds}$$

The $(\chi_A, \chi_B)$ labelling is multiplication in the ring of
constructible functions; $b_*$ is a morphism of constructible functions
(Euler calculus).

**Weakness, stated plainly.** If the arrangement and the predicted $\chi$
both derive from the same incomplete SSI, they are wrong consistently. And
distinct topologies can share $\chi$.

So the gate is a layer:

$$\text{incidence invariants} + \chi + H_*(\cdot\,; \mathbb{Z}_2)$$

Mod-2 homology on the finite output complex is cheap relative to SSI and
catches much more than $\chi$. The gate is strongest when the expected
invariant is derived independently — from real witness or critical-point
data rather than from the arrangement being checked.

## 10. Architecture

Provenance in front; interaction and arrangement as orthogonal axes; one
shared numerical-algebraic service.

```text
                P₋₁  provenance / construction proof
                              |
        +---------------------+---------------------+
        |                                           |
  INTERACTION                                 ARRANGEMENT
  analytic carrier recognition                existing trim topology
  BVH + Bernstein/interval exclusion          per-curve (Lemma F + Lin–Yap)
  boundary seeding (square systems)           simultaneous crossings
  parallelotope continuation (θρ step)        singular arrangement
  polar exclusion (Theorem E)                 symbolic tail (offline)
  sparse homotopy  [on failure only]
  deflation + isosingular
  square sphere link
  Whitney / Verdier tail (offline)
        |                                           |
        +---------------------+---------------------+
                              |
                      cell complex
                              |
              classification / selection / output
                              |
                  validity gates (§9)
```

### 10.1 One numerical-algebraic service

```rust
SparseHomotopyFamily {
    support_signature,
    cached_generic_solutions,
    parameter_map,
    tracker,
    endgame,
    krawczyk_certifier,
}
```

Instantiated for: SSI slice, distance-critical system, boundary-stratum
critical system, polar system $\Sigma_w$, link sphere system, deflated
system. Support signatures differ; the machinery does not.

**[std]** Mixed volume / BKK, not total-degree Bézout, is the path budget
for tensor-product supports; polyhedral homotopy (Huber–Sturmfels) tracks
mixed-volume-many paths. In practice use the mixed volume of each Lagrange
critical system as the direct budget; ED degree is the conceptual invariant.

**[drv] Start-system reuse.** The number of distinct support signatures
across a CAD corpus is small. Cache generic solutions once per signature;
every face pair is then a parameter homotopy with no polyhedral
construction. This may matter as much as mixed volume itself.

**[std] Caveat.** At a non-generic target, paths coalesce, diverge, or
expose a dimension jump. Endgames recover the limits. What is lost is
specifically the completeness certificate — parameter homotopy at a special
target is a solver, not an oracle, and the closure argument must come from
elsewhere for those cells.

### 10.2 Typed outcomes

```rust
enum InteractionOutcome {
    Certified(StratifiedInteraction),
    Unresolved { kappa: f64, cell: Cell, slope_estimate: f64 },
    Refuse(IntentAmbiguous),
}
```

`Unresolved` is a first-class result, not a failure. For agent consumers
with no visual sanity check in the loop, this is the single most valuable
property of the design.

## 11. Implementation plan

**Phase 0 — arithmetic (weeks 1–4).** Correctly-rounded interval arithmetic;
Bernstein range evaluation with de Casteljau subdivision; Krawczyk on square
systems. *Gate: reproduce known root counts on a bicubic test suite.*
Everything downstream inherits this layer's quality. Do not rush it.

**Phase 1 — transverse SSI (weeks 5–12).** Metric-normalized $\sigma$ (§2.2)
and chart term $\sigma_G$ (§2.3); four-subset max-volume column choice
(§3.2); boundary seeding; parallelotope continuation at step $h \lesssim \theta\rho$
(§3.3). No singular handling — return `Unresolved`. *Gates: differential
test against OCCT on transverse cases; measure the unresolved rate on the
STEP corpus. This number decides everything after it, and it cannot be
guessed.*

**Phase 2 — closure and carrier (weeks 13–20).** Theorem E polar exclusion;
`CertifiedImplicitIntersectionCurve`; B-rep type changes. *Gate: no missed
components on a synthetic suite with planted interior loops.*

**Phase 3 — arrangement (weeks 21–30).** Per-curve parameterizability via
Lemma F; inter-curve crossings via Burr–Byrd; cell classification; output
construction; validity gates. *Gate: end-to-end Boolean matching OCCT on the
transverse corpus.*

**Phase 4 — branch on measurement.**

| Phase 1 result | build next |
|---|---|
| dominated by near-tangency from dirty input | discriminant projection (§8.2) |
| dominated by genuine singularities | deflation + square sphere link (§6) |
| dominated by degenerate charts | pole excision (§2.3) |

Prior expectation: the first. It is a guess and it is cheap to replace with
a measurement.

**Deliberately deferred:** sparse homotopy infrastructure until Theorem E
actually fails on real data; provenance P₋₁ until the modelling front end is
owned; all Whitney machinery.

### 11.1 Effort ratio

Theory is the small part. Expect roughly **1:8 theory to plumbing**. The
schedule will be consumed by: interval/Bernstein arithmetic quality;
face-consistency bookkeeping across subdivision; the face-tangency retry;
carrier integration into the B-rep type system; the arrangement layer; STEP
import pathology.

## 12. Position against OCCT

**Where this is better**

| item | OCCT | here |
|---|---|---|
| curve carrier | walking line → BSpline refit, residual → tolerance | exact procedural carrier |
| tolerance accumulation | monotone increasing across history | mechanism removed with the refit |
| crossing-angle margin | implicit in tolerance comparisons | $\sigma = \sqrt{1-|n_A \cdot n_B|}$ exactly |
| failure reporting | returns a shape; validity checked after | typed `Unresolved` / `Refuse` |
| closure claims | empirical | Theorems D and E |
| degeneracy handling | per-case code | product strata + deflation labels |

**Where OCCT stays ahead**

| item | why |
|---|---|
| dirty STEP import | fuzzy Boolean, tolerant shapes, ShapeFix — a mature intent-recovery layer. §8.2 is research-stage. |
| analytic special cases | quadric-pair intersection including tangent configurations, decades tuned |
| coincident / tangent faces | working today; parity is the first milestone, not an improvement |
| breadth | sweeps, offsets, fillets, meshing, IO |
| regression knowledge | thirty years encoding facts written down nowhere else |
| measured performance | nothing here has been benchmarked |

**Not an improvement.** Operation unification is parity, not advantage.
OCCT already made this architectural move: General Fuse is the primitive,
and cut/common/fuse/section/split are wrappers over `BOPAlgo_Builder` with
different cell classification. The pipeline is intersect, image, label,
select — the same four-step shape.

**The contribution is supplying a theorem where OCCT supplies tolerances.**
That is a narrower claim than "a new unifying primitive," and a stronger
one, because it says exactly what is being competed on.

**The realistic target.** Not "replace OCCT." A certified core plus an
explicitly typed intent layer whose decisions are recorded as assumptions
the certificate is conditional on. That supports a claim OCCT cannot make:
*here is the answer, and here is exactly which snapping decisions it depends
on.* Differential testing against OCCT stays useful for a long time — as a
coverage oracle, not a correctness one.

## 13. Open problems and risks

### 13.1 [open] Degenerate charts

Unavoidable — spheres, cones, revolutions with collapsed poles are ordinary
STEP content. Constants degrade by $\sqrt{\mathrm{cond}(G)}$. Believed to be
excision plus boundary-stratum handling rather than normalizing through, but
the gluing of excised pieces is not proved, and that is where an error would
hide.

*Not research. Must be done before Phase 1 touches real files.*

### 13.2 [downgraded] Certificate composition

Previously listed as a research program. Reassessed:

Tolerance accumulation in OCCT has a mechanism — the refit discards
information and records the discarded amount. With an exact carrier there is
no refit, and the second operation's $\kappa_{\text{geom}}$ is a property of
the actual geometry, not of how it was produced. Nothing accumulates.

What can still degrade is enclosure quality over nested procedural
definitions — the wrapping effect, which compounds with nesting depth even
when the underlying object is exact. This shows up as spurious `Unresolved`
and rising cost, not as wrong answers.

Mitigations are standard: Taylor models, preconditioned enclosures,
re-certify from provenance at intervals, cap nesting depth.

Status: engineering risk with known countermeasures. Measure early — if
enclosure quality decays fast, practical tree depth is small.

### 13.3 [open] The (R′) soundness proof at face tangency

§3.5 handles it by grid perturbation and retry. The termination of that
retry loop is not proved.

### 13.4 Verification priorities

The `[drv]` results most likely to contain an error, in order:

1. **Theorem D gluing** — face endpoint sets agreeing "by construction" is
   asserted, not proved. Shared-face consistency under adaptive subdivision
   is exactly where implementations go wrong.
2. **Theorem E genericity of $w$** — the argument needs $w$ not orthogonal
   to the tangent along a positive-dimensional set. Stated as
   randomize-and-retry.
3. **Every constant in §3 and §5** — the 4096, the $\sigma/2$, the
   $\sqrt{3}$, the $\sigma/(4M_2)$. Derived quickly. Recheck each.
4. **§5.3 singular exponents** — the $4(k-1)+m$ table assumes a clean radial
   model of $\kappa$ near the stratum.
5. **§2.4 knot-weighted metric `[cnj]`** — asserted to restore refinement
   invariance. Not proved.

### 13.5 Methodological warning

This specification was developed rapidly, in one sitting, with each layer
resting on the one before. The chain from Theorem 2.1 through Theorem E is
long, internally consistent, and checked by nobody. Internal consistency is
not evidence of correctness — a single bad step in the middle would
propagate silently through everything after it, and the fluency of the
derivation is not information about its soundness.

Before any of this becomes load-bearing code: verify §13.4 items
independently, preferably by someone who did not write them.

## 14. What is actually on the critical path

Nothing in §13 blocks Phase 1.

Truck today returns None on coplanar contact and silently returns an empty
solid on near-tangent input. Exact carriers + certified transverse SSI +
typed `Unresolved` is a large improvement over that, needs zero new
theorems, and approaches parity with OCCT on the cases OCCT handles.

The theory buys the tail. The tail is where differentiation lives. It is not
on the path to something usable.

**Build Phase 1. Measure the unresolved rate. Let that number choose
Phase 4.**

## 15. References

**Subdivision and certified topology**

- Plantinga, Vegter. Isotopic approximation of implicit curves and surfaces. SGP 2004; Isotopic meshing of implicit surfaces. Visual Computer 23, 2007.
- Burr, Choi, Galehouse, Yap. Complete subdivision algorithms II: isotopic meshing of singular algebraic curves. JSC 47(2), 2012.
- Lin, Yap. Adaptive isotopic approximation of nonsingular curves: parameterizability and nonlocal isotopy. DCG 45(4), 2011.
- Burr, Byrd. Certified simultaneous isotopic approximation of pairs of curves via subdivision. ISSAC 2023; and of algebraic curves via subdivision. JSC 131, 2025.
- Burr. Continuous amortization and extensions. JSC 77, 2016.
- Liang, Mourrain, Pavone. Subdivision methods for the topology of 2D and 3D implicit curves. 2008.

**Condition-based complexity**

- Cucker, Ergür, Tonelli-Cueto. Plantinga–Vegter algorithm takes average polynomial time. ISSAC 2019; On the complexity of the Plantinga–Vegter algorithm. DCG 68, 2022.
- Tonelli-Cueto, Tsigaridas. Condition numbers for the cube I. ISSAC 2020.
- Demmel. On condition numbers and the distance to the nearest ill-posed problem. 1987.

**Certified continuation and verification**

- Martin, Goldsztejn, Granvilliers, Jermann. Certified parallelotope continuation for one-manifolds. SINUM 51(6), 2013.
- Moore, Kearfott, Cloud. Introduction to Interval Analysis. SIAM, 2009.
- Breiding, Rose, Timme. Certifying zeros of polynomial systems using interval arithmetic (HomotopyContinuation.jl certify).

**Numerical algebraic geometry**

- Hauenstein, Wampler. Isosingular sets and deflation. FoCM 13(3), 2013.
- Hauenstein, Sottile. alphaCertified. TOMS 38(4), 2012.
- Hauenstein, Mourrain, Szanto. Certifying isolated singular points and their multiplicity structure. ISSAC 2015.
- Sommese, Verschelde, Wampler. Diagonal homotopies; numerical irreducible decomposition.
- Huber, Sturmfels. A polyhedral method for solving sparse polynomial systems. 1995.
- Draisma, Horobeţ, Ottaviani, Sturmfels, Thomas. The Euclidean distance degree of an algebraic variety.

**Linear algebra**

- Gu, Eisenstat. Efficient algorithms for computing a strong rank-revealing QR factorization. SISC 17, 1996.

**Stratification theory**

- Helmer, Nanda. Conormal spaces and Whitney stratifications. FoCM, 2022 (+ correction 2023).
- Helmer, Leykin, Nanda. Effective Whitney stratification of real algebraic varieties. arXiv:2307.05427.
- Helmer, Mohr. A new algorithm for Whitney stratification of varieties. arXiv:2406.17122.
- Verdier. Stratifications de Whitney et théorème de Bertini–Sard. 1976.
- Kuo. The ratio test for analytic Whitney stratifications. 1971.
- Ta Lê Loi. Verdier and strict Thom stratifications in o-minimal structures.
- Đinh, Jelonek. Thom isotopy theorem for nonproper maps. DCG 65, 2021.

**Geometric sampling theory**

- Federer. Curvature measures. 1959.
- Chazal, Cohen-Steiner, Lieutier. A sampling theory for compact sets in Euclidean space. DCG, 2009.
- Chazal, Lieutier. Weak feature size and persistent homology.

**Euler calculus**

- Schapira. Operations on constructible functions. 1991.
- Viro. Some integral calculus based on Euler characteristic. 1988.

---

## 16. Codebase reconciliation (loop-side audit, this session)

> **Expanded into the packet program in
> [`CERTIFIED_INTERACTION_ENGINE_BUILD_SPEC.md`](CERTIFIED_INTERACTION_ENGINE_BUILD_SPEC.md)**
> — stage-by-stage tie-in map, write sets, and LOC estimates. The table
> below is the substrate summary; the build spec is the authority on
> packets.

Every theory element mapped onto the tree as of this session. Anchors were
re-derived by command; re-derive before quoting in a packet.

| Theory element | Status | Evidence |
|---|---|---|
| §3.3/§6.3 Krawczyk on square systems | **LANDED** | `truck-evidence/src/num/krawczyk.rs`: `KrawczykSystem<const N: usize>` + `krawczyk::<N>` (:62/:86), generic const-N; consumed by the certified engine and `ssi.rs` (3×3) |
| §3.1 Bernstein range evaluation | **PARTIAL** | `truck-certified/src/hull.rs`: `hull_bernstein_1d` (:95), `hull_bernstein_2d` (:126), `bernstein_derivative_1d/2d` (:178/:193), `hull_curve_homogeneous` (:283). **ABSENT: box evaluation at n = 3, 4** (the interaction domain is 4-D) |
| §3.1/Phase 0 correctly-rounded interval arithmetic | **ABSENT** | No interval-arithmetic library in tree. Only `IntervalEnclosure` (`truck-certified/src/contract.rs:117`) — a 1-D enclosure *value type* with `Method::Interval`, not outward-rounded operations. Phase 0 is real work, not wiring |
| §1.1 certified SSI on square systems | **PARTIAL** | `truck-certified/src/ssi.rs` (3×3 systems, landed), `ssi_trace.rs`, `ssi_types.rs`, `pair_dispatch.rs`. n=4 interaction systems are the extension |
| §2 conditioning / `σ` machinery | **PARTIAL** | `FaceScaleComponents` / `curvature_radius_lower` (`truck-evidence/src/fid/lfs.rs`), `rank_margin` (`certified_map.rs`), `immersion_lower_bound` (`truck-evidence/src/enclosure.rs`). The exact crossing-angle identity (Thm 2.1) is not implemented anywhere |
| §4.2/§5 BVH exclusion | **PARTIAL** | `truck-base/src/bvh.rs`: candidate pairs only; no distance query, no interval exclusion over BVH nodes. CC-004 books the distance query |
| §6.2/§6.3 deflation + square sphere link | **ABSENT** | Nothing in tree. R6 self-intersection deflation (`kernel/selfint.rs`, kernel v2) is the nearest landed analog |
| §6.4 fragment classification (χ_A, χ_B), boundary selection | **PARTIAL** | `truck-shapeops/src/boolean/{split,classify,assemble}.rs`: `FragmentMesh`, `FragmentClassification` (inside_other bits), `FragmentDecision`/`MaterialState4` — the (χ_A, χ_B) decision table exists and is `fragment_decision`-certified; canonical-carrier-only at the lift |
| §7 arrangement: pcurve embedding, crossings | **PARTIAL** | `Same`/`Flip` adjacency parity + `CoincidentPair` landed in `boolean/split.rs`; Region2 `Crossing` screen refuses coplanar-adjacent (the RW-SEED family). Lemma F's consequence (per-face-pair pcurve simplicity) is *used implicitly* but not stated or tested |
| §8.1 `CertifiedImplicitIntersectionCurve` carrier | **ABSENT** | Canonical `Curve` enum has no procedural-carrier variant. `IntersectionCurve` exists as a decorator (`canonical.rs:94/133`) but is not a certified procedural carrier with tessellation-time PL policy |
| §8.3 provenance P₋₁ (construction proof fast path) | **PARTIAL** | `truck-topology/src/entity_id.rs`: `EntityId`/`Op`/`OpKind` (incl. `OpKind::Fillet` :264) and a **`Selector` primitive** (`sel(base, selector)` :176) — the identity vocabulary is landed; propagation through booleans/transforms/blends is ABSENT (CC audit row P6) |
| §9 validity gates (χ valuation, H_*(·; Z₂)) | **ABSENT** | `shell_condition()`/manifold diagnostics landed (`truck-topology/src/manifold.rs`); Euler-char gate and mod-2 homology over the output complex are new (cheap; the complex is finite and small) |
| §10.2 typed outcomes | **PARTIAL** | `Refusal`/`EnvelopeCase`/`NumericallyUnresolved` landed; `Unresolved { kappa, cell, slope_estimate }` as a first-class *non-failure* outcome needs a new arm or a composed evidence type (the CC §6 refusal doctrine: a new arm is a SPEC_GAP, booked in `docs/CERTIFICATE_MAPPING.md`) |
| §14 "truck today returns None on coplanar contact" | **CONFIRMED, typed** | `RW-COPLANAR`/RW-TANGENT refusals recorded (BREP_GENERATION_API §10, `docs/CERTIFICATE_MAPPING.md`); the theory's improvement claim holds against the landed refusals |

**Reconciliation verdicts.**

1. Phase 0 is the true entry cost: the interval/Bernstein box layer at n = 4
   does not exist and everything downstream inherits it (the theory says the
   same). Krawczyk and the 3×3 SSI are landed, so Phase 0 is arithmetic
   quality, not new mathematics.
2. §8.1 (`CertifiedImplicitIntersectionCurve`) is a prerequisite for
   *everything* and needs **none** of the theory — it is the natural
   first packet of the Boolean program and composes with the
   `EdgeSampleLedger` doctrine (PL only at tessellation, integer identity
   preserved).
3. §8.3 is where the tree is unusually strong: the provenance DAG and even a
   `Selector` primitive are landed in `truck-topology`. The gap is
   propagation, not vocabulary — and the bridge spec's selector work
   (PB-001) should consume `entity_id.rs`'s `Selector`, not invent a second
   one.
4. The theory's typed-outcome doctrine matches the house refusal taxonomy;
   `Unresolved { kappa, cell, slope }` should map onto
   `Refusal::NumericallyUnresolved`'s witness slot with a composed evidence
   record, not a parallel outcome enum.
5. Scope interaction with the CC program: CC-020 (k=3 contact) and CC-030
   (blend continuation) build the SSI substrate the interaction engine
   consumes. Sequence the Boolean program AFTER those land; do not fork the
   solver machinery.

**Packet skeleton (BIE program, booked-not-dispatched).** Dispositions only;
packets are written when the CC solver chain lands:

| Packet | Content | Depends |
|---|---|---|
| `BIE-000-CONTRACT` | `InteractionOutcome` mapping onto `Refusal`/evidence; carrier decision for `CertifiedImplicitIntersectionCurve`; mapping rows in `docs/CERTIFICATE_MAPPING.md` | — |
| `BIE-001-ARITHMETIC` | outward-rounded interval ops + Bernstein box evaluation n = 3, 4 (Phase 0 gate: bicubic root counts) | 000 |
| `BIE-002-SSI4` | metric-normalized σ, four-subset column choice, boundary seeding, parallelotope continuation; `Unresolved` elsewhere | 001 |
| `BIE-003-CARRIER` | `CertifiedImplicitIntersectionCurve` + ledger integration (PL at tessellation only) | 000 |
| `BIE-004-CLOSURE` | Theorem E polar exclusion; completeness battery with planted interior loops | 002 |
| `BIE-005-ARRANGE` | Lemma F simplicity test, inter-curve crossings, cell classification in (s,v) charts | 002, 003 |
| `BIE-006-GATES` | χ valuation + mod-2 homology gate; differential suite vs the landed canonical booleans | 005 |
