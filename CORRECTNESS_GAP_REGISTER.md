# Correctness gap register

**Audited artifacts.** `look` @ `e7fb495`; truck-fork @ `39346191`;
`truck-*` pinned @ `79eaaf36`; `spade` 2.15.1.

One entry per **merged root cause**. Suspicious lines sharing a cause are one
entry. Line citations are `[fork]` = `truck-fork/truck-meshalgo/src/tessellation/`,
`[pin]` = the pinned truck crates, `[look]` = this repo.

---

## Implementation status

**Phase 0 landed.** Measured on ABC `00009190` (24,202 declared faces). This is
one acceptance snapshot, **not** a frozen baseline or a correctness oracle — the
reason histogram is evidence about where loss sits, and face count remains
explicitly rejected as a success metric.

| Gap | State | Evidence |
|---|---|---|
| **G8** | landed | Failures reach the caller typed. Totals reconcile exactly with the previous hand-reconstructed split, so behaviour is unchanged; `ContradictoryDualParity x143` — a *proved* inconsistency — became visible for the first time |
| **G11** | landed | Commuting test passes, and **fails without the fix**. Full `truck-geometry` suite 24/24 |
| **G5a** | landed | `unresolved_at_flood` **213 → 0**. Full `truck-meshalgo` suite 30/30 |

| Reason | Count |
|---|---|
| `ConstraintInsertionIncomplete` | 4,042 |
| `BoundaryConstructionFailed` | 262 |
| `ContradictoryDualParity` | 138 |
| `ConstraintOverlapUnsupported` | 9 |
| `NoOddParityRegion` | 1 |

Net against the Phase 0 entry state: 4,457 → 4,455 faces lost, 1,359,029 →
1,359,887 triangles, and zero constraint edges the role table cannot name.

Four findings worth carrying forward:

1. **G11 cost 19 faces on its own** (`ContradictoryDualParity` 143→159) before
   the rest of Phase 0 absorbed it. The fix is not in question — the axis
   convention was wrong and every other `Processor` method transposes. What it
   demonstrates is that a *better* starting parameter shifts which faces fall
   into populations already failing for unjustified reasons (G2's silent branch
   acceptance, the 1e-3 UV closure). Downstream sensitivity to legitimate
   upstream variation is a symptom of absent preconditions, not an argument
   against the correction.
2. **The sampling-grid role loss was reintroducing the A1 defect.** `insert_surface`
   used the same lossy lookup, so a grid edge realized as a chain lost its role
   and fell to the toggling default. Labelling the chain recovered 21 faces and
   1,732 triangles, and `ContradictoryDualParity` fell 164 → 143.
3. **The old code refused "already fully represented" as a failure** —
   `is_already_constraint` fell through to `success = false`. Exactly 5 faces on
   `00009190`, matching `REFINEMENT_AUDIT.md` §4's independently-measured figure.
4. **ARR-003 was being enforced by accident.** Removing (3) regressed
   `test_parity_intersecting_constraints_rejected`, which asserts a
   self-overlapping loop must be refused. The intent was right; the mechanism
   conflated overlap with already-represented. `insert_to` now returns a typed
   reason and detects duplicate traversal explicitly, which populates
   `ConstraintOverlapUnsupported` — a variant previously declared and never
   constructed — on 9 faces.

### Phase 1 landed — honest refusal

| Gap | State | Evidence |
|---|---|---|
| **G5b** | landed | `toggles_material` returns `Option<bool>`; an unresolved role is `ConstraintRoleMissing`, not a guessed material answer in either direction. Lands **provably non-firing** (`unresolved_at_flood` is 0 after G5a) |
| **G2** | landed | Refinement exhaustion returns `AmbiguousLift` instead of accepting the ambiguous value. `try_new` now returns typed reasons throughout |
| **G7a** | landed | `include` returns `Option<bool>`; undecidable containment is counted, not folded into "outside" |

| Reason | Phase 0 | Phase 1 |
|---|---|---|
| `ConstraintInsertionIncomplete` | 4,042 | 4,015 |
| `BoundaryProjectionFailed` (was `BoundaryConstructionFailed`) | 262 | 262 |
| `ContradictoryDualParity` | 138 | 126 |
| `AmbiguousLift` | — | **71** |
| `ConstraintOverlapUnsupported` | 9 | 8 |
| `NoOddParityRegion` | 1 | 1 |
| **faces lost** | 4,455 | 4,486 |
| **triangles** | 1,359,887 | 1,358,543 |

