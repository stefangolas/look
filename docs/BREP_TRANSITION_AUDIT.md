# BRep transition audit — static architecture, capability, and evidence audit

Read-only static audit, 2026-09-08, repository HEAD. No kernel operations,
benchmarks, corpus rows, instrumentations, or new refusals were produced.
Every implementation claim below is grounded in source inspected in this
session; line anchors refer to this snapshot (workers may be active). Related
committed audits: `docs/audits/BREP_SOLVER_AUDIT.md` (findings S1–S10),
`docs/audits/BREP_TOPOLOGY_AUDIT.md`, `docs/audits/BREP_BENCHMARK_AUDIT.md`,
`docs/BREP_SYSTEM.md` (the organizing model audited against), and
`docs/CERTIFICATE_MAPPING.md` (the frozen evidence mapping).

Companion deliverables: `BREP_EXISTING_RUNTIME_EVIDENCE.md`,
`BREP_GAP_REGISTER.md`, `BREP_THEORY_OPPORTUNITIES.md` (ends with the final
summary matrix).

Verdict vocabulary: `LANDED` / `PARTIAL` / `SPEC ONLY` / `ABSENT` / `UNCLEAR`.
Closure kinds are always qualified: **representation**, **admission**,
**semantic/proof**, **decision/completion**.

---

## 0. Where the machinery actually lives

| Layer | Path | Role |
|---|---|---|
| Evidence algebra | `vendor/truck/truck-base/src/evidence.rs` | `Outcome`/`Certified`/`Refusal`/`Budget`/`Margin`/`Modulus`/`Prop`/`Truth`, `RealizationVerdict` |
| Certified kernel | `vendor/truck/truck-certified/src/{kernel,formal,tangency,construct,cfp,domain,interval}` + root `ssi*.rs`, `pair_dispatch.rs`, `certified_map.rs`, `patch_admit.rs`, `hull.rs`, `bvh.rs` | certificate calculus, SSI engine, arrangements, construction program |
| Contact funnel | `vendor/truck/truck-evidence/src/{contact,analytic,num,enclosure,decorators,fid}` | production contact dispatch, analytic reductions, certified numerics |
| Boolean / rewrite | `vendor/truck/truck-shapeops/src/{boolean,facade,rewrite,section,gates,healing,fillet,transversal}` | Select machinery (test/probe-side for `look`; facade-routed per `loop/STATE.md` PB-011) |
| Constructive program | `vendor/truck/truck-geometry/src/constructive/`, `truck-modeling/src/facet_sweep.rs`, `truck-geometry/src/canonical.rs` | spine/frame recipes, FAC backend, canonical carrier enum |
| Realization | `vendor/truck/truck-meshalgo/src/tessellation/`, `truck-topology/src/`, `truck-stepio/src/` | tessellation outcomes, ledger, invariants, STEP I/O |
| Renderer | `src/step.rs`, `src/step/` | look production path (STEP → lattice → `robust_triangulation_with_*_outcome`) |

Two **distinct graphs** are respected in the tree and in this audit: the
realized incidence graph (`truck-topology`, `CertifiedGraph` in
`truck-certified/src/kernel/graph.rs:32` — the two types never unify) and the
construction/proof DAG (`ProvenanceSet` in
`truck-certified/src/formal/outcome.rs:163-250`; facade op lineage in
`truck123d/src/assembly_emit.rs`).

**Critical wiring fact.** The `truck-certified::kernel` wave (atlas, trimclip,
assemble, promote, identity, tier1/tier2) has **no production consumer**: it
is exercised through `vendor/truck/truck-certified/tests/kernel_*.rs` inside
the packet loop. The production render path (`src/step.rs` → `truck-meshalgo`)
consumes only `truck-certified`'s `formal`, `domain`, `source_evidence`,
`meshable` modules (`truck-meshalgo/src/tessellation/mod.rs:667-668`). The
boolean machinery is a dev-dependency of `truck-stepio`, not of `look`
(root `Cargo.toml`), and reaches the corpus only through the `truck123d`
facade (PB-011 r1 routing, `loop/STATE.md`).

---

## 1. Generate

### 1.1 Spine/frame recipe — LANDED

- `truck-geometry/src/constructive/recipe.rs:56-94` `SpineFrameRecipe<S,P,F>`
  with `position` (`X(s,v)=C(s)+T(s)·P(s,v)`, recipe.rs:359-367), `frame`
  dispatcher (372-406), `profile` (412-417).
- `Frame3::try_new` validates finiteness/unit/orthogonality/handedness
  (`constructive/mod.rs:73-128`, gate ORI-FRAME-ORTHONORMALITY-GATE-001).
- All four `FrameLaw` variants are implemented, not stubs:
  `FixedPlane` (`frame_fixed.rs:18-49`), `ArchitecturalUp`
  (`frame_up.rs:21-51`), `RadialAboutAxis` (`frame_radial.rs:22-76`),
  `ParallelTransport` double-reflection Bishop frame
  (`frame_transport.rs:43-141`), with an exact rational RMF fast path for the
  `Ph` spine (`spine_ph.rs`).
- All seven `ConstructError` variants (`errors.rs:9-57`) are constructed in
  production `src` (e.g. `ZeroTangent` recipe.rs:379; `SpineNotC1`
  recipe.rs:330 from `PolylineSpine` corners; `ProfileCollapse`
  profile.rs:44).
- Partial: `SamplingPolicy::ChordTolerance/AngularTolerance` exist but
  `resolve` refuses `InvalidInput` unconditionally
  (`constructive/sampling.rs:75-77`, booked follow-up). `PhSpine` membership
  classifier refuses with `PendingMembership` (`spine_ph.rs:32-33,93`) —
  exact membership certification external. Both are honest typed refusals,
  not silent clamps.

### 1.2 FAC facet backend — LANDED

`truck-modeling/src/facet_sweep.rs`: `facet_sweep` (88-277) emits a structured
grid where "the position array IS the grid registry: grid vertex (i,j) lives
at index i·k+j, exactly once" (134-144); no sewing/welding/healing (doc 1-9).
Returns `FacetSweepResult { mesh, audit, verdict, realization_certificate,
shared_edge_pairs }` (56-74); winding audit (283-304); three-valued
`FacetVerdict` (28-36); certified entry `facet_sweep_certified` (742-764).
Caps by deterministic ear clipping (410-662).

### 1.3 Extrude/revolve/primitive constructors — LANDED, narrow primitive set

`truck-modeling/src/`: `extrude_profile` family (`extrude.rs:71,98,127`),
`extrude_until` (`until.rs:114`), `revolve_profile` (`revolve.rs:82`) with
explicit carrier tables (line→`Plane`, circle→`Cylinder`/`Cone`, slanted→
`Cone`); axis-crossing revolve profiles refuse `NonCanonicalCarrier`.
`primitive.rs` has only `rect`/`circle`/`cuboid` — no standalone
sphere/torus/cylinder constructors (those exist as carriers, not constructors).
Sweep machinery: `builder.rs` `tsweep`/`rsweep`/`homotopy`/`try_attach_plane`;
`spine_sweep.rs:56` authors topology BREP from a recipe (side faces per
profile edge, trajectory edges shared by identity, caps via
`try_attach_plane`; all seven `ConstructError`s mapped into the refusal
envelope at 297-303).

