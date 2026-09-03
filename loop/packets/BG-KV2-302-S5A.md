# BG-KV2-302-S5A — isolated contact: the tolerance-tagged classifier and ContactCert

Wave-3 implementation packet (build spec §4; §19 row 12; spec §10.3 verbatim
— Theorem 10.1, Corollary 10.2's classification table, Proposition 10.3's
honesty contract). Produces the tolerance-tagged contact claim: the shim's
`ContactCert` becomes real. Consumes the frozen seams: `CertifiedPatchC2`
(second_derivs, shim), `CertifiedPatchC3` (third_jet, shim),
`krawczyk_c1` (S2A), the shim kit's rational-carrier fixtures, and
`config::TOL_INTERSECTION`.

```yaml
id:          BG-KV2-302-S5A
contract:    [BG-KV2-302-S5A]
class:       design
crates:      [truck-certified]
depends_on:  [BG-KV2-201-S2A, BG-KV2-104-RATCARRIER]
write_allow:
  - vendor/truck/truck-certified/src/kernel/contact.rs
  - vendor/truck/truck-certified/src/kernel/mod.rs
  - vendor/truck/truck-certified/tests/kernel_contact.rs
read_allow:
  - docs/CONSTRUCTIVE_GEOMETRY_KERNEL_SPEC_V2.md
  - vendor/truck/truck-certified/src/kernel
  - vendor/truck/truck-evidence/src/contact/singular.rs
budget:      {turns: 30, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub struct ContactCert' vendor/truck/truck-certified/src/kernel/certs.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'fn second_derivs' vendor/truck/truck-certified/src/kernel/patch.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'fn third_jet' vendor/truck/truck-certified/src/kernel/patch.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub const TOL_INTERSECTION' vendor/truck/truck-certified/src/kernel/config.rs"}
tests_required:
  - critical_point_of_nabla_g_certifies_square_c1
  - hessian_sign_classifies_saddle_extremum_and_perturbed_refusal
  - contact_cert_three_valued_contract_holds
  - perturbed_near_tangency_returns_disproven_with_certified_gap
  - genuine_contact_returns_tangency_at_tolerance
  - a2_cusp_branch_needs_c3_and_refuses_without_it
  - no_r5_enclosure_required_for_classification
  - no_transcendental_call_in_contact_module
```

## Section 1 — the classifier (`kernel/contact.rs`, NEW)

The §10.3 pipeline over two patches p, q (CertifiedPatchC2 implementors —
the rational carriers from 104 are the fixture implementors; C3 only for
the A2 branch):

1. **Critical point (EXACT).** The common-normal graph: with n0 the shared
   normal direction at the contact (from the patches' normal cones; the
   recognized-carrier fixtures supply it in closed form), g = f1 - f2
   over Pi = n0^perp; solve grad g = 0 by `krawczyk_c1` (square 2x2 via
   the R4'-shaped normal-projection residual — the S1A pattern, built
   inline as a local `SquareResidualEval` over the two carriers'
   derivative enclosures). Proven -> `PointCert` (the EXACT critical
   point certificate).
2. **Gap (tolerance-tagged).** gap = the interval enclosure of g at the
   certified critical point (homogeneous evaluation per N5 for rational
   carriers; the enclosure is a plain interval of g's VALUES).
3. **Hessian sign (EXACT).** H = II1 - II2 in the common basis of Pi per
   Theorem 10.1 (with the n2 = -n0 sign flip VERBATIM — test it);
   det H sign via the landed `CertifiedSign`/expansion discipline.
4. **Classification (Corollary 10.2 verbatim).** det H < 0 -> MorseSaddle
   (emit the TopoNode variant data — the classification OUTPUT is the
   enum name as data; no graph assembly here); det H > 0 ->
   MorseExtremum; rank H = 1 + certified nonzero cubic in the null
   direction (C3 jets, the composed closed form: f_i's 3-jet through the
   inverse 2-jet of the projection — the composition is polynomial for
   rational carriers) -> A2Cusp; else -> `Refused(HighOrderJet)`.
5. **The three-valued contract (Prop 10.3).** 0 not in gap ->
   `ClaimVerdict::Disproven(SeparationWitness { gap })` (certified
   separation or crossing — a GOOD outcome, the ordinary path resumes);
   0 in gap and width > TOL_INTERSECTION -> `Inconclusive`; 0 in gap and
   width <= tolerance -> `Proven(ContactCert::try_new(...))` tagged
   TangencyAtTolerance. `SeparationWitness { pub gap: Interval }` is this
   packet's one new type (the Disproven arm's carrier; recorded as the
   S5a-owned outcome vocabulary from the shim's design note).

The audit row (spec section 20, S5a): the classification needs NO
R5Enclosure in scope — `no_r5_enclosure_required_for_classification`
source-scan pins it.

## Section 2 — tests

The eight `tests_required` names; ground truths:
1. Sphere/plane from the shim kit at tangency (plane z=1, sphere r=1 at
   origin): critical point (0,0,1)-chart-exact, det H sign certifiable,
   gap contains 0 with width <= tolerance -> Proven TangencyAtTolerance.
2. Same pair with the plane at z = 1 + 1e-3 (deliberate perturbation):
   Disproven with a certified gap NOT containing 0 (Prop 10.3's point:
   no false contact).
3. Plane z = 1 + 1e-12 (inside the tolerance): Proven — the tag is
   HONEST about tolerance-relative truth.
4. Equal-radius cylinders crossing at right angles (transversal): the
   classifier is not consulted (gap excludes 0 -> Disproven) — asserts
   the ordinary-path resume.
5. Saddle fixture (two spheres of different radii with a crossing
   contact arrangement constructed via the coaxial kit + offset):
   MorseSaddle; the n2 = -n0 flip test (flip the second carrier's normal
   convention and assert the classification is INVARIANT).
6. A2Cusp: needs C3 — with a C2-only implementor the A2 branch refuses
   `HighOrderJet` (the trait boundary is the audit); a C3 fixture
   (polynomial patch with a cubic degeneracy, constructed from
   leaf_from_control data) classifies A2Cusp.
7. Backing-table parity with S2A's C1 (the critical-point stage).
8. Source scan.

House rules: H-1; H-3 same-line; fmt + clippy (exact verify form,
unfiltered, ALL findings) clean; `cargo check --workspace --all-targets`
green. CARGO_BUILD_JOBS=2-4. COMMIT BEFORE RESULT.json (explicit final
turns: add, commit, then RESULT).

## Stop conditions

1. A frozen seam shape differs — stop, record the diff.
2. The II1 - II2 composition needs surface data the CertifiedPatchC2
   enclosures do not carry (second FUNDAMENTAL form coefficients need
   cross-derivatives suv — SecondDerivativeEnclosure has it; if a needed
   quantity is genuinely absent from the frozen shim shapes, STOP and
   name it — that is a shim amendment, not an improvisation).
3. A fixture's classification cannot be certified honestly (gap width
   sits exactly at the tolerance boundary) — record both numbers; the
   three-valued contract decides, not a preference.

Commit subject: `feat(certified): contact classifier + ContactCert
(BG-KV2-302-S5A)`.
