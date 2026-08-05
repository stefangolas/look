# ABC corpus post-circle remainder diagnosis

Every face the ABC corpus still loses after the source-`CIRCLE` provenance
repair, classified to one primary obstruction. **Diagnosis only** — no
admission, normalization, recovery or validation behaviour was added, relaxed or
touched, and no face changed acceptance. The two new binaries are pure
observers; the census binary is the shipped one, unmodified.

```text
look        (this commit)     was 3a6fefb37747 when measured
truck-fork  73db1851          unchanged by this packet
Cargo.lock  99c6490223e24b14  unchanged by this packet
```

`remainder_sweep.py --check-pins` passes and the sweep refuses to run otherwise:
no `.cargo/config.toml` `paths` override and no Cargo path patch participated in
any number below, and truck resolved from the pushed git revision throughout.
`HANDOFF.md` and `opencode.json` were not touched.

## Reproducing

```console
cargo +stable-x86_64-pc-windows-gnullvm build --release \
    --target x86_64-pc-windows-gnullvm --example face_census --example remainder_probe
python benchmarks/remainder_sweep.py --dir C:/Users/stefa/look-corpus/abc \
    --out C:/Users/stefa/look-corpus/remainder-out
python benchmarks/remainder_report.py --out C:/Users/stefa/look-corpus/remainder-out \
    --json  C:/Users/stefa/look-corpus/remainder-out/report.json \
    --ledger C:/Users/stefa/look-corpus/remainder-out/remainder.tsv.gz
```

40 runs (20 models × {census + diagnosis, source probe}), one isolated process
each. **All 40 completed** — no crash, timeout, parse failure, or resource
failure.

## Post-circle baseline

Reproduced from the current committed heads, and it agrees exactly with the
circle-provenance packet:

| | declared | rendered | lost |
|---|---|---|---|
| gate closed | 839,179 | 797,239 | 41,940 |
| band enabled, before circle repair | 839,179 | 802,918 | 36,261 |
| band enabled, after circle repair | 839,179 | **812,362** | **26,817** |

```text
net gain from the circle-provenance repair   +9,444 faces
band eligible    16,622   band recovered 15,123   band refused 1,499
```

Two of the twenty models (`00009272`, `00005642`) now render every declared
face. `docs/CIRCLE_PROVENANCE_RESULT.md` printed the remaining loss as 26,797;
the correct figure, and the one its own `report.json` carries, is 26,817. That
line is corrected in this commit.

## Coverage

```text
26,817 lost faces
26,817 carry a typed terminal failure reason      100.0%
26,690 join a source-authority probe record        99.5%
26,817 receive exactly one primary diagnosis      100.0%
     0 fall in the "not yet sufficiently instrumented" class
```

The 127 faces with no probe record are exactly the 127 lost during import: they
never became a topological face, so there is no imported face for the probe to
read. No duplicate join keys and no unkeyed diagnosis rows in any model.

## Primary diagnosis

```text
AtlasClassification     10,392   38.8%
CurveBoundaryWitness     7,706   28.7%
CutOpenOrArrangement     6,488   24.2%
MaterialAuthority        2,103    7.8%
SourceImport               127    0.5%
MeshRealization              1    0.0%
```

Broken out by exact subreason:

```text
CurveBoundaryWitness / no_certified_preimage_on_support        6,695
AtlasClassification  / quotient_cell_not_named                 5,667
CutOpenOrArrangement / source_source_crossing                  4,027
AtlasClassification  / deck_generator_uncertified              3,008
CutOpenOrArrangement / source_synthetic_crossing                1,786
AtlasClassification  / periodic_lift_branch_unresolved          1,492
MaterialAuthority    / parity_contradiction                     1,452
CurveBoundaryWitness / band_bound_curve_unreadable              1,011
MaterialAuthority    / no_material_region                         554
CutOpenOrArrangement / overlap_unsupported                        469
AtlasClassification  / band_witness_refuted                       225
CutOpenOrArrangement / band_lift_join_no_compatible_integer       161
MaterialAuthority    / band_outer_bound_authority_absent           97
SourceImport         / AllBoundsCollapsed                          97
CutOpenOrArrangement / mixed_conflict                              40
SourceImport         / EdgeCurveConversionFailed                   28
CutOpenOrArrangement / band_orientation_incompatible                5
SourceImport         / SurfaceConversionFailed                      2
MeshRealization      / constraint_role_missing                      1
```

