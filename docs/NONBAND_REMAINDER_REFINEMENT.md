# ABC corpus non-band remainder refinement

Post-cone face-level refinement of every face the ABC corpus still loses.
**Observer-only** — no admission, normalization, recovery, or validation
behaviour was added, relaxed, or touched, and no face changed acceptance.
The cone band, cylinder band, and gate-closed holdings all stand unchanged.
This packet reads the existing post-cone ledgers, joins them to the
`band_curve_probe` edge evidence, and bins the result into a refined
taxonomy whose purpose is to name the next missing theorem or algorithm —
not to rename every error more precisely.

```text
look        e2bf18ac7b57   (branch fix/correctness-phase-0-1)
truck-fork  2b4537c4e01de54e195c4fe732417b600457f417  (origin/fix/correctness-phase-0-1)
Cargo.lock  cb293d3a9ad018af905ca01775e2a2129ac179614cd776078a4de95bb780bf37
```

`nonband_report.py` reuses `remainder_report.py`'s classify logic verbatim
and extends it with the cone-band exit map that `remainder_report.py`
predates (the cone cell did not exist when it was written). No
`.cargo/config.toml` `paths` override and no Cargo path patch participated;
truck resolves from the pushed git revision `2b4537c4`. `HANDOFF.md` and
`opencode.json` were not touched. Nothing in `src/`, `truck-fork`, the
canonical corpus scripts, or the corpus itself was modified.

## Reproducing

```console
python benchmarks/nonband_report.py \
    --json  C:/Users/stefa/look-corpus/nonband/report.json \
    --ledger C:/Users/stefa/look-corpus/nonband/nonband.tsv.gz \
    --populations 35
```

The script reads only the two existing post-cone ledgers
(`C:/Users/stefa/look-corpus/cone-final/` and `C:/Users/stefa/look-corpus/cone-band/`),
both produced by the cone-band sweep under truck `2b4537c4` / look `e2bf18ac`,
and writes its own JSON + per-face ledger outside the repo. No Rust build is
required; no corpus re-run is required.

## Post-cone baseline (reproduced)

| | declared | rendered | lost | coverage |
|---|---|---|---|---|
| cone enabled (`cone-final`, band open) | 839,179 | 817,525 | 21,654 | 97.420% |

This reproduces `docs/ABC_CONE_BAND_SWEEP.md` to the face. Declared-face
population is unchanged (839,179 = 839,179). The cone band's 5,163 recoveries
are removed from the lost set; what remains is the non-band remainder this
packet refines.

## Coverage

```text
21,654 lost faces
21,654 carry a typed terminal failure reason      100.0%
21,527 join a source-authority probe record         99.4%
21,654 receive exactly one primary diagnosis      100.0%
14,981 join a band_curve_probe edge record          69.2%
      0 fall in the "not yet sufficiently instrumented" class
```

The 127 faces with no probe record are exactly the 127 lost during import
(unchanged from the pre-cone diagnosis). The 6,673 without a curve-probe
record are faces the curve probe did not enumerate (it samples band-exit
faces); the primary diagnosis does not depend on it.

## Post-cone primary diagnosis (cone-band exits mapped)

`remainder_report.py`'s `BAND_EXIT_DIAGNOSIS` predates the cone cell, so a
cone face the cone band refused fell through to the generic
`ConstraintInsertionIncomplete / SyntheticSyntheticCrossing` bucket and was
misread as `quotient_cell_not_named`. The cone cell is now named, so a
cone-band refusal is later evidence about the same face and wins over the
generic bucket — exactly as the cylinder band exits already do. The mapping:

```text
cone_band exit                         primary / subreason
unsupported_curve_representation       CurveBoundaryWitness / band_bound_curve_unreadable
cone_witness_start_not_on_cone         AtlasClassification / band_witness_refuted
cone_witness_circle_not_a_cone_parallel AtlasClassification / band_witness_refuted
cone_band_bound_not_one_occurrence     AtlasClassification / band_witness_refuted
```

With that mapping applied, the post-cone primary diagnosis is:

```text
CurveBoundaryWitness     8,085   37.3%      (pre-cone 7,706; +379 cone unsupported_curve moved here)
CutOpenOrArrangement     6,488   30.0%      (unchanged)
AtlasClassification      4,850   22.4%      (pre-cone 10,392; -5,542 = 5,163 recovered + 379 reclassified)
MaterialAuthority        2,103    9.7%      (unchanged)
SourceImport               127    0.6%      (unchanged)
MeshRealization              1    0.0%      (unchanged)
```

Broken out by exact subreason (post-cone):