**The honesty tax, decomposed.** 71 faces had an unresolved periodic branch. Of
those, ~40 were already failing downstream under a misleading reason
(`ConstraintInsertionIncomplete` −27, `ContradictoryDualParity` −12,
overlap −1) — a folded lift producing a self-crossing, exactly the mechanism
`REFINEMENT_AUDIT.md` §3 named. The other **31 were rendering geometry built on
an unresolved branch**, and now refuse. That is the population this change
exists to find.

**The placeholder bucket resolved to a single cause.** All 262 faces that could
not build a boundary are `BoundaryProjectionFailed` — none are empty wires, none
are off-surface points. `BoundaryWireEmpty` and `BoundaryPointOffSurface` are
zero on this model (the latter expectedly, since the compatibility gate is off).

**Two corrections to G7a, both from measurement.**

The first implementation justified retrying rays on the grounds that an aborted
crossing count is a ray artifact. Measured at 1 vs 8 attempts: 77,069 vs 63,108
unresolved, every other number byte-identical. So **18% of one-ray aborts are
resolved by alternate directions — all to "outside" — and 82% remain unresolved
after eight directions.** The retry narrows the unknown population and changes
no output.

The second correction is the substantive one. That residue was then described as
"points genuinely on the boundary", which the experiment does not establish: a
float abort can equally be near-boundary degeneracy or an unlucky seed family,
and *no ray decided* licenses neither conclusion. `include` was replaced by
`locate -> PointLocation { Inside, Outside, Boundary, Indeterminate }`, where
`Boundary` comes from a **direct point-on-segment predicate** and `Indeterminate`
means no method established a location.

Measured with that split: **117,145 `Boundary`, 0 `Indeterminate`**, output
byte-identical. The original guess was right; the reasoning was not, and the
`Indeterminate` arm now exists to report it if that ever ceases to hold. The
direct predicate also classifies ~54,000 samples the ray method had called
"outside" — within tolerance of a boundary — none of which had been inserted, so
nothing moved.

Grid samples on a boundary are not an edge case here: an axis-aligned trim edge
coincides with the sampling grid by construction. The population was previously
invisible, silently reported as "outside".

Remedy categories are drawn from the fixed list: *local code correction*,
*preserve existing evidence*, *production type boundary*, *constructor with
checked postcondition*, *restricted-path guard*, *typed Unknown/Unsupported
result*, *foreign-library wrapper*, *formal document clarification*.

---

## G1 — Parameter-domain authority is absent

| Field | Value |
|---|---|
| **Formal obligation** | DOM-001; FS Def. 7 (Ω has an effective finite representation) |
| **Production assumption** | `surface.try_range_tuple()` returns the *face's* material parameter extent |
| **Existing evidence** | An accessor result. Four distinct epistemic classes arrive indistinguishable: exact `[0,2π)` ([pin] `revolved_curve.rs:135`); exact knot span ([pin] `nurbs/bspsurface.rs:304-313`); **fabricated** `[0,1]²` for `Plane` ([pin] `specifieds/plane.rs:136-139`); **10× apex-extrapolated** window for a revolved generatrix ([pin] `revolved_curve.rs:116-134`) |
| **Gap** | A supporting surface's declared range is a property of the primitive, never of a face that references it. No constructor establishes any relation between the two. The `Plane` and generatrix cases are not merely uncertified — they are known-false. |
| **Earliest affected function** | [fork] `triangulation.rs:484` (`PolyBoundaryPiece::try_new`), then `:1064` (`PolyBoundary::new`) |
| **Downstream consequences** | Synthetic closure corners at `:1227-1269` and the whole-rectangle fallback at `:1315-1329` fabricate trim geometry no source entity describes (`DOM-ARTIFICIAL-CLOSURE-001`); when the piece already lies on the rectangle edge the enclosed area is zero (`DOM-ZERO-AREA-001`); the deck-normalisation origin `u0` at `:662-668` inherits the same fiction |
| **Minimum remedy category** | production type boundary (a domain descriptor, derived after lifting per audit §6.3) |
| **Dependencies** | none — this is a root |
| **Confidence** | **High.** The four classes and their line numbers are read directly from source; `working_range` ([fork] `:1025-1048`) already exists as the derived alternative and is documented as `PAR-RANGE-INHERITANCE-001`. |

---

## G2 — Lift branch continuity is never established

