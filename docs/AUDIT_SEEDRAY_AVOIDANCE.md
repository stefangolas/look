# Certified seed-ray edge/vertex avoidance — an AUDIT-FIRST survey (DEF-SEEDRAY-C)

| | |
|---|---|
| packet | DEF-SEEDRAY-C (survey; `class: survey`, `crates: [truck-shapeops]`) |
| status | DESIGN + MEASURED PROBE — nothing here changes `classify.rs` |
| implements | nothing (DEF-SEEDRAY-B owns the production criterion) |
| probe | `vendor/truck/truck-shapeops/tests/seedray_avoidance_probe.rs` (all `#[ignore]`) |
| run | `cargo test -p truck-shapeops --test seedray_avoidance_probe -- --ignored --nocapture` |
| anchors | A1 `3.0_f64` ×1, A2 `fn find_seed` ×1 in `boolean/classify.rs` (both hold) |

This document is the audit-first survey for the certified avoidance half of the
seed-ray fix: **which criterion can certify `dist(ray, edges ∪ vertices of ∂B)
≥ δ > 0` along the whole seed ray, and at what cost?** It states candidate
criteria with soundness arguments in theorem-sketch form (hypotheses explicit,
unproved claims marked OPEN), and it reports *measured probe evidence* from the
existing boolean fixtures. It deliberately does **not** implement the criterion:
`classify.rs` is DEF-SEEDRAY-B's, and changing any verdict is out of scope.

---

## 1. Problem and severity context

The seed-parity theorem (§12 of the constructive B-rep formal system) needs
the ray-seed winding computation to be exact: the seed bit for a connected
component is computed from **one** certified seed and then propagated across
the whole parity graph. The seed-parity theorem's working hypothesis is that
the ray's crossings of `∂B` are clean — no crossing lands on, or even near, an
edge or vertex of `∂B`, because a crossing near the trimmed-region boundary is
exactly what makes the float classifier return `Boundary`/`ambiguous`
(`classify.rs`: `classify_region` trichotomy and the `Boundary` band at
parameter distance `<= tol`; `ray_seed` treats any `Boundary`-classified
crossing as ambiguous and retries the next of the fourteen directions).

Today nothing measures or certifies this:

- the only guard is the float `classify_region → Boundary → ambiguous` at found
  crossings (`classify.rs:548-556`), a *post-hoc* tolerance test, not an
  avoidance certificate;
- the fourteen-direction table is **symbolic perturbation by retry** — a
  direction that lands on or grazes an edge is simply skipped and the next
  direction tried; nothing certifies that *any* direction is actually clean.

**Severity.** A wrong seed bit propagates across an ENTIRE component (§12):
one bad bit flips every fragment bit that reaches it through the parity
graph, which is the inverted-material defect class. The kernel's contradictory
component refusal (`Refusal::Contradictory(Prop::FragmentInsideOther)`,
`classify.rs:127-142`) catches only **inconsistent** components — a component
whose seed is *uniformly wrong* is internally consistent and passes clean. The
adjudicated parity analysis concentrates the wrong-answer risk here (the
through-edge double count is odd) and in dropped crossings; avoidance is the
part with **no landed machinery to adapt**, which is why it is audited before
it is built.

**Corpus correlation.** DEF-TESS-ANALYTIC-SEAM and this defect both surface on
analytic geometry (circles/cylinders where a seam or rim is exactly on a ray's
path). The through-edge double count and the seam defect are the same family:
an edge/vertex that a ray *passes through* instead of cleanly crossing. That is
why the probe corpus below is built from the analytic ray-seeded fixtures.

---

## 2. The requirement, made precise

Let the seed be `p ∈ ∂A` (a fragment-region representative on the *classified*
solid) and let the target be `∂B` (the *other* solid, `other_shell`). The ray
is the half-line

    r(t) = p + t·d,  t ∈ [0, ∞),  ‖d‖ = 1,

with the deterministic direction table `ray_directions()` (`classify.rs:474`).
The seed-parity computation needs, on the WHOLE ray:

> **H0 (avoidance).** There is a certified `δ > 0` with
> `dist(r([0,∞)), σ) ≥ δ` for every edge curve `σ` of `∂B` **and** for every
> vertex of `∂B`.

Terminology and traps that the criteria below must be precise about:

- **point-to-ray vs point-to-line.** The ray is a HALF-line. The distance from
  a curve point `q` to the infinite line `p + t·d` is `≤` the distance to the
  ray; a point whose closest line projection lies at `t < 0` is irrelevant to
  the ray but is *caught* by any line-based criterion. A conservative
  line-based certificate is sound but refuses directions whose only near point
  is behind the origin (measured below: the `b_behind` column). A precise
  half-line certificate must clamp `t ≥ 0`, which costs an interval projection
  of `d·(q − p)` per box.
