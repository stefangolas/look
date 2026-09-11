# Theory spec: rank-deficient contact

**Status:** spec for implementation. Landed 2026-09-10 (owner-directed);
supersedes and retires the unpublished draft `RANK_DEFICIENT_THEORY_PLAN.md`.
**Inputs:**
- `SOLVER_COVERAGE_AUDIT.md` (v1 lattice, 653 rules)
- `loop/solver_coverage/checker.py`
- `docs/SOLVER_COVERAGE_SPEC.md`

**Claim tags.** Every mathematical claim carries one of these:

| tag | meaning |
|---|---|
| **[LIT]** | Established in the cited literature. |
| **[OBL]** | Proof obligation. A sketch is given; a written proof is a deliverable. |
| **[ASM]** | Modelling assumption. It must be confirmed against the codebase before use. |

Nothing tagged [OBL] or [ASM] may be cited as a certificate until it is discharged.

**Hard constraint.** No tolerances, snapping, repair or merging heuristics anywhere. Every threshold in this spec is a *certificate gate*: it selects which proof applies and never changes a reported answer.

---

## 0. Summary for the implementer

The v1 audit shows `relation.zero_set = rank_deficient(residual)` as a sink for `volume_bracket` in every relation dimension. It is also a sink for `no_intersection`, but those states are mostly goal-false rather than undecided. This spec closes the gap in four parts, in this order:

1. **Checker fixes first (§2).** The current numbers mix refuted, undecided and geometrically impossible states. Fix the accounting before measuring progress.
2. **Volume without classification (§4).** Near a tangency, both surfaces are graphs over a common plane. The uncertain volume is the thin sandwich between them, bounded by area × maximum separation, whatever the contact type. This closes the audited gap by itself.
3. **Topology through witnesses first, numerics second (§5).** Exact tangency is decided from exact evidence: quadric pencils, shared carriers, or construction. A numeric tier handles the cases that are certifiable numerically. Everything else becomes a named refusal.
4. **Named refusals for the residue (§6).** Every rank-deficient cell ends in either a certified class or a stable refusal tag.

Do not start kernel work before milestone M1 (§9) passes. M1 runs the checker on the proposed rules and confirms they close the gap in the model.

---

## 1. Problem statement

### 1.1 Audited gap

Uncovered rank-deficient states in the v1 audit:

| goal | uncovered states | status | nature |
|---|---:|---|---|
| `volume_bracket` | 11,520 | THEORY GAP | A real gap. No rule reaches a volume-proving state. |
| `no_intersection` | 11,520 | THEORY GAP | Mostly goal-false or resolvable by exclusion. Needs classification, not proof. |

`local_contact`, `complete_locus`, `material_class` and `mesh` already accept or cover rank-deficient states. `valid_brep` covers all of them except 960 PARTIAL states in 2x1.

**Retrodiction witness:** `2x2 / rank_deficient(residual) / local_dim 0 / local_only(residual) / volume_bracket`. This corresponds to swept-operand `cut` / `fuse`.

### 1.2 Root cause

The v1 `volume_bracket` predicate requires `zero_set ∈ {certified_empty, regular}`. It hard-codes transversality, so no rule can ever satisfy it from a rank-deficient state. The fix changes what counts as volume evidence (§2.6, §4). It does not try to refine the zero set.

### 1.3 Out of scope

These gaps are out of scope here but must be reported:

| goal | states (tightened lattice) | description |
|---|---:|---|
| `volume_bracket` | 5,760 | 1x1/2x1 knowledge-lifting gap |
| `no_intersection` | 5,760 | 1x1/2x1 `unknown` vertical gap |
| `valid_brep` | 1,920 | 2x1 PARTIAL |

None of these involve tangency. After this work, completeness for curve relations is the largest remaining theory gap.

---

## 2. Checker changes (milestone M0)

All changes go in `loop/solver_coverage/checker.py` and `docs/SOLVER_COVERAGE_SPEC.md`.

**CHK-1: three-way status.** Report each state as PROVED, REFUTED or UNDECIDED. Add refutation predicates for each goal:

| goal | refuted when | tag |
|---|---|---|
| `no_intersection` | any certified nonempty zero set | — |
| `local_contact` | `zero_set = certified_empty` | — |
| `no_intersection` | `regular`, if `regular` means nonempty | [ASM] — see CHK-4 |

