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
5. **Exclusion.** Boxes with no solution must be cheaply and provably
   excluded. **Landed substrate:** Bernstein exclusion (row 6 above);
   **FSSI-001 (spec'd, not landed)** adds the separable tangency-free gate —
   hull of `‖n_X × n_Y‖` as a 2D×2D composition, strictly positive lower
   bound ⇒ empty intersection and rank-3 throughout, at O(deg²) hull cost
   instead of O(deg⁴) product materialization.
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

## 3. The new theory deliverables (the honest delta)

Everything in §2 rows 1–5 and 8–9 is landed. Rows 5(partial)–7 are FSSI's
spec'd program. The admission program itself owns exactly three new pieces:

- **T1 — Loft-boundary strata.** The loft's own degeneracies (apex poles of
  a section collapsing to a point, tangent sections, seam edges where the
  cross-section interpolation closes) must be enumerated as typed strata
  with exclusion criteria, so the tracer never enters an undefined chart.
  Proof obligation: every degenerate locus is either excluded by a named
  enclosure test or refused typed (`DegenerateFold`-class vocabulary per
  FSSI-REFUSAL). No sampling anywhere.
- **T2 — Certified volume for spline-faced solids.** Generalization of the
  landed frustum telescoping (which is the revolved-polygon special case,
  and of FH-SPLINE-LATHE's segment-moment revolve integral): for a closed,
  oriented, trimmed spline boundary, `V = (1/3) Σ_patches ∫∫
  X·(X_u × X_v) du dv` — each patch contribution is a polynomial (or
  rational, if rational sections) integral over its trimmed domain, computed
  in the landed exact interval algebra (`formal::exact::CertifiedInterval`),
  with the trimming curves as integration boundaries handled by the same
  exact-winding discipline. Proof obligation: the certificate must detect a
  non-closed or mis-oriented trimmed boundary (the volume certificate is
  meaningless unless closure is proven first — this is the same discipline
  as the shell-closure checks the mesher already enforces, lifted to
  certified arithmetic).
- **T3 — Admission dispatch.** The boolean boundary routes
  `(spline-loft, spline-loft)` pairs into the pipeline above, behind typed
  refusals for every carrier form the pipeline does not yet certify (the
  NonCanonicalCarrier refusal stays the default; admission widens it case by
  case, each with its admitting test). Proof obligation: V5-pair identity —
  every currently-green pair answers bit-identically after admission lands.

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
