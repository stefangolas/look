# CONTACT ATLAS BUILD SPEC — the certified Boolean volume architecture

Status: PROGRAM SPINE (owner-directed 2026-09-14). Normative design:
`docs/SLOW_SOLVER_DIAGNOSTICS_R4.md` (measured bottleneck) and the
owner-supplied integrated specification "Certified Boolean Volume by
Contact Atlas, Exact Region Flux, and Best-First Residual Refinement"
(filed beside this spec as `docs/CONTACT_ATLAS_SPEC.md`). This document is
the packet-authoring spine: waves, frozen contracts, write-set pre-matrix,
integration order, and the acceptance battery. Per the kernel program law,
all kernel changes flow through packet/worker/verify; `vendor/truck/**` and
the truck123d kernel sources change only via dispatched packets.

## 0. What this program changes, and what it may not

REPLACES (as primary accuracy mechanism): the measure-2 contact-cell
refinement of `certify_boolean_volume_impl` phase 3 — demoted to a sound
fallback for isolated events and unsupported trace configurations.

ADDS: co-support algebraic resolution; certified contact atlas + trace
tubes; hierarchy-first subtree retirement; certified BVH pair search;
interaction-graph fold decomposition; rigid-placement volume memoization;
best-first residual scheduling; decision-directed gates.

INVARIANTS (no packet may weaken these):
- Soundness: every returned bracket contains the true volume (Theorem 4).
- The oracle policy: kernel certificates are the certification; OCC
  references stay diagnostics.
- Refusal semantics: typed refusals with the current sound bracket; a wide
  bracket is reported, never guessed.
- Determinism: bit-reproducible per platform (spec §25, P17).
- V5 net: every row/test green at a wave's base stays green at its tip.

## 1. Acceptance battery (the program's definition of done)

Run at the integrated HEAD, once (one-verify amendment holds):

1. The three CEILING rows (`f1/power_unit`, `f1/suspension_rear`,
   `hypercar/lighting`) complete with sound certificates under the 120 s
   ceiling, measured with `loop/nightly_ops/row_run.py` (py-spy profile
   retained for the record).
2. Full R4 census re-run (`loop/nightly_ops/r4_census.py`): verdict
   invariants — all 20 GREEN rows stay GREEN; no new DNF; refusal codes
   only narrow.
3. The existing kernel battery stays green: `rational_flux` (11/11),
   `fuse_fold`, `placement_cache`, `cert_cost_scale`, `surface_residue`
   (10/10), `green_trims` (7/7), `planarity_router` (5/5), plus
   G16/G17's `admission_pairs_narrow` and `trim_prism_extrude_admission`
   (they proceed independently of this program and fold into the battery).
4. Determinism spot-check: one boolean dispatch re-run twice in-process;
   interval endpoints bit-identical (P17).
5. Phase-stats observability: every certificate carries the spec's
   counters (pairs, excluded, clear/contact cells, max depth, chart
   count, trace spans, retired subtrees) — the diagnostics surface this
   program was partly motivated by.

## 2. Wave plan

Each wave freezes contracts before its packets dispatch; waves 2–4 packets
serialize on the shared write sets (below) but are authored in parallel.
One verification battery at the end; scoped `cargo check -p` + named tests
between merges.

### Wave 0 — contract shim (one packet, normal loop)

`CT-000-CONTRACT-SHIM`. No solver bodies. Lands:
- the residual-object model: `TraceSpanResidual`, `FallbackCellResidual`
  with `Phi`, `width`, canonical id, and the certified-refine interface;
- the global scheduler skeleton (best-first, canonical tie-break, budget
  freeze/refuse policy, gate trait: numeric absolute/mixed + decision
  predicates of Theorem 5A);
- the co-support oracle contract type:
  `NoPositiveAreaOverlap | Resolved(decomposition, orientations) |
   UnknownPositiveAreaOverlap` (maps to refusal
  `CoincidentSupportUncertified`);
