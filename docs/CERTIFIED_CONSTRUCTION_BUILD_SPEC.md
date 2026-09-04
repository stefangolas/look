# Certified Construction Theory — Build Spec (CC program)

**Status:** build spec derived from the audit of
`CERTIFIED_LOFT_AND_SHELL_THEORY_SPEC.md` (unified v1: Loft · Offset/Shell ·
Blend) against the codebase, session 50 substrate. This is the loop-side
packet plan: write sets, dependency graph, gates. The theory content stays in
the theory doc; this document states only what exists, what is missing, and
what gets built in what order.

**Kernel-write rule:** all production writes below are in `vendor/truck/**`
through the packet / worker / `verify.py` loop (`loop/ORCHESTRATOR.md`).
This document books the contract; packets are authored into `loop/packets/`
and registered in `loop/PACKETS.jsonl` at dispatch time.

**Cross-packet contracts are frozen in `CERTIFIED_CONSTRUCTION_CONTRACTS.md`**
(the spine: crate placement, manifest edges, refusal vocabulary, seam
signatures S1–S12, fixture kit, Phase-A write-set matrix). Where that document
contradicts this one, it wins; the placements it amended are marked below.

---

## 1. Audit: theory element → substrate status

Legend: **LANDED** (exists, tested), **PARTIAL** (substrate exists, spec
obligation unmet), **ABSENT** (nothing in tree).

| Theory element | Status | Evidence |
|---|---|---|
| P1 fast path — interval banded no-pivot GE | **ABSENT** | No banded solver anywhere. Nearest: plain f64 `gaussian_elimination` (`truck-geometry/src/nurbs/mod.rs`), non-certified; must not be reused for the certified path. |
| P1 fallback — Rump/Ogita/Oishi residual certificate | **ABSENT** (analog exists) | Residual-based Krawczyk with contraction ceiling exists (`truck-certified/src/kernel/engine.rs`, `kernel/residual.rs`); no ‖I−RA‖ verification-norm module. |
| P1 exact path — rational banded LU | **ABSENT** | Exact signs/dets via Shewchuk `Expansion` (`truck-certified/src/formal/exact.rs`); no rational solve (no `num-rational`/`rug` in any manifest). |
| P2 — injectivity radius δ=2σ/L | **PARTIAL** | σ side exists: `rank_margin` (`certified_map.rs`), `immersion_lower_bound` (`truck-evidence/src/enclosure.rs`). L side: `EnclosureSurface::enclose_der` gives second derivatives; no ‖D²S‖-sup operator, no δ operator. |
| P3 — graph-disk embedding certificate | **ABSENT** (parts exist) | Planar boundary simplicity: `formal/intersection.rs`, `formal/xmonotone.rs`. Projection search (normative candidate set): ABSENT. Seam clause: ABSENT. Whole-certificate: ABSENT. |
| P4 — Krawczyk existence/uniqueness | **LANDED** (n ≤ 4) | `truck-evidence/src/num/krawczyk.rs` (generic const-N), `formal/bezier_isect.rs` (2-D quadtree), `truck-certified/src/ssi.rs` (3×3), `kernel/engine.rs` (n=2/3/4). **5×5 ABSENT** (needed §7.3 only). |
| P4 — argmin-with-margin operator | **ABSENT** | No interval argmin anywhere. Separation bounds exist piecemeal (`fid/lfs.rs`, `fid/isotopy.rs`). |
| P5 — ball clearance | **PARTIAL** | `ImplicitField` interval signed/implicit field over {plane, cylinder, cone, sphere, torus} (`contact/implicit.rs`); BVH candidate-pairs only (`truck-base/src/bvh.rs`); **no distance queries, no general-face SDF, no Clear predicate**. |
| P6 — share by identity | **PARTIAL** | Types landed: `EntityId`/`Op`/`OpKind` (construction-DAG identity, transform-stable derivation, `entity_id.rs`), `EdgeSampleLedger` + facet grid registry (exact shared indices, no welding). **Propagation through Booleans/transforms/blends ABSENT** — `FaceProvenance` is STEP-import-only, booleans carry their own `StratumRef`, transforms emit identity-less `Placed` faces. |
| §2 Loft construction (de Boor averaging, collocation, strips) | **ABSENT** | No knot averaging, no Schoenberg–Whitney check, no tensor-product interpolation, no loft/skinning. `try_interpole` takes a caller-supplied knot vector and plain f64 solve. |
| L1r — weight-field positivity | **PARTIAL** | Machinery exists: `hull_bernstein_2d` (`truck-certified/src/hull.rs`), `EnvelopeCase::NonPositiveNurbsWeight`. No certification wired to a loft. |
| L4 — cyclic correspondence disambiguation | **ABSENT** | Depends on P4 argmin. `ProfileLaw::try_linear_correspondence` covers sweeps only. |
| L5 — regularity + self-contact | **PARTIAL** | Regularity: `rank_margin`. Self-contact: R6 deflation landed (`kernel/selfint.rs`), contact funnel + `cover_branch` exist; no loft-level postcondition composition. |
| Gordon | **ABSENT** | Nothing in tree. |
| §3 contact complex (k=1,2 strata, submersion margins) | **PARTIAL** | `SquareSystem3`, `f3_diagonal_derivatives`, surjectivity-style margins (`select_continuation_coordinate`), analytic pair dispatch, `cover_branch`. **k=3 (three-support) constrained system ABSENT.** |
| §4 offset strata + S1/S1′ | **PARTIAL** | `Offset` decorator enclosures landed (incl. `OffsetDegenerate`/`OffsetSwallowtail` refusals); global S8 booked not landed. Stratum complex construction, star certificates, S1/S1′: ABSENT. |
| §5 blends | **ABSENT** | `OpKind::Fillet` exists in the identity vocabulary only. No rolling-ball system, no event machinery, no setback patches. |
| §6 canal regularity (closed-form criterion) | **ABSENT** (cheap to add) | `EnclosureCurve` derivative enclosures exist; the criterion needs only interval `r′, r″, ‖c″‖` evaluation. |
| §7 thickness — H/K enclosures | **ABSENT** | Curvature-*radius* lower bounds exist (`fid/lfs.rs`); no mean/Gaussian curvature enclosures. |
| §7 thickness — d_min via BVH | **ABSENT** | BVH has no distance query. |
| §4.3/§7 broad phase | **PARTIAL** | `Bvh::candidate_pairs` exists; per-stratum reach ρ_A bound ABSENT. |
| Refusal taxonomy | **PARTIAL** | See §6 below — roughly half map onto existing variants. |

