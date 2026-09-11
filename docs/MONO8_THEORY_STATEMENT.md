# The theory the monocoque needs — formal statement (MONO-8/9/10 boundary)

Authored 2026-09-10 (orchestrator), owner-requested. This file states
precisely what is proved, what is assumed, and what is measured. It is
the citation surface for MONO-8-SWEPT-ADMISSION-WIRING and
MONO-9-FUSE-FOLD. Normative antecedents: MONO_WAVE3_THEORY_BRIEF.md
(Theorems 1-2, review verdict 9be72b7), FSSI-001 (transversality gate),
MONO-5 (certified membership), MONO-2 (the N-station loft convention),
SWEPT_PAIR_ADMISSION_SPEC.md (Theorems A-D).

## 1. Definitions

**Patch 2-cycle.** A solid boundary representation
`A = (P_1..P_m; sigma_1..sigma_m)` where each `P_i : [0,1]^2 -> R^3` is
a bidegree-(3,3) tensor-Bernstein patch with strictly positive weights
(the landed canonical carrier), and `sigma_i in {+1,-1}` is the
outward-orientation sign of `P_i`. Consistency (closed, oriented):
the signed boundary of the cycle vanishes; certification of individual
patches is per-patch through the landed `volume_facts` binding.

**Extraction map.** `E(R) = {(P_1,sigma_1)..(P_m,sigma_m)}` for a
recorded construction row `R` with placement `(t, R_z, rot, mirror)`:

- `P_i` are the patches of the landed kernel construction of `R`'s
  solid carrier (loft: the MONO-2 convention — chord-length station
  law, v-degree per annex A4, u-degree 3; member: the landed swept
  carrier);
- placement acts on control points: `P_i' = tau ∘ P_i` with `tau` the
  placement's affine map — exact, because Bernstein control points
  transform covariantly under affine maps with unchanged weights;
- `sigma_i = sign det(M_tau) * sigma_i^0`, where `M_tau` is `tau`'s
  linear part and `sigma_i^0` the construction's intrinsic orientation.
  **A reflective placement (`mirror`) has det < 0 and flips every
  sign.** Getting this wrong corrupts the flux sign; it is stated here
  because it is the extraction's one correctness-critical detail.

**Well-formed extraction.** `E` is total on the landed carrier
vocabulary (loft sections in the recorded Face/line/spline discipline,
members, primitives) and refuses typed naming the open carrier where
the vocabulary is not yet extracted. Extraction is exact: no
approximation, no resampling, no tolerance.

## 2. The certification contract (landed)

For patch 2-cycles A (extracted from the base operand) and B (from the
tool operand) with no coincident patches, the landed MONO-6 solver
returns:

- a certified bracket `[V_lo, V_hi] contains V(A\B)` with
  `width <= eps` (the budget), or
- a typed refusal.

Mechanism (Theorems 1-2, brief sections 1-4): the volume is the flux
`(1/3) ∮ x·n dS` over the operands' boundaries with the membership
indicator `1_B - 1_{A∩B}` evaluated by the MONO-5 certified membership
primitive (outward-rounded interval discipline, retry contract);
contact cells are covered by an error-directed shrinking cover
(terminates by the Lemma-3 separable closest-pair exclusion); each
unresolved cell contributes its interval-bracketed flux (eq. 5);
termination when `Σ E(R) <= eps` (eq. 8). No intersection-curve
reconstruction anywhere — this is the theory's central constraint and
it is what makes the certificate exact-at-the-leaves rather than
toleranced.

## 3. The admission predicate (the gate MONO-8 implements)

The pair `(A, B)` is CERTIFIABLE iff all of:

1. **Extraction.** `E(A)` and `E(B)` are well-formed (both operands'
   carriers are in the extracted vocabulary);
2. **Transversality.** the operands' surfaces meet transversally on the
   work box (FSSI-001 / Theorem C gate, landed) — no tangential
   contact locus;
3. **Budget.** the error schedule terminates: `Σ E(R) <= eps` is
   reached within the subdivision budget.

Each failure mode refuses TYPED, naming which of 1-3 failed. Typed
refusal is a first-class verdict: it is the measured boundary, booking
evidence for the next packet — never an error, never a guess.

**Honest boundary:** clause 2 on the REAL tub/cavity pair is a
measurement, not a theorem. The cavity sits inside the skin and meets
it near the rim opening; if the actual contact locus is degenerate
there, the gate refuses and the row records typed. No landed theorem
discharges this in advance; only the census answers it.

## 4. The multi-operand fold (MONO-9)

`V(A ∪ B_1 ∪ ... ∪ B_n)` — the corpus's `fuse_proud` — does NOT fold
pairwise through the solver, because a union of patch 2-cycles is not a
patch 2-cycle and the theory forbids reconstructing the union's
boundary. The certified formulation is per-tool flux against a compound
indicator:

    V(A ∪ B_1 ∪ ... ∪ B_n)
      = V(A) + Σ_i ∫_{∂B_i} (x·n_i / 3) · 1_{outside A ∪ B_1 ∪ ... ∪ B_{i-1}} dS

Each integrand is the MONO-6 flux machinery over `B_i`'s patches with
the membership indicator evaluated by MONO-5 against MULTIPLE solids
(the primitive already classifies a point against a set of certified
solids — that is why the fold is an extension, not new theory). No
intermediate union is ever constructed.

**Width budget.** Contact cells are per-pair; the fold's bracket width
is at most `Σ_i eps_i`. The corpus band is 1e-4 relative; the packet
sets `eps_i = eps_total / n` from the fold count. Fold order is
recorded (deterministic) and does not affect the bound.

## 5. Group aggregation (bracket-valued facts)

A group whose children include boolean results aggregates INTERVALS:
`[Σ V_lo, Σ V_hi]` over children (scalar children are degenerate
intervals). The green predicate "volume carries its own certificate
bracket" is satisfied by the group bracket. This is a facts-schema
extension (scalar -> interval), booked into MONO-8's scope; the
arithmetic is interval addition — no new theory.

## 6. What this statement deliberately does NOT give

1. **Transversality of the real carriers** (section 3, clause 2) —
   measured by the census, not provable in advance.
2. **A faithful boundary mesh of the boolean result.** The no-
   reconstruction constraint means the certification path produces
   numbers, not trimmed surfaces. A renderable cut solid requires
   extracting the boundary of the classified cover into a mesh
   (MONO-10, booked, unscoped, awaiting the owner's ruling on whether
   R3's "mesh emits" means emits-without-crashing or faithful-render).
   Until that ruling, the certification program is complete WITHOUT
   item 6.2, and renders of cut results remain the old carriers' soup.
3. **Chamfer, non-coordinate mirrors, double mirrors** — typed
   refusals by design; the recorded vocabulary is the program.
