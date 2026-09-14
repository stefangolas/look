# Slow-solver diagnostics, R4 — the certified-boolean bottleneck

**For a frontier-model algorithm attack. Owner directive (2026-09-14): 120 s
is the acceptable performance ceiling for one corpus row; anything above it
is a CEILING verdict and books a faster solver. Never weaken the certificate
to buy speed — the oracle policy (kernel certificates ARE the certification)
is the invariant the faster algorithm must preserve.**

## 1. Measured facts (evidence, not inference)

R4 census: 54 rows, fresh python per row, `door.py --engine truck`, kernel
`truck123d.pyd` built from HEAD `0a509ea`. Verdicts: 20 GREEN / 2 CEILING /
27 TYPED-REFUSAL (26 `unsupported_envelope` + 1 other) / 5 DNF.

The three rows exceeding the ceiling all stall inside **one native boolean
call**, per faulthandler stack dumps taken at the ceiling
(`loop/nightly_ops/row_runs/*.stackdump.txt`):

| row | stall chain (python frames, file:line) | op |
|---|---|---|
| `f1/power_unit` | `build_power_unit:892 → build_block:437 → _deck_rail:396 → _fuse:182 → __add__ → _boolean:2324 → _kernel_facts:238 → bd_facts` | fuse (union) |
| `f1/suspension_rear` | `build_suspension_rear:983 → _rear_left:883 → _fuse:106 → __add__ → _boolean → _kernel_facts → bd_facts` | fuse (union) |
| `hypercar/lighting` | `build:846 → _front:666 → __sub__ → _boolean → _kernel_facts → bd_facts` | cut (subtract) |

Leaf-callee distribution of native samples (py-spy, folded,
`*.profile.folded`):

| row | kernel rust frames (`truck123d.pyd`) | `RtlAllocateHeap`+`RtlFreeHeap` | `finite` (ucrt) |
|---|---|---|---|
| power_unit | 68.5% | 19.7% | 7.0% |
| suspension_rear | 76.0% | 13.2% | 7.5% |
| lighting | 65.8% | 18.3% | 7.0% |

Reading: the compute core is the kernel's own numeric work (`finite` = mass
interval-endpoint / predicate evaluation), and **~1/5 of wall time is pure
allocator traffic** — the solver allocates and frees per-cell tables at high
rate during refinement. Python-side overhead is negligible (<1%).

## 2. The algorithm that is hit (exact, with code refs)

`door.py::_boolean` records one `BooleanOp` over two kernel-engine solids and
dispatches it **at record time** through the certified funnel: `_boolean_node`
builds the construction-tree JSON; `truck123d.bd_facts` (pyo3, GIL released)
routes to `membership::certify_boolean_volume{,_rational}` via
`truck123d/src/bd_bridge.rs:11389/11402 → certify_boolean_volume_impl (:11410)`.
Multi-operand `fuse`/`cut` from the corpus scripts use the MONO-9
compound-indicator fold (bd_bridge.rs:11547): per-operand signed flux against
the compound membership predicate — **no intermediate union solid is ever
constructed**.

`certify_boolean_volume_impl` phases, verbatim:

1. **Patch parsing.** Each operand boundary is a list of `VolumeRow`s →
   `parse_patch` (polynomial) or `parse_rational_patch` (FHC-G1
   rational-weight path). Patches are tensor-product Bernstein patches with
   outward-rounded interval control of their own certification.
2. **Operand volume certificates.** For every patch `P`,
   `cell_flux_certified(P, unit cell)` integrates the divergence form
   `g_P = (1/3) P · (P_u × P_v)` over the unit param cell with a certified
   interval bracket (exact 2-form integration over tensor-Bernstein patches,
   ADM-003; rational faces via the FHC-G1 reciprocal-power kernel, Theorem 5.2
   exponent-3 identity). Sums give `V(A), V(B)` as intervals.
3. **Control-hull broad phase.** All patch pairs `(p ∈ A, q ∈ B)`:
   `ranges_separate(p, q)` excludes pairs whose control-hull coordinate boxes
   are disjoint; survivors update `sigma = min sigma_min(p, q)`.
4. **Transversality admission.** `sigma < TRANSVERSALITY_MIN` refuses
   `TransversalityUncertified` (MONO-6 item 5; the FSSI-001
   separable tangency-free gate, normal-cone substrate, suspicion refusals).
5. **bbox sufficiency lemma** (Amendment 1, `extremes_survive`) — cheap
   pre-refinement gate on contaminated slabs.
