# BG-KV2-304-S3B — Tier-2: the Psi_a critical-point start set (additive arity-4 C1)

Wave-3 implementation packet (build spec §4; §19 row 14; spec §9.2 verbatim
— Theorem 9.2, Corollary 9.3, the k_a retry rule). Completeness after
Tier-1 fails: every connected component of Z either meets the boundary or
contains a zero of Psi_a(x) = (F(x), a.m(x)) — a SQUARE 4x4 system. The
frozen `SquareResidualEval`/`krawczyk_c1` seam covers arity 2-3; Tier-2
needs **arity 4**, landed ADDITIVELY here (the N3CERT pattern, second
application): `PointCert4` in certs.rs, `krawczyk_c1_n4` in engine.rs.

```yaml
id:          BG-KV2-304-S3B
contract:    [BG-KV2-304-S3B]
class:       design
crates:      [truck-certified]
depends_on:  [BG-KV2-301-S03A, BG-KV2-201-S2A, BG-KV2-206-N3CERT]
write_allow:
  - vendor/truck/truck-certified/src/kernel/tier2.rs
  - vendor/truck/truck-certified/src/kernel/certs.rs
  - vendor/truck/truck-certified/src/kernel/engine.rs
  - vendor/truck/truck-certified/src/kernel/mod.rs
  - vendor/truck/truck-certified/tests/kernel_tier2.rs
read_allow:
  - docs/CONSTRUCTIVE_GEOMETRY_KERNEL_SPEC_V2.md
  - vendor/truck/truck-certified/src/kernel
  - vendor/truck/truck-certified/src/kernel/minor_algebra.rs
budget:      {turns: 30, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub fn minor_vector_encl' vendor/truck/truck-certified/src/kernel/minor_algebra.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn krawczyk_c1_n3' vendor/truck/truck-certified/src/kernel/engine.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub struct PointCert3' vendor/truck/truck-certified/src/kernel/certs.rs"}
  - {id: A4, expect: 0, cmd: "grep -c 'PointCert4' vendor/truck/truck-certified/src/kernel/certs.rs"}
tests_required:
  - point_cert4_and_n4_entry_additive_and_gated
  - psi_a_zeros_isolated_on_a_transversal_fixture
  - exclusion_clears_the_remainder
  - persistent_positive_dimensional_psi_a_routes_to_tangential
  - ka_perturbation_retries_then_incomplete_start_set
  - boundary_seeds_plus_psi_a_cover_every_oracle_component
  - no_transcendental_call_in_tier2_module
```

## Section 1 — the additive arity-4 carrier (`certs.rs` + `engine.rs`, append ONLY)

`PointCert4 { residual, box_: IBox4, rho }` with the same try_new gate as
PointCert3; `krawczyk_c1_n4(g: &dyn SquareResidualEval, b: IBox4, w:
&[CertifiedPositive]) -> ClaimVerdict<PointCert4, Refusal, Reason>` — the
same operator discipline, 4x4 adjugate path (the S2A engine's internals;
if its helpers are private, this packet's write set includes engine.rs —
extend minimally, do not restructure). Backing table identical. Residual
stamping: R1 internally, caller rebuilds (the documented one-line seam).
`point_cert4_and_n4_entry_additive_and_gated` pins the constructor gates.

## Section 2 — Tier 2 (`kernel/tier2.rs`, NEW)

- The R3 residual: `PsiA { sys: &SquareSystem3, a: [f64; 4] }` implementing
  `SquareResidualEval` (arity 4): eval = (F enclosure, a.m enclosure) —
  the m enclosure from `minor_algebra.rs::minor_vector_encl` composed with
  the Jacobian enclosure; a.m via the Theorem 6.4(iii) identity (the
  landed `a_dot_m`).
- `pub fn tier2_start_set(sys, a, domain: IBox4) -> TierTwoOutcome`:
  subdivide the domain (DEPTH_MAX cap); on each cell: exclusion first
  (0 not in the enclosure of a.m — cheap form, N7) else Krawczyk on
  PsiA. Outcome vocabulary: `Complete { start_set: Vec<PointCert4> }`
  (every zero isolated, remainder excluded), `Refused(Refusal)`.
- The a-posteriori genericity rule (§9.2): subdivision stalls at
  DEPTH_MAX without isolation -> perturb a and retry, up to KA times
  (`ka_perturbation_retries_then_incomplete_start_set` pins the count);
  each retry is a recorded, deterministic perturbation (a fixed table of
  KA alternative a vectors — unit-ish rational directions, no RNG).
- A persistent positive-dimensional Psi_a zero set (isolation fails
  because the zero set is a CURVE): `Refused(TangentialCurve)`
  (Inconclusive) — the signature routing to section 10.4, NOT
  IncompleteStartSet (`persistent_positive_dimensional_psi_a_routes_to_
  tangential` pins the distinction: stall WITH all-cells-contain-zero on
  a shrinking sub-box family -> tangential; stall with mixed -> Budget/
  IncompleteStartSet after KA).
- Corollary 9.3's composition: `boundary_seeds` (301's landed entry) +
  Psi_a zeros = the complete start set on a compact lifted leaf pair —
  `boundary_seeds_plus_psi_a_cover_every_oracle_component` runs both on
  a fixture whose oracle component count is known (the transversal
  fixture family: 2 components, both hit).

## Section 3 — tests

The seven `tests_required` names; ground truths rational (the fixture
systems are the shim kit + S1A-constructed SquareSystem3 instances with
known intersections). House rules: H-1; H-3 same-line; fmt + clippy
(exact verify form, unfiltered, ALL findings) clean; `cargo check
--workspace --all-targets` green. CARGO_BUILD_JOBS=2-4. COMMIT BEFORE
writing RESULT.json AT THE WORKTREE ROOT.

## Stop conditions

1. A frozen shape differs — stop, record the diff.
2. The 4x4 adjugate path needs restructuring of the S2A engine beyond
   minimal extension — stop, name the obstruction (a restructuring
   amendment is a separate decision).
3. The exclusion form cannot decide a fixture cell the float oracle
   decides — record the enclosure width; do not loosen.

Commit subject: `feat(certified): Tier-2 Psi_a start set + additive
arity-4 C1 (BG-KV2-304-S3B)`.
