"""Session-49 wave-close STATE.md volatile rewrite: replaces lines 12..126
(the open-ended wave-in-flight section) with the closed-wave record."""
from pathlib import Path

P = Path(__file__).resolve().parents[1] / 'STATE.md'
lines = P.read_text(encoding='utf-8').splitlines(keepends=True)

NEW = """Updated 2026-09-02, session 49, wave close. **THE PHASE-2 WAVE IS CLOSED:
shim landed, all three members ran in PARALLEL, integrated, measured, rows
DONE.** The spline census was CANCELLED by the owner (gate waived). The
session-48 "shim landed at 37b0267" claim was FALSE and is corrected here
and in the ledger.

## Where we are

- **Wave base = a27edaa** (shim BG-CK-P2-CONTRACT landing merge; verified
  at 1b5ca21; filing 6932197). The shim r1 STOPPED on the packet's own
  false Section 3 premise (the FLOOR r2 dev-dep edge never landed);
  worker caught it (stop condition 3, E0433 recorded); r2 amendment
  f2fca2c supplied the premise; resumed session landed edge + shim.
- **W1 BG-CK-P2-SYSTEM DONE** (59ade56, merged 3d991aa): SYSTEM+KRAWCZYK3
  collapsed; src/ssi.rs 961 lines (constructor, frozen coordinate rule,
  3x3 Krawczyk adjugate/det over CertifiedInterval,
  strict-inclusion-only emission) + tests.
- **W2 BG-CK-P2-TRACE DONE** (v1 7e671e8; amendment e4a5fc2, merged
  62127f5): continuation loop + frozen both-certificate switching;
  solver-private BranchCertifier; amendment activated the production
  seam - pub fn certified_pair_trace(&RationalBipatch,
  &RationalBipatch, [f64;4]) -> Result<TraceOutcome, SsiRefusal> over
  W1's pipeline, fixture-driven integration tests green first-try.
- **W3 BG-CK-P2-RESIDUAL DONE** (v1 b5487d5; amendment c0b8117, merged):
  harness seeds re-walk reproduced the booked prevalence totals EXACTLY
  (60,438); amendment filled the marked seam and RAN the measurement.
- **THE MEASUREMENT (the wave's published output, in
  docs/CERTIFIED_PHASE2_FLOOR.md + RESULT):** 60,438 booked spline
  face-pairs -> 726 admitted (funnel: 21,566 admission_refused; 9,356
  rational-form mismatch; 28,790 non-spline-carrier in the walk);
  226,654 patch-pair units from the product rule -> 400 traced (0.83%
  completion in 279 s, truncation PUBLISHED not hidden); refusals typed
  (3 non_transverse, 2 conditioning, 1 singular); certify_rate 0.0 on 6
  completed pairs - statistically empty, honestly published.
- **THE PHASE-2 FINDING (next booking's input, the FLOOR-STOP
  analogue):** the admission funnel + patch extraction eat ~99% of the
  mass before the certified engine is consulted, and the patch-pair
  product explodes. The engine WORKS (real pairs, real traces, typed
  refusals end to end); the pipeline AROUND it is where the mass dies.
  Owner decisions live in: DISPATCH-2 (special-position arms), the
  recognizer family (geometric-naturalness grounds - census waived),
  admission-screen widening, decomposition policy (multi-patch faces),
  and the Phase-1 FLOOR anomaly (4,381 adjacent certified_disjoint,
  STOP filing on record). docs/CERTIFIED_INTERLEAVE_BUILD_SPEC.md's
  manifest carries the full record.
- **Composed-HEAD battery (the wave's ONE authoritative verification):**
  cargo test -p truck-certified --lib --tests 559 + all suites green;
  harness 4 green; kernel-gates HEAD all P-3 + GATE-4 111/111; cargo
  check --workspace --all-targets green; clippy zero findings
  attributable to wave files (pre-existing grandfathered baseline
  unchanged). V8/V9 skipped on the recorded additive-only
  justification. Rows flipped DONE (loop/scripts/close_wave_ck_p2.py).

## Pick up here

1. **Owner decisions from the measurement** (the next booking): funnel
   admission, decomposition policy, DISPATCH-2 / recognizer family /
   admission widening. Do NOT book Phase-3 class-4 work against a
   certified chain that admits 1.2% of its corpus.
2. **Then the main line**: the constructive geometry kernel
   (docs/CONSTRUCTIVE_GEOMETRY_PLAN.md; CG-000..CG-009 DONE - read the
   plan for the remaining gates: Exeter regression gate after CG-004
   lands, CG-007 cert against the frozen mapping, completion list).
   The wave recipe (ORCHESTRATOR "build-spec spine workflow") is the
   template for the next build spec: contracts precede concurrency
   everywhere, INCLUDING amendment-time seams (freeze entry-point
   signatures in both amendment texts - paid serialization this
   session).
3. **If the certified chain gets its next packet**: the shim's fixture
   kit + certified_pair_trace are the substrate; the harness's funnel
   counters (not_admitted_reasons) are the instrument.
4. Environmental, do NOT chase: healing::tests::step_import;
   fillet::complex_surface; stepio assy/table/tessellate/oi/ioi;
   cone_topology debug panics; pre-existing fmt drift;
   ~65-126 pre-existing clippy findings in grandfathered certified
   modules under clippy 1.97.0 (recorded in the shim's RESULT).

## State of the machine, as left

- Watchdog RUNNING (pid 23884, LOOK_WATCHDOG_STAGNANT=3600).
- Slots: 0-3 all FINISHED (shim + W1/W2/W3), branches merged. Slot
  targets 0/1 were reclaimed mid-session (10.7 GB was the shim verify's
  baselines); 2/3 remain warm; reclaim freely - the wave is closed.
- RAM 15.7 GB is the binding constraint: CARGO_BUILD_JOBS=2-4 for every
  cargo invocation (two cold warm-build OOMs at default jobs recorded);
  RUSTC_WRAPPER=sccache installed and warm for the dep universe
  (NOTE: sccache rejects the incremental dev profile - W3 unset it
  locally; workers may need RUSTC_WRAPPER unset for test runs).
- Disk ~16 GB free at wave close. Reclaim order: idle slot targets,
  repo-root target (5.0 GB, regenerates), %TEMP% (a 12 GB
  proc-macro-srv reboot leak was reclaimed once).
- LOC ledger (re-derive): git diff --shortstat da72cd5..HEAD -- vendor/truck.

## The parallelism picture

Wave mode is PROVEN end to end this session: contracts frozen in one
shim packet (through the normal loop); three implementation workers in
PARALLEL on disjoint write sets doing scoped checks only; integration
sweep with zero seam conflicts; amendments to owning sessions via
--resume (W2/W3 amendments first-try LOCAL_GREEN); ONE composed-HEAD
verification; rows flipped DONE after. Full recipe: ORCHESTRATOR wave
mode, "The build-spec spine workflow". The certified Phase-2 wave is
the first full instantiation; the CG program's next build spec reuses
it.
"""

start, end = 11, 126
assert 'wave close' in lines[start] or 'session 49' in lines[start], f"start drift: {lines[start]!r}"
assert '### Session 47' in lines[end + 1], f"end drift: {lines[end + 1]!r}"

out = lines[:start] + [NEW] + lines[end + 1:]
P.write_text(''.join(out), encoding='utf-8', newline='\n')
print(f"spliced lines {start + 1}..{end + 1}: wave-close volatile section written")