## 2. Gap register

- **G1.** Certified banded solve (P1): interval no-pivot GE for banded TP
  matrices; Rump residual fallback; optional exact rational path under a
  threshold `n_exact`.
- **G2.** P2 injectivity-radius operator over `EnclosureSurface`/`EnclosureCurve`.
- **G3.** P3 graph-disk embedding certificate with normative projection search.
- **G4.** P4 argmin-with-margin operator over certified enclosures.
- **G5.** P5 clearance predicate: BVH distance queries + general-face signed
  distance + fillet/round ball-containment tests.
- **G6.** P6 identity propagation policy through Booleans/transforms/blends
  (representation-level; gates L3 and shared nodes).
- **G7.** Loft: stationing + de Boor averaging + SW a-priori nonsingularity +
  collocation via G1; strip construction with P6-shared split data (L3); L1r
  weight certification; Gordon as a construction algorithm.
- **G8.** L4 correspondence: wire isomorphism + cyclic-shift argmin.
- **G9.** L5 postcondition: regularity margin + off-diagonal self-contact
  composition over the existing contact funnel / R6 deflation.
- **G10.** Contact complex: k=3 constrained system, stratum table realization,
  submersion-margin certificates.
- **G11.** Rounded offset: stratum construction, stars, broad phase with ρ_A,
  S1/S1′.
- **G12.** Canal regularity certificate (closed form, arc-restricted).
- **G13.** Blend: two-support continuation with event isolation, P5
  admissibility, variable radius (foot-point), face consumption via the
  arrangement engine, setback patches.
- **G14.** Thickness: H/K second-form enclosures, closed-form `t_safe`;
  5×5 event systems deferred (§7).
- **G15.** Refusal vocabulary additions (§6 below).

## 3. Packet plan

Classes per loop convention (`design` / `mechanical` / `mechanical+`).
All write sets additive unless noted. Gates V0–V10 plus program invariants
(§5). Prefix `CC`.

### Phase A — primitives (no feature code)

