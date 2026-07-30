# PAR-RANGE-INHERITANCE-001 — Cone domain inherited from `Line`'s `[0,1]`

**Family** `PAR` · **Manifestation** `OMISSION` and `EXCESS`, both measured
under the counterfactual (§Measurements) · **Contracts** `DOM-003`, `QUO-005`,
`ARR-002`

> **Full record:**
> [`../look-collapsed-boundary/FORMALISM.md`](../../../look-collapsed-boundary/FORMALISM.md)
> §3, §4, §7 — geometry, per-claim D/A/U labelling, and the seven open
> questions. This entry does not restate it.

```
Status:
  Mechanism established
  Counterfactual experimentally validated   — flag-gated, default off
  Production correction not validated
```

A working surface domain was inherited from an implementation primitive's
arbitrary parameter range instead of being derived from the represented
geometry. The counterfactual both **recovered 137 faces and removed a blob
shell**, so neither "missing-face bug" nor "blob bug" would be a correct
primary classification.

## Obligation

The working domain of a face must be derived from the represented geometry and
the face's own bounds, and must not depend on the construction route to the
surface:

$$\Omega \supseteq \pi(\Gamma),\qquad \Omega \perp \text{(construction route to } S).$$

## What the implementation did

`Line::parameter_range()` returns $[0,1]$ unconditionally
(`truck-geometry/src/specifieds/line.rs:116`) and `RevolutedCurve` inherits it
verbatim (`revolved_curve.rs:110`), so a cone declares

$$\Omega_{\text{truck}} = [0,1] \times [0,2\pi),$$

one unit of generatrix chosen by `Line`, independent of the face. The apex sits
at $u^{*} = -R/\tan\theta$, **outside it** — $-6.010$ for the reproducer
($R=10$, $\theta=59°$).

## Counterexample / control

`repro/apex_only.stp` — one conical face `#370`, reduced from NIST
`nist_ctc_01_asme1_rd.stp` by rewriting the shell's face list and nothing else:
**0 triangles**. Control `repro/plane_control.stp`, same file, same reduction:
**46 triangles**.

Sharpest control: **cylinders are unaffected** under the same defective range,
because two circular bounds give $|\text{open}|=2$ and stitch to each other.
That localises the mechanism to the stitching, not to the range alone. **(A)** —
consistent with the code path, not separately measured.

## Measurements

```
PROBE piece pts=43 gap=6.283185e0 perimeter=6.283185e0 gap/perimeter=1.0 closed=false
PROBE in_closed=0 in_open=1 loops=1 areas=[+0.0000e0] uperiod=None
      vperiod=Some(6.283185307179586) range=true rect=false
```

Counterfactual `TRUCK_CONE_APEX_RANGE`:

| | off | on |
|---|---:|---:|
| ABC faces lost | 396 | **339** |
| NIST faces lost | 356 | **276** |
| ABC blob shells | 10 | **9** |

Blob shell `#161274` (ratio 30.3, 161 faces) disappears. Default path verified
byte-identical with the flag off.

## First divergent checkpoint

**I — material-domain construction.** The false input is set at **C — surface
conversion**, where the range is fixed; nothing between C and I notices.

## Causal derivation

```
cone inherits Line range [0,1]
→ apex u* = -R/tan θ lies outside the range
→ periodic base circle has lifted UV gap 2π
→ Euclidean closure test calls it open              [QUO-EUCLIDEAN-CLOSURE-001]
→ lone open piece stitched to the u=0 range edge    [DOM-ARTIFICIAL-CLOSURE-001]
→ range edge coincides with the base circle
→ constructed loop has area 0                       [DOM-ZERO-AREA-001]
→ no triangles retained
```

Split across four records because each arrow is a separately correctable
obligation: repairing the range alone leaves the Euclidean closure test wrong,
and vice versa.

## Correction

**Proposed**: derive the working domain from the face's bounds — either a
conversion context carrying them at surface-construction time, or a post-pass
retuning each surface to its faces. `From<&ConicalSurface>` has neither.

**Experimental**: `truck` `7199cc90`, default off, spans apex → 2× reference
radius. **Evidence, not a fix, and preserved as evidence** — it closes `U1`.
Not on because (a) the factor of 2 is arbitrary and a taller cone falls outside
again, unmeasured; (b) on NIST it moves 52 faces sideways,
`NoSurfaceProduced/cone` 216 → 268 while `MeshedToNothing/cone` 132 → 0 — those
faces trade an empty domain for a chart containing the rank-deficient apex.
That is [`SNG-COLLAPSED-DIRECTION-001`](SNG-COLLAPSED-DIRECTION-001.md), `U2`
arriving as predicted.

**Production**: none.

**What must not be done** (`FORMALISM.md` §7.4): a synthesised small circle
around the apex; using the declared rectangle when the boundary lies on its
edge; loosening the closure tolerance until $2\pi$ counts as zero; any arbitrary
range multiplier, the experiment's factor of 2 included.

## Tests

None. Required before closure: the reproducer; the `plane_control` control; an
**equivalent encoding** of the same cone yielding the same domain — the
metamorphic test that would have caught a construction-route-dependent domain;
and a corpus assertion that no cone face's declared range excludes its own
bounds.

## Known exclusions

- Not `UNKNOWN-NIST-ORDINARY-CONE` (216 faces, ordinary three-edge bounds,
  different terminal state) — `U4`.
- Does not explain the **64 ABC apex faces** that mesh to nothing while 208 of
  the same population render. NIST loses all 132, ABC 64 of 272: unexplained,
  and evidence the apex population is **not homogeneous** — `U3`.
- No seam shift, $2\pi$ translation, or chart reflection has been run, so
  periodic-lifting involvement is neither established nor excluded — `U6`.

## Claim status

- **(D)** `FORMALISM.md` D6–D10, plus the counterfactual table above.
- **(A)** The material region is the apex-to-base band; cylinders spared for the
  two-open-pieces reason; the apex is rank-deficient (standard, not measured
  here).
- **(U)** U2, U3, U4, U6, U7.

## Links

`truck` `7199cc90` · `look` `7c139b5`, `caf7236` ·
[`FORMALISM.md`](../../../look-collapsed-boundary/FORMALISM.md) ·
`../look-collapsed-boundary/measurements/probe-apex-vs-control.txt` ·
enabled by [`INC-VERTEX-LOOP-001`](INC-VERTEX-LOOP-001.md)