| Field | Value |
|---|---|
| **Formal obligation** | FS Def. 14 (lift is continuous, `q∘γ = γ̄`); FS Def. 7 (embedding on the regular set) |
| **Production assumption** | The period copy nearest the previous sample is the correct branch |
| **Existing evidence** | `get_mindiff` ([fork] `:866-872`) — a rounding rule, valid only while the true step is under half a period. Ambiguity **is** detected (`:609-616`, threshold `AMBIGUOUS_STEP_FRACTION = 0.45`) and refined by chord bisection up to `MAX_LIFT_REFINEMENTS = 8` |
| **Gap** | On refinement exhaustion the ambiguous step is **accepted with no record** (control falls through to `:623`). Nothing checks that the resulting arc is simple. The face proceeds as though the lift were certified. |
| **Earliest affected function** | [fork] `triangulation.rs:579-584` and `:608-626` |
| **Downstream consequences** | A folded loop reads as closed; the fold produces a self-crossing that `insert_to` later refuses as `ConstraintInsertionIncomplete`. Prior measurement on ABC `00009190`: 4,048 faces (84% of loss), 92% of them on a periodic axis, 91% with exactly one refusal |
| **Minimum remedy category** | typed Unknown/Unsupported result (a lift that exhausts refinement must return `Unresolved(AmbiguousLift)`, not a value) |
| **Dependencies** | G11 (a transposed hint makes the branch choice worse on inverted `Processor`s); G1 (the normalisation origin) |
| **Confidence** | **High** that the silent acceptance exists and is unsound. **Unknown** whether a certified lift removes the measured self-crossings or some survive as genuine transverse intersections — `REFINEMENT_AUDIT.md` §3 is explicit that this is not settled, and settling it requires a constructive witness, not another population study. |

---

## G3 — Deck displacement and winding are computed, then discarded

| Field | Value |
|---|---|
| **Formal obligation** | QUO-002 ("a Boolean `closed` is insufficient"); QUO-004; FS Def. 9 (δ is part of an arc); FS §VII (potential ψ) |
| **Production assumption** | Once points have been shifted into a common copy, the integer displacement carries no further information |
| **Existing evidence** | `periodic_displacement` ([fork] `:939-950`) computes `[ku,kv]` correctly and `BoundaryClosure::PeriodicClosed{displacement}` ([fork] `:926-936`) is the right shape to hold it |
| **Gap** | The value is consumed to translate points at `:1110-1130` and then dropped; it is never stored on the piece, never aggregated across bounds, never checked for cycle consistency. Separately, each piece is normalised against its **own centroid** at `:659-669`, so two bounds of one face can land in unrelated deck copies with nothing recording that they did. |
| **Earliest affected function** | [fork] `triangulation.rs:659-669`, then `:1110-1130` |
| **Downstream consequences** | Winding is unavailable to closure, to material classification, and to any consistency check; `DeckPotentialUnionFind` ([fork] `domain/deck.rs`) — a correct QUO-004 solver — cannot be called because its inputs do not survive |
| **Minimum remedy category** | preserve existing evidence |
| **Dependencies** | G1 (the absolute anchor `u0` should become a relative rule per audit §6.3) |
| **Confidence** | **High.** Both the computation and the discard are single, unambiguous sites. Note the prior negative result: the measured crossings are **intra-bound**, so this gap alone does not explain them. |

---

## G4 — No normalized arrangement stage exists

| Field | Value |
|---|---|
| **Formal obligation** | ARR-002 (every proper intersection becomes a vertex); ARR-003 (no unresolved overlap); FS Def. 18 (arcs split at a certified parameter set) |
| **Production assumption** | The lifted boundary is already an arrangement — pairwise non-crossing, non-overlapping, atomic |
| **Existing evidence** | **None.** Consecutive point pairs are handed directly to Spade at [fork] `:1437-1441` |
| **Gap** | There is no intersection solver, no splitting stage, and no overlap classification anywhere on the path. `ConstraintIntersectionUnsupported` and `ConstraintOverlapUnsupported` ([fork] `:1709-1710`) are declared and **never constructed**. Vertex identity is instead reconstructed by proximity welding at `:1409-1412` (`distance_2 < 1e-12`, linear scan), which merges `~_Λ` and `~_Σ` into one undifferentiated weld — explicitly forbidden by FS §IX |
| **Earliest affected function** | [fork] `triangulation.rs:1404-1441` (`insert_to`) |
| **Downstream consequences** | Spade is asked to realize constraints against a set that may not contain their intersections, which is exactly the precondition its atomic realization needs (G5); a T-junction manufactured by welding is indistinguishable from a source vertex |
| **Minimum remedy category** | production type boundary (`NormalizedArrangement` owning atomic subdivision) |
| **Dependencies** | G2, G3 (certifying segments before the lift is trustworthy certifies the wrong segments) |
| **Confidence** | **High** on absence. **Unknown** how much of the measured loss survives a correct lift and therefore genuinely needs this stage. |

