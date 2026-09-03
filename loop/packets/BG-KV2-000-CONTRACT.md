# BG-KV2-000-CONTRACT — the kernel-v2 shim: shared types + refusing constructors + fixture kit

The pre-wave contract packet for the kernel-v2 swarm (ORCHESTRATOR wave mode,
"build-spec spine workflow"; build spec: `docs/KERNEL_V2_BUILD_SPEC.md` §3 —
the authoritative inventory this packet materializes; normative theory:
`docs/CONSTRUCTIVE_GEOMETRY_KERNEL_SPEC_V2.md` — every predicate/certificate/
type below is a contract from §2, §3.1–3.3, §7, §8, §10.3, §14, §16, §17,
§0.4). Its ONLY job: land the shared shapes the wave packets exchange as
verified code so the wave branches fork from one base and build against
frozen contracts instead of each other. **No solver bodies, no trait
implementors**: every numeric method refuses; the types, the refusal
vocabulary, the config constants, and the machine-checked fixture kit are the
deliverable. The BG-CK-P2-CONTRACT pattern exactly.

Reuse over redefinition (build-spec decision 2): `CertifiedInterval`,
`CertifiedSign` are LANDED in `formal/exact.rs` — alias/import, never
restate. The landed refusal vocabularies (`truck_base::evidence::Refusal`,
`contract::Refusal`) are untouched; the v2 vocabulary is additive in a NEW
module. Landed CG code is not modified at all.

```yaml
id:          BG-KV2-000-CONTRACT
contract:    [BG-KV2-000-CONTRACT]
class:       design
crates:      [truck-certified]
depends_on:  [BG-CK-P2-RESIDUAL]
write_allow:
  - vendor/truck/truck-certified/src/kernel/mod.rs
  - vendor/truck/truck-certified/src/kernel/config.rs
  - vendor/truck/truck-certified/src/kernel/evidence.rs
  - vendor/truck/truck-certified/src/kernel/residual.rs
  - vendor/truck/truck-certified/src/kernel/patch.rs
  - vendor/truck/truck-certified/src/kernel/leaf.rs
  - vendor/truck/truck-certified/src/kernel/certs.rs
  - vendor/truck/truck-certified/src/kernel/graph.rs
  - vendor/truck/truck-certified/src/kernel/fixtures.rs
  - vendor/truck/truck-certified/src/lib.rs
  - vendor/truck/truck-certified/tests/kernel_contract.rs
read_allow:
  - docs/CONSTRUCTIVE_GEOMETRY_KERNEL_SPEC_V2.md
  - docs/KERNEL_V2_BUILD_SPEC.md
  - scripts/kernel-gates.sh
  - vendor/truck/truck-certified/src/lib.rs
  - vendor/truck/truck-certified/src/formal/exact.rs
  - vendor/truck/truck-certified/src/hull.rs
  - vendor/truck/truck-certified/src/contract.rs
budget:      {turns: 40, ctx_tokens: 120000}
anchors:
  - {id: A1, expect: 0, cmd: "grep -c 'pub mod kernel;' vendor/truck/truck-certified/src/lib.rs"}
  - {id: A2, expect: 0, cmd: "grep -rn 'ClaimVerdict' vendor/truck/truck-certified/src | wc -l"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub struct CertifiedInterval' vendor/truck/truck-certified/src/formal/exact.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub enum CertifiedSign' vendor/truck/truck-certified/src/formal/exact.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'deny(clippy::unwrap_used)' vendor/truck/truck-certified/src/lib.rs"}
  - {id: A6, expect: 12, cmd: "grep -c '^pub mod ' vendor/truck/truck-certified/src/lib.rs"}
  - {id: A7, expect: 0, cmd: "grep -c 'inari' vendor/truck/truck-certified/Cargo.toml"}
  - {id: A8, expect: 0, cmd: "grep -rnw 'ResidualId' vendor/truck/truck-certified/src | wc -l"}
tests_required:
  - kernel_config_constants_match_spec_defaults
  - refusal_kind_has_all_spec_variants
  - refusal_backing_class_matches_spec
  - residual_implication_order_is_exactly_rule_c
  - certified_positive_nonzero_refuse_bad_bounds
  - frame_refuses_non_orthonormal_basis
  - point_and_arc_cert_refuse_rho_above_max
  - contact_cert_requires_gap_at_tolerance
  - ibox_and_param_refuse_inverted_or_nonfinite
  - topological_node_enums_have_no_refuse_variant
  - fixture_transversal_sphere_plane_ground_truth
  - fixture_coaxial_cylinders_sheet_ground_truth
  - fixture_determinant_spans_zero_is_inconclusive_backed
  - fixture_weight_straddles_zero_is_weight_degenerate
  - fixture_deck_wrap_displacement_is_one
  - fixture_c1_discontinuity_tangent_jump_exceeds_tolerance
```