```text
CurveBoundaryWitness / no_certified_preimage_on_support        6,695
CutOpenOrArrangement / source_source_crossing                  4,027
AtlasClassification  / deck_generator_uncertified              3,008
CutOpenOrArrangement / source_synthetic_crossing               1,786
AtlasClassification  / periodic_lift_branch_unresolved         1,492
MaterialAuthority    / parity_contradiction                    1,452
CurveBoundaryWitness / band_bound_curve_unreadable             1,390
MaterialAuthority    / no_material_region                        554
CutOpenOrArrangement / overlap_unsupported                       469
AtlasClassification  / band_witness_refuted                      350
CutOpenOrArrangement / band_lift_join_no_compatible_integer      161
SourceImport         / AllBoundsCollapsed                         97
MaterialAuthority    / band_outer_bound_authority_absent          97
CutOpenOrArrangement / mixed_conflict                              40
SourceImport         / EdgeCurveConversionFailed                   28
CutOpenOrArrangement / band_orientation_incompatible                5
SourceImport         / SurfaceConversionFailed                      2
MeshRealization      / constraint_role_missing                      1
```

`deck_generator_uncertified` (3,008) is now the largest AtlasClassification
subreason: the cone cell ate `quotient_cell_not_named` on cones, and what
remains is overwhelmingly torus (2,844) and sphere (122), where a period is
*declared* but nothing *certifies* a generator.

## Refined taxonomy

Each lost face is assigned to exactly one refined bin. The order matters and
is stated in `nonband_report.py:refined_bin`: operational failure and surface
singularity first, then recognized-cell realization gaps, then curve
mathematics, then material ambiguity, then source contradiction, then quotient
artifacts (unnamed cells), then boundary-homology insufficiency, then rank-0
arrangement defects. **Synthetic-synthetic crossings on a certified-rank
chart are treated as quotient artifacts (unnamed cell), not source
contradictions**, per the wave's required distinctions — the band packet
demonstrated that directly by naming the cell and removing the crossing
without touching arrangement code.

```text
unsupported_curve_mathematics      8,085   37.3%   boundary never witnessed (spline projection / unreadable bound)
unnamed_homogeneous_atlas_cell     4,507   20.8%   homogeneous signature, no cell named (incl. quotient artifacts)
source_contradiction               4,027   18.6%   source-source crossing (candidate; needs certified predicate)
material_semantic_ambiguity        2,103    9.7%   parity / no-region / outer-bound authority absent
insufficient_evidence              1,492    6.9%   periodic lift branch unresolved (needs boundary homology)
general_arrangement_problem        1,076    5.0%   rank-0 arrangement defect (overlap / mixed / synthetic closure)
recognized_cell_broken_realization   166    0.8%   named cell certified, realization stopped (deck translate / orientation)
operational_failure                  128    0.6%   source import or unwitnessed mesh failure
surface_singularity                   70    0.3%   cone band refused on apex / opposite-nappe witness
                                   ------
                                   21,654
```

By surface family, the refined bins cross-cut as follows (top entries):

```text
unsupported_curve_mathematics  nurbs      3,130      source_contradiction          plane     2,947
unnamed_homogeneous_atlas_cell torus      2,847      unnamed_homogeneous_atlas_cell cylinder  1,674
unsupported_curve_mathematics  cylinder   2,083      unsupported_curve_mathematics bspline   1,392
material_semantic_ambiguity    cylinder   1,277      unsupported_curve_mathematics torus      939
unnamed_homogeneous_atlas_cell cone         924      unnamed_homogeneous_atlas_cell sphere     502
```

### Declared against certified periodicity (post-cone, lost faces)

```text
torus      declared=2 certified=0   4,204      no generator at all (rank-2)
cone       declared=1 certified=1   1,788      the revolution witness applies (cone cell now named)
cylinder   declared=1 certified=1   5,524      the revolution witness applies (cylinder cell named)
sphere     declared=1 certified=0     691      no generator
revolved   declared=1 certified=0      69
extruded   declared=1 certified=0      32
plane / nurbs / bspline / offset       9,065      genuinely aperiodic charts
```

The torus is the only rank-2 surface in the corpus and the only family where
a period is declared but uncertified on every lost face. The cone and
cylinder generators are already certified; their remaining loss is downstream
of the cell, not the generator.

## Boundary curve class

```text
spline_present            14,536   67.1%      a B-spline or NURBS curve bounds this face
circle_and_linear          5,613   25.9%
linear_only                  933    4.3%
noncircular_conic_present    445    2.1%
none                         127    0.6%
```

