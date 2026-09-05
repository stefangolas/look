# Certified Tangency and Exact Contact — Build Spec (CTE program)

**Status:** proposed program, written this session. Root theory: the ratified
*Certified Tangency and Exact Contact* specification (T1 singular-SSI closure,
T2 coincident-carrier Boolean classification), to be archived verbatim at
[`CERTIFIED_TANGENCY_EXACT_CONTACT_SPEC.md`](CERTIFIED_TANGENCY_EXACT_CONTACT_SPEC.md)
**before W0 dispatch** — packets quote its § numbers and theorem labels
(T1.0–T1.8, T2.1–T2.5) as normative text. This document expands the theory
into packets, write sets, and LOC, per the BIE booking pattern.

House rules apply: anchors re-derived by command before quoting in a packet;
kernel changes only through the packet loop (`loop/ORCHESTRATOR.md`);
`vendor/truck/**` is off-limits to direct editing; new `Refusal`/evidence arms
are SPEC_GAPs booked in `docs/CERTIFICATE_MAPPING.md`, never drive-by edits.

**Sequencing against the in-flight BIE program.** CTE consumes the landed
general-pair SSI engine (`truck-certified/src/ssi.rs`) that BIE deliberately
leaves as its "general-pair tail"; the two programs are write-disjoint there.
But **BIE-006-CLASSIFY owns `truck-shapeops/src/boolean/*` and
`truck-evidence/src/contact/mod.rs`** (hot-file rule), and CTE-006/CTE-007
write in the same directory. Therefore CTE-006 and CTE-007 are ordered
**strictly after the BIE-006 merge**. CTE-000..005, 008 have no BIE file in
their write sets and may run before or alongside BIE waves.

## 1. Substrate audit (theory element → landed machinery → disposition)

| Theory element | Substrate status | Anchor | Disposition |
|---|---|---|---|
| §1 `F = W2N1 − W1N2` rational tensor-Bernstein over `(u,v,s,t)` | **LANDED verbatim** — `SquareSystem3` + `construct_square_system` | `ssi_types.rs:75`, `ssi.rs:917` | Reused unchanged; CTE never re-derives the difference form |
| Outward-rounded Bernstein hulls | LANDED (4-axis tensor form) | `hull_tensor4`/`one_d_interval` (`ssi.rs:265-371`), `CertifiedInterval` (`formal/exact.rs`) | Reused; no new interval library |
| Krawczyk operator | **Two landed**: generic const-N trait/operator; specialized 3×3 slice form | `truck-evidence/src/num/krawczyk.rs:62/86`; `krawczyk3_certificate` (`ssi.rs:691`) | CTE instantiates the **generic** trait at N=4 for `T`; the 3×3 slice form is not the spec's target |
| Four maximal minors of the 3×4 `DF` | LANDED as per-box interval enclosures | `kernel/minor_algebra.rs` | Reused for the T1.2-style box predicates; **not** sufficient as a system — CTE-003 builds polynomial chart minors |
| Second-derivative Bernstein nets | Exists but **private to kernel** on the kernel `Grid4`; SSI `Tensor4` has first partials only | `kernel/engine.rs:1128` (`grid_second_partial`), `ssi.rs:202` (`partial_axis`) | CTE-003 adapts the pattern additively onto `Tensor4` (`partial2_axis`); `kernel/engine.rs` untouched (V5 guard) |
| §2.1 chart search (18 charts) | **ABSENT** — the only coordinate selection landed is the 4-way sliced-axis F3 rule | `ssi.rs:512` (`select_continuation_coordinate`) | CTE-002; a genuinely different selection (pivot pair, not continuation axis) |
| §2.2 (H-graph) parametric interval Newton + `Ŷ ⊇ φ(Z)` | **ABSENT** — no parametric Krawczyk/Newton anywhere in the tree | (searched; `truck-evidence` operator is point-box only) | CTE-002; the program's largest new numeric piece and its critical path |
| §2.3–2.4 chart minors `M₁,M₂` as polynomials; `T=(G₁,G₂,M₁,M₂)` | **ABSENT** (minors exist only as interval enclosures per box) | `minor_algebra.rs` | CTE-003 |
| §2.5 reduced Hessian T1.3 (`J`, `λ`, `H_h` over `Ŷ×Z`) | **ABSENT** at the SSI level. The *classification logic* exists for canonical carrier pairs in a shared chart (`H = II1 − II2`, Morse extremum/saddle, A2 cusp) | `kernel/contact.rs` | CTE-004 implements T1.3 over the deflated chart for **generic** patches; `kernel/contact.rs` stays read-only (congruence battery target) |
| §2.8 definiteness with explicit μ | ABSENT; the `det2` interval pattern is the template | `kernel/contact.rs` (`det2`) | CTE-004; small |
| §2.10 A₂ (T1.7) + rank-3 continuation on `C=(G₁,G₂,q)` | Parallelotope continuation LANDED but only driven from the BIE restricted-pair setting | `truck-evidence/src/num/parallelotope.rs`, `construct/bie/ssi4.rs` | CTE-005 adapts to generic `SquareSystem3`; no new continuation algebra |
| §4 exactness (`ExactVanishingWitness`) | **ABSENT — the biggest structural hole.** No rational polynomial algebra anywhere: `Expansion` is Shewchuk float-exact *evaluation*, not ℚ coefficient algebra | `formal/exact.rs:9-31` | CTE-001: hand-rolled multivariate ℚ-polynomial type + one verifier; **no CAS dependency** (product boundary). Producers stay out of scope |
| §5.4–5.5 T2 decision shape | **Already the corrected shape** — `fragment_decision` is `{Keep{flip}, Discard}`; §5.9 truth rows are test-pinned; coincident pairs emit ONE canonical face + provenance | `boolean/mod.rs:103`; `assemble.rs:594-636` | CTE-006 is a refactor to `SideState(u8)` + exhaustion battery; zero new decision logic |
| §5.2 carrier arrangement with tangential strata (H-atom) | **ABSENT** — split arrangement handles Region2/collinear/FE events; tangential curves are not strata inputs | `boolean/split.rs` | CTE-007; gated on CTE-005's `A2BranchCurve` (the T1→T2 link) |
| §5.8 self-pair entry gate | Currently a **typed refusal**; rewrite-before-sweep is the theory's requirement | `split.rs:425`; pinned by `boolean_m2.rs` test 4 | CTE-007 replaces the refusal for certified operand identity (construction-node `EntityId` + `CoincidenceWitness`) |
| Verdict vocabulary | Landed vocabularies are trace-level (`TraceOutcome`) and contact-level (`TopoNode`, canonical carriers only) | `ssi_types.rs:310`; `kernel/graph.rs:87` | CTE-000 freezes the five-way `ContactVerdict`; mapping rows into `CERTIFICATE_MAPPING.md` |