---

## G5 — The CDT realization bijection is not retained

| Field | Value |
|---|---|
| **Formal obligation** | CDT-001 (stable `ConstraintId`); CDT-002 (complete constrained chain; "a successful insertion API call is not sufficient evidence") |
| **Production assumption** | `add_constraint(vi,vj)` realizes the request as the single edge `(vi,vj)`, recoverable by `get_edge_from_neighbors` |
| **Existing evidence** | A `bool` ([fork] `:1452`) plus an after-the-fact handle lookup (`:1453`). Spade documents the opposite: *"the given constraint might be split into smaller edges… `exists_constraint(from,to)` is not necessarily `true`"* ([spade] `cdt.rs:541-544`) |
| **Gap** | Every chain Spade splits loses its role entry (213 measured). The `None` arm of `toggles_material` ([fork] `:1694-1698`) then **toggles material anyway**, so an unnameable edge still flips parity |
| **Earliest affected function** | [fork] `triangulation.rs:1446-1475` |
| **Downstream consequences** | Material classification runs on a role map with known holes; CDT-002 cannot be discharged at all |
| **Minimum remedy category** | foreign-library wrapper |
| **Dependencies** | G4 for *completeness* (atomic input); **independent for the role-loss defect itself** |
| **Confidence** | **High**, and the remedy is better-characterised than previously recorded. `try_add_constraint` ([spade] `cdt.rs:807-817`) already returns `Vec<FixedDirectedEdgeHandle>` — the complete realized chain, including pre-existing edges — is atomic on conflict, and distinguishes refusal (empty) from already-represented (non-empty). **No vendoring is required**, contrary to the qualified reading in `REFINEMENT_AUDIT.md` §4. |

---

## G6 — Source and synthetic boundary segments are indistinguishable

| Field | Value |
|---|---|
| **Formal obligation** | FS §IX `kind(h) ∈ {Physical, ArtificialCut, NativeBoundary, SingularLink}`; FS Def. 20 (a physical half-edge pins `μ_L=1, μ_R=0`; an artificial cut requires `μ_L=μ_R`) |
| **Production assumption** | Everything in a `PolyBoundary` piece is physical boundary |
| **Existing evidence** | `ConstraintRole::UnresolvedSyntheticClosure` ([fork] `:1640-1643`) is the correct type and is **never constructed**. The gap is acknowledged in-source at `:1455-1464` |
| **Gap** | `PolyBoundary::new` appends synthetic closure segments (`:1222-1295`), seam segments (`:1208-1211`) and re-synthesised full-period circles (`:628-658`) into the **same `Vec<SurfacePoint>`** as source-derived points. After that, no discriminant exists, so `:1465` tags all of them `PhysicalBoundary` |
| **Earliest affected function** | [fork] `triangulation.rs:1222-1295` (creation), `:1465` (misclassification) |
| **Downstream consequences** | Fabricated geometry toggles material parity exactly as a real trim segment does — the same class of error A1 fixed for the sampling grid, in a population A1 did not cover |
| **Minimum remedy category** | preserve existing evidence (per-segment provenance through stitching) |
| **Dependencies** | G1 (most synthetic segments exist *only because* the fabricated range demanded them; fixing G1 shrinks this population rather than reclassifying it) |
| **Confidence** | **High** — the enum variant, the acknowledgement comment, and the creation sites are all explicit. |

---

## G7 — Material solve assumes its base state and cannot express ambiguity