## Pre-made decisions (do not relitigate; quote the tags into module docs)

**H-1.** Crate-level `#![deny(clippy::unwrap_used)]` covers the new module.
No `unwrap`/`expect`/`panic!`/`unreachable!` in any new file, no module-level
`allow`. Copy the deny/attr header style from `hull.rs` (NOT the
`exact.rs` grandfathered header — that allow is inherited-baseline-only).

**H-doc.** `lib.rs` warns `missing_docs` + `missing_debug_implementations`,
and `#![cfg_attr(not(debug_assertions), deny(warnings))]` turns warnings into
release-build failures: EVERY public item in the new module carries a doc
comment and `#[derive(Debug)]` (or a manual impl). No exceptions.

**D-shim.** Types and refusing constructors only. Any method that would
evaluate, solve, isolate, or certify NUMERICALLY refuses with a named
`RefusalKind` (or returns `RefusalKind`-carrying data for later use). The
`kernel/mod.rs` doc says verbatim: "This module freezes the kernel-v2 shapes;
the wave packets (BG-KV2-1xx/2xx/3xx/4xx) implement against it and never
restate it."

**D-reuse.** `pub type Interval = crate::formal::exact::CertifiedInterval;`
and `pub type SignCert = crate::formal::exact::CertifiedSign;` — aliases to
the landed primitives, zero new manifest edges (A7 confirms no inari dep; do
not add one). The landed refusal enums are NOT widened and NOT re-exported
through `kernel`.

**D-spelling.** The spec's §16 spellings are used INSIDE `kernel` (`Refusal`,
`Arc`, `Sheet`, `Node`, ...). Known collisions, handled exactly thus:
crate-root re-export is `pub use kernel::evidence::Refusal as KernelRefusal;`
only (avoids `contract::Refusal` / base `Refusal` ambiguity at the crate
root); `kernel::graph::Arc<const N>` shadows `std::sync::Arc` module-locally
— acceptable, noted in the module doc; `Frame<const N>` does not collide
(`Frame3` lives in truck-geometry, different crate).

**D-constants.** `config.rs` holds the §0.4 normative defaults as consts.
Landed `DirectTolerance` (truck-geometry) is NOT touched (build-spec
decision 2) — recorded deviation: the two default sources coexist this
program; v2 code consumes only `kernel::config`.

**D-fixtures-public.** `fixtures.rs` is `#[doc(hidden)] pub` (the
BG-CK-P2-CONTRACT rule): test support only, excluded from the certified API
surface in the module doc, but reachable by wave workers' integration tests
through the crate's public path.

## Section 1 — `kernel/config.rs` (NEW)

§0.4 verbatim, each const doc-commented with its spec meaning:

```rust
pub const EPS_REP: f64 = 1e-9;          // model-space representation gap
pub const RHO_MAX: f64 = 0.5;           // Krawczyk contraction acceptance
pub const KAPPA_MAX: f64 = 1e6;         // conditioning bound -> frame rebuild
pub const DEPTH_MAX: u32 = 40;          // subdivision cap (3 D4 carve-out sites)
pub const KA: u32 = 4;                  // Tier-2 direction retries
pub const DECK_MAX: i32 = 8;            // max deck traversals per edge
pub const TOL_POSITION: f64 = 1e-9;     // model-space agreement
pub const TOL_PARAMETER: f64 = 1e-11;   // parameter agreement, C1 detection
pub const TOL_JACOBIAN: f64 = 1e-12;    // regularity floor EG - F^2
pub const TOL_INTERSECTION: f64 = EPS_REP; // tangency-claim tag (§10.3)
```

## Section 2 — `kernel/evidence.rs` (NEW) — §2 algebra + §17 taxonomy

```rust
pub enum ClaimVerdict<T, E, R> { Proven(T), Disproven(E), Inconclusive(R) }
pub type Construction<T> = Result<T, Refusal>;
pub enum VerdictClass { Disproven, Inconclusive }

pub struct Refusal {
    pub kind: RefusalKind,
    pub backing: VerdictClass,
    pub evidence: RefusalEvidence,
    pub partial: Option<PartialGraph>,
}

pub enum RefusalEvidence {
    Residual { residual: ResidualId, box_: IBox, note: &'static str },
    Predicate { name: &'static str, detail: String },
    None,
}
```

`RefusalKind`: ALL 25 §17 variants, doc-commented with their backing class
verbatim from the spec table (SpineNotC1, FrameSingular, ProfileCollapse,
ProfileCorrespondenceMismatch, NonFinite, WindingAuditFailed,
NonDyadicSharedRequest, CarrierSingularity, ChartExhausted,
TranscendentalCarrier, WeightDegenerate, DeckExhausted, Conditioning,
TangentialCurve, HighOrderJet, IncompleteStartSet, R5EnclosureFailed,
TrimClipFailed, NearOverlap, OffsetDegenerate, OffsetSwallowtail,
CornerUnsolved, SliverOrNearOverlap, ClaimRefuted, Budget).
`pub fn default_backing(kind: RefusalKind) -> VerdictClass` implements the
per-variant classes exactly (WeightDegenerate -> Disproven; DeckExhausted,
Conditioning, TangentialCurve, HighOrderJet, IncompleteStartSet,
R5EnclosureFailed, TrimClipFailed, CornerUnsolved, SliverOrNearOverlap,
Budget -> Inconclusive; NearOverlap -> Disproven (of ExactSheet); the
constructive/carrier set -> Disproven). Constructors: `Refusal::new(kind,
evidence)` using `default_backing`, `Refusal::with_backing(kind, backing,
evidence)` for the WeightDegenerate Disproven-or-Inconclusive split (§7.1).
Rules 2/4/6 of §2 are enforced by shape: Inconclusive is a variant, evidence
names a residual, accepted objects carry no refusal (`PartialGraph` only
ever appears inside `Refusal.partial`).

## Section 3 — `kernel/residual.rs` (NEW) — §7 family + §4.2 Rule C

```rust
pub enum ResidualId { R1, R2, R3, R4, R4Prime, R5, R6, R7, R8, R9, Carrier }
pub enum Implication { Equivalent, Stronger, None }
pub fn implication(stronger: ResidualId, weaker: ResidualId) -> Implication
```