### 1.4 Loft / blend / canal / shell program — LANDED (one PARTIAL arm)

`truck-certified/src/construct/`: `loft.rs` (CC-010, `LoftOutput`
`BSplineSurface<Vector4>`, refusals `InvalidInput`/
`SingularInterpolationSystem`, never a fallback solve), `loft_weights.rs`
(CC-011 `certify_weight_field` → `WeightCert`, dyadic subdivision +
`hull_bernstein_2d`), `loft_strips.rs` (CC-012, exact knot insertion, bitwise
seam gate), `loft_validity.rs` (CC-014: `LoftValidityCert` with three-valued
`PairVerdict`, per-strip `rank_margin` regularity, derivative Bernstein
bounds du/dv/uu/vv/uv, `ConditioningBelowThreshold` carried inside the
certificate), `gordon.rs` (CC-015), `blend.rs`/`blend_varradius.rs`
(CC-030/031 rolling-ball and variable-radius foot-point closure with a
certified uniqueness gate), `canal.rs` (CC-025 closed-form regularity with
torsion cancellation; uses `leaf.normal_cone`), `shell.rs` (CC-023
`ShellCert`, Jordan–Brouwer corollary), `setback.rs` (CC-033, four certified
counts incl. `Embeddedness`), `stars.rs` (CC-022 certified `reach_prune`
broad phase), `graphdisk.rs` (CC-005 decider), `contact3.rs` (CC-020 reduced
7→4 system via `krawczyk_c1_n4`), `offset_strata.rs` (CC-021), `banded.rs`
(CC-001 interval no-pivot elimination), `correspondence.rs` (CC-013 fixed
resolution order, refuses `AmbiguousCorrespondence`), `injectivity.rs`
(CC-002 δ=2σ/L from certified rank-margin and `sup‖D²S‖`).

**Degeneracy arm (audit §9.B):** collapsed sections/apex have **no dedicated
typed outcome** in the loft core — caller precondition + `validate_stations`
refusing non-strictly-increasing stations as `InvalidInput`
(`loft.rs:172-176,242-245,296`); degenerate regions surface as
`ConditioningBelowThreshold`. Rational normal numerators, derivative
Bernstein bounds, normal cones and deflation **do exist** but kernel-side:
`formal/bezier_isect.rs::numerator_polys` (X′W−XW′), `kernel/atlas.rs` sphere
numerators + `normal_cone`, `kernel/patch.rs::Cone`/`normal_cone`,
`kernel/mod.rs:125` deflated R6 residual. Authored seam/collapse structure is
handled in `loft_strips.rs` (exact split-value registry) and, at the carrier
level, `Atlas` `DegeneracyRoute::CarrierSingular` for the cone apex
(`kernel/atlas.rs:117`).

Stub posture: `construct/stubs.rs` is the only surface returning
`ConstructRefusal::Unfrozen` (116-271); the frozen 14-variant refusal
vocabulary is `construct/refusal.rs:34-97`.

### 1.5 Generate: O1–O8 coverage

| Obligation | Status | Ground |
|---|---|---|
| O1 input/carrier fidelity | covered | finiteness, unit weights, strict stations, `ConstructError::NonFinite/InvalidInput` (73 sites) |
| O2 local regularity | covered | `rank_margin` (CC-014), `Frame3` gates, canal closed-form regularity, `injectivity.rs` δ |
| O3 incidence/parameter | covered at authoring | `spine_sweep` shared-by-identity trajectory edges; correspondence never inferred (CC-013) |
| O4 interaction coverage | deferred to Interact | construct arms funnel into `shell.rs` → evidence contact funnel |
| O5 trim/membership | n/a at Generate | — |
| O6 global realization | partial | `Embeddedness` (setback), Jordan–Brouwer corollary (shell.rs); global embedding otherwise left as downstream obligation |
| O7 evidence transport | absent | no transform-of-evidence (see §2) |
| O8 target fidelity | covered | FAC three-valued verdict; `Method::Float` discipline (H-6) |

---

## 2. Map

**Map(transform, object) — ABSENT as a certificate-carrying production.**

- `truck-certified/src/certified_map.rs` is a *differently named* thing: it
  admits a spline over a compact domain to a Bézier piece table and answers
  certified `enclosure`/`rank_margin` queries (`admit_curve:192`,
  `admit_surface:253`; refusals `ParameterizationDegenerate`,
  `EnclosureUnavailable`, `DomainNotCompact`). Rational B-splines are out of
  its scope by doc (66-69). It is a *certified map of a spline*, not a
  transform production.
- Tree-wide: no `impl Transformed` and no `transform_by` in
  `truck-certified`; no rigid/reflection/scaling/instance vocabulary; **no
  transform-of-evidence logic** (no chain-rule/derivative-scaling/exactness
  claims transported with an object).
- Geometry-level transport exists and is exact-placing:
  `canonical.rs:374-434` `Surface::transformed` keeps bare analytic carriers
  under exactly-identity 3×3 else wraps in `Processor` (BG-CE-006-r2);
  `SpineFrameSurface` composes the placement into the sweep matrix (431).
  Topology-level `Mapped` (`truck-modeling/src/lib.rs:58-64`) and
  `builder.rs:424-443` / `cad.rs:288-414` helpers transport **no evidence**.
- Instances: `truck-assembly` is a structure-only DAG (`assy.rs`, `dag.rs`);
  the facade records contact *intents*, never resolves them
  (`truck123d/src/assembly_emit.rs`, `EvidenceRowKind::Intent`).

Consequence (O7): a rigidly placed copy of a certified object re-derives all
geometric claims. This is a deliberate current boundary, not a hidden
correctness hazard — nothing claims transport.

---

## 3. Interact

### 3.1 The 4-D certified SSI engine — LANDED (engine); wiring PARTIAL

- Form frozen in `truck-certified/src/ssi.rs` module doc (15-78):
  `F_k(u,v,s,t) = W2·N1_k − W1·N2_k`, k∈{x,y,z}, stored **per side**
  (`SquareSystem3`, `ssi_types.rs:102`) — separability avoids materializing
  the 4-D tensor (CFP-003, `ssi_types.rs` doc 41-54).
- F3 square reduction `f3_diagonal_derivatives` (ssi.rs:996) with frozen
  continuation-axis rule (`contract.rs:376`); `ConditioningBelowThreshold`
  refuses, never retries weaker.
- `krawczyk3_certificate` (ssi.rs:1213): certified Bernstein partials via
  `partial_enclosure` (749) over per-side 2-D hulls (`hull.rs`:
  `hull_bernstein_1d:95`, `hull_bernstein_2d:126`), adjugate inverse under
  directed rounding (`adjugate3:1082`), emission only on strict inclusion
  through the refusing constructor `KrawczykCertificate3::new`
  (`ssi_types.rs:451`); typed `DeterminantSpansZero`/`InclusionNotStrict`.
