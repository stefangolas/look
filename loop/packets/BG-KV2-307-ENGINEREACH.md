# BG-KV2-307-ENGINEREACH — engine reach: frame robustness, tube-reach characterization, ladder margins

Wave-3 engineering packet (build spec §4). Owns the three S4A blockers
(loop/results/BG-KV2-207-S4A.json `blocking_finding`, measured, verbatim
context): (1) build_frame4's Gram-Schmidt trips the TOL_JACOBIAN
orthogonality gate near degenerate seeds (observed dots 1e-11 to 1e-9);
(2) the tube's single-frame certified reach (~0.3-0.35 tau even on linear
branches) is UNMEASURED as an envelope; (3) the ladder's rung-3/4/5
separation needs rank margins the current hulls cannot deliver at scale.

Doctrine anchors: the frame construction is a D4 PREDICTOR (the tube
certificate re-validates everything it needs inside the Krawczyk
operator); kappa_max = 1e6 is the spec's conditioning-rebuild bound
(§0.4/§10.1); two-pass Gram-Schmidt ("reorthogonalization") is the
standard numerically-stable construction and is provably near-orthogonal
to machine precision for non-degenerate inputs.

```yaml
id:          BG-KV2-307-ENGINEREACH
contract:    [BG-KV2-307-ENGINEREACH]
class:       design
crates:      [truck-certified]
depends_on:  [BG-KV2-201-S2A, BG-KV2-207-S4A]
write_allow:
  - vendor/truck/truck-certified/src/kernel/engine.rs
  - vendor/truck/truck-certified/tests/kernel_engine.rs
read_allow:
  - docs/CONSTRUCTIVE_GEOMETRY_KERNEL_SPEC_V2.md
  - vendor/truck/truck-certified/src/kernel
  - loop/results/BG-KV2-207-S4A.json
budget:      {turns: 24, ctx_tokens: 80000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub fn build_frame4' vendor/truck/truck-certified/src/kernel/engine.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn c2_certify_tube4' vendor/truck/truck-certified/src/kernel/engine.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub const KAPPA_MAX' vendor/truck/truck-certified/src/kernel/config.rs"}
tests_required:
  - frame_two_pass_orthonormality_near_degenerate_seeds
  - frame_conditioning_rebuild_gate_uses_kappa_max
  - tube_reach_envelope_measured_and_published
  - ladder_margin_refusals_are_budget_backed
  - s4a_remaining_three_tests_pass_after_engine_fix
```

## Section 1 — frame robustness (`engine.rs`, build_frame4 only)

- TWO-PASS Gram-Schmidt: orthogonalize the perpendicular basis against
  q_tau, then REPEAT the pass (each vector re-projected against the
  others). Two passes drive the residual dot products to machine
  precision for every non-degenerate input; the observed 1e-11..1e-9
  dots fall below the 1e-12 gate WITHOUT touching the gate. If a
  residual dot still exceeds TOL_JACOBIAN after two passes, the input is
  genuinely near-rank-collapse: refuse Conditioning (Inconclusive) —
  the caller subdivides (spec section 9.2's k_a discipline does not
  apply here; the seed quality is the caller's).
- Conditioning REPORT, not a new gate: build_frame4 additionally returns
  (or logs into the certificate evidence) the frame's kappa estimate
  (the ratio of the largest to smallest basis-vector extension
  encountered); a kappa above KAPPA_MAX marks the frame
  rebuild-recommended in the evidence (spec section 10.1's rebuild rule)
  — the gate itself stays the orthonormality check.

## Section 2 — tube-reach envelope (measurement, published)

- `pub fn tube_reach_probe(sys: &SquareSystem3, seed: [f64; 4]) ->
  ProbeReport` — binary-search the largest I_tau width c2_certify_tube4
  certifies from a seed (grow-then-halve on the tau width at fixed
  perpendicular width), and the largest perpendicular half-width at a
  fixed tau width. Run over the S4A fixture family + the shim kit
  fixtures; PUBLISH the measured reach table in the RESULT and the test
  doc (linear ~0.3-0.35; quadratic: measure it; wrapped cylinder: measure
  it). The envelope is a PUBLISHED characteristic (spec section 18
  discipline: profile, do not tune against), and the tracer's policy
  defaults may be updated to the measured reach (arc_step0 = the linear
  reach's conservative fraction) — recorded, machine-checked.
- The 3x perpendicular-width-vs-tau-width observation: confirmed or
  corrected by the probe; the number goes in the doc.

## Section 3 — ladder margins (honest refusal discipline)

Rungs 3/4/5 need "certified rank margins" the hulls cannot deliver when
components cluster within one floor arc. The honest contract (spec
section 2 rules 2-3: Inconclusive is not Disproven; failure licenses
shrink, re-frame, escalate, or refuse):
- When the rank margin enclosure straddles at DEPTH_MAX: the ladder
  refuses `Refused(Budget)` (Inconclusive) with
  RefusalEvidence::Residual naming the straddling margin — NEVER a
  guessed rung. `ladder_margin_refusals_are_budget_backed` pins that
  clustered-component fixtures produce Budget-backed refusals, not
  misrouted TangentialCurve/HighOrderJet.
- S4A's three remaining tests re-run against the fixed engine and the
  honest-margin contract; where a test asserted a clean rung decision
  the hulls cannot certify, the test's assertion becomes the
  Budget-backed refusal class (recorded per test in RESULT notes — the
  refusal is the envelope line, the session-39 vertex-touch precedent).

## Done-when

- `cargo test -p truck-certified --lib --tests --no-fail-fast` green —
  ALL suites including S4A's `tests/kernel_tracer.rs` (the three
  previously-failing tests now pass or assert the Budget-backed
  envelope).
- `cargo check --workspace --all-targets` green; fmt + clippy (exact
  verify form, unfiltered, ALL findings) clean on packet files.
- CARGO_BUILD_JOBS=2-4. COMMIT BEFORE writing RESULT.json AT THE
  WORKTREE ROOT. The tube-reach table is IN the RESULT.
- **QUEUE RULE (session 50): all cargo invocations go through the queue —
  the `cargo` on PATH IS the queue shim (`loop/cargoq/`). Do not invoke
  cargo by absolute path; do not unset the shim. Direct execution is
  logged as a bypass (`fallback.log`) and is treated as an orchestrator
  fault, not a worker fault.**

## Stop conditions

1. Two-pass Gram-Schmidt still trips the gate on a NON-degenerate seed —
   stop; that is a real bug in the composition, record the numbers.
2. The tube-reach probe on the LINEAR fixture contradicts the measured
   ~0.3-0.35 (e.g. wildly larger after the frame fix) — record both; the
   reach characterization supersedes the anecdote either way.
3. An S4A test cannot pass NOR become an honest Budget-backed refusal —
   stop, name the test; that is an orchestrator decision, not an
   improvisation.

Commit subject: `feat(certified): engine reach - two-pass frames, tube-
reach envelope, ladder margin discipline (BG-KV2-307-ENGINEREACH)`.