## 2. Scope and the two corrections the spine must encode structurally

Two theory rules are enforced at the type level, not by convention:

1. **R2 (graph-enclosure evaluation).** The reduced-Hessian evaluator's
   signature takes the certified `GraphEnclosure`, never a raw box. A raw-box
   call does not compile; the §2.5 correction cannot be violated by a worker.
2. **R1 (T1.4 is not a predicate).** `det DT = det A³ det H_h` appears only in
   the termination argument and the fixture kit's symbolic identity tests. No
   runtime code evaluates it as an interval predicate; CTE-008 asserts this by
   gate (no such call site exists).

Also structurally pinned: **A₁⁻ (Morse saddle / tangential node) is a first
class verdict from day one** (correction R3). A classifier that emits
`Unresolved` for the indefinite case is a failing gate, not a scope note.

## 3. Packet plan

| Packet | Class | Content | Write set | Depends | LOC (prod+tests) |
|---|---|---|---|---|---|
| `CTE-000-SPINE` | design | The tangency shim (D-shim discipline): `Rank2Chart`, `GraphEnclosure`, `ChartMinorGrids`, `TSystem` shape, `IntervalSym2`/`Definiteness{sign, μ}`, `ExactVanishingWitness` (verifier trait + data), five-way `ContactVerdict` + `A1Cert`/`A2Cert`/`TransversalCert` per theory §3, `A2BranchCurve` (T1 output / T2 strata input; producing stub behind a pending refusal, `cone_torus` precedent), `SideState(u8)`, `CoincidenceWitness` + carrier/trim-domain/stratum shapes. Refusing constructors only — nothing evaluates. Fixture kit with hand-computable ground truths (see §5) | `truck-certified/src/tangency/{mod,shapes,fixtures}.rs`, one `pub mod` line in `truck-certified/src/lib.rs`, mapping rows in `docs/CERTIFICATE_MAPPING.md` | — | 0.8k |
| `CTE-001-QPOLY` | design | Multivariate ℚ-polynomial type (exact coefficient arithmetic, no external deps); the **one** `ExactVanishingWitness` verifier: expand both sides over ℚ, compare coefficients, plus interval nonvanishing of `ŝ` (and `â`) — theory §4.2. Provenance producer wired as the fast path; normal-form/RUR producers stub behind the pending refusal (§6.1 is deliberately open). Property tests: randomized identity instances, verifier rejects perturbed identities | `tangency/qpoly.rs`, `tangency/witness.rs` | 000 | 1.2k |
| `CTE-002-GRAPH` | design | Lemma T1.0 as a bounded search: 18-chart enumeration, certified `0 ∉ det Â(B)`; the (H-graph) parametric interval Newton/Krawczyk certifying unique `y = φ(z)` for every `z ∈ Z` plus the graph enclosure `Ŷ`; named refusals (no-chart, graph-failure). This is the critical path — fixture-first | `tangency/chart.rs`, `tangency/graph.rs` | 000 | 2.0k |
| `CTE-003-MINORS` | design | Additive `Tensor4::partial2_axis` (second-partial Bernstein nets, adapted from `kernel/engine.rs:1128` — engine.rs untouched); polynomial chart-minor grids `M₁, M₂`; the `T=(G₁,G₂,M₁,M₂)` system as a `KrawczykSystem<4>` instantiation; the T1.5 five-equation exclusion driver (subdivision + square-subsystem Krawczyk exclusion, fail-closed bisection) | `tangency/minors.rs`, `tangency/tsystem.rs`, `tangency/exclude.rs`, additive method in `ssi.rs` (sole writer) | 000 | 1.7k |
| `CTE-004-HESSIAN` | design | T1.3 reduced-Hessian evaluator over `Ŷ×Z` (R2 rule in the signature); §2.8 interval definiteness/indefiniteness with Gershgorin and closed-form μ; the A₁ arms — T1.6a (`Empty`/`Loop` refinement), T1.6b `A1Isolated`, T1.6c `A1Node` (indefinite case REQUIRED); the five-way cascade with T1.8 termination evidence | `tangency/hessian.rs`, `tangency/cascade.rs` | 002, 003 | 1.5k |
| `CTE-005-A2` | design | T1.7 wiring: identity verified through CTE-001's verifier; `ŝ, â` nonvanishing; rank-3 separation of `D(G₁,G₂,q)` (minor-algebra pattern); nonemptiness (`BranchSeed` via Krawczyk on `(G,q,coord)` / `BoundaryCrossing`) — the R6 correction, vacuous satisfaction refuses; rank-3 parallelotope continuation adapter `C=(G₁,G₂,q)` over generic `SquareSystem3` | `tangency/a2.rs` | 001, 002, 003 | 1.3k |
| `CTE-006-SIDEALG` | mechanical | `SideState(u8)` refactor of `MaterialState4`/`fragment_state`; coordinatewise algebra of §5.4; §5.9 truth rows by exhaustion as property tests; `KeepBothSplit` semantics pin. **Ordered after BIE-006** (hot file) | `truck-shapeops/src/boolean/mod.rs` (sole writer) | — | 0.4k |
| `CTE-007-T2ARRANGE` | design | `CoincidenceWitness` consumption (provenance-first via construction-node identity); carrier arrangement with tangential contact curves as strata (H-atom), consuming `A2BranchCurve`; containment predicate (interval separation of trim-boundary parameter boxes + one certified interior seed, §5.6); self-pair entry-gate rewrite (`split.rs:425` refusal → rewrite before sweep, §5.8); atom emission = single canonical geometry + provenance set (§5.5) | `tangency/arrange.rs`, `truck-shapeops/src/boolean/{split.rs, assemble.rs}` (sole writer of both) | 001, 004, 005, 006 | 2.0k |
| `CTE-008-GATES` | mechanical | Differential battery vs landed `boolean_m2`/`conformance_battery` (read-only, V5 guard); T1.8 termination battery; A₁⁻ node battery (chamfer-crossing-boss fixture family — the R3 regression gate); T1.1/T1.4 symbolic identity suite as permanent regression; determinism gate; no-T1.4-predicate gate (§2 above); the program's ONE full verify at integrated HEAD | new test files + `tangency/gates.rs` evidence rows | 007 | 2.0k |

