# Swept-pair admission — theory spec for the census-measured gap

**Status:** authored 2026-09-08 by the orchestrator, directly against the
PB-011C census RESULT (`loop/results/PB-011C-TTC-PARITY-CHURN.json`) and the
F1 machinery survey (`loop/results/F1-MACHINERY-AUDIT.SURVEY.json`, 86 rows,
validated). This doc is a booking surface: every LANDED claim below was
re-verified against the tree today; every NEW claim is a named theory
deliverable with its proof obligation. No packet may cite this doc without
citing the row that grounds it.

## 1. The measured gap (what the census established)

PB-011C ran all eight swept×swept F1 rows census-first. Result: **none
certifies, and none stagnates** — every row refuses typed
(`NonCanonicalCarrier`) at the boolean boundary, before any 4-D continuation
budget is spent. The landed restricted 4-D arm (ssi4 / CL-004 / CL-006)
admits only **circular-section restricted sweeps against canonical/analytic
sides**. A general spline-loft solid pair is never converted into an
`Ssi4System` at all.

So the gap is **admission**, not solver capability, and not fold/tangency
robustness (that is FSSI's program, downstream of admission). The theory
question this spec answers: *what exact machinery converts a pair of
spline-loft solids into certified boolean output?*

## 2. The mathematical object

A loft solid's boundary is a closed set of faces; each face is a
tensor-product spline patch `X(u,v)` (sections along one parameter,
cross-section interpolation along the other — the landed `construct` loft
family already produces these as certified objects; survey rows R0xx, a-bucket
"loft theory (construct loft family + banded substrate)"). A boolean
`cut(A, B)` between two such solids requires, per face pair
`(X(u,v), Y(s,t))`:

1. **Certified evaluation.** Range enclosures of `X`, `Y`, and their
   partials over rectangular `(u,v)` / `(s,t)` subboxes, with subdivision.
   **Landed:** `hull_bernstein_2d` (`src/hull.rs:95-117`), `TensorGrid4`,
   `bernstein_box4` (`src/interval/bounds.rs:111-208`) — Bernstein range
   enclosure over 4-D subboxes is exactly this primitive (FSSI spec §0,
   row 6).
2. **The interaction system.** `F(u,v,s,t) = X(u,v) − Y(s,t) ∈ R³`, three
   equations in four unknowns over the compact product domain: generically a
   1-dimensional solution curve. **Landed:** `Ssi4System` over `(u,v,s,t)`,
   `IntervalBox4` (`construct/bie/ssi4.rs:26-70`).
3. **Chart selection.** Where the 3×3 cofactor minor of the Jacobian is
   invertible, solve three coordinates against the fourth (the fiber). The
   F3 rule (largest relative margin, lowest index, refuse below threshold)
   is **landed** (`src/ssi.rs:45-56`, `src/contract.rs:86-88`) and frozen.
4. **Curve continuation.** The dimension-generic Krawczyk operator +
   parallelotope tube continues the curve fiber-by-fiber with certified
   uniqueness per step. **Landed:** `KrawczykSystem<N>` (`num/krawczyk.rs`),
   parallelotope continuation (`num/parallelotope.rs:12-52`).
5. **Exclusion and transversality.** Boxes with no solution must be
   cheaply and provably excluded (Bernstein exclusion of `F` itself —
   landed); separately, the interaction must be certified TRANSVERSE
   (rank 3) wherever it exists. **Landed substrate:** `bernstein_box4`
   exclusion; **FSSI-001 (spec'd, corrected 2026-09-08 per owner Theorem
   C)** is a transversality gate, NOT an exclusion test: positive normal
   separation `‖n_X × n_Y‖ ≥ sin δ > 0` via two per-surface 2-D normal
   cones ⇒ `rank DF = 3` at every point of `Σ ∩ B` — it does NOT imply
   `Σ ∩ B = ∅`. (The original draft's emptiness claim was mathematically
   false and is corrected in FSSI_BUILD_SPEC.md.)
6. **The hard strata.** Ordinary folds (where the curve is tangent to a
   chart boundary), tangency curves, coincident patches, and degenerate
   sections (loft apexes) are where interval methods stall. **This is
   FSSI's program exactly** (`docs/FSSI_BUILD_SPEC.md`): FSSI-002 certifies
   ordinary folds as a new escalation-lattice tier (Krawczyk on the
   4-equation system `(F, q_j)`, `det D(F,q_j)(p) ≻ 0` ⇒ unique event, local
   incidence count 0 ↔ 2); FSSI-003 delivers event completeness (skeleton:
   interior events + chart-transition points + boundary events) and boundary
   strata. Both are **gated on this program's admission existing** — the
   census settled that entry condition.
7. **Ruled fast path.** Ruled×ruled pairs (heavily represented in
   turbomachinery and the F1 bodies) have closed-form loci:
   `λ(t,s) = (B−A)·(d×e)` with rational `u,v` recovery, trimmed event
   system, parallel-generator excision. **Spec'd as FSSI-004** (not landed);
   consumes the landed `ContactLocus::Analytic` transverse path in
   `split.rs` — no splitter changes.
8. **Trimming.** Each face is cut by the other solid's boundary: the traced
   curve + boundary events partition the face's parametric domain, and
   region membership is decided by exact integer winding on trim loops.
   **Landed:** `kernel/trimclip.rs` (exact ray crossing). Boundary strata
   enumeration is FSSI-003's §10 (not landed); corner strata (`∂D_X × ∂D_Y`)
   discharge by exclusion, never sampling.
9. **Reconstruction.** Splitter consumes the locus (analytic or traced arcs
   carrying per-segment chart indices — FSSI-003's representation
   requirement) → trim → classify → sew. **Landed** for admitted pairs; the
   admission work must produce the locus in exactly the landed
   `ContactLocus` vocabulary — any splitter change is a stop-and-file.
10. **Facts certificates.** Solid count via shell reconstruction (landed);
    **volume of a spline-faced solid is the genuinely new theory bit** —
    see §3.

## 3. The new theory deliverables (revised 2026-09-08 per owner's theorems — replaces the original T1/T2/T3)

The owner's revision replaces the loft-specific admission with ONE generic
theorem and collapses the pathology taxonomy to two algebraic questions.
Every claim below was re-derived and checked before acceptance (the
cancellations and the transversality correction are verified in the
commit trail). The delta is smaller than the original draft:

- **T3′ — Exact tensor-spline admission (Theorem A; "essentially no" new
  theory).** Any positive-weight tensor-spline face admits. On each knot
  rectangle, Bézier extraction is an exact local change of basis
  (geometry-preserving), so with `X = Â/Ŵ_X`, `Y = B̂/Ŵ_Y` (tensor-Bernstein,
  affine spans re-parameterized to `[0,1]²`, weights strictly positive):

  `X(u,v) = Y(s,t)  ⇔  F = Ŵ_Y·Â − Ŵ_X·B̂ = 0`

  — a POLYNOMIAL map, which is exactly what the landed `bernstein_box4`
  exclusion and `Ssi4System` want. **One representation adapter, not a
  family of admission cases.** Covers F1 lofts AND Falcon-Heavy
  spline-profile revolves (a revolved spline profile is the profile spline
  tensored with rational circle arcs — itself a rational tensor-product
  surface). Runtime discipline: lazy extraction — control-hull/AABB-cull
  face pairs, then knot spans; extract only survivors; cache extraction
  operators per knot configuration. V5 identity is structural: canonical
  dispatch stays AHEAD of the adapter; the adapter fires only where the old
  path would return `NonCanonicalCarrier`.
- **T1′ — Regularity by two algebraic questions (Theorems B1/B2; "tiny").**
  No pathology taxonomy. The only questions: is the parameterization
  regular, and is a boundary stratum intentionally collapsed/identified?
  For rational faces use the polynomial normal numerator
  `M = W(A_u×A_v) − W_v(A_u×A) − W_u(A×A_v)` (verified: `X_u×X_v = M/W³`,
  so regularity is a POLYNOMIAL question). (a) **Hemisphere certificate:**
  pick `c` from the floating midpoint normal, round to dyadic, certify
  `min(bernstein coefficients of c·M) > 0` ⇒ regular on all of `B`
  (convex-hull property) — no singularity search unless this cheap test
  fails. (b) **Collapsed-edge deflation:** a boundary collapsed to `p`
  gives a known factor `(1−v)` in `M`; divide it out and certify the
  quotient (`c·M* > 0` ⇒ regular for all `v < 1`, the only rank defect is
  the intentional collapse); repeat for multiplicity `k` — higher-order
  collapses handled without a new solver. (c) **Seam identification:**
  `X(u,0) = X(u,1)` certified exactly by `A_0·W_1 − A_1·W_0 ≡ 0` (aligned
  degrees/knots); seams are paired BRep edges, not singularities. Outcome
  space: regular | regular+seam | regular-interior+collapsed-boundary |
  genuine parameter singularity → typed refusal. No "tangent sections"
  category — tangent construction sections matter only if they actually
  force `M = 0`.
- **T1′/FSSI-001 — Transversality from the same normal cones (Theorem C;
  "small").** `rank DF < 3` at a solution iff the tangent planes coincide;
  `inf ‖n_X×n_Y‖ > 0` over normal cones with angular separation
  `δ > 0 ⇒ ‖n_X×n_Y‖ ≥ sin δ` ⇒ rank 3 at every intersection. One
  per-surface normal-cone subsystem closes BOTH T1′ regularity and
  FSSI-001 transversality. **The FSSI spec's original emptiness claim was
  false and is corrected at the source** (transversality ≠ exclusion;
  emptiness is Bernstein exclusion of `F`, a separate test).
- **T2′ — Certified volume (published machinery + one small new
  primitive).** Polynomial case: Antolin–Hirschler-style boundary
  reduction — `V = (1/3) Σ ∬ g`, `g = X·(X_u×X_v)` polynomial;
  `H(u,v) = ∫ g du`, Green ⇒ `∮ H dv`; trim segments Bézier ⇒ the final
  integrand is a UNIVARIATE Bernstein polynomial, integrated as
  `Σ pᵢ/(n+1)` — **quadrature-free, published construction**, not new
  research. Rational case (verified cancellation): `g = P/W³` with
  `P = A·(A_u×A_v)` — all `W_u, W_v` terms vanish. **Theorem D —
  certified reciprocal-power polynomialization:** Bernstein weight bounds
  `0 < w₋ ≤ W ≤ w₊`, `δ = (W−w₀)/w₀`, `|δ| ≤ ρ < 1`; truncate
  `(1+δ)⁻³` after `m` terms; certified uniform tail
  `ε_m ≤ w₀⁻³ Σ_{k>m} C(k+2,2)ρᵏ`; `|I − Ĩ| ≤ area(R)·‖P‖∞·ε_m`. One
  primitive, `certified_reciprocal_power(W, p, target_error) → polynomial
  + remainder_bound`, generalizes (rational trim pcurves included).
  Geometric convergence in ρ; subdivide once (de Casteljau) if weights
  vary violently. Precedent: Krishnamurthy & McMains (trimmed-NURBS
  moments with rigorous error bounds). FH-SPLINE-LATHE's segment-moment
  derivation (in flight) is the revolve special case and lands
  independently; T2′ can absorb it later.

| Piece | Machinery | New theory? |
|---|---|---|
| T3′ | lazy exact Bézier extraction → existing `Ssi4System` | essentially no |
| T1′ | scalar normal hemisphere certificate | tiny |
| T1′ | exact collapsed-edge factor deflation | yes, but very small |
| T1′/FSSI-001 | certified normal cones → transversality | small |
| T2′ polynomial | Green/divergence → trim-line Bernstein integral | published |
| T2′ rational | reciprocal-power polynomial + rigorous tail | new small primitive |

## 4. What this program deliberately does NOT do

- No approximate answers, no sampled intersections, no tolerance-tuned
  classification (SFC: search in floats, certify exactly).
- No change to the constructive fast path (CG doctrine: SSI stays off it).
- No torus work (TORUS_CONTACT_PROGRAM owns that family; TOR-C consumes
  FSSI-001/002 as substrate).
- No re-measurement: FSSI-CENSUS discipline — sizing decisions cite
  PB-011C's census rows and FSSI-005's instrument split.

## 5. Cost envelope

New theory: T1 + T2 are the load-bearing proofs (T2 the larger: the
trimmed-domain integral machinery). Everything else is wiring landed pieces
per §2. The program is smaller than FSSI-006/007 (which remain gated on
FSSI-005's cost split) and is the *prerequisite* for FSSI-002/003 having
anything to act on. Sequencing:

```
T3 (dispatch contract, admits nothing yet) → T1 (loft strata) →
  pipeline wiring on restricted sub-classes first (ruled-section lofts,
  via FSSI-004's closed forms) → general spline sections →
  T2 (volume certificates) → corpus rows re-run (census row 2)
```

Each stage keeps the typed refusal as the default answer; admission widens
monotonically. The PB-011C census rows are the acceptance instrument: a row
flips from typed-refusal to certified-lift only with its facts matching the
recorded reference.

## 6. Owner decisions requested

1. Authorize this program as booked (packet family authored on request).
2. Confirm FH-SPLINE-LATHE stays queued as-is (it is the revolve-case
   special case of T2's facts machinery and lands independently).
3. Confirm the FSSI-002/003 entry condition is now considered MET by the
   census (their gating premise — "admission exists" — is only true after
   this program's T3/T1; until then FSSI lands names and the gate only).
