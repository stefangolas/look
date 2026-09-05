# Certified Tangency and Exact Contact

### A theory specification for singular SSI (T1) and coincident-carrier Boolean classification (T2)

Status: specification. All identities in §2–§4 have been verified symbolically; §6 states what remains
open. Verification notes are in §9.

---

## 0. Summary and verdict

The review response is **mathematically correct in all of its load-bearing claims**, and its central
strategic move — reduce the singular problem to a certified scalar function of two variables *before*
doing any singularity analysis — is right and is adopted here as the organizing principle. The four
corrections it raises against the original draft are all valid.

Confirmed correct:

| Claim | Status |
|---|---|
| $DF$ is $3\times4$, hence **four** maximal minors | correct; draft said three |
| Box-wide rank-deficiency certificate is unachievable for A₁ | correct; rank drops only at a point |
| $(F,H):\mathbb R^4\to\mathbb R^4$ cannot be regular at a rank-2 point | correct; $\operatorname{rank}\le 3$ |
| Difference is not associative, so the draft's property 2 is unprovable as stated | correct |
| Chart-minor identity $M_j=\det(A)\,\partial_{z_j}h$ | verified |
| Chart-deflation determinant $\det DT=\det(A)^3\det(H_h)$ at a critical point | verified, sign $+$ under §2.6 ordering |
| Constrained Hessian $H_h=J^\top(H_f-\lambda_1H_{G_1}-\lambda_2H_{G_2})J$ | verified, and valid off critical points too |
| A₂ quadratic-factor certificate $sf=q^2a+u_1G_1+u_2G_2$ | verified sound |
| Two-bit side algebra $\mathbf 2\times\mathbf 2$ and its eight truth rows | verified by exhaustion |

Corrections and additions made here (details in §8):

1. **The strongest consequence of the deflation was missed.** A unique deflated root with certified
   nonzero critical value proves the box is *transversal*, with **no Hessian and no definiteness test
   at all**; and *no* deflated root proves transversality outright. This subsumes minor separation
   and should be the primary transversality predicate, not a fallback (§2.7, Thm T1.5).
2. **The indefinite Morse case is missing and is generic.** A₁ contact splits into A₁⁺ (definite —
   isolated point) and A₁⁻ (indefinite — a *node*, two branches of the intersection curve crossing
   transversally at a tangential contact). The response only handles A₁⁺. The decision is therefore
   **five-way**, not four-way (§2.9).
3. **The two exactness fallbacks unify into one verifier.** The A₁ "critical value is exactly zero"
   witness and the A₂ contact-factor witness are the same object: an exact polynomial identity with an
   interval-nonvanishing multiplier. One verifier, two instantiations (§4).
4. Hessian and gradient enclosures must be evaluated over the **graph enclosure** $\hat Y\times Z$,
   not the raw box $B$ (§2.5).
5. The deflation determinant identity is **pointwise at critical points only**; it is a termination
   argument, never an interval predicate (§2.6, Remark).
6. An explicit $\lambda_{\min}$ bound is supplied — the response's definiteness test does not produce
   the $\mu$ that the Taylor isolation bound consumes (§2.8).
7. Chart existence at a rank-2 point is made a lemma with a bounded search (18 charts) (§2.1).
8. T2's "measure-zero fragment" is a misnomer and the four-valued `decide` is the wrong shape:
   after atom decomposition there is **no A-vs-B geometry choice at all**, only orientation (§5.5).
9. T2's certified carrier coincidence is the *same* exactness problem as A₁'s critical value and must
   use the same primitive (§4.4).

---

## 1. Setting and notation

Two rational tensor-Bernstein patches $S_1,S_2$ with control data in $\mathbb Q$. Cross-multiplied
difference, the landed `ssi.rs` form:

$$F = W_2N_1 - W_1N_2 : [0,1]^4 \to \mathbb R^3,$$

polynomial in $x=(u_1,v_1,u_2,v_2)$, with $F(x)=0 \iff S_1(u_1,v_1)=S_2(u_2,v_2)$.

$B\subseteq[0,1]^4$ denotes a closed subbox. $DF$ is $3\times4$; it has **four** maximal ($3\times3$)
minors. Generic solution sets are 1-dimensional. Interval quantities carry hats or are written
$\square(B)$; all interval evaluation is outward-rounded over Bernstein control-net hulls.

Contact regimes, in the standard singularity-theoretic naming:

- **Transversal**: $\operatorname{rank}DF=3$ at every point of $F^{-1}(0)\cap B$.
- **A₁**: an isolated rank-2 point with nondegenerate contact Hessian. Splits by signature:
  **A₁⁺** (definite — the zero set is a single point) and **A₁⁻** (indefinite — the zero set is two
  arcs crossing at the point).
- **A₂ / Morse–Bott**: rank 2 along a whole branch; contact order exactly 2 normal to the branch.
  The generic case for filleted mating.
- **Unresolved**: cusps, higher contact, and failure to extract structure.

---

## 2. Part I — Certified tangency closure (T1)

### 2.1 Chart selection

**Definition (chart).** A chart is a choice of one component of $F$ (call it $f$; the other two are
$G=(G_1,G_2)$) and a choice of two of the four coordinates (call them $y=(y_1,y_2)$; the other two
are $z=(z_1,z_2)$). Write $B = Y\times Z$ after the coordinate permutation. Set

$$A := D_yG \in \mathbb R^{2\times2}.$$

There are $3\cdot\binom42 = 18$ charts.

**Lemma T1.0 (chart existence).** If $\operatorname{rank}DF(x_\ast)=2$ then at least one of the 18
charts satisfies $\det A(x_\ast)\ne0$.

*Proof.* A matrix of rank 2 has a nonzero $2\times2$ minor. Take its two rows as $G_1,G_2$ and its two
columns as $y$. ∎

So the chart search is a bounded enumeration, and its success is itself certified by the interval
predicate $0\notin\det A(B)$. Charts are selected once per box and recorded; all downstream
certificates are relative to the recorded chart.

