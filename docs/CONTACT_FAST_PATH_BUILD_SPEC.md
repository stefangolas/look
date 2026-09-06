# Contact Fast-Path (CFP) build spec — revision 2

**Status:** authored 2026-09-06 from direct audit of the landed contact/SSI/CTE
machinery (every anchor re-derived by command this session). Incorporates the
external review: BG-ENC-002 reclassification
([`NUM-SPLINE-ENCLOSURE-CONVERGENCE-001`](defects/NUM-SPLINE-ENCLOSURE-CONVERGENCE-001.md)),
V5 split, the shared float/certify discipline, the single-slot decision, the
∇h elevation trap, torus exclusion, refusal localization, and the instrument
packet. Spine designed for maximum worker parallelism: every packet builds
against frozen fixtures, never sibling production code.

Siblings: [`CARRIER_LIFT_BUILD_SPEC.md`](CARRIER_LIFT_BUILD_SPEC.md) (the
admission arms this program feeds),
[`CERTIFIED_TANGENCY_BUILD_SPEC.md`](CERTIFIED_TANGENCY_BUILD_SPEC.md) (the
cascade this program routes to, landed),
[`CERTIFIED_INTERACTION_ENGINE_BUILD_SPEC.md`](CERTIFIED_INTERACTION_ENGINE_BUILD_SPEC.md)
(BIE, landed).

---

## 0. Invariants and the shared discipline

| Tag | Invariant |
|---|---|
| BG-ENC-001 | `enclose(B) ⊇ {f(p) : p ∈ B}`. Over-estimation acceptable; under-estimation is the cardinal bug. **Violated today** at `boolean/assemble.rs:338-381` ([`DSC-BOUNDARY-SAMPLE-EXTENT-001`](defects/DSC-BOUNDARY-SAMPLE-EXTENT-001.md)). |
| BG-ENC-002 | Enclosure width → 0 as box → 0. **Violated today** for every spline carrier: `BSplineSurface::enclose` returns the whole-net box regardless of query (`enclosure.rs:302`; [`NUM-SPLINE-ENCLOSURE-CONVERGENCE-001`](defects/NUM-SPLINE-ENCLOSURE-CONVERGENCE-001.md)). Consequence: any subdivision loop that separates on enclosures cannot terminate on spline pairs today; `normal_cone` never shrinks under subdivision; the Unresolved provenance split is unmeasurable until CFP-001 lands. |
| BG-ENC-003 | Outward rounding; no fast-math; no FMA contraction (inari compiled `+avx,+fma`). |
| H-6 | Floats never enter evidence. The certified statement is always the box. |
| **SFC (search in floats, certify exactly)** | **The program's shared discipline, named because it recurs:** a float heuristic may *search*; the *certificate* is an exact/interval predicate computed independently of the search. Three landed instances: Krawczyk's float preconditioner (validity is Y-independent), float Newton predictors under certified correction, and Theorem 3's GJK hint under exact `Expansion` witness verification. Every float heuristic added anywhere in this program must decompose into (float search, exact verify) or it is refused at review. |
| No silent downgrade | Budget exhaustion yields `Unresolved{κ, cell, slope}`, never a guess. Refusals are **localizable**: evidence-layer refusals carry stratum-pair indices; `truck-shapeops` re-attributes to faces on the way out using the lift's index→face map. No topology crosses the boundary. |
| V5-pair | Landed canonical×canonical *pair* answers stay bit-identical (they never traverse the refactored paths). |
| V5-boolean | CFP-001 **intentionally changes** boolean outputs: the widened screen admits previously-dropped real contacts, and the change is **monotone** — boundary samples lie on the image, so `sampled box ⊆ image box ⊆ any BG-ENC-001 enclosure`; the fix can only add contact events, never remove them. Every output diff is a previously-wrong answer. **A regression suite that goes red on CFP-001 is reporting the bug being fixed.** Each diff is adjudicated and recorded, never silently accepted, never a revert trigger. |
| D1–D3 | Exact knot-insertion decomposition; non-rational carriers (weight ≡ 1); `MAX_BIDEGREE` budget. Normal nets and BVH leaves count against D3; the budget constant is revisited exactly once, in CFP-003. |
| F1 | No `truck-evidence` ↔ `truck-certified` edge. Cross-layer reach by registered hook or duplicated leaf math; duplicated leaf math carries a permanent randomized cross-check test (fixture F-C7). |

