# BRep gap register — static audit, 2026-09-08

Gaps found by source inspection this session, cross-referenced against the
committed audits (`docs/audits/BREP_SOLVER_AUDIT.md` findings S1–S10,
`docs/audits/BREP_TOPOLOGY_AUDIT.md`, `docs/audits/BREP_BENCHMARK_AUDIT.md`).
Gap types: **REP** representation, **ADM** admission, **CERT** certification
/proof, **CMP** completion, **TOP** topology, **PERF** performance
opportunity, **MEAS** evidence/measurement gap. Fields per entry:
transition, source evidence, gap type, correctness impact, possible
performance impact, prerequisites, landed reusable substrate, missing
mathematical machinery, invasiveness, recommended priority.

Priorities: P0 = soundness/closure claim needs correction first; P1 = closure
or accounting gap worth booking; P2 = opportunity.

---

## AD-1 — Rational weights refused at SSI admission over a landed engine substrate
- **Transition:** `Map/Generate → Interact` (spline face admission into SSI).
- **Source evidence:** `patch_admit.rs:97,151-198` (`UNIT_WEIGHT_REL_TOL=1e-7`,
  `RationalWeights` refusal at 196); engine side `ssi.rs:1356-1410` accepts
  any two `RationalBipatch`es with positive weight grids; the positive-weight
  certificate machinery exists (`formal/bezier.rs` `WeightMayVanish`,
  `construct/loft_weights.rs::certify_weight_field` → `WeightCert`).
- **Type:** ADM (representation closure holds; admission closure fails).
- **Correctness impact:** none today — the refusal is typed and honest; the
  gap is coverage, not soundness. (Caveat: BREP_SOLVER_AUDIT S5 (P0) — the
  narrow admission can *normalize away real weights* in one path; fix S5
  before widening.)
- **Performance impact:** none measured; widening moves work from
  `NumericallyUnresolved` refusals into the certified engine.
- **Prerequisites:** S5 correction; per-side weight certificates composed
  into `SideCarrier`; single-span restriction (AD-2) addressed for real
  utility.
- **Landed substrate:** per-knot-span Bézier decomposition, Bernstein hulls,
  F:R⁴→R³ cross-multiplied form, Krawczyk3, trace/stitch.
- **Missing machinery:** weight-field positivity certificate composed at
  admission (span-level, not fixture-level); multi-span assembly.
- **Invasiveness:** small–medium.
- **Priority:** P1 (highest-value admission widening in the tree).

## AD-2 — Single-span / single-patch restriction at the dispatch seam
- **Transition:** spline×analytic SSI dispatch.
- **Source evidence:** `ssi_admit.rs:148` (`WindowNotOneSpan`); cross-patch
  assembly booked as CL-002; `stitch_fragments` exists (`ssi_trace.rs:859-1022`).
- **Type:** ADM.
- **Correctness impact:** none (typed refusal). **Performance:** real NURBS
  faces are multi-span; without assembly the entry is unusable for them.
- **Prerequisites:** none beyond wiring. **Substrate:** stitch machinery,
  patch stacks. **Missing:** patch-boundary event handling between spans
  feeding the trace.
- **Invasiveness:** medium. **Priority:** P1.

## AD-3 — SsiAdmitSolver registration is test-only; no production SSI entry
- **Transition:** public op → contact funnel → certified SSI.
- **Source evidence:** `ssi_admit.rs:629` (registration API), only call site
  `#[cfg(test)]` at 790; unregistered ⇒ `NumericallyUnresolved`
  (`contact/mod.rs:795-800`); BREP_SOLVER_AUDIT S9.
- **Type:** ADM/wiring. **Correctness:** none. **Performance:** N/A
  (feature unreachable). **Prerequisites:** decide the production entry that
  should register (funnel stage 6.5). **Substrate:** all of it.
- **Invasiveness:** tiny. **Priority:** P1 (unlocks AD-1/AD-2 value).