### 2.2 Certified graph

**Hypothesis (H-graph).** On $B=Y\times Z$: $0\notin\det \hat A(B)$, and parametric interval
Newton/Krawczyk on $G(\cdot,z)=0$ certifies, for every $z\in Z$, a unique $y=\phi(z)\in Y$ with
$G(\phi(z),z)=0$, together with an enclosure $\hat Y\supseteq\phi(Z)$, $\hat Y\subseteq Y$.

Under (H-graph) define the **reduced contact function**

$$h : Z\to\mathbb R,\qquad h(z) = f(\phi(z),z),$$

and note the fundamental reduction

$$F^{-1}(0)\cap B \;\xrightarrow{\ \cong\ }\; \{z\in Z : h(z)=0\},\qquad x\mapsto z,$$

a homeomorphism with inverse $z\mapsto(\phi(z),z)$. **The entire singular SSI problem is now a scalar
function of two variables.**

### 2.3 The chart minors

Define the two **chart minors**

$$M_j := \det D_{(y_1,y_2,z_j)}(G_1,G_2,f),\qquad j=1,2.$$

These are polynomials in $x$, evaluable by Bernstein nets everywhere on $B$.

**Theorem T1.1 (minor identity).** For all $x\in B$,

$$M_j(x) = \det A(x)\cdot\Big(f_{z_j} - f_y A^{-1}G_{z_j}\Big)(x),$$

and consequently, on the graph,

$$M_j(\phi(z),z) = \det A(\phi(z),z)\cdot \partial_{z_j}h(z).$$

*Proof.* $M_j = \det\begin{pmatrix}A & G_{z_j}\\ f_y & f_{z_j}\end{pmatrix}$; the Schur complement of
the invertible block $A$ gives the first identity. Differentiating $G(\phi(z),z)\equiv0$ gives
$D\phi = -A^{-1}G_z$, so $\partial_{z_j}h = f_y\,\partial_{z_j}\phi + f_{z_j} = f_{z_j}-f_yA^{-1}G_{z_j}$. ∎

**Corollary T1.2 (rank characterization).** Under (H-graph), for $x$ on the graph,

$$\operatorname{rank}DF(x)=2 \iff M_1(x)=M_2(x)=0 \iff \nabla h(z)=0,$$

and otherwise $\operatorname{rank}DF(x)=3$.

*Proof.* $\det A\ne0$ forces $\operatorname{rank}DF\ge2$, and
$\operatorname{rank}DF = 2 + \operatorname{rank}\!\big(f_z - f_yA^{-1}G_z\big) = 2+\operatorname{rank}\nabla h$. ∎

> **This replaces the draft's "certified rank deficiency over a box".** That certificate is not merely
> expensive, it is *false* in the case it was written for: at an A₁ point the rank is 2 only at the
> point itself, so no positive-width box can certify box-wide minor vanishing. Two minors and one
> $2\times2$ pivot characterize rank exactly, pointwise, which is all that is ever needed.

### 2.4 The deflated system

$$T := (G_1,\,G_2,\,M_1,\,M_2) : \mathbb R^4\to\mathbb R^4.$$

Under (H-graph), $T^{-1}(0)\cap B$ corresponds exactly to the critical points of $h$ in $Z$. $T$ is
square, so the **existing Krawczyk implementation applies unchanged** — no null-vector variables, no
bordered system, no generic-multiplicity machinery.

### 2.5 The reduced Hessian without differentiating $\phi$ twice

Set, on the graph,

$$P = D\phi = -A^{-1}G_z,\qquad
J = \begin{pmatrix}P\\ I_2\end{pmatrix}\in\mathbb R^{4\times2},\qquad
\lambda = A^{-\top}f_y^{\top}\in\mathbb R^2 .$$

**Theorem T1.3 (constrained Hessian).** At every point of the graph (not only at critical points),

$$H_h \;=\; J^\top\Big(H_f-\lambda_1H_{G_1}-\lambda_2H_{G_2}\Big)J,$$

where $H_f,H_{G_a}$ are the full $4\times4$ Hessians in $x$.

*Proof.* $h = f\circ c$ with $c(z)=(\phi(z),z)$, $Dc=J$, so
$D^2h = J^\top H_fJ + \sum_i f_{y_i}D^2\phi_i$. Differentiating $G_a(c(z))\equiv0$ twice gives
$J^\top H_{G_a}J + \sum_i A_{ai}D^2\phi_i = 0$. Writing $f_y = \lambda^\top A$ and substituting
eliminates $D^2\phi$. ∎

**Geometric content.** Up to congruence by the invertible matrix $J$ and a nonzero scalar, $H_h$ is the
difference of the two second fundamental forms restricted to the common tangent plane, in the common
normal direction. Congruence preserves signature (Sylvester), so *definiteness, indefiniteness and
degeneracy of $H_h$ are exactly the intrinsic A₁ contact conditions* and are chart-independent, even
though $H_h$ itself is not.

**Evaluation rule (correction).** All interval evaluations of $\nabla h$, $H_h$, $P$, $\lambda$ must be
performed over $\hat Y\times Z$, the certified graph enclosure, **not** over $B=Y\times Z$. Over the
raw box $\det A$ may be well separated while the Hessian enclosure is uselessly wide, and worse, the
resulting bound would not be a bound on $H_h$ at all — $H_h$ is only defined on the graph.

### 2.6 Chart deflation

**Theorem T1.4 (deflation determinant).** Let $x_\ast$ be a critical point ($G(x_\ast)=0$,
$\nabla h(z_\ast)=0$). Then, with rows ordered $(G_1,G_2,M_1,M_2)$ and columns $(y_1,y_2,z_1,z_2)$,

$$\boxed{\ \det DT(x_\ast) \;=\; \det A(x_\ast)^3\,\det H_h(z_\ast).\ }$$

