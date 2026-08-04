# ABC corpus cylinder-band sweep

Census of the formal cylinder-band recovery route across the whole 20-model ABC
corpus, in both gate states. **Census only** — no admission, normalization, or
validation behaviour was added or relaxed, and no face changed acceptance.

```text
look        7efb861f990e   (branch fix/correctness-phase-0-1)
truck-fork  23d946aa
Cargo.lock  b4d7e336da1395f9
```

No `.cargo/config.toml` `paths` override and no Cargo path patch participated;
`band_sweep.py --check-pins` verifies this and the sweep refuses to run
otherwise. `HANDOFF.md` and `opencode.json` were not touched.

Artifacts: `C:\Users\stefa\look-corpus\band-sweep-out\` (20 MB) — per-model
gzipped ledgers for both modes, the curve probe's per-edge verdicts,
`index.json` (content-addressed run cache), `report.json`, `report.txt`.

## Reproducing

```console
cargo +stable-x86_64-pc-windows-gnullvm build --release \
    --target x86_64-pc-windows-gnullvm --example face_census --example band_curve_probe
python benchmarks/band_sweep.py --dir C:/Users/stefa/look-corpus/abc --out band-sweep-out
python benchmarks/band_report.py --out band-sweep-out --json band-sweep-out/report.json
```

60 runs (20 models × {gate closed, band enabled, curve probe}), one isolated
process each. **All 60 completed** — no crash, timeout, parse failure, or
resource failure.

## Corpus totals

| | declared | rendered | lost |
|---|---|---|---|
| gate closed | 839,179 | 797,239 | 41,940 |
| band enabled | 839,179 | 802,918 | 36,261 |

```text
net gain            +5,679 faces  (+0.68% of declared, -13.5% of loss)
band eligible       16,622
band recovered       5,679   all malformed:two_outer_bounds_on_certified_band
band refused        10,943
```

The net rendering gain equals the recovery count exactly, and **no face
transitioned rendered → lost** in any model. The malformed double
`FACE_OUTER_BOUND` pattern accounts for **100%** of recoveries corpus-wide —
`00009190` was not special.

`00009190` reconciles to the frozen fixture unchanged: 19,716 / 4,486 closed,
21,684 / 2,518 enabled, 2,296 eligible, 1,968 recovered, 318 + 10 refused.

5 of 20 models gain nothing (0 eligible, or eligible but 0 recovered).

## BandExit histogram (face level)

```text
unsupported_curve_representation       10,822    e.g. 00000730:#35343
missing_outer_bound_authority              47    e.g. 00000959:#16292
witness_start_not_on_cylinder              32    e.g. 00000959:#22880
lift_join_no_compatible_integer            15    e.g. 00005760:#68637
witness_sweep_does_not_reach_endpoint      14    e.g. 00003172:#59082
witness_circle_not_a_cylinder_parallel     10    e.g. 00001075:#62345
band_orientation_incompatible               3    e.g. 00005760:#65941
                                       ------
                                       10,943   = band refused
```

`UnsupportedCurveRepresentation` is 98.9% of all remaining refusals.

## What is behind `UnsupportedCurveRepresentation`

The refusal seam is one branch: `planar_slice::traverse_bound`, on
`!CurveSchema::is_structurally_identified()`. The exit discards the schema, and
with it the reason. The census recovers the reason from two independent sides.

**Raw STEP entity chain** (`benchmarks/step_entity_chain.py`, 25,049 edge-use
occurrences on the 10,822 faces):

```text
CIRCLE                      20,802
B_SPLINE_CURVE_WITH_KNOTS    4,234
ELLIPSE                          9
_COMPLEX_ (rational spline)      4

