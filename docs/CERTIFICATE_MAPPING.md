# Unified certificate mapping — CG program × certified-kernel program

**Authority.** This is the single certificate-field mapping for both the
constructive geometry program (`docs/CONSTRUCTIVE_GEOMETRY_PLAN.md` §3.5) and
the certified-kernel program (`CERTIFIED-KERNEL-PLAN.md` Phase 0, item X1).
The certified plan's Phase-0 gate requires "unified mapping table published";
the CG plan requires "CG-007 cannot be dispatched against an unfrozen
mapping". Both point here. **New evidence kinds are booked by adding a row to
this table (orchestrator/spec edit) — never by a worker widening an evidence
type on its own judgement.**

Verified against the tree 2026-08-31 (integration/kernel-bg `003f3a7`).

## A. Frozen CG-000 rows (loop side)

Frozen at BG-CG-000-CONTRACT; the frozen snapshot lives as the module doc at
`vendor/truck/truck-geometry/src/constructive/mod.rs` (kernel code — changes
only through a packet). This table carries the same rows; CG-007 implements
them.

**Placement correction (2026-08-31, session 45, pre-CG-007).** The three
CG-007 types below are booked into **`truck-base/src/evidence.rs`**, not
truck-meshalgo: the facet outcome that must carry `shared_edge_pairs` and the
realization certificate lives in truck-modeling, a regular modeling→meshalgo
edge would drag the tessellation crate into modeling's dependency tree against
plan §3.1, and the BG-S0-001 precedent already moved the evidence algebra to
truck-base for exactly this reason (modeling and meshalgo both depend on
base; zero new manifest edges). The meshalgo side of CG-007 is the
*assembly/integration* module (building the evidence from a realization
outcome, the ledger, and the mesh), not the type home. The frozen CG-000
module doc in `constructive/mod.rs` predates this correction; this table is
the authority.

| Evidence kind | Carrier | Where the variant lands |
|---|---|---|
| Recipe construct refusals — every `ConstructError` variant (spine/frame validity, profile collapse, correspondence mismatch) | `Refusal::UnsupportedEnvelope(EnvelopeCase::ConstructRefused)` at the realization entry; the detailed `ConstructError` rides the realization evidence record as a structured summary (base cannot name `ConstructError` — geometry depends on base, not vice versa) | NEW unit variant `EnvelopeCase::ConstructRefused` in `truck-base/src/evidence.rs`; NEW `RealizationEvidence` + `ConstructErrorSummary { kind, at, law }` in `truck-base/src/evidence.rs` (placement correction above) |
| Jacobian bounds (frame conditioning during realization) | per-face, positionally aligned with `shell.faces` exactly as `MeshedShellOutcome::face_failures` is | NEW `RealizationCertificate` struct in `truck-base/src/evidence.rs` (placement correction above) + NEW field on the CG-004 realization outcome (CG-007 fills it); deliberately NOT a widening of `FaceValidityCertificate` — different vocabulary, the same separation doctrine as `band_attempts` vs `cone_band_attempts` |
| Shared-edge pair errors (`EdgeID` + FaceID A + FaceID B + error_a + error_b) | NEW field `shared_edge_pairs: Vec<SharedEdgePairEvidence>` on the realization outcome | NEW `SharedEdgePairEvidence` struct in `truck-base/src/evidence.rs` (placement correction above); never a `ProvenanceRecord` variant (that type is `Copy + Eq`; the payload carries f64s) |
| Winding audit (twin-triangle) | a three-valued verdict carried beside the emitted `PolygonMesh` | NEW `RealizationVerdict { CertifiedWithinTolerance, Failed, Inconclusive }` in `truck-base/src/evidence.rs` (placement correction above); winding-audit failure is `FAILED`, never a warning; uncertainty is `INCONCLUSIVE`, never converted into success |
| Any other realization-stage per-face evidence | the existing `MeshedShellOutcome` positional-vector doctrine | new vocabulary = a new `Vec<Option<...>>` field aligned with `shell.faces`; never a widening of an existing vector |

Standing notes carried from CG-000: construct-stage failures predate meshing,
so they never enter `MeshedShellOutcome` (there is no shell to annotate).
Every value computed in floats certifies `Method::Float` (H-6), never
`Method::Exact`. Verdicts are three-valued throughout.

