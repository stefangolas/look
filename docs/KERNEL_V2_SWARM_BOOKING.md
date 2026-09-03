# Kernel v2 swarm booking — spine-session agenda (NOT dispatched)

**Authored 2026-09-02, session 49 close.** The normative theory is
[`docs/CONSTRUCTIVE_GEOMETRY_KERNEL_SPEC_V2.md`](CONSTRUCTIVE_GEOMETRY_KERNEL_SPEC_V2.md)
("the v2 spec" below) — self-contained, every predicate/certificate/type is
a contract. This booking is the build-spec SKELETON for a massively
parallel swarm over it, per the loop's proven wave recipe (ORCHESTRATOR
wave mode, "The build-spec spine workflow"). **It books no packets and
dispatches nothing; the next session's first acts are below.**

## 0. What already exists (measure, don't guess)

The v2 spec overlaps heavily with landed work. The spine session's FIRST
act is a **gap census**: module-by-module (K0–K3, C1–C6, S0–S10) against
the tree, classified exists / partial / missing, each with command
evidence. Known landing zones:

- **C1–C6 ≈ the CG program, DONE**: BG-CG-000..009 landed
  (`constructive/` module, facet realization, edge ledger, manifold
  diagnostics, Coons4, SpineFrameSurface + B-rep constructor). The v2
  deltas on top: Spine enum with PH fast path + FrameData (§5.2/§5.3),
  dyadic sampling join (§4.3), closed/open audit declaration (§5.6),
  the one-variant enum boundary (§5.10 — landed as the enum ripple).
- **K0/K1/K2/K3 precursors exist in truck-certified**: hull kernels,
  exact.rs, the F3 contract, the ssi modules (C1/C2 certificates,
  frames), bezier_isect.rs (the 2D prior art), pair_dispatch.rs.
- **The spec's "Changes from v2" list is the new-work driver** — treat
  each bullet as a candidate packet seed.

## 1. Contract inventory → the shim (frozen BEFORE any wave dispatch)

One shim packet through the NORMAL loop (the BG-CK-P2-CONTRACT pattern:
types + refusing constructors + fixture kit with machine-checked ground
truths, no solver bodies; full verify once; **the shim's landing merge is
the wave base**). Pre-identified contents:

- **K1 evidence algebra**: `ClaimVerdict<T,E,R>`, `Construction`,
  `Refusal { kind, backing, evidence, partial }`, `VerdictClass`
  (Disproven vs Inconclusive distinction is load-bearing, §2 rule 2).
- **K2 traits**: `CertifiedPatch` (+ `CertifiedPatchC2`, `CertifiedPatchC3`
  subtraits, `weight_bound` as type-level precondition per §7.1) and the
  leaf/carrier types (`BezierLeaf`, `RationalCarrier`, `Param`/chart/deck).
- **Certificates**: `Frame<N>`, `ArcCert<N>` (ρ from Lemma 8.0 inside the
  type), `PointCert`, `ContactCert` (tolerance-tagged, §10.3),
  `GraphCert`, `R5Enclosure`, `SheetCert`, `TubeOverlapCert`.
- **Graph types**: `TopoNode` / `SegmentBreak` / `ArcEnd` / `NodeCert`
  (topology vs segmentation split, D5), `AnyArc`, `CertifiedGraph` vs
  `ClaimedGraph` (never unified, D6).