- **the segment beyond `t_exit` matters.** The parity winding counts crossings
  on the whole half-line, and an edge far past the last crossing is still on
  the ray. Nothing lets a criterion stop at the last face crossing.
- **3-D distance vs the classifier's parameter band.** `classify_region`
  decides `Boundary` by *parameter-polygon* distance `<= tol` (H-3: the
  dimensionless tolerance class, `TOL = 1e-2`, on unit-scale witnesses). A 3-D
  edge-avoidance certificate of `δ` does not, by itself, bound the parameter
  distance of a crossing to the trimmed-region boundary; the link is a metric
  property of the carrier parametrisation. The survey states the criteria in
  the 3-D form the theorem needs and flags the parameter-band link as an open
  question (§7 O1).

---

## 3. Candidate criteria (each with a soundness sketch, hypotheses explicit)

### 3a. Per-edge subdivision + hull exclusion (Bernstein/interval discipline)

**Setup.** For one boundary edge curve `Cᵢ : [aᵢ,bᵢ] → ℝ³` of `∂B`, subdivide
its parameter domain; on each sub-box `I ⊂ [aᵢ,bᵢ]` an enclosure operator
`Hull(Cᵢ(I)) ⊇ Cᵢ(I)` (the Bernstein-hull / interval discipline, width
convergent) is compared with the ray operand.

**Distance formulation (precise).** For unit `d` the perpendicular residual

    g(t) = (Cᵢ(t) − p) × d

satisfies `‖g(t)‖ = dist(Cᵢ(t), L)`, the distance to the ray **line** `L =
{p + t·d}`. A certified lower bound on `‖g‖` over `Cᵢ(I)` is therefore a
certified lower bound on the *line* distance, which is `≤` the half-line
distance. Two sub-forms:

- **(a-LINE)** certify `dist(Cᵢ(I), L) ≥ δ`. Sound and conservative; may refuse
  a direction whose only near point is behind `p`.
- **(a-HALF)** certify the true ray distance by additionally enclosing the
  projection interval `d·(Cᵢ(I) − p)` and clamping at `t = 0`: the half-line
  distance over the box is the interval distance from `Cᵢ(I)` to the segment
  `[0, ∞)` in the `t`-coordinate. One extra interval projection per box.

**Theorem sketch (a).**
> *Hypotheses.* (H1) each boundary edge is a compactly parametrised `C¹`
> carrier in `𝒢` over a closed box; (H2) `Hull` is an enclosure with width
> convergence: `width(Hull(Cᵢ(I))) → 0` as `width(I) → 0` (BG-ENC-001/002);
> (H3) `δ > 0` fixed; (H4) outward rounding (BG-ENC-003) so the per-box lower
> bound is a true lower bound; (H5, only for a-HALF) the clamped projection is
> enclosed soundly.
>
> *Claim.* Subdividing until every box either certifies
> `dist(ray-operand, Cᵢ(I)) ≥ δ` or reaches width `< ε` terminates, and
> returns either a finite certified cover (every box labelled with its lower
> bound) or a refusal leaf.
>
> *Argument.* `f(u) = dist(ray-operand, Cᵢ(u))` is continuous on `[aᵢ,bᵢ]`. If
> `min f ≥ δ`, then the set `{u : f(u) < δ/2}` is empty and the negative margin
> is uniform; by width convergence the range of `f` over each sufficiently
> small box lies above `δ`, so a finite cover is certified (compactness gives
> termination of the "certify" branch). If `min f < δ`, no finite-depth cover
> can certify every box, so subdivision reaches the width budget and the leaf
> is a **refusal**. (For analytic edges — Line/Circle — the exclusion test is
> decidable exactly; for general carriers, subdivision depth is bounded only
> under an OB-3-style quantitative non-degeneracy hypothesis — see O4.)

**Certificate record.** `EdgeAvoided { edge, boxes: Vec<BoxCert>, cover:
Cover{ complete: true } }` where each `BoxCert` is `{ domain: I, dist_lb: f64
≥ δ, clamp: HalfLine|Line }`. The refusal is a typed leaf
`{ edge, box: I, width }`. The verdict "ray avoids this edge by ≥ δ" is
**unconstructible** without the per-box `dist_lb` values — a bare boolean
assertion is exactly the M2/M3/M6 defect class the audit adjudication bans
(verdict variants asserting more than their construction site proves).

**Refusal path.** Leaf → direction refused (retry next of 14). All 14 refused →
`NumericallyUnresolved` seed refusal (unchanged control flow).

