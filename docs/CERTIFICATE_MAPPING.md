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
- CG-007-CERT may be written against row sets A + B once CG-004 lands; the
  certified program's Phase 0 may dispatch against sections A + C (and must
  still respect the X2 sequencing rule: not concurrent with CG-005/CG-007
  inside truck-meshalgo's module tree).