## B. CG-004 delta (booked as dispatched, r2 `003f3a7`, pending landing)

The CG-004-FACET packet (in flight) books its own stage-local types:

- `FacetVerdict { CertifiedWithinTolerance, Failed, Inconclusive }` — the
  three-valued verdict of the plan §3.3 sanity audit, carried on the facet
  outcome beside `mesh: PolygonMesh` and the audit facts; derived by
  `verdict_of(&audit, extent)`.
- The packet explicitly does NOT add `EnvelopeCase::ConstructRefused` or
  `RealizationVerdict` — those remain CG-007's additions (row set A).

**Relation, so the two types do not read as rival vocabularies:** one tri-state
doctrine (§9.7), two stage-local types. `FacetVerdict` is the facet backend's
immediate verdict over its own audit; `RealizationVerdict` (CG-007) is the
evidence-stage aggregate that consumes facet-stage facts (winding audit,
shared-edge pairs) plus the meshalgo meshing evidence. CG-007 must map/absorb
`FacetVerdict` into its aggregate — it must not introduce a third verdict type.

## C. Certified Phase-0 bookings (certified side)

The certified program books its entries into this same table — one mapping,
not two widenings of the same evidence types (certified plan X1).

| Certified evidence kind | Carrier | Booking |
|---|---|---|
| Certified refusals (every certified constructor, D4) | the existing `Refusal` enum, shaped per `formal/outcome.rs` | NO new top-level `Refusal` variants. The existing `UnresolvedWitness` vocabulary already covers the class-2 failure shapes (`RootNotIsolated`, `KrawczykIndeterminate`, `DeviationUncertified` — verified at `truck-base/src/evidence.rs`). New `EnvelopeCase` variants beyond row A's `ConstructRefused` must be booked here first. Failure witnesses specific to the certified layer live in `truck-certified` and ride inside the existing witness payloads. |
| Witness-edge evidence (Phase-0 freeze F1) | the certified `Edge` itself in `truck-certified` (pcurve pair + both surface handles + enclosures) | NOT booked into `MeshedShellOutcome` or `FaceValidityCertificate` — the witness is the identity claim "there was never a second edge" and stays attached to the edge. When a certified edge is consumed by realization/meshing, only its *derived* facts (with their `Method` tags) enter row-set A carriers. |
| Class-2 branch output (certified SSI branch tracing) | `Certified<…>`-shaped result type in `truck-certified` | Branch geometry is carried as a result, not annotated onto shell evidence. Failures along the way are `Refusal`s per the first row of this section. Spline emission happens at export/meshing only (F1); a spline never becomes the evidence carrier. |
| Enclosure / interval bounds (D2 hull + directed rounding; F2 per-quantity choice of interval composition vs auxiliary root isolation) | `Method::Interval` certificates | These bounds feed row A's `RealizationCertificate` (Jacobian/frame conditioning) as one of two producers. Composition rule under H-6: an aggregate certificate's method is the weakest of its inputs — a float estimate (`Method::Float`) composed with an interval bound (`Method::Interval`) aggregates as `Method::Float`. |
| Class-4 manifold consumption | landed `ManifoldDiagnostics` + `orientation_parity` (`truck-topology/src/manifold.rs`, CG-006) | Consumed substrate — the certified class-4 stage reads the aggregate; it never re-emits a parallel diagnostics type. |
| Certified outcome layers (`formal/{outcome,evidence}.rs`, four layers; promoted to `truck-certified` in Phase 0) | the certified layer's own outcome shape | Maps onto the tri-state doctrine (§9.7): certified layers → `CERTIFIED_WITHIN_TOLERANCE`; failed layers → `FAILED`; unresolved layers → `INCONCLUSIVE`. `INCONCLUSIVE` is never converted into success in either program. |
| `ImplicitReduction` Method tag (CFP-000-SPINE decision 7; produced by CFP-004's implicit-reduction stage) | certificate-provenance record on CFP certificates produced by the Theorem-4 implicit arm (`h = g∘S`, planes + quadrics; torus excluded) | NO new top-level `Refusal`/`UnresolvedWitness`/`EnvelopeCase`/`Prop` kind, and NO widening of the base `Method` set `{Exact, Interval, Float, None}` at the spine. The tag is booked here as the provenance name the producing packet (CFP-004) stamps on its certificates; if CFP-004 needs it on the base `Method` itself, that widening is an orchestrator spec edit to this row first — never a worker widening of `truck-base/src/evidence.rs`. |
| `ConeCertificate` Method tag (CFP-000-SPINE decision 7; produced by CFP-006's cone-certificate stage) | certificate-provenance record on CFP certificates carrying the Gauss-map verdicts (`ConeVerdict`: loop-free / transversal-by-proof / fixed continuation axis) | Same booking discipline as the `ImplicitReduction` row: no new top-level evidence kind, no base `Method` widening at the spine. The `ConeVerdict` shape itself ships in `truck-certified/src/cfp/spine.rs` behind the pending refusal (production paths land with CFP-006). |

## D. BIE-000 bookings (Certified Interaction Engine shim)

The Certified Interaction Engine program (`docs/BIE_BUILD_SPINE.md`,
`docs/CERTIFIED_INTERACTION_ENGINE_BUILD_SPEC.md`) books its contract rows into
this same table. The shim packet BIE-000-CONTRACT lands the restricted-pair
outcome vocabulary, records the §8.1 carrier decision, and ships a unit-shape
fixture kit whose ground truths later BIE wave tests are graded against. All
rows here were dispatched against the tree 2026-09-05 (evidence.rs anchors
`pub enum Refusal` at 1, `NumericallyUnresolved` at 2).

| BIE evidence kind | Carrier | Booking |
|---|---|---|
| Restricted-pair unresolved verdict `InteractionOutcome::Unresolved { kappa, cell, slope }` | `Refusal::NumericallyUnresolved { spent: Budget::new(0, 0, 0), witness: UnresolvedWitness::KrawczykIndeterminate }` | NO new top-level `Refusal` variants and NO new `UnresolvedWitness` variants. The κ / cell / slope witness is first-class on the engine's own `InteractionOutcome::Unresolved` arm (it stays in the engine vocabulary); the landed projection records the refusal class only, for routing through machinery that consumes the landed taxonomy. The witness is `KrawczykIndeterminate` because the restricted-pair solver (BIE-002) raises an unresolved verdict exactly when its slicewise Krawczyk operator proves neither existence nor absence on a box — the closest landed arm of the same epistemic shape. The `Refused` arm is a real landed `Refusal`, passed through unchanged (`From<Refusal>`); a `Certified` answer carries no landed refusal. Both `NumericallyUnresolved` sites (`truck-base/src/evidence.rs`, enum arm and the `Budget::spend_*` doc) anchor the shape. |
| Restricted-pair certified value (a certified scalar/point answer, e.g. a section circle's centre/radius) | `InteractionOutcome::Certified(CertificateValue)` where `CertificateValue` carries an explicit `Method` tag | `CertificateValue` is a BIE value type in `truck-certified/src/construct/bie/mod.rs`: a certified scalar or point with the producing `Method` (H-6 — float-derived closed forms are tagged `Method::Float`, never `Exact`). There is deliberately no `From<f64>`/`From<Point3>`: a `Certified` answer is never fabricated from raw floats without an explicit `Method`. |
| Restricted-pair parameter cell witness | `WitnessCell` (`(u, v) × (s, t)` four-interval box) in `construct/bie/mod.rs` | The `Unresolved` verdict's `cell` field. The cell label convention is per-side and not semantically load-bearing; it is the product-domain box the solver bisects (the shape BIE-001's `IntervalBox4` refines). |
| §8.1 procedural interaction carrier — **carrier decision (pre-decided, recorded)** | `CertifiedImplicitIntersectionCurve`: a NEW canonical `Curve` variant in `truck-geometry/src/canonical.rs`, landed by BIE-003 (NOT BIE-000) | Carries a certified 3-D polyline with per-sample tangent frames plus the unresolved witness slot. Mirrors the landed `Curve::IntersectionCurve` boxed-variant pattern (canonical.rs); PL-at-tessellation only (`EdgeSampleLedger`-compatible; truck-meshalgo read-only). BIE-000 records this decision; the tree evidence that the record is sound is the landed `IntersectionCurve` canonical variant, which the additive variant ripple copies. |
| Unit-shape fixture kit ground truths | BIE fixture records in `construct/bie/fixtures.rs` (`#[doc(hidden)] pub`, TEST SUPPORT ONLY) | plane × sphere (section circle: centre = perpendicular foot `c − δ·n`, radius `sqrt(R² − δ²)`, `δ = (c − o)·n`); plane × cylinder (section ellipse: semi-axes `r` and `r/|sin θ|`, `θ` the incidence vs the axis); sweep × plane (straight-spine `Scale`-of-a-circle sweep unit shape: section is the ring at the station `s*` selected by the plane equation, circle of radius `radius(s*)` about `C(s*)`). Ground truths are closed-form constants, tagged `Method::Float`, machine-checked in-module under `// H-3` discipline; no solver is called to build or check a fixture. Determinism: the whole kit builds from ordered dyadic data — two constructions compare equal. |

## E. CTE-000 bookings (Certified Tangency and Exact Contact spine)

The CTE program (`docs/CTE_BUILD_SPINE.md`,
`docs/CERTIFIED_TANGENCY_BUILD_SPEC.md`) freezes the certificate
vocabulary for singular SSI closure (T1) and coincident-carrier Boolean
classification (T2) before any implementation wave. The shim packet
CTE-000-SPINE lands the tangency shapes and the F1–F7 fixture kit and records
its mapping rows here. All rows were dispatched against the tree 2026-09-05.

| CTE evidence kind | Carrier | Booking |
|---|---|---|
| Tangency shape-constructor refusals (the CTE-000 D-shim) | `contract::Refusal::InvalidInput` | NO new top-level evidence kinds. Every refusing constructor in `tangency/shapes.rs` (a construction outside a frozen rule) returns `Refusal::InvalidInput`, the `ssi_types.rs` P0-freeze precedent. Numeric-evaluation requests in the shim refuse the same way; the named CTE refusal cases the wave packets raise (`no-chart`, `graph-failure`, …) wrap the landed vocabularies (`SsiRefusal`, `TraceRefusal`, kernel `Refusal`, `HullRefusal`) in the owning packet, never as new top-level arms. |
| Five-way `ContactVerdict` (theory §2.11) | certified verdict on the engine vocabulary; `Unresolved { kappa, cell }` projects onto `Refusal::NumericallyUnresolved { spent: Budget::new(0,0,0), witness: UnresolvedWitness::KrawczykIndeterminate }` | NO new `Refusal`/`UnresolvedWitness` variants. `Transversal`/`Empty`/`A1Isolated`/`A1Node`/`A2Branch` are certified answers carrying their own certificate data (a `Certified` answer never maps to a landed refusal). The `Unresolved { kappa, cell }` residual maps onto the closest landed epistemic shape — the BIE-000 section-D precedent — for routing through machinery that consumes the landed taxonomy; `kappa`/`cell` stay first-class on the CTE verdict itself. `A1Node` (A₁⁻) is first-class from day one (theory R3); it is never routed to `Unresolved`. |
| `A2BranchCurve` producing stub (pending until CTE-005) | pending refusal name `a2_branch_packet_pending`, surfaced through the owning packet's refusal vocabulary | NO new top-level evidence kind. The shape ships behind the pending refusal, the `cone_torus_carrier_packet_pending` precedent (`kernel/rational.rs`): at the shape layer the stub refuses `Refusal::InvalidInput`; the named cause is carried by the producing packet (CTE-005) and asserted by CTE-008's gates. |
| F1–F7 fixture kit (`tangency/fixtures.rs`) | TEST SUPPORT ONLY — `#[doc(hidden)] pub`, excluded from the certified API surface | A one-line mapping-table note, not a row (the `ssi_fixtures.rs` precedent): no new evidence kind. Ground truths are exact-integer records machine-checked at admission; the F7 rows copy the landed `boolean_m2` fixture *values* read-only. |

## F. FSSI-000 bookings (Fibered SSI topology contract)

The FSSI program (`docs/FSSI_BUILD_SPEC.md`) freezes its refusal names,
verdict carriers, and evidence mapping here BEFORE any solver code (packet 1
of 5). All rows dispatched against the tree 2026-09-08. Exactly three rows:
the fold tier carrier (this packet lands the type), the per-segment
projection-index booking (the type lands later, in FSSI-003), and the tube
certificate (consumed landed substrate).

| FSSI evidence kind | Carrier | Booking |
|---|---|---|
| Ordinary-fold event (theory §5; FSSI-002): a box the landed path refuses (`Conditioning` / `TraceRefusal::Conditioning`) that one `KrawczykSystem<4>` proof over `E_j = (F, q_j)` with `det D(F, q_j) ≻ 0` certifies as a unique regular ordinary fold | `FoldCert { chart: usize, sigma: i8, det_enclosure: (f64, f64) }` — a NEW refusing-constructor carrier (D-shim: type + refusing constructor only, nothing numeric) in `truck-certified/src/ssi.rs`, registered here as a NEW escalation-lattice tier alongside the CFP-008 stagnation verdicts (which stand unchanged) | FSSI-000 lands the carrier; FSSI-002 populates it through the existing certificate tuple carrying the producing proof. A genuinely degenerate fold stays a landed refusal (`DegenerateFold`-class), never a `FoldCert`. |
| Per-segment projection index (theory §8; FSSI-003): a bridge segment (`Switched`) must never be re-expressed as a `z_j`-graph | the arc type extension FSSI-003 freezes on the tracer's output arc/vertex types (its `ssi_trace.rs`/`ssi_types.rs` write set) | Booking row only — the type does NOT exist yet. FSSI-003 freezes the index field against the already-landed per-step `chart_box` discipline; FSSI-000 records the mapping before the type is written. |
| Tube certificate (theory FSSI-6, the uniform parametric Krawczyk tube) | the LANDED parallelotope proof type — `truck_evidence::num::parallelotope` (`StepVerdict` / `ParallelotopeFrame`, the generic `KrawczykSystem<N>` + `KrawczykProof` operator) | Consumed substrate, no new type and no `truck-evidence` edit. The tube's per-step certified boxes are `StepVerdict::Certified` records; `Margin`/`Certificate` evidence rides them verbatim. |

## Standing rules (both programs)

1. **H-6 method rule.** `Method ∈ {Exact, Interval, Float, None}`
   (`truck-base/src/evidence.rs`). A value computed in floats is never
   recorded `Exact`. Interval-computed values are `Interval`, not `Exact`.
2. **One tri-state doctrine.** `CERTIFIED_WITHIN_TOLERANCE | FAILED |
   INCONCLUSIVE`; uncertainty surfaces as `INCONCLUSIVE`, never silently as
   success (build-spec §9.7).
3. **No parallel validation universe.** New evidence composes with
   `MeshedShellOutcome` / `FaceValidityCertificate` / provenance vocabulary;
   new *types* only where a row above books them.
4. **Stage separation.** Construct-stage failures ride the realization entry's
   `Refusal` + `RealizationEvidence`; realization-stage per-face evidence rides
   positionally-aligned vectors; mesh-stage verdicts ride the emitted
   `PolygonMesh`. A stage's evidence never leaks into another stage's carrier.
5. **Additions to this table are spec edits** made by the orchestrator before
   the packet that needs them is written — a worker that needs an unbooked
   variant stops and files the gap instead of inventing a carrier.

## Status

- Unified table published 2026-08-31 (session 45). Resolves certified plan X1
  and loop plan §3.5's booking requirement.
- CTE-000-SPINE rows (section E) dispatched 2026-09-05; the tangency shim and
  fixture kit are test-support-only and add no evidence kind.
- CFP-000-SPINE rows (section C, `ImplicitReduction` / `ConeCertificate`
  provenance tags) dispatched 2026-09-06; the CFP shim and fixture kit
  (`truck-certified/src/cfp/{spine,fixtures}.rs`) are the D-shim spine of
  `docs/CONTACT_FAST_PATH_BUILD_SPEC.md` and add no top-level evidence kind.
- CG-007-CERT may be written against row sets A + B once CG-004 lands; the
  certified program's Phase 0 may dispatch against sections A + C (and must
  still respect the X2 sequencing rule: not concurrent with CG-005/CG-007
  inside truck-meshalgo's module tree).
