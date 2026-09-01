# Certified-kernel Phase 1 booking — class 1 (CertifiedMap) + class 2 fast path

**Authority.** `CERTIFIED-KERNEL-PLAN.md` §3 Phase 1 and §2 classes 1–2; this
doc is the loop-side packet graph, the same way
`docs/CONSTRUCTIVE_GEOMETRY_PLAN.md` booked the CG program. Contract freezes
live in `vendor/truck/truck-certified/src/contract.rs` (landed,
BG-CK-P0-FREEZE); the mapping is `docs/CERTIFICATE_MAPPING.md` section C.
**New evidence kinds are booked in that table, never invented by workers.**

**Phase-1 exit gate (plan §3):** the analytic path must *certify* (not
refuse) ≥ 95% of the corpus pairs its dispatch admits, with measured
throughput ≥ the legacy path on that subset. The prevalence table
(`docs/CERTIFIED_PREVALENCE.md`, 62.32% analytic pairs) is the corpus basis.

**Sequencing:** ONE worker at a time (session-46 pagefile rule). All packets
write inside `truck-certified/**` except the floor measurement, so strict
serialization costs nothing in parallelism.

## What the substrate already has (do not re-derive — plan §0, verified)

- Shewchuk `Expansion` + directed rounding: `formal/exact.rs` (the single
  exact-arithmetic implementation in the workspace; look consumes it).
- Checked finite wrappers: `formal/numeric.rs` (`FiniteF64`,
  `NonNegativeFinite`, `PositiveFinite`).
- B-spline → Bézier decomposition, `KnotVec::bezier_knot`, derivative
  containers to order 10: `truck-geometry/src/nurbs`, `truck-base/src/ders.rs`.
- Certified analytic identification: `formal/support.rs` (plane),
  `formal/cylinder.rs`, `formal/cone.rs`, `formal/torus.rs`.
- Certified 2D pair intersection: `formal/intersection.rs`
  (`CertifiedIntersection2`, `PairIntersectionResult`, `IntersectionPolicy`,
  `intersect_x_monotone`), above `formal/span.rs`'s frozen `CurveSpan2`
  contract (the `RationalBezierSpan2` variant is DECLARED; its constructor
  and certified operations are GEN-001B's — that is this phase's work).
- Pair-contact lift: `formal/contact.rs` (`BranchIncidence`), the family-
  independent span contract.

## The packet graph

```text
BG-CK-P1-HULL ──┬──> BG-CK-P1-MAP
BG-CK-P1-SPHERE ┼──> BG-CK-P1-DISPATCH ──> BG-CK-P1-FLOOR
                └──────────────┘
(serial dispatch; the graph edges are semantic, the pagefile rule serializes)
```

### BG-CK-P1-HULL — the D2 primitive as public API (design; orchestrator writes)

`truck-certified/src/hull.rs` (new module): control-point hull bounds of a
Bézier span (curve and surface form) over any rectangular subbox, derivative
patches to order 2, directed rounding at the evaluation leaves through
`formal/exact.rs`. This is D2's "one enclosure primitive" as a typed API —
NOT a general interval evaluator (the parsimony hinge; the frozen F2 table
names the compositions, this module is what they compose). Refusals:
`EnclosureUnavailable` / `DomainNotCompact`, mapped per
`formal/outcome.rs`. Write set: `truck-certified/src/hull.rs`,
`truck-certified/src/lib.rs` (one line), `truck-certified/tests/hull_conformance.rs`.
Consumes `RationalBezierSpan2`/Bézier forms; depends on BG-CK-P0-FREEZE.

### BG-CK-P1-SPHERE — the certified sphere constructor (booked gap; mechanical after design)

The prevalence census found 2.56% of corpus faces are sphere-carried with
representation-named evidence only (no certified constructor exists —
`identify_torus_world` refuses the sphere case; 284 degenerate-torus faces
are the honest-refusal residual). `formal/sphere.rs` (new): `identify_sphere`
in the refusing-constructor discipline of `identify_plane`/`identify_cylinder`
— a `CertifiedEmbeddedSphere` witness with private fields, representation-
derived center/radius, exact predicates for admissibility. Unblocks sphere
PAIRS in the dispatch. Zero behavior change elsewhere.

