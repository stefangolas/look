# WAVE-2C — Formal recovery routes graduated to default-on

## What this changed

The five formal recovery routes (planar rank-0 slice, planar holes, rank-1
cylinder slice, cylinder/conical essential band, rank-2 torus annulus) shipped
behind opt-in environment variables while each was being proven. They are now
**default-on with explicit opt-out**.

Nothing about the geometry changed. No route was modified. The change is which
side of the switch they ship on, plus two consequences of default-on that had
to be handled (below).

Pinned truck revision: **`6f8153ea`** (`feature/torus-rank2-cell`), up from
`c0d15d7f`.

## Why this was the highest-yield next step

The work packet targeted `ConstraintInsertionIncomplete`, on evidence that it
was 4,015 faces and 84% of losses on `00009190`. That figure is accurate — for
a run with **every recovery route switched off**, which is what the census and
the shipped default both did.

With the routes on, ~88% of that population is already recovered by routes that
were built, certified, and merely not enabled. The remaining
`ConstraintInsertionIncomplete` on `00009190` is 599 faces, and its largest
class is no longer a quotient-seam problem at all (it is rank-0 planar
inter-bound arc crossings, 183). Writing a new seam repair would have been
worth a few hundred faces; enabling what already existed was worth 22,530.

## Result — 20-model ABC corpus

Legacy (`TRUCK_FORMAL_RECOVERY=0`) against default-on, joined per face on
`source_face_id`:

| | declared | rendered | lost | loss |
|---|---:|---:|---:|---:|
| legacy | 839,179 | 797,239 | 41,940 | 5.00% |
| default-on | 839,179 | 819,769 | 19,410 | 2.31% |
| delta | 0 | **+22,530** | −22,530 | |

Every transition is `lost -> rendered`. **`rendered -> lost` is 0** on all
twenty models, and the declared population is unchanged on all twenty.

Per-model recovery, largest first: `00005760` +4,417, `00003902` +4,193,
`00009190` +3,426, `00001075` +2,813, `00000730` +1,996, `00003172` +1,452,
`00006483` +923, `00009972` +720, `00007744` +644, `00008001` +644, `00000959`
+502, `00005641` +363, `00007705` +238, `00007667` +96, `00005586` +63,
`00001116` +40. Four models recover nothing: `00000414`, `00005427`,
`00005642`, `00009272` — the latter two already rendered every declared face.

`00007744` is duplicate geometry of `00008001` and reproduces it exactly: same
644 recovered, same per-surface split (cylinder 549, cone 51, torus 36, plane
8).

### `00009190`, by route

| | rendered | lost |
|---|---:|---:|
| legacy | 19,716 | 4,486 |
| + planar slice | +2 | |
| + cylinder/cone band | +2,907 | |
| + torus annulus | +517 | |
| default-on | 23,142 | 1,060 |

Recovered by surface: cylinder 2,250, cone 657, torus 517, plane 2.

## Why this cannot regress a rendered face

Every route is **refinement-only**: each is entered only where
`failure.is_some()`, i.e. only on a face the legacy path (and every earlier
route) failed to mesh. A route can therefore replace *nothing but* a failure.
The worst outcome of enabling one is that it declines to recover. This is
enforced at each of the five call sites in `triangulation.rs`, not by
convention, and the corpus reconciliation confirms it empirically at 0
regressions across 839,179 faces.

## Gate semantics

Only an explicit negative disables a route: `0`, `off`, `false`, `no` (any
case, surrounding whitespace ignored). Unset — or set to anything else — leaves
it on.

| variable | effect |
|---|---|
| `TRUCK_FORMAL_RECOVERY=0` | master kill switch; restores the exact legacy tessellation |
| `TRUCK_FORMAL_RECOVERY_HOLES=0` | planar-holes route off |
| `TRUCK_FORMAL_RECOVERY_CYLINDER=0` | rank-1 cylinder slice off |
| `TRUCK_FORMAL_RECOVERY_BAND=0` | cylinder + conical band off |
| `TRUCK_FORMAL_RECOVERY_TORUS=0` | torus annulus off |
| `TRUCK_PROBE_RECOVERY=1` | re-enable the per-recovery stderr log |

