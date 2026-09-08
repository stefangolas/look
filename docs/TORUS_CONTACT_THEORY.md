# TORUS-CONTACT THEORY — formal architecture v2 (frontier-reviewed, adopted 2026-09-08)

**Status:** supersedes the v1 draft (same doc, committed `9489f29`). The
frontier review returned corrections and proofs; ALL are adopted. Headline
changes from v1: (1) the torus implicit is stated in the SCALE-INVARIANT
T-form (no axis normalization); (2) the Villarceau predicate in the v1
draft was WRONG — the correct classification is PROVED (completeness
included) via the factorization ansatz; (3) the Bernstein-termination
claim is corrected to off-locus termination; (4) N3 is narrowed to N3*
(Gauss-map folds only); (5) TOR-C gains a 2-D implicit-pullback fast path
ahead of the 4-D atlas; (6) TOR-B's quartic/parity/tangency formalization.

## 0. The substrate: scale-invariant torus form

Ring torus, center `O`, axis vector `a` (NOT normalized — exact corpus
data may have non-unit axes and clearing the normalization keeps
everything polynomial), `K = a·a`, `q = x − O`:

```
Φ(q) = K(q·q + R² − r²)² − 4R²(K(q·q) − (q·a)²) = 0        (T)
```

Scaling `a` multiplies `Φ` by a square: zero set and sign invariant. For
`R > r`: `Φ < 0 ⟺ inside the solid torus`. This polynomial is the exact
substrate for everything below.

## 1. TOR-A — the section classification, PROVEN complete

Plane `Π: n·(x − O) = d` (exact `n, d`); `s = n·a`, `H = K(n·n) − s²`.

**Theorem N1 (complete circle-section classification).** For a ring torus,
every real circle component of a torus-plane section forces exactly one
of:

| Case | Exact predicate | Emitted loci |
|---|---|---|
| axis-perpendicular | `H = 0` | `ρ± = R ± √(r² − h²)` coaxial circles; `Δ < 0` ⇒ Empty; `Δ = 0` ⇒ one double circle |
| axial | `d = 0, s = 0` | two profile circles, center `O ± Rw`, radius `r` |
| Villarceau | `d = 0, H > 0, s ≠ 0, (R² − r²)H = r²s²` | two circles, center `O ± rw` (`w ∝ a × n`), **radius `R`**, meeting at the two bitangency points |
| general | otherwise | degree-4 plane algebraic section — traced, never approximated |

Every predicate is a polynomial equality/inequality in the original data.
**The completeness proof** (frontier review, adopted): any circle component
forces the factorization ansatz `(S + ℓs + mt + n₁)(S − ℓs − mt + n₂)` (the
section quartic (2) has no cubic terms); matching `st` gives `ℓm = 0`;
matching `s² − t²` gives `m² − ℓ² = 4R²a₀²`; for `a₀ ≠ 0` this forces
`ℓ = 0`, then the `s`-linear coefficient forces `h·a₀c₀ = 0`, splitting
into the axial (`c₀ = 0, h = 0`) and Villarceau (`c₀ ≠ 0, h = 0`,
`c₀² = (R²−r²)/R²`, `a₀² = r²/R²`) cases; `a₀ = 0` is axis-perpendicular.
No fourth family exists. The implementing worker machine-checks the C1
factorization identity as a test and the runtime factorization re-decides
every instance (the theorem promises census completeness only).

Emission detail for the Villarceau case: in bitangent-plane orthonormal
coordinates the factorization is
`((ξ−r)² + η² − R²)((ξ+r)² + η² − R²) = 0` — the two circles meet at the
bitangency points, emitted as exact branch-node events.

**Exact aligned-degeneracy predicates (BG-ANA class):** for axis-parallel
planes, the critical comparisons `d² ⋛ (R∓r)²(n·n)` decide the
two-loop/inner-tangent/one-loop/outer-tangent/empty regimes exactly —
topology information for the tracer even though noncentral instances stay
quartic.

**Fallback:** every plane restriction has exact degree 4 (leading term
`K‖su+tv‖⁴ ≠ 0`) — the residual case is a genuine quartic plane curve,
traced.

## 2. TOR-B — ray-quartic crossings, formalized

Ray `q(t) = q₀ + t·v`, `t ≥ 0`. Coefficients (6) of `g(t) = Φ(q₀+tv)`:
`g₄ = K‖v‖⁴ > 0` — always a genuine quartic; all coefficients exact
rational/algebraic operations on carrier + ray data.

- **Parity classifier (7):** `q₀ ∈ int(T) ⟺ #{positive roots of odd
  multiplicity} ≡ 1 (mod 2)` (sign argument: `g → +∞`, `g(0) < 0` iff
  inside).
- **Oriented BRep statement (8):** `χ_S(q₀) = Σ sgn(N_i·v)` over positive-t
  crossings with outward normals — entering −1, exiting +1; mod 2 this is
  the ordinary crossing rule. No new classifier theorem: TOR-B needs only
  a torus implementation satisfying the existing ray-event contract.
- **Tangency is algebraic (9):** `∇Φ = 4KSq − 8R²(Kq − (q·a)a)`; the exact
  tangency predicate at an isolated root is `g(t*) = 0 ∧ g'(t*) = 0`. No
  numerical normal. Higher contact: compare certified flank signs — odd
  multiplicity crosses, even does not ("tangent = noncrossing" is NOT
  assumed).
- **FSSI-EXT is dispatch-only:** if `classify.rs` proves correctness from
  the face-intersection contract (sound + complete + exact crossing bit +
  seam dedup), then `Carrier::Torus => ray_torus_quartic(...)` carries no
  new classifier argument — the proof obligation is local: (6) finds every
  hit, the trim predicate selects face hits, (7)/(8) supplies the
  contribution.