**CHK-2: tighten T_geom.** Each constraint is an [ASM]. Confirm it with the kernel owner, then add it.

| constraint | reason |
|---|---|
| `src_dims ∈ {1x1, 2x1} ⇒ local_dim ≤ 1` | Two curves, or a curve and a surface, cannot meet in a 2-dimensional set. |
| `src_dims ∈ {1x1, 2x1} ∧ zero_set = regular ⇒ local_dim = 0` | Full-rank contact between curves is isolated points. |
| `zero_set = certified_empty ⇒ local_dim` collapses to a single value | Local dimension is meaningless for an empty set. |

Report state counts both before and after tightening.

**CHK-3: two coverage metrics.**
- *Strict coverage:* the winning route has no refusal branch.
- *Fail-closed coverage:* every outcome either proves the goal or refuses with a named tag.

The current semantics compute fail-closed coverage and call it coverage. Rename it, and add the strict metric.

**CHK-4: pin the meaning of `regular`.** Decide whether it means "nonempty and regular" or "regular where nonempty". Record the decision in the spec. The routes through `TANGENCY-CASCADE-REFINE-EMPTY`, which turn `2x2 / regular` into `certified_empty`, are valid under the second reading only. Re-audit them after the decision.

**CHK-5: new axes and values (lattice v2).**

| axis | values | notes |
|---|---|---|
| `relation.regime` | `unknown(residual)`, `transversal`, `tangential`, `singular_param` | New. Set by admission (§3). |
| `relation.volume_evidence` | `none(residual)`, `sandwich_bounded` | New. Set by the sandwich rule (§4). |
| `relation.zero_set` | New children under `rank_deficient`: `tangent_point`, `tangent_crossing`, `tangent_curve`, `tangent_higher`, `coincident`, `near_degenerate(residual)` | Set by the classifier (§5). |

Each new child fixes `local_dim`:

| child | `local_dim` |
|---|---|
| `tangent_point` | 0 |
| `tangent_crossing` | 1, with a singular point on the curve |
| `tangent_curve` | 1 |
| `tangent_higher` | 0 |
| `coincident` | the smaller source dimension |
| `near_degenerate` | residual |

**Naming note.** In Arnold's classification, A_k refers to isolated singularities: A1 is Morse, A2 is the cusp. Morse–Bott contact is non-isolated. If the codebase's "A2 Morse–Bott" label is kept, document the convention in `tangency/a2.rs`. Otherwise rename it to `tangent_curve` semantics.

**CHK-6: revised goal predicates.**

`volume_bracket` is proved when `knowledge ∈ {all_components, complete_locus}` **and** either of:
- `zero_set ∈ {certified_empty, regular}` (the existing clause), or
- `volume_evidence = sandwich_bounded` (new).

`valid_brep`, `complete_locus`, `local_contact` and `mesh` accept the new zero-set children with a determined `local_dim`. They do not accept `near_degenerate`.

**CHK-7: knowledge lift.** Whether `E-VOLUME-TANGENTIAL-SANDWICH` may raise `global.knowledge` to `all_components` for the cells it covers is proof obligation OBL-K (§4.5). Until OBL-K is discharged, run the checker in both modes and report both.

**CHK-8: fragment E.** Encode every rule in §7 as fragment E with `confidence: proposed`. Use the same schema as fragments A–D; the implementer must read the existing schema first. Proposed rules must never merge into production coverage numbers. Report them as a separate projection.

**CHK-9: reachability.** Extend the §4 routing check in the audit so that every new rule is shown to be reachable from the door/bridge path. A rule that is present in the model but unreachable counts as a routing gap. This is the RG-1 to RG-9 failure mode.

---

## 3. Admission: regime dichotomy

### 3.1 Surface–surface (2x2)

**Lemma D [OBL].** Take a cell pair (U, V) of regular patches, meaning ∂u × ∂v is certified nonzero on both. Let the unit-normal cones have half-angles θ_P and θ_Q about centre normals a and b. Let φ = ∠(a, b) and Θ = θ_P + θ_Q. If Θ < π/4, at least one of the following holds:

- **(T) Transversal.** φ − Θ > 0 and φ + Θ < π. Every pair of normals is non-parallel.
- **(G) Tangential.** φ + Θ < π/2, or φ − Θ > π/2. Every pair of normals has |n_P · n_Q| > 0.

