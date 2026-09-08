# TORUS-CONTACT THEORY — formal architecture (companion to TORUS_CONTACT_PROGRAM.md)

**Status:** authored 2026-09-08 (orchestrator) against the landed substrate
(`formal/torus.rs` regular-ring certification, `torus_pairs.rs` green
contact rows) and the owner theorems A–D. This doc states, per stage, what
is SOLVED, what must be PROVED, and what is genuinely new.

## 0. The mathematical object

The regular ring torus (R > r) with exact rational placement:

- **Implicit:** `T(x) = (‖x‖² + R² − r²)² − 4R²(x² + y²) = 0` — degree 4,
  exact rational coefficients. Certified evaluation of `T` and `∇T` over
  boxes is polynomial Bernstein enclosure (landed machinery, directly).
- **Rational atlas:** the angle parametrization is not rational, but the
  tan-half-angle substitution `p = tan(θ/2), q = tan(φ/2)` gives, per
  chart, an EXACT rational tensor form `X = A/W` with
  `W = (1+p²)(1+q²) > 0` everywhere. **This is Theorem A's exact input
  form.** The atlas is 2 charts per angle; completeness (every surface
  point in ≥ 1 chart) and the chart-boundary strata (φ = ±π etc.) are the
  atlas proof obligations (N2 below).
- **Normal field (closed form):** `n(θ,φ)` is the unit radial of the tube
  circle — normal cones per box are exact by construction, not estimated.
- **Gauss map (closed form):** `K = cos θ / (r(R + r cos θ))` — the
  parabolic strata are exactly the tube's top/bottom circles (`cos θ = 0`).
  Fold certification gets these as pre-classified strata.
- **Doctrine:** only the regular ring torus certifies; horn (R = r) and
  spindle (R < r, self-intersecting on the circle `r = R`) refuse typed at
  every stage (the landed `formal/torus.rs` rule carried forward).

## 1. TOR-A — torus×canonical into the funnel

**Solved/landed:** contact DECISIONS for torus×plane (axial, oblique,
offset rows green in `torus_pairs.rs`).

**To prove and build:**

- **N1 — the section-classification theorem (the closed-form prize).**
  Every torus×plane instance is exactly one of: (a) axial-plane cuts →
  profile circles; (b) axis-perpendicular planes → coaxial circles;
  (c) bitangent oblique planes → **Villarceau circle pairs** (the bitangent
  condition is an exact predicate: the plane's offset `d` from the axis
  satisfies `d² = R² − r²` — decide by exact arithmetic on the plane's
  rational data); (d) general planes → quartic sections (traced, not
  closed-form). Proof obligation: the case split is decided by EXACT
  predicates, and cases (a)–(c) emit `AnalyticIntersection` circle loci —
  converting certified contact-decisions into certified LOCI the splitter
  consumes without tracing. This is the single highest-leverage new
  theorem: the corner rows' cuts are plane cuts.
- **The implicit-reduction cost question (measure, then decide).**
  Extending Theorem-4 reduction to tori composes the quartic with the
  canonical chart: `h = T∘S` at bidegree (4p, 4q) — exact (polynomial
  composition), but the Bernstein degree quadruples and exclusion cost
  follows. Formal work: none (composition is trivially exact); empirical
  work: measure exclusion cost at (4p,4q) before committing the reduction.
  The cheaper alternative — routing torus×plane/quadric pairs through the
  landed pair machinery as analytic loci (N1) — is tried FIRST.
- **Coaxial/aligned degeneracies** (torus×cylinder coaxial → circles;
  torus×torus coaxial → circles/complex curves): exact predicates in the
  BG-ANA style — never float.

## 2. TOR-B — seed-ray torus crossings (unlocks torus-bearing solids in classify)

**To prove and build:**

- **N4 — the ray-quartic reduction.** Ray `x(t) = o + t·d` into `T`:
  a quartic in `t` with exact rational coefficients (from the exactified
  ray — SFC: float search, exact certification). The landed three-state
  root isolation (Bernstein/Descartes; every returned interval contains
  exactly one root; multiple roots refuse `NumericallyUnresolved`) applies
  unchanged at degree 4. Proof obligation: state and test the PARITY
  argument — the crossing count of the exact ray equals the winding
  contribution for the classify stage (standard, but instantiated and
  tested for the quartic).
- **Tangency strata:** double roots = ray tangent to the torus. The
  tangency cascade (landed) classifies; the torus's closed-form normal
  field makes the tangency predicate algebraic and decidable.
- **The `classify.rs` gate extension:** `surface_ray_crossings` gains the
  torus arm — dispatch-table extension only (FSSI-EXT doctrine; the
  `require_canonical_carriers` gate widens to torus-bearing solids once
  TOR-B lands).

## 3. TOR-C — torus×spline (the 4-D deep end; gated on the corpus exposure PB-011C now provides)

**To prove and build:**

- **N2 — the atlas argument.** Per §0: each torus chart is an exact
  rational tensor patch with strictly positive weights — Theorem A's exact
  input. Proof obligations: (a) atlas completeness (every torus point in
  ≥ 1 chart); (b) per-chart degree bounds for the polynomialized
  interaction `F = Ŵ_Y·Â − Ŵ_X·B̂` (the torus contributes bi-(2,2)
  numerator/denominator — bounded, so `bernstein_box4` exclusion
  terminates); (c) periodicity strata — chart-boundary events are
  FSSI-003's region-transition strata, enumerated exactly.
- **Theorem C instantiation.** Transversality via the closed-form normal
  cones: torus normal cone vs the spline face's cone, `δ > 0 ⇒ rank DF = 3`
  at every intersection. The torus side is exact by construction (the
  normal is the tube radial); only the spline side needs the cone
  machinery — ADM-002's, shared.
- **Folds on the torus.** Interaction-curve folds pass through the torus's
  parabolic circles (`cos θ = 0`, closed form) — FSSI-002's fold
  certification receives these as PRE-CLASSIFIED strata (the Hessian is
  known in closed form), which is cheaper than generic fold search.
- **Facts.** Trimmed torus-patch volume via T2′ (Theorem D handles the
  rational weights; `W = (1+p²)(1+q²)` has certified positive bounds per
  chart). The untrimmed torus volume `2π²Rr²` is the closed-form sanity
  anchor.

## 4. The genuinely-new list (everything else is instantiation)

| # | Item | Size |
|---|---|---|
| N1 | Villarceau/section classification → exact analytic circle loci for torus×plane | the prize; moderate |
| N2 | torus rational atlas: completeness, degree bounds, periodicity strata | small (classical construction; bookkeeping proof) |
| N3 | closed-form parabolic strata feeding FSSI-002's fold tier | small |
| N4 | ray-quartic parity/winding instantiation | trivial but must be stated and tested |

## 5. Sequencing (unchanged from the booking doc, now theory-grounded)

```
TOR-A (N1 + pair routing into the funnel; measure the (4p,4q) reduction
       before committing it) → TOR-B (N4 + classify gate extension) →
TOR-C (N2/N3 + Theorem A/C/D instantiation; entry condition: the F1
       re-census and PB-011C's exposure size the spline-torus demand)
```

Entry condition status: PB-011B recorded the corner-row torus refusals
(the booking evidence); the corpus exposure exists. The corner rows
(`cut(revolved-circle, canonical)`) need exactly TOR-A's N1 loci + TOR-B's
classify extension — they do NOT wait for TOR-C.