## 3. TOR-C — the atlas, the degree bound, and the 2-D pullback fast path

**The rational atlas (N2, proven).** Homogeneous circle parametrization
`C_u = u₀² − u₁²`, `S_u = 2u₀u₁`, `W_u = u₀² + u₁²` per angle; the torus:

```
X̂ = A_vC_u,  Ŷ = A_vS_u,  Ẑ = rS_vW_u,  W_T = W_uW_v,  A_v = RW_v + rC_v
```

with **multidegree (2,2)** — rigid placement does not raise it. Atlas
completeness: `P¹ × P¹` covered by four affine chart products; the torus
map is one-to-one in angular data for `R > r` modulo periodic
identification. Chart transitions are rational (`u′ = 1/u`; at the seam
`u = ±1`: `u′ = ±1`, `du′/du = −1` — position, branch germ, and tangent
handoff all exact) and are FSSI-003 region-transition events.

**Torus×spline degree bound (12).** Clearing denominators on
`T(u,v) = S(s,t)` gives `H_i = W_S·T̂_i − W_T·Ŝ_i = 0`, `i ∈ {x,y,z}`, of
multidegree **(2,2,p,q)** — no roots introduced. Bernstein storage:
`27(p+1)(q+1)` coefficients across the three equations.

**Theorem C specialization (13).** `rank DH = 3 ⟺ N_T × N_S ≠ 0`; the
torus normal in the atlas is the bi-(2,2) polynomial
`N̂_T = (C_uC_v, S_uC_v, S_vW_u)` (14) — **ADM-002 is not rebuilt for the
torus**; the spline side uses the generic normal-cone machinery, the torus
side supplies the closed-form polynomial normal direction.

**The 2-D implicit pullback (the fast path — exploit aggressively).**
Substitute the spline's homogeneous coordinates `(X, Y, Z, W)` into the
homogeneous implicit:

```
P(s,t) = (X² + Y² + Z² + (R²−r²)W²)² − 4R²W²(X² + Y²) = 0     (15)
```

`P = 0` is the exact torus/spline intersection in the spline's 2-D domain,
degree **(4p, 4q)** — the "(4p,4q) route" is trivial polynomial
composition. Regularity on the pullback is exactly Theorem C:
`∇P ≠ 0 ⟺` the normals are non-parallel at the intersection (16).

**Solver hierarchy (measure-first where the two routes compete):**

1. spline-span BVH/AABB reject;
2. 2-D `(4p,4q)` implicit pullback (15) — trace the candidate curve, use
   (16) for regularity;
3. 4-D atlas (11)/(12) only where torus-UV certification, trim-coordinate
   recovery, seam interaction, or an ambiguous 2-D candidate requires it;
4. singular/tangent path: the FSSI tangency machinery with the two
   parabolic circles supplied as exact N3* strata.

Coefficient comparison at cubic×cubic: pullback `(4p+1)(4q+1) = 169` vs
atlas `27·4·4 = 432` — before the cost of 4-D subdivision. This does not
prove the pullback wins (higher Bernstein degree may weaken exclusion) —
**"measure it" is exactly the right policy**, per the review.

**Bernstein termination (corrected theorem).** Off-locus termination: for
every compact 4-D region `B` disjoint from the zero set `Z`, sufficiently
fine subdivision rejects every descendant box by the Bernstein sign hull
of some `H_i` (continuity + coefficient convergence + compactness).
Boxes meeting `Z` are never excluded — they are certified via
transversality + the regular-curve machinery. Bounded degree alone does
NOT give termination on `Z`; the v1 wording is corrected here.

## 4. N3* — the narrowed fold stratum

The torus Gauss map loses rank exactly on `cos v = 0` — the two parabolic
circles `ρ = R, z = ±r` (17), with simple rank loss (`d(cos v)/dv ≠ 0`).

**N3\*:** fold certification defined by TORUS GAUSS-MAP / normal-map
degeneration pre-routes through these closed-form strata. **Generic
interaction-curve folds (coordinate/projection folds) do NOT
necessarily pass through parabolic points** — counter-example: the
horizontal-plane section's circles have projection folds at `u = 0, π`
with `cos v ≠ 0`. FSSI-002 keeps its generic fold search; the parabolic
circles enter only as pre-classified special strata. Solver soundness is
not weakened by assuming more.

## 5. Torus volume 2-form (feeds ADM-003's T2′)

With outward normal `N` and `dA = r(R + r cos v) du dv`,
`X·N = R cos v + r`:

```
V_T(D) = (1/3)∬_D r(R + r cos v)(R cos v + r) du dv              (18)
```

T2′/Theorem D consumes this exact face 2-form — no new volume theorem for
torus trims. The `v`-primitive `G(v)` reduces the trimmed-face
contribution to the oriented trim boundary exactly. Full-torus invariant:
`V = 2π²Rr²`.

## 6. Implementation hierarchy (final)

1. **TOR-A/N1:** exact constant-time predicates; axis/coaxial/Villarceau
   sections straight to `AnalyticIntersection::Circle`; aligned
   emptiness/tangency exact; residual planes → the quartic tracer.
2. **TOR-B/N4:** one fixed quartic per candidate face; SFC float roots →
   exact isolation → flank-sign certification. A cheap `classify.rs`
   extension.
3. **TOR-C regular fast path:** span BVH reject → 2-D pullback (15) →
   regularity via (16).
4. **TOR-C atlas certification:** (11)/(12) locally — trim coordinates,
   seams, ambiguous candidates.
5. **Singular/tangent path:** FSSI tangency machinery + N3* strata.