## AD-4 — Two parallel certified analytic-pair dispatchers, drifting
- **Source evidence:** `truck-certified/src/pair_dispatch.rs::dispatch_pair:255`
  (5 arms; ellipse refuses `UnsupportedPairClass:579-582`; only test
  callers) vs production `truck-evidence/contact/mod.rs::analytic_ff:1019`
  (8 families, ellipse emitted).
- **Type:** ADM + CERT (drift risk between admitted configurations and
  refusal vocabularies).
- **Correctness impact:** latent (a configuration certified in one arm can
  be refused in the other; conformance guarantees diverge).
- **Performance impact:** potential repeated discovery (P1 in the static
  performance model).
- **Prerequisites:** decision whether `pair_dispatch.rs` is the future
  certified core (its exact-`Expansion` doctrine is stricter) with
  `analytic_ff` as the funnel adapter.
- **Invasiveness:** small (conformance harness exists:
  `tests/pair_dispatch_conformance.rs`). **Priority:** P2.

## AD-5 — `RevolutedCurve` unrecognized; no pair dispatch for extruded/revolved decorators
- **Source evidence:** `recognize.rs:155-160` (explicit `Unrecognized`);
  `canonical.rs:315-318` (`ExtrudedCurve` RESERVED); enclosures landed
  (`decorators/extruded.rs` with the C′×V=0 singular rule, `revolved.rs`),
  but no `match` on these decorators in `pair_dispatch.rs`,
  `contact/mod.rs`, or `ssi*.rs`.
- **Type:** ADM (+ PERF, see TO-3).
- **Correctness impact:** typed refusals only. **Performance:** structural
  opportunity (section/meridian reductions) — unmeasured.
- **Prerequisites:** carrier recognition into `CanonicalSurface` or a
  decorator-keyed dispatch arm. **Substrate:** enclosures, `Processor`
  recognition, analytic families.
- **Missing machinery:** extrusion/revolution pair reduction theorems.
- **Invasiveness:** medium. **Priority:** P2.

## MP-1 — Map(transform, object) transports no evidence
- **Source evidence:** no `impl Transformed`/`transform_by` in
  `truck-certified`; `canonical.rs:374-434` exact-placement doctrine;
  topology/builder/cad transforms transport geometry only; instances are
  structure-only (`truck-assembly`, facade `EvidenceRowKind::Intent`).
- **Type:** CERT (O7).
- **Correctness impact:** none (nothing claims transport); blocks evidence
  reuse for instances/placements (P6).
- **Performance impact:** unmeasured; repeated derivation is the cost of
  honesty today.
- **Prerequisites:** rigidity/orientation rules per transform class (rigid =
  exact transport of certificates; reflection = orientation flip; uniform
  scale = enclosure scaling; nonuniform = new metric bounds per
  `BREP_SYSTEM.md` §7.3).
- **Substrate:** `Processor` homogeneous interval transform
  (`decorators/processor.rs`), `Margin`/`Modulus` algebra in truck-base.
- **Missing machinery:** transport theorems + a scoped-cache discipline.
- **Invasiveness:** medium. **Priority:** P1 (foundational for instances).

## WR-1 — Kernel wave has no production consumer; SSI trace callers test-only
- **Source evidence:** grep facts in `BREP_TRANSITION_AUDIT.md` §0;
  `ssi_trace.rs` callers only `tests/ssi_trace.rs`; `CertifiedGraph` has no
  consumer; `route_stagnation` (`ssi.rs:1564`) only called by
  `cascade.rs`'s test.
- **Type:** wiring/CMP.
- **Correctness impact:** none. **Performance:** N/A.
- **Prerequisites:** the registered-entry decision (AD-3) and a consumer for
  `PromotedEdge`/`CertifiedGraph` (note `truck-topology` constructors panic
  on circle self-loops — `kernel/promote.rs:19-22` documents the deliberate
  barrier).
