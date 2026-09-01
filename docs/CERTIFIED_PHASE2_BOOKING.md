# Certified-kernel Phase 2 booking — class 2 generic (certified SSI branch tracing)

**Authority.** `CERTIFIED-KERNEL-PLAN.md` §2 class 2 and §3 Phase 2; the
unified mapping is `docs/CERTIFICATE_MAPPING.md` section C row 3 (branch
output is a `Certified<…>` result type in `truck-certified`, never
annotated onto shell evidence). F3 (continuation coordinates) is FROZEN
landed code: `vendor/truck/truck-certified/src/contract.rs`
(`SquareSystemInput`, `ContinuationCoordinate`, `CoordinateSwitch`) —
Phase 2 implements against it and never relitigates it.

**Written 2026-09-01 (session 47)** — before Phase 1's floor published, on
purpose: everything booked here is stable regardless of the floor numbers.
The floor and the corpus gate subset are INPUT GATES at the bottom.

## Substrate census (verified by command, session 47 — do not re-derive)

- **2D bivariate Krawczyk, fully landed**:
  `formal/bezier_isect.rs` (2045 lines) — quadtree isolation
  (`MAX_DEPTH` 50, `NODE_BUDGET` 200k, `ParamBox`),
  `KrawczykCertificate` ("K(X) ⊆ int(X) over directed rounding — only a
  valid inclusion may emit a root"), typed non-results
  (`GenericUnresolved`), canonical-operand identity discipline, and the
  denominator-cleared `System` (fs/ft/gs/gt derivative patches). The 2×2
  machinery is the template Phase 2 generalizes; its Krawczyk inner loop,
  its identity rules, and its fail-closed typing are PRIOR ART to copy,
  not re-derive.
- **F3 frozen types (refusing stubs)**: `contract.rs`
  `SquareSystemInput` (a square 3×3 system's Jacobian minors + margins),
  `ContinuationCoordinate` (index + certified relative margin,
  `Method::Interval`), `CoordinateSwitch` (BOTH certificates, no
  default). The selection rule is frozen: largest relative margin,
  lowest index on ties, `ConditioningBelowThreshold` refuses, never a
  weaker retry.
- **Exact arithmetic**: `formal/exact.rs` (`Expansion`,
  `CertifiedInterval`, `exact_sq_dist`/`exact_dot2`/`cross_exp`).
- **Hull kernels (Phase 1)**: `hull.rs` — enclosure of Bernstein patches
  over subboxes, derivative patches; the enclosure oracle the system
  constructor composes.
- **B-spline → Bézier**: landed `bezier_decomposition` (curves); the
  row/column tensor cut landed in Phase 1's `certified_map.rs` (D-map,
  tensor commutation verified against `subs`).
- **Branch carrier**: `formal/contact.rs` `BranchIncidence` (span +
  certified parameter enclosure + branch germ + deck label) and
  `BranchGerm` (regular/stationary/cusp-candidate/singular/unresolved,
  `span.rs`) — the per-box branch records a trace emits already have a
  landed shape.

**The gap (all of Phase 2's authored work):** the 3×3 square system
constructor (surface-surface difference, 3 equations from 4 parameters),
the Krawczyk generalization 2×2 → 3×3 (Jacobian inverse under directed
rounding — the 2D certificate's inner loop, dimension-raised), the
continuation/branch-tracing loop itself (the 2D isolator finds ISOLATED
roots; Phase 2 traces CURVES), turning-point switching as a certified
event (both-certificate rule), and the branch output records per mapping
row 3.

## The corpus Phase 2 owns (prevalence, 38 files)

The residual the exact arms do not admit: spline~spline 21,004;
cylinder~spline 15,566; plane~spline 16,137; cone~spline 3,808;
spline~torus 3,923 — plus the special-position rejects from
DISPATCH-2 (cone~cone 2,050; cone~cylinder 3,918; cone~torus 2,400;
cylinder~torus 6,436; torus~torus 1,861; cylinder~sphere 3,249). The
spline-side rows (~60k pairs) are the reason Phase 2 exists and the
budget's center of gravity: every one needs TWO Bézier decompositions
and a traced branch.

## Packet graph

```text
BG-CK-P2-SYSTEM ──> BG-CK-P2-KRAWCZYK3 ──> BG-CK-P2-TRACE ──> BG-CK-P2-RESIDUAL
                                     (F3 switching lands in TRACE)
```

Serial dispatch (pagefile rule), one worker at a time; each packet lands
before the next dispatches.

### BG-CK-P2-SYSTEM — the square-system constructor (design; orchestrator)

`truck-certified/src/ssi.rs` (new): from two certified-admitted Bézier
patches, construct the surface-surface difference system — 3 equations
(x, y, z components of S1(u,v) − S2(s,t)) in 4 unknowns — plus its
Jacobian minors as Bernstein patches (hull kernels for enclosures). The
F3 square reduction: per-box selection of the continuation coordinate via
the frozen `select_continuation_coordinate` rule (implemented here
against `SquareSystemInput`). Refusals: the frozen F3 vocabulary
(`ConditioningBelowThreshold`) plus hull-mapped
`EnclosureUnavailable`/`DomainNotCompact`. Class pairs outside
spline-admissible shapes refuse `UnsupportedPairClass` (the DISPATCH
widening). Write set: `src/ssi.rs`, lib.rs line, tests.

### BG-CK-P2-KRAWCZYK3 — the 3×3 unique-root certificate (design)

`src/ssi.rs` continuation (same module): the 2×2 Krawczyk inner loop
dimension-raised — Jacobian inverse via the adjugate/determinant over
`CertifiedInterval` (the determinant's enclosure strictly away from zero
is the certificate's precondition, mirroring the 2D code's own
structure), K(X) enclosure over directed rounding, `K(X) ⊆ int(X)`
inclusion test. Only a valid inclusion emits a root candidate. The 2D
module's typed-unresolved discipline carries over verbatim. Write set:
`src/ssi.rs`, tests.

### BG-CK-P2-TRACE — certified branch tracing + switching (design)

The continuation loop: seed from an isolated Krawczyk root, step boxes
along the branch, per box the frozen coordinate selection, and at turning
points the `CoordinateSwitch` event with BOTH certificates (the frozen
contract: a heuristic reseed without both is a contract violation — the
implementation must refuse, never reseed). Branch records per box:
`BranchIncidence`-shaped (mapping row 3), germ classification via
`BranchGerm` (a zero first jet reads the next nonzero jet — the
span.rs discipline). Output: `Certified<Branch>`-shaped result type
booked by mapping row 3; no spline emission (F1). Write set: `src/ssi.rs`
or `src/ssi_trace.rs`, lib.rs line if split, tests.

### BG-CK-P2-RESIDUAL — the Phase-2 gate measurement (mechanical; FLOOR shape)

`tests/certified_phase2_floor.rs` + `docs/CERTIFIED_PHASE2_FLOOR.md`: on
the corpus subset seeded with grazing freeform pairs (the spline~spline
and spline~X rows), ≥80% certify is the plan's floor; refusal
distribution by cause (`NonTransverse` / `Conditioning` / `Singular` —
the plan's own named causes) published. Fail-closed is not passable by
refusing everything: the doc must show the certify rate AND the admitted
mass (the FLOOR anomaly-column discipline carries over). No threshold
assertions in-tree.

## Pre-made decisions

1. **One module family, one crate**: everything lands in
   `truck-certified/src/ssi*.rs`; truck-geometry stays certified-free
   (D1); no new workspace edges.
2. **The 2D Krawczyk code is prior art, not a dependency**: its
   certificate structure, identity rules, and fail-closed typing are
   copied in shape; its private helpers are NOT widened (the HULL
   precedent — solver internals stay solver-private).
3. **F3 is law**: the coordinate-selection rule, the both-certificate
   switching, and the no-weaker-retry discipline are implemented exactly
   as frozen; a needed widening is an orchestrator spec edit to
   contract.rs's docs first, never a worker decision.
4. **Fail-closed with published mass**: every refusal is typed and
   counted; the RESIDUAL doc publishes certify-rate AND admitted mass,
   so "refuse everything" cannot masquerade as the gate passing.
5. **H-1**: crate-level deny covers new modules; no unwraps anywhere.
6. **Dispatch order is serial**; SYSTEM → KRAWCZYK3 → TRACE may collapse
   into fewer packets if the orchestrator judges the write sets too
   entangled to verify separately — the collapse decision is made AFTER
   SYSTEM's probe, not now (the booking names the seams; it does not
   force them).

## Input gates (before the first Phase-2 dispatch)

1. **Phase-1 floor published** (`docs/CERTIFIED_PHASE1_FLOOR.md`): the
   certify-rate and the anomaly column decide whether the exact-arm
   approach holds and whether DISPATCH-2 (cone/torus) precedes Phase 2.
2. **Corpus gate subset**: the "grazing freeform pairs" seeds must be
   named (which files carry the spline~spline mass) — a census test or a
   published table, same discipline as the prevalence census.
3. **DISPATCH-2 decision**: the special-position arms may take pairs out
   of Phase 2's residual; booking them first shrinks Phase 2's budget.

Nothing in Phases 3–6 is pre-buildable beyond what is already read:
class 4 (Select) consumes Phase 2's branch output and the landed
ManifoldDiagnostics (mapping row: class-4 manifold consumption); classes
3/5/6/7/10 type against Phase 2-3 outputs that do not exist. Reading the
plan's sections again at each phase boundary is the pre-build.