*Sketch.* Every angle between a normal of P and a normal of Q lies in [φ − Θ, φ + Θ]. Split into three cases:
- φ ≤ π/4 gives (G);
- π/4 < φ < 3π/4 gives (T);
- φ ≥ 3π/4 gives (G) with anti-parallel normals.

*Lineage [LIT].* This belongs to the normal-criterion tradition of loop detection: Sinha et al. (parallel normals), Sederberg–Meyers (a line normal to both surfaces), and Hohmeyer (separability of the Gauss maps). `ADMISSION-WHOLE-BOX-REGULAR-CONE` probably already computes the cones. If so, the dichotomy is one extra branch on an existing test, not a new stage.

**Termination [OBL].** For regular patches, normal-cone half-angles tend to zero under subdivision. Admission therefore terminates in (T), (G), or `refuse(SingularParametrization)` wherever ∂u × ∂v cannot be certified nonzero.

**Tie rule.** Shallow transverse crossings can satisfy both (T) and (G). If (T) certifies, use (T); otherwise use (G). The two paths must agree on every region that both certify. Add a consistency test for this.

**Graph injectivity (G-INJ) [OBL].** (G) makes each patch a *local* graph over the plane Π perpendicular to a. It does not make the patch injective. For example, a low-pitch spiral ramp has near-vertical normals everywhere but overlaps itself under projection.

*Required sufficient condition:* the parameter cell is convex, and every matrix in the interval hull of the 2×2 Jacobian of π ∘ P over the cell is nonsingular. The same must hold for Q. Injectivity follows from the interval mean-value form f(x) − f(y) = J̃(x − y), where J̃ lies in the hull.

A (G) cell that fails G-INJ is subdivided further. If it cannot be certified, it refuses.

### 3.2 Lower dimensions

| relation | (T) gate | (G) gate | h |
|---|---|---|---|
| 2x1 (edge vs face) | \|t · n\| > 0 | ‖t × n‖ > 0, i.e. the edge is nearly in the tangent plane | Scalar on a 1D interval. |
| 1x1 (trim curves in a face domain) | \|t₁ × t₂\| > 0 | \|t₁ · t₂\| > 0 | Scalar on a 1D interval. |
| 1x1 (curves in 3D) | DG has rank 2 | \|t₁ · t₂\| > 0 | A 2-vector on a 1D interval. |

Analogous dichotomy lemmas are required for each row [OBL]. Confirm which measure `volume_bracket` consumes for curve relations before writing their sandwich rules [ASM]. Presumably it is trim-arrangement areas feeding the flux bracket.

---

## 4. Volume in the tangential regime

### 4.1 Set-up

In a (G) cell that satisfies G-INJ:
- write both patches as heights z_P and z_Q over their projections into Π;
- let R = π(P(U)) ∩ π(Q(V));
- let h = z_Q − z_P on R.

h is a local chart of the oriented distance function of one surface from the other, the implicit representation of the intersection used by Kriezis–Patrikalakis–Wolter [LIT].

### 4.2 Sandwich lemma

**Lemma S [OBL].** Let S\* be the true Boolean result. Let S̃ be a candidate solid constructed as follows:

- Outside the cylinders R × ℝa over undecided pairs, S̃ uses the certified boundary.
- Inside each such cylinder, S̃ uses a single graph (either z_P or z_Q), closed by vertical walls over ∂R.

Then S\* Δ S̃ is contained in the union of the sandwiches {points between z_P and z_Q over R}. Consequently

  |vol S\* − vol S̃| ≤ Σ_pairs |R| · sup_R |h|.

If the enclosure of h on R excludes 0, the pair is certified disjoint over R, and the cell is resolved.

*Sketch.* Inside a (G) cylinder, a point's membership in each operand is decided by which side of each graph it lies on. Those sides are fixed by the certified orientation. Outside the sandwich, the two membership tests agree for every trim choice.

*Sub-obligations:*

| id | obligation |
|---|---|
| OBL-S1 | S̃ is a closed solid, and the flux bracket of the existing contact-cover solver extends to it so that the sandwich term is additive. |
| OBL-S2 | Every point of ∂A whose classification against B is uncertified lies in the sandwich cylinder of some undecided pair. This is a coverage-of-ambiguity statement. |
| OBL-S3 | No other face of either operand passes through a cylinder inside its sandwich. Otherwise, split the cell or refuse. |

