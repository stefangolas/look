# BG-KV2-207-S4A — the float tracer: predictor-corrector with the escalation ladder

Wave-2 implementation packet (build spec §4; §19 row 8; spec §10.1–§10.2).
Lands the D4 float predictor-corrector: fast, UNCERTIFIED proposals whose
accept/reject path always goes through the certified seam. **The certified
seam is frozen verbatim (S2A + N3CERT spellings): `build_frame4` (frames),
`c2_certify_tube4` (the tube attempt), `PointCert`/`ArcCert` emissions.**
The landed certified tracer (`ssi_trace.rs::certified_pair_trace`) is
untouched — this is the new fast path beside it.

Doctrine (spec §10.1, normative): monotone in tau ONLY (never impose 3D
strong monotonicity — helices and wraps fold in R^3 while remaining graphs
in the local frame); long arcs (accept the largest I_tau that passes C2,
grow aggressively); batch interval work is the CERTIFIED side's business —
the predictor reuses the last factorization and re-factors only when
kappa exceeds KAPPA_MAX.

```yaml
id:          BG-KV2-207-S4A
contract:    [BG-KV2-207-S4A]
class:       design
crates:      [truck-certified]
depends_on:  [BG-KV2-201-S2A, BG-KV2-206-N3CERT]
write_allow:
  - vendor/truck/truck-certified/src/kernel/tracer.rs
  - vendor/truck/truck-certified/src/kernel/mod.rs
  - vendor/truck/truck-certified/tests/kernel_tracer.rs
read_allow:
  - docs/CONSTRUCTIVE_GEOMETRY_KERNEL_SPEC_V2.md
  - vendor/truck/truck-certified/src/kernel
  - vendor/truck/truck-certified/src/ssi_trace.rs
budget:      {turns: 30, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub fn c2_certify_tube4' vendor/truck/truck-certified/src/kernel/engine.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn build_frame4' vendor/truck/truck-certified/src/kernel/engine.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub enum SegmentBreak' vendor/truck/truck-certified/src/kernel/graph.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub const KAPPA_MAX' vendor/truck/truck-certified/src/kernel/config.rs"}
  - {id: A5, expect: 0, cmd: "grep -rnw 'float_trace' vendor/truck/truck-certified/src | wc -l"}
tests_required:
  - tracer_marches_a_straight_branch_and_certifies_long_arcs
  - dtau_grows_on_success_and_halves_on_failure
  - frame_rebuild_after_max_halvings_continues_the_branch
  - escalation_routes_rank2_zero_set_to_tangential_refusal
  - escalation_routes_isolated_r2_to_the_contact_future
  - high_order_singularity_refuses
  - monotone_in_tau_only_no_strong_monotonicity_imposed
  - tracer_output_never_claims_certification
```

## Section 1 — the loop (`kernel/tracer.rs`, NEW)

```rust
pub struct TracePolicy { pub arc_step0: f64, pub grow: f64, pub shrink: f64,
                         pub max_halvings: u32, pub max_frame_rebuilds: u32,
                         pub max_steps: usize }
impl TracePolicy { pub fn default() -> Self; }  // arc_step0 0.05, grow 2.0,
                                                // shrink 0.5, halvings 3, rebuilds 2, steps 4000
pub struct FloatStep { pub tau: f64, pub point: [f64; 4], pub dtau: f64,
                       pub certified: Option<ArcCert<4>> }  // Some = the C2
                       // attempt SUCCEEDED for the arc ending at this step
pub enum FloatOutcome {
    Completed { steps: Vec<FloatStep> },          // hit the box boundary
    ClosedLoop { steps: Vec<FloatStep> },         // identity recurrence (float tolerance detection,
                                                  // CERTIFIED closure is the promotion path's job)
    Refused(Refusal),
}
pub fn float_trace(sys: &SquareSystem3, seed: [f64; 4], policy: &TracePolicy) -> FloatOutcome;
```

- Seed certification FIRST: `build_frame4` + a C2 attempt on a small
  initial I_tau (the certified seam); if the seed itself cannot frame ->
  `Refused(Conditioning)` (Inconclusive).