- the typed refusal vocabulary of spec §26 (adds the Trace*/Arrangement*/
  `CoincidentSupportUncertified` codes to the landed refusal marshal);
- the deterministic arithmetic contract statement (P17: rounding mode, FMA
  policy — no contraction on interval endpoints, canonical tie-breaks);
- shared fixture kit: analytic-ground-truth compositions (abutting boxes,
  known cylinder cuts, nested groups) with exact volumes.
Acceptance: shim types compile; existing battery green (pure addition);
fixture ground truths machine-checked.

### Wave 1 — soundness kernel + scheduler over the existing solver

Packets (serial on `bd_bridge.rs`):
- `CT-100-SCHEDULER-WRAP`: wrap the EXISTING phase-3 refinement in the
  residual scheduler; global width gate (Theorems 25/27) replaces the
  per-patch budget split as the acceptance test. Theorem 14/15/22/23
  constants become recorded per-call stats. This alone is shippable and
  de-risks everything after: same soundness, better scheduling, full
  observability.
- `CT-101-SELECTOR-SEMANTICS`: discharge P1 as a conformance test — the
  selector table (Theorem 6) verified against the landed fold on ground
  truth fixtures (union/intersect/difference × parallel/antiparallel
  co-support).
- `CT-102-FALLBACK-LAWS`: discharge P14/P15 — the fallback cell width law
  `w ≤ K h²` and cover law `M(h) ≤ A_d h^-d` are measured and certified
  for the existing subdivision path (they formalize what the code already
  does; K and A_d become recorded certificate fields).
- `CT-103-DECISION-GATES`: Theorem 5A threshold/comparison early-exit at
  the scheduler level (cheap, immediately useful to the census tooling).

### Wave 2 — hierarchy, broad phase, fold decomposition (structural wins, lowest theory risk)

- `CT-200-BVH-PAIR-SEARCH` (P5): certified lower bounds per BVH node pair;
  branch-and-bound exactness (Theorem 8). Replaces the Θ(|A|·|B|) scan.
- `CT-201-HIERARCHY-RETIREMENT` (P18): per-node aggregate flux annotation
  `F(U)` (one bottom-up interval sum) + certified whole-node membership
  (`EntireInside/EntireOutside/Unresolved`) with subtree retirement
  (Theorem 8A). Reuses the landed membership primitive; worst case
  unchanged, typical case removes most per-patch work.
- `CT-202-INTERACTION-GRAPH` (P19): the fold's may-overlap graph +
  union-find components (Theorem 8B); singleton components skip boolean
  work; cross-component refinement exactly zero.
- `CT-203-RIGID-MEMO` (P20): operand-volume certificate cache keyed by
  immutable construction identity (Theorem 8C). The Falcon fold's repeated
  own-volume phases collapse (4f).

### Wave 3 — co-support algebra (unlocks spline_loft×spline_loft)

- `CT-300-CO-SUPPORT-ORACLE` (P4): the three-way oracle. Planar
  coincidence certificate from control data (interval evaluation of the
  supporting-plane equation over control hulls + polygonal overlap of the
  parameter rectangles). Unknown ⇒ typed refusal
  `CoincidentSupportUncertified` — never a guess.
- `CT-301-CO-SUPPORT-TABLE` (P1 extension): Theorem 6's orientation table
  applied to Resolved regions; the shared region contributes per the table
  and exits the refinement queue entirely (exact cancellation where
  orientations oppose). This is the production unlock: the deck-rail fuse
  and the nine body-in-white rows.

### Wave 4 — contact atlas + trace tubes (the exponent change)

Ordered sub-waves; each keeps covering fallback sound (spec §26 fallback
policy: an incomplete trace pass is discarded, never evidence of clear):
- `CT-400-TRANSVERSALITY-MARGIN` (P6): certified `μ_tr` (smallest singular
  value margin) per admitted pair; admission switches from the scalar `τ`
  gate to the rank margin.
