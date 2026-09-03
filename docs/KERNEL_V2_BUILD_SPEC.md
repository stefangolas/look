# Kernel v2 build spec — the spine document (session 50)

**Status: the committed build spec for the kernel-v2 swarm. It predates every
dispatch and is in the workers' base.** Normative theory:
[`CONSTRUCTIVE_GEOMETRY_KERNEL_SPEC_V2.md`](CONSTRUCTIVE_GEOMETRY_KERNEL_SPEC_V2.md)
("the v2 spec"; every predicate/certificate/type is a contract). Agenda source:
[`KERNEL_V2_SWARM_BOOKING.md`](KERNEL_V2_SWARM_BOOKING.md). This file completes
the skeleton: gap census (act 1), owner decisions (act 2), the shim contract
inventory with frozen spellings (act 3's packet source), the wave plan with
write-set pre-matrix (act 4), and the wave manifest (empty until waves close).

## 1. Gap census (measured, session 50, four parallel surveys)

Classifications against the tree, with the evidence file that carries them.
"Deltas" are the v2 new-work drivers (the spec's "Changes from v2" list made
concrete).

| Module | Class | Landed core (evidence) | Blocking v2 deltas |
|---|---|---|---|
| K0 numerics | PARTIAL | inari 2.0 no-GMP (`truck-evidence/src/enclosure.rs:20`), `CertifiedInterval`/`two_sum`/`Expansion` (`formal/exact.rs`), directed one-ULP steppers (`formal/deck.rs:188-245`), FMA policy documented (`enclosure.rs:14-18`), zero `par.sum()`, hash-order independence (`triangulation_with_ledger.rs:80-87`) | §0.4 constants absent (`rho_max`,`kappa_max`,`depth_max`,`k_a`,`deck_max`,`eps_rep`; `DirectTolerance::default()` is 1e-6 everywhere, normative is 1e-9/1e-11/1e-12); **N4 conflict: interval `sin/cos` ARE on certificate paths** (`elementary.rs` consumed by `torus.rs:22`, `cone.rs:63,117`) — the rational-reparameterization quarantine (§3.2) is new work; N5 homogeneous discipline not general; no kernel-init rounding config |
| K1 evidence algebra | PARTIAL | `Outcome<T>=Result<Certified<T>,Refusal>` + refusal vocab (`truck-base/src/evidence.rs:31-102`), `contract.rs::Refusal` (named-cases-only, deliberate no-widening note :64-67), Inconclusive≠Disproven scattered (`KrawczykIndeterminate` :111, `RealizationVerdict::Inconclusive`), D6 separation (`formal/evidence.rs:824-861`, `source_evidence.rs`) | `ClaimVerdict<T,E,R>` ZERO; `VerdictClass` ZERO (no backing field on any refusal); `Construction<T>` ZERO; `Refusal{kind,backing,evidence,partial}` ZERO; no residual identity type (rule 4 unimplementable) |
| K2 substrate | PARTIAL | `EnclosureSurface` trait + 10 implementors incl. Offset (`truck-evidence/src/enclosure.rs:175-195`), `immersion_lower_bound`, `DirCone`, Bernstein hull kernels (`hull.rs`), tensor-Bernstein SSI (`ssi.rs:294-342`), `RationalBipatch`, deck types (`formal/deck.rs`, `domain/lattice.rs`), `CoonsSurface`, `SpineFrameSurface`, `meshable.rs` | `CertifiedPatch`/`C2`/`C3` ZERO (no `second_derivs`/`third_jet`, no capability split); `weight_bound` ZERO (§7.1); `BezierLeaf` ZERO (no knot-insertion leaf, no OBB); no rational-carrier family (transcendental parameterization, N4 conflict); no `Param=(chart,deck,u,v)` lifted type (deck integers exist only as solver OUTPUT); no `DeckExhausted` |
| K3 identity | PARTIAL | §4.1/§4.4/§4.5 LANDED: `EdgeSampleLedger{edge,parameters,position_indices}`, once-per-edge sampling, `I(A,E)==reverse(I(B,E))` measured (`triangulation_with_ledger.rs`, `realization_evidence.rs:39-65`, tests `ledger_identity.rs`) | §4.2 Rules A/B/C ZERO (and **active contradiction**: `truck-shapeops/src/transversal/polyline_construction/mod.rs:57-70` welds node identity by `near_pt` tolerance — D2 violation recorded, its replacement is a booked seam, not this program's write set); §4.3 dyadic join ZERO (`SamplingPolicy::CustomParameters` currently legal on shared edges); ledger is v1-shaped (`edge: usize`, no dyadic addresses) |
| C1 recipe/spine | PARTIAL | `Frame3`, `FrameLaw` all four bodies, `ProfileLaw`/`ScalarLaw`, `SamplingPolicy`, `SpineFrameRecipe`, `ConstructError::{SpineNotC1,FrameSingular,ProfileCollapse,ProfileCorrespondenceMismatch}` (`truck-geometry/src/constructive/`) | `Spine` is a TRAIT (`recipe.rs:44`), spec wants the enum `Spine{Ph,General}` — `PhSpine`/`RrmfQuintic`/`RmErfSeptic`/`C1Curve` ZERO; exact rational RMF ZERO (`parallel_transport` is double-reflection over hardcoded 64 stations, `frame_transport.rs:25`); `FrameData` ZERO; `ChordTolerance`/`AngularTolerance` are refusing stubs |
| C2 facet realization | PARTIAL | grid-registry `facet_sweep` (`truck-modeling/src/facet_sweep.rs:85`), `winding_audit`, `signed_volume_of`, deterministic diagonal, caps, `FacetVerdict` | no closed/open DECLARATION (signature has no caller-supplied flag; caps always emitted; signed-volume always applied); winding failure returns the mesh with `Failed` beside it (booked CG-000 deviation) — spec wants Disproven; planarity is a twist-magnitude proxy, not certified |
| C3 edge ledger | PARTIAL | `EdgeSampleLedger`/`EdgeSampleLedgerSet`, `triangulation_with_ledger` parallel entry, **bit-identity of existing entries is tested** (`tests/ledger_identity.rs:153-248`) | `edge: usize` spelling (booked), no dyadic join, ledger/mesh index identity by test not by construction |
| C4 manifold diagnostics | **EXISTS** | `ManifoldDiagnostics` all six fields spec-named, link classification, `orientation_parity` (`truck-topology/src/manifold.rs:89-355`) | none material (per-vertex classification is a superset) |
| C5 Coons4 | PARTIAL | `CoonsSurface` corner-validated to `tol.position`, analytic derivs, `jacobian`=`S_u×S_v` (`decorators/coons.rs:30-197`) | no `CertifiedPatch` implementor (trait layer is shim); name `Coons4` absent (`CoonsSurface` is the landed spelling) |
| C6 SpineFrameSurface | PARTIAL | `Surface::SpineFrameSurface` enum variant landed (`canonical.rs:303`), decorator, `spine_sweep` B-rep constructor (`truck-modeling/src/spine_sweep.rs:52`) | struct is a per-profile-edge WINDOW decorator, not the spec's whole-sweep 4-field type; no `frame_data`; spine stored as `Box<Curve>` not the `Spine` enum; B-rep refusal **drops** the `ConstructError` detail (`spine_sweep.rs:291-293`) — typed-refusal-carrying doctrine gap |
| S0 product identity | PARTIAL | homogenized cross-multiplied identity `F_k=W2·N1_k−W1·N2_k` (`ssi.rs:30,917-961`), float maximal minors with exactly Theorem 6.4's sign pattern (`ssi_trace.rs:495-512`) | no rank dichotomy (Thm 6.1); no CERTIFIED enclosure of the 4-vector m (Thm 6.4(iii), 6.5); Corollaries 6.2/6.3 unimplemented |
| S1 residual family | MISSING | R1-analog: `SquareSystem3`+`krawczyk3_certificate` (transversal only); R9 prior art: 2D `formal/bezier_isect.rs`; R3-shape precursor: `contact/singular.rs` Lagrange system | `ResidualId`/R1–R9 ZERO; R2/R4/R4′/R5/R6/R7/R8 ZERO as residuals; §7.1 `CertifiedPositive` weight bound as VALUE argument ZERO (`RationalBipatch::new` checks weights once as floats, `ssi.rs:863-865`) |
| S2 certificate calculus | PARTIAL | `KrawczykCertificate3` strict-inclusion-only (`ssi_types.rs:169-234`), generic `KrawczykSystem` (`num/krawczyk.rs`), `CertifiedSign`, `IntervalEnclosure` | `Frame<N>` ZERO (F3 froze SQUARE 3×3 only — adopting §8.3's tube is a **contract amendment**, recorded); C2-as-tube ZERO (landed slices τ to a point, `ssi.rs:649,719`); ρ never computed/stored (Lemma 8.0); `GraphCert`/`R5Enclosure`/Δ_off ZERO |
| S3 completeness | MISSING | precursors only: `DirCone` per surface, exclusion/counting machinery (`formal/envelope.rs`), deck winding (`torus_circle.rs:490-580`) | Tier-1 two-cone LP ZERO; Tier-2 Ψ_a ZERO; R8 boundary strata ZERO; trim clip (R9 crossings + winding) ZERO |
| S4 tracer | PARTIAL | `certified_pair_trace` end to end (`ssi_trace.rs:813-832`), width ladder, F3 `CoordinateSwitch`, hull rejection | fixed `ARC_STEP` (no adaptive dtau growth); no escalation ladder (§10.2); no arc representation (TraceStep boxes, not Hermite+tube); no uncertified-fast-path split |
| S5 contact | PARTIAL | **recognized-carrier route substantially landed**: `pair_dispatch.rs` closed-form loci incl. tangency (plane/cylinder/sphere pairs :602-844), analytic-class twins in `truck-evidence/src/analytic/`; restricted-Hessian inertia test (`contact/singular.rs:71-94`); `CertifiedSign`; 2D jet-parity (`formal/contact.rs:146-159`, `formal/span.rs`) | `ContactCert` (tolerance-tagged, three-valued) ZERO; `SignCert`-on-Hessian classifier ZERO; `CertifiedPatchC2/C3` jets ZERO; `Refuse(TangentialCurve)` for the generic path exists only as `UnrelatedTangency`/`UnresolvedTangencyOrSingularity` refusals without the §10.4 disposition split |
| S6 ExactSheet | MISSING | 1-D overlap screens only (`contact/overlap.rs:43,54`) | everything: `SheetCert`, ψ arms, normal-dot sign, `NearOverlap` |
| S7 fillets/canals | MISSING | legacy approx fillets (`truck-shapeops/src/fillet/`, `rewrite/`), offset ENCLOSURE (`decorators/offset.rs`) — different axis | everything: R7, `Canal` (no-orthogonality-field rule), Δ_off, three-face corner, `CornerUnsolved` |
| S8 self-intersection | MISSING (substrate PARTIAL) | `SquareSystem3`/`KrawczykCertificate3`/trace loop/`BranchGerm`/diagonal-lift fixtures all landed | deflation (divided differences), Chart A/B, exact-cover transitions (`R6ChartSwitch`/`R6BaseSwap`), λ=0 routing |
| S9 assembly/promotion | PARTIAL | **deck machinery strong**: certified deck solver/labels/lattice/winding (`formal/deck.rs`, `quotient.rs`, `domain/lattice.rs`, `torus_circle.rs`); relative-deck on arcs (`common_arc.rs:806`) | node identity is Euclidean welding (above); `TubeOverlapCert` ZERO; gluing/promotion/`SliverOrNearOverlap` ZERO; `deck_max` named bound ZERO |
| S10 claims | MISSING | three-valued verdicts scattered; provenance types (`CurveOccurrenceProvenance`, `ProvenanceSet`) | `TopologyClaim`/`certify_claimed`/`ClaimedGraph`/`ClaimRefutation` ZERO |
| §16 types | MISSING as a set | `HermiteCurve` (`fid/rep.rs:142`) is the Approx precursor; `SpineFrameSurface`/`EdgeSampleLedger` exist (CG program) | the whole certificate/graph type layer |
| §17 taxonomy | MISSING as enum | 5 constructive variants exist as `ConstructError`; `Conditioning` fully realized; ~6 PARTIAL-adjacent | the unified `RefusalKind` enum with backing classes |

**Census conclusion.** The wave plan in booking §3 stands, with one
re-scoping: Wave 1 is thinner than booked (the shim absorbs the K1 types and
the K2/K3 type shapes), and Wave 2's C1-delta packet carries a new owner seam
(the `Spine` trait/enum name collision, §4 below).

## 2. Owner decisions (taken, session 50, with machine facts)

1. **Crate placement: extend `truck-certified` with a `kernel` module.**
   Measured: truck-certified already depends on truck-geometry/topology/
   polymesh/base/geotrait (Cargo.toml), already holds the certified substrate
   (hull/exact/contract/ssi), and its lib.rs carries the crate-wide
   `deny(clippy::unwrap_used)` header the H-1 gate grades against. A new crate
   would add manifest edges, duplicate proc-macro compilation on the 15.7 GB
   machine, and break V5's baseline package identity. The constructive-half
   deltas (C1/C2/C6) land in their existing homes (truck-geometry /
   truck-modeling) in later waves, unchanged.
2. **Reuse-vs-retype: additive retype-adjacent only; landed entry points stay
   bit-identical.** The V5 identity guard is the enforcement. v2 types land
   ADDITIVELY (new `kernel` module; new files); landed CG code
   (`constructive/`, `facet_sweep`, `triangulation_with_ledger`,
   `manifold.rs`, `CoonsSurface`, `spine_sweep`) is consumed via wrappers, not
   rewritten. The two recorded landed deviations (facet winding returns mesh +
   `Failed` verdict; `DirectTolerance::default()` = 1e-6) are NOT silently
   "fixed" — the v2 layers sit beside them, and their migration is booked as
   named amendments with their own verify cycles.
3. **Census resurrection: YES, parallel to the shim.** The spline-bucket
   census WIP exists (`loop/slots/1/abandoned-20260902-142536.patch`, verified).
   It quantifies representation-recovery mass and re-scores the Phase-2 funnel
   — exactly the input Wave-1's census-driven fill-in needs. It runs as an
   elastic-pool measurement packet (`BG-KV2-CENSUS`, survey-adjacent,
   measurement only) against the wave base; it blocks nothing.
4. **N4 second architecture: x86_64-unknown-linux-gnu on CI (gcc/glibc).**
   The gnullvm host and a glibc CI runner are maximally-divergent libm
   candidates among available machines, which is what the N4 gate is for. The
   per-module enclosure fixtures run under a `kv2_n4` test target in
   `cross-platform.yml`; macOS ARM64 stays release-matrix smoke. This is a
   CI/machine decision; recorded here so the fixtures are authored
   bit-reproducibility-first from Wave 1 onward (no float-lattice ambiguity,
   pinned evaluation order in every fixture's ground truth).
5. **Wave sizing: 4 concurrent workers on disjoint write sets.** Booking §6's
   build-sharing policy (per-agent worktrees, check-only local gates, ONE
   shared `CARGO_TARGET_DIR`, orchestrator prewarms) removes the
   duplicate-target cost that capped session 49 at 3; the remaining per-worker
   cost is one cargo process. Rules unchanged: `CARGO_BUILD_JOBS=2-4` every
   invocation; ONE worker at a time for COLD warm builds; sccache
   `RUSTC_WRAPPER` for workers, unset locally where it rejects incremental.
6. **Elastic pool: the two batteries are separate measurement packets by
   law.** `BG-KV2-BAT-TRANSVERSAL` and `BG-KV2-BAT-TANGENCY` are separate
   corpora, separate packets, separate published numbers — never one
   aggregate (§18/§20). They dispatch into idle slots from Wave 2 onward;
   their fixtures grow the shim kit rather than forking private ones.

## 3. The shim contract inventory → `BG-KV2-000-CONTRACT`

One shim packet through the NORMAL loop (BG-CK-P2-CONTRACT pattern: frozen
types + refusing constructors + fixture kit with machine-checked ground
truths; no solver bodies; no trait implementors). **Write set: NEW files under
`vendor/truck/truck-certified/src/kernel/` + one `pub mod kernel;` line in
that crate's lib.rs. Zero cross-crate ripple (truck-certified already depends
on everything the types name). Its landing merge SHA is the wave base.**

Module layout and frozen spellings (collision-checked against the census):

- `kernel/mod.rs` — `pub mod` wiring + crate-root `pub use` of ONLY the
  booking-listed types that must be reachable unqualified by wave workers
  (`ClaimVerdict`, `Construction`, `Refusal`→ re-exported as `KernelRefusal`
  at crate root to avoid colliding with `contract::Refusal` and
  `truck_base::evidence::Refusal`; inside `kernel` the spec spelling `Refusal`
  is used verbatim). Plus `config` (below).
- `kernel/config.rs` — §0.4 constants: `EPS_REP=1e-9`, `RHO_MAX=0.5`,
  `KAPPA_MAX=1e6`, `DEPTH_MAX=40u32`, `KA=4u32`, `DECK_MAX=8i32`,
  `TOL_POSITION=1e-9`, `TOL_PARAMETER=1e-11`, `TOL_JACOBIAN=1e-12`,
  `TOL_INTERSECTION=EPS_REP`. Landed `DirectTolerance` untouched (decision 2).
- `kernel/evidence.rs` — §2 verbatim: `ClaimVerdict<T,E,R>{Proven,Disproven,
  Inconclusive}`, `Construction<T>`, `VerdictClass{Disproven,Inconclusive}`,
  `Refusal{kind,backing,evidence,partial}`, `RefusalKind` (all 25 §17
  variants), `RefusalEvidence{Residual{..},Predicate{name,detail},None}`,
  `default_backing(RefusalKind)` implementing §17's per-variant backing
  (WeightDegenerate defaults Disproven; constructor accepts an override).
- `kernel/residual.rs` — `ResidualId{R1..R9, Carrier}` and the §4.2 Rule C
  implication relation as a typed fn `implication(stronger,weaker) ->
  Implication{Equivalent,Stronger,None}` admitting exactly: identity,
  R2⊒R1, R6↔R6 (chart variants are one id). R8/R9/R7 imply nothing.
- `kernel/patch.rs` — §3.1 verbatim: `CertifiedPatch{enclose(IBox2)->IBox3,
  derivs->DerivativeEnclosure, normal_cone->Cone, regularity->
  ClaimVerdict<CertifiedPositive,Degeneracy,Reason>, weight_bound->
  Option<ClaimVerdict<CertifiedPositive,Pole,Reason>>}`, `CertifiedPatchC2
  (second_derivs)`, `CertifiedPatchC3 (third_jet)`, plus `Cone{axis,
  half_angle}`, `DerivativeEnclosure`, `SecondDerivativeEnclosure`,
  `ThirdJetEnclosure`, `Degeneracy{box_, egf2:(f64,f64)}`,
  `Pole{box_, w:(f64,f64)}`, `type Reason = &'static str`.
- `kernel/leaf.rs` — §3.2 type shapes (no extraction logic): `BezierLeaf
  {degree_u,degree_v,control:Vec<[f64;4]>}` (homogeneous xyzw per N5;
  `try_new` refuses non-finite or non-positive control weights),
  `RationalCarrier` enum over `RationalCarrierKind{Plane,Sphere,Cylinder,
  Cone,Torus}` with rational half-angle parameterization data fields,
  `try_new` refusing the transcendental case is NOT needed (the family is
  closed; `RefusalKind::TranscendentalCarrier` is constructible for callers).
- `kernel/certs.rs` — `IBox<const N:{lo:[f64;N],hi:[f64;N]}>` (try_new
  refuses lo>hi or non-finite), `CertifiedPositive`/`CertifiedNonzero`
  (refusing constructors), `type Interval = CertifiedInterval` (the landed
  crate primitive — zero new dep edges), `Frame<const N>` (fields per §8.1;
  `try_new` refuses non-orthonormal Q / non-unit q_tau at TOL_JACOBIAN),
  `PointCert{residual,box_,rho}` (try_new refuses rho>RHO_MAX),
  `ArcCert<const N>` (fields per §16 incl. `rho` — try_new refuses
  rho>RHO_MAX — and `weights:Option<Vec<CertifiedPositive>>`),
  `ContactCert{critical_point:PointCert,gap:Interval,tolerance,
  hessian_sign:SignCert}` (try_new refuses 0∉gap or width(gap)>tolerance —
  the Proven case only; Disproven/Inconclusive contact outcomes are
  ClaimVerdict arms, S5a owns them), `type SignCert = CertifiedSign`
  (reused from `formal::exact`), `GraphCert{domain:IBox2,n0:[f64;3],
  det_bound:CertifiedNonzero}`, `R5Enclosure{q,preimage:[IBox2;2],
  cert:[PointCert;2]}`, `SheetCert{domain,psi_kind:PsiMapKind,
  det_dpsi:CertifiedNonzero}`, `TubeOverlapCert{shared_point:[f64;3],
  c1_bound:f64}` (c1_bound ≤ EPS_REP enforced).
- `kernel/graph.rs` — `ChartId(u32)`, `Param{chart,deck:i32,u,v}` (try_new
  refuses non-finite), `Point4{p1,p2}`, `NodeId/BreakId/ArcId(usize)`,
  `TopoNode{Boundary,TrimCrossing,MorseSaddle,MorseExtremum,A2Cusp,
  OverlapBoundary,FilletEnd}`, `SegmentBreak{ChartSwitch,FrameSwitch,
  LeafBoundary,DeckStep,R6ChartSwitch,R6BaseSwap}` (Refuse appears in
  NEITHER — audit), `ArcEnd{Topo(NodeId),Seg(BreakId)}`, `NodeCert{Exact(
  PointCert),AtTolerance(ContactCert)}`, `Node{id,at,kind,cert}`,
  `Break{id,at,kind,overlap:TubeOverlapCert}`, `HermiteSpline` (segments of
  `{p0,p1,t0,t1:[f64;3]}`; refusing non-finite), `Approx{gamma:
  HermiteSpline}`, `Arc<const N>{id,approx,cert:ArcCert<N>,ends:
  (ArcEnd,ArcEnd)}` (shadows std Arc module-locally — consumers qualify
  std::sync::Arc; recorded), `CarrierArc{id,carrier:RationalCarrierKind,
  approx}`, `AnyArc{Ordinary(Arc<4>),Difference(Arc<2>),SelfInt(Arc<4>),
  Spine(Arc<7>),Carrier(CarrierArc)}`, `Sheet{domain,psi_kind,cert,
  boundary:Vec<ArcId>}`, `Provenance{Claimed,Imported,Client}` (NOT a
  certificate; D6), `CertifiedGraph{nodes,breaks,arcs,sheets,exhaustive}`,
  `ClaimedGraph{graph,provenance}`, `PartialGraph{graph,frontier:Vec<Point4>}`.
- `kernel/fixtures.rs` — the fixture kit, each with a machine-checked test:
  (1) unit-sphere/plane transversal pair (ground truth: intersection circle
  r=1 at z=0, asserted by sampling both implicit equations); (2) equal-radius
  coaxial cylinders (ground truth: ExactSheet candidate, ψ=identity, normal
  dot +1); (3) determinant-spans-zero box (F=(x²−1,y) on [−2,2]²: det DF=2x
  spans zero — ground truth: the verdict backing is Inconclusive);
  (4) weight-straddles-zero rational quad, control weights (1,−1,1) (ground
  truth: w(0.5)=0 exactly; interval w-enclosure over [0,1] contains 0 →
  WeightDegenerate, Disproven backing); (5) deck-wrap cylinder (pcurve runs
  5.9→6.4 — ground truth: deck displacement +1, canonical unwrap 0.1166…);
  (6) C¹-discontinuity polyline spine (positions [(0,0,0),(1,0,0),(1,1,0)] —
  ground truth: tangent jump 90° > TOL_PARAMETER; carried as DATA for the
  C1 wave packet, which owns the refusal wiring).
- **H-1 gate**: every new `kernel/*.rs` file carries the crate's deny header
  pattern (copy from `hull.rs`); no unwraps anywhere; no bare absolute float
  literals in predicate code (H-3) — fixture ground truths use named
  constants or `// H-3` same-line opt-outs per kernel-gates.sh.

Deviations from the spec's verbatim §16 spelling, all recorded here so no
worker relitigates: `Refusal` re-exported as `KernelRefusal` at crate root;
`PsiMap` frozen as `PsiMapKind` enum {Identity, Affine, Bilinear,
RecognizedCarrier} pending S6's real map type; `Interval` = the landed
`CertifiedInterval`; `SignCert` = the landed `CertifiedSign`.

**§22 mapping table** — the booking surface lives in §6 below with a status
column; every later packet adds its row(s) before dispatch (packet_lint class:
CRATES_NONEMPTY-style discipline, spine-mandated).

## 4. Wave plan (over §19's 25 rows) + write-set pre-matrix

Shim absorbs: K1 types (row 2), K2/K3 type shapes (rows 3–4 partially), the
§16/§17 type layer. Waves below are implementation, not re-typing.

**Wave 1 (parallel, 4 workers) — contracts and substrate fill-in:**

| Packet | §19 row(s) | Write set (disjoint) | Summary |
|---|---|---|---|
| BG-KV2-101-K0AUDIT | 1 | NO code writes (survey class) — SURVEY.json | N1–N7 audit over certificate paths: transcendental-call inventory (`elementary.rs` consumers), par.sum/hash-order sweep, two-stage N7 sites; proposes the rational-reparameterization migration list for §3.2 |
| BG-KV2-102-LEAF | 3 | `truck-certified/src/kernel/leaf.rs` (extends shim file) + `kernel/leaf_extract.rs` (NEW) | BezierLeaf for real: knot-insertion/Bézier-span extraction from landed `Curve`/`Surface` spans; derivative nets; AABB; implements `CertifiedPatch`(+C2) |
| BG-KV2-103-IDENTITY | 4 | `truck-certified/src/kernel/identity.rs` (NEW) | §4.2 Rules A/B/C over shim types; §4.3 dyadic join (Theorem 4.1: prefix-closed address sets, integer join); `NonDyadicSharedRequest` wiring point |
| BG-KV2-104-RATCARRIER | 3/19 | `truck-certified/src/kernel/rational.rs` (NEW) | RationalCarrier half-angle parameterizations with interval enclosures (NO transcendental on any enclosure path — N4), implements `CertifiedPatch` for plane/sphere/cylinder |

`pub mod` lines: 102/103/104 each add ONE line to `kernel/mod.rs` (expected
textual conflict, resolved at integration). 102 and 104 both touch
`kernel/leaf.rs`? NO — 104's impl block lives in its own `rational.rs`;
`leaf.rs` is 102-only. Matrix: all four pairwise DISJOINT except the mod.rs
line. `BG-KV2-CENSUS` (decision 3) runs parallel, write set = measurement
scripts + docs only.

**Wave 2 (parallel) — the engine core:** C1 deltas (Spine enum + FrameData —
carries the `Spine` trait/enum name seam: the enum lands in
`truck-geometry/src/constructive/` beside the trait, spec spelling at module
level, trait renamed `SpineCurve` with a one-session deprecation alias; owner
seam RESOLVED here), S4a float tracer, S2a (Lemma 8.0 ρ-extraction + C1/C2
tube as the recorded F3 contract AMENDMENT), S1a (R8/R9), C4/C5 deltas
(CoonsSurface→CertifiedPatch implementor in its own file). Write-set matrix
authored at Wave-2 dispatch against the post-Wave-1 tree.

**Wave 3 (parallel):** S0/S3a (maximal-minor + Tier-1 LP), S5a (ContactCert),
S9a (node identity + deck identification gluing), S3b (Tier-2 start set),
S2b (GraphCert/R5 contract).

**Wave 4 (parallel):** S3c trim clip, S7 (R7/canal), S6 (ExactSheet), S8 (R6
charts), K2b (atlas/pole charts).

**Wave 5 (serial, integrator-owned):** C6 enum-boundary consolidation +
S9b promotion + S10 verification.

**Frozen-signature rule (session-49 lesson, applies at every seam):** any
function a later wave calls has its signature pinned in BOTH the producing
and the consuming packet text before either dispatches — including
amendment-time seams. Known seams to pin at Wave-2 authoring: S2a's
`c1_certify`/`c2_certify_tube` entry shapes (§7.1's weight-bound value
argument), S1a's R8/R9 residual constructors, S4a's escalation-ladder call
into S2a.

## 5. Integration order and verification plan (OWNER AMENDMENT, session 50)

**ONE full verification for the entire build spec — at the END.** Owner
direction supersedes the booking's per-wave battery: no composed-HEAD
verification battery runs after Waves 1–4; the loop's ordinary verifier runs
ONCE against the final integrated HEAD (after Wave 5), and only then do rows
flip DONE. Registry rows stay RUNNING (wave state in the note) for the whole
program; LOCAL_GREEN is never DONE.

Between merges (within and across waves), the composition discipline is
cheap and continuous:

- `cargo check -p <affected-crate>` after every merge (compile-level seam
  detection, minutes).
- The session-37 scoped rule: where two merged write sets interact
  semantically, run the affected crates' `--lib` tests at merged HEAD
  before authoring the next packet against it — targeted tests, not the
  verifier.
- The shim keeps its one NORMAL-loop verify (it is the wave-base landing;
  verify.py is the only acceptance authority). This is the packet's own
  gate, not a per-wave battery.

Risk, stated plainly: deferring test-level verification to the end means a
final-battery failure attributes across five waves. The mitigation is the
continuous fast checks above and the frozen-contract rule (§4) — seam
mismatches surface at `cargo check` time, attribution rides the write-set
matrix.
- **kernel-gates.sh additions (the §20 cross-cutting audits, grep-class):**
  no transcendental call inside `kernel/` (sin/cos/atan2/log/exp in
  `truck-certified/src/kernel/**`); no `par.sum()`; no `dist < eps`-identity
  pattern in `kernel/`; no `TopoNode` variant named Refuse/ChartSwitch/
  FrameSwitch/DeckStep (structural — enforced by the shim, grep guards
  regressions); sampling join integer-only (no float compare in the join
  path); R2 never reaches C2 (no `R2`-typed argument to tube-certify entry);
  ClaimedGraph never reaches a CertifiedGraph consumer (no fn taking
  `&CertifiedGraph` called with a `ClaimedGraph` — type-level, grep guards
  `impl From<ClaimedGraph>`); Canal has no orthogonality-certificate field.
  Added during Wave-1 integration, watched failing once before trusted.
- N4: per-module enclosure fixtures run on the gnullvm host AND the Linux CI
  job (decision 4) from Wave 2 onward.
- V8/V9 skip-justifications recorded once at the final verification if
  additive-only across the whole program.

## 6. §22 mapping table — the booking surface (status column maintained here)

| Certificate / verdict | Type | Produced by | Status |
|---|---|---|---|
| ClaimVerdict / Construction / Refusal | algebra | K1 | **shim** |
| ResidualId + implication order | typed relation | K1/S1 | **shim** |
| regularity / weight bound / normal cone | CertifiedPositive / Cone | §3.1 trait | **shim (trait)**; impls Wave 1 |
| PointCert / ArcCert<N> (ρ inside) | exact | C1/C2 §8.2–8.3 | **shim (type)**; production Wave 2 (S2a) |
| GraphCert / R5Enclosure | exact | §8.5–8.6 | **shim (type)**; production Wave 3 (S2b) |
| ContactCert (tolerance-tagged) | three-valued | §10.3 | **shim (type)**; production Wave 3 (S5a) |
| SheetCert / TubeOverlapCert | exact | §11 / §14.2 | **shim (type)**; production Waves 4 / 3 |
| Δ_off | exact diagnostic | §8.7 | Wave 4 (S7) |
| winding audit / signed volume | ClaimVerdict | §5.6 | landed (facet_sweep); declaration amendment booked |
| ManifoldDiagnostics | ClaimVerdict | §5.8 | **landed** |
| ClaimRefutation | exact | §15 | Wave 5 (S10) |
| Provenance | not a certificate | client | **shim** |
| RefusalKind (25 variants) | taxonomy | §17 | **shim** |
| config constants §0.4 | consts | K0 | **shim** |

## 7. Wave manifest (filled ONCE at the final integration)

| Wave | Base SHA | Packets (ID → commit) | Amendments | Fast checks | Final integrated SHA |
|---|---|---|---|---|---|
| — | — | — | — | — | — |

## 8. Machine facts (inherited, binding)

`CARGO_BUILD_JOBS=2–4` every cargo invocation; ONE worker at a time for COLD
warm builds; shared `CARGO_TARGET_DIR` prewarmed by the orchestrator at each
wave base; `RUSTC_WRAPPER=sccache` for workers (unset locally where it
rejects incremental); reclaim idle slot targets before any verify (floor
~15 GB — 14.6 GB free at spine time: reclaim repo-root `target/` FIRST);
watchdog running (`LOOK_WATCHDOG_STAGNANT=3600`); amendments return to
owning sessions via `--resume`; LOCAL_GREEN is never DONE.