*Coincidence.* Coincidence is the *easy* case for volume: h ≡ 0, so the contribution is zero. This requires exact evidence of the common carrier (§5.2 W2). Without it, see the floor in §4.4.

### 4.3 Enclosures (range functions)

The sandwich rule needs only two things per cell:
- a range enclosure of the scalar a · P over U, and of a · Q over V;
- an area bound on R.

Everything else is shared, so the rule is parameterised by a range function.

**Fast path: Bernstein control-net slabs.** For a rational Bézier cell with positive weights, a · P is a convex combination of the control-point heights a · p_i. So:

  h ∈ [min_V a·Q − max_U a·P, max_V a·Q − min_U a·P].

|R| is bounded by the area of the intersection of the projected control-net bounding boxes. This needs no chart inversion, and subdivision is de Casteljau.

The fast path is only available when `rep.bernstein_chart = true` and `rep.rational_positive_weights = true`. Convex-hull bounds have approximation order 2 [LIT: Schulz 2009]. In the tangential regime, each patch varies only to second order along a, because a is within O(w) of its normals near contact. So the sandwich term is O(w²) per unit area [OBL].

**Fallback: generic interval range function.** This is **mandatory**. Use a mean-value form or, preferably, a Lagrange or Hermite range form [LIT: recent work reports these outperform Taylor forms in subdivision]. It applies to every rep-flag combination.

> ⚠ **Regression trap.** A control-net-only implementation fires on only about 1/4 of rank-deficient volume states, because the fast path needs two rep flags. The fallback is what makes the rule total. The checker must model the rule with both paths, and tests must cover a non-Bernstein or non-positive-weight case.

### 4.4 Termination, rate and floor

**Lemma T [OBL].** Call a (G) pair *undecided* when its h-enclosure contains 0. Assume the range function converges as cell width w → 0. Then

  Σ_undecided |R| · sup|h| ≤ A · max_undecided (enclosure width),

where A is the total face area. The right-hand side tends to zero. No classification, curvature constant or transversality is used.

**Rate [OBL].** With second-order enclosures:
- a tangent curve of length L costs about L·μ·w³;
- an isolated tangent point costs about μ·w⁴;

where μ bounds the second derivatives of h. μ is used for the schedule and cost model only, never for correctness. Any "+O(w³)" term that appears in a certificate must be an enclosed remainder.

**Floor.** With directed rounding, enclosure widths stop shrinking at f ≈ c · u · scale, where u is unit roundoff.

| contact | limiting undecided area | achievable bracket width |
|---|---|---|
| isolated point or curve | → 0 | → 0 (the floor is harmless) |
| coincidence without exact carrier | the overlap area | ≥ overlap area × f |

For the second row, return `refuse(CoincidenceWithoutExactCarrier{floor})` whenever the requested ε is below the floor.

**Schedule.** Keep a max-priority queue of undecided pairs keyed by |R| · sup|h|, with ties broken by cell id for determinism. Refine the top pair. Stop when the total is at most ε; otherwise return `refuse(BudgetExhausted{achieved_width})`.

Refinement operator: subdivision, accelerated by clipping (§8).

### 4.5 Knowledge lift

**OBL-K.** Undecided pairs are covered exhaustively: any component hidden inside a pair, such as a tiny loop, lies within its sandwich bound. Therefore the sandwich rule may raise `global.knowledge` to `all_components` for the region it covers, for the purposes of `volume_bracket` only.

Until OBL-K is proved, CHK-7 reports both modes.

---

## 5. Topology: degeneracy classifier

This section is needed for `valid_brep`, `complete_locus` and the Boolean's output B-rep. It is **not** needed for volume.

### 5.1 Principle

Exact tangency is a positive-codimension condition. An interval enclosure can show |h(c)| < ε but never h(c) = 0 without additional exact information. For algebraic inputs, exact zeros *can* be certified numerically using evaluation or separation bounds [LIT: Burr–Choi–Galehouse–Yap], but explicit bounds are often unavailable or pessimistic. Treat this route as a last-resort witness (W4), not the default.

Order of attempts:
1. Witness tier (§5.2).
2. Numeric tier (§5.3).
3. Named refusal (§6).

### 5.2 Witness tier (exact)