Two thirds of the remaining loss carries a spline bound. The
`band_curve_probe` join confirms the unread cause on 14,981 faces: the
dominant causes are `b_spline_curve` / `rational_b_spline_curve` (no exact
curve-on-surface witness) and `arc_non_circular_affine_image` (an ellipse
that is an affine image of a circle, beyond the exact Gram predicate's
certified-equal bound).

## Per-population profile (the wave's questions)

For each major signature the ledger carries: ambient surface family;
locus regularity (the periodicity table above); single- or multi-chart
requirement (deck rank); boundary-component count (`bound_count`);
source-edge count (`edge_uses`); curve families (`curves` /
`bound_signature`); contractile vs essential loops (the band's
`witness_start_not_on_*` refusals separate essential parallels from
contractible loops); source-bound declarations (`outer_standing` /
`outer_declared_count`); material authority (`parity_contradiction` /
`no_material_region`); synthetic vs source crossings (`derived_bucket`); and
the first missing proof obligation (the ranked table below). The
representative faces cited are the lowest-id member of each population.

## Ranked candidate populations

```text
faces mdl  bin / surface / subreason                                  cr  representative
 3130  14  unsupported_curve_mathematics/nurbs/no_certified_preimage    0  00000414#78928 4[Bs2,Nu2]
 2947  16  source_contradiction/plane/source_source_crossing            0  00000730#35853 1[Ci1];6[Ln6]
 2844  14  unnamed_homogeneous_atlas_cell/torus/deck_generator_uncert.   0  00000730#36165 1[Ci1];1[Ci1]
 1392  15  unsupported_curve_mathematics/bspline/no_certified_preimage   0  00000414#79314 3[Bs3]
 1180  15  material_semantic_ambiguity/cylinder/parity_contradiction     1  00000730#35771 2[Bs1,Ln1];1[Ci1]
 1072   9  unsupported_curve_mathematics/cylinder/no_certified_preimage  1  00000730#35281 2[Ln2]
 1023  13  unnamed_homogeneous_atlas_cell/cylinder/source_synthetic_xing 1  00000730#35933 4[Bs4];4[Bs4]
 1011   8  unsupported_curve_mathematics/cylinder/band_bound_unreadable  1  00000730#64031 1[Ci1];6[Bs6]
  939  10  unsupported_curve_mathematics/torus/no_certified_preimage     0  00000730#39211 1[Bs1];1[Bs1]
  673   6  insufficient_evidence/cone/periodic_lift_branch_unresolved    1  00001075#50085 1[Bs1];1[Bs1]
  426   4  insufficient_evidence/cylinder/periodic_lift_branch_unres.    1  00000730#56025 1[Bs1];1[Bs1]
  424   4  material_semantic_ambiguity/plane/no_material_region          0  00000730#35645 4[Ln4]
  380   4  insufficient_evidence/sphere/periodic_lift_branch_unresolved  0  00000730#35715 3[Ci3]
  379   6  unsupported_curve_mathematics/cone/band_bound_curve_unreadable 1  00000959#13842 4[Bs4];3[Bs3]
  321  13  source_contradiction/bspline/source_source_crossing           0  00000414#79004 2[Bs2];3[Bs3]
  272  14  source_contradiction/nurbs/source_source_crossing             0  00000414#78996 7[Bs3,Nu4]
  225  13  unnamed_homogeneous_atlas_cell/cylinder/band_witness_refuted  1  00000730#40191 4[Ci4];1[Ci1]
  218  12  source_contradiction/cylinder/source_source_crossing          1  00000730#86681 4[Bs2,Ci2]
  205  13  general_arrangement_problem/torus/source_synthetic_crossing   0  00000730#38357 1[Ci1];4[Ci2,El2]
  196  10  unnamed_homogeneous_atlas_cell/cone/source_synthetic_crossing 1  00000730#62353 1[Ci1];4[Ci2,Ln2]
  161   7  recognized_cell_broken_realization/cylinder/band_lift_join    1  00000730#46415 5[Ci5];1[Ci1]
  152  11  material_semantic_ambiguity/torus/parity_contradiction        0  00000730#35399 1[Ci1];1[Ci1]
  147  10  general_arrangement_problem/plane/overlap_unsupported         0  00000730#35609 3[Ln3]
  136   7  general_arrangement_problem/cone/overlap_unsupported          1  00000730#35441 6[Bs4,Ci2]
  129   3  material_semantic_ambiguity/bspline/no_material_region        0  00000730#35629 4[Bs2,Ln2]
  122   3  unnamed_homogeneous_atlas_cell/sphere/deck_generator_uncert.  0  00000730#65213 1[Ci1];1[Ci1]
   70   7  surface_singularity/cone/band_witness_refuted                 1  (apex / opposite-nappe refusals)
   97   4  operational_failure/-/AllBoundsCollapsed                  None  00000414#81588 -
```