| Packet | Class | Content (gap) | Write set | Depends |
|---|---|---|---|---|
| `CC-000-CONTRACT` | design | Freeze primitive signatures, refusal additions (G15), `DirectTolerance`-style defaults; book field-level rows into `docs/CERTIFICATE_MAPPING.md` | `truck-certified/src/construct/mod.rs` (new, stub), `contract.rs` delta, mapping rows | — |
| `CC-001-BANDED` | design | P1: interval no-pivot banded GE + back-substitution; growth-factor-1 class contract documented; Rump residual fallback for dense/ribbon systems; rational path behind `n_exact` (G1) | `truck-certified/src/construct/banded.rs`, `construct/residual_solve.rs` | 000 |
| `CC-002-INJECTIVITY` | mechanical | P2: σ from existing rank machinery + ‖D²S‖ sup from `enclose_der`; δ=2σ/L; 1-D curve variant (G2) | `truck-certified/src/construct/injectivity.rs` | 000 |
| `CC-003-ARGMIN` | mechanical | P4 argmin-with-margin over `[λ_i]` enclosures; refuse on overlap (G4) | `truck-certified/src/construct/argmin.rs` | 000 |
| `CC-004-CLEAR` | design | P5: BVH minimum-distance query (`truck-base/src/bvh.rs` additive), general-face signed-distance field over strata, `Clear` with round/fillet variants (G5) | `truck-base/src/bvh.rs` (additive), `truck-evidence/src/clear.rs` | 000 |
| `CC-005-GRAPHDISK` | design | P3 certificate: projection search (normative candidate order), seam clause, planar boundary simplicity via existing modules; fallback refusal `NoAdmissibleProjection` (G3) | `truck-certified/src/construct/graphdisk.rs` | 002 |

Phase A exit gate: each primitive has a refusal-path test and a property test
against brute force on small systems (see §5).

### Phase B — loft (certified linear construction)

| Packet | Class | Content (gap) | Write set | Depends |
|---|---|---|---|---|
| `CC-010-LOFT-CORE` | design | Stationing (deterministic chord-length default, uniform option), de Boor averaging, SW a-priori nonsingularity (L0 as compile-time theorem + P1 enclosure), collocation through CC-001 (G7). *Contracts §5 errata: lives in truck-certified (C1).* | `truck-certified/src/construct/loft.rs` (new) | 001 |
| `CC-011-LOFT-WEIGHTS` | mechanical | L1r: control-net bound fast path, `hull_bernstein_2d` subdivision fallback, budgeted refinement, `NonPositiveWeightField` (G7) | same module | 010 |
| `CC-012-LOFT-STRIPS` | design | Closed-wire lofts as r strips; shared split-vertex data by `EntityId` (P6); L3 bitwise seam agreement test; shared factorization across strips (G7) | same module + `entity_id.rs` consumers | 010, 011 |
| `CC-013-CORRESPONDENCE` | design | L4: oriented cyclic complex, isomorphism check, anchor → unique → argmin-over-shifts → refuse (G8). *Contracts §5 errata: lives in truck-certified (C1).* | `truck-certified/src/construct/correspondence.rs` | 003, 012 |
| `CC-014-LOFT-VALIDITY` | design | L5: regularity margin + near-diagonal P2 + far-pair contact funnel + P3 where projection exists (G9) | `truck-certified/src/construct/loft_validity.rs` | 002, 004, 005, 012 |
| `CC-015-GORDON` | mechanical | Boolean-sum construction over compatible bases; reuses cached collocation LU; certified by CC-014. *Contracts §5 errata: lives in truck-certified (C1).* | `truck-certified/src/construct/gordon.rs` | 010 |

### Phase C — offset / shell (contact complex, rounded completion)

