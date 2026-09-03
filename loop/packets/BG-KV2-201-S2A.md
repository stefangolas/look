# BG-KV2-201-S2A — certificate calculus: Lemma 8.0's rho, C1 generic, C2 tube, frame construction

Wave-2 implementation packet (build spec §4). Turns the shim's certificate
TYPES into the real engine over the landed interval core. Normative basis:
v2 spec §8.1–§8.3 (frames, Lemma 8.0, C1, Theorem 8.1 tube), §7.1 (weight
bounds as value arguments), §2 rule 5 (no coarser re-derivation). The
recorded F3-contract AMENDMENT (build spec §1 S2 row): the landed
"square 3x3 slice, tau frozen to a point" rule is EXTENDED additively — the
tube path evaluates the 3x3 perpendicular system over the JOINT box
(I_tau, B_perp) in frame coordinates. The landed square-slice path is
untouched (V5 identity: every existing ssi/trace test stays green).

```yaml
id:          BG-KV2-201-S2A
contract:    [BG-KV2-201-S2A]
class:       design
crates:      [truck-certified]
depends_on:  [BG-KV2-102-LEAF, BG-KV2-103-IDENTITY]
write_allow:
  - vendor/truck/truck-certified/src/kernel/engine.rs
  - vendor/truck/truck-certified/src/kernel/mod.rs
  - vendor/truck/truck-certified/tests/kernel_engine.rs
read_allow:
  - docs/CONSTRUCTIVE_GEOMETRY_KERNEL_SPEC_V2.md
  - docs/KERNEL_V2_BUILD_SPEC.md
  - vendor/truck/truck-certified/src/kernel
  - vendor/truck/truck-certified/src/hull.rs
  - vendor/truck/truck-certified/src/ssi.rs
  - vendor/truck/truck-certified/src/ssi_types.rs
  - vendor/truck/truck-certified/src/ssi_trace.rs
  - vendor/truck/truck-certified/src/formal/exact.rs
budget:      {turns: 38, ctx_tokens: 120000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub struct KrawczykCertificate3' vendor/truck/truck-certified/src/ssi_types.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub struct Frame' vendor/truck/truck-certified/src/kernel/certs.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub struct ArcCert' vendor/truck/truck-certified/src/kernel/certs.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub const RHO_MAX' vendor/truck/truck-certified/src/kernel/config.rs"}
  - {id: A5, expect: 0, cmd: "grep -rnw 'c2_certify_tube4' vendor/truck/truck-certified/src | wc -l"}
  - {id: A6, expect: 1, cmd: "grep -c 'leaf_extract' vendor/truck/truck-certified/src/kernel/mod.rs"}
tests_required:
  - contraction_rho_matches_hand_computed_weighted_norm
  - c1_proves_unique_root_on_a_known_quadratic
  - c1_refuses_when_krawczyk_image_exits_the_box
  - c1_inconclusive_backing_when_inclusion_is_not_strict
  - weight_bound_is_a_value_argument_not_an_assumption
  - c2_tube_certifies_over_a_nontrivial_tau_interval
  - c2_tube_refuses_when_perpendicular_contraction_fails
  - frame_is_orthonormal_and_q_tau_is_the_normalized_kernel_direction
  - no_transcendental_call_in_engine_module
```

## Section 1 — the frozen seam (S1a consumes these EXACT shapes; do not rename)

```rust
// kernel/engine.rs
pub trait SquareResidualEval {
    /// Number of variables == number of equations (2 or 3).
    fn arity(&self) -> usize;
    /// Outward-rounded interval residual over the box (component i evaluated
    /// over ALL variables' intervals jointly).
    fn eval(&self, b: &[Interval]) -> Vec<Interval>;
    /// Outward-rounded interval Jacobian enclosure, row-major n x n.
    fn jac_encl(&self, b: &[Interval]) -> Vec<Vec<Interval>>;
}

pub fn krawczyk_c1(
    g: &dyn SquareResidualEval,
    b: IBox,
    w: &[CertifiedPositive],
) -> ClaimVerdict<PointCert, Refusal, Reason>;
```

`krawczyk_c1` is Lemma 8.0 + §8.2 C1 verbatim: z_hat = box center; A = the
interval-inverse of the midpoint Jacobian (the landed `adjugate3`/`det3`
discipline for n=3; the landed `formal/bezier_isect.rs` 2x2 Krawczyk is the
n=2 template — reuse its algorithms, do not reimplement differently);
K(B) = z_hat - A*R(z_hat) + (I - A*DR(B))*(B - z_hat); accept iff K(B) is
component-wise strictly inside B; then rho = max_i (M r)_i / r_i with
M = mag(I - A*DR(B)) and r = rad(B) (componentwise, refusing any r_i == 0
or non-finite with `RefusalKind::NonFinite`, Disproven); Proven arm emits
`PointCert::try_new(residual, b, rho)` (the shim constructor enforces
rho <= RHO_MAX). Weight handling per §7.1: `w` is a VALUE argument — the
caller obtained it from `CertifiedPatch::weight_bound`; `krawczyk_c1`
checks non-emptiness and refuses `WeightDegenerate` (Disproven) if w is
empty, and records the bounds into the emitted certificate's evidence. It
NEVER re-derives a weight bound (rule 5).

Disproven vs Inconclusive backing (§2 rule 2): K(B) ∩ B = empty ->
`Disproven` with `RefusalEvidence::Residual` (the residual IS the evidence:
no root in B); K(B) inside but not strictly, or A singular ->
`Inconclusive`; failure to enclose -> `Inconclusive` (`Conditioning`).