**Budget / retry interaction.** Cost `O(edges(∂B) × subdivision depth)` per
seed-direction decision. The depth budget is per edge and refundable per
direction: a refused direction costs one table slot, never the seed (unless the
table exhausts). Depth interacts multiplicatively with edge count, which is the
knob the calibration section (§6) quantifies.

### 3b. Cross-product sign-stability (interval evaluation, no square roots)

**Criterion.** For each edge curve `Cᵢ` and each sub-box `I`, evaluate the
interval enclosure of the three components of `g(t) = (Cᵢ(t) − p) × d` over
`I`. If **some** component's enclosure strictly avoids 0 (sign-stable), the box
is excluded: no point of `Cᵢ(I)` lies on the ray line. If all three straddle
0, the box may contain a line point — subdivide; a non-excludable leaf is a
refusal ("a sign change = potential near-pass = subdivide or refuse").

**Theorem sketch (b).**
> *Hypotheses.* (H1) same carrier hypothesis; (H2) interval evaluation sound
> and outward-rounded; (H3) exclusion is decided by a strict margin, not by a
> bare sign test that a float at an exact zero can fool (measured trap below);
> (H4) a certified partition cover of each edge domain.
>
> *Claim SOUND.* Exclusion of every box ⟹ `Cᵢ ∩ L = ∅`, i.e. the edge is
> disjoint from the ray **line**: `dist(Cᵢ, L) > 0`.
>
> *Claim OPEN (do NOT rely on).* Sign-stability does **not** certify
> `dist ≥ δ`: it certifies non-intersection only, and the margin it can leave
> is unboundedly small. Concretely, if `Cᵢ` passes distance `ε` from the line
> with `ε` arbitrarily small, every component stays sign-stable and criterion
> (b) certifies the box at `ε` (measured: the ε-sweep below certifies a rim
> miss at `ε = 1e-4` while `δ_rel ≈ 1.7e-3`). To promote (b) to a `δ`
> certificate it must record, per excluded box, a magnitude lower bound
> (`√(Σ mig²)` over the component enclosures) and take the cover minimum — that
> is (a-LINE) again. Until then, treat (b) as a *pre-filter* / non-intersection
> certificate, not the δ certificate.
>
> *Claim SOUND (sub-form b-δ).* If each excluded box records
> `lb(I) = √(Σₖ mig(gₖ(I))²)` (mig over the outward-rounded component range)
> and the cover minimum satisfies `min_I lb(I) ≥ δ`, then `dist(Cᵢ, L) ≥ δ`
> with the same cover argument as (a).

**Measured float trap (why H3 is needed).** In the probe's first exact
implementation of (b) (closed-form trig range reconstruction) a ray exactly
through a rim circle was certified CLEAR: `cos(π/2)` is not `0` in `f64`, so
the reconstructed component range at the exact root box started at `~3e-17
> 0` and the box was *falsely excluded*. The probe now samples the actual curve
at the stationary candidates and pads the exclusion by `B_EPS = 1e-12`
(`seedray_avoidance_probe.rs`). Any production implementation needs the same
discipline (strict margin, not a bare sign test).

**Certificate record.** Per edge: `{ boxes: Vec<{ domain, sign_stable_component
: Option<k>, mig_lb: f64 }>, cover }` — the verdict `EdgeLineAvoided` is
unconstructible without either `sign_stable_component` + `mig_lb` (for b-δ) or
an explicit refusal leaf. A bare per-direction boolean is not a certificate.

**Refusal path / budget.** Same as (a). Cheaper per box than (a): no square
roots at decision time (only at leaf/record time for b-δ). On Line/Circle edges
(b) is *exact*: a refusal occurs iff the ray line meets the carrier (or the
leaf is within `B_EPS` of a tangency) — this is what makes its refuse-rate
measurable analytically, which the probe exploits.

### 3c. Slab / exclusion formulation (partition the ray's t-range)

**Criterion.** Partition the ray's `t`-range into slabs `[tⱼ, t_{j+1}]` with
`t₀ = 0` and a certified terminal slab `[T, ∞)`. Per slab, form the ray
slab-hull `Hull(p + [tⱼ,t_{j+1}]·d)` and certify box-disjointness against every
edge's hull box; a slab where some edge's hull intersects the ray slab-hull is
subdivided (in `t` or in the edge) or refused.