| id | source | certifies | notes |
|---|---|---|---|
| W1 | Pairs of canonical quadric carriers (plane, sphere, cylinder, cone), via `rep.canonical_carrier` and `rep.exact_implicit` | Every class, exactly | Use exact classification of quadric pencils [LIT: Dupont–Lazard–Lazard–Petitjean, Parts I–III; Levin]. Tori are quartic and excluded; they go to the numeric tier. |
| W2 | Shared source entity or identical carrier, via importer `FaceProvenance` / `SourceEntityId` or exact equality of carrier definitions | `coincident` | Equality is decided in exact rational arithmetic on definitions. Never by sampling. |
| W3 | `rep.construction_witness` from fillet or blend construction, where the contact curve is known by construction | `tangent_curve` | The construction must emit the certificate, not merely a flag. |
| W4 | Evaluation or separation bounds on rational input | `tangent_point` / `tangent_crossing` (h(c) = 0 decided) | **Off by default.** Budgeted. STEP decimals are exact rationals, so the route is available in principle. Degree makes it expensive. |

### 5.3 Numeric tier

**System.** Use the collinear-normal system in the original parameters (u, v, s, t). Do **not** invert a chart.

  (P − Q) · P_u = 0,  (P − Q) · P_v = 0,  Q_s · (P_u × P_v) = 0,  Q_t · (P_u × P_v) = 0.

Solutions are points joined by a common normal line, which are the critical points of h [LIT: the collinear-normal criterion of Sederberg–Meyers].

Unlike the squared-distance critical-point system, this system does not vanish identically along a transversal intersection curve. There the normals are not parallel, so the last two equations fail. Its non-degeneracy at nondegenerate contacts is [OBL].

**Isolation.** Clip to isolate (§8), then certify a unique simple root with Krawczyk. Resolve the existing conflict row `TANGENCY-TSYSTEM-FROM-DEFLATED` before reusing deflation.

**Classification.** At a certified critical point c:
- the signed separation is h(c), measured along the common normal;
- the relative Hessian is the difference of second fundamental forms in a common tangent frame, plus O(h) terms [OBL].

| numeric outcome | class | lattice value |
|---|---|---|
| No root of the system in the cell, and the h-enclosure contains 0 | Shallow transverse curve; hand to the tracer in the (G) chart | `regular` |
| Hessian definite, h(c) certified to have the Hessian's sign | Empty in this cell | `certified_empty` |
| Hessian definite, h(c) certified to have the opposite sign | Single small closed transverse loop | `regular` |
| Hessian indefinite, h(c) certified nonzero | Two disjoint transverse branches | `regular` |
| Enclosure of h(c) contains 0, no witness | Near-degenerate | `refuse(NearDegenerate{hint, margin})` |
| Hessian singular, or a 1-dimensional root set (Krawczyk fails), no witness | Higher-order or tangent curve suspected | `refuse(HighOrderJet{k})` |

**Class vocabulary.** Before finalising the lattice-v2 children, cross-check this table against the four fundamental cases of Cheng et al. (TOG 2023) and the case list of Yang–Jia–Yan (TOG 2023). Adopt their enumeration where it is finer. Record any divergence.

**Existing pieces to route.**
- `tangency/a2.rs`
- `TANGENCY-SHAPES-DEFINITENESS-{DEFINITE,INDEFINITE,SINGULAR}`
- `SSI-TRACE-BRANCH-TANGENT`
- `ADMISSION-WHOLE-BOX-REGULAR-CONE`

Compare each against the collinear-normal formulation before rewriting it.

### 5.4 Tracing tangential curves

**Seeding [LIT].** For polynomial patches, a tangential contact curve starts and ends on patch borders unless it is a closed loop (Hu et al., cited in Mukundan et al.). Seed tangent curves from boundary contacts. Closed tangent loops need a witness (W1–W3).

**Tracing [LIT].** Validated interval ODE tracing handles tangential intersections without substantial overhead (Mukundan–Ko–Maekawa–Sakkalis–Patrikalakis 2004). This is a candidate for the tracer in `tangent_curve` cells.

### 5.5 Boundary tangency

Tangency on a seam, knot-span boundary or domain edge covers 3 of the 4 incidence values. Initially it returns `refuse(BoundaryTangency{incidence})`.

Follow-up: Yang–Jia–Yan report handling intersections along boundaries. Before adopting their method, determine whether it is certified or tolerance-based. Under the no-tolerance rule, adopt only certified parts.

### 5.6 Lower dimensions