## Section 2 — the tube (Theorem 8.1, additive F3 amendment)

```rust
pub fn c2_certify_tube4(
    sys: &SquareSystem3,
    frame: &Frame<4>,
    i_tau: Interval,
    b_perp: IBox,          // IBox<3>
    w: &[CertifiedPositive],
) -> ClaimVerdict<ArcCert<4>, Refusal, Reason>
```

Coordinate change per §8.1: x = z_hat + Q(tau*e1 + y); the perpendicular
residual F_perp(tau, y) = F(z_hat + Q(tau, y)) is evaluated by interval
composition (the landed hull kernels over the frame-transformed box; the
Jacobian block D_yF is 3x3 — SQUARE, so the F3 square-only rule is
respected: the amendment is only that the enclosure argument spans I_tau
jointly instead of a frozen slice point). K(I_tau, B_perp) per Theorem 8.1
with A from the midpoint of D_yF at (tau_mid, y_hat); accept iff the
perpendicular image is strictly inside B_perp for ALL tau in I_tau; rho
from M r over B_perp's radii; Proven emits `ArcCert::try_new(R1, frame,
i_tau, b_perp, rho, jac_encl, weights)` — the shim's ArcCert constructor
carries rho <= RHO_MAX and bans R2 (the type-level ban stays load-bearing:
this entry takes ResidualId::R1 internally and refuses any caller attempt
to claim another id with `RefusalEvidence::Predicate { name:
"r2_never_reaches_c2" }`).

Refusal backing: perpendicular image exits B_perp -> `Inconclusive`
(shrink-and-retry is licensed; failure is never evidence of no-branch);
A near-singular beyond KAPPA_MAX -> `Conditioning`, Inconclusive (the
caller rebuilds the frame, §10.2).

## Section 3 — frame construction (float predictor, deterministic, sqrt-only)

```rust
pub fn build_frame4(sys: &SquareSystem3, z_hat: [f64; 4])
    -> Construction<(Frame<4>, [f64; 4])>;   // frame + the float kernel dir m
```

m = maximal-minor vector at z_hat with EXACTLY Theorem 6.4's sign pattern
as landed in `ssi_trace.rs:507-510` (`d0 = minor([1,2,3]); d1 =
-minor([0,2,3]); d2 = minor([0,1,3]); d3 = -minor([0,1,2])`); q_tau =
m/||m|| (IEEE sqrt only — bit-reproducible); Gram-Schmidt the perpendicular
basis in FIXED index order; the returned Frame passes
`Frame::try_new`'s orthonormality gate at TOL_JACOBIAN. If ||m|| ~ 0
(below the normative floor) -> `RefusalKind::Conditioning` (Inconclusive):
the caller subdivides or switches coordinate — rank 2 is S0/S5a territory,
not this packet's. NO SVD (deterministic Gram-Schmidt only; record why in
the module doc: N4 cross-platform bit-reproducibility).

## Section 4 — tests (`tests/kernel_engine.rs`, NEW)

The nine `tests_required` names; machine-checked ground truths:
1. rho hand-computed on a 2x2 linear fixture (M and r computed by hand in
   the test, equality within exact f64 reproduction of the same op order).
2. A quadratic with a known root (x^2 - 2 = 0 style, box around the root):
   Proven with rho <= RHO_MAX; the root of the emitted PointCert's box
   contains the known root.
3. A box where the image exits: Disproven-or-Inconclusive per the backing
   table (assert the class, not the variant).
4. Non-strict inclusion -> Inconclusive backing.
5. Empty w -> WeightDegenerate; a fixture leaf's weight_bound output feeds
   straight through (value-argument discipline).
6. The tube: a straight-line branch (the diagonal fixture family from
   ssi_fixtures, reframed) certified over I_tau of width ~0.1 — the
   ArcCert's i_tau carries the interval, rho <= RHO_MAX.
7. A deliberately tilted frame / too-wide I_tau: Inconclusive
   (Conditioning), never a wrong Proven.
8. build_frame4 on a fixture system: Q orthonormal within TOL_JACOBIAN;
   q_tau proportional to the hand-computed minor vector.
9. Source scan: no `sin|cos|atan2|exp|ln|log|powf` outside comments in
   engine.rs (`sqrt` is permitted and used only in frame normalization).

House rules: H-1 (no unwrap/expect/panic; crate deny covers); H-3 same-line
opt-outs for the fixture tolerances; fmt + clippy (exact verify form,
unfiltered, ALL findings) clean on packet files; `cargo check --workspace
--all-targets` green (downstream ripple once).

## Done-when

- `cargo test -p truck-certified --lib --tests --no-fail-fast` green
  (everything landed stays green — the F3 amendment is additive).
- `cargo check -p truck-certified --all-targets` + workspace check green.
- CARGO_BUILD_JOBS=2-4 on every invocation; RESULT.json AT THE WORKTREE
  ROOT.

## Stop conditions

1. The landed ssi Krawczyk internals cannot be reused without modifying
   ssi.rs (outside the write set) — stop, name the blocker; reusing the
   ADJUGATE/DET kernels via `hull.rs`/`formal::exact` is the expected path,
   restructuring ssi.rs is not.
2. Lemma 8.0's rho cannot be extracted from the interval image without a
   transcendental — stop; the design is wrong somewhere upstream.
3. The tube over I_tau needs N5 division by a W enclosure that is not
   certifiably positive on the joint box — refuse WeightDegenerate and
   record; do not divide on an uncertified denominator (N6).

Commit subject: `feat(certified): Lemma 8.0 rho + C1 generic + C2 tube +
frame construction (BG-KV2-201-S2A)`.