## 4. LOC estimate

| | prod | tests | total |
|---|---|---|---|
| CTE-000 | 0.5k | 0.3k | 0.8k |
| CTE-001 | 0.7k | 0.5k | 1.2k |
| CTE-002 | 1.2k | 0.8k | 2.0k |
| CTE-003 | 1.0k | 0.7k | 1.7k |
| CTE-004 | 0.9k | 0.6k | 1.5k |
| CTE-005 | 0.8k | 0.5k | 1.3k |
| CTE-006 | 0.2k | 0.2k | 0.4k |
| CTE-007 | 1.2k | 0.8k | 2.0k |
| CTE-008 | 0.4k | 1.6k | 2.0k |
| **Total** | **~6.9k** | **~6.0k** | **~12.9k** |

**Range and sensitivities.** Realistic band **10–16k**:

- Downward (toward ~10k): if the parametric Newton reduces to a
  parameterized form of the landed `krawczyk` operator with thin glue
  (CTE-002 shrinks toward ~1.2k), and if the T1.5 exclusion driver mostly
  prunes by single-equation interval separation before bisection.
- Upward (toward ~16k): graph-enclosure tightness forcing adaptive
  sub-charting (the known failure mode of hull-over-hull evaluation), and
  arrangement certification effort in CTE-007 if trim domains arrive
  non-dyadic.