*Proof.* $\det DT = \det A\cdot\det(\Sigma)$ where $\Sigma = D_zM - D_yM\,A^{-1}G_z$ is the total
derivative of $z\mapsto M(\phi(z),z)$. By T1.1, $M_j\!\circ\! c = (\det A\!\circ\! c)\,\partial_{z_j}h$, so

$$\Sigma_{jk} = \partial_{z_k}(\det A\circ c)\cdot\partial_{z_j}h \;+\; (\det A\circ c)\,\partial^2_{z_kz_j}h .$$

At $x_\ast$ the first term vanishes, giving $\Sigma = \det A\cdot H_h$ and hence
$\det DT = \det A\cdot(\det A)^2\det H_h$. ∎

Therefore **an A₁ contact point (definite or indefinite) is a simple root of $T$.** The conditioning
certificate — $\rho = \|I - C\,DT(B)\|<1$, $\eta=\|C\,T(c)\|$, enclosure radius
$r\le \eta/(1-\rho)$ — is attached to the *simple* root of $T$, never to the double root of $F$. This
is exactly the objection the original draft raised against shrinking a tolerance on $F$, and it is
answered structurally rather than numerically.

> **Remark (do not misuse T1.4).** The identity holds **only at critical points**; the extra term in
> $\Sigma$ is nonzero elsewhere. It is never evaluated as an interval predicate. Krawczyk uses the
> honest interval $DT(B)$. T1.4 is used *only* in the termination argument (§2.10): it shows that
> $\det DT(x_\ast)\ne0$ whenever $\det A(x_\ast)\ne0$ and $H_h(z_\ast)$ is nondegenerate, hence by
> continuity $\det DT\ne0$ on a neighbourhood, hence Krawczyk eventually contracts on small enough cells.

### 2.7 Transversality by critical-point exclusion

This is the addition the response missed, and it is the cheapest and most-used branch of the cascade.

**Theorem T1.5 (transversality without Hessians).** Assume (H-graph). Then:

1. If the system $(F,M_1,M_2)$ — five equations — has **no** root in $B$, then
   $\operatorname{rank}DF=3$ at every point of $F^{-1}(0)\cap B$: the box is transversal.
2. If Krawczyk certifies that $T$ has a **unique** root $x_\ast$ in $B$, and additionally
   $0\notin \hat f(X_\ast)$ for the root enclosure $X_\ast$, then again $\operatorname{rank}DF=3$ on
   all of $F^{-1}(0)\cap B$: the box is transversal.

*Proof.* (1) A point of $F^{-1}(0)\cap B$ with rank 2 would satisfy $F=0$ and, by T1.2, $M_1=M_2=0$.
(2) $F^{-1}(0)\cap B$ corresponds to $\{h=0\}$. The hypothesis $0\notin\hat f(X_\ast)$ certifies
$h(z_\ast)\ne0$, so $z_\ast\notin\{h=0\}$; and $x_\ast$ is the *only* critical point, so
$\nabla h\ne0$ on $\{h=0\}$. Apply T1.2. ∎

Two consequences worth stating plainly:

- Part (1) is a **strictly sharper transversality predicate** than separating a $3\times3$ minor from
  zero over the whole box: it requires only that five polynomials have no *common* zero, rather than
  that one polynomial be sign-definite. Boxes straddling an inflection of the intersection curve,
  where every individual minor changes sign but no actual tangency exists, are certified transversal
  by (1) and are not certified by minor separation. The price is (H-graph), which minor separation
  does not need; so minor separation is retained as the fallback when no chart is available.
- Part (2) means **the presence of a critical point in the box does not imply a tangency**, and
  detecting one costs no Hessian work. Only when the critical value is certified *not* separated from
  zero does the singular analysis begin. This is the branch that keeps the hot path cheap.

### 2.8 Interval definiteness with a usable modulus

For $\hat H = \begin{pmatrix}[\underline a,\overline a] & [\underline b,\overline b]\\
[\underline b,\overline b] & [\underline c,\overline c]\end{pmatrix}$, write $\beta = \max(|\underline b|,|\overline b|)$.

**Positive definiteness (sufficient).** $\underline a>0$ and $\underline a\,\underline c > \beta^2$.
**Negative definiteness.** Apply the above to $-\hat H$.
**Certified indefiniteness (needed for A₁⁻).** $\overline{\;\det\hat H\;} < 0$, i.e. the interval
evaluation of $ac-b^2$ is strictly negative.

**Modulus.** The isolation bound of T1.6 consumes an explicit $\mu$, which the definiteness test above
does not produce. Use either

$$\mu \;=\; \min(\underline a,\underline c) - \beta \qquad\text{(Gershgorin, cheap)}$$

or the sharper closed form

$$\mu \;=\; \tfrac12\Big[(\underline a+\underline c) - \sqrt{\max\big((\overline a-\underline c)^2,(\overline c-\underline a)^2\big) + 4\beta^2}\Big],$$

both valid lower bounds on $\lambda_{\min}(H_h(z))$ for all $z\in Z$. At dimension 2 there is no
reason to reach for general interval-matrix eigenvalue machinery.

### 2.9 The contact trichotomy at a certified critical point

**Theorem T1.6 (local structure).** Assume (H-graph) and that Krawczyk certifies a unique root
$x_\ast$ of $T$ in $B$, with enclosure $X_\ast$. Let $z_\ast$ be its $z$-part. Then:

**(a) Nonzero critical value.** If $0\notin\hat f(X_\ast)$: transversal (T1.5). If in addition $H_h$ is
certified positive definite on $\hat Y\times Z$ and $f>0$ on $X_\ast$, then $h$ is strongly convex on
$Z$ and $h\ge h(z_\ast)>0$, so

$$F^{-1}(0)\cap B=\varnothing .$$