6. **Intersection volume by error-directed refinement.** For each patch `p`
   of `A` against all of `B` (and symmetrically): `integrate_patch(p, B, …)`
   adaptively subdivides the param cell; each sub-cell is classified by the
   MONO-5 certified membership primitive (`classify_point`, retry contract)
   as **clear** (constant-membership witness, constant integrand) or
   **contact** (certified eq.-5 bracket, subdivide again). Per-patch width
   budget `budget = target_width / (2·patch_count)` with
   `target_width = relative_tolerance · V(A)`.
7. **Bracket assembly + gate.** `V(A∩B)` interval sums → inclusion–exclusion
   `V(A∪B) = V(A)+V(B)−V(A∩B)` (fold variant: signed per-operand flux against
   the compound indicator). Width gate: `width > target_width` refuses
   `BudgetExceeded`; success returns the certificate with phase stats
   (`broad_phase_pairs`, `excluded_pairs`, `clear_cells`, `contact_cells`,
   `max_depth`, `cover_cells`).

## 3. Where the time goes — rigorous bottleneck statement

The stall is inside phase 6 (the refinement loop) for one operand pair / one
fold step. Three quantified contributors:

**(a) Ambiguous-contact refinement depth.** The corpus compositions fuse
`n`-many lofted solids that *abut* (deck rails, suspension arms share planar
interfaces). For abutting/coincident support, membership classification of
cells straddling the shared interface stays in the contact class across many
subdivision levels: the certified bracket per cell shrinks linearly with
cell diameter while the ambiguous set itself only shrinks like the interface
measure. Cost per cell is `O(deg³)` Bernstein evaluation plus a certified
membership query (itself interval-bounded). Result: `max_depth` grows and
per-level cell counts grow multiplicatively — the 65–76% kernel-resident
time.

**(b) Allocation-dominated cell bookkeeping (~20% of wall).** Every
subdivision allocates fresh cell/patch tables (control-point boxes, interval
fields) through the global allocator. The profile shows allocator traffic
comparable to the arithmetic itself — the refinement's working set is
re-built per level instead of being amortized in an arena/stack discipline.

**(c) All-pairs broad phase.** Phase 3 is `Θ(|A|·|B|)` control-box tests per
boolean dispatch, and the fold repeats the funnel per operand, so a
k-solid composition pays `k` funnel invocations over growing operand sets.
For the F1 rows the operand sets are tens of solids with hundreds of
`VolumeRow` patches each.

## 4. The ask (constraint-preserving speedups)

Preserve: certified interval brackets (soundness), transversality admission,
the oracle policy (certificates are the certification), deterministic
per-platform reproducibility. Attack:

1. **Contact-set resolution instead of cell subdivision** for abutting
   interfaces: trace the contact locus (the MONO-6 contact cover already
   names it: closest-pair control-box exclusion) and integrate over the
   resolved trace rather than subdividing cells that straddle it.
2. **Arena/SoA cell storage** to kill the 20% allocator tax; reuse child
   cell buffers across levels.
3. **Hierarchical broad phase**: BVH over patch control hulls replacing the
   `Θ(|A|·|B|)` scan; per-fold operand reuse across the k funnel invocations.
4. **Parallel per-patch integration** (phase 6 is embarrassingly parallel
   over patches; the machine is otherwise idle — brackets compose
   additively).
5. **SIMD interval evaluation** — the workspace already compiles
   `+avx,+fma` globally (`.cargo/config.toml`); the interval endpoint
   arithmetic (`finite` leaves) is scalar today.
6. **Early termination inside clear regions**: constant-membership witnesses
   already exist (MONO-5); widen their jurisdiction so entire subtrees close
   without further bracket evaluation.

Success criterion: the three CEILING rows complete with certificates under
120 s; no verdict class changes for the 20 GREEN rows (V5 net: green stays
green; brackets stay sound).

## 5. Related open fronts (same census)

- 26 rows refuse `unsupported_envelope` at admission — the *coverage*
  frontier (which composition classes the funnel admits), distinct from this
  *speed* frontier.
- Compound volume aggregation returns 0.0 through the door's python-side
  `geometry_facts` (`falcon_heavy/vehicle`: 2142 solids, 4.46M triangles,
  bbox exact, `volume 0.0`) — facts-path gap, rows unjudgeable, not
  construction failures. `bd_facts` (native, phase-timed) exists and is not
  yet used by the door.

## Evidence index

- `loop/nightly_ops/r4/` — 54 door records (verbatim)
- `loop/nightly_ops/r4_verdicts.json` — adjudication
- `loop/nightly_ops/row_runs/*.stackdump.txt` — ceiling stack dumps
- `loop/nightly_ops/row_runs/*.profile.folded` — py-spy native profiles
- `loop/nightly_ops/r4_census.py`, `r4_adjudicate.py`, `row_run.py` — the
  tooling (re-run anything from the saved records without re-paying runs)
