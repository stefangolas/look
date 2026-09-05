# Certified Interaction Engine — Build Spec (BIE program)

**Status:** proposed program, written this session. Root theory:
[`CERTIFIED_INTERACTION_ENGINE_SPEC.md`](CERTIFIED_INTERACTION_ENGINE_SPEC.md)
(theory §0–15; §16 holds the first reconciliation audit — this document
expands it into packets, write sets, and LOC). Scope decision recorded in
§2: the program targets the **restricted interaction**
(`SpineFrameSurface × canonical`) first, which is the only pair family the
showcase models need, plus the already-landed canonical × canonical pairs.

House rules apply: anchors re-derived by command before quoting in a packet;
kernel changes only through the packet loop; new `Refusal`/evidence arms are
SPEC_GAPs booked in `docs/CERTIFICATE_MAPPING.md`, never drive-by edits.

## 1. Substrate audit (theory element → landed machinery → disposition)

| Theory element | Substrate status | Anchor | Disposition |
|---|---|---|---|
| §1.1 interaction $F = R_A - R_B$ | **Directly evaluable** — `subs`/`der` exist on both sides; no cross-multiplied polynomial system needed on the normal path | `SpineFrameSweep: ParametricSurface` (`constructive/sweep_surface.rs:124`), canonical `Surface` | BIE-002 evaluates F directly; NURBS polynomialization is never built (homotopy deferred) |
| §1.2 product stratification | Face/edge/vertex strata exist as topology; product strata are enumerated per pair | `truck-topology` face/edge iteration; `BoundedStratum::Face/Edge` (`truck-evidence/src/contact/mod.rs`) | BIE-002 enumerates product strata; interval exclusion disposes of empties |
| §2.2 metric normalization $\sigma$ | First-fundamental form computable from landed derivative traits; Thm 2.1 identity not implemented | `ParametricSurface::derivative` family; `EnclosureSurface::enclose_der` (M₂) | BIE-002 implements $\sigma$/$\sigma_G$; $M_2$ from `enclose_der` |
| §2.3 degenerate charts $\sigma_G$ | Canonical carriers with poles (sphere/cone) recognized exactly; sweeps have regular grids (no poles) | `recognize.rs` canonical witnesses | $\sigma_G$ needed **canonical-side only** in the restricted scope — a genuine scope reduction |
| §3.1 (E) exclusion | Bernstein hull landed 1-D/2-D; n=4 boxes ABSENT; interval-arithmetic library ABSENT (`IntervalEnclosure` is a value type only, `contract.rs:117`) | `truck-certified/src/hull.rs:95/126` | BIE-001: outward-rounded interval ops + mean-value range bounds for our F forms (Taylor-model style), Bernstein only where the form is polynomial |
| §3.2 column selection | Closed form for 3-of-4 (try all four); no RRQR | — | BIE-002, ~100 LOC |
| §3.3 (R) slicewise Krawczyk | **LANDED, generic**: `KrawczykSystem<const N: usize>` + `krawczyk::<N>` | `truck-evidence/src/num/krawczyk.rs:62/86` | BIE-002 instantiates N=3 (boundary seeding) and N=4 (polar/link); parallelotope tracker is new |
| §3.5 (R′) minor-sign predicate | New but small: certified-sign 3×3 determinant over a box | `formal/exact.rs` Shewchuk `Expansion` exists for exact signs | BIE-002 |
| §4.2 polar exclusion (Thm E) | Square 4×4 system; machinery = (E) + Krawczyk | — | BIE-004; homotopy never built unless Thm E fails (theory §10.1 caveat) |
| §6.4 (χ_A, χ_B) + boundary selection | **LANDED and carrier-agnostic**: the classifier is seed-and-propagate over the parity graph, no implicit field | `truck-shapeops/src/boolean/classify.rs:5-12,112-146`; `fragment_decision` (pure logic, landed) | **BIE-006 is adapters only** — the biggest downward LOC surprise of this audit |
| §7.1 pcurve embedding (Lemma F) | Used implicitly; not stated/tested | — | BIE-005 asserts it as a test oracle (metamorphic: per-pair pcurve simplicity) |
| §7.2 arrangement in charts | **KEY GAP**: `Region2` machinery is planar and reusable in the (s,v) chart, but `Carrier2D` is `Line | CircleCarrier` only (`arrange.rs:682`) — no spline/PL chart carrier | `truck-geometry/src/arrange.rs:682-708` | BIE-005: certified-PL (or B-spline) chart curve carrier + crossings. PB-002 (sketch arcs) shares this substrate |
| §8.1 procedural carrier | `IntersectionCurve` decorator exists; certified procedural carrier with PL-at-tessellation policy ABSENT; `EdgeSampleLedger` landed | `canonical.rs:94/133`; CG-005 ledger | BIE-003: the type ripple + ledger integration |
| §8.3 provenance P₋₁ | `EntityId`/`Op`/`OpKind` (+ a `Selector` primitive) LANDED in topology | `truck-topology/src/entity_id.rs:249/176` | BIE-006 records provenance on output fragments; propagation rules are new but thin |
| §9 validity gates | Manifold diagnostics landed; χ valuation + mod-2 homology ABSENT (cheap: finite complex) | `truck-topology/src/manifold.rs` | BIE-007 |
| §10.2 typed outcomes | `Refusal`/`NumericallyUnresolved` landed with witness slot; three-valued verdict doctrine landed | `truck-base/src/evidence.rs`; `CERTIFICATE_MAPPING.md` doctrine | BIE-000 maps `Unresolved { κ, cell, slope }` onto the witness slot — zero new refusal arms expected (SPEC_GAP rule if violated) |
| Broad phase | BVH candidate pairs landed; no per-face AABB for swept faces (span cache treats `SpineFrameSurface` non-canonical) | `truck-base/src/bvh.rs`; `span.rs:20` | BIE-003 adds swept-face bounds (sampling/enclosure-based) |
| Output faces | **Already representable**: `SpineFrameSweep` carries windowed domains `(s_first, s_last, v0, v1)` — trimmed output sweep faces need no new surface type | `sweep_surface.rs:52` (BG-KV2-501 normative deviation) | BIE-006 emits windowed sweeps directly |