If instead $f<0$ on $X_\ast$ with $H_h$ positive definite, $\{h=0\}$ is the boundary of a convex
sublevel set: $F^{-1}(0)\cap B$ is a single smooth transversal arc or loop, with any boundary on
$\partial B$. (Negative definite: mirror.)

**(b) A₁⁺ — isolated contact.** If $h(z_\ast)=0$ exactly (certified by §4) and
$H_h\succeq\mu I$ on $\hat Y\times Z$ with $\mu>0$, then for all $z\in Z$

$$h(z)\;\ge\;\tfrac{\mu}{2}\|z-z_\ast\|^2,$$

hence

$$\boxed{\ F^{-1}(0)\cap B=\{x_\ast\}.\ }$$

*Proof:* $Z$ is convex; Taylor with integral remainder and $\nabla h(z_\ast)=0$. ∎ (Negative definite:
$h\le-\tfrac\mu2\|z-z_\ast\|^2$, same conclusion.)

**(c) A₁⁻ — tangential node.** If $h(z_\ast)=0$ exactly and $\overline{\det \hat H_h(X_\ast)}<0$, then
$F^{-1}(0)\cap B$ is a 1-complex with a single 4-valent vertex at $x_\ast$ and no other singular point:
every other point of $\{h=0\}$ is regular because $x_\ast$ is the unique critical point of $h$, and near
$z_\ast$ the Morse lemma gives exactly two transverse arcs whose tangent directions are the null cone
of $H_h(z_\ast)$. Contact is tangential ($\operatorname{rank}DF(x_\ast)=2$) but the intersection curve
*crosses itself* rather than degenerating to a point.

**(d) Degenerate.** If $H_h(z_\ast)$ is singular, this is A₂ or higher; go to §2.10.

> **Why (c) matters.** The draft and the response both equate "A₁" with "isolated contact". In contact
> classification A₁ means only that the contact function has a Morse singularity; the definite case
> gives an isolated point and the indefinite case gives a node. The indefinite case is *at least as
> common* in practice — it is what a chamfer crossing a boss produces — and a classifier that emits
> `Unresolved` for it will emit `Unresolved` on a large fraction of real mating geometry. It costs one
> extra sign test to certify.

### 2.10 A₂: Morse–Bott branch by exact quadratic factorization

When the contact set is a curve, $T$ has a *critical manifold* of roots, so Krawczyk can never contract.
**That failure is diagnostic, not a dead end**: non-contraction of $T$ on cells that keep shrinking
without excluding is the signal to attempt the A₂ certificate.

**Theorem T1.7 (contact-factor certificate).** Suppose there exist polynomials $s,q,a,u_1,u_2$ with the
**exact** identity in $\mathbb Q[x]$

$$\boxed{\ s\,f \;=\; q^2 a \;+\; u_1G_1 + u_2G_2\ }$$

and the interval certificates $0\notin \hat s(B)$, $0\notin\hat a(B)$, and
$\operatorname{rank}D(G_1,G_2,q)=3$ on $B$ (by separation of one $3\times3$ minor). Then on $B$:

1. $F^{-1}(0)\cap B \;=\; \{G_1=G_2=q=0\}\cap B$, a regular 1-manifold;
2. $\operatorname{rank}DF=2$ at every point of it;
3. the contact order normal to the branch is exactly 2 (Morse–Bott).

*Proof.* (1) On $G=0$, $sf=q^2a$ with $s,a$ nonvanishing, so $f=0\iff q=0$; regularity is the rank-3
hypothesis. (2) Differentiate the identity and restrict to $\{G=0,q=0\}$, where $f=0$ and $q=0$:
$s\,df = u_1\,dG_1+u_2\,dG_2$, so $df\in\operatorname{span}\{dG_1,dG_2\}$ and
$\operatorname{rank}DF=\operatorname{rank}D(G_1,G_2)=2$. (3) In a chart, $h = (a/s)\,\tilde q^2$ with
$\tilde q(z)=q(\phi(z),z)$; the rank-3 hypothesis plus $\det A\ne0$ gives $\nabla\tilde q\ne0$ (same
Schur argument as T1.1), so $\tilde q$ is a submersion and the vanishing order of $h$ normal to
$\{\tilde q=0\}$ is exactly 2 since $a/s\ne0$. ∎

**Nonemptiness (correction).** T1.7 as stated is also satisfied vacuously when
$\{G=q=0\}\cap B=\varnothing$. The `A2Branch` verdict must additionally carry a certified nonempty
piece — a Krawczyk-verified point on $\{G=q=0\}$ inside $B$, or a certified crossing of $\partial B$.
Otherwise emit `Empty`.

**Continuation requires no new code.** Track

$$C=(G_1,G_2,q):\mathbb R^4\to\mathbb R^3$$

with the existing rank-3 parallelotope continuation. The A₂ path is an *adapter that desingularizes $F$
into an ordinary 3-equation branch*, not a singular continuation engine. No tangent-direction blow-up,
no degenerate chart tracking.

**Finding $q$.** In order of preference: (i) construction provenance — shared fillet spine, exact
seating constraint, common analytic surface; (ii) local repeated-factor extraction in
$\mathbb Q[x]/\langle G_1,G_2\rangle$ (the multiplier $s$ is exactly what localization needs); (iii)
give up and emit `Unresolved`. Note that *verification* is cheap and provenance-independent regardless
of how $q$ was found: one exact coefficient comparison plus three interval predicates.

### 2.11 The classifier cascade

| Stage | Certificate | Verdict |
|---|---|---|
| 0 | $0\notin \hat F_i(B)$ for some $i$ | `Empty` |
| 1 | chart search: $0\notin\det\hat A(B)$ + parametric Newton graph | activates 2–5; on failure fall back to legacy minor separation |
| 2 | $(F,M_1,M_2)$ has no root in $B$ | `Transversal` (T1.5.1) |
| 3 | Krawczyk: unique root $x_\ast$ of $T$; $0\notin \hat f(X_\ast)$ | `Transversal`; refine to `Empty` / `Loop` with definiteness (T1.6a) |
| 4 | unique $x_\ast$ + exact-zero witness + definite $\hat H_h$ | `A1Isolated` (T1.6b) |
| 5 | unique $x_\ast$ + exact-zero witness + $\overline{\det\hat H_h}<0$ | `A1Node` (T1.6c) |
| 6 | $T$ non-contracting + exact factor identity $sf=q^2a+uG$ + nonempty | `A2Branch` (T1.7) |
| 7 | otherwise | `Unresolved(κ, cell, slope)` |

