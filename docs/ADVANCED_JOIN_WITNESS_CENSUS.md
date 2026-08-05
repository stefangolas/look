# Advanced join and boundary-witness census

A DEFINITIVE observer-only census of why faces fail AFTER passing the original
curve-family gate, across the 20-model ABC corpus. **Census only** — no face
was recovered, no admission changed, no production source touched, no central
enum altered, no tolerance widened. The classification authority for every
exit is the truck-meshalgo source's own `SliceCategory`, read verbatim.

```text
look (worktree HEAD)   92112c5e055bd62e95faacccaeab0d73a4a4f4d4  (cone foundation)
look (sweep that produced the ledgers)  e2bf18ac7b57
truck-fork             2b4537c4e01de54e195c4fe732417b600457f417  (READ ONLY)
Cargo.lock             cb293d3a9ad018af905ca01775e2a2129ac179614cd776078a4de95bb780bf37
```

The ledgers this reads were produced by the cone-final sweep
(`docs/ABC_CONE_BAND_SWEEP.md`) at `look e2bf18ac` / `truck 2b4537c4`. This
packet adds no new sweep; it re-reads those ledgers and classifies the
advanced-exit population. No `.cargo/config.toml` path override participated;
`HANDOFF.md` and `opencode.json` were not touched.

## Reproducing

```console
python benchmarks/join_witness_report.py \
    --out C:/Users/stefa/look-corpus/cone-final \
    --probe-out C:/Users/stefa/look-corpus/join-witness \
    --json C:/Users/stefa/look-corpus/join-witness/report.json
```

No build is required. The script reads the existing cone-final ledgers
(`ledger.tsv.gz`, `diag.jsonl.gz`, `faces.tsv.gz`) and the cone-band curve
probe (`curves.tsv.gz`), joins them on `source_face_id` as
`remainder_report.py` does, and writes the per-face ledger +
`report.json` under `C:\Users\stefa\look-corpus\join-witness\` (external, not
committed).

## Target population

Every band-eligible face whose refusal is NOT `unsupported_curve_representation`
(the curve-family gate), NOT `missing_outer_bound_authority` (the
material-authority gate), and NOT `recovered`. These are the faces that passed
the original curve-family gate and failed at a more advanced stage. Counts are
taken from the cone-final ledger's `band` and `cone_band` columns:

```text
lift_join_no_compatible_integer         161   cylinder, deck-walk join
witness_sweep_does_not_reach_endpoint    87   cylinder, witness
witness_start_not_on_cylinder            72   cylinder, witness
witness_circle_not_a_cylinder_parallel   64   cylinder, witness
witness_not_constant_axial_coordinate      2   cylinder, witness
band_orientation_incompatible              5   cylinder, orientation
cone_witness_start_not_on_cone            63   cone, witness
cone_band_bound_not_one_occurrence        55   cone, bound structure
cone_witness_circle_not_a_cone_parallel    7   cone, witness
                                       ----
                                         516