In 1D the classifier reduces to root analysis of a scalar h and its derivative h′ on an interval:
- simple zeros are transverse points;
- double zeros are tangent points;
- higher-order zeros are `tangent_higher`;
- h ≡ 0 means overlap or edge-on-face, which needs a witness.

Use clipping (§8) with Krawczyk certification.

---

## 6. Named refusals

Every rank-deficient cell ends in a certified class or one of these tags. Tags are stable strings, and refusals fail closed. The inventory already has `stable_refusal_diagnostic_tag`.

| tag | payload |
|---|---|
| `SingularParametrization` | Cell where ∂u × ∂v cannot be certified nonzero. |
| `GraphNotInjective` | Cell failing G-INJ after the subdivision budget. |
| `NearDegenerate` | Class hint (definite / indefinite / unknown) and enclosure of h(c). |
| `HighOrderJet` | Jet order k where the certificate failed. |
| `CoincidenceWithoutExactCarrier` | Bracket floor. |
| `BudgetExhausted` | Achieved bracket width and requested ε. |
| `BoundaryTangency` | Incidence value. |
| `EvidenceConflict` | The conflicting certificates. |

Refusals never count toward strict coverage (CHK-3).

---

## 7. Fragment E: proposed rules

Encode these for M1. Exact field names follow the existing fragment schema. Precondition sets are written in lattice-v2 terms.

| rule id | preconditions | outcomes |
|---|---|---|
| `E-ADMISSION-REGIME-DICHOTOMY` | `src_dims=2x2`; `zero_set ∈ {unknown, rank_deficient(residual)}`; `regime=unknown` | `regime=transversal` \| `regime=tangential` \| refuse `SingularParametrization` \| refuse `GraphNotInjective` |
| `E-ADMISSION-REGIME-DICHOTOMY-1D` | `src_dims ∈ {2x1, 1x1}`; same zero-set condition | as above, 1D gates (§3.2) |
| `E-VOLUME-TANGENTIAL-SANDWICH` | `regime=tangential`; goal `volume_bracket`; any rep flags | `volume_evidence=sandwich_bounded` (+ `knowledge=all_components` if OBL-K) \| refuse `BudgetExhausted` \| refuse `CoincidenceWithoutExactCarrier` |
| `E-TANGENCY-WITNESS-QUADRIC-PENCIL` | `zero_set=rank_deficient(residual)`; `canonical_carrier=true`; carriers ∈ quadrics | any §5.2 class, exact |
| `E-TANGENCY-WITNESS-SHARED-CARRIER` | `zero_set=rank_deficient(residual)`; provenance or carrier equality evidence | `coincident` |
| `E-TANGENCY-WITNESS-CONSTRUCTION` | `zero_set=rank_deficient(residual)`; `construction_witness=true` | `tangent_curve` |
| `E-TANGENCY-NUMERIC-COLLINEAR-NORMAL` | `regime=tangential`; `zero_set=rank_deficient(residual)` | `certified_empty` \| `regular` \| refuse `NearDegenerate` \| refuse `HighOrderJet` \| refuse `BoundaryTangency` |
| `E-TANGENCY-NUMERIC-1D` | `src_dims ∈ {2x1, 1x1}`; `regime=tangential` | `certified_empty` \| `regular` \| refuse (as above) |
| `E-TANGENCY-WITNESS-ZERO-BOUND` | numeric tier refused `NearDegenerate`; rational input; W4 enabled | `tangent_point` \| `tangent_crossing` \| refuse `BudgetExhausted` |

**Checker modelling notes.**
- The sandwich rule has no rep-flag precondition. The fast path versus the fallback is an implementation detail and must not narrow the rule in the model.
- The numeric tier cannot emit `tangent_point`, `tangent_crossing`, `tangent_curve` or `coincident`. Only witnesses can.

---

## 8. Refinement and runtime

| component | method | why |
|---|---|---|
| Range enclosure | Bernstein control net (fast path); Lagrange/Hermite form (fallback); Taylor only if neither applies | Convex-hull bounds have order 2 [LIT]. Lagrange/Hermite forms beat Taylor in subdivision [LIT]. |
| 1D root refinement near tangency | Cubic clipping | Convergence rates 4 / 2 / 4/3 for simple / double / triple roots [LIT: Liu et al. 2009]. Newton and Krawczyk degrade to linear at double roots. |
| 2D root refinement | Bivariate quadratic clipping, then Krawczyk once isolated | Rate 3 for single roots proved; about 1.5 for double roots experimentally [LIT: Bartoň–Jüttler]. |
| Sandwich schedule | Priority queue on \|R\| · sup\|h\| | Spends refinement where the bracket width actually is. |
| Witness tier | O(1) per face pair, except W4 | Exact classification of quadric pencils is efficient [LIT]. |
| W4 (separation bounds) | Off by default, budgeted | The only potentially exponential component. |