### The one judgment worth stating outright

`ConstraintInsertionIncomplete` / `SyntheticSyntheticCrossing` is the single
largest legacy bucket (10,174 faces, 37.9%). It means the **synthetic** edges —
the closure and seam segments the cut-open plan invented — cross each other. No
source geometry is in conflict on those faces, so the crossing is a symptom of a
wrong cut plan rather than a defect in the arrangement.

This is not an inference from taste. The cylinder-band route is gated on exactly
this bucket, and naming the cell removed the crossing for 15,123 faces without
touching a line of arrangement code. So where the chart carries a certified deck
generator, the primary diagnosis for that signature is `AtlasClassification` —
no quotient cell has been named for this (surface, boundary-homology) signature
— and the crossing is retained as the observed later exit. Where a *named* cell's
own certifier refused (the typed band exits), that finer verdict wins instead,
because it is later evidence about the same face and says more.

Every row of the full ledger carries `terminal_reason`, `derived_bucket` and the
band exit alongside the primary diagnosis, so nothing downstream is discarded.

## Surface family

```text
cone       6,951   25.9%      nurbs      3,494   13.0%
cylinder   5,524   20.6%      bspline    2,018    7.5%
torus      4,204   15.7%      sphere       691    2.6%
plane      3,518   13.1%      extruded     186    0.7%
                              revolved      69    0.3%
                              offset        35    0.1%
                              (no surface) 127    0.5%
```

## Atlas status

```text
NoImplementedAtlasCell             7,950   29.6%
NotReached                         6,822   25.4%
CandidateAtlasCell                 6,211   23.2%
NotEnoughEvidenceToClassify        4,501   16.8%
CellCandidateInsufficientEvidence  1,108    4.1%
ContradictoryCellEvidence            225    0.8%
```

Production implements a formal cell for **plane** (rank-0 disk, disk-with-holes)
and **cylinder** (the essential band) and for nothing else, so
`NoImplementedAtlasCell` is a statement about this system, not about the
mathematics. `NotReached` means the face stopped before any cell question could
be asked — overwhelmingly a boundary that was never witnessed.

## Declared against certified periodicity

A cell on a periodic surface needs a deck **generator**, and an axis that is only
*declared* does not supply one. Per family, over the lost population:

```text
cone       declared=1  certified=1   6,951      the revolution witness applies
cylinder   declared=1  certified=1   5,524      the revolution witness applies
torus      declared=2  certified=0   4,204      no generator at all
sphere     declared=1  certified=0     691      no generator at all
revolved   declared=1  certified=0      69
extruded   declared=1  certified=0      32
plane / nurbs / bspline / offset       9,065      genuinely aperiodic charts
```

`ConicalSurface` and `CylindricalSurface` are the same
`Processor<RevolutedCurve<Line>, Matrix4>` representation, so the exact `2π`
angular generator that `src/step/lattice.rs` certifies for a cylinder is
**already certified for a cone in production today**. A torus and a sphere are
different representations and reach the lattice with nothing certified; that is
deliberate (`lattice.rs` refuses to promote an accessor result) and it is a real
blocker, not an accounting artefact.

## Kind of work implied

```text
new mathematics              20,201   75.3%   (atlas, curve witness, material authority)
realization or arrangement    6,489   24.2%
source or import                127    0.5%
```

**Almost none of the remaining loss is provenance.** That is itself the headline:
the circle repair was the last large importer-retention defect in this corpus.
Source `outer_standing` is `declared` on 14,152 lost faces, `multiply_declared`
on 6,520 and `none_declared` on 6,018 — so a majority of what remains has
well-formed source authority and still cannot be meshed.

## Major homogeneous populations