- Trace/continuation `ssi_trace.rs`: `trace_branch:206` (identity-recurrence
  closure → `ClosedLoop`, frozen `CoordinateSwitch` only under
  both-certificate, named `TraceRefusal`), `ProductionCertifier:697`
  (certified predictor along the null-direction tangent, `MAX_STEPS`,
  closed-loop detection), `certified_pair_trace:838`, multi-patch
  `stitch_fragments` (859-1022, CL-002), germ classification
  `classify_branch_germ:332` (zero-jet ladder), strict-interior event test
  (300). **Callers: only `tests/ssi_trace.rs`.** The production funnel never
  invokes the trace.

### 3.2 Admission into SSI (audit §9.A) — LANDED, deliberately narrow

- `patch_admit.rs::admit_surface` (151): clamped `BSplineSurface<Vector4>`
  → per-knot-span Bézier `RationalBipatch` stack by exact Boehm insertion.
  Restrictions: **rational weights refused** (`UNIT_WEIGHT_REL_TOL=1e-7`,
  `RationalWeights` at 196 — D2), bidegree ≤ 8 (`MAX_BIDEGREE:97`),
  clamped-only, finite net.
- `ssi_admit.rs::certify_spline_analytic_pair` (136): spline side **single
  knot span only** (`WindowNotOneSpan`, 148; cross-patch assembly is CL-002's
  booked content, stitch machinery exists); analytic side **`Plane` only**
  via the exact-affine recognition hook `EnclosureSurface::as_plane`
  (`truck-evidence/src/enclosure.rs:194`); non-affine analytic carriers
  refuse `AnalyticNotAffine`.
- The engine itself accepts any two `RationalBipatch`es with positive weight
  grids (`ssi.rs:1356-1410`) — i.e. **the representation admits positive
  weights; the admission layer refuses to produce them.** This is an
  admission-closure gap over a landed substrate (see gap register AD-1).
- `SsiAdmitSolver` registers into `truck-evidence`'s set-once slot
  (`ssi_admit.rs:629` → `contact/mod.rs:710`), but the **only registration
  call site is `#[cfg(test)]`** (790). Unregistered,
  `spline_analytic_contact` (`contact/mod.rs:762`) answers typed
  `NumericallyUnresolved` (795-800). The solver *entry* is LANDED; the
  production seam is not closed (BREP_SOLVER_AUDIT S9).

### 3.3 Analytic pair reductions (audit §9.D) — LANDED on the evidence side

- Production funnel: `truck-evidence/src/contact/mod.rs::analytic_ff`
  (1019) over `CanonicalSurface` (plane/plane, plane/sphere, sphere/sphere,
  plane/cylinder, plane/cone, coaxial cylinder-family, parallel cylinders;
  torus×{plane,cyl,cone,sphere} via `torus_ff:1941`).
- Exact families (`truck-evidence/src/analytic/`): all eight modules landed,
  deciding by outward-rounded `inari` predicates, undecidable ⇒
  `NumericallyUnresolved`: includes plane×cylinder **ellipse**
  (`plane_cylinder.rs`), plane×cone full conic classification via
  `three_way(Nz², t²(Nx²+Ny²))` (`plane_cone.rs`), coaxial families
  (`coaxial.rs`), parallel cylinders, equal-radius cylinders with exact
  rational ellipse parameterization (`equal_radius_cylinders.rs`).
- Certified-side parallel dispatcher: `truck-certified/src/pair_dispatch.rs::
  dispatch_pair` (255) — five arms only (plane_plane, plane_cylinder,
  plane_sphere, sphere_sphere, cylinder_cylinder coaxial/parallel subset);
  oblique ellipse cut refuses `UnsupportedPairClass` (579-582). **Callers:
  only `tests/pair_dispatch_conformance.rs`.** Two dispatchers coexist with
  drifting admitted configurations (evidence side emits ellipses; certified
  side refuses them) — flagged as layering-vs-drift risk in gap register
  AD-4.
- Measured: the Phase-1 floor run recorded per-call dispatch latency over the
  full corpus (`BREP_EXISTING_RUNTIME_EVIDENCE.md` §1.1): median 6.4 µs,
  p95 23.1 µs, max 8.22 ms.

### 3.4 Contact funnel (production) — LANDED with typed deferred regions

`truck-evidence/src/contact/mod.rs::contact(lhs,rhs,budget)` (272), called
from production by `truck-shapeops/src/boolean/assemble.rs::emit_contact`
(39, 355-363):

1. C0–C2 identity/overlap screens (`overlap.rs`).
2. FF analytic (§3.3).
3. Validated FF: `gff::cover_branch` — certified AABB intersection then
   branch-cover enumeration over a 3-D box: interval exclusion + chart-aware
   2×2 slab Krawczyk (a certified-nonzero minor selects the chart); output
   `BranchCover { points, singular_boxes, unresolved_boxes }` (`gff.rs:73`).
   A box whose minors all contain zero "is never called proven rank
   deficiency" (`gff.rs:25-27`).
4. Singular events: `singular::singular_events` — exact degenerate points,
   Lagrange-system certification via the **4-D Krawczyk operator** over a
   multiplier envelope (documented τ-widening deviation, doc 32-40),
   restricted-Hessian inertia separating isolated tangencies (definite) from
   gradient-parallel saddles (indefinite); unproven residues deferred with
   named reasons.
5. FE/EE exact tables (`fe_ee.rs`): Line/Circle × Plane/Cylinder etc.;
   Line×Cone/Sphere, Circle×Cone/Sphere, transverse Circle×Cylinder,
   Circle×Circle refuse `ContactReductionDeferred`.
6. Sweep restricted dispatch: `BoundedStratum::Sweep` carries
   `SpineFrameSweep` through the dependency-inverted
   `solver_entry::dispatch_restricted_sweep` registry
   (`solver_entry.rs`); with a registered engine this goes to
   `construct/bie/ssi4.rs::certify_restricted_pair` (interval evaluation on
   the carrier maps — no cross-multiplication). No engine ⇒ typed
   `NumericallyUnresolved`. **Known defect (existing recorded evidence):
   BREP_SOLVER_AUDIT S1 (P0)** — `restricted_sweep_chart` (ssi4.rs:2341)
   admits an equal-radius polygon profile but constructs a continuous
   circular sweep ("continuous circular-section limit"), which changes the
   represented surface; **S2 (P0)** partial restricted traces lose their
   unresolved remainder; **S4 (P1)** the caller's budget does not bound
   restricted SSI work.
7. Everything else: `Refusal::UnsupportedEnvelope(ContactReductionDeferred)`.

Spline faces never enter `contact()` itself: `face_stratum` (944) refuses
non-canonical carriers at the lift; `CanonicalSurface`
(`truck-geometry/src/recognize.rs:45`) is Plane/Cylinder/Cone/Sphere/Torus/
Placed only. The CFP-004 implicit-reduction pre-screen
(`implicit2d.rs::screen_empty`, called at 781) certifies empty
plane/quadric×spline-cell in one scalar hull — **torus excluded** from the
reduction (TOR-A books "measure before committing" for the quartic
composition; `docs/TORUS_CONTACT_PROGRAM.md`).

### 3.5 Tangency (audit §9.C) — LANDED machinery, PARTIAL wiring

`truck-certified/src/tangency/`:

- Chart selection: bounded 18-chart rank-2 search, first chart whose
  certified det excludes 0 (`chart.rs::find_chart`). LANDED.
- Krawczyk contraction: point-box 3×3 (`engine.rs`) and parametric H-graph
  forms (`graph.rs`), bisection under Budget, exhaustion ⇒
  `TangencyRefusal::GraphFailure`. LANDED.
- Rank-deficiency: via composed chart-minor nets and a five-equation
  exclusion driver (`minors.rs`, `tsystem.rs`, `exclude.rs::exclude_five` —
  budget exhaustion ⇒ `ExclusionEvidence::Inconclusive`, never a guess);
  explicitly *not* labeled "proven rank deficiency". LANDED as machinery.
- Reduced-Hessian definiteness (Gershgorin + closed-form R9 μ test,
  `hessian.rs`) wired into the cascade (`cascade.rs:68`).
- Cascade: `classify_box` (`cascade.rs:703`) stages 0–7 ending
  `Unresolved{kappa, cell}`; **stage 6 (the A2 route) is explicitly not
  reachable from this signature** (doc 31-33).
- Rank-3 A2: `a2.rs::certify_a2` (1566) — exact contact-factor identity over
  ℚ, `0∉μ(B)`, `0∉Λ(B)`, rank-3 minor separation, **R6 non-emptiness
  required**, parallelotope continuation producing `A2BranchCurve` with
  certified samples/frames. **LANDED but test-gated** (callers: `gates.rs`
  doc-hidden support and its own tests).
- Exact witnesses: `qpoly.rs`/`witness.rs` exact ℚ ring + one total
  verifier; `provenance_witness` is a named pending stub
  (`WITNESS_PRODUCER_PACKET_PENDING`).
- T2 coincidence arrangement: `arrange.rs::atom_decision` (689) —
  **scope-limited** to chart-straight/axis-aligned tangential strata; general
  curved-cell arrangement is the open §6.3 tail; no caller outside its tests.
- Fold certification: `FoldCert` (`ssi.rs:201`) is a refusing-constructor
  carrier only; FSSI-002 population not landed. **SPEC/SHAPE ONLY.**

Production reachability of the tangency stack: `ssi.rs::route_stagnation`
(CFP-008, ssi.rs:1564) is the intended route, whose only caller is
`cascade.rs`'s own test. So the cascade is **machinery-LANDED,
wiring-PARTIAL/ABSENT**.

### 3.6 Self-interaction (Interact(A,A)) — LANDED machinery, PARTIAL integration

`kernel/selfint.rs` (R6: deflation, exact cover, chart transitions, λ=0
stratum) with the deflated R6 residual in `kernel/mod.rs:125`; graph carries
`SelfInt` arcs (`kernel/graph.rs:228`); residual driven by
`tests/kernel_selfint.rs`. Kernel wave is test-integration only (§0).

### 3.7 Certified curve machinery — LANDED (no resultants)

`truck-certified/src/formal/`: `intersection.rs` line×line/circle×circle by
exact expansion predicates (`orient2d`, exact discriminant);
`bezier_isect.rs` (GEN-001C) bivariate isolation of two homogeneous rational
Bézier spans by Bernstein subdivision + interval Krawczyk, clustered/multiple/
tangential/boundary ⇒ typed `GenericUnresolved`; `bezier.rs` (GEN-001B) with
the positive-weight certificate `W(u)>0` (`WeightMayVanish` refusal);
`common_arc.rs` (GEN-001E) positive-dimensional overlap **only** from
provenance-identical or exactly-identical analytic support (identity from
source identity, "never from evaluated geometry"); `xmonotone.rs` algebraic
critical points; `curve_witness.rs` cylinder curve witnesses. No algebraic
resultants anywhere — subdivision + exact predicates + Krawczyk is the
decision substrate.

### 3.8 Diagnostic libraries, not wired — LANDED-as-library

- `truck-evidence/src/fid/` (isotopy conditions with open bridge lemmas L-TUBE
  / L-COVERING / L-SEPARATES recorded OPEN, `isotopy.rs` doc 20-26; local
  feature scale `lfs.rs`; one-sheet fibre-degree certificates `one_sheet.rs`;
  `rep_curve` as the sanctioned exact-curve→geometry path, `rep.rs:474`).
  No production callers.
- `deviation.rs::certify_deviation` (279) certified leader-vs-carrier
  deviation; Route 1 exact spline-difference hulls (zero subdivisions for
  exact agreement). Re-exported at `truck-evidence/src/lib.rs:84`; **no
  callers**.
- Enclosure substrate: `enclosure.rs` (`EnclosureCurve:154`,
  `EnclosureSurface:177` incl. `as_plane:194`, `normal_cone`,
  `immersion_lower_bound`) with implementations for all analytic carriers and
  decorators — **extruded.rs (with the singular-locus rule C′×V=0),
  revolved.rs, processor.rs** landed (BG-ENC-004 family);
  `pcurve.rs`/`intersection_curve.rs` scaffolded; `offset.rs` blocked.
- `bvh.rs` (CFP-005) per-carrier span BVH broadphase: float search is a typed
  `FloatHint`; separation only where the exact `Expansion` sign row certifies
  ("the float layer can never cause a false prune"). LANDED.

### 3.9 Where subdivision is the only path before typed refusal

`exclude_five` → `Inconclusive`; `graph.rs` → `GraphFailure`; `gff.rs` →
`unresolved_boxes`; `singular.rs` → deferred residues; `num/roots.rs` even
sign change ⇒ `NumericallyUnresolved` (prevents the tangential-double-root-
reported-empty failure); `formal/bezier_isect.rs` ⇒ `GenericUnresolved`;
`ssi_trace.rs::certify_box` width ladder (588) ⇒ named `SsiRefusal`;
`construct/bie/closure.rs` escalation scheduler. Deliberately bypassed:
`deviation.rs` Route 1 and `implicit2d.rs::screen_empty` (one-hull empty
exclusion). Rank-3 regular interactions are *not* subdivision-only —
Krawczyk contraction decides; near-tangency/tangency is where the generic
subdivision floor binds.

### 3.10 Interact: O1–O8 coverage

| Obligation | Status | Ground |
|---|---|---|
| O1 | covered | `SideCarrier` refuses degree-0/ragged/non-finite/non-positive weights (`ssi_types.rs:136-157`) |
| O2 | covered | certified partials, normal cones, rank margins, Hessian inertia |
| O3 | partial | single-span requirement; cross-patch assembly CL-002 booked; `common_arc` support-identity doctrine strong |
| O4 | **the hard one** | engine-side completeness discipline exists (Tier-1 loop-free certificate `kernel/tier1.rs:71`, Tier-2 critical-point start set `kernel/tier2.rs`, germ/event ladder); the *production funnel* has honest `unresolved_boxes` remainders instead of completeness proofs; one traced branch is never claimed complete |
| O5 | out of scope here | see §5 |
| O6 | partial | R6 self-interaction machinery test-wired only |
| O7 | **absent** | no evidence transport under placement; see §2 |
| O8 | covered | typed refusals/`NumericallyUnresolved` with budgets; three-valued discipline end-to-end |

