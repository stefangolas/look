# Source conic provenance — result

Restores the source `CIRCLE` / `ELLIPSE` distinction through import, so a
circle is admitted on its entity type rather than re-proved from a rounded
transform. Census packet `ABC-CORPUS-CYL-BAND-SWEEP` measured the population;
this is what repairing it did.

```text
look        (this commit)
truck-fork  73db1851   (was 23d946aa)
Cargo.lock  99c6490223e24b14
```

No `.cargo/config.toml` `paths` override and no Cargo path patch participated
in any number below; `band_sweep.py --check-pins` gates the sweep and every run
resolved truck from the pushed git revision. `HANDOFF.md` and `opencode.json`
untouched.

## The change

`truck-stepio` imported `circle` and `ellipse` to the same `Conic3D::Ellipse`,
so consumers had to re-derive circularity from the imported transform. For a
`circle` that transform is an ISO 10303-42 derived orthonormal basis times a
uniform scale — a similarity in exact arithmetic, and not bit-exactly one after
the file's direction cosines are normalized and crossed in `f64`. The exact
Gram predicate then refuses an ordinary circle, correctly and uselessly.

`Conic2D`/`Conic3D` now carry a `Circle` variant, routed from `Conic::Circle`
in both construction paths (including `sub_parse_curve3d`, the `edge_curve`
trim recovery nearly every real bound curve goes through). `look` gained
`decode_source_circle` beside `decode_transformed_circle`: same computation,
same interval/orientation/placement/complete-circle semantics, differing only
in the circularity obligation. A source circle must still be shown to carry a
circle-preserving transform, within the rounding a derived orthonormal basis
can accumulate; beyond that it is refused, and a measurable non-similarity is
refused as such.

**The exact predicate is unchanged and still governs every representation
without source authority.** A source `ellipse` never reaches the new gate, so
no ellipse is reclassified however nearly circular it is.

## Corpus baseline (20 ABC models, 60 isolated runs, all completed)

| | declared | rendered | lost |
|---|---|---|---|
| gate closed | 839,179 | 797,239 | 41,940 |
| band enabled, before | 839,179 | 802,918 | 36,261 |
| band enabled, after | 839,179 | **812,362** | **26,797** |

```text
band eligible    16,622   unchanged
band recovered    5,679 -> 15,123   (+9,444)
band refused     10,943 ->  1,499
```

**Gate-closed rendering is byte-identical in every model**, and no face
transitioned rendered → lost anywhere. The recovery gain equals the rendering
gain exactly.

## How the refused population reconciles

Per model and in total, with band eligibility unchanged
(`benchmarks/band_compare.py`, every model balances to zero):

```text
old UnsupportedCurveRepresentation      10,822
    recovered                            9,444
    advanced to a later typed exit         367
    still refused                        1,011
```

The census predicted 9,811 faces whose every unread bound curve was a
near-exact circle. **Exactly 9,811 left the curve exit** — 9,444 recovered and
367 advanced to a later stage they could not previously reach. Nothing else
moved.

`00009190`, the frozen fixture: 19,716 / 4,486 gate closed (unchanged);
21,684 → **21,966** rendered band-enabled; 2,296 eligible (unchanged);
1,968 → **2,250** recovered; `UnsupportedCurveRepresentation` 318 → **32**;
`JoinNoCompatibleInteger` **10, unchanged**. Its 318 reconcile as
282 recovered + 4 advanced + 32 refused.

### The nine ellipse controls

The 9 raw `ELLIPSE` occurrences, on 7 faces in 3 models, **remain refused** and
are now the *only* conic refusals in the corpus. All 20,388 near-exact-circle
occurrences are admitted; the class the census called `near_circle_faces` is
now empty.

```text
BandExit histogram, face level     before   after
unsupported_curve_representation   10,822   1,011
lift_join_no_compatible_integer        15     161
missing_outer_bound_authority          47      97
witness_sweep_does_not_reach_endpoint  14      87
witness_start_not_on_cylinder          32      72
witness_circle_not_a_cylinder_parallel 10      64
band_orientation_incompatible           3       5
witness_not_constant_axial_coordinate   0       2
                                   ------   -----
                                   10,943   1,499
```

Every non-curve exit grew, because faces that used to stop at the traversal
gate now reach the stages beyond it. Those are the 367 advanced faces, not
regressions — `JoinNoCompatibleInteger` is a later stage than the curve gate
and a face reaching it has progressed. On the fixture it is unchanged at 10.

## What is left

```text
1,004 faces   every unread curve is a spline   7 models
    7 faces   a genuinely non-circular conic   3 models
```

4,234 `B_SPLINE_CURVE_WITH_KNOTS` and 4 rational-spline occurrences. Still no
wrapper indirection and no `PCURVE` anywhere in the refused population. The
spline class needs a whole-interval flatness certificate — new mathematics, and
deliberately not attempted here.

## Tests

`look`: 109 lib tests pass. New coverage — a source circle under a
finite-precision derived placement is retained (and the same representation
with its family erased is refused); a one-ULP `ellipse` stays an ellipse; a
non-uniformly transformed circle is refused; a measurable non-similarity is
undecided rather than admitted; both decoders agree bit-for-bit wherever the
exact predicate certifies; orientation folds exactly once; the complete-circle
and distinct-vertex closure rules are unchanged. A guard test asserts the
fixture placement really is inexact, so the suite cannot silently stop testing
the gate.

`truck-stepio`: `the_source_conic_family_survives_import` parses raw STEP text
and asserts `CIRCLE` → `Conic3D::Circle` and `ELLIPSE` → `Conic3D::Ellipse`,
including an `ELLIPSE` with equal semi-axes — geometrically a circle, and still
an ellipse, because the family is what the source said.

Pre-existing and untouched: `truck-stepio`'s *lib* tests do not compile at
`23d946aa` or at its parent (`convert.rs:1035` missing
`FaceProvenance::outer_bound`). Confirmed by stashing this change and
reproducing on the clean baseline. The integration tests build and pass.

## Artifacts

`C:\Users\stefa\look-corpus\band-sweep-out\` — `index.json` (120 cached runs,
both revisions), `report.json` / `report.txt` (this baseline),
`report.baseline-23d946aa.json` / `.txt` (the previous one),
`reconciliation.txt`, and per-model gzipped ledgers.

Reproduce:

```console
python benchmarks/band_sweep.py --dir C:/Users/stefa/look-corpus/abc --out band-sweep-out
python benchmarks/band_report.py --out band-sweep-out --truck-rev 73db1851 --json band-sweep-out/report.json
python benchmarks/band_compare.py --baseline band-sweep-out/report.baseline-23d946aa.json \
    --new band-sweep-out/report.json
```
