# ABC corpus conical essential-band sweep

Census of the formal conical essential-band recovery route across the whole
20-model ABC corpus. **Census only** — no admission, normalization, or
validation behaviour was added or relaxed beyond the cone cell itself, and the
cylinder band's prerequisites are unchanged. The cone cell shares the rank-one
periodic-annulus realizer with the cylinder cell; it does not share admission
semantics.

```text
look        e2bf18ac7b57   (branch fix/correctness-phase-0-1)
truck-fork  2b4537c4e01de54e195c4fe732417b600457f417  (origin/fix/correctness-phase-0-1)
Cargo.lock  cb293d3a9ad018af905ca01775e2a2129ac179614cd776078a4de95bb780bf37
```

No `.cargo/config.toml` `paths` override and no Cargo path patch participated;
`remainder_sweep.py --check-pins` (which shares `band_sweep.check_pins`)
verifies this and the sweep refuses to run otherwise. `HANDOFF.md` and
`opencode.json` were not touched. `cargo tree` resolves `truck-meshalgo v0.4.0`
from `git+https://github.com/stefangolas/truck?rev=2b4537c4#2b4537c4…`, not a
path, so the reproduction is independent of the local truck checkout.

## Reproducing

```console
cargo +stable-x86_64-pc-windows-gnullvm build --release \
    --target x86_64-pc-windows-gnullvm \
    --example face_census --example remainder_probe --example band_curve_probe
python benchmarks/remainder_sweep.py --dir C:/Users/stefa/look-corpus/abc --out cone-final
python benchmarks/band_sweep.py     --dir C:/Users/stefa/look-corpus/abc --out cone-band
```

`cone-final` is the band-open reading (census + structured `FailedFaceDiagnosis`
+ the `remainder_probe` source-authority reading) — 40 runs, one isolated
process each, all `completed`. `cone-band` is the gate-closed and band-enabled
census plus the gate-independent curve probe — 60 runs, all `completed`. The
previous session's interrupted partial run is preserved verbatim at
`C:\Users\stefa\look-corpus\cone-out\` (10.5/20 models, keyed on the pre-commit
`look` HEAD) as evidence of the interruption; it was not mixed into this run.

## Corpus totals

| | declared | rendered | lost | coverage |
|---|---|---|---|---|
| pre-cone baseline (`remainder-out`, band open) | 839,179 | 812,362 | 26,817 | 96.804% |
| cone enabled (`cone-final`, band open) | 839,179 | 817,525 | 21,654 | 97.420% |

```text
net gain            +5,163 faces  (+0.61% of declared, -19.2% of loss)
```

The pre-cone baseline reproduces `ABC_REMAINDER_DIAG.md` to the face
(812,362 rendered). Declared-face population is unchanged (839,179 = 839,179,
0 missing, 0 new).

## Face-level reconciliation

The diagnosed conical essential-band population — Cone surface,
`SyntheticSyntheticCrossing` bucket, exactly two bounds each one complete source
CIRCLE (`bound_signature 1[Ci1];1[Ci1]`, `unread_rank1 = 0`), certified deck
rank 1 — is **5,228 faces in 15 of 20 models**, derived face-by-face from the
pre-cone `diag.jsonl` joined to `remainder_probe` source authority. This matches
the `ABC_REMAINDER_DIAG.md` concentration to the face.

Every one of the 5,228 reconciles into exactly one category against `cone-final`:

```text
recovered                         5,163
still refused (cone band exit)       65
  cone_witness_start_not_on_cone     59
  cone_witness_circle_not_a_cone_parallel  6
advanced to a later typed exit        0
no longer matches the signature       0
                                   -----
total                              5,228
```

The 65 refusals are the cone band's certification correctly rejecting faces
whose witness geometry does not lie on the certified cone — not a regression
and not a relaxation. The recovery count is **not** inferred from the aggregate
rendered-count increase; it is the per-face `cone_band=recovered:…` count joined
on `source_face_id`, and it equals the rendered gain exactly.

The cone route attempts 5,667 faces (every Cone + `SyntheticSyntheticCrossing`
face, all of which carry `bound_count = 2` and `chart_rank = 1`). Of the
5,667 − 5,228 = 439 attempted faces outside the diagnosed signature, all 439
are refused (their bounds are not both complete source circles):

```text
cone attempted                    5,667
cone recovered                    5,163
cone refused                        504
  unsupported_curve_representation   379
  cone_witness_start_not_on_cone     63
  cone_band_bound_not_one_occurrence 55
  cone_witness_circle_not_a_cone_parallel 7