`_BAND` and `_TORUS` are now nested under the master gate. They previously
stood outside it so a band or torus run could be measured without the planar
route's recoveries mixed in; under default-on that is done instead by setting
the one route to `0` and diffing, so the reason no longer applies and the
master variable is a single kill switch.

The kill switch was verified to restore the legacy result face for face, not
merely in total.

## Two consequences of default-on, handled

**The diagnostic sink now runs on a default render.** The band routes admit
exactly the `SyntheticSyntheticCrossing` bucket, and that bucket is *derived
from the insertion witnesses* — so the sink is an input to a production
decision, not only a report, and it must be filled for the routes to know what
they may attempt. Measured cost on `00009190`, minimum of five alternating
reps: **5.504s legacy against 5.580s default-on**, inside this machine's noise.
(Measure by alternating configurations. Running all reps of one config and then
all of the other produced an apparent 2.6× slowdown that does not exist —
this machine has interference that hits whichever config runs second.)

**Recoveries no longer announce themselves on stderr.** Each recovery printed a
`RECOVERED` line, and the planar route a `RECOVERED_VERTEX` line per vertex.
That was the point while the routes were opt-in; with them on it is 525
unrequested lines per render of `00009190`, on a tool whose stderr agents
parse. The recoveries are already carried structurally by
`MeshedShellOutcome::{band_attempts, cone_band_attempts, torus_band_attempts}`,
which is what the census reads, so the log moved behind `TRUCK_PROBE_RECOVERY`.

## A census bug this uncovered

`examples/face_census.rs` called `robust_triangulation_with_cone_outcome`,
which supplies **no torus adapter**, while production calls
`robust_triangulation_with_torus_outcome`. The torus annulus route therefore
never ran under the census regardless of its gate, and every toroidal face was
reported lost. On `00009190` that understated recovery by 517 faces.

The census now uses production's entry point and carries a `torus=` ledger
column and its own funnel. **Any torus figure produced by this example before
this change is wrong and should not be compared against.**

## Verification performed

- 20-model ABC corpus, both configurations, per-face join on `source_face_id`:
  0 regressions, 0 population drift, +22,530 recovered.
- Kill switch restores legacy face for face.
- Per-route opt-out deltas reconcile exactly against the total
  (19,716 + 2,907 + 517 + 2 = 23,142 on `00009190`).
- `truck-meshalgo` lib tests: 589 passed, 0 failed.
- `look` tests: 153 lib + integration suites passed, 0 failed.
- Both repositories `cargo fmt`-clean.

Not run: `truck-meshalgo`'s three `tests/tessellation` integration tests, which
require `resources/shape/*.json` — absent from this checkout, and failing
identically before this change.

## Artifacts

Outside git, under `C:\Users\stefa\look-corpus\p1-out\`:

- `corpus/off_<id>.{census,ledger}.txt`, `corpus/on_<id>.{census,ledger}.txt` —
  per-model, per-face, both configurations
- `reconcile.py` — the per-face transition reconciler (joins on
  `source_face_id`; `declared_face_index` is per-shell and collides)
- `sweep.sh` — the corpus runner
- `base_*.jsonl`, `t_*.jsonl` — DIAG-001 records for the baseline and
  corrected-config censuses

## What is next

With the routes on, the remaining loss is 19,410 faces corpus-wide (2.31%).
On the two benchmark geometries the remaining population is:

| class | faces | note |
|---|---:|---|
| `ConstraintInsertionIncomplete` | 829 | 48.3% of remaining loss |
| `BoundaryProjectionFailed` | 598 | 34.8%; largest single class on `00008001` |
| `ContradictoryDualParity` | 148 | |
| `AmbiguousLift` | 107 | |

Within `ConstraintInsertionIncomplete`, the largest coherent class is **rank-0
source/source arc crossings (299 inter-bound + 154 same-bound)** — planar and
spline faces whose boundary arcs properly cross in a simply connected domain.
That is arrangement normalization (`NormalizeIntersections`, HANDOFF Priority
2 / ARR-003), not a quotient-seam problem: split both arcs at the certified
intersection and re-insert.

The quotient-seam population the original packet aimed at is **399 faces** on
periodic charts (seam×seam 189, source×seam 187), spread across torus 72+43,
cylinder 60+49, cone 55+26, sphere 24. It is real and formally well-posed, but
it is now a few-hundred-face target rather than a few-thousand-face one.