| faces | models | primary / subreason | surface | atlas | representative |
|---|---|---|---|---|---|
| 5,283 | 15 | AtlasClassification / quotient_cell_not_named | cone | NoImplementedAtlasCell | `00000730#35469` `1[Ci1];1[Ci1]` |
| 2,957 | 14 | CurveBoundaryWitness / no_certified_preimage_on_support | nurbs | NotReached | `00000414#78928` `4[Bs2,Nu2]` |
| 2,543 | 14 | AtlasClassification / deck_generator_uncertified | torus | NotEnoughEvidenceToClassify | `00000730#36165` `1[Ci1];1[Ci1]` |
| 1,652 | 16 | CutOpenOrArrangement / source_source_crossing | plane | CandidateAtlasCell | `00000730#36147` `5[Bs2,Ci1,Ln2]` |
| 1,392 | 15 | CurveBoundaryWitness / no_certified_preimage_on_support | bspline | NotReached | `00000414#79314` `3[Bs3]` |
| 1,250 | 12 | CutOpenOrArrangement / source_source_crossing | plane | CandidateAtlasCell | `00000730#35853` `1[Ci1];6[Ln6]` |
| 1,004 | 7 | CurveBoundaryWitness / band_bound_curve_unreadable | cylinder | CellCandidateInsufficientEvidence | `00000730#64031` `1[Ci1];6[Bs6]` |
| 938 | 10 | CurveBoundaryWitness / no_certified_preimage_on_support | torus | NotReached | `00000730#39211` `1[Bs1];1[Bs1]` |
| 852 | 13 | CutOpenOrArrangement / source_synthetic_crossing | cylinder | CandidateAtlasCell | `00000730#35933` `4[Bs4];4[Bs4]` |
| 827 | 13 | MaterialAuthority / parity_contradiction | cylinder | CandidateAtlasCell | `00000730#35771` `2[Bs1,Ln1];1[Ci1]` |
| 673 | 6 | AtlasClassification / periodic_lift_branch_unresolved | cone | NotEnoughEvidenceToClassify | `00001075#50085` `1[Bs1];1[Bs1]` |
| 550 | 2 | CurveBoundaryWitness / no_certified_preimage_on_support | cylinder | NotReached | `00000730#35281` `2[Ln2]` |

`bound_signature` reads `n[Fk,…]` per bound: `n` edge uses carrying `k` curves of
each imported family — `Ci` circle, `El` ellipse, `Ln` line, `Bs` B-spline, `Nu`
NURBS. It is a source datum, not a shape claim.

### Remaining epistemic gap, per population

```text
cone quotient_cell_not_named        Need new atlas cell + its quotient cut plan
nurbs/bspline projection            Need exact curve-on-surface witness
torus deck_generator_uncertified    Need a representation-derived period witness
plane source_source_crossing        Need certified arrangement predicate, or a proof
                                    the source self-intersects
cylinder band_bound_curve_unreadable Need exact curve-on-surface witness for splines
cylinder source_synthetic_crossing  Need a quotient cut plan that cannot cross
                                    authoritative source trim
cylinder parity_contradiction       Need certified arrangement predicate, or the
                                    source is genuinely inconsistent
cone/cylinder/sphere lift branch    Need boundary homology before the branch is chosen
```

### Two smaller findings worth keeping

**550 cylinder faces bounded by exactly two line segments** (`2[Ln2]`, models
`00000730` and `00003172` only) fail boundary projection. Two rulings do not
bound a region on a cylinder on their own; this is a source-topology oddity
concentrated in two files and should not be read as a projection-mathematics gap
until it is looked at directly.

**2,412 torus faces carry the malformed double `FACE_OUTER_BOUND` pattern** on
two single-circle bounds, across 14 models — the identical source defect the
cylinder-band route recovers, on a torus. 2,375 of them are blocked earlier, at
the uncertified deck generator, so no band certificate can even be attempted.

## Model concentration

```text
00007705  3,810   00003172  2,156   00007744    753   00005641    476
00001075  3,329   00003902  1,908   00008001    752   00005586    102
00005760  3,136   00006483  1,425   00005427    723   00001116     96
00009190  2,236   00007667    949   00000959    584   00009972      8
00000730  2,190   00000414  2,184   00009272      0   00005642      0
```

## Recommended next packet — the conical essential band

**5,228 faces, 15 of the 20 models, one signature, 19.5% of the entire remaining
loss.** Every one of them:

```text
support           a conical surface, certified deck rank 1 (the 2π angular
                  generator, already certified in production today)
bounds            exactly two, each exactly one complete source CIRCLE
                  (bound_signature 1[Ci1];1[Ci1], unread_rank1 = 0 — the
                  production rank-1 classifier reads both)
source authority  outer_bound declared, count 1, on 5,191 of 5,228
                  none_declared on the other 37; multiply_declared on none
lift              Certified          projection   Successful
terminal          ConstraintInsertionIncomplete
bucket            SyntheticSyntheticCrossing, exactly one conflict witness,
                  two synthetic segments — the two artificial join segments
                  crossing each other
```