**Theorem sketch (c).**
> *Hypotheses.* (H1) compact edge domains; (H2) box disjointness certified by
> interval separation of hulls; (H3) a **certified terminal slab**: since
> `∂B`'s edge set is bounded, `max d·(q − p)` over `q ∈ edges(∂B)` is a finite
> `T`; for `t ≥ T` the ray point leaves every edge's hull box forever in the
> `d`-coordinate (certified by interval arithmetic on the hull boxes), so the
> tail slab `[T, ∞)` is certified disjoint in finitely many steps.
>
> *Claim.* A finite partition `{slabs}` with each slab certified disjoint from
> every edge hull box implies `dist(r, edges ∪ vertices) > 0`; with a margin
> recorded per slab it implies `≥ δ`.
>
> *Honest caveat.* Box-disjointness of *hulls* is the cheapest per step but the
> coarsest: an edge that runs almost parallel to the ray at distance `< δ`
> keeps its hull intersecting the ray slab-hull for many slabs, so (c)
> subdivides or refuses where (a) would certify. (c) is the cheap pre-filter;
> (a)/(b) are the per-edge deciders.

**Certificate record.** `SlabCover { slabs: Vec<{ t_range, per_edge: Vec<
Disjoint{ edge, margin }> }>, tail: TailSlab[T,∞] }`.

**Budget / retry.** Cheapest per step, more slabs; subdividing `t` interacts
with the table only through the per-direction budget.

### Vertex avoidance (the cheap half — spelled out)

Vertices are 0-dimensional: for a vertex `v ∈ ∂B`, the point-to-ray distance
`dist(r, v)` is interval-evaluable directly. For unit `d`:

    dist(r, v) = ‖ (v − p) − d·(v − p)·d ‖   if d·(v − p) ≥ 0,
                 ‖ v − p ‖                   otherwise.

Enclose both branches in interval arithmetic, take the certified lower bound;
certify `≥ δ`, else refuse the direction. Cost `O(|V(∂B)|)` per decision, no
subdivision, no square-root budget concern. The certificate is
`{ vertex, dist_lb ≥ δ, clamped: bool }`. This half is trivial; it is spelled
out because it is the part a "whole-ray avoidance" implementation is most
likely to forget (a line-grazing direction is caught by the edge criterion but
a *vertex*-grazing direction needs the vertex term — measured in the probe:
`vertex-graze`).

---

## 4. Probe and methodology (numbers cited below)

All evidence below is printed by
`vendor/truck/truck-shapeops/tests/seedray_avoidance_probe.rs` (six `#[ignore]`
tests). Methodology, verbatim from the probe header:

- Corpus = the `classify.rs` ray-seeded fixtures (disjoint / contained /
  ambiguous — tests 2, 3, 4) plus probe-constructed near configurations over
  the SAME shells. Contact-free fixtures are the corpus because only their
  components take rule-(b) ray seeds; components touching a contact arc seed by
  the arc-side rule and never cast a ray. The green register confirms each pair
  splits and classifies green under today's classifier.
- A "seed" is a rule-(b) representative point on one solid, measured against
  the OTHER solid's boundary over the whole half-line `t ≥ 0`.
- Edge curves in the corpus are Lines and Circles only. Distance to a Line
  edge is exact point-set distance between the ray half-line and the segment;
  distance to a Circle edge is the ray-to-segment distance over a uniform
  tessellation (sagitta `≤ r·(2π/2048)²/8 ≈ 1.2e-6·r`, disclosed).
- Criterion (b) is evaluated **exactly** on Line/Circle edges: a sub-box is
  excluded when some component of `g = (C − p) × d` strictly avoids 0 (sampled
  at the actual curve at the stationary candidates, padding `B_EPS = 1e-12`);
  subdivision is depth-capped at 24 (`leaf ≈ (t1−t0)·2⁻²⁴`). Refusal leaves
  record whether the near/crossing point is ahead of (`t_c > 0`) or behind
  (`t_c < 0`) the origin (the `b_behind` half-line caveat column).
- δ thresholds: `δ_abs = 1.0e-2` (the classifier tolerance class, H-3,
  dimensionless on the unit-scale witnesses) and `δ_rel = 1.0e-3 · scale`
  (scale = target bounding-box diagonal).

Numbers below are deterministic on a fixed build except the §T4 runtime rows,
which are labelled as sampled.

### Fixture green register

```
fixture-pair   faces  fragments  split-ok  classify(a,b)
disjoint         9       9        true        true
contained        9       9        true        true
ambiguous        9       9        true        true
```

All three corpus pairs are green today: every seed resolves, none is refused,
and no `NumericallyUnresolved` fires. This is the "formerly green" set the
doctrine floor question is about.

### T1 — direction-table hit profile (near-edge census per direction)

Corpus: 98 seed-direction decisions over 7 seeds (all 14 directions each).
`edge_hit`/`vert_hit`/`near` = direction whose half-line min distance to a
target edge/vertex/any is `< δ_abs = 1e-2`. `b_refuse` = criterion (b) refuses
the direction (ray LINE meets a Line/Circle carrier, or leaf within `B_EPS` of
one). `b_behind` = count of refused directions whose non-excludable leaves all
lie behind the origin (false positives for the half-line).