`implication` admits EXACTLY: identity (`Equivalent`), `R2 ⊒ R1`
(`Stronger`), `R6 ⊒ R6` via identity (the A/B chart variants are one id;
Theorem 13.3's transition is the consumer's concern, not the relation's).
Everything else — including R8, R9, R7 with anything — is `None` (§4.2:
"R8, R9 ⊒ nothing; R7 ⊒ nothing"). A total match with no catch-all; a test
pins the full 11×11 table.

## Section 4 — `kernel/patch.rs` (NEW) — §3.1 traits, verbatim shapes

```rust
pub struct IBox<const N: usize> { pub lo: [f64; N], pub hi: [f64; N] } // try_new refuses lo>hi, non-finite
pub struct CertifiedPositive(f64);   // try_new refuses <= 0 or non-finite; pub fn value(&self) -> f64
pub struct CertifiedNonzero(f64);    // try_new refuses == 0 or non-finite; records sign
pub struct Cone { pub axis: [f64; 3], pub half_angle: f64 } // try_new refuses half_angle outside [0, PI), non-unit axis beyond 1e-9
pub struct DerivativeEnclosure { pub su: IBox3, pub sv: IBox3 }   // type IBox3 = IBox<3>, IBox2 = IBox<2>
pub struct SecondDerivativeEnclosure { pub suu: IBox3, pub suv: IBox3, pub svv: IBox3 }
pub struct ThirdJetEnclosure { pub suuu: IBox3, pub suuv: IBox3, pub suvv: IBox3, pub svvv: IBox3 }
pub struct Degeneracy { pub box_: IBox2, pub egf2: (f64, f64) }    // EG - F^2 enclosure straddling/excluding 0
pub struct Pole { pub box_: IBox2, pub w: (f64, f64) }
pub type Reason = &'static str;

pub trait CertifiedPatch {
    fn enclose(&self, d: IBox2) -> IBox3;
    fn derivs(&self, d: IBox2) -> DerivativeEnclosure;
    fn normal_cone(&self, d: IBox2) -> Cone;
    fn regularity(&self, d: IBox2) -> ClaimVerdict<CertifiedPositive, Degeneracy, Reason>;
    fn weight_bound(&self, d: IBox2) -> Option<ClaimVerdict<CertifiedPositive, Pole, Reason>>;
}
pub trait CertifiedPatchC2: CertifiedPatch { fn second_derivs(&self, d: IBox2) -> SecondDerivativeEnclosure; }
pub trait CertifiedPatchC3: CertifiedPatchC2 { fn third_jet(&self, d: IBox2) -> ThirdJetEnclosure; }
```

Doc comments carry the spec's capability split verbatim (C2 required by R2,
R7, contact classifier; NOT by R1 tracing, completeness, overlap; C3 only by
the A2 cusp classifier, takes a BOX not a point). No implementors in this
packet — a compile-time doc test or comment names Wave-1 implementors.

## Section 5 — `kernel/leaf.rs` (NEW) — §3.2 type shapes, no extraction

```rust
pub struct BezierLeaf { pub degree_u: usize, pub degree_v: usize,
                        pub control: Vec<[f64; 4]> }   // homogeneous xyzw (N5)
```
`BezierLeaf::try_new(degree_u, degree_v, control)` refuses: control.len() !=
(degree_u+1)*(degree_v+1), degree 0, non-finite coordinates, and control
weight (w) <= 0 (positive CONTROL weights are the constructor-level
precondition; the per-box `weight_bound` certificate is derived later by the
implementor wave, and the fixture §7.4 pins the straddle case).
```rust
pub enum RationalCarrierKind { Plane, Sphere, Cylinder, Cone, Torus }
pub struct RationalCarrier { pub kind: RationalCarrierKind,
                             pub data: CarrierData, pub domain: IBox2 }
pub enum CarrierData { Plane { origin: [f64; 3], u_dir: [f64; 3], v_dir: [f64; 3] },
                       Sphere { center: [f64; 3], radius: f64 },
                       Cylinder { origin: [f64; 3], axis: [f64; 3], radius: f64, height: (f64, f64) },
                       Cone { apex: [f64; 3], axis: [f64; 3], half_angle: f64, height: (f64, f64) },
                       Torus { center: [f64; 3], axis: [f64; 3], major_r: f64, minor_r: f64 } }
```
`RationalCarrier::try_new` refuses non-finite data, non-positive radii,
non-unit axis (1e-9 slack, named constant), degenerate u/v directions, and
any half_angle outside (0, PI). Module doc: carriers are rational (half-angle
for the quadrics) per §3.2/N4; a transcendental-only carrier is
`RefusalKind::TranscendentalCarrier`, constructible by callers.

## Section 6 — `kernel/certs.rs` (NEW) — §8/§10.3/§11/§14.2 certificates