- March: predictor = one Gauss-Newton step reusing the last factorization
  (float, per §10.1's cheap-predictor rule); corrector/confirm = the C2
  tube attempt over [tau, tau+dtau] via `c2_certify_tube4` with the
  CURRENT frame. Ok -> extend, then TRY GROWING dtau (accept the largest
  I_tau that passes: on success attempt 2x, keep the largest that
  certifies); fail -> halve; after max_halvings consecutive halvings ->
  `build_frame4` rebuild at the current point (a `FrameSwitch`
  SegmentBreak is RECORDED in the step stream as data on FloatStep? NO —
  FrameSwitch events land in the S9a graph wave; this packet records
  rebuilds in the step's `dtau` reset and RESULT notes; the segment-break
  event stream is a booked seam, do not invent a partial one here).
  After max_frame_rebuilds -> `Refused(Conditioning)`.
- Depth cap: DEPTH_MAX is the SUBDIVISION cap at the three D4 carve-out
  sites — the tracer's step cap is policy.max_steps; exceeding it is
  `Refused(Budget)` (Inconclusive).

## Section 2 — the escalation ladder (§10.2 verbatim routing)

When the C2 attempt fails, classify BEFORE retrying (the ladder's order is
normative):

1. sigma_min(DF) > 0 certified on the box (an interval check via the
   engine's Jacobian machinery — the smallest singular VALUE bound is
   computed by the landed adjugate/det discipline on the 3x4 system's
   blocks: use the certified margin on the selected continuation block,
   the F3 discipline): rebuild frame, retry C2 (conditioning, not
   geometry).
2. Parametric regularity fails (the EG - F^2 floor at the current
   parameter box): route to `Refused` with
   `RefusalEvidence::Predicate { name: "parametric_degeneracy_chart_or_
   carrier" }` — the chart-switch MECHANISM is §3.4's (later wave); the
   refusal names the route.
3. The R2 rank test shows the contact zero set is 1-dimensional:
   `Refused(TangentialCurve)` (Inconclusive) — NEVER trace (§10.4). The
   rank test: the two contact-row enclosures vanish jointly on the box
   while F does not isolate — implement as the enclosure-based rank
   screen the spec describes (DG's rows jointly containing zero with the
   tangency witness), and PIN it with a fixture.
4. R2 zero set isolated: this packet REFUSES with
   `RefusalEvidence::Predicate { name: "isolated_contact_is_s5a" }` —
   S5a owns the ContactCert path (Wave 3); the refusal is the seam.
5. Otherwise: `Refused(HighOrderJet)` (spec's HighOrderSingularity).

`tracer_output_never_claims_certification`: FloatStep.certified carries
ONLY genuine ArcCert emissions from the frozen seam; the predictor's own
points are data, never certificates — a source test pins that no
FloatStep field is constructed outside the c2_certify_tube4 call path.

## Section 3 — tests

The eight `tests_required` names; ground truths:
1. Straight branch (the diagonal fixture family): marches to the box
   boundary; the certified arc total covers >= half the branch (long-arc
   growth working: assert dtau grew from arc_step0 by >= 2x at some step).
2. A deliberately failing C2 region: dtau halves (assert a step with
   dtau < arc_step0), then grows again past the hard part.
3. A branch that needs a frame rebuild (curved fixture): completes after
   rebuilds (assert the rebuild count in RESULT notes; the outcome is
   Completed).
4. A near-tangential fixture (coaxial-adjacent carriers from the shim
   kit's rational forms): the ladder routes to TangentialCurve (assert
   the refusal kind), never a traced arc.
5. An isolated-contact fixture: the named s5a-seam refusal.
6. A degenerate-jet fixture: HighOrderJet.
7. Helix-like wrapped branch (cylinder rational carrier pair with angular
   advance): monotone in tau only — the model-space image folds but the
   trace completes (this is the anti-strong-monotonicity pin).
8. Source scan: FloatStep.certified constructed only inside the C2 seam
   call.

House rules: H-1; H-3 same-line opt-outs for predictor tolerances; fmt +
clippy (exact verify form, unfiltered, ALL findings) clean; `cargo check
--workspace --all-targets` green. CARGO_BUILD_JOBS=2-4.

## Done-when

- `cargo test -p truck-certified --lib --tests --no-fail-fast` green.
- RESULT.json AT THE WORKTREE ROOT, including the rebuild/step counts for
  fixtures 1-3 and 7.

## Stop conditions

1. The frozen C2 seam (`c2_certify_tube4`/`build_frame4`) differs from
   the spellings above — stop, record the diff.
2. An escalation route cannot be decided from enclosures alone and needs
   a float heuristic on the DECISION (not the proposal) — stop; that is a
   D4 violation the spec forbids.
3. The tracer needs to emit SegmentBreak events to function — stop; that
   stream is S9a's contract and gets frozen there, not improvised.

Commit subject: `feat(certified): float tracer + escalation ladder
(BG-KV2-207-S4A)`.