wrapper chain    none — bare basis entity, all 25,049
p-curve          none supplied, all 25,049
```

Every occurrence references its geometry entity directly. There is **no**
`TRIMMED_CURVE`, `SURFACE_CURVE`, `CURVE_REPLICA`, or `COMPOSITE_CURVE`
indirection anywhere in the population, and **no `PCURVE` at all**.

**Imported representation and reader verdict** (`examples/band_curve_probe.rs`,
24,635 unread occurrences):

```text
Conic(Ellipse)  arc_non_circular_affine_image  shadow: circle          20,388
BSplineCurve    b_spline_curve                                          4,234
Conic(Ellipse)  arc_non_circular_affine_image  shadow: non-circular         9
NurbsCurve      rational_b_spline_curve                                     4
```

Circularity discrepancy of the refused conics, in units of `f64::EPSILON`
(`decode_transformed_circle`'s Gram predicate is exact; the pre-P0 ULP
classifier is retained for diagnostics):

```text
<= 64 eps   (within the certified-equal bound)   20,388
>  64e6 eps (certified non-circular)                  9
```

The 9 certified non-circular occurrences are exactly the 9 raw `ELLIPSE`
entities. **Every one of the other 20,388 is a raw STEP `CIRCLE` that misses
bitwise-exact circularity by at most 64 machine epsilons** — on `00009190` the
median is 0.66 ULP and the 99th percentile is 2.4 ULP.

### Face-level refusal classes

```text
every unread use is a near-exact circle    9,811   in 15 models
every unread use is a spline                 966   in  6 models
mixed, or a genuinely non-circular conic      45   in  6 models
                                          ------
                                          10,822
```

## `JoinNoCompatibleInteger`

15 faces in 3 models (`00009190` 10, `00005760` 4, `00007705` 1). Two raw
signatures, both structurally uniform:

```text
14   2 bounds, both FACE_OUTER_BOUND, 3 CIRCLE edge uses
 1   2 bounds, both FACE_OUTER_BOUND, 7 CIRCLE edge uses
```

The join's own numeric evidence is not retained by the exit, so this partition
is by raw source signature only. Distinguishing the deck-join failures further
would require instrumenting the lift in truck.

## Exporter association

Not available in this corpus. Every file blanks `originating_system` to `' '`,
and all 20 share `AUTOMOTIVE_DESIGN { 1 0 10303 214 1 1 1 1 }` and a
`/vol/tmp/translate-<n>/<id>.step` `FILE_NAME`. The corpus cannot separate
exporters, so no correlation drawn from it would be evidence about one.

## Recommended next packet

**Restore the source's own `CIRCLE`/`ELLIPSE` distinction at the import seam.**

- **Size**: 9,811 faces whose every unread bound curve is a near-exact circle —
  90.7% of remaining refusals, 1.7× the entire current recovery.
- **Spread**: 15 of 20 models. Not one file's defect.
- **Homogeneity**: one raw entity type, no wrapper, no p-curve, one reader
  cause, one shadow verdict. The most homogeneous population in the census.
- **Atlas cell exists**: `CircularArc` / `CompleteCircle` are already the band's
  admitted families, and 5,679 faces already recover through them.
- **New mathematics**: none.

The defect is *not* that the predicate is too strict. `truck-stepio` imports a
STEP `CIRCLE` and a STEP `ELLIPSE` to the same variant,
`Conic3D::Ellipse(Processor<TrimmedCurve<UnitCircle>, Matrix4>)`, so the entity
type the source declared is destroyed before any classifier runs. Circularity
then has to be *re-proved* from a floating-point transform the importer built
out of the file's finite-precision direction cosines — and it does not survive
the round trip. This census shows the loss directly: 9 genuine ellipses are
indistinguishable in the imported type from 20,388 genuine circles, and only
the ULP magnitude tells them apart.

So this is Route C, importer loss, at the source-evidence seam — not a
tolerance relaxation. `decode_transformed_circle`'s exact Gram predicate is
sound and should stay exact; it is being asked the wrong question. A `CIRCLE`
is source-authorized as a circle by its entity type and its declared radius and
axis placement, which is structural evidence, not something to rediscover from
a matrix.

**Do not** implement this by widening the circularity tolerance. That would
admit the 9 genuine ellipses on the same evidence and re-introduce exactly the
false-circle soundness defect `src/step/circular_arc.rs` documents removing.

Runners-up, both rejected for this round: the spline class (966 faces, 6
models) needs a genuine whole-interval flatness certificate — new mathematics,
Route D. The 15 `JoinNoCompatibleInteger` faces are too small to rank and need
truck-side instrumentation before they can even be partitioned.