## 2. The tie-in map — exact pipeline, stage by stage

The landed Boolean (`truck-shapeops/src/boolean/assemble.rs:56`) runs:
lift → contact → events → split → classify → decide → assemble → gate.
Per stage: what the interaction engine replaces, reuses, or adds.

```text
                    LANDED PIPELINE                     BIE CHANGE
 ┌─────────────────────────────────────────────┐
 │ 0. provenance (EntityId/Op)                 │  reuse; record on output (§8.3)
 ├─────────────────────────────────────────────┤
 │ 1. LIFT  face/edge → BoundedStratum         │  ADD BoundedStratum::Sweep{recipe,window};
 │    recognize_surface → canonical witness    │  canonical side unchanged; Unrecognized
 │    Unrecognized → NonCanonicalCarrier  ◄────┼── the gate that blocks sweeps today
 ├─────────────────────────────────────────────┤
 │ 2. CONTACT  per-pair analytic dispatch      │  RESTRICTED ADD: Sweep×Canonical and
 │    coaxial/parallel + analytic cells        │  Sweep×Sweep interaction systems (§1.1);
 │    (plane_plane, plane_cylinder, …)         │  canonical×canonical stays landed
 ├─────────────────────────────────────────────┤
 │ 3. EVENTS  AABB screening → ContactEvent    │  REUSE bvh candidate pairs; ADD swept-
 │    StratumRef{A|B, face, edge}              │  face bounds (BIE-003)
 ├─────────────────────────────────────────────┤
 │ 4. SPLIT  point cuts, FF arcs, Region2      │  GENERALIZE: trims in the (s,v) chart
 │    FragmentMesh, Same/Flip parity,          │  (a plane — Region2 reusable) with the
 │    CoincidentPair                           │  BIE-005 chart carrier; parity unchanged
 ├─────────────────────────────────────────────┤
 │ 5. CLASSIFY  seed + propagate over parity   │  REUSE VERBATIM — carrier-agnostic by
 │    graph → inside_other bits                │  construction (classify.rs:5-12); sweep-
 │                                             │  side seeds resolve via arrangement cells
 ├─────────────────────────────────────────────┤
 │ 6. DECIDE  fragment_decision(op, χ-states)  │  REUSE VERBATIM — pure logic
 ├─────────────────────────────────────────────┤
 │ 7. ASSEMBLE  keep/flip, sew, output         │  ADAPT: output sweep faces are windowed
 │    Solid::try_new gate                      │  SpineFrameSweeps (type exists); edges
 │                                             │  carry BIE-003 procedural carrier
 ├─────────────────────────────────────────────┤
 │ 8. GATES  Solid::try_new (closed/manifold)  │  ADD χ valuation + Z₂ homology layer;
 │                                             │  typed Unresolved upstream (never a shape)
 └─────────────────────────────────────────────┘
        ▲                                  ▲
        │ BIE-001 interval/Bernstein boxes │ BIE-002 σ, Krawczyk N=3/4, continuation
        └──────── shared numerical service ──────── (theory §10.1, minus homotopy)
```