Representative: `00000730#35469`. Concentration: `00001075` 1,217, `00005760`
975, `00003902` 935, `00009190` 657, `00003172` 394, `00000730` 384, then eleven
more models down to 8.

Why this population and not a larger one:

- **The cell is the direct analogue of one already built and validated.** Two
  oppositely oriented essential parallels on ordered carriers, material
  authority from the compact strip between them modulo a rank-1 angular deck.
  The cylinder band's own header names cones as refused-by-name, not as
  impossible.
- **No new periodicity mathematics.** The deck generator is already certified,
  because a cone and a cylinder are the same `RevolutedCurve<Line>`
  representation. Contrast the torus population, whose 2,543 faces are blocked
  behind a period witness that does not exist yet.
- **The source is conformant.** Unlike the cylinder band, this needs no
  malformed-source normalization at all — `outer_bound` is singly declared on
  99.3% of the population.
- **It is the most homogeneous large population in the corpus.** One bound
  signature, one terminal reason, one bucket, one conflict count, across 15
  models.
- **It can be validated without heuristic geometry.** The same discipline
  applies: certificates admit, and the delta of a standalone gate is the route's
  own recovery count.

### What the packet must *not* inherit

The cylinder band's material-authority proof does not transfer by analogy, only
its shape. Three obligations are genuinely new and must be discharged, not
assumed:

1. **Carrier separation is not axial separation.** On a cylinder the two
   parallels are separated by a strict order of certified axial enclosures. On a
   cone the parallels have different radii, and the certificate has to be stated
   against the cone's own axial parameter with the half-angle carried, or a
   near-apex pair will certify falsely.
2. **The apex is a singular stratum and must be excluded by proof.** A STEP
   conical surface extends through its apex to the opposite nappe. Two circles
   on *opposite* nappes match this signature exactly and do not bound a band. A
   same-nappe obligation (both carriers strictly on one side of the apex, both
   radii certified nonzero) has no counterpart in the cylinder work.
3. **The outer-bound declaration is not the material authority here either.** A
   frustum band has no intrinsically outer loop, so the fact that 5,191 of these
   faces *do* declare one must not be used to select the region. Authority comes
   from the ordered carriers, exactly as it does on the cylinder — the
   declaration is conformance evidence, not a material claim.

The natural follow-on, once the cone cell exists, is the torus period witness
(2,844 faces, 2,375 of them carrying the malformed double-outer-bound pattern
the band route already knows how to handle); it is a prerequisite rather
than a recovery on its own, because certifying rank 2 moves those faces to a
toroidal cell that also does not exist yet.

## Artifacts

`C:\Users\stefa\look-corpus\remainder-out\` (13 MB), outside the repository:

```text
index.json            40 content-addressed runs, keyed on the file digest,
                      both revisions, the Cargo.lock hash and the schema version
<model>/ledger.tsv.gz per-face rendered/lost verdict and band attempt
<model>/diag.jsonl.gz per-lost-face structured FailedFaceDiagnosis
<model>/faces.tsv.gz  per-imported-face source authority (remainder_probe)
report.json           the machine-readable summary, schema remainder-diag-1
report.txt            the same, human-readable
remainder.tsv.gz      the full per-face ledger, 26,817 rows, 33 columns
                      sha256 7287c05de913e7c76a23156a86a8ee8ca8da2905c4fe4f07b870b6ebbd9003e9
```

## What was added, and what it can do

```text
examples/remainder_probe.rs      new, observation only — reads the imported
                                 shell and prints retained source authority
benchmarks/remainder_sweep.py    new, reuses band_sweep.py's guards verbatim
benchmarks/remainder_report.py   new, joins the three readings and classifies
docs/CIRCLE_PROVENANCE_RESULT.md one corrected figure (26,797 -> 26,817)
```

Nothing in `src/` or in `truck-fork` changed. `TRUCK_FACE_DIAG_JSONL` is set on
the census run and adds no behavioural change on top of the band gate, because
`diagnosis::diag_enabled()` is already true whenever `TRUCK_FORMAL_RECOVERY_BAND`
is set — the band route derives its own admission bucket from the same sink. The
baseline reproducing to the face (812,362 rendered) is the check on that claim.
