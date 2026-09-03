# BG-KV2-305-S2B — GraphCert, the R5 enclosure contract, and R4/R4-prime

Wave-3 implementation packet (build spec §4; §19 row 16; spec §8.5, §8.6,
§7 R4/R4-prime). The projection certificates: `GraphCert` (Theorem 8.3 —
a cone test with NO solve), the R5 enclosure contract (§8.6's five-step
pipeline — preimage via C1 on R4, value, gradient), and the R4/R4-prime
projection residuals. Consumes: `GraphCert`/`R5Enclosure` shim types,
`krawczyk_c1` (S2A), the rational carriers (104), `minor_algebra`
discipline.

```yaml
id:          BG-KV2-305-S2B
contract:    [BG-KV2-305-S2B]
class:       design
crates:      [truck-certified]
depends_on:  [BG-KV2-201-S2A, BG-KV2-104-RATCARRIER]
write_allow:
  - vendor/truck/truck-certified/src/kernel/projection.rs
  - vendor/truck/truck-certified/src/kernel/mod.rs
  - vendor/truck/truck-certified/tests/kernel_projection.rs
read_allow:
  - docs/CONSTRUCTIVE_GEOMETRY_KERNEL_SPEC_V2.md
  - vendor/truck/truck-certified/src/kernel
  - vendor/truck/truck-certified/src/kernel/minor_algebra.rs
budget:      {turns: 28, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub struct GraphCert' vendor/truck/truck-certified/src/kernel/certs.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub struct R5Enclosure' vendor/truck/truck-certified/src/kernel/certs.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub fn krawczyk_c1(' vendor/truck/truck-certified/src/kernel/engine.rs"}
  - {id: A4, expect: 0, cmd: "grep -c 'projection.rs' vendor/truck/truck-certified/src/kernel/mod.rs"}
tests_required:
  - graphcert_is_a_cone_test_with_no_solve
  - graphcert_injectivity_on_a_sphere_chart
  - graphcert_refuses_when_no_feasible_n0
  - r5_enclosure_preimage_via_c1_on_r4
  - r5_value_and_gradient_enclose_the_truth
  - r5_refusal_when_krawczyk_fails_at_depth_max
  - r4_fallback_prime_exercised_on_no_feasible_n0_fixture
  - no_bernstein_applies_to_r5_audit
  - no_transcendental_call_in_projection_module
```

## Section 1 — GraphCert (`kernel/projection.rs`, NEW)

`pub fn graphcert(p: &dyn CertifiedPatch, domain: IBox2, n0: [f64; 3])
-> Construction<GraphCert>` — Theorem 8.3 verbatim: det Dq = n0.N where
N = S_u x S_v; the certificate is 0 not-in the interval enclosure of
(n0.N) over the domain — a CONE TEST: n0.N's enclosure from the patch's
`normal_cone` and derivative enclosures; NO linear solve anywhere (the
`graphcert_is_a_cone_test_with_no_solve` source-scan pins: no matrix
inversion / LU / adjugate call in the GraphCert path). n0 feasibility:
for a leaf PAIR the feasible n0 comes from the two normal cones' LP (the
Tier-1 cos-space discipline — 301's `tier1_loop_free` shape reused on the
cone pair; cite, do not fork). Refuses: no feasible n0
(`graphcert_refuses_when_no_feasible_n0` — the tangential-adjacent
fixture; the caller subdivides or falls back to R4-prime).
`graphcert_injectivity_on_a_sphere_chart`: the rational sphere chart with
n0 = the chart's polar axis — 0 excluded, injective on the chart box.

## Section 2 — the R5 enclosure contract (§8.6's five steps, verbatim order)

`pub fn r5_enclose(p1: &dyn CertifiedPatch, p2: &dyn CertifiedPatch, q:
IBox2, n0: [f64; 3]) -> ClaimVerdict<R5Enclosure, Refusal, Reason>`:
1. Preimage: for each i, C1 on the R4 residual (Pi-proj(S_i(u,v)) = q —
   a 2x2 square system, a local `SquareResidualEval` over the patch's
   derivative enclosures) via `krawczyk_c1`; produces D_i' with
   sigma_i(Q) subset D_i' certified. Krawczyk stalls at DEPTH_MAX ->
   `Refused(R5EnclosureFailed)` (Inconclusive) — the named refusal.
2. Value: f_i(Q) subset of n0 . (interval S_i over D_i').
3. Gradient: grad f_i = (D sigma_i)^T (n0.S_u, n0.S_v)^T with D sigma_i =
   (Dq)^{-1}, enclosed by interval inversion of the Dq enclosure
   (nonsingular BY GraphCert — take the GraphCert as a value argument;
   §8.6's discipline: without it, refuse).
4. Hessian (C2 carriers only) — DEFERRED with a named predicate
   (`RefusalEvidence::Predicate { name: 'r5_hessian_is_s5a_contact' }`):
   the Hess f route composes into S5A's classifier, not this packet.
5. g = f1 - f2 and derivatives by subtraction.
Emit `R5Enclosure::try_new(q, preimage, certs)`. The audit:
`no_bernstein_applies_to_r5_audit` — a source test that no Bernstein
evaluation appears on the g path (g is analytic non-polynomial; the spec
makes asserting otherwise an audit failure).

## Section 3 — R4 / R4-prime

`pub fn r4_project(p: &dyn CertifiedPatch, q: [f64; 2], n0: [f64; 3]) ->
...` — the square 2x2 projection solve per surface INDEPENDENT of the
other (the C1 machinery IS the solver; this fn packages it). R4-prime
(fixed (u,v), the normal-projection residual P(u,v;s,t) = (S1_u.(S2-S1),
S1_v.(S2-S1))) as the fallback where no feasible n0 exists —
`r4_fallback_prime_exercised_on_no_feasible_n0_fixture` runs the fallback
on 302's tangential-adjacent fixture family and records the honest
outcome (Proven or Inconclusive — never a false Proven).

House rules: H-1; H-3 same-line; fmt + clippy (exact verify form,
unfiltered, ALL findings) clean; `cargo check --workspace --all-targets`
green. CARGO_BUILD_JOBS=2-4. COMMIT BEFORE writing RESULT.json AT THE
WORKTREE ROOT.

## Stop conditions

1. A frozen shape differs — stop, record the diff.
2. The R5 gradient's interval inversion needs machinery beyond the landed
   interval primitives (a 2x2 interval inverse) — implement it locally
   (adjugate/det over intervals, the landed det discipline); if that is
   insufficient for a needed fixture, stop and record the widths.
3. A fixture's GraphCert cannot exclude zero honestly — record the
   numbers; the fallback exists precisely for that outcome.

Commit subject: `feat(certified): GraphCert + R5 enclosure contract + R4/
R4-prime (BG-KV2-305-S2B)`.
