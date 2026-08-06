# Handoff — residual loss after WAVE-2C

> **Superseded in part by WAVE-3A** (`docs/WAVE_3A_DECK_JOIN_AND_SEED.md`,
> truck `562299a5`). Packages 1 and 2 below are built and measured:
> **+3,790 faces**, 819,769 → 823,559, loss 2.31% → 1.86%, zero
> `rendered -> lost`. Three corrections to what is written below:
>
> - Package 1's population is **3,087**, not 4,850. The `+1,779 source×seam`
>   faces do not come from the two-loop join at all — 1,507 are
>   `SeamWithoutTwoLoopJoin` and 269 carry no seam evidence. The DIAG-001
>   extension §1 demands was built first and said so.
> - Package 2 yields **+705**, not the 4,522 the family/reason cell suggests:
>   a face needs *every* one of its failing boundary points fixed, so partial
>   success recovers nothing.
> - Package 3's prerequisite `GEN-001` A7 is **closed** (GEN-001E). It is not
>   blocked — but it is a build, not a splice; see §3 of the WAVE-3A note.
>
> Package 2's instruction to re-measure the cylinder/torus `BoundaryProjectionFailed`
> rows *after* package 1 is now due and has not been done.

**State.** look `8eda7d3` on `integration/formal-atlas-wave-2`, pinned to truck
`6f8153ea` on `feature/torus-rank2-cell`. Both pushed. No path override.

The five formal recovery routes are now **default-on** (WAVE-2C). Corpus-wide
that took rendered from 797,239 to 819,769 (**+22,530**, loss 5.00% → 2.31%)
with **zero** `rendered -> lost` regressions across 839,179 faces. See
`docs/WAVE_2C_GATE_GRADUATION.md` for the graduation itself, its gate
semantics, and the verification.

This document is about the **19,410 faces still lost**, and what to do next.

---

## Read this before you plan anything

**Measure the corpus, not `00009190`.** The previous edition of this file
planned Priority 1 from a single model. That model is not representative, and
planning from it produced a target that was 88% already-solved. I repeated the
mistake in miniature this session: the two benchmark geometries
(`00009190` + `00008001`) say the largest `ConstraintInsertionIncomplete` class
is rank-0 planar arc crossings; the **corpus says it is seam×seam on periodic
charts, by a wide margin**. Both statements are true of their sample. Only one
is true of the product.

**Measure with the routes on.** Every number below is default-on. A census
taken with `TRUCK_FORMAL_RECOVERY=0` describes a configuration nobody ships and
will hand you a target that is mostly already recovered.

**Use production's entry point.** `face_census` must call
`robust_triangulation_with_torus_outcome`. The cone form supplies no torus
adapter and silently reports every toroidal face as lost. This bug hid 517
faces on `00009190` alone. If a route's recovery count is suspiciously zero,
check this first.

**Join on `source_face_id`.** `declared_face_index` resets per shell and
collides. `p1-out/reconcile.py` does this correctly; reuse it.

---

## The residual, corpus-wide

19,402 of the 19,410 lost faces carry a DIAG-001 record (the remaining 8 are in
one model whose diagnostic run is trivially re-creatable). Every one has
exactly one terminal outcome; `InsertionUnknown` is 0.

| terminal reason | faces | share |
|---|---:|---:|
| `ConstraintInsertionIncomplete` | 8,917 | 46.0% |
| `BoundaryProjectionFailed` | 6,687 | 34.5% |
| `AmbiguousLift` | 1,492 | 7.7% |
| `ContradictoryDualParity` | 1,422 | 7.3% |
| `ConstraintOverlapUnsupported` | 426 | 2.2% |
| `NoOddParityRegion` | 330 | 1.7% |
| `BoundaryConstructionFailed` | 127 | 0.7% |
| `ConstraintRoleMissing` | 1 | 0.0% |

Cross-tabbed against surface family — the view that actually separates work:

```
family        CII   Proj   Lift Parity   Ovlp  NoMat    Bnd   total
Cylinder     2745   1072    426   1180    101                  5524
Nurbs         352   3130             3      8      1           3494
Plane        2947                         104    200           3251
Torus        1163    939      3    122                         2227
Bspline       427   1392                  69     129           2017
Cone          813     85    673     81    136                  1788
Sphere        252     31    380     28                          691
Extruded      160     12      4      6      4                   186
Revolved       50      7      6      2      4                    69
Offset          8     27                                         35
Unknown                                                 127     127
```