```
dir  vector                     tested edge_hit vert_hit  near b_refuse b_behind
0    +z(0,0,1)                      7      2       1       2      2        0
1    +x(1,0,0)                      7      0       0       0      0        0
2    +y(0,1,0)                      7      0       0       0      0        0
3    -z(0,0,-1)                     7      0       0       0      2        2
4    -x(-1,0,0)                     7      0       0       0      0        0
5    -y(0,-1,0)                     7      0       0       0      0        0
6    diag(0.5774,0.5774,0.5774)     7      2       0       2      2        0
7    diag(0.5774,0.5774,-0.5774)    7      0       0       0      2        2
8    diag(0.5774,-0.5774,0.5774)    7      2       0       2      2        0
9    diag(0.5774,-0.5774,-0.5774)   7      0       0       0      1        1
10   diag(-0.5774,0.5774,0.5774)    7      1       0       1      1        0
11   diag(-0.5774,0.5774,-0.5774)   7      0       0       0      2        2
12   diag(-0.5774,-0.5774,0.5774)   7      2       1       2      2        0
13   diag(-0.5774,-0.5774,-0.5774)  7      0       0       0      2        2
TOTAL                               98     9       2       9     18        9
```

Reads: near-edge configurations are **rare and direction-concentrated** (9/98
decisions, i.e. ~9% of seed-direction pairs), on `+z` (the vertical graze of a
rim) and the `+xy`-type diagonals (rays that pass through a rim point or a
block corner). The antipodal directions of those (`−z`, `−diag`) are refused by
criterion (b) only because the crossing lies on the negative half of the line —
**9 of the 18 refusals are half-line false positives** (`b_behind`). That
number is the measured price of a pure line-based criterion and motivates the
`t ≥ 0` clamp (a-HALF).

### T2 — criterion (b) refuse-rate on the Line/Circle subset (doctrine-floor input)

```
trial.seed                    target               decisions b_refuse b_clear census_near clear_dirs
disjoint.a_bottom(2,2,0)      edges(L=0,C=2,O=0)       14        0       14       0         14
disjoint.b_bottom(6,6,0)      edges(L=12,C=0,O=0)      14        2       12       1         12
contained.a_bottom(2,2,0)     edges(L=0,C=2,O=0)       14        0       14       0         14
contained.b_bottom(2,2,0.5)   edges(L=12,C=0,O=0)      14        0       14       0         14
ambiguous.a_bottom(2,2,0)     edges(L=0,C=2,O=0)       14        6        8       3          8
ambiguous.b_bottom(2.5,2,0.5) edges(L=12,C=0,O=0)      14        4       10       2         10
vertex-graze.a(3,2,0)         edges(L=0,C=2,O=0)       14        6        8       3          8

aggregate: 18/98 decisions criterion-(b)-refused (0.184); census-near 9/98 (0.092)
```

`clear_dirs` = directions that are BOTH criterion-(b) certified AND
census-clear at `δ_abs` — the certified path's surviving direction budget.
Every seed keeps **at least 8 of 14** certified-clear directions; no formerly
green seed would be refused under direction-retry semantics. The aggregate
refuse-rate (0.184) is therefore *not* a seed-level refusal rate: with the
direction-refusal semantics the doctrine floor is not violated on the corpus.

### T3 — per-direction rows in the near window

Rows shown when `min(edge,vertex) < 1e-1` or criterion (b) refuses.