- The theory's §7 claim that "all hot-path work is small" is consistent with
  these bands: the heavy fallback (witness **production**) is deliberately
  out of scope, not under-estimated.

## 5. Fixture kit (frozen in CTE-000, normative for every wave)

Each fixture is a hand-computable ground truth machine-checked at admission;
wave workers build against these, never against an upstream sibling's
production code (BIE-000 pattern).

- **F1 (T1.4 identity).** The theory §9 instance: `h = 3z₁² − 5z₂²`,
  `det A = 1` ⇒ `det DT = det A³·det H_h = −60`. Asserted as an exact
  identity test — the termination argument's permanent regression.
- **F2 (T1.1 identity).** Random rational charts with residual exactly 0 for
  both `j` (the §9 verification record, re-derived in-tree).
- **F3 (A₁⁺).** Plane × rational sphere chart (stereographic, landed
  rational-carrier form) at a tangential contact with known `z*`: isolated
  contact, definite `H_h`, `F⁻¹(0) ∩ B = {x*}` (T1.6b).
- **F4 (A₁⁻).** Plane `z=0` × rational saddle bipatch `z = x² − y²`: node at
  the origin, `det Ĥ_h < 0` certified, two transverse arcs (T1.6c). This
  fixture is the R3 gate.
- **F5 (A₂).** Plane `z=0` × extruded parabola bipatch `z = u₁²` (degree
  (2,1)): contact along a line, `q = u₁`, `s = a = 1` — the T1.7 witness is
  hand-written and the verifier must accept it.
- **F6 (transversal).** Plane × plane crossing with an inflection-straddling
  box where every individual minor changes sign: T1.5(1) must certify
  `Transversal` where minor separation cannot (the theory's sharpening
  example).
- **F7 (T2 rows).** Coplanar Identical/Anti pairs — reuse the landed
  `boolean_m2` fixtures read-only; the exhaustion battery covers the rest.

## 6. Gates

- **Typed outcomes only**: `Unresolved{κ, cell}` is first-class; no code path
  returns an uncertified shape.
- **Zero new top-level evidence kinds** expected — verdicts and refusals map
  onto the landed vocabularies (`SsiRefusal`, `TraceRefusal`, kernel
  `Refusal`) per CTE-000's `CERTIFICATE_MAPPING.md` rows; a violation is a
  SPEC_GAP.
- **A₁⁻ is not `Unresolved`**: the F4 battery asserts the node verdict; a
  classifier that refuses the indefinite case fails the program.
- **R1 gate**: no runtime evaluation of the T1.4 identity as an interval
  predicate (grep-level + review gate in CTE-008).
- **R2 gate**: the Hessian evaluator's signature accepts only `GraphEnclosure`
  (type-level, CTE-000).
- **One verifier** (theory §4.2): producers never verify; the verifier never
  produces. Two instantiations (A₁, A₂), one code path (CTE-001).
- **Determinism**: identical ordered input → identical verdicts; no output
  ordering from hash iteration.
- **V5 identity guard**: `boolean_m2.rs`, `conformance_battery.rs`, and every
  landed test name byte-identical; the landed transversal/analytic contact
  paths are never regressed (differential congruence vs `kernel/contact.rs`
  `TopoNode` on canonical pairs, where both apply).

## 7. Deliberately not built (and why)

| Item | Theory ref | Why not |
|---|---|---|
| Witness **producers** beyond provenance (normal form over ℚ, local RUR, triangular decomposition) | §4.3, §6.1 | The verifier is total and provenance-independent; production runs only on cells that reach stages 4–6 with an inconclusive sign. Build when a measured unresolved rate demands it |
| A₃ and higher contact (cusps, degenerate-indefinite critical points) | §6.2 | The theory's own scope: `Unresolved` is the correct verdict |
| Non-manifold carriers shared by ≥3 operands | §6.3 | The algebra generalizes; the arrangement certification does not — booked open |
| Tangential carrier contact of positive codimension | §6.4 | Requires T1 `A2Branch` completeness on incident faces; strata *booking* lands (CTE-007), production stays open |
| Face coalescing of cosmetically split coplanar faces | §6.5 | Cosmetic, interacts with provenance merging; separate pass, never needed for a certified Boolean |
| pyo3 binding translation over this surface | — | Booked and deferred behind the CG core (`AGENTS.md`) |
