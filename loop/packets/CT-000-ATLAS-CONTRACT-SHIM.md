# WORK PACKET CT-000-ATLAS-CONTRACT-SHIM — the Contact Atlas program spine (contracts only, no solver bodies)

Program spine for `docs/CONTACT_ATLAS_BUILD_SPEC.md`; normative theory is
`docs/CONTACT_ATLAS_SPEC.md`. This packet creates the `atlas` module with
the frozen contracts every later packet builds against, plus the shared
ground-truth fixture kit. **No solver behavior changes**: the existing
certified boolean funnel is untouched; the module is additive and inert
until integration packets wire it in.

```yaml
id:          CT-000-ATLAS-CONTRACT-SHIM
contract:    [CT-000-ATLAS-CONTRACT-SHIM]
class:       design
crates:      [truck123d]
depends_on:  [FHC-G1-RATIONAL-FLUX]
needs:       [FHC-G1-RATIONAL-FLUX]
write_allow:
  - truck123d/src/lib.rs
  - truck123d/src/atlas/mod.rs
  - truck123d/src/atlas/contract.rs
  - truck123d/src/atlas/scheduler.rs
  - truck123d/src/atlas/refusal.rs
  - truck123d/tests/atlas_fixtures.rs
read_allow:
  - docs/CONTACT_ATLAS_SPEC.md
  - docs/CONTACT_ATLAS_BUILD_SPEC.md
  - docs/SLOW_SOLVER_DIAGNOSTICS_R4.md
  - docs/REFUSALS.md
tests_required: [truck123d/tests/atlas_fixtures.rs]
anchors:
  - {id: A1, min: 10, cmd: "grep -cE '^(pub )?mod ' truck123d/src/lib.rs"}
  - {id: A2, min: 40, cmd: "grep -cE '\\bVolumeRow\\b' truck123d/src/bd_bridge.rs"}
  - {id: A3, min: 5,  cmd: "grep -cE '\\bclassify_point\\b' truck123d/src/bd_bridge.rs"}
budget:      {turns: 45, ctx_tokens: 140000}
```

Anchors measured 2026-09-14 at HEAD `e271ba9`. **Re-measure at dispatch.**

## The frozen contracts (exact; later packets implement against THESE)

1. **Residual object model** (`contract.rs`): a residual carries
   `phi: [f64; 2]` (outward-rounded interval), `canonical_id: u64`
   (deterministic identity), and a refine operation returning finitely
   many children each enclosing the same semantic contribution (Theorem
   25 soundness contract). Two kinds: `TraceSpanResidual`,
   `FallbackCellResidual` — as data-carrying enums, not traits-with-dyn
   (determinism and serializability of the bookkeeping).
2. **Scheduler** (`scheduler.rs`): deterministic best-first over
   descending `width`, tie-break ascending `canonical_id`; finite budget
   accounting; gate trait with three implementations: absolute `W <=
   tau`, mixed `W <= eps_abs + eps_rel*max(V_lo,0)` (Theorem 5), decision
   predicates `V_lo > X` / `V_hi <= X` (Theorem 5A). Empty queue or
   exhausted budget => typed refusal carrying the current bracket.
3. **Co-support oracle contract** (`contract.rs`):
   `NoPositiveAreaOverlap | Resolved(shared decomposition, orientations)
   | UnknownPositiveAreaOverlap` — exactly one answer per candidate pair;
   `Unknown` maps to refusal `CoincidentSupportUncertified`. The oracle
   IMPLEMENTATION is Wave 3; only the contract type exists here.
4. **ConstructionIdentity** (`contract.rs`): canonical serialization of a
   pre-placement construction subtree + certificate-precision tag;
   content-hash addressed; the cache-key contract for CT-203 (Theorem
   8C). Rigid placements MUST NOT change the identity.
5. **Refusal vocabulary** (`refusal.rs`): the spec §26 codes not already
   in the landed marshal — `CoincidentSupportUncertified`,
   `TransversalityUncertified` (re-export/alias of the landed code),
   `UnsupportedPatchRepresentation`, `SeparationCertificateUnavailable`,
   `TraceTopologyBudgetExceeded`, `TraceChartUncertified`,
   `TraceAdjacencyUncertified`, `TraceRegularityInsufficient`,
   `ArrangementUncertified`, `ResolvedFluxUncertified`,
   `RefinementBudgetExceeded`, `DepthLimitExceeded`. Marshaling into the
   landed typed-refusal surface is integration work (CT-100+), not here.
6. **Fixture kit** (`tests/atlas_fixtures.rs`): analytic ground-truth
   compositions with machine-checked exact volumes — abutting boxes
   (shared planar face), nested boxes, a known cylinder-overlap pair, a
   nested group tree. Every later wave's tests consume THESE; exactness
   of expected volumes is asserted against analytic formulas.

## Pre-made judgements

1. **New module, additive only**: `truck123d/src/atlas/` registered by ONE
   line in `lib.rs`. The one-line lib.rs registration is the DESIGNED
   textual conflict with no other packet (verified: G15/G16/G17
   write_allow do not include lib.rs).
2. **No dyn-dispatch in bookkeeping paths**: enums + matches; determinism
   (spec §25) outranks extensibility.
3. **Every interval type is outward-rounded by construction**: helper
   constructors take `(lo, hi)` and assert `lo <= hi` in debug; the
   `hull`/`add`/`scale` helpers round outward. No fast-math anywhere.
4. **The scheduler is total**: finite budgets, typed refusal with the
   current bracket (Theorem 26).

## Done-when

1. `cargo check -p truck123d --tests --locked` green.
2. `cargo test -p truck123d --test atlas_fixtures --locked` green: the
   fixture ground truths hold; the scheduler sorts deterministically and
   respects the three gates on synthetic residuals; the refusal
   vocabulary round-trips through its constructors.
3. `cargo fmt -p truck123d -- --check` clean on touched files; `cargo
   clippy -p truck123d --lib --locked` clean on the diff.
4. The existing battery untouched: `cargo test -p truck123d --test
   fuse_fold --locked` green (the funnel is inert to this packet).
5. Write `RESULT.json` AT THE WORKTREE ROOT (nowhere else), including the
   anchor re-measurement and the exact public signatures chosen for the
   five contracts (they are the program's frozen interface — deviations
   require a spine amendment, not a worker decision).