```
trial=disjoint seed=origin=b_bottom(6,6,0) origin=(6.000000,6.000000,0.000000) target[edges(L=12,C=0,O=0) verts=24 scale=6.0000]
dir  vector                       edge_dist    vert_dist     min_dist   b_clear  b_leaves  b_behind
7    diag(0.5774,0.5774,-0.5774)  2.828427e0    2.828427e0    2.828427e0  false      3        3
12   diag(-0.5774,-0.5774,0.5774) 7.691851e-16  7.691851e-16  7.691851e-16 false      3        0

trial=ambiguous seed=origin=a_bottom(2,2,0) origin=(2.000000,2.000000,0.000000) target[edges(L=0,C=2,O=0) verts=4 scale=1.7321]
dir  vector                       edge_dist    vert_dist     min_dist   b_clear  b_leaves  b_behind
0    +z(0,0,1)                   0.000000e0    1.000000e0    0.000000e0  false      4        0
3    -z(0,0,-1)                 5.000000e-1    1.118034e0    5.000000e-1  false      4        4
6    diag(0.5774,0.5774,0.5774) 1.750886e-16  7.071068e-1   1.750886e-16  false      2        0
8    diag(0.5774,-0.5774,0.5774) 1.572090e-16 7.071068e-1   1.572090e-16  false      2        0
11   diag(-0.5774,0.5774,-0.5774) 5.000000e-1  1.118034e0    5.000000e-1  false      2        2
13   diag(-0.5774,-0.5774,-0.5774) 5.000000e-1 1.118034e0    5.000000e-1  false      2        2

trial=ambiguous seed=origin=b_bottom(2.5,2,0.5) origin=(2.500000,2.000000,0.500000) target[edges(L=12,C=0,O=0) verts=24 scale=6.0000]
dir  vector                       edge_dist    vert_dist     min_dist   b_clear  b_leaves  b_behind
6    diag(0.5774,0.5774,0.5774) 5.438960e-16  4.082483e-1   5.438960e-16  false      2        0
8    diag(0.5774,-0.5774,0.5774) 6.304855e-16  4.082483e-1   6.304855e-16  false      2        0
11   diag(-0.5774,0.5774,-0.5774) 1.060660e0   1.471960e0    1.060660e0   false      2        2
13   diag(-0.5774,-0.5774,-0.5774) 1.060660e0  1.471960e0    1.060660e0   false      2        2

trial=vertex-graze seed=origin=a(3,2,0) origin=(3.000000,2.000000,0.000000) target[edges(L=0,C=2,O=0) verts=4 scale=1.7321]
dir  vector                       edge_dist    vert_dist     min_dist   b_clear  b_leaves  b_behind
0    +z(0,0,1)                   0.000000e0    0.000000e0    0.000000e0  false      4        0
3    -z(0,0,-1)                 5.000000e-1    5.000000e-1    5.000000e-1  false      4        4
7    diag(0.5774,0.5774,-0.5774) 5.000000e-1    5.000000e-1    5.000000e-1  false      2        2
9    diag(0.5774,-0.5774,-0.5774) 5.000000e-1   5.000000e-1    5.000000e-1  false      2        2
10   diag(-0.5774,0.5774,0.5774) 1.572090e-16  4.082483e-1   1.572090e-16  false      2        0
12   diag(-0.5774,-0.5774,0.5774) 1.750886e-16 4.082483e-1   1.750886e-16  false      2        0
```

The rows are the four analytic signatures the audit cares about:

- **through-edge**: `ambiguous.a` `+z` passes exactly through the bottom rim at
  `(2,2,0.5)` — edge distance `0` (and `ambiguous.a` diagonals 6/8 pass through
  other rim points, `edge_dist ~1e-16`); criterion (b) refuses, 2 leaves per
  intersected rim circle (the root appears at both the `0` and the `TAU` end of
  the closed domain).
- **through-vertex**: `vertex-graze` `+z` passes through the rim self-loop
  vertex `(3,2,0.5)` — vertex distance `0` **and** edge distance `0`; this is
  why vertex avoidance (§3) cannot be elided.
- **through-corner**: `disjoint.b` diagonal 12 passes through the block top
  corner `(4,4,2)` (`7.7e-16` to the vertex AND to the three edges meeting
  there) — a formerly green seed whose unused directions include a real
  vertex/edge graze.
- **half-line false positives**: every `b_behind = leaves` row (e.g.
  `ambiguous.a` `−z`) is a direction whose ray is clean but whose *line* meets
  an edge behind the origin. Criterion (b) refuses them; the half-line census
  shows their actual ray distance is `≥ 5e-1`.

### T4 — measured runtime overhead per seed decision (sampled)

Timings are debug-build (the Done command runs the dev profile) and vary run to
run; measured across runs on this machine:

```
distance census (disk target, 4096 circle segments):   ~0.22-0.24 us per ray-segment QP;
                                                        ~0.91-1.00 ms per seed-direction decision  [debug]
criterion (b):  Line-edge verdict   ~0.24-0.42 us  (12-edge block target: ~2.9-5.0 us per decision)
criterion (b):  Circle-edge verdict ~1.8-2.9 us    (2-circle disk target: ~3.7-5.8 us per decision)
```

Notes for the reader: the `~1 ms` distance-census row is the *naive
tessellated* distance measure (2048 segments/circle × 2 circles), NOT the cost
of the intended criterion — the per-box interval/exact recursion that (b)
implements is the honest cost model, and it is **microseconds per edge, i.e.
tens of microseconds per decision even on a 12-edge block**, orders of
magnitude below the tessellated distance census. The interval-hull (a)
production path is not implemented here; its cost is bracketed between (b)
(per-box recursion, measured) and the tessellated census (upper bound).

### T5 — the ε-sweep (near-rim offset vs certification)