`ConstraintInsertionIncomplete` splits: **5,045 on periodic charts, 3,872 at
rank 0.** The two halves are different problems and must not be merged.

---

## Work packages, in yield order

### 1. Periodic seam×seam — 3,071 faces (+1,779 source×seam)

The largest single class in the residual, and the original Priority 1 target.
It survived the band routes because those routes admit only faces that are
*complete two-circle bands*; everything else with a seam still fails.

```
1499  Cylinder  SyntheticSyntheticCrossing
 904  Torus     SyntheticSyntheticCrossing
 504  Cone      SyntheticSyntheticCrossing
 122  Sphere    SyntheticSyntheticCrossing
1023  Cylinder  SourceSyntheticCrossing
 198  Torus     SourceSyntheticCrossing
 196  Cone      SourceSyntheticCrossing
```

**Mechanism (read the code before trusting this).** In
`triangulation.rs::PolyBoundary::new`, the two-closed-loop branch cuts each
`periodic_source_walk` loop open, **reverses one**, and bridges the two with a
pair of `SegmentOrigin::Seam` segments. It aligns the loops by *mean u* and
never reconciles their **deck displacements**. The two boundary circles of a
band carry opposite winding (`ku = +1` and `ku = −1`) — as they must, for the
face boundary to be coherently oriented — so after the unconditional reverse
they run *parallel* rather than antiparallel, and the two bridges become
crossing diagonals instead of the two vertical cut edges of a rectangle. Spade
refuses the second bridge, and the face dies as
`ConstraintInsertionIncomplete` with both origins `Seam`.

**The fix is a decidable equation, not a heuristic.** For a quotient-closed
boundary walk, `Σδᵢ = Δ_walk`, and `Δ_walk = 0` for a contractible regular
boundary. Traversing loop1 reversed contributes `−(−1) = +1`, giving
`Σδ = +2 ≠ 0` — inconsistent, which is exactly what the crossing witnesses.
Traversing it forward gives `Σδ = 0`. So:

- Solve the deck potential across both loops from the **source vertex
  correspondence** carried by `SourceEdgeUseInput` (`source_vertices` /
  `use_vertices`), not from mean-u proximity and not from `get_mindiff`.
- Choose loop1's traversal direction and lattice translate to satisfy
  `Σδ = Δ_walk`. Unique solution → resolved; none → `Inconsistent`; several →
  `Unresolved`. Refuse with a typed exit in the latter two; do not guess.
- The two cut copies are lattice translates of each other by exactly one
  period. Hold that as a **certified cut-pair relation** — not as two
  independent CDT constraint edges, which is what makes them crossable in the
  first place.

`DeckPotentialUnionFind` (`domain/deck.rs`) and `CertifiedDeckLabel`
(`formal/quotient.rs`) already exist and give typed `Unique`/`Ambiguous`/
`Incompatible`/`Unresolved` outcomes. Use them; do not write a third solver.

**Before you build anything**, extend DIAG-001 to record, per face: each
boundary piece's `ku`/`kv`, its winding sign, the chosen fundamental-domain
representative, and both seam endpoints. That turns `SyntheticSyntheticCrossing`
into mechanism-level subtypes and will tell you what fraction of the 3,071 is
actually the opposite-winding case versus something else. **I did not do this
— the hypothesis above is read off the source, not measured.** Do not skip it.

Expected yield if the opposite-winding case dominates: low thousands. Verify
the premise first.

### 2. Spline projection — 4,522 faces

NURBS 3,130 + B-spline 1,392 `BoundaryProjectionFailed`. The single largest
*family/reason* cell in the whole residual, and the reason splines are 28% of
remaining loss.

`by_search_parameter` calls `surface.search_parameter(point, hint, 100)`, then
retries with `None`, then declares failure. It is a numerical inverse problem
failing to converge from one bad initial hint. Seed instead from **knot-span
midpoints** — the knot vector already partitions the domain, and each span
midpoint is a natural start. Only the initialisation changes; the iteration
stays as it is.

Do not accept a parameter merely because the search returned one. §VI
Definition 13 requires a monotone traversal correspondence with endpoint and
orientation agreement; check it before admitting the projection.

Also note cylinder 1,072 and torus 939 `BoundaryProjectionFailed`. Those may be
deck artifacts — if the boundary was lifted to the wrong period copy, the
search looks in the wrong chart. **Re-measure them after package 1**, not
before.