| Field | Value |
|---|---|
| **Formal obligation** | DOM-003 (explicit `BaseDomain`); DOM-005 (`χ_M = χ_base ⊕ parity`); FS Def. 21 (Unique / Ambiguous / Inconsistent trichotomy) |
| **Production assumption** | The CDT outer face is non-material, and parity from there is the material predicate |
| **Existing evidence** | `face_parity.insert(outer.index(), 0)` ([fork] `:1980-1982`). Nothing establishes it |
| **Gap** | Parity is not the FS Def. 20 constraint system: it cannot represent `μ_L = μ_R` for artificial cuts (only the *ability to toggle* is modelled, via `toggles_material`), it has no deck-identification constraint, and `Ambiguous` is inexpressible — `|M| > 1` has no representation, so an underdetermined face silently becomes one arbitrary labelling. `include()` ([fork] `:1334-1357`) independently returns `false` on its degenerate arm (`:1348`), collapsing "cannot decide" into "outside" |
| **Earliest affected function** | [fork] `triangulation.rs:1980-1982`; `:1334-1357` |
| **Downstream consequences** | A face with two valid labellings is meshed as though one were proved |
| **Minimum remedy category** | typed Unknown/Unsupported result |
| **Dependencies** | G5, G6 (the constraint system is only as good as the roles feeding it) |
| **Confidence** | **High** on the base assumption and on `Ambiguous` being inexpressible. **Unknown** how often real faces are genuinely ambiguous. |

---

## G8 — Every typed failure is erased into an empty mesh

| Field | Value |
|---|---|
| **Formal obligation** | FS Def. 3 and Theorem 1 ("no input reaches an untyped state"); MF §27, §31 (policy separate from detection) |
| **Production assumption** | A caller that receives an empty mesh can act on it |
| **Existing evidence** | Detection is **already correct and complete-ish**: 11 typed reasons ([fork] `:1703-1716`), `TessellationFailure` carries `source_bound`, `source_edge_use`, `constraint_ids`, `uv_location` (`:1718-1725`), and `TessellationOutcome` (`:1819-1823`) is the right sum type |
| **Gap** | `trimming_tessellation` ([fork] `:1885-1907`) matches the outcome and returns `PolygonMesh::default()` for **every** failure, including `ContradictoryDualParity` — a *proved inconsistency*. The typed value is constructed and immediately destroyed. This is MF §31's detection/policy conflation, inverted: the detector is right and the policy discards it |
| **Earliest affected function** | [fork] `triangulation.rs:1900-1905` |
| **Downstream consequences** | [look] `src/step.rs:208-…` cannot distinguish "no surface produced" from "surface meshed to nothing" and had to reconstruct the split by hand; a face that is *proved inconsistent* is reported identically to one that is merely empty; no obligation anywhere downstream can be conditioned on the reason |
| **Minimum remedy category** | preserve existing evidence |
| **Dependencies** | **none** — fully independent, and it is what makes every other gap observable |
| **Confidence** | **High.** Single site, single line range, no ambiguity. |

---

## G9 — Curve–surface compatibility is detected but disabled

| Field | Value |
|---|---|
| **Formal obligation** | GEO-005 (`sup_t ‖C_e(t) − S_f(q(t))‖ ≤ ε`); GEO-006 ("a distant nearest point is not a valid inverse"); FS Def. 13 |
| **Production assumption** | Any parameter `search_nearest_parameter` returns is an incidence |
| **Existing evidence** | The gate exists and is correct in shape ([fork] `:559-577`), but `COMPATIBILITY_FACTOR = f64::INFINITY` ([fork] `:890`) disables it by default |
| **Gap** | The default is deliberate and documented (`:874-889`): the rejected points are a real population (median 191× tolerance on `00009190`), but rejecting them "repairs nothing visible" while costing 292 faces. So this is a **known, reasoned deferral**, not an oversight — the refusal has nowhere to go until G8 gives it a typed destination |
| **Earliest affected function** | [fork] `triangulation.rs:559-577` |
| **Downstream consequences** | A boundary belonging to another face yields a plausible UV path that triangulates into a large wrong region — the mechanism named at `:543-558` |
| **Minimum remedy category** | restricted-path guard |
| **Dependencies** | **G8** — the policy is only actionable once a refusal can be reported as a typed reason rather than an empty mesh |
| **Confidence** | **High** on the mechanism and on the reasoning behind the default. Turning it on is a *policy* change requiring its own measurement, which this map does not perform. |

---

## G10 — Edge-use orientation and wire closure are applied, not established