**Optimization metric: decisiveness per cost.** A cheaper-and-wider tier that
prunes the same cells is pure win; a narrower-and-slower tier is worthless
unless it flips cells the cheap tier cannot. Widening never breaks soundness;
it only costs verdicts. Float heuristics are safe anywhere under SFC.

---

## 1. Substrate facts (landed machinery; all anchors re-derived 2026-09-06)

- **Enclosure interface** — `truck-evidence/src/enclosure.rs:154-190`:
  `enclose`, `enclose_der`, `normal_cone → DirCone`,
  `immersion_lower_bound`, `as_plane`; `Box3`, `DirCone`, inari intervals.
  Canonical carrier impls are per-box exact. The spline impl (`:302`) is
  whole-net (BG-ENC-002 above).
- **Boolean entry** — `truck-shapeops/src/boolean/assemble.rs:64`:
  `boolean()` = guards (single-shell) → `sweep_contact_events` →
  `split_fragments` → `classify_fragments`/`classify_from_cells` →
  `decide_and_assemble` → `Solid::try_new`. The pair screen (`:144-189`) is a
  flat O(n·m) AABB `touches()` loop; `face_aabb`/`face_uv_box` (`:338-381`)
  derive extents from boundary samples (the DSC defect above). The
  `ee_circle_circle` skip (`:315-331`) is a **documented** v1-envelope
  decision — do not re-chase.