**Verdict type (five-way):**

$$\text{Transversal}\ \mid\ \text{A1Isolated}\ \mid\ \text{A1Node}\ \mid\ \text{A2Branch}\ \mid\ \text{Unresolved}$$

with `Empty` as a degenerate case of the first.

**Theorem T1.8 (termination).** The refinement policy terminates on the first four verdicts.

*Proof sketch.* *Transversal:* continuity separates the relevant minor, or the five-equation system
excludes, on small enough cells. *A₁ (either signature):* $\det A(x_\ast)\ne0$ and $\det H_h(z_\ast)\ne0$
are open conditions, so by T1.4 $\det DT(x_\ast)\ne0$, hence $\det DT\ne0$ on a neighbourhood and the
deflated Krawczyk operator eventually contracts; the definiteness/indefiniteness interval tests are
continuity-separated on the same neighbourhood. *A₂:* $s,a$ and one $3\times3$ minor of $D(G,q)$ are
bounded away from zero near the branch, so all interval inequalities certify on small enough cells and
ordinary rank-3 continuation succeeds. Nothing is claimed for A₃⁺, indefinite-degenerate, or
factor-extraction failure — those are exactly `Unresolved`. ∎

---

## 3. Certificate data (T1)

```
Rank2Chart {
  pivot:        (component_pair, coord_pair),   // one of 18
  det_A_encl:   Interval,                        // 0 ∉ det_A_encl
  graph_encl:   Box2,                            // Ŷ ⊇ φ(Z)
  chart_minors: (BernsteinNet, BernsteinNet),    // M₁, M₂
  J, lambda:    evaluated over Ŷ × Z,
  hessian_eval: fn(Box) -> IntervalSym2,         // Theorem T1.3
}

A1Cert {
  chart:        Rank2Chart,
  krawczyk:     KrawczykEvidence<T = (G₁,G₂,M₁,M₂)>,   // rho, eta, X*
  hessian:      Definite{ sign, mu } | Indefinite{ det_upper_bound },
  zero_witness: ExactVanishingWitness,            // §4 — REQUIRED
  verdict:      A1Isolated | A1Node,
}

A2Cert {
  identity:     ExactVanishingWitness,            // s·f = q²a + u₁G₁ + u₂G₂
  s_encl, a_encl: Interval,                       // both exclude 0
  rank3:        MinorSeparation<D(G₁,G₂,q)>,
  nonempty:     BranchSeed | BoundaryCrossing,    // REQUIRED
  continuation: Rank3ParallelotopeCert<C=(G₁,G₂,q)>,
}
```

---

## 4. The shared exactness primitive

### 4.1 Why it is unavoidable

No floating or interval certificate can prove that a critical value is exactly zero. The families

$$h = z_1^2+z_2^2 \qquad\text{and}\qquad h = z_1^2+z_2^2+\varepsilon$$

are arbitrarily close in coefficient space; the first has an exact isolated contact, the second has
none. Exact tangency is intrinsically non-robust and no amount of subdivision changes that. The same is
true of exact carrier coincidence in T2 (§4.4). This is a property of the problem, not a deficiency of
the toolkit, and the spec should say so rather than hiding it in a tolerance.

### 4.2 One verifier, two uses

Both exactness obligations reduce to the same shape:

> **ExactVanishingWitness.** Polynomials $s, w_1,\dots,w_m$ (and optionally $q,a$) with the exact
> identity in $\mathbb Q[x]$
> $$s\cdot f \;=\; q^2a \;+\; \sum_{i=1}^m w_i P_i$$
> together with interval certificates $0\notin\hat s(B)$ and (when $q$ is present) $0\notin\hat a(B)$.

- **A₁ instantiation:** $P=(G_1,G_2,M_1,M_2)$, no $q^2a$ term. Then at the certified root $x_\ast$ of
  $T$ all $P_i$ vanish, so $s(x_\ast)f(x_\ast)=0$ and $s(x_\ast)\ne0$, giving $f(x_\ast)=0$, i.e.
  $h(z_\ast)=0$ **exactly**. This is precisely ideal membership of $f$ in the saturation of
  $\langle G_1,G_2,M_1,M_2\rangle$ by $s$.
- **A₂ instantiation:** $P=(G_1,G_2)$, with the $q^2a$ term. This is T1.7.

Verification in both cases is: expand both sides in a common basis over $\mathbb Q$ and compare
coefficients, plus one or two interval nonvanishing tests. The verifier is small, total, and
provenance-independent.

> This replaces the response's split between a `Structural(...)` fast path and an `ExactAlgebraic(...)`
> RUR fallback. Provenance and RUR are two *producers*; there is one *verifier*. Keeping the verifier
> single is what keeps the trusted core small.

### 4.3 Producers

1. **Construction provenance** (fast path, expected to dominate): shared fillet data, exact seating
   constraints, common analytic construction. In a construction-graph kernel where geometry is
   referenced by construction identity, the witness is often already implied by the graph — a
   deliberately mated pair *knows* it is mated.
2. **Exact ideal membership / normal form** over $\mathbb Q$: compute a normal form of $f$ modulo
   $\langle P\rangle$ localized at $s$.
3. **Local RUR / triangular decomposition** for the isolated simple root of $T$, then exact sign or
   equality evaluation of $f$ at that algebraic point.

Producers 2–3 run only on cells that reach stage 4–6 with an inconclusive sign, which is rare.