```

= **161 join + 225 cylinder witness + 130 other** (70 cone witness + 55 cone
bound + 5 orientation), exceeding the >=386 minimum. The population partitions
cleanly by surface family: 391 cylinder + 125 cone = 516 (no face carries both
a cylinder-band and a cone-band typed exit; the surface selects the route).

## Required census output

```text
total targeted                          516
fully explained                         516
not instrumented                          0
missing source evidence                   0
branch-selection candidate                0
orientation candidate                     5
source inconsistency                    390
multi-piece supported in principle      55
operational failure                       0
other typed classes                      66
```

Every targeted face carries a typed band exit (the certifier's own verdict),
a source-probe record, and a structured `FailedFaceDiagnosis`, so all 516 are
fully explained and none is uninstrumented. The single documented gap is the
deck solver's per-face numeric detail for the 161 join faces
(`missing_constraint_numeric_detail = 161`); see the dedicated section below.
It does not prevent classification — the typed exit + signature are sufficient
— so it is reported separately, not as "not instrumented".

## Classification authority

Each exit's `SliceCategory` is taken VERBATIM from the truck-meshalgo source
(READ ONLY), not inferred:

```text
cylinder_lift.rs:155   CylinderLiftExit::category
curve_witness.rs:144   WitnessFailure::category
cylinder_band.rs:390   BandExit::category
cone_band.rs:191       ConeWitnessFailure::category
cone_band.rs:463       ConicalBandExit::category
cone_band.rs:535       deck_join_category
```

The authority line (curve_witness.rs:135-143) is *authority*, not severity:

- a curve whose endpoints are not on the certified surface, or whose own
  declared sweep does not reach its own declared endpoint, contradicts a claim
  the source itself makes — `Inconsistent`;
- a curve that is simply not axial, not at constant axial coordinate, or not
  the parallel it is presented as, is valid geometry the supported subset does
  not cover — `Unsupported`;
- a non-finite input is a machine fact — `OperationalFailure`.

```text
slice category (source authority)     faces    %
Inconsistent                          390   75.6   -> all source_inconsistency
Unsupported                           126   24.4   -> 55 multi-piece + 66 other + 5 orientation
```

### The authorized cylinder/cone asymmetry

On a CYLINDER, `CircleNotACylinderParallel` is `Unsupported`
(curve_witness.rs:154): the circle is valid geometry, just not the parallel
the band witness certifies. On a CONE, `CircleNotAConeParallel` is
`Inconsistent` (cone_band.rs:194): a circle *presented as* a cone parallel
that isn't one contradicts the source. This census respects that distinction
rather than collapsing it, so the 64 cylinder `witness_circle_not_a_cylinder_parallel`
faces are `other_typed_classes` while the 7 cone
`cone_witness_circle_not_a_cone_parallel` faces are `source_inconsistency`.

## Per-exit classification

| exit | faces | slice (source) | primary | subreason | pop. question |
|---|---|---|---|---|---|
| `lift_join_no_compatible_integer` | 161 | Inconsistent | source_inconsistency | multi_piece_period_join_inconsistent | Q8 (+Q2) |
| `witness_sweep_does_not_reach_endpoint` | 87 | Inconsistent | source_inconsistency | contradictory_boundary_sweep | Q8 |
| `witness_start_not_on_cylinder` | 72 | Inconsistent | source_inconsistency | source_vertex_off_certified_surface | Q3 |
| `witness_circle_not_a_cylinder_parallel` | 64 | Unsupported | other_typed_classes | unsupported_boundary_placement | Q10 |
| `cone_witness_start_not_on_cone` | 63 | Inconsistent | source_inconsistency | source_vertex_off_certified_surface | Q3 |
| `cone_band_bound_not_one_occurrence` | 55 | Unsupported | multi_piece_supported_in_principle | bound_not_one_complete_circle_occurrence | Q2 |
| `cone_witness_circle_not_a_cone_parallel` | 7 | Inconsistent | source_inconsistency | contradictory_boundary_placement | Q8 |
| `band_orientation_incompatible` | 5 | Unsupported | orientation_candidate | carrier_homology_same_sign | Q5 |
| `witness_not_constant_axial_coordinate` | 2 | Unsupported | other_typed_classes | unsupported_arc_geometry | Q7 |

## The 161 join faces — a single homogeneous subpopulation

Every one of the 161 `lift_join_no_compatible_integer` faces is identical on
every retained field:

```text
surface_kind            cylinder             161/161
outer_standing          multiply_declared    161/161
lift_status             Certified            161/161
projection_status       Successful           161/161
terminal_reason         ConstraintInsertionIncomplete   161/161
derived_bucket          SyntheticSyntheticCrossing      161/161
conflict_count          1                    161/161
conflict_origin         Seam/Seam            161/161
conflict_relation       ProperInteriorCrossing          161/161
synthetic_segment_count 2                    161/161
deck_status (diag)      Unavailable          161/161
```

`deck_status: Unavailable` is a generic-diagnosis field: it is `Unavailable`
because the band route produced its own typed exit, so the generic deck
diagnosis was not computed. It is NOT the band deck solver's outcome (that is
the `band` column value). All 516 targeted faces carry it.

The bound signature is uniformly **multi-piece**: one bound is a single
complete circle (`1[Ci1]`) and the other is N complete circles
(`2[Ci2]`, `6[Ci6]`, `12[Ci12]`, …). Zero of the 161 are single-piece
`1[Ci1];1[Ci1]`. Concentration: `00003902` 97, `00005760` 46, `00009190` 10,
then four more models down to 1.

This is the deck solver (`solve_axis_aligned`, deck.rs:513) proving, via a
certified false-negative-free enclosure, that the developed displacement
around the multi-piece bound is not compatible with any integer multiple of
the certified 2π angular period. The `SliceCategory::Inconsistent` verdict
(cylinder_lift.rs:159) is a positive proof, not a "not supported" gap: the
multi-circle bound's circles are not deck translates of the single circle.

## JoinNoCompatibleInteger constraint structure (READ-ONLY source)

The deck solver checks four constraints IN ORDER. `NoCompatibleInteger` is
returned by C1, C3, or C4 — never by C2:

```text
C1  aperiodic_contains_zero     deck.rs:520  -> NoCompatibleInteger   (strict)
    The aperiodic (generator-orthogonal) displacement must contain zero.
    The shared source vertex must have the same aperiodic coordinate
    through both occurrences it joins.