`cr` = certified deck rank. `bound_signature` reads `n[Fk,…]` per bound: `n`
edge uses carrying `k` curves of each imported family (`Ci` circle, `El`
ellipse, `Ln` line, `Bs` B-spline, `Nu` NURBS). It is a source datum, not a
shape claim.

### The exact missing proof obligation, per population

```text
3130  nurbs/no_certified_preimage: exact curve-on-surface preimage for a NURBS
      bound on a NURBS support (free-form / free-form intersection).
2947  plane/source_source_crossing: certified arrangement predicate admitting
      or refuting source self-intersection on a rank-0 chart, or a proof the
      source wires genuinely cross.
2844  torus/deck_generator_uncertified: representation-derived rank-2 toroidal
      period witness (both angular generators), then the toroidal essential-
      band cell and its quotient cut plan. The band route already handles the
      malformed double-outer-bound normalization; the blocker is the period
      witness and the cell, not the arrangement.
1392  bspline/no_certified_preimage: exact curve-on-surface preimage for a
      B-spline bound on a B-spline support.
1180  cylinder/parity_contradiction: certified arrangement predicate that
      resolves the dual-parity flood, or a proof the source is inconsistent.
1072  cylinder/no_certified_preimage: exact curve-on-cylinder preimage (the
      cell is certified; the curve witness is the gap).
1023  cylinder/source_synthetic_crossing: quotient cut plan for the cylinder
      that cannot cross authoritative source trim. The cell exists; the cut
      plan is the gap.
1011  cylinder/band_bound_curve_unreadable: exact curve-on-cylinder witness
      for the unread spline / non-circular conic bound.
 939  torus/no_certified_preimage: exact curve-on-torus preimage (prerequisite:
      the torus period witness).
 673  cone/periodic_lift_branch_unresolved: boundary homology of the cone face
      so the periodic lift branch can be chosen. The cone cell exists.
 426  cylinder/periodic_lift_branch_unresolved: boundary homology of the
      cylinder face. The cylinder cell exists.
 380  sphere/periodic_lift_branch_unresolved: boundary homology; a sphere cell
      and its rank-2 period witness are also missing.
 379  cone/band_bound_curve_unreadable: exact curve-on-cone witness for the
      unread spline bound.
 225  cylinder/band_witness_refuted: name a different cylinder cell — the band
      candidate is refuted on positive evidence.
 196  cone/source_synthetic_crossing: quotient cut plan for the cone that
      cannot cross authoritative source trim. The cone cell exists.
 161  cylinder/band_lift_join_no_compatible_integer: prove a compatible deck
      translate joins the lift, or prove none exists.
  70  cone/band_witness_refuted (surface_singularity): prove the carriers lie
      on one nappe of the apex (same-nappe, both radii nonzero), or name a
      cell for the opposite-nappe / apex-crossing case the cone band refuses.
```

## The wave's eight questions

1. **Largest remaining homogeneous population.** Torus
   `deck_generator_uncertified`, **2,844 faces in 14 models**. 2,404 of them
   carry the `1[Ci1];1[Ci1]` bound signature (two complete source circles);
   2,585 carry the malformed double-`FACE_OUTER_BOUND` pattern
   (`outer_standing=multiply_declared`) the cylinder band already normalizes.
   Every lost torus face is `declared=2 certified=0` — no generator at all.
   This is the cone-band-sweep doc's named follow-on, and it is the most
   homogeneous large population in the post-cone corpus.

2. **Largest fitting an existing realization substrate.** Cylinder
   `source_synthetic_crossing`, **1,023 faces in 13 models** (certified
   rank 1, the cylinder band cell is named). The realization substrate
   exists; the gap is a quotient cut plan that cannot cross authoritative
   source trim. The cone analogue (196 faces, the cone cell is named) brings
   the "named cell, broken cut plan" total to 1,219. The purest
   recognized-cell realization gap is cylinder
   `band_lift_join_no_compatible_integer` (161): the cell is certified, the
   deck-translate join is the only missing proof.

3. **Largest requiring a new atlas-cell theorem.** Torus
   `deck_generator_uncertified`, **2,844**. A rank-2 toroidal essential-band
   cell does not exist, and its prerequisite — a representation-derived
   rank-2 period witness — does not exist either. The sphere (502 + 122)
   is the smaller new-cell population of the same kind (rank-2, no
   generator).