`type Interval = crate::formal::exact::CertifiedInterval;` (D-reuse).

```rust
pub struct Frame<const N: usize> { pub z_hat: [f64; N], pub q: [[f64; N]; N],
    pub q_tau: [f64; N], pub q_perp: [[f64; N]; N], pub a: [[f64; N]; N] }
```
`Frame::try_new` refuses: non-unit `q_tau` or `z_hat` beyond
`TOL_JACOBIAN`; Q's columns not orthonormal beyond `TOL_JACOBIAN`;
`q_perp` not the complement of `q_tau` in Q (column-wise check); `a`
non-finite. (N is 2..=7 in practice; no check beyond finiteness for N=1.)
```rust
pub struct PointCert { pub residual: ResidualId, pub box_: IBox, pub rho: f64 } // try_new refuses rho > RHO_MAX, box_ != IBox<1> is impossible by type
pub struct ArcCert<const N: usize> { pub residual: ResidualId, pub frame: Frame<N>,
    pub i_tau: Interval, pub b_perp: IBox, pub rho: f64,
    pub jac_encl: Vec<[f64; 2]>, pub weights: Option<Vec<CertifiedPositive>> }
```
`ArcCert::try_new` refuses `rho > RHO_MAX` (ρ is Lemma 8.0's contraction
rate, stored in the type per the booking), a `residual` that is `R2` (§8.3:
R2 is never an instance of the tube certificate — THE load-bearing type-level
ban; refuse with `RefusalKind::HighOrderJet`-adjacent named evidence... use
`RefusalKind::Conditioning`? NO — refuse `Refusal::new(RefusalKind::Budget,
..)` is wrong too: use `RefusalEvidence::Predicate { name: "R2_never_reaches_C2", .. }`
with kind `Conditioning`) — SPELLING: `Refusal::with_backing(
RefusalKind::Conditioning, VerdictClass::Inconclusive,
RefusalEvidence::Predicate { name: "R2_never_reaches_C2", detail: ... })`.
Also refuses empty `jac_encl` and `weights: Some(v)` with an empty v.
```rust
pub struct ContactCert { pub critical_point: PointCert, pub gap: Interval,
    pub tolerance: f64, pub hessian_sign: SignCert }
```
`ContactCert::try_new(critical_point, gap, hessian_sign)` derives tolerance
from `TOL_INTERSECTION` and refuses unless 0 ∈ gap AND width(gap) ≤
tolerance — the Proven case ONLY (§10.3); the Disproven (0 ∉ gap) and
Inconclusive outcomes are `ClaimVerdict` arms owned by the S5a packet, not
this type. The doc carries rule 7 verbatim: a tolerance-tagged claim never
unifies with an exact certificate.
```rust
pub struct GraphCert { pub domain: IBox2, pub n0: [f64; 3], pub det_bound: CertifiedNonzero }
pub struct R5Enclosure { pub q: IBox2, pub preimage: [IBox2; 2], pub cert: [PointCert; 2] }
pub enum PsiMapKind { Identity, Affine, Bilinear, RecognizedCarrier }
pub struct SheetCert { pub domain: IBox2, pub psi_kind: PsiMapKind, pub det_dpsi: CertifiedNonzero }
pub struct TubeOverlapCert { pub shared_point: [f64; 3], pub c1_bound: f64 } // try_new refuses c1_bound > EPS_REP, non-finite/non-unit-relevant data
```
`GraphCert::try_new` refuses a non-unit `n0` (beyond 1e-12, named constant).
`SheetCert`/`PsiMapKind` are the recorded spelling deviation for §16's
`psi: PsiMap` (S6's real map type arrives with its wave; the kind enum is
frozen NOW so `Sheet` below is stable).

## Section 7 — `kernel/graph.rs` (NEW) — §14/§16 topology types