### BG-CK-P1-MAP — class 1 CertifiedMap (design; orchestrator writes)

`truck-certified/src/certified_map.rs`: admission of a compact rectangular
parameter domain of a B-spline curve/surface decomposed to Bézier spans
(D2), the enclosure oracle over subboxes (hull of f and ∂f, via HULL), and
the rank margin: interval evaluation of Jacobian minors against a declared
τ. Refusals `ParameterizationDegenerate` / `EnclosureUnavailable` /
`DomainNotCompact` per plan §2 class 1. **Admission lives here, not in
truck-geometry** (D1) — geometry types gain no knowledge of certification.
Correspondence-is-input: loft/screw/developable/section-law maps are
CLIENTS. First consumers: the SpineFrameRecipe sweep core (certified
Jacobian evidence for TR-VAL-001). Write set: `truck-certified/src/certified_map.rs`,
lib.rs line, tests.

### BG-CK-P1-DISPATCH — class 2 analytic pair dispatch (design; orchestrator writes)

**AMENDED 2026-09-01 (session 47, orchestrator spec edit — mass-driven
split):** the corpus pair masses decide the split. The exact-certifiable
arms (plane~plane 26,274; cylinder~plane 37,361; plane~sphere 281;
sphere~sphere 126; the coaxial/parallel subset of cylinder~cylinder 5,354)
carry ~62% of analytic mass and land in BG-CK-P1-DISPATCH. The
special-position-only arms (plane~cone 8,379, plane~torus 5,385,
cylinder~sphere 3,249 — ellipse/conic/quartic cuts certifiable only in
named geometric configurations) book as **BG-CK-P1-DISPATCH-2** after
FLOOR's first measurement (velocity-recalibration doctrine; one
rational-conic machinery serves plane~cone and the plane~cylinder general
ellipse cut). `Plane/torus` in the v1 admitted set would have violated the
exact-decision doctrine.

`truck-certified/src/pair_dispatch.rs`: the fast-path dispatcher over
certified support schemas — the exact arms above. Each
arm produces a certified contact through exact-predicate admission
screens (`formal/exact.rs` expansions decide every configuration) with
directed rounding at the value leaves; the landed `intersection.rs` 2D
pipeline is the implementation model and `PairUnsupported` (widened by
one named variant `UnsupportedPairClass`, booked per mapping section C
row 1) is the refuse-class — classes OUTSIDE the admitted set refuse
typed, never swallowed (the no-silent-downgrade doctrine). Zero
mesh-derived intersection polylines in the certified path (F1: witnesses,
never approximations). Chart (pcurve) emission books with Phase 3's
boolean core — no Phase-1 consumer needs it (FLOOR measures
certify/refuse, not pcurves). Write set:
`truck-certified/src/pair_dispatch.rs`,
`truck-certified/src/formal/intersection.rs` (the enum variant),
lib.rs line, tests.

### BG-CK-P1-FLOOR — the Phase-1 gate measurement (mechanical; worker)

Extends the landed `tests/certified_prevalence.rs` harness: for every corpus
pair the dispatch ADMITS, run the certified path and count
certify/refuse/unresolved; publish the certify-rate (floor 95%) and the
throughput comparison vs the legacy path on the certified subset. Publishes
`docs/CERTIFIED_PHASE1_FLOOR.md`. Write set: `tests/certified_phase1_floor.rs`,
`docs/CERTIFIED_PHASE1_FLOOR.md`. No threshold assertions in-tree (the
measurement is the output, same discipline as the prevalence census).

## Pre-made decisions (the packet writer's job, done here)

1. The primitive is hull + directed rounding; no interval crate, no second
   root engine (D2). The F2 frozen table governs composition vs isolation
   per quantity.
2. `truck-geometry` gains no dependency on `truck-certified` (D1); callers
   admit.
3. Sphere evidence: representation-derived like the plane's retained native
   basis — never orthogonalised, never normalised downstream.
4. The dispatch's refuse-class is `PairUnsupported` (mapping section C row 1
   vocabulary); no new top-level `Refusal` variants.
5. Serial dispatch; each packet lands before the next dispatches (pagefile
   rule; the graph's parallelism is not worth the disk risk).
6. H-1: every new module carries `#![deny(clippy::unwrap_used)]` — now
   linted (packet_lint H1_NEW_MODULE).
