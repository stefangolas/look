# ORI-FRAME-ORTHONORMALITY-GATE-001 — FrameLaw dispatch bypasses Frame3's orthonormality gate

**Family** `ORI` · **Manifestation** `DISTORTION`, `DEGENERATION` (measured
at the frame level; realized-geometry effects *(asserted)*)
**Contracts** the frozen `Frame3` axis convention and `Frame3::try_new`'s
validation (`truck-geometry/src/constructive/mod.rs`); CG plan §3.2's
frame-law refusal semantics (`FrameSingular` family).

## 1. Status

```
Closed (CC-DEF-BREP-FIXES, commit 10a1d13)
```n
## 2. Mathematical objects

A spine frame $(t, n, b)$ promised orthonormal: $\|t\| = \|n\| = \|b\| = 1$,
all pairwise dots zero, right-handed. `Frame3::try_new` validates exactly
this and refuses `InvalidInput` otherwise. The `FrameLaw` dispatch
(`SpineFrameRecipe::frame`, `constructive/recipe.rs:372-406`) is the only
producer of frames during realization.

## 3. Required obligation

A frame that `Frame3::try_new` would refuse must never reach realization:
the type's own constructor is the gate, and every producer must pass
through it (or refuse `FrameSingular` at the law level where the geometry
makes the frame ill-defined).

## 4. What the implementation did

All four frame laws construct `Frame3` by **struct literal**, never through
`try_new`:

- `frame_fixed.rs:35-39`: $b$ pinned to the plane normal, $n = b \times t$.
  Unit and pairwise-perpendicular **only while $t \perp b$** — on a
  non-planar spine the tangent leaves the plane, $t \cdot b \neq 0$, and
  the promised orthonormality fails, silently. No refusal exists for
  "tangent left the fixed plane".
- `frame_radial.rs`: $n$ analytic from the axis, $b = t \times n$ —
  right-handed by construction but non-orthogonal ($t \cdot n \neq 0$)
  wherever the spine tangent has a radial component. No refusal.
- `frame_up.rs`: see
  [`ORI-FRAME-HANDEDNESS-001`](ORI-FRAME-HANDEDNESS-001.md) (the exact
  sign error).
- `frame_transport.rs` (ParallelTransport) — clean: measured 0 violations
  (the double-reflection grid preserves the validated start frame).

## 5. Minimal counterexample

`showcases/examples/frame_probe.rs` on the waterslide composite spine
(drop → helix → runout, a genuinely non-planar spine):

- `FixedPlane { normal: unit_y() }`: $\max|t \cdot b| = 0.880$ over the
  13 stations (tangent nearly parallel to the pinned plane normal on the
  runout region), 13/13 stations violating orthonormality or handedness.
- `RadialAboutAxis`: $\max|t \cdot n| = 0.877$ (the runout heads nearly
  radially out of the tower axis), plus $0.34$ at the launch drop;
  handedness fine.
- Control: `ParallelTransport` — 0 violations of everything.

## 6. Control / oracle

The same recipe under `ParallelTransport` (the only law whose frames pass
`try_new`'s checks), and `Frame3::try_new` itself applied to the produced
triples — which refuses exactly the frames the dispatcher emitted.

## 7. Measurements

Frame probe values above. Realized-geometry effect measured at the volume
level in the waterslide battery: per-law volumes differ legitimately with
framing, so the frame-level probe is the witness, not the volume.

## 8. First divergent checkpoint

**Frame construction** (checkpoint: recipe frame dispatch). Spine and
profile evaluation are upstream and clean; realization downstream consumes
whatever frame it is handed without further checks.

## 9. Causal derivation

```
law constructor builds Frame3 by struct literal
→ try_new's orthonormality/handedness gate never runs
→ FixedPlane / RadialAboutAxis emit frames that try_new refuses
→ the profile plane rides a degenerate frame
→ the cross-section shears/collapses along the offending axis
→ a plausible-but-wrong swept solid, no refusal raised
```

## 10. Proposed correction

Route every law's result through `Frame3::try_new` (or a shared private
validated constructor) so the failure surfaces as the typed refusal the
convention already defines — mapping the orthonormality violation onto
`ConstructError::FrameSingular { at, law }` with the station attached.
`FixedPlane` additionally owes a booked decision: refuse when
$|t \cdot b|$ exceeds a declared bound, or project and certify, per plan
§3.2's "preferred for planar spines" intent — silent shear is the
one behavior the doctrine forbids.

## 11. Experimental correction

None yet.

## 12. Production correction

None — `vendor/truck` changes only through the packet loop.

## 13. Regression tests

`showcases/examples/frame_probe.rs` (station-level oracle; prints the
per-law violation table). A typed-refusal test belongs in the kernel-side
packet that lands the fix.

## 14. Corpus-wide effect

Not measured beyond the showcase; every non-planar-spine `FixedPlane` sweep
and every radial-component `RadialAboutAxis` sweep is affected.

## 15. Known exclusions

Planar spines under `FixedPlane` and helices under `RadialAboutAxis` (the
law's intended domain) are genuinely clean — this defect is about the
missing gate on the boundary, not about the happy path.

## 16. Relationship to other defects

[`ORI-FRAME-HANDEDNESS-001`](ORI-FRAME-HANDEDNESS-001.md) is the exact
sign-error instance of the same bypass. Both argue one correction: make
the unvalidated frame unrepresentable.

## 17. Claim status

- **(D)** The law paths build `Frame3` without `try_new` — read from the
  tree (`frame_fixed.rs:35`, `frame_up.rs:38`, `frame_radial.rs`
  struct literals).
- **(D)** The measured frame violations (probe values above).
- **(A)** Realized-geometry shear/collapse — asserted from the mapping
  $X = C + n p_x + b p_y$ with non-unit-orthogonal $(n, b)$; not yet
  isolated per-station on realized output.

## 18. Links

- `truck-geometry/src/constructive/{recipe,frame_fixed,frame_up,frame_radial}.rs`
- `truck-geometry/src/constructive/mod.rs` (`Frame3::try_new`)
- [`ORI-FRAME-HANDEDNESS-001`](ORI-FRAME-HANDEDNESS-001.md)
- CG plan §3.2 (frame-law semantics and refusals)