---

## 4. Refine

- **`kernel/trimclip.rs`** — LANDED (packet/test-wired). `trim_clip:777`
  clips clippable arcs against closed trim loops: certified R9 crossings
  (`certify_crossings:341` via `R9System`+`krawczyk_c1` to `DEPTH_MAX`),
  arc splitting, inside/outside classification by exact `i64` winding number
  with certified Bernstein sign discipline (`winding_number:528`) over a
  certified-off interior sample (`OffLoopCert`, `certify_off_loop:430`);
  crossings become `TopoNode::TrimCrossing`. Multi-loop trims use loop
  **intersection** semantics (sample inside every loop, 983-987) — no
  nesting/island logic. Periodic seams only via lifted-chart deck carry
  (`Param::try_new(ends.chart, ends.deck, …)`, 944-945). **Collapsed
  boundaries not handled**; open loops refuse
  (`trim_loop_open_cannot_bound:778`); exhaustion ⇒
  `RefusalKind::TrimClipFailed` (Inconclusive).
- **`kernel/atlas.rs`** — LANDED for plane/cylinder/sphere (sphere pole-chart
  partner/stereographic family, cylinder exact angular deck generators);
  **PARTIAL for cone/torus**: chart families constructed, but chart leaves
  certify structure only with named pendings `CONE_FORM_PENDING`/
  `TORUS_FORM_PENDING` (90, 94). Cone apex routes
  `DegeneracyRoute::CarrierSingular` (117).
- **`kernel/leaf_extract.rs`** — exact Boehm per-knot-span extraction from
  `NurbsSurface<Vector4>`; refuses non-clamped/non-positive.
- **`formal/` arrangements (the production-wired Refine layer)**:
  `planar_slice.rs` (Jordan arrangement + ear clipping + 11-item validity
  battery; production via `formal::run_planar_slice`,
  `triangulation.rs:3131`), `planar_holes.rs` (outer-minus-holes;
  production-wired **but recovers 0 faces today** — all 187 exits are
  `unsupported_curve_representation` because arc bounds are unsupported
  upstream, honest measurement recorded in the module doc),
  `cylinder_band.rs`/`cone_band.rs` (two-bound band admission with edge
  identification, intrinsic material authority; the one scoped repair of the
  double-`FACE_OUTER_BOUND` malformation tagged `RecoveredFromMalformedSource`),
  `rank1_annulus.rs` (shared rank-1 periodic realizer for cylinder+cone),
  `torus_cell.rs`/`torus_circle.rs` (rank-2 annular cells with null-homology
  material authority; `Z²` winding by exact signed seam-crossing counting
  over whole-interval Fourier certificates — no `atan2` sampling), `deck.rs`
  (certified integer deck arithmetic, four-way `DeckSolveResult`), 
  `cylinder_lift.rs`/`cylinder_cover.rs`/`cylinder_face.rs` (production),
  `support.rs` (unforgeable `PlaneSchema`; production via
  `look/src/step/lattice.rs:412`), `ambient.rs` (five distinguishable period
  states; production).
  **Substrate, test-only:** `quotient.rs` (deck labels as integer pairs,
  `DeckSignature` gauge-invariant), `span.rs` (`SpanId{edge_use_id,
  source_edge_id}` — identity never coordinates), `planar_developed.rs`
  (exact arc development, produces no mesh), `envelope.rs`
  (`FormalEnvelope`/`ExecutionBudget` — exceeding an envelope is
  `Unsupported`, a face property; exhausting a budget is
  `OperationalFailure`, a run property; deliberately two types),
  `cylinder_arrangement.rs` (superseded on the production path by the band
  route).

**Refine closure kinds:** representation closure holds for the landed carrier
classes (lines, arcs, Bézier spans, unit-weight splines, canonical surfaces
with charts); the *seam-crossing pcurve inside one un-recoverable leaf* case
refuses (`TrimClipFailed`) rather than mis-representing — an admission gap
typed honestly. Completion closure is **not** proved anywhere: `DEPTH_MAX`,
`DECK_MAX`, and `ExecutionBudget` are budget ceilings, not proved envelopes
(`formal/outcome.rs` deliberately keeps envelope-violation and budget-
exhaustion distinct; no theorem in-tree bounds subdivision).

---

## 5. Select

Three independent certified classification layers exist; there is **no single
ambient inside/outside decider**.

1. **Kernel trim winding** — `trimclip.rs` (§4) + deck winding
   (`kernel/assemble.rs::deck_identify`, exact integer sums, winding recorded
   on `SegmentBreak::DeckStep` breaks).
2. **Formal pipeline (production Select for STEP ingestion)** — exact Jordan
   material regions (`planar_slice::bounded_material_region`,
   `planar_holes::classify_components`), null-homology material authority
   (`torus_cell`), effective orientation folding traversal×edge×loop×bound×
   surface conventions, torus `Z²` winding.
