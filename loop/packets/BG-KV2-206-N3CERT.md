# BG-KV2-206-N3CERT — the arity-3 C1 carrier: PointCert3 + krawczyk_c1_n3

Micro-amendment packet (build spec §4, Wave 2). The shim froze
`PointCert.box_: IBox2` (a shim-time interpretation of the packet's bare
`IBox`, now landed and consumed by identity.rs/engine.rs/fixtures), so
S2A's `krawczyk_c1` can only emit Proven for arity-2 systems — and R8
(curve-surface, 3 eq in 3 unknowns, spec §7) has no typed Proven carrier.
The spec's C1 is n-generic (§8.2: "Square residual R : R^n -> R^n"); this
packet adds the n=3 carrier ADDITIVELY (the recorded spelling deviation,
same class as PsiMapKind). No existing type changes; no existing test
changes.

```yaml
id:          BG-KV2-206-N3CERT
contract:    [BG-KV2-206-N3CERT]
class:       design
crates:      [truck-certified]
depends_on:  [BG-KV2-201-S2A]
write_allow:
  - vendor/truck/truck-certified/src/kernel/certs.rs
  - vendor/truck/truck-certified/src/kernel/engine.rs
  - vendor/truck/truck-certified/tests/kernel_engine.rs
read_allow:
  - docs/CONSTRUCTIVE_GEOMETRY_KERNEL_SPEC_V2.md
  - vendor/truck/truck-certified/src/kernel
budget:      {turns: 16, ctx_tokens: 60000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub fn krawczyk_c1(' vendor/truck/truck-certified/src/kernel/engine.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub struct PointCert' vendor/truck/truck-certified/src/kernel/certs.rs"}
  - {id: A3, expect: 0, cmd: "grep -c 'PointCert3' vendor/truck/truck-certified/src/kernel/certs.rs"}
tests_required:
  - point_cert3_try_new_enforces_rho_and_finite_box
  - krawczyk_c1_n3_proves_a_known_3var_root
  - krawczyk_c1_n3_backing_matches_the_2d_table
```

## Section 1 — `certs.rs` (append ONLY)

```rust
/// The arity-3 zero-dimensional certificate (R8-class C1; the recorded
/// additive spelling for the spec's n-generic PointCert, §8.2/§16).
pub struct PointCert3 { pub residual: ResidualId, pub box_: IBox3, pub rho: f64 }
impl PointCert3 {
    pub fn try_new(residual: ResidualId, box_: IBox3, rho: f64) -> Result<Self, Refusal>;
    // same gate as PointCert::try_new: rho <= RHO_MAX (config), finite box.
}
```

## Section 2 — `engine.rs` (append ONLY)

```rust
/// The arity-3 C1 entry (R8-class): identical operator discipline to
/// krawczyk_c1 (Lemma 8.0 + §8.2), n=3 adjugate/det path, emitting
/// PointCert3. Weight bounds remain the §7.1 value argument.
pub fn krawczyk_c1_n3(
    g: &dyn SquareResidualEval,
    b: IBox3,
    w: &[CertifiedPositive],
) -> ClaimVerdict<PointCert3, Refusal, Reason>
```

Reuse the S2A engine's internals (the same module already implements the
3x3 adjugate/det discipline for the tube's D_yF block); the Disproven/
Inconclusive backing table is IDENTICAL to krawczyk_c1's. ResidualId
stamping: the caller's residual flows through a `ResidualId` parameter?
NO — keep the S2A convention: stamp R1 internally, caller rebuilds via
PointCert3::try_new with its own id (the documented one-line seam,
verbatim from S2A's notes).

## Section 3 — tests (append to `tests/kernel_engine.rs`)

The three `tests_required` names: constructor gates; a known 3-var root
(e.g. the R8-shaped line-pierce-plane fixture data as a raw
SquareResidualEval closure — the fixture does not need R8System, that is
S1A's); backing-table parity with the 2D entry (image-exits ->
Disproven-or-Inconclusive class; non-strict -> Inconclusive).

House rules: H-1; H-3 same-line; fmt + clippy (exact verify form) clean;
`cargo test -p truck-certified --lib --tests --no-fail-fast` green;
`cargo check --workspace --all-targets` green. CARGO_BUILD_JOBS=2-4.

## Stop conditions

1. Adding PointCert3 breaks an exhaustive match or an exhaustive test
   pinning the certificate types — record it; the match grows (additive),
   a pinning test that forbids growth is amended BY THIS PACKET with the
   reason stated.
2. The S2A engine internals are not reusable for the n=3 entry without
   restructuring — stop, name the obstruction.

Commit subject: `feat(certified): PointCert3 + krawczyk_c1_n3 (BG-KV2-206-
N3CERT)`.