**Expected cost** [OBL; measure in M2]:
- about L · (Lμ/ε)^{1/3} undecided cells for a tangent curve of length L;
- O(1) cells for isolated contacts after isolation.

**Determinism.** Every queue and tie-break is ordered by (key, cell id). Output must be bit-reproducible across runs.

---

## 9. Milestones and acceptance

| milestone | deliverable | acceptance |
|---|---|---|
| **M0** | CHK-1 to CHK-4 | Re-run the audit with refuted/undecided split and tightened counts, plus a strict-vs-fail-closed table. The CHK-2 and CHK-4 [ASM]s are confirmed or amended in the spec. |
| **M1** | Lattice v2 (CHK-5, CHK-6), fragment E (§7), CHK-7 dual mode | Under fragment E, the `volume_bracket` rank-deficient UNDECIDED count is 0 (fail-closed) in lift mode. The retrodiction witness cell flips to PROVED. The residual gaps in §1.3 are reported, not hidden. |
| **M2** | Lemmas D, S (S1–S3), T written up; `E-ADMISSION-REGIME-DICHOTOMY` and `E-VOLUME-TANGENTIAL-SANDWICH` implemented for 2x2 with **both** range-function paths; reachable from the door (CHK-9) | Fixtures V1–V6 pass (§10). Bracket width vs. budget curves are published. Swept `cut` / `fuse` cells return a certified bracket or a named refusal. |
| **M3** | Witness tier W1–W3 for 2x2 | Fixtures T1–T5 give correct exact classes and a valid B-rep. The `valid_brep` and `complete_locus` rows are re-audited. |
| **M4** | Numeric tier for 2x2, then 1D rules; refusal tags | Fixtures T6–T9 pass. No rank-deficient cell ends without a class or a named tag. |
| **M5** | Corpus measurement over the STEP and build123d corpora | Prevalence per class, per witness route and per refusal tag. Decide on W4, boundary tangency and higher-order jets from the data. |

**Projected checker numbers** (for orientation only, not acceptance criteria). Tightened lattice, fail-closed, from the v1 tables. These depend on the CHK-2 [ASM]s.

| stage | undecided (all goals) | `volume_bracket` proved |
|---|---:|---:|
| v1 today | 22.3% | 41.3% |
| M0 | 18.7% | 38.5% |
| M2, no knowledge lift | 16.5% | 53.8% |
| M2, with lift | 13.2% | 76.9% |
| M3 + M4 | 7.7% | 76.9% |

---

## 10. Fixtures

True volumes are analytic where possible. Every volume fixture asserts four things:
- the true volume lies inside the bracket;
- bracket width decreases monotonically with budget;
- output is deterministic;
- the refusal tag is correct wherever the expected outcome is a refusal.

**Volume fixtures (M2)**

| id | configuration | expected |
|---|---|---|
| V1 | Sphere resting on a plane (exact contact), union and cut | Bracket → true volume. Tangent point. |
| V2 | Cylinder resting in a larger cylinder, internally tangent along a line | Tangent-curve cost ≈ w³ per unit length. |
| V3 | Coaxial equal-radius cylinders, shared `SourceEntityId` | Zero sandwich contribution via W2. |
| V4 | Same as V3 with no shared provenance and floating-point radii | `CoincidenceWithoutExactCarrier`, floor reported. |
| V5 | Same as V1 in a non-Bernstein or zero-weight representation | Fallback range function used. Bracket still certified (regression trap). |
| V6 | Retrodiction case: swept `cut` / `fuse` tangent to a canonical face | Certified bracket. |

**Topology fixtures (M3–M4)**