**The four genuinely new mechanisms** (everything else is wiring):

1. **BIE-002** the interaction solver: σ/σ_G, column choice, (R′), seeding,
   continuation — the theory's Phase 1, restricted to our F forms.
2. **BIE-003** the procedural curve carrier (theory §8.1: "prerequisite for
   everything, requires none of the theory").
3. **BIE-005** the chart arrangement carrier (2-D certified curves in (s,v)).
4. **BIE-007** the validity gate layer.

**Scope reductions the restricted target buys** (each is LOC avoided):

- No NURBS cross-multiplication/polynomialization: F is evaluated directly
  through landed `subs`/`der` (~1k avoided, and a whole failure class).
- No $\sigma_G$ machinery for sweep-side charts: sweeps are pole-free;
  canonical-side $\sigma_G$ reduces to the recognized-carrier list
  (sphere/cone excision is booked, not built, in v1 — those pairs keep the
  landed analytic path).
- No homotopy stack (theory §10.1: mixed volume, polyhedral tracking,
  endgames — ~3–5k avoided), no deflation (~2k avoided), no Whitney/Verdier
  machinery (offline, ~0 in-tree). Theorem E makes these unnecessary on the
  normal path; they are the theory's tail, not its Phase 1.
- Classifier and decision algebra: **zero new logic** (see §1 rows) — the
  audit's single largest downward correction versus a naive "booleans are
  hard everywhere" prior.

## 3. Packet plan

| Packet | Class | Content | Write set | Depends | LOC (prod+tests) |
|---|---|---|---|---|---|
| `BIE-000-CONTRACT` | design | `InteractionOutcome` mapping onto `Refusal::NumericallyUnresolved` witness + evidence rows in `docs/CERTIFICATE_MAPPING.md`; carrier decision for §8.1; restricted-pair scope freeze; unit-shape fixtures marked as contract | `truck-certified/src/construct/bie/` stub, mapping rows | — | 0.3k |
| `BIE-001-ARITHMETIC` | design | Outward-rounded interval operations; box interval type; mean-value/Taylor-model range bounds for F over 4-D boxes consuming `enclose_der`; Bernstein where polynomial; property tests vs brute sampling | `truck-certified/src/interval/` (new) | 000 | 1.5k |
| `BIE-002-SSI4` | design | σ/σ_G; four-subset column choice; (R′) minor-sign via `Expansion`; boundary seeding (N=3 square systems on E×F, F×E); parallelotope continuation (θρ step); `Unresolved` elsewhere. Gate: bicubic/known-curve suite; differential vs OCCT on transverse pairs | same module + `truck-evidence` | 001 | 3.5k |
| `BIE-003-CARRIER` | design | `CertifiedImplicitIntersectionCurve` edge carrier; canonical enum ripple (integrator-owned); `EdgeSampleLedger` integration (PL at tessellation only); swept-face AABB bounds for the BVH broad phase | `truck-geometry/src/canonical.rs` ripple, `truck-base/src/bvh.rs` additive | 000 | 1.3k |
| `BIE-004-CLOSURE` | design | Theorem E polar exclusion; slope diagnostic (§5.4); escalate-iff-predicted-cost scheduler; planted-interior-loop battery | BIE-002 module | 002 | 1.2k |
| `BIE-005-ARRANGE` | design | Chart-curve carrier (certified PL/B-spline in 2-D) in `Carrier2D`; FF-arc + Region2 containment equivalents in (s,v); inter-curve crossings; Lemma-F simplicity as a test oracle | `truck-geometry/src/arrange.rs` additive | 002, 003 | 2.5k |
| `BIE-006-CLASSIFY` | design | Lift/path adapters for `BoundedStratum::Sweep`; classifier seeds from arrangement cells; assembler emits windowed sweep faces; provenance rows | `truck-shapeops/src/boolean/*` + `contact/mod.rs` (hot file — serial) | 003, 005 | 1.4k |
| `BIE-007-GATES` | mechanical | χ valuation + mod-2 homology over the output complex; differential battery vs the landed canonical booleans (congruence with `boolean_m2` results); corpus + mutation batteries | `truck-shapeops` new module + tests | 006 | 2.0k |

## 4. LOC estimate

| | prod | tests | total |
|---|---|---|---|
| BIE-000 | 0.2k | 0.1k | 0.3k |
| BIE-001 | 0.9k | 0.6k | 1.5k |
| BIE-002 | 2.2k | 1.3k | 3.5k |
| BIE-003 | 0.8k | 0.5k | 1.3k |
| BIE-004 | 0.7k | 0.5k | 1.2k |
| BIE-005 | 1.5k | 1.0k | 2.5k |
| BIE-006 | 0.8k | 0.6k | 1.4k |
| BIE-007 | 0.6k | 1.4k | 2.0k |
| **Total** | **~7.7k** | **~6.0k** | **~13.7k** |

**Range and sensitivities.** Realistic band **11–17k total**:

- Downward (toward ~11k): if chart trimming reuses `Region2` wholesale
  (BIE-005 shrinks), and if sweep-side boundary seeds reduce to the landed
  FE stratum machinery (BIE-002 shrinks). Both plausible; neither assumed.
- Upward (toward ~17k): the face-tangency retry loop (theory §3.5/§13.3 —
  expected bug source), shared-face consistency across adaptive subdivision
  (theory §13.4 item 1), and corpus scale.
- Sequencing credit: landing CC-001/CC-003/CC-020/CC-030 first reuses the
  banded solve, argmin, and k-contact/continuation machinery — realistically
  another 15–20% off BIE-002. This program should be sequenced AFTER the CC
  solver chain, not in parallel with it (shared hot files, shared machinery).

**Comparison:** the CC program is ~20k across 12–13 packets; BIE is ~13.7k
across 8 — but with a harder per-line verification burden (every predicate
carries a certificate). The theory's own 1:8 theory-to-plumbing ratio is
consistent with these numbers: the theory is ~1.5k of it; the rest is
arithmetic quality, bookkeeping, and batteries.

## 5. Gates

- **Typed outcomes only**: no code path returns an uncertified shape;
  `Unresolved` is first-class and carries κ/cell/slope.
- **Zero new refusal arms** expected (mapping onto the landed taxonomy per
  BIE-000); a violation is a SPEC_GAP.
- **Completeness batteries**: planted interior loops (Theorem E), no-loop
  property (Theorem B) asserted on every cover, differential suite vs the
  landed canonical booleans (`boolean_m2` congruence — 7/256-face agreement
  preserved bit-for-bit on the canonical pairs).
- **Determinism**: identical ordered input → identical verdicts (§7 house
  rule); no output ordering from hash iteration.
- **V5 identity guard**: existing entry points bit-identical; the canonical
  × canonical path is never regressed by the restricted additions.
- Face-tangency retry terminates or escalates to a distinct typed outcome
  (§13.3 is an open risk — the battery must measure it).

## 6. Deliberately not built (and why)

| Item | Theory ref | Why not |
|---|---|---|
| Sparse homotopy / polyhedral start systems | §10.1 | Theorem E makes it a failure-path-only tool; defer until it fires on real data |
| Deflation + isosingular sets | §6.2 | Restricted scope: singular pairs route to the landed analytic cells or `Unresolved` |
| Whitney/Verdier stratification | §6.5 | Offline ground-truth tooling; never hot path |
| NURBS coefficient-space systems | §1.1 caveat | F is directly evaluable for the restricted pairs; polynomial form adds spurious-component machinery for no benefit |
| Discriminant projection / intent snapping | §8.2 | `[cnj]`; needs the dirty-STEP corpus pressure the restricted scope doesn't have |
| General SSI (arbitrary non-canonical pairs) | — | Post-v1; the program's own Phase-1 unresolved-rate measurement decides |

## 7. Restricted-case disposition (with the showcases)

- **Waterslide**: chute×pool union avoided by authoring the pool as the
  sweep's terminal stations (integral, landed today); tower is a
  non-touching assembly part. BIE not required for this model.
- **Amphora**: chip authored into the foot silhouette; handles attach via
  CC-030 root blends. BIE not required.
- **Teapot**: the one genuine union (spout/body, handle/body). Short term:
  assembly emission with recorded contact intents. BIE's restricted pairs
  exist precisely to certify this merge end-to-end when it lands.

## 8. Definition of completion

- A `Sweep × Canonical` Boolean returns `Certified` with windowed sweep
  output faces and provenance, `Unresolved` with κ/cell/slope, or a typed
  refusal — measured on the teapot junction pair and a transverse corpus.
- The canonical × canonical path is bit-identical to today's (`boolean_m2`
  green, unchanged).
- The validity gate layer runs on every output; homology mismatches are
  FAILED, never warnings.
- The unresolved rate on the transverse corpus is recorded — that number
  chooses the theory's Phase 4 branch.