| Field | Value |
|---|---|
| **Formal obligation** | TOP-004 (cyclic `end(eᵢ) = start(eᵢ₊₁)`); TOP-005 (effective traversal is the composition of face, bound, oriented-edge and edge-curve orientation, and must agree with source incidence) |
| **Production assumption** | Reversing a polyline expresses the edge use; the wire closes |
| **Existing evidence** | Partial and **upstream-correct**: [pin] `convert.rs:244` composes bound × oriented-edge into `CompressedEdgeIndex.orientation`, and TOP-003 is genuinely discharged there by `collect::<Result<Vec<_>,_>>()` (`:253-255`, with the reasoning in the comment at `:216-227`) |
| **Gap** | Three distinct losses, one cause — orientation is treated as a geometric operation rather than a retained fact. (a) [fork] `:340-343` applies it as `curve.inverse()` and keeps nothing. (b) [fork] `:515` closes the walk by `bdry3d.push(bdry3d[0])`, so TOP-004 is **assumed by construction** and a non-closing wire is silently repaired. (c) [fork] `:345` re-opens TOP-003 with `filter_map(create_edge)` |
| **Earliest affected function** | [fork] `triangulation.rs:340-345`, `:515` |
| **Downstream consequences** | No half-edge carries `ℓ` (FS Def. 9), so the FS Orientation Axiom ("material lies locally on the left") has no representation and material side must be recovered by parity instead — the root reason G7 uses parity at all |
| **Minimum remedy category** | preserve existing evidence |
| **Dependencies** | none for (a)/(b); (c) is a local correction |
| **Confidence** | **High** for (a) and (b). For (c): the hazard is currently **unreachable** — indices come from `checked_edge_position` ([pin] `convert.rs:245-247`) and [fork] `:258` builds `edges` 1:1 from `shell.edges`, so `edges.get()` cannot fail today. It is a latent trap, not an active loss, and is recorded as such rather than as a defect. |

---

## G11 — `Processor::search_parameter` transposes its result but not its hint

| Field | Value |
|---|---|
| **Formal obligation** | Parameter-axis identity (starting fact 7: every axis-indexed fact must be stated in the caller-visible convention); GEO-006 |
| **Production assumption** | The continuity hint passed to `sp` is interpreted on the same axes as the returned parameters |
| **Existing evidence** | [pin] `truck-geometry/src/decorators/processor.rs:507-521`. Every other `Processor` method swaps consistently — `subs` (`:217-222`), `uder`/`vder` (`:224-235`), `uuder`/`uvder`/`vvder` (`:237-256`), `parameter_range` (`:259-265`), `u_period`/`v_period` (`:267-282`), `normal` (`:288-296`) |
| **Gap** | `search_parameter` forwards `hint` **verbatim** to `self.entity.search_parameter(...)` while transposing the result. On an inverted `Processor`, the caller's `(u,v)` hint is interpreted by the entity as `(v,u)` |
| **Earliest affected function** | [pin] `processor.rs:507-521`, reached from [fork] `triangulation.rs:532` where the previous lifted UV is passed as the hint |
| **Downstream consequences** | Degrades exactly the continuity mechanism G2 depends on, on precisely the surfaces (inverted cylinders and cones) where branch choice matters most |
| **Minimum remedy category** | local code correction |
| **Dependencies** | none — independent, and upstream of G2 |
| **Confidence** | **High** that the asymmetry exists; it is visible in nine lines. **Unknown** what fraction of STEP cylinders and cones arrive with `orientation == false`, and therefore what the blast radius is — measuring that is out of this map's scope. Note this sits in a **pinned** crate, so the remedy is a fork-and-bump, not a local edit. |

---

## Dependency graph

```
G8  (typed failure erased) ──────────► G9  (compatibility guard)
 │        [independent root]
 │
 └─ makes every gap below observable

G11 (hint axis) ──┐
                  ▼
G1  (domain authority) ──► G2 (lift continuity) ──► G4 (arrangement) ──► G5 (realization)
 │                          ▲                                              │
 │                          │                                              ▼
 └──► G6 (synthetic role)   └── G3 (deck/winding)                    G7 (material solve)
              │                                                            ▲
              └────────────────────────────────────────────────────────────┘

G10 (orientation retained) ──► G7   [also independent of the G1 chain]
```

**Roots** (no prerequisite): **G8**, **G11**, **G1**, **G10**.

**Highest fan-out**: **G1** — it feeds the lift's normalisation origin (G2), the
synthetic-segment population (G6), and every fabricated closure corner.

**Cheapest independent wins**: **G8** (one match arm), **G11** (one argument),
**G5**'s role-loss half (one public API swap).
