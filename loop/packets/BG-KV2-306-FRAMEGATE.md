# BG-KV2-306-FRAMEGATE — Frame::try_new gate correction: z_hat is a POINT

Micro seam-correction packet (build spec §4, Wave 3). S4A's measured
blocking finding (loop/results/BG-KV2-207-S4A.json, `blocking_finding`):
the shim's `Frame::try_new` refuses a non-unit `z_hat`. The spec (section
8.1) defines `Frame<N> = (z_hat, Q, q_tau, Q_perp, A)` where **z_hat is
the expansion point in R^n — no unit constraint exists on it in the spec;
the unit requirements are on the frame basis (q_tau unit, Q orthonormal)**.
The unit-z_hat gate was a shim-time over-constraint (BG-KV2-000 packet
text: "refuses non-unit q_tau or z_hat" — the z_hat half is wrong), and it
blocks every mid-branch frame rebuild (S4A fixture 3: `frame_zhat_not_
unit`). This packet corrects the gate TO SPEC. This is a gate FIX, not a
loosening: the constraint being removed does not exist in the normative
spec, and removing it restores the spec's contract.

```yaml
id:          BG-KV2-306-FRAMEGATE
contract:    [BG-KV2-306-FRAMEGATE]
class:       design
crates:      [truck-certified]
depends_on:  [BG-KV2-201-S2A]
write_allow:
  - vendor/truck/truck-certified/src/kernel/certs.rs
  - vendor/truck/truck-certified/tests/kernel_contract.rs
read_allow:
  - docs/CONSTRUCTIVE_GEOMETRY_KERNEL_SPEC_V2.md
  - vendor/truck/truck-certified/src/kernel
budget:      {turns: 12, ctx_tokens: 50000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub struct Frame' vendor/truck/truck-certified/src/kernel/certs.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'frame_is_orthonormal_and_q_tau_is_the_normalized_kernel_direction' vendor/truck/truck-certified/tests/kernel_contract.rs"}
tests_required:
  - frame_try_new_accepts_nonunit_z_hat_and_still_gates_the_basis
```

## Section 1 — the gate correction (`certs.rs`)

`Frame::try_new`: REMOVE the z_hat unit-norm check (z_hat is a point;
only finiteness is required — refuse non-finite z_hat). KEEP verbatim:
q_tau unit within TOL_JACOBIAN, Q orthonormal within TOL_JACOBIAN,
q_perp the complement of q_tau in Q, A finite. The doc comment on Frame
gains the spec citation (section 8.1: z_hat is the expansion point;
the basis carries the unit constraints).

## Section 2 — tests (`tests/kernel_contract.rs`, in place)

The shim contract test that pinned the old behavior keeps its LANDED NAME
(`frame_is_orthonormal_and_q_tau_is_the_normalized_kernel_direction`,
V5 identity) and its assertions are updated IN PLACE: the orthonormality
and q_tau assertions stand; any z_hat-unit assertion is replaced by a
finiteness assertion plus a non-unit-z_hat acceptance case (the new
`frame_try_new_accepts_nonunit_z_hat_and_still_gates_the_basis` covers
both directions: accepts z_hat = [0.3, 0.7, 1.2, 0.5]-class non-unit
point with a valid basis; refuses non-unit q_tau; refuses non-finite
z_hat).

## Done-when

`cargo test -p truck-certified --lib --tests --no-fail-fast` green (all
landed suites unchanged except the in-place assertion update);
`cargo check --workspace --all-targets` green; fmt + clippy (exact verify
form, unfiltered) clean on the packet's files. CARGO_BUILD_JOBS=2-4.
COMMIT BEFORE writing RESULT.json AT THE WORKTREE ROOT.

## Stop conditions

1. Any landed consumer of Frame::try_new DEPENDS on the unit-z_hat
   refusal (grep the call sites; S2A's build_frame4 constructs frames at
   box centers — if any site relies on the refusal, stop and name it;
   the correction then grows by amendment).

Commit subject: `fix(certified): Frame::try_new gate to spec - z_hat is a
point (BG-KV2-306-FRAMEGATE)`.