### 4.4 The same primitive is required by T2

Certifying that two separately constructed faces lie on a *common carrier* is the same kind of
statement: an exact algebraic coincidence that no enclosure can establish. The spec therefore uses one
`ExactCoincidenceWitness` notion across both parts — provenance-first, exact-algebraic fallback —
rather than treating "certified common carrier $C$" as a free hypothesis in T2, which is how the draft
states it.

---

## 5. Part II — The exact-contact calculus (T2)

### 5.1 Terminology correction

"Measure-zero fragment" is a misnomer. A coplanar overlap patch or a shared wall has *positive
two-dimensional measure*; what it lacks is a well-defined **volume parity**, because it lies on the
boundary of both operands and therefore has no interior point of either operand to seed from. The
correct name is **carrier-coincident fragment**, and the correct statement of the gap is: the
seed-and-propagate parity classifier is undefined on fragments whose interior meets no full-dimensional
cell of either operand.

### 5.2 Carriers, atoms, and hypotheses

**Definition.** A **carrier** $C$ is a certified common surface for two or more operand faces, with a
chosen **carrier normal** $n_C$ and a chart. Operand faces contribute **trim domains** $D_A, D_B,\dots$
in the carrier chart.

**Definition (arrangement and atoms).** The **carrier arrangement** is the certified arrangement of all
trim-domain boundary curves *and* all tangential contact curves incident to $C$ — including the
`A2Branch` curves produced by T1. Its 2-dimensional open cells are the **atoms**.

**Hypothesis (H-atom).** For each atom $\alpha$ there is $\varepsilon_0>0$ such that for every
$p\in\alpha$ and every $0<\varepsilon<\varepsilon_0$, the probe points

$$p^{\pm} = p \pm \varepsilon\, n_C$$

lie in the interior or the exterior — never on the boundary — of every operand.

(H-atom) is exactly why tangential contact curves must be strata of the arrangement: a curve along
which an operand boundary touches $C$ without crossing it is a locus where the side bits change, and
omitting it breaks constancy on the atom. **This is the structural link between T1 and T2**: T1's A₂
output is a required input to T2's arrangement.

All operands are assumed **regular closed** and all operations **regularized**
($A\cup^*B=\overline{\operatorname{int}(A\cup B)}$, etc.), which is the standard solid-modelling
setting in which regular closed sets form a Boolean algebra.

### 5.3 Side signatures

**Definition.** For a solid $S$ and an atom $\alpha$,

$$\sigma_S(\alpha) = (m_S^-, m_S^+)\in\{0,1\}^2,\qquad
m^{\pm}_S = \mathbb 1[\,p^{\pm}\in\operatorname{int}S\,].$$

**Lemma T2.1 (well-definedness).** Under (H-atom), $\sigma_S(\alpha)$ is independent of $p\in\alpha$
and of $\varepsilon<\varepsilon_0$.

The four states: $00$ = exterior on both sides, $11$ = interior on both sides (the carrier is internal
to $S$, not a face of it), $10$ and $01$ = oriented boundary.

**Implementation note.** $m^\pm$ need not be computed by any new predicate: they are exactly the parity
labels the existing classifier already assigns to the two full-measure fragments adjacent to $\alpha$
across $C$. The carrier-coincident decision is *derived from data the classifier already produces*.

### 5.4 The algebra

$$\sigma_{A\cup B}=\sigma_A\vee\sigma_B,\qquad
\sigma_{A\cap B}=\sigma_A\wedge\sigma_B,\qquad
\sigma_{\neg A}=\neg\sigma_A,\qquad
\sigma_{A\setminus B}=\sigma_A\wedge\neg\sigma_B,$$

all coordinatewise.

**Theorem T2.2 (germ homomorphism).** Under (H-atom), $\sigma(\cdot)(\alpha)$ is a homomorphism of
Boolean algebras from regular closed sets with regularized operations onto $\mathbf2\times\mathbf2$.

*Proof.* Each bit is membership of a fixed point $p^\pm$ in the interior of the operand. (H-atom)
places $p^\pm$ off every operand boundary, so regularization does not move it across any boundary:
$p^\pm\in\operatorname{int}(A\cup^*B)\iff p^\pm\in\operatorname{int}A$ or $p^\pm\in\operatorname{int}B$,
and likewise for $\cap^*$ and $\neg^*$. Membership of a fixed point is a Boolean homomorphism to
$\mathbf 2$; two independent points give $\mathbf2\times\mathbf2$. ∎

### 5.5 Boundary extraction, and the absence of discretion

$$00,\ 11 \;\longrightarrow\; \text{Drop},\qquad
10 \;\longrightarrow\; \text{Keep, canonical orientation},\qquad
01 \;\longrightarrow\; \text{Keep, flipped},$$

equivalently: $m_R^-=m_R^+ \Rightarrow$ discard, else keep with $\mathrm{flip}=\neg m_R^-$.

