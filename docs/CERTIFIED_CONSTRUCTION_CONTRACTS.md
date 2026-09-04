# Certified Construction Contracts — CC program spine

**Status:** spine document, session 51. Freezes every cross-packet contract
for the CC program (`CERTIFIED_CONSTRUCTION_BUILD_SPEC.md`) before any packet
dispatches. Rust code is NOT landed by this document; the types below land as
ONE shim packet (`CC-000-CONTRACT`) through the normal loop, per the
build-spec spine workflow (`loop/ORCHESTRATOR.md`, "The build-spec spine
workflow"). Exact Rust spelling is adjustable at CC-000 review; semantics are
not. Where this document contradicts the build spec, this document wins and
the build spec carries the errata (§5).

---

## 1. Measured substrate facts (re-derived by command, session 51)

These are the facts every decision below rests on. Each was verified by
command this session; the anchor ritual applies to all of them.

| # | Fact | Evidence |
|---|---|---|
| F1 | `truck-certified` does NOT depend on `truck-evidence`, and `truck-evidence` does NOT depend on `truck-certified`. No direct edge either direction. | `vendor/truck/truck-certified/Cargo.toml` deps: cgmath, robust, spade, truck-{base,geotrait,geometry,polymesh,topology}. `truck-evidence/Cargo.toml` deps: inari, truck-{base,geotrait,geometry}. |
| F2 | `truck-geometry` does NOT depend on `truck-certified`. | `truck-geometry/Cargo.toml`: truck-base, truck-geotrait only. |
| F3 | `inari` appears NOWHERE in `truck-certified` (zero imports; two prose mentions). Its interval type is `formal::exact::CertifiedInterval` (outward via next_down/next_up), aliased in the kernel as `pub type Interval = crate::formal::exact::CertifiedInterval` (`kernel/mod.rs:50`). The KV2 shim doctrine is "zero new manifest edges (no inari)" (`kernel/mod.rs:29`). | rg over tree; kernel/mod.rs |
| F4 | `truck-evidence`'s interval world is `inari::Interval` (re-exported as `truck_evidence::enclosure::Interval`); its box type is `Box3 { x, y, z: Interval }`. Two interval universes exist and must be bridged explicitly, never silently. | `truck-evidence/src/enclosure.rs:20,27-35` |
| F5 | `truck-certified/src/construct/` does not exist; `lib.rs` carries no `construct` module. Free for the shim. | `Test-Path` false; lib.rs module list |
| F6 | Landed certified primitives available for reuse: `kernel::patch::{IBox, IBox2, IBox3}` (pub, const-N interval boxes), `CertifiedSurfaceMap/CertifiedCurveMap::rank_margin` (certified lower bound of \|Sᵤ×Sᵥ\| resp. \|C′\|), `hull::{hull_bernstein_1d, hull_bernstein_2d, bernstein_derivative_*, JetOrder}`, `ssi::krawczyk3_certificate`, `kernel::engine::{krawczyk_c1, krawczyk_c1_n3, krawczyk_c1_n4}`, `kernel::fixtures` fixture-kit pattern (doc-hidden, `Result<_, Refusal>` builders, machine-checked ground truths), `kernel::config` const pattern. | probes + tree |
| F7 | Landed evidence primitives available for reuse: `num::krawczyk::krawczyk<const N>` + `KrawczykSystem`/`KrawczykProof`, `contact::{contact, BoundedStratum, ContactComplex}`, `contact::gff::cover_branch`, `contact::implicit::ImplicitField` (plane/sphere/cylinder/cone/torus), `EnclosureSurface/EnclosureCurve` (inari world), `fid::lfs::FaceScaleComponents` + private `box_distance`. | probes |
| F8 | `truck-base/src/bvh.rs` `Bvh<P: BoundedPiece>` has ONLY overlap queries (`candidate_pairs`, `candidate_pairs_self`, `query`). No box/point/BVH distance anywhere in truck-base; a private `box_distance` exists in `truck-evidence/src/fid/lfs.rs:298`. | probe |
| F9 | `truck_base::evidence::Certificate` is constructed field-by-field (no convenience constructor); `Outcome<T> = Result<Certified<T>, Refusal>`; `Budget { subdiv, newton, depth }` is the universal budget ledger. | probe |
| F10 | `OpKind` already carries `Loft`, `Fillet`, `Offset` (9-variant closed vocabulary, spec-amendment-only). `EntityId`/`Op`/`OpParams` are landed with bit-exact equality and transform-stable derivation. | `truck-topology/src/entity_id.rs` |
| F11 | Unwrap discipline baseline: `lib.rs:5` `#![deny(clippy::unwrap_used)]`; authored kernel modules carry zero unwrap/expect/panic and no module allow; 19 grandfathered `formal/` modules carry documented allows. New construct modules must match the authored-kernel discipline, not the grandfathered one. | grep census, probe |
| F12 | Config constant pattern: `kernel/config.rs` publishes normative f64/u32 consts with verbatim doc comments (BG-KV2-000 contract). `DirectTolerance` in truck-geometry is deliberately untouched; the two default sources coexist. | probe |

## 2. Contract decisions (pre-made; the worker does not relitigate)

**C1 — One home: `truck-certified/src/construct/`.** All Phase A/B/C/D
construction modules live in one new module tree of `truck-certified`.
Rationale: F2 kills the build-spec placement of loft core in
`truck-geometry` (it could not call the certified banded solve), and F1 kills
placement of the blend spine in `truck-evidence` (it could not call
`ssi_trace`/`kernel::engine`). `truck-certified` already owns the collocation,
Krawczyk, branch-tracing, and hull substrate; the construct layer is its
natural consumer, not a new universe. Consequences for the build spec: §5
errata (CC-010..015, CC-020, CC-025, CC-026, CC-030..033 write sets move to
`truck-certified/src/construct/**`).

**C2 — Exactly one new manifest edge: `truck-certified → truck-evidence`.**
Added by the CC-000 shim, once, and never extended further without a spec
amendment. Needed because P5 clearance and the L5 far-pair contact funnel live
in `truck-evidence` (F7) while their consumers (loft validity, blend) live in
`truck-certified`. This invokes the booking escape hatch deliberately and is
recorded here as such (precedent: CG-004's polymesh edge). No cycle: F1 shows
`truck-evidence` depends on nothing that depends on `truck-certified`. The
KV2 "zero new manifest edges" doctrine is scoped to the kernel module and is
amended, with this paragraph as the record, for the construct module only.

**C3 — Interval universe: no inari in `truck-certified`.** The construct
module uses `pub type Interval = crate::formal::exact::CertifiedInterval`
(same alias as the kernel, F3) and `kernel::patch::IBox{2,3}` for parameter
boxes. The inari world is never imported except through the C2 edge's
boundary types. The sole bridge is `construct/convert.rs`:
`from_inari(inari::Interval) -> CertifiedInterval` and
`box3_to_ibox(Box3) -> IBox<3>`, both exact lo/hi field copies with a
documented soundness note (both universes are outward-rounded; the copy is
order-preserving and introduces no width).

**C4 — Refusal vocabulary: a new `construct::ConstructRefusal` enum.** The
theory §9 taxonomy lands as a dedicated enum in `construct/refusal.rs` —
NOT as new variants on `truck_base::evidence::Refusal` (whose envelope is
frozen and consumed workspace-wide) and NOT on `kernel::evidence::Refusal`
(KV2-scoped). Initial frozen variant set (mapping table in
`CERTIFICATE_MAPPING.md`):

```
NonPositiveWeightField, SingularInterpolationSystem, AmbiguousCorrespondence,
FocalDegeneracy, CanalSingular, RankDeficientContact,
UnintendedContact, StarNotEmbedded, NoAdmissibleProjection,
NonGenericThicknessEvent, AmbiguousEventOrdering,
InvalidInput, ConditioningBelowThreshold
```

`RankDeficientContact` and `ConditioningBelowThreshold` coexist deliberately:
the construct enum carries the theory name; conversion to
`contract::Refusal::ConditioningBelowThreshold` at the frozen-contract
boundary is documented, not conflated. Every variant must be reachable in a
test (build-spec gate §5).

**C5 — Reuse, never re-derive, these landed primitives.** σ margins from
`rank_margin` (F6); Bernstein hull kernels from `hull.rs` for L1r; Krawczyk
arity ≤ 4 from `kernel::engine`; the branch-tracing vocabulary from
`ssi_trace`; the fixture-kit pattern from `kernel/fixtures.rs`; `Budget` from
`truck_base::evidence` (F9). Where a construct function needs something these
provide, it takes them as arguments, not as re-implementations.

**C6 — Config constants land in `construct/config.rs`** following F12's
pattern, all normative in CC-000: `CC_N_EXACT: usize` (P1 exact rational path
threshold; interval fast path above it), `CC_ETA_J: f64` (regularity margin
floor), `CC_ETA_PI: f64` (P3 projection determinant margin),
`CC_MU_CLEAR: f64` (P5 clearance margin μ), `CC_DEPTH_MAX: u32` (subdivision
depth cap). Values are pre-made here so no wave worker picks a constant:
`CC_N_EXACT = 64`, `CC_ETA_J = 1e-12`, `CC_ETA_PI = 1e-12`,
`CC_MU_CLEAR = 1e-9`, `CC_DEPTH_MAX = 40`.

**C7 — Stub posture for the shim.** CC-000 lands types, refusing
constructors, and the fixture kit — no solver bodies. Every public function
returns `Err(ConstructRefusal::Unfrozen)` (a variant reserved for exactly
this posture, matching the `contract::Refusal::Unfrozen` precedent). The
refusing-stub pattern is what downstream wave packets type against.

**C8 — Crate placement of the two exceptions.** CC-004 (P5 clearance) keeps
its cross-crate write set — `truck-base/src/bvh.rs` (additive distance
query, leaf crate) and `truck-evidence/src/clear.rs` — because the distance
substrate is a leaf fact (F8), not a construct fact. It is the only packet
outside `truck-certified` + the C2 edge.

**C9 — Determinism house rules carry into construct/ verbatim:** fixed-order
float reductions, no hash-iteration-dependent output, no bare absolute
literals (H-3 same-line opt-out in tests), no unwrap/expect/panic in shipped
code (F11), COMMIT BEFORE RESULT.json. These go into every packet's house
rules block unchanged.

## 3. Frozen seams (the cross-packet inventory)

Semantics frozen now; exact spelling adjustable at CC-000 review. Each seam
names its producer packet and consumer packets. Signatures use C3's
`Interval`/`IBox` universe unless marked otherwise.

**S1 — `Interval` + box types (CC-000 → all).**
`pub type Interval = crate::formal::exact::CertifiedInterval;`
Parameter boxes are `kernel::patch::IBox2/IBox3` (reused, not redefined).
Bridge: `construct/convert.rs::{from_inari, box3_to_ibox}` (C3).

**S2 — `ConstructRefusal` (CC-000 → all).** The C4 enum, `Debug + Clone +
PartialEq`, plus `tag(&self) -> &'static str` (the `MapRefusal::tag`
precedent). Grows only by CC-000 amendment.

**S3 — P1 banded solve (CC-001 → CC-010/012/015, CC-033).**
```rust
pub struct BandedFactor { /* private bands, q, n */ }
pub fn factor_banded_tp(bands: &[Interval]) -> Result<BandedFactor, ConstructRefusal>
    // bands = row-major collocation coefficients; refuses on any interval pivot containing 0
impl BandedFactor {
    pub fn solve_homogeneous(&self, rhs: &[[Interval; 4]]) -> Result<Vec<[Interval; 4]>, ConstructRefusal>;
    pub fn max_control_error(&self) -> f64;   // the L2 enclosure width ε
}
```
Fast path only in CC-001 (interval no-pivot GE for the banded-TP class; the
de Boor–Pinkus growth-factor-1 justification goes in the module doc). The
Rump residual fallback is `construct/residual_solve.rs`, separate fn, dense
`R ≈ A⁻¹` input, same refusal type. Rational exact path is behind
`CC_N_EXACT`, separate fn, booked CC-001 but MAY be stub-refused in v1 if
`num-rational` stays out of the manifest (decision at CC-000 review; refusal
`SingularInterpolationSystem` is NOT reachable via the stub — the stub
refuses `Unfrozen`).

**S4 — P2 injectivity radius (CC-002 → CC-014, CC-021, CC-030).**
```rust
pub fn injectivity_radius(map: &CertifiedSurfaceMap, sub: SurfaceRegion)
    -> Result<Interval, ConstructRefusal>;      // δ = 2σ/L, σ from rank_margin, L from hull jets
pub fn curve_injectivity_radius(map: &CertifiedCurveMap, sub: CurveRegion)
    -> Result<Interval, ConstructRefusal>;      // 1-D variant
```
Refuses when σ ≤ 0 (degenerate) — never returns δ = 0 as a success.

**S5 — P4 argmin-with-margin (CC-003 → CC-013, CC-026, CC-030).**
```rust
pub fn argmin_margin(enclosures: &[Interval]) -> Result<usize, ConstructRefusal>;
    // returns i* only if sup[i*] < inf[j] for all j != i*; overlap -> AmbiguousEventOrdering
```

**S6 — P3 graph-disk (CC-005 → CC-014, CC-022, CC-033).**
```rust
pub struct DiskPiece { pub det_lower: Interval, pub boundary_simple: bool, pub seam_glued: bool }
pub struct GraphDiskCert { /* witness set, projection w, per-piece records */ }
pub fn certify_graph_disk(pieces: &[DiskPiece], boundary: &BoundaryPlan)
    -> Result<GraphDiskCert, ConstructRefusal>;
```
`BoundaryPlan` (boundary simplicity input) is frozen in CC-000 as a stub
type; its production comes from the planar machinery (`formal/intersection`,
`formal/xmonotone`) inside CC-005, not across the seam.

**S7 — P5 clearance (CC-004 → CC-014, CC-021, CC-023, CC-030).** Two-layer
seam. Layer 1 (truck-base, leaf, additive):
```rust
impl<P: BoundedPiece> Bvh<P> {
    pub fn distance_lower_bound(&self, other: &Bvh<P>) -> f64;  // ≥ true min distance; +inf when disjoint-certified
    pub fn distance_lower_bound_self(&self) -> f64;
}
```
Layer 2 (truck-evidence, `construct`-adjacent, inari world):
```rust
pub enum BallAdmissibility { Fillet, Round }
pub fn ball_clearance(field: &impl ImplicitField, centre: &Box3, exclusion: &Box3,
                      r: Interval, mu: f64, mode: BallAdmissibility)
    -> Result<bool, Refusal>;
    // mu passed explicitly: truck-evidence cannot read construct/config.rs (F1);
    // the certified-side wrapper supplies CC_MU_CLEAR.
    // AMENDED at CC-004 landing (session 51): `centre: &Box3` (the ball-centre
    // region) is an explicit parameter — the displaced-ball and straddle ground
    // truths cannot be expressed without it. Separation is a tri-state over the
    // single certified axis-gap lower bound: d > mu => Clear; d == 0 => Rejected;
    // 0 < d <= mu => Err(NumericallyUnresolved/UncertifiedContainment) with a
    // zero-spend ledger (this layer performs one decisive interval evaluation
    // per side, no retry; higher precision is the caller's escalation).
```
Consumed through the C2 edge; the inari→CertifiedInterval bridge at this
boundary is `convert.rs` (S1), the ONLY sanctioned conversion site.

**S8 — Loft construction (CC-010 → CC-012/013/014, CC-015).**
```rust
pub fn averaged_knot_vector(stations: &[f64], degree: usize) -> KnotVec;  // de Boor averaging, L0 construction
// AMENDED at CC-010 landing (session 51): the homogeneous carrier is the
// landed `Vector4` (truck-geometry), not a new Point4 — no new carrier type.
pub struct LoftOutput { pub surface: BSplineSurface<Vector4>, pub epsilon: f64 }  // homogeneous net + L2 enclosure
pub fn loft_sections(sections: &[BSplineCurve<Vector4>], stations: &[f64], degree: usize,
                     factor: &BandedFactor) -> Result<LoftOutput, ConstructRefusal>;
```
Chord-length stationing fn is CC-010-internal (normative summation order in
its doc); uniform stationing is the `stations = linspace` caller choice.
`BSplineSurface<Point4>` over `KnotVec` reuses `truck_geometry::nurbs` types
(certified already depends on geometry, F1).

**S9 — Correspondence (CC-013 → CC-012, CC-014).**
```rust
pub struct Correspondence { pub orientation: bool, pub anchor: usize, pub shifts: Vec<usize> }
pub fn resolve_correspondence(wire: &WireComplex, sections: &[WireComplex],
                              functional: &ShiftFunctional) -> Result<Correspondence, ConstructRefusal>;
```
`WireComplex` (abstract oriented cyclic complex) and `ShiftFunctional`
(declared geometric functional) are frozen as stub types in CC-000; the
argmin over r cyclic shifts calls S5. `AmbiguousCorrespondence` on enclosure
overlap — never a proximity tie-break.

**S10 — Canal regularity (CC-025 → CC-021, CC-030/031).**
```rust
pub struct RadiusLaw;  // frozen enum stub in CC-000: Constant(f64) | Linear(..) | CubicHermite(..) | MonotoneCubic(..) | VertexControl(..)
pub fn canal_regularity(spine: &CertifiedCurveMap, radius: &RadiusLaw, arc: (f64, f64))
    -> Result<Interval, ConstructRefusal>;   // min |a²−rq| − ra‖c″‖ over the arc; straddle -> CanalSingular
```
Arc-restricted form per theory §6.3; the all-θ variant is a separate fn
booked with the closed-pipe consumer, not this seam.

**S11 — k=3 contact system (CC-020 → CC-021, CC-030).** Lives in
`construct/contact3.rs` over `kernel::engine` arity-4 machinery (the
constrained system with Φ reduces to a ≤4-unknown square form; the
reduction is CC-020's designed content). Seam frozen as output type only:
```rust
pub struct TripleContactNode { pub centre: [Interval; 3], pub radius: Interval, pub contacts: [[Interval; 2]; 3] }
pub fn solve_triple_node(supports: &[SurfaceRegion; 3], radius: &RadiusLaw, seed: IBox3, budget: &mut Budget)
    -> Result<TripleContactNode, ConstructRefusal>;
```
The exact square-system formulation is CC-020's to design; the seam is this
output type and the refusal path (`RankDeficientContact`).

**S12 — Blend trace (CC-030 → CC-032/033).** Output-type seam only:
```rust
pub struct BlendEvent { pub kind: EventKind, pub at: Interval, pub node: Option<TripleContactNode> }
pub struct EventKind;  // frozen enum stub in CC-000: Trim | ThirdFace | Focal | Rank | Collision | Trace
pub struct BlendTrace { pub events: Vec<BlendEvent> }  // complete walk; no topology between events
pub fn trace_blend_chain(branches: &[BranchSeed], radius: &RadiusLaw, budget: &mut Budget)
    -> Result<BlendTrace, ConstructRefusal>;
```
`BranchSeed` frozen as stub in CC-000. Consumers must not need more than
`BlendTrace` + S6 + S12 to do face consumption and setback work.

## 4. Write-set pre-matrix, Phase A (pairwise)

`mod` = the one-line `pub mod x;` insertion in `construct/mod.rs` (designed
conflict, session-50 exempt) — every packet below also inserts one.

| Pair | 000 | 001 | 002 | 003 | 004 | 005 |
|---|---|---|---|---|---|---|
| 000 (construct core: mod/refusal/convert/config/fixtures, lib.rs) | — | mod | mod | mod | mod | mod |
| 001 (`construct/banded.rs`, `construct/residual_solve.rs`) | | — | disjoint | disjoint | disjoint | disjoint |
| 002 (`construct/injectivity.rs`) | | | — | disjoint | disjoint | disjoint |
| 003 (`construct/argmin.rs`) | | | | — | disjoint | disjoint |
| 004 (`truck-base/src/bvh.rs` additive, `truck-evidence/src/clear.rs`, `truck-evidence/Cargo.toml` if needed) | | | | | — | disjoint |
| 005 (`construct/graphdisk.rs`) | | | | | | — |

All Phase-A packets are mutually disjoint except the designed `mod.rs`
one-liners. CC-000 additionally owns `Cargo.toml` (the C2 edge) — no other
packet touches any manifest. 000 must land before any of 001–005 dispatches
(genuine shared contract: S1/S2 types). 001–005 dispatch in parallel after
000's landing merge SHA becomes the wave base.

## 5. Build-spec errata (recorded; build spec rows to be amended at next edit)

- **CC-010..013, CC-015** (loft/gordon/correspondence): write sets move from
  `truck-geometry/src/construct/*` to `truck-certified/src/construct/*`
  (C1; F2 makes the original placement unimplementable).
- **CC-020** (k=3 contact): moves from `truck-evidence/src/contact/triple.rs`
  to `truck-certified/src/construct/contact3.rs` (C1; F1 makes the original
  placement unable to reach the blend consumers).
- **CC-025, CC-026, CC-030..033**: write sets move to
  `truck-certified/src/construct/**` accordingly (C1).
- **CC-004** keeps `truck-base` + `truck-evidence` and gains one line in
  `truck-certified`'s consumer tests through the C2 edge (C8).
- **CC-000** gains: `truck-certified/Cargo.toml` (C2 edge),
  `construct/{mod, refusal, convert, config, fixtures}.rs`, one `pub mod
  construct;` line in `lib.rs`, and the `CERTIFICATE_MAPPING.md` rows are
  booked in packet prose but edited by the orchestrator at landing (docs are
  not worker write set).

## 6. Fixture kit inventory (CC-000's `construct/fixtures.rs`, doc-hidden)

Machine-checked ground truths, `Result<_, ConstructRefusal>` builders,
kernel/fixtures.rs pattern (F6). Each primitive's success AND refusal paths
get a fixture at shim time so wave workers build against fixtures, not
against each other's code.

| Fixture | Ground truth | Serves |
|---|---|---|
| `banded_cubic_uniform(n=4)` | cubic collocation matrix under uniform stations; exact rational solution known; det sign known | CC-001 success |
| `banded_pivot_spans_zero()` | matrix whose first interval pivot contains 0 | CC-001 refusal path |
| `argmin_separated()` / `argmin_overlapping()` | enclosure arrays with strict sup<inf resp. overlap | CC-003 both paths |
| `flat_patch()` / `curved_patch()` | planar patch (σ>0, L=0, δ=∞) and a patch with known curvature bound | CC-002 success |
| `degenerate_patch()` | σ enclosure contains 0 | CC-002 refusal path |
| `folded_corner()` | constructed fold (determinant sign change) | CC-005 refusal `NoAdmissibleProjection`/`StarNotEmbedded` |
| `genuine_star()` | two-plane wedge star, known embedded | CC-005 success |
| `pn_prism()` | plane/sphere/cylinder face set with known clearance answers at small t | CC-004 both paths |

## 7. What CC-000's packet carries (authoring checklist)

1. YAML front matter per the schema (`BG-KV2-502-S9B` shape): id
   `CC-000-CONTRACT`, class design, crates `[truck-certified]`, write allow
   per §5, anchors measured by command before dispatch, tests_required
   listing the fixture kit's ground truths.
2. Sections: S1/S2 types verbatim, C4 refusal enum verbatim, C6 constants
   verbatim, stub constructors refusing `Unfrozen`, convert.rs with the C3
   soundness note, the fixture kit (§6), the C2 manifest edge, module doc
   stating the C1/C2/C7/C9 doctrine.
3. House rules: C9 block verbatim, plus the cargoq PATH rule.
4. `gen_packet.py --check` + `packet_lint.py` before dispatch; anchors
   re-run against the post-KV2-battery tree (anchor drift is a per-dispatch
   ritual).

Dispatch posture: CC-000 goes through the NORMAL loop (full verify) — it is
the shim; its landing merge SHA is the CC wave base. It dispatches only
after the KV2 final battery completes and Wave-5 rows flip DONE (the battery
must run at a stable integrated HEAD; CC-000 touches the same crate).