4. **Largest requiring general arrangement machinery.** Plane
   `source_source_crossing`, **2,947 faces in 16 models** (rank-0 chart,
   no quotient to blame). Needs a certified arrangement predicate that
   admits or refutes source self-intersection. Plane `overlap_unsupported`
   (147) and the rank-0 `mixed_conflict` / `synthetic_closure_self_crossing`
   populations are the smaller arrangement-predicate gaps.

5. **Largest blocked only by material authority.** Cylinder
   `parity_contradiction`, **1,180 faces in 15 models** (certified rank 1,
   `CandidateAtlasCell`). The dual-parity flood contradicts; needs a
   certified arrangement predicate or a proof the source material regions
   are genuinely inconsistent.

6. **Largest requiring spline intersections.** Nurbs
   `no_certified_preimage_on_support`, **3,130 faces in 14 models** — exact
   curve-on-surface preimage for a NURBS bound on a NURBS support (free-form
   / free-form intersection). Spline-bound lost faces total **14,536
   (67.1%)**: cylinder 3,742, nurbs 3,282, bspline 2,018, plane 1,913, cone
   1,616, torus 1,606. The exact spline-on-surface witness is the single
   highest-impact theorem in the remainder.

7. **Largest containing surface singularities.** Cone
   `band_witness_refuted` (surface_singularity), **70 faces** — the cone
   band's own certifier refused because the witness does not lie on one
   nappe of the certified cone (`cone_witness_start_not_on_cone` 63,
   `cone_witness_circle_not_a_cone_parallel` 7). This is the apex /
   opposite-nappe stratum the cone packet's proof obligations explicitly
   flag; the refusal is correct, and the missing work is either a same-nappe
   proof or a cell for the apex-crossing case.

8. **Likely genuinely ambiguous.** Plane `source_source_crossing`,
   **2,947**. On a rank-0 chart no quotient can be blamed, so a source-source
   crossing is either a genuine source self-intersection or an arrangement
   defect, and nothing currently distinguishes them. They stay genuinely
   ambiguous until a certified arrangement predicate exists. Cylinder
   `parity_contradiction` (1,180) is the material-authority analogue:
   genuine inconsistency cannot be ruled out without the predicate.

## Recommended next population

The cone-band-sweep doc names the torus period witness as the natural
follow-on, and the post-cone ledger confirms it is the largest homogeneous
population (2,844, of which 2,404 are the two-circle signature and 2,585
carry the malformed double-outer-bound the band route already normalizes).
It is also the most expensive: it needs **two** new theorems — a
representation-derived rank-2 period witness, then the toroidal band cell —
and the rank-one annulus realizer the cone and cylinder cells share does
not apply to a rank-2 deck.

The lower-effort, high-confidence alternative that fits an **existing**
named cell is the cylinder `source_synthetic_crossing` (1,023) plus the cone
analogue (196): both cells are already named and their realizers built; the
gap is a quotient cut plan that respects authoritative source trim. The
recognized-cell realization gap on the cylinder
(`band_lift_join_no_compatible_integer`, 161) is the smallest and cleanest
proof obligation of the lot.

The highest-impact single theorem is the exact spline-on-surface witness
(unlocks 8,085 faces across every surface family), but it is the hardest:
free-form / free-form intersection. It is a research investment, not a
packet.

## What was added

```text
benchmarks/nonband_report.py   new, observer-only — reads the existing post-cone
                                ledgers, adds the cone-band exit map, joins the
                                band_curve_probe evidence, bins into the refined
                                taxonomy. Reuses remainder_report.py verbatim.
docs/NONBAND_REMAINDER_REFINEMENT.md  this report.
```

Nothing in `src/`, `truck-fork`, `Cargo.toml`/`Cargo.lock`,
`.cargo/config.toml`, `HANDOFF.md`, `opencode.json`, or any canonical corpus
script was modified. No new Rust example was needed — the ledgers and the
existing `band_curve_probe` reading carried every field the taxonomy requires.

## Artifacts

Outside the repository (not committed; multi-megabyte generated ledgers):

```text
C:/Users/stefa/look-corpus/nonband/
    report.json          machine-readable summary, schema nonband-refine-1
    nonband.tsv.gz       full per-face ledger, 21,654 rows, 40 columns
                         sha256 ee3d965a9e22df8eb56cb85b4032acab4162131a33b45a0634fa088635d7ea51
C:/Users/stefa/look-corpus/cone-final/   40 runs (census_diag + source_probe), read-only input
C:/Users/stefa/look-corpus/cone-band/    60 runs (gate_closed + band_enabled + curve_probe), read-only input
```