3. **Boolean parity graph (truck-shapeops)** — `boolean/mod.rs` §5.4
   coordinatewise two-bit combine over `MaterialState4`; `classify.rs`
   seed-and-propagate: one certified seed per component (`find_seed:237`),
   bits propagated across `AdjacencyParity` edges, every non-tree edge
   verified, first violation ⇒ `Refusal::Contradictory`. **Seed-ray status:**
   the actual seed geometry `classify::surface_ray_crossings` is plain f64
   with `NORMAL_SLACK` guards; the certified interval ray machinery
   `ray_cert.rs` covers **only Plane/Sphere/Cylinder/Cone** and is
   explicitly standalone ("does NOT touch the live classify path — that is
   DEF-SEEDRAY-B", doc 4-9). This is existing recorded finding **S7 (P0)**:
   the canonical seed path still uses uncertified root/trim decisions.

**Boolean pipeline status** (`truck-shapeops/src/boolean/`): LANDED as the
certified replacement for the legacy `transversal/` path ("transversal is
rewritten, not extended", `split.rs:6-7`): `assemble.rs::boolean` (71) →
lift via `recognize_surface` → AABB-screened broadphase (CFP-009, exactness
contract) → `split_fragments` with **shared edge instances between the two
solids** (the RW4 closing mechanism) → classify → decide → sew; `Solid::
try_new` is the acceptance gate; all-discarded = empty solid. Closedness is
enforced by construction + acceptance, orientation by the coincident-pair
fold (`CoincidentOrientation::{Identical, Anti}` with `flips_ok`); **the
homology gate** (`gates/homology.rs`: χ + Z₂ Betti over the output
2-complex, mismatch = typed FAILED) runs **beside**, not inside, `boolean()`.
`Outcome<Solid>` with single-shell guard (multi-shell refused at 79).

**Carrier admission:** default canonical×canonical
(`require_canonical_carriers`, `classify.rs:572`); swept carriers admitted
only through the BIE-006 windowed-stratum adapters
(`sweep_lift.rs::sweep_face_stratum:73`, `classify_from_cells`,
`windowed_sweep_face`; branch at `assemble.rs:93-103`). Per `loop/STATE.md`,
PB-011 r1 LANDED the swept-carrier routing (mirror of the CL-006
solver-entry dispatch) and the cutaway row lifted green; the monocoque lift
is PB-011 r2. Existing recorded defects against this pipeline: **S6 (P0)**
sweep fragment classification assumes unproved planarity/convexity; **S8
(P1)** even empty-set and disconnected results lack re-entry closure; **S10
(P1)** fragment identity/evidence computed but not preserved as a contract.

**Select closure kinds:** holes are represented (annular faces survive;
faces-with-holes cross-cut deterministically for the homology gate);
periodic domains are the strongest area (band forms, ±period query
translates, `unwrap_periodic_parameter`); seam crossing handled via
parameter-unwrap + deck carries; nonmanifold results are *refused*, not
represented (`Solid::try_new`); **`OpenShell`/`BodySet`/`Empty` as distinct
public output kinds are SPEC ONLY** (`docs/BREP_SYSTEM.md:55` names them; no
such enum exists — see §7). Legacy `transversal/` (integrate/loops_store/
polyline_construction) is upstream machinery, referenced by nothing in the
certified pipeline; `healing/` is closed edge/face splitting only (cylinder
limited); `fillet/` is the legacy single-edge three-face fillet — certified
blending lives in `construct/blend*` + `rewrite.rs` (right-dihedral plane
envelope).

---

## 6. Identify

- **`kernel/identity.rs`** — LANDED. `IdentityRule {RuleA, RuleB, RuleC}`;
  Rule A exact box containment against a union certificate ("No tolerance
  anywhere", 84-94); Rule B transport across deck translations with exact
  representability check via two-product `fma` (`exact_deck_shift:194-213`,
  refuses `NonFinite`) + outward-ULP affine transport (222-248); Rule C only
  via the typed `implication` relation (150-168). Dyadic join is exact
  integer address arithmetic (253-348); `refuse_custom_on_shared` refuses
  `NonDyadicSharedRequest` (355-368). **Coincident coordinates never
  identify** — the module doc pins the doctrine (19-23).
- **Shared-edge pair evidence** — `truck-base` `SharedEdgePairEvidence`
  {error_a, error_b}; populated by
  `truck-meshalgo/src/tessellation/realization_evidence.rs::
  ledger_shared_edge_pairs` (51-84): compares the two faces' ledgers of the
  same edge **as integer index sequences** (`I(A,E)==reverse(I(B,E))`);
  "Exactness is expressed by absence of rows"; nothing welded, averaged, or
  rounded.
- **Edge sample ledger** — `triangulation_with_ledger.rs`:
  `EdgeSampleLedger { edge, parameters, position_indices }` with exact f64
  bit-pattern interning (`-0.0` normalized); runs the unchanged production
  path and builds the ledger **beside** it — evidence, not yet the mesh
  driver. Known adapter defects recorded in
  `docs/audits/BREP_TOPOLOGY_AUDIT.md`: the global keying mismatch
  (confirmed P1) and the exposed-edge-sampler contract gap.
- **Quotient/deck (formal layer)** — LANDED: `formal/quotient.rs` (`DeckLabel
  {u: i64, v: i64}`, "identity is the integer pair, never a rounded
  coordinate", 93-95; `CertifiedDeckLabel` lattice-bound with typed
  `LatticeMismatch`; gauge-invariant `DeckSignature`), `formal/deck.rs`
  (four-way `DeckSolveResult`, never nearest-integer rounding),
  `formal/common_arc.rs` (`CommonSupportBasis::{IdenticalSourceProvenance,
  IdenticalAnalyticSupport}` — identity from provenance, "approximate
  center/radius or sampled-point equality is never used").
- **Domain layer** — `domain/lattice.rs` `CertifiedLattice` is the landed
  production boundary (`PeriodWitness::{ExactRevolutionAngle,
  ExactSphereAzimuth, SourceDeclaredClosedSplineAxis}`; "no
  numerically-verified variant, and there must not be";
  `AxisPeriodStatus::Uncertified` is "**never** offered as a deck
  generator"); `domain/{schema,quotient,deck}.rs` are prototype-leaning
  (consumers only inside `domain`; `DeckPotentialUnionFind` — the correct
  QUO-004 solver per `CODE_NAVIGATION_INDEX.md:134` — waits for inputs).
- **STEP ingestion identity** — `truck-stepio/src/in/arena.rs` typed
  per-kind `SourceId<K>`/`Index<K>` arenas with transactional
  `get_or_try_insert`; `CompressedFace.provenance {use_id, definition_id,
  surface_id}` retains entity identity
  (`truck-topology/src/compress.rs:190-218`); pcurves parsed and realized
  (`PCURVE`/`SURFACE_CURVE`/`SEAM_CURVE` at `in/mod.rs:535-542`; gated
  controlled fallback `reconciling_pcurve_fallback:4002-4023`); a strict
  path refuses pcurves as 2D curves (1828).
- **`source_evidence.rs`** — Step-0 measurement seam: five-factor
  orientation evidence populated but explicitly **not consumed in production
  decisions** (doc 11-16); tripwire expects zero
  `computable_normalized_sign_count` on the compressed path.

---

## 7. Realize

- **Evidence algebra (Layer A)** — `truck-base/src/evidence.rs`:
  `Outcome<T>=Result<Certified<T>,Refusal>` (31); `Certificate` = (π,μ,β,𝔪,ω)
  (166-177); `Method {Exact, Interval, Float, None}`; `Truth` knowledge order
  with `join` → `ContradictionWitness` on `Both`; typed `Budget {subdiv,
  newton, depth}` with `Exhausted{counter}` (306-382); 13 typed `Prop`
  invariants; **`RealizationVerdict {CertifiedWithinTolerance, Failed,
  Inconclusive}`** (673-683), `RealizationEvidence` (714-725),
  `SharedEdgePairEvidence` (701-708). The ALL-CAPS
  `CERTIFIED_WITHIN_TOLERANCE` tokens exist only in docs; the code enum is
  the authority.
- **Kernel claim algebra (Layer B)** — `kernel/evidence.rs`:
  `ClaimVerdict {Proven, Disproven, Inconclusive}` (59-66); `Refusal {kind,
  backing, evidence, partial: Option<PartialGraph>}` (93-103) — a partial
  result can only ever exist **inside** a refusal (rule 6); 25-variant
  `RefusalKind` taxonomy (169-233) with a `default_backing` table
  distinguishing 10 Inconclusive from 15 Disproven kinds.
- **Certificates (Layer C)** — `kernel/certs.rs` refusing-constructor
  carriers (`PointCert/PointCert3/PointCert4/ArcCert/ContactCert/GraphCert/
  SheetCert/TubeOverlapCert/R5Enclosure`), populated at `engine.rs:373`,
  `projection.rs:225/475/508/586`, `contact.rs:230/473`, `trimclip.rs:366`,
  `sheet.rs:391`. `ContactCert` is the Proven case only (§10.3 rule 7).
  `kernel/claims.rs` D6-by-types: `CertifiedGraph` vs `ClaimedGraph` distinct;
  claim predicates are the fixed strings `tube-chain-via-C2` /
  `endpoints-via-C1` / `nodes-via-A4.2` (112-116); completeness discharged by
  Tier-1/Tier-2 box-complement exclusion (`discharge_completeness:1111-1162`).
- **Scoped/authority evidence (Layer D, O7's strongest in-tree form)** —
  `formal/evidence.rs`: `Evidence<T>` with `Declared/Analytic/
  CertifiedNumerical/Assumed/Unresolved` inner variants; authority-minting
  constructors are `pub(super)` ("authority cannot be forged"); `Unresolved`
  has **no value field**; consumption only via
  `AuthoritativeFact::try_from_evidence(evidence, requirement, use_site)`
  (1051-1099) which consumes the evidence; deliberately non-ordered
  `admits` matrix (1169-1224); `FaceKey {document, shell, source_face_id,
  declared_face_index}` (`formal/outcome.rs:126-135`). `Declared` and
  `CertifiedNumerical` are declared-but-never-constructed, deliberately
  (`#[allow(dead_code)]` with the five-statuses-become-three rationale).
  **Currently only `Analytic` is minted** — certified-numerical authority
  has no production producer yet.
- **Outcome algebra (Layer E)** — `formal/outcome.rs`:
  `StageOutcome {Resolved, Inconsistent, Ambiguous, Unsupported, Unresolved}`;
  `StageEvaluation = Result<StageOutcome, OperationalFailure>` with **no
  `From<OperationalFailure>`** (asserted by test 1206-1216 — machine failure
  is never a geometric verdict); `SemanticOutcome`/`ValidSemantic`/
  `ValidityCertificate`/`RealizationOutcome` are **SPEC ONLY
  (unconstructible by design)** in Step 1; report types are constructed with
  witness-derived reasons (`AmbiguityReport::new` refuses a witness that does
  not separate its alternatives).
- **Production realization evidence** — `MeshedShellOutcome`
  (`triangulation.rs:1274-1308`: mesh + positionally-aligned per-face
  `face_failures`/`face_diagnoses`/`band_attempts`/`cone_band_attempts`/
  `torus_band_attempts`) and `FaceValidityCertificate`
  (`validity.rs:134-235`), both constructed and wired into `look`
  (`src/step.rs:643`, reason counting 396/685/892).
  `TessellationFailure` is constructed exclusively through
  `diagnosis::fail/reject` (refusals cannot bypass diagnostics); 7 of the
  11 `TessellationFailureReason` variants plus `RejectedAmbiguous` are
  documented never-constructed in-source (9543-9636).
- **FAC** — `facet_sweep.rs` (§1.2) with `FacetVerdict` → `RealizationVerdict`.
  `RealizationEvidence::assemble` (`realization_evidence.rs`) — nonzero
  winding count overrides to `Failed` (25-40) — is currently **test-wired
  only**; production realization evidence flows through `FacetSweepResult`
  fields and `MeshedShellOutcome` vectors.
- **Output kinds** — no unified `Empty/Body/BodySet/OpenShell/Assembly/Mesh`
  enum exists anywhere (exhaustive grep: only STEP entity names in a test
  fixture). What exists: `ProductShape {Shells, Solid, Matrix}`
  (`truck-stepio/src/in/convert.rs:705-712`; unknown shapes error), the
  assembly DAG, `PolygonMesh`, opaque facade handles
  (`truck123d/src/python.rs:25-32`). STEP **export** is a Display-based
  writer; the facade refuses STEP out of constructive/swept parts
  (`NonCanonicalCarrier`, TR-NRB-001; `docs/OP_CAPABILITY_MATRIX.md` cell 26).
- **Volume / mass properties (audit §9.E)** — **ABSENT as certified
  machinery**: zero matches for volume/quadrature in `truck-certified`;
  `truck-meshalgo/src/analyzers/volume.rs::CalcVolume` is float divergence-
  theorem over triangles, conditioned in-doc on "if the mesh is closed";
  `facet_sweep.rs::signed_volume_of` feeds the three-valued verdict
  (degenerate volume ⇒ `Inconclusive` floor), not a certified bound. The
  substrate for certified polynomial integration exists (Bernstein
  coefficients, `hull.rs`, `formal/exact.rs::CertifiedInterval`) but no
  composition over trimmed faces is implemented. Certified BRep construction
  and certified volume/reference facts are correctly decoupled today — by
  absence, not by contract.

**Realize closure kinds:** representation closure holds for FAC/PolygonMesh
and for the landed analytic carriers; admission closure is limited by the
absent output-kind vocabulary and the single-shell Boolean gate; semantic
closure is expressed but **unconstructible** (`ValidSemantic`); completion
closure is absent (budgets, not envelopes).

---

## 8. Carrier / domain inventory (Phase C)

### 8.1 Curves

| Class | Storage | Recognized carrier | Certified machinery |
|---|---|---|---|
| line | `Curve::Line` (`canonical.rs:40-65`) | yes | exact everywhere |
| conic (circle/ellipse) | `Circle` placed trimmed unit circle, kept analytic; `Ellipse` in analytic tables | circle yes; ellipse only as analytic intersection output (`plane_cylinder.rs`, `equal_radius_cylinders.rs`) | exact predicates; no ellipse carrier in `CanonicalSurface` |
| polynomial Bézier | B-spline special case | via knot insertion | full (hulls, Krawczyk, subdivision) |
| rational Bézier | `NurbsCurve`/`Vector4` | admission refuses non-unit weights for SSI (`patch_admit.rs:196`); curves with weights admitted in `formal/bezier.rs` with `W>0` certificate | landed on the curve side |
| B-spline / NURBS surfaces | `BSplineSurface`, `NurbsSurface` (`canonical.rs`) | `recognize_surface`: NURBS `Unrecognized` (158); `NurbsSurface` never converted | unit-weight B-spline admitted (§3.2); true NURBS refused |
| procedural / intersection curves | `IntersectionCurve`, `SpineFrameCurve`, `CertifiedImplicitIntersectionCurve` (`canonical.rs:40-65`) | `Unrecognized` at the lift | evaluator-level (`enclosure` for sweeps); no pair dispatch |

### 8.2 Surfaces

| Class | Storage | Recognized | Notes |
|---|---|---|---|
| plane, cylinder, sphere, torus, cone | `canonical.rs:302-347` | `CanonicalSurface` (Placed via `Processor` recognized, `recognize.rs:161-184`) | full funnel: analytic tables, gff, singular, bands, charts |
| `RevolutedCurve` | enum variant | **explicitly `Unrecognized`** (`recognize.rs:155-160`) | emitted by nothing (`revolve_profile` uses carrier tables); a stored one is refused at `face_stratum` |
| `ExtrudedCurve` | enum variant, RESERVED (`canonical.rs:315-318`) | reduced: line→`Plane`, circle→`Cylinder` (`recognize.rs:185-260`) | decorator enclosure exists (`decorators/extruded.rs`) but **no pair dispatch keyed on it** |
| swept (`SpineFrameSurface`) | canonical variant + decorator (`decorators/spine_frame.rs:234`, full trait checklist 518-653) | `Unrecognized` (honest, `recognize.rs:160`); restricted funnel path via `solver_entry` | S1/S2/S4/S6 defects recorded |
| polynomial / rational tensor spline | `BSplineSurface`/`NurbsSurface<Vector4>` | unit-weight only into SSI | engine type accepts positive weights |
| lofted | `LoftOutput` `BSplineSurface<Vector4>` (`construct/loft.rs`) | as B-spline | certified validity (`loft_validity.rs`) |
| procedural evaluator | `SpineFrameSweep` via `enclosure_sweep.rs` + `num/sweep_sigma.rs` | σ_G bounds landed | restricted dispatch only |

### 8.3 Domain/pathology classes

| Class | Status |
|---|---|
| regular rectangular patch | LANDED (atlas charts, `rank_margin`) |
| simple trim | LANDED (`planar_slice`, `trimclip`) |
| multi-loop trim | LANDED with intersection semantics; no nesting logic |
| periodic seam | LANDED (deck carries, band routes, torus GL(2,Z); `DeckStep` breaks) |
| seam crossing | LANDED in formal layer (`torus_circle` exact seam-crossing counting; `unwrap_periodic_parameter` in shapeops); trimclip refuses un-recoverable in-leaf crossings |
| collapsed boundary / pole | PARTIAL: sphere pole charts + `ExactSpherePole` collapse witness (`domain/lattice.rs`); cone apex routes `CarrierSingular`; **loft collapse has no dedicated typed outcome**; trimclip cannot bound collapsed loops |
| singular parameterization | typed routes (`DegeneracyRoute::{SwitchChart, CarrierSingular}`); cone/torus chart *enclosures* pending |
| boundary-coincident contact | LANDED in funnel stage C0–C2 + `common_arc` support-identity; overlap.rs rotated-coplanar planes deliberately unscreened (booked BG-SOL-S7-OVERLAP-PLANE) |

---

## 9. Major transition paths (Phase B)

Legend per edge: `LANDED` / `PARTIAL` / `SPEC ONLY` / `ABSENT` / `UNCLEAR`.

### 9.1 STEP file → rendered mesh (the look production path)

```text
src/step.rs Table::from_owned_data_section (:87)          LANDED
  → truck-stepio ingestion (typed arenas, pcurves, provenance)  LANDED
  → compress (CompressedFace, no outer/inner distinction)       LANDED (representation gap noted)
  → look/src/step/lattice.rs lattice_of (:34-51)                LANDED
  → CertifiedLattice / support_schema_of                        LANDED
  → truck-meshalgo cshell_tessellation + MeshedShellOutcome     LANDED
  → formal routes: planar_slice / planar_holes / cylinder_band
    / cone_band / rank1_annulus / torus_cell / torus_realize    LANDED (holes route: 0 faces recovered today)
  → FaceTessellation / TessellationFailure (typed)              LANDED
  → look outcome counting → PNG                                 LANDED
```

### 9.2 Certified SSI (spline×analytic and spline×spline)

```text
public entry (none in production)                             ABSENT
  → admission patch_admit/ssi_admit (unit-weight, 1 span, plane) LANDED (narrow)
  → SsiAdmitSolver registration                                  PARTIAL (test-only call site)
  → ssi.rs F:R4→R3 per-side engine + krawczyk3                   LANDED
  → ssi_trace branch/loop/stitch                                 LANDED (test-only callers)
  → completeness (tier1/tier2, germ ladder)                      LANDED (kernel wave, no production consumer)
  → realization of the traced curve                              PARTIAL (fid/rep.rs sanctioned path, unwired)
```

### 9.3 Contact funnel → Boolean → solid

```text
facade boolean_op (truck-shapeops facade.rs)                  LANDED (naming layer)
  → assemble.rs boolean(a,op,b,budget)                           LANDED
  → recognize_surface lift                                       LANDED (canonical); swept via sweep_lift adapters PARTIAL (S6)
  → broadphase CFP-009 (exact pair set)                          LANDED
  → contact() funnel (analytic / gff / singular / fe_ee)         LANDED (typed deferred regions)
  → split_fragments (shared edge instances)                      LANDED
  → classify (seed + parity propagation)                         PARTIAL (seed rays f64; ray_cert standalone — S7)
  → decide + sew → Solid::try_new gate                           LANDED (single-shell; S8 re-entry gap)
  → homology gate (χ + Z₂ Betti)                                 LANDED (beside, not inside)
  → evidence preservation as contract                            PARTIAL (S10)
```

### 9.4 Constructive spine/frame → FAC mesh

```text
SpineFrameRecipe + frame laws                                  LANDED
  → ConstructError typed refusals                                LANDED
  → facet_sweep grid registry (exact shared indices)             LANDED
  → winding audit + signed volume + FacetVerdict                 LANDED
  → RealizationCertificate / shared_edge_pairs                   LANDED (types + facet fields)
  → EdgeSampleLedger integration (S9b)                           PARTIAL (beside-path evidence)
  → CG-009 parametric SpineFrameSurface enum ripple              LANDED (canonical.rs:346; honest Unrecognized witness)
```

### 9.5 Kernel SSI loop (trim/atlas/assemble/promote)

```text
engine krawczyk_c1/n3/n4 + tube4                               LANDED
  → atlas charts (plane/cyl/sphere; cone/torus structure-only)   PARTIAL
  → leaf extraction (NurbsSurface<Vector4> → leaves)             LANDED
  → tier1 loop-free cert / tier2 critical-point start set        LANDED
  → trimclip (R9 crossings, winding)                             LANDED (collapsed boundaries ABSENT)
  → assemble regions + deck_identify                             LANDED
  → identity Rules A/B/C                                         LANDED
  → promote → PromotedEdge (not live topology)                   LANDED (deliberate; topology ctors panic on self-loops)
  → production consumer of CertifiedGraph                        ABSENT
```

---

## 10. Source/spec discrepancies observed

1. `docs/BREP_SYSTEM.md:55` names output kinds `Empty/Body/BodySet/
   OpenShell/Assembly/Mesh`; no such enum exists (§7). The spec itself
   anticipates this ("do not propose implementing this tuple"), but the
   *capability vocabulary* is ahead of the code.
2. `docs/BREP_SYSTEM.md` links `audits/BREP_SYSTEM_GAPS.md`, which does not
   exist (only SOLVER/TOPOLOGY/BENCHMARK audits are present).
3. `docs/OP_CAPABILITY_MATRIX.md` cell 26 still annotates
   `step-export(constructive)` refusal — consistent with code; cells 1–6 G1
   flips are recorded as PB-011, partially landed (r1 cutaway; r2 monocoque
   in flight per `loop/STATE.md`).
4. `truck-shapeops/src/lib.rs:7-9` still describes the *legacy* transversal
   Boolean envelope ("supported only … transversally"), while the crate's
   actual certified boolean pipeline is `boolean/`. Doc drift.
5. The `CERTIFIED_WITHIN_TOLERANCE` doc tokens vs. the `RealizationVerdict`
   enum — cosmetic, but any grep-based audit must use the enum.
6. `docs/CERTIFICATE_MAPPING.md` placement correction (evidence types in
   `truck-base`, not meshalgo) matches the tree — no discrepancy found there.
7. The Phase-2 floor doc records a **0.0 certify-rate** for traced
   spline~spline unit-pairs with all completed pairs refusing — the doc and
   code agree; nothing overclaims SSI readiness.