**Theorem T2.3 (soundness against parity — the draft's property 1).** The carrier-coincident decision
is not merely *consistent with* the parity decisions on adjacent full-measure fragments; it is the
*restriction of parity to the two germs adjacent to the carrier*. The result $R$ has boundary on
$\alpha$ precisely when $m_R^-\ne m_R^+$, and its outward orientation is determined by which bit is 1.

*Proof.* Immediate from T2.2 and the definition of the boundary of a regular closed set. ∎

**Corollary T2.4 (no discretionary choice — corrects the draft's `decide` signature).** On an atom, the
geometry is the shared carrier itself; there is no A-versus-B choice to make. The draft's four-valued
$\mathrm{decide}\in\{\text{keep }A,\ \text{keep }B,\ \text{keep both},\ \text{drop}\}$ is the wrong
shape. The actual codomain is

$$\{\,\text{Drop},\ \text{Keep(canonical)},\ \text{Keep(flipped)}\,\},$$

and it is a *function of the signature*, not a policy.

**Consequently `KeepBothSplit` must never mean "emit two coincident faces."** It means: *emit disjoint
carrier-domain atoms derived from the common refinement of the input domains.* For the coincident
geometry itself, emit one canonical geometry record carrying a provenance set. This removes a large
source of evaluation-order dependence at the representation level, before any theorem about ordering is
needed.

### 5.6 Domain containment without a case explosion

Convert the trim domains on a common carrier into a canonical arrangement. For two domains the atoms
are $D_A\cap D_B$, $D_A\setminus D_B$, $D_B\setminus D_A$; signatures are constant on each, and §5.4
applies pointwise. If $D_A\subseteq D_B$ only two atoms exist.

Containment stays cheap: certify that the relevant trim boundaries do not cross (interval separation on
the boundary curves' parameter boxes; dyadic-exact where the chart is dyadic), then one certified
interior seed determines inside/outside for a connected domain. Only when boundaries touch or overlap
is the full certified curve arrangement invoked. **Containment determines *where* the bit operations
are evaluated; it never determines *what* they return.**

### 5.7 Composition and determinism

$L=\{0,1\}^2$ with coordinatewise $\wedge,\vee,\neg$ is the product Boolean algebra
$\mathbf2\times\mathbf2$. Hence associativity, commutativity, idempotence, distributivity, De Morgan
and complement laws all hold for $\cup$ and $\cap$ — verified by exhaustion over the 4-element carrier.

**Difference is not associative**, in $L$ as in sets: $(\sigma_A\wedge\neg\sigma_B)\wedge\neg\sigma_C
\ne \sigma_A\wedge\neg(\sigma_B\wedge\neg\sigma_C)$ in general (verified by exhaustion). This is
*correct behaviour*, not a defect: the record algebra must mirror the set semantics exactly, and
$(A\setminus B)\setminus C\ne A\setminus(B\setminus C)$ at the set level.

**Theorem T2.5 (the theorem that should replace the draft's property 2).** For every Boolean expression
tree $E$ over operands, with difference represented internally as $A\wedge\neg B$,

$$\boxed{\ \mathcal R(E_1\ \mathrm{op}\ E_2) \;=\; \mathcal R(E_1)\ \mathrm{op}_L\ \mathcal R(E_2).\ }$$

Moreover $\sigma$ is a function of the *denoted point set*, not of the tree; and the canonical common
refinement of carrier domains is itself independent of grouping. Therefore **two algebraically
equivalent trees normalize to the same record set**, which is exactly the determinism gate. Arbitrary
reassociation of subtraction is forbidden because it changes the solid — no representation could or
should be invariant under it.

The draft's requested "associativity of the extended record algebra including $\setminus$" is not a
theorem to be proved but a statement to be corrected.

### 5.8 The self-pair (the draft's property 3)

For every $\sigma$: $\sigma\vee\sigma=\sigma$, $\sigma\wedge\sigma=\sigma$, $\sigma\wedge\neg\sigma=00$.
Hence

$$A\cup A = A,\qquad A\cap A = A,\qquad A\setminus A=\varnothing,$$

and the landed idempotence pin is the $n=1$ instance. The consequences for the entry gate:

- If **operand identity** is certified (same construction node), rewrite before the sweep. The self-pair
  never enters SSI, so there is nothing to prove about intra-solid adjacency event classes.
- If two separately constructed solids are only *geometrically* equal, certify common carriers (§4.4),
  equal chart domains, and equal side signatures, and obtain the same result through the contact
  calculus.

This is strictly stronger than the draft's proposed obligation ("prove the intra-solid adjacency event
class the sweep refuses is exactly the class the calculus decides, with no residue"): that obligation
presumes the self-pair must go through the sweep at all, which the algebra shows it need not.

### 5.9 Derived truth rows

Not a specification — these fall out of §5.4–5.5. Relative to a fixed carrier orientation:

| $\sigma_A$ | $\sigma_B$ | op | result | action |
|---|---|---|---|---|
| 10 | 10 | $\cup$ | 10 | keep one canonical face |
| 10 | 10 | $\cap$ | 10 | keep one canonical face |
| 10 | 10 | $A\setminus B$ | 00 | drop |
| 10 | 01 | $\cup$ | 11 | drop shared wall (butt join) |
| 10 | 01 | $\cap$ | 00 | drop |
| 10 | 01 | $A\setminus B$ | 10 | keep $A$, unflipped |
| 11 | 10 | $A\setminus B$ | 01 | keep carrier, flipped |
| 00 | 10 | $\cup$ | 10 | keep $B$ |

**Butt-join union.** The common wall carries anti-oriented states $10$ and $01$; union gives $11$, so
the wall is certified internal and discarded. The recorded 10-cosmetically-split exterior faces may
remain split — T2 does not need to merge them to make the Boolean certified. Face coalescing on a
common carrier is a separate, purely cosmetic pass.

**Scoping correction.** The recorded exact-footprint halfspace refusal is *not* a T2 contact refusal:
that case has no coplanar cap pair and fails because cutter walls terminate inside the plate, requiring
interior-loop rewrite machinery. It should not be booked as a T2 battery row. T2 closes
coincident/shared-wall classification only.

---

## 6. What remains genuinely open

1. **Producing** the exact witnesses of §4 outside the provenance fast path — normal-form and RUR
   computations over $\mathbb Q$, and their cost on real Bernstein degrees.
2. **A₃ and higher contact** (cusps, degenerate-indefinite critical points) — deliberately
   `Unresolved`. Closing these needs genuinely more singularity theory and is out of scope.
3. **Non-manifold carriers shared by $\ge3$ operands.** The algebra generalizes trivially (it is a
   Boolean function of $n$ bit-pairs), but the *arrangement* and its certification do not; the atom
   decomposition is where the work is.
4. **Tangential carrier contact of positive codimension** — the case where operands touch along a
   carrier only on a curve. (H-atom) requires those curves be strata; producing them requires T1's
   `A2Branch` to be complete on the incident faces, so T1 gates T2 here.
5. **Coalescing** cosmetically split coplanar faces after the Boolean — cosmetic, but it interacts with
   provenance-set merging.

---

## 7. Implementation surface

**T1 additions (all hot-path work is small):**

- `Rank2Chart` — the $2\times2$ pivot, the two chart minors $M_1,M_2$, the graph enclosure, $J$,
  $\lambda$, and the reduced-Hessian evaluator of T1.3. Second-derivative Bernstein control nets are
  needed; the existing derivative-net generator extends directly.
- Reuse of the **existing** Krawczyk implementation on the square system $T=(G_1,G_2,M_1,M_2)$.
- Reuse of the **existing** rank-3 parallelotope continuation on $C=(G_1,G_2,q)$ for A₂.
- `ExactVanishingWitness` verifier (§4.2) — exact coefficient comparison plus interval nonvanishing.
- A $2\times2$ interval definiteness/indefiniteness test with the $\mu$ bound of §2.8.

**T2 additions:**

- `SideState(u8) ∈ {00,01,10,11}`; classification is two bitwise operations plus boundary extraction.
- Carrier arrangement into atoms, with tangential contact curves included as strata.
- `KeepBothSplit` redefined as *disjoint atoms*, with a single canonical geometry record plus a
  provenance set.
- Entry-gate rewrite for certified-identical operands (§5.8), replacing the current refusal.

**The two results to put in the spec verbatim:**

$$\det D(G_1,G_2,M_1,M_2)\big|_{\nabla h=0} \;=\; \det(D_yG)^3\,\det(D^2h)$$

$$sf=q^2a+u_1G_1+u_2G_2,\quad sa\ne0,\quad \operatorname{rank}D(G,q)=3
\;\Longrightarrow\; F^{-1}(0)=\{G=q=0\},\ \operatorname{rank}DF=2$$

Together with T1.5 (transversality by critical-point exclusion, which needs no new machinery at all),
these remove nearly all of the singular-SSI apparatus the original draft anticipated. The only heavy
fallback left is exact algebraic witness production, and it runs only on the cases where exact contact
is inherently uncertifiable from floating enclosures.

---

## 8. Errata index

**Against the original T1/T2 draft**

| # | Claim | Correction |
|---|---|---|
| D1 | "three $3\times3$ minors of $DF$" | four ($3\times4$ matrix) |
| D2 | "prove $\forall x\in B$: all minors vanish" | false for A₁ (rank drops at a point only); holds only on a curve for A₂. Replaced by T1.2 |
| D3 | "augment $F$ with a scalar $H$ to get a simple root" | impossible: $\operatorname{rank}D(F,H)\le 3$. Replaced by T1.4 |
| D4 | "associativity of the record algebra including $\setminus$" | $\setminus$ is not associative on sets; replaced by T2.5 |
| D5 | "measure-zero fragment" | positive 2-D measure; what is missing is volume parity (§5.1) |
| D6 | `decide ∈ {keep A, keep B, keep both, drop}` | wrong codomain; no A-vs-B choice exists (T2.4) |
| D7 | A₁ isolation certifiable by interval methods | non-generic; requires an exact witness (§4.1) |
| D8 | "certified carrier $C$" taken as given in T2 | same exactness problem as D7; shares the §4 primitive |

**Against the review response**

| # | Claim | Correction |
|---|---|---|
| R1 | $\det DT=\det A^3\det H_h$ used loosely | pointwise at critical points only; never an interval predicate (§2.6 Remark) |
| R2 | Hessian hulled "over $B$" | must be over the graph enclosure $\hat Y\times Z$ (§2.5) |
| R3 | A₁ = definite case only | indefinite Morse case (A₁⁻, a node) missing and generic (§2.9c) |
| R4 | Deflation used only to reach A₁ | its strongest use is transversality with no Hessian at all (T1.5) — and it sharpens the existing rank-3 test |
| R5 | Two separate exactness fallbacks (`Structural` / `ExactAlgebraic`) | one verifier, two producers, two instantiations (§4.2) |
| R6 | A₂ certificate as stated | is vacuously satisfiable; needs a certified-nonempty branch (§2.10) |
| R7 | A₂ contact order 2 | needs $\nabla\tilde q\ne0$, which follows from rank 3 + chart; stated in T1.7(3) |
| R8 | Chart assumed available | made Lemma T1.0 with a bounded 18-way search |
| R9 | PD test $\underline a>0,\ \underline c>\beta^2/\underline a$ | correct but yields no $\mu$; explicit $\lambda_{\min}$ bounds added (§2.8) |
| R10 | Probe-point argument informal | formalized as (H-atom); homomorphism holds only under it (§5.2, T2.2) |
| R11 | "certified common carrier" | is itself non-generic; shares §4 (§4.4) |
| R12 | "$\pm\det A^3\det H_h$" | exactly $+$ under the row/column ordering of T1.4 |

---

## 9. Verification record

Symbolically verified on random rational polynomial systems in $\mathbb Q[y_1,y_2,z_1,z_2]$:

- T1.1, the chart-minor identity — residual exactly 0 for both $j$.
- The Schur decomposition $\Sigma_{jk}=\partial_k(\det A\circ c)\,\partial_j h+(\det A\circ c)\,H_{h,jk}$
  and $\det DT=\det A\cdot\det\Sigma$ — exact at random rational points.
- T1.3, symmetry and correctness of the constrained Hessian.
- T1.4 on an explicit constructed instance ($h=3z_1^2-5z_2^2$, $\det A=1$): $\det DT=-60$,
  $\det A^3\det H_h=-60$.
- §5.4–5.7 by exhaustion over all $4^3$ triples: $\cup,\cap$ associative, distributive, De Morgan,
  self-pair identities hold; $\setminus$ non-associative; all eight truth rows of §5.9 reproduced.