Target `raised_disk((2.5,2), r=0.5, z ∈ [0.5,1.5])`; seed `(2+ε, 2, 0)`. `ε` is
the radial gap to the rim: the `+z` ray passes the bottom rim at distance ~`ε`.
`δ_abs = 1e-2`, `δ_rel = 1e-3 · scale = 1.732e-3`.

```
      eps         +z edge_d   b_clear  near_abs near_rel  +x edge_d b_clear_x  clear  refuse
5.000000e-2    4.999994e-2      true     false    false  5.000000e-1    true     12     2
2.000000e-2    1.999998e-2      true     false    false  5.000000e-1    true     12     2
1.000000e-2    9.999988e-3      true      true    false  5.000000e-1    true     11     3
5.000000e-3    4.999994e-3      true      true    false  5.000000e-1    true     11     3
2.000000e-3    1.999998e-3      true      true    false  5.000000e-1    true     11     3
1.000000e-3    9.999988e-4      true      true     true  5.000000e-1    true     11     3
5.000000e-4    4.999994e-4      true      true     true  5.000000e-1    true     11     3
1.000000e-4    9.999988e-5      true      true     true  5.000000e-1    true     11     3
```

This is the sharpest single measurement in the survey: **criterion (b)
certifies the `+z` ray as CLEAR at every `ε > 0`, including `ε = 1e-4`, where
the ray passes the rim at 1e-4 — 17x under `δ_rel` and 100x under `δ_abs`.** A
sign-stability criterion alone cannot deliver the `δ > 0` requirement; it
certifies non-intersection, not clearance. The same table shows the 
certified-direction budget is stable (`clear` drops only from 12 to 11 exactly
when `ε` crosses `δ_abs` and the `+z` direction leaves the *census*-clear set —
criterion (b) itself keeps certifying it).

---

## 5. Calibration question and recommendation

**The doctrine floor, stated honestly.** "Fail-closed is not passable by
refusing everything": a certified path whose refusal rate is too high degrades
every formerly-green model to `NumericallyUnresolved`, which is a *worse* state
than the float path it replaces. The measured numbers bound the question:
refusal is direction-local (18/98 decisions, and **half of those are the
behind-origin half-line artefact**), and seed-level availability never drops
below 8/14 certified-clear directions on the corpus. So the fail-closed cost on
the *existing* fixtures is **zero refused seeds** under direction-refusal
semantics, and the price is paid only in genuinely ambiguous directions that
today's classifier already retries.

**Recommendation.**

1. **Criterion.** Adopt **(a-LINE) per-edge subdivision with a certified
   `‖g‖`-magnitude lower bound** (equivalently b-δ with the mig record) as the
   δ certificate on the analytic Line/Circle subset, extended to general
   carriers by the interval/`Hull` discipline of the tree (`truck-evidence`
   `enclosure`). Use criterion (c) slabs as the cheap whole-ray pre-filter and
   criterion (b) sign-stability as the per-box exclusion inside (a) — (b) is
   *not* a standalone δ certificate (T5). The half-line issue argues for the
   a-HALF clamp, but see O2 for whether the conservative a-LINE price (9/18 of
   refusals measured behind-only) is acceptable in DEF-SEEDRAY-B's budget.
2. **δ formulation.** Use **scale-relative** δ (`δ_rel = η · scale`, scale =
   target bounding diagonal; probe used `η = 1e-3`), per H-3: absolute
   constants in predicates are a defect (FORMAL_SYSTEM §2). `δ_abs = 1e-2`
   exists only because the corpus is unit-scale; the production constant must
   be relative to a declared model scale. The ε-sweep (T5) is the template for
   measuring the refusal-rate curve as a function of `η` — DEF-SEEDRAY-B should
   sweep `η` and report the curve, not a point.
3. **Refusal semantics.** Refuse the **DIRECTION** (retry the next of the 14)
   when avoidance fails for that direction; refuse the **SEED**
   (`NumericallyUnresolved`) only when all 14 are refused. This matches the
   current `ray_seed` exhaustion control flow, keeps the corpus's formerly
   green seeds green, and keeps the fail-closed doctrine meaningful (a seed is
   refused only when the certified path can produce no admissible ray at all).
   Refusing the seed on a single direction's avoidance failure would violate
   the doctrine floor for no measured benefit.

---

## 6. Open questions for the external auditor