- **Contact funnel** — `truck-evidence/src/contact/mod.rs:258`: stages 1–5
  (C0–C2 identity/overlap → exact FF analytic closed forms → FE/EE
  reductions → sweep restricted-pair dispatch → `gff::cover_branch`). Spline
  entry via the **single-slot** `SplineSsiEntry` hook (`:588`,
  `Mutex<Option<Box<dyn SplineSsiEntry>>>) — CFP-004 resolves the slot, not
  the implementation (§4, packet row).
- **Analytic closed forms** — `truck-evidence/src/analytic/*` (BG-ANA-001-*)
  + `truck-certified/src/pair_dispatch.rs`: exact `Expansion` admission,
  D-sorted operands; 64,042 pairs ≈ 62% of corpus analytic mass; cone/torus
  arms booked DISPATCH-2.
- **SSI engine** — `truck-certified/src/ssi.rs`:
  `construct_square_system` materializes `F_k = W2·N1_k − W1·N2_k` as 4-axis
  tensor-Bernstein grids stored in `SquareSystem3 { grids, degrees }`
  (`ssi_types.rs:75`) — the materialization CFP-003 removes.
  `krawczyk3_certificate` (`:691`): interval `det3` precondition, adjugate
  inverse under directed rounding, strict-inclusion emission. F3 square
  reduction + `select_continuation_coordinate`. Grid consumers CFP-003
  carries: `ssi_trace`, `ssi_admit`, CTE `minors`/`exclude`/`tsystem` (via
  `from_system_grid`). `ssi4.rs` is **unaffected** (direct carrier
  evaluation, own systems).
- **Krawczyk operator** — `truck-evidence/src/num/krawczyk.rs`: frozen;
  generic `KrawczykSystem<N>` (point-center `f_point`, row-major interval
  `jacobian`, float `preconditioner`; `None` bisects); worklist widest-axis
  bisection under `Budget{subdiv, newton, depth}`; f64-resolution floor.
  Instantiated, never extended.
- **Branch machinery** — `ssi_trace.rs` (seed → certified step march;
  identity-recurrence loop closure; both-certificate `CoordinateSwitch`;
  `classify_branch_germ` jet ladder) and `construct/bie/ssi4.rs` (direct
  interval evaluation on carriers, first-form metric normalization, (R′)
  column choice = exact `Expansion` point sign + box interval sign
  `minor_det_sign_iv`, 8-stratum N=3 boundary seeding, N=4
  hyperplane-augmented parallelotope continuation).
- **CL admission** — `patch_admit.rs`: D1 exact knot insertion, D2 unit
  weights, D3 bidegree budget; `AdmittedPatch`/`SplinePatchStack` with source
  span rectangles (`:87-101`) — CFP-005's BVH leaves are ready-made data.
- **CTE** — `tangency/{chart, graph, minors, tsystem, exclude, qpoly,
  witness, a2, hessian, cascade}.rs`, all landed (hessian/cascade fresh).
  Five-way `ContactVerdict`; `exclude_five` cheap-first ladder; T-system
  Krawczyk<4>; reduced-Hessian A₁ arms; A₂ factorization + rank-3
  continuation. CFP-008's routing target exists.
- **Dependency edges** — `truck-shapeops → truck-evidence` exists (no new
  edge for CFP-001); F1 forbids `evidence → certified`.

**Formal I/O contract.** Inputs: `(canonical carrier witness, parameter box)`
strata — representation-derived f64 witnesses, no topology handles. Outputs:
`Outcome<ContactComplex>` — certified loci carry `Method`-tagged
`Certificate`s; every non-answer is a named typed refusal or
`Unresolved{κ, cell, slope}`, localizable per §0.

---

## 2. Spine: what freezes first, and why it maximizes parallelism

**`CFP-000-SPINE` freezes everything two or more packets need, before any
geometry lands.** The loop's parallelism rule: workers build against frozen
fixtures and frozen shapes, never against a sibling's in-flight production
code (BIE-000 pattern). The spine is what makes that possible here.

**The spine freezes:**

1. **Types and refusal vocabulary.** The sub-box hull primitive's signature
   (`span_locate → de Casteljau restrict → union hulls`); the
   localizable-refusal shape (`{cause, stratum_pair}` + the shapeops-side
   re-attribution wrapper); the instrument schema (counter names, JSON
   shape); the `SplineSsiEntry` dispatch-method signature (the slot
   resolution is decided here, not in CFP-004).
2. **The fixture kit** (normative for every wave; hand-computable ground
   truths, machine-checked at admission):
   - **F-C0** — the DSC defect counterexample: sphere-cap × plane-slab whose
     contact region is the cap apex; red under the sampled screen, green
     under the certified screen.
   - **F-C1** — the parameter-space twin: trim whose parameter extent is not
     spanned by its boundary polygon.
   - **F-C2** — the enclosure battery: randomized sub-box **containment**
     (BG-ENC-001), **width-convergence under repeated bisection**
     (BG-ENC-002 — the invariant currently broken, so this test is its
     permanent guard), and **monotonicity**
     `B ⊆ B′ ⇒ enclose(B) ⊆ enclose(B′)`.
   - **F-C3** — Theorem 3 ground truths: control-net pairs with a known
     separating direction (separated) and a known touching pair (not
     separated); the certificate is the exact `Expansion` dot-sign row.
   - **F-C4** — Theorem 4 ground truths: plane×bicubic with known `h` and
     known critical point; quadric×spline; and the **elevation-trap twin** —
     the non-elevated `∇h` hull pairing must disagree with the elevated one
     on a constructed input, pinning why the elevation step exists.
   - **F-C5** — span-BVH ground truth: a known contact span pair discoverable
     inside a 20×30-span decomposition (the 360k-cell case, pruned to a
     handful of hull evaluations).
   - **F-C6** — stagnation fixture: a designed near-tangency pair whose
     per-box cones stay overlapping at every depth, routing to the cascade
     with an A₁ verdict (not budget burn).
   - **F-C7** — F1 cross-check fixtures: randomized inputs on which the
     evidence-side sub-box hull and the certified-side per-span hull must
     agree (F1 forces duplicated leaf math; this test is what stops the
     duplicate from drifting).
3. **The one-writer map** (§4) — the file×packet matrix proving the wave
   schedule has no collisions except the declared serial pairs.
4. **Gate definitions** — V5-pair/V5-boolean, SFC, the monotone-diff
   adjudication procedure.
5. **Mapping rows** in `CERTIFICATE_MAPPING.md` for the new `Method` tags
   (implicit-reduction stage, cone-certificate verdicts).

**Spine size: ~0.5k** (types, refusing constructors, fixtures, docs) — same
shape as CTE-000.

---

## 3. Formal results (numbered for packet citation)

**Theorem 1 (additive separability under D2).** With `W₁ ≡ W₂ ≡ 1`, the
stored system's polynomial is `F = S₁ − S₂` and its 4-axis tensor-Bernstein
array over `B₁ × B₂` is `c_αβ = P¹_α − P²_β` exactly (outer product with the
all-ones factor; disjoint variable sets; no degree elevation).

**Corollary 1.1 (enclosure by parts).** `F(B₁×B₂) = S₁(B₁) ⊖ S₂(B₂)`; the
interval difference of per-side hulls is the **set-identical** enclosure to
the 4-axis de Casteljau hull. Not bit-identical — float operation ordering
differs; determinism is per-implementation, and the refactor fixes one
implementation.

**Corollary 1.2 (block Jacobian).** `J = [S¹_u | S¹_v | −S²_s | −S²_t]`;
every partial belongs to one carrier; no mixed-derivative grid;
`enclose_der` on the product chart is one-sided.

**Corollary 1.3 (minor factorization).** `M_u = S¹_v·n₂`, `M_v = S¹_u·n₂`,
`M_s = −n₁·S²_t`, `M_t = −n₁·S²_s` (all four verified by direct expansion of
the signed cofactor products).

**Corollary 1.4 (memoization).** Hull work keys on one side's box; a
subdivision visiting `M` cells drawn from `k` distinct per-side boxes pays
`O(k)` hulls per side plus `O(M)` interval arithmetic on cached values.

**Proposition 2 (rank deficiency = normal parallelism).** Under immersion,
`M_u = M_v = 0 ⇔ n₂ ∥ n₁`; all four minors vanish under the same condition.

**Proposition 2.1 (closed-form minor bound).**
`√(M_u²+M_v²) ≥ √λ_min(G₁)·‖n₂‖·sin φ`, via `‖Eᵀv‖ ≥ σ_min(E)‖v‖` for
`v ∈ span(E)` and `σ_min(E) = √λ_min(EᵀE)`. Dependency-free **only when the
normals are enclosed by their own Bernstein nets** — bidegree
`(2p−1, 2q−1)`, `~4pq` coefficients per component vs `p²q²` for the 4-D grid
(36 vs 256 at bicubic; nets count against D3). Cheaper at equal-or-better
tightness than interval `det3`; supersedes the earlier two-tier pre-exit
proposal.

**Theorem 3 (exclusion = hull separation).** `0 ∉ conv{c_αβ} ⇔ conv{P¹} ∩
conv{P²} = ∅` (Minkowski sum identity), on every subbox (de Casteljau
restriction replaces nets). Separating directions inherit downward
monotonically. Under D2, the 4-axis grid contributes **nothing** to pruning
power. Per-component interval hulls are strictly weaker at `O(n₁n₂)` vs
`O(n₁+n₂)`.
**Certificate discipline (SFC instance):** the *search* for a separating
direction (GJK, float) and the *certificate* are separate. The certificate is
`min_α λ·P¹_α > max_β λ·P²_β` decided by exact `Expansion` sign predicates —
`O(n₁+n₂)`, BG-ENC-003-clean, and the thing that inherits. GJK output is a
hint, never evidence.

**Rational generalization (staged behind any D2 relaxation).** With
`d^k_α = (N¹_{αk}, −w¹_α)`, `p^k_β = (w²_β, N²_{βk})`, `F_k` is bilinear and
`λ·c_αβ = w¹_αw²_β(λ·P¹_α − λ·P²_β)` — Theorem 3 recovers verbatim
**provided weights are certified positive on the box** (Bernstein positivity
of weight coefficients). Any D2 relaxation lands together with the positivity
admission gate (C-6), as one packet.

**Theorem 4 (implicit reduction, analytic × spline).** The contact set on the
spline chart is the zero set of `h(u,v) = g(S(u,v))` — one scalar equation,
exact Bernstein form: bidegree `(p,q)` preserved for planes (coefficients =
signed control-point distances), `(2p,2q)` for quadrics (exact Bernstein
product). **The torus is excluded explicitly** — its implicit form is
quartic, giving `(4p,4q)` (169 coefficients at bicubic) and its own D3
economics; out of scope (§7). Critical points are `∇h = 0` — a 2×2 square
system for the landed Krawczyk directly; F3 reduction, continuation-axis
choice, and `ConditioningBelowThreshold` disappear; A₁ classification is
literal scalar Morse theory. **Elevation trap (packet-normative):**
`∂h/∂u` and `∂h/∂v` have bidegrees `(p−1,q)` and `(p,q−1)`; their arrays
cannot be paired into 2-vectors until both are degree-elevated to a common
bidegree — only then is `∇h(x) = Σ B_α(x)(a_α, b_α)` a convex combination and
the hull test valid. Cheap and exact; F-C4 pins it. Loop detection:
`0 ∉ conv{∇h coefficients}` (Jordan + extreme value theorem; `h ≡ 0` on the
disk fails the test honestly). Coefficient mass at bicubic×plane: 16 vs 192.

**Krawczyk Y-independence (CFP-007 basis).** `K(Q) = m − Y·F(m) +
(I − Y·J(Q))(Q − m)` proves unique root by strict inclusion for **any**
invertible Y; Y's quality affects tightness only. Float Y is H-6-clean (the
landed operator already takes a float preconditioner; the delta is reuse
across samples). ε-inflation (Rump): test `X = x* + [−r,r]ⁿ`, double `r` on
failure, carry `r` forward. Kantorovich: with `‖Y‖ ≤ β`, `‖G″‖ ≤ γ` on a
tube, `r₀ = 1/(2βγ)` is a certified basin; Smale's point-based alternative
uses `α = βγ < (13−3√17)/4 ≈ 0.1577`.

**Gauss-map certificates (CFP-006 basis).** Per-box cone disjointness
certifies: (i) loop-freedom (Sinha parallel-normal; Sederberg's
collinear-normal strengthening; Hohmeyer's LP formulation) — every branch
meets `∂(B₁×B₂)`, so boundary-crossing seeds are complete, closing the gff
seed-completeness question; (ii) transversality (Prop 2 — the CTE cascade
unreachable by proof on those cells); (iii) a fixed continuation axis with
certified margin (Prop 2.1), so `ConditioningBelowThreshold` cannot fire
along the branch.

---

## 3a. Correctness findings carried into this program

- **`DSC-BOUNDARY-SAMPLE-EXTENT-001`** and
  **`NUM-SPLINE-ENCLOSURE-CONVERGENCE-001`** (logged): the two live enclosure
  defects, corrected by CFP-001. Mechanisms distinct — wrong points, wrong
  scale.
- **gff seed completeness** (unverified, not logged): closed by proof under
  CFP-006(i) rather than by investigation.
- **`Unresolved` propagation into the boundary rewrite**: the stage contract
  (CFP-004) states the consumer rule — typed refusal to the caller of
  `boolean()`, localizable to faces via the lift's index→face map; no
  narrowing retry, no fallback. **DECIDED 2026-09-06 (owner sign-off,
  session 54): whole-operation failure (Option A).** One resistant cell
  fails the entire `boolean()` call with the typed, localizable refusal.
  Rationale: parity with OCC (the original engine fails the whole op), native
  to the landed evidence algebra (no partial-certified result shape, no new
  evidence kind), and reversible — the refusal's localization payload keeps a
  client-layer partition strategy available later without touching kernel
  semantics. A partial-result shape (Option B) is rejected: it muddies H-6
  (a partial result can silently render as fully certified) and re-opens the
  spine's frozen vocabulary.

---

## 4. Packets, one-writer map, and wave schedule

### One-writer map (hot files)

| File | Written by |
|---|---|
| `truck-evidence/src/enclosure.rs` | CFP-001 |
| `truck-shapeops/src/boolean/assemble.rs` | CFP-001, then CFP-009 (serial) |
| `truck-evidence/src/contact/mod.rs` | CFP-002, then CFP-006 (serial) |
| `truck-certified/src/{ssi.rs, ssi_types.rs}` | CFP-003, then CFP-008 (serial) |
| `truck-certified/src/tangency/{minors,exclude,tsystem}.rs` | CFP-003 |
| `truck-certified/src/patch_admit.rs` | CFP-002 (counter), then CFP-003 (accessors) |
| `truck-certified/src/construct/bie/ssi4.rs`, `ssi_trace.rs` | CFP-007 |
| `truck-evidence/src/contact/gff.rs` | CFP-006 |
| new files (`implicit2d.rs`, `bvh.rs`, `cfp/*`) | sole writer each |

### Packets

| Packet | Wave | Class | Content | Write set | Depends | LOC |
|---|---|---|---|---|---|---|
| `CFP-000-SPINE` | A | design | Everything in §2: types, refusing constructors, fixture kit F-C0..F-C7, one-writer map, gate definitions, mapping rows. Refusing constructors only — nothing evaluates | `truck-certified/src/cfp/{spine,fixtures}.rs` (new), mapping rows | — | ~0.5k |
| `CFP-001-LIFT-ENCLOSURE` | A | correctness | (a) sub-box Bernstein hull for `BSplineSurface` **in evidence**: span-locate → de Casteljau restrict → union hulls (the shared primitive; F1 forces the certified-side duplicate, cross-checked by F-C7); replaces whole-net returns in `enclose`/`enclose_der`/`normal_cone`; (b) `face_aabb`/`face_uv_box` → per-knot-span certified boxes unioned; lift keeps the index→face map and refusal re-attribution; (c) both defect statuses → corrected | `enclosure.rs`, `assemble.rs`, both defect records | CFP-000 | 0.8–1.2k |
| `CFP-002-INSTRUMENT` | A | mechanical | Three counters: knot-span distribution on admitted carriers; pair-type histogram at stage-5 entry (the plane/quadric×spline share `f`); composed-degree vs `MAX_BIDEGREE`. Two more (cells-vs-boxes, Unresolved provenance) ride the same hook after CFP-001 | `contact/mod.rs`, `patch_admit.rs` | CFP-000 | 0.3–0.5k |
| `CFP-007-SAMPLE-CONSTANTS` | A | mechanical | Float-preconditioner reuse across branch samples (invalidate on inclusion failure); ε-inflation with carried `r`; optional Kantorovich step length from per-side net second differences | `ssi4.rs`, `ssi_trace.rs` | CFP-000 | 0.5–1k |
| `CFP-003-SEPARABILITY` | B | design | `SquareSystem3` → per-side patches; value/derivative enclosures by Cor. 1.1–1.2; minors by 1.3 from per-side normal nets (D3-counted); memoization by 1.4; carry `ssi_trace`/`ssi_admit`/CTE consumers across the accessors; `ssi4` untouched; D3 budget constant revisited here, once | `ssi.rs`, `ssi_types.rs`, `tangency/{minors,exclude,tsystem}.rs`, `patch_admit.rs` | CFP-000, CFP-002 (file order) | 2.5–3k |
| `CFP-004-IMPLICIT-REDUCTION` | B | design | New funnel stage: dispatch `(recognized analytic class, spline patch)`; `h = g∘S` per Theorem 4 (planes + quadrics; torus excluded); 2-D scalar exclusion/trace adapted from the `bezier_isect` lineage; scalar-Morse A₁; **slot decision executed**: `SplineSsiEntry` grows the dispatch method frozen at spine, one registrant routes internally; ∇h elevation per F-C4; stage contract states the localizable-`Unresolved` consumer rule | `contact/mod.rs`, new `implicit2d.rs`, `BREP_GENERATION_API.md` rows | CFP-000, CFP-002 (file order), CFP-003 (composes through separable enclosures) | 2.2–3.3k |
| `CFP-005-BRANCH-BVH` | C | design | Per-carrier BVH over `SplinePatchStack` span rectangles with certified `enclose` leaves; Theorem 3 separation under SFC (GJK hint, exact-`Expansion` witness); dyadic memoization of per-side hulls | new `bvh.rs`; consumers read-only | CFP-003 | 1.5–2k |
| `CFP-006-CONE-CERTIFICATES` | C | design | Cone disjointness → three verdicts (§3). Canonical carriers first; spline arms ride CFP-001's sub-box cones | `contact/{mod,gff}.rs`, `tangency/` verdict wiring | CFP-000, CFP-001 (spline arms), CFP-004 (file order) | 0.5–1k |
| `CFP-008-STAGNATION` | D | mechanical | Cone-overlap non-shrink between subdivision levels → route to the landed CTE cascade. Converts budget-burn `Unresolved` into A₁/A₂ verdicts. **Conditional in effect on CFP-001**: today's spline stagnation is the BG-ENC-002 artifact, not geometry | `ssi.rs`, `tangency/cascade.rs` consumer | CFP-003 (file order), CFP-006 | 0.3–0.5k |
| `CFP-009-BROADPHASE` | B/C, optional | mechanical | Spatial index over lifted strata replacing the flat `touches()` loop; construction-identity caching booked separately against edit-graph provenance keys | `assemble.rs` | CFP-001 (file order) | ~0.5k |

### Wave schedule (parallelism)

```text
Wave A (4 workers, fully parallel):  CFP-000 · CFP-001 · CFP-002 · CFP-007
Wave B (3 workers):                  CFP-003 · CFP-004* · CFP-009
Wave C (2 workers):                  CFP-005 · CFP-006
Wave D:                              CFP-008
```

\* CFP-004 starts when its file (`contact/mod.rs`) frees after CFP-002 **and**
CFP-003's enclosures land; within wave B it trails CFP-003, not the wave.

**Critical path: SPINE → SEPARABILITY → IMPLICIT → CONE → STAGNATION** — five
serial packets, the minimum the hot files force. Everything else rides
parallel to it. Total wall time ≈ critical path + one verify at integrated
HEAD after CFP-004 and CFP-005.

---

## 5. Gates

- **BG-ENC-001/002/003** tested per F-C2 (containment, convergence,
  monotonicity) — convergence is the permanent guard on the defect this
  program exists to fix.
- **SFC**: every float heuristic lands as (search, exact-verify) pairs;
  review refuses any float that is neither evidence nor a hint consumed by an
  exact check.
- **No silent downgrade + localization**: refusals carry stratum-pair
  identity; re-attribution at the shapeops boundary; zero new top-level
  evidence kinds beyond the spine's mapping rows; a seeming new arm is a
  SPEC_GAP against `CERTIFICATE_MAPPING.md`.
- **V5-pair** holds unchanged; **V5-boolean** diffs on CFP-001 are
  adjudicated, recorded, monotone-only — never a revert trigger.
- **D3**: normal nets and BVH leaves counted; budget constant touched exactly
  once (CFP-003).
- **Determinism**: fixed traversal orders (BVH axis, low-before-high,
  worklist discipline unchanged); `SplineSsiEntry` registration order made
  irrelevant by the spine's dispatch-method decision.
- One verify per merge; full battery at integrated HEAD after CFP-004 and
  CFP-005.

---

## 6. Expected effect (per-packet phase claims; end-to-end deliberately unfilled)

| Packet | Phase attacked | Phase speedup |
|---|---|---|
| CFP-001 | lift + screen soundness; **termination restoration** for enclosure-separating loops | screen-dependent; spline-heavy work expected ≥ the naive 1.1–1.3x, magnitude from CFP-002's post-fix counters |
| CFP-003 | per-cell hull math | 5–20x subdivision phase (O(deg⁴)→O(deg²) + memoization) |
| CFP-004 | pair mix | 10–50x on plane/quadric×spline pairs |
| CFP-005 | inner admissions | 10–100x on multi-span loft pairs; feasibility item |
| CFP-007 | per-sample Krawczyk | 2–4x steady state |
| CFP-006/008 | degenerate tail | seconds → ms on near-tangency (post-CFP-001) |
| CFP-009 | outer screen | assembly scale only (~1x below ~50 parts) |

**Composed end-to-end, filled only by CFP-002's data:**
`1 / [ (1−f)/g₃×₅ + f/g₄ ]` on the boolean phase, with `f` the stage-5
plane/quadric×spline share. The pairs do not all multiply — CFP-004 removes a
class from the sum, and its marginal win depends on landing before CFP-003
accelerates the path it drains. No combined figure is claimed until `f`
exists.

**Sizing: ~9.6–13k LOC** including the spine and instrument.

---

## 7. Explicitly out of scope

D2 relaxation (one packet with Theorem 3's rational generalization + the C-6
positivity gate, when corpus evidence demands); the torus implicit arm
(quartic implicit → `(4p,4q)`; its own D3 economics, excluded); AP214/AP242
presentation; multi-shell booleans (RW-MULTISHELL); construction-identity
caching (booked against edit-graph provenance keys); animation/choreography;
any edit to `num/krawczyk.rs` (frozen — instantiated, never extended);
re-litigating the `ee_circle_circle` skip (documented envelope decision,
`assemble.rs:315-331`).

---

## 8. Reference lineage

Bernstein-hull root isolation / LP on control polyhedra: Sherbrooke &
Patrikalakis 1993; the Interval Projected Polyhedron successor
(Patrikalakis & Maekawa 2002). Theorem 3 is the SSI-specific specialization —
the relevant hull lives in model space. Loop detection: Sinha et al.
(parallel-normal); Sederberg et al., CAD 1989 (collinear-normal, stronger);
Hohmeyer 1991 (Gauss-map bounding by pseudo-normal patches, LP
separability). Critical-point counting: Kriezis, Patrikalakis & Wolter
(Poincaré index; the known extremum-saddle cancellation failure). 
Topology-before-tracing: Grandine & Klein 1997. Uniqueness certification:
Moore; Krawczyk 1969; Rump (ε-inflation); Smale's α-theory (Hauenstein &
Sottile's alphaCertified as the point-based alternative).