C2  period_resolvable           deck.rs:535  -> Indeterminate          (epistemic)
    The period must exceed the f64 ULP at the displacement scale. If not,
    the enclosure cannot count compatible integers -> Indeterminate,
    NOT NoCompatibleInteger.

C3  quotient_range_nonempty     deck.rs:540  -> NoCompatibleInteger   (strict)
    The conservative integer range [k_min, k_max] from the outward-rounded
    quotient must be non-empty. k_min > k_max -> NoCompatibleInteger.

C4  compatible_integer_exists   deck.rs:546  -> NoCompatibleInteger   (strict)
    At least one integer k in [k_min, k_max] must have k*period inside the
    periodic enclosure (constant-time first/last_compatible check).
```

`solve_join` (rank1_annulus.rs:1251) widens each join's developed
displacement by `certified_join_tolerance` (`JOIN_EVALUATION_ULPS = 8.0`,
rank1_annulus.rs:1221) before calling `solve_axis_aligned`, so a true
zero-holonomy join is not refused by floating-point noise.
`propagate_deck_placements` (rank1_annulus.rs:1288) walks the occurrence
chain in order and stops at the FIRST join that does not resolve uniquely,
returning `DeckJoinFailure::NoCompatibleInteger { join_index }`. The FIRST
pair of constraints that makes the set empty is therefore: the first join
(between occurrence `join_index` and the next, cyclically) at which C1, C3,
or C4 returns `NoCompatibleInteger`.

### The documented gap (per-face numeric detail)

The production diagnostic sink retains the typed exit
`lift_join_no_compatible_integer` but does NOT retain:

- `join_index` (which join failed first);
- which of C1 / C3 / C4 fired;
- the candidate integer range `[k_min, k_max]`;
- the per-constraint feasible intervals and their intersection;
- whether the contradiction is strict (C1/C3/C4) or unresolved (C2).

The contradiction is strict by construction (C2 returns `Indeterminate`, not
`NoCompatibleInteger`, so any face that reaches this exit failed C1, C3, or
C4 — all strict). Which of the three, and the numeric values, are the gap.

This signal lives inside the PRIVATE `solve_axis_aligned` /
`propagate_deck_placements` path in
`truck-meshalgo/src/tessellation/formal/{deck,rank1_annulus}.rs`. Deriving it
observer-only would require either instrumenting that private solver (modifying
truck — out of scope) or re-implementing the whole band admission + lift +
projection + deck-walk pipeline in an example (recovery-adjacent,
divergence-prone, and forbidden from adding deps or modifying manifests). A
Rust example was therefore evaluated and deliberately NOT added: the ledger +
source analysis is sufficient to classify the population, and the numeric
detail is a precisely-documented gap rather than an uninstrumented face.

## Later boundary-witness exits

The 225 cylinder witness + 70 cone witness faces break into two structural
families by bound signature:

- **Single-piece** (`1[Ci1];1[Ci1]`): the `*_start_not_on_*` and
  `*_circle_not_a_*_parallel` exits. These are faces whose two bounds are each
  one complete circle, but a vertex is off the certified surface, or the
  circle's placement is not the surface's parallel. 189 of the 516 faces carry
  `1[Ci1];1[Ci1]`.
- **Multi-piece** (`1[Ci1];N[CiN]` or `N[CiN];1[Ci1]`): the
  `witness_sweep_does_not_reach_endpoint` (87, all multi-piece),
  `band_orientation_incompatible` (5, all multi-piece), and
  `witness_not_constant_axial_coordinate` (2, multi-piece) exits, plus the
  cone `cone_band_bound_not_one_occurrence` (55, all multi-piece).

The multi-piece witness exits (`witness_sweep_does_not_reach_endpoint`) are
`Inconsistent`: the source's own declared angular sweep, developed from the
start angle, does not land on the declared endpoint — the multi-circle bound's
sweep contradicts its declared endpoint. This is the same multi-piece
substructure as the join faces, refused one stage earlier (at the witness,
before the deck walk).

## Major populations

```text
faces mdl  exit                                       surface   bound_sig
   59  10  witness_start_not_on_cylinder              cylinder  1[Ci1];1[Ci1]
   52   4  lift_join_no_compatible_integer            cylinder  2[Ci2];1[Ci1]
   50   6  cone_witness_start_not_on_cone             cone      1[Ci1];1[Ci1]
   50   2  lift_join_no_compatible_integer            cylinder  1[Ci1];6[Ci6]
   48   1  lift_join_no_compatible_integer            cylinder  6[Ci6];1[Ci1]
   36   4  witness_circle_not_a_cylinder_parallel     cylinder  1[Ci1];1[Ci1]
   28   2  witness_sweep_does_not_reach_endpoint      cylinder  6[Ci6];1[Ci1]
   25   1  witness_circle_not_a_cylinder_parallel     cylinder  1[Ci1];1[Ci1]
   20   3  cone_band_bound_not_one_occurrence         cone      2[Ci2];1[Ci1]
   15   2  cone_band_bound_not_one_occurrence         cone      1[Ci1];2[Ci2]
   15   3  witness_sweep_does_not_reach_endpoint      cylinder  1[Ci1];6[Ci6]
   12   1  witness_sweep_does_not_reach_endpoint      cylinder  1[Ci1];12[Ci12]