- **Residual family**: `ResidualId` = the closed R1–R9 enum (D8) with the
  §4.2 implication order as a typed relation (Rule C's admissible set).
- **Refusal taxonomy**: the full `RefusalKind` enum (§17) with
  backing class per variant.
- **Config constants** (§0.4): ρ_max, κ_max, depth_max, k_a, deck_max,
  tol.intersection added to the landed `DirectTolerance`.
- **The §22 mapping table**: land as the doc row-source; every later
  packet adds its row before dispatch.

Fixture kit (shim-carried, machine-checked ground truths): transversal
sphere/cylinder/plane patch pairs; a coaxial-cylinder pair (recognized
carrier, closed-form contact locus); a determinant-spans-zero box; a
weight-straddles-zero rational leaf; a deck-wrap cylinder patch pair;
a C¹-discontinuity spine.

## 2. Open owner decisions at spine time

1. **Crate placement**: extend `truck-certified`/`truck-geometry` vs a
   new kernel crate (affects every write set; decide first).
2. **Reuse-vs-retype policy** for landed CG code under the V5 identity
   guard (existing tessellation entry points bit-identical — §5.7).
3. **Census resurrection**: the spline-bucket census WIP is archived
   (`loop/slots/1/abandoned-20260902-142536.patch`, session resumable);
   it quantifies representation-recovery mass and re-scores the Phase-2
   funnel. Cheap; decide whether it runs parallel to the shim.
4. **N4 two-architecture CI gate**: which second architecture (this is a
   CI/machine decision, not a packet).
5. **Wave sizing**: the spec's own §19 cap says ≤3 live packets; the
   proven session-49 pattern ran 3 in parallel with scoped checks and
   could go wider — owner sets it.
6. **Elastic pool disposition**: the two §20 batteries
   (transversal / designed-tangency) are separate measurement corpora by
   law — book them as measurement packets, never one aggregate.

## 3. Wave plan (proposal to refine at spine time)

Map §19's 25 build-order rows onto waves along its dependency graph:

- **Wave 0**: the shim (normal loop).
- **Wave 1** (parallel): K0 numerics audit + K1 + K2 traits/leaves +
  K3 identity (contracts; largely census-driven fill-in).
- **Wave 2** (parallel): C1 deltas (Spine/PH/FrameData), S4a float
  tracer, S2a (Lemma 8.0 + C1/C2 over the landed interval core),
  S1a (R8/R9), C4/C5 deltas.
- **Wave 3** (parallel): S0/S3a (maximal-minor + Tier-1), S5a
  (ContactCert), S9a (node identity + deck identification), S3b
  (Tier-2 start set), S2b (GraphCert/R5 contract).
- **Wave 4** (parallel): S3c trim clip, S7 (R7/canal), S6 (ExactSheet),
  S8 (R6 charts), K2b (atlas/carriers).
- **Wave 5** (serial, integrator-owned): C6 enum boundary + S9b
  promotion + S10 verification.
- Elastic pool throughout: corpus fixtures, both batteries, mutation
  batteries, microbenchmarks.

**The frozen-contract rule applies at every seam, including
amendment-time ones**: any function a later wave calls must have its
signature pinned in the producing packet AND the consuming packet's text
before either dispatches (session-49 lesson — do not give the producer
"exact spelling yours" while sequencing a consumer behind it).

## 4. Verification plan

- Per-packet done-whens = §20's acceptance rows, scoped to the crate
  (worker LOCAL gates), per wave mode.
- **One composed-HEAD verification battery per wave** (the loop's
  ordinary gates; V8/V9 skip-justifications recorded if additive-only).
- The **cross-cutting audits** (§20) become kernel-gates.sh additions —
  grep-class, cheap, high-signal: no transcendental call on certificate
  paths; R2 never reaches C2; no `dist < eps` identity; no
  `par.sum()`; no TopoNode named Refuse/ChartSwitch/FrameSwitch/DeckStep;
  sampling join integer-only; ClaimedGraph never reaches a
  CertifiedGraph consumer; Canal has no orthogonality certificate field.
- N4 bit-reproducibility: per-module enclosure fixtures run on two
  architectures (decision 4 above).

## 5. Machine facts (inherited, binding)

- `CARGO_BUILD_JOBS=2–4` for every cargo invocation (two cold warm-build
  OOMs at default jobs on the 15.7 GB machine); one worker at a time for
  COLD warm builds; parallel workers with warm targets are proven fine.
- `RUSTC_WRAPPER=sccache` for workers; unset it locally if sccache
  rejects the incremental dev profile (recorded by W3).
- Reclaim idle slot targets before any verify (disk floor ~15 GB);
  watch for `%TEMP%` proc-macro-srv leaks after reboots.
- One authoritative verification per wave at composed HEAD; amendments
  return to owning sessions via `--resume`; LOCAL_GREEN is never DONE.

## 6. Build sharing policy for the v2 swarm (owner direction, session 49)

Parallel agents do NOT each build the world:

1. **Worktrees stay per-agent** (branch-per-packet is structural; the
   agent's uncommitted WIP lives there) — but a worktree implies
   NOTHING about builds.
2. **Workers run check-only local gates** (`cargo check -p <crate>`);
   test suites never run in a worker — they run ONCE at the composed
   HEAD. A worker packet scoped so it needs `cargo test` locally is
   scoped wrong.
3. **Shared target directory**: workers on the same base point
   `CARGO_TARGET_DIR` at ONE shared location (cargo's file locking
   serializes concurrent invocations — acceptable for check-only
   steps); duplicate per-worker target trees are eliminated. sccache
   deduplicates the compile work itself.
4. The orchestrator prewarms the shared target once at the wave base;
   workers never warm their own.
5. Per-agent RAM is the only remaining per-worker cost; the JOBS cap
   still applies to every invocation.