- **O1 (metric link).** The theorem needs `dist(ray, ∂B-strata) ≥ δ` in 3-D,
  but `classify_region`'s `Boundary` band is parameter-domain distance `<= tol`
  with `tol = TOL` (H-3 tolerance class). Under what carrier-metric hypothesis
  does 3-D clearance `δ` imply the crossing is not `Boundary`-classified? Is the
  correct certificate really a 3-D clearance, or a *parameter-band* clearance
  (crossing's uv-distance to the trimmed-region boundary polygon)?
- **O2 (half-line policy).** 9/18 of measured criterion-(b) refusals are
  behind-the-origin line crossings (the ray itself is clean). Accept the
  conservative a-LINE over-refusal, or implement the a-HALF `t ≥ 0` clamp
  (extra interval projection per box)? The decision changes the refusal-rate
  by ~2x on the corpus.
- **O3 (what invariant is actually needed).** Does the seed-parity theorem need
  `dist ≥ δ` at all, or only (i) no crossing on an edge/vertex and (ii) no
  `Boundary`-classified crossing (i.e. no *tolerance-near* crossing)? If (ii),
  the certification target is the classifier's band, not a bare 3-D clearance —
  see O1.
- **O4 (subdivision depth on general carriers).** For analytic Line/Circle
  edges the depth budget is trivial, but for general `𝒢` carriers the exclusion
  depth is bounded only under a quantitative non-degeneracy hypothesis (OB-3
  `sin θ ≥ δ`, separation `≥ σ` style). What is the depth/refusal behaviour on
  non-analytic edges (BSpline/NURBS), and does the 14-direction budget need a
  per-edge cap with typed refusal rather than silent widening?
- **O5 (per-direction shared state).** The on-boundary pre-screen in
  `ray_seed` depends only on `p`, not on `d`; can the avoidance certificate
  also be split into a direction-independent vertex part (once per seed) and a
  direction-dependent edge part? That would change the budget from
  `O(dir × edges)` to `O(edges + dir × surviving-edges)`.
- **O6 (scope: whose edges).** Rule-(b) seeds cast rays against `∂B` only, but
  the seed origin `p` lies on `∂A`. Must the certified path also certify that
  the *initial* ray segment near `p` avoids `∂A`'s own edges (a rep point is
  interior to its region, but the fragment-region representative and `p` may be
  extremely close to `A`'s trimmed boundary)? Not covered by the corpus.
- **O7 (tight-tangency resolution).** With `B_EPS = 1e-12` the recursion
  refuses near-tangent misses closer than ~1e-12 as well as exact tangencies;
  is refusing exact tangencies correct (they are measure-zero but manufacture
  the through-edge double count)? And is `B_EPS` the right floor, or should it
  be derived from `δ_rel` (e.g. `δ_rel/10⁴`)?
- **O8 (interval trig width).** Production (a)/(b) over arcs will use
  `truck-evidence`'s certified interval `sin`/`cos` (BG-ENC-005); near
  reduction boundaries those widen to `[-1,1]` and would refuse boxes the exact
  probe certifies. Does the widening interact with the depth cap on realistic
  `𝒢` arcs? (The exact Line/Circle probe sidesteps this; production cannot.)
- **O9 (uniformly-wrong components).** The audit adjudication (M9) requires
  three certificates (avoidance; transversality enclosure; certified
  disjoint crossing intervals). This survey covers the first. Confirm the
  avoidance certificate record (§3) is designed so DEF-SEEDRAY-B can compose
  all three without re-opening the verdict types, and that the
  "unconstructible without the record" doctrine is kept (bare per-direction
  booleans are not certificates).

---

## 7. References and provenance

- `vendor/truck/truck-shapeops/src/boolean/classify.rs` — the current seed
  machinery (rule-b `find_seed`, `ray_seed` + 14-direction table,
  `classify_region`, `surface_ray_crossings`, the contradictory-component
  refusal). NOT changed by this packet.
- `docs/FORMAL_SYSTEM_BREP_GENERATION.md` §12 (membership by propagation;
  `CLS-SEED-001`: "Ray direction admissible iff the ray meets no vertex, edge,
  or tangential locus — certified by interval separation, not
  symbolic-perturbation folklore") and §2 (scale-invariance, H-3).
- `docs/AUDIT_MATH_CODE_CORRESPONDENCE.md` M9 adjudication (confirmed high;
  three certificate conditions booked as DEF-SEEDRAY-CERT; this survey is the
  audit-first pass over condition 1) and the cross-cutting "write the proof
  before booking the packet" / evidence-carrying-verdict doctrine.
- `vendor/truck/truck-shapeops/tests/seedray_avoidance_probe.rs` — the probe
  whose stdout this document cites. Reproduce with:
  `cargo test -p truck-shapeops --test seedray_avoidance_probe -- --ignored --nocapture`.

Probe evidence recorded on 2026-09-07 (this worktree); census tables (T1–T3,
T5, green register) are deterministic on a fixed build; T4 timings are sampled
debug-build figures and are quoted with their range.