- **Invasiveness:** medium. **Priority:** P1.

## CT-1 — Fold certification is shape-only (FSSI-002 unpopulated)
- **Source evidence:** `ssi.rs:201-228` `FoldCert` refusing constructor;
  FSSI-002 booked, not landed.
- **Type:** CERT. **Correctness:** tangency near-folds stay
  `Inconclusive`/unresolved (honest). **Performance:** N/A.
- **Missing machinery:** fold chart construction + σ sign certification.
- **Invasiveness:** medium–large. **Priority:** P2.

## CT-2 — A2 (rank-3) certification is test-gated; cascade stage 6 unreachable
- **Source evidence:** `a2.rs:1566` `certify_a2`; `cascade.rs` doc 31-33
  ("stage 6 … not reachable from this signature"); `gates.rs` doc-hidden.
- **Type:** wiring. **Correctness:** none. **Performance:** N/A.
- **Prerequisites:** a classify route that admits stage 6 (A2) — i.e. a
  caller contract for `certify_a2` with R6 non-emptiness satisfied.
- **Invasiveness:** small–medium. **Priority:** P1 (the machinery is the
  expensive part and it exists).

## CT-3 — T2 coincidence arrangement scope-limited to axis-aligned strata
- **Source evidence:** `arrange.rs` doc 21-24; `atom_decision:689`; §6.3
  open tail.
- **Type:** CERT (O4/O5 for coincident strata).
- **Correctness impact:** coincident/overlap classes refuse or stay
  unresolved in general curvature. **Performance:** N/A.
- **Missing machinery:** general curved-cell arrangement.
- **Invasiveness:** large. **Priority:** P2.

## CT-4 — Exact witness producer stub
- **Source evidence:** `witness.rs` `provenance_witness` =
  `WITNESS_PRODUCER_PACKET_PENDING`; verifier total and landed.
- **Type:** CERT. **Invasiveness:** small. **Priority:** P2.

## CM-1 — No completion envelope anywhere (budgets, not theorems)
- **Source evidence:** `DEPTH_MAX`/`DECK_MAX`/`Budget {subdiv,newton,depth}`
  ceilings (`truck-base/src/evidence.rs:306-382`); `FormalEnvelope`/
  `ExecutionBudget` declared-but-unconstructible (`formal/outcome.rs`,
  `envelope.rs`); no in-tree theorem bounds subdivision, root isolation, or
  predicate decision; `tier1.rs` loop-free certificate is the only
  closed-form completeness statement (over its own hypotheses).
- **Type:** CMP.
- **Correctness impact:** none (three-valued outcomes make exhaustion
  honest); this is the semantic-vs-completion closure distinction of
  `BREP_SYSTEM.md` §4.
- **Performance impact:** unbounded worst-case work before honest refusal
  (measured once: Phase-2 floor, 279 s debug for 6 dispositions).
- **Missing machinery:** proved envelopes per transition (e.g. quantified
  exclusion-convergence bounds; subdivision completeness for transversal
  pairs under a conditioning floor).
- **Invasiveness:** large. **Priority:** P1 as a *documented* envelope per
  production (do not attempt a global theorem).

## TP-1 — Collapsed boundaries / loft apex have no dedicated typed outcome
- **Source evidence:** `loft.rs:172-176,242-245,296` (flat stations ⇒
  `InvalidInput`); degenerate regions ⇒ `ConditioningBelowThreshold` inside
  `LoftValidityCert` (`loft_validity.rs:230-245`); `trimclip.rs` has no
  collapse case (`trim_loop_open_cannot_bound:778`); cone apex routes
  `CarrierSingular` (`atlas.rs:117`); sphere pole collapse landed
  (`domain/lattice.rs::CollapseWitness::ExactSpherePole`).
- **Type:** REP + CERT (O2).
- **Correctness impact:** honest refusals; apex boundaries are representable
  (degenerate charts exist) but not constructed through loft.