```

### Per-model cone recovery

```text
model      declared  preRend coneRend recov diag5228 ref5228
00000730     30302    28112    28496   384     384       0
00000959     10298     9714     9798    84      84       0
00001075     30276    26947    28122  1175    1217      42
00001116      1674     1578     1592    14      14       0
00003172     22971    20815    21209   394     394       0
00003902     26045    24137    25072   935     935       0
00005586      2280     2178     2186     8       8       0
00005641    179656   179180   179279    99     100       1
00005760     43986    40850    41823   973     975       2
00006483     23049    21624    21860   236     252      16
00007667      7713     6764     6824    60      60       0
00007705     22076    18266    18308    42      42       0
00007744     12030    11277    11328    51      53       2
00008001     12030    11278    11329    51      53       2
00009190     24202    21966    22623   657     657       0
TOTAL       839179   812362   817525  5163    5228      65
```

`recov` is the rendered gain vs the pre-cone baseline; `diag5228` is the
diagnosed population; `ref5228` is the diagnosed faces the cone band refused.
4 cone-bearing models (00001116, 00003902, 00005586, 00007667) recover every
attempted face with zero refusals.

## Regression checks

### Rendered-face monotonicity

No face that rendered in the pre-cone band-open baseline regressed:

```text
old rendered -> new rendered   812,362
old rendered -> new lost              0
old lost     -> new rendered      5,163   (all via cone_band=recovered)
old lost     -> new lost         21,654
```

Every recovery is attributed to `cone_band=recovered:…`; recoveries outside the
cone population: **0**.

### Cylinder band unchanged

The cylinder `band=` ledger column is identical across all 839,179 faces between
the pre-cone and cone-enabled runs (0 differences). Cylinder recoveries
(15,123) and refusals (1,499) are unchanged; cone admission did not move a
single cylinder outcome.

### Gate-closed invariance

With `TRUCK_FORMAL_RECOVERY_BAND` unset the cone route is never entered
(`band_recovery_gate` is false in `triangulation.rs`, so `cone_of` is never
called), so the cone entry point behaves identically to the cylinder entry
point. Measured against the `band-sweep-out` gate-closed baseline
(`look 54f7db1` / `truck 73db1851`):

```text
cone-build gate closed   797,239 rendered / 41,940 lost   (== baseline)
per-face rendered diffs  0  (across 839,179 faces)
```

As a determinism cross-check, `cone-band`'s `band_enabled` reading is
byte-identical to `cone-final`'s `census_diag` reading (0 differences across
839,179 faces in `rendered`, `band` and `cone_band`).

## Final revisions and pin

```text
truck commit        2b4537c4e01de54e195c4fe732417b600457f417
truck remote branch origin/fix/correctness-phase-0-1
look commit         e2bf18ac7b57794b8bbcb7df39901f03bd204482
pinned truck rev    2b4537c4  (cargo tree, git source, no path override)
Cargo.lock SHA-256  cb293d3a9ad018af905ca01775e2a2129ac179614cd776078a4de95bb780bf37
pin check           pins clean
```

## Artifacts

Outside the repository (not committed; multi-megabyte generated ledgers):

```text
C:\Users\stefa\look-corpus\cone-final\   40 runs: census_diag + source_probe, 20 models
C:\Users\stefa\look-corpus\cone-band\    60 runs: gate_closed + band_enabled + curve_probe
C:\Users\stefa\look-corpus\cone-out\     interrupted partial (10.5/20), preserved as evidence
C:\Users\stefa\look-corpus\remainder-out\ pre-cone baseline (band open + source_probe)
C:\Users\stefa\look-corpus\band-sweep-out\ gate-closed + cylinder baseline
```

## Known unrelated test failure

`cargo test -p truck-stepio --lib` fails to compile at
`truck-stepio/src/in/convert.rs:1035`: the `FaceProvenance` struct gained an
`outer_bound` field in `truck-topology` that the stepio test initializer
(`the_surface_identity_is_recorded_separately`, a `#[test]`) has not been
updated for (`error[E0063]: missing field outer_bound`). It is a library-test
compile error in `truck-stepio`, not in `truck-meshalgo` where the cone cell
lives, and it is unrelated to this packet. `cargo test -p truck-meshalgo --lib`
passing (cone 14, cone_band 16, cone_topology 12, cylinder_band 9, and the
shared rank-one annulus tests) is the check on the cone work.

## Validation summary

```text
cargo test --lib step::cone::tests                      10/10 ok
cargo test -p truck-meshalgo --lib cone                 43/43 ok
    (formal::cone 14, formal::cone_band 16, cone_topology 12,
     cylinder::tests::cone_is_refused 1)
cargo test -p truck-meshalgo --lib cylinder_band         9/9 ok   (no regression)
cargo build --release --example face_census/remainder_probe/band_curve_probe  ok
remainder_sweep.py --check-pins                         pins clean
remainder_sweep.py (cone-final, 40 runs)                all completed
band_sweep.py (cone-band, 60 runs)                      all completed
```
