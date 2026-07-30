# GEO-INCIDENCE-ACCEPTANCE-001 — Nearest point accepted as an incidence

**Family** `GEO` · **Manifestation** `OVERACCEPTANCE`
**Contracts** measures `GEO-005`; partially `GEO-006`. Satisfies **neither**.

## 1. Status

```
Mechanism established
Population confirmed by measurement
Correction implemented, DEFAULT OFF
Production correction refused — detection and policy are still fused,
  and the refusal discards the measurement that justified it
```

## 2. Mathematical objects

A boundary curve $C : [a,b] \to \mathbb{R}^3$ and the surface $S$ of the face it
bounds. Inverse projection returns $(u,v)$ minimising $\lVert C(t) - S(u,v)\rVert$.

## 3. Required obligation

Curve–surface compatibility. A boundary must *lie on* its face's surface:

$$\max_t\ \min_{u,v}\ \bigl\lVert C(t) - S(u,v)\bigr\rVert \le \varepsilon.$$

**A nearest point is not an incidence.** `search_nearest_parameter` answers
*where the closest point is*, never *whether the query lies on the surface*, so
a boundary belonging to another face still yields a plausible parameter and a
smooth uv path. `GEO-005` demands the residual bound; `GEO-006` demands that the
projection return evidence, not just an answer.

## 4. What the implementation did

Accepted the returned $(u,v)$ unconditionally and discarded the residual.

## 5. Minimal counterexample

Not reduced to a single face. The population is 315 boundary points on
`00009190` — a real, characterised population, but the surgical reduction step
was never taken and that is the main thing missing from this record.

## 6. Control / oracle

Brute force: $d_{\min}$ over the whole parameter domain **equals** the
projection's residual. So the projection is right and **the input is wrong** —
which is the measurement that turns "our solver is bad" into "our acceptance
criterion is missing".

## 7. Measurements

`00009190`, one fresh `LOOK_CACHE_DIR` per run. `COMPATIBILITY_FACTOR` is
overridable by `TRUCK_COMPAT_FACTOR`, read once through a `OnceLock` because it
sits in the per-boundary-point loop; `inf` disables the gate.

| factor | faces lost | no surface | meshed to nothing | triangles | fires |
|---|---:|---:|---:|---:|---:|
| off (`inf`) | 393 | 227 | 166 | 216,379 | 0 |
| 5 | 685 | 519 | 166 | 195,248 | 315 |
| 10 | 674 | 508 | 166 | 195,545 | 304 |
| 25 | 665 | 499 | 166 | 195,775 | 295 |
| 100 | 624 | 458 | 166 | 198,463 | 253 |

**The factor is not a tuning knob.** Loosening it twenty-fold removes 62 of 315
rejections. Of the 315 rejected points the **median sits at 191× the chord
tolerance** and the maximum at 617×; only 14 fall in the 5–10× band. Anything
from 5 to 100 selects the same population. A boundary point at 191× tolerance is
not export slack: **~1.2% of this model genuinely violates curve-on-surface
incidence.**

Boundary points on the blob shells sit **0.027 from their own surface** — nine
times the chord tolerance.

## 8. First divergent checkpoint

**F — curve–surface compatibility.** Confirmed by the brute-force control at
**G**: inverse projection is doing its job correctly on bad input.

## 9. Causal derivation

```
search_nearest_parameter returns the closest (u,v) unconditionally
→ a curve that does not lie on this surface still yields a smooth uv path
→ the path is used as a trim boundary
→ the face is trimmed by a region no source entity describes
→ a plausible, smooth, wrong mesh with no error raised
```

## 10. Proposed correction

`GEO-006`'s shape: return
`Projection { uv, projected, residual, stationarity_error }` and a
`WithinTolerance` witness, and separate **detection** from **policy** via
`InvalidGeometryPolicy` (`MATHEMATICAL_FOUNDATION.md` §31).

## 11. Experimental correction

`stefangolas/truck` `c465242c` and `f306311e`. Implemented in
`PolyBoundaryPiece::try_new`: `tol` threaded through both call sites, boundary
points rejected at `residual > tol * COMPATIBILITY_FACTOR`, diagnostic behind
`TRUCK_PROBE_COMPAT`.

> **The population is real; the gate does not fix the render.** With the gate
> off and at 5, `find_blobs` reports the same 10 blob shells with ratios
> identical to five decimals (160144 at 43.4, 160784 at 42.1, 161274 at 30.3).
> The gate was confirmed to fire 315 times inside that same binary, so this is
> not a plumbing artifact. It costs 292 faces and 21,131 triangles — about 10%
> of the model — and fixes not one blob.
>
> **This splits one assumed defect into two.** The incidence violation is real
> and is *not* what produces the blobs.

## 12. Production correction

**Refused, deliberately.** `COMPATIBILITY_FACTOR` is `f64::INFINITY` — the gate
is compiled in and off. Deleting a tenth of the model's triangles to fix nothing
is the wrong default; the same measurement is available on demand via
`TRUCK_COMPAT_FACTOR=5`. Turn it on for real only when something downstream can
**act** on the refusal rather than just drop the face.

The current refusal throws away the residual at exactly the moment it matters,
which is why this cannot be called a correction at all: it fails `GEO-006` on
its own terms.

## 13. Regression tests

None ID-named. The measurement harness *is* the test today, which is the wrong
shape: it needs a synthetic face whose boundary is displaced a known distance
off its surface, asserting the reported residual equals the displacement.

## 14. Corpus-wide effect

`00009190` only, in detail. The gate was also active during six-model sweeps at
`inf`.

## 15. Known exclusions

Does not explain the blobs. Does not overlap
[`INC-EDGE-DROP-001`](INC-EDGE-DROP-001.md): between them the two refuse 566
faces on individually sound grounds and move the blob count by zero.

## 16. A second finding, recorded because it generalises

**The gate was masking a hard crash.** Turning it off exposed an abort on ABC
`00000730` — a request for 6,638,692,106,004,871,184 bytes, root-caused as
[`NUM-SUBDIVISION-GROWTH-001`](NUM-SUBDIVISION-GROWTH-001.md). With that fixed
the model renders **better** gate-off (425,328 triangles) than it did with the
gate masking the fault (423,170).

> A validation layer that silently prevents a downstream crash makes the system
> look sounder than it is, and removing it looks like a regression. **Before
> removing any gate, establish what it is actually holding up.** This is an
> argument for fixing causes rather than adding gates.

## 17. Claim status

- **(D)** 315 boundary points on `00009190` sit at a median 191× chord tolerance
  from their own surface; brute force confirms the projection, not the solver,
  is right.
- **(D)** Rejecting them fixes no blob and costs ~10% of the model.
- **(D)** The gate was load-bearing for a crash it was not designed to prevent.
- **(A)** That the 1.2% is genuine source-geometry error rather than an artifact
  of an upstream transform. **The transform-provenance dump — evaluate source
  and converted geometry in the same coordinates at each transform stage and
  find the first stage where $d(C(t),S) \le \varepsilon$ stops holding — is the
  probe that would settle it, and it has not been run.**
- **(U)** Whether face→surface pairing is correct. It is checked for edges and
  **not** for surfaces; that and an inconsistently applied transform are the two
  remaining blob candidates.

## 18. Links

- `truck` `c465242c`, `f306311e`
- [`PLAN.md` § PR 1](../../PLAN.md)
- `../look-trimming-residual` — the residual scale defect and the isolated blob
  reproducers `shell_160144.step` (76 faces), `shell_160014.step` (53)
