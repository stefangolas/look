# ORI-FRAME-HANDEDNESS-001 — ArchitecturalUp frame law returns a left-handed frame

**Family** `ORI` · **Manifestation** `INVERSION` (measured), `DISTORTION`
*(asserted downstream)*
**Contracts** the frozen `Frame3` axis convention
(`truck-geometry/src/constructive/mod.rs`, `Frame3` doc + `Frame3::try_new`
validation); the CG plan §3.2 frame-law semantics.

## 1. Status

```
Closed (CC-DEF-BREP-FIXES, commit 10a1d13)
```n
## 2. Mathematical objects

A spine frame $(t, n, b)$ at a spine station: $t$ the unit tangent, and the
triple is normatively **right-handed orthonormal**: $t \times n = b$,
$n \times b = t$, $b \times t = n$ (`Frame3`'s frozen convention, and exactly
what `Frame3::try_new` validates). The sweep mapping is
$X(s, v) = C(s) + n\,p_x + b\,p_y$ — profile-x rides the NORMAL, profile-y
rides the BINORMAL — so the frame's handedness decides which side of the
spine the profile ring lands on and with which orientation.

## 3. Required obligation

Every frame any `FrameLaw` produces must satisfy the `Frame3` convention it
is typed as: unit vectors, pairwise orthogonal, right-handed.

## 4. What the implementation did

`frame_up.rs` (`constructive/frame_up.rs:38-42`) constructs

```rust
Ok(Frame3 { tangent, normal: tangent.cross(binormal), binormal })
```

i.e. $n = t \times b$. Then $t \times n = t \times (t \times b) = -b$
(with $t \perp b$): **left-handed, exactly, at every station** — not a
numerical artifact but a sign error in the construction formula. The plan
(§3.2, "ArchitecturalUp: `b = normalize(up × t)`, `n = t × b`") spells the
same left-handed formula, so the spec contradicts its own `Frame3`
convention and the implementation faithfully landed the contradiction. The
`Frame3::try_new` right-handedness gate is bypassed because the law builds
the struct directly.

## 5. Minimal counterexample

`showcases/examples/frame_probe.rs`: any `SpineFrameRecipe` under
`FrameLaw::ArchitecturalUp`. At all 13 sampled stations of the waterslide
spine, $|t \times n - b| = 2.000$ exactly (= $|-b - b|$), while pairwise
dots are $\sim 10^{-17}$ — perfectly orthogonal, perfectly wrong-handed.

## 6. Control / oracle

`FrameLaw::ParallelTransport` on the same spine: 0 handedness violations
(the double-reflection transport preserves the right-handed start frame).
`FrameLaw::RadialAboutAxis` and `FixedPlane` are right-handed by
construction ($b = t \times n$ resp. $n = b \times t$); see
[`ORI-FRAME-ORTHONORMALITY-GATE-001`](ORI-FRAME-ORTHONORMALITY-GATE-001.md)
for their separate orthonormality failure.

## 7. Measurements

- Frame probe (above): 13/13 stations left-handed.
- Waterslide battery (`showcases/tests/battery_waterslide.rs`,
  `architectural_up_handedness_inverts_the_solid`): the raw
  parametrization-signed volume of the swept solid is **negative under
  ParallelTransport (+magnitude), positive under ArchitecturalUp** —
  the left-handed frame maps the profile ring with flipped handedness, so
  the entire swept solid comes out inside-out relative to the right-handed
  laws. Same spine, same profile, same stations; only the frame law
  differs. The test pins this sign opposition as a regression witness.

## 8. First divergent checkpoint

**The frame law itself.** Spine evaluation, profile evaluation, station
sampling, and both realization backends are law-agnostic; the inversion
enters at `frame_up.rs`'s construction and propagates through
`SpineFrameRecipe::position` into every realized point.

## 9. Causal derivation

```
n = t x b  (instead of n = b x t)
→ t x n = -b
→ the (n, b) pair handedness flips relative to the ring convention
→ the profile ring maps to the reflected point set
→ the swept shell is consistently oriented but globally inside-out
→ signed volume flips sign; any orientation-sensitive consumer
  (booleans, clearance, outward-offset) sees the inverted solid
```

## 10. Proposed correction

`normal: binormal.cross(tangent)` in `frame_up.rs` (i.e. $n = b \times t$),
which makes $t \times n = b$. Then reconcile the plan §3.2 formula text
with the `Frame3` convention so the spec stops prescribing both. Also
route the law constructors through `Frame3::try_new` (or a private
validated constructor) so this class of error is unrepresentable — that is
[`ORI-FRAME-ORTHONORMALITY-GATE-001`](ORI-FRAME-ORTHONORMALITY-GATE-001.md)'s
correction.

## 11. Experimental correction

None yet. The battery witness test must be **inverted** (asserting equal
signs) when the kernel fix lands.

## 12. Production correction

None — `vendor/truck` changes only through the packet loop.

## 13. Regression tests

`showcases/tests/battery_waterslide.rs::architectural_up_handedness_inverts_the_solid`
(currently pins the defect's signature), plus
`showcases/examples/frame_probe.rs` as the station-level oracle.

## 14. Corpus-wide effect

Not yet measured beyond the showcase; every `ArchitecturalUp` sweep is
affected by construction.

## 15. Known exclusions

Planar spines with the profile plane aligned so the flip is invisible
(e.g. symmetric profiles) hide the manifestation but not the defect: the
frame is still left-handed.

## 16. Relationship to other defects

Shares one root cause (unvalidated `Frame3` construction) with
[`ORI-FRAME-ORTHONORMALITY-GATE-001`](ORI-FRAME-ORTHONORMALITY-GATE-001.md);
that record covers the non-orthogonal cases, this one the exact sign error.

## 17. Claim status

- **(D)** $t \times (t \times b) = -b$ for $t \perp b$ — arithmetic; the
  probe measures $|t\times n - b| = 2$ exactly.
- **(D)** The swept solid inverts — measured as the volume-sign flip under
  otherwise identical inputs (battery witness test).
- **(A)** Downstream consumers (booleans, offsets) would misbehave on the
  inverted solid — asserted, not yet exercised through those entries.

## 18. Links

- `truck-geometry/src/constructive/frame_up.rs` (construction)
- `truck-geometry/src/constructive/mod.rs` (`Frame3` convention, `try_new`)
- CG plan §3.2 (the contradicting formula text)
- `showcases/examples/frame_probe.rs`, `showcases/tests/battery_waterslide.rs`