`ChartId(pub u32)`; `Param { chart: ChartId, deck: i32, u: f64, v: f64 }`
(try_new refuses non-finite u/v); `Point4 { p1: Param, p2: Param }`;
`NodeId(pub usize)`, `BreakId(pub usize)`, `ArcId(pub usize)`;
`TopoNode { Boundary, TrimCrossing, MorseSaddle, MorseExtremum, A2Cusp,
OverlapBoundary, FilletEnd }`; `SegmentBreak { ChartSwitch, FrameSwitch,
LeafBoundary, DeckStep, R6ChartSwitch, R6BaseSwap }` — NEITHER enum has a
Refuse variant (§16 audit: "Refuse must not appear in TopoNode"); a test
pins the exhaustive variant lists. `ArcEnd { Topo(NodeId), Seg(BreakId) }`;
`NodeCert { Exact(PointCert), AtTolerance(ContactCert) }` (§2 rule 7's
never-unify, typed); `Node { id, at: Point4, kind: TopoNode, cert: NodeCert }`;
`Break { id, at: Point4, kind: SegmentBreak, overlap: TubeOverlapCert }`.

```rust
pub struct HermiteSegment { pub p0: [f64; 3], pub p1: [f64; 3], pub t0: [f64; 3], pub t1: [f64; 3] }
pub struct HermiteSpline { pub segments: Vec<HermiteSegment> }  // try_new refuses empty or non-finite
pub struct Approx { pub gamma: HermiteSpline }
pub struct Arc<const N: usize> { pub id: ArcId, pub approx: Approx,
                                 pub cert: ArcCert<N>, pub ends: (ArcEnd, ArcEnd) }
pub struct CarrierArc { pub id: ArcId, pub carrier: RationalCarrierKind, pub approx: Approx }
pub enum AnyArc { Ordinary(Arc<4>), Difference(Arc<2>), SelfInt(Arc<4>), Spine(Arc<7>), Carrier(CarrierArc) }
pub struct Sheet { pub domain: IBox2, pub psi_kind: PsiMapKind, pub cert: SheetCert, pub boundary: Vec<ArcId> }
pub enum Provenance { Claimed, Imported, Client }   // D6: not a certificate
pub struct CertifiedGraph { pub nodes: Vec<Node>, pub breaks: Vec<Break>,
    pub arcs: Vec<AnyArc>, pub sheets: Vec<Sheet>, pub exhaustive: bool }
pub struct ClaimedGraph { pub graph: CertifiedGraph, pub provenance: Provenance }
pub struct PartialGraph { pub graph: CertifiedGraph, pub frontier: Vec<Point4> }
```
Module doc: `ClaimedGraph` and `CertifiedGraph` never unify (D6/§15 — no
`From<ClaimedGraph> for CertifiedGraph`, ever); `Arc` shadows std module-
locally (D-spelling).

## Section 8 — `kernel/fixtures.rs` (NEW, `#[doc(hidden)] pub`) + `kernel/mod.rs`

`mod.rs`: module wiring, the D-shim statement, the crate-root re-export
(`KernelRefusal` only; also re-export `ClaimVerdict`, `Construction`,
`ResidualId`, `CertifiedPatch`, `IBox`, `PointCert` — the wave workers'
import surface, none of which collide at crate root), and `pub mod config;
pub mod evidence; pub mod residual; pub mod patch; pub mod leaf; pub mod
certs; pub mod graph; pub mod fixtures;`.

Fixtures — each a `pub fn` returning constructed shim types plus a doc-stated
NUMERIC ground truth, and a `#[cfg(test)]` test machine-checking it:

1. `transversal_sphere_plane()` — sphere center (0,0,0) r=1 (RationalCarrier)
   + plane z=0: ground truth: intersection circle radius 1 at z=0; test
   samples the implicit forms at the circle and off it (agreement within
   1e-12 named constant, `// H-3` opt-out where needed).
2. `coaxial_cylinders()` — two r=1 cylinders on the z-axis: ground truth:
   ExactSheet candidate, ψ=Identity, normal dot identically +1; test asserts
   the sheet construction from `SheetCert::try_new` with the two carriers'
   data fields and the constant-sign fact stated (no solver: the SIGN is
   input data, the test checks the certificate accepts the consistent
   inputs and refuses a flipped anti-parallel one).