- **Substrate:** rational normal numerators (`bezier_isect.rs::
  numerator_polys`), deflation (`kernel/mod.rs:125`), normal cones.
- **Missing machinery:** apex/collapse-typed loft outcomes; collapsed-loop
  trim semantics.
- **Invasiveness:** medium. **Priority:** P2.

## TP-2 — Cone/torus chart enclosures pending
- **Source evidence:** `kernel/atlas.rs:90,94` (`CONE_FORM_PENDING`,
  `TORUS_FORM_PENDING`); chart families constructed, enclosures certify
  nothing yet; `ConeChartRefusal`/`TorusChartRefusal`.
- **Type:** CERT (O2). **Substrate:** `leaf.rs` `RationalCarrierKind`
  includes Cone/Torus; `implicit.rs` interval implicit machinery covers the
  torus (excluded from the *reduction* only).
- **Invasiveness:** medium. **Priority:** P2.

## TL-1 — Output kinds `Empty/Body/BodySet/OpenShell/Assembly/Mesh` SPEC ONLY
- **Source evidence:** `docs/BREP_SYSTEM.md:55`; exhaustive grep — no such
  enum; `ProductShape {Shells, Solid, Matrix}` ingestion-side only
  (`convert.rs:705-712`); `Outcome<Solid>` single-shell gate
  (`assemble.rs:79`); empty = zero-shell `Solid` (117-120); facade handles
  opaque (`truck123d/src/python.rs:25-32`).
- **Type:** REP + TOP.
- **Correctness impact:** empty results and open shells are representable
  but not distinguished at the API; re-entry contracts unclear (S8).
- **Invasiveness:** small (enum + mapping) but touches facade/serialization.
- **Priority:** P1.

## TP-3 — Certified volume / mass properties ABSENT
- **Source evidence:** zero matches in `truck-certified`;
  `truck-meshalgo/src/analyzers/volume.rs` (float, doc-conditioned);
  `facet_sweep.rs::signed_volume_of` (verdict input only).
- **Type:** REP/CERT (kept deliberately separate from construction per the
  audit contract — the absence is a missing *reference-fact* op, not a
  construction defect).
- **Missing machinery:** Green/divergence reduction over trimmed Bernstein
  patches with outward-rounded integration; rational-face handling.
- **Invasiveness:** small–medium (see theory opportunity TO-6).
- **Priority:** P2.

## TP-4 — `planar_holes` production route recovers 0 faces today
- **Source evidence:** module doc records 187/187 exits
  `unsupported_curve_representation` (arc bounds unsupported upstream);
  production call site `triangulation.rs:3140`.
- **Type:** ADM (curves: arcs on face bounds not admitted by the slice
  route). **Correctness:** none (typed exits). **Performance:** recovery
  routes fall to band routes.
- **Substrate:** `planar_developed.rs` exact arc development (test-only),
  `xmonotone.rs`, `CurveSpan2` (`span.rs`).
- **Invasiveness:** medium. **Priority:** P1 (directly widens the production
  STEP recovery rate).

