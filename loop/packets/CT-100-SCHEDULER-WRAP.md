# WORK PACKET CT-100-SCHEDULER-WRAP — the global residual scheduler over the existing certified refinement

Implements build-spec Wave 1. The existing phase-3 refinement
(`integrate_patch`/the contact-cover loop in `certify_boolean_volume_impl`)
keeps its soundness machinery EXACTLY as is; this packet changes the
ACCEPTANCE POLICY: per-patch width budgets are replaced by the atlas
global scheduler (best-first over descending residual width, gate on the
GLOBAL bracket width). Same soundness (Theorem 25), better scheduling
(Theorem 27), full observability (the certificate gains the phase stats
the R4 diagnostics asked for).

```yaml
id:          CT-100-SCHEDULER-WRAP
contract:    [CT-100-SCHEDULER-WRAP]
class:       mechanical
crates:      [truck123d]
depends_on:  [CT-000-ATLAS-CONTRACT-SHIM]
needs:       [CT-000-ATLAS-CONTRACT-SHIM]
write_allow:
  - truck123d/src/bd_bridge.rs
  - truck123d/tests/atlas_scheduler_wrap.rs
read_allow:
  - docs/CONTACT_ATLAS_SPEC.md
  - docs/CONTACT_ATLAS_BUILD_SPEC.md
tests_required: [truck123d/tests/atlas_scheduler_wrap.rs]
anchors:
  - {id: A1, min: 5, cmd: "grep -cE '\\bPhaseStats\\b' truck123d/src/bd_bridge.rs"}
  - {id: A2, min: 6, cmd: "grep -cE '\\bBooleanVolumeCertificate\\b' truck123d/src/bd_bridge.rs"}
  - {id: A3, min: 1, cmd: "grep -cE '\\bmod atlas\\b' truck123d/src/lib.rs"}
budget:      {turns: 50, ctx_tokens: 150000}
```

Anchors at authoring time `e271ba9`; A3 is 1 AFTER CT-000 lands.
**Re-measure at dispatch** (G15/G16/G17 landings shift bd_bridge counts —
H-8 re-scope on mismatch).

## Pre-made judgements

1. **Refactor, not rewrite.** The refinement's per-cell certification
   (MONO-5 membership, eq.-5 brackets, subdivision) is untouched. The
   per-patch budget `target_width/(2*patch_count)` split is replaced by:
   seed the global queue with one residual per patch (its initial
   bracket), refine max-width-first under the global gate
   `W <= relative_tolerance * |V(A)|` (absolute/mixed per the landed
   options). Local cell acceptance becomes "child brackets are certified
   enclosures", not a local width test.
2. **The certificate keeps its shape.** `BooleanVolumeCertificate` gains
   the scheduler stats (refinement count, final global width, per-phase
   counters already in `PhaseStats`) — additive fields only; existing
   fields keep names/semantics so the battery is unaffected.
3. **No parallelism yet.** The scheduler is sequential (deterministic,
   spec §30); rayon is Wave 5 / explicit owner call.
4. **Refusal mapping**: queue empty or budget exhausted =>
   `RefinementBudgetExceeded` with the current sound bracket (typed, per
   spec §26). Never a widened tolerance.

## Done-when

1. `cargo check -p truck123d --tests --locked` green.
2. `cargo test -p truck123d --test atlas_scheduler_wrap --locked` green:
   (a) on the CT-000 fixtures, the wrapped funnel returns brackets
   containing the analytic volumes; (b) global-width gate holds on every
   accepted call; (c) the greedy order is observed on an instrumented
   run; (d) the previously-green fuse/nested-box tests keep their
   verdicts.
3. `cargo test -p truck123d --test fuse_fold --locked` green (V5 net).
4. fmt clean on touched files; `clippy -p truck123d --lib` clean on the
   diff.
5. Write `RESULT.json` AT THE WORKTREE ROOT (nowhere else), including the
   anchor re-measurement and before/after wall-time on one CT-000 fixture
   (observability only — never a published number).