| Packet | Class | Content (gap) | Write set | Depends |
|---|---|---|---|---|
| `CC-020-CONTACT-K3` | design | k=3 constrained system (three-support), submersion margins, stratum-table dimension checks; builds on n=4 Krawczyk (G10). *Contracts §5 errata: lives in truck-certified (C1; F1 makes the evidence placement unable to reach the blend consumers).* | `truck-certified/src/construct/contact3.rs` | 000 |
| `CC-021-OFFSET-STRATA` | design | Rounded stratum construction k=1 (focal margin J_t), k=2 (canal via CC-025), k=3 (spherical patch, P4-isolated centre); per-stratum reach ρ_A (G11) | `truck-certified/src/construct/offset_strata.rs` | 020, 025 |
| `CC-022-STARS` | design | Closed-star P3 certification across glued strata; broad phase over constructed strata with ρ pruning (G11) | same module | 005, 021 |
| `CC-023-SHELL-BRIDGE` | design | S1 injectivity-on-quotient (three discharge regimes) + S1′ solid corollary; typed output distinguishing certified surface from certified solid (G11) | `truck-certified/src/construct/shell.rs` | 022, 004 |
| `CC-024-OFFSET-EXACT` | mechanical | Sharp/concave completion via the arrangement engine (extends `arrange()` / trim-clip line); reach bounds ρ_A for non-ball strata (open obligation §10.2 of theory) | `truck-geometry/src/arrange.rs` + shapeops hooks | 021 |
| `CC-025-CANAL` | mechanical | §6 closed-form regularity, arc-restricted variant, `CanalSingular`; consumes `EnclosureCurve` derivative enclosures (G12) | `truck-certified/src/construct/canal.rs` | 000 |
| `CC-026-THICKNESS` | mechanical | Second-form H/K enclosures; closed-form `t_safe = min(t_focal, d_min/2)`; conservative `max_shell_thickness` v1 (G14; 5×5 systems deferred) | `truck-certified/src/construct/thickness.rs` | 004, 021 |

### Phase D — blends

| Packet | Class | Content (gap) | Write set | Depends |
|---|---|---|---|---|
| `CC-030-BLEND-SPINE` | design | Two-support constrained continuation (reuse `ssi_trace` machinery) with event isolation: Σ constant between certified events; P5 admissibility on branch boundaries (G13) | `truck-certified/src/construct/blend/` (new) | 020, 004, 003 |
| `CC-031-BLEND-VARRADIUS` | design | Foot-point system (λ, R(λ)); admissible laws v1: constant, linear, cubic Hermite, monotone cubic, vertex control radii (G13) | same module | 030 |
| `CC-032-FACE-CONSUMPTION` | design | Face arrangement over contact pcurves; `F_i^new = F_i \ R_i`; disappearing faces; same engine reused by CC-024 concave trims (G13) | `truck-geometry/src/arrange.rs` extension | 030, 024 |
| `CC-033-SETBACK` | design | n-valent setback corners: Hermite ribbons via P1 fallback, four-count certification (boundary, G¹, regularity, P3) (G13) | `truck-certified/src/construct/blend/setback.rs` | 005, 030 |

**Elastic pool** (dispatch on idle slots, lowest review-judgment): fixtures —
plane/cylinder/cone/sphere/torus prisms for shell-A; two-circle four-arc
correspondence ambiguity; four-arc closed-wire loft seam bitwise test;
constant-radius pipe with known singular spine; corpus counts (PN/PH admission
rates, provenance-resolvable seam rates) carried over from the theory doc's
open items.

## 4. Dependency graph

```text
CC-000 (serial; everything types against it)
   ├─→ 001 ─────────────→ 010 ─→ 011 ─┬→ 012 ─→ 013 ─┐
   ├─→ 002 ─→ 005                     │              ├→ 014
   ├─→ 003 ──────┬────────────────────┘→ 015         │
   ├─→ 004       └───────────────────────────────────┤
   └─→ 025 ─→ 020 ─→ 021 ─→ 022 ─→ 023               │
                 │    └─→ 024 ─→ 032                  │
                 └─→ 026                              │
                      030 ─→ 031                      │
                        └───┴─────────────────────────┘ (014 after 012; 030 after 020/004/003)
```

Concurrency cap ≤3 live packets over the write-set-disjoint set, per the CG
program's recalibrated velocity doctrine. Phase A packets 001–004 are mutually
disjoint; 005 depends on 002. Phase B is effectively serial through 012.
CC-024/032 touch shared arrangement files — schedule 032 after 024 lands.

## 5. Gates and program invariants

Standard V0–V10 apply. Program-specific done-when invariants:

- **P1 class contract:** no-pivot path refuses (not pivots) on any interval
  pivot containing 0 for matrices outside the banded-TP class; a deliberately
  ill-conditioned non-TP banded fixture produces a typed refusal, never a
  wide-but-accepted enclosure. Rump fallback proven with the ‖I−RA‖<1 test on
  its own fixture.
- **P2 separation:** on a subdivided patch, all parameter pairs within δ are
  excluded from the contact candidate list *by construction*; a fixture with a
  known near-diagonal self-contact just outside δ is still caught.