## SR-1 — Canonical seed path uses uncertified ray decisions; ray_cert standalone (S7)
- **Source evidence:** `classify::surface_ray_crossings` plain f64 +
  `NORMAL_SLACK`; `ray_cert.rs` doc 4-9 ("does NOT touch the live classify
  path — DEF-SEEDRAY-B"), 4 carriers only (Plane/Sphere/Cylinder/Cone);
  BREP_SOLVER_AUDIT S7 (P0).
- **Type:** CERT (O5 soundness of fragment classification seeds).
- **Correctness impact:** a P0 *in the audited pipeline* (wrong seed parity
  is guarded by the parity-graph contradiction check but the seed itself is
  uncertified).
- **Missing machinery:** certified ray solves for Torus + sweep carriers;
  DEF-SEEDRAY-B wiring.
- **Invasiveness:** small for wiring; medium for torus/sweep solves.
- **Priority:** P0 (existing recorded first blocker).

## SW-1 — Restricted sweep path defects (S1/S2/S4/S6, existing recorded)
- **Source evidence:** BREP_SOLVER_AUDIT S1 (P0) representation-changing
  circular-section limit (`ssi4.rs:2341`, `circumradius_about_axis:2427`);
  S2 (P0) partial traces lose unresolved remainder; S4 (P1) caller budget
  not bounding restricted work; S6 (P0) unproved planarity/convexity
  assumptions in sweep fragment classification.
- **Type:** CERT (P0s) + CMP (S4).
- **Correctness impact:** soundness claims need correction before widening
  sweep admission.
- **Priority:** P0 (these are existing recorded first blockers for the
  swept-carrier program; PB-011 landed routing around them in r1).

## WS-1 — Boolean re-entry and evidence-preservation contracts (S8/S10)
- **Source evidence:** BREP_SOLVER_AUDIT S8 (empty/disconnected results lack
  re-entry closure), S10 (fragment identity/evidence computed but not
  preserved as a contract).
- **Type:** TOP + CERT. **Priority:** P1.

## ID-1 — Edge-sample ledger is beside-path evidence, not the mesh driver; keying defects
- **Source evidence:** `triangulation_with_ledger.rs:44-68` (runs unchanged
  path, builds ledger beside it);
  `docs/audits/BREP_TOPOLOGY_AUDIT.md` P1 findings (global keying mismatch
  at `triangulation_with_ledger.rs:88`; exposed-edge-sampler contract).
- **Type:** REP/MEAS. **Correctness:** none (evidence only).
- **Performance:** the ledger duplicates sampling work today (P6);
  `BREP_SYSTEM.md` §7.5 names the remedy (shared-edge sampling consumed by
  the emitted index buffer).
- **Invasiveness:** medium. **Priority:** P2 (perf) / P1 (contract fix
  booked as BG-CG-005 follow-ups).

## HG-1 — Homology gate runs beside, not inside, `boolean()`
- **Source evidence:** `gates/homology.rs:12-13` ("no edit to `boolean/*` is
  needed"); typed FAILED gate exists.
- **Type:** CERT wiring. **Invasiveness:** tiny. **Priority:** P2.

## PG-1 — `Declared`/`CertifiedNumerical` authority never minted
- **Source evidence:** `formal/evidence.rs:833-839` (deliberate
  `#[allow(dead_code)]` with rationale); only `Analytic` introduction rules
  landed.
- **Type:** CERT (O7). **Impact:** the scoped-evidence system's strongest
  variants have no producers, so downstream consumers can only demand
  analytic authority today.
- **Missing machinery:** certified-numerical introduction rules wrapping
  the landed certificate carriers (PointCert etc.) into `Evidence`.
- **Invasiveness:** small–medium. **Priority:** P1 (composes the two
  evidence worlds).

## ME-1 — Measurement gaps (summary)
- **Type:** MEAS. Every entry in `BREP_EXISTING_RUNTIME_EVIDENCE.md` §6:
  per-pair SSI cost, tangency, boolean, construction, band routes, TOR-A,
  whole-model kernel timings (B9 door-field defect), kernel-vs-OCC
  differential. **Priority:** P1 for the TTC door field fix (B9) and
  per-stage timing scopes (`BREP_SYSTEM.md` §7), before any optimization
  packet.

---

### First-blockers note

The only *recorded* first refusals (existing evidence, not hypothetical) on
corpus-scale paths are: the Phase-1 certify-rate floor (0.586), the Phase-2
trace budget exhaustion with 0.0 certify-rate, `planar_holes` 0-face
recovery, UR10 CDT blowup (fixed at `09726a9e`), and the TTC kernel DNF
refusals. Everything else in this register is a static finding.
