# FSSI build spec — certified SSI topology on the landed funnel

**Status:** file authored 2026-09-07 ~8:44 PM in one burst with its packets
(mtime evidence); its header previously attributed it to session 54 — that
attribution is unverified and treated as superseded. **Re-verified against
the tree 2026-09-08 (orchestrator):** `SsiRefusal` lives in
`truck-certified/src/ssi.rs:97` — the original draft cited
`ssi_types.rs:97-116`, which is the CFP wave shim and holds no such enum.
Citations corrected below; packets must anchor on the measured substrate. Implements the external theory
spec "Fibered SSI (FSSI) — Corrected Theory Specification, revision 2" as
packets on this loop, **revised against the audit**: every FSSI item that is
already landed is cited by landed name (workers must not re-implement it);
every item that collides with booked work is deferred or re-scoped to extend,
never replace.

Siblings: [`CONTACT_FAST_PATH_BUILD_SPEC.md`](CONTACT_FAST_PATH_BUILD_SPEC.md)
(the funnel and shared discipline this program sits inside),
[`TORUS_CONTACT_PROGRAM.md`](TORUS_CONTACT_PROGRAM.md) (owns the torus pair
family; TOR-C is this program's first external consumer),
[`CERTIFIED_TANGENCY_BUILD_SPEC.md`](CERTIFIED_TANGENCY_BUILD_SPEC.md) (the
tangency cascade — FSSI-002 is a new tier of its escalation lattice),
[`CONSTRUCTIVE_GEOMETRY_PLAN.md`](CONSTRUCTIVE_GEOMETRY_PLAN.md) (unaffected:
the CG fast path stays SSI-free by doctrine).

**Entry condition.** FSSI-001-GATE may be authored and dispatched immediately
(its write set is disjoint from every live packet). FSSI-002 and FSSI-003 are
**gated on the PB-011C-TTC-PARITY-CHURN census** (the swept×swept 4-D wave):
its stagnation/refusal records are the measurement plan §14 of the theory
spec, already booked — this program does not re-measure what PB-011B/C
records. If the census shows mostly certified lifts, FSSI-002/003 shrink and
the reductions (FSSI-006/007) become the next program; if it shows
stagnation-dominant rows, the fold tier is the priority. This replaces the
theory spec's §14 with the booked census.

---

## 0. What is already landed (mapping table — do not rebuild)

| Theory item | Landed carrier | Citation |
|---|---|---|
| 4-D setup `F = X − Y` over compact product domain | `Ssi4System`/`Ssi3System` over `(u,v,s,t)`; `IntervalBox4` | `truck-certified/src/construct/bie/ssi4.rs:26-70`, `src/interval/box4.rs:30-110` |
| FSSI-1 cofactor coordinates (3-of-4 minor, square 3×3 solve) | exact-`Expansion` minor column choice + `Ssi3System` | `ssi4.rs:45-49` |
| FSSI-2/3 regional projection | **F3 rule** (largest relative margin, lowest index, refuse below threshold) | `src/ssi.rs:45-56`, `src/contract.rs:86-88` |
| FSSI-6 uniform parametric Krawczyk tube | dimension-generic `KrawczykSystem<N>` + parallelotope continuation | `truck-evidence/src/num/krawczyk.rs:62-90`, `src/num/parallelotope.rs:12-52` |
| FSSI-7 fold bridge | `TraceOutcome::Switched` in the branch tracer | `src/ssi_trace.rs:35-52, 188-280` |
| Bernstein range/exclusion over 4-D subboxes | `hull_bernstein_2d`, `TensorGrid4`, `bernstein_box4` | `src/hull.rs:95-117`, `src/interval/bounds.rs:111-208` |
| Event completeness substrate | `gff::cover_branch` (slab decomposition, chart-aware 2×2) | `truck-evidence/src/contact/gff.rs:1-110` |
| Tangency/singularity classification | CTE cascade: Hessian inertia, `TSystem` (N=4), five-equation exclusion | `contact/singular.rs:678-758`, `src/tangency/tsystem.rs:98-105`, `src/tangency/exclude.rs:15-76` |
| Budget-burn as certified verdict | CFP-008 stagnation route + BG-KV2-207B escalation-lattice remainder | `feat(certified)` `a7b5abd`, `7580294` |
| Exact trim-winding discipline | `kernel/trimclip.rs` (exact integer ray crossing on trim loops) | `truck-certified/src/kernel/trimclip.rs` |

Axis-order note: landed charts are `(u,v,s,t)`; the theory spec writes
`(u,t,v,s)`. All packets below use the **landed order**. The theory spec's
FSSI-6/7 sections are therefore **documentation deltas only** (folded into
FSSI-000-CONTRACT), not implementation packets.

---

## 1. Invariants (inherited + program-specific)

Inherited from the CFP spec without exception: BG-ENC-001/002/003, H-1
(no panics; new modules carry `#![deny(clippy::unwrap_used)]`), H-3, H-6,
SFC (search in floats, certify exactly), no-silent-downgrade, V5-pair/V5-pair
identity, F1 (no `truck-evidence` ↔ `truck-certified` edge), the
single-interval-algebra rule (`formal::exact::CertifiedInterval` is the only
scalar algebra in `truck-certified`), and the frozen F3 rule ("never retried
with a weaker test").

Program-specific:

| Tag | Invariant |
|---|---|
| **FSSI-EXT** | The frozen exclusions and verdict classes lift by **extending** dispatch tables, refusal enums, and the escalation lattice — never by widening a verdict or weakening a landed test (governing doctrine: `TORUS_CONTACT_PROGRAM.md` doctrine constraints). |
| **FSSI-REFUSAL** | No new variants on the base `truck_base::evidence::Refusal`. New named cases land in the layered local vocabularies (`SsiRefusal`, `TraceRefusal`, `UnresolvedWitness`) with stable `tag()` strings. Named cases only — no catch-all. |
| **FSSI-LAYER** | The gate lives at the **SSI admission layer** (`truck-certified/src/ssi*`), below the contact funnel and the classify layer. `truck-shapeops/boolean/classify.rs` is off-limits for the whole program (DEF-SEEDRAY-B owns it; its "no new refusal arm" rule is respected because the gate refuses before stratum dispatch, not inside classify). |
| **FSSI-ELEV** | Any hull of a product of enclosed factors (in particular `q_j` minor hulls) uses the **elevated pairing discipline** — the naive non-elevated pairing can emit a false loop-free certificate (`truck-evidence/src/contact/implicit2d.rs:21-30`). A reviewer must be able to find the elevation in the code. |
| **FSSI-CENSUS** | FSSI-002/003 sizing decisions cite PB-011C's RESULT.json census rows (stratum pair, budget spent, verdict class). No re-measurement packets. |
| **FSSI-TRIM** | Boundary/trim strata reuse the `trimclip.rs` exact-winding substrate (and whatever DEF-SEEDRAY-B lands for fragment faces). No parallel trim enumeration. |

**Optimization metric (inherited):** decisiveness per cost. The program's
purpose is to convert the two largest refusal classes — conditioning refusals
at ordinary folds, and un-decided tangency suspicion — into certified
verdicts, and to unlock the canonical ruled×ruled pairs as exact analytic
loci. It is not allowed to slow down already-green paths (V5-pair identity).

---

## 2. Refusal vocabulary (the complete delta)

| Theory name | Lands as | Layer |
|---|---|---|
| `TangentCurveSuspected` | `SsiRefusal::TangentCurveSuspected { margin }` — the gate's undecided-measure halt | `truck-certified::ssi` |
| `CoincidentPatchSuspected` | `SsiRefusal::CoincidentPatchSuspected { margin }` | `truck-certified::ssi` |
| `DegenerateFold` | new escalation-lattice terminal class alongside CFP-008's stagnation verdicts | `truck-certified::tangency` |
| fold certified | new lattice tier verdict `CertifiedFold { chart, sigma }` — not a refusal | `truck-certified::tangency` |
| ruled×ruled locus | `AnalyticIntersection::RuledCrossing { … }` inside the landed analytic family | `truck-evidence::analytic` |

All new variants carry `tag()` strings in the established style and are
added by extension only (FSSI-EXT).

---

## 3. Packet family

### FSSI-000-CONTRACT (design, dispatch now)

Freezes names, signatures, and the mapping table before any solver code.

- Write set: `truck-certified/src/ssi.rs` (the `SsiRefusal` enum lives here,
  line 97 — stub variants, refusing
  constructors only — the D-shim rule: types that evaluate/solve refuse
  `InvalidInput`), `docs/CERTIFICATE_MAPPING.md` (field-level mapping for
  `CertifiedFold`, per-segment projection index, tube certificates — the
  single booking surface), module docs in `src/ssi.rs` citing the theory
  spec's FSSI-6/7 landed carriers (documentation delta above).
- Tests: contract tests asserting the stub constructors refuse and the
  `tag()` strings are stable.
- Done when: `cargo test -p truck-certified --lib` green; every downstream
  packet builds against these names.

### FSSI-001-GATE (design, dispatch now — parallel-safe with PB-011B/C)

The separable tangency-free certificate (theory §1), landed at SSI admission.

- **Mechanism (corrected 2026-09-08, owner Theorem C — the original draft
  claimed the gate proves `Σ ∩ B = ∅`, which is FALSE: transverse surfaces
  can intersect inside `B`; positive normal separation certifies
  TRANSVERSALITY, not emptiness):** on box `B` with `B ∩ P = ∅`, certify a
  strictly positive lower bound on `‖n_X × n_Y‖` over `B` — mechanized as
  TWO per-surface 2-D normal cones (`cone(a, θ_X)`, `cone(b, θ_Y)` from the
  hemisphere certificate, not a 4-D normal-product field):
  `δ = min{∠(a,b), π−∠(a,b)} − θ_X − θ_Y > 0  ⇒  ‖n_X × n_Y‖ ≥ sin δ > 0`.
  **What this certifies: `rank DF = 3` at every point of `Σ ∩ B`
  (transversality — the intersection curve through `B` is regular and
  well-conditioned for continuation; no tangencies).** Emptiness of
  `Σ ∩ B` is certified by the separate Bernstein exclusion of `F` itself,
  never by this gate. Straddling zero ⇒ subdivide under the existing
  budget; on budget exhaustion, apply the **positive-dimension suspicion
  halt** and refuse `TangentCurveSuspected` (undecided measure failing to
  shrink like 2⁻⁴ᵏ) or `CoincidentPatchSuspected` (undecided measure at
  area scale). The suspicion criterion lives **only in the refusal
  decision** — it can cause a premature refusal, never a wrong acceptance.
- Write set: `truck-certified/src/ssi_gate.rs` (new), admission wiring in
  `src/ssi.rs` (`SquareSystem3` entry, and the `SsiRefusal` variants per §2 —
  the enum lives here) and `src/construct/bie/ssi4.rs`,
- Parametric-pole handling: the gate composes with the landed F3 rule —
  pole loci are excluded by the `B ∩ P = ∅` precondition, discharged by the
  selector picking the surviving coordinate (theory §4.1); the packet must
  include a loft-apex fixture proving the `q_t ≡ q_v ≡ q_s ≡ 0` case does
  **not** refuse.
- Tests (each named, no catch-all): near-tangent rank-3 pair **passes** the
  gate (conditioning cost, not correctness hazard — theory §1.3); planted
  tangent-curve fixture refuses `TangentCurveSuspected`; coplanar-patch
  fixture refuses `CoincidentPatchSuspected`; loft-apex fixture passes;
  V5-pair identity on all landed green spline pairs (admission may only
  widen what certifies, never flip a certified verdict); no-panic scan.
- Budget: {turns: 80, ctx_tokens: 200000}. ~300–500 LOC production,
  ~500–800 test.
- Note: this packet changes **no boolean outputs** — the gate is
  monotone-widening only (admits what previously refused; V5-boolean
  adjudication applies if any output diff appears, which it must not).

### FSSI-002-FOLD (design; **gated on PB-011C census**)

Ordinary-fold certification (theory §5) as a **new tier** of the CFP-008
escalation lattice.

- **Mechanism.** At a box where the landed path currently refuses
  (`ConditioningBelowThreshold` / `SsiRefusal::Conditioning` /
  `TraceRefusal::Conditioning`): form `E_j = (F, q_j)` (4 equations,
  4 unknowns, N=4 `KrawczykSystem` — the `TSystem` instantiation culture,
  never an operator edit). One Krawczyk proof with
  `det D(F, q_j)(p) ≻ 0` certifies: unique event, regular SSI point,
  ordinary fold, exact local incidence count (0 ↔ 2). Where the proof
  fails, the **existing** stagnation verdict stands unchanged — FSSI-EXT.
- **The `q_j` Jacobian is the cost center.** Interval partials of the 3×3
  minor of `DF` must come from the landed hull kernels under FSSI-ELEV
  elevation; if a new hull composition is required, it is a named,
  separately-tested kernel (added to `hull.rs`, with a randomized
  cross-check against brute-force interval evaluation).
- Write set: `truck-certified/src/tangency/fold.rs` (new), escalation
  lattice tier registration, `src/ssi_trace.rs` (refusal-site swap),
  `src/hull.rs` (only if the new composition is needed).
- Tests: synthetic fold fixture (two ruled patches with an ordinary
  turning point) certifies `CertifiedFold` with `σ` recovered; degenerate
  fold (merged pair) refuses `DegenerateFold`, never certifies; every
  previously-green path unchanged (V5-pair).
- Budget: {turns: 90, ctx_tokens: 220000}. ~400–700 LOC production,
  ~600–1,000 test.

### FSSI-003-SKELETON (design; **gated on PB-011C census + FSSI-002**)

Event completeness + boundary strata (theory §6, §10).

- **Mechanism.** Assert and use the skeleton property: interior events
  (FSSI-002 roots) + region-transition points (`M ∩ ∂B_k` for chart
  changes) + boundary events (`M ∩ ∂D` via trim strata) form a vertex set
  that every component of `M` meets. Boundary strata enumerate per theory
  §10 using FSSI-TRIM substrate; corner strata (`∂D_X × ∂D_Y`,
  overdetermined) discharge by exclusion, never by sampling.
- **Representation requirement (theory §8) lands here:** the tracer's arcs
  already carry per-step chart boxes (`TraceStep::chart_box`); the packet
  freezes the per-segment projection index on the output arc type so a
  bridge segment (`Switched`) is never re-expressed as a `z_j`-graph.
- Write set: `src/ssi_trace.rs`, `src/ssi_types.rs` (arc/vertex types),
  new `src/ssi_skeleton.rs`; `docs/CERTIFICATE_MAPPING.md` additions.
- Tests: loop fixture of diameter ε certifies both fold events and the
  closed-loop arc without a separate loop hunt; boundary-crossing fixture
  certifies the two domain-boundary events; completeness counter-test — a
  planted unreachable component makes the skeleton assertion fail loudly.
- Budget: {turns: 100, ctx_tokens: 250000}. ~700–1,200 LOC production,
  ~1,000–1,800 test. The largest packet; expect an R2 split if the stratum
  enumeration exceeds one slot (the DEF-TESS-ANALYTIC-SEAM precedent is the
  budgeting model, not a risk signal).

### FSSI-004-RULED (mechanical+; dispatch now — disjoint from 001)

Ruled × ruled exact locus (theory §9.3), booked as the `BG-ANA-002` family.

- **Mechanism.** Exact predicate `λ(t,s) = (B−A)·(d×e)` with rational
  recovery of `u, v`; the **trimmed event system** includes the
  domain-crossing curves (`u − u_min = 0`, …) and the parallel-generator
  excision (`d × e = 0` gets its own enclosure test and refusal path —
  `u,v` blow up in a neighborhood). Events are `{λ, λ_s}` in two variables;
  fixed-`t` continuation is a scalar root solve (no preconditioner, no
  wrapping term — Bernstein/Descartes via `num/roots.rs` discipline).
- Consumed through the **landed** `ContactLocus::Analytic` transverse path
  in `truck-shapeops/src/boolean/split.rs` — no splitter changes (any
  splitter change is a stop-and-file, not scope growth).
- Write set: `truck-evidence/src/analytic/ruled_pair.rs` (new) + pair
  dispatch registration; F1 note: the locus lives in `truck-evidence`,
  which the funnel consumes directly — no cross-layer edge.
- Tests: exact crossing of two linear extrusions (dyadic-exact, the
  CC-024 discipline); parallel-generator fixture refuses typed; clipped
  generator fixture certifies the domain-crossing events; all existing
  analytic pair answers bit-identical (V5-pair).
- Budget: {turns: 70, ctx_tokens: 180000}. ~400–700 LOC production,
  ~600–900 test.

### FSSI-005-INSTRUMENT (mechanical; after 001–003)

- Extends the CFP instrument with `ssi_gate_class`, `fold_class`,
  `tube_count`, `event_count` keys. **The schema is frozen** — this packet
  performs an explicit schema-version bump (both emitters,
  `truck-certified::cfp::spine` and the mirrored `truck-evidence` counter
  crate, in one packet), never a silent key addition.
- Also records the theory §12 cost split (`C_gate` / `C_events` / tube term)
  per pair — the data that sizes or kills FSSI-006/007.
- Budget: {turns: 40, ctx_tokens: 100000}. ~150 LOC.

### FSSI-006-FIBER / FSSI-007-PIPE (conditional — do not author without evidence)

Synchronized-fiber reduction (theory §9.1, with the corrected `β' ≠ 0`
hypothesis) and implicit-fiber profiles (§9.2, pipes without frames).
**Authored only if** the PB-011C census + FSSI-005 cost split shows the
fibered/ruled fraction of refusing pairs justifies the surface (theory §14,
consumed via FSSI-CENSUS). Expected ~1,500–3,000 LOC production each;
booked behind the measurement, per the promotion doctrine (measured client
need, not anticipated elegance).

---

## 4. Sequencing and write-set disjointness

```text
FSSI-000 ──┬──> FSSI-001-GATE ──┬──> FSSI-002-FOLD ──> FSSI-003-SKELETON ──> FSSI-005
           └──> FSSI-004-RULED ┘        (gated: PB-011C census)      (gated: FSSI-005 cost split)
                                        └──> FSSI-006/007 conditional

Live queue (concurrent-safe):
  PB-011B -> PB-011C (truck123d only; census feeds FSSI-002/003 gates)
  DEF-SEEDRAY-B (shapeops/boolean classify; FSSI-LAYER keeps us out)
  TOR-A/B (torus program; independent)
  TOR-C (torus×spline) — sequenced AFTER PB-011C by its own booking doc;
        consumes FSSI-001/002 as substrate. Do not book overlapping work.
```

- Concurrency cap ≤3 live per the orchestrator; FSSI-001 + FSSI-004 +
  PB-011B is the intended first wave (three disjoint write sets).
- `vendor/truck` writes only through the packet/worker/`verify.py` loop.
- The TTC timing chain and the tess/spineframe DEF r2 packets are
  unaffected and share no files with this program.

## 5. Cost envelope and risk register

Core program (000–005): **~2,100–3,400 LOC production, ~3,000–4,800 test**
— at the measured loop rate (~250 LOC/h over the trailing 12 h,
commit-derived): **~20–30 loop-hours**, i.e. the core lands in 2–3 loop
waves. With FSSI-006/007: ~8,000–13,000 total.

| Risk | Mitigation |
|---|---|
| `q_j` interval Jacobian balloons (FSSI-002) | named hull kernel, randomized cross-check, elevation audit (FSSI-ELEV); if it exceeds ~600 LOC the packet splits into FSSI-002a (kernel) / 002b (tier) |
| Skeleton strata exceed one slot (FSSI-003) | pre-booked R2 split; boundary strata land first, region-transition vertices second |
| Gate widens refusals on green corpus rows (FSSI-001) | V5-pair identity tests + the PB-011B/C census comparison: a green row that newly refuses is a **defect record**, never an accepted regression |
| Small-loop depth blow-up (theory §6 caveat) | correctness unaffected by design; cost is visible in `event_count`/`cells_visited` counters, and merged folds refuse `DegenerateFold` rather than loop |
| Schema version bump ripples (FSSI-005) | single packet owns both emitters; determinism battery re-run |

## 6. Definition of completion

- FSSI-0 through FSSI-7 of the theory spec are either landed under the
  names of §0/§2 above or explicitly deferred (FSSI-8/9, gated).
- Ordinary folds certify instead of refusing, with the stagnation verdict
  class preserved for genuinely degenerate boxes (FSSI-EXT observable in
  the lattice).
- Every component of every certified intersection curve on the corpus
  regression fixtures is seeded by a skeleton vertex — no loop hunting.
- Ruled×ruled pairs certify as exact analytic loci; parallel-generator
  pairs refuse typed.
- TOR-C can book torus×spline rows against FSSI-001/002 without new
  substrate work.
- No constructive-geometry fast-path regression: the CG program's
  no-SSI-on-the-fast-path doctrine is untouched.
- The theory spec's revision-2 deltas (Appendix C) each map to exactly one
  of: landed (§0), this program (§3), or deferred-with-entry-condition
  (FSSI-006/007, TOR-C).