- **P3:** a folded corner-patch fixture (constructed fold) is refused; a
  genuine glued star passes; projection search order is fixed and
  exhaustion → `NoAdmissibleProjection` is tested.
- **P4 argmin:** overlapping enclosures refuse; strictly separated enclosures
  select; no tie-breaking by value comparison anywhere.
- **P5:** fillet vs round ball-containment orientations both tested; clearance
  margin μ explicit in every call.
- **L3 bitwise:** two independently invoked strip constructions over the same
  `EntityId`-referenced split data produce byte-identical boundary control
  rows (asserted as bytes, not floats-with-tolerance).
- **Determinism:** identical ordered input → identical refusals, enclosures,
  and argmin outcomes across runs; float reductions in fixed order; no
  hash-iteration-dependent output (carried from CG §7).
- **Typed refusals:** every §9 refusal of the theory is reachable in a test;
  no refusal is emitted as a panic or a silent approximation.
- **Precision retry:** where a refusal is width-limited, at least one
  higher-precision retry attempt is demonstrable before refusal (theory §9).
- **Existing entry points bit-identical:** `arrange()`, `triangulation`,
  `boolean()` behaviors unchanged by CC-024/032 (V5 identity guard doctrine).

## 6. Refusal vocabulary integration

Map the theory §9 taxonomy onto existing enums where possible; additions are
booked by `CC-000` into `docs/CERTIFICATE_MAPPING.md`.

| Theory refusal | Disposition |
|---|---|
| `NonPositiveWeightField` | New variant; near-neighbor `EnvelopeCase::NonPositiveNurbsWeight` stays for import-time checks. |
| `SingularInterpolationSystem` | New variant in the construct module (P1). `truck-geometry::Error::GaussianEliminationFailure` remains the non-certified path's error and must not be reused. |
| `AmbiguousCorrespondence` | New (CC-013). |
| `FocalDegeneracy` | New; coexists with kernel `RefusalKind::OffsetDegenerate` / `OffsetSwallowtail` (decorator-level regularity), which keep their meanings. |
| `CanalSingular` | New (CC-025). |
| `RankDeficientContact` | Maps onto `contract.rs::Refusal::ConditioningBelowThreshold` — reuse, do not duplicate. |
| `UnintendedContact` | New; P5-backed, distinct from `UnresolvedWitness::UncertifiedContainment` (which stays the undecided case). |
| `StarNotEmbedded` | New (P2/P3-backed). |
| `NoAdmissibleProjection` | New (CC-005); triggers the pairwise-SSI fallback. |
| `NonGenericThicknessEvent` | New (CC-026 / deferred §7.3). |
| `AmbiguousEventOrdering` | New; the P4 argmin refusal in blend/trace contexts. |

## 7. Deferred and out of scope

- 5×5 Krawczyk and exact `valid_shell_interval` (theory §7.2–7.3) — behind
  CC-026; `t_safe` conservative bound is the v1.
- Sharp/concave completion beyond the CC-024 interface (theory §10.2).
- P6 propagation *implementation* through Booleans/transforms (G6) — the
  `EntityId` algebra is landed; wiring it into `truck-shapeops` operations is
  its own program and is booked, not scheduled here. CC-012 depends only on
  P6 *within* one construction (loft strips), which is in scope.
- General sweeps (theory §10.5) — the CG constructive program owns
  spine/frame sweeps; no overlap.
- Network radius optimization (theory §5.3 out-of-scope clause).

## 8. Definition of completion

- Phase A: all six primitives exist as certified, refusal-typed operators with
  property tests; `CERTIFICATE_MAPPING.md` carries their rows.
- Phase B: a closed four-arc two-section loft constructs via CC-001 with L0
  a-priori nonsingularity, passes L1r, produces bitwise-identical seams (L3),
  resolves the four-fold cyclic ambiguity via argmin (L4), and passes L5 on a
  corpus fixture; Gordon composes without new certificates.
- Phase C: a PN/PH-class convex body shells at arbitrary fixed t with S1 +
  S1′ certificates; `t_safe` measured against the corpus; canal criterion
  rejects a known singular-spine fixture.
- Phase D: a constant-radius chain over three faces traces with event
  isolation, consumes the intermediate face via the arrangement, and closes at
  a certified triple node; a 3-edge setback corner passes the four counts.
- No result in any phase depends on heuristic search, fairness objectives,
  tolerance fits, or unverified Newton iterations — auditable by the gate list
  in §5.