- `CT-401-CHARTS` (P7): parametric Krawczyk chart certificate C1 + C0
  exclusion predicates; deterministic marching-coordinate selection.
- `CT-402-TOPOLOGY` (P8): the T1–T5 obligations — chart cover, continuation
  adjacency, component closure, projection coverage, junction isolation.
- `CT-403-TRACE-TUBES` (P9/P10/P11): derivative enclosures through order
  k+1 (seam-splitting at knots — no continuation across), Hermite/Taylor
  approximant with certified remainder, tube radius `ρ_i`, tube flux bound
  (12). Start k=1 (validates the machinery), then k=3 (the exponent).
- `CT-404-ARRANGEMENT` (P12): planar arrangement of projected traces +
  R1–R4; seed classification per face (Theorem 16).
- `CT-405-RESOLVED-FLUX` (P13): exact Green-contour integration for
  polynomial patches; rational case (RI1) via the landed FHC-G1
  reciprocal-power machinery. Hardest obligation — scheduled last within
  the wave, with the polynomial path shippable alone.
- `CT-406-ISOLATED-EVENTS` (P19 Theorem 19): the O(A₀ log) event path for
  vertices/junctions the tracer isolates.

### Wave 5 — integration + battery

- `CT-500-INTEGRATION`: composition of all waves at the integrated HEAD;
  the acceptance battery of §1; wave manifest filled (base SHAs, packet
  commits, amendment list, verifier version, final integrated SHA).
- `CT-501-SIMD-LANES` (P21, optional/last): lane-batched interval kernels
  for independent residual objects; constants only, no exponent change.
  Skip if the battery already meets the 120 s ceiling — measure first
  (Amdahl).

## 3. Write-set pre-matrix

| packet | bd_bridge.rs | membership module | new test file | door/fixtures |
|---|---|---|---|---|
| CT-000 | no | no | contract types crate/module | yes (fixtures) |
| CT-100 | yes | no | scheduler conformance | no |
| CT-101 | yes | no | selector table tests | no |
| CT-102 | yes | no | fallback law tests | no |
| CT-103 | yes | no | decision gate tests | no |
| CT-200/201/202/203 | yes | yes | one per packet | no |
| CT-300/301 | yes | yes | co-support tests | no |
| CT-400–406 | yes | yes | one per packet | no |
| CT-500 | yes | yes | battery harness | yes |

`bd_bridge.rs` is the serialization spine (every dispatch change lands
there). Within a wave, packets serialize; across waves, only the frontier
packet holds the file. Consider extracting the facts/dispatch plumbing
into a new module early (CT-000 may name it) so later waves touch
disjoint files — decide at the spine, not per packet.

## 4. Integration order and merge discipline

Dependency order within the integrated HEAD: contract shim → scheduler →
selector semantics → fallback laws → BVH → hierarchy → graph → memo →
co-support → margin → charts → topology → tubes → arrangement → resolved
flux → events → battery. Between merges: `cargo check -p truck123d
--tests` + the merged packet's own tests (scoped; no full battery until
Wave 5). Every seam mismatch returns to the owning worker as an amendment
(`--resume`), never a redispatch that redoes proven math.

## 5. Standing rules imported unchanged

ORCHESTRATOR.md applies verbatim: no gate loosening; RESULT.json is a
claim; anchors re-measured at dispatch (H-8); cargoq for every cargo
invocation; the RAM cap arithmetic before raising worker count; slots
poll, never wait. The 120 s row ceiling (owner directive) is the program's
runtime acceptance line; anything above it after Wave 5 is a CEILING
verdict booking the next theory (the spec's §14 lower-bound hypotheses are
where to look).

## 6. Wave manifest (filled as packets land)

base SHA, packet id, worker commit, merged-as, amendments, verifier
version, per-packet obligation map (which P-obligations each packet
discharges), final integrated SHA. Lives in `docs/CONTACT_ATLAS_MANIFEST.json`.