3. `determinant_spans_zero()` — residual F=(x²−1, y) on box [−2,2]²: det DF
   = 2x spans zero; ground truth: the box is NOT certifiable by C1 — the
   test asserts `Refusal` with backing `Inconclusive` from the refusing
   constructor path that consumes it (`IBox`/evidence path, no solver).
4. `weight_straddles_zero()` — homogeneous quadratic weights (1, −1, 1):
   w(t) = 1 − 4t(1−t); ground truth: w(0.5)=0 exactly (expansion or
   direct), interval-style enclosure over [0,1] contains 0 → the
   `CertifiedPositive` bound construction refuses `WeightDegenerate` with
   backing Disproven; a shifted box where w > 0 certifies fine (the §7.1
   Disproven-vs-Inconclusive pair).
5. `deck_wrap()` — pcurve parameter run 5.9 → 6.4 over period 2π: ground
   truth: deck displacement +1 (exact integer from floor-div on the
   crossing count), canonical unwrap 6.4 − 2π computed as data (state the
   f64 value; do not recompute transcendental — the fixture STORES the
   constant, the test checks the deck integer only).
6. `c1_discontinuity()` — polyline positions [(0,0,0),(1,0,0),(1,1,0)]:
   ground truth: tangent direction jumps 90° between segments; test asserts
   the dot product of consecutive unit tangents is 0 (exact data), which is
   > TOL_PARAMETER discontinuity; carried as DATA — the C1 wave packet owns
   the `SpineNotC1` wiring (it lives in truck-geometry's landed vocabulary).

## Section 9 — lib.rs + tests

lib.rs gains exactly ONE line beside the other `pub mod`s (A6 goes 12 → 13):
`pub mod kernel;` plus the crate-root `pub use kernel::{...}` re-export line
from Section 8.

Tests: `tests/kernel_contract.rs` (NEW) — the 16 `tests_required` names.
The variant-list tests (refusal_kind_has_all_spec_variants,
topological_node_enums_have_no_refuse_variant,
residual_implication_order_is_exactly_rule_c) pin the FULL variant/relation
tables exhaustively — adding a §17 variant or a Rule-C implication later is
a deliberate spec amendment, not a silent edit. House rules: H-3 same-line
opt-outs; clippy zero findings on new files; fmt clean.

## Done-when

- `cargo fmt` clean; clippy `-p truck-certified` zero findings attributable
  to the new files (run the EXACT verify invocation form:
  `cargo clippy -p truck-certified --all-targets --message-format=short
  --no-deps`, unfiltered output, fix ALL findings before claiming).
- `cargo test -p truck-certified --lib --tests --no-fail-fast` green —
  landed suites unchanged plus the 16 contract tests.
- `cargo check --workspace --all-targets` green (the module is additive;
  zero consumers outside the crate exist yet).
- CARGO_BUILD_JOBS=2-4 on every cargo invocation; if `RUSTC_WRAPPER=sccache`
  rejects the incremental dev profile, unset it locally and record that in
  RESULT notes.

## Stop conditions

Stop, commit nothing beyond WIP evidence, write RESULT.json (AT THE
WORKTREE ROOT) with the finding verbatim if:

1. A landed shape this packet aliases (`CertifiedInterval`,
   `CertifiedSign`) differs from the signatures quoted above — stop, do
   not adapt silently.
2. A required fixture cannot carry its stated ground truth without solving
   — the fixture list is frozen; say which fixture and what obstructs.
3. Adding `pub mod kernel;` breaks an existing crate-root re-export or
   triggers a missing_docs/deny(warnings) failure you cannot fix within the
   write set — record the exact compiler output.

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

Commit your work on the current branch (subject: `feat(certified): kernel
v2 shim — shared types + refusing constructors + fixture kit
(BG-KV2-000-CONTRACT)`) BEFORE writing `RESULT.json`.