```

Model concentration: `00005760` 120, `00001075` 116, `00003902` 97,
`00006483` 95, then ten more down to 1. Four models (`00009972`, `00005642`,
`00005427`, `00007744`-`00008001` pair aside) carry few or none; the
population is concentrated in the four high-count models.

## Answers to the ten population questions

**(1) One incorrect branch-selection rule?** No. Zero faces are
`branch-selection candidate`. No exit is `SliceCategory::Unresolved` (the
branch-unresolved population was blocked earlier, at
`periodic_lift_branch_unresolved`, before the band route). Every advanced-exit
face has a decisive verdict (`Inconsistent` or `Unsupported`), not an
unresolved branch.

**(2) Multi-piece circular bands already supported in principle?** Partially.
55 cone `cone_band_bound_not_one_occurrence` faces (Unsupported, multi-piece)
are exactly this: the cone cell exists, the bound is multi-piece and does not
match the "one occurrence of a complete circle" admission, and a multi-piece
cell could support them in principle. The 161 join faces are the related but
distinct case: multi-piece bounds that are PROVED inconsistent (Inconsistent,
not Unsupported) — the deck solver certifies the multi-circle bound is not a
periodic translate of the single circle, so they are not merely unsupported
but refuted as essential bands. A multiply-connected-cell route is the
mathematics both would need; the 55 are "supported in principle", the 161 are
"proved not a band".

**(3) Source vertices with incompatible geometry?** Yes — 135 faces
(72 `witness_start_not_on_cylinder` + 63 `cone_witness_start_not_on_cone`).
The traversal start vertex is certified not to lie on the surface the face is
trimmed from (Inconsistent). These are predominantly single-piece
(`1[Ci1];1[Ci1]`): well-formed two-circle bands whose vertex position
contradicts the certified surface.

**(4) Geometry-compatible but source-distinct vertices?** Not observed in this
population. The deck walk joins at shared source vertices (the traversal's
endpoint pairing is proved cyclically continuous by source identity); a
`JoinNoCompatibleInteger` exit means the shared vertex's developed
displacement is inconsistent, not that distinct vertices were treated as
shared. No exit in the target set corresponds to source-distinct vertices.

**(5) Orientation folded twice?** Yes — 5 faces `band_orientation_incompatible`
(Unsupported). The two induced boundary homologies have the same sign
(cylinder_band.rs:397 / cone_band.rs:469); they do not bound a strip. All 5
are multi-piece cylinder faces. Refused, not repaired by reversing.

**(6) Incorrect integer-lift propagation?** Not as a defect. The integer-lift
propagation (`propagate_deck_placements`) is certified and stops at the first
non-unique join; the 161 join exits are the propagation correctly *refusing*
an inconsistent lift, not propagating an incorrect one. The propagation is
observer-verified sound (false-negative-free `provably_outside`).

**(7) Complete-circle vs arc confusion?** Marginally — 2 faces
`witness_not_constant_axial_coordinate` (Unsupported). A circumferential-arc
candidate whose endpoints do not share an axial coordinate (a helical/tilted
arc). Valid geometry outside the subset, not a circle/arc misclassification
in the certifier.

**(8) Genuine contradictory source boundaries?** Yes — the largest class, 255
faces (161 join + 87 sweep + 7 cone-circle). All `Inconsistent`: the source's
own boundary contradicts a claim it makes (the multi-piece period does not
reconcine, or the declared sweep does not reach the declared endpoint, or a
cone-parallel circle is not the parallel it is presented as).

**(9) Operational precision/subdivision limits?** No. Zero faces are
`operational_failure`. No exit is `SliceCategory::OperationalFailure` or
`Indeterminate` (C2). The `certified_join_tolerance` (8 ULP) enclosure
ensures no true join is refused by floating-point noise; every refusal is a
geometric fact, not a precision limit.

**(10) Several distinct subpopulations?** Yes. The 516 resolve into six
distinct subpopulations by (exit, slice_category, signature): the 161
multi-piece join (Q8), 135 vertex-off-surface (Q3), 87 multi-piece
sweep-contradiction (Q8), 64 single-piece unsupported-placement (Q10), 55
multi-piece bound-structure (Q2), 7 cone-parallel-contradiction (Q8), 5
same-sign orientation (Q5), 2 unsupported-arc (Q7). The dominant signal is
multi-piece circular bounds (161 + 87 + 55 + 5 + 2 = 310 of 516, 60.1%) and
vertex-off-surface (135, 26.2%).

## Scope and exclusions held

No production rendering, band admission, central failure enum, tolerance,
source topology, `Cargo.toml`/`Cargo.lock`/`.cargo/config.toml`, `HANDOFF.md`,
or `opencode.json` was modified. No generated ledger was committed. The
truck-fork was read only. The cone-final ledgers were re-read, not re-run.

## Artifacts

Outside the repository (not committed; generated ledgers):

```text
C:\Users\stefa\look-corpus\join-witness\join_witness.tsv.gz   516 rows, 43 fields
C:\Users\stefa\look-corpus\join-witness\report.json           machine-readable summary
```

Committed in this packet:

```text
benchmarks/join_witness_report.py   the census script (Python-ledger-first, no build)
docs/ADVANCED_JOIN_WITNESS_CENSUS.md this report
```

## Recommended recovery packets

Ordered by population size and mathematical readiness:

1. **Multiply-connected cylindrical cell** (161 + 87 + 5 + 2 = 255 cylinder
   multi-piece faces). The largest coherent prize. These are faces whose one
   bound is a single complete circle and whose other bound is N complete
   circles that are NOT deck translates of the first. The deck solver proves
   the inconsistency; a multiply-connected-region cell (a disk with N−1 holes
   on the cylinder, modulo the angular deck) is the mathematics. Prerequisite:
   a certified arrangement predicate for the multi-circle bound, and deck-walk
   instrumentation to emit `join_index` + the firing constraint so the cell
   can distinguish "not a periodic translate" (recoverable as a
   multiply-connected region) from "vertex axial-coordinate contradiction"
   (C1, genuinely inconsistent).

2. **Vertex-off-surface repair** (135 faces, single-piece). The start vertex
   is certified off the surface. Either a vertex-position re-certification
   against the surface (a provenance/precision packet) or a witness that
   tolerates the certified gap. Predominantly `1[Ci1];1[Ci1]` two-circle
   bands — structurally valid bands with one bad vertex.

3. **Cone multi-piece bound** (55 faces). `cone_band_bound_not_one_occurrence`
   — the cone cell exists; extend its admission from "one occurrence of a
   complete circle" to a multi-piece bound. Lower risk than (1) because the
   cell already certifies the single-piece case; this is an admission
   extension, not new mathematics.

4. **Cylinder unsupported-placement** (64 faces). `witness_circle_not_a_
   cylinder_parallel` (Unsupported, not Inconsistent). A bounding circle is
   valid geometry but not a cylinder parallel. Investigate whether the surface
   certification picked the wrong cylinder, or whether these are genuinely
   non-parallel circles needing a different cell. Distinct from (1)-(3); not
   ready for recovery until the placement discrepancy is characterized per
   face.

5. **Cone-parallel contradiction** (7 faces). `cone_witness_circle_not_a_
   cone_parallel` (Inconsistent). Small; investigate per face whether the
   circle placement or the cone certification is wrong.

6. **Orientation** (5 faces). `band_orientation_incompatible` — same-sign
   homology. Trivial count; handle individually once the multi-piece cell (1)
   exists, since all 5 are multi-piece.

### Deck-solver instrumentation prerequisite

Recovery packets (1) and (2) both need the deck solver to emit, per join:
`join_index`, the firing constraint (C1/C3/C4), the candidate range
`[k_min, k_max]`, and the periodic/aperiodic intervals. This is a one-line
extension to the production diagnostic sink (NOT to the solver logic itself),
retaining the values `solve_axis_aligned` already computes. It changes no
admission and no verdict; it only surfaces what the private solver already
decides, so the multiply-connected cell can separate the C1
(vertex-contradiction) subpopulation from the C3/C4 (period-translate)
subpopulation. This packet is observer-only and does not attempt it.