### 3. Rank-0 arrangement (ARR-003) — 3,872 faces

Dominated by plane: 2,250 inter-bound + 697 same-bound source/source crossings,
plus 590 on splines. Physical boundary arcs properly crossing each other in a
simply connected parameter domain.

This is `NormalizeIntersections(B)` (§IV.B, §IX Definition 18): compute the
intersection with the existing certified predicates (`formal/bezier_isect`,
`formal/exact`), populate `ParameterEnclosure2` (currently `None` in every
witness), split both arcs at their certified parameters, and re-insert the
sub-arcs. Sub-arcs sharing a vertex do not cross.

Cleanest formally — no periodicity, no deck, no lift. `GEN-001` (the chord-side
audit item) is named as the prerequisite; check its state before starting.

### 4. Cylinder/cone parity — 1,422 faces

`ContradictoryDualParity`, and it is overwhelmingly cylindrical (1,180) with
torus 122 and cone 81.

The old Priority 4 predicted this would shrink once seams stopped generating
physical-boundary constraints. **It has not** — it is essentially unchanged. So
the live question is the one that was deferred: `toggles_material` returns
`Some(true)` for `ConstraintRole::UnresolvedSyntheticClosure`, generating
`μ_L = 1, μ_R = 0`. An artificial cut should generate `μ_L = μ_R` (§X
Definition 20, second bullet). Flipping it is one line and changes material
state for *every* face with synthetic segments, so measure it strictly on its
own, and watch triangle counts per face, not just rendered/lost — the ledger
carries `triangles=` for exactly this.

Sequence it **after** package 1: if seams become cut-pair relations rather than
constraint edges, this population may change shape.

### 5. Sphere lift — 380 faces

Spheres own 380 of the 1,492 `AmbiguousLift`, and cone another 673. Together
that is 71% of the class in two families. `AMBIGUOUS_STEP_FRACTION = 0.45`
bisection exhausting is the proximate cause; a certified per-family lift rule
should clear most of it. Small, self-contained, good warm-up task.

---

## Three models to look at directly

Residual is not uniform. These three hold 7,664 faces — 39% of all remaining
loss:

| model | residual | note |
|---|---:|---|
| `00007705` | 3,727 | worst in corpus; recovered only 238 of 3,965 |
| `00000414` | 2,184 | **recovered nothing at all** |
| `00001075` | 1,753 | recovered 2,813, still the third worst |

`00000414` and `00005427` (723 residual) are the two models where the formal
routes fired zero times. Whatever they are made of is not what the band routes
were built for, and a look at either may be worth more than another corpus
sweep.

---

## Regression discipline

Any new route must:

- be **refinement-only** — entered only where `failure.is_some()`, so it can
  replace nothing but a failure. This is what makes `rendered -> lost = 0`
  structural rather than lucky;
- ship behind `TRUCK_FORMAL_RECOVERY_<ROUTE>`, default-on, disabled by an
  explicit `0`/`off`/`false`/`no`, nested under the master gate;
- keep the declared population fixed and be reconciled per `source_face_id`
  against the previous pinned revision on all 20 models;
- validate before replacing: constraint completeness, boundary preservation,
  seam-pair consistency, connectedness, Euler characteristic, boundary-component
  count, world-space tessellation tolerance.

Two traps that cost me time this session, both now in
`docs/WAVE_2C_GATE_GRADUATION.md`:

- **Never batch benchmark configs.** All-of-A-then-all-of-B reported a 2.6×
  slowdown that does not exist. Alternate and take the minimum of ≥5 reps.
- **A fresh exe timestamp is not a fresh exe.** `cargo test` rebuilds examples
  using whatever `Cargo.toml` said at the time. Wait for the build's own
  completion, then verify by behaviour.

## Artifacts

Outside git, `C:\Users\stefa\look-corpus\p1-out\`:

- `corpus-pinned/{off,on}_<id>.{census,ledger}.txt` — 20 models, both configs,
  at the pinned revision; byte-identical to the override build
- `diag/<id>.jsonl` — DIAG-001 residual records, default-on, per model
- `reconcile.py`, `sweep.sh`, `sweep-pinned.sh`, `diag-sweep.sh`
- `HANDOFF.superseded-2026-08-06.md` — the previous edition of this file,
  preserved because it was uncommitted when this replaced it