| id | configuration | expected |
|---|---|---|
| T1 | V1 | `tangent_point` via W1 |
| T2 | V2 | `tangent_curve` via W1 |
| T3 | V3 | `coincident` via W2 |
| T4 | Fillet face vs. its neighbour under a Boolean | `tangent_curve` via W3 |
| T5 | Sphere vs. cone, tangent | Class via W1 |
| T6 | Sphere lowered into a plane by 1e-6 | Certified small transverse loop (numeric) |
| T7 | Sphere raised above a plane by 1e-6 | `certified_empty` (numeric) |
| T8 | Sphere raised by 1e-15 | `NearDegenerate` for topology; V1-quality volume bracket |
| T9 | Bicubic patch z = x⁴ + y⁴ against a plane | `HighOrderJet` without a witness; `tangent_higher` with W4 enabled |

---

## 11. Open questions (implementer reports back before M1)

1. What is the exact schema of the fragment rows, and where do `!unmodeled` preconditions go for the new rules?
2. How does the contact-cover volume solver's flux bracket compose with an additive sandwich term? This bears on OBL-S1.
3. Which measure does `volume_bracket` consume for 1x1 and 2x1 relations?
4. Does `regular` mean nonempty (CHK-4)?
5. Do the CHK-2 constraints hold for every representation the kernel admits?
6. Does `ADMISSION-WHOLE-BOX-REGULAR-CONE` already expose normal cones in a form Lemma D can reuse?
7. What should the door expose when volume is certified but topology is refused in named cells?

---

## 12. Non-goals

- General singularity theory beyond the classes the witness tier can certify.
- Any tolerance-based snapping, merging, or overlap extraction within an error threshold. Tolerance-based overlap methods, including Yang–Jia 2025, are explicitly rejected.
- Changes to the transversal path beyond the admission split.
- The curve-relation completeness gap in §1.3. It is next in line, but it is a separate spec.

---

## References

Bartoň, M., Jüttler, B. Computing roots of polynomials by quadratic clipping. *CAGD* 24 (2007) 125–141.

Bartoň, M., Jüttler, B. A quadratic clipping step with superquadratic convergence for bivariate polynomial systems. *Mathematics in Computer Science* (2011).

Burr, M., Choi, S. W., Galehouse, B., Yap, C. K. Complete subdivision algorithms, II: Isotopic meshing of singular algebraic curves. *J. Symbolic Computation* 47(2) (2012) 131–152.

Cheng, J.-S., Zhang, B., Xiao, Y., Li, M. Topology driven approximation to rational surface-surface intersection via interval algebraic topology analysis. *ACM TOG* 42(4) (2023).

Dupont, L., Lazard, D., Lazard, S., Petitjean, S. Near-optimal parameterization of the intersection of quadrics, Parts I–III. *J. Symbolic Computation* 43(3) (2008).

Hohmeyer, M. E. A surface intersection algorithm based on loop detection. *IJCGA* 1(4) (1991) 473–490.

Hu, C.-Y., Maekawa, T., Patrikalakis, N. M., Ye, X. Robust interval algorithm for surface intersections. *CAD* 29(9) (1997) 617–627.

Kriezis, G. A., Patrikalakis, N. M., Wolter, F.-E. Topological and differential-equation methods for surface intersections. *CAD* 24(1) (1992) 41–55.

Li, K., Yang, J., Jia, X. Advances and challenges in surface–surface intersection computation — an overview. *CAD* 193 (2026) 104039.

Liu, L., Zhang, L., Lin, B., Wang, G. Fast approach for computing roots of polynomials using cubic clipping. *CAGD* 26 (2009) 547–559.

Mukundan, H., Ko, K. H., Maekawa, T., Sakkalis, T., Patrikalakis, N. M. Tracing surface intersections with validated ODE system solver. *ACM Solid Modeling* (2004).

Plantinga, S., Vegter, G. Isotopic approximation of implicit curves and surfaces. *SGP* (2004).

Schulz, C. Bézier clipping is quadratically convergent. *CAGD* 26(1) (2009) 61–74.

Sederberg, T. W., Meyers, R. J. Loop detection in surface patch intersections. *CAGD* 5(2) (1988) 161–171.

Yang, J., Jia, X., Yan, D.-M. Topology guaranteed B-spline surface/surface intersection. *ACM TOG* 42(6) (2023).

Yang, J., Jia, X. Overlap region extraction of two NURBS surfaces. *ACM TOG* 44(6) (2025). Cited as rejected; tolerance-based.

"Bivariate range functions with superior convergence order." arXiv:2604.14400 (GMP 2026).
